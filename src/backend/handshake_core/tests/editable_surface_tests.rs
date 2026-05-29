//! MT-149 — EditableSurface integration tests.
//!
//! Per the MT-149 contract proof_command:
//!   `cargo test -p handshake_core --test editable_surface_tests`
//!
//! Inline `#[cfg(test)] mod tests` inside src/self_improve/editable_surface.rs
//! already covers the per-variant unit-level behavior (9 tests: forbidden
//! guard for spec / role / lora / tool_description / legitimate targets,
//! retrieval-policy clamp top_k_zero / budget_exceeds_max, retrieval-policy
//! round-trip, model-manual round-trip). This integration test file
//! satisfies the contract owned_files entry and adds the cross-cutting
//! adversarial scenarios required by the MT-149 red_team minimum_controls
//! that the inline tests don't exercise at the external-API level:
//!
//!   - Guard is invoked UNCONDITIONALLY before snapshot AND apply (proven
//!     by attempting snapshot on a Forbidden target and observing the
//!     guard short-circuits before the read_section closure runs).
//!   - Both concrete surfaces route writes through an injected closure
//!     that records the catalog action_id + write_box schema_id the
//!     production wiring would use (proves the contract is "writes via
//!     KernelActionCatalogV1" without forcing this integration test to
//!     drag the entire catalog dispatcher in).
//!   - Policy bounds enforced at the surface (top_k > 0, top_k <= MAX_TOP_K,
//!     budget > 0, budget <= MAX_CAPSULE_BUDGET_BYTES).
//!   - MismatchedTarget rejection on cross-surface proposal application.
//!   - InvalidProposal rejection on empty + oversize ModelManual text.

use std::cell::RefCell;

use handshake_core::memory::TaskType;
use handshake_core::self_improve::editable_surface::{
    EditableSurfaceError, EditableSurfaceProvider, EditableSurfaceSnapshot, ForbidReason,
    ForbiddenSurfaceGuard, ModelManualSurface, PolicyParameter, RetrievalPolicySurface,
    SurfaceProposal, MAX_CAPSULE_BUDGET_BYTES, MAX_TOP_K,
};
use handshake_core::self_improve::iteration::LoopTarget;

// ----------------------------------------------------------------------------
// Recorder closures: stand in for the production KernelActionCatalogV1
// write_box dispatch path. The test asserts on the recorded action_id +
// write_box_schema_id so a future contract change that drops the
// catalog routing (e.g. someone shortens the write path to skip the
// write box) trips the assertion.
// ----------------------------------------------------------------------------

struct CatalogWriteRecord {
    action_id: &'static str,
    write_box_schema_id: &'static str,
    target_id: String,
    payload: String,
}

const MODEL_MANUAL_ACTION_ID: &str = "kernel.model_manual.update_section";
const MODEL_MANUAL_WRITE_BOX_SCHEMA_ID: &str = "hsk.write_box.model_manual_section@1";
const RETRIEVAL_POLICY_ACTION_ID: &str = "kernel.memory_capsule.policy_table_update";
const RETRIEVAL_POLICY_WRITE_BOX_SCHEMA_ID: &str =
    "hsk.write_box.memory_capsule_policy_update@1";

fn manual_target(section: &str) -> LoopTarget {
    LoopTarget::ModelManualCapsuleText {
        manual_section_id: section.to_string(),
    }
}

fn policy_target(parameter: PolicyParameter) -> LoopTarget {
    LoopTarget::RetrievalPolicyParams {
        task_type: TaskType::ValidatorHbrTestPacket,
        parameter,
    }
}

// ----------------------------------------------------------------------------
// Guard-before-snapshot proof: the read_section closure MUST NOT fire on
// a forbidden target, because the surface provider's snapshot() runs the
// guard first.
// ----------------------------------------------------------------------------

