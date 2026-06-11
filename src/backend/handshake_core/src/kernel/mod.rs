//! # KB003 Sandbox / Validation / Promotion (Research Basis & Module Topology)
//!
//! WP-KERNEL-003 sequences sandbox adapters in product-native tiers; container
//! and microVM stacks are extension slots, never mandatory MVP infrastructure
//! (packet forbids "production-grade VM/container stack as the only supported
//! adapter").
//!
//! ## Adapter sequence (MT-004 research basis)
//!
//! 1. **Process tier** — day-one default. Native Rust child process under
//!    capped permissions (Loom-style); zero external runtime dependency.
//! 2. **HardIsolation tier** — opt-in. Container (Docker/Podman, reset brief
//!    §6.5) or microVM (Firecracker/gVisor) behind a `HardIsolationAdapter`
//!    trait; absence is typed `BLOCKED|UNSUPPORTED`, never silent success
//!    (MT-020).
//! 3. **Workflow tooling** — Dagger / SWE-ReX style harnesses sit above the
//!    adapter trait as orchestration layers, not as the adapter itself.
//!
//! Rejected: container-only or microVM-only MVP (host portability + Windows
//! constraints from reset brief §6.5); raw shell adapter without ToolGate
//! (KB002 conflict register); non-Postgres authority backends (CX-503R).
//!
//! ## Module topology (MT-006 placement decision)
//!
//! New KB003 submodules land in:
//! - `kernel/kb003_schemas.rs` — schema-id constants + `Kb003EventEnvelope` (MT-007/008).
//! - `kernel/kb003_artifact_classes.rs` — artifact class taxonomy (MT-009).
//! - `kernel/sandbox/` — `SandboxRun*`, `SandboxPolicy*`, `SandboxWorkspace*` (MT-010+).
//! - `kernel/validation/` — `ValidationRun*`, descriptors, deterministic checks (MT-030+).
//! - `kernel/promotion/` — `PromotionGate`, `PromotionDecision`, receipts (MT-040+).
//!
//! Storage extensions land in `storage/postgres.rs` (rows + migrations for
//! SandboxRunV1, SandboxArtifactBundleV1, ValidationRunV1, PromotionDecisionV1,
//! PromotionReceiptV1). No non-Postgres authority paths (CX-503R, reset brief §4.1).
//!
//! EventLedger consumption stays through the existing `KernelActor` →
//! `EventLedger` path; KB003 adds new typed event names, not a new ledger.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use thiserror::Error;
use uuid::Uuid;

#[cfg(feature = "runtime-full")]
use crate::storage::ControlPlaneStorageMode;

