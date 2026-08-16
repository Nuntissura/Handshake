//! `Database` implementation backed by the Handshake-native embedded SurrealDB
//! store.
//!
//! Every method here is either a real SurrealDB implementation or returns the
//! trait's own `StorageError::NotImplemented` contract, which fails closed and
//! is the same idiom the trait uses for its 93 default methods. A
//! `NotImplemented` arm is NOT a completed surface: no microtask may be closed
//! while a behaviour it claims still resolves to one. The remaining surface is
//! enumerated mechanically by `not_implemented_surface_is_declared` so it can
//! never be silently forgotten.

use async_trait::async_trait;

use super::SurrealStorage;
use crate::storage::{Database, StorageError};

#[allow(unused_imports)]
use crate::storage::*;

/// Embedded-SurrealDB control-plane database.
#[derive(Clone)]
pub struct SurrealDatabase {
    storage: SurrealStorage,
}

impl SurrealDatabase {
    pub fn new(storage: SurrealStorage) -> Self {
        Self { storage }
    }

    /// Borrow the embedded store for domain implementations.
    pub fn storage(&self) -> &SurrealStorage {
        &self.storage
    }
}

#[async_trait]
impl Database for SurrealDatabase {
    fn loom_search_observability_tier(&self) -> u8 {
        // Tier 0 until the SurrealDB loom search backend lands (see loom wave).
        0
    }

    fn loom_traverse_graph_perf_target_ms(&self) -> u128 {
        // Mirrors the trait contract default target; revisited when the
        // SurrealDB graph traversal backend is implemented.
        250
    }

    async fn ping(&self) -> StorageResult<()> {
        Err(StorageError::NotImplemented("surreal core backend"))
    }

    async fn list_workspaces(&self) -> StorageResult<Vec<Workspace>> {
        Err(StorageError::NotImplemented("surreal workspace backend"))
    }

    async fn create_workspace(
        &self,
        ctx: &WriteContext,
        workspace: NewWorkspace,
    ) -> StorageResult<Workspace> {
        Err(StorageError::NotImplemented("surreal workspace backend"))
    }

    async fn delete_workspace(&self, ctx: &WriteContext, id: &str) -> StorageResult<()> {
        Err(StorageError::NotImplemented("surreal workspace backend"))
    }

    async fn get_workspace(&self, id: &str) -> StorageResult<Option<Workspace>> {
        Err(StorageError::NotImplemented("surreal workspace backend"))
    }

    async fn list_documents(&self, workspace_id: &str) -> StorageResult<Vec<Document>> {
        Err(StorageError::NotImplemented("surreal document backend"))
    }

    async fn get_document(&self, doc_id: &str) -> StorageResult<Document> {
        Err(StorageError::NotImplemented("surreal document backend"))
    }

    async fn create_document(
        &self,
        ctx: &WriteContext,
        doc: NewDocument,
    ) -> StorageResult<Document> {
        Err(StorageError::NotImplemented("surreal document backend"))
    }

    async fn delete_document(&self, ctx: &WriteContext, doc_id: &str) -> StorageResult<()> {
        Err(StorageError::NotImplemented("surreal document backend"))
    }

    async fn get_blocks(&self, doc_id: &str) -> StorageResult<Vec<Block>> {
        Err(StorageError::NotImplemented("surreal block backend"))
    }

    async fn get_block(&self, block_id: &str) -> StorageResult<Block> {
        Err(StorageError::NotImplemented("surreal block backend"))
    }

    async fn create_block(&self, ctx: &WriteContext, block: NewBlock) -> StorageResult<Block> {
        Err(StorageError::NotImplemented("surreal block backend"))
    }

    async fn update_block(
        &self,
        ctx: &WriteContext,
        block_id: &str,
        data: BlockUpdate,
    ) -> StorageResult<()> {
        Err(StorageError::NotImplemented("surreal block backend"))
    }

    async fn delete_block(&self, ctx: &WriteContext, block_id: &str) -> StorageResult<()> {
        Err(StorageError::NotImplemented("surreal block backend"))
    }

    async fn replace_blocks(
        &self,
        ctx: &WriteContext,
        document_id: &str,
        blocks: Vec<NewBlock>,
    ) -> StorageResult<Vec<Block>> {
        Err(StorageError::NotImplemented("surreal block backend"))
    }

    async fn create_asset(&self, ctx: &WriteContext, asset: NewAsset) -> StorageResult<Asset> {
        Err(StorageError::NotImplemented("surreal asset backend"))
    }

