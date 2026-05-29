//! MT-057: MTE Authority Mutation Boundary.
//!
//! Acceptance (MT-057.json): "Authority Mutation Boundary: sandbox and
//! validation cannot mutate authority except through PromotionGate.
//! Acceptance: direct mutation attempt produces denial evidence."
//!
//! The MTE-layer boundary check guards the boundary between the *sandbox /
//! validation* tier (which produces evidence) and the *authority* tier (which
//! the operator-facing surfaces depend on). Only `PromotionGate` (Batch E)
//! may write authority rows. Any other actor that attempts a mutation must
//! be refused at this boundary with a typed `AuthorityBoundaryDenialV1`
//! record.
//!
//! Design note: this module is the *contract* for the boundary. The actual
//! storage writes are routed through `kb003_promotion::gate::PromotionGate`;
//! this file provides the typed actor enum, the typed denial record, and a
//! check function that the validation runner and sandbox adapter can call
//! before they ever try to write authority data.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Actor that is attempting a mutation. Promotion gate is the only allowed
/// authority writer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AuthorityMutationActor {
    /// `kb003_promotion::gate::PromotionGate` — the only allowed actor.
    PromotionGate,
    /// `kernel::sandbox::run` — sandbox adapter; must not write authority.
    SandboxAdapter,
    /// `kernel::validation::run` — validation runner; must not write authority.
    ValidationRunner,
    /// `kernel::mte_*` — MTE scheduler; must not write authority directly.
    MteScheduler,
    /// Coder role session — must not write authority directly. H2.
    Coder,
    /// Validator role session — must not write authority directly. H2.
    Validator,
    /// DCC projection-rebuild path — must not write authority directly. H2.
    DccProjection,
    /// External caller from outside the kernel — must not write authority. H2.
    ExternalActor,
    /// External / unknown actor — must not write authority.
    UnknownActor,
}

impl AuthorityMutationActor {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::PromotionGate => "PROMOTION_GATE",
            Self::SandboxAdapter => "SANDBOX_ADAPTER",
            Self::ValidationRunner => "VALIDATION_RUNNER",
            Self::MteScheduler => "MTE_SCHEDULER",
            Self::Coder => "CODER",
            Self::Validator => "VALIDATOR",
            Self::DccProjection => "DCC_PROJECTION",
            Self::ExternalActor => "EXTERNAL_ACTOR",
            Self::UnknownActor => "UNKNOWN_ACTOR",
        }
    }

    pub fn is_allowed_authority_writer(&self) -> bool {
        matches!(self, Self::PromotionGate)
    }
}

/// Authority-row classes the boundary protects.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AuthorityRowClass {
    PromotionDecision,
    PromotionReceipt,
    SandboxRunRecord,
    ValidationRunRecord,
    SandboxPolicyRecord,
}

impl AuthorityRowClass {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::PromotionDecision => "PROMOTION_DECISION",
            Self::PromotionReceipt => "PROMOTION_RECEIPT",
            Self::SandboxRunRecord => "SANDBOX_RUN_RECORD",
            Self::ValidationRunRecord => "VALIDATION_RUN_RECORD",
            Self::SandboxPolicyRecord => "SANDBOX_POLICY_RECORD",
        }
    }

    /// True when this class is gate-only writable. PromotionDecision and
    /// PromotionReceipt are gate-only; the others may be written by their
    /// owning subsystem (sandbox adapter writes its own run records, etc).
    pub fn is_gate_only(&self) -> bool {
        matches!(self, Self::PromotionDecision | Self::PromotionReceipt)
    }
}

/// Typed denial record emitted when a non-gate actor attempts a gate-only
/// mutation. Persistable and replayable. Schema id
/// `hsk.kernel.mte.authority_boundary_denial@1`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuthorityBoundaryDenialV1 {
    pub schema_version: String,
    pub denial_id: String,
    pub actor: AuthorityMutationActor,
    pub row_class: AuthorityRowClass,
    pub attempted_op: String,
    pub rationale: String,
    pub denied_at_utc: DateTime<Utc>,
}

impl AuthorityBoundaryDenialV1 {
    pub const SCHEMA_VERSION: &'static str = "hsk.kernel.mte.authority_boundary_denial@1";

    pub fn new(
        actor: AuthorityMutationActor,
        row_class: AuthorityRowClass,
        attempted_op: impl Into<String>,
        rationale: impl Into<String>,
    ) -> Self {
        Self {
            schema_version: Self::SCHEMA_VERSION.to_string(),
            denial_id: format!("MTE-ABD-{}", uuid::Uuid::now_v7()),
            actor,
            row_class,
            attempted_op: attempted_op.into(),
            rationale: rationale.into(),
            denied_at_utc: Utc::now(),
        }
    }
}

