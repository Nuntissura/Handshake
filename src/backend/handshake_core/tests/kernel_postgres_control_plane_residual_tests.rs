use handshake_core::kernel::{
    action_catalog::{kernel002_action_catalog, validate_kernel_action_catalog},
    action_envelope::AuthorityEffect,
    fold_manifest::LEGACY_CACHE_OFFLINE_BOUNDARY_STUB_ID,
    postgres_control_plane_residual::{
        project_postgres_control_plane_residual_scope,
        validate_postgres_control_plane_residual_scope, LegacyCacheBoundaryRole,
        PostgresControlPlaneResidualScopeV1, PostgresResidualDisposition,
        PostgresResidualObligationKind, PostgresResidualObligationV1, StorageAuthorityMode,
    },
};

#[test]
fn postgres_residual_scope_preserves_folded_stubs_and_maps_without_reopening_bundle() {
    let scope = sample_scope();

    validate_postgres_control_plane_residual_scope(&scope).expect("residual scope validates");
    let projection = project_postgres_control_plane_residual_scope(&scope)
        .expect("residual scope should project");

    assert_eq!(
        projection.bundle_stub_id,
        "WP-1-Postgres-Control-Plane-Shift-Bundle-v1"
    );
    assert!(!projection.reopens_old_bundle);
    assert_eq!(projection.folded_stub_ids.len(), 7);
    for kind in required_residual_kinds() {
        assert!(
            projection.residual_kinds.contains(&kind),
            "missing residual kind: {kind:?}"
        );
    }
    assert!(projection
        .kernel002_obligation_ids
        .contains(&"postgres-live-service-proof".to_string()));
    assert!(projection
        .kernel003_obligation_ids
        .contains(&"modelsession-queue-workers".to_string()));
    assert!(projection
        .kernel004_obligation_ids
        .contains(&"dcc-postgres-projections".to_string()));
    assert!(projection
        .excluded_stub_ids
        .contains(&"WP-1-Loom-MVP-v1".to_string()));
}

#[test]
fn postgres_residual_scope_rejects_sqlite_as_authority_for_postgres_required_work() {
    let mut scope = sample_scope();
    let obligation = scope
        .obligations
        .iter_mut()
        .find(|obligation| obligation.kind == PostgresResidualObligationKind::ModelSessionQueue)
        .expect("modelsession obligation exists");
    obligation.storage_authority_mode = StorageAuthorityMode::LegacyCacheOnly;

    let errors = validate_postgres_control_plane_residual_scope(&scope)
        .expect_err("SQLite cannot become authority for Postgres-required residuals");

    assert!(
        errors.iter().any(|error| {
            error.field == "storage_authority_mode" && error.message.contains("Postgres-required")
        }),
        "expected Postgres-required storage denial, got {errors:?}"
    );
}

#[test]
fn kernel_action_catalog_exposes_postgres_residual_projection_action() {
    let catalog = kernel002_action_catalog();
    validate_kernel_action_catalog(&catalog).expect("catalog must validate");

    let action = catalog
        .action("kernel.postgres_residual.project")
        .expect("Postgres residual projection action must be cataloged");

    assert_eq!(action.authority_effect, AuthorityEffect::ProjectionOnly);
    assert!(action
        .validation_hooks
        .iter()
        .any(|hook| hook.hook_id == "postgres_residual_mapping"));
    assert!(action
        .dcc_preview
        .primary_state_fields
        .contains(&"disposition".to_string()));
}