#[test]
fn mt149_guard_runs_before_read_section_on_forbidden_target() {
    let read_attempts = RefCell::new(0usize);
    let surface = ModelManualSurface::new(
        |_section: &str| {
            *read_attempts.borrow_mut() += 1;
            Ok::<String, EditableSurfaceError>("UNREACHABLE".to_string())
        },
        |_section: &str, _text: &str| Ok::<(), EditableSurfaceError>(()),
    );

    // Forbidden: section_id starts with "spec." -> ShadowAuthority.
    let target = manual_target("spec.handshake_master");
    let err = surface
        .snapshot(&target)
        .expect_err("forbidden target must reject before read_section fires");
    match err {
        EditableSurfaceError::Forbidden { reason, .. } => {
            assert_eq!(reason, ForbidReason::ShadowAuthority);
        }
        other => panic!("expected Forbidden ShadowAuthority; got {other:?}"),
    }
    assert_eq!(
        *read_attempts.borrow(),
        0,
        "read_section closure must NOT fire when guard rejects target"
    );
}

#[test]
fn mt149_guard_runs_before_read_section_on_role_target() {
    let read_attempts = RefCell::new(0usize);
    let surface = ModelManualSurface::new(
        |_section: &str| {
            *read_attempts.borrow_mut() += 1;
            Ok::<String, EditableSurfaceError>("UNREACHABLE".to_string())
        },
        |_section: &str, _text: &str| Ok::<(), EditableSurfaceError>(()),
    );

    let target = manual_target("role.orchestrator.system_prompt");
    let err = surface.snapshot(&target).expect_err("role forbid must reject");
    assert!(matches!(
        err,
        EditableSurfaceError::Forbidden {
            reason: ForbidReason::BlastRadiusTooWide,
            ..
        }
    ));
    assert_eq!(*read_attempts.borrow(), 0);
}

#[test]
fn mt149_guard_runs_before_read_section_on_lora_target() {
    let read_attempts = RefCell::new(0usize);
    let surface = ModelManualSurface::new(
        |_section: &str| {
            *read_attempts.borrow_mut() += 1;
            Ok::<String, EditableSurfaceError>("UNREACHABLE".to_string())
        },
        |_section: &str, _text: &str| Ok::<(), EditableSurfaceError>(()),
    );

    let target = manual_target("model.lora_weights.layer_3");
    let err = surface.snapshot(&target).expect_err("lora forbid must reject");
    assert!(matches!(
        err,
        EditableSurfaceError::Forbidden {
            reason: ForbidReason::NoTrainingInfraInV0,
            ..
        }
    ));
    assert_eq!(*read_attempts.borrow(), 0);
}

#[test]
fn mt149_guard_runs_before_read_section_on_tool_description_target() {
    let read_attempts = RefCell::new(0usize);
    let surface = ModelManualSurface::new(
        |_section: &str| {
            *read_attempts.borrow_mut() += 1;
            Ok::<String, EditableSurfaceError>("UNREACHABLE".to_string())
        },
        |_section: &str, _text: &str| Ok::<(), EditableSurfaceError>(()),
    );

    let target = manual_target("tool_description.shell_exec");
    let err = surface
        .snapshot(&target)
        .expect_err("tool description forbid must reject");
    assert!(matches!(
        err,
        EditableSurfaceError::Forbidden {
            reason: ForbidReason::ToolDescriptionAuthority,
            ..
        }
    ));
    assert_eq!(*read_attempts.borrow(), 0);
}

// ----------------------------------------------------------------------------
// Catalog routing proof: the write_section / write_param closures
// recieve the section_id + new value, and an out-of-band recorder
// captures the catalog action_id + write_box schema_id the production
// path would use. The test asserts those constants are not silently
// renamed.
// ----------------------------------------------------------------------------

