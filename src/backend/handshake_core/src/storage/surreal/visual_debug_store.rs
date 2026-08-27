//! Bounded, read-only Loom visual-debug projection over SurrealDB authority.

use serde_json::Value;
use surrealdb::types::{RecordId, SurrealValue};

use super::SurrealDatabase;
use crate::storage::{
    Database, LoomAuthorityBackend, LoomBacklink, LoomFolder, LoomGraphEdge, LoomGraphNode,
    LoomGraphSearchResult, LoomSearchFilters, LoomVisualDebugBacklinkState,
    LoomVisualDebugBacklinkSummary, LoomVisualDebugCounts, LoomVisualDebugFolderSummary,
    LoomVisualDebugGraphEdgeSummary, LoomVisualDebugGraphNodeSummary, LoomVisualDebugGraphState,
    LoomVisualDebugSearchHitSummary, LoomVisualDebugSearchState, LoomVisualDebugSnapshot,
    StorageError, StorageResult, LOOM_VISUAL_DEBUG_SCHEMA_ID,
};

#[derive(SurrealValue)]
struct WorkspaceBinding {
    workspace: RecordId,
}

#[derive(SurrealValue)]
struct FolderBinding {
    workspace: RecordId,
    folder: RecordId,
}

#[derive(SurrealValue)]
struct CountRow {
    blocks: i64,
    edges: i64,
    folders: i64,
    folder_members: i64,
    tag_hubs: i64,
    pinned_blocks: i64,
    favorite_blocks: i64,
    indexed_bridges: i64,
}

#[derive(SurrealValue)]
struct ScalarCountRow {
    total: i64,
}

pub(crate) async fn snapshot(
    database: &SurrealDatabase,
    workspace_id: &str,
    start_block_id: &str,
    query: &str,
    limit: u32,
) -> StorageResult<LoomVisualDebugSnapshot> {
    let start_block_id = start_block_id.trim();
    if start_block_id.is_empty() {
        return Err(StorageError::Validation(
            "loom visual-debug start_block_id is required",
        ));
    }
    let query = query.trim();
    if query.is_empty() {
        return Err(StorageError::Validation(
            "loom visual-debug query is required",
        ));
    }

    let cap = limit.clamp(1, 100);
    let folder_sample_limit = cap.min(10);
    let counts = counts(database, workspace_id).await?;
    let local_graph = database
        .local_graph(workspace_id, start_block_id, 2, &[], cap)
        .await?;
    let backlinks = database
        .get_backlinks_with_context(workspace_id, start_block_id)
        .await?;
    let folders = database.list_loom_folders(workspace_id).await?;
    let search_hits = database
        .search_loom_graph(workspace_id, query, LoomSearchFilters::default(), cap, 0)
        .await?;

    let graph = LoomVisualDebugGraphState {
        scope: "local".to_owned(),
        nodes: local_graph.nodes.into_iter().map(node_summary).collect(),
        edges: local_graph.edges.into_iter().map(edge_summary).collect(),
        truncated: local_graph.truncated,
        suppressed_hub_ids: local_graph.suppressed_hub_ids,
    };
    let backlinks = LoomVisualDebugBacklinkState {
        target_block_id: start_block_id.to_owned(),
        incoming: backlinks
            .into_iter()
            .take(cap as usize)
            .map(backlink_summary)
            .collect(),
    };

    let mut folder_summaries = Vec::new();
    for folder in folders.into_iter().take(cap as usize) {
        let member_count = folder_member_count(database, workspace_id, &folder.folder_id).await?;
        let sample_block_ids = database
            .list_loom_folder_blocks(workspace_id, &folder.folder_id, folder_sample_limit, 0)
            .await?
            .into_iter()
            .map(|block| block.block_id)
            .collect();
        folder_summaries.push(folder_summary(folder, member_count, sample_block_ids));
    }

    let result_count = search_hits.len();
    let search = LoomVisualDebugSearchState {
        query: query.to_owned(),
        result_count,
        results: search_hits.into_iter().map(search_summary).collect(),
    };
    Ok(LoomVisualDebugSnapshot {
        workspace_id: workspace_id.to_owned(),
        schema_id: LOOM_VISUAL_DEBUG_SCHEMA_ID,
        authority_backend: LoomAuthorityBackend::SurrealEventLedger,
        authority_class: "projection",
        start_block_id: start_block_id.to_owned(),
        route_ids: route_ids(),
        counts,
        graph,
        backlinks,
        folders: folder_summaries,
        search,
    })
}

