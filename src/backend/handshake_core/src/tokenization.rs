//! Tokenization Service (normative) - panic-free token counting and truncation.
//! Aligns with Master Spec A4.6 Tokenization Service and WP-1-Tokenization-Service-v3.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use serde::Serialize;
use serde_json::{json, Value};
use thiserror::Error;
use tracing::{error, warn};
use uuid::Uuid;

use crate::flight_recorder::{
    FlightRecorder, FlightRecorderActor, FlightRecorderEvent, FlightRecorderEventType,
};

#[derive(Debug, Error, Clone)]
pub enum TokenizerError {
    #[error("Unknown model: {0}")]
    UnknownModel(String),
    #[error("Tokenization failed: {0}")]
    TokenizationFailed(String),
    #[error("Tokenizer unavailable: {0}")]
    TokenizerUnavailable(String),
    #[error("Decode failed: {0}")]
    DecodeFailed(String),
    #[error("Tokenizer config fetch failed: {0}")]
    TokenizerConfigFetchFailed(String),
    #[error("Tokenizer config parse failed: {0}")]
    TokenizerConfigParseFailed(String),
    #[error("Tokenizer config missing: {0}")]
    TokenizerConfigMissing(String),
    #[error("Tokenizer config unsupported: {0}")]
    TokenizerConfigUnsupported(String),
    #[error("Tokenizer init failed: {0}")]
    TokenizerInitFailed(String),
}

pub trait TokenizationService: Send + Sync {
    fn count_tokens(&self, text: &str, model: &str) -> Result<u32, TokenizerError>;
    fn truncate(&self, text: &str, limit: u32, model: &str) -> String;
}

pub trait TokenizerEngine: Send + Sync {
    fn count_tokens(&self, text: &str) -> Result<u32, TokenizerError>;
    fn truncate(&self, text: &str, limit: u32) -> Result<String, TokenizerError>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenizerKind {
    Tiktoken,
    SentencePiece,
    Vibe,
}

impl TokenizerKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            TokenizerKind::Tiktoken => "tiktoken",
            TokenizerKind::SentencePiece => "sentencepiece",
            TokenizerKind::Vibe => "vibe",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccuracyWarningReason {
    UnknownModel,
    ConfigFetchFailed,
    ConfigMissing,
    UnsupportedTokenizer,
    TokenizerInitFailed,
}

impl AccuracyWarningReason {
    pub fn as_str(&self) -> &'static str {
        match self {
            AccuracyWarningReason::UnknownModel => "unknown_model",
            AccuracyWarningReason::ConfigFetchFailed => "config_fetch_failed",
            AccuracyWarningReason::ConfigMissing => "config_missing",
            AccuracyWarningReason::UnsupportedTokenizer => "unsupported_tokenizer",
            AccuracyWarningReason::TokenizerInitFailed => "tokenizer_init_failed",
        }
    }
}

#[derive(Debug, Clone)]
pub struct TokenizationOutcome {
    pub count: u32,
    pub tokenizer_kind: TokenizerKind,
    pub used_fallback: bool,
    pub fallback_reason: Option<AccuracyWarningReason>,
}

#[derive(Debug, Clone)]
pub struct TruncationOutcome {
    pub text: String,
    pub tokenizer_kind: TokenizerKind,
    pub used_fallback: bool,
    pub fallback_reason: Option<AccuracyWarningReason>,
}

#[async_trait]
pub trait Tokenizer: Send + Sync {
    async fn count_tokens(&self, text: &str, model: &str) -> Result<u32, TokenizerError>;
    async fn truncate(&self, text: &str, limit: u32, model: &str)
        -> Result<String, TokenizerError>;
}

#[derive(Debug, Clone)]
pub struct TiktokenAdapter {
    fallback_model: &'static str,
}

impl Default for TiktokenAdapter {
    fn default() -> Self {
        Self {
            fallback_model: "cl100k_base",
        }
    }
}