pub mod action_catalog;
pub mod action_envelope;
pub mod coder_handoff_validation_request;
pub mod context_bundle;
pub mod crdt;
pub mod crdt_adr;
pub mod dcc_kb003_aggregate_summary;
pub mod dcc_kb003_blocked_reasons;
pub mod dcc_kb003_bootstrap_skeleton;
pub mod dcc_kb003_capability_audit;
pub mod dcc_kb003_console_network_evidence;
pub mod dcc_kb003_debug_bundle_bridge;
pub mod dcc_kb003_dropback;
pub mod dcc_kb003_evidence_portability;
pub mod dcc_kb003_lane_wake;
pub mod dcc_kb003_mex_evidence;
pub mod dcc_kb003_model_manual_hints;
pub mod dcc_kb003_mt_summary;
pub mod dcc_kb003_promotion_control_state;
pub mod dcc_kb003_retry_budget;
pub mod dcc_kb003_rollup;
pub mod dcc_kb003_run_detail;
pub mod dcc_kb003_sandbox_run_list;
pub mod dcc_kb003_visual_validation_gate;
pub mod dcc_layout_projection_registry;
pub mod dcc_mvp_runtime_surface;
pub mod dcc_structured_artifact_viewer;
pub mod direct_edit_guard;
pub mod fems_memory_poisoning_drift_guardrails;
pub mod fems_mt_handoff_memory_context;
pub mod fems_working_memory_checkpoint;
pub mod fems_write_time_safeguards;
pub mod fold_manifest;
pub mod generated_documentation_status_projection;
pub mod git_engine_decision_gate;
pub mod governance_overlay_boundary;
pub mod governance_pack_instantiation;
pub mod kb003_artifact_classes;
pub mod kb003_promotion;
pub mod kb003_schemas;
pub mod local_first_mcp_posture;
pub mod local_model_microtask_loop;
pub mod locus_mt_validation_work_graph;
pub mod locus_work_tracking_reset;
pub mod markdown_mirror_sync_drift_guard;
pub mod mechanical_contract_generation;
pub mod mirror_advisory;
pub mod model_adapter;
pub mod model_manual;
pub mod mt_loop_scheduler_contract;
pub mod mte_aggregate_summary;
pub mod mte_authority_mutation_boundary;
pub mod mte_blocked_taxonomy;
pub mod mte_closeout_bundle;
pub mod mte_drop_back;
pub mod mte_idempotency_enforcement;
pub mod mte_lane_settlement;
pub mod mte_per_mt_summary;
pub mod mte_resource_caps;
pub mod mte_retry_budget;
pub mod mte_validation_report_projection;
pub mod overlay_coordination_records;
pub mod overlay_lifecycle_recovery;
pub mod postgres_control_plane_residual;
pub mod pre_use_kernel_acceptance_run;
pub mod product_screenshot_capture;
#[cfg(feature = "runtime-full")]
pub mod promotion;
#[cfg(feature = "runtime-full")]
pub mod proof;
pub mod remediation_work_generation_contract;
pub mod reset_invariants;
pub mod role_mailbox_claim_lease;
pub mod role_mailbox_contract;
pub mod role_mailbox_handoff_bundle;
pub mod role_mailbox_inbox_evidence_bridge;
pub mod role_mailbox_loop_control;
pub mod role_mailbox_triage_queue;
pub mod role_turn_isolation;
pub mod sandbox;
pub mod session_anti_pattern_registry;
pub mod session_broker;
pub mod session_spawn_conversation_distillation;
pub mod session_spawn_tree_dcc;
pub mod software_delivery_runtime_truth;
pub mod task_contract_lifecycle;
#[cfg(feature = "runtime-full")]
pub mod trace_projection;
pub mod validation;
pub mod validator_finding_report_contract;
pub mod validator_verdict_mediation_contract;
pub mod visual_debugging_loop;
pub mod visual_diff_baseline;
pub mod work_packet_full_detail_authority;
pub mod work_profiles;
pub mod workflow_transition_registry;
pub mod write_boxes;

pub use context_bundle::*;
pub use dcc_mvp_runtime_surface::DccMvpRuntimeSurfaceV1;
pub use model_adapter::*;
pub use pre_use_kernel_acceptance_run::build_pre_use_dcc_mvp_runtime_surface;
#[cfg(feature = "runtime-full")]
pub use promotion::*;
#[cfg(feature = "runtime-full")]
pub use proof::*;
pub use session_broker::*;
#[cfg(feature = "runtime-full")]
pub use trace_projection::*;

