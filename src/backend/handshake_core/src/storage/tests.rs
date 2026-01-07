#[allow(unused_imports)]
use super::{
    postgres::PostgresDatabase, sqlite::SqliteDatabase, AccessMode, BlockUpdate, Database,
    DefaultStorageGuard, EntityRef, GuardError, JobKind, JobMetrics, JobState, JobStatusUpdate,
    NewAiJob, NewBlock, NewCanvas, NewCanvasEdge, NewCanvasNode, NewDocument, NewNodeExecution,
    NewWorkspace, OperationType, PlannedOperation, SafetyMode, StorageError, StorageGuard,
    StorageResult, WriteContext,
};
#[cfg(test)]
use chrono::{Duration, Utc};
use serde_json::json;
use std::sync::Arc;
use uuid::Uuid;

/// Build an in-memory SQLite backend for test validation.
#[allow(dead_code)]
pub async fn sqlite_backend() -> StorageResult<Arc<dyn super::Database>> {
    let db = SqliteDatabase::connect("sqlite::memory:", 5).await?;
    db.run_migrations().await?;
    Ok(db.into_arc())
}

/// Build a PostgreSQL backend from POSTGRES_TEST_URL for test validation.
#[allow(dead_code)]
pub async fn postgres_backend_from_env() -> StorageResult<Arc<dyn super::Database>> {
    let url = std::env::var("POSTGRES_TEST_URL")
        .map_err(|_| StorageError::Validation("POSTGRES_TEST_URL not set for postgres tests"))?;
    let db = PostgresDatabase::connect(&url, 5).await?;
    db.run_migrations().await?;
    Ok(db.into_arc())
}

