use std::{
    collections::HashSet,
    fs::File,
    io::{self, BufReader, Read},
    path::Path,
};

use sha2::{Digest, Sha256};

use super::{super::*, context::LlamaCppContext, tokenizer_cache::LlamaTokenizer};

pub const NOT_GGUF_V2_V3_ARTIFACT: &str = "not a GGUF v2/v3 artifact";

const GGUF_MAGIC: &[u8; 4] = b"GGUF";
const GGUF_VERSION_2: u32 = 2;
const GGUF_VERSION_3: u32 = 3;
const GGUF_TYPE_UINT8: u32 = 0;
const GGUF_TYPE_INT8: u32 = 1;
const GGUF_TYPE_UINT16: u32 = 2;
const GGUF_TYPE_INT16: u32 = 3;
const GGUF_TYPE_UINT32: u32 = 4;
const GGUF_TYPE_INT32: u32 = 5;
const GGUF_TYPE_FLOAT32: u32 = 6;
const GGUF_TYPE_BOOL: u32 = 7;
const GGUF_TYPE_STRING: u32 = 8;
const GGUF_TYPE_ARRAY: u32 = 9;
const GGUF_TYPE_UINT64: u32 = 10;
const GGUF_TYPE_INT64: u32 = 11;
const GGUF_TYPE_FLOAT64: u32 = 12;
const MAX_GGUF_STRING_BYTES: u64 = 16 * 1024 * 1024;
const MAX_GGUF_ARRAY_VALUES: u64 = 10_000_000;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GpuLayerOffload {
    CpuOnly,
    LayerCount(u32),
    All,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LlamaCppModelLoadConfig {
    pub gpu_layers: GpuLayerOffload,
    pub main_gpu: i32,
    pub vocab_only: bool,
    pub use_mmap: bool,
    pub use_mlock: bool,
}

impl Default for LlamaCppModelLoadConfig {
    fn default() -> Self {
        Self {
            gpu_layers: GpuLayerOffload::CpuOnly,
            main_gpu: 0,
            vocab_only: false,
            use_mmap: true,
            use_mlock: false,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LlamaCppEmbeddingPooling {
    Unspecified,
    None,
    Mean,
    Cls,
    Last,
    Rank,
}

impl Default for LlamaCppEmbeddingPooling {
    fn default() -> Self {
        Self::Unspecified
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LlamaCppContextLoadConfig {
    pub n_ctx: u32,
    pub n_batch: u32,
    pub n_threads: u32,
    pub embeddings: bool,
    pub causal_attn: bool,
    pub n_seq_max: u32,
    pub embedding_pooling: LlamaCppEmbeddingPooling,
}

impl Default for LlamaCppContextLoadConfig {
    fn default() -> Self {
        Self {
            n_ctx: 8192,
            n_batch: 512,
            n_threads: default_n_threads(),
            embeddings: true,
            causal_attn: true,
            n_seq_max: 1,
            embedding_pooling: LlamaCppEmbeddingPooling::default(),
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct LlamaCppLoadConfig {
    pub model: LlamaCppModelLoadConfig,
    pub context: LlamaCppContextLoadConfig,
}

pub fn load_gguf_context(
    spec: &LoadSpec,
    config: &LlamaCppLoadConfig,
) -> Result<LlamaCppContext, ModelRuntimeError> {
    validate_llama_cpp_load_spec(spec)?;
    LlamaCppContext::load_from_file(&spec.artifact_path, config)
}

pub fn validate_llama_cpp_load_spec(spec: &LoadSpec) -> Result<(), ModelRuntimeError> {
    if spec.runtime_kind != RuntimeKind::LlamaCpp {
        return Err(ModelRuntimeError::LoadError(format!(
            "LlamaCppRuntime requires RuntimeKind::LlamaCpp, got {:?}",
            spec.runtime_kind
        )));
    }

    if spec.provider != ProviderKind::Local {
        return Err(ModelRuntimeError::LoadError(format!(
            "LlamaCppRuntime accepts only local provider specs, got {:?}",
            spec.provider
        )));
    }

    if spec.declared_capabilities.supports_activation_steering {
        return Err(ModelRuntimeError::LoadError(
            "LlamaCppRuntime cannot declare activation_steering support".to_string(),
        ));
    }

    validate_gguf_magic(&spec.artifact_path)?;

    let actual = sha256_file(&spec.artifact_path)?;
    if !actual.eq_ignore_ascii_case(spec.sha256_expected.trim()) {
        return Err(ModelRuntimeError::LoadError(format!(
            "llama.cpp artifact sha256 mismatch: expected {}, got {actual}",
            spec.sha256_expected
        )));
    }

    Ok(())
}

pub fn validate_gguf_magic(path: &Path) -> Result<u32, ModelRuntimeError> {
    let mut reader = open_reader(path)?;
    read_magic_version(&mut reader, path)
}

pub(crate) fn parse_gguf_tokenizer_metadata(
    path: &Path,
) -> Result<LlamaTokenizer, ModelRuntimeError> {
    let mut reader = open_reader(path)?;
    read_magic_version(&mut reader, path)?;
    let _tensor_count = read_u64(&mut reader, path, "tensor_count")?;
    let metadata_count = read_u64(&mut reader, path, "metadata_kv_count")?;

    let mut tokens = None;
    let mut bos_id = None;
    let mut eos_id = None;

    for _ in 0..metadata_count {
        let key = read_string(&mut reader, path, "metadata key")?;
        let value_type = read_u32(&mut reader, path, "metadata value type")?;

        match key.as_str() {
            "tokenizer.ggml.tokens" => {
                tokens = Some(read_string_array(&mut reader, value_type, path, &key)?);
            }
            "tokenizer.ggml.bos_token_id" => {
                bos_id = Some(read_token_id(&mut reader, value_type, path, &key)?);
            }
            "tokenizer.ggml.eos_token_id" => {
                eos_id = Some(read_token_id(&mut reader, value_type, path, &key)?);
            }
            _ => skip_value(&mut reader, value_type, path, 0)?,
        }
    }

    let tokens = tokens.ok_or_else(|| {
        ModelRuntimeError::LoadError(format!(
            "GGUF tokenizer metadata missing tokenizer.ggml.tokens in {}",
            path.display()
        ))
    })?;

    let special_tokens = collect_special_tokens(&tokens, bos_id, eos_id);
    Ok(LlamaTokenizer {
        vocab_size: tokens.len(),
        bos_id,
        eos_id,
        special_tokens,
    })
}

pub fn sha256_file(path: &Path) -> Result<String, ModelRuntimeError> {
    let mut file = File::open(path).map_err(|error| {
        ModelRuntimeError::LoadError(format!(
            "failed to open llama.cpp artifact {}: {error}",
            path.display()
        ))
    })?;
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let read = file.read(&mut buffer).map_err(|error| {
            ModelRuntimeError::LoadError(format!(
                "failed to read llama.cpp artifact {}: {error}",
                path.display()
            ))
        })?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(hex::encode(hasher.finalize()))
}

fn default_n_threads() -> u32 {
    std::thread::available_parallelism()
        .map(|threads| threads.get().min(8) as u32)
        .unwrap_or(1)
        .max(1)
}

fn open_reader(path: &Path) -> Result<BufReader<File>, ModelRuntimeError> {
    File::open(path).map(BufReader::new).map_err(|error| {
        ModelRuntimeError::LoadError(format!(
            "failed to open llama.cpp artifact {}: {error}",
            path.display()
        ))
    })
}

fn read_magic_version<R: Read>(reader: &mut R, path: &Path) -> Result<u32, ModelRuntimeError> {
    let mut magic = [0_u8; 4];
    if let Err(error) = reader.read_exact(&mut magic) {
        return Err(invalid_gguf(path, Some(error)));
    }

    let version = read_u32(reader, path, "GGUF version").map_err(|error| match error {
        ModelRuntimeError::LoadError(_) => invalid_gguf(path, None),
        other => other,
    })?;

    if &magic != GGUF_MAGIC || !matches!(version, GGUF_VERSION_2 | GGUF_VERSION_3) {
        return Err(invalid_gguf(path, None));
    }

    Ok(version)
}

fn invalid_gguf(path: &Path, error: Option<io::Error>) -> ModelRuntimeError {
    let detail = error.map(|error| format!(": {error}")).unwrap_or_default();
    ModelRuntimeError::LoadError(format!(
        "{NOT_GGUF_V2_V3_ARTIFACT}: {}{detail}",
        path.display()
    ))
}

fn read_string_array<R: Read>(
    reader: &mut R,
    value_type: u32,
    path: &Path,
    key: &str,
) -> Result<Vec<String>, ModelRuntimeError> {
    if value_type != GGUF_TYPE_ARRAY {
        return Err(ModelRuntimeError::LoadError(format!(
            "{key} must be a GGUF array<string> in {}",
            path.display()
        )));
    }

    let element_type = read_u32(reader, path, "array element type")?;
    if element_type != GGUF_TYPE_STRING {
        return Err(ModelRuntimeError::LoadError(format!(
            "{key} must be a GGUF array<string> in {}",
            path.display()
        )));
    }

    let len = read_u64(reader, path, "array length")?;
    ensure_array_len(len, path)?;
    let mut values = Vec::with_capacity(usize::try_from(len).map_err(|_| {
        ModelRuntimeError::LoadError(format!("GGUF array too large in {}", path.display()))
    })?);
    for _ in 0..len {
        values.push(read_string(reader, path, key)?);
    }
    Ok(values)
}

fn read_token_id<R: Read>(
    reader: &mut R,
    value_type: u32,
    path: &Path,
    key: &str,
) -> Result<u32, ModelRuntimeError> {
    let value = match value_type {
        GGUF_TYPE_UINT32 => u64::from(read_u32(reader, path, key)?),
        GGUF_TYPE_UINT64 => read_u64(reader, path, key)?,
        GGUF_TYPE_INT32 => {
            let value = read_i32(reader, path, key)?;
            if value < 0 {
                return Err(invalid_token_id(path, key));
            }
            value as u64
        }
        GGUF_TYPE_INT64 => {
            let value = read_i64(reader, path, key)?;
            if value < 0 {
                return Err(invalid_token_id(path, key));
            }
            value as u64
        }
        _ => {
            return Err(ModelRuntimeError::LoadError(format!(
                "{key} must be a numeric GGUF token id in {}",
                path.display()
            )))
        }
    };

    u32::try_from(value).map_err(|_| invalid_token_id(path, key))
}

fn invalid_token_id(path: &Path, key: &str) -> ModelRuntimeError {
    ModelRuntimeError::LoadError(format!(
        "{key} has an invalid token id in {}",
        path.display()
    ))
}

fn collect_special_tokens(
    tokens: &[String],
    bos_id: Option<u32>,
    eos_id: Option<u32>,
) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut special_tokens = Vec::new();

    for token_id in [bos_id, eos_id].into_iter().flatten() {
        if let Some(token) = tokens.get(token_id as usize) {
            push_unique(token, &mut seen, &mut special_tokens);
        }
    }

    for token in tokens {
        if token.starts_with('<') && token.ends_with('>') {
            push_unique(token, &mut seen, &mut special_tokens);
        }
    }

    special_tokens
}

fn push_unique(token: &str, seen: &mut HashSet<String>, values: &mut Vec<String>) {
    if seen.insert(token.to_string()) {
        values.push(token.to_string());
    }
}

fn skip_value<R: Read>(
    reader: &mut R,
    value_type: u32,
    path: &Path,
    depth: u8,
) -> Result<(), ModelRuntimeError> {
    match value_type {
        GGUF_TYPE_UINT8 | GGUF_TYPE_INT8 | GGUF_TYPE_BOOL => read_exact_len(reader, path, 1),
        GGUF_TYPE_UINT16 | GGUF_TYPE_INT16 => read_exact_len(reader, path, 2),
        GGUF_TYPE_UINT32 | GGUF_TYPE_INT32 | GGUF_TYPE_FLOAT32 => read_exact_len(reader, path, 4),
        GGUF_TYPE_UINT64 | GGUF_TYPE_INT64 | GGUF_TYPE_FLOAT64 => read_exact_len(reader, path, 8),
        GGUF_TYPE_STRING => {
            let _ = read_string(reader, path, "metadata string")?;
            Ok(())
        }
        GGUF_TYPE_ARRAY => {
            if depth > 1 {
                return Err(ModelRuntimeError::LoadError(format!(
                    "unsupported nested GGUF metadata array in {}",
                    path.display()
                )));
            }
            let element_type = read_u32(reader, path, "array element type")?;
            let len = read_u64(reader, path, "array length")?;
            ensure_array_len(len, path)?;
            for _ in 0..len {
                skip_value(reader, element_type, path, depth + 1)?;
            }
            Ok(())
        }
        other => Err(ModelRuntimeError::LoadError(format!(
            "unsupported GGUF metadata type {other} in {}",
            path.display()
        ))),
    }
}

fn ensure_array_len(len: u64, path: &Path) -> Result<(), ModelRuntimeError> {
    if len > MAX_GGUF_ARRAY_VALUES {
        return Err(ModelRuntimeError::LoadError(format!(
            "GGUF metadata array length {len} exceeds cap {MAX_GGUF_ARRAY_VALUES} in {}",
            path.display()
        )));
    }
    Ok(())
}

fn read_exact_len<R: Read>(
    reader: &mut R,
    path: &Path,
    len: usize,
) -> Result<(), ModelRuntimeError> {
    let mut buffer = vec![0_u8; len];
    reader.read_exact(&mut buffer).map_err(|error| {
        ModelRuntimeError::LoadError(format!(
            "failed to read GGUF metadata in {}: {error}",
            path.display()
        ))
    })
}

fn read_string<R: Read>(
    reader: &mut R,
    path: &Path,
    label: &str,
) -> Result<String, ModelRuntimeError> {
    let len = read_u64(reader, path, label)?;
    if len > MAX_GGUF_STRING_BYTES {
        return Err(ModelRuntimeError::LoadError(format!(
            "GGUF {label} length {len} exceeds cap {MAX_GGUF_STRING_BYTES} in {}",
            path.display()
        )));
    }
    let len = usize::try_from(len).map_err(|_| {
        ModelRuntimeError::LoadError(format!("GGUF {label} is too large in {}", path.display()))
    })?;
    let mut bytes = vec![0_u8; len];
    reader.read_exact(&mut bytes).map_err(|error| {
        ModelRuntimeError::LoadError(format!(
            "failed to read GGUF {label} in {}: {error}",
            path.display()
        ))
    })?;
    String::from_utf8(bytes).map_err(|error| {
        ModelRuntimeError::LoadError(format!(
            "GGUF {label} is not UTF-8 in {}: {error}",
            path.display()
        ))
    })
}

fn read_u32<R: Read>(reader: &mut R, path: &Path, label: &str) -> Result<u32, ModelRuntimeError> {
    let mut bytes = [0_u8; 4];
    reader.read_exact(&mut bytes).map_err(|error| {
        ModelRuntimeError::LoadError(format!(
            "failed to read GGUF {label} in {}: {error}",
            path.display()
        ))
    })?;
    Ok(u32::from_le_bytes(bytes))
}

fn read_i32<R: Read>(reader: &mut R, path: &Path, label: &str) -> Result<i32, ModelRuntimeError> {
    let mut bytes = [0_u8; 4];
    reader.read_exact(&mut bytes).map_err(|error| {
        ModelRuntimeError::LoadError(format!(
            "failed to read GGUF {label} in {}: {error}",
            path.display()
        ))
    })?;
    Ok(i32::from_le_bytes(bytes))
}

fn read_u64<R: Read>(reader: &mut R, path: &Path, label: &str) -> Result<u64, ModelRuntimeError> {
    let mut bytes = [0_u8; 8];
    reader.read_exact(&mut bytes).map_err(|error| {
        ModelRuntimeError::LoadError(format!(
            "failed to read GGUF {label} in {}: {error}",
            path.display()
        ))
    })?;
    Ok(u64::from_le_bytes(bytes))
}

fn read_i64<R: Read>(reader: &mut R, path: &Path, label: &str) -> Result<i64, ModelRuntimeError> {
    let mut bytes = [0_u8; 8];
    reader.read_exact(&mut bytes).map_err(|error| {
        ModelRuntimeError::LoadError(format!(
            "failed to read GGUF {label} in {}: {error}",
            path.display()
        ))
    })?;
    Ok(i64::from_le_bytes(bytes))
}