#[derive(Debug, Error)]
pub enum KernelError {
    #[cfg(feature = "runtime-full")]
    #[error("Kernel V1 authority requires PostgresPrimary storage mode, got {mode}")]
    NonPostgresAuthority { mode: ControlPlaneStorageMode },
    #[error("invalid kernel event: {0}")]
    InvalidEvent(&'static str),
    #[error("invalid kernel event type: {0}")]
    InvalidEventType(String),
    #[error("invalid session transition from {from} to {to}")]
    InvalidSessionTransition { from: String, to: String },
    #[error("kernel storage error: {0}")]
    Storage(String),
    #[error("kernel artifact error: {0}")]
    Artifact(String),
    #[error("kernel flight recorder error: {0}")]
    FlightRecorder(String),
    #[error("kernel model runtime error: {0}")]
    ModelRuntime(String),
}

pub type KernelResult<T> = Result<T, KernelError>;

#[cfg(feature = "runtime-full")]
impl From<crate::storage::StorageError> for KernelError {
    fn from(value: crate::storage::StorageError) -> Self {
        Self::Storage(value.to_string())
    }
}

#[cfg(feature = "runtime-full")]
impl From<crate::storage::artifacts::ArtifactError> for KernelError {
    fn from(value: crate::storage::artifacts::ArtifactError) -> Self {
        Self::Artifact(value.to_string())
    }
}

#[cfg(feature = "runtime-full")]
pub fn assert_kernel_authority_storage_mode(mode: ControlPlaneStorageMode) -> KernelResult<()> {
    if mode.is_control_plane_authority() {
        Ok(())
    } else {
        Err(KernelError::NonPostgresAuthority { mode })
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum KernelEventType {
    TaskIntentRecorded,
    SessionQueued,
    SessionClaimed,
    SessionStarted,
    SessionCompleted,
    SessionFailed,
    SessionCancelled,
    SessionBackpressureDelayed,
    SessionRetryScheduled,
    SessionDeadLettered,
    ContextBundleRecorded,
    ModelAdapterInvoked,
    ModelResponseRecorded,
    ToolRequestRecorded,
    ToolDecisionRecorded,
    ToolResultRecorded,
    ArtifactProposed,
    ArtifactStored,
    ValidationRecorded,
    PromotionDecided,
    PromotionRequested,
    PromotionAccepted,
    PromotionRejected,
    FlightRecorderMirrorRecorded,
    TraceReplayed,
    InspectorReplayDrive,
    FrEvtLedgerOverflow,
    HbrHandoffGate,
    AtelierDomainEventRecorded,
    // WP-KERNEL-009 CRDTAndConcurrencyCore (MT-065..MT-080) event families.
    // Spec 2.3.13.11: CRDT draft state, graph mutation proposals, AI edit
    // proposals, leases, and recovery receipts MUST leave EventLedger
    // receipts; these variants are the typed event names for that surface.
    KnowledgeCrdtUpdateRecorded,
    KnowledgeCrdtSnapshotRecorded,
    KnowledgeCrdtConflictDetected,
    KnowledgeCrdtLeaseClaimed,
    KnowledgeCrdtLeaseRenewed,
    KnowledgeCrdtLeaseReleased,
    KnowledgeCrdtLeaseExpired,
    KnowledgeCrdtLeaseTakenOver,
    KnowledgeCrdtLeaseWriteDenied,
    KnowledgeCrdtCheckpointRecorded,
    KnowledgeCrdtRecoveryReceiptRecorded,
    GraphMutationProposalRecorded,
    GraphMutationProposalDecided,
    AiEditProposalRecorded,
    AiEditProposalDecided,
    // WP-KERNEL-009 PostgresEventLedgerCore (MT-061) event families.
    // Spec 2.3.13.11: index runs, claim lifecycle/conflicts, retrieval
    // traces, editor (RichDocument) saves/promotions, Loom blocks,
    // projections, UserManual entries, and knowledge validation MUST leave
    // EventLedger receipts; the knowledge_* receipt FK columns
    // (migrations 0133/0137/0138/0139/0140/0141) target events of these
    // families.
    KnowledgeIndexRunStarted,
    KnowledgeIndexRunCompleted,
    KnowledgeIndexRunFailed,
    KnowledgeIndexRunCancelled,
    KnowledgeClaimProposed,
    KnowledgeClaimAccepted,
    KnowledgeClaimRetired,
    KnowledgeClaimConflictDetected,
    KnowledgeClaimConflictResolved,
    KnowledgeRetrievalTraceRecorded,
    KnowledgeRichDocumentSaved,
    KnowledgeRichDocumentPromoted,
    KnowledgeLoomBlockIndexed,
    KnowledgeProjectionRebuilt,
    KnowledgeUserManualEntryRecorded,
    KnowledgeValidationRecorded,
}

impl KernelEventType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::TaskIntentRecorded => "TASK_INTENT_RECORDED",
            Self::SessionQueued => "SESSION_QUEUED",
            Self::SessionClaimed => "SESSION_CLAIMED",
            Self::SessionStarted => "SESSION_STARTED",
            Self::SessionCompleted => "SESSION_COMPLETED",
            Self::SessionFailed => "SESSION_FAILED",
            Self::SessionCancelled => "SESSION_CANCELLED",
            Self::SessionBackpressureDelayed => "SESSION_BACKPRESSURE_DELAYED",
            Self::SessionRetryScheduled => "SESSION_RETRY_SCHEDULED",
            Self::SessionDeadLettered => "SESSION_DEAD_LETTERED",
            Self::ContextBundleRecorded => "CONTEXT_BUNDLE_RECORDED",
            Self::ModelAdapterInvoked => "MODEL_ADAPTER_INVOKED",
            Self::ModelResponseRecorded => "MODEL_RESPONSE_RECORDED",
            Self::ToolRequestRecorded => "TOOL_REQUEST_RECORDED",
            Self::ToolDecisionRecorded => "TOOL_DECISION_RECORDED",
            Self::ToolResultRecorded => "TOOL_RESULT_RECORDED",
            Self::ArtifactProposed => "ARTIFACT_PROPOSED",
            Self::ArtifactStored => "ARTIFACT_STORED",
            Self::ValidationRecorded => "VALIDATION_RECORDED",
            Self::PromotionDecided => "PROMOTION_DECIDED",
            Self::PromotionRequested => "PROMOTION_REQUESTED",
            Self::PromotionAccepted => "PROMOTION_ACCEPTED",
            Self::PromotionRejected => "PROMOTION_REJECTED",
            Self::FlightRecorderMirrorRecorded => "FLIGHT_RECORDER_MIRROR_RECORDED",
            Self::TraceReplayed => "TRACE_REPLAYED",
            Self::InspectorReplayDrive => "INSPECTOR_REPLAY_DRIVE",
            Self::FrEvtLedgerOverflow => "FR_EVT_LEDGER_OVERFLOW",
            Self::HbrHandoffGate => "HBR_HANDOFF_GATE",
            Self::AtelierDomainEventRecorded => "ATELIER_DOMAIN_EVENT_RECORDED",
            Self::KnowledgeCrdtUpdateRecorded => "KNOWLEDGE_CRDT_UPDATE_RECORDED",
            Self::KnowledgeCrdtSnapshotRecorded => "KNOWLEDGE_CRDT_SNAPSHOT_RECORDED",
            Self::KnowledgeCrdtConflictDetected => "KNOWLEDGE_CRDT_CONFLICT_DETECTED",
            Self::KnowledgeCrdtLeaseClaimed => "KNOWLEDGE_CRDT_LEASE_CLAIMED",
            Self::KnowledgeCrdtLeaseRenewed => "KNOWLEDGE_CRDT_LEASE_RENEWED",
            Self::KnowledgeCrdtLeaseReleased => "KNOWLEDGE_CRDT_LEASE_RELEASED",
            Self::KnowledgeCrdtLeaseExpired => "KNOWLEDGE_CRDT_LEASE_EXPIRED",
            Self::KnowledgeCrdtLeaseTakenOver => "KNOWLEDGE_CRDT_LEASE_TAKEN_OVER",
            Self::KnowledgeCrdtLeaseWriteDenied => "KNOWLEDGE_CRDT_LEASE_WRITE_DENIED",
            Self::KnowledgeCrdtCheckpointRecorded => "KNOWLEDGE_CRDT_CHECKPOINT_RECORDED",
            Self::KnowledgeCrdtRecoveryReceiptRecorded => {
                "KNOWLEDGE_CRDT_RECOVERY_RECEIPT_RECORDED"
            }
            Self::GraphMutationProposalRecorded => "GRAPH_MUTATION_PROPOSAL_RECORDED",
            Self::GraphMutationProposalDecided => "GRAPH_MUTATION_PROPOSAL_DECIDED",
            Self::AiEditProposalRecorded => "AI_EDIT_PROPOSAL_RECORDED",
            Self::AiEditProposalDecided => "AI_EDIT_PROPOSAL_DECIDED",
            Self::KnowledgeIndexRunStarted => "KNOWLEDGE_INDEX_RUN_STARTED",
            Self::KnowledgeIndexRunCompleted => "KNOWLEDGE_INDEX_RUN_COMPLETED",
            Self::KnowledgeIndexRunFailed => "KNOWLEDGE_INDEX_RUN_FAILED",
            Self::KnowledgeIndexRunCancelled => "KNOWLEDGE_INDEX_RUN_CANCELLED",
            Self::KnowledgeClaimProposed => "KNOWLEDGE_CLAIM_PROPOSED",
            Self::KnowledgeClaimAccepted => "KNOWLEDGE_CLAIM_ACCEPTED",
            Self::KnowledgeClaimRetired => "KNOWLEDGE_CLAIM_RETIRED",
            Self::KnowledgeClaimConflictDetected => "KNOWLEDGE_CLAIM_CONFLICT_DETECTED",
            Self::KnowledgeClaimConflictResolved => "KNOWLEDGE_CLAIM_CONFLICT_RESOLVED",
            Self::KnowledgeRetrievalTraceRecorded => "KNOWLEDGE_RETRIEVAL_TRACE_RECORDED",
            Self::KnowledgeRichDocumentSaved => "KNOWLEDGE_RICH_DOCUMENT_SAVED",
            Self::KnowledgeRichDocumentPromoted => "KNOWLEDGE_RICH_DOCUMENT_PROMOTED",
            Self::KnowledgeLoomBlockIndexed => "KNOWLEDGE_LOOM_BLOCK_INDEXED",
            Self::KnowledgeProjectionRebuilt => "KNOWLEDGE_PROJECTION_REBUILT",
            Self::KnowledgeUserManualEntryRecorded => "KNOWLEDGE_USER_MANUAL_ENTRY_RECORDED",
            Self::KnowledgeValidationRecorded => "KNOWLEDGE_VALIDATION_RECORDED",
        }
    }

