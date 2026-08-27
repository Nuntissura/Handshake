//! MT-136 real-engine proof for the `Database` methods not exercised by proofs A/B.
//!
//! The calls in this module are intentionally made through `dyn Database` over
//! isolated embedded RocksDB-backed SurrealDB stores. Each proof shuts the store
//! down, reopens the same directory, and asserts the resulting durable state.

use std::{collections::BTreeMap, sync::Arc};

use chrono::{Duration, Utc};
use serde_json::json;
use tokio::sync::Barrier;
use uuid::Uuid;

use super::mt136_proof_harness::{embedded_proof_backend, EmbeddedProofBackend};
use crate::{
    kernel::{KernelActor, KernelEventType, NewKernelEvent},
    storage::{
        AccessMode, AiJobMcpUpdate, BlockUpdate, Database, EntityRef, JobKind, JobMetrics,
        JobState, LoomBlockContentType, LoomBlockDerived, LoomEdgeCreatedBy, LoomEdgeType,
        LoomSearchFilters, LoomSearchV2Request, LoomViewFilters, LoomViewResponse, LoomViewType,
        ModelSessionState, NewAiJob, NewAsset, NewBlock, NewDocument, NewGovernanceCheckRun,
        NewLoomBlock, NewLoomEdge, NewModelSession, NewNodeExecution, NewSessionMessage,
        NewWorkspace, OperationType, PlannedOperation, SafetyMode, SessionCheckpoint,
        SessionMessageRole, StorageError, StorageResult, StructuredCollaborationStore,
        WriteContext,
    },
    workflows::locus,
};

fn ctx() -> WriteContext {
    WriteContext::human(Some("mt136-proof-c".to_owned()))
}

fn new_block(document_id: &str, id: Option<&str>, sequence: i64, content: &str) -> NewBlock {
    NewBlock {
        id: id.map(str::to_owned),
        document_id: document_id.to_owned(),
        kind: "paragraph".to_owned(),
        sequence,
        raw_content: content.to_owned(),
        display_content: Some(content.to_owned()),
        derived_content: Some(json!({"proof": "mt136-c"})),
        sensitivity: Some("low".to_owned()),
        exportable: Some(true),
    }
}

fn new_job() -> NewAiJob {
    NewAiJob {
        trace_id: Uuid::now_v7(),
        job_kind: JobKind::WorkflowRun,
        protocol_id: "mt136-proof-c".to_owned(),
        profile_id: "mt136-proof-c".to_owned(),
        capability_profile_id: "mt136-proof-c".to_owned(),
        access_mode: AccessMode::AnalysisOnly,
        safety_mode: SafetyMode::Strict,
        entity_refs: vec![EntityRef {
            entity_id: "mt136-proof-c".to_owned(),
            entity_kind: "proof".to_owned(),
        }],
        planned_operations: vec![PlannedOperation {
            op_type: OperationType::Read,
            target: EntityRef {
                entity_id: "mt136-proof-c".to_owned(),
                entity_kind: "proof".to_owned(),
            },
            description: Some("exercise uncovered Database methods".to_owned()),
        }],
        status_reason: "queued for MT-136 proof C".to_owned(),
        metrics: JobMetrics::zero(),
        job_inputs: Some(json!({"proof": "mt136-c"})),
    }
}

fn model_session(job_id: Uuid) -> NewModelSession {
    NewModelSession {
        session_id: "mt136-proof-c-session".to_owned(),
        parent_session_id: None,
        spawn_depth: 0,
        state: ModelSessionState::Active,
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
        agent: Some("mt136-proof-c".to_owned()),
        purpose: Some("prove uncovered embedded Database methods".to_owned()),
    }
}

fn kernel_event() -> NewKernelEvent {
    NewKernelEvent::builder(
        "KTR-mt136-proof-c",
        "mt136-proof-c-session",
        KernelEventType::ArtifactStored,
        KernelActor::System("mt136-proof-c".to_owned()),
    )
    .aggregate("mt136-proof", "surface-c")
    .idempotency_key("mt136-proof-c-event")
    .correlation_id("mt136-proof-c")
    .source_component("mt136_database_surface_proof_c")
    .payload(json!({"durable": true}))
    .build()
    .expect("valid MT-136 proof C event")
}

