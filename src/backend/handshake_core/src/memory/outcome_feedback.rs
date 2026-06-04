//! MT-158: FEMS outcome feedback loop.
//!
//! After a MemoryCapsule is used in a model call, attach the downstream
//! outcome and tune per-item scores. Pure arithmetic; no LLM call.

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use thiserror::Error;
use uuid::Uuid;

use crate::kernel::{
    action_envelope::{
        ApprovalPosture, AuthorityEffect, ExpectedWriteBoxRef, KernelActionRequestV1,
        KernelActorRef, KernelSessionRef, KernelTargetRef, ValidationRequirement,
    },
    context_bundle::{canonical_json_bytes, sha256_hex},
    write_boxes::{
        MemoryBox, WriteBoxCommon, WriteBoxKind, WriteBoxLifecycleState, WriteBoxOwnerRef,
        WriteBoxPayloadRef, WriteBoxReplayMetadataV1, WriteBoxTargetRef, WriteBoxValidationState,
        WriteBoxValidationStatus, validate_write_box_common,
    },
};

use super::persistence::{
    KERNEL_ACTION_REQUEST_SCHEMA_ID, KernelActionSubmission, MEMORY_WRITE_BOX_SCHEMA_ID,
    RecordReceipt, WRITE_BOX_V1_ENVELOPE_SCHEMA_ID, WriteBoxV1Envelope,
};

pub const OUTCOME_ATTACH_INPUT_SCHEMA_ID: &str = "hsk.kernel.memory_capsule_outcome_input@1";
pub const OUTCOME_ATTACH_PAYLOAD_SCHEMA_ID: &str = "hsk.memory_capsule.outcome_payload@1";
pub const OUTCOME_ATTACH_ACTION_ID: &str = "kernel.memory_capsule.attach_outcome";
pub const OUTCOME_ATTACH_ACTOR_ID: &str = "memory_outcome_feedback_loop";
pub const OUTCOME_ATTACH_SESSION_ID: &str = "memory-outcome-feedback";

/// Outcome class attached to a capsule. Mirror of the
/// [`crate::memory::persistence::CapsuleOutcome`] surface but lifted into
/// the feedback-loop shape with the additional `Escalation` variant per
/// MT-158.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "outcome")]
pub enum CapsuleOutcome {
    Pass {
        mt_id: String,
        validator_verdict_id: Uuid,
    },
    Fail {
        mt_id: String,
        validator_verdict_id: Uuid,
        failure_class: FailureClass,
    },
    Escalation {
        mt_id: String,
        escalation_reason: String,
    },
    Skipped {
        reason: String,
    },
}

impl CapsuleOutcome {
    pub fn kind(&self) -> &'static str {
        match self {
            Self::Pass { .. } => "pass",
            Self::Fail { .. } => "fail",
            Self::Escalation { .. } => "escalation",
            Self::Skipped { .. } => "skipped",
        }
    }
}

/// Classification of failure. Closed enum so new classes require a typed
/// code change.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FailureClass {
    ValidatorRejected,
    ContractViolation,
    TimeoutBudgetExceeded,
    UpstreamError,
    Other,
}

/// Tuning knobs for the outcome scoring tuner. Per MT-158 these are
/// durable, bounded, and conservative so a single bad outcome cannot
/// collapse a memory's score.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct TuningParams {
    pub pass_boost: f64,
    pub fail_penalty: f64,
    pub escalation_penalty: f64,
    pub per_item_decay_per_use: f64,
    /// Maximum absolute score change applied per call. Prevents runaway.
    pub max_abs_change_per_call: f64,
}

impl Default for TuningParams {
    fn default() -> Self {
        Self {
            pass_boost: 0.05,
            fail_penalty: 0.10,
            escalation_penalty: 0.15,
            per_item_decay_per_use: 0.01,
            max_abs_change_per_call: 0.20,
        }
    }
}

/// Identifier shape required by the tuner — only memory_id + pinned flag.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemoryPackItemRef {
    pub memory_id: Uuid,
    pub pinned: bool,
}

/// Pure tuner: apply a per-item score adjustment based on the outcome.
/// Pinned items are skipped per MT-159 (operator-locked memories must
/// survive transient bad outcomes).
#[derive(Debug, Clone, Copy, Default)]
pub struct OutcomeScoringTuner;

