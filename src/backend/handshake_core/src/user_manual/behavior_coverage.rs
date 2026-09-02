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
    DexterityLaunchAdapterRegistry, ModelLaneDiagnosticTier, ModelLaneDiagnosticTierPosture,
    ModelLaneDiagnosticTierState, ModelLaneDiagnosticsProjection, ModelLaneSchemaRegistryRow,
    ModelLaneStore,
};
use crate::swarm_orchestration::operator_chat::{
    ModelLaneCaptureRecorder, OperatorChatLaunchService,
};
use crate::swarm_orchestration::routing_execution::ModelLaneRoutingExecutionStore;

use super::registry::{wp009_surface_registry, SurfaceGroup};
use super::store::{
    NewUserManualPage, UserManualFeatureEntry, UserManualPage, UserManualToolEntry,
};
use super::USER_MANUAL_VERSION;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
pub enum DiagnosticTierPosture {
    /// The tier emits its own per-behavior observation for this row.
    Wired,
    /// Run-level WIRED: this behavior's Tier-2 internal_diagnostics and Tier-3
    /// Palmistry coverage is carried by the single correlated HBR-INT-009
    /// envelope that the native producer and the authenticated watcher emit
    /// ONCE per `ModelLaneRun` (one Flight Recorder + internal_diagnostics +
    /// Palmistry triplet), NOT by a fabricated per-behavior observation. This
    /// literal is a declaration, not a proof: its liveness is established only
    /// by [`verify_model_lane_behavior_evidence`], which validates a real run's
    /// durable run-level `model_lane_diagnostic_tier_statuses` records
    /// (`internal-diagnostics://session/` + `palmistry-observation://session/`
    /// evidence refs) through `ModelLaneStore::validate_diagnostic_tier_posture`.
    RunLevelWired,
    NotApplicableWithReason,
    DeferredWithReason,
}

impl DiagnosticTierPosture {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Wired => "WIRED",
            Self::RunLevelWired => "RUN_LEVEL_WIRED",
            Self::NotApplicableWithReason => "NOT_APPLICABLE-with-reason",
            Self::DeferredWithReason => "DEFERRED-with-reason",
        }
    }
}

// ---------------------------------------------------------------------------
// MT-011 / HBR-INT-009: the EXACT tier-to-evidence-URI binding.
//
// The three diagnostic tiers are produced by three DIFFERENT subsystems and
// each writes its evidence under its OWN URI scheme:
//
//   Tier 1  FlightRecorder       `eventledger://kernel/`
//           the kept-as-is backend BUSINESS-EVENT ledger.
//   Tier 2  InternalDiagnostics  `internal-diagnostics://session/`
//           Handshake's own IN-PROCESS self-diagnostics: panic hook, UI-thread
//           heartbeat, frame-time, CPU/RSS/GPU counters, open diagnostic-event
//           API. Emitted by `api::palmistry::record_diagnostic_observation`
//           from the native producer ring.
//   Tier 3  Palmistry            `palmistry-observation://session/`
//           the EXTERNAL, out-of-process watcher that survives a Handshake
//           freeze or crash and writes durable survivor records.
//
// Before MT-011 remediation, `verify_model_lane_behavior_evidence` accepted ANY
// of the three prefixes for EVERY tier — one `||` chain applied uniformly to
// every record — so a Tier-3 Palmistry URI offered as Tier-2 internal_diagnostics
// evidence, or the reverse, was accepted. That is coverage forgery, not a
// cosmetic laxity: Tier 2 and Tier 3 observe different failure classes (a hung
// UI thread is invisible to Tier 2's own in-process ring and is exactly what
// Tier 3 exists to catch), so they are NOT interchangeable. A run that only ever
// produced Palmistry observations has no in-process diagnostics coverage at all,
// and substituting one scheme for the other would let a work packet claim a
// diagnostic tier it never wired.
//
// The binding below is therefore exact and total, and a cross-tier scheme is a
// REJECTION, never a fallback.
// ---------------------------------------------------------------------------

/// Tier 1. EventLedger kernel stream evidence for the Flight Recorder tier.
pub const FLIGHT_RECORDER_EVIDENCE_URI_PREFIX: &str = "eventledger://kernel/";
/// Tier 2. Handshake-native in-process diagnostics session evidence.
pub const INTERNAL_DIAGNOSTICS_EVIDENCE_URI_PREFIX: &str = "internal-diagnostics://session/";
/// Tier 3. External Palmistry watcher observation evidence.
pub const PALMISTRY_EVIDENCE_URI_PREFIX: &str = "palmistry-observation://session/";

/// Scheme token of [`FLIGHT_RECORDER_EVIDENCE_URI_PREFIX`].
pub const FLIGHT_RECORDER_EVIDENCE_URI_SCHEME: &str = "eventledger:";
/// Scheme token of [`INTERNAL_DIAGNOSTICS_EVIDENCE_URI_PREFIX`].
pub const INTERNAL_DIAGNOSTICS_EVIDENCE_URI_SCHEME: &str = "internal-diagnostics:";
/// Scheme token of [`PALMISTRY_EVIDENCE_URI_PREFIX`].
pub const PALMISTRY_EVIDENCE_URI_SCHEME: &str = "palmistry-observation:";

/// The complete tier -> (scheme token, required URI prefix) binding.
///
/// Total over [`ModelLaneDiagnosticTier`]: adding a fourth tier to that enum
/// makes [`diagnostic_tier_evidence_uri_prefix`] a non-exhaustive `match` and
/// therefore a COMPILE error, so a new tier cannot inherit another tier's
/// scheme by omission.
pub const DIAGNOSTIC_TIER_EVIDENCE_URI_BINDING: [(ModelLaneDiagnosticTier, &str, &str); 3] = [
    (
        ModelLaneDiagnosticTier::FlightRecorder,
        FLIGHT_RECORDER_EVIDENCE_URI_SCHEME,
        FLIGHT_RECORDER_EVIDENCE_URI_PREFIX,
    ),
    (
        ModelLaneDiagnosticTier::InternalDiagnostics,
        INTERNAL_DIAGNOSTICS_EVIDENCE_URI_SCHEME,
        INTERNAL_DIAGNOSTICS_EVIDENCE_URI_PREFIX,
    ),
    (
        ModelLaneDiagnosticTier::Palmistry,
        PALMISTRY_EVIDENCE_URI_SCHEME,
        PALMISTRY_EVIDENCE_URI_PREFIX,
    ),
];

/// The one URI prefix this tier's evidence is allowed to use.
pub const fn diagnostic_tier_evidence_uri_prefix(tier: ModelLaneDiagnosticTier) -> &'static str {
    match tier {
        ModelLaneDiagnosticTier::FlightRecorder => FLIGHT_RECORDER_EVIDENCE_URI_PREFIX,
        ModelLaneDiagnosticTier::InternalDiagnostics => INTERNAL_DIAGNOSTICS_EVIDENCE_URI_PREFIX,
        ModelLaneDiagnosticTier::Palmistry => PALMISTRY_EVIDENCE_URI_PREFIX,
    }
}

/// The one URI scheme token this tier's evidence is allowed to use.
pub const fn diagnostic_tier_evidence_uri_scheme(tier: ModelLaneDiagnosticTier) -> &'static str {
    match tier {
        ModelLaneDiagnosticTier::FlightRecorder => FLIGHT_RECORDER_EVIDENCE_URI_SCHEME,
        ModelLaneDiagnosticTier::InternalDiagnostics => INTERNAL_DIAGNOSTICS_EVIDENCE_URI_SCHEME,
        ModelLaneDiagnosticTier::Palmistry => PALMISTRY_EVIDENCE_URI_SCHEME,
    }
}

/// Which tier a URI's scheme BELONGS to, independent of the tier that offered
/// it. This is what turns "wrong prefix" into the far more specific finding
/// "this is another tier's evidence wearing this tier's label".
///
/// The three scheme tokens are mutually non-prefixing, so the first match is
/// the only match.
pub fn diagnostic_tier_owning_evidence_uri_scheme(
    evidence_ref: &str,
) -> Option<ModelLaneDiagnosticTier> {
    let evidence_ref = evidence_ref.trim();
    DIAGNOSTIC_TIER_EVIDENCE_URI_BINDING
        .iter()
        .find(|(_, scheme, _)| evidence_ref.starts_with(scheme))
        .map(|(tier, _, _)| *tier)
}

/// Why a diagnostic tier record's `evidence_ref` was refused.
///
/// Every field is a compile-time constant tier label or prefix. The offending
/// URI itself is deliberately NOT carried: a denial rendered for one account
/// must not become a metadata side channel for another account's diagnostic
/// evidence (HBR-PRIV-004). The scheme that WAS found is a bounded, compile-known
/// token, so the message stays actionable without disclosing row content.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(tag = "violation", rename_all = "snake_case")]
pub enum DiagnosticEvidenceUriViolation {
    /// The URI carries ANOTHER tier's scheme. This is the cross-tier
    /// substitution the MT-011 evidence gate exists to reject.
    CrossTierSchemeSubstitution {
        declared_tier: &'static str,
        scheme_owned_by_tier: &'static str,
        expected_prefix: &'static str,
    },
    /// The URI carries no diagnostic scheme bound to any tier.
    UnboundScheme {
        declared_tier: &'static str,
        expected_prefix: &'static str,
    },
    /// The tier's own scheme, but not under its required authority root.
    WrongAuthorityRoot {
        declared_tier: &'static str,
        expected_prefix: &'static str,
    },
}

impl DiagnosticEvidenceUriViolation {
    /// Stable machine-readable reason code for typed receipts and tests.
    pub const fn reason_code(self) -> &'static str {
        match self {
            Self::CrossTierSchemeSubstitution { .. } => {
                "DIAGNOSTIC_EVIDENCE_CROSS_TIER_SCHEME_SUBSTITUTION"
            }
            Self::UnboundScheme { .. } => "DIAGNOSTIC_EVIDENCE_UNBOUND_SCHEME",
            Self::WrongAuthorityRoot { .. } => "DIAGNOSTIC_EVIDENCE_WRONG_AUTHORITY_ROOT",
        }
    }
}

impl std::fmt::Display for DiagnosticEvidenceUriViolation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::CrossTierSchemeSubstitution {
                declared_tier,
                scheme_owned_by_tier,
                expected_prefix,
            } => write!(
                f,
                "{}: HBR-INT-009 tier `{declared_tier}` offered evidence whose URI scheme belongs \
                 to tier `{scheme_owned_by_tier}`; the tiers observe different failure classes and \
                 are not interchangeable. Tier `{declared_tier}` evidence must use \
                 `{expected_prefix}`.",
                self.reason_code()
            ),
            Self::UnboundScheme {
                declared_tier,
                expected_prefix,
            } => write!(
                f,
                "{}: HBR-INT-009 tier `{declared_tier}` offered evidence with no diagnostic URI \
                 scheme bound to any tier; it must use `{expected_prefix}`.",
                self.reason_code()
            ),
            Self::WrongAuthorityRoot {
                declared_tier,
                expected_prefix,
            } => write!(
                f,
                "{}: HBR-INT-009 tier `{declared_tier}` used its own URI scheme but not its \
                 required authority root; it must use `{expected_prefix}`.",
                self.reason_code()
            ),
        }
    }
}

