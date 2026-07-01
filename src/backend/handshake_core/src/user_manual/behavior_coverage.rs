use std::collections::BTreeSet;

use crate::storage::StorageError;
use crate::swarm_orchestration::model_lane::ModelLaneSchemaRegistryRow;

use super::store::{UserManualPage, UserManualToolEntry};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosticTierPosture {
    Wired,
    NotApplicableWithReason,
    DeferredWithReason,
}

impl DiagnosticTierPosture {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Wired => "WIRED",
            Self::NotApplicableWithReason => "NOT_APPLICABLE-with-reason",
            Self::DeferredWithReason => "DEFERRED-with-reason",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BehaviorCoverageRow {
    pub behavior_id: &'static str,
    pub schema_id: Option<&'static str>,
    pub event_family: &'static str,
    pub runtime_surface_id: &'static str,
    pub user_manual_slug: &'static str,
    pub tool_id: &'static str,
    pub eventledger_flight_recorder_path: &'static str,
    pub internal_diagnostics_posture: DiagnosticTierPosture,
    pub palmistry_posture: DiagnosticTierPosture,
    pub deferred_reason: Option<&'static str>,
    pub follow_up_ref: Option<&'static str>,
}

impl BehaviorCoverageRow {
    pub fn schema_or_event_family(&self) -> &'static str {
        self.schema_id.unwrap_or(self.event_family)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BehaviorCoverageError {
    pub behavior_id: &'static str,
    pub reason: String,
}

impl std::fmt::Display for BehaviorCoverageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.behavior_id, self.reason)
    }
}

impl std::error::Error for BehaviorCoverageError {}

impl From<StorageError> for BehaviorCoverageError {
    fn from(value: StorageError) -> Self {
        Self {
            behavior_id: "user_manual_storage",
            reason: value.to_string(),
        }
    }
}

