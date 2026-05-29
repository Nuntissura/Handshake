use std::{collections::HashMap, path::Path, sync::Arc};

use crate::model_runtime::{ModelId, ModelRuntimeError};

pub const TOKENIZER_JSON_FILE: &str = "tokenizer.json";

#[cfg(any(feature = "tokenization", feature = "candle-runtime-engine"))]
pub type CandleTokenizerCache = HashMap<ModelId, Arc<tokenizers::Tokenizer>>;

#[cfg(not(any(feature = "tokenization", feature = "candle-runtime-engine")))]
pub type CandleTokenizerCache = HashMap<ModelId, Arc<CandleTokenizerPlaceholder>>;

#[cfg(not(any(feature = "tokenization", feature = "candle-runtime-engine")))]
#[derive(Debug)]
pub struct CandleTokenizerPlaceholder {
    pub path: std::path::PathBuf,
}

pub fn tokenizer_json_path_for_artifact(artifact_path: &Path) -> std::path::PathBuf {
    if artifact_path.is_dir() {
        return artifact_path.join(TOKENIZER_JSON_FILE);
    }

    artifact_path
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join(TOKENIZER_JSON_FILE)
}

#[cfg(any(feature = "tokenization", feature = "candle-runtime-engine"))]
pub fn cache_tokenizer_if_present(
    cache: &mut CandleTokenizerCache,
    id: ModelId,
    artifact_path: &Path,
) -> Result<(), ModelRuntimeError> {
    let tokenizer_path = tokenizer_json_path_for_artifact(artifact_path);
    if !tokenizer_path.is_file() {
        return Ok(());
    }

    let tokenizer = tokenizers::Tokenizer::from_file(&tokenizer_path).map_err(|error| {
        ModelRuntimeError::LoadError(format!(
            "failed to load Candle tokenizer {}: {error}",
            tokenizer_path.display()
        ))
    })?;
    cache.insert(id, Arc::new(tokenizer));
    Ok(())
}

#[cfg(not(any(feature = "tokenization", feature = "candle-runtime-engine")))]
pub fn cache_tokenizer_if_present(
    cache: &mut CandleTokenizerCache,
    id: ModelId,
    artifact_path: &Path,
) -> Result<(), ModelRuntimeError> {
    let tokenizer_path = tokenizer_json_path_for_artifact(artifact_path);
    if tokenizer_path.is_file() {
        cache.insert(
            id,
            Arc::new(CandleTokenizerPlaceholder {
                path: tokenizer_path,
            }),
        );
    }
    Ok(())
}
