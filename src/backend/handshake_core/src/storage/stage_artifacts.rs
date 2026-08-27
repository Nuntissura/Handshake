//! WP-KERNEL-012 MT-066/074 exact-byte Stage capture authority on embedded SurrealDB.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use surrealdb::types::{Bytes, Datetime, RecordId, RecordIdKey, SurrealValue};
use tokio::sync::Mutex;
use uuid::Uuid;

use super::{
    surreal::{event_ledger, SurrealStorage},
    StorageError,
};
use crate::kernel::NewKernelEvent;

pub const STAGE_CAPTURE_MANIFEST_SCHEMA: &str = "hsk.stage.capture_manifest@1";
const STAGE_CONTENT_KINDS: [&str; 4] = ["document", "selection", "canvas_node", "atelier_item"];
const WORKSPACES_TABLE: &str = "workspaces";
const ARTIFACTS_TABLE: &str = "stage_capture_artifacts";
const JOBS_TABLE: &str = "ai_jobs";

static STAGE_INSERT_LOCK: Mutex<()> = Mutex::const_new(());

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct StageCaptureArtifact {
    pub artifact_id: String,
    pub workspace_id: String,
    pub content_kind: String,
    pub label: String,
    pub content_type: String,
    pub content_json: Value,
    pub content_bytes: Vec<u8>,
    pub size_bytes: i64,
    pub content_sha256: String,
    pub manifest: Value,
    pub manifest_ref: String,
    pub source_ref: Option<String>,
    pub idempotency_key: String,
    pub request_hash: String,
    pub actor_kind: String,
    pub actor_id: String,
    pub correlation_id: String,
    pub approval_id: String,
    pub job_id: Option<String>,
    pub event_ledger_event_id: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Clone, Debug)]
pub struct NewStageCaptureArtifact {
    pub workspace_id: String,
    pub content_kind: String,
    pub label: String,
    pub content_type: String,
    pub content_json: Value,
    pub content_bytes: Vec<u8>,
    pub source_ref: Option<String>,
    pub idempotency_key: String,
    pub request_hash: String,
    pub actor_kind: String,
    pub actor_id: String,
    pub correlation_id: String,
    pub approval_id: String,
    pub decision_receipt: NewKernelEvent,
    pub receipt: NewKernelEvent,
}

#[derive(Clone, Debug)]
pub struct StageArtifactInsertResult {
    pub artifact: StageCaptureArtifact,
    pub replayed: bool,
}

#[derive(SurrealValue)]
struct ArtifactRow {
    artifact_id: String,
    workspace_id: RecordId,
    content_kind: String,
    label: String,
    content_type: String,
    content_json: Value,
    content_bytes: Bytes,
    size_bytes: i64,
    content_sha256: String,
    manifest: Value,
    manifest_ref: String,
    source_ref: Option<String>,
    idempotency_key: String,
    request_hash: String,
    actor_kind: String,
    actor_id: String,
    correlation_id: String,
    approval_id: String,
    job_id: Option<String>,
    event_ledger_event_id: Option<RecordId>,
    created_at: Datetime,
    updated_at: Datetime,
}

#[derive(SurrealValue)]
struct ArtifactLookup {
    workspace: RecordId,
    value: String,
}

#[derive(SurrealValue)]
struct InsertBindings {
    job_record: RecordId,
    job_id: String,
    trace_id: String,
    entity_refs: Value,
    job_inputs: Value,
    job_outputs: Value,
    now: Datetime,
    decision: event_ledger::LedgerWrite,
    receipt: event_ledger::LedgerWrite,
    artifact_record: RecordId,
    artifact_id: String,
    workspace: RecordId,
    content_kind: String,
    label: String,
    content_type: String,
    content_json: Value,
    content_bytes: Bytes,
    size_bytes: i64,
    content_sha256: String,
    manifest: Value,
    manifest_ref: String,
    source_ref: Option<String>,
    idempotency_key: String,
    request_hash: String,
    actor_kind: String,
    actor_id: String,
    correlation_id: String,
    approval_id: String,
}

#[derive(Clone)]
pub struct StageArtifactStore {
    storage: SurrealStorage,
}

impl StageArtifactStore {
    pub fn new(storage: SurrealStorage) -> Self {
        Self { storage }
    }

    fn content_sha256(content: &[u8]) -> String {
        hex::encode(Sha256::digest(content))
    }

