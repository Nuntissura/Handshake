use surrealdb::types::{Datetime, RecordId, RecordIdKey, SurrealValue};
use uuid::Uuid;

use super::{SurrealDataContext, SurrealStorage, SurrealStorageError};
use crate::storage::{Document, NewDocument, StorageError, StorageResult, WriteContext};

const DOCUMENTS_TABLE: &str = "documents";
const WORKSPACES_TABLE: &str = "workspaces";

#[derive(SurrealValue)]
struct DocumentCreate {
    workspace_id: RecordId,
    title: String,
    last_job_id: Option<String>,
    last_workflow_id: Option<String>,
    last_actor_id: Option<String>,
    edit_event_id: String,
    last_actor_kind: String,
}

#[derive(Debug, SurrealValue)]
struct DocumentRecord {
    id: RecordId,
    workspace_id: RecordId,
    title: String,
    created_at: Datetime,
    updated_at: Datetime,
}

impl TryFrom<DocumentRecord> for Document {
    type Error = SurrealStorageError;

    fn try_from(record: DocumentRecord) -> Result<Self, Self::Error> {
        let id = string_record_key(
            record.id,
            DOCUMENTS_TABLE,
            "document id belongs to a different table",
            "document id is not a string key",
        )?;
        let workspace_id = string_record_key(
            record.workspace_id,
            WORKSPACES_TABLE,
            "workspace id belongs to a different table",
            "workspace id is not a string key",
        )?;

        Ok(Self {
            id,
            workspace_id,
            title: record.title,
            created_at: record.created_at.into_inner(),
            updated_at: record.updated_at.into_inner(),
        })
    }
}

fn string_record_key(
    record_id: RecordId,
    expected_table: &str,
    wrong_table_reason: &'static str,
    wrong_key_reason: &'static str,
) -> Result<String, SurrealStorageError> {
    if record_id.table.as_str() != expected_table {
        return Err(SurrealStorageError::InvalidDocumentRecord {
            reason: wrong_table_reason,
        });
    }
    let RecordIdKey::String(id) = record_id.key else {
        return Err(SurrealStorageError::InvalidDocumentRecord {
            reason: wrong_key_reason,
        });
    };
    Ok(id)
}

impl SurrealDataContext<'_> {
    async fn create_document_record(
        &self,
        id: &str,
        content: DocumentCreate,
    ) -> Result<Document, SurrealStorageError> {
        let created: Option<DocumentRecord> = self
            .client
            .create((DOCUMENTS_TABLE, id))
            .content(content)
            .await?;
        created
            .ok_or(SurrealStorageError::InvalidDocumentRecord {
                reason: "CREATE returned no record",
            })?
            .try_into()
    }

    async fn get_document_record(&self, id: &str) -> Result<Option<Document>, SurrealStorageError> {
        let record: Option<DocumentRecord> = self.client.select((DOCUMENTS_TABLE, id)).await?;
        record.map(TryInto::try_into).transpose()
    }

    async fn list_document_records(
        &self,
        workspace_id: &str,
    ) -> Result<Vec<Document>, SurrealStorageError> {
        let workspace_id = RecordId::new(WORKSPACES_TABLE, workspace_id.to_owned());
        let mut response = self
            .client
            .query(
                "SELECT * FROM documents WHERE workspace_id = $workspace_id ORDER BY created_at ASC;",
            )
            .bind(("workspace_id", workspace_id))
            .await?
            .check()?;
        let records: Vec<DocumentRecord> = response.take(0)?;
        records
            .into_iter()
            .map(TryInto::try_into)
            .collect::<Result<Vec<_>, _>>()
    }

    async fn delete_document_record(&self, id: &str) -> Result<bool, SurrealStorageError> {
        let deleted: Option<DocumentRecord> = self.client.delete((DOCUMENTS_TABLE, id)).await?;
        Ok(deleted.is_some())
    }
}

impl SurrealStorage {
    pub async fn list_documents(&self, workspace_id: &str) -> StorageResult<Vec<Document>> {
        let workspace_id = workspace_id.to_owned();
        self.with_data_operation(move |database| {
            Box::pin(async move { database.list_document_records(&workspace_id).await })
        })
        .await
        .map_err(map_storage_error)
    }

    pub async fn get_document(&self, doc_id: &str) -> StorageResult<Document> {
        let doc_id = doc_id.to_owned();
        let document = self
            .with_data_operation(move |database| {
                Box::pin(async move { database.get_document_record(&doc_id).await })
            })
            .await
            .map_err(map_storage_error)?;
        document.ok_or(StorageError::NotFound("document"))
    }

    pub async fn create_document(
        &self,
        ctx: &WriteContext,
        document: NewDocument,
    ) -> StorageResult<Document> {
        let id = Uuid::now_v7().to_string();
        let metadata = self
            .inner
            .guard
            .validate_write(ctx, &id)
            .await
            .map_err(StorageError::from)?;
        let content = DocumentCreate {
            workspace_id: RecordId::new(WORKSPACES_TABLE, document.workspace_id),
            title: document.title,
            last_job_id: metadata.job_id.map(|value| value.to_string()),
            last_workflow_id: metadata.workflow_id.map(|value| value.to_string()),
            last_actor_id: metadata.actor_id,
            edit_event_id: metadata.edit_event_id.to_string(),
            last_actor_kind: metadata.actor_kind.as_str().to_owned(),
        };
        self.with_data_operation(move |database| {
            Box::pin(async move { database.create_document_record(&id, content).await })
        })
        .await
        .map_err(map_storage_error)
    }

    pub async fn delete_document(&self, ctx: &WriteContext, doc_id: &str) -> StorageResult<()> {
        self.inner
            .guard
            .validate_write(ctx, doc_id)
            .await
            .map_err(StorageError::from)?;
        let doc_id = doc_id.to_owned();
        let deleted = self
            .with_data_operation(move |database| {
                Box::pin(async move { database.delete_document_record(&doc_id).await })
            })
            .await
            .map_err(map_storage_error)?;
        if !deleted {
            return Err(StorageError::NotFound("document"));
        }
        Ok(())
    }
}

fn map_storage_error(error: SurrealStorageError) -> StorageError {
    StorageError::Database(error.to_string())
}
