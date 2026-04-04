#[allow(unused_imports)]
use super::{
    postgres::PostgresDatabase, sqlite::SqliteDatabase, AccessMode, BlockUpdate,
    CalendarEventExportMode, CalendarEventStatus, CalendarEventUpsert, CalendarEventVisibility,
    CalendarEventWindowQuery, CalendarSourceProviderType, CalendarSourceSyncState,
    CalendarSourceUpsert, CalendarSourceWritePolicy, Database, DefaultStorageGuard, EntityRef,
    GuardError, JobKind, JobMetrics, JobState, JobStatusUpdate, LoomBlock, LoomBlockContentType,
    LoomBlockSearchResult, LoomEdgeCreatedBy, LoomEdgeType, LoomSearchFilters, LoomSourceAnchor,
    LoomViewFilters, LoomViewResponse, LoomViewType, NewAiJob, NewAsset, NewBlock, NewCanvas,
    NewCanvasEdge, NewCanvasNode, NewDocument, NewLoomBlock, NewLoomEdge, NewNodeExecution,
    NewWorkspace, OperationType, PlannedOperation, SafetyMode, StorageError, StorageGuard,
    StorageBackendKind, StorageCapabilityStore, StorageResult, WriteContext,
};
use chrono::Duration;
use chrono::Utc;
use serde_json::json;
use sqlx::Connection;
#[cfg(test)]
use sqlx::Row;
use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::Instant;
use uuid::Uuid;

#[cfg(test)]
const NIL_EDIT_EVENT_ID: &str = "00000000-0000-0000-0000-000000000000";
const LOOM_TRAVERSAL_PERF_TOTAL_BLOCKS: usize = 10_000;

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
    let mut conn = sqlx::PgConnection::connect(&url).await?;
    let schema = format!("storage_test_{}", Uuid::new_v4().simple());
    sqlx::query(&format!("CREATE SCHEMA {schema}"))
        .execute(&mut conn)
        .await?;
    drop(conn);

    let sep = if url.contains('?') { "&" } else { "?" };
    let schema_url = format!("{url}{sep}options=-csearch_path%3D{schema}");

    let db = PostgresDatabase::connect(&schema_url, 5).await?;
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

    let source = db
        .upsert_calendar_source(
            &ctx,
            CalendarSourceUpsert {
                id: format!("local:{}", Uuid::new_v4()),
                workspace_id: workspace.id.clone(),
                display_name: "Local".into(),
                provider_type: CalendarSourceProviderType::Local,
                write_policy: CalendarSourceWritePolicy::PublishFromHandshake,
                default_tzid: "Europe/Brussels".into(),
                auto_export: false,
                credentials_ref: None,
                provider_calendar_id: None,
                capability_profile_id: Some("calendar-local".into()),
                config: json!({"kind": "local"}),
                sync_state: CalendarSourceSyncState::default(),
            },
        )
        .await?;

    let loaded_source = db
        .get_calendar_source(&workspace.id, &source.id)
        .await?
        .ok_or(StorageError::NotFound("calendar_source"))?;
    assert_eq!(loaded_source.id, source.id);
    assert_eq!(loaded_source.workspace_id, workspace.id);

    let listed_sources = db.list_calendar_sources(&workspace.id).await?;
    assert!(listed_sources.iter().any(|item| item.id == source.id));

    let event_start = Utc::now() + Duration::hours(2);
    let event_end = event_start + Duration::hours(1);
    let event = db
        .upsert_calendar_event(
            &ctx,
            CalendarEventUpsert {
                id: Uuid::new_v4().to_string(),
                workspace_id: workspace.id.clone(),
                source_id: source.id.clone(),
                external_id: None,
                external_etag: None,
                title: "Calendar smoke".into(),
                description: Some("storage conformance".into()),
                location: Some("Desk".into()),
                start_ts_utc: event_start,
                end_ts_utc: event_end,
                start_local: Some("2026-03-06T10:00:00".into()),
                end_local: Some("2026-03-06T11:00:00".into()),
                tzid: "Europe/Brussels".into(),
                all_day: false,
                was_floating: false,
                status: CalendarEventStatus::Confirmed,
                visibility: CalendarEventVisibility::Private,
                export_mode: CalendarEventExportMode::LocalOnly,
                rrule: None,
                rdate: Vec::new(),
                exdate: Vec::new(),
                is_recurring: false,
                series_id: None,
                instance_key: None,
                is_override: false,
                source_last_seen_at: None,
                attendees: json!([]),
                links: json!([]),
                provider_payload: Some(json!({"kind": "smoke"})),
            },
        )
        .await?;

    let queried_events = db
        .query_calendar_events(CalendarEventWindowQuery {
            workspace_id: workspace.id.clone(),
            window_start_utc: event_start - Duration::minutes(30),
            window_end_utc: event_end + Duration::minutes(30),
            source_ids: Vec::new(),
        })
        .await?;
    assert!(queried_events.iter().any(|item| item.id == event.id));

    db.delete_calendar_data_by_source(&ctx, &workspace.id, &source.id)
        .await?;
    let remaining_events = db
        .query_calendar_events(CalendarEventWindowQuery {
            workspace_id: workspace.id.clone(),
            window_start_utc: event_start - Duration::minutes(30),
            window_end_utc: event_end + Duration::minutes(30),
            source_ids: Vec::new(),
        })
        .await?;
    assert!(!remaining_events.iter().any(|item| item.id == event.id));

    db.delete_document(&ctx, &document.id).await?;
    db.delete_canvas(&ctx, &canvas.id).await?;
    db.delete_workspace(&ctx, &workspace.id).await?;

    Ok(())
}

fn sorted_strings<I>(items: I) -> Vec<String>
where
    I: IntoIterator<Item = String>,
{
    let mut items: Vec<String> = items.into_iter().collect();
    items.sort();
    items
}

fn loom_block_ids(blocks: &[LoomBlock]) -> Vec<String> {
    sorted_strings(blocks.iter().map(|block| block.block_id.clone()))
}

fn loom_search_ids(results: &[LoomBlockSearchResult]) -> Vec<String> {
    sorted_strings(results.iter().map(|result| result.block.block_id.clone()))
}

fn sorted_view_groups(resp: &LoomViewResponse) -> BTreeMap<String, Vec<String>> {
    let LoomViewResponse::Sorted { groups } = resp else {
        panic!("expected sorted loom view response");
    };

    groups
        .iter()
        .map(|group| {
            (
                format!("{}:{}", group.edge_type.as_str(), group.target_block_id),
                loom_block_ids(&group.blocks),
            )
        })
        .collect()
}

fn loom_traversal_signature(results: &[(LoomBlock, u32)]) -> Vec<(String, u32)> {
    results
        .iter()
        .map(|(block, depth)| (block.block_id.clone(), *depth))
        .collect()
}

async fn create_test_loom_block(
    db: &Arc<dyn super::Database>,
    ctx: &WriteContext,
    workspace_id: &str,
    content_type: LoomBlockContentType,
    document_id: Option<&str>,
    title: &str,
    full_text_index: &str,
) -> StorageResult<LoomBlock> {
    db.create_loom_block(
        ctx,
        NewLoomBlock {
            block_id: None,
            workspace_id: workspace_id.to_string(),
            content_type,
            document_id: document_id.map(str::to_string),
            asset_id: None,
            title: Some(title.to_string()),
            original_filename: None,
            content_hash: None,
            pinned: false,
            journal_date: None,
            imported_at: None,
            derived: super::LoomBlockDerived {
                full_text_index: Some(full_text_index.to_string()),
                ..Default::default()
            },
        },
    )
    .await
}

struct LoomGraphFixture {
    start_block_id: String,
    mid_block_id: String,
    leaf_block_id: String,
    tag_block_id: String,
}

