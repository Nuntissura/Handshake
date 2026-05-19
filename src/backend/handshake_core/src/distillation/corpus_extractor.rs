//! MT-119: Distillation EventLedger replay -> training corpus extractor.
//!
//! Per Master Spec §4.8.1 + AC-DISTILL-OPT-IN + operator decision
//! Q-DISTILL-CORPUS: only sessions flagged `distill_corpus = true` at
//! session close enter the training corpus. Default is opt-out and the
//! flag is non-transitive (one session's opt-in does NOT propagate to
//! parent / child sessions).
//!
//! Adult-production privacy: per Q-DISTILL-CORPUS the explicit opt-in
//! flag IS the privacy gate. The kernel does not infer consent from
//! content; the operator marks the session at close time.
//!
//! Architecture:
//! - [`SessionMetadataSource`] trait abstracts the
//!   `governed_sessions.distill_corpus` Postgres lookup so concrete
//!   storage wiring can land in a follow-on without touching this
//!   extraction logic.
//! - [`EventLedgerSource`] trait abstracts the FlightRecorder
//!   `LlmInference` event query (filtered to `FR-EVT-LLM-INFER-START`,
//!   `FR-EVT-LLM-INFER-TOKEN`, `FR-EVT-LLM-INFER-END` triples).
//! - [`CorpusExtractor::extract`] enforces opt-in default-deny THEN
//!   replays events into [`TrainingTurn`] rows. The Postgres flag
//!   lookup happens before any event read (no in-memory shortcut per
//!   MT-119 red_team minimum_controls).

use thiserror::Error;
use uuid::Uuid;

/// Phase of an LLM inference event triple per spec §4.8.1.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LlmInferencePhase {
    Start,
    Token,
    End,
}

/// Decoded LlmInference flight-recorder event with the fields the
/// corpus extractor needs. The concrete `EventLedgerSource` impl is
/// responsible for translating the on-disk
/// `FlightRecorderEvent { event_type = LlmInference, payload = ... }`
/// into this typed shape; that decoder lives outside this module so the
/// extractor stays I/O-agnostic.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LlmInferenceEvent {
    pub event_id: String,
    pub session_id: String,
    pub recorded_at_utc: String,
    pub phase: LlmInferencePhase,
    pub model_id: String,
    pub model_call_correlation_id: String,
    pub prompt: Option<String>,
    pub token_text: Option<String>,
    pub finish_reason: Option<String>,
    pub ordered_index: u32,
}

/// One assembled training turn (prompt + completion + provenance).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TrainingTurn {
    pub id: String,
    pub session_id: String,
    pub model_id: String,
    pub prompt: String,
    pub completion: String,
    pub finish_reason: Option<String>,
    pub license_tag: String,
    pub source_event_ids: Vec<String>,
    pub sourced_at_utc: String,
}

/// Result of an extraction run.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TrainingCorpus {
    pub session_id: String,
    pub turns: Vec<TrainingTurn>,
}

#[derive(Debug, Error)]
pub enum ExtractionError {
    #[error("session not opted in to distillation corpus extraction (session_id={session_id})")]
    NotOptedIn { session_id: String },
    #[error("session metadata lookup failed: {0}")]
    MetadataLookup(String),
    #[error("event ledger query failed: {0}")]
    EventLedger(String),
    #[error("malformed event triple at correlation_id={correlation_id}: {reason}")]
    MalformedTriple {
        correlation_id: String,
        reason: String,
    },
    #[error("license tag must not be empty (sourced from model artifact + LoRA stack metadata)")]
    EmptyLicenseTag,
}

/// Abstraction over the Postgres `governed_sessions.distill_corpus`
/// lookup. Concrete impls wire to sqlx; mock impls drive the unit
/// tests. Required by MT-119 red_team: "Postgres flag lookup before
/// extraction (no in-memory shortcut)" — the production impl reads the
/// flag at extract time, NOT from a cached in-process value.
pub trait SessionMetadataSource {
    fn distill_corpus_opted_in(&self, session_id: &str) -> Result<bool, ExtractionError>;
}

