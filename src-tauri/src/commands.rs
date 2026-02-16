use crate::agent::{Agent, AgentResponse};
use crate::cases::{CaseManager, Target};
use crate::mac_spoof;
use crate::models::{Address, Job, Nickname, OsintConfig, OsintResult, Person, SocialProfile};
use crate::tools;
use crate::tor_manager;
use serde_json::json;
use std::collections::{HashMap, HashSet};
use tauri::{AppHandle, Emitter, State};
use tokio::sync::Mutex;

#[tauri::command]
pub fn greet(name: &str) -> String {
    format!("Hola, {}! Bienvenido al OSINT Dashboard.", name)
}

#[tauri::command]
pub async fn log_info(msg: String) {
    println!("Frontend Log: {}", msg);
}

#[tauri::command]
pub async fn run_osint_lookup(
    target: String,
    tool: String,
    config: State<'_, Mutex<OsintConfig>>,
) -> Result<OsintResult, String> {
    let conf = config.lock().await;
    match tool.as_str() {
        "ping" => Ok(tools::perform_ping(&target).await),
        "whois" => Ok(tools::perform_whois(&target, &*conf).await),
        "dns" => Ok(tools::perform_dns_lookup(&target).await),
        "shodan" => Ok(tools::shodan_intel(target, &*conf).await),
        "virustotal" => Ok(tools::virus_total_scan(target, &*conf).await),
        "ip_intel" => Ok(tools::ip_intel(target, &*conf).await),
        "username" => Ok(tools::search_username(target, &*conf).await),
        "dorks" => Ok(tools::generate_dorks(target).await),
        "social" => Ok(tools::social_search(target, &*conf).await),
        "leaks" => Ok(tools::search_leaks(target, &*conf).await),
        _ => Err("Herramienta no implementada".into()),
    }
}

#[tauri::command]
pub async fn extract_metadata(path: String) -> Result<OsintResult, String> {
    Ok(tools::extract_metadata(path).await)
}

#[tauri::command]
pub async fn web_scrape_search(
    query: String,
    config: State<'_, Mutex<OsintConfig>>,
) -> Result<OsintResult, String> {
    let conf = config.lock().await;
    Ok(tools::web_scrape_search(query, &*conf).await)
}

#[tauri::command]
pub async fn download_face_models(_app: AppHandle) -> Result<OsintResult, String> {
    // Implementación real pendiente, por ahora avisamos
    Ok(OsintResult {
        success: true,
        data: "Modelos listos (Simulado)".into(),
        error: None,
    })
}

#[tauri::command]
pub async fn read_file_base64(path: String) -> Result<String, String> {
    use base64::{engine::general_purpose, Engine as _};
    let data = std::fs::read(&path).map_err(|e| {
        eprintln!("ERROR [system]: File read failure at {}: {}", path, e);
        "No se pudo leer el archivo solicitado. Verifique los permisos y la ruta.".to_string()
    })?;
    Ok(general_purpose::STANDARD.encode(data))
}

#[tauri::command]
pub async fn update_osint_config(
    new_config: OsintConfig,
    config: State<'_, Mutex<OsintConfig>>,
) -> Result<(), String> {
    let mut conf = config.lock().await;
    *conf = new_config;
    Ok(())
}

#[tauri::command]
pub async fn set_tor_active(active: bool, app: AppHandle) -> Result<OsintResult, String> {
    if active {
        match tor_manager::start_tor(&app).await {
            Ok(_) => Ok(OsintResult {
                success: true,
                data: "Servicio Tor iniciado satisfactoriamente.".into(),
                error: None,
            }),
            Err(e) => Ok(OsintResult {
                success: false,
                data: "".into(),
                error: Some(e),
            }),
        }
    } else {
        tor_manager::stop_tor(&app).await;
        Ok(OsintResult {
            success: true,
            data: "Servicio Tor detenido.".into(),
            error: None,
        })
    }
}

