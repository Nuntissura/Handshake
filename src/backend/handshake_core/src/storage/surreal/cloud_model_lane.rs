use super::{SurrealStorage, SurrealStorageError};
use serde::{Deserialize, Serialize};
use std::fmt;
use surrealdb::types::SurrealValue;

const SCHEMA: &str = include_str!("cloud_model_lane_schema.surql");
const SCHEMA_STATE: &str = "\
DEFINE TABLE OVERWRITE model_lane_cloud_schema_state SCHEMAFULL PERMISSIONS FOR select WHERE record::id($this.id) = 'primary' AND schema_version = 'mt006-cloud-authority-v1' AND schema_revision = 1 AND apply_state = 'complete' AND fn::wp012_cloud_schema_access() FOR create, update, delete NONE;\
DEFINE FIELD IF NOT EXISTS schema_version ON model_lane_cloud_schema_state TYPE string;\
DEFINE FIELD IF NOT EXISTS schema_revision ON model_lane_cloud_schema_state TYPE int;\
DEFINE FIELD IF NOT EXISTS apply_state ON model_lane_cloud_schema_state TYPE string;";
const SCHEMA_STATE_ID: &str = "model_lane_cloud_schema_state:primary";
const SCHEMA_VERSION: &str = "mt006-cloud-authority-v1";
const SCHEMA_REVISION: i64 = 1;

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, SurrealValue)]
pub(crate) struct CloudModelLaneSchemaState {
    pub(crate) schema_version: String,
    pub(crate) schema_revision: i64,
    pub(crate) apply_state: String,
}

#[derive(Debug, SurrealValue)]
struct CloudSchemaStateBindings {
    schema_version: String,
    schema_revision: i64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum CloudModelLaneRecordKind {
    ProjectionPlan,
    ConsentReceipt,
    ConsentDenial,
    CloudRun,
    CloudLane,
}

impl CloudModelLaneRecordKind {
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::ProjectionPlan => "projection_plan",
            Self::ConsentReceipt => "consent_receipt",
            Self::ConsentDenial => "consent_denial",
            Self::CloudRun => "cloud_run",
            Self::CloudLane => "cloud_lane",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub(crate) struct CloudModelLaneScope {
    pub owner_account_id: String,
    pub actor_principal_id: String,
    pub authenticated_session_id: String,
    pub access_space_id: String,
    pub workspace_id: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct CloudModelLaneStoredRow {
    pub record_json: String,
    pub event_id: String,
    pub event_seq: i64,
    pub event_payload_json: String,
}

#[derive(Clone)]
pub(crate) struct CloudModelLaneStore {
    storage: SurrealStorage,
}

impl fmt::Debug for CloudModelLaneStore {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("CloudModelLaneStore")
            .field("config", self.storage.config())
            .finish_non_exhaustive()
    }
}

#[derive(Debug, SurrealValue)]
struct AuthorityWriteBindings {
    record_id: String,
    event_id: String,
    kind: String,
    aggregate_id: String,
    run_id: String,
    projection_plan_id: String,
    consent_receipt_id: String,
    idempotency_key: String,
    record_json: String,
    event_seq: i64,
    event_payload_json: String,
    owner_account_id: String,
    actor_principal_id: String,
    authenticated_session_id: String,
    access_space_id: String,
    workspace_id: String,
}

#[derive(Debug, SurrealValue)]
struct ScopedLookupBindings {
    kind: String,
    aggregate_id: String,
    run_id: String,
    consent_receipt_id: String,
    owner_account_id: String,
    actor_principal_id: String,
    authenticated_session_id: String,
    access_space_id: String,
    workspace_id: String,
}

#[derive(Debug, SurrealValue)]
struct StoredRowValue {
    record_json: String,
    event_id: String,
    event_seq: i64,
    event_payload_json: String,
}

impl From<StoredRowValue> for CloudModelLaneStoredRow {
    fn from(value: StoredRowValue) -> Self {
        Self {
            record_json: value.record_json,
            event_id: value.event_id,
            event_seq: value.event_seq,
            event_payload_json: value.event_payload_json,
        }
    }
}

pub(crate) async fn bootstrap_cloud_model_lane_schema(
    storage: &SurrealStorage,
) -> Result<(), SurrealStorageError> {
    storage
        .with_admin_operation(|database| {
            Box::pin(async move {
                database.query(SCHEMA_STATE).await?;
                let mut state_response = database
                    .query(format!("SELECT * FROM ONLY {SCHEMA_STATE_ID};"))
                    .await?;
                let state: Option<CloudModelLaneSchemaState> = state_response.take(0)?;
                if let Some(state) = state.as_ref() {
                    if state.schema_version != SCHEMA_VERSION
                        || state.schema_revision != SCHEMA_REVISION
                        || state.apply_state != "complete"
                    {
                        return Err(SurrealStorageError::InvalidCloudModelLaneRecord {
                            reason: "cloud authority schema state version/revision mismatch",
                        });
                    }
                }
                database.query(SCHEMA).await?;
                if state.is_none() {
                    database
                        .query_bound(
                            "UPSERT model_lane_cloud_schema_state:primary CONTENT { schema_version: $schema_version, schema_revision: $schema_revision, apply_state: 'complete' };",
                            CloudSchemaStateBindings {
                                schema_version: SCHEMA_VERSION.to_owned(),
                                schema_revision: SCHEMA_REVISION,
                            },
                        )
                        .await?;
                }
                Ok(())
            })
        })
        .await
}

impl CloudModelLaneStore {
    pub(crate) fn new(storage: SurrealStorage) -> Self {
        Self { storage }
    }

