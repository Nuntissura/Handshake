//! MT-165 remediation: production [`TraceSource`] backed by the kernel
//! EventLedger (PostgreSQL) and the real ArtifactStore.
//!
//! Integration validation v2 found that [`TraceSource`] had no production
//! implementation and [`RetrievalTraceExporter`] had zero non-test callers:
//! every proof loaded traces and artifacts from in-memory stubs. This module
//! closes both gaps:
//!
//!  - [`PostgresTraceSource`] persists/loads [`TraceBundle`] records through
//!    the durable `kernel_event_ledger` table (aggregate type
//!    [`MEMORY_TRACE_AGGREGATE_TYPE`], aggregate id = `trace_id`) and loads
//!    referenced artifact payload bytes from the real on-disk ArtifactStore
//!    (hash-validated before the bytes are handed to the redactor).
//!  - [`export_persisted_trace`] is the production export entry point: it
//!    builds a [`RetrievalTraceExporter`] over the Postgres source, runs the
//!    redact-then-serialize pipeline, and appends an export receipt event to
//!    the ledger so every support-bundle export is auditable.
//!
//! Sub-rule 1 compliance: no placeholders, no `*Unavailable` escape paths.
//! Storage failures surface as the typed [`ExportError::Store`] variant.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use serde_json::{json, Value};
use uuid::Uuid;

use super::persistence_postgres::block_on;
use super::trace_export::{
    ExportError, ExportFormat, ExportedBundle, RedactionPolicy, RetrievalTraceExporter,
    TraceBundle, TraceSource,
};
use crate::kernel::{KernelActor, KernelEventType, NewKernelEvent};
use crate::storage::artifacts::{artifact_root_dir, validate_artifact_content_hash, ArtifactLayer};
use crate::storage::{Database, StorageError};

/// Aggregate type used for persisted retrieval trace bundles in the kernel
/// event ledger. The aggregate id is the `trace_id`.
pub const MEMORY_TRACE_AGGREGATE_TYPE: &str = "memory_retrieval_trace";

/// Source component label written to `kernel_event_ledger.source_component`
/// so operators can filter MT-165 trace-export traffic in queries.
pub const MEMORY_TRACE_SOURCE_COMPONENT: &str = "memory_trace_export_postgres";

/// Payload schema for a persisted trace bundle event.
pub const MEMORY_TRACE_BUNDLE_PAYLOAD_SCHEMA_ID: &str = "hsk.memory_trace.bundle@1";

/// Payload schema for an export receipt event.
pub const MEMORY_TRACE_EXPORTED_PAYLOAD_SCHEMA_ID: &str = "hsk.memory_trace.exported@1";

/// Production [`TraceSource`]: trace bundles live in the kernel event ledger
/// (PostgreSQL); referenced artifact payloads live in the on-disk
/// ArtifactStore under `workspace_root`.
///
/// Artifact ids referenced from [`TraceBundle::referenced_artifacts`] use the
/// `"<layer>/<uuid>"` form (e.g. `"L1/018f...".`); a bare UUID defaults to
/// layer `L1`.
pub struct PostgresTraceSource {
    db: Arc<dyn Database>,
    workspace_root: PathBuf,
}

impl PostgresTraceSource {
    pub fn new(db: Arc<dyn Database>, workspace_root: impl Into<PathBuf>) -> Self {
        Self {
            db,
            workspace_root: workspace_root.into(),
        }
    }

    /// Persist a trace bundle as a durable ledger event so a later support
    /// export (possibly in another process) can re-read it from PostgreSQL.
    ///
    /// Idempotent per (trace_id, deterministic content hash): re-persisting
    /// the same bundle collapses onto the existing ledger row.
    pub fn persist_trace(&self, bundle: &TraceBundle) -> Result<(), ExportError> {
        let payload = json!({
            "schema_id": MEMORY_TRACE_BUNDLE_PAYLOAD_SCHEMA_ID,
            "bundle": bundle,
        });
        let event = NewKernelEvent::builder(
            format!("KTR-MEMORY-TRACE-{}", bundle.trace_id),
            format!("SR-MEMORY-TRACE-{}", bundle.trace_id),
            KernelEventType::ArtifactProposed,
            KernelActor::System("memory_trace_export".to_string()),
        )
        .aggregate(MEMORY_TRACE_AGGREGATE_TYPE, bundle.trace_id.to_string())
        .idempotency_key(format!(
            "memory_trace_bundle:{}:{}",
            bundle.trace_id,
            bundle.deterministic_hash()
        ))
        .event_version("kernel_event_v1")
        .source_component(MEMORY_TRACE_SOURCE_COMPONENT)
        .payload(payload)
        .build()
        .map_err(|err| ExportError::Store {
            message: format!("trace bundle event build failed: {err}"),
        })?;

        let db = Arc::clone(&self.db);
        match block_on(async move { db.append_kernel_event(event).await }) {
            Ok(_) => Ok(()),
            Err(error) if is_idempotency_conflict(&error) => Ok(()),
            Err(error) => Err(ExportError::Store {
                message: format!("appending trace bundle to kernel_event_ledger failed: {error}"),
            }),
        }
    }

    fn decode_bundle_event(payload: &Value) -> Option<TraceBundle> {
        if payload.get("schema_id")?.as_str()? != MEMORY_TRACE_BUNDLE_PAYLOAD_SCHEMA_ID {
            return None;
        }
        serde_json::from_value(payload.get("bundle")?.clone()).ok()
    }
}

fn is_idempotency_conflict(error: &StorageError) -> bool {
    matches!(
        error,
        StorageError::Validation(message) if message.starts_with("kernel event idempotency conflict")
    )
}