#[tauri::command]
pub async fn set_mac_masking(active: bool) -> Result<OsintResult, String> {
    match mac_spoof::get_active_adapter_info() {
        Ok((name, _)) => {
            if active {
                let new_mac = mac_spoof::generate_random_mac();
                match mac_spoof::apply_mac_spoof(&name, &new_mac) {
                    Ok(_) => Ok(OsintResult {
                        success: true,
                        data: format!("MAC cambiada a: {}", new_mac),
                        error: None,
                    }),
                    Err(e) => Ok(OsintResult {
                        success: false,
                        data: "".into(),
                        error: Some(e),
                    }),
                }
            } else {
                match mac_spoof::reset_mac(&name) {
                    Ok(_) => Ok(OsintResult {
                        success: true,
                        data: "MAC original restaurada".into(),
                        error: None,
                    }),
                    Err(e) => Ok(OsintResult {
                        success: false,
                        data: "".into(),
                        error: Some(e),
                    }),
                }
            }
        }
        Err(e) => Ok(OsintResult {
            success: false,
            data: "".into(),
            error: Some(e),
        }),
    }
}

#[tauri::command]
pub async fn abort_agent(state: State<'_, Mutex<Agent>>) -> Result<(), String> {
    match state.try_lock() {
        Ok(agent) => {
            agent
                .abort_flag
                .store(true, std::sync::atomic::Ordering::SeqCst);
            Ok(())
        }
        Err(_) => {
            // Si no podemos bloquearlo, es que ya está trabajando o bloqueado,
            // pero el flag es atómico, así que esto es solo preventivo.
            Err("No se pudo enviar la señal de interrupción al agente.".to_string())
        }
    }
}

