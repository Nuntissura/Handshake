//! ComfyUI custom-node intake -- governed records + receipts (MT-202).
//!
//! Spec authority: master-spec-v02.189 / spec-modules/06-mechanical-integrations.md
//! Section 6.9 "ComfyUI Custom-Node Intake (Normative)" (6.9.1 governed-job
//! containment LAW; 6.9.2 bridge-node presence detection; 6.9.3 capability
//! registration; 6.9.4 output routing into ArtifactStore; 6.9.5 EventLedger +
//! Flight-Recorder evidence; 6.9.6 SaveImage fallback boundary).
//!
//! legacy source (INTENT ONLY, never copied): `comfyui_node/legacy_bridge.py`
//! (the `LegacyBridgeNode` node: IMAGE outputs, `workflow_json`/`prompt`,
//! `seed`/`model`/`sampler`/`cfg`/`steps`, `filename_hint`, `character_id`,
//! and a bearer-token POST). The legacy source node executes generation, saves to a
//! ComfyUI output directory, and POSTs to a `localhost`/HTTP intake endpoint
//! with the raw image bytes and a bearer token. Handshake DELIBERATELY does NOT
//! reproduce any of that execution surface here: there is no socket, no process
//! spawn, no localhost endpoint, no polling, and no bytes-in-DB. This module is
//! the GOVERNED DATA + RECEIPT model only -- the durable records a
//! Workflow-Engine `engine.comfyui` AI Job writes THROUGH after it has already
//! probed/registered/materialized through the shared ArtifactStore. Actual tool
//! execution lives in the capability-gated Workflow-Engine job, OUT OF THIS
//! MODULE (LAW-COMFY-INTAKE-001).
//!
//! Translated contract (the load-bearing invariants from Section 6.9):
//!   * Containment (6.9.1): every record pins a `workflow_run_id`; this module
//!     stores/queries records, it never executes ComfyUI.
//!   * Probe (6.9.2): one `ComfyBridgeNodeProbeV1` per job start, idempotent on
//!     `workflow_run_id` (one probe per run); records `probe_outcome` and the
//!     `fallback_reason` required when the outcome is not `bridge_present`.
//!   * Registration (6.9.3): exactly one `ComfyBridgeCapabilityRecordV1` per
//!     job (idempotent on `workflow_run_id`); declared bridge outputs are stored
//!     as ordered child rows, and capability-rejected outputs are recorded as
//!     typed reject rows that are never routed.
//!   * Output routing (6.9.4): one `ComfyIntakeOutputRecordV1` per routed
//!     output, idempotent on `(workflow_run_id, content_hash)` so a retry
//!     re-delivery resolves to the existing `artifact_ref` (dedup evidence,
//!     6.9.5). Carries the workflow-receipt lineage: prompt-json ref, output
//!     image artifact_ref, graph hash, and seed (Section 9.12 STOCHASTIC pins).
//!   * SaveImage fallback (6.9.6): the same output record table, with
//!     `source_output_slot = 'saveimage_fallback'` and a null `registration_id`,
//!     marked by an explicit fallback-engaged record so the operator sees why the
//!     bridge path was not taken.
//!   * Receipt (6.9.5): a `ComfyIntakeReceiptV1` summary per run, recoverable so
//!     a no-context model can reconstruct exactly what intake produced.
//!   * Scrubbing (6.9.5 / LAW-COMFY-INTAKE-005): secrets, cookies, API tokens,
//!     and auth headers are never persisted. Records reference artifacts and
//!     graph node ids only. Free-form provenance is scrubbed on the way in
//!     (settings.rs redaction style) and never embeds credential material or
//!     machine-local absolute paths.
//!
//! Storage authority is PostgreSQL only (LAW-COMFY-INTAKE-004); SQLite is
//! forbidden. Microtasks: MT-202 (ComfyUI intake records), MT-005 (event
//! coverage).

use crate::capabilities::CapabilityRegistry;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{QueryBuilder, Row};
use std::collections::BTreeMap;
use uuid::Uuid;

use super::intake::IntakeLane;
use super::{reject_legacy_runtime_ref, AtelierError, AtelierResult, AtelierStore};

/// ComfyUI intake event families (MT-202, extends the MT-005 coverage set).
///
/// Mirrors the Section 6.9.5 evidence shape. Defined here so the parent can
/// fold these into [`super::event_family::ALL`] and the MT-005 coverage check
/// picks up every intake mutation.
pub mod comfy_event_family {
    /// A bridge-node presence probe was recorded (6.9.2; carries
    /// `probe_outcome`, `fallback_reason`).
    pub const PROBE_RECORDED: &str = "atelier.comfy.intake.probe.recorded";
    /// The bridge capability was registered for a job (6.9.3; carries declared
    /// output count + reject count).
    pub const CAPABILITY_REGISTERED: &str = "atelier.comfy.intake.capability.registered";
    /// A declared output was dropped by the capability gate (6.9.3 / 6.9.5;
    /// carries `output_slot`, reason). Never routed.
    pub const CAPABILITY_REJECTED: &str = "atelier.comfy.intake.capability.rejected";
    /// A routed output was materialized into the ArtifactStore (6.9.4; carries
    /// `artifact_ref`, `artifact_manifest_ref`, `routing_intent`).
    pub const OUTPUT_MATERIALIZED: &str = "atelier.comfy.intake.output.materialized";
    /// An idempotent re-delivery resolved to an existing artifact (6.9.4 /
    /// 6.9.5; carries the existing `artifact_ref`).
    pub const OUTPUT_DEDUPLICATED: &str = "atelier.comfy.intake.output.deduplicated";
    /// Routing degraded to the SaveImage scan fallback (6.9.6; carries
    /// `fallback_reason`).
    pub const FALLBACK_ENGAGED: &str = "atelier.comfy.intake.fallback.engaged";
    /// A per-job intake receipt was produced at job completion (6.9.5).
    pub const RECEIPT_PRODUCED: &str = "atelier.comfy.intake.receipt.produced";
    /// A durable workflow-level receipt was recorded for a ComfyUI run.
    pub const WORKFLOW_RECEIPT_RECORDED: &str = "atelier.comfy.workflow.receipt.recorded";
    /// A generated output was preserved because registration failed after save.
    pub const OUTPUT_REGISTRATION_FAILURE_RECORDED: &str =
        "atelier.comfy.output.registration_failure.recorded";
    /// A preserved generated output was retried into a normal intake output.
    pub const OUTPUT_REGISTRATION_FAILURE_RETRIED: &str =
        "atelier.comfy.output.registration_failure.retried";
    /// A replayable-workflow-inputs resolution was requested (MT-104/MT-130).
    pub const REPLAY_REQUESTED: &str = "atelier.replay.requested";
    /// All replay input artifact refs resolved to stored artifacts (MT-130).
    pub const REPLAY_COMPLETED: &str = "atelier.replay.completed";
    /// A replay request was rejected (unresolved/invalid/legacy ref) (MT-130).
    pub const REPLAY_FAILED: &str = "atelier.replay.failed";
    /// A versioned workflow spec was registered/upserted (MT-106).
    pub const WORKFLOW_SPEC_REGISTERED: &str = "atelier.comfy.workflow.spec.registered";
    /// Version metadata (pose/image-tool/comfy versions) was pinned for a run
    /// (MT-110).
    pub const VERSION_METADATA_RECORDED: &str = "atelier.comfy.version_metadata.recorded";

    /// All ComfyUI intake event families (parity/coverage helper).
    pub const ALL: &[&str] = &[
        PROBE_RECORDED,
        CAPABILITY_REGISTERED,
        CAPABILITY_REJECTED,
        OUTPUT_MATERIALIZED,
        OUTPUT_DEDUPLICATED,
        FALLBACK_ENGAGED,
        RECEIPT_PRODUCED,
        WORKFLOW_RECEIPT_RECORDED,
        OUTPUT_REGISTRATION_FAILURE_RECORDED,
        OUTPUT_REGISTRATION_FAILURE_RETRIED,
        REPLAY_REQUESTED,
        REPLAY_COMPLETED,
        REPLAY_FAILED,
        WORKFLOW_SPEC_REGISTERED,
        VERSION_METADATA_RECORDED,
    ];
}

/// Re-export at module root so callers can write `comfy::PROBE_RECORDED`.
pub use comfy_event_family::{
    CAPABILITY_REGISTERED, CAPABILITY_REJECTED, FALLBACK_ENGAGED, OUTPUT_DEDUPLICATED,
    OUTPUT_MATERIALIZED, OUTPUT_REGISTRATION_FAILURE_RECORDED, OUTPUT_REGISTRATION_FAILURE_RETRIED,
    PROBE_RECORDED, RECEIPT_PRODUCED, REPLAY_COMPLETED, REPLAY_FAILED, REPLAY_REQUESTED,
    VERSION_METADATA_RECORDED, WORKFLOW_RECEIPT_RECORDED, WORKFLOW_SPEC_REGISTERED,
};

/// The slot token used for SaveImage-fallback output rows (Section 6.9.6). A
/// fallback output carries no in-graph slot, so this stable sentinel is stored
/// in `source_output_slot` with a null `registration_id`.
pub const SAVEIMAGE_FALLBACK_SLOT: &str = "saveimage_fallback";

/// Capability required to register and route governed ComfyUI bridge outputs.
pub const ENGINE_COMFYUI_CAPABILITY: &str = "engine.comfyui";

pub const COMFY_WORKFLOW_RECEIPT_SCHEMA: &str = "hsk.atelier.comfy.workflow_receipt@1";

const ENGINE_COMFYUI_GRANT_PREFIX: &str = "capgrant://engine.comfyui/";
const REDACTED_PLACEHOLDER: &str = "[REDACTED]";

/// Scrub credential-bearing keys from a free-form provenance JSON object before
/// it is persisted or echoed into an event payload (LAW-COMFY-INTAKE-005).
///
/// legacy source's node POSTed a bearer token and raw image bytes; Handshake stores
/// neither. Any object key whose (lowercased) name looks like an auth header,
/// cookie, token, secret, password, or bearer credential is replaced with a
/// redaction placeholder, recursively. This is intentionally conservative: it
/// matches by key name, never by value, so a benign value under a sensitive key
/// is still masked. Non-object inputs (scalars, arrays of scalars) pass through
/// unchanged because they cannot carry a credential under a key.
pub fn scrub_provenance(value: &serde_json::Value) -> serde_json::Value {
    match value {
        serde_json::Value::Object(map) => {
            let mut out = serde_json::Map::with_capacity(map.len());
            for (key, child) in map {
                if is_secret_key(key) {
                    out.insert(
                        key.clone(),
                        serde_json::Value::String(REDACTED_PLACEHOLDER.into()),
                    );
                } else {
                    out.insert(key.clone(), scrub_provenance(child));
                }
            }
            serde_json::Value::Object(out)
        }
        serde_json::Value::Array(items) => {
            serde_json::Value::Array(items.iter().map(scrub_provenance).collect())
        }
        other => other.clone(),
    }
}

/// Whether a JSON object key names credential material that must be scrubbed.
fn is_secret_key(key: &str) -> bool {
    let lowered = key.to_ascii_lowercase();
    const NEEDLES: &[&str] = &[
        "authorization",
        "auth_header",
        "auth_token",
        "cookie",
        "token",
        "secret",
        "password",
        "passwd",
        "bearer",
        "api_key",
        "apikey",
        "access_key",
        "credential",
        "session_token",
    ];
    NEEDLES.iter().any(|needle| lowered.contains(needle))
}

/// Map a recorded governed ComfyUI output (`IntakeOutput`, Section 6.9.4) to the
/// Core intake lane it lands in (MT-107 / MT-108).
///
/// This is the image-sourcing adapter → Core intake-state bridge: a workflow
/// output that has been materialized through the ArtifactStore is routed into
/// the operator's accepted / pending / rejected lane vocabulary
/// ([`IntakeLane`]) so generated and imported outputs join the same triage
/// surface as folder-scanned sources. The decision is derived from the output's
/// [`RoutingIntent`]:
///   * `Artifact` — a first-class durable artifact → `Accepted` (it is a real,
///     gallery-visible output of the run).
///   * `Sidecar` — a durable sidecar bound to a primary → `Accepted` (it is a
///     materialized, lineage-bound output; sidecar visibility is a gallery
///     concern, not a triage-lane concern).
///   * `Transient` — a preview that is not persisted as a durable artifact →
///     `Skipped` (nothing durable to accept; the operator can re-run for a
///     persisted variant).
///
/// The function is pure and deterministic: the same output always maps to the
/// same lane, so the comfy→intake path produces idempotent, replay-stable lane
/// assignments. It never inspects credentials or raw bytes.
pub fn map_comfy_output_to_intake_lane(output: &IntakeOutput) -> IntakeLane {
    map_comfy_routing_intent_to_intake_lane(output.routing_intent)
}

/// Lane decision for a [`RoutingIntent`] without a full [`IntakeOutput`] record
/// (MT-108). Used to decide a lane at capability-declaration time, before an
/// output row exists, so the accepted/pending/rejected lanes are preserved for
/// both generated and imported outputs. Kept consistent with
/// [`map_comfy_output_to_intake_lane`] so there is exactly one routing rule.
pub fn map_comfy_routing_intent_to_intake_lane(routing_intent: RoutingIntent) -> IntakeLane {
    match routing_intent {
        RoutingIntent::Artifact | RoutingIntent::Sidecar => IntakeLane::Accepted,
        RoutingIntent::Transient => IntakeLane::Skipped,
    }
}

/// A Handshake-native ComfyUI execution endpoint reference and its declared
/// kind (MT-109). This is the adapter-boundary contract: Comfy-compatible
/// workflow execution is an integrated, capability-gated Handshake-native
/// adapter, never a manual ComfyUI app on a direct endpoint. The config carries
/// only a portable `endpoint_ref` (e.g. a capability-profile / managed-adapter
/// handle) — never a raw socket, localhost URL, or direct LLM/model-server
/// endpoint.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ComfyEndpointConfig {
    /// Portable Handshake-native reference to the managed execution adapter.
    pub endpoint_ref: String,
    /// Declared adapter kind; only the Handshake-native managed kind is
    /// authorized for execution.
    pub adapter_kind: ComfyAdapterKind,
}

/// The declared execution-adapter kind for a [`ComfyEndpointConfig`] (MT-109).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ComfyAdapterKind {
    /// The only authorized kind: execution runs through the capability-gated
    /// Handshake-native Workflow-Engine `engine.comfyui` adapter.
    HandshakeNativeManaged,
    /// A direct, un-managed ComfyUI endpoint (e.g. a raw `localhost:8188`
    /// server). Never authorized for execution; declaring this kind is a
    /// validation error.
    DirectComfyEndpoint,
    /// A direct LLM / model-server endpoint. Never authorized; declaring this
    /// kind is a validation error.
    DirectLlmEndpoint,
}

