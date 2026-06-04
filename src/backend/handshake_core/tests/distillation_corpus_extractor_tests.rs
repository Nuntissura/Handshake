//! MT-119: Distillation EventLedger replay -> training corpus extractor
//! integration smoke. Detailed pure-logic coverage lives in
//! `distillation::corpus_extractor::tests`; this file pins the
//! cross-crate API surface, the default-deny invariant, and the
//! Spec-Realism Gate Sub-rule 2 real-FlightRecorder composition.

use std::cell::RefCell;
use std::sync::Arc;

use serde_json::json;
use uuid::Uuid;

use handshake_core::distillation::corpus_extractor::{
    CorpusExtractor, EventLedgerSource, ExtractionError, LlmInferenceEvent, LlmInferencePhase,
    SessionMetadataSource,
};
use handshake_core::distillation::flight_recorder_ledger::FlightRecorderEventLedger;
use handshake_core::flight_recorder::{
    duckdb::DuckDbFlightRecorder, FlightRecorder, FlightRecorderActor, FlightRecorderEvent,
    FlightRecorderEventType,
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

/// MT-119 Spec-Realism Gate Sub-rule 2 rework: compose the real
/// production `DuckDbFlightRecorder` (KERNEL-001 EventLedger) with the
/// production `FlightRecorderEventLedger` adapter and the production
/// `CorpusExtractor::extract` pipeline end-to-end. No mock event ledger
/// crosses the trait boundary; the real FlightRecorder is seeded via
/// its `record_event` API and queried via `list_events`, exactly the
/// composition the prior validator flagged as missing.
#[test]
fn corpus_extractor_composes_with_real_flight_recorder() {
    let session_id = "session-mt119-real-fr";
    let trace_id = Uuid::now_v7();
    let model_call_correlation_id = Uuid::now_v7().to_string();

    let recorder: Arc<dyn FlightRecorder> = Arc::new(
        DuckDbFlightRecorder::new_in_memory(7)
            .expect("DuckDbFlightRecorder in-memory construction"),
    );

    let make_event = |phase: &str, ordered_index: u64, payload_extras: serde_json::Value| {
        let mut payload = json!({
            "type": "llm_inference",
            "trace_id": trace_id.to_string(),
            "model_id": "candle-test-model",
            "token_usage": {
                "prompt_tokens": 5_u64,
                "completion_tokens": 2_u64,
                "total_tokens": 7_u64,
            },
            "phase": phase,
            "model_call_correlation_id": model_call_correlation_id,
            "ordered_index": ordered_index,
        });
        if let Some(extras_map) = payload_extras.as_object() {
            let target_map = payload.as_object_mut().expect("payload object");
            for (key, value) in extras_map {
                target_map.insert(key.clone(), value.clone());
            }
        }
        FlightRecorderEvent::new(
            FlightRecorderEventType::LlmInference,
            FlightRecorderActor::System,
            trace_id,
            payload,
        )
        .with_model_session_id(session_id)
        .with_model_id("candle-test-model")
    };

    let start_event = make_event("start", 0, json!({ "prompt": "Translate: hello" }));
    let token_event = make_event("token", 1, json!({ "token_text": "bonjour" }));
    let end_event = make_event("end", 2, json!({ "finish_reason": "stop" }));

    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("tokio runtime");
    let recorder_for_seed = Arc::clone(&recorder);
    runtime.block_on(async move {
        recorder_for_seed
            .record_event(start_event)
            .await
            .expect("record start");
        recorder_for_seed
            .record_event(token_event)
            .await
            .expect("record token");
        recorder_for_seed
            .record_event(end_event)
            .await
            .expect("record end");
    });

    let ledger = FlightRecorderEventLedger::new(Arc::clone(&recorder));
    let metadata = OptIn(vec![session_id.to_string()]);
    let extractor = CorpusExtractor::new(metadata, ledger);

    let corpus = extractor
        .extract(session_id, "Apache-2.0", "2026-05-20T12:00:00Z")
        .expect("extract succeeds against real FlightRecorder");

    assert_eq!(corpus.session_id, session_id);
    assert_eq!(
        corpus.turns.len(),
        1,
        "exactly one assembled turn from the seeded triple"
    );
    let turn = &corpus.turns[0];
    assert_eq!(turn.prompt, "Translate: hello");
    assert_eq!(turn.completion, "bonjour");
    assert_eq!(turn.finish_reason.as_deref(), Some("stop"));
    assert_eq!(turn.license_tag, "Apache-2.0");
    assert_eq!(turn.model_id, "candle-test-model");
    assert_eq!(
        turn.source_event_ids.len(),
        3,
        "source_event_ids should include start + token + end FlightRecorder event_ids"
    );
}

/// MT-119: confirm the adapter respects model_session_id scoping when
/// the real FlightRecorder holds triples for several sessions. Other
/// sessions' events must not leak into the corpus and a session with no
/// events returns an empty corpus rather than an error (opt-in is the
/// gate, not corpus presence).
#[test]
fn corpus_extractor_via_flight_recorder_filters_other_sessions() {
    let target_session = "session-target-real";
    let other_session = "session-other-real";
    let trace_id_target = Uuid::now_v7();
    let trace_id_other = Uuid::now_v7();
    let correlation_target = Uuid::now_v7().to_string();
    let correlation_other = Uuid::now_v7().to_string();

    let recorder: Arc<dyn FlightRecorder> = Arc::new(
        DuckDbFlightRecorder::new_in_memory(7)
            .expect("DuckDbFlightRecorder in-memory construction"),
    );

    let event = |session: &str,
                 trace: Uuid,
                 correlation: &str,
                 phase: &str,
                 ordered: u64,
                 extra: serde_json::Value| {
        let mut payload = json!({
            "type": "llm_inference",
            "trace_id": trace.to_string(),
            "model_id": "candle-test-model",
            "token_usage": {
                "prompt_tokens": 1_u64,
                "completion_tokens": 1_u64,
                "total_tokens": 2_u64,
            },
            "phase": phase,
            "model_call_correlation_id": correlation,
            "ordered_index": ordered,
        });
        if let Some(map) = extra.as_object() {
            let dst = payload.as_object_mut().expect("payload object");
            for (k, v) in map {
                dst.insert(k.clone(), v.clone());
            }
        }
        FlightRecorderEvent::new(
            FlightRecorderEventType::LlmInference,
            FlightRecorderActor::System,
            trace,
            payload,
        )
        .with_model_session_id(session)
        .with_model_id("candle-test-model")
    };

    let target_events = vec![
        event(
            target_session,
            trace_id_target,
            &correlation_target,
            "start",
            0,
            json!({ "prompt": "target prompt" }),
        ),
        event(
            target_session,
            trace_id_target,
            &correlation_target,
            "token",
            1,
            json!({ "token_text": "tgt" }),
        ),
        event(
            target_session,
            trace_id_target,
            &correlation_target,
            "end",
            2,
            json!({ "finish_reason": "stop" }),
        ),
    ];
    let other_events = vec![
        event(
            other_session,
            trace_id_other,
            &correlation_other,
            "start",
            0,
            json!({ "prompt": "other prompt" }),
        ),
        event(
            other_session,
            trace_id_other,
            &correlation_other,
            "token",
            1,
            json!({ "token_text": "oth" }),
        ),
        event(
            other_session,
            trace_id_other,
            &correlation_other,
            "end",
            2,
            json!({ "finish_reason": "stop" }),
        ),
    ];

    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("tokio runtime");
    let recorder_for_seed = Arc::clone(&recorder);
    runtime.block_on(async move {
        for evt in target_events.into_iter().chain(other_events.into_iter()) {
            recorder_for_seed.record_event(evt).await.expect("record");
        }
    });

    let ledger = FlightRecorderEventLedger::new(Arc::clone(&recorder));
    let metadata = OptIn(vec![target_session.to_string()]);
    let extractor = CorpusExtractor::new(metadata, ledger);

    let corpus = extractor
        .extract(target_session, "MIT", "2026-05-20T13:00:00Z")
        .expect("extract succeeds for target session");

    assert_eq!(corpus.session_id, target_session);
    assert_eq!(corpus.turns.len(), 1);
    let turn = &corpus.turns[0];
    assert_eq!(turn.prompt, "target prompt");
    assert_eq!(turn.completion, "tgt");
    // Ensure the other session's payload did not leak.
    assert_ne!(turn.prompt, "other prompt");
    assert_ne!(turn.completion, "oth");
}