impl OutcomeScoringTuner {
    pub fn apply_outcome(
        item_scores: &mut HashMap<Uuid, f64>,
        outcome: &CapsuleOutcome,
        pack_items: &[MemoryPackItemRef],
        tuning: &TuningParams,
    ) {
        let outcome_delta = match outcome {
            CapsuleOutcome::Pass { .. } => tuning.pass_boost,
            CapsuleOutcome::Fail { .. } => -tuning.fail_penalty,
            CapsuleOutcome::Escalation { .. } => -tuning.escalation_penalty,
            CapsuleOutcome::Skipped { .. } => return, // no change
        };
        // Decay per use (small) applied to all included items so even
        // accepted items slowly decay if they are reused without renewal.
        let raw_delta = outcome_delta - tuning.per_item_decay_per_use;
        // Per-call cap covers both outcome adjustment and decay so even bad
        // parameter overrides cannot collapse a score in a single call.
        let bounded = if raw_delta >= 0.0 {
            raw_delta.min(tuning.max_abs_change_per_call)
        } else {
            raw_delta.max(-tuning.max_abs_change_per_call)
        };
        for item in pack_items {
            if item.pinned {
                continue;
            }
            let entry = item_scores.entry(item.memory_id).or_insert(0.5);
            *entry += bounded;
            // Clamp to [0, 1] so the score remains a probability-like
            // signal.
            if *entry < 0.0 {
                *entry = 0.0;
            } else if *entry > 1.0 {
                *entry = 1.0;
            }
        }
    }
}

/// Receipt returned by [`OutcomeFeedbackLoop::record_outcome`]. Captures
/// the attach action id for audit.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OutcomeReceipt {
    pub receipt_id: Uuid,
    pub capsule_id: Uuid,
    pub action_id: String,
    pub recorded_at_utc: DateTime<Utc>,
}

/// Outcome attribution record persisted via KernelActionCatalogV1.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OutcomeAttribution {
    pub capsule_id: Uuid,
    pub outcome: CapsuleOutcome,
    pub attached_at_utc: DateTime<Utc>,
}

/// Submitter trait for outcome attach actions through the kernel action
/// catalog. Production wires to KernelActionCatalogV1; tests use mocks.
pub trait OutcomeAttachSubmitter {
    fn attach_outcome(
        &self,
        attribution: OutcomeAttribution,
    ) -> Result<OutcomeReceipt, OutcomeError>;
}

/// OutcomeFeedbackLoop entry point.
pub struct OutcomeFeedbackLoop<'a> {
    pub action_catalog: &'a dyn OutcomeAttachSubmitter,
}

impl<'a> OutcomeFeedbackLoop<'a> {
    pub fn new(action_catalog: &'a dyn OutcomeAttachSubmitter) -> Self {
        Self { action_catalog }
    }