/// Abstraction over the FlightRecorder `LlmInference` event read. The
/// concrete impl filters by session_id + event_type=LlmInference and
/// returns the decoded triples in recorded_at order.
pub trait EventLedgerSource {
    fn llm_inference_events_for_session(
        &self,
        session_id: &str,
    ) -> Result<Vec<LlmInferenceEvent>, ExtractionError>;
}

/// Pure assembler: groups events by `model_call_correlation_id`, asserts
/// each group is a valid Start -> Token* -> End triple, concatenates
/// token_text fields in `ordered_index` order, and emits one
/// [`TrainingTurn`] per group.
pub fn assemble_turns(
    events: &[LlmInferenceEvent],
    session_id: &str,
    license_tag: &str,
    sourced_at_utc: &str,
) -> Result<Vec<TrainingTurn>, ExtractionError> {
    use std::collections::BTreeMap;
    if license_tag.trim().is_empty() {
        return Err(ExtractionError::EmptyLicenseTag);
    }

    let mut groups: BTreeMap<String, Vec<&LlmInferenceEvent>> = BTreeMap::new();
    for event in events {
        groups
            .entry(event.model_call_correlation_id.clone())
            .or_default()
            .push(event);
    }

    let mut turns = Vec::with_capacity(groups.len());
    for (correlation_id, mut group) in groups {
        group.sort_by_key(|e| e.ordered_index);
        let start = group.iter().find(|e| e.phase == LlmInferencePhase::Start);
        let end = group.iter().find(|e| e.phase == LlmInferencePhase::End);
        let tokens: Vec<&LlmInferenceEvent> = group
            .iter()
            .copied()
            .filter(|e| e.phase == LlmInferencePhase::Token)
            .collect();

        let start = start.ok_or_else(|| ExtractionError::MalformedTriple {
            correlation_id: correlation_id.clone(),
            reason: "missing FR-EVT-LLM-INFER-START phase".to_string(),
        })?;
        let end = end.ok_or_else(|| ExtractionError::MalformedTriple {
            correlation_id: correlation_id.clone(),
            reason: "missing FR-EVT-LLM-INFER-END phase".to_string(),
        })?;
        if tokens.is_empty() {
            return Err(ExtractionError::MalformedTriple {
                correlation_id: correlation_id.clone(),
                reason: "no FR-EVT-LLM-INFER-TOKEN phase between START and END".to_string(),
            });
        }
        let prompt = start.prompt.clone().ok_or_else(|| ExtractionError::MalformedTriple {
            correlation_id: correlation_id.clone(),
            reason: "FR-EVT-LLM-INFER-START missing prompt payload".to_string(),
        })?;
        let completion = tokens
            .iter()
            .map(|t| t.token_text.clone().unwrap_or_default())
            .collect::<String>();

        let model_id = start.model_id.clone();
        let mut source_event_ids = vec![start.event_id.clone()];
        for token in &tokens {
            source_event_ids.push(token.event_id.clone());
        }
        source_event_ids.push(end.event_id.clone());

        turns.push(TrainingTurn {
            id: Uuid::now_v7().to_string(),
            session_id: session_id.to_string(),
            model_id,
            prompt,
            completion,
            finish_reason: end.finish_reason.clone(),
            license_tag: license_tag.to_string(),
            source_event_ids,
            sourced_at_utc: sourced_at_utc.to_string(),
        });
    }
    Ok(turns)
}

/// The extractor wires a [`SessionMetadataSource`] and an
/// [`EventLedgerSource`] together. The constructor takes ownership of
/// both so tests can inject mocks and production can wire real
/// Postgres + FlightRecorder.
pub struct CorpusExtractor<S: SessionMetadataSource, L: EventLedgerSource> {
    metadata: S,
    ledger: L,
}

impl<S: SessionMetadataSource, L: EventLedgerSource> CorpusExtractor<S, L> {
    pub fn new(metadata: S, ledger: L) -> Self {
        Self { metadata, ledger }
    }

