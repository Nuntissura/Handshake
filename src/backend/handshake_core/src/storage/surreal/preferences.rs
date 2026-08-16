use chrono::Utc;
use serde_json::Value;
use sha2::{Digest, Sha256};
use surrealdb::types::{Datetime, RecordId, RecordIdKey, SurrealValue};
use uuid::Uuid;

use super::{SurrealDataContext, SurrealStorage, SurrealStorageError};
use crate::kernel::{KernelActor, KernelEventType, NewKernelEvent};
use crate::preferences::{
    preference_changed_event_payload, value_hash_ref, PreferenceChangeReceipt,
    PreferenceProjectionRow, PreferenceRecord, PreferenceSchemaEntry, PreferenceScope,
    PreferenceSource, RedactionClass, PREFERENCE_CHANGE_RECEIPT_SCHEMA_ID,
    PREFERENCE_RECORD_SCHEMA_ID,
};
use crate::storage::{StorageError, StorageResult, WriteContext};

const PREFERENCE_RECORDS_TABLE: &str = "preference_records";
const PREFERENCE_CHANGE_RECEIPTS_TABLE: &str = "preference_change_receipts";
const KERNEL_EVENT_LEDGER_TABLE: &str = "kernel_event_ledger";

#[derive(Debug, SurrealValue)]
struct PreferenceRecordRow {
    id: RecordId,
    preference_id: String,
    scope_kind: String,
    scope_ref: String,
    namespace: String,
    value_type: String,
    value: Value,
    default_value: Value,
    source: String,
    redaction_class: String,
    revision: i64,
    updated_by: String,
    event_ledger_event_id: RecordId,
}

#[derive(Debug, SurrealValue)]
struct PreferenceReceiptRow {
    id: RecordId,
    preference_id: String,
    scope_kind: String,
    scope_ref: String,
    before_revision: Option<i64>,
    after_revision: i64,
    old_value: Option<Value>,
    new_value: Value,
    source: String,
    actor: String,
    event_ledger_event_id: RecordId,
    created_at: Datetime,
}

#[derive(SurrealValue)]
struct PreferenceWriteBindings {
    preference_record: RecordId,
    receipt_record: RecordId,
    event_record: RecordId,
    preference_id: String,
    scope_kind: String,
    scope_ref: String,
    namespace: String,
    value_type: String,
    value: Value,
    default_value: Value,
    source: String,
    redaction_class: String,
    actor: String,
    updated_at: Datetime,
    event_version: String,
    kernel_task_run_id: String,
    session_run_id: String,
    aggregate_type: String,
    aggregate_id: String,
    idempotency_key: String,
    event_type: String,
    actor_kind: String,
    actor_id: String,
    causation_id: Option<String>,
    correlation_id: Option<String>,
    payload_hash: String,
    source_component: String,
    payload: Value,
}

impl TryFrom<PreferenceRecordRow> for PreferenceRecord {
    type Error = SurrealStorageError;

    fn try_from(row: PreferenceRecordRow) -> Result<Self, Self::Error> {
        require_string_key(row.id, PREFERENCE_RECORDS_TABLE)?;
        let event_ledger_event_id =
            require_string_key(row.event_ledger_event_id, KERNEL_EVENT_LEDGER_TABLE)?;
        Ok(Self {
            schema_id: PREFERENCE_RECORD_SCHEMA_ID.to_owned(),
            preference_id: row.preference_id,
            namespace: row.namespace,
            value_type: row.value_type,
            value: row.value,
            scope: row.scope_kind,
            scope_ref: row.scope_ref,
            default_value: row.default_value,
            source: row.source,
            revision: row.revision,
            redaction_class: row.redaction_class,
            updated_by: row.updated_by,
            event_ledger_event_id,
        })
    }
}

impl TryFrom<PreferenceReceiptRow> for PreferenceChangeReceipt {
    type Error = SurrealStorageError;

