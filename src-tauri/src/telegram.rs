use crate::agent::{Agent, AgentResponse};
use crate::secrets;
use reqwest::Client;
use serde_json::Value;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tauri::{AppHandle, Manager};
use tokio::sync::Mutex;

const TELEGRAM_API_URL: &str = "https://api.telegram.org/bot";

pub struct TelegramState {
    pub is_running: Arc<AtomicBool>,
}

impl Default for TelegramState {
    fn default() -> Self {
        Self {
            is_running: Arc::new(AtomicBool::new(false)),
        }
    }
}

pub async fn start_telegram_polling(app: AppHandle) -> Result<(), String> {
    let state = app.state::<TelegramState>();

    // Check if already running
    if state.is_running.load(Ordering::SeqCst) {
        return Err("Telegram bot is already running.".to_string());
    }

    let token = secrets::get_secret("telegram_token")
        .map_err(|_| "Telegram Token not found in Keyring. Please configure it.".to_string())?;

    let admin_id_str = secrets::get_secret("telegram_admin_id")
        .map_err(|_| "Telegram Admin ID not found in Keyring. Please configure it.".to_string())?;

    let admin_id: i64 = admin_id_str
        .parse()
        .map_err(|_| "Invalid Admin ID format.".to_string())?;

    state.is_running.store(true, Ordering::SeqCst);
    let is_running_clone = state.is_running.clone();

    tokio::spawn(async move {
        let client = Client::new();
        let mut last_update_id: i64 = 0;

        println!("🟢 Telegram Polling Started for Admin: {}", admin_id);

        while is_running_clone.load(Ordering::SeqCst) {
            let url = format!(
                "{}{}getUpdates?offset={}&timeout=10",
                TELEGRAM_API_URL,
                token,
                last_update_id + 1
            );

            match client.get(&url).send().await {
                Ok(response) => {
                    if let Ok(json) = response.json::<Value>().await {
                        if let Some(result_arr) = json.get("result").and_then(|r| r.as_array()) {
                            for update in result_arr {
                                if let Some(update_id) =
                                    update.get("update_id").and_then(|u| u.as_i64())
                                {
                                    last_update_id = update_id;
                                }

                                // Extraer mensaje
                                if let Some(message) = update.get("message") {
                                    let chat_id = message
                                        .get("chat")
                                        .and_then(|c| c.get("id"))
                                        .and_then(|id| id.as_i64())
                                        .unwrap_or(0);

                                    if chat_id != admin_id {
                                        println!("⚠️ Unauthorized Telegram access attempt from Chat ID: {}", chat_id);
                                        continue;
                                    }

                                    if let Some(text) = message.get("text").and_then(|t| t.as_str())
                                    {
                                        println!("📥 [Telegram] Recibido: {}", text);

                                        // Forward al agente
                                        let reply = process_message_with_agent(&app, text).await;

                                        // Responder
                                        let send_url =
                                            format!("{}{}sendMessage", TELEGRAM_API_URL, token);
                                        let _ = client
                                            .post(&send_url)
                                            .json(&serde_json::json!({
                                                "chat_id": chat_id,
                                                "text": reply
                                            }))
                                            .send()
                                            .await;
                                    }
                                }
                            }
                        }
                    }
                }
                Err(e) => {
                    println!("🔴 Telegram Polling Error: {}", e);
                    tokio::time::sleep(Duration::from_secs(5)).await;
                }
            }

            // Pausa breve para no reventar CPU local si baja la red
            tokio::time::sleep(Duration::from_millis(500)).await;
        }

        println!("🛑 Telegram Polling Stopped.");
    });

    Ok(())
}

pub fn stop_telegram_polling(app: AppHandle) -> Result<(), String> {
    let state = app.state::<TelegramState>();
    state.is_running.store(false, Ordering::SeqCst);
    Ok(())
}

async fn process_message_with_agent(app: &AppHandle, text: &str) -> String {
    let agent_mutex = app.state::<Mutex<Agent>>();
    let mut agent = agent_mutex.lock().await;

    // Solo contexto básico (no adjuntamos casos por Telegram para esta V1)
    agent.add_message("user", text);

    // Acá simulamos el think sin contexto adicional. En la implementación real habría que cargar skills
    // Si queremos invocar a ask_agent (commands.rs) se nos complican las dependencias circulares,
    // así que interactuamos temporalmente de forma básica para la refactorización profunda después.
    // Esto es un placeholder hasta inyectar adecuadamente los casos y LoopDetectors.

    match agent.think(None, None, None, false, None).await {
        AgentResponse::Text(txt) => {
            agent.add_message("assistant", &txt);
            txt
        }
        AgentResponse::Error(e) => {
            format!("Error invocando modelo: {}", e)
        }
        _ => "Investigación disparada en Background o comandos OSINT. (UI Action Required)"
            .to_string(), // Tools return no soportados directo por chat sin handler real aún
    }
}
