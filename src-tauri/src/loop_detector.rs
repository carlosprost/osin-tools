// src-tauri/src/loop_detector.rs
//
// Detector de bucles del agente IA — inspirado en OpenClaw tool-loop-detection.ts
// Usa SHA-256 hash de args + resultados para identificar repetición real vs. progreso.
//
// Detectores implementados:
//   1. generic_repeat     — misma tool+args N veces sin resultado diferente
//   2. ping_pong          — alternancia A→B→A→B sin progreso
//   3. circuit_breaker    — umbral global de 30 llamadas con mismo hash

use sha2::{Digest, Sha256};
use std::fmt::Write;

/// Tamaño del sliding window de historial
pub const HISTORY_SIZE: usize = 30;
/// Umbral de advertencia
pub const WARNING_THRESHOLD: usize = 5;
/// Umbral crítico
pub const CRITICAL_THRESHOLD: usize = 10;
/// Circuit breaker global
pub const GLOBAL_BREAKER_THRESHOLD: usize = 20;

/// Tipo de detector que disparó la alerta
#[derive(Debug, Clone, PartialEq)]
pub enum DetectorKind {
    GenericRepeat,
    PingPong,
    CircuitBreaker,
}

/// Nivel de severidad de la alerta
#[derive(Debug, Clone, PartialEq)]
pub enum LoopLevel {
    Warning,
    Critical,
}

/// Resultado de la evaluación del detector
#[derive(Debug, Clone)]
pub enum LoopResult {
    /// No se detectó bucle
    Ok,
    /// Se detectó un bucle
    Stuck {
        level: LoopLevel,
        #[allow(dead_code)]
        kind: DetectorKind,
        #[allow(dead_code)]
        count: usize,
        /// Mensaje para inyectar al agente como advertencia
        message: String,
    },
}

/// Entrada individual en el historial del detector
#[derive(Debug, Clone)]
struct ToolCallEntry {
    tool_name: String,
    /// Hash SHA-256 de (tool_name + args)
    args_hash: String,
    /// Hash SHA-256 del resultado (se setea después de ejecutar)
    result_hash: Option<String>,
}

/// Detector de bucles — mantiene un sliding window de las últimas N llamadas a herramientas
pub struct LoopDetector {
    history: Vec<ToolCallEntry>,
}

impl LoopDetector {
    /// Crea un nuevo detector vacío
    pub fn new() -> Self {
        LoopDetector {
            history: Vec::with_capacity(HISTORY_SIZE),
        }
    }

    /// Resetea el historial (cuando el usuario inicia una nueva consulta)
    #[allow(dead_code)]
    pub fn reset(&mut self) {
        self.history.clear();
    }

    /// Genera un hash SHA-256 de un string arbitrario
    fn sha256_hex(input: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(input.as_bytes());
        let result = hasher.finalize();
        result.iter().fold(String::new(), |mut acc, b| {
            let _ = write!(acc, "{:02x}", b);
            acc
        })
    }

    /// Genera el hash canónico de una llamada: tool_name + args serializado
    pub fn hash_call(tool_name: &str, args: &str) -> String {
        Self::sha256_hex(&format!("{}:{}", tool_name, args))
    }

    /// Genera el hash de un resultado
    pub fn hash_result(result: &str) -> String {
        Self::sha256_hex(result)
    }

    /// Registra una nueva llamada a herramienta ANTES de ejecutarla.
    /// Devuelve el resultado del análisis de bucle.
    pub fn record_call(&mut self, tool_name: &str, args: &str) -> LoopResult {
        let args_hash = Self::hash_call(tool_name, args);

        // Evaluar ANTES de agregar al historial
        let result = self.evaluate(tool_name, &args_hash);

        // Agregar al sliding window
        self.history.push(ToolCallEntry {
            tool_name: tool_name.to_string(),
            args_hash,
            result_hash: None,
        });

        // Mantener tamaño del window
        if self.history.len() > HISTORY_SIZE {
            self.history.remove(0);
        }

        result
    }

    /// Registra el resultado de la última llamada (para detectar no-progreso)
    pub fn record_result(&mut self, tool_name: &str, args: &str, result_data: &str) {
        let args_hash = Self::hash_call(tool_name, args);
        let result_hash = Self::hash_result(result_data);

        // Buscar la última entrada sin resultado para este tool+args
        for entry in self.history.iter_mut().rev() {
            if entry.tool_name == tool_name
                && entry.args_hash == args_hash
                && entry.result_hash.is_none()
            {
                entry.result_hash = Some(result_hash);
                break;
            }
        }
    }

    /// Lógica principal de evaluación — llama a los 3 detectores
    fn evaluate(&self, tool_name: &str, args_hash: &str) -> LoopResult {
        // 1. Circuit breaker global
        if let Some(result) = self.check_circuit_breaker(tool_name, args_hash) {
            return result;
        }

        // 2. Generic repeat + no-progress
        if let Some(result) = self.check_generic_repeat(tool_name, args_hash) {
            return result;
        }

        // 3. Ping-pong
        if let Some(result) = self.check_ping_pong(args_hash) {
            return result;
        }

        LoopResult::Ok
    }

