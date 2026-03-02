# SECURITY_REPORT.md - Auditoría de Seguridad SODIIC/Osin-Tools

Este documento detalla la postura de seguridad, el manejo de datos y las defensas implementadas en la aplicación, siguiendo los estándares OWASP y normativas ISO.

## 1. Auditoría de Stack y Dependencias

| Tecnología/Dependencia | Propósito             | Contribución a la Seguridad                                                    |
| :--------------------- | :-------------------- | :----------------------------------------------------------------------------- |
| **Rust (Tauri)**       | Core Backend          | Gestión de memoria segura (Memory Safety) y aislamiento del frontend.          |
| **Keyring-rs**         | Almacenamiento Seguro | Uso de almacenes nativos (Windows Credential Manager) para API Keys y cookies. |
| **AES-GCM (Rust)**     | Cifrado               | Cifrado de datos sensibles en el transporte interno si fuera necesario.        |
| **Zod (Frontend)**     | Validación            | Validación de esquemas en el frontend para prevenir tipos inesperados.         |
| **Svelte (A11y)**      | Accesibilidad         | Cumplimiento de estándares de accesibilidad para evitar exclusión.             |

## 2. Mapa de Endpoints y Seguridad (Backend)

| Ruta / Comando Tauri | Nivel de Seguridad | Mecanismo de Manejo                                             |
| :------------------- | :----------------- | :-------------------------------------------------------------- |
| `save_secure_secret` | Sensible           | Escribe directamente en el Keyring de Windows.                  |
| `get_secure_secret`  | Sensible           | Recuperación desde Keyring con acceso restringido.              |
| `run_wsl_command`    | Crítico            | Requiere contraseña de SUDO (manejada vía Keyring).             |
| `ask_agent`          | Informativo        | No persiste PII en logs externos; usa historial local por caso. |
| `web_scrape_search`  | Operativo          | Inyecta cookies de sesión de forma segura y temporal.           |

## 3. Estrategia de Sanitización y Prevención

- **Inyección SQL**: La aplicación utiliza serialización JSON y estructuras de datos tipadas en Rust. No se concatenan strings para consultas a bases de datos (uso de CaseManager con archivos locales).
- **XSS (Cross-Site Scripting)**: Svelte escapa automáticamente el contenido dinámico. En el agente, se utiliza un parser híbrido para limpiar bloques de código sospechosos.
- **Inyección de Comandos**: El acceso a WSL (`run_wsl_command`) está restringido a comandos específicos si se desea, y los parámetros son escapados.

## 4. Matriz de Prevención de Vulnerabilidades (OWASP Top 10)

| Vulnerabilidad                 | Medida Implementada                                                      | Ataque Prevenido                                       |
| :----------------------------- | :----------------------------------------------------------------------- | :----------------------------------------------------- |
| **A01: Broken Access Control** | RBAC implícito por sesión de usuario de Windows.                         | Acceso no autorizado a secretos del Keyring.           |
| **A03: Injection**             | Parametrización de comandos en Rust.                                     | Command Injection en herramientas OSINT.               |
| **A04: Insecure Design**       | Separación clara entre el Agente (LLM) y la ejecución real.              | Ejecuciones accidentales por alucinaciones del modelo. |
| **A07: ID & Auth**             | Manejo de cookies de sesión aisladas por plataforma (LinkedIn, FB, etc). | Session Hijacking/Fixation.                            |

---

_Última actualización: 2026-03-02_
