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
#[cfg(test)]
use sqlx::{Connection, Row};
use std::sync::Arc;
use uuid::Uuid;

#[cfg(test)]
const NIL_EDIT_EVENT_ID: &str = "00000000-0000-0000-0000-000000000000";

#[cfg(test)]
fn postgres_test_url() -> Option<String> {
    std::env::var("POSTGRES_TEST_URL").ok()
}

#[cfg(test)]
fn assert_metadata_matches_ctx(
    row_actor_kind: &str,
    row_actor_id: Option<String>,
    row_job_id: Option<String>,
    row_workflow_id: Option<String>,
    row_edit_event_id: &str,
    ctx: &WriteContext,
) {
    assert_eq!(row_actor_kind, ctx.actor_kind.as_str());
    assert_eq!(row_actor_id.as_deref(), ctx.actor_id.as_deref());

    let expected_job_id = ctx.job_id.map(|v| v.to_string());
    let expected_workflow_id = ctx.workflow_id.map(|v| v.to_string());
    assert_eq!(row_job_id.as_deref(), expected_job_id.as_deref());
    assert_eq!(row_workflow_id.as_deref(), expected_workflow_id.as_deref());

    assert_ne!(row_edit_event_id, NIL_EDIT_EVENT_ID);
    let Ok(parsed) = Uuid::parse_str(row_edit_event_id) else {
        unreachable!("edit_event_id must be valid UUID");
    };
    assert_ne!(parsed, Uuid::nil());
}

#[cfg(test)]
async fn sqlite_user_table_names(conn: &mut sqlx::SqliteConnection) -> StorageResult<Vec<String>> {
    let rows = sqlx::query(
        r#"
        SELECT name
        FROM sqlite_master
        WHERE type = 'table'
          AND name NOT LIKE 'sqlite_%'
          AND name != '_sqlx_migrations'
        ORDER BY name
        "#,
    )
    .fetch_all(conn)
    .await?;

    Ok(rows
        .into_iter()
        .map(|row| row.get::<String, _>("name"))
        .collect())
}

#[cfg(test)]
async fn sqlite_schema_fingerprint(
    conn: &mut sqlx::SqliteConnection,
) -> StorageResult<Vec<String>> {
    let rows = sqlx::query(
        r#"
        SELECT type, name, tbl_name, COALESCE(sql, '') as sql
        FROM sqlite_master
        WHERE name NOT LIKE 'sqlite_%'
          AND name != '_sqlx_migrations'
        ORDER BY type, name
        "#,
    )
    .fetch_all(conn)
    .await?;

    Ok(rows
        .into_iter()
        .map(|row| {
            let kind = row.get::<String, _>("type");
            let name = row.get::<String, _>("name");
            let table = row.get::<String, _>("tbl_name");
            let sql = row.get::<String, _>("sql");
            format!("{kind}|{name}|{table}|{sql}")
        })
        .collect())
}

#[cfg(test)]
async fn postgres_schema_fingerprint(conn: &mut sqlx::PgConnection) -> StorageResult<Vec<String>> {
    let mut fingerprint = Vec::new();

    let column_rows = sqlx::query(
        r#"
        SELECT table_name, column_name, data_type, is_nullable, COALESCE(column_default, '') as column_default
        FROM information_schema.columns
        WHERE table_schema = current_schema()
          AND table_name <> '_sqlx_migrations'
        ORDER BY table_name, ordinal_position
        "#,
    )
    .fetch_all(&mut *conn)
    .await?;

    for row in column_rows {
        let table_name = row.get::<String, _>("table_name");
        let column_name = row.get::<String, _>("column_name");
        let data_type = row.get::<String, _>("data_type");
        let is_nullable = row.get::<String, _>("is_nullable");
        let column_default = row.get::<String, _>("column_default");
        fingerprint.push(format!(
            "COL|{table_name}|{column_name}|{data_type}|{is_nullable}|{column_default}"
        ));
    }

    let index_rows = sqlx::query(
        r#"
        SELECT indexname, indexdef
        FROM pg_indexes
        WHERE schemaname = current_schema()
          AND tablename <> '_sqlx_migrations'
        ORDER BY indexname
        "#,
    )
    .fetch_all(&mut *conn)
    .await?;

    for row in index_rows {
        let indexname = row.get::<String, _>("indexname");
        let indexdef = row.get::<String, _>("indexdef");
        fingerprint.push(format!("IDX|{indexname}|{indexdef}"));
    }

    Ok(fingerprint)
}

