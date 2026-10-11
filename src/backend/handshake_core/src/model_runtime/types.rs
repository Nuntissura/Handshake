use std::{
    fmt,
    path::PathBuf,
    pin::Pin,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};

use futures::Stream;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::storage::artifacts::{bundle_index_content_hash, bundle_index_json, BundleIndexEntry};

use super::{
    error::ModelRuntimeError, ExternalEngineImportRecord, KvCachePolicy, KvPrefixHandle, LoraId,
    ModelCapabilities, SteeringVectorId,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ModelId(Uuid);

impl ModelId {
    pub fn new_v7() -> Self {
        Self(Uuid::now_v7())
    }

    pub fn as_uuid(&self) -> Uuid {
        self.0
    }
}

impl Default for ModelId {
    fn default() -> Self {
        Self::new_v7()
    }
}

impl From<Uuid> for ModelId {
    fn from(value: Uuid) -> Self {
        Self(value)
    }
}

impl fmt::Display for ModelId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeKind {
    LlamaCpp,
    Candle,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderKind {
    Local,
    ExternalCompat,
    ByokCloud,
    OfficialCli,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SamplingParams {
    pub temperature: Option<f32>,
    pub top_p: Option<f32>,
    pub top_k: Option<u32>,
    pub min_p: Option<f32>,
    pub repetition_penalty: Option<f32>,
    pub frequency_penalty: Option<f32>,
    pub presence_penalty: Option<f32>,
    pub seed: Option<u32>,
}

impl Default for SamplingParams {
    fn default() -> Self {
        Self {
            temperature: None,
            top_p: None,
            top_k: None,
            min_p: None,
            repetition_penalty: None,
            frequency_penalty: None,
            presence_penalty: None,
            seed: None,
        }
    }
}

#[derive(Debug)]
pub struct LoadSpec {
    pub artifact_path: PathBuf,
    pub sha256_expected: String,
    pub runtime_kind: RuntimeKind,
    pub sampling_defaults: SamplingParams,
    pub kv_cache_policy: KvCachePolicy,
    pub declared_capabilities: ModelCapabilities,
    pub provider: ProviderKind,
    pub engine_origin: Option<String>,
    pub external_engine_import: Option<ExternalEngineImportRecord>,
}

/// Digest and exact byte length of one behavior-bearing model artifact
/// component captured by a runtime before model construction.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelArtifactComponentIntegrity {
    pub sha256: String,
    pub length_bytes: u64,
}

/// Path-independent receipt for the exact Candle artifact bytes consumed by a
/// runtime load. This safetensors/config/tokenizer shape is not a universal
/// contract for GGUF or other artifact formats.
///
/// `bundle_sha256` is the SHA-256 of the project's canonical bundle-index JSON
/// over the fixed semantic component names `model.safetensors`, `config.json`,
/// and (when present) `tokenizer.json`. Source directories and absolute paths
/// are deliberately excluded from the digest basis.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelArtifactIntegrityReceipt {
    pub schema_id: String,
    pub bundle_sha256: String,
    pub weights: ModelArtifactComponentIntegrity,
    pub config: ModelArtifactComponentIntegrity,
    pub tokenizer: Option<ModelArtifactComponentIntegrity>,
}

/// Path-independent receipt for one exact GGUF artifact. GGUF embeds its
/// configuration and tokenizer metadata in the same file as the tensor data,
/// so representing it as Candle `config`/`tokenizer` components would be a
/// false claim.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LlamaCppArtifactIntegrityReceipt {
    pub schema_id: String,
    pub bundle_sha256: String,
    pub gguf: ModelArtifactComponentIntegrity,
}

/// Format-discriminated receipt returned by model runtimes. The enum is
/// deliberately untagged: serializing the Candle variant produces the exact
/// pre-existing Candle JSON object, while each variant's mandatory component
/// fields and `schema_id` provide fail-closed format discrimination.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum RuntimeArtifactIntegrityReceipt {
    Candle(ModelArtifactIntegrityReceipt),
    LlamaCpp(LlamaCppArtifactIntegrityReceipt),
}

pub const CANDLE_ARTIFACT_INTEGRITY_SCHEMA_ID: &str =
    "handshake.model_artifact_integrity.candle.v1";
pub const LLAMA_CPP_ARTIFACT_INTEGRITY_SCHEMA_ID: &str =
    "handshake.model_artifact_integrity.gguf.v1";
