use handshake_core::kernel::{
    action_catalog::{kernel002_action_catalog, validate_kernel_action_catalog},
    action_envelope::AuthorityEffect,
    role_mailbox_claim_lease::RoleMailboxExecutorKind,
    role_mailbox_handoff_bundle::{
        preview_role_mailbox_announce_back, project_role_mailbox_handoff_bundles,
        validate_role_mailbox_handoff_bundle_controls, AnnounceBackProvenanceKind,
        AnnounceBackProvenanceV1, HandoffBundleState, HandoffConfidenceLevel, HandoffRiskLevel,
        RecommendedNextActorV1, RoleMailboxHandoffBundleControlsV1, RoleMailboxHandoffBundleV1,
        TranscriptionStatus, TranscriptionTargetKind, TranscriptionTargetV1,
    },
};

#[test]
fn handoff_bundle_controls_validate_required_handoff_transcription_and_provenance_fields() {
    let controls = sample_controls();

    validate_role_mailbox_handoff_bundle_controls(&controls).expect("handoff controls validate");

    let bundle = &controls.bundles[0];
    assert_eq!(bundle.bundle_state, HandoffBundleState::HandoffReady);
    assert_eq!(
        bundle.recommended_next_actor.executor_kind,
        RoleMailboxExecutorKind::Validator
    );
    assert!(bundle.transcription_targets.iter().any(|target| target.kind
        == TranscriptionTargetKind::WorkPacketNote
        && target.status == TranscriptionStatus::Pending));
    assert!(bundle
        .announce_back_provenance
        .iter()
        .any(
            |provenance| provenance.kind == AnnounceBackProvenanceKind::CompletionNotice
                && provenance.completion_notice
                && !provenance.mutates_authority
        ));
}

#[test]
fn handoff_bundle_projects_compact_resume_state_without_authority_transfer() {
    let controls = sample_controls();
    let projections = project_role_mailbox_handoff_bundles(&controls).expect("handoff projects");

    assert_eq!(projections.len(), 1);
    assert_eq!(
        projections[0].recommended_next_actor_kind,
        RoleMailboxExecutorKind::Validator
    );
    assert!(projections[0].handoff_ready);
    assert!(projections[0].transcription_pending);
    assert_eq!(
        projections[0].latest_provenance_kind,
        AnnounceBackProvenanceKind::TranscriptionConfirmedOutcome
    );
    assert!(!projections[0].mutates_locus_authority);
    assert!(!projections[0].mutates_work_packet_authority);
    assert!(!projections[0].mutates_task_board_authority);
}

#[test]
fn announce_back_preview_distinguishes_advisory_from_completion_notice() {
    let controls = sample_controls();

    let advisory = preview_role_mailbox_announce_back(
        &controls,
        "handoff-bundle-mt031",
        AnnounceBackProvenanceKind::AdvisoryStatus,
    )
    .expect("advisory preview");
    assert!(advisory.advisory_only);
    assert!(!advisory.completion_notice);
    assert_eq!(advisory.authority_effect, AuthorityEffect::None);

    let completion = preview_role_mailbox_announce_back(
        &controls,
        "handoff-bundle-mt031",
        AnnounceBackProvenanceKind::CompletionNotice,
    )
    .expect("completion preview");
    assert!(!completion.advisory_only);
    assert!(completion.completion_notice);
    assert_eq!(completion.authority_effect, AuthorityEffect::None);
    assert!(!completion.mutates_authority);
}

#[test]
fn handoff_bundle_rejects_transcript_replay_and_announce_back_authority_leaks() {
    let mut controls = sample_controls();
    controls.work_packet_projection_authoritative = true;
    controls.dcc_projection_authoritative = true;
    controls.bundles[0].transcript_replay_required = true;
    controls.bundles[0].announce_back_authoritative_for_completion = true;
    controls.bundles[0].announce_back_provenance[0].mutates_authority = true;
    controls.bundles[0].transcription_targets[0].status = TranscriptionStatus::NotRequired;

    let errors = validate_role_mailbox_handoff_bundle_controls(&controls)
        .expect_err("unsafe handoff controls must fail");

    assert!(errors
        .iter()
        .any(|error| error.field == "work_packet_projection_authoritative"));
    assert!(errors
        .iter()
        .any(|error| error.field == "dcc_projection_authoritative"));
    assert!(errors
        .iter()
        .any(|error| error.field == "transcript_replay_required"));
    assert!(errors
        .iter()
        .any(|error| error.field == "announce_back_authoritative_for_completion"));
    assert!(errors
        .iter()
        .any(|error| error.field == "announce_back_provenance.mutates_authority"));
    assert!(errors
        .iter()
        .any(|error| error.field == "transcription_targets.status"));
}

#[test]
fn kernel_action_catalog_exposes_role_mailbox_handoff_bundle_projection_action() {
    let catalog = kernel002_action_catalog();
    validate_kernel_action_catalog(&catalog).expect("catalog must validate");

    let action = catalog
        .action("kernel.role_mailbox_handoff_bundle.project")
        .expect("Role Mailbox handoff bundle projection action must be cataloged");

    assert_eq!(action.authority_effect, AuthorityEffect::ProjectionOnly);
    assert!(action
        .validation_hooks
        .iter()
        .any(|hook| hook.hook_id == "mailbox_handoff_bundle_state"));
}