async fn build_loom_graph_fixture(
    db: &Arc<dyn super::Database>,
    ctx: &WriteContext,
    workspace_id: &str,
    document_id: &str,
) -> StorageResult<LoomGraphFixture> {
    let graph_start = create_test_loom_block(
        db,
        ctx,
        workspace_id,
        LoomBlockContentType::Note,
        Some(document_id),
        "Graph Start",
        "graph depth start",
    )
    .await?;
    let graph_mid = create_test_loom_block(
        db,
        ctx,
        workspace_id,
        LoomBlockContentType::Note,
        Some(document_id),
        "Graph Mid",
        "graph depth mid",
    )
    .await?;
    let graph_leaf = create_test_loom_block(
        db,
        ctx,
        workspace_id,
        LoomBlockContentType::Note,
        Some(document_id),
        "Graph Leaf",
        "graph depth leaf",
    )
    .await?;
    let graph_tag = create_test_loom_block(
        db,
        ctx,
        workspace_id,
        LoomBlockContentType::TagHub,
        None,
        "Graph Deep Tag",
        "graph deep tag",
    )
    .await?;

    db.create_loom_edge(
        ctx,
        NewLoomEdge {
            edge_id: None,
            workspace_id: workspace_id.to_string(),
            source_block_id: graph_start.block_id.clone(),
            target_block_id: graph_mid.block_id.clone(),
            edge_type: LoomEdgeType::Mention,
            created_by: LoomEdgeCreatedBy::User,
            crdt_site_id: None,
            source_anchor: None,
        },
    )
    .await?;
    db.create_loom_edge(
        ctx,
        NewLoomEdge {
            edge_id: None,
            workspace_id: workspace_id.to_string(),
            source_block_id: graph_mid.block_id.clone(),
            target_block_id: graph_leaf.block_id.clone(),
            edge_type: LoomEdgeType::Parent,
            created_by: LoomEdgeCreatedBy::User,
            crdt_site_id: None,
            source_anchor: None,
        },
    )
    .await?;
    db.create_loom_edge(
        ctx,
        NewLoomEdge {
            edge_id: None,
            workspace_id: workspace_id.to_string(),
            source_block_id: graph_leaf.block_id.clone(),
            target_block_id: graph_tag.block_id.clone(),
            edge_type: LoomEdgeType::Tag,
            created_by: LoomEdgeCreatedBy::User,
            crdt_site_id: None,
            source_anchor: None,
        },
    )
    .await?;
    db.create_loom_edge(
        ctx,
        NewLoomEdge {
            edge_id: None,
            workspace_id: workspace_id.to_string(),
            source_block_id: graph_leaf.block_id.clone(),
            target_block_id: graph_start.block_id.clone(),
            edge_type: LoomEdgeType::AiSuggested,
            created_by: LoomEdgeCreatedBy::Ai,
            crdt_site_id: None,
            source_anchor: None,
        },
    )
    .await?;

    Ok(LoomGraphFixture {
        start_block_id: graph_start.block_id,
        mid_block_id: graph_mid.block_id,
        leaf_block_id: graph_leaf.block_id,
        tag_block_id: graph_tag.block_id,
    })
}

async fn overwrite_loom_block_metrics(
    db: &Arc<dyn super::Database>,
    workspace_id: &str,
    block_id: &str,
    mention_count: i64,
    tag_count: i64,
    backlink_count: i64,
) -> StorageResult<()> {
    if let Some(sqlite) = db.as_any().downcast_ref::<SqliteDatabase>() {
        sqlx::query(
            r#"
            UPDATE loom_blocks
            SET mention_count = $1, tag_count = $2, backlink_count = $3
            WHERE workspace_id = $4 AND block_id = $5
            "#,
        )
        .bind(mention_count)
        .bind(tag_count)
        .bind(backlink_count)
        .bind(workspace_id)
        .bind(block_id)
        .execute(sqlite.pool())
        .await?;
        return Ok(());
    }

    if let Some(postgres) = db.as_any().downcast_ref::<PostgresDatabase>() {
        sqlx::query(
            r#"
            UPDATE loom_blocks
            SET mention_count = $1, tag_count = $2, backlink_count = $3
            WHERE workspace_id = $4 AND block_id = $5
            "#,
        )
        .bind(mention_count as i32)
        .bind(tag_count as i32)
        .bind(backlink_count as i32)
        .bind(workspace_id)
        .bind(block_id)
        .execute(postgres.pool())
        .await?;
        return Ok(());
    }

    Err(StorageError::Validation("unsupported loom metrics backend"))
}

async fn zero_workspace_loom_metrics(
    db: &Arc<dyn super::Database>,
    workspace_id: &str,
) -> StorageResult<()> {
    if let Some(sqlite) = db.as_any().downcast_ref::<SqliteDatabase>() {
        sqlx::query(
            r#"
            UPDATE loom_blocks
            SET mention_count = 0, tag_count = 0, backlink_count = 0
            WHERE workspace_id = $1
            "#,
        )
        .bind(workspace_id)
        .execute(sqlite.pool())
        .await?;
        return Ok(());
    }

    if let Some(postgres) = db.as_any().downcast_ref::<PostgresDatabase>() {
        sqlx::query(
            r#"
            UPDATE loom_blocks
            SET mention_count = 0, tag_count = 0, backlink_count = 0
            WHERE workspace_id = $1
            "#,
        )
        .bind(workspace_id)
        .execute(postgres.pool())
        .await?;
        return Ok(());
    }

    Err(StorageError::Validation("unsupported loom metrics backend"))
}

async fn loom_metrics_recompute_idempotent(
    db: &Arc<dyn super::Database>,
    workspace_id: &str,
    portable_note_id: &str,
    mention_target_id: &str,
    tag_hub_id: &str,
) -> StorageResult<()> {
    overwrite_loom_block_metrics(db, workspace_id, portable_note_id, 99, 98, 97).await?;
    db.recompute_block_metrics(workspace_id, portable_note_id)
        .await?;

    let portable_note = db.get_loom_block(workspace_id, portable_note_id).await?;
    assert_eq!(portable_note.derived.mention_count, 1);
    assert_eq!(portable_note.derived.tag_count, 1);
    assert_eq!(portable_note.derived.backlink_count, 0);

    zero_workspace_loom_metrics(db, workspace_id).await?;
    db.recompute_all_metrics(workspace_id).await?;

    let portable_note = db.get_loom_block(workspace_id, portable_note_id).await?;
    let mention_target = db.get_loom_block(workspace_id, mention_target_id).await?;
    let tag_hub = db.get_loom_block(workspace_id, tag_hub_id).await?;
    assert_eq!(portable_note.derived.mention_count, 1);
    assert_eq!(portable_note.derived.tag_count, 1);
    assert_eq!(portable_note.derived.backlink_count, 0);
    assert_eq!(mention_target.derived.backlink_count, 1);
    assert_eq!(tag_hub.derived.backlink_count, 2);

    db.recompute_all_metrics(workspace_id).await?;
    let portable_note_again = db.get_loom_block(workspace_id, portable_note_id).await?;
    assert_eq!(portable_note_again.derived.mention_count, 1);
    assert_eq!(portable_note_again.derived.tag_count, 1);
    assert_eq!(portable_note_again.derived.backlink_count, 0);

    Ok(())
}

async fn loom_traverse_graph_depth_limit(
    db: &Arc<dyn super::Database>,
    workspace_id: &str,
    graph: &LoomGraphFixture,
) -> StorageResult<()> {
    let depth_one = db
        .traverse_graph(
            workspace_id,
            &graph.start_block_id,
            1,
            &[
                LoomEdgeType::Mention,
                LoomEdgeType::Parent,
                LoomEdgeType::Tag,
                LoomEdgeType::AiSuggested,
            ],
        )
        .await?;
    assert_eq!(
        loom_traversal_signature(&depth_one),
        vec![(graph.mid_block_id.clone(), 1)]
    );

    let depth_two = db
        .traverse_graph(
            workspace_id,
            &graph.start_block_id,
            2,
            &[
                LoomEdgeType::Mention,
                LoomEdgeType::Parent,
                LoomEdgeType::Tag,
                LoomEdgeType::AiSuggested,
            ],
        )
        .await?;
    assert_eq!(
        loom_traversal_signature(&depth_two),
        vec![
            (graph.mid_block_id.clone(), 1),
            (graph.leaf_block_id.clone(), 2),
        ]
    );

    let depth_three = db
        .traverse_graph(
            workspace_id,
            &graph.start_block_id,
            3,
            &[
                LoomEdgeType::Mention,
                LoomEdgeType::Parent,
                LoomEdgeType::Tag,
                LoomEdgeType::AiSuggested,
            ],
        )
        .await?;
    assert_eq!(
        loom_traversal_signature(&depth_three),
        vec![
            (graph.mid_block_id.clone(), 1),
            (graph.leaf_block_id.clone(), 2),
            (graph.tag_block_id.clone(), 3),
        ]
    );

    Ok(())
}

