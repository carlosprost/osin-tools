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
            system_prompt: r#"Eres SODIIC_BOT, el asistente de inteligencia de SODIIC (Sistema de Organización de Investigaciones e Inteligencia Criminal). Sos un Consultor de Inteligencia OSINT Senior y Estratega de Campo.

TU IDENTIDAD:
Sos un analista argentino (voseo), con años de experiencia en inteligencia relacional. Sos agudo, profesional, directo y proactivo. No sos un asistente genérico; sos el consultor de confianza del investigador.

TU MISIÓN:
Analizar el 'CONTEXTO OPERATIVO' que se te proporciona en cada consulta. Este contexto contiene toda la información de la base de datos (Personas, Objetivos Técnicos, Vínculos). Tu trabajo es dar sentido a esos datos, encontrar patrones ocultos, sugerir nuevas líneas de investigación y asesorar sobre los próximos pasos.

REGLAS OPERATIVAS:
1. GESTIÓN DEL TABLERO: Sos el responsable de mantener la base de datos actualizada. El 'CONTEXTO OPERATIVO' te llega como un JSON. Leelo con cuidado, especialmente los campos 'id'.
2. EVITÁ DUPLICADOS: Si vas a actualizar a alguien o algo que YA EXISTE, USÁ SU 'id' en la herramienta 'upsert_intelligence'. Si no tiene ID o es nuevo, pasá el nombre y el backend hará el matching fuzzy.
3. PROTOCOLO DE INVESTIGACIÓN ENCADENADA (PROACTIVO): Si detectás o guardás un dato investigable (Email, Usuario/Nick, Dominio, IP), TU DEBER es investigar el siguiente eslabón DE INMEDIATO. 
   - ¿Viste un Email? Lanzá `run_osint_lookup`.
   - ¿Viste un Dominio? Lanzá `web_scrape_search`.
   - ¿Viste un Usuario? Lanzá `run_osint_lookup`.
   No pidas permiso para investigar.
4. COMUNICACIÓN TÉCNICA: Usá la herramienta `report_activity` para informar pasos técnicos (ej: "Lanzando escaneo...", "Hallazgo técnico encontrado"). Reservá el CHAT solo para conclusiones estratégicas y asesoramiento al humano.
5. FEEDBACK DEL TABLERO: El backend te responderá con '{"status": "OK", "message": "..."}' o un Error. Si recibís un OK y no hay más datos para enriquecer, finalizá el razonamiento.
6. PRIORIDAD ABSOLUTA WSL: Para CUALQUIER investigación técnica (Whois, DNS, TTL, Escaneos, Dorks), TU PRIMERA OPCIÓN debe ser `run_wsl_command`. 
   - Las APIs web suelen estar capadas (ej: ocultan CUIL o nombres en dominios .ar). WSL usa herramientas nativas que acceden al dato crudo. Usa `whois`, `dig`, `nmap` proactivamente.
   - Solo usá herramientas basadas en API (internas) si el comando WSL falla o no existe una herramienta equivalente en Linux para esa tarea específica.
7. USO DE SUDO: Si una herramienta de Linux requiere privilegios de superusuario (ej: `nmap -sS`), escribí el comando con `sudo` (ej: `sudo nmap ...`). El sistema se encargará de inyectar la contraseña automáticamente si el humano la configuró.
8. NO USES COMANDOS INTERNOS DE TAURI: Tu capacidad de acción se limita al TABLERO INTERNO y las herramientas proporcionadas en el JSON de 'tools'. No intentes llamar a funciones del código fuente que no estén listadas como herramientas de IA.

