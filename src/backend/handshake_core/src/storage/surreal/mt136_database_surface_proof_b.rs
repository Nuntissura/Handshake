//! MT-136 real-engine proofs for the Database AI/session/kernel/workflow surface.
//!
//! Every test opens an isolated embedded RocksDB-backed SurrealDB store. The
//! tests deliberately exercise public [`Database`] methods, then close and
//! reopen the same store before making the final durability assertions.

use crate::ai_ready_data::records::{
    EmbeddingModelRecord, EmbeddingModelStatus, IngestionSourceType, NewBronzeRecord,
    NewSilverRecord, ValidationStatus,
};
use crate::kernel::crdt::{
    identity::{CrdtAuthorityLinksV1, CrdtWorkspaceIdentityV1},
    persistence::{
        new_crdt_update_record, CrdtReplayMetadataV1, CrdtStorageAuthorityPosture,
        CrdtUpdateRecordInputV1,
    },
    snapshot::{new_crdt_snapshot_record, CrdtSnapshotRecordInputV1},
};
use crate::kernel::{
    session_broker::{SessionRun, SessionRunState},
    KernelActor, KernelEventType, NewKernelEvent,
};
use crate::storage::knowledge::{
    KnowledgePermissionScope, KnowledgeRedactionState, KnowledgeSourceKind, KnowledgeSpanKind,
    KnowledgeStore, NewKnowledgeSource, NewKnowledgeSpan,
};
use crate::storage::knowledge_crdt::{NewGraphMutationProposal, NewPromotedFact};
use crate::storage::{
    AccessMode, AiJobListFilter, Database, EntityRef, JobKind, JobMetrics, JobState,
    JobStatusUpdate, ModelSessionState, NewAiJob, NewModelSession, NewWorkspace, OperationType,
    PlannedOperation, SafetyMode, StorageError, StorageResult, WriteContext,
};
use crate::workspace_safety::MergeBackArtifact;
use chrono::Utc;
use serde_json::{json, Value};
use surrealdb::types::SurrealValue;
use uuid::Uuid;

use super::{
    mt136_proof_harness::{embedded_proof_backend, EmbeddedProofBackend},
    SurrealDatabase, SurrealStorage,
};

#[derive(SurrealValue)]
struct EmptyAuthorityBindings {}

async fn assert_crdt_authority_wires(storage: &SurrealStorage) -> StorageResult<()> {
    let (updates, snapshots): (Vec<String>, Vec<String>) = storage
        .with_data_operation(|database| {
            Box::pin(async move {
                let updates = database
                    .query_values(
                        "SELECT VALUE storage_authority FROM kernel_crdt_updates;",
                        EmptyAuthorityBindings {},
                    )
                    .await?;
                let snapshots = database
                    .query_values(
                        "SELECT VALUE storage_authority FROM kernel_crdt_snapshots;",
                        EmptyAuthorityBindings {},
                    )
                    .await?;
                Ok((updates, snapshots))
            })
        })
        .await
        .map_err(StorageError::from)?;
    assert!(!updates.is_empty());
    assert!(!snapshots.is_empty());
    assert!(updates.iter().chain(&snapshots).all(|authority| {
        authority == "surreal_event_ledger" && !authority.contains("postgres")
    }));
    Ok(())
}

fn event(
    key: &str,
    session: &str,
    event_type: KernelEventType,
    aggregate_type: &str,
    aggregate_id: &str,
    payload: Value,
) -> NewKernelEvent {
    NewKernelEvent::builder(
        "KTR-mt136-proof-b",
        session,
        event_type,
        KernelActor::System("mt136-proof-b".to_owned()),
    )
    .aggregate(aggregate_type, aggregate_id)
    .idempotency_key(key)
    .correlation_id("mt136-proof-b")
    .source_component("mt136_database_surface_proof_b")
    .payload(payload)
    .build()
    .expect("valid MT-136 event fixture")
}

fn new_job() -> NewAiJob {
    NewAiJob {
        trace_id: Uuid::now_v7(),
        job_kind: JobKind::WorkflowRun,
        protocol_id: "mt136-proof-b".to_owned(),
        profile_id: "mt136-proof-b".to_owned(),
        capability_profile_id: "mt136-proof-b".to_owned(),
        access_mode: AccessMode::AnalysisOnly,
        safety_mode: SafetyMode::Strict,
        entity_refs: vec![EntityRef {
            entity_id: "mt136-proof-b".to_owned(),
            entity_kind: "proof".to_owned(),
        }],
        planned_operations: vec![PlannedOperation {
            op_type: OperationType::Read,
            target: EntityRef {
                entity_id: "mt136-proof-b".to_owned(),
                entity_kind: "proof".to_owned(),
            },
            description: Some("exercise the embedded AI-job surface".to_owned()),
        }],
        status_reason: "queued for MT-136 proof".to_owned(),
        metrics: JobMetrics::zero(),
        job_inputs: Some(json!({"proof": "mt136-b"})),
    }
}

