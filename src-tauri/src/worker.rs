use crate::agent::{Agent, AgentResponse};
use crate::cases::{CaseManager, Objective, ObjectiveStatus};
use crate::loop_detector::LoopDetector;
use crate::models::OsintConfig;
use crate::orchestrator::Orchestrator;
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Manager};
use tokio::sync::Mutex;

pub struct BackgroundWorker {
    app_handle: AppHandle,
    orchestrator: Arc<Orchestrator>,
    case_manager: Arc<CaseManager>,
}

impl BackgroundWorker {
    pub fn new(app_handle: AppHandle) -> Self {
        let case_manager = app_handle.state::<Arc<CaseManager>>().inner().clone();
        let orchestrator = Orchestrator::new(app_handle.clone(), case_manager.clone());

        Self {
            app_handle,
            orchestrator: Arc::new(orchestrator),
            case_manager,
        }
    }

    pub async fn start_processing(&self) {
        println!("INFO [Worker]: Bucle de fondo iniciado.");
        loop {
            if let Ok(cases) = self.case_manager.list_cases() {
                for case_name in cases {
                    if let Ok(objectives) = self.case_manager.get_objectives(&case_name) {
                        for mut obj in objectives {
                            if let ObjectiveStatus::Pending = obj.status {
                                self.process_objective(&case_name, &mut obj).await;
                            }
                        }
                    }
                }
            }
            tokio::time::sleep(tokio::time::Duration::from_secs(20)).await;
        }
    }

    async fn process_objective(&self, case_name: &str, objective: &mut Objective) {
        let _ = self.case_manager.update_objective_status(
            case_name,
            &objective.id,
            ObjectiveStatus::Running,
        );
        let _ = self.app_handle.emit(
            "objective-status",
            format!("Worker: {}", objective.description),
        );

        let mut agent = Agent::new();
        {
            let config_state = self.app_handle.state::<Arc<Mutex<OsintConfig>>>();
            let config = config_state.lock().await;
            agent.model = config.ollama_model.clone();
            agent.url = config.ollama_url.clone();
        }

        let context = self.orchestrator.build_case_context(case_name);
        let mut loop_detector = LoopDetector::new();

        let mut iterations = 0;
        let max_iterations = 6;
        let mut current_query = Some(format!("OBJETIVO AUTÓNOMO: {}", objective.description));

        while iterations < max_iterations {
            iterations += 1;

            let skills_dir = self
                .app_handle
                .path()
                .app_data_dir()
                .unwrap_or_default()
                .join("skills");
            let available_skills = crate::skills::load_skills(&skills_dir);

            let response = agent
                .think(
                    current_query.as_deref(),
                    None,
                    Some(&context),
                    false,
                    Some(&available_skills),
                )
                .await;

            match response {
                AgentResponse::Text(t) => {
                    let _ = self.case_manager.log_event(
                        case_name,
                        "SUCCESS",
                        &format!("Objetivo finalizado: {}", t),
                        Some("BackgroundWorker"),
                    );
                    let _ = self.case_manager.update_objective_status(
                        case_name,
                        &objective.id,
                        ObjectiveStatus::Completed,
                    );
                    break;
                }
                AgentResponse::Tools(calls) => {
                    let results = self
                        .orchestrator
                        .execute_tools(case_name, calls, &mut loop_detector, &mut agent)
                        .await;
                    current_query = Some(results.join("\n---\n"));
                }
                AgentResponse::Error(_) => {
                    let _ = self.case_manager.update_objective_status(
                        case_name,
                        &objective.id,
                        ObjectiveStatus::Failed,
                    );
                    break;
                }
            }
        }
    }
}
