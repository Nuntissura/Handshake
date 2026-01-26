use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::ai_ready_data::chunking::estimate_tokens;

pub const DEFAULT_EMBEDDING_MODEL_ID: &str = "deterministic_hash";
pub const DEFAULT_EMBEDDING_MODEL_VERSION: &str = "v1";
pub const DEFAULT_EMBEDDING_DIMENSIONS: u32 = 512;
pub const DEFAULT_EMBEDDING_MAX_INPUT_TOKENS: u32 = 2048;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingArtifact {
    pub schema_version: String,
    pub model_id: String,
    pub model_version: String,
    pub dimensions: u32,
    pub vector: Vec<f32>,
}

fn deterministic_seed(text: &str, model_id: &str, model_version: &str) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(model_id.as_bytes());
    hasher.update(b"\n");
    hasher.update(model_version.as_bytes());
    hasher.update(b"\n");
    hasher.update(text.as_bytes());
    hasher.finalize().into()
}

pub fn truncate_for_embedding<'a>(text: &'a str, max_input_tokens: u32) -> (&'a str, bool) {
    let estimated_tokens = estimate_tokens(text);
    if estimated_tokens <= max_input_tokens {
        return (text, false);
    }

    let max_bytes = (max_input_tokens as usize).saturating_mul(4).max(1);
    if text.len() <= max_bytes {
        return (text, false);
    }

    let mut cut = max_bytes.min(text.len());
    while cut > 0 && !text.is_char_boundary(cut) {
        cut = cut.saturating_sub(1);
    }
    (&text[..cut], true)
}

pub fn embed_text_deterministic(
    text: &str,
    model_id: &str,
    model_version: &str,
    dimensions: u32,
    max_input_tokens: u32,
) -> (Vec<f32>, bool) {
    let (input, was_truncated) = truncate_for_embedding(text, max_input_tokens);
    let seed = deterministic_seed(input, model_id, model_version);

    let mut vector: Vec<f32> = Vec::with_capacity(dimensions as usize);
    let mut counter: u32 = 0;

    while vector.len() < dimensions as usize {
        let mut hasher = Sha256::new();
        hasher.update(seed);
        hasher.update(counter.to_le_bytes());
        let digest = hasher.finalize();

        for bytes in digest.chunks(4) {
            if vector.len() >= dimensions as usize {
                break;
            }
            let mut fixed = [0u8; 4];
            fixed.copy_from_slice(bytes);
            let raw = u32::from_le_bytes(fixed);
            let unit = (raw as f64) / (u32::MAX as f64);
            let mapped = (unit * 2.0) - 1.0;
            vector.push(mapped as f32);
        }

        counter = counter.wrapping_add(1);
    }

    let norm = vector
        .iter()
        .map(|value| (*value as f64) * (*value as f64))
        .sum::<f64>()
        .sqrt();
    if norm > 0.0 {
        for value in &mut vector {
            *value = (*value as f64 / norm) as f32;
        }
    }

    (vector, was_truncated)
}