    /// Record outcome attribution against a capsule and tune the scores for
    /// the exact pack that produced the outcome. Tuning runs only after a
    /// successful attach so failed audit writes cannot silently mutate score
    /// state.
    pub fn record_outcome(
        &self,
        capsule_id: Uuid,
        outcome: CapsuleOutcome,
        item_scores: &mut HashMap<Uuid, f64>,
        pack_items: &[MemoryPackItemRef],
        tuning: &TuningParams,
    ) -> Result<OutcomeReceipt, OutcomeError> {
        let attribution = OutcomeAttribution {
            capsule_id,
            outcome: outcome.clone(),
            attached_at_utc: Utc::now(),
        };
        let receipt = self.action_catalog.attach_outcome(attribution)?;
        OutcomeScoringTuner::apply_outcome(item_scores, &outcome, pack_items, tuning);
        Ok(receipt)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Error, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum OutcomeError {
    #[error("outcome attach rejected: {code}: {reason}")]
    Rejected { code: String, reason: String },
    #[error("outcome attach serialization failed: {0}")]
    Serialization(String),
    #[error("invalid outcome attach shape {field}: {message}")]
    InvalidShape {
        field: &'static str,
        message: String,
    },
}

pub(crate) fn outcome_attach_submission(
    attribution: &OutcomeAttribution,
    receipt: &OutcomeReceipt,
) -> Result<KernelActionSubmission, OutcomeError> {
    let payload = outcome_payload_value(attribution, receipt)?;
    let payload_sha256 = sha256_hex(&canonical_json_bytes(&payload));
    let write_box = outcome_write_box(attribution, receipt, &payload_sha256);
    validate_write_box_common(&write_box.common).map_err(|errors| OutcomeError::InvalidShape {
        field: "write_box",
        message: format!("{errors:?}"),
    })?;

    Ok(KernelActionSubmission {
        request: outcome_action_request(attribution, receipt),
        write_box_envelope: WriteBoxV1Envelope {
            schema_id: WRITE_BOX_V1_ENVELOPE_SCHEMA_ID.to_string(),
            envelope_id: receipt.receipt_id,
            payload_schema_id: OUTCOME_ATTACH_PAYLOAD_SCHEMA_ID.to_string(),
            payload,
            payload_sha256,
            write_box,
        },
        proposed_receipt: RecordReceipt {
            record_id: receipt.receipt_id,
            write_box_envelope_id: receipt.receipt_id,
            persisted_at_utc: receipt.recorded_at_utc,
        },
    })
}

fn outcome_payload_value(
    attribution: &OutcomeAttribution,
    receipt: &OutcomeReceipt,
) -> Result<Value, OutcomeError> {
    serde_json::to_value(json!({
        "schema_id": OUTCOME_ATTACH_PAYLOAD_SCHEMA_ID,
        "outcome_receipt_id": receipt.receipt_id,
        "attribution": attribution,
    }))
    .map_err(|error| OutcomeError::Serialization(error.to_string()))
}

fn outcome_action_request(
    attribution: &OutcomeAttribution,
    receipt: &OutcomeReceipt,
) -> KernelActionRequestV1 {
    KernelActionRequestV1 {
        schema_id: KERNEL_ACTION_REQUEST_SCHEMA_ID.to_string(),
        action_id: OUTCOME_ATTACH_ACTION_ID.to_string(),
        actor: KernelActorRef {
            actor_id: OUTCOME_ATTACH_ACTOR_ID.to_string(),
            actor_kind: "role".to_string(),
            role_id: OUTCOME_ATTACH_ACTOR_ID.to_string(),
        },
        session: KernelSessionRef {
            session_id: OUTCOME_ATTACH_SESSION_ID.to_string(),
            work_profile_id: "memory-outcome-feedback".to_string(),
        },
        target_ids: vec![KernelTargetRef {
            target_id: attribution.capsule_id.to_string(),
            target_kind: "memory_capsule".to_string(),
            authority_class: "pre_promotion_memory".to_string(),
        }],
        input_schema_id: OUTCOME_ATTACH_INPUT_SCHEMA_ID.to_string(),
        expected_write_boxes: vec![ExpectedWriteBoxRef {
            write_box_kind: "MemoryBox".to_string(),
            write_box_schema_id: MEMORY_WRITE_BOX_SCHEMA_ID.to_string(),
            target_id: "memory_capsule_outcome".to_string(),
        }],
        authority_effect: AuthorityEffect::PrePromotionEvidenceOnly,
        approval_posture: ApprovalPosture::RequiresPromotionGate,
        validation_requirements: outcome_validation_requirements(),
        trace_id: format!("memory-capsule-outcome:{}", receipt.receipt_id),
        idempotency_key: outcome_idempotency_key(attribution),
    }
}

fn outcome_write_box(
    attribution: &OutcomeAttribution,
    receipt: &OutcomeReceipt,
    payload_sha256: &str,
) -> MemoryBox {
    let payload_ref = format!("memory-capsule-outcome://{}", receipt.receipt_id);
    MemoryBox {
        common: WriteBoxCommon {
            write_box_id: receipt.receipt_id.to_string(),
            kind: WriteBoxKind::Memory,
            schema_version: MEMORY_WRITE_BOX_SCHEMA_ID.to_string(),
            workspace_id: OUTCOME_ATTACH_SESSION_ID.to_string(),
            owner: WriteBoxOwnerRef {
                actor_id: OUTCOME_ATTACH_ACTOR_ID.to_string(),
                actor_kind: "role".to_string(),
                role_id: OUTCOME_ATTACH_ACTOR_ID.to_string(),
            },
            crdt_site_id: "memory-outcome-feedback".to_string(),
            target_refs: vec![WriteBoxTargetRef {
                target_id: attribution.capsule_id.to_string(),
                target_kind: "memory_capsule".to_string(),
                authority_class: "pre_promotion_memory".to_string(),
            }],
            base_snapshot_refs: vec![format!("memory-capsule://{}", attribution.capsule_id)],
            intent_summary: "Attach MemoryCapsule outcome through MemoryBox evidence".to_string(),
            operation_payload_refs: vec![WriteBoxPayloadRef {
                payload_id: receipt.receipt_id.to_string(),
                payload_kind: "memory_capsule_outcome_v1".to_string(),
                payload_ref: payload_ref.clone(),
                payload_sha256: payload_sha256.to_string(),
            }],
            lifecycle_state: WriteBoxLifecycleState::Open,
            allowed_transitions: vec![
                WriteBoxLifecycleState::ReadyForValidation,
                WriteBoxLifecycleState::Denied,
            ],
            authority_effect: AuthorityEffect::PrePromotionEvidenceOnly,
            evidence_refs: vec![format!("memory-capsule://{}", attribution.capsule_id)],
            receipt_refs: vec![format!(
                "receipt://memory-capsule-outcome/{}",
                receipt.receipt_id
            )],
            denial_receipt_refs: Vec::new(),
            promotion_receipt_refs: Vec::new(),
            validation_status: WriteBoxValidationStatus {
                state: WriteBoxValidationState::Pending,
                check_ids: outcome_validation_check_ids(),
            },
            projection_rules: vec!["dcc.memory_capsule_review".to_string()],
            replay_metadata: WriteBoxReplayMetadataV1 {
                replay_plan_ref: format!("memory-capsule-outcome://{}", attribution.capsule_id),
                replay_order_key: format!(
                    "{}/{}/{}",
                    OUTCOME_ATTACH_SESSION_ID,
                    attribution.attached_at_utc.to_rfc3339(),
                    receipt.receipt_id
                ),
                idempotency_key: outcome_idempotency_key(attribution),
                source_event_refs: vec![format!("memory-capsule://{}", attribution.capsule_id)],
            },
        },
        memory_extract_ref: payload_ref,
    }
}

fn outcome_validation_requirements() -> Vec<ValidationRequirement> {
    outcome_validation_check_ids()
        .into_iter()
        .map(|check_id| ValidationRequirement {
            check_id,
            required: true,
        })
        .collect()
}

fn outcome_validation_check_ids() -> Vec<String> {
    [
        "schema_validity",
        "outcome_attribution",
        "write_box_review_gate",
    ]
    .into_iter()
    .map(str::to_string)
    .collect()
}

fn outcome_idempotency_key(attribution: &OutcomeAttribution) -> String {
    let value = json!({
        "capsule_id": attribution.capsule_id,
        "outcome": attribution.outcome,
    });
    format!(
        "memory_capsule_outcome:{}:{}",
        attribution.capsule_id,
        sha256_hex(&canonical_json_bytes(&value))
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    struct MockSubmitter {
        attachments: Mutex<Vec<OutcomeAttribution>>,
    }

    impl MockSubmitter {
        fn new() -> Self {
            Self {
                attachments: Mutex::new(Vec::new()),
            }
        }
    }

    impl OutcomeAttachSubmitter for MockSubmitter {
        fn attach_outcome(
            &self,
            attribution: OutcomeAttribution,
        ) -> Result<OutcomeReceipt, OutcomeError> {
            self.attachments.lock().unwrap().push(attribution.clone());
            Ok(OutcomeReceipt {
                receipt_id: Uuid::now_v7(),
                capsule_id: attribution.capsule_id,
                action_id: OUTCOME_ATTACH_ACTION_ID.to_string(),
                recorded_at_utc: Utc::now(),
            })
        }
    }

    fn items(n: u128) -> Vec<MemoryPackItemRef> {
        (0..n)
            .map(|i| MemoryPackItemRef {
                memory_id: Uuid::from_u128(i + 1),
                pinned: false,
            })
            .collect()
    }

    #[test]
    fn pass_outcome_increases_scores_by_exactly_pass_boost() {
        let tuning = TuningParams {
            pass_boost: 0.05,
            fail_penalty: 0.0,
            escalation_penalty: 0.0,
            per_item_decay_per_use: 0.0,
            max_abs_change_per_call: 1.0,
        };
        let mut scores: HashMap<Uuid, f64> = HashMap::new();
        let pack = items(3);
        for it in &pack {
            scores.insert(it.memory_id, 0.5);
        }
        OutcomeScoringTuner::apply_outcome(
            &mut scores,
            &CapsuleOutcome::Pass {
                mt_id: "MT-1".to_string(),
                validator_verdict_id: Uuid::now_v7(),
            },
            &pack,
            &tuning,
        );
        for it in &pack {
            assert!((scores[&it.memory_id] - 0.55).abs() < 1e-9);
        }
    }

    #[test]
    fn fail_outcome_decreases_scores_by_exactly_fail_penalty() {
        let tuning = TuningParams::default();
        let mut scores: HashMap<Uuid, f64> = HashMap::new();
        let pack = items(2);
        for it in &pack {
            scores.insert(it.memory_id, 0.5);
        }
        OutcomeScoringTuner::apply_outcome(
            &mut scores,
            &CapsuleOutcome::Fail {
                mt_id: "MT-1".to_string(),
                validator_verdict_id: Uuid::now_v7(),
                failure_class: FailureClass::ValidatorRejected,
            },
            &pack,
            &tuning,
        );
        for it in &pack {
            // 0.5 + (-0.10) - 0.01 = 0.39
            assert!((scores[&it.memory_id] - 0.39).abs() < 1e-9);
        }
    }

    #[test]
    fn skipped_outcome_leaves_scores_unchanged() {
        let tuning = TuningParams::default();
        let mut scores: HashMap<Uuid, f64> = HashMap::new();
        let pack = items(1);
        scores.insert(pack[0].memory_id, 0.5);
        OutcomeScoringTuner::apply_outcome(
            &mut scores,
            &CapsuleOutcome::Skipped {
                reason: "n/a".to_string(),
            },
            &pack,
            &tuning,
        );
        assert!((scores[&pack[0].memory_id] - 0.5).abs() < 1e-9);
    }

    #[test]
    fn pinned_items_are_skipped() {
        let tuning = TuningParams::default();
        let mut scores: HashMap<Uuid, f64> = HashMap::new();
        let pack = vec![
            MemoryPackItemRef {
                memory_id: Uuid::from_u128(1),
                pinned: true,
            },
            MemoryPackItemRef {
                memory_id: Uuid::from_u128(2),
                pinned: false,
            },
        ];
        for it in &pack {
            scores.insert(it.memory_id, 0.5);
        }
        OutcomeScoringTuner::apply_outcome(
            &mut scores,
            &CapsuleOutcome::Fail {
                mt_id: "MT-1".to_string(),
                validator_verdict_id: Uuid::now_v7(),
                failure_class: FailureClass::Other,
            },
            &pack,
            &tuning,
        );
        // pinned: unchanged
        assert!((scores[&pack[0].memory_id] - 0.5).abs() < 1e-9);
        // non-pinned: changed
        assert!(scores[&pack[1].memory_id] < 0.5);
    }

    #[test]
    fn changes_bounded_per_call() {
        let tuning = TuningParams {
            pass_boost: 0.0,
            fail_penalty: 2.0, // larger than cap
            escalation_penalty: 0.0,
            per_item_decay_per_use: 0.0,
            max_abs_change_per_call: 0.25,
        };
        let mut scores: HashMap<Uuid, f64> = HashMap::new();
        let pack = items(1);
        scores.insert(pack[0].memory_id, 0.5);
        OutcomeScoringTuner::apply_outcome(
            &mut scores,
            &CapsuleOutcome::Fail {
                mt_id: "MT-1".to_string(),
                validator_verdict_id: Uuid::now_v7(),
                failure_class: FailureClass::Other,
            },
            &pack,
            &tuning,
        );
        // Should be clamped to 0.25 absolute change.
        assert!((scores[&pack[0].memory_id] - 0.25).abs() < 1e-9);
    }

    #[test]
    fn outcome_attaches_via_mock_catalog() {
        let submitter = MockSubmitter::new();
        let loop_ = OutcomeFeedbackLoop::new(&submitter);
        let capsule_id = Uuid::now_v7();
        let pack = items(1);
        let mut scores: HashMap<Uuid, f64> = HashMap::new();
        scores.insert(pack[0].memory_id, 0.5);
        let receipt = loop_
            .record_outcome(
                capsule_id,
                CapsuleOutcome::Pass {
                    mt_id: "MT-1".to_string(),
                    validator_verdict_id: Uuid::now_v7(),
                },
                &mut scores,
                &pack,
                &TuningParams {
                    per_item_decay_per_use: 0.0,
                    ..TuningParams::default()
                },
            )
            .unwrap();
        assert_eq!(receipt.capsule_id, capsule_id);
        assert_eq!(receipt.action_id, OUTCOME_ATTACH_ACTION_ID);
        assert_eq!(submitter.attachments.lock().unwrap().len(), 1);
        assert!(scores[&pack[0].memory_id] > 0.5);
    }
}
