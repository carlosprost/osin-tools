use crate::skills::Skill;
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

#[derive(Default)]
pub struct PromptParams<'a> {
    pub case_context: Option<&'a str>,
    pub force_text_only: bool,
    pub skills: Option<&'a [Skill]>,
}

#[allow(dead_code)]
pub struct Agent {
    pub history: Vec<AgentMessage>,
    pub model: String,
    pub url: String,
    pub client: Client,
    pub abort_flag: Arc<AtomicBool>,
}

#[allow(dead_code)]
impl Agent {
    pub fn new() -> Self {
        Agent {
            history: Vec::new(),
            model: "llama3.2".to_string(),
            url: "http://localhost:11434".to_string(),
            client: Client::new(),
            abort_flag: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Determina si el modelo configurado soporta la API nativa de 'tools' de Ollama.
    /// DeepSeek-R1 y otros modelos de razonamiento puro suelen fallar con esta API.
    fn supports_native_tools(&self) -> bool {
        let m = self.model.to_lowercase();
        // Excluimos modelos conocidos por no soportar tools o ser reasoning-heavy
        if m.contains("deepseek-r1") || m.contains("thought") || m.contains("reasoning") {
            return false;
        }
        true
    }

    /// Retorna la definición de herramientas disponibles para el Agente.
    fn get_tools_definition(&self) -> Vec<serde_json::Value> {
        vec![
            json!({
                "type": "function",
                "function": {
                    "name": "guardar_hallazgo",
                    "description": "GUARDA datos nuevos en el tablero. OBLIGATORIO proveer 'name' y 'target_type'.",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "name": { "type": "string", "description": "Nombre/valor del hallazgo (ej: 'wolftei.com.ar' o 'Juan Perez')." },
                            "target_type": { "type": "string", "description": "Categoría: Domain, IP, Email, Person." },
                            "attributes": { "type": "string", "description": "JSON string con info extra (ej: '{\"ISP\": \"Telecom\", \"Registrant\": \"...\"}')." }
                        },
                        "required": ["name", "target_type"]
                    }
                }
            }),
            json!({
                "type": "function",
                "function": {
                    "name": "registrar_actividad_tecnica",
                    "description": "Log técnico INVISIBLE para el usuario. NO USAR para responder preguntas. Solo para auditoría interna de pasos técnicos.",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "message": { "type": "string", "description": "Descripción técnica (ej: 'Iniciando escaneo de puertos')." },
                            "level": { "type": "string", "enum": ["INFO", "SUCCESS", "WARN"] }
                        },
                        "required": ["message", "level"]
                    }
                }
            }),
            json!({
                "type": "function",
                "function": {
                    "name": "ejecutar_herramienta_linux",
                    "description": "Corre un comando real en Kali Linux (whois, nmap, etc).",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "command": { "type": "string", "description": "Comando bash completo." }
                        },
                        "required": ["command"]
                    }
                }
            }),
            json!({
                "type": "function",
                "function": {
                    "name": "scrape_generic",
                    "description": "Navega a un enlace o URL y extrae su texto limpio.",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "url": { "type": "string", "description": "URL completa http/https." }
                        },
                        "required": ["url"]
                    }
                }
            }),
            json!({
                "type": "function",
                "function": {
                    "name": "scrape_social",
                    "description": "Extrae info de perfiles públicos de LinkedIn, Instagram o X.",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "url": { "type": "string", "description": "URL del perfil." }
                        },
                        "required": ["url"]
                    }
                }
            }),
            json!({
                "type": "function",
                "function": {
                    "name": "ver_configuracion",
                    "description": "Muestra el estado actual de las APIs y servicios configurados (sin revelar contraseñas).",
                    "parameters": {
                        "type": "object",
                        "properties": {}
                    }
                }
            }),
            json!({
                "type": "function",
                "function": {
                    "name": "actualizar_configuracion",
                    "description": "Permite al agente cambiar un ajuste de la aplicación (ej: cambiar el modelo de Ollama o una API Key).",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "key": { "type": "string", "description": "Nombre del ajuste (ej: 'ollama_model', 'shodan', 'proxy_url')." },
                            "value": { "type": "string", "description": "Nuevo valor para el ajuste." }
                        },
                        "required": ["key", "value"]
                    }
                }
            }),
            json!({
                "type": "function",
                "function": {
                    "name": "obtener_ayuda",
                    "description": "Carga el manual interactivo para que el agente sepa quién es, qué hace y qué herramientas tiene disponibles.",
                    "parameters": {
                        "type": "object",
                        "properties": {}
                    }
                }
            }),
        ]
    }

    /// Construye dinámicamente el System Prompt como lo hace OpenClaw, por secciones.
    fn build_system_prompt(&self, params: &PromptParams) -> String {
        let mut prompt = String::new();

        // 1. Identidad
        prompt.push_str("Eres SODIIC_BOT, el consultor de inteligencia senior de SODIIC. Experto en OSINT y ciberseguridad.\n");
        prompt.push_str("OBJETIVO: Ayudar al analista a recolectar, procesar y visualizar inteligencia de fuentes abiertas.\n");
        prompt.push_str(
            "FILOSOFÍA: Rigor técnico, sigilo operativo y enfoque en resultados accionables.\n\n",
        );

        prompt.push_str("## MANUAL BREVE DE OPERACIÓN\n");
        prompt.push_str("- Para buscar personas: Brindame nombre completo o alias. Usaré herramientas de scraping y búsqueda en redes.\n");
        prompt.push_str("- Para dominios/IPs: Usaré WHOIS, DNS y escaneos técnicos. Siempre persistiré los hallazgos en el tablero.\n");
        prompt.push_str("- Para configuraciones: Podés pedirme que cambie el modelo de IA o que configure una API Key (ej: 'Cambiá el modelo a llama3').\n\n");

        // 2. Reglas críticas compartidas
        prompt.push_str("## REGLAS CRÍTICAS DE RAZONAMIENTO\n");
        prompt.push_str("1. VERDAD ABSOLUTA: El tablero es la única fuente de verdad. Si el usuario pregunta algo, buscalo primero en el contexto.\n");
        prompt.push_str("2. REFERENCIAS (@): Los términos que empiezan con '@' (ej: @Objetivo) son referencias directas a elementos del tablero de hechos. NO son URLs literales ni comandos; usalos para identificar de qué objetivo estamos hablando. IMPORTANTE: Al llamar a una herramienta (ej: scrape_generic), NUNCA incluyas el símbolo '@' en los parámetros; usá el nombre/valor real del objetivo.\n");
        prompt.push_str("3. PROHIBICIÓN DE REPETICIÓN: Si una herramienta aparece en tu historial reciente o figura como '[PROHIBIDO] YA EJECUTADO', tenés terminantemente prohibido volver a llamarla para ese objetivo.\n");
        prompt.push_str("4. RESPUESTAS LIMPIAS: No relates paso por paso qué herramientas vas a usar si no es estrictamente necesario. Entregá los hallazgos valiosos directamente.\n");
        prompt.push_str(
            "5. TONO: Respondé siempre con un tono profesional, sereno y rioplatense (voseo).\n\n",
        );

        // 3. Skills (Si hay alguna disponible)
        if let Some(skills) = params.skills {
            prompt.push_str(&crate::skills::build_skills_prompt_section(skills));
            prompt.push_str("\n");
        }

        // 4. Contexto Operativo Dinámico (Tablero)
        if let Some(ctx) = params.case_context {
            prompt.push_str("## CONTEXTO OPERATIVO DEL CASO\n");
            prompt.push_str(ctx);
            prompt.push_str("\n\nNOTA: Usá este contexto para entender relaciones. Si NO hay objetivos listados, ESPERÁ instrucciones.\n");
        }

        // 5. Hard break por Loop / Bucle o Inyección de Herramientas manual
        if params.force_text_only {
            prompt.push_str("\n\nSISTEMA: Estás forzado a responder en TEXTO PLANO. Tenés PROHIBIDO terminantemente llamar a herramientas. Resumí la información que ya sabés o pedí ayuda al analista.\n");
        } else if !self.supports_native_tools() {
            prompt.push_str("\n\n## CAPACIDADES TÉCNICAS (HERRAMIENTAS)\n");
            prompt.push_str("Para ejecutar una acción técnica (como guardar hallazgos o correr comandos), debés incluir un bloque JSON al FINAL de tu respuesta.\n");
            prompt.push_str("Si solo querés conversar, responder preguntas o pedir aclaraciones, hacélo normalmente en TEXTO PLANO rioplatense.\n\n");
            prompt.push_str("Formato para herramientas:\n");
            prompt.push_str("```json\n{\n  \"name\": \"nombre_de_la_funcion\",\n  \"parameters\": {\"arg1\": \"valor1\"}\n}\n```\n\n");
            prompt.push_str("Herramientas disponibles:\n");
            for tool in self.get_tools_definition() {
                if let Some(func) = tool.get("function") {
                    let name = func.get("name").and_then(|v| v.as_str()).unwrap_or("");
                    let desc = func
                        .get("description")
                        .and_then(|v| v.as_str())
                        .unwrap_or("");
                    prompt.push_str(&format!("- {}: {}\n", name, desc));
                }
            }
        }

        prompt
    }

    /// Estimación simple de tokens por chars / 4, con buffer de seguridad como en OpenClaw's compaction
    fn estimate_tokens_from_chars(text: &str) -> usize {
        let chars = text.chars().count();
        ((chars as f64 / 4.0) * 1.2) as usize
    }

    /// Filtra los mensajes de ruido técnico del historial antes de mandar al contexto
    fn sanitize_history(&self) -> Vec<&AgentMessage> {
        let is_tool_noise = |msg: &&AgentMessage| -> bool {
            let c = &msg.content;
            c.starts_with("[TOOL_CALLS]")
                || c.contains("Te estás quedando en un bucle")
                || c.contains("respondeme con texto directamente")
        };

        self.history
            .iter()
            .filter(|msg| msg.role != "error" && msg.role != "system")
            .filter(|msg| !is_tool_noise(msg))
            .collect()
    }

    pub fn add_message(&mut self, role: &str, content: &str) {
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
        self.history.push(AgentMessage {
            role: "user".to_string(),
            content: format!("[SISTEMA] Resultado de {}:\n{}", tool_name, content),
        });
    }

    pub async fn think(
        &self,
        last_input: Option<&str>,
        image_path: Option<&str>,
        context_data: Option<&str>,
        force_text_only: bool,
        skills: Option<&[Skill]>,
    ) -> AgentResponse {
        let url = format!("{}/api/chat", self.url);
        let mut messages = Vec::new();

        // 1. System Prompt Modular
        let prompt_params = PromptParams {
            case_context: context_data,
            force_text_only,
            skills,
        };
        messages.push(json!({
            "role": "system",
            "content": self.build_system_prompt(&prompt_params)
        }));

        // 2. Historial Compresible (Sanitizado)
        let cleaned_history = self.sanitize_history();

        // En lugar de una ventana de 6 mensajes estática, aplicamos la estrategia
        // de OpenClaw: enviamos todo el historial posible hasta el límite de la ventana.
        // Si superamos un umbral (e.g. 6000 tokens), truncamos los msgs más viejos.
        // (La summarización por LLM como compaction se delegará a un background run si crece mucho).

        const MAX_CONTEXT_TOKENS: usize = 6000;
        let mut history_slice = Vec::new();
        let mut current_tokens = 0;

        for msg in cleaned_history.iter().rev() {
            let tokens = Self::estimate_tokens_from_chars(&msg.content);
            if current_tokens + tokens > MAX_CONTEXT_TOKENS && !history_slice.is_empty() {
                break;
            }
            history_slice.push(*msg);
            current_tokens += tokens;
        }
        history_slice.reverse();

        if !history_slice.is_empty() {
            let history_lines: Vec<String> = history_slice
                .iter()
                .map(|m| {
                    let tag = if m.role == "user" {
                        "Usuario"
                    } else {
                        "Asistente"
                    };
                    format!("{}: {}", tag, m.content)
                })
                .collect();

            let history_block = history_lines.join("\n---\n");

            // Inyectamos todo el historial como un solo mensaje previo (history compression)
            messages.push(json!({
                "role": "user",
                "content": format!(
                    "[HISTORIAL DE LA SESIÓN (Limpiado y Comprimido)]\n{}",
                    history_block
                )
            }));
            messages.push(json!({
                "role": "assistant",
                "content": "Entendido. Sigo la conversación."
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

        // Definición de Herramientas
        let tools = self.get_tools_definition();
        let use_native_tools = self.supports_native_tools() && !force_text_only;

        // Enviar al modelo
        let body = if force_text_only {
            // Inyectamos un recordatorio extra para que el modelo responda en texto
            messages.push(json!({
                "role": "user",
                "content": "SISTEMA: Tenes que responderme con texto directamente. No llames a ninguna herramienta. Resumi la informacion que ya tenes en el contexto."
            }));
            json!({
                "model": self.model,
                "messages": messages,
                "stream": false
            })
        } else if use_native_tools {
            json!({
                "model": self.model,
                "messages": messages,
                "tools": tools,
                "stream": false
            })
        } else {
            // Modo Híbirdo/Inyectado: No mandamos el campo 'tools' para evitar el error de Ollama
            json!({
                "model": self.model,
                "messages": messages,
                "stream": false
            })
        };

        if self.abort_flag.load(std::sync::atomic::Ordering::SeqCst) {
            return AgentResponse::Error("Operación abortada por el usuario.".to_string());
        }

        println!("DEBUG [Agent]: Enviando a Ollama:");
        println!("- Model: {}", self.model);
        println!(
            "- Messages: {}",
            serde_json::to_string_pretty(&messages).unwrap_or_default()
        );
        println!("- Tools enabled: {}", tools.len());

        let resp = self.client.post(url).json(&body).send().await;

        match resp {
            Ok(r) => {
                if !r.status().is_success() {
                    let err_text = r.text().await.unwrap_or_default();
                    return AgentResponse::Error(format!("Ollama API Error: {}", err_text));
                }

                let json_resp: serde_json::Value = match r.json().await {
                    Ok(v) => {
                        println!("DEBUG [Agent]: Respuesta RAW de Ollama:");
                        println!("{}", serde_json::to_string_pretty(&v).unwrap_or_default());
                        v
                    }
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

                    // Parsear respuesta de texto CON DETECCIÓN DE JSON (Parser Híbrido)
                    if let Some(content) = message.get("content").and_then(|c| c.as_str()) {
                        // DEEPSEEK-R1 SOPORTE: Remover tags <think>...</think> del contenido
                        let mut clean_content = content.trim().to_string();
                        while let Some(start) = clean_content.find("<think>") {
                            if let Some(end) = clean_content.find("</think>") {
                                let mut new_str = String::new();
                                new_str.push_str(&clean_content[..start]);
                                new_str.push_str(&clean_content[end + 8..]);
                                clean_content = new_str;
                            } else {
                                clean_content = clean_content[..start].to_string();
                                break;
                            }
                        }

                        let trimmed = clean_content.trim();
                        if !trimmed.is_empty() {
                            // Buscar bloque JSON en el texto (incluso si hay texto antes o después)
                            // Buscamos el primer '{' y el último '}' que parezcan un objeto de herramienta
                            if let Some(start_idx) = trimmed.find('{') {
                                if let Some(end_idx) = trimmed.rfind('}') {
                                    let potential_json = &trimmed[start_idx..=end_idx];

                                    // Validar si contiene campos clave de nuestras herramientas
                                    if potential_json.contains("\"name\"")
                                        && (potential_json.contains("\"parameters\"")
                                            || potential_json.contains("\"arguments\""))
                                    {
                                        if let Ok(val) = serde_json::from_str::<serde_json::Value>(
                                            potential_json,
                                        ) {
                                            if let (Some(n), p) = (
                                                val.get("name").and_then(|v| v.as_str()),
                                                val.get("parameters").or(val.get("arguments")),
                                            ) {
                                                // Mapeo amigable de alias (por si el modelo alucina nombres de OpenClaw)
                                                let n_clean = n
                                                    .replace("_", "")
                                                    .replace(" ", "")
                                                    .to_lowercase();
                                                let normalized_name = match n_clean.as_str() {
                                                    "runosintlookup"
                                                    | "runwslcommand"
                                                    | "ejecutarherramientalinux"
                                                    | "ejecutarherramienta" => {
                                                        "ejecutar_herramienta_linux"
                                                    }
                                                    "upsertintelligence" | "guardarhallazgo"
                                                    | "guardar" => "guardar_hallazgo",
                                                    "reportactivity"
                                                    | "registraractividadtecnica"
                                                    | "registraractividad" => {
                                                        "registrar_actividad_tecnica"
                                                    }
                                                    "verconfiguracion" | "obtenerconfiguracion" => {
                                                        "ver_configuracion"
                                                    }
                                                    "actualizarconfiguracion"
                                                    | "cambiarconfiguracion" => {
                                                        "actualizar_configuracion"
                                                    }
                                                    "ayuda" | "obtenerayuda" | "manual" => {
                                                        "obtener_ayuda"
                                                    }
                                                    _ => n,
                                                };

                                                let mut arguments = HashMap::new();
                                                if let Some(p_val) = p {
                                                    if let Some(obj) = p_val.as_object() {
                                                        for (k, v) in obj {
                                                            arguments.insert(
                                                                k.clone(),
                                                                v.to_string().replace("\"", ""),
                                                            );
                                                        }
                                                    } else if let Some(s) = p_val.as_str() {
                                                        if let Ok(p_json) = serde_json::from_str::<
                                                            serde_json::Value,
                                                        >(
                                                            s
                                                        ) {
                                                            if let Some(obj) = p_json.as_object() {
                                                                for (k, v) in obj {
                                                                    arguments.insert(
                                                                        k.clone(),
                                                                        v.to_string()
                                                                            .replace("\"", ""),
                                                                    );
                                                                }
                                                            }
                                                        }
                                                    }
                                                }

                                                println!("DEBUG [Agent]: Detectada llamada a herramienta FANTASMA ({}) en texto.", normalized_name);
                                                return AgentResponse::Tools(vec![ToolCall {
                                                    tool_name: normalized_name.to_string(),
                                                    arguments,
                                                }]);
                                            }
                                        }
                                    }
                                }
                            }
                            return AgentResponse::Text(clean_content);
                        }
                    }
                }

                AgentResponse::Error(
                    "El modelo de IA no generó texto útil ni llamó a herramientas.".to_string(),
                )
            }
            Err(e) => AgentResponse::Error(format!("Error de conexión con Ollama: {}", e)),
        }
    }
}