pub fn model_lane_behavior_coverage_matrix() -> Vec<BehaviorCoverageRow> {
    vec![
        BehaviorCoverageRow {
            behavior_id: "wp1.model_lane.run",
            schema_id: Some("hsk.model_lane_run@1"),
            event_family: "model_lane_run",
            runtime_surface_id: "ModelLaneStore::record_run",
            user_manual_slug: "model-lane-schema",
            tool_id: "model_lane_schema_pg_tests",
            eventledger_flight_recorder_path: "kernel_event_ledger:model_lane_run",
            internal_diagnostics_posture: DiagnosticTierPosture::Wired,
            palmistry_posture: DiagnosticTierPosture::DeferredWithReason,
            deferred_reason: Some("Palmistry watcher is integrated from a separate worktree."),
            follow_up_ref: Some("palmistry://wp1/model-lane/run"),
        },
        BehaviorCoverageRow {
            behavior_id: "wp1.model_lane.launch",
            schema_id: Some("hsk.model_lane@1"),
            event_family: "model_lane",
            runtime_surface_id: "DexterityLaunchAdapterRegistry::normalize",
            user_manual_slug: "model-lane-launch-adapters",
            tool_id: "model_lane_launch_tests",
            eventledger_flight_recorder_path: "kernel_event_ledger:model_lane",
            internal_diagnostics_posture: DiagnosticTierPosture::Wired,
            palmistry_posture: DiagnosticTierPosture::DeferredWithReason,
            deferred_reason: Some("Palmistry watcher is integrated from a separate worktree."),
            follow_up_ref: Some("palmistry://wp1/model-lane/launch"),
        },
        BehaviorCoverageRow {
            behavior_id: "wp1.model_lane.message",
            schema_id: Some("hsk.model_lane_message@1"),
            event_family: "model_lane_message",
            runtime_surface_id: "ModelLaneStore::record_message",
            user_manual_slug: "model-lane-schema",
            tool_id: "model_lane_schema_pg_tests",
            eventledger_flight_recorder_path: "kernel_event_ledger:model_lane_message",
            internal_diagnostics_posture: DiagnosticTierPosture::Wired,
            palmistry_posture: DiagnosticTierPosture::DeferredWithReason,
            deferred_reason: Some("Palmistry watcher is integrated from a separate worktree."),
            follow_up_ref: Some("palmistry://wp1/model-lane/message"),
        },
        BehaviorCoverageRow {
            behavior_id: "wp1.model_lane.terminal",
            schema_id: Some("hsk.model_lane_terminal@1"),
            event_family: "model_lane_terminal",
            runtime_surface_id: "ModelLaneStore::record_lane_terminal_status",
            user_manual_slug: "model-lane-launch-adapters",
            tool_id: "model_lane_launch_tests",
            eventledger_flight_recorder_path: "kernel_event_ledger:model_lane_terminal",
            internal_diagnostics_posture: DiagnosticTierPosture::Wired,
            palmistry_posture: DiagnosticTierPosture::DeferredWithReason,
            deferred_reason: Some("Palmistry watcher is integrated from a separate worktree."),
            follow_up_ref: Some("palmistry://wp1/model-lane/terminal"),
        },
        BehaviorCoverageRow {
            behavior_id: "wp1.model_lane.promotion",
            schema_id: Some("hsk.model_lane_promotion_decision@1"),
            event_family: "model_lane_promotion_decision",
            runtime_surface_id: "ModelLaneStore::record_promotion_decision",
            user_manual_slug: "model-lane-promotion",
            tool_id: "model_lane_promotion_pg_tests",
            eventledger_flight_recorder_path: "kernel_event_ledger:model_lane_promotion_decision",
            internal_diagnostics_posture: DiagnosticTierPosture::Wired,
            palmistry_posture: DiagnosticTierPosture::DeferredWithReason,
            deferred_reason: Some("Palmistry watcher is integrated from a separate worktree."),
            follow_up_ref: Some("palmistry://wp1/model-lane/promotion"),
        },
        BehaviorCoverageRow {
            behavior_id: "wp1.model_lane.context_bundle_artifact",
            schema_id: Some("hsk.model_lane_context_bundle_artifact@1"),
            event_family: "model_lane_context_bundle_artifact",
            runtime_surface_id: "ModelLaneStore::record_context_bundle_artifact_binding",
            user_manual_slug: "model-lane-context-bundle-handoff",
            tool_id: "model_lane_context_bundle_pg_tests",
            eventledger_flight_recorder_path:
                "kernel_event_ledger:model_lane_context_bundle_artifact",
            internal_diagnostics_posture: DiagnosticTierPosture::Wired,
            palmistry_posture: DiagnosticTierPosture::DeferredWithReason,
            deferred_reason: Some("Palmistry watcher is integrated from a separate worktree."),
            follow_up_ref: Some("palmistry://wp1/model-lane/context-bundle-artifact"),
        },
        BehaviorCoverageRow {
            behavior_id: "wp1.model_lane.context_bundle",
            schema_id: Some("hsk.model_lane_context_bundle_handoff@1"),
            event_family: "model_lane_context_bundle_handoff",
            runtime_surface_id: "ModelLaneStore::record_context_bundle_handoff",
            user_manual_slug: "model-lane-context-bundle-handoff",
            tool_id: "model_lane_context_bundle_pg_tests",
            eventledger_flight_recorder_path:
                "kernel_event_ledger:model_lane_context_bundle_handoff",
            internal_diagnostics_posture: DiagnosticTierPosture::Wired,
            palmistry_posture: DiagnosticTierPosture::DeferredWithReason,
            deferred_reason: Some("Palmistry watcher is integrated from a separate worktree."),
            follow_up_ref: Some("palmistry://wp1/model-lane/context-bundle"),
        },
        BehaviorCoverageRow {
            behavior_id: "wp1.model_lane.cloud_projection_plan",
            schema_id: Some("hsk.model_lane_cloud_projection_plan@1"),
            event_family: "model_lane_cloud_projection_plan",
            runtime_surface_id: "ModelLaneStore::record_cloud_projection_plan",
            user_manual_slug: "model-lane-cloud-projection-consent",
            tool_id: "cloud_model_lane_policy_pg_tests",
            eventledger_flight_recorder_path:
                "kernel_event_ledger:model_lane_cloud_projection_plan",
            internal_diagnostics_posture: DiagnosticTierPosture::Wired,
            palmistry_posture: DiagnosticTierPosture::DeferredWithReason,
            deferred_reason: Some("Palmistry watcher is integrated from a separate worktree."),
            follow_up_ref: Some("palmistry://wp1/model-lane/cloud-projection-plan"),
        },
        BehaviorCoverageRow {
            behavior_id: "wp1.model_lane.cloud_consent",
            schema_id: Some("hsk.model_lane_cloud_consent_receipt@1"),
            event_family: "model_lane_cloud_consent_receipt",
            runtime_surface_id: "ModelLaneStore::record_cloud_consent_receipt",
            user_manual_slug: "model-lane-cloud-projection-consent",
            tool_id: "cloud_model_lane_policy_pg_tests",
            eventledger_flight_recorder_path:
                "kernel_event_ledger:model_lane_cloud_consent_receipt",
            internal_diagnostics_posture: DiagnosticTierPosture::Wired,
            palmistry_posture: DiagnosticTierPosture::DeferredWithReason,
            deferred_reason: Some("Palmistry watcher is integrated from a separate worktree."),
            follow_up_ref: Some("palmistry://wp1/model-lane/cloud-consent"),
        },
        BehaviorCoverageRow {
            behavior_id: "wp1.model_lane.cloud_consent_denial",
            schema_id: Some("hsk.model_lane_cloud_consent_denial@1"),
            event_family: "model_lane_cloud_consent_denial",
            runtime_surface_id: "ModelLaneStore::record_cloud_consent_denial",
            user_manual_slug: "model-lane-cloud-projection-consent",
            tool_id: "cloud_model_lane_policy_pg_tests",
            eventledger_flight_recorder_path: "kernel_event_ledger:model_lane_cloud_consent_denial",
            internal_diagnostics_posture: DiagnosticTierPosture::Wired,
            palmistry_posture: DiagnosticTierPosture::DeferredWithReason,
            deferred_reason: Some("Palmistry watcher is integrated from a separate worktree."),
            follow_up_ref: Some("palmistry://wp1/model-lane/cloud-consent-denial"),
        },
        BehaviorCoverageRow {
            behavior_id: "wp1.model_lane.recovery",
            schema_id: Some("hsk.model_lane_recovery_checkpoint@1"),
            event_family: "model_lane_recovery_checkpoint",
            runtime_surface_id: "ModelLaneStore::recover_run_after_restart",
            user_manual_slug: "model-lane-recovery",
            tool_id: "model_lane_recovery_pg_tests",
            eventledger_flight_recorder_path: "kernel_event_ledger:model_lane_recovery_checkpoint",
            internal_diagnostics_posture: DiagnosticTierPosture::Wired,
            palmistry_posture: DiagnosticTierPosture::DeferredWithReason,
            deferred_reason: Some("Palmistry watcher is integrated from a separate worktree."),
            follow_up_ref: Some("palmistry://wp1/model-lane/recovery"),
        },
        BehaviorCoverageRow {
            behavior_id: "wp1.model_lane.recovery_event",
            schema_id: Some("hsk.model_lane_recovery_event@1"),
            event_family: "model_lane_recovery_event",
            runtime_surface_id: "ModelLaneStore::record_recovery_event",
            user_manual_slug: "model-lane-recovery",
            tool_id: "model_lane_recovery_pg_tests",
            eventledger_flight_recorder_path: "kernel_event_ledger:model_lane_recovery_event",
            internal_diagnostics_posture: DiagnosticTierPosture::Wired,
            palmistry_posture: DiagnosticTierPosture::DeferredWithReason,
            deferred_reason: Some("Palmistry watcher is integrated from a separate worktree."),
            follow_up_ref: Some("palmistry://wp1/model-lane/recovery-event"),
        },
        BehaviorCoverageRow {
            behavior_id: "wp1.model_lane.lease",
            schema_id: Some("hsk.model_lane_lease@1"),
            event_family: "model_lane_lease",
            runtime_surface_id: "ModelLaneStore::record_lane_lease",
            user_manual_slug: "model-lane-recovery",
            tool_id: "model_lane_recovery_pg_tests",
            eventledger_flight_recorder_path: "kernel_event_ledger:model_lane_lease",
            internal_diagnostics_posture: DiagnosticTierPosture::Wired,
            palmistry_posture: DiagnosticTierPosture::DeferredWithReason,
            deferred_reason: Some("Palmistry watcher is integrated from a separate worktree."),
            follow_up_ref: Some("palmistry://wp1/model-lane/lease"),
        },
        BehaviorCoverageRow {
            behavior_id: "wp1.model_lane.diagnostics",
            schema_id: Some("hsk.model_lane_diagnostic_tier@1"),
            event_family: "model_lane_diagnostic_tier",
            runtime_surface_id: "native_swarm_lane_diagnostics",
            user_manual_slug: "model-lane-diagnostics",
            tool_id: "swarm_lane_diagnostics_runtime_proof",
            eventledger_flight_recorder_path: "kernel_event_ledger:model_lane_diagnostic_tier",
            internal_diagnostics_posture: DiagnosticTierPosture::Wired,
            palmistry_posture: DiagnosticTierPosture::DeferredWithReason,
            deferred_reason: Some("Palmistry watcher is integrated from a separate worktree."),
            follow_up_ref: Some("palmistry://wp1/model-lane/diagnostics"),
        },
        BehaviorCoverageRow {
            behavior_id: "wp1.model_lane.mixed_validation",
            schema_id: Some("hsk.model_lane_mt_runtime_status@1"),
            event_family: "model_lane_mt_runtime_status",
            runtime_surface_id: "mixed_model_lane_integration_pg_tests",
            user_manual_slug: "model-lane-validation-harness",
            tool_id: "mixed_model_lane_integration_pg_tests",
            eventledger_flight_recorder_path: "kernel_event_ledger:model_lane_mt_runtime_status",
            internal_diagnostics_posture: DiagnosticTierPosture::Wired,
            palmistry_posture: DiagnosticTierPosture::DeferredWithReason,
            deferred_reason: Some("Palmistry watcher is integrated from a separate worktree."),
            follow_up_ref: Some("palmistry://wp1/model-lane/mixed-validation"),
        },
    ]
}

