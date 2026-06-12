//! MT-158 DocumentPermissionBoundary.
//!
//! The server-enforced permission boundary for rich documents: which actor
//! kinds may read, write, or index a document. This is the AUTHORITY decision
//! function the API layer calls before performing a document action; it is not a
//! UI hint. The default policy is deliberately conservative and explicit so a
//! no-context reader can see exactly who may do what.

use serde::{Deserialize, Serialize};

/// The actor kinds that can act on a document (mirrors the document owner
/// vocabulary in migration 0280 and the KernelActor taxonomy).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DocumentActorKind {
    Operator,
    LocalModel,
    CloudModel,
    Validator,
    System,
    /// A caller that did NOT explicitly assert an actor kind (adversarial-v2
    /// MT-158 hardening). Privilege is never inferred: an absent or
    /// unauthenticated kind is the LEAST-privileged actor — read-only, no
    /// write, no index. This kind can never own a document (it cannot create
    /// one), so it never enters the migration-0280 owner vocabulary.
    Unauthenticated,
}

impl DocumentActorKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Operator => "operator",
            Self::LocalModel => "local_model",
            Self::CloudModel => "cloud_model",
            Self::Validator => "validator",
            Self::System => "system",
            Self::Unauthenticated => "unauthenticated",
        }
    }

    /// Parse from the wire token used by the API identity header. STRICT: an
    /// unknown token is `None` (the API rejects it with a 400) — it is never
    /// coerced to a privileged kind. `unauthenticated` is accepted explicitly
    /// (it is never an escalation: it only ever lowers privilege).
    pub fn from_wire(value: &str) -> Option<Self> {
        Some(match value {
            "operator" => Self::Operator,
            "local_model" => Self::LocalModel,
            "cloud_model" => Self::CloudModel,
            "validator" => Self::Validator,
            "system" => Self::System,
            "unauthenticated" => Self::Unauthenticated,
            _ => return None,
        })
    }

    /// The kind applied when a caller asserts NO actor kind (MT-158
    /// fail-closed default). Least-privileged: read-only.
    pub fn least_privileged() -> Self {
        Self::Unauthenticated
    }
}

/// A document action subject to the permission boundary (MT-158).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DocumentAction {
    /// Load / project / export a document.
    Read,
    /// Save / promote / batch-mutate a document.
    Write,
    /// Index a document into the Project Knowledge Index / backlinks.
    Index,
}

impl DocumentAction {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Read => "read",
            Self::Write => "write",
            Self::Index => "index",
        }
    }
}

/// The outcome of a permission check (MT-158).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PermissionDecision {
    pub allowed: bool,
    /// Stable machine reason code (e.g. `allowed`, `cloud_model_write_denied`).
    pub reason: String,
}

impl PermissionDecision {
    fn allow() -> Self {
        Self {
            allowed: true,
            reason: "allowed".to_string(),
        }
    }

    fn deny(reason: impl Into<String>) -> Self {
        Self {
            allowed: false,
            reason: reason.into(),
        }
    }
}

/// The server-enforced document permission policy (MT-158).
///
/// Default matrix (the conservative baseline; a document may later carry an
/// explicit override row, but the boundary itself is server-side):
///
/// | actor           | read | write | index |
/// |-----------------|------|-------|-------|
/// | operator        |  yes |  yes  |  yes  |
/// | local_model     |  yes |  yes  |  yes  |
/// | cloud_model     |  yes |  no   |  yes  |
/// | validator       |  yes |  no   |  no   |
/// | system          |  yes |  yes  |  yes  |
/// | unauthenticated |  yes |  no   |  no   |
///
/// Rationale: a cloud model may read and index but must route writes through a
/// promotion path rather than mutate authority directly (spec 2.3.13.11 draft
/// -> promotion); a validator observes and indexes-read but does not author or
/// re-index. An unauthenticated caller (no asserted kind) is read-only:
/// privilege must be explicitly asserted on the wire and is validated
/// server-side — it is never inferred from absence (adversarial-v2 MT-158).
#[derive(Debug, Clone, Copy, Default)]
pub struct DocumentPermission;