    /// Detecta si este hash de args se repite con el mismo resultado (no-progreso)
    fn no_progress_streak(&self, tool_name: &str, args_hash: &str) -> usize {
        let mut streak = 0usize;
        let mut latest_result_hash: Option<&str> = None;

        for entry in self.history.iter().rev() {
            if entry.tool_name != tool_name || entry.args_hash != args_hash {
                continue;
            }
            match &entry.result_hash {
                None => continue,
                Some(rh) => {
                    if latest_result_hash.is_none() {
                        latest_result_hash = Some(rh.as_str());
                        streak = 1;
                    } else if latest_result_hash == Some(rh.as_str()) {
                        streak += 1;
                    } else {
                        // Resultado diferente → hay progreso
                        break;
                    }
                }
            }
        }
        streak
    }

    /// Detector 1: repetición genérica
    fn check_generic_repeat(&self, tool_name: &str, args_hash: &str) -> Option<LoopResult> {
        let count = self
            .history
            .iter()
            .filter(|e| e.tool_name == tool_name && e.args_hash == args_hash)
            .count();

        let no_progress = self.no_progress_streak(tool_name, args_hash);

        if no_progress >= CRITICAL_THRESHOLD {
            return Some(LoopResult::Stuck {
                level: LoopLevel::Critical,
                kind: DetectorKind::GenericRepeat,
                count: no_progress,
                message: format!(
                    "❌ CRÍTICO: Llamaste '{}' {} veces con los mismos argumentos y el resultado no cambió. \
                     Detení la ejecución y respondé al usuario con la info que ya tenés en el contexto.",
                    tool_name, no_progress
                ),
            });
        }

        if count >= WARNING_THRESHOLD {
            return Some(LoopResult::Stuck {
                level: LoopLevel::Warning,
                kind: DetectorKind::GenericRepeat,
                count,
                message: format!(
                    "⚠️ ADVERTENCIA: Llamaste '{}' {} veces con los mismos argumentos. \
                     Si no estás progresando, respondé directamente al usuario con lo que ya sabés.",
                    tool_name, count
                ),
            });
        }

        None
    }

    /// Detector 2: ping-pong (A→B→A→B sin avance)
    fn check_ping_pong(&self, current_hash: &str) -> Option<LoopResult> {
        if self.history.len() < 4 {
            return None;
        }

        // Buscar el otro hash (el que alterna con el actual)
        let last_entry = self.history.last()?;
        let other_hash = &last_entry.args_hash;

        if other_hash == current_hash {
            return None; // Misma llamada, no ping-pong
        }

        // Contar cuántas entradas finales alternan entre current_hash y other_hash
        let mut alternating = 0usize;
        let mut expected = current_hash; // el próximo esperado en la secuencia
        for entry in self.history.iter().rev() {
            // Verificamos si el hash de este entry coincide con el esperado
            if entry.args_hash == expected {
                alternating += 1;
                expected = if expected == current_hash {
                    other_hash.as_str()
                } else {
                    current_hash
                };
            } else {
                break;
            }
        }

        if alternating >= CRITICAL_THRESHOLD {
            return Some(LoopResult::Stuck {
                level: LoopLevel::Critical,
                kind: DetectorKind::PingPong,
                count: alternating,
                message: format!(
                    "❌ CRÍTICO: Estás alternando entre dos herramientas en bucle ({} veces) sin avanzar. \
                     Cortá el ciclo y respondé al usuario con la información disponible en el contexto.",
                    alternating
                ),
            });
        }

        if alternating >= WARNING_THRESHOLD {
            return Some(LoopResult::Stuck {
                level: LoopLevel::Warning,
                kind: DetectorKind::PingPong,
                count: alternating,
                message: format!(
                    "⚠️ ADVERTENCIA: Estás alternando entre dos herramientas ({} veces). \
                     Revisá si estás progresando; si no, respondé directamente.",
                    alternating
                ),
            });
        }

        None
    }

    /// Detector 3: circuit breaker global
    fn check_circuit_breaker(&self, tool_name: &str, args_hash: &str) -> Option<LoopResult> {
        let no_progress = self.no_progress_streak(tool_name, args_hash);

        if no_progress >= GLOBAL_BREAKER_THRESHOLD {
            return Some(LoopResult::Stuck {
                level: LoopLevel::Critical,
                kind: DetectorKind::CircuitBreaker,
                count: no_progress,
                message: format!(
                    "🚨 CIRCUIT BREAKER: '{}' se repitió {} veces con resultado idéntico. \
                     Sesión bloqueada para proteger el sistema. Respondé al usuario inmediatamente.",
                    tool_name, no_progress
                ),
            });
        }

        None
    }
}

impl Default for LoopDetector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sin_bucle_no_alerta() {
        let mut detector = LoopDetector::new();
        let result = detector.record_call("whois", "dominio-a.com");
        assert!(matches!(result, LoopResult::Ok));
    }

    #[test]
    fn test_detecta_repeticion_generica() {
        let mut detector = LoopDetector::new();
        // Simular WARNING_THRESHOLD llamadas iguales
        let mut result = LoopResult::Ok;
        for _ in 0..=WARNING_THRESHOLD {
            result = detector.record_call("whois", "dominio-repetido.com");
        }
        assert!(matches!(
            result,
            LoopResult::Stuck {
                kind: DetectorKind::GenericRepeat,
                ..
            }
        ));
    }
}
