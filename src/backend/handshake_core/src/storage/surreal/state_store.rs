//! Durable native-editor support state backed by embedded SurrealDB.
//!
//! Every mutation in this module writes its EventLedger receipt and domain
//! row(s) in one SurrealQL transaction. Values are always passed as bindings;
//! none of the operator-controlled state is interpolated into query text.

use std::collections::{BTreeSet, HashSet};
use std::str::FromStr;

use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use surrealdb::types::{Datetime, RecordId, RecordIdKey, SurrealValue};

use super::{event_ledger, SurrealStorage};
use crate::kernel::{KernelActor, KernelEventType, NewKernelEvent};
use crate::storage::{
    DebugBreakpoint, DebugBreakpointInput, LoomSearchResultKind, LoomSearchSourceKind,
    QuickSwitcherRecent, QuickSwitcherRecentInput, StorageError, StorageResult,
    WorkbenchLayoutState, WorkbenchLayoutStateInput, WorkspaceSearchBookmarkState,
    WorkspaceSearchBookmarkStateInput, WorkspaceSettingsState, WorkspaceSettingsStateInput,
    WORKBENCH_LAYOUT_SCHEMA_ID, WORKSPACE_SEARCH_BOOKMARK_SCHEMA_ID, WORKSPACE_SETTINGS_SCHEMA_ID,
};

const WORKSPACES: &str = "workspaces";
const EVENT_LEDGER: &str = "kernel_event_ledger";
const QUICK_RECENTS: &str = "knowledge_quick_switcher_recents";
const RICH_DOCUMENTS: &str = "knowledge_rich_documents";
const DEBUG_BREAKPOINTS: &str = "knowledge_debug_breakpoints";

macro_rules! atomic_with_event {
    ($mutation:literal, $projection:literal) => {
        concat!(
            "BEGIN TRANSACTION; ",
            "CREATE $event.record CONTENT { ",
            "event_id: $event.event_id, event_version: $event.event_version, ",
            "kernel_task_run_id: $event.kernel_task_run_id, session_run_id: $event.session_run_id, ",
            "aggregate_type: $event.aggregate_type, aggregate_id: $event.aggregate_id, ",
            "idempotency_key: $event.idempotency_key, event_type: $event.event_type, ",
            "actor_kind: $event.actor_kind, actor_id: $event.actor_id, ",
            "causation_id: $event.causation_id, correlation_id: $event.correlation_id, ",
            "payload_hash: $event.payload_hash, source_component: $event.source_component, ",
            "payload: $event.payload, created_at: $event.created_at }; ",
            $mutation,
            " COMMIT TRANSACTION; ",
            $projection
        )
    };
}

#[derive(SurrealValue)]
struct WorkspaceBinding {
    workspace: RecordId,
}

#[derive(SurrealValue)]
struct DocumentBinding {
    document: RecordId,
}

#[derive(SurrealValue)]
struct QuickRecentBindings {
    recent: RecordId,
    workspace: RecordId,
    hit_key: String,
    source_kind: String,
    ref_id: String,
    result_kind: String,
    title: String,
    excerpt: String,
    metadata: Value,
    event: event_ledger::LedgerWrite,
}

#[derive(SurrealValue)]
struct StateWriteBindings {
    workspace: RecordId,
    state: Value,
    event: event_ledger::LedgerWrite,
}

#[derive(SurrealValue)]
struct BreakpointWrite {
    record: RecordId,
    source_url: String,
    line: i32,
    condition: Option<String>,
    verified: bool,
}

#[derive(SurrealValue)]
struct BreakpointSetBindings {
    document: RecordId,
    workspace: RecordId,
    breakpoints: Vec<BreakpointWrite>,
    event: event_ledger::LedgerWrite,
}

#[derive(SurrealValue)]
struct QuickRecentRow {
    workspace_id: RecordId,
    hit_key: String,
    source_kind: String,
    ref_id: String,
    result_kind: String,
    title: String,
    excerpt: String,
    metadata: Value,
    selected_count: i64,
    selected_at: Datetime,
    event_ledger_event_id: RecordId,
}

#[derive(SurrealValue)]
struct WorkbenchLayoutRow {
    workspace_id: RecordId,
    layout_state: Value,
    updated_at: Datetime,
    event_ledger_event_id: RecordId,
}

#[derive(SurrealValue)]
struct WorkspaceSettingsRow {
    workspace_id: RecordId,
    settings_state: Value,
    updated_at: Datetime,
    event_ledger_event_id: RecordId,
}

#[derive(SurrealValue)]
struct SearchBookmarkRow {
    workspace_id: RecordId,
    bookmark_state: Value,
    updated_at: Datetime,
    event_ledger_event_id: RecordId,
}

#[derive(SurrealValue)]
struct DebugBreakpointRow {
    breakpoint_id: String,
    rich_document_id: RecordId,
    workspace_id: RecordId,
    source_url: String,
    line: i32,
    condition: Option<String>,
    verified: bool,
    updated_at: Datetime,
    event_ledger_event_id: RecordId,
}