    fn try_from(row: PreferenceReceiptRow) -> Result<Self, Self::Error> {
        let receipt_id = require_string_key(row.id, PREFERENCE_CHANGE_RECEIPTS_TABLE)?;
        let event_ledger_event_id =
            require_string_key(row.event_ledger_event_id, KERNEL_EVENT_LEDGER_TABLE)?;
        Ok(Self {
            schema_id: PREFERENCE_CHANGE_RECEIPT_SCHEMA_ID.to_owned(),
            receipt_id,
            preference_id: row.preference_id,
            scope: row.scope_kind,
            scope_ref: row.scope_ref,
            before_revision: row.before_revision,
            after_revision: row.after_revision,
            old_value: row.old_value,
            new_value: row.new_value,
            source: row.source,
            actor: row.actor,
            event_ledger_event_id,
            created_at: row.created_at.into_inner().to_rfc3339(),
        })
    }
}

fn require_string_key(
    record_id: RecordId,
    expected_table: &'static str,
) -> Result<String, SurrealStorageError> {
    if record_id.table.as_str() != expected_table {
        return Err(SurrealStorageError::InvalidPreferenceRecord {
            reason: "record id belongs to an unexpected table",
        });
    }
    let RecordIdKey::String(id) = record_id.key else {
        return Err(SurrealStorageError::InvalidPreferenceRecord {
            reason: "record id is not a string key",
        });
    };
    Ok(id)
}

fn preference_record_key(scope: &PreferenceScope, preference_id: &str) -> String {
    let mut digest = Sha256::new();
    digest.update(scope.kind.as_str().as_bytes());
    digest.update([0]);
    digest.update(scope.scope_ref.as_bytes());
    digest.update([0]);
    digest.update(preference_id.as_bytes());
    format!("{digest:x}")
}

