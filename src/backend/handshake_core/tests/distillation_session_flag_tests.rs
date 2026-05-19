//! MT-121: Cross-crate integration smoke for distillation session_flag.
//!
//! Detailed coverage lives in the inline tests in
//! `distillation::session_flag::tests`; this file pins the public API
//! shape, the default-deny posture, and the CorpusExtractor (MT-119)
//! linkage required by MT-121.

use handshake_core::distillation::corpus_extractor::{
    CorpusExtractor, EventLedgerSource, ExtractionError, LlmInferenceEvent, SessionMetadataSource,
};
use handshake_core::distillation::session_flag::{
    get_distill_flag, mark_for_distillation, InMemorySessionFlagStore, SessionFlagError,
};
use std::sync::Arc;

#[test]
fn default_deny_default_session_returns_false() {
    let store = InMemorySessionFlagStore::default();
    assert!(!get_distill_flag(&store, "fresh-session").expect("read"));
}

#[test]
fn mark_with_signature_sets_then_unsets() {
    let store = InMemorySessionFlagStore::default();
    let record = mark_for_distillation(&store, "s", true, "op", "2026-05-20T00:00:00Z")
        .expect("set true");
    assert_eq!(record.previous_flag, false);
    assert_eq!(record.new_flag, true);
    assert!(get_distill_flag(&store, "s").unwrap());

    let record = mark_for_distillation(&store, "s", false, "op", "2026-05-20T00:01:00Z")
        .expect("set false");
    assert_eq!(record.previous_flag, true);
    assert_eq!(record.new_flag, false);
    assert!(!get_distill_flag(&store, "s").unwrap());
}

#[test]
fn mark_without_signature_is_refused_per_hbr_int_006() {
    let store = InMemorySessionFlagStore::default();
    let err = mark_for_distillation(&store, "s", true, " ", "2026-05-20T00:00:00Z")
        .expect_err("empty signature");
    assert!(matches!(err, SessionFlagError::EmptySignature));
    // Default-deny preserved despite the rejected write.
    assert!(!get_distill_flag(&store, "s").unwrap());
}

/// MT-119 gate verification: a CorpusExtractor backed by the
/// SessionFlagStore (via a thin adapter) honours the flag.
struct StoreBackedMetadata {
    store: Arc<InMemorySessionFlagStore>,
}
impl SessionMetadataSource for StoreBackedMetadata {
    fn distill_corpus_opted_in(&self, session_id: &str) -> Result<bool, ExtractionError> {
        get_distill_flag(&*self.store, session_id)
            .map_err(|err| ExtractionError::MetadataLookup(err.to_string()))
    }
}

struct EmptyLedger;
impl EventLedgerSource for EmptyLedger {
    fn llm_inference_events_for_session(
        &self,
        _session_id: &str,
    ) -> Result<Vec<LlmInferenceEvent>, ExtractionError> {
        Ok(vec![])
    }
}

#[test]
fn corpus_extractor_honours_session_flag_via_store_backed_adapter() {
    let store = Arc::new(InMemorySessionFlagStore::default());
    let metadata = StoreBackedMetadata {
        store: store.clone(),
    };
    let extractor = CorpusExtractor::new(metadata, EmptyLedger);

    // Default-deny: extract refuses.
    let err = extractor
        .extract("s", "MIT", "2026-05-20T00:00:00Z")
        .expect_err("default-deny");
    assert!(matches!(err, ExtractionError::NotOptedIn { .. }));

    // After the operator marks the session, extraction succeeds (empty
    // corpus because the ledger is empty - we are gating, not asserting
    // turn content here).
    mark_for_distillation(&*store, "s", true, "op-ilja", "2026-05-20T00:00:01Z")
        .expect("mark");
    let corpus = extractor
        .extract("s", "MIT", "2026-05-20T00:00:02Z")
        .expect("opt-in");
    assert_eq!(corpus.session_id, "s");
    assert!(corpus.turns.is_empty());
}
