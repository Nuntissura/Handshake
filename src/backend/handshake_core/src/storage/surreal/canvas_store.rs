use chrono::{DateTime, Utc};
use serde_json::{Map, Value};
use surrealdb::types::{Datetime, RecordId, RecordIdKey, SurrealValue};
use uuid::Uuid;

use super::SurrealStorage;
use crate::storage::{
    Canvas, CanvasEdge, CanvasGraph, CanvasNode, NewCanvas, NewCanvasEdge, NewCanvasNode,
    StorageError, StorageResult, WriteContext,
};

const CANVASES: &str = "canvases";
const CANVAS_NODES: &str = "canvas_nodes";
const CANVAS_EDGES: &str = "canvas_edges";
const WORKSPACES: &str = "workspaces";

#[derive(SurrealValue)]
struct CanvasRow {
    id: RecordId,
    workspace_id: RecordId,
    title: String,
    created_at: Datetime,
    updated_at: Datetime,
}

#[derive(SurrealValue)]
struct CanvasNodeRow {
    id: RecordId,
    canvas_id: RecordId,
    kind: String,
    position_x: f64,
    position_y: f64,
    data: Value,
    created_at: Datetime,
    updated_at: Datetime,
}

#[derive(SurrealValue)]
struct CanvasEdgeRow {
    id: RecordId,
    canvas_id: RecordId,
    from_node_id: RecordId,
    to_node_id: RecordId,
    kind: String,
    created_at: Datetime,
    updated_at: Datetime,
}

#[derive(SurrealValue)]
struct CanvasWriteBindings {
    canvas: RecordId,
    workspace: RecordId,
    title: String,
    actor_kind: String,
    actor_id: Option<String>,
    job_id: Option<String>,
    workflow_id: Option<String>,
    edit_event_id: String,
    now: Datetime,
}

#[derive(SurrealValue)]
struct CanvasLookupBindings {
    canvas: RecordId,
}

#[derive(SurrealValue)]
struct WorkspaceLookupBindings {
    workspace: RecordId,
}

#[derive(SurrealValue)]
struct RenameBindings {
    canvas: RecordId,
    title: String,
    expected_updated_at: Option<Datetime>,
    actor_kind: String,
    actor_id: Option<String>,
    job_id: Option<String>,
    workflow_id: Option<String>,
    edit_event_id: String,
    now: Datetime,
}

#[derive(SurrealValue)]
struct NodeWrite {
    record: RecordId,
    kind: String,
    position_x: f64,
    position_y: f64,
    data: Value,
    actor_kind: String,
    actor_id: Option<String>,
    job_id: Option<String>,
    workflow_id: Option<String>,
    edit_event_id: String,
}

#[derive(SurrealValue)]
struct EdgeWrite {
    record: RecordId,
    from_node: RecordId,
    to_node: RecordId,
    kind: String,
    actor_kind: String,
    actor_id: Option<String>,
    job_id: Option<String>,
    workflow_id: Option<String>,
    edit_event_id: String,
}

#[derive(SurrealValue)]
struct GraphWriteBindings {
    canvas: RecordId,
    nodes: Vec<NodeWrite>,
    edges: Vec<EdgeWrite>,
    actor_kind: String,
    actor_id: Option<String>,
    job_id: Option<String>,
    workflow_id: Option<String>,
    edit_event_id: String,
    now: Datetime,
}