/// Verdict of the boundary check.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AuthorityBoundaryVerdict {
    Allow,
    Deny { denial: AuthorityBoundaryDenialV1 },
}

impl AuthorityBoundaryVerdict {
    pub fn is_allowed(&self) -> bool {
        matches!(self, Self::Allow)
    }
}

pub struct AuthorityBoundaryCheck;

impl AuthorityBoundaryCheck {
    /// Decide whether `actor` may write `row_class`. Returns a typed denial
    /// when the actor is not allowed.
    pub fn evaluate(
        actor: AuthorityMutationActor,
        row_class: AuthorityRowClass,
        attempted_op: impl Into<String>,
    ) -> AuthorityBoundaryVerdict {
        let attempted_op = attempted_op.into();
        if !row_class.is_gate_only() {
            // Non-gate-only rows: owning subsystem writes them; this boundary
            // does not interfere.
            return AuthorityBoundaryVerdict::Allow;
        }
        if actor.is_allowed_authority_writer() {
            return AuthorityBoundaryVerdict::Allow;
        }
        AuthorityBoundaryVerdict::Deny {
            denial: AuthorityBoundaryDenialV1::new(
                actor,
                row_class,
                attempted_op,
                format!(
                    "actor {} cannot mutate {} (gate-only); route through PromotionGate",
                    actor.as_str(),
                    row_class.as_str()
                ),
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // MT-057 acceptance: direct mutation attempt produces denial evidence.
    #[test]
    fn sandbox_attempting_promotion_receipt_write_is_denied() {
        let v = AuthorityBoundaryCheck::evaluate(
            AuthorityMutationActor::SandboxAdapter,
            AuthorityRowClass::PromotionReceipt,
            "insert_promotion_receipt",
        );
        match v {
            AuthorityBoundaryVerdict::Deny { denial } => {
                assert_eq!(denial.actor, AuthorityMutationActor::SandboxAdapter);
                assert_eq!(denial.row_class, AuthorityRowClass::PromotionReceipt);
                assert!(denial.denial_id.starts_with("MTE-ABD-"));
                assert!(denial.rationale.contains("gate-only"));
            }
            other => panic!("expected Deny, got {other:?}"),
        }
    }

    #[test]
    fn validation_attempting_promotion_decision_write_is_denied() {
        let v = AuthorityBoundaryCheck::evaluate(
            AuthorityMutationActor::ValidationRunner,
            AuthorityRowClass::PromotionDecision,
            "insert_promotion_decision",
        );
        assert!(!v.is_allowed());
    }

    #[test]
    fn mte_scheduler_cannot_write_promotion_rows_directly() {
        let v = AuthorityBoundaryCheck::evaluate(
            AuthorityMutationActor::MteScheduler,
            AuthorityRowClass::PromotionReceipt,
            "shortcut_insert",
        );
        assert!(!v.is_allowed());
    }

    #[test]
    fn promotion_gate_is_allowed_for_gate_only_rows() {
        let v = AuthorityBoundaryCheck::evaluate(
            AuthorityMutationActor::PromotionGate,
            AuthorityRowClass::PromotionReceipt,
            "insert_promotion_receipt",
        );
        assert!(v.is_allowed());
    }

    #[test]
    fn sandbox_can_write_its_own_run_record() {
        let v = AuthorityBoundaryCheck::evaluate(
            AuthorityMutationActor::SandboxAdapter,
            AuthorityRowClass::SandboxRunRecord,
            "insert_sandbox_run",
        );
        assert!(v.is_allowed());
    }

    #[test]
    fn validation_can_write_its_own_run_record() {
        let v = AuthorityBoundaryCheck::evaluate(
            AuthorityMutationActor::ValidationRunner,
            AuthorityRowClass::ValidationRunRecord,
            "insert_validation_run",
        );
        assert!(v.is_allowed());
    }

    #[test]
    fn unknown_actor_cannot_write_gate_rows() {
        let v = AuthorityBoundaryCheck::evaluate(
            AuthorityMutationActor::UnknownActor,
            AuthorityRowClass::PromotionDecision,
            "rogue_insert",
        );
        match v {
            AuthorityBoundaryVerdict::Deny { denial } => {
                assert_eq!(denial.actor, AuthorityMutationActor::UnknownActor);
            }
            other => panic!("expected Deny, got {other:?}"),
        }
    }

    #[test]
    fn denial_serde_round_trips() {
        let d = AuthorityBoundaryDenialV1::new(
            AuthorityMutationActor::SandboxAdapter,
            AuthorityRowClass::PromotionReceipt,
            "insert_promotion_receipt",
            "test",
        );
        let j = serde_json::to_string(&d).unwrap();
        let back: AuthorityBoundaryDenialV1 = serde_json::from_str(&j).unwrap();
        assert_eq!(back, d);
    }
}