    pub async fn insert_stage_artifact(
        &self,
        mut input: NewStageCaptureArtifact,
    ) -> Result<StageArtifactInsertResult, StorageError> {
        validate_input(&input)?;
        let _serial = STAGE_INSERT_LOCK.lock().await;

        if let Some(existing) = self
            .get_by_idempotency(&input.workspace_id, &input.idempotency_key)
            .await?
        {
            if existing.request_hash != input.request_hash {
                return Err(StorageError::Conflict(
                    "stage capture idempotency key was reused with a different request",
                ));
            }
            return Ok(StageArtifactInsertResult {
                artifact: existing,
                replayed: true,
            });
        }

        let content_kind = input.content_kind.trim().to_owned();
        let content_type = input.content_type.trim().to_owned();
        let artifact_id = format!("STGA-{}", Uuid::now_v7().simple());
        let content_sha256 = Self::content_sha256(&input.content_bytes);
        let manifest_ref = format!("manifest://{artifact_id}");
        let source_ref = input
            .source_ref
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_owned);
        let manifest = json!({
            "schema": STAGE_CAPTURE_MANIFEST_SCHEMA,
            "sha256": content_sha256,
            "manifest_ref": manifest_ref,
            "content_type": content_type,
            "source_ref": source_ref,
            "size_bytes": input.content_bytes.len(),
            "correlation_id": input.correlation_id,
        });
        let job_id = Uuid::now_v7().to_string();
        let trace_id = Uuid::now_v7().to_string();

        input.decision_receipt.aggregate_id = artifact_id.clone();
        input.decision_receipt.payload = json!({
            "workspace_id": input.workspace_id,
            "artifact_id": artifact_id,
            "capability_id": "stage.jobs.enqueue",
            "decision_outcome": "allow",
            "approval_id": input.approval_id,
            "correlation_id": input.correlation_id,
            "job_id": job_id,
        });
        input.decision_receipt.payload_hash = payload_hash(&input.decision_receipt.payload);
        let (decision_event, decision_write) = event_ledger::prepare_event(input.decision_receipt)?;

        input.receipt.aggregate_id = artifact_id.clone();
        input.receipt.payload = json!({
            "receipt_kind": "stage_capture_stored",
            "workspace_id": input.workspace_id,
            "artifact_id": artifact_id,
            "artifact_ref": format!("artifact://sha256/{content_sha256}"),
            "manifest_ref": manifest_ref,
            "sha256": content_sha256,
            "size_bytes": input.content_bytes.len(),
            "content_kind": content_kind,
            "source_ref": source_ref,
            "job_id": job_id,
            "capability_id": "stage.jobs.enqueue",
            "decision_outcome": "allow",
            "decision_event_id": decision_event.event_id,
            "approval_id": input.approval_id,
            "correlation_id": input.correlation_id,
        });
        input.receipt.payload_hash = payload_hash(&input.receipt.payload);
        let (stored_event, receipt_write) = event_ledger::prepare_event(input.receipt)?;

        let now = Utc::now();
        let entity_refs = json!([
            {"entity_id": input.workspace_id, "entity_kind": "workspace"},
            {"entity_id": artifact_id, "entity_kind": "stage_capture_artifact"}
        ]);
        let job_inputs = json!({
            "workspace_id": input.workspace_id,
            "artifact_id": artifact_id,
            "content_kind": content_kind,
            "content_sha256": content_sha256,
            "size_bytes": input.content_bytes.len(),
            "source_ref": source_ref,
            "correlation_id": input.correlation_id,
            "approval_id": input.approval_id,
        });
        let job_outputs = json!({
            "artifact_id": artifact_id,
            "artifact_ref": format!("artifact://sha256/{content_sha256}"),
            "manifest_ref": manifest_ref,
            "sha256": content_sha256,
        });