/// Pin one diagnostic tier record's `evidence_ref` to that tier's exact URI.
///
/// Rejects, in order of specificity: another tier's scheme (cross-tier
/// substitution), no bound scheme at all, and the right scheme under the wrong
/// authority root.
pub fn verify_diagnostic_tier_evidence_uri(
    tier: ModelLaneDiagnosticTier,
    evidence_ref: &str,
) -> Result<(), DiagnosticEvidenceUriViolation> {
    let evidence_ref = evidence_ref.trim();
    let declared_tier = tier.as_str();
    let expected_prefix = diagnostic_tier_evidence_uri_prefix(tier);
    match diagnostic_tier_owning_evidence_uri_scheme(evidence_ref) {
        Some(scheme_tier) if scheme_tier != tier => Err(
            DiagnosticEvidenceUriViolation::CrossTierSchemeSubstitution {
                declared_tier,
                scheme_owned_by_tier: scheme_tier.as_str(),
                expected_prefix,
            },
        ),
        Some(_) if evidence_ref.starts_with(expected_prefix) => Ok(()),
        Some(_) => Err(DiagnosticEvidenceUriViolation::WrongAuthorityRoot {
            declared_tier,
            expected_prefix,
        }),
        None => Err(DiagnosticEvidenceUriViolation::UnboundScheme {
            declared_tier,
            expected_prefix,
        }),
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
    // MT-011 anti-gaming (run-level HBR-INT-009 encoding): the 24 wp1.model_lane.*
    // behaviors do NOT each fabricate a per-behavior diagnostic observation. The
    // native internal_diagnostics producer and the authenticated Palmistry watcher
    // emit ONE correlated run-level HBR-INT-009 envelope per ModelLaneRun, so each
    // per-behavior row declares `RunLevelWired` ("covered by the run-level
    // envelope"). This is a STRUCTURAL declaration check only; it does not prove
    // liveness. Liveness is proven separately and against real durable records by
    // `verify_model_lane_behavior_evidence` (a run-level MT-011 proof target that
    // fails closed when the run's HBR-INT-009 records are absent/incomplete). A
    // hardcoded `Wired` (per-behavior) posture on these rows is therefore rejected.
    if row.behavior_id.starts_with("wp1.model_lane.")
        && (row.internal_diagnostics_posture != DiagnosticTierPosture::RunLevelWired
            || row.palmistry_posture != DiagnosticTierPosture::RunLevelWired)
    {
        errors.push(BehaviorCoverageError {
            behavior_id: row.behavior_id,
            reason: "internal_diagnostics and Palmistry postures for model-lane behaviors must be RUN_LEVEL_WIRED (covered by the single run-level HBR-INT-009 envelope, proven live by verify_model_lane_behavior_evidence against a real run, not by a per-behavior WIRED literal)"
                .to_owned(),
        });
    }
    // A RUN_LEVEL_WIRED row must name where the run-level envelope evidence lives:
    // the explanatory reason and the family-scoped palmistry follow_up_ref.
    if (row.internal_diagnostics_posture == DiagnosticTierPosture::RunLevelWired
        || row.palmistry_posture == DiagnosticTierPosture::RunLevelWired)
        && (row.deferred_reason.is_none() || row.follow_up_ref.is_none())
    {
        errors.push(BehaviorCoverageError {
            behavior_id: row.behavior_id,
            reason: "RUN_LEVEL_WIRED posture requires an explanatory reason and a run-level HBR-INT-009 follow_up_ref"
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

// ---------------------------------------------------------------------------
// MT-022 / HBR-MAN-003: named-surface existence gate for the CANONICAL corpus.
//
// [`compute_behavior_consistency`] above proves ROUTES, compiled runtime-surface
// SYMBOLS, and SCHEMA ids for the declared behavior rows. It never reads prose,
// so a product symbol or Flight Recorder event id typed inside a `section(...)`
// markdown body used to be unverified free text — which is exactly how the
// `external_compat` and swarm-budget gaps survived, and how
// `ModelLaneStore::revoke_cloud_consent_receipt` (the method actually lives on
// `SwarmCoordinator`) drifted into three manual pages unnoticed.
//
// The legacy `.GOV/roles_shared/checks/hbr-man-003-scan.mjs` cannot close this:
// it reads ONLY the legacy `model_manual/content.rs` file, it resolves names by
// "does this string appear anywhere under src/", and it was deregistered from
// `gov-check` on 2026-06-03 as product-owned. This gate is the product-owned
// replacement, and it is strictly stronger than a text search: every accepted
// name is bound to a real Rust item through [`ManualNamedSymbol::assert_compiled`]
// or to a compiled Flight-Recorder id vocabulary, so a renamed product symbol
// becomes a COMPILE error and an invented one becomes a TEST failure.
//
// Scan rule (deterministic, no heuristics). MT-022 widened this from "Rust
// paths only" after a validator found that lowercase routes, env/config keys,
// schema ids and bare type names all evaded the gate:
//  * BOTH `body_md` prose AND every string leaf of `body_json` are scanned.
//    Prose hidden inside a JSON section used to be entirely unchecked.
//  * A ROUTE claim is a backticked span of the shape `METHOD /path`. It must
//    resolve against [`wp009_surface_registry`] — exactly, or as a concrete
//    instance of a registry pattern (`GET /usermanual/pages/manual-toc`
//    resolves through `/usermanual/pages/:slug`). A claim that writes its own
//    parameter names (`/workspaces/:ws/...` against the mounted
//    `/workspaces/:workspace_id/...`) does NOT resolve — that is real drift,
//    because a model copying the manual would call a path the router rejects.
//    A lowercase method token is reported rather than accepted.
//    [`NON_REGISTRY_DOCUMENTED_ROUTES`] is the written escape hatch for real
//    routes that are deliberately not WP-009 registry rows.
//  * A SYMBOL claim is a backticked span whose text — after cutting at the
//    first `(`, `{`, `<` or `[` and trimming — is a Rust path of the shape
//    `Uppercase(::segment)*`. A `::` path must be in the compiled vocabulary.
//    A BARE `CamelCase` name resolves when it is a segment of a compiled
//    vocabulary entry (so `ModelLaneStore` is anchored by
//    `ModelLaneStore::record_message`); otherwise it degrades to a LITERAL
//    claim rather than being silently ignored.
//  * A LITERAL claim is a backticked `SCREAMING_SNAKE` token (env var,
//    EventLedger event type, state label), a `snake_case` token (config key,
//    JSON field, table name), an `hsk.<name>@<n>` schema id, or an unanchored
//    bare `CamelCase` name. These name string literals, not nameable Rust
//    items, so they cannot be compile-anchored from here. They are collected by
//    [`manual_literal_claims`] and resolved by [`verify_manual_literal_claims`]
//    against the real product source text, which the proof suite supplies. That
//    is weaker than compile anchoring (a coordinated rename in both places
//    passes) but it does catch fabricated and mistyped names, which is the
//    failure this gate exists to stop.
//  * A FLIGHT RECORDER claim is any `FR-EVT-` token, backticked or not, matched
//    CASE-INSENSITIVELY so a lowercase `fr-evt-...` cannot slip past; a
//    non-canonical spelling is itself reported. A token resolves when it is a
//    compiled event id exactly, or a prefix of one AT A `-` SEGMENT BOUNDARY —
//    so the family `FR-EVT-AGENT-*` still resolves through
//    `FR-EVT-AGENT-TOOLCALL`, while a truncation like `FR-EVT-S` no longer
//    resolves against `FR-EVT-SWARM-...`.
//  * [`PLANNED_FLIGHT_RECORDER_EVENT_FAMILIES`] is the explicit, reviewable
//    escape hatch for families the manual documents as DEFERRED wiring. It
//    mirrors the legacy scanner's `CommandStatus::Planned` semantics: an
//    exemption must be written down, never inferred.
//
// Deliberately NOT checked, so the limit is visible instead of implied:
//  * Backticked BARE paths without a method (`/knowledge/code/*`,
//    `/routing/cancel`, `/proc/self/fd`). They are route FRAGMENTS, globs and
//    OS paths, not callable-surface claims; checking them against the registry
//    would report the whole class as drift. The method-prefixed form above is
//    the unambiguous claim shape and is enforced.
//  * Shell/command spans, prose, and identifiers that are none of the shapes
//    above. Those land in neither the symbol nor the literal classes; the
//    counts returned by the gate report exactly how much it looked at.
// ---------------------------------------------------------------------------

/// Behavior id stamped on every named-surface failure so the error stream stays
/// machine-groupable alongside the behavior-coverage errors.
pub const MANUAL_NAMED_SURFACE_BEHAVIOR_ID: &str = "user_manual.named_surface";

/// How a product symbol named by the manual is proven to exist.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ManualNamedSymbolProof {
    /// [`ManualNamedSymbol::assert_compiled`] references the real Rust item, so
    /// a rename or deletion in product code fails the build here.
    CompileAnchored,
    /// The item is real but cannot be NAMED from this module (module-private,
    /// `pub(super)`, or owned by a crate `handshake_core` does not depend on).
    /// Recorded with an explicit reason instead of being silently accepted.
    DeclaredNotNameableHere(&'static str),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ManualNamedSymbol {
    /// `AccountBoundAuthority::Account`
    AccountBoundAuthorityAccount,
    /// `AccountBoundAuthority::Unattributed`
    AccountBoundAuthorityUnattributed,
    /// `AdmitDecision::Suppress`
    AdmitDecisionSuppress,
    /// `AppState::model_catalog`
    AppStateModelCatalog,
    /// `BreakerState::Closed`
    BreakerStateClosed,
    /// `BreakerState::HalfOpen`
    BreakerStateHalfOpen,
    /// `BreakerState::Open`
    BreakerStateOpen,
    /// `ConsoleCategory::Breaker`
    ConsoleCategoryBreaker,
    /// `ConsoleCategory::SpawnRejected`
    ConsoleCategorySpawnRejected,
    /// `ConsoleSwarmSink::shared`
    ConsoleSwarmSinkShared,
    /// `DexterityLaunchAdapterRegistry::adapter_kind_for_spawn_request`
    DexterityLaunchAdapterRegistryAdapterKindForSpawnRequest,
    /// `DexterityLaunchContract::attach_to_spawn_request`
    DexterityLaunchContractAttachToSpawnRequest,
    /// `DexterityLaunchContract::restart_generation`
    DexterityLaunchContractRestartGeneration,
    /// `DisabledLlmClient::completion`
    DisabledLlmClientCompletion,
    /// `DisabledLlmClient::embedding`
    DisabledLlmClientEmbedding,
    /// `EmbeddedModelProcess::record_reserved_load_with_durable_ack`
    EmbeddedModelProcessRecordReservedLoadWithDurableAck,
    /// `EmbeddedModelProcess::shutdown`
    EmbeddedModelProcessShutdown,
    /// `EngineOriginValidator::validate_load_spec`
    EngineOriginValidatorValidateLoadSpec,
    /// `ExternalEngineImportRecord::new`
    ExternalEngineImportRecordNew,
    /// `FailureFingerprint::compute`
    FailureFingerprintCompute,
    /// `FinishReason::Cancelled`
    FinishReasonCancelled,
    /// `FlightRecorderEventType::LlmInference`
    FlightRecorderEventTypeLlmInference,
    /// `FlightRecorderSwarmSink::new`
    FlightRecorderSwarmSinkNew,
    /// `GuardedCliChild::terminate_and_collect`
    GuardedCliChildTerminateAndCollect,
    /// `HandshakeNativeSandboxAdapter::spawn_attached_with_stdio`
    HandshakeNativeSandboxAdapterSpawnAttachedWithStdio,
    /// `IsolationTier::Tier3Microvm`
    IsolationTierTier3Microvm,
    /// `KernelEventType::ModelRuntimeSelectionRecorded`
    KernelEventTypeModelRuntimeSelectionRecorded,
    /// `KillOutcome::Failed`
    KillOutcomeFailed,
    /// `LaunchAuthority::SubagentManager`
    LaunchAuthoritySubagentManager,
    /// `LiveCliSpawner::spawn`
    LiveCliSpawnerSpawn,
    /// `LlmClient::embedding`
    LlmClientEmbedding,
    /// `LlmClient::shutdown_gracefully`
    LlmClientShutdownGracefully,
    /// `LocalModelAdapterInvariant::validate`
    LocalModelAdapterInvariantValidate,
    /// `LocalModelRuntimeLlmClient::embedding`
    LocalModelRuntimeLlmClientEmbedding,
    /// `ModelCatalog::embedding_model_for_dim`
    ModelCatalogEmbeddingModelForDim,
    /// `ModelCatalog::list`
    ModelCatalogList,
    /// `ModelCatalog::record_selection_decision`
    ModelCatalogRecordSelectionDecision,
    /// `ModelCatalog::record_selection_decision_with_context`
    ModelCatalogRecordSelectionDecisionWithContext,
    /// `ModelLaneAuthority::Advisory`
    ModelLaneAuthorityAdvisory,
    /// `ModelLaneAuthority::Promoted`
    ModelLaneAuthorityPromoted,
    /// `ModelLaneError::InvalidInput`
    ModelLaneErrorInvalidInput,
    /// `ModelLaneStatus::Cancelled`
    ModelLaneStatusCancelled,
    /// `ModelLaneStore::consume_context_bundle_for_downstream`
    ModelLaneStoreConsumeContextBundleForDownstream,
    /// `ModelLaneStore::diagnostics_projection`
    ModelLaneStoreDiagnosticsProjection,
    /// `ModelLaneStore::preflight_cloud_spawn_request`
    ModelLaneStorePreflightCloudSpawnRequest,
    /// `ModelLaneStore::record_cloud_consent_receipt`
    ModelLaneStoreRecordCloudConsentReceipt,
    /// `ModelLaneStore::record_cloud_projection_plan`
    ModelLaneStoreRecordCloudProjectionPlan,
    /// `ModelLaneStore::record_context_bundle_artifact_binding`
    ModelLaneStoreRecordContextBundleArtifactBinding,
    /// `ModelLaneStore::record_context_bundle_handoff`
    ModelLaneStoreRecordContextBundleHandoff,
    /// `ModelLaneStore::record_message`
    ModelLaneStoreRecordMessage,
    /// `ModelLaneStore::record_promotion_decision`
    ModelLaneStoreRecordPromotionDecision,
    /// `ModelLaneStore::recover_run_after_restart`
    ModelLaneStoreRecoverRunAfterRestart,
    /// `ModelLaneStore::replay_cloud_consent_authority`
    ModelLaneStoreReplayCloudConsentAuthority,
    /// `ModelLaneStore::replay_context_bundle_handoffs`
    ModelLaneStoreReplayContextBundleHandoffs,
    /// `ModelLaneStore::replay_promotion_decisions`
    ModelLaneStoreReplayPromotionDecisions,
    /// `ModelLaneStore::replay_run`
    ModelLaneStoreReplayRun,
    /// `ModelLaneStore::validate_diagnostic_tier_posture`
    ModelLaneStoreValidateDiagnosticTierPosture,
    /// `ModelRuntimeError::AdapterMismatch`
    ModelRuntimeErrorAdapterMismatch,
    /// `ProcessReclaimRuntime::production_with_lease`
    ProcessReclaimRuntimeProductionWithLease,
    /// `ProviderKind::ExternalCompat`
    ProviderKindExternalCompat,
    /// `ProviderKind::Local`
    ProviderKindLocal,
    /// `ProviderRegistry::from_env`
    ProviderRegistryFromEnv,
    /// `Reclaim::run_owned_process`
    ReclaimRunOwnedProcess,
    /// `Role::Dialog`
    RoleDialog,
    /// `Role::Window`
    RoleWindow,
    /// `RunBudget::defaulted`
    RunBudgetDefaulted,
    /// `RuntimeBinding::Subagent`
    RuntimeBindingSubagent,
    /// `SemanticUnavailableReason::DimMismatch`
    SemanticUnavailableReasonDimMismatch,
    /// `SemanticUnavailableReason::NoModel`
    SemanticUnavailableReasonNoModel,
    /// `SpawnRequest::with_dexterity_launch`
    SpawnRequestWithDexterityLaunch,
    /// `SwarmCoordinator::context_bundle_for_downstream_lane`
    SwarmCoordinatorContextBundleForDownstreamLane,
    /// `SwarmCoordinator::invoke_downstream_context_bundle`
    SwarmCoordinatorInvokeDownstreamContextBundle,
    /// `SwarmCoordinator::launch_operator_subagent_model_lane`
    SwarmCoordinatorLaunchOperatorSubagentModelLane,
    /// `SwarmCoordinator::live_session_count`
    SwarmCoordinatorLiveSessionCount,
    /// `SwarmCoordinator::max_concurrent`
    SwarmCoordinatorMaxConcurrent,
    /// `SwarmCoordinator::remaining`
    SwarmCoordinatorRemaining,
    /// `SwarmCoordinator::retry_pending_session_cleanups`
    SwarmCoordinatorRetryPendingSessionCleanups,
    /// `SwarmCoordinator::revoke_cloud_consent_receipt`
    SwarmCoordinatorRevokeCloudConsentReceipt,
    /// `SwarmCoordinator::session_runtime`
    SwarmCoordinatorSessionRuntime,
    /// `SwarmCoordinator::set_max_concurrent`
    SwarmCoordinatorSetMaxConcurrent,
    /// `SwarmCoordinator::spawn_session`
    SwarmCoordinatorSpawnSession,
    /// `SwarmError::BreakerOpen`
    SwarmErrorBreakerOpen,
    /// `SwarmError::BudgetExhausted`
    SwarmErrorBudgetExhausted,
    /// `SwarmError::ConcurrencyCapReached`
    SwarmErrorConcurrencyCapReached,
    /// `SwarmError::DuplicateInstance`
    SwarmErrorDuplicateInstance,
    /// `SwarmError::LifetimeSpawnCeilingReached`
    SwarmErrorLifetimeSpawnCeilingReached,
    /// `SwarmError::ProviderNotConfigured`
    SwarmErrorProviderNotConfigured,
    /// `SwarmErrorClass::BreakerOpen`
    SwarmErrorClassBreakerOpen,
    /// `SwarmEvent::BreakerTripped`
    SwarmEventBreakerTripped,
    /// `SwarmEvent::SpawnRejected`
    SwarmEventSpawnRejected,
    /// `SwarmFrEventId::SpawnRejected`
    SwarmFrEventIdSpawnRejected,
    /// `SystemScopeAuthority::boot_recovery`
    SystemScopeAuthorityBootRecovery,
    /// `Update::decode_v1`
    UpdateDecodeV1,
    /// `WorktreeVmRegistry::teardown_worktree_vm`
    WorktreeVmRegistryTeardownWorktreeVm,
}

impl ManualNamedSymbol {
    const ALL: &'static [ManualNamedSymbol] = &[
        Self::AccountBoundAuthorityAccount,
        Self::AccountBoundAuthorityUnattributed,
        Self::AdmitDecisionSuppress,
        Self::AppStateModelCatalog,
        Self::BreakerStateClosed,
        Self::BreakerStateHalfOpen,
        Self::BreakerStateOpen,
        Self::ConsoleCategoryBreaker,
        Self::ConsoleCategorySpawnRejected,
        Self::ConsoleSwarmSinkShared,
        Self::DexterityLaunchAdapterRegistryAdapterKindForSpawnRequest,
        Self::DexterityLaunchContractAttachToSpawnRequest,
        Self::DexterityLaunchContractRestartGeneration,
        Self::DisabledLlmClientCompletion,
        Self::DisabledLlmClientEmbedding,
        Self::EmbeddedModelProcessRecordReservedLoadWithDurableAck,
        Self::EmbeddedModelProcessShutdown,
        Self::EngineOriginValidatorValidateLoadSpec,
        Self::ExternalEngineImportRecordNew,
        Self::FailureFingerprintCompute,
        Self::FinishReasonCancelled,
        Self::FlightRecorderEventTypeLlmInference,
        Self::FlightRecorderSwarmSinkNew,
        Self::GuardedCliChildTerminateAndCollect,
        Self::HandshakeNativeSandboxAdapterSpawnAttachedWithStdio,
        Self::IsolationTierTier3Microvm,
        Self::KernelEventTypeModelRuntimeSelectionRecorded,
        Self::KillOutcomeFailed,
        Self::LaunchAuthoritySubagentManager,
        Self::LiveCliSpawnerSpawn,
        Self::LlmClientEmbedding,
        Self::LlmClientShutdownGracefully,
        Self::LocalModelAdapterInvariantValidate,
        Self::LocalModelRuntimeLlmClientEmbedding,
        Self::ModelCatalogEmbeddingModelForDim,
        Self::ModelCatalogList,
        Self::ModelCatalogRecordSelectionDecision,
        Self::ModelCatalogRecordSelectionDecisionWithContext,
        Self::ModelLaneAuthorityAdvisory,
        Self::ModelLaneAuthorityPromoted,
        Self::ModelLaneErrorInvalidInput,
        Self::ModelLaneStatusCancelled,
        Self::ModelLaneStoreConsumeContextBundleForDownstream,
        Self::ModelLaneStoreDiagnosticsProjection,
        Self::ModelLaneStorePreflightCloudSpawnRequest,
        Self::ModelLaneStoreRecordCloudConsentReceipt,
        Self::ModelLaneStoreRecordCloudProjectionPlan,
        Self::ModelLaneStoreRecordContextBundleArtifactBinding,
        Self::ModelLaneStoreRecordContextBundleHandoff,
        Self::ModelLaneStoreRecordMessage,
        Self::ModelLaneStoreRecordPromotionDecision,
        Self::ModelLaneStoreRecoverRunAfterRestart,
        Self::ModelLaneStoreReplayCloudConsentAuthority,
        Self::ModelLaneStoreReplayContextBundleHandoffs,
        Self::ModelLaneStoreReplayPromotionDecisions,
        Self::ModelLaneStoreReplayRun,
        Self::ModelLaneStoreValidateDiagnosticTierPosture,
        Self::ModelRuntimeErrorAdapterMismatch,
        Self::ProcessReclaimRuntimeProductionWithLease,
        Self::ProviderKindExternalCompat,
        Self::ProviderKindLocal,
        Self::ProviderRegistryFromEnv,
        Self::ReclaimRunOwnedProcess,
        Self::RoleDialog,
        Self::RoleWindow,
        Self::RunBudgetDefaulted,
        Self::RuntimeBindingSubagent,
        Self::SemanticUnavailableReasonDimMismatch,
        Self::SemanticUnavailableReasonNoModel,
        Self::SpawnRequestWithDexterityLaunch,
        Self::SwarmCoordinatorContextBundleForDownstreamLane,
        Self::SwarmCoordinatorInvokeDownstreamContextBundle,
        Self::SwarmCoordinatorLaunchOperatorSubagentModelLane,
        Self::SwarmCoordinatorLiveSessionCount,
        Self::SwarmCoordinatorMaxConcurrent,
        Self::SwarmCoordinatorRemaining,
        Self::SwarmCoordinatorRetryPendingSessionCleanups,
        Self::SwarmCoordinatorRevokeCloudConsentReceipt,
        Self::SwarmCoordinatorSessionRuntime,
        Self::SwarmCoordinatorSetMaxConcurrent,
        Self::SwarmCoordinatorSpawnSession,
        Self::SwarmErrorBreakerOpen,
        Self::SwarmErrorBudgetExhausted,
        Self::SwarmErrorConcurrencyCapReached,
        Self::SwarmErrorDuplicateInstance,
        Self::SwarmErrorLifetimeSpawnCeilingReached,
        Self::SwarmErrorProviderNotConfigured,
        Self::SwarmErrorClassBreakerOpen,
        Self::SwarmEventBreakerTripped,
        Self::SwarmEventSpawnRejected,
        Self::SwarmFrEventIdSpawnRejected,
        Self::SystemScopeAuthorityBootRecovery,
        Self::UpdateDecodeV1,
        Self::WorktreeVmRegistryTeardownWorktreeVm,
    ];

    /// The exact text the manual prose is allowed to print between backticks.
    fn literal(self) -> &'static str {
        match self {
            Self::AccountBoundAuthorityAccount => "AccountBoundAuthority::Account",
            Self::AccountBoundAuthorityUnattributed => "AccountBoundAuthority::Unattributed",
            Self::AdmitDecisionSuppress => "AdmitDecision::Suppress",
            Self::AppStateModelCatalog => "AppState::model_catalog",
            Self::BreakerStateClosed => "BreakerState::Closed",
            Self::BreakerStateHalfOpen => "BreakerState::HalfOpen",
            Self::BreakerStateOpen => "BreakerState::Open",
            Self::ConsoleCategoryBreaker => "ConsoleCategory::Breaker",
            Self::ConsoleCategorySpawnRejected => "ConsoleCategory::SpawnRejected",
            Self::ConsoleSwarmSinkShared => "ConsoleSwarmSink::shared",
            Self::DexterityLaunchAdapterRegistryAdapterKindForSpawnRequest => {
                "DexterityLaunchAdapterRegistry::adapter_kind_for_spawn_request"
            }
            Self::DexterityLaunchContractAttachToSpawnRequest => {
                "DexterityLaunchContract::attach_to_spawn_request"
            }
            Self::DexterityLaunchContractRestartGeneration => {
                "DexterityLaunchContract::restart_generation"
            }
            Self::DisabledLlmClientCompletion => "DisabledLlmClient::completion",
            Self::DisabledLlmClientEmbedding => "DisabledLlmClient::embedding",
            Self::EmbeddedModelProcessRecordReservedLoadWithDurableAck => {
                "EmbeddedModelProcess::record_reserved_load_with_durable_ack"
            }
            Self::EmbeddedModelProcessShutdown => "EmbeddedModelProcess::shutdown",
            Self::EngineOriginValidatorValidateLoadSpec => {
                "EngineOriginValidator::validate_load_spec"
            }
            Self::ExternalEngineImportRecordNew => "ExternalEngineImportRecord::new",
            Self::FailureFingerprintCompute => "FailureFingerprint::compute",
            Self::FinishReasonCancelled => "FinishReason::Cancelled",
            Self::FlightRecorderEventTypeLlmInference => "FlightRecorderEventType::LlmInference",
            Self::FlightRecorderSwarmSinkNew => "FlightRecorderSwarmSink::new",
            Self::GuardedCliChildTerminateAndCollect => "GuardedCliChild::terminate_and_collect",
            Self::HandshakeNativeSandboxAdapterSpawnAttachedWithStdio => {
                "HandshakeNativeSandboxAdapter::spawn_attached_with_stdio"
            }
            Self::IsolationTierTier3Microvm => "IsolationTier::Tier3Microvm",
            Self::KernelEventTypeModelRuntimeSelectionRecorded => {
                "KernelEventType::ModelRuntimeSelectionRecorded"
            }
            Self::KillOutcomeFailed => "KillOutcome::Failed",
            Self::LaunchAuthoritySubagentManager => "LaunchAuthority::SubagentManager",
            Self::LiveCliSpawnerSpawn => "LiveCliSpawner::spawn",
            Self::LlmClientEmbedding => "LlmClient::embedding",
            Self::LlmClientShutdownGracefully => "LlmClient::shutdown_gracefully",
            Self::LocalModelAdapterInvariantValidate => "LocalModelAdapterInvariant::validate",
            Self::LocalModelRuntimeLlmClientEmbedding => "LocalModelRuntimeLlmClient::embedding",
            Self::ModelCatalogEmbeddingModelForDim => "ModelCatalog::embedding_model_for_dim",
            Self::ModelCatalogList => "ModelCatalog::list",
            Self::ModelCatalogRecordSelectionDecision => "ModelCatalog::record_selection_decision",
            Self::ModelCatalogRecordSelectionDecisionWithContext => {
                "ModelCatalog::record_selection_decision_with_context"
            }
            Self::ModelLaneAuthorityAdvisory => "ModelLaneAuthority::Advisory",
            Self::ModelLaneAuthorityPromoted => "ModelLaneAuthority::Promoted",
            Self::ModelLaneErrorInvalidInput => "ModelLaneError::InvalidInput",
            Self::ModelLaneStatusCancelled => "ModelLaneStatus::Cancelled",
            Self::ModelLaneStoreConsumeContextBundleForDownstream => {
                "ModelLaneStore::consume_context_bundle_for_downstream"
            }
            Self::ModelLaneStoreDiagnosticsProjection => "ModelLaneStore::diagnostics_projection",
            Self::ModelLaneStorePreflightCloudSpawnRequest => {
                "ModelLaneStore::preflight_cloud_spawn_request"
            }
            Self::ModelLaneStoreRecordCloudConsentReceipt => {
                "ModelLaneStore::record_cloud_consent_receipt"
            }
            Self::ModelLaneStoreRecordCloudProjectionPlan => {
                "ModelLaneStore::record_cloud_projection_plan"
            }
            Self::ModelLaneStoreRecordContextBundleArtifactBinding => {
                "ModelLaneStore::record_context_bundle_artifact_binding"
            }
            Self::ModelLaneStoreRecordContextBundleHandoff => {
                "ModelLaneStore::record_context_bundle_handoff"
            }
            Self::ModelLaneStoreRecordMessage => "ModelLaneStore::record_message",
            Self::ModelLaneStoreRecordPromotionDecision => {
                "ModelLaneStore::record_promotion_decision"
            }
            Self::ModelLaneStoreRecoverRunAfterRestart => {
                "ModelLaneStore::recover_run_after_restart"
            }
            Self::ModelLaneStoreReplayCloudConsentAuthority => {
                "ModelLaneStore::replay_cloud_consent_authority"
            }
            Self::ModelLaneStoreReplayContextBundleHandoffs => {
                "ModelLaneStore::replay_context_bundle_handoffs"
            }
            Self::ModelLaneStoreReplayPromotionDecisions => {
                "ModelLaneStore::replay_promotion_decisions"
            }
            Self::ModelLaneStoreReplayRun => "ModelLaneStore::replay_run",
            Self::ModelLaneStoreValidateDiagnosticTierPosture => {
                "ModelLaneStore::validate_diagnostic_tier_posture"
            }
            Self::ModelRuntimeErrorAdapterMismatch => "ModelRuntimeError::AdapterMismatch",
            Self::ProcessReclaimRuntimeProductionWithLease => {
                "ProcessReclaimRuntime::production_with_lease"
            }
            Self::ProviderKindExternalCompat => "ProviderKind::ExternalCompat",
            Self::ProviderKindLocal => "ProviderKind::Local",
            Self::ProviderRegistryFromEnv => "ProviderRegistry::from_env",
            Self::ReclaimRunOwnedProcess => "Reclaim::run_owned_process",
            Self::RoleDialog => "Role::Dialog",
            Self::RoleWindow => "Role::Window",
            Self::RunBudgetDefaulted => "RunBudget::defaulted",
            Self::RuntimeBindingSubagent => "RuntimeBinding::Subagent",
            Self::SemanticUnavailableReasonDimMismatch => "SemanticUnavailableReason::DimMismatch",
            Self::SemanticUnavailableReasonNoModel => "SemanticUnavailableReason::NoModel",
            Self::SpawnRequestWithDexterityLaunch => "SpawnRequest::with_dexterity_launch",
            Self::SwarmCoordinatorContextBundleForDownstreamLane => {
                "SwarmCoordinator::context_bundle_for_downstream_lane"
            }
            Self::SwarmCoordinatorInvokeDownstreamContextBundle => {
                "SwarmCoordinator::invoke_downstream_context_bundle"
            }
            Self::SwarmCoordinatorLaunchOperatorSubagentModelLane => {
                "SwarmCoordinator::launch_operator_subagent_model_lane"
            }
            Self::SwarmCoordinatorLiveSessionCount => "SwarmCoordinator::live_session_count",
            Self::SwarmCoordinatorMaxConcurrent => "SwarmCoordinator::max_concurrent",
            Self::SwarmCoordinatorRemaining => "SwarmCoordinator::remaining",
            Self::SwarmCoordinatorRetryPendingSessionCleanups => {
                "SwarmCoordinator::retry_pending_session_cleanups"
            }
            Self::SwarmCoordinatorRevokeCloudConsentReceipt => {
                "SwarmCoordinator::revoke_cloud_consent_receipt"
            }
            Self::SwarmCoordinatorSessionRuntime => "SwarmCoordinator::session_runtime",
            Self::SwarmCoordinatorSetMaxConcurrent => "SwarmCoordinator::set_max_concurrent",
            Self::SwarmCoordinatorSpawnSession => "SwarmCoordinator::spawn_session",
            Self::SwarmErrorBreakerOpen => "SwarmError::BreakerOpen",
            Self::SwarmErrorBudgetExhausted => "SwarmError::BudgetExhausted",
            Self::SwarmErrorConcurrencyCapReached => "SwarmError::ConcurrencyCapReached",
            Self::SwarmErrorDuplicateInstance => "SwarmError::DuplicateInstance",
            Self::SwarmErrorLifetimeSpawnCeilingReached => {
                "SwarmError::LifetimeSpawnCeilingReached"
            }
            Self::SwarmErrorProviderNotConfigured => "SwarmError::ProviderNotConfigured",
            Self::SwarmErrorClassBreakerOpen => "SwarmErrorClass::BreakerOpen",
            Self::SwarmEventBreakerTripped => "SwarmEvent::BreakerTripped",
            Self::SwarmEventSpawnRejected => "SwarmEvent::SpawnRejected",
            Self::SwarmFrEventIdSpawnRejected => "SwarmFrEventId::SpawnRejected",
            Self::SystemScopeAuthorityBootRecovery => "SystemScopeAuthority::boot_recovery",
            Self::UpdateDecodeV1 => "Update::decode_v1",
            Self::WorktreeVmRegistryTeardownWorktreeVm => {
                "WorktreeVmRegistry::teardown_worktree_vm"
            }
        }
    }

    fn proof(self) -> ManualNamedSymbolProof {
        match self {
            Self::AccountBoundAuthorityAccount => ManualNamedSymbolProof::CompileAnchored,
            Self::AccountBoundAuthorityUnattributed => ManualNamedSymbolProof::CompileAnchored,
            Self::AdmitDecisionSuppress => ManualNamedSymbolProof::CompileAnchored,
            Self::AppStateModelCatalog => ManualNamedSymbolProof::CompileAnchored,
            Self::BreakerStateClosed => ManualNamedSymbolProof::CompileAnchored,
            Self::BreakerStateHalfOpen => ManualNamedSymbolProof::CompileAnchored,
            Self::BreakerStateOpen => ManualNamedSymbolProof::CompileAnchored,
            Self::ConsoleCategoryBreaker => ManualNamedSymbolProof::CompileAnchored,
            Self::ConsoleCategorySpawnRejected => ManualNamedSymbolProof::CompileAnchored,
            Self::ConsoleSwarmSinkShared => ManualNamedSymbolProof::CompileAnchored,
            Self::DexterityLaunchAdapterRegistryAdapterKindForSpawnRequest => ManualNamedSymbolProof::CompileAnchored,
            Self::DexterityLaunchContractAttachToSpawnRequest => ManualNamedSymbolProof::CompileAnchored,
            Self::DexterityLaunchContractRestartGeneration => ManualNamedSymbolProof::CompileAnchored,
            Self::DisabledLlmClientCompletion => ManualNamedSymbolProof::CompileAnchored,
            Self::DisabledLlmClientEmbedding => ManualNamedSymbolProof::CompileAnchored,
            Self::EmbeddedModelProcessRecordReservedLoadWithDurableAck => ManualNamedSymbolProof::DeclaredNotNameableHere(
                "pub(super) inside llm::embedded_ledger; not nameable from user_manual. Verified by inspection at src/llm/embedded_ledger.rs.",
            ),
            Self::EmbeddedModelProcessShutdown => ManualNamedSymbolProof::CompileAnchored,
            Self::EngineOriginValidatorValidateLoadSpec => ManualNamedSymbolProof::CompileAnchored,
            Self::ExternalEngineImportRecordNew => ManualNamedSymbolProof::CompileAnchored,
            Self::FailureFingerprintCompute => ManualNamedSymbolProof::CompileAnchored,
            Self::FinishReasonCancelled => ManualNamedSymbolProof::CompileAnchored,
            Self::FlightRecorderEventTypeLlmInference => ManualNamedSymbolProof::CompileAnchored,
            Self::FlightRecorderSwarmSinkNew => ManualNamedSymbolProof::CompileAnchored,
            Self::GuardedCliChildTerminateAndCollect => ManualNamedSymbolProof::DeclaredNotNameableHere(
                "module-private struct GuardedCliChild inside model_runtime::cloud::official_cli_bridge; not nameable from user_manual. Verified by inspection at src/model_runtime/cloud/official_cli_bridge.rs.",
            ),
            Self::HandshakeNativeSandboxAdapterSpawnAttachedWithStdio => ManualNamedSymbolProof::CompileAnchored,
            Self::IsolationTierTier3Microvm => ManualNamedSymbolProof::CompileAnchored,
            Self::KernelEventTypeModelRuntimeSelectionRecorded => ManualNamedSymbolProof::CompileAnchored,
            Self::KillOutcomeFailed => ManualNamedSymbolProof::CompileAnchored,
            Self::LaunchAuthoritySubagentManager => ManualNamedSymbolProof::CompileAnchored,
            Self::LiveCliSpawnerSpawn => ManualNamedSymbolProof::CompileAnchored,
            Self::LlmClientEmbedding => ManualNamedSymbolProof::CompileAnchored,
            Self::LlmClientShutdownGracefully => ManualNamedSymbolProof::CompileAnchored,
            Self::LocalModelAdapterInvariantValidate => ManualNamedSymbolProof::CompileAnchored,
            Self::LocalModelRuntimeLlmClientEmbedding => ManualNamedSymbolProof::CompileAnchored,
            Self::ModelCatalogEmbeddingModelForDim => ManualNamedSymbolProof::CompileAnchored,
            Self::ModelCatalogList => ManualNamedSymbolProof::CompileAnchored,
            Self::ModelCatalogRecordSelectionDecision => ManualNamedSymbolProof::CompileAnchored,
            Self::ModelCatalogRecordSelectionDecisionWithContext => ManualNamedSymbolProof::CompileAnchored,
            Self::ModelLaneAuthorityAdvisory => ManualNamedSymbolProof::CompileAnchored,
            Self::ModelLaneAuthorityPromoted => ManualNamedSymbolProof::CompileAnchored,
            Self::ModelLaneErrorInvalidInput => ManualNamedSymbolProof::CompileAnchored,
            Self::ModelLaneStatusCancelled => ManualNamedSymbolProof::CompileAnchored,
            Self::ModelLaneStoreConsumeContextBundleForDownstream => ManualNamedSymbolProof::CompileAnchored,
            Self::ModelLaneStoreDiagnosticsProjection => ManualNamedSymbolProof::CompileAnchored,
            Self::ModelLaneStorePreflightCloudSpawnRequest => ManualNamedSymbolProof::CompileAnchored,
            Self::ModelLaneStoreRecordCloudConsentReceipt => ManualNamedSymbolProof::CompileAnchored,
            Self::ModelLaneStoreRecordCloudProjectionPlan => ManualNamedSymbolProof::CompileAnchored,
            Self::ModelLaneStoreRecordContextBundleArtifactBinding => ManualNamedSymbolProof::CompileAnchored,
            Self::ModelLaneStoreRecordContextBundleHandoff => ManualNamedSymbolProof::CompileAnchored,
            Self::ModelLaneStoreRecordMessage => ManualNamedSymbolProof::CompileAnchored,
            Self::ModelLaneStoreRecordPromotionDecision => ManualNamedSymbolProof::CompileAnchored,
            Self::ModelLaneStoreRecoverRunAfterRestart => ManualNamedSymbolProof::CompileAnchored,
            Self::ModelLaneStoreReplayCloudConsentAuthority => ManualNamedSymbolProof::CompileAnchored,
            Self::ModelLaneStoreReplayContextBundleHandoffs => ManualNamedSymbolProof::CompileAnchored,
            Self::ModelLaneStoreReplayPromotionDecisions => ManualNamedSymbolProof::CompileAnchored,
            Self::ModelLaneStoreReplayRun => ManualNamedSymbolProof::CompileAnchored,
            Self::ModelLaneStoreValidateDiagnosticTierPosture => ManualNamedSymbolProof::CompileAnchored,
            Self::ModelRuntimeErrorAdapterMismatch => ManualNamedSymbolProof::CompileAnchored,
            Self::ProcessReclaimRuntimeProductionWithLease => ManualNamedSymbolProof::CompileAnchored,
            Self::ProviderKindExternalCompat => ManualNamedSymbolProof::CompileAnchored,
            Self::ProviderKindLocal => ManualNamedSymbolProof::CompileAnchored,
            Self::ProviderRegistryFromEnv => ManualNamedSymbolProof::CompileAnchored,
            Self::ReclaimRunOwnedProcess => ManualNamedSymbolProof::CompileAnchored,
            Self::RoleDialog => ManualNamedSymbolProof::DeclaredNotNameableHere(
                "accesskit::Role belongs to the native frontend crate; handshake_core does not depend on accesskit, so the variant cannot be compile-anchored here. Proven instead by the native Argus tests named on the cloud-model-access page.",
            ),
            Self::RoleWindow => ManualNamedSymbolProof::DeclaredNotNameableHere(
                "accesskit::Role belongs to the native frontend crate; handshake_core does not depend on accesskit, so the variant cannot be compile-anchored here. Proven instead by the native Argus tests named on the cloud-model-access page.",
            ),
            Self::RunBudgetDefaulted => ManualNamedSymbolProof::CompileAnchored,
            Self::RuntimeBindingSubagent => ManualNamedSymbolProof::CompileAnchored,
            Self::SemanticUnavailableReasonDimMismatch => ManualNamedSymbolProof::CompileAnchored,
            Self::SemanticUnavailableReasonNoModel => ManualNamedSymbolProof::CompileAnchored,
            Self::SpawnRequestWithDexterityLaunch => ManualNamedSymbolProof::CompileAnchored,
            Self::SwarmCoordinatorContextBundleForDownstreamLane => ManualNamedSymbolProof::CompileAnchored,
            Self::SwarmCoordinatorInvokeDownstreamContextBundle => ManualNamedSymbolProof::CompileAnchored,
            Self::SwarmCoordinatorLaunchOperatorSubagentModelLane => ManualNamedSymbolProof::CompileAnchored,
            Self::SwarmCoordinatorLiveSessionCount => ManualNamedSymbolProof::CompileAnchored,
            Self::SwarmCoordinatorMaxConcurrent => ManualNamedSymbolProof::CompileAnchored,
            Self::SwarmCoordinatorRemaining => ManualNamedSymbolProof::CompileAnchored,
            Self::SwarmCoordinatorRetryPendingSessionCleanups => ManualNamedSymbolProof::CompileAnchored,
            Self::SwarmCoordinatorRevokeCloudConsentReceipt => ManualNamedSymbolProof::CompileAnchored,
            Self::SwarmCoordinatorSessionRuntime => ManualNamedSymbolProof::CompileAnchored,
            Self::SwarmCoordinatorSetMaxConcurrent => ManualNamedSymbolProof::CompileAnchored,
            Self::SwarmCoordinatorSpawnSession => ManualNamedSymbolProof::CompileAnchored,
            Self::SwarmErrorBreakerOpen => ManualNamedSymbolProof::CompileAnchored,
            Self::SwarmErrorBudgetExhausted => ManualNamedSymbolProof::CompileAnchored,
            Self::SwarmErrorConcurrencyCapReached => ManualNamedSymbolProof::CompileAnchored,
            Self::SwarmErrorDuplicateInstance => ManualNamedSymbolProof::CompileAnchored,
            Self::SwarmErrorLifetimeSpawnCeilingReached => ManualNamedSymbolProof::CompileAnchored,
            Self::SwarmErrorProviderNotConfigured => ManualNamedSymbolProof::CompileAnchored,
            Self::SwarmErrorClassBreakerOpen => ManualNamedSymbolProof::CompileAnchored,
            Self::SwarmEventBreakerTripped => ManualNamedSymbolProof::CompileAnchored,
            Self::SwarmEventSpawnRejected => ManualNamedSymbolProof::CompileAnchored,
            Self::SwarmFrEventIdSpawnRejected => ManualNamedSymbolProof::CompileAnchored,
            Self::SystemScopeAuthorityBootRecovery => ManualNamedSymbolProof::CompileAnchored,
            Self::UpdateDecodeV1 => ManualNamedSymbolProof::CompileAnchored,
            Self::WorktreeVmRegistryTeardownWorktreeVm => ManualNamedSymbolProof::CompileAnchored,
        }
    }

    /// Reference the real product item. A rename or removal in product code is a
    /// COMPILE error here, so a manual page can never outlive the surface it names.
    fn assert_compiled(self) {
        match self {
            Self::AccountBoundAuthorityAccount => {
                let _ =
                    |value: &crate::swarm_orchestration::resource_scope::AccountBoundAuthority| {
                        matches!(value, crate::swarm_orchestration::resource_scope::AccountBoundAuthority::Account { .. })
                    };
            }
            Self::AccountBoundAuthorityUnattributed => {
                let _ =
                    |value: &crate::swarm_orchestration::resource_scope::AccountBoundAuthority| {
                        matches!(value, crate::swarm_orchestration::resource_scope::AccountBoundAuthority::Unattributed { .. })
                    };
            }
            Self::AdmitDecisionSuppress => {
                let _ = |value: &crate::swarm_orchestration::AdmitDecision| {
                    matches!(
                        value,
                        crate::swarm_orchestration::AdmitDecision::Suppress { .. }
                    )
                };
            }
            Self::AppStateModelCatalog => {
                let _ = crate::AppState::model_catalog;
            }
            Self::BreakerStateClosed => {
                let _ = |value: &crate::swarm_orchestration::BreakerState| {
                    matches!(value, crate::swarm_orchestration::BreakerState::Closed)
                };
            }
            Self::BreakerStateHalfOpen => {
                let _ = |value: &crate::swarm_orchestration::BreakerState| {
                    matches!(value, crate::swarm_orchestration::BreakerState::HalfOpen)
                };
            }
            Self::BreakerStateOpen => {
                let _ = |value: &crate::swarm_orchestration::BreakerState| {
                    matches!(value, crate::swarm_orchestration::BreakerState::Open)
                };
            }
            Self::ConsoleCategoryBreaker => {
                let _ = |value: &crate::console_stream::ConsoleCategory| {
                    matches!(value, crate::console_stream::ConsoleCategory::Breaker)
                };
            }
            Self::ConsoleCategorySpawnRejected => {
                let _ = |value: &crate::console_stream::ConsoleCategory| {
                    matches!(value, crate::console_stream::ConsoleCategory::SpawnRejected)
                };
            }
            Self::ConsoleSwarmSinkShared => {
                let _ = crate::console_stream::ConsoleSwarmSink::shared;
            }
            Self::DexterityLaunchAdapterRegistryAdapterKindForSpawnRequest => {
                let _ = crate::swarm_orchestration::model_lane::DexterityLaunchAdapterRegistry::adapter_kind_for_spawn_request;
            }
            Self::DexterityLaunchContractAttachToSpawnRequest => {
                // `attach_to_spawn_request` declares argument-position `impl
                // Into<String>` parameters. Those cannot be turbofished
                // (E0107), and a bare function-item reference cannot infer them
                // either (E0283), so neither of the obvious anchor forms
                // compiles. Coerce the function item to a concrete fn pointer
                // instead: this still fails to compile if the symbol is renamed,
                // moved, or its arity/return type changes, which is exactly the
                // guarantee this anchor exists to provide -- and it additionally
                // pins the signature shape.
                let _: fn(
                    crate::swarm_orchestration::SpawnRequest,
                    String,
                    String,
                ) -> crate::swarm_orchestration::model_lane::ModelLaneResult<
                    crate::swarm_orchestration::SpawnRequest,
                > = crate::swarm_orchestration::model_lane::DexterityLaunchContract::attach_to_spawn_request;
            }
            Self::DexterityLaunchContractRestartGeneration => {
                let _ =
                    |value: &crate::swarm_orchestration::model_lane::DexterityLaunchContract| {
                        value.restart_generation
                    };
            }
            Self::DisabledLlmClientCompletion => {
                let _ = <crate::llm::DisabledLlmClient as crate::llm::LlmClient>::completion;
            }
            Self::DisabledLlmClientEmbedding => {
                let _ = <crate::llm::DisabledLlmClient as crate::llm::LlmClient>::embedding;
            }
            Self::EmbeddedModelProcessRecordReservedLoadWithDurableAck => {
                // Declared-with-reason; see `proof()`.
            }
            Self::EmbeddedModelProcessShutdown => {
                let _ = crate::llm::embedded_ledger::EmbeddedModelProcess::shutdown;
            }
            Self::EngineOriginValidatorValidateLoadSpec => {
                let _ = crate::model_runtime::EngineOriginValidator::validate_load_spec;
            }
            Self::ExternalEngineImportRecordNew => {
                // Argument-position `impl AsRef<str>` / `impl Into<String>`:
                // a bare fn-item reference cannot infer them (E0283) and they
                // cannot be turbofished (E0107). Coerce to a concrete fn
                // pointer, which also pins arity and return type.
                // `String` (not `&str`) for the `impl AsRef<str>` slot: a
                // borrowed instantiation would need a higher-ranked fn pointer
                // that the concrete instantiation cannot satisfy (E0308).
                let _: fn(
                    String,
                    bool,
                    chrono::DateTime<chrono::Utc>,
                    String,
                ) -> Result<
                    crate::model_runtime::ExternalEngineImportRecord,
                    crate::model_runtime::ModelRuntimeError,
                > = crate::model_runtime::ExternalEngineImportRecord::new;
            }
            Self::FailureFingerprintCompute => {
                let _ = crate::swarm_orchestration::FailureFingerprint::compute;
            }
            Self::FinishReasonCancelled => {
                let _ = |value: &crate::model_runtime::FinishReason| {
                    matches!(value, crate::model_runtime::FinishReason::Cancelled)
                };
            }
            Self::FlightRecorderEventTypeLlmInference => {
                let _ = |value: &crate::flight_recorder::FlightRecorderEventType| {
                    matches!(
                        value,
                        crate::flight_recorder::FlightRecorderEventType::LlmInference
                    )
                };
            }
            Self::FlightRecorderSwarmSinkNew => {
                let _ = crate::swarm_orchestration::FlightRecorderSwarmSink::<
                    fn(crate::flight_recorder::FlightRecorderEvent) -> Result<(), String>,
                >::new;
            }
            Self::GuardedCliChildTerminateAndCollect => {
                // Declared-with-reason; see `proof()`.
            }
            Self::HandshakeNativeSandboxAdapterSpawnAttachedWithStdio => {
                let _ = <crate::sandbox::HandshakeNativeSandboxAdapter as crate::sandbox::SandboxAdapter>::spawn_attached_with_stdio;
            }
            Self::IsolationTierTier3Microvm => {
                let _ = |value: &crate::sandbox::IsolationTier| {
                    matches!(value, crate::sandbox::IsolationTier::Tier3Microvm)
                };
            }
            Self::KernelEventTypeModelRuntimeSelectionRecorded => {
                let _ = |value: &crate::kernel::KernelEventType| {
                    matches!(
                        value,
                        crate::kernel::KernelEventType::ModelRuntimeSelectionRecorded
                    )
                };
            }
            Self::KillOutcomeFailed => {
                let _ = |value: &crate::process_ledger::KillOutcome| {
                    matches!(value, crate::process_ledger::KillOutcome::Failed { .. })
                };
            }
            Self::LaunchAuthoritySubagentManager => {
                let _ = |value: &crate::swarm_orchestration::model_lane::LaunchAuthority| {
                    matches!(
                        value,
                        crate::swarm_orchestration::model_lane::LaunchAuthority::SubagentManager
                    )
                };
            }
            Self::LiveCliSpawnerSpawn => {
                let _ = <crate::model_runtime::cloud::LiveCliSpawner as crate::model_runtime::cloud::CliSubprocessSpawner>::spawn;
            }
            Self::LlmClientEmbedding => {
                let _ = <crate::llm::DisabledLlmClient as crate::llm::LlmClient>::embedding;
            }
            Self::LlmClientShutdownGracefully => {
                let _ =
                    <crate::llm::DisabledLlmClient as crate::llm::LlmClient>::shutdown_gracefully;
            }
            Self::LocalModelAdapterInvariantValidate => {
                let _ = crate::model_runtime::LocalModelAdapterInvariant::validate;
            }
            Self::LocalModelRuntimeLlmClientEmbedding => {
                let _ = <crate::llm::local_router::LocalModelRuntimeLlmClient as crate::llm::LlmClient>::embedding;
            }
            Self::ModelCatalogEmbeddingModelForDim => {
                let _ = crate::model_runtime::ModelCatalog::embedding_model_for_dim;
            }
            Self::ModelCatalogList => {
                let _ = crate::model_runtime::ModelCatalog::list;
            }
            Self::ModelCatalogRecordSelectionDecision => {
                let _ = crate::model_runtime::ModelCatalog::record_selection_decision;
            }
            Self::ModelCatalogRecordSelectionDecisionWithContext => {
                let _ = crate::model_runtime::ModelCatalog::record_selection_decision_with_context;
            }
            Self::ModelLaneAuthorityAdvisory => {
                let _ = |value: &crate::swarm_orchestration::model_lane::ModelLaneAuthority| {
                    matches!(
                        value,
                        crate::swarm_orchestration::model_lane::ModelLaneAuthority::Advisory
                    )
                };
            }
            Self::ModelLaneAuthorityPromoted => {
                let _ = |value: &crate::swarm_orchestration::model_lane::ModelLaneAuthority| {
                    matches!(
                        value,
                        crate::swarm_orchestration::model_lane::ModelLaneAuthority::Promoted
                    )
                };
            }
            Self::ModelLaneErrorInvalidInput => {
                let _ = |value: &crate::swarm_orchestration::model_lane::ModelLaneError| {
                    matches!(
                        value,
                        crate::swarm_orchestration::model_lane::ModelLaneError::InvalidInput(..)
                    )
                };
            }
            Self::ModelLaneStatusCancelled => {
                let _ = |value: &crate::swarm_orchestration::model_lane::ModelLaneStatus| {
                    matches!(
                        value,
                        crate::swarm_orchestration::model_lane::ModelLaneStatus::Cancelled
                    )
                };
            }
            Self::ModelLaneStoreConsumeContextBundleForDownstream => {
                let _ = crate::swarm_orchestration::model_lane::ModelLaneStore::consume_context_bundle_for_downstream;
            }
            Self::ModelLaneStoreDiagnosticsProjection => {
                let _ =
                    crate::swarm_orchestration::model_lane::ModelLaneStore::diagnostics_projection;
            }
            Self::ModelLaneStorePreflightCloudSpawnRequest => {
                let _ = crate::swarm_orchestration::model_lane::ModelLaneStore::preflight_cloud_spawn_request;
            }
            Self::ModelLaneStoreRecordCloudConsentReceipt => {
                let _ = crate::swarm_orchestration::model_lane::ModelLaneStore::record_cloud_consent_receipt;
            }
            Self::ModelLaneStoreRecordCloudProjectionPlan => {
                let _ = crate::swarm_orchestration::model_lane::ModelLaneStore::record_cloud_projection_plan;
            }
            Self::ModelLaneStoreRecordContextBundleArtifactBinding => {
                let _ = crate::swarm_orchestration::model_lane::ModelLaneStore::record_context_bundle_artifact_binding;
            }
            Self::ModelLaneStoreRecordContextBundleHandoff => {
                let _ = crate::swarm_orchestration::model_lane::ModelLaneStore::record_context_bundle_handoff;
            }
            Self::ModelLaneStoreRecordMessage => {
                let _ = crate::swarm_orchestration::model_lane::ModelLaneStore::record_message;
            }
            Self::ModelLaneStoreRecordPromotionDecision => {
                let _ = crate::swarm_orchestration::model_lane::ModelLaneStore::record_promotion_decision;
            }
            Self::ModelLaneStoreRecoverRunAfterRestart => {
                let _ = crate::swarm_orchestration::model_lane::ModelLaneStore::recover_run_after_restart;
            }
            Self::ModelLaneStoreReplayCloudConsentAuthority => {
                let _ = crate::swarm_orchestration::model_lane::ModelLaneStore::replay_cloud_consent_authority;
            }
            Self::ModelLaneStoreReplayContextBundleHandoffs => {
                let _ = crate::swarm_orchestration::model_lane::ModelLaneStore::replay_context_bundle_handoffs;
            }
            Self::ModelLaneStoreReplayPromotionDecisions => {
                let _ = crate::swarm_orchestration::model_lane::ModelLaneStore::replay_promotion_decisions;
            }
            Self::ModelLaneStoreReplayRun => {
                let _ = crate::swarm_orchestration::model_lane::ModelLaneStore::replay_run;
            }
            Self::ModelLaneStoreValidateDiagnosticTierPosture => {
                let _ = crate::swarm_orchestration::model_lane::ModelLaneStore::validate_diagnostic_tier_posture;
            }
            Self::ModelRuntimeErrorAdapterMismatch => {
                let _ = |value: &crate::model_runtime::ModelRuntimeError| {
                    matches!(
                        value,
                        crate::model_runtime::ModelRuntimeError::AdapterMismatch { .. }
                    )
                };
            }
            Self::ProcessReclaimRuntimeProductionWithLease => {
                let _ = crate::process_ledger::ProcessReclaimRuntime::production_with_lease;
            }
            Self::ProviderKindExternalCompat => {
                let _ = |value: &crate::model_runtime::ProviderKind| {
                    matches!(value, crate::model_runtime::ProviderKind::ExternalCompat)
                };
            }
            Self::ProviderKindLocal => {
                let _ = |value: &crate::model_runtime::ProviderKind| {
                    matches!(value, crate::model_runtime::ProviderKind::Local)
                };
            }
            Self::ProviderRegistryFromEnv => {
                let _ = crate::llm::registry::ProviderRegistry::from_env;
            }
            Self::ReclaimRunOwnedProcess => {
                let _ = crate::process_ledger::Reclaim::run_owned_process;
            }
            Self::RoleDialog => {
                // Declared-with-reason; see `proof()`.
            }
            Self::RoleWindow => {
                // Declared-with-reason; see `proof()`.
            }
            Self::RunBudgetDefaulted => {
                let _ = crate::swarm_orchestration::RunBudget::defaulted;
            }
            Self::RuntimeBindingSubagent => {
                let _ = |value: &crate::swarm_orchestration::model_lane::RuntimeBinding| {
                    matches!(
                        value,
                        crate::swarm_orchestration::model_lane::RuntimeBinding::Subagent
                    )
                };
            }
            Self::SemanticUnavailableReasonDimMismatch => {
                let _ = |value: &crate::storage::loom::SemanticUnavailableReason| {
                    matches!(
                        value,
                        crate::storage::loom::SemanticUnavailableReason::DimMismatch { .. }
                    )
                };
            }
            Self::SemanticUnavailableReasonNoModel => {
                let _ = |value: &crate::storage::loom::SemanticUnavailableReason| {
                    matches!(
                        value,
                        crate::storage::loom::SemanticUnavailableReason::NoModel
                    )
                };
            }
            Self::SpawnRequestWithDexterityLaunch => {
                let _ = crate::swarm_orchestration::SpawnRequest::with_dexterity_launch;
            }
            Self::SwarmCoordinatorContextBundleForDownstreamLane => {
                let _ = crate::swarm_orchestration::SwarmCoordinator::context_bundle_for_downstream_lane;
            }
            Self::SwarmCoordinatorInvokeDownstreamContextBundle => {
                let _ =
                    crate::swarm_orchestration::SwarmCoordinator::invoke_downstream_context_bundle;
            }
            Self::SwarmCoordinatorLaunchOperatorSubagentModelLane => {
                let _ = crate::swarm_orchestration::SwarmCoordinator::launch_operator_subagent_model_lane;
            }
            Self::SwarmCoordinatorLiveSessionCount => {
                let _ = crate::swarm_orchestration::SwarmCoordinator::live_session_count;
            }
            Self::SwarmCoordinatorMaxConcurrent => {
                let _ = crate::swarm_orchestration::SwarmCoordinator::max_concurrent;
            }
            Self::SwarmCoordinatorRemaining => {
                let _ = crate::swarm_orchestration::SwarmCoordinator::remaining;
            }
            Self::SwarmCoordinatorRetryPendingSessionCleanups => {
                let _ =
                    crate::swarm_orchestration::SwarmCoordinator::retry_pending_session_cleanups;
            }
            Self::SwarmCoordinatorRevokeCloudConsentReceipt => {
                let _ = crate::swarm_orchestration::SwarmCoordinator::revoke_cloud_consent_receipt;
            }
            Self::SwarmCoordinatorSessionRuntime => {
                // `SwarmCoordinator::session_runtime` is itself
                // `#[cfg(any(test, feature = "test-utils"))]` (coordinator.rs
                // ~2117). An UNGATED compile anchor on it breaks the DEFAULT
                // build of `handshake_core`, which in turn breaks every
                // dependent crate -- including the Tauri shell, whose default
                // configuration does not enable `test-utils`. The anchor must
                // therefore carry the same gate as the symbol it anchors.
                //
                // The guarantee is preserved where it can be enforced: under
                // test/test-utils (the configuration every gate in this module
                // actually runs in) a rename or removal is still a compile
                // error. Outside that configuration the symbol does not exist
                // to be anchored.
                #[cfg(any(test, feature = "test-utils"))]
                {
                    let _ = crate::swarm_orchestration::SwarmCoordinator::session_runtime;
                }
            }
            Self::SwarmCoordinatorSetMaxConcurrent => {
                let _ = crate::swarm_orchestration::SwarmCoordinator::set_max_concurrent;
            }
            Self::SwarmCoordinatorSpawnSession => {
                let _ = crate::swarm_orchestration::SwarmCoordinator::spawn_session;
            }
            Self::SwarmErrorBreakerOpen => {
                let _ = |value: &crate::swarm_orchestration::SwarmError| {
                    matches!(
                        value,
                        crate::swarm_orchestration::SwarmError::BreakerOpen { .. }
                    )
                };
            }
            Self::SwarmErrorBudgetExhausted => {
                let _ = |value: &crate::swarm_orchestration::SwarmError| {
                    matches!(
                        value,
                        crate::swarm_orchestration::SwarmError::BudgetExhausted { .. }
                    )
                };
            }
            Self::SwarmErrorConcurrencyCapReached => {
                let _ = |value: &crate::swarm_orchestration::SwarmError| {
                    matches!(
                        value,
                        crate::swarm_orchestration::SwarmError::ConcurrencyCapReached { .. }
                    )
                };
            }
            Self::SwarmErrorDuplicateInstance => {
                let _ = |value: &crate::swarm_orchestration::SwarmError| {
                    matches!(
                        value,
                        crate::swarm_orchestration::SwarmError::DuplicateInstance(..)
                    )
                };
            }
            Self::SwarmErrorLifetimeSpawnCeilingReached => {
                let _ = |value: &crate::swarm_orchestration::SwarmError| {
                    matches!(
                        value,
                        crate::swarm_orchestration::SwarmError::LifetimeSpawnCeilingReached { .. }
                    )
                };
            }
            Self::SwarmErrorProviderNotConfigured => {
                let _ = |value: &crate::swarm_orchestration::SwarmError| {
                    matches!(
                        value,
                        crate::swarm_orchestration::SwarmError::ProviderNotConfigured { .. }
                    )
                };
            }
            Self::SwarmErrorClassBreakerOpen => {
                let _ = |value: &crate::swarm_orchestration::SwarmErrorClass| {
                    matches!(
                        value,
                        crate::swarm_orchestration::SwarmErrorClass::BreakerOpen
                    )
                };
            }
            Self::SwarmEventBreakerTripped => {
                let _ = |value: &crate::swarm_orchestration::SwarmEvent| {
                    matches!(
                        value,
                        crate::swarm_orchestration::SwarmEvent::BreakerTripped { .. }
                    )
                };
            }
            Self::SwarmEventSpawnRejected => {
                let _ = |value: &crate::swarm_orchestration::SwarmEvent| {
                    matches!(
                        value,
                        crate::swarm_orchestration::SwarmEvent::SpawnRejected { .. }
                    )
                };
            }
            Self::SwarmFrEventIdSpawnRejected => {
                let _ = |value: &crate::swarm_orchestration::SwarmFrEventId| {
                    matches!(
                        value,
                        crate::swarm_orchestration::SwarmFrEventId::SpawnRejected
                    )
                };
            }
            Self::SystemScopeAuthorityBootRecovery => {
                let _ =
                    crate::swarm_orchestration::resource_scope::SystemScopeAuthority::boot_recovery;
            }
            Self::UpdateDecodeV1 => {
                let _ = <yrs::Update as yrs::updates::decoder::Decode>::decode_v1;
            }
            Self::WorktreeVmRegistryTeardownWorktreeVm => {
                let _ = crate::swarm_orchestration::WorktreeVmRegistry::teardown_worktree_vm;
            }
        }
    }
}

/// Flight Recorder event families the manual is allowed to name as PLANNED /
/// DEFERRED wiring. This is the HBR-MAN-003 escape hatch and it is deliberately
/// tiny and explicit: each entry must carry the reason the family has no
/// compiled id yet. Adding an entry is a reviewable act, not a side effect.
const PLANNED_FLIGHT_RECORDER_EVENT_FAMILIES: &[(&str, &str)] = &[
    (
        "FR-EVT-CLOUD",
        "cloud-escalation events are documented in flight_recorder (FR-EVT-CLOUD-001..004) but are \
         not yet exposed as compiled id constants; the cloud pages cite this family only inside an \
         explicit DEFERRED-with-reason HBR-INT-009 posture.",
    ),
    (
        "FR-EVT-DEXTERITY",
        "the Dexterity promotion-decision Flight Recorder family is not wired yet; the promotion \
         page cites it only inside an explicit DEFERRED-with-reason HBR-INT-009 posture, and \
         promotion authority today is durable EventLedger rows.",
    ),
];

/// Routes the manual is allowed to name that are deliberately NOT WP-009
/// registry rows. Same discipline as [`PLANNED_FLIGHT_RECORDER_EVENT_FAMILIES`]:
/// tiny, explicit, and every entry carries the reason it is not a
/// [`SurfaceDescriptor`](super::registry::SurfaceDescriptor). Adding an entry is
/// a reviewable act, not a side effect.
const NON_REGISTRY_DOCUMENTED_ROUTES: &[(&str, &str, &str)] = &[(
    "GET",
    "/health",
    "the liveness probe mounted directly on the root axum router in `main.rs`; \
     it is an operational readiness endpoint with no workspace, identity, or \
     model semantics, so it owns no SurfaceGroup page and no SurfaceDescriptor \
     row. The startup page documents it as a `curl` liveness check only.",
)];

/// Every Flight Recorder event id the product can actually emit, assembled from
/// compiled sources only: two exhaustive enums plus the free-standing `pub const`
/// ids that have not been folded into `FrEventId` yet. Renaming any of them is a
/// compile error here.
fn compiled_flight_recorder_event_ids() -> Vec<&'static str> {
    let mut ids: Vec<&'static str> = crate::flight_recorder::fr_event_registry::FrEventId::all()
        .iter()
        .map(|id| id.as_str())
        .collect();
    ids.extend(
        crate::swarm_orchestration::SwarmFrEventId::all()
            .iter()
            .map(|id| id.as_str()),
    );
    ids.extend_from_slice(&[
        crate::flight_recorder::events_agent_activity::FR_EVT_AGENT_TOOLCALL,
        crate::flight_recorder::events_agent_activity::FR_EVT_AGENT_THINKING,
        crate::flight_recorder::events_agent_activity::FR_EVT_AGENT_TEXT,
        crate::flight_recorder::events_agent_activity::FR_EVT_AGENT_OTHER,
        crate::loom_search::LOOM_SEMANTIC_DEGRADED_FR_EVENT,
        crate::model_runtime::MODEL_SELECTION_FR_EVENT,
        crate::kernel::work_profiles::PROVIDER_REF_MIGRATION_FR_EVENT,
    ]);
    ids.sort_unstable();
    ids.dedup();
    ids
}

/// What one clean [`verify_manual_named_surface_existence`] run actually checked.
/// Counts are reported so a "gate passed" claim can be distinguished from a gate
/// that silently scanned nothing.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct ManualNamedSurfaceProof {
    pub pages_scanned: usize,
    pub sections_scanned: usize,
    /// Sections whose `body_json` was walked for string leaves as well as
    /// `body_md`. Before MT-022 this was always zero: JSON prose was unchecked.
    pub json_sections_scanned: usize,
    pub symbol_claims_checked: usize,
    /// `METHOD /path` spans resolved against [`wp009_surface_registry`].
    pub route_claims_checked: usize,
    pub flight_recorder_event_claims_checked: usize,
    /// Spans classified as string-literal claims and handed to
    /// [`verify_manual_literal_claims`]. Counted here so a caller can see that
    /// this gate alone did NOT prove them.
    pub literal_claims_collected: usize,
    pub compiled_symbol_vocabulary: usize,
    pub compiled_flight_recorder_vocabulary: usize,
}

/// Which string-literal family a [`ManualLiteralClaim`] belongs to. Reported so
/// a failure says what kind of name drifted, not just that something did.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ManualClaimClass {
    /// `HANDSHAKE_LLM_PROVIDER`, `KNOWLEDGE_INDEX_RUN_COMPLETED`, `DORMANT` —
    /// env vars, EventLedger event types, and state labels.
    ScreamingSnake,
    /// `max_concurrent`, `breaker_open`, `kernel_event_ledger` — config keys,
    /// JSON fields, table names, proof-target names.
    SnakeCase,
    /// `hsk.model_lane_run@1` — a typed schema id.
    SchemaId,
    /// A bare `CamelCase` name the compiled symbol vocabulary does not anchor.
    BareTypeName,
}

