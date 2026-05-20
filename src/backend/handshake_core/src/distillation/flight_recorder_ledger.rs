//! MT-119 rework (Spec-Realism Gate Sub-rule 2): production
//! `EventLedgerSource` impl backed by the live FlightRecorder from
//! KERNEL-001.
//!
//! The corpus extractor describes the FlightRecorder triple (FR-EVT-
//! LLM-INFER-START / TOKEN / END) as three rows of the existing
//! `FlightRecorderEventType::LlmInference` type. The legacy FR-EVT-002
//! validator only requires `type` / `trace_id` / `model_id` /
//! `token_usage`; distillation-relevant fields (`phase`, `prompt`,
//! `token_text`, `finish_reason`, `model_call_correlation_id`,
//! `ordered_index`) ride on the same payload as additive extensions, so
//! seeding distillation triples via the production
//! `FlightRecorder::record_event` path is schema-safe and survives the
//! existing payload validator without migration.
//!
//! Tests in `tests/distillation_corpus_extractor_tests.rs` compose this
//! adapter against a real `DuckDbFlightRecorder` (`new_in_memory(7)`)
//! end-to-end through `CorpusExtractor::extract` to discharge the
//! Sub-rule 2 obligation: the adversarial / validator round on MT-119
//! flagged the prior in-memory `MockLedger` / `OneTriple` test stubs as
//! the trait-plus-mock anti-pattern and required a real-FlightRecorder
//! composition test analogous to MT-143's
//! `capsule_builder_composes_with_real_fems_mt_handoff_retriever` test.

use std::sync::Arc;

use crate::flight_recorder::{EventFilter, FlightRecorder, FlightRecorderEventType};

use super::corpus_extractor::{
    EventLedgerSource, ExtractionError, LlmInferenceEvent, LlmInferencePhase,
};

/// Production `EventLedgerSource` implementation backed by the real
/// FlightRecorder.
///
/// Queries FlightRecorder events scoped to a `model_session_id`,
/// filters to `event_type = LlmInference`, and decodes the three-phase
/// distillation payload extension into the corpus_extractor typed event
/// shape.
pub struct FlightRecorderEventLedger {
    recorder: Arc<dyn FlightRecorder>,
}

impl FlightRecorderEventLedger {
    pub fn new(recorder: Arc<dyn FlightRecorder>) -> Self {
        Self { recorder }
    }
}

impl EventLedgerSource for FlightRecorderEventLedger {
    fn llm_inference_events_for_session(
        &self,
        session_id: &str,
    ) -> Result<Vec<LlmInferenceEvent>, ExtractionError> {
        let filter = EventFilter {
            model_session_id: Some(session_id.to_string()),
            ..EventFilter::default()
        };

        let recorder = Arc::clone(&self.recorder);
        let raw_events = match tokio::runtime::Handle::try_current() {
            Ok(handle) => tokio::task::block_in_place(|| {
                handle.block_on(async move { recorder.list_events(filter).await })
            }),
            Err(_) => {
                let runtime = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .map_err(|e| {
                        ExtractionError::EventLedger(format!(
                            "tokio current-thread runtime unavailable: {e}"
                        ))
                    })?;
                runtime.block_on(async move { recorder.list_events(filter).await })
            }
        }
        .map_err(|e| ExtractionError::EventLedger(e.to_string()))?;

        let mut decoded = Vec::with_capacity(raw_events.len());
        for raw in raw_events {
            if raw.event_type != FlightRecorderEventType::LlmInference {
                continue;
            }
            let event_session = match raw.model_session_id.as_deref() {
                Some(value) => value.to_string(),
                None => continue,
            };
            if event_session != session_id {
                continue;
            }
            let phase = match raw.payload.get("phase").and_then(|v| v.as_str()) {
                Some("start") => LlmInferencePhase::Start,
                Some("token") => LlmInferencePhase::Token,
                Some("end") => LlmInferencePhase::End,
                Some(other) => {
                    return Err(ExtractionError::EventLedger(format!(
                        "unrecognized llm_inference phase \"{other}\" in event {}",
                        raw.event_id
                    )));
                }
                None => {
                    // Legacy FR-EVT-002 single-row llm_inference (stats
                    // only); not part of the distillation triple
                    // taxonomy. Skip silently so the extractor remains
                    // forward-compatible with mixed event populations.
                    continue;
                }
            };

            let model_id = raw
                .model_id
                .clone()
                .or_else(|| {
                    raw.payload
                        .get("model_id")
                        .and_then(|v| v.as_str())
                        .map(String::from)
                })
                .ok_or_else(|| {
                    ExtractionError::EventLedger(format!(
                        "FlightRecorder llm_inference event {} missing model_id",
                        raw.event_id
                    ))
                })?;

            let model_call_correlation_id = raw
                .payload
                .get("model_call_correlation_id")
                .and_then(|v| v.as_str())
                .map(String::from)
                .unwrap_or_else(|| raw.trace_id.to_string());

            let ordered_index = raw
                .payload
                .get("ordered_index")
                .and_then(|v| v.as_u64())
                .ok_or_else(|| {
                    ExtractionError::EventLedger(format!(
                        "llm_inference event {} missing ordered_index",
                        raw.event_id
                    ))
                })? as u32;

            let prompt = raw
                .payload
                .get("prompt")
                .and_then(|v| v.as_str())
                .map(String::from);
            let token_text = raw
                .payload
                .get("token_text")
                .and_then(|v| v.as_str())
                .map(String::from);
            let finish_reason = raw
                .payload
                .get("finish_reason")
                .and_then(|v| v.as_str())
                .map(String::from);

            decoded.push(LlmInferenceEvent {
                event_id: raw.event_id.to_string(),
                session_id: event_session,
                recorded_at_utc: raw.timestamp.to_rfc3339(),
                phase,
                model_id,
                model_call_correlation_id,
                prompt,
                token_text,
                finish_reason,
                ordered_index,
            });
        }
        Ok(decoded)
    }
}
