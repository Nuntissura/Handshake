use surrealdb::types::{Datetime, RecordId, RecordIdKey, SurrealValue};

use super::SurrealStorage;
#[cfg(any(test, feature = "surreal-test-support"))]
use crate::storage::MutationTraceabilityRow;
use crate::storage::{AiJobMcpFields, AiJobMcpUpdate, StorageError, StorageResult};

const AI_JOB_MCP_FIELDS: &str = "ai_job_mcp_fields";

#[derive(SurrealValue)]
struct McpUpdateBindings {
    job: RecordId,
    mcp_record: RecordId,
    mcp_server_id: Option<String>,
    mcp_call_id: Option<String>,
    mcp_progress_token: Option<String>,
    now: Datetime,
}

#[derive(SurrealValue)]
struct McpFieldsRow {
    mcp_server_id: Option<String>,
    mcp_call_id: Option<String>,
    mcp_progress_token: Option<String>,
}

#[derive(SurrealValue)]
struct McpReadBindings {
    job: RecordId,
    mcp_record: RecordId,
}

#[derive(SurrealValue)]
struct TokenBinding {
    progress_token: String,
}

#[derive(SurrealValue)]
struct JobReferenceRow {
    job_id: RecordId,
}

#[cfg(any(test, feature = "surreal-test-support"))]
#[derive(SurrealValue)]
struct TestMetadataBindings {
    job: RecordId,
    status: String,
    created_at: Datetime,
    is_pinned: bool,
}

#[cfg(any(test, feature = "surreal-test-support"))]
#[derive(SurrealValue)]
struct RecordBinding {
    record: RecordId,
}

#[cfg(any(test, feature = "surreal-test-support"))]
#[derive(SurrealValue)]
struct MutationTraceabilityRowValue {
    last_actor_kind: String,
    last_actor_id: Option<String>,
    last_job_id: Option<String>,
    last_workflow_id: Option<String>,
    edit_event_id: String,
}

pub(crate) async fn update_ai_job_mcp_fields(
    storage: &SurrealStorage,
    job_id: uuid::Uuid,
    update: AiJobMcpUpdate,
) -> StorageResult<()> {
    let mcp_progress_token = update.mcp_progress_token.clone();
    let bindings = McpUpdateBindings {
        job: RecordId::new("ai_jobs", job_id.to_string()),
        mcp_record: RecordId::new(AI_JOB_MCP_FIELDS, job_id.to_string()),
        mcp_server_id: update.mcp_server_id,
        mcp_call_id: update.mcp_call_id,
        mcp_progress_token: update.mcp_progress_token,
        now: Datetime::from(chrono::Utc::now()),
    };
    let result = storage
        .with_data_operation(move |database| {
            Box::pin(async move {
                database
                    .query_values_at::<surrealdb::types::Value, _>(
                        "BEGIN TRANSACTION; \
                         IF !record::exists($job) { THROW 'HSK-AI-JOB-MISSING'; }; \
                         UPSERT $mcp_record SET job_id = $job, \
                           mcp_server_id = $mcp_server_id ?? mcp_server_id, \
                           mcp_call_id = $mcp_call_id ?? mcp_call_id, \
                           mcp_progress_token = $mcp_progress_token ?? mcp_progress_token \
                           RETURN AFTER; \
                         UPDATE $job SET updated_at = $now; \
                         COMMIT TRANSACTION;",
                        bindings,
                        2,
                    )
                    .await
            })
        })
        .await;

    match result {
        Ok(rows) if rows.len() == 1 => Ok(()),
        Ok(_) => Err(StorageError::Database(
            "AI job MCP upsert returned no row".to_owned(),
        )),
        Err(error) => {
            let message = error.to_string();
            if message.contains("HSK-AI-JOB-MISSING") || message.contains("record::exists") {
                return Err(StorageError::NotFound("ai_job"));
            }
            if let Some(progress_token) = mcp_progress_token {
                if matches!(
                    find_ai_job_id_by_mcp_progress_token(storage, &progress_token).await?,
                    Some(existing_job_id) if existing_job_id != job_id
                ) {
                    return Err(StorageError::Conflict("mcp_progress_token already mapped"));
                }
                // Preserve the conflict classification even if the unique
                // winner is deleted between the failed write and the re-read.
                if message.contains("idx_ai_job_mcp_fields_progress_token")
                    || (message.contains("already contains")
                        && message.contains("mcp_progress_token"))
                {
                    return Err(StorageError::Conflict("mcp_progress_token already mapped"));
                }
            }
            Err(StorageError::from(error))
        }
    }
}

