// src-tauri/src/agent.rs
use base64::{engine::general_purpose, Engine as _};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;
use std::fs;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

// --- Estructuras para la API de Ollama (Compatible con OpenAI) ---

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ToolCall {
    pub tool_name: String,
    pub arguments: HashMap<String, String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum AgentResponse {
    Text(String),
    Tools(Vec<ToolCall>),
    Error(String),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AgentMessage {
    pub role: String, // "user", "assistant", "system"
    pub content: String,
}

#[allow(dead_code)]
pub struct Agent {
    pub system_prompt: String,
    pub history: Vec<AgentMessage>,
    pub model: String,
    pub client: Client,
    pub abort_flag: Arc<AtomicBool>,
}

#[allow(dead_code)]
impl Agent {
    pub fn new() -> Self {
        Agent {
            system_prompt: "Eres el Agente OSINT, un Analista de Inteligencia Relacional experto. \
            \
            OBJETIVO PRINCIPAL: Asistir al investigador en la recolección y análisis de datos. \
            \
            DIRECTIVAS DE COMPORTAMIENTO: \
            1. REACTIVIDAD ABSOLUTA: Si el usuario te saluda o conversa, RESPONDE con texto. NO uses herramientas si no te han pedido una tarea específica. \
            2. USO DE HERRAMIENTAS: Solo ejecuta comandos (como 'ping', 'whois', 'manage_target') cuando sean necesarios para cumplir una orden del usuario. \
            3. MEMORIA Y CONTEXTO: Si necesitás consultar datos previos, hacelo en silencio. No satures el chat con reportes técnicos a menos que sean el resultado solicitado. \
            4. ESTILO: Usá español rioplatense (voseo), sé profesional pero cercano. Al grano, sin vueltas. \
            \
            Si el usuario dice 'hola', devolvé un saludo cordial y preguntá qué investigar. NO busques objetivos ni inventes tareas.".to_string(),
            history: Vec::new(),
            model: "llama3.2".to_string(),
            client: Client::new(),
            abort_flag: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn add_message(&mut self, role: &str, content: &str) {
        // Mapear roles de Gemini/Internos a Ollama
        let ollama_role = match role {
            "model" => "assistant",
            "assistant" => "assistant",
            "system" => "system",
            _ => "user",
        };

        self.history.push(AgentMessage {
            role: ollama_role.to_string(),
            content: content.to_string(),
        });
    }

    pub fn add_function_response(&mut self, tool_name: &str, content: &str) {
        // En Ollama, la respuesta de una tool se puede inyectar como un mensaje del usuario
        // explicando el resultado, para que el modelo lo procese en la siguiente iteración.
        self.history.push(AgentMessage {
            role: "user".to_string(),
            content: format!("Resultado de la herramienta '{}':\n{}", tool_name, content),
        });
    }

    pub async fn think(
        &self,
        last_input: Option<&str>,
        image_path: Option<&str>,
        context_data: Option<&str>,
    ) -> AgentResponse {
        let url = "http://localhost:11434/api/chat";

        // Construir historial de mensajes
        let mut messages = Vec::new();

        // 1. System Prompt + Contexto Dinámico
        let final_system_prompt = if let Some(ctx) = context_data {
            format!("{}\n\nCONTEXTO OPERATIVO DEL CASO:\n{}\n\nNOTA: Usá este contexto para entender relaciones. Si NO hay objetivos listados, ESPERÁ instrucciones.", self.system_prompt, ctx)
        } else {
            self.system_prompt.clone()
        };

        messages.push(json!({
            "role": "system",
            "content": final_system_prompt
        }));

        // 2. Historial previo
        for msg in &self.history {
            messages.push(json!({
                "role": msg.role,
                "content": msg.content
            }));
        }

        // 3. Input actual + Imagen (si existe)
        if let Some(input) = last_input {
            let mut message_obj = json!({
                "role": "user",
                "content": input
            });

            // Manejo de imágenes (Ollama soporta array "images" con base64)
            if let Some(path) = image_path {
                if let Ok(bytes) = fs::read(path) {
                    let b64 = general_purpose::STANDARD.encode(&bytes);
                    if let Some(obj) = message_obj.as_object_mut() {
                        obj.insert("images".to_string(), json!([b64]));
                    }
                } else {
                    // Si falla leer imagen, no rompemos el flujo, solo enviamos texto
                    // println!("Error leyendo imagen: {}", path);
                }
            }
            messages.push(message_obj);
        } else if let Some(path) = image_path {
            // Caso solo imagen (raro pero posible)
            if let Ok(bytes) = fs::read(path) {
                let b64 = general_purpose::STANDARD.encode(&bytes);
                messages.push(json!({
                    "role": "user",
                    "content": "Analiza esta imagen.",
                    "images": [b64]
                }));
            }
        }

        // Definición de Herramientas (OpenAI compatible)
        let tools = vec![
            json!({
                "type": "function",
                "function": {
                    "name": "ping",
                    "description": "Comprueba la conectividad con un host.",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "target": { "type": "string", "description": "Host o IP" }
                        },
                        "required": ["target"]
                    }
                }
            }),
            json!({
                "type": "function",
                "function": {
                    "name": "whois",
                    "description": "Obtiene información de registro de dominio.",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "target": { "type": "string", "description": "Dominio" }
                        },
                        "required": ["target"]
                    }
                }
            }),
            // ... (Abreviado para limpieza, añadiré todas las tools)
            json!({
                "type": "function",
                "function": {
                    "name": "dns",
                    "description": "Busca registros DNS/IP.",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "target": { "type": "string", "description": "Dominio" }
                        },
                        "required": ["target"]
                    }
                }
            }),
            json!({
                "type": "function",
                "function": {
                    "name": "web_scrape_search",
                    "description": "Busca información en la web sobre una persona o tema.",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "query": { "type": "string", "description": "Consulta de búsqueda" }
                        },
                        "required": ["query"]
                    }
                }
            }),
            json!({
                "type": "function",
                "function": {
                    "name": "browse_url",
                    "description": "Visita una URL con un navegador y extrae el texto.",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "url": { "type": "string", "description": "URL completa" }
                        },
                        "required": ["url"]
                    }
                }
            }),
            json!({
                "type": "function",
                "function": {
                    "name": "generate_dorks",
                    "description": "Genera Google Dorks para investigar.",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "target": { "type": "string", "description": "Nombre objetivo" }
                        },
                        "required": ["target"]
                    }
                }
            }),
            json!({
                "type": "function",
                "function": {
                    "name": "social_search",
                    "description": "Barrido en redes sociales principales.",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "target": { "type": "string", "description": "Nombre o Alias" }
                        },
                        "required": ["target"]
                    }
                }
            }),
            json!({
                "type": "function",
                "function": {
                    "name": "search_username",
                    "description": "Busca nombre de usuario en múltiples plataformas.",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "username": { "type": "string", "description": "Username" }
                        },
                        "required": ["username"]
                    }
                }
            }),
            json!({
                "type": "function",
                "function": {
                    "name": "search_leaks",
                    "description": "Busca en filtraciones de datos.",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "target": { "type": "string", "description": "Email o Username" }
                        },
                        "required": ["target"]
                    }
                }
            }),
            json!({
                "type": "function",
                "function": {
                    "name": "dark_search",
                    "description": "Busca en la Dark Web (.onion).",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "query": { "type": "string", "description": "Consulta" }
                        },
                        "required": ["query"]
                    }
                }
            }),
            json!({
                "type": "function",
                "function": {
                    "name": "ip_intel",
                    "description": "Información geográfica e ISP de IP.",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "ip": { "type": "string", "description": "IP" }
                        },
                        "required": ["ip"]
                    }
                }
            }),
            json!({
                "type": "function",
                "function": {
                    "name": "shodan_intel",
                    "description": "Consulta Shodan para puertos y vulnerabilidades.",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "ip": { "type": "string", "description": "IP" }
                        },
                        "required": ["ip"]
                    }
                }
            }),
            json!({
                "type": "function",
                "function": {
                    "name": "manage_target",
                    "description": "Crea o actualiza una ficha de inteligencia para un objetivo (Persona, Dominio, IP, Email). Úsalo para guardar hallazgos confirmados.",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "name": { "type": "string", "description": "Nombre del objetivo o dominio" },
                            "target_type": { "type": "string", "enum": ["Person", "Domain", "IP", "Email", "Other"], "description": "Tipo de objetivo" },
                            "key": { "type": "string", "description": "Clave del dato (ej: 'IP', 'DNS', 'Empresa')" },
                            "value": { "type": "string", "description": "Valor del dato" },
                            "category": { "type": "string", "enum": ["Technical", "Personal"], "description": "Categoría del dato" }
                        },
                        "required": ["name", "target_type", "key", "value", "category"]
                    }
                }
            }),
            json!({
                "type": "function",
                "function": {
                    "name": "get_targets",
                    "description": "Obtiene la lista de objetivos (fichas) guardados en esta investigación.",
                    "parameters": {
                        "type": "object",
                        "properties": {},
                        "required": []
                    }
                }
            }),
            json!({
                "type": "function",
                "function": {
                    "name": "link_targets",
                    "description": "Establece un vínculo relacional entre dos objetivos (ej: 'Dueño de', 'Publicado en').",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "source_id": { "type": "string", "description": "ID del objetivo origen" },
                            "target_id": { "type": "string", "description": "ID del objetivo destino" },
                            "relation": { "type": "string", "description": "Tipo de relación (ej: 'Vínculo técnico', 'Asociado a')" }
                        },
                        "required": ["source_id", "target_id", "relation"]
                    }
                }
            }),
        ];

        let body = json!({
            "model": self.model,
            "messages": messages,
            "stream": false,
            "tools": tools
        });

        let resp = self.client.post(url).json(&body).send().await;

        match resp {
            Ok(r) => {
                if !r.status().is_success() {
                    let err_text = r.text().await.unwrap_or_default();
                    return AgentResponse::Error(format!("Ollama API Error: {}", err_text));
                }

                let json_resp: serde_json::Value = match r.json().await {
                    Ok(v) => v,
                    Err(e) => {
                        return AgentResponse::Error(format!(
                            "Error parseando JSON de Ollama: {}",
                            e
                        ))
                    }
                };

                // Parsear respuesta (formato OpenAI compatible de Ollama)
                if let Some(message) = json_resp.get("message") {
                    // 1. Tool Calls
                    if let Some(tool_calls) = message.get("tool_calls").and_then(|tc| tc.as_array())
                    {
                        let mut parsed_calls = Vec::new();

                        for call in tool_calls {
                            if let Some(func) = call.get("function") {
                                let name = func
                                    .get("name")
                                    .and_then(|n| n.as_str())
                                    .unwrap_or("unknown")
                                    .to_string();
                                let args_val = func.get("arguments");

                                let mut arguments = HashMap::new();
                                if let Some(args_obj) = args_val.and_then(|a| a.as_object()) {
                                    for (k, v) in args_obj {
                                        arguments.insert(
                                            k.clone(),
                                            v.as_str().unwrap_or(&v.to_string()).to_string(),
                                        );
                                    }
                                }

                                parsed_calls.push(ToolCall {
                                    tool_name: name,
                                    arguments,
                                });
                            }
                        }

                        if !parsed_calls.is_empty() {
                            return AgentResponse::Tools(parsed_calls);
                        }
                    }

                    // 2. Content (Check for JSON tool calls if explicit tool_calls are missing)
                    if let Some(content) = message.get("content").and_then(|c| c.as_str()) {
                        let fallback_tools = Self::parse_tools_from_content(content);

                        if !fallback_tools.is_empty() {
                            return AgentResponse::Tools(fallback_tools);
                        }

                        return AgentResponse::Text(content.to_string());
                    }
                }

                AgentResponse::Error("Respuesta vacía o formato desconocido de Ollama".to_string())
            }
            Err(e) => AgentResponse::Error(format!("Error de conexión con Ollama: {}", e)),
        }
    }

    pub fn parse_tools_from_content(content: &str) -> Vec<ToolCall> {
        let mut fallback_tools = Vec::new();
        let mut remaining = content;
        while let Some(start) = remaining.find('{') {
            let mut brace_count = 0;
            let mut end = None;

            for (i, c) in remaining[start..].char_indices() {
                if c == '{' {
                    brace_count += 1;
                } else if c == '}' {
                    brace_count -= 1;
                }

                if brace_count == 0 {
                    end = Some(start + i);
                    break;
                }
            }

            if let Some(e) = end {
                let json_str = &remaining[start..=e];
                if let Ok(json_val) = serde_json::from_str::<serde_json::Value>(json_str) {
                    let name = json_val
                        .get("name")
                        .or(json_val.get("tool"))
                        .and_then(|n| n.as_str());
                    if let Some(tool_name) = name {
                        let mut arguments = HashMap::new();
                        let args_src = json_val
                            .get("args")
                            .or(json_val.get("parameters"))
                            .or(json_val.get("arguments"));
                        if let Some(args_obj) = args_src.and_then(|a| a.as_object()) {
                            for (k, v) in args_obj {
                                arguments.insert(
                                    k.clone(),
                                    v.as_str().unwrap_or(&v.to_string()).to_string(),
                                );
                            }
                        }
                        fallback_tools.push(ToolCall {
                            tool_name: tool_name.to_string(),
                            arguments,
                        });
                    }
                }
                remaining = &remaining[e + 1..];
            } else {
                break;
            }
        }
        fallback_tools
    }
}