#[tauri::command]
pub async fn ask_agent(
    app: AppHandle,
    query: String,
    image_path: Option<String>,
    case_name: Option<String>,
    state: State<'_, Mutex<Agent>>,
    config_lock: State<'_, Mutex<OsintConfig>>,
    case_manager: State<'_, CaseManager>,
) -> Result<OsintResult, String> {
    let mut agent = state.lock().await;

    if let Some(case) = &case_name {
        if let Ok(history) = case_manager.load_history(case) {
            if let Ok(msgs) = serde_json::from_str(&history) {
                agent.history = msgs;
            }
        }
    }

    let mut seen_calls = HashSet::new();
    for h_msg in &agent.history {
        if h_msg.role == "assistant" || h_msg.role == "model" {
            if let Ok(json_calls) = serde_json::from_str::<serde_json::Value>(&h_msg.content) {
                if let Some(calls_array) = json_calls.as_array() {
                    for call_val in calls_array {
                        let name = call_val["name"].as_str().unwrap_or_default();
                        let args = call_val["args"].as_object();
                        if !name.is_empty() && args.is_some() {
                            let mut keys: Vec<String> = args
                                .unwrap()
                                .values()
                                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                                .collect();
                            keys.sort();
                            seen_calls.insert(format!("{}:{}", name, keys.join(",")));
                        }
                    }
                }
            }
        }
    }

    // Construcción del Contexto Dinámico (Objetivos + Vínculos)
    let mut case_context = String::new();
    if let Some(case) = &case_name {
        if let Ok(targets) = case_manager.get_targets(case) {
            if !targets.is_empty() {
                case_context.push_str("OBJETIVOS REGISTRADOS:\n");
                for t in &targets {
                    case_context.push_str(&format!(
                        "- ID: {}, Nombre: {}, Tipo: {:?}\n",
                        t.id, t.name, t.target_type
                    ));
                    if !t.data.is_empty() {
                        case_context.push_str(&format!("  Datos: {:?}\n", t.data));
                    }
                }

                case_context.push_str("\nVÍNCULOS:\n");
                for t in &targets {
                    for link in &t.linked_targets {
                        case_context.push_str(&format!(
                            "- {} -> {} (Rel: {})\n",
                            t.name, link.target_id, link.relation
                        ));
                    }
                }
            } else {
                case_context.push_str("No hay objetivos registrados en este caso aún.");
            }
        }
    }

    loop {
        let response = agent
            .think(Some(&query), image_path.as_deref(), Some(&case_context))
            .await;
        match response {
            AgentResponse::Text(text) => {
                agent.add_message("assistant", &text);
                if let Some(case) = &case_name {
                    let _ = case_manager.save_history(
                        case,
                        &serde_json::to_string(&agent.history).unwrap_or_default(),
                    );
                }
                return Ok(OsintResult {
                    success: true,
                    data: text,
                    error: None,
                });
            }
            AgentResponse::Tools(tool_calls) => {
                let tools_json = tool_calls
                    .iter()
                    .map(|tc| json!({"name": tc.tool_name, "args": tc.arguments}))
                    .collect::<Vec<_>>();
                agent.add_message("model", &serde_json::json!(tools_json).to_string());
                let mut tool_outputs = String::new();

                for tool in tool_calls {
                    let name = tool.tool_name.as_str();
                    let args = tool.arguments;
                    let mut keys: Vec<_> = args.values().cloned().collect();
                    keys.sort();
                    let call_key = format!("{}:{}", name, keys.join(","));

                    if seen_calls.contains(&call_key) {
                        let mut last_result = None;
                        for h_msg in agent.history.iter().rev() {
                            if h_msg.content.contains(&format!("Resultado de {}:", name)) {
                                if let Some(pos) = h_msg.content.find(':') {
                                    last_result = Some(h_msg.content[pos + 1..].trim().to_string());
                                    break;
                                }
                            }
                        }
                        if let Some(res) = last_result {
                            tool_outputs
                                .push_str(&format!("\nResultado de {} (Cache): {}\n", name, res));
                            agent.add_function_response(name, &res);
                            continue;
                        }
                    }
                    seen_calls.insert(call_key);

                    let tool_result = match name {
                        "ping" => {
                            tools::perform_ping(args.get("target").unwrap_or(&"".into()))
                                .await
                                .data
                        }
                        "whois" => {
                            let config = config_lock.lock().await;
                            tools::perform_whois(args.get("target").unwrap_or(&"".into()), &*config)
                                .await
                                .data
                        }
                        "dns" => {
                            tools::perform_dns_lookup(args.get("target").unwrap_or(&"".into()))
                                .await
                                .data
                        }
                        "web_scrape_search" => {
                            let q = args.get("query").unwrap_or(&"".into()).to_string();
                            let config = config_lock.lock().await;
                            tools::web_scrape_search(q, &*config).await.data
                        }
                        "browse_url" => {
                            let url = args.get("url").unwrap_or(&"".into()).to_string();
                            let config = config_lock.lock().await;
                            tools::browse_url(url, &*config).await.data
                        }
                        "dark_search" => {
                            let q = args.get("query").unwrap_or(&"".into()).to_string();
                            let config = config_lock.lock().await;
                            tools::dark_search(q, &*config).await.data
                        }
                        "manage_target" => {
                            if let (Some(n), Some(t), Some(k), Some(v), Some(c)) = (
                                args.get("name"),
                                args.get("target_type"),
                                args.get("key"),
                                args.get("value"),
                                args.get("category"),
                            ) {
                                if let Some(case) = &case_name {
                                    use crate::cases::{Target, TargetType};
                                    let tt = match t.as_str() {
                                        "Person" => TargetType::Person,
                                        "Domain" => TargetType::Domain,
                                        "IP" => TargetType::IP,
                                        "Email" => TargetType::Email,
                                        _ => TargetType::Other,
                                    };
                                    let mut d = HashMap::new();
                                    d.insert(k.to_string(), v.to_string());
                                    let targ = Target {
                                        id: format!(
                                            "{}-{}",
                                            n.to_lowercase().replace(' ', "_"),
                                            t.to_lowercase()
                                        ),
                                        name: n.to_string(),
                                        target_type: tt,
                                        data: d,
                                        linked_targets: Vec::new(),
                                        created_at: chrono::Utc::now(),
                                    };
                                    match case_manager.upsert_target_with_cat(case, targ, c) {
                                        Ok(_) => format!("✅ Ficha actualizada: {}={}", k, v),
                                        Err(e) => format!("❌ Error: {}", e),
                                    }
                                } else {
                                    "No hay caso activo".to_string()
                                }
                            } else {
                                "Faltan argumentos (name, target_type, key, value, category)"
                                    .to_string()
                            }
                        }
                        "get_targets" => {
                            if let Some(case) = &case_name {
                                match case_manager.get_targets(case) {
                                    Ok(targets) => {
                                        let mut r = String::from("🗂️ Objetivos:\n");
                                        for t in targets {
                                            r.push_str(&format!("- {}: {:?}\n", t.name, t.data));
                                        }
                                        r
                                    }
                                    Err(e) => e,
                                }
                            } else {
                                "No hay caso activo".to_string()
                            }
                        }
                        "link_targets" => {
                            if let (Some(s), Some(dst), Some(r)) = (
                                args.get("source_id"),
                                args.get("target_id"),
                                args.get("relation"),
                            ) {
                                if let Some(case) = &case_name {
                                    match case_manager.add_link(case, s, dst, r) {
                                        Ok(_) => format!("✅ Vínculo: {} -> {}", s, dst),
                                        Err(e) => e,
                                    }
                                } else {
                                    "No hay caso activo".to_string()
                                }
                            } else {
                                "Faltan argumentos (source_id, target_id, relation)".to_string()
                            }
                        }
                        "generate_dorks" => {
                            tools::generate_dorks(
                                args.get("target").unwrap_or(&"".into()).to_string(),
                            )
                            .await
                            .data
                        }
                        "social_search" => {
                            tools::social_search(
                                args.get("target").unwrap_or(&"".into()).to_string(),
                                &*config_lock.lock().await,
                            )
                            .await
                            .data
                        }
                        "search_leaks" => {
                            tools::search_leaks(
                                args.get("target").unwrap_or(&"".into()).to_string(),
                                &*config_lock.lock().await,
                            )
                            .await
                            .data
                        }
                        _ => "Herramienta no encontrada".to_string(),
                    };

                    tool_outputs.push_str(&format!("\nResultado de {}: {}\n", name, tool_result));
                    agent.add_function_response(name, &tool_result);
                    let _ = app.emit(
                        "agent-tool-result",
                        json!({"tool": name, "result": tool_result}),
                    );
                }
            }
            AgentResponse::Error(e) => {
                return Ok(OsintResult {
                    success: false,
                    data: e.clone(),
                    error: Some(e),
                })
            }
        }
    }
}

