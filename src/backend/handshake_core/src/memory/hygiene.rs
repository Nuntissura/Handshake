//! MT-160: FEMS hygiene manager job.
//!
//! Consolidate + Prune + Flag + PromoteProcedural tasks. No auto-merge,
//! no auto-delete — every mutation flows through KernelActionCatalogV1
//! and is operator-review-gated. Pruning uses bitemporal invalidation
//! (sets `invalidated_at`) instead of destructive delete.

use std::time::Duration;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use thiserror::Error;
use uuid::Uuid;

use super::bitemporal::{AsOfQuery, BitemporalIndex, PostgresBitemporalMemoryIndex};
use super::persistence::{
    KernelActionSubmission, RecordReceipt, WriteBoxV1Envelope, KERNEL_ACTION_REQUEST_SCHEMA_ID,
    MEMORY_WRITE_BOX_SCHEMA_ID, WRITE_BOX_V1_ENVELOPE_SCHEMA_ID,
};
use crate::kernel::{
    action_envelope::{
        validate_kernel_action_request, ApprovalPosture, AuthorityEffect, ExpectedWriteBoxRef,
        KernelActionRequestV1, KernelActorRef, KernelSessionRef, KernelTargetRef,
        ValidationRequirement,
    },
    context_bundle::{canonical_json_bytes, sha256_hex},
    write_boxes::{
        validate_write_box_common, MemoryBox, WriteBoxCommon, WriteBoxKind, WriteBoxLifecycleState,
        WriteBoxOwnerRef, WriteBoxPayloadRef, WriteBoxReplayMetadataV1, WriteBoxTargetRef,
        WriteBoxValidationState, WriteBoxValidationStatus,
    },
};

pub const HYGIENE_CONSOLIDATION_ACTION_ID: &str = "kernel.memory_hygiene.consolidation_candidate";
pub const HYGIENE_PRUNE_ACTION_ID: &str = "kernel.memory_hygiene.prune";
pub const HYGIENE_FLAG_ACTION_ID: &str = "kernel.memory_hygiene.flag_contradiction";
pub const HYGIENE_PROMOTE_ACTION_ID: &str = "kernel.memory_hygiene.promote_procedural";
pub const HYGIENE_CONSOLIDATION_INPUT_SCHEMA_ID: &str =
    "hsk.kernel.memory_hygiene_consolidation_input@1";
pub const HYGIENE_PRUNE_INPUT_SCHEMA_ID: &str = "hsk.kernel.memory_hygiene_prune_input@1";
pub const HYGIENE_FLAG_INPUT_SCHEMA_ID: &str =
    "hsk.kernel.memory_hygiene_contradiction_flag_input@1";
pub const HYGIENE_PROMOTE_INPUT_SCHEMA_ID: &str =
    "hsk.kernel.memory_hygiene_procedural_promotion_input@1";
pub const HYGIENE_PAYLOAD_SCHEMA_ID: &str = "hsk.memory_hygiene.candidate_payload@1";
pub const HYGIENE_RESULT_SCHEMA_ID: &str = "hsk.kernel.memory_hygiene_result@1";
pub const HYGIENE_ACTOR_ID: &str = "memory_hygiene_manager";
pub const HYGIENE_SESSION_ID: &str = "memory-hygiene";
pub const MEMORY_HYGIENE_SOURCE_COMPONENT: &str = "memory_hygiene_kernel_action_catalog";

