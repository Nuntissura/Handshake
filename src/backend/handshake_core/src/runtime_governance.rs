use std::{
    io,
    path::{Component, Path, PathBuf},
};

use serde::{Deserialize, Serialize};

use crate::workflows::locus::{
    self,
    task_board::{SoftwareDeliveryCloseoutProjectionBadgeV1, TaskBoardEntryRecordV1},
};
use crate::storage::artifacts::resolve_workspace_root;

pub const RUNTIME_GOVERNANCE_ROOT_ENV: &str = "HANDSHAKE_GOVERNANCE_ROOT";
pub const RUNTIME_GOVERNANCE_DEFAULT_ROOT: &str = ".handshake/gov";
pub const RUNTIME_TASK_BOARD_FILE: &str = "TASK_BOARD.md";
pub const RUNTIME_WP_TRACEABILITY_REGISTRY_FILE: &str = "WP_TRACEABILITY_REGISTRY.md";
pub const RUNTIME_SPEC_CURRENT_FILE: &str = "SPEC_CURRENT.md";
pub const RUNTIME_ROLE_MAILBOX_DIR: &str = "ROLE_MAILBOX";
pub const RUNTIME_WORK_PACKETS_DIR: &str = "work_packets";
pub const RUNTIME_MICRO_TASKS_DIR: &str = "micro_tasks";
pub const RUNTIME_TASK_BOARD_DIR: &str = "task_board";
pub const RUNTIME_TASK_BOARD_VIEWS_DIR: &str = "views";
pub const RUNTIME_VALIDATOR_GATES_DIR: &str = "validator_gates";
pub const RUNTIME_ACTIVATION_TRACEABILITY_DIR: &str = "activation_traceability";
pub const RUNTIME_GOVERNANCE_DECISIONS_DIR: &str = "governance_decisions";
pub const RUNTIME_GOVERNANCE_AUTO_SIGNATURES_DIR: &str = "auto_signatures";
// MT-003 v02.181: canonical lineage directory for software-delivery closeout
// posture derivation. Checkpoint lineage MUST resolve to this
// governance-root-relative segment so closeout derivation cannot be tricked
// by spoofed evidence refs.
pub const RUNTIME_CHECKPOINTS_DIR: &str = "checkpoints";
// MT-004 v02.181: canonical overlay record directories for software-delivery
// claim/lease and queued-instruction records. Overlay records MUST resolve
// to these governance-root-relative segments so projection surfaces and
// mailbox triage rows cannot be tricked into surfacing spoofed paths as
// authoritative overlay state.
pub const RUNTIME_CLAIM_LEASES_DIR: &str = "claim_leases";
pub const RUNTIME_QUEUED_INSTRUCTIONS_DIR: &str = "queued_instructions";
// MT-004 v02.181: canonical workflow run lifecycle record directory. Each
// software-delivery work_packet_id has at most one record at
// `<gov_root>/workflow_runs/<wp_id>.json` carrying workflow_failed,
// workflow_canceled, workflow_settled, and has_unresolved_governed_actions
// posture plus the stable workflow_run_id/workflow_binding_id/model_session_id.
pub const RUNTIME_WORKFLOW_RUNS_DIR: &str = "workflow_runs";

// DCC Control Plane projection constants
pub const DCC_CONTROL_PLANE_SCHEMA_ID: &str = "hsk.dcc_control_plane_snapshot@1";
pub const DCC_CONTROL_PLANE_SCHEMA_VERSION: &str = "1.0.0";

/// Read-only projection aggregate for the Dev Command Center control plane.
/// All state is sourced from existing backend artifacts — never a second authority.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DccControlPlaneSnapshot {
    pub schema_id: String,
    pub schema_version: String,
    pub snapshot_id: String,
    pub generated_at: String,
    pub work_state: DccWorkState,
    pub session_state: DccSessionState,
    pub governance_state: DccGovernanceState,
    pub collaboration_state: DccCollaborationState,
}

/// Work orchestration projection from task board entries.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DccWorkState {
    pub task_board_id: String,
    pub entries: Vec<TaskBoardEntryRecordV1>,
    pub active_workflow_summaries: Vec<DccWorkflowSummary>,
    pub freshness: String,
    /// Work packet IDs whose workflow_state_family is Ready — ready-query result.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub ready_queue: Vec<String>,
}

/// Compact workflow state per work packet — stable-id-first.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DccWorkflowSummary {
    pub work_packet_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workflow_run_id: Option<String>,
    pub state_family: locus::WorkflowStateFamily,
    pub queue_reason_code: locus::WorkflowQueueReasonCode,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub allowed_action_ids: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model_session_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub summary_ref: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub micro_task_summary: Option<DccMicroTaskSummary>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gate_state: Option<DccGateState>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub closeout_badge: Option<SoftwareDeliveryCloseoutProjectionBadgeV1>,
    #[serde(default)]
    pub authority_refs: Vec<String>,
    #[serde(default)]
    pub evidence_refs: Vec<String>,
}

/// Aggregated micro-task status per work packet.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DccMicroTaskSummary {
    pub total: u32,
    pub completed: u32,
    pub failed: u32,
    pub in_progress: u32,
    pub blocked: u32,
    #[serde(default)]
    pub mt_ids: Vec<String>,
}

/// Hard-gate state projection per work packet.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DccGateState {
    pub pre_work: String,
    pub post_work: String,
}

/// Session binding projection from SessionRegistry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DccSessionState {
    pub bindings: Vec<DccSessionBinding>,
}

/// Individual session entry for DCC projection.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DccSessionBinding {
    pub session_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub worktree_dir: Option<String>,
    pub role: String,
    pub state: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bound_work_packet_id: Option<String>,
    /// Model provider identity for parallel session occupancy.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model_id: Option<String>,
    /// Backend provider for parallel session occupancy.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub backend: Option<String>,
    /// Micro-task ID this session is bound to, if any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bound_micro_task_id: Option<String>,
}

/// Governance evidence projection from RuntimeGovernancePaths artifacts.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DccGovernanceState {
    pub governance_root: String,
    #[serde(default)]
    pub pending_decisions: Vec<String>,
    /// Approval decision state read from governance_decisions JSON files.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub approval_decisions: Vec<DccApprovalDecision>,
    #[serde(default)]
    pub active_auto_signatures: Vec<String>,
    #[serde(default)]
    pub effective_capability_axes: Vec<String>,
    #[serde(default)]
    pub effective_capability_ids: Vec<String>,
    #[serde(default)]
    pub authority_refs: Vec<String>,
    #[serde(default)]
    pub evidence_refs: Vec<String>,
}

/// Projected approval decision from governance_decisions directory.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DccApprovalDecision {
    pub decision_id: String,
    pub gate_type: String,
    pub target_ref: String,
    pub decision: String,
    pub timestamp: String,
}

/// Collaboration state projection from role-mailbox export artifacts.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DccCollaborationState {
    pub active_threads: Vec<DccMailboxThreadSummary>,
    pub pending_wait_reasons: Vec<DccWaitReason>,
    pub mailbox_summary: DccMailboxSummary,
}

/// Projected summary of a single mailbox thread.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DccMailboxThreadSummary {
    pub thread_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub work_packet_id: Option<String>,
    pub participants: Vec<String>,
    pub message_count: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub latest_message_type: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub latest_from_role: Option<String>,
    pub created_at: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub closed_at: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub evidence_refs: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub linked_work_ids: Vec<String>,
}

/// Pending wait reason derived from the latest unanswered request in a thread.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DccWaitReason {
    pub thread_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub work_packet_id: Option<String>,
    pub waiting_for_role: String,
    pub expected_response: String,
    pub wait_since: String,
}

/// Aggregate mailbox statistics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DccMailboxSummary {
    pub total_threads: u64,
    pub active_threads: u64,
    pub total_messages: u64,
}

// ── Compact summary derivation (MT-005) ──────────────────────────────

