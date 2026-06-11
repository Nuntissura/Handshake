//! WP-KERNEL-005 MT-183 / MT-185 / MT-186 / MT-187 proof.
//!
//! Executes the real ModelManual source-row merges (Core/Data, Pose/ComfyUI,
//! Diagnostics-owned) and the manual drift guard, persists the runs through
//! `AtelierStore` against Handshake-managed PostgreSQL, re-reads them, and
//! asserts the canonical EventLedger events. Synthetic-manual tests prove the
//! normalize / missing-as-blocker / drift-finding logic on non-literal inputs.

mod atelier_pg_support;

use std::collections::BTreeSet;

use atelier_pg_support::database_url;
use handshake_core::atelier::model_manual_merge::{
    merge_manual_source_rows, model_manual_merge_event_family, normalize_manual_command_id,
    run_manual_drift_guard, wired_surface_fingerprint, ManualDriftKind, ManualMergeSourceKind,
    RegisteredSurfaceIndex, MERGE_BLOCKER_ID_NORMALIZATION_COLLISION,
    MERGE_BLOCKER_MISSING_FEATURE_GROUP, MERGE_BLOCKER_MISSING_SOURCE_ROW,
};
use handshake_core::atelier::AtelierStore;
use handshake_core::kernel::KernelEventType;
use handshake_core::model_manual::{
    model_manual, CommandReference, CommandStatus, Manual, ManualFeatureGroup,
};
use handshake_core::storage::{postgres::PostgresDatabase, Database};
use sqlx::postgres::PgPoolOptions;
use std::sync::Arc;
use uuid::Uuid;

async fn connected_store_with_ledger(url: &str) -> (AtelierStore, Arc<dyn Database>) {
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(url)
        .await
        .expect("connect to PostgreSQL");
    let database = PostgresDatabase::new(pool.clone());
    database
        .run_migrations()
        .await
        .expect("run kernel migrations");
    let database = database.into_arc();
    let store = AtelierStore::with_event_ledger(pool, database.clone());
    store.ensure_schema().await.expect("ensure atelier schema");
    (store, database)
}

const fn synthetic_row(
    id: &'static str,
    status: CommandStatus,
    ipc_channel: Option<&'static str>,
    schema_fields: &'static [&'static str],
) -> CommandReference {
    CommandReference {
        id,
        name: id,
        status,
        ipc_channel,
        tauri_command: None,
        schema_fields,
        cli_flag: None,
        description: "synthetic test row",
        expected_input: "synthetic",
        expected_output: "synthetic",
        common_errors: &[],
        recovery_steps: &[],
    }
}

// ---------------------------------------------------------------------------
// MT-185: Core/Data manual source merge
// ---------------------------------------------------------------------------