const CANDLE_WEIGHTS_COMPONENT_NAME: &str = "model.safetensors";
const CANDLE_CONFIG_COMPONENT_NAME: &str = "config.json";
const CANDLE_TOKENIZER_COMPONENT_NAME: &str = "tokenizer.json";
const LLAMA_CPP_GGUF_COMPONENT_NAME: &str = "model.gguf";

impl ModelArtifactIntegrityReceipt {
    pub(crate) fn from_candle_components(
        weights: ModelArtifactComponentIntegrity,
        config: ModelArtifactComponentIntegrity,
        tokenizer: Option<ModelArtifactComponentIntegrity>,
    ) -> Result<Self, ModelRuntimeError> {
        let bundle_sha256 = canonical_candle_bundle_sha256(&weights, &config, tokenizer.as_ref())?;
        Ok(Self {
            schema_id: CANDLE_ARTIFACT_INTEGRITY_SCHEMA_ID.to_string(),
            bundle_sha256,
            weights,
            config,
            tokenizer,
        })
    }

    /// Validate the bounded receipt shape and bind it back to the operator's
    /// configured weights digest before any READY or durable-ledger exposure.
    pub fn validate_for_expected_weights(
        &self,
        expected_weights_sha256: [u8; 32],
    ) -> Result<(), ModelRuntimeError> {
        if self.schema_id != CANDLE_ARTIFACT_INTEGRITY_SCHEMA_ID {
            return Err(ModelRuntimeError::LoadError(format!(
                "invalid Candle artifact integrity schema id {:?}; expected {:?}",
                self.schema_id, CANDLE_ARTIFACT_INTEGRITY_SCHEMA_ID
            )));
        }
        validate_integrity_digest("bundle", &self.bundle_sha256)?;
        let weights = validate_integrity_component("weights", &self.weights)?;
        validate_integrity_component("config", &self.config)?;
        if let Some(tokenizer) = &self.tokenizer {
            validate_integrity_component("tokenizer", tokenizer)?;
        }
        let canonical_bundle =
            canonical_candle_bundle_sha256(&self.weights, &self.config, self.tokenizer.as_ref())?;
        if self.bundle_sha256 != canonical_bundle {
            return Err(ModelRuntimeError::LoadError(format!(
                "model artifact integrity bundle sha256 mismatch: receipt {}, canonical {}",
                self.bundle_sha256, canonical_bundle
            )));
        }
        if weights != expected_weights_sha256 {
            return Err(ModelRuntimeError::LoadError(format!(
                "model artifact integrity receipt weights sha256 mismatch: expected {}, got {}",
                hex::encode(expected_weights_sha256),
                self.weights.sha256
            )));
        }
        Ok(())
    }
}

impl LlamaCppArtifactIntegrityReceipt {
    pub(crate) fn from_gguf_component(
        gguf: ModelArtifactComponentIntegrity,
    ) -> Result<Self, ModelRuntimeError> {
        let bundle_sha256 =
            canonical_single_component_bundle_sha256(LLAMA_CPP_GGUF_COMPONENT_NAME, &gguf, "GGUF")?;
        Ok(Self {
            schema_id: LLAMA_CPP_ARTIFACT_INTEGRITY_SCHEMA_ID.to_string(),
            bundle_sha256,
            gguf,
        })
    }

    fn validate_for_expected_gguf(
        &self,
        expected_gguf_sha256: [u8; 32],
    ) -> Result<(), ModelRuntimeError> {
        if self.schema_id != LLAMA_CPP_ARTIFACT_INTEGRITY_SCHEMA_ID {
            return Err(ModelRuntimeError::LoadError(format!(
                "invalid llama.cpp artifact integrity schema id {:?}; expected {:?}",
                self.schema_id, LLAMA_CPP_ARTIFACT_INTEGRITY_SCHEMA_ID
            )));
        }
        validate_integrity_digest("bundle", &self.bundle_sha256)?;
        let gguf = validate_integrity_component("gguf", &self.gguf)?;
        let canonical_bundle = canonical_single_component_bundle_sha256(
            LLAMA_CPP_GGUF_COMPONENT_NAME,
            &self.gguf,
            "GGUF",
        )?;
        if self.bundle_sha256 != canonical_bundle {
            return Err(ModelRuntimeError::LoadError(format!(
                "model artifact integrity bundle sha256 mismatch: receipt {}, canonical {}",
                self.bundle_sha256, canonical_bundle
            )));
        }
        if gguf != expected_gguf_sha256 {
            return Err(ModelRuntimeError::LoadError(format!(
                "model artifact integrity receipt GGUF sha256 mismatch: expected {}, got {}",
                hex::encode(expected_gguf_sha256),
                self.gguf.sha256
            )));
        }
        Ok(())
    }
}