    pub(crate) fn storage(&self) -> &SurrealStorage {
        &self.storage
    }

    pub(crate) async fn schema_state(
        &self,
    ) -> Result<Option<CloudModelLaneSchemaState>, SurrealStorageError> {
        self.storage
            .with_data_operation(|database| {
                Box::pin(async move {
                    database
                        .select_one("model_lane_cloud_schema_state", "primary")
                        .await
                })
            })
            .await
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) async fn put_immutable(
        &self,
        kind: CloudModelLaneRecordKind,
        aggregate_id: &str,
        run_id: &str,
        projection_plan_id: Option<&str>,
        consent_receipt_id: Option<&str>,
        idempotency_key: &str,
        record_json: String,
        event_id: String,
        event_seq: i64,
        event_payload_json: String,
        scope: &CloudModelLaneScope,
    ) -> Result<CloudModelLaneStoredRow, SurrealStorageError> {
        let sealed = super::PROTECTED_TRANSACTION.try_with(|transaction| std::sync::Arc::ptr_eq(&transaction.storage.inner, &self.storage.inner)).unwrap_or(false);
        let expected_purpose = match kind {
            CloudModelLaneRecordKind::ProjectionPlan => super::model_lane::ContextCoordinationPurpose::CloudProjectionPlan,
            CloudModelLaneRecordKind::ConsentReceipt => super::model_lane::ContextCoordinationPurpose::CloudConsentReceipt,
            CloudModelLaneRecordKind::CloudRun => super::model_lane::ContextCoordinationPurpose::CloudRun,
            CloudModelLaneRecordKind::CloudLane => super::model_lane::ContextCoordinationPurpose::CloudLane,
            CloudModelLaneRecordKind::ConsentDenial => super::model_lane::ContextCoordinationPurpose::CloudConsentDenial,
        };
        if !sealed || !super::MODEL_LANE_OPERATION_PURPOSE.try_with(|purpose| *purpose == expected_purpose).unwrap_or(false) {
            return Err(SurrealStorageError::TransactionAuthorityRequired);
        }
        let expected: serde_json::Value = serde_json::from_str(&record_json).map_err(|_| SurrealStorageError::InvalidCloudModelLaneRecord { reason: "cloud record JSON is malformed" })?;
        let expected_payload: serde_json::Value = serde_json::from_str(&event_payload_json).map_err(|_| SurrealStorageError::InvalidCloudModelLaneRecord { reason: "cloud payload JSON is malformed" })?;
        let record_id = stable_record_id(kind.as_str(), idempotency_key, scope);
        let bindings = AuthorityWriteBindings {
            record_id,
            event_id,
            kind: kind.as_str().to_owned(),
            aggregate_id: aggregate_id.to_owned(),
            run_id: run_id.to_owned(),
            projection_plan_id: projection_plan_id.unwrap_or_default().to_owned(),
            consent_receipt_id: consent_receipt_id.unwrap_or_default().to_owned(),
            idempotency_key: idempotency_key.to_owned(),
            record_json,
            event_seq,
            event_payload_json,
            owner_account_id: scope.owner_account_id.clone(),
            actor_principal_id: scope.actor_principal_id.clone(),
            authenticated_session_id: scope.authenticated_session_id.clone(),
            access_space_id: scope.access_space_id.clone(),
            workspace_id: scope.workspace_id.clone(),
        };
        let rows = self
            .storage
            .with_data_operation(|database| {
                Box::pin(async move {
                    database
                        .query_values_at::<StoredRowValue, _>(
                            "LET $existing = (SELECT record_json, event_id, event_seq, event_payload_json FROM type::record('model_lane_cloud_authority', $record_id) WHERE owner_account_id = $owner_account_id AND actor_principal_id = $actor_principal_id AND authenticated_session_id = $authenticated_session_id AND access_space_id = $access_space_id AND workspace_id = $workspace_id);\
                             IF array::len($existing) = 1 {\
                               LET $old = encoding::json::decode($existing[0].record_json);\
                               LET $metadata = ['event_ledger_event_id', 'event_ledger_seq', 'event_stream_version', 'transaction_seq'];\
                               LET $old_body = object::from_entries(array::filter(object::entries($old), |$entry| $entry[0] NOT IN $metadata));\
                               LET $new_body = object::from_entries(array::filter(object::entries(encoding::json::decode($record_json)), |$entry| $entry[0] NOT IN $metadata));\
                               IF $old_body != $new_body { THROW 'cloud immutable identity conflict'; };\
                               LET $old_payload = encoding::json::decode($existing[0].event_payload_json);\
                               IF object::from_entries(array::filter(object::entries($old_payload), |$entry| $entry[0] != 'record')) != object::from_entries(array::filter(object::entries(encoding::json::decode($event_payload_json)), |$entry| $entry[0] != 'record')) { THROW 'cloud immutable payload conflict'; };\
                               RETURN $existing;\
                             } ELSE {\
                               IF array::len($existing) != 0 { THROW 'cloud authority is ambiguous'; };\
                               LET $attribution = { owner_account_id: $owner_account_id, actor_principal_id: $actor_principal_id, authenticated_session_id: $authenticated_session_id, access_space_id: $access_space_id, workspace_id: $workspace_id, authority_resource_id: type::record('protected_resources', $wp012_original_resource), authority_grant_id: type::record('resource_grants', $wp012_original_grant), authority_session_id: $auth.id, authority_capability_id: $wp012_original_capability, authority_action: $wp012_original_action, authority_purpose: $wp012_original_purpose };\
                               LET $allocated_id = type::string(rand::uuid::v7());\
                               LET $prepared = encoding::json::decode($record_json);\
                               LET $body = object::from_entries(array::filter(object::entries($prepared), |$entry| $entry[0] NOT IN ['event_ledger_event_id', 'event_ledger_seq', 'event_stream_version', 'transaction_seq']));\
                               LET $prepared_payload = encoding::json::encode(object::extend(encoding::json::decode($event_payload_json), { record: $body }));\
                               LET $ledger = CREATE type::record('model_lane_cloud_event_ledger', $allocated_id) CONTENT object::extend($attribution, { aggregate_kind: $kind, aggregate_id: $aggregate_id, run_id: $run_id, event_ledger_stream_id: $body.event_ledger_stream_id, payload_json: $prepared_payload });\
                               IF array::len($ledger) != 1 { THROW 'HSK-CLOUD-ALLOCATION-CONFLICT'; };\
                               LET $seq = $ledger[0].event_seq;\
                               LET $final_payload = $ledger[0].payload_json;\
                               LET $final_record = encoding::json::decode($final_payload).record;\
                               RETURN CREATE type::record('model_lane_cloud_authority', $record_id) CONTENT object::extend($attribution, { kind: $kind, aggregate_id: $aggregate_id, run_id: $run_id, projection_plan_id: $projection_plan_id, consent_receipt_id: $consent_receipt_id, idempotency_key: $idempotency_key, record_json: encoding::json::encode($final_record), event_id: $allocated_id, event_seq: $seq, event_payload_json: $final_payload, receipt_id: type::record('model_lane_cloud_event_ledger', $allocated_id) });\
                             };",
                            bindings,
                            1,
                        )
                        .await
                })
            })
            .await?;
        if rows.len() != 1 { return Err(SurrealStorageError::InvalidCloudModelLaneRecord { reason: "authority write returned no unique row" }); }
        let row: CloudModelLaneStoredRow = rows.into_iter().next().expect("one cloud row").into();
        let actual: serde_json::Value = serde_json::from_str(&row.record_json).map_err(|_| SurrealStorageError::InvalidCloudModelLaneRecord { reason: "cloud canonical record JSON is malformed" })?;
        if immutable_cloud_body(actual) != immutable_cloud_body(expected) {
            return Err(SurrealStorageError::InvalidCloudModelLaneRecord { reason: "cloud immutable body conflict" });
        }
        let mut actual_payload: serde_json::Value = serde_json::from_str(&row.event_payload_json).map_err(|_| SurrealStorageError::InvalidCloudModelLaneRecord { reason: "cloud canonical payload JSON is malformed" })?;
        let mut expected_payload = expected_payload;
        actual_payload.as_object_mut().ok_or(SurrealStorageError::InvalidCloudModelLaneRecord { reason: "cloud payload is not an object" })?.remove("record");
        expected_payload.as_object_mut().ok_or(SurrealStorageError::InvalidCloudModelLaneRecord { reason: "cloud prepared payload is not an object" })?.remove("record");
        if actual_payload != expected_payload { return Err(SurrealStorageError::InvalidCloudModelLaneRecord { reason: "cloud immutable payload conflict" }); }
        self.validate_actual_receipt(kind, aggregate_id, run_id, scope, &row).await?;
        Ok(row)
    }