fn work_packet(wp_id: &str) -> locus::LocusCreateWpParams {
    locus::LocusCreateWpParams {
        wp_id: wp_id.to_owned(),
        title: "MT-136 proof C work packet".to_owned(),
        description: "Exercise the embedded Locus and structured collaboration methods".to_owned(),
        priority: 1,
        kind: locus::WorkPacketType::Test,
        phase: locus::WorkPacketPhase::Phase1,
        routing: locus::RoutingPolicy::GovStandard,
        task_packet_path: Some(format!(".GOV/task_packets/{wp_id}/packet.json")),
        assignee: Some("MT-136".to_owned()),
        labels: Some(vec!["surreal".to_owned(), "proof-c".to_owned()]),
        spec_session_id: Some("mt136-proof-c".to_owned()),
        reporter: "mt136-proof-c".to_owned(),
    }
}

fn micro_task(wp_id: &str, mt_id: &str) -> locus::TrackedMicroTask {
    locus::TrackedMicroTask {
        schema_id: String::new(),
        schema_version: String::new(),
        record_id: String::new(),
        record_kind: String::new(),
        project_profile_kind: locus::ProjectProfileKind::SoftwareDelivery,
        updated_at: Utc::now(),
        mirror_state: locus::MirrorSyncState::CanonicalOnly,
        authority_refs: vec![format!("authority:{wp_id}")],
        evidence_refs: vec!["mt136-proof-c".to_owned()],
        summary_record_path: None,
        profile_extension: None,
        mt_id: mt_id.to_owned(),
        wp_id: wp_id.to_owned(),
        name: "MT-136 proof C micro-task".to_owned(),
        scope: "Prove durable structured collaboration reads".to_owned(),
        files: locus::MicroTaskFiles {
            read: Vec::new(),
            modify: vec!["src/backend/handshake_core/src/storage/".to_owned()],
            create: Vec::new(),
        },
        done_criteria: vec!["close/reopen retains structured state".to_owned()],
        status: locus::MicroTaskStatus::Pending,
        active_session_ids: Vec::new(),
        iterations: Vec::new(),
        current_iteration: 0,
        max_iterations: 3,
        validation_result: None,
        escalation: locus::MicroTaskEscalation {
            current_level: 0,
            escalation_chain: Vec::new(),
            escalations_count: 0,
            drop_backs_count: 0,
        },
        started_at: None,
        completed_at: None,
        duration_ms: None,
        depends_on: Vec::new(),
        metadata: json!({"proof": "mt136-c"}),
    }
}

async fn reopen(backend: EmbeddedProofBackend) -> StorageResult<EmbeddedProofBackend> {
    backend.reopen().await
}