impl ComfyAdapterKind {
    pub fn as_token(self) -> &'static str {
        match self {
            ComfyAdapterKind::HandshakeNativeManaged => "handshake_native_managed",
            ComfyAdapterKind::DirectComfyEndpoint => "direct_comfy_endpoint",
            ComfyAdapterKind::DirectLlmEndpoint => "direct_llm_endpoint",
        }
    }

    /// Whether this adapter kind is authorized to execute Comfy-compatible
    /// workflows. Only the Handshake-native managed adapter is authorized.
    pub fn is_authorized_for_execution(self) -> bool {
        matches!(self, ComfyAdapterKind::HandshakeNativeManaged)
    }
}

impl ComfyEndpointConfig {
    /// Validate this endpoint config for the ComfyUI adapter boundary (MT-109).
    ///
    /// REJECTS any non-Handshake-native execution endpoint: a direct ComfyUI
    /// endpoint kind, a direct LLM / model-server endpoint kind, or an
    /// `endpoint_ref` that resolves to a localhost / direct-LLM / machine-local
    /// authority (reusing the canonical [`reject_legacy_runtime_ref`] boundary
    /// shared with every other atelier ref). Only a Handshake-native managed
    /// adapter with a portable ref passes.
    pub fn validate(&self) -> AtelierResult<()> {
        if !self.adapter_kind.is_authorized_for_execution() {
            return Err(AtelierError::Validation(format!(
                "ComfyUI endpoint adapter_kind {} is not authorized for execution; \
                 only the Handshake-native managed engine.comfyui adapter may execute workflows",
                self.adapter_kind.as_token()
            )));
        }
        // The ref itself must be a portable Handshake-native handle, not a
        // localhost / direct-LLM / machine-local execution endpoint. This is the
        // direct-localhost ComfyUI execution rejection (also exercised by MT-119
        // via reject_direct_localhost_comfy_execution).
        reject_legacy_runtime_ref("comfy endpoint_ref", &self.endpoint_ref)?;
        Ok(())
    }
}

/// Explicit guard rejecting DIRECT localhost ComfyUI execution (MT-119).
///
/// Comfy-compatible workflow execution is only permitted through the
/// capability-gated Handshake-native managed adapter. A direct
/// `localhost:8188`-style execution endpoint (or any loopback / machine-local /
/// direct-LLM authority) is never an authorized execution path. Returns an
/// error when `endpoint_ref` names such a direct endpoint; returns `Ok(())`
/// only for a portable Handshake-native ref. Reuses the canonical
/// [`reject_legacy_runtime_ref`] boundary so the localhost/loopback rejection
/// rule has a single source of truth.
pub fn reject_direct_localhost_comfy_execution(endpoint_ref: &str) -> AtelierResult<()> {
    reject_legacy_runtime_ref("comfy execution endpoint_ref", endpoint_ref)
}

fn validate_identity_metadata_component(
    field: &str,
    value: Option<&serde_json::Value>,
) -> AtelierResult<()> {
    if let Some(value) = value {
        if !value.is_object() && !value.is_array() {
            return Err(AtelierError::Validation(format!(
                "identity metadata {field} must be a JSON object or array"
            )));
        }
    }
    Ok(())
}

fn workflow_input_metadata_from_identity(
    metadata: Option<&IdentityWorkflowMetadata>,
) -> AtelierResult<serde_json::Value> {
    let Some(metadata) = metadata else {
        return Ok(serde_json::json!({ "identity": {} }));
    };
    validate_identity_metadata_component("landmarks", metadata.landmarks.as_ref())?;
    validate_identity_metadata_component("measurements", metadata.measurements.as_ref())?;
    validate_identity_metadata_component("pose_metadata", metadata.pose_metadata.as_ref())?;

    let mut identity = serde_json::Map::new();
    if let Some(landmarks) = &metadata.landmarks {
        identity.insert("landmarks".to_string(), scrub_provenance(landmarks));
    }
    if let Some(measurements) = &metadata.measurements {
        identity.insert("measurements".to_string(), scrub_provenance(measurements));
    }
    if let Some(pose_metadata) = &metadata.pose_metadata {
        identity.insert("pose_metadata".to_string(), scrub_provenance(pose_metadata));
    }
    Ok(serde_json::json!({ "identity": serde_json::Value::Object(identity) }))
}

/// Outcome of the bounded bridge-node presence probe (Section 6.9.2).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProbeOutcome {
    /// Bridge node present and protocol-compatible; the governed bridge path is
    /// taken.
    BridgePresent,
    /// Bridge node absent (including probe timeout); routes to SaveImage
    /// fallback (Section 6.9.6).
    BridgeAbsent,
    /// Bridge node present but `bridge_protocol_version` is outside the
    /// supported range; routes to fallback with the version in `fallback_reason`.
    BridgeIncompatible,
}

impl ProbeOutcome {
    /// Stable DB token.
    pub fn as_token(self) -> &'static str {
        match self {
            ProbeOutcome::BridgePresent => "bridge_present",
            ProbeOutcome::BridgeAbsent => "bridge_absent",
            ProbeOutcome::BridgeIncompatible => "bridge_incompatible",
        }
    }

    /// Parse a stored token; unknown tokens are a validation error rather than a
    /// silent default so a corrupt row never masquerades as a present bridge.
    pub fn from_token(token: &str) -> AtelierResult<Self> {
        match token {
            "bridge_present" => Ok(ProbeOutcome::BridgePresent),
            "bridge_absent" => Ok(ProbeOutcome::BridgeAbsent),
            "bridge_incompatible" => Ok(ProbeOutcome::BridgeIncompatible),
            other => Err(AtelierError::Validation(format!(
                "unknown comfy probe outcome token: {other}"
            ))),
        }
    }

    /// Whether this outcome takes the governed bridge path (vs. fallback).
    pub fn is_bridge_present(self) -> bool {
        matches!(self, ProbeOutcome::BridgePresent)
    }
}

/// Declared media kind of a bridge output port (Section 6.9.3).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MediaKind {
    Image,
    Mask,
    LatentPreview,
    Video,
    SidecarJson,
}

impl MediaKind {
    pub fn as_token(self) -> &'static str {
        match self {
            MediaKind::Image => "image",
            MediaKind::Mask => "mask",
            MediaKind::LatentPreview => "latent_preview",
            MediaKind::Video => "video",
            MediaKind::SidecarJson => "sidecar_json",
        }
    }

    pub fn from_token(token: &str) -> AtelierResult<Self> {
        match token {
            "image" => Ok(MediaKind::Image),
            "mask" => Ok(MediaKind::Mask),
            "latent_preview" => Ok(MediaKind::LatentPreview),
            "video" => Ok(MediaKind::Video),
            "sidecar_json" => Ok(MediaKind::SidecarJson),
            other => Err(AtelierError::Validation(format!(
                "unknown comfy media kind token: {other}"
            ))),
        }
    }
}

/// Routing intent of a declared/produced bridge output (Section 6.9.3/6.9.4).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RoutingIntent {
    /// Materialized as a first-class ArtifactStore artifact with its own
    /// manifest.
    Artifact,
    /// Materialized as a sidecar bound to a `parent_artifact_ref`; never an
    /// orphan primary.
    Sidecar,
    /// Streamed to the operator preview surface; NOT persisted as a durable
    /// artifact unless the capability profile permits preview persistence.
    Transient,
}

impl RoutingIntent {
    pub fn as_token(self) -> &'static str {
        match self {
            RoutingIntent::Artifact => "artifact",
            RoutingIntent::Sidecar => "sidecar",
            RoutingIntent::Transient => "transient",
        }
    }

    pub fn from_token(token: &str) -> AtelierResult<Self> {
        match token {
            "artifact" => Ok(RoutingIntent::Artifact),
            "sidecar" => Ok(RoutingIntent::Sidecar),
            "transient" => Ok(RoutingIntent::Transient),
            other => Err(AtelierError::Validation(format!(
                "unknown comfy routing intent token: {other}"
            ))),
        }
    }
}

/// Durable ComfyUI workflow receipt status.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ComfyWorkflowStatus {
    Queued,
    Running,
    Succeeded,
    Failed,
    Cancelled,
}

impl ComfyWorkflowStatus {
    pub fn as_token(self) -> &'static str {
        match self {
            ComfyWorkflowStatus::Queued => "queued",
            ComfyWorkflowStatus::Running => "running",
            ComfyWorkflowStatus::Succeeded => "succeeded",
            ComfyWorkflowStatus::Failed => "failed",
            ComfyWorkflowStatus::Cancelled => "cancelled",
        }
    }

    pub fn from_token(token: &str) -> AtelierResult<Self> {
        match token {
            "queued" => Ok(ComfyWorkflowStatus::Queued),
            "running" => Ok(ComfyWorkflowStatus::Running),
            "succeeded" => Ok(ComfyWorkflowStatus::Succeeded),
            "failed" => Ok(ComfyWorkflowStatus::Failed),
            "cancelled" => Ok(ComfyWorkflowStatus::Cancelled),
            other => Err(AtelierError::Validation(format!(
                "unknown comfy workflow status token: {other}"
            ))),
        }
    }
}

/// Recovery state for a generated output preserved before registration finished.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ComfyOutputRegistrationFailureStatus {
    Retryable,
    Registered,
}

impl ComfyOutputRegistrationFailureStatus {
    pub fn as_token(self) -> &'static str {
        match self {
            ComfyOutputRegistrationFailureStatus::Retryable => "retryable",
            ComfyOutputRegistrationFailureStatus::Registered => "registered",
        }
    }

    pub fn from_token(token: &str) -> AtelierResult<Self> {
        match token {
            "retryable" => Ok(ComfyOutputRegistrationFailureStatus::Retryable),
            "registered" => Ok(ComfyOutputRegistrationFailureStatus::Registered),
            other => Err(AtelierError::Validation(format!(
                "unknown comfy output registration failure status token: {other}"
            ))),
        }
    }
}

/// A bridge-node presence probe record (`ComfyBridgeNodeProbeV1`, Section 6.9.2).
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct BridgeProbe {
    pub probe_id: Uuid,
    pub workflow_run_id: Uuid,
    /// Bridge node class name, e.g. `HandshakeIntakeBridge`.
    pub node_class_id: String,
    pub detected: bool,
    /// Semver of the detected bridge protocol; null when not detected.
    pub bridge_protocol_version: Option<String>,
    /// Graph node ids bound to the bridge class within this graph.
    pub node_instance_ids: Vec<String>,
    pub probe_outcome: ProbeOutcome,
    /// Required when outcome != `bridge_present` (enforced server-side).
    pub fallback_reason: Option<String>,
    pub probed_at_utc: DateTime<Utc>,
}

/// Parameters to record a bridge probe (the engine writes this after probing).
#[derive(Clone, Debug)]
pub struct NewBridgeProbe {
    pub workflow_run_id: Uuid,
    pub node_class_id: String,
    pub bridge_protocol_version: Option<String>,
    pub node_instance_ids: Vec<String>,
    pub probe_outcome: ProbeOutcome,
    pub fallback_reason: Option<String>,
}

/// A declared output port from the bridge capability registration (6.9.3).
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct DeclaredOutput {
    /// Node output port name.
    pub output_slot: String,
    pub media_kind: MediaKind,
    pub expected_mime: String,
    pub routing_intent: RoutingIntent,
}

/// A bridge capability registration record (`ComfyBridgeCapabilityRecordV1`,
/// Section 6.9.3). Exactly one per job; declared outputs and rejects are stored
/// as ordered child rows keyed on `registration_id`.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct CapabilityRegistration {
    pub registration_id: Uuid,
    pub workflow_run_id: Uuid,
    pub node_class_id: String,
    pub bridge_protocol_version: String,
    /// Outputs accepted by the capability gate (the routable set).
    pub declared_outputs: Vec<DeclaredOutput>,
    /// FK to the granted `engine.comfyui` capability registry entry / profile.
    pub capability_grant_ref: String,
    /// FK to a consent decision when escalation/consent applied; null otherwise.
    pub consent_decision_ref: Option<String>,
    pub registered_at_utc: DateTime<Utc>,
}

/// Parameters to register the bridge capability for a job.
#[derive(Clone, Debug)]
pub struct NewCapabilityRegistration {
    pub workflow_run_id: Uuid,
    pub node_class_id: String,
    pub bridge_protocol_version: String,
    /// Outputs that passed the capability gate (routable).
    pub accepted_outputs: Vec<DeclaredOutput>,
    /// Outputs dropped by the gate: `(output_slot, reason)`. Recorded as typed
    /// reject rows + `CAPABILITY_REJECTED` events; never routed.
    pub rejected_outputs: Vec<(String, String)>,
    pub capability_grant_ref: String,
    pub consent_decision_ref: Option<String>,
}

/// Deterministic offline adapter for the Section 6.9 bridge-node contract.
///
/// This adapter never opens a socket, spawns ComfyUI, reads local output
/// folders, or persists bytes. It produces the same probe + capability
/// registration shape as the governed bridge path so acceptance tests can prove
/// the storage, gating, EventLedger, and reject-row behavior without external
/// runtime dependency.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ComfyBridgeFakeAdapterV1 {
    accepted_outputs: Vec<DeclaredOutput>,
    rejected_outputs: Vec<(String, String)>,
}

impl Default for ComfyBridgeFakeAdapterV1 {
    fn default() -> Self {
        Self {
            accepted_outputs: vec![
                DeclaredOutput {
                    output_slot: "IMAGE".to_string(),
                    media_kind: MediaKind::Image,
                    expected_mime: "image/png".to_string(),
                    routing_intent: RoutingIntent::Artifact,
                },
                DeclaredOutput {
                    output_slot: "MASK".to_string(),
                    media_kind: MediaKind::Mask,
                    expected_mime: "image/png".to_string(),
                    routing_intent: RoutingIntent::Sidecar,
                },
            ],
            rejected_outputs: vec![(
                "PREVIEW".to_string(),
                "transient preview not permitted by capability profile".to_string(),
            )],
        }
    }
}

impl ComfyBridgeFakeAdapterV1 {
    pub const NODE_CLASS_ID: &'static str = "ComfyBridgeFakeAdapterV1";
    pub const BRIDGE_PROTOCOL_VERSION: &'static str = "1.0.0";
    pub const CAPABILITY_PROFILE_ID: &'static str = "ComfyUIWorker";

    pub fn accepted_outputs(&self) -> &[DeclaredOutput] {
        &self.accepted_outputs
    }

    pub fn rejected_outputs(&self) -> &[(String, String)] {
        &self.rejected_outputs
    }

    pub fn probe(&self, workflow_run_id: Uuid) -> NewBridgeProbe {
        NewBridgeProbe {
            workflow_run_id,
            node_class_id: Self::NODE_CLASS_ID.to_string(),
            bridge_protocol_version: Some(Self::BRIDGE_PROTOCOL_VERSION.to_string()),
            node_instance_ids: vec!["fake-bridge-node".to_string()],
            probe_outcome: ProbeOutcome::BridgePresent,
            fallback_reason: None,
        }
    }