/// Parse a `"<layer>/<uuid>"` (or bare-uuid, defaulting to L1) artifact id.
fn parse_artifact_id(artifact_id: &str) -> Result<(ArtifactLayer, Uuid), ExportError> {
    let (layer, raw_uuid) = match artifact_id.split_once('/') {
        Some((layer_str, rest)) => {
            let layer = match layer_str {
                "L1" => ArtifactLayer::L1,
                "L2" => ArtifactLayer::L2,
                "L3" => ArtifactLayer::L3,
                "L4" => ArtifactLayer::L4,
                other => {
                    return Err(ExportError::ArtifactLoad {
                        artifact_id: artifact_id.to_string(),
                        message: format!("unknown ArtifactStore layer {other}"),
                    })
                }
            };
            (layer, rest)
        }
        None => (ArtifactLayer::L1, artifact_id),
    };
    let uuid = Uuid::parse_str(raw_uuid).map_err(|err| ExportError::ArtifactLoad {
        artifact_id: artifact_id.to_string(),
        message: format!("artifact id is not a UUID: {err}"),
    })?;
    Ok((layer, uuid))
}

impl TraceSource for PostgresTraceSource {
    fn load_trace(&self, trace_id: Uuid) -> Result<TraceBundle, ExportError> {
        let db = Arc::clone(&self.db);
        let aggregate_id = trace_id.to_string();
        let events = block_on(async move {
            db.list_kernel_events_for_aggregate(MEMORY_TRACE_AGGREGATE_TYPE, &aggregate_id)
                .await
        })
        .map_err(|error| ExportError::Store {
            message: format!("reading trace bundle from kernel_event_ledger failed: {error}"),
        })?;

        let mut latest: Option<TraceBundle> = None;
        let mut latest_sequence = i64::MIN;
        for event in events {
            if let Some(bundle) = Self::decode_bundle_event(&event.payload) {
                if event.event_sequence > latest_sequence {
                    latest_sequence = event.event_sequence;
                    latest = Some(bundle);
                }
            }
        }
        latest.ok_or(ExportError::UnknownTrace { trace_id })
    }

    fn load_artifact(&self, artifact_id: &str) -> Result<Vec<u8>, ExportError> {
        let (layer, artifact_uuid) = parse_artifact_id(artifact_id)?;
        // Hash-validate the stored payload BEFORE handing the bytes to the
        // redactor so a tampered ArtifactStore entry cannot smuggle content
        // into a support bundle under a stale manifest hash.
        validate_artifact_content_hash(&self.workspace_root, layer, artifact_uuid).map_err(
            |err| ExportError::ArtifactLoad {
                artifact_id: artifact_id.to_string(),
                message: format!("ArtifactStore hash validation failed: {err}"),
            },
        )?;
        let payload_path =
            artifact_root_dir(&self.workspace_root, layer, artifact_uuid).join("payload");
        std::fs::read(&payload_path).map_err(|err| ExportError::ArtifactLoad {
            artifact_id: artifact_id.to_string(),
            message: format!(
                "reading ArtifactStore payload {} failed: {err}",
                payload_path.display()
            ),
        })
    }
}

/// Production export entry point: load the persisted trace from PostgreSQL,
/// run the redaction + serialization pipeline, and append an auditable export
/// receipt event to the kernel event ledger.
pub fn export_persisted_trace(
    db: Arc<dyn Database>,
    workspace_root: &Path,
    trace_id: Uuid,
    policy: &RedactionPolicy,
    format: ExportFormat,
) -> Result<ExportedBundle, ExportError> {
    let source = PostgresTraceSource::new(Arc::clone(&db), workspace_root);
    let exporter = RetrievalTraceExporter::new(&source);
    let exported = exporter.export(trace_id, policy, format)?;

    let format_label = serde_json::to_value(format)
        .ok()
        .and_then(|value| value.as_str().map(str::to_string))
        .unwrap_or_else(|| "json".to_string());
    let receipt_payload = json!({
        "schema_id": MEMORY_TRACE_EXPORTED_PAYLOAD_SCHEMA_ID,
        "trace_id": trace_id.to_string(),
        "format": format_label,
        "content_hash": exported.content_hash,
        "exported_bytes": exported.bytes.len(),
        "policy": policy,
    });
    let event = NewKernelEvent::builder(
        format!("KTR-MEMORY-TRACE-{trace_id}"),
        format!("SR-MEMORY-TRACE-{trace_id}"),
        KernelEventType::ArtifactProposed,
        KernelActor::System("memory_trace_export".to_string()),
    )
    .aggregate(MEMORY_TRACE_AGGREGATE_TYPE, trace_id.to_string())
    .idempotency_key(format!(
        "memory_trace_exported:{trace_id}:{}:{format_label}",
        exported.content_hash
    ))
    .event_version("kernel_event_v1")
    .source_component(MEMORY_TRACE_SOURCE_COMPONENT)
    .payload(receipt_payload)
    .build()
    .map_err(|err| ExportError::Store {
        message: format!("trace export receipt event build failed: {err}"),
    })?;

    match block_on(async move { db.append_kernel_event(event).await }) {
        Ok(_) => Ok(exported),
        // Re-exporting the same trace with the same policy/format collapses
        // onto the existing receipt row — the export itself still succeeded.
        Err(error) if is_idempotency_conflict(&error) => Ok(exported),
        Err(error) => Err(ExportError::Store {
            message: format!(
                "appending trace export receipt to kernel_event_ledger failed: {error}"
            ),
        }),
    }
}
