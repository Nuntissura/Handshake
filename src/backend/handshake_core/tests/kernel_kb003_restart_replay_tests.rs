//! MT-077 Restart Replay Test.
//!
//! Acceptance (MT-077.json): "prove state survives restart.
//! Acceptance: replay is complete from durable product state."
//!
//! The test builds a full KB003 chain:
//!     SandboxPolicy -> SandboxRun -> ValidationRun -> PromotionDecision -> PromotionReceipt
//! inside embedded SurrealDB, closes its handles, and reopens the same directory.
//! Replay loads durable rows without copying runtime state.
//!
//!   1. Loads rows through `SurrealKb003Storage::load_run_for_replay`.
//!   2. Feeds the bag through `kernel::sandbox::replay_projection::reconstruct_projection`
//!      to rebuild the `DccSandboxProjectionV1`.
//!   3. Wraps that projection into the top-level `DccKb003RollupV1` (Batch G)
//!      and asserts the rebuilt rollup is byte-equal (via canonical JSON) to
//!      the pre-restart rollup.
//!
//! Compared fields on `DccKb003RollupV1`:
//!   * `projection_family_id`
//!   * `sandbox_run_id`
//!   * `projection.run_id`, `projection.policy_version_id`,
//!     `projection.run_status`, `projection.outcome`, `projection.workspace_id`,
//!     `projection.workspace_root_relative`, `projection.capability_rows`,
//!     `projection.validation`, `projection.promotion`,
//!     `projection.source_schema_ids`
//!   * `blocked_overlay.overlay_family_id`
//!   * `lane_wake_timeline` count
//!   * `promotion_control.eligibility`
//!   * `manual_hints` list
//!
//! The whole rollup is compared via `serde_json::Value` equality which covers
//! every public field above without enumerating them by hand.

use chrono::Utc;
use handshake_core::kernel::dcc_kb003_blocked_reasons::DccKb003BlockedReasonOverlayV1;
use handshake_core::kernel::dcc_kb003_model_manual_hints::DccKb003ManualHintsV1;
use handshake_core::kernel::dcc_kb003_promotion_control_state::DccKb003PromotionControlStateV1;
use handshake_core::kernel::dcc_kb003_rollup::DccKb003RollupV1;
use handshake_core::kernel::mte_authority_mutation_boundary::AuthorityMutationActor;
use handshake_core::kernel::sandbox::policy::SandboxPolicyV1;
use handshake_core::kernel::sandbox::replay_projection::{
    reconstruct_projection, ReplayInputsV1, ReplayPromotionFactsV1, ReplayValidationFactsV1,
};
use handshake_core::kernel::sandbox::run::{SandboxRunId, SandboxRunStatus, SandboxRunV1};
use handshake_core::kernel::sandbox::workspace::SandboxWorkspaceV1;
use handshake_core::storage::kb003_storage::{
    Kb003Storage, PromotionDecisionRowV1, PromotionReceiptRowV1, ValidationRunRowV1,
};
use handshake_core::storage::surreal::SurrealKb003Storage;
use serde_json::json;

mod kb003_surreal_support;

fn build_workspace() -> SandboxWorkspaceV1 {
    SandboxWorkspaceV1::new_default("kb003-mt077", "handshake-product/kb003/work/mt077")
}

