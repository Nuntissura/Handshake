use std::future::{Future, IntoFuture};
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
    PreferenceScopeKind, PreferenceSource, PreferenceValueType, RedactionClass,
    PREFERENCE_CHANGE_RECEIPT_SCHEMA_ID, PREFERENCE_RECORD_SCHEMA_ID,
};
use crate::storage::{StorageError, StorageResult, WriteContext};

const PREFERENCE_RECORDS_TABLE: &str = "preference_records";
const PREFERENCE_CHANGE_RECEIPTS_TABLE: &str = "preference_change_receipts";
const KERNEL_EVENT_LEDGER_TABLE: &str = "kernel_event_ledger";
const PREFERENCE_REVISION_CONFLICT: &str = "HSK-PREFERENCE-REVISION-CONFLICT";
const PREFERENCE_WRITE_MAX_ATTEMPTS: usize = 8;

tokio::task_local! {
    static WORK_PROFILE_WRITE_CANCELLATION: tokio_util::sync::CancellationToken;
}

#[cfg(feature = "test-utils")]
tokio::task_local! {
    static WORK_PROFILE_WRITE_PROOF: std::sync::Arc<WorkProfileWriteProof>;
}

#[cfg(feature = "test-utils")]
struct WorkProfileWriteProof {
    dispatched: tokio::sync::Notify,
    responded: tokio::sync::Notify,
    release: tokio::sync::Semaphore,
    rollback: bool,
}

#[cfg(feature = "test-utils")]
impl WorkProfileWriteProof {
    fn new(rollback: bool) -> std::sync::Arc<Self> {
        std::sync::Arc::new(Self { dispatched: tokio::sync::Notify::new(), responded: tokio::sync::Notify::new(),
            release: tokio::sync::Semaphore::new(0), rollback })
    }
}