        let bindings = InsertBindings {
            job_record: RecordId::new(JOBS_TABLE, job_id.clone()),
            job_id: job_id.clone(),
            trace_id,
            entity_refs,
            job_inputs,
            job_outputs,
            now: Datetime::from(now),
            decision: decision_write,
            receipt: receipt_write,
            artifact_record: RecordId::new(ARTIFACTS_TABLE, artifact_id.clone()),
            artifact_id,
            workspace: RecordId::new(WORKSPACES_TABLE, input.workspace_id),
            content_kind,
            label: input.label,
            content_type,
            content_json: input.content_json,
            content_bytes: Bytes::from(input.content_bytes.clone()),
            size_bytes: input.content_bytes.len() as i64,
            content_sha256,
            manifest,
            manifest_ref,
            source_ref,
            idempotency_key: input.idempotency_key,
            request_hash: input.request_hash,
            actor_kind: input.actor_kind,
            actor_id: input.actor_id,
            correlation_id: input.correlation_id,
            approval_id: input.approval_id,
        };
        let rows: Vec<ArtifactRow> = self
            .storage
            .with_data_operation(move |database| {
                Box::pin(async move {
                    database
                        .query_values_at(INSERT_TRANSACTION, bindings, 4)
                        .await
                })
            })
            .await
            .map_err(StorageError::from)?;
        let artifact = rows
            .into_iter()
            .next()
            .map(map_artifact_row)
            .transpose()?
            .ok_or_else(|| {
                StorageError::Database("stage capture transaction returned no artifact".to_owned())
            })?;
        if artifact.event_ledger_event_id.as_deref() != Some(stored_event.event_id.as_str()) {
            return Err(StorageError::Database(
                "stage capture transaction returned the wrong EventLedger receipt".to_owned(),
            ));
        }
        Ok(StageArtifactInsertResult {
            artifact,
            replayed: false,
        })
    }

    pub async fn get_stage_artifact(
        &self,
        workspace_id: &str,
        artifact_id: &str,
    ) -> Result<Option<StageCaptureArtifact>, StorageError> {
        self.lookup(workspace_id, artifact_id, false).await
    }

    async fn get_by_idempotency(
        &self,
        workspace_id: &str,
        idempotency_key: &str,
    ) -> Result<Option<StageCaptureArtifact>, StorageError> {
        self.lookup(workspace_id, idempotency_key, true).await
    }

    async fn lookup(
        &self,
        workspace_id: &str,
        value: &str,
        by_idempotency: bool,
    ) -> Result<Option<StageCaptureArtifact>, StorageError> {
        let bindings = ArtifactLookup {
            workspace: RecordId::new(WORKSPACES_TABLE, workspace_id),
            value: value.to_owned(),
        };
        let statement = if by_idempotency {
            SELECT_BY_IDEMPOTENCY
        } else {
            SELECT_BY_ID
        };
        let row: Option<ArtifactRow> = self
            .storage
            .with_data_operation(move |database| {
                Box::pin(async move { database.query_first(statement, bindings).await })
            })
            .await
            .map_err(StorageError::from)?;
        row.map(map_artifact_row).transpose()
    }
}

const SELECT_BY_ID: &str = "SELECT artifact_id, workspace_id, content_kind, label, content_type, \
content_json, content_bytes, size_bytes, content_sha256, manifest, manifest_ref, source_ref, \
idempotency_key, request_hash, actor_kind, actor_id, correlation_id, approval_id, job_id, \
event_ledger_event_id, created_at, updated_at FROM stage_capture_artifacts \
WHERE workspace_id = $workspace AND artifact_id = $value LIMIT 1;";

const SELECT_BY_IDEMPOTENCY: &str =
    "SELECT artifact_id, workspace_id, content_kind, label, content_type, \
content_json, content_bytes, size_bytes, content_sha256, manifest, manifest_ref, source_ref, \
idempotency_key, request_hash, actor_kind, actor_id, correlation_id, approval_id, job_id, \
event_ledger_event_id, created_at, updated_at FROM stage_capture_artifacts \
WHERE workspace_id = $workspace AND idempotency_key = $value LIMIT 1;";