    async fn get_asset(&self, workspace_id: &str, asset_id: &str) -> StorageResult<Asset> {
        Err(StorageError::NotImplemented("surreal asset backend"))
    }

    async fn find_asset_by_content_hash(
        &self,
        workspace_id: &str,
        content_hash: &str,
    ) -> StorageResult<Option<Asset>> {
        Err(StorageError::NotImplemented("surreal asset backend"))
    }

    async fn upsert_media_tier(
        &self,
        ctx: &WriteContext,
        upsert: MediaTierUpsert,
    ) -> StorageResult<MediaAssetTier> {
        Err(StorageError::NotImplemented("surreal media tier backend"))
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
        Err(StorageError::NotImplemented("surreal media tier backend"))
    }

    async fn get_media_tier(
        &self,
        workspace_id: &str,
        asset_id: &str,
        tier: MediaTier,
    ) -> StorageResult<Option<MediaAssetTier>> {
        Err(StorageError::NotImplemented("surreal media tier backend"))
    }

    async fn list_media_tiers(
        &self,
        workspace_id: &str,
        asset_id: &str,
    ) -> StorageResult<Vec<MediaAssetTier>> {
        Err(StorageError::NotImplemented("surreal media tier backend"))
    }

    async fn list_failed_media_tiers(
        &self,
        workspace_id: &str,
    ) -> StorageResult<Vec<MediaAssetTier>> {
        Err(StorageError::NotImplemented("surreal media tier backend"))
    }

    async fn delete_media_tiers(
        &self,
        ctx: &WriteContext,
        workspace_id: &str,
        asset_id: &str,
    ) -> StorageResult<u64> {
        Err(StorageError::NotImplemented("surreal media tier backend"))
    }

    async fn create_loom_collection(
        &self,
        ctx: &WriteContext,
        workspace_id: &str,
        title: Option<String>,
    ) -> StorageResult<LoomCollection> {
        Err(StorageError::NotImplemented("surreal loom backend"))
    }

    async fn get_loom_collection(
        &self,
        workspace_id: &str,
        collection_id: &str,
    ) -> StorageResult<LoomCollectionWithMembers> {
        Err(StorageError::NotImplemented("surreal loom backend"))
    }

    async fn set_loom_collection_order(
        &self,
        ctx: &WriteContext,
        workspace_id: &str,
        collection_id: &str,
        asset_ids: &[String],
    ) -> StorageResult<LoomCollectionWithMembers> {
        Err(StorageError::NotImplemented("surreal loom backend"))
    }

    async fn create_loom_block(
        &self,
        ctx: &WriteContext,
        block: NewLoomBlock,
    ) -> StorageResult<LoomBlock> {
        Err(StorageError::NotImplemented("surreal loom backend"))
    }

    async fn get_or_create_daily_journal_block(
        &self,
        ctx: &WriteContext,
        workspace_id: &str,
        journal_date: &str,
    ) -> StorageResult<LoomBlock> {
        Err(StorageError::NotImplemented("surreal block backend"))
    }

    async fn get_loom_block(&self, workspace_id: &str, block_id: &str) -> StorageResult<LoomBlock> {
        Err(StorageError::NotImplemented("surreal loom backend"))
    }

    async fn find_loom_block_by_content_hash(
        &self,
        workspace_id: &str,
        content_hash: &str,
    ) -> StorageResult<Option<LoomBlock>> {
        Err(StorageError::NotImplemented("surreal loom backend"))
    }

    async fn find_loom_block_by_asset_id(
        &self,
        workspace_id: &str,
        asset_id: &str,
    ) -> StorageResult<Option<LoomBlock>> {
        Err(StorageError::NotImplemented("surreal loom backend"))
    }

    async fn update_loom_block(
        &self,
        ctx: &WriteContext,
        workspace_id: &str,
        block_id: &str,
        update: LoomBlockUpdate,
    ) -> StorageResult<LoomBlock> {
        Err(StorageError::NotImplemented("surreal loom backend"))
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
        Err(StorageError::NotImplemented("surreal loom backend"))
    }

    async fn delete_loom_block(
        &self,
        ctx: &WriteContext,
        workspace_id: &str,
        block_id: &str,
    ) -> StorageResult<()> {
        Err(StorageError::NotImplemented("surreal loom backend"))
    }