impl DccControlPlaneSnapshot {
    /// Derive compact summary-first payloads from the full snapshot.
    /// One `DccCompactSummaryV1` per work packet in `work_state`, cross-
    /// referencing session bindings and collaboration state for routing hints.
    /// Shares record_id, project_profile_kind, and authority_refs with the
    /// canonical detail record so deterministic joins remain possible.
    pub fn compact_summaries(&self) -> Vec<locus::DccCompactSummaryV1> {
        let descriptor = locus::structured_collaboration_schema_descriptor(
            locus::StructuredCollaborationRecordFamily::DccCompactSummary,
        );

        self.work_state
            .active_workflow_summaries
            .iter()
            .map(|ws| {
                let session_bound = ws.model_session_id.is_some();

                // Collect wait reasons linked to this work packet
                let wp_waits: Vec<&DccWaitReason> = self
                    .collaboration_state
                    .pending_wait_reasons
                    .iter()
                    .filter(|w| w.work_packet_id.as_deref() == Some(&ws.work_packet_id))
                    .collect();
                let pending_wait_count = wp_waits.len() as u32;

                let active_thread_count = self
                    .collaboration_state
                    .active_threads
                    .iter()
                    .filter(|t| {
                        t.work_packet_id.as_deref() == Some(&ws.work_packet_id)
                            && t.closed_at.is_none()
                    })
                    .count() as u32;

                // Derive status from state family
                let status = format!("{:?}", ws.state_family);

                // Derive bounded title_or_objective for triage without detail load.
                // Combines work_packet_id, state, and queue context into a
                // bounded 160-char string usable by local-small-model routing.
                let queue_label = queue_reason_label(ws.queue_reason_code);
                let mt_progress = ws.micro_task_summary.as_ref().map(|mt| {
                    format!(" [{}/{} MTs]", mt.completed, mt.total)
                });
                let raw_title = format!(
                    "{}: {} — {}{}",
                    ws.work_packet_id,
                    status,
                    queue_label,
                    mt_progress.as_deref().unwrap_or(""),
                );
                let title_or_objective = bounded_text(&raw_title, 160);

                // Derive blockers from wait reasons and blocked-family state
                let mut blockers: Vec<String> = Vec::new();
                for wr in &wp_waits {
                    blockers.push(bounded_text(
                        &format!(
                            "waiting for {} ({})",
                            wr.waiting_for_role, wr.expected_response
                        ),
                        160,
                    ));
                }
                if matches!(
                    ws.state_family,
                    locus::WorkflowStateFamily::Blocked
                ) && blockers.is_empty()
                {
                    blockers.push(bounded_text(
                        &format!("blocked: {}", queue_label),
                        160,
                    ));
                }

                // Derive next_action from allowed actions or state family default
                let next_action = if let Some(first) = ws.allowed_action_ids.first() {
                    Some(first.clone())
                } else {
                    default_next_action(ws.state_family)
                };

                // Find matching task board entry for profile_kind and mirror_state
                let entry = self
                    .work_state
                    .entries
                    .iter()
                    .find(|e| e.work_packet_id == ws.work_packet_id);
                let project_profile_kind = entry
                    .map(|e| e.project_profile_kind)
                    .unwrap_or(locus::ProjectProfileKind::SoftwareDelivery);
                let mirror_state = entry
                    .map(|e| e.mirror_state)
                    .unwrap_or(locus::MirrorSyncState::CanonicalOnly);

                locus::DccCompactSummaryV1 {
                    schema_id: descriptor.schema_id.to_string(),
                    schema_version: descriptor.schema_version.to_string(),
                    record_id: ws.work_packet_id.clone(),
                    record_kind: descriptor.record_kind.to_string(),
                    project_profile_kind,
                    updated_at: self.generated_at.clone(),
                    mirror_state,
                    authority_refs: ws.authority_refs.clone(),
                    evidence_refs: ws.evidence_refs.clone(),
                    mirror_contract: None,
                    workflow_state_family: ws.state_family,
                    queue_reason_code: ws.queue_reason_code,
                    allowed_action_ids: ws.allowed_action_ids.clone(),
                    status,
                    title_or_objective,
                    blockers,
                    next_action,
                    summary_ref: ws.summary_ref.clone(),
                    work_packet_id: ws.work_packet_id.clone(),
                    task_board_id: Some(self.work_state.task_board_id.clone()),
                    workflow_run_id: ws.workflow_run_id.clone(),
                    model_session_id: ws.model_session_id.clone(),
                    pending_wait_count,
                    active_thread_count,
                    session_bound,
                }
            })
            .collect()
    }
}

/// Bounded text: collapse whitespace and truncate to max_len chars.
fn bounded_text(value: &str, max_len: usize) -> String {
    let collapsed = value.split_whitespace().collect::<Vec<_>>().join(" ");
    let mut out = String::new();
    for ch in collapsed.chars() {
        if out.chars().count() >= max_len {
            break;
        }
        out.push(ch);
    }
    out.trim().to_string()
}

/// Human-readable label for a queue reason code, for triage routing.
fn queue_reason_label(code: locus::WorkflowQueueReasonCode) -> &'static str {
    match code {
        locus::WorkflowQueueReasonCode::NewUntriaged => "new, untriaged",
        locus::WorkflowQueueReasonCode::DependencyWait => "dependency wait",
        locus::WorkflowQueueReasonCode::ReadyForLocalSmallModel => "ready for local model",
        locus::WorkflowQueueReasonCode::ReadyForCloudModel => "ready for cloud model",
        locus::WorkflowQueueReasonCode::ReadyForHuman => "ready for human",
        locus::WorkflowQueueReasonCode::ReviewWait => "review wait",
        locus::WorkflowQueueReasonCode::ApprovalWait => "approval wait",
        locus::WorkflowQueueReasonCode::ValidationWait => "validation wait",
        locus::WorkflowQueueReasonCode::MailboxResponseWait => "mailbox response wait",
        locus::WorkflowQueueReasonCode::TimerWait => "timer wait",
        locus::WorkflowQueueReasonCode::BlockedMissingContext => "blocked: missing context",
        locus::WorkflowQueueReasonCode::BlockedPolicy => "blocked: policy",
        locus::WorkflowQueueReasonCode::BlockedCapability => "blocked: capability",
        locus::WorkflowQueueReasonCode::BlockedError => "blocked: error",
    }
}

/// Default next action for a state family when no allowed_action_ids are set.
fn default_next_action(family: locus::WorkflowStateFamily) -> Option<String> {
    match family {
        locus::WorkflowStateFamily::Intake => Some("triage".into()),
        locus::WorkflowStateFamily::Ready => Some("start".into()),
        locus::WorkflowStateFamily::Active => Some("continue".into()),
        locus::WorkflowStateFamily::Waiting => Some("wait".into()),
        locus::WorkflowStateFamily::Review => Some("complete_review".into()),
        locus::WorkflowStateFamily::Approval => Some("approve".into()),
        locus::WorkflowStateFamily::Validation => Some("validate".into()),
        locus::WorkflowStateFamily::Blocked => Some("unblock".into()),
        locus::WorkflowStateFamily::Done
        | locus::WorkflowStateFamily::Canceled
        | locus::WorkflowStateFamily::Archived => None,
    }
}

#[derive(Debug, Clone)]
pub struct RuntimeGovernancePaths {
    workspace_root: PathBuf,
    governance_root: PathBuf,
}

impl RuntimeGovernancePaths {
    pub fn resolve() -> Result<Self, io::Error> {
        let workspace_root = resolve_workspace_root()
            .map_err(|err| io::Error::new(io::ErrorKind::Other, err.to_string()))?;
        let configured = std::env::var(RUNTIME_GOVERNANCE_ROOT_ENV)
            .ok()
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
            .map(PathBuf::from);
        Self::from_workspace_root_with_override(workspace_root, configured)
    }

    pub fn from_workspace_root(workspace_root: PathBuf) -> Result<Self, io::Error> {
        Self::from_workspace_root_with_override(workspace_root, None)
    }