impl ManualClaimClass {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ScreamingSnake => "screaming_snake",
            Self::SnakeCase => "snake_case",
            Self::SchemaId => "schema_id",
            Self::BareTypeName => "bare_type_name",
        }
    }
}

/// One manual claim whose truth lives in a product-source string literal rather
/// than in a Rust item this module can name. Resolved by
/// [`verify_manual_literal_claims`] against real product source text.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct ManualLiteralClaim {
    pub page_slug: String,
    pub section_kind: &'static str,
    pub class: ManualClaimClass,
    pub literal: String,
}

/// Backtick-delimited spans of a markdown body, in order.
fn backticked_spans(text: &str) -> Vec<&str> {
    let mut spans = Vec::new();
    let mut inside = false;
    for part in text.split('`') {
        if inside {
            spans.push(part);
        }
        inside = !inside;
    }
    spans
}

/// Every string leaf of a JSON section body, so prose parked in `body_json` is
/// scanned by the same rules as `body_md`.
fn json_string_leaves(value: &serde_json::Value, out: &mut Vec<String>) {
    match value {
        serde_json::Value::String(text) => out.push(text.clone()),
        serde_json::Value::Array(items) => {
            for item in items {
                json_string_leaves(item, out);
            }
        }
        serde_json::Value::Object(map) => {
            for item in map.values() {
                json_string_leaves(item, out);
            }
        }
        _ => {}
    }
}

