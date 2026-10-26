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

/// The supported embedded Qwen3 tokenizer. The captured template is provenance
/// only: GenerateRequest remains plain/preformatted text and does not render it.
#[cfg(feature = "candle-runtime-engine")]
#[derive(Debug)]
pub struct Qwen3GgufTokenizer {
    pub tokenizer: Arc<tokenizers::Tokenizer>,
    pub eos_token_ids: Vec<u32>,
    pub chat_template: String,
    pub vocab_size: u32,
}

#[cfg(feature = "candle-runtime-engine")]
pub fn tokenizer_from_qwen3_gguf_metadata(
    metadata: &HashMap<String, candle_core::quantized::gguf_file::Value>,
) -> Result<Qwen3GgufTokenizer, ModelRuntimeError> {
    use candle_core::quantized::gguf_file::Value;
    use tokenizers::{AddedToken, Tokenizer, models::bpe::{BPE, Vocab},
        pre_tokenizers::{byte_level::ByteLevel, sequence::Sequence, split::{Split, SplitPattern}},
        SplitDelimiterBehavior};
    let invalid = |detail: &str| ModelRuntimeError::LoadError(format!("unsupported or malformed Qwen3 GGUF tokenizer: {detail}"));
    let string = |key: &str| -> Result<&str, ModelRuntimeError> {
        match metadata.get(key) { Some(Value::String(v)) => Ok(v), _ => Err(invalid(key)) }
    };
    if string("general.architecture")? != "qwen3" || string("tokenizer.ggml.model")? != "gpt2"
        || string("tokenizer.ggml.pre")? != "qwen2" { return Err(invalid("architecture/model/pre")); }
    for key in ["tokenizer.ggml.add_bos_token", "tokenizer.ggml.add_eos_token"] {
        match metadata.get(key) { None | Some(Value::Bool(false)) => {}, _ => return Err(invalid(key)) }
    }
    let array = |key: &str| -> Result<&Vec<Value>, ModelRuntimeError> {
        match metadata.get(key) { Some(Value::Array(v)) if !v.is_empty() && v.len() <= 1_000_000 => Ok(v), _ => Err(invalid(key)) }
    };
    let tokens = array("tokenizer.ggml.tokens")?;
    let types = array("tokenizer.ggml.token_type")?;
    if tokens.len() != types.len() { return Err(invalid("token/type count differs")); }
    let mut vocab = Vocab::default();
    let mut added = Vec::new();
    for (index, (token, kind)) in tokens.iter().zip(types).enumerate() {
        let (Value::String(token), Value::I32(kind)) = (token, kind) else { return Err(invalid("token/type value")); };
        if token.is_empty() || token.len() > 65536 || !matches!(*kind, 1 | 3 | 4 | 5)
            || vocab.insert(token.clone(), index as u32).is_some() { return Err(invalid("empty/duplicate/unsupported token")); }
        if matches!(*kind, 3 | 4) { added.push(AddedToken::from(token.clone(), *kind == 3).normalized(false)); }
    }
    for symbol in ByteLevel::alphabet() {
        if !vocab.contains_key(&symbol.to_string()) { return Err(invalid("missing GPT2 byte alphabet token")); }
    }
    let mut merge_pairs = std::collections::HashSet::new();
    let mut merges = Vec::new();
    for value in array("tokenizer.ggml.merges")? {
        let Value::String(pair) = value else { return Err(invalid("merge value")); };
        let Some((left, right)) = pair.split_once(' ') else { return Err(invalid("merge pair")); };
        if left.is_empty() || right.is_empty() || right.contains(' ') { return Err(invalid("merge pair")); }
        if !merge_pairs.insert((left.to_owned(), right.to_owned())) { return Err(invalid("duplicate merge pair")); }
        merges.push((left.to_owned(), right.to_owned()));
    }
    let model = BPE::builder().vocab_and_merges(vocab, merges).byte_fallback(false).build()
        .map_err(|e| invalid(&format!("BPE: {e}")))?;
    let mut tokenizer = Tokenizer::new(model);
    // Qwen2 pretokenizer configuration; tokenization itself is HF tokenizers.
    let split = Split::new(SplitPattern::Regex(r"(?i:'s|'t|'re|'ve|'m|'ll|'d)|[^\r\n\p{L}\p{N}]?\p{L}+|\p{N}| ?[^\s\p{L}\p{N}]+[\r\n]*|\s*[\r\n]+|\s+(?!\S)|\s+".into()), SplitDelimiterBehavior::Isolated, false)
        .map_err(|e| invalid(&format!("pretokenizer: {e}")))?;
    tokenizer.with_pre_tokenizer(Some(Sequence::new(vec![split.into(), ByteLevel::new(false, false, false).into()])));
    tokenizer.with_decoder(Some(ByteLevel::new(false, false, false)));
    tokenizer.add_tokens(added)
        .map_err(|e| invalid(&format!("added tokens: {e}")))?;
    for (index, token) in tokens.iter().enumerate() {
        let Value::String(token) = token else { unreachable!() };
        if tokenizer.token_to_id(token) != Some(index as u32) { return Err(invalid("token ID changed")); }
    }
    let eos = metadata.get("tokenizer.ggml.eos_token_id").ok_or_else(|| invalid("missing EOS"))?
        .to_u32().map_err(|_| invalid("EOS type"))?;
    if eos as usize >= tokens.len() || !matches!(types[eos as usize], Value::I32(3)) { return Err(invalid("EOS ID/type")); }
    for key in ["tokenizer.ggml.bos_token_id", "tokenizer.ggml.padding_token_id"] {
        if let Some(value) = metadata.get(key) {
            let id = value.to_u32().map_err(|_| invalid(key))?;
            if id as usize >= tokens.len() { return Err(invalid(key)); }
        }
    }
    let chat_template = string("tokenizer.chat_template")?;
    if chat_template.is_empty() || chat_template.len() > 1024 * 1024 { return Err(invalid("chat template")); }
    Ok(Qwen3GgufTokenizer { tokenizer: Arc::new(tokenizer), eos_token_ids: vec![eos],
        chat_template: chat_template.to_owned(), vocab_size: tokens.len() as u32 })
}