fn string_key(record: RecordId, expected_table: &'static str) -> StorageResult<String> {
    if record.table.as_str() != expected_table {
        return Err(StorageError::Database(format!(
            "expected {expected_table} record id, observed {}",
            record.table
        )));
    }
    match record.key {
        RecordIdKey::String(value) => Ok(value),
        _ => Err(StorageError::Database(format!(
            "{expected_table} record id has a non-string key"
        ))),
    }
}

fn quick_recent_key(workspace_id: &str, hit_key: &str) -> String {
    let mut digest = Sha256::new();
    digest.update(workspace_id.as_bytes());
    digest.update([0]);
    digest.update(hit_key.as_bytes());
    format!("{:x}", digest.finalize())
}

fn build_event(
    run_id: String,
    event_type: KernelEventType,
    actor_id: &str,
    aggregate_type: &str,
    aggregate_id: String,
    source_component: &str,
    payload: Value,
) -> StorageResult<event_ledger::LedgerWrite> {
    let event = NewKernelEvent::builder(
        run_id.clone(),
        run_id,
        event_type,
        KernelActor::System(actor_id.to_owned()),
    )
    .aggregate(aggregate_type, aggregate_id)
    .source_component(source_component)
    .payload(payload)
    .build()
    .map_err(|error| {
        tracing::error!(target: "handshake_core", %error, "state_store_event_build_failed");
        StorageError::Validation("loom bridge EventLedger receipt build failed")
    })?;
    event_ledger::prepare_event(event).map(|(_, write)| write)
}

fn map_quick_recent(row: QuickRecentRow) -> StorageResult<QuickSwitcherRecent> {
    Ok(QuickSwitcherRecent {
        workspace_id: string_key(row.workspace_id, WORKSPACES)?,
        hit_key: row.hit_key,
        source_kind: LoomSearchSourceKind::from_str(&row.source_kind)?,
        ref_id: row.ref_id,
        result_kind: LoomSearchResultKind::from_str(&row.result_kind)?,
        title: row.title,
        excerpt: row.excerpt,
        metadata: row.metadata,
        selected_count: row.selected_count,
        selected_at: row.selected_at.into_inner(),
        event_ledger_event_id: string_key(row.event_ledger_event_id, EVENT_LEDGER)?,
    })
}

fn map_workbench_layout(row: WorkbenchLayoutRow) -> StorageResult<WorkbenchLayoutState> {
    Ok(WorkbenchLayoutState {
        workspace_id: string_key(row.workspace_id, WORKSPACES)?,
        layout_state: row.layout_state,
        updated_at: row.updated_at.into_inner(),
        event_ledger_event_id: string_key(row.event_ledger_event_id, EVENT_LEDGER)?,
    })
}

fn map_workspace_settings(row: WorkspaceSettingsRow) -> StorageResult<WorkspaceSettingsState> {
    Ok(WorkspaceSettingsState {
        workspace_id: string_key(row.workspace_id, WORKSPACES)?,
        settings_state: row.settings_state,
        updated_at: row.updated_at.into_inner(),
        event_ledger_event_id: string_key(row.event_ledger_event_id, EVENT_LEDGER)?,
    })
}

fn map_search_bookmarks(row: SearchBookmarkRow) -> StorageResult<WorkspaceSearchBookmarkState> {
    Ok(WorkspaceSearchBookmarkState {
        workspace_id: string_key(row.workspace_id, WORKSPACES)?,
        bookmark_state: row.bookmark_state,
        updated_at: row.updated_at.into_inner(),
        event_ledger_event_id: string_key(row.event_ledger_event_id, EVENT_LEDGER)?,
    })
}

fn map_debug_breakpoint(row: DebugBreakpointRow) -> StorageResult<DebugBreakpoint> {
    Ok(DebugBreakpoint {
        breakpoint_id: row.breakpoint_id,
        rich_document_id: string_key(row.rich_document_id, RICH_DOCUMENTS)?,
        workspace_id: string_key(row.workspace_id, WORKSPACES)?,
        source_url: row.source_url,
        line: row.line,
        condition: row.condition,
        verified: row.verified,
        updated_at: row.updated_at.into_inner(),
        event_ledger_event_id: string_key(row.event_ledger_event_id, EVENT_LEDGER)?,
    })
}

