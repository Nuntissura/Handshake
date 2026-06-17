use super::{
    validate_job_contract, AccessMode, AiJob, AiJobListFilter, Asset, Block, BlockUpdate,
    BronzeRecord, CalendarEvent, CalendarEventExportMode, CalendarEventStatus, CalendarEventUpsert,
    CalendarEventVisibility, CalendarEventWindowQuery, CalendarSource, CalendarSourceProviderType,
    CalendarSourceSyncState, CalendarSourceUpsert, CalendarSourceWritePolicy,
    CalendarSyncStateStage, Canvas, CanvasEdge, CanvasGraph, CanvasNode, DebugBreakpoint,
    DebugBreakpointInput, DefaultStorageGuard,
    Document, EmbeddingModelRecord, EmbeddingRegistry, EntityRef, JobKind, JobMetrics, JobState,
    JobStatusUpdate, LoomBlock, LoomBlockContentType, LoomBlockDerived, LoomBlockSearchResult,
    LoomBlockUpdate, LoomCanvasBoard, LoomCanvasBoardView, LoomCanvasPlacement,
    LoomCanvasPlacementUpdate, LoomCanvasVisualEdge, LoomCollection, LoomCollectionMember,
    LoomCollectionWithMembers, LoomEdge,
    LoomEdgeCreatedBy, LoomEdgeType, LoomFolder, LoomGraphSearchResult,
    LoomSearchFilters, LoomSearchResultKind, LoomSearchSourceKind, LoomSourceAnchor,
    MediaAssetTier, MediaTier, MediaTierStatus, MediaTierUpsert,
    LoomViewFilters, LoomViewGroup, LoomViewResponse, LoomViewType, LoomVisualDebugBacklinkState,
    LoomVisualDebugBacklinkSummary, LoomVisualDebugCounts, LoomVisualDebugFolderSummary,
    LoomVisualDebugGraphEdgeSummary, LoomVisualDebugGraphNodeSummary, LoomVisualDebugGraphState,
    LoomVisualDebugSearchHitSummary, LoomVisualDebugSearchState, LoomVisualDebugSnapshot,
    MergeBackArtifact, ModelSession, ModelSessionState, MutationMetadata, NewAiJob, NewAsset,
    NewBlock, NewBronzeRecord, NewCanvas, NewCanvasEdge, NewCanvasNode, NewDocument, NewLoomBlock,
    NewLoomEdge, NewModelSession, NewNodeExecution, NewSessionMessage, NewSilverRecord,
    NewWorkspace, PlannedOperation, PreviewStatus, QuickSwitcherRecent, QuickSwitcherRecentInput,
    SafetyMode, SessionCheckpoint, SessionMessage, SessionMessageRole, SilverRecord, StorageError,
    StorageGuard, StorageResult, WorkbenchLayoutState, WorkbenchLayoutStateInput,
    WorkflowNodeExecution, WorkflowRun, Workspace, WorkspaceSearchBookmarkState,
    WorkspaceSearchBookmarkStateInput, WorkspaceSettingsState, WorkspaceSettingsStateInput,
    NewLoomCanvasPlacement,
    WriteContext, LOOM_CANVAS_BOARD_SCHEMA_ID, LOOM_VISUAL_DEBUG_SCHEMA_ID,
    WORKBENCH_LAYOUT_SCHEMA_ID,
    WORKSPACE_SEARCH_BOOKMARK_SCHEMA_ID, WORKSPACE_SETTINGS_SCHEMA_ID,
};
use crate::kernel::{
    context_bundle::canonical_json_bytes,
    crdt::{
        persistence::{
            sha256_hex as crdt_sha256_hex, validate_crdt_update_record, CrdtReplayMetadataV1,
            CrdtStorageAuthorityPosture, CrdtUpdateRecordV1,
        },
        snapshot::{validate_crdt_snapshot_record, CrdtSnapshotRecordV1},
    },
    KernelActor, KernelEvent, KernelEventType, KernelSessionLease, NewKernelEvent, SessionBroker,
    SessionRun, SessionRunState,
};
use async_trait::async_trait;
use chrono::{NaiveDateTime, Utc};
use serde_json::{json, Value};
use sqlx::QueryBuilder;
use sqlx::{
    postgres::{PgPool, PgPoolOptions, PgRow},
    Executor, Postgres, Row,
};
use std::collections::{BTreeSet, HashSet, VecDeque};
use std::str::FromStr;
use std::sync::Arc;
use uuid::Uuid;

use crate::workflows::locus::types::{
    executor_eligibility_policy_ids_for_family, governed_action_ids_for_family,
    queue_automation_rule_ids_for_reason, resolve_queue_reason_with_mailbox_context,
    transition_rule_ids_for_family, LocusBindSessionParams, LocusCloseWpParams,
    LocusCompleteMtParams, LocusCreateWpParams, LocusDeleteWpParams, LocusGateKind,
    LocusGateWpParams, LocusGetMtProgressParams, LocusOperation, LocusRecordIterationParams,
    LocusRegisterMtsParams, LocusStartMtParams, LocusUnbindSessionParams, LocusUpdateWpParams,
    MicroTaskIterationOutcome, MicroTaskStatus, RoutingPolicy, TaskBoardStatus, TrackedMicroTask,
    TrackedMicroTaskArtifactV1, WorkPacketPhase, WorkPacketStatus, WorkflowQueueReasonCode,
    WorkflowStateFamily,
};

pub struct PostgresDatabase {
    pool: PgPool,
    guard: Arc<dyn StorageGuard>,
}

impl PostgresDatabase {
    pub(crate) fn pool(&self) -> &PgPool {
        &self.pool
    }

    /// MT-182: the source blocks of edges of a given type whose TARGET is
    /// `target_block_id` (e.g. sub-tags = SUB_TAG sources, tagged blocks = TAG
    /// sources). Deterministic order; newest first.
    async fn loom_blocks_by_incoming_edge(
        &self,
        workspace_id: &str,
        target_block_id: &str,
        edge_type: &str,
    ) -> StorageResult<Vec<LoomBlock>> {
        let rows = sqlx::query(
            r#"
            SELECT DISTINCT
                b.block_id, b.workspace_id, b.content_type, b.document_id,
                b.asset_id, b.title, b.original_filename, b.content_hash,
                b.pinned, b.favorite, b.journal_date, b.created_at, b.updated_at,
                b.imported_at, b.backlink_count, b.mention_count, b.tag_count,
                b.derived_json, b.preview_status, b.thumbnail_asset_id,
                b.proxy_asset_id
            FROM loom_edges e
            JOIN loom_blocks b
              ON b.workspace_id = e.workspace_id
             AND b.block_id = e.source_block_id
            WHERE e.workspace_id = $1
              AND e.target_block_id = $2
              AND e.edge_type = $3
            ORDER BY b.updated_at DESC, b.block_id ASC
            "#,
        )
        .bind(workspace_id)
        .bind(target_block_id)
        .bind(edge_type)
        .fetch_all(&self.pool)
        .await?;
        rows.into_iter().map(map_loom_block).collect()
    }

    /// MT-179/180: assemble a `LoomGraph` from a fixed node id set. Fetches the
    /// blocks, their ProjectKnowledgeIndex bridge entity ids (stale marker +
    /// citation), and the edges WHOSE BOTH ENDPOINTS are in the node set (within
    /// the edge_type filter). Per-node degree is counted within the returned
    /// edges; `ai_suggested` edges are marked stale.
    async fn assemble_loom_graph(
        &self,
        workspace_id: &str,
        node_ids: &[String],
        depth_by_id: &std::collections::HashMap<String, u32>,
        edge_type_filter: Option<&Vec<String>>,
        truncated: bool,
        suppressed_hub_ids: Vec<String>,
    ) -> StorageResult<super::LoomGraph> {
        if node_ids.is_empty() {
            return Ok(super::LoomGraph {
                nodes: Vec::new(),
                edges: Vec::new(),
                truncated,
                suppressed_hub_ids,
            });
        }

        // Blocks for the node set.
        let block_rows = sqlx::query(
            r#"
            SELECT
                b.block_id, b.workspace_id, b.content_type, b.document_id,
                b.asset_id, b.title, b.original_filename, b.content_hash,
                b.pinned, b.favorite, b.journal_date, b.created_at, b.updated_at,
                b.imported_at, b.backlink_count, b.mention_count, b.tag_count,
                b.derived_json, b.preview_status, b.thumbnail_asset_id,
                b.proxy_asset_id
            FROM loom_blocks b
            WHERE b.workspace_id = $1 AND b.block_id = ANY($2::TEXT[])
            "#,
        )
        .bind(workspace_id)
        .bind(node_ids)
        .fetch_all(&self.pool)
        .await?;
        let mut blocks_by_id: std::collections::HashMap<String, LoomBlock> =
            std::collections::HashMap::new();
        for row in block_rows {
            let block = map_loom_block(row)?;
            blocks_by_id.insert(block.block_id.clone(), block);
        }

        // ProjectKnowledgeIndex citations: bridge entity ids for the node set.
        let bridge_rows = sqlx::query(
            r#"
            SELECT block_id, entity_id
            FROM loom_block_knowledge_bridge
            WHERE workspace_id = $1 AND block_id = ANY($2::TEXT[])
            "#,
        )
        .bind(workspace_id)
        .bind(node_ids)
        .fetch_all(&self.pool)
        .await?;
        let mut entity_by_block: std::collections::HashMap<String, String> =
            std::collections::HashMap::new();
        for row in bridge_rows {
            entity_by_block.insert(row.get("block_id"), row.get("entity_id"));
        }

        // Edges with BOTH endpoints in the node set (induced subgraph), within
        // the edge_type filter.
        let edge_rows = sqlx::query(
            r#"
            SELECT
                edge_id, workspace_id, source_block_id, target_block_id,
                edge_type, created_by, created_at, crdt_site_id,
                source_document_id, source_text_block_id, offset_start, offset_end
            FROM loom_edges
            WHERE workspace_id = $1
              AND source_block_id = ANY($2::TEXT[])
              AND target_block_id = ANY($2::TEXT[])
              AND ($3::TEXT[] IS NULL OR edge_type = ANY($3::TEXT[]))
            ORDER BY created_at ASC, edge_id ASC
            "#,
        )
        .bind(workspace_id)
        .bind(node_ids)
        .bind(edge_type_filter)
        .fetch_all(&self.pool)
        .await?;

        let mut degree: std::collections::HashMap<String, u32> = std::collections::HashMap::new();
        let mut edges = Vec::with_capacity(edge_rows.len());
        for row in edge_rows {
            let edge = map_loom_edge(row)?;
            *degree.entry(edge.source_block_id.clone()).or_default() += 1;
            *degree.entry(edge.target_block_id.clone()).or_default() += 1;
            let stale = matches!(edge.edge_type, LoomEdgeType::AiSuggested);
            edges.push(super::LoomGraphEdge { edge, stale });
        }

        // Nodes in deterministic order (BFS depth then id).
        let mut ordered_ids: Vec<String> = blocks_by_id.keys().cloned().collect();
        ordered_ids.sort_by(|a, b| {
            let da = depth_by_id.get(a).copied().unwrap_or(0);
            let db = depth_by_id.get(b).copied().unwrap_or(0);
            da.cmp(&db).then_with(|| a.cmp(b))
        });

        let mut nodes = Vec::with_capacity(ordered_ids.len());
        for id in ordered_ids {
            let Some(block) = blocks_by_id.remove(&id) else {
                continue;
            };
            let entity_id = entity_by_block.get(&id).cloned();
            nodes.push(super::LoomGraphNode {
                block,
                depth: depth_by_id.get(&id).copied().unwrap_or(0),
                degree: degree.get(&id).copied().unwrap_or(0),
                // Stale = not bridged to the ProjectKnowledgeIndex authority.
                stale: entity_id.is_none(),
                entity_id,
            });
        }

        Ok(super::LoomGraph {
            nodes,
            edges,
            truncated,
            suppressed_hub_ids,
        })
    }

    async fn loom_visual_debug_counts(
        &self,
        workspace_id: &str,
    ) -> StorageResult<LoomVisualDebugCounts> {
        let row = sqlx::query(
            r#"
            SELECT
                (SELECT COUNT(*)::BIGINT FROM loom_blocks WHERE workspace_id = $1) AS blocks,
                (SELECT COUNT(*)::BIGINT FROM loom_edges WHERE workspace_id = $1) AS edges,
                (SELECT COUNT(*)::BIGINT FROM loom_folders WHERE workspace_id = $1) AS folders,
                (SELECT COUNT(*)::BIGINT FROM loom_folder_members WHERE workspace_id = $1) AS folder_members,
                (SELECT COUNT(*)::BIGINT FROM loom_blocks WHERE workspace_id = $1 AND content_type = 'tag_hub') AS tag_hubs,
                (SELECT COUNT(*)::BIGINT FROM loom_blocks WHERE workspace_id = $1 AND pinned <> 0) AS pinned_blocks,
                (SELECT COUNT(*)::BIGINT FROM loom_blocks WHERE workspace_id = $1 AND favorite <> 0) AS favorite_blocks,
                (SELECT COUNT(*)::BIGINT FROM loom_block_knowledge_bridge WHERE workspace_id = $1) AS indexed_bridges
            "#,
        )
        .bind(workspace_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(LoomVisualDebugCounts {
            blocks: row.get("blocks"),
            edges: row.get("edges"),
            folders: row.get("folders"),
            folder_members: row.get("folder_members"),
            tag_hubs: row.get("tag_hubs"),
            pinned_blocks: row.get("pinned_blocks"),
            favorite_blocks: row.get("favorite_blocks"),
            indexed_bridges: row.get("indexed_bridges"),
        })
    }

    async fn loom_folder_member_count(
        &self,
        workspace_id: &str,
        folder_id: &str,
    ) -> StorageResult<i64> {
        let count = sqlx::query_scalar(
            r#"
            SELECT COUNT(*)::BIGINT
            FROM loom_folder_members
            WHERE workspace_id = $1 AND folder_id = $2
            "#,
        )
        .bind(workspace_id)
        .bind(folder_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(count)
    }
}

async fn ensure_locus_schema_postgres(pool: &PgPool) -> StorageResult<()> {
    let mut tx = pool.begin().await?;

    let statements = [
        r#"
        CREATE TABLE IF NOT EXISTS work_packets (
            wp_id TEXT PRIMARY KEY,
            version BIGINT NOT NULL,
            title TEXT NOT NULL,
            description TEXT,
            status TEXT NOT NULL,
            priority BIGINT NOT NULL,
            phase TEXT,
            routing TEXT,
            task_packet_path TEXT,
            task_board_status TEXT NOT NULL,
            assignee TEXT,
            reporter TEXT NOT NULL,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            vector_clock TEXT NOT NULL,
            metadata TEXT NOT NULL
        )
        "#,
        r#"CREATE INDEX IF NOT EXISTS idx_wp_status ON work_packets(status)"#,
        r#"CREATE INDEX IF NOT EXISTS idx_wp_priority ON work_packets(priority)"#,
        r#"CREATE INDEX IF NOT EXISTS idx_wp_task_board_status ON work_packets(task_board_status)"#,
        r#"
        CREATE TABLE IF NOT EXISTS micro_tasks (
            mt_id TEXT PRIMARY KEY,
            wp_id TEXT NOT NULL,
            name TEXT NOT NULL,
            status TEXT NOT NULL,
            current_iteration BIGINT,
            escalation_level BIGINT,
            metadata TEXT NOT NULL,
            FOREIGN KEY (wp_id) REFERENCES work_packets(wp_id) ON DELETE CASCADE
        )
        "#,
        r#"
        CREATE TABLE IF NOT EXISTS mt_iterations (
            iteration_id BIGSERIAL PRIMARY KEY,
            mt_id TEXT NOT NULL,
            iteration BIGINT NOT NULL,
            model_id TEXT NOT NULL,
            lora_id TEXT,
            outcome TEXT NOT NULL,
            validation_passed BIGINT,
            duration_ms BIGINT NOT NULL,
            FOREIGN KEY (mt_id) REFERENCES micro_tasks(mt_id) ON DELETE CASCADE
        )
        "#,
        r#"
        CREATE TABLE IF NOT EXISTS dependencies (
            dependency_id TEXT PRIMARY KEY,
            from_wp_id TEXT NOT NULL,
            to_wp_id TEXT NOT NULL,
            type TEXT NOT NULL,
            created_at TEXT NOT NULL,
            vector_clock TEXT NOT NULL,
            FOREIGN KEY (from_wp_id) REFERENCES work_packets(wp_id) ON DELETE CASCADE,
            FOREIGN KEY (to_wp_id) REFERENCES work_packets(wp_id) ON DELETE CASCADE
        )
        "#,
        r#"CREATE INDEX IF NOT EXISTS idx_dep_from ON dependencies(from_wp_id)"#,
        r#"CREATE INDEX IF NOT EXISTS idx_dep_to ON dependencies(to_wp_id)"#,
        r#"CREATE INDEX IF NOT EXISTS idx_dep_type ON dependencies(type)"#,
    ];

    for statement in statements {
        sqlx::query(statement).execute(&mut *tx).await?;
    }

    tx.commit().await?;
    Ok(())
}

async fn ensure_kernel_event_ledger_schema_postgres(pool: &PgPool) -> StorageResult<()> {
    let statements = [
        r#"
        DO $$
        DECLARE
            ledger_rows BIGINT;
            missing_required_columns BOOLEAN;
        BEGIN
            SELECT COUNT(*) INTO ledger_rows FROM kernel_event_ledger;

            SELECT EXISTS (
                SELECT 1
                FROM (
                    VALUES
                        ('event_sequence'),
                        ('event_version'),
                        ('kernel_task_run_id'),
                        ('session_run_id'),
                        ('aggregate_type'),
                        ('aggregate_id'),
                        ('idempotency_key'),
                        ('payload_hash'),
                        ('source_component')
                ) AS required(column_name)
                WHERE NOT EXISTS (
                    SELECT 1
                    FROM information_schema.columns
                    WHERE table_schema = current_schema()
                      AND table_name = 'kernel_event_ledger'
                      AND column_name = required.column_name
                )
            ) INTO missing_required_columns;

            IF missing_required_columns AND ledger_rows > 0 THEN
                RAISE EXCEPTION
                    'kernel_event_ledger has legacy rows without proof-critical fields; run an explicit ledger backfill before applying Kernel V1 schema hardening';
            END IF;
        END $$
        "#,
        r#"
        ALTER TABLE kernel_event_ledger
            ADD COLUMN IF NOT EXISTS event_sequence BIGINT,
            ADD COLUMN IF NOT EXISTS event_version TEXT,
            ADD COLUMN IF NOT EXISTS kernel_task_run_id TEXT,
            ADD COLUMN IF NOT EXISTS session_run_id TEXT,
            ADD COLUMN IF NOT EXISTS aggregate_type TEXT,
            ADD COLUMN IF NOT EXISTS aggregate_id TEXT,
            ADD COLUMN IF NOT EXISTS idempotency_key TEXT,
            ADD COLUMN IF NOT EXISTS payload_hash TEXT,
            ADD COLUMN IF NOT EXISTS source_component TEXT
        "#,
        "CREATE SEQUENCE IF NOT EXISTS kernel_event_ledger_event_sequence_seq",
        "ALTER TABLE kernel_event_ledger ALTER COLUMN event_sequence SET DEFAULT nextval('kernel_event_ledger_event_sequence_seq')",
        "ALTER SEQUENCE kernel_event_ledger_event_sequence_seq OWNED BY kernel_event_ledger.event_sequence",
        "UPDATE kernel_event_ledger SET event_sequence = nextval('kernel_event_ledger_event_sequence_seq') WHERE event_sequence IS NULL",
        "UPDATE kernel_event_ledger SET event_version = 'kernel-event-v1' WHERE event_version IS NULL OR event_version = ''",
        "UPDATE kernel_event_ledger SET kernel_task_run_id = COALESCE(NULLIF(kernel_task_run_id, ''), NULLIF(session_run_id, ''), event_id) WHERE kernel_task_run_id IS NULL OR kernel_task_run_id = ''",
        "UPDATE kernel_event_ledger SET session_run_id = COALESCE(NULLIF(session_run_id, ''), kernel_task_run_id, event_id) WHERE session_run_id IS NULL OR session_run_id = ''",
        "UPDATE kernel_event_ledger SET aggregate_type = 'kernel_task_run' WHERE aggregate_type IS NULL OR aggregate_type = ''",
        "UPDATE kernel_event_ledger SET aggregate_id = kernel_task_run_id WHERE aggregate_id IS NULL OR aggregate_id = ''",
        "UPDATE kernel_event_ledger SET idempotency_key = event_id WHERE idempotency_key IS NULL OR idempotency_key = ''",
        "UPDATE kernel_event_ledger SET source_component = 'legacy-ledger-hardening' WHERE source_component IS NULL OR source_component = ''",
        r#"
        ALTER TABLE kernel_event_ledger
            ALTER COLUMN payload TYPE JSONB
            USING CASE
                WHEN payload IS NULL OR payload::text = '' THEN '{}'::jsonb
                ELSE payload::jsonb
            END
        "#,
        r#"
        ALTER TABLE kernel_event_ledger
            ALTER COLUMN event_sequence SET NOT NULL,
            ALTER COLUMN event_version SET NOT NULL,
            ALTER COLUMN kernel_task_run_id SET NOT NULL,
            ALTER COLUMN session_run_id SET NOT NULL,
            ALTER COLUMN aggregate_type SET NOT NULL,
            ALTER COLUMN aggregate_id SET NOT NULL,
            ALTER COLUMN idempotency_key SET NOT NULL,
            ALTER COLUMN payload_hash SET NOT NULL,
            ALTER COLUMN source_component SET NOT NULL,
            ALTER COLUMN payload SET NOT NULL
        "#,
        "CREATE UNIQUE INDEX IF NOT EXISTS idx_kernel_event_ledger_sequence ON kernel_event_ledger (event_sequence)",
        "CREATE UNIQUE INDEX IF NOT EXISTS idx_kernel_event_ledger_idempotency ON kernel_event_ledger (idempotency_key)",
        "CREATE INDEX IF NOT EXISTS idx_kernel_event_ledger_task ON kernel_event_ledger (kernel_task_run_id)",
        "CREATE INDEX IF NOT EXISTS idx_kernel_event_ledger_session ON kernel_event_ledger (session_run_id)",
        "CREATE INDEX IF NOT EXISTS idx_kernel_event_ledger_aggregate_replay ON kernel_event_ledger (aggregate_type, aggregate_id, event_sequence)",
        "CREATE INDEX IF NOT EXISTS idx_kernel_event_ledger_event_type ON kernel_event_ledger (event_type)",
        "CREATE INDEX IF NOT EXISTS idx_kernel_event_ledger_payload_hash ON kernel_event_ledger (payload_hash)",
        "CREATE INDEX IF NOT EXISTS idx_kernel_event_ledger_correlation ON kernel_event_ledger (correlation_id)",
        "CREATE INDEX IF NOT EXISTS idx_kernel_event_ledger_causation ON kernel_event_ledger (causation_id)",
        "CREATE INDEX IF NOT EXISTS idx_kernel_event_ledger_created_at ON kernel_event_ledger (created_at)",
    ];

    for statement in statements {
        sqlx::query(statement).execute(pool).await?;
    }

    Ok(())
}

fn now_rfc3339() -> String {
    Utc::now().to_rfc3339()
}

fn work_packet_status_str(status: WorkPacketStatus) -> &'static str {
    match status {
        WorkPacketStatus::Unknown => "stub",
        WorkPacketStatus::Ready => "ready",
        WorkPacketStatus::InProgress => "in_progress",
        WorkPacketStatus::Blocked => "blocked",
        WorkPacketStatus::Gated => "gated",
        WorkPacketStatus::Done => "done",
        WorkPacketStatus::Cancelled => "cancelled",
    }
}

fn canonical_work_packet_status_for_storage(value: &str) -> &str {
    match value.trim() {
        "STUB" | "UNKNOWN" | "stub" => "stub",
        "READY" | "READY_FOR_DEV" | "ready" => "ready",
        "IN_PROGRESS" | "in_progress" => "in_progress",
        "BLOCKED" | "blocked" => "blocked",
        "GATED" | "gated" => "gated",
        "DONE" | "done" => "done",
        "CANCELLED" | "cancelled" => "cancelled",
        other => other,
    }
}

fn task_board_status_str(status: TaskBoardStatus) -> &'static str {
    match status {
        TaskBoardStatus::Unknown => "STUB",
        TaskBoardStatus::Ready => "READY",
        TaskBoardStatus::InProgress => "IN_PROGRESS",
        TaskBoardStatus::Blocked => "BLOCKED",
        TaskBoardStatus::Gated => "GATED",
        TaskBoardStatus::Done => "DONE",
        TaskBoardStatus::Cancelled => "CANCELLED",
    }
}

fn micro_task_status_str(status: MicroTaskStatus) -> &'static str {
    match status {
        MicroTaskStatus::Pending => "pending",
        MicroTaskStatus::InProgress => "in_progress",
        MicroTaskStatus::Completed => "completed",
        MicroTaskStatus::Failed => "failed",
        MicroTaskStatus::Blocked => "blocked",
        MicroTaskStatus::Skipped => "skipped",
    }
}

fn micro_task_workflow_state_with_mailbox(
    status: MicroTaskStatus,
    has_pending_mailbox_wait: bool,
) -> (WorkflowStateFamily, WorkflowQueueReasonCode) {
    let (family, base_reason) = match status {
        MicroTaskStatus::Pending => (
            WorkflowStateFamily::Ready,
            WorkflowQueueReasonCode::ReadyForLocalSmallModel,
        ),
        MicroTaskStatus::InProgress => (
            WorkflowStateFamily::Active,
            WorkflowQueueReasonCode::ReadyForLocalSmallModel,
        ),
        MicroTaskStatus::Completed => (
            WorkflowStateFamily::Done,
            WorkflowQueueReasonCode::ValidationWait,
        ),
        MicroTaskStatus::Failed => (
            WorkflowStateFamily::Blocked,
            WorkflowQueueReasonCode::BlockedError,
        ),
        MicroTaskStatus::Blocked => (
            WorkflowStateFamily::Blocked,
            WorkflowQueueReasonCode::BlockedMissingContext,
        ),
        MicroTaskStatus::Skipped => (
            WorkflowStateFamily::Canceled,
            WorkflowQueueReasonCode::BlockedPolicy,
        ),
    };
    let reason = resolve_queue_reason_with_mailbox_context(base_reason, has_pending_mailbox_wait);
    (family, reason)
}

fn tracked_mt_progress_metadata(tracked_mt: &TrackedMicroTask) -> Value {
    let has_pending_mailbox_wait = tracked_mt
        .metadata
        .get("has_pending_mailbox_wait")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let (workflow_state_family, queue_reason_code) =
        micro_task_workflow_state_with_mailbox(tracked_mt.status, has_pending_mailbox_wait);
    let summary_ref = tracked_mt
        .summary_record_path
        .clone()
        .or_else(|| {
            tracked_mt
                .metadata
                .get("structured_collaboration_summary_path")
                .and_then(Value::as_str)
                .map(ToOwned::to_owned)
        })
        .unwrap_or_default();

    let mut artifact_json = serde_json::to_value(TrackedMicroTaskArtifactV1 {
        schema_id: tracked_mt.schema_id.clone(),
        schema_version: tracked_mt.schema_version.clone(),
        record_id: tracked_mt.record_id.clone(),
        record_kind: tracked_mt.record_kind.clone(),
        project_profile_kind: tracked_mt.project_profile_kind,
        profile_extension: tracked_mt.profile_extension.clone(),
        updated_at: tracked_mt.updated_at.to_rfc3339(),
        mirror_state: tracked_mt.mirror_state,
        authority_refs: tracked_mt.authority_refs.clone(),
        evidence_refs: tracked_mt.evidence_refs.clone(),
        mirror_contract: None,
        workflow_state_family,
        queue_reason_code,
        allowed_action_ids: governed_action_ids_for_family(workflow_state_family),
        transition_rule_ids: transition_rule_ids_for_family(workflow_state_family),
        queue_automation_rule_ids: queue_automation_rule_ids_for_reason(queue_reason_code),
        executor_eligibility_policy_ids: executor_eligibility_policy_ids_for_family(
            workflow_state_family,
        ),
        summary_ref,
        mt_id: tracked_mt.mt_id.clone(),
        wp_id: tracked_mt.wp_id.clone(),
        name: tracked_mt.name.clone(),
        scope: tracked_mt.scope.clone(),
        files: tracked_mt.files.clone(),
        done_criteria: tracked_mt.done_criteria.clone(),
        status: tracked_mt.status,
        active_session_ids: tracked_mt.active_session_ids.clone(),
        iterations: tracked_mt.iterations.clone(),
        current_iteration: tracked_mt.current_iteration,
        max_iterations: tracked_mt.max_iterations,
        validation_result: tracked_mt.validation_result.clone(),
        escalation: tracked_mt.escalation.clone(),
        started_at: tracked_mt.started_at,
        completed_at: tracked_mt.completed_at,
        duration_ms: tracked_mt.duration_ms,
        depends_on: tracked_mt.depends_on.clone(),
        metadata: tracked_mt.metadata.clone(),
    })
    .unwrap_or_else(|_| tracked_mt.metadata.clone());

    if let Some(obj) = artifact_json.as_object_mut() {
        obj.insert(
            "active_session_ids".to_string(),
            Value::Array(
                tracked_mt
                    .active_session_ids
                    .iter()
                    .cloned()
                    .map(Value::String)
                    .collect(),
            ),
        );
    }

    apply_canonical_overrides_to_progress_metadata(&mut artifact_json, tracked_mt);

    artifact_json
}

fn apply_canonical_overrides_to_progress_metadata(
    artifact_json: &mut Value,
    tracked_mt: &TrackedMicroTask,
) {
    let Some(obj) = artifact_json.as_object_mut() else {
        return;
    };
    let workflow_state_family = obj
        .get("workflow_state_family")
        .and_then(|value| serde_json::from_value::<WorkflowStateFamily>(value.clone()).ok());
    if let Some(family) = workflow_state_family {
        let canonical_actions: Vec<String> = governed_action_ids_for_family(family)
            .iter()
            .map(|action| (*action).to_string())
            .collect();
        if let Ok(value) = serde_json::to_value(&canonical_actions) {
            obj.insert("allowed_action_ids".to_string(), value);
        }
    }

    let has_pending_mailbox_wait = tracked_mt
        .metadata
        .get("has_pending_mailbox_wait")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    if !has_pending_mailbox_wait {
        return;
    }
    let base_reason = obj
        .get("queue_reason_code")
        .and_then(|value| serde_json::from_value::<WorkflowQueueReasonCode>(value.clone()).ok())
        .unwrap_or(WorkflowQueueReasonCode::ReadyForLocalSmallModel);
    let resolved = resolve_queue_reason_with_mailbox_context(base_reason, true);
    if let Ok(value) = serde_json::to_value(resolved) {
        obj.insert("queue_reason_code".to_string(), value);
    }
}

fn phase_str(phase: WorkPacketPhase) -> &'static str {
    match phase {
        WorkPacketPhase::Phase0 => "0",
        WorkPacketPhase::Phase0_5 => "0.5",
        WorkPacketPhase::Phase1 => "1",
        WorkPacketPhase::Phase2 => "2",
        WorkPacketPhase::Phase3 => "3",
        WorkPacketPhase::Phase4 => "4",
    }
}

fn routing_str(routing: RoutingPolicy) -> &'static str {
    match routing {
        RoutingPolicy::GovStrict => "GOV_STRICT",
        RoutingPolicy::GovStandard => "GOV_STANDARD",
        RoutingPolicy::GovLight => "GOV_LIGHT",
        RoutingPolicy::GovNone => "GOV_NONE",
    }
}

fn mt_iteration_outcome_str(outcome: MicroTaskIterationOutcome) -> &'static str {
    match outcome {
        MicroTaskIterationOutcome::Success => "SUCCESS",
        MicroTaskIterationOutcome::Retry => "RETRY",
        MicroTaskIterationOutcome::Escalate => "ESCALATE",
        MicroTaskIterationOutcome::Blocked => "BLOCKED",
        MicroTaskIterationOutcome::Skipped => "SKIPPED",
    }
}

async fn ensure_wp_exists(pool: &PgPool, wp_id: &str) -> StorageResult<()> {
    let exists =
        sqlx::query_scalar::<_, bool>("SELECT EXISTS(SELECT 1 FROM work_packets WHERE wp_id = $1)")
            .bind(wp_id)
            .fetch_one(pool)
            .await?;

    if !exists {
        return Err(StorageError::NotFound("work_packet"));
    }

    Ok(())
}

async fn ensure_mt_exists_for_wp(pool: &PgPool, wp_id: &str, mt_id: &str) -> StorageResult<()> {
    let exists = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM micro_tasks WHERE mt_id = $1 AND wp_id = $2)",
    )
    .bind(mt_id)
    .bind(wp_id)
    .fetch_one(pool)
    .await?;

    if !exists {
        return Err(StorageError::NotFound("micro_task"));
    }

    Ok(())
}

fn dedupe_session_ids(active_session_ids: &mut Vec<String>) {
    let mut seen = HashSet::new();
    let normalized = active_session_ids
        .iter()
        .filter_map(|session_id| {
            let trimmed = session_id.trim();
            if trimmed.is_empty() {
                return None;
            }

            let normalized = trimmed.to_string();
            if seen.insert(normalized.clone()) {
                Some(normalized)
            } else {
                None
            }
        })
        .collect();
    *active_session_ids = normalized;
}

fn tracked_mt_iteration_index(
    tracked_mt: &TrackedMicroTask,
    iteration: &crate::workflows::locus::types::MicroTaskIterationRecord,
) -> Option<usize> {
    tracked_mt.iterations.iter().position(|existing| {
        existing.iteration == iteration.iteration
            && existing.escalation_level == iteration.escalation_level
    })
}

fn upsert_tracked_mt_iteration(
    tracked_mt: &mut TrackedMicroTask,
    iteration: crate::workflows::locus::types::MicroTaskIterationRecord,
) {
    if let Some(index) = tracked_mt_iteration_index(tracked_mt, &iteration) {
        tracked_mt.iterations[index] = iteration;
    } else {
        tracked_mt.iterations.push(iteration);
    }
}

async fn load_tracked_mt_for_update(
    tx: &mut sqlx::Transaction<'_, Postgres>,
    wp_id: &str,
    mt_id: &str,
) -> StorageResult<TrackedMicroTask> {
    let metadata = sqlx::query_scalar::<_, String>(
        r#"
        SELECT metadata
        FROM micro_tasks
        WHERE mt_id = $1 AND wp_id = $2
        "#,
    )
    .bind(mt_id)
    .bind(wp_id)
    .fetch_optional(&mut **tx)
    .await?;

    let Some(metadata) = metadata else {
        return Err(StorageError::NotFound("micro_task"));
    };

    let mut tracked_mt: TrackedMicroTask = serde_json::from_str(&metadata)?;
    dedupe_session_ids(&mut tracked_mt.active_session_ids);
    Ok(tracked_mt)
}

async fn persist_tracked_mt(
    tx: &mut sqlx::Transaction<'_, Postgres>,
    tracked_mt: &TrackedMicroTask,
) -> StorageResult<()> {
    let metadata = serde_json::to_string(tracked_mt)?;
    let result = sqlx::query(
        r#"
        UPDATE micro_tasks
        SET
            name = $1,
            status = $2,
            current_iteration = $3,
            escalation_level = $4,
            metadata = $5
        WHERE mt_id = $6 AND wp_id = $7
        "#,
    )
    .bind(&tracked_mt.name)
    .bind(micro_task_status_str(tracked_mt.status))
    .bind(tracked_mt.current_iteration as i64)
    .bind(tracked_mt.escalation.current_level as i64)
    .bind(metadata)
    .bind(&tracked_mt.mt_id)
    .bind(&tracked_mt.wp_id)
    .execute(&mut **tx)
    .await?;

    if result.rows_affected() == 0 {
        return Err(StorageError::NotFound("micro_task"));
    }

    Ok(())
}

async fn create_wp(pool: &PgPool, params: LocusCreateWpParams) -> StorageResult<Value> {
    if params.priority > 4 {
        return Err(StorageError::Validation("priority must be between 0 and 4"));
    }

    let existing =
        sqlx::query_scalar::<_, bool>("SELECT EXISTS(SELECT 1 FROM work_packets WHERE wp_id = $1)")
            .bind(&params.wp_id)
            .fetch_one(pool)
            .await?;
    if existing {
        return Err(StorageError::Conflict("work_packet already exists"));
    }

    let now = now_rfc3339();
    let status = WorkPacketStatus::Unknown;
    let task_board_status = TaskBoardStatus::Unknown;
    let vector_clock = json!({"local": 1});
    let metadata = json!({
        "labels": params.labels.unwrap_or_default(),
        "spec_session_id": params.spec_session_id,
        "notes": [],
        "gates": {
            "pre_work": { "status": "pending" },
            "post_work": { "status": "pending" }
        },
        "started_at": null,
        "completed_at": null,
        "due_at": null,
        "tombstone": null,
        "work_packet_type": serde_json::to_value(params.kind)?,
    });

    sqlx::query(
        r#"
        INSERT INTO work_packets (
            wp_id, version, title, description, status, priority, phase, routing, task_packet_path,
            task_board_status, assignee, reporter, created_at, updated_at, vector_clock, metadata
        )
        VALUES (
            $1, $2, $3, $4, $5, $6, $7, $8, $9,
            $10, $11, $12, $13, $14, $15, $16
        )
        "#,
    )
    .bind(&params.wp_id)
    .bind(1i64)
    .bind(&params.title)
    .bind(&params.description)
    .bind(work_packet_status_str(status))
    .bind(params.priority as i64)
    .bind(phase_str(params.phase))
    .bind(routing_str(params.routing))
    .bind(params.task_packet_path.as_deref())
    .bind(task_board_status_str(task_board_status))
    .bind(params.assignee.as_deref())
    .bind(&params.reporter)
    .bind(&now)
    .bind(&now)
    .bind(serde_json::to_string(&vector_clock)?)
    .bind(serde_json::to_string(&metadata)?)
    .execute(pool)
    .await?;

    Ok(json!({
        "wp_id": params.wp_id,
        "version": 1,
        "status": work_packet_status_str(status),
        "task_board_status": task_board_status_str(task_board_status),
        "created_at": now,
        "updated_at": now,
    }))
}

async fn update_wp(pool: &PgPool, params: LocusUpdateWpParams) -> StorageResult<Value> {
    ensure_wp_exists(pool, &params.wp_id).await?;

    let now = now_rfc3339();

    let mut cols: Vec<(&'static str, Value)> = Vec::new();
    for (key, value) in params.updates {
        let col = match key.as_str() {
            "title" => "title",
            "description" => "description",
            "priority" => "priority",
            "status" => "status",
            "assignee" => "assignee",
            "governance.phase" | "phase" => "phase",
            "governance.routing" | "routing" => "routing",
            "governance.task_packet_path" | "task_packet_path" => "task_packet_path",
            "governance.task_board_status" | "task_board_status" => "task_board_status",
            other => {
                return Err(StorageError::Validation(match other {
                    "" => "empty update key",
                    _ => "unsupported update key",
                }));
            }
        };
        cols.push((col, value));
    }

    if cols.is_empty() {
        return Ok(json!({
            "wp_id": params.wp_id,
            "updated_at": now,
            "no_op": true,
        }));
    }

    let mut sql = String::from("UPDATE work_packets SET version = version + 1, updated_at = $1");
    for (idx, (col, _)) in cols.iter().enumerate() {
        sql.push_str(", ");
        sql.push_str(col);
        sql.push_str(" = $");
        sql.push_str(&(idx + 2).to_string());
    }
    sql.push_str(" WHERE wp_id = $");
    sql.push_str(&(cols.len() + 2).to_string());

    let mut query = sqlx::query(&sql).bind(&now);
    for (col, value) in cols {
        match col {
            "priority" => match value {
                Value::Number(n) => {
                    let prio = n
                        .as_i64()
                        .ok_or(StorageError::Validation("priority must be an integer"))?;
                    if !(0..=4).contains(&prio) {
                        return Err(StorageError::Validation("priority must be between 0 and 4"));
                    }
                    query = query.bind(prio);
                }
                _ => return Err(StorageError::Validation("priority must be an integer")),
            },
            _ => match value {
                Value::String(s) => query = query.bind(s),
                Value::Null => query = query.bind(Option::<String>::None),
                _ => return Err(StorageError::Validation("unsupported update value type")),
            },
        }
    }
    query = query.bind(&params.wp_id);

    let result = query.execute(pool).await?;
    if result.rows_affected() == 0 {
        return Err(StorageError::NotFound("work_packet"));
    }

    Ok(json!({
        "wp_id": params.wp_id,
        "updated_at": now,
    }))
}

async fn gate_wp(pool: &PgPool, params: LocusGateWpParams) -> StorageResult<Value> {
    ensure_wp_exists(pool, &params.wp_id).await?;

    let row = sqlx::query_scalar::<_, String>("SELECT metadata FROM work_packets WHERE wp_id = $1")
        .bind(&params.wp_id)
        .fetch_one(pool)
        .await?;
    let mut metadata: Value = serde_json::from_str(&row)?;

    let gate_key = match params.gate {
        LocusGateKind::PreWork => "pre_work",
        LocusGateKind::PostWork => "post_work",
    };

    let gate_value = serde_json::to_value(&params.result)?;
    metadata
        .as_object_mut()
        .ok_or(StorageError::Validation("metadata must be an object"))?
        .entry("gates".to_string())
        .or_insert_with(|| json!({}))
        .as_object_mut()
        .ok_or(StorageError::Validation("metadata.gates must be an object"))?
        .insert(gate_key.to_string(), gate_value);

    let now = now_rfc3339();
    sqlx::query(
        "UPDATE work_packets SET version = version + 1, updated_at = $1, metadata = $2 WHERE wp_id = $3",
    )
    .bind(&now)
    .bind(serde_json::to_string(&metadata)?)
    .bind(&params.wp_id)
    .execute(pool)
    .await?;

    Ok(json!({
        "wp_id": params.wp_id,
        "gate": gate_key,
        "updated_at": now,
    }))
}

async fn close_wp(pool: &PgPool, params: LocusCloseWpParams) -> StorageResult<Value> {
    ensure_wp_exists(pool, &params.wp_id).await?;
    let now = now_rfc3339();
    sqlx::query(
        r#"
        UPDATE work_packets
        SET
            version = version + 1,
            status = $1,
            task_board_status = $2,
            updated_at = $3
        WHERE wp_id = $4
        "#,
    )
    .bind(work_packet_status_str(WorkPacketStatus::Done))
    .bind(task_board_status_str(TaskBoardStatus::Done))
    .bind(&now)
    .bind(&params.wp_id)
    .execute(pool)
    .await?;

    Ok(json!({
        "wp_id": params.wp_id,
        "status": "done",
        "updated_at": now,
    }))
}

async fn delete_wp(pool: &PgPool, params: LocusDeleteWpParams) -> StorageResult<Value> {
    ensure_wp_exists(pool, &params.wp_id).await?;
    let now = now_rfc3339();

    let row = sqlx::query_scalar::<_, String>("SELECT metadata FROM work_packets WHERE wp_id = $1")
        .bind(&params.wp_id)
        .fetch_one(pool)
        .await?;
    let mut metadata: Value = serde_json::from_str(&row)?;
    metadata
        .as_object_mut()
        .ok_or(StorageError::Validation("metadata must be an object"))?
        .insert("tombstone".to_string(), json!({ "deleted_at": now }));

    sqlx::query(
        r#"
        UPDATE work_packets
        SET
            version = version + 1,
            status = $1,
            task_board_status = $2,
            updated_at = $3,
            metadata = $4
        WHERE wp_id = $5
        "#,
    )
    .bind(work_packet_status_str(WorkPacketStatus::Cancelled))
    .bind(task_board_status_str(TaskBoardStatus::Cancelled))
    .bind(&now)
    .bind(serde_json::to_string(&metadata)?)
    .bind(&params.wp_id)
    .execute(pool)
    .await?;

    Ok(json!({
        "wp_id": params.wp_id,
        "status": "cancelled",
        "updated_at": now,
    }))
}

async fn register_mts(pool: &PgPool, params: LocusRegisterMtsParams) -> StorageResult<Value> {
    ensure_wp_exists(pool, &params.wp_id).await?;

    let mut tx = pool.begin().await?;
    for mut mt in params.micro_tasks {
        if mt.wp_id != params.wp_id {
            return Err(StorageError::Validation("micro task wp_id mismatch"));
        }

        dedupe_session_ids(&mut mt.active_session_ids);
        let existing_wp_id = sqlx::query_scalar::<_, String>(
            "SELECT wp_id FROM micro_tasks WHERE mt_id = $1 LIMIT 1",
        )
        .bind(&mt.mt_id)
        .fetch_optional(&mut *tx)
        .await?;
        if let Some(existing_wp_id) = existing_wp_id {
            if existing_wp_id != params.wp_id {
                return Err(StorageError::Conflict(
                    "micro_task already registered to a different work_packet",
                ));
            }
            continue;
        }

        let metadata = serde_json::to_string(&mt)?;
        sqlx::query(
            r#"
            INSERT INTO micro_tasks (
                mt_id, wp_id, name, status, current_iteration, escalation_level, metadata
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            "#,
        )
        .bind(&mt.mt_id)
        .bind(&mt.wp_id)
        .bind(&mt.name)
        .bind(micro_task_status_str(mt.status))
        .bind(mt.current_iteration as i64)
        .bind(mt.escalation.current_level as i64)
        .bind(metadata)
        .execute(&mut *tx)
        .await?;
    }

    tx.commit().await?;
    Ok(json!({
        "wp_id": params.wp_id,
        "registered": true,
    }))
}

async fn start_mt(pool: &PgPool, params: LocusStartMtParams) -> StorageResult<Value> {
    ensure_wp_exists(pool, &params.wp_id).await?;
    ensure_mt_exists_for_wp(pool, &params.wp_id, &params.mt_id).await?;
    let now = now_rfc3339();
    let mut tx = pool.begin().await?;
    let mut tracked_mt = load_tracked_mt_for_update(&mut tx, &params.wp_id, &params.mt_id).await?;
    tracked_mt.status = MicroTaskStatus::InProgress;
    tracked_mt.escalation.current_level = params.escalation_level;
    if tracked_mt.started_at.is_none() {
        tracked_mt.started_at = Some(Utc::now());
    }
    persist_tracked_mt(&mut tx, &tracked_mt).await?;
    tx.commit().await?;

    Ok(json!({
        "wp_id": params.wp_id,
        "mt_id": params.mt_id,
        "status": "in_progress",
        "model_id": params.model_id,
        "lora_id": params.lora_id,
        "escalation_level": params.escalation_level,
        "updated_at": now,
    }))
}

async fn record_iteration(
    pool: &PgPool,
    params: LocusRecordIterationParams,
) -> StorageResult<Value> {
    ensure_wp_exists(pool, &params.wp_id).await?;
    ensure_mt_exists_for_wp(pool, &params.wp_id, &params.mt_id).await?;

    let mut tx = pool.begin().await?;
    let mut tracked_mt = load_tracked_mt_for_update(&mut tx, &params.wp_id, &params.mt_id).await?;
    let recorded_iteration = tracked_mt_iteration_index(&tracked_mt, &params.iteration)
        .map(|index| index as u32 + 1)
        .unwrap_or(tracked_mt.iterations.len() as u32 + 1);
    let existing_iteration_id = sqlx::query_scalar::<_, i64>(
        r#"
        SELECT iteration_id
        FROM mt_iterations
        WHERE mt_id = $1 AND iteration = $2
        LIMIT 1
        "#,
    )
    .bind(&params.mt_id)
    .bind(recorded_iteration as i64)
    .fetch_optional(&mut *tx)
    .await?;
    if let Some(iteration_id) = existing_iteration_id {
        sqlx::query(
            r#"
            UPDATE mt_iterations
            SET
                model_id = $1,
                lora_id = $2,
                outcome = $3,
                validation_passed = $4,
                duration_ms = $5
            WHERE iteration_id = $6
            "#,
        )
        .bind(&params.iteration.model_id)
        .bind(params.iteration.lora_id.as_deref())
        .bind(mt_iteration_outcome_str(params.iteration.outcome))
        .bind(
            params
                .iteration
                .validation_passed
                .map(|v| if v { 1i64 } else { 0i64 }),
        )
        .bind(params.iteration.duration_ms as i64)
        .bind(iteration_id)
        .execute(&mut *tx)
        .await?;
    } else {
        sqlx::query(
            r#"
            INSERT INTO mt_iterations (
                mt_id, iteration, model_id, lora_id, outcome, validation_passed, duration_ms
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            "#,
        )
        .bind(&params.mt_id)
        .bind(recorded_iteration as i64)
        .bind(&params.iteration.model_id)
        .bind(params.iteration.lora_id.as_deref())
        .bind(mt_iteration_outcome_str(params.iteration.outcome))
        .bind(
            params
                .iteration
                .validation_passed
                .map(|v| if v { 1i64 } else { 0i64 }),
        )
        .bind(params.iteration.duration_ms as i64)
        .execute(&mut *tx)
        .await?;
    }

    tracked_mt.status = MicroTaskStatus::InProgress;
    tracked_mt.current_iteration = tracked_mt.current_iteration.max(recorded_iteration);
    tracked_mt.escalation.current_level = params.iteration.escalation_level;
    upsert_tracked_mt_iteration(&mut tracked_mt, params.iteration.clone());
    persist_tracked_mt(&mut tx, &tracked_mt).await?;

    tx.commit().await?;

    Ok(json!({
        "wp_id": params.wp_id,
        "mt_id": params.mt_id,
        "iteration": params.iteration.iteration,
        "recorded_iteration": recorded_iteration,
    }))
}

async fn complete_mt(pool: &PgPool, params: LocusCompleteMtParams) -> StorageResult<Value> {
    ensure_wp_exists(pool, &params.wp_id).await?;
    ensure_mt_exists_for_wp(pool, &params.wp_id, &params.mt_id).await?;
    let mut tx = pool.begin().await?;
    let mut tracked_mt = load_tracked_mt_for_update(&mut tx, &params.wp_id, &params.mt_id).await?;
    tracked_mt.status = MicroTaskStatus::Completed;
    tracked_mt.current_iteration = tracked_mt
        .current_iteration
        .max(tracked_mt.iterations.len() as u32)
        .max(params.final_iteration);
    tracked_mt.active_session_ids.clear();
    if tracked_mt.completed_at.is_none() {
        tracked_mt.completed_at = Some(Utc::now());
    }
    persist_tracked_mt(&mut tx, &tracked_mt).await?;
    tx.commit().await?;

    Ok(json!({
        "wp_id": params.wp_id,
        "mt_id": params.mt_id,
        "status": "completed",
    }))
}

async fn bind_session(pool: &PgPool, params: LocusBindSessionParams) -> StorageResult<Value> {
    ensure_wp_exists(pool, &params.wp_id).await?;
    ensure_mt_exists_for_wp(pool, &params.wp_id, &params.mt_id).await?;
    let session_id = params.session_id.trim().to_string();
    if session_id.is_empty() {
        return Err(StorageError::Validation("session_id"));
    }

    let mut tx = pool.begin().await?;
    let mut tracked_mt = load_tracked_mt_for_update(&mut tx, &params.wp_id, &params.mt_id).await?;
    tracked_mt.status = MicroTaskStatus::InProgress;
    tracked_mt.escalation.current_level = params.escalation_level;
    tracked_mt.active_session_ids.push(session_id.clone());
    dedupe_session_ids(&mut tracked_mt.active_session_ids);
    persist_tracked_mt(&mut tx, &tracked_mt).await?;
    tx.commit().await?;

    Ok(json!({
        "wp_id": params.wp_id,
        "mt_id": params.mt_id,
        "session_id": session_id,
        "active_session_ids": tracked_mt.active_session_ids,
    }))
}

async fn unbind_session(pool: &PgPool, params: LocusUnbindSessionParams) -> StorageResult<Value> {
    ensure_wp_exists(pool, &params.wp_id).await?;
    ensure_mt_exists_for_wp(pool, &params.wp_id, &params.mt_id).await?;
    let session_id = params.session_id.trim().to_string();
    if session_id.is_empty() {
        return Err(StorageError::Validation("session_id"));
    }

    let mut tx = pool.begin().await?;
    let mut tracked_mt = load_tracked_mt_for_update(&mut tx, &params.wp_id, &params.mt_id).await?;
    tracked_mt
        .active_session_ids
        .retain(|existing_session_id| existing_session_id != &session_id);
    persist_tracked_mt(&mut tx, &tracked_mt).await?;
    tx.commit().await?;

    Ok(json!({
        "wp_id": params.wp_id,
        "mt_id": params.mt_id,
        "session_id": session_id,
        "active_session_ids": tracked_mt.active_session_ids,
        "reason": params.reason,
    }))
}

async fn get_mt_progress(pool: &PgPool, params: LocusGetMtProgressParams) -> StorageResult<Value> {
    let row = sqlx::query_as::<
        _,
        (
            String,
            String,
            String,
            String,
            Option<i64>,
            Option<i64>,
            String,
        ),
    >(
        r#"
        SELECT mt_id, wp_id, name, status, current_iteration, escalation_level, metadata
        FROM micro_tasks
        WHERE mt_id = $1
        "#,
    )
    .bind(&params.mt_id)
    .fetch_optional(pool)
    .await?;

    let Some((mt_id, wp_id, name, status, current_iteration, escalation_level, metadata)) = row
    else {
        return Err(StorageError::NotFound("micro_task"));
    };

    let metadata_json: Value = match serde_json::from_str::<TrackedMicroTask>(&metadata) {
        Ok(tracked_mt) => tracked_mt_progress_metadata(&tracked_mt),
        Err(_) => serde_json::from_str(&metadata).unwrap_or_else(|_| json!({})),
    };

    Ok(json!({
        "mt_id": mt_id,
        "wp_id": wp_id,
        "name": name,
        "status": status,
        "current_iteration": current_iteration,
        "escalation_level": escalation_level,
        "metadata": metadata_json,
    }))
}

pub(crate) async fn execute_locus_operation(
    postgres: &PostgresDatabase,
    op: LocusOperation,
) -> StorageResult<Value> {
    let pool = postgres.pool();
    match op {
        LocusOperation::CreateWp(params) => create_wp(pool, params).await,
        LocusOperation::UpdateWp(params) => update_wp(pool, params).await,
        LocusOperation::GateWp(params) => gate_wp(pool, params).await,
        LocusOperation::CloseWp(params) => close_wp(pool, params).await,
        LocusOperation::DeleteWp(params) => delete_wp(pool, params).await,
        LocusOperation::RegisterMts(params) => register_mts(pool, params).await,
        LocusOperation::StartMt(params) => start_mt(pool, params).await,
        LocusOperation::BindSession(params) => bind_session(pool, params).await,
        LocusOperation::UnbindSession(params) => unbind_session(pool, params).await,
        LocusOperation::RecordIteration(params) => record_iteration(pool, params).await,
        LocusOperation::CompleteMt(params) => complete_mt(pool, params).await,
        LocusOperation::GetMtProgress(params) => get_mt_progress(pool, params).await,
        other => Err(StorageError::NotImplemented(match other {
            LocusOperation::AddDependency(_)
            | LocusOperation::RemoveDependency(_)
            | LocusOperation::QueryReady(_)
            | LocusOperation::GetWpStatus(_)
            | LocusOperation::SyncTaskBoard(_) => "locus operation not yet supported for postgres",
            _ => "unsupported locus operation",
        })),
    }
}

pub(crate) async fn locus_work_packet_exists(
    postgres: &PostgresDatabase,
    wp_id: &str,
) -> StorageResult<bool> {
    let exists =
        sqlx::query_scalar::<_, bool>("SELECT EXISTS(SELECT 1 FROM work_packets WHERE wp_id = $1)")
            .bind(wp_id)
            .fetch_one(postgres.pool())
            .await?;
    Ok(exists)
}

pub(crate) async fn locus_task_board_get_status_and_metadata(
    postgres: &PostgresDatabase,
    wp_id: &str,
) -> StorageResult<Option<(String, String)>> {
    sqlx::query_as::<_, (String, String)>(
        "SELECT task_board_status, metadata FROM work_packets WHERE wp_id = $1",
    )
    .bind(wp_id)
    .fetch_optional(postgres.pool())
    .await
    .map_err(StorageError::from)
}

pub(crate) async fn locus_task_board_update_work_packet(
    postgres: &PostgresDatabase,
    status: &str,
    task_board_status: &str,
    updated_at: &str,
    metadata: &str,
    wp_id: &str,
) -> StorageResult<()> {
    sqlx::query(
        r#"
        UPDATE work_packets
        SET
            version = version + 1,
            status = $1,
            task_board_status = $2,
            updated_at = $3,
            metadata = $4
        WHERE wp_id = $5
        "#,
    )
    .bind(canonical_work_packet_status_for_storage(status))
    .bind(task_board_status)
    .bind(updated_at)
    .bind(metadata)
    .bind(wp_id)
    .execute(postgres.pool())
    .await?;
    Ok(())
}

pub(crate) async fn locus_task_board_list_rows(
    postgres: &PostgresDatabase,
) -> StorageResult<Vec<(String, String, String)>> {
    let rows = sqlx::query_as::<_, (String, String, String)>(
        "SELECT wp_id, task_board_status, metadata FROM work_packets",
    )
    .fetch_all(postgres.pool())
    .await?;
    Ok(rows)
}

impl PostgresDatabase {
    pub async fn connect(db_url: &str, max_connections: u32) -> StorageResult<Self> {
        let guard: Arc<dyn StorageGuard> = Arc::new(DefaultStorageGuard);
        Self::connect_with_guard(db_url, max_connections, guard).await
    }

    pub async fn connect_with_guard(
        db_url: &str,
        max_connections: u32,
        guard: Arc<dyn StorageGuard>,
    ) -> StorageResult<Self> {
        let pool = PgPoolOptions::new()
            .max_connections(max_connections)
            .connect(db_url)
            .await?;
        Ok(Self { pool, guard })
    }

    pub fn new(pool: PgPool) -> Self {
        Self {
            pool,
            guard: Arc::new(DefaultStorageGuard),
        }
    }

    pub fn into_arc(self) -> Arc<dyn super::Database> {
        Arc::new(self)
    }

    async fn ensure_model_session_schema(&self) -> StorageResult<()> {
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS model_sessions (
                session_id TEXT PRIMARY KEY,
                parent_session_id TEXT,
                spawn_depth INTEGER NOT NULL DEFAULT 0,
                state TEXT NOT NULL,
                model_id TEXT NOT NULL,
                backend TEXT NOT NULL,
                parameter_class TEXT NOT NULL,
                role TEXT NOT NULL,
                wp_id TEXT,
                mt_id TEXT,
                work_profile_id TEXT,
                execution_mode TEXT NOT NULL,
                memory_policy TEXT NOT NULL,
                consent_receipt_id TEXT,
                capability_grants TEXT NOT NULL DEFAULT '[]',
                capability_token_ids TEXT,
                job_id TEXT,
                checkpoint_artifact_id TEXT,
                last_checkpoint_at TIMESTAMPTZ,
                checkpoint_count INTEGER NOT NULL DEFAULT 0,
                merge_back_artifact TEXT,
                created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
            )
            "#,
        )
        .execute(&self.pool)
        .await?;
        sqlx::query("ALTER TABLE model_sessions ADD COLUMN IF NOT EXISTS merge_back_artifact TEXT")
            .execute(&self.pool)
            .await?;

        sqlx::query(
            "CREATE UNIQUE INDEX IF NOT EXISTS idx_model_sessions_job_id ON model_sessions(job_id)",
        )
        .execute(&self.pool)
        .await?;
        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_model_sessions_parent ON model_sessions(parent_session_id)",
        )
        .execute(&self.pool)
        .await?;
        sqlx::query(
            "ALTER TABLE model_sessions ADD COLUMN IF NOT EXISTS checkpoint_artifact_id TEXT",
        )
        .execute(&self.pool)
        .await?;
        sqlx::query(
            "ALTER TABLE model_sessions ADD COLUMN IF NOT EXISTS last_checkpoint_at TIMESTAMPTZ",
        )
        .execute(&self.pool)
        .await?;
        sqlx::query(
            "ALTER TABLE model_sessions ADD COLUMN IF NOT EXISTS checkpoint_count INTEGER NOT NULL DEFAULT 0",
        )
        .execute(&self.pool)
        .await?;
        // MT-142: durable session identity (agent, purpose) + close metadata
        // (close_reason, closed_by_actor, closed_at) surviving restart.
        for statement in [
            "ALTER TABLE model_sessions ADD COLUMN IF NOT EXISTS agent TEXT",
            "ALTER TABLE model_sessions ADD COLUMN IF NOT EXISTS purpose TEXT",
            "ALTER TABLE model_sessions ADD COLUMN IF NOT EXISTS close_reason TEXT",
            "ALTER TABLE model_sessions ADD COLUMN IF NOT EXISTS closed_by_actor TEXT",
            "ALTER TABLE model_sessions ADD COLUMN IF NOT EXISTS closed_at TIMESTAMPTZ",
        ] {
            sqlx::query(statement).execute(&self.pool).await?;
        }

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS session_checkpoints (
                checkpoint_id TEXT PRIMARY KEY,
                session_id TEXT NOT NULL REFERENCES model_sessions(session_id) ON DELETE CASCADE,
                timestamp TIMESTAMPTZ NOT NULL,
                session_state_json TEXT NOT NULL,
                message_thread_tail_id TEXT NOT NULL,
                pending_tool_calls_json TEXT NOT NULL,
                checkpoint_artifact_id TEXT NOT NULL
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS session_messages (
                message_id TEXT PRIMARY KEY,
                session_id TEXT NOT NULL REFERENCES model_sessions(session_id) ON DELETE CASCADE,
                role TEXT NOT NULL,
                content_hash TEXT NOT NULL,
                content_artifact_id TEXT NOT NULL,
                token_count INTEGER,
                redacted BOOLEAN NOT NULL DEFAULT FALSE,
                tool_call_id TEXT,
                attachments TEXT NOT NULL DEFAULT '[]',
                created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_session_messages_session_created ON session_messages(session_id, created_at)",
        )
        .execute(&self.pool)
        .await?;

        // Deterministic runtime schema upgrades for existing installs.
        sqlx::query("ALTER TABLE session_messages ADD COLUMN IF NOT EXISTS token_count INTEGER")
            .execute(&self.pool)
            .await?;
        sqlx::query(
            "ALTER TABLE session_messages ADD COLUMN IF NOT EXISTS redacted BOOLEAN NOT NULL DEFAULT FALSE",
        )
        .execute(&self.pool)
        .await?;
        sqlx::query("ALTER TABLE session_messages ADD COLUMN IF NOT EXISTS tool_call_id TEXT")
            .execute(&self.pool)
            .await?;
        sqlx::query(
            "ALTER TABLE session_messages ADD COLUMN IF NOT EXISTS attachments TEXT NOT NULL DEFAULT '[]'",
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }
}

fn map_workspace(row: PgRow) -> Workspace {
    Workspace {
        id: row.get("id"),
        name: row.get("name"),
        created_at: map_timestamp(&row, "created_at"),
        updated_at: map_timestamp(&row, "updated_at"),
    }
}

fn map_document(row: PgRow) -> Document {
    Document {
        id: row.get("id"),
        workspace_id: row.get("workspace_id"),
        title: row.get("title"),
        created_at: map_timestamp(&row, "created_at"),
        updated_at: map_timestamp(&row, "updated_at"),
    }
}

fn map_canvas(row: PgRow) -> Canvas {
    Canvas {
        id: row.get("id"),
        workspace_id: row.get("workspace_id"),
        title: row.get("title"),
        created_at: map_timestamp(&row, "created_at"),
        updated_at: map_timestamp(&row, "updated_at"),
    }
}

fn map_canvas_edge(row: PgRow) -> CanvasEdge {
    CanvasEdge {
        id: row.get("id"),
        canvas_id: row.get("canvas_id"),
        from_node_id: row.get("from_node_id"),
        to_node_id: row.get("to_node_id"),
        kind: row.get("kind"),
        created_at: map_timestamp(&row, "created_at"),
        updated_at: map_timestamp(&row, "updated_at"),
    }
}

fn map_block(row: PgRow) -> StorageResult<Block> {
    let derived_raw: String = row.get("derived_content");
    let derived = serde_json::from_str(&derived_raw)?;
    let exportable_int: Option<i32> = row.get("exportable");
    Ok(Block {
        id: row.get("id"),
        document_id: row.get("document_id"),
        kind: row.get("kind"),
        sequence: map_i64_from_i32(&row, "sequence"),
        raw_content: row.get("raw_content"),
        display_content: row.get("display_content"),
        derived_content: derived,
        created_at: map_timestamp(&row, "created_at"),
        updated_at: map_timestamp(&row, "updated_at"),
        sensitivity: row.get("sensitivity"),
        exportable: exportable_int.map(|v| v != 0),
    })
}

fn map_canvas_node(row: PgRow) -> StorageResult<CanvasNode> {
    let data_raw: String = row.get("data");
    let data = serde_json::from_str(&data_raw)?;
    Ok(CanvasNode {
        id: row.get("id"),
        canvas_id: row.get("canvas_id"),
        kind: row.get("kind"),
        position_x: map_f64_from_f32(&row, "position_x"),
        position_y: map_f64_from_f32(&row, "position_y"),
        data,
        created_at: map_timestamp(&row, "created_at"),
        updated_at: map_timestamp(&row, "updated_at"),
    })
}

fn map_asset(row: PgRow) -> Asset {
    let exportable_int: i32 = row.get("exportable");
    let width: Option<i32> = row.get("width");
    let height: Option<i32> = row.get("height");
    Asset {
        asset_id: row.get("asset_id"),
        workspace_id: row.get("workspace_id"),
        kind: row.get("kind"),
        mime: row.get("mime"),
        original_filename: row.get("original_filename"),
        content_hash: row.get("content_hash"),
        size_bytes: row.get("size_bytes"),
        width: width.map(|v| v as i64),
        height: height.map(|v| v as i64),
        created_at: map_timestamp(&row, "created_at"),
        classification: row.get("classification"),
        exportable: exportable_int != 0,
        is_proxy_of: row.get("is_proxy_of"),
        proxy_asset_id: row.get("proxy_asset_id"),
    }
}

fn map_media_tier(row: PgRow) -> StorageResult<MediaAssetTier> {
    let tier = MediaTier::from_str(row.get::<String, _>("tier").as_str())?;
    let status = MediaTierStatus::from_str(row.get::<String, _>("status").as_str())?;
    Ok(MediaAssetTier {
        tier_row_id: row.get("tier_row_id"),
        workspace_id: row.get("workspace_id"),
        asset_id: row.get("asset_id"),
        tier,
        status,
        tier_asset_id: row.get("tier_asset_id"),
        content_hash: row.get("content_hash"),
        failure_reason: row.get("failure_reason"),
        attempt_count: row.get("attempt_count"),
        created_at: map_timestamp(&row, "created_at"),
        updated_at: map_timestamp(&row, "updated_at"),
    })
}

fn map_loom_collection(row: PgRow) -> LoomCollection {
    LoomCollection {
        collection_id: row.get("collection_id"),
        workspace_id: row.get("workspace_id"),
        title: row.get("title"),
        created_at: map_timestamp(&row, "created_at"),
        updated_at: map_timestamp(&row, "updated_at"),
    }
}

fn map_loom_block(row: PgRow) -> StorageResult<LoomBlock> {
    let derived_raw: String = row.get("derived_json");
    let mut derived: LoomBlockDerived = serde_json::from_str(&derived_raw).unwrap_or_default();

    let content_type =
        LoomBlockContentType::from_str(row.get::<String, _>("content_type").as_str())?;
    let preview_status = PreviewStatus::from_str(row.get::<String, _>("preview_status").as_str())?;

    let pinned_int: i32 = row.get("pinned");
    let favorite_int: i32 = row.get("favorite");
    let backlink_count: i32 = row.get("backlink_count");
    let mention_count: i32 = row.get("mention_count");
    let tag_count: i32 = row.get("tag_count");
    let thumbnail_asset_id: Option<String> = row.get("thumbnail_asset_id");
    let proxy_asset_id: Option<String> = row.get("proxy_asset_id");

    derived.backlink_count = backlink_count as i64;
    derived.mention_count = mention_count as i64;
    derived.tag_count = tag_count as i64;
    derived.preview_status = preview_status;
    derived.thumbnail_asset_id = thumbnail_asset_id.clone();
    derived.proxy_asset_id = proxy_asset_id.clone();

    Ok(LoomBlock {
        block_id: row.get("block_id"),
        workspace_id: row.get("workspace_id"),
        content_type,
        document_id: row.get("document_id"),
        asset_id: row.get("asset_id"),
        title: row.get("title"),
        original_filename: row.get("original_filename"),
        content_hash: row.get("content_hash"),
        pinned: pinned_int != 0,
        favorite: favorite_int != 0,
        // MT-183: pin_order is tolerant — SELECTs that do not project it (most)
        // yield None; only the Pins view + single-block reads populate it.
        pin_order: row.try_get::<Option<i32>, _>("pin_order").ok().flatten(),
        journal_date: row.get("journal_date"),
        created_at: map_timestamp(&row, "created_at"),
        updated_at: map_timestamp(&row, "updated_at"),
        imported_at: map_optional_timestamp(&row, "imported_at"),
        derived,
    })
}

fn loom_debug_trim(value: &str, max_chars: usize) -> String {
    let trimmed = value.trim();
    let mut out: String = trimmed.chars().take(max_chars).collect();
    if trimmed.chars().count() > max_chars {
        out.push_str("...");
    }
    out
}

fn loom_debug_block_label(block: &LoomBlock) -> String {
    block
        .title
        .as_deref()
        .or(block.original_filename.as_deref())
        .map(|value| loom_debug_trim(value, 120))
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| format!("{} {}", block.content_type.as_str(), block.block_id))
}

fn loom_visual_debug_route_ids() -> Vec<String> {
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
    .map(str::to_string)
    .collect()
}

fn loom_visual_debug_node_summary(node: super::LoomGraphNode) -> LoomVisualDebugGraphNodeSummary {
    let title = loom_debug_block_label(&node.block);
    LoomVisualDebugGraphNodeSummary {
        block_id: node.block.block_id,
        title,
        content_type: node.block.content_type,
        depth: node.depth,
        degree: node.degree,
        stale: node.stale,
        entity_id: node.entity_id,
    }
}

fn loom_visual_debug_edge_summary(edge: super::LoomGraphEdge) -> LoomVisualDebugGraphEdgeSummary {
    LoomVisualDebugGraphEdgeSummary {
        edge_id: edge.edge.edge_id,
        source_block_id: edge.edge.source_block_id,
        target_block_id: edge.edge.target_block_id,
        edge_type: edge.edge.edge_type,
        stale: edge.stale,
    }
}

fn loom_visual_debug_backlink_summary(
    backlink: super::LoomBacklink,
) -> LoomVisualDebugBacklinkSummary {
    LoomVisualDebugBacklinkSummary {
        edge_id: backlink.edge.edge_id,
        source_block_id: backlink.source_block.block_id,
        edge_type: backlink.edge.edge_type,
        context_snippet: backlink
            .context_snippet
            .map(|snippet| loom_debug_trim(&snippet, 160)),
    }
}

fn loom_visual_debug_folder_summary(
    folder: LoomFolder,
    member_count: i64,
    sample_block_ids: Vec<String>,
) -> LoomVisualDebugFolderSummary {
    LoomVisualDebugFolderSummary {
        folder_id: folder.folder_id,
        parent_folder_id: folder.parent_folder_id,
        name: loom_debug_trim(&folder.name, 120),
        color: folder.color,
        sort_mode: folder.sort_mode,
        project_ref: folder.project_ref,
        member_count,
        sample_block_ids,
    }
}

fn loom_visual_debug_search_summary(hit: LoomGraphSearchResult) -> LoomVisualDebugSearchHitSummary {
    let authority_table = hit
        .metadata
        .get("authority_table")
        .and_then(|value| value.as_str())
        .unwrap_or("unknown")
        .to_string();
    let retrieval_bias_schema_id = hit
        .metadata
        .get("retrieval_bias_schema_id")
        .and_then(Value::as_str)
        .map(str::to_string);
    let retrieval_bias_score = hit
        .metadata
        .get("retrieval_bias_score")
        .and_then(Value::as_f64);
    let retrieval_bias_reasons = hit
        .metadata
        .get("retrieval_bias_reasons")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    LoomVisualDebugSearchHitSummary {
        result_kind: hit.result_kind,
        source_kind: hit.source_kind,
        ref_id: hit.ref_id,
        title: loom_debug_trim(&hit.title, 120),
        excerpt: loom_debug_trim(&hit.excerpt, 160),
        authority_table,
        retrieval_bias_schema_id,
        retrieval_bias_score,
        retrieval_bias_reasons,
    }
}

/// MT-177: extractor version stamped onto the bridge knowledge entity's
/// detection provenance and the EventLedger receipt payload. Bump when the
/// bridge derivation changes so a re-index is attributable.
const LOOM_KNOWLEDGE_BRIDGE_EXTRACTOR_VERSION: &str = "loom_block_knowledge_bridge_v1";

/// MT-177: map a `loom_block_knowledge_bridge` row (TIMESTAMPTZ columns).
fn map_loom_knowledge_bridge(row: &PgRow) -> super::LoomKnowledgeBridge {
    super::LoomKnowledgeBridge {
        block_id: row.get("block_id"),
        workspace_id: row.get("workspace_id"),
        entity_id: row.get("entity_id"),
        index_event_id: row.get("index_event_id"),
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
    }
}

fn map_quick_switcher_recent(row: &PgRow) -> StorageResult<QuickSwitcherRecent> {
    let source_kind = row
        .get::<String, _>("source_kind")
        .parse::<LoomSearchSourceKind>()?;
    let result_kind = row
        .get::<String, _>("result_kind")
        .parse::<LoomSearchResultKind>()?;
    Ok(QuickSwitcherRecent {
        workspace_id: row.get("workspace_id"),
        hit_key: row.get("hit_key"),
        source_kind,
        ref_id: row.get("ref_id"),
        result_kind,
        title: row.get("title"),
        excerpt: row.get("excerpt"),
        metadata: row.get("metadata"),
        selected_count: row.get("selected_count"),
        selected_at: row.get("selected_at"),
        event_ledger_event_id: row.get("event_ledger_event_id"),
    })
}

fn map_workbench_layout_state(row: &PgRow) -> StorageResult<WorkbenchLayoutState> {
    Ok(WorkbenchLayoutState {
        workspace_id: row.get("workspace_id"),
        layout_state: row.get("layout_state"),
        updated_at: row.get("updated_at"),
        event_ledger_event_id: row.get("event_ledger_event_id"),
    })
}

/// Validate the MT-261 canvas board-state (viewport) JSONB shape. Requires the
/// schema id plus numeric pan_x/pan_y and a positive finite zoom so the DB CHECK
/// (schema id) and the application invariants stay aligned.
fn validate_loom_canvas_board_state(board_state: &Value) -> StorageResult<()> {
    let Some(obj) = board_state.as_object() else {
        return Err(StorageError::Validation(
            "loom canvas board_state must be a JSON object",
        ));
    };
    if obj.get("schema_id").and_then(Value::as_str) != Some(LOOM_CANVAS_BOARD_SCHEMA_ID) {
        return Err(StorageError::Validation(
            "loom canvas board_state schema_id must be hsk.loom_canvas_board@1",
        ));
    }
    let num = |key: &str| obj.get(key).and_then(Value::as_f64);
    let pan_x = num("pan_x");
    let pan_y = num("pan_y");
    let zoom = num("zoom");
    let (Some(pan_x), Some(pan_y), Some(zoom)) = (pan_x, pan_y, zoom) else {
        return Err(StorageError::Validation(
            "loom canvas board_state requires numeric pan_x, pan_y, zoom",
        ));
    };
    if !pan_x.is_finite() || !pan_y.is_finite() || !zoom.is_finite() || zoom <= 0.0 {
        return Err(StorageError::Validation(
            "loom canvas board_state pan/zoom must be finite and zoom > 0",
        ));
    }
    Ok(())
}

fn map_loom_canvas_board(row: &PgRow) -> StorageResult<LoomCanvasBoard> {
    Ok(LoomCanvasBoard {
        block_id: row.get("block_id"),
        workspace_id: row.get("workspace_id"),
        board_state: row.get("board_state"),
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
        event_ledger_event_id: row.get("event_ledger_event_id"),
    })
}

fn map_loom_canvas_placement(row: &PgRow) -> StorageResult<LoomCanvasPlacement> {
    Ok(LoomCanvasPlacement {
        placement_id: row.get("placement_id"),
        canvas_block_id: row.get("canvas_block_id"),
        workspace_id: row.get("workspace_id"),
        placed_block_id: row.get("placed_block_id"),
        x: row.get("x"),
        y: row.get("y"),
        w: row.get("w"),
        h: row.get("h"),
        z_index: row.get("z_index"),
        group_id: row.get("group_id"),
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
    })
}

fn map_loom_canvas_visual_edge(row: &PgRow) -> StorageResult<LoomCanvasVisualEdge> {
    Ok(LoomCanvasVisualEdge {
        visual_edge_id: row.get("visual_edge_id"),
        canvas_block_id: row.get("canvas_block_id"),
        workspace_id: row.get("workspace_id"),
        from_placement_id: row.get("from_placement_id"),
        to_placement_id: row.get("to_placement_id"),
        label: row.get("label"),
        created_at: row.get("created_at"),
    })
}

fn map_workspace_settings_state(row: &PgRow) -> StorageResult<WorkspaceSettingsState> {
    Ok(WorkspaceSettingsState {
        workspace_id: row.get("workspace_id"),
        settings_state: row.get("settings_state"),
        updated_at: row.get("updated_at"),
        event_ledger_event_id: row.get("event_ledger_event_id"),
    })
}

fn map_workspace_search_bookmark_state(row: &PgRow) -> StorageResult<WorkspaceSearchBookmarkState> {
    Ok(WorkspaceSearchBookmarkState {
        workspace_id: row.get("workspace_id"),
        bookmark_state: row.get("bookmark_state"),
        updated_at: row.get("updated_at"),
        event_ledger_event_id: row.get("event_ledger_event_id"),
    })
}

fn map_debug_breakpoint(row: &PgRow) -> StorageResult<DebugBreakpoint> {
    Ok(DebugBreakpoint {
        breakpoint_id: row.get("breakpoint_id"),
        rich_document_id: row.get("rich_document_id"),
        workspace_id: row.get("workspace_id"),
        source_url: row.get("source_url"),
        line: row.get("line"),
        condition: row.get("condition"),
        verified: row.get("verified"),
        updated_at: row.get("updated_at"),
        event_ledger_event_id: row.get("event_ledger_event_id"),
    })
}

const WORKSPACE_SEARCH_BOOKMARK_SHAPE_VALIDATION_ERROR: &str =
    "workspace search bookmark_state must match hsk.workspace_search_bookmark_state@1 shape";
const WORKSPACE_SEARCH_BOOKMARK_MAX: usize = 100;

/// Validates the saved-search bookmark blob shape: a JSON object carrying the
/// schema id and a bounded `bookmarks` array of well-formed saved-search records.
/// The shape mirrors the frontend `WorkspaceSearchBookmark` so the projection
/// (UI) can re-hydrate searches losslessly from canonical state.
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
            "workspace search bookmark_state exceeds 100 saved searches",
        ));
    }
    for bookmark in bookmarks {
        let Some(entry) = bookmark.as_object() else {
            return Err(StorageError::Validation(
                WORKSPACE_SEARCH_BOOKMARK_SHAPE_VALIDATION_ERROR,
            ));
        };
        let require_str = |key: &str| -> StorageResult<()> {
            match entry.get(key).and_then(Value::as_str) {
                Some(_) => Ok(()),
                None => Err(StorageError::Validation(
                    WORKSPACE_SEARCH_BOOKMARK_SHAPE_VALIDATION_ERROR,
                )),
            }
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
        require_str("tagFilter")?;
        require_str("pathFilter")?;
        require_bool("caseSensitive")?;
        require_bool("wholeWord")?;
        require_bool("isRegex")?;
        require_nonempty_str("savedAt")?;
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
    match value.and_then(Value::as_str) {
        Some(candidate) => allowed.contains(&candidate),
        None => false,
    }
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
    match value.and_then(Value::as_f64) {
        Some(candidate) => (0.2..=0.8).contains(&candidate),
        None => false,
    }
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
        let valid_document_id = document
            .get("documentId")
            .and_then(Value::as_str)
            .is_some_and(|document_id| !document_id.trim().is_empty());
        valid_document_id
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
        key.to_string()
    };
    let mut normalized = Vec::new();
    for modifier in ["Mod", "Alt", "Shift"] {
        if modifiers.contains(modifier) {
            normalized.push(modifier.to_string());
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

/// MT-184: deterministic wiki markdown for a set of LoomBlocks. Pure function
/// of (title, blocks) so the same authority renders byte-identically. Each
/// block becomes a cited section: its title (or a content-type fallback), its
/// content type, and its block-id citation. NEVER includes absolute paths.
fn render_loom_wiki_markdown(title: &str, blocks: &[LoomBlock]) -> String {
    let mut out = String::new();
    out.push_str("# ");
    out.push_str(title.trim());
    out.push('\n');
    if blocks.is_empty() {
        out.push_str("\n_No source blocks._\n");
        return out;
    }
    for block in blocks {
        let label = block
            .title
            .as_deref()
            .map(str::trim)
            .filter(|v| !v.is_empty())
            .map(|v| v.to_string())
            .unwrap_or_else(|| format!("{} {}", block.content_type.as_str(), block.block_id));
        out.push_str("\n## ");
        out.push_str(&label);
        out.push('\n');
        out.push_str("- type: ");
        out.push_str(block.content_type.as_str());
        out.push('\n');
        // Citation: the source LoomBlock id (block-as-unit-of-meaning).
        out.push_str("- cite: loom_block:");
        out.push_str(&block.block_id);
        out.push('\n');
        if let Some(text) = block
            .derived
            .full_text_index
            .as_deref()
            .map(str::trim)
            .filter(|v| !v.is_empty())
        {
            out.push('\n');
            out.push_str(text);
            out.push('\n');
        }
    }
    out
}

/// MT-184: staleness hash over the source blocks' identity + content + last
/// update. A change to any source block (or its set) flips the hash, marking
/// the projection stale. Deterministic and order-sensitive.
fn loom_wiki_staleness_hash(blocks: &[LoomBlock]) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(b"loom_wiki_projection_v1");
    for block in blocks {
        hasher.update(b"|id:");
        hasher.update(block.block_id.as_bytes());
        hasher.update(b"|t:");
        hasher.update(block.title.as_deref().unwrap_or("").as_bytes());
        hasher.update(b"|ct:");
        hasher.update(block.content_type.as_str().as_bytes());
        hasher.update(b"|ft:");
        hasher.update(
            block
                .derived
                .full_text_index
                .as_deref()
                .unwrap_or("")
                .as_bytes(),
        );
        hasher.update(b"|u:");
        hasher.update(block.updated_at.to_rfc3339().as_bytes());
    }
    format!("{:x}", hasher.finalize())
}

/// MT-184: convert a stored KnowledgeWikiProjection into the Loom view type
/// (lifts the source LoomBlock ids out of the source_records JSON).
fn loom_wiki_projection_from_knowledge(
    p: crate::storage::knowledge::KnowledgeWikiProjection,
) -> super::LoomWikiProjection {
    let source_block_ids = p
        .source_records
        .as_array()
        .map(|arr| {
            arr.iter()
                // Typed project-wiki pages (MT-241) cite entities/sources/
                // documents; only genuine LoomBlock citations are block ids.
                .filter(|v| v.get("record_family").and_then(|f| f.as_str()) == Some("LoomBlock"))
                .filter_map(|v| v.get("record_id").and_then(|r| r.as_str()))
                .map(|s| s.to_string())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    super::LoomWikiProjection {
        projection_id: p.projection_id,
        workspace_id: p.workspace_id,
        title: p.title,
        source_block_ids,
        rendered_content: p.rendered_content,
        staleness_hash: p.staleness_hash,
        rebuild_status: p.rebuild_status.as_str().to_string(),
        page_type: p.page_type,
        compile_stamp: p.compile_stamp,
        page_links: p.page_links,
        created_at: p.created_at,
        updated_at: p.updated_at,
    }
}

/// MT-185: map a `loom_wiki_overlays` row.
fn map_loom_wiki_overlay(row: &PgRow) -> super::LoomWikiOverlay {
    super::LoomWikiOverlay {
        overlay_id: row.get("overlay_id"),
        projection_id: row.get("projection_id"),
        workspace_id: row.get("workspace_id"),
        annotation: row.get("annotation"),
        anchor: row.get("anchor"),
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
    }
}

/// MT-181: map a `loom_folders` row.
fn map_loom_folder(row: &PgRow) -> StorageResult<super::LoomFolder> {
    Ok(super::LoomFolder {
        folder_id: row.get("folder_id"),
        workspace_id: row.get("workspace_id"),
        parent_folder_id: row.get("parent_folder_id"),
        name: row.get("name"),
        color: row.get("color"),
        sort_mode: row.get::<String, _>("sort_mode").parse()?,
        sort_order: row.get("sort_order"),
        project_ref: row.get("project_ref"),
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
    })
}

/// MT-178: the human-readable text of a LoomBlock used for backlink snippets
/// and unlinked-mention scanning. Combines the title and the derived
/// full_text_index (the same text `search_loom_blocks` matches), title first so
/// a snippet leads with the most salient text. Never includes ids or paths.
fn loom_block_scan_text(block: &LoomBlock) -> String {
    let mut parts: Vec<&str> = Vec::new();
    if let Some(title) = block
        .title
        .as_deref()
        .map(str::trim)
        .filter(|v| !v.is_empty())
    {
        parts.push(title);
    }
    if let Some(text) = block
        .derived
        .full_text_index
        .as_deref()
        .map(str::trim)
        .filter(|v| !v.is_empty())
    {
        parts.push(text);
    }
    parts.join("\n")
}

/// MT-177: pick the EventLedger actor for a bridge receipt from the write
/// context's actor kind. The bridge is normally a system-internal indexing
/// step, but an operator- or AI-initiated bridge is attributed accordingly so
/// the receipt's actor matches who triggered it.
fn kernel_actor_for_bridge(ctx: &WriteContext) -> KernelActor {
    let actor_id = ctx
        .actor_id
        .clone()
        .unwrap_or_else(|| "loom_block_knowledge_bridge".to_string());
    match ctx.actor_kind {
        super::WriteActorKind::Human => KernelActor::Operator(actor_id),
        super::WriteActorKind::Ai => KernelActor::ModelAdapter(actor_id),
        super::WriteActorKind::System => KernelActor::System(actor_id),
    }
}

/// MT-177: a kernel-event builder failure is a programmer/data error, surfaced
/// as a typed validation error with a stable, leak-free code.
fn kernel_event_build_error(err: crate::kernel::KernelError) -> &'static str {
    tracing::error!(target: "handshake_core", error = %err, "loom_bridge_event_build_failed");
    "loom bridge EventLedger receipt build failed"
}

fn map_loom_edge(row: PgRow) -> StorageResult<LoomEdge> {
    let edge_type = LoomEdgeType::from_str(row.get::<String, _>("edge_type").as_str())?;
    let created_by = LoomEdgeCreatedBy::from_str(row.get::<String, _>("created_by").as_str())?;

    let source_document_id: Option<String> = row.get("source_document_id");
    let source_text_block_id: Option<String> = row.get("source_text_block_id");
    let offset_start: Option<i32> = row.get("offset_start");
    let offset_end: Option<i32> = row.get("offset_end");
    let source_anchor = match (
        source_document_id,
        source_text_block_id,
        offset_start,
        offset_end,
    ) {
        (Some(document_id), Some(block_id), Some(offset_start), Some(offset_end)) => {
            Some(LoomSourceAnchor {
                document_id,
                block_id,
                offset_start: offset_start as i64,
                offset_end: offset_end as i64,
            })
        }
        _ => None,
    };

    Ok(LoomEdge {
        edge_id: row.get("edge_id"),
        workspace_id: row.get("workspace_id"),
        source_block_id: row.get("source_block_id"),
        target_block_id: row.get("target_block_id"),
        edge_type,
        created_by,
        created_at: map_timestamp(&row, "created_at"),
        crdt_site_id: row.get("crdt_site_id"),
        source_anchor,
    })
}

fn normalize_loom_search_tokens(raw: &str) -> Vec<String> {
    raw.split_whitespace()
        .map(|token| token.trim_matches('"').trim())
        .filter(|token| !token.is_empty())
        .map(|token| token.to_string())
        .collect()
}

fn escape_like_token(token: &str) -> String {
    token
        .replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_")
}

fn escape_postgres_regex_literal(token: &str) -> String {
    let mut escaped = String::new();
    for ch in token.chars() {
        if matches!(
            ch,
            '\\' | '.' | '^' | '$' | '*' | '+' | '?' | '(' | ')' | '[' | ']' | '{' | '}' | '|'
        ) {
            escaped.push('\\');
        }
        escaped.push(ch);
    }
    escaped
}

fn loom_search_uses_exact_sql(filters: &LoomSearchFilters) -> bool {
    filters.case_sensitive || filters.whole_word || filters.is_regex
}

fn loom_search_regex_pattern(query: &str, filters: &LoomSearchFilters) -> String {
    if filters.is_regex {
        query.trim().to_string()
    } else if filters.whole_word {
        format!(
            "(^|[^[:alnum:]_]){}([^[:alnum:]_]|$)",
            escape_postgres_regex_literal(query.trim())
        )
    } else {
        escape_postgres_regex_literal(query.trim())
    }
}

fn push_loom_exact_text_match(
    qb: &mut QueryBuilder<Postgres>,
    expressions: &[&str],
    query: &str,
    filters: &LoomSearchFilters,
) {
    let query = query.trim();
    if query.is_empty() {
        return;
    }
    qb.push(" AND (");
    if filters.is_regex || filters.whole_word {
        let pattern = loom_search_regex_pattern(query, filters);
        for (index, expression) in expressions.iter().enumerate() {
            if index > 0 {
                qb.push(" OR ");
            }
            qb.push(*expression)
                .push(if filters.case_sensitive {
                    " ~ "
                } else {
                    " ~* "
                })
                .push_bind(pattern.clone());
        }
    } else {
        let pattern = format!("%{}%", escape_like_token(query));
        for (index, expression) in expressions.iter().enumerate() {
            if index > 0 {
                qb.push(" OR ");
            }
            qb.push(*expression)
                .push(if filters.case_sensitive {
                    " LIKE "
                } else {
                    " ILIKE "
                })
                .push_bind(pattern.clone())
                .push(" ESCAPE '\\'");
        }
    }
    qb.push(")");
}

fn push_loom_path_filter(
    qb: &mut QueryBuilder<Postgres>,
    expressions: &[&str],
    path: Option<&str>,
) {
    let Some(path) = path.map(str::trim).filter(|value| !value.is_empty()) else {
        return;
    };
    let pattern = format!("%{}%", escape_like_token(path));
    qb.push(" AND (");
    for (index, expression) in expressions.iter().enumerate() {
        if index > 0 {
            qb.push(" OR ");
        }
        qb.push(*expression)
            .push(" ILIKE ")
            .push_bind(pattern.clone())
            .push(" ESCAPE '\\'");
    }
    qb.push(")");
}

fn loom_fuzzy_query(tokens: &[String]) -> Option<String> {
    if tokens.len() != 1 {
        return None;
    }
    let compact: String = tokens[0]
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric())
        .map(|ch| ch.to_ascii_lowercase())
        .collect();
    if (2..=12).contains(&compact.len()) {
        Some(compact)
    } else {
        None
    }
}

fn loom_fuzzy_contains_like_pattern(query: &str) -> String {
    format!("%{}%", escape_like_token(query))
}

fn loom_fuzzy_subsequence_like_pattern(query: &str) -> String {
    let mut pattern = String::from("%");
    for ch in query.chars() {
        pattern.push(ch);
        pattern.push('%');
    }
    pattern
}

const LOOM_FUZZY_TRIGRAM_THRESHOLD: f32 = 0.22;

fn loom_fuzzy_sql_initials_expression(expression: &str) -> String {
    format!(
        r#"lower(regexp_replace(regexp_replace(regexp_replace({expression}, '([[:lower:][:digit:]])([[:upper:]])', E'\\1 \\2', 'g'), '(^|[^[:alnum:]])([[:alnum:]])[[:alnum:]]*', E'\\2', 'g'), '[^[:alnum:]]+', '', 'g'))"#
    )
}

fn push_loom_fuzzy_sql_match(qb: &mut QueryBuilder<Postgres>, expressions: &[&str], query: &str) {
    let contains_pattern = loom_fuzzy_contains_like_pattern(query);
    let subsequence_pattern = loom_fuzzy_subsequence_like_pattern(query);
    let initials_prefix_pattern = format!("{}%", escape_like_token(query));
    qb.push(" AND (");
    for (index, expression) in expressions.iter().enumerate() {
        if index > 0 {
            qb.push(" OR ");
        }
        let normalized_expression = format!("lower({expression})");
        let initials_expression = loom_fuzzy_sql_initials_expression(expression);
        qb.push("(");
        qb.push(normalized_expression.as_str())
            .push(" LIKE ")
            .push_bind(contains_pattern.clone())
            .push(" ESCAPE '\\'");
        if query.len() >= 4 {
            qb.push(" OR ")
                .push(normalized_expression.as_str())
                .push(" LIKE ")
                .push_bind(subsequence_pattern.clone())
                .push(" ESCAPE '\\'");
        }
        qb.push(" OR ")
            .push(normalized_expression.as_str())
            .push(" OPERATOR(public.%) ")
            .push_bind(query.to_string());
        qb.push(" OR public.similarity(")
            .push(normalized_expression.as_str())
            .push(", ")
            .push_bind(query.to_string())
            .push(") >= ")
            .push_bind(LOOM_FUZZY_TRIGRAM_THRESHOLD);
        qb.push(" OR ")
            .push(initials_expression.as_str())
            .push(" LIKE ")
            .push_bind(initials_prefix_pattern.clone())
            .push(" ESCAPE '\\'");
        qb.push(" OR ")
            .push(initials_expression.as_str())
            .push(" LIKE ")
            .push_bind(subsequence_pattern.clone())
            .push(" ESCAPE '\\'");
        qb.push(")");
    }
    qb.push(")");
}

fn is_ascii_subsequence(needle: &str, haystack: &str) -> bool {
    let mut haystack_chars = haystack.chars();
    needle.chars().all(|needle_ch| {
        haystack_chars
            .by_ref()
            .any(|haystack_ch| haystack_ch == needle_ch)
    })
}

fn loom_fuzzy_forms(value: &str) -> (String, String) {
    let mut compact = String::new();
    let mut initials = String::new();
    let mut at_word_start = true;
    let mut previous: Option<char> = None;

    for ch in value.chars() {
        if ch.is_ascii_alphanumeric() {
            let lower = ch.to_ascii_lowercase();
            compact.push(lower);
            let camel_boundary = ch.is_ascii_uppercase()
                && previous
                    .map(|prev| prev.is_ascii_lowercase() || prev.is_ascii_digit())
                    .unwrap_or(false);
            if at_word_start || camel_boundary {
                initials.push(lower);
            }
            at_word_start = false;
        } else {
            at_word_start = true;
        }
        previous = Some(ch);
    }

    (compact, initials)
}

fn loom_typo_max_distance(query_len: usize) -> usize {
    match query_len {
        0..=3 => 0,
        4..=8 => 1,
        9..=16 => 2,
        _ => 3,
    }
}

fn loom_bounded_edit_distance(left: &str, right: &str, max_distance: usize) -> Option<usize> {
    let left_len = left.len();
    let right_len = right.len();
    if left_len.abs_diff(right_len) > max_distance {
        return None;
    }

    let mut previous: Vec<usize> = (0..=right_len).collect();
    let mut current = vec![0; right_len + 1];

    for (left_index, left_byte) in left.bytes().enumerate() {
        current[0] = left_index + 1;
        let mut row_min = current[0];
        for (right_index, right_byte) in right.bytes().enumerate() {
            let substitution = if left_byte == right_byte { 0 } else { 1 };
            let value = (previous[right_index + 1] + 1)
                .min(current[right_index] + 1)
                .min(previous[right_index] + substitution);
            current[right_index + 1] = value;
            row_min = row_min.min(value);
        }
        if row_min > max_distance {
            return None;
        }
        std::mem::swap(&mut previous, &mut current);
    }

    let distance = previous[right_len];
    (distance <= max_distance).then_some(distance)
}

fn loom_typo_field_score(query: &str, compact: &str) -> Option<f64> {
    let max_distance = loom_typo_max_distance(query.len());
    if max_distance == 0 || compact.len() < 4 {
        return None;
    }

    let min_window = query.len().saturating_sub(max_distance).max(1);
    let max_window = (query.len() + max_distance).min(compact.len());
    let mut best: Option<(usize, usize)> = None;

    for window_len in min_window..=max_window {
        for start in 0..=compact.len().saturating_sub(window_len) {
            let candidate = &compact[start..start + window_len];
            if let Some(distance) = loom_bounded_edit_distance(query, candidate, max_distance) {
                let start_penalty = start.min(8);
                let current = (distance, start_penalty);
                if best.map(|best| current < best).unwrap_or(true) {
                    best = Some(current);
                }
            }
        }
    }

    best.map(|(distance, start_penalty)| {
        14.0 - (distance as f64 * 2.0) - (start_penalty as f64 * 0.25)
    })
    .filter(|score| *score > 0.0)
}

fn loom_fuzzy_field_score(query: &str, value: &str) -> Option<f64> {
    if value.trim().is_empty() {
        return None;
    }
    let (compact, initials) = loom_fuzzy_forms(value);
    if initials.starts_with(query) {
        return Some(24.0);
    }
    if is_ascii_subsequence(query, &initials) {
        return Some(20.0);
    }
    if compact.starts_with(query) {
        return Some(12.0);
    }
    if compact.contains(query) {
        return Some(10.0);
    }
    if query.len() >= 4 && is_ascii_subsequence(query, &compact) {
        return Some(6.0);
    }
    loom_typo_field_score(query, &compact)
}

fn loom_fuzzy_score<'a>(query: &str, fields: impl IntoIterator<Item = &'a str>) -> Option<f64> {
    fields
        .into_iter()
        .filter_map(|field| loom_fuzzy_field_score(query, field))
        .max_by(|a, b| a.total_cmp(b))
}

fn loom_search_source_allowed(
    filters: &LoomSearchFilters,
    source_kind: LoomSearchSourceKind,
) -> bool {
    filters.source_kinds.is_empty() || filters.source_kinds.contains(&source_kind)
}

const LOOM_GRAPH_SOURCE_ORDER: [LoomSearchSourceKind; 9] = [
    LoomSearchSourceKind::LoomBlock,
    LoomSearchSourceKind::File,
    LoomSearchSourceKind::TagHub,
    LoomSearchSourceKind::Document,
    LoomSearchSourceKind::Symbol,
    LoomSearchSourceKind::WorkPacket,
    LoomSearchSourceKind::MicroTask,
    LoomSearchSourceKind::UserManualPage,
    LoomSearchSourceKind::WikiPage,
];

fn loom_graph_source_order_index(source_kind: LoomSearchSourceKind) -> usize {
    LOOM_GRAPH_SOURCE_ORDER
        .iter()
        .position(|candidate| *candidate == source_kind)
        .unwrap_or(LOOM_GRAPH_SOURCE_ORDER.len())
}

fn order_loom_graph_results_for_breadth(
    results: Vec<LoomGraphSearchResult>,
) -> Vec<LoomGraphSearchResult> {
    if results.len() <= 1 {
        return results;
    }

    let mut buckets: Vec<VecDeque<LoomGraphSearchResult>> = LOOM_GRAPH_SOURCE_ORDER
        .iter()
        .map(|_| VecDeque::new())
        .collect();
    let mut fallback = VecDeque::new();
    let total = results.len();

    for result in results {
        let index = loom_graph_source_order_index(result.source_kind);
        if let Some(bucket) = buckets.get_mut(index) {
            bucket.push_back(result);
        } else {
            fallback.push_back(result);
        }
    }

    let mut ordered = Vec::with_capacity(total);
    loop {
        let mut progressed = false;
        for bucket in &mut buckets {
            if let Some(result) = bucket.pop_front() {
                ordered.push(result);
                progressed = true;
            }
        }
        if let Some(result) = fallback.pop_front() {
            ordered.push(result);
            progressed = true;
        }
        if !progressed {
            break;
        }
    }

    ordered
}

fn loom_block_graph_source_kind(content_type: &LoomBlockContentType) -> LoomSearchSourceKind {
    match content_type {
        LoomBlockContentType::File | LoomBlockContentType::AnnotatedFile => {
            LoomSearchSourceKind::File
        }
        LoomBlockContentType::TagHub => LoomSearchSourceKind::TagHub,
        LoomBlockContentType::Note
        | LoomBlockContentType::Journal
        | LoomBlockContentType::Canvas => LoomSearchSourceKind::LoomBlock,
    }
}

fn loom_search_block_content_type_filter(filters: &LoomSearchFilters) -> Option<Vec<&'static str>> {
    if filters.source_kinds.is_empty() {
        return None;
    }

    let mut content_types = Vec::new();
    if loom_search_source_allowed(filters, LoomSearchSourceKind::LoomBlock) {
        content_types.push(LoomBlockContentType::Note.as_str());
        content_types.push(LoomBlockContentType::Journal.as_str());
    }
    if loom_search_source_allowed(filters, LoomSearchSourceKind::File) {
        content_types.push(LoomBlockContentType::File.as_str());
        content_types.push(LoomBlockContentType::AnnotatedFile.as_str());
    }
    if loom_search_source_allowed(filters, LoomSearchSourceKind::TagHub) {
        content_types.push(LoomBlockContentType::TagHub.as_str());
    }

    Some(content_types)
}

fn loom_search_has_block_scoped_filters(filters: &LoomSearchFilters) -> bool {
    filters.content_type.is_some()
        || filters.mime.is_some()
        || !filters.tag_ids.is_empty()
        || !filters.mention_ids.is_empty()
        || filters.backlink_depth.is_some()
}

const LOOM_RETRIEVAL_BIAS_SCHEMA_ID: &str = "hsk.loom_retrieval_bias@1";
const LOOM_RETRIEVAL_BIAS_PINNED_WEIGHT: f64 = 5.0;
const LOOM_RETRIEVAL_BIAS_FAVORITE_WEIGHT: f64 = 3.0;
const LOOM_RETRIEVAL_BIAS_TAG_WEIGHT: f64 = 1.5;
const LOOM_RETRIEVAL_BIAS_BACKLINK_WEIGHT: f64 = 1.0;
const LOOM_RETRIEVAL_BIAS_COUNT_CAP: i64 = 10;

fn loom_retrieval_bias_count(value: i64) -> i64 {
    value.max(0).min(LOOM_RETRIEVAL_BIAS_COUNT_CAP)
}

fn loom_retrieval_bias_score(block: &LoomBlock) -> f64 {
    let mut score = 0.0;
    if block.pinned {
        score += LOOM_RETRIEVAL_BIAS_PINNED_WEIGHT;
    }
    if block.favorite {
        score += LOOM_RETRIEVAL_BIAS_FAVORITE_WEIGHT;
    }
    score +=
        loom_retrieval_bias_count(block.derived.tag_count) as f64 * LOOM_RETRIEVAL_BIAS_TAG_WEIGHT;
    score += loom_retrieval_bias_count(block.derived.backlink_count) as f64
        * LOOM_RETRIEVAL_BIAS_BACKLINK_WEIGHT;
    score
}

fn loom_retrieval_bias_reasons(block: &LoomBlock) -> Vec<Value> {
    let mut reasons = Vec::new();
    if block.pinned {
        reasons.push(json!({
            "code": "pinned",
            "label": "Pinned Loom block",
            "weight": LOOM_RETRIEVAL_BIAS_PINNED_WEIGHT,
            "evidence_ref": "loom_blocks.pinned",
        }));
    }
    if block.favorite {
        reasons.push(json!({
            "code": "favorite",
            "label": "Favorite Loom block",
            "weight": LOOM_RETRIEVAL_BIAS_FAVORITE_WEIGHT,
            "evidence_ref": "loom_blocks.favorite",
        }));
    }
    let raw_tag_count = block.derived.tag_count.max(0);
    let score_tag_count = loom_retrieval_bias_count(raw_tag_count);
    if raw_tag_count > 0 {
        reasons.push(json!({
            "code": "tagged",
            "label": "Tagged Loom block",
            "weight": score_tag_count as f64 * LOOM_RETRIEVAL_BIAS_TAG_WEIGHT,
            "evidence_ref": "loom_blocks.tag_count",
            "count": raw_tag_count,
            "score_count": score_tag_count,
            "score_count_cap": LOOM_RETRIEVAL_BIAS_COUNT_CAP,
        }));
    }
    let raw_backlink_count = block.derived.backlink_count.max(0);
    let score_backlink_count = loom_retrieval_bias_count(raw_backlink_count);
    if raw_backlink_count > 0 {
        reasons.push(json!({
            "code": "backlinked",
            "label": "Backlinked Loom block",
            "weight": score_backlink_count as f64 * LOOM_RETRIEVAL_BIAS_BACKLINK_WEIGHT,
            "evidence_ref": "loom_blocks.backlink_count",
            "count": raw_backlink_count,
            "score_count": score_backlink_count,
            "score_count_cap": LOOM_RETRIEVAL_BIAS_COUNT_CAP,
        }));
    }
    reasons
}

fn loom_retrieval_bias_metadata(block: &LoomBlock, score: f64) -> Value {
    json!({
        "authority_table": "loom_blocks",
        "retrieval_bias_schema_id": LOOM_RETRIEVAL_BIAS_SCHEMA_ID,
        "retrieval_bias_score": score,
        "retrieval_bias_reasons": loom_retrieval_bias_reasons(block),
    })
}

fn loom_search_excerpt(value: &str) -> String {
    let compact = value.split_whitespace().collect::<Vec<_>>().join(" ");
    compact.chars().take(240).collect()
}

fn map_ai_job(row: PgRow) -> StorageResult<AiJob> {
    let job_inputs = row
        .get::<Option<String>, _>("job_inputs")
        .map(|val| serde_json::from_str::<Value>(&val))
        .transpose()?;
    let job_outputs = row
        .get::<Option<String>, _>("job_outputs")
        .map(|val| serde_json::from_str::<Value>(&val))
        .transpose()?;

    Ok(AiJob {
        job_id: Uuid::parse_str(row.get::<String, _>("id").as_str())
            .map_err(|_| StorageError::Validation("invalid job_id uuid"))?,
        trace_id: Uuid::parse_str(row.get::<String, _>("trace_id").as_str())
            .map_err(|_| StorageError::Validation("invalid trace_id uuid"))?,
        workflow_run_id: row
            .get::<Option<String>, _>("workflow_run_id")
            .map(|id| Uuid::parse_str(&id))
            .transpose()
            .map_err(|_| StorageError::Validation("invalid workflow_run_id uuid"))?,
        job_kind: JobKind::from_str(row.get::<String, _>("job_kind").as_str())?,
        state: JobState::try_from(row.get::<String, _>("status").as_str())?,
        error_message: row.get("error_message"),
        protocol_id: row.get("protocol_id"),
        profile_id: row.get("profile_id"),
        capability_profile_id: row.get("capability_profile_id"),
        access_mode: AccessMode::try_from(row.get::<String, _>("access_mode").as_str())?,
        safety_mode: SafetyMode::try_from(row.get::<String, _>("safety_mode").as_str())?,
        entity_refs: serde_json::from_str::<Vec<EntityRef>>(&row.get::<String, _>("entity_refs"))?,
        planned_operations: serde_json::from_str::<Vec<PlannedOperation>>(
            &row.get::<String, _>("planned_operations"),
        )?,
        metrics: serde_json::from_str::<JobMetrics>(&row.get::<String, _>("metrics"))?,
        status_reason: row.get("status_reason"),
        job_inputs,
        job_outputs,
        created_at: map_timestamp(&row, "created_at"),
        updated_at: map_timestamp(&row, "updated_at"),
    })
}

fn map_model_session(row: PgRow) -> StorageResult<ModelSession> {
    let grants_raw: String = row.get("capability_grants");
    let token_ids_raw: Option<String> = row.get("capability_token_ids");
    let job_id_raw: Option<String> = row.get("job_id");
    let checkpoint_artifact_id: Option<String> = row.get("checkpoint_artifact_id");
    let last_checkpoint_at = map_optional_timestamp(&row, "last_checkpoint_at");
    let checkpoint_count = match row.try_get::<i64, _>("checkpoint_count") {
        Ok(value) => value,
        Err(_) => i64::from(row.try_get::<i32, _>("checkpoint_count")?),
    };
    let merge_back_artifact_raw: Option<String> = row.get("merge_back_artifact");
    let merge_back_artifact = merge_back_artifact_raw
        .as_deref()
        .map(serde_json::from_str::<MergeBackArtifact>)
        .transpose()
        .map_err(|_| StorageError::Validation("invalid merge_back_artifact"))?;

    Ok(ModelSession {
        session_id: row.get("session_id"),
        parent_session_id: row.get("parent_session_id"),
        spawn_depth: row.get("spawn_depth"),
        state: ModelSessionState::try_from(row.get::<String, _>("state").as_str())?,
        model_id: row.get("model_id"),
        backend: row.get("backend"),
        parameter_class: row.get("parameter_class"),
        role: row.get("role"),
        wp_id: row.get("wp_id"),
        mt_id: row.get("mt_id"),
        work_profile_id: row.get("work_profile_id"),
        execution_mode: row.get("execution_mode"),
        memory_policy: row.get("memory_policy"),
        consent_receipt_id: row.get("consent_receipt_id"),
        capability_grants: serde_json::from_str(&grants_raw).unwrap_or_default(),
        capability_token_ids: token_ids_raw
            .as_deref()
            .map(serde_json::from_str::<Vec<String>>)
            .transpose()
            .map_err(|_| StorageError::Validation("invalid capability_token_ids"))?,
        job_id: job_id_raw
            .as_deref()
            .map(Uuid::parse_str)
            .transpose()
            .map_err(|_| StorageError::Validation("invalid model session job_id"))?,
        checkpoint_artifact_id,
        last_checkpoint_at,
        checkpoint_count,
        merge_back_artifact,
        agent: row.get("agent"),
        purpose: row.get("purpose"),
        close_reason: row.get("close_reason"),
        closed_by_actor: row.get("closed_by_actor"),
        closed_at: map_optional_timestamp(&row, "closed_at"),
        created_at: map_timestamp(&row, "created_at"),
        updated_at: map_timestamp(&row, "updated_at"),
    })
}

fn map_session_checkpoint_row(row: PgRow) -> StorageResult<SessionCheckpoint> {
    Ok(SessionCheckpoint {
        checkpoint_id: row.get("checkpoint_id"),
        session_id: row.get("session_id"),
        timestamp: map_timestamp(&row, "timestamp"),
        session_state_json: row.get("session_state_json"),
        message_thread_tail_id: row.get("message_thread_tail_id"),
        pending_tool_calls_json: row.get("pending_tool_calls_json"),
        checkpoint_artifact_id: row.get("checkpoint_artifact_id"),
    })
}

fn map_session_message(row: PgRow) -> StorageResult<SessionMessage> {
    let attachments_raw: String = row.get("attachments");
    let token_count = map_optional_i64_flexible(&row, "token_count");
    let redacted: bool = row.get("redacted");
    let tool_call_id: Option<String> = row.get("tool_call_id");

    Ok(SessionMessage {
        message_id: row.get("message_id"),
        session_id: row.get("session_id"),
        role: SessionMessageRole::try_from(row.get::<String, _>("role").as_str())?,
        content_hash: row.get("content_hash"),
        content_artifact_id: row.get("content_artifact_id"),
        token_count,
        redacted,
        tool_call_id,
        attachments: serde_json::from_str(&attachments_raw).unwrap_or_default(),
        created_at: map_timestamp(&row, "created_at"),
    })
}

fn map_kernel_event(row: PgRow) -> StorageResult<KernelEvent> {
    let event_type_raw: String = row.get("event_type");
    let event_type = KernelEventType::try_from(event_type_raw.as_str())
        .map_err(|_| StorageError::Validation("invalid kernel event_type"))?;
    let actor_kind: String = row.get("actor_kind");
    let actor_id: String = row.get("actor_id");
    let payload_raw: String = row.get("payload");

    Ok(KernelEvent {
        event_id: row.get("event_id"),
        event_sequence: row.get("event_sequence"),
        event_version: row.get("event_version"),
        kernel_task_run_id: row.get("kernel_task_run_id"),
        session_run_id: row.get("session_run_id"),
        aggregate_type: row.get("aggregate_type"),
        aggregate_id: row.get("aggregate_id"),
        idempotency_key: row.get("idempotency_key"),
        event_type,
        actor: kernel_actor_from_parts(actor_kind.as_str(), actor_id)?,
        causation_id: row.get("causation_id"),
        correlation_id: row.get("correlation_id"),
        payload_hash: row.get("payload_hash"),
        source_component: row.get("source_component"),
        payload: serde_json::from_str(payload_raw.as_str())?,
        created_at: map_timestamp(&row, "created_at"),
    })
}

fn crdt_storage_authority_str(authority: CrdtStorageAuthorityPosture) -> &'static str {
    match authority {
        CrdtStorageAuthorityPosture::PostgresEventLedger => "postgres_event_ledger",
        CrdtStorageAuthorityPosture::FileSystemAuthority => "filesystem_authority",
        CrdtStorageAuthorityPosture::MemoryOnly => "memory_only",
    }
}

fn parse_crdt_storage_authority(value: &str) -> StorageResult<CrdtStorageAuthorityPosture> {
    match value {
        "postgres_event_ledger" => Ok(CrdtStorageAuthorityPosture::PostgresEventLedger),
        "filesystem_authority" => Ok(CrdtStorageAuthorityPosture::FileSystemAuthority),
        "memory_only" => Ok(CrdtStorageAuthorityPosture::MemoryOnly),
        _ => Err(StorageError::Validation("invalid CRDT storage authority")),
    }
}

async fn ensure_kernel_crdt_event_ref_exists(pool: &PgPool, event_id: &str) -> StorageResult<()> {
    if event_id.trim().is_empty() {
        return Err(StorageError::Validation(
            "kernel CRDT EventLedger event ref is missing",
        ));
    }
    let exists = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS (SELECT 1 FROM kernel_event_ledger WHERE event_id = $1)",
    )
    .bind(event_id)
    .fetch_one(pool)
    .await?;
    if exists {
        Ok(())
    } else {
        Err(StorageError::Validation(
            "kernel CRDT EventLedger event ref is missing",
        ))
    }
}

fn map_kernel_crdt_update(row: PgRow) -> StorageResult<CrdtUpdateRecordV1> {
    let replay_metadata_raw: String = row.get("replay_metadata_json");
    let storage_authority_raw: String = row.get("storage_authority");
    let update_seq: i64 = row.get("update_seq");
    Ok(CrdtUpdateRecordV1 {
        schema_id: row.get("schema_id"),
        workspace_id: row.get("workspace_id"),
        document_id: row.get("document_id"),
        crdt_document_id: row.get("crdt_document_id"),
        update_id: row.get("update_id"),
        update_seq: update_seq as u64,
        update_sha256: row.get("update_sha256"),
        update_bytes_ref: row.get("update_bytes_ref"),
        actor_id: row.get("actor_id"),
        actor_kind: row.get("actor_kind"),
        session_id: row.get("session_id"),
        trace_id: row.get("trace_id"),
        state_vector_before: row.get("state_vector_before"),
        state_vector_after: row.get("state_vector_after"),
        replay_metadata: serde_json::from_str::<CrdtReplayMetadataV1>(&replay_metadata_raw)?,
        event_ledger_stream_id: row.get("event_ledger_stream_id"),
        event_ledger_event_id: row.get("event_ledger_event_id"),
        storage_authority: parse_crdt_storage_authority(&storage_authority_raw)?,
    })
}

fn map_kernel_crdt_snapshot(row: PgRow) -> StorageResult<CrdtSnapshotRecordV1> {
    let promotion_evidence_raw: String = row.get("promotion_evidence_update_ids");
    let storage_authority_raw: String = row.get("storage_authority");
    let covered_update_seq: i64 = row.get("covered_update_seq");
    Ok(CrdtSnapshotRecordV1 {
        schema_id: row.get("schema_id"),
        snapshot_id: row.get("snapshot_id"),
        workspace_id: row.get("workspace_id"),
        document_id: row.get("document_id"),
        crdt_document_id: row.get("crdt_document_id"),
        covered_update_seq: covered_update_seq as u64,
        state_vector: row.get("state_vector"),
        snapshot_sha256: row.get("snapshot_sha256"),
        snapshot_bytes_ref: row.get("snapshot_bytes_ref"),
        actor_id: row.get("actor_id"),
        actor_kind: row.get("actor_kind"),
        event_ledger_stream_id: row.get("event_ledger_stream_id"),
        event_ledger_event_id: row.get("event_ledger_event_id"),
        promotion_evidence_update_ids: serde_json::from_str::<Vec<String>>(
            &promotion_evidence_raw,
        )?,
        storage_authority: parse_crdt_storage_authority(&storage_authority_raw)?,
    })
}

fn kernel_actor_from_parts(actor_kind: &str, actor_id: String) -> StorageResult<KernelActor> {
    match actor_kind {
        "operator" => Ok(KernelActor::Operator(actor_id)),
        "system" => Ok(KernelActor::System(actor_id)),
        "session_broker" => Ok(KernelActor::SessionBroker(actor_id)),
        "model_adapter" => Ok(KernelActor::ModelAdapter(actor_id)),
        "toolgate" => Ok(KernelActor::ToolGate(actor_id)),
        "validation_runner" => Ok(KernelActor::ValidationRunner(actor_id)),
        "promotion_gate" => Ok(KernelActor::PromotionGate(actor_id)),
        _ => Err(StorageError::Validation("invalid kernel actor_kind")),
    }
}

fn map_kernel_session_lease(row: PgRow) -> StorageResult<KernelSessionLease> {
    let state_raw: String = row.get("state");
    let state = SessionRunState::parse(state_raw.as_str())
        .map_err(|_| StorageError::Validation("invalid kernel session state"))?;

    Ok(KernelSessionLease {
        session_run_id: row.get("session_run_id"),
        kernel_task_run_id: row.get("kernel_task_run_id"),
        adapter_id: row.get("adapter_id"),
        state,
        claimed_by: row.get("claimed_by"),
        lease_expires_at: map_optional_timestamp(&row, "lease_expires_at"),
        attempt_count: map_i64_flexible(&row, "attempt_count"),
        created_at: map_timestamp(&row, "created_at"),
        updated_at: map_timestamp(&row, "updated_at"),
    })
}

fn session_state_event_type(state: SessionRunState) -> KernelEventType {
    match state {
        SessionRunState::Queued => KernelEventType::SessionQueued,
        SessionRunState::Claimed => KernelEventType::SessionClaimed,
        SessionRunState::Running => KernelEventType::SessionStarted,
        SessionRunState::Completed => KernelEventType::SessionCompleted,
        SessionRunState::Failed => KernelEventType::SessionFailed,
        SessionRunState::Cancelled => KernelEventType::SessionCancelled,
        SessionRunState::BackpressureDelayed => KernelEventType::SessionBackpressureDelayed,
        SessionRunState::RetryScheduled => KernelEventType::SessionRetryScheduled,
        SessionRunState::DeadLettered => KernelEventType::SessionDeadLettered,
    }
}

fn build_kernel_session_event(
    kernel_task_run_id: &str,
    session_run_id: &str,
    event_type: KernelEventType,
    causation_id: Option<String>,
    correlation_id: String,
    payload: Value,
) -> StorageResult<NewKernelEvent> {
    let mut builder = NewKernelEvent::builder(
        kernel_task_run_id,
        session_run_id,
        event_type,
        KernelActor::SessionBroker("kernel-session-broker".to_string()),
    )
    .correlation_id(correlation_id)
    .source_component("session_broker")
    .payload(payload);

    if let Some(causation_id) = causation_id {
        builder = builder.causation_id(causation_id);
    }

    builder
        .build()
        .map_err(|err| StorageError::Serialization(err.to_string()))
}

pub(crate) async fn append_kernel_event_with_executor<'e, E>(
    executor: E,
    event: NewKernelEvent,
) -> StorageResult<KernelEvent>
where
    E: Executor<'e, Database = Postgres>,
{
    event
        .validate()
        .map_err(|_| StorageError::Validation("invalid kernel event"))?;
    let kernel_event = KernelEvent::from_new(event.clone());
    let payload = String::from_utf8(canonical_json_bytes(&event.payload))
        .map_err(|err| StorageError::Serialization(err.to_string()))?;

    let new_event_id = kernel_event.event_id.clone();
    let row = sqlx::query(
        r#"
        WITH inserted AS (
        INSERT INTO kernel_event_ledger (
            event_id,
            event_version,
            kernel_task_run_id,
            session_run_id,
            aggregate_type,
            aggregate_id,
            idempotency_key,
            event_type,
            actor_kind,
            actor_id,
            causation_id,
            correlation_id,
            payload_hash,
            source_component,
            payload,
            created_at
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15::jsonb, $16)
        ON CONFLICT (idempotency_key) DO NOTHING
        RETURNING
            event_id,
            event_sequence,
            event_version,
            kernel_task_run_id,
            session_run_id,
            aggregate_type,
            aggregate_id,
            idempotency_key,
            event_type,
            actor_kind,
            actor_id,
            causation_id,
            correlation_id,
            payload_hash,
            source_component,
            payload,
            created_at
        )
        SELECT
            event_id,
            event_sequence,
            event_version,
            kernel_task_run_id,
            session_run_id,
            aggregate_type,
            aggregate_id,
            idempotency_key,
            event_type,
            actor_kind,
            actor_id,
            causation_id,
            correlation_id,
            payload_hash,
            source_component,
            payload::text AS payload,
            created_at
        FROM inserted
        UNION ALL
        SELECT
            event_id,
            event_sequence,
            event_version,
            kernel_task_run_id,
            session_run_id,
            aggregate_type,
            aggregate_id,
            idempotency_key,
            event_type,
            actor_kind,
            actor_id,
            causation_id,
            correlation_id,
            payload_hash,
            source_component,
            payload::text AS payload,
            created_at
        FROM kernel_event_ledger
        WHERE idempotency_key = $7
        LIMIT 1
        "#,
    )
    .bind(&kernel_event.event_id)
    .bind(&event.event_version)
    .bind(&event.kernel_task_run_id)
    .bind(&event.session_run_id)
    .bind(&event.aggregate_type)
    .bind(&event.aggregate_id)
    .bind(&event.idempotency_key)
    .bind(event.event_type.as_str())
    .bind(event.actor.actor_kind())
    .bind(event.actor.actor_id())
    .bind(event.causation_id.as_deref())
    .bind(event.correlation_id.as_deref())
    .bind(&event.payload_hash)
    .bind(&event.source_component)
    .bind(payload)
    .bind(kernel_event.created_at)
    .fetch_one(executor)
    .await?;

    let stored = map_kernel_event(row)?;
    let mut mismatches = Vec::new();
    if stored.event_version != event.event_version {
        mismatches.push("event_version");
    }
    if stored.kernel_task_run_id != event.kernel_task_run_id {
        mismatches.push("kernel_task_run_id");
    }
    if stored.session_run_id != event.session_run_id {
        mismatches.push("session_run_id");
    }
    if stored.aggregate_type != event.aggregate_type {
        mismatches.push("aggregate_type");
    }
    if stored.aggregate_id != event.aggregate_id {
        mismatches.push("aggregate_id");
    }
    if stored.event_type != event.event_type {
        mismatches.push("event_type");
    }
    if stored.actor != event.actor {
        mismatches.push("actor");
    }
    if stored.causation_id != event.causation_id {
        mismatches.push("causation_id");
    }
    if stored.correlation_id != event.correlation_id {
        mismatches.push("correlation_id");
    }
    if stored.payload_hash != event.payload_hash {
        mismatches.push("payload_hash");
    }
    if stored.source_component != event.source_component {
        mismatches.push("source_component");
    }
    // `payload_hash` is computed from Handshake canonical JSON before insert.
    // PostgreSQL JSONB may normalize the raw payload shape on readback
    // (especially numeric rendering), so comparing `Value` directly creates
    // false idempotency conflicts for successfully inserted events.
    if !mismatches.is_empty() {
        return Err(StorageError::Validation(leak_kernel_event_conflict(
            format!(
                "kernel event idempotency conflict: idempotency_key={} aggregate={}/{} existing_event_id={} new_event_id={} existing_payload_hash={} new_payload_hash={} mismatches={}",
                event.idempotency_key,
                event.aggregate_type,
                event.aggregate_id,
                stored.event_id,
                new_event_id,
                stored.payload_hash,
                event.payload_hash,
                mismatches.join(",")
            ),
        )));
    }

    Ok(stored)
}

fn leak_kernel_event_conflict(message: String) -> &'static str {
    Box::leak(message.into_boxed_str())
}

fn is_sha256_hex(value: &str) -> bool {
    value.len() == 64 && value.chars().all(|c| c.is_ascii_hexdigit())
}

fn map_workflow_run(row: PgRow) -> StorageResult<WorkflowRun> {
    Ok(WorkflowRun {
        id: Uuid::parse_str(row.get::<String, _>("id").as_str())
            .map_err(|_| StorageError::Validation("invalid workflow_run id"))?,
        job_id: Uuid::parse_str(row.get::<String, _>("job_id").as_str())
            .map_err(|_| StorageError::Validation("invalid workflow_run job_id"))?,
        status: JobState::try_from(row.get::<String, _>("status").as_str())?,
        last_heartbeat: map_timestamp(&row, "last_heartbeat"),
        created_at: map_timestamp(&row, "created_at"),
        updated_at: map_timestamp(&row, "updated_at"),
    })
}

fn map_workflow_node_execution(row: PgRow) -> StorageResult<WorkflowNodeExecution> {
    let input_payload = row
        .get::<Option<String>, _>("input_payload")
        .map(|val| serde_json::from_str(&val))
        .transpose()?;
    let output_payload = row
        .get::<Option<String>, _>("output_payload")
        .map(|val| serde_json::from_str(&val))
        .transpose()?;

    Ok(WorkflowNodeExecution {
        id: Uuid::parse_str(row.get::<String, _>("id").as_str())
            .map_err(|_| StorageError::Validation("invalid node execution id"))?,
        workflow_run_id: Uuid::parse_str(row.get::<String, _>("workflow_run_id").as_str())
            .map_err(|_| StorageError::Validation("invalid workflow_run_id"))?,
        node_id: row.get("node_id"),
        node_type: row.get("node_type"),
        status: JobState::try_from(row.get::<String, _>("status").as_str())?,
        sequence: map_i64_from_i32(&row, "sequence"),
        input_payload,
        output_payload,
        error_message: row.get("error_message"),
        started_at: map_timestamp(&row, "started_at"),
        finished_at: map_optional_timestamp(&row, "finished_at"),
        created_at: map_timestamp(&row, "created_at"),
        updated_at: map_timestamp(&row, "updated_at"),
    })
}

fn map_timestamp(row: &PgRow, column: &str) -> chrono::DateTime<Utc> {
    // Some legacy tables use `TIMESTAMP` while newer tables use `TIMESTAMPTZ`.
    // Decode both without panicking to keep mixed-schema compatibility.
    if let Ok(value) = row.try_get::<chrono::DateTime<Utc>, _>(column) {
        return value;
    }

    let value: NaiveDateTime = row.get(column);
    value.and_utc()
}

fn map_optional_timestamp(row: &PgRow, column: &str) -> Option<chrono::DateTime<Utc>> {
    // Some legacy tables use `TIMESTAMP` while newer tables use `TIMESTAMPTZ`.
    if let Ok(value) = row.try_get::<Option<chrono::DateTime<Utc>>, _>(column) {
        return value;
    }

    row.get::<Option<NaiveDateTime>, _>(column)
        .map(|value| value.and_utc())
}

fn map_i64_from_i32(row: &PgRow, column: &str) -> i64 {
    let value: i32 = row.get(column);
    value as i64
}

fn map_i64_flexible(row: &PgRow, column: &str) -> i64 {
    row.try_get::<i64, _>(column)
        .or_else(|_| row.try_get::<i32, _>(column).map(i64::from))
        .expect("integer column should decode as i64 or i32")
}

fn map_optional_i64_flexible(row: &PgRow, column: &str) -> Option<i64> {
    row.try_get::<Option<i64>, _>(column)
        .or_else(|_| {
            row.try_get::<Option<i32>, _>(column)
                .map(|value| value.map(i64::from))
        })
        .expect("nullable integer column should decode as i64 or i32")
}

fn map_f64_from_f32(row: &PgRow, column: &str) -> f64 {
    let value: f32 = row.get(column);
    value as f64
}

fn is_pg_unique_violation(err: &sqlx::Error) -> bool {
    matches!(err, sqlx::Error::Database(db_err) if db_err.code().as_deref() == Some("23505"))
}

fn is_pg_foreign_key_violation(err: &sqlx::Error) -> bool {
    matches!(err, sqlx::Error::Database(db_err) if db_err.code().as_deref() == Some("23503"))
}

fn empty_json_object() -> Value {
    Value::Object(Default::default())
}

fn empty_json_array() -> Value {
    Value::Array(Vec::new())
}

fn encode_json(value: &Value) -> StorageResult<String> {
    Ok(serde_json::to_string(value)?)
}

fn encode_string_vec(values: &[String]) -> StorageResult<String> {
    Ok(serde_json::to_string(values)?)
}

fn decode_json_or_default(raw: Option<String>, default: Value) -> StorageResult<Value> {
    match raw {
        Some(raw) => Ok(serde_json::from_str(&raw)?),
        None => Ok(default),
    }
}

fn decode_string_vec_or_default(raw: Option<String>) -> StorageResult<Vec<String>> {
    match raw {
        Some(raw) => Ok(serde_json::from_str(&raw)?),
        None => Ok(Vec::new()),
    }
}

fn validate_calendar_source_upsert(source: &CalendarSourceUpsert) -> StorageResult<()> {
    if source.id.trim().is_empty() {
        return Err(StorageError::Validation("calendar source id is required"));
    }
    if source.workspace_id.trim().is_empty() {
        return Err(StorageError::Validation(
            "calendar source workspace_id is required",
        ));
    }
    if source.display_name.trim().is_empty() {
        return Err(StorageError::Validation(
            "calendar source display_name is required",
        ));
    }
    if source.default_tzid.trim().is_empty() {
        return Err(StorageError::Validation(
            "calendar source default_tzid is required",
        ));
    }
    Ok(())
}

fn validate_calendar_event_upsert(event: &CalendarEventUpsert) -> StorageResult<()> {
    if event.id.trim().is_empty() {
        return Err(StorageError::Validation("calendar event id is required"));
    }
    if event.workspace_id.trim().is_empty() {
        return Err(StorageError::Validation(
            "calendar event workspace_id is required",
        ));
    }
    if event.source_id.trim().is_empty() {
        return Err(StorageError::Validation(
            "calendar event source_id is required",
        ));
    }
    if event.title.trim().is_empty() {
        return Err(StorageError::Validation("calendar event title is required"));
    }
    if event.tzid.trim().is_empty() {
        return Err(StorageError::Validation("calendar event tzid is required"));
    }
    if event.end_ts_utc <= event.start_ts_utc {
        return Err(StorageError::Validation(
            "calendar event end_ts_utc must be after start_ts_utc",
        ));
    }
    if event
        .external_id
        .as_deref()
        .is_some_and(|value| value.trim().is_empty())
    {
        return Err(StorageError::Validation(
            "calendar event external_id cannot be blank",
        ));
    }
    Ok(())
}

fn validate_calendar_event_query(query: &CalendarEventWindowQuery) -> StorageResult<()> {
    if query.workspace_id.trim().is_empty() {
        return Err(StorageError::Validation(
            "calendar query workspace_id is required",
        ));
    }
    if query.window_end_utc <= query.window_start_utc {
        return Err(StorageError::Validation(
            "calendar query window_end_utc must be after window_start_utc",
        ));
    }
    Ok(())
}

fn map_calendar_source(row: PgRow) -> StorageResult<CalendarSource> {
    let sync_state = row
        .get::<Option<String>, _>("sync_state")
        .as_deref()
        .map(CalendarSyncStateStage::from_str)
        .transpose()?;

    Ok(CalendarSource {
        id: row.get("id"),
        workspace_id: row.get("workspace_id"),
        display_name: row.get("display_name"),
        provider_type: CalendarSourceProviderType::from_str(
            row.get::<String, _>("provider_type").as_str(),
        )?,
        write_policy: CalendarSourceWritePolicy::from_str(
            row.get::<String, _>("write_policy").as_str(),
        )?,
        default_tzid: row.get("default_tzid"),
        auto_export: row.get("auto_export"),
        credentials_ref: row.get("credentials_ref"),
        provider_calendar_id: row.get("provider_calendar_id"),
        capability_profile_id: row.get("capability_profile_id"),
        config: decode_json_or_default(row.get("config_json"), empty_json_object())?,
        sync_state: CalendarSourceSyncState {
            state: sync_state,
            sync_token: row.get("sync_token"),
            last_synced_at: map_optional_timestamp(&row, "last_sync_ts"),
            last_full_sync_at: map_optional_timestamp(&row, "last_full_sync_ts"),
            last_ok_at: map_optional_timestamp(&row, "last_ok_at"),
            last_pull_at: map_optional_timestamp(&row, "last_pull_at"),
            last_push_at: map_optional_timestamp(&row, "last_push_at"),
            last_error_at: map_optional_timestamp(&row, "last_error_at"),
            last_error_code: row.get("last_error_code"),
            last_error: row.get("last_error"),
            backoff_until: map_optional_timestamp(&row, "backoff_until"),
            consecutive_failures: row.get("consecutive_failures"),
            last_remote_watermark: row.get("last_remote_watermark"),
            last_local_applied_rev: row.get("last_local_applied_rev"),
        },
        last_job_id: row.get("last_job_id"),
        last_workflow_id: row.get("last_workflow_id"),
        last_actor_id: row.get("last_actor_id"),
        edit_event_id: row.get("edit_event_id"),
        last_actor_kind: row.get("last_actor_kind"),
        created_at: map_timestamp(&row, "created_at"),
        updated_at: map_timestamp(&row, "updated_at"),
    })
}

fn map_calendar_event(row: PgRow) -> StorageResult<CalendarEvent> {
    Ok(CalendarEvent {
        id: row.get("id"),
        workspace_id: row.get("workspace_id"),
        source_id: row.get("source_id"),
        external_id: row.get("external_id"),
        external_etag: row.get("external_etag"),
        title: row.get("title"),
        description: row.get("description"),
        location: row.get("location"),
        start_ts_utc: map_timestamp(&row, "start_ts_utc"),
        end_ts_utc: map_timestamp(&row, "end_ts_utc"),
        start_local: row.get("start_local"),
        end_local: row.get("end_local"),
        tzid: row.get("tzid"),
        all_day: row.get("all_day"),
        was_floating: row.get("was_floating"),
        status: CalendarEventStatus::from_str(row.get::<String, _>("status").as_str())?,
        visibility: CalendarEventVisibility::from_str(row.get::<String, _>("visibility").as_str())?,
        export_mode: CalendarEventExportMode::from_str(
            row.get::<String, _>("export_mode").as_str(),
        )?,
        rrule: row.get("rrule"),
        rdate: decode_string_vec_or_default(row.get("rdate_json"))?,
        exdate: decode_string_vec_or_default(row.get("exdate_json"))?,
        is_recurring: row.get("is_recurring"),
        series_id: row.get("series_id"),
        instance_key: row.get("instance_key"),
        is_override: row.get("is_override"),
        source_last_seen_at: map_optional_timestamp(&row, "source_last_seen_at"),
        created_by: row.get("created_by"),
        attendees: decode_json_or_default(row.get("attendees_json"), empty_json_array())?,
        links: decode_json_or_default(row.get("links_json"), empty_json_array())?,
        provider_payload: row
            .get::<Option<String>, _>("provider_payload_json")
            .map(|raw| serde_json::from_str::<Value>(&raw))
            .transpose()?,
        last_job_id: row.get("last_job_id"),
        last_workflow_id: row.get("last_workflow_id"),
        last_actor_id: row.get("last_actor_id"),
        edit_event_id: row.get("edit_event_id"),
        last_actor_kind: row.get("last_actor_kind"),
        created_at: map_timestamp(&row, "created_at"),
        updated_at: map_timestamp(&row, "updated_at"),
    })
}

#[async_trait]
impl super::Database for PostgresDatabase {
    fn supports_locus_runtime(&self) -> bool {
        true
    }

    fn supports_structured_collab_artifacts(&self) -> bool {
        true
    }

    fn loom_search_observability_tier(&self) -> u8 {
        2
    }

    fn supports_loom_graph_filtering(&self) -> bool {
        true
    }

    fn loom_traverse_graph_perf_target_ms(&self) -> u128 {
        50
    }

    async fn run_migrations(&self) -> StorageResult<()> {
        sqlx::migrate!("./migrations").run(&self.pool).await?;
        ensure_locus_schema_postgres(&self.pool).await?;
        ensure_kernel_event_ledger_schema_postgres(&self.pool).await?;
        Ok(())
    }

    async fn migration_version(&self) -> StorageResult<i64> {
        let version = sqlx::query_scalar::<_, i64>(
            "SELECT COALESCE(MAX(version), 0) FROM _sqlx_migrations WHERE success = TRUE",
        )
        .fetch_one(&self.pool)
        .await?;
        Ok(version)
    }

    async fn execute_locus_operation(
        &self,
        op: crate::workflows::locus::types::LocusOperation,
    ) -> StorageResult<Value> {
        execute_locus_operation(self, op).await
    }

    async fn locus_task_board_update_work_packet(
        &self,
        status: &str,
        task_board_status: &str,
        updated_at: &str,
        metadata: &str,
        wp_id: &str,
    ) -> StorageResult<()> {
        locus_task_board_update_work_packet(
            self,
            status,
            task_board_status,
            updated_at,
            metadata,
            wp_id,
        )
        .await
    }

    async fn structured_collab_work_packet_row(
        &self,
        wp_id: &str,
    ) -> StorageResult<Option<super::StructuredCollabWorkPacketRow>> {
        sqlx::query_as::<_, super::StructuredCollabWorkPacketRow>(
            r#"
            SELECT
                wp_id,
                version,
                title,
                description,
                status,
                priority,
                phase,
                routing,
                task_packet_path,
                task_board_status,
                assignee,
                reporter,
                created_at,
                updated_at,
                vector_clock,
                metadata
            FROM work_packets
            WHERE wp_id = $1
            "#,
        )
        .bind(wp_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(StorageError::from)
    }

    async fn structured_collab_work_packet_rows(
        &self,
    ) -> StorageResult<Vec<super::StructuredCollabWorkPacketRow>> {
        sqlx::query_as::<_, super::StructuredCollabWorkPacketRow>(
            r#"
            SELECT
                wp_id,
                version,
                title,
                description,
                status,
                priority,
                phase,
                routing,
                task_packet_path,
                task_board_status,
                assignee,
                reporter,
                created_at,
                updated_at,
                vector_clock,
                metadata
            FROM work_packets
            ORDER BY updated_at ASC, wp_id ASC
            "#,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(StorageError::from)
    }

    async fn structured_collab_micro_task_metadata(
        &self,
        wp_id: &str,
        mt_id: &str,
    ) -> StorageResult<Option<String>> {
        sqlx::query_scalar::<_, String>(
            r#"
            SELECT metadata
            FROM micro_tasks
            WHERE wp_id = $1 AND mt_id = $2
            "#,
        )
        .bind(wp_id)
        .bind(mt_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(StorageError::from)
    }

    async fn structured_collab_micro_task_status_rows(
        &self,
        wp_id: &str,
    ) -> StorageResult<Vec<(String, String)>> {
        sqlx::query_as::<_, (String, String)>(
            "SELECT mt_id, status FROM micro_tasks WHERE wp_id = $1 ORDER BY mt_id ASC",
        )
        .bind(wp_id)
        .fetch_all(&self.pool)
        .await
        .map_err(StorageError::from)
    }

    async fn structured_collab_micro_task_rows(
        &self,
        wp_id: &str,
    ) -> StorageResult<Vec<(String, String)>> {
        sqlx::query_as::<_, (String, String)>(
            "SELECT mt_id, metadata FROM micro_tasks WHERE wp_id = $1 ORDER BY mt_id ASC",
        )
        .bind(wp_id)
        .fetch_all(&self.pool)
        .await
        .map_err(StorageError::from)
    }

    #[cfg(test)]
    async fn test_overwrite_loom_block_metrics(
        &self,
        workspace_id: &str,
        block_id: &str,
        mention_count: i64,
        tag_count: i64,
        backlink_count: i64,
    ) -> StorageResult<()> {
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
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    #[cfg(test)]
    async fn test_zero_workspace_loom_metrics(&self, workspace_id: &str) -> StorageResult<()> {
        sqlx::query(
            r#"
            UPDATE loom_blocks
            SET mention_count = 0, tag_count = 0, backlink_count = 0
            WHERE workspace_id = $1
            "#,
        )
        .bind(workspace_id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    #[cfg(any(test, feature = "test-utils"))]
    async fn test_insert_loom_traversal_perf_fixture(
        &self,
        workspace_id: &str,
        total_blocks: usize,
    ) -> StorageResult<String> {
        if total_blocks == 0 {
            return Err(StorageError::Validation(
                "loom traversal perf fixture requires at least one block",
            ));
        }

        const INSERT_CHUNK_ROWS: usize = 1_000;

        let created_at = Utc::now();
        let derived_json = serde_json::to_string(&super::LoomBlockDerived::default())?;
        let start_block_id = "perf-block-00000".to_string();
        let mut tx = self.pool.begin().await?;

        for chunk_start in (0..total_blocks).step_by(INSERT_CHUNK_ROWS) {
            let chunk_end = (chunk_start + INSERT_CHUNK_ROWS).min(total_blocks);
            let mut builder: QueryBuilder<Postgres> = QueryBuilder::new(
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
                "#,
            );

            builder.push_values(chunk_start..chunk_end, |mut row, idx| {
                row.push_bind(format!("perf-block-{idx:05}"))
                    .push_bind(workspace_id)
                    .push(" 'note' ")
                    .push_bind(format!("Perf Block {idx}"))
                    .push(" 0 ")
                    .push(" 'system' ")
                    .push_bind(Uuid::now_v7().to_string())
                    .push_bind(created_at)
                    .push_bind(created_at)
                    .push(" 0 ")
                    .push(" 0 ")
                    .push(" 0 ")
                    .push_bind(&derived_json)
                    .push(" 'none' ");
            });

            builder.build().execute(&mut *tx).await?;
        }

        if total_blocks > 1 {
            for chunk_start in (1..total_blocks).step_by(INSERT_CHUNK_ROWS) {
                let chunk_end = (chunk_start + INSERT_CHUNK_ROWS).min(total_blocks);
                let mut builder: QueryBuilder<Postgres> = QueryBuilder::new(
                    r#"
                    INSERT INTO loom_edges (
                        edge_id,
                        workspace_id,
                        source_block_id,
                        target_block_id,
                        edge_type,
                        created_by,
                        created_at
                    )
                    "#,
                );

                builder.push_values(chunk_start..chunk_end, |mut row, idx| {
                    let block_id = format!("perf-block-{idx:05}");
                    let previous_block_id = format!("perf-block-{:05}", idx - 1);
                    row.push_bind(Uuid::now_v7().to_string())
                        .push_bind(workspace_id)
                        .push_bind(previous_block_id)
                        .push_bind(block_id)
                        .push(" 'mention' ")
                        .push(" 'user' ")
                        .push_bind(created_at);
                });

                builder.build().execute(&mut *tx).await?;
            }
        }

        tx.commit().await?;
        Ok(start_block_id)
    }

    #[cfg(test)]
    async fn test_update_ai_job_metadata(
        &self,
        job_id: Uuid,
        status: &str,
        created_at: chrono::DateTime<Utc>,
        is_pinned: bool,
    ) -> StorageResult<()> {
        sqlx::query(
            "UPDATE ai_jobs SET status = $1, created_at = $2, is_pinned = $3 WHERE id = $4",
        )
        .bind(status)
        .bind(created_at)
        .bind(if is_pinned { 1_i32 } else { 0_i32 })
        .bind(job_id.to_string())
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    #[cfg(test)]
    async fn test_fetch_mutation_traceability_row(
        &self,
        table: &str,
        id: &str,
    ) -> StorageResult<super::MutationTraceabilityRow> {
        let sql = format!(
            "SELECT last_actor_kind, last_actor_id, last_job_id, last_workflow_id, edit_event_id FROM {table} WHERE id = $1"
        );
        sqlx::query_as::<_, super::MutationTraceabilityRow>(&sql)
            .bind(id)
            .fetch_one(&self.pool)
            .await
            .map_err(StorageError::from)
    }

    async fn ping(&self) -> StorageResult<()> {
        sqlx::query("SELECT 1").execute(&self.pool).await?;
        Ok(())
    }

    async fn list_workspaces(&self) -> StorageResult<Vec<Workspace>> {
        let rows = sqlx::query(
            r#"SELECT id, name, created_at, updated_at FROM workspaces ORDER BY created_at ASC"#,
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(map_workspace).collect())
    }

    async fn create_workspace(
        &self,
        ctx: &WriteContext,
        workspace: NewWorkspace,
    ) -> StorageResult<Workspace> {
        let now = Utc::now();
        let id = Uuid::now_v7().to_string();
        let metadata = self.guard.validate_write(ctx, &id).await?;
        let actor_kind = metadata.actor_kind.as_str();
        let actor_id = metadata.actor_id.clone();
        let job_id = metadata.job_id.map(|v| v.to_string());
        let workflow_id = metadata.workflow_id.map(|v| v.to_string());
        let edit_event_id = metadata.edit_event_id.to_string();

        let row = sqlx::query(
            r#"
            INSERT INTO workspaces (
                id,
                name,
                created_at,
                updated_at,
                last_actor_kind,
                last_actor_id,
                last_job_id,
                last_workflow_id,
                edit_event_id
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            RETURNING id, name, created_at, updated_at
            "#,
        )
        .bind(&id)
        .bind(&workspace.name)
        .bind(now)
        .bind(now)
        .bind(actor_kind)
        .bind(&actor_id)
        .bind(job_id)
        .bind(workflow_id)
        .bind(edit_event_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(map_workspace(row))
    }

    async fn delete_workspace(&self, ctx: &WriteContext, id: &str) -> StorageResult<()> {
        self.guard.validate_write(ctx, id).await?;
        let res = sqlx::query(r#"DELETE FROM workspaces WHERE id = $1"#)
            .bind(id)
            .execute(&self.pool)
            .await?;
        if res.rows_affected() == 0 {
            return Err(StorageError::NotFound("workspace"));
        }
        Ok(())
    }

    async fn get_workspace(&self, id: &str) -> StorageResult<Option<Workspace>> {
        let row =
            sqlx::query(r#"SELECT id, name, created_at, updated_at FROM workspaces WHERE id = $1"#)
                .bind(id)
                .fetch_optional(&self.pool)
                .await?;

        Ok(row.map(map_workspace))
    }

    async fn list_documents(&self, workspace_id: &str) -> StorageResult<Vec<Document>> {
        let rows = sqlx::query(
            r#"
            SELECT id, workspace_id, title, created_at, updated_at
            FROM documents
            WHERE workspace_id = $1
            ORDER BY created_at ASC
            "#,
        )
        .bind(workspace_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(map_document).collect())
    }

    async fn get_document(&self, doc_id: &str) -> StorageResult<Document> {
        let row = sqlx::query(
            r#"
            SELECT id, workspace_id, title, created_at, updated_at
            FROM documents
            WHERE id = $1
            "#,
        )
        .bind(doc_id)
        .fetch_optional(&self.pool)
        .await?;

        match row {
            Some(row) => Ok(map_document(row)),
            None => Err(StorageError::NotFound("document")),
        }
    }

    async fn create_document(
        &self,
        ctx: &WriteContext,
        doc: NewDocument,
    ) -> StorageResult<Document> {
        let now = Utc::now();
        let id = Uuid::now_v7().to_string();
        let metadata = self.guard.validate_write(ctx, &id).await?;
        let actor_kind = metadata.actor_kind.as_str();
        let actor_id = metadata.actor_id.clone();
        let job_id = metadata.job_id.map(|v| v.to_string());
        let workflow_id = metadata.workflow_id.map(|v| v.to_string());
        let edit_event_id = metadata.edit_event_id.to_string();

        let row = sqlx::query(
            r#"
            INSERT INTO documents (
                id,
                workspace_id,
                title,
                created_at,
                updated_at,
                last_actor_kind,
                last_actor_id,
                last_job_id,
                last_workflow_id,
                edit_event_id
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            RETURNING id, workspace_id, title, created_at, updated_at
            "#,
        )
        .bind(&id)
        .bind(&doc.workspace_id)
        .bind(&doc.title)
        .bind(now)
        .bind(now)
        .bind(actor_kind)
        .bind(&actor_id)
        .bind(job_id)
        .bind(workflow_id)
        .bind(edit_event_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(map_document(row))
    }

    async fn delete_document(&self, ctx: &WriteContext, doc_id: &str) -> StorageResult<()> {
        self.guard.validate_write(ctx, doc_id).await?;
        let res = sqlx::query(r#"DELETE FROM documents WHERE id = $1"#)
            .bind(doc_id)
            .execute(&self.pool)
            .await?;
        if res.rows_affected() == 0 {
            return Err(StorageError::NotFound("document"));
        }
        Ok(())
    }

    async fn get_blocks(&self, doc_id: &str) -> StorageResult<Vec<Block>> {
        let rows = sqlx::query(
            r#"
            SELECT id, document_id, kind, sequence, raw_content, display_content, derived_content,
                   created_at, updated_at, sensitivity, exportable
            FROM blocks
            WHERE document_id = $1
            ORDER BY sequence ASC
            "#,
        )
        .bind(doc_id)
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter()
            .map(map_block)
            .collect::<StorageResult<Vec<_>>>()
    }

    async fn get_block(&self, block_id: &str) -> StorageResult<Block> {
        let row = sqlx::query(
            r#"
            SELECT id, document_id, kind, sequence, raw_content, display_content, derived_content,
                   created_at, updated_at, sensitivity, exportable
            FROM blocks
            WHERE id = $1
            "#,
        )
        .bind(block_id)
        .fetch_optional(&self.pool)
        .await?;

        match row {
            Some(row) => map_block(row),
            None => Err(StorageError::NotFound("block")),
        }
    }

    async fn create_block(&self, ctx: &WriteContext, block: NewBlock) -> StorageResult<Block> {
        let now = Utc::now();
        let id = block.id.unwrap_or_else(|| Uuid::now_v7().to_string());
        let metadata = self.guard.validate_write(ctx, &id).await?;
        let actor_kind = metadata.actor_kind.as_str();
        let actor_id = metadata.actor_id.clone();
        let job_id = metadata.job_id.map(|v| v.to_string());
        let workflow_id = metadata.workflow_id.map(|v| v.to_string());
        let edit_event_id = metadata.edit_event_id.to_string();
        let display_content = block
            .display_content
            .unwrap_or_else(|| block.raw_content.clone());
        let derived_content = block
            .derived_content
            .unwrap_or_else(|| Value::Object(Default::default()))
            .to_string();
        let exportable_int: Option<i32> = block.exportable.map(|v| if v { 1 } else { 0 });

        let row = sqlx::query(
            r#"
            INSERT INTO blocks (
                id,
                document_id,
                kind,
                sequence,
                raw_content,
                display_content,
                derived_content,
                created_at,
                updated_at,
                sensitivity,
                exportable,
                last_actor_kind,
                last_actor_id,
                last_job_id,
                last_workflow_id,
                edit_event_id
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15::jsonb, $16)
            RETURNING id, document_id, kind, sequence, raw_content, display_content, derived_content,
                      created_at, updated_at, sensitivity, exportable
            "#,
        )
        .bind(&id)
        .bind(&block.document_id)
        .bind(&block.kind)
        .bind(block.sequence)
        .bind(&block.raw_content)
        .bind(&display_content)
        .bind(&derived_content)
        .bind(now)
        .bind(now)
        .bind(&block.sensitivity)
        .bind(exportable_int)
        .bind(actor_kind)
        .bind(&actor_id)
        .bind(job_id)
        .bind(workflow_id)
        .bind(edit_event_id)
        .fetch_one(&self.pool)
        .await?;

        map_block(row)
    }

    async fn update_block(
        &self,
        ctx: &WriteContext,
        block_id: &str,
        data: BlockUpdate,
    ) -> StorageResult<()> {
        if data.kind.is_none()
            && data.sequence.is_none()
            && data.raw_content.is_none()
            && data.display_content.is_none()
            && data.derived_content.is_none()
        {
            return Err(StorageError::Validation("no block fields provided"));
        }

        let metadata = self.guard.validate_write(ctx, block_id).await?;
        let actor_kind = metadata.actor_kind.as_str();
        let actor_id = metadata.actor_id.clone();
        let job_id = metadata.job_id.map(|v| v.to_string());
        let workflow_id = metadata.workflow_id.map(|v| v.to_string());
        let edit_event_id = metadata.edit_event_id.to_string();
        let mut block = self.get_block(block_id).await?;

        if let Some(kind) = data.kind {
            block.kind = kind;
        }
        if let Some(seq) = data.sequence {
            block.sequence = seq;
        }
        if let Some(raw) = data.raw_content {
            block.raw_content = raw.clone();
            if block.display_content.is_empty() {
                block.display_content = raw;
            }
        }
        if let Some(display) = data.display_content {
            block.display_content = display;
        }
        if let Some(derived) = data.derived_content {
            block.derived_content = derived;
        }

        let derived_content = block.derived_content.to_string();
        let now = Utc::now();

        sqlx::query(
            r#"
            UPDATE blocks
            SET kind = $1,
                sequence = $2,
                raw_content = $3,
                display_content = $4,
                derived_content = $5,
                last_actor_kind = $6,
                last_actor_id = $7,
                last_job_id = $8,
                last_workflow_id = $9,
                edit_event_id = $10,
                updated_at = $11
            WHERE id = $12
            "#,
        )
        .bind(&block.kind)
        .bind(block.sequence)
        .bind(&block.raw_content)
        .bind(&block.display_content)
        .bind(&derived_content)
        .bind(actor_kind)
        .bind(actor_id)
        .bind(job_id)
        .bind(workflow_id)
        .bind(edit_event_id)
        .bind(now)
        .bind(&block.id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn delete_block(&self, ctx: &WriteContext, block_id: &str) -> StorageResult<()> {
        self.guard.validate_write(ctx, block_id).await?;
        let res = sqlx::query(r#"DELETE FROM blocks WHERE id = $1"#)
            .bind(block_id)
            .execute(&self.pool)
            .await?;
        if res.rows_affected() == 0 {
            return Err(StorageError::NotFound("block"));
        }
        Ok(())
    }

    async fn replace_blocks(
        &self,
        ctx: &WriteContext,
        document_id: &str,
        blocks: Vec<NewBlock>,
    ) -> StorageResult<Vec<Block>> {
        let mut tx = self.pool.begin().await?;
        sqlx::query(r#"DELETE FROM blocks WHERE document_id = $1"#)
            .bind(document_id)
            .execute(&mut *tx)
            .await?;

        let mut inserted = Vec::with_capacity(blocks.len());
        for block in blocks {
            let now = Utc::now();
            let id = block.id.unwrap_or_else(|| Uuid::now_v7().to_string());
            let metadata = self.guard.validate_write(ctx, &id).await?;
            let actor_kind = metadata.actor_kind.as_str();
            let actor_id = metadata.actor_id.clone();
            let job_id = metadata.job_id.map(|v| v.to_string());
            let workflow_id = metadata.workflow_id.map(|v| v.to_string());
            let edit_event_id = metadata.edit_event_id.to_string();
            let display_content = block
                .display_content
                .unwrap_or_else(|| block.raw_content.clone());
            let derived_content = block
                .derived_content
                .unwrap_or_else(|| Value::Object(Default::default()))
                .to_string();
            let exportable_int: Option<i32> = block.exportable.map(|v| if v { 1 } else { 0 });

            let row = sqlx::query(
                r#"
                INSERT INTO blocks (
                    id,
                    document_id,
                    kind,
                    sequence,
                    raw_content,
                    display_content,
                    derived_content,
                    created_at,
                    updated_at,
                    sensitivity,
                    exportable,
                    last_actor_kind,
                    last_actor_id,
                    last_job_id,
                    last_workflow_id,
                    edit_event_id
                )
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16)
                RETURNING id, document_id, kind, sequence, raw_content, display_content, derived_content,
                          created_at, updated_at, sensitivity, exportable
                "#,
            )
            .bind(&id)
            .bind(document_id)
            .bind(&block.kind)
            .bind(block.sequence)
            .bind(&block.raw_content)
            .bind(&display_content)
            .bind(&derived_content)
            .bind(now)
            .bind(now)
            .bind(&block.sensitivity)
            .bind(exportable_int)
            .bind(actor_kind)
            .bind(&actor_id)
            .bind(job_id)
            .bind(workflow_id)
            .bind(edit_event_id)
            .fetch_one(&mut *tx)
            .await?;

            inserted.push(map_block(row)?);
        }

        let doc_metadata = self.guard.validate_write(ctx, document_id).await?;
        let doc_actor_kind = doc_metadata.actor_kind.as_str();
        let doc_actor_id = doc_metadata.actor_id.clone();
        let doc_job_id = doc_metadata.job_id.map(|v| v.to_string());
        let doc_workflow_id = doc_metadata.workflow_id.map(|v| v.to_string());
        let doc_edit_event_id = doc_metadata.edit_event_id.to_string();
        let doc_updated_at = doc_metadata.timestamp;

        let updated = sqlx::query(
            r#"
            UPDATE documents
            SET last_actor_kind = $1,
                last_actor_id = $2,
                last_job_id = $3,
                last_workflow_id = $4,
                edit_event_id = $5,
                updated_at = $6
            WHERE id = $7
            "#,
        )
        .bind(doc_actor_kind)
        .bind(doc_actor_id)
        .bind(doc_job_id)
        .bind(doc_workflow_id)
        .bind(doc_edit_event_id)
        .bind(doc_updated_at)
        .bind(document_id)
        .execute(&mut *tx)
        .await?;

        if updated.rows_affected() == 0 {
            return Err(StorageError::NotFound("document"));
        }

        tx.commit().await?;
        Ok(inserted)
    }

    async fn create_asset(&self, ctx: &WriteContext, asset: NewAsset) -> StorageResult<Asset> {
        let now = Utc::now();
        let id = Uuid::now_v7().to_string();
        let metadata = self.guard.validate_write(ctx, &id).await?;
        let actor_kind = metadata.actor_kind.as_str();
        let actor_id = metadata.actor_id.clone();
        let job_id = metadata.job_id.map(|v| v.to_string());
        let workflow_id = metadata.workflow_id.map(|v| v.to_string());
        let edit_event_id = metadata.edit_event_id.to_string();

        let exportable: i32 = if asset.exportable { 1 } else { 0 };
        let width: Option<i32> = asset.width.map(|v| v as i32);
        let height: Option<i32> = asset.height.map(|v| v as i32);

        let row = sqlx::query(
            r#"
            INSERT INTO assets (
                asset_id,
                workspace_id,
                kind,
                mime,
                original_filename,
                content_hash,
                size_bytes,
                width,
                height,
                last_actor_kind,
                last_actor_id,
                last_job_id,
                last_workflow_id,
                edit_event_id,
                created_at,
                classification,
                exportable,
                is_proxy_of,
                proxy_asset_id
            )
            VALUES (
                $1, $2, $3, $4, $5, $6, $7, $8, $9,
                $10, $11, $12, $13, $14,
                $15, $16, $17, $18, $19
            )
            RETURNING
                asset_id,
                workspace_id,
                kind,
                mime,
                original_filename,
                content_hash,
                size_bytes,
                width,
                height,
                created_at,
                classification,
                exportable,
                is_proxy_of,
                proxy_asset_id
            "#,
        )
        .bind(&id)
        .bind(&asset.workspace_id)
        .bind(&asset.kind)
        .bind(&asset.mime)
        .bind(&asset.original_filename)
        .bind(&asset.content_hash)
        .bind(asset.size_bytes)
        .bind(width)
        .bind(height)
        .bind(actor_kind)
        .bind(actor_id)
        .bind(job_id)
        .bind(workflow_id)
        .bind(edit_event_id)
        .bind(now)
        .bind(&asset.classification)
        .bind(exportable)
        .bind(&asset.is_proxy_of)
        .bind(&asset.proxy_asset_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(map_asset(row))
    }

    async fn get_asset(&self, workspace_id: &str, asset_id: &str) -> StorageResult<Asset> {
        let row = sqlx::query(
            r#"
            SELECT
                asset_id,
                workspace_id,
                kind,
                mime,
                original_filename,
                content_hash,
                size_bytes,
                width,
                height,
                created_at,
                classification,
                exportable,
                is_proxy_of,
                proxy_asset_id
            FROM assets
            WHERE workspace_id = $1 AND asset_id = $2
            "#,
        )
        .bind(workspace_id)
        .bind(asset_id)
        .fetch_optional(&self.pool)
        .await?;

        match row {
            Some(row) => Ok(map_asset(row)),
            None => Err(StorageError::NotFound("asset")),
        }
    }

    async fn find_asset_by_content_hash(
        &self,
        workspace_id: &str,
        content_hash: &str,
    ) -> StorageResult<Option<Asset>> {
        let row = sqlx::query(
            r#"
            SELECT
                asset_id,
                workspace_id,
                kind,
                mime,
                original_filename,
                content_hash,
                size_bytes,
                width,
                height,
                created_at,
                classification,
                exportable,
                is_proxy_of,
                proxy_asset_id
            FROM assets
            WHERE workspace_id = $1 AND content_hash = $2
            "#,
        )
        .bind(workspace_id)
        .bind(content_hash)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(map_asset))
    }

    // ===== MT-259 MediaCacheTiers =====================================

    async fn upsert_media_tier(
        &self,
        ctx: &WriteContext,
        upsert: MediaTierUpsert,
    ) -> StorageResult<MediaAssetTier> {
        // validate_write enforces actor metadata; tiers are derived artifacts
        // so we key the guard on the asset_id being derived from.
        self.guard.validate_write(ctx, &upsert.asset_id).await?;
        let now = Utc::now();
        let id = Uuid::now_v7().to_string();
        let row = sqlx::query(
            r#"
            INSERT INTO media_asset_tiers (
                tier_row_id, workspace_id, asset_id, tier, status,
                tier_asset_id, content_hash, failure_reason,
                attempt_count, created_at, updated_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, 0, $9, $9)
            ON CONFLICT (asset_id, tier) DO UPDATE SET
                status = EXCLUDED.status,
                tier_asset_id = EXCLUDED.tier_asset_id,
                content_hash = EXCLUDED.content_hash,
                failure_reason = EXCLUDED.failure_reason,
                updated_at = EXCLUDED.updated_at
            RETURNING
                tier_row_id, workspace_id, asset_id, tier, status,
                tier_asset_id, content_hash, failure_reason,
                attempt_count, created_at, updated_at
            "#,
        )
        .bind(&id)
        .bind(&upsert.workspace_id)
        .bind(&upsert.asset_id)
        .bind(upsert.tier.as_str())
        .bind(upsert.status.as_str())
        .bind(&upsert.tier_asset_id)
        .bind(&upsert.content_hash)
        .bind(&upsert.failure_reason)
        .bind(now)
        .fetch_one(&self.pool)
        .await?;
        map_media_tier(row)
    }

    async fn set_media_tier_status(
        &self,
        ctx: &WriteContext,
        workspace_id: &str,
        asset_id: &str,
        tier: MediaTier,
        status: MediaTierStatus,
        failure_reason: Option<String>,
    ) -> StorageResult<MediaAssetTier> {
        self.guard.validate_write(ctx, asset_id).await?;
        let now = Utc::now();
        // A transition INTO `pending` from a non-pending state is a retry: bump
        // attempt_count so the retry queue accounting is real and visible.
        let row = sqlx::query(
            r#"
            UPDATE media_asset_tiers
            SET
                status = $1,
                failure_reason = $2,
                attempt_count = attempt_count + CASE
                    WHEN $1 = 'pending' AND status <> 'pending' THEN 1 ELSE 0 END,
                updated_at = $3
            WHERE workspace_id = $4 AND asset_id = $5 AND tier = $6
            RETURNING
                tier_row_id, workspace_id, asset_id, tier, status,
                tier_asset_id, content_hash, failure_reason,
                attempt_count, created_at, updated_at
            "#,
        )
        .bind(status.as_str())
        .bind(&failure_reason)
        .bind(now)
        .bind(workspace_id)
        .bind(asset_id)
        .bind(tier.as_str())
        .fetch_optional(&self.pool)
        .await?;
        match row {
            Some(row) => map_media_tier(row),
            None => Err(StorageError::NotFound("media_asset_tier")),
        }
    }

    async fn get_media_tier(
        &self,
        workspace_id: &str,
        asset_id: &str,
        tier: MediaTier,
    ) -> StorageResult<Option<MediaAssetTier>> {
        let row = sqlx::query(
            r#"
            SELECT
                tier_row_id, workspace_id, asset_id, tier, status,
                tier_asset_id, content_hash, failure_reason,
                attempt_count, created_at, updated_at
            FROM media_asset_tiers
            WHERE workspace_id = $1 AND asset_id = $2 AND tier = $3
            "#,
        )
        .bind(workspace_id)
        .bind(asset_id)
        .bind(tier.as_str())
        .fetch_optional(&self.pool)
        .await?;
        match row {
            Some(row) => Ok(Some(map_media_tier(row)?)),
            None => Ok(None),
        }
    }

    async fn list_media_tiers(
        &self,
        workspace_id: &str,
        asset_id: &str,
    ) -> StorageResult<Vec<MediaAssetTier>> {
        let rows = sqlx::query(
            r#"
            SELECT
                tier_row_id, workspace_id, asset_id, tier, status,
                tier_asset_id, content_hash, failure_reason,
                attempt_count, created_at, updated_at
            FROM media_asset_tiers
            WHERE workspace_id = $1 AND asset_id = $2
            ORDER BY tier
            "#,
        )
        .bind(workspace_id)
        .bind(asset_id)
        .fetch_all(&self.pool)
        .await?;
        rows.into_iter().map(map_media_tier).collect()
    }

    async fn list_failed_media_tiers(
        &self,
        workspace_id: &str,
    ) -> StorageResult<Vec<MediaAssetTier>> {
        let rows = sqlx::query(
            r#"
            SELECT
                tier_row_id, workspace_id, asset_id, tier, status,
                tier_asset_id, content_hash, failure_reason,
                attempt_count, created_at, updated_at
            FROM media_asset_tiers
            WHERE workspace_id = $1 AND status = 'failed'
            ORDER BY updated_at DESC
            "#,
        )
        .bind(workspace_id)
        .fetch_all(&self.pool)
        .await?;
        rows.into_iter().map(map_media_tier).collect()
    }

    async fn delete_media_tiers(
        &self,
        ctx: &WriteContext,
        workspace_id: &str,
        asset_id: &str,
    ) -> StorageResult<u64> {
        // Deleting derived tier rows NEVER touches the `assets` row or the
        // original blob on disk: this statement only deletes from
        // media_asset_tiers. The original asset is authority.
        self.guard.validate_write(ctx, asset_id).await?;
        let res = sqlx::query(
            r#"
            DELETE FROM media_asset_tiers
            WHERE workspace_id = $1 AND asset_id = $2
            "#,
        )
        .bind(workspace_id)
        .bind(asset_id)
        .execute(&self.pool)
        .await?;
        Ok(res.rows_affected())
    }

    // ===== MT-259 LoomCollections (GAP-LM-244a) =======================

    async fn create_loom_collection(
        &self,
        ctx: &WriteContext,
        workspace_id: &str,
        title: Option<String>,
    ) -> StorageResult<LoomCollection> {
        let id = Uuid::now_v7().to_string();
        self.guard.validate_write(ctx, &id).await?;
        let now = Utc::now();
        let row = sqlx::query(
            r#"
            INSERT INTO loom_collections (
                collection_id, workspace_id, title, created_at, updated_at
            )
            VALUES ($1, $2, $3, $4, $4)
            RETURNING collection_id, workspace_id, title, created_at, updated_at
            "#,
        )
        .bind(&id)
        .bind(workspace_id)
        .bind(&title)
        .bind(now)
        .fetch_one(&self.pool)
        .await?;
        Ok(map_loom_collection(row))
    }

    async fn get_loom_collection(
        &self,
        workspace_id: &str,
        collection_id: &str,
    ) -> StorageResult<LoomCollectionWithMembers> {
        let row = sqlx::query(
            r#"
            SELECT collection_id, workspace_id, title, created_at, updated_at
            FROM loom_collections
            WHERE workspace_id = $1 AND collection_id = $2
            "#,
        )
        .bind(workspace_id)
        .bind(collection_id)
        .fetch_optional(&self.pool)
        .await?;
        let collection = match row {
            Some(row) => map_loom_collection(row),
            None => return Err(StorageError::NotFound("loom_collection")),
        };
        let member_rows = sqlx::query(
            r#"
            SELECT asset_id, position
            FROM loom_collection_members
            WHERE collection_id = $1
            ORDER BY position
            "#,
        )
        .bind(collection_id)
        .fetch_all(&self.pool)
        .await?;
        let members = member_rows
            .into_iter()
            .map(|r| LoomCollectionMember {
                asset_id: r.get("asset_id"),
                position: r.get("position"),
            })
            .collect();
        Ok(LoomCollectionWithMembers {
            collection,
            members,
        })
    }

    async fn set_loom_collection_order(
        &self,
        ctx: &WriteContext,
        workspace_id: &str,
        collection_id: &str,
        asset_ids: &[String],
    ) -> StorageResult<LoomCollectionWithMembers> {
        self.guard.validate_write(ctx, collection_id).await?;
        let now = Utc::now();
        let mut tx = self.pool.begin().await?;

        // Confirm collection ownership inside the transaction.
        let exists: Option<(String,)> = sqlx::query_as(
            r#"
            SELECT collection_id FROM loom_collections
            WHERE workspace_id = $1 AND collection_id = $2
            "#,
        )
        .bind(workspace_id)
        .bind(collection_id)
        .fetch_optional(&mut *tx)
        .await?;
        if exists.is_none() {
            return Err(StorageError::NotFound("loom_collection"));
        }

        // Replace the ordered member set: clear then re-insert dense 0..n.
        // The UNIQUE(collection_id, position) is DEFERRABLE so reorder within a
        // transaction never trips on transient duplicate positions.
        sqlx::query("DELETE FROM loom_collection_members WHERE collection_id = $1")
            .bind(collection_id)
            .execute(&mut *tx)
            .await?;
        for (idx, asset_id) in asset_ids.iter().enumerate() {
            sqlx::query(
                r#"
                INSERT INTO loom_collection_members (collection_id, asset_id, position)
                VALUES ($1, $2, $3)
                "#,
            )
            .bind(collection_id)
            .bind(asset_id)
            .bind(idx as i32)
            .execute(&mut *tx)
            .await?;
        }
        sqlx::query("UPDATE loom_collections SET updated_at = $1 WHERE collection_id = $2")
            .bind(now)
            .bind(collection_id)
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;

        self.get_loom_collection(workspace_id, collection_id).await
    }

    async fn create_loom_block(
        &self,
        ctx: &WriteContext,
        block: NewLoomBlock,
    ) -> StorageResult<LoomBlock> {
        let now = Utc::now();
        let id = block
            .block_id
            .map_or_else(|| Uuid::now_v7().to_string(), |v| v);
        // MT-177: the block id becomes the ProjectKnowledgeIndex entity_key
        // (knowledge_entities.entity_key has a `btrim = value AND <> ''`
        // CHECK). Reject an un-bridgeable id at the source so a LoomBlock can
        // never exist that the authority bridge would later refuse — which
        // would otherwise leave an orphan block outside Postgres/EventLedger
        // authority (§10.12 #9.1.1).
        if id.trim().is_empty() || id.trim() != id {
            return Err(StorageError::Validation(
                "loom block_id must be non-empty without surrounding whitespace",
            ));
        }
        let metadata = self.guard.validate_write(ctx, &id).await?;
        let actor_kind = metadata.actor_kind.as_str();
        let actor_id = metadata.actor_id.clone();
        let job_id = metadata.job_id.map(|v| v.to_string());
        let workflow_id = metadata.workflow_id.map(|v| v.to_string());
        let edit_event_id = metadata.edit_event_id.to_string();

        let derived_json = serde_json::to_string(&block.derived)?;
        let preview_status = block.derived.preview_status.as_str();

        let pinned: i32 = if block.pinned { 1 } else { 0 };

        let row = sqlx::query(
            r#"
            INSERT INTO loom_blocks (
                block_id,
                workspace_id,
                content_type,
                document_id,
                asset_id,
                title,
                original_filename,
                content_hash,
                pinned,
                journal_date,
                last_actor_kind,
                last_actor_id,
                last_job_id,
                last_workflow_id,
                edit_event_id,
                created_at,
                updated_at,
                imported_at,
                backlink_count,
                mention_count,
                tag_count,
                derived_json,
                preview_status,
                thumbnail_asset_id,
                proxy_asset_id
            )
            VALUES (
                $1, $2, $3, $4, $5, $6, $7, $8, $9, $10,
                $11, $12, $13, $14, $15,
                $16, $17, $18,
                $19, $20, $21,
                $22, $23, $24, $25
            )
            RETURNING
                block_id,
                workspace_id,
                content_type,
                document_id,
                asset_id,
                title,
                original_filename,
                content_hash,
                pinned,
                favorite,
                journal_date,
                created_at,
                updated_at,
                imported_at,
                backlink_count,
                mention_count,
                tag_count,
                derived_json,
                preview_status,
                thumbnail_asset_id,
                proxy_asset_id
            "#,
        )
        .bind(&id)
        .bind(&block.workspace_id)
        .bind(block.content_type.as_str())
        .bind(&block.document_id)
        .bind(&block.asset_id)
        .bind(&block.title)
        .bind(&block.original_filename)
        .bind(&block.content_hash)
        .bind(pinned)
        .bind(&block.journal_date)
        .bind(actor_kind)
        .bind(actor_id)
        .bind(job_id)
        .bind(workflow_id)
        .bind(edit_event_id)
        .bind(now)
        .bind(now)
        .bind(block.imported_at)
        .bind(block.derived.backlink_count as i32)
        .bind(block.derived.mention_count as i32)
        .bind(block.derived.tag_count as i32)
        .bind(derived_json)
        .bind(preview_status)
        .bind(&block.derived.thumbnail_asset_id)
        .bind(&block.derived.proxy_asset_id)
        .fetch_one(&self.pool)
        .await?;

        map_loom_block(row)
    }

    async fn get_or_create_daily_journal_block(
        &self,
        ctx: &WriteContext,
        workspace_id: &str,
        journal_date: &str,
    ) -> StorageResult<LoomBlock> {
        let now = Utc::now();
        let id = Uuid::now_v7().to_string();
        let write_key = format!("{workspace_id}:journal:{journal_date}");
        let metadata = self.guard.validate_write(ctx, &write_key).await?;
        let actor_kind = metadata.actor_kind.as_str();
        let actor_id = metadata.actor_id.clone();
        let job_id = metadata.job_id.map(|v| v.to_string());
        let workflow_id = metadata.workflow_id.map(|v| v.to_string());
        let edit_event_id = metadata.edit_event_id.to_string();
        let title = format!("Daily Note {journal_date}");
        let mut derived = LoomBlockDerived::default();
        derived.full_text_index = Some(format!("# {title}\n\n"));
        let derived_json = serde_json::to_string(&derived)?;
        let preview_status = derived.preview_status.as_str();

        let row = sqlx::query(
            r#"
            WITH inserted AS (
                INSERT INTO loom_blocks (
                    block_id,
                    workspace_id,
                    content_type,
                    document_id,
                    asset_id,
                    title,
                    original_filename,
                    content_hash,
                    pinned,
                    journal_date,
                    last_actor_kind,
                    last_actor_id,
                    last_job_id,
                    last_workflow_id,
                    edit_event_id,
                    created_at,
                    updated_at,
                    imported_at,
                    backlink_count,
                    mention_count,
                    tag_count,
                    derived_json,
                    preview_status,
                    thumbnail_asset_id,
                    proxy_asset_id
                )
                VALUES (
                    $1, $2, $3, NULL, NULL, $4, NULL, NULL, 0, $5,
                    $6, $7, $8, $9, $10,
                    $11, $11, NULL,
                    0, 0, 0,
                    $12, $13, NULL, NULL
                )
                ON CONFLICT (workspace_id, journal_date)
                    WHERE content_type = 'journal' AND journal_date IS NOT NULL
                DO NOTHING
                RETURNING
                    block_id,
                    workspace_id,
                    content_type,
                    document_id,
                    asset_id,
                    title,
                    original_filename,
                    content_hash,
                    pinned,
                    favorite,
                    pin_order,
                    journal_date,
                    created_at,
                    updated_at,
                    imported_at,
                    backlink_count,
                    mention_count,
                    tag_count,
                    derived_json,
                    preview_status,
                    thumbnail_asset_id,
                    proxy_asset_id
            )
            SELECT
                block_id,
                workspace_id,
                content_type,
                document_id,
                asset_id,
                title,
                original_filename,
                content_hash,
                pinned,
                favorite,
                pin_order,
                journal_date,
                created_at,
                updated_at,
                imported_at,
                backlink_count,
                mention_count,
                tag_count,
                derived_json,
                preview_status,
                thumbnail_asset_id,
                proxy_asset_id
            FROM inserted
            UNION ALL
            SELECT
                block_id,
                workspace_id,
                content_type,
                document_id,
                asset_id,
                title,
                original_filename,
                content_hash,
                pinned,
                favorite,
                pin_order,
                journal_date,
                created_at,
                updated_at,
                imported_at,
                backlink_count,
                mention_count,
                tag_count,
                derived_json,
                preview_status,
                thumbnail_asset_id,
                proxy_asset_id
            FROM loom_blocks
            WHERE workspace_id = $2
              AND content_type = 'journal'
              AND journal_date = $5
              AND NOT EXISTS (SELECT 1 FROM inserted)
            LIMIT 1
            "#,
        )
        .bind(&id)
        .bind(workspace_id)
        .bind(LoomBlockContentType::Journal.as_str())
        .bind(title)
        .bind(journal_date)
        .bind(actor_kind)
        .bind(actor_id)
        .bind(job_id)
        .bind(workflow_id)
        .bind(edit_event_id)
        .bind(now)
        .bind(derived_json)
        .bind(preview_status)
        .fetch_one(&self.pool)
        .await?;

        map_loom_block(row)
    }

    async fn get_loom_block(&self, workspace_id: &str, block_id: &str) -> StorageResult<LoomBlock> {
        let row = sqlx::query(
            r#"
            SELECT
                block_id,
                workspace_id,
                content_type,
                document_id,
                asset_id,
                title,
                original_filename,
                content_hash,
                pinned,
                favorite,
                pin_order,
                journal_date,
                created_at,
                updated_at,
                imported_at,
                backlink_count,
                mention_count,
                tag_count,
                derived_json,
                preview_status,
                thumbnail_asset_id,
                proxy_asset_id
            FROM loom_blocks
            WHERE workspace_id = $1 AND block_id = $2
            "#,
        )
        .bind(workspace_id)
        .bind(block_id)
        .fetch_optional(&self.pool)
        .await?;

        match row {
            Some(row) => map_loom_block(row),
            None => Err(StorageError::NotFound("loom_block")),
        }
    }

    async fn find_loom_block_by_content_hash(
        &self,
        workspace_id: &str,
        content_hash: &str,
    ) -> StorageResult<Option<LoomBlock>> {
        let row = sqlx::query(
            r#"
            SELECT
                block_id,
                workspace_id,
                content_type,
                document_id,
                asset_id,
                title,
                original_filename,
                content_hash,
                pinned,
                favorite,
                journal_date,
                created_at,
                updated_at,
                imported_at,
                backlink_count,
                mention_count,
                tag_count,
                derived_json,
                preview_status,
                thumbnail_asset_id,
                proxy_asset_id
            FROM loom_blocks
            WHERE workspace_id = $1 AND content_hash = $2
            "#,
        )
        .bind(workspace_id)
        .bind(content_hash)
        .fetch_optional(&self.pool)
        .await?;

        match row {
            Some(row) => Ok(Some(map_loom_block(row)?)),
            None => Ok(None),
        }
    }

    async fn find_loom_block_by_asset_id(
        &self,
        workspace_id: &str,
        asset_id: &str,
    ) -> StorageResult<Option<LoomBlock>> {
        let row = sqlx::query(
            r#"
            SELECT
                block_id,
                workspace_id,
                content_type,
                document_id,
                asset_id,
                title,
                original_filename,
                content_hash,
                pinned,
                favorite,
                journal_date,
                created_at,
                updated_at,
                imported_at,
                backlink_count,
                mention_count,
                tag_count,
                derived_json,
                preview_status,
                thumbnail_asset_id,
                proxy_asset_id
            FROM loom_blocks
            WHERE workspace_id = $1 AND asset_id = $2
            ORDER BY updated_at DESC
            LIMIT 1
            "#,
        )
        .bind(workspace_id)
        .bind(asset_id)
        .fetch_optional(&self.pool)
        .await?;

        match row {
            Some(row) => Ok(Some(map_loom_block(row)?)),
            None => Ok(None),
        }
    }

    async fn update_loom_block(
        &self,
        ctx: &WriteContext,
        workspace_id: &str,
        block_id: &str,
        update: LoomBlockUpdate,
    ) -> StorageResult<LoomBlock> {
        let now = Utc::now();
        let metadata = self.guard.validate_write(ctx, block_id).await?;
        let actor_kind = metadata.actor_kind.as_str();
        let actor_id = metadata.actor_id.clone();
        let job_id = metadata.job_id.map(|v| v.to_string());
        let workflow_id = metadata.workflow_id.map(|v| v.to_string());
        let edit_event_id = metadata.edit_event_id.to_string();

        let pinned: Option<i32> = update.pinned.map(|v| if v { 1 } else { 0 });
        let favorite: Option<i32> = update.favorite.map(|v| if v { 1 } else { 0 });

        let row = sqlx::query(
            r#"
            UPDATE loom_blocks
            SET
                title = COALESCE($1, title),
                pinned = COALESCE($2, pinned),
                journal_date = COALESCE($3, journal_date),
                pin_order = COALESCE($4, pin_order),
                favorite = COALESCE($5, favorite),
                last_actor_kind = $6,
                last_actor_id = $7,
                last_job_id = $8,
                last_workflow_id = $9,
                edit_event_id = $10,
                updated_at = $11
            WHERE workspace_id = $12 AND block_id = $13
            RETURNING
                block_id,
                workspace_id,
                content_type,
                document_id,
                asset_id,
                title,
                original_filename,
                content_hash,
                pinned,
                favorite,
                pin_order,
                journal_date,
                created_at,
                updated_at,
                imported_at,
                backlink_count,
                mention_count,
                tag_count,
                derived_json,
                preview_status,
                thumbnail_asset_id,
                proxy_asset_id
            "#,
        )
        .bind(update.title)
        .bind(pinned)
        .bind(update.journal_date)
        .bind(update.pin_order)
        .bind(favorite)
        .bind(actor_kind)
        .bind(actor_id)
        .bind(job_id)
        .bind(workflow_id)
        .bind(edit_event_id)
        .bind(now)
        .bind(workspace_id)
        .bind(block_id)
        .fetch_optional(&self.pool)
        .await?;

        match row {
            Some(row) => map_loom_block(row),
            None => Err(StorageError::NotFound("loom_block")),
        }
    }

    async fn set_loom_block_preview(
        &self,
        ctx: &WriteContext,
        workspace_id: &str,
        block_id: &str,
        preview_status: PreviewStatus,
        thumbnail_asset_id: Option<String>,
        proxy_asset_id: Option<String>,
    ) -> StorageResult<()> {
        let now = Utc::now();
        let metadata = self.guard.validate_write(ctx, block_id).await?;
        let actor_kind = metadata.actor_kind.as_str();
        let actor_id = metadata.actor_id.clone();
        let job_id = metadata.job_id.map(|v| v.to_string());
        let workflow_id = metadata.workflow_id.map(|v| v.to_string());
        let edit_event_id = metadata.edit_event_id.to_string();

        let res = sqlx::query(
            r#"
            UPDATE loom_blocks
            SET
                preview_status = $1,
                thumbnail_asset_id = $2,
                proxy_asset_id = $3,
                last_actor_kind = $4,
                last_actor_id = $5,
                last_job_id = $6,
                last_workflow_id = $7,
                edit_event_id = $8,
                updated_at = $9
            WHERE workspace_id = $10 AND block_id = $11
            "#,
        )
        .bind(preview_status.as_str())
        .bind(thumbnail_asset_id)
        .bind(proxy_asset_id)
        .bind(actor_kind)
        .bind(actor_id)
        .bind(job_id)
        .bind(workflow_id)
        .bind(edit_event_id)
        .bind(now)
        .bind(workspace_id)
        .bind(block_id)
        .execute(&self.pool)
        .await?;

        if res.rows_affected() == 0 {
            return Err(StorageError::NotFound("loom_block"));
        }
        Ok(())
    }

    async fn delete_loom_block(
        &self,
        ctx: &WriteContext,
        workspace_id: &str,
        block_id: &str,
    ) -> StorageResult<()> {
        self.guard.validate_write(ctx, block_id).await?;
        let mut tx = self.pool.begin().await?;
        let affected_rows: Vec<(String, String)> = sqlx::query_as(
            r#"
            SELECT source_block_id, target_block_id
            FROM loom_edges
            WHERE workspace_id = $1
              AND (source_block_id = $2 OR target_block_id = $2)
            "#,
        )
        .bind(workspace_id)
        .bind(block_id)
        .fetch_all(&mut *tx)
        .await?;
        let affected_block_ids: BTreeSet<String> = affected_rows
            .into_iter()
            .flat_map(|(source_block_id, target_block_id)| [source_block_id, target_block_id])
            .filter(|candidate| candidate != block_id)
            .collect();

        let res = sqlx::query(
            r#"
            DELETE FROM loom_blocks
            WHERE workspace_id = $1 AND block_id = $2
            "#,
        )
        .bind(workspace_id)
        .bind(block_id)
        .execute(&mut *tx)
        .await?;

        if res.rows_affected() == 0 {
            return Err(StorageError::NotFound("loom_block"));
        }

        for affected_block_id in affected_block_ids {
            sqlx::query(
                r#"
                UPDATE loom_blocks
                SET
                    mention_count = (SELECT COUNT(*)::INT FROM loom_edges WHERE workspace_id = $1 AND source_block_id = $2 AND edge_type = 'mention'),
                    tag_count = (SELECT COUNT(*)::INT FROM loom_edges WHERE workspace_id = $1 AND source_block_id = $2 AND edge_type = 'tag'),
                    backlink_count = (SELECT COUNT(*)::INT FROM loom_edges WHERE workspace_id = $1 AND target_block_id = $2 AND edge_type IN ('mention', 'tag'))
                WHERE workspace_id = $1 AND block_id = $2
                "#,
            )
            .bind(workspace_id)
            .bind(&affected_block_id)
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        Ok(())
    }

    async fn create_loom_edge(
        &self,
        ctx: &WriteContext,
        edge: NewLoomEdge,
    ) -> StorageResult<LoomEdge> {
        let mut tx = self.pool.begin().await?;
        let now = Utc::now();
        let id = edge
            .edge_id
            .map_or_else(|| Uuid::now_v7().to_string(), |v| v);
        let metadata = self.guard.validate_write(ctx, &id).await?;
        let actor_kind = metadata.actor_kind.as_str();
        let actor_id = metadata.actor_id.clone();
        let job_id = metadata.job_id.map(|v| v.to_string());
        let workflow_id = metadata.workflow_id.map(|v| v.to_string());
        let edit_event_id = metadata.edit_event_id.to_string();

        let (source_document_id, source_text_block_id, offset_start, offset_end) =
            match edge.source_anchor.clone() {
                Some(anchor) => (
                    Some(anchor.document_id),
                    Some(anchor.block_id),
                    Some(anchor.offset_start as i32),
                    Some(anchor.offset_end as i32),
                ),
                None => (None, None, None, None),
            };

        let row = sqlx::query(
            r#"
            INSERT INTO loom_edges (
                edge_id,
                workspace_id,
                source_block_id,
                target_block_id,
                edge_type,
                created_by,
                last_actor_kind,
                last_actor_id,
                last_job_id,
                last_workflow_id,
                edit_event_id,
                created_at,
                crdt_site_id,
                source_document_id,
                source_text_block_id,
                offset_start,
                offset_end
            )
            VALUES (
                $1, $2, $3, $4, $5, $6,
                $7, $8, $9, $10, $11,
                $12, $13, $14, $15, $16, $17
            )
            RETURNING
                edge_id,
                workspace_id,
                source_block_id,
                target_block_id,
                edge_type,
                created_by,
                created_at,
                crdt_site_id,
                source_document_id,
                source_text_block_id,
                offset_start,
                offset_end
            "#,
        )
        .bind(&id)
        .bind(&edge.workspace_id)
        .bind(&edge.source_block_id)
        .bind(&edge.target_block_id)
        .bind(edge.edge_type.as_str())
        .bind(edge.created_by.as_str())
        .bind(actor_kind)
        .bind(actor_id)
        .bind(job_id)
        .bind(workflow_id)
        .bind(edit_event_id)
        .bind(now)
        .bind(edge.crdt_site_id)
        .bind(source_document_id)
        .bind(source_text_block_id)
        .bind(offset_start)
        .bind(offset_end)
        .fetch_one(&mut *tx)
        .await?;

        if matches!(edge.edge_type, LoomEdgeType::Mention | LoomEdgeType::Tag) {
            for block_id in [&edge.source_block_id, &edge.target_block_id] {
                sqlx::query(
                    r#"
                    UPDATE loom_blocks
                    SET
                        mention_count = (SELECT COUNT(*)::INT FROM loom_edges WHERE workspace_id = $1 AND source_block_id = $2 AND edge_type = 'mention'),
                        tag_count = (SELECT COUNT(*)::INT FROM loom_edges WHERE workspace_id = $1 AND source_block_id = $2 AND edge_type = 'tag'),
                        backlink_count = (SELECT COUNT(*)::INT FROM loom_edges WHERE workspace_id = $1 AND target_block_id = $2 AND edge_type IN ('mention', 'tag'))
                    WHERE workspace_id = $1 AND block_id = $2
                    "#,
                )
                .bind(&edge.workspace_id)
                .bind(block_id)
                .execute(&mut *tx)
                .await?;
            }
        }

        tx.commit().await?;
        map_loom_edge(row)
    }

    async fn delete_loom_edge(
        &self,
        ctx: &WriteContext,
        workspace_id: &str,
        edge_id: &str,
    ) -> StorageResult<LoomEdge> {
        let mut tx = self.pool.begin().await?;
        self.guard.validate_write(ctx, edge_id).await?;

        let existing = sqlx::query(
            r#"
            SELECT
                edge_id,
                workspace_id,
                source_block_id,
                target_block_id,
                edge_type,
                created_by,
                created_at,
                crdt_site_id,
                source_document_id,
                source_text_block_id,
                offset_start,
                offset_end
            FROM loom_edges
            WHERE workspace_id = $1 AND edge_id = $2
            "#,
        )
        .bind(workspace_id)
        .bind(edge_id)
        .fetch_optional(&mut *tx)
        .await?;

        let existing = existing.ok_or(StorageError::NotFound("loom_edge"))?;
        let mapped_existing = map_loom_edge(existing)?;

        sqlx::query(
            r#"
            DELETE FROM loom_edges
            WHERE workspace_id = $1 AND edge_id = $2
            "#,
        )
        .bind(workspace_id)
        .bind(edge_id)
        .execute(&mut *tx)
        .await?;

        if matches!(
            mapped_existing.edge_type,
            LoomEdgeType::Mention | LoomEdgeType::Tag
        ) {
            for block_id in [
                &mapped_existing.source_block_id,
                &mapped_existing.target_block_id,
            ] {
                sqlx::query(
                    r#"
                    UPDATE loom_blocks
                    SET
                        mention_count = (SELECT COUNT(*)::INT FROM loom_edges WHERE workspace_id = $1 AND source_block_id = $2 AND edge_type = 'mention'),
                        tag_count = (SELECT COUNT(*)::INT FROM loom_edges WHERE workspace_id = $1 AND source_block_id = $2 AND edge_type = 'tag'),
                        backlink_count = (SELECT COUNT(*)::INT FROM loom_edges WHERE workspace_id = $1 AND target_block_id = $2 AND edge_type IN ('mention', 'tag'))
                    WHERE workspace_id = $1 AND block_id = $2
                    "#,
                )
                .bind(workspace_id)
                .bind(block_id)
                .execute(&mut *tx)
                .await?;
            }
        }

        tx.commit().await?;
        Ok(mapped_existing)
    }

    async fn list_loom_edges_for_block(
        &self,
        workspace_id: &str,
        block_id: &str,
    ) -> StorageResult<Vec<LoomEdge>> {
        let rows = sqlx::query(
            r#"
            SELECT
                edge_id,
                workspace_id,
                source_block_id,
                target_block_id,
                edge_type,
                created_by,
                created_at,
                crdt_site_id,
                source_document_id,
                source_text_block_id,
                offset_start,
                offset_end
            FROM loom_edges
            WHERE workspace_id = $1
              AND (source_block_id = $2 OR target_block_id = $2)
            ORDER BY created_at ASC
            "#,
        )
        .bind(workspace_id)
        .bind(block_id)
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter().map(map_loom_edge).collect()
    }

    async fn get_backlinks(
        &self,
        workspace_id: &str,
        block_id: &str,
    ) -> StorageResult<Vec<LoomEdge>> {
        let rows = sqlx::query(
            r#"
            SELECT
                edge_id,
                workspace_id,
                source_block_id,
                target_block_id,
                edge_type,
                created_by,
                created_at,
                crdt_site_id,
                source_document_id,
                source_text_block_id,
                offset_start,
                offset_end
            FROM loom_edges
            WHERE workspace_id = $1
              AND target_block_id = $2
            ORDER BY created_at ASC, edge_id ASC
            "#,
        )
        .bind(workspace_id)
        .bind(block_id)
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter().map(map_loom_edge).collect()
    }

    async fn get_outgoing_edges(
        &self,
        workspace_id: &str,
        block_id: &str,
    ) -> StorageResult<Vec<LoomEdge>> {
        let rows = sqlx::query(
            r#"
            SELECT
                edge_id,
                workspace_id,
                source_block_id,
                target_block_id,
                edge_type,
                created_by,
                created_at,
                crdt_site_id,
                source_document_id,
                source_text_block_id,
                offset_start,
                offset_end
            FROM loom_edges
            WHERE workspace_id = $1
              AND source_block_id = $2
            ORDER BY created_at ASC, edge_id ASC
            "#,
        )
        .bind(workspace_id)
        .bind(block_id)
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter().map(map_loom_edge).collect()
    }

    async fn traverse_graph(
        &self,
        workspace_id: &str,
        start_block_id: &str,
        max_depth: u32,
        edge_types: &[LoomEdgeType],
    ) -> StorageResult<Vec<(LoomBlock, u32)>> {
        if max_depth == 0 {
            return Ok(Vec::new());
        }

        let edge_type_filter = (!edge_types.is_empty()).then(|| {
            edge_types
                .iter()
                .map(|edge_type| edge_type.as_str().to_string())
                .collect::<Vec<_>>()
        });

        let rows = sqlx::query(
            r#"
            WITH RECURSIVE reachable(block_id, depth, path) AS (
                SELECT
                    e.target_block_id,
                    1 AS depth,
                    ARRAY[e.source_block_id, e.target_block_id]::TEXT[] AS path
                FROM loom_edges e
                WHERE e.workspace_id = $1
                  AND e.source_block_id = $2
                  AND ($4::TEXT[] IS NULL OR e.edge_type = ANY($4::TEXT[]))

                UNION ALL

                SELECT
                    e.target_block_id,
                    r.depth + 1,
                    r.path || e.target_block_id
                FROM loom_edges e
                JOIN reachable r
                  ON e.source_block_id = r.block_id
                WHERE e.workspace_id = $1
                  AND r.depth < $3
                  AND NOT e.target_block_id = ANY(r.path)
                  AND ($4::TEXT[] IS NULL OR e.edge_type = ANY($4::TEXT[]))
            ),
            dedup AS (
                SELECT block_id, MIN(depth) AS depth
                FROM reachable
                GROUP BY block_id
            )
            SELECT
                d.depth,
                b.block_id,
                b.workspace_id,
                b.content_type,
                b.document_id,
                b.asset_id,
                b.title,
                b.original_filename,
                b.content_hash,
                b.pinned,
                b.favorite,
                b.journal_date,
                b.created_at,
                b.updated_at,
                b.imported_at,
                b.backlink_count,
                b.mention_count,
                b.tag_count,
                b.derived_json,
                b.preview_status,
                b.thumbnail_asset_id,
                b.proxy_asset_id
            FROM dedup d
            JOIN loom_blocks b
              ON b.workspace_id = $1
             AND b.block_id = d.block_id
            ORDER BY d.depth ASC, b.block_id ASC
            "#,
        )
        .bind(workspace_id)
        .bind(start_block_id)
        .bind(max_depth as i32)
        .bind(edge_type_filter)
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter()
            .map(|row| {
                let depth: i32 = row.get("depth");
                let depth = u32::try_from(depth)
                    .map_err(|_| StorageError::Validation("invalid loom traversal depth"))?;
                let block = map_loom_block(row)?;
                Ok((block, depth))
            })
            .collect()
    }

    async fn recompute_block_metrics(
        &self,
        workspace_id: &str,
        block_id: &str,
    ) -> StorageResult<()> {
        let res = sqlx::query(
            r#"
            UPDATE loom_blocks
            SET
                mention_count = (
                    SELECT COUNT(*)::INT
                    FROM loom_edges
                    WHERE workspace_id = $1
                      AND source_block_id = $2
                      AND edge_type = 'mention'
                ),
                tag_count = (
                    SELECT COUNT(*)::INT
                    FROM loom_edges
                    WHERE workspace_id = $1
                      AND source_block_id = $2
                      AND edge_type = 'tag'
                ),
                backlink_count = (
                    SELECT COUNT(*)::INT
                    FROM loom_edges
                    WHERE workspace_id = $1
                      AND target_block_id = $2
                      AND edge_type IN ('mention', 'tag')
                )
            WHERE workspace_id = $1
              AND block_id = $2
            "#,
        )
        .bind(workspace_id)
        .bind(block_id)
        .execute(&self.pool)
        .await?;

        if res.rows_affected() == 0 {
            return Err(StorageError::NotFound("loom_block"));
        }

        Ok(())
    }

    async fn recompute_all_metrics(&self, workspace_id: &str) -> StorageResult<()> {
        sqlx::query(
            r#"
            UPDATE loom_blocks AS b
            SET
                mention_count = (
                    SELECT COUNT(*)::INT
                    FROM loom_edges e
                    WHERE e.workspace_id = b.workspace_id
                      AND e.source_block_id = b.block_id
                      AND e.edge_type = 'mention'
                ),
                tag_count = (
                    SELECT COUNT(*)::INT
                    FROM loom_edges e
                    WHERE e.workspace_id = b.workspace_id
                      AND e.source_block_id = b.block_id
                      AND e.edge_type = 'tag'
                ),
                backlink_count = (
                    SELECT COUNT(*)::INT
                    FROM loom_edges e
                    WHERE e.workspace_id = b.workspace_id
                      AND e.target_block_id = b.block_id
                      AND e.edge_type IN ('mention', 'tag')
                )
            WHERE b.workspace_id = $1
            "#,
        )
        .bind(workspace_id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn query_loom_view(
        &self,
        workspace_id: &str,
        view_type: LoomViewType,
        filters: LoomViewFilters,
        limit: u32,
        offset: u32,
    ) -> StorageResult<LoomViewResponse> {
        let limit_i64 = limit as i64;
        let offset_i64 = offset as i64;
        let select_filters = filters.clone();

        let select_blocks = |extra_where: Option<&'static str>, order_by: &'static str| {
            let filters = select_filters.clone();
            async move {
                let mut qb = sqlx::QueryBuilder::<sqlx::Postgres>::new(
                    r#"
                SELECT
                    b.block_id,
                    b.workspace_id,
                    b.content_type,
                    b.document_id,
                    b.asset_id,
                    b.title,
                    b.original_filename,
                    b.content_hash,
                    b.pinned,
                    b.favorite,
                    b.pin_order,
                    b.journal_date,
                    b.created_at,
                    b.updated_at,
                    b.imported_at,
                    b.backlink_count,
                    b.mention_count,
                    b.tag_count,
                    b.derived_json,
                    b.preview_status,
                    b.thumbnail_asset_id,
                    b.proxy_asset_id
                FROM loom_blocks b
                LEFT JOIN assets a
                  ON a.workspace_id = b.workspace_id AND a.asset_id = b.asset_id
                "#,
                );

                let mut has_where = false;
                let mut push_clause = |builder: &mut sqlx::QueryBuilder<sqlx::Postgres>| {
                    if has_where {
                        builder.push(" AND ");
                    } else {
                        builder.push(" WHERE ");
                        has_where = true;
                    }
                };

                push_clause(&mut qb);
                qb.push("b.workspace_id = ").push_bind(workspace_id);

                if let Some(extra) = extra_where {
                    push_clause(&mut qb);
                    qb.push(extra);
                }

                if let Some(content_type) = filters.content_type {
                    push_clause(&mut qb);
                    qb.push("b.content_type = ")
                        .push_bind(content_type.as_str());
                }

                if let Some(mime) = filters.mime {
                    push_clause(&mut qb);
                    qb.push("a.mime = ").push_bind(mime);
                }

                if let Some(from) = filters.date_from {
                    push_clause(&mut qb);
                    qb.push(
                        "(CASE WHEN b.content_type = 'journal' AND b.journal_date IS NOT NULL \
                         THEN b.journal_date >= to_char(",
                    )
                    .push_bind(from)
                    .push("::timestamptz, 'YYYY-MM-DD') ELSE b.updated_at >= ")
                    .push_bind(from)
                    .push(" END)");
                }
                if let Some(to) = filters.date_to {
                    push_clause(&mut qb);
                    qb.push(
                        "(CASE WHEN b.content_type = 'journal' AND b.journal_date IS NOT NULL \
                         THEN b.journal_date <= to_char(",
                    )
                    .push_bind(to)
                    .push("::timestamptz, 'YYYY-MM-DD') ELSE b.updated_at <= ")
                    .push_bind(to)
                    .push(" END)");
                }

                if !filters.tag_ids.is_empty() {
                    push_clause(&mut qb);
                    qb.push(
                    "EXISTS (SELECT 1 FROM loom_edges e WHERE e.workspace_id = b.workspace_id AND e.source_block_id = b.block_id AND e.edge_type = 'tag' AND e.target_block_id IN (",
                );
                    let mut separated = qb.separated(", ");
                    for tag_id in &filters.tag_ids {
                        separated.push_bind(tag_id);
                    }
                    separated.push_unseparated("))");
                }

                if !filters.mention_ids.is_empty() {
                    push_clause(&mut qb);
                    qb.push(
                    "EXISTS (SELECT 1 FROM loom_edges e WHERE e.workspace_id = b.workspace_id AND e.source_block_id = b.block_id AND e.edge_type = 'mention' AND e.target_block_id IN (",
                );
                    let mut separated = qb.separated(", ");
                    for mention_id in &filters.mention_ids {
                        separated.push_bind(mention_id);
                    }
                    separated.push_unseparated("))");
                }

                qb.push(" ");
                qb.push(order_by);
                qb.push(" LIMIT ").push_bind(limit_i64);
                qb.push(" OFFSET ").push_bind(offset_i64);

                let rows = qb.build().fetch_all(&self.pool).await?;
                let blocks: Vec<LoomBlock> = rows
                    .into_iter()
                    .map(map_loom_block)
                    .collect::<StorageResult<Vec<_>>>()?;
                Ok::<_, StorageError>(blocks)
            }
        };

        match view_type {
            LoomViewType::All => {
                let blocks =
                    select_blocks(None, "ORDER BY b.updated_at DESC, b.block_id ASC").await?;
                Ok(LoomViewResponse::All { blocks })
            }
            LoomViewType::Pins => {
                // MT-183: the reorderable Pins grid — explicit pin_order first
                // (NULLS LAST so newly pinned, un-ordered blocks trail), then by
                // recency for a deterministic, stable feed.
                let blocks = select_blocks(
                    Some("b.pinned != 0"),
                    "ORDER BY b.pin_order ASC NULLS LAST, b.updated_at DESC, b.block_id ASC",
                )
                .await?;
                Ok(LoomViewResponse::Pins { blocks })
            }
            LoomViewType::Favorites => {
                let blocks = select_blocks(
                    Some("b.favorite != 0"),
                    "ORDER BY b.updated_at DESC, b.block_id ASC",
                )
                .await?;
                Ok(LoomViewResponse::Favorites { blocks })
            }
            LoomViewType::Unlinked => {
                let blocks = select_blocks(
                    Some(
                        r#"
                    NOT EXISTS (
                        SELECT 1
                        FROM loom_edges e
                        WHERE e.workspace_id = b.workspace_id
                          AND (e.source_block_id = b.block_id OR e.target_block_id = b.block_id)
                          AND e.edge_type IN ('mention', 'tag')
                    )
                    "#,
                    ),
                    "ORDER BY b.updated_at DESC, b.block_id ASC",
                )
                .await?;
                Ok(LoomViewResponse::Unlinked { blocks })
            }
            LoomViewType::Sorted => {
                let mut qb = sqlx::QueryBuilder::<sqlx::Postgres>::new(
                    r#"
                    SELECT DISTINCT e.edge_type, e.target_block_id
                    FROM loom_edges e
                    JOIN loom_blocks b
                      ON b.workspace_id = e.workspace_id AND b.block_id = e.source_block_id
                    LEFT JOIN assets a
                      ON a.workspace_id = b.workspace_id AND a.asset_id = b.asset_id
                    "#,
                );

                let mut has_where = false;
                let mut push_clause = |builder: &mut sqlx::QueryBuilder<sqlx::Postgres>| {
                    if has_where {
                        builder.push(" AND ");
                    } else {
                        builder.push(" WHERE ");
                        has_where = true;
                    }
                };

                push_clause(&mut qb);
                qb.push("e.workspace_id = ").push_bind(workspace_id);

                push_clause(&mut qb);
                qb.push("e.edge_type IN ('mention', 'tag')");

                if let Some(content_type) = filters.content_type.clone() {
                    push_clause(&mut qb);
                    qb.push("b.content_type = ")
                        .push_bind(content_type.as_str());
                }

                if let Some(mime) = filters.mime.clone() {
                    push_clause(&mut qb);
                    qb.push("a.mime = ").push_bind(mime);
                }

                if let Some(from) = filters.date_from {
                    push_clause(&mut qb);
                    qb.push(
                        "(CASE WHEN b.content_type = 'journal' AND b.journal_date IS NOT NULL \
                         THEN b.journal_date >= to_char(",
                    )
                    .push_bind(from)
                    .push("::timestamptz, 'YYYY-MM-DD') ELSE b.updated_at >= ")
                    .push_bind(from)
                    .push(" END)");
                }
                if let Some(to) = filters.date_to {
                    push_clause(&mut qb);
                    qb.push(
                        "(CASE WHEN b.content_type = 'journal' AND b.journal_date IS NOT NULL \
                         THEN b.journal_date <= to_char(",
                    )
                    .push_bind(to)
                    .push("::timestamptz, 'YYYY-MM-DD') ELSE b.updated_at <= ")
                    .push_bind(to)
                    .push(" END)");
                }

                if !filters.tag_ids.is_empty() {
                    push_clause(&mut qb);
                    qb.push(
                        "EXISTS (SELECT 1 FROM loom_edges e2 WHERE e2.workspace_id = b.workspace_id AND e2.source_block_id = b.block_id AND e2.edge_type = 'tag' AND e2.target_block_id IN (",
                    );
                    let mut separated = qb.separated(", ");
                    for tag_id in &filters.tag_ids {
                        separated.push_bind(tag_id);
                    }
                    separated.push_unseparated("))");
                }

                if !filters.mention_ids.is_empty() {
                    push_clause(&mut qb);
                    qb.push(
                        "EXISTS (SELECT 1 FROM loom_edges e2 WHERE e2.workspace_id = b.workspace_id AND e2.source_block_id = b.block_id AND e2.edge_type = 'mention' AND e2.target_block_id IN (",
                    );
                    let mut separated = qb.separated(", ");
                    for mention_id in &filters.mention_ids {
                        separated.push_bind(mention_id);
                    }
                    separated.push_unseparated("))");
                }

                qb.push(
                    r#"
                    ORDER BY edge_type ASC, target_block_id ASC
                    LIMIT "#,
                );
                qb.push_bind(limit_i64);
                qb.push(" OFFSET ").push_bind(offset_i64);

                let group_rows = qb.build().fetch_all(&self.pool).await?;

                let mut groups: Vec<LoomViewGroup> = Vec::new();
                for row in group_rows {
                    let edge_type_raw: String = row.get("edge_type");
                    let target_block_id: String = row.get("target_block_id");
                    let edge_type = LoomEdgeType::from_str(edge_type_raw.as_str())?;

                    let mut qb = sqlx::QueryBuilder::<sqlx::Postgres>::new(
                        r#"
                        SELECT
                            b.block_id,
                            b.workspace_id,
                            b.content_type,
                            b.document_id,
                            b.asset_id,
                            b.title,
                            b.original_filename,
                            b.content_hash,
                            b.pinned,
                            b.favorite,
                            b.journal_date,
                            b.created_at,
                            b.updated_at,
                            b.imported_at,
                            b.backlink_count,
                            b.mention_count,
                            b.tag_count,
                            b.derived_json,
                            b.preview_status,
                            b.thumbnail_asset_id,
                            b.proxy_asset_id
                        FROM loom_edges e
                        JOIN loom_blocks b
                          ON b.workspace_id = e.workspace_id AND b.block_id = e.source_block_id
                        LEFT JOIN assets a
                          ON a.workspace_id = b.workspace_id AND a.asset_id = b.asset_id
                        "#,
                    );

                    let mut has_where = false;
                    let mut push_clause = |builder: &mut sqlx::QueryBuilder<sqlx::Postgres>| {
                        if has_where {
                            builder.push(" AND ");
                        } else {
                            builder.push(" WHERE ");
                            has_where = true;
                        }
                    };

                    push_clause(&mut qb);
                    qb.push("e.workspace_id = ").push_bind(workspace_id);

                    push_clause(&mut qb);
                    qb.push("e.edge_type = ").push_bind(edge_type.as_str());

                    push_clause(&mut qb);
                    qb.push("e.target_block_id = ").push_bind(&target_block_id);

                    if let Some(content_type) = filters.content_type.clone() {
                        push_clause(&mut qb);
                        qb.push("b.content_type = ")
                            .push_bind(content_type.as_str());
                    }

                    if let Some(mime) = filters.mime.clone() {
                        push_clause(&mut qb);
                        qb.push("a.mime = ").push_bind(mime);
                    }

                    if let Some(from) = filters.date_from {
                        push_clause(&mut qb);
                        qb.push(
                            "(CASE WHEN b.content_type = 'journal' AND b.journal_date IS NOT NULL \
                             THEN b.journal_date >= to_char(",
                        )
                        .push_bind(from)
                        .push("::timestamptz, 'YYYY-MM-DD') ELSE b.updated_at >= ")
                        .push_bind(from)
                        .push(" END)");
                    }
                    if let Some(to) = filters.date_to {
                        push_clause(&mut qb);
                        qb.push(
                            "(CASE WHEN b.content_type = 'journal' AND b.journal_date IS NOT NULL \
                             THEN b.journal_date <= to_char(",
                        )
                        .push_bind(to)
                        .push("::timestamptz, 'YYYY-MM-DD') ELSE b.updated_at <= ")
                        .push_bind(to)
                        .push(" END)");
                    }

                    if !filters.tag_ids.is_empty() {
                        push_clause(&mut qb);
                        qb.push(
                            "EXISTS (SELECT 1 FROM loom_edges e2 WHERE e2.workspace_id = b.workspace_id AND e2.source_block_id = b.block_id AND e2.edge_type = 'tag' AND e2.target_block_id IN (",
                        );
                        let mut separated = qb.separated(", ");
                        for tag_id in &filters.tag_ids {
                            separated.push_bind(tag_id);
                        }
                        separated.push_unseparated("))");
                    }

                    if !filters.mention_ids.is_empty() {
                        push_clause(&mut qb);
                        qb.push(
                            "EXISTS (SELECT 1 FROM loom_edges e2 WHERE e2.workspace_id = b.workspace_id AND e2.source_block_id = b.block_id AND e2.edge_type = 'mention' AND e2.target_block_id IN (",
                        );
                        let mut separated = qb.separated(", ");
                        for mention_id in &filters.mention_ids {
                            separated.push_bind(mention_id);
                        }
                        separated.push_unseparated("))");
                    }

                    qb.push(" ORDER BY b.updated_at DESC ");
                    qb.push(" LIMIT 100");

                    let rows = qb.build().fetch_all(&self.pool).await?;

                    let blocks: Vec<LoomBlock> = rows
                        .into_iter()
                        .map(map_loom_block)
                        .collect::<StorageResult<Vec<_>>>()?;

                    if !blocks.is_empty() {
                        groups.push(LoomViewGroup {
                            edge_type,
                            target_block_id,
                            blocks,
                        });
                    }
                }

                Ok(LoomViewResponse::Sorted { groups })
            }
        }
    }

    async fn search_loom_blocks(
        &self,
        workspace_id: &str,
        query: &str,
        filters: LoomSearchFilters,
        limit: u32,
        offset: u32,
    ) -> StorageResult<Vec<LoomBlockSearchResult>> {
        let exact_sql = loom_search_uses_exact_sql(&filters);
        let tokens = if exact_sql {
            vec![query.trim().to_string()]
        } else {
            normalize_loom_search_tokens(query)
        };
        if tokens.iter().all(|token| token.trim().is_empty()) {
            return Ok(Vec::new());
        }
        let fuzzy_query = if exact_sql {
            None
        } else {
            loom_fuzzy_query(&tokens)
        };
        let limit_i64 = limit as i64;
        let offset_i64 = offset as i64;

        let mut qb = sqlx::QueryBuilder::<sqlx::Postgres>::new(
            r#"
            SELECT
                b.block_id,
                b.workspace_id,
                b.content_type,
                b.document_id,
                b.asset_id,
                b.title,
                b.original_filename,
                b.content_hash,
                b.pinned,
                b.favorite,
                b.journal_date,
                b.created_at,
                b.updated_at,
                b.imported_at,
                b.backlink_count,
                b.mention_count,
                b.tag_count,
                b.derived_json,
                b.preview_status,
                b.thumbnail_asset_id,
                b.proxy_asset_id
            FROM loom_blocks b
            LEFT JOIN assets a
              ON a.workspace_id = b.workspace_id AND a.asset_id = b.asset_id
            "#,
        );

        let mut has_where = false;
        let mut push_clause = |builder: &mut sqlx::QueryBuilder<sqlx::Postgres>| {
            if has_where {
                builder.push(" AND ");
            } else {
                builder.push(" WHERE ");
                has_where = true;
            }
        };

        push_clause(&mut qb);
        qb.push("b.workspace_id = ").push_bind(workspace_id);

        if exact_sql {
            push_loom_exact_text_match(
                &mut qb,
                &[
                    "COALESCE(b.title, '')",
                    "COALESCE(b.original_filename, '')",
                    "COALESCE((b.derived_json::jsonb ->> 'full_text_index'), '')",
                ],
                query,
                &filters,
            );
        } else if let Some(fuzzy_query) = fuzzy_query.as_deref() {
            push_loom_fuzzy_sql_match(
                &mut qb,
                &[
                    "b.block_id",
                    "COALESCE(b.title, '')",
                    "COALESCE(b.original_filename, '')",
                ],
                fuzzy_query,
            );
        } else {
            for token in tokens {
                let pattern = format!("%{}%", escape_like_token(&token));
                push_clause(&mut qb);
                qb.push("(");
                qb.push("COALESCE(b.title, '') ILIKE ")
                    .push_bind(pattern.clone())
                    .push(" ESCAPE '\\'");
                qb.push(" OR COALESCE(b.original_filename, '') ILIKE ")
                    .push_bind(pattern.clone())
                    .push(" ESCAPE '\\'");
                qb.push(" OR COALESCE((b.derived_json::jsonb ->> 'full_text_index'), '') ILIKE ")
                    .push_bind(pattern)
                    .push(" ESCAPE '\\'");
                qb.push(")");
            }
        }
        push_loom_path_filter(
            &mut qb,
            &[
                "b.block_id",
                "COALESCE(b.document_id, '')",
                "COALESCE(b.original_filename, '')",
                "COALESCE(b.title, '')",
            ],
            filters.path.as_deref(),
        );

        if let Some(content_type) = filters.content_type.as_ref() {
            push_clause(&mut qb);
            qb.push("b.content_type = ")
                .push_bind(content_type.as_str());
        }
        if let Some(source_content_types) = loom_search_block_content_type_filter(&filters) {
            push_clause(&mut qb);
            if source_content_types.is_empty() {
                qb.push("FALSE");
            } else {
                qb.push("b.content_type IN (");
                let mut separated = qb.separated(", ");
                for content_type in source_content_types {
                    separated.push_bind(content_type);
                }
                separated.push_unseparated(")");
            }
        }
        if let Some(mime) = filters.mime {
            push_clause(&mut qb);
            qb.push("a.mime = ").push_bind(mime);
        }

        let backlink_depth = filters.backlink_depth.unwrap_or(1);

        if !filters.tag_ids.is_empty() {
            push_clause(&mut qb);
            if backlink_depth <= 1 {
                qb.push(
                    "EXISTS (SELECT 1 FROM loom_edges e WHERE e.workspace_id = b.workspace_id AND e.source_block_id = b.block_id AND e.edge_type = 'tag' AND e.target_block_id IN (",
                );
                let mut separated = qb.separated(", ");
                for tag_id in &filters.tag_ids {
                    separated.push_bind(tag_id);
                }
                separated.push_unseparated("))");
            } else {
                qb.push(
                    "EXISTS (WITH RECURSIVE reachable(block_id, depth, edge_type, path) AS (\
                        SELECT e.target_block_id, 1, e.edge_type, ARRAY[e.source_block_id, e.target_block_id]::TEXT[] \
                        FROM loom_edges e \
                        WHERE e.workspace_id = b.workspace_id \
                          AND e.source_block_id = b.block_id \
                        UNION ALL \
                        SELECT e.target_block_id, r.depth + 1, e.edge_type, r.path || e.target_block_id \
                        FROM loom_edges e \
                        JOIN reachable r ON e.source_block_id = r.block_id \
                        WHERE e.workspace_id = b.workspace_id \
                          AND r.depth < ",
                );
                qb.push_bind(backlink_depth as i32);
                qb.push(
                    " \
                          AND NOT e.target_block_id = ANY(r.path) \
                    ) \
                    SELECT 1 FROM reachable r \
                    WHERE r.edge_type = 'tag' \
                      AND r.block_id IN (",
                );
                let mut separated = qb.separated(", ");
                for tag_id in &filters.tag_ids {
                    separated.push_bind(tag_id);
                }
                separated.push_unseparated("))");
            }
        }

        if !filters.mention_ids.is_empty() {
            push_clause(&mut qb);
            if backlink_depth <= 1 {
                qb.push(
                    "EXISTS (SELECT 1 FROM loom_edges e WHERE e.workspace_id = b.workspace_id AND e.source_block_id = b.block_id AND e.edge_type = 'mention' AND e.target_block_id IN (",
                );
                let mut separated = qb.separated(", ");
                for mention_id in &filters.mention_ids {
                    separated.push_bind(mention_id);
                }
                separated.push_unseparated("))");
            } else {
                qb.push(
                    "EXISTS (WITH RECURSIVE reachable(block_id, depth, edge_type, path) AS (\
                        SELECT e.target_block_id, 1, e.edge_type, ARRAY[e.source_block_id, e.target_block_id]::TEXT[] \
                        FROM loom_edges e \
                        WHERE e.workspace_id = b.workspace_id \
                          AND e.source_block_id = b.block_id \
                        UNION ALL \
                        SELECT e.target_block_id, r.depth + 1, e.edge_type, r.path || e.target_block_id \
                        FROM loom_edges e \
                        JOIN reachable r ON e.source_block_id = r.block_id \
                        WHERE e.workspace_id = b.workspace_id \
                          AND r.depth < ",
                );
                qb.push_bind(backlink_depth as i32);
                qb.push(
                    " \
                          AND NOT e.target_block_id = ANY(r.path) \
                    ) \
                    SELECT 1 FROM reachable r \
                    WHERE r.edge_type = 'mention' \
                      AND r.block_id IN (",
                );
                let mut separated = qb.separated(", ");
                for mention_id in &filters.mention_ids {
                    separated.push_bind(mention_id);
                }
                separated.push_unseparated("))");
            }
        }

        qb.push(
            r#"
            ORDER BY
                (
                    CASE WHEN b.pinned != 0 THEN 5.0 ELSE 0.0 END
                    + CASE WHEN b.favorite != 0 THEN 3.0 ELSE 0.0 END
                    + (LEAST(GREATEST(COALESCE(b.tag_count, 0), 0), 10)::DOUBLE PRECISION * 1.5)
                    + (LEAST(GREATEST(COALESCE(b.backlink_count, 0), 0), 10)::DOUBLE PRECISION * 1.0)
                ) DESC,
                b.updated_at DESC,
                b.block_id ASC
            "#,
        );
        if fuzzy_query.is_none() {
            qb.push(" LIMIT ").push_bind(limit_i64);
            qb.push(" OFFSET ").push_bind(offset_i64);
        }

        let rows = qb.build().fetch_all(&self.pool).await?;
        let mut blocks = Vec::new();
        for row in rows {
            let block = map_loom_block(row)?;
            let fuzzy_score = fuzzy_query.as_deref().and_then(|needle| {
                loom_fuzzy_score(
                    needle,
                    [
                        block.block_id.as_str(),
                        block.title.as_deref().unwrap_or_default(),
                        block.original_filename.as_deref().unwrap_or_default(),
                    ],
                )
            });
            if fuzzy_query.is_some() && fuzzy_score.is_none() {
                continue;
            }
            let score = loom_retrieval_bias_score(&block) + fuzzy_score.unwrap_or(0.0);
            blocks.push(LoomBlockSearchResult { block, score });
        }

        if fuzzy_query.is_some() {
            blocks.sort_by(|left, right| {
                right
                    .score
                    .total_cmp(&left.score)
                    .then_with(|| left.block.block_id.cmp(&right.block.block_id))
            });
            Ok(blocks
                .into_iter()
                .skip(offset as usize)
                .take(limit as usize)
                .collect())
        } else {
            Ok(blocks)
        }
    }

    async fn search_loom_graph(
        &self,
        workspace_id: &str,
        query: &str,
        filters: LoomSearchFilters,
        limit: u32,
        offset: u32,
    ) -> StorageResult<Vec<LoomGraphSearchResult>> {
        let exact_sql = loom_search_uses_exact_sql(&filters);
        let tokens = if exact_sql {
            vec![query.trim().to_string()]
        } else {
            normalize_loom_search_tokens(query)
        };
        if tokens.iter().all(|token| token.trim().is_empty()) || limit == 0 {
            return Ok(Vec::new());
        }

        let fuzzy_query = if exact_sql {
            None
        } else {
            loom_fuzzy_query(&tokens)
        };
        let source_limit = limit.min(500);
        let fetch_limit = offset.saturating_add(source_limit);
        let candidate_limit = if fuzzy_query.is_some() {
            offset.saturating_add(source_limit.max(500))
        } else {
            fetch_limit
        };
        let fetch_limit_i64 = candidate_limit as i64;
        let block_scoped_filters = loom_search_has_block_scoped_filters(&filters);
        let mut results = Vec::new();

        let block_source_kinds: Vec<LoomSearchSourceKind> = [
            LoomSearchSourceKind::LoomBlock,
            LoomSearchSourceKind::File,
            LoomSearchSourceKind::TagHub,
        ]
        .into_iter()
        .filter(|kind| loom_search_source_allowed(&filters, *kind))
        .collect();

        for source_kind_filter in block_source_kinds {
            let mut source_filters = filters.clone();
            source_filters.source_kinds = vec![source_kind_filter];
            let block_hits = self
                .search_loom_blocks(workspace_id, query, source_filters, candidate_limit, 0)
                .await?;
            results.extend(block_hits.into_iter().filter_map(|hit| {
                let block = hit.block;
                let source_kind = loom_block_graph_source_kind(&block.content_type);
                if source_kind != source_kind_filter {
                    return None;
                }
                let score = hit.score;
                let title = block
                    .title
                    .clone()
                    .or_else(|| block.original_filename.clone())
                    .unwrap_or_else(|| block.block_id.clone());
                let excerpt = block
                    .derived
                    .full_text_index
                    .as_deref()
                    .map(loom_search_excerpt)
                    .unwrap_or_default();
                let metadata = loom_retrieval_bias_metadata(&block, score);
                Some(LoomGraphSearchResult {
                    result_kind: LoomSearchResultKind::LoomBlock,
                    source_kind,
                    ref_id: block.block_id.clone(),
                    title,
                    excerpt,
                    block: Some(block),
                    score,
                    metadata,
                })
            }));
        }

        let knowledge_kinds: Vec<LoomSearchSourceKind> = [
            LoomSearchSourceKind::Symbol,
            LoomSearchSourceKind::WorkPacket,
            LoomSearchSourceKind::MicroTask,
        ]
        .into_iter()
        .filter(|kind| !block_scoped_filters && loom_search_source_allowed(&filters, *kind))
        .collect();

        for source_kind_filter in knowledge_kinds {
            let mut qb = sqlx::QueryBuilder::<Postgres>::new(
                r#"
                SELECT entity_id, entity_kind, entity_key, display_name, detection_provenance
                FROM knowledge_entities
                WHERE workspace_id =
                "#,
            );
            qb.push_bind(workspace_id);
            qb.push(" AND lifecycle_state = 'active' AND entity_kind = ");
            qb.push_bind(source_kind_filter.as_str());

            if exact_sql {
                push_loom_exact_text_match(
                    &mut qb,
                    &[
                        "entity_id",
                        "entity_key",
                        "display_name",
                        "detection_provenance::text",
                    ],
                    query,
                    &filters,
                );
            } else if let Some(fuzzy_query) = fuzzy_query.as_deref() {
                push_loom_fuzzy_sql_match(
                    &mut qb,
                    &["entity_id", "entity_key", "display_name"],
                    fuzzy_query,
                );
            } else {
                for token in &tokens {
                    let pattern = format!("%{}%", escape_like_token(token));
                    qb.push(" AND (display_name ILIKE ")
                        .push_bind(pattern.clone())
                        .push(" ESCAPE '\\'");
                    qb.push(" OR entity_key ILIKE ")
                        .push_bind(pattern.clone())
                        .push(" ESCAPE '\\'");
                    qb.push(" OR detection_provenance::text ILIKE ")
                        .push_bind(pattern)
                        .push(" ESCAPE '\\')");
                }
            }
            push_loom_path_filter(
                &mut qb,
                &[
                    "entity_id",
                    "entity_key",
                    "display_name",
                    "detection_provenance::text",
                ],
                filters.path.as_deref(),
            );

            qb.push(" ORDER BY updated_at DESC, entity_id ASC");
            if fuzzy_query.is_none() {
                qb.push(" LIMIT ").push_bind(fetch_limit_i64);
            }

            let rows = qb.build().fetch_all(&self.pool).await?;
            for row in rows {
                let source_kind = row
                    .get::<String, _>("entity_kind")
                    .parse::<LoomSearchSourceKind>()?;
                let entity_key: String = row.get("entity_key");
                let display_name: String = row.get("display_name");
                let detection_provenance: Value = row.get("detection_provenance");
                let ref_id: String = row.get("entity_id");
                let fuzzy_score = fuzzy_query.as_deref().and_then(|needle| {
                    loom_fuzzy_score(
                        needle,
                        [ref_id.as_str(), entity_key.as_str(), display_name.as_str()],
                    )
                });
                if fuzzy_query.is_some() && fuzzy_score.is_none() {
                    continue;
                }
                results.push(LoomGraphSearchResult {
                    result_kind: LoomSearchResultKind::KnowledgeEntity,
                    source_kind,
                    ref_id,
                    title: display_name,
                    excerpt: loom_search_excerpt(&entity_key),
                    block: None,
                    score: fuzzy_score.unwrap_or(0.0),
                    metadata: json!({
                        "authority_table": "knowledge_entities",
                        "entity_key": entity_key,
                        "detection_provenance": detection_provenance,
                    }),
                });
            }
        }

        if !block_scoped_filters
            && loom_search_source_allowed(&filters, LoomSearchSourceKind::Document)
        {
            let mut document_qb = sqlx::QueryBuilder::<Postgres>::new(
                r#"
                SELECT rich_document_id, document_id, title, schema_version,
                       doc_version, authority_label, content_json
                FROM knowledge_rich_documents
                WHERE workspace_id =
                "#,
            );
            document_qb.push_bind(workspace_id);

            if exact_sql {
                push_loom_exact_text_match(
                    &mut document_qb,
                    &[
                        "rich_document_id",
                        "COALESCE(document_id, '')",
                        "title",
                        "content_json::text",
                    ],
                    query,
                    &filters,
                );
            } else if let Some(fuzzy_query) = fuzzy_query.as_deref() {
                push_loom_fuzzy_sql_match(
                    &mut document_qb,
                    &[
                        "rich_document_id",
                        "COALESCE(document_id, '')",
                        "title",
                        "content_json::text",
                    ],
                    fuzzy_query,
                );
            } else {
                for token in &tokens {
                    let pattern = format!("%{}%", escape_like_token(token));
                    document_qb
                        .push(" AND (rich_document_id ILIKE ")
                        .push_bind(pattern.clone())
                        .push(" ESCAPE '\\'");
                    document_qb
                        .push(" OR COALESCE(document_id, '') ILIKE ")
                        .push_bind(pattern.clone())
                        .push(" ESCAPE '\\'");
                    document_qb
                        .push(" OR title ILIKE ")
                        .push_bind(pattern.clone())
                        .push(" ESCAPE '\\'");
                    document_qb
                        .push(" OR content_json::text ILIKE ")
                        .push_bind(pattern)
                        .push(" ESCAPE '\\')");
                }
            }
            push_loom_path_filter(
                &mut document_qb,
                &["rich_document_id", "COALESCE(document_id, '')", "title"],
                filters.path.as_deref(),
            );
            document_qb.push(" ORDER BY updated_at DESC, rich_document_id ASC");
            if fuzzy_query.is_none() {
                document_qb.push(" LIMIT ").push_bind(fetch_limit_i64);
            }

            let document_rows = document_qb.build().fetch_all(&self.pool).await?;
            for row in document_rows {
                let rich_document_id: String = row.get("rich_document_id");
                let document_id: Option<String> = row.get("document_id");
                let title: String = row.get("title");
                let schema_version: String = row.get("schema_version");
                let doc_version: i64 = row.get("doc_version");
                let authority_label: String = row.get("authority_label");
                let content_json: Value = row.get("content_json");
                let content_json_text = content_json.to_string();
                let fuzzy_score = fuzzy_query.as_deref().and_then(|needle| {
                    loom_fuzzy_score(
                        needle,
                        [
                            rich_document_id.as_str(),
                            document_id.as_deref().unwrap_or_default(),
                            title.as_str(),
                            content_json_text.as_str(),
                        ],
                    )
                });
                if fuzzy_query.is_some() && fuzzy_score.is_none() {
                    continue;
                }
                results.push(LoomGraphSearchResult {
                    result_kind: LoomSearchResultKind::KnowledgeEntity,
                    source_kind: LoomSearchSourceKind::Document,
                    ref_id: rich_document_id.clone(),
                    title,
                    excerpt: loom_search_excerpt(&content_json_text),
                    block: None,
                    score: fuzzy_score.unwrap_or(0.0),
                    metadata: json!({
                        "authority_table": "knowledge_rich_documents",
                        "rich_document_id": rich_document_id,
                        "document_id": document_id,
                        "schema_version": schema_version,
                        "doc_version": doc_version,
                        "authority_label": authority_label,
                    }),
                });
            }
        }

        if !block_scoped_filters
            && loom_search_source_allowed(&filters, LoomSearchSourceKind::UserManualPage)
        {
            let mut page_qb = sqlx::QueryBuilder::<Postgres>::new(
                r#"
                SELECT slug, title, ''::text AS excerpt
                FROM user_manual_pages
                WHERE status = 'current'
                "#,
            );
            if exact_sql {
                push_loom_exact_text_match(&mut page_qb, &["slug", "title"], query, &filters);
            } else if let Some(fuzzy_query) = fuzzy_query.as_deref() {
                push_loom_fuzzy_sql_match(&mut page_qb, &["slug", "title"], fuzzy_query);
            } else {
                for token in &tokens {
                    let pattern = format!("%{}%", escape_like_token(token));
                    page_qb
                        .push(" AND (slug ILIKE ")
                        .push_bind(pattern.clone())
                        .push(" ESCAPE '\\'");
                    page_qb
                        .push(" OR title ILIKE ")
                        .push_bind(pattern)
                        .push(" ESCAPE '\\')");
                }
            }
            push_loom_path_filter(&mut page_qb, &["slug", "title"], filters.path.as_deref());
            page_qb.push(" ORDER BY slug");
            if fuzzy_query.is_none() {
                page_qb.push(" LIMIT ").push_bind(fetch_limit_i64);
            }
            let page_rows = page_qb.build().fetch_all(&self.pool).await?;
            for row in page_rows {
                let slug: String = row.get("slug");
                let title: String = row.get("title");
                let excerpt: String = row.get("excerpt");
                let fuzzy_score = fuzzy_query.as_deref().and_then(|needle| {
                    loom_fuzzy_score(needle, [slug.as_str(), title.as_str(), excerpt.as_str()])
                });
                if fuzzy_query.is_some() && fuzzy_score.is_none() {
                    continue;
                }
                results.push(LoomGraphSearchResult {
                    result_kind: LoomSearchResultKind::UserManualPage,
                    source_kind: LoomSearchSourceKind::UserManualPage,
                    ref_id: slug.clone(),
                    title,
                    excerpt,
                    block: None,
                    score: fuzzy_score.unwrap_or(0.0),
                    metadata: json!({
                        "authority_table": "user_manual_pages",
                        "page_slug": slug,
                    }),
                });
            }

            let mut section_qb = sqlx::QueryBuilder::<Postgres>::new(
                r#"
                SELECT p.slug, s.title, s.body_md AS excerpt
                FROM user_manual_sections s
                JOIN user_manual_pages p ON p.page_id = s.page_id
                WHERE p.status = 'current'
                "#,
            );
            if exact_sql {
                push_loom_exact_text_match(
                    &mut section_qb,
                    &["p.slug", "p.title", "s.title", "s.body_md"],
                    query,
                    &filters,
                );
            } else if let Some(fuzzy_query) = fuzzy_query.as_deref() {
                push_loom_fuzzy_sql_match(
                    &mut section_qb,
                    &["p.slug", "p.title", "s.title"],
                    fuzzy_query,
                );
            } else {
                for token in &tokens {
                    let pattern = format!("%{}%", escape_like_token(token));
                    section_qb
                        .push(" AND (p.slug ILIKE ")
                        .push_bind(pattern.clone())
                        .push(" ESCAPE '\\'");
                    section_qb
                        .push(" OR p.title ILIKE ")
                        .push_bind(pattern.clone())
                        .push(" ESCAPE '\\'");
                    section_qb
                        .push(" OR s.title ILIKE ")
                        .push_bind(pattern.clone())
                        .push(" ESCAPE '\\'");
                    section_qb
                        .push(" OR s.body_md ILIKE ")
                        .push_bind(pattern)
                        .push(" ESCAPE '\\')");
                }
            }
            push_loom_path_filter(
                &mut section_qb,
                &["p.slug", "p.title", "s.title"],
                filters.path.as_deref(),
            );
            section_qb.push(" ORDER BY p.slug, s.position");
            if fuzzy_query.is_none() {
                section_qb.push(" LIMIT ").push_bind(fetch_limit_i64);
            }
            let section_rows = section_qb.build().fetch_all(&self.pool).await?;
            for row in section_rows {
                let slug: String = row.get("slug");
                let title: String = row.get("title");
                let excerpt: String = row.get("excerpt");
                let fuzzy_score = fuzzy_query
                    .as_deref()
                    .and_then(|needle| loom_fuzzy_score(needle, [slug.as_str(), title.as_str()]));
                if fuzzy_query.is_some() && fuzzy_score.is_none() {
                    continue;
                }
                results.push(LoomGraphSearchResult {
                    result_kind: LoomSearchResultKind::UserManualPage,
                    source_kind: LoomSearchSourceKind::UserManualPage,
                    ref_id: slug.clone(),
                    title,
                    excerpt: loom_search_excerpt(&excerpt),
                    block: None,
                    score: fuzzy_score.unwrap_or(0.0),
                    metadata: json!({
                        "authority_table": "user_manual_sections",
                        "page_slug": slug,
                    }),
                });
            }
        }

        if !block_scoped_filters
            && loom_search_source_allowed(&filters, LoomSearchSourceKind::WikiPage)
        {
            let mut wiki_qb = sqlx::QueryBuilder::<Postgres>::new(
                r#"
                SELECT projection_id, title, rendered_content, page_type, rebuild_status
                FROM knowledge_wiki_projections
                WHERE workspace_id =
                "#,
            );
            wiki_qb.push_bind(workspace_id);
            wiki_qb.push(" AND projection_kind = 'wiki_page'");
            if exact_sql {
                push_loom_exact_text_match(
                    &mut wiki_qb,
                    &["projection_id", "title", "rendered_content"],
                    query,
                    &filters,
                );
            } else if let Some(fuzzy_query) = fuzzy_query.as_deref() {
                push_loom_fuzzy_sql_match(&mut wiki_qb, &["projection_id", "title"], fuzzy_query);
            } else {
                for token in &tokens {
                    let pattern = format!("%{}%", escape_like_token(token));
                    wiki_qb
                        .push(" AND (projection_id ILIKE ")
                        .push_bind(pattern.clone())
                        .push(" ESCAPE '\\'");
                    wiki_qb
                        .push(" OR title ILIKE ")
                        .push_bind(pattern.clone())
                        .push(" ESCAPE '\\'");
                    wiki_qb
                        .push(" OR rendered_content ILIKE ")
                        .push_bind(pattern)
                        .push(" ESCAPE '\\')");
                }
            }
            push_loom_path_filter(
                &mut wiki_qb,
                &["projection_id", "title"],
                filters.path.as_deref(),
            );
            wiki_qb.push(" ORDER BY COALESCE(page_type, 'zz') ASC, title ASC");
            if fuzzy_query.is_none() {
                wiki_qb.push(" LIMIT ").push_bind(fetch_limit_i64);
            }
            let wiki_rows = wiki_qb.build().fetch_all(&self.pool).await?;
            for row in wiki_rows {
                let projection_id: String = row.get("projection_id");
                let title: String = row.get("title");
                let rendered_content: String = row.get("rendered_content");
                let page_type: Option<String> = row.get("page_type");
                let rebuild_status: String = row.get("rebuild_status");
                let fuzzy_score = fuzzy_query.as_deref().and_then(|needle| {
                    loom_fuzzy_score(needle, [projection_id.as_str(), title.as_str()])
                });
                if fuzzy_query.is_some() && fuzzy_score.is_none() {
                    continue;
                }
                results.push(LoomGraphSearchResult {
                    result_kind: LoomSearchResultKind::WikiPage,
                    source_kind: LoomSearchSourceKind::WikiPage,
                    ref_id: projection_id.clone(),
                    title,
                    excerpt: loom_search_excerpt(&rendered_content),
                    block: None,
                    score: fuzzy_score.unwrap_or(0.0),
                    metadata: json!({
                        "authority_table": "knowledge_wiki_projections",
                        "projection_id": projection_id,
                        "page_type": page_type,
                        "rebuild_status": rebuild_status,
                    }),
                });
            }
        }

        if fuzzy_query.is_some() {
            results.sort_by(|left, right| {
                right
                    .score
                    .total_cmp(&left.score)
                    .then_with(|| left.source_kind.as_str().cmp(right.source_kind.as_str()))
                    .then_with(|| left.ref_id.cmp(&right.ref_id))
            });
        }

        let results = order_loom_graph_results_for_breadth(results);

        Ok(results
            .into_iter()
            .skip(offset as usize)
            .take(source_limit as usize)
            .collect())
    }

    async fn record_quick_switcher_recent(
        &self,
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

        let ref_id = ref_id.to_string();
        let title = title.to_string();
        let excerpt = input.excerpt.trim().to_string();
        let source_kind = input.source_kind.as_str();
        let result_kind = input.result_kind.as_str();
        let hit_key = format!("{source_kind}:{ref_id}");
        let metadata = if input.metadata.is_null() {
            json!({})
        } else {
            input.metadata
        };
        let run_id = format!("QUICK-SWITCHER-RECENTS-{workspace_id}");
        let aggregate_id = format!("{workspace_id}:{hit_key}");
        let payload = json!({
            "type": "knowledge_quick_switcher_recent_recorded",
            "workspace_id": workspace_id,
            "hit_key": hit_key.clone(),
            "source_kind": source_kind,
            "ref_id": ref_id.clone(),
            "result_kind": result_kind,
            "title": title.clone(),
            "metadata": metadata.clone(),
        });
        let event = NewKernelEvent::builder(
            run_id.clone(),
            run_id,
            KernelEventType::KnowledgeQuickSwitcherRecentRecorded,
            KernelActor::System("quick-switcher-ui".to_string()),
        )
        .aggregate("quick_switcher_recent", aggregate_id)
        .source_component("quick_switcher_recents")
        .payload(payload)
        .build()
        .map_err(|err| StorageError::Validation(kernel_event_build_error(err)))?;

        let mut tx = self.pool.begin().await?;
        let stored_event = append_kernel_event_with_executor(&mut *tx, event).await?;
        let row = sqlx::query(
            r#"
            INSERT INTO knowledge_quick_switcher_recents (
                workspace_id,
                hit_key,
                source_kind,
                ref_id,
                result_kind,
                title,
                excerpt,
                metadata,
                selected_count,
                selected_at,
                event_ledger_event_id
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8::jsonb, 1, NOW(), $9)
            ON CONFLICT (workspace_id, hit_key) DO UPDATE SET
                source_kind = EXCLUDED.source_kind,
                ref_id = EXCLUDED.ref_id,
                result_kind = EXCLUDED.result_kind,
                title = EXCLUDED.title,
                excerpt = EXCLUDED.excerpt,
                metadata = EXCLUDED.metadata,
                selected_count = knowledge_quick_switcher_recents.selected_count + 1,
                selected_at = NOW(),
                event_ledger_event_id = EXCLUDED.event_ledger_event_id
            RETURNING workspace_id, hit_key, source_kind, ref_id, result_kind,
                      title, excerpt, metadata, selected_count, selected_at,
                      event_ledger_event_id
            "#,
        )
        .bind(workspace_id)
        .bind(&hit_key)
        .bind(source_kind)
        .bind(&ref_id)
        .bind(result_kind)
        .bind(&title)
        .bind(&excerpt)
        .bind(&metadata)
        .bind(&stored_event.event_id)
        .fetch_one(&mut *tx)
        .await?;
        let recent = map_quick_switcher_recent(&row)?;
        tx.commit().await?;
        Ok(recent)
    }

    async fn list_quick_switcher_recents(
        &self,
        workspace_id: &str,
        limit: u32,
    ) -> StorageResult<Vec<QuickSwitcherRecent>> {
        if limit == 0 {
            return Ok(Vec::new());
        }
        let cap = limit.min(100) as i64;
        let rows = sqlx::query(
            r#"
            SELECT workspace_id, hit_key, source_kind, ref_id, result_kind,
                   title, excerpt, metadata, selected_count, selected_at,
                   event_ledger_event_id
            FROM knowledge_quick_switcher_recents
            WHERE workspace_id = $1
            ORDER BY selected_at DESC, hit_key ASC
            LIMIT $2
            "#,
        )
        .bind(workspace_id)
        .bind(cap)
        .fetch_all(&self.pool)
        .await?;
        rows.iter().map(map_quick_switcher_recent).collect()
    }

    // -- MT-261 CanvasBoard ----------------------------------------------------

    async fn create_canvas_board(
        &self,
        ctx: &WriteContext,
        workspace_id: &str,
        block_id: &str,
        board_state: Value,
    ) -> StorageResult<LoomCanvasBoard> {
        validate_loom_canvas_board_state(&board_state)?;
        self.guard.validate_write(ctx, block_id).await?;

        // The canvas block must exist, belong to the workspace, and be a canvas.
        let block = self.get_loom_block(workspace_id, block_id).await?;
        if !matches!(block.content_type, LoomBlockContentType::Canvas) {
            return Err(StorageError::Validation(
                "canvas board block must be content_type=canvas",
            ));
        }

        let run_id = format!("LOOM-CANVAS-BOARD-{block_id}");
        let payload = json!({
            "type": "knowledge_loom_canvas_board_recorded",
            "op": "create",
            "workspace_id": workspace_id,
            "block_id": block_id,
            "board_state": board_state.clone(),
        });
        let event = NewKernelEvent::builder(
            run_id.clone(),
            run_id,
            KernelEventType::KnowledgeLoomCanvasBoardRecorded,
            KernelActor::System("loom-canvas-board".to_string()),
        )
        .aggregate("loom_canvas_board", block_id.to_string())
        .source_component("loom_canvas_board")
        .payload(payload)
        .build()
        .map_err(|err| StorageError::Validation(kernel_event_build_error(err)))?;

        let mut tx = self.pool.begin().await?;
        let stored_event = append_kernel_event_with_executor(&mut *tx, event).await?;
        let row = sqlx::query(
            r#"
            INSERT INTO loom_canvas_boards (
                block_id, workspace_id, board_state, created_at, updated_at,
                event_ledger_event_id
            )
            VALUES ($1, $2, $3::jsonb, NOW(), NOW(), $4)
            ON CONFLICT (block_id) DO UPDATE SET
                board_state = EXCLUDED.board_state,
                updated_at = NOW(),
                event_ledger_event_id = EXCLUDED.event_ledger_event_id
            RETURNING block_id, workspace_id, board_state, created_at, updated_at,
                      event_ledger_event_id
            "#,
        )
        .bind(block_id)
        .bind(workspace_id)
        .bind(&board_state)
        .bind(&stored_event.event_id)
        .fetch_one(&mut *tx)
        .await?;
        let board = map_loom_canvas_board(&row)?;
        tx.commit().await?;
        Ok(board)
    }

    async fn get_canvas_board(
        &self,
        workspace_id: &str,
        block_id: &str,
    ) -> StorageResult<LoomCanvasBoardView> {
        let board_row = sqlx::query(
            r#"
            SELECT block_id, workspace_id, board_state, created_at, updated_at,
                   event_ledger_event_id
            FROM loom_canvas_boards
            WHERE workspace_id = $1 AND block_id = $2
            "#,
        )
        .bind(workspace_id)
        .bind(block_id)
        .fetch_optional(&self.pool)
        .await?;
        let board = match board_row {
            Some(row) => map_loom_canvas_board(&row)?,
            None => return Err(StorageError::NotFound("loom_canvas_board")),
        };

        let placement_rows = sqlx::query(
            r#"
            SELECT placement_id, canvas_block_id, workspace_id, placed_block_id,
                   x, y, w, h, z_index, group_id, created_at, updated_at
            FROM loom_canvas_placements
            WHERE workspace_id = $1 AND canvas_block_id = $2
            ORDER BY z_index ASC, created_at ASC, placement_id ASC
            "#,
        )
        .bind(workspace_id)
        .bind(block_id)
        .fetch_all(&self.pool)
        .await?;
        let placements = placement_rows
            .iter()
            .map(map_loom_canvas_placement)
            .collect::<StorageResult<Vec<_>>>()?;

        let visual_edge_rows = sqlx::query(
            r#"
            SELECT visual_edge_id, canvas_block_id, workspace_id,
                   from_placement_id, to_placement_id, label, created_at
            FROM loom_canvas_visual_edges
            WHERE workspace_id = $1 AND canvas_block_id = $2
            ORDER BY created_at ASC, visual_edge_id ASC
            "#,
        )
        .bind(workspace_id)
        .bind(block_id)
        .fetch_all(&self.pool)
        .await?;
        let visual_edges = visual_edge_rows
            .iter()
            .map(map_loom_canvas_visual_edge)
            .collect::<StorageResult<Vec<_>>>()?;

        Ok(LoomCanvasBoardView {
            board,
            placements,
            visual_edges,
        })
    }

    async fn update_canvas_board_state(
        &self,
        ctx: &WriteContext,
        workspace_id: &str,
        block_id: &str,
        board_state: Value,
    ) -> StorageResult<LoomCanvasBoard> {
        validate_loom_canvas_board_state(&board_state)?;
        self.guard.validate_write(ctx, block_id).await?;

        let run_id = format!("LOOM-CANVAS-BOARD-{block_id}");
        let payload = json!({
            "type": "knowledge_loom_canvas_board_recorded",
            "op": "viewport",
            "workspace_id": workspace_id,
            "block_id": block_id,
            "board_state": board_state.clone(),
        });
        let event = NewKernelEvent::builder(
            run_id.clone(),
            run_id,
            KernelEventType::KnowledgeLoomCanvasBoardRecorded,
            KernelActor::System("loom-canvas-board".to_string()),
        )
        .aggregate("loom_canvas_board", block_id.to_string())
        .source_component("loom_canvas_board")
        .payload(payload)
        .build()
        .map_err(|err| StorageError::Validation(kernel_event_build_error(err)))?;

        let mut tx = self.pool.begin().await?;
        let stored_event = append_kernel_event_with_executor(&mut *tx, event).await?;
        let row = sqlx::query(
            r#"
            UPDATE loom_canvas_boards
            SET board_state = $3::jsonb, updated_at = NOW(),
                event_ledger_event_id = $4
            WHERE workspace_id = $1 AND block_id = $2
            RETURNING block_id, workspace_id, board_state, created_at, updated_at,
                      event_ledger_event_id
            "#,
        )
        .bind(workspace_id)
        .bind(block_id)
        .bind(&board_state)
        .bind(&stored_event.event_id)
        .fetch_optional(&mut *tx)
        .await?;
        let row = row.ok_or(StorageError::NotFound("loom_canvas_board"))?;
        let board = map_loom_canvas_board(&row)?;
        tx.commit().await?;
        Ok(board)
    }

    async fn place_block_on_canvas(
        &self,
        ctx: &WriteContext,
        placement: NewLoomCanvasPlacement,
    ) -> StorageResult<LoomCanvasPlacement> {
        if placement.w <= 0.0 || placement.h <= 0.0 {
            return Err(StorageError::Validation(
                "canvas placement w/h must be positive",
            ));
        }
        let placement_id = format!("LCP-{}", Uuid::now_v7().simple());
        self.guard.validate_write(ctx, &placement_id).await?;

        // Both the canvas board and the placed block must exist in the same
        // workspace. The placed block is referenced (FK) — never copied.
        let board = self
            .get_canvas_board(&placement.workspace_id, &placement.canvas_block_id)
            .await?;
        let placed = self
            .get_loom_block(&placement.workspace_id, &placement.placed_block_id)
            .await?;
        if board.board.workspace_id != placement.workspace_id
            || placed.workspace_id != placement.workspace_id
        {
            return Err(StorageError::Validation(
                "canvas placement requires same-workspace board and block",
            ));
        }

        let row = sqlx::query(
            r#"
            INSERT INTO loom_canvas_placements (
                placement_id, canvas_block_id, workspace_id, placed_block_id,
                x, y, w, h, z_index, group_id, created_at, updated_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, NOW(), NOW())
            RETURNING placement_id, canvas_block_id, workspace_id, placed_block_id,
                      x, y, w, h, z_index, group_id, created_at, updated_at
            "#,
        )
        .bind(&placement_id)
        .bind(&placement.canvas_block_id)
        .bind(&placement.workspace_id)
        .bind(&placement.placed_block_id)
        .bind(placement.x)
        .bind(placement.y)
        .bind(placement.w)
        .bind(placement.h)
        .bind(placement.z_index)
        .bind(&placement.group_id)
        .fetch_one(&self.pool)
        .await?;
        map_loom_canvas_placement(&row)
    }

    async fn update_canvas_placement(
        &self,
        ctx: &WriteContext,
        workspace_id: &str,
        placement_id: &str,
        update: LoomCanvasPlacementUpdate,
    ) -> StorageResult<LoomCanvasPlacement> {
        if matches!(update.w, Some(w) if w <= 0.0) || matches!(update.h, Some(h) if h <= 0.0) {
            return Err(StorageError::Validation(
                "canvas placement w/h must be positive",
            ));
        }
        self.guard.validate_write(ctx, placement_id).await?;
        let row = sqlx::query(
            r#"
            UPDATE loom_canvas_placements
            SET x = COALESCE($3, x),
                y = COALESCE($4, y),
                w = COALESCE($5, w),
                h = COALESCE($6, h),
                z_index = COALESCE($7, z_index),
                group_id = CASE WHEN $8 THEN $9 ELSE group_id END,
                updated_at = NOW()
            WHERE workspace_id = $1 AND placement_id = $2
            RETURNING placement_id, canvas_block_id, workspace_id, placed_block_id,
                      x, y, w, h, z_index, group_id, created_at, updated_at
            "#,
        )
        .bind(workspace_id)
        .bind(placement_id)
        .bind(update.x)
        .bind(update.y)
        .bind(update.w)
        .bind(update.h)
        .bind(update.z_index)
        .bind(update.group_id.is_some())
        .bind(update.group_id.flatten())
        .fetch_optional(&self.pool)
        .await?;
        let row = row.ok_or(StorageError::NotFound("loom_canvas_placement"))?;
        map_loom_canvas_placement(&row)
    }

    async fn remove_canvas_placement(
        &self,
        ctx: &WriteContext,
        workspace_id: &str,
        placement_id: &str,
    ) -> StorageResult<()> {
        self.guard.validate_write(ctx, placement_id).await?;
        // Deletes ONLY the placement row. The referenced LoomBlock is never
        // touched (reference-not-copy). Visual edges touching this placement
        // CASCADE via the FK; semantic loom_edges are unaffected.
        let res = sqlx::query(
            r#"
            DELETE FROM loom_canvas_placements
            WHERE workspace_id = $1 AND placement_id = $2
            "#,
        )
        .bind(workspace_id)
        .bind(placement_id)
        .execute(&self.pool)
        .await?;
        if res.rows_affected() == 0 {
            return Err(StorageError::NotFound("loom_canvas_placement"));
        }
        Ok(())
    }

    async fn add_canvas_visual_edge(
        &self,
        ctx: &WriteContext,
        workspace_id: &str,
        canvas_block_id: &str,
        from_placement_id: &str,
        to_placement_id: &str,
        label: Option<String>,
    ) -> StorageResult<LoomCanvasVisualEdge> {
        if from_placement_id == to_placement_id {
            return Err(StorageError::Validation(
                "canvas visual edge endpoints must differ",
            ));
        }
        let visual_edge_id = format!("LCV-{}", Uuid::now_v7().simple());
        self.guard.validate_write(ctx, &visual_edge_id).await?;

        // Both placements must belong to the same canvas + workspace. This is a
        // board-local decoration: it is NOT graph authority and never becomes a
        // loom_edge.
        let endpoint_count: i64 = sqlx::query_scalar(
            r#"
            SELECT COUNT(*)
            FROM loom_canvas_placements
            WHERE workspace_id = $1
              AND canvas_block_id = $2
              AND placement_id IN ($3, $4)
            "#,
        )
        .bind(workspace_id)
        .bind(canvas_block_id)
        .bind(from_placement_id)
        .bind(to_placement_id)
        .fetch_one(&self.pool)
        .await?;
        if endpoint_count != 2 {
            return Err(StorageError::Validation(
                "canvas visual edge endpoints must be placements on this canvas",
            ));
        }

        let row = sqlx::query(
            r#"
            INSERT INTO loom_canvas_visual_edges (
                visual_edge_id, canvas_block_id, workspace_id,
                from_placement_id, to_placement_id, label, created_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, NOW())
            RETURNING visual_edge_id, canvas_block_id, workspace_id,
                      from_placement_id, to_placement_id, label, created_at
            "#,
        )
        .bind(&visual_edge_id)
        .bind(canvas_block_id)
        .bind(workspace_id)
        .bind(from_placement_id)
        .bind(to_placement_id)
        .bind(&label)
        .fetch_one(&self.pool)
        .await?;
        map_loom_canvas_visual_edge(&row)
    }

    async fn remove_canvas_visual_edge(
        &self,
        ctx: &WriteContext,
        workspace_id: &str,
        visual_edge_id: &str,
    ) -> StorageResult<()> {
        self.guard.validate_write(ctx, visual_edge_id).await?;
        let res = sqlx::query(
            r#"
            DELETE FROM loom_canvas_visual_edges
            WHERE workspace_id = $1 AND visual_edge_id = $2
            "#,
        )
        .bind(workspace_id)
        .bind(visual_edge_id)
        .execute(&self.pool)
        .await?;
        if res.rows_affected() == 0 {
            return Err(StorageError::NotFound("loom_canvas_visual_edge"));
        }
        Ok(())
    }

    async fn get_workbench_layout_state(
        &self,
        workspace_id: &str,
    ) -> StorageResult<Option<WorkbenchLayoutState>> {
        let row = sqlx::query(
            r#"
            SELECT workspace_id, layout_state, updated_at, event_ledger_event_id
            FROM knowledge_workbench_layout_states
            WHERE workspace_id = $1
            "#,
        )
        .bind(workspace_id)
        .fetch_optional(&self.pool)
        .await?;
        row.as_ref().map(map_workbench_layout_state).transpose()
    }

    async fn save_workbench_layout_state(
        &self,
        workspace_id: &str,
        input: WorkbenchLayoutStateInput,
    ) -> StorageResult<WorkbenchLayoutState> {
        if !input.layout_state.is_object() {
            return Err(StorageError::Validation(
                "workbench layout_state must be a JSON object",
            ));
        }
        if input
            .layout_state
            .get("schema_id")
            .and_then(|value| value.as_str())
            != Some(WORKBENCH_LAYOUT_SCHEMA_ID)
        {
            return Err(StorageError::Validation(
                "workbench layout_state schema_id must be hsk.workbench_layout_state@1",
            ));
        }
        validate_workbench_layout_state_shape(&input.layout_state)?;

        let run_id = format!("WORKBENCH-LAYOUT-{workspace_id}");
        let payload = json!({
            "type": "knowledge_workbench_layout_state_recorded",
            "workspace_id": workspace_id,
            "layout_state": input.layout_state.clone(),
        });
        let event = NewKernelEvent::builder(
            run_id.clone(),
            run_id,
            KernelEventType::KnowledgeWorkbenchLayoutStateRecorded,
            KernelActor::System("workbench-layout-ui".to_string()),
        )
        .aggregate("workbench_layout_state", workspace_id.to_string())
        .source_component("workbench_layout_state")
        .payload(payload)
        .build()
        .map_err(|err| StorageError::Validation(kernel_event_build_error(err)))?;

        let mut tx = self.pool.begin().await?;
        let stored_event = append_kernel_event_with_executor(&mut *tx, event).await?;
        let row = sqlx::query(
            r#"
            INSERT INTO knowledge_workbench_layout_states (
                workspace_id,
                layout_state,
                updated_at,
                event_ledger_event_id
            )
            VALUES ($1, $2::jsonb, NOW(), $3)
            ON CONFLICT (workspace_id) DO UPDATE SET
                layout_state = EXCLUDED.layout_state,
                updated_at = NOW(),
                event_ledger_event_id = EXCLUDED.event_ledger_event_id
            RETURNING workspace_id, layout_state, updated_at, event_ledger_event_id
            "#,
        )
        .bind(workspace_id)
        .bind(&input.layout_state)
        .bind(&stored_event.event_id)
        .fetch_one(&mut *tx)
        .await?;
        let state = map_workbench_layout_state(&row)?;
        tx.commit().await?;
        Ok(state)
    }

    async fn get_workspace_settings_state(
        &self,
        workspace_id: &str,
    ) -> StorageResult<Option<WorkspaceSettingsState>> {
        let row = sqlx::query(
            r#"
            SELECT workspace_id, settings_state, updated_at, event_ledger_event_id
            FROM knowledge_workspace_settings_states
            WHERE workspace_id = $1
            "#,
        )
        .bind(workspace_id)
        .fetch_optional(&self.pool)
        .await?;
        row.as_ref().map(map_workspace_settings_state).transpose()
    }

    async fn save_workspace_settings_state(
        &self,
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
            .and_then(|value| value.as_str())
            != Some(WORKSPACE_SETTINGS_SCHEMA_ID)
        {
            return Err(StorageError::Validation(
                "workspace settings_state schema_id must be hsk.workspace_settings_state@1",
            ));
        }
        validate_workspace_settings_state_shape(&input.settings_state)?;

        let run_id = format!("WORKSPACE-SETTINGS-{workspace_id}");
        let payload = json!({
            "type": "knowledge_workspace_settings_state_recorded",
            "workspace_id": workspace_id,
            "settings_state": input.settings_state.clone(),
        });
        let event = NewKernelEvent::builder(
            run_id.clone(),
            run_id,
            KernelEventType::KnowledgeWorkspaceSettingsStateRecorded,
            KernelActor::System("workspace-settings-ui".to_string()),
        )
        .aggregate("workspace_settings_state", workspace_id.to_string())
        .source_component("workspace_settings_state")
        .payload(payload)
        .build()
        .map_err(|err| StorageError::Validation(kernel_event_build_error(err)))?;

        let mut tx = self.pool.begin().await?;
        let stored_event = append_kernel_event_with_executor(&mut *tx, event).await?;
        let row = sqlx::query(
            r#"
            INSERT INTO knowledge_workspace_settings_states (
                workspace_id,
                settings_state,
                updated_at,
                event_ledger_event_id
            )
            VALUES ($1, $2::jsonb, NOW(), $3)
            ON CONFLICT (workspace_id) DO UPDATE SET
                settings_state = EXCLUDED.settings_state,
                updated_at = NOW(),
                event_ledger_event_id = EXCLUDED.event_ledger_event_id
            RETURNING workspace_id, settings_state, updated_at, event_ledger_event_id
            "#,
        )
        .bind(workspace_id)
        .bind(&input.settings_state)
        .bind(&stored_event.event_id)
        .fetch_one(&mut *tx)
        .await?;
        let state = map_workspace_settings_state(&row)?;
        tx.commit().await?;
        Ok(state)
    }

    async fn get_workspace_search_bookmark_state(
        &self,
        workspace_id: &str,
    ) -> StorageResult<Option<WorkspaceSearchBookmarkState>> {
        let row = sqlx::query(
            r#"
            SELECT workspace_id, bookmark_state, updated_at, event_ledger_event_id
            FROM knowledge_workspace_search_bookmark_states
            WHERE workspace_id = $1
            "#,
        )
        .bind(workspace_id)
        .fetch_optional(&self.pool)
        .await?;
        row.as_ref()
            .map(map_workspace_search_bookmark_state)
            .transpose()
    }

    async fn save_workspace_search_bookmark_state(
        &self,
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
            .and_then(|value| value.as_str())
            != Some(WORKSPACE_SEARCH_BOOKMARK_SCHEMA_ID)
        {
            return Err(StorageError::Validation(
                "workspace search bookmark_state schema_id must be hsk.workspace_search_bookmark_state@1",
            ));
        }
        validate_workspace_search_bookmark_state_shape(&input.bookmark_state)?;

        let run_id = format!("WORKSPACE-SEARCH-BOOKMARKS-{workspace_id}");
        let payload = json!({
            "type": "knowledge_workspace_search_bookmark_state_recorded",
            "workspace_id": workspace_id,
            "bookmark_state": input.bookmark_state.clone(),
        });
        let event = NewKernelEvent::builder(
            run_id.clone(),
            run_id,
            KernelEventType::KnowledgeWorkspaceSearchBookmarkStateRecorded,
            KernelActor::System("workspace-search-bookmarks-ui".to_string()),
        )
        .aggregate(
            "workspace_search_bookmark_state",
            workspace_id.to_string(),
        )
        .source_component("workspace_search_bookmark_state")
        .payload(payload)
        .build()
        .map_err(|err| StorageError::Validation(kernel_event_build_error(err)))?;

        let mut tx = self.pool.begin().await?;
        let stored_event = append_kernel_event_with_executor(&mut *tx, event).await?;
        let row = sqlx::query(
            r#"
            INSERT INTO knowledge_workspace_search_bookmark_states (
                workspace_id,
                bookmark_state,
                updated_at,
                event_ledger_event_id
            )
            VALUES ($1, $2::jsonb, NOW(), $3)
            ON CONFLICT (workspace_id) DO UPDATE SET
                bookmark_state = EXCLUDED.bookmark_state,
                updated_at = NOW(),
                event_ledger_event_id = EXCLUDED.event_ledger_event_id
            RETURNING workspace_id, bookmark_state, updated_at, event_ledger_event_id
            "#,
        )
        .bind(workspace_id)
        .bind(&input.bookmark_state)
        .bind(&stored_event.event_id)
        .fetch_one(&mut *tx)
        .await?;
        let state = map_workspace_search_bookmark_state(&row)?;
        tx.commit().await?;
        Ok(state)
    }

    async fn list_debug_breakpoints(
        &self,
        rich_document_id: &str,
    ) -> StorageResult<Vec<DebugBreakpoint>> {
        let rows = sqlx::query(
            r#"
            SELECT breakpoint_id, rich_document_id, workspace_id, source_url, line,
                   condition, verified, updated_at, event_ledger_event_id
            FROM knowledge_debug_breakpoints
            WHERE rich_document_id = $1
            ORDER BY source_url ASC, line ASC
            "#,
        )
        .bind(rich_document_id)
        .fetch_all(&self.pool)
        .await?;
        rows.iter().map(map_debug_breakpoint).collect()
    }

    async fn set_debug_breakpoints(
        &self,
        rich_document_id: &str,
        workspace_id: &str,
        breakpoints: Vec<DebugBreakpointInput>,
    ) -> StorageResult<Vec<DebugBreakpoint>> {
        for bp in &breakpoints {
            if bp.line < 1 {
                return Err(StorageError::Validation(
                    "debug breakpoint line must be >= 1",
                ));
            }
            if bp.source_url.trim().is_empty() {
                return Err(StorageError::Validation(
                    "debug breakpoint source_url is required",
                ));
            }
        }

        let run_id = format!("DEBUG-BREAKPOINTS-{rich_document_id}");
        let payload = json!({
            "type": "knowledge_debug_breakpoints_recorded",
            "rich_document_id": rich_document_id,
            "workspace_id": workspace_id,
            "breakpoint_count": breakpoints.len(),
        });
        let event = NewKernelEvent::builder(
            run_id.clone(),
            run_id,
            KernelEventType::KnowledgeRichDocumentSaved,
            KernelActor::System("debug-breakpoints-ui".to_string()),
        )
        .aggregate("debug_breakpoints", rich_document_id.to_string())
        .source_component("debug_breakpoints")
        .payload(payload)
        .build()
        .map_err(|err| StorageError::Validation(kernel_event_build_error(err)))?;

        let mut tx = self.pool.begin().await?;
        let stored_event = append_kernel_event_with_executor(&mut *tx, event).await?;

        // PUT semantics: the request is the full breakpoint set for the doc.
        sqlx::query("DELETE FROM knowledge_debug_breakpoints WHERE rich_document_id = $1")
            .bind(rich_document_id)
            .execute(&mut *tx)
            .await?;

        for bp in &breakpoints {
            let breakpoint_id = format!("bp-{}", uuid::Uuid::new_v4());
            sqlx::query(
                r#"
                INSERT INTO knowledge_debug_breakpoints (
                    breakpoint_id, rich_document_id, workspace_id, source_url,
                    line, condition, verified, created_at, updated_at,
                    event_ledger_event_id
                )
                VALUES ($1, $2, $3, $4, $5, $6, $7, NOW(), NOW(), $8)
                "#,
            )
            .bind(&breakpoint_id)
            .bind(rich_document_id)
            .bind(workspace_id)
            .bind(&bp.source_url)
            .bind(bp.line)
            .bind(&bp.condition)
            .bind(bp.verified)
            .bind(&stored_event.event_id)
            .execute(&mut *tx)
            .await?;
        }

        let rows = sqlx::query(
            r#"
            SELECT breakpoint_id, rich_document_id, workspace_id, source_url, line,
                   condition, verified, updated_at, event_ledger_event_id
            FROM knowledge_debug_breakpoints
            WHERE rich_document_id = $1
            ORDER BY source_url ASC, line ASC
            "#,
        )
        .bind(rich_document_id)
        .fetch_all(&mut *tx)
        .await?;
        let out: Vec<DebugBreakpoint> =
            rows.iter().map(map_debug_breakpoint).collect::<StorageResult<_>>()?;
        tx.commit().await?;
        Ok(out)
    }

    async fn loom_visual_debug_snapshot(
        &self,
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

        let counts = self.loom_visual_debug_counts(workspace_id).await?;
        let local_graph = self
            .local_graph(workspace_id, start_block_id, 2, &[], cap)
            .await?;
        let backlinks = self
            .get_backlinks_with_context(workspace_id, start_block_id)
            .await?;
        let folders = self.list_loom_folders(workspace_id).await?;
        let search_hits = self
            .search_loom_graph(workspace_id, query, LoomSearchFilters::default(), cap, 0)
            .await?;

        let graph = LoomVisualDebugGraphState {
            scope: "local".to_string(),
            nodes: local_graph
                .nodes
                .into_iter()
                .map(loom_visual_debug_node_summary)
                .collect(),
            edges: local_graph
                .edges
                .into_iter()
                .map(loom_visual_debug_edge_summary)
                .collect(),
            truncated: local_graph.truncated,
            suppressed_hub_ids: local_graph.suppressed_hub_ids,
        };

        let backlinks = LoomVisualDebugBacklinkState {
            target_block_id: start_block_id.to_string(),
            incoming: backlinks
                .into_iter()
                .take(cap as usize)
                .map(loom_visual_debug_backlink_summary)
                .collect(),
        };

        let mut folder_summaries = Vec::new();
        for folder in folders.into_iter().take(cap as usize) {
            let member_count = self
                .loom_folder_member_count(workspace_id, &folder.folder_id)
                .await?;
            let sample_block_ids = self
                .list_loom_folder_blocks(workspace_id, &folder.folder_id, folder_sample_limit, 0)
                .await?
                .into_iter()
                .map(|block| block.block_id)
                .collect();
            folder_summaries.push(loom_visual_debug_folder_summary(
                folder,
                member_count,
                sample_block_ids,
            ));
        }

        let result_count = search_hits.len();
        let search = LoomVisualDebugSearchState {
            query: query.to_string(),
            result_count,
            results: search_hits
                .into_iter()
                .map(loom_visual_debug_search_summary)
                .collect(),
        };

        Ok(LoomVisualDebugSnapshot {
            workspace_id: workspace_id.to_string(),
            schema_id: LOOM_VISUAL_DEBUG_SCHEMA_ID,
            authority_backend: super::LoomAuthorityBackend::PostgresEventLedger,
            authority_class: "projection",
            start_block_id: start_block_id.to_string(),
            route_ids: loom_visual_debug_route_ids(),
            counts,
            graph,
            backlinks,
            folders: folder_summaries,
            search,
        })
    }

    // -- MT-177 LoomBlockKnowledgeBridge ---------------------------------------

    async fn bridge_loom_block_to_knowledge(
        &self,
        ctx: &WriteContext,
        workspace_id: &str,
        block_id: &str,
    ) -> StorageResult<super::LoomKnowledgeBridge> {
        use crate::storage::knowledge::{KnowledgeEntityKind, KnowledgeStore, NewKnowledgeEntity};

        // 1. The block must exist and belong to the workspace. This both
        //    fail-closes on a missing/foreign block and gives us the
        //    display_name for the ProjectKnowledgeIndex entity.
        let block = self.get_loom_block(workspace_id, block_id).await?;

        // A knowledge entity REQUIRES a non-empty display_name (0135 CHECK).
        // A LoomBlock title/filename can be absent (e.g. an imported file with
        // no title yet), so fall back to a stable, human-meaningful label
        // derived from the block id and content type. NEVER an absolute path.
        let display_name = block
            .title
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .or_else(|| {
                block
                    .original_filename
                    .as_deref()
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
            })
            .map(|value| value.to_string())
            .unwrap_or_else(|| format!("{} {}", block.content_type.as_str(), block.block_id));

        // 2. Upsert the ProjectKnowledgeIndex authority entity. Natural identity
        //    (workspace, 'loom_block', block_id) — stable + idempotent.
        let entity = self
            .upsert_knowledge_entity(NewKnowledgeEntity {
                workspace_id: workspace_id.to_string(),
                entity_kind: KnowledgeEntityKind::LoomBlock,
                entity_key: block.block_id.clone(),
                display_name,
                detection_provenance: json!({
                    "extractor": "loom_block_knowledge_bridge",
                    "extractor_version": LOOM_KNOWLEDGE_BRIDGE_EXTRACTOR_VERSION,
                    "method": "mt177_bridge",
                    "content_type": block.content_type.as_str(),
                }),
                primary_source_id: None,
                detected_in_run: None,
                evidence_span_ids: Vec::new(),
            })
            .await?;

        // 3. Append the EventLedger receipt (KNOWLEDGE_LOOM_BLOCK_INDEXED).
        //    EventLedger is authority (§10.12 #9.1.1); the bridge row's
        //    index_event_id FK proves a receipt exists. The bridge is a
        //    system-internal indexing operation, so it uses a deterministic
        //    Loom-scoped synthetic run id (mirrors KnowledgeIndexRun events
        //    that are not driven by an interactive session).
        let actor = kernel_actor_for_bridge(ctx);
        let run_id = format!("LOOM-BRIDGE-{workspace_id}");
        let payload = json!({
            "type": "knowledge_loom_block_indexed",
            "workspace_id": workspace_id,
            "block_id": block.block_id,
            "entity_id": entity.entity_id,
            "content_type": block.content_type.as_str(),
            "extractor_version": LOOM_KNOWLEDGE_BRIDGE_EXTRACTOR_VERSION,
        });
        let event = NewKernelEvent::builder(
            run_id.clone(),
            run_id,
            KernelEventType::KnowledgeLoomBlockIndexed,
            actor,
        )
        .aggregate("knowledge_loom_block", entity.entity_id.clone())
        .idempotency_key(format!(
            "KEI-loom-bridge-{}-{}",
            entity.entity_id,
            entity.updated_at.timestamp_nanos_opt().unwrap_or_default()
        ))
        .source_component("loom_block_knowledge_bridge")
        .payload(payload)
        .build()
        .map_err(|err| StorageError::Validation(kernel_event_build_error(err)))?;
        let stored_event = self.append_kernel_event(event).await?;

        // 4. Upsert the authority bridge row (block_id -> entity_id + receipt).
        let row = sqlx::query(
            r#"
            INSERT INTO loom_block_knowledge_bridge
                (block_id, workspace_id, entity_id, index_event_id)
            VALUES ($1, $2, $3, $4)
            ON CONFLICT (block_id) DO UPDATE SET
                entity_id = EXCLUDED.entity_id,
                index_event_id = EXCLUDED.index_event_id,
                updated_at = NOW()
            RETURNING block_id, workspace_id, entity_id, index_event_id,
                      created_at, updated_at
            "#,
        )
        .bind(&block.block_id)
        .bind(workspace_id)
        .bind(&entity.entity_id)
        .bind(&stored_event.event_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(map_loom_knowledge_bridge(&row))
    }

    async fn get_loom_block_knowledge_bridge(
        &self,
        workspace_id: &str,
        block_id: &str,
    ) -> StorageResult<Option<super::LoomKnowledgeBridge>> {
        let row = sqlx::query(
            r#"
            SELECT block_id, workspace_id, entity_id, index_event_id,
                   created_at, updated_at
            FROM loom_block_knowledge_bridge
            WHERE workspace_id = $1 AND block_id = $2
            "#,
        )
        .bind(workspace_id)
        .bind(block_id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.as_ref().map(map_loom_knowledge_bridge))
    }

    async fn list_loom_block_knowledge_bridges(
        &self,
        workspace_id: &str,
    ) -> StorageResult<Vec<super::LoomKnowledgeBridge>> {
        let rows = sqlx::query(
            r#"
            SELECT block_id, workspace_id, entity_id, index_event_id,
                   created_at, updated_at
            FROM loom_block_knowledge_bridge
            WHERE workspace_id = $1
            ORDER BY created_at ASC, block_id ASC
            "#,
        )
        .bind(workspace_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.iter().map(map_loom_knowledge_bridge).collect())
    }

    // -- MT-178 BacklinkComputation --------------------------------------------

    async fn get_backlinks_with_context(
        &self,
        workspace_id: &str,
        block_id: &str,
    ) -> StorageResult<Vec<super::LoomBacklink>> {
        // The viewed block must exist (clean fail-closed; also gives us the
        // title used to locate the reference inside each source block's text).
        let target = self.get_loom_block(workspace_id, block_id).await?;
        let target_title = target
            .title
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(|value| value.to_string());

        // Reuse the existing incoming-edge query (authority = loom_edges).
        let edges = self.get_backlinks(workspace_id, block_id).await?;

        let mut backlinks = Vec::with_capacity(edges.len());
        for edge in edges {
            let source_block = self
                .get_loom_block(workspace_id, &edge.source_block_id)
                .await?;
            let scanned = loom_block_scan_text(&source_block);
            // Locate the reference: prefer the viewed block's title; otherwise
            // fall back to a leading window of the source block's own text.
            let context_snippet = target_title
                .as_deref()
                .and_then(|title| {
                    super::loom_find_unlinked_term(&scanned, title)
                        .map(|(start, len)| super::loom_context_snippet(&scanned, start, len))
                })
                .or_else(|| {
                    let trimmed = scanned.trim();
                    if trimmed.is_empty() {
                        None
                    } else {
                        Some(super::loom_context_snippet(&scanned, 0, 0))
                    }
                });
            backlinks.push(super::LoomBacklink {
                edge,
                source_block,
                context_snippet,
            });
        }
        Ok(backlinks)
    }

    async fn scan_unlinked_mentions(
        &self,
        workspace_id: &str,
        block_id: &str,
        aliases: &[String],
        limit: u32,
    ) -> StorageResult<Vec<super::LoomUnlinkedMention>> {
        let target = self.get_loom_block(workspace_id, block_id).await?;

        // The terms to scan for: the block's own title plus any provided
        // aliases. Empty/whitespace terms are dropped.
        let mut terms: Vec<String> = Vec::new();
        if let Some(title) = target
            .title
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            terms.push(title.to_string());
        }
        for alias in aliases {
            let trimmed = alias.trim();
            if !trimmed.is_empty() {
                terms.push(trimmed.to_string());
            }
        }
        if terms.is_empty() {
            // A block with no title/aliases has nothing to be unlinked-mentioned
            // by; return empty rather than scanning the whole workspace.
            return Ok(Vec::new());
        }

        // Candidate source blocks: any OTHER block in the workspace whose
        // title / original_filename / full_text_index contains a term
        // (cheap ILIKE prefilter), that does NOT already have a formal
        // MENTION/TAG edge to the target (those are LINKED, not unlinked).
        let scan_limit = limit.clamp(1, 500) as i64;
        let mut qb = sqlx::QueryBuilder::<sqlx::Postgres>::new(
            r#"
            SELECT
                b.block_id, b.workspace_id, b.content_type, b.document_id,
                b.asset_id, b.title, b.original_filename, b.content_hash,
                b.pinned, b.favorite, b.journal_date, b.created_at, b.updated_at,
                b.imported_at, b.backlink_count, b.mention_count, b.tag_count,
                b.derived_json, b.preview_status, b.thumbnail_asset_id,
                b.proxy_asset_id
            FROM loom_blocks b
            WHERE b.workspace_id =
            "#,
        );
        qb.push_bind(workspace_id);
        qb.push(" AND b.block_id <> ").push_bind(block_id);
        // Not already linked to the target by a mention/tag edge.
        qb.push(
            r#" AND NOT EXISTS (
                SELECT 1 FROM loom_edges e
                WHERE e.workspace_id = b.workspace_id
                  AND e.source_block_id = b.block_id
                  AND e.target_block_id = "#,
        );
        qb.push_bind(block_id);
        qb.push(" AND e.edge_type IN ('mention','tag','sub_tag')) ");
        // ILIKE prefilter on any term.
        qb.push(" AND ( ");
        for (idx, term) in terms.iter().enumerate() {
            if idx > 0 {
                qb.push(" OR ");
            }
            let pattern = format!("%{}%", escape_like_token(term));
            qb.push(" COALESCE(b.title,'') ILIKE ")
                .push_bind(pattern.clone());
            qb.push(" OR COALESCE(b.original_filename,'') ILIKE ")
                .push_bind(pattern.clone());
            qb.push(" OR COALESCE((b.derived_json::jsonb ->> 'full_text_index'),'') ILIKE ")
                .push_bind(pattern);
        }
        qb.push(" ) ");
        qb.push(" ORDER BY b.updated_at DESC, b.block_id ASC ");
        qb.push(" LIMIT ").push_bind(scan_limit);

        let rows = qb.build().fetch_all(&self.pool).await?;

        let mut mentions = Vec::new();
        for row in rows {
            let source_block = map_loom_block(row)?;
            let scanned = loom_block_scan_text(&source_block);
            // Precise word-boundary detection + offset for the first matching
            // term (the ILIKE prefilter can over-match substrings; this is the
            // authoritative unlinked-mention test).
            let mut best: Option<(String, usize, usize)> = None;
            for term in &terms {
                if let Some((start, len)) = super::loom_find_unlinked_term(&scanned, term) {
                    let is_better = match &best {
                        Some((_, best_start, _)) => start < *best_start,
                        None => true,
                    };
                    if is_better {
                        best = Some((term.clone(), start, len));
                    }
                }
            }
            if let Some((matched_term, start, len)) = best {
                let snippet = super::loom_context_snippet(&scanned, start, len);
                mentions.push(super::LoomUnlinkedMention {
                    source_block,
                    matched_term,
                    snippet,
                    match_offset: start as i64,
                });
            }
        }
        Ok(mentions)
    }

    // -- MT-179 LocalGraphApi --------------------------------------------------

    async fn local_graph(
        &self,
        workspace_id: &str,
        start_block_id: &str,
        max_depth: u32,
        edge_types: &[LoomEdgeType],
        node_limit: u32,
    ) -> StorageResult<super::LoomGraph> {
        // Start must exist (fail-closed).
        self.get_loom_block(workspace_id, start_block_id).await?;

        let depth = max_depth.max(1);
        let cap = node_limit.clamp(1, 5000) as i64;
        let edge_type_filter = (!edge_types.is_empty()).then(|| {
            edge_types
                .iter()
                .map(|e| e.as_str().to_string())
                .collect::<Vec<_>>()
        });

        // UNDIRECTED neighborhood: a local graph shows what a block is connected
        // to in BOTH directions. The recursive CTE walks edges treating each as
        // bidirectional (source<->target), with a visited-path cycle guard.
        // We collect the node id set (with min BFS depth) including the start.
        let id_rows = sqlx::query(
            r#"
            WITH RECURSIVE nbr(block_id, depth, path) AS (
                SELECT $2::TEXT, 0, ARRAY[$2]::TEXT[]
                UNION ALL
                SELECT
                    nxt.other,
                    n.depth + 1,
                    n.path || nxt.other
                FROM nbr n
                JOIN LATERAL (
                    SELECT e.target_block_id AS other
                    FROM loom_edges e
                    WHERE e.workspace_id = $1
                      AND e.source_block_id = n.block_id
                      AND ($4::TEXT[] IS NULL OR e.edge_type = ANY($4::TEXT[]))
                    UNION
                    SELECT e.source_block_id AS other
                    FROM loom_edges e
                    WHERE e.workspace_id = $1
                      AND e.target_block_id = n.block_id
                      AND ($4::TEXT[] IS NULL OR e.edge_type = ANY($4::TEXT[]))
                ) nxt ON TRUE
                WHERE n.depth < $3
                  AND NOT nxt.other = ANY(n.path)
            )
            SELECT block_id, MIN(depth)::BIGINT AS depth
            FROM nbr
            GROUP BY block_id
            ORDER BY depth ASC, block_id ASC
            LIMIT $5
            "#,
        )
        .bind(workspace_id)
        .bind(start_block_id)
        .bind(depth as i32)
        .bind(edge_type_filter.as_ref())
        .bind(cap + 1) // fetch one extra to detect truncation
        .fetch_all(&self.pool)
        .await?;

        let truncated = id_rows.len() as i64 > cap;
        let kept: Vec<(String, i64)> = id_rows
            .into_iter()
            .take(cap as usize)
            .map(|r| (r.get::<String, _>("block_id"), r.get::<i64, _>("depth")))
            .collect();
        let depth_by_id: std::collections::HashMap<String, u32> =
            kept.iter().map(|(id, d)| (id.clone(), *d as u32)).collect();
        let node_ids: Vec<String> = kept.iter().map(|(id, _)| id.clone()).collect();

        self.assemble_loom_graph(
            workspace_id,
            &node_ids,
            &depth_by_id,
            edge_type_filter.as_ref(),
            truncated,
            Vec::new(),
        )
        .await
    }

    // -- MT-180 GlobalGraphApi -------------------------------------------------

    async fn global_graph(
        &self,
        workspace_id: &str,
        edge_types: &[LoomEdgeType],
        node_limit: u32,
        hub_degree_threshold: u32,
    ) -> StorageResult<super::LoomGraph> {
        let cap = node_limit.clamp(1, super::LOOM_GLOBAL_GRAPH_MAX_NODE_LIMIT) as i64;
        let edge_type_filter = (!edge_types.is_empty()).then(|| {
            edge_types
                .iter()
                .map(|e| e.as_str().to_string())
                .collect::<Vec<_>>()
        });

        // Degree per node across the (filtered) edge set: count an edge once for
        // its source and once for its target.
        let degree_rows = sqlx::query(
            r#"
            SELECT block_id, COUNT(*)::BIGINT AS degree
            FROM (
                SELECT source_block_id AS block_id FROM loom_edges
                WHERE workspace_id = $1
                  AND ($2::TEXT[] IS NULL OR edge_type = ANY($2::TEXT[]))
                UNION ALL
                SELECT target_block_id AS block_id FROM loom_edges
                WHERE workspace_id = $1
                  AND ($2::TEXT[] IS NULL OR edge_type = ANY($2::TEXT[]))
            ) deg
            GROUP BY block_id
            "#,
        )
        .bind(workspace_id)
        .bind(edge_type_filter.as_ref())
        .fetch_all(&self.pool)
        .await?;

        let mut degree_by_id: std::collections::HashMap<String, u32> =
            std::collections::HashMap::new();
        for row in &degree_rows {
            degree_by_id.insert(row.get("block_id"), row.get::<i64, _>("degree") as u32);
        }

        // Hub suppression: drop nodes whose total degree exceeds the threshold.
        let mut suppressed: Vec<String> = degree_by_id
            .iter()
            .filter(|(_, &d)| hub_degree_threshold > 0 && d > hub_degree_threshold)
            .map(|(id, _)| id.clone())
            .collect();
        suppressed.sort();

        // Candidate node ids: every block in the workspace, minus suppressed
        // hubs, capped for performance (deterministic order).
        let block_id_rows = sqlx::query(
            r#"
            SELECT block_id
            FROM loom_blocks
            WHERE workspace_id = $1
            ORDER BY updated_at DESC, block_id ASC
            "#,
        )
        .bind(workspace_id)
        .fetch_all(&self.pool)
        .await?;

        let suppressed_set: std::collections::HashSet<&String> = suppressed.iter().collect();
        let all_ids: Vec<String> = block_id_rows
            .into_iter()
            .map(|r| r.get::<String, _>("block_id"))
            .filter(|id| !suppressed_set.contains(id))
            .collect();
        let truncated = all_ids.len() as i64 > cap;
        let node_ids: Vec<String> = all_ids.into_iter().take(cap as usize).collect();

        // depth is 0 for the global graph (no single origin).
        let depth_by_id: std::collections::HashMap<String, u32> =
            node_ids.iter().map(|id| (id.clone(), 0u32)).collect();

        self.assemble_loom_graph(
            workspace_id,
            &node_ids,
            &depth_by_id,
            edge_type_filter.as_ref(),
            truncated,
            suppressed,
        )
        .await
    }

    // -- MT-182 TagsAndTagHubs -------------------------------------------------

    async fn list_tag_hubs(
        &self,
        workspace_id: &str,
        limit: u32,
        offset: u32,
    ) -> StorageResult<Vec<LoomBlock>> {
        let limit_i64 = limit.clamp(1, 500) as i64;
        let offset_i64 = offset as i64;
        let rows = sqlx::query(
            r#"
            SELECT
                b.block_id, b.workspace_id, b.content_type, b.document_id,
                b.asset_id, b.title, b.original_filename, b.content_hash,
                b.pinned, b.favorite, b.journal_date, b.created_at, b.updated_at,
                b.imported_at, b.backlink_count, b.mention_count, b.tag_count,
                b.derived_json, b.preview_status, b.thumbnail_asset_id,
                b.proxy_asset_id
            FROM loom_blocks b
            WHERE b.workspace_id = $1 AND b.content_type = 'tag_hub'
            ORDER BY b.updated_at DESC, b.block_id ASC
            LIMIT $2 OFFSET $3
            "#,
        )
        .bind(workspace_id)
        .bind(limit_i64)
        .bind(offset_i64)
        .fetch_all(&self.pool)
        .await?;
        rows.into_iter().map(map_loom_block).collect()
    }

    async fn get_tag_hub(
        &self,
        workspace_id: &str,
        tag_block_id: &str,
    ) -> StorageResult<super::LoomTagHub> {
        // Must exist AND be a tag_hub (fail-closed).
        let block = self.get_loom_block(workspace_id, tag_block_id).await?;
        if !matches!(block.content_type, LoomBlockContentType::TagHub) {
            return Err(StorageError::Validation("loom block is not a tag_hub"));
        }

        // Direct sub-tags: SOURCES of SUB_TAG edges whose TARGET is this tag.
        let sub_tags = self
            .loom_blocks_by_incoming_edge(workspace_id, tag_block_id, "sub_tag")
            .await?;
        // Tagged blocks: SOURCES of TAG edges whose TARGET is this tag.
        let tagged_blocks = self
            .loom_blocks_by_incoming_edge(workspace_id, tag_block_id, "tag")
            .await?;

        let backlink_count: i64 = sqlx::query_scalar(
            r#"SELECT COUNT(*)::BIGINT FROM loom_edges
               WHERE workspace_id = $1 AND target_block_id = $2"#,
        )
        .bind(workspace_id)
        .bind(tag_block_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(super::LoomTagHub {
            block,
            sub_tags,
            tagged_blocks,
            backlink_count,
        })
    }

    async fn list_blocks_for_tag(
        &self,
        workspace_id: &str,
        tag_block_id: &str,
        include_subtags: bool,
        limit: u32,
        offset: u32,
    ) -> StorageResult<Vec<LoomBlock>> {
        let block = self.get_loom_block(workspace_id, tag_block_id).await?;
        if !matches!(block.content_type, LoomBlockContentType::TagHub) {
            return Err(StorageError::Validation("loom block is not a tag_hub"));
        }

        let limit_i64 = limit.clamp(1, 500) as i64;
        let offset_i64 = offset as i64;

        // The set of tag ids to match: the tag itself, plus (when requested) all
        // DESCENDANT sub-tags via the SUB_TAG hierarchy (child SOURCE ->
        // parent TARGET), cycle-guarded.
        let rows = sqlx::query(
            r#"
            WITH RECURSIVE tag_set(tag_id, path) AS (
                SELECT $2::TEXT, ARRAY[$2]::TEXT[]
                UNION ALL
                SELECT e.source_block_id, ts.path || e.source_block_id
                FROM loom_edges e
                JOIN tag_set ts ON e.target_block_id = ts.tag_id
                WHERE e.workspace_id = $1
                  AND e.edge_type = 'sub_tag'
                  AND $4::BOOL = TRUE
                  AND NOT e.source_block_id = ANY(ts.path)
            )
            SELECT DISTINCT
                b.block_id, b.workspace_id, b.content_type, b.document_id,
                b.asset_id, b.title, b.original_filename, b.content_hash,
                b.pinned, b.favorite, b.journal_date, b.created_at, b.updated_at,
                b.imported_at, b.backlink_count, b.mention_count, b.tag_count,
                b.derived_json, b.preview_status, b.thumbnail_asset_id,
                b.proxy_asset_id
            FROM loom_edges te
            JOIN loom_blocks b
              ON b.workspace_id = te.workspace_id
             AND b.block_id = te.source_block_id
            WHERE te.workspace_id = $1
              AND te.edge_type = 'tag'
              AND te.target_block_id IN (SELECT tag_id FROM tag_set)
            ORDER BY b.updated_at DESC, b.block_id ASC
            LIMIT $3 OFFSET $5
            "#,
        )
        .bind(workspace_id)
        .bind(tag_block_id)
        .bind(limit_i64)
        .bind(include_subtags)
        .bind(offset_i64)
        .fetch_all(&self.pool)
        .await?;
        rows.into_iter().map(map_loom_block).collect()
    }

    // -- MT-183 PinsFavoritesAndUnlinked ---------------------------------------

    async fn set_loom_block_pin_order(
        &self,
        ctx: &WriteContext,
        workspace_id: &str,
        block_id: &str,
        pin_order: Option<i32>,
    ) -> StorageResult<LoomBlock> {
        let now = Utc::now();
        let metadata = self.guard.validate_write(ctx, block_id).await?;
        let actor_kind = metadata.actor_kind.as_str();
        let actor_id = metadata.actor_id.clone();
        let job_id = metadata.job_id.map(|v| v.to_string());
        let workflow_id = metadata.workflow_id.map(|v| v.to_string());
        let edit_event_id = metadata.edit_event_id.to_string();

        // Directly SET pin_order (NULL clears it). $1 is bound as Option<i32>,
        // so NULL is a real value here, not a no-op like COALESCE.
        let row = sqlx::query(
            r#"
            UPDATE loom_blocks
            SET
                pin_order = $1,
                last_actor_kind = $2,
                last_actor_id = $3,
                last_job_id = $4,
                last_workflow_id = $5,
                edit_event_id = $6,
                updated_at = $7
            WHERE workspace_id = $8 AND block_id = $9
            RETURNING
                block_id, workspace_id, content_type, document_id, asset_id,
                title, original_filename, content_hash, pinned, favorite, pin_order,
                journal_date, created_at, updated_at, imported_at,
                backlink_count, mention_count, tag_count, derived_json,
                preview_status, thumbnail_asset_id, proxy_asset_id
            "#,
        )
        .bind(pin_order)
        .bind(actor_kind)
        .bind(actor_id)
        .bind(job_id)
        .bind(workflow_id)
        .bind(edit_event_id)
        .bind(now)
        .bind(workspace_id)
        .bind(block_id)
        .fetch_optional(&self.pool)
        .await?;

        match row {
            Some(row) => map_loom_block(row),
            None => Err(StorageError::NotFound("loom_block")),
        }
    }

    // -- MT-181 FolderTreeAndColorLabels ---------------------------------------

    async fn create_loom_folder(
        &self,
        workspace_id: &str,
        folder: super::NewLoomFolder,
    ) -> StorageResult<super::LoomFolder> {
        let name = folder.name.trim();
        if name.is_empty() {
            return Err(StorageError::Validation("loom folder name is required"));
        }
        // Parent (if given) must exist in this workspace.
        if let Some(parent_id) = &folder.parent_folder_id {
            self.get_loom_folder(workspace_id, parent_id).await?;
        }
        let id = folder
            .folder_id
            .unwrap_or_else(|| format!("LFD-{}", Uuid::now_v7().simple()));
        let row = sqlx::query(
            r#"
            INSERT INTO loom_folders
                (folder_id, workspace_id, parent_folder_id, name, color,
                 sort_mode, sort_order, project_ref)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            RETURNING folder_id, workspace_id, parent_folder_id, name, color,
                      sort_mode, sort_order, project_ref, created_at, updated_at
            "#,
        )
        .bind(&id)
        .bind(workspace_id)
        .bind(&folder.parent_folder_id)
        .bind(name)
        .bind(folder.color.as_deref().map(str::trim))
        .bind(folder.sort_mode.as_str())
        .bind(folder.sort_order)
        .bind(folder.project_ref.as_deref())
        .fetch_one(&self.pool)
        .await?;
        map_loom_folder(&row)
    }

    async fn get_loom_folder(
        &self,
        workspace_id: &str,
        folder_id: &str,
    ) -> StorageResult<super::LoomFolder> {
        let row = sqlx::query(
            r#"
            SELECT folder_id, workspace_id, parent_folder_id, name, color,
                   sort_mode, sort_order, project_ref, created_at, updated_at
            FROM loom_folders
            WHERE workspace_id = $1 AND folder_id = $2
            "#,
        )
        .bind(workspace_id)
        .bind(folder_id)
        .fetch_optional(&self.pool)
        .await?;
        match row {
            Some(row) => map_loom_folder(&row),
            None => Err(StorageError::NotFound("loom_folder")),
        }
    }

    async fn list_loom_folders(&self, workspace_id: &str) -> StorageResult<Vec<super::LoomFolder>> {
        // Parent-before-child order via a recursive walk from the roots, so a
        // caller can build the tree in one pass.
        let rows = sqlx::query(
            r#"
            WITH RECURSIVE tree(folder_id, depth) AS (
                SELECT folder_id, 0
                FROM loom_folders
                WHERE workspace_id = $1 AND parent_folder_id IS NULL
                UNION ALL
                SELECT f.folder_id, t.depth + 1
                FROM loom_folders f
                JOIN tree t ON f.parent_folder_id = t.folder_id
                WHERE f.workspace_id = $1
            )
            SELECT f.folder_id, f.workspace_id, f.parent_folder_id, f.name,
                   f.color, f.sort_mode, f.sort_order, f.project_ref,
                   f.created_at, f.updated_at
            FROM tree t
            JOIN loom_folders f ON f.folder_id = t.folder_id
            ORDER BY t.depth ASC,
                     f.sort_order ASC NULLS LAST,
                     f.name ASC,
                     f.folder_id ASC
            "#,
        )
        .bind(workspace_id)
        .fetch_all(&self.pool)
        .await?;
        rows.iter().map(map_loom_folder).collect()
    }

    async fn update_loom_folder(
        &self,
        workspace_id: &str,
        folder_id: &str,
        update: super::LoomFolderUpdate,
    ) -> StorageResult<super::LoomFolder> {
        // Folder must exist.
        let _existing = self.get_loom_folder(workspace_id, folder_id).await?;

        // Re-parent cycle guard: the new parent cannot be the folder itself or
        // any of its descendants (that would create a cycle / detach a subtree).
        if let Some(new_parent) = &update.parent_folder_id {
            if let Some(parent_id) = new_parent {
                if parent_id == folder_id {
                    return Err(StorageError::Validation(
                        "loom folder cannot be its own parent",
                    ));
                }
                // Parent must exist in this workspace.
                self.get_loom_folder(workspace_id, parent_id).await?;
                // Walk up from the proposed parent; if we reach folder_id, the
                // move would form a cycle.
                let is_descendant: bool = sqlx::query_scalar(
                    r#"
                    WITH RECURSIVE up(folder_id) AS (
                        SELECT $2::TEXT
                        UNION ALL
                        SELECT f.parent_folder_id
                        FROM loom_folders f
                        JOIN up ON f.folder_id = up.folder_id
                        WHERE f.workspace_id = $1 AND f.parent_folder_id IS NOT NULL
                    )
                    SELECT EXISTS (SELECT 1 FROM up WHERE folder_id = $3)
                    "#,
                )
                .bind(workspace_id)
                .bind(parent_id)
                .bind(folder_id)
                .fetch_one(&self.pool)
                .await?;
                if is_descendant {
                    return Err(StorageError::Validation(
                        "loom folder move would create a cycle",
                    ));
                }
            }
        }

        // Build the update with COALESCE-style semantics, distinguishing
        // "leave unchanged" (outer None) from "set/clear" (Some).
        let new_name = update.name.as_deref().map(str::trim);
        if let Some(n) = new_name {
            if n.is_empty() {
                return Err(StorageError::Validation("loom folder name is required"));
            }
        }
        let row = sqlx::query(
            r#"
            UPDATE loom_folders
            SET
                name = COALESCE($3, name),
                color = CASE WHEN $4::BOOL THEN $5 ELSE color END,
                sort_mode = COALESCE($6, sort_mode),
                sort_order = CASE WHEN $7::BOOL THEN $8 ELSE sort_order END,
                parent_folder_id = CASE WHEN $9::BOOL THEN $10 ELSE parent_folder_id END,
                project_ref = CASE WHEN $11::BOOL THEN $12 ELSE project_ref END,
                updated_at = NOW()
            WHERE workspace_id = $1 AND folder_id = $2
            RETURNING folder_id, workspace_id, parent_folder_id, name, color,
                      sort_mode, sort_order, project_ref, created_at, updated_at
            "#,
        )
        .bind(workspace_id)
        .bind(folder_id)
        .bind(new_name)
        .bind(update.color.is_some())
        .bind(update.color.clone().flatten().as_deref().map(str::trim))
        .bind(update.sort_mode.map(|m| m.as_str()))
        .bind(update.sort_order.is_some())
        .bind(update.sort_order.flatten())
        .bind(update.parent_folder_id.is_some())
        .bind(update.parent_folder_id.clone().flatten())
        .bind(update.project_ref.is_some())
        .bind(update.project_ref.clone().flatten().as_deref())
        .fetch_one(&self.pool)
        .await?;
        map_loom_folder(&row)
    }

    async fn delete_loom_folder(&self, workspace_id: &str, folder_id: &str) -> StorageResult<()> {
        let res =
            sqlx::query("DELETE FROM loom_folders WHERE workspace_id = $1 AND folder_id = $2")
                .bind(workspace_id)
                .bind(folder_id)
                .execute(&self.pool)
                .await?;
        if res.rows_affected() == 0 {
            return Err(StorageError::NotFound("loom_folder"));
        }
        Ok(())
    }

    async fn add_block_to_loom_folder(
        &self,
        workspace_id: &str,
        folder_id: &str,
        block_id: &str,
        sort_order: Option<i32>,
    ) -> StorageResult<()> {
        // Folder + block must both exist in the workspace (fail-closed).
        self.get_loom_folder(workspace_id, folder_id).await?;
        self.get_loom_block(workspace_id, block_id).await?;
        sqlx::query(
            r#"
            INSERT INTO loom_folder_members (folder_id, block_id, workspace_id, sort_order)
            VALUES ($1, $2, $3, $4)
            ON CONFLICT (folder_id, block_id)
            DO UPDATE SET sort_order = EXCLUDED.sort_order
            "#,
        )
        .bind(folder_id)
        .bind(block_id)
        .bind(workspace_id)
        .bind(sort_order)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn remove_block_from_loom_folder(
        &self,
        workspace_id: &str,
        folder_id: &str,
        block_id: &str,
    ) -> StorageResult<()> {
        sqlx::query(
            r#"DELETE FROM loom_folder_members
               WHERE workspace_id = $1 AND folder_id = $2 AND block_id = $3"#,
        )
        .bind(workspace_id)
        .bind(folder_id)
        .bind(block_id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn list_loom_folder_blocks(
        &self,
        workspace_id: &str,
        folder_id: &str,
        limit: u32,
        offset: u32,
    ) -> StorageResult<Vec<LoomBlock>> {
        let folder = self.get_loom_folder(workspace_id, folder_id).await?;
        let limit_i64 = limit.clamp(1, 500) as i64;
        let offset_i64 = offset as i64;

        // The ORDER BY is chosen by the folder's sort_mode (fixed literals;
        // never user input).
        let order_by = match folder.sort_mode {
            super::LoomFolderSortMode::NameAsc => {
                "ORDER BY COALESCE(b.title,'') ASC, b.block_id ASC"
            }
            super::LoomFolderSortMode::NameDesc => {
                "ORDER BY COALESCE(b.title,'') DESC, b.block_id ASC"
            }
            super::LoomFolderSortMode::CreatedDesc => "ORDER BY b.created_at DESC, b.block_id ASC",
            super::LoomFolderSortMode::UpdatedDesc => "ORDER BY b.updated_at DESC, b.block_id ASC",
            super::LoomFolderSortMode::Manual => {
                "ORDER BY m.sort_order ASC NULLS LAST, b.updated_at DESC, b.block_id ASC"
            }
        };

        let sql = format!(
            r#"
            SELECT
                b.block_id, b.workspace_id, b.content_type, b.document_id,
                b.asset_id, b.title, b.original_filename, b.content_hash,
                b.pinned, b.favorite, b.pin_order, b.journal_date, b.created_at, b.updated_at,
                b.imported_at, b.backlink_count, b.mention_count, b.tag_count,
                b.derived_json, b.preview_status, b.thumbnail_asset_id,
                b.proxy_asset_id
            FROM loom_folder_members m
            JOIN loom_blocks b
              ON b.workspace_id = m.workspace_id AND b.block_id = m.block_id
            WHERE m.workspace_id = $1 AND m.folder_id = $2
            {order_by}
            LIMIT $3 OFFSET $4
            "#
        );
        let rows = sqlx::query(&sql)
            .bind(workspace_id)
            .bind(folder_id)
            .bind(limit_i64)
            .bind(offset_i64)
            .fetch_all(&self.pool)
            .await?;
        rows.into_iter().map(map_loom_block).collect()
    }

    // -- MT-184 WikiPageProjectionCompiler -------------------------------------

    async fn compile_loom_wiki_projection(
        &self,
        workspace_id: &str,
        title: &str,
        block_ids: &[String],
    ) -> StorageResult<super::LoomWikiProjection> {
        use crate::knowledge_wiki::{
            loom_block_content_hash, CitedSource, CitedSourceKind, WikiCompileStamp,
        };
        use crate::storage::knowledge::NewKnowledgeWikiPage;
        let title = title.trim();
        if title.is_empty() {
            return Err(StorageError::Validation(
                "loom wiki projection title is required",
            ));
        }
        // Load the source blocks (in the given order, skipping unknown ids would
        // hide errors — instead fail closed on a missing block so citations are
        // always valid).
        let mut blocks = Vec::with_capacity(block_ids.len());
        for id in block_ids {
            blocks.push(self.get_loom_block(workspace_id, id).await?);
        }

        let rendered_content = render_loom_wiki_markdown(title, &blocks);
        let staleness_hash = loom_wiki_staleness_hash(&blocks);
        let source_records = json!(blocks
            .iter()
            .map(|b| json!({
                "record_family": "LoomBlock",
                "record_id": b.block_id,
                "content_hash": loom_block_content_hash(b),
            }))
            .collect::<Vec<_>>());

        // MT-242 ship-together upgrade: the MT-184 compile path stamps too —
        // EventLedger source version + the exact cited LoomBlock set with
        // content hashes (LM-PWIKI-006). No unstamped compile output exists.
        let ledger_version = self.current_event_ledger_version().await?;
        let stamp = WikiCompileStamp::new(
            ledger_version,
            blocks
                .iter()
                .map(|b| CitedSource {
                    kind: CitedSourceKind::LoomBlock,
                    id: b.block_id.clone(),
                    content_hash: loom_block_content_hash(b),
                    span_id: None,
                    source_id: None,
                })
                .collect(),
        );

        let projection = self
            .upsert_knowledge_wiki_page(NewKnowledgeWikiPage {
                workspace_id: workspace_id.to_string(),
                title: title.to_string(),
                page_type: None,
                source_records,
                rendered_content,
                staleness_hash,
                compile_stamp: stamp.to_value(),
                compile_recipe: Some(json!({
                    "kind": "loom_topic",
                    "block_ids": block_ids,
                })),
                page_links: json!([]),
                rebuild_receipt_event_id: None,
            })
            .await?;
        Ok(loom_wiki_projection_from_knowledge(projection))
    }

    async fn get_loom_wiki_projection(
        &self,
        workspace_id: &str,
        projection_id: &str,
    ) -> StorageResult<super::LoomWikiProjection> {
        use crate::storage::knowledge::KnowledgeStore;
        let projection = self
            .get_knowledge_wiki_projection(projection_id)
            .await?
            .filter(|p| p.workspace_id == workspace_id)
            .ok_or(StorageError::NotFound("loom_wiki_projection"))?;
        Ok(loom_wiki_projection_from_knowledge(projection))
    }

    async fn loom_wiki_projection_is_stale(
        &self,
        workspace_id: &str,
        projection_id: &str,
    ) -> StorageResult<bool> {
        let projection = self
            .get_loom_wiki_projection(workspace_id, projection_id)
            .await?;
        // Typed project-wiki pages (MT-241) cite entities/sources, not Loom
        // blocks — their staleness is the MT-242 drift verdict
        // (`knowledge_wiki::drift::WikiDriftChecker`), never this block-hash
        // path. Fail closed instead of mis-reporting.
        if projection.page_type.is_some() {
            return Err(StorageError::Validation(
                "typed project-wiki pages take staleness from the drift verdict",
            ));
        }
        // Recompute the hash over the CURRENT source blocks; a mismatch (or a
        // since-deleted source block) means the projection is stale.
        let mut blocks = Vec::with_capacity(projection.source_block_ids.len());
        for id in &projection.source_block_ids {
            match self.get_loom_block(workspace_id, id).await {
                Ok(block) => blocks.push(block),
                Err(StorageError::NotFound(_)) => return Ok(true),
                Err(err) => return Err(err),
            }
        }
        Ok(loom_wiki_staleness_hash(&blocks) != projection.staleness_hash)
    }

    async fn regenerate_loom_wiki_projection(
        &self,
        workspace_id: &str,
        projection_id: &str,
    ) -> StorageResult<super::LoomWikiProjection> {
        let current = self
            .get_loom_wiki_projection(workspace_id, projection_id)
            .await?;
        // Typed project-wiki pages regenerate through their compile recipe
        // (knowledge_wiki fan-out / bootstrap), which re-stamps citations from
        // code-index/knowledge authority. Re-rendering them as a Loom topic
        // page would silently destroy the compiled content — fail closed.
        if current.page_type.is_some() {
            return Err(StorageError::Validation(
                "typed project-wiki pages regenerate via the project wiki engine",
            ));
        }
        // Re-render from current authority (skip source blocks that vanished).
        let mut blocks = Vec::new();
        for id in &current.source_block_ids {
            match self.get_loom_block(workspace_id, id).await {
                Ok(block) => blocks.push(block),
                Err(StorageError::NotFound(_)) => {}
                Err(err) => return Err(err),
            }
        }
        // Re-compile (same identity upsert) so the refreshed page carries a
        // CURRENT stamp — regeneration without restamping would be a stale
        // verdict lying fresh.
        let block_ids: Vec<String> = blocks.iter().map(|b| b.block_id.clone()).collect();
        self.compile_loom_wiki_projection(workspace_id, &current.title, &block_ids)
            .await
    }

    async fn delete_loom_wiki_projection(
        &self,
        workspace_id: &str,
        projection_id: &str,
    ) -> StorageResult<()> {
        use crate::storage::knowledge::KnowledgeStore;
        // Confirm it exists in this workspace (clean 404 + workspace scoping).
        self.get_loom_wiki_projection(workspace_id, projection_id)
            .await?;
        // Deleting a projection mutates NO authority record (the
        // projection-never-authority law). Operator overlays are AUTHORITY
        // rows and SURVIVE this delete (soft reference since migration 0301;
        // the 0295 CASCADE was a law violation - it destroyed authority on
        // projection churn).
        self.delete_knowledge_wiki_projection(projection_id).await
    }

    // -- MT-185 WikiPageEditableOverlay ----------------------------------------

    async fn add_loom_wiki_overlay(
        &self,
        workspace_id: &str,
        projection_id: &str,
        annotation: &str,
        anchor: Option<&str>,
    ) -> StorageResult<super::LoomWikiOverlay> {
        let annotation = annotation.trim();
        if annotation.is_empty() {
            return Err(StorageError::Validation(
                "loom wiki overlay annotation is required",
            ));
        }
        // The projection must exist in this workspace.
        self.get_loom_wiki_projection(workspace_id, projection_id)
            .await?;
        let overlay_id = format!("LWO-{}", Uuid::now_v7().simple());
        let row = sqlx::query(
            r#"
            INSERT INTO loom_wiki_overlays
                (overlay_id, projection_id, workspace_id, annotation, anchor)
            VALUES ($1, $2, $3, $4, $5)
            RETURNING overlay_id, projection_id, workspace_id, annotation, anchor,
                      created_at, updated_at
            "#,
        )
        .bind(&overlay_id)
        .bind(projection_id)
        .bind(workspace_id)
        .bind(annotation)
        .bind(anchor.map(str::trim).filter(|s| !s.is_empty()))
        .fetch_one(&self.pool)
        .await?;
        Ok(map_loom_wiki_overlay(&row))
    }

    async fn list_loom_wiki_overlays(
        &self,
        workspace_id: &str,
        projection_id: &str,
    ) -> StorageResult<Vec<super::LoomWikiOverlay>> {
        let rows = sqlx::query(
            r#"
            SELECT overlay_id, projection_id, workspace_id, annotation, anchor,
                   created_at, updated_at
            FROM loom_wiki_overlays
            WHERE workspace_id = $1 AND projection_id = $2
            ORDER BY created_at ASC, overlay_id ASC
            "#,
        )
        .bind(workspace_id)
        .bind(projection_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.iter().map(map_loom_wiki_overlay).collect())
    }

    async fn delete_loom_wiki_overlay(
        &self,
        workspace_id: &str,
        overlay_id: &str,
    ) -> StorageResult<()> {
        let res = sqlx::query(
            "DELETE FROM loom_wiki_overlays WHERE workspace_id = $1 AND overlay_id = $2",
        )
        .bind(workspace_id)
        .bind(overlay_id)
        .execute(&self.pool)
        .await?;
        if res.rows_affected() == 0 {
            return Err(StorageError::NotFound("loom_wiki_overlay"));
        }
        Ok(())
    }

    // -- MT-187 ObsidianImportBoundary -----------------------------------------

    async fn import_markdown_to_loom(
        &self,
        ctx: &WriteContext,
        workspace_id: &str,
        title: &str,
        markdown: &str,
    ) -> StorageResult<super::LoomMarkdownImport> {
        use crate::knowledge_document::block_tree::DOCUMENT_SCHEMA_VERSION;
        use crate::knowledge_document::import::{import_snippet, ImportFormat};
        use crate::storage::knowledge::{KnowledgeStore, NewKnowledgeRichDocument};

        let title = title.trim();
        if title.is_empty() {
            return Err(StorageError::Validation("loom import title is required"));
        }

        // Parse the markdown SOURCE into a ProseMirror block tree. The source
        // text itself is NEVER stored as authority — only the parsed authority
        // document below (MT-187: a vault/markdown layout cannot be the truth).
        let outcome = import_snippet(markdown, ImportFormat::Markdown);

        // Create the authority RichDocument from the parsed tree.
        let document = self
            .create_knowledge_rich_document(NewKnowledgeRichDocument {
                workspace_id: workspace_id.to_string(),
                document_id: None,
                title: title.to_string(),
                schema_version: DOCUMENT_SCHEMA_VERSION.to_string(),
                content_json: outcome.document_json,
                crdt_document_id: None,
                crdt_snapshot_id: None,
                promotion_receipt_event_id: None,
                project_ref: None,
                folder_ref: None,
                authority_label: None,
                owner_actor_kind: None,
                owner_actor_id: None,
            })
            .await?;

        // Create the LoomBlock (a note) backed by that authority document.
        // NOTE: loom_blocks.document_id FKs the LEGACY `documents` table, not
        // knowledge_rich_documents, so the RichDocument link is logical (carried
        // in the returned LoomMarkdownImport.rich_document_id), not via that
        // column. Leave document_id NULL to avoid a cross-table FK violation.
        let block = self
            .create_loom_block(
                ctx,
                NewLoomBlock {
                    block_id: None,
                    workspace_id: workspace_id.to_string(),
                    content_type: LoomBlockContentType::Note,
                    document_id: None,
                    asset_id: None,
                    title: Some(title.to_string()),
                    original_filename: None,
                    content_hash: None,
                    pinned: false,
                    journal_date: None,
                    imported_at: Some(Utc::now()),
                    derived: LoomBlockDerived::default(),
                },
            )
            .await?;

        // Bridge the new block to the ProjectKnowledgeIndex + EventLedger (so
        // the imported note is authority-resolved, never a vault-only row).
        self.bridge_loom_block_to_knowledge(ctx, workspace_id, &block.block_id)
            .await?;

        let warnings = outcome
            .warnings
            .into_iter()
            .map(|w| format!("{}: {}", w.code, w.detail))
            .collect();

        Ok(super::LoomMarkdownImport {
            block,
            rich_document_id: document.rich_document_id,
            warnings,
        })
    }

    // -- MT-188 NavigationBreadcrumbs ------------------------------------------

    async fn loom_block_breadcrumbs(
        &self,
        workspace_id: &str,
        block_id: &str,
    ) -> StorageResult<super::LoomBreadcrumbTrail> {
        // Block must exist (fail-closed) and gives the leaf label.
        let block = self.get_loom_block(workspace_id, block_id).await?;

        let mut crumbs: Vec<super::LoomBreadcrumb> = Vec::new();

        // 1. Workspace root.
        let workspace_name = self
            .get_workspace(workspace_id)
            .await?
            .map(|w| w.name)
            .unwrap_or_else(|| workspace_id.to_string());
        crumbs.push(super::LoomBreadcrumb {
            kind: "workspace".to_string(),
            id: workspace_id.to_string(),
            label: workspace_name,
        });

        // 2. Folder ancestry: pick ONE folder this block belongs to (the first
        //    by deterministic order), then walk up to the root, emitting crumbs
        //    root-first.
        let first_folder: Option<(String, String, Option<String>)> = sqlx::query_as(
            r#"
            SELECT f.folder_id, f.name, f.project_ref
            FROM loom_folder_members m
            JOIN loom_folders f
              ON f.folder_id = m.folder_id AND f.workspace_id = m.workspace_id
            WHERE m.workspace_id = $1 AND m.block_id = $2
            ORDER BY f.created_at ASC, f.folder_id ASC
            LIMIT 1
            "#,
        )
        .bind(workspace_id)
        .bind(block_id)
        .fetch_optional(&self.pool)
        .await?;

        if let Some((folder_id, _name, _project_ref)) = first_folder {
            // Walk the folder ancestry up to the root (parent_folder_id chain),
            // cycle-guarded. depth 0 = the block's own folder; higher depth =
            // closer to the root. Emit root-first (highest depth first).
            let ancestry: Vec<(String, String, Option<String>, i32)> = sqlx::query_as(
                r#"
                WITH RECURSIVE up(folder_id, name, parent_folder_id, project_ref, depth, path) AS (
                    SELECT folder_id, name, parent_folder_id, project_ref, 0,
                           ARRAY[folder_id]::TEXT[]
                    FROM loom_folders
                    WHERE workspace_id = $1 AND folder_id = $2
                    UNION ALL
                    SELECT f.folder_id, f.name, f.parent_folder_id, f.project_ref,
                           up.depth + 1, up.path || f.folder_id
                    FROM loom_folders f
                    JOIN up ON f.folder_id = up.parent_folder_id
                    WHERE f.workspace_id = $1
                      AND NOT f.folder_id = ANY(up.path)
                )
                SELECT folder_id, name, project_ref, depth FROM up
                ORDER BY depth DESC
                "#,
            )
            .bind(workspace_id)
            .bind(&folder_id)
            .fetch_all(&self.pool)
            .await?;

            // Optional project crumb: the root-most folder in the ancestry that
            // carries a project_ref (the project the branch belongs to).
            if let Some(project) = ancestry
                .iter()
                .find_map(|(_, _, pr, _)| pr.as_deref().map(str::trim).filter(|v| !v.is_empty()))
            {
                crumbs.push(super::LoomBreadcrumb {
                    kind: "project".to_string(),
                    id: project.to_string(),
                    label: project.to_string(),
                });
            }

            for (fid, name, _pr, _depth) in ancestry {
                crumbs.push(super::LoomBreadcrumb {
                    kind: "folder".to_string(),
                    id: fid,
                    label: name,
                });
            }
        }

        // 3. The block itself.
        let block_label = block
            .title
            .as_deref()
            .map(str::trim)
            .filter(|v| !v.is_empty())
            .map(|v| v.to_string())
            .unwrap_or_else(|| format!("{} {}", block.content_type.as_str(), block.block_id));
        crumbs.push(super::LoomBreadcrumb {
            kind: "block".to_string(),
            id: block.block_id.clone(),
            label: block_label,
        });

        // 4. The ProjectKnowledgeIndex entity (when bridged) — the authority
        //    anchor at the tip of the spine.
        if let Some(bridge) = self
            .get_loom_block_knowledge_bridge(workspace_id, block_id)
            .await?
        {
            crumbs.push(super::LoomBreadcrumb {
                kind: "entity".to_string(),
                id: bridge.entity_id,
                label: "knowledge_entity".to_string(),
            });
        }

        Ok(super::LoomBreadcrumbTrail {
            block_id: block.block_id,
            crumbs,
        })
    }

    async fn upsert_calendar_source(
        &self,
        ctx: &WriteContext,
        source: CalendarSourceUpsert,
    ) -> StorageResult<CalendarSource> {
        validate_calendar_source_upsert(&source)?;

        let now = Utc::now();
        let metadata = self.guard.validate_write(ctx, &source.id).await?;
        let actor_kind = metadata.actor_kind.as_str();
        let actor_id = metadata.actor_id.clone();
        let job_id = metadata.job_id.map(|id| id.to_string());
        let workflow_id = metadata.workflow_id.map(|id| id.to_string());
        let edit_event_id = metadata.edit_event_id.to_string();
        let config_json = encode_json(&source.config)?;
        let sync_state = source.sync_state.state.as_ref().map(|value| value.as_str());

        let row = sqlx::query(
            r#"
            INSERT INTO calendar_sources (
                id,
                workspace_id,
                display_name,
                provider_type,
                write_policy,
                default_tzid,
                auto_export,
                credentials_ref,
                provider_calendar_id,
                capability_profile_id,
                config_json,
                sync_state,
                sync_token,
                last_sync_ts,
                last_full_sync_ts,
                last_ok_at,
                last_pull_at,
                last_push_at,
                last_error_at,
                last_error_code,
                last_error,
                backoff_until,
                consecutive_failures,
                last_remote_watermark,
                last_local_applied_rev,
                last_actor_kind,
                last_actor_id,
                last_job_id,
                last_workflow_id,
                edit_event_id,
                created_at,
                updated_at
            )
            VALUES (
                $1, $2, $3, $4, $5, $6, $7, $8, $9, $10,
                $11, $12, $13, $14, $15, $16, $17, $18, $19, $20,
                $21, $22, $23, $24, $25, $26, $27, $28, $29, $30,
                $31, $32
            )
            ON CONFLICT (id) DO UPDATE SET
                workspace_id = excluded.workspace_id,
                display_name = excluded.display_name,
                provider_type = excluded.provider_type,
                write_policy = excluded.write_policy,
                default_tzid = excluded.default_tzid,
                auto_export = excluded.auto_export,
                credentials_ref = excluded.credentials_ref,
                provider_calendar_id = excluded.provider_calendar_id,
                capability_profile_id = excluded.capability_profile_id,
                config_json = excluded.config_json,
                sync_state = excluded.sync_state,
                sync_token = excluded.sync_token,
                last_sync_ts = excluded.last_sync_ts,
                last_full_sync_ts = excluded.last_full_sync_ts,
                last_ok_at = excluded.last_ok_at,
                last_pull_at = excluded.last_pull_at,
                last_push_at = excluded.last_push_at,
                last_error_at = excluded.last_error_at,
                last_error_code = excluded.last_error_code,
                last_error = excluded.last_error,
                backoff_until = excluded.backoff_until,
                consecutive_failures = excluded.consecutive_failures,
                last_remote_watermark = excluded.last_remote_watermark,
                last_local_applied_rev = excluded.last_local_applied_rev,
                last_actor_kind = excluded.last_actor_kind,
                last_actor_id = excluded.last_actor_id,
                last_job_id = excluded.last_job_id,
                last_workflow_id = excluded.last_workflow_id,
                edit_event_id = excluded.edit_event_id,
                updated_at = excluded.updated_at
            RETURNING
                id,
                workspace_id,
                display_name,
                provider_type,
                write_policy,
                default_tzid,
                auto_export,
                credentials_ref,
                provider_calendar_id,
                capability_profile_id,
                config_json,
                sync_state,
                sync_token,
                last_sync_ts,
                last_full_sync_ts,
                last_ok_at,
                last_pull_at,
                last_push_at,
                last_error_at,
                last_error_code,
                last_error,
                backoff_until,
                consecutive_failures,
                last_remote_watermark,
                last_local_applied_rev,
                last_job_id,
                last_workflow_id,
                last_actor_id,
                edit_event_id,
                last_actor_kind,
                created_at,
                updated_at
            "#,
        )
        .bind(source.id)
        .bind(source.workspace_id)
        .bind(source.display_name)
        .bind(source.provider_type.as_str())
        .bind(source.write_policy.as_str())
        .bind(source.default_tzid)
        .bind(source.auto_export)
        .bind(source.credentials_ref)
        .bind(source.provider_calendar_id)
        .bind(source.capability_profile_id)
        .bind(config_json)
        .bind(sync_state)
        .bind(source.sync_state.sync_token)
        .bind(source.sync_state.last_synced_at)
        .bind(source.sync_state.last_full_sync_at)
        .bind(source.sync_state.last_ok_at)
        .bind(source.sync_state.last_pull_at)
        .bind(source.sync_state.last_push_at)
        .bind(source.sync_state.last_error_at)
        .bind(source.sync_state.last_error_code)
        .bind(source.sync_state.last_error)
        .bind(source.sync_state.backoff_until)
        .bind(source.sync_state.consecutive_failures)
        .bind(source.sync_state.last_remote_watermark)
        .bind(source.sync_state.last_local_applied_rev)
        .bind(actor_kind)
        .bind(actor_id)
        .bind(job_id)
        .bind(workflow_id)
        .bind(edit_event_id)
        .bind(now)
        .bind(now)
        .fetch_one(&self.pool)
        .await?;

        map_calendar_source(row)
    }

    async fn list_calendar_sources(
        &self,
        workspace_id: &str,
    ) -> StorageResult<Vec<CalendarSource>> {
        let rows = sqlx::query(
            r#"
            SELECT
                id,
                workspace_id,
                display_name,
                provider_type,
                write_policy,
                default_tzid,
                auto_export,
                credentials_ref,
                provider_calendar_id,
                capability_profile_id,
                config_json,
                sync_state,
                sync_token,
                last_sync_ts,
                last_full_sync_ts,
                last_ok_at,
                last_pull_at,
                last_push_at,
                last_error_at,
                last_error_code,
                last_error,
                backoff_until,
                consecutive_failures,
                last_remote_watermark,
                last_local_applied_rev,
                last_job_id,
                last_workflow_id,
                last_actor_id,
                edit_event_id,
                last_actor_kind,
                created_at,
                updated_at
            FROM calendar_sources
            WHERE workspace_id = $1
            ORDER BY display_name ASC, id ASC
            "#,
        )
        .bind(workspace_id)
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter()
            .map(map_calendar_source)
            .collect::<StorageResult<Vec<_>>>()
    }

    async fn get_calendar_source(
        &self,
        workspace_id: &str,
        source_id: &str,
    ) -> StorageResult<Option<CalendarSource>> {
        let row = sqlx::query(
            r#"
            SELECT
                id,
                workspace_id,
                display_name,
                provider_type,
                write_policy,
                default_tzid,
                auto_export,
                credentials_ref,
                provider_calendar_id,
                capability_profile_id,
                config_json,
                sync_state,
                sync_token,
                last_sync_ts,
                last_full_sync_ts,
                last_ok_at,
                last_pull_at,
                last_push_at,
                last_error_at,
                last_error_code,
                last_error,
                backoff_until,
                consecutive_failures,
                last_remote_watermark,
                last_local_applied_rev,
                last_job_id,
                last_workflow_id,
                last_actor_id,
                edit_event_id,
                last_actor_kind,
                created_at,
                updated_at
            FROM calendar_sources
            WHERE workspace_id = $1 AND id = $2
            "#,
        )
        .bind(workspace_id)
        .bind(source_id)
        .fetch_optional(&self.pool)
        .await?;

        row.map(map_calendar_source).transpose()
    }

    async fn upsert_calendar_event(
        &self,
        ctx: &WriteContext,
        event: CalendarEventUpsert,
    ) -> StorageResult<CalendarEvent> {
        validate_calendar_event_upsert(&event)?;

        let now = Utc::now();
        let metadata = self.guard.validate_write(ctx, &event.id).await?;
        let actor_kind = metadata.actor_kind.as_str();
        let actor_id = metadata.actor_id.clone();
        let job_id = metadata.job_id.map(|id| id.to_string());
        let workflow_id = metadata.workflow_id.map(|id| id.to_string());
        let edit_event_id = metadata.edit_event_id.to_string();
        let rdate_json = encode_string_vec(&event.rdate)?;
        let exdate_json = encode_string_vec(&event.exdate)?;
        let attendees_json = encode_json(&event.attendees)?;
        let links_json = encode_json(&event.links)?;
        let provider_payload_json = event
            .provider_payload
            .as_ref()
            .map(encode_json)
            .transpose()?;

        let row = if event.external_id.is_some() {
            sqlx::query(
                r#"
                INSERT INTO calendar_events (
                    id,
                    workspace_id,
                    source_id,
                    external_id,
                    external_etag,
                    title,
                    description,
                    location,
                    start_ts_utc,
                    end_ts_utc,
                    start_local,
                    end_local,
                    tzid,
                    all_day,
                    was_floating,
                    status,
                    visibility,
                    export_mode,
                    rrule,
                    rdate_json,
                    exdate_json,
                    is_recurring,
                    series_id,
                    instance_key,
                    is_override,
                    source_last_seen_at,
                    created_by,
                    attendees_json,
                    links_json,
                    provider_payload_json,
                    last_actor_kind,
                    last_actor_id,
                    last_job_id,
                    last_workflow_id,
                    edit_event_id,
                    created_at,
                    updated_at
                )
                VALUES (
                    $1, $2, $3, $4, $5, $6, $7, $8, $9, $10,
                    $11, $12, $13, $14, $15, $16, $17, $18, $19, $20,
                    $21, $22, $23, $24, $25, $26, $27, $28, $29, $30,
                    $31, $32, $33, $34, $35, $36, $37
                )
                ON CONFLICT (source_id, external_id) DO UPDATE SET
                    workspace_id = excluded.workspace_id,
                    external_etag = excluded.external_etag,
                    title = excluded.title,
                    description = excluded.description,
                    location = excluded.location,
                    start_ts_utc = excluded.start_ts_utc,
                    end_ts_utc = excluded.end_ts_utc,
                    start_local = excluded.start_local,
                    end_local = excluded.end_local,
                    tzid = excluded.tzid,
                    all_day = excluded.all_day,
                    was_floating = excluded.was_floating,
                    status = excluded.status,
                    visibility = excluded.visibility,
                    export_mode = excluded.export_mode,
                    rrule = excluded.rrule,
                    rdate_json = excluded.rdate_json,
                    exdate_json = excluded.exdate_json,
                    is_recurring = excluded.is_recurring,
                    series_id = excluded.series_id,
                    instance_key = excluded.instance_key,
                    is_override = excluded.is_override,
                    source_last_seen_at = excluded.source_last_seen_at,
                    attendees_json = excluded.attendees_json,
                    links_json = excluded.links_json,
                    provider_payload_json = excluded.provider_payload_json,
                    last_actor_kind = excluded.last_actor_kind,
                    last_actor_id = excluded.last_actor_id,
                    last_job_id = excluded.last_job_id,
                    last_workflow_id = excluded.last_workflow_id,
                    edit_event_id = excluded.edit_event_id,
                    updated_at = excluded.updated_at
                RETURNING
                    id,
                    workspace_id,
                    source_id,
                    external_id,
                    external_etag,
                    title,
                    description,
                    location,
                    start_ts_utc,
                    end_ts_utc,
                    start_local,
                    end_local,
                    tzid,
                    all_day,
                    was_floating,
                    status,
                    visibility,
                    export_mode,
                    rrule,
                    rdate_json,
                    exdate_json,
                    is_recurring,
                    series_id,
                    instance_key,
                    is_override,
                    source_last_seen_at,
                    created_by,
                    attendees_json,
                    links_json,
                    provider_payload_json,
                    last_job_id,
                    last_workflow_id,
                    last_actor_id,
                    edit_event_id,
                    last_actor_kind,
                    created_at,
                    updated_at
                "#,
            )
            .bind(event.id)
            .bind(event.workspace_id)
            .bind(event.source_id)
            .bind(event.external_id)
            .bind(event.external_etag)
            .bind(event.title)
            .bind(event.description)
            .bind(event.location)
            .bind(event.start_ts_utc)
            .bind(event.end_ts_utc)
            .bind(event.start_local)
            .bind(event.end_local)
            .bind(event.tzid)
            .bind(event.all_day)
            .bind(event.was_floating)
            .bind(event.status.as_str())
            .bind(event.visibility.as_str())
            .bind(event.export_mode.as_str())
            .bind(event.rrule)
            .bind(rdate_json)
            .bind(exdate_json)
            .bind(event.is_recurring)
            .bind(event.series_id)
            .bind(event.instance_key)
            .bind(event.is_override)
            .bind(event.source_last_seen_at)
            .bind(actor_id.clone())
            .bind(attendees_json)
            .bind(links_json)
            .bind(provider_payload_json)
            .bind(actor_kind)
            .bind(actor_id.clone())
            .bind(job_id.clone())
            .bind(workflow_id.clone())
            .bind(edit_event_id.clone())
            .bind(now)
            .bind(now)
            .fetch_one(&self.pool)
            .await?
        } else {
            sqlx::query(
                r#"
                INSERT INTO calendar_events (
                    id,
                    workspace_id,
                    source_id,
                    external_id,
                    external_etag,
                    title,
                    description,
                    location,
                    start_ts_utc,
                    end_ts_utc,
                    start_local,
                    end_local,
                    tzid,
                    all_day,
                    was_floating,
                    status,
                    visibility,
                    export_mode,
                    rrule,
                    rdate_json,
                    exdate_json,
                    is_recurring,
                    series_id,
                    instance_key,
                    is_override,
                    source_last_seen_at,
                    created_by,
                    attendees_json,
                    links_json,
                    provider_payload_json,
                    last_actor_kind,
                    last_actor_id,
                    last_job_id,
                    last_workflow_id,
                    edit_event_id,
                    created_at,
                    updated_at
                )
                VALUES (
                    $1, $2, $3, $4, $5, $6, $7, $8, $9, $10,
                    $11, $12, $13, $14, $15, $16, $17, $18, $19, $20,
                    $21, $22, $23, $24, $25, $26, $27, $28, $29, $30,
                    $31, $32, $33, $34, $35, $36, $37
                )
                ON CONFLICT (id) DO UPDATE SET
                    workspace_id = excluded.workspace_id,
                    source_id = excluded.source_id,
                    external_id = excluded.external_id,
                    external_etag = excluded.external_etag,
                    title = excluded.title,
                    description = excluded.description,
                    location = excluded.location,
                    start_ts_utc = excluded.start_ts_utc,
                    end_ts_utc = excluded.end_ts_utc,
                    start_local = excluded.start_local,
                    end_local = excluded.end_local,
                    tzid = excluded.tzid,
                    all_day = excluded.all_day,
                    was_floating = excluded.was_floating,
                    status = excluded.status,
                    visibility = excluded.visibility,
                    export_mode = excluded.export_mode,
                    rrule = excluded.rrule,
                    rdate_json = excluded.rdate_json,
                    exdate_json = excluded.exdate_json,
                    is_recurring = excluded.is_recurring,
                    series_id = excluded.series_id,
                    instance_key = excluded.instance_key,
                    is_override = excluded.is_override,
                    source_last_seen_at = excluded.source_last_seen_at,
                    attendees_json = excluded.attendees_json,
                    links_json = excluded.links_json,
                    provider_payload_json = excluded.provider_payload_json,
                    last_actor_kind = excluded.last_actor_kind,
                    last_actor_id = excluded.last_actor_id,
                    last_job_id = excluded.last_job_id,
                    last_workflow_id = excluded.last_workflow_id,
                    edit_event_id = excluded.edit_event_id,
                    updated_at = excluded.updated_at
                RETURNING
                    id,
                    workspace_id,
                    source_id,
                    external_id,
                    external_etag,
                    title,
                    description,
                    location,
                    start_ts_utc,
                    end_ts_utc,
                    start_local,
                    end_local,
                    tzid,
                    all_day,
                    was_floating,
                    status,
                    visibility,
                    export_mode,
                    rrule,
                    rdate_json,
                    exdate_json,
                    is_recurring,
                    series_id,
                    instance_key,
                    is_override,
                    source_last_seen_at,
                    created_by,
                    attendees_json,
                    links_json,
                    provider_payload_json,
                    last_job_id,
                    last_workflow_id,
                    last_actor_id,
                    edit_event_id,
                    last_actor_kind,
                    created_at,
                    updated_at
                "#,
            )
            .bind(event.id)
            .bind(event.workspace_id)
            .bind(event.source_id)
            .bind(event.external_id)
            .bind(event.external_etag)
            .bind(event.title)
            .bind(event.description)
            .bind(event.location)
            .bind(event.start_ts_utc)
            .bind(event.end_ts_utc)
            .bind(event.start_local)
            .bind(event.end_local)
            .bind(event.tzid)
            .bind(event.all_day)
            .bind(event.was_floating)
            .bind(event.status.as_str())
            .bind(event.visibility.as_str())
            .bind(event.export_mode.as_str())
            .bind(event.rrule)
            .bind(rdate_json)
            .bind(exdate_json)
            .bind(event.is_recurring)
            .bind(event.series_id)
            .bind(event.instance_key)
            .bind(event.is_override)
            .bind(event.source_last_seen_at)
            .bind(actor_id.clone())
            .bind(attendees_json)
            .bind(links_json)
            .bind(provider_payload_json)
            .bind(actor_kind)
            .bind(actor_id)
            .bind(job_id)
            .bind(workflow_id)
            .bind(edit_event_id)
            .bind(now)
            .bind(now)
            .fetch_one(&self.pool)
            .await?
        };

        map_calendar_event(row)
    }

    async fn query_calendar_events(
        &self,
        query: CalendarEventWindowQuery,
    ) -> StorageResult<Vec<CalendarEvent>> {
        validate_calendar_event_query(&query)?;

        let mut qb = sqlx::QueryBuilder::<sqlx::Postgres>::new(
            r#"
            SELECT
                id,
                workspace_id,
                source_id,
                external_id,
                external_etag,
                title,
                description,
                location,
                start_ts_utc,
                end_ts_utc,
                start_local,
                end_local,
                tzid,
                all_day,
                was_floating,
                status,
                visibility,
                export_mode,
                rrule,
                rdate_json,
                exdate_json,
                is_recurring,
                series_id,
                instance_key,
                is_override,
                source_last_seen_at,
                created_by,
                attendees_json,
                links_json,
                provider_payload_json,
                last_job_id,
                last_workflow_id,
                last_actor_id,
                edit_event_id,
                last_actor_kind,
                created_at,
                updated_at
            FROM calendar_events
            WHERE workspace_id = "#,
        );
        qb.push_bind(&query.workspace_id);
        qb.push(" AND start_ts_utc < ")
            .push_bind(query.window_end_utc);
        qb.push(" AND end_ts_utc > ")
            .push_bind(query.window_start_utc);

        if !query.source_ids.is_empty() {
            qb.push(" AND source_id IN (");
            let mut separated = qb.separated(", ");
            for source_id in &query.source_ids {
                separated.push_bind(source_id);
            }
            separated.push_unseparated(")");
        }

        qb.push(" ORDER BY start_ts_utc ASC, end_ts_utc ASC, id ASC");

        let rows = qb.build().fetch_all(&self.pool).await?;
        rows.into_iter()
            .map(map_calendar_event)
            .collect::<StorageResult<Vec<_>>>()
    }

    async fn delete_calendar_data_by_source(
        &self,
        ctx: &WriteContext,
        workspace_id: &str,
        source_id: &str,
    ) -> StorageResult<()> {
        self.guard.validate_write(ctx, source_id).await?;

        let res = sqlx::query(
            r#"
            DELETE FROM calendar_sources
            WHERE workspace_id = $1 AND id = $2
            "#,
        )
        .bind(workspace_id)
        .bind(source_id)
        .execute(&self.pool)
        .await?;

        if res.rows_affected() == 0 {
            return Err(StorageError::NotFound("calendar_source"));
        }

        Ok(())
    }

    async fn create_canvas(&self, ctx: &WriteContext, canvas: NewCanvas) -> StorageResult<Canvas> {
        let now = Utc::now();
        let id = Uuid::now_v7().to_string();
        let metadata = self.guard.validate_write(ctx, &id).await?;
        let actor_kind = metadata.actor_kind.as_str();
        let actor_id = metadata.actor_id.clone();
        let job_id = metadata.job_id.map(|v| v.to_string());
        let workflow_id = metadata.workflow_id.map(|v| v.to_string());
        let edit_event_id = metadata.edit_event_id.to_string();

        let row = sqlx::query(
            r#"
            INSERT INTO canvases (
                id,
                workspace_id,
                title,
                created_at,
                updated_at,
                last_actor_kind,
                last_actor_id,
                last_job_id,
                last_workflow_id,
                edit_event_id
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            RETURNING id, workspace_id, title, created_at, updated_at
            "#,
        )
        .bind(&id)
        .bind(&canvas.workspace_id)
        .bind(&canvas.title)
        .bind(now)
        .bind(now)
        .bind(actor_kind)
        .bind(&actor_id)
        .bind(job_id)
        .bind(workflow_id)
        .bind(edit_event_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(map_canvas(row))
    }

    async fn list_canvases(&self, workspace_id: &str) -> StorageResult<Vec<Canvas>> {
        let rows = sqlx::query(
            r#"
            SELECT id, workspace_id, title, created_at, updated_at
            FROM canvases
            WHERE workspace_id = $1
            ORDER BY created_at ASC
            "#,
        )
        .bind(workspace_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(map_canvas).collect())
    }

    async fn get_canvas_with_graph(&self, canvas_id: &str) -> StorageResult<CanvasGraph> {
        let canvas_row = sqlx::query(
            r#"
            SELECT id, workspace_id, title, created_at, updated_at
            FROM canvases
            WHERE id = $1
            "#,
        )
        .bind(canvas_id)
        .fetch_optional(&self.pool)
        .await?;

        let canvas_row = match canvas_row {
            Some(row) => row,
            None => return Err(StorageError::NotFound("canvas")),
        };

        let nodes = sqlx::query(
            r#"
            SELECT id, canvas_id, kind, position_x, position_y, data, created_at, updated_at
            FROM canvas_nodes
            WHERE canvas_id = $1
            ORDER BY created_at ASC
            "#,
        )
        .bind(canvas_id)
        .fetch_all(&self.pool)
        .await?;

        let edges = sqlx::query(
            r#"
            SELECT id, canvas_id, from_node_id, to_node_id, kind, created_at, updated_at
            FROM canvas_edges
            WHERE canvas_id = $1
            ORDER BY created_at ASC
            "#,
        )
        .bind(canvas_id)
        .fetch_all(&self.pool)
        .await?;

        let parsed_nodes = nodes
            .into_iter()
            .map(map_canvas_node)
            .collect::<StorageResult<Vec<_>>>()?;
        let parsed_edges = edges.into_iter().map(map_canvas_edge).collect::<Vec<_>>();

        Ok(CanvasGraph {
            canvas: map_canvas(canvas_row),
            nodes: parsed_nodes,
            edges: parsed_edges,
        })
    }

    async fn update_canvas_graph(
        &self,
        ctx: &WriteContext,
        canvas_id: &str,
        nodes: Vec<NewCanvasNode>,
        edges: Vec<NewCanvasEdge>,
    ) -> StorageResult<CanvasGraph> {
        let canvas_metadata = self.guard.validate_write(ctx, canvas_id).await?;
        let mut tx = self.pool.begin().await?;

        let canvas_row = sqlx::query(
            r#"SELECT id, workspace_id, title, created_at, updated_at FROM canvases WHERE id = $1"#,
        )
        .bind(canvas_id)
        .fetch_optional(&mut *tx)
        .await?;

        let canvas_row = match canvas_row {
            Some(row) => row,
            None => return Err(StorageError::NotFound("canvas")),
        };

        sqlx::query(r#"DELETE FROM canvas_edges WHERE canvas_id = $1"#)
            .bind(canvas_id)
            .execute(&mut *tx)
            .await?;
        sqlx::query(r#"DELETE FROM canvas_nodes WHERE canvas_id = $1"#)
            .bind(canvas_id)
            .execute(&mut *tx)
            .await?;

        let now = Utc::now();
        let mut inserted_nodes = Vec::with_capacity(nodes.len());
        for node in nodes {
            let id = node.id.unwrap_or_else(|| Uuid::now_v7().to_string());
            let metadata = self.guard.validate_write(ctx, &id).await?;
            let actor_kind = metadata.actor_kind.as_str();
            let actor_id = metadata.actor_id.clone();
            let job_id = metadata.job_id.map(|v| v.to_string());
            let workflow_id = metadata.workflow_id.map(|v| v.to_string());
            let edit_event_id = metadata.edit_event_id.to_string();
            let data = node
                .data
                .unwrap_or_else(|| Value::Object(Default::default()))
                .to_string();

            let row = sqlx::query(
                r#"
                INSERT INTO canvas_nodes (
                    id,
                    canvas_id,
                    kind,
                    position_x,
                    position_y,
                    data,
                    created_at,
                    updated_at,
                    last_actor_kind,
                    last_actor_id,
                    last_job_id,
                    last_workflow_id,
                    edit_event_id
                )
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)
                RETURNING id, canvas_id, kind, position_x, position_y, data, created_at, updated_at
                "#,
            )
            .bind(&id)
            .bind(canvas_id)
            .bind(&node.kind)
            .bind(node.position_x)
            .bind(node.position_y)
            .bind(&data)
            .bind(now)
            .bind(now)
            .bind(actor_kind)
            .bind(&actor_id)
            .bind(job_id)
            .bind(workflow_id)
            .bind(edit_event_id)
            .fetch_one(&mut *tx)
            .await?;

            inserted_nodes.push(map_canvas_node(row)?);
        }

        let mut inserted_edges = Vec::with_capacity(edges.len());
        for edge in edges {
            let id = edge.id.unwrap_or_else(|| Uuid::now_v7().to_string());
            let metadata = self.guard.validate_write(ctx, &id).await?;
            let actor_kind = metadata.actor_kind.as_str();
            let actor_id = metadata.actor_id.clone();
            let job_id = metadata.job_id.map(|v| v.to_string());
            let workflow_id = metadata.workflow_id.map(|v| v.to_string());
            let edit_event_id = metadata.edit_event_id.to_string();

            let row = sqlx::query(
                r#"
                INSERT INTO canvas_edges (
                    id,
                    canvas_id,
                    from_node_id,
                    to_node_id,
                    kind,
                    created_at,
                    updated_at,
                    last_actor_kind,
                    last_actor_id,
                    last_job_id,
                    last_workflow_id,
                    edit_event_id
                )
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
                RETURNING id, canvas_id, from_node_id, to_node_id, kind, created_at, updated_at
                "#,
            )
            .bind(&id)
            .bind(canvas_id)
            .bind(&edge.from_node_id)
            .bind(&edge.to_node_id)
            .bind(&edge.kind)
            .bind(now)
            .bind(now)
            .bind(actor_kind)
            .bind(&actor_id)
            .bind(job_id)
            .bind(workflow_id)
            .bind(edit_event_id)
            .fetch_one(&mut *tx)
            .await?;

            inserted_edges.push(map_canvas_edge(row));
        }

        let canvas_actor_kind = canvas_metadata.actor_kind.as_str();
        let canvas_actor_id = canvas_metadata.actor_id.clone();
        let canvas_job_id = canvas_metadata.job_id.map(|v| v.to_string());
        let canvas_workflow_id = canvas_metadata.workflow_id.map(|v| v.to_string());
        let canvas_edit_event_id = canvas_metadata.edit_event_id.to_string();
        let canvas_updated_at = canvas_metadata.timestamp;

        let updated = sqlx::query(
            r#"
            UPDATE canvases
            SET last_actor_kind = $1,
                last_actor_id = $2,
                last_job_id = $3,
                last_workflow_id = $4,
                edit_event_id = $5,
                updated_at = $6
            WHERE id = $7
            "#,
        )
        .bind(canvas_actor_kind)
        .bind(canvas_actor_id)
        .bind(canvas_job_id)
        .bind(canvas_workflow_id)
        .bind(canvas_edit_event_id)
        .bind(canvas_updated_at)
        .bind(canvas_id)
        .execute(&mut *tx)
        .await?;

        if updated.rows_affected() == 0 {
            return Err(StorageError::NotFound("canvas"));
        }

        tx.commit().await?;

        let mut canvas = map_canvas(canvas_row);
        canvas.updated_at = canvas_updated_at;

        Ok(CanvasGraph {
            canvas,
            nodes: inserted_nodes,
            edges: inserted_edges,
        })
    }

    async fn delete_canvas(&self, ctx: &WriteContext, canvas_id: &str) -> StorageResult<()> {
        self.guard.validate_write(ctx, canvas_id).await?;
        let res = sqlx::query(r#"DELETE FROM canvases WHERE id = $1"#)
            .bind(canvas_id)
            .execute(&self.pool)
            .await?;
        if res.rows_affected() == 0 {
            return Err(StorageError::NotFound("canvas"));
        }
        Ok(())
    }

    async fn create_ai_bronze_record(
        &self,
        ctx: &WriteContext,
        record: NewBronzeRecord,
    ) -> StorageResult<BronzeRecord> {
        let now = Utc::now();
        self.guard.validate_write(ctx, &record.bronze_id).await?;

        let row = sqlx::query(
            r#"
            INSERT INTO ai_bronze_records (
                bronze_id, workspace_id, content_hash, content_type, content_encoding, size_bytes,
                original_filename, artifact_path, ingested_at, ingestion_source_type, ingestion_source_id,
                ingestion_method, external_source_json, is_deleted, deleted_at, retention_policy
            )
            VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,0,NULL,$14)
            RETURNING
                bronze_id,
                workspace_id,
                content_hash,
                content_type,
                content_encoding,
                size_bytes,
                original_filename,
                artifact_path,
                ingested_at,
                ingestion_source_type,
                ingestion_source_id,
                ingestion_method,
                external_source_json,
                is_deleted,
                deleted_at,
                retention_policy
            "#,
        )
        .bind(record.bronze_id)
        .bind(record.workspace_id)
        .bind(record.content_hash)
        .bind(record.content_type)
        .bind(record.content_encoding)
        .bind(record.size_bytes as i64)
        .bind(record.original_filename)
        .bind(record.artifact_path)
        .bind(now.naive_utc())
        .bind(record.ingestion_source_type.as_str())
        .bind(record.ingestion_source_id)
        .bind(record.ingestion_method)
        .bind(record.external_source_json)
        .bind(record.retention_policy)
        .fetch_one(&self.pool)
        .await?;

        Ok(BronzeRecord {
            bronze_id: row.get("bronze_id"),
            workspace_id: row.get("workspace_id"),
            content_hash: row.get("content_hash"),
            content_type: row.get("content_type"),
            content_encoding: row.get("content_encoding"),
            size_bytes: row.get::<i64, _>("size_bytes") as u64,
            original_filename: row.get("original_filename"),
            artifact_path: row.get("artifact_path"),
            ingested_at: map_timestamp(&row, "ingested_at"),
            ingestion_source_type: crate::ai_ready_data::records::IngestionSourceType::from_str(
                row.get::<String, _>("ingestion_source_type").as_str(),
            )
            .map_err(|_| StorageError::Validation("invalid ingestion_source_type"))?,
            ingestion_source_id: row.get("ingestion_source_id"),
            ingestion_method: row.get("ingestion_method"),
            external_source_json: row.get("external_source_json"),
            is_deleted: map_i64_from_i32(&row, "is_deleted") != 0,
            deleted_at: map_optional_timestamp(&row, "deleted_at"),
            retention_policy: row.get("retention_policy"),
        })
    }

    async fn get_ai_bronze_record(&self, bronze_id: &str) -> StorageResult<Option<BronzeRecord>> {
        let row = sqlx::query(
            r#"
            SELECT
                bronze_id,
                workspace_id,
                content_hash,
                content_type,
                content_encoding,
                size_bytes,
                original_filename,
                artifact_path,
                ingested_at,
                ingestion_source_type,
                ingestion_source_id,
                ingestion_method,
                external_source_json,
                is_deleted,
                deleted_at,
                retention_policy
            FROM ai_bronze_records
            WHERE bronze_id = $1
            "#,
        )
        .bind(bronze_id)
        .fetch_optional(&self.pool)
        .await?;

        let Some(row) = row else {
            return Ok(None);
        };

        Ok(Some(BronzeRecord {
            bronze_id: row.get("bronze_id"),
            workspace_id: row.get("workspace_id"),
            content_hash: row.get("content_hash"),
            content_type: row.get("content_type"),
            content_encoding: row.get("content_encoding"),
            size_bytes: row.get::<i64, _>("size_bytes") as u64,
            original_filename: row.get("original_filename"),
            artifact_path: row.get("artifact_path"),
            ingested_at: map_timestamp(&row, "ingested_at"),
            ingestion_source_type: crate::ai_ready_data::records::IngestionSourceType::from_str(
                row.get::<String, _>("ingestion_source_type").as_str(),
            )
            .map_err(|_| StorageError::Validation("invalid ingestion_source_type"))?,
            ingestion_source_id: row.get("ingestion_source_id"),
            ingestion_method: row.get("ingestion_method"),
            external_source_json: row.get("external_source_json"),
            is_deleted: map_i64_from_i32(&row, "is_deleted") != 0,
            deleted_at: map_optional_timestamp(&row, "deleted_at"),
            retention_policy: row.get("retention_policy"),
        }))
    }

    async fn list_ai_bronze_records(&self, workspace_id: &str) -> StorageResult<Vec<BronzeRecord>> {
        let rows = sqlx::query(
            r#"
            SELECT
                bronze_id,
                workspace_id,
                content_hash,
                content_type,
                content_encoding,
                size_bytes,
                original_filename,
                artifact_path,
                ingested_at,
                ingestion_source_type,
                ingestion_source_id,
                ingestion_method,
                external_source_json,
                is_deleted,
                deleted_at,
                retention_policy
            FROM ai_bronze_records
            WHERE workspace_id = $1
            ORDER BY ingested_at ASC
            "#,
        )
        .bind(workspace_id)
        .fetch_all(&self.pool)
        .await?;

        let mut out = Vec::with_capacity(rows.len());
        for row in rows {
            out.push(BronzeRecord {
                bronze_id: row.get("bronze_id"),
                workspace_id: row.get("workspace_id"),
                content_hash: row.get("content_hash"),
                content_type: row.get("content_type"),
                content_encoding: row.get("content_encoding"),
                size_bytes: row.get::<i64, _>("size_bytes") as u64,
                original_filename: row.get("original_filename"),
                artifact_path: row.get("artifact_path"),
                ingested_at: map_timestamp(&row, "ingested_at"),
                ingestion_source_type:
                    crate::ai_ready_data::records::IngestionSourceType::from_str(
                        row.get::<String, _>("ingestion_source_type").as_str(),
                    )
                    .map_err(|_| StorageError::Validation("invalid ingestion_source_type"))?,
                ingestion_source_id: row.get("ingestion_source_id"),
                ingestion_method: row.get("ingestion_method"),
                external_source_json: row.get("external_source_json"),
                is_deleted: map_i64_from_i32(&row, "is_deleted") != 0,
                deleted_at: map_optional_timestamp(&row, "deleted_at"),
                retention_policy: row.get("retention_policy"),
            });
        }

        Ok(out)
    }

    async fn mark_ai_bronze_deleted(
        &self,
        ctx: &WriteContext,
        bronze_id: &str,
    ) -> StorageResult<()> {
        self.guard.validate_write(ctx, bronze_id).await?;
        let now = Utc::now();
        let res = sqlx::query(
            r#"
            UPDATE ai_bronze_records
            SET is_deleted = 1, deleted_at = $2
            WHERE bronze_id = $1
            "#,
        )
        .bind(bronze_id)
        .bind(now.naive_utc())
        .execute(&self.pool)
        .await?;
        if res.rows_affected() == 0 {
            return Err(StorageError::NotFound("ai_bronze_record"));
        }
        Ok(())
    }

    async fn create_ai_silver_record(
        &self,
        ctx: &WriteContext,
        record: NewSilverRecord,
    ) -> StorageResult<SilverRecord> {
        self.guard.validate_write(ctx, &record.silver_id).await?;
        let now = Utc::now();

        let row = sqlx::query(
            r#"
            INSERT INTO ai_silver_records (
                silver_id, workspace_id, bronze_ref, chunk_index, total_chunks, token_count,
                content_hash, byte_start, byte_end, line_start, line_end,
                chunk_artifact_path, embedding_artifact_path, embedding_model_id, embedding_model_version,
                embedding_dimensions, embedding_compute_latency_ms,
                chunking_strategy, chunking_version, processing_pipeline_version,
                processed_at, processing_duration_ms, metadata_json,
                validation_status, validation_failed_checks_json, validated_at, validator_version,
                is_current, superseded_by, created_at
            )
            VALUES (
                $1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,
                $12,$13,$14,$15,$16,$17,
                $18,$19,$20,
                $21,$22,$23,
                $24,$25,$26,$27,
                1,NULL,$28
            )
            RETURNING
                silver_id,
                workspace_id,
                bronze_ref,
                chunk_index,
                total_chunks,
                token_count,
                content_hash,
                byte_start,
                byte_end,
                line_start,
                line_end,
                chunk_artifact_path,
                embedding_artifact_path,
                embedding_model_id,
                embedding_model_version,
                embedding_dimensions,
                embedding_compute_latency_ms,
                chunking_strategy,
                chunking_version,
                processing_pipeline_version,
                processed_at,
                processing_duration_ms,
                metadata_json,
                validation_status,
                validation_failed_checks_json,
                validated_at,
                validator_version,
                is_current,
                superseded_by,
                created_at
            "#,
        )
        .bind(record.silver_id)
        .bind(record.workspace_id)
        .bind(record.bronze_ref)
        .bind(record.chunk_index as i32)
        .bind(record.total_chunks as i32)
        .bind(record.token_count as i32)
        .bind(record.content_hash)
        .bind(record.byte_start as i64)
        .bind(record.byte_end as i64)
        .bind(record.line_start as i32)
        .bind(record.line_end as i32)
        .bind(record.chunk_artifact_path)
        .bind(record.embedding_artifact_path)
        .bind(record.embedding_model_id)
        .bind(record.embedding_model_version)
        .bind(record.embedding_dimensions as i32)
        .bind(record.embedding_compute_latency_ms as i64)
        .bind(record.chunking_strategy)
        .bind(record.chunking_version)
        .bind(record.processing_pipeline_version)
        .bind(now.naive_utc())
        .bind(record.processing_duration_ms as i64)
        .bind(record.metadata_json)
        .bind(record.validation_status.as_str())
        .bind(record.validation_failed_checks_json)
        .bind(now.naive_utc())
        .bind(record.validator_version)
        .bind(now.naive_utc())
        .fetch_one(&self.pool)
        .await?;

        Ok(SilverRecord {
            silver_id: row.get("silver_id"),
            workspace_id: row.get("workspace_id"),
            bronze_ref: row.get("bronze_ref"),
            chunk_index: map_i64_from_i32(&row, "chunk_index") as u32,
            total_chunks: map_i64_from_i32(&row, "total_chunks") as u32,
            token_count: map_i64_from_i32(&row, "token_count") as u32,
            content_hash: row.get("content_hash"),
            byte_start: row.get::<i64, _>("byte_start") as u64,
            byte_end: row.get::<i64, _>("byte_end") as u64,
            line_start: map_i64_from_i32(&row, "line_start") as u32,
            line_end: map_i64_from_i32(&row, "line_end") as u32,
            chunk_artifact_path: row.get("chunk_artifact_path"),
            embedding_artifact_path: row.get("embedding_artifact_path"),
            embedding_model_id: row.get("embedding_model_id"),
            embedding_model_version: row.get("embedding_model_version"),
            embedding_dimensions: map_i64_from_i32(&row, "embedding_dimensions") as u32,
            embedding_compute_latency_ms: row.get::<i64, _>("embedding_compute_latency_ms") as u64,
            chunking_strategy: row.get("chunking_strategy"),
            chunking_version: row.get("chunking_version"),
            processing_pipeline_version: row.get("processing_pipeline_version"),
            processed_at: map_timestamp(&row, "processed_at"),
            processing_duration_ms: row.get::<i64, _>("processing_duration_ms") as u64,
            metadata_json: row.get("metadata_json"),
            validation_status: crate::ai_ready_data::records::ValidationStatus::from_str(
                row.get::<String, _>("validation_status").as_str(),
            )
            .map_err(|_| StorageError::Validation("invalid validation_status"))?,
            validation_failed_checks_json: row.get("validation_failed_checks_json"),
            validated_at: map_timestamp(&row, "validated_at"),
            validator_version: row.get("validator_version"),
            is_current: map_i64_from_i32(&row, "is_current") != 0,
            superseded_by: row.get("superseded_by"),
            created_at: map_timestamp(&row, "created_at"),
        })
    }

    async fn get_ai_silver_record(&self, silver_id: &str) -> StorageResult<Option<SilverRecord>> {
        let row = sqlx::query(
            r#"
            SELECT
                silver_id,
                workspace_id,
                bronze_ref,
                chunk_index,
                total_chunks,
                token_count,
                content_hash,
                byte_start,
                byte_end,
                line_start,
                line_end,
                chunk_artifact_path,
                embedding_artifact_path,
                embedding_model_id,
                embedding_model_version,
                embedding_dimensions,
                embedding_compute_latency_ms,
                chunking_strategy,
                chunking_version,
                processing_pipeline_version,
                processed_at,
                processing_duration_ms,
                metadata_json,
                validation_status,
                validation_failed_checks_json,
                validated_at,
                validator_version,
                is_current,
                superseded_by,
                created_at
            FROM ai_silver_records
            WHERE silver_id = $1
            "#,
        )
        .bind(silver_id)
        .fetch_optional(&self.pool)
        .await?;

        let Some(row) = row else {
            return Ok(None);
        };

        Ok(Some(SilverRecord {
            silver_id: row.get("silver_id"),
            workspace_id: row.get("workspace_id"),
            bronze_ref: row.get("bronze_ref"),
            chunk_index: map_i64_from_i32(&row, "chunk_index") as u32,
            total_chunks: map_i64_from_i32(&row, "total_chunks") as u32,
            token_count: map_i64_from_i32(&row, "token_count") as u32,
            content_hash: row.get("content_hash"),
            byte_start: row.get::<i64, _>("byte_start") as u64,
            byte_end: row.get::<i64, _>("byte_end") as u64,
            line_start: map_i64_from_i32(&row, "line_start") as u32,
            line_end: map_i64_from_i32(&row, "line_end") as u32,
            chunk_artifact_path: row.get("chunk_artifact_path"),
            embedding_artifact_path: row.get("embedding_artifact_path"),
            embedding_model_id: row.get("embedding_model_id"),
            embedding_model_version: row.get("embedding_model_version"),
            embedding_dimensions: map_i64_from_i32(&row, "embedding_dimensions") as u32,
            embedding_compute_latency_ms: row.get::<i64, _>("embedding_compute_latency_ms") as u64,
            chunking_strategy: row.get("chunking_strategy"),
            chunking_version: row.get("chunking_version"),
            processing_pipeline_version: row.get("processing_pipeline_version"),
            processed_at: map_timestamp(&row, "processed_at"),
            processing_duration_ms: row.get::<i64, _>("processing_duration_ms") as u64,
            metadata_json: row.get("metadata_json"),
            validation_status: crate::ai_ready_data::records::ValidationStatus::from_str(
                row.get::<String, _>("validation_status").as_str(),
            )
            .map_err(|_| StorageError::Validation("invalid validation_status"))?,
            validation_failed_checks_json: row.get("validation_failed_checks_json"),
            validated_at: map_timestamp(&row, "validated_at"),
            validator_version: row.get("validator_version"),
            is_current: map_i64_from_i32(&row, "is_current") != 0,
            superseded_by: row.get("superseded_by"),
            created_at: map_timestamp(&row, "created_at"),
        }))
    }

    async fn list_ai_silver_records_by_bronze(
        &self,
        bronze_id: &str,
    ) -> StorageResult<Vec<SilverRecord>> {
        let rows = sqlx::query(
            r#"
            SELECT
                silver_id,
                workspace_id,
                bronze_ref,
                chunk_index,
                total_chunks,
                token_count,
                content_hash,
                byte_start,
                byte_end,
                line_start,
                line_end,
                chunk_artifact_path,
                embedding_artifact_path,
                embedding_model_id,
                embedding_model_version,
                embedding_dimensions,
                embedding_compute_latency_ms,
                chunking_strategy,
                chunking_version,
                processing_pipeline_version,
                processed_at,
                processing_duration_ms,
                metadata_json,
                validation_status,
                validation_failed_checks_json,
                validated_at,
                validator_version,
                is_current,
                superseded_by,
                created_at
            FROM ai_silver_records
            WHERE bronze_ref = $1
            ORDER BY chunk_index ASC
            "#,
        )
        .bind(bronze_id)
        .fetch_all(&self.pool)
        .await?;

        let mut out = Vec::with_capacity(rows.len());
        for row in rows {
            out.push(SilverRecord {
                silver_id: row.get("silver_id"),
                workspace_id: row.get("workspace_id"),
                bronze_ref: row.get("bronze_ref"),
                chunk_index: map_i64_from_i32(&row, "chunk_index") as u32,
                total_chunks: map_i64_from_i32(&row, "total_chunks") as u32,
                token_count: map_i64_from_i32(&row, "token_count") as u32,
                content_hash: row.get("content_hash"),
                byte_start: row.get::<i64, _>("byte_start") as u64,
                byte_end: row.get::<i64, _>("byte_end") as u64,
                line_start: map_i64_from_i32(&row, "line_start") as u32,
                line_end: map_i64_from_i32(&row, "line_end") as u32,
                chunk_artifact_path: row.get("chunk_artifact_path"),
                embedding_artifact_path: row.get("embedding_artifact_path"),
                embedding_model_id: row.get("embedding_model_id"),
                embedding_model_version: row.get("embedding_model_version"),
                embedding_dimensions: map_i64_from_i32(&row, "embedding_dimensions") as u32,
                embedding_compute_latency_ms: row.get::<i64, _>("embedding_compute_latency_ms")
                    as u64,
                chunking_strategy: row.get("chunking_strategy"),
                chunking_version: row.get("chunking_version"),
                processing_pipeline_version: row.get("processing_pipeline_version"),
                processed_at: map_timestamp(&row, "processed_at"),
                processing_duration_ms: row.get::<i64, _>("processing_duration_ms") as u64,
                metadata_json: row.get("metadata_json"),
                validation_status: crate::ai_ready_data::records::ValidationStatus::from_str(
                    row.get::<String, _>("validation_status").as_str(),
                )
                .map_err(|_| StorageError::Validation("invalid validation_status"))?,
                validation_failed_checks_json: row.get("validation_failed_checks_json"),
                validated_at: map_timestamp(&row, "validated_at"),
                validator_version: row.get("validator_version"),
                is_current: map_i64_from_i32(&row, "is_current") != 0,
                superseded_by: row.get("superseded_by"),
                created_at: map_timestamp(&row, "created_at"),
            });
        }

        Ok(out)
    }

    async fn list_ai_silver_records(&self, workspace_id: &str) -> StorageResult<Vec<SilverRecord>> {
        let rows = sqlx::query(
            r#"
            SELECT
                silver_id,
                workspace_id,
                bronze_ref,
                chunk_index,
                total_chunks,
                token_count,
                content_hash,
                byte_start,
                byte_end,
                line_start,
                line_end,
                chunk_artifact_path,
                embedding_artifact_path,
                embedding_model_id,
                embedding_model_version,
                embedding_dimensions,
                embedding_compute_latency_ms,
                chunking_strategy,
                chunking_version,
                processing_pipeline_version,
                processed_at,
                processing_duration_ms,
                metadata_json,
                validation_status,
                validation_failed_checks_json,
                validated_at,
                validator_version,
                is_current,
                superseded_by,
                created_at
            FROM ai_silver_records
            WHERE workspace_id = $1
            ORDER BY created_at ASC
            "#,
        )
        .bind(workspace_id)
        .fetch_all(&self.pool)
        .await?;

        let mut out = Vec::with_capacity(rows.len());
        for row in rows {
            out.push(SilverRecord {
                silver_id: row.get("silver_id"),
                workspace_id: row.get("workspace_id"),
                bronze_ref: row.get("bronze_ref"),
                chunk_index: map_i64_from_i32(&row, "chunk_index") as u32,
                total_chunks: map_i64_from_i32(&row, "total_chunks") as u32,
                token_count: map_i64_from_i32(&row, "token_count") as u32,
                content_hash: row.get("content_hash"),
                byte_start: row.get::<i64, _>("byte_start") as u64,
                byte_end: row.get::<i64, _>("byte_end") as u64,
                line_start: map_i64_from_i32(&row, "line_start") as u32,
                line_end: map_i64_from_i32(&row, "line_end") as u32,
                chunk_artifact_path: row.get("chunk_artifact_path"),
                embedding_artifact_path: row.get("embedding_artifact_path"),
                embedding_model_id: row.get("embedding_model_id"),
                embedding_model_version: row.get("embedding_model_version"),
                embedding_dimensions: map_i64_from_i32(&row, "embedding_dimensions") as u32,
                embedding_compute_latency_ms: row.get::<i64, _>("embedding_compute_latency_ms")
                    as u64,
                chunking_strategy: row.get("chunking_strategy"),
                chunking_version: row.get("chunking_version"),
                processing_pipeline_version: row.get("processing_pipeline_version"),
                processed_at: map_timestamp(&row, "processed_at"),
                processing_duration_ms: row.get::<i64, _>("processing_duration_ms") as u64,
                metadata_json: row.get("metadata_json"),
                validation_status: crate::ai_ready_data::records::ValidationStatus::from_str(
                    row.get::<String, _>("validation_status").as_str(),
                )
                .map_err(|_| StorageError::Validation("invalid validation_status"))?,
                validation_failed_checks_json: row.get("validation_failed_checks_json"),
                validated_at: map_timestamp(&row, "validated_at"),
                validator_version: row.get("validator_version"),
                is_current: map_i64_from_i32(&row, "is_current") != 0,
                superseded_by: row.get("superseded_by"),
                created_at: map_timestamp(&row, "created_at"),
            });
        }

        Ok(out)
    }

    async fn supersede_ai_silver_record(
        &self,
        ctx: &WriteContext,
        superseded_silver_id: &str,
        new_silver_id: &str,
    ) -> StorageResult<()> {
        self.guard.validate_write(ctx, superseded_silver_id).await?;
        self.guard.validate_write(ctx, new_silver_id).await?;

        let res = sqlx::query(
            r#"
            UPDATE ai_silver_records
            SET is_current = 0, superseded_by = $2
            WHERE silver_id = $1
            "#,
        )
        .bind(superseded_silver_id)
        .bind(new_silver_id)
        .execute(&self.pool)
        .await?;

        if res.rows_affected() == 0 {
            return Err(StorageError::NotFound("ai_silver_record"));
        }

        Ok(())
    }

    async fn upsert_ai_embedding_model(
        &self,
        ctx: &WriteContext,
        model: EmbeddingModelRecord,
    ) -> StorageResult<()> {
        let key = format!("embedding_model:{}@{}", model.model_id, model.model_version);
        self.guard.validate_write(ctx, &key).await?;

        let content_types_json = serde_json::to_string(&model.content_types)?;
        let compatible_with_json = serde_json::to_string(&model.compatible_with)?;

        sqlx::query(
            r#"
            INSERT INTO ai_embedding_models (
                model_id, model_version, dimensions, max_input_tokens, content_types_json, status, introduced_at, compatible_with_json
            )
            VALUES ($1,$2,$3,$4,$5,$6,$7,$8)
            ON CONFLICT (model_id, model_version) DO UPDATE SET
                dimensions = excluded.dimensions,
                max_input_tokens = excluded.max_input_tokens,
                content_types_json = excluded.content_types_json,
                status = excluded.status,
                compatible_with_json = excluded.compatible_with_json
            "#,
        )
        .bind(model.model_id)
        .bind(model.model_version)
        .bind(model.dimensions as i32)
        .bind(model.max_input_tokens as i32)
        .bind(content_types_json)
        .bind(model.status.as_str())
        .bind(model.introduced_at.naive_utc())
        .bind(compatible_with_json)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn list_ai_embedding_models(&self) -> StorageResult<Vec<EmbeddingModelRecord>> {
        let rows = sqlx::query(
            r#"
            SELECT
                model_id,
                model_version,
                dimensions,
                max_input_tokens,
                content_types_json,
                status,
                introduced_at,
                compatible_with_json
            FROM ai_embedding_models
            ORDER BY model_id ASC, model_version ASC
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        let mut out = Vec::with_capacity(rows.len());
        for row in rows {
            let content_types_json: String = row.get("content_types_json");
            let compatible_with_json: String = row.get("compatible_with_json");
            let content_types: Vec<String> = serde_json::from_str(&content_types_json)?;
            let compatible_with: Vec<String> = serde_json::from_str(&compatible_with_json)?;

            out.push(EmbeddingModelRecord {
                model_id: row.get("model_id"),
                model_version: row.get("model_version"),
                dimensions: map_i64_from_i32(&row, "dimensions") as u32,
                max_input_tokens: map_i64_from_i32(&row, "max_input_tokens") as u32,
                content_types,
                status: crate::ai_ready_data::records::EmbeddingModelStatus::from_str(
                    row.get::<String, _>("status").as_str(),
                )
                .map_err(|_| StorageError::Validation("invalid embedding model status"))?,
                introduced_at: map_timestamp(&row, "introduced_at"),
                compatible_with,
            });
        }

        Ok(out)
    }

    async fn set_ai_embedding_default_model(
        &self,
        ctx: &WriteContext,
        model_id: &str,
        model_version: &str,
    ) -> StorageResult<()> {
        self.guard
            .validate_write(ctx, "ai_embedding_registry")
            .await?;
        let now = Utc::now();

        sqlx::query(
            r#"
            INSERT INTO ai_embedding_registry (
                id, current_default_model_id, current_default_model_version, updated_at
            )
            VALUES ('global', $1, $2, $3)
            ON CONFLICT (id) DO UPDATE SET
                current_default_model_id = excluded.current_default_model_id,
                current_default_model_version = excluded.current_default_model_version,
                updated_at = excluded.updated_at
            "#,
        )
        .bind(model_id)
        .bind(model_version)
        .bind(now.naive_utc())
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn get_ai_embedding_registry(&self) -> StorageResult<Option<EmbeddingRegistry>> {
        let row = sqlx::query(
            r#"
            SELECT
                current_default_model_id,
                current_default_model_version,
                updated_at
            FROM ai_embedding_registry
            WHERE id = 'global'
            "#,
        )
        .fetch_optional(&self.pool)
        .await?;

        let Some(row) = row else {
            return Ok(None);
        };

        Ok(Some(EmbeddingRegistry {
            current_default_model_id: row.get("current_default_model_id"),
            current_default_model_version: row.get("current_default_model_version"),
            updated_at: map_timestamp(&row, "updated_at"),
        }))
    }

    async fn get_ai_job(&self, job_id: &str) -> StorageResult<AiJob> {
        let row = sqlx::query(
            r#"
            SELECT
                id,
                trace_id,
                workflow_run_id,
                job_kind,
                status,
                status_reason,
                error_message,
                protocol_id,
                profile_id,
                capability_profile_id,
                access_mode,
                safety_mode,
                entity_refs,
                planned_operations,
                metrics,
                job_inputs,
                job_outputs,
                created_at,
                updated_at
            FROM ai_jobs
            WHERE id = $1
            "#,
        )
        .bind(job_id)
        .fetch_optional(&self.pool)
        .await?;

        match row {
            Some(row) => map_ai_job(row),
            None => Err(StorageError::NotFound("ai_job")),
        }
    }

    async fn list_ai_jobs(&self, filter: AiJobListFilter) -> StorageResult<Vec<AiJob>> {
        let mut qb = sqlx::QueryBuilder::<sqlx::Postgres>::new(
            r#"
            SELECT
                id,
                trace_id,
                workflow_run_id,
                job_kind,
                status,
                status_reason,
                error_message,
                protocol_id,
                profile_id,
                capability_profile_id,
                access_mode,
                safety_mode,
                entity_refs,
                planned_operations,
                metrics,
                job_inputs,
                job_outputs,
                created_at,
                updated_at
            FROM ai_jobs
            "#,
        );

        let mut has_where = false;
        let mut push_clause = |builder: &mut sqlx::QueryBuilder<sqlx::Postgres>| {
            if has_where {
                builder.push(" AND ");
            } else {
                builder.push(" WHERE ");
                has_where = true;
            }
        };

        if let Some(status) = filter.status {
            push_clause(&mut qb);
            qb.push("status = ").push_bind(status.as_str());
        }
        if let Some(kind) = filter.job_kind {
            push_clause(&mut qb);
            qb.push("job_kind = ").push_bind(kind.as_str());
        }
        if let Some(wsid) = filter.wsid {
            push_clause(&mut qb);
            qb.push(
                "EXISTS (SELECT 1 FROM jsonb_array_elements(entity_refs::jsonb) AS elem WHERE elem->>'entity_kind' = 'workspace' AND elem->>'entity_id' = ",
            )
            .push_bind(wsid)
            .push(")");
        }
        if let Some(from) = filter.from {
            push_clause(&mut qb);
            qb.push("created_at >= ").push_bind(from);
        }
        if let Some(to) = filter.to {
            push_clause(&mut qb);
            qb.push("created_at <= ").push_bind(to);
        }

        qb.push(" ORDER BY created_at DESC LIMIT ");
        qb.push_bind(200_i64);

        let rows = qb.build().fetch_all(&self.pool).await?;
        rows.into_iter().map(map_ai_job).collect()
    }

    async fn create_ai_job(&self, job: NewAiJob) -> StorageResult<AiJob> {
        validate_job_contract(&job.job_kind, &job.profile_id, &job.protocol_id)?;

        let id = Uuid::now_v7().to_string();
        let now = Utc::now();
        let job_inputs = job.job_inputs.clone().map(|value| value.to_string());
        let metrics_json = serde_json::to_string(&job.metrics)?;
        let entity_refs_json = serde_json::to_string(&job.entity_refs)?;
        let planned_ops_json = serde_json::to_string(&job.planned_operations)?;

        let row = sqlx::query(
            r#"
            INSERT INTO ai_jobs (
                id,
                trace_id,
                workflow_run_id,
                job_kind,
                status,
                status_reason,
                protocol_id,
                profile_id,
                capability_profile_id,
                access_mode,
                safety_mode,
                entity_refs,
                planned_operations,
                metrics,
                job_inputs,
                created_at,
                updated_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17)
            RETURNING
                id,
                trace_id,
                workflow_run_id,
                job_kind,
                status,
                status_reason,
                error_message,
                protocol_id,
                profile_id,
                capability_profile_id,
                access_mode,
                safety_mode,
                entity_refs,
                planned_operations,
                metrics,
                job_inputs,
                job_outputs,
                created_at,
                updated_at
            "#,
        )
        .bind(&id)
        .bind(job.trace_id.to_string())
        .bind(Option::<String>::None)
        .bind(job.job_kind.as_str())
        .bind(JobState::Queued.as_str())
        .bind(&job.status_reason)
        .bind(&job.protocol_id)
        .bind(&job.profile_id)
        .bind(&job.capability_profile_id)
        .bind(job.access_mode.as_str())
        .bind(job.safety_mode.as_str())
        .bind(entity_refs_json)
        .bind(planned_ops_json)
        .bind(metrics_json)
        .bind(&job_inputs)
        .bind(now)
        .bind(now)
        .fetch_one(&self.pool)
        .await?;

        map_ai_job(row)
    }

    async fn update_ai_job_status(&self, update: JobStatusUpdate) -> StorageResult<AiJob> {
        let job_outputs = update.job_outputs.as_ref().map(|val| val.to_string());
        let metrics_json = update
            .metrics
            .as_ref()
            .map(serde_json::to_string)
            .transpose()?;
        let now = Utc::now();
        let row = sqlx::query(
            r#"
            UPDATE ai_jobs
            SET status = $1,
                status_reason = $2,
                metrics = COALESCE($3, metrics),
                workflow_run_id = COALESCE($4, workflow_run_id),
                trace_id = COALESCE($5, trace_id),
                error_message = COALESCE($6, error_message),
                job_outputs = COALESCE($7, job_outputs),
                updated_at = $8
            WHERE id = $9
            RETURNING
                id,
                trace_id,
                workflow_run_id,
                job_kind,
                status,
                status_reason,
                error_message,
                protocol_id,
                profile_id,
                capability_profile_id,
                access_mode,
                safety_mode,
                entity_refs,
                planned_operations,
                metrics,
                job_inputs,
                job_outputs,
                created_at,
                updated_at
            "#,
        )
        .bind(update.state.as_str())
        .bind(&update.status_reason)
        .bind(metrics_json)
        .bind(update.workflow_run_id.map(|id| id.to_string()))
        .bind(update.trace_id.map(|id| id.to_string()))
        .bind(update.error_message.clone())
        .bind(&job_outputs)
        .bind(now)
        .bind(update.job_id.to_string())
        .fetch_optional(&self.pool)
        .await?;

        match row {
            Some(row) => map_ai_job(row),
            None => Err(StorageError::NotFound("ai_job")),
        }
    }

    async fn set_job_outputs(
        &self,
        job_id: &str,
        outputs: Option<serde_json::Value>,
    ) -> StorageResult<()> {
        let now = Utc::now();
        let outputs = outputs.map(|val| val.to_string());
        sqlx::query(
            r#"
            UPDATE ai_jobs
            SET job_outputs = $1,
                updated_at = $2
            WHERE id = $3
            "#,
        )
        .bind(&outputs)
        .bind(now)
        .bind(job_id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn upsert_model_session(&self, session: NewModelSession) -> StorageResult<ModelSession> {
        self.ensure_model_session_schema().await?;

        let now = Utc::now();
        let session_id = session.session_id.clone();
        let memory_policy = session.memory_policy.clone();
        let capability_grants = serde_json::to_string(&session.capability_grants)?;
        let capability_token_ids = session
            .capability_token_ids
            .as_ref()
            .map(serde_json::to_string)
            .transpose()?;
        let job_id = session.job_id.map(|value| value.to_string());

        let row = sqlx::query(
            r#"
            INSERT INTO model_sessions (
                session_id,
                parent_session_id,
                spawn_depth,
                state,
                model_id,
                backend,
                parameter_class,
                role,
                wp_id,
                mt_id,
                work_profile_id,
                execution_mode,
                memory_policy,
                consent_receipt_id,
                capability_grants,
                capability_token_ids,
                job_id,
                checkpoint_artifact_id,
                last_checkpoint_at,
                checkpoint_count,
                agent,
                purpose,
                created_at,
                updated_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19, $20, $21, $22, $23, $24)
            ON CONFLICT(session_id) DO UPDATE SET
                parent_session_id = excluded.parent_session_id,
                spawn_depth = excluded.spawn_depth,
                state = excluded.state,
                model_id = excluded.model_id,
                backend = excluded.backend,
                parameter_class = excluded.parameter_class,
                role = excluded.role,
                wp_id = excluded.wp_id,
                mt_id = excluded.mt_id,
                work_profile_id = excluded.work_profile_id,
                execution_mode = excluded.execution_mode,
                consent_receipt_id = excluded.consent_receipt_id,
                capability_grants = excluded.capability_grants,
                capability_token_ids = excluded.capability_token_ids,
                job_id = excluded.job_id,
                checkpoint_artifact_id = excluded.checkpoint_artifact_id,
                last_checkpoint_at = excluded.last_checkpoint_at,
                checkpoint_count = excluded.checkpoint_count,
                agent = excluded.agent,
                purpose = excluded.purpose,
                updated_at = excluded.updated_at
            WHERE model_sessions.memory_policy = excluded.memory_policy
            RETURNING
                session_id,
                parent_session_id,
                spawn_depth,
                state,
                model_id,
                backend,
                parameter_class,
                role,
                wp_id,
                mt_id,
                work_profile_id,
                execution_mode,
                memory_policy,
                consent_receipt_id,
                capability_grants,
                capability_token_ids,
                job_id,
                checkpoint_artifact_id,
                last_checkpoint_at,
                checkpoint_count,
                merge_back_artifact,
                agent,
                purpose,
                close_reason,
                closed_by_actor,
                closed_at,
                created_at,
                updated_at
            "#,
        )
        .bind(session.session_id)
        .bind(session.parent_session_id)
        .bind(session.spawn_depth)
        .bind(session.state.as_str())
        .bind(session.model_id)
        .bind(session.backend)
        .bind(session.parameter_class)
        .bind(session.role)
        .bind(session.wp_id)
        .bind(session.mt_id)
        .bind(session.work_profile_id)
                .bind(session.execution_mode)
                .bind(session.memory_policy)
                .bind(session.consent_receipt_id)
                .bind(capability_grants)
                .bind(capability_token_ids)
                .bind(job_id)
                .bind(session.checkpoint_artifact_id)
                .bind(session.last_checkpoint_at)
                .bind(session.checkpoint_count)
                .bind(session.agent)
                .bind(session.purpose)
                .bind(now)
                .bind(now)
                .fetch_optional(&self.pool)
        .await?;

        match row {
            Some(row) => map_model_session(row),
            None => {
                // INV-SESS-004: memory_policy is immutable once declared at session creation.
                let existing =
                    sqlx::query("SELECT memory_policy FROM model_sessions WHERE session_id = $1")
                        .bind(&session_id)
                        .fetch_optional(&self.pool)
                        .await?;
                if let Some(existing) = existing {
                    let existing_policy: String = existing.get("memory_policy");
                    if existing_policy != memory_policy {
                        return Err(StorageError::Validation(
                            "memory_policy is immutable for an existing session",
                        ));
                    }
                    return self.get_model_session(session_id.as_str()).await;
                }
                Err(StorageError::NotFound("model_session"))
            }
        }
    }

    async fn get_model_session(&self, session_id: &str) -> StorageResult<ModelSession> {
        self.ensure_model_session_schema().await?;
        let row = sqlx::query(
            r#"
            SELECT
                session_id,
                parent_session_id,
                spawn_depth,
                state,
                model_id,
                backend,
                parameter_class,
                role,
                wp_id,
                mt_id,
                work_profile_id,
                execution_mode,
                memory_policy,
                consent_receipt_id,
                capability_grants,
                capability_token_ids,
                job_id,
                checkpoint_artifact_id,
                last_checkpoint_at,
                checkpoint_count,
                merge_back_artifact,
                agent,
                purpose,
                close_reason,
                closed_by_actor,
                closed_at,
                created_at,
                updated_at
            FROM model_sessions
            WHERE session_id = $1
            "#,
        )
        .bind(session_id)
        .fetch_optional(&self.pool)
        .await?;

        match row {
            Some(row) => map_model_session(row),
            None => Err(StorageError::NotFound("model_session")),
        }
    }

    async fn get_model_session_by_job_id(&self, job_id: Uuid) -> StorageResult<ModelSession> {
        self.ensure_model_session_schema().await?;
        let row = sqlx::query(
            r#"
            SELECT
                session_id,
                parent_session_id,
                spawn_depth,
                state,
                model_id,
                backend,
                parameter_class,
                role,
                wp_id,
                mt_id,
                work_profile_id,
                execution_mode,
                memory_policy,
                consent_receipt_id,
                capability_grants,
                capability_token_ids,
                job_id,
                checkpoint_artifact_id,
                last_checkpoint_at,
                checkpoint_count,
                merge_back_artifact,
                agent,
                purpose,
                close_reason,
                closed_by_actor,
                closed_at,
                created_at,
                updated_at
            FROM model_sessions
            WHERE job_id = $1
            "#,
        )
        .bind(job_id.to_string())
        .fetch_optional(&self.pool)
        .await?;

        match row {
            Some(row) => map_model_session(row),
            None => Err(StorageError::NotFound("model_session")),
        }
    }

    async fn update_model_session_state(
        &self,
        session_id: &str,
        state: ModelSessionState,
        job_id: Option<Uuid>,
    ) -> StorageResult<ModelSession> {
        self.update_model_session_state_with_merge_back_artifact(session_id, state, job_id, None)
            .await
    }

    async fn update_model_session_state_with_merge_back_artifact(
        &self,
        session_id: &str,
        state: ModelSessionState,
        job_id: Option<Uuid>,
        merge_back_artifact: Option<MergeBackArtifact>,
    ) -> StorageResult<ModelSession> {
        self.ensure_model_session_schema().await?;
        let now = Utc::now();
        let merge_back_artifact = merge_back_artifact
            .as_ref()
            .map(serde_json::to_string)
            .transpose()?;
        let row = sqlx::query(
            r#"
            UPDATE model_sessions
            SET state = $1,
                job_id = COALESCE($2, job_id),
                merge_back_artifact = $3,
                updated_at = $4
            WHERE session_id = $5
            RETURNING
                session_id,
                parent_session_id,
                spawn_depth,
                state,
                model_id,
                backend,
                parameter_class,
                role,
                wp_id,
                mt_id,
                work_profile_id,
                execution_mode,
                memory_policy,
                consent_receipt_id,
                capability_grants,
                capability_token_ids,
                job_id,
                checkpoint_artifact_id,
                last_checkpoint_at,
                checkpoint_count,
                merge_back_artifact,
                agent,
                purpose,
                close_reason,
                closed_by_actor,
                closed_at,
                created_at,
                updated_at
            "#,
        )
        .bind(state.as_str())
        .bind(job_id.map(|value| value.to_string()))
        .bind(merge_back_artifact)
        .bind(now)
        .bind(session_id)
        .fetch_optional(&self.pool)
        .await?;

        match row {
            Some(row) => map_model_session(row),
            None => Err(StorageError::NotFound("model_session")),
        }
    }

    async fn close_model_session(
        &self,
        session_id: &str,
        state: ModelSessionState,
        close_reason: &str,
        actor: &str,
    ) -> StorageResult<ModelSession> {
        if !state.is_terminal() {
            return Err(StorageError::Validation(
                "close_model_session requires a terminal session state",
            ));
        }
        if close_reason.trim().is_empty() {
            return Err(StorageError::Validation(
                "close_model_session requires a non-empty close_reason",
            ));
        }
        if actor.trim().is_empty() {
            return Err(StorageError::Validation(
                "close_model_session requires a non-empty actor",
            ));
        }

        self.ensure_model_session_schema().await?;
        let now = Utc::now();
        let row = sqlx::query(
            r#"
            UPDATE model_sessions
            SET state = $1,
                close_reason = $2,
                closed_by_actor = $3,
                closed_at = $4,
                updated_at = $4
            WHERE session_id = $5
            RETURNING
                session_id,
                parent_session_id,
                spawn_depth,
                state,
                model_id,
                backend,
                parameter_class,
                role,
                wp_id,
                mt_id,
                work_profile_id,
                execution_mode,
                memory_policy,
                consent_receipt_id,
                capability_grants,
                capability_token_ids,
                job_id,
                checkpoint_artifact_id,
                last_checkpoint_at,
                checkpoint_count,
                merge_back_artifact,
                agent,
                purpose,
                close_reason,
                closed_by_actor,
                closed_at,
                created_at,
                updated_at
            "#,
        )
        .bind(state.as_str())
        .bind(close_reason)
        .bind(actor)
        .bind(now)
        .bind(session_id)
        .fetch_optional(&self.pool)
        .await?;

        match row {
            Some(row) => map_model_session(row),
            None => Err(StorageError::NotFound("model_session")),
        }
    }

    async fn create_session_checkpoint(
        &self,
        checkpoint: SessionCheckpoint,
    ) -> StorageResult<SessionCheckpoint> {
        self.ensure_model_session_schema().await?;
        let row = sqlx::query(
            r#"
            INSERT INTO session_checkpoints (
                checkpoint_id,
                session_id,
                timestamp,
                session_state_json,
                message_thread_tail_id,
                pending_tool_calls_json,
                checkpoint_artifact_id
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            RETURNING
                checkpoint_id,
                session_id,
                timestamp,
                session_state_json,
                message_thread_tail_id,
                pending_tool_calls_json,
                checkpoint_artifact_id
            "#,
        )
        .bind(checkpoint.checkpoint_id)
        .bind(checkpoint.session_id)
        .bind(checkpoint.timestamp)
        .bind(checkpoint.session_state_json)
        .bind(checkpoint.message_thread_tail_id)
        .bind(checkpoint.pending_tool_calls_json)
        .bind(checkpoint.checkpoint_artifact_id)
        .fetch_one(&self.pool)
        .await?;
        map_session_checkpoint_row(row)
    }

    async fn get_latest_session_checkpoint(
        &self,
        session_id: &str,
    ) -> StorageResult<SessionCheckpoint> {
        self.ensure_model_session_schema().await?;
        let row = sqlx::query(
            r#"
            SELECT
                checkpoint_id,
                session_id,
                timestamp,
                session_state_json,
                message_thread_tail_id,
                pending_tool_calls_json,
                checkpoint_artifact_id
            FROM session_checkpoints
            WHERE session_id = $1
            ORDER BY timestamp DESC, checkpoint_id DESC
            LIMIT 1
            "#,
        )
        .bind(session_id)
        .fetch_optional(&self.pool)
        .await?;

        match row {
            Some(row) => map_session_checkpoint_row(row),
            None => Err(StorageError::NotFound("session_checkpoint")),
        }
    }

    async fn append_session_message(
        &self,
        message: NewSessionMessage,
    ) -> StorageResult<SessionMessage> {
        self.ensure_model_session_schema().await?;
        if !is_sha256_hex(message.content_hash.as_str()) {
            return Err(StorageError::Validation("invalid content_hash"));
        }

        let message_id = message
            .message_id
            .unwrap_or_else(|| Uuid::now_v7().to_string());
        let attachments = serde_json::to_string(&message.attachments)?;
        let row = sqlx::query(
            r#"
            INSERT INTO session_messages (
                message_id,
                session_id,
                role,
                content_hash,
                content_artifact_id,
                token_count,
                redacted,
                tool_call_id,
                attachments
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            RETURNING
                message_id,
                session_id,
                role,
                content_hash,
                content_artifact_id,
                token_count,
                redacted,
                tool_call_id,
                attachments,
                created_at
            "#,
        )
        .bind(message_id)
        .bind(message.session_id)
        .bind(message.role.as_str())
        .bind(message.content_hash)
        .bind(message.content_artifact_id)
        .bind(message.token_count)
        .bind(message.redacted)
        .bind(message.tool_call_id)
        .bind(attachments)
        .fetch_one(&self.pool)
        .await?;

        map_session_message(row)
    }

    async fn list_session_messages(&self, session_id: &str) -> StorageResult<Vec<SessionMessage>> {
        self.ensure_model_session_schema().await?;
        let rows = sqlx::query(
            r#"
            SELECT
                message_id,
                session_id,
                role,
                content_hash,
                content_artifact_id,
                token_count,
                redacted,
                tool_call_id,
                attachments,
                created_at
            FROM session_messages
            WHERE session_id = $1
            ORDER BY created_at ASC, message_id ASC
            "#,
        )
        .bind(session_id)
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter().map(map_session_message).collect()
    }

    async fn append_kernel_event(&self, event: NewKernelEvent) -> StorageResult<KernelEvent> {
        append_kernel_event_with_executor(&self.pool, event).await
    }

    async fn append_kernel_events_atomic(
        &self,
        events: Vec<NewKernelEvent>,
    ) -> StorageResult<Vec<KernelEvent>> {
        let mut tx = self.pool.begin().await?;
        let mut appended = Vec::with_capacity(events.len());
        for event in events {
            appended.push(append_kernel_event_with_executor(&mut *tx, event).await?);
        }
        tx.commit().await?;
        Ok(appended)
    }

    async fn append_kernel_event_pair_atomic_with_causation(
        &self,
        first: NewKernelEvent,
        mut second: NewKernelEvent,
    ) -> StorageResult<Vec<KernelEvent>> {
        let mut tx = self.pool.begin().await?;
        let first_event = append_kernel_event_with_executor(&mut *tx, first).await?;
        second.causation_id = Some(first_event.event_id.clone());
        let second_event = append_kernel_event_with_executor(&mut *tx, second).await?;
        tx.commit().await?;
        Ok(vec![first_event, second_event])
    }

    /// WP-KERNEL-009 authority-hardening #2: ledger pair + fact insert +
    /// proposal flip in a SINGLE transaction (atomic promotion). See the
    /// trait doc. The fact insert is `ON CONFLICT (proposal_id) DO NOTHING`
    /// so a crashed-then-retried promotion converges on one fact row; the
    /// committed event ids on that row are the first writer's.
    async fn promote_graph_fact_atomic(
        &self,
        requested: NewKernelEvent,
        mut accepted: NewKernelEvent,
        fact: crate::storage::knowledge_crdt::NewPromotedFact,
    ) -> StorageResult<crate::storage::knowledge_crdt::PromotedFactRow> {
        let mut tx = self.pool.begin().await?;
        let requested_event = append_kernel_event_with_executor(&mut *tx, requested).await?;
        accepted.causation_id = Some(requested_event.event_id.clone());
        let accepted_event = append_kernel_event_with_executor(&mut *tx, accepted).await?;

        // Frozen authority fact, carrying the just-appended ledger receipts.
        // Migration 0190's trigger re-validates span existence at INSERT time
        // as the schema backstop; an invalid ref aborts the whole tx.
        sqlx::query(
            r#"
            INSERT INTO knowledge_crdt_promoted_facts (
                fact_id, proposal_id, workspace_id, mutation_kind, fact_payload,
                source_span_refs, confidence, proposed_by, promoted_by,
                promotion_requested_event_id, promotion_accepted_event_id
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
            ON CONFLICT (proposal_id) DO NOTHING
            "#,
        )
        .bind(&fact.fact_id)
        .bind(&fact.proposal_id)
        .bind(&fact.workspace_id)
        .bind(&fact.mutation_kind)
        .bind(&fact.fact_payload)
        .bind(&fact.source_span_refs)
        .bind(fact.confidence)
        .bind(&fact.proposed_by)
        .bind(&fact.promoted_by)
        .bind(&requested_event.event_id)
        .bind(&accepted_event.event_id)
        .execute(&mut *tx)
        .await?;

        // Finalize the proposal lifecycle in the same tx (approved ->
        // promoted; idempotent: a re-promotion is already 'promoted').
        sqlx::query(
            r#"
            UPDATE knowledge_crdt_graph_proposals
            SET review_state = 'promoted'
            WHERE proposal_id = $1 AND review_state = 'approved'
            "#,
        )
        .bind(&fact.proposal_id)
        .execute(&mut *tx)
        .await?;

        let row = sqlx::query(&format!(
            "SELECT {cols} FROM knowledge_crdt_promoted_facts WHERE proposal_id = $1",
            cols = crate::storage::knowledge_crdt::PROMOTED_FACT_COLUMNS,
        ))
        .bind(&fact.proposal_id)
        .fetch_optional(&mut *tx)
        .await?;

        tx.commit().await?;

        let row = row.ok_or(StorageError::NotFound(
            "promoted fact after atomic promotion",
        ))?;
        crate::storage::knowledge_crdt::map_promoted_fact_row(row)
    }

    async fn list_kernel_events_for_session(
        &self,
        session_run_id: &str,
    ) -> StorageResult<Vec<KernelEvent>> {
        let rows = sqlx::query(
            r#"
            SELECT
                event_id,
                event_sequence,
                event_version,
                kernel_task_run_id,
                session_run_id,
                aggregate_type,
                aggregate_id,
                idempotency_key,
                event_type,
                actor_kind,
                actor_id,
                causation_id,
                correlation_id,
                payload_hash,
                source_component,
                payload::text AS payload,
                created_at
            FROM kernel_event_ledger
            WHERE session_run_id = $1
            ORDER BY event_sequence ASC
            "#,
        )
        .bind(session_run_id)
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter().map(map_kernel_event).collect()
    }

    async fn list_kernel_events_for_aggregate(
        &self,
        aggregate_type: &str,
        aggregate_id: &str,
    ) -> StorageResult<Vec<KernelEvent>> {
        let rows = sqlx::query(
            r#"
            SELECT
                event_id,
                event_sequence,
                event_version,
                kernel_task_run_id,
                session_run_id,
                aggregate_type,
                aggregate_id,
                idempotency_key,
                event_type,
                actor_kind,
                actor_id,
                causation_id,
                correlation_id,
                payload_hash,
                source_component,
                payload::text AS payload,
                created_at
            FROM kernel_event_ledger
            WHERE aggregate_type = $1 AND aggregate_id = $2
            ORDER BY event_sequence ASC
            "#,
        )
        .bind(aggregate_type)
        .bind(aggregate_id)
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter().map(map_kernel_event).collect()
    }

    async fn append_kernel_crdt_update(
        &self,
        record: CrdtUpdateRecordV1,
        update_bytes: Vec<u8>,
    ) -> StorageResult<CrdtUpdateRecordV1> {
        validate_crdt_update_record(&record)
            .map_err(|_| StorageError::Validation("invalid kernel CRDT update record"))?;
        if crdt_sha256_hex(&update_bytes) != record.update_sha256 {
            return Err(StorageError::Validation(
                "kernel CRDT update bytes do not match update_sha256",
            ));
        }
        ensure_kernel_crdt_event_ref_exists(&self.pool, &record.event_ledger_event_id).await?;
        let update_seq = i64::try_from(record.update_seq)
            .map_err(|_| StorageError::Validation("kernel CRDT update sequence too large"))?;
        let replay_metadata_json = serde_json::to_string(&record.replay_metadata)?;

        let maybe_row = sqlx::query(
            r#"
            INSERT INTO kernel_crdt_updates (
                schema_id,
                workspace_id,
                document_id,
                crdt_document_id,
                update_id,
                update_seq,
                update_sha256,
                update_bytes_ref,
                update_bytes,
                actor_id,
                actor_kind,
                session_id,
                trace_id,
                state_vector_before,
                state_vector_after,
                replay_metadata_json,
                event_ledger_stream_id,
                event_ledger_event_id,
                storage_authority
            )
            VALUES (
                $1, $2, $3, $4, $5, $6, $7, $8, $9,
                $10, $11, $12, $13, $14, $15, $16::jsonb,
                $17, $18, $19
            )
            ON CONFLICT (workspace_id, document_id, crdt_document_id, update_id) DO NOTHING
            RETURNING
                schema_id,
                workspace_id,
                document_id,
                crdt_document_id,
                update_id,
                update_seq,
                update_sha256,
                update_bytes_ref,
                actor_id,
                actor_kind,
                session_id,
                trace_id,
                state_vector_before,
                state_vector_after,
                replay_metadata_json::text AS replay_metadata_json,
                event_ledger_stream_id,
                event_ledger_event_id,
                storage_authority
            "#,
        )
        .bind(&record.schema_id)
        .bind(&record.workspace_id)
        .bind(&record.document_id)
        .bind(&record.crdt_document_id)
        .bind(&record.update_id)
        .bind(update_seq)
        .bind(&record.update_sha256)
        .bind(&record.update_bytes_ref)
        .bind(update_bytes)
        .bind(&record.actor_id)
        .bind(&record.actor_kind)
        .bind(&record.session_id)
        .bind(&record.trace_id)
        .bind(&record.state_vector_before)
        .bind(&record.state_vector_after)
        .bind(replay_metadata_json)
        .bind(&record.event_ledger_stream_id)
        .bind(&record.event_ledger_event_id)
        .bind(crdt_storage_authority_str(record.storage_authority))
        .fetch_optional(&self.pool)
        .await?;

        let stored = match maybe_row {
            Some(row) => map_kernel_crdt_update(row)?,
            None => {
                let rows = self
                    .list_kernel_crdt_updates(
                        &record.workspace_id,
                        &record.document_id,
                        &record.crdt_document_id,
                    )
                    .await?;
                rows.into_iter()
                    .find(|stored| stored.update_id == record.update_id)
                    .ok_or(StorageError::Conflict(
                        "kernel CRDT update idempotency conflict",
                    ))?
            }
        };
        if stored != record {
            return Err(StorageError::Conflict(
                "kernel CRDT update idempotency conflict",
            ));
        }
        Ok(stored)
    }

    async fn list_kernel_crdt_updates(
        &self,
        workspace_id: &str,
        document_id: &str,
        crdt_document_id: &str,
    ) -> StorageResult<Vec<CrdtUpdateRecordV1>> {
        let rows = sqlx::query(
            r#"
            SELECT
                schema_id,
                workspace_id,
                document_id,
                crdt_document_id,
                update_id,
                update_seq,
                update_sha256,
                update_bytes_ref,
                actor_id,
                actor_kind,
                session_id,
                trace_id,
                state_vector_before,
                state_vector_after,
                replay_metadata_json::text AS replay_metadata_json,
                event_ledger_stream_id,
                event_ledger_event_id,
                storage_authority
            FROM kernel_crdt_updates
            WHERE workspace_id = $1
              AND document_id = $2
              AND crdt_document_id = $3
            ORDER BY update_seq ASC
            "#,
        )
        .bind(workspace_id)
        .bind(document_id)
        .bind(crdt_document_id)
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter().map(map_kernel_crdt_update).collect()
    }

    async fn read_kernel_crdt_update_bytes(
        &self,
        update_bytes_ref: &str,
    ) -> StorageResult<Vec<u8>> {
        sqlx::query_scalar::<_, Vec<u8>>(
            r#"
            SELECT update_bytes
            FROM kernel_crdt_updates
            WHERE update_bytes_ref = $1
            "#,
        )
        .bind(update_bytes_ref)
        .fetch_optional(&self.pool)
        .await?
        .ok_or(StorageError::NotFound("kernel CRDT update bytes"))
    }

    async fn append_kernel_crdt_snapshot(
        &self,
        record: CrdtSnapshotRecordV1,
        snapshot_bytes: Vec<u8>,
    ) -> StorageResult<CrdtSnapshotRecordV1> {
        validate_crdt_snapshot_record(&record)
            .map_err(|_| StorageError::Validation("invalid kernel CRDT snapshot record"))?;
        if crdt_sha256_hex(&snapshot_bytes) != record.snapshot_sha256 {
            return Err(StorageError::Validation(
                "kernel CRDT snapshot bytes do not match snapshot_sha256",
            ));
        }
        ensure_kernel_crdt_event_ref_exists(&self.pool, &record.event_ledger_event_id).await?;
        let covered_update_seq = i64::try_from(record.covered_update_seq).map_err(|_| {
            StorageError::Validation("kernel CRDT snapshot covered sequence too large")
        })?;
        let promotion_evidence_json = serde_json::to_string(&record.promotion_evidence_update_ids)?;

        let maybe_row = sqlx::query(
            r#"
            INSERT INTO kernel_crdt_snapshots (
                schema_id,
                snapshot_id,
                workspace_id,
                document_id,
                crdt_document_id,
                covered_update_seq,
                state_vector,
                snapshot_sha256,
                snapshot_bytes_ref,
                snapshot_bytes,
                actor_id,
                actor_kind,
                event_ledger_stream_id,
                event_ledger_event_id,
                promotion_evidence_update_ids,
                storage_authority
            )
            VALUES (
                $1, $2, $3, $4, $5, $6, $7, $8, $9,
                $10, $11, $12, $13, $14, $15::jsonb, $16
            )
            ON CONFLICT (workspace_id, document_id, crdt_document_id, snapshot_id) DO NOTHING
            RETURNING
                schema_id,
                snapshot_id,
                workspace_id,
                document_id,
                crdt_document_id,
                covered_update_seq,
                state_vector,
                snapshot_sha256,
                snapshot_bytes_ref,
                actor_id,
                actor_kind,
                event_ledger_stream_id,
                event_ledger_event_id,
                promotion_evidence_update_ids::text AS promotion_evidence_update_ids,
                storage_authority
            "#,
        )
        .bind(&record.schema_id)
        .bind(&record.snapshot_id)
        .bind(&record.workspace_id)
        .bind(&record.document_id)
        .bind(&record.crdt_document_id)
        .bind(covered_update_seq)
        .bind(&record.state_vector)
        .bind(&record.snapshot_sha256)
        .bind(&record.snapshot_bytes_ref)
        .bind(snapshot_bytes)
        .bind(&record.actor_id)
        .bind(&record.actor_kind)
        .bind(&record.event_ledger_stream_id)
        .bind(&record.event_ledger_event_id)
        .bind(promotion_evidence_json)
        .bind(crdt_storage_authority_str(record.storage_authority))
        .fetch_optional(&self.pool)
        .await?;

        let stored = match maybe_row {
            Some(row) => map_kernel_crdt_snapshot(row)?,
            None => {
                let rows = self
                    .list_kernel_crdt_snapshots(
                        &record.workspace_id,
                        &record.document_id,
                        &record.crdt_document_id,
                    )
                    .await?;
                rows.into_iter()
                    .find(|stored| stored.snapshot_id == record.snapshot_id)
                    .ok_or(StorageError::Conflict(
                        "kernel CRDT snapshot idempotency conflict",
                    ))?
            }
        };
        if stored != record {
            return Err(StorageError::Conflict(
                "kernel CRDT snapshot idempotency conflict",
            ));
        }
        Ok(stored)
    }

    async fn list_kernel_crdt_snapshots(
        &self,
        workspace_id: &str,
        document_id: &str,
        crdt_document_id: &str,
    ) -> StorageResult<Vec<CrdtSnapshotRecordV1>> {
        let rows = sqlx::query(
            r#"
            SELECT
                schema_id,
                snapshot_id,
                workspace_id,
                document_id,
                crdt_document_id,
                covered_update_seq,
                state_vector,
                snapshot_sha256,
                snapshot_bytes_ref,
                actor_id,
                actor_kind,
                event_ledger_stream_id,
                event_ledger_event_id,
                promotion_evidence_update_ids::text AS promotion_evidence_update_ids,
                storage_authority
            FROM kernel_crdt_snapshots
            WHERE workspace_id = $1
              AND document_id = $2
              AND crdt_document_id = $3
            ORDER BY covered_update_seq DESC, snapshot_id ASC
            "#,
        )
        .bind(workspace_id)
        .bind(document_id)
        .bind(crdt_document_id)
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter().map(map_kernel_crdt_snapshot).collect()
    }

    async fn read_kernel_crdt_snapshot_bytes(
        &self,
        snapshot_bytes_ref: &str,
    ) -> StorageResult<Vec<u8>> {
        sqlx::query_scalar::<_, Vec<u8>>(
            r#"
            SELECT snapshot_bytes
            FROM kernel_crdt_snapshots
            WHERE snapshot_bytes_ref = $1
            "#,
        )
        .bind(snapshot_bytes_ref)
        .fetch_optional(&self.pool)
        .await?
        .ok_or(StorageError::NotFound("kernel CRDT snapshot bytes"))
    }

    async fn enqueue_kernel_session_run(&self, session: SessionRun) -> StorageResult<SessionRun> {
        if session.session_run_id.trim().is_empty()
            || session.kernel_task_run_id.trim().is_empty()
            || session.adapter_id.trim().is_empty()
        {
            return Err(StorageError::Validation("invalid kernel session run"));
        }

        let row = sqlx::query(
            r#"
            INSERT INTO kernel_session_queue (
                session_run_id,
                kernel_task_run_id,
                adapter_id,
                state,
                attempt_count,
                available_at,
                created_at,
                updated_at
            )
            VALUES ($1, $2, $3, $4, 0, CURRENT_TIMESTAMP, $5, $6)
            ON CONFLICT (session_run_id) DO UPDATE SET
                kernel_task_run_id = excluded.kernel_task_run_id,
                adapter_id = excluded.adapter_id,
                updated_at = excluded.updated_at
            RETURNING
                session_run_id,
                kernel_task_run_id,
                adapter_id,
                state,
                claimed_by,
                lease_expires_at,
                attempt_count,
                created_at,
                updated_at
            "#,
        )
        .bind(&session.session_run_id)
        .bind(&session.kernel_task_run_id)
        .bind(&session.adapter_id)
        .bind(session.state.as_str())
        .bind(session.created_at)
        .bind(session.updated_at)
        .fetch_one(&self.pool)
        .await?;

        let stored = map_kernel_session_lease(row)?;
        Ok(SessionRun {
            session_run_id: stored.session_run_id,
            kernel_task_run_id: stored.kernel_task_run_id,
            adapter_id: stored.adapter_id,
            state: stored.state,
            created_at: stored.created_at,
            updated_at: stored.updated_at,
        })
    }

    async fn enqueue_kernel_session_run_and_record_event(
        &self,
        session: SessionRun,
        causation_id: Option<String>,
        correlation_id: String,
    ) -> StorageResult<(SessionRun, KernelEvent)> {
        if session.session_run_id.trim().is_empty()
            || session.kernel_task_run_id.trim().is_empty()
            || session.adapter_id.trim().is_empty()
        {
            return Err(StorageError::Validation("invalid kernel session run"));
        }

        let mut tx = self.pool.begin().await?;
        let row = sqlx::query(
            r#"
            INSERT INTO kernel_session_queue (
                session_run_id,
                kernel_task_run_id,
                adapter_id,
                state,
                attempt_count,
                available_at,
                created_at,
                updated_at
            )
            VALUES ($1, $2, $3, $4, 0, CURRENT_TIMESTAMP, $5, $6)
            ON CONFLICT (session_run_id) DO UPDATE SET
                kernel_task_run_id = excluded.kernel_task_run_id,
                adapter_id = excluded.adapter_id,
                updated_at = excluded.updated_at
            RETURNING
                session_run_id,
                kernel_task_run_id,
                adapter_id,
                state,
                claimed_by,
                lease_expires_at,
                attempt_count,
                created_at,
                updated_at
            "#,
        )
        .bind(&session.session_run_id)
        .bind(&session.kernel_task_run_id)
        .bind(&session.adapter_id)
        .bind(session.state.as_str())
        .bind(session.created_at)
        .bind(session.updated_at)
        .fetch_one(&mut *tx)
        .await?;

        let stored = map_kernel_session_lease(row)?;
        let queued = SessionRun {
            session_run_id: stored.session_run_id,
            kernel_task_run_id: stored.kernel_task_run_id,
            adapter_id: stored.adapter_id,
            state: stored.state,
            created_at: stored.created_at,
            updated_at: stored.updated_at,
        };
        let event = build_kernel_session_event(
            &queued.kernel_task_run_id,
            &queued.session_run_id,
            KernelEventType::SessionQueued,
            causation_id,
            correlation_id,
            json!({
                "session_run_id": queued.session_run_id.clone(),
                "adapter_id": queued.adapter_id.clone(),
                "state": queued.state.as_str()
            }),
        )?;
        let stored_event = append_kernel_event_with_executor(&mut *tx, event).await?;
        tx.commit().await?;

        Ok((queued, stored_event))
    }

    async fn claim_kernel_session_run(
        &self,
        session_run_id: &str,
        claimed_by: &str,
        lease_seconds: i64,
    ) -> StorageResult<Option<KernelSessionLease>> {
        if session_run_id.trim().is_empty() {
            return Err(StorageError::Validation("session_run_id is required"));
        }
        if claimed_by.trim().is_empty() {
            return Err(StorageError::Validation("claimed_by is required"));
        }
        if lease_seconds <= 0 {
            return Err(StorageError::Validation("lease_seconds must be positive"));
        }

        let row = sqlx::query(
            r#"
            UPDATE kernel_session_queue
            SET
                state = 'CLAIMED',
                claimed_by = $2,
                lease_expires_at = CURRENT_TIMESTAMP + ($3::BIGINT * INTERVAL '1 second'),
                attempt_count = attempt_count + 1,
                updated_at = CURRENT_TIMESTAMP
            WHERE session_run_id = $1
                AND available_at <= CURRENT_TIMESTAMP
                AND (
                    state IN ('QUEUED', 'RETRY_SCHEDULED')
                    OR (
                        state IN ('CLAIMED', 'RUNNING')
                        AND lease_expires_at IS NOT NULL
                        AND lease_expires_at <= CURRENT_TIMESTAMP
                    )
                )
            RETURNING
                session_run_id,
                kernel_task_run_id,
                adapter_id,
                state,
                claimed_by,
                lease_expires_at,
                attempt_count,
                created_at,
                updated_at
            "#,
        )
        .bind(session_run_id)
        .bind(claimed_by)
        .bind(lease_seconds)
        .fetch_optional(&self.pool)
        .await?;

        row.map(map_kernel_session_lease).transpose()
    }

    async fn claim_kernel_session_run_and_record_event(
        &self,
        session_run_id: &str,
        claimed_by: &str,
        lease_seconds: i64,
        causation_id: Option<String>,
        correlation_id: String,
    ) -> StorageResult<Option<(KernelSessionLease, KernelEvent)>> {
        if session_run_id.trim().is_empty() {
            return Err(StorageError::Validation("session_run_id is required"));
        }
        if claimed_by.trim().is_empty() {
            return Err(StorageError::Validation("claimed_by is required"));
        }
        if lease_seconds <= 0 {
            return Err(StorageError::Validation("lease_seconds must be positive"));
        }

        let mut tx = self.pool.begin().await?;
        let row = sqlx::query(
            r#"
            UPDATE kernel_session_queue
            SET
                state = 'CLAIMED',
                claimed_by = $2,
                lease_expires_at = CURRENT_TIMESTAMP + ($3::BIGINT * INTERVAL '1 second'),
                attempt_count = attempt_count + 1,
                updated_at = CURRENT_TIMESTAMP
            WHERE session_run_id = $1
                AND available_at <= CURRENT_TIMESTAMP
                AND (
                    state IN ('QUEUED', 'RETRY_SCHEDULED')
                    OR (
                        state IN ('CLAIMED', 'RUNNING')
                        AND lease_expires_at IS NOT NULL
                        AND lease_expires_at <= CURRENT_TIMESTAMP
                    )
                )
            RETURNING
                session_run_id,
                kernel_task_run_id,
                adapter_id,
                state,
                claimed_by,
                lease_expires_at,
                attempt_count,
                created_at,
                updated_at
            "#,
        )
        .bind(session_run_id)
        .bind(claimed_by)
        .bind(lease_seconds)
        .fetch_optional(&mut *tx)
        .await?;

        let Some(row) = row else {
            tx.commit().await?;
            return Ok(None);
        };
        let lease = map_kernel_session_lease(row)?;
        let event = build_kernel_session_event(
            &lease.kernel_task_run_id,
            &lease.session_run_id,
            KernelEventType::SessionClaimed,
            causation_id,
            correlation_id,
            json!({
                "state": lease.state.as_str(),
                "claimed_by": lease.claimed_by.clone(),
                "lease_expires_at": lease.lease_expires_at,
                "attempt_count": lease.attempt_count
            }),
        )?;
        let stored_event = append_kernel_event_with_executor(&mut *tx, event).await?;
        tx.commit().await?;

        Ok(Some((lease, stored_event)))
    }

    async fn update_kernel_session_run_state(
        &self,
        session_run_id: &str,
        state: SessionRunState,
    ) -> StorageResult<KernelSessionLease> {
        if session_run_id.trim().is_empty() {
            return Err(StorageError::Validation("session_run_id is required"));
        }

        let mut tx = self.pool.begin().await?;
        let current = sqlx::query(
            r#"
            SELECT
                session_run_id,
                kernel_task_run_id,
                adapter_id,
                state,
                claimed_by,
                lease_expires_at,
                attempt_count,
                created_at,
                updated_at
            FROM kernel_session_queue
            WHERE session_run_id = $1
            FOR UPDATE
            "#,
        )
        .bind(session_run_id)
        .fetch_optional(&mut *tx)
        .await?;

        let Some(current) = current else {
            return Err(StorageError::NotFound("kernel_session_run"));
        };
        let current = map_kernel_session_lease(current)?;
        if current.state != state && !SessionBroker::can_transition(current.state, state) {
            return Err(StorageError::Validation(
                "invalid kernel session transition",
            ));
        }

        let release_claim = state.is_terminal()
            || matches!(
                state,
                SessionRunState::Queued | SessionRunState::RetryScheduled
            );
        let row = sqlx::query(
            r#"
            UPDATE kernel_session_queue
            SET
                state = $2,
                claimed_by = CASE WHEN $3::BOOLEAN THEN NULL ELSE claimed_by END,
                lease_expires_at = CASE WHEN $3::BOOLEAN THEN NULL ELSE lease_expires_at END,
                available_at = CASE WHEN $2 IN ('RETRY_SCHEDULED', 'BACKPRESSURE_DELAYED') THEN CURRENT_TIMESTAMP ELSE available_at END,
                updated_at = CURRENT_TIMESTAMP
            WHERE session_run_id = $1
            RETURNING
                session_run_id,
                kernel_task_run_id,
                adapter_id,
                state,
                claimed_by,
                lease_expires_at,
                attempt_count,
                created_at,
                updated_at
            "#,
        )
        .bind(session_run_id)
        .bind(state.as_str())
        .bind(release_claim)
        .fetch_one(&mut *tx)
        .await?;

        tx.commit().await?;
        map_kernel_session_lease(row)
    }

    async fn update_kernel_session_run_state_and_record_event(
        &self,
        session_run_id: &str,
        state: SessionRunState,
        causation_id: Option<String>,
        correlation_id: String,
    ) -> StorageResult<(KernelSessionLease, KernelEvent)> {
        if session_run_id.trim().is_empty() {
            return Err(StorageError::Validation("session_run_id is required"));
        }

        let mut tx = self.pool.begin().await?;
        let current = sqlx::query(
            r#"
            SELECT
                session_run_id,
                kernel_task_run_id,
                adapter_id,
                state,
                claimed_by,
                lease_expires_at,
                attempt_count,
                created_at,
                updated_at
            FROM kernel_session_queue
            WHERE session_run_id = $1
            FOR UPDATE
            "#,
        )
        .bind(session_run_id)
        .fetch_optional(&mut *tx)
        .await?;

        let Some(current) = current else {
            return Err(StorageError::NotFound("kernel_session_run"));
        };
        let current = map_kernel_session_lease(current)?;
        if current.state != state && !SessionBroker::can_transition(current.state, state) {
            return Err(StorageError::Validation(
                "invalid kernel session transition",
            ));
        }

        let release_claim = state.is_terminal()
            || matches!(
                state,
                SessionRunState::Queued | SessionRunState::RetryScheduled
            );
        let row = sqlx::query(
            r#"
            UPDATE kernel_session_queue
            SET
                state = $2,
                claimed_by = CASE WHEN $3::BOOLEAN THEN NULL ELSE claimed_by END,
                lease_expires_at = CASE WHEN $3::BOOLEAN THEN NULL ELSE lease_expires_at END,
                available_at = CASE WHEN $2 IN ('RETRY_SCHEDULED', 'BACKPRESSURE_DELAYED') THEN CURRENT_TIMESTAMP ELSE available_at END,
                updated_at = CURRENT_TIMESTAMP
            WHERE session_run_id = $1
            RETURNING
                session_run_id,
                kernel_task_run_id,
                adapter_id,
                state,
                claimed_by,
                lease_expires_at,
                attempt_count,
                created_at,
                updated_at
            "#,
        )
        .bind(session_run_id)
        .bind(state.as_str())
        .bind(release_claim)
        .fetch_one(&mut *tx)
        .await?;

        let lease = map_kernel_session_lease(row)?;
        let event_type = if current.state == state {
            session_state_event_type(state)
        } else {
            SessionBroker::transition_event_type(current.state, state)
                .map_err(|err| StorageError::Serialization(err.to_string()))?
        };
        let event = build_kernel_session_event(
            &lease.kernel_task_run_id,
            &lease.session_run_id,
            event_type,
            causation_id,
            correlation_id,
            json!({"state": lease.state.as_str()}),
        )?;
        let stored_event = append_kernel_event_with_executor(&mut *tx, event).await?;
        tx.commit().await?;

        Ok((lease, stored_event))
    }

    async fn update_ai_job_mcp_fields(
        &self,
        job_id: Uuid,
        update: super::AiJobMcpUpdate,
    ) -> StorageResult<()> {
        let now = Utc::now();
        let job_id = job_id.to_string();
        let super::AiJobMcpUpdate {
            mcp_server_id,
            mcp_call_id,
            mcp_progress_token,
        } = update;

        let mut tx = self.pool.begin().await?;

        let exists =
            sqlx::query_scalar::<_, bool>("SELECT EXISTS(SELECT 1 FROM ai_jobs WHERE id = $1)")
                .bind(&job_id)
                .fetch_one(&mut *tx)
                .await?;
        if !exists {
            return Err(StorageError::NotFound("ai_job"));
        }

        let upsert = sqlx::query(
            r#"
            INSERT INTO ai_job_mcp_fields (job_id, mcp_server_id, mcp_call_id, mcp_progress_token)
            VALUES ($1, $2, $3, $4)
            ON CONFLICT (job_id) DO UPDATE SET
                mcp_server_id = COALESCE(excluded.mcp_server_id, ai_job_mcp_fields.mcp_server_id),
                mcp_call_id = COALESCE(excluded.mcp_call_id, ai_job_mcp_fields.mcp_call_id),
                mcp_progress_token = COALESCE(excluded.mcp_progress_token, ai_job_mcp_fields.mcp_progress_token)
            "#,
        )
        .bind(&job_id)
        .bind(mcp_server_id)
        .bind(mcp_call_id)
        .bind(mcp_progress_token)
        .execute(&mut *tx)
        .await;

        match upsert {
            Ok(_) => {}
            Err(e) if is_pg_unique_violation(&e) => {
                return Err(StorageError::Conflict("mcp_progress_token already mapped"));
            }
            Err(e) if is_pg_foreign_key_violation(&e) => {
                return Err(StorageError::NotFound("ai_job"));
            }
            Err(e) => return Err(e.into()),
        }

        sqlx::query("UPDATE ai_jobs SET updated_at = $1 WHERE id = $2")
            .bind(now)
            .bind(&job_id)
            .execute(&mut *tx)
            .await?;

        tx.commit().await?;
        Ok(())
    }

    async fn get_ai_job_mcp_fields(&self, job_id: Uuid) -> StorageResult<super::AiJobMcpFields> {
        let job_id = job_id.to_string();

        let exists =
            sqlx::query_scalar::<_, bool>("SELECT EXISTS(SELECT 1 FROM ai_jobs WHERE id = $1)")
                .bind(&job_id)
                .fetch_one(&self.pool)
                .await?;
        if !exists {
            return Err(StorageError::NotFound("ai_job"));
        }

        let row = sqlx::query(
            r#"
            SELECT mcp_server_id, mcp_call_id, mcp_progress_token
            FROM ai_job_mcp_fields
            WHERE job_id = $1
            "#,
        )
        .bind(&job_id)
        .fetch_optional(&self.pool)
        .await?;

        let Some(row) = row else {
            return Ok(super::AiJobMcpFields::default());
        };

        Ok(super::AiJobMcpFields {
            mcp_server_id: row.get("mcp_server_id"),
            mcp_call_id: row.get("mcp_call_id"),
            mcp_progress_token: row.get("mcp_progress_token"),
        })
    }

    async fn find_ai_job_id_by_mcp_progress_token(
        &self,
        progress_token: &str,
    ) -> StorageResult<Option<Uuid>> {
        let id: Option<String> = sqlx::query_scalar(
            r#"
            SELECT job_id
            FROM ai_job_mcp_fields
            WHERE mcp_progress_token = $1
            LIMIT 1
            "#,
        )
        .bind(progress_token)
        .fetch_optional(&self.pool)
        .await?;

        id.map(|id| {
            Uuid::parse_str(&id).map_err(|_| StorageError::Validation("invalid job_id uuid"))
        })
        .transpose()
    }

    async fn create_workflow_run(
        &self,
        job_id: Uuid,
        status: JobState,
        last_heartbeat: Option<chrono::DateTime<chrono::Utc>>,
    ) -> StorageResult<WorkflowRun> {
        let id = Uuid::now_v7();
        let heartbeat = last_heartbeat.unwrap_or_else(Utc::now);
        let now = Utc::now();

        let row = sqlx::query(
            r#"
            INSERT INTO workflow_runs (id, job_id, status, last_heartbeat, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6)
            RETURNING id, job_id, status, last_heartbeat, created_at, updated_at
            "#,
        )
        .bind(id.to_string())
        .bind(job_id.to_string())
        .bind(status.as_str())
        .bind(heartbeat)
        .bind(now)
        .bind(now)
        .fetch_one(&self.pool)
        .await?;

        map_workflow_run(row)
    }

    async fn update_workflow_run_status(
        &self,
        run_id: Uuid,
        status: JobState,
        error_message: Option<String>,
    ) -> StorageResult<WorkflowRun> {
        let now = Utc::now();
        let row = sqlx::query(
            r#"
            UPDATE workflow_runs
            SET status = $1,
                updated_at = $2
            WHERE id = $3
            RETURNING id, job_id, status, last_heartbeat, created_at, updated_at
            "#,
        )
        .bind(status.as_str())
        .bind(now)
        .bind(run_id.to_string())
        .fetch_optional(&self.pool)
        .await?;

        let row = row.ok_or(StorageError::NotFound("workflow_run"))?;

        if let Some(message) = error_message.clone() {
            sqlx::query(
                r#"
                UPDATE ai_jobs
                SET error_message = $1,
                    updated_at = $2
                WHERE id = (SELECT job_id FROM workflow_runs WHERE id = $3)
                "#,
            )
            .bind(&message)
            .bind(now)
            .bind(run_id.to_string())
            .execute(&self.pool)
            .await?;
        }

        map_workflow_run(row)
    }

    async fn heartbeat_workflow(
        &self,
        run_id: Uuid,
        at: chrono::DateTime<chrono::Utc>,
    ) -> StorageResult<()> {
        sqlx::query(
            r#"
            UPDATE workflow_runs
            SET last_heartbeat = $1,
                updated_at = $1
            WHERE id = $2
            "#,
        )
        .bind(at)
        .bind(run_id.to_string())
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn create_workflow_node_execution(
        &self,
        exec: NewNodeExecution,
    ) -> StorageResult<WorkflowNodeExecution> {
        let id = Uuid::now_v7();
        let input_payload = exec.input_payload.as_ref().map(|v| v.to_string());
        let row = sqlx::query(
            r#"
            INSERT INTO workflow_node_executions (
                id, workflow_run_id, node_id, node_type, status, sequence, input_payload, started_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            RETURNING
                id, workflow_run_id, node_id, node_type, status, sequence, input_payload,
                output_payload, error_message, started_at, finished_at, created_at, updated_at
            "#,
        )
        .bind(id.to_string())
        .bind(exec.workflow_run_id.to_string())
        .bind(exec.node_id)
        .bind(exec.node_type)
        .bind(exec.status.as_str())
        .bind(exec.sequence)
        .bind(input_payload)
        .bind(exec.started_at)
        .fetch_one(&self.pool)
        .await?;

        map_workflow_node_execution(row)
    }

    async fn update_workflow_node_execution_status(
        &self,
        exec_id: Uuid,
        status: JobState,
        output: Option<Value>,
        error_message: Option<String>,
    ) -> StorageResult<WorkflowNodeExecution> {
        let now = Utc::now();
        let output_payload = output.as_ref().map(|v| v.to_string());
        let row = sqlx::query(
            r#"
            UPDATE workflow_node_executions
            SET status = $1,
                output_payload = COALESCE($2, output_payload),
                error_message = COALESCE($3, error_message),
                finished_at = CASE WHEN $1 IN ('completed','completed_with_issues','failed','cancelled','stalled','poisoned') THEN $4 ELSE finished_at END,
                updated_at = $4
            WHERE id = $5
            RETURNING
                id, workflow_run_id, node_id, node_type, status, sequence, input_payload,
                output_payload, error_message, started_at, finished_at, created_at, updated_at
            "#,
        )
        .bind(status.as_str())
        .bind(output_payload)
        .bind(error_message)
        .bind(now)
        .bind(exec_id.to_string())
        .fetch_optional(&self.pool)
        .await?;

        let row = row.ok_or(StorageError::NotFound("workflow_node_execution"))?;
        map_workflow_node_execution(row)
    }

    async fn list_workflow_node_executions(
        &self,
        run_id: Uuid,
    ) -> StorageResult<Vec<WorkflowNodeExecution>> {
        let rows = sqlx::query(
            r#"
            SELECT
                id, workflow_run_id, node_id, node_type, status, sequence,
                input_payload, output_payload, error_message, started_at,
                finished_at, created_at, updated_at
            FROM workflow_node_executions
            WHERE workflow_run_id = $1
            ORDER BY sequence ASC
            "#,
        )
        .bind(run_id.to_string())
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter().map(map_workflow_node_execution).collect()
    }

    async fn find_stalled_workflows(&self, threshold_secs: u64) -> StorageResult<Vec<WorkflowRun>> {
        let cutoff = Utc::now() - chrono::Duration::seconds(threshold_secs as i64);
        let rows = sqlx::query(
            r#"
            SELECT id, job_id, status, last_heartbeat, created_at, updated_at
            FROM workflow_runs
            WHERE status = 'running'
              AND last_heartbeat < $1
            "#,
        )
        .bind(cutoff)
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter().map(map_workflow_run).collect()
    }

    async fn validate_write_with_guard(
        &self,
        ctx: &WriteContext,
        resource_id: &str,
    ) -> StorageResult<MutationMetadata> {
        let metadata = self.guard.validate_write(ctx, resource_id).await?;
        Ok(metadata)
    }

    async fn prune_ai_jobs(
        &self,
        cutoff: chrono::DateTime<chrono::Utc>,
        min_versions: u32,
        dry_run: bool,
    ) -> StorageResult<super::PruneReport> {
        let mut report = super::PruneReport::new();

        let scan_row = sqlx::query(
            r#"
            SELECT
                COUNT(*) as total,
                COALESCE(SUM(CASE WHEN is_pinned = 1 THEN 1 ELSE 0 END), 0) as pinned
            FROM ai_jobs
            WHERE status IN ('completed', 'failed')
              AND created_at < $1
            "#,
        )
        .bind(cutoff)
        .fetch_one(&self.pool)
        .await?;

        let total_eligible: i64 = scan_row.get("total");
        let pinned_count: i64 = scan_row.get("pinned");

        let total_eligible = total_eligible.max(0) as u32;
        let pinned_count = pinned_count.max(0) as u32;
        let deletable_count = total_eligible.saturating_sub(pinned_count);

        report.items_scanned += total_eligible;
        report.items_spared_pinned += pinned_count;

        let non_pinned_row = sqlx::query(
            r#"
            SELECT COUNT(*) as count
            FROM ai_jobs
            WHERE is_pinned = 0
              AND status IN ('completed', 'failed')
            "#,
        )
        .fetch_one(&self.pool)
        .await?;

        let total_non_pinned: i64 = non_pinned_row.get("count");
        let total_non_pinned = total_non_pinned.max(0) as u32;

        let max_deletable = total_non_pinned.saturating_sub(min_versions);
        let actual_to_delete = deletable_count.min(max_deletable);

        if actual_to_delete == 0 {
            report.items_spared_window += deletable_count;
            return Ok(report);
        }

        if dry_run {
            report.items_pruned += actual_to_delete;
            report.items_spared_window += deletable_count.saturating_sub(actual_to_delete);
            return Ok(report);
        }

        let mut deleted = 0u32;
        let batch_size = 1000i64;

        while deleted < actual_to_delete {
            let remaining = (actual_to_delete - deleted) as i64;
            let limit = remaining.min(batch_size);

            let result = sqlx::query(
                r#"
                DELETE FROM ai_jobs
                WHERE id IN (
                    SELECT id FROM ai_jobs
                    WHERE status IN ('completed', 'failed')
                      AND created_at < $1
                      AND is_pinned = 0
                    ORDER BY created_at ASC
                    LIMIT $2
                )
                "#,
            )
            .bind(cutoff)
            .bind(limit)
            .execute(&self.pool)
            .await?;

            let batch_deleted = result.rows_affected() as u32;
            if batch_deleted == 0 {
                break;
            }
            deleted += batch_deleted;
        }

        report.items_pruned += deleted;
        report.items_spared_window += deletable_count.saturating_sub(deleted);
        Ok(report)
    }
}
