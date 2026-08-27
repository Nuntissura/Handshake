//! Read-side storage for Locus structured-collaboration artifacts.
//!
//! `work_packets` and `micro_tasks` are global governance records rather than
//! workspace-scoped content. These projections deliberately preserve the
//! ordering and optional-column behavior of the retired PostgreSQL queries.

use surrealdb::types::{RecordId, SurrealValue};

use super::SurrealStorage;
use crate::storage::{StorageError, StorageResult, StructuredCollabWorkPacketRow};

const WORK_PACKETS: &str = "work_packets";

#[derive(SurrealValue)]
struct WorkPacketIdBinding {
    wp_id: String,
}

#[derive(SurrealValue)]
struct WorkPacketRecordBinding {
    work_packet: RecordId,
}

#[derive(SurrealValue)]
struct MicroTaskBinding {
    work_packet: RecordId,
    mt_id: String,
}

#[derive(SurrealValue)]
struct EmptyBindings {}

#[derive(SurrealValue)]
struct WorkPacketRecord {
    wp_id: String,
    version: i64,
    title: String,
    description: Option<String>,
    status: String,
    priority: i64,
    phase: Option<String>,
    routing: Option<String>,
    task_packet_path: Option<String>,
    task_board_status: String,
    assignee: Option<String>,
    reporter: String,
    created_at: String,
    updated_at: String,
    vector_clock: String,
    metadata: String,
}

#[derive(SurrealValue)]
struct MicroTaskMetadataRecord {
    metadata: String,
}

#[derive(SurrealValue)]
struct MicroTaskStatusRecord {
    mt_id: String,
    status: String,
}

#[derive(SurrealValue)]
struct MicroTaskRecord {
    mt_id: String,
    metadata: String,
}

impl From<WorkPacketRecord> for StructuredCollabWorkPacketRow {
    fn from(row: WorkPacketRecord) -> Self {
        Self {
            wp_id: row.wp_id,
            version: row.version,
            title: row.title,
            description: row.description.unwrap_or_default(),
            status: row.status,
            priority: row.priority,
            phase: row.phase.unwrap_or_default(),
            routing: row.routing.unwrap_or_default(),
            task_packet_path: row.task_packet_path,
            task_board_status: row.task_board_status,
            assignee: row.assignee,
            reporter: row.reporter,
            created_at: row.created_at,
            updated_at: row.updated_at,
            vector_clock: row.vector_clock,
            metadata: row.metadata,
        }
    }
}

pub(crate) async fn work_packet_row(
    storage: &SurrealStorage,
    wp_id: &str,
) -> StorageResult<Option<StructuredCollabWorkPacketRow>> {
    let row: Option<WorkPacketRecord> = storage
        .with_data_operation({
            let bindings = WorkPacketIdBinding {
                wp_id: wp_id.to_owned(),
            };
            move |database| {
                Box::pin(async move {
                    database
                        .query_first(
                            "SELECT wp_id, version, title, description, status, priority, phase, routing, \
                             task_packet_path, task_board_status, assignee, reporter, created_at, updated_at, \
                             vector_clock, metadata FROM work_packets WHERE wp_id = $wp_id LIMIT 1;",
                            bindings,
                        )
                        .await
                })
            }
        })
        .await
        .map_err(StorageError::from)?;
    Ok(row.map(Into::into))
}

pub(crate) async fn work_packet_rows(
    storage: &SurrealStorage,
) -> StorageResult<Vec<StructuredCollabWorkPacketRow>> {
    let rows: Vec<WorkPacketRecord> = storage
        .with_data_operation(|database| {
            Box::pin(async move {
                database
                    .query_values(
                        "SELECT wp_id, version, title, description, status, priority, phase, routing, \
                         task_packet_path, task_board_status, assignee, reporter, created_at, updated_at, \
                         vector_clock, metadata FROM work_packets ORDER BY updated_at ASC, wp_id ASC;",
                        EmptyBindings {},
                    )
                    .await
            })
        })
        .await
        .map_err(StorageError::from)?;
    Ok(rows.into_iter().map(Into::into).collect())
}

pub(crate) async fn micro_task_metadata(
    storage: &SurrealStorage,
    wp_id: &str,
    mt_id: &str,
) -> StorageResult<Option<String>> {
    let row: Option<MicroTaskMetadataRecord> = storage
        .with_data_operation({
            let bindings = MicroTaskBinding {
                work_packet: RecordId::new(WORK_PACKETS, wp_id.to_owned()),
                mt_id: mt_id.to_owned(),
            };
            move |database| {
                Box::pin(async move {
                    database
                        .query_first(
                            "SELECT metadata FROM micro_tasks \
                             WHERE wp_id = $work_packet AND mt_id = $mt_id LIMIT 1;",
                            bindings,
                        )
                        .await
                })
            }
        })
        .await
        .map_err(StorageError::from)?;
    Ok(row.map(|row| row.metadata))
}

pub(crate) async fn micro_task_status_rows(
    storage: &SurrealStorage,
    wp_id: &str,
) -> StorageResult<Vec<(String, String)>> {
    let rows: Vec<MicroTaskStatusRecord> = storage
        .with_data_operation({
            let bindings = WorkPacketRecordBinding {
                work_packet: RecordId::new(WORK_PACKETS, wp_id.to_owned()),
            };
            move |database| {
                Box::pin(async move {
                    database
                        .query_values(
                            "SELECT mt_id, status FROM micro_tasks \
                             WHERE wp_id = $work_packet ORDER BY mt_id ASC;",
                            bindings,
                        )
                        .await
                })
            }
        })
        .await
        .map_err(StorageError::from)?;
    Ok(rows
        .into_iter()
        .map(|row| (row.mt_id, row.status))
        .collect())
}

pub(crate) async fn micro_task_rows(
    storage: &SurrealStorage,
    wp_id: &str,
) -> StorageResult<Vec<(String, String)>> {
    let rows: Vec<MicroTaskRecord> = storage
        .with_data_operation({
            let bindings = WorkPacketRecordBinding {
                work_packet: RecordId::new(WORK_PACKETS, wp_id.to_owned()),
            };
            move |database| {
                Box::pin(async move {
                    database
                        .query_values(
                            "SELECT mt_id, metadata FROM micro_tasks \
                             WHERE wp_id = $work_packet ORDER BY mt_id ASC;",
                            bindings,
                        )
                        .await
                })
            }
        })
        .await
        .map_err(StorageError::from)?;
    Ok(rows
        .into_iter()
        .map(|row| (row.mt_id, row.metadata))
        .collect())
}