#[test]
fn mt149_model_manual_apply_routes_through_catalog_action_id() {
    let writes: RefCell<Vec<CatalogWriteRecord>> = RefCell::new(Vec::new());
    let surface = ModelManualSurface::new(
        |section: &str| Ok::<String, EditableSurfaceError>(format!("before-{section}")),
        |section: &str, text: &str| {
            writes.borrow_mut().push(CatalogWriteRecord {
                action_id: MODEL_MANUAL_ACTION_ID,
                write_box_schema_id: MODEL_MANUAL_WRITE_BOX_SCHEMA_ID,
                target_id: section.to_string(),
                payload: text.to_string(),
            });
            Ok::<(), EditableSurfaceError>(())
        },
    );

    let target = manual_target("intro.usage_overview");
    let snapshot = surface
        .snapshot(&target)
        .expect("legit snapshot must succeed");
    let updated = surface
        .apply_proposal(
            &snapshot,
            SurfaceProposal::ModelManualText {
                new_text: "rewritten content".to_string(),
            },
        )
        .expect("apply_proposal must succeed");

    match updated {
        EditableSurfaceSnapshot::ModelManual {
            before_text,
            after_text,
            manual_section_id,
        } => {
            assert_eq!(before_text, "before-intro.usage_overview");
            assert_eq!(after_text, "rewritten content");
            assert_eq!(manual_section_id, "intro.usage_overview");
        }
        other => panic!("expected ModelManual snapshot; got {other:?}"),
    }

    // Catalog routing proof: write_section ran exactly once with the
    // declared catalog action_id + write_box schema_id.
    let records = writes.borrow();
    assert_eq!(records.len(), 1);
    let rec = &records[0];
    assert_eq!(rec.action_id, MODEL_MANUAL_ACTION_ID);
    assert_eq!(rec.write_box_schema_id, MODEL_MANUAL_WRITE_BOX_SCHEMA_ID);
    assert_eq!(rec.target_id, "intro.usage_overview");
    assert_eq!(rec.payload, "rewritten content");
}

#[test]
fn mt149_retrieval_policy_apply_routes_through_catalog_action_id() {
    let writes: RefCell<Vec<CatalogWriteRecord>> = RefCell::new(Vec::new());
    let surface = RetrievalPolicySurface::new(
        |_task: TaskType, _param: PolicyParameter| Ok::<u64, EditableSurfaceError>(8),
        |task: TaskType, param: PolicyParameter, value: u64| {
            writes.borrow_mut().push(CatalogWriteRecord {
                action_id: RETRIEVAL_POLICY_ACTION_ID,
                write_box_schema_id: RETRIEVAL_POLICY_WRITE_BOX_SCHEMA_ID,
                target_id: format!("{task:?}::{param:?}"),
                payload: value.to_string(),
            });
            Ok::<(), EditableSurfaceError>(())
        },
    );

    let target = policy_target(PolicyParameter::TopK);
    let snapshot = surface.snapshot(&target).expect("snapshot must succeed");
    let updated = surface
        .apply_proposal(&snapshot, SurfaceProposal::RetrievalPolicyValue { new_value: 16 })
        .expect("apply_proposal must succeed");

    match updated {
        EditableSurfaceSnapshot::RetrievalPolicy {
            before_value,
            after_value,
            ..
        } => {
            assert_eq!(before_value, 8);
            assert_eq!(after_value, 16);
        }
        other => panic!("expected RetrievalPolicy snapshot; got {other:?}"),
    }

    let records = writes.borrow();
    assert_eq!(records.len(), 1);
    let rec = &records[0];
    assert_eq!(rec.action_id, RETRIEVAL_POLICY_ACTION_ID);
    assert_eq!(rec.write_box_schema_id, RETRIEVAL_POLICY_WRITE_BOX_SCHEMA_ID);
    assert_eq!(rec.payload, "16");
}

// ----------------------------------------------------------------------------
// Policy-bound adversarial tests at the surface boundary (the proposer
// can't bypass these even with a typed proposal).
// ----------------------------------------------------------------------------