/// Runs the shared storage conformance suite against the provided backend.
#[allow(dead_code)]
pub async fn run_storage_conformance(db: Arc<dyn super::Database>) -> StorageResult<()> {
    db.ping().await?;

    let ctx = WriteContext::human(None);

    let workspace = db
        .create_workspace(
            &ctx,
            NewWorkspace {
                name: format!("ws-{}", Uuid::new_v4()),
            },
        )
        .await?;

    let fetched = db
        .get_workspace(&workspace.id)
        .await?
        .ok_or(StorageError::NotFound("workspace"))?;
    assert_eq!(fetched.id, workspace.id);
    assert_eq!(fetched.name, workspace.name);

    let workspaces = db.list_workspaces().await?;
    assert!(workspaces.iter().any(|w| w.id == workspace.id));

    let document = db
        .create_document(
            &ctx,
            NewDocument {
                workspace_id: workspace.id.clone(),
                title: "Doc A".into(),
            },
        )
        .await?;

    let documents = db.list_documents(&workspace.id).await?;
    assert!(documents.iter().any(|d| d.id == document.id));

    let mut block = db
        .create_block(
            &ctx,
            NewBlock {
                id: None,
                document_id: document.id.clone(),
                kind: "paragraph".into(),
                sequence: 1,
                raw_content: "hello".into(),
                display_content: None,
                derived_content: Some(json!({"k": 1})),
                sensitivity: None,
                exportable: None,
            },
        )
        .await?;
    assert_eq!(block.display_content, "hello");
    assert_eq!(block.derived_content["k"], 1);

    db.update_block(
        &ctx,
        &block.id,
        BlockUpdate {
            kind: None,
            sequence: Some(2),
            raw_content: Some("updated".into()),
            display_content: Some("view".into()),
            derived_content: Some(json!({"k": 2})),
        },
    )
    .await?;

    block = db.get_block(&block.id).await?;
    assert_eq!(block.sequence, 2);
    assert_eq!(block.raw_content, "updated");
    assert_eq!(block.display_content, "view");
    assert_eq!(block.derived_content["k"], 2);

    let replacement_blocks = db
        .replace_blocks(
            &ctx,
            &document.id,
            vec![
                NewBlock {
                    id: Some(Uuid::new_v4().to_string()),
                    document_id: document.id.clone(),
                    kind: "p".into(),
                    sequence: 1,
                    raw_content: "b1".into(),
                    display_content: None,
                    derived_content: None,
                    sensitivity: None,
                    exportable: None,
                },
                NewBlock {
                    id: Some(Uuid::new_v4().to_string()),
                    document_id: document.id.clone(),
                    kind: "p".into(),
                    sequence: 2,
                    raw_content: "b2".into(),
                    display_content: Some("b2".into()),
                    derived_content: Some(json!({"k": 3})),
                    sensitivity: None,
                    exportable: None,
                },
            ],
        )
        .await?;
    assert_eq!(replacement_blocks.len(), 2);
    assert_eq!(replacement_blocks[0].sequence, 1);
    assert_eq!(replacement_blocks[1].display_content, "b2");
    assert_eq!(replacement_blocks[1].derived_content["k"], 3);

    db.delete_block(&ctx, &replacement_blocks[0].id).await?;
    let remaining = db.get_blocks(&document.id).await?;
    assert_eq!(remaining.len(), 1);
    assert_eq!(remaining[0].id, replacement_blocks[1].id);

    let canvas = db
        .create_canvas(
            &ctx,
            NewCanvas {
                workspace_id: workspace.id.clone(),
                title: "Canvas".into(),
            },
        )
        .await?;

    let canvases = db.list_canvases(&workspace.id).await?;
    assert!(canvases.iter().any(|c| c.id == canvas.id));

    let node_a = Uuid::new_v4().to_string();
    let node_b = Uuid::new_v4().to_string();
    let graph = db
        .update_canvas_graph(
            &ctx,
            &canvas.id,
            vec![
                NewCanvasNode {
                    id: Some(node_a.clone()),
                    kind: "text".into(),
                    position_x: 1.0,
                    position_y: 2.0,
                    data: Some(json!({"k": "v"})),
                },
                NewCanvasNode {
                    id: Some(node_b.clone()),
                    kind: "text".into(),
                    position_x: 3.0,
                    position_y: 4.0,
                    data: Some(json!({"k2": "v2"})),
                },
            ],
            vec![NewCanvasEdge {
                id: Some(Uuid::new_v4().to_string()),
                from_node_id: node_a.clone(),
                to_node_id: node_b.clone(),
                kind: "link".into(),
            }],
        )
        .await?;
    assert_eq!(graph.nodes.len(), 2);
    assert_eq!(graph.nodes[0].position_x, 1.0);
    assert_eq!(graph.nodes[0].data["k"], "v");
    assert_eq!(graph.edges.len(), 1);
    assert_eq!(graph.edges[0].from_node_id, node_a);

    let loaded_graph = db.get_canvas_with_graph(&canvas.id).await?;
    assert_eq!(loaded_graph.canvas.id, canvas.id);
    assert_eq!(loaded_graph.nodes.len(), 2);
    assert_eq!(loaded_graph.edges.len(), 1);

    let job = db
        .create_ai_job(NewAiJob {
            trace_id: Uuid::new_v4(),
            job_kind: JobKind::WorkflowRun,
            protocol_id: "p1".into(),
            profile_id: "profile1".into(),
            capability_profile_id: "cap1".into(),
            access_mode: AccessMode::AnalysisOnly,
            safety_mode: SafetyMode::Normal,
            entity_refs: vec![EntityRef {
                entity_id: "doc-1".into(),
                entity_kind: "document".into(),
            }],
            planned_operations: vec![PlannedOperation {
                op_type: OperationType::Read,
                target: EntityRef {
                    entity_id: "doc-1".into(),
                    entity_kind: "document".into(),
                },
                description: None,
            }],
            status_reason: "queued".to_string(),
            metrics: JobMetrics::zero(),
            job_inputs: Some(json!({"input": 1})),
        })
        .await?;
    let job_loaded = db.get_ai_job(&job.job_id.to_string()).await?;
    assert!(matches!(job_loaded.job_kind, JobKind::WorkflowRun));

    db.update_ai_job_status(JobStatusUpdate {
        job_id: job.job_id,
        state: JobState::Running,
        error_message: None,
        status_reason: "running".into(),
        metrics: Some(JobMetrics::zero()),
        workflow_run_id: None,
        trace_id: None,
        job_outputs: None,
    })
    .await?;
    db.set_job_outputs(&job.job_id.to_string(), Some(json!({"out": true})))
        .await?;
    let job_final = db.get_ai_job(&job.job_id.to_string()).await?;
    assert!(matches!(job_final.state, JobState::Running));
    let outputs = job_final
        .job_outputs
        .ok_or(StorageError::NotFound("job_outputs"))?;
    assert_eq!(outputs["out"], true);

    let run = db
        .create_workflow_run(job.job_id, JobState::Queued, None)
        .await?;
    let updated_run = db
        .update_workflow_run_status(run.id, JobState::Failed, Some("boom".into()))
        .await?;
    assert!(matches!(updated_run.status, JobState::Failed));

    let guard_ctx = WriteContext::ai(
        Some("tester".into()),
        Some(Uuid::new_v4()),
        Some(Uuid::new_v4()),
    );
    let guard = db
        .validate_write_with_guard(&guard_ctx, "resource-1")
        .await?;
    assert_eq!(guard.actor_kind.as_str(), "AI");
    assert_eq!(guard.resource_id, "resource-1");

    db.delete_document(&ctx, &document.id).await?;
    db.delete_canvas(&ctx, &canvas.id).await?;
    db.delete_workspace(&ctx, &workspace.id).await?;

    Ok(())
}