impl RuntimeArtifactIntegrityReceipt {
    /// Validate that the receipt format matches the selected runtime and bind
    /// its primary component to the operator-configured digest.
    pub fn validate_for_runtime_expected(
        &self,
        runtime_kind: RuntimeKind,
        expected_sha256: [u8; 32],
    ) -> Result<(), ModelRuntimeError> {
        match (runtime_kind, self) {
            (RuntimeKind::Candle, Self::Candle(receipt)) => {
                receipt.validate_for_expected_weights(expected_sha256)
            }
            (RuntimeKind::LlamaCpp, Self::LlamaCpp(receipt)) => {
                receipt.validate_for_expected_gguf(expected_sha256)
            }
            (runtime_kind, receipt) => Err(ModelRuntimeError::LoadError(format!(
                "artifact integrity receipt format {} does not match runtime {runtime_kind:?}",
                receipt.schema_id()
            ))),
        }
    }

    pub fn schema_id(&self) -> &str {
        match self {
            Self::Candle(receipt) => &receipt.schema_id,
            Self::LlamaCpp(receipt) => &receipt.schema_id,
        }
    }

    /// Raw digest placed in ProcessOwnershipLedger.model_artifact_sha256.
    pub fn primary_artifact_sha256(&self) -> &str {
        match self {
            Self::Candle(receipt) => &receipt.weights.sha256,
            Self::LlamaCpp(receipt) => &receipt.gguf.sha256,
        }
    }
}

impl From<ModelArtifactIntegrityReceipt> for RuntimeArtifactIntegrityReceipt {
    fn from(receipt: ModelArtifactIntegrityReceipt) -> Self {
        Self::Candle(receipt)
    }
}

impl From<LlamaCppArtifactIntegrityReceipt> for RuntimeArtifactIntegrityReceipt {
    fn from(receipt: LlamaCppArtifactIntegrityReceipt) -> Self {
        Self::LlamaCpp(receipt)
    }
}

fn canonical_candle_bundle_sha256(
    weights: &ModelArtifactComponentIntegrity,
    config: &ModelArtifactComponentIntegrity,
    tokenizer: Option<&ModelArtifactComponentIntegrity>,
) -> Result<String, ModelRuntimeError> {
    let mut entries = vec![
        artifact_bundle_entry(CANDLE_WEIGHTS_COMPONENT_NAME, weights),
        artifact_bundle_entry(CANDLE_CONFIG_COMPONENT_NAME, config),
    ];
    if let Some(tokenizer) = tokenizer {
        entries.push(artifact_bundle_entry(
            CANDLE_TOKENIZER_COMPONENT_NAME,
            tokenizer,
        ));
    }
    let canonical = bundle_index_json(&entries).map_err(|error| {
        ModelRuntimeError::LoadError(format!(
            "failed to canonicalize Candle artifact integrity receipt: {error}"
        ))
    })?;
    Ok(bundle_index_content_hash(&canonical))
}

fn canonical_single_component_bundle_sha256(
    component_name: &str,
    integrity: &ModelArtifactComponentIntegrity,
    format_name: &str,
) -> Result<String, ModelRuntimeError> {
    let canonical = bundle_index_json(&[artifact_bundle_entry(component_name, integrity)])
        .map_err(|error| {
            ModelRuntimeError::LoadError(format!(
                "failed to canonicalize {format_name} artifact integrity receipt: {error}"
            ))
        })?;
    Ok(bundle_index_content_hash(&canonical))
}

fn artifact_bundle_entry(
    name: &str,
    integrity: &ModelArtifactComponentIntegrity,
) -> BundleIndexEntry {
    BundleIndexEntry {
        path: name.to_string(),
        content_hash: integrity.sha256.clone(),
        size_bytes: integrity.length_bytes,
    }
}