    fn from_workspace_root_with_override(
        workspace_root: PathBuf,
        override_root: Option<PathBuf>,
    ) -> Result<Self, io::Error> {
        let workspace_root = absolutize(workspace_root)?;
        let configured =
            override_root.unwrap_or_else(|| PathBuf::from(RUNTIME_GOVERNANCE_DEFAULT_ROOT));
        if has_parent_dir(&configured) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "runtime governance root must not contain '..' segments",
            ));
        }

        let governance_root = if configured.is_absolute() {
            configured
        } else {
            workspace_root.join(configured)
        };
        let governance_root = absolutize(governance_root)?;

        ensure_runtime_boundary(&workspace_root, &governance_root)?;

        Ok(Self {
            workspace_root,
            governance_root,
        })
    }

    pub fn workspace_root(&self) -> &Path {
        &self.workspace_root
    }

    pub fn governance_root(&self) -> &Path {
        &self.governance_root
    }

    pub fn governance_root_display(&self) -> String {
        ensure_trailing_slash(display_path(&self.workspace_root, &self.governance_root))
    }

    pub fn spec_current_path(&self) -> PathBuf {
        self.governance_root.join(RUNTIME_SPEC_CURRENT_FILE)
    }

    pub fn spec_current_display(&self) -> String {
        display_path(&self.workspace_root, &self.spec_current_path())
    }

    pub fn task_board_path(&self) -> PathBuf {
        self.governance_root.join(RUNTIME_TASK_BOARD_FILE)
    }

    pub fn task_board_display(&self) -> String {
        display_path(&self.workspace_root, &self.task_board_path())
    }

    pub fn wp_traceability_registry_path(&self) -> PathBuf {
        self.governance_root
            .join(RUNTIME_WP_TRACEABILITY_REGISTRY_FILE)
    }

    pub fn wp_traceability_registry_display(&self) -> String {
        display_path(&self.workspace_root, &self.wp_traceability_registry_path())
    }

    pub fn role_mailbox_export_dir(&self) -> PathBuf {
        self.governance_root.join(RUNTIME_ROLE_MAILBOX_DIR)
    }

    pub fn role_mailbox_export_dir_display(&self) -> String {
        ensure_trailing_slash(display_path(
            &self.workspace_root,
            &self.role_mailbox_export_dir(),
        ))
    }

    pub fn work_packets_dir(&self) -> PathBuf {
        self.governance_root.join(RUNTIME_WORK_PACKETS_DIR)
    }

    pub fn work_packets_dir_display(&self) -> String {
        ensure_trailing_slash(display_path(&self.workspace_root, &self.work_packets_dir()))
    }

    pub fn work_packet_dir(&self, wp_id: &str) -> PathBuf {
        self.work_packets_dir().join(wp_id)
    }

    pub fn work_packet_dir_display(&self, wp_id: &str) -> String {
        ensure_trailing_slash(display_path(
            &self.workspace_root,
            &self.work_packet_dir(wp_id),
        ))
    }

    pub fn work_packet_packet_path(&self, wp_id: &str) -> PathBuf {
        self.work_packet_dir(wp_id).join("packet.json")
    }

    pub fn work_packet_packet_display(&self, wp_id: &str) -> String {
        display_path(&self.workspace_root, &self.work_packet_packet_path(wp_id))
    }

    pub fn work_packet_summary_path(&self, wp_id: &str) -> PathBuf {
        self.work_packet_dir(wp_id).join("summary.json")
    }

    pub fn work_packet_summary_display(&self, wp_id: &str) -> String {
        display_path(&self.workspace_root, &self.work_packet_summary_path(wp_id))
    }

    // MT-003 v02.181: software-delivery closeout posture artifact path.
    pub fn work_packet_closeout_posture_path(&self, wp_id: &str) -> PathBuf {
        self.work_packet_dir(wp_id).join("closeout_posture.json")
    }

    pub fn work_packet_closeout_posture_display(&self, wp_id: &str) -> String {
        display_path(
            &self.workspace_root,
            &self.work_packet_closeout_posture_path(wp_id),
        )
    }

    // MT-004 v02.181: software-delivery projection surface artifact path.
    pub fn work_packet_projection_surface_path(&self, wp_id: &str) -> PathBuf {
        self.work_packet_dir(wp_id).join("projection_surface.json")
    }

    pub fn work_packet_projection_surface_display(&self, wp_id: &str) -> String {
        display_path(
            &self.workspace_root,
            &self.work_packet_projection_surface_path(wp_id),
        )
    }

    pub fn work_packet_notes_dir(&self, wp_id: &str) -> PathBuf {
        self.work_packet_dir(wp_id).join("notes")
    }

    pub fn work_packet_note_path(&self, wp_id: &str, note_id: &str) -> PathBuf {
        self.work_packet_notes_dir(wp_id)
            .join(format!("{note_id}.md"))
    }

    pub fn work_packet_note_display(&self, wp_id: &str, note_id: &str) -> String {
        display_path(
            &self.workspace_root,
            &self.work_packet_note_path(wp_id, note_id),
        )
    }

    pub fn micro_tasks_dir(&self) -> PathBuf {
        self.governance_root.join(RUNTIME_MICRO_TASKS_DIR)
    }

    pub fn micro_tasks_dir_display(&self) -> String {
        ensure_trailing_slash(display_path(&self.workspace_root, &self.micro_tasks_dir()))
    }

    pub fn micro_task_dir(&self, wp_id: &str, mt_id: &str) -> PathBuf {
        self.micro_tasks_dir().join(wp_id).join(mt_id)
    }

    pub fn micro_task_dir_display(&self, wp_id: &str, mt_id: &str) -> String {
        ensure_trailing_slash(display_path(
            &self.workspace_root,
            &self.micro_task_dir(wp_id, mt_id),
        ))
    }

    pub fn micro_task_packet_path(&self, wp_id: &str, mt_id: &str) -> PathBuf {
        self.micro_task_dir(wp_id, mt_id).join("packet.json")
    }

    pub fn micro_task_packet_display(&self, wp_id: &str, mt_id: &str) -> String {
        display_path(
            &self.workspace_root,
            &self.micro_task_packet_path(wp_id, mt_id),
        )
    }

    pub fn micro_task_summary_path(&self, wp_id: &str, mt_id: &str) -> PathBuf {
        self.micro_task_dir(wp_id, mt_id).join("summary.json")
    }

    pub fn micro_task_summary_display(&self, wp_id: &str, mt_id: &str) -> String {
        display_path(
            &self.workspace_root,
            &self.micro_task_summary_path(wp_id, mt_id),
        )
    }

    pub fn task_board_projection_dir(&self) -> PathBuf {
        self.governance_root.join(RUNTIME_TASK_BOARD_DIR)
    }

    pub fn task_board_projection_dir_display(&self) -> String {
        ensure_trailing_slash(display_path(
            &self.workspace_root,
            &self.task_board_projection_dir(),
        ))
    }

    pub fn task_board_index_path(&self) -> PathBuf {
        self.task_board_projection_dir().join("index.json")
    }

    pub fn task_board_index_display(&self) -> String {
        display_path(&self.workspace_root, &self.task_board_index_path())
    }

    pub fn task_board_views_dir(&self) -> PathBuf {
        self.task_board_projection_dir()
            .join(RUNTIME_TASK_BOARD_VIEWS_DIR)
    }

    pub fn task_board_views_dir_display(&self) -> String {
        ensure_trailing_slash(display_path(
            &self.workspace_root,
            &self.task_board_views_dir(),
        ))
    }

    pub fn task_board_view_path(&self, view_id: &str) -> PathBuf {
        self.task_board_views_dir().join(format!("{view_id}.json"))
    }

    pub fn task_board_view_display(&self, view_id: &str) -> String {
        display_path(&self.workspace_root, &self.task_board_view_path(view_id))
    }

    pub fn governance_decisions_dir(&self) -> PathBuf {
        self.governance_root.join(RUNTIME_GOVERNANCE_DECISIONS_DIR)
    }

    pub fn governance_decisions_dir_display(&self) -> String {
        ensure_trailing_slash(display_path(
            &self.workspace_root,
            &self.governance_decisions_dir(),
        ))
    }

    pub fn governance_decision_path(&self, decision_id: &str) -> PathBuf {
        self.governance_decisions_dir()
            .join(format!("{decision_id}.json"))
    }

    pub fn governance_decision_display(&self, decision_id: &str) -> String {
        display_path(
            &self.workspace_root,
            &self.governance_decision_path(decision_id),
        )
    }

    pub fn validator_gates_dir(&self) -> PathBuf {
        self.governance_root.join(RUNTIME_VALIDATOR_GATES_DIR)
    }

    pub fn validator_gate_path(&self, wp_id: &str) -> PathBuf {
        self.validator_gates_dir().join(format!("{wp_id}.json"))
    }

    pub fn validator_gate_display(&self, wp_id: &str) -> String {
        display_path(&self.workspace_root, &self.validator_gate_path(wp_id))
    }

    pub fn activation_traceability_dir(&self) -> PathBuf {
        self.governance_root
            .join(RUNTIME_ACTIVATION_TRACEABILITY_DIR)
    }

    pub fn activation_traceability_dir_display(&self) -> String {
        ensure_trailing_slash(display_path(
            &self.workspace_root,
            &self.activation_traceability_dir(),
        ))
    }

    pub fn activation_traceability_path(&self, wp_id: &str) -> PathBuf {
        self.activation_traceability_dir()
            .join(format!("{wp_id}.json"))
    }

    pub fn activation_traceability_display(&self, wp_id: &str) -> String {
        display_path(
            &self.workspace_root,
            &self.activation_traceability_path(wp_id),
        )
    }

    pub fn auto_signatures_dir(&self) -> PathBuf {
        self.governance_root
            .join(RUNTIME_GOVERNANCE_AUTO_SIGNATURES_DIR)
    }

    pub fn auto_signatures_dir_display(&self) -> String {
        ensure_trailing_slash(display_path(
            &self.workspace_root,
            &self.auto_signatures_dir(),
        ))
    }

    pub fn auto_signature_path(&self, auto_signature_id: &str) -> PathBuf {
        self.auto_signatures_dir()
            .join(format!("{auto_signature_id}.json"))
    }

    // MT-003 v02.181: canonical software-delivery closeout lineage paths.
    pub fn validator_gates_dir_display(&self) -> String {
        ensure_trailing_slash(display_path(
            &self.workspace_root,
            &self.validator_gates_dir(),
        ))
    }

    pub fn validator_gate_record_path(&self, wp_id: &str) -> PathBuf {
        self.validator_gates_dir().join(format!("{wp_id}.json"))
    }

    pub fn validator_gate_record_display(&self, wp_id: &str) -> String {
        display_path(
            &self.workspace_root,
            &self.validator_gate_record_path(wp_id),
        )
    }

    pub fn checkpoints_dir(&self) -> PathBuf {
        self.governance_root.join(RUNTIME_CHECKPOINTS_DIR)
    }

    pub fn checkpoints_dir_display(&self) -> String {
        ensure_trailing_slash(display_path(
            &self.workspace_root,
            &self.checkpoints_dir(),
        ))
    }

    pub fn checkpoint_record_path(&self, checkpoint_id: &str) -> PathBuf {
        self.checkpoints_dir()
            .join(format!("{checkpoint_id}.json"))
    }

    pub fn checkpoint_record_display(&self, checkpoint_id: &str) -> String {
        display_path(
            &self.workspace_root,
            &self.checkpoint_record_path(checkpoint_id),
        )
    }

    /// True iff `value` is a canonical validator-gate evidence ref under the
    /// product runtime governance root for the SAME stable id `expected_wp_id`:
    /// `<gov_root>/validator_gates/<expected_wp_id>.json`. Substring spoofs
    /// (e.g. `/notes/validator_gates/...`, no `.json` suffix) and same-record
    /// spoofs (foreign WP ids) are rejected.
    pub fn is_canonical_validator_gate_ref(&self, value: &str, expected_wp_id: &str) -> bool {
        if expected_wp_id.is_empty() || expected_wp_id.contains('/') {
            return false;
        }
        let normalized = normalize_display_like(value);
        if normalized.is_empty() || !normalized.ends_with(".json") {
            return false;
        }
        let prefix = normalize_display_like(&self.validator_gates_dir_display());
        if !normalized.starts_with(&prefix) {
            return false;
        }
        let after_prefix = &normalized[prefix.len()..];
        let expected = format!("{expected_wp_id}.json");
        after_prefix == expected
    }

    /// True iff `value` is a canonical work-packet `packet.json` authority ref
    /// under the product runtime governance root for the SAME stable id
    /// `expected_wp_id`: `<gov_root>/work_packets/<expected_wp_id>/packet.json`.
    /// Substring spoofs and foreign WP ids are rejected.
    pub fn is_canonical_work_packet_packet_ref(&self, value: &str, expected_wp_id: &str) -> bool {
        if expected_wp_id.is_empty() || expected_wp_id.contains('/') {
            return false;
        }
        let normalized = normalize_display_like(value);
        if normalized.is_empty() || !normalized.ends_with("/packet.json") {
            return false;
        }
        let prefix = normalize_display_like(&self.work_packets_dir_display());
        if !normalized.starts_with(&prefix) {
            return false;
        }
        let inner = &normalized[prefix.len()..];
        let expected = format!("{expected_wp_id}/packet.json");
        inner == expected
    }

    // ── MT-004 v02.181: claim/lease canonical paths ─────────────────────────

    /// True iff `value` is a canonical governed-action decision/ref under the
    /// product runtime governance root: `<gov_root>/governance_decisions/<id>.json`.
    /// Substring spoofs and nested paths are rejected.
    pub fn is_canonical_governance_decision_ref(&self, value: &str) -> bool {
        let normalized = normalize_display_like(value);
        if normalized.is_empty() || !normalized.ends_with(".json") {
            return false;
        }
        let prefix = normalize_display_like(&self.governance_decisions_dir_display());
        if !normalized.starts_with(&prefix) {
            return false;
        }
        let inner = &normalized[prefix.len()..];
        inner
            .strip_suffix(".json")
            .is_some_and(|decision_id| !decision_id.is_empty() && !decision_id.contains('/'))
    }

    pub fn claim_leases_dir(&self) -> PathBuf {
        self.governance_root.join(RUNTIME_CLAIM_LEASES_DIR)
    }

    pub fn claim_leases_dir_display(&self) -> String {
        ensure_trailing_slash(display_path(
            &self.workspace_root,
            &self.claim_leases_dir(),
        ))
    }

    pub fn claim_lease_dir(&self, wp_id: &str) -> PathBuf {
        self.claim_leases_dir().join(wp_id)
    }

    pub fn claim_lease_record_path(&self, wp_id: &str, claim_id: &str) -> PathBuf {
        self.claim_lease_dir(wp_id).join(format!("{claim_id}.json"))
    }

    pub fn claim_lease_record_display(&self, wp_id: &str, claim_id: &str) -> String {
        display_path(
            &self.workspace_root,
            &self.claim_lease_record_path(wp_id, claim_id),
        )
    }

    /// True iff `value` is a canonical claim/lease record ref under the
    /// product runtime governance root for the SAME stable ids
    /// `expected_wp_id`/`expected_claim_id`:
    /// `<gov_root>/claim_leases/<expected_wp_id>/<expected_claim_id>.json`.
    /// Substring spoofs and foreign WP/claim ids are rejected.
    pub fn is_canonical_claim_lease_record_ref(
        &self,
        value: &str,
        expected_wp_id: &str,
        expected_claim_id: &str,
    ) -> bool {
        if expected_wp_id.is_empty()
            || expected_wp_id.contains('/')
            || expected_claim_id.is_empty()
            || expected_claim_id.contains('/')
        {
            return false;
        }
        let normalized = normalize_display_like(value);
        if normalized.is_empty() || !normalized.ends_with(".json") {
            return false;
        }
        let prefix = normalize_display_like(&self.claim_leases_dir_display());
        if !normalized.starts_with(&prefix) {
            return false;
        }
        let inner = &normalized[prefix.len()..];
        let expected = format!("{expected_wp_id}/{expected_claim_id}.json");
        inner == expected
    }

    // ── MT-004 v02.181: queued-instruction canonical paths ──────────────────

    pub fn queued_instructions_dir(&self) -> PathBuf {
        self.governance_root.join(RUNTIME_QUEUED_INSTRUCTIONS_DIR)
    }

    pub fn queued_instructions_dir_display(&self) -> String {
        ensure_trailing_slash(display_path(
            &self.workspace_root,
            &self.queued_instructions_dir(),
        ))
    }

    pub fn queued_instruction_dir(&self, wp_id: &str) -> PathBuf {
        self.queued_instructions_dir().join(wp_id)
    }

    pub fn queued_instruction_record_path(&self, wp_id: &str, instruction_id: &str) -> PathBuf {
        self.queued_instruction_dir(wp_id)
            .join(format!("{instruction_id}.json"))
    }

    pub fn queued_instruction_record_display(&self, wp_id: &str, instruction_id: &str) -> String {
        display_path(
            &self.workspace_root,
            &self.queued_instruction_record_path(wp_id, instruction_id),
        )
    }

    /// True iff `value` is a canonical queued-instruction record ref under the
    /// product runtime governance root for the SAME stable ids
    /// `expected_wp_id`/`expected_instruction_id`:
    /// `<gov_root>/queued_instructions/<expected_wp_id>/<expected_instruction_id>.json`.
    /// Substring spoofs and foreign WP/instruction ids are rejected.
    pub fn is_canonical_queued_instruction_record_ref(
        &self,
        value: &str,
        expected_wp_id: &str,
        expected_instruction_id: &str,
    ) -> bool {
        if expected_wp_id.is_empty()
            || expected_wp_id.contains('/')
            || expected_instruction_id.is_empty()
            || expected_instruction_id.contains('/')
        {
            return false;
        }
        let normalized = normalize_display_like(value);
        if normalized.is_empty() || !normalized.ends_with(".json") {
            return false;
        }
        let prefix = normalize_display_like(&self.queued_instructions_dir_display());
        if !normalized.starts_with(&prefix) {
            return false;
        }
        let inner = &normalized[prefix.len()..];
        let expected = format!("{expected_wp_id}/{expected_instruction_id}.json");
        inner == expected
    }

    // ── MT-004 v02.181: workflow run lifecycle canonical paths ──────────────

    pub fn workflow_runs_dir(&self) -> PathBuf {
        self.governance_root.join(RUNTIME_WORKFLOW_RUNS_DIR)
    }

    pub fn workflow_runs_dir_display(&self) -> String {
        ensure_trailing_slash(display_path(
            &self.workspace_root,
            &self.workflow_runs_dir(),
        ))
    }

    pub fn workflow_run_record_path(&self, wp_id: &str) -> PathBuf {
        self.workflow_runs_dir().join(format!("{wp_id}.json"))
    }

    pub fn workflow_run_record_display(&self, wp_id: &str) -> String {
        display_path(
            &self.workspace_root,
            &self.workflow_run_record_path(wp_id),
        )
    }

    /// True iff `value` is a canonical workflow run lifecycle record ref
    /// under the product runtime governance root for the SAME stable id
    /// `expected_wp_id`: `<gov_root>/workflow_runs/<expected_wp_id>.json`.
    /// Substring spoofs and foreign WP ids are rejected.
    pub fn is_canonical_workflow_run_record_ref(
        &self,
        value: &str,
        expected_wp_id: &str,
    ) -> bool {
        if expected_wp_id.is_empty() || expected_wp_id.contains('/') {
            return false;
        }
        let normalized = normalize_display_like(value);
        if normalized.is_empty() || !normalized.ends_with(".json") {
            return false;
        }
        let prefix = normalize_display_like(&self.workflow_runs_dir_display());
        if !normalized.starts_with(&prefix) {
            return false;
        }
        let inner = &normalized[prefix.len()..];
        let expected = format!("{expected_wp_id}.json");
        inner == expected
    }

    /// True iff `value` is a canonical checkpoint record ref under the
    /// product runtime governance root: `<gov_root>/checkpoints/<id>.json`.
    /// Path-shape only (checkpoint id is governed by checkpoint lineage,
    /// not by record_id binding).
    pub fn is_canonical_checkpoint_record_ref(&self, value: &str) -> bool {
        let normalized = normalize_display_like(value);
        if normalized.is_empty() || !normalized.ends_with(".json") {
            return false;
        }
        let prefix = normalize_display_like(&self.checkpoints_dir_display());
        if !normalized.starts_with(&prefix) {
            return false;
        }
        let after_prefix = &normalized[prefix.len()..];
        !after_prefix.is_empty() && !after_prefix.contains('/')
    }

    pub fn is_runtime_artifact_display_path(&self, value: &str) -> bool {
        let normalized = normalize_display_like(value);
        if normalized.is_empty() {
            return false;
        }

        let governance_root = normalize_display_like(&self.governance_root_display());
        normalized.starts_with(&governance_root)
    }

    pub fn dcc_control_plane_dir(&self) -> PathBuf {
        self.governance_root.join("dcc")
    }

    pub fn dcc_control_plane_snapshot_path(&self) -> PathBuf {
        self.dcc_control_plane_dir().join("control_plane.json")
    }

    /// Build a DccGovernanceState by reading existing governance artifacts
    /// and projecting effective capability state from the registry.
    pub fn build_dcc_governance_state(
        &self,
        capability_registry: &crate::capabilities::CapabilityRegistry,
    ) -> DccGovernanceState {
        let pending_decisions = list_dir_stems(&self.governance_decisions_dir());
        let approval_decisions = read_approval_decisions(&self.governance_decisions_dir());
        let active_auto_signatures = list_dir_stems(&self.auto_signatures_dir());
        let governance_root = self.governance_root_display();
        let authority_refs = vec![
            self.governance_root_display(),
            self.task_board_display(),
        ];
        let mut effective_capability_axes: Vec<String> =
            capability_registry.axes().iter().cloned().collect();
        effective_capability_axes.sort();
        let mut effective_capability_ids: Vec<String> =
            capability_registry.ids().iter().cloned().collect();
        effective_capability_ids.sort();
        DccGovernanceState {
            governance_root,
            pending_decisions,
            approval_decisions,
            active_auto_signatures,
            effective_capability_axes,
            effective_capability_ids,
            authority_refs,
            evidence_refs: Vec::new(),
        }
    }

    pub fn invalid_runtime_authority_refs<'a>(&self, refs: &'a [String]) -> Vec<&'a str> {
        refs.iter()
            .filter_map(|value| {
                if self.is_runtime_artifact_display_path(value) {
                    None
                } else {
                    Some(value.as_str())
                }
            })
            .collect()
    }
}