/// `true` when every char is legal inside one Rust path segment.
fn is_ident_segment(segment: &str) -> bool {
    !segment.is_empty()
        && segment
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_')
}

/// `true` when `value` has the shape of a Rust path rooted at a type or enum:
/// an uppercase-initial first segment, optionally followed by `::segment`s.
/// MT-022: a BARE `CamelCase` name (zero `::` segments) now qualifies, so bare
/// type names are no longer invisible to the gate.
fn is_rust_symbol_path(value: &str) -> bool {
    let mut segments = value.split("::");
    let Some(first) = segments.next() else {
        return false;
    };
    if !first.chars().next().is_some_and(|c| c.is_ascii_uppercase()) {
        return false;
    }
    if !is_ident_segment(first) {
        return false;
    }
    segments.all(is_ident_segment)
}

/// The symbol claim carried by one backticked span, if any. `Foo::bar(x)`,
/// `Foo::Bar { .. }` and `Foo::<T>::bar` all reduce to their path head.
fn manual_symbol_candidate(span: &str) -> Option<&str> {
    let cut = span
        .find(|c: char| matches!(c, '(' | '{' | '<' | '['))
        .unwrap_or(span.len());
    let candidate = span[..cut].trim();
    if candidate.is_empty() || !is_rust_symbol_path(candidate) {
        return None;
    }
    Some(candidate)
}