fn sample_scope() -> PostgresControlPlaneResidualScopeV1 {
    PostgresControlPlaneResidualScopeV1 {
        schema_id: "hsk.kernel.postgres_control_plane_residual_scope@1".to_string(),
        scope_id: "kernel002-postgres-residual-scope-mt022".to_string(),
        bundle_stub_id: "WP-1-Postgres-Control-Plane-Shift-Bundle-v1".to_string(),
        reopens_old_bundle: false,
        obligations: vec![
            obligation(
                "postgres-live-service-proof",
                "WP-1-Postgres-Dev-Test-Container-Matrix-v1",
                PostgresResidualObligationKind::LiveServiceProof,
                PostgresResidualDisposition::EnvironmentBlocked,
                Some("WP-KERNEL-002-CRDT-Workspace-Write-Box-Preuse-Hardening-v1"),
                StorageAuthorityMode::PostgresRequired,
                Some("cargo test -p handshake_core --target-dir ../Handshake_Artifacts/handshake-cargo-target"),
                Some("libduckdb-sys native MSVC build fails before Rust test compilation"),
            ),
            obligation(
                "postgres-migration-seed-proof",
                "WP-1-Postgres-Dev-Test-Container-Matrix-v1",
                PostgresResidualObligationKind::MigrationSeed,
                PostgresResidualDisposition::MappedToKernel003,
                Some("WP-KERNEL-003-Postgres-Control-Plane-Activation-v1"),
                StorageAuthorityMode::PostgresRequired,
                Some("cargo test -p handshake_core postgres_migration"),
                None,
            ),
            obligation(
                "postgres-proof-command-matrix",
                "WP-1-Postgres-Dev-Test-Container-Matrix-v1",
                PostgresResidualObligationKind::ProofCommandMatrix,
                PostgresResidualDisposition::ImplementedInKernel002,
                Some("WP-KERNEL-002-CRDT-Workspace-Write-Box-Preuse-Hardening-v1"),
                StorageAuthorityMode::DerivedProjectionOnly,
                Some("cargo test -p handshake_core kernel_postgres_control_plane_residual"),
                None,
            ),
            obligation(
                "leases-backpressure",
                "WP-1-Postgres-Control-Plane-Leases-Backpressure-v1",
                PostgresResidualObligationKind::QueueLeaseBackpressure,
                PostgresResidualDisposition::MappedToKernel003,
                Some("WP-KERNEL-003-Postgres-Control-Plane-Activation-v1"),
                StorageAuthorityMode::PostgresRequired,
                Some("cargo test -p handshake_core leases_backpressure"),
                None,
            ),
            obligation(
                "modelsession-queue-workers",
                "WP-1-ModelSession-Postgres-Queue-Workers-v1",
                PostgresResidualObligationKind::ModelSessionQueue,
                PostgresResidualDisposition::MappedToKernel003,
                Some("WP-KERNEL-003-Postgres-Control-Plane-Activation-v1"),
                StorageAuthorityMode::PostgresRequired,
                Some("cargo test -p handshake_core modelsession_queue"),
                None,
            ),
            obligation(
                "fems-memory-store",
                "WP-1-FEMS-Postgres-Memory-Store-v1",
                PostgresResidualObligationKind::FemsMemoryStore,
                PostgresResidualDisposition::MappedToKernel003,
                Some("WP-KERNEL-003-Postgres-Control-Plane-Activation-v1"),
                StorageAuthorityMode::PostgresRequired,
                Some("cargo test -p handshake_core fems_memory_store"),
                None,
            ),
            obligation(
                "workflow-durable-execution",
                "WP-1-Workflow-Engine-Postgres-Durable-Execution-v1",
                PostgresResidualObligationKind::WorkflowDurableExecution,
                PostgresResidualDisposition::MappedToKernel003,
                Some("WP-KERNEL-003-Postgres-Control-Plane-Activation-v1"),
                StorageAuthorityMode::PostgresRequired,
                Some("cargo test -p handshake_core workflow_durable_execution"),
                None,
            ),
            obligation(
                "dcc-postgres-projections",
                "WP-1-DCC-Postgres-Control-Plane-Projections-v1",
                PostgresResidualObligationKind::DccProjection,
                PostgresResidualDisposition::MappedToKernel004,
                Some("WP-KERNEL-004-DCC-Control-Plane-Projections-v1"),
                StorageAuthorityMode::DerivedProjectionOnly,
                Some("cargo test -p handshake_core dcc_postgres_projection"),
                None,
            ),
            obligation(
                "legacy-cache-offline-boundary",
                LEGACY_CACHE_OFFLINE_BOUNDARY_STUB_ID,
                PostgresResidualObligationKind::LegacyCacheBoundary,
                PostgresResidualDisposition::ImplementedInKernel002,
                Some("WP-KERNEL-002-CRDT-Workspace-Write-Box-Preuse-Hardening-v1"),
                StorageAuthorityMode::LegacyCacheOnly,
                Some("cargo test -p handshake_core legacy_cache_boundary"),
                None,
            )
            .with_legacy_cache_roles(vec![
                LegacyCacheBoundaryRole::Cache,
                LegacyCacheBoundaryRole::OfflineReplica,
                LegacyCacheBoundaryRole::ReadOnlyProjection,
            ]),
            obligation(
                "cross-subsystem-integration-proof",
                "WP-1-Postgres-Control-Plane-Shift-Bundle-v1",
                PostgresResidualObligationKind::CrossSubsystemIntegration,
                PostgresResidualDisposition::MappedToKernel004,
                Some("WP-KERNEL-004-DCC-Control-Plane-Projections-v1"),
                StorageAuthorityMode::PostgresRequired,
                Some("cargo test -p handshake_core postgres_cross_subsystem"),
                None,
            ),
            obligation(
                "postgres-carry-forward-debt-ledger",
                "WP-1-Postgres-Control-Plane-Shift-Bundle-v1",
                PostgresResidualObligationKind::CarryForwardDebt,
                PostgresResidualDisposition::DebtLedger,
                Some("WP-KERNEL-003-Postgres-Control-Plane-Activation-v1"),
                StorageAuthorityMode::DerivedProjectionOnly,
                Some("just phase-check CLOSEOUT WP-KERNEL-002-CRDT-Workspace-Write-Box-Preuse-Hardening-v1"),
                None,
            ),
        ],
        excluded_stub_ids: vec![
            "WP-1-Loom-Storage-Portability-v4".to_string(),
            "WP-1-Loom-MVP-v1".to_string(),
            "WP-1-Video-Archive-Loom-Integration-v1".to_string(),
            "WP-1-Media-Downloader-Loom-Bridge-v1".to_string(),
            "WP-1-Loom-Preview-VideoPosterFrames-v1".to_string(),
        ],
        product_authority_refs: vec![
            "kernel.event_ledger".to_string(),
            "kernel.session_broker".to_string(),
            "kernel.write_box.promotion".to_string(),
        ],
        folded_source_refs: vec![
            ".GOV/task_packets/stubs/WP-1-Postgres-Control-Plane-Shift-Bundle-v1.contract.json".to_string(),
            ".GOV/task_packets/stubs/WP-1-Postgres-Control-Plane-Shift-Bundle-v1.md".to_string(),
        ],
    }
}