fn absolutize(path: PathBuf) -> Result<PathBuf, io::Error> {
    if path.is_absolute() {
        return Ok(path);
    }
    Ok(std::env::current_dir()?.join(path))
}

fn has_parent_dir(path: &Path) -> bool {
    path.components()
        .any(|component| matches!(component, Component::ParentDir))
}

fn ensure_runtime_boundary(workspace_root: &Path, governance_root: &Path) -> Result<(), io::Error> {
    if !governance_root.starts_with(workspace_root) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "runtime governance root must stay under workspace root",
        ));
    }

    let relative = governance_root.strip_prefix(workspace_root).map_err(|_| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "invalid runtime governance root",
        )
    })?;

    for component in relative.components() {
        let Component::Normal(segment) = component else {
            continue;
        };
        let segment = segment.to_string_lossy();
        if segment.eq_ignore_ascii_case(".GOV") {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "runtime governance root must not use .GOV directory",
            ));
        }
        if segment.eq_ignore_ascii_case("docs") {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "runtime governance root must not use docs directory",
            ));
        }
    }

    Ok(())
}

fn display_path(workspace_root: &Path, path: &Path) -> String {
    let shown = path.strip_prefix(workspace_root).unwrap_or(path);
    shown
        .to_string_lossy()
        .replace('\\', "/")
        .trim_start_matches("./")
        .to_string()
}