async fn counts(
    database: &SurrealDatabase,
    workspace_id: &str,
) -> StorageResult<LoomVisualDebugCounts> {
    let row: Option<CountRow> = database
        .storage()
        .with_data_operation({
            let workspace = RecordId::new("workspaces", workspace_id.to_owned());
            move |db| {
                Box::pin(async move {
                    db.query_first(
                        "SELECT \
                           (SELECT count() AS total FROM loom_blocks WHERE workspace_id = $workspace GROUP ALL)[0].total ?? 0 AS blocks, \
                           (SELECT count() AS total FROM loom_edges WHERE workspace_id = $workspace GROUP ALL)[0].total ?? 0 AS edges, \
                           (SELECT count() AS total FROM loom_folders WHERE workspace_id = $workspace GROUP ALL)[0].total ?? 0 AS folders, \
                           (SELECT count() AS total FROM loom_folder_members WHERE workspace_id = $workspace GROUP ALL)[0].total ?? 0 AS folder_members, \
                           (SELECT count() AS total FROM loom_blocks WHERE workspace_id = $workspace AND content_type = 'tag_hub' GROUP ALL)[0].total ?? 0 AS tag_hubs, \
                           (SELECT count() AS total FROM loom_blocks WHERE workspace_id = $workspace AND pinned = true GROUP ALL)[0].total ?? 0 AS pinned_blocks, \
                           (SELECT count() AS total FROM loom_blocks WHERE workspace_id = $workspace AND favorite = true GROUP ALL)[0].total ?? 0 AS favorite_blocks, \
                           (SELECT count() AS total FROM loom_block_knowledge_bridge WHERE workspace_id = $workspace GROUP ALL)[0].total ?? 0 AS indexed_bridges \
                         FROM $workspace LIMIT 1;",
                        WorkspaceBinding { workspace },
                    )
                    .await
                })
            }
        })
        .await
        .map_err(StorageError::from)?;
    let row = row.ok_or(StorageError::NotFound("workspace"))?;
    Ok(LoomVisualDebugCounts {
        blocks: row.blocks,
        edges: row.edges,
        folders: row.folders,
        folder_members: row.folder_members,
        tag_hubs: row.tag_hubs,
        pinned_blocks: row.pinned_blocks,
        favorite_blocks: row.favorite_blocks,
        indexed_bridges: row.indexed_bridges,
    })
}

async fn folder_member_count(
    database: &SurrealDatabase,
    workspace_id: &str,
    folder_id: &str,
) -> StorageResult<i64> {
    let row: Option<ScalarCountRow> = database
        .storage()
        .with_data_operation({
            let bindings = FolderBinding {
                workspace: RecordId::new("workspaces", workspace_id.to_owned()),
                folder: RecordId::new("loom_folders", folder_id.to_owned()),
            };
            move |db| {
                Box::pin(async move {
                    db.query_first(
                        "SELECT count() AS total FROM loom_folder_members \
                         WHERE workspace_id = $workspace AND folder_id = $folder GROUP ALL;",
                        bindings,
                    )
                    .await
                })
            }
        })
        .await
        .map_err(StorageError::from)?;
    Ok(row.map_or(0, |row| row.total))
}

fn trim(value: &str, max_chars: usize) -> String {
    let trimmed = value.trim();
    let mut output: String = trimmed.chars().take(max_chars).collect();
    if trimmed.chars().count() > max_chars {
        output.push_str("...");
    }
    output
}

fn block_label(block: &crate::storage::LoomBlock) -> String {
    block
        .title
        .as_deref()
        .or(block.original_filename.as_deref())
        .map(|value| trim(value, 120))
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| format!("{} {}", block.content_type.as_str(), block.block_id))
}

fn route_ids() -> Vec<String> {
    [
        "loom.visual_debug",
        "loom.blocks.backlinks",
        "loom.folders.list",
        "loom.graph.local",
        "loom.graph.global",
        "loom.graph_search",
        "loom.search",
    ]
    .into_iter()
    .map(str::to_owned)
    .collect()
}

fn node_summary(node: LoomGraphNode) -> LoomVisualDebugGraphNodeSummary {
    LoomVisualDebugGraphNodeSummary {
        title: block_label(&node.block),
        block_id: node.block.block_id,
        content_type: node.block.content_type,
        depth: node.depth,
        degree: node.degree,
        stale: node.stale,
        entity_id: node.entity_id,
    }
}

fn edge_summary(edge: LoomGraphEdge) -> LoomVisualDebugGraphEdgeSummary {
    LoomVisualDebugGraphEdgeSummary {
        edge_id: edge.edge.edge_id,
        source_block_id: edge.edge.source_block_id,
        target_block_id: edge.edge.target_block_id,
        edge_type: edge.edge.edge_type,
        stale: edge.stale,
    }
}

fn backlink_summary(backlink: LoomBacklink) -> LoomVisualDebugBacklinkSummary {
    LoomVisualDebugBacklinkSummary {
        edge_id: backlink.edge.edge_id,
        source_block_id: backlink.source_block.block_id,
        edge_type: backlink.edge.edge_type,
        context_snippet: backlink.context_snippet.map(|snippet| trim(&snippet, 160)),
    }
}

fn folder_summary(
    folder: LoomFolder,
    member_count: i64,
    sample_block_ids: Vec<String>,
) -> LoomVisualDebugFolderSummary {
    LoomVisualDebugFolderSummary {
        folder_id: folder.folder_id,
        parent_folder_id: folder.parent_folder_id,
        name: trim(&folder.name, 120),
        color: folder.color,
        sort_mode: folder.sort_mode,
        project_ref: folder.project_ref,
        member_count,
        sample_block_ids,
    }
}

fn search_summary(hit: LoomGraphSearchResult) -> LoomVisualDebugSearchHitSummary {
    LoomVisualDebugSearchHitSummary {
        result_kind: hit.result_kind,
        source_kind: hit.source_kind,
        ref_id: hit.ref_id,
        title: trim(&hit.title, 120),
        excerpt: trim(&hit.excerpt, 160),
        authority_table: hit
            .metadata
            .get("authority_table")
            .and_then(Value::as_str)
            .unwrap_or("unknown")
            .to_owned(),
        retrieval_bias_schema_id: hit
            .metadata
            .get("retrieval_bias_schema_id")
            .and_then(Value::as_str)
            .map(str::to_owned),
        retrieval_bias_score: hit
            .metadata
            .get("retrieval_bias_score")
            .and_then(Value::as_f64),
        retrieval_bias_reasons: hit
            .metadata
            .get("retrieval_bias_reasons")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default(),
    }
}