#[tokio::test]
async fn mt185_core_row_merge_merges_real_core_rows_and_persists_with_ledger() {
    let Some(url) = database_url().await else {
        eprintln!("SKIP mt185_core_row_merge: PostgreSQL unavailable");
        return;
    };
    let (store, database) = connected_store_with_ledger(&url).await;

    let outcome = merge_manual_source_rows(model_manual(), ManualMergeSourceKind::CoreData);
    assert!(
        outcome.blockers.is_empty(),
        "real manual Core/Data merge must produce no blockers: {:?}",
        outcome.blockers
    );

    // Independent expected count: the distinct command ids of the nine
    // Core/Data source groups, computed directly from the manual dataset.
    let core_group_ids = [
        "atelier_character_core",
        "atelier_media_intake",
        "atelier_collections_contact_sheets",
        "atelier_documents_scripts",
        "atelier_moodboards",
        "atelier_relationships",
        "atelier_search_tags_similarity",
        "atelier_exports",
        "atelier_reset_recovery",
    ];
    let expected_ids: BTreeSet<&str> = model_manual()
        .feature_groups
        .iter()
        .filter(|group| core_group_ids.contains(&group.id))
        .flat_map(|group| group.commands.iter().copied())
        .collect();
    assert_eq!(
        outcome.merged_rows.len(),
        expected_ids.len(),
        "merged row count must equal the distinct Core/Data source command count"
    );

    let create_character = outcome
        .merged_rows
        .iter()
        .find(|row| row.normalized_command_id == "atelier_create_character")
        .expect("Core/Data merge must include atelier_create_character");
    assert_eq!(create_character.source_group_id, "atelier_character_core");
    assert_eq!(create_character.source_mt, "MT-052");
    assert_eq!(create_character.row_kind, None);

    let backup_preflight = outcome
        .merged_rows
        .iter()
        .find(|row| row.normalized_command_id == "atelier_backup_restore_preflight")
        .expect("Core/Data merge must include atelier_backup_restore_preflight");
    assert_eq!(backup_preflight.source_group_id, "atelier_exports");
    assert_eq!(backup_preflight.source_mt, "MT-059/MT-073..MT-075");

    let reset = outcome
        .merged_rows
        .iter()
        .find(|row| row.normalized_command_id == "atelier_record_atelier_reset")
        .expect("Core/Data merge must include atelier_record_atelier_reset");
    assert_eq!(reset.source_mt, "MT-060");

    // Persist through the real store, re-read from PostgreSQL, and assert the
    // EventLedger mirror.
    let record = store
        .record_manual_row_merge(&outcome)
        .await
        .expect("persist Core/Data merge run");
    let reloaded = store
        .get_manual_row_merge(record.run_id)
        .await
        .expect("re-read Core/Data merge run from PostgreSQL");
    assert_eq!(reloaded, record);
    assert_eq!(reloaded.source_kind, ManualMergeSourceKind::CoreData);
    assert_eq!(reloaded.merged_rows, outcome.merged_rows);
    assert_eq!(reloaded.manual_version, model_manual().version);

    let latest = store
        .latest_manual_row_merge(ManualMergeSourceKind::CoreData)
        .await
        .expect("query latest Core/Data merge run")
        .expect("latest Core/Data merge run must exist");
    assert_eq!(latest.run_id, record.run_id);

    let kernel_events = database
        .list_kernel_events_for_aggregate(
            "atelier_model_manual_row_merge",
            &record.run_id.to_string(),
        )
        .await
        .expect("list merge EventLedger rows");
    let event = kernel_events
        .iter()
        .find(|event| {
            event.event_type == KernelEventType::AtelierDomainEventRecorded
                && event.payload["event_family"]
                    == model_manual_merge_event_family::MANUAL_ROW_MERGE_RECORDED
        })
        .expect("Core/Data merge must emit canonical EventLedger event");
    assert_eq!(
        event.payload["atelier_payload"]["source_kind"],
        serde_json::json!("core_data")
    );
    assert_eq!(
        event.payload["atelier_payload"]["merged_row_count"],
        serde_json::json!(outcome.merged_rows.len())
    );
    assert_eq!(
        event.payload["atelier_payload"]["blocker_count"],
        serde_json::json!(0)
    );
}

static CORE_MERGE_PARTIAL_MANUAL: Manual = Manual {
    version: "0.1.0-test",
    feature_groups: &[ManualFeatureGroup {
        id: "atelier_character_core",
        title: "Synthetic core group",
        description: "synthetic",
        commands: &["Atelier-Create-Character", "atelier_missing_surface"],
    }],
    command_reference: &[synthetic_row(
        "atelier_create_character",
        CommandStatus::Wired,
        None,
        &[],
    )],
    safety_constraints: &[],
    workflows: &[],
};

static CORE_MERGE_COLLISION_MANUAL: Manual = Manual {
    version: "0.1.1-test",
    feature_groups: &[ManualFeatureGroup {
        id: "atelier_character_core",
        title: "Synthetic core group",
        description: "synthetic",
        commands: &["atelier_create_character", "Atelier.Create.Character"],
    }],
    command_reference: &[synthetic_row(
        "atelier_create_character",
        CommandStatus::Wired,
        None,
        &[],
    )],
    safety_constraints: &[],
    workflows: &[],
};

