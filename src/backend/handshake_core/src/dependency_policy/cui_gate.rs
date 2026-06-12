//! WP-KERNEL-009 / MT-022 — CuiPortableBoundary.
//!
//! CUI portable artifacts are an allowed *operator-provided artifact class
//! only where a separate owning WP explicitly gates them*. WP-009 core
//! editor/index/memory behavior must never depend on them. This module is the
//! single typed gate every CUI-portable consumer must pass through:
//!
//! - [`CuiPortableGate::default`] / [`CuiPortableGate::closed`] — the gate is
//!   CLOSED unless an owning WP opens it with an explicit operator grant.
//! - [`CuiPortableGate::open_for_owning_wp`] — the only way to open the gate;
//!   requires a non-empty owning WP id and operator grant reference, which are
//!   retained for receipts/audit.
//! - There is intentionally NO global mutable gate, NO environment-variable
//!   override, and NO config-file fallback: a gate instance must be
//!   constructed and passed explicitly, so usage is visible in code review
//!   and validator scans.
//!
//! Mirrors the `cui_portable_artifact` entry in the runtime dependency
//! allowlist (operator_gated: true, default_enabled: false), enforced by the
//! parity test below.

use thiserror::Error;

use super::RuntimeInputKind;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum CuiGateError {
    #[error(
        "CUI portable gate is closed: {kind} inputs require an owning WP to open the gate \
         with an explicit operator grant (WP-009 core behavior must not depend on CUI portable artifacts)"
    )]
    GateClosed { kind: String },
    #[error("CUI portable gate requires a non-empty owning WP id")]
    MissingOwningWp,
    #[error("CUI portable gate requires a non-empty operator grant reference")]
    MissingOperatorGrant,
}

/// Operator gate for CUI-portable runtime inputs. Default: CLOSED.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CuiPortableGate {
    state: GateState,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum GateState {
    Closed,
    Open {
        owning_wp: String,
        operator_grant: String,
    },
}

impl Default for CuiPortableGate {
    /// The default gate is closed — `cui_portable_artifact` is
    /// `default_enabled: false` in the runtime dependency allowlist.
    fn default() -> Self {
        Self::closed()
    }
}

impl CuiPortableGate {
    /// A closed gate (the only state WP-009 itself may use).
    pub fn closed() -> Self {
        CuiPortableGate {
            state: GateState::Closed,
        }
    }

    /// Opens the gate for a specific owning WP under an explicit operator
    /// grant. Both identifiers are mandatory and retained for audit.
    pub fn open_for_owning_wp(owning_wp: &str, operator_grant: &str) -> Result<Self, CuiGateError> {
        if owning_wp.trim().is_empty() {
            return Err(CuiGateError::MissingOwningWp);
        }
        if operator_grant.trim().is_empty() {
            return Err(CuiGateError::MissingOperatorGrant);
        }
        Ok(CuiPortableGate {
            state: GateState::Open {
                owning_wp: owning_wp.trim().to_string(),
                operator_grant: operator_grant.trim().to_string(),
            },
        })
    }

    pub fn is_open(&self) -> bool {
        matches!(self.state, GateState::Open { .. })
    }

    /// The owning WP that opened the gate, when open.
    pub fn owning_wp(&self) -> Option<&str> {
        match &self.state {
            GateState::Open { owning_wp, .. } => Some(owning_wp),
            GateState::Closed => None,
        }
    }

    /// The operator grant reference that opened the gate, when open.
    pub fn operator_grant(&self) -> Option<&str> {
        match &self.state {
            GateState::Open { operator_grant, .. } => Some(operator_grant),
            GateState::Closed => None,
        }
    }

    /// Checks whether `kind` may be used under this gate. Non-CUI kinds are
    /// not this gate's concern and pass through; CUI-portable inputs require
    /// the gate to be open.
    pub fn permit(&self, kind: RuntimeInputKind) -> Result<(), CuiGateError> {
        match kind {
            RuntimeInputKind::CuiPortableArtifact => {
                if self.is_open() {
                    Ok(())
                } else {
                    Err(CuiGateError::GateClosed {
                        kind: kind.as_str().to_string(),
                    })
                }
            }
            _ => Ok(()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::{repo_root_from_manifest_dir, RuntimeDependencyAllowlist};
    use super::*;

    #[test]
    fn gate_defaults_closed() {
        let gate = CuiPortableGate::default();
        assert!(!gate.is_open());
        assert_eq!(gate.owning_wp(), None);
        assert_eq!(gate.operator_grant(), None);
    }

    #[test]
    fn closed_gate_rejects_cui_portable_inputs() {
        let gate = CuiPortableGate::closed();
        let err = gate
            .permit(RuntimeInputKind::CuiPortableArtifact)
            .expect_err("closed gate must reject CUI portable inputs");
        assert!(matches!(err, CuiGateError::GateClosed { .. }));
        let message = err.to_string();
        assert!(message.contains("closed"));
        assert!(message.contains("owning WP"));
    }

    #[test]
    fn closed_gate_does_not_block_non_cui_kinds() {
        let gate = CuiPortableGate::closed();
        assert_eq!(gate.permit(RuntimeInputKind::ModelGguf), Ok(()));
        assert_eq!(gate.permit(RuntimeInputKind::ModelSafetensors), Ok(()));
        assert_eq!(gate.permit(RuntimeInputKind::TensorArtifact), Ok(()));
    }

    #[test]
    fn gate_opens_only_with_owning_wp_and_operator_grant() {
        assert_eq!(
            CuiPortableGate::open_for_owning_wp("", "grant-1"),
            Err(CuiGateError::MissingOwningWp)
        );
        assert_eq!(
            CuiPortableGate::open_for_owning_wp("WP-FUTURE-CUI", "  "),
            Err(CuiGateError::MissingOperatorGrant)
        );
        let gate =
            CuiPortableGate::open_for_owning_wp("WP-FUTURE-CUI", "operator-grant-receipt-42")
                .expect("valid open");
        assert!(gate.is_open());
        assert_eq!(gate.owning_wp(), Some("WP-FUTURE-CUI"));
        assert_eq!(gate.operator_grant(), Some("operator-grant-receipt-42"));
        assert_eq!(gate.permit(RuntimeInputKind::CuiPortableArtifact), Ok(()));
    }

    #[test]
    fn gate_default_matches_allowlist_document() {
        let allowlist =
            RuntimeDependencyAllowlist::load_from_repo_root(&repo_root_from_manifest_dir())
                .expect("allowlist loads");
        let cui = allowlist
            .allowed_external_runtime_inputs
            .iter()
            .find(|i| i.kind == "cui_portable_artifact")
            .expect("cui_portable_artifact declared");
        assert!(cui.operator_gated, "allowlist must keep CUI operator-gated");
        assert!(
            !cui.default_enabled,
            "allowlist must keep CUI default-off, matching CuiPortableGate::default"
        );
        assert!(!CuiPortableGate::default().is_open());
    }
}