async fn core_crud_search_and_loom_methods_are_durable() -> StorageResult<()> {
    let mut backend = embedded_proof_backend().await?;
    let database = backend.database.clone();
    assert!(database.supports_locus_runtime());
    assert!(database.supports_structured_collab_artifacts());
    assert_eq!(database.loom_search_observability_tier(), 2);
    assert!(database.supports_loom_graph_filtering());
    assert_eq!(database.loom_traverse_graph_perf_target_ms(), 250);
    let durable_workspace = database
        .create_workspace(
            &ctx(),
            NewWorkspace {
                name: "MT-136 proof C durable workspace".to_owned(),
            },
        )
        .await?;
    let deleted_workspace = database
        .create_workspace(
            &ctx(),
            NewWorkspace {
                name: "MT-136 proof C deleted workspace".to_owned(),
            },
        )
        .await?;
    assert!(database
        .list_workspaces()
        .await?
        .iter()
        .any(|row| row.id == durable_workspace.id));
    assert_eq!(
        database
            .get_workspace(&durable_workspace.id)
            .await?
            .expect("created workspace")
            .name,
        durable_workspace.name
    );
    database
        .delete_workspace(&ctx(), &deleted_workspace.id)
        .await?;
    assert!(database
        .get_workspace(&deleted_workspace.id)
        .await?
        .is_none());

    let trace = database
        .test_fetch_mutation_traceability_row("workspaces", &durable_workspace.id)
        .await?;
    assert_eq!(trace.last_actor_kind, "HUMAN");
    assert_eq!(trace.last_actor_id.as_deref(), Some("mt136-proof-c"));
    assert!(!trace.edit_event_id.is_empty());

    let document = database
        .create_document(
            &ctx(),
            NewDocument {
                workspace_id: durable_workspace.id.clone(),
                title: "MT-136 proof C durable document".to_owned(),
            },
        )
        .await?;
    let deleted_document = database
        .create_document(
            &ctx(),
            NewDocument {
                workspace_id: durable_workspace.id.clone(),
                title: "MT-136 proof C deleted document".to_owned(),
            },
        )
        .await?;
    assert_eq!(database.get_document(&document.id).await?.id, document.id);
    assert_eq!(
        database.list_documents(&durable_workspace.id).await?.len(),
        2
    );
    database
        .delete_document(&ctx(), &deleted_document.id)
        .await?;

    let block = database
        .create_block(&ctx(), new_block(&document.id, None, 0, "proof C initial"))
        .await?;
    assert_eq!(
        database.get_block(&block.id).await?.raw_content,
        "proof C initial"
    );
    assert_eq!(database.get_blocks(&document.id).await?.len(), 1);
    database
        .update_block(
            &ctx(),
            &block.id,
            BlockUpdate {
                kind: None,
                sequence: Some(2),
                raw_content: Some("proof C updated".to_owned()),
                display_content: Some("proof C updated".to_owned()),
                derived_content: Some(json!({"updated": true})),
            },
        )
        .await?;
    assert_eq!(database.get_block(&block.id).await?.sequence, 2);
    let deleted_block = database
        .create_block(
            &ctx(),
            new_block(&document.id, None, 3, "proof C delete me"),
        )
        .await?;
    database.delete_block(&ctx(), &deleted_block.id).await?;
    let replaced = database
        .replace_blocks(
            &ctx(),
            &document.id,
            vec![new_block(
                &document.id,
                Some("mt136-proof-c-replaced"),
                0,
                "proof C replacement",
            )],
        )
        .await?;
    assert_eq!(replaced.len(), 1);
    assert_eq!(replaced[0].id, "mt136-proof-c-replaced");

    let asset_hash = "c".repeat(64);
    let asset = database
        .create_asset(
            &ctx(),
            NewAsset {
                workspace_id: durable_workspace.id.clone(),
                kind: "image".to_owned(),
                mime: "image/png".to_owned(),
                original_filename: Some("proof-c.png".to_owned()),
                content_hash: asset_hash.clone(),
                size_bytes: 128,
                width: Some(16),
                height: Some(8),
                classification: "low".to_owned(),
                exportable: true,
                is_proxy_of: None,
                proxy_asset_id: None,
            },
        )
        .await?;
    let second_asset = database
        .create_asset(
            &ctx(),
            NewAsset {
                workspace_id: durable_workspace.id.clone(),
                kind: "image".to_owned(),
                mime: "image/png".to_owned(),
                original_filename: Some("proof-c-second.png".to_owned()),
                content_hash: "d".repeat(64),
                size_bytes: 256,
                width: Some(32),
                height: Some(16),
                classification: "low".to_owned(),
                exportable: true,
                is_proxy_of: None,
                proxy_asset_id: None,
            },
        )
        .await?;
    assert_eq!(
        database
            .find_asset_by_content_hash(&durable_workspace.id, &asset_hash)
            .await?
            .expect("asset by hash")
            .asset_id,
        asset.asset_id
    );
    let collection = database
        .create_loom_collection(
            &ctx(),
            &durable_workspace.id,
            Some("proof C collection".to_owned()),
        )
        .await?;
    let collection = database
        .set_loom_collection_order(
            &ctx(),
            &durable_workspace.id,
            &collection.collection_id,
            &[second_asset.asset_id.clone(), asset.asset_id.clone()],
        )
        .await?;
    assert_eq!(collection.members[0].asset_id, second_asset.asset_id);
    assert_eq!(
        database
            .get_loom_collection(&durable_workspace.id, &collection.collection.collection_id)
            .await?
            .members
            .len(),
        2
    );

    let mut source_derived = LoomBlockDerived::default();
    source_derived.full_text_index = Some("mt136 unique searchable surface".to_owned());
    let source_hash = "e".repeat(64);
    let source = database
        .create_loom_block(
            &ctx(),
            NewLoomBlock {
                block_id: Some("mt136-proof-c-source".to_owned()),
                workspace_id: durable_workspace.id.clone(),
                content_type: LoomBlockContentType::Note,
                document_id: Some(document.id.clone()),
                asset_id: None,
                title: Some("MT136 unique searchable surface".to_owned()),
                original_filename: None,
                content_hash: Some(source_hash.clone()),
                pinned: true,
                journal_date: None,
                imported_at: None,
                derived: source_derived,
            },
        )
        .await?;
    let target = database
        .create_loom_block(
            &ctx(),
            NewLoomBlock {
                block_id: Some("mt136-proof-c-target".to_owned()),
                workspace_id: durable_workspace.id.clone(),
                content_type: LoomBlockContentType::Note,
                document_id: None,
                asset_id: None,
                title: Some("MT136 proof C target".to_owned()),
                original_filename: None,
                content_hash: Some("f".repeat(64)),
                pinned: false,
                journal_date: None,
                imported_at: None,
                derived: LoomBlockDerived::default(),
            },
        )
        .await?;
    assert_eq!(
        database
            .find_loom_block_by_content_hash(&durable_workspace.id, &source_hash)
            .await?
            .expect("loom block by hash")
            .block_id,
        source.block_id
    );
    let journal = database
        .get_or_create_daily_journal_block(&ctx(), &durable_workspace.id, "2026-08-23")
        .await?;
    assert_eq!(journal.journal_date.as_deref(), Some("2026-08-23"));
    let edge = database
        .create_loom_edge(
            &ctx(),
            NewLoomEdge {
                edge_id: Some("mt136-proof-c-edge".to_owned()),
                workspace_id: durable_workspace.id.clone(),
                source_block_id: source.block_id.clone(),
                target_block_id: target.block_id.clone(),
                edge_type: LoomEdgeType::Mention,
                created_by: LoomEdgeCreatedBy::User,
                crdt_site_id: None,
                source_anchor: None,
            },
        )
        .await?;
    assert_eq!(
        database
            .list_loom_edges_for_block(&durable_workspace.id, &source.block_id)
            .await?
            .len(),
        1
    );
    let traversal = database
        .traverse_graph(
            &durable_workspace.id,
            &source.block_id,
            2,
            &[LoomEdgeType::Mention],
        )
        .await?;
    assert!(traversal
        .iter()
        .any(|(block, depth)| block.block_id == target.block_id && *depth == 1));

    database
        .test_overwrite_loom_block_metrics(&durable_workspace.id, &source.block_id, 7, 8, 9)
        .await?;
    let overwritten = database
        .get_loom_block(&durable_workspace.id, &source.block_id)
        .await?;
    assert_eq!(
        (
            overwritten.derived.mention_count,
            overwritten.derived.tag_count,
            overwritten.derived.backlink_count,
        ),
        (7, 8, 9)
    );
    database
        .test_zero_workspace_loom_metrics(&durable_workspace.id)
        .await?;
    let zeroed = database
        .get_loom_block(&durable_workspace.id, &source.block_id)
        .await?;
    assert_eq!(
        (
            zeroed.derived.mention_count,
            zeroed.derived.tag_count,
            zeroed.derived.backlink_count,
        ),
        (0, 0, 0)
    );
    database
        .recompute_block_metrics(&durable_workspace.id, &source.block_id)
        .await?;
    database
        .recompute_all_metrics(&durable_workspace.id)
        .await?;
    let perf_start = database
        .test_insert_loom_traversal_perf_fixture(&durable_workspace.id, 3)
        .await?;
    assert_eq!(perf_start, "perf-block-00000");

    let LoomViewResponse::All { blocks } = database
        .query_loom_view(
            &durable_workspace.id,
            LoomViewType::All,
            LoomViewFilters::default(),
            100,
            0,
        )
        .await?
    else {
        panic!("expected all Loom view");
    };
    assert!(blocks.iter().any(|block| block.block_id == source.block_id));
    assert!(database
        .search_loom_blocks(
            &durable_workspace.id,
            "unique searchable surface",
            LoomSearchFilters::default(),
            20,
            0,
        )
        .await?
        .iter()
        .any(|hit| hit.block.block_id == source.block_id));
    assert!(database
        .search_loom_graph(
            &durable_workspace.id,
            "unique searchable surface",
            LoomSearchFilters::default(),
            20,
            0,
        )
        .await?
        .iter()
        .any(|hit| hit.ref_id == source.block_id));
    database
        .reindex_loom_block_search(
            &ctx(),
            &durable_workspace.id,
            &source.block_id,
            "mt136 unique searchable surface",
            None,
            None,
        )
        .await?;
    let v2 = database
        .loom_search_v2(
            &durable_workspace.id,
            LoomSearchV2Request {
                query: "unique searchable".to_owned(),
                limit: 20,
                ..LoomSearchV2Request::default()
            },
        )
        .await?;
    assert!(v2
        .hits
        .iter()
        .any(|hit| hit.block.block_id == source.block_id));

    assert_eq!(
        database
            .delete_loom_edge(&ctx(), &durable_workspace.id, &edge.edge_id)
            .await?
            .edge_id,
        edge.edge_id
    );
    database
        .delete_loom_block(&ctx(), &durable_workspace.id, &target.block_id)
        .await?;

    drop(database);
    backend = reopen(backend).await?;
    let database = backend.database.clone();
    assert!(database
        .get_workspace(&durable_workspace.id)
        .await?
        .is_some());
    assert!(database
        .get_workspace(&deleted_workspace.id)
        .await?
        .is_none());
    assert_eq!(
        database.list_documents(&durable_workspace.id).await?.len(),
        1
    );
    assert_eq!(
        database
            .get_block("mt136-proof-c-replaced")
            .await?
            .raw_content,
        "proof C replacement"
    );
    assert_eq!(
        database
            .get_loom_collection(&durable_workspace.id, &collection.collection.collection_id,)
            .await?
            .members[0]
            .asset_id,
        second_asset.asset_id
    );
    assert_eq!(
        database
            .find_loom_block_by_content_hash(&durable_workspace.id, &source_hash)
            .await?
            .expect("durable indexed Loom block")
            .block_id,
        source.block_id
    );
    assert!(matches!(
        database
            .get_loom_block(&durable_workspace.id, &target.block_id)
            .await,
        Err(StorageError::NotFound("loom_block"))
    ));
    assert!(database
        .list_loom_edges_for_block(&durable_workspace.id, &source.block_id)
        .await?
        .is_empty());
    backend.close_and_remove().await?;
    Ok(())
}

