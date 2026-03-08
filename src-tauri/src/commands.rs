use crate::agent::{Agent, AgentResponse};
use crate::cases::{CaseManager, ObjectiveStatus, Target};
use crate::mac_spoof;
use crate::models::{Address, Job, Nickname, OsintConfig, OsintResult, Person, SocialProfile};
use crate::secrets;
use crate::tools;
use crate::tor_manager;
use crate::AgentAbort;
use std::sync::Arc;
use tauri::{AppHandle, Manager, State};
use tokio::sync::Mutex;

#[tauri::command]
pub async fn log_info(msg: String) {
    println!("Frontend Log: {}", msg);
}

#[tauri::command]
pub async fn run_osint_lookup(
    target: String,
    tool: String,
    config: State<'_, Arc<Mutex<OsintConfig>>>,
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
    config: State<'_, Arc<Mutex<OsintConfig>>>,
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
    config: State<'_, Arc<Mutex<OsintConfig>>>,
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

// --- OBJECTIVES COMMANDS ---

#[tauri::command]
pub async fn get_objectives_cmd(
    case_name: String,
    case_manager: State<'_, Arc<CaseManager>>,
) -> Result<OsintResult, String> {
    match case_manager.get_objectives(&case_name) {
        Ok(objectives) => Ok(OsintResult {
            success: true,
            data: serde_json::to_string(&objectives).unwrap_or_else(|_| "[]".to_string()),
            error: None,
        }),
        Err(e) => Ok(OsintResult {
            success: false,
            data: "[]".to_string(),
            error: Some(e),
        }),
    }
}

#[tauri::command]
pub async fn create_objective_cmd(
    case_name: String,
    description: String,
    priority: i32,
    case_manager: State<'_, Arc<CaseManager>>,
) -> Result<OsintResult, String> {
    match case_manager.create_objective(&case_name, &description, priority) {
        Ok(objective) => Ok(OsintResult {
            success: true,
            data: serde_json::to_string(&objective).unwrap_or_else(|_| "{}".to_string()),
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
pub async fn update_objective_status_cmd(
    case_name: String,
    id: String,
    status: String,
    case_manager: State<'_, Arc<CaseManager>>,
) -> Result<OsintResult, String> {
    let obj_status = match status.as_str() {
        "Running" => ObjectiveStatus::Running,
        "Completed" => ObjectiveStatus::Completed,
        "Failed" => ObjectiveStatus::Failed,
        _ => ObjectiveStatus::Pending,
    };

    match case_manager.update_objective_status(&case_name, &id, obj_status) {
        Ok(_) => Ok(OsintResult {
            success: true,
            data: "Estado actualizado.".to_string(),
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
pub async fn delete_objective_cmd(
    case_name: String,
    id: String,
    case_manager: State<'_, Arc<CaseManager>>,
) -> Result<OsintResult, String> {
    match case_manager.delete_objective(&case_name, &id) {
        Ok(_) => Ok(OsintResult {
            success: true,
            data: "Objetivo eliminado.".to_string(),
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
    query: String,
    image_path: Option<String>,
    case_name: Option<String>,
    agent_state: State<'_, Mutex<Agent>>,
    abort_state: State<'_, AgentAbort>,
    case_manager: State<'_, Arc<CaseManager>>,
    app: AppHandle,
) -> Result<OsintResult, String> {
    let mut agent = agent_state.lock().await;

    // Obtener orquestador
    let orchestrator =
        crate::orchestrator::Orchestrator::new(app.clone(), case_manager.inner().clone());

    let case_context = if let Some(case) = &case_name {
        orchestrator.build_case_context(case)
    } else {
        String::new()
    };

    // Inicializar Skills
    let skills_dir = app.path().app_data_dir().unwrap_or_default().join("skills");
    let available_skills = crate::skills::load_skills(&skills_dir);

    let mut loop_detector = crate::loop_detector::LoopDetector::new();
    let mut current_query: Option<String> = Some(query.clone());
    let mut iterations = 0;
    let mut context_injected = false;
    const MAX_ITERATIONS: u32 = 10;

    while iterations < MAX_ITERATIONS {
        iterations += 1;

        // Chequear aborto
        if abort_state.0.load(std::sync::atomic::Ordering::SeqCst) {
            return Ok(OsintResult {
                success: false,
                data: "Interrumpido por el usuario.".into(),
                error: Some("Abortado".into()),
            });
        }

        let ctx_to_send = if !context_injected {
            context_injected = true;
            if !case_context.is_empty() {
                Some(case_context.as_str())
            } else {
                None
            }
        } else {
            None
        };

        let response = agent
            .think(
                current_query.as_deref(),
                image_path.as_deref(),
                ctx_to_send,
                iterations >= MAX_ITERATIONS - 1,
                Some(&available_skills),
            )
            .await;

        match response {
            AgentResponse::Text(t) => {
                // Guardar historial
                if let Some(case) = &case_name {
                    let _ = case_manager.save_history(
                        case,
                        &serde_json::to_string(&agent.history).unwrap_or_default(),
                    );
                }
                return Ok(OsintResult {
                    success: true,
                    data: t,
                    error: None,
                });
            }
            AgentResponse::Tools(calls) => {
                let results = orchestrator
                    .execute_tools(
                        case_name.as_deref().unwrap_or(""),
                        calls,
                        &mut loop_detector,
                        &mut agent,
                    )
                    .await;
                current_query = Some(results.join("\n---\n"));
            }
            AgentResponse::Error(e) => {
                return Ok(OsintResult {
                    success: false,
                    data: e,
                    error: Some("Error del Agente".into()),
                });
            }
        }
    }

    Ok(OsintResult {
        success: true,
        data: "Se alcanzó el límite de razonamiento. Revisá los hallazgos en el tablero.".into(),
        error: None,
    })
}

#[tauri::command]
pub async fn run_manual_wsl(command: String) -> OsintResult {
    let sudo_pass = crate::secrets::get_secret("wsl_sudo_password").ok();
    crate::tools::run_wsl_command(command, sudo_pass).await
}

// --- TELEGRAM COMMANDS ---

#[tauri::command]
pub async fn start_telegram_cmd(app: AppHandle) -> Result<String, String> {
    crate::telegram::start_telegram_polling(app)
        .await
        .map(|_| "Servicio de Telegram iniciado en background.".to_string())
}

#[tauri::command]
pub fn stop_telegram_cmd(app: AppHandle) -> Result<String, String> {
    crate::telegram::stop_telegram_polling(app)
        .map(|_| "Servicio de Telegram detenido.".to_string())
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
    case_manager: State<'_, Arc<CaseManager>>,
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
pub async fn list_cases(case_manager: State<'_, Arc<CaseManager>>) -> Result<Vec<String>, String> {
    case_manager.list_cases()
}

#[tauri::command]
pub async fn load_case(
    name: String,
    case_manager: State<'_, Arc<CaseManager>>,
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
pub async fn open_case_folder(
    case_name: String,
    case_manager: State<'_, Arc<CaseManager>>,
) -> Result<OsintResult, String> {
    let path = case_manager.get_case_path(&case_name);

    if !path.exists() {
        return Ok(OsintResult {
            success: false,
            data: "".into(),
            error: Some("La carpeta del caso no existe.".into()),
        });
    }

    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("explorer")
            .arg(path)
            .spawn()
            .map_err(|e| e.to_string())?;
    }

    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open")
            .arg(path)
            .spawn()
            .map_err(|e| e.to_string())?;
    }

    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(path)
            .spawn()
            .map_err(|e| e.to_string())?;
    }

    Ok(OsintResult {
        success: true,
        data: "Carpeta abierta.".into(),
        error: None,
    })
}

#[tauri::command]
pub async fn save_case_history(
    case_name: String,
    history_json: String,
    case_manager: State<'_, Arc<CaseManager>>,
) -> Result<(), String> {
    case_manager.save_history(&case_name, &history_json)
}

#[tauri::command]
pub async fn get_case_history(
    case_name: String,
    case_manager: State<'_, Arc<CaseManager>>,
) -> Result<String, String> {
    case_manager.load_history(&case_name)
}

#[tauri::command]
pub fn delete_case_cmd(
    case_manager: State<'_, Arc<CaseManager>>,
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
    case_manager: State<'_, Arc<CaseManager>>,
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
    case_manager: State<'_, Arc<CaseManager>>,
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
    case_manager: State<'_, Arc<CaseManager>>,
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
    case_manager: State<'_, Arc<CaseManager>>,
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
    case_manager: State<'_, Arc<CaseManager>>,
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
    case_manager: State<'_, Arc<CaseManager>>,
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
    case_manager: State<'_, Arc<CaseManager>>,
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
    case_manager: State<'_, Arc<CaseManager>>,
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
    case_manager: State<'_, Arc<CaseManager>>,
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
    case_manager: State<'_, Arc<CaseManager>>,
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
    case_manager: State<'_, Arc<CaseManager>>,
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
    case_manager: State<'_, Arc<CaseManager>>,
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
    case_manager: State<'_, Arc<CaseManager>>,
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
    case_manager: State<'_, Arc<CaseManager>>,
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
    case_manager: State<'_, Arc<CaseManager>>,
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
    case_manager: State<'_, Arc<CaseManager>>,
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

// --- MEMORY COMMANDS ---
#[tauri::command]
pub async fn add_memory_cmd(
    app: tauri::AppHandle,
    case_manager: State<'_, Arc<CaseManager>>,
    case_name: String,
    text: String,
) -> Result<String, String> {
    let memory = app.state::<crate::memory::SemanticMemoryManager>();
    let config_state = app.state::<Arc<Mutex<OsintConfig>>>();
    let config = config_state.lock().await;
    let url = config.ollama_url.clone();
    memory
        .add_memory(
            &case_manager.get_case_path(&case_name),
            &text,
            "nomic-embed-text",
            &url,
        )
        .await
        .map(|_| "Memoria guardada".to_string())
}

#[tauri::command]
pub async fn search_memory_cmd(
    app: tauri::AppHandle,
    case_manager: State<'_, Arc<CaseManager>>,
    case_name: String,
    query: String,
) -> Result<Vec<String>, String> {
    let memory = app.state::<crate::memory::SemanticMemoryManager>();
    let config_state = app.state::<Arc<Mutex<OsintConfig>>>();
    let config = config_state.lock().await;
    let url = config.ollama_url.clone();
    memory
        .search_memory(
            &case_manager.get_case_path(&case_name),
            &query,
            "nomic-embed-text",
            &url,
            5,
        )
        .await
}

#[tauri::command]
pub async fn get_ollama_models(ollama_url: String) -> Result<OsintResult, String> {
    let client = reqwest::Client::new();
    let url = format!("{}/api/tags", ollama_url);

    match client.get(url).send().await {
        Ok(resp) => {
            if resp.status().is_success() {
                let data = resp.text().await.unwrap_or_default();
                Ok(OsintResult {
                    success: true,
                    data,
                    error: None,
                })
            } else {
                Ok(OsintResult {
                    success: false,
                    data: "".into(),
                    error: Some(format!("Error de Ollama: {}", resp.status())),
                })
            }
        }
        Err(e) => Ok(OsintResult {
            success: false,
            data: "".into(),
            error: Some(format!("Error de conexión: {}", e)),
        }),
    }
}
