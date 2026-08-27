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

    /// Construct the durable KB-003 authority adapter over this database's
    /// embedded store. The adapter implements the existing synchronous
    /// `Kb003Storage` consumer contract and also exposes async entrypoints.
    pub fn kb003_storage(&self) -> super::SurrealKb003Storage {
        super::SurrealKb003Storage::new(self.storage.clone())
    }

    async fn mutation_metadata(
        &self,
        ctx: &WriteContext,
        resource_id: &str,
    ) -> StorageResult<MutationMetadata> {
        self.storage
            .inner
            .guard
            .validate_write(ctx, resource_id)
            .await
            .map_err(StorageError::from)
    }
}

#[async_trait]
impl Database for SurrealDatabase {
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
        // Mirrors the trait contract default target; revisited when the
        // SurrealDB graph traversal backend is implemented.
        250
    }

    async fn ping(&self) -> StorageResult<()> {
        self.storage
            .with_data_operation(|database| {
                Box::pin(async move {
                    database.client.query("RETURN true;").await?.check()?;
                    Ok(())
                })
            })
            .await
            .map_err(|error| StorageError::Database(error.to_string()))
    }

    async fn list_workspaces(&self) -> StorageResult<Vec<Workspace>> {
        self.storage.list_workspaces().await
    }

    async fn create_workspace(
        &self,
        ctx: &WriteContext,
        workspace: NewWorkspace,
    ) -> StorageResult<Workspace> {
        self.storage.create_workspace(ctx, workspace).await
    }

    async fn delete_workspace(&self, ctx: &WriteContext, id: &str) -> StorageResult<()> {
        self.storage.delete_workspace(ctx, id).await
    }

    async fn get_workspace(&self, id: &str) -> StorageResult<Option<Workspace>> {
        self.storage.get_workspace(id).await
    }

    async fn list_documents(&self, workspace_id: &str) -> StorageResult<Vec<Document>> {
        self.storage.list_documents(workspace_id).await
    }

    async fn get_document(&self, doc_id: &str) -> StorageResult<Document> {
        self.storage.get_document(doc_id).await
    }

    async fn create_document(
        &self,
        ctx: &WriteContext,
        doc: NewDocument,
    ) -> StorageResult<Document> {
        self.storage.create_document(ctx, doc).await
    }

    async fn delete_document(&self, ctx: &WriteContext, doc_id: &str) -> StorageResult<()> {
        self.storage.delete_document(ctx, doc_id).await
    }

    async fn get_blocks(&self, doc_id: &str) -> StorageResult<Vec<Block>> {
        self.storage.get_blocks(doc_id).await
    }

    async fn get_block(&self, block_id: &str) -> StorageResult<Block> {
        self.storage.get_block(block_id).await
    }

    async fn create_block(&self, ctx: &WriteContext, block: NewBlock) -> StorageResult<Block> {
        self.storage.create_block(ctx, block).await
    }

    async fn update_block(
        &self,
        ctx: &WriteContext,
        block_id: &str,
        data: BlockUpdate,
    ) -> StorageResult<()> {
        self.storage.update_block(ctx, block_id, data).await
    }

    async fn delete_block(&self, ctx: &WriteContext, block_id: &str) -> StorageResult<()> {
        self.storage.delete_block(ctx, block_id).await
    }

    async fn replace_blocks(
        &self,
        ctx: &WriteContext,
        document_id: &str,
        blocks: Vec<NewBlock>,
    ) -> StorageResult<Vec<Block>> {
        self.storage.replace_blocks(ctx, document_id, blocks).await
    }

    async fn create_asset(&self, ctx: &WriteContext, asset: NewAsset) -> StorageResult<Asset> {
        let asset_id = uuid::Uuid::now_v7().to_string();
        let metadata = self.mutation_metadata(ctx, &asset_id).await?;
        self.storage
            .with_storage_operation(move |database| {
                Box::pin(async move {
                    super::loom_store::create_asset(&database, asset_id, asset, metadata).await
                })
            })
            .await
            .map_err(StorageError::from)?
    }

    async fn get_asset(&self, workspace_id: &str, asset_id: &str) -> StorageResult<Asset> {
        let workspace_id = workspace_id.to_owned();
        let asset_id = asset_id.to_owned();
        self.storage
            .with_storage_operation(move |database| {
                Box::pin(async move {
                    super::loom_store::get_asset(&database, &workspace_id, &asset_id).await
                })
            })
            .await
            .map_err(StorageError::from)?
    }

    async fn find_asset_by_content_hash(
        &self,
        workspace_id: &str,
        content_hash: &str,
    ) -> StorageResult<Option<Asset>> {
        let workspace_id = workspace_id.to_owned();
        let content_hash = content_hash.to_owned();
        self.storage
            .with_storage_operation(move |database| {
                Box::pin(async move {
                    super::loom_store::find_asset_by_content_hash(
                        &database,
                        &workspace_id,
                        &content_hash,
                    )
                    .await
                })
            })
            .await
            .map_err(StorageError::from)?
    }

    async fn upsert_media_tier(
        &self,
        ctx: &WriteContext,
        upsert: MediaTierUpsert,
    ) -> StorageResult<MediaAssetTier> {
        self.mutation_metadata(ctx, &upsert.asset_id).await?;
        self.storage
            .with_storage_operation(move |database| {
                Box::pin(
                    async move { super::loom_store::upsert_media_tier(&database, upsert).await },
                )
            })
            .await
            .map_err(StorageError::from)?
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
        self.mutation_metadata(ctx, asset_id).await?;
        let workspace_id = workspace_id.to_owned();
        let asset_id = asset_id.to_owned();
        self.storage
            .with_storage_operation(move |database| {
                Box::pin(async move {
                    super::loom_store::set_media_tier_status(
                        &database,
                        &workspace_id,
                        &asset_id,
                        tier,
                        status,
                        failure_reason,
                    )
                    .await
                })
            })
            .await
            .map_err(StorageError::from)?
    }

    async fn get_media_tier(
        &self,
        workspace_id: &str,
        asset_id: &str,
        tier: MediaTier,
    ) -> StorageResult<Option<MediaAssetTier>> {
        let workspace_id = workspace_id.to_owned();
        let asset_id = asset_id.to_owned();
        self.storage
            .with_storage_operation(move |database| {
                Box::pin(async move {
                    super::loom_store::get_media_tier(&database, &workspace_id, &asset_id, tier)
                        .await
                })
            })
            .await
            .map_err(StorageError::from)?
    }

    async fn list_media_tiers(
        &self,
        workspace_id: &str,
        asset_id: &str,
    ) -> StorageResult<Vec<MediaAssetTier>> {
        let workspace_id = workspace_id.to_owned();
        let asset_id = asset_id.to_owned();
        self.storage
            .with_storage_operation(move |database| {
                Box::pin(async move {
                    super::loom_store::list_media_tiers(&database, &workspace_id, &asset_id).await
                })
            })
            .await
            .map_err(StorageError::from)?
    }

    async fn list_failed_media_tiers(
        &self,
        workspace_id: &str,
    ) -> StorageResult<Vec<MediaAssetTier>> {
        let workspace_id = workspace_id.to_owned();
        self.storage
            .with_storage_operation(move |database| {
                Box::pin(async move {
                    super::loom_store::list_failed_media_tiers(&database, &workspace_id).await
                })
            })
            .await
            .map_err(StorageError::from)?
    }

    async fn delete_media_tiers(
        &self,
        ctx: &WriteContext,
        workspace_id: &str,
        asset_id: &str,
    ) -> StorageResult<u64> {
        self.mutation_metadata(ctx, asset_id).await?;
        let workspace_id = workspace_id.to_owned();
        let asset_id = asset_id.to_owned();
        self.storage
            .with_storage_operation(move |database| {
                Box::pin(async move {
                    super::loom_store::delete_media_tiers(&database, &workspace_id, &asset_id).await
                })
            })
            .await
            .map_err(StorageError::from)?
    }

    async fn create_loom_collection(
        &self,
        ctx: &WriteContext,
        workspace_id: &str,
        title: Option<String>,
    ) -> StorageResult<LoomCollection> {
        let collection_id = uuid::Uuid::now_v7().to_string();
        let metadata = self.mutation_metadata(ctx, &collection_id).await?;
        let workspace_id = workspace_id.to_owned();
        self.storage
            .with_storage_operation(move |database| {
                Box::pin(async move {
                    super::loom_store::create_loom_collection(
                        &database,
                        collection_id,
                        &workspace_id,
                        title,
                        metadata,
                    )
                    .await
                })
            })
            .await
            .map_err(StorageError::from)?
    }

    async fn get_loom_collection(
        &self,
        workspace_id: &str,
        collection_id: &str,
    ) -> StorageResult<LoomCollectionWithMembers> {
        let workspace_id = workspace_id.to_owned();
        let collection_id = collection_id.to_owned();
        self.storage
            .with_storage_operation(move |database| {
                Box::pin(async move {
                    super::loom_store::get_loom_collection(&database, &workspace_id, &collection_id)
                        .await
                })
            })
            .await
            .map_err(StorageError::from)?
    }

    async fn set_loom_collection_order(
        &self,
        ctx: &WriteContext,
        workspace_id: &str,
        collection_id: &str,
        asset_ids: &[String],
    ) -> StorageResult<LoomCollectionWithMembers> {
        self.mutation_metadata(ctx, collection_id).await?;
        let workspace_id = workspace_id.to_owned();
        let collection_id = collection_id.to_owned();
        let asset_ids = asset_ids.to_vec();
        self.storage
            .with_storage_operation(move |database| {
                Box::pin(async move {
                    super::loom_store::set_loom_collection_order(
                        &database,
                        &workspace_id,
                        &collection_id,
                        &asset_ids,
                    )
                    .await
                })
            })
            .await
            .map_err(StorageError::from)?
    }

    async fn create_loom_block(
        &self,
        ctx: &WriteContext,
        block: NewLoomBlock,
    ) -> StorageResult<LoomBlock> {
        let mut block = block;
        let block_id = block
            .block_id
            .clone()
            .unwrap_or_else(|| uuid::Uuid::now_v7().to_string());
        block.block_id = Some(block_id.clone());
        let metadata = self.mutation_metadata(ctx, &block_id).await?;
        self.storage
            .with_storage_operation(move |database| {
                Box::pin(async move {
                    super::loom_store::create_loom_block(&database, block, metadata).await
                })
            })
            .await
            .map_err(StorageError::from)?
    }

    async fn get_or_create_daily_journal_block(
        &self,
        ctx: &WriteContext,
        workspace_id: &str,
        journal_date: &str,
    ) -> StorageResult<LoomBlock> {
        let block_id = uuid::Uuid::now_v7().to_string();
        let metadata = self.mutation_metadata(ctx, &block_id).await?;
        let workspace_id = workspace_id.to_owned();
        let journal_date = journal_date.to_owned();
        self.storage
            .with_storage_operation(move |database| {
                Box::pin(async move {
                    super::loom_store::get_or_create_daily_journal_block(
                        &database,
                        block_id,
                        &workspace_id,
                        &journal_date,
                        metadata,
                    )
                    .await
                })
            })
            .await
            .map_err(StorageError::from)?
    }

    async fn get_loom_block(&self, workspace_id: &str, block_id: &str) -> StorageResult<LoomBlock> {
        let workspace_id = workspace_id.to_owned();
        let block_id = block_id.to_owned();
        self.storage
            .with_storage_operation(move |database| {
                Box::pin(async move {
                    super::loom_store::get_loom_block(&database, &workspace_id, &block_id).await
                })
            })
            .await
            .map_err(StorageError::from)?
    }

    async fn bridge_loom_block_to_knowledge(
        &self,
        ctx: &WriteContext,
        workspace_id: &str,
        block_id: &str,
    ) -> StorageResult<LoomKnowledgeBridge> {
        let metadata = self.mutation_metadata(ctx, block_id).await?;
        super::bridge_store::bridge_loom_block_to_knowledge(
            &self.storage,
            ctx,
            metadata,
            workspace_id,
            block_id,
        )
        .await
    }

    async fn get_loom_block_knowledge_bridge(
        &self,
        workspace_id: &str,
        block_id: &str,
    ) -> StorageResult<Option<LoomKnowledgeBridge>> {
        super::bridge_store::get_loom_block_knowledge_bridge(&self.storage, workspace_id, block_id)
            .await
    }

    async fn list_loom_block_knowledge_bridges(
        &self,
        workspace_id: &str,
    ) -> StorageResult<Vec<LoomKnowledgeBridge>> {
        super::bridge_store::list_loom_block_knowledge_bridges(&self.storage, workspace_id).await
    }

    async fn find_loom_block_by_content_hash(
        &self,
        workspace_id: &str,
        content_hash: &str,
    ) -> StorageResult<Option<LoomBlock>> {
        let workspace_id = workspace_id.to_owned();
        let content_hash = content_hash.to_owned();
        self.storage
            .with_storage_operation(move |database| {
                Box::pin(async move {
                    super::loom_store::find_loom_block_by_content_hash(
                        &database,
                        &workspace_id,
                        &content_hash,
                    )
                    .await
                })
            })
            .await
            .map_err(StorageError::from)?
    }

    async fn find_loom_block_by_asset_id(
        &self,
        workspace_id: &str,
        asset_id: &str,
    ) -> StorageResult<Option<LoomBlock>> {
        let workspace_id = workspace_id.to_owned();
        let asset_id = asset_id.to_owned();
        self.storage
            .with_storage_operation(move |database| {
                Box::pin(async move {
                    super::loom_store::find_loom_block_by_asset_id(
                        &database,
                        &workspace_id,
                        &asset_id,
                    )
                    .await
                })
            })
            .await
            .map_err(StorageError::from)?
    }

    async fn update_loom_block(
        &self,
        ctx: &WriteContext,
        workspace_id: &str,
        block_id: &str,
        update: LoomBlockUpdate,
    ) -> StorageResult<LoomBlock> {
        let metadata = self.mutation_metadata(ctx, block_id).await?;
        let workspace_id = workspace_id.to_owned();
        let block_id = block_id.to_owned();
        self.storage
            .with_storage_operation(move |database| {
                Box::pin(async move {
                    super::loom_store::update_loom_block(
                        &database,
                        &workspace_id,
                        &block_id,
                        update,
                        metadata,
                    )
                    .await
                })
            })
            .await
            .map_err(StorageError::from)?
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
        let metadata = self.mutation_metadata(ctx, block_id).await?;
        let workspace_id = workspace_id.to_owned();
        let block_id = block_id.to_owned();
        self.storage
            .with_storage_operation(move |database| {
                Box::pin(async move {
                    super::loom_store::set_loom_block_preview(
                        &database,
                        &workspace_id,
                        &block_id,
                        preview_status,
                        thumbnail_asset_id,
                        proxy_asset_id,
                        metadata,
                    )
                    .await
                })
            })
            .await
            .map_err(StorageError::from)?
    }

    async fn delete_loom_block(
        &self,
        ctx: &WriteContext,
        workspace_id: &str,
        block_id: &str,
    ) -> StorageResult<()> {
        self.mutation_metadata(ctx, block_id).await?;
        let workspace_id = workspace_id.to_owned();
        let block_id = block_id.to_owned();
        self.storage
            .with_storage_operation(move |database| {
                Box::pin(async move {
                    super::loom_store::delete_loom_block(&database, &workspace_id, &block_id).await
                })
            })
            .await
            .map_err(StorageError::from)?
    }

    async fn create_loom_edge(
        &self,
        ctx: &WriteContext,
        edge: NewLoomEdge,
    ) -> StorageResult<LoomEdge> {
        let mut edge = edge;
        let edge_id = edge
            .edge_id
            .clone()
            .unwrap_or_else(|| uuid::Uuid::now_v7().to_string());
        edge.edge_id = Some(edge_id.clone());
        let metadata = self.mutation_metadata(ctx, &edge_id).await?;
        self.storage
            .with_storage_operation(move |database| {
                Box::pin(async move {
                    super::loom_store::create_loom_edge(&database, edge, metadata).await
                })
            })
            .await
            .map_err(StorageError::from)?
    }

    async fn delete_loom_edge(
        &self,
        ctx: &WriteContext,
        workspace_id: &str,
        edge_id: &str,
    ) -> StorageResult<LoomEdge> {
        self.mutation_metadata(ctx, edge_id).await?;
        let workspace_id = workspace_id.to_owned();
        let edge_id = edge_id.to_owned();
        self.storage
            .with_storage_operation(move |database| {
                Box::pin(async move {
                    super::loom_store::delete_loom_edge(&database, &workspace_id, &edge_id).await
                })
            })
            .await
            .map_err(StorageError::from)?
    }

    async fn list_loom_edges_for_block(
        &self,
        workspace_id: &str,
        block_id: &str,
    ) -> StorageResult<Vec<LoomEdge>> {
        let workspace_id = workspace_id.to_owned();
        let block_id = block_id.to_owned();
        self.storage
            .with_storage_operation(move |database| {
                Box::pin(async move {
                    super::loom_store::list_loom_edges_for_block(
                        &database,
                        &workspace_id,
                        &block_id,
                    )
                    .await
                })
            })
            .await
            .map_err(StorageError::from)?
    }

    async fn get_backlinks(
        &self,
        workspace_id: &str,
        block_id: &str,
    ) -> StorageResult<Vec<LoomEdge>> {
        let workspace_id = workspace_id.to_owned();
        let block_id = block_id.to_owned();
        self.storage
            .with_storage_operation(move |database| {
                Box::pin(async move {
                    super::loom_store::get_backlinks(&database, &workspace_id, &block_id).await
                })
            })
            .await
            .map_err(StorageError::from)?
    }

    async fn get_outgoing_edges(
        &self,
        workspace_id: &str,
        block_id: &str,
    ) -> StorageResult<Vec<LoomEdge>> {
        let workspace_id = workspace_id.to_owned();
        let block_id = block_id.to_owned();
        self.storage
            .with_storage_operation(move |database| {
                Box::pin(async move {
                    super::loom_store::get_outgoing_edges(&database, &workspace_id, &block_id).await
                })
            })
            .await
            .map_err(StorageError::from)?
    }

    async fn traverse_graph(
        &self,
        workspace_id: &str,
        start_block_id: &str,
        max_depth: u32,
        edge_types: &[LoomEdgeType],
    ) -> StorageResult<Vec<(LoomBlock, u32)>> {
        let workspace_id = workspace_id.to_owned();
        let start_block_id = start_block_id.to_owned();
        let edge_types = edge_types.to_vec();
        self.storage
            .with_storage_operation(move |database| {
                Box::pin(async move {
                    super::loom_store::traverse_graph(
                        &database,
                        &workspace_id,
                        &start_block_id,
                        max_depth,
                        &edge_types,
                    )
                    .await
                })
            })
            .await
            .map_err(StorageError::from)?
    }

    async fn recompute_block_metrics(
        &self,
        workspace_id: &str,
        block_id: &str,
    ) -> StorageResult<()> {
        let workspace_id = workspace_id.to_owned();
        let block_id = block_id.to_owned();
        self.storage
            .with_storage_operation(move |database| {
                Box::pin(async move {
                    super::loom_store::recompute_block_metrics(&database, &workspace_id, &block_id)
                        .await
                })
            })
            .await
            .map_err(StorageError::from)?
    }

    async fn recompute_all_metrics(&self, workspace_id: &str) -> StorageResult<()> {
        let workspace_id = workspace_id.to_owned();
        self.storage
            .with_storage_operation(move |database| {
                Box::pin(async move {
                    super::loom_store::recompute_all_metrics(&database, &workspace_id).await
                })
            })
            .await
            .map_err(StorageError::from)?
    }

    async fn query_loom_view(
        &self,
        workspace_id: &str,
        view_type: LoomViewType,
        filters: LoomViewFilters,
        limit: u32,
        offset: u32,
    ) -> StorageResult<LoomViewResponse> {
        let workspace_id = workspace_id.to_owned();
        self.storage
            .with_storage_operation(move |database| {
                Box::pin(async move {
                    super::loom_store::query_loom_view(
                        &database,
                        &workspace_id,
                        view_type,
                        filters,
                        limit,
                        offset,
                    )
                    .await
                })
            })
            .await
            .map_err(StorageError::from)?
    }

    async fn search_loom_blocks(
        &self,
        workspace_id: &str,
        query: &str,
        filters: LoomSearchFilters,
        limit: u32,
        offset: u32,
    ) -> StorageResult<Vec<LoomBlockSearchResult>> {
        let workspace_id = workspace_id.to_owned();
        let query = query.to_owned();
        self.storage
            .with_storage_operation(move |database| {
                Box::pin(async move {
                    super::loom_store::search_loom_blocks(
                        &database,
                        &workspace_id,
                        &query,
                        filters,
                        limit,
                        offset,
                    )
                    .await
                })
            })
            .await
            .map_err(StorageError::from)?
    }

    async fn search_loom_graph(
        &self,
        workspace_id: &str,
        query: &str,
        filters: LoomSearchFilters,
        limit: u32,
        offset: u32,
    ) -> StorageResult<Vec<LoomGraphSearchResult>> {
        let workspace_id = workspace_id.to_owned();
        let query = query.to_owned();
        self.storage
            .with_storage_operation(move |database| {
                Box::pin(async move {
                    super::loom_store::search_loom_graph(
                        &database,
                        &workspace_id,
                        &query,
                        filters,
                        limit,
                        offset,
                    )
                    .await
                })
            })
            .await
            .map_err(StorageError::from)?
    }

    async fn reindex_loom_block_search(
        &self,
        ctx: &WriteContext,
        workspace_id: &str,
        block_id: &str,
        search_text: &str,
        embedding: Option<&[f32]>,
        embedding_model: Option<&str>,
    ) -> StorageResult<()> {
        let metadata = self.mutation_metadata(ctx, block_id).await?;
        let workspace_id = workspace_id.to_owned();
        let block_id = block_id.to_owned();
        let search_text = search_text.to_owned();
        let embedding = embedding.map(<[f32]>::to_vec);
        let embedding_model = embedding_model.map(str::to_owned);
        self.storage
            .with_storage_operation(move |database| {
                Box::pin(async move {
                    super::search_store::reindex_loom_block_search(
                        &database,
                        &workspace_id,
                        &block_id,
                        &search_text,
                        embedding.as_deref(),
                        embedding_model.as_deref(),
                        metadata,
                    )
                    .await
                })
            })
            .await
            .map_err(StorageError::from)?
    }

    async fn loom_search_v2(
        &self,
        workspace_id: &str,
        request: LoomSearchV2Request,
    ) -> StorageResult<LoomSearchV2Response> {
        let workspace_id = workspace_id.to_owned();
        self.storage
            .with_storage_operation(move |database| {
                Box::pin(async move {
                    super::search_store::loom_search_v2(&database, &workspace_id, request).await
                })
            })
            .await
            .map_err(StorageError::from)?
    }

    async fn record_quick_switcher_recent(
        &self,
        workspace_id: &str,
        input: QuickSwitcherRecentInput,
    ) -> StorageResult<QuickSwitcherRecent> {
        super::state_store::record_quick_switcher_recent(&self.storage, workspace_id, input).await
    }

    async fn list_quick_switcher_recents(
        &self,
        workspace_id: &str,
        limit: u32,
    ) -> StorageResult<Vec<QuickSwitcherRecent>> {
        super::state_store::list_quick_switcher_recents(&self.storage, workspace_id, limit).await
    }

    async fn get_workbench_layout_state(
        &self,
        workspace_id: &str,
    ) -> StorageResult<Option<WorkbenchLayoutState>> {
        super::state_store::get_workbench_layout_state(&self.storage, workspace_id).await
    }

    async fn save_workbench_layout_state(
        &self,
        workspace_id: &str,
        input: WorkbenchLayoutStateInput,
    ) -> StorageResult<WorkbenchLayoutState> {
        super::state_store::save_workbench_layout_state(&self.storage, workspace_id, input).await
    }

    async fn get_workspace_settings_state(
        &self,
        workspace_id: &str,
    ) -> StorageResult<Option<WorkspaceSettingsState>> {
        super::state_store::get_workspace_settings_state(&self.storage, workspace_id).await
    }

    async fn save_workspace_settings_state(
        &self,
        workspace_id: &str,
        input: WorkspaceSettingsStateInput,
    ) -> StorageResult<WorkspaceSettingsState> {
        super::state_store::save_workspace_settings_state(&self.storage, workspace_id, input).await
    }

    async fn get_workspace_search_bookmark_state(
        &self,
        workspace_id: &str,
    ) -> StorageResult<Option<WorkspaceSearchBookmarkState>> {
        super::state_store::get_workspace_search_bookmark_state(&self.storage, workspace_id).await
    }

    async fn save_workspace_search_bookmark_state(
        &self,
        workspace_id: &str,
        input: WorkspaceSearchBookmarkStateInput,
    ) -> StorageResult<WorkspaceSearchBookmarkState> {
        super::state_store::save_workspace_search_bookmark_state(&self.storage, workspace_id, input)
            .await
    }

    async fn list_debug_breakpoints(
        &self,
        rich_document_id: &str,
    ) -> StorageResult<Vec<DebugBreakpoint>> {
        super::state_store::list_debug_breakpoints(&self.storage, rich_document_id).await
    }

    async fn set_debug_breakpoints(
        &self,
        rich_document_id: &str,
        workspace_id: &str,
        breakpoints: Vec<DebugBreakpointInput>,
    ) -> StorageResult<Vec<DebugBreakpoint>> {
        super::state_store::set_debug_breakpoints(
            &self.storage,
            rich_document_id,
            workspace_id,
            breakpoints,
        )
        .await
    }

    async fn loom_visual_debug_snapshot(
        &self,
        workspace_id: &str,
        start_block_id: &str,
        query: &str,
        limit: u32,
    ) -> StorageResult<LoomVisualDebugSnapshot> {
        super::visual_debug_store::snapshot(self, workspace_id, start_block_id, query, limit).await
    }

    async fn preference_get(
        &self,
        scope: &crate::preferences::PreferenceScope,
        entry: &crate::preferences::PreferenceSchemaEntry,
    ) -> StorageResult<crate::preferences::PreferenceRecord> {
        self.storage.preference_get(scope, entry).await
    }

    async fn preference_set(
        &self,
        scope: &crate::preferences::PreferenceScope,
        entry: &crate::preferences::PreferenceSchemaEntry,
        value: Value,
        source: crate::preferences::PreferenceSource,
        actor: &str,
    ) -> StorageResult<(
        crate::preferences::PreferenceRecord,
        crate::preferences::PreferenceChangeReceipt,
    )> {
        self.storage
            .preference_set(scope, entry, value, source, actor)
            .await
    }

    async fn preference_reset(
        &self,
        scope: &crate::preferences::PreferenceScope,
        entry: &crate::preferences::PreferenceSchemaEntry,
        actor: &str,
    ) -> StorageResult<(
        crate::preferences::PreferenceRecord,
        crate::preferences::PreferenceChangeReceipt,
    )> {
        self.storage.preference_reset(scope, entry, actor).await
    }

    async fn preference_history(
        &self,
        scope: &crate::preferences::PreferenceScope,
        preference_id: &str,
    ) -> StorageResult<Vec<crate::preferences::PreferenceChangeReceipt>> {
        self.storage.preference_history(scope, preference_id).await
    }

    async fn preference_projection(
        &self,
        scope: &crate::preferences::PreferenceScope,
        entries: &[crate::preferences::PreferenceSchemaEntry],
    ) -> StorageResult<Vec<crate::preferences::PreferenceProjectionRow>> {
        self.storage.preference_projection(scope, entries).await
    }

    async fn upsert_calendar_source(
        &self,
        ctx: &WriteContext,
        source: CalendarSourceUpsert,
    ) -> StorageResult<CalendarSource> {
        super::calendar_store::upsert_source(&self.storage, ctx, source).await
    }

    async fn list_calendar_sources(
        &self,
        workspace_id: &str,
    ) -> StorageResult<Vec<CalendarSource>> {
        super::calendar_store::list_sources(&self.storage, workspace_id).await
    }

    async fn get_calendar_source(
        &self,
        workspace_id: &str,
        source_id: &str,
    ) -> StorageResult<Option<CalendarSource>> {
        super::calendar_store::get_source(&self.storage, workspace_id, source_id).await
    }

    async fn upsert_calendar_event(
        &self,
        ctx: &WriteContext,
        event: CalendarEventUpsert,
    ) -> StorageResult<CalendarEvent> {
        super::calendar_store::upsert_event(&self.storage, ctx, event).await
    }

    async fn query_calendar_events(
        &self,
        query: CalendarEventWindowQuery,
    ) -> StorageResult<Vec<CalendarEvent>> {
        super::calendar_store::query_events(&self.storage, query).await
    }

    async fn delete_calendar_data_by_source(
        &self,
        ctx: &WriteContext,
        workspace_id: &str,
        source_id: &str,
    ) -> StorageResult<()> {
        super::calendar_store::delete_source(&self.storage, ctx, workspace_id, source_id).await
    }

    async fn create_canvas(&self, ctx: &WriteContext, canvas: NewCanvas) -> StorageResult<Canvas> {
        super::canvas_store::create(&self.storage, ctx, canvas).await
    }

    async fn list_canvases(&self, workspace_id: &str) -> StorageResult<Vec<Canvas>> {
        super::canvas_store::list(&self.storage, workspace_id).await
    }

    async fn get_canvas_with_graph(&self, canvas_id: &str) -> StorageResult<CanvasGraph> {
        super::canvas_store::get_graph(&self.storage, canvas_id).await
    }

    async fn rename_canvas(
        &self,
        ctx: &WriteContext,
        canvas_id: &str,
        title: &str,
        expected_updated_at: Option<DateTime<Utc>>,
    ) -> StorageResult<Canvas> {
        super::canvas_store::rename(&self.storage, ctx, canvas_id, title, expected_updated_at).await
    }

    async fn update_canvas_graph(
        &self,
        ctx: &WriteContext,
        canvas_id: &str,
        nodes: Vec<NewCanvasNode>,
        edges: Vec<NewCanvasEdge>,
    ) -> StorageResult<CanvasGraph> {
        super::canvas_store::update_graph(&self.storage, ctx, canvas_id, nodes, edges).await
    }

    async fn delete_canvas(&self, ctx: &WriteContext, canvas_id: &str) -> StorageResult<()> {
        super::canvas_store::delete(&self.storage, ctx, canvas_id).await
    }

    async fn create_ai_bronze_record(
        &self,
        ctx: &WriteContext,
        record: NewBronzeRecord,
    ) -> StorageResult<BronzeRecord> {
        super::ai_ready_store::create_bronze(&self.storage, ctx, record).await
    }

    async fn get_ai_bronze_record(&self, bronze_id: &str) -> StorageResult<Option<BronzeRecord>> {
        super::ai_ready_store::get_bronze(&self.storage, bronze_id).await
    }

    async fn list_ai_bronze_records(&self, workspace_id: &str) -> StorageResult<Vec<BronzeRecord>> {
        super::ai_ready_store::list_bronze(&self.storage, workspace_id).await
    }

    async fn mark_ai_bronze_deleted(
        &self,
        ctx: &WriteContext,
        bronze_id: &str,
    ) -> StorageResult<()> {
        super::ai_ready_store::mark_bronze_deleted(&self.storage, ctx, bronze_id).await
    }

    async fn create_ai_silver_record(
        &self,
        ctx: &WriteContext,
        record: NewSilverRecord,
    ) -> StorageResult<SilverRecord> {
        super::ai_ready_store::create_silver(&self.storage, ctx, record).await
    }

    async fn get_ai_silver_record(&self, silver_id: &str) -> StorageResult<Option<SilverRecord>> {
        super::ai_ready_store::get_silver(&self.storage, silver_id).await
    }

    async fn list_ai_silver_records_by_bronze(
        &self,
        bronze_id: &str,
    ) -> StorageResult<Vec<SilverRecord>> {
        super::ai_ready_store::list_silver_by_bronze(&self.storage, bronze_id).await
    }

    async fn list_ai_silver_records(&self, workspace_id: &str) -> StorageResult<Vec<SilverRecord>> {
        super::ai_ready_store::list_silver(&self.storage, workspace_id).await
    }

    async fn supersede_ai_silver_record(
        &self,
        ctx: &WriteContext,
        superseded_silver_id: &str,
        new_silver_id: &str,
    ) -> StorageResult<()> {
        super::ai_ready_store::supersede_silver(
            &self.storage,
            ctx,
            superseded_silver_id,
            new_silver_id,
        )
        .await
    }

    async fn upsert_ai_embedding_model(
        &self,
        ctx: &WriteContext,
        model: EmbeddingModelRecord,
    ) -> StorageResult<()> {
        super::ai_ready_store::upsert_embedding_model(&self.storage, ctx, model).await
    }

    async fn list_ai_embedding_models(&self) -> StorageResult<Vec<EmbeddingModelRecord>> {
        super::ai_ready_store::list_embedding_models(&self.storage).await
    }

    async fn set_ai_embedding_default_model(
        &self,
        ctx: &WriteContext,
        model_id: &str,
        model_version: &str,
    ) -> StorageResult<()> {
        super::ai_ready_store::set_default_embedding_model(
            &self.storage,
            ctx,
            model_id,
            model_version,
        )
        .await
    }

    async fn get_ai_embedding_registry(&self) -> StorageResult<Option<EmbeddingRegistry>> {
        super::ai_ready_store::get_embedding_registry(&self.storage).await
    }

    async fn get_ai_job(&self, job_id: &str) -> StorageResult<AiJob> {
        super::ai_job_store::get(&self.storage, job_id).await
    }

    async fn list_ai_jobs(&self, filter: AiJobListFilter) -> StorageResult<Vec<AiJob>> {
        super::ai_job_store::list(&self.storage, filter).await
    }

    async fn create_ai_job(&self, job: NewAiJob) -> StorageResult<AiJob> {
        super::ai_job_store::create(&self.storage, job).await
    }

    async fn update_ai_job_status(&self, update: JobStatusUpdate) -> StorageResult<AiJob> {
        super::ai_job_store::update_status(&self.storage, update).await
    }

    async fn set_job_outputs(&self, job_id: &str, outputs: Option<Value>) -> StorageResult<()> {
        super::ai_job_store::set_outputs(&self.storage, job_id, outputs).await
    }

    async fn upsert_model_session(&self, session: NewModelSession) -> StorageResult<ModelSession> {
        super::session_store::upsert_model_session(&self.storage, session).await
    }

    async fn get_model_session(&self, session_id: &str) -> StorageResult<ModelSession> {
        super::session_store::get_model_session(&self.storage, session_id).await
    }

    async fn get_model_session_by_job_id(&self, job_id: Uuid) -> StorageResult<ModelSession> {
        super::session_store::get_model_session_by_job_id(&self.storage, job_id).await
    }

    async fn update_model_session_state_with_merge_back_artifact(
        &self,
        session_id: &str,
        state: ModelSessionState,
        job_id: Option<Uuid>,
        merge_back_artifact: Option<MergeBackArtifact>,
    ) -> StorageResult<ModelSession> {
        super::session_store::update_model_session_state(
            &self.storage,
            session_id,
            state,
            job_id,
            merge_back_artifact,
        )
        .await
    }

    async fn close_model_session(
        &self,
        session_id: &str,
        state: ModelSessionState,
        close_reason: &str,
        actor: &str,
    ) -> StorageResult<ModelSession> {
        super::session_store::close_model_session(
            &self.storage,
            session_id,
            state,
            close_reason,
            actor,
        )
        .await
    }

    async fn create_session_checkpoint(
        &self,
        checkpoint: SessionCheckpoint,
    ) -> StorageResult<SessionCheckpoint> {
        super::session_store::create_session_checkpoint(&self.storage, checkpoint).await
    }

    async fn get_latest_session_checkpoint(
        &self,
        session_id: &str,
    ) -> StorageResult<SessionCheckpoint> {
        super::session_store::get_latest_session_checkpoint(&self.storage, session_id).await
    }

    async fn append_session_message(
        &self,
        message: NewSessionMessage,
    ) -> StorageResult<SessionMessage> {
        super::session_store::append_session_message(&self.storage, message).await
    }

    async fn list_session_messages(&self, session_id: &str) -> StorageResult<Vec<SessionMessage>> {
        super::session_store::list_session_messages(&self.storage, session_id).await
    }

    async fn append_kernel_event(&self, event: NewKernelEvent) -> StorageResult<KernelEvent> {
        super::event_ledger::append(&self.storage, event).await
    }

    async fn get_backlinks_with_context(
        &self,
        workspace_id: &str,
        block_id: &str,
    ) -> StorageResult<Vec<LoomBacklink>> {
        let workspace_id = workspace_id.to_owned();
        let block_id = block_id.to_owned();
        self.storage
            .with_storage_operation(move |database| {
                Box::pin(async move {
                    super::loom_store::get_backlinks_with_context(
                        &database,
                        &workspace_id,
                        &block_id,
                    )
                    .await
                })
            })
            .await
            .map_err(StorageError::from)?
    }

    async fn scan_unlinked_mentions(
        &self,
        workspace_id: &str,
        block_id: &str,
        aliases: &[String],
        limit: u32,
    ) -> StorageResult<Vec<LoomUnlinkedMention>> {
        let workspace_id = workspace_id.to_owned();
        let block_id = block_id.to_owned();
        let aliases = aliases.to_vec();
        self.storage
            .with_storage_operation(move |database| {
                Box::pin(async move {
                    super::loom_store::scan_unlinked_mentions(
                        &database,
                        &workspace_id,
                        &block_id,
                        &aliases,
                        limit,
                    )
                    .await
                })
            })
            .await
            .map_err(StorageError::from)?
    }

    async fn local_graph(
        &self,
        workspace_id: &str,
        start_block_id: &str,
        max_depth: u32,
        edge_types: &[LoomEdgeType],
        node_limit: u32,
    ) -> StorageResult<LoomGraph> {
        let workspace_id = workspace_id.to_owned();
        let start_block_id = start_block_id.to_owned();
        let edge_types = edge_types.to_vec();
        self.storage
            .with_storage_operation(move |database| {
                Box::pin(async move {
                    super::loom_store::local_graph(
                        &database,
                        &workspace_id,
                        &start_block_id,
                        max_depth,
                        &edge_types,
                        node_limit,
                    )
                    .await
                })
            })
            .await
            .map_err(StorageError::from)?
    }

    async fn global_graph(
        &self,
        workspace_id: &str,
        edge_types: &[LoomEdgeType],
        node_limit: u32,
        hub_degree_threshold: u32,
    ) -> StorageResult<LoomGraph> {
        let workspace_id = workspace_id.to_owned();
        let edge_types = edge_types.to_vec();
        self.storage
            .with_storage_operation(move |database| {
                Box::pin(async move {
                    super::loom_store::global_graph(
                        &database,
                        &workspace_id,
                        &edge_types,
                        node_limit,
                        hub_degree_threshold,
                    )
                    .await
                })
            })
            .await
            .map_err(StorageError::from)?
    }

    async fn list_tag_hubs(
        &self,
        workspace_id: &str,
        limit: u32,
        offset: u32,
    ) -> StorageResult<Vec<LoomBlock>> {
        let workspace_id = workspace_id.to_owned();
        self.storage
            .with_storage_operation(move |database| {
                Box::pin(async move {
                    super::loom_store::list_tag_hubs(&database, &workspace_id, limit, offset).await
                })
            })
            .await
            .map_err(StorageError::from)?
    }

    async fn get_tag_hub(
        &self,
        workspace_id: &str,
        tag_block_id: &str,
    ) -> StorageResult<LoomTagHub> {
        let workspace_id = workspace_id.to_owned();
        let tag_block_id = tag_block_id.to_owned();
        self.storage
            .with_storage_operation(move |database| {
                Box::pin(async move {
                    super::loom_store::get_tag_hub(&database, &workspace_id, &tag_block_id).await
                })
            })
            .await
            .map_err(StorageError::from)?
    }

    async fn list_blocks_for_tag(
        &self,
        workspace_id: &str,
        tag_block_id: &str,
        include_subtags: bool,
        limit: u32,
        offset: u32,
    ) -> StorageResult<Vec<LoomBlock>> {
        let workspace_id = workspace_id.to_owned();
        let tag_block_id = tag_block_id.to_owned();
        self.storage
            .with_storage_operation(move |database| {
                Box::pin(async move {
                    super::loom_store::list_blocks_for_tag(
                        &database,
                        &workspace_id,
                        &tag_block_id,
                        include_subtags,
                        limit,
                        offset,
                    )
                    .await
                })
            })
            .await
            .map_err(StorageError::from)?
    }

    async fn set_loom_block_pin_order(
        &self,
        ctx: &WriteContext,
        workspace_id: &str,
        block_id: &str,
        pin_order: Option<i32>,
    ) -> StorageResult<LoomBlock> {
        let metadata = self.mutation_metadata(ctx, block_id).await?;
        let workspace_id = workspace_id.to_owned();
        let block_id = block_id.to_owned();
        self.storage
            .with_storage_operation(move |database| {
                Box::pin(async move {
                    super::loom_store::set_loom_block_pin_order(
                        &database,
                        &workspace_id,
                        &block_id,
                        pin_order,
                        metadata,
                    )
                    .await
                })
            })
            .await
            .map_err(StorageError::from)?
    }

    async fn remove_loom_block_pin(
        &self,
        ctx: &WriteContext,
        workspace_id: &str,
        block_id: &str,
    ) -> StorageResult<LoomBlock> {
        let metadata = self.mutation_metadata(ctx, block_id).await?;
        let workspace_id = workspace_id.to_owned();
        let block_id = block_id.to_owned();
        self.storage
            .with_storage_operation(move |database| {
                Box::pin(async move {
                    super::loom_store::remove_loom_block_pin(
                        &database,
                        &workspace_id,
                        &block_id,
                        metadata,
                    )
                    .await
                })
            })
            .await
            .map_err(StorageError::from)?
    }

    async fn create_loom_folder(
        &self,
        workspace_id: &str,
        folder: NewLoomFolder,
    ) -> StorageResult<LoomFolder> {
        let workspace_id = workspace_id.to_owned();
        self.storage
            .with_storage_operation(move |database| {
                Box::pin(async move {
                    super::loom_store::create_loom_folder(&database, &workspace_id, folder).await
                })
            })
            .await
            .map_err(StorageError::from)?
    }

    async fn get_loom_folder(
        &self,
        workspace_id: &str,
        folder_id: &str,
    ) -> StorageResult<LoomFolder> {
        let workspace_id = workspace_id.to_owned();
        let folder_id = folder_id.to_owned();
        self.storage
            .with_storage_operation(move |database| {
                Box::pin(async move {
                    super::loom_store::get_loom_folder(&database, &workspace_id, &folder_id).await
                })
            })
            .await
            .map_err(StorageError::from)?
    }

    async fn list_loom_folders(&self, workspace_id: &str) -> StorageResult<Vec<LoomFolder>> {
        let workspace_id = workspace_id.to_owned();
        self.storage
            .with_storage_operation(move |database| {
                Box::pin(async move {
                    super::loom_store::list_loom_folders(&database, &workspace_id).await
                })
            })
            .await
            .map_err(StorageError::from)?
    }

    async fn update_loom_folder(
        &self,
        workspace_id: &str,
        folder_id: &str,
        update: LoomFolderUpdate,
    ) -> StorageResult<LoomFolder> {
        let workspace_id = workspace_id.to_owned();
        let folder_id = folder_id.to_owned();
        self.storage
            .with_storage_operation(move |database| {
                Box::pin(async move {
                    super::loom_store::update_loom_folder(
                        &database,
                        &workspace_id,
                        &folder_id,
                        update,
                    )
                    .await
                })
            })
            .await
            .map_err(StorageError::from)?
    }

    async fn delete_loom_folder(&self, workspace_id: &str, folder_id: &str) -> StorageResult<()> {
        let workspace_id = workspace_id.to_owned();
        let folder_id = folder_id.to_owned();
        self.storage
            .with_storage_operation(move |database| {
                Box::pin(async move {
                    super::loom_store::delete_loom_folder(&database, &workspace_id, &folder_id)
                        .await
                })
            })
            .await
            .map_err(StorageError::from)?
    }

    async fn add_block_to_loom_folder(
        &self,
        workspace_id: &str,
        folder_id: &str,
        block_id: &str,
        sort_order: Option<i32>,
    ) -> StorageResult<()> {
        let workspace_id = workspace_id.to_owned();
        let folder_id = folder_id.to_owned();
        let block_id = block_id.to_owned();
        self.storage
            .with_storage_operation(move |database| {
                Box::pin(async move {
                    super::loom_store::add_block_to_loom_folder(
                        &database,
                        &workspace_id,
                        &folder_id,
                        &block_id,
                        sort_order,
                    )
                    .await
                })
            })
            .await
            .map_err(StorageError::from)?
    }

    async fn remove_block_from_loom_folder(
        &self,
        workspace_id: &str,
        folder_id: &str,
        block_id: &str,
    ) -> StorageResult<()> {
        let workspace_id = workspace_id.to_owned();
        let folder_id = folder_id.to_owned();
        let block_id = block_id.to_owned();
        self.storage
            .with_storage_operation(move |database| {
                Box::pin(async move {
                    super::loom_store::remove_block_from_loom_folder(
                        &database,
                        &workspace_id,
                        &folder_id,
                        &block_id,
                    )
                    .await
                })
            })
            .await
            .map_err(StorageError::from)?
    }

    async fn list_loom_folder_blocks(
        &self,
        workspace_id: &str,
        folder_id: &str,
        limit: u32,
        offset: u32,
    ) -> StorageResult<Vec<LoomBlock>> {
        let workspace_id = workspace_id.to_owned();
        let folder_id = folder_id.to_owned();
        self.storage
            .with_storage_operation(move |database| {
                Box::pin(async move {
                    super::loom_store::list_loom_folder_blocks(
                        &database,
                        &workspace_id,
                        &folder_id,
                        limit,
                        offset,
                    )
                    .await
                })
            })
            .await
            .map_err(StorageError::from)?
    }

    async fn create_canvas_board(
        &self,
        ctx: &WriteContext,
        workspace_id: &str,
        block_id: &str,
        board_state: Value,
    ) -> StorageResult<LoomCanvasBoard> {
        super::loom_canvas_store::create_canvas_board(
            &self.storage,
            ctx,
            workspace_id,
            block_id,
            board_state,
        )
        .await
    }

    async fn get_canvas_board(
        &self,
        workspace_id: &str,
        block_id: &str,
    ) -> StorageResult<LoomCanvasBoardView> {
        super::loom_canvas_store::get_canvas_board(&self.storage, workspace_id, block_id).await
    }

    async fn update_canvas_board_state(
        &self,
        ctx: &WriteContext,
        workspace_id: &str,
        block_id: &str,
        board_state: Value,
    ) -> StorageResult<LoomCanvasBoard> {
        super::loom_canvas_store::update_canvas_board_state(
            &self.storage,
            ctx,
            workspace_id,
            block_id,
            board_state,
        )
        .await
    }

    async fn place_block_on_canvas(
        &self,
        ctx: &WriteContext,
        placement: NewLoomCanvasPlacement,
    ) -> StorageResult<LoomCanvasPlacement> {
        super::loom_canvas_store::place_block_on_canvas(&self.storage, ctx, placement).await
    }

    async fn create_stage_canvas_card(
        &self,
        ctx: &WriteContext,
        card: NewLoomCanvasStageCard,
    ) -> StorageResult<LoomCanvasStageCard> {
        super::loom_canvas_store::create_stage_canvas_card(&self.storage, ctx, card).await
    }

    async fn compensate_stage_canvas_card(
        &self,
        ctx: &WriteContext,
        card: CompensateLoomCanvasStageCard,
    ) -> StorageResult<LoomCanvasStageCompensation> {
        super::loom_canvas_store::compensate_stage_canvas_card(&self.storage, ctx, card).await
    }

    async fn update_canvas_placement(
        &self,
        ctx: &WriteContext,
        workspace_id: &str,
        placement_id: &str,
        update: LoomCanvasPlacementUpdate,
    ) -> StorageResult<LoomCanvasPlacement> {
        super::loom_canvas_store::update_canvas_placement(
            &self.storage,
            ctx,
            workspace_id,
            placement_id,
            update,
        )
        .await
    }

    async fn remove_canvas_placement(
        &self,
        ctx: &WriteContext,
        workspace_id: &str,
        placement_id: &str,
    ) -> StorageResult<()> {
        super::loom_canvas_store::remove_canvas_placement(
            &self.storage,
            ctx,
            workspace_id,
            placement_id,
        )
        .await
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
        super::loom_canvas_store::add_canvas_visual_edge(
            &self.storage,
            ctx,
            workspace_id,
            canvas_block_id,
            from_placement_id,
            to_placement_id,
            label,
        )
        .await
    }

    async fn remove_canvas_visual_edge(
        &self,
        ctx: &WriteContext,
        workspace_id: &str,
        visual_edge_id: &str,
    ) -> StorageResult<()> {
        super::loom_canvas_store::remove_canvas_visual_edge(
            &self.storage,
            ctx,
            workspace_id,
            visual_edge_id,
        )
        .await
    }

    async fn create_block_view(
        &self,
        ctx: &WriteContext,
        workspace_id: &str,
        block_id: &str,
        title: Option<String>,
        definition: BlockViewDefinition,
    ) -> StorageResult<BlockViewRecord> {
        let metadata = self.mutation_metadata(ctx, block_id).await?;
        let workspace_id = workspace_id.to_owned();
        let block_id = block_id.to_owned();
        self.storage
            .with_storage_operation(move |database| {
                Box::pin(async move {
                    super::block_view_store::create_block_view(
                        &database,
                        &workspace_id,
                        &block_id,
                        title,
                        definition,
                        metadata,
                    )
                    .await
                })
            })
            .await
            .map_err(StorageError::from)?
    }

    async fn get_block_view(
        &self,
        workspace_id: &str,
        block_id: &str,
    ) -> StorageResult<BlockViewRecord> {
        let workspace_id = workspace_id.to_owned();
        let block_id = block_id.to_owned();
        self.storage
            .with_storage_operation(move |database| {
                Box::pin(async move {
                    super::block_view_store::get_block_view(&database, &workspace_id, &block_id)
                        .await
                })
            })
            .await
            .map_err(StorageError::from)?
    }

    async fn update_block_view_definition(
        &self,
        ctx: &WriteContext,
        workspace_id: &str,
        block_id: &str,
        definition: BlockViewDefinition,
    ) -> StorageResult<BlockViewRecord> {
        let metadata = self.mutation_metadata(ctx, block_id).await?;
        let workspace_id = workspace_id.to_owned();
        let block_id = block_id.to_owned();
        self.storage
            .with_storage_operation(move |database| {
                Box::pin(async move {
                    super::block_view_store::update_block_view_definition(
                        &database,
                        &workspace_id,
                        &block_id,
                        definition,
                        metadata,
                    )
                    .await
                })
            })
            .await
            .map_err(StorageError::from)?
    }

    async fn query_block_view_results(
        &self,
        workspace_id: &str,
        definition: &BlockViewDefinition,
        limit: u32,
        offset: u32,
    ) -> StorageResult<BlockViewResults> {
        let workspace_id = workspace_id.to_owned();
        let definition = definition.clone();
        self.storage
            .with_storage_operation(move |database| {
                Box::pin(async move {
                    super::block_view_store::query_block_view_results(
                        &database,
                        &workspace_id,
                        &definition,
                        limit,
                        offset,
                    )
                    .await
                })
            })
            .await
            .map_err(StorageError::from)?
    }

    async fn compile_loom_wiki_projection(
        &self,
        workspace_id: &str,
        title: &str,
        block_ids: &[String],
    ) -> StorageResult<LoomWikiProjection> {
        super::wiki_store::compile_loom_wiki_projection(self, workspace_id, title, block_ids).await
    }

    async fn get_loom_wiki_projection(
        &self,
        workspace_id: &str,
        projection_id: &str,
    ) -> StorageResult<LoomWikiProjection> {
        super::wiki_store::get_loom_wiki_projection(&self.storage, workspace_id, projection_id)
            .await
    }

    async fn loom_wiki_projection_is_stale(
        &self,
        workspace_id: &str,
        projection_id: &str,
    ) -> StorageResult<bool> {
        super::wiki_store::loom_wiki_projection_is_stale(&self.storage, workspace_id, projection_id)
            .await
    }

    async fn regenerate_loom_wiki_projection(
        &self,
        workspace_id: &str,
        projection_id: &str,
    ) -> StorageResult<LoomWikiProjection> {
        super::wiki_store::regenerate_loom_wiki_projection(self, workspace_id, projection_id).await
    }

    async fn delete_loom_wiki_projection(
        &self,
        workspace_id: &str,
        projection_id: &str,
    ) -> StorageResult<()> {
        super::wiki_store::delete_loom_wiki_projection(&self.storage, workspace_id, projection_id)
            .await
    }

    async fn add_loom_wiki_overlay(
        &self,
        workspace_id: &str,
        projection_id: &str,
        annotation: &str,
        anchor: Option<&str>,
    ) -> StorageResult<LoomWikiOverlay> {
        super::wiki_store::add_loom_wiki_overlay(
            &self.storage,
            workspace_id,
            projection_id,
            annotation,
            anchor,
        )
        .await
    }

    async fn list_loom_wiki_overlays(
        &self,
        workspace_id: &str,
        projection_id: &str,
    ) -> StorageResult<Vec<LoomWikiOverlay>> {
        super::wiki_store::list_loom_wiki_overlays(&self.storage, workspace_id, projection_id).await
    }

    async fn delete_loom_wiki_overlay(
        &self,
        workspace_id: &str,
        overlay_id: &str,
    ) -> StorageResult<()> {
        super::wiki_store::delete_loom_wiki_overlay(&self.storage, workspace_id, overlay_id).await
    }

    async fn import_markdown_to_loom(
        &self,
        ctx: &WriteContext,
        workspace_id: &str,
        title: &str,
        markdown: &str,
    ) -> StorageResult<LoomMarkdownImport> {
        super::wiki_store::import_markdown_to_loom(
            &self.storage,
            ctx,
            workspace_id,
            title,
            markdown,
        )
        .await
    }

    async fn loom_block_breadcrumbs(
        &self,
        workspace_id: &str,
        block_id: &str,
    ) -> StorageResult<LoomBreadcrumbTrail> {
        super::wiki_store::loom_block_breadcrumbs(&self.storage, workspace_id, block_id).await
    }

    async fn append_kernel_events_atomic(
        &self,
        events: Vec<NewKernelEvent>,
    ) -> StorageResult<Vec<KernelEvent>> {
        super::event_ledger::append_atomic(&self.storage, events).await
    }

    async fn append_kernel_event_pair_atomic_with_causation(
        &self,
        first: NewKernelEvent,
        second: NewKernelEvent,
    ) -> StorageResult<Vec<KernelEvent>> {
        super::event_ledger::append_pair_atomic_with_causation(&self.storage, first, second).await
    }

    async fn promote_graph_fact_atomic(
        &self,
        requested: NewKernelEvent,
        accepted: NewKernelEvent,
        fact: knowledge_crdt::NewPromotedFact,
    ) -> StorageResult<knowledge_crdt::PromotedFactRow> {
        super::promotion_store::promote_graph_fact_atomic(&self.storage, requested, accepted, fact)
            .await
    }

    async fn list_kernel_events_for_session(
        &self,
        session_run_id: &str,
    ) -> StorageResult<Vec<KernelEvent>> {
        super::event_ledger::list_for_session(&self.storage, session_run_id).await
    }

    async fn list_kernel_events_for_aggregate(
        &self,
        aggregate_type: &str,
        aggregate_id: &str,
    ) -> StorageResult<Vec<KernelEvent>> {
        super::event_ledger::list_for_aggregate(&self.storage, aggregate_type, aggregate_id).await
    }

    async fn list_pending_native_editor_mirrors(
        &self,
        after_event_sequence: i64,
        limit: i64,
    ) -> StorageResult<Vec<KernelEvent>> {
        super::event_ledger::list_pending_native_editor_mirrors(
            &self.storage,
            after_event_sequence,
            limit,
        )
        .await
    }

    async fn append_kernel_crdt_update(
        &self,
        record: CrdtUpdateRecordV1,
        update_bytes: Vec<u8>,
    ) -> StorageResult<CrdtUpdateRecordV1> {
        super::kernel_crdt_store::append_update(&self.storage, record, update_bytes).await
    }

    async fn list_kernel_crdt_updates(
        &self,
        workspace_id: &str,
        document_id: &str,
        crdt_document_id: &str,
    ) -> StorageResult<Vec<CrdtUpdateRecordV1>> {
        super::kernel_crdt_store::list_updates(
            &self.storage,
            workspace_id,
            document_id,
            crdt_document_id,
        )
        .await
    }

    async fn read_kernel_crdt_update_bytes(
        &self,
        update_bytes_ref: &str,
    ) -> StorageResult<Vec<u8>> {
        super::kernel_crdt_store::read_update_bytes(&self.storage, update_bytes_ref).await
    }

    async fn append_kernel_crdt_snapshot(
        &self,
        record: CrdtSnapshotRecordV1,
        snapshot_bytes: Vec<u8>,
    ) -> StorageResult<CrdtSnapshotRecordV1> {
        super::kernel_crdt_store::append_snapshot(&self.storage, record, snapshot_bytes).await
    }

    async fn list_kernel_crdt_snapshots(
        &self,
        workspace_id: &str,
        document_id: &str,
        crdt_document_id: &str,
    ) -> StorageResult<Vec<CrdtSnapshotRecordV1>> {
        super::kernel_crdt_store::list_snapshots(
            &self.storage,
            workspace_id,
            document_id,
            crdt_document_id,
        )
        .await
    }

    async fn read_kernel_crdt_snapshot_bytes(
        &self,
        snapshot_bytes_ref: &str,
    ) -> StorageResult<Vec<u8>> {
        super::kernel_crdt_store::read_snapshot_bytes(&self.storage, snapshot_bytes_ref).await
    }

    async fn enqueue_kernel_session_run(&self, session: SessionRun) -> StorageResult<SessionRun> {
        super::kernel_queue_store::enqueue(&self.storage, session).await
    }

    async fn enqueue_kernel_session_run_and_record_event(
        &self,
        session: SessionRun,
        causation_id: Option<String>,
        correlation_id: String,
    ) -> StorageResult<(SessionRun, KernelEvent)> {
        super::kernel_queue_store::enqueue_and_record_event(
            &self.storage,
            session,
            causation_id,
            correlation_id,
        )
        .await
    }

    async fn claim_kernel_session_run(
        &self,
        session_run_id: &str,
        claimed_by: &str,
        lease_seconds: i64,
    ) -> StorageResult<Option<KernelSessionLease>> {
        super::kernel_queue_store::claim(&self.storage, session_run_id, claimed_by, lease_seconds)
            .await
    }

    async fn claim_kernel_session_run_and_record_event(
        &self,
        session_run_id: &str,
        claimed_by: &str,
        lease_seconds: i64,
        causation_id: Option<String>,
        correlation_id: String,
    ) -> StorageResult<Option<(KernelSessionLease, KernelEvent)>> {
        super::kernel_queue_store::claim_and_record_event(
            &self.storage,
            session_run_id,
            claimed_by,
            lease_seconds,
            causation_id,
            correlation_id,
        )
        .await
    }

    async fn update_kernel_session_run_state(
        &self,
        session_run_id: &str,
        state: SessionRunState,
    ) -> StorageResult<KernelSessionLease> {
        super::kernel_queue_store::update_state(&self.storage, session_run_id, state).await
    }

    async fn update_kernel_session_run_state_and_record_event(
        &self,
        session_run_id: &str,
        state: SessionRunState,
        causation_id: Option<String>,
        correlation_id: String,
    ) -> StorageResult<(KernelSessionLease, KernelEvent)> {
        super::kernel_queue_store::update_state_and_record_event(
            &self.storage,
            session_run_id,
            state,
            causation_id,
            correlation_id,
        )
        .await
    }

    async fn update_ai_job_mcp_fields(
        &self,
        job_id: Uuid,
        update: AiJobMcpUpdate,
    ) -> StorageResult<()> {
        super::mcp_store::update_ai_job_mcp_fields(&self.storage, job_id, update).await
    }

    async fn get_ai_job_mcp_fields(&self, job_id: Uuid) -> StorageResult<AiJobMcpFields> {
        super::mcp_store::get_ai_job_mcp_fields(&self.storage, job_id).await
    }

    async fn find_ai_job_id_by_mcp_progress_token(
        &self,
        progress_token: &str,
    ) -> StorageResult<Option<Uuid>> {
        super::mcp_store::find_ai_job_id_by_mcp_progress_token(&self.storage, progress_token).await
    }

    async fn create_workflow_run(
        &self,
        job_id: Uuid,
        status: JobState,
        last_heartbeat: Option<DateTime<Utc>>,
    ) -> StorageResult<WorkflowRun> {
        super::workflow_store::create_workflow_run(&self.storage, job_id, status, last_heartbeat)
            .await
    }

    async fn update_workflow_run_status(
        &self,
        run_id: Uuid,
        status: JobState,
        error_message: Option<String>,
    ) -> StorageResult<WorkflowRun> {
        super::workflow_store::update_workflow_run_status(
            &self.storage,
            run_id,
            status,
            error_message,
        )
        .await
    }

    async fn heartbeat_workflow(&self, run_id: Uuid, at: DateTime<Utc>) -> StorageResult<()> {
        super::workflow_store::heartbeat_workflow(&self.storage, run_id, at).await
    }

    async fn create_workflow_node_execution(
        &self,
        exec: NewNodeExecution,
    ) -> StorageResult<WorkflowNodeExecution> {
        super::workflow_store::create_node_execution(&self.storage, exec).await
    }

    async fn update_workflow_node_execution_status(
        &self,
        exec_id: Uuid,
        status: JobState,
        output: Option<Value>,
        error_message: Option<String>,
    ) -> StorageResult<WorkflowNodeExecution> {
        super::workflow_store::update_node_execution_status(
            &self.storage,
            exec_id,
            status,
            output,
            error_message,
        )
        .await
    }

    async fn list_workflow_node_executions(
        &self,
        run_id: Uuid,
    ) -> StorageResult<Vec<WorkflowNodeExecution>> {
        super::workflow_store::list_node_executions(&self.storage, run_id).await
    }

    async fn find_stalled_workflows(&self, threshold_secs: u64) -> StorageResult<Vec<WorkflowRun>> {
        super::workflow_store::find_stalled_workflows(&self.storage, threshold_secs).await
    }

    async fn create_governance_check_run(
        &self,
        ctx: &WriteContext,
        run: NewGovernanceCheckRun,
    ) -> StorageResult<GovernanceCheckRun> {
        let run_id = Uuid::now_v7();
        let metadata = self.mutation_metadata(ctx, &run_id.to_string()).await?;
        super::governance_check_store::create_governance_check_run(
            &self.storage,
            run_id,
            run,
            metadata,
        )
        .await
    }

    async fn list_governance_check_runs(
        &self,
        session_id: Uuid,
    ) -> StorageResult<Vec<GovernanceCheckRun>> {
        super::governance_check_store::list_governance_check_runs(&self.storage, session_id).await
    }

    async fn validate_write_with_guard(
        &self,
        ctx: &WriteContext,
        resource_id: &str,
    ) -> StorageResult<MutationMetadata> {
        self.storage
            .inner
            .guard
            .validate_write(ctx, resource_id)
            .await
            .map_err(StorageError::from)
    }

    async fn prune_ai_jobs(
        &self,
        cutoff: DateTime<Utc>,
        min_versions: u32,
        dry_run: bool,
    ) -> StorageResult<PruneReport> {
        super::ai_job_store::prune(&self.storage, cutoff, min_versions, dry_run).await
    }

    async fn run_migrations(&self) -> StorageResult<()> {
        super::bootstrap_schema(&self.storage)
            .await
            .map(|_| ())
            .map_err(|error| StorageError::Migration(error.to_string()))
    }

    async fn migration_version(&self) -> StorageResult<i64> {
        Ok(super::SCHEMA_REVISION)
    }

    async fn execute_locus_operation(
        &self,
        op: crate::workflows::locus::types::LocusOperation,
    ) -> StorageResult<serde_json::Value> {
        super::locus_store::execute_locus_operation(&self.storage, op).await
    }

    async fn locus_task_board_update_work_packet(
        &self,
        expected_version: i64,
        status: &str,
        task_board_status: &str,
        updated_at: &str,
        metadata: &str,
        wp_id: &str,
    ) -> StorageResult<()> {
        super::locus_store::locus_task_board_update_work_packet(
            &self.storage,
            expected_version,
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
    ) -> StorageResult<Option<StructuredCollabWorkPacketRow>> {
        super::structured_collab_store::work_packet_row(&self.storage, wp_id).await
    }

    async fn structured_collab_work_packet_rows(
        &self,
    ) -> StorageResult<Vec<StructuredCollabWorkPacketRow>> {
        super::structured_collab_store::work_packet_rows(&self.storage).await
    }

    async fn structured_collab_micro_task_status_rows(
        &self,
        wp_id: &str,
    ) -> StorageResult<Vec<(String, String)>> {
        super::structured_collab_store::micro_task_status_rows(&self.storage, wp_id).await
    }

    async fn structured_collab_micro_task_metadata(
        &self,
        wp_id: &str,
        mt_id: &str,
    ) -> StorageResult<Option<String>> {
        super::structured_collab_store::micro_task_metadata(&self.storage, wp_id, mt_id).await
    }

    async fn structured_collab_micro_task_rows(
        &self,
        wp_id: &str,
    ) -> StorageResult<Vec<(String, String)>> {
        super::structured_collab_store::micro_task_rows(&self.storage, wp_id).await
    }

    #[cfg(any(test, feature = "surreal-test-support"))]
    async fn test_overwrite_loom_block_metrics(
        &self,
        workspace_id: &str,
        block_id: &str,
        mention_count: i64,
        tag_count: i64,
        backlink_count: i64,
    ) -> StorageResult<()> {
        super::loom_store::test_overwrite_loom_block_metrics(
            &self.storage,
            workspace_id,
            block_id,
            mention_count,
            tag_count,
            backlink_count,
        )
        .await
    }

    #[cfg(any(test, feature = "surreal-test-support"))]
    async fn test_zero_workspace_loom_metrics(&self, workspace_id: &str) -> StorageResult<()> {
        super::loom_store::test_zero_workspace_loom_metrics(&self.storage, workspace_id).await
    }

    #[cfg(any(test, feature = "test-utils", feature = "surreal-test-support"))]
    async fn test_insert_loom_traversal_perf_fixture(
        &self,
        workspace_id: &str,
        total_blocks: usize,
    ) -> StorageResult<String> {
        super::loom_store::test_insert_loom_traversal_perf_fixture(
            &self.storage,
            workspace_id,
            total_blocks,
        )
        .await
    }

    #[cfg(any(test, feature = "surreal-test-support"))]
    async fn test_update_ai_job_metadata(
        &self,
        job_id: Uuid,
        status: &str,
        created_at: DateTime<Utc>,
        is_pinned: bool,
    ) -> StorageResult<()> {
        super::mcp_store::test_update_ai_job_metadata(
            &self.storage,
            job_id,
            status,
            created_at,
            is_pinned,
        )
        .await
    }

    #[cfg(any(test, feature = "surreal-test-support"))]
    async fn test_fetch_mutation_traceability_row(
        &self,
        table: &str,
        id: &str,
    ) -> StorageResult<MutationTraceabilityRow> {
        super::mcp_store::test_fetch_mutation_traceability_row(&self.storage, table, id).await
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
    const DECLARED_NOT_IMPLEMENTED: usize = 0;

    fn async_method_name(fragment: &str) -> Option<&str> {
        let signature = fragment.trim_start();
        let end = signature.find('(')?;
        let name = signature[..end].trim();
        (!name.is_empty()
            && name
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_'))
        .then_some(name)
    }

    fn method_names(source: &str) -> std::collections::BTreeSet<&str> {
        source
            .lines()
            .filter_map(|line| {
                let signature = line.trim_start();
                signature
                    .strip_prefix("async fn ")
                    .or_else(|| signature.strip_prefix("fn "))
                    .and_then(async_method_name)
            })
            .collect()
    }

    #[test]
    fn not_implemented_surface_is_declared() {
        let module_source = include_str!("../mod.rs");
        let trait_start = module_source
            .find("pub trait Database: Send + Sync {")
            .expect("Database trait declaration must remain discoverable");
        let trait_end = module_source[trait_start..]
            .find("\n}\n\nimpl<T> StorageCapabilityStore")
            .map(|offset| trait_start + offset)
            .expect("Database trait end marker must remain discoverable");
        let trait_source = &module_source[trait_start..trait_end];
        let trait_methods = method_names(trait_source);
        assert_eq!(
            trait_methods.len(),
            201,
            "Database method count changed; re-audit the exact Surreal override surface"
        );

        let module_impl_source = include_str!("database.rs");
        let impl_start = module_impl_source
            .find("impl Database for SurrealDatabase {")
            .expect("SurrealDatabase impl declaration must remain discoverable");
        let impl_end = module_impl_source[impl_start..]
            .find("\n}\n\n#[cfg(test)]\nmod not_implemented_surface")
            .map(|offset| impl_start + offset + 2)
            .expect("SurrealDatabase impl end marker must remain discoverable");
        let implemented = method_names(&module_impl_source[impl_start..impl_end]);
        assert_eq!(
            implemented.len(),
            199,
            "the exact `impl Database for SurrealDatabase` override count changed"
        );

        let inherited_methods = trait_methods
            .difference(&implemented)
            .copied()
            .collect::<Vec<_>>();
        assert_eq!(
            inherited_methods,
            vec!["loom_authority_backend", "update_model_session_state"],
            "only the two audited working Database defaults may remain inherited"
        );

        let needle = concat!("StorageError::", "NotImplemented(");
        let inherited = trait_source
            .split("async fn ")
            .skip(1)
            .filter_map(|fragment| {
                let name = async_method_name(fragment)?;
                fragment.contains(needle).then_some(name)
            })
            .filter(|name| !implemented.contains(name))
            .collect::<Vec<_>>();
        let actual = inherited.len();
        assert_eq!(
            actual, DECLARED_NOT_IMPLEMENTED,
            "the un-ported SurrealDB surface changed: {actual} inherited Database methods still return NotImplemented but {DECLARED_NOT_IMPLEMENTED} were declared; inherited={inherited:?}. Update DECLARED_NOT_IMPLEMENTED in the same change that ports or adds a method so the remaining surface stays visible."
        );
    }
}