    async fn create_loom_edge(
        &self,
        ctx: &WriteContext,
        edge: NewLoomEdge,
    ) -> StorageResult<LoomEdge> {
        Err(StorageError::NotImplemented("surreal loom backend"))
    }

    async fn delete_loom_edge(
        &self,
        ctx: &WriteContext,
        workspace_id: &str,
        edge_id: &str,
    ) -> StorageResult<LoomEdge> {
        Err(StorageError::NotImplemented("surreal loom backend"))
    }

    async fn list_loom_edges_for_block(
        &self,
        workspace_id: &str,
        block_id: &str,
    ) -> StorageResult<Vec<LoomEdge>> {
        Err(StorageError::NotImplemented("surreal loom backend"))
    }

    async fn get_backlinks(
        &self,
        workspace_id: &str,
        block_id: &str,
    ) -> StorageResult<Vec<LoomEdge>> {
        Err(StorageError::NotImplemented("surreal core backend"))
    }

    async fn get_outgoing_edges(
        &self,
        workspace_id: &str,
        block_id: &str,
    ) -> StorageResult<Vec<LoomEdge>> {
        Err(StorageError::NotImplemented("surreal core backend"))
    }

    async fn traverse_graph(
        &self,
        workspace_id: &str,
        start_block_id: &str,
        max_depth: u32,
        edge_types: &[LoomEdgeType],
    ) -> StorageResult<Vec<(LoomBlock, u32)>> {
        Err(StorageError::NotImplemented("surreal graph backend"))
    }

    async fn recompute_block_metrics(
        &self,
        workspace_id: &str,
        block_id: &str,
    ) -> StorageResult<()> {
        Err(StorageError::NotImplemented("surreal block backend"))
    }

    async fn recompute_all_metrics(&self, workspace_id: &str) -> StorageResult<()> {
        Err(StorageError::NotImplemented("surreal core backend"))
    }

    async fn query_loom_view(
        &self,
        workspace_id: &str,
        view_type: LoomViewType,
        filters: LoomViewFilters,
        limit: u32,
        offset: u32,
    ) -> StorageResult<LoomViewResponse> {
        Err(StorageError::NotImplemented("surreal loom backend"))
    }

    async fn search_loom_blocks(
        &self,
        workspace_id: &str,
        query: &str,
        filters: LoomSearchFilters,
        limit: u32,
        offset: u32,
    ) -> StorageResult<Vec<LoomBlockSearchResult>> {
        Err(StorageError::NotImplemented("surreal loom backend"))
    }

    async fn search_loom_graph(
        &self,
        workspace_id: &str,
        query: &str,
        filters: LoomSearchFilters,
        limit: u32,
        offset: u32,
    ) -> StorageResult<Vec<LoomGraphSearchResult>> {
        Err(StorageError::NotImplemented("surreal loom backend"))
    }

    async fn upsert_calendar_source(
        &self,
        ctx: &WriteContext,
        source: CalendarSourceUpsert,
    ) -> StorageResult<CalendarSource> {
        Err(StorageError::NotImplemented("surreal calendar backend"))
    }

    async fn list_calendar_sources(
        &self,
        workspace_id: &str,
    ) -> StorageResult<Vec<CalendarSource>> {
        Err(StorageError::NotImplemented("surreal calendar backend"))
    }

    async fn get_calendar_source(
        &self,
        workspace_id: &str,
        source_id: &str,
    ) -> StorageResult<Option<CalendarSource>> {
        Err(StorageError::NotImplemented("surreal calendar backend"))
    }

    async fn upsert_calendar_event(
        &self,
        ctx: &WriteContext,
        event: CalendarEventUpsert,
    ) -> StorageResult<CalendarEvent> {
        Err(StorageError::NotImplemented("surreal calendar backend"))
    }

    async fn query_calendar_events(
        &self,
        query: CalendarEventWindowQuery,
    ) -> StorageResult<Vec<CalendarEvent>> {
        Err(StorageError::NotImplemented("surreal calendar backend"))
    }

    async fn delete_calendar_data_by_source(
        &self,
        ctx: &WriteContext,
        workspace_id: &str,
        source_id: &str,
    ) -> StorageResult<()> {
        Err(StorageError::NotImplemented("surreal calendar backend"))
    }

    async fn create_canvas(&self, ctx: &WriteContext, canvas: NewCanvas) -> StorageResult<Canvas> {
        Err(StorageError::NotImplemented("surreal canvas backend"))
    }