impl SurrealDataContext<'_> {
    async fn get_preference_record(
        &self,
        scope: &PreferenceScope,
        entry: &PreferenceSchemaEntry,
    ) -> Result<Option<PreferenceRecord>, SurrealStorageError> {
        let key = preference_record_key(scope, entry.preference_id);
        let row: Option<PreferenceRecordRow> = self
            .client
            .select((PREFERENCE_RECORDS_TABLE, key))
            .await?;
        row.map(TryInto::try_into).transpose()
    }

    async fn write_preference_record(
        &self,
        scope: &PreferenceScope,
        entry: &PreferenceSchemaEntry,
        value: Value,
        source: PreferenceSource,
        actor: &str,
    ) -> Result<(PreferenceRecord, PreferenceChangeReceipt), SurrealStorageError> {
        let receipt_id = Uuid::now_v7().to_string();
        let event_id = Uuid::now_v7().to_string();
        let preference_key = preference_record_key(scope, entry.preference_id);
        let aggregate_id = format!(
            "{}:{}:{}",
            scope.kind.as_str(),
            scope.scope_ref,
            entry.preference_id
        );
        let run_id = format!("PREFERENCE-{aggregate_id}");
        let mut event_receipt = PreferenceChangeReceipt {
            schema_id: PREFERENCE_CHANGE_RECEIPT_SCHEMA_ID.to_owned(),
            receipt_id: receipt_id.clone(),
            preference_id: entry.preference_id.to_owned(),
            scope: scope.kind.as_str().to_owned(),
            scope_ref: scope.scope_ref.clone(),
            before_revision: None,
            after_revision: 0,
            old_value: None,
            new_value: value.clone(),
            source: source.as_str().to_owned(),
            actor: actor.to_owned(),
            event_ledger_event_id: event_id.clone(),
            created_at: Utc::now().to_rfc3339(),
        };
        let payload = preference_changed_event_payload(
            &event_receipt,
            entry.redaction_class,
            entry.value_type(),
        );
        let kernel_actor = if actor.trim().is_empty() {
            KernelActor::System("preferences".to_owned())
        } else {
            KernelActor::Operator(actor.to_owned())
        };
        let event = NewKernelEvent::builder(
            run_id.clone(),
            run_id,
            KernelEventType::PreferenceRecordChanged,
            kernel_actor,
        )
        .aggregate("preference_record", aggregate_id)
        .source_component("preferences")
        .payload(payload)
        .build()
        .map_err(|_| SurrealStorageError::InvalidPreferenceRecord {
            reason: "preference change event failed validation",
        })?;

        let now = Datetime::from(Utc::now());
        let bindings = PreferenceWriteBindings {
            preference_record: RecordId::new(PREFERENCE_RECORDS_TABLE, preference_key),
            receipt_record: RecordId::new(PREFERENCE_CHANGE_RECEIPTS_TABLE, receipt_id.clone()),
            event_record: RecordId::new(KERNEL_EVENT_LEDGER_TABLE, event_id),
            preference_id: entry.preference_id.to_owned(),
            scope_kind: scope.kind.as_str().to_owned(),
            scope_ref: scope.scope_ref.clone(),
            namespace: entry.namespace.to_owned(),
            value_type: entry.value_type().as_str().to_owned(),
            value,
            default_value: entry.default_value.clone(),
            source: source.as_str().to_owned(),
            redaction_class: entry.redaction_class.as_str().to_owned(),
            actor: actor.to_owned(),
            updated_at: now,
            event_version: event.event_version,
            kernel_task_run_id: event.kernel_task_run_id,
            session_run_id: event.session_run_id,
            aggregate_type: event.aggregate_type,
            aggregate_id: event.aggregate_id,
            idempotency_key: event.idempotency_key,
            event_type: event.event_type.as_str().to_owned(),
            actor_kind: event.actor.actor_kind().to_owned(),
            actor_id: event.actor.actor_id().to_owned(),
            causation_id: event.causation_id,
            correlation_id: event.correlation_id,
            payload_hash: event.payload_hash,
            source_component: event.source_component,
            payload: event.payload,
        };

        let statement = r#"
BEGIN TRANSACTION;
LET $before = SELECT ONLY value, revision FROM $preference_record;
LET $after_revision = IF $before = NONE { 1 } ELSE { $before.revision + 1 };
CREATE ONLY $event_record SET
    event_version = $event_version,
    kernel_task_run_id = $kernel_task_run_id,
    session_run_id = $session_run_id,
    aggregate_type = $aggregate_type,
    aggregate_id = $aggregate_id,
    idempotency_key = $idempotency_key,
    event_type = $event_type,
    actor_kind = $actor_kind,
    actor_id = $actor_id,
    causation_id = $causation_id,
    correlation_id = $correlation_id,
    payload_hash = $payload_hash,
    source_component = $source_component,
    payload = $payload;
UPSERT ONLY $preference_record SET
    preference_id = $preference_id,
    scope_kind = $scope_kind,
    scope_ref = $scope_ref,
    namespace = $namespace,
    value_type = $value_type,
    value = $value,
    default_value = $default_value,
    source = $source,
    redaction_class = $redaction_class,
    revision = $after_revision,
    updated_at = $updated_at,
    updated_by = $actor,
    event_ledger_event_id = $event_record;
CREATE ONLY $receipt_record SET
    preference_id = $preference_id,
    scope_kind = $scope_kind,
    scope_ref = $scope_ref,
    before_revision = $before.revision,
    after_revision = $after_revision,
    old_value = $before.value,
    new_value = $value,
    value_type = $value_type,
    source = $source,
    actor = $actor,
    redaction_class = $redaction_class,
    event_ledger_event_id = $event_record,
    created_at = $updated_at;
COMMIT TRANSACTION;
"#;
        self.client
            .query(statement)
            .bind(SurrealValue::into_value(bindings))
            .await?
            .check()?;

        let record = self
            .get_preference_record(scope, entry)
            .await?
            .ok_or(SurrealStorageError::InvalidPreferenceRecord {
                reason: "committed preference record is missing",
            })?;
        let receipt_row: Option<PreferenceReceiptRow> = self
            .client
            .select((PREFERENCE_CHANGE_RECEIPTS_TABLE, receipt_id))
            .await?;
        let receipt = receipt_row
            .ok_or(SurrealStorageError::InvalidPreferenceRecord {
                reason: "committed preference receipt is missing",
            })?
            .try_into()?;
        event_receipt.before_revision = receipt.before_revision;
        event_receipt.after_revision = receipt.after_revision;
        Ok((record, receipt))
    }

    async fn preference_receipts(
        &self,
        scope: &PreferenceScope,
        preference_id: &str,
    ) -> Result<Vec<PreferenceChangeReceipt>, SurrealStorageError> {
        let mut response = self
            .client
            .query(
                "SELECT * FROM preference_change_receipts WHERE preference_id = $preference_id AND scope_kind = $scope_kind AND scope_ref = $scope_ref ORDER BY after_revision DESC, id DESC;",
            )
            .bind(("preference_id", preference_id.to_owned()))
            .bind(("scope_kind", scope.kind.as_str().to_owned()))
            .bind(("scope_ref", scope.scope_ref.clone()))
            .await?
            .check()?;
        let rows: Vec<PreferenceReceiptRow> = response.take(0)?;
        rows.into_iter().map(TryInto::try_into).collect()
    }
}