fn ensure_trailing_slash(value: String) -> String {
    if value.ends_with('/') {
        value
    } else {
        format!("{value}/")
    }
}

fn safe_runtime_segment(value: &str) -> Result<String, io::Error> {
    let trimmed = value.trim();
    if trimmed.is_empty()
        || trimmed.contains('/')
        || trimmed.contains('\\')
        || trimmed.contains("..")
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "runtime path segment must be non-empty and must not contain path separators or '..'",
        ));
    }
    Ok(trimmed.to_string())
}

fn normalize_display_like(value: &str) -> String {
    value.trim().replace('\\', "/")
}

fn list_dir_stems(dir: &Path) -> Vec<String> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    entries
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_file())
        .filter_map(|e| e.path().file_stem().map(|s| s.to_string_lossy().into_owned()))
        .collect()
}

/// Read governance decision JSON files and project approval state.
/// Matches the GovernanceDecision schema from workflows.rs (decision_id,
/// gate_type, target_ref, decision, timestamp).
fn read_approval_decisions(dir: &Path) -> Vec<DccApprovalDecision> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut decisions = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_file() || path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        let Ok(content) = std::fs::read_to_string(&path) else {
            continue;
        };
        let Ok(val) = serde_json::from_str::<serde_json::Value>(&content) else {
            continue;
        };
        let decision_id = val.get("decision_id")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let gate_type = val.get("gate_type")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let target_ref = val.get("target_ref")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let decision = val.get("decision")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let timestamp = val.get("timestamp")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        if !decision_id.is_empty() {
            decisions.push(DccApprovalDecision {
                decision_id,
                gate_type,
                target_ref,
                decision,
                timestamp,
            });
        }
    }
    decisions.sort_by(|a, b| a.decision_id.cmp(&b.decision_id));
    decisions
}

