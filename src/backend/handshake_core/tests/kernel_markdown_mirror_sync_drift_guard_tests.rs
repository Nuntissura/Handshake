use handshake_core::kernel::{
    action_catalog::{kernel002_action_catalog, validate_kernel_action_catalog},
    action_envelope::AuthorityEffect,
    markdown_mirror_sync_drift_guard::{
        project_markdown_mirror_sync_drift_guard, validate_markdown_mirror_sync_drift_guard,
        MarkdownMirrorContractV1, MarkdownMirrorDriftRecordV1, MarkdownMirrorDriftSource,
        MarkdownMirrorProjectionBannerV1, MarkdownMirrorReconciliationActionKind,
        MarkdownMirrorReconciliationActionV1, MarkdownMirrorSyncDriftGuardV1, MirrorAuthorityMode,
        MirrorDriftState, MirrorSurfaceKind,
    },
};

#[test]
fn kernel_markdown_mirror_guard_projects_drift_states_and_queue() {
    let guard = sample_guard();
    validate_markdown_mirror_sync_drift_guard(&guard).expect("guard validates");

    let projection = project_markdown_mirror_sync_drift_guard(&guard).expect("projection builds");

    assert!(projection
        .stale_contract_ids
        .contains(&"mirror.taskboard".to_string()));
    assert!(projection
        .advisory_contract_ids
        .contains(&"mirror.workpacket".to_string()));
    assert!(projection
        .manual_resolution_contract_ids
        .contains(&"mirror.rolemailbox".to_string()));
    assert_eq!(projection.dcc_queue_item_ids.len(), 3);
    assert!(!projection.mirror_is_authority);
    assert!(!projection.mutates_authority);
}

#[test]
fn kernel_markdown_mirror_guard_preserves_projection_banners() {
    let projection =
        project_markdown_mirror_sync_drift_guard(&sample_guard()).expect("projection builds");

    assert!(projection
        .banner_ids
        .contains(&"banner.workpacket.advisory".to_string()));
    assert!(projection
        .allowed_action_ids
        .contains(&"kernel.mirror_advisory.capture".to_string()));
    assert!(projection
        .allowed_action_ids
        .contains(&"kernel.mirror_advisory.normalize".to_string()));
}

#[test]
fn kernel_markdown_mirror_guard_rejects_unsafe_mirror_authority_or_missing_advisory() {
    let mut guard = sample_guard();
    guard.mirror_contracts[0].deterministic_regeneration = false;
    guard.drift_records[1].advisory_ref = None;
    guard.reconciliation_actions[1].authority_mutation = true;
    guard.projection_banners[1].generated_from_canonical = false;

    let errors =
        validate_markdown_mirror_sync_drift_guard(&guard).expect_err("unsafe guard must fail");

    assert!(errors
        .iter()
        .any(|error| error.field == "mirror_contracts.deterministic_regeneration"));
    assert!(errors
        .iter()
        .any(|error| error.field == "drift_records.advisory_ref"));
    assert!(errors
        .iter()
        .any(|error| error.field == "reconciliation_actions.authority_mutation"));
    assert!(errors
        .iter()
        .any(|error| error.field == "projection_banners.generated_from_canonical"));
}

#[test]
fn kernel_markdown_mirror_guard_catalogs_projection_action() {
    let catalog = kernel002_action_catalog();
    validate_kernel_action_catalog(&catalog).expect("catalog validates");

    let action = catalog
        .action("kernel.markdown_mirror_sync_drift_guard.project")
        .expect("markdown mirror guard projection action must be cataloged");

    assert_eq!(action.authority_effect, AuthorityEffect::ProjectionOnly);
    assert!(action
        .validation_hooks
        .iter()
        .any(|hook| hook.hook_id == "markdown_mirror_dcc_queue"));
}