/// A `METHOD /path` claim carried by a backticked span.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct RouteClaim<'a> {
    /// The method token exactly as the manual wrote it.
    method_as_written: &'a str,
    /// The canonical uppercase method.
    method: &'static str,
    path: &'a str,
}

const HTTP_METHODS: [&str; 5] = ["GET", "POST", "PUT", "PATCH", "DELETE"];

/// The route claim carried by one backticked span, if any. Method matching is
/// case-INSENSITIVE on purpose: a lowercase `get /usermanual/pages` must be
/// caught and reported, not quietly skipped as "not a route".
fn manual_route_claim(span: &str) -> Option<RouteClaim<'_>> {
    let span = span.trim();
    let (method_as_written, rest) = span.split_once(' ')?;
    let rest = rest.trim();
    if !rest.starts_with('/') {
        return None;
    }
    // Query strings are documentation of parameters, not part of the mounted
    // route pattern; match on the path only.
    let path = rest.split('?').next()?;
    // A glob/ellipsis is a family reference (`/knowledge/code/*`), not a
    // callable claim; documented as out of scope in the scan rule above.
    if path.contains('*') || path.contains("...") || path.contains(' ') || path.is_empty() {
        return None;
    }
    let method = HTTP_METHODS
        .iter()
        .copied()
        .find(|m| m.eq_ignore_ascii_case(method_as_written))?;
    Some(RouteClaim {
        method_as_written,
        method,
        path,
    })
}

