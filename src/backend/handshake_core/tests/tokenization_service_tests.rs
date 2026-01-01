use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use serde_json::{json, Value};
use uuid::Uuid;

use handshake_core::tokenization::{
    map_ollama_show_to_tokenizer_config, AccuracyWarningEmitter, AccuracyWarningReason,
    OllamaTokenizerConfigCache, SentencePieceTokenizerCache, TiktokenAdapter, TokenizationRouter,
    TokenizationWithTrace, TokenizerConfigData, TokenizerConfigFetcher, TokenizerError,
    TokenizerKind, VibeTokenizer,
};

#[derive(Clone)]
struct TestConfigFetcher {
    result: Arc<Mutex<Result<Value, TokenizerError>>>,
}

impl TestConfigFetcher {
    fn ok(value: Value) -> Self {
        Self {
            result: Arc::new(Mutex::new(Ok(value))),
        }
    }
}

#[async_trait]
impl TokenizerConfigFetcher for TestConfigFetcher {
    async fn fetch_show(&self, _model: &str) -> Result<Value, TokenizerError> {
        self.result.lock().expect("lock test config").clone()
    }
}

#[derive(Debug, Clone)]
struct WarningRecord {
    trace_id: Uuid,
    reason: AccuracyWarningReason,
    tokenizer_kind: TokenizerKind,
}

#[derive(Default)]
struct TestAccuracyWarningEmitter {
    warnings: Mutex<Vec<WarningRecord>>,
}

impl TestAccuracyWarningEmitter {
    fn warnings(&self) -> Vec<WarningRecord> {
        self.warnings.lock().expect("lock warnings").clone()
    }
}

impl AccuracyWarningEmitter for TestAccuracyWarningEmitter {
    fn emit_accuracy_warning(
        &self,
        trace_id: Uuid,
        _model: &str,
        reason: AccuracyWarningReason,
        tokenizer_kind: TokenizerKind,
    ) {
        let mut guard = self.warnings.lock().expect("lock warnings");
        guard.push(WarningRecord {
            trace_id,
            reason,
            tokenizer_kind,
        });
    }
}

#[test]
fn map_ollama_show_sentencepiece_config() {
    let value = json!({
        "tokenizer": {
            "kind": "sentencepiece",
            "model_path": "/tmp/model.json"
        }
    });

    let config = map_ollama_show_to_tokenizer_config(&value).expect("config parsed");
    assert_eq!(config.kind, TokenizerKind::SentencePiece);
    match config.data {
        TokenizerConfigData::SentencePiece { model_path } => {
            assert_eq!(model_path, "/tmp/model.json");
        }
        _ => panic!("expected sentencepiece config"),
    }
}

#[test]
fn map_ollama_show_tiktoken_config() {
    let value = json!({
        "tokenizer_config": {
            "type": "tiktoken",
            "encoding": "cl100k_base"
        }
    });

    let config = map_ollama_show_to_tokenizer_config(&value).expect("config parsed");
    assert_eq!(config.kind, TokenizerKind::Tiktoken);
    match config.data {
        TokenizerConfigData::Tiktoken { encoding } => {
            assert_eq!(encoding, "cl100k_base");
        }
        _ => panic!("expected tiktoken config"),
    }
}

#[tokio::test]
async fn tokenization_emits_warning_on_fallback() {
    let router = Arc::new(TokenizationRouter::new(
        Arc::new(TiktokenAdapter::default()),
        Arc::new(VibeTokenizer),
    ));
    let emitter = Arc::new(TestAccuracyWarningEmitter::default());
    let tokenization = TokenizationWithTrace::new(router, emitter.clone());

    let trace_id = Uuid::new_v4();
    let count = tokenization
        .count_tokens_with_trace("fallback text", "unknown-model", trace_id)
        .expect("count tokens");
    assert!(count > 0);

    let warnings = emitter.warnings();
    assert_eq!(warnings.len(), 1);
    assert_eq!(warnings[0].trace_id, trace_id);
    assert_eq!(warnings[0].reason, AccuracyWarningReason::ConfigMissing);
    assert_eq!(warnings[0].tokenizer_kind, TokenizerKind::Vibe);
}

#[tokio::test]
async fn tokenization_uses_ollama_tiktoken_config_without_warning() {
    let fetcher = Arc::new(TestConfigFetcher::ok(json!({
        "tokenizer_config": {
            "kind": "tiktoken",
            "encoding": "cl100k_base"
        }
    })));
    let cache = Arc::new(OllamaTokenizerConfigCache::new(fetcher));
    cache.refresh("llama3").await.expect("refresh config");

    let router = Arc::new(TokenizationRouter::new_with_ollama_config(
        Arc::new(TiktokenAdapter::default()),
        Arc::new(VibeTokenizer),
        SentencePieceTokenizerCache::default(),
        cache,
    ));
    let emitter = Arc::new(TestAccuracyWarningEmitter::default());
    let tokenization = TokenizationWithTrace::new(router, emitter.clone());

    let trace_id = Uuid::new_v4();
    let count = tokenization
        .count_tokens_with_trace("hello world", "llama3", trace_id)
        .expect("count tokens");
    assert!(count > 0);
    assert!(emitter.warnings().is_empty());
}