    #[cfg(feature = "test-utils")]
    pub(crate) async fn admission_counterfactual(
        &self, case: &'static str, record_json: String, event_payload_json: String,
        scope: &CloudModelLaneScope,
    ) -> Result<(), SurrealStorageError> {
        if !matches!(case, "sequence" | "metadata" | "target_hash")
            || !super::PROTECTED_TRANSACTION.try_with(|transaction| std::sync::Arc::ptr_eq(&transaction.storage.inner, &self.storage.inner)).unwrap_or(false)
            || !super::MODEL_LANE_OPERATION_PURPOSE.try_with(|purpose| *purpose == super::model_lane::ContextCoordinationPurpose::CloudProjectionPlan).unwrap_or(false) {
            return Err(SurrealStorageError::TransactionAuthorityRequired);
        }
        #[derive(Debug, SurrealValue)]
        struct ProbeBindings {
            case: String, record_id: String, record_json: String, event_payload_json: String,
            owner_account_id: String, actor_principal_id: String, authenticated_session_id: String,
            access_space_id: String, workspace_id: String,
        }
        let prepared: serde_json::Value = serde_json::from_str(&record_json).map_err(|_| SurrealStorageError::InvalidCloudModelLaneRecord { reason: "cloud probe record is malformed" })?;
        let key = prepared.get("idempotency_key").and_then(serde_json::Value::as_str).ok_or(SurrealStorageError::InvalidCloudModelLaneRecord { reason: "cloud probe identity is absent" })?;
        let record_id = stable_record_id("projection_plan", key, scope);
        let bindings = ProbeBindings { case: case.to_owned(), record_id, record_json, event_payload_json,
            owner_account_id: scope.owner_account_id.clone(), actor_principal_id: scope.actor_principal_id.clone(),
            authenticated_session_id: scope.authenticated_session_id.clone(), access_space_id: scope.access_space_id.clone(),
            workspace_id: scope.workspace_id.clone() };
        self.storage.with_data_operation(|database| Box::pin(async move {
            database.query_bound(
                "LET $attribution = { owner_account_id: $owner_account_id, actor_principal_id: $actor_principal_id, authenticated_session_id: $authenticated_session_id, access_space_id: $access_space_id, workspace_id: $workspace_id, authority_resource_id: type::record('protected_resources', $wp012_original_resource), authority_grant_id: type::record('resource_grants', $wp012_original_grant), authority_session_id: $auth.id, authority_capability_id: $wp012_original_capability, authority_action: $wp012_original_action, authority_purpose: $wp012_original_purpose };\
                 LET $original = encoding::json::decode($record_json);\
                 LET $body = object::from_entries(array::filter(object::entries($original), |$entry| $entry[0] NOT IN ['event_ledger_event_id','event_ledger_seq','event_stream_version','transaction_seq']));\
                 LET $record = IF $case = 'metadata' { object::extend($body, { event_ledger_seq: 987654321, event_stream_version: 987654322, transaction_seq: 987654323 }) } ELSE IF $case = 'target_hash' { object::extend($body, { target_bindings_hash: string::repeat('0',64) }) } ELSE { $body };\
                 LET $payload = encoding::json::encode(object::extend(encoding::json::decode($event_payload_json), { record: $record }));\
                 LET $event_id = type::string(rand::uuid::v7());\
                 LET $content = object::extend($attribution, { aggregate_kind: 'projection_plan', aggregate_id: $record.projection_plan_id, run_id: $record.run_id, event_ledger_stream_id: $record.event_ledger_stream_id, payload_json: $payload });\
                 LET $ledger = CREATE type::record('model_lane_cloud_event_ledger', $event_id) CONTENT IF $case = 'sequence' { object::extend($content, { event_seq: 987654321 }) } ELSE { $content };\
                 LET $final_payload = $ledger[0].payload_json;\
                 LET $final_record = encoding::json::decode($final_payload).record;\
                 IF array::len($ledger) != 1 { THROW 'HSK-CLOUD-PROBE-LEDGER-DENIED'; };\
                 LET $authority = CREATE type::record('model_lane_cloud_authority', $record_id) CONTENT object::extend($attribution, { kind: 'projection_plan', aggregate_id: $record.projection_plan_id, run_id: $record.run_id, projection_plan_id: $record.projection_plan_id, consent_receipt_id: '', idempotency_key: $record.idempotency_key, record_json: encoding::json::encode($final_record), event_id: $event_id, event_seq: $ledger[0].event_seq, event_payload_json: $final_payload, receipt_id: type::record('model_lane_cloud_event_ledger', $event_id) });\
                 IF array::len($authority) != 1 { THROW 'HSK-CLOUD-PROBE-AUTHORITY-DENIED'; };",
                bindings,
            ).await
        })).await
    }

