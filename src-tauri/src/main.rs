// src-tauri/src/main.rs
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use tauri::Emitter;
use tauri::Manager;
use tauri::State;
use tokio::sync::Mutex;

use dns_lookup::lookup_host;
use serde::Serialize;
use std::fs::File;
use std::io::BufReader;
use tauri::{command, generate_handler};

mod agent;
use agent::Agent;
use exif;
use serde_json::json;
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
        "extract_metadata" => extract_metadata(target).await,
        "web_scrape_search" => web_scrape_search(target).await,
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

// ...

#[command]
async fn ask_agent(
    app: tauri::AppHandle,
    state: State<'_, Mutex<Agent>>,
    query: String,
    image_path: Option<String>,
) -> Result<OsintResult, String> {
    dotenv::dotenv().ok();

    // Obtain the lock asynchronously
    let mut agent = state.lock().await;

    let mut turns = 0;
    let max_turns = 5;

    // Prepare initial user message with context
    let user_text = if let Some(ref path) = image_path {
        format!(
            "{}\n[System: La imagen adjunta se localiza en: {}]",
            query, path
        )
    } else {
        query.clone()
    };

    // 1. Add User Message to History (Persistence)
    agent.add_message("user", &user_text);

    // Initial inputs for the loop
    let mut current_image = image_path.clone();

    loop {
        turns += 1;
        if turns > max_turns {
            return Ok(OsintResult {
                success: false,
                data: "⚠️ El agente excedió el límite de pasos.".to_string(),
                error: Some("Max turns reached".into()),
            });
        }

        // Call Gemini
        let response = agent.think(None, current_image.as_deref()).await;

        match response {
            agent::AgentResponse::Text(text) => {
                // 2. Add Model Response to History (Persistence)
                agent.add_message("model", &text);

                return Ok(OsintResult {
                    success: true,
                    data: text,
                    error: None,
                });
            }
            agent::AgentResponse::Tools(tool_calls) => {
                let mut tool_outputs = String::new();

                for tool in tool_calls {
                    let name = tool.tool_name.as_str();
                    let args = tool.arguments;

                    let tool_result = match name {
                        "ping" => {
                            if let Some(target) = args.get("target") {
                                let res = perform_ping(target).await;
                                if res.success {
                                    res.data
                                } else {
                                    format!("Fallo: {}", res.error.unwrap_or_default())
                                }
                            } else {
                                "Error: Falta argumento 'target'".to_string()
                            }
                        }
                        "whois" => {
                            if let Some(target) = args.get("target") {
                                let res = perform_whois(target).await;
                                if res.success {
                                    res.data
                                } else {
                                    format!("Fallo: {}", res.error.unwrap_or_default())
                                }
                            } else {
                                "Error: Falta argumento 'target'".to_string()
                            }
                        }
                        "dns" => {
                            if let Some(target) = args.get("target") {
                                let res = perform_dns_lookup(target).await;
                                if res.success {
                                    res.data
                                } else {
                                    format!("Fallo: {}", res.error.unwrap_or_default())
                                }
                            } else {
                                "Error: Falta argumento 'target'".to_string()
                            }
                        }
                        "extract_metadata" => {
                            if let Some(path) = args.get("path") {
                                let res = extract_metadata(path.to_string()).await;
                                if res.success {
                                    res.data
                                } else {
                                    format!("Fallo: {}", res.error.unwrap_or_default())
                                }
                            } else {
                                "Error: Falta argumento 'path'".to_string()
                            }
                        }
                        "reverse_image_search" => {
                            if let Some(path) = args.get("path") {
                                // Emit event to frontend
                                let _ = app.emit(
                                    "open-tool",
                                    json!({ "tool": "reverse-image", "imageUrl": path }),
                                );

                                "Herramienta de búsqueda inversa abierta en la interfaz."
                                    .to_string()
                            } else {
                                "Error: Falta argumento 'path'".to_string()
                            }
                        }
                        "web_scrape_search" => {
                            if let Some(q) = args.get("query") {
                                let res = web_scrape_search(q.to_string()).await;
                                if res.success {
                                    res.data
                                } else {
                                    format!("Fallo: {}", res.error.unwrap_or_default())
                                }
                            } else {
                                "Error: Falta argumento 'query'".to_string()
                            }
                        }
                        "browse_url" => {
                            if let Some(url) = args.get("url") {
                                let res = browse_url(url.to_string()).await;
                                if res.success {
                                    res.data
                                } else {
                                    format!("Fallo: {}", res.error.unwrap_or_default())
                                }
                            } else {
                                "Error: Falta argumento 'url'".to_string()
                            }
                        }
                        _ => format!("Herramienta no encontrada: {}", name),
                    };

                    tool_outputs.push_str(&format!("\nResultado de {}: {}\n", name, tool_result));
                    // 3. Add Tool Result to History (Persistence)
                    agent.add_function_response(name, &tool_result);
                }

                // Update loop state
                current_image = None;
            }
            agent::AgentResponse::Error(e) => {
                return Ok(OsintResult {
                    success: false,
                    data: format!("Error del agente: {}", e),
                    error: Some(e),
                });
            }
        }
    }
}