const INSERT_TRANSACTION: &str = r#"
BEGIN TRANSACTION;
CREATE $job_record CONTENT {
    trace_id: $trace_id, workflow_run_id: NONE, job_kind: 'workflow_run', status: 'completed',
    status_reason: 'stage_capture_stored', protocol_id: 'hsk.stage.capture@1', profile_id: 'default',
    capability_profile_id: 'stage.jobs.enqueue', access_mode: 'apply_scoped', safety_mode: 'strict',
    entity_refs: $entity_refs, planned_operations: [], metrics: {}, job_inputs: $job_inputs,
    job_outputs: $job_outputs, created_at: $now, updated_at: $now
};
CREATE $decision.record CONTENT {
    event_id: $decision.event_id, event_version: $decision.event_version,
    kernel_task_run_id: $decision.kernel_task_run_id, session_run_id: $decision.session_run_id,
    aggregate_type: $decision.aggregate_type, aggregate_id: $decision.aggregate_id,
    idempotency_key: $decision.idempotency_key, event_type: $decision.event_type,
    actor_kind: $decision.actor_kind, actor_id: $decision.actor_id,
    causation_id: $decision.causation_id, correlation_id: $decision.correlation_id,
    payload_hash: $decision.payload_hash, source_component: $decision.source_component,
    payload: $decision.payload, created_at: $decision.created_at
};
CREATE $receipt.record CONTENT {
    event_id: $receipt.event_id, event_version: $receipt.event_version,
    kernel_task_run_id: $receipt.kernel_task_run_id, session_run_id: $receipt.session_run_id,
    aggregate_type: $receipt.aggregate_type, aggregate_id: $receipt.aggregate_id,
    idempotency_key: $receipt.idempotency_key, event_type: $receipt.event_type,
    actor_kind: $receipt.actor_kind, actor_id: $receipt.actor_id,
    causation_id: $receipt.causation_id, correlation_id: $receipt.correlation_id,
    payload_hash: $receipt.payload_hash, source_component: $receipt.source_component,
    payload: $receipt.payload, created_at: $receipt.created_at
};
CREATE $artifact_record CONTENT {
    artifact_id: $artifact_id, workspace_id: $workspace, content_kind: $content_kind,
    label: $label, content_type: $content_type, content_json: $content_json,
    content_bytes: $content_bytes, size_bytes: $size_bytes, content_sha256: $content_sha256,
    manifest: $manifest, manifest_ref: $manifest_ref, source_ref: $source_ref,
    idempotency_key: $idempotency_key, request_hash: $request_hash,
    actor_kind: $actor_kind, actor_id: $actor_id, correlation_id: $correlation_id,
    approval_id: $approval_id, job_id: $job_id, event_ledger_event_id: $receipt.record,
    created_at: $now, updated_at: $now
};
COMMIT TRANSACTION;
"#;