    async fn list_canvases(&self, workspace_id: &str) -> StorageResult<Vec<Canvas>> {
        Err(StorageError::NotImplemented("surreal canvas backend"))
    }

    async fn get_canvas_with_graph(&self, canvas_id: &str) -> StorageResult<CanvasGraph> {
        Err(StorageError::NotImplemented("surreal graph backend"))
    }

    async fn rename_canvas(
        &self,
        ctx: &WriteContext,
        canvas_id: &str,
        title: &str,
        expected_updated_at: Option<DateTime<Utc>>,
    ) -> StorageResult<Canvas> {
        Err(StorageError::NotImplemented("surreal canvas backend"))
    }

    async fn update_canvas_graph(
        &self,
        ctx: &WriteContext,
        canvas_id: &str,
        nodes: Vec<NewCanvasNode>,
        edges: Vec<NewCanvasEdge>,
    ) -> StorageResult<CanvasGraph> {
        Err(StorageError::NotImplemented("surreal graph backend"))
    }

    async fn delete_canvas(&self, ctx: &WriteContext, canvas_id: &str) -> StorageResult<()> {
        Err(StorageError::NotImplemented("surreal canvas backend"))
    }

    async fn create_ai_bronze_record(
        &self,
        ctx: &WriteContext,
        record: NewBronzeRecord,
    ) -> StorageResult<BronzeRecord> {
        Err(StorageError::NotImplemented("surreal ai backend"))
    }

    async fn get_ai_bronze_record(&self, bronze_id: &str) -> StorageResult<Option<BronzeRecord>> {
        Err(StorageError::NotImplemented("surreal ai backend"))
    }

    async fn list_ai_bronze_records(&self, workspace_id: &str) -> StorageResult<Vec<BronzeRecord>> {
        Err(StorageError::NotImplemented("surreal ai backend"))
    }

    async fn mark_ai_bronze_deleted(
        &self,
        ctx: &WriteContext,
        bronze_id: &str,
    ) -> StorageResult<()> {
        Err(StorageError::NotImplemented("surreal ai backend"))
    }

    async fn create_ai_silver_record(
        &self,
        ctx: &WriteContext,
        record: NewSilverRecord,
    ) -> StorageResult<SilverRecord> {
        Err(StorageError::NotImplemented("surreal ai backend"))
    }

    async fn get_ai_silver_record(&self, silver_id: &str) -> StorageResult<Option<SilverRecord>> {
        Err(StorageError::NotImplemented("surreal ai backend"))
    }

    async fn list_ai_silver_records_by_bronze(
        &self,
        bronze_id: &str,
    ) -> StorageResult<Vec<SilverRecord>> {
        Err(StorageError::NotImplemented("surreal ai backend"))
    }

    async fn list_ai_silver_records(&self, workspace_id: &str) -> StorageResult<Vec<SilverRecord>> {
        Err(StorageError::NotImplemented("surreal ai backend"))
    }

    async fn supersede_ai_silver_record(
        &self,
        ctx: &WriteContext,
        superseded_silver_id: &str,
        new_silver_id: &str,
    ) -> StorageResult<()> {
        Err(StorageError::NotImplemented("surreal ai backend"))
    }

    async fn upsert_ai_embedding_model(
        &self,
        ctx: &WriteContext,
        model: EmbeddingModelRecord,
    ) -> StorageResult<()> {
        Err(StorageError::NotImplemented("surreal ai backend"))
    }

    async fn list_ai_embedding_models(&self) -> StorageResult<Vec<EmbeddingModelRecord>> {
        Err(StorageError::NotImplemented("surreal ai backend"))
    }

    async fn set_ai_embedding_default_model(
        &self,
        ctx: &WriteContext,
        model_id: &str,
        model_version: &str,
    ) -> StorageResult<()> {
        Err(StorageError::NotImplemented("surreal ai backend"))
    }

    async fn get_ai_embedding_registry(&self) -> StorageResult<Option<EmbeddingRegistry>> {
        Err(StorageError::NotImplemented("surreal ai backend"))
    }

    async fn get_ai_job(&self, job_id: &str) -> StorageResult<AiJob> {
        Err(StorageError::NotImplemented("surreal ai backend"))
    }

    async fn list_ai_jobs(&self, filter: AiJobListFilter) -> StorageResult<Vec<AiJob>> {
        Err(StorageError::NotImplemented("surreal ai backend"))
    }