#[tauri::command]
pub async fn create_case(
    name: String,
    description: String,
    case_manager: State<'_, CaseManager>,
) -> Result<OsintResult, String> {
    match case_manager.create_case(&name, &description) {
        Ok(_) => Ok(OsintResult {
            success: true,
            data: format!("Investigación '{}' creada.", name),
            error: None,
        }),
        Err(e) => Ok(OsintResult {
            success: false,
            data: "".into(),
            error: Some(e),
        }),
    }
}

#[tauri::command]
pub async fn list_cases(case_manager: State<'_, CaseManager>) -> Result<Vec<String>, String> {
    case_manager.list_cases()
}

#[tauri::command]
pub async fn load_case(
    name: String,
    case_manager: State<'_, CaseManager>,
) -> Result<OsintResult, String> {
    match case_manager.load_case(&name) {
        Ok(metadata) => {
            let data = serde_json::to_string(&metadata).map_err(|e| {
                eprintln!("ERROR [commands]: Metadata serialization error: {}", e);
                "Error al procesar los datos de la investigación.".to_string()
            })?;
            Ok(OsintResult {
                success: true,
                data,
                error: None,
            })
        }
        Err(e) => Ok(OsintResult {
            success: false,
            data: "".into(),
            error: Some(e),
        }),
    }
}

#[tauri::command]
pub async fn save_case_history(
    case_name: String,
    history_json: String,
    case_manager: State<'_, CaseManager>,
) -> Result<(), String> {
    case_manager.save_history(&case_name, &history_json)
}

#[tauri::command]
pub async fn get_case_history(
    case_name: String,
    case_manager: State<'_, CaseManager>,
) -> Result<String, String> {
    case_manager.load_history(&case_name)
}

