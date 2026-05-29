use std::{
    collections::HashMap,
    path::Path,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc, Mutex,
    },
};

use crate::model_runtime::{ModelId, ModelRuntimeError};

use super::gguf_loader;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LlamaTokenizer {
    pub vocab_size: usize,
    pub bos_id: Option<u32>,
    pub eos_id: Option<u32>,
    pub special_tokens: Vec<String>,
}

#[derive(Debug, Default)]
pub struct TokenizerCache {
    entries: Mutex<HashMap<ModelId, Arc<LlamaTokenizer>>>,
    parse_count: AtomicU64,
}

impl TokenizerCache {
    pub fn get_or_parse(
        &self,
        model_id: ModelId,
        artifact_path: &Path,
    ) -> Result<Arc<LlamaTokenizer>, ModelRuntimeError> {
        let mut entries = self.entries.lock().map_err(|_| {
            ModelRuntimeError::LoadError("tokenizer cache lock poisoned".to_string())
        })?;

        if let Some(tokenizer) = entries.get(&model_id) {
            return Ok(Arc::clone(tokenizer));
        }

        let tokenizer = Arc::new(parse_gguf_tokenizer(artifact_path)?);
        entries.insert(model_id, Arc::clone(&tokenizer));
        self.parse_count.fetch_add(1, Ordering::SeqCst);
        Ok(tokenizer)
    }

    pub fn get(&self, model_id: ModelId) -> Option<Arc<LlamaTokenizer>> {
        self.entries
            .lock()
            .ok()
            .and_then(|entries| entries.get(&model_id).cloned())
    }

    pub fn parse_count(&self) -> u64 {
        self.parse_count.load(Ordering::SeqCst)
    }
}

pub fn parse_gguf_tokenizer(artifact_path: &Path) -> Result<LlamaTokenizer, ModelRuntimeError> {
    gguf_loader::parse_gguf_tokenizer_metadata(artifact_path)
}
