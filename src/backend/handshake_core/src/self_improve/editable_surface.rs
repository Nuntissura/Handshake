//! MT-149: EditableSurface registry — V0 allow-list and forbid-list per
//! AC-DISTILL-EDITABLE-SURFACE.
//!
//! Allow:
//! - ModelManual capsule text (one section per snapshot)
//! - RetrievalPolicy parameters (top_k, capsule_budget_bytes)
//!
//! Forbid (returns typed [`EditableSurfaceError::Forbidden`]):
//! - Spec text (ShadowAuthority risk)
//! - Role-shared system prompts (BlastRadiusTooWide)
//! - LoRA weights (NoTrainingInfraInV0)
//! - Tool descriptions (ToolDescriptionAuthority)

use serde::{Deserialize, Serialize};
use thiserror::Error;

use super::iteration::{LoopTarget, PolicyParameterRef};

/// Maximum capsule budget bytes a proposal may set. Caps the RetrievalPolicy
/// surface so a bad proposal cannot blow the in-prompt budget into a
/// degenerate state. The cap is durable: changing it requires a typed code
/// change.
pub const MAX_CAPSULE_BUDGET_BYTES: u64 = 1_048_576; // 1 MiB

/// Maximum reasonable top_k value. Same rationale as
/// [`MAX_CAPSULE_BUDGET_BYTES`].
pub const MAX_TOP_K: u32 = 64;

/// Snapshot of the editable surface before+after a candidate proposal.
/// Variants mirror the [`LoopTarget`] variants.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "surface_kind")]
pub enum EditableSurfaceSnapshot {
    ModelManual {
        manual_section_id: String,
        before_text: String,
        after_text: String,
    },
    RetrievalPolicy {
        task_type: crate::memory::TaskType,
        parameter: PolicyParameterRef,
        before_value: u64,
        after_value: u64,
    },
}

impl EditableSurfaceSnapshot {
    /// Returns true if the proposal would not change anything (before == after).
    pub fn is_noop(&self) -> bool {
        match self {
            Self::ModelManual {
                before_text,
                after_text,
                ..
            } => before_text == after_text,
            Self::RetrievalPolicy {
                before_value,
                after_value,
                ..
            } => before_value == after_value,
        }
    }
}

/// Concrete proposal types accepted by the surface provider.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "proposal_kind")]
pub enum SurfaceProposal {
    ModelManualText { new_text: String },
    RetrievalPolicyValue { new_value: u64 },
}

/// Closed PolicyParameter view used by the surface provider to clamp
/// proposals. Mirror of [`PolicyParameterRef`] in the iteration module so
/// the surface can be reasoned about independently.
pub use super::iteration::PolicyParameterRef as PolicyParameter;

/// Why a particular target is on the forbid-list.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ForbidReason {
    ShadowAuthority,
    BlastRadiusTooWide,
    NoTrainingInfraInV0,
    ToolDescriptionAuthority,
}

impl ForbidReason {
    pub fn slug(self) -> &'static str {
        match self {
            Self::ShadowAuthority => "shadow_authority",
            Self::BlastRadiusTooWide => "blast_radius_too_wide",
            Self::NoTrainingInfraInV0 => "no_training_infra_in_v0",
            Self::ToolDescriptionAuthority => "tool_description_authority",
        }
    }
}

/// Guard the surface provider invokes BEFORE any snapshot or apply. The
/// `LoopTarget` enum is closed at compile time so synthetic forbidden
/// targets must be constructed by manual `Other` strings; this guard
/// catches those and any future surface-id heuristics.
#[derive(Debug, Default)]
pub struct ForbiddenSurfaceGuard;

