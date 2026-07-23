use std::collections::BTreeSet;

use crate::api::model_runtime_registry::{
    MODEL_RUNTIME_REGISTRY_INTEGRITY_ERROR_CODE, MODEL_RUNTIME_REGISTRY_PROJECTION_SCHEMA_ID,
    MODEL_RUNTIME_REGISTRY_ROUTE, MODEL_RUNTIME_REGISTRY_UNAVAILABLE_CODE,
    MODEL_RUNTIME_SELECTION_INVALID_CODE, MODEL_RUNTIME_SELECTION_REJECTED_CODE,
    MODEL_RUNTIME_SELECTION_ROUTE,
};
use crate::flight_recorder::events_agent_activity::agent_activity_event;
use crate::flight_recorder::FlightRecorderEventType;
use crate::kernel::KernelEventType;
use crate::llm::embedded_ledger::EmbeddedModelProcess;
use crate::llm::{DisabledLlmClient, LlmClient};
use crate::model_runtime::cloud::{CliSubprocessSpawner, LiveCliSpawner};
use crate::model_runtime::{ModelCatalog, ModelRegistryStore, MODEL_RUNTIME_REGISTRY_SCHEMA_ID};
use crate::process_ledger::reclaim::reclaim_pidless_embedded_orphans;
use crate::sandbox::{HandshakeNativeSandboxAdapter, SandboxAdapter};
use crate::storage::StorageError;
use crate::swarm_orchestration::model_lane::{
    DexterityLaunchAdapterRegistry, ModelLaneDiagnosticTierPosture, ModelLaneDiagnosticsProjection,
    ModelLaneSchemaRegistryRow, ModelLaneStore,
};
use crate::swarm_orchestration::operator_chat::{
    ModelLaneCaptureRecorder, OperatorChatLaunchService,
};
use crate::swarm_orchestration::routing_execution::ModelLaneRoutingExecutionStore;

