use chrono::Utc;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fs;
use std::path::Path;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MemoryItem {
    pub id: String,
    pub text: String,
    pub embedding: Vec<f32>,
    pub timestamp: i64,
}

pub struct SemanticMemoryManager {
    client: Client,
}

impl SemanticMemoryManager {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
        }
    }

    /// Llama a Ollama para oabtener el vector [f32]
    pub async fn get_embedding(
        &self,
        text: &str,
        model: &str,
        ollama_url: &str,
    ) -> Result<Vec<f32>, String> {
        let endpoint = format!("{}/api/embeddings", ollama_url);

        let payload = serde_json::json!({
            "model": model,
            "prompt": text
        });

        let res = self
            .client
            .post(&endpoint)
            .json(&payload)
            .send()
            .await
            .map_err(|e| format!("Error conectando a Ollama Embeddings: {}", e))?;

        if !res.status().is_success() {
            let error_text = res.text().await.unwrap_or_default();
            return Err(format!("Ollama devolvió un error: {}", error_text));
        }

        let json_resp: Value = res.json().await.map_err(|e| e.to_string())?;

        if let Some(embedding) = json_resp.get("embedding").and_then(|e| e.as_array()) {
            let vec: Vec<f32> = embedding
                .iter()
                .filter_map(|v| v.as_f64().map(|f| f as f32))
                .collect();
            Ok(vec)
        } else {
            Err("Ollama no incluyó 'embedding' en la respuesta JSON.".to_string())
        }
    }

    /// Guarda un fragmento de memoria semántica en el caso
    pub async fn add_memory(
        &self,
        case_path: &Path,
        text: &str,
        model: &str,
        ollama_url: &str,
    ) -> Result<(), String> {
        let embedding = self.get_embedding(text, model, ollama_url).await?;

        let mut memories = self.load_memories(case_path)?;

        memories.push(MemoryItem {
            id: uuid::Uuid::new_v4().to_string(),
            text: text.to_string(),
            embedding,
            timestamp: Utc::now().timestamp(),
        });

        self.save_memories(case_path, &memories)
    }

    /// Busca texto relevante comparando contra la Vector DB local usando Cosine Similarity
    pub async fn search_memory(
        &self,
        case_path: &Path,
        query: &str,
        model: &str,
        ollama_url: &str,
        top_k: usize,
    ) -> Result<Vec<String>, String> {
        let memories = self.load_memories(case_path)?;
        if memories.is_empty() {
            return Ok(Vec::new());
        }

        let query_embedding = self.get_embedding(query, model, ollama_url).await?;

        let mut scored_memories: Vec<(f32, String)> = memories
            .into_iter()
            .map(|mem| {
                let score = cosine_similarity(&query_embedding, &mem.embedding);
                (score, mem.text)
            })
            .collect();

        // Ordenar descendente (mayor similitud primero)
        scored_memories.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

        // Tomar top K y desechar los de bajo threshold
        let threshold = 0.50; // Threshold arbitrario para relevancia

        let results = scored_memories
            .into_iter()
            .filter(|(score, _)| *score > threshold)
            .take(top_k)
            .map(|(_, text)| text)
            .collect();

        Ok(results)
    }

    fn load_memories(&self, case_path: &Path) -> Result<Vec<MemoryItem>, String> {
        let file_path = case_path.join("vector_memory.json");
        if !file_path.exists() {
            return Ok(Vec::new());
        }

        let content = fs::read_to_string(&file_path)
            .map_err(|e| format!("Error leyendo vector_memory.json: {}", e))?;
        let memories: Vec<MemoryItem> =
            serde_json::from_str(&content).unwrap_or_else(|_| Vec::new());
        Ok(memories)
    }

    fn save_memories(&self, case_path: &Path, memories: &[MemoryItem]) -> Result<(), String> {
        let file_path = case_path.join("vector_memory.json");
        let json = serde_json::to_string_pretty(memories)
            .map_err(|e| format!("Formato JSON inválido: {}", e))?;
        fs::write(&file_path, json)
            .map_err(|e| format!("Error escribiendo vector_memory.json: {}", e))
    }
}

/// Helper para calcular Distancia Coseno
fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    let mut dot_product = 0.0;
    let mut norm_a_sq = 0.0;
    let mut norm_b_sq = 0.0;

    for i in 0..a.len().min(b.len()) {
        dot_product += a[i] * b[i];
        norm_a_sq += a[i] * a[i];
        norm_b_sq += b[i] * b[i];
    }

    let denom = norm_a_sq.sqrt() * norm_b_sq.sqrt();
    if denom == 0.0 {
        return 0.0;
    }

    dot_product / denom
}