fn work_profile_write_cancelled() -> bool {
    WORK_PROFILE_WRITE_CANCELLATION.try_with(|token| token.is_cancelled()).unwrap_or(false)
}

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
    wsids: Vec<String>,
    authority_resource_id: Option<RecordId>,
    authority_session_id: Option<RecordId>,
    authority_capability_id: Option<String>,
    authority_action: Option<String>,
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
        expected_revision: Option<i64>,
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
IF array::len((CREATE $event_record SET
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
    payload = $payload,
    wsids = $wsids,
    authority_resource_id = $authority_resource_id,
    authority_session_id = $authority_session_id,
    authority_capability_id = $authority_capability_id,
    authority_action = $authority_action
    RETURN VALUE id)) != 1 {
    THROW 'HSK-403-PROTECTED-RESOURCE';
};
IF array::len((UPSERT $preference_record SET
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
    event_ledger_event_id = $event_record
    RETURN VALUE id)) != 1 {
    THROW 'HSK-403-PROTECTED-RESOURCE';
};
IF array::len((CREATE $receipt_record SET
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
    created_at = $updated_at
    RETURN VALUE id)) != 1 {
    THROW 'HSK-403-PROTECTED-RESOURCE';
};
COMMIT TRANSACTION;
"#;

        for attempt in 0..PREFERENCE_WRITE_MAX_ATTEMPTS {
            let before = self.get_preference_record(scope, entry).await?;
            let expected_before_revision = before.as_ref().map(|record| record.revision);
            if expected_revision.is_some_and(|revision| revision != expected_before_revision.unwrap_or(0)) {
                return Err(SurrealStorageError::InvalidPreferenceRecord { reason: "HSK-PREFERENCE-REVISION-CONFLICT" });
            }
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
            let mut payload = preference_changed_event_payload(
                &event_receipt,
                entry.redaction_class,
                entry.value_type(),
            );
            // MT-154 AC-154-5: a workspace-scope receipt names its workspace so
            // `fn::mt154_workspace_receipt` can bind it to `wsids[0]` and the workspace grant.
            if scope.kind == PreferenceScopeKind::Workspace {
                if let Some(object) = payload.as_object_mut() {
                    object.insert(
                        "workspace_id".to_owned(),
                        Value::String(scope.scope_ref.clone()),
                    );
                }
            }
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
            let (_, ledger_write) =
                super::event_ledger::prepare_event(event.clone()).map_err(|_| {
                    SurrealStorageError::InvalidPreferenceRecord {
                        reason: "preference change event failed validation",
                    }
                })?;
            let bindings = PreferenceWriteBindings {
                preference_record: RecordId::new(PREFERENCE_RECORDS_TABLE, preference_key.clone()),
                receipt_record: RecordId::new(PREFERENCE_CHANGE_RECEIPTS_TABLE, receipt_id.clone()),
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
                wsids: ledger_write.wsids,
                authority_resource_id: ledger_write.authority_resource_id,
                authority_session_id: ledger_write.authority_session_id,
                authority_capability_id: ledger_write.authority_capability_id,
                authority_action: ledger_write.authority_action,
            };

            if work_profile_write_cancelled() {
                return Err(SurrealStorageError::TransactionCancelled);
            }
            let owned = WORK_PROFILE_WRITE_CANCELLATION.try_with(|_| ()).is_ok();
            if owned { super::authority_mutation_submitted(); }
            #[cfg(feature = "test-utils")]
            let proof = WORK_PROFILE_WRITE_PROOF.try_with(std::sync::Arc::clone).ok();
            #[cfg(feature = "test-utils")]
            let statement = match &proof {
                Some(proof) => statement.replace("LET $before =", if proof.rollback {
                    "SLEEP 200ms; THROW 'MT196-ROLLBACK'; LET $before ="
                } else { "SLEEP 200ms; LET $before =" }),
                None => statement.to_owned(),
            };
            let query = self.client.query(statement).bind(SurrealValue::into_value(bindings)).into_future();
            tokio::pin!(query);
            let response = futures::future::poll_fn(|cx| {
                let result = query.as_mut().poll(cx);
                #[cfg(feature = "test-utils")]
                if result.is_pending() {
                    if let Some(proof) = &proof { proof.dispatched.notify_one(); }
                }
                result
            }).await;
            #[cfg(feature = "test-utils")]
            if let Some(proof) = &proof {
                proof.responded.notify_one();
                if let Ok(permit) = proof.release.acquire().await { permit.forget(); }
            }
            let mut response = response.map_err(|error| if owned { super::authority_mutation_error(error) } else { error })?;
            // A THROW aborts the transaction and every earlier slot then reads "The query was not
            // executed due to a failed transaction"; `check()` returns that first slot and hides
            // the THROWn code (HSK-403 silent-deny guard, revision conflict). Pick the meaningful
            // error, as `decode_query_values` does.
            let mut errors = response.take_errors().into_iter().collect::<Vec<_>>();
            errors.sort_by_key(|(statement_index, _)| *statement_index);
            let meaningful = errors
                .iter()
                .position(|(_, error)| {
                    !error
                        .to_string()
                        .to_ascii_lowercase()
                        .contains("query was not executed due to a failed transaction")
                })
                .unwrap_or(0);
            if !errors.is_empty() {
                let (_, error) = errors.swap_remove(meaningful);
                let error = if owned { super::authority_mutation_error(error) } else { error };
                if error.to_string().contains(PREFERENCE_REVISION_CONFLICT)
                    && expected_revision.is_none() && attempt + 1 < PREFERENCE_WRITE_MAX_ATTEMPTS
                {
                    continue;
                }
                return Err(error.into());
            }

            // The response proves this inline transaction committed, independently
            // of the fallible post-commit record/receipt reads.
            if owned { super::authority_mutation_settled(); }
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

fn preference_write_operation(
    scope: PreferenceScope, entry: PreferenceSchemaEntry, value: Value, source: PreferenceSource,
    actor: String, expected_revision: Option<i64>, special: bool,
) -> impl for<'a> FnOnce(SurrealDataContext<'a>) -> super::SurrealOperation<'a, (PreferenceRecord, PreferenceChangeReceipt)> + Send {
    move |database| {
            Box::pin(async move {
                if special {
                    let identity = database.work_profile_identity(&scope.scope_ref).await?;
                    if identity.actor_id != actor { return Err(invalid_authority("work profile actor does not match authenticated session")); }
                    if entry.preference_id == crate::preferences::PREF_WORK_PROFILE_SELECTION && value != entry.default_value {
                        let selection: crate::preferences::WorkProfileSelectionV1 = serde_json::from_value(value.clone()).map_err(|_| invalid_authority("invalid selection"))?;
                        let registry_entry = crate::preferences::work_profile_preference_entry(crate::preferences::PREF_WORK_PROFILE_REGISTRY).unwrap();
                        let record = database.get_preference_record(&scope, &registry_entry).await?.ok_or_else(|| invalid_authority("work profile registry unconfigured"))?;
                        let registry: crate::kernel::work_profiles::WorkProfileRegistryV1 = serde_json::from_value(record.value).map_err(|_| invalid_authority("invalid registry"))?;
                        if record.revision != selection.registry_revision || registry.selected_profile_id != selection.profile_id
                            || !registry.profiles.iter().any(|profile| profile.profile_id == selection.profile_id && profile.profile_version == selection.profile_version
                                && profile.role_routes.iter().any(|route| route.role_id == selection.role_id)) {
                            return Err(invalid_authority("HSK-PREFERENCE-REVISION-CONFLICT"));
                        }
                    }
                }
                database.write_preference_record(&scope, &entry, value, source, &actor, expected_revision).await
            })
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
        &self, scope: &PreferenceScope, entry: &PreferenceSchemaEntry, value: Value,
        source: PreferenceSource, actor: &str,
    ) -> StorageResult<(PreferenceRecord, PreferenceChangeReceipt)> {
        self.preference_set_expected(scope, entry, value, source, actor, None).await
    }

    pub async fn preference_set_expected(
        &self, scope: &PreferenceScope, entry: &PreferenceSchemaEntry, value: Value,
        source: PreferenceSource, actor: &str, expected_revision: Option<i64>,
    ) -> StorageResult<(PreferenceRecord, PreferenceChangeReceipt)> {
        use crate::preferences::{is_work_profile_preference, work_profile_preference_entry};
        let special = is_work_profile_preference(entry.preference_id);
        let canonical_entry = if special { work_profile_preference_entry(entry.preference_id).unwrap() } else { entry.clone() };
        let entry = &canonical_entry;
        if special && (scope.kind != PreferenceScopeKind::Workspace || scope.scope_ref.trim().is_empty()
            || expected_revision.is_none() || expected_revision.is_some_and(|revision| revision < 0)) {
            return Err(StorageError::Validation("work profiles require workspace scope and expected revision"));
        }
        if !(special && value == entry.default_value) {
            entry.validate(&value).map_err(|_| StorageError::Validation("preference value failed registry validation"))?;
        }
        let resource_id = format!("{}:{}:{}", scope.kind.as_str(), scope.scope_ref, entry.preference_id);
        self.inner.guard.validate_write(&WriteContext::human(non_empty_actor(actor)), &resource_id)
            .await.map_err(StorageError::from)?;
        let scope = scope.clone();
        let entry = entry.clone();
        let actor = actor.to_owned();
        let workspace = scope.scope_ref.clone();
        let operation = preference_write_operation(scope, entry, value, source, actor, expected_revision, special);
        if !special {
            return self.with_data_operation(operation).await.map_err(map_storage_error);
        }
        let cancellation = tokio_util::sync::CancellationToken::new();
        let _cancel_on_drop = cancellation.clone().drop_guard();
        let timeout = self.effective_statement_timeout();
        if timeout.is_zero() {
            return Err(map_storage_error(SurrealStorageError::StatementBudgetExhausted { budget_ms: self.config().statement_timeout().as_millis() }));
        }
        #[cfg(feature = "test-utils")]
        let proof = WORK_PROFILE_WRITE_PROOF.try_with(std::sync::Arc::clone).ok();
        let owned = self.with_owned_authority_mutation(move |storage| async move {
            let gate = tokio::select! {
                biased;
                _ = cancellation.cancelled() => return Err(SurrealStorageError::TransactionCancelled),
                gate = storage.inner.work_profile_authority_gate.acquire(super::keyed_lock::LockKey::workspace(&workspace)) => gate,
            };
            if gate.is_noop() { return Err(invalid_authority("work profile authority gate unavailable")); }
            if cancellation.is_cancelled() { return Err(SurrealStorageError::TransactionCancelled); }
            let operation = WORK_PROFILE_WRITE_CANCELLATION.scope(cancellation, storage.with_data_operation_until_settled(operation));
            #[cfg(feature = "test-utils")]
            if let Some(proof) = proof { return WORK_PROFILE_WRITE_PROOF.scope(proof, operation).await; }
            operation.await
        });
        match tokio::time::timeout(timeout, owned).await {
            Ok(result) => result.map_err(map_storage_error)?.map_err(map_storage_error),
            Err(_) => Err(map_storage_error(SurrealStorageError::StatementTimeout { waited_ms: timeout.as_millis() })),
        }
    }

    pub async fn preference_reset(
        &self, scope: &PreferenceScope, entry: &PreferenceSchemaEntry, actor: &str,
    ) -> StorageResult<(PreferenceRecord, PreferenceChangeReceipt)> {
        self.preference_reset_expected(scope, entry, actor, None).await
    }

    pub async fn preference_reset_expected(
        &self, scope: &PreferenceScope, entry: &PreferenceSchemaEntry, actor: &str, expected_revision: Option<i64>,
    ) -> StorageResult<(PreferenceRecord, PreferenceChangeReceipt)> {
        let entry = crate::preferences::work_profile_preference_entry(entry.preference_id).unwrap_or_else(|| entry.clone());
        let source = if crate::preferences::is_work_profile_preference(entry.preference_id) { PreferenceSource::Default } else { PreferenceSource::Operator };
        self.preference_set_expected(scope, &entry, entry.default_value.clone(), source, actor, expected_revision).await
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
    let rendered = error.to_string();
    // MT-154 silent-deny ruling: a record-user write the table permissions dropped THROWs the
    // protected-resource code inside the write transaction; surface it as the constant denial.
    if rendered.contains("HSK-403-PROTECTED-RESOURCE") {
        return StorageError::Guard("HSK-403-PROTECTED-RESOURCE");
    }
    StorageError::Database(rendered)
}