#[cfg(test)]
mod tests {
    use super::RuntimeGovernancePaths;
    use std::path::PathBuf;
    use tempfile::tempdir;

    #[test]
    fn defaults_to_handshake_gov_under_workspace() -> std::io::Result<()> {
        let dir = tempdir()?;
        let workspace_root = dir.path().to_path_buf();
        let paths = RuntimeGovernancePaths::from_workspace_root(workspace_root.clone())?;
        assert_eq!(
            paths.governance_root(),
            workspace_root.join(".handshake").join("gov")
        );
        assert_eq!(
            paths.task_board_path(),
            workspace_root
                .join(".handshake")
                .join("gov")
                .join("TASK_BOARD.md")
        );
        assert_eq!(
            paths.wp_traceability_registry_path(),
            workspace_root
                .join(".handshake")
                .join("gov")
                .join("WP_TRACEABILITY_REGISTRY.md")
        );
        assert_eq!(
            paths.governance_decisions_dir(),
            workspace_root
                .join(".handshake")
                .join("gov")
                .join("governance_decisions")
        );
        assert_eq!(
            paths.work_packet_packet_path("WP-1"),
            workspace_root
                .join(".handshake")
                .join("gov")
                .join("work_packets")
                .join("WP-1")
                .join("packet.json")
        );
        assert_eq!(
            paths.micro_task_summary_path("WP-1", "MT-1"),
            workspace_root
                .join(".handshake")
                .join("gov")
                .join("micro_tasks")
                .join("WP-1")
                .join("MT-1")
                .join("summary.json")
        );
        assert_eq!(
            paths.task_board_index_path(),
            workspace_root
                .join(".handshake")
                .join("gov")
                .join("task_board")
                .join("index.json")
        );
        assert_eq!(
            paths.auto_signatures_dir(),
            workspace_root
                .join(".handshake")
                .join("gov")
                .join("auto_signatures")
        );
        assert_eq!(
            paths.activation_traceability_path("WP-1"),
            workspace_root
                .join(".handshake")
                .join("gov")
                .join("activation_traceability")
                .join("WP-1.json")
        );
        Ok(())
    }

    #[test]
    fn rejects_docs_runtime_root() {
        let err = RuntimeGovernancePaths::from_workspace_root_with_override(
            PathBuf::from("/tmp/hsk"),
            Some(PathBuf::from("docs")),
        )
        .expect_err("docs root must be rejected");
        assert!(err.to_string().contains("docs directory"));
    }

    #[test]
    fn rejects_dot_gov_runtime_root() {
        let err = RuntimeGovernancePaths::from_workspace_root_with_override(
            PathBuf::from("/tmp/hsk"),
            Some(PathBuf::from(".GOV")),
        )
        .expect_err(".GOV root must be rejected");
        assert!(err.to_string().contains(".GOV directory"));
    }

    // ── MT-003 proof: workflow transition matrix, queue automation, executor eligibility ──

    use crate::workflows::locus::types::{
        WorkflowStateFamily, WorkflowQueueReasonCode, ExecutorKind,
        transition_rules_for_family, transition_rule_ids_for_family,
        queue_automation_rules, queue_automation_rule_ids_for_reason,
        executor_eligibility_policies, executor_eligibility_policy_ids_for_family,
        is_local_small_model_eligible,
    };

    #[test]
    fn transition_rules_cover_all_families_and_archived_is_terminal() {
        let all_families = [
            WorkflowStateFamily::Intake, WorkflowStateFamily::Ready,
            WorkflowStateFamily::Active, WorkflowStateFamily::Waiting,
            WorkflowStateFamily::Review, WorkflowStateFamily::Approval,
            WorkflowStateFamily::Validation, WorkflowStateFamily::Blocked,
            WorkflowStateFamily::Done, WorkflowStateFamily::Canceled,
            WorkflowStateFamily::Archived,
        ];

        for family in &all_families {
            let rules = transition_rules_for_family(*family);
            if *family == WorkflowStateFamily::Archived {
                assert!(
                    rules.is_empty(),
                    "Archived must be terminal with no outbound transitions"
                );
            } else {
                assert!(
                    !rules.is_empty(),
                    "Non-terminal family {:?} must have at least one transition rule",
                    family
                );
            }

            for rule in &rules {
                assert_eq!(rule.from_family, *family);
                assert_ne!(
                    rule.from_family, rule.to_family,
                    "self-transitions are not valid: {}",
                    rule.rule_id
                );
                assert!(
                    rule.rule_id.starts_with("transition:"),
                    "rule_id must use transition: prefix: {}",
                    rule.rule_id
                );
            }
        }

        // Verify id resolution matches
        let ids = transition_rule_ids_for_family(WorkflowStateFamily::Active);
        assert!(ids.len() >= 3, "Active must have transitions to Waiting, Review, Blocked, Done, Canceled");
        assert!(ids.iter().all(|id| id.starts_with("transition:active_")));
    }

    #[test]
    fn queue_automation_rules_cover_canonical_triggers() {
        let rules = queue_automation_rules();
        assert!(rules.len() >= 4, "must cover dependency, mailbox, validation, retry triggers");

        let rule_ids: Vec<&str> = rules.iter().map(|r| r.rule_id.as_str()).collect();
        assert!(rule_ids.contains(&"automation:dependency_cleared"));
        assert!(rule_ids.contains(&"automation:mailbox_response_received"));
        assert!(rule_ids.contains(&"automation:validation_passed"));
        assert!(rule_ids.contains(&"automation:retry_timer_elapsed"));

        for rule in &rules {
            assert!(
                rule.rule_id.starts_with("automation:"),
                "rule_id must use automation: prefix: {}",
                rule.rule_id
            );
            assert_ne!(
                rule.from_reason, rule.to_reason,
                "automation rule must change reason: {}",
                rule.rule_id
            );
        }

        // Verify reason-based resolution
        let mailbox_rules = queue_automation_rule_ids_for_reason(
            WorkflowQueueReasonCode::MailboxResponseWait,
        );
        assert_eq!(mailbox_rules, vec!["automation:mailbox_response_received"]);

        let dependency_rules = queue_automation_rule_ids_for_reason(
            WorkflowQueueReasonCode::DependencyWait,
        );
        assert_eq!(dependency_rules, vec!["automation:dependency_cleared"]);

        // ReadyForHuman has no outbound automation rule
        let ready_rules = queue_automation_rule_ids_for_reason(
            WorkflowQueueReasonCode::ReadyForHuman,
        );
        assert!(ready_rules.is_empty());
    }