#[tauri::command]
pub fn delete_case_cmd(
    case_manager: State<'_, CaseManager>,
    case_name: String,
) -> Result<OsintResult, String> {
    match case_manager.delete_case(&case_name) {
        Ok(_) => Ok(OsintResult {
            success: true,
            data: "Investigación eliminada correctamente.".to_string(),
            error: None,
        }),
        Err(e) => Ok(OsintResult {
            success: false,
            data: "".to_string(),
            error: Some(e),
        }),
    }
}

// --- PERSON COMMANDS ---

#[tauri::command]
pub fn create_person_cmd(
    case_manager: State<'_, CaseManager>,
    case_name: String,
    person: Person,
) -> Result<OsintResult, String> {
    match case_manager.create_person(&case_name, person) {
        Ok(p) => Ok(OsintResult {
            success: true,
            data: serde_json::to_string(&p).unwrap_or_default(),
            error: None,
        }),
        Err(e) => Ok(OsintResult {
            success: false,
            data: "".to_string(),
            error: Some(e),
        }),
    }
}

#[tauri::command]
pub fn get_persons_cmd(
    case_manager: State<'_, CaseManager>,
    case_name: String,
) -> Result<OsintResult, String> {
    match case_manager.get_persons(&case_name) {
        Ok(persons) => Ok(OsintResult {
            success: true,
            data: serde_json::to_string(&persons).unwrap_or_default(),
            error: None,
        }),
        Err(e) => Ok(OsintResult {
            success: false,
            data: "".to_string(),
            error: Some(e),
        }),
    }
}

#[tauri::command]
pub fn update_person_cmd(
    case_manager: State<'_, CaseManager>,
    case_name: String,
    person: Person,
) -> Result<OsintResult, String> {
    match case_manager.update_person_basic(&case_name, person) {
        Ok(_) => Ok(OsintResult {
            success: true,
            data: "Datos básicos actualizados.".to_string(),
            error: None,
        }),
        Err(e) => Ok(OsintResult {
            success: false,
            data: "".to_string(),
            error: Some(e),
        }),
    }
}

#[tauri::command]
pub fn delete_person_cmd(
    case_manager: State<'_, CaseManager>,
    case_name: String,
    person_id: String,
) -> Result<OsintResult, String> {
    match case_manager.delete_person(&case_name, &person_id) {
        Ok(_) => Ok(OsintResult {
            success: true,
            data: "Persona eliminada correctamente.".to_string(),
            error: None,
        }),
        Err(e) => Ok(OsintResult {
            success: false,
            data: "".to_string(),
            error: Some(e),
        }),
    }
}

// --- NICKNAME COMMANDS ---

#[tauri::command]
pub fn add_nickname_cmd(
    case_manager: State<'_, CaseManager>,
    case_name: String,
    person_id: String,
    nickname: Nickname,
) -> Result<OsintResult, String> {
    match case_manager.add_nickname(&case_name, &person_id, nickname) {
        Ok(nick) => Ok(OsintResult {
            success: true,
            data: serde_json::to_string(&nick).unwrap_or_default(),
            error: None,
        }),
        Err(e) => Ok(OsintResult {
            success: false,
            data: "".to_string(),
            error: Some(e),
        }),
    }
}

#[tauri::command]
pub fn remove_nickname_cmd(
    case_manager: State<'_, CaseManager>,
    case_name: String,
    nickname_id: String,
) -> Result<OsintResult, String> {
    match case_manager.remove_nickname(&case_name, &nickname_id) {
        Ok(_) => Ok(OsintResult {
            success: true,
            data: "Apodo eliminado.".to_string(),
            error: None,
        }),
        Err(e) => Ok(OsintResult {
            success: false,
            data: "".to_string(),
            error: Some(e),
        }),
    }
}

// --- SUB-ENTITY COMMANDS (Address, Job, Social) ---