pub fn verify_model_lane_behavior_coverage(
    rows: &[BehaviorCoverageRow],
    schema_registry: &[ModelLaneSchemaRegistryRow],
    pages: &[UserManualPage],
    tools: &[UserManualToolEntry],
) -> Result<(), Vec<BehaviorCoverageError>> {
    let schema_ids = schema_registry
        .iter()
        .map(|row| row.schema_id.as_str())
        .collect::<BTreeSet<_>>();
    let page_slugs = pages
        .iter()
        .map(|page| page.slug.as_str())
        .collect::<BTreeSet<_>>();
    let tool_ids = tools
        .iter()
        .map(|tool| tool.tool_id.as_str())
        .collect::<BTreeSet<_>>();

    let mut errors = Vec::new();
    let coverage_schema_ids = rows
        .iter()
        .filter_map(|row| row.schema_id)
        .collect::<BTreeSet<_>>();
    for schema_id in &schema_ids {
        if !coverage_schema_ids.contains(schema_id) {
            errors.push(BehaviorCoverageError {
                behavior_id: "model_lane_schema_registry",
                reason: format!("schema_id {schema_id} lacks UserManual behavior coverage row"),
            });
        }
    }
    for row in rows {
        if let Some(schema_id) = row.schema_id {
            if !schema_ids.contains(schema_id) {
                errors.push(BehaviorCoverageError {
                    behavior_id: row.behavior_id,
                    reason: format!("schema_id {schema_id} missing from ModelLane schema registry"),
                });
            }
        }
        if !page_slugs.contains(row.user_manual_slug) {
            errors.push(BehaviorCoverageError {
                behavior_id: row.behavior_id,
                reason: format!("UserManual page {} missing", row.user_manual_slug),
            });
        }
        if !tool_ids.contains(row.tool_id) {
            errors.push(BehaviorCoverageError {
                behavior_id: row.behavior_id,
                reason: format!("UserManual tool {} missing", row.tool_id),
            });
        }
        if row.eventledger_flight_recorder_path.trim().is_empty()
            || !row
                .eventledger_flight_recorder_path
                .contains("kernel_event_ledger")
        {
            errors.push(BehaviorCoverageError {
                behavior_id: row.behavior_id,
                reason: "EventLedger/FlightRecorder evidence path missing".to_owned(),
            });
        }
        if row.internal_diagnostics_posture != DiagnosticTierPosture::Wired {
            errors.push(BehaviorCoverageError {
                behavior_id: row.behavior_id,
                reason: format!(
                    "internal_diagnostics posture must be WIRED, got {}",
                    row.internal_diagnostics_posture.as_str()
                ),
            });
        }
        if row.palmistry_posture == DiagnosticTierPosture::DeferredWithReason
            && (row.deferred_reason.is_none() || row.follow_up_ref.is_none())
        {
            errors.push(BehaviorCoverageError {
                behavior_id: row.behavior_id,
                reason: "Palmistry DEFERRED-with-reason requires deferred_reason and follow_up_ref"
                    .to_owned(),
            });
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}