fn build_chain_in_storage(
    store: &mut SurrealKb003Storage,
    workspace: &SandboxWorkspaceV1,
) -> (
    SandboxRunV1,
    SandboxPolicyV1,
    ValidationRunRowV1,
    PromotionDecisionRowV1,
    PromotionReceiptRowV1,
) {
    // 1. Policy version.
    let policy = SandboxPolicyV1::default_deny("mt077-baseline");
    store.insert_sandbox_policy_version(&policy).unwrap();

    // 2. Sandbox run, in Completed state with stable identity.
    let run = SandboxRunV1 {
        run_id: SandboxRunId("SBX-mt077-stable".into()),
        kernel_task_run_id: "KTR-mt077".into(),
        session_run_id: "SES-mt077".into(),
        adapter_kind: "policy_scoped_local".into(),
        policy_version_id: policy.version_id(),
        workspace_id: workspace.workspace_id.clone(),
        status: SandboxRunStatus::Requested,
        requested_at_utc: Utc::now(),
        started_at_utc: None,
        finished_at_utc: None,
        denial_id: None,
        artifact_refs: vec!["kb003://sandbox_log/h1aaaaaaaaaaaaaa".into()],
        terminal_cause: None,
        requested_capabilities: vec![],
    };
    let run_id = run.run_id.0.clone();
    store.insert_sandbox_run(&run).unwrap();
    store
        .update_sandbox_run_status(&run_id, SandboxRunStatus::Started)
        .unwrap();
    store
        .update_sandbox_run_status(&run_id, SandboxRunStatus::Completed)
        .unwrap();

    // 3. Validation run.
    let vr = ValidationRunRowV1 {
        validation_run_id: "VR-mt077-stable".into(),
        sandbox_run_id: run_id.clone(),
        descriptor_id: "DESC-mt077".into(),
        verdict: "PASS".into(),
        check_count: 4,
        failed_check_count: 0,
        report_artifact_ref: Some("kb003://validation_report/h2bbbbbbbbbbbbbb".into()),
        started_at_utc: "2026-05-17T00:00:00Z".into(),
        finished_at_utc: "2026-05-17T00:00:01Z".into(),
        summary_json: json!({"checks": ["lint", "fmt", "unit", "schema"]}),
    };
    store.insert_validation_run(&vr).unwrap();

    // 4. Promotion decision.
    let dec = PromotionDecisionRowV1 {
        decision_id: "PD-mt077-stable".into(),
        validation_run_id: vr.validation_run_id.clone(),
        decision: "PROMOTED".into(),
        rationale_short: "checks green".into(),
        decided_at_utc: "2026-05-17T00:00:02Z".into(),
    };
    store
        .insert_promotion_decision(&dec, AuthorityMutationActor::PromotionGate)
        .unwrap();

    // 5. Promotion receipt.
    let receipt = PromotionReceiptRowV1 {
        receipt_id: "PR-mt077-stable".into(),
        decision_id: dec.decision_id.clone(),
        idempotency_key: "IK-mt077-stable".into(),
        payload_hash: "h-canonical-payload-stable".into(),
        artifact_ref: Some("kb003://promotion_receipt/h3ccccccccccccc".into()),
        issued_at_utc: "2026-05-17T00:00:03Z".into(),
    };
    let stored = store
        .insert_promotion_receipt(&receipt, AuthorityMutationActor::PromotionGate)
        .unwrap();
    assert_eq!(stored, receipt.receipt_id);

    // Re-fetch run with terminal status for the projection.
    let final_run = store
        .load_run_for_replay(&run_id, &policy.version_id())
        .expect("run row present after completion")
        .run;

    (final_run, policy, vr, dec, receipt)
}