impl TiktokenAdapter {
    fn engine_for_model(&self, model: &str) -> Result<TiktokenEngine, TokenizerError> {
        #[cfg(feature = "tokenization")]
        {
            use tiktoken_rs::get_bpe_from_model;

            match get_bpe_from_model(model) {
                Ok(bpe) => Ok(TiktokenEngine { bpe }),
                Err(err) => {
                    warn!(
                        target: "handshake_core::tokenization",
                        model = %model,
                        fallback_model = %self.fallback_model,
                        error = %err,
                        "tiktoken model unknown; falling back to cl100k_base"
                    );
                    self.engine_for_encoding(self.fallback_model)
                }
            }
        }

        #[cfg(not(feature = "tokenization"))]
        {
            Err(TokenizerError::TokenizerUnavailable(
                "tokenization feature disabled".to_string(),
            ))
        }
    }

    fn engine_for_encoding(&self, encoding: &str) -> Result<TiktokenEngine, TokenizerError> {
        #[cfg(feature = "tokenization")]
        {
            use tiktoken_rs::get_bpe_from_tokenizer;
            use tiktoken_rs::tokenizer::Tokenizer;

            let encoding = encoding.to_ascii_lowercase();
            let tokenizer = match encoding.as_str() {
                "o200k_base" => Some(Tokenizer::O200kBase),
                "cl100k_base" => Some(Tokenizer::Cl100kBase),
                "p50k_base" => Some(Tokenizer::P50kBase),
                "r50k_base" => Some(Tokenizer::R50kBase),
                "p50k_edit" => Some(Tokenizer::P50kEdit),
                "gpt2" => Some(Tokenizer::Gpt2),
                _ => None,
            };

            let tokenizer = tokenizer.ok_or_else(|| {
                TokenizerError::TokenizerConfigParseFailed(format!(
                    "unsupported tiktoken encoding: {}",
                    encoding
                ))
            })?;

            get_bpe_from_tokenizer(tokenizer)
                .map(|bpe| TiktokenEngine { bpe })
                .map_err(|err| TokenizerError::TokenizerInitFailed(err.to_string()))
        }

        #[cfg(not(feature = "tokenization"))]
        {
            Err(TokenizerError::TokenizerUnavailable(
                "tokenization feature disabled".to_string(),
            ))
        }
    }
}

#[cfg(feature = "tokenization")]
#[derive(Debug, Clone)]
pub struct TiktokenEngine {
    bpe: tiktoken_rs::CoreBPE,
}

#[cfg(not(feature = "tokenization"))]
#[derive(Debug, Clone)]
pub struct TiktokenEngine;

#[cfg(feature = "tokenization")]
impl TokenizerEngine for TiktokenEngine {
    fn count_tokens(&self, text: &str) -> Result<u32, TokenizerError> {
        Ok(self.bpe.encode_ordinary(text).len() as u32)
    }

    fn truncate(&self, text: &str, limit: u32) -> Result<String, TokenizerError> {
        let mut tokens = self.bpe.encode_ordinary(text);
        tokens.truncate(limit as usize);
        self.bpe
            .decode(tokens)
            .map_err(|err| TokenizerError::DecodeFailed(err.to_string()))
    }
}

#[cfg(not(feature = "tokenization"))]
impl TokenizerEngine for TiktokenEngine {
    fn count_tokens(&self, _text: &str) -> Result<u32, TokenizerError> {
        Err(TokenizerError::TokenizerUnavailable(
            "tokenization feature disabled".to_string(),
        ))
    }

    fn truncate(&self, _text: &str, _limit: u32) -> Result<String, TokenizerError> {
        Err(TokenizerError::TokenizerUnavailable(
            "tokenization feature disabled".to_string(),
        ))
    }
}

#[cfg(feature = "tokenization")]
#[derive(Debug, Clone)]
pub struct SentencePieceTokenizer {
    tokenizer: tokenizers::Tokenizer,
}

#[cfg(not(feature = "tokenization"))]
#[derive(Debug, Clone)]
pub struct SentencePieceTokenizer;

