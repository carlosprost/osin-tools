use crate::agent::Agent;
use std::sync::Arc;
use tauri::{Manager, WindowEvent};
use tokio::sync::Mutex;

mod agent;
mod cases;
mod commands;
mod mac_spoof;
mod models;
mod tools;
mod tor_manager;

fn main() {
    dotenv::dotenv().ok();
    let agent = Agent::new();
    let tor_state = tor_manager::TorState {
        child: Arc::new(Mutex::new(None)),
    };

    tauri::Builder::default()
        .manage(Mutex::new(agent))
        .manage(Mutex::new(models::OsintConfig::default()))
        .manage(tor_state)
        .setup(|app| {
            let app_data_dir = app
                .path()
                .app_data_dir()
                .unwrap_or_else(|_| std::path::PathBuf::from("."));
            app.manage(cases::CaseManager::new(app_data_dir));
            Ok(())
        })
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            commands::greet,
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
            commands::create_target_cmd
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