async fn loom_traverse_graph_cycle_detection(
    db: &Arc<dyn super::Database>,
    workspace_id: &str,
    graph: &LoomGraphFixture,
) -> StorageResult<()> {
    let traversed = db
        .traverse_graph(
            workspace_id,
            &graph.start_block_id,
            8,
            &[
                LoomEdgeType::Mention,
                LoomEdgeType::Parent,
                LoomEdgeType::Tag,
                LoomEdgeType::AiSuggested,
            ],
        )
        .await?;
    let signature = loom_traversal_signature(&traversed);
    assert_eq!(
        signature,
        vec![
            (graph.mid_block_id.clone(), 1),
            (graph.leaf_block_id.clone(), 2),
            (graph.tag_block_id.clone(), 3),
        ]
    );
    assert!(
        !signature
            .iter()
            .any(|(block_id, _)| block_id == &graph.start_block_id),
        "cycle traversal must not re-emit the starting block"
    );

    Ok(())
}

async fn loom_traverse_graph_edge_type_filter(
    db: &Arc<dyn super::Database>,
    workspace_id: &str,
    graph: &LoomGraphFixture,
) -> StorageResult<()> {
    let mention_parent_only = db
        .traverse_graph(
            workspace_id,
            &graph.start_block_id,
            3,
            &[LoomEdgeType::Mention, LoomEdgeType::Parent],
        )
        .await?;
    assert_eq!(
        loom_traversal_signature(&mention_parent_only),
        vec![
            (graph.mid_block_id.clone(), 1),
            (graph.leaf_block_id.clone(), 2),
        ]
    );

    Ok(())
}

async fn loom_directional_edge_queries(
    db: &Arc<dyn super::Database>,
    ctx: &WriteContext,
    workspace_id: &str,
    document_id: &str,
    target_block_id: &str,
    outgoing_edge_ids: &[String],
) -> StorageResult<()> {
    let incoming_source = create_test_loom_block(
        db,
        ctx,
        workspace_id,
        LoomBlockContentType::Note,
        Some(document_id),
        "Incoming Edge Source",
        "incoming edge source",
    )
    .await?;

    let incoming_edge = db
        .create_loom_edge(
            ctx,
            NewLoomEdge {
                edge_id: None,
                workspace_id: workspace_id.to_string(),
                source_block_id: incoming_source.block_id,
                target_block_id: target_block_id.to_string(),
                edge_type: LoomEdgeType::AiSuggested,
                created_by: LoomEdgeCreatedBy::Ai,
                crdt_site_id: None,
                source_anchor: None,
            },
        )
        .await?;

    let backlinks = db.get_backlinks(workspace_id, target_block_id).await?;
    let outgoing = db.get_outgoing_edges(workspace_id, target_block_id).await?;

    assert_eq!(
        sorted_strings(backlinks.iter().map(|edge| edge.edge_id.clone())),
        vec![incoming_edge.edge_id]
    );
    assert_eq!(
        sorted_strings(outgoing.iter().map(|edge| edge.edge_id.clone())),
        sorted_strings(outgoing_edge_ids.iter().cloned())
    );

    Ok(())
}

async fn loom_search_graph_filter_postgres(
    db: &Arc<dyn super::Database>,
    workspace_id: &str,
    graph: &LoomGraphFixture,
) -> StorageResult<()> {
    if !db.as_any().is::<PostgresDatabase>() {
        return Ok(());
    }

    let direct_only = db
        .search_loom_blocks(
            workspace_id,
            "graph depth start",
            LoomSearchFilters {
                tag_ids: vec![graph.tag_block_id.clone()],
                backlink_depth: Some(1),
                ..Default::default()
            },
            50,
            0,
        )
        .await?;
    assert!(
        direct_only.is_empty(),
        "depth-1 graph filtering should not match indirect tag paths"
    );

    let graph_filtered = db
        .search_loom_blocks(
            workspace_id,
            "graph depth start",
            LoomSearchFilters {
                tag_ids: vec![graph.tag_block_id.clone()],
                backlink_depth: Some(3),
                ..Default::default()
            },
            50,
            0,
        )
        .await?;
    assert_eq!(
        loom_search_ids(&graph_filtered),
        vec![graph.start_block_id.clone()]
    );

    Ok(())
}

async fn loom_source_anchor_round_trip(
    db: &Arc<dyn super::Database>,
    ctx: &WriteContext,
    workspace_id: &str,
    document_id: &str,
    anchor: &LoomSourceAnchor,
) -> StorageResult<()> {
    let exported_anchor_json = serde_json::to_string(anchor)
        .map_err(|_| StorageError::Validation("invalid source anchor export"))?;
    let replayed_anchor: LoomSourceAnchor = serde_json::from_str(&exported_anchor_json)
        .map_err(|_| StorageError::Validation("invalid source anchor replay"))?;

    let replay_source = create_test_loom_block(
        db,
        ctx,
        workspace_id,
        LoomBlockContentType::Note,
        Some(document_id),
        "Anchor Replay Source",
        "anchor replay source",
    )
    .await?;
    let replay_target = create_test_loom_block(
        db,
        ctx,
        workspace_id,
        LoomBlockContentType::Note,
        Some(document_id),
        "Anchor Replay Target",
        "anchor replay target",
    )
    .await?;

    let replayed_edge = db
        .create_loom_edge(
            ctx,
            NewLoomEdge {
                edge_id: None,
                workspace_id: workspace_id.to_string(),
                source_block_id: replay_source.block_id.clone(),
                target_block_id: replay_target.block_id.clone(),
                edge_type: LoomEdgeType::AiSuggested,
                created_by: LoomEdgeCreatedBy::Ai,
                crdt_site_id: None,
                source_anchor: Some(replayed_anchor.clone()),
            },
        )
        .await?;

    let stored_edge = db
        .get_outgoing_edges(workspace_id, &replay_source.block_id)
        .await?
        .into_iter()
        .find(|edge| edge.edge_id == replayed_edge.edge_id)
        .ok_or(StorageError::NotFound("loom_edge"))?;
    let stored_anchor = stored_edge
        .source_anchor
        .clone()
        .ok_or(StorageError::NotFound("loom_source_anchor"))?;
    assert_eq!(stored_anchor.document_id, anchor.document_id);
    assert_eq!(stored_anchor.block_id, anchor.block_id);
    assert_eq!(stored_anchor.offset_start, anchor.offset_start);
    assert_eq!(stored_anchor.offset_end, anchor.offset_end);

    let exported_edge_json = serde_json::to_string(&stored_edge)
        .map_err(|_| StorageError::Validation("invalid loom edge export"))?;
    let replayed_edge_again: super::LoomEdge = serde_json::from_str(&exported_edge_json)
        .map_err(|_| StorageError::Validation("invalid loom edge replay"))?;
    let replayed_anchor_again = replayed_edge_again
        .source_anchor
        .ok_or(StorageError::NotFound("loom_source_anchor"))?;
    assert_eq!(replayed_anchor_again.document_id, anchor.document_id);
    assert_eq!(replayed_anchor_again.block_id, anchor.block_id);
    assert_eq!(replayed_anchor_again.offset_start, anchor.offset_start);
    assert_eq!(replayed_anchor_again.offset_end, anchor.offset_end);

    Ok(())
}