    async fn create_ai_job(&self, job: NewAiJob) -> StorageResult<AiJob> {
        Err(StorageError::NotImplemented("surreal ai backend"))
    }

    async fn update_ai_job_status(&self, update: JobStatusUpdate) -> StorageResult<AiJob> {
        Err(StorageError::NotImplemented("surreal ai backend"))
    }

    async fn set_job_outputs(&self, job_id: &str, outputs: Option<Value>) -> StorageResult<()> {
        Err(StorageError::NotImplemented("surreal job backend"))
    }

    async fn upsert_model_session(&self, session: NewModelSession) -> StorageResult<ModelSession> {
        Err(StorageError::NotImplemented("surreal session backend"))
    }

    async fn get_model_session(&self, session_id: &str) -> StorageResult<ModelSession> {
        Err(StorageError::NotImplemented("surreal session backend"))
    }

    async fn get_model_session_by_job_id(&self, job_id: Uuid) -> StorageResult<ModelSession> {
        Err(StorageError::NotImplemented("surreal session backend"))
    }

    async fn update_model_session_state_with_merge_back_artifact(
        &self,
        session_id: &str,
        state: ModelSessionState,
        job_id: Option<Uuid>,
        merge_back_artifact: Option<MergeBackArtifact>,
    ) -> StorageResult<ModelSession> {
        Err(StorageError::NotImplemented("surreal session backend"))
    }

    async fn close_model_session(
        &self,
        session_id: &str,
        state: ModelSessionState,
        close_reason: &str,
        actor: &str,
    ) -> StorageResult<ModelSession> {
        Err(StorageError::NotImplemented("surreal session backend"))
    }

    async fn create_session_checkpoint(
        &self,
        checkpoint: SessionCheckpoint,
    ) -> StorageResult<SessionCheckpoint> {
        Err(StorageError::NotImplemented("surreal session backend"))
    }

    async fn get_latest_session_checkpoint(
        &self,
        session_id: &str,
    ) -> StorageResult<SessionCheckpoint> {
        Err(StorageError::NotImplemented("surreal session backend"))
    }

    async fn append_session_message(
        &self,
        message: NewSessionMessage,
    ) -> StorageResult<SessionMessage> {
        Err(StorageError::NotImplemented("surreal session backend"))
    }

    async fn list_session_messages(&self, session_id: &str) -> StorageResult<Vec<SessionMessage>> {
        Err(StorageError::NotImplemented("surreal session backend"))
    }

    async fn append_kernel_event(&self, event: NewKernelEvent) -> StorageResult<KernelEvent> {
        Err(StorageError::NotImplemented("surreal kernel backend"))
    }

    async fn list_kernel_events_for_session(
        &self,
        session_run_id: &str,
    ) -> StorageResult<Vec<KernelEvent>> {
        Err(StorageError::NotImplemented("surreal kernel backend"))
    }

    async fn list_kernel_events_for_aggregate(
        &self,
        aggregate_type: &str,
        aggregate_id: &str,
    ) -> StorageResult<Vec<KernelEvent>> {
        Err(StorageError::NotImplemented("surreal kernel backend"))
    }

    async fn list_pending_native_editor_mirrors(
        &self,
        after_event_sequence: i64,
        limit: i64,
    ) -> StorageResult<Vec<KernelEvent>> {
        Err(StorageError::NotImplemented("surreal core backend"))
    }

    async fn enqueue_kernel_session_run(&self, session: SessionRun) -> StorageResult<SessionRun> {
        Err(StorageError::NotImplemented("surreal kernel backend"))
    }

    async fn enqueue_kernel_session_run_and_record_event(
        &self,
        session: SessionRun,
        causation_id: Option<String>,
        correlation_id: String,
    ) -> StorageResult<(SessionRun, KernelEvent)> {
        Err(StorageError::NotImplemented("surreal kernel backend"))
    }

    async fn claim_kernel_session_run(
        &self,
        session_run_id: &str,
        claimed_by: &str,
        lease_seconds: i64,
    ) -> StorageResult<Option<KernelSessionLease>> {
        Err(StorageError::NotImplemented("surreal kernel backend"))
    }

    async fn claim_kernel_session_run_and_record_event(
        &self,
        session_run_id: &str,
        claimed_by: &str,
        lease_seconds: i64,
        causation_id: Option<String>,
        correlation_id: String,
    ) -> StorageResult<Option<(KernelSessionLease, KernelEvent)>> {
        Err(StorageError::NotImplemented("surreal kernel backend"))
    }