pub(crate) async fn create(
    storage: &SurrealStorage,
    ctx: &WriteContext,
    canvas: NewCanvas,
) -> StorageResult<Canvas> {
    if canvas.workspace_id.trim().is_empty() {
        return Err(StorageError::Validation("canvas_workspace_id_empty"));
    }
    let title = canvas.title.trim();
    if title.is_empty() {
        return Err(StorageError::Validation("canvas_title_empty"));
    }
    let id = Uuid::now_v7().to_string();
    let metadata = storage
        .inner
        .guard
        .validate_write(ctx, &id)
        .await
        .map_err(StorageError::from)?;
    let bindings = CanvasWriteBindings {
        canvas: RecordId::new(CANVASES, id),
        workspace: RecordId::new(WORKSPACES, canvas.workspace_id),
        title: title.to_owned(),
        actor_kind: metadata.actor_kind.as_str().to_owned(),
        actor_id: metadata.actor_id,
        job_id: metadata.job_id.map(|value| value.to_string()),
        workflow_id: metadata.workflow_id.map(|value| value.to_string()),
        edit_event_id: metadata.edit_event_id.to_string(),
        now: Datetime::from(metadata.timestamp),
    };
    let row: Option<CanvasRow> = storage
        .with_data_operation(move |database| {
            Box::pin(async move {
                database
                    .query_first(
                        "CREATE $canvas CONTENT { workspace_id: $workspace, title: $title, \
                         last_actor_kind: $actor_kind, last_actor_id: $actor_id, last_job_id: $job_id, \
                         last_workflow_id: $workflow_id, edit_event_id: $edit_event_id, \
                         created_at: $now, updated_at: $now } RETURN AFTER;",
                        bindings,
                    )
                    .await
            })
        })
        .await
        .map_err(StorageError::from)?;
    row.map(map_canvas)
        .transpose()?
        .ok_or_else(|| StorageError::Database("canvas create returned no row".to_owned()))
}

pub(crate) async fn list(
    storage: &SurrealStorage,
    workspace_id: &str,
) -> StorageResult<Vec<Canvas>> {
    let bindings = WorkspaceLookupBindings {
        workspace: RecordId::new(WORKSPACES, workspace_id.to_owned()),
    };
    let rows: Vec<CanvasRow> = storage
        .with_data_operation(move |database| {
            Box::pin(async move {
                database
                    .query_values(
                        "SELECT id, workspace_id, title, created_at, updated_at FROM canvases \
                         WHERE workspace_id = $workspace ORDER BY created_at ASC, id ASC;",
                        bindings,
                    )
                    .await
            })
        })
        .await
        .map_err(StorageError::from)?;
    rows.into_iter().map(map_canvas).collect()
}

pub(crate) async fn get_graph(
    storage: &SurrealStorage,
    canvas_id: &str,
) -> StorageResult<CanvasGraph> {
    let canvas = get_canvas(storage, canvas_id)
        .await?
        .ok_or(StorageError::NotFound("canvas"))?;
    let bindings = CanvasLookupBindings {
        canvas: RecordId::new(CANVASES, canvas_id.to_owned()),
    };
    let nodes: Vec<CanvasNodeRow> = storage
        .with_data_operation(move |database| {
            Box::pin(async move {
                database
                    .query_values(
                        "SELECT id, canvas_id, kind, position_x, position_y, data, created_at, updated_at \
                         FROM canvas_nodes WHERE canvas_id = $canvas ORDER BY created_at ASC, id ASC;",
                        bindings,
                    )
                    .await
            })
        })
        .await
        .map_err(StorageError::from)?;
    let bindings = CanvasLookupBindings {
        canvas: RecordId::new(CANVASES, canvas_id.to_owned()),
    };
    let edges: Vec<CanvasEdgeRow> = storage
        .with_data_operation(move |database| {
            Box::pin(async move {
                database
                    .query_values(
                        "SELECT id, canvas_id, from_node_id, to_node_id, kind, created_at, updated_at \
                         FROM canvas_edges WHERE canvas_id = $canvas ORDER BY created_at ASC, id ASC;",
                        bindings,
                    )
                    .await
            })
        })
        .await
        .map_err(StorageError::from)?;
    Ok(CanvasGraph {
        canvas,
        nodes: nodes
            .into_iter()
            .map(map_node)
            .collect::<StorageResult<_>>()?,
        edges: edges
            .into_iter()
            .map(map_edge)
            .collect::<StorageResult<_>>()?,
    })
}