async fn insert_loom_traversal_perf_fixture(
    db: &Arc<dyn super::Database>,
    workspace_id: &str,
) -> StorageResult<String> {
    let created_at = Utc::now();
    let derived_json = serde_json::to_string(&super::LoomBlockDerived::default())?;
    let start_block_id = "perf-block-00000".to_string();

    if let Some(sqlite) = db.as_any().downcast_ref::<SqliteDatabase>() {
        let mut tx = sqlite.pool().begin().await?;
        for idx in 0..LOOM_TRAVERSAL_PERF_TOTAL_BLOCKS {
            let block_id = format!("perf-block-{idx:05}");
            sqlx::query(
                r#"
                INSERT INTO loom_blocks (
                    block_id,
                    workspace_id,
                    content_type,
                    title,
                    pinned,
                    last_actor_kind,
                    edit_event_id,
                    created_at,
                    updated_at,
                    backlink_count,
                    mention_count,
                    tag_count,
                    derived_json,
                    preview_status
                )
                VALUES (
                    $1, $2, 'note', $3, 0, 'SYSTEM',
                    '00000000-0000-0000-0000-000000000000',
                    $4, $4, 0, 0, 0, $5, 'none'
                )
                "#,
            )
            .bind(&block_id)
            .bind(workspace_id)
            .bind(format!("Perf Block {idx}"))
            .bind(created_at)
            .bind(&derived_json)
            .execute(&mut *tx)
            .await?;

            if idx > 0 {
                sqlx::query(
                    r#"
                    INSERT INTO loom_edges (
                        edge_id,
                        workspace_id,
                        source_block_id,
                        target_block_id,
                        edge_type,
                        created_by,
                        last_actor_kind,
                        edit_event_id,
                        created_at
                    )
                    VALUES (
                        $1, $2, $3, $4, 'mention', 'user', 'SYSTEM',
                        '00000000-0000-0000-0000-000000000000',
                        $5
                    )
                    "#,
                )
                .bind(format!("perf-edge-{idx:05}"))
                .bind(workspace_id)
                .bind(format!("perf-block-{:05}", idx - 1))
                .bind(&block_id)
                .bind(created_at)
                .execute(&mut *tx)
                .await?;
            }
        }
        tx.commit().await?;
        return Ok(start_block_id);
    }

    if let Some(postgres) = db.as_any().downcast_ref::<PostgresDatabase>() {
        let mut tx = postgres.pool().begin().await?;
        for idx in 0..LOOM_TRAVERSAL_PERF_TOTAL_BLOCKS {
            let block_id = format!("perf-block-{idx:05}");
            sqlx::query(
                r#"
                INSERT INTO loom_blocks (
                    block_id,
                    workspace_id,
                    content_type,
                    title,
                    pinned,
                    last_actor_kind,
                    edit_event_id,
                    created_at,
                    updated_at,
                    backlink_count,
                    mention_count,
                    tag_count,
                    derived_json,
                    preview_status
                )
                VALUES (
                    $1, $2, 'note', $3, 0, 'SYSTEM',
                    '00000000-0000-0000-0000-000000000000',
                    $4, $4, 0, 0, 0, $5, 'none'
                )
                "#,
            )
            .bind(&block_id)
            .bind(workspace_id)
            .bind(format!("Perf Block {idx}"))
            .bind(created_at)
            .bind(&derived_json)
            .execute(&mut *tx)
            .await?;

            if idx > 0 {
                sqlx::query(
                    r#"
                    INSERT INTO loom_edges (
                        edge_id,
                        workspace_id,
                        source_block_id,
                        target_block_id,
                        edge_type,
                        created_by,
                        last_actor_kind,
                        edit_event_id,
                        created_at
                    )
                    VALUES (
                        $1, $2, $3, $4, 'mention', 'user', 'SYSTEM',
                        '00000000-0000-0000-0000-000000000000',
                        $5
                    )
                    "#,
                )
                .bind(format!("perf-edge-{idx:05}"))
                .bind(workspace_id)
                .bind(format!("perf-block-{:05}", idx - 1))
                .bind(&block_id)
                .bind(created_at)
                .execute(&mut *tx)
                .await?;
            }
        }
        tx.commit().await?;
        return Ok(start_block_id);
    }

    Err(StorageError::Validation(
        "unsupported loom traversal performance backend",
    ))
}

async fn loom_traverse_graph_meets_performance_target(
    db: &Arc<dyn super::Database>,
    workspace_id: &str,
) -> StorageResult<()> {
    let start_block_id = insert_loom_traversal_perf_fixture(db, workspace_id).await?;
    let expected = vec![
        ("perf-block-00001".to_string(), 1),
        ("perf-block-00002".to_string(), 2),
        ("perf-block-00003".to_string(), 3),
    ];

    let warmed = db
        .traverse_graph(workspace_id, &start_block_id, 3, &[LoomEdgeType::Mention])
        .await?;
    assert_eq!(loom_traversal_signature(&warmed), expected);

    let limit_ms = if db.as_any().is::<SqliteDatabase>() {
        100_u128
    } else {
        50_u128
    };
    let mut samples_ms = Vec::new();
    for _ in 0..3 {
        let started = Instant::now();
        let traversed = db
            .traverse_graph(workspace_id, &start_block_id, 3, &[LoomEdgeType::Mention])
            .await?;
        let elapsed_ms = started.elapsed().as_millis();
        assert_eq!(loom_traversal_signature(&traversed), expected);
        samples_ms.push(elapsed_ms);
    }
    samples_ms.sort_unstable();
    let median_ms = samples_ms[samples_ms.len() / 2];
    assert!(
        median_ms <= limit_ms,
        "expected 3-hop traverse_graph median <= {limit_ms}ms on {LOOM_TRAVERSAL_PERF_TOTAL_BLOCKS} blocks, observed samples {samples_ms:?}"
    );

    Ok(())
}

