use std::sync::Arc;

use handshake_core::tokenization::{TiktokenAdapter, TokenizationRouter, Tokenizer, VibeTokenizer};

#[tokio::test]
async fn tiktoken_counts_gpt4o() {
    let adapter = TiktokenAdapter::default();
    let count = adapter.count_tokens("hello world", "gpt-4o").await.unwrap();
    assert!(count > 0);
}

#[tokio::test]
async fn tiktoken_falls_back_to_cl100k_base() {
    let adapter = TiktokenAdapter::default();
    let count = adapter
        .count_tokens("fallback path should not panic", "unknown-model")
        .await
        .unwrap();
    assert!(count > 0);
}

#[tokio::test]
async fn router_uses_fallback_for_unknown_models() {
    let router = TokenizationRouter::new(
        Arc::new(TiktokenAdapter::default()),
        Arc::new(VibeTokenizer),
    );
    let count = router
        .count_tokens("fallback routing", "not-gpt-model")
        .await
        .unwrap();
    assert!(count > 0);
}

#[tokio::test]
async fn truncate_respects_limit_without_panic() {
    let adapter = TiktokenAdapter::default();
    let truncated = adapter
        .truncate(
            "This is a longer sentence that should be truncated safely",
            8,
            "gpt-4o",
        )
        .await
        .unwrap();
    assert!(!truncated.is_empty());
    // Rough upper bound check: 8 tokens ~ 32 chars.
    assert!(truncated.len() <= 64);
}

#[tokio::test]
async fn vibe_handles_unknown_model_consistently() {
    let vibe = VibeTokenizer;
    let count = vibe
        .count_tokens("rough estimate", "unknown")
        .await
        .unwrap();
    let truncated = vibe.truncate("rough estimate", 1, "unknown").await.unwrap();
    assert!(count > 0);
    assert!(!truncated.is_empty());
}
