use std::sync::Arc;

use serde::{Deserialize, Serialize};

use super::context_bundle::{canonical_json_bytes, sha256_hex};
use super::{KernelError, KernelEvent, KernelEventType, KernelResult};
use crate::storage::Database;

const KERNEL_V1_PROOF_CHAIN: &[KernelEventType] = &[
    KernelEventType::TaskIntentRecorded,
    KernelEventType::SessionQueued,
    KernelEventType::SessionClaimed,
    KernelEventType::SessionStarted,
    KernelEventType::ContextBundleRecorded,
    KernelEventType::ModelAdapterInvoked,
    KernelEventType::ModelResponseRecorded,
    KernelEventType::ToolRequestRecorded,
    KernelEventType::ToolDecisionRecorded,
    KernelEventType::ArtifactProposed,
    KernelEventType::ArtifactStored,
    KernelEventType::ValidationRecorded,
    KernelEventType::PromotionDecided,
    KernelEventType::SessionCompleted,
];

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct TraceProjection {
    pub kernel_task_run_id: String,
    pub session_run_id: String,
    pub authority_source: String,
    pub event_count: usize,
    pub events: Vec<KernelEvent>,
}

impl TraceProjection {
    pub fn from_events(
        kernel_task_run_id: impl Into<String>,
        session_run_id: impl Into<String>,
        events: Vec<KernelEvent>,
    ) -> KernelResult<Self> {
        let kernel_task_run_id = kernel_task_run_id.into();
        let session_run_id = session_run_id.into();
        if events.is_empty() {
            return Err(KernelError::InvalidEvent(
                "trace projection requires EventLedger events",
            ));
        }
        if events.iter().any(|event| {
            event.kernel_task_run_id != kernel_task_run_id || event.session_run_id != session_run_id
        }) {
            return Err(KernelError::InvalidEvent(
                "trace projection events must share run ids",
            ));
        }
        validate_event_sequence(&events)?;
        validate_payload_hashes(&events)?;
        if missing_required_chain_events(&events).is_some() {
            return Err(KernelError::InvalidEvent(
                "incomplete trace: missing Kernel V1 proof chain evidence",
            ));
        }
        validate_causation_chain(&events)?;
        validate_terminal_state(&events)?;
        validate_authority_evidence(&events)?;
        Ok(Self {
            kernel_task_run_id,
            session_run_id,
            authority_source: "postgres_event_ledger".to_string(),
            event_count: events.len(),
            events,
        })
    }

    pub fn contains_event_type(&self, event_type: KernelEventType) -> bool {
        self.events
            .iter()
            .any(|event| event.event_type == event_type)
    }
}

fn validate_event_sequence(events: &[KernelEvent]) -> KernelResult<()> {
    for pair in events.windows(2) {
        if pair[0].event_sequence >= pair[1].event_sequence {
            return Err(KernelError::InvalidEvent(
                "trace projection requires monotonic event_sequence",
            ));
        }
    }
    Ok(())
}

fn validate_payload_hashes(events: &[KernelEvent]) -> KernelResult<()> {
    for event in events {
        let expected = sha256_hex(&canonical_json_bytes(&event.payload));
        if event.payload_hash != expected {
            return Err(KernelError::InvalidEvent(
                "trace projection payload_hash mismatch",
            ));
        }
    }
    Ok(())
}

fn validate_causation_chain(events: &[KernelEvent]) -> KernelResult<()> {
    for pair in events.windows(2) {
        if pair[1].causation_id.as_deref() != Some(pair[0].event_id.as_str()) {
            return Err(KernelError::InvalidEvent(
                "trace projection causation chain is broken",
            ));
        }
    }
    Ok(())
}

fn validate_terminal_state(events: &[KernelEvent]) -> KernelResult<()> {
    let terminal_count = events
        .iter()
        .filter(|event| {
            matches!(
                event.event_type,
                KernelEventType::SessionCompleted
                    | KernelEventType::SessionFailed
                    | KernelEventType::SessionCancelled
                    | KernelEventType::SessionDeadLettered
            )
        })
        .count();
    if terminal_count != 1 {
        return Err(KernelError::InvalidEvent(
            "trace projection requires one terminal session event",
        ));
    }
    if !matches!(
        events.last().map(|event| &event.event_type),
        Some(
            KernelEventType::SessionCompleted
                | KernelEventType::SessionFailed
                | KernelEventType::SessionCancelled
                | KernelEventType::SessionDeadLettered
        )
    ) {
        return Err(KernelError::InvalidEvent(
            "trace projection terminal event must be final",
        ));
    }
    Ok(())
}

