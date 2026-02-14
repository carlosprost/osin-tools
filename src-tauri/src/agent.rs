// src-tauri/src/agent.rs
use base64::{engine::general_purpose, Engine as _};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;
use std::env;
use std::fs;

// --- Estructuras para la API de Gemini ---

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ToolCall {
    pub tool_name: String,
    pub arguments: HashMap<String, String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum AgentResponse {
    Text(String),
    Tools(Vec<ToolCall>), // Changed from Tool(ToolCall) to Tools(Vec<ToolCall>)
    Error(String),
}

#[derive(Serialize, Deserialize, Debug, Clone)] // Agregado Clone
pub struct AgentMessage {
    pub role: String, // "user", "model" (Gemini usa "model", no "assistant")
    pub content: String,
}

#[allow(dead_code)]
pub struct Agent {
    pub system_prompt: String,
    pub history: Vec<AgentMessage>,
    pub api_key: String,
    pub client: Client,
}

#[allow(dead_code)]
impl Agent {
    pub fn new() -> Self {
        let api_key = env::var("GEMINI_API_KEY").unwrap_or_default();
        Agent {
            system_prompt: "Eres un experto analista de OSINT en una aplicación de tablero. \
            Tienes acceso a herramientas para investigar objetivos. \
            Si el usuario te pide escanear algo, PUEDES USAR MULTIPLES HERRAMIENTAS SI ES NECESARIO (ping, whois, dns). \
            Cuando tengas los resultados, genera un REPORTE DETALLADO Y ESTRUCTURADO en Markdown. \
            Responde siempre en Español.".to_string(),
            history: Vec::new(),
            api_key,
            client: Client::new(),
        }
    }

    pub fn add_message(&mut self, role: &str, content: &str) {
        let gemini_role = if role == "assistant" { "model" } else { role };
        self.history.push(AgentMessage {
            role: gemini_role.to_string(),
            content: content.to_string(),
        });
    }

    pub fn add_function_response(&mut self, tool_name: &str, content: &str) {
        // En la API de Gemini (v1beta), las respuestas de funciones se añaden como 'functionResponse'
        // Pero para simplificar en este cliente REST, las añadiremos como mensajes 'user' con un formato especial
        // o idealmente como 'function' role si estuviéramos usando la estructura completa de `Content`.
        // Hack para prototipo: Simular que el sistema le entrega el resultado.
        self.history.push(AgentMessage {
            role: "user".to_string(), // Gemini a veces prefiere 'function', pero 'user' funciona para contexto
            content: format!("Resultado de la herramienta '{}':\n{}", tool_name, content),
        });
    }

    pub async fn think(&self, last_input: Option<&str>, image_path: Option<&str>) -> AgentResponse {
        if self.api_key.is_empty() || self.api_key.contains("TU_API_KEY") {
            return AgentResponse::Text(
                "⚠️ Error: No se encontró la API Key de Gemini. \
                Verifica el archivo src-tauri/.env y reinicia la app."
                    .to_string(),
            );
        }

        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/gemini-2.5-flash:generateContent?key={}",
            self.api_key
        );

        // Construir el historial para Gemini
        let mut contents = Vec::new();

        // 1. Agregar System Prompt (Gemini Flash soporta system instructions, pero para simpleza lo uniremos o usaremos hacks si es necesario.
        // La API v1beta soporta "system_instruction" en el body).

        // 2. Agregar historial
        for msg in &self.history {
            contents.push(json!({
                "role": msg.role,
                "parts": [{ "text": msg.content }]
            }));
        }

        // 3. Agregar el input actual + Imagen si existe
        if let Some(input) = last_input {
            let mut parts = Vec::new();

            // Texto del usuario
            parts.push(json!({ "text": input }));

            // Imagen adjunta?
            if let Some(path) = image_path {
                if let Ok(bytes) = fs::read(path) {
                    let b64 = general_purpose::STANDARD.encode(&bytes);
                    // Guess mime type simply by extension or default to jpeg
                    let mime = if path.to_lowercase().ends_with(".png") {
                        "image/png"
                    } else {
                        "image/jpeg"
                    };

                    parts.push(json!({
                        "inline_data": {
                            "mime_type": mime,
                            "data": b64
                        }
                    }));
                } else {
                    // Si falla leer, avisar en el texto? O ignorar?
                    // Mejor agregar un texto de sistema/error
                    parts.push(json!({ "text": "[Error leyendo la imagen adjunta]" }));
                }
            }

            contents.push(json!({
                "role": "user",
                "parts": parts
            }));
        } else if let Some(path) = image_path {
            // Caso raro: Imagen sin texto (solo adjuntar)
            // Gemini soporta multimodal prompt solo imagen? Si.
            let mut parts = Vec::new();
            if let Ok(bytes) = fs::read(path) {
                let b64 = general_purpose::STANDARD.encode(&bytes);
                let mime = if path.to_lowercase().ends_with(".png") {
                    "image/png"
                } else {
                    "image/jpeg"
                };
                parts.push(json!({
                    "inline_data": {
                        "mime_type": mime,
                        "data": b64
                    }
                }));
                // Gemini suele requerir algun prompt, aunque sea "Describe esto".
                // Pero si es 'user', está bien.
            }
            contents.push(json!({
                "role": "user",
                "parts": parts
            }));
        }

        // Definir herramientas
        let tools = json!([{
            "function_declarations": [
                {
                    "name": "ping",
                    "description": "Comprueba la conectividad con un host.",
                    "parameters": {
                        "type": "OBJECT",
                        "properties": {
                            "target": { "type": "STRING", "description": "Host o IP" }
                        },
                        "required": ["target"]
                    }
                },
                {
                    "name": "whois",
                    "description": "Obtiene información de registro de dominio.",
                    "parameters": {
                        "type": "OBJECT",
                        "properties": {
                            "target": { "type": "STRING", "description": "Dominio" }
                        },
                        "required": ["target"]
                    }
                },
                 {
                    "name": "dns",
                    "description": "Busca registros DNS/IP.",
                    "parameters": {
                        "type": "OBJECT",
                        "properties": {
                            "target": { "type": "STRING", "description": "Dominio" }
                        },
                        "required": ["target"]
                    }
                },
                {
                    "name": "extract_metadata",
                    "description": "Extrae metadatos EXIF de una imagen local.",
                    "parameters": {
                        "type": "OBJECT",
                        "properties": {
                            "path": { "type": "STRING", "description": "Ruta absoluta de la imagen" }
                        },
                        "required": ["path"]
                    }
                },
                {
                    "name": "reverse_image_search",
                    "description": "Abre la herramienta de búsqueda inversa de imágenes con la imagen especificada.",
                    "parameters": {
                        "type": "OBJECT",
                        "properties": {
                            "path": { "type": "STRING", "description": "Ruta absoluta de la imagen" }
                        },
                        "required": ["path"]
                    }
                },
                {
                    "name": "web_scrape_search",
                    "description": "Busca información en la web sobre una persona o tema",
                    "parameters": {
                        "type": "OBJECT",
                        "properties": {
                            "query": { "type": "STRING", "description": "Consulta de búsqueda (nombre, tema, etc)" }
                        },
                        "required": ["query"]
                    }
                },
                {
                    "name": "browse_url",
                    "description": "Visita una URL con un navegador real (Chrome headless) y extrae el texto visible de la página. Usa esto cuando necesites ver el contenido de una página web específica que requiere JavaScript o bloquea scrapers HTTP simples (ej: Facebook, LinkedIn, páginas dinámicas). NO uses esto para buscar en Google/DuckDuckGo — para eso usa web_scrape_search.",
                    "parameters": {
                        "type": "OBJECT",
                        "properties": {
                            "url": { "type": "STRING", "description": "URL completa a visitar (ej: https://facebook.com/zuck)" }
                        },
                        "required": ["url"]
                    }
                }
            ]
        }]);

        let body = json!({
            "system_instruction": {
                "parts": [{ "text": self.system_prompt }]
            },
            "contents": contents,
            "tools": tools
        });

        // Enviar Petición
        let resp = self.client.post(&url).json(&body).send().await;

        match resp {
            Ok(r) => {
                if !r.status().is_success() {
                    let err_text = r.text().await.unwrap_or_default();
                    return AgentResponse::Error(format!("API Error: {}", err_text));
                }

                let json_resp: serde_json::Value = match r.json().await {
                    Ok(v) => v,
                    Err(e) => return AgentResponse::Error(format!("Error parseando JSON: {}", e)),
                };

                // Analizar respuesta
                if let Some(candidates) = json_resp.get("candidates").and_then(|c| c.as_array()) {
                    if let Some(first) = candidates.first() {
                        if let Some(content) = first.get("content") {
                            if let Some(parts) = content.get("parts").and_then(|p| p.as_array()) {
                                let mut tool_calls = Vec::new();
                                let mut text_response = String::new();

                                for part in parts {
                                    // 1. Revisar si hay llamada a función
                                    if let Some(fc) = part.get("functionCall") {
                                        let name =
                                            fc["name"].as_str().unwrap_or("unknown").to_string();
                                        let args_val = &fc["args"];

                                        // Convertir args a HashMap<String, String> simplificado
                                        let mut arguments = HashMap::new();
                                        if let Some(obj) = args_val.as_object() {
                                            for (k, v) in obj {
                                                if let Some(s) = v.as_str() {
                                                    arguments.insert(k.clone(), s.to_string());
                                                } else {
                                                    arguments.insert(k.clone(), v.to_string());
                                                }
                                            }
                                        }

                                        tool_calls.push(ToolCall {
                                            tool_name: name,
                                            arguments,
                                        });
                                    }

                                    // 2. Si no, devolver texto
                                    if let Some(text) = part.get("text").and_then(|t| t.as_str()) {
                                        text_response.push_str(text);
                                    }
                                }

                                if !tool_calls.is_empty() {
                                    return AgentResponse::Tools(tool_calls);
                                } else if !text_response.is_empty() {
                                    return AgentResponse::Text(text_response);
                                }
                            }
                        }
                    }
                }

                AgentResponse::Error("Respuesta vacía o incomprensible de Gemini".to_string())
            }
            Err(e) => AgentResponse::Error(format!("Error de red: {}", e)),
        }
    }
}