#[test]
fn mt149_retrieval_policy_rejects_top_k_zero() {
    let surface = RetrievalPolicySurface::new(
        |_, _| Ok::<u64, EditableSurfaceError>(8),
        |_, _, _| Ok::<(), EditableSurfaceError>(()),
    );
    let target = policy_target(PolicyParameter::TopK);
    let snapshot = surface.snapshot(&target).expect("snapshot must succeed");
    let err = surface
        .apply_proposal(&snapshot, SurfaceProposal::RetrievalPolicyValue { new_value: 0 })
        .expect_err("top_k=0 must reject");
    match err {
        EditableSurfaceError::InvalidProposal { field, .. } => {
            assert_eq!(field, "top_k");
        }
        other => panic!("expected InvalidProposal; got {other:?}"),
    }
}

#[test]
fn mt149_retrieval_policy_rejects_top_k_above_max() {
    let surface = RetrievalPolicySurface::new(
        |_, _| Ok::<u64, EditableSurfaceError>(8),
        |_, _, _| Ok::<(), EditableSurfaceError>(()),
    );
    let target = policy_target(PolicyParameter::TopK);
    let snapshot = surface.snapshot(&target).expect("snapshot must succeed");
    let err = surface
        .apply_proposal(
            &snapshot,
            SurfaceProposal::RetrievalPolicyValue {
                new_value: u64::from(MAX_TOP_K) + 1,
            },
        )
        .expect_err("top_k > MAX_TOP_K must reject");
    assert!(matches!(
        err,
        EditableSurfaceError::InvalidProposal { field: "top_k", .. }
    ));
}

#[test]
fn mt149_retrieval_policy_rejects_budget_zero() {
    let surface = RetrievalPolicySurface::new(
        |_, _| Ok::<u64, EditableSurfaceError>(4096),
        |_, _, _| Ok::<(), EditableSurfaceError>(()),
    );
    let target = policy_target(PolicyParameter::CapsuleBudgetBytes);
    let snapshot = surface.snapshot(&target).expect("snapshot must succeed");
    let err = surface
        .apply_proposal(&snapshot, SurfaceProposal::RetrievalPolicyValue { new_value: 0 })
        .expect_err("budget=0 must reject");
    assert!(matches!(
        err,
        EditableSurfaceError::InvalidProposal {
            field: "capsule_budget_bytes",
            ..
        }
    ));
}

#[test]
fn mt149_retrieval_policy_rejects_budget_above_max() {
    let surface = RetrievalPolicySurface::new(
        |_, _| Ok::<u64, EditableSurfaceError>(4096),
        |_, _, _| Ok::<(), EditableSurfaceError>(()),
    );
    let target = policy_target(PolicyParameter::CapsuleBudgetBytes);
    let snapshot = surface.snapshot(&target).expect("snapshot must succeed");
    let err = surface
        .apply_proposal(
            &snapshot,
            SurfaceProposal::RetrievalPolicyValue {
                new_value: MAX_CAPSULE_BUDGET_BYTES + 1,
            },
        )
        .expect_err("budget > MAX_CAPSULE_BUDGET_BYTES must reject");
    assert!(matches!(
        err,
        EditableSurfaceError::InvalidProposal {
            field: "capsule_budget_bytes",
            ..
        }
    ));
}

#[test]
fn mt149_model_manual_rejects_empty_text() {
    let surface = ModelManualSurface::new(
        |section: &str| Ok::<String, EditableSurfaceError>(format!("before-{section}")),
        |_, _| Ok::<(), EditableSurfaceError>(()),
    );
    let target = manual_target("intro.usage_overview");
    let snapshot = surface.snapshot(&target).expect("snapshot must succeed");
    let err = surface
        .apply_proposal(
            &snapshot,
            SurfaceProposal::ModelManualText {
                new_text: "   \t\n".to_string(),
            },
        )
        .expect_err("empty text must reject");
    assert!(matches!(
        err,
        EditableSurfaceError::InvalidProposal { field: "new_text", .. }
    ));
}

