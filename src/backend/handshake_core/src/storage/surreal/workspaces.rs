use surrealdb::types::{Datetime, RecordId, RecordIdKey, SurrealValue};
use uuid::Uuid;

use super::keyed_lock::KeyedLockRegistry;
use super::retry::Replay;
use super::{SurrealDataContext, SurrealDatabase, SurrealStorage, SurrealStorageError};
use crate::storage::fems_memory::{workspace_write_anchor, WorkspaceWriteAnchor};
use crate::storage::{NewWorkspace, StorageError, StorageResult, Workspace, WriteContext};

const WORKSPACES_TABLE: &str = "workspaces";

/// MT-152 race-proof pause point between a read-decide step and its transaction; a no-op
/// outside `race_test_support::with_pause_after_decision`. Called with `first_attempt` so
/// only the FIRST attempt of a retried mutation parks on the two-party barrier: a retried
/// attempt (the loser of the engine-level collision the proof provokes) must not wait for a
/// second party that has already left.
async fn pause_after_decision(first_attempt: bool) {
    #[cfg(any(test, feature = "surreal-test-support"))]
    if first_attempt {
        super::keyed_lock::race_test_support::pause_after_decision().await;
    }
    #[cfg(not(any(test, feature = "surreal-test-support")))]
    let _ = first_attempt;
}

#[derive(SurrealValue)]
struct WorkspaceCreate {
    name: String,
    last_job_id: Option<String>,
    last_workflow_id: Option<String>,
    last_actor_id: Option<String>,
    edit_event_id: String,
    last_actor_kind: String,
}

#[derive(Debug, SurrealValue)]
struct WorkspaceRecord {
    id: RecordId,
    name: String,
    created_at: Datetime,
    updated_at: Datetime,
}

#[derive(SurrealValue)]
struct WorkspaceDeleteBinding {
    workspace: RecordId,
    anchor: WorkspaceWriteAnchor,
}

impl TryFrom<WorkspaceRecord> for Workspace {
    type Error = SurrealStorageError;

    fn try_from(record: WorkspaceRecord) -> Result<Self, Self::Error> {
        if record.id.table.as_str() != WORKSPACES_TABLE {
            return Err(SurrealStorageError::InvalidWorkspaceRecord {
                reason: "record id belongs to a different table",
            });
        }
        let RecordIdKey::String(id) = record.id.key else {
            return Err(SurrealStorageError::InvalidWorkspaceRecord {
                reason: "record id is not a string key",
            });
        };

        Ok(Self {
            id,
            name: record.name,
            created_at: record.created_at.into_inner(),
            updated_at: record.updated_at.into_inner(),
        })
    }
}

impl SurrealDataContext<'_> {
    async fn create_workspace_record(
        &self,
        id: &str,
        content: WorkspaceCreate,
    ) -> Result<Workspace, SurrealStorageError> {
        let created: Option<WorkspaceRecord> = self
            .client
            .create((WORKSPACES_TABLE, id))
            .content(content)
            .await?;
        created
            .ok_or(SurrealStorageError::InvalidWorkspaceRecord {
                reason: "CREATE returned no record",
            })?
            .try_into()
    }

    async fn get_workspace_record(
        &self,
        id: &str,
    ) -> Result<Option<Workspace>, SurrealStorageError> {
        let record: Option<WorkspaceRecord> = self.client.select((WORKSPACES_TABLE, id)).await?;
        record.map(TryInto::try_into).transpose()
    }

    async fn list_workspace_records(&self) -> Result<Vec<Workspace>, SurrealStorageError> {
        let records: Vec<WorkspaceRecord> = self.client.select(WORKSPACES_TABLE).await?;
        // Named explicitly: with only `Vec<_>` the element type is not pinned
        // until the `Ok(workspaces)` at the end, so the `sort_by` closure below
        // has nothing to resolve `left`/`right` against and inference fails.
        let mut workspaces = records
            .into_iter()
            .map(TryInto::try_into)
            .collect::<Result<Vec<Workspace>, SurrealStorageError>>()?;
        workspaces.sort_by(|left, right| {
            left.created_at
                .cmp(&right.created_at)
                .then_with(|| left.id.cmp(&right.id))
        });
        Ok(workspaces)
    }

    async fn delete_workspace_record(&self, id: &str) -> Result<bool, SurrealStorageError> {
        // A workspace owns Loom blocks through ON DELETE CASCADE, while placements and Atelier
        // projections deliberately REJECT deletion of a referenced block. Remove those dependants,
        // then the high-cardinality CASCADE dependants before the blocks. Letting each block discover
        // and cascade its edges and search projection makes large workspace teardown effectively
        // quadratic. The final workspace delete can then cascade the remaining workspace-owned rows
        // atomically.
        //
        // MT-152 I-152-4 (D-146-1 database-side guard): the transaction first UPSERTs and then
        // DELETEs this workspace's `fems_workspace_write_anchors` row, so that key is in its
        // write set whether or not the row existed. Every FEMS transaction that creates a row
        // referencing the workspace UPSERTs the same key, so an overlapping FEMS insert and this
        // delete collide at commit (the pinned engine validates written keys only); the loser's
        // bounded retry re-reads - the insert fails closed on `record::exists`, this delete's
        // cascade now sees the committed row. The removed FEMS process-global mutex used to order the two.
        let deleted = self
            .query_values_at::<WorkspaceRecord, _>(
                "BEGIN TRANSACTION; \
                 UPSERT type::record('fems_workspace_write_anchors', $anchor.key) SET anchor_key = $anchor.key, workspace_key = $anchor.key, claim_nonce = $anchor.nonce, updated_at = time::now() RETURN NONE; \
                 DELETE type::record('fems_workspace_write_anchors', $anchor.key) RETURN NONE; \
                 DELETE atelier_intake_item_loom_projection WHERE workspace_id = $workspace; \
                 DELETE loom_canvas_visual_edges WHERE workspace_id = $workspace; \
                 DELETE loom_canvas_placements WHERE workspace_id = $workspace; \
                 DELETE loom_edges WHERE workspace_id = $workspace; \
                 DELETE loom_block_search_index WHERE workspace_id = $workspace; \
                 DELETE loom_block_view_fr_outbox WHERE workspace_id = $workspace; \
                 DELETE loom_blocks WHERE workspace_id = $workspace; \
                 DELETE $workspace RETURN BEFORE; \
                 COMMIT TRANSACTION;",
                WorkspaceDeleteBinding {
                    workspace: RecordId::new(WORKSPACES_TABLE, id.to_owned()),
                    anchor: workspace_write_anchor(id),
                },
                10,
            )
            .await?;
        Ok(!deleted.is_empty())
    }
}