    #[test]
    fn executor_eligibility_policies_cover_all_executor_kinds() {
        let policies = executor_eligibility_policies();

        let kinds: Vec<ExecutorKind> = policies.iter().map(|p| p.executor_kind).collect();
        assert!(kinds.contains(&ExecutorKind::Operator));
        assert!(kinds.contains(&ExecutorKind::LocalSmallModel));
        assert!(kinds.contains(&ExecutorKind::CloudModel));
        assert!(kinds.contains(&ExecutorKind::WorkflowEngine));
        assert!(kinds.contains(&ExecutorKind::Reviewer));
        assert!(kinds.contains(&ExecutorKind::Governance));

        for policy in &policies {
            assert!(
                policy.policy_id.starts_with("eligibility:"),
                "policy_id must use eligibility: prefix: {}",
                policy.policy_id
            );
            assert!(
                !policy.eligible_families.is_empty(),
                "policy must have at least one eligible family: {}",
                policy.policy_id
            );
        }

        // Verify family-based resolution
        let ready_policies = executor_eligibility_policy_ids_for_family(WorkflowStateFamily::Ready);
        assert!(ready_policies.contains(&"eligibility:operator".to_string()));
        assert!(ready_policies.contains(&"eligibility:local_small_model".to_string()));
        assert!(ready_policies.contains(&"eligibility:cloud_model".to_string()));
        assert!(ready_policies.contains(&"eligibility:workflow_engine".to_string()));

        // Archived should only have operator and workflow_engine
        let archived_policies = executor_eligibility_policy_ids_for_family(WorkflowStateFamily::Archived);
        assert!(archived_policies.contains(&"eligibility:operator".to_string()));
        assert!(archived_policies.contains(&"eligibility:workflow_engine".to_string()));
        assert!(!archived_policies.contains(&"eligibility:local_small_model".to_string()));
    }

    #[test]
    fn local_small_model_requires_ready_family_and_compact_summary() {
        // Per v02.172: local-small-model eligibility MUST require
        // a ready-family state AND a compact summary being available.
        assert!(
            is_local_small_model_eligible(WorkflowStateFamily::Ready, true),
            "ready + compact_summary = eligible"
        );
        assert!(
            !is_local_small_model_eligible(WorkflowStateFamily::Ready, false),
            "ready without compact_summary = not eligible"
        );
        assert!(
            !is_local_small_model_eligible(WorkflowStateFamily::Active, true),
            "active + compact_summary = not eligible (wrong family)"
        );
        assert!(
            !is_local_small_model_eligible(WorkflowStateFamily::Intake, true),
            "intake + compact_summary = not eligible"
        );
        assert!(
            !is_local_small_model_eligible(WorkflowStateFamily::Blocked, false),
            "blocked without compact_summary = not eligible"
        );

        // Verify the policy registry matches
        let lsm_policy = executor_eligibility_policies()
            .into_iter()
            .find(|p| p.executor_kind == ExecutorKind::LocalSmallModel)
            .expect("local_small_model policy must exist");
        assert!(
            lsm_policy.requires_compact_summary,
            "local_small_model policy must require compact summary"
        );
        assert_eq!(
            lsm_policy.eligible_families,
            vec![WorkflowStateFamily::Ready],
            "local_small_model policy must only allow Ready family"
        );
    }

    #[test]
    fn transition_and_eligibility_ids_are_portable_across_families() {
        // Verify the same function works for any family — proving portability
        // across WP, MT, TaskBoard, and Mailbox surfaces.
        let all_families = [
            WorkflowStateFamily::Intake, WorkflowStateFamily::Ready,
            WorkflowStateFamily::Active, WorkflowStateFamily::Waiting,
            WorkflowStateFamily::Review, WorkflowStateFamily::Approval,
            WorkflowStateFamily::Validation, WorkflowStateFamily::Blocked,
            WorkflowStateFamily::Done, WorkflowStateFamily::Canceled,
            WorkflowStateFamily::Archived,
        ];

        for family in &all_families {
            let transition_ids = transition_rule_ids_for_family(*family);
            let eligibility_ids = executor_eligibility_policy_ids_for_family(*family);

            // All ids must be stable strings, not empty
            for id in &transition_ids {
                assert!(!id.is_empty());
                assert!(id.starts_with("transition:"));
            }
            for id in &eligibility_ids {
                assert!(!id.is_empty());
                assert!(id.starts_with("eligibility:"));
            }

            // Every family must have at least operator eligibility
            assert!(
                eligibility_ids.contains(&"eligibility:operator".to_string()),
                "{:?} must be eligible for operator",
                family
            );
        }
    }

