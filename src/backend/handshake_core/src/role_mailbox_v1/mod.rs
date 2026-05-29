//! WP-KERNEL-004 cluster X.1 (MT-176..MT-183) RoleMailbox V1.
//!
//! This module layers the cluster-X durable Role Mailbox primitives on top of
//! the legacy KERNEL-002 `crate::role_mailbox` projection layer. The legacy
//! module remains the operator-facing DuckDB export surface; this module owns
//! the authoritative Postgres-backed thread + message + lease + handoff state
//! per spec v02.186 §02-system-architecture.md role mailbox subsection [ADD
//! v02.173] and [ADD v02.176].
//!
//! Cluster X.1 MTs implemented here:
//!  - MT-176: `thread` + `message` + `lifecycle` typed primitives
//!  - MT-177: `repo` (Postgres-backed transactional CRUD)
//!  - MT-178: `exporter` (deterministic repo export)
//!  - MT-179: `families` (10 Phase-1 message family payloads)
//!  - MT-180: `lease` (RoleMailboxClaimLeaseV1 + LeaseManager)
//!  - MT-181: `router` (executor routing decision surface)
//!  - MT-182: `backpressure` (per-role inbox cap + rate-limit)
//!  - MT-183: `handoff` (handoff bundle + announce-back provenance)
//!
//! Per HBR-INT-008 every Uuid mint site uses `Uuid::now_v7()`.

pub mod backpressure;
pub mod exporter;
pub mod families;
pub mod handoff;
pub mod lease;
pub mod lifecycle;
pub mod message;
pub mod repo;
pub mod router;
pub mod thread;

pub use backpressure::{
    BackpressureClock, BackpressureConfig, BackpressureDecision, BackpressureError,
    BackpressureGuard, BackpressureReceipt, DenyReason as BackpressureDenyReason, FixedClock,
    SystemClock,
};
pub use exporter::{ExportReport, MailboxExporter, MailboxExporterConfig};
pub use families::{
    AnnounceBackBody, ArtifactPointer, BlockerBody, BlockerSeverity, CapabilityGrant,
    CompletionState, DecisionAuthority, DecisionOption, DecisionRequestBody, DelegateWorkBody,
    EscalationTier as FamilyEscalationTier, EvidencePointer, ExpectedResponse, FamilyError,
    MessageFamily, MicroTaskCompletionReportBody, MicroTaskEscalationBody,
    MicroTaskExecutorContractRef, MicroTaskFeedbackBody, MicroTaskRef, MicroTaskRequestBody,
    MicroTaskVerificationNeededBody, PriorAttemptRef, ReviewKind, ReviewRequestBody, ReviewTarget,
    MAX_FAMILY_PAYLOAD_BYTES,
};
pub use handoff::{
    AnnounceBackComposer, ChainError, HandoffBundleBuilder, MailboxHandoffBundleV1, ProvenanceLink,
    TranscriptPointer, MAX_PROVENANCE_CHAIN_DEPTH,
};
pub use lease::{LeaseError, LeaseManager, LeaseRequest, RoleMailboxClaimLeaseV1, TakeoverPolicy};
pub use lifecycle::{
    transition_message_state, transition_thread_state, InvalidTransition, MessageDeliveryState,
    ThreadLifecycleState,
};
pub use message::{MessageType, RoleMailboxMessage, RoleMailboxMessageId};
pub use repo::{MailboxError, RoleMailboxRepository};
pub use router::{ExecutorIdentity, ExecutorKind, ExecutorRouter, RouteDecision, RouteReason};
pub use thread::{
    ClaimMode, LinkedRecordKind, ResponseAuthorityScope, RoleMailboxThread, RoleMailboxThreadId,
};