fn required_residual_kinds() -> Vec<PostgresResidualObligationKind> {
    vec![
        PostgresResidualObligationKind::LiveServiceProof,
        PostgresResidualObligationKind::QueueLeaseBackpressure,
        PostgresResidualObligationKind::ModelSessionQueue,
        PostgresResidualObligationKind::FemsMemoryStore,
        PostgresResidualObligationKind::WorkflowDurableExecution,
        PostgresResidualObligationKind::DccProjection,
        PostgresResidualObligationKind::LegacyCacheBoundary,
    ]
}

fn obligation(
    obligation_id: &str,
    source_stub_id: &str,
    kind: PostgresResidualObligationKind,
    disposition: PostgresResidualDisposition,
    target_kernel_wp_id: Option<&str>,
    storage_authority_mode: StorageAuthorityMode,
    proof_command: Option<&str>,
    environment_blocker: Option<&str>,
) -> PostgresResidualObligationV1 {
    PostgresResidualObligationV1 {
        obligation_id: obligation_id.to_string(),
        source_stub_id: source_stub_id.to_string(),
        kind,
        disposition,
        target_kernel_wp_id: target_kernel_wp_id.map(str::to_string),
        storage_authority_mode,
        proof_command: proof_command.map(str::to_string),
        environment_blocker: environment_blocker.map(str::to_string),
        legacy_cache_allowed_roles: Vec::new(),
        acceptance_refs: vec!["MT-022".to_string()],
        folded_source_refs: vec![
            ".GOV/task_packets/stubs/WP-1-Postgres-Control-Plane-Shift-Bundle-v1.contract.json"
                .to_string(),
        ],
    }
}
