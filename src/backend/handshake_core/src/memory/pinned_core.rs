//! MT-159: FEMS pinned core memory.
//!
//! `pinned: bool` flag (already present on `MemoryPackItem`-projection;
//! see `CapsuleAuditEntry.pinned` and `RetrievedItem.pinned`). This module
//! provides:
//! - `PinnedCoreSelector` — selects all pinned items first, then runs the
//!   remaining scoring on the remaining budget
//! - `PinIpc` Tauri commands - set/list pin status, all routed through
//!   `KernelActionCatalogV1`

use std::collections::BTreeSet;

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

use super::builder::RetrievedItem;
use super::persistence::{
    KERNEL_ACTION_REQUEST_SCHEMA_ID, KernelActionSubmission, MEMORY_WRITE_BOX_SCHEMA_ID,
    RecordReceipt, WRITE_BOX_V1_ENVELOPE_SCHEMA_ID, WriteBoxV1Envelope,
};

pub const PIN_MEMORY_ACTION_ID: &str = "kernel.memory_pin.set";
pub const UNPIN_MEMORY_ACTION_ID: &str = "kernel.memory_pin.unset";
pub const PIN_MEMORY_INPUT_SCHEMA_ID: &str = "hsk.kernel.memory_pin_input@1";
pub const PIN_MEMORY_PAYLOAD_SCHEMA_ID: &str = "hsk.memory_pin.payload@1";
pub const PIN_MEMORY_RESULT_SCHEMA_ID: &str = "hsk.kernel.memory_pin_result@1";
pub const PIN_MEMORY_ACTOR_ID: &str = "memory_pin_ipc";
pub const PIN_MEMORY_SESSION_ID: &str = "memory-pin";
pub const MEMORY_PIN_AGGREGATE_TYPE: &str = "memory_item";
pub const MEMORY_PIN_MANIFEST_AGGREGATE_TYPE: &str = "memory_pin_manifest";
pub const MEMORY_PIN_MANIFEST_AGGREGATE_ID: &str = "memory_pin_manifest_v1";
pub const MEMORY_PIN_SOURCE_COMPONENT: &str = "memory_pin_kernel_action_catalog";
pub const FR_EVT_MEMORY_PIN: &str = "FR-EVT-MEMORY-PIN";
pub const FR_EVT_MEMORY_UNPIN: &str = "FR-EVT-MEMORY-UNPIN";

/// Result of running the pinned-aware selector: pinned-first ordered
/// MemoryPack items plus accounting of bytes / overflow.
#[derive(Debug, Clone, PartialEq)]
pub struct PinnedSelection {
    pub ordered_items: Vec<RetrievedItem>,
    pub pinned_bytes: u64,
    pub remaining_budget_bytes: u64,
}

/// Budget shape the selector consumes. Mirrors the capsule policy budget.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PinnedBudget {
    pub max_items: u32,
    pub max_bytes: u64,
}

pub struct PinnedCoreSelector;