    pub fn capability_grant_ref(profile_id: &str, evidence_ref: &str) -> String {
        format!("{ENGINE_COMFYUI_GRANT_PREFIX}{profile_id}/{evidence_ref}")
    }

    pub fn capability_registration(
        &self,
        workflow_run_id: Uuid,
        capability_profile_id: &str,
        evidence_ref: &str,
    ) -> NewCapabilityRegistration {
        NewCapabilityRegistration {
            workflow_run_id,
            node_class_id: Self::NODE_CLASS_ID.to_string(),
            bridge_protocol_version: Self::BRIDGE_PROTOCOL_VERSION.to_string(),
            accepted_outputs: self.accepted_outputs.clone(),
            rejected_outputs: self.rejected_outputs.clone(),
            capability_grant_ref: Self::capability_grant_ref(capability_profile_id, evidence_ref),
            consent_decision_ref: None,
        }
    }
}

fn validate_engine_comfyui_grant_ref(grant_ref: &str) -> AtelierResult<()> {
    let trimmed = grant_ref.trim();
    let rest = trimmed.strip_prefix(ENGINE_COMFYUI_GRANT_PREFIX).ok_or_else(|| {
        AtelierError::Validation(format!(
            "capability_grant_ref must start with {ENGINE_COMFYUI_GRANT_PREFIX} and name a profile granted {ENGINE_COMFYUI_CAPABILITY}"
        ))
    })?;
    let (profile_id, evidence_ref) = rest.split_once('/').ok_or_else(|| {
        AtelierError::Validation(format!(
            "capability_grant_ref must include profile/evidence for {ENGINE_COMFYUI_CAPABILITY}"
        ))
    })?;
    if profile_id.trim().is_empty() || evidence_ref.trim().is_empty() {
        return Err(AtelierError::Validation(format!(
            "capability_grant_ref must include non-empty profile/evidence for {ENGINE_COMFYUI_CAPABILITY}"
        )));
    }
    reject_legacy_runtime_ref("capability_grant_ref evidence_ref", evidence_ref)?;

    match CapabilityRegistry::new().profile_can(profile_id, ENGINE_COMFYUI_CAPABILITY) {
        Ok(true) => Ok(()),
        Ok(false) => Err(AtelierError::Validation(format!(
            "capability profile {profile_id} is not granted {ENGINE_COMFYUI_CAPABILITY}"
        ))),
        Err(err) => Err(AtelierError::Validation(format!(
            "capability profile {profile_id} cannot grant {ENGINE_COMFYUI_CAPABILITY}: {err}"
        ))),
    }
}

/// One governed intake output record (`ComfyIntakeOutputRecordV1`, 6.9.4), with
/// the workflow-receipt lineage (prompt-json ref, output image artifact_ref,
/// graph hash, seed). Used both for bridge-routed outputs and SaveImage-fallback
/// outputs (the fallback variant has `registration_id = None`,
/// `source_output_slot = SAVEIMAGE_FALLBACK_SLOT`).
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct IntakeOutput {
    pub intake_output_id: Uuid,
    pub workflow_run_id: Uuid,
    /// FK -> `PRIM-WorkflowNodeExecution` (string handle; portable).
    pub node_execution_id: String,
    /// FK to the capability registration; null for SaveImage-fallback rows.
    pub registration_id: Option<Uuid>,
    /// Graph node id that produced this output; the fallback sentinel for scans.
    pub source_node_instance_id: String,
    /// Output port name; `SAVEIMAGE_FALLBACK_SLOT` for fallback rows.
    pub source_output_slot: String,
    pub media_kind: MediaKind,
    pub mime: String,
    /// ArtifactStore content-addressed handle (portable; never a machine-local
    /// path).
    pub artifact_ref: String,
    /// Manifest produced at materialization (content hash, byte length, mime,
    /// provenance pins). Stored as a ref, never inlined bytes.
    pub artifact_manifest_ref: String,
    /// Content hash used for idempotent dedup on `(workflow_run_id, *)`.
    pub content_hash: String,
    pub routing_intent: RoutingIntent,
    /// For `sidecar`: the primary artifact it annotates; null otherwise.
    pub parent_artifact_ref: Option<String>,
    // --- Workflow receipt lineage (Section 6.9.4 + 9.12 STOCHASTIC pins) ---
    /// ArtifactStore ref to the captured prompt/graph JSON (legacy source `workflow_json`).
    pub prompt_json_ref: Option<String>,
    /// Graph hash pin (9.12 determinism).
    pub graph_hash: Option<String>,
    /// Seed pin (9.12 STOCHASTIC determinism).
    pub seed: Option<i64>,
    /// Workflow input metadata preserved for receipt reconstruction. Identity
    /// metadata is optional; absence is stored as `{ "identity": {} }`.
    pub workflow_input_metadata: serde_json::Value,
    pub materialized_at_utc: DateTime<Utc>,
}

/// Optional identity metadata serialized into workflow receipt inputs (MT-100).
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct IdentityWorkflowMetadata {
    pub landmarks: Option<serde_json::Value>,
    pub measurements: Option<serde_json::Value>,
    pub pose_metadata: Option<serde_json::Value>,
}

/// Parameters to record a routed intake output. Use [`Self::saveimage_fallback`]
/// for the no-bridge variant.
#[derive(Clone, Debug)]
pub struct NewIntakeOutput {
    pub workflow_run_id: Uuid,
    pub node_execution_id: String,
    pub registration_id: Option<Uuid>,
    pub source_node_instance_id: String,
    pub source_output_slot: String,
    pub media_kind: MediaKind,
    pub mime: String,
    pub artifact_ref: String,
    pub artifact_manifest_ref: String,
    pub content_hash: String,
    pub routing_intent: RoutingIntent,
    pub parent_artifact_ref: Option<String>,
    pub prompt_json_ref: Option<String>,
    pub graph_hash: Option<String>,
    pub seed: Option<i64>,
    pub identity_metadata: Option<IdentityWorkflowMetadata>,
}

impl NewIntakeOutput {
    /// Build a SaveImage-fallback output record (Section 6.9.6): no
    /// registration, the fallback sentinel slot, and `artifact` routing (the
    /// fallback only materializes durable artifacts, never sidecars/transients).
    pub fn saveimage_fallback(
        workflow_run_id: Uuid,
        node_execution_id: impl Into<String>,
        source_node_instance_id: impl Into<String>,
        mime: impl Into<String>,
        artifact_ref: impl Into<String>,
        artifact_manifest_ref: impl Into<String>,
        content_hash: impl Into<String>,
    ) -> Self {
        NewIntakeOutput {
            workflow_run_id,
            node_execution_id: node_execution_id.into(),
            registration_id: None,
            source_node_instance_id: source_node_instance_id.into(),
            source_output_slot: SAVEIMAGE_FALLBACK_SLOT.to_string(),
            media_kind: MediaKind::Image,
            mime: mime.into(),
            artifact_ref: artifact_ref.into(),
            artifact_manifest_ref: artifact_manifest_ref.into(),
            content_hash: content_hash.into(),
            routing_intent: RoutingIntent::Artifact,
            parent_artifact_ref: None,
            prompt_json_ref: None,
            graph_hash: None,
            seed: None,
            identity_metadata: None,
        }
    }
}

/// Result of recording an intake output: whether it was a fresh materialization
/// or an idempotent dedup hit (Section 6.9.4 dedup evidence).
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct RecordOutputOutcome {
    pub output: IntakeOutput,
    /// `true` when this content hash was already present for the run and the
    /// existing artifact was returned instead of a duplicate.
    pub deduplicated: bool,
}

/// The per-job intake receipt summary (`ComfyIntakeReceiptV1`, Section 6.9.5),
/// derived from the recorded records so a no-context model can reconstruct what
/// intake produced for the job.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct IntakeReceipt {
    pub workflow_run_id: Uuid,
    pub probe_outcome: Option<ProbeOutcome>,
    pub registered_output_count: i64,
    pub materialized_artifact_refs: Vec<String>,
    pub workflow_inputs: Vec<serde_json::Value>,
    pub fallback_engaged: bool,
}

/// Parameters to record the durable ComfyUI workflow receipt (MT-101).
#[derive(Clone, Debug)]
pub struct NewComfyWorkflowReceipt {
    pub system_id: String,
    pub workflow_run_id: Uuid,
    pub workflow_spec_ref: String,
    pub workflow_json_ref: String,
    pub prompt_ref: String,
    pub status: ComfyWorkflowStatus,
    pub error_ref: Option<String>,
    pub evidence: serde_json::Value,
}

/// Durable workflow-level ComfyUI receipt, including all refs and output rows.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ComfyWorkflowReceipt {
    pub receipt_id: Uuid,
    pub system_id: String,
    pub workflow_run_id: Uuid,
    pub character_ref: Option<String>,
    pub workflow_spec_ref: String,
    pub workflow_json_ref: String,
    pub prompt_ref: String,
    pub all_refs: serde_json::Value,
    pub outputs: Vec<serde_json::Value>,
    pub status: ComfyWorkflowStatus,
    pub error_ref: Option<String>,
    pub evidence: serde_json::Value,
    pub receipt_json: serde_json::Value,
    pub created_at_utc: DateTime<Utc>,
    pub updated_at_utc: DateTime<Utc>,
}

/// Filters for workflow receipt history and stats queries (MT-103).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ComfyWorkflowHistoryQuery {
    pub character_ref: Option<String>,
    pub workflow_spec_ref: Option<String>,
    pub status: Option<ComfyWorkflowStatus>,
    pub from_utc: Option<DateTime<Utc>>,
    pub to_utc: Option<DateTime<Utc>>,
}

/// Aggregate stats over the same filtered receipt set returned by history.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ComfyWorkflowStats {
    pub total_count: i64,
    pub failure_count: i64,
    pub status_counts: BTreeMap<String, i64>,
}

/// Parameters to preserve a generated output when registration failed after save.
#[derive(Clone, Debug)]
pub struct NewComfyOutputRegistrationFailure {
    pub workflow_run_id: Uuid,
    pub node_execution_id: String,
    pub attempted_registration_id: Option<Uuid>,
    pub source_node_instance_id: String,
    pub source_output_slot: String,
    pub media_kind: MediaKind,
    pub mime: String,
    pub artifact_ref: String,
    pub artifact_manifest_ref: String,
    pub content_hash: String,
    pub routing_intent: RoutingIntent,
    pub parent_artifact_ref: Option<String>,
    pub prompt_json_ref: Option<String>,
    pub graph_hash: Option<String>,
    pub seed: Option<i64>,
    pub identity_metadata: Option<IdentityWorkflowMetadata>,
    pub failure_stage: String,
    pub failure_reason: String,
    pub evidence: serde_json::Value,
}

/// Durable retryable evidence for output-first registration failure recovery.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ComfyOutputRegistrationFailure {
    pub failure_id: Uuid,
    pub workflow_run_id: Uuid,
    pub node_execution_id: String,
    pub attempted_registration_id: Option<Uuid>,
    pub source_node_instance_id: String,
    pub source_output_slot: String,
    pub media_kind: MediaKind,
    pub mime: String,
    pub artifact_ref: String,
    pub artifact_manifest_ref: String,
    pub content_hash: String,
    pub routing_intent: RoutingIntent,
    pub parent_artifact_ref: Option<String>,
    pub prompt_json_ref: Option<String>,
    pub graph_hash: Option<String>,
    pub seed: Option<i64>,
    pub workflow_input_metadata: serde_json::Value,
    pub failure_stage: String,
    pub failure_reason: String,
    pub evidence: serde_json::Value,
    pub status: ComfyOutputRegistrationFailureStatus,
    pub retry_count: i32,
    pub resolved_intake_output_id: Option<Uuid>,
    pub created_at_utc: DateTime<Utc>,
    pub updated_at_utc: DateTime<Utc>,
}

// ---------------------------------------------------------------------------
// Replay Input Contract (MT-104) + Replay Event Family wiring (MT-130).
//
// A `ReplayRequest` captures exactly what a no-context engine needs to replay a
// prior Comfy-compatible workflow run: the workflow identity (run id + spec/json
// refs) plus the set of input artifact refs the graph consumed. Resolution
// proves every input ref is (a) a Handshake-native PORTABLE handle (never a
// legacy/`.GOV`/SQLite/localhost/machine-local ref -- rejected via the canonical
// `reject_legacy_runtime_ref` boundary) AND (b) resolves to a STORED artifact:
// an `atelier_comfy_intake_output` row for THIS run carrying that `artifact_ref`.
// Replay never executes ComfyUI; it only resolves durable inputs, exactly like
// the rest of this module (LAW-COMFY-INTAKE-001). Storage stays in the existing
// `atelier_comfy_intake_output` + `atelier_event` tables; no new table.
// ---------------------------------------------------------------------------

/// A replayable-workflow-inputs request (MT-104). Captures the workflow identity
/// and the set of input artifact refs required to replay the run.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReplayRequest {
    /// The workflow run being replayed; replay inputs are resolved within this
    /// run's stored outputs (containment, Section 6.9.1).
    pub workflow_run_id: Uuid,
    /// Portable ArtifactStore ref to the captured workflow spec.
    pub workflow_spec_ref: String,
    /// Portable ArtifactStore ref to the captured workflow/prompt graph JSON.
    pub workflow_json_ref: String,
    /// Optional graph hash pin (9.12 determinism) preserved for replay identity.
    pub graph_hash: Option<String>,
    /// Optional seed pin (9.12 STOCHASTIC determinism) preserved for replay.
    pub seed: Option<i64>,
    /// The set of input artifact refs the graph consumed and that must resolve to
    /// stored artifacts for the replay to be reproducible.
    pub input_artifact_refs: Vec<String>,
}

/// The resolution outcome for one replay input artifact ref (MT-104).
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ResolvedReplayInput {
    /// The portable ref that was requested.
    pub artifact_ref: String,
    /// The stored intake output the ref resolved to (containment evidence).
    pub intake_output_id: Uuid,
    /// The stored content hash for the resolved artifact (replay determinism).
    pub content_hash: String,
}

/// The successful resolution of a [`ReplayRequest`] (MT-104): every input ref
/// resolved to a stored artifact for the run.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ResolvedReplayInputs {
    pub workflow_run_id: Uuid,
    pub workflow_spec_ref: String,
    pub workflow_json_ref: String,
    pub graph_hash: Option<String>,
    pub seed: Option<i64>,
    pub resolved_inputs: Vec<ResolvedReplayInput>,
}