pub(crate) async fn get_ai_job_mcp_fields(
    storage: &SurrealStorage,
    job_id: uuid::Uuid,
) -> StorageResult<AiJobMcpFields> {
    // Preserve the existing contract atomically: an existing job without an
    // MCP sidecar yields defaults, while a missing job is an explicit NotFound.
    let result = storage
        .with_data_operation(move |database| {
            Box::pin(async move {
                database
                    .query_values_at::<McpFieldsRow, _>(
                        "BEGIN TRANSACTION; \
                         IF !record::exists($job) { THROW 'HSK-AI-JOB-MISSING'; }; \
                         SELECT mcp_server_id, mcp_call_id, mcp_progress_token \
                           FROM $mcp_record; \
                         COMMIT TRANSACTION;",
                        McpReadBindings {
                            job: RecordId::new("ai_jobs", job_id.to_string()),
                            mcp_record: RecordId::new(AI_JOB_MCP_FIELDS, job_id.to_string()),
                        },
                        2,
                    )
                    .await
            })
        })
        .await;
    let rows = match result {
        Ok(rows) => rows,
        Err(error) if error.to_string().contains("HSK-AI-JOB-MISSING") => {
            return Err(StorageError::NotFound("ai_job"));
        }
        Err(error) => return Err(StorageError::from(error)),
    };
    let row = rows.into_iter().next();
    Ok(row
        .map(|row| AiJobMcpFields {
            mcp_server_id: row.mcp_server_id,
            mcp_call_id: row.mcp_call_id,
            mcp_progress_token: row.mcp_progress_token,
        })
        .unwrap_or_default())
}

pub(crate) async fn find_ai_job_id_by_mcp_progress_token(
    storage: &SurrealStorage,
    progress_token: &str,
) -> StorageResult<Option<uuid::Uuid>> {
    let row: Option<JobReferenceRow> = storage
        .with_data_operation({
            let progress_token = progress_token.to_owned();
            move |database| {
                Box::pin(async move {
                    database
                        .query_first(
                            "SELECT job_id FROM ai_job_mcp_fields \
                             WHERE mcp_progress_token = $progress_token LIMIT 1;",
                            TokenBinding { progress_token },
                        )
                        .await
                })
            }
        })
        .await
        .map_err(StorageError::from)?;
    row.map(|row| match row.job_id.key {
        RecordIdKey::String(value) => uuid::Uuid::parse_str(&value)
            .map_err(|_| StorageError::Validation("invalid job_id uuid")),
        _ => Err(StorageError::Validation("invalid job_id uuid")),
    })
    .transpose()
}

#[cfg(any(test, feature = "surreal-test-support"))]
pub(crate) async fn test_update_ai_job_metadata(
    storage: &SurrealStorage,
    job_id: uuid::Uuid,
    status: &str,
    created_at: chrono::DateTime<chrono::Utc>,
    is_pinned: bool,
) -> StorageResult<()> {
    let count = storage
        .with_data_operation({
            let status = status.to_owned();
            move |database| {
                Box::pin(async move {
                    database
                        .execute_returning(
                            "UPDATE $job SET status = $status, created_at = $created_at, \
                             is_pinned = $is_pinned RETURN AFTER;",
                            TestMetadataBindings {
                                job: RecordId::new("ai_jobs", job_id.to_string()),
                                status,
                                created_at: Datetime::from(created_at),
                                is_pinned,
                            },
                        )
                        .await
                })
            }
        })
        .await
        .map_err(StorageError::from)?;
    if count == 1 {
        Ok(())
    } else {
        Err(StorageError::NotFound("ai_job"))
    }
}