    pub(crate) async fn get(
        &self,
        kind: CloudModelLaneRecordKind,
        aggregate_id: &str,
        scope: &CloudModelLaneScope,
    ) -> Result<Option<CloudModelLaneStoredRow>, SurrealStorageError> {
        let bindings = lookup_bindings(kind, aggregate_id, "", "", scope);
        self.query_rows(
            "SELECT record_json, event_id, event_seq, event_payload_json FROM model_lane_cloud_authority WHERE kind = $kind AND aggregate_id = $aggregate_id AND owner_account_id = $owner_account_id AND actor_principal_id = $actor_principal_id AND authenticated_session_id = $authenticated_session_id AND access_space_id = $access_space_id AND workspace_id = $workspace_id LIMIT 1;",
            bindings,
        )
        .await
        .map(|rows| rows.into_iter().next())
    }

    pub(crate) async fn list_run(
        &self,
        kind: CloudModelLaneRecordKind,
        run_id: &str,
        scope: &CloudModelLaneScope,
    ) -> Result<Vec<CloudModelLaneStoredRow>, SurrealStorageError> {
        self.query_rows(
            "SELECT record_json, event_id, event_seq, event_payload_json FROM model_lane_cloud_authority WHERE kind = $kind AND run_id = $run_id AND owner_account_id = $owner_account_id AND actor_principal_id = $actor_principal_id AND authenticated_session_id = $authenticated_session_id AND access_space_id = $access_space_id AND workspace_id = $workspace_id ORDER BY event_seq ASC;",
            lookup_bindings(kind, "", run_id, "", scope),
        )
        .await
    }

