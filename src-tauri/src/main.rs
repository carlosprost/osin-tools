// src-tauri/src/main.rs
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use dns_lookup::lookup_host;
use serde::Serialize;
use std::fs::File;
use std::io::BufReader;
use std::process::Command; // Removed, but keeping line for diff context? No, just remove it.
                           // Actually, easier to just replace the block.
use tauri::{command, generate_handler};

mod agent;
use agent::Agent;
use tauri::async_runtime::spawn_blocking;
use tokio::process::Command as AsyncCommand;

// --- Data Structures ---

#[derive(Serialize)]
struct OsintResult {
    success: bool,
    data: String,
    error: Option<String>,
}

// --- Commands ---

#[command]
async fn greet(name: String) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[command]
async fn run_osint_lookup(tool: String, target: String) -> OsintResult {
    match tool.as_str() {
        "ping" => perform_ping(&target).await,
        "whois" => perform_whois(&target).await,
        "dns" => perform_dns_lookup(&target).await,
        _ => OsintResult {
            success: false,
            data: String::new(),
            error: Some(format!("Tool '{}' not implemented", tool)),
        },
    }
}

// ... ask_agent stays the same ...

// --- Implementations ---

async fn perform_ping(target: &str) -> OsintResult {
    // Windows specific ping using Tokio (Async)
    // Note: ensure command is available in path
    let output = AsyncCommand::new("ping")
        .args(["-n", "4", target])
        .output()
        .await;

    match output {
        Ok(out) => {
            let result = String::from_utf8_lossy(&out.stdout).to_string();
            OsintResult {
                success: out.status.success(),
                data: result,
                error: if out.status.success() {
                    None
                } else {
                    Some("Ping failed".into())
                },
            }
        }
        Err(e) => OsintResult {
            success: false,
            data: String::new(),
            error: Some(e.to_string()),
        },
    }
}

async fn perform_whois(target: &str) -> OsintResult {
    // Basic WHOIS implementation using HTTP Fallback (Async reqwest)

    // Simple heuristic for TLD check
    let parts: Vec<&str> = target.split('.').collect();
    if parts.len() < 2 {
        return OsintResult {
            success: false,
            data: "".into(),
            error: Some("Invalid domain".into()),
        };
    }

    // Async Request
    let client = reqwest::Client::new();
    // Using networkcalc API or similar for simple fallback
    let res = client
        .get(format!("https://networkcalc.com/api/dns/lookup/{}", target))
        .send()
        .await;

    match res {
        Ok(resp) => match resp.text().await {
            Ok(text) => OsintResult {
                success: true,
                data: text,
                error: None,
            },
            Err(e) => OsintResult {
                success: false,
                data: "".into(),
                error: Some(e.to_string()),
            },
        },
        Err(e) => OsintResult {
            success: false,
            data: "".into(),
            error: Some(format!("HTTP Whois failed: {}", e)),
        },
    }
}

async fn perform_dns_lookup(target: &str) -> OsintResult {
    let target_owned = target.to_string();

    // dns_lookup is blocking, so we spawn it in a blocking thread
    let result = spawn_blocking(move || lookup_host(&target_owned)).await;

    match result {
        Ok(lookup_res) => match lookup_res {
            Ok(ips) => {
                let ip_strings: Vec<String> = ips.iter().map(|ip| ip.to_string()).collect();
                OsintResult {
                    success: true,
                    data: ip_strings.join("\n"),
                    error: None,
                }
            }
            Err(e) => OsintResult {
                success: false,
                data: "".into(),
                error: Some(e.to_string()),
            },
        },
        Err(e) => OsintResult {
            success: false,
            data: "".into(),
            error: Some(format!("Task execution failed: {}", e)),
        },
    }
}

#[command]
async fn ask_agent(query: String) -> OsintResult {
    // 1. Cargar Variables de Entorno (asegurar que se carguen)
    dotenv::dotenv().ok();

    // 2. Crear agente
    let agent = Agent::new();

    // NOTA: Para un chat real, deberíamos persistir el 'agent' o su 'history'
    // en un State de Tauri (Mutex<Agent>). Por ahora, es "stateless" por request
    // pero le pasamos la consulta actual.

    // 3. "Pensar" (Llamar a Gemini)
    match agent.think(&query).await {
        agent::AgentResponse::Text(text) => OsintResult {
            success: true,
            data: text,
            error: None,
        },
        agent::AgentResponse::Tool(tool_call) => {
            // Ejecutar herramienta
            let target = tool_call
                .arguments
                .get("target")
                .unwrap_or(&"ERROR".to_string())
                .clone();

            let result = run_osint_lookup(tool_call.tool_name.clone(), target.clone()).await;

            // En un loop real, le devolveríamos el resultado a Gemini para que genere la respuesta final.
            // Por ahora, devolvemos el resultado crudo formateado.
            OsintResult {
                success: true,
                data: format!(
                    "🔧 **Herramienta Ejecutada**: `{}`\n🎯 **Objetivo**: `{}`\n\n📄 **Resultado**:\n{}",
                    tool_call.tool_name, target, result.data
                ),
                error: None,
            }
        }
        agent::AgentResponse::Error(err) => OsintResult {
            success: false,
            data: format!("Error del Agente: {}", err),
            error: Some(err),
        },
    }
}

#[command]
async fn extract_metadata(path: String) -> OsintResult {
    let file = match File::open(&path) {
        Ok(f) => f,
        Err(e) => {
            return OsintResult {
                success: false,
                data: "".into(),
                error: Some(e.to_string()),
            }
        }
    };

    let mut reader = BufReader::new(file);
    let exifreader = exif::Reader::new();

    match exifreader.read_from_container(&mut reader) {
        Ok(exif) => {
            let mut data = String::new();
            for f in exif.fields() {
                data.push_str(&format!(
                    "{} ({}): {}\n",
                    f.tag,
                    f.ifd_num,
                    f.display_value().with_unit(&exif)
                ));
            }
            OsintResult {
                success: true,
                data,
                error: None,
            }
        }
        Err(e) => OsintResult {
            success: false,
            data: "".into(),
            error: Some(format!("No EXIF data found or error reading: {}", e)),
        },
    }
}

// --- Implementations ---

// --- Main ---

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(generate_handler![
            greet,
            run_osint_lookup,
            ask_agent,
            extract_metadata
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