impl SurrealStorage {
    pub async fn preference_get(
        &self,
        scope: &PreferenceScope,
        entry: &PreferenceSchemaEntry,
    ) -> StorageResult<PreferenceRecord> {
        let scope = scope.clone();
        let entry = entry.clone();
        let result = self
            .with_data_operation(move |database| {
                Box::pin(async move { database.get_preference_record(&scope, &entry).await })
            })
            .await
            .map_err(map_storage_error)?;
        Ok(result.unwrap_or_else(|| PreferenceRecord::resolved_default(&entry, &scope)))
    }

    pub async fn preference_set(
        &self,
        scope: &PreferenceScope,
        entry: &PreferenceSchemaEntry,
        value: Value,
        source: PreferenceSource,
        actor: &str,
    ) -> StorageResult<(PreferenceRecord, PreferenceChangeReceipt)> {
        entry
            .validate(&value)
            .map_err(|_| StorageError::Validation("preference value failed registry validation"))?;
        let resource_id = format!(
            "{}:{}:{}",
            scope.kind.as_str(),
            scope.scope_ref,
            entry.preference_id
        );
        self.inner
            .guard
            .validate_write(&WriteContext::human(non_empty_actor(actor)), &resource_id)
            .await
            .map_err(StorageError::from)?;
        let scope = scope.clone();
        let entry = entry.clone();
        let actor = actor.to_owned();
        self.with_data_operation(move |database| {
            Box::pin(async move {
                database
                    .write_preference_record(&scope, &entry, value, source, &actor)
                    .await
            })
        })
        .await
        .map_err(map_storage_error)
    }

    pub async fn preference_reset(
        &self,
        scope: &PreferenceScope,
        entry: &PreferenceSchemaEntry,
        actor: &str,
    ) -> StorageResult<(PreferenceRecord, PreferenceChangeReceipt)> {
        self.preference_set(
            scope,
            entry,
            entry.default_value.clone(),
            PreferenceSource::Operator,
            actor,
        )
        .await
    }

    pub async fn preference_history(
        &self,
        scope: &PreferenceScope,
        preference_id: &str,
    ) -> StorageResult<Vec<PreferenceChangeReceipt>> {
        let scope = scope.clone();
        let preference_id = preference_id.to_owned();
        self.with_data_operation(move |database| {
            Box::pin(async move {
                database
                    .preference_receipts(&scope, &preference_id)
                    .await
            })
        })
        .await
        .map_err(map_storage_error)
    }

    pub async fn preference_projection(
        &self,
        scope: &PreferenceScope,
        entries: &[PreferenceSchemaEntry],
    ) -> StorageResult<Vec<PreferenceProjectionRow>> {
        let mut projection = Vec::with_capacity(entries.len());
        for entry in entries {
            let record = self.preference_get(scope, entry).await?;
            let redacted = record.redaction_class == RedactionClass::NonPublic.as_str();
            projection.push(PreferenceProjectionRow {
                preference_id: record.preference_id,
                namespace: record.namespace,
                scope: record.scope,
                value: if redacted {
                    Value::String(value_hash_ref(&record.value))
                } else {
                    record.value
                },
                default_value: record.default_value,
                source: record.source,
                revision: record.revision,
                redacted,
            });
        }
        Ok(projection)
    }
}

fn non_empty_actor(actor: &str) -> Option<String> {
    let actor = actor.trim();
    (!actor.is_empty()).then(|| actor.to_owned())
}

fn map_storage_error(error: SurrealStorageError) -> StorageError {
    StorageError::Database(error.to_string())
}
