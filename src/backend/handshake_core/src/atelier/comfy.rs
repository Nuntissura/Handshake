//! ComfyUI custom-node intake -- governed records + receipts (MT-202).
//!
//! Spec authority: master-spec-v02.189 / spec-modules/06-mechanical-integrations.md
//! Section 6.9 "ComfyUI Custom-Node Intake (Normative)" (6.9.1 governed-job
//! containment LAW; 6.9.2 bridge-node presence detection; 6.9.3 capability
//! registration; 6.9.4 output routing into ArtifactStore; 6.9.5 EventLedger +
//! Flight-Recorder evidence; 6.9.6 SaveImage fallback boundary).
//!
//! CKC source (INTENT ONLY, never copied): `comfyui_node/castkit_codex_bridge.py`
//! (the `CastKitCodexBridge` node: IMAGE outputs, `workflow_json`/`prompt`,
//! `seed`/`model`/`sampler`/`cfg`/`steps`, `filename_hint`, `character_id`,
//! and a bearer-token POST). The CKC node executes generation, saves to a
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

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::Row;
use uuid::Uuid;

use super::{event_family, AtelierError, AtelierResult, AtelierStore};

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

    /// All ComfyUI intake event families (parity/coverage helper).
    pub const ALL: &[&str] = &[
        PROBE_RECORDED,
        CAPABILITY_REGISTERED,
        CAPABILITY_REJECTED,
        OUTPUT_MATERIALIZED,
        OUTPUT_DEDUPLICATED,
        FALLBACK_ENGAGED,
        RECEIPT_PRODUCED,
    ];
}

/// Re-export at module root so callers can write `comfy::PROBE_RECORDED`.
pub use comfy_event_family::{
    CAPABILITY_REGISTERED, CAPABILITY_REJECTED, FALLBACK_ENGAGED, OUTPUT_DEDUPLICATED,
    OUTPUT_MATERIALIZED, PROBE_RECORDED, RECEIPT_PRODUCED,
};

/// The slot token used for SaveImage-fallback output rows (Section 6.9.6). A
/// fallback output carries no in-graph slot, so this stable sentinel is stored
/// in `source_output_slot` with a null `registration_id`.
pub const SAVEIMAGE_FALLBACK_SLOT: &str = "saveimage_fallback";

const REDACTED_PLACEHOLDER: &str = "[REDACTED]";

/// Scrub credential-bearing keys from a free-form provenance JSON object before
/// it is persisted or echoed into an event payload (LAW-COMFY-INTAKE-005).
///
/// CKC's node POSTed a bearer token and raw image bytes; Handshake stores
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
                    out.insert(key.clone(), serde_json::Value::String(REDACTED_PLACEHOLDER.into()));
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
    /// ArtifactStore ref to the captured prompt/graph JSON (CKC `workflow_json`).
    pub prompt_json_ref: Option<String>,
    /// Graph hash pin (9.12 determinism).
    pub graph_hash: Option<String>,
    /// Seed pin (9.12 STOCHASTIC determinism).
    pub seed: Option<i64>,
    pub materialized_at_utc: DateTime<Utc>,
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
    pub fallback_engaged: bool,
}

fn probe_from_row(row: &sqlx::postgres::PgRow) -> AtelierResult<BridgeProbe> {
    let outcome: String = row.get("probe_outcome");
    let node_instance_ids: serde_json::Value = row.get("node_instance_ids");
    let node_instance_ids: Vec<String> =
        serde_json::from_value(node_instance_ids).map_err(|e| {
            AtelierError::Validation(format!("invalid node_instance_ids json: {e}"))
        })?;
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
        materialized_at_utc: row.get("materialized_at_utc"),
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
    pub async fn record_bridge_probe(
        &self,
        new: &NewBridgeProbe,
    ) -> AtelierResult<BridgeProbe> {
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
        sqlx::query(
            "DELETE FROM atelier_comfy_declared_output WHERE registration_id = $1",
        )
        .bind(registration_id)
        .execute(&mut *tx)
        .await?;
        sqlx::query(
            "DELETE FROM atelier_comfy_capability_reject WHERE registration_id = $1",
        )
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
            return Err(AtelierError::Validation("artifact_ref must not be empty".into()));
        }
        if new.artifact_manifest_ref.trim().is_empty() {
            return Err(AtelierError::Validation(
                "artifact_manifest_ref must not be empty".into(),
            ));
        }
        if new.content_hash.trim().is_empty() {
            return Err(AtelierError::Validation("content_hash must not be empty".into()));
        }
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
                  parent_artifact_ref, prompt_json_ref, graph_hash, seed)
               VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15)
               ON CONFLICT (workflow_run_id, content_hash) DO UPDATE
                 SET artifact_ref = EXCLUDED.artifact_ref
               RETURNING intake_output_id, workflow_run_id, node_execution_id,
                         registration_id, source_node_instance_id, source_output_slot,
                         media_kind, mime, artifact_ref, artifact_manifest_ref,
                         content_hash, routing_intent, parent_artifact_ref,
                         prompt_json_ref, graph_hash, seed, materialized_at_utc"#,
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
                      prompt_json_ref, graph_hash, seed, materialized_at_utc
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
                      prompt_json_ref, graph_hash, seed, materialized_at_utc
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

        let materialized_artifact_refs: Vec<String> = sqlx::query_scalar(
            r#"SELECT artifact_ref
               FROM atelier_comfy_intake_output
               WHERE workflow_run_id = $1
               ORDER BY materialized_at_utc ASC, intake_output_id ASC"#,
        )
        .bind(workflow_run_id)
        .fetch_all(self.pool())
        .await?;

        let fallback_engaged = self
            .get_saveimage_fallback(workflow_run_id)
            .await?
            .is_some();

        let receipt = IntakeReceipt {
            workflow_run_id,
            probe_outcome,
            registered_output_count,
            materialized_artifact_refs,
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
                "fallback_engaged": receipt.fallback_engaged,
            }),
        )
        .await?;
        Ok(receipt)
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
        assert_eq!(scrubbed["metadata"]["api_key"], serde_json::json!("[REDACTED]"));
        assert_eq!(
            scrubbed["metadata"]["nested"]["cookie"],
            serde_json::json!("[REDACTED]")
        );
        assert_eq!(
            scrubbed["metadata"]["nested"]["model"],
            serde_json::json!("sdxl")
        );
        assert_eq!(scrubbed["headers"][0]["token"], serde_json::json!("[REDACTED]"));
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
        for r in [RoutingIntent::Artifact, RoutingIntent::Sidecar, RoutingIntent::Transient] {
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