#[command]
async fn extract_metadata(path: String) -> OsintResult {
    // ... (existing implementation)
    // (Ensure I don't overwrite it, but I need to place the new function after it)
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

#[command]
async fn web_scrape_search(query: String) -> OsintResult {
    let url = format!(
        "https://html.duckduckgo.com/html/?q={}",
        urlencoding::encode(&query)
    );

    // User-Agent to avoid immediate blocking
    let client = reqwest::Client::builder()
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/91.0.4472.124 Safari/537.36")
        .build()
        .unwrap_or(reqwest::Client::new());

    match client.get(&url).send().await {
        Ok(res) => {
            if let Ok(html) = res.text().await {
                let document = scraper::Html::parse_document(&html);
                let result_selector = scraper::Selector::parse(".result").unwrap();
                let title_selector = scraper::Selector::parse(".result__title").unwrap();
                let snippet_selector = scraper::Selector::parse(".result__snippet").unwrap();
                let link_selector = scraper::Selector::parse(".result__url").unwrap();

                let mut output = format!("🔍 **Resultados de Búsqueda para '{}':**\n\n", query);
                let mut count = 0;

                for element in document.select(&result_selector) {
                    if count >= 5 {
                        break;
                    }

                    let title = element
                        .select(&title_selector)
                        .next()
                        .map(|e| e.text().collect::<String>())
                        .unwrap_or_default()
                        .trim()
                        .to_string();
                    let snippet = element
                        .select(&snippet_selector)
                        .next()
                        .map(|e| e.text().collect::<String>())
                        .unwrap_or_default()
                        .trim()
                        .to_string();
                    let link = element
                        .select(&link_selector)
                        .next()
                        .map(|e| e.text().collect::<String>())
                        .unwrap_or_default()
                        .trim()
                        .to_string();

                    if !title.is_empty() {
                        output.push_str(&format!("**[{}]**\n{}\n🔗 {}\n\n", title, snippet, link));
                        count += 1;
                    }
                }

                if count == 0 {
                    output.push_str("No se encontraron resultados o el scraping fue bloqueado/cambió el formato.");
                }

                OsintResult {
                    success: true,
                    data: output,
                    error: None,
                }
            } else {
                OsintResult {
                    success: false,
                    data: "".into(),
                    error: Some("Error reading response text".into()),
                }
            }
        }
        Err(e) => OsintResult {
            success: false,
            data: "".into(),
            error: Some(format!("Request failed: {}", e)),
        },
    }
}

/// Visita una URL con Chrome headless y extrae el texto visible de la página.
/// Esto permite ver contenido de sitios que requieren JavaScript o bloquean scrapers HTTP.
async fn browse_url(url: String) -> OsintResult {
    let result = spawn_blocking(move || {
        use headless_chrome::{Browser, LaunchOptions};
        use std::time::Duration;

        let launch_options = LaunchOptions {
            headless: true,
            window_size: Some((1280, 900)),
            ..Default::default()
        };

        let browser = match Browser::new(launch_options) {
            Ok(b) => b,
            Err(e) => return Err(format!("No se pudo iniciar Chrome: {}", e)),
        };

        let tab = match browser.new_tab() {
            Ok(t) => t,
            Err(e) => return Err(format!("No se pudo abrir pestaña: {}", e)),
        };

        // Navegar a la URL
        if let Err(e) = tab.navigate_to(&url) {
            return Err(format!("Error navegando a {}: {}", url, e));
        }

        // Esperar a que cargue el contenido
        std::thread::sleep(Duration::from_secs(4));

        // Extraer texto visible via JavaScript
        let text = match tab.evaluate("document.body.innerText", false) {
            Ok(result) => result
                .value
                .and_then(|v| v.as_str().map(|s| s.to_string()))
                .unwrap_or_else(|| "No se pudo extraer texto de la página.".to_string()),
            Err(e) => return Err(format!("Error extrayendo texto: {}", e)),
        };

        // Truncar a ~4000 caracteres para no saturar el contexto del LLM
        let truncated = if text.len() > 4000 {
            format!(
                "{}...\n\n[Texto truncado, {} caracteres totales]",
                &text[..4000],
                text.len()
            )
        } else {
            text
        };

        Ok(truncated)
    })
    .await;

    match result {
        Ok(Ok(text)) => OsintResult {
            success: true,
            data: text,
            error: None,
        },
        Ok(Err(e)) => OsintResult {
            success: false,
            data: "".into(),
            error: Some(e),
        },
        Err(e) => OsintResult {
            success: false,
            data: "".into(),
            error: Some(format!("Task error: {}", e)),
        },
    }
}

#[command]
async fn download_face_models(app: tauri::AppHandle) -> OsintResult {
    let _resource_path = app
        .path()
        .resource_dir()
        .unwrap_or_default()
        .join("public")
        .join("models");
    // In dev, resource_dir might be weird, let's try to target "public/models" relative to CWD if possible, or use AppHandle to find it.
    // Actually, for a Tauri app, "public" is part of the frontend build.
    // If we want to write to it at runtime during DEV, we can target "public/models" in the project root.
    // Let's assume CWD is project root or src-tauri.

    // Better: Allow user to trigger it, and we write to "models" dir in AppData or similar, then frontend loads from there?
    // No, frontend needs to load via URL.
    // Simpler: Write to `../public/models` (assuming dev environment) OR `resources/models` (prod).
    // For now, let's try to write to `../public/models` relative to `src-tauri` (where cargo runs).

    let target_dir = std::path::Path::new("../public/models");
    if !target_dir.exists() {
        let _ = std::fs::create_dir_all(target_dir);
    }

    let base_url = "https://raw.githubusercontent.com/justadudewhohacks/face-api.js/master/weights";
    let files = vec![
        "ssd_mobilenetv1_model-weights_manifest.json",
        "ssd_mobilenetv1_model-shard1",
        "ssd_mobilenetv1_model-shard2",
        "face_landmark_68_model-weights_manifest.json",
        "face_landmark_68_model-shard1",
        "face_recognition_model-weights_manifest.json",
        "face_recognition_model-shard1",
        "face_recognition_model-shard2",
    ];

    let client = reqwest::Client::new();
    let mut messages = String::new();

    for file in files {
        let dest = target_dir.join(file);
        if !dest.exists() {
            let url = format!("{}/{}", base_url, file);
            match client.get(&url).send().await {
                Ok(resp) => {
                    if let Ok(bytes) = resp.bytes().await {
                        if let Ok(_) = std::fs::write(&dest, bytes) {
                            messages.push_str(&format!("✅ Descargado: {}\n", file));
                        } else {
                            messages.push_str(&format!("❌ Error escribiendo: {}\n", file));
                        }
                    }
                }
                Err(_) => {
                    messages.push_str(&format!("❌ Error descargando: {}\n", file));
                }
            }
        } else {
            messages.push_str(&format!("ℹ️ Ya existe: {}\n", file));
        }
    }

    OsintResult {
        success: true,
        data: messages,
        error: None,
    }
}

#[command]
async fn read_file_base64(path: String) -> OsintResult {
    use base64::Engine as _;
    match std::fs::read(&path) {
        Ok(bytes) => {
            let b64 = base64::engine::general_purpose::STANDARD.encode(bytes);
            OsintResult {
                success: true,
                data: format!("data:image/jpeg;base64,{}", b64), // Assuming jpeg/png, frontend can adjust
                error: None,
            }
        }
        Err(e) => OsintResult {
            success: false,
            data: "".into(),
            error: Some(e.to_string()),
        },
    }
}

// --- Implementations ---

// --- Main ---

fn main() {
    dotenv::dotenv().ok();
    let agent = Agent::new();

    tauri::Builder::default()
        .manage(Mutex::new(agent))
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(generate_handler![
            greet,
            run_osint_lookup,
            ask_agent,
            extract_metadata,
            web_scrape_search,
            download_face_models,
            read_file_base64
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
