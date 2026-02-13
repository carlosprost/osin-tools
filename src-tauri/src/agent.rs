// src-tauri/src/agent.rs
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;
use std::env;

// --- Estructuras para la API de Gemini ---

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ToolCall {
    pub tool_name: String,
    pub arguments: HashMap<String, String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum AgentResponse {
    Text(String),
    Tool(ToolCall),
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
            Si el usuario te pide escanear algo, USA LAS HERRAMIENTAS. \
            Responde siempre en Español."
                .to_string(),
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

    pub async fn think(&self, last_input: &str) -> AgentResponse {
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

        // 3. Agregar el input actual
        contents.push(json!({
            "role": "user",
            "parts": [{ "text": last_input }]
        }));

        // Definir herramientas
        let tools = json!([{
            "function_declarations": [
                {
                    "name": "ping",
                    "description": "Comprueba la conectividad con un host (IP o dominio) usando ICMP ping.",
                    "parameters": {
                        "type": "OBJECT",
                        "properties": {
                            "target": {
                                "type": "STRING",
                                "description": "La dirección IP o nombre de dominio a contactar (ej: google.com, 8.8.8.8)"
                            }
                        },
                        "required": ["target"]
                    }
                },
                {
                    "name": "whois",
                    "description": "Obtiene información de registro de un dominio (WHOIS).",
                    "parameters": {
                        "type": "OBJECT",
                        "properties": {
                            "target": {
                                "type": "STRING",
                                "description": "El nombre de dominio a consultar (ej: google.com)"
                            }
                        },
                        "required": ["target"]
                    }
                },
                 {
                    "name": "dns",
                    "description": "Realiza una búsqueda de DNS para resolver IPs de un dominio.",
                    "parameters": {
                        "type": "OBJECT",
                        "properties": {
                            "target": {
                                "type": "STRING",
                                "description": "El dominio a resolver"
                            }
                        },
                        "required": ["target"]
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
                    return AgentResponse::Error(format!("Error de API Gemini: {}", err_text));
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

                                        return AgentResponse::Tool(ToolCall {
                                            tool_name: name,
                                            arguments,
                                        });
                                    }

                                    // 2. Si no, devolver texto
                                    if let Some(text) = part.get("text").and_then(|t| t.as_str()) {
                                        return AgentResponse::Text(text.to_string());
                                    }
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