    async fn update_kernel_session_run_state(
        &self,
        session_run_id: &str,
        state: SessionRunState,
    ) -> StorageResult<KernelSessionLease> {
        Err(StorageError::NotImplemented("surreal kernel backend"))
    }

    async fn update_kernel_session_run_state_and_record_event(
        &self,
        session_run_id: &str,
        state: SessionRunState,
        causation_id: Option<String>,
        correlation_id: String,
    ) -> StorageResult<(KernelSessionLease, KernelEvent)> {
        Err(StorageError::NotImplemented("surreal kernel backend"))
    }

    async fn create_workflow_run(
        &self,
        job_id: Uuid,
        status: JobState,
        last_heartbeat: Option<DateTime<Utc>>,
    ) -> StorageResult<WorkflowRun> {
        Err(StorageError::NotImplemented("surreal workflow backend"))
    }

    async fn update_workflow_run_status(
        &self,
        run_id: Uuid,
        status: JobState,
        error_message: Option<String>,
    ) -> StorageResult<WorkflowRun> {
        Err(StorageError::NotImplemented("surreal workflow backend"))
    }

    async fn heartbeat_workflow(&self, run_id: Uuid, at: DateTime<Utc>) -> StorageResult<()> {
        Err(StorageError::NotImplemented("surreal workflow backend"))
    }

    async fn create_workflow_node_execution(
        &self,
        exec: NewNodeExecution,
    ) -> StorageResult<WorkflowNodeExecution> {
        Err(StorageError::NotImplemented("surreal workflow backend"))
    }

    async fn update_workflow_node_execution_status(
        &self,
        exec_id: Uuid,
        status: JobState,
        output: Option<Value>,
        error_message: Option<String>,
    ) -> StorageResult<WorkflowNodeExecution> {
        Err(StorageError::NotImplemented("surreal workflow backend"))
    }

    async fn list_workflow_node_executions(
        &self,
        run_id: Uuid,
    ) -> StorageResult<Vec<WorkflowNodeExecution>> {
        Err(StorageError::NotImplemented("surreal workflow backend"))
    }

    async fn find_stalled_workflows(&self, threshold_secs: u64) -> StorageResult<Vec<WorkflowRun>> {
        Err(StorageError::NotImplemented("surreal workflow backend"))
    }

    async fn validate_write_with_guard(
        &self,
        ctx: &WriteContext,
        resource_id: &str,
    ) -> StorageResult<MutationMetadata> {
        Err(StorageError::NotImplemented("surreal core backend"))
    }

    async fn prune_ai_jobs(
        &self,
        cutoff: DateTime<Utc>,
        min_versions: u32,
        dry_run: bool,
    ) -> StorageResult<PruneReport> {
        Err(StorageError::NotImplemented("surreal ai backend"))
    }

    async fn run_migrations(&self) -> StorageResult<()> {
        Err(StorageError::NotImplemented("surreal core backend"))
    }

    async fn migration_version(&self) -> StorageResult<i64> {
        Err(StorageError::NotImplemented("surreal core backend"))
    }
}

#[cfg(test)]
mod not_implemented_surface {
    /// Number of `Database` methods on [`super::SurrealDatabase`] that still
    /// resolve to the trait's `NotImplemented` contract instead of a real
    /// SurrealDB implementation.
    ///
    /// This number MUST fall to zero before the SurrealDB port can be called
    /// complete. It is asserted mechanically so a port wave cannot quietly
    /// leave a surface unimplemented, and so raising it is always a deliberate,
    /// reviewable edit rather than an accident.
    const DECLARED_NOT_IMPLEMENTED: usize = 106;

    #[test]
    fn not_implemented_surface_is_declared() {
        // Assembled at compile time from two fragments so this test's own
        // source text cannot inflate the count it is measuring.
        let needle = concat!("StorageError::", "NotImplemented(");
        let actual = include_str!("database.rs").matches(needle).count();
        assert_eq!(
            actual, DECLARED_NOT_IMPLEMENTED,
            "the un-ported SurrealDB surface changed: {actual} methods still return              NotImplemented but {DECLARED_NOT_IMPLEMENTED} were declared. Update              DECLARED_NOT_IMPLEMENTED in the same change that ports or adds a method,              so the remaining surface stays visible."
        );
    }
}