#[test]
fn mt185_core_row_merge_normalizes_ids_and_marks_missing_as_blockers() {
    // Id normalization is a real transformation, not an echo.
    assert_eq!(
        normalize_manual_command_id("  Atelier--Create.Character "),
        "atelier_create_character"
    );
    assert_eq!(normalize_manual_command_id("_already__normal_"), "already_normal");

    let outcome =
        merge_manual_source_rows(&CORE_MERGE_PARTIAL_MANUAL, ManualMergeSourceKind::CoreData);

    // The hyphen/case-variant group command resolves to the snake_case manual
    // row through normalization.
    let merged = outcome
        .merged_rows
        .iter()
        .find(|row| row.normalized_command_id == "atelier_create_character")
        .expect("normalized id must resolve the variant raw command id");
    assert_eq!(merged.raw_command_id, "Atelier-Create-Character");
    assert_ne!(merged.raw_command_id, merged.normalized_command_id);

    // The group command without a manual row is a blocker, never fabricated.
    let missing_row = outcome
        .blockers
        .iter()
        .find(|blocker| blocker.expected_id == "atelier_missing_surface")
        .expect("missing source row must be marked as a blocker");
    assert_eq!(missing_row.reason, MERGE_BLOCKER_MISSING_SOURCE_ROW);
    assert_eq!(missing_row.source_mt, "MT-052");

    // The eight absent Core/Data source groups are blockers too.
    let missing_groups: Vec<_> = outcome
        .blockers
        .iter()
        .filter(|blocker| blocker.reason == MERGE_BLOCKER_MISSING_FEATURE_GROUP)
        .collect();
    assert_eq!(
        missing_groups.len(),
        8,
        "every absent expected source group must be a blocker: {missing_groups:?}"
    );
    assert!(missing_groups
        .iter()
        .any(|blocker| blocker.expected_id == "atelier_reset_recovery"));

    // Distinct raw ids collapsing onto one normalized id are a collision
    // blocker, not a silent merge.
    let collision_outcome =
        merge_manual_source_rows(&CORE_MERGE_COLLISION_MANUAL, ManualMergeSourceKind::CoreData);
    assert!(
        collision_outcome.blockers.iter().any(|blocker| {
            blocker.reason == MERGE_BLOCKER_ID_NORMALIZATION_COLLISION
                && blocker.expected_id == "atelier_create_character"
        }),
        "normalization collision must be surfaced: {:?}",
        collision_outcome.blockers
    );
}

// ---------------------------------------------------------------------------
// MT-186: Pose/ComfyUI manual source merge
// ---------------------------------------------------------------------------

#[tokio::test]
async fn mt186_pose_row_merge_merges_real_pose_rows_and_persists_with_ledger() {
    let Some(url) = database_url().await else {
        eprintln!("SKIP mt186_pose_row_merge: PostgreSQL unavailable");
        return;
    };
    let (store, database) = connected_store_with_ledger(&url).await;

    let outcome = merge_manual_source_rows(model_manual(), ManualMergeSourceKind::PoseComfy);
    assert!(
        outcome.blockers.is_empty(),
        "real manual Pose/ComfyUI merge must produce no blockers: {:?}",
        outcome.blockers
    );

    let pose_group_ids = [
        "atelier_pose_context_and_rig",
        "atelier_pose_sidecar_and_identity",
        "atelier_comfy_workflow_receipts",
        "atelier_pose_comfy_deferred_boundaries",
    ];
    let expected_ids: BTreeSet<&str> = model_manual()
        .feature_groups
        .iter()
        .filter(|group| pose_group_ids.contains(&group.id))
        .flat_map(|group| group.commands.iter().copied())
        .collect();
    assert_eq!(outcome.merged_rows.len(), expected_ids.len());

    let ingest_rig = outcome
        .merged_rows
        .iter()
        .find(|row| row.normalized_command_id == "atelier_ingest_pose_rig")
        .expect("Pose merge must include atelier_ingest_pose_rig");
    assert_eq!(ingest_rig.source_mt, "MT-122");
    assert_eq!(ingest_rig.source_group_id, "atelier_pose_context_and_rig");

    let comfy_receipt = outcome
        .merged_rows
        .iter()
        .find(|row| row.normalized_command_id == "atelier_record_comfy_workflow_receipt")
        .expect("Pose merge must include atelier_record_comfy_workflow_receipt");
    assert_eq!(comfy_receipt.source_mt, "MT-124");

    let deferred = outcome
        .merged_rows
        .iter()
        .find(|row| row.normalized_command_id == "atelier_set_calibration_blocked")
        .expect("Pose merge must include atelier_set_calibration_blocked");
    assert_eq!(deferred.source_mt, "MT-125");

    let record = store
        .record_manual_row_merge(&outcome)
        .await
        .expect("persist Pose/ComfyUI merge run");
    let reloaded = store
        .get_manual_row_merge(record.run_id)
        .await
        .expect("re-read Pose/ComfyUI merge run from PostgreSQL");
    assert_eq!(reloaded, record);
    assert_eq!(reloaded.source_kind, ManualMergeSourceKind::PoseComfy);
    assert_eq!(reloaded.merged_rows, outcome.merged_rows);

    let kernel_events = database
        .list_kernel_events_for_aggregate(
            "atelier_model_manual_row_merge",
            &record.run_id.to_string(),
        )
        .await
        .expect("list merge EventLedger rows");
    let event = kernel_events
        .iter()
        .find(|event| {
            event.event_type == KernelEventType::AtelierDomainEventRecorded
                && event.payload["event_family"]
                    == model_manual_merge_event_family::MANUAL_ROW_MERGE_RECORDED
        })
        .expect("Pose/ComfyUI merge must emit canonical EventLedger event");
    assert_eq!(
        event.payload["atelier_payload"]["source_kind"],
        serde_json::json!("pose_comfy")
    );
}

