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
    PreferenceSource, PreferenceValueType, RedactionClass, PREFERENCE_CHANGE_RECEIPT_SCHEMA_ID,
    PREFERENCE_RECORD_SCHEMA_ID,
};
use crate::storage::{StorageError, StorageResult, WriteContext};

const PREFERENCE_RECORDS_TABLE: &str = "preference_records";
const PREFERENCE_CHANGE_RECEIPTS_TABLE: &str = "preference_change_receipts";
const KERNEL_EVENT_LEDGER_TABLE: &str = "kernel_event_ledger";
const PREFERENCE_REVISION_CONFLICT: &str = "HSK-PREFERENCE-REVISION-CONFLICT";
const PREFERENCE_WRITE_MAX_ATTEMPTS: usize = 8;

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
    value_type: String,
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
    expected_before_revision: Option<i64>,
    after_revision: i64,
    old_value: Option<Value>,
    updated_at: Datetime,
    event_id: String,
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
        let value = normalize_preference_value(row.value, &row.value_type)?;
        let default_value = normalize_preference_value(row.default_value, &row.value_type)?;
        Ok(Self {
            schema_id: PREFERENCE_RECORD_SCHEMA_ID.to_owned(),
            preference_id: row.preference_id,
            namespace: row.namespace,
            value_type: row.value_type,
            value,
            scope: row.scope_kind,
            scope_ref: row.scope_ref,
            default_value,
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
        let old_value = row
            .old_value
            .map(|value| normalize_preference_value(value, &row.value_type))
            .transpose()?;
        let new_value = normalize_preference_value(row.new_value, &row.value_type)?;
        Ok(Self {
            schema_id: PREFERENCE_CHANGE_RECEIPT_SCHEMA_ID.to_owned(),
            receipt_id,
            preference_id: row.preference_id,
            scope: row.scope_kind,
            scope_ref: row.scope_ref,
            before_revision: row.before_revision,
            after_revision: row.after_revision,
            old_value,
            new_value,
            source: row.source,
            actor: row.actor,
            event_ledger_event_id,
            created_at: row.created_at.into_inner().to_rfc3339(),
        })
    }
}

fn normalize_preference_value(
    value: Value,
    value_type: &str,
) -> Result<Value, SurrealStorageError> {
    if value_type != PreferenceValueType::Float.as_str() {
        return Ok(value);
    }
    let numeric = value
        .as_f64()
        .and_then(serde_json::Number::from_f64)
        .ok_or(SurrealStorageError::InvalidPreferenceRecord {
            reason: "float preference contains a non-finite or non-numeric value",
        })?;
    Ok(Value::Number(numeric))
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
    format!("{:x}", digest.finalize())
}