impl ForbiddenSurfaceGuard {
    pub fn check(target: &LoopTarget) -> Result<(), EditableSurfaceError> {
        match target {
            LoopTarget::ModelManualCapsuleText { manual_section_id } => {
                // Section ids that look like spec anchors are forbidden.
                let lowered = manual_section_id.to_ascii_lowercase();
                if lowered.starts_with("spec.") || lowered.contains("/spec/") {
                    return Err(EditableSurfaceError::Forbidden {
                        surface_id: manual_section_id.clone(),
                        reason: ForbidReason::ShadowAuthority,
                    });
                }
                if lowered.starts_with("role.") || lowered.contains("/roles_shared/") {
                    return Err(EditableSurfaceError::Forbidden {
                        surface_id: manual_section_id.clone(),
                        reason: ForbidReason::BlastRadiusTooWide,
                    });
                }
                if lowered.contains("lora") || lowered.contains("weights") {
                    return Err(EditableSurfaceError::Forbidden {
                        surface_id: manual_section_id.clone(),
                        reason: ForbidReason::NoTrainingInfraInV0,
                    });
                }
                if lowered.contains("tool_description") || lowered.contains("tool.description") {
                    return Err(EditableSurfaceError::Forbidden {
                        surface_id: manual_section_id.clone(),
                        reason: ForbidReason::ToolDescriptionAuthority,
                    });
                }
                Ok(())
            }
            LoopTarget::RetrievalPolicyParams { .. } => Ok(()),
        }
    }
}

/// Trait implemented by every concrete editable surface (ModelManual,
/// RetrievalPolicy). The guard runs unconditionally before snapshot.
pub trait EditableSurfaceProvider {
    fn snapshot(
        &self,
        target: &LoopTarget,
    ) -> Result<EditableSurfaceSnapshot, EditableSurfaceError>;

    fn apply_proposal(
        &self,
        snapshot: &EditableSurfaceSnapshot,
        proposal: SurfaceProposal,
    ) -> Result<EditableSurfaceSnapshot, EditableSurfaceError>;
}

/// Concrete editable surface for ModelManual section text. Production
/// wires `read_section` + `write_section` to the real ModelManual surface;
/// tests inject in-memory closures.
pub struct ModelManualSurface<R, W>
where
    R: Fn(&str) -> Result<String, EditableSurfaceError>,
    W: Fn(&str, &str) -> Result<(), EditableSurfaceError>,
{
    read_section: R,
    write_section: W,
}

impl<R, W> ModelManualSurface<R, W>
where
    R: Fn(&str) -> Result<String, EditableSurfaceError>,
    W: Fn(&str, &str) -> Result<(), EditableSurfaceError>,
{
    pub fn new(read_section: R, write_section: W) -> Self {
        Self {
            read_section,
            write_section,
        }
    }
}

impl<R, W> EditableSurfaceProvider for ModelManualSurface<R, W>
where
    R: Fn(&str) -> Result<String, EditableSurfaceError>,
    W: Fn(&str, &str) -> Result<(), EditableSurfaceError>,
{
    fn snapshot(
        &self,
        target: &LoopTarget,
    ) -> Result<EditableSurfaceSnapshot, EditableSurfaceError> {
        ForbiddenSurfaceGuard::check(target)?;
        match target {
            LoopTarget::ModelManualCapsuleText { manual_section_id } => {
                let before = (self.read_section)(manual_section_id)?;
                Ok(EditableSurfaceSnapshot::ModelManual {
                    manual_section_id: manual_section_id.clone(),
                    before_text: before.clone(),
                    after_text: before,
                })
            }
            LoopTarget::RetrievalPolicyParams { .. } => {
                Err(EditableSurfaceError::MismatchedTarget {
                    expected: "model_manual_capsule_text",
                    got: "retrieval_policy_params",
                })
            }
        }
    }

    fn apply_proposal(
        &self,
        snapshot: &EditableSurfaceSnapshot,
        proposal: SurfaceProposal,
    ) -> Result<EditableSurfaceSnapshot, EditableSurfaceError> {
        match (snapshot, proposal) {
            (
                EditableSurfaceSnapshot::ModelManual {
                    manual_section_id,
                    before_text,
                    ..
                },
                SurfaceProposal::ModelManualText { new_text },
            ) => {
                if new_text.trim().is_empty() {
                    return Err(EditableSurfaceError::InvalidProposal {
                        field: "new_text",
                        message: "model manual text must not be empty".to_string(),
                    });
                }
                if new_text.len() > 1_048_576 {
                    return Err(EditableSurfaceError::InvalidProposal {
                        field: "new_text",
                        message: "model manual text exceeds 1MiB cap".to_string(),
                    });
                }
                (self.write_section)(manual_section_id, &new_text)?;
                Ok(EditableSurfaceSnapshot::ModelManual {
                    manual_section_id: manual_section_id.clone(),
                    before_text: before_text.clone(),
                    after_text: new_text,
                })
            }
            _ => Err(EditableSurfaceError::MismatchedTarget {
                expected: "model_manual_capsule_text",
                got: "retrieval_policy_params",
            }),
        }
    }
}

