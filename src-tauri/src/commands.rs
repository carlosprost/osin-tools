use crate::agent::{Agent, AgentResponse};
use crate::cases::{CaseManager, Target, TargetType};
use crate::loop_detector::{LoopDetector, LoopLevel, LoopResult};
use crate::mac_spoof;
use crate::models::{Address, Job, Nickname, OsintConfig, OsintResult, Person, SocialProfile};
use crate::secrets;
use crate::skills;
use crate::tools;
use crate::tor_manager;
use crate::AgentAbort;
use std::collections::HashMap;
use tauri::{AppHandle, Emitter, Manager, State};
use tokio::sync::Mutex;

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
    config_lock: State<'_, Mutex<OsintConfig>>,
    case_manager: State<'_, CaseManager>,
) -> Result<OsintResult, String> {
    // Resetear flag de aborto global al iniciar nueva consulta
    abort_state
        .inner()
        .0
        .store(false, std::sync::atomic::Ordering::SeqCst);

    let mut agent = state.lock().await;

    {
        let conf = config_lock.lock().await;
        agent.model = conf.ollama_model.clone();
        agent.url = conf.ollama_url.clone();
    }

    // Resetear flag de aborto interno
    agent
        .abort_flag
        .store(false, std::sync::atomic::Ordering::SeqCst);

    if let Some(case) = &case_name {
        if let Ok(history) = case_manager.load_history(case.as_str()) {
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

        let mut markdown_ctx =
            String::from("### 📋 TABLERO DE HECHOS CONFIRMADOS (Memoria del Caso) ###\n\n");

        if tech_targets.is_empty() && persons_data.is_empty() {
            markdown_ctx.push_str("_No hay objetivos ni personas detectadas todavía._\n");
        }

        // 1. Objetivos Técnicos
        for t in &tech_targets {
            markdown_ctx.push_str(&format!(
                "- **OBJETIVO: {}** (Tipo: {:?}, ID: {})\n",
                t.name, t.target_type, t.id
            ));

            let mut done_tools = Vec::new();

            if let Some(tech_data) = t.data.get("detalles_tecnicos").and_then(|v| v.as_object()) {
                for (tool, fields) in tech_data {
                    done_tools.push(tool.to_uppercase());
                    markdown_ctx.push_str(&format!("  * DATOS DE {}:\n", tool.to_uppercase()));

                    if let Some(obj) = fields.as_object() {
                        for (k, v) in obj {
                            markdown_ctx.push_str(&format!(
                                "    - {}: {}\n",
                                k,
                                v.to_string().replace("\"", "")
                            ));
                        }
                    } else {
                        markdown_ctx.push_str(&format!(
                            "    - INFO: {}\n",
                            fields.to_string().replace("\"", "")
                        ));
                    }
                }
            }

            // OTROS ATRIBUTOS TÉCNICOS O GENERALES
            for (k, v) in &t.data {
                if k != "detalles_tecnicos" {
                    markdown_ctx.push_str(&format!(
                        "  * {}: {}\n",
                        k,
                        v.to_string().replace("\"", "")
                    ));
                }
            }

            if !done_tools.is_empty() {
                markdown_ctx.push_str(&format!(
                    "  * [PROHIBIDO] YA EJECUTADO: {}. No repitas estos comandos.\n",
                    done_tools.join(", ")
                ));
            }
            markdown_ctx.push('\n');
        }

        // 2. Personas (Detallado para contexto completo)
        for p in &persons_data {
            let nombre = format!(
                "{} {}",
                p.first_name.as_deref().unwrap_or(""),
                p.last_name.as_deref().unwrap_or("")
            )
            .trim()
            .to_string();

            markdown_ctx.push_str(&format!(
                "- **PERSONA: {}** (ID: {})\n",
                if nombre.is_empty() {
                    "Sin Nombre"
                } else {
                    &nombre
                },
                p.id
            ));

            if !p.nicknames.is_empty() {
                let nicks: Vec<String> = p.nicknames.iter().map(|n| n.value.clone()).collect();
                markdown_ctx.push_str(&format!("  * APODOS/ALIAS: {}\n", nicks.join(", ")));
            }

            if let Some(dni) = &p.dni {
                markdown_ctx.push_str(&format!("  * DNI: {}\n", dni));
            }
            if let Some(dob) = &p.birth_date {
                markdown_ctx.push_str(&format!("  * FECHA NACIMIENTO: {}\n", dob));
            }
            if let Some(phone) = &p.phone {
                markdown_ctx.push_str(&format!("  * TELÉFONO: {}\n", phone));
            }
            if let Some(email) = &p.email {
                markdown_ctx.push_str(&format!("  * EMAIL: {}\n", email));
            }

            if !p.addresses.is_empty() {
                markdown_ctx.push_str("  * UBICACIONES:\n");
                for addr in &p.addresses {
                    markdown_ctx.push_str(&format!(
                        "    - {} {}, {}, {} (CP: {})\n",
                        addr.street, addr.number, addr.locality, addr.state, addr.zip_code
                    ));
                }
            }

            if !p.jobs.is_empty() {
                markdown_ctx.push_str("  * HISTORIAL LABORAL:\n");
                for job in &p.jobs {
                    markdown_ctx.push_str(&format!(
                        "    - {} en {} ({} - {})\n",
                        job.title,
                        job.company,
                        job.date_start.as_deref().unwrap_or("?"),
                        job.date_end.as_deref().unwrap_or("Presente")
                    ));
                }
            }

            if !p.social_profiles.is_empty() {
                markdown_ctx.push_str("  * REDES SOCIALES / HUELLA DIGITAL:\n");
                for soc in &p.social_profiles {
                    markdown_ctx.push_str(&format!(
                        "    - {}: {} ({})\n",
                        soc.platform, soc.username, soc.url
                    ));
                }
            }

            markdown_ctx.push('\n');
        }

        case_context = format!(
            "{}\n--------------------------------------------\n",
            markdown_ctx
        );
    }

    // Inicializar Skills
    let skills_dir = app.path().app_data_dir().unwrap_or_default().join("skills");
    let available_skills = skills::load_skills(&skills_dir);

    // Inicializar Loop Detector (reemplazo avanzado de tool_streak)
    let mut loop_detector = LoopDetector::new();

    // Loop de razonamiento y ejecución de herramientas
    let mut current_query: Option<String> = Some(query.clone());
    let mut iterations = 0;
    let mut context_injected = false; // El contexto se inyecta solo una vez por sesión
    const MAX_ITERATIONS: u32 = 12; // Bajamos un poco el tope para no saturar hardware

    while iterations < MAX_ITERATIONS {
        iterations += 1;

        // --- DETECCIÓN DIRECTA DE CONSULTAS DE LECTURA ---
        // Si el usuario pide la lista de objetivos, respondemos directamente desde el contexto
        // sin necesidad de invocar al LLM (es más rápido y preciso)
        if iterations == 1 && !case_context.is_empty() {
            let lower_q = query.to_lowercase();
            let is_read_query = [
                "lista",
                "objetivos",
                "cuáles",
                "que tenes",
                "qué tenés",
                "dame los",
                "mostrame",
                "tablero",
                "perfiles",
                "personas",
                "investigación",
            ]
            .iter()
            .any(|&w| lower_q.contains(w));

            let is_informative = [
                "nombre", "apellido", "dni", "cuil", "nació", "trabaja", "cuenta", "social",
                "perfil",
            ]
            .iter()
            .any(|&w| lower_q.contains(w));

            let is_special_cmd = ["manual", "config", "ayuda", "info", "modelo"]
                .iter()
                .any(|&w| lower_q.contains(w));

            if is_read_query && query.len() < 60 && !is_informative && !is_special_cmd {
                // Lógica de respuesta directa desde el contexto JSON
                if let Some(case) = &case_name {
                    let targets = case_manager.get_targets(case).unwrap_or_default();
                    let persons = case_manager.get_persons(case).unwrap_or_default();

                    let mut respuesta = String::from("## 📋 Estado del Tablero\n\n");

                    if !targets.is_empty() {
                        respuesta.push_str("### 🎯 Objetivos Técnicos\n\n");
                        respuesta.push_str("| Nombre | Tipo | Creado |\n");
                        respuesta.push_str("|--------|------|--------| \n");
                        for t in &targets {
                            let tipo = format!("{:?}", t.target_type);
                            let nombre = &t.name;
                            let creado = t.created_at.to_rfc3339();
                            let creado_short = &creado[..10.min(creado.len())];
                            respuesta.push_str(&format!(
                                "| **{}** | {} | {} |\n",
                                nombre, tipo, creado_short
                            ));
                        }
                        respuesta.push('\n');
                    } else {
                        respuesta.push_str("_No hay objetivos técnicos registrados todavía._\n\n");
                    }

                    if !persons.is_empty() {
                        respuesta.push_str("### 👤 Personas\n\n");
                        for p in &persons {
                            let nombre = match (&p.first_name, &p.last_name) {
                                (Some(f), Some(l)) => format!("{} {}", f, l),
                                (Some(f), None) => f.clone(),
                                (None, Some(l)) => l.clone(),
                                _ => "Sin nombre".to_string(),
                            };
                            let dni_str = p
                                .dni
                                .as_deref()
                                .filter(|d| !d.is_empty())
                                .map(|d| format!(" — DNI: {}", d))
                                .unwrap_or_default();
                            respuesta.push_str(&format!("- **{}**{}\n", nombre, dni_str));
                        }
                    } else {
                        respuesta.push_str("_No hay perfiles de personas registrados todavía._\n");
                    }

                    agent.add_message("user", &query);
                    agent.add_message("assistant", &respuesta);
                    if let Some(case) = &case_name {
                        let _ = case_manager.save_history(
                            case,
                            &serde_json::to_string(&agent.history).unwrap_or_default(),
                        );
                    }
                    return Ok(OsintResult {
                        success: true,
                        data: respuesta,
                        error: None,
                    });
                }
            }
        }

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

        // Determinar si inyectar el contexto en esta iteración:
        // - Primera vez: siempre lo mandamos
        // - Resto de iteraciones: None para ahorrar tokens
        // El LoopDetector maneja el bucle internamente ahora.
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

        // Forzamos texto si estamos cerca del límite de iteraciones
        let force_text = iterations >= MAX_ITERATIONS - 2;

        let response = agent
            .think(
                current_query.as_deref(),
                image_path.as_deref(),
                ctx_to_send,
                force_text,
                Some(&available_skills),
            )
            .await;

        // Si hay un error en Ollama, cortamos con un mensaje amigable
        if let AgentResponse::Error(e) = &response {
            let fallback = format!("Che, me topé con un temita técnico: {}. Pero no te preocupes, revisá el tablero arriba que seguro la info ya está procesada.", e);
            return Ok(OsintResult {
                success: true,
                data: fallback,
                error: Some(e.clone()),
            });
        }

        match response {
            AgentResponse::Text(text) => {
                let trimmed = text.trim();
                if trimmed.is_empty() {
                    let fallback = "Che, me quedé analizando los datos en silencio. Ya tenés todo actualizado en el tablero de arriba, fijate si necesitás que profundice en algo más.".to_string();
                    // Guardar par limpio en history para memoria inter-sesión
                    agent.add_message("user", &query);
                    agent.add_message("assistant", &fallback);
                    return Ok(OsintResult {
                        success: true,
                        data: fallback,
                        error: Some("Respuesta vacía detectada".into()),
                    });
                }

                // Guardar solo el par user→assistant en history (historial limpio para memoria)
                agent.add_message("user", &query);
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
                let mut tool_results = Vec::new();

                for call in calls {
                    let tool_name = call.tool_name.as_str();
                    let _ = app.emit("agent-status", format!("Ejecutando {}...", tool_name));

                    // Serializar argumentos para el validador de loops
                    let args_json = serde_json::to_string(&call.arguments).unwrap_or_default();

                    // --- LOOP DETECTION ---
                    match loop_detector.record_call(tool_name, &args_json) {
                        LoopResult::Stuck { level, message, .. } => {
                            let status = if level == LoopLevel::Critical {
                                "CRITICAL_ERROR"
                            } else {
                                "WARNING"
                            };
                            tool_results.push(format!(
                                r#"{{"status": "{}", "message": "{}"}}"#,
                                status, message
                            ));
                            if level == LoopLevel::Critical {
                                continue;
                            }
                        }
                        LoopResult::Ok => {}
                    }

                    // Registrar la llamada en el historial para que el modelo "recuerde" lo que hizo
                    agent.add_message(
                        "assistant",
                        &format!(
                            r#"{{"name": "{}", "parameters": {}}}"#,
                            tool_name, args_json
                        ),
                    );

                    match tool_name {
                        "registrar_actividad_tecnica"
                        | "report_activity"
                        | "registrar_actividad" => {
                            if let Some(case) = &case_name {
                                let mut args = call.arguments.clone();
                                // Hallucinación de anidamiento
                                for _ in 0..2 {
                                    if let Some(obj_str) = args
                                        .get("object")
                                        .or(args.get("parameters"))
                                        .or(args.get("attributes"))
                                    {
                                        if let Ok(parsed_obj) =
                                            serde_json::from_str::<serde_json::Value>(obj_str)
                                        {
                                            if let Some(obj_map) = parsed_obj.as_object() {
                                                for (k, v) in obj_map {
                                                    let val = if v.is_string() {
                                                        v.as_str().unwrap().to_string()
                                                    } else {
                                                        v.to_string()
                                                    };
                                                    args.insert(k.clone(), val.replace("\"", ""));
                                                }
                                            }
                                        }
                                    }
                                }

                                let msg = args.get("message").cloned().unwrap_or_default();
                                let lower_msg = msg.to_lowercase();
                                let looks_like_answer = [
                                    "objetivo",
                                    "encontré",
                                    "aquí",
                                    "lista",
                                    "tenés",
                                    "está",
                                    "wolftei",
                                ]
                                .iter()
                                .any(|&w| lower_msg.contains(w) && lower_msg.len() > 30);

                                if looks_like_answer {
                                    tool_results.push(r#"{"status": "ERROR", "message": "ERROR: Estás intentando responder al usuario a través del log técnico. USÁ EL CHAT para dar respuestas directas."}"#.to_string());
                                } else {
                                    let level = args
                                        .get("level")
                                        .cloned()
                                        .unwrap_or_else(|| "INFO".to_string());
                                    let _ = case_manager.log_event(
                                        case,
                                        &level,
                                        &msg,
                                        Some("report_activity"),
                                    );
                                    tool_results.push(
                                        r#"{"status": "OK", "message": "Actividad registrada."}"#
                                            .to_string(),
                                    );
                                }
                            } else {
                                tool_results.push(
                                    r#"{"status": "ERROR", "message": "No hay un caso activo."}"#
                                        .into(),
                                );
                            }
                        }
                        "ejecutar_herramienta_linux"
                        | "run_wsl_command"
                        | "run_osint_lookup"
                        | "ejecutar_linux" => {
                            let mut cmd =
                                call.arguments.get("command").cloned().unwrap_or_default();
                            cmd = cmd
                                .trim()
                                .trim_matches(|c: char| c == '[' || c == ']')
                                .trim()
                                .to_string();

                            let lower_cmd = cmd.to_lowercase();
                            if lower_cmd.starts_with("echo ") {
                                let outcome = r#"{"status": "ERROR", "data": "ALERTA: No uses 'echo' para hablar. Escribí tu respuesta directamente como texto en el chat."}"#.to_string();
                                tool_results.push(outcome.clone());
                                loop_detector.record_result(tool_name, &args_json, &outcome);
                            } else {
                                // INHIBIDOR DINÁMICO
                                let mut inhibited = false;
                                if let Some(case) = &case_name {
                                    let norm_cmd = cmd.to_lowercase();
                                    let tool_type = if norm_cmd.contains("whois") {
                                        "whois"
                                    } else if norm_cmd.contains("ping") {
                                        "ping"
                                    } else if norm_cmd.contains("nmap") {
                                        "nmap"
                                    } else if norm_cmd.contains("dns")
                                        || norm_cmd.contains("host ")
                                        || norm_cmd.contains("dig ")
                                    {
                                        "dns"
                                    } else {
                                        "otros"
                                    };
                                    let target_in_cmd = norm_cmd
                                        .split_whitespace()
                                        .last()
                                        .unwrap_or("")
                                        .to_string();
                                    if let Ok(targets) = case_manager.get_targets(case) {
                                        if let Some(matched) = targets
                                            .iter()
                                            .find(|t| t.name.to_lowercase() == target_in_cmd)
                                        {
                                            if matched
                                                .data
                                                .get("detalles_tecnicos")
                                                .and_then(|v| v.as_object())
                                                .map(|obj| obj.contains_key(tool_type))
                                                .unwrap_or(false)
                                            {
                                                let outcome = format!(
                                                    r#"{{"status": "ERROR", "data": "ERROR: Ya tenés los resultados de {} para '{}' en el TABLERO DE HECHOS."}}"#,
                                                    tool_type.to_uppercase(),
                                                    target_in_cmd
                                                );
                                                tool_results.push(outcome);
                                                inhibited = true;
                                            }
                                        }
                                    }
                                }

                                if !inhibited {
                                    // Sanitarizar ping
                                    if lower_cmd.starts_with("ping") && !lower_cmd.contains(" -c ")
                                    {
                                        cmd = cmd.replacen("ping ", "ping -c 4 ", 1);
                                    }

                                    let sudo_pass =
                                        crate::secrets::get_secret("wsl_sudo_password").ok();
                                    let res =
                                        crate::tools::run_wsl_command(cmd.clone(), sudo_pass).await;

                                    let tool_type = if cmd.contains("whois") {
                                        "whois"
                                    } else if cmd.contains("ping") {
                                        "ping"
                                    } else if cmd.contains("nslookup") || cmd.contains("dig") {
                                        "dns"
                                    } else if cmd.contains("nmap") {
                                        "nmap"
                                    } else if cmd.contains("curl") {
                                        "http"
                                    } else {
                                        "cmd"
                                    };
                                    let clean_data =
                                        crate::tools::clean_technical_noise(&res.data, tool_type);

                                    if res.success {
                                        if let Some(case) = &case_name {
                                            let target_name = cmd
                                                .split_whitespace()
                                                .filter(|s| !s.starts_with('-'))
                                                .last()
                                                .unwrap_or("")
                                                .to_string();
                                            if !target_name.is_empty() {
                                                let targets = case_manager
                                                    .get_targets(case)
                                                    .unwrap_or_default();
                                                let norm_target = target_name.to_lowercase();
                                                let matched = targets
                                                    .iter()
                                                    .find(|t| t.name.to_lowercase() == norm_target);
                                                let is_new = matched.is_none();
                                                let existing_id =
                                                    matched.map(|t| t.id.clone()).unwrap_or_else(
                                                        || uuid::Uuid::new_v4().to_string(),
                                                    );
                                                let mut existing_data = matched
                                                    .map(|t| t.data.clone())
                                                    .unwrap_or_default();

                                                if let Ok(clean_json) =
                                                    serde_json::from_str::<serde_json::Value>(
                                                        &clean_data,
                                                    )
                                                {
                                                    existing_data.remove(tool_type);
                                                    let mut tech_map = existing_data
                                                        .get("detalles_tecnicos")
                                                        .and_then(|v| v.as_object())
                                                        .cloned()
                                                        .unwrap_or_default();
                                                    if let Some(wrapped) =
                                                        clean_json.get("detalles_tecnicos")
                                                    {
                                                        if let Some(tools_obj) = wrapped.as_object()
                                                        {
                                                            for (k, v) in tools_obj {
                                                                tech_map
                                                                    .insert(k.clone(), v.clone());
                                                            }
                                                        }
                                                    } else {
                                                        tech_map.insert(
                                                            tool_type.to_string(),
                                                            clean_json,
                                                        );
                                                    }
                                                    existing_data.insert(
                                                        "detalles_tecnicos".to_string(),
                                                        serde_json::Value::Object(tech_map),
                                                    );
                                                }

                                                let t_type = matched
                                                    .map(|t| t.target_type.clone())
                                                    .unwrap_or(crate::cases::TargetType::Domain);
                                                let upsert_target = crate::cases::Target {
                                                    id: existing_id,
                                                    name: target_name.clone(),
                                                    target_type: t_type,
                                                    category: "Technical".to_string(),
                                                    data: existing_data,
                                                    linked_targets: Vec::new(),
                                                    created_at: chrono::Utc::now(),
                                                };
                                                let _ = case_manager.upsert_target_with_cat(
                                                    case,
                                                    upsert_target,
                                                    "Technical",
                                                );
                                                let _ = case_manager.log_event(
                                                    case,
                                                    "INFO",
                                                    &format!(
                                                        "[AUTO] Objetivo '{}' {} con datos.",
                                                        target_name,
                                                        if is_new {
                                                            "CREADO"
                                                        } else {
                                                            "actualizado"
                                                        }
                                                    ),
                                                    Some("auto_persist"),
                                                );
                                            }
                                        }
                                    }

                                    let outcome = format!(
                                        r#"{{"status": "{}", "data": "{}"}}"#,
                                        if res.success { "OK" } else { "ERROR" },
                                        "Ejecución finalizada".to_string()
                                    );
                                    tool_results.push(outcome.clone());
                                    loop_detector.record_result(tool_name, &args_json, &outcome);
                                }
                            }
                        }
                        "guardar_hallazgo" | "upsert_intelligence" => {
                            if let Some(case) = &case_name {
                                let name = call.arguments.get("name").cloned().unwrap_or_default();
                                let target_type_str = call
                                    .arguments
                                    .get("target_type")
                                    .cloned()
                                    .unwrap_or_default();
                                let attributes_str = call
                                    .arguments
                                    .get("attributes")
                                    .cloned()
                                    .unwrap_or_else(|| "{}".to_string());

                                if !name.is_empty() {
                                    let t_type = match target_type_str.to_lowercase().as_str() {
                                        "domain" => TargetType::Domain,
                                        "ip" => TargetType::IP,
                                        "email" => TargetType::Email,
                                        "person" => TargetType::Person,
                                        "username" => TargetType::Username,
                                        "phone" => TargetType::Phone,
                                        _ => TargetType::Other,
                                    };

                                    let mut data = HashMap::new();
                                    if let Ok(attr_json) =
                                        serde_json::from_str::<serde_json::Value>(&attributes_str)
                                    {
                                        if let Some(obj) = attr_json.as_object() {
                                            for (k, v) in obj {
                                                data.insert(k.clone(), v.clone());
                                            }
                                        }
                                    }

                                    let target_id = uuid::Uuid::new_v4().to_string();
                                    let target = Target {
                                        id: target_id,
                                        name: name.clone(),
                                        target_type: t_type,
                                        category: "Intelligence".to_string(),
                                        data,
                                        linked_targets: Vec::new(),
                                        created_at: chrono::Utc::now(),
                                    };

                                    let _ = case_manager.upsert_target_with_cat(
                                        case,
                                        target,
                                        "Intelligence",
                                    );
                                    let _ = case_manager.log_event(
                                        case,
                                        "SUCCESS",
                                        &format!("Hallazgo guardado: {}", name),
                                        Some("guardar_hallazgo"),
                                    );

                                    tool_results.push(format!(r#"{{"status": "OK", "message": "Objetivo '{}' guardado en el tablero."}}"#, name));
                                } else {
                                    tool_results.push(
                                        r#"{"status": "ERROR", "message": "Falta nombre del descubrimiento."}"#
                                            .to_string(),
                                    );
                                }
                            }
                        }
                        "scrape_generic" | "scrape_social" => {
                            let url = call.arguments.get("url").cloned().unwrap_or_default();
                            if !url.is_empty() {
                                let res = if tool_name == "scrape_generic" {
                                    crate::tools::scrape_generic(url).await
                                } else {
                                    crate::tools::scrape_social(url).await
                                };
                                let outcome = if res.success {
                                    serde_json::json!({"status": "OK", "data": res.data})
                                        .to_string()
                                } else {
                                    serde_json::json!({"status": "ERROR", "message": res.error})
                                        .to_string()
                                };
                                tool_results.push(outcome);
                            }
                        }
                        "ver_configuracion" => {
                            let cfg = config_lock.lock().await;
                            let mut info = String::from("### ⚙️ Configuración Actual\n\n");
                            info.push_str(&format!(
                                "- **Modelo Ollama:** `{}`\n",
                                cfg.ollama_model
                            ));
                            info.push_str(&format!("- **Nodo Ollama:** `{}`\n", cfg.ollama_url));
                            info.push_str(&format!(
                                "- **Proxy:** `{}`\n",
                                if cfg.proxy_url.is_empty() {
                                    "No configurado"
                                } else {
                                    &cfg.proxy_url
                                }
                            ));
                            info.push_str(&format!(
                                "- **Tor:** `{}`\n",
                                if cfg.tor_active { "Activo" } else { "Inactivo" }
                            ));

                            let apis = [
                                ("Shodan", &cfg.shodan),
                                ("VirusTotal", &cfg.virustotal),
                                ("Hunter.io", &cfg.hunter_io),
                                ("HIBP", &cfg.hibp_api_key),
                            ];

                            info.push_str("\n**APIs Externas:**\n");
                            for (name, val) in apis {
                                info.push_str(&format!(
                                    "- {}: {}\n",
                                    name,
                                    if val.is_empty() {
                                        "❌ Faltante"
                                    } else {
                                        "✅ Configurada"
                                    }
                                ));
                            }

                            tool_results.push(format!(
                                r#"{{"status": "OK", "data": "{}"}}"#,
                                info.replace("\n", "\\n").replace("\"", "\\\"")
                            ));
                        }
                        "actualizar_configuracion" => {
                            let key = call.arguments.get("key").cloned().unwrap_or_default();
                            let value = call.arguments.get("value").cloned().unwrap_or_default();

                            if !key.is_empty() {
                                let mut cfg = config_lock.lock().await;
                                let mut updated = true;
                                let mut is_secret = false;

                                let norm_key = key.to_lowercase();
                                match norm_key.as_str() {
                                    "ollama_model" | "modelo" => cfg.ollama_model = value.clone(),
                                    "ollama_url" | "url" => cfg.ollama_url = value.clone(),
                                    "shodan" => {
                                        cfg.shodan = value.clone();
                                        is_secret = true;
                                    }
                                    "virustotal" => {
                                        cfg.virustotal = value.clone();
                                        is_secret = true;
                                    }
                                    "hunter_io" => {
                                        cfg.hunter_io = value.clone();
                                        is_secret = true;
                                    }
                                    "hibp" => {
                                        cfg.hibp_api_key = value.clone();
                                        is_secret = true;
                                    }
                                    "proxy_url" => cfg.proxy_url = value.clone(),
                                    _ => updated = false,
                                }

                                if updated {
                                    if is_secret {
                                        // Persistir secretos de forma segura si el agente los cambia
                                        let _ = crate::secrets::set_secret(&norm_key, &value);
                                    }
                                    tool_results.push(format!(r#"{{"status": "OK", "message": "Ajuste '{}' actualizado exitosamente."}}"#, key));
                                } else {
                                    tool_results.push(format!(r#"{{"status": "ERROR", "message": "El ajuste '{}' no es reconocido o no se puede editar directamente."}}"#, key));
                                }
                            }
                        }
                        "obtener_ayuda" => {
                            let manual_path = std::path::Path::new("MANUAL_INTERACTIVO.md");
                            match std::fs::read_to_string(manual_path) {
                                Ok(content) => {
                                    tool_results.push(format!(
                                        r#"{{"status": "OK", "data": "{}"}}"#,
                                        content.replace("\n", "\\n").replace("\"", "\\\"")
                                    ));
                                }
                                Err(_) => {
                                    tool_results.push(r#"{"status": "ERROR", "message": "No se pudo cargar el manual interactivo."}"#.to_string());
                                }
                            }
                        }
                        _ => {
                            tool_results.push(format!(r#"{{"status": "ERROR", "message": "Herramienta '{}' no soportada."}}"#, tool_name));
                        }
                    }

                    // Registrar resultado en historial para persistencia de razonamiento
                    if let Some(res) = tool_results.last() {
                        agent.add_function_response(tool_name, res);
                    }
                }

                let combined_results = tool_results.join("\n---\n");
                current_query = Some(format!(
                    "SISTEMA: Se procesaron las herramientas. Aquí tenés los resultados consolidados para que sigas con tu análisis:\n{}",
                    combined_results
                ));
            }
            AgentResponse::Error(_) => unreachable!("Error handled above"),
        }
    }

    Ok(OsintResult {
        success: true,
        data: "Che, me quedé sin hilos para tirar (límite de razonamiento alcanzado). Revisamos lo que encontré hasta ahora en el tablero de arriba.".into(),
        error: Some("Timeout Iterativo".into()),
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
pub async fn open_case_folder(
    case_name: String,
    case_manager: State<'_, CaseManager>,
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

// --- MEMORY COMMANDS ---
#[tauri::command]
pub async fn add_memory_cmd(
    app: tauri::AppHandle,
    case_manager: tauri::State<'_, crate::cases::CaseManager>,
    case_name: String,
    text: String,
) -> Result<String, String> {
    let memory = app.state::<crate::memory::SemanticMemoryManager>();
    let config = app.state::<tokio::sync::Mutex<crate::models::OsintConfig>>();
    let url = config.lock().await.ollama_url.clone();
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
    case_manager: tauri::State<'_, crate::cases::CaseManager>,
    case_name: String,
    query: String,
) -> Result<Vec<String>, String> {
    let memory = app.state::<crate::memory::SemanticMemoryManager>();
    let config = app.state::<tokio::sync::Mutex<crate::models::OsintConfig>>();
    let url = config.lock().await.ollama_url.clone();
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