pub(crate) async fn rename(
    storage: &SurrealStorage,
    ctx: &WriteContext,
    canvas_id: &str,
    title: &str,
    expected_updated_at: Option<DateTime<Utc>>,
) -> StorageResult<Canvas> {
    let title = title.trim();
    if title.is_empty() {
        return Err(StorageError::Validation("canvas_title_empty"));
    }
    let metadata = storage
        .inner
        .guard
        .validate_write(ctx, canvas_id)
        .await
        .map_err(StorageError::from)?;
    let guarded = expected_updated_at.is_some();
    let bindings = RenameBindings {
        canvas: RecordId::new(CANVASES, canvas_id.to_owned()),
        title: title.to_owned(),
        expected_updated_at: expected_updated_at.map(Datetime::from),
        actor_kind: metadata.actor_kind.as_str().to_owned(),
        actor_id: metadata.actor_id,
        job_id: metadata.job_id.map(|value| value.to_string()),
        workflow_id: metadata.workflow_id.map(|value| value.to_string()),
        edit_event_id: metadata.edit_event_id.to_string(),
        now: Datetime::from(metadata.timestamp),
    };
    let row: Option<CanvasRow> = storage
        .with_data_operation(move |database| {
            Box::pin(async move {
                database
                    .query_first(
                        "UPDATE $canvas SET title = $title, updated_at = $now, \
                         last_actor_kind = $actor_kind, last_actor_id = $actor_id, last_job_id = $job_id, \
                         last_workflow_id = $workflow_id, edit_event_id = $edit_event_id \
                         WHERE $expected_updated_at = NONE OR updated_at = $expected_updated_at RETURN AFTER;",
                        bindings,
                    )
                    .await
            })
        })
        .await
        .map_err(StorageError::from)?;
    if let Some(row) = row {
        return map_canvas(row);
    }
    if get_canvas(storage, canvas_id).await?.is_some() && guarded {
        Err(StorageError::Conflict("canvas_updated_at_conflict"))
    } else {
        Err(StorageError::NotFound("canvas"))
    }
}