use super::registry::{wp009_surface_registry, SurfaceGroup};
use super::store::{UserManualFeatureEntry, UserManualPage, UserManualToolEntry};
use super::USER_MANUAL_VERSION;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
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

    pub fn self_consistency_result(
        &self,
        schema_registry: &[ModelLaneSchemaRegistryRow],
        pages: &[UserManualPage],
        tools: &[UserManualToolEntry],
    ) -> Result<BehaviorConsistencyProof, Vec<BehaviorCoverageError>> {
        compute_behavior_consistency(self, schema_registry, pages, tools)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BehaviorConsistencyProof {
    pub behavior_id: &'static str,
    pub checked_authorities: BTreeSet<&'static str>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CompiledSurfaceAnchor {
    ModelLaneRecordRun,
    DexterityNormalize,
    OfficialCliLiveSpawn,
    OfficialCliAttachedSandbox,
    ModelLaneRecordMessage,
    ModelLaneRecordTerminal,
    ModelLaneRecordPromotion,
    ModelLaneRecordContextArtifact,
    ModelLaneRecordContextHandoff,
    ModelLaneRecordCloudPlan,
    ModelLaneRecordCloudConsent,
    ModelLaneRecordCloudDenial,
    ModelLaneRecoverRun,
    ModelLaneRecordRecovery,
    ModelLaneRecordLease,
    ModelLaneDiagnostics,
    ModelLaneStore,
    ModelLaneRoutingExecutionStore,
    EmbeddedModelProcess,
    EmbeddedModelShutdown,
    ReclaimPidlessEmbeddedOrphans,
    DisabledLlmCompletion,
    LlmEmbedding,
    OperatorChatLaunch,
    OperatorChatCapture,
    OperatorChatActivity,
    AgentActivityEvent,
    OperatorChatSelection,
    ModelAccessRoutes,
    OperatorChatRoutes,
    ModelCatalogEmbedding,
    DataEmbeddingComputed,
}

impl CompiledSurfaceAnchor {
    fn canonical_runtime_surface_id(self) -> &'static str {
        match self {
            Self::ModelLaneRecordRun => "ModelLaneStore::record_run",
            Self::DexterityNormalize => "DexterityLaunchAdapterRegistry::normalize",
            Self::OfficialCliLiveSpawn => "LiveCliSpawner::spawn",
            Self::OfficialCliAttachedSandbox => {
                "HandshakeNativeSandboxAdapter::spawn_attached_with_stdio"
            }
            Self::ModelLaneRecordMessage => "ModelLaneStore::record_message",
            Self::ModelLaneRecordTerminal => "ModelLaneStore::record_lane_terminal_status",
            Self::ModelLaneRecordPromotion => "ModelLaneStore::record_promotion_decision",
            Self::ModelLaneRecordContextArtifact => {
                "ModelLaneStore::record_context_bundle_artifact_binding"
            }
            Self::ModelLaneRecordContextHandoff => "ModelLaneStore::record_context_bundle_handoff",
            Self::ModelLaneRecordCloudPlan => "ModelLaneStore::record_cloud_projection_plan",
            Self::ModelLaneRecordCloudConsent => "ModelLaneStore::record_cloud_consent_receipt",
            Self::ModelLaneRecordCloudDenial => "ModelLaneStore",
            Self::ModelLaneRecoverRun => "ModelLaneStore::recover_run_after_restart",
            Self::ModelLaneRecordRecovery => "ModelLaneStore::record_recovery_event",
            Self::ModelLaneRecordLease => "ModelLaneStore::record_lane_lease",
            Self::ModelLaneDiagnostics => "ModelLaneDiagnosticsProjection",
            Self::ModelLaneStore => "ModelLaneStore",
            Self::ModelLaneRoutingExecutionStore => "ModelLaneRoutingExecutionStore",
            Self::EmbeddedModelProcess => "EmbeddedModelProcess",
            Self::EmbeddedModelShutdown => "EmbeddedModelProcess::shutdown",
            Self::ReclaimPidlessEmbeddedOrphans => "reclaim_pidless_embedded_orphans",
            Self::DisabledLlmCompletion => "DisabledLlmClient::completion",
            Self::LlmEmbedding => "LlmClient::embedding",
            Self::OperatorChatLaunch => "OperatorChatLaunchService::launch",
            Self::OperatorChatCapture => "ModelLaneCaptureRecorder::capture_cli_stream",
            Self::OperatorChatActivity => "ModelLaneCaptureRecorder::record_activity",
            Self::AgentActivityEvent => "agent_activity_event",
            Self::OperatorChatSelection => "OperatorChatLaunchService::record_selection",
            Self::ModelAccessRoutes => "api::model_access::routes",
            Self::OperatorChatRoutes => "api::operator_chat::routes",
            Self::ModelCatalogEmbedding => "ModelCatalog::embedding_model_for_dim",
            Self::DataEmbeddingComputed => "FlightRecorderEventType::DataEmbeddingComputed",
        }
    }

    fn assert_compiled(self) {
        match self {
            Self::ModelLaneRecordRun => {
                let _ = ModelLaneStore::record_run;
            }
            Self::DexterityNormalize => {
                let _ = DexterityLaunchAdapterRegistry::normalize;
            }
            Self::OfficialCliLiveSpawn => {
                let _ = <LiveCliSpawner as CliSubprocessSpawner>::spawn;
            }
            Self::OfficialCliAttachedSandbox => {
                let _ =
                    <HandshakeNativeSandboxAdapter as SandboxAdapter>::spawn_attached_with_stdio;
            }
            Self::ModelLaneRecordMessage => {
                let _ = ModelLaneStore::record_message;
            }
            Self::ModelLaneRecordTerminal => {
                let _ = ModelLaneStore::record_lane_terminal_status;
            }
            Self::ModelLaneRecordPromotion => {
                let _ = ModelLaneStore::record_promotion_decision;
            }
            Self::ModelLaneRecordContextArtifact => {
                let _ = ModelLaneStore::record_context_bundle_artifact_binding;
            }
            Self::ModelLaneRecordContextHandoff => {
                let _ = ModelLaneStore::record_context_bundle_handoff;
            }
            Self::ModelLaneRecordCloudPlan => {
                let _ = ModelLaneStore::record_cloud_projection_plan;
            }
            Self::ModelLaneRecordCloudConsent => {
                let _ = ModelLaneStore::record_cloud_consent_receipt;
            }
            Self::ModelLaneRecordCloudDenial => {
                let _ = std::mem::size_of::<ModelLaneStore>();
            }
            Self::ModelLaneRecoverRun => {
                let _ = ModelLaneStore::recover_run_after_restart;
            }
            Self::ModelLaneRecordRecovery => {
                let _ = ModelLaneStore::record_recovery_event;
            }
            Self::ModelLaneRecordLease => {
                let _ = ModelLaneStore::record_lane_lease;
            }
            Self::ModelLaneDiagnostics => {
                let _ = std::mem::size_of::<ModelLaneDiagnosticsProjection>();
            }
            Self::ModelLaneStore => {
                let _ = std::mem::size_of::<ModelLaneStore>();
            }
            Self::ModelLaneRoutingExecutionStore => {
                let _ = std::mem::size_of::<ModelLaneRoutingExecutionStore>();
            }
            Self::EmbeddedModelProcess => {
                let _ = std::mem::size_of::<EmbeddedModelProcess>();
            }
            Self::EmbeddedModelShutdown => {
                let _ = EmbeddedModelProcess::shutdown;
            }
            Self::ReclaimPidlessEmbeddedOrphans => {
                let _ = reclaim_pidless_embedded_orphans;
            }
            Self::DisabledLlmCompletion => {
                let _ = <DisabledLlmClient as LlmClient>::completion;
            }
            Self::LlmEmbedding => {
                let _ = <DisabledLlmClient as LlmClient>::embedding;
            }
            Self::OperatorChatLaunch => {
                let _ = OperatorChatLaunchService::launch;
            }
            Self::OperatorChatCapture => {
                let _ = ModelLaneCaptureRecorder::capture_cli_stream::<Vec<String>, String>;
            }
            Self::OperatorChatActivity => {
                let _ = ModelLaneCaptureRecorder::record_activity;
            }
            Self::AgentActivityEvent => {
                let _ = agent_activity_event;
            }
            Self::OperatorChatSelection => {
                let _ = OperatorChatLaunchService::record_selection;
            }
            Self::ModelAccessRoutes => {
                let _ = crate::api::model_access::routes;
            }
            Self::OperatorChatRoutes => {
                let _ = crate::api::operator_chat::routes;
            }
            Self::ModelCatalogEmbedding => {
                let _ = ModelCatalog::embedding_model_for_dim;
            }
            Self::DataEmbeddingComputed => {
                let _ = FlightRecorderEventType::DataEmbeddingComputed;
            }
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct RuntimeSurfaceAnchor {
    behavior_id: &'static str,
    anchor: CompiledSurfaceAnchor,
}

const INTERNAL_RUNTIME_SURFACE_ANCHORS: &[RuntimeSurfaceAnchor] = &[
    RuntimeSurfaceAnchor {
        behavior_id: "wp1.model_lane.run",
        anchor: CompiledSurfaceAnchor::ModelLaneRecordRun,
    },
    RuntimeSurfaceAnchor {
        behavior_id: "wp1.model_lane.launch",
        anchor: CompiledSurfaceAnchor::DexterityNormalize,
    },
    RuntimeSurfaceAnchor {
        behavior_id: "wp1.model_lane.official_cli_spawn",
        anchor: CompiledSurfaceAnchor::OfficialCliLiveSpawn,
    },
    RuntimeSurfaceAnchor {
        behavior_id: "wp1.model_lane.official_cli_attached_sandbox",
        anchor: CompiledSurfaceAnchor::OfficialCliAttachedSandbox,
    },
    RuntimeSurfaceAnchor {
        behavior_id: "wp1.model_lane.message",
        anchor: CompiledSurfaceAnchor::ModelLaneRecordMessage,
    },
    RuntimeSurfaceAnchor {
        behavior_id: "wp1.model_lane.terminal",
        anchor: CompiledSurfaceAnchor::ModelLaneRecordTerminal,
    },
    RuntimeSurfaceAnchor {
        behavior_id: "wp1.model_lane.promotion",
        anchor: CompiledSurfaceAnchor::ModelLaneRecordPromotion,
    },
    RuntimeSurfaceAnchor {
        behavior_id: "wp1.model_lane.context_bundle_artifact",
        anchor: CompiledSurfaceAnchor::ModelLaneRecordContextArtifact,
    },
    RuntimeSurfaceAnchor {
        behavior_id: "wp1.model_lane.context_bundle",
        anchor: CompiledSurfaceAnchor::ModelLaneRecordContextHandoff,
    },
    RuntimeSurfaceAnchor {
        behavior_id: "wp1.model_lane.cloud_projection_plan",
        anchor: CompiledSurfaceAnchor::ModelLaneRecordCloudPlan,
    },
    RuntimeSurfaceAnchor {
        behavior_id: "wp1.model_lane.cloud_projection_plan_v2",
        anchor: CompiledSurfaceAnchor::ModelLaneRecordCloudPlan,
    },
    RuntimeSurfaceAnchor {
        behavior_id: "wp1.model_lane.cloud_consent",
        anchor: CompiledSurfaceAnchor::ModelLaneRecordCloudConsent,
    },
    RuntimeSurfaceAnchor {
        behavior_id: "wp1.model_lane.cloud_consent_v2",
        anchor: CompiledSurfaceAnchor::ModelLaneRecordCloudConsent,
    },
    RuntimeSurfaceAnchor {
        behavior_id: "wp1.model_lane.cloud_consent_denial",
        anchor: CompiledSurfaceAnchor::ModelLaneRecordCloudDenial,
    },
    RuntimeSurfaceAnchor {
        behavior_id: "wp1.model_lane.recovery",
        anchor: CompiledSurfaceAnchor::ModelLaneRecoverRun,
    },
    RuntimeSurfaceAnchor {
        behavior_id: "wp1.model_lane.recovery_event",
        anchor: CompiledSurfaceAnchor::ModelLaneRecordRecovery,
    },
    RuntimeSurfaceAnchor {
        behavior_id: "wp1.model_lane.recovery_event_v2",
        anchor: CompiledSurfaceAnchor::ModelLaneRecordRecovery,
    },
    RuntimeSurfaceAnchor {
        behavior_id: "wp1.model_lane.lease",
        anchor: CompiledSurfaceAnchor::ModelLaneRecordLease,
    },
    RuntimeSurfaceAnchor {
        behavior_id: "wp1.model_lane.diagnostics",
        anchor: CompiledSurfaceAnchor::ModelLaneDiagnostics,
    },
    RuntimeSurfaceAnchor {
        behavior_id: "wp1.model_lane.mixed_validation",
        anchor: CompiledSurfaceAnchor::ModelLaneStore,
    },
    RuntimeSurfaceAnchor {
        behavior_id: "wp1.model_lane.routing_execution",
        anchor: CompiledSurfaceAnchor::ModelLaneRoutingExecutionStore,
    },
    RuntimeSurfaceAnchor {
        behavior_id: "wp1.model_lane.routing_outbox",
        anchor: CompiledSurfaceAnchor::ModelLaneRoutingExecutionStore,
    },
    RuntimeSurfaceAnchor {
        behavior_id: "wp1.model_lane.routing_stage_attempt",
        anchor: CompiledSurfaceAnchor::ModelLaneRoutingExecutionStore,
    },
    RuntimeSurfaceAnchor {
        behavior_id: "wp1.model_lane.run_extension",
        anchor: CompiledSurfaceAnchor::ModelLaneRoutingExecutionStore,
    },
    RuntimeSurfaceAnchor {
        behavior_id: "wp1.embedded_model.ledger_start",
        anchor: CompiledSurfaceAnchor::EmbeddedModelProcess,
    },
    RuntimeSurfaceAnchor {
        behavior_id: "wp1.embedded_model.ledger_stop",
        anchor: CompiledSurfaceAnchor::EmbeddedModelShutdown,
    },
    RuntimeSurfaceAnchor {
        behavior_id: "wp1.embedded_model.os_lease_reclaim",
        anchor: CompiledSurfaceAnchor::ReclaimPidlessEmbeddedOrphans,
    },
    RuntimeSurfaceAnchor {
        behavior_id: "wp1.llm.fail_closed_fr",
        anchor: CompiledSurfaceAnchor::DisabledLlmCompletion,
    },
    RuntimeSurfaceAnchor {
        behavior_id: "wp1.llm.embedding_fr",
        anchor: CompiledSurfaceAnchor::LlmEmbedding,
    },
    RuntimeSurfaceAnchor {
        behavior_id: "wp1.operator_chat.capture_message",
        anchor: CompiledSurfaceAnchor::OperatorChatCapture,
    },
    RuntimeSurfaceAnchor {
        behavior_id: "wp1.operator_chat.agent_activity_fr",
        anchor: CompiledSurfaceAnchor::AgentActivityEvent,
    },
    RuntimeSurfaceAnchor {
        behavior_id: "wp1.cloud_access.secret_leak_guard",
        anchor: CompiledSurfaceAnchor::ModelAccessRoutes,
    },
    RuntimeSurfaceAnchor {
        behavior_id: "wp1.cloud_access.settings_argus",
        anchor: CompiledSurfaceAnchor::ModelAccessRoutes,
    },
    RuntimeSurfaceAnchor {
        behavior_id: "wp1.cloud_access.cli_bridge_login",
        anchor: CompiledSurfaceAnchor::ModelAccessRoutes,
    },
    RuntimeSurfaceAnchor {
        behavior_id: "wp1.llm.dedicated_embedding_model",
        anchor: CompiledSurfaceAnchor::ModelCatalogEmbedding,
    },
];

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

fn compute_behavior_consistency(
    row: &BehaviorCoverageRow,
    schema_registry: &[ModelLaneSchemaRegistryRow],
    pages: &[UserManualPage],
    tools: &[UserManualToolEntry],
) -> Result<BehaviorConsistencyProof, Vec<BehaviorCoverageError>> {
    let mut errors = Vec::new();
    let mut checked_authorities = BTreeSet::new();

    let route_surface = wp009_surface_registry()
        .iter()
        .find(|surface| surface.surface_id == row.behavior_id);
    if let Some(surface) = route_surface {
        let route_anchor = match surface.group {
            SurfaceGroup::ModelAccess => CompiledSurfaceAnchor::ModelAccessRoutes,
            SurfaceGroup::OperatorChat => CompiledSurfaceAnchor::OperatorChatRoutes,
            _ => {
                errors.push(BehaviorCoverageError {
                    behavior_id: row.behavior_id,
                    reason: "behavior route belongs to an unsupported coverage family".to_owned(),
                });
                CompiledSurfaceAnchor::ModelLaneStore
            }
        };
        route_anchor.assert_compiled();
        checked_authorities.insert("compiled_route_module");
        checked_authorities.insert("surface_registry");
        if row.runtime_surface_id != surface.route {
            errors.push(BehaviorCoverageError {
                behavior_id: row.behavior_id,
                reason: format!(
                    "runtime_surface_id `{}` does not equal canonical {} {} route `{}`",
                    row.runtime_surface_id, surface.method, surface.surface_id, surface.route
                ),
            });
        }
        if row.user_manual_slug != surface.group.page_slug() {
            errors.push(BehaviorCoverageError {
                behavior_id: row.behavior_id,
                reason: format!(
                    "UserManual slug {} does not equal canonical route-group slug {}",
                    row.user_manual_slug,
                    surface.group.page_slug()
                ),
            });
        }
    } else if let Some(registry_anchor) = INTERNAL_RUNTIME_SURFACE_ANCHORS
        .iter()
        .find(|anchor| anchor.behavior_id == row.behavior_id)
    {
        registry_anchor.anchor.assert_compiled();
        checked_authorities.insert("compiled_internal_symbol");
        checked_authorities.insert("runtime_surface_anchor_registry");
        let expected = registry_anchor.anchor.canonical_runtime_surface_id();
        if row.runtime_surface_id != expected {
            errors.push(BehaviorCoverageError {
                behavior_id: row.behavior_id,
                reason: format!(
                    "runtime_surface_id `{}` does not equal compile-anchored symbol `{expected}`",
                    row.runtime_surface_id
                ),
            });
        }
    } else {
        errors.push(BehaviorCoverageError {
            behavior_id: row.behavior_id,
            reason: "behavior has no canonical route or compiled runtime-surface anchor".to_owned(),
        });
    }

    if let Some(schema_id) = row.schema_id {
        checked_authorities.insert("schema_registry");
        if !schema_registry
            .iter()
            .any(|schema| schema.schema_id == schema_id)
        {
            errors.push(BehaviorCoverageError {
                behavior_id: row.behavior_id,
                reason: format!("schema_id {schema_id} missing from ModelLane schema registry"),
            });
        }
    }

    checked_authorities.insert("usermanual_page_registry");
    if !pages.iter().any(|page| page.slug == row.user_manual_slug) {
        errors.push(BehaviorCoverageError {
            behavior_id: row.behavior_id,
            reason: format!("UserManual page {} missing", row.user_manual_slug),
        });
    }
    checked_authorities.insert("usermanual_tool_registry");
    if !tools.iter().any(|tool| tool.tool_id == row.tool_id) {
        errors.push(BehaviorCoverageError {
            behavior_id: row.behavior_id,
            reason: format!("UserManual tool {} missing", row.tool_id),
        });
    }
    checked_authorities.insert("event_evidence_contract");
    if row.event_family.trim().is_empty() || row.eventledger_flight_recorder_path.trim().is_empty()
    {
        errors.push(BehaviorCoverageError {
            behavior_id: row.behavior_id,
            reason:
                "event family and EventLedger/Flight Recorder evidence path must both be present"
                    .to_owned(),
        });
    }
    checked_authorities.insert("diagnostic_posture_contract");
    if row.behavior_id.starts_with("wp1.model_lane.")
        && (row.internal_diagnostics_posture != DiagnosticTierPosture::Wired
            || row.palmistry_posture != DiagnosticTierPosture::Wired)
    {
        errors.push(BehaviorCoverageError {
            behavior_id: row.behavior_id,
            reason: "internal_diagnostics and Palmistry postures must be WIRED for the native diagnostics producer and watcher recovery path"
                .to_owned(),
        });
    }
    if (row.internal_diagnostics_posture == DiagnosticTierPosture::DeferredWithReason
        || row.palmistry_posture == DiagnosticTierPosture::DeferredWithReason)
        && (row.deferred_reason.is_none() || row.follow_up_ref.is_none())
    {
        errors.push(BehaviorCoverageError {
            behavior_id: row.behavior_id,
            reason: "DEFERRED-with-reason posture requires deferred_reason and follow_up_ref"
                .to_owned(),
        });
    }
    if (row.internal_diagnostics_posture == DiagnosticTierPosture::NotApplicableWithReason
        || row.palmistry_posture == DiagnosticTierPosture::NotApplicableWithReason)
        && row.deferred_reason.is_none()
    {
        errors.push(BehaviorCoverageError {
            behavior_id: row.behavior_id,
            reason: "NOT_APPLICABLE-with-reason posture requires an explicit reason".to_owned(),
        });
    }

    if errors.is_empty() {
        Ok(BehaviorConsistencyProof {
            behavior_id: row.behavior_id,
            checked_authorities,
        })
    } else {
        Err(errors)
    }
}

pub const MODEL_RUNTIME_REGISTRY_MANUAL_FEATURE_ID: &str =
    "wp1.mt014_model_catalog_and_loom_degrade";
pub const MODEL_RUNTIME_REGISTRY_DECLARED_PROOF_SCOPE: &str =
    "DECLARED PROOF TARGETS are not executed by UserManual coverage validation";

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelRuntimeProofExecutionStatus {
    DeclaredNotExecuted,
}

/// Machine-readable MT-014 contract joining each ModelRuntime registry behavior
/// to its canonical runtime surface, durable evidence, UserManual entry, proof
/// target, recovery instruction, and HBR-INT-009 posture.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct ModelRuntimeRegistryBehaviorCoverageRow {
    pub behavior_id: &'static str,
    pub runtime_surface_id: &'static str,
    pub schema_id: Option<&'static str>,
    pub eventledger_event_type: Option<&'static str>,
    pub eventledger_evidence_path: Option<&'static str>,
    pub response_code: Option<&'static str>,
    pub manual_feature_id: &'static str,
    pub manual_evidence_marker: &'static str,
    pub recovery_instruction_marker: &'static str,
    pub proof_tool_id: &'static str,
    pub proof_execution_status: ModelRuntimeProofExecutionStatus,
    pub manual_version: &'static str,
    pub internal_diagnostics_posture: DiagnosticTierPosture,
    pub palmistry_posture: DiagnosticTierPosture,
    pub diagnostic_reason: &'static str,
    pub follow_up_ref: &'static str,
}

pub fn model_runtime_registry_behavior_coverage_matrix(
) -> Vec<ModelRuntimeRegistryBehaviorCoverageRow> {
    const EVENTLEDGER_PATH: &str = "eventledger://kernel/MODEL_RUNTIME_SELECTION_RECORDED";
    const RECOVERY: &str =
        "Restore the current migration chain/database authority and restore configuration to the persisted SHA/binding.";
    const DIAGNOSTIC_REASON: &str = "internal_diagnostics and Palmistry are wired through the native diagnostics ring, authenticated Palmistry watcher, Problems projection, and survivor recovery importer.";

    let selection_event_type = KernelEventType::ModelRuntimeSelectionRecorded.as_str();
    let row = |behavior_id,
               runtime_surface_id,
               manual_evidence_marker,
               proof_tool_id,
               follow_up_ref,
               schema_id,
               eventledger_event_type,
               eventledger_evidence_path,
               response_code| {
        ModelRuntimeRegistryBehaviorCoverageRow {
            behavior_id,
            runtime_surface_id,
            schema_id,
            eventledger_event_type,
            eventledger_evidence_path,
            response_code,
            manual_feature_id: MODEL_RUNTIME_REGISTRY_MANUAL_FEATURE_ID,
            manual_evidence_marker,
            recovery_instruction_marker: RECOVERY,
            proof_tool_id,
            proof_execution_status: ModelRuntimeProofExecutionStatus::DeclaredNotExecuted,
            manual_version: USER_MANUAL_VERSION,
            internal_diagnostics_posture: DiagnosticTierPosture::Wired,
            palmistry_posture: DiagnosticTierPosture::Wired,
            diagnostic_reason: DIAGNOSTIC_REASON,
            follow_up_ref,
        }
    };

    vec![
        row(
            "wp1.model_runtime_registry.persistent_adapter_selection",
            "ModelRegistryStore::persist_boot_set_and_read_back",
            "atomically persists and reads back the complete primary-plus-embedding boot set",
            "mt014_persistent_registry_survives_restart_and_reads_back_selection",
            "palmistry://wp1/model-runtime-registry/persistent-selection",
            Some(MODEL_RUNTIME_REGISTRY_SCHEMA_ID),
            Some(selection_event_type),
            Some(EVENTLEDGER_PATH),
            None,
        ),
        row(
            "wp1.model_runtime_registry.restart_recovery",
            "ModelRegistryStore::recover_configured_runtime_binding_set",
            "A normal restart or display-name/path change preserves selection revision",
            "mt014_persistent_registry_survives_restart_and_reads_back_selection",
            "palmistry://wp1/model-runtime-registry/restart-recovery",
            Some(MODEL_RUNTIME_REGISTRY_SCHEMA_ID),
            Some(selection_event_type),
            Some(EVENTLEDGER_PATH),
            None,
        ),
        row(
            "wp1.model_runtime_registry.fail_closed_selection_conflict",
            "ModelRegistryStore::persist_boot_set_and_read_back",
            "a conflicting adapter/capability choice fails closed",
            "mt014_concurrent_incompatible_adapter_selection_has_one_winner",
            "palmistry://wp1/model-runtime-registry/selection-conflict",
            Some(MODEL_RUNTIME_REGISTRY_SCHEMA_ID),
            None,
            None,
            None,
        ),
        row(
            "wp1.model_runtime_registry.api_projection",
            MODEL_RUNTIME_REGISTRY_ROUTE,
            "GET /model-runtime/registry",
            "mt014_registry_api_joins_real_pg_rows_to_current_ready_catalog_by_sha256",
            "palmistry://wp1/model-runtime-registry/api-projection",
            Some(MODEL_RUNTIME_REGISTRY_PROJECTION_SCHEMA_ID),
            Some(selection_event_type),
            Some(EVENTLEDGER_PATH),
            None,
        ),
        row(
            "wp1.model_runtime_registry.native_panel",
            "PaneType::ModelRuntime + model-runtime.registry.*",
            "model-runtime.registry.*",
            "mt014_argus_renders_real_pg_live_and_dormant_registry_rows",
            "palmistry://wp1/model-runtime-registry/native-panel",
            Some(MODEL_RUNTIME_REGISTRY_PROJECTION_SCHEMA_ID),
            None,
            None,
            None,
        ),
        row(
            "wp1.model_runtime_registry.eventledger_selection_evidence",
            "ModelRegistryStore::persist_and_read_back -> KernelEventType::ModelRuntimeSelectionRecorded",
            "KernelEventType::ModelRuntimeSelectionRecorded",
            "mt014_registry_rejects_eventledger_chain_and_immutable_row_tampering",
            "palmistry://wp1/model-runtime-registry/eventledger-selection",
            None,
            Some(selection_event_type),
            Some(EVENTLEDGER_PATH),
            None,
        ),
        row(
            "wp1.model_runtime.selection.post.success",
            MODEL_RUNTIME_SELECTION_ROUTE,
            "Success advances the already-validated projection in memory and returns `selection_receipt_ref`",
            "mt014_selection_post_prevalidates_then_returns_audited_projection",
            "palmistry://wp1/model-runtime-registry/selection-post-success",
            Some(MODEL_RUNTIME_REGISTRY_PROJECTION_SCHEMA_ID),
            None,
            None,
            None,
        ),
        row(
            "wp1.model_runtime.selection.post.failure.audit",
            MODEL_RUNTIME_SELECTION_ROUTE,
            "Audit failure",
            "mt014_selection_post_audit_failure_preserves_prior_selection",
            "palmistry://wp1/model-runtime-registry/selection-post-audit-failure",
            None,
            None,
            None,
            Some(MODEL_RUNTIME_SELECTION_REJECTED_CODE),
        ),
        row(
            "wp1.model_runtime.selection.post.failure.stale_target",
            MODEL_RUNTIME_SELECTION_ROUTE,
            "stale current selection",
            "mt014_selection_post_rejects_stale_target_before_swap",
            "palmistry://wp1/model-runtime-registry/selection-post-stale-target",
            None,
            None,
            None,
            Some(MODEL_RUNTIME_SELECTION_REJECTED_CODE),
        ),
        row(
            "wp1.model_runtime.selection.post.failure.embedding_role",
            MODEL_RUNTIME_SELECTION_ROUTE,
            "embedding-role target",
            "mt014_selection_post_rejects_embedding_role_before_swap",
            "palmistry://wp1/model-runtime-registry/selection-post-embedding-role",
            None,
            None,
            None,
            Some(MODEL_RUNTIME_SELECTION_REJECTED_CODE),
        ),
        row(
            "wp1.model_runtime.selection.post.failure.integrity",
            MODEL_RUNTIME_SELECTION_ROUTE,
            "integrity failure",
            "mt014_selection_post_integrity_failure_occurs_before_swap",
            "palmistry://wp1/model-runtime-registry/selection-post-integrity",
            Some(MODEL_RUNTIME_REGISTRY_PROJECTION_SCHEMA_ID),
            None,
            None,
            Some(MODEL_RUNTIME_REGISTRY_INTEGRITY_ERROR_CODE),
        ),
        row(
            "wp1.model_runtime.selection.post.failure.invalid_input",
            MODEL_RUNTIME_SELECTION_ROUTE,
            "invalid target_model_id, actor, or reason",
            "model_runtime_selection_failure_recovery_rows_match_compiled_api_contract",
            "palmistry://wp1/model-runtime-registry/selection-post-invalid-input",
            None,
            None,
            None,
            Some(MODEL_RUNTIME_SELECTION_INVALID_CODE),
        ),
        row(
            "wp1.model_runtime.selection.post.failure.non_ready_target",
            MODEL_RUNTIME_SELECTION_ROUTE,
            "non-READY",
            "model_runtime_selection_failure_recovery_rows_match_compiled_api_contract",
            "palmistry://wp1/model-runtime-registry/selection-post-non-ready-target",
            None,
            None,
            None,
            Some(MODEL_RUNTIME_SELECTION_REJECTED_CODE),
        ),
        row(
            "wp1.model_runtime.selection.post.failure.timeout",
            MODEL_RUNTIME_SELECTION_ROUTE,
            "timeout keeps the prior active model",
            "model_runtime_selection_failure_recovery_rows_match_compiled_api_contract",
            "palmistry://wp1/model-runtime-registry/selection-post-timeout",
            None,
            None,
            None,
            Some(MODEL_RUNTIME_SELECTION_REJECTED_CODE),
        ),
        row(
            "wp1.model_runtime.selection.post.failure.unavailable",
            MODEL_RUNTIME_SELECTION_ROUTE,
            "503 MODEL_RUNTIME_REGISTRY_UNAVAILABLE",
            "model_runtime_selection_failure_recovery_rows_match_compiled_api_contract",
            "palmistry://wp1/model-runtime-registry/selection-post-unavailable",
            None,
            None,
            None,
            Some(MODEL_RUNTIME_REGISTRY_UNAVAILABLE_CODE),
        ),
        row(
            "wp1.model_runtime.selection.post.recovery.preserve_prior",
            MODEL_RUNTIME_SELECTION_ROUTE,
            "keeps the prior active model",
            "model_runtime_selection_failure_recovery_rows_match_compiled_api_contract",
            "palmistry://wp1/model-runtime-registry/selection-post-preserve-prior",
            None,
            None,
            None,
            None,
        ),
        row(
            "wp1.model_runtime.selection.post.recovery.reobserve",
            MODEL_RUNTIME_REGISTRY_ROUTE,
            "Refresh re-reads the durable projection",
            "mt014_stable_switch_author_id_posts_then_reobserves_backend_projection",
            "palmistry://wp1/model-runtime-registry/selection-post-reobserve",
            Some(MODEL_RUNTIME_REGISTRY_PROJECTION_SCHEMA_ID),
            None,
            None,
            None,
        ),
    ]
}

// These references are compile-time anchors only. They deliberately do not
// claim that the named async runtime proofs executed in this coverage check.
#[allow(dead_code)]
fn model_runtime_registry_compiled_surface_anchors() {
    let _ = ModelRegistryStore::persist_and_read_back;
    let _ = ModelRegistryStore::recover_configured_runtime_binding_set;
    let _ = ModelRegistryStore::persist_boot_set_and_read_back;
    let _ = ModelRegistryStore::rebind_selection_after_verified_unload;
    let _ = crate::api::model_runtime_registry::routes;
    let _ = MODEL_RUNTIME_SELECTION_INVALID_CODE;
    let _ = MODEL_RUNTIME_SELECTION_REJECTED_CODE;
    let _ = MODEL_RUNTIME_REGISTRY_INTEGRITY_ERROR_CODE;
    let _ = MODEL_RUNTIME_REGISTRY_UNAVAILABLE_CODE;
    let _ = KernelEventType::ModelRuntimeSelectionRecorded;
}

/// Verifies the declared MT-014 UserManual coverage contract against canonical
/// compiled symbols/constants plus the seeded PostgreSQL feature row. This does
/// not execute the declared runtime proof targets or query live runtime events.
pub fn verify_model_runtime_registry_behavior_coverage(
    rows: &[ModelRuntimeRegistryBehaviorCoverageRow],
    features: &[UserManualFeatureEntry],
) -> Result<(), Vec<BehaviorCoverageError>> {
    let mut errors = Vec::new();
    let Some(feature) = features
        .iter()
        .find(|feature| feature.feature_id == MODEL_RUNTIME_REGISTRY_MANUAL_FEATURE_ID)
    else {
        return Err(vec![BehaviorCoverageError {
            behavior_id: "wp1.model_runtime_registry.manual_entry",
            reason: format!(
                "UserManual feature {} missing",
                MODEL_RUNTIME_REGISTRY_MANUAL_FEATURE_ID
            ),
        }]);
    };

    if feature.manual_version != USER_MANUAL_VERSION {
        errors.push(BehaviorCoverageError {
            behavior_id: "wp1.model_runtime_registry.manual_version",
            reason: format!(
                "UserManual feature version {} does not match current {}",
                feature.manual_version, USER_MANUAL_VERSION
            ),
        });
    }
    if feature.content_hash.trim().is_empty() {
        errors.push(BehaviorCoverageError {
            behavior_id: "wp1.model_runtime_registry.manual_entry",
            reason: "UserManual feature content_hash missing".to_owned(),
        });
    }
    if !feature
        .description
        .contains(MODEL_RUNTIME_REGISTRY_DECLARED_PROOF_SCOPE)
    {
        errors.push(BehaviorCoverageError {
            behavior_id: "wp1.model_runtime_registry.proof_scope",
            reason: "UserManual must state that declared proof targets are not executed by coverage validation"
                .to_owned(),
        });
    }

    let mut behavior_ids = BTreeSet::new();
    for row in rows {
        if !behavior_ids.insert(row.behavior_id) {
            errors.push(BehaviorCoverageError {
                behavior_id: row.behavior_id,
                reason: "duplicate behavior_id".to_owned(),
            });
        }
        let expected_contract = match row.behavior_id {
            "wp1.model_runtime_registry.persistent_adapter_selection"
            | "wp1.model_runtime_registry.restart_recovery" => (
                Some(MODEL_RUNTIME_REGISTRY_SCHEMA_ID),
                Some(KernelEventType::ModelRuntimeSelectionRecorded.as_str()),
            ),
            "wp1.model_runtime_registry.fail_closed_selection_conflict" => {
                (Some(MODEL_RUNTIME_REGISTRY_SCHEMA_ID), None)
            }
            "wp1.model_runtime_registry.api_projection" => (
                Some(MODEL_RUNTIME_REGISTRY_PROJECTION_SCHEMA_ID),
                Some(KernelEventType::ModelRuntimeSelectionRecorded.as_str()),
            ),
            "wp1.model_runtime_registry.native_panel" => {
                (Some(MODEL_RUNTIME_REGISTRY_PROJECTION_SCHEMA_ID), None)
            }
            "wp1.model_runtime_registry.eventledger_selection_evidence" => (
                None,
                Some(KernelEventType::ModelRuntimeSelectionRecorded.as_str()),
            ),
            "wp1.model_runtime.selection.post.success"
            | "wp1.model_runtime.selection.post.failure.integrity"
            | "wp1.model_runtime.selection.post.recovery.reobserve" => {
                (Some(MODEL_RUNTIME_REGISTRY_PROJECTION_SCHEMA_ID), None)
            }
            "wp1.model_runtime.selection.post.failure.audit"
            | "wp1.model_runtime.selection.post.failure.stale_target"
            | "wp1.model_runtime.selection.post.failure.embedding_role"
            | "wp1.model_runtime.selection.post.failure.invalid_input"
            | "wp1.model_runtime.selection.post.failure.non_ready_target"
            | "wp1.model_runtime.selection.post.failure.timeout"
            | "wp1.model_runtime.selection.post.failure.unavailable"
            | "wp1.model_runtime.selection.post.recovery.preserve_prior" => (None, None),
            _ => {
                errors.push(BehaviorCoverageError {
                    behavior_id: row.behavior_id,
                    reason: "unknown MT-014 behavior has no canonical contract anchor".to_owned(),
                });
                continue;
            }
        };
        if (row.schema_id, row.eventledger_event_type) != expected_contract {
            errors.push(BehaviorCoverageError {
                behavior_id: row.behavior_id,
                reason: "behavior-specific schema/event anchor does not match the canonical compiled contract"
                    .to_owned(),
            });
        }
        let expected_runtime_surface = match row.behavior_id {
            "wp1.model_runtime_registry.persistent_adapter_selection" => {
                "ModelRegistryStore::persist_boot_set_and_read_back"
            }
            "wp1.model_runtime_registry.restart_recovery" => {
                "ModelRegistryStore::recover_configured_runtime_binding_set"
            }
            "wp1.model_runtime_registry.fail_closed_selection_conflict" => {
                "ModelRegistryStore::persist_boot_set_and_read_back"
            }
            "wp1.model_runtime_registry.api_projection" => {
                MODEL_RUNTIME_REGISTRY_ROUTE
            }
            "wp1.model_runtime_registry.native_panel" => {
                "PaneType::ModelRuntime + model-runtime.registry.*"
            }
            "wp1.model_runtime_registry.eventledger_selection_evidence" => {
                "ModelRegistryStore::persist_and_read_back -> KernelEventType::ModelRuntimeSelectionRecorded"
            }
            "wp1.model_runtime.selection.post.success"
            | "wp1.model_runtime.selection.post.failure.audit"
            | "wp1.model_runtime.selection.post.failure.stale_target"
            | "wp1.model_runtime.selection.post.failure.embedding_role"
            | "wp1.model_runtime.selection.post.failure.integrity"
            | "wp1.model_runtime.selection.post.failure.invalid_input"
            | "wp1.model_runtime.selection.post.failure.non_ready_target"
            | "wp1.model_runtime.selection.post.failure.timeout"
            | "wp1.model_runtime.selection.post.failure.unavailable"
            | "wp1.model_runtime.selection.post.recovery.preserve_prior" => {
                MODEL_RUNTIME_SELECTION_ROUTE
            }
            "wp1.model_runtime.selection.post.recovery.reobserve" => MODEL_RUNTIME_REGISTRY_ROUTE,
            _ => unreachable!("unknown behavior ids are rejected above"),
        };
        if row.runtime_surface_id != expected_runtime_surface {
            errors.push(BehaviorCoverageError {
                behavior_id: row.behavior_id,
                reason: format!(
                    "runtime surface `{}` does not match canonical `{expected_runtime_surface}`",
                    row.runtime_surface_id
                ),
            });
        }
        let expected_response_code = match row.behavior_id {
            "wp1.model_runtime.selection.post.failure.invalid_input" => {
                Some(MODEL_RUNTIME_SELECTION_INVALID_CODE)
            }
            "wp1.model_runtime.selection.post.failure.audit"
            | "wp1.model_runtime.selection.post.failure.stale_target"
            | "wp1.model_runtime.selection.post.failure.embedding_role"
            | "wp1.model_runtime.selection.post.failure.non_ready_target"
            | "wp1.model_runtime.selection.post.failure.timeout" => {
                Some(MODEL_RUNTIME_SELECTION_REJECTED_CODE)
            }
            "wp1.model_runtime.selection.post.failure.integrity" => {
                Some(MODEL_RUNTIME_REGISTRY_INTEGRITY_ERROR_CODE)
            }
            "wp1.model_runtime.selection.post.failure.unavailable" => {
                Some(MODEL_RUNTIME_REGISTRY_UNAVAILABLE_CODE)
            }
            _ => None,
        };
        if row.response_code != expected_response_code {
            errors.push(BehaviorCoverageError {
                behavior_id: row.behavior_id,
                reason: format!(
                    "response code {:?} does not match canonical compiled response code {expected_response_code:?}",
                    row.response_code
                ),
            });
        }
        match (row.eventledger_event_type, row.eventledger_evidence_path) {
            (Some(event_type), Some(path))
                if event_type == KernelEventType::ModelRuntimeSelectionRecorded.as_str()
                    && path.ends_with(event_type) => {}
            (None, None) => {}
            _ => {
                errors.push(BehaviorCoverageError {
                    behavior_id: row.behavior_id,
                    reason: "declared EventLedger evidence must use the canonical typed event and matching path, or be explicitly absent"
                        .to_owned(),
                });
            }
        }
        if row.manual_feature_id != feature.feature_id || row.manual_version != USER_MANUAL_VERSION
        {
            errors.push(BehaviorCoverageError {
                behavior_id: row.behavior_id,
                reason: "coverage row does not target the current MT-014 UserManual entry/version"
                    .to_owned(),
            });
        }
        if !feature
            .tool_ids
            .iter()
            .any(|tool| tool == row.proof_tool_id)
        {
            errors.push(BehaviorCoverageError {
                behavior_id: row.behavior_id,
                reason: format!(
                    "UserManual feature does not declare proof target {}",
                    row.proof_tool_id
                ),
            });
        }
        if row.proof_execution_status != ModelRuntimeProofExecutionStatus::DeclaredNotExecuted {
            errors.push(BehaviorCoverageError {
                behavior_id: row.behavior_id,
                reason: "UserManual coverage must not claim that runtime proof targets executed"
                    .to_owned(),
            });
        }
        for marker in [row.manual_evidence_marker, row.recovery_instruction_marker] {
            if !feature.description.contains(marker) {
                errors.push(BehaviorCoverageError {
                    behavior_id: row.behavior_id,
                    reason: format!("UserManual feature is missing required text `{marker}`"),
                });
            }
        }
        if row.internal_diagnostics_posture != DiagnosticTierPosture::Wired
            || row.palmistry_posture != DiagnosticTierPosture::Wired
            || row.diagnostic_reason.trim().is_empty()
        {
            errors.push(BehaviorCoverageError {
                behavior_id: row.behavior_id,
                reason: "declared HBR-INT-009 wired diagnostic posture is incomplete".to_owned(),
            });
        }
    }

    for marker in [
        "Tier-1 Flight Recorder events are WIRED",
        "internal_diagnostics is WIRED",
        "Palmistry is WIRED",
    ] {
        if !feature.description.contains(marker) {
            errors.push(BehaviorCoverageError {
                behavior_id: "wp1.model_runtime_registry.hbr_int_009",
                reason: format!(
                    "UserManual feature is missing declared HBR-INT-009 text `{marker}`"
                ),
            });
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

pub fn model_lane_behavior_coverage_matrix(
    schema_registry: &[ModelLaneSchemaRegistryRow],
) -> Result<Vec<BehaviorCoverageRow>, BehaviorCoverageError> {
    let mut templates = vec![
        BehaviorCoverageRow {
            behavior_id: "wp1.model_lane.run",
            schema_id: Some("hsk.model_lane_run@1"),
            event_family: "model_lane_run",
            runtime_surface_id: "ModelLaneStore::record_run",
            user_manual_slug: "model-lane-schema",
            tool_id: "model_lane_schema_pg_tests",
            eventledger_flight_recorder_path: "kernel_event_ledger:model_lane_run",
            internal_diagnostics_posture: DiagnosticTierPosture::Wired,
            palmistry_posture: DiagnosticTierPosture::Wired,
            deferred_reason: Some(
                "Tier-2 internal_diagnostics and the authenticated Tier-3 Palmistry watcher are wired observers; they do not replace the row's EventLedger/Flight Recorder authority.",
            ),
            follow_up_ref: None,
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
            palmistry_posture: DiagnosticTierPosture::Wired,
            deferred_reason: Some(
                "Tier-2 internal_diagnostics and the authenticated Tier-3 Palmistry watcher are wired observers; they do not replace the row's EventLedger/Flight Recorder authority.",
            ),
            follow_up_ref: None,
        },
        BehaviorCoverageRow {
            behavior_id: "wp1.model_lane.official_cli_spawn",
            schema_id: Some("hsk.model_lane@1"),
            event_family: "process_ownership_lifecycle",
            runtime_surface_id: "LiveCliSpawner::spawn",
            user_manual_slug: "model-lane-launch-adapters",
            tool_id: "official_cli_attached_lifecycle_tests",
            eventledger_flight_recorder_path:
                "kernel_process_lifecycle:official_cli_bridge START/STOP",
            internal_diagnostics_posture: DiagnosticTierPosture::Wired,
            palmistry_posture: DiagnosticTierPosture::Wired,
            deferred_reason: Some(
                "Tier-2 internal_diagnostics and the authenticated Tier-3 Palmistry watcher are wired observers; they do not replace the row's EventLedger/Flight Recorder authority.",
            ),
            follow_up_ref: None,
        },
        BehaviorCoverageRow {
            behavior_id: "wp1.model_lane.official_cli_attached_sandbox",
            schema_id: Some("hsk.model_lane@1"),
            event_family: "process_ownership_lifecycle",
            runtime_surface_id: "HandshakeNativeSandboxAdapter::spawn_attached_with_stdio",
            user_manual_slug: "model-lane-launch-adapters",
            tool_id: "official_cli_attached_lifecycle_tests",
            eventledger_flight_recorder_path:
                "kernel_process_lifecycle:official_cli_bridge START/STOP",
            internal_diagnostics_posture: DiagnosticTierPosture::Wired,
            palmistry_posture: DiagnosticTierPosture::Wired,
            deferred_reason: Some(
                "Tier-2 internal_diagnostics and the authenticated Tier-3 Palmistry watcher are wired observers; they do not replace the row's EventLedger/Flight Recorder authority.",
            ),
            follow_up_ref: None,
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
            palmistry_posture: DiagnosticTierPosture::Wired,
            deferred_reason: Some(
                "Tier-2 internal_diagnostics and the authenticated Tier-3 Palmistry watcher are wired observers; they do not replace the row's EventLedger/Flight Recorder authority.",
            ),
            follow_up_ref: None,
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
            palmistry_posture: DiagnosticTierPosture::Wired,
            deferred_reason: Some(
                "Tier-2 internal_diagnostics and the authenticated Tier-3 Palmistry watcher are wired observers; they do not replace the row's EventLedger/Flight Recorder authority.",
            ),
            follow_up_ref: None,
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
            palmistry_posture: DiagnosticTierPosture::Wired,
            deferred_reason: Some(
                "Tier-2 internal_diagnostics and the authenticated Tier-3 Palmistry watcher are wired observers; they do not replace the row's EventLedger/Flight Recorder authority.",
            ),
            follow_up_ref: None,
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
            palmistry_posture: DiagnosticTierPosture::Wired,
            deferred_reason: Some(
                "Tier-2 internal_diagnostics and the authenticated Tier-3 Palmistry watcher are wired observers; they do not replace the row's EventLedger/Flight Recorder authority.",
            ),
            follow_up_ref: None,
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
            palmistry_posture: DiagnosticTierPosture::Wired,
            deferred_reason: Some(
                "Tier-2 internal_diagnostics and the authenticated Tier-3 Palmistry watcher are wired observers; they do not replace the row's EventLedger/Flight Recorder authority.",
            ),
            follow_up_ref: None,
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
            palmistry_posture: DiagnosticTierPosture::Wired,
            deferred_reason: Some(
                "Tier-2 internal_diagnostics and the authenticated Tier-3 Palmistry watcher are wired observers; they do not replace the row's EventLedger/Flight Recorder authority.",
            ),
            follow_up_ref: None,
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
            palmistry_posture: DiagnosticTierPosture::Wired,
            deferred_reason: Some(
                "Tier-2 internal_diagnostics and the authenticated Tier-3 Palmistry watcher are wired observers; they do not replace the row's EventLedger/Flight Recorder authority.",
            ),
            follow_up_ref: None,
        },
        BehaviorCoverageRow {
            behavior_id: "wp1.model_lane.cloud_consent_denial",
            schema_id: Some("hsk.model_lane_cloud_consent_denial@1"),
            event_family: "model_lane_cloud_consent_denial",
            runtime_surface_id: "ModelLaneStore",
            user_manual_slug: "model-lane-cloud-projection-consent",
            tool_id: "cloud_model_lane_policy_pg_tests",
            eventledger_flight_recorder_path: "kernel_event_ledger:model_lane_cloud_consent_denial",
            internal_diagnostics_posture: DiagnosticTierPosture::Wired,
            palmistry_posture: DiagnosticTierPosture::Wired,
            deferred_reason: Some(
                "Tier-2 internal_diagnostics and the authenticated Tier-3 Palmistry watcher are wired observers; they do not replace the row's EventLedger/Flight Recorder authority.",
            ),
            follow_up_ref: None,
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
            palmistry_posture: DiagnosticTierPosture::Wired,
            deferred_reason: Some(
                "Tier-2 internal_diagnostics and the authenticated Tier-3 Palmistry watcher are wired observers; they do not replace the row's EventLedger/Flight Recorder authority.",
            ),
            follow_up_ref: None,
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
            palmistry_posture: DiagnosticTierPosture::Wired,
            deferred_reason: Some(
                "Tier-2 internal_diagnostics and the authenticated Tier-3 Palmistry watcher are wired observers; they do not replace the row's EventLedger/Flight Recorder authority.",
            ),
            follow_up_ref: None,
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
            palmistry_posture: DiagnosticTierPosture::Wired,
            deferred_reason: Some(
                "Tier-2 internal_diagnostics and the authenticated Tier-3 Palmistry watcher are wired observers; they do not replace the row's EventLedger/Flight Recorder authority.",
            ),
            follow_up_ref: None,
        },
        BehaviorCoverageRow {
            behavior_id: "wp1.model_lane.diagnostics",
            schema_id: Some("hsk.model_lane_diagnostic_tier@1"),
            event_family: "model_lane_diagnostic_tier",
            runtime_surface_id: "ModelLaneDiagnosticsProjection",
            user_manual_slug: "model-lane-diagnostics",
            tool_id: "swarm_lane_diagnostics_runtime_proof",
            eventledger_flight_recorder_path: "kernel_event_ledger:model_lane_diagnostic_tier",
            internal_diagnostics_posture: DiagnosticTierPosture::Wired,
            palmistry_posture: DiagnosticTierPosture::Wired,
            deferred_reason: Some(
                "Tier-2 internal_diagnostics and the authenticated Tier-3 Palmistry watcher are wired observers; they do not replace the row's EventLedger/Flight Recorder authority.",
            ),
            follow_up_ref: None,
        },
        BehaviorCoverageRow {
            behavior_id: "wp1.model_lane.mixed_validation",
            schema_id: Some("hsk.model_lane_mt_runtime_status@1"),
            event_family: "model_lane_mt_runtime_status",
            runtime_surface_id: "ModelLaneStore",
            user_manual_slug: "model-lane-validation-harness",
            tool_id: "mixed_model_lane_integration_pg_tests",
            eventledger_flight_recorder_path: "kernel_event_ledger:model_lane_mt_runtime_status",
            internal_diagnostics_posture: DiagnosticTierPosture::Wired,
            palmistry_posture: DiagnosticTierPosture::Wired,
            deferred_reason: Some(
                "Tier-2 internal_diagnostics and the authenticated Tier-3 Palmistry watcher are wired observers; they do not replace the row's EventLedger/Flight Recorder authority.",
            ),
            follow_up_ref: None,
        },
        BehaviorCoverageRow {
            behavior_id: "wp1.model_lane.cloud_projection_plan_v2",
            schema_id: Some("hsk.model_lane_cloud_projection_plan@2"),
            event_family: "model_lane_cloud_projection_plan",
            runtime_surface_id: "ModelLaneStore::record_cloud_projection_plan",
            user_manual_slug: "model-lane-cloud-projection-consent",
            tool_id: "cloud_model_lane_policy_pg_tests",
            eventledger_flight_recorder_path:
                "kernel_event_ledger:model_lane_cloud_projection_plan",
            internal_diagnostics_posture: DiagnosticTierPosture::Wired,
            palmistry_posture: DiagnosticTierPosture::Wired,
            deferred_reason: Some(
                "Tier-2 internal_diagnostics and the authenticated Tier-3 Palmistry watcher are wired observers; they do not replace the row's EventLedger/Flight Recorder authority.",
            ),
            follow_up_ref: None,
        },
        BehaviorCoverageRow {
            behavior_id: "wp1.model_lane.cloud_consent_v2",
            schema_id: Some("hsk.model_lane_cloud_consent_receipt@2"),
            event_family: "model_lane_cloud_consent_receipt",
            runtime_surface_id: "ModelLaneStore::record_cloud_consent_receipt",
            user_manual_slug: "model-lane-cloud-projection-consent",
            tool_id: "cloud_model_lane_policy_pg_tests",
            eventledger_flight_recorder_path:
                "kernel_event_ledger:model_lane_cloud_consent_receipt",
            internal_diagnostics_posture: DiagnosticTierPosture::Wired,
            palmistry_posture: DiagnosticTierPosture::Wired,
            deferred_reason: Some(
                "Tier-2 internal_diagnostics and the authenticated Tier-3 Palmistry watcher are wired observers; they do not replace the row's EventLedger/Flight Recorder authority.",
            ),
            follow_up_ref: None,
        },
        BehaviorCoverageRow {
            behavior_id: "wp1.model_lane.recovery_event_v2",
            schema_id: Some("hsk.model_lane_recovery_event@2"),
            event_family: "model_lane_recovery_event",
            runtime_surface_id: "ModelLaneStore::record_recovery_event",
            user_manual_slug: "model-lane-recovery",
            tool_id: "model_lane_recovery_pg_tests",
            eventledger_flight_recorder_path: "kernel_event_ledger:model_lane_recovery_event",
            internal_diagnostics_posture: DiagnosticTierPosture::Wired,
            palmistry_posture: DiagnosticTierPosture::Wired,
            deferred_reason: Some(
                "Tier-2 internal_diagnostics and the authenticated Tier-3 Palmistry watcher are wired observers; they do not replace the row's EventLedger/Flight Recorder authority.",
            ),
            follow_up_ref: None,
        },
        BehaviorCoverageRow {
            behavior_id: "wp1.model_lane.routing_execution",
            schema_id: Some("hsk.model_lane_routing_execution@5"),
            event_family: "model_lane_routing_execution",
            runtime_surface_id: "ModelLaneRoutingExecutionStore",
            user_manual_slug: "operator-chat-launch",
            tool_id: "mixed_model_lane_integration_pg_tests",
            eventledger_flight_recorder_path: "kernel_event_ledger:model_lane_routing_execution",
            internal_diagnostics_posture: DiagnosticTierPosture::Wired,
            palmistry_posture: DiagnosticTierPosture::Wired,
            deferred_reason: Some(
                "Tier-2 internal_diagnostics and the authenticated Tier-3 Palmistry watcher are wired observers; they do not replace the row's EventLedger/Flight Recorder authority.",
            ),
            follow_up_ref: None,
        },
        BehaviorCoverageRow {
            behavior_id: "wp1.model_lane.routing_outbox",
            schema_id: Some("hsk.model_lane_routing_outbox@4"),
            event_family: "model_lane_routing_outbox",
            runtime_surface_id: "ModelLaneRoutingExecutionStore",
            user_manual_slug: "operator-chat-launch",
            tool_id: "mixed_model_lane_integration_pg_tests",
            eventledger_flight_recorder_path: "kernel_event_ledger:model_lane_routing_outbox",
            internal_diagnostics_posture: DiagnosticTierPosture::Wired,
            palmistry_posture: DiagnosticTierPosture::Wired,
            deferred_reason: Some(
                "Tier-2 internal_diagnostics and the authenticated Tier-3 Palmistry watcher are wired observers; they do not replace the row's EventLedger/Flight Recorder authority.",
            ),
            follow_up_ref: None,
        },
        BehaviorCoverageRow {
            behavior_id: "wp1.model_lane.routing_stage_attempt",
            schema_id: Some("hsk.model_lane_routing_stage_attempt@4"),
            event_family: "model_lane_routing_stage_attempt",
            runtime_surface_id: "ModelLaneRoutingExecutionStore",
            user_manual_slug: "operator-chat-launch",
            tool_id: "mixed_model_lane_integration_pg_tests",
            eventledger_flight_recorder_path:
                "kernel_event_ledger:model_lane_routing_stage_attempt",
            internal_diagnostics_posture: DiagnosticTierPosture::Wired,
            palmistry_posture: DiagnosticTierPosture::Wired,
            deferred_reason: Some(
                "Tier-2 internal_diagnostics and the authenticated Tier-3 Palmistry watcher are wired observers; they do not replace the row's EventLedger/Flight Recorder authority.",
            ),
            follow_up_ref: None,
        },
        BehaviorCoverageRow {
            behavior_id: "wp1.model_lane.run_extension",
            schema_id: Some("hsk.model_lane_run_extension@1"),
            event_family: "model_lane_run_extension",
            runtime_surface_id: "ModelLaneRoutingExecutionStore",
            user_manual_slug: "operator-chat-launch",
            tool_id: "mixed_model_lane_integration_pg_tests",
            eventledger_flight_recorder_path: "kernel_event_ledger:model_lane_run_extension",
            internal_diagnostics_posture: DiagnosticTierPosture::Wired,
            palmistry_posture: DiagnosticTierPosture::Wired,
            deferred_reason: Some(
                "Tier-2 internal_diagnostics and the authenticated Tier-3 Palmistry watcher are wired observers; they do not replace the row's EventLedger/Flight Recorder authority.",
            ),
            follow_up_ref: None,
        },
    ];

    for row in &mut templates {
        row.deferred_reason = Some(
            "Tier-2 internal_diagnostics is produced by the native panic/heartbeat/frame/resource/open-event ring and Problems projection. Tier-3 Palmistry is the authenticated separate watcher with durable survivor recovery. The lane pane consumes their observation evidence but does not replace either producer.",
        );
    }

    let mut seen_schema_ids = BTreeSet::new();
    for schema in schema_registry {
        if !seen_schema_ids.insert(schema.schema_id.as_str()) {
            return Err(BehaviorCoverageError {
                behavior_id: "wp1.model_lane.schema_registry",
                reason: format!(
                    "duplicate ModelLane schema registry row {}",
                    schema.schema_id
                ),
            });
        }
        let Some(_) = templates
            .iter()
            .find(|row| row.schema_id == Some(schema.schema_id.as_str()))
        else {
            return Err(BehaviorCoverageError {
                behavior_id: "wp1.model_lane.schema_registry",
                reason: format!(
                    "registered schema {} has no compile-linked UserManual behavior coverage template",
                    schema.schema_id
                ),
            });
        };
    }
    Ok(templates
        .into_iter()
        .filter(|row| {
            row.schema_id
                .is_some_and(|schema_id| seen_schema_ids.contains(schema_id))
        })
        .collect())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CanonicalBehaviorFamily {
    EmbeddedModel,
    OperatorChat,
    CloudModelAccess,
    DedicatedEmbedding,
}

#[derive(Debug, Clone)]
struct CanonicalBehaviorDescriptor {
    family: CanonicalBehaviorFamily,
    row: BehaviorCoverageRow,
}

/// Canonical non-schema behavior/event/API registry. Internal event behaviors
/// are declared once here. Shipped Operator Chat and Model Access HTTP behavior
/// rows are generated from `wp009_surface_registry`, so a new route in either
/// product family cannot silently escape the corresponding behavior matrix.
fn canonical_non_schema_behavior_registry() -> Vec<CanonicalBehaviorDescriptor> {
    const WIRED_REASON: &str =
        "Flight Recorder/EventLedger is authoritative; native internal_diagnostics and the \
         authenticated Palmistry watcher are wired and observe these rows without becoming their \
         authority.";
    const NOT_APPLICABLE_REASON: &str =
        "Cloud access is a Settings/keychain/API surface, not a ModelLane runtime lane; behavior is \
         verified by route, OS-keychain leak, and native Argus proof surfaces.";

    let mut rows = vec![
        CanonicalBehaviorDescriptor { family: CanonicalBehaviorFamily::EmbeddedModel, row: BehaviorCoverageRow {
            behavior_id: "wp1.embedded_model.ledger_start",
            schema_id: None,
            event_family: "kernel_process_lifecycle_start",
            runtime_surface_id: "EmbeddedModelProcess",
            user_manual_slug: "embedded-model-lifecycle-ledger",
            tool_id: "embedded_model_ledger_tests",
            eventledger_flight_recorder_path: "kernel_process_lifecycle:embedded_model_start",
            internal_diagnostics_posture: DiagnosticTierPosture::Wired,
            palmistry_posture: DiagnosticTierPosture::Wired,
            deferred_reason: Some(WIRED_REASON),
            follow_up_ref: None,
        }},
        CanonicalBehaviorDescriptor { family: CanonicalBehaviorFamily::EmbeddedModel, row: BehaviorCoverageRow {
            behavior_id: "wp1.embedded_model.ledger_stop",
            schema_id: None,
            event_family: "kernel_process_lifecycle_stop",
            runtime_surface_id: "EmbeddedModelProcess::shutdown",
            user_manual_slug: "embedded-model-lifecycle-ledger",
            tool_id: "embedded_model_ledger_tests",
            eventledger_flight_recorder_path: "kernel_process_lifecycle:embedded_model_stop",
            internal_diagnostics_posture: DiagnosticTierPosture::Wired,
            palmistry_posture: DiagnosticTierPosture::Wired,
            deferred_reason: Some(WIRED_REASON),
            follow_up_ref: None,
        }},
        CanonicalBehaviorDescriptor { family: CanonicalBehaviorFamily::EmbeddedModel, row: BehaviorCoverageRow {
            behavior_id: "wp1.embedded_model.os_lease_reclaim",
            schema_id: None,
            event_family: "kernel_process_lifecycle_stop",
            runtime_surface_id: "reclaim_pidless_embedded_orphans",
            user_manual_slug: "embedded-model-lifecycle-ledger",
            tool_id: "embedded_model_ledger_tests",
            eventledger_flight_recorder_path:
                "kernel_process_lifecycle:orphan_reclaim_pidless_embedded_boot",
            internal_diagnostics_posture: DiagnosticTierPosture::Wired,
            palmistry_posture: DiagnosticTierPosture::Wired,
            deferred_reason: Some(WIRED_REASON),
            follow_up_ref: None,
        }},
        CanonicalBehaviorDescriptor { family: CanonicalBehaviorFamily::EmbeddedModel, row: BehaviorCoverageRow {
            behavior_id: "wp1.llm.fail_closed_fr",
            schema_id: None,
            event_family: "llm_inference",
            runtime_surface_id: "DisabledLlmClient::completion",
            user_manual_slug: "embedded-model-lifecycle-ledger",
            tool_id: "llm_client_local_routing_tests",
            eventledger_flight_recorder_path: "flight_recorder:llm_inference_fail_closed",
            internal_diagnostics_posture: DiagnosticTierPosture::Wired,
            palmistry_posture: DiagnosticTierPosture::Wired,
            deferred_reason: Some(WIRED_REASON),
            follow_up_ref: None,
        }},
        CanonicalBehaviorDescriptor { family: CanonicalBehaviorFamily::EmbeddedModel, row: BehaviorCoverageRow {
            behavior_id: "wp1.llm.embedding_fr",
            schema_id: None,
            event_family: "data_embedding_computed",
            runtime_surface_id: "LlmClient::embedding",
            user_manual_slug: "embedded-model-lifecycle-ledger",
            tool_id: "llm_client_local_routing_tests",
            eventledger_flight_recorder_path: "flight_recorder:data_embedding_computed",
            internal_diagnostics_posture: DiagnosticTierPosture::Wired,
            palmistry_posture: DiagnosticTierPosture::Wired,
            deferred_reason: Some(WIRED_REASON),
            follow_up_ref: None,
        }},
        CanonicalBehaviorDescriptor { family: CanonicalBehaviorFamily::OperatorChat, row: BehaviorCoverageRow {
            behavior_id: "wp1.operator_chat.capture_message",
            schema_id: None,
            event_family: "model_lane_message",
            runtime_surface_id: "ModelLaneCaptureRecorder::capture_cli_stream",
            user_manual_slug: "operator-chat-launch",
            tool_id: "operator_chat_capture_tests",
            eventledger_flight_recorder_path: "flight_recorder:model_lane_message",
            internal_diagnostics_posture: DiagnosticTierPosture::Wired,
            palmistry_posture: DiagnosticTierPosture::Wired,
            deferred_reason: Some(WIRED_REASON),
            follow_up_ref: None,
        }},
        CanonicalBehaviorDescriptor { family: CanonicalBehaviorFamily::OperatorChat, row: BehaviorCoverageRow {
            behavior_id: "wp1.operator_chat.agent_activity_fr",
            schema_id: None,
            event_family: "agent_activity",
            runtime_surface_id: "agent_activity_event",
            user_manual_slug: "operator-chat-launch",
            tool_id: "operator_chat_capture_tests",
            eventledger_flight_recorder_path: "flight_recorder:agent_activity",
            internal_diagnostics_posture: DiagnosticTierPosture::Wired,
            palmistry_posture: DiagnosticTierPosture::Wired,
            deferred_reason: Some(WIRED_REASON),
            follow_up_ref: None,
        }},
        CanonicalBehaviorDescriptor { family: CanonicalBehaviorFamily::CloudModelAccess, row: BehaviorCoverageRow {
            behavior_id: "wp1.cloud_access.secret_leak_guard",
            schema_id: None,
            event_family: "cloud_access_byok_secret_leak_guard",
            runtime_surface_id: "api::model_access::routes",
            user_manual_slug: "cloud-model-access",
            tool_id: "cloud_byok_access_config_leak_tests",
            eventledger_flight_recorder_path:
                "os_keychain:OsKeychainSecretsVault + cloud_invocation_audit + tracing_capture",
            internal_diagnostics_posture: DiagnosticTierPosture::NotApplicableWithReason,
            palmistry_posture: DiagnosticTierPosture::NotApplicableWithReason,
            deferred_reason: Some(NOT_APPLICABLE_REASON),
            follow_up_ref: None,
        }},
        CanonicalBehaviorDescriptor { family: CanonicalBehaviorFamily::CloudModelAccess, row: BehaviorCoverageRow {
            behavior_id: "wp1.cloud_access.settings_argus",
            schema_id: None,
            event_family: "cloud_models_settings_accesskit",
            runtime_surface_id: "api::model_access::routes",
            user_manual_slug: "cloud-model-access",
            tool_id: "test_cloud_models_settings_argus",
            eventledger_flight_recorder_path:
                "settings_argus:Cloud Models AccessKit tree + static provider fallback + key-buffer wipe",
            internal_diagnostics_posture: DiagnosticTierPosture::NotApplicableWithReason,
            palmistry_posture: DiagnosticTierPosture::NotApplicableWithReason,
            deferred_reason: Some(NOT_APPLICABLE_REASON),
            follow_up_ref: None,
        }},
        CanonicalBehaviorDescriptor { family: CanonicalBehaviorFamily::CloudModelAccess, row: BehaviorCoverageRow {
            behavior_id: "wp1.cloud_access.cli_bridge_login",
            schema_id: None,
            event_family: "cloud_models_cli_bridge_login",
            runtime_surface_id: "api::model_access::routes",
            user_manual_slug: "cloud-model-access",
            tool_id: "test_cloud_models_settings_argus",
            eventledger_flight_recorder_path:
                "settings_argus:official_cli_bridge_login + provider_owned_login_command",
            internal_diagnostics_posture: DiagnosticTierPosture::NotApplicableWithReason,
            palmistry_posture: DiagnosticTierPosture::NotApplicableWithReason,
            deferred_reason: Some(NOT_APPLICABLE_REASON),
            follow_up_ref: None,
        }},
        CanonicalBehaviorDescriptor { family: CanonicalBehaviorFamily::DedicatedEmbedding, row: BehaviorCoverageRow {
        behavior_id: "wp1.llm.dedicated_embedding_model",
        schema_id: None,
        event_family: "data_embedding_computed",
        runtime_surface_id: "ModelCatalog::embedding_model_for_dim",
        user_manual_slug: "dedicated-embedding-model-routing",
        tool_id: "dedicated_embedding_model_tests",
        eventledger_flight_recorder_path:
            "flight_recorder:data_embedding_computed + loom_block_search_index.embedding_model",
            internal_diagnostics_posture: DiagnosticTierPosture::Wired,
            palmistry_posture: DiagnosticTierPosture::Wired,
        deferred_reason: Some(
            "MT-016 wires the authoritative runtime/catalog/Loom path; native internal_diagnostics and the authenticated Palmistry watcher are wired observers of these selected-model receipts without becoming their authority.",
        ),
        follow_up_ref: None,
    }},
    ];

    rows.extend(wp009_surface_registry().iter().filter_map(|surface| {
        let (family, tool_id, posture, reason, follow_up_ref) = match surface.group {
            SurfaceGroup::OperatorChat => (
                CanonicalBehaviorFamily::OperatorChat,
                "operator_chat_capture_tests",
                DiagnosticTierPosture::Wired,
                WIRED_REASON,
                Some("palmistry://wp1/operator-chat/http-routes"),
            ),
            SurfaceGroup::ModelAccess => (
                CanonicalBehaviorFamily::CloudModelAccess,
                "model_access_route_tests",
                DiagnosticTierPosture::NotApplicableWithReason,
                NOT_APPLICABLE_REASON,
                None,
            ),
            _ => return None,
        };
        Some(CanonicalBehaviorDescriptor {
            family,
            row: BehaviorCoverageRow {
                behavior_id: surface.surface_id,
                schema_id: None,
                event_family: surface.surface_id,
                runtime_surface_id: surface.route,
                user_manual_slug: surface.group.page_slug(),
                tool_id,
                eventledger_flight_recorder_path: surface.route,
                internal_diagnostics_posture: posture,
                palmistry_posture: posture,
                deferred_reason: Some(reason),
                follow_up_ref,
            },
        })
    }));
    rows
}

fn behavior_matrix(family: CanonicalBehaviorFamily) -> Vec<BehaviorCoverageRow> {
    canonical_non_schema_behavior_registry()
        .into_iter()
        .filter(|descriptor| descriptor.family == family)
        .map(|descriptor| descriptor.row)
        .collect()
}

pub fn embedded_model_behavior_coverage_matrix() -> Vec<BehaviorCoverageRow> {
    behavior_matrix(CanonicalBehaviorFamily::EmbeddedModel)
}

pub fn operator_chat_launch_behavior_coverage_matrix() -> Vec<BehaviorCoverageRow> {
    behavior_matrix(CanonicalBehaviorFamily::OperatorChat)
}

pub fn cloud_model_access_behavior_coverage_matrix() -> Vec<BehaviorCoverageRow> {
    behavior_matrix(CanonicalBehaviorFamily::CloudModelAccess)
}

pub fn dedicated_embedding_model_behavior_coverage_matrix() -> Vec<BehaviorCoverageRow> {
    behavior_matrix(CanonicalBehaviorFamily::DedicatedEmbedding)
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
        if let Err(consistency_errors) = row.self_consistency_result(&[], pages, tools) {
            errors.extend(consistency_errors);
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
        let is_registry_route = wp009_surface_registry().iter().any(|surface| {
            surface.group == SurfaceGroup::ModelAccess
                && surface.surface_id == row.behavior_id
                && surface.route == path
        });
        if path.is_empty()
            || (!is_registry_route
                && ![
                    "http_route:/model-access",
                    "os_keychain:",
                    "settings_argus:",
                    "cloud_invocation_audit",
                ]
                .iter()
                .any(|marker| path.contains(marker)))
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
/// durable surface), internal_diagnostics + Palmistry WIRED.
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
        if let Err(consistency_errors) = row.self_consistency_result(&[], pages, tools) {
            errors.extend(consistency_errors);
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
        let is_registry_route = wp009_surface_registry().iter().any(|surface| {
            surface.group == SurfaceGroup::OperatorChat
                && surface.surface_id == row.behavior_id
                && surface.route == path
        });
        if path.is_empty()
            || !(path.contains("kernel_process_lifecycle")
                || path.contains("flight_recorder")
                || is_registry_route)
        {
            errors.push(BehaviorCoverageError {
                behavior_id: row.behavior_id,
                reason: "EventLedger/FlightRecorder evidence path missing or not WIRED to a durable surface".to_owned(),
            });
        }
        // MT-013 posture: internal_diagnostics is wired.
        if row.internal_diagnostics_posture != DiagnosticTierPosture::Wired {
            errors.push(BehaviorCoverageError {
                behavior_id: row.behavior_id,
                reason: format!(
                    "internal_diagnostics posture must be WIRED for MT-013, got {}",
                    row.internal_diagnostics_posture.as_str()
                ),
            });
        }
        // MT-013 posture: Palmistry is wired.
        if row.palmistry_posture != DiagnosticTierPosture::Wired {
            errors.push(BehaviorCoverageError {
                behavior_id: row.behavior_id,
                reason: format!(
                    "Palmistry posture must be WIRED for MT-013, got {}",
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
        if let Err(consistency_errors) = row.self_consistency_result(schema_registry, pages, tools)
        {
            errors.extend(consistency_errors);
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
            || !(row
                .eventledger_flight_recorder_path
                .contains("kernel_event_ledger")
                || row
                    .eventledger_flight_recorder_path
                    .contains("kernel_process_lifecycle")
                || row
                    .eventledger_flight_recorder_path
                    .contains("flight_recorder"))
        {
            errors.push(BehaviorCoverageError {
                behavior_id: row.behavior_id,
                reason: "EventLedger/FlightRecorder evidence path missing".to_owned(),
            });
        }
        if row.internal_diagnostics_posture != DiagnosticTierPosture::Wired
            || row.palmistry_posture != DiagnosticTierPosture::Wired
        {
            errors.push(BehaviorCoverageError {
                behavior_id: row.behavior_id,
                reason: format!(
                    "internal_diagnostics and Palmistry postures must be WIRED, got {}/{}",
                    row.internal_diagnostics_posture.as_str(),
                    row.palmistry_posture.as_str()
                ),
            });
        }
        if (row.internal_diagnostics_posture == DiagnosticTierPosture::DeferredWithReason
            || row.palmistry_posture == DiagnosticTierPosture::DeferredWithReason)
            && (row.deferred_reason.is_none() || row.follow_up_ref.is_none())
        {
            errors.push(BehaviorCoverageError {
                behavior_id: row.behavior_id,
                reason: "DEFERRED-with-reason diagnostics tiers require deferred_reason and follow_up_ref"
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

/// Runtime evidence gate for the shared HBR-INT-009 posture declared by the statically-covered
/// ModelLane behaviors. The runtime producers intentionally write one correlated three-tier
/// HBR-INT-009 envelope per run; they do not fabricate a duplicate tier triplet for every manual
/// behavior row. Each row must declare both live tiers as `WIRED`, and that shared declaration is
/// established only by the exact run's durable HBR-INT-009 records validated by `ModelLaneStore`.
pub async fn verify_model_lane_behavior_evidence(
    store: &ModelLaneStore,
    run_id: &str,
    rows: &[BehaviorCoverageRow],
) -> Result<Vec<ModelLaneDiagnosticTierPosture>, Vec<BehaviorCoverageError>> {
    let mut errors = Vec::new();
    for row in rows {
        if row.internal_diagnostics_posture != DiagnosticTierPosture::Wired
            || row.palmistry_posture != DiagnosticTierPosture::Wired
        {
            errors.push(BehaviorCoverageError {
                behavior_id: row.behavior_id,
                reason: "behavior does not declare the shared HBR-INT-009 internal_diagnostics and Palmistry tiers as WIRED".to_owned(),
            });
        }
    }
    if !errors.is_empty() {
        return Err(errors);
    }

    const BEHAVIOR_ID: &str = "HBR-INT-009";
    match store
        .validate_diagnostic_tier_posture(run_id, BEHAVIOR_ID)
        .await
    {
        Ok(posture)
            if posture.run_id == run_id
                && posture.behavior_id == BEHAVIOR_ID
                && posture.tiers.iter().all(|tier| {
                    !tier.event_ledger_event_id.trim().is_empty()
                        && (tier.evidence_ref.starts_with("eventledger://kernel/")
                            || tier
                                .evidence_ref
                                .starts_with("internal-diagnostics://session/")
                            || tier
                                .evidence_ref
                                .starts_with("palmistry-observation://session/"))
                }) =>
        {
            Ok(vec![posture])
        }
        Ok(_) => Err(vec![BehaviorCoverageError {
            behavior_id: BEHAVIOR_ID,
            reason: "diagnostic tier records lack correlated durable EventLedger evidence"
                .to_owned(),
        }]),
        Err(error) => Err(vec![BehaviorCoverageError {
            behavior_id: BEHAVIOR_ID,
            reason: format!("run {run_id} has no valid three-tier diagnostic evidence: {error}"),
        }]),
    }
}