#[cfg(test)]
async fn postgres_user_table_names(conn: &mut sqlx::PgConnection) -> StorageResult<Vec<String>> {
    let rows = sqlx::query(
        r#"
        SELECT table_name
        FROM information_schema.tables
        WHERE table_schema = current_schema()
          AND table_type = 'BASE TABLE'
          AND table_name <> '_sqlx_migrations'
        ORDER BY table_name
        "#,
    )
    .fetch_all(&mut *conn)
    .await?;

    Ok(rows
        .into_iter()
        .map(|row| row.get::<String, _>("table_name"))
        .collect())
}

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
async fn sqlite_rejects_ai_writes_without_context_with_hsk_403_silent_edit() -> StorageResult<()> {
    let db = sqlite_backend().await?;
    let ctx = WriteContext::ai(Some("ai-writer".into()), None, None);

    let res = db
        .create_workspace(
            &ctx,
            NewWorkspace {
                name: format!("ws-{}", Uuid::new_v4()),
            },
        )
        .await;
    assert!(matches!(
        res,
        Err(StorageError::Guard("HSK-403-SILENT-EDIT"))
    ));

    Ok(())
}

#[tokio::test]
async fn sqlite_persists_mutation_traceability_metadata_on_writes() -> StorageResult<()> {
    let db = sqlite_backend().await?;
    let sqlite = db
        .as_any()
        .downcast_ref::<SqliteDatabase>()
        .ok_or(StorageError::Validation("expected sqlite backend"))?;
    let pool = sqlite.pool();

    let contexts = vec![
        WriteContext::human(Some("human-1".into())),
        WriteContext::system(Some("system-1".into())),
        WriteContext::ai(
            Some("ai-1".into()),
            Some(Uuid::new_v4()),
            Some(Uuid::new_v4()),
        ),
    ];

    for ctx in contexts {
        let workspace = db
            .create_workspace(
                &ctx,
                NewWorkspace {
                    name: format!("ws-{}", Uuid::new_v4()),
                },
            )
            .await?;

        let workspace_row = sqlx::query(
            r#"
            SELECT last_actor_kind, last_actor_id, last_job_id, last_workflow_id, edit_event_id
            FROM workspaces
            WHERE id = ?
            "#,
        )
        .bind(&workspace.id)
        .fetch_one(pool)
        .await?;
        assert_metadata_matches_ctx(
            &workspace_row.get::<String, _>("last_actor_kind"),
            workspace_row.get::<Option<String>, _>("last_actor_id"),
            workspace_row.get::<Option<String>, _>("last_job_id"),
            workspace_row.get::<Option<String>, _>("last_workflow_id"),
            &workspace_row.get::<String, _>("edit_event_id"),
            &ctx,
        );

        let document = db
            .create_document(
                &ctx,
                NewDocument {
                    workspace_id: workspace.id.clone(),
                    title: format!("doc-{}", Uuid::new_v4()),
                },
            )
            .await?;

        let document_row = sqlx::query(
            r#"
            SELECT last_actor_kind, last_actor_id, last_job_id, last_workflow_id, edit_event_id
            FROM documents
            WHERE id = ?
            "#,
        )
        .bind(&document.id)
        .fetch_one(pool)
        .await?;
        assert_metadata_matches_ctx(
            &document_row.get::<String, _>("last_actor_kind"),
            document_row.get::<Option<String>, _>("last_actor_id"),
            document_row.get::<Option<String>, _>("last_job_id"),
            document_row.get::<Option<String>, _>("last_workflow_id"),
            &document_row.get::<String, _>("edit_event_id"),
            &ctx,
        );

        let block = db
            .create_block(
                &ctx,
                NewBlock {
                    id: None,
                    document_id: document.id.clone(),
                    kind: "paragraph".into(),
                    sequence: 1,
                    raw_content: "hello".into(),
                    display_content: None,
                    derived_content: None,
                    sensitivity: None,
                    exportable: None,
                },
            )
            .await?;

        let block_row = sqlx::query(
            r#"
            SELECT last_actor_kind, last_actor_id, last_job_id, last_workflow_id, edit_event_id
            FROM blocks
            WHERE id = ?
            "#,
        )
        .bind(&block.id)
        .fetch_one(pool)
        .await?;
        assert_metadata_matches_ctx(
            &block_row.get::<String, _>("last_actor_kind"),
            block_row.get::<Option<String>, _>("last_actor_id"),
            block_row.get::<Option<String>, _>("last_job_id"),
            block_row.get::<Option<String>, _>("last_workflow_id"),
            &block_row.get::<String, _>("edit_event_id"),
            &ctx,
        );

        let canvas = db
            .create_canvas(
                &ctx,
                NewCanvas {
                    workspace_id: workspace.id.clone(),
                    title: format!("canvas-{}", Uuid::new_v4()),
                },
            )
            .await?;

        let node_a_id = Uuid::new_v4().to_string();
        let node_b_id = Uuid::new_v4().to_string();
        let edge_id = Uuid::new_v4().to_string();
        db.update_canvas_graph(
            &ctx,
            &canvas.id,
            vec![
                NewCanvasNode {
                    id: Some(node_a_id.clone()),
                    kind: "text".into(),
                    position_x: 0.0,
                    position_y: 0.0,
                    data: None,
                },
                NewCanvasNode {
                    id: Some(node_b_id.clone()),
                    kind: "text".into(),
                    position_x: 1.0,
                    position_y: 1.0,
                    data: None,
                },
            ],
            vec![NewCanvasEdge {
                id: Some(edge_id.clone()),
                from_node_id: node_a_id.clone(),
                to_node_id: node_b_id.clone(),
                kind: "direct".into(),
            }],
        )
        .await?;

        let canvas_row = sqlx::query(
            r#"
            SELECT last_actor_kind, last_actor_id, last_job_id, last_workflow_id, edit_event_id
            FROM canvases
            WHERE id = ?
            "#,
        )
        .bind(&canvas.id)
        .fetch_one(pool)
        .await?;
        assert_metadata_matches_ctx(
            &canvas_row.get::<String, _>("last_actor_kind"),
            canvas_row.get::<Option<String>, _>("last_actor_id"),
            canvas_row.get::<Option<String>, _>("last_job_id"),
            canvas_row.get::<Option<String>, _>("last_workflow_id"),
            &canvas_row.get::<String, _>("edit_event_id"),
            &ctx,
        );

        let node_row = sqlx::query(
            r#"
            SELECT last_actor_kind, last_actor_id, last_job_id, last_workflow_id, edit_event_id
            FROM canvas_nodes
            WHERE id = ?
            "#,
        )
        .bind(&node_a_id)
        .fetch_one(pool)
        .await?;
        assert_metadata_matches_ctx(
            &node_row.get::<String, _>("last_actor_kind"),
            node_row.get::<Option<String>, _>("last_actor_id"),
            node_row.get::<Option<String>, _>("last_job_id"),
            node_row.get::<Option<String>, _>("last_workflow_id"),
            &node_row.get::<String, _>("edit_event_id"),
            &ctx,
        );

        let edge_row = sqlx::query(
            r#"
            SELECT last_actor_kind, last_actor_id, last_job_id, last_workflow_id, edit_event_id
            FROM canvas_edges
            WHERE id = ?
            "#,
        )
        .bind(&edge_id)
        .fetch_one(pool)
        .await?;
        assert_metadata_matches_ctx(
            &edge_row.get::<String, _>("last_actor_kind"),
            edge_row.get::<Option<String>, _>("last_actor_id"),
            edge_row.get::<Option<String>, _>("last_job_id"),
            edge_row.get::<Option<String>, _>("last_workflow_id"),
            &edge_row.get::<String, _>("edit_event_id"),
            &ctx,
        );
    }

    Ok(())
}

