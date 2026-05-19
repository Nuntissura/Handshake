//! MT-119: Distillation EventLedger replay -> training corpus extractor
//! integration smoke. Detailed pure-logic coverage lives in
//! `distillation::corpus_extractor::tests`; this file pins the
//! cross-crate API surface and the default-deny invariant.

use std::cell::RefCell;

use handshake_core::distillation::corpus_extractor::{
    CorpusExtractor, EventLedgerSource, ExtractionError, LlmInferenceEvent, LlmInferencePhase,
    SessionMetadataSource,
};

struct OptIn(Vec<String>);
impl SessionMetadataSource for OptIn {
    fn distill_corpus_opted_in(&self, session_id: &str) -> Result<bool, ExtractionError> {
        Ok(self.0.iter().any(|s| s == session_id))
    }
}

struct OneTriple {
    session_id: String,
    queried: RefCell<usize>,
}
impl EventLedgerSource for OneTriple {
    fn llm_inference_events_for_session(
        &self,
        session_id: &str,
    ) -> Result<Vec<LlmInferenceEvent>, ExtractionError> {
        *self.queried.borrow_mut() += 1;
        if session_id != self.session_id {
            return Ok(vec![]);
        }
        Ok(vec![
            LlmInferenceEvent {
                event_id: "e1".to_string(),
                session_id: session_id.to_string(),
                recorded_at_utc: "2026-05-20T00:00:00Z".to_string(),
                phase: LlmInferencePhase::Start,
                model_id: "model-a".to_string(),
                model_call_correlation_id: "call-a".to_string(),
                prompt: Some("Q: What is 7*8?".to_string()),
                token_text: None,
                finish_reason: None,
                ordered_index: 0,
            },
            LlmInferenceEvent {
                event_id: "e2".to_string(),
                session_id: session_id.to_string(),
                recorded_at_utc: "2026-05-20T00:00:01Z".to_string(),
                phase: LlmInferencePhase::Token,
                model_id: "model-a".to_string(),
                model_call_correlation_id: "call-a".to_string(),
                prompt: None,
                token_text: Some("56".to_string()),
                finish_reason: None,
                ordered_index: 1,
            },
            LlmInferenceEvent {
                event_id: "e3".to_string(),
                session_id: session_id.to_string(),
                recorded_at_utc: "2026-05-20T00:00:02Z".to_string(),
                phase: LlmInferencePhase::End,
                model_id: "model-a".to_string(),
                model_call_correlation_id: "call-a".to_string(),
                prompt: None,
                token_text: None,
                finish_reason: Some("stop".to_string()),
                ordered_index: 2,
            },
        ])
    }
}

#[test]
fn corpus_extractor_default_deny_when_session_not_opted_in() {
    let metadata = OptIn(vec!["other".to_string()]);
    let ledger = OneTriple {
        session_id: "target".to_string(),
        queried: RefCell::new(0),
    };
    let extractor = CorpusExtractor::new(metadata, ledger);

    let err = extractor
        .extract("target", "MIT", "2026-05-20T00:00:03Z")
        .expect_err("default-deny");
    assert!(matches!(err, ExtractionError::NotOptedIn { .. }));
}

#[test]
fn corpus_extractor_returns_assembled_turn_when_opted_in() {
    let metadata = OptIn(vec!["target".to_string()]);
    let ledger = OneTriple {
        session_id: "target".to_string(),
        queried: RefCell::new(0),
    };
    let extractor = CorpusExtractor::new(metadata, ledger);

    let corpus = extractor
        .extract("target", "Permissive", "2026-05-20T00:00:03Z")
        .expect("opt-in succeeds");
    assert_eq!(corpus.session_id, "target");
    assert_eq!(corpus.turns.len(), 1);
    let turn = &corpus.turns[0];
    assert_eq!(turn.prompt, "Q: What is 7*8?");
    assert_eq!(turn.completion, "56");
    assert_eq!(turn.license_tag, "Permissive");
    assert_eq!(turn.finish_reason.as_deref(), Some("stop"));
    assert_eq!(turn.source_event_ids, vec!["e1", "e2", "e3"]);
}
