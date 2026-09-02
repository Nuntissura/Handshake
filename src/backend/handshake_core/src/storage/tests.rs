//! Shared storage conformance suites and the embedded-store test harness.
//!
//! The conformance suites below take `Arc<dyn Database>` and are deliberately
//! BACKEND-AGNOSTIC: they describe what Handshake storage must do, not how any
//! one engine does it. That is why the PostgreSQL removal did not touch them -
//! only the harness that produces a backend changed.
//!
//! Test isolation is now a DIRECTORY, not a schema. Each harness call opens a
//! fresh embedded SurrealDB store under its own data dir inside the external
//! artifacts root, so two suites running at once cannot see each other's rows
//! and neither needs a server, a connection string, or a cleanup DROP SCHEMA.

use super::surreal::{SurrealDatabase, SurrealStorage, SurrealStorageConfig};
#[allow(unused_imports)]
use super::{
    AccessMode, BlockUpdate, CalendarEventExportMode, CalendarEventStatus, CalendarEventUpsert,
    CalendarEventVisibility, CalendarEventWindowQuery, CalendarSourceProviderType,
    CalendarSourceSyncState, CalendarSourceUpsert, CalendarSourceWritePolicy,
    ControlPlaneStorageConfig, ControlPlaneStorageMode, Database, DefaultStorageGuard, EntityRef,
    GuardError, JobKind, JobMetrics, JobState, JobStatusUpdate, LoomBlock, LoomBlockContentType,
    LoomBlockSearchResult, LoomEdgeCreatedBy, LoomEdgeType, LoomSearchFilters, LoomSourceAnchor,
    LoomViewFilters, LoomViewResponse, LoomViewType, NewAiJob, NewAsset, NewBlock, NewCanvas,
    NewCanvasEdge, NewCanvasNode, NewDocument, NewLoomBlock, NewLoomEdge, NewNodeExecution,
    NewWorkspace, OperationType, PlannedOperation, SafetyMode, StorageBackendKind,
    StorageCapabilityStore, StorageError, StorageGuard, StorageResult,
    StructuredCollaborationStore, WriteContext,
};
use crate::workflows::locus;
use chrono::{Duration, NaiveDate, TimeZone, Utc};
use serde_json::json;
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex as StdMutex};
use std::time::Instant;
use surrealdb::types::SurrealValue;
use uuid::Uuid;

#[cfg(test)]
const NIL_EDIT_EVENT_ID: &str = "00000000-0000-0000-0000-000000000000";
const LOOM_TRAVERSAL_PERF_TOTAL_BLOCKS: usize = 10_000;

/// Root for per-test store directories.
///
/// Resolved only from an explicit absolute `HANDSHAKE_ARTIFACTS_ROOT`.
/// Relative/current-directory fallbacks are forbidden because a fixture may be
/// launched from several worktrees and must never create a second artifact root.
fn test_store_root() -> StorageResult<PathBuf> {
    let configured = std::env::var_os("HANDSHAKE_ARTIFACTS_ROOT").ok_or_else(|| {
        StorageError::Database(
            "HANDSHAKE_ARTIFACTS_ROOT must name the absolute _Artifacts root for embedded tests"
                .to_owned(),
        )
    })?;
    let configured = PathBuf::from(configured);
    if !configured.is_absolute() {
        return Err(StorageError::Database(format!(
            "HANDSHAKE_ARTIFACTS_ROOT must be absolute, got {}",
            configured.display()
        )));
    }
    std::fs::create_dir_all(&configured).map_err(|error| {
        StorageError::Database(format!(
            "could not create embedded-test artifacts root {}: {error}",
            configured.display()
        ))
    })?;
    let artifacts_root = dunce::canonicalize(&configured).map_err(|error| {
        StorageError::Database(format!(
            "could not resolve embedded-test artifacts root {}: {error}",
            configured.display()
        ))
    })?;
    let store_root = artifacts_root
        .join("handshake-test")
        .join("storage-conformance");
    std::fs::create_dir_all(&store_root).map_err(|error| {
        StorageError::Database(format!(
            "could not create embedded-test store root {}: {error}",
            store_root.display()
        ))
    })?;
    let store_root = dunce::canonicalize(&store_root).map_err(|error| {
        StorageError::Database(format!(
            "could not resolve embedded-test store root {}: {error}",
            store_root.display()
        ))
    })?;
    if !store_root.starts_with(&artifacts_root) {
        return Err(StorageError::Database(format!(
            "embedded-test store root escaped HANDSHAKE_ARTIFACTS_ROOT: {}",
            store_root.display()
        )));
    }
    Ok(store_root)
}

/// A live embedded store plus the `Database` handle over it.
///
/// `storage` is exposed so a test can close the store deterministically and
/// prove restart behaviour by reopening the same directory.
#[derive(Clone)]
pub struct EmbeddedTestBackend {
    pub database: Arc<dyn super::Database>,
    pub storage: SurrealStorage,
    pub data_dir: PathBuf,
    cleanup: Arc<TestStoreCleanupGuard>,
}

struct TestStoreCleanupGuard {
    storage: StdMutex<Option<SurrealStorage>>,
    data_dir: PathBuf,
}

impl TestStoreCleanupGuard {
    fn take_storage(&self) -> Option<SurrealStorage> {
        match self.storage.lock() {
            Ok(mut storage) => storage.take(),
            Err(poisoned) => poisoned.into_inner().take(),
        }
    }

    async fn cleanup(&self) -> StorageResult<()> {
        let Some(storage) = self.take_storage() else {
            return Ok(());
        };
        shutdown_and_remove_test_store(storage, self.data_dir.clone()).await
    }
}

impl Drop for TestStoreCleanupGuard {
    fn drop(&mut self) {
        let Some(storage) = self.take_storage() else {
            return;
        };
        let data_dir = self.data_dir.clone();
        let cleanup = std::thread::Builder::new()
            .name("handshake-test-store-cleanup".to_owned())
            .spawn(move || {
                let runtime = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .map_err(|error| {
                        StorageError::Database(format!(
                            "could not build embedded-test cleanup runtime: {error}"
                        ))
                    })?;
                runtime.block_on(shutdown_and_remove_test_store(storage, data_dir))
            });
        let result = match cleanup {
            Ok(thread) => match thread.join() {
                Ok(result) => result,
                Err(_) => Err(StorageError::Database(
                    "embedded-test cleanup thread panicked".to_owned(),
                )),
            },
            Err(error) => Err(StorageError::Database(format!(
                "could not start embedded-test cleanup thread: {error}"
            ))),
        };
        if let Err(error) = result {
            eprintln!("HANDSHAKE_TEST_STORE_CLEANUP_FAILURE {error}");
        }
    }
}

impl EmbeddedTestBackend {
    /// Close the store and remove its directory.
    pub async fn close_and_remove(self) -> StorageResult<()> {
        let EmbeddedTestBackend {
            database,
            storage,
            data_dir: _,
            cleanup,
        } = self;
        drop(database);
        drop(storage);
        cleanup.cleanup().await
    }
}

