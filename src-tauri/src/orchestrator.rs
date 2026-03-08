use crate::agent::{Agent, ToolCall};
use crate::cases::{CaseManager, Target, TargetType};
use crate::loop_detector::{LoopDetector, LoopLevel, LoopResult};
use std::collections::HashMap;
use std::sync::Arc;
use tauri::{AppHandle, Emitter};
use uuid::Uuid;

pub struct Orchestrator {
    app: AppHandle,
    case_manager: Arc<CaseManager>,
}

impl Orchestrator {
    pub fn new(app: AppHandle, case_manager: Arc<CaseManager>) -> Self {
        Self { app, case_manager }
    }

    /// Construye el markdown de contexto del caso (Tablero de Hechos)
    pub fn build_case_context(&self, case_name: &str) -> String {
        let mut tech_targets = Vec::new();
        let mut persons_data = Vec::new();

        if let Ok(targets) = self.case_manager.get_targets(case_name) {
            tech_targets = targets;
        }

        if let Ok(persons) = self.case_manager.get_persons(case_name) {
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

        // 2. Personas
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
            markdown_ctx.push('\n');
        }

        format!(
            "{}\n--------------------------------------------\n",
            markdown_ctx
        )
    }

    /// Ejecuta una lista de herramientas y retorna sus resultados estructurados
    pub async fn execute_tools(
        &self,
        case_name: &str,
        calls: Vec<ToolCall>,
        loop_detector: &mut LoopDetector,
        agent: &mut Agent,
    ) -> Vec<String> {
        let mut tool_results = Vec::new();

        for call in calls {
            let tool_name = call.tool_name.as_str();
            let _ = self
                .app
                .emit("agent-status", format!("Ejecutando {}...", tool_name));

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

            agent.add_message(
                "assistant",
                &format!(
                    r#"{{"name": "{}", "parameters": {}}}"#,
                    tool_name, args_json
                ),
            );

            match tool_name {
                "registrar_actividad_tecnica" | "report_activity" | "registrar_actividad" => {
                    let msg = call.arguments.get("message").cloned().unwrap_or_default();
                    let level = call
                        .arguments
                        .get("level")
                        .cloned()
                        .unwrap_or_else(|| "INFO".to_string());
                    let _ = self.case_manager.log_event(
                        case_name,
                        &level,
                        &msg,
                        Some("report_activity"),
                    );
                    tool_results.push(
                        r#"{"status": "OK", "message": "Actividad registrada."}"#.to_string(),
                    );
                }
                "ejecutar_herramienta_linux" | "run_wsl_command" | "run_osint_lookup" => {
                    let cmd = call.arguments.get("command").cloned().unwrap_or_default();
                    let sudo_pass = crate::secrets::get_secret("wsl_sudo_password").ok();
                    let res = crate::tools::run_wsl_command(cmd.clone(), sudo_pass).await;

                    let tool_type = if cmd.contains("whois") {
                        "whois"
                    } else if cmd.contains("ping") {
                        "ping"
                    } else {
                        "cmd"
                    };
                    let clean_data = crate::tools::clean_technical_noise(&res.data, tool_type);

                    if res.success {
                        // Lógica de guardado automático de hallazgos
                        self.auto_save_findings(case_name, &cmd, &clean_data, tool_type)
                            .await;
                        tool_results.push(clean_data);
                    } else {
                        tool_results.push(format!(
                            r#"{{"status": "ERROR", "message": "{}"}}"#,
                            res.error.unwrap_or_default()
                        ));
                    }
                }
                "guardar_hallazgo" | "upsert_intelligence" => {
                    let name = call.arguments.get("name").cloned().unwrap_or_default();
                    let t_type_str = call
                        .arguments
                        .get("target_type")
                        .cloned()
                        .unwrap_or_else(|| "Other".to_string());
                    let attr_json = call
                        .arguments
                        .get("attributes")
                        .cloned()
                        .unwrap_or_else(|| "{}".to_string());

                    let t_type = match t_type_str.as_str() {
                        "Domain" => TargetType::Domain,
                        "IP" => TargetType::IP,
                        "Email" => TargetType::Email,
                        "Person" => TargetType::Person,
                        _ => TargetType::Other,
                    };

                    let data = serde_json::from_str(&attr_json).unwrap_or_default();
                    let target = Target {
                        id: Uuid::new_v4().to_string(),
                        name,
                        target_type: t_type,
                        category: "Technical".to_string(),
                        data,
                        linked_targets: vec![],
                        created_at: chrono::Utc::now(),
                    };

                    match self
                        .case_manager
                        .upsert_target_with_cat(case_name, target, "Technical")
                    {
                        Ok(_) => tool_results.push(
                            r#"{"status": "OK", "message": "Hallazgo guardado."}"#.to_string(),
                        ),
                        Err(e) => tool_results
                            .push(format!(r#"{{"status": "ERROR", "message": "{}"}}"#, e)),
                    }
                }
                _ => tool_results.push(format!(
                    r#"{{"status": "ERROR", "message": "Herramienta '{}' no reconocida."}}"#,
                    tool_name
                )),
            }
        }
        tool_results
    }

    async fn auto_save_findings(
        &self,
        case_name: &str,
        cmd: &str,
        clean_data: &str,
        tool_type: &str,
    ) {
        // Extraer target del comando (lógica simplificada)
        let target_name = cmd.split_whitespace().last().unwrap_or("").to_string();
        if target_name.is_empty() {
            return;
        }

        if let Ok(clean_json) = serde_json::from_str::<serde_json::Value>(clean_data) {
            let mut data = HashMap::new();
            let mut tech_map = serde_json::Map::new();
            tech_map.insert(tool_type.to_string(), clean_json);
            data.insert(
                "detalles_tecnicos".to_string(),
                serde_json::Value::Object(tech_map),
            );

            let target = Target {
                id: Uuid::new_v4().to_string(),
                name: target_name,
                target_type: TargetType::Other, // El orquestador debería ser más inteligente aquí
                category: "Technical".to_string(),
                data,
                linked_targets: vec![],
                created_at: chrono::Utc::now(),
            };
            let _ = self
                .case_manager
                .upsert_target_with_cat(case_name, target, "Technical");
        }
    }
}