impl SurrealStorage {
    pub async fn create_workspace(
        &self,
        ctx: &WriteContext,
        workspace: NewWorkspace,
    ) -> StorageResult<Workspace> {
        let id = Uuid::now_v7().to_string();
        let metadata = self
            .inner
            .guard
            .validate_write(ctx, &id)
            .await
            .map_err(StorageError::from)?;
        let content = WorkspaceCreate {
            name: workspace.name,
            last_job_id: metadata.job_id.map(|value| value.to_string()),
            last_workflow_id: metadata.workflow_id.map(|value| value.to_string()),
            last_actor_id: metadata.actor_id,
            edit_event_id: metadata.edit_event_id.to_string(),
            last_actor_kind: metadata.actor_kind.as_str().to_owned(),
        };
        self.with_data_operation(move |database| {
            Box::pin(async move { database.create_workspace_record(&id, content).await })
        })
        .await
        .map_err(map_storage_error)
    }

    pub async fn get_workspace(&self, id: &str) -> StorageResult<Option<Workspace>> {
        let id = id.to_owned();
        self.with_data_operation(move |database| {
            Box::pin(async move { database.get_workspace_record(&id).await })
        })
        .await
        .map_err(map_storage_error)
    }

    pub async fn list_workspaces(&self) -> StorageResult<Vec<Workspace>> {
        self.with_data_operation(|database| {
            Box::pin(async move { database.list_workspace_records().await })
        })
        .await
        .map_err(map_storage_error)
    }

    pub async fn delete_workspace(&self, ctx: &WriteContext, id: &str) -> StorageResult<()> {
        self.inner
            .guard
            .validate_write(ctx, id)
            .await
            .map_err(StorageError::from)?;
        // WP-KERNEL-012 MT-146 D-146-1. SurrealDB gives snapshot isolation with write-write
        // conflict detection only, so a proposal insert racing this delete writes disjoint keys
        // and nothing aborts: the insert's `record::exists` assert reads a stale snapshot that
        // still holds the workspace, while this delete's REFERENCE cascade is a snapshot-bound
        // scan that cannot see the insert's reference key. Both commit and the proposal outlives
        // its workspace. MT-146 ordered the two snapshots with the process-global FEMS mutation
        // lock; MT-152 I-152-4 replaced that with the `fems_workspace_write_anchors` key both
        // transactions write (see `delete_workspace_record`), so the race is decided at commit
        // for any two callers, in-process or not, and the loser converges through the bounded
        // MT-142 retry: this delete re-runs and cascades the committed insert, or the insert
        // re-runs and fails closed with NotFound. No keyed lock is taken (empty key set): a
        // workspace delete is a rare, heavy teardown and must not serialize ordinary writes.
        let replay_key = format!("workspace-delete:{id}");
        let mut attempts = 0u32;
        let deleted =
            SurrealDatabase::with_lock_registry(self.clone(), KeyedLockRegistry::disabled())
                .guarded_mutation(Vec::new(), Replay::idempotent(replay_key), None, || {
                    let owned_id = id.to_owned();
                    attempts += 1;
                    let first_attempt = attempts == 1;
                    async move {
                        pause_after_decision(first_attempt).await;
                        self.with_data_operation(move |database| {
                            Box::pin(
                                async move { database.delete_workspace_record(&owned_id).await },
                            )
                        })
                        .await
                        .map_err(map_storage_error)
                    }
                })
                .await?;
        if !deleted {
            return Err(StorageError::NotFound("workspace"));
        }
        Ok(())
    }
}

fn map_storage_error(error: SurrealStorageError) -> StorageError {
    StorageError::Database(error.to_string())
}