#[cfg(any(test, feature = "surreal-test-support"))]
pub(crate) async fn test_fetch_mutation_traceability_row(
    storage: &SurrealStorage,
    table: &str,
    id: &str,
) -> StorageResult<MutationTraceabilityRow> {
    let record = RecordId::new(table, id);
    let row: Option<MutationTraceabilityRowValue> = storage
        .with_data_operation(move |database| {
            Box::pin(async move {
                database
                    .query_first(
                        "SELECT last_actor_kind, last_actor_id, last_job_id, \
                         last_workflow_id, edit_event_id FROM $record;",
                        RecordBinding { record },
                    )
                    .await
            })
        })
        .await
        .map_err(StorageError::from)?;
    row.map(|row| MutationTraceabilityRow {
        last_actor_kind: row.last_actor_kind,
        last_actor_id: row.last_actor_id,
        last_job_id: row.last_job_id,
        last_workflow_id: row.last_workflow_id,
        edit_event_id: row.edit_event_id,
    })
    .ok_or(StorageError::NotFound("mutation_traceability_row"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::surreal::{bootstrap_schema, SurrealDatabase, SurrealStorageConfig};
    use crate::storage::{
        AccessMode, Database, JobKind, JobMetrics, NewAiJob, NewWorkspace, SafetyMode, WriteContext,
    };

    async fn open(path: &std::path::Path) -> SurrealStorage {
        let storage = SurrealStorage::open(
            SurrealStorageConfig::with_path(path).expect("valid embedded MCP test path"),
        )
        .await
        .expect("open embedded MCP store");
        bootstrap_schema(&storage)
            .await
            .expect("bootstrap embedded MCP schema");
        storage
    }

    async fn create_job(database: &SurrealDatabase) -> uuid::Uuid {
        database
            .create_ai_job(NewAiJob {
                trace_id: uuid::Uuid::now_v7(),
                job_kind: JobKind::MicroTaskExecution,
                protocol_id: "micro_task_executor_v1".to_owned(),
                profile_id: "micro_task_executor_v1".to_owned(),
                capability_profile_id: "Coder".to_owned(),
                access_mode: AccessMode::AnalysisOnly,
                safety_mode: SafetyMode::Normal,
                entity_refs: Vec::new(),
                planned_operations: Vec::new(),
                status_reason: "queued".to_owned(),
                metrics: JobMetrics::zero(),
                job_inputs: None,
            })
            .await
            .expect("create MCP test job")
            .job_id
    }

    async fn delete_job(storage: &SurrealStorage, job_id: uuid::Uuid) {
        storage
            .with_data_operation(move |database| {
                Box::pin(async move {
                    database
                        .query_values::<surrealdb::types::Value, _>(
                            "DELETE $record RETURN BEFORE;",
                            RecordBinding {
                                record: RecordId::new("ai_jobs", job_id.to_string()),
                            },
                        )
                        .await
                })
            })
            .await
            .expect("delete MCP race job");
    }

    #[tokio::test]
    async fn mcp_fields_preserve_partial_updates_reject_token_reuse_and_survive_reopen() {
        let directory = tempfile::tempdir().expect("temporary MT-136 MCP root");
        let path = directory.path().join("store");
        let storage = open(&path).await;
        let database = SurrealDatabase::new(storage.clone());
        let job_id = create_job(&database).await;
        let other_job_id = create_job(&database).await;

        assert_eq!(
            get_ai_job_mcp_fields(&storage, job_id)
                .await
                .expect("read empty MCP fields")
                .mcp_server_id,
            None
        );
        update_ai_job_mcp_fields(
            &storage,
            job_id,
            AiJobMcpUpdate {
                mcp_server_id: Some("server-a".to_owned()),
                mcp_call_id: Some("call-a".to_owned()),
                mcp_progress_token: Some("progress-a".to_owned()),
            },
        )
        .await
        .expect("persist MCP fields");
        update_ai_job_mcp_fields(
            &storage,
            job_id,
            AiJobMcpUpdate {
                mcp_call_id: Some("call-b".to_owned()),
                ..AiJobMcpUpdate::default()
            },
        )
        .await
        .expect("merge partial MCP fields");
        let fields = get_ai_job_mcp_fields(&storage, job_id)
            .await
            .expect("read merged MCP fields");
        assert_eq!(fields.mcp_server_id.as_deref(), Some("server-a"));
        assert_eq!(fields.mcp_call_id.as_deref(), Some("call-b"));
        assert_eq!(fields.mcp_progress_token.as_deref(), Some("progress-a"));
        assert_eq!(
            find_ai_job_id_by_mcp_progress_token(&storage, "progress-a")
                .await
                .expect("find MCP token"),
            Some(job_id)
        );
        assert!(matches!(
            update_ai_job_mcp_fields(
                &storage,
                other_job_id,
                AiJobMcpUpdate {
                    mcp_progress_token: Some("progress-a".to_owned()),
                    ..AiJobMcpUpdate::default()
                },
            )
            .await,
            Err(StorageError::Conflict(_))
        ));
        assert!(matches!(
            update_ai_job_mcp_fields(&storage, uuid::Uuid::now_v7(), AiJobMcpUpdate::default(),)
                .await,
            Err(StorageError::NotFound("ai_job"))
        ));

        drop(database);
        storage.shutdown().await.expect("close embedded MCP store");
        drop(storage);

        let reopened = open(&path).await;
        let persisted = get_ai_job_mcp_fields(&reopened, job_id)
            .await
            .expect("read reopened MCP fields");
        assert_eq!(persisted.mcp_server_id.as_deref(), Some("server-a"));
        assert_eq!(persisted.mcp_call_id.as_deref(), Some("call-b"));
        assert_eq!(
            find_ai_job_id_by_mcp_progress_token(&reopened, "progress-a")
                .await
                .expect("find reopened MCP token"),
            Some(job_id)
        );
        reopened.shutdown().await.expect("close reopened MCP store");
    }

    #[tokio::test]
    async fn test_helpers_mutate_and_read_the_real_embedded_store() {
        let directory = tempfile::tempdir().expect("temporary MT-136 helper root");
        let storage = open(&directory.path().join("store")).await;
        let database = SurrealDatabase::new(storage.clone());
        let job_id = create_job(&database).await;
        let created_at = chrono::Utc::now() - chrono::Duration::days(30);
        test_update_ai_job_metadata(&storage, job_id, "completed", created_at, true)
            .await
            .expect("update real AI job metadata");
        let job = database
            .get_ai_job(&job_id.to_string())
            .await
            .expect("read updated AI job");
        assert_eq!(job.state.as_str(), "completed");
        assert_eq!(job.created_at, created_at);

        let workspace = database
            .create_workspace(
                &WriteContext::human(Some("mt-136-helper".to_owned())),
                NewWorkspace {
                    name: "MT-136 helper proof".to_owned(),
                },
            )
            .await
            .expect("create traceable workspace");
        let trace = test_fetch_mutation_traceability_row(&storage, "workspaces", &workspace.id)
            .await
            .expect("read mutation traceability");
        assert_eq!(trace.last_actor_kind, "HUMAN");
        assert_eq!(trace.last_actor_id.as_deref(), Some("mt-136-helper"));
        assert!(!trace.edit_event_id.is_empty());
        storage.shutdown().await.expect("close helper store");
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn mcp_reads_and_unique_token_writes_are_race_safe() {
        let directory = tempfile::tempdir().expect("temporary MT-136 MCP race root");
        let storage = open(&directory.path().join("store")).await;
        let database = SurrealDatabase::new(storage.clone());

        for attempt in 0..16 {
            let job_id = create_job(&database).await;
            update_ai_job_mcp_fields(
                &storage,
                job_id,
                AiJobMcpUpdate {
                    mcp_server_id: Some(format!("server-{attempt}")),
                    mcp_call_id: Some(format!("call-{attempt}")),
                    mcp_progress_token: None,
                },
            )
            .await
            .expect("seed non-default MCP sidecar without token");

            let barrier = std::sync::Arc::new(tokio::sync::Barrier::new(3));
            let read_storage = storage.clone();
            let read_barrier = barrier.clone();
            let reader = tokio::spawn(async move {
                read_barrier.wait().await;
                get_ai_job_mcp_fields(&read_storage, job_id).await
            });
            let delete_storage = storage.clone();
            let delete_barrier = barrier.clone();
            let deleter = tokio::spawn(async move {
                delete_barrier.wait().await;
                delete_job(&delete_storage, job_id).await;
            });
            barrier.wait().await;
            let read_result = reader.await.expect("MCP race reader joins");
            deleter.await.expect("MCP race deleter joins");
            match read_result {
                Ok(fields) => {
                    assert_eq!(fields.mcp_server_id, Some(format!("server-{attempt}")));
                    assert_eq!(fields.mcp_call_id, Some(format!("call-{attempt}")));
                    assert_eq!(fields.mcp_progress_token, None);
                }
                Err(StorageError::NotFound("ai_job")) => {}
                other => panic!("unexpected MCP read/delete race result: {other:?}"),
            }
        }

        let first_job = create_job(&database).await;
        let second_job = create_job(&database).await;
        let barrier = std::sync::Arc::new(tokio::sync::Barrier::new(3));
        let first_storage = storage.clone();
        let first_barrier = barrier.clone();
        let first = tokio::spawn(async move {
            first_barrier.wait().await;
            update_ai_job_mcp_fields(
                &first_storage,
                first_job,
                AiJobMcpUpdate {
                    mcp_progress_token: Some("simultaneous-token".to_owned()),
                    ..AiJobMcpUpdate::default()
                },
            )
            .await
        });
        let second_storage = storage.clone();
        let second_barrier = barrier.clone();
        let second = tokio::spawn(async move {
            second_barrier.wait().await;
            update_ai_job_mcp_fields(
                &second_storage,
                second_job,
                AiJobMcpUpdate {
                    mcp_progress_token: Some("simultaneous-token".to_owned()),
                    ..AiJobMcpUpdate::default()
                },
            )
            .await
        });
        barrier.wait().await;
        let first = first.await.expect("first MCP writer joins");
        let second = second.await.expect("second MCP writer joins");
        assert_eq!(first.is_ok() as usize + second.is_ok() as usize, 1);
        assert_eq!(
            matches!(first, Err(StorageError::Conflict(_))) as usize
                + matches!(second, Err(StorageError::Conflict(_))) as usize,
            1
        );

        storage.shutdown().await.expect("close MCP race store");
    }
}