impl PinnedCoreSelector {
    /// Build a pack: (1) every pinned item first, (2) then run scoring on
    /// the rest with remaining budget.
    pub fn select_pack_with_pins(
        items: &[RetrievedItem],
        budget: PinnedBudget,
    ) -> Result<PinnedSelection, PinError> {
        let mut pinned_items: Vec<&RetrievedItem> = items.iter().filter(|it| it.pinned).collect();
        let unpinned_items: Vec<&RetrievedItem> = items.iter().filter(|it| !it.pinned).collect();

        let mut pinned_bytes = 0u64;
        for it in &pinned_items {
            pinned_bytes = pinned_bytes.saturating_add(it.capsule_bytes);
        }
        if pinned_bytes > budget.max_bytes || pinned_items.len() as u32 > budget.max_items {
            return Err(PinError::PinnedExceedsBudget {
                pinned_bytes,
                budget_bytes: budget.max_bytes,
                pinned_items: pinned_items.len() as u32,
                budget_items: budget.max_items,
            });
        }
        let remaining_budget_bytes = budget.max_bytes.saturating_sub(pinned_bytes);

        pinned_items.sort_by(|a, b| a.item_id.cmp(&b.item_id));
        let mut sorted_unpinned: Vec<&RetrievedItem> = unpinned_items;
        sorted_unpinned.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.item_id.cmp(&b.item_id))
        });

        let mut ordered_items: Vec<RetrievedItem> = Vec::new();
        let mut selected_ids = BTreeSet::new();
        for it in &pinned_items {
            selected_ids.insert(it.item_id.clone());
            ordered_items.push((*it).clone());
        }

        let mut used_bytes = pinned_bytes;
        for it in &sorted_unpinned {
            if selected_ids.contains(&it.item_id) {
                continue;
            }
            if ordered_items.len() as u32 >= budget.max_items {
                break;
            }
            let next = used_bytes.saturating_add(it.capsule_bytes);
            if next > budget.max_bytes {
                continue;
            }
            used_bytes = next;
            selected_ids.insert(it.item_id.clone());
            ordered_items.push((*it).clone());
        }

        Ok(PinnedSelection {
            ordered_items,
            pinned_bytes,
            remaining_budget_bytes,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PinnedItem {
    pub memory_id: Uuid,
    pub pinned: bool,
    pub reason: String,
    pub actor_id: String,
    pub session_id: String,
    pub set_at_utc: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PinReceipt {
    pub receipt_id: Uuid,
    pub memory_id: Uuid,
    pub pinned: bool,
    pub action_id: String,
    pub fr_event_kind: String,
}

pub trait PinSubmitter {
    fn set_pin(&self, item: PinnedItem) -> Result<PinReceipt, PinError>;
    fn list_pinned(&self) -> Result<Vec<PinnedItem>, PinError>;
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "event")]
pub enum PinFlightRecorderEvent {
    MemoryPin { memory_id: Uuid, reason: String },
    MemoryUnpin { memory_id: Uuid, reason: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SetPinRequest {
    pub item_id: Uuid,
    pub pinned: bool,
    pub reason: String,
    pub actor_id: String,
    pub session_id: String,
}

pub struct PinIpcService<'a> {
    submitter: &'a dyn PinSubmitter,
}

impl<'a> PinIpcService<'a> {
    pub fn new(submitter: &'a dyn PinSubmitter) -> Self {
        Self { submitter }
    }

    pub fn set(&self, request: SetPinRequest) -> Result<PinReceipt, PinError> {
        let reason = normalized_reason(request.reason)?;
        let actor_id = normalized_non_empty(request.actor_id, "actor_id")?;
        let session_id = normalized_non_empty(request.session_id, "session_id")?;
        let item = PinnedItem {
            memory_id: request.item_id,
            pinned: request.pinned,
            reason: reason.clone(),
            actor_id,
            session_id,
            set_at_utc: Utc::now(),
        };
        self.submitter.set_pin(item)
    }

    pub fn list(&self) -> Result<Vec<PinnedItem>, PinError> {
        self.submitter.list_pinned()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Error, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum PinError {
    #[error("pinned items exceed budget: pinned={pinned_bytes} budget={budget_bytes}")]
    PinnedExceedsBudget {
        pinned_bytes: u64,
        budget_bytes: u64,
        pinned_items: u32,
        budget_items: u32,
    },
    #[error("pin operation rejected: {code}: {reason}")]
    Rejected { code: String, reason: String },
    #[error("empty pin rationale")]
    EmptyReason,
    #[error("pin operation serialization failed: {0}")]
    Serialization(String),
    #[error("invalid pin operation shape {field}: {message}")]
    InvalidShape {
        field: &'static str,
        message: String,
    },
}

pub(crate) fn pin_submission(
    item: &PinnedItem,
    receipt: &PinReceipt,
) -> Result<KernelActionSubmission, PinError> {
    let payload = pin_payload(item, receipt)?;
    let payload_sha256 = sha256_hex(&canonical_json_bytes(&payload));
    let write_box = pin_write_box(item, receipt, &payload_sha256);
    validate_write_box_common(&write_box.common).map_err(|errors| PinError::InvalidShape {
        field: "write_box",
        message: format!("{errors:?}"),
    })?;

    Ok(KernelActionSubmission {
        request: pin_action_request(item, receipt),
        write_box_envelope: WriteBoxV1Envelope {
            schema_id: WRITE_BOX_V1_ENVELOPE_SCHEMA_ID.to_string(),
            envelope_id: receipt.receipt_id,
            payload_schema_id: PIN_MEMORY_PAYLOAD_SCHEMA_ID.to_string(),
            payload,
            payload_sha256,
            write_box,
        },
        proposed_receipt: RecordReceipt {
            record_id: receipt.receipt_id,
            write_box_envelope_id: receipt.receipt_id,
            persisted_at_utc: item.set_at_utc,
        },
    })
}

fn pin_payload(item: &PinnedItem, receipt: &PinReceipt) -> Result<Value, PinError> {
    serde_json::to_value(json!({
        "schema_id": PIN_MEMORY_PAYLOAD_SCHEMA_ID,
        "pin_receipt_id": receipt.receipt_id,
        "pinned_item": item,
        "flight_recorder_event_id": receipt.fr_event_kind,
    }))
    .map_err(|error| PinError::Serialization(error.to_string()))
}

fn pin_action_request(item: &PinnedItem, receipt: &PinReceipt) -> KernelActionRequestV1 {
    KernelActionRequestV1 {
        schema_id: KERNEL_ACTION_REQUEST_SCHEMA_ID.to_string(),
        action_id: receipt.action_id.clone(),
        actor: KernelActorRef {
            actor_id: item.actor_id.clone(),
            actor_kind: "role".to_string(),
            role_id: item.actor_id.clone(),
        },
        session: KernelSessionRef {
            session_id: item.session_id.clone(),
            work_profile_id: "memory-pin".to_string(),
        },
        target_ids: vec![KernelTargetRef {
            target_id: item.memory_id.to_string(),
            target_kind: "memory_item".to_string(),
            authority_class: "pre_promotion_memory".to_string(),
        }],
        input_schema_id: PIN_MEMORY_INPUT_SCHEMA_ID.to_string(),
        expected_write_boxes: vec![ExpectedWriteBoxRef {
            write_box_kind: "MemoryBox".to_string(),
            write_box_schema_id: MEMORY_WRITE_BOX_SCHEMA_ID.to_string(),
            target_id: pin_target_id(item.pinned).to_string(),
        }],
        authority_effect: AuthorityEffect::PrePromotionEvidenceOnly,
        approval_posture: ApprovalPosture::RequiresPromotionGate,
        validation_requirements: pin_validation_requirements(),
        trace_id: format!("memory-pin:{}", receipt.receipt_id),
        idempotency_key: pin_idempotency_key(item),
    }
}

fn pin_write_box(item: &PinnedItem, receipt: &PinReceipt, payload_sha256: &str) -> MemoryBox {
    let payload_ref = format!("memory-pin://{}", receipt.receipt_id);
    MemoryBox {
        common: WriteBoxCommon {
            write_box_id: receipt.receipt_id.to_string(),
            kind: WriteBoxKind::Memory,
            schema_version: MEMORY_WRITE_BOX_SCHEMA_ID.to_string(),
            workspace_id: item.session_id.clone(),
            owner: WriteBoxOwnerRef {
                actor_id: item.actor_id.clone(),
                actor_kind: "role".to_string(),
                role_id: item.actor_id.clone(),
            },
            crdt_site_id: "memory-pin-ipc".to_string(),
            target_refs: vec![WriteBoxTargetRef {
                target_id: item.memory_id.to_string(),
                target_kind: "memory_item".to_string(),
                authority_class: "pre_promotion_memory".to_string(),
            }],
            base_snapshot_refs: vec![format!("memory-item://{}", item.memory_id)],
            intent_summary: if item.pinned {
                "Pin MemoryItem as core memory through MemoryBox evidence".to_string()
            } else {
                "Unpin MemoryItem core-memory flag through MemoryBox evidence".to_string()
            },
            operation_payload_refs: vec![WriteBoxPayloadRef {
                payload_id: receipt.receipt_id.to_string(),
                payload_kind: "memory_pin_v1".to_string(),
                payload_ref: payload_ref.clone(),
                payload_sha256: payload_sha256.to_string(),
            }],
            lifecycle_state: WriteBoxLifecycleState::Open,
            allowed_transitions: vec![
                WriteBoxLifecycleState::ReadyForValidation,
                WriteBoxLifecycleState::Denied,
            ],
            authority_effect: AuthorityEffect::PrePromotionEvidenceOnly,
            evidence_refs: vec![format!("memory-item://{}", item.memory_id)],
            receipt_refs: vec![format!("receipt://memory-pin/{}", receipt.receipt_id)],
            denial_receipt_refs: Vec::new(),
            promotion_receipt_refs: Vec::new(),
            validation_status: WriteBoxValidationStatus {
                state: WriteBoxValidationState::Pending,
                check_ids: pin_validation_check_ids(),
            },
            projection_rules: vec!["dcc.memory_pin_review".to_string()],
            replay_metadata: WriteBoxReplayMetadataV1 {
                replay_plan_ref: format!("memory-pin://{}", item.memory_id),
                replay_order_key: format!(
                    "{}/{}/{}",
                    item.session_id,
                    item.set_at_utc.to_rfc3339(),
                    receipt.receipt_id
                ),
                idempotency_key: pin_idempotency_key(item),
                source_event_refs: vec![format!("{}://{}", receipt.fr_event_kind, item.memory_id)],
            },
        },
        memory_extract_ref: payload_ref,
    }
}

fn pin_validation_requirements() -> Vec<ValidationRequirement> {
    pin_validation_check_ids()
        .into_iter()
        .map(|check_id| ValidationRequirement {
            check_id,
            required: true,
        })
        .collect()
}

fn pin_validation_check_ids() -> Vec<String> {
    [
        "schema_validity",
        "pin_reason",
        "write_box_review_gate",
        "flight_recorder_event",
    ]
    .into_iter()
    .map(str::to_string)
    .collect()
}

fn pin_target_id(pinned: bool) -> &'static str {
    if pinned {
        "memory_item_pin"
    } else {
        "memory_item_unpin"
    }
}

pub(crate) fn action_id_for_pin_state(pinned: bool) -> &'static str {
    if pinned {
        PIN_MEMORY_ACTION_ID
    } else {
        UNPIN_MEMORY_ACTION_ID
    }
}

pub(crate) fn fr_event_for_pin_state(pinned: bool) -> &'static str {
    if pinned {
        FR_EVT_MEMORY_PIN
    } else {
        FR_EVT_MEMORY_UNPIN
    }
}

fn pin_idempotency_key(item: &PinnedItem) -> String {
    let value = json!({
        "memory_id": item.memory_id,
        "pinned": item.pinned,
        "reason": item.reason,
    });
    format!(
        "memory_pin:{}:{}",
        item.memory_id,
        sha256_hex(&canonical_json_bytes(&value))
    )
}

fn normalized_reason(reason: String) -> Result<String, PinError> {
    normalized_non_empty(reason, "reason").map_err(|error| match error {
        PinError::InvalidShape { .. } => PinError::EmptyReason,
        other => other,
    })
}

fn normalized_non_empty(value: String, field: &'static str) -> Result<String, PinError> {
    let value = value.trim().to_string();
    if value.is_empty() {
        if field == "reason" {
            return Err(PinError::EmptyReason);
        }
        return Err(PinError::InvalidShape {
            field,
            message: "value must not be empty".to_string(),
        });
    }
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    fn item(id: &str, score: f64, bytes: u64, pinned: bool) -> RetrievedItem {
        RetrievedItem {
            item_id: id.to_string(),
            memory_class: "test".to_string(),
            item_type: "doc".to_string(),
            summary: id.to_string(),
            content: id.to_string(),
            structured: None,
            trust_level: "trusted".to_string(),
            confidence: score,
            scope_refs: Vec::new(),
            source_refs: Vec::new(),
            score,
            score_breakdown: BTreeMap::new(),
            capsule_bytes: bytes,
            token_estimate: bytes as u32 / 4,
            pinned,
        }
    }

    fn pinned_item() -> PinnedItem {
        PinnedItem {
            memory_id: Uuid::now_v7(),
            pinned: true,
            reason: "operator core memory".to_string(),
            actor_id: "KERNEL_BUILDER".to_string(),
            session_id: "session-159".to_string(),
            set_at_utc: Utc::now(),
        }
    }

    #[test]
    fn pin_submission_uses_caller_actor_and_session() {
        let item = pinned_item();
        let receipt = PinReceipt {
            receipt_id: Uuid::now_v7(),
            memory_id: item.memory_id,
            pinned: true,
            action_id: PIN_MEMORY_ACTION_ID.to_string(),
            fr_event_kind: FR_EVT_MEMORY_PIN.to_string(),
        };

        let submission = pin_submission(&item, &receipt).expect("pin submission");

        assert_eq!(submission.request.actor.actor_id, "KERNEL_BUILDER");
        assert_eq!(submission.request.actor.role_id, "KERNEL_BUILDER");
        assert_eq!(submission.request.session.session_id, "session-159");
        assert_eq!(
            submission.write_box_envelope.write_box.common.workspace_id,
            "session-159"
        );
        assert_eq!(
            submission
                .write_box_envelope
                .write_box
                .common
                .owner
                .actor_id,
            "KERNEL_BUILDER"
        );
    }

    #[test]
    fn pinned_items_always_included_first() {
        let items = vec![
            item("u1", 0.9, 1000, false),
            item("p1", 0.1, 1000, true),
            item("u2", 0.8, 1000, false),
        ];
        let sel = PinnedCoreSelector::select_pack_with_pins(
            &items,
            PinnedBudget {
                max_items: 5,
                max_bytes: 100_000,
            },
        )
        .unwrap();
        assert!(sel.ordered_items[0].pinned);
        assert_eq!(sel.ordered_items[0].item_id, "p1");
    }

    #[test]
    fn pinned_overflow_returns_typed_error() {
        let items = vec![
            item("p1", 0.5, 200_000, true),
            item("p2", 0.5, 200_000, true),
        ];
        let err = PinnedCoreSelector::select_pack_with_pins(
            &items,
            PinnedBudget {
                max_items: 5,
                max_bytes: 300_000,
            },
        )
        .unwrap_err();
        assert!(matches!(err, PinError::PinnedExceedsBudget { .. }));
    }

    #[test]
    fn budget_clamp_respected_on_unpinned() {
        let items = vec![
            item("p1", 0.5, 5000, true),
            item("u1", 0.9, 5000, false),
            item("u2", 0.8, 5000, false),
            item("u3", 0.7, 5000, false),
        ];
        let sel = PinnedCoreSelector::select_pack_with_pins(
            &items,
            PinnedBudget {
                max_items: 10,
                max_bytes: 15_000,
            },
        )
        .unwrap();
        assert_eq!(sel.ordered_items.len(), 3); // p1 + 2 unpinned
        assert_eq!(sel.pinned_bytes, 5_000);
        assert_eq!(sel.remaining_budget_bytes, 10_000);
    }

    #[test]
    fn max_items_caps_unpinned() {
        let items = vec![
            item("p1", 0.5, 1000, true),
            item("u1", 0.9, 1000, false),
            item("u2", 0.8, 1000, false),
        ];
        let sel = PinnedCoreSelector::select_pack_with_pins(
            &items,
            PinnedBudget {
                max_items: 2,
                max_bytes: 100_000,
            },
        )
        .unwrap();
        assert_eq!(sel.ordered_items.len(), 2);
    }
}