fn model_session(job_id: Uuid) -> NewModelSession {
    NewModelSession {
        session_id: "mt136-proof-b-session".to_owned(),
        parent_session_id: None,
        spawn_depth: 0,
        state: ModelSessionState::Created,
        model_id: "mt136-proof-model".to_owned(),
        backend: "surreal-embedded".to_owned(),
        parameter_class: "proof".to_owned(),
        role: "validator".to_owned(),
        wp_id: Some("WP-KERNEL-012".to_owned()),
        mt_id: Some("MT-136".to_owned()),
        work_profile_id: None,
        execution_mode: "proof".to_owned(),
        memory_policy: "workspace_scoped".to_owned(),
        consent_receipt_id: None,
        capability_grants: vec!["storage.read".to_owned(), "storage.write".to_owned()],
        capability_token_ids: None,
        job_id: Some(job_id),
        checkpoint_artifact_id: None,
        last_checkpoint_at: None,
        checkpoint_count: 0,
        agent: Some("mt136-proof-b".to_owned()),
        purpose: Some("real embedded Database surface proof".to_owned()),
    }
}

async fn reopen(
    backend: EmbeddedProofBackend,
) -> StorageResult<(EmbeddedProofBackend, SurrealDatabase)> {
    let backend = backend.reopen().await?;
    let reopened = SurrealDatabase::new(backend.storage.clone());
    Ok((backend, reopened))
}

