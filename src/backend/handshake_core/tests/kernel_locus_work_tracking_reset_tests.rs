use handshake_core::kernel::{
    action_catalog::{kernel002_action_catalog, validate_kernel_action_catalog},
    action_envelope::AuthorityEffect,
    locus_work_tracking_reset::{
        project_locus_task_board_rows, query_locus_ready_micro_tasks,
        validate_locus_flight_recorder_event_type, validate_locus_work_tracking_reset_contract,
        LocusAuthorityMode, LocusMicroTaskRecordV1, LocusMicroTaskStatus,
        LocusTrackingCapabilityKind, LocusTrackingCapabilityV1, LocusWorkPacketRecordV1,
        LocusWorkTrackingResetContractV1,
    },
};

#[test]
fn locus_reset_preserves_tracking_surfaces_without_sqlite_authority() {
    let contract = sample_contract();

    validate_locus_work_tracking_reset_contract(&contract)
        .expect("Locus reset contract should validate");

    assert!(!contract.legacy_local_authority_allowed);
    for required_kind in required_capability_kinds() {
        assert!(
            contract
                .capabilities
                .iter()
                .any(|capability| capability.kind == required_kind),
            "missing Locus capability: {required_kind:?}"
        );
    }
    assert!(contract
        .folded_source_refs
        .iter()
        .any(|source| source.contains("WP-1-Locus-Work-Tracking-System-Phase1-v1")));
    assert!(contract
        .product_authority_refs
        .contains(&"kernel.event_ledger".to_string()));
}

#[test]
fn locus_ready_query_respects_dependencies_and_task_board_projection_keeps_occupancy() {
    let contract = sample_contract();

    let ready = query_locus_ready_micro_tasks(&contract, "WP-KERNEL-002")
        .expect("ready query should validate and project");
    let ready_ids: Vec<_> = ready.iter().map(|mt| mt.mt_id.as_str()).collect();
    assert_eq!(ready_ids, vec!["MT-002"]);

    let rows = project_locus_task_board_rows(&contract, "WP-KERNEL-002")
        .expect("Task Board rows should project");
    let running_row = rows
        .iter()
        .find(|row| row.mt_id == "MT-004")
        .expect("running MT row should project");

    assert_eq!(
        running_row.active_session_ids,
        vec!["model-session-7".to_string()]
    );
    assert_eq!(running_row.authority_projection, "projection-only");
}

#[test]
fn locus_reset_rejects_sqlite_authority_and_unknown_flight_recorder_events() {
    let mut contract = sample_contract();
    contract.capabilities[0].authority_mode = LocusAuthorityMode::LegacyLocalStoreAuthority;

    let errors = validate_locus_work_tracking_reset_contract(&contract)
        .expect_err("legacy local authority must be rejected");

    assert!(
        errors.iter().any(|error| error.field == "authority_mode"
            && error.message.contains("legacy local authority")),
        "expected legacy local authority denial, got {errors:?}"
    );

    let event_error = validate_locus_flight_recorder_event_type("locus.unknown")
        .expect_err("unknown Locus Flight Recorder event must fail fast");
    assert_eq!(event_error.field, "flight_recorder_event_type");
}

#[test]
fn kernel_action_catalog_exposes_locus_work_tracking_projection_action() {
    let catalog = kernel002_action_catalog();
    validate_kernel_action_catalog(&catalog).expect("catalog must validate");

    let action = catalog
        .action("kernel.locus_work_tracking.project")
        .expect("Locus work tracking projection action must be cataloged");

    assert_eq!(action.authority_effect, AuthorityEffect::ProjectionOnly);
    assert!(action
        .validation_hooks
        .iter()
        .any(|hook| hook.hook_id == "locus_legacy_authority_removed"));
}

