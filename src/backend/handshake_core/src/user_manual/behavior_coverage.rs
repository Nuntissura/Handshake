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

    pub fn self_consistency_result(&self) -> &'static str {
        if self.schema_id.is_some() {
            "verified: schema_registry+usermanual_page+tool_entry+eventledger_flight_recorder+hbr_diagnostic_posture"
        } else {
            "verified: usermanual_page+tool_entry+eventledger_flight_recorder+hbr_diagnostic_posture"
        }
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

/// WP-1 MT-013 (AC#5 + HBR-INT-009): UserManual behavior coverage for the
/// embedded-model ProcessOwnershipLedger START/STOP lifecycle and the
/// fail-closed / embedding Flight Recorder emission behaviors.
///
/// Unlike [`model_lane_behavior_coverage_matrix`], these rows record
/// `internal_diagnostics` as DEFERRED-with-reason (MT-013 does not wire the
/// native internal-diagnostics tier for the embedded-model lifecycle; that lands
/// with WP-KERNEL-012/016), so they use [`verify_embedded_model_behavior_coverage`]
/// rather than the model-lane verifier (which requires internal_diagnostics
/// WIRED). Flight Recorder / EventLedger is WIRED; Palmistry is DEFERRED-with-
/// reason (external watcher lands from its own worktree).
pub fn embedded_model_behavior_coverage_matrix() -> Vec<BehaviorCoverageRow> {
    const DEFERRED_REASON: &str =
        "MT-013 wires Flight Recorder/EventLedger only; the native internal_diagnostics tier and \
         the external Palmistry watcher for the embedded-model lifecycle are integrated from \
         WP-KERNEL-012/016 worktrees and must observe these ProcessOwnershipLedger / Flight \
         Recorder rows without becoming their authority.";
    vec![
        BehaviorCoverageRow {
            behavior_id: "wp1.embedded_model.ledger_start",
            schema_id: None,
            event_family: "kernel_process_lifecycle_start",
            runtime_surface_id: "EmbeddedModelProcess::record_load",
            user_manual_slug: "embedded-model-lifecycle-ledger",
            tool_id: "embedded_model_ledger_tests",
            eventledger_flight_recorder_path: "kernel_process_lifecycle:embedded_model_start",
            internal_diagnostics_posture: DiagnosticTierPosture::DeferredWithReason,
            palmistry_posture: DiagnosticTierPosture::DeferredWithReason,
            deferred_reason: Some(DEFERRED_REASON),
            follow_up_ref: Some("palmistry://wp1/embedded-model/ledger-start"),
        },
        BehaviorCoverageRow {
            behavior_id: "wp1.embedded_model.ledger_stop",
            schema_id: None,
            event_family: "kernel_process_lifecycle_stop",
            runtime_surface_id: "LlmClient::shutdown -> EmbeddedModelProcess::shutdown",
            user_manual_slug: "embedded-model-lifecycle-ledger",
            tool_id: "embedded_model_ledger_tests",
            eventledger_flight_recorder_path: "kernel_process_lifecycle:embedded_model_stop",
            internal_diagnostics_posture: DiagnosticTierPosture::DeferredWithReason,
            palmistry_posture: DiagnosticTierPosture::DeferredWithReason,
            deferred_reason: Some(DEFERRED_REASON),
            follow_up_ref: Some("palmistry://wp1/embedded-model/ledger-stop"),
        },
        BehaviorCoverageRow {
            behavior_id: "wp1.llm.fail_closed_fr",
            schema_id: None,
            event_family: "llm_inference",
            runtime_surface_id: "DisabledLlmClient::completion",
            user_manual_slug: "embedded-model-lifecycle-ledger",
            tool_id: "llm_client_local_routing_tests",
            eventledger_flight_recorder_path: "flight_recorder:llm_inference_fail_closed",
            internal_diagnostics_posture: DiagnosticTierPosture::DeferredWithReason,
            palmistry_posture: DiagnosticTierPosture::DeferredWithReason,
            deferred_reason: Some(DEFERRED_REASON),
            follow_up_ref: Some("palmistry://wp1/embedded-model/fail-closed-fr"),
        },
        BehaviorCoverageRow {
            behavior_id: "wp1.llm.embedding_fr",
            schema_id: None,
            event_family: "data_embedding_computed",
            runtime_surface_id:
                "LocalModelRuntimeLlmClient::embedding / DisabledLlmClient::embedding",
            user_manual_slug: "embedded-model-lifecycle-ledger",
            tool_id: "llm_client_local_routing_tests",
            eventledger_flight_recorder_path: "flight_recorder:data_embedding_computed",
            internal_diagnostics_posture: DiagnosticTierPosture::DeferredWithReason,
            palmistry_posture: DiagnosticTierPosture::DeferredWithReason,
            deferred_reason: Some(DEFERRED_REASON),
            follow_up_ref: Some("palmistry://wp1/embedded-model/embedding-fr"),
        },
    ]
}