    pub fn required_first_slice_events() -> &'static [KernelEventType] {
        &[
            KernelEventType::TaskIntentRecorded,
            KernelEventType::SessionQueued,
            KernelEventType::SessionClaimed,
            KernelEventType::SessionStarted,
            KernelEventType::SessionCompleted,
            KernelEventType::SessionFailed,
            KernelEventType::SessionCancelled,
            KernelEventType::SessionBackpressureDelayed,
            KernelEventType::SessionRetryScheduled,
            KernelEventType::SessionDeadLettered,
            KernelEventType::ContextBundleRecorded,
            KernelEventType::ModelAdapterInvoked,
            KernelEventType::ModelResponseRecorded,
            KernelEventType::ToolRequestRecorded,
            KernelEventType::ToolDecisionRecorded,
            KernelEventType::ToolResultRecorded,
            KernelEventType::ArtifactProposed,
            KernelEventType::ArtifactStored,
            KernelEventType::ValidationRecorded,
            KernelEventType::PromotionDecided,
            KernelEventType::PromotionRequested,
            KernelEventType::PromotionAccepted,
            KernelEventType::PromotionRejected,
            KernelEventType::FlightRecorderMirrorRecorded,
            KernelEventType::TraceReplayed,
            KernelEventType::InspectorReplayDrive,
            KernelEventType::FrEvtLedgerOverflow,
            KernelEventType::HbrHandoffGate,
            KernelEventType::AtelierDomainEventRecorded,
            KernelEventType::KnowledgeCrdtUpdateRecorded,
            KernelEventType::KnowledgeCrdtSnapshotRecorded,
            KernelEventType::KnowledgeCrdtConflictDetected,
            KernelEventType::KnowledgeCrdtLeaseClaimed,
            KernelEventType::KnowledgeCrdtLeaseRenewed,
            KernelEventType::KnowledgeCrdtLeaseReleased,
            KernelEventType::KnowledgeCrdtLeaseExpired,
            KernelEventType::KnowledgeCrdtLeaseTakenOver,
            KernelEventType::KnowledgeCrdtLeaseWriteDenied,
            KernelEventType::KnowledgeCrdtCheckpointRecorded,
            KernelEventType::KnowledgeCrdtRecoveryReceiptRecorded,
            KernelEventType::GraphMutationProposalRecorded,
            KernelEventType::GraphMutationProposalDecided,
            KernelEventType::AiEditProposalRecorded,
            KernelEventType::AiEditProposalDecided,
            KernelEventType::KnowledgeIndexRunStarted,
            KernelEventType::KnowledgeIndexRunCompleted,
            KernelEventType::KnowledgeIndexRunFailed,
            KernelEventType::KnowledgeIndexRunCancelled,
            KernelEventType::KnowledgeClaimProposed,
            KernelEventType::KnowledgeClaimAccepted,
            KernelEventType::KnowledgeClaimRetired,
            KernelEventType::KnowledgeClaimConflictDetected,
            KernelEventType::KnowledgeClaimConflictResolved,
            KernelEventType::KnowledgeRetrievalTraceRecorded,
            KernelEventType::KnowledgeRichDocumentSaved,
            KernelEventType::KnowledgeRichDocumentPromoted,
            KernelEventType::KnowledgeLoomBlockIndexed,
            KernelEventType::KnowledgeProjectionRebuilt,
            KernelEventType::KnowledgeUserManualEntryRecorded,
            KernelEventType::KnowledgeValidationRecorded,
        ]
    }
}

