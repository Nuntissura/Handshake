use serde_json::{Map, Value};
use surrealdb::types::{Datetime, RecordId, RecordIdKey, SurrealValue};
use uuid::Uuid;

use super::{SurrealDataContext, SurrealStorage, SurrealStorageError};
use crate::storage::{
    Block, BlockUpdate, MutationMetadata, NewBlock, StorageError, StorageResult, WriteContext,
};

const BLOCKS_TABLE: &str = "blocks";
const DOCUMENTS_TABLE: &str = "documents";

#[derive(SurrealValue)]
struct BlockContent {
    document_id: RecordId,
    kind: String,
    sequence: i64,
    raw_content: String,
    display_content: String,
    derived_content: Value,
    last_job_id: Option<String>,
    last_workflow_id: Option<String>,
    last_actor_id: Option<String>,
    edit_event_id: String,
    last_actor_kind: String,
    sensitivity: Option<String>,
    exportable: Option<bool>,
    created_at: Datetime,
    updated_at: Datetime,
}

#[derive(SurrealValue)]
struct DocumentTraceUpdate {
    last_job_id: Option<String>,
    last_workflow_id: Option<String>,
    last_actor_id: Option<String>,
    edit_event_id: String,
    last_actor_kind: String,
    updated_at: Datetime,
}

#[derive(Debug, SurrealValue)]
struct BlockRecord {
    id: RecordId,
    document_id: RecordId,
    kind: String,
    sequence: i64,
    raw_content: String,
    display_content: String,
    derived_content: Value,
    created_at: Datetime,
    updated_at: Datetime,
    sensitivity: Option<String>,
    exportable: Option<bool>,
}

impl TryFrom<BlockRecord> for Block {
    type Error = SurrealStorageError;

