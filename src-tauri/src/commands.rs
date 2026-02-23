use crate::agent::{Agent, AgentResponse};
use crate::cases::{CaseManager, Target, TargetType};
use crate::mac_spoof;
use crate::models::{Address, Job, Nickname, OsintConfig, OsintResult, Person, SocialProfile};
use crate::secrets;
use crate::tools;
use crate::tor_manager;
use crate::AgentAbort;
use serde_json::json;
use std::collections::HashMap;
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
pub async fn abort_agent(abort_state: State<'_, AgentAbort>) -> Result<(), String> {
    abort_state
        .inner()
        .0
        .store(true, std::sync::atomic::Ordering::SeqCst);
    Ok(())
}

#[tauri::command]
pub async fn ask_agent(
    app: AppHandle,
    query: String,
    image_path: Option<String>,
    case_name: Option<String>,
    state: State<'_, Mutex<Agent>>,
    abort_state: State<'_, AgentAbort>,
    _config_lock: State<'_, Mutex<OsintConfig>>,
    case_manager: State<'_, CaseManager>,
) -> Result<OsintResult, String> {
    // Resetear flag de aborto global al iniciar nueva consulta
    abort_state
        .inner()
        .0
        .store(false, std::sync::atomic::Ordering::SeqCst);

    let mut agent = state.lock().await;

    // Resetear flag de aborto interno
    agent
        .abort_flag
        .store(false, std::sync::atomic::Ordering::SeqCst);

    if let Some(case) = &case_name {
        if let Ok(history) = case_manager.load_history(case) {
            if let Ok(msgs) = serde_json::from_str(&history) {
                agent.history = msgs;
            }
        }
    }

    // Construcción del Contexto Dinámico (Consultor Estratégico)
    let mut case_context = String::new();
    if let Some(case) = &case_name {
        let mut tech_targets = Vec::new();
        let mut persons_data = Vec::new();

        if let Ok(targets) = case_manager.get_targets(case) {
            tech_targets = targets;
        }

        if let Ok(persons) = case_manager.get_persons(case) {
            persons_data = persons;
        }

        let context_json = json!({
            "technical_targets": tech_targets,
            "persons": persons_data
        });

        case_context = format!(
            "--- CONTEXTO OPERATIVO (ESTRUCTURADO JSON) ---\n{}\n--------------------------------------------\n",
            serde_json::to_string_pretty(&context_json).unwrap_or_default()
        );
    }

    // Loop de razonamiento y ejecución de herramientas
    let mut current_query: Option<String> = Some(query.clone());
    let mut iterations = 0;
    const MAX_ITERATIONS: u32 = 15;

    while iterations < MAX_ITERATIONS {
        iterations += 1;

        // Chequear interrupción antes de llamar a la IA
        if abort_state
            .inner()
            .0
            .load(std::sync::atomic::Ordering::SeqCst)
        {
            return Ok(OsintResult {
                success: false,
                data: "Interrumpido por el usuario.".into(),
                error: Some("Abortado".into()),
            });
        }

        let response = agent
            .think(
                current_query.as_deref(),
                image_path.as_deref(),
                Some(&case_context),
            )
            .await;

        // Limpiar current_query después del primer uso (ya estará en el historial si es necesario)
        if iterations == 1 {
            // Guardamos el primer input del usuario en el historial para que persista
            agent.add_message("user", &query);
            current_query = None;
        }

        // Si hay un error en Ollama, cortamos
        if let AgentResponse::Error(e) = response {
            return Ok(OsintResult {
                success: false,
                data: "".into(),
                error: Some(e),
            });
        }

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
            AgentResponse::Tools(calls) => {
                // IMPORTANTE: Guardar el hecho de que el asistente llamó a estas herramientas
                let tools_desc = calls
                    .iter()
                    .map(|c| format!("Llamando a {}({:?})", c.tool_name, c.arguments))
                    .collect::<Vec<_>>()
                    .join("\n");
                agent.add_message("assistant", &format!("[TOOL_CALLS]\n{}", tools_desc));

                let mut tool_results = Vec::new();

                for call in calls {
                    // Emitir qué herramienta se está usando
                    let _ = app.emit("agent-status", format!("Ejecutando {}...", call.tool_name));

                    if call.tool_name == "report_activity" {
                        if let Some(case) = &case_name {
                            let msg = call.arguments.get("message").cloned().unwrap_or_default();
                            let level = call
                                .arguments
                                .get("level")
                                .cloned()
                                .unwrap_or_else(|| "INFO".to_string());
                            let _ =
                                case_manager.log_event(case, &level, &msg, Some("report_activity"));
                            tool_results.push(
                                r#"{"status": "OK", "message": "Actividad registrada."}"#
                                    .to_string(),
                            );
                        } else {
                            tool_results.push(
                                r#"{"status": "ERROR", "message": "No hay un caso activo para registrar actividad."}"#
                                    .into(),
                            );
                        }
                    } else if call.tool_name == "run_wsl_command" {
                        let cmd = call.arguments.get("command").cloned().unwrap_or_default();

                        // Intentar obtener la contraseña sudo del keyring para este comando
                        let sudo_pass = crate::secrets::get_secret("wsl_sudo_password").ok();

                        let res = crate::tools::run_wsl_command(cmd, sudo_pass).await;
                        tool_results.push(format!(
                            r#"{{"status": "{}", "data": "{}"}}"#,
                            if res.success { "OK" } else { "ERROR" },
                            res.data.replace("\"", "\\\"")
                        ));
                    } else if call.tool_name == "upsert_intelligence" {
                        if let Some(case) = &case_name {
                            let name = call.arguments.get("name").cloned().unwrap_or_default();
                            let t_type_str = call
                                .arguments
                                .get("target_type")
                                .cloned()
                                .unwrap_or_default();
                            let category = call
                                .arguments
                                .get("category")
                                .cloned()
                                .unwrap_or_else(|| "Technical".into());

                            // FILTRO DE RUIDO: No permitir nombres vacíos o genéricos
                            if name.trim().is_empty() || name.to_lowercase() == "sin nombre" {
                                tool_results.push(format!(r#"{{"status": "ERROR", "message": "Nombre del objetivo '{}' no es válido. Sé más específico."}}"#, name));
                                continue;
                            }

                            let id = if let Some(id_val) =
                                call.arguments.get("id").filter(|s| !s.is_empty())
                            {
                                id_val.clone()
                            } else {
                                // Smart matching integral con lógica Fuzzy básica
                                let t_match = case_manager
                                    .get_targets(case)
                                    .ok()
                                    .and_then(|targets| {
                                        targets.into_iter().find(|t| {
                                            let n = t.name.to_lowercase();
                                            let query = name.to_lowercase();
                                            n == query || n.contains(&query) || query.contains(&n)
                                        })
                                    })
                                    .map(|t| t.id);

                                if let Some(tid) = t_match {
                                    tid
                                } else {
                                    let p_match = case_manager
                                        .get_persons(case)
                                        .ok()
                                        .and_then(|persons| {
                                            persons.into_iter().find(|p| {
                                                let full_name = format!(
                                                    "{} {}",
                                                    p.first_name.as_deref().unwrap_or(""),
                                                    p.last_name.as_deref().unwrap_or("")
                                                )
                                                .trim()
                                                .to_string()
                                                .to_lowercase();
                                                let query = name.to_lowercase();
                                                full_name == query
                                                    || full_name.contains(&query)
                                                    || query.contains(&full_name)
                                                    || p.nicknames
                                                        .iter()
                                                        .any(|n| n.value.to_lowercase() == query)
                                            })
                                        })
                                        .map(|p| p.id);

                                    p_match.unwrap_or_else(|| uuid::Uuid::new_v4().to_string())
                                }
                            };

                            let attr_json = call
                                .arguments
                                .get("attributes")
                                .cloned()
                                .unwrap_or_else(|| "{}".into());

                            let mut data = HashMap::new();
                            if let Ok(parsed) =
                                serde_json::from_str::<HashMap<String, String>>(&attr_json)
                            {
                                data = parsed;
                            }

                            // Sincronización proactiva con la tabla de Personas
                            if category == "Person" {
                                if let Ok(persons) = case_manager.get_persons(case) {
                                    if let Some(mut person) =
                                        persons.into_iter().find(|p| p.id == id)
                                    {
                                        let mut changed = false;
                                        if let Some(email) = data.get("Email").or(data.get("email"))
                                        {
                                            person.email = Some(email.clone());
                                            changed = true;
                                        }
                                        if let Some(phone) = data
                                            .get("Phone")
                                            .or(data.get("phone"))
                                            .or(data.get("Tel"))
                                        {
                                            person.phone = Some(phone.clone());
                                            changed = true;
                                        }
                                        if let Some(dni) =
                                            data.get("DNI").or(data.get("dni")).or(data.get("ID"))
                                        {
                                            person.dni = Some(dni.clone());
                                            changed = true;
                                        }

                                        if changed {
                                            let _ = case_manager.update_person_basic(case, person);
                                        }
                                    }
                                }
                            }

                            // Construir objeto Target
                            let t_type = match t_type_str.as_str() {
                                "Domain" => TargetType::Domain,
                                "IP" => TargetType::IP,
                                "Email" => TargetType::Email,
                                "Username" => TargetType::Username,
                                "Phone" => TargetType::Phone,
                                "File" => TargetType::File,
                                "Hash" => TargetType::Hash,
                                _ => TargetType::Other,
                            };

                            let target = Target {
                                id,
                                name: name.clone(),
                                target_type: t_type,
                                category: category.clone(),
                                data,
                                linked_targets: vec![],
                                created_at: chrono::Utc::now(),
                            };

                            match case_manager.upsert_target_with_cat(case, target, &category) {
                                Ok(_) => {
                                    let _ = case_manager.log_event(
                                        case,
                                        "SUCCESS",
                                        &format!("Objetivo '{}' actualizado.", name),
                                        Some("upsert_intelligence"),
                                    );
                                    tool_results.push(format!(r#"{{"status": "OK", "message": "Objetivo '{}' actualizado correctamente."}}"#, name));
                                }
                                Err(e) => tool_results
                                    .push(format!(r#"{{"status": "ERROR", "message": "{}"}}"#, e)),
                            }
                        } else {
                            tool_results.push(
                                r#"{"status": "ERROR", "message": "No hay un caso activo."}"#
                                    .into(),
                            );
                        }
                    } else {
                        tool_results.push(format!(
                            r#"{{"status": "ERROR", "message": "Herramienta '{}' no soportada."}}"#,
                            call.tool_name
                        ));
                    }
                }

                // Inyectar resultados y seguir pensando
                let combined_results = tool_results.join("\n");
                agent.add_function_response("agent_tools", &combined_results);
            }
            AgentResponse::Error(e) => {
                return Ok(OsintResult {
                    success: false,
                    data: "".into(),
                    error: Some(e),
                })
            }
        }
    }

    Ok(OsintResult {
        success: false,
        data: "El agente excedió el límite de razonamiento.".into(),
        error: Some("Timeout Iterativo".into()),
    })
}

// --- SECRETS COMMANDS ---

#[tauri::command]
pub async fn save_secure_secret(service: String, value: String) -> Result<OsintResult, String> {
    match secrets::set_secret(&service, &value) {
        Ok(_) => Ok(OsintResult {
            success: true,
            data: format!("Secreto para {} guardado en Keyring.", service),
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
pub async fn get_secure_secret(service: String) -> Result<OsintResult, String> {
    match secrets::get_secret(&service) {
        Ok(val) => Ok(OsintResult {
            success: true,
            data: val,
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
pub async fn delete_secure_secret(service: String) -> Result<OsintResult, String> {
    match secrets::delete_secret(&service) {
        Ok(_) => Ok(OsintResult {
            success: true,
            data: format!("Secreto para {} eliminado.", service),
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
#[tauri::command]
pub fn delete_target_cmd(
    case_manager: State<'_, CaseManager>,
    case_name: String,
    target_id: String,
) -> Result<OsintResult, String> {
    match case_manager.delete_target(&case_name, &target_id) {
        Ok(_) => Ok(OsintResult {
            success: true,
            data: "Objetivo técnico eliminado correctamente.".to_string(),
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
pub fn get_activity_log_cmd(
    case_manager: State<'_, CaseManager>,
    case_name: String,
) -> Result<OsintResult, String> {
    match case_manager.get_activity_log(&case_name) {
        Ok(logs) => Ok(OsintResult {
            success: true,
            data: serde_json::to_string(&logs).unwrap_or_default(),
            error: None,
        }),
        Err(e) => Ok(OsintResult {
            success: false,
            data: "".to_string(),
            error: Some(e),
        }),
    }
}