// ---------------------------------------------------------------------------
// Workflow Spec Registry (MT-106) + External Tool/Model Version Policy (MT-110).
//
// A workflow spec is the durable, versioned, replay-stable contract for a
// ComfyUI/pose workflow graph: the graph/spec JSON, a content hash pin, the
// handler that routes the workflow, and an optional compatibility pin. Identity
// is (workflow_kind, spec_version); re-registration is an idempotent upsert.
// Version metadata pins the three named provenance versions (pose model-asset,
// image tool, ComfyUI model) per workflow run so provenance is reproducible.
// Storage stays PostgreSQL only (LAW-COMFY-INTAKE-004). Neither path executes
// ComfyUI; they only persist durable governed records (LAW-COMFY-INTAKE-001).
// ---------------------------------------------------------------------------

/// A registered, versioned ComfyUI/pose workflow spec (MT-106).
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorkflowSpec {
    pub spec_id: Uuid,
    /// Workflow family/kind, e.g. `pose_rig_v1`.
    pub workflow_kind: String,
    /// The spec version pin (part of identity).
    pub spec_version: String,
    /// Content-addressed pin over the spec graph/contract.
    pub spec_hash: String,
    /// Handler that routes this workflow (engine.comfyui adapter handler id).
    pub handler_id: String,
    /// Optional compatibility pin (bridge protocol / engine version).
    pub compatibility_pin: Option<String>,
    /// The workflow graph / contract JSON.
    pub spec_json: serde_json::Value,
    /// Optional portable, read-only source ref the spec was lifted from.
    pub source_ref: Option<String>,
    pub created_at_utc: DateTime<Utc>,
    pub updated_at_utc: DateTime<Utc>,
}

/// Parameters to register a versioned workflow spec (MT-106). Registration is
/// idempotent on `(workflow_kind, spec_version)`.
#[derive(Clone, Debug)]
pub struct NewWorkflowSpec {
    pub workflow_kind: String,
    pub spec_version: String,
    pub spec_hash: String,
    pub handler_id: String,
    pub compatibility_pin: Option<String>,
    pub spec_json: serde_json::Value,
    pub source_ref: Option<String>,
}

/// Pinned external tool and model versions for a workflow run (MT-110).
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ComfyVersionMetadata {
    pub version_metadata_id: Uuid,
    pub workflow_run_id: Uuid,
    /// Optional registered workflow spec this run executed.
    pub spec_id: Option<Uuid>,
    /// Pinned pose model-asset version.
    pub pose_model_asset_version: String,
    /// Pinned image-tool version (blur/sharpen/etc).
    pub image_tool_version: String,
    /// Pinned ComfyUI model version.
    pub comfy_model_version: String,
    /// Optional preflight discovery evidence.
    pub preflight_evidence: serde_json::Value,
    pub created_at_utc: DateTime<Utc>,
    pub updated_at_utc: DateTime<Utc>,
}

/// Parameters to record version metadata for a run (MT-110). Idempotent on
/// `workflow_run_id`.
#[derive(Clone, Debug)]
pub struct NewComfyVersionMetadata {
    pub workflow_run_id: Uuid,
    pub spec_id: Option<Uuid>,
    pub pose_model_asset_version: String,
    pub image_tool_version: String,
    pub comfy_model_version: String,
    pub preflight_evidence: serde_json::Value,
}

fn workflow_spec_from_row(row: &sqlx::postgres::PgRow) -> WorkflowSpec {
    WorkflowSpec {
        spec_id: row.get("spec_id"),
        workflow_kind: row.get("workflow_kind"),
        spec_version: row.get("spec_version"),
        spec_hash: row.get("spec_hash"),
        handler_id: row.get("handler_id"),
        compatibility_pin: row.get("compatibility_pin"),
        spec_json: row.get("spec_json"),
        source_ref: row.get("source_ref"),
        created_at_utc: row.get("created_at_utc"),
        updated_at_utc: row.get("updated_at_utc"),
    }
}

fn version_metadata_from_row(row: &sqlx::postgres::PgRow) -> ComfyVersionMetadata {
    ComfyVersionMetadata {
        version_metadata_id: row.get("version_metadata_id"),
        workflow_run_id: row.get("workflow_run_id"),
        spec_id: row.get("spec_id"),
        pose_model_asset_version: row.get("pose_model_asset_version"),
        image_tool_version: row.get("image_tool_version"),
        comfy_model_version: row.get("comfy_model_version"),
        preflight_evidence: row.get("preflight_evidence"),
        created_at_utc: row.get("created_at_utc"),
        updated_at_utc: row.get("updated_at_utc"),
    }
}

fn require_token_field(field: &str, value: &str) -> AtelierResult<()> {
    if value.trim().is_empty() || value.trim() != value {
        return Err(AtelierError::Validation(format!(
            "{field} must not be empty or padded"
        )));
    }
    Ok(())
}

fn probe_from_row(row: &sqlx::postgres::PgRow) -> AtelierResult<BridgeProbe> {
    let outcome: String = row.get("probe_outcome");
    let node_instance_ids: serde_json::Value = row.get("node_instance_ids");
    let node_instance_ids: Vec<String> = serde_json::from_value(node_instance_ids)
        .map_err(|e| AtelierError::Validation(format!("invalid node_instance_ids json: {e}")))?;
    Ok(BridgeProbe {
        probe_id: row.get("probe_id"),
        workflow_run_id: row.get("workflow_run_id"),
        node_class_id: row.get("node_class_id"),
        detected: row.get("detected"),
        bridge_protocol_version: row.get("bridge_protocol_version"),
        node_instance_ids,
        probe_outcome: ProbeOutcome::from_token(&outcome)?,
        fallback_reason: row.get("fallback_reason"),
        probed_at_utc: row.get("probed_at_utc"),
    })
}

fn declared_output_from_row(row: &sqlx::postgres::PgRow) -> AtelierResult<DeclaredOutput> {
    let media_kind: String = row.get("media_kind");
    let routing_intent: String = row.get("routing_intent");
    Ok(DeclaredOutput {
        output_slot: row.get("output_slot"),
        media_kind: MediaKind::from_token(&media_kind)?,
        expected_mime: row.get("expected_mime"),
        routing_intent: RoutingIntent::from_token(&routing_intent)?,
    })
}

fn output_from_row(row: &sqlx::postgres::PgRow) -> AtelierResult<IntakeOutput> {
    let media_kind: String = row.get("media_kind");
    let routing_intent: String = row.get("routing_intent");
    Ok(IntakeOutput {
        intake_output_id: row.get("intake_output_id"),
        workflow_run_id: row.get("workflow_run_id"),
        node_execution_id: row.get("node_execution_id"),
        registration_id: row.get("registration_id"),
        source_node_instance_id: row.get("source_node_instance_id"),
        source_output_slot: row.get("source_output_slot"),
        media_kind: MediaKind::from_token(&media_kind)?,
        mime: row.get("mime"),
        artifact_ref: row.get("artifact_ref"),
        artifact_manifest_ref: row.get("artifact_manifest_ref"),
        content_hash: row.get("content_hash"),
        routing_intent: RoutingIntent::from_token(&routing_intent)?,
        parent_artifact_ref: row.get("parent_artifact_ref"),
        prompt_json_ref: row.get("prompt_json_ref"),
        graph_hash: row.get("graph_hash"),
        seed: row.get("seed"),
        workflow_input_metadata: row.get("workflow_input_metadata"),
        materialized_at_utc: row.get("materialized_at_utc"),
    })
}

fn validate_workflow_system_id(system_id: &str) -> AtelierResult<()> {
    if system_id.trim().is_empty() || system_id.trim() != system_id {
        return Err(AtelierError::Validation(
            "system_id must not be empty or padded".into(),
        ));
    }
    if !system_id
        .bytes()
        .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'-' | b'_'))
    {
        return Err(AtelierError::Validation(
            "system_id must be a stable ASCII token".into(),
        ));
    }
    Ok(())
}

fn workflow_receipt_outputs_from_value(
    value: serde_json::Value,
) -> AtelierResult<Vec<serde_json::Value>> {
    match value {
        serde_json::Value::Array(values) => Ok(values),
        _ => Err(AtelierError::Validation(
            "workflow receipt outputs must be a JSON array".into(),
        )),
    }
}

fn workflow_receipt_output_from_row(row: &sqlx::postgres::PgRow) -> serde_json::Value {
    let intake_output_id: Uuid = row.get("intake_output_id");
    let workflow_run_id: Uuid = row.get("workflow_run_id");
    let media_kind: String = row.get("media_kind");
    let routing_intent: String = row.get("routing_intent");
    let registration_id: Option<Uuid> = row.get("registration_id");
    let parent_artifact_ref: Option<String> = row.get("parent_artifact_ref");
    let prompt_json_ref: Option<String> = row.get("prompt_json_ref");
    let graph_hash: Option<String> = row.get("graph_hash");
    let seed: Option<i64> = row.get("seed");
    let materialized_at_utc: DateTime<Utc> = row.get("materialized_at_utc");
    serde_json::json!({
        "intake_output_id": intake_output_id,
        "workflow_run_id": workflow_run_id,
        "node_execution_id": row.get::<String, _>("node_execution_id"),
        "registration_id": registration_id,
        "source_node_instance_id": row.get::<String, _>("source_node_instance_id"),
        "source_output_slot": row.get::<String, _>("source_output_slot"),
        "media_kind": media_kind,
        "mime": row.get::<String, _>("mime"),
        "artifact_ref": row.get::<String, _>("artifact_ref"),
        "artifact_manifest_ref": row.get::<String, _>("artifact_manifest_ref"),
        "content_hash": row.get::<String, _>("content_hash"),
        "routing_intent": routing_intent,
        "parent_artifact_ref": parent_artifact_ref,
        "prompt_json_ref": prompt_json_ref,
        "graph_hash": graph_hash,
        "seed": seed,
        "workflow_input_metadata": row.get::<serde_json::Value, _>("workflow_input_metadata"),
        "materialized_at_utc": materialized_at_utc,
    })
}

fn workflow_receipt_refs(
    new: &NewComfyWorkflowReceipt,
    outputs: &[serde_json::Value],
) -> serde_json::Value {
    let output_refs: Vec<serde_json::Value> = outputs
        .iter()
        .map(|output| {
            serde_json::json!({
                "intake_output_id": output["intake_output_id"].clone(),
                "artifact_ref": output["artifact_ref"].clone(),
                "artifact_manifest_ref": output["artifact_manifest_ref"].clone(),
                "prompt_json_ref": output["prompt_json_ref"].clone(),
            })
        })
        .collect();
    serde_json::json!({
        "workflow_spec_ref": &new.workflow_spec_ref,
        "workflow_json_ref": &new.workflow_json_ref,
        "prompt_ref": &new.prompt_ref,
        "error_ref": &new.error_ref,
        "outputs": output_refs,
    })
}

fn workflow_receipt_character_ref(evidence: &serde_json::Value) -> AtelierResult<Option<String>> {
    let Some(value) = evidence.get("character_ref") else {
        return Ok(None);
    };
    if value.is_null() {
        return Ok(None);
    }
    let Some(character_ref) = value.as_str() else {
        return Err(AtelierError::Validation(
            "workflow receipt character_ref evidence must be a string".into(),
        ));
    };
    reject_legacy_runtime_ref("character_ref", character_ref)?;
    Ok(Some(character_ref.to_string()))
}

fn workflow_receipt_from_row(row: &sqlx::postgres::PgRow) -> AtelierResult<ComfyWorkflowReceipt> {
    let status: String = row.get("status");
    let outputs: serde_json::Value = row.get("outputs");
    Ok(ComfyWorkflowReceipt {
        receipt_id: row.get("receipt_id"),
        system_id: row.get("system_id"),
        workflow_run_id: row.get("workflow_run_id"),
        character_ref: row.get("character_ref"),
        workflow_spec_ref: row.get("workflow_spec_ref"),
        workflow_json_ref: row.get("workflow_json_ref"),
        prompt_ref: row.get("prompt_ref"),
        all_refs: row.get("all_refs"),
        outputs: workflow_receipt_outputs_from_value(outputs)?,
        status: ComfyWorkflowStatus::from_token(&status)?,
        error_ref: row.get("error_ref"),
        evidence: row.get("evidence"),
        receipt_json: row.get("receipt_json"),
        created_at_utc: row.get("created_at_utc"),
        updated_at_utc: row.get("updated_at_utc"),
    })
}

fn validate_registration_failure_shape(
    artifact_ref: &str,
    artifact_manifest_ref: &str,
    content_hash: &str,
    parent_artifact_ref: Option<&String>,
    prompt_json_ref: Option<&String>,
    routing_intent: RoutingIntent,
    failure_stage: &str,
    failure_reason: &str,
    evidence: &serde_json::Value,
) -> AtelierResult<()> {
    reject_legacy_runtime_ref("artifact_ref", artifact_ref)?;
    reject_legacy_runtime_ref("artifact_manifest_ref", artifact_manifest_ref)?;
    if content_hash.trim().is_empty() || content_hash.trim() != content_hash {
        return Err(AtelierError::Validation(
            "content_hash must not be empty or padded".into(),
        ));
    }
    if let Some(parent_artifact_ref) = parent_artifact_ref {
        reject_legacy_runtime_ref("parent_artifact_ref", parent_artifact_ref)?;
    }
    if let Some(prompt_json_ref) = prompt_json_ref {
        reject_legacy_runtime_ref("prompt_json_ref", prompt_json_ref)?;
    }
    if matches!(routing_intent, RoutingIntent::Sidecar)
        && parent_artifact_ref
            .map(|value| value.trim().is_empty())
            .unwrap_or(true)
    {
        return Err(AtelierError::Validation(
            "sidecar output registration failure requires parent_artifact_ref".into(),
        ));
    }
    if failure_stage.trim().is_empty() || failure_stage.trim() != failure_stage {
        return Err(AtelierError::Validation(
            "failure_stage must not be empty or padded".into(),
        ));
    }
    if failure_reason.trim().is_empty() || failure_reason.trim() != failure_reason {
        return Err(AtelierError::Validation(
            "failure_reason must not be empty or padded".into(),
        ));
    }
    if !evidence.is_object() {
        return Err(AtelierError::Validation(
            "output registration failure evidence must be a JSON object".into(),
        ));
    }
    Ok(())
}