async fn job_session_kernel_workflow_and_governance_methods_are_durable() -> StorageResult<()> {
    let mut backend = embedded_proof_backend().await?;
    let database = backend.database.clone();
    let job = database.create_ai_job(new_job()).await?;
    assert_eq!(
        database.get_ai_job(&job.job_id.to_string()).await?.job_id,
        job.job_id
    );
    database
        .update_ai_job_mcp_fields(
            job.job_id,
            AiJobMcpUpdate {
                mcp_server_id: Some("mt136-proof-c-server".to_owned()),
                mcp_call_id: Some("mt136-proof-c-call".to_owned()),
                mcp_progress_token: Some("mt136-proof-c-token".to_owned()),
            },
        )
        .await?;
    let mcp = database.get_ai_job_mcp_fields(job.job_id).await?;
    assert_eq!(mcp.mcp_server_id.as_deref(), Some("mt136-proof-c-server"));
    assert_eq!(
        database
            .find_ai_job_id_by_mcp_progress_token("mt136-proof-c-token")
            .await?,
        Some(job.job_id)
    );

    database
        .upsert_model_session(model_session(job.job_id))
        .await?;
    assert_eq!(
        database
            .get_model_session("mt136-proof-c-session")
            .await?
            .job_id,
        Some(job.job_id)
    );
    let message = database
        .append_session_message(NewSessionMessage {
            message_id: Some("mt136-proof-c-message".to_owned()),
            session_id: "mt136-proof-c-session".to_owned(),
            role: SessionMessageRole::Assistant,
            content_hash: "a".repeat(64),
            content_artifact_id: "artifact:mt136-proof-c-message".to_owned(),
            token_count: Some(11),
            redacted: false,
            tool_call_id: None,
            attachments: vec!["artifact:mt136-proof-c-attachment".to_owned()],
        })
        .await?;
    assert_eq!(message.message_id, "mt136-proof-c-message");
    assert_eq!(
        database
            .list_session_messages("mt136-proof-c-session")
            .await?
            .len(),
        1
    );
    let checkpoint = database
        .create_session_checkpoint(SessionCheckpoint {
            checkpoint_id: "mt136-proof-c-checkpoint".to_owned(),
            session_id: "mt136-proof-c-session".to_owned(),
            timestamp: Utc::now(),
            session_state_json: json!({"state": "active"}).to_string(),
            message_thread_tail_id: message.message_id.clone(),
            pending_tool_calls_json: "[]".to_owned(),
            checkpoint_artifact_id: "artifact:mt136-proof-c-checkpoint".to_owned(),
        })
        .await?;
    assert_eq!(
        database
            .get_latest_session_checkpoint("mt136-proof-c-session")
            .await?
            .checkpoint_id,
        checkpoint.checkpoint_id
    );
    let closed = database
        .close_model_session(
            "mt136-proof-c-session",
            ModelSessionState::Completed,
            "MT-136 proof complete",
            "mt136-proof-c",
        )
        .await?;
    assert_eq!(
        closed.close_reason.as_deref(),
        Some("MT-136 proof complete")
    );

    let event = database.append_kernel_event(kernel_event()).await?;
    assert_eq!(
        database
            .list_kernel_events_for_aggregate("mt136-proof", "surface-c")
            .await?
            .len(),
        1
    );

    let stale_at = Utc::now() - Duration::minutes(5);
    let workflow = database
        .create_workflow_run(job.job_id, JobState::Running, Some(stale_at))
        .await?;
    database.heartbeat_workflow(workflow.id, stale_at).await?;
    assert!(database
        .find_stalled_workflows(60)
        .await?
        .iter()
        .any(|row| row.id == workflow.id));
    let node = database
        .create_workflow_node_execution(NewNodeExecution {
            workflow_run_id: workflow.id,
            node_id: "mt136-proof-c-node".to_owned(),
            node_type: "proof".to_owned(),
            status: JobState::Running,
            sequence: 0,
            input_payload: Some(json!({"proof": "mt136-c"})),
            started_at: Utc::now(),
        })
        .await?;
    let completed_node = database
        .update_workflow_node_execution_status(
            node.id,
            JobState::Completed,
            Some(json!({"durable": true})),
            None,
        )
        .await?;
    assert_eq!(completed_node.status, JobState::Completed);
    assert_eq!(
        database
            .list_workflow_node_executions(workflow.id)
            .await?
            .len(),
        1
    );

    let governance_session_id = Uuid::now_v7();
    let governance = database
        .create_governance_check_run(
            &ctx(),
            NewGovernanceCheckRun {
                check_id: Uuid::now_v7(),
                session_id: governance_session_id,
                check_name: "MT-136 proof C".to_owned(),
                check_kind: "embedded_surreal".to_owned(),
                descriptor_hash: "b".repeat(64),
                result_status: "passed".to_owned(),
                checks_duration_ms: 12,
                evidence_artifact_id: Some("artifact:mt136-proof-c".to_owned()),
                evidence_artifact_content_hash: Some("c".repeat(64)),
            },
        )
        .await?;
    assert_eq!(
        database
            .list_governance_check_runs(governance_session_id)
            .await?
            .len(),
        1
    );

    let old_created_at = Utc::now() - Duration::days(30);
    database
        .test_update_ai_job_metadata(job.job_id, "completed", old_created_at, false)
        .await?;
    let prune = database
        .prune_ai_jobs(Utc::now() - Duration::days(1), 0, true)
        .await?;
    assert!(prune.items_scanned >= 1);
    assert!(prune.items_pruned >= 1);

    drop(database);
    backend = reopen(backend).await?;
    let database = backend.database.clone();
    assert_eq!(
        database
            .get_ai_job_mcp_fields(job.job_id)
            .await?
            .mcp_progress_token
            .as_deref(),
        Some("mt136-proof-c-token")
    );
    let durable_session = database.get_model_session("mt136-proof-c-session").await?;
    assert_eq!(durable_session.state, ModelSessionState::Completed);
    assert_eq!(
        durable_session.closed_by_actor.as_deref(),
        Some("mt136-proof-c")
    );
    assert_eq!(
        database
            .get_latest_session_checkpoint("mt136-proof-c-session")
            .await?
            .checkpoint_id,
        checkpoint.checkpoint_id
    );
    assert_eq!(
        database
            .list_session_messages("mt136-proof-c-session")
            .await?[0]
            .message_id,
        message.message_id
    );
    assert_eq!(
        database
            .list_kernel_events_for_aggregate("mt136-proof", "surface-c")
            .await?[0]
            .event_id,
        event.event_id
    );
    assert_eq!(
        database.list_workflow_node_executions(workflow.id).await?[0].status,
        JobState::Completed
    );
    assert_eq!(
        database
            .list_governance_check_runs(governance_session_id)
            .await?[0]
            .id,
        governance.id
    );
    assert_eq!(
        database.get_ai_job(&job.job_id.to_string()).await?.job_id,
        job.job_id,
        "dry-run pruning must leave the durable job intact"
    );
    backend.close_and_remove().await?;
    Ok(())
}