fn build_rollup_for_store(
    store: &SurrealKb003Storage,
    workspace: &SandboxWorkspaceV1,
    run_id: &str,
    policy_version_id: &str,
) -> DccKb003RollupV1 {
    let bag = store
        .load_run_for_replay(run_id, policy_version_id)
        .expect("durable rows must be replay-loadable");
    let validation_facts = bag.validation.as_ref().map(|v| ReplayValidationFactsV1 {
        validation_run_id: v.validation_run_id.clone(),
        verdict: v.verdict.clone(),
        check_count: v.check_count,
        failed_check_count: v.failed_check_count,
        report_artifact_ref: v.report_artifact_ref.clone(),
    });
    let promotion_facts = bag.decision.as_ref().map(|d| {
        let receipt = bag.receipt.as_ref();
        ReplayPromotionFactsV1 {
            decision_id: d.decision_id.clone(),
            decision: d.decision.clone(),
            receipt_id: receipt.map(|r| r.receipt_id.clone()),
            receipt_artifact_ref: receipt.and_then(|r| r.artifact_ref.clone()),
            rationale_short: d.rationale_short.clone(),
        }
    });
    let arts = bag.run.artifact_refs.clone();
    let projection = reconstruct_projection(ReplayInputsV1 {
        run: &bag.run,
        policy: &bag.policy,
        workspace,
        denial: None,
        validation: validation_facts.as_ref(),
        promotion: promotion_facts.as_ref(),
        artifact_refs: &arts,
        // Round 1 H-A1 fix added this field — replay must accept it.
        artifact_classes: &[],
    });
    let control = DccKb003PromotionControlStateV1::derive(&projection, true, Some("operator_id"));
    let hints = DccKb003ManualHintsV1::derive(&projection, &control);
    DccKb003RollupV1::new(
        projection,
        DccKb003BlockedReasonOverlayV1::new(vec![]),
        vec![],
        control,
        hints,
        None,
    )
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn full_chain_survives_simulated_restart_and_rollup_replays_identically() {
    let workspace = build_workspace();

    // ---- Pre-restart store: build the full chain ----
    let directory = tempfile::tempdir().expect("isolated KB003 directory");
    let mut pre_store = kb003_surreal_support::open(directory.path()).await;
    let (run, policy, _vr, _dec, _receipt) = build_chain_in_storage(&mut pre_store, &workspace);
    let policy_version_id = policy.version_id();
    let run_id = run.run_id.0.clone();

    let pre_rollup = build_rollup_for_store(&pre_store, &workspace, &run_id, &policy_version_id);
    assert!(pre_rollup.is_self_describing());

    // Preserve the historical test name; this is real backend close/reopen
    // in the same process, without vector snapshots.
    let post_store = kb003_surreal_support::reopen(pre_store, directory.path()).await;

    // ---- Rebuild rollup from durable state only ----
    let post_rollup = build_rollup_for_store(&post_store, &workspace, &run_id, &policy_version_id);

    // ---- Assert the rollup is byte-equal via canonical JSON ----
    let pre_json = serde_json::to_value(&pre_rollup).expect("pre serialises");
    let post_json = serde_json::to_value(&post_rollup).expect("post serialises");
    assert_eq!(
        pre_json, post_json,
        "DccKb003RollupV1 must be byte-equal across restart-replay; pre={pre_json}\npost={post_json}"
    );

    // ---- Additional load-bearing field-level checks ----
    assert_eq!(
        post_rollup.projection_family_id,
        DccKb003RollupV1::FAMILY_ID
    );
    assert_eq!(post_rollup.sandbox_run_id, run_id);
    assert_eq!(post_rollup.projection.policy_version_id, policy_version_id);
    assert_eq!(
        post_rollup.projection.run_status,
        SandboxRunStatus::Completed
    );
    assert!(post_rollup.projection.validation.is_some());
    assert!(post_rollup.projection.promotion.is_some());
    assert_eq!(post_rollup.projection.workspace_id, workspace.workspace_id);
    assert_eq!(
        post_rollup.projection.workspace_root_relative,
        workspace.root_relative_path
    );
    // capability_rows covers every SandboxCapability.
    assert_eq!(
        post_rollup.projection.capability_rows.len(),
        handshake_core::kernel::sandbox::policy::SandboxCapability::ALL.len(),
        "all capabilities must be present in rebuilt projection"
    );
    // Source-schema list survives the round-trip.
    assert!(!post_rollup.projection.source_schema_ids.is_empty());
    kb003_surreal_support::close(post_store).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn replay_uses_only_durable_rows_no_session_state() {
    // Build a chain, drop the original store, and confirm the rollup can still
    // be reconstructed purely from the persisted rows. This guards
    // MT-016's "no provider chat / terminal scrollback / transient log" rule.
    let workspace = build_workspace();
    let directory = tempfile::tempdir().expect("isolated KB003 directory");
    let mut store = kb003_surreal_support::open(directory.path()).await;
    let (run, policy, _vr, _dec, _receipt) = build_chain_in_storage(&mut store, &workspace);
    let run_id = run.run_id.0.clone();
    let policy_version_id = policy.version_id();

    let fresh = kb003_surreal_support::reopen(store, directory.path()).await;

    let rollup = build_rollup_for_store(&fresh, &workspace, &run_id, &policy_version_id);
    assert!(
        rollup.is_self_describing(),
        "rollup must be self-describing from durable rows alone"
    );
    // Promotion decision survived as PROMOTED.
    let prom = rollup
        .projection
        .promotion
        .expect("promotion summary present");
    assert_eq!(prom.decision, "PROMOTED");
    assert_eq!(prom.receipt_id.as_deref(), Some("PR-mt077-stable"));
    kb003_surreal_support::close(fresh).await;
}
