// src-tauri/src/skills.rs
//
// Sistema de Skills OSINT — inspirado en el workspace skills de OpenClaw.
// Cada skill es un directorio con un archivo SKILL.md que el agente puede leer
// antes de responder para seguir instrucciones especializadas.
//
// Estructura esperada:
//   src-tauri/skills/
//     whois-avanzado/
//       SKILL.md
//     persona-investigacion/
//       SKILL.md
//     red-mapping/
//       SKILL.md

use std::fs;
use std::path::{Path, PathBuf};

/// Representa una skill disponible para el agente
#[derive(Debug, Clone)]
pub struct Skill {
    /// Identificador único (nombre del directorio)
    pub id: String,
    /// Descripción breve extraída del frontmatter del SKILL.md
    pub description: String,
    /// Ruta al archivo SKILL.md
    pub path: PathBuf,
}

/// Carga todas las skills disponibles desde el directorio `skills/` relativo al ejecutable.
/// Si el directorio no existe, devuelve una lista vacía (no falla).
pub fn load_skills(skills_base_dir: &Path) -> Vec<Skill> {
    if !skills_base_dir.exists() {
        return Vec::new();
    }

    let mut skills = Vec::new();

    let entries = match fs::read_dir(skills_base_dir) {
        Ok(e) => e,
        Err(_) => return Vec::new(),
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }

        let skill_file = path.join("SKILL.md");
        if !skill_file.exists() {
            continue;
        }

        let content = match fs::read_to_string(&skill_file) {
            Ok(c) => c,
            Err(_) => continue,
        };

        let description = parse_description_from_skill_md(&content);
        let id = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string();

        skills.push(Skill {
            id,
            description,
            path: skill_file,
        });
    }

    // Ordenar alfabéticamente para output determinístico
    skills.sort_by(|a, b| a.id.cmp(&b.id));
    skills
}

/// Extrae la descripción del frontmatter YAML del SKILL.md.
/// Busca la línea `description: "..."` en el bloque `---`.
/// Si no encuentra frontmatter, usa la primera línea no vacía del contenido.
fn parse_description_from_skill_md(content: &str) -> String {
    let lines: Vec<&str> = content.lines().collect();
    let mut in_frontmatter = false;

    for (i, &line) in lines.iter().enumerate() {
        if i == 0 && line.trim() == "---" {
            in_frontmatter = true;
            continue;
        }
        if in_frontmatter {
            if line.trim() == "---" {
                break; // Fin del frontmatter
            }
            if let Some(desc) = parse_frontmatter_field(line, "description") {
                return desc;
            }
        }
    }

    // Fallback: primera línea no vacía que no sea `#` heading
    lines
        .iter()
        .find(|&&l| {
            !l.trim().is_empty() && !l.trim().starts_with('#') && !l.trim().starts_with("---")
        })
        .map(|l| l.trim().to_string())
        .unwrap_or_else(|| "Skill OSINT".to_string())
}

/// Parsea un campo YAML simple: `key: "value"` o `key: value`
fn parse_frontmatter_field(line: &str, key: &str) -> Option<String> {
    let prefix = format!("{}:", key);
    if !line.trim_start().starts_with(&prefix) {
        return None;
    }
    let value = line.trim_start()[prefix.len()..].trim();
    // Remover comillas opcionales
    let value = value.trim_matches('"').trim_matches('\'').to_string();
    if value.is_empty() {
        None
    } else {
        Some(value)
    }
}

/// Construye el bloque de skills para inyectar en el system prompt del agente.
/// Formato similar a OpenClaw: lista de skills disponibles con sus descripciones.
pub fn build_skills_prompt_section(skills: &[Skill]) -> String {
    if skills.is_empty() {
        return String::new();
    }

    let mut section = String::from("## Skills Disponibles (Protocolos Especializados)\n");
    section.push_str("Cuentas con las siguientes habilidades OSINT. Sigue estrictamente sus procesos cuando el contexto lo requiera:\n\n");
    section.push_str("<skills_disponibles>\n");

    for skill in skills {
        let base_dir = skill.path.parent().unwrap().parent().unwrap();
        let content =
            read_skill_content(base_dir, &skill.id).unwrap_or_else(|| skill.description.clone());

        section.push_str(&format!(
            "  <skill id=\"{}\">\n    <descripcion>{}</descripcion>\n    <contenido>\n{}\n    </contenido>\n  </skill>\n",
            skill.id,
            skill.description,
            content
        ));
    }

    section.push_str("</skills_disponibles>\n");
    section
}

/// Lee el contenido completo de una skill por su ID (para devolverlo al agente cuando lo solicite o inyectarlo).
#[allow(dead_code)]
fn read_skill_content(skills_base_dir: &Path, skill_id: &str) -> Option<String> {
    let skill_file = skills_base_dir.join(skill_id).join("SKILL.md");
    fs::read_to_string(&skill_file).ok()
}