#[tauri::command]
pub fn add_address_cmd(
    case_manager: State<'_, CaseManager>,
    case_name: String,
    person_id: String,
    address: Address,
) -> Result<OsintResult, String> {
    match case_manager.add_address(&case_name, &person_id, address) {
        Ok(addr) => Ok(OsintResult {
            success: true,
            data: serde_json::to_string(&addr).unwrap_or_default(),
            error: None,
        }),
        Err(e) => Ok(OsintResult {
            success: false,
            data: "".to_string(),
            error: Some(e),
        }),
    }
}

#[tauri::command]
pub fn remove_address_cmd(
    case_manager: State<'_, CaseManager>,
    case_name: String,
    address_id: String,
) -> Result<OsintResult, String> {
    match case_manager.remove_address(&case_name, &address_id) {
        Ok(_) => Ok(OsintResult {
            success: true,
            data: "Dirección eliminada.".to_string(),
            error: None,
        }),
        Err(e) => Ok(OsintResult {
            success: false,
            data: "".to_string(),
            error: Some(e),
        }),
    }
}

#[tauri::command]
pub fn add_job_cmd(
    case_manager: State<'_, CaseManager>,
    case_name: String,
    person_id: String,
    job: Job,
) -> Result<OsintResult, String> {
    match case_manager.add_job(&case_name, &person_id, job) {
        Ok(j) => Ok(OsintResult {
            success: true,
            data: serde_json::to_string(&j).unwrap_or_default(),
            error: None,
        }),
        Err(e) => Ok(OsintResult {
            success: false,
            data: "".to_string(),
            error: Some(e),
        }),
    }
}

#[tauri::command]
pub fn remove_job_cmd(
    case_manager: State<'_, CaseManager>,
    case_name: String,
    job_id: String,
) -> Result<OsintResult, String> {
    match case_manager.remove_job(&case_name, &job_id) {
        Ok(_) => Ok(OsintResult {
            success: true,
            data: "Empleo eliminado.".to_string(),
            error: None,
        }),
        Err(e) => Ok(OsintResult {
            success: false,
            data: "".to_string(),
            error: Some(e),
        }),
    }
}

#[tauri::command]
pub fn add_social_cmd(
    case_manager: State<'_, CaseManager>,
    case_name: String,
    person_id: String,
    social: SocialProfile,
) -> Result<OsintResult, String> {
    match case_manager.add_social(&case_name, &person_id, social) {
        Ok(s) => Ok(OsintResult {
            success: true,
            data: serde_json::to_string(&s).unwrap_or_default(),
            error: None,
        }),
        Err(e) => Ok(OsintResult {
            success: false,
            data: "".to_string(),
            error: Some(e),
        }),
    }
}

#[tauri::command]
pub fn remove_social_cmd(
    case_manager: State<'_, CaseManager>,
    case_name: String,
    social_id: String,
) -> Result<OsintResult, String> {
    match case_manager.remove_social(&case_name, &social_id) {
        Ok(_) => Ok(OsintResult {
            success: true,
            data: "Perfil social eliminado.".to_string(),
            error: None,
        }),
        Err(e) => Ok(OsintResult {
            success: false,
            data: "".to_string(),
            error: Some(e),
        }),
    }
}

// --- TECHNICAL TARGETS COMMANDS ---

#[tauri::command]
pub fn get_targets_json_cmd(
    case_manager: State<'_, CaseManager>,
    case_name: String,
) -> Result<OsintResult, String> {
    match case_manager.get_targets(&case_name) {
        Ok(targets) => Ok(OsintResult {
            success: true,
            data: serde_json::to_string(&targets).unwrap_or_default(),
            error: None,
        }),
        Err(e) => Ok(OsintResult {
            success: false,
            data: "".to_string(),
            error: Some(e),
        }),
    }
}

#[tauri::command]
pub fn create_target_cmd(
    case_manager: State<'_, CaseManager>,
    case_name: String,
    target: Target,
    category: String,
) -> Result<OsintResult, String> {
    match case_manager.upsert_target_with_cat(&case_name, target, &category) {
        Ok(_) => Ok(OsintResult {
            success: true,
            data: "Objetivo técnico creado/actualizado.".to_string(),
            error: None,
        }),
        Err(e) => Ok(OsintResult {
            success: false,
            data: "".to_string(),
            error: Some(e),
        }),
    }
}