pub(crate) async fn record_quick_switcher_recent(
    storage: &SurrealStorage,
    workspace_id: &str,
    input: QuickSwitcherRecentInput,
) -> StorageResult<QuickSwitcherRecent> {
    let ref_id = input.ref_id.trim();
    if ref_id.is_empty() {
        return Err(StorageError::Validation(
            "quick switcher recent ref_id is required",
        ));
    }
    let title = input.title.trim();
    if title.is_empty() {
        return Err(StorageError::Validation(
            "quick switcher recent title is required",
        ));
    }

    let ref_id = ref_id.to_owned();
    let title = title.to_owned();
    let excerpt = input.excerpt.trim().to_owned();
    let source_kind = input.source_kind.as_str().to_owned();
    let result_kind = input.result_kind.as_str().to_owned();
    let hit_key = format!("{source_kind}:{ref_id}");
    let metadata = if input.metadata.is_null() {
        json!({})
    } else {
        input.metadata
    };
    let event = build_event(
        format!("QUICK-SWITCHER-RECENTS-{workspace_id}"),
        KernelEventType::KnowledgeQuickSwitcherRecentRecorded,
        "quick-switcher-ui",
        "quick_switcher_recent",
        format!("{workspace_id}:{hit_key}"),
        "quick_switcher_recents",
        json!({
            "type": "knowledge_quick_switcher_recent_recorded",
            "workspace_id": workspace_id,
            "hit_key": hit_key.clone(),
            "source_kind": source_kind.clone(),
            "ref_id": ref_id.clone(),
            "result_kind": result_kind.clone(),
            "title": title.clone(),
            "metadata": metadata.clone(),
        }),
    )?;
    let bindings = QuickRecentBindings {
        recent: RecordId::new(QUICK_RECENTS, quick_recent_key(workspace_id, &hit_key)),
        workspace: RecordId::new(WORKSPACES, workspace_id.to_owned()),
        hit_key,
        source_kind,
        ref_id,
        result_kind,
        title,
        excerpt,
        metadata,
        event,
    };
    let rows: Vec<QuickRecentRow> = storage
        .with_data_operation(move |database| {
            Box::pin(async move {
                database
                    .query_values_at(
                        atomic_with_event!(
                            "LET $prior = SELECT selected_count FROM ONLY $recent; \
                             UPSERT $recent SET workspace_id = $workspace, hit_key = $hit_key, \
                               source_kind = $source_kind, ref_id = $ref_id, result_kind = $result_kind, \
                               title = $title, excerpt = $excerpt, metadata = $metadata, \
                               selected_count = IF $prior = NONE { 1 } ELSE { $prior.selected_count + 1 }, \
                               selected_at = time::now(), event_ledger_event_id = $event.record;",
                            "SELECT workspace_id, hit_key, source_kind, ref_id, result_kind, title, excerpt, \
                               metadata, selected_count, selected_at, event_ledger_event_id FROM $recent;"
                        ),
                        bindings,
                        5,
                    )
                    .await
            })
        })
        .await
        .map_err(StorageError::from)?;
    rows.into_iter()
        .next()
        .map(map_quick_recent)
        .transpose()?
        .ok_or_else(|| StorageError::Database("quick switcher recent write returned no row".into()))
}

pub(crate) async fn list_quick_switcher_recents(
    storage: &SurrealStorage,
    workspace_id: &str,
    limit: u32,
) -> StorageResult<Vec<QuickSwitcherRecent>> {
    if limit == 0 {
        return Ok(Vec::new());
    }
    #[derive(SurrealValue)]
    struct Bindings {
        workspace: RecordId,
        limit: i64,
    }
    let rows: Vec<QuickRecentRow> = storage
        .with_data_operation({
            let workspace = RecordId::new(WORKSPACES, workspace_id.to_owned());
            move |database| {
                Box::pin(async move {
                    database
                        .query_values(
                            "SELECT workspace_id, hit_key, source_kind, ref_id, result_kind, title, excerpt, \
                               metadata, selected_count, selected_at, event_ledger_event_id \
                             FROM knowledge_quick_switcher_recents WHERE workspace_id = $workspace \
                             ORDER BY selected_at DESC, hit_key ASC LIMIT $limit;",
                            Bindings {
                                workspace,
                                limit: i64::from(limit.min(100)),
                            },
                        )
                        .await
                })
            }
        })
        .await
        .map_err(StorageError::from)?;
    rows.into_iter().map(map_quick_recent).collect()
}

pub(crate) async fn get_workbench_layout_state(
    storage: &SurrealStorage,
    workspace_id: &str,
) -> StorageResult<Option<WorkbenchLayoutState>> {
    let row: Option<WorkbenchLayoutRow> = storage
        .with_data_operation({
            let workspace = RecordId::new(WORKSPACES, workspace_id.to_owned());
            move |database| {
                Box::pin(async move {
                    database
                        .query_first(
                            "SELECT workspace_id, layout_state, updated_at, event_ledger_event_id \
                             FROM knowledge_workbench_layout_states WHERE workspace_id = $workspace LIMIT 1;",
                            WorkspaceBinding { workspace },
                        )
                        .await
                })
            }
        })
        .await
        .map_err(StorageError::from)?;
    row.map(map_workbench_layout).transpose()
}