pub(crate) async fn update_graph(
    storage: &SurrealStorage,
    ctx: &WriteContext,
    canvas_id: &str,
    nodes: Vec<NewCanvasNode>,
    edges: Vec<NewCanvasEdge>,
) -> StorageResult<CanvasGraph> {
    let canvas_metadata = storage
        .inner
        .guard
        .validate_write(ctx, canvas_id)
        .await
        .map_err(StorageError::from)?;

    let mut node_writes = Vec::with_capacity(nodes.len());
    for node in nodes {
        let id = node.id.unwrap_or_else(|| Uuid::now_v7().to_string());
        let metadata = storage
            .inner
            .guard
            .validate_write(ctx, &id)
            .await
            .map_err(StorageError::from)?;
        let data = node.data.unwrap_or_else(|| Value::Object(Map::new()));
        if !data.is_object() {
            return Err(StorageError::Validation("canvas_node_data_must_be_object"));
        }
        node_writes.push(NodeWrite {
            record: RecordId::new(CANVAS_NODES, id),
            kind: node.kind,
            position_x: node.position_x,
            position_y: node.position_y,
            data,
            actor_kind: metadata.actor_kind.as_str().to_owned(),
            actor_id: metadata.actor_id,
            job_id: metadata.job_id.map(|value| value.to_string()),
            workflow_id: metadata.workflow_id.map(|value| value.to_string()),
            edit_event_id: metadata.edit_event_id.to_string(),
        });
    }
    let node_ids: std::collections::HashSet<String> = node_writes
        .iter()
        .map(|node| key_ref(&node.record).map(str::to_owned))
        .collect::<StorageResult<_>>()?;
    let mut edge_writes = Vec::with_capacity(edges.len());
    for edge in edges {
        if !node_ids.contains(&edge.from_node_id) || !node_ids.contains(&edge.to_node_id) {
            return Err(StorageError::Validation(
                "canvas_edge_endpoint_not_in_graph",
            ));
        }
        let id = edge.id.unwrap_or_else(|| Uuid::now_v7().to_string());
        let metadata = storage
            .inner
            .guard
            .validate_write(ctx, &id)
            .await
            .map_err(StorageError::from)?;
        edge_writes.push(EdgeWrite {
            record: RecordId::new(CANVAS_EDGES, id),
            from_node: RecordId::new(CANVAS_NODES, edge.from_node_id),
            to_node: RecordId::new(CANVAS_NODES, edge.to_node_id),
            kind: edge.kind,
            actor_kind: metadata.actor_kind.as_str().to_owned(),
            actor_id: metadata.actor_id,
            job_id: metadata.job_id.map(|value| value.to_string()),
            workflow_id: metadata.workflow_id.map(|value| value.to_string()),
            edit_event_id: metadata.edit_event_id.to_string(),
        });
    }
    let bindings = GraphWriteBindings {
        canvas: RecordId::new(CANVASES, canvas_id.to_owned()),
        nodes: node_writes,
        edges: edge_writes,
        actor_kind: canvas_metadata.actor_kind.as_str().to_owned(),
        actor_id: canvas_metadata.actor_id,
        job_id: canvas_metadata.job_id.map(|value| value.to_string()),
        workflow_id: canvas_metadata.workflow_id.map(|value| value.to_string()),
        edit_event_id: canvas_metadata.edit_event_id.to_string(),
        now: Datetime::from(canvas_metadata.timestamp),
    };
    storage
        .with_data_operation(move |database| {
            Box::pin(async move {
                database
                    .query_values_at::<surrealdb::types::Value, _>(
                        "BEGIN TRANSACTION; \
                         IF (SELECT VALUE id FROM $canvas LIMIT 1)[0] = NONE { THROW 'HSK-CANVAS-NOT-FOUND'; }; \
                         DELETE canvas_edges WHERE canvas_id = $canvas; \
                         DELETE canvas_nodes WHERE canvas_id = $canvas; \
                         FOR $node IN $nodes { CREATE $node.record CONTENT { canvas_id: $canvas, kind: $node.kind, \
                             position_x: $node.position_x, position_y: $node.position_y, data: $node.data, \
                             last_actor_kind: $node.actor_kind, last_actor_id: $node.actor_id, \
                             last_job_id: $node.job_id, last_workflow_id: $node.workflow_id, \
                             edit_event_id: $node.edit_event_id, created_at: $now, updated_at: $now }; }; \
                         FOR $edge IN $edges { CREATE $edge.record CONTENT { canvas_id: $canvas, \
                             from_node_id: $edge.from_node, to_node_id: $edge.to_node, kind: $edge.kind, \
                             last_actor_kind: $edge.actor_kind, last_actor_id: $edge.actor_id, \
                             last_job_id: $edge.job_id, last_workflow_id: $edge.workflow_id, \
                             edit_event_id: $edge.edit_event_id, created_at: $now, updated_at: $now }; }; \
                         UPDATE $canvas SET last_actor_kind = $actor_kind, last_actor_id = $actor_id, \
                             last_job_id = $job_id, last_workflow_id = $workflow_id, \
                             edit_event_id = $edit_event_id, updated_at = $now RETURN AFTER; \
                         COMMIT TRANSACTION;",
                        bindings,
                        6,
                    )
                    .await
            })
        })
        .await
        .map_err(|error| {
            if error.to_string().contains("HSK-CANVAS-NOT-FOUND") {
                StorageError::NotFound("canvas")
            } else {
                StorageError::from(error)
            }
        })?;
    get_graph(storage, canvas_id).await
}

pub(crate) async fn delete(
    storage: &SurrealStorage,
    ctx: &WriteContext,
    canvas_id: &str,
) -> StorageResult<()> {
    storage
        .inner
        .guard
        .validate_write(ctx, canvas_id)
        .await
        .map_err(StorageError::from)?;
    let deleted: Option<CanvasRow> = storage
        .with_data_operation({
            let id = canvas_id.to_owned();
            move |database| Box::pin(async move { database.delete_one(CANVASES, &id).await })
        })
        .await
        .map_err(StorageError::from)?;
    deleted.map(|_| ()).ok_or(StorageError::NotFound("canvas"))
}

