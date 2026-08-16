use surrealdb::types::{Datetime, RecordId, RecordIdKey, SurrealValue};
use uuid::Uuid;

use super::{SurrealDataContext, SurrealStorage, SurrealStorageError};
use crate::storage::{NewWorkspace, StorageError, StorageResult, Workspace, WriteContext};

const WORKSPACES_TABLE: &str = "workspaces";

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
        let mut workspaces = records
            .into_iter()
            .map(TryInto::try_into)
            .collect::<Result<Vec<_>, _>>()?;
        workspaces.sort_by(|left, right| {
            left.created_at
                .cmp(&right.created_at)
                .then_with(|| left.id.cmp(&right.id))
        });
        Ok(workspaces)
    }

    async fn delete_workspace_record(&self, id: &str) -> Result<bool, SurrealStorageError> {
        let deleted: Option<WorkspaceRecord> = self.client.delete((WORKSPACES_TABLE, id)).await?;
        Ok(deleted.is_some())
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
        let owned_id = id.to_owned();
        let deleted = self
            .with_data_operation(move |database| {
                Box::pin(async move { database.delete_workspace_record(&owned_id).await })
            })
            .await
            .map_err(map_storage_error)?;
        if !deleted {
            return Err(StorageError::NotFound("workspace"));
        }
        Ok(())
    }
}

fn map_storage_error(error: SurrealStorageError) -> StorageError {
    StorageError::Database(error.to_string())
}