static POSE_MERGE_PARTIAL_MANUAL: Manual = Manual {
    version: "0.2.0-test",
    feature_groups: &[ManualFeatureGroup {
        id: "atelier_pose_context_and_rig",
        title: "Synthetic pose group",
        description: "synthetic",
        commands: &["atelier_ingest_pose_rig"],
    }],
    command_reference: &[synthetic_row(
        "atelier_ingest_pose_rig",
        CommandStatus::Wired,
        None,
        &[],
    )],
    safety_constraints: &[],
    workflows: &[],
};

#[test]
fn mt186_pose_row_merge_marks_missing_groups_as_blockers() {
    let outcome =
        merge_manual_source_rows(&POSE_MERGE_PARTIAL_MANUAL, ManualMergeSourceKind::PoseComfy);
    assert_eq!(outcome.merged_rows.len(), 1);

    let missing_groups: BTreeSet<&str> = outcome
        .blockers
        .iter()
        .filter(|blocker| blocker.reason == MERGE_BLOCKER_MISSING_FEATURE_GROUP)
        .map(|blocker| blocker.expected_id.as_str())
        .collect();
    assert_eq!(
        missing_groups,
        BTreeSet::from([
            "atelier_pose_sidecar_and_identity",
            "atelier_comfy_workflow_receipts",
            "atelier_pose_comfy_deferred_boundaries",
        ]),
        "absent Pose/ComfyUI source groups must be blockers"
    );
}

// ---------------------------------------------------------------------------
// MT-187: Diagnostics-owned manual source merge
// ---------------------------------------------------------------------------

#[tokio::test]
async fn mt187_owned_row_merge_classifies_row_kinds_and_persists_with_ledger() {
    let Some(url) = database_url().await else {
        eprintln!("SKIP mt187_owned_row_merge: PostgreSQL unavailable");
        return;
    };
    let (store, database) = connected_store_with_ledger(&url).await;

    let outcome =
        merge_manual_source_rows(model_manual(), ManualMergeSourceKind::DiagnosticsOwned);
    assert!(
        outcome.blockers.is_empty(),
        "real manual Diagnostics-owned merge must produce no blockers: {:?}",
        outcome.blockers
    );

    // Every supported row kind is represented in the merged dataset.
    let row_kinds: BTreeSet<&str> = outcome
        .merged_rows
        .iter()
        .filter_map(|row| row.row_kind.as_deref())
        .collect();
    assert_eq!(
        row_kinds,
        BTreeSet::from(["manual", "action", "state", "error", "bundle"]),
        "Diagnostics-owned merge must cover manual/action/state/error/bundle rows"
    );

    let problem_store = outcome
        .merged_rows
        .iter()
        .find(|row| row.normalized_command_id == "diagnostics_problem_store_query")
        .expect("Diagnostics-owned merge must include diagnostics_problem_store_query");
    assert_eq!(problem_store.row_kind.as_deref(), Some("error"));
    assert_eq!(problem_store.status, "planned");

    let bundle_export = outcome
        .merged_rows
        .iter()
        .find(|row| row.normalized_command_id == "diagnostics_debug_bundle_export")
        .expect("Diagnostics-owned merge must include diagnostics_debug_bundle_export");
    assert_eq!(bundle_export.row_kind.as_deref(), Some("bundle"));

    let manual_get = outcome
        .merged_rows
        .iter()
        .find(|row| row.normalized_command_id == "model_manual_get")
        .expect("Diagnostics-owned merge must include model_manual_get");
    assert_eq!(manual_get.row_kind.as_deref(), Some("manual"));
    assert_eq!(manual_get.status, "wired");

    let record = store
        .record_manual_row_merge(&outcome)
        .await
        .expect("persist Diagnostics-owned merge run");
    let reloaded = store
        .get_manual_row_merge(record.run_id)
        .await
        .expect("re-read Diagnostics-owned merge run from PostgreSQL");
    assert_eq!(reloaded, record);
    assert_eq!(reloaded.source_kind, ManualMergeSourceKind::DiagnosticsOwned);
    assert_eq!(reloaded.merged_rows, outcome.merged_rows);

    let kernel_events = database
        .list_kernel_events_for_aggregate(
            "atelier_model_manual_row_merge",
            &record.run_id.to_string(),
        )
        .await
        .expect("list merge EventLedger rows");
    let event = kernel_events
        .iter()
        .find(|event| {
            event.event_type == KernelEventType::AtelierDomainEventRecorded
                && event.payload["event_family"]
                    == model_manual_merge_event_family::MANUAL_ROW_MERGE_RECORDED
        })
        .expect("Diagnostics-owned merge must emit canonical EventLedger event");
    assert_eq!(
        event.payload["atelier_payload"]["source_kind"],
        serde_json::json!("diagnostics_owned")
    );
}