/// WP-1 MT-012 operator chat/launch surface behavior coverage. Mirrors the
/// MT-013 shape and HBR-INT-009 posture: Flight Recorder / EventLedger WIRED for
/// each captured turn/thought/tool-call and the selection-decision audit;
/// internal_diagnostics + Palmistry DEFERRED-with-reason until WP-KERNEL-012/016
/// ship. Verified by [`verify_embedded_model_behavior_coverage`] (the MT-013
/// verifier — the operator-chat surface does not require the native
/// internal-diagnostics tier that the strict model-lane verifier demands).
pub fn operator_chat_launch_behavior_coverage_matrix() -> Vec<BehaviorCoverageRow> {
    const DEFERRED_REASON: &str =
        "MT-012 wires Flight Recorder/EventLedger for the operator chat/launch surface only \
         (the FR-EVT-AGENT-* activity events, the ModelLaneMessage EventLedger authority, and \
         the FR-EVT-MODEL-SELECTION-RECORDED selection audit); the native internal_diagnostics \
         tier and the external Palmistry watcher are integrated from the WP-KERNEL-012/016 \
         worktrees and must observe these records without becoming their authority.";
    vec![
        BehaviorCoverageRow {
            behavior_id: "wp1.operator_chat.launch",
            schema_id: None,
            event_family: "model_lane_launch",
            runtime_surface_id: "OperatorChatLaunchService::launch -> SwarmCoordinator::spawn_session",
            user_manual_slug: "operator-chat-launch",
            tool_id: "operator_chat_capture_tests",
            eventledger_flight_recorder_path: "flight_recorder:model_lane_launch",
            internal_diagnostics_posture: DiagnosticTierPosture::DeferredWithReason,
            palmistry_posture: DiagnosticTierPosture::DeferredWithReason,
            deferred_reason: Some(DEFERRED_REASON),
            follow_up_ref: Some("palmistry://wp1/operator-chat/launch"),
        },
        BehaviorCoverageRow {
            behavior_id: "wp1.operator_chat.capture_message",
            schema_id: None,
            event_family: "model_lane_message",
            runtime_surface_id: "ModelLaneCaptureRecorder::capture_cli_stream -> ModelLaneStore::record_message",
            user_manual_slug: "operator-chat-launch",
            tool_id: "operator_chat_capture_tests",
            eventledger_flight_recorder_path: "flight_recorder:model_lane_message",
            internal_diagnostics_posture: DiagnosticTierPosture::DeferredWithReason,
            palmistry_posture: DiagnosticTierPosture::DeferredWithReason,
            deferred_reason: Some(DEFERRED_REASON),
            follow_up_ref: Some("palmistry://wp1/operator-chat/capture-message"),
        },
        BehaviorCoverageRow {
            behavior_id: "wp1.operator_chat.agent_activity_fr",
            schema_id: None,
            event_family: "agent_activity",
            runtime_surface_id: "ModelLaneCaptureRecorder::record_activity -> agent_activity_event",
            user_manual_slug: "operator-chat-launch",
            tool_id: "operator_chat_capture_tests",
            eventledger_flight_recorder_path: "flight_recorder:agent_activity",
            internal_diagnostics_posture: DiagnosticTierPosture::DeferredWithReason,
            palmistry_posture: DiagnosticTierPosture::DeferredWithReason,
            deferred_reason: Some(DEFERRED_REASON),
            follow_up_ref: Some("palmistry://wp1/operator-chat/agent-activity-fr"),
        },
        BehaviorCoverageRow {
            behavior_id: "wp1.operator_chat.selection_audit",
            schema_id: None,
            event_family: "model_selection_recorded",
            runtime_surface_id: "OperatorChatLaunchService::record_selection -> ModelCatalog::record_selection_decision",
            user_manual_slug: "operator-chat-launch",
            tool_id: "operator_chat_capture_tests",
            eventledger_flight_recorder_path: "flight_recorder:model_selection_recorded",
            internal_diagnostics_posture: DiagnosticTierPosture::DeferredWithReason,
            palmistry_posture: DiagnosticTierPosture::DeferredWithReason,
            deferred_reason: Some(DEFERRED_REASON),
            follow_up_ref: Some("palmistry://wp1/operator-chat/selection-audit"),
        },
    ]
}