fn output_registration_failure_from_row(
    row: &sqlx::postgres::PgRow,
) -> AtelierResult<ComfyOutputRegistrationFailure> {
    let media_kind: String = row.get("media_kind");
    let routing_intent: String = row.get("routing_intent");
    let status: String = row.get("status");
    Ok(ComfyOutputRegistrationFailure {
        failure_id: row.get("failure_id"),
        workflow_run_id: row.get("workflow_run_id"),
        node_execution_id: row.get("node_execution_id"),
        attempted_registration_id: row.get("attempted_registration_id"),
        source_node_instance_id: row.get("source_node_instance_id"),
        source_output_slot: row.get("source_output_slot"),
        media_kind: MediaKind::from_token(&media_kind)?,
        mime: row.get("mime"),
        artifact_ref: row.get("artifact_ref"),
        artifact_manifest_ref: row.get("artifact_manifest_ref"),
        content_hash: row.get("content_hash"),
        routing_intent: RoutingIntent::from_token(&routing_intent)?,
        parent_artifact_ref: row.get("parent_artifact_ref"),
        prompt_json_ref: row.get("prompt_json_ref"),
        graph_hash: row.get("graph_hash"),
        seed: row.get("seed"),
        workflow_input_metadata: row.get("workflow_input_metadata"),
        failure_stage: row.get("failure_stage"),
        failure_reason: row.get("failure_reason"),
        evidence: row.get("evidence"),
        status: ComfyOutputRegistrationFailureStatus::from_token(&status)?,
        retry_count: row.get("retry_count"),
        resolved_intake_output_id: row.get("resolved_intake_output_id"),
        created_at_utc: row.get("created_at_utc"),
        updated_at_utc: row.get("updated_at_utc"),
    })
}

impl AtelierStore {
    /// Record the bounded bridge-node presence probe for a job (Section 6.9.2).
    ///
    /// Idempotent on `workflow_run_id`: a job probes exactly once, so
    /// re-recording the same run returns/refreshes the single probe row rather
    /// than duplicating it (ON CONFLICT on the unique run column). Enforces the
    /// normative rule that `fallback_reason` is required whenever the outcome is
    /// not `bridge_present`. Emits `PROBE_RECORDED`.
    pub async fn record_bridge_probe(&self, new: &NewBridgeProbe) -> AtelierResult<BridgeProbe> {
        if new.node_class_id.trim().is_empty() {
            return Err(AtelierError::Validation(
                "node_class_id must not be empty".into(),
            ));
        }
        if !new.probe_outcome.is_bridge_present()
            && new
                .fallback_reason
                .as_deref()
                .map(str::trim)
                .unwrap_or("")
                .is_empty()
        {
            return Err(AtelierError::Validation(format!(
                "fallback_reason is required when probe outcome is {}",
                new.probe_outcome.as_token()
            )));
        }
        let detected = new.probe_outcome.is_bridge_present();
        let node_instance_ids = serde_json::to_value(&new.node_instance_ids)
            .map_err(|e| AtelierError::Validation(format!("node_instance_ids: {e}")))?;

        let row = sqlx::query(
            r#"INSERT INTO atelier_comfy_bridge_probe
                 (workflow_run_id, node_class_id, detected, bridge_protocol_version,
                  node_instance_ids, probe_outcome, fallback_reason)
               VALUES ($1, $2, $3, $4, $5, $6, $7)
               ON CONFLICT (workflow_run_id) DO UPDATE
                 SET node_class_id           = EXCLUDED.node_class_id,
                     detected                = EXCLUDED.detected,
                     bridge_protocol_version = EXCLUDED.bridge_protocol_version,
                     node_instance_ids       = EXCLUDED.node_instance_ids,
                     probe_outcome           = EXCLUDED.probe_outcome,
                     fallback_reason         = EXCLUDED.fallback_reason
               RETURNING probe_id, workflow_run_id, node_class_id, detected,
                         bridge_protocol_version, node_instance_ids, probe_outcome,
                         fallback_reason, probed_at_utc"#,
        )
        .bind(new.workflow_run_id)
        .bind(&new.node_class_id)
        .bind(detected)
        .bind(&new.bridge_protocol_version)
        .bind(node_instance_ids)
        .bind(new.probe_outcome.as_token())
        .bind(&new.fallback_reason)
        .fetch_one(self.pool())
        .await?;
        let probe = probe_from_row(&row)?;

        self.record_event(
            PROBE_RECORDED,
            "atelier_comfy_bridge_probe",
            &probe.workflow_run_id.to_string(),
            serde_json::json!({
                "probe_id": probe.probe_id,
                "node_class_id": probe.node_class_id,
                "probe_outcome": probe.probe_outcome.as_token(),
                "fallback_reason": probe.fallback_reason,
            }),
        )
        .await?;
        Ok(probe)
    }

    /// Fetch the single probe for a job, if one was recorded.
    pub async fn get_bridge_probe(
        &self,
        workflow_run_id: Uuid,
    ) -> AtelierResult<Option<BridgeProbe>> {
        let row = sqlx::query(
            r#"SELECT probe_id, workflow_run_id, node_class_id, detected,
                      bridge_protocol_version, node_instance_ids, probe_outcome,
                      fallback_reason, probed_at_utc
               FROM atelier_comfy_bridge_probe WHERE workflow_run_id = $1"#,
        )
        .bind(workflow_run_id)
        .fetch_optional(self.pool())
        .await?;
        match row {
            Some(r) => Ok(Some(probe_from_row(&r)?)),
            None => Ok(None),
        }
    }

    /// Register the bridge capability for a job (Section 6.9.3).
    ///
    /// Exactly one registration per job (idempotent on `workflow_run_id`):
    /// re-registering the same pinned graph + profile rebuilds the same shape
    /// (replay-stable, modulo ids/timestamps). Accepted declared outputs and
    /// capability-rejected outputs are written atomically inside one
    /// transaction. Each accepted output is an ordered child row; each rejected
    /// output is a typed reject row and emits a `CAPABILITY_REJECTED` event and
    /// MUST NOT be routed. Emits `CAPABILITY_REGISTERED`.
    pub async fn register_bridge_capability(
        &self,
        new: &NewCapabilityRegistration,
    ) -> AtelierResult<CapabilityRegistration> {
        if new.node_class_id.trim().is_empty() {
            return Err(AtelierError::Validation(
                "node_class_id must not be empty".into(),
            ));
        }
        if new.capability_grant_ref.trim().is_empty() {
            return Err(AtelierError::Validation(
                "capability_grant_ref must not be empty".into(),
            ));
        }
        validate_engine_comfyui_grant_ref(&new.capability_grant_ref)?;

        let mut tx = self.pool().begin().await?;

        // One registration per run. Re-registration replaces the child rows so
        // the shape is replay-stable rather than accreting duplicates.
        let reg_row = sqlx::query(
            r#"INSERT INTO atelier_comfy_capability_registration
                 (workflow_run_id, node_class_id, bridge_protocol_version,
                  capability_grant_ref, consent_decision_ref)
               VALUES ($1, $2, $3, $4, $5)
               ON CONFLICT (workflow_run_id) DO UPDATE
                 SET node_class_id           = EXCLUDED.node_class_id,
                     bridge_protocol_version = EXCLUDED.bridge_protocol_version,
                     capability_grant_ref    = EXCLUDED.capability_grant_ref,
                     consent_decision_ref    = EXCLUDED.consent_decision_ref,
                     registered_at_utc       = NOW()
               RETURNING registration_id, workflow_run_id, node_class_id,
                         bridge_protocol_version, capability_grant_ref,
                         consent_decision_ref, registered_at_utc"#,
        )
        .bind(new.workflow_run_id)
        .bind(&new.node_class_id)
        .bind(&new.bridge_protocol_version)
        .bind(&new.capability_grant_ref)
        .bind(&new.consent_decision_ref)
        .fetch_one(&mut *tx)
        .await?;

        let registration_id: Uuid = reg_row.get("registration_id");

        // Replace child rows for replay-stability on re-registration.
        sqlx::query("DELETE FROM atelier_comfy_declared_output WHERE registration_id = $1")
            .bind(registration_id)
            .execute(&mut *tx)
            .await?;
        sqlx::query("DELETE FROM atelier_comfy_capability_reject WHERE registration_id = $1")
            .bind(registration_id)
            .execute(&mut *tx)
            .await?;

        for (seq, out) in new.accepted_outputs.iter().enumerate() {
            if out.output_slot.trim().is_empty() {
                return Err(AtelierError::Validation(
                    "declared output_slot must not be empty".into(),
                ));
            }
            sqlx::query(
                r#"INSERT INTO atelier_comfy_declared_output
                     (registration_id, seq, output_slot, media_kind, expected_mime, routing_intent)
                   VALUES ($1, $2, $3, $4, $5, $6)"#,
            )
            .bind(registration_id)
            .bind(seq as i64)
            .bind(&out.output_slot)
            .bind(out.media_kind.as_token())
            .bind(&out.expected_mime)
            .bind(out.routing_intent.as_token())
            .execute(&mut *tx)
            .await?;
        }

        for (seq, (slot, reason)) in new.rejected_outputs.iter().enumerate() {
            sqlx::query(
                r#"INSERT INTO atelier_comfy_capability_reject
                     (registration_id, seq, output_slot, reason)
                   VALUES ($1, $2, $3, $4)"#,
            )
            .bind(registration_id)
            .bind(seq as i64)
            .bind(slot)
            .bind(reason)
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;

        // One reject event per dropped output (Section 6.9.5).
        for (slot, reason) in &new.rejected_outputs {
            self.record_event(
                CAPABILITY_REJECTED,
                "atelier_comfy_capability_registration",
                &registration_id.to_string(),
                serde_json::json!({
                    "registration_id": registration_id,
                    "output_slot": slot,
                    "reason": reason,
                }),
            )
            .await?;
        }

        self.record_event(
            CAPABILITY_REGISTERED,
            "atelier_comfy_capability_registration",
            &registration_id.to_string(),
            serde_json::json!({
                "registration_id": registration_id,
                "workflow_run_id": new.workflow_run_id,
                "declared_output_count": new.accepted_outputs.len(),
                "reject_count": new.rejected_outputs.len(),
            }),
        )
        .await?;

        self.get_capability_registration(new.workflow_run_id)
            .await?
            .ok_or_else(|| {
                AtelierError::NotFound(format!(
                    "capability registration for run {}",
                    new.workflow_run_id
                ))
            })
    }

    /// Fetch the capability registration (with its ordered accepted outputs) for
    /// a job, if one was recorded.
    pub async fn get_capability_registration(
        &self,
        workflow_run_id: Uuid,
    ) -> AtelierResult<Option<CapabilityRegistration>> {
        let reg_row = sqlx::query(
            r#"SELECT registration_id, workflow_run_id, node_class_id,
                      bridge_protocol_version, capability_grant_ref,
                      consent_decision_ref, registered_at_utc
               FROM atelier_comfy_capability_registration
               WHERE workflow_run_id = $1"#,
        )
        .bind(workflow_run_id)
        .fetch_optional(self.pool())
        .await?;
        let reg_row = match reg_row {
            Some(r) => r,
            None => return Ok(None),
        };
        let registration_id: Uuid = reg_row.get("registration_id");

        let out_rows = sqlx::query(
            r#"SELECT output_slot, media_kind, expected_mime, routing_intent
               FROM atelier_comfy_declared_output
               WHERE registration_id = $1
               ORDER BY seq ASC"#,
        )
        .bind(registration_id)
        .fetch_all(self.pool())
        .await?;
        let mut declared_outputs = Vec::with_capacity(out_rows.len());
        for r in &out_rows {
            declared_outputs.push(declared_output_from_row(r)?);
        }

        Ok(Some(CapabilityRegistration {
            registration_id,
            workflow_run_id: reg_row.get("workflow_run_id"),
            node_class_id: reg_row.get("node_class_id"),
            bridge_protocol_version: reg_row.get("bridge_protocol_version"),
            declared_outputs,
            capability_grant_ref: reg_row.get("capability_grant_ref"),
            consent_decision_ref: reg_row.get("consent_decision_ref"),
            registered_at_utc: reg_row.get("registered_at_utc"),
        }))
    }

    /// The capability-rejected outputs for a job's registration (Section 6.9.3).
    /// Returns `(output_slot, reason)` pairs in declaration order.
    pub async fn list_capability_rejects(
        &self,
        workflow_run_id: Uuid,
    ) -> AtelierResult<Vec<(String, String)>> {
        let rows = sqlx::query(
            r#"SELECT r.output_slot AS output_slot, r.reason AS reason
               FROM atelier_comfy_capability_reject r
               JOIN atelier_comfy_capability_registration g
                 ON g.registration_id = r.registration_id
               WHERE g.workflow_run_id = $1
               ORDER BY r.seq ASC"#,
        )
        .bind(workflow_run_id)
        .fetch_all(self.pool())
        .await?;
        Ok(rows
            .iter()
            .map(|r| {
                let slot: String = r.get("output_slot");
                let reason: String = r.get("reason");
                (slot, reason)
            })
            .collect())
    }

    /// Record one governed intake output, materialized through the ArtifactStore
    /// by the engine (Section 6.9.4). Works for both bridge-routed and
    /// SaveImage-fallback outputs.
    ///
    /// Idempotent on `(workflow_run_id, content_hash)`: re-delivery of the same
    /// output (e.g. on job retry) resolves to the existing `artifact_ref` and is
    /// recorded as dedup evidence via `OUTPUT_DEDUPLICATED` rather than creating
    /// a duplicate row (Section 6.9.4 / 6.9.5). A fresh materialization emits
    /// `OUTPUT_MATERIALIZED`. `artifact_ref` and `artifact_manifest_ref` must be
    /// portable ArtifactStore handles, never machine-local paths (validated as
    /// non-empty here; portability is the engine's contract). Any caller-supplied
    /// provenance must already be scrubbed; this method never persists raw bytes
    /// or credential material.
    pub async fn record_intake_output(
        &self,
        new: &NewIntakeOutput,
    ) -> AtelierResult<RecordOutputOutcome> {
        if new.artifact_ref.trim().is_empty() {
            return Err(AtelierError::Validation(
                "artifact_ref must not be empty".into(),
            ));
        }
        reject_legacy_runtime_ref("artifact_ref", &new.artifact_ref)?;
        if new.artifact_manifest_ref.trim().is_empty() {
            return Err(AtelierError::Validation(
                "artifact_manifest_ref must not be empty".into(),
            ));
        }
        reject_legacy_runtime_ref("artifact_manifest_ref", &new.artifact_manifest_ref)?;
        if let Some(parent_artifact_ref) = &new.parent_artifact_ref {
            reject_legacy_runtime_ref("parent_artifact_ref", parent_artifact_ref)?;
        }
        if let Some(prompt_json_ref) = &new.prompt_json_ref {
            reject_legacy_runtime_ref("prompt_json_ref", prompt_json_ref)?;
        }
        if new.content_hash.trim().is_empty() {
            return Err(AtelierError::Validation(
                "content_hash must not be empty".into(),
            ));
        }
        let workflow_input_metadata =
            workflow_input_metadata_from_identity(new.identity_metadata.as_ref())?;
        // Sidecars must bind to a primary; primaries must not (Section 6.9.4).
        match new.routing_intent {
            RoutingIntent::Sidecar => {
                if new
                    .parent_artifact_ref
                    .as_deref()
                    .map(str::trim)
                    .unwrap_or("")
                    .is_empty()
                {
                    return Err(AtelierError::Validation(
                        "sidecar output requires a parent_artifact_ref".into(),
                    ));
                }
            }
            RoutingIntent::Artifact | RoutingIntent::Transient => {}
        }

        // Idempotent fast path: an existing output with this content hash for the
        // run is a dedup hit; return it and record dedup evidence.
        if let Some(existing) = self
            .get_intake_output_by_hash(new.workflow_run_id, &new.content_hash)
            .await?
        {
            self.record_event(
                OUTPUT_DEDUPLICATED,
                "atelier_comfy_intake_output",
                &existing.intake_output_id.to_string(),
                serde_json::json!({
                    "workflow_run_id": existing.workflow_run_id,
                    "content_hash": existing.content_hash,
                    "artifact_ref": existing.artifact_ref,
                }),
            )
            .await?;
            return Ok(RecordOutputOutcome {
                output: existing,
                deduplicated: true,
            });
        }

        let row = sqlx::query(
            r#"INSERT INTO atelier_comfy_intake_output
                 (workflow_run_id, node_execution_id, registration_id,
                  source_node_instance_id, source_output_slot, media_kind, mime,
                  artifact_ref, artifact_manifest_ref, content_hash, routing_intent,
                  parent_artifact_ref, prompt_json_ref, graph_hash, seed,
                  workflow_input_metadata)
               VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16)
               ON CONFLICT (workflow_run_id, content_hash) DO UPDATE
                 SET artifact_ref = EXCLUDED.artifact_ref
               RETURNING intake_output_id, workflow_run_id, node_execution_id,
                         registration_id, source_node_instance_id, source_output_slot,
                         media_kind, mime, artifact_ref, artifact_manifest_ref,
                         content_hash, routing_intent, parent_artifact_ref,
                         prompt_json_ref, graph_hash, seed, workflow_input_metadata,
                         materialized_at_utc"#,
        )
        .bind(new.workflow_run_id)
        .bind(&new.node_execution_id)
        .bind(new.registration_id)
        .bind(&new.source_node_instance_id)
        .bind(&new.source_output_slot)
        .bind(new.media_kind.as_token())
        .bind(&new.mime)
        .bind(&new.artifact_ref)
        .bind(&new.artifact_manifest_ref)
        .bind(&new.content_hash)
        .bind(new.routing_intent.as_token())
        .bind(&new.parent_artifact_ref)
        .bind(&new.prompt_json_ref)
        .bind(&new.graph_hash)
        .bind(new.seed)
        .bind(&workflow_input_metadata)
        .fetch_one(self.pool())
        .await?;
        let output = output_from_row(&row)?;

        self.record_event(
            OUTPUT_MATERIALIZED,
            "atelier_comfy_intake_output",
            &output.intake_output_id.to_string(),
            serde_json::json!({
                "workflow_run_id": output.workflow_run_id,
                "artifact_ref": output.artifact_ref,
                "artifact_manifest_ref": output.artifact_manifest_ref,
                "routing_intent": output.routing_intent.as_token(),
                "source_output_slot": output.source_output_slot,
                "graph_hash": output.graph_hash,
                "seed": output.seed,
                "workflow_input_metadata": output.workflow_input_metadata,
            }),
        )
        .await?;
        Ok(RecordOutputOutcome {
            output,
            deduplicated: false,
        })
    }