static OWNED_MERGE_PARTIAL_MANUAL: Manual = Manual {
    version: "0.3.0-test",
    feature_groups: &[ManualFeatureGroup {
        id: "diagnostics_owned_row_merge",
        title: "Synthetic diagnostics group",
        description: "synthetic",
        commands: &["model_manual_get"],
    }],
    command_reference: &[
        synthetic_row(
            "model_manual_get",
            CommandStatus::Wired,
            None,
            &[],
        ),
        synthetic_row(
            "kernel_action_catalog_view",
            CommandStatus::Wired,
            None,
            &[],
        ),
        synthetic_row(
            "kernel_inspector_session_state",
            CommandStatus::Wired,
            None,
            &[],
        ),
        synthetic_row(
            "kernel_inspector_event_ledger_tail",
            CommandStatus::Wired,
            None,
            &[],
        ),
        synthetic_row(
            "model_manual_list_commands",
            CommandStatus::Wired,
            None,
            &[],
        ),
        synthetic_row("model_manual_search", CommandStatus::Wired, None, &[]),
        synthetic_row(
            "diagnostics_problem_store_query",
            CommandStatus::Planned,
            None,
            &[],
        ),
        // diagnostics_debug_bundle_export intentionally absent.
    ],
    safety_constraints: &[],
    workflows: &[],
};

#[test]
fn mt187_owned_row_merge_marks_missing_bundle_row_as_blocker() {
    let outcome = merge_manual_source_rows(
        &OWNED_MERGE_PARTIAL_MANUAL,
        ManualMergeSourceKind::DiagnosticsOwned,
    );
    assert_eq!(outcome.merged_rows.len(), 7);

    let bundle_blocker = outcome
        .blockers
        .iter()
        .find(|blocker| blocker.expected_id == "diagnostics_debug_bundle_export")
        .expect("missing bundle row must be a blocker, never fabricated");
    assert_eq!(bundle_blocker.reason, MERGE_BLOCKER_MISSING_SOURCE_ROW);
    assert_eq!(bundle_blocker.source_mt, "bundle");
    assert!(
        !outcome
            .merged_rows
            .iter()
            .any(|row| row.normalized_command_id == "diagnostics_debug_bundle_export"),
        "the missing row must not be fabricated into the merged dataset"
    );
}

// ---------------------------------------------------------------------------
// MT-183: manual drift guard
// ---------------------------------------------------------------------------

static DRIFTY_MANUAL: Manual = Manual {
    version: "0.4.0-test",
    feature_groups: &[ManualFeatureGroup {
        id: "synthetic_group",
        title: "Synthetic drift group",
        description: "synthetic",
        commands: &["phantom_command", "ghost_command", "dup_row"],
    }],
    command_reference: &[
        // Wired with an unregistered IPC channel -> unresolved surface.
        synthetic_row(
            "phantom_command",
            CommandStatus::Wired,
            Some("kernel.fabricated.route"),
            &[],
        ),
        // Not referenced by any feature group -> orphan manual row.
        synthetic_row("unlisted_row", CommandStatus::Planned, None, &[]),
        // Two rows collapsing onto one normalized id -> collision.
        synthetic_row("dup_row", CommandStatus::Planned, None, &[]),
        synthetic_row("Dup.Row", CommandStatus::Planned, None, &[]),
        // "ghost_command" has no row at all -> missing manual row.
    ],
    safety_constraints: &[],
    workflows: &[],
};