/// `true` when a documented path resolves to a mounted registry pattern.
///
/// Exact equality, or a CONCRETE instance of a pattern segment — so
/// `/usermanual/pages/manual-toc` resolves through `/usermanual/pages/:slug`.
/// A documented segment that is itself a `:param` must match the registry's
/// parameter name exactly: `/workspaces/:ws/...` does NOT resolve against
/// `/workspaces/:workspace_id/...`, because a model that copies the manual's
/// parameter names into its own client code is documenting a different API.
fn route_path_matches(documented: &str, registered: &str) -> bool {
    let mut doc = documented.split('/');
    let mut reg = registered.split('/');
    loop {
        match (doc.next(), reg.next()) {
            (None, None) => return true,
            (Some(d), Some(r)) => {
                if d == r {
                    continue;
                }
                if r.starts_with(':') && !d.starts_with(':') && !d.is_empty() {
                    continue;
                }
                return false;
            }
            _ => return false,
        }
    }
}

/// One `FR-EVT-` token found in a body.
#[derive(Debug, Clone, PartialEq, Eq)]
struct FlightRecorderTokenClaim {
    /// The token as written, uppercased. This is what is resolved.
    token: String,
    /// The token exactly as the manual wrote it.
    as_written: String,
}

/// Every `FR-EVT-...` token in a body, matched CASE-INSENSITIVELY so a
/// lowercase spelling cannot evade the gate, with any trailing separator or
/// wildcard trimmed so a family reference (`FR-EVT-AGENT-*`) reduces to its
/// prefix.
fn flight_recorder_event_tokens(text: &str) -> Vec<FlightRecorderTokenClaim> {
    const PREFIX: &str = "FR-EVT-";
    let upper = text.to_ascii_uppercase();
    let bytes = upper.as_bytes();
    let mut tokens = Vec::new();
    let mut cursor = 0usize;
    while let Some(offset) = upper[cursor..].find(PREFIX) {
        let start = cursor + offset;
        let mut end = start + PREFIX.len();
        while end < bytes.len()
            && (bytes[end].is_ascii_uppercase()
                || bytes[end].is_ascii_digit()
                || bytes[end] == b'-')
        {
            end += 1;
        }
        // `upper` is an ASCII-uppercased copy of `text`; ASCII case folding is
        // length-preserving, so the same byte range indexes both.
        let mut token = &upper[start..end];
        let mut as_written = &text[start..end];
        while let Some(trimmed) = token.strip_suffix('-') {
            token = trimmed;
            as_written = &as_written[..trimmed.len()];
        }
        tokens.push(FlightRecorderTokenClaim {
            token: token.to_string(),
            as_written: as_written.to_string(),
        });
        cursor = end;
    }
    tokens
}