    pub(crate) async fn list_consent_lanes(
        &self,
        consent_receipt_id: &str,
        scope: &CloudModelLaneScope,
    ) -> Result<Vec<CloudModelLaneStoredRow>, SurrealStorageError> {
        self.query_rows(
            "SELECT record_json, event_id, event_seq, event_payload_json FROM model_lane_cloud_authority WHERE kind = $kind AND consent_receipt_id = $consent_receipt_id AND owner_account_id = $owner_account_id AND actor_principal_id = $actor_principal_id AND authenticated_session_id = $authenticated_session_id AND access_space_id = $access_space_id AND workspace_id = $workspace_id ORDER BY event_seq ASC;",
            lookup_bindings(
                CloudModelLaneRecordKind::CloudLane,
                "",
                "",
                consent_receipt_id,
                scope,
            ),
        )
        .await
    }

    pub(crate) async fn replace(
        &self,
        kind: CloudModelLaneRecordKind,
        aggregate_id: &str,
        record_json: String,
        event_id: String,
        event_seq: i64,
        event_payload_json: String,
        scope: &CloudModelLaneScope,
    ) -> Result<Option<CloudModelLaneStoredRow>, SurrealStorageError> {
        let bindings = AuthorityWriteBindings {
            record_id: String::new(),
            event_id,
            kind: kind.as_str().to_owned(),
            aggregate_id: aggregate_id.to_owned(),
            run_id: String::new(),
            projection_plan_id: String::new(),
            consent_receipt_id: String::new(),
            idempotency_key: String::new(),
            record_json,
            event_seq,
            event_payload_json,
            owner_account_id: scope.owner_account_id.clone(),
            actor_principal_id: scope.actor_principal_id.clone(),
            authenticated_session_id: scope.authenticated_session_id.clone(),
            access_space_id: scope.access_space_id.clone(),
            workspace_id: scope.workspace_id.clone(),
        };
        let rows = self.storage.with_data_operation(|database| Box::pin(async move {
                database.query_values_at::<StoredRowValue, _>(
                "BEGIN TRANSACTION;\
                 LET $target = (SELECT VALUE id FROM model_lane_cloud_authority WHERE kind = $kind AND aggregate_id = $aggregate_id AND owner_account_id = $owner_account_id AND actor_principal_id = $actor_principal_id AND authenticated_session_id = $authenticated_session_id AND access_space_id = $access_space_id AND workspace_id = $workspace_id LIMIT 1);\
                 IF array::len($target) = 0 { RETURN []; } ELSE {\
                   LET $current = (SELECT * FROM ONLY $target[0]);\
                   LET $ledger = CREATE type::record('model_lane_cloud_event_ledger', $event_id) CONTENT { aggregate_kind: $kind, aggregate_id: $aggregate_id, run_id: $current.run_id, event_seq: $event_seq, payload_json: $event_payload_json, owner_account_id: $owner_account_id, actor_principal_id: $actor_principal_id, authenticated_session_id: $authenticated_session_id, access_space_id: $access_space_id, workspace_id: $workspace_id };\
                   RETURN UPDATE $target[0] SET record_json = $record_json, event_id = $event_id, event_seq = $event_seq, event_payload_json = $event_payload_json;\
                 };\
                 COMMIT TRANSACTION;",
                    bindings,
                    2,
            ).await
        })).await?;
        Ok(rows.into_iter().next().map(Into::into))
    }