/// Concrete editable surface for RetrievalPolicy parameters. Production
/// wires the read/write to the [`CapsulePolicyTable`]; tests inject
/// closures.
pub struct RetrievalPolicySurface<R, W>
where
    R: Fn(crate::memory::TaskType, PolicyParameter) -> Result<u64, EditableSurfaceError>,
    W: Fn(crate::memory::TaskType, PolicyParameter, u64) -> Result<(), EditableSurfaceError>,
{
    read_param: R,
    write_param: W,
}

impl<R, W> RetrievalPolicySurface<R, W>
where
    R: Fn(crate::memory::TaskType, PolicyParameter) -> Result<u64, EditableSurfaceError>,
    W: Fn(crate::memory::TaskType, PolicyParameter, u64) -> Result<(), EditableSurfaceError>,
{
    pub fn new(read_param: R, write_param: W) -> Self {
        Self {
            read_param,
            write_param,
        }
    }
}

impl<R, W> EditableSurfaceProvider for RetrievalPolicySurface<R, W>
where
    R: Fn(crate::memory::TaskType, PolicyParameter) -> Result<u64, EditableSurfaceError>,
    W: Fn(crate::memory::TaskType, PolicyParameter, u64) -> Result<(), EditableSurfaceError>,
{
    fn snapshot(
        &self,
        target: &LoopTarget,
    ) -> Result<EditableSurfaceSnapshot, EditableSurfaceError> {
        ForbiddenSurfaceGuard::check(target)?;
        match target {
            LoopTarget::RetrievalPolicyParams {
                task_type,
                parameter,
            } => {
                let before = (self.read_param)(*task_type, *parameter)?;
                clamp_policy_value(*parameter, before)?;
                Ok(EditableSurfaceSnapshot::RetrievalPolicy {
                    task_type: *task_type,
                    parameter: *parameter,
                    before_value: before,
                    after_value: before,
                })
            }
            LoopTarget::ModelManualCapsuleText { .. } => {
                Err(EditableSurfaceError::MismatchedTarget {
                    expected: "retrieval_policy_params",
                    got: "model_manual_capsule_text",
                })
            }
        }
    }

    fn apply_proposal(
        &self,
        snapshot: &EditableSurfaceSnapshot,
        proposal: SurfaceProposal,
    ) -> Result<EditableSurfaceSnapshot, EditableSurfaceError> {
        match (snapshot, proposal) {
            (
                EditableSurfaceSnapshot::RetrievalPolicy {
                    task_type,
                    parameter,
                    before_value,
                    ..
                },
                SurfaceProposal::RetrievalPolicyValue { new_value },
            ) => {
                clamp_policy_value(*parameter, new_value)?;
                (self.write_param)(*task_type, *parameter, new_value)?;
                Ok(EditableSurfaceSnapshot::RetrievalPolicy {
                    task_type: *task_type,
                    parameter: *parameter,
                    before_value: *before_value,
                    after_value: new_value,
                })
            }
            _ => Err(EditableSurfaceError::MismatchedTarget {
                expected: "retrieval_policy_params",
                got: "model_manual_capsule_text",
            }),
        }
    }
}