/// `true` when `token` is a compiled event id, or names a family by prefixing
/// one at a `-` segment boundary. The boundary is what stops a truncation such
/// as `FR-EVT-S` from resolving against `FR-EVT-SWARM-SPAWN-REJECTED`.
fn flight_recorder_token_resolves(token: &str, vocabulary: &[&'static str]) -> bool {
    vocabulary.iter().any(|id| {
        *id == token
            || (id.len() > token.len()
                && id.starts_with(token)
                && id.as_bytes()[token.len()] == b'-')
    })
}

/// HBR-MAN-003 for the canonical `user_manual` corpus: every product symbol and
/// Flight Recorder event id the seeded prose names must actually exist.
///
/// Pass the compiled-in corpus (`seed::seed_corpus().pages`). That corpus is the
/// exact content the seeder writes to embedded SurrealDB — the stored row carries the
/// same `content_hash` — so checking it checks what an operator or a no-context
/// model will read back over `GET /usermanual/pages/:slug`.
///
/// Returns the scan counts on success, or one error per unresolvable claim.
pub fn verify_manual_named_surface_existence(
    pages: &[NewUserManualPage],
) -> Result<ManualNamedSurfaceProof, Vec<BehaviorCoverageError>> {
    let mut errors: Vec<BehaviorCoverageError> = Vec::new();

    // (1) Vocabulary integrity: every declared symbol is either bound to a real
    //     compiled item, or exempt WITH a written reason. `assert_compiled`
    //     turns a product-side rename into a build failure right here.
    let mut symbol_vocabulary: BTreeSet<&'static str> = BTreeSet::new();
    for symbol in ManualNamedSymbol::ALL {
        symbol.assert_compiled();
        let literal = symbol.literal();
        if !symbol_vocabulary.insert(literal) {
            errors.push(BehaviorCoverageError {
                behavior_id: MANUAL_NAMED_SURFACE_BEHAVIOR_ID,
                reason: format!("duplicate named-surface vocabulary entry `{literal}`"),
            });
        }
        if let ManualNamedSymbolProof::DeclaredNotNameableHere(reason) = symbol.proof() {
            if reason.trim().is_empty() {
                errors.push(BehaviorCoverageError {
                    behavior_id: MANUAL_NAMED_SURFACE_BEHAVIOR_ID,
                    reason: format!(
                        "named-surface entry `{literal}` is exempt from compile anchoring without a written reason"
                    ),
                });
            }
        }
    }
    for (family, reason) in PLANNED_FLIGHT_RECORDER_EVENT_FAMILIES {
        if reason.trim().is_empty() {
            errors.push(BehaviorCoverageError {
                behavior_id: MANUAL_NAMED_SURFACE_BEHAVIOR_ID,
                reason: format!("planned Flight Recorder family `{family}` has no written reason"),
            });
        }
    }

    // A BARE `CamelCase` name is anchored when it is a SEGMENT of a declared
    // path: `ModelLaneStore` is real because `ModelLaneStore::record_message`
    // compiles. Only uppercase-initial segments join this set, so a generic
    // method segment such as `list` can never resolve a prose token.
    let mut symbol_segment_vocabulary: BTreeSet<&'static str> = BTreeSet::new();
    for literal in &symbol_vocabulary {
        for segment in literal.split("::") {
            if segment
                .chars()
                .next()
                .is_some_and(|c| c.is_ascii_uppercase())
            {
                symbol_segment_vocabulary.insert(segment);
            }
        }
    }

    let event_vocabulary = compiled_flight_recorder_event_ids();
    let registry = wp009_surface_registry();

    // (2) Corpus scan — `body_md` AND every `body_json` string leaf.
    let mut sections_scanned = 0usize;
    let mut json_sections_scanned = 0usize;
    let mut symbol_claims_checked = 0usize;
    let mut route_claims_checked = 0usize;
    let mut flight_recorder_event_claims_checked = 0usize;
    let mut literal_claims_collected = 0usize;

    for page in pages {
        for section in &page.sections {
            sections_scanned += 1;
            let mut bodies: Vec<String> = vec![section.body_md.clone()];
            if let Some(json) = &section.body_json {
                json_sections_scanned += 1;
                json_string_leaves(json, &mut bodies);
            }

            for body in &bodies {
                for span in backticked_spans(body) {
                    if let Some(route) = manual_route_claim(span) {
                        route_claims_checked += 1;
                        if route.method_as_written != route.method {
                            errors.push(BehaviorCoverageError {
                                behavior_id: MANUAL_NAMED_SURFACE_BEHAVIOR_ID,
                                reason: format!(
                                    "UserManual page `{}` section `{}` writes route method `{}` in \
                                     `{} {}`; documented methods must be canonical uppercase `{}` \
                                     so a model copying the manual issues the request the router \
                                     actually matches (HBR-MAN-003).",
                                    page.slug,
                                    section.section_kind,
                                    route.method_as_written,
                                    route.method_as_written,
                                    route.path,
                                    route.method
                                ),
                            });
                            continue;
                        }
                        let mounted = registry.iter().any(|surface| {
                            surface.method == route.method
                                && route_path_matches(route.path, surface.route)
                        });
                        let exempt =
                            NON_REGISTRY_DOCUMENTED_ROUTES
                                .iter()
                                .any(|(method, path, _)| {
                                    *method == route.method && *path == route.path
                                });
                        if !mounted && !exempt {
                            errors.push(BehaviorCoverageError {
                                behavior_id: MANUAL_NAMED_SURFACE_BEHAVIOR_ID,
                                reason: format!(
                                    "UserManual page `{}` section `{}` documents route `{} {}`, \
                                     which resolves to no row of wp009_surface_registry() \
                                     (HBR-MAN-003). Either the route is not mounted, the method is \
                                     wrong, or the path parameter names drift from the mounted \
                                     pattern (for example `:ws` where the router mounts \
                                     `:workspace_id`). Fix the prose, add the missing \
                                     SurfaceDescriptor row, or declare it in \
                                     NON_REGISTRY_DOCUMENTED_ROUTES with a written reason.",
                                    page.slug, section.section_kind, route.method, route.path
                                ),
                            });
                        }
                        continue;
                    }

                    // Literal classes are tested BEFORE the symbol shape: a
                    // `SCREAMING_SNAKE` env var is uppercase-initial and made
                    // only of identifier characters, so the bare-symbol rule
                    // would otherwise swallow the whole env/event-type class.
                    if literal_claim_class(span).is_some() {
                        literal_claims_collected += 1;
                        continue;
                    }

                    if let Some(candidate) = manual_symbol_candidate(span) {
                        let bare = !candidate.contains("::");
                        if symbol_vocabulary.contains(candidate)
                            || (bare && symbol_segment_vocabulary.contains(candidate))
                        {
                            symbol_claims_checked += 1;
                            continue;
                        }
                        if bare {
                            // A bare name the compiled vocabulary does not
                            // anchor is not ignored: it degrades to a literal
                            // claim resolved against product source.
                            literal_claims_collected += 1;
                            continue;
                        }
                        symbol_claims_checked += 1;
                        errors.push(BehaviorCoverageError {
                            behavior_id: MANUAL_NAMED_SURFACE_BEHAVIOR_ID,
                            reason: format!(
                                "UserManual page `{}` section `{}` names product symbol `{}`, which is \
                                 not in the compiled named-surface vocabulary (HBR-MAN-003: the manual \
                                 must never name a surface that does not exist). Either the symbol was \
                                 renamed/removed in product code, or the claim is fabricated. Fix the \
                                 prose, or add a ManualNamedSymbol entry that compile-anchors it.",
                                page.slug, section.section_kind, candidate
                            ),
                        });
                    }
                }

                for claim in flight_recorder_event_tokens(body) {
                    flight_recorder_event_claims_checked += 1;
                    if claim.as_written != claim.token {
                        errors.push(BehaviorCoverageError {
                            behavior_id: MANUAL_NAMED_SURFACE_BEHAVIOR_ID,
                            reason: format!(
                                "UserManual page `{}` section `{}` writes Flight Recorder event \
                                 `{}`; compiled event ids are uppercase `{}`, and a lowercase \
                                 spelling would let a fabricated id evade this gate (HBR-MAN-003).",
                                page.slug, section.section_kind, claim.as_written, claim.token
                            ),
                        });
                        continue;
                    }
                    let resolved = flight_recorder_token_resolves(&claim.token, &event_vocabulary);
                    let planned = PLANNED_FLIGHT_RECORDER_EVENT_FAMILIES
                        .iter()
                        .any(|(family, _)| *family == claim.token);
                    if !resolved && !planned {
                        errors.push(BehaviorCoverageError {
                            behavior_id: MANUAL_NAMED_SURFACE_BEHAVIOR_ID,
                            reason: format!(
                                "UserManual page `{}` section `{}` names Flight Recorder event `{}`, \
                                 which is neither a compiled event id, nor a family prefix of one at \
                                 a `-` boundary, nor a declared PLANNED family (HBR-MAN-003).",
                                page.slug, section.section_kind, claim.token
                            ),
                        });
                    }
                }
            }
        }
    }

    if errors.is_empty() {
        Ok(ManualNamedSurfaceProof {
            pages_scanned: pages.len(),
            sections_scanned,
            json_sections_scanned,
            symbol_claims_checked,
            route_claims_checked,
            flight_recorder_event_claims_checked,
            literal_claims_collected,
            compiled_symbol_vocabulary: symbol_vocabulary.len(),
            compiled_flight_recorder_vocabulary: event_vocabulary.len(),
        })
    } else {
        Err(errors)
    }
}

/// The literal class a backticked span belongs to, if any.
fn literal_claim_class(span: &str) -> Option<ManualClaimClass> {
    let value = span.trim();
    if value.is_empty() || value.contains(char::is_whitespace) {
        return None;
    }
    // `hsk.<snake_name>@<digits>`
    if let Some((head, version)) = value.rsplit_once('@') {
        if let Some(name) = head.strip_prefix("hsk.") {
            if !name.is_empty()
                && !version.is_empty()
                && version.chars().all(|c| c.is_ascii_digit())
                && name
                    .chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' || c == '.')
            {
                return Some(ManualClaimClass::SchemaId);
            }
        }
    }
    if !is_ident_segment(value) {
        return None;
    }
    let first = value.chars().next()?;
    if first.is_ascii_uppercase() {
        // Uppercase-initial: either SCREAMING_SNAKE or a bare CamelCase name
        // (the latter only reaches here through the symbol path).
        if value
            .chars()
            .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_')
            && value.contains('_')
        {
            return Some(ManualClaimClass::ScreamingSnake);
        }
        return None;
    }
    if first.is_ascii_lowercase()
        && value.contains('_')
        && value
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
    {
        return Some(ManualClaimClass::SnakeCase);
    }
    None
}

/// Every string-literal claim the seeded corpus makes: env vars, EventLedger
/// event types, config/JSON keys, typed schema ids, and bare type names the
/// compiled vocabulary does not anchor.
///
/// These name string literals in product source rather than Rust items this
/// module can name, so they cannot be compile-anchored here. Hand them to
/// [`verify_manual_literal_claims`] together with the real product source text.
pub fn manual_literal_claims(pages: &[NewUserManualPage]) -> Vec<ManualLiteralClaim> {
    let mut symbol_vocabulary: BTreeSet<&'static str> = BTreeSet::new();
    for symbol in ManualNamedSymbol::ALL {
        symbol_vocabulary.insert(symbol.literal());
    }
    let mut symbol_segment_vocabulary: BTreeSet<&'static str> = BTreeSet::new();
    for literal in &symbol_vocabulary {
        for segment in literal.split("::") {
            if segment
                .chars()
                .next()
                .is_some_and(|c| c.is_ascii_uppercase())
            {
                symbol_segment_vocabulary.insert(segment);
            }
        }
    }

    let mut claims = Vec::new();
    for page in pages {
        for section in &page.sections {
            let mut bodies: Vec<String> = vec![section.body_md.clone()];
            if let Some(json) = &section.body_json {
                json_string_leaves(json, &mut bodies);
            }
            for body in &bodies {
                for span in backticked_spans(body) {
                    if manual_route_claim(span).is_some() {
                        continue;
                    }
                    // Same ordering as `verify_manual_named_surface_existence`:
                    // literal classes first, then the bare-symbol fallback.
                    let (class, literal) = if let Some(class) = literal_claim_class(span) {
                        (class, span.trim().to_string())
                    } else if let Some(candidate) = manual_symbol_candidate(span) {
                        if candidate.contains("::")
                            || symbol_vocabulary.contains(candidate)
                            || symbol_segment_vocabulary.contains(candidate)
                        {
                            continue;
                        }
                        (ManualClaimClass::BareTypeName, candidate.to_string())
                    } else {
                        continue;
                    };
                    claims.push(ManualLiteralClaim {
                        page_slug: page.slug.clone(),
                        section_kind: section.section_kind,
                        class,
                        literal,
                    });
                }
            }
        }
    }
    claims
}

/// Resolve [`manual_literal_claims`] against real product source text.
///
/// `product_source` is the concatenation of the product's own source files with
/// the manual corpus itself EXCLUDED — otherwise every claim would resolve
/// against the sentence that made it. The proof suite owns the file walk,
/// because the library must never read the source tree at runtime.
///
/// Returns the number of claims resolved, or one error per claim that names a
/// literal appearing nowhere in the product. This is deliberately weaker than
/// compile anchoring — a rename applied to both the manual and the code passes —
/// and that limitation is why the strongest classes (Rust paths, routes, Flight
/// Recorder ids) are proven by [`verify_manual_named_surface_existence`] instead.
pub fn verify_manual_literal_claims(
    claims: &[ManualLiteralClaim],
    product_source: &str,
) -> Result<usize, Vec<BehaviorCoverageError>> {
    let mut errors = Vec::new();
    for claim in claims {
        if !product_source.contains(&claim.literal) {
            errors.push(BehaviorCoverageError {
                behavior_id: MANUAL_NAMED_SURFACE_BEHAVIOR_ID,
                reason: format!(
                    "UserManual page `{}` section `{}` names {} `{}`, which appears nowhere in \
                     product source (HBR-MAN-003). The manual must never name a surface that does \
                     not exist: fix the spelling, or remove the claim.",
                    claim.page_slug,
                    claim.section_kind,
                    claim.class.as_str(),
                    claim.literal
                ),
            });
        }
    }
    if errors.is_empty() {
        Ok(claims.len())
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
            "model_runtime_registry_behaviors_have_canonical_manual_coverage",
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
            "model_runtime_registry_behaviors_have_canonical_manual_coverage",
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
/// compiled symbols/constants plus the seeded embedded SurrealDB feature row. This does
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

/// Compile-linked schema registry used by UserManual coverage proof while the
/// durable rows live in embedded SurrealDB. Runtime registration may enrich
/// `record_kind` and `table_name`; it may not add an unlisted schema silently.
pub fn canonical_model_lane_schema_registry() -> Vec<ModelLaneSchemaRegistryRow> {
    [
        "hsk.model_lane_run@1",
        "hsk.model_lane@1",
        "hsk.model_lane_message@1",
        "hsk.model_lane_terminal@1",
        "hsk.model_lane_promotion_decision@1",
        "hsk.model_lane_context_bundle_artifact@1",
        "hsk.model_lane_context_bundle_handoff@1",
        "hsk.model_lane_cloud_projection_plan@1",
        "hsk.model_lane_cloud_consent_receipt@1",
        "hsk.model_lane_cloud_consent_denial@1",
        "hsk.model_lane_recovery_checkpoint@1",
        "hsk.model_lane_recovery_event@1",
        "hsk.model_lane_lease@1",
        "hsk.model_lane_diagnostic_tier@1",
        "hsk.model_lane_mt_runtime_status@1",
        "hsk.model_lane_cloud_projection_plan@2",
        "hsk.model_lane_cloud_consent_receipt@2",
        "hsk.model_lane_recovery_event@2",
        "hsk.model_lane_routing_execution@5",
        "hsk.model_lane_routing_outbox@4",
        "hsk.model_lane_routing_stage_attempt@4",
        "hsk.model_lane_run_extension@1",
    ]
    .into_iter()
    .map(|schema_id| {
        let schema_version = schema_id
            .rsplit_once('@')
            .and_then(|(_, version)| version.parse().ok())
            .expect("canonical ModelLane schema id carries a numeric version");
        ModelLaneSchemaRegistryRow {
            schema_id: schema_id.to_owned(),
            schema_version,
            record_kind: "model_lane".to_owned(),
            table_name: "model_lane_authority".to_owned(),
        }
    })
    .collect()
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
            tool_id: "model_lane_launch_tests",
            eventledger_flight_recorder_path: "kernel_event_ledger:model_lane_run",
            internal_diagnostics_posture: DiagnosticTierPosture::RunLevelWired,
            palmistry_posture: DiagnosticTierPosture::RunLevelWired,
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
            internal_diagnostics_posture: DiagnosticTierPosture::RunLevelWired,
            palmistry_posture: DiagnosticTierPosture::RunLevelWired,
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
            internal_diagnostics_posture: DiagnosticTierPosture::RunLevelWired,
            palmistry_posture: DiagnosticTierPosture::RunLevelWired,
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
            internal_diagnostics_posture: DiagnosticTierPosture::RunLevelWired,
            palmistry_posture: DiagnosticTierPosture::RunLevelWired,
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
            tool_id: "model_lane_launch_tests",
            eventledger_flight_recorder_path: "kernel_event_ledger:model_lane_message",
            internal_diagnostics_posture: DiagnosticTierPosture::RunLevelWired,
            palmistry_posture: DiagnosticTierPosture::RunLevelWired,
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
            internal_diagnostics_posture: DiagnosticTierPosture::RunLevelWired,
            palmistry_posture: DiagnosticTierPosture::RunLevelWired,
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
            tool_id: "worktree_model_lane_live_surreal_tests",
            eventledger_flight_recorder_path: "kernel_event_ledger:model_lane_promotion_decision",
            internal_diagnostics_posture: DiagnosticTierPosture::RunLevelWired,
            palmistry_posture: DiagnosticTierPosture::RunLevelWired,
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
            tool_id: "worktree_model_lane_live_surreal_tests",
            eventledger_flight_recorder_path:
                "kernel_event_ledger:model_lane_context_bundle_artifact",
            internal_diagnostics_posture: DiagnosticTierPosture::RunLevelWired,
            palmistry_posture: DiagnosticTierPosture::RunLevelWired,
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
            tool_id: "worktree_model_lane_live_surreal_tests",
            eventledger_flight_recorder_path:
                "kernel_event_ledger:model_lane_context_bundle_handoff",
            internal_diagnostics_posture: DiagnosticTierPosture::RunLevelWired,
            palmistry_posture: DiagnosticTierPosture::RunLevelWired,
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
            tool_id: "cloud_model_lane_policy_surreal_tests",
            eventledger_flight_recorder_path:
                "model_lane_cloud_event_ledger:model_lane_cloud_projection_plan",
            internal_diagnostics_posture: DiagnosticTierPosture::RunLevelWired,
            palmistry_posture: DiagnosticTierPosture::RunLevelWired,
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
            tool_id: "cloud_model_lane_policy_surreal_tests",
            eventledger_flight_recorder_path:
                "model_lane_cloud_event_ledger:model_lane_cloud_consent_receipt",
            internal_diagnostics_posture: DiagnosticTierPosture::RunLevelWired,
            palmistry_posture: DiagnosticTierPosture::RunLevelWired,
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
            tool_id: "cloud_model_lane_policy_surreal_tests",
            eventledger_flight_recorder_path:
                "model_lane_cloud_event_ledger:model_lane_cloud_consent_denial",
            internal_diagnostics_posture: DiagnosticTierPosture::RunLevelWired,
            palmistry_posture: DiagnosticTierPosture::RunLevelWired,
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
            tool_id: "worktree_model_lane_live_surreal_tests",
            eventledger_flight_recorder_path: "kernel_event_ledger:model_lane_recovery_checkpoint",
            internal_diagnostics_posture: DiagnosticTierPosture::RunLevelWired,
            palmistry_posture: DiagnosticTierPosture::RunLevelWired,
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
            tool_id: "worktree_model_lane_live_surreal_tests",
            eventledger_flight_recorder_path: "kernel_event_ledger:model_lane_recovery_event",
            internal_diagnostics_posture: DiagnosticTierPosture::RunLevelWired,
            palmistry_posture: DiagnosticTierPosture::RunLevelWired,
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
            tool_id: "worktree_model_lane_live_surreal_tests",
            eventledger_flight_recorder_path: "kernel_event_ledger:model_lane_lease",
            internal_diagnostics_posture: DiagnosticTierPosture::RunLevelWired,
            palmistry_posture: DiagnosticTierPosture::RunLevelWired,
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
            internal_diagnostics_posture: DiagnosticTierPosture::RunLevelWired,
            palmistry_posture: DiagnosticTierPosture::RunLevelWired,
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
            tool_id: "worktree_model_lane_live_surreal_tests",
            eventledger_flight_recorder_path: "kernel_event_ledger:model_lane_mt_runtime_status",
            internal_diagnostics_posture: DiagnosticTierPosture::RunLevelWired,
            palmistry_posture: DiagnosticTierPosture::RunLevelWired,
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
            tool_id: "cloud_model_lane_policy_surreal_tests",
            eventledger_flight_recorder_path:
                "model_lane_cloud_event_ledger:model_lane_cloud_projection_plan",
            internal_diagnostics_posture: DiagnosticTierPosture::RunLevelWired,
            palmistry_posture: DiagnosticTierPosture::RunLevelWired,
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
            tool_id: "cloud_model_lane_policy_surreal_tests",
            eventledger_flight_recorder_path:
                "model_lane_cloud_event_ledger:model_lane_cloud_consent_receipt",
            internal_diagnostics_posture: DiagnosticTierPosture::RunLevelWired,
            palmistry_posture: DiagnosticTierPosture::RunLevelWired,
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
            tool_id: "worktree_model_lane_live_surreal_tests",
            eventledger_flight_recorder_path: "kernel_event_ledger:model_lane_recovery_event",
            internal_diagnostics_posture: DiagnosticTierPosture::RunLevelWired,
            palmistry_posture: DiagnosticTierPosture::RunLevelWired,
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
            tool_id: "worktree_model_lane_live_surreal_tests",
            eventledger_flight_recorder_path: "kernel_event_ledger:model_lane_routing_execution",
            internal_diagnostics_posture: DiagnosticTierPosture::RunLevelWired,
            palmistry_posture: DiagnosticTierPosture::RunLevelWired,
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
            tool_id: "worktree_model_lane_live_surreal_tests",
            eventledger_flight_recorder_path: "kernel_event_ledger:model_lane_routing_outbox",
            internal_diagnostics_posture: DiagnosticTierPosture::RunLevelWired,
            palmistry_posture: DiagnosticTierPosture::RunLevelWired,
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
            tool_id: "worktree_model_lane_live_surreal_tests",
            eventledger_flight_recorder_path:
                "kernel_event_ledger:model_lane_routing_stage_attempt",
            internal_diagnostics_posture: DiagnosticTierPosture::RunLevelWired,
            palmistry_posture: DiagnosticTierPosture::RunLevelWired,
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
            tool_id: "worktree_model_lane_live_surreal_tests",
            eventledger_flight_recorder_path: "kernel_event_ledger:model_lane_run_extension",
            internal_diagnostics_posture: DiagnosticTierPosture::RunLevelWired,
            palmistry_posture: DiagnosticTierPosture::RunLevelWired,
            deferred_reason: Some(
                "Tier-2 internal_diagnostics and the authenticated Tier-3 Palmistry watcher are wired observers; they do not replace the row's EventLedger/Flight Recorder authority.",
            ),
            follow_up_ref: None,
        },
    ];

    for row in &mut templates {
        // MT-011 run-level HBR-INT-009 encoding: the postures above are
        // `RunLevelWired`, meaning Tier-2 internal_diagnostics and Tier-3
        // Palmistry coverage for these rows is carried by the SINGLE correlated
        // HBR-INT-009 envelope emitted once per ModelLaneRun, not by a fabricated
        // per-behavior observation. Its liveness is proven by
        // `verify_model_lane_behavior_evidence` against a real run. Each row still
        // names those wired observers in its reason and carries a family-scoped
        // `palmistry://wp1/model-lane/...` correlation ref alongside its
        // EventLedger/Flight Recorder authority.
        row.deferred_reason = Some(
            "Tier-2 internal_diagnostics is produced by the native panic/heartbeat/frame/resource/open-event ring and Problems projection. Tier-3 Palmistry is the authenticated separate watcher with durable survivor recovery. Both are wired observers correlated at the run level: they emit one HBR-INT-009 envelope per ModelLaneRun (validated by verify_model_lane_behavior_evidence), not a per-behavior observation, and the lane pane consumes that run-level observation evidence without replacing either producer's EventLedger/Flight Recorder authority.",
        );
        row.follow_up_ref = Some(match row.behavior_id {
            "wp1.model_lane.run" => "palmistry://wp1/model-lane/run",
            "wp1.model_lane.launch" => "palmistry://wp1/model-lane/launch",
            "wp1.model_lane.official_cli_spawn" => "palmistry://wp1/model-lane/official-cli-spawn",
            "wp1.model_lane.official_cli_attached_sandbox" => {
                "palmistry://wp1/model-lane/official-cli-attached-sandbox"
            }
            "wp1.model_lane.message" => "palmistry://wp1/model-lane/message",
            "wp1.model_lane.terminal" => "palmistry://wp1/model-lane/terminal",
            "wp1.model_lane.promotion" => "palmistry://wp1/model-lane/promotion",
            "wp1.model_lane.context_bundle_artifact" => {
                "palmistry://wp1/model-lane/context-bundle-artifact"
            }
            "wp1.model_lane.context_bundle" => "palmistry://wp1/model-lane/context-bundle-handoff",
            "wp1.model_lane.cloud_projection_plan" => {
                "palmistry://wp1/model-lane/cloud-projection-plan"
            }
            "wp1.model_lane.cloud_consent" => "palmistry://wp1/model-lane/cloud-consent-receipt",
            "wp1.model_lane.cloud_consent_denial" => {
                "palmistry://wp1/model-lane/cloud-consent-denial"
            }
            "wp1.model_lane.recovery" => "palmistry://wp1/model-lane/recovery-checkpoint",
            "wp1.model_lane.recovery_event" => "palmistry://wp1/model-lane/recovery-event",
            "wp1.model_lane.lease" => "palmistry://wp1/model-lane/lease",
            "wp1.model_lane.diagnostics" => "palmistry://wp1/model-lane/diagnostic-tier",
            "wp1.model_lane.mixed_validation" => "palmistry://wp1/model-lane/mixed-validation",
            "wp1.model_lane.cloud_projection_plan_v2" => {
                "palmistry://wp1/model-lane/cloud-projection-plan-v2"
            }
            "wp1.model_lane.cloud_consent_v2" => {
                "palmistry://wp1/model-lane/cloud-consent-receipt-v2"
            }
            "wp1.model_lane.recovery_event_v2" => "palmistry://wp1/model-lane/recovery-event-v2",
            "wp1.model_lane.routing_execution" => "palmistry://wp1/model-lane/routing-execution",
            "wp1.model_lane.routing_outbox" => "palmistry://wp1/model-lane/routing-outbox",
            "wp1.model_lane.routing_stage_attempt" => {
                "palmistry://wp1/model-lane/routing-stage-attempt"
            }
            "wp1.model_lane.run_extension" => "palmistry://wp1/model-lane/run-extension",
            _ => "palmistry://wp1/model-lane/lane",
        });
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
                "settings_argus:official_cli_bridge_in_app_pty_login + provider_owned_login_command + process_ledger_start_stop",
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
        follow_up_ref: Some("palmistry://wp1/dedicated-embedding-model/selected-model-receipts"),
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
    // MT-011 wired native internal_diagnostics + the authenticated Palmistry
    // watcher as WIRED observers of these non-schema rows too. Every genuinely
    // WIRED row that does not already carry a correlation ref gets a family-scoped
    // `palmistry://wp1/<family-surface>/...` ref; NOT_APPLICABLE rows (cloud
    // Settings/keychain surfaces) and rows that already declare a ref are left
    // unchanged.
    for descriptor in &mut rows {
        if descriptor.row.follow_up_ref.is_some()
            || descriptor.row.internal_diagnostics_posture != DiagnosticTierPosture::Wired
        {
            continue;
        }
        let follow_up_ref = match descriptor.row.behavior_id {
            "wp1.embedded_model.ledger_start" => "palmistry://wp1/embedded-model/ledger-start",
            "wp1.embedded_model.ledger_stop" => "palmistry://wp1/embedded-model/ledger-stop",
            "wp1.embedded_model.os_lease_reclaim" => {
                "palmistry://wp1/embedded-model/os-lease-reclaim"
            }
            "wp1.llm.fail_closed_fr" => "palmistry://wp1/embedded-model/fail-closed-fr",
            "wp1.llm.embedding_fr" => "palmistry://wp1/embedded-model/embedding-fr",
            "wp1.operator_chat.capture_message" => "palmistry://wp1/operator-chat/capture-message",
            "wp1.operator_chat.agent_activity_fr" => {
                "palmistry://wp1/operator-chat/agent-activity-fr"
            }
            _ => match descriptor.family {
                CanonicalBehaviorFamily::EmbeddedModel => "palmistry://wp1/embedded-model/observer",
                CanonicalBehaviorFamily::OperatorChat => "palmistry://wp1/operator-chat/observer",
                CanonicalBehaviorFamily::DedicatedEmbedding => {
                    "palmistry://wp1/dedicated-embedding-model/observer"
                }
                CanonicalBehaviorFamily::CloudModelAccess => {
                    "palmistry://wp1/cloud-model-access/observer"
                }
            },
        };
        descriptor.row.follow_up_ref = Some(follow_up_ref);
    }
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
        // Run-level HBR-INT-009 encoding: model-lane rows declare RUN_LEVEL_WIRED
        // (covered by the single per-run envelope). A per-behavior `Wired` literal
        // here is rejected as a vacuous static flip; the live proof is
        // `verify_model_lane_behavior_evidence` against a real run's durable records.
        if row.internal_diagnostics_posture != DiagnosticTierPosture::RunLevelWired
            || row.palmistry_posture != DiagnosticTierPosture::RunLevelWired
        {
            errors.push(BehaviorCoverageError {
                behavior_id: row.behavior_id,
                reason: format!(
                    "internal_diagnostics and Palmistry postures for model-lane behaviors must be RUN_LEVEL_WIRED (proven live by verify_model_lane_behavior_evidence), got {}/{}",
                    row.internal_diagnostics_posture.as_str(),
                    row.palmistry_posture.as_str()
                ),
            });
        }
        if (row.internal_diagnostics_posture == DiagnosticTierPosture::RunLevelWired
            || row.palmistry_posture == DiagnosticTierPosture::RunLevelWired)
            && (row.deferred_reason.is_none() || row.follow_up_ref.is_none())
        {
            errors.push(BehaviorCoverageError {
                behavior_id: row.behavior_id,
                reason: "RUN_LEVEL_WIRED diagnostics tiers require an explanatory reason and a run-level HBR-INT-009 follow_up_ref"
                    .to_owned(),
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

/// Run-level evidence gate for the HBR-INT-009 posture declared by the
/// statically-covered ModelLane behaviors. This is the non-tautological proof
/// that backs the `RunLevelWired` declaration: the runtime producers write ONE
/// correlated three-tier HBR-INT-009 envelope per run (they do not fabricate a
/// per-behavior tier triplet), so each row declares `RunLevelWired` and this
/// gate holds only when the exact run's durable run-level HBR-INT-009 records
/// exist and correlate through `ModelLaneStore::validate_diagnostic_tier_posture`
/// with `eventledger://kernel/`, `internal-diagnostics://session/`, and
/// `palmistry-observation://session/` evidence refs. When those records are
/// absent or incomplete the gate fails closed. This function is the MT-011
/// run-level evidence proof target (see
/// `user_manual_behavior_coverage_tests::model_lane_run_level_hbr_int_009_*`).
pub async fn verify_model_lane_behavior_evidence(
    store: &ModelLaneStore,
    run_id: &str,
    rows: &[BehaviorCoverageRow],
) -> Result<Vec<ModelLaneDiagnosticTierPosture>, Vec<BehaviorCoverageError>> {
    let mut errors = Vec::new();
    for row in rows {
        if row.internal_diagnostics_posture != DiagnosticTierPosture::RunLevelWired
            || row.palmistry_posture != DiagnosticTierPosture::RunLevelWired
        {
            errors.push(BehaviorCoverageError {
                behavior_id: row.behavior_id,
                reason: "behavior does not declare the run-level HBR-INT-009 internal_diagnostics and Palmistry tiers as RUN_LEVEL_WIRED".to_owned(),
            });
        }
    }
    if !errors.is_empty() {
        return Err(errors);
    }

    const BEHAVIOR_ID: &str = "HBR-INT-009";
    // A read this reader is not scoped for returns NO tier rows (the store's
    // owner predicate keeps them inside embedded SurrealDB), so a cross-account call
    // lands here as "no valid evidence" rather than disclosing another
    // account's diagnostic records. The message names only the caller-supplied
    // run id (HBR-PRIV-004: a denial must not be an existence oracle).
    let posture = match store
        .validate_diagnostic_tier_posture(run_id, BEHAVIOR_ID)
        .await
    {
        Ok(posture) => posture,
        Err(error) => {
            return Err(vec![BehaviorCoverageError {
                behavior_id: BEHAVIOR_ID,
                reason: format!(
                    "run {run_id} has no valid three-tier diagnostic evidence: {error}"
                ),
            }])
        }
    };

    let mut errors = Vec::new();
    if posture.run_id != run_id || posture.behavior_id != BEHAVIOR_ID {
        errors.push(BehaviorCoverageError {
            behavior_id: BEHAVIOR_ID,
            reason: format!("diagnostic tier posture does not correlate to run {run_id}"),
        });
    }

    // Exactly one record per canonical tier. `validate_diagnostic_tier_posture`
    // already refuses a missing tier; this additionally refuses a duplicate or
    // an unexpected tier, so the envelope shape is pinned rather than assumed.
    for (tier, _, _) in DIAGNOSTIC_TIER_EVIDENCE_URI_BINDING {
        let recorded = posture
            .tiers
            .iter()
            .filter(|record| record.tier == tier)
            .count();
        if recorded != 1 {
            errors.push(BehaviorCoverageError {
                behavior_id: BEHAVIOR_ID,
                reason: format!(
                    "run-level HBR-INT-009 envelope must carry exactly one `{}` tier record, found {recorded}",
                    tier.as_str()
                ),
            });
        }
    }

    for record in &posture.tiers {
        if record.event_ledger_event_id.trim().is_empty() {
            errors.push(BehaviorCoverageError {
                behavior_id: BEHAVIOR_ID,
                reason: format!(
                    "HBR-INT-009 tier `{}` has no durable EventLedger event id",
                    record.tier.as_str()
                ),
            });
        }
        // RUN_LEVEL_WIRED is a liveness claim. A tier that is merely deferred or
        // not-applicable cannot carry it, so the run-level gate demands WIRED.
        if record.state != ModelLaneDiagnosticTierState::Wired {
            errors.push(BehaviorCoverageError {
                behavior_id: BEHAVIOR_ID,
                reason: format!(
                    "RUN_LEVEL_WIRED coverage requires a live WIRED `{}` tier record, got `{}`",
                    record.tier.as_str(),
                    record.state.as_str()
                ),
            });
        }
        // The load-bearing MT-011 remediation: each tier's evidence must use its
        // OWN URI scheme. Offering Tier-3 Palmistry evidence as Tier-2
        // internal_diagnostics evidence (or the reverse) is rejected here.
        if let Err(violation) =
            verify_diagnostic_tier_evidence_uri(record.tier, &record.evidence_ref)
        {
            errors.push(BehaviorCoverageError {
                behavior_id: BEHAVIOR_ID,
                reason: violation.to_string(),
            });
        }
    }

    if errors.is_empty() {
        Ok(vec![posture])
    } else {
        Err(errors)
    }
}

// ---------------------------------------------------------------------------
// MT-011 acceptance: the machine-readable per-behavior UserManual coverage
// matrix, emitted under the Rust-owned schema id
// `hsk.user_manual_behavior_coverage@1`.
//
// This is a PROJECTION of the already-generated behavior rows
// ([`model_lane_behavior_coverage_matrix`] and the canonical non-schema
// registries), not a second source of truth: it joins each row to its computed
// self-consistency result so the matrix is complete on its own terms and can be
// serialized for a no-context model, an operator surface, or a validator.
//
// [`verify_user_manual_behavior_coverage_matrix`] is the acceptance gate: it
// FAILS when any behavior lacks a UserManual entry, an EventLedger/Flight
// Recorder evidence path, a per-tier internal_diagnostics/Palmistry posture, a
// runtime route/schema, or a self-consistency result. Keyword grep is never the
// proof target.
// ---------------------------------------------------------------------------

/// Rust-owned schema id for the MT-011 coverage matrix. Already named by the
/// seeded UserManual corpus and by the storage proof; this is the compiled
/// definition those references point at.
pub const USER_MANUAL_BEHAVIOR_COVERAGE_SCHEMA_ID: &str = "hsk.user_manual_behavior_coverage@1";

fn serialize_posture<S>(posture: &DiagnosticTierPosture, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    serializer.serialize_str(posture.as_str())
}

/// The self-consistency column. Either the authorities that were actually
/// checked for this behavior, or the reasons it is inconsistent; never absent.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(tag = "result", rename_all = "snake_case")]
pub enum BehaviorSelfConsistencyResult {
    Consistent {
        checked_authorities: Vec<&'static str>,
    },
    Inconsistent {
        errors: Vec<String>,
    },
}

impl BehaviorSelfConsistencyResult {
    pub const fn is_consistent(&self) -> bool {
        matches!(self, Self::Consistent { .. })
    }
}

/// One machine-readable coverage row, keyed by `behavior_id` and by
/// `schema_id`-or-`event_family`.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct UserManualBehaviorCoverageMatrixRow {
    pub behavior_id: &'static str,
    pub schema_id: Option<&'static str>,
    pub event_family: &'static str,
    /// The stable matrix key: the typed schema id when the behavior has one,
    /// otherwise its event family.
    pub schema_or_event_family: &'static str,
    /// Implemented command / API route / IPC surface for this behavior.
    pub runtime_surface_id: &'static str,
    pub user_manual_slug: &'static str,
    pub user_manual_version: &'static str,
    pub proof_tool_id: &'static str,
    pub eventledger_flight_recorder_evidence_path: &'static str,
    #[serde(serialize_with = "serialize_posture")]
    pub internal_diagnostics_posture: DiagnosticTierPosture,
    #[serde(serialize_with = "serialize_posture")]
    pub palmistry_posture: DiagnosticTierPosture,
    /// Required whenever a posture is anything other than a per-behavior
    /// `WIRED` observation.
    pub diagnostic_reason: Option<&'static str>,
    pub follow_up_ref: Option<&'static str>,
    pub self_consistency_result: BehaviorSelfConsistencyResult,
}

/// The full matrix plus the schema id and manual version it was generated for.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct UserManualBehaviorCoverageMatrix {
    pub schema_id: &'static str,
    pub user_manual_version: &'static str,
    /// Exact tier to URI prefix binding this matrix was validated under, so a
    /// serialized matrix carries the binding rather than implying it.
    pub diagnostic_tier_evidence_uri_binding: Vec<(&'static str, &'static str)>,
    pub rows: Vec<UserManualBehaviorCoverageMatrixRow>,
}

/// Project behavior coverage rows into the machine-readable MT-011 matrix,
/// computing each row's self-consistency result against the real ModelLane
/// schema registry and the real UserManual page/tool rows.
pub fn user_manual_behavior_coverage_matrix(
    rows: &[BehaviorCoverageRow],
    schema_registry: &[ModelLaneSchemaRegistryRow],
    pages: &[UserManualPage],
    tools: &[UserManualToolEntry],
) -> UserManualBehaviorCoverageMatrix {
    let rows = rows
        .iter()
        .map(|row| {
            let self_consistency_result =
                match row.self_consistency_result(schema_registry, pages, tools) {
                    Ok(proof) => BehaviorSelfConsistencyResult::Consistent {
                        checked_authorities: proof.checked_authorities.into_iter().collect(),
                    },
                    Err(errors) => BehaviorSelfConsistencyResult::Inconsistent {
                        errors: errors.into_iter().map(|error| error.reason).collect(),
                    },
                };
            UserManualBehaviorCoverageMatrixRow {
                behavior_id: row.behavior_id,
                schema_id: row.schema_id,
                event_family: row.event_family,
                schema_or_event_family: row.schema_or_event_family(),
                runtime_surface_id: row.runtime_surface_id,
                user_manual_slug: row.user_manual_slug,
                user_manual_version: USER_MANUAL_VERSION,
                proof_tool_id: row.tool_id,
                eventledger_flight_recorder_evidence_path: row.eventledger_flight_recorder_path,
                internal_diagnostics_posture: row.internal_diagnostics_posture,
                palmistry_posture: row.palmistry_posture,
                diagnostic_reason: row.deferred_reason,
                follow_up_ref: row.follow_up_ref,
                self_consistency_result,
            }
        })
        .collect();

    UserManualBehaviorCoverageMatrix {
        schema_id: USER_MANUAL_BEHAVIOR_COVERAGE_SCHEMA_ID,
        user_manual_version: USER_MANUAL_VERSION,
        diagnostic_tier_evidence_uri_binding: compiled_diagnostic_tier_uri_binding(),
        rows,
    }
}

fn compiled_diagnostic_tier_uri_binding() -> Vec<(&'static str, &'static str)> {
    DIAGNOSTIC_TIER_EVIDENCE_URI_BINDING
        .iter()
        .map(|(tier, _, prefix)| (tier.as_str(), *prefix))
        .collect()
}

/// MT-011 acceptance gate over the machine-readable matrix.
///
/// Returns the number of behaviors proven, or one error per missing column.
/// Fails when a behavior lacks any of: a UserManual entry, an EventLedger/Flight
/// Recorder evidence path, a per-tier diagnostic posture with its required
/// reason, a runtime route/schema, or a self-consistency result.
pub fn verify_user_manual_behavior_coverage_matrix(
    matrix: &UserManualBehaviorCoverageMatrix,
) -> Result<usize, Vec<BehaviorCoverageError>> {
    const MATRIX_ID: &str = "user_manual.behavior_coverage_matrix";
    let mut errors = Vec::new();

    if matrix.schema_id != USER_MANUAL_BEHAVIOR_COVERAGE_SCHEMA_ID {
        errors.push(BehaviorCoverageError {
            behavior_id: MATRIX_ID,
            reason: format!(
                "coverage matrix must be emitted as {USER_MANUAL_BEHAVIOR_COVERAGE_SCHEMA_ID}"
            ),
        });
    }
    if matrix.user_manual_version != USER_MANUAL_VERSION {
        errors.push(BehaviorCoverageError {
            behavior_id: MATRIX_ID,
            reason: format!(
                "coverage matrix targets manual version {} but the compiled corpus is {USER_MANUAL_VERSION}",
                matrix.user_manual_version
            ),
        });
    }
    if matrix.rows.is_empty() {
        // A matrix with no rows would let every per-row check below pass
        // vacuously, which is exactly the false green MT-011 forbids.
        errors.push(BehaviorCoverageError {
            behavior_id: MATRIX_ID,
            reason: "coverage matrix is empty; a matrix that covers nothing is not proof"
                .to_owned(),
        });
    }

    let mut seen = BTreeSet::new();
    for row in &matrix.rows {
        if !seen.insert(row.behavior_id) {
            errors.push(BehaviorCoverageError {
                behavior_id: row.behavior_id,
                reason: "duplicate behavior_id in the coverage matrix".to_owned(),
            });
        }
        if row.behavior_id.trim().is_empty() || row.schema_or_event_family.trim().is_empty() {
            errors.push(BehaviorCoverageError {
                behavior_id: row.behavior_id,
                reason: "matrix key (behavior_id plus schema_id/event_family) is incomplete"
                    .to_owned(),
            });
        }
        if row.schema_or_event_family != row.schema_id.unwrap_or(row.event_family) {
            errors.push(BehaviorCoverageError {
                behavior_id: row.behavior_id,
                reason: "schema_or_event_family key does not derive from schema_id/event_family"
                    .to_owned(),
            });
        }
        if row.runtime_surface_id.trim().is_empty() {
            errors.push(BehaviorCoverageError {
                behavior_id: row.behavior_id,
                reason: "matrix row lacks an implemented command/API/IPC runtime surface"
                    .to_owned(),
            });
        }
        if row.user_manual_slug.trim().is_empty() {
            errors.push(BehaviorCoverageError {
                behavior_id: row.behavior_id,
                reason: "matrix row lacks a UserManual entry".to_owned(),
            });
        }
        if row.proof_tool_id.trim().is_empty() {
            errors.push(BehaviorCoverageError {
                behavior_id: row.behavior_id,
                reason: "matrix row lacks a UserManual proof target".to_owned(),
            });
        }
        if row
            .eventledger_flight_recorder_evidence_path
            .trim()
            .is_empty()
        {
            errors.push(BehaviorCoverageError {
                behavior_id: row.behavior_id,
                reason: "matrix row lacks an EventLedger/Flight Recorder evidence path".to_owned(),
            });
        }
        for (tier_label, posture) in [
            ("internal_diagnostics", row.internal_diagnostics_posture),
            ("palmistry", row.palmistry_posture),
        ] {
            if !matches!(posture, DiagnosticTierPosture::Wired) && row.diagnostic_reason.is_none() {
                errors.push(BehaviorCoverageError {
                    behavior_id: row.behavior_id,
                    reason: format!(
                        "{tier_label} posture {} requires an explicit reason",
                        posture.as_str()
                    ),
                });
            }
            let follow_up_required = matches!(
                posture,
                DiagnosticTierPosture::RunLevelWired | DiagnosticTierPosture::DeferredWithReason
            );
            if follow_up_required && row.follow_up_ref.is_none() {
                errors.push(BehaviorCoverageError {
                    behavior_id: row.behavior_id,
                    reason: format!(
                        "{tier_label} posture {} requires a follow_up_ref",
                        posture.as_str()
                    ),
                });
            }
        }
        if let BehaviorSelfConsistencyResult::Inconsistent { errors: reasons } =
            &row.self_consistency_result
        {
            if reasons.is_empty() {
                errors.push(BehaviorCoverageError {
                    behavior_id: row.behavior_id,
                    reason: "self-consistency result is inconsistent with no stated reason"
                        .to_owned(),
                });
            }
            for reason in reasons {
                errors.push(BehaviorCoverageError {
                    behavior_id: row.behavior_id,
                    reason: format!("self-consistency failed: {reason}"),
                });
            }
        }
    }

    // The serialized binding must match the compiled one, so a matrix handed to
    // a validator cannot advertise a looser tier-to-scheme contract than the one
    // `verify_diagnostic_tier_evidence_uri` actually enforces.
    if matrix.diagnostic_tier_evidence_uri_binding != compiled_diagnostic_tier_uri_binding() {
        errors.push(BehaviorCoverageError {
            behavior_id: MATRIX_ID,
            reason:
                "declared diagnostic tier to evidence URI binding does not match the compiled binding"
                    .to_owned(),
        });
    }

    if errors.is_empty() {
        Ok(matrix.rows.len())
    } else {
        Err(errors)
    }
}