async fn locus_and_structured_collaboration_methods_are_durable() -> StorageResult<()> {
    let mut backend = embedded_proof_backend().await?;
    let database = backend.database.clone();
    let wp_id = "WP-MT136-PROOF-C";
    let mt_id = "MT-MT136-PROOF-C";
    let created = database
        .execute_locus_operation(locus::LocusOperation::CreateWp(work_packet(wp_id)))
        .await?;
    assert_eq!(created["wp_id"], wp_id);
    database
        .execute_locus_operation(locus::LocusOperation::RegisterMts(
            locus::LocusRegisterMtsParams {
                wp_id: wp_id.to_owned(),
                micro_tasks: vec![micro_task(wp_id, mt_id)],
            },
        ))
        .await?;
    let (expected_version, _, _) = database
        .locus_task_board_get_status_and_metadata(wp_id)
        .await?
        .expect("created MT-136 proof work packet");
    database
        .locus_task_board_update_work_packet(
            expected_version,
            "READY",
            "READY",
            &Utc::now().to_rfc3339(),
            r#"{"proof":"mt136-c"}"#,
            wp_id,
        )
        .await?;
    assert_eq!(
        database
            .structured_collab_work_packet_row(wp_id)
            .await?
            .expect("created structured work packet")
            .wp_id,
        wp_id
    );
    assert_eq!(
        database.structured_collab_work_packet_rows().await?.len(),
        1
    );
    assert_eq!(
        database
            .structured_collab_micro_task_status_rows(wp_id)
            .await?,
        vec![(mt_id.to_owned(), "pending".to_owned())]
    );
    assert!(database
        .structured_collab_micro_task_metadata(wp_id, mt_id)
        .await?
        .expect("registered micro-task metadata")
        .contains("mt136-c"));
    assert_eq!(
        database
            .structured_collab_micro_task_rows(wp_id)
            .await?
            .len(),
        1
    );

    let updated = database
        .execute_locus_operation(locus::LocusOperation::UpdateWp(
            locus::LocusUpdateWpParams {
                wp_id: wp_id.to_owned(),
                updates: BTreeMap::from([
                    ("status".to_owned(), json!("ready")),
                    ("task_board_status".to_owned(), json!("READY")),
                ]),
                source: Some("mt136-proof-c".to_owned()),
            },
        ))
        .await?;
    assert_eq!(updated["wp_id"], wp_id);

    drop(database);
    backend = reopen(backend).await?;
    let database = backend.database.clone();
    let durable = database
        .execute_locus_operation(locus::LocusOperation::GetWpStatus(
            locus::LocusGetWpStatusParams {
                wp_id: wp_id.to_owned(),
            },
        ))
        .await?;
    assert_eq!(durable["status"], "ready");
    assert_eq!(
        database.structured_collab_work_packet_rows().await?.len(),
        1
    );
    assert_eq!(
        database
            .structured_collab_micro_task_rows(wp_id)
            .await?
            .len(),
        1
    );
    assert!(database
        .structured_collab_micro_task_metadata(wp_id, mt_id)
        .await?
        .expect("durable micro-task metadata")
        .contains("mt136-c"));
    backend.close_and_remove().await?;
    Ok(())
}