fn clamp_policy_value(
    parameter: PolicyParameter,
    new_value: u64,
) -> Result<(), EditableSurfaceError> {
    match parameter {
        PolicyParameter::TopK => {
            if new_value == 0 {
                return Err(EditableSurfaceError::InvalidProposal {
                    field: "top_k",
                    message: "top_k must be greater than zero".to_string(),
                });
            }
            if new_value > u64::from(MAX_TOP_K) {
                return Err(EditableSurfaceError::InvalidProposal {
                    field: "top_k",
                    message: format!("top_k must not exceed {}", MAX_TOP_K),
                });
            }
        }
        PolicyParameter::CapsuleBudgetBytes => {
            if new_value == 0 {
                return Err(EditableSurfaceError::InvalidProposal {
                    field: "capsule_budget_bytes",
                    message: "capsule_budget_bytes must be greater than zero".to_string(),
                });
            }
            if new_value > MAX_CAPSULE_BUDGET_BYTES {
                return Err(EditableSurfaceError::InvalidProposal {
                    field: "capsule_budget_bytes",
                    message: format!(
                        "capsule_budget_bytes must not exceed {} bytes",
                        MAX_CAPSULE_BUDGET_BYTES
                    ),
                });
            }
        }
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq, Error, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum EditableSurfaceError {
    #[error("editable surface forbidden: {surface_id} ({:?})", reason)]
    Forbidden {
        surface_id: String,
        reason: ForbidReason,
    },
    #[error("editable surface target mismatch: expected {expected}, got {got}")]
    MismatchedTarget {
        expected: &'static str,
        got: &'static str,
    },
    #[error("editable surface proposal invalid {field}: {message}")]
    InvalidProposal {
        field: &'static str,
        message: String,
    },
    #[error("editable surface IO error: {message}")]
    Io { message: String },
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::TaskType;
    use std::cell::RefCell;

    fn pol_target(parameter: PolicyParameter) -> LoopTarget {
        LoopTarget::RetrievalPolicyParams {
            task_type: TaskType::ValidatorHbrTestPacket,
            parameter,
        }
    }

    #[test]
    fn forbidden_guard_blocks_spec_section() {
        let target = LoopTarget::ModelManualCapsuleText {
            manual_section_id: "spec.handshake_core".to_string(),
        };
        let err = ForbiddenSurfaceGuard::check(&target).unwrap_err();
        match err {
            EditableSurfaceError::Forbidden { reason, .. } => {
                assert_eq!(reason, ForbidReason::ShadowAuthority);
            }
            _ => panic!("expected Forbidden error"),
        }
    }

    #[test]
    fn forbidden_guard_blocks_role_section() {
        let target = LoopTarget::ModelManualCapsuleText {
            manual_section_id: "role.orchestrator".to_string(),
        };
        let err = ForbiddenSurfaceGuard::check(&target).unwrap_err();
        assert!(matches!(
            err,
            EditableSurfaceError::Forbidden {
                reason: ForbidReason::BlastRadiusTooWide,
                ..
            }
        ));
    }

    #[test]
    fn forbidden_guard_blocks_lora() {
        let target = LoopTarget::ModelManualCapsuleText {
            manual_section_id: "model.lora_weights".to_string(),
        };
        let err = ForbiddenSurfaceGuard::check(&target).unwrap_err();
        assert!(matches!(
            err,
            EditableSurfaceError::Forbidden {
                reason: ForbidReason::NoTrainingInfraInV0,
                ..
            }
        ));
    }

    #[test]
    fn forbidden_guard_blocks_tool_description() {
        let target = LoopTarget::ModelManualCapsuleText {
            manual_section_id: "tool_description.shell".to_string(),
        };
        let err = ForbiddenSurfaceGuard::check(&target).unwrap_err();
        assert!(matches!(
            err,
            EditableSurfaceError::Forbidden {
                reason: ForbidReason::ToolDescriptionAuthority,
                ..
            }
        ));
    }

    #[test]
    fn forbidden_guard_passes_legitimate_targets() {
        let target = LoopTarget::ModelManualCapsuleText {
            manual_section_id: "intro.usage_overview".to_string(),
        };
        ForbiddenSurfaceGuard::check(&target).unwrap();
        ForbiddenSurfaceGuard::check(&pol_target(PolicyParameter::TopK)).unwrap();
    }

    #[test]
    fn retrieval_policy_surface_clamps_top_k_zero() {
        let surface = RetrievalPolicySurface::new(|_, _| Ok(6), |_, _, _| Ok(()));
        let snap = surface
            .snapshot(&pol_target(PolicyParameter::TopK))
            .unwrap();
        let err = surface
            .apply_proposal(
                &snap,
                SurfaceProposal::RetrievalPolicyValue { new_value: 0 },
            )
            .unwrap_err();
        assert!(matches!(
            err,
            EditableSurfaceError::InvalidProposal { field: "top_k", .. }
        ));
    }

    #[test]
    fn retrieval_policy_surface_clamps_budget_exceeds_max() {
        let surface = RetrievalPolicySurface::new(|_, _| Ok(32_768), |_, _, _| Ok(()));
        let snap = surface
            .snapshot(&pol_target(PolicyParameter::CapsuleBudgetBytes))
            .unwrap();
        let err = surface
            .apply_proposal(
                &snap,
                SurfaceProposal::RetrievalPolicyValue {
                    new_value: MAX_CAPSULE_BUDGET_BYTES + 1,
                },
            )
            .unwrap_err();
        assert!(matches!(
            err,
            EditableSurfaceError::InvalidProposal {
                field: "capsule_budget_bytes",
                ..
            }
        ));
    }

    #[test]
    fn retrieval_policy_surface_round_trips() {
        let writes = RefCell::new(Vec::new());
        let surface = RetrievalPolicySurface::new(
            |_, _| Ok(6),
            |task_type, param, value| {
                writes.borrow_mut().push((task_type, param, value));
                Ok(())
            },
        );
        let snap = surface
            .snapshot(&pol_target(PolicyParameter::TopK))
            .unwrap();
        let applied = surface
            .apply_proposal(
                &snap,
                SurfaceProposal::RetrievalPolicyValue { new_value: 8 },
            )
            .unwrap();
        match applied {
            EditableSurfaceSnapshot::RetrievalPolicy {
                before_value,
                after_value,
                ..
            } => {
                assert_eq!(before_value, 6);
                assert_eq!(after_value, 8);
            }
            _ => panic!("expected retrieval policy snapshot"),
        }
        assert_eq!(writes.borrow().len(), 1);
    }

    #[test]
    fn model_manual_surface_round_trips() {
        let writes = RefCell::new(Vec::new());
        let surface = ModelManualSurface::new(
            |_| Ok("old text".to_string()),
            |section, new| {
                writes
                    .borrow_mut()
                    .push((section.to_string(), new.to_string()));
                Ok(())
            },
        );
        let target = LoopTarget::ModelManualCapsuleText {
            manual_section_id: "intro.usage_overview".to_string(),
        };
        let snap = surface.snapshot(&target).unwrap();
        let applied = surface
            .apply_proposal(
                &snap,
                SurfaceProposal::ModelManualText {
                    new_text: "new text".to_string(),
                },
            )
            .unwrap();
        match applied {
            EditableSurfaceSnapshot::ModelManual {
                before_text,
                after_text,
                ..
            } => {
                assert_eq!(before_text, "old text");
                assert_eq!(after_text, "new text");
            }
            _ => panic!("expected model manual snapshot"),
        }
        assert_eq!(writes.borrow().len(), 1);
    }
}