fn combine_test_body_and_cleanup(
    body: StorageResult<()>,
    cleanup: StorageResult<()>,
) -> StorageResult<()> {
    match (body, cleanup) {
        (Ok(()), Ok(())) => Ok(()),
        (Err(error), Ok(())) | (Ok(()), Err(error)) => Err(error),
        (Err(body_error), Err(cleanup_error)) => Err(StorageError::Database(format!(
            "test body failed: {body_error}; cleanup also failed: {cleanup_error}"
        ))),
    }
}

async fn shutdown_and_remove_test_store(
    storage: SurrealStorage,
    data_dir: PathBuf,
) -> StorageResult<()> {
    let shutdown = storage.shutdown().await;
    drop(storage);
    let removal = std::fs::remove_dir_all(&data_dir);
    let mut failures = Vec::new();
    if let Err(error) = shutdown {
        failures.push(format!("shutdown failed: {error}"));
    }
    if let Err(error) = removal {
        if error.kind() != std::io::ErrorKind::NotFound {
            failures.push(format!(
                "cleanup failed for {}: {error}",
                data_dir.display()
            ));
        }
    }
    match data_dir.try_exists() {
        Ok(false) => {}
        Ok(true) => failures.push(format!(
            "embedded test store still exists after teardown: {}",
            data_dir.display()
        )),
        Err(error) => failures.push(format!(
            "could not verify embedded test store removal for {}: {error}",
            data_dir.display()
        )),
    }
    if failures.is_empty() {
        Ok(())
    } else {
        Err(StorageError::Database(failures.join("; ")))
    }
}

fn cleanup_unopened_test_store(data_dir: &PathBuf, error: impl std::fmt::Display) -> StorageError {
    match std::fs::remove_dir_all(data_dir) {
        Ok(()) => StorageError::Database(error.to_string()),
        Err(cleanup_error) => StorageError::Database(format!(
            "{error}; cleanup failed for {}: {cleanup_error}",
            data_dir.display()
        )),
    }
}