impl SentencePieceTokenizer {
    pub fn from_file(path: &str) -> Result<Self, TokenizerError> {
        #[cfg(feature = "tokenization")]
        {
            let tokenizer = tokenizers::Tokenizer::from_file(path)
                .map_err(|err| TokenizerError::TokenizerInitFailed(err.to_string()))?;
            Ok(Self { tokenizer })
        }

        #[cfg(not(feature = "tokenization"))]
        {
            Err(TokenizerError::TokenizerUnavailable(
                "tokenization feature disabled".to_string(),
            ))
        }
    }
}

#[cfg(feature = "tokenization")]
impl TokenizerEngine for SentencePieceTokenizer {
    fn count_tokens(&self, text: &str) -> Result<u32, TokenizerError> {
        let encoding = self
            .tokenizer
            .encode(text, false)
            .map_err(|err| TokenizerError::TokenizationFailed(err.to_string()))?;
        Ok(encoding.get_ids().len() as u32)
    }

    fn truncate(&self, text: &str, limit: u32) -> Result<String, TokenizerError> {
        let encoding = self
            .tokenizer
            .encode(text, false)
            .map_err(|err| TokenizerError::TokenizationFailed(err.to_string()))?;
        let truncated_ids: Vec<u32> = encoding
            .get_ids()
            .iter()
            .take(limit as usize)
            .copied()
            .collect();
        self.tokenizer
            .decode(&truncated_ids, true)
            .map_err(|err| TokenizerError::DecodeFailed(err.to_string()))
    }
}

#[cfg(not(feature = "tokenization"))]
impl TokenizerEngine for SentencePieceTokenizer {
    fn count_tokens(&self, _text: &str) -> Result<u32, TokenizerError> {
        Err(TokenizerError::TokenizerUnavailable(
            "tokenization feature disabled".to_string(),
        ))
    }

    fn truncate(&self, _text: &str, _limit: u32) -> Result<String, TokenizerError> {
        Err(TokenizerError::TokenizerUnavailable(
            "tokenization feature disabled".to_string(),
        ))
    }
}

#[derive(Debug, Default)]
pub struct SentencePieceTokenizerCache {
    inner: Mutex<HashMap<String, Arc<SentencePieceTokenizer>>>,
}

