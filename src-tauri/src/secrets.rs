// src-tauri/src/secrets.rs
use keyring::Entry;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct SecretResponse {
    pub success: bool,
    pub data: Option<String>,
    pub error: Option<String>,
}

const SERVICE_PREFIX: &str = "com.sodiic.investigacion";

pub fn set_secret(service: &str, value: &str) -> Result<(), String> {
    let entry = Entry::new(SERVICE_PREFIX, service)
        .map_err(|e| format!("Error accediendo al keyring: {}", e))?;

    entry
        .set_password(value)
        .map_err(|e| format!("Error guardando secreto: {}", e))?;

    Ok(())
}

pub fn get_secret(service: &str) -> Result<String, String> {
    let entry = Entry::new(SERVICE_PREFIX, service)
        .map_err(|e| format!("Error accediendo al keyring: {}", e))?;

    entry
        .get_password()
        .map_err(|e| format!("Error obteniendo secreto: {}", e))
}

pub fn delete_secret(service: &str) -> Result<(), String> {
    let entry = Entry::new(SERVICE_PREFIX, service)
        .map_err(|e| format!("Error accediendo al keyring: {}", e))?;

    entry
        .delete_credential()
        .map_err(|e| format!("Error eliminando secreto: {}", e))
}