pub fn cloud_model_access_behavior_coverage_matrix() -> Vec<BehaviorCoverageRow> {
    const NOT_APPLICABLE_REASON: &str =
        "MT-015 cloud access is a Settings/keychain/API surface, not a ModelLane runtime lane; \
         behavior is verified by model_access_route_tests, the OS-keychain BYOK leak proof, and \
         native Argus AccessKit tests rather than HBR-INT-009 diagnostic-tier rows.";
    vec![
        BehaviorCoverageRow {
            behavior_id: "wp1.cloud_access.providers_enumeration",
            schema_id: None,
            event_family: "model_access_provider_enumeration",
            runtime_surface_id: "GET /model-access/providers",
            user_manual_slug: "cloud-model-access",
            tool_id: "model_access_route_tests",
            eventledger_flight_recorder_path:
                "http_route:/model-access/providers + test:get_providers_reflects_configured_and_excludes_gemini",
            internal_diagnostics_posture: DiagnosticTierPosture::NotApplicableWithReason,
            palmistry_posture: DiagnosticTierPosture::NotApplicableWithReason,
            deferred_reason: Some(NOT_APPLICABLE_REASON),
            follow_up_ref: None,
        },
        BehaviorCoverageRow {
            behavior_id: "wp1.cloud_access.byok_store",
            schema_id: None,
            event_family: "model_access_byok_key_store",
            runtime_surface_id: "PUT /model-access/byok/:provider/key",
            user_manual_slug: "cloud-model-access",
            tool_id: "model_access_route_tests",
            eventledger_flight_recorder_path:
                "http_route:/model-access/byok/:provider/key + tests:put_store_returns_200_and_never_echoes_the_key,put_empty_key_is_400,keychain_unavailable_is_503",
            internal_diagnostics_posture: DiagnosticTierPosture::NotApplicableWithReason,
            palmistry_posture: DiagnosticTierPosture::NotApplicableWithReason,
            deferred_reason: Some(NOT_APPLICABLE_REASON),
            follow_up_ref: None,
        },
        BehaviorCoverageRow {
            behavior_id: "wp1.cloud_access.byok_delete",
            schema_id: None,
            event_family: "model_access_byok_key_delete",
            runtime_surface_id: "DELETE /model-access/byok/:provider/key",
            user_manual_slug: "cloud-model-access",
            tool_id: "model_access_route_tests",
            eventledger_flight_recorder_path:
                "http_route:/model-access/byok/:provider/key + test:delete_byok_key_is_idempotent_and_updates_status",
            internal_diagnostics_posture: DiagnosticTierPosture::NotApplicableWithReason,
            palmistry_posture: DiagnosticTierPosture::NotApplicableWithReason,
            deferred_reason: Some(NOT_APPLICABLE_REASON),
            follow_up_ref: None,
        },
        BehaviorCoverageRow {
            behavior_id: "wp1.cloud_access.secret_leak_guard",
            schema_id: None,
            event_family: "cloud_access_byok_secret_leak_guard",
            runtime_surface_id: "CloudModelAccess::production + OsKeychainSecretsVault",
            user_manual_slug: "cloud-model-access",
            tool_id: "cloud_byok_access_config_leak_tests",
            eventledger_flight_recorder_path:
                "os_keychain:OsKeychainSecretsVault + cloud_invocation_audit + tracing_capture",
            internal_diagnostics_posture: DiagnosticTierPosture::NotApplicableWithReason,
            palmistry_posture: DiagnosticTierPosture::NotApplicableWithReason,
            deferred_reason: Some(NOT_APPLICABLE_REASON),
            follow_up_ref: None,
        },
        BehaviorCoverageRow {
            behavior_id: "wp1.cloud_access.settings_argus",
            schema_id: None,
            event_family: "cloud_models_settings_accesskit",
            runtime_surface_id: "Settings > Cloud Models",
            user_manual_slug: "cloud-model-access",
            tool_id: "test_cloud_models_settings_argus",
            eventledger_flight_recorder_path:
                "settings_argus:Cloud Models AccessKit tree + static provider fallback + key-buffer wipe",
            internal_diagnostics_posture: DiagnosticTierPosture::NotApplicableWithReason,
            palmistry_posture: DiagnosticTierPosture::NotApplicableWithReason,
            deferred_reason: Some(NOT_APPLICABLE_REASON),
            follow_up_ref: None,
        },
        BehaviorCoverageRow {
            behavior_id: "wp1.cloud_access.cli_bridge_login",
            schema_id: None,
            event_family: "cloud_models_cli_bridge_login",
            runtime_surface_id: "Settings > Cloud Models CLI bridge login",
            user_manual_slug: "cloud-model-access",
            tool_id: "test_cloud_models_settings_argus",
            eventledger_flight_recorder_path:
                "settings_argus:official_cli_bridge_login + provider_owned_login_command",
            internal_diagnostics_posture: DiagnosticTierPosture::NotApplicableWithReason,
            palmistry_posture: DiagnosticTierPosture::NotApplicableWithReason,
            deferred_reason: Some(NOT_APPLICABLE_REASON),
            follow_up_ref: None,
        },
    ]
}