pub(crate) async fn save_workbench_layout_state(
    storage: &SurrealStorage,
    workspace_id: &str,
    input: WorkbenchLayoutStateInput,
) -> StorageResult<WorkbenchLayoutState> {
    if !input.layout_state.is_object() {
        return Err(StorageError::Validation(
            "workbench layout_state must be a JSON object",
        ));
    }
    if input.layout_state.get("schema_id").and_then(Value::as_str)
        != Some(WORKBENCH_LAYOUT_SCHEMA_ID)
    {
        return Err(StorageError::Validation(
            "workbench layout_state schema_id must be hsk.workbench_layout_state@1",
        ));
    }
    validate_workbench_layout_state_shape(&input.layout_state)?;
    let event = build_event(
        format!("WORKBENCH-LAYOUT-{workspace_id}"),
        KernelEventType::KnowledgeWorkbenchLayoutStateRecorded,
        "workbench-layout-ui",
        "workbench_layout_state",
        workspace_id.to_owned(),
        "workbench_layout_state",
        json!({
            "type": "knowledge_workbench_layout_state_recorded",
            "workspace_id": workspace_id,
            "layout_state": input.layout_state.clone(),
        }),
    )?;
    let rows: Vec<WorkbenchLayoutRow> = storage
        .with_data_operation({
            let workspace = RecordId::new(WORKSPACES, workspace_id.to_owned());
            let bindings = StateWriteBindings {
                workspace,
                state: input.layout_state,
                event,
            };
            move |database| {
                Box::pin(async move {
                    database
                        .query_values_at(
                            atomic_with_event!(
                                "LET $state_record = type::record('knowledge_workbench_layout_states', $workspace); \
                                 UPSERT $state_record SET workspace_id = $workspace, layout_state = $state, \
                                   updated_at = time::now(), event_ledger_event_id = $event.record;",
                                "SELECT workspace_id, layout_state, updated_at, event_ledger_event_id \
                                 FROM $state_record;"
                            ),
                            bindings,
                            5,
                        )
                        .await
                })
            }
        })
        .await
        .map_err(StorageError::from)?;
    rows.into_iter()
        .next()
        .map(map_workbench_layout)
        .transpose()?
        .ok_or_else(|| StorageError::Database("workbench layout write returned no row".into()))
}

pub(crate) async fn get_workspace_settings_state(
    storage: &SurrealStorage,
    workspace_id: &str,
) -> StorageResult<Option<WorkspaceSettingsState>> {
    let row: Option<WorkspaceSettingsRow> = storage
        .with_data_operation({
            let workspace = RecordId::new(WORKSPACES, workspace_id.to_owned());
            move |database| {
                Box::pin(async move {
                    database
                        .query_first(
                            "SELECT workspace_id, settings_state, updated_at, event_ledger_event_id \
                             FROM knowledge_workspace_settings_states WHERE workspace_id = $workspace LIMIT 1;",
                            WorkspaceBinding { workspace },
                        )
                        .await
                })
            }
        })
        .await
        .map_err(StorageError::from)?;
    row.map(map_workspace_settings).transpose()
}

pub(crate) async fn save_workspace_settings_state(
    storage: &SurrealStorage,
    workspace_id: &str,
    input: WorkspaceSettingsStateInput,
) -> StorageResult<WorkspaceSettingsState> {
    if !input.settings_state.is_object() {
        return Err(StorageError::Validation(
            "workspace settings_state must be a JSON object",
        ));
    }
    if input
        .settings_state
        .get("schema_id")
        .and_then(Value::as_str)
        != Some(WORKSPACE_SETTINGS_SCHEMA_ID)
    {
        return Err(StorageError::Validation(
            "workspace settings_state schema_id must be hsk.workspace_settings_state@1",
        ));
    }
    validate_workspace_settings_state_shape(&input.settings_state)?;
    let event = build_event(
        format!("WORKSPACE-SETTINGS-{workspace_id}"),
        KernelEventType::KnowledgeWorkspaceSettingsStateRecorded,
        "workspace-settings-ui",
        "workspace_settings_state",
        workspace_id.to_owned(),
        "workspace_settings_state",
        json!({
            "type": "knowledge_workspace_settings_state_recorded",
            "workspace_id": workspace_id,
            "settings_state": input.settings_state.clone(),
        }),
    )?;
    let rows: Vec<WorkspaceSettingsRow> = storage
        .with_data_operation({
            let workspace = RecordId::new(WORKSPACES, workspace_id.to_owned());
            let bindings = StateWriteBindings {
                workspace,
                state: input.settings_state,
                event,
            };
            move |database| {
                Box::pin(async move {
                    database
                        .query_values_at(
                            atomic_with_event!(
                                "LET $state_record = type::record('knowledge_workspace_settings_states', $workspace); \
                                 UPSERT $state_record SET workspace_id = $workspace, settings_state = $state, \
                                   updated_at = time::now(), event_ledger_event_id = $event.record;",
                                "SELECT workspace_id, settings_state, updated_at, event_ledger_event_id \
                                 FROM $state_record;"
                            ),
                            bindings,
                            5,
                        )
                        .await
                })
            }
        })
        .await
        .map_err(StorageError::from)?;
    rows.into_iter()
        .next()
        .map(map_workspace_settings)
        .transpose()?
        .ok_or_else(|| StorageError::Database("workspace settings write returned no row".into()))
}

pub(crate) async fn get_workspace_search_bookmark_state(
    storage: &SurrealStorage,
    workspace_id: &str,
) -> StorageResult<Option<WorkspaceSearchBookmarkState>> {
    let row: Option<SearchBookmarkRow> = storage
        .with_data_operation({
            let workspace = RecordId::new(WORKSPACES, workspace_id.to_owned());
            move |database| {
                Box::pin(async move {
                    database
                        .query_first(
                            "SELECT workspace_id, bookmark_state, updated_at, event_ledger_event_id \
                             FROM knowledge_workspace_search_bookmark_states \
                             WHERE workspace_id = $workspace LIMIT 1;",
                            WorkspaceBinding { workspace },
                        )
                        .await
                })
            }
        })
        .await
        .map_err(StorageError::from)?;
    row.map(map_search_bookmarks).transpose()
}