fn sample_contract() -> LocusWorkTrackingResetContractV1 {
    LocusWorkTrackingResetContractV1 {
        schema_id: "hsk.kernel.locus_work_tracking_reset@1".to_string(),
        contract_id: "kernel002-locus-work-tracking-reset-mt023".to_string(),
        folded_stub_id: "WP-1-Locus-Work-Tracking-System-Phase1-v1".to_string(),
        legacy_local_authority_allowed: false,
        capabilities: vec![
            capability(
                "locus-wp-tracking",
                LocusTrackingCapabilityKind::WorkPacketTracking,
                LocusAuthorityMode::PostgresAuthority,
                &[
                    "locus_create_wp",
                    "locus_update_wp",
                    "locus_gate_wp",
                    "locus_close_wp",
                ],
                &["locus.wp.created", "locus.wp.updated"],
            ),
            capability(
                "locus-mt-tracking",
                LocusTrackingCapabilityKind::MicroTaskTracking,
                LocusAuthorityMode::CrdtWorkspaceAuthority,
                &[
                    "locus_register_mt",
                    "locus_start_mt",
                    "locus_record_iteration",
                    "locus_complete_mt",
                ],
                &[
                    "locus.mt.registered",
                    "locus.mt.started",
                    "locus.mt.iteration_recorded",
                    "locus.mt.completed",
                ],
            ),
            capability(
                "locus-dependencies",
                LocusTrackingCapabilityKind::DependencyGraph,
                LocusAuthorityMode::EventLedgerAuthority,
                &["locus_add_dependency", "locus_remove_dependency"],
                &["locus.dep.added", "locus.dep.removed"],
            ),
            capability(
                "locus-occupancy",
                LocusTrackingCapabilityKind::Occupancy,
                LocusAuthorityMode::CrdtWorkspaceAuthority,
                &["locus_bind_session", "locus_unbind_session"],
                &["locus.occupancy.bound", "locus.occupancy.unbound"],
            ),
            capability(
                "locus-ready-query",
                LocusTrackingCapabilityKind::ReadyQuery,
                LocusAuthorityMode::PostgresAuthority,
                &[
                    "locus_query_ready",
                    "locus_get_status",
                    "locus_get_progress",
                ],
                &["locus.query.ready"],
            ),
            capability(
                "locus-task-board",
                LocusTrackingCapabilityKind::TaskBoardProjection,
                LocusAuthorityMode::ProjectionOnly,
                &["locus_sync_task_board"],
                &["locus.task_board.synced"],
            ),
            capability(
                "locus-flight-recorder",
                LocusTrackingCapabilityKind::FlightRecorder,
                LocusAuthorityMode::EventLedgerAuthority,
                &["flight_recorder_append"],
                &["locus.flight_recorder.appended"],
            ),
        ],
        work_packets: vec![LocusWorkPacketRecordV1 {
            wp_id: "WP-KERNEL-002".to_string(),
            title: "Kernel002".to_string(),
            status: "IN_PROGRESS".to_string(),
            micro_task_ids: vec![
                "MT-001".to_string(),
                "MT-002".to_string(),
                "MT-003".to_string(),
                "MT-004".to_string(),
            ],
            authority_ref: "kernel.event_ledger/wp/WP-KERNEL-002".to_string(),
        }],
        micro_tasks: vec![
            micro_task(
                "MT-001",
                LocusMicroTaskStatus::Complete,
                &[],
                &[],
                &["iter-1"],
            ),
            micro_task(
                "MT-002",
                LocusMicroTaskStatus::Pending,
                &["MT-001"],
                &[],
                &[],
            ),
            micro_task(
                "MT-003",
                LocusMicroTaskStatus::Pending,
                &["MT-002"],
                &[],
                &[],
            ),
            micro_task(
                "MT-004",
                LocusMicroTaskStatus::Running,
                &[],
                &["model-session-7"],
                &["iter-4"],
            ),
        ],
        task_board_projection_id: "locus-task-board-kernel002".to_string(),
        flight_recorder_event_types: vec![
            "locus.wp.created".to_string(),
            "locus.mt.started".to_string(),
            "locus.mt.iteration_recorded".to_string(),
            "locus.dep.added".to_string(),
            "locus.task_board.synced".to_string(),
            "locus.query.ready".to_string(),
        ],
        product_authority_refs: vec![
            "kernel.postgres_control_plane".to_string(),
            "kernel.event_ledger".to_string(),
            "kernel.crdt_workspace".to_string(),
        ],
        folded_source_refs: vec![
            ".GOV/task_packets/stubs/WP-1-Locus-Work-Tracking-System-Phase1-v1.contract.json"
                .to_string(),
            ".GOV/task_packets/stubs/WP-1-Locus-Work-Tracking-System-Phase1-v1.md".to_string(),
        ],
    }
}

fn required_capability_kinds() -> Vec<LocusTrackingCapabilityKind> {
    vec![
        LocusTrackingCapabilityKind::WorkPacketTracking,
        LocusTrackingCapabilityKind::MicroTaskTracking,
        LocusTrackingCapabilityKind::DependencyGraph,
        LocusTrackingCapabilityKind::Occupancy,
        LocusTrackingCapabilityKind::ReadyQuery,
        LocusTrackingCapabilityKind::TaskBoardProjection,
        LocusTrackingCapabilityKind::FlightRecorder,
    ]
}

fn capability(
    capability_id: &str,
    kind: LocusTrackingCapabilityKind,
    authority_mode: LocusAuthorityMode,
    operations: &[&str],
    events: &[&str],
) -> LocusTrackingCapabilityV1 {
    LocusTrackingCapabilityV1 {
        capability_id: capability_id.to_string(),
        kind,
        authority_mode,
        operation_ids: operations
            .iter()
            .map(|value| (*value).to_string())
            .collect(),
        flight_recorder_event_types: events.iter().map(|value| (*value).to_string()).collect(),
        authority_refs: vec![
            "kernel.postgres_control_plane".to_string(),
            "kernel.event_ledger".to_string(),
            "kernel.crdt_workspace".to_string(),
        ],
        folded_source_refs: vec![
            ".GOV/task_packets/stubs/WP-1-Locus-Work-Tracking-System-Phase1-v1.contract.json"
                .to_string(),
        ],
    }
}

fn micro_task(
    mt_id: &str,
    status: LocusMicroTaskStatus,
    depends_on: &[&str],
    active_session_ids: &[&str],
    iteration_ids: &[&str],
) -> LocusMicroTaskRecordV1 {
    LocusMicroTaskRecordV1 {
        mt_id: mt_id.to_string(),
        wp_id: "WP-KERNEL-002".to_string(),
        status,
        depends_on: depends_on
            .iter()
            .map(|value| (*value).to_string())
            .collect(),
        active_session_ids: active_session_ids
            .iter()
            .map(|value| (*value).to_string())
            .collect(),
        iteration_ids: iteration_ids
            .iter()
            .map(|value| (*value).to_string())
            .collect(),
        authority_ref: format!("kernel.crdt_workspace/mt/{mt_id}"),
    }
}