async fn ai_ready_job_session_and_workflow_methods_use_durable_embedded_state() -> StorageResult<()>
{
    let backend = embedded_proof_backend().await?;
    let database = backend.database.clone();
    let storage = backend.storage.clone();
    let ctx = WriteContext::system(Some("mt136-proof-b".to_owned()));
    let workspace = database
        .create_workspace(
            &ctx,
            NewWorkspace {
                name: "mt136-proof-b-ai".to_owned(),
            },
        )
        .await?;

    let bronze = NewBronzeRecord {
        bronze_id: "mt136-b-bronze".to_owned(),
        workspace_id: workspace.id.clone(),
        content_hash: "a".repeat(64),
        content_type: "document/markdown".to_owned(),
        content_encoding: "utf-8".to_owned(),
        size_bytes: 12,
        original_filename: Some("proof.md".to_owned()),
        artifact_path: "bronze/mt136-b".to_owned(),
        ingestion_source_type: IngestionSourceType::System,
        ingestion_source_id: Some("mt136-proof-b".to_owned()),
        ingestion_method: "proof".to_owned(),
        external_source_json: Some("{}".to_owned()),
        retention_policy: "default".to_owned(),
    };
    let created_bronze = database
        .create_ai_bronze_record(&ctx, bronze.clone())
        .await?;
    assert_eq!(created_bronze.bronze_id, bronze.bronze_id);
    assert_eq!(
        database
            .get_ai_bronze_record(&bronze.bronze_id)
            .await?
            .expect("created bronze")
            .content_hash,
        bronze.content_hash
    );
    assert_eq!(
        database.list_ai_bronze_records(&workspace.id).await?.len(),
        1
    );
    assert!(matches!(
        database.create_ai_bronze_record(&ctx, bronze.clone()).await,
        Err(StorageError::Conflict(_))
    ));

    let silver = |id: &str, hash_byte: char| NewSilverRecord {
        silver_id: id.to_owned(),
        workspace_id: workspace.id.clone(),
        bronze_ref: bronze.bronze_id.clone(),
        chunk_index: 0,
        total_chunks: 1,
        token_count: 3,
        content_hash: hash_byte.to_string().repeat(64),
        byte_start: 0,
        byte_end: 12,
        line_start: 1,
        line_end: 1,
        chunk_artifact_path: format!("silver/{id}/chunk"),
        embedding_artifact_path: format!("silver/{id}/embedding"),
        embedding_model_id: "embed-proof".to_owned(),
        embedding_model_version: "1".to_owned(),
        embedding_dimensions: 3,
        embedding_compute_latency_ms: 1,
        chunking_strategy: "whole".to_owned(),
        chunking_version: "1".to_owned(),
        processing_pipeline_version: "1".to_owned(),
        processing_duration_ms: 2,
        metadata_json: "{}".to_owned(),
        validation_status: ValidationStatus::Passed,
        validation_failed_checks_json: "[]".to_owned(),
        validator_version: "1".to_owned(),
    };
    database
        .create_ai_silver_record(&ctx, silver("mt136-b-silver-1", 'b'))
        .await?;
    database
        .create_ai_silver_record(&ctx, silver("mt136-b-silver-2", 'c'))
        .await?;
    assert!(database
        .get_ai_silver_record("mt136-b-silver-1")
        .await?
        .is_some());
    assert_eq!(
        database
            .list_ai_silver_records_by_bronze(&bronze.bronze_id)
            .await?
            .len(),
        2
    );
    assert_eq!(
        database.list_ai_silver_records(&workspace.id).await?.len(),
        2
    );
    let failed_supersede = database
        .supersede_ai_silver_record(&ctx, "mt136-b-silver-1", "missing-silver")
        .await;
    assert!(failed_supersede.is_err());
    assert!(
        database
            .get_ai_silver_record("mt136-b-silver-1")
            .await?
            .expect("rollback preserved old silver")
            .is_current
    );
    database
        .supersede_ai_silver_record(&ctx, "mt136-b-silver-1", "mt136-b-silver-2")
        .await?;
    let superseded = database
        .get_ai_silver_record("mt136-b-silver-1")
        .await?
        .expect("superseded silver");
    assert!(!superseded.is_current);
    assert_eq!(
        superseded.superseded_by.as_deref(),
        Some("mt136-b-silver-2")
    );

    let embedding = EmbeddingModelRecord {
        model_id: "embed-proof".to_owned(),
        model_version: "1".to_owned(),
        dimensions: 3,
        max_input_tokens: 1024,
        content_types: vec!["document/markdown".to_owned()],
        status: EmbeddingModelStatus::Active,
        introduced_at: Utc::now(),
        compatible_with: vec!["embed-proof:1".to_owned()],
    };
    database
        .upsert_ai_embedding_model(&ctx, embedding.clone())
        .await?;
    database.upsert_ai_embedding_model(&ctx, embedding).await?;
    assert_eq!(database.list_ai_embedding_models().await?.len(), 1);
    assert!(database
        .set_ai_embedding_default_model(&ctx, "missing-model", "1")
        .await
        .is_err());
    assert!(database.get_ai_embedding_registry().await?.is_none());
    database
        .set_ai_embedding_default_model(&ctx, "embed-proof", "1")
        .await?;
    assert_eq!(
        database
            .get_ai_embedding_registry()
            .await?
            .expect("embedding registry")
            .current_default_model_id,
        "embed-proof"
    );

    let job = database.create_ai_job(new_job()).await?;
    assert_eq!(
        database
            .list_ai_jobs(AiJobListFilter {
                status: Some(JobState::Queued),
                job_kind: Some(JobKind::WorkflowRun),
                ..AiJobListFilter::default()
            })
            .await?
            .len(),
        1
    );
    let running = database
        .update_ai_job_status(JobStatusUpdate {
            job_id: job.job_id,
            state: JobState::Running,
            error_message: None,
            status_reason: "running embedded proof".to_owned(),
            metrics: Some(JobMetrics::zero()),
            workflow_run_id: None,
            trace_id: None,
            job_outputs: None,
        })
        .await?;
    assert_eq!(running.state, JobState::Running);
    database
        .set_job_outputs(&job.job_id.to_string(), Some(json!({"durable": true})))
        .await?;

    database
        .upsert_model_session(model_session(job.job_id))
        .await?;
    assert_eq!(
        database
            .get_model_session_by_job_id(job.job_id)
            .await?
            .session_id,
        "mt136-proof-b-session"
    );
    database
        .update_model_session_state(
            "mt136-proof-b-session",
            ModelSessionState::Active,
            Some(job.job_id),
        )
        .await?;
    let merge_back = MergeBackArtifact {
        session_id: "mt136-proof-b-session".to_owned(),
        worktree_path: "worktrees/mt136-proof-b".to_owned(),
        produced_at: Utc::now(),
        diff_patch: "diff --git a/proof b/proof".to_owned(),
        conflict_report: None,
    };
    database
        .update_model_session_state_with_merge_back_artifact(
            "mt136-proof-b-session",
            ModelSessionState::Completed,
            Some(job.job_id),
            Some(merge_back.clone()),
        )
        .await?;

    let run = database
        .create_workflow_run(job.job_id, JobState::Queued, None)
        .await?;
    let failed_run = database
        .update_workflow_run_status(
            run.id,
            JobState::Failed,
            Some("durable workflow failure".to_owned()),
        )
        .await?;
    assert_eq!(failed_run.status, JobState::Failed);
    assert!(matches!(
        database
            .update_workflow_run_status(Uuid::now_v7(), JobState::Failed, None)
            .await,
        Err(StorageError::NotFound("workflow_run"))
    ));
    database
        .mark_ai_bronze_deleted(&ctx, &bronze.bronze_id)
        .await?;

    drop(storage);
    drop(database);
    let (backend, reopened) = reopen(backend).await?;
    assert!(
        Database::get_ai_bronze_record(&reopened, &bronze.bronze_id)
            .await?
            .expect("durable bronze")
            .is_deleted
    );
    assert_eq!(
        Database::list_ai_silver_records(&reopened, &workspace.id)
            .await?
            .len(),
        2
    );
    assert_eq!(
        Database::list_ai_embedding_models(&reopened).await?.len(),
        1
    );
    assert_eq!(
        Database::get_ai_job(&reopened, &job.job_id.to_string())
            .await?
            .job_outputs,
        Some(json!({"durable": true}))
    );
    assert_eq!(
        Database::update_workflow_run_status(&reopened, run.id, JobState::Completed, None)
            .await?
            .status,
        JobState::Completed
    );
    let durable_session = Database::get_model_session_by_job_id(&reopened, job.job_id).await?;
    assert_eq!(durable_session.state, ModelSessionState::Completed);
    assert_eq!(durable_session.merge_back_artifact, Some(merge_back));
    drop(reopened);
    backend.close_and_remove().await?;
    Ok(())
}

