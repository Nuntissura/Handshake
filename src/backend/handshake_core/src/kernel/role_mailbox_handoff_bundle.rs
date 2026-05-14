use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use super::{action_envelope::AuthorityEffect, role_mailbox_claim_lease::RoleMailboxExecutorKind};

pub const FOLDED_ROLE_MAILBOX_HANDOFF_BUNDLE_STUB_ID: &str =
    "WP-1-Role-Mailbox-Handoff-Bundle-Transcription-Announce-Back-v1";

const REQUIRED_PROVENANCE_KINDS: [AnnounceBackProvenanceKind; 6] = [
    AnnounceBackProvenanceKind::AdvisoryStatus,
    AnnounceBackProvenanceKind::CompletionNotice,
    AnnounceBackProvenanceKind::EscalationSummary,
    AnnounceBackProvenanceKind::ScopeChangeNotice,
    AnnounceBackProvenanceKind::HandoffReady,
    AnnounceBackProvenanceKind::TranscriptionConfirmedOutcome,
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum HandoffBundleState {
    Draft,
    HandoffReady,
    TranscriptionPending,
    TranscriptionConfirmed,
    Stale,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TranscriptionTargetKind {
    WorkPacketNote,
    LocusJoin,
    MicroTaskCheckpoint,
    TaskBoardOverlay,
    DccProjection,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TranscriptionStatus {
    Pending,
    Confirmed,
    Failed,
    NotRequired,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AnnounceBackProvenanceKind {
    AdvisoryStatus,
    CompletionNotice,
    EscalationSummary,
    ScopeChangeNotice,
    HandoffReady,
    TranscriptionConfirmedOutcome,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum HandoffRiskLevel {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum HandoffConfidenceLevel {
    Low,
    Medium,
    High,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecommendedNextActorV1 {
    pub actor_id: String,
    pub executor_kind: RoleMailboxExecutorKind,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TranscriptionTargetV1 {
    pub target_id: String,
    pub kind: TranscriptionTargetKind,
    pub target_ref: String,
    pub status: TranscriptionStatus,
    pub required: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AnnounceBackProvenanceV1 {
    pub provenance_id: String,
    pub kind: AnnounceBackProvenanceKind,
    pub source_thread_id: String,
    pub source_message_id: String,
    pub evidence_refs: Vec<String>,
    pub advisory_only: bool,
    pub completion_notice: bool,
    pub transcription_confirmed: bool,
    pub mutates_authority: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RoleMailboxHandoffBundleV1 {
    pub bundle_id: String,
    pub thread_id: String,
    pub source_message_id: String,
    pub linked_work_packet_id: String,
    pub linked_micro_task_id: String,
    pub locus_join_refs: Vec<String>,
    pub bundle_state: HandoffBundleState,
    pub remaining_work: String,
    pub unresolved_blockers: Vec<String>,
    pub changed_scope: String,
    pub evidence_refs: Vec<String>,
    pub recommended_next_actor: RecommendedNextActorV1,
    pub risk: HandoffRiskLevel,
    pub confidence: HandoffConfidenceLevel,
    pub transcription_targets: Vec<TranscriptionTargetV1>,
    pub announce_back_provenance: Vec<AnnounceBackProvenanceV1>,
    pub compact_summary: String,
    pub handoff_ready: bool,
    pub transcript_replay_required: bool,
    pub announce_back_authoritative_for_completion: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RoleMailboxHandoffBundleControlsV1 {
    pub schema_id: String,
    pub controls_id: String,
    pub folded_stub_id: String,
    pub bundles: Vec<RoleMailboxHandoffBundleV1>,
    pub compact_summary_first: bool,
    pub locus_projection_authoritative: bool,
    pub task_board_projection_authoritative: bool,
    pub work_packet_projection_authoritative: bool,
    pub micro_task_projection_authoritative: bool,
    pub dcc_projection_authoritative: bool,
    pub product_authority_refs: Vec<String>,
    pub folded_source_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoleMailboxHandoffBundleProjectionV1 {
    pub schema_id: String,
    pub bundle_id: String,
    pub thread_id: String,
    pub linked_work_packet_id: String,
    pub linked_micro_task_id: String,
    pub recommended_next_actor_kind: RoleMailboxExecutorKind,
    pub bundle_state: HandoffBundleState,
    pub handoff_ready: bool,
    pub transcription_pending: bool,
    pub latest_provenance_kind: AnnounceBackProvenanceKind,
    pub compact_summary: String,
    pub locus_join_refs: Vec<String>,
    pub mutates_locus_authority: bool,
    pub mutates_task_board_authority: bool,
    pub mutates_work_packet_authority: bool,
    pub mutates_micro_task_authority: bool,
    pub mutates_dcc_authority: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoleMailboxAnnounceBackPreviewV1 {
    pub schema_id: String,
    pub bundle_id: String,
    pub provenance_id: String,
    pub kind: AnnounceBackProvenanceKind,
    pub source_thread_id: String,
    pub source_message_id: String,
    pub evidence_refs: Vec<String>,
    pub advisory_only: bool,
    pub completion_notice: bool,
    pub transcription_confirmed: bool,
    pub authority_effect: AuthorityEffect,
    pub mutates_authority: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoleMailboxHandoffBundleValidationError {
    pub field: &'static str,
    pub message: &'static str,
}

pub fn validate_role_mailbox_handoff_bundle_controls(
    controls: &RoleMailboxHandoffBundleControlsV1,
) -> Result<(), Vec<RoleMailboxHandoffBundleValidationError>> {
    let mut errors = Vec::new();

    require_non_empty(&mut errors, "schema_id", &controls.schema_id);
    require_non_empty(&mut errors, "controls_id", &controls.controls_id);
    require_non_empty(&mut errors, "folded_stub_id", &controls.folded_stub_id);
    require_vec(&mut errors, "bundles", &controls.bundles);
    require_vec(
        &mut errors,
        "product_authority_refs",
        &controls.product_authority_refs,
    );
    require_vec(
        &mut errors,
        "folded_source_refs",
        &controls.folded_source_refs,
    );

    if controls.folded_stub_id != FOLDED_ROLE_MAILBOX_HANDOFF_BUNDLE_STUB_ID {
        errors.push(RoleMailboxHandoffBundleValidationError {
            field: "folded_stub_id",
            message: "handoff bundle controls must bind the folded Role Mailbox handoff stub",
        });
    }
    if !contains_text(
        &controls.folded_source_refs,
        FOLDED_ROLE_MAILBOX_HANDOFF_BUNDLE_STUB_ID,
    ) {
        errors.push(RoleMailboxHandoffBundleValidationError {
            field: "folded_source_refs",
            message: "folded Role Mailbox handoff source must be preserved",
        });
    }
    if !controls.compact_summary_first {
        errors.push(RoleMailboxHandoffBundleValidationError {
            field: "compact_summary_first",
            message: "handoff bundle reads must be compact-summary-first",
        });
    }
    if controls.locus_projection_authoritative {
        errors.push(RoleMailboxHandoffBundleValidationError {
            field: "locus_projection_authoritative",
            message: "handoff bundle projection must not become Locus authority",
        });
    }
    if controls.task_board_projection_authoritative {
        errors.push(RoleMailboxHandoffBundleValidationError {
            field: "task_board_projection_authoritative",
            message: "handoff bundle projection must not become Task Board authority",
        });
    }
    if controls.work_packet_projection_authoritative {
        errors.push(RoleMailboxHandoffBundleValidationError {
            field: "work_packet_projection_authoritative",
            message: "handoff bundle projection must not become Work Packet authority",
        });
    }
    if controls.micro_task_projection_authoritative {
        errors.push(RoleMailboxHandoffBundleValidationError {
            field: "micro_task_projection_authoritative",
            message: "handoff bundle projection must not become Micro-Task authority",
        });
    }
    if controls.dcc_projection_authoritative {
        errors.push(RoleMailboxHandoffBundleValidationError {
            field: "dcc_projection_authoritative",
            message: "handoff bundle projection must not become DCC authority",
        });
    }

    validate_refs(&mut errors, controls);
    validate_bundles(&mut errors, &controls.bundles);

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

pub fn project_role_mailbox_handoff_bundles(
    controls: &RoleMailboxHandoffBundleControlsV1,
) -> Result<Vec<RoleMailboxHandoffBundleProjectionV1>, Vec<RoleMailboxHandoffBundleValidationError>>
{
    validate_role_mailbox_handoff_bundle_controls(controls)?;

    Ok(controls
        .bundles
        .iter()
        .map(|bundle| RoleMailboxHandoffBundleProjectionV1 {
            schema_id: "hsk.kernel.role_mailbox_handoff_bundle_projection@1".to_string(),
            bundle_id: bundle.bundle_id.clone(),
            thread_id: bundle.thread_id.clone(),
            linked_work_packet_id: bundle.linked_work_packet_id.clone(),
            linked_micro_task_id: bundle.linked_micro_task_id.clone(),
            recommended_next_actor_kind: bundle.recommended_next_actor.executor_kind,
            bundle_state: bundle.bundle_state,
            handoff_ready: bundle.handoff_ready,
            transcription_pending: bundle
                .transcription_targets
                .iter()
                .any(|target| target.status == TranscriptionStatus::Pending),
            latest_provenance_kind: bundle
                .announce_back_provenance
                .last()
                .map(|provenance| provenance.kind)
                .unwrap_or(AnnounceBackProvenanceKind::AdvisoryStatus),
            compact_summary: bundle.compact_summary.clone(),
            locus_join_refs: bundle.locus_join_refs.clone(),
            mutates_locus_authority: false,
            mutates_task_board_authority: false,
            mutates_work_packet_authority: false,
            mutates_micro_task_authority: false,
            mutates_dcc_authority: false,
        })
        .collect())
}

pub fn preview_role_mailbox_announce_back(
    controls: &RoleMailboxHandoffBundleControlsV1,
    bundle_id: &str,
    kind: AnnounceBackProvenanceKind,
) -> Result<RoleMailboxAnnounceBackPreviewV1, Vec<RoleMailboxHandoffBundleValidationError>> {
    validate_role_mailbox_handoff_bundle_controls(controls)?;

    let Some(bundle) = controls
        .bundles
        .iter()
        .find(|bundle| bundle.bundle_id == bundle_id)
    else {
        return Err(vec![RoleMailboxHandoffBundleValidationError {
            field: "bundle_id",
            message: "requested Role Mailbox handoff bundle is not registered",
        }]);
    };
    let Some(provenance) = bundle
        .announce_back_provenance
        .iter()
        .find(|provenance| provenance.kind == kind)
    else {
        return Err(vec![RoleMailboxHandoffBundleValidationError {
            field: "announce_back_provenance.kind",
            message: "requested announce-back provenance kind is not registered",
        }]);
    };

    Ok(RoleMailboxAnnounceBackPreviewV1 {
        schema_id: "hsk.kernel.role_mailbox_announce_back_preview@1".to_string(),
        bundle_id: bundle.bundle_id.clone(),
        provenance_id: provenance.provenance_id.clone(),
        kind: provenance.kind,
        source_thread_id: provenance.source_thread_id.clone(),
        source_message_id: provenance.source_message_id.clone(),
        evidence_refs: provenance.evidence_refs.clone(),
        advisory_only: provenance.advisory_only,
        completion_notice: provenance.completion_notice,
        transcription_confirmed: provenance.transcription_confirmed,
        authority_effect: AuthorityEffect::None,
        mutates_authority: false,
    })
}

fn validate_refs(
    errors: &mut Vec<RoleMailboxHandoffBundleValidationError>,
    controls: &RoleMailboxHandoffBundleControlsV1,
) {
    for required_ref in [
        "kernel.role_mailbox_contract",
        "kernel.role_mailbox_loop_control",
        "kernel.role_mailbox_claim_lease",
        "kernel.workflow_transition_registry",
        "kernel.locus_work_tracking",
        "kernel.dcc_layout_projection_registry",
    ] {
        if !contains_exact(&controls.product_authority_refs, required_ref) {
            errors.push(RoleMailboxHandoffBundleValidationError {
                field: "product_authority_refs",
                message: "handoff bundles must cite Role Mailbox, loop, claim-lease, workflow, Locus, and DCC refs",
            });
        }
    }
}

fn validate_bundles(
    errors: &mut Vec<RoleMailboxHandoffBundleValidationError>,
    bundles: &[RoleMailboxHandoffBundleV1],
) {
    let mut bundle_ids = HashSet::new();
    for bundle in bundles {
        if !bundle_ids.insert(bundle.bundle_id.as_str()) {
            errors.push(RoleMailboxHandoffBundleValidationError {
                field: "bundle_id",
                message: "handoff bundle ids must be unique",
            });
        }

        require_non_empty(errors, "bundle_id", &bundle.bundle_id);
        require_non_empty(errors, "thread_id", &bundle.thread_id);
        require_non_empty(errors, "source_message_id", &bundle.source_message_id);
        require_non_empty(
            errors,
            "linked_work_packet_id",
            &bundle.linked_work_packet_id,
        );
        require_non_empty(errors, "linked_micro_task_id", &bundle.linked_micro_task_id);
        require_vec(errors, "locus_join_refs", &bundle.locus_join_refs);
        require_non_empty(errors, "remaining_work", &bundle.remaining_work);
        require_vec(errors, "unresolved_blockers", &bundle.unresolved_blockers);
        require_non_empty(errors, "changed_scope", &bundle.changed_scope);
        require_vec(errors, "evidence_refs", &bundle.evidence_refs);
        require_non_empty(
            errors,
            "recommended_next_actor.actor_id",
            &bundle.recommended_next_actor.actor_id,
        );
        require_non_empty(
            errors,
            "recommended_next_actor.reason",
            &bundle.recommended_next_actor.reason,
        );
        require_vec(
            errors,
            "transcription_targets",
            &bundle.transcription_targets,
        );
        require_vec(
            errors,
            "announce_back_provenance",
            &bundle.announce_back_provenance,
        );
        require_non_empty(errors, "compact_summary", &bundle.compact_summary);

        if bundle.handoff_ready && bundle.bundle_state != HandoffBundleState::HandoffReady {
            errors.push(RoleMailboxHandoffBundleValidationError {
                field: "handoff_ready",
                message: "handoff-ready flag must align with handoff-ready bundle state",
            });
        }
        if bundle.transcript_replay_required {
            errors.push(RoleMailboxHandoffBundleValidationError {
                field: "transcript_replay_required",
                message: "handoff bundles must be queryable without transcript replay",
            });
        }
        if bundle.announce_back_authoritative_for_completion {
            errors.push(RoleMailboxHandoffBundleValidationError {
                field: "announce_back_authoritative_for_completion",
                message: "announce-back messages must not become completion authority",
            });
        }

        validate_transcription_targets(errors, &bundle.transcription_targets);
        validate_announce_back_provenance(errors, &bundle.announce_back_provenance);
    }
}

fn validate_transcription_targets(
    errors: &mut Vec<RoleMailboxHandoffBundleValidationError>,
    targets: &[TranscriptionTargetV1],
) {
    let mut target_ids = HashSet::new();
    for target in targets {
        if !target_ids.insert(target.target_id.as_str()) {
            errors.push(RoleMailboxHandoffBundleValidationError {
                field: "transcription_targets.target_id",
                message: "transcription target ids must be unique",
            });
        }
        require_non_empty(errors, "transcription_targets.target_id", &target.target_id);
        require_non_empty(
            errors,
            "transcription_targets.target_ref",
            &target.target_ref,
        );
        if target.required && target.status == TranscriptionStatus::NotRequired {
            errors.push(RoleMailboxHandoffBundleValidationError {
                field: "transcription_targets.status",
                message: "required transcription targets cannot be marked not-required",
            });
        }
    }
}

fn validate_announce_back_provenance(
    errors: &mut Vec<RoleMailboxHandoffBundleValidationError>,
    provenance_entries: &[AnnounceBackProvenanceV1],
) {
    let mut provenance_ids = HashSet::new();
    let mut provenance_kinds = HashSet::new();
    for provenance in provenance_entries {
        if !provenance_ids.insert(provenance.provenance_id.as_str()) {
            errors.push(RoleMailboxHandoffBundleValidationError {
                field: "announce_back_provenance.provenance_id",
                message: "announce-back provenance ids must be unique",
            });
        }
        provenance_kinds.insert(provenance.kind);
        require_non_empty(
            errors,
            "announce_back_provenance.provenance_id",
            &provenance.provenance_id,
        );
        require_non_empty(
            errors,
            "announce_back_provenance.source_thread_id",
            &provenance.source_thread_id,
        );
        require_non_empty(
            errors,
            "announce_back_provenance.source_message_id",
            &provenance.source_message_id,
        );
        require_vec(
            errors,
            "announce_back_provenance.evidence_refs",
            &provenance.evidence_refs,
        );
        if provenance.mutates_authority {
            errors.push(RoleMailboxHandoffBundleValidationError {
                field: "announce_back_provenance.mutates_authority",
                message: "announce-back provenance must not mutate authority",
            });
        }
        validate_provenance_kind(errors, provenance);
    }

    for required_kind in REQUIRED_PROVENANCE_KINDS {
        if !provenance_kinds.contains(&required_kind) {
            errors.push(RoleMailboxHandoffBundleValidationError {
                field: "announce_back_provenance.kind",
                message: "advisory, completion, escalation, scope-change, handoff-ready, and transcription-confirmed provenance are required",
            });
        }
    }
}

fn validate_provenance_kind(
    errors: &mut Vec<RoleMailboxHandoffBundleValidationError>,
    provenance: &AnnounceBackProvenanceV1,
) {
    match provenance.kind {
        AnnounceBackProvenanceKind::AdvisoryStatus
        | AnnounceBackProvenanceKind::EscalationSummary
        | AnnounceBackProvenanceKind::ScopeChangeNotice
        | AnnounceBackProvenanceKind::HandoffReady => {
            if !provenance.advisory_only || provenance.completion_notice {
                errors.push(RoleMailboxHandoffBundleValidationError {
                    field: "announce_back_provenance.advisory_only",
                    message: "advisory announce-back kinds must be advisory-only and not completion notices",
                });
            }
        }
        AnnounceBackProvenanceKind::CompletionNotice => {
            if provenance.advisory_only || !provenance.completion_notice {
                errors.push(RoleMailboxHandoffBundleValidationError {
                    field: "announce_back_provenance.completion_notice",
                    message: "completion notices must be typed distinctly from advisory status",
                });
            }
        }
        AnnounceBackProvenanceKind::TranscriptionConfirmedOutcome => {
            if !provenance.transcription_confirmed || !provenance.completion_notice {
                errors.push(RoleMailboxHandoffBundleValidationError {
                    field: "announce_back_provenance.transcription_confirmed",
                    message: "transcription-confirmed outcomes require confirmed transcription and completion notice posture",
                });
            }
        }
    }
}

fn require_non_empty(
    errors: &mut Vec<RoleMailboxHandoffBundleValidationError>,
    field: &'static str,
    value: &str,
) {
    if value.trim().is_empty() {
        errors.push(RoleMailboxHandoffBundleValidationError {
            field,
            message: "value must not be empty",
        });
    }
}

fn require_vec<T>(
    errors: &mut Vec<RoleMailboxHandoffBundleValidationError>,
    field: &'static str,
    value: &[T],
) {
    if value.is_empty() {
        errors.push(RoleMailboxHandoffBundleValidationError {
            field,
            message: "at least one value is required",
        });
    }
}

fn contains_exact(values: &[String], needle: &str) -> bool {
    values.iter().any(|value| value == needle)
}

fn contains_text(values: &[String], needle: &str) -> bool {
    values.iter().any(|value| value.contains(needle))
}