pub(crate) async fn save_workspace_search_bookmark_state(
    storage: &SurrealStorage,
    workspace_id: &str,
    input: WorkspaceSearchBookmarkStateInput,
) -> StorageResult<WorkspaceSearchBookmarkState> {
    if !input.bookmark_state.is_object() {
        return Err(StorageError::Validation(
            "workspace search bookmark_state must be a JSON object",
        ));
    }
    if input
        .bookmark_state
        .get("schema_id")
        .and_then(Value::as_str)
        != Some(WORKSPACE_SEARCH_BOOKMARK_SCHEMA_ID)
    {
        return Err(StorageError::Validation(
            "workspace search bookmark_state schema_id must be hsk.workspace_search_bookmark_state@1",
        ));
    }
    validate_workspace_search_bookmark_state_shape(&input.bookmark_state)?;
    let event = build_event(
        format!("WORKSPACE-SEARCH-BOOKMARKS-{workspace_id}"),
        KernelEventType::KnowledgeWorkspaceSearchBookmarkStateRecorded,
        "workspace-search-bookmarks-ui",
        "workspace_search_bookmark_state",
        workspace_id.to_owned(),
        "workspace_search_bookmark_state",
        json!({
            "type": "knowledge_workspace_search_bookmark_state_recorded",
            "workspace_id": workspace_id,
            "bookmark_state": input.bookmark_state.clone(),
        }),
    )?;
    let rows: Vec<SearchBookmarkRow> = storage
        .with_data_operation({
            let workspace = RecordId::new(WORKSPACES, workspace_id.to_owned());
            let bindings = StateWriteBindings {
                workspace,
                state: input.bookmark_state,
                event,
            };
            move |database| {
                Box::pin(async move {
                    database
                        .query_values_at(
                            atomic_with_event!(
                                "LET $state_record = type::record('knowledge_workspace_search_bookmark_states', $workspace); \
                                 UPSERT $state_record SET workspace_id = $workspace, bookmark_state = $state, \
                                   updated_at = time::now(), event_ledger_event_id = $event.record;",
                                "SELECT workspace_id, bookmark_state, updated_at, event_ledger_event_id \
                                 FROM $state_record;"
                            ),
                            bindings,
                            5,
                        )
                        .await
                })
            }
        })
        .await
        .map_err(StorageError::from)?;
    rows.into_iter()
        .next()
        .map(map_search_bookmarks)
        .transpose()?
        .ok_or_else(|| StorageError::Database("workspace bookmark write returned no row".into()))
}

pub(crate) async fn list_debug_breakpoints(
    storage: &SurrealStorage,
    rich_document_id: &str,
) -> StorageResult<Vec<DebugBreakpoint>> {
    let rows: Vec<DebugBreakpointRow> = storage
        .with_data_operation({
            let document = RecordId::new(RICH_DOCUMENTS, rich_document_id.to_owned());
            move |database| {
                Box::pin(async move {
                    database
                        .query_values(
                            "SELECT breakpoint_id, rich_document_id, workspace_id, source_url, line, \
                               condition, verified, updated_at, event_ledger_event_id \
                             FROM knowledge_debug_breakpoints WHERE rich_document_id = $document \
                             ORDER BY source_url ASC, line ASC;",
                            DocumentBinding { document },
                        )
                        .await
                })
            }
        })
        .await
        .map_err(StorageError::from)?;
    rows.into_iter().map(map_debug_breakpoint).collect()
}

pub(crate) async fn set_debug_breakpoints(
    storage: &SurrealStorage,
    rich_document_id: &str,
    workspace_id: &str,
    breakpoints: Vec<DebugBreakpointInput>,
) -> StorageResult<Vec<DebugBreakpoint>> {
    for breakpoint in &breakpoints {
        if breakpoint.line < 1 {
            return Err(StorageError::Validation(
                "debug breakpoint line must be >= 1",
            ));
        }
        if breakpoint.source_url.trim().is_empty() {
            return Err(StorageError::Validation(
                "debug breakpoint source_url is required",
            ));
        }
    }
    let event = build_event(
        format!("DEBUG-BREAKPOINTS-{rich_document_id}"),
        KernelEventType::KnowledgeRichDocumentSaved,
        "debug-breakpoints-ui",
        "debug_breakpoints",
        rich_document_id.to_owned(),
        "debug_breakpoints",
        json!({
            "type": "knowledge_debug_breakpoints_recorded",
            "rich_document_id": rich_document_id,
            "workspace_id": workspace_id,
            "breakpoint_count": breakpoints.len(),
        }),
    )?;
    let bindings = BreakpointSetBindings {
        document: RecordId::new(RICH_DOCUMENTS, rich_document_id.to_owned()),
        workspace: RecordId::new(WORKSPACES, workspace_id.to_owned()),
        breakpoints: breakpoints
            .into_iter()
            .map(|breakpoint| BreakpointWrite {
                record: RecordId::new(DEBUG_BREAKPOINTS, format!("bp-{}", uuid::Uuid::new_v4())),
                source_url: breakpoint.source_url,
                line: breakpoint.line,
                condition: breakpoint.condition,
                verified: breakpoint.verified,
            })
            .collect(),
        event,
    };
    let rows: Vec<DebugBreakpointRow> = storage
        .with_data_operation(move |database| {
            Box::pin(async move {
                database
                    .query_values_at(
                        atomic_with_event!(
                            "IF (SELECT VALUE id FROM $document WHERE workspace_id = $workspace)[0] = NONE { \
                               THROW 'HSK-DEBUG-BREAKPOINT-DOCUMENT-WORKSPACE'; \
                             }; \
                             DELETE knowledge_debug_breakpoints \
                               WHERE rich_document_id = $document AND workspace_id = $workspace; \
                             FOR $breakpoint IN $breakpoints { \
                               CREATE $breakpoint.record SET breakpoint_id = record::id($breakpoint.record), \
                                 rich_document_id = $document, workspace_id = $workspace, \
                                 source_url = $breakpoint.source_url, line = $breakpoint.line, \
                                 condition = $breakpoint.condition, verified = $breakpoint.verified, \
                                 created_at = time::now(), updated_at = time::now(), \
                                 event_ledger_event_id = $event.record; \
                             };",
                            "SELECT breakpoint_id, rich_document_id, workspace_id, source_url, line, \
                               condition, verified, updated_at, event_ledger_event_id \
                             FROM knowledge_debug_breakpoints WHERE rich_document_id = $document \
                             ORDER BY source_url ASC, line ASC;"
                        ),
                        bindings,
                        6,
                    )
                    .await
            })
        })
        .await
        .map_err(StorageError::from)?;
    rows.into_iter().map(map_debug_breakpoint).collect()
}