#[tokio::test]
async fn postgres_rejects_ai_writes_without_context_with_hsk_403_silent_edit() -> StorageResult<()>
{
    let Some(url) = postgres_test_url() else {
        return Ok(());
    };

    let mut conn = sqlx::PgConnection::connect(&url).await?;
    let schema = format!("wp1_trace_{}", Uuid::new_v4().simple());
    sqlx::query(&format!("CREATE SCHEMA {schema}"))
        .execute(&mut conn)
        .await?;
    drop(conn);

    let sep = if url.contains('?') { "&" } else { "?" };
    let schema_url = format!("{url}{sep}options=-csearch_path%3D{schema}");

    let db = PostgresDatabase::connect(&schema_url, 5).await?;
    db.run_migrations().await?;
    let db = db.into_arc();

    let ctx = WriteContext::ai(Some("ai-writer".into()), None, None);
    let res = db
        .create_workspace(
            &ctx,
            NewWorkspace {
                name: format!("ws-{}", Uuid::new_v4()),
            },
        )
        .await;
    assert!(matches!(
        res,
        Err(StorageError::Guard("HSK-403-SILENT-EDIT"))
    ));

    drop(db);
    let mut conn = sqlx::PgConnection::connect(&url).await?;
    sqlx::query(&format!("DROP SCHEMA IF EXISTS {schema} CASCADE"))
        .execute(&mut conn)
        .await?;
    Ok(())
}