    /// Fetch an intake output by its dedup key `(workflow_run_id, content_hash)`.
    pub async fn get_intake_output_by_hash(
        &self,
        workflow_run_id: Uuid,
        content_hash: &str,
    ) -> AtelierResult<Option<IntakeOutput>> {
        let row = sqlx::query(
            r#"SELECT intake_output_id, workflow_run_id, node_execution_id,
                      registration_id, source_node_instance_id, source_output_slot,
                      media_kind, mime, artifact_ref, artifact_manifest_ref,
                      content_hash, routing_intent, parent_artifact_ref,
                      prompt_json_ref, graph_hash, seed, workflow_input_metadata,
                      materialized_at_utc
               FROM atelier_comfy_intake_output
               WHERE workflow_run_id = $1 AND content_hash = $2"#,
        )
        .bind(workflow_run_id)
        .bind(content_hash)
        .fetch_optional(self.pool())
        .await?;
        match row {
            Some(r) => Ok(Some(output_from_row(&r)?)),
            None => Ok(None),
        }
    }

    /// List all intake outputs for a job in materialization order.
    pub async fn list_intake_outputs(
        &self,
        workflow_run_id: Uuid,
    ) -> AtelierResult<Vec<IntakeOutput>> {
        let rows = sqlx::query(
            r#"SELECT intake_output_id, workflow_run_id, node_execution_id,
                      registration_id, source_node_instance_id, source_output_slot,
                      media_kind, mime, artifact_ref, artifact_manifest_ref,
                      content_hash, routing_intent, parent_artifact_ref,
                      prompt_json_ref, graph_hash, seed, workflow_input_metadata,
                      materialized_at_utc
               FROM atelier_comfy_intake_output
               WHERE workflow_run_id = $1
               ORDER BY materialized_at_utc ASC, intake_output_id ASC"#,
        )
        .bind(workflow_run_id)
        .fetch_all(self.pool())
        .await?;
        rows.iter().map(output_from_row).collect()
    }

    /// Mark that intake degraded to the SaveImage scan fallback for a job
    /// (Section 6.9.6). Idempotent on `workflow_run_id` (one fallback marker per
    /// run); records the `fallback_reason` so the operator sees why the bridge
    /// path was not taken. Emits `FALLBACK_ENGAGED`.
    pub async fn mark_saveimage_fallback(
        &self,
        workflow_run_id: Uuid,
        fallback_reason: &str,
    ) -> AtelierResult<()> {
        if fallback_reason.trim().is_empty() {
            return Err(AtelierError::Validation(
                "fallback_reason must not be empty".into(),
            ));
        }
        sqlx::query(
            r#"INSERT INTO atelier_comfy_fallback_marker
                 (workflow_run_id, fallback_reason)
               VALUES ($1, $2)
               ON CONFLICT (workflow_run_id) DO UPDATE
                 SET fallback_reason = EXCLUDED.fallback_reason,
                     engaged_at_utc  = NOW()"#,
        )
        .bind(workflow_run_id)
        .bind(fallback_reason)
        .execute(self.pool())
        .await?;

        self.record_event(
            FALLBACK_ENGAGED,
            "atelier_comfy_fallback_marker",
            &workflow_run_id.to_string(),
            serde_json::json!({
                "workflow_run_id": workflow_run_id,
                "fallback_reason": fallback_reason,
            }),
        )
        .await?;
        Ok(())
    }

    /// Whether a job engaged the SaveImage fallback, and why.
    pub async fn get_saveimage_fallback(
        &self,
        workflow_run_id: Uuid,
    ) -> AtelierResult<Option<String>> {
        let reason: Option<String> = sqlx::query_scalar(
            "SELECT fallback_reason FROM atelier_comfy_fallback_marker WHERE workflow_run_id = $1",
        )
        .bind(workflow_run_id)
        .fetch_optional(self.pool())
        .await?;
        Ok(reason)
    }

    /// Preserve a generated output when registration fails after the image was
    /// saved. This records retryable evidence without creating a successful
    /// intake output row.
    pub async fn record_comfy_output_registration_failure(
        &self,
        new: &NewComfyOutputRegistrationFailure,
    ) -> AtelierResult<ComfyOutputRegistrationFailure> {
        if new.node_execution_id.trim().is_empty()
            || new.source_node_instance_id.trim().is_empty()
            || new.source_output_slot.trim().is_empty()
            || new.mime.trim().is_empty()
        {
            return Err(AtelierError::Validation(
                "node, output slot, and mime fields must not be empty".into(),
            ));
        }
        validate_registration_failure_shape(
            &new.artifact_ref,
            &new.artifact_manifest_ref,
            &new.content_hash,
            new.parent_artifact_ref.as_ref(),
            new.prompt_json_ref.as_ref(),
            new.routing_intent,
            &new.failure_stage,
            &new.failure_reason,
            &new.evidence,
        )?;
        let workflow_input_metadata =
            workflow_input_metadata_from_identity(new.identity_metadata.as_ref())?;
        let evidence = scrub_provenance(&new.evidence);

        let row = sqlx::query(
            r#"INSERT INTO atelier_comfy_output_registration_failure
                 (workflow_run_id, node_execution_id, attempted_registration_id,
                  source_node_instance_id, source_output_slot, media_kind, mime,
                  artifact_ref, artifact_manifest_ref, content_hash, routing_intent,
                  parent_artifact_ref, prompt_json_ref, graph_hash, seed,
                  workflow_input_metadata, failure_stage, failure_reason, evidence)
               VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12,
                       $13, $14, $15, $16, $17, $18, $19)
               ON CONFLICT (workflow_run_id, content_hash, failure_stage) DO UPDATE
                 SET node_execution_id = EXCLUDED.node_execution_id,
                     attempted_registration_id = EXCLUDED.attempted_registration_id,
                     source_node_instance_id = EXCLUDED.source_node_instance_id,
                     source_output_slot = EXCLUDED.source_output_slot,
                     media_kind = EXCLUDED.media_kind,
                     mime = EXCLUDED.mime,
                     artifact_ref = EXCLUDED.artifact_ref,
                     artifact_manifest_ref = EXCLUDED.artifact_manifest_ref,
                     routing_intent = EXCLUDED.routing_intent,
                     parent_artifact_ref = EXCLUDED.parent_artifact_ref,
                     prompt_json_ref = EXCLUDED.prompt_json_ref,
                     graph_hash = EXCLUDED.graph_hash,
                     seed = EXCLUDED.seed,
                     workflow_input_metadata = EXCLUDED.workflow_input_metadata,
                     failure_reason = EXCLUDED.failure_reason,
                     evidence = EXCLUDED.evidence,
                     updated_at_utc = NOW()
               RETURNING failure_id, workflow_run_id, node_execution_id,
                         attempted_registration_id, source_node_instance_id,
                         source_output_slot, media_kind, mime, artifact_ref,
                         artifact_manifest_ref, content_hash, routing_intent,
                         parent_artifact_ref, prompt_json_ref, graph_hash, seed,
                         workflow_input_metadata, failure_stage, failure_reason,
                         evidence, status, retry_count, resolved_intake_output_id,
                         created_at_utc, updated_at_utc"#,
        )
        .bind(new.workflow_run_id)
        .bind(&new.node_execution_id)
        .bind(new.attempted_registration_id)
        .bind(&new.source_node_instance_id)
        .bind(&new.source_output_slot)
        .bind(new.media_kind.as_token())
        .bind(&new.mime)
        .bind(&new.artifact_ref)
        .bind(&new.artifact_manifest_ref)
        .bind(&new.content_hash)
        .bind(new.routing_intent.as_token())
        .bind(&new.parent_artifact_ref)
        .bind(&new.prompt_json_ref)
        .bind(&new.graph_hash)
        .bind(new.seed)
        .bind(&workflow_input_metadata)
        .bind(&new.failure_stage)
        .bind(&new.failure_reason)
        .bind(&evidence)
        .fetch_one(self.pool())
        .await?;
        let failure = output_registration_failure_from_row(&row)?;