    #[test]
    fn dcc_control_plane_projection_keeps_stable_ids() {
        use super::*;

        let wp_id = "WP-1-Test";
        let task_board_id = "tb-001";
        let workflow_run_id = "wfr-abc-123";
        let session_id = "sess-xyz-789";

        let snapshot = DccControlPlaneSnapshot {
            schema_id: DCC_CONTROL_PLANE_SCHEMA_ID.to_string(),
            schema_version: DCC_CONTROL_PLANE_SCHEMA_VERSION.to_string(),
            snapshot_id: "snap-001".to_string(),
            generated_at: "2026-04-11T00:00:00Z".to_string(),
            work_state: DccWorkState {
                task_board_id: task_board_id.to_string(),
                entries: vec![],
                active_workflow_summaries: vec![DccWorkflowSummary {
                    work_packet_id: wp_id.to_string(),
                    workflow_run_id: Some(workflow_run_id.to_string()),
                    state_family: locus::WorkflowStateFamily::Active,
                    queue_reason_code: locus::WorkflowQueueReasonCode::ReadyForHuman,
                    allowed_action_ids: vec![],
                    model_session_id: Some(session_id.to_string()),
                    summary_ref: None,
                    micro_task_summary: Some(DccMicroTaskSummary {
                        total: 3,
                        completed: 1,
                        failed: 0,
                        in_progress: 1,
                        blocked: 0,
                        mt_ids: vec!["MT-001".to_string(), "MT-002".to_string(), "MT-003".to_string()],
                    }),
                    gate_state: Some(DccGateState {
                        pre_work: "pass".to_string(),
                        post_work: "pending".to_string(),
                    }),
                    closeout_badge: None,
                    authority_refs: vec![".handshake/gov/work_packets/WP-1-Test/".to_string()],
                    evidence_refs: vec![".handshake/gov/task_board/index.json".to_string()],
                }],
                freshness: "2026-04-11T00:00:00Z".to_string(),
                ready_queue: vec![],
            },
            session_state: DccSessionState {
                bindings: vec![DccSessionBinding {
                    session_id: session_id.to_string(),
                    worktree_dir: Some("/tmp/wt".to_string()),
                    role: "coder".to_string(),
                    state: "active".to_string(),
                    bound_work_packet_id: Some(wp_id.to_string()),
                    model_id: Some("claude-opus-4-6".to_string()),
                    backend: Some("anthropic".to_string()),
                    bound_micro_task_id: Some("MT-002".to_string()),
                }],
            },
            governance_state: DccGovernanceState {
                governance_root: ".handshake/gov/".to_string(),
                pending_decisions: vec![],
                approval_decisions: vec![DccApprovalDecision {
                    decision_id: "dec-001".to_string(),
                    gate_type: "pre_work".to_string(),
                    target_ref: "WP-1-Test".to_string(),
                    decision: "approve".to_string(),
                    timestamp: "2026-04-11T00:00:00Z".to_string(),
                }],
                active_auto_signatures: vec![],
                effective_capability_axes: vec!["fs".to_string(), "locus".to_string()],
                effective_capability_ids: vec!["doc.summarize".to_string()],
                authority_refs: vec![".handshake/gov/".to_string()],
                evidence_refs: vec![],
            },
            collaboration_state: DccCollaborationState {
                active_threads: vec![DccMailboxThreadSummary {
                    thread_id: "thread-001".to_string(),
                    work_packet_id: Some(wp_id.to_string()),
                    participants: vec!["orchestrator".to_string(), "coder".to_string()],
                    message_count: 2,
                    latest_message_type: Some("clarification_request".to_string()),
                    latest_from_role: Some("orchestrator".to_string()),
                    created_at: "2026-04-11T00:00:00Z".to_string(),
                    closed_at: None,
                    evidence_refs: vec![".handshake/gov/ROLE_MAILBOX/threads/thread-001.jsonl".to_string()],
                    linked_work_ids: vec![wp_id.to_string()],
                }],
                pending_wait_reasons: vec![DccWaitReason {
                    thread_id: "thread-001".to_string(),
                    work_packet_id: Some(wp_id.to_string()),
                    waiting_for_role: "coder".to_string(),
                    expected_response: "clarification_response".to_string(),
                    wait_since: "2026-04-11T00:00:00Z".to_string(),
                }],
                mailbox_summary: DccMailboxSummary {
                    total_threads: 1,
                    active_threads: 1,
                    total_messages: 2,
                },
            },
        };

        // Round-trip through serde and verify stable IDs survive
        let json = serde_json::to_value(&snapshot).expect("serialize");
        assert_eq!(json["schema_id"], DCC_CONTROL_PLANE_SCHEMA_ID);
        assert_eq!(json["work_state"]["task_board_id"], task_board_id);
        assert_eq!(
            json["work_state"]["active_workflow_summaries"][0]["work_packet_id"],
            wp_id
        );
        assert_eq!(
            json["work_state"]["active_workflow_summaries"][0]["workflow_run_id"],
            workflow_run_id
        );
        assert_eq!(
            json["work_state"]["active_workflow_summaries"][0]["model_session_id"],
            session_id
        );
        assert_eq!(
            json["work_state"]["active_workflow_summaries"][0]["state_family"],
            "active"
        );
        assert_eq!(
            json["work_state"]["active_workflow_summaries"][0]["authority_refs"][0],
            ".handshake/gov/work_packets/WP-1-Test/"
        );
        assert_eq!(json["session_state"]["bindings"][0]["session_id"], session_id);
        assert_eq!(
            json["session_state"]["bindings"][0]["bound_work_packet_id"],
            wp_id
        );
        assert_eq!(
            json["governance_state"]["governance_root"],
            ".handshake/gov/"
        );
        assert_eq!(
            json["governance_state"]["effective_capability_axes"],
            serde_json::json!(["fs", "locus"])
        );
        assert_eq!(
            json["governance_state"]["effective_capability_ids"],
            serde_json::json!(["doc.summarize"])
        );

        // MT-002: micro-task summary round-trip
        let mt_summary = &json["work_state"]["active_workflow_summaries"][0]["micro_task_summary"];
        assert_eq!(mt_summary["total"], 3);
        assert_eq!(mt_summary["completed"], 1);
        assert_eq!(mt_summary["in_progress"], 1);
        assert_eq!(mt_summary["mt_ids"], serde_json::json!(["MT-001", "MT-002", "MT-003"]));

        // MT-002: gate state round-trip
        let gate = &json["work_state"]["active_workflow_summaries"][0]["gate_state"];
        assert_eq!(gate["pre_work"], "pass");
        assert_eq!(gate["post_work"], "pending");

        // MT-002: session occupancy fields
        assert_eq!(json["session_state"]["bindings"][0]["model_id"], "claude-opus-4-6");
        assert_eq!(json["session_state"]["bindings"][0]["backend"], "anthropic");
        assert_eq!(json["session_state"]["bindings"][0]["bound_micro_task_id"], "MT-002");

        // MT-001 fix: approval decision state round-trip
        let ad = &json["governance_state"]["approval_decisions"][0];
        assert_eq!(ad["decision_id"], "dec-001");
        assert_eq!(ad["gate_type"], "pre_work");
        assert_eq!(ad["target_ref"], "WP-1-Test");
        assert_eq!(ad["decision"], "approve");

        // MT-003: collaboration state round-trip (must check JSON before from_value consumes it)
        let collab = &json["collaboration_state"];
        assert_eq!(collab["active_threads"][0]["thread_id"], "thread-001");
        assert_eq!(collab["active_threads"][0]["work_packet_id"], wp_id);
        assert_eq!(collab["active_threads"][0]["latest_message_type"], "clarification_request");
        assert_eq!(collab["active_threads"][0]["linked_work_ids"][0], wp_id);
        assert_eq!(collab["pending_wait_reasons"][0]["expected_response"], "clarification_response");
        assert_eq!(collab["pending_wait_reasons"][0]["waiting_for_role"], "coder");
        assert_eq!(collab["mailbox_summary"]["total_threads"], 1);
        assert_eq!(collab["mailbox_summary"]["active_threads"], 1);
        assert_eq!(collab["mailbox_summary"]["total_messages"], 2);

        // Deserialize back and verify structural equality
        let deserialized: DccControlPlaneSnapshot =
            serde_json::from_value(json).expect("deserialize");
        assert_eq!(deserialized.snapshot_id, "snap-001");
        assert_eq!(deserialized.work_state.task_board_id, task_board_id);
        assert_eq!(
            deserialized.work_state.active_workflow_summaries[0].work_packet_id,
            wp_id
        );
        assert_eq!(deserialized.session_state.bindings[0].session_id, session_id);
        // MT-002: micro-task summary survives deserialization
        let mt = deserialized.work_state.active_workflow_summaries[0]
            .micro_task_summary.as_ref().expect("micro_task_summary present");
        assert_eq!(mt.total, 3);
        assert_eq!(mt.completed, 1);
        assert_eq!(mt.mt_ids.len(), 3);
        // MT-002: gate state survives deserialization
        let gs = deserialized.work_state.active_workflow_summaries[0]
            .gate_state.as_ref().expect("gate_state present");
        assert_eq!(gs.pre_work, "pass");
        assert_eq!(gs.post_work, "pending");
        // MT-002: session occupancy survives deserialization
        assert_eq!(deserialized.session_state.bindings[0].model_id.as_deref(), Some("claude-opus-4-6"));
        assert_eq!(deserialized.session_state.bindings[0].bound_micro_task_id.as_deref(), Some("MT-002"));
        // MT-001 fix: approval decisions survive deserialization
        assert_eq!(deserialized.governance_state.approval_decisions.len(), 1);
        assert_eq!(deserialized.governance_state.approval_decisions[0].decision_id, "dec-001");
        assert_eq!(deserialized.governance_state.approval_decisions[0].decision, "approve");

        // MT-003: collaboration state survives deserialization
        assert_eq!(deserialized.collaboration_state.active_threads.len(), 1);
        assert_eq!(deserialized.collaboration_state.active_threads[0].thread_id, "thread-001");
        assert_eq!(deserialized.collaboration_state.active_threads[0].work_packet_id.as_deref(), Some(wp_id));
        assert_eq!(deserialized.collaboration_state.pending_wait_reasons.len(), 1);
        assert_eq!(deserialized.collaboration_state.pending_wait_reasons[0].expected_response, "clarification_response");
        assert_eq!(deserialized.collaboration_state.mailbox_summary.total_threads, 1);
        assert_eq!(deserialized.collaboration_state.mailbox_summary.active_threads, 1);
    }
}