fn crdt_identity() -> CrdtWorkspaceIdentityV1 {
    CrdtWorkspaceIdentityV1 {
        schema_id: "hsk.kernel.crdt_workspace_identity@1".to_owned(),
        workspace_id: "mt136-b-workspace".to_owned(),
        document_id: "mt136-b-document".to_owned(),
        crdt_document_id: "mt136-b-crdt-document".to_owned(),
        actor_id: "mt136-proof-b".to_owned(),
        actor_kind: "system".to_owned(),
        crdt_site_id: "mt136-b-site".to_owned(),
        crdt_client_id: "mt136-b-client".to_owned(),
        document_schema_id: "hsk.rich_document@1".to_owned(),
        authority_links: CrdtAuthorityLinksV1 {
            work_item_id: "MT-136".to_owned(),
            action_trace_id: "mt136-b-trace".to_owned(),
            artifact_proposal_id: "mt136-b-artifact".to_owned(),
            role_mailbox_thread_id: "mt136-b-thread".to_owned(),
            dcc_projection_id: "mt136-b-dcc".to_owned(),
            event_ledger_stream_id: "mt136-b-stream".to_owned(),
        },
    }
}

async fn event_crdt_and_kernel_queue_methods_are_atomic_and_durable() -> StorageResult<()> {
    let backend = embedded_proof_backend().await?;
    let database = backend.database.clone();
    let storage = backend.storage.clone();
    let session = "mt136-b-kernel-session";

    let first = event(
        "mt136-b-batch-1",
        session,
        KernelEventType::ArtifactStored,
        "mt136_proof",
        "batch",
        json!({"ordinal": 1}),
    );
    let second = event(
        "mt136-b-batch-2",
        session,
        KernelEventType::ArtifactStored,
        "mt136_proof",
        "batch",
        json!({"ordinal": 2}),
    );
    let batch = database
        .append_kernel_events_atomic(vec![first.clone(), second.clone()])
        .await?;
    let replay = database
        .append_kernel_events_atomic(vec![first.clone(), second.clone()])
        .await?;
    assert_eq!(batch[0].event_id, replay[0].event_id);
    let rolled_back = event(
        "mt136-b-rollback-new",
        session,
        KernelEventType::ArtifactStored,
        "mt136_proof",
        "rollback",
        json!({"must": "rollback"}),
    );
    let conflicting = event(
        "mt136-b-batch-1",
        session,
        KernelEventType::ArtifactStored,
        "mt136_proof",
        "batch",
        json!({"ordinal": 99}),
    );
    assert!(matches!(
        database
            .append_kernel_events_atomic(vec![rolled_back, conflicting])
            .await,
        Err(StorageError::Conflict(_))
    ));
    assert!(!database
        .list_kernel_events_for_session(session)
        .await?
        .iter()
        .any(|stored| stored.idempotency_key == "mt136-b-rollback-new"));

    let pair = database
        .append_kernel_event_pair_atomic_with_causation(
            event(
                "mt136-b-pair-1",
                session,
                KernelEventType::PromotionRequested,
                "mt136_proof",
                "pair",
                json!({"stage": "requested"}),
            ),
            event(
                "mt136-b-pair-2",
                session,
                KernelEventType::PromotionAccepted,
                "mt136_proof",
                "pair",
                json!({"stage": "accepted"}),
            ),
        )
        .await?;
    assert_eq!(
        pair[1].causation_id.as_deref(),
        Some(pair[0].event_id.as_str())
    );

    let pending = event(
        "mt136-b-native-pending",
        session,
        KernelEventType::FlightRecorderMirrorPending,
        "native_editor_event",
        "mt136-b-native",
        json!({"legacy_pending": true}),
    );
    database.append_kernel_events_atomic(vec![pending]).await?;
    assert_eq!(
        database
            .list_pending_native_editor_mirrors(0, 10)
            .await?
            .len(),
        1
    );

    let update_event = database
        .append_kernel_events_atomic(vec![event(
            "mt136-b-crdt-update-event",
            session,
            KernelEventType::KnowledgeCrdtUpdateRecorded,
            "crdt_document",
            "mt136-b-crdt-document",
            json!({"update": 1}),
        )])
        .await?
        .remove(0);
    let snapshot_event = database
        .append_kernel_events_atomic(vec![event(
            "mt136-b-crdt-snapshot-event",
            session,
            KernelEventType::KnowledgeCrdtSnapshotRecorded,
            "crdt_document",
            "mt136-b-crdt-document",
            json!({"snapshot": 1}),
        )])
        .await?
        .remove(0);
    let identity = crdt_identity();
    let update_bytes = b"mt136-update".to_vec();
    let update = new_crdt_update_record(CrdtUpdateRecordInputV1 {
        identity: &identity,
        update_id: "mt136-b-update-1",
        update_seq: 1,
        update_bytes: &update_bytes,
        update_bytes_ref: "surreal://mt136-b/update-1",
        session_id: session,
        trace_id: "mt136-b-trace",
        state_vector_before: "sv0",
        state_vector_after: "sv1",
        replay_metadata: CrdtReplayMetadataV1 {
            replay_order_key: "00000001".to_owned(),
            dependency_update_ids: Vec::new(),
            encoding: "yjs-v1".to_owned(),
            schema_version: "1".to_owned(),
        },
        event_ledger_event_id: &update_event.event_id,
    });
    assert_eq!(
        update.storage_authority,
        CrdtStorageAuthorityPosture::SurrealEventLedger
    );
    database
        .append_kernel_crdt_update(update.clone(), update_bytes.clone())
        .await?;
    database
        .append_kernel_crdt_update(update.clone(), update_bytes.clone())
        .await?;
    let mut divergent_update = update.clone();
    divergent_update.trace_id = "divergent".to_owned();
    assert!(matches!(
        database
            .append_kernel_crdt_update(divergent_update, update_bytes.clone())
            .await,
        Err(StorageError::Conflict(_))
    ));
    let stored_updates = database
        .list_kernel_crdt_updates(
            &identity.workspace_id,
            &identity.document_id,
            &identity.crdt_document_id,
        )
        .await?;
    assert_eq!(stored_updates.len(), 1);
    assert!(stored_updates.iter().all(|record| {
        record.storage_authority == CrdtStorageAuthorityPosture::SurrealEventLedger
    }));
    assert_eq!(
        database
            .read_kernel_crdt_update_bytes(&update.update_bytes_ref)
            .await?,
        update_bytes
    );

    let snapshot_bytes = b"mt136-snapshot".to_vec();
    let snapshot = new_crdt_snapshot_record(CrdtSnapshotRecordInputV1 {
        identity: &identity,
        snapshot_id: "mt136-b-snapshot-1",
        covered_update_seq: 1,
        snapshot_bytes: &snapshot_bytes,
        snapshot_bytes_ref: "surreal://mt136-b/snapshot-1",
        state_vector: "sv1",
        event_ledger_event_id: &snapshot_event.event_id,
        promotion_evidence_update_ids: &["mt136-b-update-1"],
    });
    assert_eq!(
        snapshot.storage_authority,
        CrdtStorageAuthorityPosture::SurrealEventLedger
    );
    database
        .append_kernel_crdt_snapshot(snapshot.clone(), snapshot_bytes.clone())
        .await?;
    database
        .append_kernel_crdt_snapshot(snapshot.clone(), snapshot_bytes.clone())
        .await?;
    let mut divergent_snapshot = snapshot.clone();
    divergent_snapshot.state_vector = "divergent".to_owned();
    assert!(matches!(
        database
            .append_kernel_crdt_snapshot(divergent_snapshot, snapshot_bytes.clone())
            .await,
        Err(StorageError::Conflict(_))
    ));
    let stored_snapshots = database
        .list_kernel_crdt_snapshots(
            &identity.workspace_id,
            &identity.document_id,
            &identity.crdt_document_id,
        )
        .await?;
    assert_eq!(stored_snapshots.len(), 1);
    assert!(stored_snapshots.iter().all(|record| {
        record.storage_authority == CrdtStorageAuthorityPosture::SurrealEventLedger
    }));
    assert_crdt_authority_wires(&storage).await?;
    assert_eq!(
        database
            .read_kernel_crdt_snapshot_bytes(&snapshot.snapshot_bytes_ref)
            .await?,
        snapshot_bytes
    );

    let plain = SessionRun::queued("KTR-mt136-b-plain", "adapter-plain");
    let plain_id = plain.session_run_id.clone();
    database.enqueue_kernel_session_run(plain.clone()).await?;
    database.enqueue_kernel_session_run(plain).await?;
    assert!(matches!(
        database
            .claim_kernel_session_run(&plain_id, "worker", 0)
            .await,
        Err(StorageError::Validation(_))
    ));
    assert!(database
        .claim_kernel_session_run(&plain_id, "worker", 30)
        .await?
        .is_some());
    assert!(database
        .claim_kernel_session_run(&plain_id, "other-worker", 30)
        .await?
        .is_none());
    assert_eq!(
        database
            .update_kernel_session_run_state(&plain_id, SessionRunState::Running)
            .await?
            .state,
        SessionRunState::Running
    );
    assert!(matches!(
        database
            .update_kernel_session_run_state(&plain_id, SessionRunState::Queued)
            .await,
        Err(StorageError::Validation(_))
    ));

    let atomic = SessionRun::queued("KTR-mt136-b-atomic", "adapter-atomic");
    let atomic_id = atomic.session_run_id.clone();
    let (_, queued_event) = database
        .enqueue_kernel_session_run_and_record_event(
            atomic,
            None,
            "mt136-b-queue-correlation".to_owned(),
        )
        .await?;
    let (claimed, claimed_event) = database
        .claim_kernel_session_run_and_record_event(
            &atomic_id,
            "atomic-worker",
            30,
            Some(queued_event.event_id.clone()),
            "mt136-b-claim-correlation".to_owned(),
        )
        .await?
        .expect("atomic claim");
    assert_eq!(claimed.state, SessionRunState::Claimed);
    let (running, running_event) = database
        .update_kernel_session_run_state_and_record_event(
            &atomic_id,
            SessionRunState::Running,
            Some(claimed_event.event_id),
            "mt136-b-state-correlation".to_owned(),
        )
        .await?;
    assert_eq!(running.state, SessionRunState::Running);
    assert_eq!(running_event.event_type, KernelEventType::SessionStarted);

    let durable_queued = SessionRun::queued("KTR-mt136-b-durable", "adapter-durable");
    let durable_queued_id = durable_queued.session_run_id.clone();
    database.enqueue_kernel_session_run(durable_queued).await?;

    let before_reopen = database
        .list_kernel_events_for_session(session)
        .await?
        .len();
    drop(storage);
    drop(database);
    let (backend, reopened) = reopen(backend).await?;
    assert_eq!(
        Database::list_kernel_events_for_session(&reopened, session)
            .await?
            .len(),
        before_reopen
    );
    assert_eq!(
        Database::read_kernel_crdt_update_bytes(&reopened, &update.update_bytes_ref)
            .await?
            .as_slice(),
        b"mt136-update"
    );
    assert_eq!(
        Database::read_kernel_crdt_snapshot_bytes(&reopened, &snapshot.snapshot_bytes_ref)
            .await?
            .as_slice(),
        b"mt136-snapshot"
    );
    assert!(Database::list_kernel_crdt_updates(
        &reopened,
        &identity.workspace_id,
        &identity.document_id,
        &identity.crdt_document_id,
    )
    .await?
    .iter()
    .all(|record| record.storage_authority == CrdtStorageAuthorityPosture::SurrealEventLedger));
    assert!(Database::list_kernel_crdt_snapshots(
        &reopened,
        &identity.workspace_id,
        &identity.document_id,
        &identity.crdt_document_id,
    )
    .await?
    .iter()
    .all(|record| record.storage_authority == CrdtStorageAuthorityPosture::SurrealEventLedger));
    assert_crdt_authority_wires(&backend.storage).await?;
    assert!(Database::claim_kernel_session_run(
        &reopened,
        &durable_queued_id,
        "post-reopen-worker",
        30,
    )
    .await?
    .is_some());
    drop(reopened);
    backend.close_and_remove().await?;
    Ok(())
}

