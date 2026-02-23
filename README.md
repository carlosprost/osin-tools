# SODIIC - Sistema de Organización de Investigaciones e Inteligencia Criminal 🕵️‍♂️🇦🇷

Bienvenido al ecosistema de inteligencia relacional más avanzado. Este dashboard permite realizar investigaciones OSINT automatizadas, gestionar objetivos y mapear vínculos utilizando un Agente IA autónomo.

## 🚀 Características Principales

- **Agente Ninja Autónomo**: Un analista de inteligencia que conecta puntos, realiza búsquedas y genera reportes estratégicos por vos.
- **Inteligencia Relacional**: Base de datos SQLite dedicada por investigación para mapear vínculos entre Personas, Dominios, IPs y Emails.
- **Privacidad Total**: Integración nativa con Tor (Sidecar) y enmascaramiento de MAC address.
- **Herramientas Avanzadas**: Google Dorks, Web Scraping, Filtraciones (HIBP), VirusTotal, Shodan y Reconocimiento Facial (en desarrollo).

## 🛠️ Stack Tecnológico

- **Frontend**: Svelte (Signals, BEM, SoC).
- **Backend**: Rust (Tauri v2.0).
- **Base de Datos**: SQLite (Relacional).
- **IA**: Ollama (Llama 3.2 local).

## 🗂️ Estructura del Proyecto

- `src-tauri/src/agent.rs`: El cerebro del analista IA.
- `src-tauri/src/cases.rs`: Gestión de investigaciones y persistencia relacional.
- `src-tauri/src/commands.rs`: Controlador principal de herramientas y flujo de datos.
- `src-tauri/src/tools.rs`: Implementación técnica de herramientas OSINT.
- `src/components/`: Interfaz moderna y dinámica construida con Svelte.

## 📋 Manual de Uso del Agente

Para interactuar con el Agente, simplemente usá lenguaje natural:

- _"Investigá el dominio wolftei.com y sacá todo lo técnico que encuentres."_
- _"Encontré este email: info@ejemplo.com. ¿Podés vincularlo a alguna persona?"_
- _"Hacé un reporte ejecutivo de la investigación hasta ahora."_

## 🛡️ Seguridad

Consultá el [SECURITY_REPORT.md](./SECURITY_REPORT.md) para detalles sobre estándares ISO y mitigación OWASP.

---

_Desarrollado con pasión técnica y precisión judicial._
