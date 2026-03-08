use crate::agent::Agent;
use std::sync::Arc;
use tauri::{Manager, WindowEvent};
use tokio::sync::Mutex;

mod agent;
mod cases;
mod commands;
mod loop_detector;
mod mac_spoof;
mod memory;
mod models;
mod orchestrator;
mod scraper;
mod secrets;
mod skills;
mod telegram;
mod tools;
mod tor_manager;
mod worker;

pub struct AgentAbort(pub Arc<std::sync::atomic::AtomicBool>);

fn main() {
    let agent = Agent::new();
    let config = Arc::new(Mutex::new(models::OsintConfig::default()));
    let abort_flag = agent.abort_flag.clone();
    let tor_state = tor_manager::TorState {
        child: Arc::new(Mutex::new(None)),
    };

    tauri::Builder::default()
        .manage(Mutex::new(agent))
        .manage(AgentAbort(abort_flag))
        .manage(telegram::TelegramState::default())
        .manage(memory::SemanticMemoryManager::new())
        .manage(config.clone())
        .manage(tor_state)
        .setup(|app| {
            let app_data_dir = app
                .path()
                .app_data_dir()
                .unwrap_or_else(|_| std::path::PathBuf::from("."));
            let case_manager = Arc::new(cases::CaseManager::new(app_data_dir));
            app.manage(case_manager.clone());

            // Iniciar Background Worker
            let app_handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                let worker = worker::BackgroundWorker::new(app_handle);
                worker.start_processing().await;
            });

            Ok(())
        })
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            commands::run_osint_lookup,
            commands::ask_agent,
            commands::extract_metadata,
            commands::web_scrape_search,
            commands::download_face_models,
            commands::read_file_base64,
            commands::update_osint_config,
            commands::set_tor_active,
            commands::set_mac_masking,
            commands::abort_agent,
            commands::log_info,
            commands::create_case,
            commands::list_cases,
            commands::delete_case_cmd,
            commands::load_case,
            commands::open_case_folder,
            commands::save_case_history,
            commands::get_case_history,
            // Personas CRUD
            commands::create_person_cmd,
            commands::update_person_cmd,
            commands::get_persons_cmd,
            commands::delete_person_cmd,
            commands::add_nickname_cmd,
            commands::remove_nickname_cmd,
            commands::add_address_cmd,
            commands::remove_address_cmd,
            commands::add_job_cmd,
            commands::remove_job_cmd,
            commands::add_social_cmd,
            commands::remove_social_cmd,
            // Technical Targets
            commands::get_targets_json_cmd,
            commands::create_target_cmd,
            commands::delete_target_cmd,
            commands::get_activity_log_cmd,
            // Secrets
            commands::save_secure_secret,
            commands::get_secure_secret,
            commands::delete_secure_secret,
            commands::run_manual_wsl,
            // Telegram Bot
            commands::start_telegram_cmd,
            commands::stop_telegram_cmd,
            // Semantic Memory
            commands::add_memory_cmd,
            commands::search_memory_cmd,
            commands::get_ollama_models,
            // Objetivos
            commands::get_objectives_cmd,
            commands::create_objective_cmd,
            commands::update_objective_status_cmd,
            commands::delete_objective_cmd
        ])
        .on_window_event(|window, event| {
            if let WindowEvent::Destroyed = event {
                let app = window.app_handle();
                tauri::async_runtime::block_on(async move {
                    tor_manager::stop_tor(app).await;
                });
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