#[test]
fn mt183_drift_guard_flags_unresolved_orphan_and_collision_drift() {
    let surfaces = RegisteredSurfaceIndex::handshake_runtime_default();
    let findings = run_manual_drift_guard(&DRIFTY_MANUAL, &surfaces);

    let kinds_for = |kind: ManualDriftKind| -> Vec<&str> {
        findings
            .iter()
            .filter(|finding| finding.drift_kind == kind)
            .filter_map(|finding| finding.command_id.as_deref())
            .collect()
    };

    assert_eq!(
        kinds_for(ManualDriftKind::WiredUnresolvedSurface),
        vec!["phantom_command"],
        "the fabricated wired route must be flagged: {findings:?}"
    );
    assert_eq!(
        kinds_for(ManualDriftKind::MissingManualRow),
        vec!["ghost_command"],
        "the group command without a manual row must be flagged: {findings:?}"
    );
    assert_eq!(
        kinds_for(ManualDriftKind::OrphanManualRow),
        vec!["unlisted_row"],
        "the row in no feature group must be flagged: {findings:?}"
    );
    assert_eq!(
        kinds_for(ManualDriftKind::IdNormalizationCollision),
        vec!["Dup.Row"],
        "the normalization collision must be flagged: {findings:?}"
    );

    // The shipped manual against the real registered runtime surface is
    // drift-free; this is the live negative check, not an echo.
    let real_findings =
        run_manual_drift_guard(model_manual(), &RegisteredSurfaceIndex::handshake_runtime_default());
    assert!(
        real_findings.is_empty(),
        "shipped ModelManual must be drift-free against the registered surface: {real_findings:?}"
    );
}

static VERSION_GUARD_MANUAL_V1: Manual = Manual {
    version: "1.0.0-test",
    feature_groups: &[ManualFeatureGroup {
        id: "version_guard_group",
        title: "Version guard group",
        description: "synthetic",
        commands: &["probe_row"],
    }],
    command_reference: &[synthetic_row(
        "probe_row",
        CommandStatus::Wired,
        Some("kernel.action_catalog.view"),
        &["alpha"],
    )],
    safety_constraints: &[],
    workflows: &[],
};

// Same version as V1 but a different wired surface (extra schema field):
// recording this after V1 is the HBR-MAN-001 violation the guard must flag.
static VERSION_GUARD_MANUAL_V1_DRIFTED: Manual = Manual {
    version: "1.0.0-test",
    feature_groups: &[ManualFeatureGroup {
        id: "version_guard_group",
        title: "Version guard group",
        description: "synthetic",
        commands: &["probe_row"],
    }],
    command_reference: &[synthetic_row(
        "probe_row",
        CommandStatus::Wired,
        Some("kernel.action_catalog.view"),
        &["alpha", "beta"],
    )],
    safety_constraints: &[],
    workflows: &[],
};

static VERSION_GUARD_MANUAL_V2: Manual = Manual {
    version: "1.1.0-test",
    feature_groups: &[ManualFeatureGroup {
        id: "version_guard_group",
        title: "Version guard group",
        description: "synthetic",
        commands: &["probe_row"],
    }],
    command_reference: &[synthetic_row(
        "probe_row",
        CommandStatus::Wired,
        Some("kernel.action_catalog.view"),
        &["alpha", "beta"],
    )],
    safety_constraints: &[],
    workflows: &[],
};