    async fn validate_actual_receipt(&self, kind: CloudModelLaneRecordKind, aggregate_id: &str, run_id: &str,
        scope: &CloudModelLaneScope, row: &CloudModelLaneStoredRow) -> Result<(), SurrealStorageError> {
        let bindings = ReceiptLookupBindings { event_id: row.event_id.clone(), kind: kind.as_str().to_owned(),
            aggregate_id: aggregate_id.to_owned(), run_id: run_id.to_owned(), event_seq: row.event_seq,
            payload_json: row.event_payload_json.clone(), owner_account_id: scope.owner_account_id.clone(),
            actor_principal_id: scope.actor_principal_id.clone(), authenticated_session_id: scope.authenticated_session_id.clone(),
            access_space_id: scope.access_space_id.clone(), workspace_id: scope.workspace_id.clone() };
        let receipts = self.storage.with_data_operation(|database| Box::pin(async move {
            database.query_values::<String, _>("SELECT VALUE event_id FROM type::record('model_lane_cloud_event_ledger', $event_id) WHERE event_id = $event_id AND aggregate_kind = $kind AND aggregate_id = $aggregate_id AND run_id = $run_id AND event_seq = $event_seq AND event_seq > 0 AND payload_json = $payload_json AND payload_hash = crypto::sha256(payload_json) AND owner_account_id = $owner_account_id AND actor_principal_id = $actor_principal_id AND authenticated_session_id = $authenticated_session_id AND access_space_id = $access_space_id AND workspace_id = $workspace_id;", bindings).await
        })).await?;
        let record: serde_json::Value = serde_json::from_str(&row.record_json).map_err(|_| SurrealStorageError::InvalidCloudModelLaneRecord { reason: "cloud canonical JSON is malformed" })?;
        let payload: serde_json::Value = serde_json::from_str(&row.event_payload_json).map_err(|_| SurrealStorageError::InvalidCloudModelLaneRecord { reason: "cloud canonical receipt JSON is malformed" })?;
        if receipts != vec![row.event_id.clone()] || record.get("event_ledger_event_id").and_then(serde_json::Value::as_str) != Some(row.event_id.as_str())
            || record.get("event_ledger_seq").and_then(serde_json::Value::as_i64) != Some(row.event_seq)
            || payload.get("record") != Some(&record) {
            return Err(SurrealStorageError::InvalidCloudModelLaneRecord { reason: "cloud authority has no matching independent durable receipt" });
        }
        Ok(())
    }