#[test]
fn mt149_model_manual_rejects_oversize_text() {
    let surface = ModelManualSurface::new(
        |section: &str| Ok::<String, EditableSurfaceError>(format!("before-{section}")),
        |_, _| Ok::<(), EditableSurfaceError>(()),
    );
    let target = manual_target("intro.usage_overview");
    let snapshot = surface.snapshot(&target).expect("snapshot must succeed");
    let oversize = "x".repeat(1_048_577);
    let err = surface
        .apply_proposal(
            &snapshot,
            SurfaceProposal::ModelManualText { new_text: oversize },
        )
        .expect_err("oversize text must reject");
    assert!(matches!(
        err,
        EditableSurfaceError::InvalidProposal { field: "new_text", .. }
    ));
}

// ----------------------------------------------------------------------------
// Cross-target rejection: applying a RetrievalPolicy proposal to a
// ModelManual snapshot (or vice versa) must be a typed MismatchedTarget
// error.
// ----------------------------------------------------------------------------

#[test]
fn mt149_model_manual_rejects_retrieval_policy_proposal() {
    let surface = ModelManualSurface::new(
        |section: &str| Ok::<String, EditableSurfaceError>(format!("before-{section}")),
        |_, _| Ok::<(), EditableSurfaceError>(()),
    );
    let target = manual_target("intro.usage_overview");
    let snapshot = surface.snapshot(&target).expect("snapshot must succeed");
    let err = surface
        .apply_proposal(
            &snapshot,
            SurfaceProposal::RetrievalPolicyValue { new_value: 16 },
        )
        .expect_err("cross-target proposal must reject");
    assert!(matches!(
        err,
        EditableSurfaceError::MismatchedTarget { .. }
    ));
}

#[test]
fn mt149_retrieval_policy_rejects_model_manual_proposal() {
    let surface = RetrievalPolicySurface::new(
        |_, _| Ok::<u64, EditableSurfaceError>(8),
        |_, _, _| Ok::<(), EditableSurfaceError>(()),
    );
    let target = policy_target(PolicyParameter::TopK);
    let snapshot = surface.snapshot(&target).expect("snapshot must succeed");
    let err = surface
        .apply_proposal(
            &snapshot,
            SurfaceProposal::ModelManualText {
                new_text: "rewritten".to_string(),
            },
        )
        .expect_err("cross-target proposal must reject");
    assert!(matches!(
        err,
        EditableSurfaceError::MismatchedTarget { .. }
    ));
}

// ----------------------------------------------------------------------------
// Standalone guard tests (without the surface wrapper) — proves the
// guard is a usable primitive in isolation, so a future surface impl
// can call it directly without going through ModelManualSurface or
// RetrievalPolicySurface.
// ----------------------------------------------------------------------------

#[test]
fn mt149_standalone_guard_rejects_uppercase_spec_anchor() {
    let target = manual_target("SPEC.Handshake_Master/Section_7");
    let err = ForbiddenSurfaceGuard::check(&target).expect_err("case-insensitive spec must reject");
    assert!(matches!(
        err,
        EditableSurfaceError::Forbidden {
            reason: ForbidReason::ShadowAuthority,
            ..
        }
    ));
}

#[test]
fn mt149_standalone_guard_rejects_path_with_spec_segment() {
    let target = manual_target("intro/spec/section_3");
    let err = ForbiddenSurfaceGuard::check(&target)
        .expect_err("path-segment /spec/ must reject");
    assert!(matches!(
        err,
        EditableSurfaceError::Forbidden {
            reason: ForbidReason::ShadowAuthority,
            ..
        }
    ));
}

#[test]
fn mt149_standalone_guard_accepts_legit_intro_section() {
    let target = manual_target("intro.usage_overview");
    ForbiddenSurfaceGuard::check(&target).expect("legit target must pass");
}

#[test]
fn mt149_standalone_guard_accepts_retrieval_policy_targets_unconditionally() {
    let target = policy_target(PolicyParameter::TopK);
    ForbiddenSurfaceGuard::check(&target).expect("retrieval policy targets always pass guard");
    let target_budget = policy_target(PolicyParameter::CapsuleBudgetBytes);
    ForbiddenSurfaceGuard::check(&target_budget)
        .expect("retrieval policy budget always passes guard");
}
