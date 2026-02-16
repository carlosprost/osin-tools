# SECURITY_REPORT.md - OSINT Tools Dashboard

Este documento detalla la postura de seguridad, el stack tecnológico y las medidas preventivas implementadas en el proyecto para garantizar la integridad y confidencialidad de las investigaciones.

## 1. Auditoría de Stack y Dependencias

| Tecnología            | Rol          | Contribución a la Seguridad                                          |
| :-------------------- | :----------- | :------------------------------------------------------------------- |
| **Rust (Tauri)**      | Backend      | Seguridad de memoria (Memory Safety) inherente.                      |
| **Svelte**            | Frontend     | Protección contra XSS mediante sanitización automática de templates. |
| **SQLite (Rusqlite)** | Persistencia | Almacenamiento local cifrado (vía FS) y consultas parametrizadas.    |
| **Tor Sidecar**       | Red          | Ofuscación de IP y anonimización de tráfico OSINT.                   |
| **Argon2 / Bcrypt**   | Criptografía | Estándar para hashing de credenciales (si aplica).                   |
| **Zod / Serde**       | Validación   | Estricto tipado de datos de entrada/salida para evitar inyecciones.  |

## 2. Mapa de Endpoints y Seguridad (Backend - Tauri Invoke)

| Comando           | Nivel de Seguridad | Mecanismo de Manejo                                |
| :---------------- | :----------------- | :------------------------------------------------- |
| `ask_agent`       | Autenticado        | Verificación de Estado Local / Guardián de Tauri.  |
| `set_tor_active`  | Privilegiado       | Control de proceso Sidecar (Kill switch incluido). |
| `set_mac_masking` | Privilegiado       | Script PowerShell con ejecución controlada.        |
| `create_case`     | Autenticado        | Validación de caracteres y sanitización de Path.   |

## 3. Estrategia de Sanitización

- **SQL Injection**: Prevención absoluta mediante el uso exclusivo de consultas parametrizadas en `rusqlite`. Prohibida la concatenación de strings en queries.
- **XSS (Cross-Site Scripting)**: El dashboard utiliza el motor de renderizado de Svelte que escapa automáticamente el contenido dinámico.
- **Command Injection**: Los comandos de sistema (PowerShell/Ping) utilizan argumentos separados y una función de validación de caracteres (`is_safe_target`) que bloquea caracteres especiales de shell (| , & , ; , etc.).
- **Error Handling (ISO 27032)**: Se ha implementado una capa de abstracción de errores que filtra excepciones técnicas del sistema (IO, SQL, HTTP) y devuelve mensajes genéricos al usuario, logueando el detalle técnico solo en canales internos (`stderr`).

## 4. Matriz de Prevención de Vulnerabilidades (OWASP Top 10)

| ID OWASP     | Riesgo Mitigado       | Implementación Técnica                                                               |
| :----------- | :-------------------- | :----------------------------------------------------------------------------------- |
| **A01:2021** | Broken Access Control | Implementación de RBAC básico y aislamiento de directorios por investigación.        |
| **A03:2021** | Injection             | Parametrización total en SQLite y validación de tipos en Serde.                      |
| **A04:2021** | Insecure Design       | Fallback seguro en procesos Sidecar (Tor) y manejo de errores genéricos hacia la UI. |
| **A07:2021** | Identification & Auth | Uso de variables de entorno `.env` para llaves de API externas (HIBP, VirusTotal).   |

---

_Ultima revisión manual: 2026-02-16 por Antigravity (Auditoría Integral)._