fn sample_guard() -> MarkdownMirrorSyncDriftGuardV1 {
    MarkdownMirrorSyncDriftGuardV1 {
        schema_id: "hsk.kernel.markdown_mirror_sync_drift_guard@1".to_string(),
        guard_id: "markdown-mirror-mt047".to_string(),
        folded_stub_ids: vec!["WP-1-Markdown-Mirror-Sync-Drift-Guard-v1".to_string()],
        mirror_contracts: vec![
            contract("mirror.taskboard", MirrorSurfaceKind::TaskBoard),
            contract("mirror.workpacket", MirrorSurfaceKind::WorkPacket),
            contract("mirror.rolemailbox", MirrorSurfaceKind::RoleMailbox),
        ],
        drift_records: vec![
            drift(
                "drift.taskboard.stale",
                "mirror.taskboard",
                MirrorDriftState::Stale,
                MarkdownMirrorDriftSource::CanonicalFieldChange,
                None,
                false,
            ),
            drift(
                "drift.workpacket.advisory",
                "mirror.workpacket",
                MirrorDriftState::AdvisoryEdit,
                MarkdownMirrorDriftSource::AdvisoryHumanEdit,
                Some("advisory://mirror/workpacket/1"),
                false,
            ),
            drift(
                "drift.rolemailbox.manual",
                "mirror.rolemailbox",
                MirrorDriftState::ManualResolutionRequired,
                MarkdownMirrorDriftSource::TemplateMismatch,
                Some("advisory://mirror/rolemailbox/1"),
                true,
            ),
        ],
        reconciliation_actions: vec![
            action(
                "action.taskboard.regenerate",
                "mirror.taskboard",
                MarkdownMirrorReconciliationActionKind::RegenerateMirror,
                "kernel.markdown_mirror_sync_drift_guard.project",
                "projection-regenerator://markdown-mirror/taskboard",
            ),
            action(
                "action.workpacket.capture_advisory",
                "mirror.workpacket",
                MarkdownMirrorReconciliationActionKind::CaptureAdvisoryEdit,
                "kernel.mirror_advisory.capture",
                "write-box://mirror-advisory/workpacket",
            ),
            action(
                "action.rolemailbox.normalize",
                "mirror.rolemailbox",
                MarkdownMirrorReconciliationActionKind::NormalizeAdvisoryEdit,
                "kernel.mirror_advisory.normalize",
                "write-box://mirror-advisory/rolemailbox",
            ),
        ],
        projection_banners: vec![
            banner(
                "banner.taskboard.stale",
                "mirror.taskboard",
                MirrorDriftState::Stale,
            ),
            banner(
                "banner.workpacket.advisory",
                "mirror.workpacket",
                MirrorDriftState::AdvisoryEdit,
            ),
            banner(
                "banner.rolemailbox.manual",
                "mirror.rolemailbox",
                MirrorDriftState::ManualResolutionRequired,
            ),
        ],
        dcc_queue_ref: "dcc://mirror-queue/kernel002".to_string(),
        product_authority_refs: vec![
            "kernel.mirror_advisory".to_string(),
            "kernel.action_catalog".to_string(),
            "dcc.mirror_advisory_queue".to_string(),
            "locus.sync_controller".to_string(),
            "projection_banners".to_string(),
        ],
        folded_source_refs: vec![
            ".GOV/task_packets/stubs/WP-1-Markdown-Mirror-Sync-Drift-Guard-v1.contract.json"
                .to_string(),
        ],
    }
}

fn contract(contract_id: &str, surface_kind: MirrorSurfaceKind) -> MarkdownMirrorContractV1 {
    MarkdownMirrorContractV1 {
        contract_id: contract_id.to_string(),
        surface_kind,
        canonical_record_ref: format!("canonical://{contract_id}"),
        mirror_path: format!("generated/{contract_id}.md"),
        mirror_hash_ref: format!("sha256://mirror/{contract_id}"),
        generation_template_ref: format!("template://markdown-mirror/{contract_id}"),
        authority_mode: MirrorAuthorityMode::CanonicalRecordAuthority,
        deterministic_regeneration: true,
        append_only_note_sidecar_ref: format!("notes-sidecar://{contract_id}"),
    }
}

fn drift(
    drift_id: &str,
    contract_id: &str,
    state: MirrorDriftState,
    source: MarkdownMirrorDriftSource,
    advisory_ref: Option<&str>,
    manual_resolution_required: bool,
) -> MarkdownMirrorDriftRecordV1 {
    MarkdownMirrorDriftRecordV1 {
        drift_id: drift_id.to_string(),
        contract_id: contract_id.to_string(),
        state,
        source,
        canonical_hash_ref: format!("sha256://canonical/{contract_id}"),
        mirror_hash_ref: format!("sha256://mirror/{contract_id}"),
        diff_evidence_ref: format!("evidence://mirror-drift/{drift_id}"),
        advisory_ref: advisory_ref.map(str::to_string),
        manual_resolution_required,
    }
}

fn action(
    action_id: &str,
    contract_id: &str,
    kind: MarkdownMirrorReconciliationActionKind,
    action_catalog_id: &str,
    write_box_ref: &str,
) -> MarkdownMirrorReconciliationActionV1 {
    MarkdownMirrorReconciliationActionV1 {
        action_id: action_id.to_string(),
        contract_id: contract_id.to_string(),
        kind,
        action_catalog_id: action_catalog_id.to_string(),
        write_box_ref: write_box_ref.to_string(),
        evidence_ref: format!("evidence://mirror-drift/{action_id}"),
        authority_mutation: false,
        requires_operator_approval: false,
    }
}

fn banner(
    banner_id: &str,
    contract_id: &str,
    visible_state: MirrorDriftState,
) -> MarkdownMirrorProjectionBannerV1 {
    MarkdownMirrorProjectionBannerV1 {
        banner_id: banner_id.to_string(),
        contract_id: contract_id.to_string(),
        visible_state,
        banner_text: "Derived from canonical structured record".to_string(),
        generated_from_canonical: true,
        stale_visible: visible_state == MirrorDriftState::Stale,
        advisory_visible: matches!(
            visible_state,
            MirrorDriftState::AdvisoryEdit | MirrorDriftState::ManualResolutionRequired
        ),
    }
}