impl SurrealDataContext<'_> {
    async fn get_preference_record(
        &self,
        scope: &PreferenceScope,
        entry: &PreferenceSchemaEntry,
    ) -> Result<Option<PreferenceRecord>, SurrealStorageError> {
        let key = preference_record_key(scope, entry.preference_id);
        let row: Option<PreferenceRecordRow> =
            self.client.select((PREFERENCE_RECORDS_TABLE, key)).await?;
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
        let preference_key = preference_record_key(scope, entry.preference_id);
        let aggregate_id = format!(
            "{}:{}:{}",
            scope.kind.as_str(),
            scope.scope_ref,
            entry.preference_id
        );
        let run_id = format!("PREFERENCE-{aggregate_id}");
        let kernel_actor = if actor.trim().is_empty() {
            KernelActor::System("preferences".to_owned())
        } else {
            KernelActor::Operator(actor.to_owned())
        };
        let statement = r#"
BEGIN TRANSACTION;
LET $before = SELECT * FROM ONLY $preference_record;
IF $before = NONE AND $expected_before_revision != NONE {
    THROW 'HSK-PREFERENCE-REVISION-CONFLICT';
};
IF $before != NONE AND ($expected_before_revision = NONE OR $before.revision != $expected_before_revision) {
    THROW 'HSK-PREFERENCE-REVISION-CONFLICT';
};
CREATE ONLY $event_record SET
    event_id = $event_id,
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
    before_revision = $expected_before_revision,
    after_revision = $after_revision,
    old_value = $old_value,
    new_value = $value,
    value_type = $value_type,
    source = $source,
    actor = $actor,
    redaction_class = $redaction_class,
    event_ledger_event_id = $event_record,
    created_at = $updated_at;
COMMIT TRANSACTION;
"#;

        for attempt in 0..PREFERENCE_WRITE_MAX_ATTEMPTS {
            let before = self.get_preference_record(scope, entry).await?;
            let expected_before_revision = before.as_ref().map(|record| record.revision);
            let after_revision = expected_before_revision.unwrap_or(0) + 1;
            let old_value = before.as_ref().map(|record| record.value.clone());
            let receipt_id = Uuid::now_v7().to_string();
            let event_id = format!("KE-{}", Uuid::now_v7());
            let event_receipt = PreferenceChangeReceipt {
                schema_id: PREFERENCE_CHANGE_RECEIPT_SCHEMA_ID.to_owned(),
                receipt_id: receipt_id.clone(),
                preference_id: entry.preference_id.to_owned(),
                scope: scope.kind.as_str().to_owned(),
                scope_ref: scope.scope_ref.clone(),
                before_revision: expected_before_revision,
                after_revision,
                old_value: old_value.clone(),
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
            let event = NewKernelEvent::builder(
                run_id.clone(),
                run_id.clone(),
                KernelEventType::PreferenceRecordChanged,
                kernel_actor.clone(),
            )
            .aggregate("preference_record", aggregate_id.clone())
            .source_component("preferences")
            .payload(payload)
            .build()
            .map_err(|_| SurrealStorageError::InvalidPreferenceRecord {
                reason: "preference change event failed validation",
            })?;
            let bindings = PreferenceWriteBindings {
                preference_record: RecordId::new(PREFERENCE_RECORDS_TABLE, preference_key.clone()),
                receipt_record: RecordId::new(
                    PREFERENCE_CHANGE_RECEIPTS_TABLE,
                    receipt_id.clone(),
                ),
                event_record: RecordId::new(KERNEL_EVENT_LEDGER_TABLE, event_id.clone()),
                preference_id: entry.preference_id.to_owned(),
                scope_kind: scope.kind.as_str().to_owned(),
                scope_ref: scope.scope_ref.clone(),
                namespace: entry.namespace.to_owned(),
                value_type: entry.value_type().as_str().to_owned(),
                value: value.clone(),
                default_value: entry.default_value.clone(),
                source: source.as_str().to_owned(),
                redaction_class: entry.redaction_class.as_str().to_owned(),
                actor: actor.to_owned(),
                expected_before_revision,
                after_revision,
                old_value,
                updated_at: Datetime::from(Utc::now()),
                event_id,
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

            let response = self
                .client
                .query(statement)
                .bind(SurrealValue::into_value(bindings))
                .await?;
            if let Err(error) = response.check() {
                if error.to_string().contains(PREFERENCE_REVISION_CONFLICT)
                    && attempt + 1 < PREFERENCE_WRITE_MAX_ATTEMPTS
                {
                    continue;
                }
                return Err(error.into());
            }

            let record = self.get_preference_record(scope, entry).await?.ok_or(
                SurrealStorageError::InvalidPreferenceRecord {
                    reason: "committed preference record is missing",
                },
            )?;
            let receipt_row: Option<PreferenceReceiptRow> = self
                .client
                .select((PREFERENCE_CHANGE_RECEIPTS_TABLE, receipt_id))
                .await?;
            let receipt = receipt_row
                .ok_or(SurrealStorageError::InvalidPreferenceRecord {
                    reason: "committed preference receipt is missing",
                })?
                .try_into()?;
            return Ok((record, receipt));
        }

        Err(SurrealStorageError::InvalidPreferenceRecord {
            reason: "preference write exhausted revision retries",
        })
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
        let scope_for_query = scope.clone();
        let entry_for_query = entry.clone();
        let result = self
            .with_data_operation(move |database| {
                Box::pin(async move {
                    database
                        .get_preference_record(&scope_for_query, &entry_for_query)
                        .await
                })
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
            Box::pin(async move { database.preference_receipts(&scope, &preference_id).await })
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
