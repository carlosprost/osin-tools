---
description: Procedimiento para mapear, identificar y evaluar infraestructuras de red asociadas a objetivos.
---

# 🌐 Skill: Mapeo de Red (Network Mapping)

Esta skill te da la capacidad de identificar la infraestructura tecnológica real detrás de dominios, IPs y servicios.

## ¿Cuándo usar esta skill?

- Cuando ya tienes un dominio o una IP validada en el tablero.
- Cuando el usuario solicita "saber qué puertos están abiertos", "ver dónde está alojado", o "descubrir subdominios".

## Proceso Analítico

1.  **Validación de Rango:** Confirma si el objetivo es un servidor dedicado propio, o infraestructura compartida (ej: un Shared Hosting de GoDaddy o un nodo de Cloudflare).
    - Usa la herramienta `ejecutar_herramienta_linux` con el comando `ping -c 4 [objetivo]`.
2.  **Mapeo de Topología:**
    - Ejecuta `nslookup` o `dig` para identificar registros A, AAAA, MX o TXT.
    - Usa la herramienta `nmap -F [ip/dominio]` para hacer un escaneo rápido (Fast) de puertos comunes si es necesario identificar servicios.
3.  **Registro y Correlación:**
    - Si el IP pertenece a Google, Amazon AWS o Cloudflare, anótalo rápido usando `guardar_hallazgo` con el detalle "Infraestructura Cloud Provider".
    - Si descubres puertos sospechosos abiertos (como RDP 3389, FTP 21, SSH 22, Telnet 23), adviérteselo al usuario en el chat, marcándolo como _"Punto Crítico de Entrada"_.

## Reglas Críticas ⚠️

- NO realices escaneos intrusivos, de fuerza bruta (`-p-`), ni escaneo de vulnerabilidades (`--script vuln`). Tu objetivo es el Mapeo pasivo e Identificación de huellas (Footprinting). SODIIC no es una herramienta ofensiva.
- Procura no levantar alarmas innecesarias en los IDS/IPS ajenos; prefiere escaneos rápidos y sigilosos.