        self.record_event(
            OUTPUT_REGISTRATION_FAILURE_RECORDED,
            "atelier_comfy_output_registration_failure",
            &failure.failure_id.to_string(),
            serde_json::json!({
                "failure_id": failure.failure_id,
                "workflow_run_id": failure.workflow_run_id,
                "artifact_ref": failure.artifact_ref,
                "artifact_manifest_ref": failure.artifact_manifest_ref,
                "content_hash": failure.content_hash,
                "failure_stage": failure.failure_stage,
                "status": failure.status.as_token(),
            }),
        )
        .await?;
        Ok(failure)
    }

    pub async fn get_comfy_output_registration_failure(
        &self,
        failure_id: Uuid,
    ) -> AtelierResult<Option<ComfyOutputRegistrationFailure>> {
        let row = sqlx::query(
            r#"SELECT failure_id, workflow_run_id, node_execution_id,
                      attempted_registration_id, source_node_instance_id,
                      source_output_slot, media_kind, mime, artifact_ref,
                      artifact_manifest_ref, content_hash, routing_intent,
                      parent_artifact_ref, prompt_json_ref, graph_hash, seed,
                      workflow_input_metadata, failure_stage, failure_reason,
                      evidence, status, retry_count, resolved_intake_output_id,
                      created_at_utc, updated_at_utc
               FROM atelier_comfy_output_registration_failure
               WHERE failure_id = $1"#,
        )
        .bind(failure_id)
        .fetch_optional(self.pool())
        .await?;
        match row {
            Some(row) => Ok(Some(output_registration_failure_from_row(&row)?)),
            None => Ok(None),
        }
    }

    pub async fn list_comfy_output_registration_failures(
        &self,
        workflow_run_id: Uuid,
    ) -> AtelierResult<Vec<ComfyOutputRegistrationFailure>> {
        let rows = sqlx::query(
            r#"SELECT failure_id, workflow_run_id, node_execution_id,
                      attempted_registration_id, source_node_instance_id,
                      source_output_slot, media_kind, mime, artifact_ref,
                      artifact_manifest_ref, content_hash, routing_intent,
                      parent_artifact_ref, prompt_json_ref, graph_hash, seed,
                      workflow_input_metadata, failure_stage, failure_reason,
                      evidence, status, retry_count, resolved_intake_output_id,
                      created_at_utc, updated_at_utc
               FROM atelier_comfy_output_registration_failure
               WHERE workflow_run_id = $1
               ORDER BY created_at_utc ASC, failure_id ASC"#,
        )
        .bind(workflow_run_id)
        .fetch_all(self.pool())
        .await?;
        rows.iter()
            .map(output_registration_failure_from_row)
            .collect()
    }

    /// Retry a preserved output-first registration failure into the normal
    /// intake output table, then mark the failure evidence as resolved.
    pub async fn retry_comfy_output_registration_failure(
        &self,
        failure_id: Uuid,
        registration_id: Option<Uuid>,
    ) -> AtelierResult<RecordOutputOutcome> {
        let failure = self
            .get_comfy_output_registration_failure(failure_id)
            .await?
            .ok_or_else(|| {
                AtelierError::NotFound(format!("comfy output registration failure {failure_id}"))
            })?;
        if !matches!(
            failure.status,
            ComfyOutputRegistrationFailureStatus::Retryable
        ) {
            return Err(AtelierError::Validation(format!(
                "output registration failure {failure_id} is not retryable"
            )));
        }

        let retry = NewIntakeOutput {
            workflow_run_id: failure.workflow_run_id,
            node_execution_id: failure.node_execution_id.clone(),
            registration_id,
            source_node_instance_id: failure.source_node_instance_id.clone(),
            source_output_slot: failure.source_output_slot.clone(),
            media_kind: failure.media_kind,
            mime: failure.mime.clone(),
            artifact_ref: failure.artifact_ref.clone(),
            artifact_manifest_ref: failure.artifact_manifest_ref.clone(),
            content_hash: failure.content_hash.clone(),
            routing_intent: failure.routing_intent,
            parent_artifact_ref: failure.parent_artifact_ref.clone(),
            prompt_json_ref: failure.prompt_json_ref.clone(),
            graph_hash: failure.graph_hash.clone(),
            seed: failure.seed,
            identity_metadata: None,
        };
        let outcome = self.record_intake_output(&retry).await?;

        sqlx::query(
            r#"UPDATE atelier_comfy_output_registration_failure
               SET status = 'registered',
                   retry_count = retry_count + 1,
                   resolved_intake_output_id = $2,
                   updated_at_utc = NOW()
               WHERE failure_id = $1"#,
        )
        .bind(failure_id)
        .bind(outcome.output.intake_output_id)
        .execute(self.pool())
        .await?;

        self.record_event(
            OUTPUT_REGISTRATION_FAILURE_RETRIED,
            "atelier_comfy_output_registration_failure",
            &failure_id.to_string(),
            serde_json::json!({
                "failure_id": failure_id,
                "workflow_run_id": outcome.output.workflow_run_id,
                "intake_output_id": outcome.output.intake_output_id,
                "registration_id": registration_id,
                "deduplicated": outcome.deduplicated,
            }),
        )
        .await?;
        Ok(outcome)
    }

    /// Produce the per-job intake receipt (`ComfyIntakeReceiptV1`, Section
    /// 6.9.5) by summarizing the recorded probe, registration, and outputs. The
    /// receipt is the recoverable artifact that lets a no-context model
    /// reconstruct exactly what intake produced for the job. Emits
    /// `RECEIPT_PRODUCED`. All refs are by-reference; no bytes or credentials are
    /// included.
    pub async fn produce_intake_receipt(
        &self,
        workflow_run_id: Uuid,
    ) -> AtelierResult<IntakeReceipt> {
        let probe_outcome = self
            .get_bridge_probe(workflow_run_id)
            .await?
            .map(|p| p.probe_outcome);

        let registered_output_count: i64 = sqlx::query_scalar(
            r#"SELECT COUNT(*)
               FROM atelier_comfy_declared_output d
               JOIN atelier_comfy_capability_registration g
                 ON g.registration_id = d.registration_id
               WHERE g.workflow_run_id = $1"#,
        )
        .bind(workflow_run_id)
        .fetch_one(self.pool())
        .await?;

        let output_rows = sqlx::query(
            r#"SELECT artifact_ref, workflow_input_metadata
               FROM atelier_comfy_intake_output
               WHERE workflow_run_id = $1
               ORDER BY materialized_at_utc ASC, intake_output_id ASC"#,
        )
        .bind(workflow_run_id)
        .fetch_all(self.pool())
        .await?;
        let mut materialized_artifact_refs = Vec::with_capacity(output_rows.len());
        let mut workflow_inputs = Vec::with_capacity(output_rows.len());
        for row in output_rows {
            let artifact_ref: String = row.get("artifact_ref");
            let workflow_input_metadata: serde_json::Value = row.get("workflow_input_metadata");
            let identity = workflow_input_metadata
                .get("identity")
                .cloned()
                .unwrap_or_else(|| serde_json::json!({}));
            materialized_artifact_refs.push(artifact_ref.clone());
            workflow_inputs.push(serde_json::json!({
                "artifact_ref": artifact_ref,
                "identity": identity,
            }));
        }

        let fallback_engaged = self
            .get_saveimage_fallback(workflow_run_id)
            .await?
            .is_some();

        let receipt = IntakeReceipt {
            workflow_run_id,
            probe_outcome,
            registered_output_count,
            materialized_artifact_refs,
            workflow_inputs,
            fallback_engaged,
        };

        self.record_event(
            RECEIPT_PRODUCED,
            "atelier_comfy_intake_receipt",
            &workflow_run_id.to_string(),
            serde_json::json!({
                "workflow_run_id": receipt.workflow_run_id,
                "probe_outcome": receipt.probe_outcome.map(|o| o.as_token()),
                "registered_output_count": receipt.registered_output_count,
                "materialized_artifact_count": receipt.materialized_artifact_refs.len(),
                "workflow_input_count": receipt.workflow_inputs.len(),
                "fallback_engaged": receipt.fallback_engaged,
            }),
        )
        .await?;
        Ok(receipt)
    }

    /// Record the durable workflow-level ComfyUI receipt (MT-101).
    pub async fn record_comfy_workflow_receipt(
        &self,
        new: &NewComfyWorkflowReceipt,
    ) -> AtelierResult<ComfyWorkflowReceipt> {
        validate_workflow_system_id(&new.system_id)?;
        reject_legacy_runtime_ref("workflow_spec_ref", &new.workflow_spec_ref)?;
        reject_legacy_runtime_ref("workflow_json_ref", &new.workflow_json_ref)?;
        reject_legacy_runtime_ref("prompt_ref", &new.prompt_ref)?;
        if let Some(error_ref) = &new.error_ref {
            reject_legacy_runtime_ref("error_ref", error_ref)?;
        }
        if matches!(new.status, ComfyWorkflowStatus::Failed)
            && new
                .error_ref
                .as_deref()
                .map(str::trim)
                .unwrap_or("")
                .is_empty()
        {
            return Err(AtelierError::Validation(
                "failed workflow receipt requires error_ref".into(),
            ));
        }
        if !new.evidence.is_object() {
            return Err(AtelierError::Validation(
                "workflow receipt evidence must be a JSON object".into(),
            ));
        }

        let output_rows = sqlx::query(
            r#"SELECT intake_output_id, workflow_run_id, node_execution_id,
                      registration_id, source_node_instance_id, source_output_slot,
                      media_kind, mime, artifact_ref, artifact_manifest_ref,
                      content_hash, routing_intent, parent_artifact_ref,
                      prompt_json_ref, graph_hash, seed, workflow_input_metadata,
                      materialized_at_utc
               FROM atelier_comfy_intake_output
               WHERE workflow_run_id = $1
               ORDER BY materialized_at_utc ASC, intake_output_id ASC"#,
        )
        .bind(new.workflow_run_id)
        .fetch_all(self.pool())
        .await?;
        let outputs: Vec<serde_json::Value> = output_rows
            .iter()
            .map(workflow_receipt_output_from_row)
            .collect();
        let all_refs = workflow_receipt_refs(new, &outputs);
        let evidence = scrub_provenance(&new.evidence);
        let character_ref = workflow_receipt_character_ref(&evidence)?;
        let receipt_json = serde_json::json!({
            "schema": COMFY_WORKFLOW_RECEIPT_SCHEMA,
            "system_id": &new.system_id,
            "workflow_run_id": new.workflow_run_id,
            "character_ref": &character_ref,
            "refs": all_refs.clone(),
            "outputs": outputs.clone(),
            "status": new.status.as_token(),
            "error_ref": &new.error_ref,
            "evidence": evidence.clone(),
        });

        let row = sqlx::query(
            r#"INSERT INTO atelier_comfy_workflow_receipt
                 (system_id, workflow_run_id, character_ref, workflow_spec_ref,
                  workflow_json_ref, prompt_ref, all_refs, outputs, status,
                  error_ref, evidence, receipt_json)
               VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
               ON CONFLICT (workflow_run_id) DO UPDATE
                 SET system_id = EXCLUDED.system_id,
                     character_ref = EXCLUDED.character_ref,
                     workflow_spec_ref = EXCLUDED.workflow_spec_ref,
                     workflow_json_ref = EXCLUDED.workflow_json_ref,
                     prompt_ref = EXCLUDED.prompt_ref,
                     all_refs = EXCLUDED.all_refs,
                     outputs = EXCLUDED.outputs,
                     status = EXCLUDED.status,
                     error_ref = EXCLUDED.error_ref,
                     evidence = EXCLUDED.evidence,
                     receipt_json = EXCLUDED.receipt_json,
                     updated_at_utc = NOW()
               RETURNING receipt_id, system_id, workflow_run_id, character_ref,
                         workflow_spec_ref, workflow_json_ref, prompt_ref,
                         all_refs, outputs, status, error_ref, evidence,
                         receipt_json, created_at_utc, updated_at_utc"#,
        )
        .bind(&new.system_id)
        .bind(new.workflow_run_id)
        .bind(&character_ref)
        .bind(&new.workflow_spec_ref)
        .bind(&new.workflow_json_ref)
        .bind(&new.prompt_ref)
        .bind(&all_refs)
        .bind(&serde_json::Value::Array(outputs))
        .bind(new.status.as_token())
        .bind(&new.error_ref)
        .bind(&evidence)
        .bind(&receipt_json)
        .fetch_one(self.pool())
        .await?;
        let receipt = workflow_receipt_from_row(&row)?;

        self.record_event(
            WORKFLOW_RECEIPT_RECORDED,
            "atelier_comfy_workflow_receipt",
            &receipt.workflow_run_id.to_string(),
            serde_json::json!({
                "receipt_id": receipt.receipt_id,
                "workflow_run_id": receipt.workflow_run_id,
                "system_id": receipt.system_id,
                "character_ref": receipt.character_ref,
                "status": receipt.status.as_token(),
                "output_count": receipt.outputs.len(),
                "error_ref": receipt.error_ref,
            }),
        )
        .await?;
        Ok(receipt)
    }

    /// Fetch the durable workflow-level ComfyUI receipt by workflow run.
    pub async fn get_comfy_workflow_receipt(
        &self,
        workflow_run_id: Uuid,
    ) -> AtelierResult<Option<ComfyWorkflowReceipt>> {
        let row = sqlx::query(
            r#"SELECT receipt_id, system_id, workflow_run_id, workflow_spec_ref,
                      character_ref, workflow_json_ref, prompt_ref, all_refs,
                      outputs, status, error_ref, evidence, receipt_json,
                      created_at_utc, updated_at_utc
               FROM atelier_comfy_workflow_receipt
               WHERE workflow_run_id = $1"#,
        )
        .bind(workflow_run_id)
        .fetch_optional(self.pool())
        .await?;
        match row {
            Some(row) => Ok(Some(workflow_receipt_from_row(&row)?)),
            None => Ok(None),
        }
    }

    fn validate_comfy_workflow_history_query(
        query: &ComfyWorkflowHistoryQuery,
    ) -> AtelierResult<()> {
        if let Some(character_ref) = &query.character_ref {
            reject_legacy_runtime_ref("character_ref", character_ref)?;
        }
        if let Some(workflow_spec_ref) = &query.workflow_spec_ref {
            reject_legacy_runtime_ref("workflow_spec_ref", workflow_spec_ref)?;
        }
        if let (Some(from_utc), Some(to_utc)) = (query.from_utc, query.to_utc) {
            if from_utc > to_utc {
                return Err(AtelierError::Validation(
                    "workflow history from_utc must be <= to_utc".into(),
                ));
            }
        }
        Ok(())
    }

    fn push_comfy_workflow_history_filters(
        builder: &mut QueryBuilder<'_, sqlx::Postgres>,
        query: &ComfyWorkflowHistoryQuery,
    ) {
        if let Some(character_ref) = &query.character_ref {
            builder.push(" AND character_ref = ");
            builder.push_bind(character_ref.clone());
        }
        if let Some(workflow_spec_ref) = &query.workflow_spec_ref {
            builder.push(" AND workflow_spec_ref = ");
            builder.push_bind(workflow_spec_ref.clone());
        }
        if let Some(status) = query.status {
            builder.push(" AND status = ");
            builder.push_bind(status.as_token());
        }
        if let Some(from_utc) = query.from_utc {
            builder.push(" AND created_at_utc >= ");
            builder.push_bind(from_utc);
        }
        if let Some(to_utc) = query.to_utc {
            builder.push(" AND created_at_utc <= ");
            builder.push_bind(to_utc);
        }
    }

    /// List durable ComfyUI workflow receipts by character, spec, status, and
    /// time range. Failed receipts are included by default.
    pub async fn list_comfy_workflow_history(
        &self,
        query: &ComfyWorkflowHistoryQuery,
    ) -> AtelierResult<Vec<ComfyWorkflowReceipt>> {
        Self::validate_comfy_workflow_history_query(query)?;
        let mut builder = QueryBuilder::<sqlx::Postgres>::new(
            r#"SELECT receipt_id, system_id, workflow_run_id, character_ref,
                      workflow_spec_ref, workflow_json_ref, prompt_ref, all_refs,
                      outputs, status, error_ref, evidence, receipt_json,
                      created_at_utc, updated_at_utc
               FROM atelier_comfy_workflow_receipt
               WHERE TRUE"#,
        );
        Self::push_comfy_workflow_history_filters(&mut builder, query);
        builder.push(" ORDER BY created_at_utc ASC, receipt_id ASC");
        let rows = builder.build().fetch_all(self.pool()).await?;
        rows.iter().map(workflow_receipt_from_row).collect()
    }

    /// Aggregate ComfyUI workflow receipt stats over the same filter contract as
    /// history. This counts failures even when no output rows were materialized.
    pub async fn comfy_workflow_stats(
        &self,
        query: &ComfyWorkflowHistoryQuery,
    ) -> AtelierResult<ComfyWorkflowStats> {
        Self::validate_comfy_workflow_history_query(query)?;
        let mut builder = QueryBuilder::<sqlx::Postgres>::new(
            r#"SELECT status, COUNT(*)::BIGINT AS status_count
               FROM atelier_comfy_workflow_receipt
               WHERE TRUE"#,
        );
        Self::push_comfy_workflow_history_filters(&mut builder, query);
        builder.push(" GROUP BY status ORDER BY status ASC");
        let rows = builder.build().fetch_all(self.pool()).await?;
        let mut status_counts = BTreeMap::new();
        let mut total_count = 0_i64;
        for row in rows {
            let status: String = row.get("status");
            let status_count: i64 = row.get("status_count");
            total_count += status_count;
            status_counts.insert(status, status_count);
        }
        let failure_count = status_counts.get("failed").copied().unwrap_or(0);
        Ok(ComfyWorkflowStats {
            total_count,
            failure_count,
            status_counts,
        })
    }

    /// Resolve every input artifact ref required to replay a workflow run
    /// (MT-104). For each ref this proves: (a) it is a Handshake-native portable
    /// handle (legacy/`.GOV`/SQLite/localhost/machine-local refs are rejected via
    /// [`reject_legacy_runtime_ref`]), and (b) it resolves to a STORED artifact --
    /// an `atelier_comfy_intake_output` row for THIS run carrying that
    /// `artifact_ref`. The workflow identity refs are validated as portable too.
    /// Returns the resolved set; an unresolved or invalid ref is a hard error.
    /// This method performs NO event emission -- callers that want EventLedger
    /// evidence use [`Self::request_replay`] (MT-130).
    pub async fn resolve_replay_inputs(
        &self,
        request: &ReplayRequest,
    ) -> AtelierResult<ResolvedReplayInputs> {
        // Workflow identity refs must be portable Handshake-native handles.
        reject_legacy_runtime_ref("workflow_spec_ref", &request.workflow_spec_ref)?;
        reject_legacy_runtime_ref("workflow_json_ref", &request.workflow_json_ref)?;
        if request.input_artifact_refs.is_empty() {
            return Err(AtelierError::Validation(
                "replay request must declare at least one input artifact ref".into(),
            ));
        }

        let mut resolved = Vec::with_capacity(request.input_artifact_refs.len());
        for artifact_ref in &request.input_artifact_refs {
            // Reject legacy/.GOV/SQLite/localhost/machine-local refs up front so a
            // forbidden ref never reaches a lookup.
            reject_legacy_runtime_ref("replay input artifact_ref", artifact_ref)?;

            // The ref must resolve to a stored artifact for THIS run (containment,
            // Section 6.9.1). Reuse the existing intake-output store as the
            // portable-handle resolution surface.
            let row = sqlx::query(
                r#"SELECT intake_output_id, content_hash
                   FROM atelier_comfy_intake_output
                   WHERE workflow_run_id = $1 AND artifact_ref = $2
                   ORDER BY materialized_at_utc ASC, intake_output_id ASC
                   LIMIT 1"#,
            )
            .bind(request.workflow_run_id)
            .bind(artifact_ref)
            .fetch_optional(self.pool())
            .await?;
            let row = row.ok_or_else(|| {
                AtelierError::Validation(format!(
                    "replay input artifact_ref {artifact_ref} does not resolve to a stored \
                     artifact for workflow run {}",
                    request.workflow_run_id
                ))
            })?;
            resolved.push(ResolvedReplayInput {
                artifact_ref: artifact_ref.clone(),
                intake_output_id: row.get("intake_output_id"),
                content_hash: row.get("content_hash"),
            });
        }

        Ok(ResolvedReplayInputs {
            workflow_run_id: request.workflow_run_id,
            workflow_spec_ref: request.workflow_spec_ref.clone(),
            workflow_json_ref: request.workflow_json_ref.clone(),
            graph_hash: request.graph_hash.clone(),
            seed: request.seed,
            resolved_inputs: resolved,
        })
    }

    /// Request a replay (MT-130): emit `REPLAY_REQUESTED`, resolve all input
    /// refs, and emit `REPLAY_COMPLETED` on success or `REPLAY_FAILED` on
    /// rejection. All events are run-scoped on `workflow_run_id` (the aggregate
    /// id) so replay history is reconstructable per run. Resolution itself is
    /// delegated to [`Self::resolve_replay_inputs`]; this wrapper only adds the
    /// EventLedger evidence seam. Returns the resolved inputs on success and
    /// propagates the resolution error (after emitting `REPLAY_FAILED`) on
    /// failure.
    pub async fn request_replay(
        &self,
        request: &ReplayRequest,
    ) -> AtelierResult<ResolvedReplayInputs> {
        let aggregate_id = request.workflow_run_id.to_string();
        self.record_event(
            comfy_event_family::REPLAY_REQUESTED,
            "atelier_comfy_intake_output",
            &aggregate_id,
            serde_json::json!({
                "workflow_run_id": request.workflow_run_id,
                "workflow_spec_ref": request.workflow_spec_ref,
                "workflow_json_ref": request.workflow_json_ref,
                "input_artifact_ref_count": request.input_artifact_refs.len(),
            }),
        )
        .await?;

        match self.resolve_replay_inputs(request).await {
            Ok(resolved) => {
                self.record_event(
                    comfy_event_family::REPLAY_COMPLETED,
                    "atelier_comfy_intake_output",
                    &aggregate_id,
                    serde_json::json!({
                        "workflow_run_id": request.workflow_run_id,
                        "resolved_input_count": resolved.resolved_inputs.len(),
                    }),
                )
                .await?;
                Ok(resolved)
            }
            Err(err) => {
                self.record_event(
                    comfy_event_family::REPLAY_FAILED,
                    "atelier_comfy_intake_output",
                    &aggregate_id,
                    serde_json::json!({
                        "workflow_run_id": request.workflow_run_id,
                        "reason": err.to_string(),
                    }),
                )
                .await?;
                Err(err)
            }
        }
    }

    /// Register a versioned ComfyUI/pose workflow spec (MT-106).
    ///
    /// Idempotent upsert keyed on `(workflow_kind, spec_version)`: re-registering
    /// the same kind+version refreshes the spec body/handler/pin in place rather
    /// than duplicating. `spec_hash` is additionally unique so a graph hash never
    /// registers under two conflicting identities. Any `source_ref` must be a
    /// portable Handshake-native handle (legacy/`.GOV`/SQLite/localhost/
    /// machine-local refs are rejected via [`reject_legacy_runtime_ref`]). Emits
    /// `WORKFLOW_SPEC_REGISTERED`.
    pub async fn register_workflow_spec(
        &self,
        new: &NewWorkflowSpec,
    ) -> AtelierResult<WorkflowSpec> {
        require_token_field("workflow_kind", &new.workflow_kind)?;
        require_token_field("spec_version", &new.spec_version)?;
        require_token_field("spec_hash", &new.spec_hash)?;
        require_token_field("handler_id", &new.handler_id)?;
        if let Some(compatibility_pin) = &new.compatibility_pin {
            require_token_field("compatibility_pin", compatibility_pin)?;
        }
        if let Some(source_ref) = &new.source_ref {
            reject_legacy_runtime_ref("workflow spec source_ref", source_ref)?;
        }
        if !new.spec_json.is_object() {
            return Err(AtelierError::Validation(
                "workflow spec spec_json must be a JSON object".into(),
            ));
        }

        let row = sqlx::query(
            r#"INSERT INTO atelier_comfy_workflow_spec
                 (workflow_kind, spec_version, spec_hash, handler_id,
                  compatibility_pin, spec_json, source_ref)
               VALUES ($1, $2, $3, $4, $5, $6, $7)
               ON CONFLICT (workflow_kind, spec_version) DO UPDATE
                 SET spec_hash         = EXCLUDED.spec_hash,
                     handler_id        = EXCLUDED.handler_id,
                     compatibility_pin = EXCLUDED.compatibility_pin,
                     spec_json         = EXCLUDED.spec_json,
                     source_ref        = EXCLUDED.source_ref,
                     updated_at_utc    = NOW()
               RETURNING spec_id, workflow_kind, spec_version, spec_hash,
                         handler_id, compatibility_pin, spec_json, source_ref,
                         created_at_utc, updated_at_utc"#,
        )
        .bind(&new.workflow_kind)
        .bind(&new.spec_version)
        .bind(&new.spec_hash)
        .bind(&new.handler_id)
        .bind(&new.compatibility_pin)
        .bind(&new.spec_json)
        .bind(&new.source_ref)
        .fetch_one(self.pool())
        .await?;
        let spec = workflow_spec_from_row(&row);

        self.record_event(
            comfy_event_family::WORKFLOW_SPEC_REGISTERED,
            "atelier_comfy_workflow_spec",
            &spec.spec_id.to_string(),
            serde_json::json!({
                "spec_id": spec.spec_id,
                "workflow_kind": spec.workflow_kind,
                "spec_version": spec.spec_version,
                "spec_hash": spec.spec_hash,
                "handler_id": spec.handler_id,
                "compatibility_pin": spec.compatibility_pin,
            }),
        )
        .await?;
        Ok(spec)
    }

    /// Fetch a registered workflow spec by id.
    pub async fn get_workflow_spec(&self, spec_id: Uuid) -> AtelierResult<Option<WorkflowSpec>> {
        let row = sqlx::query(
            r#"SELECT spec_id, workflow_kind, spec_version, spec_hash, handler_id,
                      compatibility_pin, spec_json, source_ref, created_at_utc,
                      updated_at_utc
               FROM atelier_comfy_workflow_spec WHERE spec_id = $1"#,
        )
        .bind(spec_id)
        .fetch_optional(self.pool())
        .await?;
        Ok(row.as_ref().map(workflow_spec_from_row))
    }

    /// List registered workflow specs, newest first, optionally filtered by kind.
    pub async fn list_workflow_specs(
        &self,
        workflow_kind: Option<&str>,
    ) -> AtelierResult<Vec<WorkflowSpec>> {
        let mut builder = QueryBuilder::<sqlx::Postgres>::new(
            r#"SELECT spec_id, workflow_kind, spec_version, spec_hash, handler_id,
                      compatibility_pin, spec_json, source_ref, created_at_utc,
                      updated_at_utc
               FROM atelier_comfy_workflow_spec WHERE TRUE"#,
        );
        if let Some(workflow_kind) = workflow_kind {
            builder.push(" AND workflow_kind = ");
            builder.push_bind(workflow_kind.to_string());
        }
        builder.push(" ORDER BY created_at_utc DESC, spec_id DESC");
        let rows = builder.build().fetch_all(self.pool()).await?;
        Ok(rows.iter().map(workflow_spec_from_row).collect())
    }

    /// Record (pin) external tool/model version metadata for a workflow run
    /// (MT-110).
    ///
    /// Captures the three named provenance versions -- pose model-asset, image
    /// tool, ComfyUI model -- so a run's provenance is reproducible. Idempotent
    /// on `workflow_run_id` (one metadata row per run): re-recording refreshes
    /// the pinned versions in place. When `spec_id` is set it must reference a
    /// registered workflow spec (FK enforced). Emits `VERSION_METADATA_RECORDED`.
    pub async fn record_version_metadata(
        &self,
        new: &NewComfyVersionMetadata,
    ) -> AtelierResult<ComfyVersionMetadata> {
        require_token_field("pose_model_asset_version", &new.pose_model_asset_version)?;
        require_token_field("image_tool_version", &new.image_tool_version)?;
        require_token_field("comfy_model_version", &new.comfy_model_version)?;
        if !new.preflight_evidence.is_object() {
            return Err(AtelierError::Validation(
                "version metadata preflight_evidence must be a JSON object".into(),
            ));
        }
        let preflight_evidence = scrub_provenance(&new.preflight_evidence);

        let row = sqlx::query(
            r#"INSERT INTO atelier_comfy_version_metadata
                 (workflow_run_id, spec_id, pose_model_asset_version,
                  image_tool_version, comfy_model_version, preflight_evidence)
               VALUES ($1, $2, $3, $4, $5, $6)
               ON CONFLICT (workflow_run_id) DO UPDATE
                 SET spec_id                  = EXCLUDED.spec_id,
                     pose_model_asset_version = EXCLUDED.pose_model_asset_version,
                     image_tool_version       = EXCLUDED.image_tool_version,
                     comfy_model_version      = EXCLUDED.comfy_model_version,
                     preflight_evidence       = EXCLUDED.preflight_evidence,
                     updated_at_utc           = NOW()
               RETURNING version_metadata_id, workflow_run_id, spec_id,
                         pose_model_asset_version, image_tool_version,
                         comfy_model_version, preflight_evidence,
                         created_at_utc, updated_at_utc"#,
        )
        .bind(new.workflow_run_id)
        .bind(new.spec_id)
        .bind(&new.pose_model_asset_version)
        .bind(&new.image_tool_version)
        .bind(&new.comfy_model_version)
        .bind(&preflight_evidence)
        .fetch_one(self.pool())
        .await?;
        let metadata = version_metadata_from_row(&row);

        self.record_event(
            comfy_event_family::VERSION_METADATA_RECORDED,
            "atelier_comfy_version_metadata",
            &metadata.workflow_run_id.to_string(),
            serde_json::json!({
                "version_metadata_id": metadata.version_metadata_id,
                "workflow_run_id": metadata.workflow_run_id,
                "spec_id": metadata.spec_id,
                "pose_model_asset_version": metadata.pose_model_asset_version,
                "image_tool_version": metadata.image_tool_version,
                "comfy_model_version": metadata.comfy_model_version,
            }),
        )
        .await?;
        Ok(metadata)
    }

    /// Fetch the pinned version metadata for a workflow run, if recorded.
    pub async fn get_version_metadata(
        &self,
        workflow_run_id: Uuid,
    ) -> AtelierResult<Option<ComfyVersionMetadata>> {
        let row = sqlx::query(
            r#"SELECT version_metadata_id, workflow_run_id, spec_id,
                      pose_model_asset_version, image_tool_version,
                      comfy_model_version, preflight_evidence, created_at_utc,
                      updated_at_utc
               FROM atelier_comfy_version_metadata WHERE workflow_run_id = $1"#,
        )
        .bind(workflow_run_id)
        .fetch_optional(self.pool())
        .await?;
        Ok(row.as_ref().map(version_metadata_from_row))
    }
}