const WORKSPACE_SEARCH_BOOKMARK_SHAPE_VALIDATION_ERROR: &str =
    "workspace search bookmark_state must match hsk.workspace_search_bookmark_state@1 shape";
const WORKSPACE_SEARCH_BOOKMARK_MAX: usize = 20;
const WORKSPACE_SEARCH_BOOKMARK_KINDS: [&str; 10] = [
    "all",
    "document",
    "loom_block",
    "file",
    "tag_hub",
    "symbol",
    "work_packet",
    "micro_task",
    "user_manual_page",
    "wiki_page",
];

fn validate_workspace_search_bookmark_state_shape(bookmark_state: &Value) -> StorageResult<()> {
    let Some(object) = bookmark_state.as_object() else {
        return Err(StorageError::Validation(
            WORKSPACE_SEARCH_BOOKMARK_SHAPE_VALIDATION_ERROR,
        ));
    };
    let Some(bookmarks) = object.get("bookmarks").and_then(Value::as_array) else {
        return Err(StorageError::Validation(
            WORKSPACE_SEARCH_BOOKMARK_SHAPE_VALIDATION_ERROR,
        ));
    };
    if bookmarks.len() > WORKSPACE_SEARCH_BOOKMARK_MAX {
        return Err(StorageError::Validation(
            "workspace search bookmark_state exceeds 20 saved searches",
        ));
    }
    for bookmark in bookmarks {
        let Some(entry) = bookmark.as_object() else {
            return Err(StorageError::Validation(
                WORKSPACE_SEARCH_BOOKMARK_SHAPE_VALIDATION_ERROR,
            ));
        };
        let require_str = |key: &str| -> StorageResult<()> {
            entry
                .get(key)
                .and_then(Value::as_str)
                .map(|_| ())
                .ok_or(StorageError::Validation(
                    WORKSPACE_SEARCH_BOOKMARK_SHAPE_VALIDATION_ERROR,
                ))
        };
        let require_nonempty_str = |key: &str| -> StorageResult<()> {
            match entry.get(key).and_then(Value::as_str) {
                Some(value) if !value.trim().is_empty() => Ok(()),
                _ => Err(StorageError::Validation(
                    WORKSPACE_SEARCH_BOOKMARK_SHAPE_VALIDATION_ERROR,
                )),
            }
        };
        let require_bool = |key: &str| -> StorageResult<()> {
            match entry.get(key) {
                Some(Value::Bool(_)) => Ok(()),
                _ => Err(StorageError::Validation(
                    WORKSPACE_SEARCH_BOOKMARK_SHAPE_VALIDATION_ERROR,
                )),
            }
        };
        require_nonempty_str("id")?;
        require_nonempty_str("label")?;
        require_str("query")?;
        require_nonempty_str("kind")?;
        let kind = entry
            .get("kind")
            .and_then(Value::as_str)
            .ok_or(StorageError::Validation(
                WORKSPACE_SEARCH_BOOKMARK_SHAPE_VALIDATION_ERROR,
            ))?;
        if !WORKSPACE_SEARCH_BOOKMARK_KINDS.contains(&kind) {
            return Err(StorageError::Validation(
                WORKSPACE_SEARCH_BOOKMARK_SHAPE_VALIDATION_ERROR,
            ));
        }
        require_str("tagFilter")?;
        require_str("pathFilter")?;
        require_bool("caseSensitive")?;
        require_bool("wholeWord")?;
        require_bool("isRegex")?;
        require_nonempty_str("savedAt")?;
        let saved_at =
            entry
                .get("savedAt")
                .and_then(Value::as_str)
                .ok_or(StorageError::Validation(
                    WORKSPACE_SEARCH_BOOKMARK_SHAPE_VALIDATION_ERROR,
                ))?;
        if chrono::DateTime::parse_from_rfc3339(saved_at).is_err() {
            return Err(StorageError::Validation(
                WORKSPACE_SEARCH_BOOKMARK_SHAPE_VALIDATION_ERROR,
            ));
        }
    }
    Ok(())
}