async fn graph_fact_promotion_rolls_back_on_mismatch_and_survives_reopen() -> StorageResult<()> {
    let backend = embedded_proof_backend().await?;
    let database = backend.database.clone();
    let storage = backend.storage.clone();
    let ctx = WriteContext::system(Some("mt136-proof-b-promotion".to_owned()));
    let workspace = database
        .create_workspace(
            &ctx,
            NewWorkspace {
                name: "mt136-proof-b-promotion".to_owned(),
            },
        )
        .await?;
    let knowledge_database = SurrealDatabase::new(storage.clone());
    let source = KnowledgeStore::upsert_knowledge_source(
        &knowledge_database,
        NewKnowledgeSource {
            workspace_id: workspace.id.clone(),
            root_id: None,
            source_kind: KnowledgeSourceKind::OperatorArtifact,
            relative_path: None,
            asset_id: None,
            loom_block_id: None,
            document_id: None,
            content_hash: "d".repeat(64),
            size_bytes: Some(12),
            provenance: json!({"proof": "mt136-b"}),
            permission_scope: KnowledgePermissionScope::Workspace,
            redaction_state: KnowledgeRedactionState::None,
            source_modified_at: Some(Utc::now()),
        },
    )
    .await?;
    let span = KnowledgeStore::create_knowledge_span(
        &knowledge_database,
        NewKnowledgeSpan {
            source_id: source.source_id,
            span_kind: KnowledgeSpanKind::Text,
            range_start: 0,
            range_end: 12,
            line_start: Some(1),
            line_end: Some(1),
            section_path: Some("proof".to_owned()),
            content_sha256: "e".repeat(64),
            parser_version: "mt136-proof-b".to_owned(),
            extraction_receipt_event_id: None,
            index_run_id: None,
            display_snippet: Some("proof".to_owned()),
        },
    )
    .await?;
    let setup_events = database
        .append_kernel_events_atomic(vec![
            event(
                "mt136-b-proposal-recorded",
                "mt136-b-promotion-session",
                KernelEventType::GraphMutationProposalRecorded,
                "graph_proposal",
                "mt136-b-proposal",
                json!({"proposal": "recorded"}),
            ),
            event(
                "mt136-b-proposal-decided",
                "mt136-b-promotion-session",
                KernelEventType::GraphMutationProposalDecided,
                "graph_proposal",
                "mt136-b-proposal",
                json!({"decision": "approved"}),
            ),
        ])
        .await?;
    let payload = json!({"subject": "mt136", "predicate": "uses", "object": "surreal"});
    crate::storage::knowledge_crdt::insert_graph_proposal(
        &storage,
        NewGraphMutationProposal {
            proposal_id: "mt136-b-proposal".to_owned(),
            workspace_id: workspace.id.clone(),
            mutation_kind: "add_claim".to_owned(),
            mutation_payload: payload.clone(),
            source_span_refs: vec![span.span_id.clone()],
            confidence: 0.95,
            actor_id: "mt136-proof-b".to_owned(),
            actor_kind: "system".to_owned(),
            session_id: "mt136-b-promotion-session".to_owned(),
            correlation_id: "mt136-b-promotion".to_owned(),
            lease_id: None,
            recorded_event_id: setup_events[0].event_id.clone(),
        },
    )
    .await?;
    crate::storage::knowledge_crdt::decide_graph_proposal(
        &storage,
        "mt136-b-proposal",
        "approved",
        "mt136-proof-b",
        "real embedded proof",
        &setup_events[1].event_id,
    )
    .await?
    .expect("proposal decision");

    let requested = event(
        "mt136-b-promotion-requested",
        "mt136-b-promotion-session",
        KernelEventType::PromotionRequested,
        "graph_proposal",
        "mt136-b-proposal",
        json!({"stage": "requested"}),
    );
    let accepted = event(
        "mt136-b-promotion-accepted",
        "mt136-b-promotion-session",
        KernelEventType::PromotionAccepted,
        "graph_proposal",
        "mt136-b-proposal",
        json!({"stage": "accepted"}),
    );
    let fact = NewPromotedFact {
        fact_id: "mt136-b-fact".to_owned(),
        proposal_id: "mt136-b-proposal".to_owned(),
        workspace_id: workspace.id.clone(),
        mutation_kind: "add_claim".to_owned(),
        fact_payload: payload,
        source_span_refs: json!([span.span_id]),
        confidence: 0.95,
        proposed_by: "mt136-proof-b".to_owned(),
        promoted_by: "mt136-proof-b-validator".to_owned(),
        promotion_requested_event_id: String::new(),
        promotion_accepted_event_id: String::new(),
    };
    let promoted = database
        .promote_graph_fact_atomic(requested.clone(), accepted.clone(), fact.clone())
        .await?;
    assert_eq!(promoted.fact_id, "mt136-b-fact");
    let replayed = database
        .promote_graph_fact_atomic(requested, accepted, fact)
        .await?;
    assert_eq!(replayed.fact_id, promoted.fact_id);

    let before_failure = database
        .list_kernel_events_for_session("mt136-b-promotion-session")
        .await?
        .len();
    let mismatch = database
        .promote_graph_fact_atomic(
            event(
                "mt136-b-rollback-requested",
                "mt136-b-promotion-session",
                KernelEventType::PromotionRequested,
                "graph_proposal",
                "missing-proposal",
                json!({"must": "rollback"}),
            ),
            event(
                "mt136-b-rollback-accepted",
                "mt136-b-promotion-session",
                KernelEventType::PromotionAccepted,
                "graph_proposal",
                "missing-proposal",
                json!({"must": "rollback"}),
            ),
            NewPromotedFact {
                fact_id: "mt136-b-rollback-fact".to_owned(),
                proposal_id: "missing-proposal".to_owned(),
                workspace_id: workspace.id,
                mutation_kind: "add_claim".to_owned(),
                fact_payload: json!({"must": "rollback"}),
                source_span_refs: json!(["KSP-00000000000000000000000000000000"]),
                confidence: 1.0,
                proposed_by: "mt136-proof-b".to_owned(),
                promoted_by: "mt136-proof-b".to_owned(),
                promotion_requested_event_id: String::new(),
                promotion_accepted_event_id: String::new(),
            },
        )
        .await;
    assert!(mismatch.is_err());
    assert_eq!(
        database
            .list_kernel_events_for_session("mt136-b-promotion-session")
            .await?
            .len(),
        before_failure
    );

    drop(knowledge_database);
    drop(storage);
    drop(database);
    let (backend, reopened) = reopen(backend).await?;
    let durable_events =
        Database::list_kernel_events_for_session(&reopened, "mt136-b-promotion-session").await?;
    assert!(durable_events
        .iter()
        .any(|stored| stored.idempotency_key == "mt136-b-promotion-accepted"));
    assert!(!durable_events
        .iter()
        .any(|stored| stored.idempotency_key == "mt136-b-rollback-requested"));
    assert_eq!(
        crate::storage::knowledge_crdt::get_promoted_fact_by_proposal(
            &backend.storage,
            "mt136-b-proposal",
        )
        .await?
        .expect("durable promoted fact")
        .fact_id,
        "mt136-b-fact"
    );
    assert!(
        crate::storage::knowledge_crdt::get_promoted_fact_by_proposal(
            &backend.storage,
            "missing-proposal",
        )
        .await?
        .is_none()
    );
    drop(reopened);
    backend.close_and_remove().await?;
    Ok(())
}

pub(super) async fn run_all() -> StorageResult<()> {
    ai_ready_job_session_and_workflow_methods_use_durable_embedded_state().await?;
    event_crdt_and_kernel_queue_methods_are_atomic_and_durable().await?;
    graph_fact_promotion_rolls_back_on_mismatch_and_survives_reopen().await?;
    Ok(())
}

#[cfg(test)]
#[tokio::test]
async fn mt136_database_surface_proof_b() -> StorageResult<()> {
    run_all().await
}
