---
description: Procedimiento estandarizado para la recolección, cruce y perfilado de identidades en investigaciones OSINT.
---

# 👱‍♂️ Skill: Investigación de Personas (Perfilado)

Esta skill te da la capacidad de organizar la investigación sobre individuos, buscando correlacionar alias, correos, teléfonos y registros públicos.

## ¿Cuándo usar esta skill?

- Cuando el usuario te da un nombre, DNI, correo, o número de teléfono.
- Cuando necesitas estructurar la información de un sujeto que ha aparecido como subproducto de otra investigación (ej: a través del Whois de una empresa).

## Proceso de Inteligencia

1.  **Deduplicación:** Busca si la persona ya existe en el `Tablero de Hechos` del contexto. Si ya existe, NO crees una nueva; utiliza las herramientas para actualizar sus datos.
2.  **Extracción Cruzada:**
    - Si tienes el **Email**, busca alias comúnes relacionados (fuga de datos, dorks). Ejecuta herramientas en WSL (`holehe`, `sherlock`) si tienes autorización del usuario.
    - Si tienes el **Teléfono**, busca coincidencias en formato internacional.
    - Si tienes el **Nombre Completo**, busca en redes sociales comunes o registros corporativos (ej: LinkedIn, Boletines Oficiales).
3.  **Registro Estructurado:** Usa `guardar_hallazgo` o las herramientas específicas de personas (`crear_persona`, `actualizar_persona`) para asentar la huella digital en la base de datos de SODIIC.
    - No agregues información adivinada o supuesta (Alucinación).
    - Registra siempre la fuente de la información cruzada en un comentario técnico (`registrar_actividad_tecnica`).

## Reglas Críticas ⚠️

- Respeta la privacidad legal ("Minimum Necessary"). Sólo procesa la información relevante al caso.
- No intentes adivinar contraseñas. SODIIC es pasivo.