const WORKBENCH_LAYOUT_SHAPE_VALIDATION_ERROR: &str =
    "workbench layout_state must match hsk.workbench_layout_state@1 renderable shape";
const WORKBENCH_LAYOUT_PANE_IDS: [&str; 4] = ["pane-a", "pane-b", "pane-c", "pane-d"];
const WORKBENCH_LAYOUT_MODULE_IDS: [&str; 6] = ["MAIN", "CKC", "INGEST", "STAGE", "LAB", "STUDIO"];
const WORKBENCH_LAYOUT_TAB_IDS: [&str; 17] = [
    "workspace",
    "media-downloader",
    "fonts",
    "flight-recorder",
    "kernel-dcc",
    "inference-lab",
    "model-runtime",
    "swarm",
    "problems",
    "jobs",
    "timeline",
    "user-manual",
    "code-symbol",
    "loom-block",
    "loom-wiki-page",
    "atelier",
    "visual-debugger",
];

fn json_string_in(value: Option<&Value>, allowed: &[&str]) -> bool {
    value
        .and_then(Value::as_str)
        .is_some_and(|candidate| allowed.contains(&candidate))
}

fn json_required_bool(value: Option<&Value>) -> bool {
    matches!(value, Some(Value::Bool(_)))
}

fn json_optional_non_empty_string(value: Option<&Value>) -> bool {
    match value {
        None | Some(Value::Null) => true,
        Some(Value::String(candidate)) => !candidate.trim().is_empty(),
        _ => false,
    }
}

fn json_required_split_weight(value: Option<&Value>) -> bool {
    value
        .and_then(Value::as_f64)
        .is_some_and(|candidate| (0.2..=0.8).contains(&candidate))
}

fn validate_workbench_layout_open_documents(value: Option<&Value>) -> bool {
    let Some(value) = value else {
        return true;
    };
    let Some(documents) = value.as_array() else {
        return false;
    };
    documents.iter().all(|document| {
        let Some(document) = document.as_object() else {
            return false;
        };
        document
            .get("documentId")
            .and_then(Value::as_str)
            .is_some_and(|document_id| !document_id.trim().is_empty())
            && document
                .get("pinned")
                .is_none_or(|value| matches!(value, Value::Bool(_)))
            && document
                .get("dirty")
                .is_none_or(|value| matches!(value, Value::Bool(_)))
    })
}

fn validate_workbench_layout_pane(value: &Value) -> bool {
    let Some(pane) = value.as_object() else {
        return false;
    };
    let Some(tabs) = pane.get("tabs").and_then(Value::as_array) else {
        return false;
    };
    let active_document_id = pane.get("activeDocumentId");
    let active_canvas_id = pane.get("activeCanvasId");
    json_string_in(pane.get("id"), &WORKBENCH_LAYOUT_PANE_IDS)
        && json_string_in(pane.get("module"), &WORKBENCH_LAYOUT_MODULE_IDS)
        && json_string_in(pane.get("activeTab"), &WORKBENCH_LAYOUT_TAB_IDS)
        && tabs
            .iter()
            .all(|tab| json_string_in(Some(tab), &WORKBENCH_LAYOUT_TAB_IDS))
        && json_required_bool(pane.get("locked"))
        && matches!(pane.get("projectRef"), Some(Value::String(_)))
        && json_optional_non_empty_string(active_document_id)
        && json_optional_non_empty_string(active_canvas_id)
        && !(matches!(active_document_id, Some(Value::String(_)))
            && matches!(active_canvas_id, Some(Value::String(_))))
        && validate_workbench_layout_open_documents(pane.get("openDocuments"))
}

fn validate_workbench_layout_state_shape(layout_state: &Value) -> StorageResult<()> {
    let Some(layout) = layout_state.as_object() else {
        return Err(StorageError::Validation(
            WORKBENCH_LAYOUT_SHAPE_VALIDATION_ERROR,
        ));
    };
    let Some(split_weights) = layout.get("splitWeights").and_then(Value::as_object) else {
        return Err(StorageError::Validation(
            WORKBENCH_LAYOUT_SHAPE_VALIDATION_ERROR,
        ));
    };
    let Some(drawers) = layout.get("drawers").and_then(Value::as_object) else {
        return Err(StorageError::Validation(
            WORKBENCH_LAYOUT_SHAPE_VALIDATION_ERROR,
        ));
    };
    let Some(panes) = layout.get("panes").and_then(Value::as_array) else {
        return Err(StorageError::Validation(
            WORKBENCH_LAYOUT_SHAPE_VALIDATION_ERROR,
        ));
    };
    let pane_ids = panes
        .iter()
        .filter_map(|pane| pane.get("id").and_then(Value::as_str))
        .collect::<BTreeSet<_>>();
    if !json_string_in(layout.get("activePaneId"), &WORKBENCH_LAYOUT_PANE_IDS)
        || !json_string_in(layout.get("activeModule"), &WORKBENCH_LAYOUT_MODULE_IDS)
        || !json_required_split_weight(split_weights.get("vertical"))
        || !json_required_split_weight(split_weights.get("horizontal"))
        || !json_required_bool(drawers.get("project"))
        || !json_required_bool(drawers.get("file"))
        || !json_required_bool(drawers.get("bottom"))
        || panes.len() != WORKBENCH_LAYOUT_PANE_IDS.len()
        || !WORKBENCH_LAYOUT_PANE_IDS
            .iter()
            .all(|pane_id| pane_ids.contains(pane_id))
        || !panes.iter().all(validate_workbench_layout_pane)
    {
        return Err(StorageError::Validation(
            WORKBENCH_LAYOUT_SHAPE_VALIDATION_ERROR,
        ));
    }
    Ok(())
}