impl SentencePieceTokenizerCache {
    pub fn get_or_load(
        &self,
        model_path: &str,
    ) -> Result<Arc<SentencePieceTokenizer>, TokenizerError> {
        let cached = {
            let guard = self.inner.lock().map_err(|_| {
                TokenizerError::TokenizerInitFailed("sentencepiece cache lock poisoned".to_string())
            })?;
            guard.get(model_path).cloned()
        };

        if let Some(tokenizer) = cached {
            return Ok(tokenizer);
        }

        let tokenizer = Arc::new(SentencePieceTokenizer::from_file(model_path)?);

        let mut guard = self.inner.lock().map_err(|_| {
            TokenizerError::TokenizerInitFailed("sentencepiece cache lock poisoned".to_string())
        })?;
        guard.insert(model_path.to_string(), tokenizer.clone());
        Ok(tokenizer)
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct VibeTokenizer;

impl VibeTokenizer {
    pub fn count_tokens_sync(&self, text: &str) -> u32 {
        vibe_count_tokens(text)
    }

    pub fn truncate_sync(&self, text: &str, limit: u32) -> String {
        vibe_truncate(text, limit)
    }
}

impl TokenizerEngine for VibeTokenizer {
    fn count_tokens(&self, text: &str) -> Result<u32, TokenizerError> {
        Ok(self.count_tokens_sync(text))
    }

    fn truncate(&self, text: &str, limit: u32) -> Result<String, TokenizerError> {
        Ok(self.truncate_sync(text, limit))
    }
}

#[async_trait]
impl Tokenizer for VibeTokenizer {
    async fn count_tokens(&self, text: &str, _model: &str) -> Result<u32, TokenizerError> {
        Ok(self.count_tokens_sync(text))
    }

    async fn truncate(
        &self,
        text: &str,
        limit: u32,
        _model: &str,
    ) -> Result<String, TokenizerError> {
        Ok(self.truncate_sync(text, limit))
    }
}

#[async_trait]
impl Tokenizer for TiktokenAdapter {
    async fn count_tokens(&self, text: &str, model: &str) -> Result<u32, TokenizerError> {
        let engine = self.engine_for_model(model)?;
        engine.count_tokens(text)
    }

    async fn truncate(
        &self,
        text: &str,
        limit: u32,
        model: &str,
    ) -> Result<String, TokenizerError> {
        let engine = self.engine_for_model(model)?;
        engine.truncate(text, limit)
    }
}

#[derive(Debug, Clone)]
pub enum TokenizerConfigData {
    Tiktoken { encoding: String },
    SentencePiece { model_path: String },
}

#[derive(Debug, Clone)]
pub struct ResolvedTokenizerConfig {
    pub kind: TokenizerKind,
    pub data: TokenizerConfigData,
}

#[async_trait]
pub trait TokenizerConfigFetcher: Send + Sync {
    async fn fetch_show(&self, model: &str) -> Result<Value, TokenizerError>;
}

#[derive(Debug, Clone)]
enum CacheEntry {
    Ready(ResolvedTokenizerConfig),
    Failed(TokenizerError),
}

pub struct OllamaTokenizerConfigCache {
    fetcher: Arc<dyn TokenizerConfigFetcher>,
    inner: Mutex<HashMap<String, CacheEntry>>,
}

impl OllamaTokenizerConfigCache {
    pub fn new(fetcher: Arc<dyn TokenizerConfigFetcher>) -> Self {
        Self {
            fetcher,
            inner: Mutex::new(HashMap::new()),
        }
    }

    pub fn disabled() -> Self {
        Self::new(Arc::new(DisabledTokenizerConfigFetcher))
    }

    pub async fn refresh(&self, model: &str) -> Result<(), TokenizerError> {
        let fetched = self.fetcher.fetch_show(model).await;
        let resolved = fetched.and_then(|value| map_ollama_show_to_tokenizer_config(&value));

        let entry = match &resolved {
            Ok(config) => CacheEntry::Ready(config.clone()),
            Err(err) => CacheEntry::Failed(err.clone()),
        };

        let mut guard = self.inner.lock().map_err(|_| {
            TokenizerError::TokenizerInitFailed("tokenizer config cache lock poisoned".to_string())
        })?;
        guard.insert(model.to_string(), entry);

        resolved.map(|_| ())
    }

    pub fn resolve(&self, model: &str) -> Result<ResolvedTokenizerConfig, TokenizerError> {
        let entry = {
            let guard = self.inner.lock().map_err(|_| {
                TokenizerError::TokenizerInitFailed(
                    "tokenizer config cache lock poisoned".to_string(),
                )
            })?;
            guard.get(model).cloned()
        };

        match entry {
            Some(CacheEntry::Ready(config)) => Ok(config),
            Some(CacheEntry::Failed(err)) => Err(err),
            None => Err(TokenizerError::TokenizerConfigMissing(format!(
                "no tokenizer config cached for model: {}",
                model
            ))),
        }
    }
}

struct DisabledTokenizerConfigFetcher;

#[async_trait]
impl TokenizerConfigFetcher for DisabledTokenizerConfigFetcher {
    async fn fetch_show(&self, model: &str) -> Result<Value, TokenizerError> {
        Err(TokenizerError::TokenizerConfigMissing(format!(
            "tokenizer config fetcher disabled for model: {}",
            model
        )))
    }
}

pub struct TokenizationRouter {
    tiktoken: Arc<TiktokenAdapter>,
    vibe: Arc<VibeTokenizer>,
    sentencepiece_cache: SentencePieceTokenizerCache,
    config_cache: Arc<OllamaTokenizerConfigCache>,
}

impl TokenizationRouter {
    pub fn new(tiktoken: Arc<TiktokenAdapter>, vibe: Arc<VibeTokenizer>) -> Self {
        Self {
            tiktoken,
            vibe,
            sentencepiece_cache: SentencePieceTokenizerCache::default(),
            config_cache: Arc::new(OllamaTokenizerConfigCache::disabled()),
        }
    }

    pub fn new_with_ollama_config(
        tiktoken: Arc<TiktokenAdapter>,
        vibe: Arc<VibeTokenizer>,
        sentencepiece_cache: SentencePieceTokenizerCache,
        config_cache: Arc<OllamaTokenizerConfigCache>,
    ) -> Self {
        Self {
            tiktoken,
            vibe,
            sentencepiece_cache,
            config_cache,
        }
    }

    pub fn count_tokens_internal(
        &self,
        text: &str,
        model: &str,
    ) -> Result<TokenizationOutcome, TokenizerError> {
        if model.trim().is_empty() {
            return Ok(self.fallback_count(
                text,
                model,
                TokenizerError::UnknownModel("empty model".to_string()),
            ));
        }

        if is_gpt_class(model) {
            let engine = match self.tiktoken.engine_for_model(model) {
                Ok(engine) => engine,
                Err(err) => return Ok(self.fallback_count(text, model, err)),
            };

            return engine
                .count_tokens(text)
                .map(|count| TokenizationOutcome {
                    count,
                    tokenizer_kind: TokenizerKind::Tiktoken,
                    used_fallback: false,
                    fallback_reason: None,
                })
                .or_else(|err| Ok(self.fallback_count(text, model, err)));
        }

        match self.config_cache.resolve(model) {
            Ok(config) => match config.data {
                TokenizerConfigData::SentencePiece { model_path } => {
                    let tokenizer = match self.sentencepiece_cache.get_or_load(&model_path) {
                        Ok(tokenizer) => tokenizer,
                        Err(err) => return Ok(self.fallback_count(text, model, err)),
                    };

                    tokenizer
                        .count_tokens(text)
                        .map(|count| TokenizationOutcome {
                            count,
                            tokenizer_kind: TokenizerKind::SentencePiece,
                            used_fallback: false,
                            fallback_reason: None,
                        })
                        .or_else(|err| Ok(self.fallback_count(text, model, err)))
                }
                TokenizerConfigData::Tiktoken { encoding } => {
                    let engine = match self.tiktoken.engine_for_encoding(&encoding) {
                        Ok(engine) => engine,
                        Err(err) => return Ok(self.fallback_count(text, model, err)),
                    };

                    engine
                        .count_tokens(text)
                        .map(|count| TokenizationOutcome {
                            count,
                            tokenizer_kind: TokenizerKind::Tiktoken,
                            used_fallback: false,
                            fallback_reason: None,
                        })
                        .or_else(|err| Ok(self.fallback_count(text, model, err)))
                }
            },
            Err(err) => Ok(self.fallback_count(text, model, err)),
        }
    }

    pub fn truncate_internal(&self, text: &str, limit: u32, model: &str) -> TruncationOutcome {
        if model.trim().is_empty() {
            return self.fallback_truncate(
                text,
                limit,
                model,
                TokenizerError::UnknownModel("empty model".to_string()),
            );
        }

        if is_gpt_class(model) {
            let engine = match self.tiktoken.engine_for_model(model) {
                Ok(engine) => engine,
                Err(err) => return self.fallback_truncate(text, limit, model, err),
            };

            return match engine.truncate(text, limit) {
                Ok(truncated) => TruncationOutcome {
                    text: truncated,
                    tokenizer_kind: TokenizerKind::Tiktoken,
                    used_fallback: false,
                    fallback_reason: None,
                },
                Err(err) => self.fallback_truncate(text, limit, model, err),
            };
        }

        match self.config_cache.resolve(model) {
            Ok(config) => match config.data {
                TokenizerConfigData::SentencePiece { model_path } => {
                    let tokenizer = match self.sentencepiece_cache.get_or_load(&model_path) {
                        Ok(tokenizer) => tokenizer,
                        Err(err) => return self.fallback_truncate(text, limit, model, err),
                    };

                    match tokenizer.truncate(text, limit) {
                        Ok(truncated) => TruncationOutcome {
                            text: truncated,
                            tokenizer_kind: TokenizerKind::SentencePiece,
                            used_fallback: false,
                            fallback_reason: None,
                        },
                        Err(err) => self.fallback_truncate(text, limit, model, err),
                    }
                }
                TokenizerConfigData::Tiktoken { encoding } => {
                    let engine = match self.tiktoken.engine_for_encoding(&encoding) {
                        Ok(engine) => engine,
                        Err(err) => return self.fallback_truncate(text, limit, model, err),
                    };

                    match engine.truncate(text, limit) {
                        Ok(truncated) => TruncationOutcome {
                            text: truncated,
                            tokenizer_kind: TokenizerKind::Tiktoken,
                            used_fallback: false,
                            fallback_reason: None,
                        },
                        Err(err) => self.fallback_truncate(text, limit, model, err),
                    }
                }
            },
            Err(err) => self.fallback_truncate(text, limit, model, err),
        }
    }

    fn fallback_count(&self, text: &str, model: &str, err: TokenizerError) -> TokenizationOutcome {
        let reason = reason_from_error(&err);
        warn!(
            target: "handshake_core::tokenization",
            model = %model,
            reason = %reason.as_str(),
            error = %err,
            "Falling back to Vibe tokenizer for count"
        );

        TokenizationOutcome {
            count: self.vibe.count_tokens_sync(text),
            tokenizer_kind: TokenizerKind::Vibe,
            used_fallback: true,
            fallback_reason: Some(reason),
        }
    }

    fn fallback_truncate(
        &self,
        text: &str,
        limit: u32,
        model: &str,
        err: TokenizerError,
    ) -> TruncationOutcome {
        let reason = reason_from_error(&err);
        warn!(
            target: "handshake_core::tokenization",
            model = %model,
            reason = %reason.as_str(),
            error = %err,
            "Falling back to Vibe tokenizer for truncate"
        );

        TruncationOutcome {
            text: self.vibe.truncate_sync(text, limit),
            tokenizer_kind: TokenizerKind::Vibe,
            used_fallback: true,
            fallback_reason: Some(reason),
        }
    }
}

impl TokenizationService for TokenizationRouter {
    fn count_tokens(&self, text: &str, model: &str) -> Result<u32, TokenizerError> {
        self.count_tokens_internal(text, model)
            .map(|outcome| outcome.count)
    }

    fn truncate(&self, text: &str, limit: u32, model: &str) -> String {
        self.truncate_internal(text, limit, model).text
    }
}

#[async_trait]
impl Tokenizer for TokenizationRouter {
    async fn count_tokens(&self, text: &str, model: &str) -> Result<u32, TokenizerError> {
        self.count_tokens_internal(text, model)
            .map(|outcome| outcome.count)
    }

    async fn truncate(
        &self,
        text: &str,
        limit: u32,
        model: &str,
    ) -> Result<String, TokenizerError> {
        Ok(self.truncate_internal(text, limit, model).text)
    }
}

pub trait AccuracyWarningEmitter: Send + Sync {
    fn emit_accuracy_warning(
        &self,
        trace_id: Uuid,
        model: &str,
        reason: AccuracyWarningReason,
        tokenizer_kind: TokenizerKind,
    );
}

#[derive(Debug)]
pub struct AsyncFlightRecorderEmitter {
    sender: tokio::sync::mpsc::UnboundedSender<FlightRecorderEvent>,
}

impl AsyncFlightRecorderEmitter {
    pub fn try_new(flight_recorder: Arc<dyn FlightRecorder>) -> Result<Self, TokenizerError> {
        let handle = tokio::runtime::Handle::try_current().map_err(|err| {
            TokenizerError::TokenizerUnavailable(format!("tokio runtime unavailable: {}", err))
        })?;

        let (sender, mut receiver) = tokio::sync::mpsc::unbounded_channel();
        handle.spawn(async move {
            while let Some(event) = receiver.recv().await {
                if let Err(err) = flight_recorder.record_event(event).await {
                    warn!(
                        target: "handshake_core::tokenization",
                        error = %err,
                        "Failed to record accuracy warning event"
                    );
                }
            }
        });

        Ok(Self { sender })
    }
}

impl AccuracyWarningEmitter for AsyncFlightRecorderEmitter {
    fn emit_accuracy_warning(
        &self,
        trace_id: Uuid,
        model: &str,
        reason: AccuracyWarningReason,
        tokenizer_kind: TokenizerKind,
    ) {
        let metric = AccuracyWarningMetric::new(model, tokenizer_kind, reason);
        let payload = match serde_json::to_value(&metric) {
            Ok(value) => value,
            Err(err) => {
                error!(
                    target: "handshake_core::tokenization",
                    error = %err,
                    "Failed to serialize accuracy warning payload"
                );
                json!({
                    "metric_id": "metric.accuracy_warning",
                    "model_id": model,
                    "tokenizer_kind": tokenizer_kind.as_str(),
                    "reason": reason.as_str()
                })
            }
        };

        let event = FlightRecorderEvent::new(
            FlightRecorderEventType::System,
            FlightRecorderActor::System,
            trace_id,
            payload,
        )
        .with_model_id(model);

        if let Err(err) = self.sender.send(event) {
            error!(
                target: "handshake_core::tokenization",
                error = %err,
                "Accuracy warning emission failed"
            );
        }
    }
}

#[derive(Debug)]
pub struct DisabledAccuracyWarningEmitter {
    reason: String,
}

impl DisabledAccuracyWarningEmitter {
    pub fn new(reason: impl Into<String>) -> Self {
        Self {
            reason: reason.into(),
        }
    }
}

impl AccuracyWarningEmitter for DisabledAccuracyWarningEmitter {
    fn emit_accuracy_warning(
        &self,
        trace_id: Uuid,
        model: &str,
        reason: AccuracyWarningReason,
        tokenizer_kind: TokenizerKind,
    ) {
        error!(
            target: "handshake_core::tokenization",
            trace_id = %trace_id,
            model = %model,
            reason = %reason.as_str(),
            tokenizer_kind = %tokenizer_kind.as_str(),
            error = %self.reason,
            "Accuracy warning emitter unavailable"
        );
    }
}

#[derive(Clone)]
pub struct TokenizationWithTrace {
    router: Arc<TokenizationRouter>,
    emitter: Arc<dyn AccuracyWarningEmitter>,
}

impl TokenizationWithTrace {
    pub fn new(router: Arc<TokenizationRouter>, emitter: Arc<dyn AccuracyWarningEmitter>) -> Self {
        Self { router, emitter }
    }

    pub fn count_tokens_with_trace(
        &self,
        text: &str,
        model: &str,
        trace_id: Uuid,
    ) -> Result<u32, TokenizerError> {
        let outcome = self.router.count_tokens_internal(text, model)?;
        if outcome.used_fallback {
            if let Some(reason) = outcome.fallback_reason {
                self.emitter
                    .emit_accuracy_warning(trace_id, model, reason, outcome.tokenizer_kind);
            }
        }
        Ok(outcome.count)
    }

    pub fn truncate_with_trace(
        &self,
        text: &str,
        limit: u32,
        model: &str,
        trace_id: Uuid,
    ) -> String {
        let outcome = self.router.truncate_internal(text, limit, model);
        if outcome.used_fallback {
            if let Some(reason) = outcome.fallback_reason {
                self.emitter
                    .emit_accuracy_warning(trace_id, model, reason, outcome.tokenizer_kind);
            }
        }
        outcome.text
    }
}

#[derive(Debug, Serialize)]
struct AccuracyWarningMetric {
    metric_id: String,
    model_id: String,
    tokenizer_kind: String,
    reason: String,
}

impl AccuracyWarningMetric {
    fn new(model: &str, tokenizer_kind: TokenizerKind, reason: AccuracyWarningReason) -> Self {
        Self {
            metric_id: "metric.accuracy_warning".to_string(),
            model_id: model.to_string(),
            tokenizer_kind: tokenizer_kind.as_str().to_string(),
            reason: reason.as_str().to_string(),
        }
    }
}

pub fn map_ollama_show_to_tokenizer_config(
    value: &Value,
) -> Result<ResolvedTokenizerConfig, TokenizerError> {
    let kind_value = find_string(value, &["tokenizer", "kind"])
        .or_else(|| find_string(value, &["tokenizer", "type"]))
        .or_else(|| find_string(value, &["tokenizer", "name"]))
        .or_else(|| find_string(value, &["tokenizer_config", "kind"]))
        .or_else(|| find_string(value, &["tokenizer_config", "type"]));

    let kind_value = kind_value.ok_or_else(|| {
        TokenizerError::TokenizerConfigMissing("tokenizer kind missing".to_string())
    })?;

    let kind = match kind_value.to_ascii_lowercase().as_str() {
        "sentencepiece" => TokenizerKind::SentencePiece,
        "tiktoken" => TokenizerKind::Tiktoken,
        other => {
            return Err(TokenizerError::TokenizerConfigUnsupported(format!(
                "unsupported tokenizer kind: {}",
                other
            )))
        }
    };

    match kind {
        TokenizerKind::SentencePiece => {
            let model_path = find_string(value, &["tokenizer", "model_path"])
                .or_else(|| find_string(value, &["tokenizer_config", "model_path"]))
                .ok_or_else(|| {
                    TokenizerError::TokenizerConfigMissing(
                        "tokenizer.model_path missing".to_string(),
                    )
                })?;

            Ok(ResolvedTokenizerConfig {
                kind,
                data: TokenizerConfigData::SentencePiece { model_path },
            })
        }
        TokenizerKind::Tiktoken => {
            let encoding = find_string(value, &["tokenizer", "encoding"])
                .or_else(|| find_string(value, &["tokenizer_config", "encoding"]))
                .ok_or_else(|| {
                    TokenizerError::TokenizerConfigMissing("tokenizer.encoding missing".to_string())
                })?;

            Ok(ResolvedTokenizerConfig {
                kind,
                data: TokenizerConfigData::Tiktoken { encoding },
            })
        }
        TokenizerKind::Vibe => Err(TokenizerError::TokenizerConfigUnsupported(
            "vibe tokenizer is not valid in config".to_string(),
        )),
    }
}

fn find_string(value: &Value, path: &[&str]) -> Option<String> {
    let mut current = value;
    for key in path {
        current = current.get(*key)?;
    }
    current.as_str().map(|value| value.to_string())
}

fn reason_from_error(err: &TokenizerError) -> AccuracyWarningReason {
    match err {
        TokenizerError::UnknownModel(_) => AccuracyWarningReason::UnknownModel,
        TokenizerError::TokenizerConfigFetchFailed(_) => AccuracyWarningReason::ConfigFetchFailed,
        TokenizerError::TokenizerConfigMissing(_) => AccuracyWarningReason::ConfigMissing,
        TokenizerError::TokenizerConfigUnsupported(_) => {
            AccuracyWarningReason::UnsupportedTokenizer
        }
        TokenizerError::TokenizerInitFailed(_) => AccuracyWarningReason::TokenizerInitFailed,
        TokenizerError::TokenizerConfigParseFailed(_) => AccuracyWarningReason::TokenizerInitFailed,
        TokenizerError::TokenizerUnavailable(_) => AccuracyWarningReason::TokenizerInitFailed,
        TokenizerError::TokenizationFailed(_) => AccuracyWarningReason::TokenizerInitFailed,
        TokenizerError::DecodeFailed(_) => AccuracyWarningReason::TokenizerInitFailed,
    }
}

fn is_gpt_class(model: &str) -> bool {
    let model = model.to_ascii_lowercase();
    model.starts_with("gpt-") || model.starts_with("o1-") || model.starts_with("o3-")
}

fn vibe_count_tokens(text: &str) -> u32 {
    let char_count = text.chars().count() as f32;
    if char_count == 0.0 {
        0
    } else {
        (char_count / 4.0).ceil() as u32
    }
}

fn vibe_truncate(text: &str, limit: u32) -> String {
    if limit == 0 {
        return String::new();
    }
    let max_chars = limit.saturating_mul(4) as usize;
    text.chars().take(max_chars).collect()
}