fn validate_integrity_component(
    name: &str,
    component: &ModelArtifactComponentIntegrity,
) -> Result<[u8; 32], ModelRuntimeError> {
    if component.length_bytes == 0 {
        return Err(ModelRuntimeError::LoadError(format!(
            "model artifact integrity {name} component is empty"
        )));
    }
    validate_integrity_digest(name, &component.sha256)
}

fn validate_integrity_digest(name: &str, digest: &str) -> Result<[u8; 32], ModelRuntimeError> {
    if digest.len() != 64
        || !digest
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(ModelRuntimeError::LoadError(format!(
            "model artifact integrity {name} sha256 is not canonical lowercase hex"
        )));
    }
    let decoded = hex::decode(digest).map_err(|error| {
        ModelRuntimeError::LoadError(format!(
            "model artifact integrity {name} sha256 cannot be decoded: {error}"
        ))
    })?;
    decoded.try_into().map_err(|bytes: Vec<u8>| {
        ModelRuntimeError::LoadError(format!(
            "model artifact integrity {name} sha256 decoded to {} bytes, expected 32",
            bytes.len()
        ))
    })
}

impl LoadSpec {
    pub fn with_engine_origin(mut self, engine_origin: impl Into<String>) -> Self {
        self.engine_origin = Some(engine_origin.into());
        self
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GenPrompt {
    pub text: String,
}

impl GenPrompt {
    pub fn new(text: impl Into<String>) -> Self {
        Self { text: text.into() }
    }

    pub fn as_str(&self) -> &str {
        &self.text
    }
}

impl From<String> for GenPrompt {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

impl From<&str> for GenPrompt {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct JsonSchema {
    pub value: serde_json::Value,
}

impl JsonSchema {
    pub fn new(value: serde_json::Value) -> Self {
        Self { value }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct GenerateRequest {
    pub id: ModelId,
    pub prompt: GenPrompt,
    pub sampling: SamplingParams,
    pub lora_overrides: Vec<LoraId>,
    pub steering_overrides: Vec<SteeringVectorId>,
    pub kv_prefix_handle: Option<KvPrefixHandle>,
    pub cancel: CancellationToken,
    pub max_tokens: u32,
    pub stop_sequences: Vec<String>,
    pub speculative_mode: Option<SpeculativeMode>,
    pub structured_decoding: Option<JsonSchema>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "mode")]
pub enum SpeculativeMode {
    Ngram { lookback: u32, max_draft: u32 },
    DraftModel { draft_id: ModelId, max_draft: u32 },
    Eagle3 { max_draft: u32 },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GeneratedToken {
    pub token_id: u32,
    pub text: String,
    pub logprob: Option<f32>,
    pub finish_reason: Option<FinishReason>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FinishReason {
    Stop,
    Length,
    Cancelled,
    Error,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Score {
    pub token_logprobs: Vec<f32>,
    pub mean_logprob: f32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Embedding {
    pub vector: Vec<f32>,
}

#[derive(Clone, Debug)]
pub struct CancellationToken {
    inner: Arc<AtomicBool>,
    notify: Arc<tokio::sync::Notify>,
}

impl Default for CancellationToken {
    fn default() -> Self {
        Self {
            inner: Arc::new(AtomicBool::new(false)),
            notify: Arc::new(tokio::sync::Notify::new()),
        }
    }
}

impl CancellationToken {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn cancel(&self) {
        self.inner.store(true, Ordering::SeqCst);
        self.notify.notify_waiters();
    }

    pub fn is_cancelled(&self) -> bool {
        self.inner.load(Ordering::SeqCst)
    }

    pub async fn cancelled(&self) {
        loop {
            if self.is_cancelled() {
                return;
            }
            let notified = self.notify.notified();
            if self.is_cancelled() {
                return;
            }
            notified.await;
        }
    }
}

impl PartialEq for CancellationToken {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.inner, &other.inner) || self.is_cancelled() == other.is_cancelled()
    }
}

impl Eq for CancellationToken {}

pub type TokenStream =
    Pin<Box<dyn Stream<Item = Result<GeneratedToken, ModelRuntimeError>> + Send>>;

/// Maximum generated-token items buffered between a blocking native inference
/// worker and its async consumer. Backpressure beyond this bound is
/// cancellation-aware so a dropped/slow stream cannot grow memory without bound
/// or strand shutdown behind a permanently full queue.
pub const MODEL_RUNTIME_TOKEN_STREAM_CAPACITY: usize = 64;

#[cfg(test)]
mod artifact_integrity_tests {
    use super::*;

    fn component(byte: u8, length_bytes: u64) -> ModelArtifactComponentIntegrity {
        ModelArtifactComponentIntegrity {
            sha256: hex::encode([byte; 32]),
            length_bytes,
        }
    }

    #[test]
    fn mt013_forged_bundle_digest_and_non_candle_schema_fail_closed() {
        let expected_weights = [0x11; 32];
        let mut receipt = ModelArtifactIntegrityReceipt::from_candle_components(
            component(0x11, 1024),
            component(0x22, 256),
            Some(component(0x33, 128)),
        )
        .expect("canonical Candle receipt");
        receipt
            .validate_for_expected_weights(expected_weights)
            .expect("canonical receipt validates");

        receipt.bundle_sha256 = "00".repeat(32);
        let error = receipt
            .validate_for_expected_weights(expected_weights)
            .expect_err("well-formed forged bundle digest must be rejected");
        assert!(
            error.to_string().contains("bundle sha256 mismatch"),
            "{error}"
        );

        let mut wrong_schema = ModelArtifactIntegrityReceipt::from_candle_components(
            component(0x11, 1024),
            component(0x22, 256),
            None,
        )
        .expect("canonical Candle receipt");
        wrong_schema.schema_id = "handshake.model_artifact_integrity.other.v1".to_string();
        let error = wrong_schema
            .validate_for_expected_weights(expected_weights)
            .expect_err("non-Candle schema must be rejected at Candle boot boundary");
        assert!(
            error
                .to_string()
                .contains("Candle artifact integrity schema"),
            "{error}"
        );
    }

    #[test]
    fn mt013_runtime_receipt_preserves_candle_json_shape_and_rejects_format_confusion() {
        let candle = ModelArtifactIntegrityReceipt::from_candle_components(
            component(0x11, 1024),
            component(0x22, 256),
            Some(component(0x33, 128)),
        )
        .expect("canonical Candle receipt");
        let legacy_json = serde_json::to_value(&candle).expect("serialize Candle receipt");
        let runtime_json =
            serde_json::to_value(RuntimeArtifactIntegrityReceipt::from(candle.clone()))
                .expect("serialize runtime receipt");
        assert_eq!(runtime_json, legacy_json);

        let gguf = LlamaCppArtifactIntegrityReceipt::from_gguf_component(component(0x44, 2048))
            .expect("canonical GGUF receipt");
        let runtime = RuntimeArtifactIntegrityReceipt::from(gguf);
        runtime
            .validate_for_runtime_expected(RuntimeKind::LlamaCpp, [0x44; 32])
            .expect("GGUF receipt validates for llama.cpp");
        let error = runtime
            .validate_for_runtime_expected(RuntimeKind::Candle, [0x44; 32])
            .expect_err("GGUF receipt cannot satisfy a Candle runtime");
        assert!(error.to_string().contains("does not match runtime Candle"));
    }

    #[test]
    fn mt013_gguf_receipt_rejects_schema_digest_and_bundle_forgery() {
        let expected = [0x55; 32];
        let canonical =
            LlamaCppArtifactIntegrityReceipt::from_gguf_component(component(0x55, 4096))
                .expect("canonical GGUF receipt");

        let mut wrong_schema = canonical.clone();
        wrong_schema.schema_id = "handshake.model_artifact_integrity.other.v1".to_string();
        assert!(RuntimeArtifactIntegrityReceipt::from(wrong_schema)
            .validate_for_runtime_expected(RuntimeKind::LlamaCpp, expected)
            .expect_err("wrong GGUF schema must fail")
            .to_string()
            .contains("llama.cpp artifact integrity schema"));

        assert!(RuntimeArtifactIntegrityReceipt::from(canonical.clone())
            .validate_for_runtime_expected(RuntimeKind::LlamaCpp, [0x56; 32])
            .expect_err("wrong expected GGUF digest must fail")
            .to_string()
            .contains("GGUF sha256 mismatch"));

        let mut forged_bundle = canonical;
        forged_bundle.bundle_sha256 = "00".repeat(32);
        assert!(RuntimeArtifactIntegrityReceipt::from(forged_bundle)
            .validate_for_runtime_expected(RuntimeKind::LlamaCpp, expected)
            .expect_err("forged GGUF bundle must fail")
            .to_string()
            .contains("bundle sha256 mismatch"));
    }
}