    /// Extract the training corpus for `session_id`. Default-deny on the
    /// opt-in flag (Postgres lookup happens BEFORE any event read).
    /// `license_tag` is inherited from the model artifact + active LoRA
    /// stack and is stamped on every turn for downstream Skill Bank
    /// license discipline.
    pub fn extract(
        &self,
        session_id: &str,
        license_tag: &str,
        sourced_at_utc: &str,
    ) -> Result<TrainingCorpus, ExtractionError> {
        let opted_in = self.metadata.distill_corpus_opted_in(session_id)?;
        if !opted_in {
            return Err(ExtractionError::NotOptedIn {
                session_id: session_id.to_string(),
            });
        }
        let events = self.ledger.llm_inference_events_for_session(session_id)?;
        let turns = assemble_turns(&events, session_id, license_tag, sourced_at_utc)?;
        Ok(TrainingCorpus {
            session_id: session_id.to_string(),
            turns,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;

    struct MockMetadata {
        opted_in_sessions: Vec<String>,
        lookup_calls: RefCell<Vec<String>>,
    }
    impl SessionMetadataSource for MockMetadata {
        fn distill_corpus_opted_in(&self, session_id: &str) -> Result<bool, ExtractionError> {
            self.lookup_calls.borrow_mut().push(session_id.to_string());
            Ok(self.opted_in_sessions.iter().any(|s| s == session_id))
        }
    }

    struct MockLedger {
        events: Vec<LlmInferenceEvent>,
        query_calls: RefCell<Vec<String>>,
    }
    impl EventLedgerSource for MockLedger {
        fn llm_inference_events_for_session(
            &self,
            session_id: &str,
        ) -> Result<Vec<LlmInferenceEvent>, ExtractionError> {
            self.query_calls.borrow_mut().push(session_id.to_string());
            Ok(self
                .events
                .iter()
                .filter(|e| e.session_id == session_id)
                .cloned()
                .collect())
        }
    }

    fn evt(
        event_id: &str,
        session_id: &str,
        correlation_id: &str,
        phase: LlmInferencePhase,
        idx: u32,
        prompt: Option<&str>,
        token_text: Option<&str>,
        finish: Option<&str>,
    ) -> LlmInferenceEvent {
        LlmInferenceEvent {
            event_id: event_id.to_string(),
            session_id: session_id.to_string(),
            recorded_at_utc: "2026-05-20T00:00:00Z".to_string(),
            phase,
            model_id: "019a-model".to_string(),
            model_call_correlation_id: correlation_id.to_string(),
            prompt: prompt.map(String::from),
            token_text: token_text.map(String::from),
            finish_reason: finish.map(String::from),
            ordered_index: idx,
        }
    }

    #[test]
    fn extract_refuses_non_opted_in_session_with_default_deny() {
        let session = "session-not-opted-in";
        let metadata = MockMetadata {
            opted_in_sessions: vec!["other-session".to_string()],
            lookup_calls: RefCell::new(Vec::new()),
        };
        let ledger = MockLedger {
            events: Vec::new(),
            query_calls: RefCell::new(Vec::new()),
        };

        let extractor = CorpusExtractor::new(metadata, ledger);
        let err = extractor
            .extract(session, "MIT", "2026-05-20T00:00:00Z")
            .expect_err("default-deny");

        match err {
            ExtractionError::NotOptedIn { session_id } => {
                assert_eq!(session_id, session);
            }
            other => panic!("expected NotOptedIn, got {other:?}"),
        }
        // Metadata lookup happened (Postgres flag lookup BEFORE event read).
        assert_eq!(extractor.metadata.lookup_calls.borrow().len(), 1);
        // Event ledger NOT queried because metadata refused.
        assert!(extractor.ledger.query_calls.borrow().is_empty());
    }

    #[test]
    fn extract_assembles_turns_from_event_triples_when_opted_in() {
        let session = "session-opted-in";
        let events = vec![
            evt(
                "e1",
                session,
                "call-1",
                LlmInferencePhase::Start,
                0,
                Some("user prompt 1"),
                None,
                None,
            ),
            evt(
                "e2",
                session,
                "call-1",
                LlmInferencePhase::Token,
                1,
                None,
                Some("hello "),
                None,
            ),
            evt(
                "e3",
                session,
                "call-1",
                LlmInferencePhase::Token,
                2,
                None,
                Some("world"),
                None,
            ),
            evt(
                "e4",
                session,
                "call-1",
                LlmInferencePhase::End,
                3,
                None,
                None,
                Some("stop"),
            ),
            // Second call triple
            evt(
                "e5",
                session,
                "call-2",
                LlmInferencePhase::Start,
                0,
                Some("user prompt 2"),
                None,
                None,
            ),
            evt(
                "e6",
                session,
                "call-2",
                LlmInferencePhase::Token,
                1,
                None,
                Some("answer"),
                None,
            ),
            evt(
                "e7",
                session,
                "call-2",
                LlmInferencePhase::End,
                2,
                None,
                None,
                Some("length"),
            ),
        ];

        let metadata = MockMetadata {
            opted_in_sessions: vec![session.to_string()],
            lookup_calls: RefCell::new(Vec::new()),
        };
        let ledger = MockLedger {
            events,
            query_calls: RefCell::new(Vec::new()),
        };
        let extractor = CorpusExtractor::new(metadata, ledger);

        let corpus = extractor
            .extract(session, "MIT", "2026-05-20T00:00:00Z")
            .expect("extract succeeds");

        assert_eq!(corpus.session_id, session);
        assert_eq!(corpus.turns.len(), 2);
        let mut by_prompt: Vec<&TrainingTurn> = corpus.turns.iter().collect();
        by_prompt.sort_by_key(|t| t.prompt.clone());
        assert_eq!(by_prompt[0].prompt, "user prompt 1");
        assert_eq!(by_prompt[0].completion, "hello world");
        assert_eq!(by_prompt[0].finish_reason.as_deref(), Some("stop"));
        assert_eq!(by_prompt[0].license_tag, "MIT");
        assert_eq!(by_prompt[1].prompt, "user prompt 2");
        assert_eq!(by_prompt[1].completion, "answer");
        assert_eq!(by_prompt[1].finish_reason.as_deref(), Some("length"));
        // source_event_ids: start + token(s) + end. Turn 1 has 4 events,
        // turn 2 has 3 events.
        assert_eq!(by_prompt[0].source_event_ids.len(), 4);
        assert_eq!(by_prompt[1].source_event_ids.len(), 3);
    }

    #[test]
    fn assemble_rejects_malformed_triples() {
        // Missing END phase.
        let events = vec![
            evt(
                "e1",
                "s",
                "call-a",
                LlmInferencePhase::Start,
                0,
                Some("prompt"),
                None,
                None,
            ),
            evt(
                "e2",
                "s",
                "call-a",
                LlmInferencePhase::Token,
                1,
                None,
                Some("tok"),
                None,
            ),
        ];
        let err = assemble_turns(&events, "s", "MIT", "2026-05-20T00:00:00Z")
            .expect_err("missing END");
        assert!(matches!(
            err,
            ExtractionError::MalformedTriple { ref reason, .. } if reason.contains("END")
        ));

        // Missing TOKEN phase between START and END.
        let events = vec![
            evt(
                "e1",
                "s",
                "call-b",
                LlmInferencePhase::Start,
                0,
                Some("prompt"),
                None,
                None,
            ),
            evt(
                "e2",
                "s",
                "call-b",
                LlmInferencePhase::End,
                1,
                None,
                None,
                Some("stop"),
            ),
        ];
        let err = assemble_turns(&events, "s", "MIT", "2026-05-20T00:00:00Z")
            .expect_err("no token");
        assert!(matches!(
            err,
            ExtractionError::MalformedTriple { ref reason, .. } if reason.contains("TOKEN")
        ));
    }

    #[test]
    fn assemble_rejects_empty_license_tag() {
        let err = assemble_turns(&[], "s", "  ", "2026-05-20T00:00:00Z").expect_err("empty license");
        assert!(matches!(err, ExtractionError::EmptyLicenseTag));
    }
}