#[cfg(test)]
mod mt154_account_preference_tests {
    //! MT-154 (D-154-3; Master Spec 02-system-architecture.md:2740/:2751/:2773): a global preference
    //! record is an account-owned ProtectedResource. Written as account A's record user it is stamped
    //! with A's owner_account_id, its PREFERENCE_RECORD_CHANGED account receipt carries A's session
    //! principal, and account B can neither read the row, its history or its receipt, nor overwrite it.
    use super::super::resource_authority::{ProvisionedPrincipal, RecordUserScope, ResourceAction};
    use super::SurrealStorage;
    use crate::preferences::{
        lookup_editor_preference, PreferenceScope, PreferenceScopeKind, PreferenceSource,
        PREF_EDITOR_TAB_SIZE,
    };
    use crate::storage::StorageError;
    use serde_json::json;

    const BINDING: &str = "mt154-global-preference-binding";

    async fn principal(storage: &SurrealStorage, key: &str) -> ProvisionedPrincipal {
        storage
            .provision_principal(
                &format!("mt154-pref-account-{key}"),
                &format!("mt154-pref-principal-{key}"),
                "human_account",
                &format!("mt154-pref-actor-{key}"),
                "Operator",
                &["fs.read".to_owned(), "fs.write".to_owned()],
                &format!("mt154-pref-space-{key}"),
                Some(BINDING),
                std::time::Duration::from_secs(3600),
            )
            .await
            .expect("provision MT-154 preference principal")
    }