#[allow(dead_code)]
pub async fn run_loom_storage_conformance(db: Arc<dyn super::Database>) -> StorageResult<()> {
    db.ping().await?;

    let ctx = WriteContext::human(Some("loom-tester".into()));
    let workspace = db
        .create_workspace(
            &ctx,
            NewWorkspace {
                name: format!("loom-ws-{}", Uuid::new_v4()),
            },
        )
        .await?;
    let document = db
        .create_document(
            &ctx,
            NewDocument {
                workspace_id: workspace.id.clone(),
                title: "Loom Source Doc".into(),
            },
        )
        .await?;
    let source_block = db
        .create_block(
            &ctx,
            NewBlock {
                id: None,
                document_id: document.id.clone(),
                kind: "paragraph".into(),
                sequence: 1,
                raw_content: "portable anchor source".into(),
                display_content: None,
                derived_content: Some(json!({"loom": true})),
                sensitivity: None,
                exportable: None,
            },
        )
        .await?;

    let tag_hub = db
        .create_loom_block(
            &ctx,
            NewLoomBlock {
                block_id: None,
                workspace_id: workspace.id.clone(),
                content_type: LoomBlockContentType::TagHub,
                document_id: None,
                asset_id: None,
                title: Some("Portable Tag".into()),
                original_filename: None,
                content_hash: None,
                pinned: false,
                journal_date: None,
                imported_at: None,
                derived: super::LoomBlockDerived {
                    full_text_index: Some("portable tag hub".into()),
                    ..Default::default()
                },
            },
        )
        .await?;

    let portable_note = db
        .create_loom_block(
            &ctx,
            NewLoomBlock {
                block_id: None,
                workspace_id: workspace.id.clone(),
                content_type: LoomBlockContentType::Note,
                document_id: Some(document.id.clone()),
                asset_id: None,
                title: Some("Portable".into()),
                original_filename: None,
                content_hash: None,
                pinned: false,
                journal_date: None,
                imported_at: None,
                derived: super::LoomBlockDerived {
                    full_text_index: Some("parity notes".into()),
                    auto_caption: Some("metadata_shadow".into()),
                    ..Default::default()
                },
            },
        )
        .await?;

    let unlinked_note = db
        .create_loom_block(
            &ctx,
            NewLoomBlock {
                block_id: None,
                workspace_id: workspace.id.clone(),
                content_type: LoomBlockContentType::Note,
                document_id: Some(document.id.clone()),
                asset_id: None,
                title: Some("Detached".into()),
                original_filename: None,
                content_hash: None,
                pinned: false,
                journal_date: None,
                imported_at: None,
                derived: super::LoomBlockDerived {
                    full_text_index: Some("orphaned note".into()),
                    ..Default::default()
                },
            },
        )
        .await?;

    let mention_target = db
        .create_loom_block(
            &ctx,
            NewLoomBlock {
                block_id: None,
                workspace_id: workspace.id.clone(),
                content_type: LoomBlockContentType::Note,
                document_id: Some(document.id.clone()),
                asset_id: None,
                title: Some("Anchor Target".into()),
                original_filename: None,
                content_hash: None,
                pinned: false,
                journal_date: None,
                imported_at: None,
                derived: super::LoomBlockDerived {
                    full_text_index: Some("anchor target".into()),
                    ..Default::default()
                },
            },
        )
        .await?;

    let file_only_target = db
        .create_loom_block(
            &ctx,
            NewLoomBlock {
                block_id: Some("00000000-0000-0000-0000-000000000001".into()),
                workspace_id: workspace.id.clone(),
                content_type: LoomBlockContentType::Note,
                document_id: Some(document.id.clone()),
                asset_id: None,
                title: Some("File Only Target".into()),
                original_filename: None,
                content_hash: None,
                pinned: false,
                journal_date: None,
                imported_at: None,
                derived: super::LoomBlockDerived {
                    full_text_index: Some("file scoped mention target".into()),
                    ..Default::default()
                },
            },
        )
        .await?;

    let asset = db
        .create_asset(
            &ctx,
            NewAsset {
                workspace_id: workspace.id.clone(),
                kind: "original".into(),
                mime: "text/plain".into(),
                original_filename: Some("portable plan.txt".into()),
                content_hash: format!("{:064x}", 42_u32),
                size_bytes: 128,
                width: None,
                height: None,
                classification: "low".into(),
                exportable: true,
                is_proxy_of: None,
                proxy_asset_id: None,
            },
        )
        .await?;

    let file_block = db
        .create_loom_block(
            &ctx,
            NewLoomBlock {
                block_id: None,
                workspace_id: workspace.id.clone(),
                content_type: LoomBlockContentType::File,
                document_id: None,
                asset_id: Some(asset.asset_id.clone()),
                title: Some("Transport".into()),
                original_filename: asset.original_filename.clone(),
                content_hash: Some(asset.content_hash.clone()),
                pinned: false,
                journal_date: None,
                imported_at: Some(Utc::now()),
                derived: super::LoomBlockDerived {
                    full_text_index: Some("document archive".into()),
                    ..Default::default()
                },
            },
        )
        .await?;

    let by_hash = db
        .find_loom_block_by_content_hash(&workspace.id, &asset.content_hash)
        .await?
        .ok_or(StorageError::NotFound("loom_block_by_content_hash"))?;
    assert_eq!(by_hash.block_id, file_block.block_id);

    let by_asset = db
        .find_loom_block_by_asset_id(&workspace.id, &asset.asset_id)
        .await?
        .ok_or(StorageError::NotFound("loom_block_by_asset_id"))?;
    assert_eq!(by_asset.block_id, file_block.block_id);

    let portable_note = db
        .update_loom_block(
            &ctx,
            &workspace.id,
            &portable_note.block_id,
            super::LoomBlockUpdate {
                title: Some("Portable".into()),
                pinned: Some(true),
                journal_date: Some("2026-03-14".into()),
            },
        )
        .await?;
    assert!(portable_note.pinned);
    assert_eq!(portable_note.journal_date.as_deref(), Some("2026-03-14"));

    let anchor = LoomSourceAnchor {
        document_id: document.id.clone(),
        block_id: source_block.id.clone(),
        offset_start: 3,
        offset_end: 12,
    };

    let tag_edge = db
        .create_loom_edge(
            &ctx,
            NewLoomEdge {
                edge_id: None,
                workspace_id: workspace.id.clone(),
                source_block_id: portable_note.block_id.clone(),
                target_block_id: tag_hub.block_id.clone(),
                edge_type: LoomEdgeType::Tag,
                created_by: LoomEdgeCreatedBy::User,
                crdt_site_id: Some("site-a".into()),
                source_anchor: Some(anchor.clone()),
            },
        )
        .await?;
    let mention_edge = db
        .create_loom_edge(
            &ctx,
            NewLoomEdge {
                edge_id: None,
                workspace_id: workspace.id.clone(),
                source_block_id: portable_note.block_id.clone(),
                target_block_id: mention_target.block_id.clone(),
                edge_type: LoomEdgeType::Mention,
                created_by: LoomEdgeCreatedBy::User,
                crdt_site_id: None,
                source_anchor: Some(anchor.clone()),
            },
        )
        .await?;
    let _file_tag_edge = db
        .create_loom_edge(
            &ctx,
            NewLoomEdge {
                edge_id: None,
                workspace_id: workspace.id.clone(),
                source_block_id: file_block.block_id.clone(),
                target_block_id: tag_hub.block_id.clone(),
                edge_type: LoomEdgeType::Tag,
                created_by: LoomEdgeCreatedBy::User,
                crdt_site_id: None,
                source_anchor: None,
            },
        )
        .await?;
    let _file_mention_edge = db
        .create_loom_edge(
            &ctx,
            NewLoomEdge {
                edge_id: None,
                workspace_id: workspace.id.clone(),
                source_block_id: file_block.block_id.clone(),
                target_block_id: file_only_target.block_id.clone(),
                edge_type: LoomEdgeType::Mention,
                created_by: LoomEdgeCreatedBy::User,
                crdt_site_id: None,
                source_anchor: None,
            },
        )
        .await?;

    let portable_note = db
        .get_loom_block(&workspace.id, &portable_note.block_id)
        .await?;
    assert_eq!(portable_note.derived.mention_count, 1);
    assert_eq!(portable_note.derived.tag_count, 1);
    assert_eq!(portable_note.derived.backlink_count, 0);

    let mention_target_loaded = db
        .get_loom_block(&workspace.id, &mention_target.block_id)
        .await?;
    assert_eq!(mention_target_loaded.derived.backlink_count, 1);

    let tag_hub_loaded = db.get_loom_block(&workspace.id, &tag_hub.block_id).await?;
    assert_eq!(tag_hub_loaded.derived.backlink_count, 2);

    let file_block_loaded = db
        .get_loom_block(&workspace.id, &file_block.block_id)
        .await?;
    assert_eq!(file_block_loaded.derived.tag_count, 1);

    loom_metrics_recompute_idempotent(
        &db,
        &workspace.id,
        &portable_note.block_id,
        &mention_target.block_id,
        &tag_hub.block_id,
    )
    .await?;

    let note_edges = db
        .list_loom_edges_for_block(&workspace.id, &portable_note.block_id)
        .await?;
    assert_eq!(note_edges.len(), 2);
    let round_tripped_anchor = note_edges
        .iter()
        .find(|edge| edge.edge_id == mention_edge.edge_id)
        .and_then(|edge| edge.source_anchor.as_ref())
        .ok_or(StorageError::NotFound("loom_source_anchor"))?;
    assert_eq!(round_tripped_anchor.document_id, anchor.document_id);
    assert_eq!(round_tripped_anchor.block_id, anchor.block_id);
    assert_eq!(round_tripped_anchor.offset_start, anchor.offset_start);
    assert_eq!(round_tripped_anchor.offset_end, anchor.offset_end);

    let all_notes = db
        .query_loom_view(
            &workspace.id,
            LoomViewType::All,
            LoomViewFilters {
                content_type: Some(LoomBlockContentType::Note),
                ..Default::default()
            },
            50,
            0,
        )
        .await?;
    let LoomViewResponse::All { blocks } = all_notes else {
        panic!("expected all view response");
    };
    assert_eq!(
        loom_block_ids(&blocks),
        sorted_strings(vec![
            file_only_target.block_id.clone(),
            portable_note.block_id.clone(),
            unlinked_note.block_id.clone(),
            mention_target.block_id.clone(),
        ])
    );

    let future_notes = db
        .query_loom_view(
            &workspace.id,
            LoomViewType::All,
            LoomViewFilters {
                date_from: Some(Utc::now() + Duration::days(1)),
                ..Default::default()
            },
            50,
            0,
        )
        .await?;
    let LoomViewResponse::All {
        blocks: future_blocks,
    } = future_notes
    else {
        panic!("expected all view response");
    };
    assert!(future_blocks.is_empty());

    let pinned = db
        .query_loom_view(
            &workspace.id,
            LoomViewType::Pins,
            LoomViewFilters::default(),
            50,
            0,
        )
        .await?;
    let LoomViewResponse::Pins { blocks } = pinned else {
        panic!("expected pins view response");
    };
    assert_eq!(
        loom_block_ids(&blocks),
        vec![portable_note.block_id.clone()]
    );

    let tagged_notes = db
        .query_loom_view(
            &workspace.id,
            LoomViewType::All,
            LoomViewFilters {
                content_type: Some(LoomBlockContentType::Note),
                tag_ids: vec![tag_hub.block_id.clone()],
                ..Default::default()
            },
            50,
            0,
        )
        .await?;
    let LoomViewResponse::All { blocks } = tagged_notes else {
        panic!("expected all view response");
    };
    assert_eq!(
        loom_block_ids(&blocks),
        vec![portable_note.block_id.clone()]
    );

    let mentioned_notes = db
        .query_loom_view(
            &workspace.id,
            LoomViewType::All,
            LoomViewFilters {
                content_type: Some(LoomBlockContentType::Note),
                mention_ids: vec![mention_target.block_id.clone()],
                ..Default::default()
            },
            50,
            0,
        )
        .await?;
    let LoomViewResponse::All { blocks } = mentioned_notes else {
        panic!("expected all view response");
    };
    assert_eq!(
        loom_block_ids(&blocks),
        vec![portable_note.block_id.clone()]
    );

    let mime_blocks = db
        .query_loom_view(
            &workspace.id,
            LoomViewType::All,
            LoomViewFilters {
                mime: Some("text/plain".into()),
                ..Default::default()
            },
            50,
            0,
        )
        .await?;
    let LoomViewResponse::All { blocks } = mime_blocks else {
        panic!("expected all view response");
    };
    assert_eq!(loom_block_ids(&blocks), vec![file_block.block_id.clone()]);

    let unlinked = db
        .query_loom_view(
            &workspace.id,
            LoomViewType::Unlinked,
            LoomViewFilters::default(),
            50,
            0,
        )
        .await?;
    let LoomViewResponse::Unlinked { blocks } = unlinked else {
        panic!("expected unlinked view response");
    };
    assert_eq!(
        loom_block_ids(&blocks),
        vec![unlinked_note.block_id.clone()]
    );

    let sorted_notes = db
        .query_loom_view(
            &workspace.id,
            LoomViewType::Sorted,
            LoomViewFilters {
                content_type: Some(LoomBlockContentType::Note),
                ..Default::default()
            },
            50,
            0,
        )
        .await?;
    let sorted_groups = sorted_view_groups(&sorted_notes);
    assert_eq!(sorted_groups.len(), 2);
    assert_eq!(
        sorted_groups.get(&format!("mention:{}", mention_target.block_id)),
        Some(&vec![portable_note.block_id.clone()])
    );
    assert_eq!(
        sorted_groups.get(&format!("tag:{}", tag_hub.block_id)),
        Some(&vec![portable_note.block_id.clone()])
    );

    let paged_sorted_notes = db
        .query_loom_view(
            &workspace.id,
            LoomViewType::Sorted,
            LoomViewFilters {
                content_type: Some(LoomBlockContentType::Note),
                ..Default::default()
            },
            1,
            0,
        )
        .await?;
    let paged_groups = sorted_view_groups(&paged_sorted_notes);
    assert_eq!(paged_groups.len(), 1);
    assert_eq!(
        paged_groups.get(&format!("mention:{}", mention_target.block_id)),
        Some(&vec![portable_note.block_id.clone()])
    );

    let tagged_note_search = db
        .search_loom_blocks(
            &workspace.id,
            "portable parity",
            LoomSearchFilters {
                content_type: Some(LoomBlockContentType::Note),
                tag_ids: vec![tag_hub.block_id.clone()],
                ..Default::default()
            },
            50,
            0,
        )
        .await?;
    assert_eq!(
        loom_search_ids(&tagged_note_search),
        vec![portable_note.block_id.clone()]
    );

    let filename_search = db
        .search_loom_blocks(
            &workspace.id,
            "plan",
            LoomSearchFilters {
                mime: Some("text/plain".into()),
                ..Default::default()
            },
            50,
            0,
        )
        .await?;
    assert_eq!(
        loom_search_ids(&filename_search),
        vec![file_block.block_id.clone()]
    );

    let metadata_only_search = db
        .search_loom_blocks(
            &workspace.id,
            "metadata_shadow",
            LoomSearchFilters {
                content_type: Some(LoomBlockContentType::Note),
                ..Default::default()
            },
            50,
            0,
        )
        .await?;
    assert!(
        metadata_only_search.is_empty(),
        "metadata-only derived fields must not be searchable"
    );

    let literal_percent_search = db
        .search_loom_blocks(
            &workspace.id,
            "%",
            LoomSearchFilters {
                content_type: Some(LoomBlockContentType::Note),
                ..Default::default()
            },
            50,
            0,
        )
        .await?;
    assert!(
        literal_percent_search.is_empty(),
        "literal wildcard characters must not broad-match by backend"
    );

    let literal_underscore_search = db
        .search_loom_blocks(
            &workspace.id,
            "_",
            LoomSearchFilters {
                content_type: Some(LoomBlockContentType::Note),
                ..Default::default()
            },
            50,
            0,
        )
        .await?;
    assert!(
        literal_underscore_search.is_empty(),
        "literal wildcard characters must not broad-match by backend"
    );

    let graph_fixture = build_loom_graph_fixture(&db, &ctx, &workspace.id, &document.id).await?;
    loom_traverse_graph_depth_limit(&db, &workspace.id, &graph_fixture).await?;
    loom_traverse_graph_cycle_detection(&db, &workspace.id, &graph_fixture).await?;
    loom_traverse_graph_edge_type_filter(&db, &workspace.id, &graph_fixture).await?;
    loom_search_graph_filter_postgres(&db, &workspace.id, &graph_fixture).await?;
    loom_directional_edge_queries(
        &db,
        &ctx,
        &workspace.id,
        &document.id,
        &portable_note.block_id,
        &[tag_edge.edge_id.clone(), mention_edge.edge_id.clone()],
    )
    .await?;
    loom_source_anchor_round_trip(&db, &ctx, &workspace.id, &document.id, &anchor).await?;

    let removed_edge = db
        .delete_loom_edge(&ctx, &workspace.id, &mention_edge.edge_id)
        .await?;
    assert_eq!(removed_edge.edge_id, mention_edge.edge_id);
    let portable_note_after_delete = db
        .get_loom_block(&workspace.id, &portable_note.block_id)
        .await?;
    let mention_target_after_delete = db
        .get_loom_block(&workspace.id, &mention_target.block_id)
        .await?;
    assert_eq!(portable_note_after_delete.derived.mention_count, 0);
    assert_eq!(mention_target_after_delete.derived.backlink_count, 0);

    let removed_tag = db
        .delete_loom_edge(&ctx, &workspace.id, &tag_edge.edge_id)
        .await?;
    assert_eq!(removed_tag.edge_id, tag_edge.edge_id);
    let portable_note_after_tag_delete = db
        .get_loom_block(&workspace.id, &portable_note.block_id)
        .await?;
    assert_eq!(portable_note_after_tag_delete.derived.tag_count, 0);

    let delete_target_source = db
        .create_loom_block(
            &ctx,
            NewLoomBlock {
                block_id: None,
                workspace_id: workspace.id.clone(),
                content_type: LoomBlockContentType::Note,
                document_id: Some(document.id.clone()),
                asset_id: None,
                title: Some("Delete Target Source".into()),
                original_filename: None,
                content_hash: None,
                pinned: false,
                journal_date: None,
                imported_at: None,
                derived: super::LoomBlockDerived {
                    full_text_index: Some("delete target source".into()),
                    ..Default::default()
                },
            },
        )
        .await?;
    let delete_mention_target = db
        .create_loom_block(
            &ctx,
            NewLoomBlock {
                block_id: None,
                workspace_id: workspace.id.clone(),
                content_type: LoomBlockContentType::Note,
                document_id: Some(document.id.clone()),
                asset_id: None,
                title: Some("Delete Mention Target".into()),
                original_filename: None,
                content_hash: None,
                pinned: false,
                journal_date: None,
                imported_at: None,
                derived: super::LoomBlockDerived {
                    full_text_index: Some("delete mention target".into()),
                    ..Default::default()
                },
            },
        )
        .await?;
    let delete_tag_target = db
        .create_loom_block(
            &ctx,
            NewLoomBlock {
                block_id: None,
                workspace_id: workspace.id.clone(),
                content_type: LoomBlockContentType::TagHub,
                document_id: None,
                asset_id: None,
                title: Some("Delete Tag Target".into()),
                original_filename: None,
                content_hash: None,
                pinned: false,
                journal_date: None,
                imported_at: None,
                derived: super::LoomBlockDerived {
                    full_text_index: Some("delete tag target".into()),
                    ..Default::default()
                },
            },
        )
        .await?;

    db.create_loom_edge(
        &ctx,
        NewLoomEdge {
            edge_id: None,
            workspace_id: workspace.id.clone(),
            source_block_id: delete_target_source.block_id.clone(),
            target_block_id: delete_mention_target.block_id.clone(),
            edge_type: LoomEdgeType::Mention,
            created_by: LoomEdgeCreatedBy::User,
            crdt_site_id: None,
            source_anchor: None,
        },
    )
    .await?;
    db.create_loom_edge(
        &ctx,
        NewLoomEdge {
            edge_id: None,
            workspace_id: workspace.id.clone(),
            source_block_id: delete_target_source.block_id.clone(),
            target_block_id: delete_tag_target.block_id.clone(),
            edge_type: LoomEdgeType::Tag,
            created_by: LoomEdgeCreatedBy::User,
            crdt_site_id: None,
            source_anchor: None,
        },
    )
    .await?;

    let delete_target_source_before = db
        .get_loom_block(&workspace.id, &delete_target_source.block_id)
        .await?;
    let delete_mention_target_before = db
        .get_loom_block(&workspace.id, &delete_mention_target.block_id)
        .await?;
    assert_eq!(delete_target_source_before.derived.mention_count, 1);
    assert_eq!(delete_target_source_before.derived.tag_count, 1);
    assert_eq!(delete_mention_target_before.derived.backlink_count, 1);

    db.delete_loom_block(&ctx, &workspace.id, &delete_mention_target.block_id)
        .await?;
    let delete_target_source_after_mention_delete = db
        .get_loom_block(&workspace.id, &delete_target_source.block_id)
        .await?;
    assert_eq!(
        delete_target_source_after_mention_delete
            .derived
            .mention_count,
        0
    );
    assert_eq!(
        delete_target_source_after_mention_delete.derived.tag_count,
        1
    );

    db.delete_loom_block(&ctx, &workspace.id, &delete_tag_target.block_id)
        .await?;
    let delete_target_source_after_tag_delete = db
        .get_loom_block(&workspace.id, &delete_target_source.block_id)
        .await?;
    assert_eq!(delete_target_source_after_tag_delete.derived.tag_count, 0);

    let delete_source_block = db
        .create_loom_block(
            &ctx,
            NewLoomBlock {
                block_id: None,
                workspace_id: workspace.id.clone(),
                content_type: LoomBlockContentType::Note,
                document_id: Some(document.id.clone()),
                asset_id: None,
                title: Some("Delete Source Block".into()),
                original_filename: None,
                content_hash: None,
                pinned: false,
                journal_date: None,
                imported_at: None,
                derived: super::LoomBlockDerived {
                    full_text_index: Some("delete source block".into()),
                    ..Default::default()
                },
            },
        )
        .await?;
    let surviving_backlink_target = db
        .create_loom_block(
            &ctx,
            NewLoomBlock {
                block_id: None,
                workspace_id: workspace.id.clone(),
                content_type: LoomBlockContentType::Note,
                document_id: Some(document.id.clone()),
                asset_id: None,
                title: Some("Surviving Backlink Target".into()),
                original_filename: None,
                content_hash: None,
                pinned: false,
                journal_date: None,
                imported_at: None,
                derived: super::LoomBlockDerived {
                    full_text_index: Some("surviving backlink target".into()),
                    ..Default::default()
                },
            },
        )
        .await?;

    db.create_loom_edge(
        &ctx,
        NewLoomEdge {
            edge_id: None,
            workspace_id: workspace.id.clone(),
            source_block_id: delete_source_block.block_id.clone(),
            target_block_id: surviving_backlink_target.block_id.clone(),
            edge_type: LoomEdgeType::Mention,
            created_by: LoomEdgeCreatedBy::User,
            crdt_site_id: None,
            source_anchor: None,
        },
    )
    .await?;

    let surviving_backlink_target_before = db
        .get_loom_block(&workspace.id, &surviving_backlink_target.block_id)
        .await?;
    assert_eq!(surviving_backlink_target_before.derived.backlink_count, 1);

    db.delete_loom_block(&ctx, &workspace.id, &delete_source_block.block_id)
        .await?;
    let surviving_backlink_target_after = db
        .get_loom_block(&workspace.id, &surviving_backlink_target.block_id)
        .await?;
    assert_eq!(surviving_backlink_target_after.derived.backlink_count, 0);

    db.delete_loom_block(&ctx, &workspace.id, &unlinked_note.block_id)
        .await?;
    assert!(matches!(
        db.get_loom_block(&workspace.id, &unlinked_note.block_id)
            .await,
        Err(StorageError::NotFound("loom_block"))
    ));

    Ok(())
}