fn validate_authority_evidence(events: &[KernelEvent]) -> KernelResult<()> {
    let mut artifact_id: Option<&str> = None;
    let mut artifact_hash: Option<&str> = None;
    let mut validation_id: Option<&str> = None;

    for event in events {
        match event.event_type {
            KernelEventType::ToolDecisionRecorded => {
                require_string(&event.payload, "gate_receipt_kind")?;
                require_string(&event.payload, "args_ref")?;
                require_sha256(&event.payload, "args_hash")?;
                require_string(&event.payload, "result_ref")?;
                require_sha256(&event.payload, "result_hash")?;
            }
            KernelEventType::ArtifactStored => {
                artifact_id = Some(require_string(&event.payload, "artifact_id")?);
                artifact_hash = Some(require_sha256(&event.payload, "content_hash")?);
                require_string(&event.payload, "artifact_manifest_ref")?;
            }
            KernelEventType::ValidationRecorded => {
                validation_id = Some(require_string(&event.payload, "validation_id")?);
                if Some(require_string(&event.payload, "artifact_id")?) != artifact_id {
                    return Err(KernelError::InvalidEvent(
                        "trace projection validation artifact mismatch",
                    ));
                }
                if Some(require_sha256(&event.payload, "content_hash")?) != artifact_hash {
                    return Err(KernelError::InvalidEvent(
                        "trace projection validation content_hash mismatch",
                    ));
                }
                if event
                    .payload
                    .get("artifact_content_hash_validated")
                    .and_then(|value| value.as_bool())
                    != Some(true)
                {
                    return Err(KernelError::InvalidEvent(
                        "trace projection validation requires artifact hash evidence",
                    ));
                }
                if event
                    .payload
                    .get("evidence_refs")
                    .and_then(|value| value.as_array())
                    .is_none_or(|refs| refs.is_empty())
                {
                    return Err(KernelError::InvalidEvent(
                        "trace projection validation requires evidence refs",
                    ));
                }
            }
            KernelEventType::PromotionDecided => {
                if Some(require_string(&event.payload, "artifact_id")?) != artifact_id {
                    return Err(KernelError::InvalidEvent(
                        "trace projection promotion artifact mismatch",
                    ));
                }
                if Some(require_string(&event.payload, "validation_id")?) != validation_id {
                    return Err(KernelError::InvalidEvent(
                        "trace projection promotion validation mismatch",
                    ));
                }
                require_string(&event.payload, "operator_id")?;
                let operator_review =
                    event
                        .payload
                        .get("operator_review")
                        .ok_or(KernelError::InvalidEvent(
                            "trace projection promotion requires operator review",
                        ))?;
                let receipt = require_string(operator_review, "review_receipt_id")?;
                let source = require_string(operator_review, "approval_source")?;
                if receipt.to_ascii_lowercase().contains("fixture")
                    || source != "operator_review_receipt"
                {
                    return Err(KernelError::InvalidEvent(
                        "trace projection promotion requires operator-reviewable evidence",
                    ));
                }
            }
            _ => {}
        }
    }
    Ok(())
}

fn require_string<'a>(payload: &'a serde_json::Value, field: &str) -> KernelResult<&'a str> {
    payload
        .get(field)
        .and_then(|value| value.as_str())
        .filter(|value| !value.trim().is_empty())
        .ok_or(KernelError::InvalidEvent(
            "trace projection missing required evidence field",
        ))
}

fn require_sha256<'a>(payload: &'a serde_json::Value, field: &str) -> KernelResult<&'a str> {
    let value = require_string(payload, field)?;
    if value.len() == 64 && value.chars().all(|character| character.is_ascii_hexdigit()) {
        Ok(value)
    } else {
        Err(KernelError::InvalidEvent(
            "trace projection evidence hash must be sha256",
        ))
    }
}

fn missing_required_chain_events(events: &[KernelEvent]) -> Option<&'static [KernelEventType]> {
    let mut next_required = 0;
    for event in events {
        if KERNEL_V1_PROOF_CHAIN
            .get(next_required)
            .is_some_and(|required| &event.event_type == required)
        {
            next_required += 1;
        }
    }
    (next_required < KERNEL_V1_PROOF_CHAIN.len()).then_some(&KERNEL_V1_PROOF_CHAIN[next_required..])
}

pub struct KernelTraceInspector {
    db: Arc<dyn Database>,
}

impl KernelTraceInspector {
    pub fn new(db: Arc<dyn Database>) -> Self {
        Self { db }
    }

    pub async fn inspect_session(
        &self,
        kernel_task_run_id: &str,
        session_run_id: &str,
    ) -> KernelResult<TraceProjection> {
        let events = self
            .db
            .list_kernel_events_for_session(session_run_id)
            .await?;
        TraceProjection::from_events(kernel_task_run_id, session_run_id, events)
    }
}