const NEAR_DUPLICATE_COSINE_THRESHOLD: f64 = 0.97;
const CONTRADICTION_COSINE_THRESHOLD: f64 = 0.94;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "task")]
pub enum HygieneTask {
    Consolidate {
        max_pairs: u32,
    },
    PruneStale {
        older_than_secs: u64,
        min_score: f64,
    },
    FlagContradictions,
    PromoteProcedural {
        min_use_count: u32,
        min_pass_rate: f64,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HygieneConfig {
    pub tasks: Vec<HygieneTask>,
}

impl HygieneConfig {
    pub fn validate(&self) -> Result<(), HygieneError> {
        for task in &self.tasks {
            match task {
                HygieneTask::Consolidate { .. } | HygieneTask::FlagContradictions => {}
                HygieneTask::PruneStale { min_score, .. } => {
                    validate_finite_range(*min_score, "prune_stale.min_score")?;
                }
                HygieneTask::PromoteProcedural { min_pass_rate, .. } => {
                    validate_finite_range(*min_pass_rate, "promote_procedural.min_pass_rate")?;
                }
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HygieneTaskReport {
    pub task_kind: String,
    pub items_visited: u32,
    pub items_acted_on: u32,
    pub errors: Vec<String>,
    pub duration_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HygieneReport {
    pub started_at_utc: DateTime<Utc>,
    pub completed_at_utc: DateTime<Utc>,
    pub tasks: Vec<HygieneTaskReport>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProceduralPromotion {
    pub memory_id: Uuid,
    pub use_count: u32,
    pub pass_rate: f64,
    pub candidate_at_utc: DateTime<Utc>,
}

/// FemsAccessor trait — production wires to Postgres-backed FEMS surface;
/// tests use an in-memory BitemporalIndex + per-item stats.
pub trait FemsAccessor: Send + Sync {
    fn list_items(&self) -> Result<Vec<HygieneItemView>, HygieneError>;
    fn invalidate(&self, memory_id: Uuid, at: DateTime<Utc>) -> Result<(), HygieneError>;
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HygieneItemView {
    pub memory_id: Uuid,
    pub recorded_at: DateTime<Utc>,
    pub score: f64,
    pub use_count: u32,
    pub pass_count: u32,
    pub pinned: bool,
    pub content_fingerprint: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub embedding: Option<Vec<f32>>,
}

impl HygieneItemView {
    pub fn pass_rate(&self) -> f64 {
        if self.use_count == 0 {
            0.0
        } else {
            f64::from(self.pass_count) / f64::from(self.use_count)
        }
    }
}

/// Hygiene action submitter — every mutation flows through this trait.
pub trait HygieneActionSubmitter: Send + Sync {
    fn submit_consolidation_candidate(&self, left: Uuid, right: Uuid)
        -> Result<Uuid, HygieneError>;
    fn submit_prune(&self, memory_id: Uuid, at: DateTime<Utc>) -> Result<Uuid, HygieneError>;
    fn submit_contradiction_flag(&self, left: Uuid, right: Uuid) -> Result<Uuid, HygieneError>;
    fn submit_procedural_promotion(
        &self,
        candidate: ProceduralPromotion,
    ) -> Result<Uuid, HygieneError>;
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum HygieneCandidate {
    Consolidation {
        left: Uuid,
        right: Uuid,
    },
    Prune {
        memory_id: Uuid,
        requested_invalidated_at: DateTime<Utc>,
    },
    ContradictionFlag {
        left: Uuid,
        right: Uuid,
    },
    ProceduralPromotion {
        candidate: ProceduralPromotion,
    },
}

impl HygieneCandidate {
    fn action_id(&self) -> &'static str {
        match self {
            HygieneCandidate::Consolidation { .. } => HYGIENE_CONSOLIDATION_ACTION_ID,
            HygieneCandidate::Prune { .. } => HYGIENE_PRUNE_ACTION_ID,
            HygieneCandidate::ContradictionFlag { .. } => HYGIENE_FLAG_ACTION_ID,
            HygieneCandidate::ProceduralPromotion { .. } => HYGIENE_PROMOTE_ACTION_ID,
        }
    }

    fn input_schema_id(&self) -> &'static str {
        match self {
            HygieneCandidate::Consolidation { .. } => HYGIENE_CONSOLIDATION_INPUT_SCHEMA_ID,
            HygieneCandidate::Prune { .. } => HYGIENE_PRUNE_INPUT_SCHEMA_ID,
            HygieneCandidate::ContradictionFlag { .. } => HYGIENE_FLAG_INPUT_SCHEMA_ID,
            HygieneCandidate::ProceduralPromotion { .. } => HYGIENE_PROMOTE_INPUT_SCHEMA_ID,
        }
    }

    fn write_box_target_id(&self) -> &'static str {
        match self {
            HygieneCandidate::Consolidation { .. } => "memory_hygiene_consolidation_candidate",
            HygieneCandidate::Prune { .. } => "memory_hygiene_prune_candidate",
            HygieneCandidate::ContradictionFlag { .. } => "memory_hygiene_contradiction_flag",
            HygieneCandidate::ProceduralPromotion { .. } => "memory_hygiene_procedural_promotion",
        }
    }

    fn payload_kind(&self) -> &'static str {
        match self {
            HygieneCandidate::Consolidation { .. } => "memory_hygiene_consolidation_v1",
            HygieneCandidate::Prune { .. } => "memory_hygiene_prune_v1",
            HygieneCandidate::ContradictionFlag { .. } => "memory_hygiene_contradiction_v1",
            HygieneCandidate::ProceduralPromotion { .. } => "memory_hygiene_promotion_v1",
        }
    }

    fn intent_summary(&self) -> &'static str {
        match self {
            HygieneCandidate::Consolidation { .. } => {
                "Propose MemoryItem consolidation as review-gated hygiene evidence"
            }
            HygieneCandidate::Prune { .. } => {
                "Propose bitemporal MemoryItem invalidation as review-gated hygiene evidence"
            }
            HygieneCandidate::ContradictionFlag { .. } => {
                "Flag likely contradictory MemoryItems as review-gated hygiene evidence"
            }
            HygieneCandidate::ProceduralPromotion { .. } => {
                "Propose MemoryItem procedural promotion as review-gated hygiene evidence"
            }
        }
    }

    fn target_ids(&self) -> Vec<Uuid> {
        match self {
            HygieneCandidate::Consolidation { left, right }
            | HygieneCandidate::ContradictionFlag { left, right } => vec![*left, *right],
            HygieneCandidate::Prune { memory_id, .. } => vec![*memory_id],
            HygieneCandidate::ProceduralPromotion { candidate } => vec![candidate.memory_id],
        }
    }

    fn validation_check_ids(&self) -> Vec<String> {
        let task_check = match self {
            HygieneCandidate::Consolidation { .. } => "near_duplicate_evidence",
            HygieneCandidate::Prune { .. } => "bitemporal_invalidation_review",
            HygieneCandidate::ContradictionFlag { .. } => "contradiction_outcome_evidence",
            HygieneCandidate::ProceduralPromotion { .. } => "procedural_promotion_threshold",
        };
        ["schema_validity", task_check, "write_box_review_gate"]
            .into_iter()
            .map(str::to_string)
            .collect()
    }
}

pub(crate) fn hygiene_submission(
    candidate: &HygieneCandidate,
    receipt: &RecordReceipt,
) -> Result<KernelActionSubmission, HygieneError> {
    let payload = hygiene_payload(candidate, receipt)?;
    let payload_sha256 = sha256_hex(&canonical_json_bytes(&payload));
    let write_box = hygiene_write_box(candidate, receipt, &payload_sha256);
    validate_write_box_common(&write_box.common).map_err(|errors| HygieneError::InvalidShape {
        field: "write_box",
        message: format!("{errors:?}"),
    })?;

    let request = hygiene_action_request(candidate, receipt);
    validate_kernel_action_request(&request).map_err(|errors| HygieneError::InvalidShape {
        field: "kernel_action_request",
        message: format!("{errors:?}"),
    })?;

    Ok(KernelActionSubmission {
        request,
        write_box_envelope: WriteBoxV1Envelope {
            schema_id: WRITE_BOX_V1_ENVELOPE_SCHEMA_ID.to_string(),
            envelope_id: receipt.write_box_envelope_id,
            payload_schema_id: HYGIENE_PAYLOAD_SCHEMA_ID.to_string(),
            payload,
            payload_sha256,
            write_box,
        },
        proposed_receipt: receipt.clone(),
    })
}

fn hygiene_payload(
    candidate: &HygieneCandidate,
    receipt: &RecordReceipt,
) -> Result<Value, HygieneError> {
    serde_json::to_value(json!({
        "schema_id": HYGIENE_PAYLOAD_SCHEMA_ID,
        "candidate_receipt_id": receipt.record_id,
        "action_id": candidate.action_id(),
        "candidate": candidate,
    }))
    .map_err(|error| HygieneError::Serialization {
        message: error.to_string(),
    })
}

fn hygiene_action_request(
    candidate: &HygieneCandidate,
    receipt: &RecordReceipt,
) -> KernelActionRequestV1 {
    KernelActionRequestV1 {
        schema_id: KERNEL_ACTION_REQUEST_SCHEMA_ID.to_string(),
        action_id: candidate.action_id().to_string(),
        actor: KernelActorRef {
            actor_id: HYGIENE_ACTOR_ID.to_string(),
            actor_kind: "role".to_string(),
            role_id: HYGIENE_ACTOR_ID.to_string(),
        },
        session: KernelSessionRef {
            session_id: HYGIENE_SESSION_ID.to_string(),
            work_profile_id: HYGIENE_SESSION_ID.to_string(),
        },
        target_ids: hygiene_target_refs(candidate),
        input_schema_id: candidate.input_schema_id().to_string(),
        expected_write_boxes: vec![ExpectedWriteBoxRef {
            write_box_kind: "MemoryBox".to_string(),
            write_box_schema_id: MEMORY_WRITE_BOX_SCHEMA_ID.to_string(),
            target_id: candidate.write_box_target_id().to_string(),
        }],
        authority_effect: AuthorityEffect::PrePromotionEvidenceOnly,
        approval_posture: ApprovalPosture::RequiresPromotionGate,
        validation_requirements: candidate
            .validation_check_ids()
            .into_iter()
            .map(|check_id| ValidationRequirement {
                check_id,
                required: true,
            })
            .collect(),
        trace_id: format!("memory-hygiene:{}", receipt.record_id),
        idempotency_key: hygiene_idempotency_key(candidate),
    }
}

fn hygiene_write_box(
    candidate: &HygieneCandidate,
    receipt: &RecordReceipt,
    payload_sha256: &str,
) -> MemoryBox {
    let payload_ref = format!("memory-hygiene://{}", receipt.record_id);
    let target_refs = candidate
        .target_ids()
        .into_iter()
        .map(|target_id| WriteBoxTargetRef {
            target_id: target_id.to_string(),
            target_kind: "memory_item".to_string(),
            authority_class: "pre_promotion_memory".to_string(),
        })
        .collect::<Vec<_>>();
    let memory_refs = target_refs
        .iter()
        .map(|target| format!("memory-item://{}", target.target_id))
        .collect::<Vec<_>>();
    let source_event_refs = memory_refs.clone();

    MemoryBox {
        common: WriteBoxCommon {
            write_box_id: receipt.write_box_envelope_id.to_string(),
            kind: WriteBoxKind::Memory,
            schema_version: MEMORY_WRITE_BOX_SCHEMA_ID.to_string(),
            workspace_id: HYGIENE_SESSION_ID.to_string(),
            owner: WriteBoxOwnerRef {
                actor_id: HYGIENE_ACTOR_ID.to_string(),
                actor_kind: "role".to_string(),
                role_id: HYGIENE_ACTOR_ID.to_string(),
            },
            crdt_site_id: "memory-hygiene-manager".to_string(),
            target_refs,
            base_snapshot_refs: memory_refs.clone(),
            intent_summary: candidate.intent_summary().to_string(),
            operation_payload_refs: vec![WriteBoxPayloadRef {
                payload_id: receipt.record_id.to_string(),
                payload_kind: candidate.payload_kind().to_string(),
                payload_ref: payload_ref.clone(),
                payload_sha256: payload_sha256.to_string(),
            }],
            lifecycle_state: WriteBoxLifecycleState::Open,
            allowed_transitions: vec![
                WriteBoxLifecycleState::ReadyForValidation,
                WriteBoxLifecycleState::Denied,
            ],
            authority_effect: AuthorityEffect::PrePromotionEvidenceOnly,
            evidence_refs: memory_refs,
            receipt_refs: vec![format!("receipt://memory-hygiene/{}", receipt.record_id)],
            denial_receipt_refs: Vec::new(),
            promotion_receipt_refs: Vec::new(),
            validation_status: WriteBoxValidationStatus {
                state: WriteBoxValidationState::Pending,
                check_ids: candidate.validation_check_ids(),
            },
            projection_rules: vec!["dcc.memory_hygiene_review".to_string()],
            replay_metadata: WriteBoxReplayMetadataV1 {
                replay_plan_ref: format!("memory-hygiene://{}", receipt.record_id),
                replay_order_key: format!(
                    "{}/{}/{}",
                    HYGIENE_SESSION_ID,
                    receipt.persisted_at_utc.to_rfc3339(),
                    receipt.record_id
                ),
                idempotency_key: hygiene_idempotency_key(candidate),
                source_event_refs,
            },
        },
        memory_extract_ref: payload_ref,
    }
}

fn hygiene_target_refs(candidate: &HygieneCandidate) -> Vec<KernelTargetRef> {
    candidate
        .target_ids()
        .into_iter()
        .map(|target_id| KernelTargetRef {
            target_id: target_id.to_string(),
            target_kind: "memory_item".to_string(),
            authority_class: "pre_promotion_memory".to_string(),
        })
        .collect()
}

fn hygiene_idempotency_key(candidate: &HygieneCandidate) -> String {
    let value = json!({
        "action_id": candidate.action_id(),
        "candidate": candidate,
    });
    format!(
        "memory_hygiene:{}:{}",
        candidate.action_id(),
        sha256_hex(&canonical_json_bytes(&value))
    )
}

pub struct HygieneJobRunner<'a> {
    pub fems: &'a dyn FemsAccessor,
    pub action_catalog: &'a dyn HygieneActionSubmitter,
}

impl<'a> HygieneJobRunner<'a> {
    pub fn new(fems: &'a dyn FemsAccessor, action_catalog: &'a dyn HygieneActionSubmitter) -> Self {
        Self {
            fems,
            action_catalog,
        }
    }

    pub fn run_once(&self, config: HygieneConfig) -> Result<HygieneReport, HygieneError> {
        config.validate()?;
        let started_at_utc = Utc::now();
        let mut task_reports = Vec::new();
        for task in &config.tasks {
            let task_started = std::time::Instant::now();
            let report = match task {
                HygieneTask::Consolidate { max_pairs } => self.run_consolidate(*max_pairs),
                HygieneTask::PruneStale {
                    older_than_secs,
                    min_score,
                } => self.run_prune(*older_than_secs, *min_score),
                HygieneTask::FlagContradictions => self.run_flag_contradictions(),
                HygieneTask::PromoteProcedural {
                    min_use_count,
                    min_pass_rate,
                } => self.run_promote(*min_use_count, *min_pass_rate),
            };
            let mut report = report?;
            report.duration_ms = task_started.elapsed().as_millis() as u64;
            task_reports.push(report);
        }
        Ok(HygieneReport {
            started_at_utc,
            completed_at_utc: Utc::now(),
            tasks: task_reports,
        })
    }

    fn run_consolidate(&self, max_pairs: u32) -> Result<HygieneTaskReport, HygieneError> {
        let items = self.fems.list_items()?;
        let mut pairs_emitted = 0u32;
        let mut errors = Vec::new();
        for (i, left) in items.iter().enumerate() {
            for right in items.iter().skip(i + 1) {
                if pairs_emitted >= max_pairs {
                    break;
                }
                if left.pinned || right.pinned {
                    continue;
                }
                if near_duplicate(left, right) {
                    if let Err(e) = self
                        .action_catalog
                        .submit_consolidation_candidate(left.memory_id, right.memory_id)
                    {
                        errors.push(format!("{e}"));
                        continue;
                    }
                    pairs_emitted += 1;
                }
            }
        }
        Ok(HygieneTaskReport {
            task_kind: "consolidate".to_string(),
            items_visited: items.len() as u32,
            items_acted_on: pairs_emitted,
            errors,
            duration_ms: 0,
        })
    }

    fn run_prune(
        &self,
        older_than_secs: u64,
        min_score: f64,
    ) -> Result<HygieneTaskReport, HygieneError> {
        let items = self.fems.list_items()?;
        let cutoff = Utc::now() - chrono::Duration::seconds(older_than_secs as i64);
        let mut acted = 0u32;
        let mut errors = Vec::new();
        for item in &items {
            if item.pinned {
                continue;
            }
            if item.recorded_at < cutoff && item.score < min_score {
                if let Err(e) = self.action_catalog.submit_prune(item.memory_id, Utc::now()) {
                    errors.push(format!("{e}"));
                    continue;
                }
                acted += 1;
            }
        }
        Ok(HygieneTaskReport {
            task_kind: "prune_stale".to_string(),
            items_visited: items.len() as u32,
            items_acted_on: acted,
            errors,
            duration_ms: 0,
        })
    }

    fn run_flag_contradictions(&self) -> Result<HygieneTaskReport, HygieneError> {
        let items = self.fems.list_items()?;
        let mut flagged = 0u32;
        let mut errors = Vec::new();
        // Naive O(n^2) pair detection: items with identical content
        // fingerprint but diverging pass rates.
        for (i, left) in items.iter().enumerate() {
            for right in items.iter().skip(i + 1) {
                if left.pinned || right.pinned {
                    continue;
                }
                if likely_same_subject(left, right) {
                    let lpr = left.pass_rate();
                    let rpr = right.pass_rate();
                    // strict contradiction: one above 0.7 and another
                    // below 0.3
                    if (lpr > 0.7 && rpr < 0.3) || (rpr > 0.7 && lpr < 0.3) {
                        if let Err(e) = self
                            .action_catalog
                            .submit_contradiction_flag(left.memory_id, right.memory_id)
                        {
                            errors.push(format!("{e}"));
                            continue;
                        }
                        flagged += 1;
                    }
                }
            }
        }
        Ok(HygieneTaskReport {
            task_kind: "flag_contradictions".to_string(),
            items_visited: items.len() as u32,
            items_acted_on: flagged,
            errors,
            duration_ms: 0,
        })
    }

    fn run_promote(
        &self,
        min_use_count: u32,
        min_pass_rate: f64,
    ) -> Result<HygieneTaskReport, HygieneError> {
        let items = self.fems.list_items()?;
        let mut promoted = 0u32;
        let mut errors = Vec::new();
        for item in &items {
            if item.pinned {
                continue;
            }
            if item.use_count >= min_use_count && item.pass_rate() >= min_pass_rate {
                let candidate = ProceduralPromotion {
                    memory_id: item.memory_id,
                    use_count: item.use_count,
                    pass_rate: item.pass_rate(),
                    candidate_at_utc: Utc::now(),
                };
                if let Err(e) = self.action_catalog.submit_procedural_promotion(candidate) {
                    errors.push(format!("{e}"));
                    continue;
                }
                promoted += 1;
            }
        }
        Ok(HygieneTaskReport {
            task_kind: "promote_procedural".to_string(),
            items_visited: items.len() as u32,
            items_acted_on: promoted,
            errors,
            duration_ms: 0,
        })
    }
}

fn validate_finite_range(value: f64, field: &'static str) -> Result<(), HygieneError> {
    if !value.is_finite() {
        return Err(HygieneError::InvalidConfig {
            field,
            message: "threshold must be finite".to_string(),
        });
    }
    if !(0.0..=1.0).contains(&value) {
        return Err(HygieneError::InvalidConfig {
            field,
            message: "threshold must be within 0..=1".to_string(),
        });
    }
    Ok(())
}

fn near_duplicate(left: &HygieneItemView, right: &HygieneItemView) -> bool {
    if let Some(similarity) = cosine_similarity_for_items(left, right) {
        return similarity >= NEAR_DUPLICATE_COSINE_THRESHOLD;
    }
    left.content_fingerprint == right.content_fingerprint
}

fn likely_same_subject(left: &HygieneItemView, right: &HygieneItemView) -> bool {
    if let Some(similarity) = cosine_similarity_for_items(left, right) {
        return similarity >= CONTRADICTION_COSINE_THRESHOLD;
    }
    left.content_fingerprint == right.content_fingerprint
}

fn cosine_similarity_for_items(left: &HygieneItemView, right: &HygieneItemView) -> Option<f64> {
    cosine_similarity(left.embedding.as_deref()?, right.embedding.as_deref()?)
}

fn cosine_similarity(left: &[f32], right: &[f32]) -> Option<f64> {
    if left.is_empty() || left.len() != right.len() {
        return None;
    }
    let mut dot = 0.0f64;
    let mut left_norm = 0.0f64;
    let mut right_norm = 0.0f64;
    for (l, r) in left.iter().zip(right.iter()) {
        let l = f64::from(*l);
        let r = f64::from(*r);
        dot += l * r;
        left_norm += l * l;
        right_norm += r * r;
    }
    if left_norm <= f64::EPSILON || right_norm <= f64::EPSILON {
        return None;
    }
    Some(dot / (left_norm.sqrt() * right_norm.sqrt()))
}

#[derive(Debug, Clone, PartialEq, Eq, Error, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum HygieneError {
    #[error("hygiene action rejected: {code}: {reason}")]
    Rejected { code: String, reason: String },
    #[error("hygiene IO failed: {message}")]
    Io { message: String },
    #[error("invalid hygiene config {field}: {message}")]
    InvalidConfig {
        field: &'static str,
        message: String,
    },
    #[error("hygiene serialization failed: {message}")]
    Serialization { message: String },
    #[error("invalid hygiene action shape {field}: {message}")]
    InvalidShape {
        field: &'static str,
        message: String,
    },
}

/// Adapter from BitemporalIndex to FemsAccessor — used in unit tests.
pub struct InMemoryFemsAccessor<'a> {
    pub index: &'a std::sync::Mutex<BitemporalIndex>,
    pub stats: Vec<HygieneItemView>,
}

impl<'a> FemsAccessor for InMemoryFemsAccessor<'a> {
    fn list_items(&self) -> Result<Vec<HygieneItemView>, HygieneError> {
        Ok(self.stats.clone())
    }

    fn invalidate(&self, memory_id: Uuid, at: DateTime<Utc>) -> Result<(), HygieneError> {
        let mut idx = self.index.lock().unwrap();
        idx.invalidate(memory_id, at);
        Ok(())
    }
}

impl FemsAccessor for PostgresBitemporalMemoryIndex {
    fn list_items(&self) -> Result<Vec<HygieneItemView>, HygieneError> {
        let items =
            block_on_hygiene(self.items_visible_at(&AsOfQuery::now())).map_err(|error| {
                HygieneError::Io {
                    message: error.to_string(),
                }
            })?;
        Ok(items
            .into_iter()
            .map(hygiene_item_from_bitemporal)
            .collect())
    }

    fn invalidate(&self, memory_id: Uuid, at: DateTime<Utc>) -> Result<(), HygieneError> {
        block_on_hygiene(self.invalidate_item(memory_id, at)).map_err(|error| {
            HygieneError::Io {
                message: error.to_string(),
            }
        })?;
        Ok(())
    }
}

fn block_on_hygiene<F: std::future::Future>(future: F) -> F::Output {
    match tokio::runtime::Handle::try_current() {
        Ok(handle) => tokio::task::block_in_place(|| handle.block_on(future)),
        Err(_) => {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("tokio current-thread runtime must build");
            runtime.block_on(future)
        }
    }
}

fn hygiene_item_from_bitemporal(item: super::bitemporal::BitemporalItem) -> HygieneItemView {
    let payload = item.payload;
    let content_fingerprint = payload
        .get("content_fingerprint")
        .and_then(Value::as_u64)
        .unwrap_or_else(|| fingerprint_for_text(memory_payload_text(&payload).as_str()));

    HygieneItemView {
        memory_id: item.item_id,
        recorded_at: item.stamps.recorded_at,
        score: json_f64(&payload, &["score", "confidence"]).unwrap_or(0.0),
        use_count: json_u32(&payload, &["use_count", "uses"]).unwrap_or(0),
        pass_count: json_u32(&payload, &["pass_count", "passes"]).unwrap_or(0),
        pinned: payload
            .get("pinned")
            .or_else(|| payload.get("is_pinned"))
            .and_then(Value::as_bool)
            .unwrap_or(false),
        content_fingerprint,
        embedding: json_f32_vec(&payload, "embedding"),
    }
}

fn memory_payload_text(payload: &Value) -> String {
    ["content", "text", "summary", "title"]
        .into_iter()
        .filter_map(|field| payload.get(field).and_then(Value::as_str))
        .collect::<Vec<_>>()
        .join("\n")
}

fn json_f64(payload: &Value, fields: &[&str]) -> Option<f64> {
    fields
        .iter()
        .filter_map(|field| payload.get(*field).and_then(Value::as_f64))
        .find(|value| value.is_finite())
}

fn json_u32(payload: &Value, fields: &[&str]) -> Option<u32> {
    fields
        .iter()
        .filter_map(|field| payload.get(*field).and_then(Value::as_u64))
        .find_map(|value| u32::try_from(value).ok())
}

fn json_f32_vec(payload: &Value, field: &str) -> Option<Vec<f32>> {
    let values = payload.get(field)?.as_array()?;
    let mut embedding = Vec::with_capacity(values.len());
    for value in values {
        let value = value.as_f64()?;
        if !value.is_finite() {
            return None;
        }
        embedding.push(value as f32);
    }
    if embedding.is_empty() {
        None
    } else {
        Some(embedding)
    }
}

#[allow(dead_code)]
fn _ensure_duration_in_use() {
    // Make sure Duration is referenced so we don't accidentally drop the
    // import in a refactor — Duration semantics are part of the public
    // contract via the older_than_secs field.
    let _ = Duration::from_secs(1);
}

/// Used by tests to construct items.
pub use chrono::Duration as ChronoDuration;

/// Build a deterministic content fingerprint over a memory item's text
/// for the hygiene-tests scenarios. Production wires to FEMS embedding
/// distance.
pub fn fingerprint_for_text(text: &str) -> u64 {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(text.as_bytes());
    let digest = hasher.finalize();
    let mut bytes = [0u8; 8];
    bytes.copy_from_slice(&digest[..8]);
    u64::from_be_bytes(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    struct MockSubmitter {
        consolidations: Mutex<Vec<(Uuid, Uuid)>>,
        prunes: Mutex<Vec<Uuid>>,
        flags: Mutex<Vec<(Uuid, Uuid)>>,
        promotions: Mutex<Vec<ProceduralPromotion>>,
    }
    impl MockSubmitter {
        fn new() -> Self {
            Self {
                consolidations: Mutex::new(Vec::new()),
                prunes: Mutex::new(Vec::new()),
                flags: Mutex::new(Vec::new()),
                promotions: Mutex::new(Vec::new()),
            }
        }
    }
    impl HygieneActionSubmitter for MockSubmitter {
        fn submit_consolidation_candidate(&self, l: Uuid, r: Uuid) -> Result<Uuid, HygieneError> {
            self.consolidations.lock().unwrap().push((l, r));
            Ok(Uuid::now_v7())
        }
        fn submit_prune(&self, id: Uuid, _at: DateTime<Utc>) -> Result<Uuid, HygieneError> {
            self.prunes.lock().unwrap().push(id);
            Ok(Uuid::now_v7())
        }
        fn submit_contradiction_flag(&self, l: Uuid, r: Uuid) -> Result<Uuid, HygieneError> {
            self.flags.lock().unwrap().push((l, r));
            Ok(Uuid::now_v7())
        }
        fn submit_procedural_promotion(
            &self,
            p: ProceduralPromotion,
        ) -> Result<Uuid, HygieneError> {
            self.promotions.lock().unwrap().push(p);
            Ok(Uuid::now_v7())
        }
    }

    fn view(
        id: u128,
        recorded_secs_ago: i64,
        score: f64,
        pinned: bool,
        fp: u64,
    ) -> HygieneItemView {
        HygieneItemView {
            memory_id: Uuid::from_u128(id),
            recorded_at: Utc::now() - chrono::Duration::seconds(recorded_secs_ago),
            score,
            use_count: 10,
            pass_count: 8,
            pinned,
            content_fingerprint: fp,
            embedding: None,
        }
    }

    fn make_runner<'a>(
        sub: &'a MockSubmitter,
        index: &'a Mutex<BitemporalIndex>,
        stats: Vec<HygieneItemView>,
    ) -> InMemoryFemsAccessor<'a> {
        let _ = sub;
        InMemoryFemsAccessor { index, stats }
    }

    #[test]
    fn prune_skips_pinned_items() {
        let sub = MockSubmitter::new();
        let index = Mutex::new(BitemporalIndex::new());
        let stats = vec![
            view(1, 86_400, 0.1, true, 1),
            view(2, 86_400, 0.1, false, 2),
        ];
        let fems = make_runner(&sub, &index, stats);
        let runner = HygieneJobRunner::new(&fems, &sub);
        let cfg = HygieneConfig {
            tasks: vec![HygieneTask::PruneStale {
                older_than_secs: 60,
                min_score: 0.5,
            }],
        };
        let report = runner.run_once(cfg).unwrap();
        assert_eq!(report.tasks[0].items_acted_on, 1);
        let prunes = sub.prunes.lock().unwrap();
        assert_eq!(prunes.len(), 1);
        assert_eq!(prunes[0], Uuid::from_u128(2));
    }

    #[test]
    fn consolidate_emits_candidates_for_identical_fingerprints() {
        let sub = MockSubmitter::new();
        let index = Mutex::new(BitemporalIndex::new());
        let stats = vec![
            view(1, 100, 0.9, false, 42),
            view(2, 100, 0.9, false, 42),
            view(3, 100, 0.9, false, 99),
        ];
        let fems = make_runner(&sub, &index, stats);
        let runner = HygieneJobRunner::new(&fems, &sub);
        let cfg = HygieneConfig {
            tasks: vec![HygieneTask::Consolidate { max_pairs: 10 }],
        };
        let report = runner.run_once(cfg).unwrap();
        assert_eq!(report.tasks[0].items_acted_on, 1);
        let candidates = sub.consolidations.lock().unwrap();
        assert_eq!(candidates.len(), 1);
    }

    #[test]
    fn promote_emits_for_high_use_and_pass_rate() {
        let sub = MockSubmitter::new();
        let index = Mutex::new(BitemporalIndex::new());
        let stats = vec![view(1, 100, 0.9, false, 1)];
        let fems = make_runner(&sub, &index, stats);
        let runner = HygieneJobRunner::new(&fems, &sub);
        let cfg = HygieneConfig {
            tasks: vec![HygieneTask::PromoteProcedural {
                min_use_count: 5,
                min_pass_rate: 0.7,
            }],
        };
        let report = runner.run_once(cfg).unwrap();
        assert_eq!(report.tasks[0].items_acted_on, 1);
        assert_eq!(sub.promotions.lock().unwrap().len(), 1);
    }

    #[test]
    fn flag_contradictions_pairs_divergent_outcomes() {
        let sub = MockSubmitter::new();
        let index = Mutex::new(BitemporalIndex::new());
        let mut a = view(1, 100, 0.9, false, 7);
        a.use_count = 10;
        a.pass_count = 9; // 0.9
        let mut b = view(2, 100, 0.9, false, 7);
        b.use_count = 10;
        b.pass_count = 1; // 0.1
        let stats = vec![a, b];
        let fems = make_runner(&sub, &index, stats);
        let runner = HygieneJobRunner::new(&fems, &sub);
        let cfg = HygieneConfig {
            tasks: vec![HygieneTask::FlagContradictions],
        };
        let report = runner.run_once(cfg).unwrap();
        assert_eq!(report.tasks[0].items_acted_on, 1);
        assert_eq!(sub.flags.lock().unwrap().len(), 1);
    }
}