    /// The account-global record-user scope the account routes build
    /// (`api::authority::authorize_account_session`).
    fn account_scope(principal: &ProvisionedPrincipal) -> RecordUserScope {
        RecordUserScope {
            grant_id: None,
            workspace_id: None,
            session_token: principal.session.token.clone(),
            channel_binding_hash: Some(BINDING.to_owned()),
            resource_id: principal.identity.account_id.clone(),
            session_id: principal.session.session_id.clone(),
            capability_id: "fs.write".to_owned(),
            action: ResourceAction::Update,
        }
    }

    async fn body(storage: &SurrealStorage) {
        let a = principal(storage, "a").await;
        let b = principal(storage, "b").await;
        assert_ne!(a.identity.account_id, b.identity.account_id);
        let entry = lookup_editor_preference(PREF_EDITOR_TAB_SIZE).expect("tab-size entry");
        let global = PreferenceScope {
            kind: PreferenceScopeKind::Global,
            scope_ref: String::new(),
        };

        // Owner positive: A writes the global preference as its record user.
        let (record, receipt) = storage
            .with_record_user_scope(
                account_scope(&a),
                storage.preference_set(
                    &global,
                    &entry,
                    json!(2),
                    PreferenceSource::Operator,
                    "mt154-pref-actor-a",
                ),
            )
            .await
            .expect("account A writes its global preference");
        assert_eq!(record.value, json!(2));
        assert_eq!(record.scope, "global");
        assert_eq!(receipt.actor, "mt154-pref-actor-a");

        // Canonical re-read as A, plus the stamped owner read at the storage boundary.
        let reread = storage
            .with_record_user_scope(account_scope(&a), storage.preference_get(&global, &entry))
            .await
            .expect("A re-reads its global preference");
        assert_eq!(reread.value, json!(2));
        assert_eq!(reread.revision, 1);
        let owners: Vec<String> = storage
            .with_data_operation(|ctx| {
                Box::pin(async move {
                    ctx.query_values::<String, ()>(
                        "RETURN (SELECT VALUE record::id(owner_account_id) FROM preference_records \
                         WHERE scope_kind = 'global');",
                        (),
                    )
                    .await
                })
            })
            .await
            .expect("inspect the stamped owner");
        assert_eq!(owners, vec![a.identity.account_id.clone()]);

        // The account receipt: A reads it, B reads 0 receipts.
        let aggregate_id = format!("global::{PREF_EDITOR_TAB_SIZE}");
        let a_events = storage
            .list_account_kernel_events_for_aggregate(
                &a.session.token,
                BINDING,
                "preference_record",
                &aggregate_id,
            )
            .await
            .expect("A lists its preference receipts");
        assert_eq!(a_events.len(), 1, "owner sees its account receipt");
        assert_eq!(a_events[0].event_id, receipt.event_ledger_event_id);
        assert_eq!(
            a_events[0].actor,
            crate::kernel::KernelActor::Operator("mt154-pref-actor-a".to_owned())
        );
        let b_events = storage
            .list_account_kernel_events_for_aggregate(
                &b.session.token,
                BINDING,
                "preference_record",
                &aggregate_id,
            )
            .await
            .expect("B lists preference receipts");
        assert!(b_events.is_empty(), "another account reads 0 receipts");

        // Cross-account: B sees the registry default and an empty history, never A's value.
        let b_view = storage
            .with_record_user_scope(account_scope(&b), storage.preference_get(&global, &entry))
            .await
            .expect("B reads the global preference");
        assert_eq!(b_view.value, entry.default_value);
        assert_eq!(b_view.revision, 0);
        let b_history = storage
            .with_record_user_scope(
                account_scope(&b),
                storage.preference_history(&global, PREF_EDITOR_TAB_SIZE),
            )
            .await
            .expect("B reads the global preference history");
        assert!(b_history.is_empty(), "B sees none of A's receipts");

        // B cannot overwrite A's row: the dropped write is the constant denial, never a silent 200.
        let b_write = storage
            .with_record_user_scope(
                account_scope(&b),
                storage.preference_set(
                    &global,
                    &entry,
                    json!(8),
                    PreferenceSource::Operator,
                    "mt154-pref-actor-b",
                ),
            )
            .await;
        assert!(
            matches!(
                b_write,
                Err(StorageError::Guard("HSK-403-PROTECTED-RESOURCE"))
            ),
            "cross-account overwrite must be the constant denial, got {b_write:?}"
        );
        let after = storage
            .with_record_user_scope(account_scope(&a), storage.preference_get(&global, &entry))
            .await
            .expect("A re-reads after B's denied write");
        assert_eq!(after.value, json!(2));
        assert_eq!(after.revision, 1);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn mt154_global_preference_is_private_to_the_owning_account() {
        let backend = crate::storage::tests::embedded_test_backend()
            .await
            .expect("MT-154 global preference proof requires an isolated embedded store");
        let storage = backend.storage.clone();
        let outcome = tokio::spawn(async move { body(&storage).await }).await;
        backend
            .close_and_remove()
            .await
            .expect("remove the MT-154 global preference store");
        if let Err(panic) = outcome {
            std::panic::resume_unwind(panic.into_panic());
        }
    }
}


fn invalid_authority(reason: &'static str) -> SurrealStorageError {
    SurrealStorageError::InvalidPreferenceRecord { reason }
}

#[derive(SurrealValue)]
struct WorkProfileIdentity { owner_account_id: String, actor_principal_id: String, actor_id: String }

impl SurrealDataContext<'_> {
    async fn work_profile_identity(&self, workspace_id: &str) -> Result<WorkProfileIdentity, SurrealStorageError> {
        let scope = super::current_record_user_scope().ok_or_else(|| invalid_authority("authenticated workspace scope required"))?;
        if scope.workspace_id.as_deref() != Some(workspace_id) { return Err(invalid_authority("workspace scope mismatch")); }
        let mut result = self.client.query("SELECT record::id(account_id) AS owner_account_id, record::id(principal_id) AS actor_principal_id, principal_id.actor_id AS actor_id FROM authenticated_sessions WHERE id = $auth.id AND id = $session LIMIT 2;")
            .bind(("session", RecordId::new("authenticated_sessions", scope.session_id))).await?.check()?;
        let mut rows: Vec<WorkProfileIdentity> = result.take(0)?;
        if rows.len() != 1 { return Err(invalid_authority("authenticated identity missing or ambiguous")); }
        Ok(rows.remove(0))
    }