async fn get_canvas(storage: &SurrealStorage, canvas_id: &str) -> StorageResult<Option<Canvas>> {
    let row: Option<CanvasRow> = storage
        .with_data_operation({
            let id = canvas_id.to_owned();
            move |database| Box::pin(async move { database.select_one(CANVASES, &id).await })
        })
        .await
        .map_err(StorageError::from)?;
    row.map(map_canvas).transpose()
}

fn key(record: RecordId) -> StorageResult<String> {
    match record.key {
        RecordIdKey::String(value) => Ok(value),
        _ => Err(StorageError::Database(
            "embedded canvas record has a non-string id".to_owned(),
        )),
    }
}

fn key_ref(record: &RecordId) -> StorageResult<&str> {
    match &record.key {
        RecordIdKey::String(value) => Ok(value),
        _ => Err(StorageError::Database(
            "embedded canvas record has a non-string id".to_owned(),
        )),
    }
}

fn map_canvas(row: CanvasRow) -> StorageResult<Canvas> {
    Ok(Canvas {
        id: key(row.id)?,
        workspace_id: key(row.workspace_id)?,
        title: row.title,
        created_at: row.created_at.into_inner(),
        updated_at: row.updated_at.into_inner(),
    })
}

fn map_node(row: CanvasNodeRow) -> StorageResult<CanvasNode> {
    Ok(CanvasNode {
        id: key(row.id)?,
        canvas_id: key(row.canvas_id)?,
        kind: row.kind,
        position_x: row.position_x,
        position_y: row.position_y,
        data: row.data,
        created_at: row.created_at.into_inner(),
        updated_at: row.updated_at.into_inner(),
    })
}

fn map_edge(row: CanvasEdgeRow) -> StorageResult<CanvasEdge> {
    Ok(CanvasEdge {
        id: key(row.id)?,
        canvas_id: key(row.canvas_id)?,
        from_node_id: key(row.from_node_id)?,
        to_node_id: key(row.to_node_id)?,
        kind: row.kind,
        created_at: row.created_at.into_inner(),
        updated_at: row.updated_at.into_inner(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::{surreal::schema, NewWorkspace};

    async fn open(path: &std::path::Path) -> SurrealStorage {
        let storage = SurrealStorage::open(
            super::super::SurrealStorageConfig::with_path(path).expect("valid embedded test path"),
        )
        .await
        .expect("open embedded SurrealDB");
        schema::bootstrap_schema(&storage)
            .await
            .expect("bootstrap embedded schema");
        storage
    }

    #[tokio::test]
    async fn canvas_graph_round_trip_survives_shutdown_and_reopen() {
        let directory = tempfile::tempdir().expect("temporary MT-136 store root");
        let path = directory.path().join("store");
        let storage = open(&path).await;
        let ctx = WriteContext::system(Some("mt-136-canvas-proof".to_owned()));
        let workspace = storage
            .create_workspace(
                &ctx,
                NewWorkspace {
                    name: "MT-136 canvas durability".to_owned(),
                },
            )
            .await
            .expect("create workspace");
        let canvas = create(
            &storage,
            &ctx,
            NewCanvas {
                workspace_id: workspace.id,
                title: "Durable canvas".to_owned(),
            },
        )
        .await
        .expect("create canvas");
        let graph = update_graph(
            &storage,
            &ctx,
            &canvas.id,
            vec![NewCanvasNode {
                id: Some("node-a".to_owned()),
                kind: "note".to_owned(),
                position_x: 12.5,
                position_y: 25.0,
                data: Some(serde_json::json!({"text": "persist me"})),
            }],
            Vec::new(),
        )
        .await
        .expect("replace canvas graph");
        assert_eq!(graph.nodes.len(), 1);
        storage.shutdown().await.expect("close embedded store");
        drop(storage);

        let reopened = open(&path).await;
        let persisted = get_graph(&reopened, &canvas.id)
            .await
            .expect("read reopened canvas graph");
        assert_eq!(persisted.canvas.title, "Durable canvas");
        assert_eq!(persisted.nodes[0].id, "node-a");
        assert_eq!(
            persisted.nodes[0].data,
            serde_json::json!({"text": "persist me"})
        );
        reopened.shutdown().await.expect("close reopened store");
    }
}
