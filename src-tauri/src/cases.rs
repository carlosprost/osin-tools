use crate::models::{Address, Job, Nickname, Person, SocialProfile};
use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, Result as SqlResult};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use uuid::Uuid;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum TargetType {
    Person,
    Domain,
    IP,
    Email,
    Username,
    Phone,
    File,
    Hash,
    Other,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum ObjectiveStatus {
    Pending,
    Running,
    Completed,
    Failed,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Objective {
    pub id: String,
    pub description: String,
    pub status: ObjectiveStatus,
    pub priority: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TargetLink {
    pub target_id: String,
    pub relation: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Target {
    pub id: String,
    pub name: String,
    pub target_type: TargetType,
    pub category: String, // Nueva categoría persistente
    pub data: HashMap<String, serde_json::Value>,
    pub linked_targets: Vec<TargetLink>,
    pub created_at: DateTime<Utc>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct LogEntry {
    pub id: String,
    pub timestamp: String,
    pub level: String,
    pub message: String,
    pub tool_name: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CaseMetadata {
    pub name: String,
    pub description: String,
    pub created_at: DateTime<Utc>,
    pub last_modified: DateTime<Utc>,
    pub targets: Vec<Target>,
}

pub struct CaseManager {
    base_path: PathBuf,
}

impl CaseManager {
    pub fn new(app_data_dir: PathBuf) -> Self {
        let base_path = app_data_dir.join("investigaciones");
        if !base_path.exists() {
            let _ = fs::create_dir_all(&base_path);
        }
        CaseManager { base_path }
    }

    pub fn get_case_path(&self, case_name: &str) -> PathBuf {
        self.base_path.join(case_name)
    }

    fn get_db_conn(&self, case_name: &str) -> SqlResult<Connection> {
        let db_path = self.base_path.join(case_name).join("intelligence.db");
        let conn = Connection::open(db_path)?;

        // Inicializar esquema relacional
        conn.execute(
            "CREATE TABLE IF NOT EXISTS targets (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                type TEXT NOT NULL,
                category TEXT,
                metadata TEXT, -- JSON con atributos flexibles
                created_at TEXT NOT NULL
            )",
            [],
        )?;

        // Intentar añadir columnas si la tabla ya existía (migración liviana)
        let _ = conn.execute("ALTER TABLE targets ADD COLUMN category TEXT", []);
        let _ = conn.execute("ALTER TABLE targets ADD COLUMN metadata TEXT", []);

        conn.execute(
            "CREATE TABLE IF NOT EXISTS attributes (
                target_id TEXT,
                key TEXT,
                value TEXT,
                category TEXT, -- 'Technical' o 'Personal'
                FOREIGN KEY(target_id) REFERENCES targets(id)
            )",
            [],
        )?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS links (
                source_id TEXT,
                target_id TEXT,
                relation TEXT,
                FOREIGN KEY(source_id) REFERENCES targets(id),
                FOREIGN KEY(target_id) REFERENCES targets(id)
            )",
            [],
        )?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS activity_log (
                id TEXT PRIMARY KEY,
                timestamp TEXT NOT NULL,
                level TEXT NOT NULL, -- 'INFO', 'WARN', 'SUCCESS'
                message TEXT NOT NULL,
                tool_name TEXT
            )",
            [],
        )?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS objectives (
                id TEXT PRIMARY KEY,
                description TEXT NOT NULL,
                status TEXT NOT NULL, -- 'Pending', 'Running', 'Completed', 'Failed'
                priority INTEGER NOT NULL DEFAULT 0,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            )",
            [],
        )?;

        // Nuevas tablas para el modelo rico de personas
        conn.execute(
            "CREATE TABLE IF NOT EXISTS persons (
                id TEXT PRIMARY KEY,
                first_name TEXT, -- Ya no es NOT NULL
                last_name TEXT,
                dni TEXT,
                birth_date TEXT,
                phone TEXT,
                email TEXT,
                created_at TEXT NOT NULL
            )",
            [],
        )?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS person_nicknames (
                id TEXT PRIMARY KEY,
                person_id TEXT NOT NULL,
                value TEXT NOT NULL,
                FOREIGN KEY(person_id) REFERENCES persons(id) ON DELETE CASCADE
            )",
            [],
        )?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS addresses (
                id TEXT PRIMARY KEY,
                person_id TEXT NOT NULL,
                street TEXT,
                number TEXT,
                locality TEXT,
                state TEXT,
                country TEXT,
                zip_code TEXT,
                FOREIGN KEY(person_id) REFERENCES persons(id) ON DELETE CASCADE
            )",
            [],
        )?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS jobs (
                id TEXT PRIMARY KEY,
                person_id TEXT NOT NULL,
                title TEXT,
                company TEXT,
                date_start TEXT,
                date_end TEXT,
                FOREIGN KEY(person_id) REFERENCES persons(id) ON DELETE CASCADE
            )",
            [],
        )?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS social_profiles (
                id TEXT PRIMARY KEY,
                person_id TEXT NOT NULL,
                platform TEXT,
                username TEXT,
                url TEXT,
                FOREIGN KEY(person_id) REFERENCES persons(id) ON DELETE CASCADE
            )",
            [],
        )?;

        Ok(conn)
    }

    pub fn create_case(&self, name: &str, description: &str) -> Result<CaseMetadata, String> {
        let case_dir = self.base_path.join(name);
        if case_dir.exists() {
            return Err("Ya existe una investigación con ese nombre.".to_string());
        }

        fs::create_dir_all(&case_dir).map_err(|e| {
            eprintln!("ERROR [cases]: Failed to create case dir: {}", e);
            "No se pudo crear el directorio de la investigación.".to_string()
        })?;
        fs::create_dir_all(case_dir.join("exports")).map_err(|e| {
            eprintln!("ERROR [cases]: Failed to create exports dir: {}", e);
            "No se pudieron crear las carpetas de exportación.".to_string()
        })?;

        let metadata = CaseMetadata {
            name: name.to_string(),
            description: description.to_string(),
            created_at: Utc::now(),
            last_modified: Utc::now(),
            targets: Vec::new(),
        };

        // Guardar metadata básica en JSON (opcional para compatibilidad UI)
        self.save_metadata(&metadata)?;

        // Inicializar DB
        let _ = self.get_db_conn(name).map_err(|e| {
            eprintln!("ERROR [cases]: DB initialization failure: {}", e);
            "No se pudo inicializar la base de datos de inteligencia.".to_string()
        })?;

        Ok(metadata)
    }

    pub fn save_metadata(&self, metadata: &CaseMetadata) -> Result<(), String> {
        let path = self.base_path.join(&metadata.name).join("case.json");
        let content = serde_json::to_string_pretty(metadata).map_err(|e| {
            eprintln!("ERROR [cases]: Metadata serialization failure: {}", e);
            "Error al procesar los datos de la investigación.".to_string()
        })?;
        fs::write(path, content).map_err(|e| {
            eprintln!("ERROR [cases]: Metadata write failure: {}", e);
            "No se pudo guardar la información de la investigación.".to_string()
        })
    }

    pub fn load_case(&self, name: &str) -> Result<CaseMetadata, String> {
        let path = self.base_path.join(name).join("case.json");
        if !path.exists() {
            return Err("No se encontró la investigación.".to_string());
        }
        let content = fs::read_to_string(path).map_err(|e| e.to_string())?;
        let mut metadata: CaseMetadata =
            serde_json::from_str(&content).map_err(|e| e.to_string())?;

        // Cargar targets desde SQLite
        if let Ok(targets) = self.get_targets(name) {
            metadata.targets = targets;
        }

        Ok(metadata)
    }

    pub fn upsert_target_with_cat(
        &self,
        case_name: &str,
        target: Target,
        category: &str,
    ) -> Result<(), String> {
        // VALIDACIÓN DE INTEGRIDAD
        if target.name.trim().is_empty() || target.name.to_lowercase() == "sin nombre" {
            return Err("El objetivo debe tener un nombre válido para ser guardado.".to_string());
        }

        let conn = self.get_db_conn(case_name).map_err(|e| {
            eprintln!("ERROR [cases]: DB connection failure: {}", e);
            "No se pudo conectar con la base de datos de inteligencia.".to_string()
        })?;

        let t_type = format!("{:?}", target.target_type);
        let now = Utc::now().to_rfc3339();

        // Convertir el HashMap de data a JSON string
        let metadata_json =
            serde_json::to_string(&target.data).unwrap_or_else(|_| "{}".to_string());

        conn.execute(
            "INSERT INTO targets (id, name, type, category, metadata, created_at) 
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)
             ON CONFLICT(id) DO UPDATE SET name=?2, type=?3, category=?4, metadata=?5",
            params![target.id, target.name, t_type, category, metadata_json, now],
        )
        .map_err(|e| {
            eprintln!("ERROR [cases]: Upsert target failure: {}", e);
            "Error al guardar el objetivo en la base de datos.".to_string()
        })?;

        Ok(())
    }

    pub fn delete_target(&self, case_name: &str, target_id: &str) -> Result<(), String> {
        let conn = self.get_db_conn(case_name).map_err(|e| e.to_string())?;

        // 1. Borrar vínculos donde sea el origen o el destino
        conn.execute(
            "DELETE FROM links WHERE source_id = ?1 OR target_id = ?1",
            params![target_id],
        )
        .map_err(|e| format!("Error eliminando vínculos del objetivo: {}", e))?;

        // 2. Borrar atributos
        conn.execute(
            "DELETE FROM attributes WHERE target_id = ?1",
            params![target_id],
        )
        .map_err(|e| format!("Error eliminando atributos del objetivo: {}", e))?;

        // 3. Borrar el objetivo propiamente dicho
        conn.execute("DELETE FROM targets WHERE id = ?1", params![target_id])
            .map_err(|e| format!("Error eliminando el objetivo: {}", e))?;

        Ok(())
    }

    pub fn get_targets(&self, case_name: &str) -> Result<Vec<Target>, String> {
        let conn = self.get_db_conn(case_name).map_err(|e| e.to_string())?;

        let mut stmt = conn
            .prepare("SELECT id, name, type, category, metadata, created_at FROM targets")
            .map_err(|e| e.to_string())?;

        let target_rows = stmt
            .query_map([], |row| {
                let id: String = row.get(0)?;
                let name: String = row.get(1)?;
                let type_str: String = row.get(2)?;
                let category: String = row.get(3).unwrap_or_else(|_| "Technical".to_string());
                let metadata_str: Option<String> = row.get(4).ok();
                let created_at: String = row.get(5)?;

                let target_type = match type_str.as_str() {
                    "Domain" => TargetType::Domain,
                    "IP" => TargetType::IP,
                    "Email" => TargetType::Email,
                    "Username" => TargetType::Username,
                    "Phone" => TargetType::Phone,
                    "File" => TargetType::File,
                    "Hash" => TargetType::Hash,
                    "Person" => TargetType::Person,
                    _ => TargetType::Other,
                };

                let mut data: HashMap<String, serde_json::Value> = HashMap::new();
                if let Some(m_str) = metadata_str {
                    // Función recursiva para desempaquetar CUALQUIER nivel de serialización JSON en strings
                    fn deep_rescue(v: serde_json::Value) -> serde_json::Value {
                        match v {
                            serde_json::Value::String(s) => {
                                let trimmed = s.trim();
                                if (trimmed.starts_with('{') && trimmed.ends_with('}'))
                                    || (trimmed.starts_with('[') && trimmed.ends_with(']'))
                                {
                                    if let Ok(parsed) =
                                        serde_json::from_str::<serde_json::Value>(trimmed)
                                    {
                                        deep_rescue(parsed)
                                    } else {
                                        serde_json::Value::String(s)
                                    }
                                } else {
                                    serde_json::Value::String(s)
                                }
                            }
                            serde_json::Value::Object(map) => {
                                let mut new_map = serde_json::Map::new();
                                for (k, val) in map {
                                    new_map.insert(k, deep_rescue(val));
                                }
                                serde_json::Value::Object(new_map)
                            }
                            serde_json::Value::Array(arr) => {
                                serde_json::Value::Array(arr.into_iter().map(deep_rescue).collect())
                            }
                            _ => v,
                        }
                    }

                    if let Ok(mut parsed) =
                        serde_json::from_str::<HashMap<String, serde_json::Value>>(&m_str)
                    {
                        // 1. Limpiar y rescatar todo el mapa
                        let mut rescued_map = HashMap::new();
                        for (k, v) in parsed.drain() {
                            rescued_map.insert(k, deep_rescue(v));
                        }

                        // 2. Consolidar herramientas técnicas en 'detalles_tecnicos'
                        let mut tech_map = rescued_map
                            .remove("detalles_tecnicos")
                            .and_then(|v| v.as_object().cloned())
                            .unwrap_or_default();

                        let tools = ["whois", "ping", "dns", "nmap", "traceroute", "curl", "http"];
                        for tool in tools {
                            if let Some(val) = rescued_map.remove(tool) {
                                // Si el dato ya venía estructurado como { detalles_tecnicos: { tool: ... } } por error previo
                                let inner = val
                                    .get("detalles_tecnicos")
                                    .and_then(|dt| dt.get(tool))
                                    .unwrap_or(&val)
                                    .clone();
                                tech_map.insert(tool.to_string(), inner);
                            }
                        }

                        rescued_map.insert(
                            "detalles_tecnicos".to_string(),
                            serde_json::Value::Object(tech_map),
                        );
                        data = rescued_map;
                    }
                }

                Ok(Target {
                    id,
                    name,
                    target_type,
                    category,
                    data,
                    linked_targets: vec![],
                    created_at: DateTime::parse_from_rfc3339(&created_at)
                        .unwrap_or_else(|_| Utc::now().into())
                        .with_timezone(&Utc),
                })
            })
            .map_err(|e| e.to_string())?;

        let mut targets = Vec::new();
        for t_res in target_rows {
            let mut target = t_res.map_err(|e| e.to_string())?;

            // Cargar links para este target (donde target es source)
            let mut link_stmt = conn
                .prepare("SELECT target_id, relation FROM links WHERE source_id = ?1")
                .map_err(|e| e.to_string())?;
            let link_rows = link_stmt
                .query_map(params![target.id], |row| {
                    Ok(TargetLink {
                        target_id: row.get(0)?,
                        relation: row.get(1)?,
                    })
                })
                .map_err(|e| e.to_string())?;

            for link in link_rows {
                if let Ok(l) = link {
                    target.linked_targets.push(l);
                }
            }

            targets.push(target);
        }

        Ok(targets)
    }

    #[allow(dead_code)]
    pub fn add_link(
        &self,
        case_name: &str,
        source_id: &str,
        target_id: &str,
        relation: &str,
    ) -> Result<(), String> {
        let conn = self.get_db_conn(case_name).map_err(|e| e.to_string())?;
        conn.execute(
            "INSERT INTO links (source_id, target_id, relation) VALUES (?1, ?2, ?3)",
            params![source_id, target_id, relation],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn list_cases(&self) -> Result<Vec<String>, String> {
        let mut cases = Vec::new();
        if let Ok(entries) = fs::read_dir(&self.base_path) {
            for entry in entries.flatten() {
                if entry.path().is_dir() {
                    if let Some(name) = entry.file_name().to_str() {
                        cases.push(name.to_string());
                    }
                }
            }
        }
        Ok(cases)
    }

    pub fn save_history(&self, case_name: &str, history_json: &str) -> Result<(), String> {
        let path = self.base_path.join(case_name).join("history.json");
        fs::write(path, history_json).map_err(|e| {
            eprintln!("ERROR [cases]: History write failure: {}", e);
            "No se pudo guardar el historial de la investigación.".to_string()
        })
    }

    pub fn load_history(&self, case_name: &str) -> Result<String, String> {
        let path = self.base_path.join(case_name).join("history.json");
        if !path.exists() {
            return Ok("[]".to_string());
        }
        fs::read_to_string(path).map_err(|e| e.to_string())
    }

    pub fn delete_case(&self, case_name: &str) -> Result<(), String> {
        let case_dir = self.base_path.join(case_name);
        if !case_dir.exists() {
            return Err("La investigación no existe.".to_string());
        }
        fs::remove_dir_all(case_dir).map_err(|e| {
            eprintln!("ERROR [cases]: Failed to delete case dir: {}", e);
            "No se pudo eliminar la investigación.".to_string()
        })
    }

    // --- PERSONS CRUD ---

    pub fn create_person(&self, case_name: &str, person: Person) -> Result<Person, String> {
        let conn = self.get_db_conn(case_name).map_err(|e| e.to_string())?;

        conn.execute(
            "INSERT INTO persons (id, first_name, last_name, dni, birth_date, phone, email, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                person.id,
                person.first_name, // Option<String>
                person.last_name,
                person.dni,
                person.birth_date,
                person.phone,
                person.email,
                person.created_at
            ],
        ).map_err(|e| format!("Error creating person: {}", e))?;

        // Insertar apodos
        for nick in &person.nicknames {
            let nick_id = nick
                .id
                .clone()
                .unwrap_or_else(|| Uuid::new_v4().to_string());
            conn.execute(
                "INSERT INTO person_nicknames (id, person_id, value) VALUES (?1, ?2, ?3)",
                params![nick_id, person.id, nick.value],
            )
            .map_err(|e| format!("Error adding nickname: {}", e))?;
        }

        // Insertar direcciones
        for addr in &person.addresses {
            // Generar ID si no tiene (aunque el frontend debería mandarlo o lo generamos acá)
            let addr_id = addr
                .id
                .clone()
                .unwrap_or_else(|| Uuid::new_v4().to_string());
            conn.execute(
                "INSERT INTO addresses (id, person_id, street, number, locality, state, country, zip_code)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![addr_id, person.id, addr.street, addr.number, addr.locality, addr.state, addr.country, addr.zip_code]
             ).map_err(|e| format!("Error adding address: {}", e))?;
        }

        // Insertar trabajos
        for job in &person.jobs {
            let job_id = job.id.clone().unwrap_or_else(|| Uuid::new_v4().to_string());
            conn.execute(
                "INSERT INTO jobs (id, person_id, title, company, date_start, date_end)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    job_id,
                    person.id,
                    job.title,
                    job.company,
                    job.date_start,
                    job.date_end
                ],
            )
            .map_err(|e| format!("Error adding job: {}", e))?;
        }

        // Insertar redes sociales
        for social in &person.social_profiles {
            let social_id = social
                .id
                .clone()
                .unwrap_or_else(|| Uuid::new_v4().to_string());
            conn.execute(
                "INSERT INTO social_profiles (id, person_id, platform, username, url)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                params![
                    social_id,
                    person.id,
                    social.platform,
                    social.username,
                    social.url
                ],
            )
            .map_err(|e| format!("Error adding social profile: {}", e))?;
        }

        Ok(person)
    }

    pub fn get_persons(&self, case_name: &str) -> Result<Vec<Person>, String> {
        let conn = self.get_db_conn(case_name).map_err(|e| e.to_string())?;

        let mut stmt = conn.prepare("SELECT id, first_name, last_name, dni, birth_date, phone, email, created_at FROM persons").map_err(|e| e.to_string())?;

        let person_rows = stmt
            .query_map([], |row| {
                Ok(Person {
                    id: row.get(0)?,
                    first_name: row.get(1)?,
                    last_name: row.get(2)?,
                    dni: row.get(3)?,
                    birth_date: row.get(4)?,
                    phone: row.get(5)?,
                    email: row.get(6)?,
                    nicknames: Vec::new(),
                    addresses: Vec::new(),
                    jobs: Vec::new(),
                    social_profiles: Vec::new(),
                    created_at: row.get(7)?,
                })
            })
            .map_err(|e| e.to_string())?;

        let mut persons = Vec::new();
        for p_res in person_rows {
            let mut person = p_res.map_err(|e| e.to_string())?;

            // Cargar Nicknames
            let mut nick_stmt = conn
                .prepare("SELECT id, value FROM person_nicknames WHERE person_id = ?1")
                .map_err(|e| e.to_string())?;
            let nick_rows = nick_stmt
                .query_map(params![person.id], |row| {
                    Ok(Nickname {
                        id: row.get(0)?,
                        value: row.get(1)?,
                    })
                })
                .map_err(|e| e.to_string())?;

            for nick in nick_rows {
                if let Ok(n) = nick {
                    person.nicknames.push(n);
                }
            }

            // Cargar Addresses
            let mut addr_stmt = conn.prepare("SELECT id, street, number, locality, state, country, zip_code FROM addresses WHERE person_id = ?1").map_err(|e| e.to_string())?;
            let addr_rows = addr_stmt
                .query_map(params![person.id], |row| {
                    Ok(Address {
                        id: row.get(0)?,
                        street: row.get(1)?,
                        number: row.get(2)?,
                        locality: row.get(3)?,
                        state: row.get(4)?,
                        country: row.get(5)?,
                        zip_code: row.get(6)?,
                    })
                })
                .map_err(|e| e.to_string())?;

            for addr in addr_rows {
                if let Ok(a) = addr {
                    person.addresses.push(a);
                }
            }

            // Cargar Jobs
            let mut job_stmt = conn.prepare("SELECT id, title, company, date_start, date_end FROM jobs WHERE person_id = ?1").map_err(|e| e.to_string())?;
            let job_rows = job_stmt
                .query_map(params![person.id], |row| {
                    Ok(Job {
                        id: row.get(0)?,
                        title: row.get(1)?,
                        company: row.get(2)?,
                        date_start: row.get(3)?,
                        date_end: row.get(4)?,
                    })
                })
                .map_err(|e| e.to_string())?;

            for job in job_rows {
                if let Ok(j) = job {
                    person.jobs.push(j);
                }
            }

            // Cargar Socials
            let mut soc_stmt = conn
                .prepare(
                    "SELECT id, platform, username, url FROM social_profiles WHERE person_id = ?1",
                )
                .map_err(|e| e.to_string())?;
            let soc_rows = soc_stmt
                .query_map(params![person.id], |row| {
                    Ok(SocialProfile {
                        id: row.get(0)?,
                        platform: row.get(1)?,
                        username: row.get(2)?,
                        url: row.get(3)?,
                    })
                })
                .map_err(|e| e.to_string())?;

            for soc in soc_rows {
                if let Ok(s) = soc {
                    person.social_profiles.push(s);
                }
            }

            persons.push(person);
        }

        Ok(persons)
    }

    pub fn delete_person(&self, case_name: &str, person_id: &str) -> Result<(), String> {
        let conn = self.get_db_conn(case_name).map_err(|e| e.to_string())?;
        // Gracias a ON DELETE CASCADE, esto borrará direcciones, trabajos y redes sociales.
        conn.execute("DELETE FROM persons WHERE id = ?1", params![person_id])
            .map_err(|e| format!("Error deleting person: {}", e))?;
        Ok(())
    }

    pub fn update_person_basic(&self, case_name: &str, person: Person) -> Result<(), String> {
        let conn = self.get_db_conn(case_name).map_err(|e| e.to_string())?;
        conn.execute(
            "UPDATE persons SET first_name=?1, last_name=?2, dni=?3, birth_date=?4, phone=?5, email=?6 WHERE id=?7",
            params![
                person.first_name,
                person.last_name,
                person.dni,
                person.birth_date,
                person.phone,
                person.email,
                person.id
            ],
        ).map_err(|e| format!("Error updating person basic info: {}", e))?;
        Ok(())
    }

    pub fn add_address(
        &self,
        case_name: &str,
        person_id: &str,
        address: Address,
    ) -> Result<Address, String> {
        let conn = self.get_db_conn(case_name).map_err(|e| e.to_string())?;
        let addr_id = address
            .id
            .clone()
            .unwrap_or_else(|| Uuid::new_v4().to_string());

        conn.execute(
            "INSERT INTO addresses (id, person_id, street, number, locality, state, country, zip_code)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![addr_id, person_id, address.street, address.number, address.locality, address.state, address.country, address.zip_code]
        ).map_err(|e| format!("Error adding address: {}", e))?;

        let mut saved_addr = address;
        saved_addr.id = Some(addr_id);
        Ok(saved_addr)
    }

    pub fn remove_address(&self, case_name: &str, address_id: &str) -> Result<(), String> {
        let conn = self.get_db_conn(case_name).map_err(|e| e.to_string())?;
        conn.execute("DELETE FROM addresses WHERE id = ?1", params![address_id])
            .map_err(|e| format!("Error removing address: {}", e))?;
        Ok(())
    }

    // Funciones similares para Jobs y Socials... (omito por brevedad, el usuario pidió "CRUD" y esto es el andamiaje)
    // Implemento Job y Social para tener completo el soporte básico.

    pub fn add_job(&self, case_name: &str, person_id: &str, job: Job) -> Result<Job, String> {
        let conn = self.get_db_conn(case_name).map_err(|e| e.to_string())?;
        let job_id = job.id.clone().unwrap_or_else(|| Uuid::new_v4().to_string());

        conn.execute(
            "INSERT INTO jobs (id, person_id, title, company, date_start, date_end)
              VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                job_id,
                person_id,
                job.title,
                job.company,
                job.date_start,
                job.date_end
            ],
        )
        .map_err(|e| format!("Error adding job: {}", e))?;

        let mut saved_job = job;
        saved_job.id = Some(job_id);
        Ok(saved_job)
    }

    pub fn remove_job(&self, case_name: &str, job_id: &str) -> Result<(), String> {
        let conn = self.get_db_conn(case_name).map_err(|e| e.to_string())?;
        conn.execute("DELETE FROM jobs WHERE id = ?1", params![job_id])
            .map_err(|e| format!("Error removing job: {}", e))?;
        Ok(())
    }

    pub fn add_social(
        &self,
        case_name: &str,
        person_id: &str,
        social: SocialProfile,
    ) -> Result<SocialProfile, String> {
        let conn = self.get_db_conn(case_name).map_err(|e| e.to_string())?;
        let soc_id = social
            .id
            .clone()
            .unwrap_or_else(|| Uuid::new_v4().to_string());

        conn.execute(
            "INSERT INTO social_profiles (id, person_id, platform, username, url)
              VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                soc_id,
                person_id,
                social.platform,
                social.username,
                social.url
            ],
        )
        .map_err(|e| format!("Error adding social: {}", e))?;

        let mut saved_soc = social;
        saved_soc.id = Some(soc_id);
        Ok(saved_soc)
    }

    pub fn remove_social(&self, case_name: &str, social_id: &str) -> Result<(), String> {
        let conn = self.get_db_conn(case_name).map_err(|e| e.to_string())?;
        conn.execute(
            "DELETE FROM social_profiles WHERE id = ?1",
            params![social_id],
        )
        .map_err(|e| format!("Error removing social: {}", e))?;
        Ok(())
    }

    pub fn add_nickname(
        &self,
        case_name: &str,
        person_id: &str,
        nickname: Nickname,
    ) -> Result<Nickname, String> {
        let conn = self.get_db_conn(case_name).map_err(|e| e.to_string())?;
        let nid = nickname
            .id
            .clone()
            .unwrap_or_else(|| Uuid::new_v4().to_string());

        conn.execute(
            "INSERT INTO person_nicknames (id, person_id, value) VALUES (?1, ?2, ?3)",
            params![nid, person_id, nickname.value],
        )
        .map_err(|e| format!("Error adding nickname: {}", e))?;

        let mut saved = nickname;
        saved.id = Some(nid);
        Ok(saved)
    }

    pub fn remove_nickname(&self, case_name: &str, nickname_id: &str) -> Result<(), String> {
        let conn = self.get_db_conn(case_name).map_err(|e| e.to_string())?;
        conn.execute(
            "DELETE FROM person_nicknames WHERE id = ?1",
            params![nickname_id],
        )
        .map_err(|e| format!("Error removing nickname: {}", e))?;
        Ok(())
    }

    pub fn log_event(
        &self,
        case_name: &str,
        level: &str,
        message: &str,
        tool: Option<&str>,
    ) -> Result<(), String> {
        let conn = self.get_db_conn(case_name).map_err(|e| e.to_string())?;
        let now = Utc::now().to_rfc3339();
        let id = uuid::Uuid::new_v4().to_string();

        conn.execute(
            "INSERT INTO activity_log (id, timestamp, level, message, tool_name) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![id, now, level, message, tool],
        ).map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn get_activity_log(&self, case_name: &str) -> Result<Vec<LogEntry>, String> {
        let conn = self.get_db_conn(case_name).map_err(|e| e.to_string())?;
        let mut stmt = conn
            .prepare("SELECT id, timestamp, level, message, tool_name FROM activity_log ORDER BY timestamp DESC LIMIT 50")
            .map_err(|e| e.to_string())?;

        let rows = stmt
            .query_map([], |row| {
                Ok(LogEntry {
                    id: row.get(0)?,
                    timestamp: row.get(1)?,
                    level: row.get(2)?,
                    message: row.get(3)?,
                    tool_name: row.get(4)?,
                })
            })
            .map_err(|e| e.to_string())?;

        let mut logs = Vec::new();
        for r in rows {
            if let Ok(l) = r {
                logs.push(l);
            }
        }
        Ok(logs)
    }

    // --- OBJECTIVES CRUD ---

    pub fn create_objective(
        &self,
        case_name: &str,
        description: &str,
        priority: i32,
    ) -> Result<Objective, String> {
        let conn = self.get_db_conn(case_name).map_err(|e| e.to_string())?;
        let now = Utc::now();
        let objective = Objective {
            id: uuid::Uuid::new_v4().to_string(),
            description: description.to_string(),
            status: ObjectiveStatus::Pending,
            priority,
            created_at: now,
            updated_at: now,
        };

        conn.execute(
            "INSERT INTO objectives (id, description, status, priority, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                objective.id,
                objective.description,
                format!("{:?}", objective.status),
                objective.priority,
                objective.created_at.to_rfc3339(),
                objective.updated_at.to_rfc3339()
            ],
        )
        .map_err(|e| format!("Error creando objetivo: {}", e))?;

        Ok(objective)
    }

    pub fn get_objectives(&self, case_name: &str) -> Result<Vec<Objective>, String> {
        let conn = self.get_db_conn(case_name).map_err(|e| e.to_string())?;
        let mut stmt = conn
            .prepare("SELECT id, description, status, priority, created_at, updated_at FROM objectives ORDER BY priority DESC, created_at ASC")
            .map_err(|e| e.to_string())?;

        let rows = stmt
            .query_map([], |row| {
                let status_str: String = row.get(2)?;
                let status = match status_str.as_str() {
                    "Running" => ObjectiveStatus::Running,
                    "Completed" => ObjectiveStatus::Completed,
                    "Failed" => ObjectiveStatus::Failed,
                    _ => ObjectiveStatus::Pending,
                };

                Ok(Objective {
                    id: row.get(0)?,
                    description: row.get(1)?,
                    status,
                    priority: row.get(3)?,
                    created_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(4)?)
                        .unwrap_or_else(|_| Utc::now().into())
                        .with_timezone(&Utc),
                    updated_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(5)?)
                        .unwrap_or_else(|_| Utc::now().into())
                        .with_timezone(&Utc),
                })
            })
            .map_err(|e| e.to_string())?;

        let mut objectives = Vec::new();
        for r in rows {
            if let Ok(o) = r {
                objectives.push(o);
            }
        }
        Ok(objectives)
    }

    pub fn update_objective_status(
        &self,
        case_name: &str,
        id: &str,
        status: ObjectiveStatus,
    ) -> Result<(), String> {
        let conn = self.get_db_conn(case_name).map_err(|e| e.to_string())?;
        let now = Utc::now().to_rfc3339();
        conn.execute(
            "UPDATE objectives SET status = ?1, updated_at = ?2 WHERE id = ?3",
            params![format!("{:?}", status), now, id],
        )
        .map_err(|e| format!("Error actualizando estado del objetivo: {}", e))?;
        Ok(())
    }

    pub fn delete_objective(&self, case_name: &str, id: &str) -> Result<(), String> {
        let conn = self.get_db_conn(case_name).map_err(|e| e.to_string())?;
        conn.execute("DELETE FROM objectives WHERE id = ?1", params![id])
            .map_err(|e| format!("Error eliminando objetivo: {}", e))?;
        Ok(())
    }
}