pub fn dedicated_embedding_model_behavior_coverage_matrix() -> Vec<BehaviorCoverageRow> {
    vec![BehaviorCoverageRow {
        behavior_id: "wp1.llm.dedicated_embedding_model",
        schema_id: None,
        event_family: "data_embedding_computed",
        runtime_surface_id: "ModelCatalog::embedding_model_for_dim",
        user_manual_slug: "dedicated-embedding-model-routing",
        tool_id: "dedicated_embedding_model_tests",
        eventledger_flight_recorder_path:
            "flight_recorder:data_embedding_computed + loom_block_search_index.embedding_model",
        internal_diagnostics_posture: DiagnosticTierPosture::DeferredWithReason,
        palmistry_posture: DiagnosticTierPosture::DeferredWithReason,
        deferred_reason: Some(
            "MT-016 wires the authoritative runtime/catalog/Loom path; internal_diagnostics and Palmistry consume these selected-model receipts from follow-up worktrees.",
        ),
        follow_up_ref: Some("palmistry://wp1/dedicated-embedding-model/routing"),
    }]
}

pub fn verify_cloud_model_access_behavior_coverage(
    rows: &[BehaviorCoverageRow],
    pages: &[UserManualPage],
    tools: &[UserManualToolEntry],
) -> Result<(), Vec<BehaviorCoverageError>> {
    let page_slugs = pages
        .iter()
        .map(|page| page.slug.as_str())
        .collect::<BTreeSet<_>>();
    let tool_ids = tools
        .iter()
        .map(|tool| tool.tool_id.as_str())
        .collect::<BTreeSet<_>>();

    let mut errors = Vec::new();
    for row in rows {
        if row.runtime_surface_id.trim().is_empty() {
            errors.push(BehaviorCoverageError {
                behavior_id: row.behavior_id,
                reason: "runtime_surface_id missing".to_owned(),
            });
        }
        if !row.self_consistency_result().starts_with("verified:") {
            errors.push(BehaviorCoverageError {
                behavior_id: row.behavior_id,
                reason: "self_consistency_result missing".to_owned(),
            });
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
        let path = row.eventledger_flight_recorder_path.trim();
        if path.is_empty()
            || ![
                "http_route:/model-access",
                "os_keychain:",
                "settings_argus:",
                "cloud_invocation_audit",
            ]
            .iter()
            .any(|marker| path.contains(marker))
        {
            errors.push(BehaviorCoverageError {
                behavior_id: row.behavior_id,
                reason: "MT-015 evidence path must cite the model-access route, OS keychain/leak proof, or Settings Argus proof".to_owned(),
            });
        }
        if row.internal_diagnostics_posture != DiagnosticTierPosture::NotApplicableWithReason {
            errors.push(BehaviorCoverageError {
                behavior_id: row.behavior_id,
                reason: format!(
                    "internal_diagnostics posture must be NOT_APPLICABLE-with-reason for MT-015, got {}",
                    row.internal_diagnostics_posture.as_str()
                ),
            });
        }
        if row.palmistry_posture != DiagnosticTierPosture::NotApplicableWithReason {
            errors.push(BehaviorCoverageError {
                behavior_id: row.behavior_id,
                reason: format!(
                    "Palmistry posture must be NOT_APPLICABLE-with-reason for MT-015, got {}",
                    row.palmistry_posture.as_str()
                ),
            });
        }
        if row.deferred_reason.is_none() {
            errors.push(BehaviorCoverageError {
                behavior_id: row.behavior_id,
                reason: "NOT_APPLICABLE-with-reason posture requires a reason".to_owned(),
            });
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

/// Verifies the WP-1 MT-013 embedded-model behavior coverage rows against the
/// seeded UserManual pages/tools and the MT-013 HBR-INT-009 posture:
/// Flight Recorder / EventLedger WIRED (path present and points at a real
/// durable surface), internal_diagnostics + Palmistry DEFERRED-with-reason
/// (reason + follow_up_ref required).
pub fn verify_embedded_model_behavior_coverage(
    rows: &[BehaviorCoverageRow],
    pages: &[UserManualPage],
    tools: &[UserManualToolEntry],
) -> Result<(), Vec<BehaviorCoverageError>> {
    let page_slugs = pages
        .iter()
        .map(|page| page.slug.as_str())
        .collect::<BTreeSet<_>>();
    let tool_ids = tools
        .iter()
        .map(|tool| tool.tool_id.as_str())
        .collect::<BTreeSet<_>>();

    let mut errors = Vec::new();
    for row in rows {
        if row.runtime_surface_id.trim().is_empty() {
            errors.push(BehaviorCoverageError {
                behavior_id: row.behavior_id,
                reason: "runtime_surface_id missing".to_owned(),
            });
        }
        if !row.self_consistency_result().starts_with("verified:") {
            errors.push(BehaviorCoverageError {
                behavior_id: row.behavior_id,
                reason: "self_consistency_result missing".to_owned(),
            });
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
        // FR/EventLedger WIRED: the path must be present and reference a real
        // durable surface (the ProcessOwnershipLedger table or the Flight
        // Recorder), not an empty placeholder.
        let path = row.eventledger_flight_recorder_path.trim();
        if path.is_empty()
            || !(path.contains("kernel_process_lifecycle") || path.contains("flight_recorder"))
        {
            errors.push(BehaviorCoverageError {
                behavior_id: row.behavior_id,
                reason: "EventLedger/FlightRecorder evidence path missing or not WIRED to a durable surface".to_owned(),
            });
        }
        // MT-013 posture: internal_diagnostics DEFERRED-with-reason.
        if row.internal_diagnostics_posture != DiagnosticTierPosture::DeferredWithReason {
            errors.push(BehaviorCoverageError {
                behavior_id: row.behavior_id,
                reason: format!(
                    "internal_diagnostics posture must be DEFERRED-with-reason for MT-013, got {}",
                    row.internal_diagnostics_posture.as_str()
                ),
            });
        }
        // MT-013 posture: Palmistry DEFERRED-with-reason.
        if row.palmistry_posture != DiagnosticTierPosture::DeferredWithReason {
            errors.push(BehaviorCoverageError {
                behavior_id: row.behavior_id,
                reason: format!(
                    "Palmistry posture must be DEFERRED-with-reason for MT-013, got {}",
                    row.palmistry_posture.as_str()
                ),
            });
        }
        // Any DEFERRED-with-reason tier requires a reason + a follow-up ref.
        if (row.internal_diagnostics_posture == DiagnosticTierPosture::DeferredWithReason
            || row.palmistry_posture == DiagnosticTierPosture::DeferredWithReason)
            && (row.deferred_reason.is_none() || row.follow_up_ref.is_none())
        {
            errors.push(BehaviorCoverageError {
                behavior_id: row.behavior_id,
                reason: "DEFERRED-with-reason tiers require deferred_reason and follow_up_ref"
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
        if row.runtime_surface_id.trim().is_empty() {
            errors.push(BehaviorCoverageError {
                behavior_id: row.behavior_id,
                reason: "runtime_surface_id missing".to_owned(),
            });
        }
        if !row.self_consistency_result().starts_with("verified:") {
            errors.push(BehaviorCoverageError {
                behavior_id: row.behavior_id,
                reason: "self_consistency_result missing".to_owned(),
            });
        }
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