/// Open an isolated embedded store for one test.
///
/// There is no environment variable to point this at a server and no skip
/// condition: the store is created here, so a test either proves behaviour
/// against a real engine or fails. That is the whole reason the PostgreSQL
/// `POSTGRES_TEST_URL` / `DATABASE_URL` resolution chain is gone rather than
/// ported - there is nothing left to resolve.
pub async fn embedded_test_backend() -> StorageResult<EmbeddedTestBackend> {
    let data_dir = test_store_root()?.join(format!("store-{}", Uuid::now_v7().simple()));
    std::fs::create_dir_all(&data_dir).map_err(|error| {
        StorageError::Database(format!(
            "could not create embedded test store dir {}: {error}",
            data_dir.display()
        ))
    })?;

    let config = SurrealStorageConfig::for_data_dir(&data_dir)
        .map_err(|error| cleanup_unopened_test_store(&data_dir, error))?;
    let storage = SurrealStorage::open(config)
        .await
        .map_err(|error| cleanup_unopened_test_store(&data_dir, error))?;
    let database = SurrealDatabase::new(storage.clone());
    if let Err(error) = database.run_migrations().await {
        drop(database);
        let cleanup = shutdown_and_remove_test_store(storage, data_dir).await;
        return match cleanup {
            Ok(()) => Err(error),
            Err(cleanup_error) => Err(StorageError::Database(format!(
                "{error}; cleanup also failed: {cleanup_error}"
            ))),
        };
    }

    let cleanup = Arc::new(TestStoreCleanupGuard {
        storage: StdMutex::new(Some(storage.clone())),
        data_dir: data_dir.clone(),
    });
    Ok(EmbeddedTestBackend {
        database: Arc::new(database),
        storage,
        data_dir,
        cleanup,
    })
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

/// Runs the shared storage conformance suite against the provided backend.
#[allow(dead_code)]
pub async fn run_storage_conformance(db: Arc<dyn super::Database>) -> StorageResult<()> {
    db.ping().await?;

    let ctx = WriteContext::human(None);

    let workspace = db
        .create_workspace(
            &ctx,
            NewWorkspace {
                name: format!("ws-{}", Uuid::now_v7()),
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
                    id: Some(Uuid::now_v7().to_string()),
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
                    id: Some(Uuid::now_v7().to_string()),
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


    let node_a = Uuid::now_v7().to_string();
    let node_b = Uuid::now_v7().to_string();
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
                id: Some(Uuid::now_v7().to_string()),
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
            trace_id: Uuid::now_v7(),
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
        Some(Uuid::now_v7()),
        Some(Uuid::now_v7()),
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
                id: format!("local:{}", Uuid::now_v7()),
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

    let event_start = Utc.with_ymd_and_hms(2026, 3, 6, 9, 0, 0).unwrap();
    let event_end = event_start + Duration::hours(1);
    let event = db
        .upsert_calendar_event(
            &ctx,
            CalendarEventUpsert {
                id: Uuid::now_v7().to_string(),
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
                start_date: None,
                end_date_exclusive: None,
                was_floating: false,
                normalization_note: None,
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
            query_start_date: NaiveDate::from_ymd_opt(2026, 3, 6).unwrap(),
            query_end_date_exclusive: NaiveDate::from_ymd_opt(2026, 3, 7).unwrap(),
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
            query_start_date: NaiveDate::from_ymd_opt(2026, 3, 6).unwrap(),
            query_end_date_exclusive: NaiveDate::from_ymd_opt(2026, 3, 7).unwrap(),
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

#[cfg(test)]
async fn overwrite_loom_block_metrics(
    db: &Arc<dyn super::Database>,
    workspace_id: &str,
    block_id: &str,
    mention_count: i64,
    tag_count: i64,
    backlink_count: i64,
) -> StorageResult<()> {
    db.test_overwrite_loom_block_metrics(
        workspace_id,
        block_id,
        mention_count,
        tag_count,
        backlink_count,
    )
    .await
}

#[cfg(not(test))]
async fn overwrite_loom_block_metrics(
    db: &Arc<dyn super::Database>,
    workspace_id: &str,
    block_id: &str,
    mention_count: i64,
    tag_count: i64,
    backlink_count: i64,
) -> StorageResult<()> {
    let _ = (
        db,
        workspace_id,
        block_id,
        mention_count,
        tag_count,
        backlink_count,
    );
    Ok(())
}

#[cfg(test)]
async fn zero_workspace_loom_metrics(
    db: &Arc<dyn super::Database>,
    workspace_id: &str,
) -> StorageResult<()> {
    db.test_zero_workspace_loom_metrics(workspace_id).await
}

#[cfg(not(test))]
async fn zero_workspace_loom_metrics(
    db: &Arc<dyn super::Database>,
    workspace_id: &str,
) -> StorageResult<()> {
    let _ = (db, workspace_id);
    Ok(())
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

async fn loom_search_graph_filter_when_supported(
    db: &Arc<dyn super::Database>,
    workspace_id: &str,
    graph: &LoomGraphFixture,
) -> StorageResult<()> {
    if !db.supports_loom_graph_filtering() {
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

#[cfg(any(test, feature = "test-utils"))]
async fn insert_loom_traversal_perf_fixture(
    db: &Arc<dyn super::Database>,
    workspace_id: &str,
) -> StorageResult<String> {
    db.test_insert_loom_traversal_perf_fixture(workspace_id, LOOM_TRAVERSAL_PERF_TOTAL_BLOCKS)
        .await
}

#[cfg(not(any(test, feature = "test-utils")))]
async fn insert_loom_traversal_perf_fixture(
    db: &Arc<dyn super::Database>,
    workspace_id: &str,
) -> StorageResult<String> {
    let ctx = WriteContext::system(None);
    let start_block_id = "perf-block-00000".to_string();
    db.create_loom_block(
        &ctx,
        NewLoomBlock {
            block_id: Some(start_block_id.clone()),
            workspace_id: workspace_id.to_string(),
            content_type: LoomBlockContentType::Note,
            document_id: None,
            asset_id: None,
            title: Some("Perf Block 0".to_string()),
            original_filename: None,
            content_hash: None,
            pinned: false,
            journal_date: None,
            imported_at: None,
            derived: super::LoomBlockDerived {
                full_text_index: Some("perf traversal start".to_string()),
                ..Default::default()
            },
        },
    )
    .await?;
    let mut previous_block_id = start_block_id.clone();

    for idx in 1..LOOM_TRAVERSAL_PERF_TOTAL_BLOCKS {
        let block_id = format!("perf-block-{idx:05}");
        db.create_loom_block(
            &ctx,
            NewLoomBlock {
                block_id: Some(block_id.clone()),
                workspace_id: workspace_id.to_string(),
                content_type: LoomBlockContentType::Note,
                document_id: None,
                asset_id: None,
                title: Some(format!("Perf Block {idx}")),
                original_filename: None,
                content_hash: None,
                pinned: false,
                journal_date: None,
                imported_at: None,
                derived: super::LoomBlockDerived {
                    full_text_index: Some(format!("perf traversal block {idx}")),
                    ..Default::default()
                },
            },
        )
        .await?;
        db.create_loom_edge(
            &ctx,
            NewLoomEdge {
                edge_id: None,
                workspace_id: workspace_id.to_string(),
                source_block_id: previous_block_id,
                target_block_id: block_id.clone(),
                edge_type: LoomEdgeType::Mention,
                created_by: LoomEdgeCreatedBy::User,
                crdt_site_id: None,
                source_anchor: None,
            },
        )
        .await?;
        previous_block_id = block_id;
    }

    Ok(start_block_id)
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

    let limit_ms = db.loom_traverse_graph_perf_target_ms();
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
                name: format!("loom-ws-{}", Uuid::now_v7()),
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
                favorite: None,
                journal_date: Some("2026-03-14".into()),
                pin_order: None,
                expected_updated_at: None,
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
    loom_search_graph_filter_when_supported(&db, &workspace.id, &graph_fixture).await?;
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
                name: format!("loom-perf-ws-{}", Uuid::now_v7()),
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
                name: format!("calendar-ws-{}", Uuid::now_v7()),
            },
        )
        .await?;

    let source_id = format!("google:test:{}", Uuid::now_v7());
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
    assert_eq!(updated_source.last_actor_kind, "HUMAN");
    assert_eq!(
        updated_source.last_actor_id.as_deref(),
        Some("calendar-tester")
    );
    assert_eq!(updated_source.last_job_id, None);
    assert_eq!(updated_source.last_workflow_id, None);
    assert!(!updated_source.edit_event_id.is_empty());

    let listed_sources = db.list_calendar_sources(&workspace.id).await?;
    assert_eq!(listed_sources.len(), 1);
    let fetched_source = db
        .get_calendar_source(&workspace.id, &source.id)
        .await?
        .ok_or(StorageError::NotFound("calendar_source"))?;
    assert_eq!(fetched_source.display_name, "Google / Updated");

    let provider_start = Utc.with_ymd_and_hms(2026, 3, 7, 8, 0, 0).unwrap();
    let provider_end = provider_start + Duration::hours(1);
    let original_provider_event = db
        .upsert_calendar_event(
            &ctx,
            CalendarEventUpsert {
                id: Uuid::now_v7().to_string(),
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
                start_date: None,
                end_date_exclusive: None,
                was_floating: false,
                normalization_note: None,
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
                id: Uuid::now_v7().to_string(),
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
                start_date: None,
                end_date_exclusive: None,
                was_floating: false,
                normalization_note: None,
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
    assert_eq!(duplicate_provider_event.last_actor_kind, "HUMAN");
    assert_eq!(
        duplicate_provider_event.last_actor_id.as_deref(),
        Some("calendar-tester")
    );
    assert_eq!(duplicate_provider_event.last_job_id, None);
    assert!(!duplicate_provider_event.edit_event_id.is_empty());

    let local_start = provider_start + Duration::hours(5);
    let local_end = local_start + Duration::hours(2);
    let local_event = db
        .upsert_calendar_event(
            &ctx,
            CalendarEventUpsert {
                id: Uuid::now_v7().to_string(),
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
                start_date: None,
                end_date_exclusive: None,
                was_floating: true,
                normalization_note: None,
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
            query_start_date: NaiveDate::from_ymd_opt(2026, 3, 7).unwrap(),
            query_end_date_exclusive: NaiveDate::from_ymd_opt(2026, 3, 8).unwrap(),
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
            query_start_date: NaiveDate::from_ymd_opt(2026, 3, 7).unwrap(),
            query_end_date_exclusive: NaiveDate::from_ymd_opt(2026, 3, 8).unwrap(),
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
            query_start_date: NaiveDate::from_ymd_opt(2026, 3, 7).unwrap(),
            query_end_date_exclusive: NaiveDate::from_ymd_opt(2026, 3, 8).unwrap(),
            window_start_utc: provider_start - Duration::minutes(15),
            window_end_utc: local_end + Duration::minutes(15),
            source_ids: Vec::new(),
        })
        .await?;
    assert!(no_events.is_empty());

    // ── Workflow-backed / job-backed provenance round-trip ──────────
    let test_job_id = Uuid::now_v7();
    let test_workflow_id = Uuid::now_v7();
    let ai_ctx = WriteContext::ai(
        Some("ai-sync-agent".into()),
        Some(test_job_id),
        Some(test_workflow_id),
    );

    let ai_source_id = format!("google:ai-test:{}", Uuid::now_v7());
    let ai_source = db
        .upsert_calendar_source(
            &ai_ctx,
            CalendarSourceUpsert {
                id: ai_source_id.clone(),
                workspace_id: workspace.id.clone(),
                display_name: "Google / AI Sync".into(),
                provider_type: CalendarSourceProviderType::Google,
                write_policy: CalendarSourceWritePolicy::TwoWayMirror,
                default_tzid: "UTC".into(),
                auto_export: false,
                credentials_ref: Some("cred:ai".into()),
                provider_calendar_id: Some("ai-primary".into()),
                capability_profile_id: Some("calendar-google".into()),
                config: json!({"calendar_id": "ai-primary"}),
                sync_state: CalendarSourceSyncState {
                    state: None,
                    sync_token: Some("ai-sync-1".into()),
                    last_synced_at: Some(Utc::now()),
                    last_full_sync_at: None,
                    last_ok_at: None,
                    last_pull_at: None,
                    last_push_at: None,
                    last_error_at: None,
                    last_error_code: None,
                    last_error: None,
                    backoff_until: None,
                    consecutive_failures: Some(0),
                    last_remote_watermark: None,
                    last_local_applied_rev: None,
                },
            },
        )
        .await?;
    let job_str = test_job_id.to_string();
    let wf_str = test_workflow_id.to_string();
    assert_eq!(ai_source.last_actor_kind, "AI");
    assert_eq!(ai_source.last_actor_id.as_deref(), Some("ai-sync-agent"));
    assert_eq!(ai_source.last_job_id.as_deref(), Some(job_str.as_str()));
    assert_eq!(ai_source.last_workflow_id.as_deref(), Some(wf_str.as_str()));
    assert!(!ai_source.edit_event_id.is_empty());

    let fetched_ai_source = db
        .get_calendar_source(&workspace.id, &ai_source_id)
        .await?
        .ok_or(StorageError::NotFound("calendar_source"))?;
    assert_eq!(fetched_ai_source.last_actor_kind, "AI");
    assert_eq!(
        fetched_ai_source.last_actor_id.as_deref(),
        Some("ai-sync-agent")
    );
    assert_eq!(
        fetched_ai_source.last_job_id.as_deref(),
        Some(job_str.as_str())
    );
    assert_eq!(
        fetched_ai_source.last_workflow_id.as_deref(),
        Some(wf_str.as_str())
    );
    assert!(!fetched_ai_source.edit_event_id.is_empty());

    let listed_ai = db.list_calendar_sources(&workspace.id).await?;
    let listed_match = listed_ai
        .iter()
        .find(|s| s.id == ai_source_id)
        .expect("AI source in list");
    assert_eq!(listed_match.last_actor_kind, "AI");
    assert_eq!(listed_match.last_job_id.as_deref(), Some(job_str.as_str()));
    assert_eq!(
        listed_match.last_workflow_id.as_deref(),
        Some(wf_str.as_str())
    );

    let ai_event_start = Utc.with_ymd_and_hms(2026, 3, 17, 9, 0, 0).unwrap();
    let ai_event_end = ai_event_start + Duration::hours(1);
    let ai_event = db
        .upsert_calendar_event(
            &ai_ctx,
            CalendarEventUpsert {
                id: Uuid::now_v7().to_string(),
                workspace_id: workspace.id.clone(),
                source_id: ai_source_id.clone(),
                external_id: Some("ai-provider-event-1".into()),
                external_etag: Some("ai-etag-1".into()),
                title: "AI-synced meeting".into(),
                description: Some("synced by workflow".into()),
                location: None,
                start_ts_utc: ai_event_start,
                end_ts_utc: ai_event_end,
                start_local: Some("2026-03-17T09:00:00".into()),
                end_local: Some("2026-03-17T10:00:00".into()),
                tzid: "UTC".into(),
                all_day: false,
                start_date: None,
                end_date_exclusive: None,
                was_floating: false,
                normalization_note: None,
                status: CalendarEventStatus::Confirmed,
                visibility: CalendarEventVisibility::Public,
                export_mode: CalendarEventExportMode::FullExport,
                rrule: None,
                rdate: Vec::new(),
                exdate: Vec::new(),
                is_recurring: false,
                series_id: None,
                instance_key: None,
                is_override: false,
                source_last_seen_at: Some(Utc::now()),
                attendees: json!([]),
                links: json!([]),
                provider_payload: Some(json!({"source": "workflow"})),
            },
        )
        .await?;
    assert_eq!(ai_event.last_actor_kind, "AI");
    assert_eq!(ai_event.last_actor_id.as_deref(), Some("ai-sync-agent"));
    assert_eq!(ai_event.last_job_id.as_deref(), Some(job_str.as_str()));
    assert_eq!(ai_event.last_workflow_id.as_deref(), Some(wf_str.as_str()));
    assert!(!ai_event.edit_event_id.is_empty());

    let queried_ai_events = db
        .query_calendar_events(CalendarEventWindowQuery {
            workspace_id: workspace.id.clone(),
            query_start_date: NaiveDate::from_ymd_opt(2026, 3, 17).unwrap(),
            query_end_date_exclusive: NaiveDate::from_ymd_opt(2026, 3, 18).unwrap(),
            window_start_utc: ai_event_start - Duration::minutes(15),
            window_end_utc: ai_event_end + Duration::minutes(15),
            source_ids: vec![ai_source_id.clone()],
        })
        .await?;
    assert_eq!(queried_ai_events.len(), 1);
    assert_eq!(queried_ai_events[0].last_actor_kind, "AI");
    assert_eq!(
        queried_ai_events[0].last_actor_id.as_deref(),
        Some("ai-sync-agent")
    );
    assert_eq!(
        queried_ai_events[0].last_job_id.as_deref(),
        Some(job_str.as_str())
    );
    assert_eq!(
        queried_ai_events[0].last_workflow_id.as_deref(),
        Some(wf_str.as_str())
    );
    assert!(!queried_ai_events[0].edit_event_id.is_empty());

    db.delete_calendar_data_by_source(&ai_ctx, &workspace.id, &ai_source_id)
        .await?;

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
    let backend = embedded_test_backend().await?;
    let db = backend.database.clone();
    let body = async {
        let job = db
            .create_ai_job(NewAiJob {
                trace_id: Uuid::now_v7(),
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
        Ok::<(), StorageError>(())
    }
    .await;
    drop(db);
    let cleanup = backend.close_and_remove().await;
    combine_test_body_and_cleanup(body, cleanup)
}

#[tokio::test]
async fn workflow_node_execution_sets_finished_at_for_terminal_statuses() -> StorageResult<()> {
    let backend = embedded_test_backend().await?;
    let db = backend.database.clone();
    let body = async {
        let job = db
            .create_ai_job(NewAiJob {
                trace_id: Uuid::now_v7(),
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

        for (sequence, terminal_status) in
            [(1, JobState::CompletedWithIssues), (2, JobState::Poisoned)]
        {
            let exec = db
                .create_workflow_node_execution(NewNodeExecution {
                    workflow_run_id: run.id,
                    node_id: format!("node-{sequence}"),
                    node_type: "test".into(),
                    status: JobState::Running,
                    sequence,
                    input_payload: Some(json!({"input": true})),
                    started_at: Utc::now(),
                })
                .await?;

            let updated = db
                .update_workflow_node_execution_status(
                    exec.id,
                    terminal_status.clone(),
                    None,
                    Some("terminal".to_string()),
                )
                .await?;

            assert_eq!(updated.status, terminal_status);
            assert!(
                updated.finished_at.is_some(),
                "terminal workflow node status should set finished_at"
            );
        }

        Ok::<(), StorageError>(())
    }
    .await;
    drop(db);
    let cleanup = backend.close_and_remove().await;
    combine_test_body_and_cleanup(body, cleanup)
}

#[tokio::test]
async fn stalled_workflows_are_detected_by_heartbeat() -> StorageResult<()> {
    let backend = embedded_test_backend().await?;
    let db = backend.database.clone();
    let body = async {
        let job = db
            .create_ai_job(NewAiJob {
                trace_id: Uuid::now_v7(),
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
        Ok::<(), StorageError>(())
    }
    .await;
    drop(db);
    let cleanup = backend.close_and_remove().await;
    combine_test_body_and_cleanup(body, cleanup)
}

#[cfg(test)]
async fn assert_structured_collab_artifacts_supported() -> StorageResult<()> {
    // The PostgreSQL version of this helper hand-built an isolated SCHEMA with
    // raw SQL and dropped it afterwards. The embedded store isolates by data
    // DIRECTORY instead, so the setup and the teardown both disappear - what is
    // being asserted (the capability flag and the empty-state reads) is
    // unchanged, and it now runs against a real engine with no external
    // database and no skip path.
    let backend = embedded_test_backend().await?;
    let db = backend.database.clone();

    assert!(db.supports_structured_collab_artifacts());
    assert!(db
        .structured_collab_work_packet_row("WP-TEST")
        .await?
        .is_none());
    assert!(db.structured_collab_work_packet_rows().await?.is_empty());
    assert!(db
        .structured_collab_micro_task_status_rows("WP-TEST")
        .await?
        .is_empty());
    assert!(db
        .structured_collab_micro_task_rows("WP-TEST")
        .await?
        .is_empty());
    assert_eq!(
        db.structured_collab_micro_task_metadata("WP-TEST", "MT-TEST")
            .await?,
        None
    );
    assert!(!db.locus_work_packet_exists("WP-TEST").await?);

    drop(db);
    backend.close_and_remove().await?;
    Ok(())
}

#[cfg(test)]
fn mt136_sample_work_packet(wp_id: &str) -> locus::LocusCreateWpParams {
    locus::LocusCreateWpParams {
        wp_id: wp_id.to_owned(),
        title: format!("MT-136 real-store proof for {wp_id}"),
        description: "Exercise the embedded SurrealDB Locus storage surface".to_owned(),
        priority: 1,
        kind: locus::WorkPacketType::Test,
        phase: locus::WorkPacketPhase::Phase1,
        routing: locus::RoutingPolicy::GovStandard,
        task_packet_path: Some(format!(".GOV/task_packets/{wp_id}/packet.json")),
        assignee: Some("MT-136".to_owned()),
        labels: Some(vec!["surreal".to_owned(), "durability".to_owned()]),
        spec_session_id: Some("mt136-real-store".to_owned()),
        reporter: "storage-conformance".to_owned(),
    }
}

#[cfg(test)]
fn mt136_sample_micro_task(wp_id: &str, mt_id: &str) -> locus::TrackedMicroTask {
    locus::TrackedMicroTask {
        schema_id: String::new(),
        schema_version: String::new(),
        record_id: String::new(),
        record_kind: String::new(),
        project_profile_kind: locus::ProjectProfileKind::SoftwareDelivery,
        updated_at: Utc::now(),
        mirror_state: locus::MirrorSyncState::CanonicalOnly,
        authority_refs: vec![format!("authority:{wp_id}")],
        evidence_refs: Vec::new(),
        summary_record_path: None,
        profile_extension: None,
        mt_id: mt_id.to_owned(),
        wp_id: wp_id.to_owned(),
        name: "MT-136 embedded lifecycle".to_owned(),
        scope: "Prove every ported Locus operation against a real store".to_owned(),
        files: locus::MicroTaskFiles {
            read: Vec::new(),
            modify: vec!["src/backend/handshake_core/src/storage/".to_owned()],
            create: Vec::new(),
        },
        done_criteria: vec!["close/reopen retains lifecycle state".to_owned()],
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
        metadata: json!({"proof": "mt136-real-store"}),
    }
}

#[derive(surrealdb::types::SurrealValue)]
struct Mt136InvalidWorkPacketStateBindings {
    record: surrealdb::types::RecordId,
    invalid_state: String,
}

#[tokio::test]
async fn locus_and_structured_collaboration_roundtrip_real_store_and_reopen() -> StorageResult<()> {
    let backend = embedded_test_backend().await?;
    let database = backend.database.clone();
    let storage = backend.storage.clone();
    let data_dir = backend.data_dir.clone();
    let first_wp = "WP-MT136-A";
    let second_wp = "WP-MT136-B";
    let mt_id = "MT-MT136-1";

    let missing_task_board_update = database
        .locus_task_board_update_work_packet(
            "READY",
            "READY",
            &Utc::now().to_rfc3339(),
            r#"{"proof":"missing-row"}"#,
            "WP-MT136-MISSING",
        )
        .await;
    assert!(matches!(
        missing_task_board_update,
        Err(StorageError::NotFound("work_packet"))
    ));

    database
        .execute_locus_operation(locus::LocusOperation::CreateWp(mt136_sample_work_packet(
            first_wp,
        )))
        .await?;
    database
        .execute_locus_operation(locus::LocusOperation::CreateWp(mt136_sample_work_packet(
            second_wp,
        )))
        .await?;
    let duplicate = database
        .execute_locus_operation(locus::LocusOperation::CreateWp(mt136_sample_work_packet(
            first_wp,
        )))
        .await;
    assert!(matches!(duplicate, Err(StorageError::Conflict(_))));

    for update in [
        BTreeMap::from([("status".to_owned(), json!("invalid-state"))]),
        BTreeMap::from([("task_board_status".to_owned(), json!("INVALID_STATE"))]),
    ] {
        let invalid = database
            .execute_locus_operation(locus::LocusOperation::UpdateWp(
                locus::LocusUpdateWpParams {
                    wp_id: first_wp.to_owned(),
                    updates: update,
                    source: Some("mt136-invalid-state-proof".to_owned()),
                },
            ))
            .await;
        assert!(matches!(invalid, Err(StorageError::Validation(_))));
    }
    let invalid_task_board_state = database
        .locus_task_board_update_work_packet(
            "ready",
            "INVALID_STATE",
            &Utc::now().to_rfc3339(),
            r#"{"proof":"invalid-task-board-state"}"#,
            first_wp,
        )
        .await;
    assert!(matches!(
        invalid_task_board_state,
        Err(StorageError::Validation(_))
    ));

    for statement in [
        "UPDATE $record SET status = $invalid_state RETURN AFTER;",
        "UPDATE $record SET task_board_status = $invalid_state RETURN AFTER;",
    ] {
        let bindings = Mt136InvalidWorkPacketStateBindings {
            record: surrealdb::types::RecordId::new("work_packets", first_wp.to_owned()),
            invalid_state: "invalid-state".to_owned(),
        };
        let direct_schema_write = storage
            .with_data_operation(move |surreal| {
                Box::pin(async move {
                    surreal
                        .query_values_at::<surrealdb::types::Value, _>(statement, bindings, 0)
                        .await
                })
            })
            .await;
        assert!(
            direct_schema_write.is_err(),
            "work-packet state domains must fail closed in the embedded schema"
        );
    }
    let unchanged = database
        .execute_locus_operation(locus::LocusOperation::GetWpStatus(
            locus::LocusGetWpStatusParams {
                wp_id: first_wp.to_owned(),
            },
        ))
        .await?;
    assert_eq!(unchanged["status"], "stub");
    assert_eq!(unchanged["task_board_status"], "STUB");

    for wp_id in [first_wp, second_wp] {
        database
            .execute_locus_operation(locus::LocusOperation::UpdateWp(
                locus::LocusUpdateWpParams {
                    wp_id: wp_id.to_owned(),
                    updates: BTreeMap::from([
                        ("status".to_owned(), json!("ready")),
                        ("task_board_status".to_owned(), json!("READY")),
                    ]),
                    source: Some("mt136-real-store".to_owned()),
                },
            ))
            .await?;
    }
    database
        .execute_locus_operation(locus::LocusOperation::GateWp(locus::LocusGateWpParams {
            wp_id: first_wp.to_owned(),
            gate: locus::LocusGateKind::PreWork,
            result: locus::GateStatus {
                status: locus::GateStatusKind::Pass,
                validated_at: Some(Utc::now()),
                validated_by: Some("mt136-real-store".to_owned()),
                notes: Some("real embedded gate proof".to_owned()),
                validation_report_ref: Some(json!({"proof": "surreal"})),
            },
        }))
        .await?;

    let registered_task = mt136_sample_micro_task(first_wp, mt_id);
    database
        .execute_locus_operation(locus::LocusOperation::RegisterMts(
            locus::LocusRegisterMtsParams {
                wp_id: first_wp.to_owned(),
                micro_tasks: vec![registered_task.clone()],
            },
        ))
        .await?;
    database
        .execute_locus_operation(locus::LocusOperation::RegisterMts(
            locus::LocusRegisterMtsParams {
                wp_id: first_wp.to_owned(),
                micro_tasks: vec![registered_task.clone()],
            },
        ))
        .await?;
    let mut divergent_task = registered_task;
    divergent_task.name = "divergent retry must fail closed".to_owned();
    let divergent_retry = database
        .execute_locus_operation(locus::LocusOperation::RegisterMts(
            locus::LocusRegisterMtsParams {
                wp_id: first_wp.to_owned(),
                micro_tasks: vec![divergent_task],
            },
        ))
        .await;
    assert!(matches!(divergent_retry, Err(StorageError::Conflict(_))));
    assert_eq!(
        database
            .structured_collab_work_packet_row(first_wp)
            .await?
            .expect("created work packet")
            .wp_id,
        first_wp
    );
    assert_eq!(
        database.structured_collab_work_packet_rows().await?.len(),
        2
    );
    assert_eq!(
        database
            .structured_collab_micro_task_status_rows(first_wp)
            .await?
            .len(),
        1
    );
    assert_eq!(
        database
            .structured_collab_micro_task_rows(first_wp)
            .await?
            .len(),
        1
    );
    assert!(database
        .structured_collab_micro_task_metadata(first_wp, mt_id)
        .await?
        .expect("registered micro-task metadata")
        .contains("mt136-real-store"));

    database
        .execute_locus_operation(locus::LocusOperation::StartMt(locus::LocusStartMtParams {
            wp_id: first_wp.to_owned(),
            mt_id: mt_id.to_owned(),
            model_id: "gpt-mt136".to_owned(),
            lora_id: None,
            escalation_level: 0,
        }))
        .await?;
    database
        .execute_locus_operation(locus::LocusOperation::BindSession(
            locus::LocusBindSessionParams {
                wp_id: first_wp.to_owned(),
                mt_id: mt_id.to_owned(),
                session_id: "session-mt136".to_owned(),
                model_id: Some("gpt-mt136".to_owned()),
                lora_id: None,
                escalation_level: 0,
            },
        ))
        .await?;
    database
        .execute_locus_operation(locus::LocusOperation::UnbindSession(
            locus::LocusUnbindSessionParams {
                wp_id: first_wp.to_owned(),
                mt_id: mt_id.to_owned(),
                session_id: "session-mt136".to_owned(),
                reason: Some("proof complete".to_owned()),
            },
        ))
        .await?;

    let iteration = locus::MicroTaskIterationRecord {
        iteration: 3,
        model_id: "gpt-mt136".to_owned(),
        lora_id: None,
        escalation_level: 0,
        started_at: Utc::now(),
        completed_at: Utc::now(),
        duration_ms: 12,
        tokens_prompt: 10,
        tokens_completion: 5,
        claimed_complete: true,
        validation_passed: Some(true),
        outcome: locus::MicroTaskIterationOutcome::Success,
        output_artifact_ref: json!({"artifact": "mt136"}),
        validation_artifact_ref: Some(json!({"validation": "real-store"})),
        error_summary: None,
        failure_category: None,
    };
    database
        .execute_locus_operation(locus::LocusOperation::RecordIteration(
            locus::LocusRecordIterationParams {
                wp_id: first_wp.to_owned(),
                mt_id: mt_id.to_owned(),
                iteration: iteration.clone(),
            },
        ))
        .await?;
    let mut escalated_iteration = iteration;
    escalated_iteration.escalation_level = 1;
    escalated_iteration.outcome = locus::MicroTaskIterationOutcome::Retry;
    escalated_iteration.validation_passed = Some(false);
    database
        .execute_locus_operation(locus::LocusOperation::RecordIteration(
            locus::LocusRecordIterationParams {
                wp_id: first_wp.to_owned(),
                mt_id: mt_id.to_owned(),
                iteration: escalated_iteration,
            },
        ))
        .await?;
    let progress = database
        .execute_locus_operation(locus::LocusOperation::GetMtProgress(
            locus::LocusGetMtProgressParams {
                mt_id: mt_id.to_owned(),
            },
        ))
        .await?;
    assert_eq!(progress["current_iteration"], 3);
    assert_eq!(progress["escalation_level"], 1);
    database
        .execute_locus_operation(locus::LocusOperation::CompleteMt(
            locus::LocusCompleteMtParams {
                wp_id: first_wp.to_owned(),
                mt_id: mt_id.to_owned(),
                final_iteration: 3,
            },
        ))
        .await?;

    database
        .execute_locus_operation(locus::LocusOperation::AddDependency(
            locus::LocusAddDependencyParams {
                dependency_id: "DEP-MT136-1".to_owned(),
                from_wp_id: second_wp.to_owned(),
                to_wp_id: first_wp.to_owned(),
                kind: locus::DependencyType::Blocks,
            },
        ))
        .await?;
    let cycle = database
        .execute_locus_operation(locus::LocusOperation::AddDependency(
            locus::LocusAddDependencyParams {
                dependency_id: "DEP-MT136-CYCLE".to_owned(),
                from_wp_id: first_wp.to_owned(),
                to_wp_id: second_wp.to_owned(),
                kind: locus::DependencyType::Blocks,
            },
        ))
        .await;
    assert!(matches!(cycle, Err(StorageError::Validation(_))));
    let ready_while_blocked = database
        .execute_locus_operation(locus::LocusOperation::QueryReady(
            locus::LocusQueryReadyParams { limit: Some(10) },
        ))
        .await?;
    assert_eq!(ready_while_blocked["wp_ids"], json!([second_wp]));
    database
        .execute_locus_operation(locus::LocusOperation::RemoveDependency(
            locus::LocusRemoveDependencyParams {
                dependency_id: "DEP-MT136-1".to_owned(),
            },
        ))
        .await?;
    database
        .execute_locus_operation(locus::LocusOperation::AddDependency(
            locus::LocusAddDependencyParams {
                dependency_id: "DEP-MT136-DEPENDS-ON".to_owned(),
                from_wp_id: second_wp.to_owned(),
                to_wp_id: first_wp.to_owned(),
                kind: locus::DependencyType::DependsOn,
            },
        ))
        .await?;
    let ready_while_dependency_unfinished = database
        .execute_locus_operation(locus::LocusOperation::QueryReady(
            locus::LocusQueryReadyParams { limit: Some(10) },
        ))
        .await?;
    assert_eq!(
        ready_while_dependency_unfinished["wp_ids"],
        json!([first_wp])
    );
    database
        .execute_locus_operation(locus::LocusOperation::RemoveDependency(
            locus::LocusRemoveDependencyParams {
                dependency_id: "DEP-MT136-DEPENDS-ON".to_owned(),
            },
        ))
        .await?;
    let ready = database
        .execute_locus_operation(locus::LocusOperation::QueryReady(
            locus::LocusQueryReadyParams { limit: Some(10) },
        ))
        .await?;
    assert_eq!(ready["wp_ids"].as_array().map(Vec::len), Some(2));

    let (_first_wp_status, _first_wp_metadata) = database
        .locus_task_board_get_status_and_metadata(first_wp)
        .await?
        .expect("created work packet should exist before task-board update");
    database
        .locus_task_board_update_work_packet(
            "READY",
            "READY",
            &Utc::now().to_rfc3339(),
            r#"{"proof":"task-board-update"}"#,
            first_wp,
        )
        .await?;
    let status = database
        .execute_locus_operation(locus::LocusOperation::GetWpStatus(
            locus::LocusGetWpStatusParams {
                wp_id: first_wp.to_owned(),
            },
        ))
        .await?;
    assert_eq!(status["status"], "ready");
    let snapshot = database
        .execute_locus_operation(locus::LocusOperation::SyncTaskBoard(
            locus::LocusSyncTaskBoardParams {
                dry_run: Some(true),
            },
        ))
        .await?;
    assert_eq!(snapshot["authority_rows"].as_array().map(Vec::len), Some(2));

    database
        .execute_locus_operation(locus::LocusOperation::CloseWp(locus::LocusCloseWpParams {
            wp_id: first_wp.to_owned(),
        }))
        .await?;
    database
        .execute_locus_operation(locus::LocusOperation::DeleteWp(
            locus::LocusDeleteWpParams {
                wp_id: second_wp.to_owned(),
            },
        ))
        .await?;

    drop(database);
    storage
        .shutdown()
        .await
        .map_err(|error| StorageError::Database(error.to_string()))?;
    drop(storage);

    let config = SurrealStorageConfig::for_data_dir(&data_dir)
        .map_err(|error| StorageError::Database(error.to_string()))?;
    let reopened_storage = SurrealStorage::open(config)
        .await
        .map_err(|error| StorageError::Database(error.to_string()))?;
    let reopened = SurrealDatabase::new(reopened_storage.clone());
    reopened.run_migrations().await?;
    let first_status = Database::execute_locus_operation(
        &reopened,
        locus::LocusOperation::GetWpStatus(locus::LocusGetWpStatusParams {
            wp_id: first_wp.to_owned(),
        }),
    )
    .await?;
    let second_status = Database::execute_locus_operation(
        &reopened,
        locus::LocusOperation::GetWpStatus(locus::LocusGetWpStatusParams {
            wp_id: second_wp.to_owned(),
        }),
    )
    .await?;
    assert_eq!(first_status["status"], "done");
    assert_eq!(second_status["status"], "cancelled");
    let durable_progress = Database::execute_locus_operation(
        &reopened,
        locus::LocusOperation::GetMtProgress(locus::LocusGetMtProgressParams {
            mt_id: mt_id.to_owned(),
        }),
    )
    .await?;
    assert_eq!(durable_progress["status"], "completed");
    assert_eq!(
        Database::structured_collab_work_packet_rows(&reopened)
            .await?
            .len(),
        2
    );
    assert_eq!(
        Database::structured_collab_micro_task_rows(&reopened, first_wp)
            .await?
            .len(),
        1
    );

    drop(reopened);
    let reopened_shutdown = reopened_storage
        .shutdown()
        .await
        .map_err(|error| StorageError::Database(error.to_string()));
    drop(reopened_storage);
    let cleanup = backend.close_and_remove().await;
    match (reopened_shutdown, cleanup) {
        (Ok(()), Ok(())) => Ok(()),
        (Err(error), Ok(())) | (Ok(()), Err(error)) => Err(error),
        (Err(reopen_error), Err(cleanup_error)) => Err(StorageError::Database(format!(
            "reopened store shutdown failed: {reopen_error}; cleanup also failed: {cleanup_error}"
        ))),
    }
}

#[tokio::test]
async fn database_trait_purity() -> StorageResult<()> {
    assert_structured_collab_artifacts_supported().await?;
    Ok(())
}

#[tokio::test]
async fn locus_backend_capability() -> StorageResult<()> {
    assert_structured_collab_artifacts_supported().await
}

#[tokio::test]
async fn structured_collab_artifacts_are_supported() -> StorageResult<()> {
    assert_structured_collab_artifacts_supported().await
}

#[tokio::test]
async fn loom_search_graph_filter_backend_support() -> StorageResult<()> {
    let backend = embedded_test_backend().await?;
    let db = backend.database.clone();
    let body = async {
        let ctx = WriteContext::human(Some("loom-search-proof".into()));
        let workspace = db
            .create_workspace(
                &ctx,
                NewWorkspace {
                    name: format!("loom-search-proof-{}", Uuid::now_v7()),
                },
            )
            .await?;
        let document = db
            .create_document(
                &ctx,
                NewDocument {
                    workspace_id: workspace.id.clone(),
                    title: format!("loom-search-proof-doc-{}", Uuid::now_v7()),
                },
            )
            .await?;
        let graph_fixture =
            build_loom_graph_fixture(&db, &ctx, &workspace.id, &document.id).await?;

        loom_search_graph_filter_when_supported(&db, &workspace.id, &graph_fixture).await
    }
    .await;
    drop(db);
    let cleanup = backend.close_and_remove().await;
    combine_test_body_and_cleanup(body, cleanup)
}

#[test]
fn database_trait_purity_source_regressions() {
    let storage_mod = include_str!("mod.rs").replace("\r\n", "\n");
    let workflows_prod = include_str!("../workflows.rs")
        .split("#[cfg(test)]")
        .next()
        .unwrap_or_default();
    let loom_api_prod = include_str!("../api/loom.rs")
        .split("#[cfg(test)]")
        .next()
        .unwrap_or_default();
    let retention_prod = include_str!("retention.rs");
    let surreal_storage = include_str!("surreal/database.rs");
    let database_trait_start = storage_mod
        .find("pub trait Database: Send + Sync {")
        .expect("Database trait should exist in storage/mod.rs");
    let database_trait_end = storage_mod[database_trait_start..]
        .find("\n\nimpl<T> StorageCapabilityStore for T")
        .expect("Database trait terminator should precede StorageCapabilityStore impl");
    let database_trait =
        &storage_mod[database_trait_start..database_trait_start + database_trait_end];

    assert!(storage_mod.contains("pub trait StructuredCollaborationStore"));
    assert!(storage_mod.contains("pub trait StorageCapabilityStore"));
    for required in [
        "fn supports_locus_runtime(",
        "fn supports_structured_collab_artifacts(",
        "fn loom_search_observability_tier(",
        "fn supports_loom_graph_filtering(",
        "fn loom_traverse_graph_perf_target_ms(",
        "async fn execute_locus_operation(",
        "async fn locus_task_board_update_work_packet(",
        "async fn structured_collab_work_packet_row(",
        "async fn structured_collab_work_packet_rows(",
        "async fn structured_collab_micro_task_metadata(",
        "async fn structured_collab_micro_task_status_rows(",
        "async fn structured_collab_micro_task_rows(",
    ] {
        assert!(
            database_trait.contains(required),
            "Database trait should retain the current backend compatibility baseline: {required}"
        );
    }
    for forbidden in [
        "fn storage_capabilities(",
        "StorageCapabilitySnapshot",
        "StorageBackendKind",
        "async fn structured_collab_list_work_packet_ids(",
        "async fn structured_collab_load_work_packet_row(",
        "async fn structured_collab_list_micro_task_status_rows(",
        "async fn structured_collab_load_micro_task_metadata(",
        "async fn structured_collab_list_micro_task_metadata(",
        "async fn structured_collab_list_task_board_projection_rows(",
        "StructuredCollabTaskBoardProjectionRow",
    ] {
        assert!(
            !database_trait.contains(forbidden),
            "Database trait must not accrete boundary-only helper surface: {forbidden}"
        );
    }
    assert!(!database_trait.contains("std::any::Any"));
    assert!(!database_trait.contains("fn as_any("));
    assert!(workflows_prod.contains("StructuredCollaborationStore"));
    assert!(workflows_prod.contains("StorageCapabilityStore"));
    assert!(loom_api_prod.contains(".storage_capabilities()"));
    assert!(!workflows_prod.contains("crate::storage::locus_sqlite::"));
    assert!(!workflows_prod.contains("downcast_ref::<crate::storage::sqlite::SqliteDatabase>()"));
    assert!(!workflows_prod.contains(".as_any()"));
    assert!(!loom_api_prod.contains(".as_any()"));
    assert!(!loom_api_prod.contains("state.storage.loom_search_observability_tier()"));
    assert!(!retention_prod.contains(".as_any()"));
    assert!(retention_prod.contains(".test_update_ai_job_metadata("));
    for backend_src in [surreal_storage] {
        assert!(backend_src.contains("fn supports_locus_runtime(&self) -> bool {"));
        assert!(backend_src.contains("fn supports_structured_collab_artifacts(&self) -> bool {"));
        assert!(backend_src.contains("fn loom_search_observability_tier(&self) -> u8 {"));
        assert!(backend_src.contains("fn supports_loom_graph_filtering(&self) -> bool {"));
        assert!(backend_src.contains("fn loom_traverse_graph_perf_target_ms(&self) -> u128 {"));
        assert!(backend_src.contains("async fn test_update_ai_job_metadata("));
        assert!(backend_src.contains("async fn test_fetch_mutation_traceability_row("));
        assert!(!backend_src.contains("fn as_any("));
    }
}

#[test]
fn storage_mode_defaults_to_surreal_embedded_when_unset() -> StorageResult<()> {
    // SurrealDB is the only accepted authority, so an ABSENT mode resolves to it
    // rather than failing. There is no DATABASE_URL to supply any more: the
    // store location comes from HANDSHAKE_DATA_DIR, and leaving that unset is
    // also valid because the platform-local application data directory is the
    // documented fallback.
    let config = ControlPlaneStorageConfig::resolve(None, None)?;
    assert_eq!(config.mode, ControlPlaneStorageMode::SurrealEmbedded);
    assert_eq!(config.mode.as_str(), "surreal_embedded");
    assert_eq!(config.data_dir, None);

    let scoped = ControlPlaneStorageConfig::resolve(Some("surreal_embedded"), Some("/tmp/hsk"))?;
    assert_eq!(scoped.mode, ControlPlaneStorageMode::SurrealEmbedded);
    assert!(scoped.data_dir.is_some());

    Ok(())
}

#[test]
fn storage_mode_fails_closed_on_a_stale_postgres_mode() {
    // A leftover HANDSHAKE_STORAGE_MODE=postgres_primary in someone's
    // environment must FAIL rather than be silently ignored. Ignoring it would
    // start Handshake on the embedded store while the operator believed they had
    // selected PostgreSQL - the config would be lying about which database holds
    // their data, which is worse than refusing to start.
    for stale in ["postgres_primary", "postgres", "sqlite"] {
        let err = ControlPlaneStorageConfig::resolve(Some(stale), None).unwrap_err();
        match err {
            StorageError::Validation(message) => {
                assert_eq!(message, "unsupported storage mode");
            }
            other => panic!("expected validation error for {stale}, got {other:?}"),
        }
    }
}

#[test]
fn unsupported_storage_modes_fail_closed() {
    let err = ControlPlaneStorageConfig::resolve(Some("legacy_cache"), None).unwrap_err();
    assert!(matches!(
        err,
        StorageError::Validation("unsupported storage mode")
    ));
}

#[tokio::test]
async fn database_trait_purity_capability_snapshot_reports_surreal() -> StorageResult<()> {
    assert!(ControlPlaneStorageMode::SurrealEmbedded.is_control_plane_authority());
    assert_eq!(
        ControlPlaneStorageMode::SurrealEmbedded.authority_label(),
        "primary_authority"
    );
    assert_eq!(
        ControlPlaneStorageMode::SurrealEmbedded.freshness_label(),
        "current_source_of_truth"
    );

    let backend = embedded_test_backend().await?;
    let db = backend.database.clone();
    let caps = db.storage_capabilities();

    assert_eq!(caps.backend, StorageBackendKind::Surreal);
    assert!(caps.supports_structured_collab_artifacts);
    assert!(caps.supports_loom_graph_filtering);
    assert_eq!(caps.loom_search_observability_tier(), 2);
    drop(db);
    backend.close_and_remove().await
}