impl DocumentPermission {
    /// Decide whether `actor` may perform `action` on a document (MT-158). Pure
    /// and server-side; the API layer calls this before acting and refuses on a
    /// denial.
    pub fn decide(actor: DocumentActorKind, action: DocumentAction) -> PermissionDecision {
        use DocumentAction::*;
        use DocumentActorKind::*;
        let allowed = match (actor, action) {
            (Operator | System | LocalModel, _) => true,
            (CloudModel, Read | Index) => true,
            (CloudModel, Write) => false,
            (Validator, Read) => true,
            (Validator, Write | Index) => false,
            // MT-158 hardening: an unasserted/unauthenticated kind is
            // read-only — it can never write or index.
            (Unauthenticated, Read) => true,
            (Unauthenticated, Write | Index) => false,
        };
        if allowed {
            PermissionDecision::allow()
        } else {
            PermissionDecision::deny(format!("{}_{}_denied", actor.as_str(), action.as_str()))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_matrix_matches_the_documented_policy() {
        use DocumentAction::*;
        use DocumentActorKind::*;
        // Operator / system / local_model: full access.
        for actor in [Operator, System, LocalModel] {
            for action in [Read, Write, Index] {
                assert!(DocumentPermission::decide(actor, action).allowed);
            }
        }
        // Cloud model: read + index, but not write.
        assert!(DocumentPermission::decide(CloudModel, Read).allowed);
        assert!(DocumentPermission::decide(CloudModel, Index).allowed);
        assert!(!DocumentPermission::decide(CloudModel, Write).allowed);
        // Validator: read only.
        assert!(DocumentPermission::decide(Validator, Read).allowed);
        assert!(!DocumentPermission::decide(Validator, Write).allowed);
        assert!(!DocumentPermission::decide(Validator, Index).allowed);
        // Unauthenticated (no asserted kind): read only — never write/index.
        assert!(DocumentPermission::decide(Unauthenticated, Read).allowed);
        assert!(!DocumentPermission::decide(Unauthenticated, Write).allowed);
        assert!(!DocumentPermission::decide(Unauthenticated, Index).allowed);
    }

    #[test]
    fn least_privileged_default_cannot_write_or_index() {
        // MT-158 fail-closed: the absent-header default is read-only.
        let kind = DocumentActorKind::least_privileged();
        assert_eq!(kind, DocumentActorKind::Unauthenticated);
        assert!(DocumentPermission::decide(kind, DocumentAction::Read).allowed);
        let write = DocumentPermission::decide(kind, DocumentAction::Write);
        assert!(!write.allowed);
        assert_eq!(write.reason, "unauthenticated_write_denied");
        assert!(!DocumentPermission::decide(kind, DocumentAction::Index).allowed);
    }

    #[test]
    fn denials_carry_a_stable_reason_code() {
        let d = DocumentPermission::decide(DocumentActorKind::CloudModel, DocumentAction::Write);
        assert!(!d.allowed);
        assert_eq!(d.reason, "cloud_model_write_denied");
    }

    #[test]
    fn actor_kind_wire_roundtrip() {
        for kind in [
            DocumentActorKind::Operator,
            DocumentActorKind::LocalModel,
            DocumentActorKind::CloudModel,
            DocumentActorKind::Validator,
            DocumentActorKind::System,
            DocumentActorKind::Unauthenticated,
        ] {
            assert_eq!(DocumentActorKind::from_wire(kind.as_str()), Some(kind));
        }
        // Unknown tokens are rejected, never coerced to a privileged kind.
        assert_eq!(DocumentActorKind::from_wire("nope"), None);
        assert_eq!(DocumentActorKind::from_wire("SYSTEM"), None);
        assert_eq!(DocumentActorKind::from_wire("root"), None);
    }
}