fn validate_input(input: &NewStageCaptureArtifact) -> Result<(), StorageError> {
    if input.workspace_id.trim().is_empty() {
        return Err(StorageError::Validation(
            "stage artifact workspace_id is required",
        ));
    }
    if !STAGE_CONTENT_KINDS.contains(&input.content_kind.trim()) {
        return Err(StorageError::Validation(
            "stage artifact content_kind must be document|selection|canvas_node|atelier_item",
        ));
    }
    if input.content_type.trim().is_empty() {
        return Err(StorageError::Validation(
            "stage artifact content_type is required",
        ));
    }
    if input.content_bytes.is_empty() || input.content_bytes.len() > 16 * 1024 {
        return Err(StorageError::Validation(
            "stage artifact content_bytes must be 1..=16384 bytes",
        ));
    }
    if input.idempotency_key.trim().is_empty()
        || input.idempotency_key.len() > 256
        || !is_sha256(&input.request_hash)
        || !matches!(input.actor_kind.as_str(), "operator" | "system")
        || input.actor_id.trim().is_empty()
        || input.correlation_id.trim().is_empty()
        || input.approval_id.trim().is_empty()
    {
        return Err(StorageError::Validation(
            "stage artifact privileged identity/idempotency contract is invalid",
        ));
    }
    Ok(())
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn payload_hash(payload: &Value) -> String {
    crate::kernel::context_bundle::sha256_hex(&crate::kernel::context_bundle::canonical_json_bytes(
        payload,
    ))
}

fn map_artifact_row(row: ArtifactRow) -> Result<StageCaptureArtifact, StorageError> {
    Ok(StageCaptureArtifact {
        artifact_id: row.artifact_id,
        workspace_id: record_key(row.workspace_id, "stage artifact workspace")?,
        content_kind: row.content_kind,
        label: row.label,
        content_type: row.content_type,
        content_json: row.content_json,
        content_bytes: row.content_bytes.into_inner().to_vec(),
        size_bytes: row.size_bytes,
        content_sha256: row.content_sha256,
        manifest: row.manifest,
        manifest_ref: row.manifest_ref,
        source_ref: row.source_ref,
        idempotency_key: row.idempotency_key,
        request_hash: row.request_hash,
        actor_kind: row.actor_kind,
        actor_id: row.actor_id,
        correlation_id: row.correlation_id,
        approval_id: row.approval_id,
        job_id: row.job_id,
        event_ledger_event_id: row
            .event_ledger_event_id
            .map(|record| record_key(record, "stage artifact EventLedger receipt"))
            .transpose()?,
        created_at: row.created_at.into_inner(),
        updated_at: row.updated_at.into_inner(),
    })
}

fn record_key(record: RecordId, field: &str) -> Result<String, StorageError> {
    match record.key {
        RecordIdKey::String(value) => Ok(value),
        _ => Err(StorageError::Serialization(format!(
            "{field} is not a string record key"
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        kernel::{KernelActor, KernelEventType},
        storage::{
            surreal::{bootstrap_schema, SurrealDatabase, SurrealStorageConfig},
            Database, NewWorkspace, WriteContext,
        },
    };

    fn receipt(event_type: KernelEventType, key: &str) -> NewKernelEvent {
        NewKernelEvent::builder(
            "mt-136-stage-task",
            "mt-136-stage-session",
            event_type,
            KernelActor::Operator("mt-136-operator".to_owned()),
        )
        .aggregate("stage_capture_proof", "pending")
        .idempotency_key(key)
        .correlation_id("mt-136-stage-correlation")
        .source_component("storage_mt_136_proof")
        .payload(json!({"pending": true}))
        .build()
        .expect("valid stage receipt")
    }

    async fn open(path: &std::path::Path) -> SurrealStorage {
        let storage = SurrealStorage::open(
            SurrealStorageConfig::with_path(path).expect("valid embedded test path"),
        )
        .await
        .expect("open embedded SurrealDB");
        bootstrap_schema(&storage)
            .await
            .expect("bootstrap embedded schema");
        storage
    }

    #[tokio::test]
    async fn exact_stage_bytes_and_ledger_link_survive_shutdown_and_reopen() {
        let directory = tempfile::tempdir().expect("temporary MT-136 stage root");
        let path = directory.path().join("store");
        let storage = open(&path).await;
        let database = SurrealDatabase::new(storage.clone());
        let workspace = database
            .create_workspace(
                &WriteContext::human(Some("mt-136-operator".to_owned())),
                NewWorkspace {
                    name: "MT-136 Stage Proof".to_owned(),
                },
            )
            .await
            .expect("create proof workspace");
        let bytes = b"\x00MT-136\xffexact-stage-bytes".to_vec();
        let request_hash = hex::encode(Sha256::digest(b"mt-136-stage-request"));
        let inserted = StageArtifactStore::new(storage.clone())
            .insert_stage_artifact(NewStageCaptureArtifact {
                workspace_id: workspace.id.clone(),
                content_kind: "selection".to_owned(),
                label: "durability-proof".to_owned(),
                content_type: "application/octet-stream".to_owned(),
                content_json: json!({"proof": true}),
                content_bytes: bytes.clone(),
                source_ref: Some("document://mt-136".to_owned()),
                idempotency_key: "mt-136-stage-idempotency".to_owned(),
                request_hash,
                actor_kind: "operator".to_owned(),
                actor_id: "mt-136-operator".to_owned(),
                correlation_id: "mt-136-stage-correlation".to_owned(),
                approval_id: "mt-136-stage-approval".to_owned(),
                decision_receipt: receipt(
                    KernelEventType::ToolDecisionRecorded,
                    "mt-136-stage-decision",
                ),
                receipt: receipt(KernelEventType::ArtifactStored, "mt-136-stage-stored"),
            })
            .await
            .expect("insert exact stage artifact")
            .artifact;
        assert_eq!(inserted.content_bytes, bytes);
        assert!(inserted.event_ledger_event_id.is_some());
        drop(database);
        storage.shutdown().await.expect("close embedded store");
        drop(storage);

        let reopened = open(&path).await;
        let persisted = StageArtifactStore::new(reopened.clone())
            .get_stage_artifact(&workspace.id, &inserted.artifact_id)
            .await
            .expect("read reopened stage artifact")
            .expect("durable stage artifact");
        assert_eq!(persisted.content_bytes, bytes);
        assert_eq!(persisted.content_sha256, inserted.content_sha256);
        assert_eq!(
            persisted.event_ledger_event_id,
            inserted.event_ledger_event_id
        );
        reopened.shutdown().await.expect("close reopened store");
    }
}
