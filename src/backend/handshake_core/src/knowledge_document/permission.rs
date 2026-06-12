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
}

impl DocumentActorKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Operator => "operator",
            Self::LocalModel => "local_model",
            Self::CloudModel => "cloud_model",
            Self::Validator => "validator",
            Self::System => "system",
        }
    }

    /// Parse from the wire token used by the API identity header.
    pub fn from_wire(value: &str) -> Option<Self> {
        Some(match value {
            "operator" => Self::Operator,
            "local_model" => Self::LocalModel,
            "cloud_model" => Self::CloudModel,
            "validator" => Self::Validator,
            "system" => Self::System,
            _ => return None,
        })
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
/// | actor        | read | write | index |
/// |--------------|------|-------|-------|
/// | operator     |  yes |  yes  |  yes  |
/// | local_model  |  yes |  yes  |  yes  |
/// | cloud_model  |  yes |  no   |  yes  |
/// | validator    |  yes |  no   |  no   |
/// | system       |  yes |  yes  |  yes  |
///
/// Rationale: a cloud model may read and index but must route writes through a
/// promotion path rather than mutate authority directly (spec 2.3.13.11 draft
/// -> promotion); a validator observes and indexes-read but does not author or
/// re-index.
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
        ] {
            assert_eq!(DocumentActorKind::from_wire(kind.as_str()), Some(kind));
        }
        assert_eq!(DocumentActorKind::from_wire("nope"), None);
    }
}
