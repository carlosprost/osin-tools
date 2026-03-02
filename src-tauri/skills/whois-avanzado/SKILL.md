---
description: Realizar un análisis exhaustivo de propiedades de dominio utilizando Whois y cruce de datos.
---

# 🕵️‍♂️ Skill: OSINT de Dominio y Whois Avanzado

Esta skill te da la capacidad de analizar dominios más allá del simple registro, buscando conexiones y pivotes.

## ¿Cuándo usar esta skill?

- Cuando el usuario te pide investigar un dominio (ej: `ejemplo.com`).
- Cuando necesitas encontrar al dueño de una página web, correos electrónicos asociados o servidores de nombres.

## Proceso de Investigación (Pivoting)

1.  **Ejecución Inicial:** Ejecuta la herramienta de terminal `ejecutar_herramienta_linux` con el comando `whois [dominio]`.
2.  **Análisis de Resultados:**
    - Busca correos electrónicos (ej: `Registrant Email`, `Admin Email`).
    - Busca nombres de personas u organizaciones (ej: `Registrant Name`, `Registrant Organization`).
    - Busca servidores DNS (`Name Server`).
3.  **Persistencia Inmediata:** Usa la herramienta `guardar_hallazgo` para almacenar los datos clave estructurados bajo el objetivo (el dominio). No incluyas todo el texto, solo lo relevante.
4.  **Pivote (Si hay hallazgos):**
    - Si encuentras un correo electrónico, infórmale al usuario y pregúntale si desea que lo investigues como un nuevo objetivo (Pivote hacia persona/email).
    - Si encuentras un Name Server sospechoso (no comercial como Cloudflare/AWS), investiga su IP mediante ping o dns.

## Reglas Críticas ⚠️

- NO repitas el comando `whois` sobre el mismo dominio si ya tienes los datos en el Tablero de Hechos (Contexto).
- Sanitiza siempre la información antes de guardarla o mostrarla al usuario.
