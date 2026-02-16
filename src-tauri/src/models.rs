// src-tauri/src/models.rs
use serde::{Deserialize, Serialize};

#[derive(Serialize)]
pub struct OsintResult {
    pub success: bool,
    pub data: String,
    pub error: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct OsintConfig {
    pub hunter_io: String,
    pub shodan: String,
    pub virustotal: String,
    pub ipapi: String,
    pub hibp_api_key: String,
    pub proxy_url: String,
    pub tor_active: bool,
    pub mac_masking_active: bool,
    pub original_mac: String,
    // Cookies de Sesión para OSINT Autenticado
    pub linkedin_session: String,
    pub instagram_session: String,
    pub twitter_session: String,
    pub facebook_session: String,
    pub spotify_client_id: String,
    pub spotify_client_secret: String,
}

impl Default for OsintConfig {
    fn default() -> Self {
        Self {
            hunter_io: String::new(),
            shodan: String::new(),
            virustotal: String::new(),
            ipapi: String::new(),
            hibp_api_key: String::new(),
            proxy_url: String::new(),
            tor_active: false,
            mac_masking_active: false,
            original_mac: String::new(),
            linkedin_session: String::new(),
            instagram_session: String::new(),
            twitter_session: String::new(),
            facebook_session: String::new(),
            spotify_client_id: String::new(),
            spotify_client_secret: String::new(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Address {
    pub id: Option<String>,
    pub street: String,
    pub number: String,
    pub locality: String,
    pub state: String,
    pub country: String,
    pub zip_code: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Job {
    pub id: Option<String>,
    pub title: String,
    pub company: String,
    pub date_start: Option<String>,
    pub date_end: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SocialProfile {
    pub id: Option<String>,
    pub platform: String,
    pub username: String,
    pub url: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Nickname {
    pub id: Option<String>,
    pub value: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Person {
    pub id: String,                 // UUID
    pub first_name: Option<String>, // Ahora opcional
    pub last_name: Option<String>,
    pub nicknames: Vec<Nickname>, // Nuevo campo
    pub dni: Option<String>,
    pub birth_date: Option<String>, // ISO 8601
    pub phone: Option<String>,
    pub email: Option<String>,
    pub addresses: Vec<Address>,
    pub jobs: Vec<Job>,
    pub social_profiles: Vec<SocialProfile>,
    pub created_at: String,
}