#[tokio::test]
async fn mt183_drift_guard_detects_wired_surface_diff_without_version_bump() {
    let Some(url) = database_url().await else {
        eprintln!("SKIP mt183_drift_guard_version_bump: PostgreSQL unavailable");
        return;
    };
    let (store, database) = connected_store_with_ledger(&url).await;
    let surfaces = RegisteredSurfaceIndex::handshake_runtime_default();
    let scope = format!("test-scope-{}", Uuid::new_v4());

    assert_ne!(
        wired_surface_fingerprint(&VERSION_GUARD_MANUAL_V1),
        wired_surface_fingerprint(&VERSION_GUARD_MANUAL_V1_DRIFTED),
        "the drifted manual must change the wired-surface fingerprint"
    );

    let run1 = store
        .record_manual_drift_guard_run(&scope, &VERSION_GUARD_MANUAL_V1, &surfaces)
        .await
        .expect("record first guard run");
    assert!(!run1.wired_surface_changed);
    assert!(
        run1.findings.is_empty(),
        "baseline guard run must be clean: {:?}",
        run1.findings
    );

    // Wired surface changed, version did not -> HBR-MAN-001 drift finding.
    let run2 = store
        .record_manual_drift_guard_run(&scope, &VERSION_GUARD_MANUAL_V1_DRIFTED, &surfaces)
        .await
        .expect("record drifted guard run");
    assert!(run2.wired_surface_changed);
    let version_finding = run2
        .findings
        .iter()
        .find(|finding| finding.drift_kind == ManualDriftKind::ManualVersionNotBumped)
        .expect("wired-surface diff without version bump must be flagged");
    assert!(version_finding.detail.contains("1.0.0-test"));

    // The persisted run re-reads identically from PostgreSQL.
    let reloaded = store
        .get_manual_drift_guard_run(run2.run_id)
        .await
        .expect("re-read drifted guard run from PostgreSQL");
    assert_eq!(reloaded, run2);
    assert_eq!(reloaded.guard_scope, scope);
    assert_eq!(reloaded.manual_version, "1.0.0-test");

    // Bumping the version with an unchanged surface clears the drift.
    let run3 = store
        .record_manual_drift_guard_run(&scope, &VERSION_GUARD_MANUAL_V2, &surfaces)
        .await
        .expect("record bumped guard run");
    assert!(!run3.wired_surface_changed);
    assert!(
        run3.findings.is_empty(),
        "bumped run must be clean: {:?}",
        run3.findings
    );

    let kernel_events = database
        .list_kernel_events_for_aggregate(
            "atelier_model_manual_drift_guard",
            &run2.run_id.to_string(),
        )
        .await
        .expect("list drift guard EventLedger rows");
    let event = kernel_events
        .iter()
        .find(|event| {
            event.event_type == KernelEventType::AtelierDomainEventRecorded
                && event.payload["event_family"]
                    == model_manual_merge_event_family::MANUAL_DRIFT_GUARD_RECORDED
        })
        .expect("drift guard run must emit canonical EventLedger event");
    assert_eq!(
        event.payload["atelier_payload"]["wired_surface_changed"],
        serde_json::json!(true)
    );
    assert_eq!(
        event.payload["atelier_payload"]["finding_count"],
        serde_json::json!(run2.findings.len())
    );
}

#[tokio::test]
async fn mt183_drift_guard_real_manual_persists_clean_run() {
    let Some(url) = database_url().await else {
        eprintln!("SKIP mt183_drift_guard_real_manual: PostgreSQL unavailable");
        return;
    };
    let (store, database) = connected_store_with_ledger(&url).await;
    let scope = format!("test-real-manual-{}", Uuid::new_v4());

    let record = store
        .record_manual_drift_guard_run(
            &scope,
            model_manual(),
            &RegisteredSurfaceIndex::handshake_runtime_default(),
        )
        .await
        .expect("record real-manual guard run");
    assert!(
        record.findings.is_empty(),
        "shipped manual guard run must be clean: {:?}",
        record.findings
    );
    assert_eq!(record.manual_version, model_manual().version);
    assert!(record.wired_surface_sha256.starts_with("sha256:"));

    let reloaded = store
        .get_manual_drift_guard_run(record.run_id)
        .await
        .expect("re-read real-manual guard run from PostgreSQL");
    assert_eq!(reloaded, record);

    let kernel_events = database
        .list_kernel_events_for_aggregate(
            "atelier_model_manual_drift_guard",
            &record.run_id.to_string(),
        )
        .await
        .expect("list drift guard EventLedger rows");
    assert!(
        kernel_events.iter().any(|event| {
            event.event_type == KernelEventType::AtelierDomainEventRecorded
                && event.payload["event_family"]
                    == model_manual_merge_event_family::MANUAL_DRIFT_GUARD_RECORDED
        }),
        "real-manual guard run must emit canonical EventLedger event"
    );
}
