//! KB003 promotion gate module tree (MT-040..MT-049, Batch E).
//!
//! Why `kb003_promotion/` and not `promotion/`:
//!
//! The kernel already carries a legacy `kernel::promotion` (KB001 generic
//! promotion gate that decides over a single `ArtifactRecord` and an
//! `OperatorPromotionApproval`). KB003's promotion gate is a different shape:
//! it consumes a `SandboxRunV1`, a `ValidationReport` (Batch D), an
//! `Kb003ArtifactBundleV1` (MT-040), and a typed `OperatorApprovalEvidence`,
//! then emits a `PromotionDecisionV1` + `PromotionReceiptV1` that round-trips
//! through `Kb003Storage` with idempotency on `(idempotency_key,
//! payload_hash)`.
//!
//! Keeping both modules avoids touching KB001 surface during Wave 1 of
//! WP-KERNEL-003 and lets KB001 stay deployable while KB003 lands.
//!
//! Module map:
//!
//! - `artifact_bundle`     (MT-040) — `KbArtifactBundleAssembler` +
//!   `Kb003ArtifactBundleV1` + `Kb003ArtifactHandleV1`.
//! - `decision`            (MT-042/043) — `PromotionDecisionV1` + the typed
//!   rejection-reason taxonomy.
//! - `receipt`             (MT-043/044) — `PromotionReceiptV1` carrying
//!   evidence + idempotency key.
//! - `event_emission`      (MT-045) — typed helpers that turn decisions into
//!   `Kb003EventEnvelope`s for the EventLedger.
//! - `gate`                (MT-046/047/048) — `PromotionGate::evaluate(...)`
//!   the single entry point that callers (operator UI, orchestrator
//!   automation) use. Owns the policy that wires validation outcome +
//!   approval evidence + artifact bundle into a typed decision.
//! - `dcc_promotion_overlay` (MT-049) — populates the `promotion` field of
//!   `DccSandboxProjectionV1` so the operator surface shows the latest gate
//!   outcome without reading the EventLedger.

pub mod artifact_bundle;
pub mod dcc_promotion_overlay;
pub mod decision;
pub mod event_emission;
pub mod gate;
pub mod gate_error_kind;
pub mod receipt;

pub use artifact_bundle::{
    ArtifactBundleError, Kb003ArtifactBundleV1, Kb003ArtifactHandleV1, KbArtifactBundleAssembler,
};
pub use dcc_promotion_overlay::DccPromotionOverlay;
pub use decision::{
    CanonicalRejectionPayload, PromotionDecisionV1, PromotionOutcome, PromotionRejectionReason,
};
pub use event_emission::{
    build_promotion_decided_event, build_promotion_receipt_issued_event,
    build_promotion_rejected_event,
};
pub use gate::{OperatorApprovalEvidence, PromotionGate, PromotionGateError, PromotionGateInputs};
pub use gate_error_kind::{classify_storage_error, NormalisedStorageErrorKind};
pub use receipt::PromotionReceiptV1;