#[cfg(test)]
mod scrub_tests {
    use super::*;

    #[test]
    fn scrubs_credential_keys_recursively() {
        let raw = serde_json::json!({
            "title": "front yaw",
            "Authorization": "Bearer abc123",
            "metadata": {
                "seed": 42,
                "api_key": "sk-secret",
                "nested": { "cookie": "sid=xyz", "model": "sdxl" }
            },
            "headers": [ { "token": "t" }, { "ok": "v" } ]
        });
        let scrubbed = scrub_provenance(&raw);
        assert_eq!(scrubbed["title"], serde_json::json!("front yaw"));
        assert_eq!(scrubbed["Authorization"], serde_json::json!("[REDACTED]"));
        assert_eq!(scrubbed["metadata"]["seed"], serde_json::json!(42));
        assert_eq!(
            scrubbed["metadata"]["api_key"],
            serde_json::json!("[REDACTED]")
        );
        assert_eq!(
            scrubbed["metadata"]["nested"]["cookie"],
            serde_json::json!("[REDACTED]")
        );
        assert_eq!(
            scrubbed["metadata"]["nested"]["model"],
            serde_json::json!("sdxl")
        );
        assert_eq!(
            scrubbed["headers"][0]["token"],
            serde_json::json!("[REDACTED]")
        );
        assert_eq!(scrubbed["headers"][1]["ok"], serde_json::json!("v"));
    }

    #[test]
    fn probe_outcome_round_trips() {
        for o in [
            ProbeOutcome::BridgePresent,
            ProbeOutcome::BridgeAbsent,
            ProbeOutcome::BridgeIncompatible,
        ] {
            assert_eq!(ProbeOutcome::from_token(o.as_token()).unwrap(), o);
        }
        assert!(ProbeOutcome::from_token("nope").is_err());
    }

    #[test]
    fn routing_and_media_round_trip() {
        for r in [
            RoutingIntent::Artifact,
            RoutingIntent::Sidecar,
            RoutingIntent::Transient,
        ] {
            assert_eq!(RoutingIntent::from_token(r.as_token()).unwrap(), r);
        }
        for m in [
            MediaKind::Image,
            MediaKind::Mask,
            MediaKind::LatentPreview,
            MediaKind::Video,
            MediaKind::SidecarJson,
        ] {
            assert_eq!(MediaKind::from_token(m.as_token()).unwrap(), m);
        }
    }
}