impl TryFrom<&str> for KernelEventType {
    type Error = KernelError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let normalized = value.trim().to_ascii_uppercase();
        for event_type in Self::required_first_slice_events() {
            if event_type.as_str() == normalized {
                return Ok(event_type.clone());
            }
        }
        Err(KernelError::InvalidEventType(value.to_string()))
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", content = "id", rename_all = "snake_case")]
pub enum KernelActor {
    Operator(String),
    System(String),
    SessionBroker(String),
    ModelAdapter(String),
    ToolGate(String),
    ValidationRunner(String),
    PromotionGate(String),
}

impl KernelActor {
    pub fn actor_kind(&self) -> &'static str {
        match self {
            Self::Operator(_) => "operator",
            Self::System(_) => "system",
            Self::SessionBroker(_) => "session_broker",
            Self::ModelAdapter(_) => "model_adapter",
            Self::ToolGate(_) => "toolgate",
            Self::ValidationRunner(_) => "validation_runner",
            Self::PromotionGate(_) => "promotion_gate",
        }
    }

    pub fn actor_id(&self) -> &str {
        match self {
            Self::Operator(id)
            | Self::System(id)
            | Self::SessionBroker(id)
            | Self::ModelAdapter(id)
            | Self::ToolGate(id)
            | Self::ValidationRunner(id)
            | Self::PromotionGate(id) => id,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct NewKernelEvent {
    pub event_version: String,
    pub kernel_task_run_id: String,
    pub session_run_id: String,
    pub aggregate_type: String,
    pub aggregate_id: String,
    pub idempotency_key: String,
    pub event_type: KernelEventType,
    pub actor: KernelActor,
    pub causation_id: Option<String>,
    pub correlation_id: Option<String>,
    pub payload_hash: String,
    pub source_component: String,
    pub payload: Value,
}

impl NewKernelEvent {
    pub fn builder(
        kernel_task_run_id: impl Into<String>,
        session_run_id: impl Into<String>,
        event_type: KernelEventType,
        actor: KernelActor,
    ) -> NewKernelEventBuilder {
        NewKernelEventBuilder {
            kernel_task_run_id: kernel_task_run_id.into(),
            session_run_id: session_run_id.into(),
            aggregate_type: None,
            aggregate_id: None,
            idempotency_key: None,
            event_version: "kernel_event_v1".to_string(),
            event_type,
            actor,
            causation_id: None,
            correlation_id: None,
            source_component: None,
            payload: json!({}),
        }
    }

    pub fn validate(&self) -> KernelResult<()> {
        if self.event_version.trim().is_empty() {
            return Err(KernelError::InvalidEvent("event_version is required"));
        }
        if self.kernel_task_run_id.trim().is_empty() {
            return Err(KernelError::InvalidEvent("kernel_task_run_id is required"));
        }
        if self.session_run_id.trim().is_empty() {
            return Err(KernelError::InvalidEvent("session_run_id is required"));
        }
        if self.aggregate_type.trim().is_empty() {
            return Err(KernelError::InvalidEvent("aggregate_type is required"));
        }
        if self.aggregate_id.trim().is_empty() {
            return Err(KernelError::InvalidEvent("aggregate_id is required"));
        }
        if self.idempotency_key.trim().is_empty() {
            return Err(KernelError::InvalidEvent("idempotency_key is required"));
        }
        if !is_sha256_hex(self.payload_hash.as_str()) {
            return Err(KernelError::InvalidEvent(
                "payload_hash must be a sha256 hex digest",
            ));
        }
        if self.source_component.trim().is_empty() {
            return Err(KernelError::InvalidEvent("source_component is required"));
        }
        Ok(())
    }
}

pub struct NewKernelEventBuilder {
    kernel_task_run_id: String,
    session_run_id: String,
    aggregate_type: Option<String>,
    aggregate_id: Option<String>,
    idempotency_key: Option<String>,
    event_version: String,
    event_type: KernelEventType,
    actor: KernelActor,
    causation_id: Option<String>,
    correlation_id: Option<String>,
    source_component: Option<String>,
    payload: Value,
}

impl NewKernelEventBuilder {
    pub fn aggregate(
        mut self,
        aggregate_type: impl Into<String>,
        aggregate_id: impl Into<String>,
    ) -> Self {
        self.aggregate_type = Some(aggregate_type.into());
        self.aggregate_id = Some(aggregate_id.into());
        self
    }

    pub fn idempotency_key(mut self, idempotency_key: impl Into<String>) -> Self {
        self.idempotency_key = Some(idempotency_key.into());
        self
    }

    pub fn event_version(mut self, event_version: impl Into<String>) -> Self {
        self.event_version = event_version.into();
        self
    }

    pub fn causation_id(mut self, causation_id: impl Into<String>) -> Self {
        self.causation_id = Some(causation_id.into());
        self
    }

    pub fn correlation_id(mut self, correlation_id: impl Into<String>) -> Self {
        self.correlation_id = Some(correlation_id.into());
        self
    }

    pub fn payload(mut self, payload: Value) -> Self {
        self.payload = payload;
        self
    }

    pub fn source_component(mut self, source_component: impl Into<String>) -> Self {
        self.source_component = Some(source_component.into());
        self
    }

    pub fn build(self) -> KernelResult<NewKernelEvent> {
        let aggregate_type = self
            .aggregate_type
            .unwrap_or_else(|| "session_run".to_string());
        let aggregate_id = self
            .aggregate_id
            .unwrap_or_else(|| self.session_run_id.clone());
        let idempotency_key = self
            .idempotency_key
            .unwrap_or_else(|| format!("KEI-{}", Uuid::now_v7()));
        let source_component = self
            .source_component
            .unwrap_or_else(|| self.actor.actor_kind().to_string());
        let payload_hash = payload_hash(&self.payload);
        let event = NewKernelEvent {
            event_version: self.event_version,
            kernel_task_run_id: self.kernel_task_run_id,
            session_run_id: self.session_run_id,
            aggregate_type,
            aggregate_id,
            idempotency_key,
            event_type: self.event_type,
            actor: self.actor,
            causation_id: self.causation_id,
            correlation_id: self.correlation_id,
            payload_hash,
            source_component,
            payload: self.payload,
        };
        event.validate()?;
        Ok(event)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct KernelEvent {
    pub event_id: String,
    pub event_sequence: i64,
    pub event_version: String,
    pub kernel_task_run_id: String,
    pub session_run_id: String,
    pub aggregate_type: String,
    pub aggregate_id: String,
    pub idempotency_key: String,
    pub event_type: KernelEventType,
    pub actor: KernelActor,
    pub causation_id: Option<String>,
    pub correlation_id: Option<String>,
    pub payload_hash: String,
    pub source_component: String,
    pub payload: Value,
    pub created_at: DateTime<Utc>,
}

impl KernelEvent {
    pub fn from_new(event: NewKernelEvent) -> Self {
        Self {
            event_id: format!("KE-{}", Uuid::now_v7()),
            event_sequence: 0,
            event_version: event.event_version,
            kernel_task_run_id: event.kernel_task_run_id,
            session_run_id: event.session_run_id,
            aggregate_type: event.aggregate_type,
            aggregate_id: event.aggregate_id,
            idempotency_key: event.idempotency_key,
            event_type: event.event_type,
            actor: event.actor,
            causation_id: event.causation_id,
            correlation_id: event.correlation_id,
            payload_hash: event.payload_hash,
            source_component: event.source_component,
            payload: event.payload,
            created_at: Utc::now(),
        }
    }
}

#[cfg(feature = "runtime-full")]
pub fn flight_recorder_mirror_event(
    event: &KernelEvent,
) -> crate::flight_recorder::FlightRecorderEvent {
    crate::flight_recorder::FlightRecorderEvent::new(
        crate::flight_recorder::FlightRecorderEventType::Diagnostic,
        crate::flight_recorder::FlightRecorderActor::System,
        Uuid::now_v7(),
        json!({
            "diagnostic_id": "kernel_event_mirror",
            "authority_source": "postgres_event_ledger",
            "projection_only": true,
            "kernel_event_id": event.event_id,
            "kernel_event_sequence": event.event_sequence,
            "kernel_event_type": event.event_type.as_str(),
            "kernel_event_version": event.event_version,
            "kernel_task_run_id": event.kernel_task_run_id,
            "session_run_id": event.session_run_id,
            "aggregate_type": event.aggregate_type,
            "aggregate_id": event.aggregate_id,
            "payload_hash": event.payload_hash,
            "source_component": event.source_component,
            "causation_id": event.causation_id,
            "correlation_id": event.correlation_id,
            "idempotency_key": event.idempotency_key,
            "actor_kind": event.actor.actor_kind(),
            "actor_id": event.actor.actor_id()
        }),
    )
    .with_model_session_id(event.session_run_id.clone())
}

fn payload_hash(payload: &Value) -> String {
    context_bundle::sha256_hex(&context_bundle::canonical_json_bytes(payload))
}

fn is_sha256_hex(value: &str) -> bool {
    value.len() == 64 && value.chars().all(|character| character.is_ascii_hexdigit())
}