async fn assert_stale_task_board_sync_is_rejected(
    database: Arc<dyn Database>,
    wp_id: &str,
    mutation: locus::LocusOperation,
) -> StorageResult<()> {
    let (expected_version, _, metadata_raw) = database
        .locus_task_board_get_status_and_metadata(wp_id)
        .await?
        .expect("work packet must exist before the concurrency proof");
    let mut stale_metadata: serde_json::Value = serde_json::from_str(&metadata_raw)?;
    stale_metadata["task_board_token"] = json!("stale-task-board-token");

    let stale_writer_ready = Arc::new(Barrier::new(2));
    let mutation_committed = Arc::new(Barrier::new(2));
    let stale_writer = {
        let database = database.clone();
        let wp_id = wp_id.to_owned();
        let stale_metadata = serde_json::to_string(&stale_metadata)?;
        let stale_writer_ready = stale_writer_ready.clone();
        let mutation_committed = mutation_committed.clone();
        tokio::spawn(async move {
            stale_writer_ready.wait().await;
            mutation_committed.wait().await;
            database
                .locus_task_board_update_work_packet(
                    expected_version,
                    "ready",
                    "READY",
                    &Utc::now().to_rfc3339(),
                    &stale_metadata,
                    &wp_id,
                )
                .await
        })
    };

    stale_writer_ready.wait().await;
    database.execute_locus_operation(mutation).await?;
    mutation_committed.wait().await;
    let stale_result = stale_writer
        .await
        .expect("stale task-board writer task must not panic");
    match stale_result {
        Err(StorageError::Conflict(message)) => {
            assert_eq!(message, "work_packet changed concurrently")
        }
        other => panic!("stale task-board synchronization was not rejected: {other:?}"),
    }
    Ok(())
}