const WORKSPACE_SETTINGS_SHAPE_VALIDATION_ERROR: &str =
    "workspace settings_state must match hsk.workspace_settings_state@1 shape";
const WORKSPACE_SETTINGS_KEYBINDING_ACTION_IDS: [&str; 2] =
    ["app.quick_switcher.open", "app.command_palette.open"];

fn normalize_workspace_settings_chord(value: &str) -> Option<String> {
    let mut parts = value
        .split('-')
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>();
    if parts.is_empty() {
        return None;
    }
    let key = parts.pop()?;
    let mut modifiers = BTreeSet::new();
    for part in parts {
        match part.to_ascii_lowercase().as_str() {
            "mod" | "cmd" | "command" | "meta" | "ctrl" | "control" => {
                modifiers.insert("Mod");
            }
            "alt" | "option" => {
                modifiers.insert("Alt");
            }
            "shift" => {
                modifiers.insert("Shift");
            }
            _ => return None,
        }
    }
    let key = if key.chars().count() == 1 {
        key.to_ascii_lowercase()
    } else {
        key.to_owned()
    };
    let mut normalized = Vec::new();
    for modifier in ["Mod", "Alt", "Shift"] {
        if modifiers.contains(modifier) {
            normalized.push(modifier.to_owned());
        }
    }
    normalized.push(key);
    Some(normalized.join("-"))
}

fn validate_workspace_settings_state_shape(settings_state: &Value) -> StorageResult<()> {
    let Some(settings_object) = settings_state.as_object() else {
        return Err(StorageError::Validation(
            WORKSPACE_SETTINGS_SHAPE_VALIDATION_ERROR,
        ));
    };
    if !json_string_in(settings_object.get("theme"), &["light", "dark"]) {
        return Err(StorageError::Validation(
            WORKSPACE_SETTINGS_SHAPE_VALIDATION_ERROR,
        ));
    }
    let Some(custom_theme_tokens) = settings_object
        .get("custom_theme_tokens")
        .and_then(Value::as_object)
    else {
        return Err(StorageError::Validation(
            WORKSPACE_SETTINGS_SHAPE_VALIDATION_ERROR,
        ));
    };
    if !custom_theme_tokens
        .iter()
        .all(|(key, value)| key.starts_with("--hs-color-") && value.as_str().is_some())
    {
        return Err(StorageError::Validation(
            WORKSPACE_SETTINGS_SHAPE_VALIDATION_ERROR,
        ));
    }
    let Some(keybindings) = settings_object
        .get("keybindings")
        .and_then(Value::as_object)
    else {
        return Err(StorageError::Validation(
            WORKSPACE_SETTINGS_SHAPE_VALIDATION_ERROR,
        ));
    };
    if !keybindings
        .keys()
        .all(|key| WORKSPACE_SETTINGS_KEYBINDING_ACTION_IDS.contains(&key.as_str()))
        || !WORKSPACE_SETTINGS_KEYBINDING_ACTION_IDS
            .iter()
            .all(|action_id| keybindings.contains_key(*action_id))
    {
        return Err(StorageError::Validation(
            WORKSPACE_SETTINGS_SHAPE_VALIDATION_ERROR,
        ));
    }
    let mut normalized_chords = HashSet::new();
    for action_id in WORKSPACE_SETTINGS_KEYBINDING_ACTION_IDS {
        let Some(chord) = keybindings.get(action_id).and_then(Value::as_str) else {
            return Err(StorageError::Validation(
                WORKSPACE_SETTINGS_SHAPE_VALIDATION_ERROR,
            ));
        };
        let Some(normalized) = normalize_workspace_settings_chord(chord) else {
            return Err(StorageError::Validation(
                WORKSPACE_SETTINGS_SHAPE_VALIDATION_ERROR,
            ));
        };
        if !normalized_chords.insert(normalized) {
            return Err(StorageError::Validation(
                "workspace settings_state duplicate keybinding chord",
            ));
        }
    }
    let Some(settings) = settings_object.get("settings").and_then(Value::as_object) else {
        return Err(StorageError::Validation(
            WORKSPACE_SETTINGS_SHAPE_VALIDATION_ERROR,
        ));
    };
    if !json_string_in(settings.get("view_mode"), &["NSFW", "SFW"])
        || !json_required_bool(settings.get("swarm_board_default_open"))
    {
        return Err(StorageError::Validation(
            WORKSPACE_SETTINGS_SHAPE_VALIDATION_ERROR,
        ));
    }
    Ok(())
}