#[tokio::test]
async fn guard_blocks_ai_without_context() {
    let guard = DefaultStorageGuard;
    let ctx = WriteContext::ai(Some("ai-writer".into()), None, None);
    let result = guard.validate_write(&ctx, "res-123").await;
    assert!(matches!(result, Err(GuardError::SilentEdit)));
}

#[tokio::test]
async fn workflow_node_execution_persists_inputs_and_outputs() -> StorageResult<()> {
    let db = sqlite_backend().await?;
    let job = db
        .create_ai_job(NewAiJob {
            trace_id: Uuid::new_v4(),
            job_kind: JobKind::WorkflowRun,
            protocol_id: "p1".into(),
            profile_id: "profile1".into(),
            capability_profile_id: "cap1".into(),
            access_mode: AccessMode::AnalysisOnly,
            safety_mode: SafetyMode::Normal,
            entity_refs: Vec::new(),
            planned_operations: Vec::new(),
            status_reason: "queued".to_string(),
            metrics: JobMetrics::zero(),
            job_inputs: Some(json!({"input": true})),
        })
        .await?;
    let run = db
        .create_workflow_run(job.job_id, JobState::Running, None)
        .await?;

    let exec = db
        .create_workflow_node_execution(NewNodeExecution {
            workflow_run_id: run.id,
            node_id: "node-1".into(),
            node_type: "test".into(),
            status: JobState::Running,
            sequence: 1,
            input_payload: Some(json!({"input": true})),
            started_at: Utc::now(),
        })
        .await?;

    assert!(matches!(exec.status, JobState::Running));
    assert_eq!(exec.node_id, "node-1");

    let updated = db
        .update_workflow_node_execution_status(
            exec.id,
            JobState::Completed,
            Some(json!({"output": 42})),
            None,
        )
        .await?;
    assert!(matches!(updated.status, JobState::Completed));
    assert_eq!(
        updated
            .output_payload
            .as_ref()
            .and_then(|v| v.get("output"))
            .and_then(|v| v.as_i64()),
        Some(42)
    );

    let executions = db.list_workflow_node_executions(run.id).await?;
    assert_eq!(executions.len(), 1);
    Ok(())
}

#[tokio::test]
async fn stalled_workflows_are_detected_by_heartbeat() -> StorageResult<()> {
    let db = sqlite_backend().await?;
    let job = db
        .create_ai_job(NewAiJob {
            trace_id: Uuid::new_v4(),
            job_kind: JobKind::WorkflowRun,
            protocol_id: "p1".into(),
            profile_id: "profile1".into(),
            capability_profile_id: "cap1".into(),
            access_mode: AccessMode::AnalysisOnly,
            safety_mode: SafetyMode::Normal,
            entity_refs: Vec::new(),
            planned_operations: Vec::new(),
            status_reason: "queued".to_string(),
            metrics: JobMetrics::zero(),
            job_inputs: Some(json!({"input": true})),
        })
        .await?;
    let stale_time = Utc::now() - Duration::seconds(120);
    let run = db
        .create_workflow_run(job.job_id, JobState::Running, Some(stale_time))
        .await?;

    let stalled = db.find_stalled_workflows(60).await?;
    assert!(
        stalled
            .iter()
            .any(|r| r.id == run.id && matches!(r.status, JobState::Running)),
        "expected running workflow to be reported as stalled candidate"
    );

    // Refresh heartbeat and confirm it no longer appears stale
    db.heartbeat_workflow(run.id, Utc::now()).await?;
    let after = db.find_stalled_workflows(60).await?;
    assert!(!after.iter().any(|r| r.id == run.id));
    Ok(())
}