    fn try_from(record: BlockRecord) -> Result<Self, Self::Error> {
        let id = string_record_key(
            record.id,
            BLOCKS_TABLE,
            "block id belongs to a different table",
            "block id is not a string key",
        )?;
        let document_id = string_record_key(
            record.document_id,
            DOCUMENTS_TABLE,
            "document id belongs to a different table",
            "document id is not a string key",
        )?;
        if !record.derived_content.is_object() {
            return Err(SurrealStorageError::InvalidBlockRecord {
                reason: "derived content is not an object",
            });
        }

        Ok(Self {
            id,
            document_id,
            kind: record.kind,
            sequence: record.sequence,
            raw_content: record.raw_content,
            display_content: record.display_content,
            derived_content: record.derived_content,
            created_at: record.created_at.into_inner(),
            updated_at: record.updated_at.into_inner(),
            sensitivity: record.sensitivity,
            exportable: record.exportable,
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
        return Err(SurrealStorageError::InvalidBlockRecord {
            reason: wrong_table_reason,
        });
    }
    let RecordIdKey::String(id) = record_id.key else {
        return Err(SurrealStorageError::InvalidBlockRecord {
            reason: wrong_key_reason,
        });
    };
    Ok(id)
}

impl BlockContent {
    fn from_new(block: NewBlock, document_id: &str, metadata: MutationMetadata) -> Self {
        let display_content = block
            .display_content
            .unwrap_or_else(|| block.raw_content.clone());
        Self {
            document_id: RecordId::new(DOCUMENTS_TABLE, document_id.to_owned()),
            kind: block.kind,
            sequence: block.sequence,
            raw_content: block.raw_content,
            display_content,
            derived_content: block
                .derived_content
                .unwrap_or_else(|| Value::Object(Map::new())),
            last_job_id: metadata.job_id.map(|value| value.to_string()),
            last_workflow_id: metadata.workflow_id.map(|value| value.to_string()),
            last_actor_id: metadata.actor_id,
            edit_event_id: metadata.edit_event_id.to_string(),
            last_actor_kind: metadata.actor_kind.as_str().to_owned(),
            sensitivity: block.sensitivity,
            exportable: block.exportable,
            created_at: Datetime::from(metadata.timestamp),
            updated_at: Datetime::from(metadata.timestamp),
        }
    }
}

impl From<MutationMetadata> for DocumentTraceUpdate {
    fn from(metadata: MutationMetadata) -> Self {
        Self {
            last_job_id: metadata.job_id.map(|value| value.to_string()),
            last_workflow_id: metadata.workflow_id.map(|value| value.to_string()),
            last_actor_id: metadata.actor_id,
            edit_event_id: metadata.edit_event_id.to_string(),
            last_actor_kind: metadata.actor_kind.as_str().to_owned(),
            updated_at: Datetime::from(metadata.timestamp),
        }
    }
}

impl SurrealDataContext<'_> {
    async fn create_block_record(
        &self,
        id: &str,
        content: BlockContent,
    ) -> Result<Block, SurrealStorageError> {
        let created: Option<BlockRecord> = self
            .client
            .create((BLOCKS_TABLE, id))
            .content(content)
            .await?;
        created
            .ok_or(SurrealStorageError::InvalidBlockRecord {
                reason: "CREATE returned no record",
            })?
            .try_into()
    }

    async fn get_block_record(&self, id: &str) -> Result<Option<Block>, SurrealStorageError> {
        let record: Option<BlockRecord> = self.client.select((BLOCKS_TABLE, id)).await?;
        record.map(TryInto::try_into).transpose()
    }

    async fn list_block_records(
        &self,
        document_id: &str,
    ) -> Result<Vec<Block>, SurrealStorageError> {
        let document_id = RecordId::new(DOCUMENTS_TABLE, document_id.to_owned());
        let mut response = self
            .client
            .query(
                "SELECT * FROM blocks WHERE document_id = $document_id ORDER BY sequence ASC, id ASC;",
            )
            .bind(("document_id", document_id))
            .await?
            .check()?;
        let records: Vec<BlockRecord> = response.take(0)?;
        records
            .into_iter()
            .map(TryInto::try_into)
            .collect::<Result<Vec<_>, _>>()
    }

    async fn update_block_record(
        &self,
        id: &str,
        data: BlockUpdate,
        metadata: MutationMetadata,
    ) -> Result<bool, SurrealStorageError> {
        let BlockUpdate {
            kind,
            sequence,
            raw_content,
            display_content,
            derived_content,
        } = data;
        let mut assignments = Vec::with_capacity(11);
        if kind.is_some() {
            assignments.push("kind = $kind");
        }
        if sequence.is_some() {
            assignments.push("sequence = $sequence");
        }
        if raw_content.is_some() {
            assignments.push("raw_content = $raw_content");
            if display_content.is_none() {
                assignments.push(
                    "display_content = IF display_content = '' { $raw_content } ELSE { display_content }",
                );
            }
        }
        if display_content.is_some() {
            assignments.push("display_content = $display_content");
        }
        if derived_content.is_some() {
            assignments.push("derived_content = $derived_content");
        }
        assignments.extend([
            "last_job_id = $last_job_id",
            "last_workflow_id = $last_workflow_id",
            "last_actor_id = $last_actor_id",
            "edit_event_id = $edit_event_id",
            "last_actor_kind = $last_actor_kind",
            "updated_at = $updated_at",
        ]);

        let statement = format!(
            "BEGIN TRANSACTION;\nIF record::exists($block_record) = false {{ THROW 'HSK-SURREAL-BLOCK-NOT-FOUND'; }};\nUPDATE ONLY $block_record SET {} RETURN NONE;\nCOMMIT TRANSACTION;",
            assignments.join(", ")
        );
        let mut query = self
            .client
            .query(statement)
            .bind(("block_record", RecordId::new(BLOCKS_TABLE, id.to_owned())))
            .bind((
                "last_job_id",
                metadata.job_id.map(|value| value.to_string()),
            ))
            .bind((
                "last_workflow_id",
                metadata.workflow_id.map(|value| value.to_string()),
            ))
            .bind(("last_actor_id", metadata.actor_id))
            .bind(("edit_event_id", metadata.edit_event_id.to_string()))
            .bind(("last_actor_kind", metadata.actor_kind.as_str().to_owned()))
            .bind(("updated_at", Datetime::from(metadata.timestamp)));
        if let Some(value) = kind {
            query = query.bind(("kind", value));
        }
        if let Some(value) = sequence {
            query = query.bind(("sequence", value));
        }
        if let Some(value) = raw_content {
            query = query.bind(("raw_content", value));
        }
        if let Some(value) = display_content {
            query = query.bind(("display_content", value));
        }
        if let Some(value) = derived_content {
            query = query.bind(("derived_content", value));
        }

        let response = query.await?;
        match response.check() {
            Ok(_) => Ok(true),
            Err(error) if error.to_string().contains("HSK-SURREAL-BLOCK-NOT-FOUND") => Ok(false),
            Err(error) => Err(error.into()),
        }
    }

    async fn delete_block_record(&self, id: &str) -> Result<bool, SurrealStorageError> {
        let deleted: Option<BlockRecord> = self.client.delete((BLOCKS_TABLE, id)).await?;
        Ok(deleted.is_some())
    }

    async fn replace_block_records(
        &self,
        document_id: &str,
        blocks: Vec<(String, BlockContent)>,
        document_trace: DocumentTraceUpdate,
    ) -> Result<Vec<Block>, SurrealStorageError> {
        let document_record = RecordId::new(DOCUMENTS_TABLE, document_id.to_owned());
        let replacement_count = blocks.len();
        let mut statement = String::from(
            "BEGIN TRANSACTION;\nIF record::exists($document_record) = false { THROW 'HSK-SURREAL-DOCUMENT-NOT-FOUND'; };\nDELETE blocks WHERE document_id = $document_id;\n",
        );
        for index in 0..blocks.len() {
            statement.push_str(&format!(
                "CREATE $block_record_{index} CONTENT $block_content_{index};\n"
            ));
        }
        statement.push_str("UPDATE $document_record MERGE $document_trace;\nRETURN [");
        for index in 0..replacement_count {
            if index > 0 {
                statement.push_str(", ");
            }
            statement.push_str(&format!("(SELECT * FROM ONLY $block_record_{index})"));
        }
        statement.push_str("];\nCOMMIT TRANSACTION;");

        let mut query = self
            .client
            .query(statement)
            .bind(("document_id", document_record.clone()))
            .bind(("document_record", document_record))
            .bind(("document_trace", document_trace));
        for (index, (id, content)) in blocks.into_iter().enumerate() {
            query = query
                .bind((
                    format!("block_record_{index}"),
                    RecordId::new(BLOCKS_TABLE, id),
                ))
                .bind((format!("block_content_{index}"), content));
        }
        let mut response = query.await?.check()?;
        let return_statement_index = replacement_count + 4;
        let records: Vec<BlockRecord> = response.take(return_statement_index)?;
        if records.len() != replacement_count {
            return Err(SurrealStorageError::InvalidBlockRecord {
                reason: "transaction returned an incomplete replacement set",
            });
        }
        records
            .into_iter()
            .map(TryInto::try_into)
            .collect::<Result<Vec<_>, _>>()
    }
}

impl SurrealStorage {
    pub async fn get_blocks(&self, doc_id: &str) -> StorageResult<Vec<Block>> {
        let doc_id = doc_id.to_owned();
        self.with_data_operation(move |database| {
            Box::pin(async move { database.list_block_records(&doc_id).await })
        })
        .await
        .map_err(map_storage_error)
    }

