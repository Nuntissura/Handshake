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

#[derive(Clone, Debug, Default)]
pub struct CancellationToken {
    inner: Arc<AtomicBool>,
}

impl CancellationToken {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn cancel(&self) {
        self.inner.store(true, Ordering::SeqCst);
    }

    pub fn is_cancelled(&self) -> bool {
        self.inner.load(Ordering::SeqCst)
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
