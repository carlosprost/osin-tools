// src-tauri/src/tor_manager.rs
use std::net::TcpStream;
use std::sync::Arc;
use std::time::Duration;
use tauri::{AppHandle, Manager};
use tauri_plugin_shell::ShellExt;
use tokio::sync::Mutex;

pub struct TorState {
    pub child: Arc<Mutex<Option<tauri_plugin_shell::process::CommandChild>>>,
}

pub async fn start_tor(app: &AppHandle) -> Result<(), String> {
    let shell = app.shell();

    // Rutas de recursos
    let resource_dir = app.path().resource_dir().map_err(|e| e.to_string())?;
    let geoip_path = resource_dir.join("resources/tor/geoip");
    let geoip6_path = resource_dir.join("resources/tor/geoip6");

    // Directorio de datos de la app
    let app_data = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let tor_data_dir = app_data.join("tor_data");
    std::fs::create_dir_all(&tor_data_dir).ok();

    println!("Iniciando Sidecar de Tor...");
    let sidecar = shell.sidecar("tor").map_err(|e| e.to_string())?.args([
        "--SocksPort",
        "9050",
        "--DataDirectory",
        tor_data_dir.to_str().unwrap(),
        "--GeoIPFile",
        geoip_path.to_str().unwrap(),
        "--GeoIPv6File",
        geoip6_path.to_str().unwrap(),
        "--Log",
        "notice stdout",
    ]);

    let (mut rx, child) = sidecar.spawn().map_err(|e| e.to_string())?;
    let (tx_ready, mut rx_ready) = tokio::sync::mpsc::channel::<bool>(1);

    // Monitorear logs de Tor para bootstrapping y depuración
    tauri::async_runtime::spawn(async move {
        let mut ready_sent = false;
        while let Some(event) = rx.recv().await {
            match event {
                tauri_plugin_shell::process::CommandEvent::Stdout(line) => {
                    let out = String::from_utf8_lossy(&line);
                    let trimmed = out.trim();
                    println!("Tor STDOUT: {}", trimmed);

                    if trimmed.contains("Bootstrapped 100%") && !ready_sent {
                        let _ = tx_ready.send(true).await;
                        ready_sent = true;
                    }
                }
                tauri_plugin_shell::process::CommandEvent::Stderr(line) => {
                    let out = String::from_utf8_lossy(&line);
                    eprintln!("Tor STDERR: {}", out.trim());
                }
                _ => {}
            }
        }
    });

    let state = app.state::<TorState>();
    let mut lock = state.child.lock().await;
    *lock = Some(child);

    // Esperar a que Tor esté realmente listo (Bootstrapped 100%)
    println!("Dando tiempo a Tor para establecer circuitos (Bootstrapping)...");
    let timeout = Duration::from_secs(60);

    match tokio::time::timeout(timeout, rx_ready.recv()).await {
        Ok(Some(true)) => {
            println!("Tor está 100% listo.");
        }
        _ => {
            // Fallback: verificar al menos si el puerto está abierto si el log falla
            if TcpStream::connect("127.0.0.1:9050").is_err() {
                return Err("Tor no pudo conectarse a la red en el tiempo esperado.".into());
            }
            println!("Advertencia: No se detectó 'Bootstrapped 100%' en logs, pero el puerto está abierto.");
        }
    }

    Ok(())
}

pub async fn stop_tor(app: &AppHandle) {
    let state = app.state::<TorState>();
    let mut lock = state.child.lock().await;
    if let Some(child) = lock.take() {
        println!("Deteniendo servicio Tor...");
        let _ = child.kill();
    }
}