    async fn query_rows(&self, statement: &'static str, bindings: ScopedLookupBindings)
        -> Result<Vec<CloudModelLaneStoredRow>, SurrealStorageError> {
        let scope = CloudModelLaneScope { owner_account_id: bindings.owner_account_id.clone(),
            actor_principal_id: bindings.actor_principal_id.clone(), authenticated_session_id: bindings.authenticated_session_id.clone(),
            access_space_id: bindings.access_space_id.clone(), workspace_id: bindings.workspace_id.clone() };
        let kind = match bindings.kind.as_str() {
            "projection_plan" => CloudModelLaneRecordKind::ProjectionPlan, "consent_receipt" => CloudModelLaneRecordKind::ConsentReceipt,
            "cloud_run" => CloudModelLaneRecordKind::CloudRun, "cloud_lane" => CloudModelLaneRecordKind::CloudLane,
            _ => return Err(SurrealStorageError::TransactionAuthorityRequired),
        };
        let rows = self.storage.with_data_operation(|database| Box::pin(async move {
            database.query_values::<StoredRowValue, _>(statement, bindings).await
        })).await?;
        let mut result = Vec::with_capacity(rows.len());
        for row in rows {
            let row: CloudModelLaneStoredRow = row.into();
            let record: serde_json::Value = serde_json::from_str(&row.record_json).map_err(|_| SurrealStorageError::InvalidCloudModelLaneRecord { reason: "cloud canonical record is malformed" })?;
            let aggregate = record.get(match kind { CloudModelLaneRecordKind::ProjectionPlan => "projection_plan_id", CloudModelLaneRecordKind::ConsentReceipt => "consent_receipt_id", CloudModelLaneRecordKind::CloudRun => "run_id", CloudModelLaneRecordKind::CloudLane => "lane_id", _ => unreachable!() }).and_then(serde_json::Value::as_str).ok_or(SurrealStorageError::InvalidCloudModelLaneRecord { reason: "cloud canonical aggregate is absent" })?;
            let run = record.get("run_id").and_then(serde_json::Value::as_str).ok_or(SurrealStorageError::InvalidCloudModelLaneRecord { reason: "cloud canonical run is absent" })?;
            self.validate_actual_receipt(kind, aggregate, run, &scope, &row).await?;
            result.push(row);
        }
        Ok(result)
    }

}

fn lookup_bindings(
    kind: CloudModelLaneRecordKind,
    aggregate_id: &str,
    run_id: &str,
    consent_receipt_id: &str,
    scope: &CloudModelLaneScope,
) -> ScopedLookupBindings {
    ScopedLookupBindings {
        kind: kind.as_str().to_owned(),
        aggregate_id: aggregate_id.to_owned(),
        run_id: run_id.to_owned(),
        consent_receipt_id: consent_receipt_id.to_owned(),
        owner_account_id: scope.owner_account_id.clone(),
        actor_principal_id: scope.actor_principal_id.clone(),
        authenticated_session_id: scope.authenticated_session_id.clone(),
        access_space_id: scope.access_space_id.clone(),
        workspace_id: scope.workspace_id.clone(),
    }
}

fn stable_record_id(kind: &str, idempotency_key: &str, scope: &CloudModelLaneScope) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(b"handshake.model-lane.cloud-authority.v1\0");
    hasher.update(kind.as_bytes());
    hasher.update([0]);
    hasher.update(idempotency_key.as_bytes());
    for field in [&scope.owner_account_id, &scope.actor_principal_id, &scope.authenticated_session_id, &scope.access_space_id, &scope.workspace_id] {
        hasher.update([0]); hasher.update(field.as_bytes());
    }
    hex::encode(hasher.finalize())
}

fn immutable_cloud_body(mut record: serde_json::Value) -> serde_json::Value {
    if let Some(object) = record.as_object_mut() {
        for field in ["event_ledger_event_id", "event_ledger_seq", "event_stream_version", "transaction_seq"] { object.remove(field); }
    }
    record
}

#[derive(Debug, SurrealValue)]
struct ReceiptLookupBindings {
    event_id: String, kind: String, aggregate_id: String, run_id: String, event_seq: i64, payload_json: String,
    owner_account_id: String, actor_principal_id: String, authenticated_session_id: String, access_space_id: String, workspace_id: String,
}