#[allow(dead_code)]
pub async fn run_loom_traversal_performance_probe(
    db: Arc<dyn super::Database>,
) -> StorageResult<()> {
    db.ping().await?;

    let ctx = WriteContext::human(Some("loom-perf".into()));
    let workspace = db
        .create_workspace(
            &ctx,
            NewWorkspace {
                name: format!("loom-perf-ws-{}", Uuid::new_v4()),
            },
        )
        .await?;

    loom_traverse_graph_meets_performance_target(&db, &workspace.id).await?;
    Ok(())
}

#[allow(dead_code)]
pub async fn run_calendar_storage_conformance(db: Arc<dyn super::Database>) -> StorageResult<()> {
    db.ping().await?;

    let ctx = WriteContext::human(Some("calendar-tester".into()));
    let workspace = db
        .create_workspace(
            &ctx,
            NewWorkspace {
                name: format!("calendar-ws-{}", Uuid::new_v4()),
            },
        )
        .await?;

    let source_id = format!("google:test:{}", Uuid::new_v4());
    let source = db
        .upsert_calendar_source(
            &ctx,
            CalendarSourceUpsert {
                id: source_id.clone(),
                workspace_id: workspace.id.clone(),
                display_name: "Google / Test".into(),
                provider_type: CalendarSourceProviderType::Google,
                write_policy: CalendarSourceWritePolicy::TwoWayMirror,
                default_tzid: "Europe/Brussels".into(),
                auto_export: true,
                credentials_ref: Some("cred:test".into()),
                provider_calendar_id: Some("primary".into()),
                capability_profile_id: Some("calendar-google".into()),
                config: json!({"calendar_id": "primary", "color": "blue"}),
                sync_state: CalendarSourceSyncState {
                    state: None,
                    sync_token: Some("sync-token-1".into()),
                    last_synced_at: Some(Utc::now()),
                    last_full_sync_at: Some(Utc::now()),
                    last_ok_at: None,
                    last_pull_at: None,
                    last_push_at: None,
                    last_error_at: None,
                    last_error_code: None,
                    last_error: None,
                    backoff_until: None,
                    consecutive_failures: Some(0),
                    last_remote_watermark: Some("etag-1".into()),
                    last_local_applied_rev: Some(1),
                },
            },
        )
        .await?;

    let updated_source = db
        .upsert_calendar_source(
            &ctx,
            CalendarSourceUpsert {
                id: source.id.clone(),
                workspace_id: workspace.id.clone(),
                display_name: "Google / Updated".into(),
                provider_type: CalendarSourceProviderType::Google,
                write_policy: CalendarSourceWritePolicy::TwoWayMirror,
                default_tzid: "Europe/Brussels".into(),
                auto_export: true,
                credentials_ref: Some("cred:test".into()),
                provider_calendar_id: Some("primary".into()),
                capability_profile_id: Some("calendar-google".into()),
                config: json!({"calendar_id": "primary", "color": "green"}),
                sync_state: CalendarSourceSyncState {
                    state: None,
                    sync_token: Some("sync-token-2".into()),
                    last_synced_at: Some(Utc::now()),
                    last_full_sync_at: Some(Utc::now()),
                    last_ok_at: Some(Utc::now()),
                    last_pull_at: None,
                    last_push_at: None,
                    last_error_at: None,
                    last_error_code: None,
                    last_error: None,
                    backoff_until: None,
                    consecutive_failures: Some(0),
                    last_remote_watermark: Some("etag-2".into()),
                    last_local_applied_rev: Some(2),
                },
            },
        )
        .await?;
    assert_eq!(updated_source.id, source.id);
    assert_eq!(updated_source.display_name, "Google / Updated");
    assert_eq!(
        updated_source.sync_state.sync_token.as_deref(),
        Some("sync-token-2")
    );

    let listed_sources = db.list_calendar_sources(&workspace.id).await?;
    assert_eq!(listed_sources.len(), 1);
    let fetched_source = db
        .get_calendar_source(&workspace.id, &source.id)
        .await?
        .ok_or(StorageError::NotFound("calendar_source"))?;
    assert_eq!(fetched_source.display_name, "Google / Updated");

    let provider_start = Utc::now() + Duration::days(1);
    let provider_end = provider_start + Duration::hours(1);
    let original_provider_event = db
        .upsert_calendar_event(
            &ctx,
            CalendarEventUpsert {
                id: Uuid::new_v4().to_string(),
                workspace_id: workspace.id.clone(),
                source_id: source.id.clone(),
                external_id: Some("provider-event-1".into()),
                external_etag: Some("etag-1".into()),
                title: "Provider event".into(),
                description: Some("initial".into()),
                location: Some("Room A".into()),
                start_ts_utc: provider_start,
                end_ts_utc: provider_end,
                start_local: Some("2026-03-07T09:00:00".into()),
                end_local: Some("2026-03-07T10:00:00".into()),
                tzid: "Europe/Brussels".into(),
                all_day: false,
                was_floating: false,
                status: CalendarEventStatus::Confirmed,
                visibility: CalendarEventVisibility::Private,
                export_mode: CalendarEventExportMode::FullExport,
                rrule: Some("FREQ=WEEKLY".into()),
                rdate: Vec::new(),
                exdate: Vec::new(),
                is_recurring: true,
                series_id: Some("series-1".into()),
                instance_key: Some("instance-1".into()),
                is_override: false,
                source_last_seen_at: Some(Utc::now()),
                attendees: json!([{ "email": "person@example.com" }]),
                links: json!([{ "type": "doc", "target": "doc-1" }]),
                provider_payload: Some(json!({"raw": "payload-1"})),
            },
        )
        .await?;

    let duplicate_provider_event = db
        .upsert_calendar_event(
            &ctx,
            CalendarEventUpsert {
                id: Uuid::new_v4().to_string(),
                workspace_id: workspace.id.clone(),
                source_id: source.id.clone(),
                external_id: Some("provider-event-1".into()),
                external_etag: Some("etag-2".into()),
                title: "Provider event updated".into(),
                description: Some("updated".into()),
                location: Some("Room B".into()),
                start_ts_utc: provider_start,
                end_ts_utc: provider_end + Duration::minutes(30),
                start_local: Some("2026-03-07T09:00:00".into()),
                end_local: Some("2026-03-07T10:30:00".into()),
                tzid: "Europe/Brussels".into(),
                all_day: false,
                was_floating: false,
                status: CalendarEventStatus::Tentative,
                visibility: CalendarEventVisibility::BusyOnly,
                export_mode: CalendarEventExportMode::BusyOnly,
                rrule: Some("FREQ=WEEKLY".into()),
                rdate: vec!["2026-03-08T09:00:00".into()],
                exdate: vec!["2026-03-15T09:00:00".into()],
                is_recurring: true,
                series_id: Some("series-1".into()),
                instance_key: Some("instance-1".into()),
                is_override: true,
                source_last_seen_at: Some(Utc::now()),
                attendees: json!([{ "email": "updated@example.com" }]),
                links: json!([{ "type": "canvas", "target": "canvas-1" }]),
                provider_payload: Some(json!({"raw": "payload-2"})),
            },
        )
        .await?;

    assert_eq!(duplicate_provider_event.id, original_provider_event.id);
    assert_eq!(duplicate_provider_event.title, "Provider event updated");
    assert_eq!(
        duplicate_provider_event.external_etag.as_deref(),
        Some("etag-2")
    );
    assert!(duplicate_provider_event.is_override);

    let local_start = provider_start + Duration::hours(5);
    let local_end = local_start + Duration::hours(2);
    let local_event = db
        .upsert_calendar_event(
            &ctx,
            CalendarEventUpsert {
                id: Uuid::new_v4().to_string(),
                workspace_id: workspace.id.clone(),
                source_id: source.id.clone(),
                external_id: None,
                external_etag: None,
                title: "Local draft".into(),
                description: Some("local-only".into()),
                location: None,
                start_ts_utc: local_start,
                end_ts_utc: local_end,
                start_local: Some("2026-03-07T14:00:00".into()),
                end_local: Some("2026-03-07T16:00:00".into()),
                tzid: "Europe/Brussels".into(),
                all_day: false,
                was_floating: true,
                status: CalendarEventStatus::Confirmed,
                visibility: CalendarEventVisibility::Private,
                export_mode: CalendarEventExportMode::LocalOnly,
                rrule: None,
                rdate: Vec::new(),
                exdate: Vec::new(),
                is_recurring: false,
                series_id: None,
                instance_key: None,
                is_override: false,
                source_last_seen_at: None,
                attendees: json!([]),
                links: json!([]),
                provider_payload: None,
            },
        )
        .await?;
    assert_eq!(local_event.external_id, None);
    assert!(local_event.was_floating);

    let matching_events = db
        .query_calendar_events(CalendarEventWindowQuery {
            workspace_id: workspace.id.clone(),
            window_start_utc: provider_start - Duration::minutes(15),
            window_end_utc: local_end + Duration::minutes(15),
            source_ids: vec![source.id.clone()],
        })
        .await?;
    assert_eq!(matching_events.len(), 2);
    assert_eq!(matching_events[0].id, original_provider_event.id);
    assert_eq!(matching_events[1].id, local_event.id);

    let narrow_window = db
        .query_calendar_events(CalendarEventWindowQuery {
            workspace_id: workspace.id.clone(),
            window_start_utc: provider_start - Duration::minutes(15),
            window_end_utc: provider_end + Duration::minutes(15),
            source_ids: Vec::new(),
        })
        .await?;
    assert_eq!(narrow_window.len(), 1);
    assert_eq!(narrow_window[0].id, original_provider_event.id);

    db.delete_calendar_data_by_source(&ctx, &workspace.id, &source.id)
        .await?;

    let no_sources = db.list_calendar_sources(&workspace.id).await?;
    assert!(no_sources.is_empty());
    let no_events = db
        .query_calendar_events(CalendarEventWindowQuery {
            workspace_id: workspace.id.clone(),
            window_start_utc: provider_start - Duration::minutes(15),
            window_end_utc: local_end + Duration::minutes(15),
            source_ids: Vec::new(),
        })
        .await?;
    assert!(no_events.is_empty());

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
async fn loom_migration_schema_is_portable_sqlite() -> StorageResult<()> {
    let migrator = sqlx::migrate!("./migrations");
    let mut conn = sqlx::SqliteConnection::connect("sqlite::memory:").await?;

    migrator.run(&mut conn).await?;

    let tables = sqlite_user_table_names(&mut conn).await?;
    assert!(tables.iter().any(|name| name == "assets"));
    assert!(tables.iter().any(|name| name == "loom_blocks"));
    assert!(tables.iter().any(|name| name == "loom_edges"));
    assert!(
        !tables.iter().any(|name| name == "loom_blocks_fts"),
        "provider-local SQLite FTS tables must not be part of portable migration DDL"
    );

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
async fn loom_migration_schema_is_portable_postgres() -> StorageResult<()> {
    let Some(url) = postgres_test_url() else {
        return Ok(());
    };

    let mut conn = sqlx::PgConnection::connect(&url).await?;
    let schema = format!("wp1_loom_mig_{}", Uuid::new_v4().simple());

    sqlx::query(&format!("CREATE SCHEMA {schema}"))
        .execute(&mut conn)
        .await?;
    sqlx::query(&format!("SET search_path TO {schema}"))
        .execute(&mut conn)
        .await?;

    let migrator = sqlx::migrate!("./migrations");
    migrator.run(&mut conn).await?;

    let tables = postgres_user_table_names(&mut conn).await?;
    assert!(tables.iter().any(|name| name == "assets"));
    assert!(tables.iter().any(|name| name == "loom_blocks"));
    assert!(tables.iter().any(|name| name == "loom_edges"));
    assert!(
        !tables.iter().any(|name| name == "loom_blocks_fts"),
        "provider-local search structures must not be part of portable PostgreSQL migration DDL"
    );

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

#[test]
fn database_trait_purity_source_regressions() {
    let storage_mod = include_str!("mod.rs");
    let workflows_prod = include_str!("../workflows.rs")
        .split("#[cfg(test)]")
        .next()
        .unwrap_or_default();
    let loom_api_prod = include_str!("../api/loom.rs")
        .split("#[cfg(test)]")
        .next()
        .unwrap_or_default();

    assert!(storage_mod.contains("pub trait StructuredCollaborationStore"));
    assert!(storage_mod.contains("pub trait StorageCapabilityStore"));
    assert!(!workflows_prod.contains("crate::storage::locus_sqlite::"));
    assert!(!workflows_prod.contains("downcast_ref::<crate::storage::sqlite::SqliteDatabase>()"));
    assert!(!loom_api_prod.contains(".as_any()"));
}

#[tokio::test]
async fn database_trait_purity_capability_snapshot_reports_sqlite() -> StorageResult<()> {
    let db = sqlite_backend().await?;
    let caps = db.storage_capabilities();

    assert_eq!(caps.backend, StorageBackendKind::Sqlite);
    assert!(caps.supports_structured_collab_artifacts);
    assert!(!caps.supports_loom_graph_filtering);
    assert_eq!(caps.loom_search_observability_tier(), 1);

    Ok(())
}

#[tokio::test]
async fn database_trait_purity_capability_snapshot_reports_postgres() -> StorageResult<()> {
    if postgres_test_url().is_none() {
        return Ok(());
    }

    let db = postgres_backend_from_env().await?;
    let caps = db.storage_capabilities();

    assert_eq!(caps.backend, StorageBackendKind::Postgres);
    assert!(!caps.supports_structured_collab_artifacts);
    assert!(caps.supports_loom_graph_filtering);
    assert_eq!(caps.loom_search_observability_tier(), 2);

    Ok(())
}