#[tokio::test]
async fn postgres_persists_mutation_traceability_metadata_on_writes() -> StorageResult<()> {
    let Some(url) = postgres_test_url() else {
        return Ok(());
    };

    let mut conn = sqlx::PgConnection::connect(&url).await?;
    let schema = format!("wp1_trace_{}", Uuid::new_v4().simple());
    sqlx::query(&format!("CREATE SCHEMA {schema}"))
        .execute(&mut conn)
        .await?;
    drop(conn);

    let sep = if url.contains('?') { "&" } else { "?" };
    let schema_url = format!("{url}{sep}options=-csearch_path%3D{schema}");

    let db = PostgresDatabase::connect(&schema_url, 5).await?;
    db.run_migrations().await?;
    let db = db.into_arc();

    let postgres = db
        .as_any()
        .downcast_ref::<PostgresDatabase>()
        .ok_or(StorageError::Validation("expected postgres backend"))?;
    let pool = postgres.pool();

    let contexts = vec![
        WriteContext::human(Some("human-1".into())),
        WriteContext::system(Some("system-1".into())),
        WriteContext::ai(
            Some("ai-1".into()),
            Some(Uuid::new_v4()),
            Some(Uuid::new_v4()),
        ),
    ];

    for ctx in contexts {
        let workspace = db
            .create_workspace(
                &ctx,
                NewWorkspace {
                    name: format!("ws-{}", Uuid::new_v4()),
                },
            )
            .await?;

        let workspace_row = sqlx::query(
            r#"
            SELECT last_actor_kind, last_actor_id, last_job_id, last_workflow_id, edit_event_id
            FROM workspaces
            WHERE id = $1
            "#,
        )
        .bind(&workspace.id)
        .fetch_one(pool)
        .await?;
        assert_metadata_matches_ctx(
            &workspace_row.get::<String, _>("last_actor_kind"),
            workspace_row.get::<Option<String>, _>("last_actor_id"),
            workspace_row.get::<Option<String>, _>("last_job_id"),
            workspace_row.get::<Option<String>, _>("last_workflow_id"),
            &workspace_row.get::<String, _>("edit_event_id"),
            &ctx,
        );

        let document = db
            .create_document(
                &ctx,
                NewDocument {
                    workspace_id: workspace.id.clone(),
                    title: format!("doc-{}", Uuid::new_v4()),
                },
            )
            .await?;

        let document_row = sqlx::query(
            r#"
            SELECT last_actor_kind, last_actor_id, last_job_id, last_workflow_id, edit_event_id
            FROM documents
            WHERE id = $1
            "#,
        )
        .bind(&document.id)
        .fetch_one(pool)
        .await?;
        assert_metadata_matches_ctx(
            &document_row.get::<String, _>("last_actor_kind"),
            document_row.get::<Option<String>, _>("last_actor_id"),
            document_row.get::<Option<String>, _>("last_job_id"),
            document_row.get::<Option<String>, _>("last_workflow_id"),
            &document_row.get::<String, _>("edit_event_id"),
            &ctx,
        );

        let block = db
            .create_block(
                &ctx,
                NewBlock {
                    id: None,
                    document_id: document.id.clone(),
                    kind: "paragraph".into(),
                    sequence: 1,
                    raw_content: "hello".into(),
                    display_content: None,
                    derived_content: None,
                    sensitivity: None,
                    exportable: None,
                },
            )
            .await?;

        let block_row = sqlx::query(
            r#"
            SELECT last_actor_kind, last_actor_id, last_job_id, last_workflow_id, edit_event_id
            FROM blocks
            WHERE id = $1
            "#,
        )
        .bind(&block.id)
        .fetch_one(pool)
        .await?;
        assert_metadata_matches_ctx(
            &block_row.get::<String, _>("last_actor_kind"),
            block_row.get::<Option<String>, _>("last_actor_id"),
            block_row.get::<Option<String>, _>("last_job_id"),
            block_row.get::<Option<String>, _>("last_workflow_id"),
            &block_row.get::<String, _>("edit_event_id"),
            &ctx,
        );

        let canvas = db
            .create_canvas(
                &ctx,
                NewCanvas {
                    workspace_id: workspace.id.clone(),
                    title: format!("canvas-{}", Uuid::new_v4()),
                },
            )
            .await?;

        let node_a_id = Uuid::new_v4().to_string();
        let node_b_id = Uuid::new_v4().to_string();
        let edge_id = Uuid::new_v4().to_string();
        db.update_canvas_graph(
            &ctx,
            &canvas.id,
            vec![
                NewCanvasNode {
                    id: Some(node_a_id.clone()),
                    kind: "text".into(),
                    position_x: 0.0,
                    position_y: 0.0,
                    data: None,
                },
                NewCanvasNode {
                    id: Some(node_b_id.clone()),
                    kind: "text".into(),
                    position_x: 1.0,
                    position_y: 1.0,
                    data: None,
                },
            ],
            vec![NewCanvasEdge {
                id: Some(edge_id.clone()),
                from_node_id: node_a_id.clone(),
                to_node_id: node_b_id.clone(),
                kind: "direct".into(),
            }],
        )
        .await?;

        let canvas_row = sqlx::query(
            r#"
            SELECT last_actor_kind, last_actor_id, last_job_id, last_workflow_id, edit_event_id
            FROM canvases
            WHERE id = $1
            "#,
        )
        .bind(&canvas.id)
        .fetch_one(pool)
        .await?;
        assert_metadata_matches_ctx(
            &canvas_row.get::<String, _>("last_actor_kind"),
            canvas_row.get::<Option<String>, _>("last_actor_id"),
            canvas_row.get::<Option<String>, _>("last_job_id"),
            canvas_row.get::<Option<String>, _>("last_workflow_id"),
            &canvas_row.get::<String, _>("edit_event_id"),
            &ctx,
        );

        let node_row = sqlx::query(
            r#"
            SELECT last_actor_kind, last_actor_id, last_job_id, last_workflow_id, edit_event_id
            FROM canvas_nodes
            WHERE id = $1
            "#,
        )
        .bind(&node_a_id)
        .fetch_one(pool)
        .await?;
        assert_metadata_matches_ctx(
            &node_row.get::<String, _>("last_actor_kind"),
            node_row.get::<Option<String>, _>("last_actor_id"),
            node_row.get::<Option<String>, _>("last_job_id"),
            node_row.get::<Option<String>, _>("last_workflow_id"),
            &node_row.get::<String, _>("edit_event_id"),
            &ctx,
        );

        let edge_row = sqlx::query(
            r#"
            SELECT last_actor_kind, last_actor_id, last_job_id, last_workflow_id, edit_event_id
            FROM canvas_edges
            WHERE id = $1
            "#,
        )
        .bind(&edge_id)
        .fetch_one(pool)
        .await?;
        assert_metadata_matches_ctx(
            &edge_row.get::<String, _>("last_actor_kind"),
            edge_row.get::<Option<String>, _>("last_actor_id"),
            edge_row.get::<Option<String>, _>("last_job_id"),
            edge_row.get::<Option<String>, _>("last_workflow_id"),
            &edge_row.get::<String, _>("edit_event_id"),
            &ctx,
        );
    }

    drop(db);
    let mut conn = sqlx::PgConnection::connect(&url).await?;
    sqlx::query(&format!("DROP SCHEMA IF EXISTS {schema} CASCADE"))
        .execute(&mut conn)
        .await?;
    Ok(())
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

#[tokio::test]
async fn migrations_are_replay_safe_sqlite() -> StorageResult<()> {
    let migrator = sqlx::migrate!("./migrations");
    let mut conn = sqlx::SqliteConnection::connect("sqlite::memory:").await?;

    migrator.run(&mut conn).await?;
    let before = sqlite_schema_fingerprint(&mut conn).await?;

    sqlx::query("DROP TABLE IF EXISTS _sqlx_migrations")
        .execute(&mut conn)
        .await?;

    migrator.run(&mut conn).await?;
    let after = sqlite_schema_fingerprint(&mut conn).await?;

    assert_eq!(before, after);
    Ok(())
}

#[tokio::test]
async fn migrations_can_undo_to_baseline_sqlite() -> StorageResult<()> {
    let migrator = sqlx::migrate!("./migrations");
    let mut conn = sqlx::SqliteConnection::connect("sqlite::memory:").await?;

    migrator.run(&mut conn).await?;
    migrator.undo(&mut conn, 0).await?;

    let tables = sqlite_user_table_names(&mut conn).await?;
    assert!(tables.is_empty());

    let applied_count_row = sqlx::query("SELECT COUNT(*) as count FROM _sqlx_migrations")
        .fetch_one(&mut conn)
        .await?;
    let applied_count = applied_count_row.get::<i64, _>("count");
    assert_eq!(applied_count, 0);

    Ok(())
}

#[tokio::test]
async fn migrations_are_replay_safe_postgres() -> StorageResult<()> {
    let Some(url) = postgres_test_url() else {
        return Ok(());
    };

    let mut conn = sqlx::PgConnection::connect(&url).await?;
    let schema = format!("wp1_mig_{}", Uuid::new_v4().simple());

    sqlx::query(&format!("CREATE SCHEMA {schema}"))
        .execute(&mut conn)
        .await?;
    sqlx::query(&format!("SET search_path TO {schema}"))
        .execute(&mut conn)
        .await?;

    let migrator = sqlx::migrate!("./migrations");

    migrator.run(&mut conn).await?;
    let before = postgres_schema_fingerprint(&mut conn).await?;

    sqlx::query("DROP TABLE IF EXISTS _sqlx_migrations")
        .execute(&mut conn)
        .await?;

    migrator.run(&mut conn).await?;
    let after = postgres_schema_fingerprint(&mut conn).await?;

    assert_eq!(before, after);

    sqlx::query("SET search_path TO public")
        .execute(&mut conn)
        .await?;
    sqlx::query(&format!("DROP SCHEMA IF EXISTS {schema} CASCADE"))
        .execute(&mut conn)
        .await?;

    Ok(())
}

#[tokio::test]
async fn migrations_can_undo_to_baseline_postgres() -> StorageResult<()> {
    let Some(url) = postgres_test_url() else {
        return Ok(());
    };

    let mut conn = sqlx::PgConnection::connect(&url).await?;
    let schema = format!("wp1_mig_{}", Uuid::new_v4().simple());

    sqlx::query(&format!("CREATE SCHEMA {schema}"))
        .execute(&mut conn)
        .await?;
    sqlx::query(&format!("SET search_path TO {schema}"))
        .execute(&mut conn)
        .await?;

    let migrator = sqlx::migrate!("./migrations");

    migrator.run(&mut conn).await?;
    migrator.undo(&mut conn, 0).await?;

    let tables = postgres_user_table_names(&mut conn).await?;
    assert!(tables.is_empty());

    let applied_count_row = sqlx::query("SELECT COUNT(*) as count FROM _sqlx_migrations")
        .fetch_one(&mut conn)
        .await?;
    let applied_count = applied_count_row.get::<i64, _>("count");
    assert_eq!(applied_count, 0);

    sqlx::query("SET search_path TO public")
        .execute(&mut conn)
        .await?;
    sqlx::query(&format!("DROP SCHEMA IF EXISTS {schema} CASCADE"))
        .execute(&mut conn)
        .await?;

    Ok(())
}