    pub async fn get_block(&self, block_id: &str) -> StorageResult<Block> {
        let block_id = block_id.to_owned();
        let block = self
            .with_data_operation(move |database| {
                Box::pin(async move { database.get_block_record(&block_id).await })
            })
            .await
            .map_err(map_storage_error)?;
        block.ok_or(StorageError::NotFound("block"))
    }

    pub async fn create_block(&self, ctx: &WriteContext, block: NewBlock) -> StorageResult<Block> {
        let id = block
            .id
            .clone()
            .unwrap_or_else(|| Uuid::now_v7().to_string());
        let metadata = self
            .inner
            .guard
            .validate_write(ctx, &id)
            .await
            .map_err(StorageError::from)?;
        let document_id = block.document_id.clone();
        let content = BlockContent::from_new(block, &document_id, metadata);
        self.with_data_operation(move |database| {
            Box::pin(async move { database.create_block_record(&id, content).await })
        })
        .await
        .map_err(map_storage_error)
    }

    pub async fn update_block(
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

        let metadata = self
            .inner
            .guard
            .validate_write(ctx, block_id)
            .await
            .map_err(StorageError::from)?;
        let block_id = block_id.to_owned();
        let updated = self
            .with_data_operation(move |database| {
                Box::pin(async move {
                    database
                        .update_block_record(&block_id, data, metadata)
                        .await
                })
            })
            .await
            .map_err(map_storage_error)?;
        if !updated {
            return Err(StorageError::NotFound("block"));
        }
        Ok(())
    }

    pub async fn delete_block(&self, ctx: &WriteContext, block_id: &str) -> StorageResult<()> {
        self.inner
            .guard
            .validate_write(ctx, block_id)
            .await
            .map_err(StorageError::from)?;
        let block_id = block_id.to_owned();
        let deleted = self
            .with_data_operation(move |database| {
                Box::pin(async move { database.delete_block_record(&block_id).await })
            })
            .await
            .map_err(map_storage_error)?;
        if !deleted {
            return Err(StorageError::NotFound("block"));
        }
        Ok(())
    }

    pub async fn replace_blocks(
        &self,
        ctx: &WriteContext,
        document_id: &str,
        blocks: Vec<NewBlock>,
    ) -> StorageResult<Vec<Block>> {
        if blocks.iter().any(|block| block.document_id != document_id) {
            return Err(StorageError::Validation(
                "block document does not match replacement document",
            ));
        }

        let mut replacements = Vec::with_capacity(blocks.len());
        for block in blocks {
            let id = block
                .id
                .clone()
                .unwrap_or_else(|| Uuid::now_v7().to_string());
            let metadata = self
                .inner
                .guard
                .validate_write(ctx, &id)
                .await
                .map_err(StorageError::from)?;
            replacements.push((id, BlockContent::from_new(block, document_id, metadata)));
        }
        let document_metadata = self
            .inner
            .guard
            .validate_write(ctx, document_id)
            .await
            .map_err(StorageError::from)?;
        let document_trace = DocumentTraceUpdate::from(document_metadata);
        let document_id = document_id.to_owned();
        let result = self
            .with_data_operation(move |database| {
                Box::pin(async move {
                    database
                        .replace_block_records(&document_id, replacements, document_trace)
                        .await
                })
            })
            .await;
        match result {
            Ok(blocks) => Ok(blocks),
            Err(error) if error.to_string().contains("HSK-SURREAL-DOCUMENT-NOT-FOUND") => {
                Err(StorageError::NotFound("document"))
            }
            Err(error) => Err(map_storage_error(error)),
        }
    }
}

fn map_storage_error(error: SurrealStorageError) -> StorageError {
    StorageError::Database(error.to_string())
}