    async fn verified_work_profile_receipt(&self, record: &PreferenceRecord, identity: &WorkProfileIdentity) -> Result<PreferenceChangeReceipt, SurrealStorageError> {
        let scope = PreferenceScope::workspace(&record.scope_ref);
        let receipts = self.preference_receipts(&scope, &record.preference_id).await?;
        let mut matching = receipts.into_iter().filter(|receipt| receipt.after_revision == record.revision);
        let receipt = matching.next().ok_or_else(|| invalid_authority("canonical work profile receipt missing"))?;
        if matching.next().is_some() || receipt.preference_id != record.preference_id || receipt.scope != "workspace"
            || receipt.scope_ref != record.scope_ref || receipt.new_value != record.value || receipt.actor != record.updated_by
            || receipt.source != record.source || receipt.event_ledger_event_id != record.event_ledger_event_id
            || receipt.before_revision.unwrap_or(0) != record.revision - 1 {
            return Err(invalid_authority("canonical work profile receipt mismatch"));
        }
        let mut response = self.client.query(r#"
RETURN array::len((SELECT id FROM preference_records WHERE id = $record AND owner_account_id = $owner)) = 1
    AND array::len((SELECT id FROM preference_change_receipts WHERE id = $receipt AND owner_account_id = $owner)) = 1
    AND array::len((SELECT id FROM kernel_event_ledger WHERE id = $event AND event_id = $event_id
        AND event_type = 'PREFERENCE_RECORD_CHANGED' AND source_component = 'preferences' AND actor_kind = 'operator' AND actor_id = $actor
        AND authority_session_id.account_id = $owner AND actor_id = authority_session_id.principal_id.actor_id
        AND aggregate_type = 'preference_record' AND aggregate_id = $aggregate
        AND wsids = [$workspace] AND payload.workspace_id = $workspace AND payload.receipt_id = $receipt_id
        AND payload.preference_id = $preference_id AND payload.scope = 'workspace' AND payload.scope_ref = $workspace
        AND payload.actor = $actor AND payload.revision = $revision AND payload.source = $source
        AND payload.before_revision = $before_revision
        AND payload.new_value_ref = $value)) = 1;
"#).bind(("record", RecordId::new(PREFERENCE_RECORDS_TABLE, preference_record_key(&scope, &record.preference_id))))
            .bind(("receipt", RecordId::new(PREFERENCE_CHANGE_RECEIPTS_TABLE, receipt.receipt_id.clone())))
            .bind(("owner", RecordId::new("local_accounts", identity.owner_account_id.clone())))
            .bind(("event", RecordId::new(KERNEL_EVENT_LEDGER_TABLE, receipt.event_ledger_event_id.clone())))
            .bind(("event_id", receipt.event_ledger_event_id.clone())).bind(("actor", receipt.actor.clone()))
            .bind(("aggregate", format!("workspace:{}:{}", record.scope_ref, record.preference_id)))
            .bind(("workspace", record.scope_ref.clone())).bind(("receipt_id", receipt.receipt_id.clone()))
            .bind(("preference_id", record.preference_id.clone())).bind(("revision", record.revision))
            .bind(("source", record.source.clone())).bind(("before_revision", serde_json::json!(receipt.before_revision)))
            .bind(("value", record.value.clone())).await?.check()?;
        let valid: Option<bool> = response.take(0)?;
        if valid != Some(true) { return Err(invalid_authority("canonical work profile event or ownership mismatch")); }
        Ok(receipt)
    }
}

impl SurrealStorage {
    pub async fn local_execution_authority_snapshot(&self, request: &crate::preferences::LocalExecutionAuthorityRequest)
        -> StorageResult<crate::preferences::LocalExecutionAuthorityLease> {
        use crate::preferences::*;
        if request.workspace_id.trim().is_empty() || request.expected_registry_revision <= 0 || request.expected_selection_revision <= 0 {
            return Err(StorageError::Validation("configured work profile revisions required"));
        }
        let publication = self.authority_publication_read().await.map_err(map_storage_error)?;
        let gate = self.inner.work_profile_authority_gate.acquire(super::keyed_lock::LockKey::workspace(&request.workspace_id)).await;
        if gate.is_noop() { return Err(StorageError::Validation("work profile authority gate unavailable")); }
        let request = request.clone();
        let snapshot = self.with_data_operation(move |database| Box::pin(async move {
            let identity = database.work_profile_identity(&request.workspace_id).await?;
            let scope = PreferenceScope::workspace(&request.workspace_id);
            let registry_entry = work_profile_preference_entry(PREF_WORK_PROFILE_REGISTRY).unwrap();
            let selection_entry = work_profile_preference_entry(PREF_WORK_PROFILE_SELECTION).unwrap();
            let registry_record = database.get_preference_record(&scope, &registry_entry).await?.ok_or_else(|| invalid_authority("work profile registry unconfigured"))?;
            let selection_record = database.get_preference_record(&scope, &selection_entry).await?.ok_or_else(|| invalid_authority("work profile selection unconfigured"))?;
            for (record, entry, revision) in [(&registry_record, &registry_entry, request.expected_registry_revision), (&selection_record, &selection_entry, request.expected_selection_revision)] {
                if record.revision != revision || record.source != PreferenceSource::Operator.as_str() || record.scope != "workspace"
                    || record.scope_ref != request.workspace_id || record.preference_id != entry.preference_id
                    || record.namespace != entry.namespace || record.value_type != entry.value_type().as_str() || record.value == entry.default_value {
                    return Err(invalid_authority("work profile canonical record mismatch"));
                }
                entry.validate(&record.value).map_err(|_| invalid_authority("work profile domain validation failed"))?;
            }
            let registry: crate::kernel::work_profiles::WorkProfileRegistryV1 = serde_json::from_value(registry_record.value.clone()).map_err(|_| invalid_authority("invalid registry"))?;
            let selection: WorkProfileSelectionV1 = serde_json::from_value(selection_record.value.clone()).map_err(|_| invalid_authority("invalid selection"))?;
            if selection.registry_revision != registry_record.revision || registry.selected_profile_id != selection.profile_id
                || !registry.profiles.iter().any(|profile| profile.profile_id == selection.profile_id && profile.profile_version == selection.profile_version
                    && profile.role_routes.iter().any(|route| route.role_id == selection.role_id)) {
                return Err(invalid_authority("work profile selection is stale or route missing"));
            }
            let registry_receipt = database.verified_work_profile_receipt(&registry_record, &identity).await?;
            let selection_receipt = database.verified_work_profile_receipt(&selection_record, &identity).await?;
            Ok(LocalExecutionAuthoritySnapshot { registry_record, selection_record, registry, selection, registry_receipt, selection_receipt,
                owner_account_id: identity.owner_account_id, actor_principal_id: identity.actor_principal_id, actor_id: identity.actor_id })
        })).await.map_err(map_storage_error)?;
        Ok(LocalExecutionAuthorityLease::new(snapshot, gate, publication))
    }
}

#[cfg(any(test, feature = "test-utils"))]
pub mod mt196_local_execution_authority_contract {
    use crate as handshake_core;
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/local_execution_authority_surreal_tests.rs"));
    authenticated_store_contract_tests!();
}