fn sample_controls() -> RoleMailboxHandoffBundleControlsV1 {
    RoleMailboxHandoffBundleControlsV1 {
        schema_id: "hsk.kernel.role_mailbox_handoff_bundle_controls@1".to_string(),
        controls_id: "kernel002-role-mailbox-handoff-bundle-mt031".to_string(),
        folded_stub_id: "WP-1-Role-Mailbox-Handoff-Bundle-Transcription-Announce-Back-v1"
            .to_string(),
        bundles: vec![RoleMailboxHandoffBundleV1 {
            bundle_id: "handoff-bundle-mt031".to_string(),
            thread_id: "role-mailbox-thread-mt031".to_string(),
            source_message_id: "message-mt031".to_string(),
            linked_work_packet_id: "WP-KERNEL-002-CRDT-Workspace-Write-Box-Preuse-Hardening-v1"
                .to_string(),
            linked_micro_task_id: "MT-031".to_string(),
            locus_join_refs: vec!["locus://MT-031".to_string()],
            bundle_state: HandoffBundleState::HandoffReady,
            remaining_work: "Validator should review typed handoff projection evidence."
                .to_string(),
            unresolved_blockers: vec!["product cargo blocked by libduckdb-sys".to_string()],
            changed_scope: "No scope change; handoff preserves MT-031 contract.".to_string(),
            evidence_refs: vec!["evidence://handoff-mt031".to_string()],
            recommended_next_actor: RecommendedNextActorV1 {
                actor_id: "validator-session-next".to_string(),
                executor_kind: RoleMailboxExecutorKind::Validator,
                reason: "handoff bundle is ready for validation".to_string(),
            },
            risk: HandoffRiskLevel::High,
            confidence: HandoffConfidenceLevel::Medium,
            transcription_targets: sample_transcription_targets(),
            announce_back_provenance: sample_provenance(),
            compact_summary: "MT-031 handoff ready with pending WP note transcription.".to_string(),
            handoff_ready: true,
            transcript_replay_required: false,
            announce_back_authoritative_for_completion: false,
        }],
        compact_summary_first: true,
        locus_projection_authoritative: false,
        task_board_projection_authoritative: false,
        work_packet_projection_authoritative: false,
        micro_task_projection_authoritative: false,
        dcc_projection_authoritative: false,
        product_authority_refs: vec![
            "kernel.role_mailbox_contract".to_string(),
            "kernel.role_mailbox_loop_control".to_string(),
            "kernel.role_mailbox_claim_lease".to_string(),
            "kernel.workflow_transition_registry".to_string(),
            "kernel.locus_work_tracking".to_string(),
            "kernel.dcc_layout_projection_registry".to_string(),
        ],
        folded_source_refs: vec![
            ".GOV/task_packets/stubs/WP-1-Role-Mailbox-Handoff-Bundle-Transcription-Announce-Back-v1.contract.json"
                .to_string(),
            ".GOV/task_packets/stubs/WP-1-Role-Mailbox-Handoff-Bundle-Transcription-Announce-Back-v1.md"
                .to_string(),
        ],
    }
}

fn sample_transcription_targets() -> Vec<TranscriptionTargetV1> {
    vec![
        target(
            "target-wp-note",
            TranscriptionTargetKind::WorkPacketNote,
            TranscriptionStatus::Pending,
            true,
        ),
        target(
            "target-locus",
            TranscriptionTargetKind::LocusJoin,
            TranscriptionStatus::Confirmed,
            true,
        ),
        target(
            "target-mt",
            TranscriptionTargetKind::MicroTaskCheckpoint,
            TranscriptionStatus::Confirmed,
            true,
        ),
    ]
}

fn target(
    target_id: &str,
    kind: TranscriptionTargetKind,
    status: TranscriptionStatus,
    required: bool,
) -> TranscriptionTargetV1 {
    TranscriptionTargetV1 {
        target_id: target_id.to_string(),
        kind,
        target_ref: format!("ref://{target_id}"),
        status,
        required,
    }
}

fn sample_provenance() -> Vec<AnnounceBackProvenanceV1> {
    vec![
        provenance(
            AnnounceBackProvenanceKind::AdvisoryStatus,
            true,
            false,
            false,
        ),
        provenance(
            AnnounceBackProvenanceKind::CompletionNotice,
            false,
            true,
            false,
        ),
        provenance(
            AnnounceBackProvenanceKind::EscalationSummary,
            true,
            false,
            false,
        ),
        provenance(
            AnnounceBackProvenanceKind::ScopeChangeNotice,
            true,
            false,
            false,
        ),
        provenance(AnnounceBackProvenanceKind::HandoffReady, true, false, false),
        provenance(
            AnnounceBackProvenanceKind::TranscriptionConfirmedOutcome,
            false,
            true,
            true,
        ),
    ]
}

fn provenance(
    kind: AnnounceBackProvenanceKind,
    advisory_only: bool,
    completion_notice: bool,
    transcription_confirmed: bool,
) -> AnnounceBackProvenanceV1 {
    AnnounceBackProvenanceV1 {
        provenance_id: format!("provenance-{kind:?}").to_lowercase(),
        kind,
        source_thread_id: "role-mailbox-thread-mt031".to_string(),
        source_message_id: "message-mt031".to_string(),
        evidence_refs: vec!["evidence://announce-back-mt031".to_string()],
        advisory_only,
        completion_notice,
        transcription_confirmed,
        mutates_authority: false,
    }
}