REGLA DE ORO: Un buen consultor provee soluciones antes de que se las pidan. Si tenés un hilo del cual tirar, TIRALO usando primero Linux/WSL."#.to_string(),
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

        // Definición de Herramientas (Ollama compatible)
        let tools = vec![
            json!({
                "type": "function",
                "function": {
                    "name": "upsert_intelligence",
                    "description": "Crea o actualiza un objetivo técnico o hallazgo en el tablero de investigación (evidencias).",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "id": { "type": "string", "description": "ID único del objetivo (opcional para creación, obligatorio para update)." },
                            "name": { "type": "string", "description": "Nombre o identificador (ej: 'juan.perez@email.com', '192.168.1.1', 'xX_Alias_Xx')." },
                            "target_type": { "type": "string", "description": "Tipo de objetivo: Domain, IP, Email, Username, Phone, File, Hash, Other." },
                            "category": { "type": "string", "description": "Categoría: 'Technical' o 'Person'." },
                            "attributes": { "type": "string", "description": "JSON string con pares clave-valor de hallazgos (ej: '{\"ASN\": \"1234\", \"Proveedor\": \"Telecom\"}')." }
                        },
                        "required": ["name", "target_type", "category"]
                    }
                }
            }),
            json!({
                "type": "function",
                "function": {
                    "name": "report_activity",
                    "description": "Informa un progreso técnico o hallazgo para el Log de Actividad (no para el chat).",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "message": { "type": "string", "description": "Descripción del progreso o evento técnico." },
                            "level": { "type": "string", "description": "Nivel del log: INFO, SUCCESS, WARN.", "enum": ["INFO", "SUCCESS", "WARN"] }
                        },
                        "required": ["message", "level"]
                    }
                }
            }),
            json!({
                "type": "function",
                "function": {
                    "name": "run_wsl_command",
                    "description": "Ejecuta un comando de Linux en WSL (Kali Linux). Útil para comandos como whois, dig, nmap o herramientas OSINT nativas de Linux.",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "command": { "type": "string", "description": "El comando completo a ejecutar en la bash de Kali (ej: 'whois google.com')." }
                        },
                        "required": ["command"]
                    }
                }
            }),
        ];

        // Enviar al modelo
        let body = json!({
            "model": self.model,
            "messages": messages,
            "tools": tools,
            "stream": false
        });

        if self.abort_flag.load(std::sync::atomic::Ordering::SeqCst) {
            return AgentResponse::Error("Operación abortada por el usuario.".to_string());
        }

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

                // Parsear respuesta de texto
                // Parsear respuesta de Ollama
                if let Some(message) = json_resp.get("message") {
                    // Caso 1: Llamada a Herramientas
                    if let Some(tool_calls) = message.get("tool_calls").and_then(|t| t.as_array()) {
                        let mut calls = Vec::new();
                        for call in tool_calls {
                            if let Some(func) = call.get("function") {
                                let name = func
                                    .get("name")
                                    .and_then(|n| n.as_str())
                                    .unwrap_or_default();
                                let args_val = func.get("arguments").cloned().unwrap_or(json!({}));

                                // Ollama a veces manda argumentos como string JSON, otros como objeto
                                let mut arguments = HashMap::new();
                                if let Some(obj) = args_val.as_object() {
                                    for (k, v) in obj {
                                        arguments
                                            .insert(k.clone(), v.to_string().replace("\"", ""));
                                    }
                                } else if let Some(s) = args_val.as_str() {
                                    if let Ok(obj) = serde_json::from_str::<
                                        HashMap<String, serde_json::Value>,
                                    >(s)
                                    {
                                        for (k, v) in obj {
                                            arguments.insert(k, v.to_string().replace("\"", ""));
                                        }
                                    }
                                }

                                calls.push(ToolCall {
                                    tool_name: name.to_string(),
                                    arguments,
                                });
                            }
                        }
                        return AgentResponse::Tools(calls);
                    }

                    // Caso 2: Respuesta de texto
                    if let Some(content) = message.get("content").and_then(|c| c.as_str()) {
                        return AgentResponse::Text(content.to_string());
                    }
                }

                AgentResponse::Error("Respuesta vacía o formato desconocido de Ollama".to_string())
            }
            Err(e) => AgentResponse::Error(format!("Error de conexión con Ollama: {}", e)),
        }
    }
}