async fn task_board_sync_cas_preserves_concurrent_locus_mutations() -> StorageResult<()> {
    let mut backend = embedded_proof_backend().await?;
    let database = backend.database.clone();
    let gate_wp_id = "WP-MT136-TASK-BOARD-CAS-GATE";
    let update_wp_id = "WP-MT136-TASK-BOARD-CAS-UPDATE";
    let close_wp_id = "WP-MT136-TASK-BOARD-CAS-CLOSE";

    for wp_id in [gate_wp_id, update_wp_id, close_wp_id] {
        database
            .execute_locus_operation(locus::LocusOperation::CreateWp(work_packet(wp_id)))
            .await?;
    }

    assert_stale_task_board_sync_is_rejected(
        database.clone(),
        gate_wp_id,
        locus::LocusOperation::GateWp(locus::LocusGateWpParams {
            wp_id: gate_wp_id.to_owned(),
            gate: locus::LocusGateKind::PreWork,
            result: locus::GateStatus {
                status: locus::GateStatusKind::Pass,
                validated_at: Some(Utc::now()),
                validated_by: Some("mt136-cas-proof".to_owned()),
                notes: Some("must survive stale task-board synchronization".to_owned()),
                validation_report_ref: Some(json!({"proof": "mt136-task-board-cas"})),
            },
        }),
    )
    .await?;
    let gate_row = database
        .structured_collab_work_packet_row(gate_wp_id)
        .await?
        .expect("gated work packet must remain present");
    let gate_metadata: serde_json::Value = serde_json::from_str(&gate_row.metadata)?;
    assert_eq!(gate_row.version, 2);
    assert_eq!(gate_metadata["gates"]["pre_work"]["status"], "pass");
    assert_eq!(gate_metadata.get("task_board_token"), None);

    assert_stale_task_board_sync_is_rejected(
        database.clone(),
        update_wp_id,
        locus::LocusOperation::UpdateWp(locus::LocusUpdateWpParams {
            wp_id: update_wp_id.to_owned(),
            updates: BTreeMap::from([
                ("status".to_owned(), json!("blocked")),
                ("task_board_status".to_owned(), json!("BLOCKED")),
            ]),
            source: Some("mt136-cas-proof".to_owned()),
        }),
    )
    .await?;
    let update_row = database
        .structured_collab_work_packet_row(update_wp_id)
        .await?
        .expect("updated work packet must remain present");
    assert_eq!(update_row.version, 2);
    assert_eq!(update_row.status, "blocked");
    assert_eq!(update_row.task_board_status, "BLOCKED");
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&update_row.metadata)?.get("task_board_token"),
        None
    );

    assert_stale_task_board_sync_is_rejected(
        database.clone(),
        close_wp_id,
        locus::LocusOperation::CloseWp(locus::LocusCloseWpParams {
            wp_id: close_wp_id.to_owned(),
        }),
    )
    .await?;
    let close_row = database
        .structured_collab_work_packet_row(close_wp_id)
        .await?
        .expect("closed work packet must remain present");
    assert_eq!(close_row.version, 2);
    assert_eq!(close_row.status, "done");
    assert_eq!(close_row.task_board_status, "DONE");
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&close_row.metadata)?.get("task_board_token"),
        None
    );

    drop(database);
    backend = reopen(backend).await?;
    for (wp_id, expected_status, expected_task_board_status) in [
        (gate_wp_id, "stub", "STUB"),
        (update_wp_id, "blocked", "BLOCKED"),
        (close_wp_id, "done", "DONE"),
    ] {
        let durable = backend
            .database
            .structured_collab_work_packet_row(wp_id)
            .await?
            .expect("CAS-protected work packet must survive reopen");
        assert_eq!(durable.version, 2);
        assert_eq!(durable.status, expected_status);
        assert_eq!(durable.task_board_status, expected_task_board_status);
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(&durable.metadata)?.get("task_board_token"),
            None
        );
    }
    let durable_gate = backend
        .database
        .structured_collab_work_packet_row(gate_wp_id)
        .await?
        .expect("gated work packet must survive reopen");
    let durable_gate_metadata: serde_json::Value = serde_json::from_str(&durable_gate.metadata)?;
    assert_eq!(durable_gate_metadata["gates"]["pre_work"]["status"], "pass");

    backend.close_and_remove().await?;
    Ok(())
}

pub(crate) async fn run_all() -> StorageResult<()> {
    eprintln!("MT136_PROOF_STEP_START database_surface_c.core_crud_search_loom");
    core_crud_search_and_loom_methods_are_durable().await?;
    eprintln!("MT136_PROOF_STEP_PASS database_surface_c.core_crud_search_loom");
    eprintln!("MT136_PROOF_STEP_START database_surface_c.job_session_kernel_workflow");
    job_session_kernel_workflow_and_governance_methods_are_durable().await?;
    eprintln!("MT136_PROOF_STEP_PASS database_surface_c.job_session_kernel_workflow");
    eprintln!("MT136_PROOF_STEP_START database_surface_c.locus_structured");
    locus_and_structured_collaboration_methods_are_durable().await?;
    eprintln!("MT136_PROOF_STEP_PASS database_surface_c.locus_structured");
    eprintln!("MT136_PROOF_STEP_START database_surface_c.task_board_cas");
    task_board_sync_cas_preserves_concurrent_locus_mutations().await?;
    eprintln!("MT136_PROOF_STEP_PASS database_surface_c.task_board_cas");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn uncovered_database_methods_use_real_durable_surreal_state() -> StorageResult<()> {
        run_all().await
    }
}
