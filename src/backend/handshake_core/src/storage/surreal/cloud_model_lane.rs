use super::{SurrealStorage, SurrealStorageError};
use crate::kernel::context_bundle::{canonical_json_bytes, sha256_hex};
use serde::{Deserialize, Serialize};
use std::fmt;
use surrealdb::types::SurrealValue;

const SCHEMA: &str = include_str!("cloud_model_lane_schema.surql");
pub(super) const SCHEMA_STATE: &str = "\
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
    ConsentRevocation,
    CloudRun,
    CloudLane,
    LaneTerminal,
}

impl CloudModelLaneRecordKind {
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::ProjectionPlan => "projection_plan",
            Self::ConsentReceipt => "consent_receipt",
            Self::ConsentDenial => "consent_denial",
            Self::ConsentRevocation => "consent_revocation",
            Self::CloudRun => "cloud_run",
            Self::CloudLane => "cloud_lane",
            Self::LaneTerminal => "lane_terminal",
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
struct CheckpointDenialLookupBindings {
    event_id: String,
    run_id: String,
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
    projection_plan_id: String,
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

#[derive(Debug, SurrealValue)]
struct CloudOverlaySnapshot {
    events: Vec<String>,
    candidates: Vec<StoredRowValue>,
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

// FINAL catalog reuse requires the owning durable receipt; this never installs DDL.
pub(super) async fn verify_schema_receipt(
    database: &super::SurrealAdminContext<'_>,
) -> Result<(), SurrealStorageError> {
    let mut response = database.query(format!("SELECT * FROM ONLY {SCHEMA_STATE_ID};")).await?;
    let state: Option<CloudModelLaneSchemaState> = response.take(0)?;
    if !state.as_ref().is_some_and(|state| {
        state.schema_version == SCHEMA_VERSION
            && state.schema_revision == SCHEMA_REVISION
            && state.apply_state == "complete"
    }) {
        return Err(SurrealStorageError::InvalidCloudModelLaneRecord {
            reason: "cloud authority schema receipt missing or version/revision/state mismatch",
        });
    }
    Ok(())
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
            CloudModelLaneRecordKind::ConsentRevocation | CloudModelLaneRecordKind::LaneTerminal =>
                super::model_lane::ContextCoordinationPurpose::CloudConsentRevocation,
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

    #[allow(clippy::too_many_arguments)]
    pub(crate) async fn validate_checkpoint_denial_member(
        &self,
        event_id: &str,
        event_seq: i64,
        payload_hash: &str,
        run_id: &str,
        lane_id: &str,
        event_ledger_stream_id: &str,
        scope: &CloudModelLaneScope,
    ) -> Result<CloudModelLaneStoredRow, SurrealStorageError> {
        let sealed = super::PROTECTED_TRANSACTION
            .try_with(|transaction| std::sync::Arc::ptr_eq(&transaction.storage.inner, &self.storage.inner))
            .unwrap_or(false);
        let purpose = super::MODEL_LANE_OPERATION_PURPOSE.try_with(|purpose| {
            matches!(purpose,
                super::model_lane::ContextCoordinationPurpose::RecoveryCheckpointRecord
                    | super::model_lane::ContextCoordinationPurpose::RecoveryExecute)
        }).unwrap_or(false);
        if !sealed || !purpose {
            return Err(SurrealStorageError::TransactionAuthorityRequired);
        }
        if event_id.trim().is_empty() || event_seq <= 0 || payload_hash.len() != 64
            || run_id.trim().is_empty() || lane_id.trim().is_empty()
            || event_ledger_stream_id.trim().is_empty()
        {
            return Err(SurrealStorageError::InvalidCloudModelLaneRecord {
                reason: "checkpoint cloud receipt identity is incomplete",
            });
        }
        let bindings = CheckpointDenialLookupBindings {
            event_id: event_id.to_owned(),
            run_id: run_id.to_owned(),
            owner_account_id: scope.owner_account_id.clone(),
            actor_principal_id: scope.actor_principal_id.clone(),
            authenticated_session_id: scope.authenticated_session_id.clone(),
            access_space_id: scope.access_space_id.clone(),
            workspace_id: scope.workspace_id.clone(),
        };
        let rows = self.storage.with_data_operation(|database| Box::pin(async move {
            database.query_values::<StoredRowValue, _>(
                "SELECT record_json, event_id, event_seq, event_payload_json FROM model_lane_cloud_authority WHERE kind = 'consent_denial' AND event_id = $event_id AND run_id = $run_id AND owner_account_id = $owner_account_id AND actor_principal_id = $actor_principal_id AND authenticated_session_id = $authenticated_session_id AND access_space_id = $access_space_id AND workspace_id = $workspace_id;",
                bindings,
            ).await
        })).await?;
        if rows.len() != 1 {
            return Err(SurrealStorageError::InvalidCloudModelLaneRecord {
                reason: "checkpoint cloud receipt has no unique canonical authority",
            });
        }
        let row: CloudModelLaneStoredRow = rows.into_iter().next().expect("one cloud receipt").into();
        self.validate_actual_receipt(CloudModelLaneRecordKind::ConsentDenial, lane_id, run_id, scope, &row).await?;
        let record: serde_json::Value = serde_json::from_str(&row.record_json)
            .map_err(|_| SurrealStorageError::InvalidCloudModelLaneRecord {
                reason: "checkpoint cloud receipt record is malformed",
            })?;
        let payload: serde_json::Value = serde_json::from_str(&row.event_payload_json)
            .map_err(|_| SurrealStorageError::InvalidCloudModelLaneRecord {
                reason: "checkpoint cloud receipt payload is malformed",
            })?;
        let actual_hash = {
            use sha2::{Digest, Sha256};
            hex::encode(Sha256::digest(row.event_payload_json.as_bytes()))
        };
        if payload.get("dexterity_kernel").and_then(serde_json::Value::as_str) != Some("Dexterity")
            || payload.get("schema_id").and_then(serde_json::Value::as_str) != Some("hsk.model_lane_cloud_consent_denial@1")
            || row.event_id != event_id || row.event_seq != event_seq || actual_hash != payload_hash
            || record.get("run_id").and_then(serde_json::Value::as_str) != Some(run_id)
            || record.get("lane_id").and_then(serde_json::Value::as_str) != Some(lane_id)
            || record.get("event_ledger_stream_id").and_then(serde_json::Value::as_str) != Some(event_ledger_stream_id)
            || record.get("provider_call_attempted").and_then(serde_json::Value::as_bool) != Some(false)
            || record.get("partial_authority_state_created").and_then(serde_json::Value::as_bool) != Some(false)
        {
            return Err(SurrealStorageError::InvalidCloudModelLaneRecord {
                reason: "checkpoint cloud receipt does not match its immutable membership",
            });
        }
        Ok(row)
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

    async fn validate_actual_receipt(&self, kind: CloudModelLaneRecordKind, aggregate_id: &str, run_id: &str,
        scope: &CloudModelLaneScope, row: &CloudModelLaneStoredRow) -> Result<serde_json::Value, SurrealStorageError> {
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
        Ok(payload)
    }

    async fn read_overlay(
        &self,
        base_kind: CloudModelLaneRecordKind,
        aggregate_id: &str,
        run_id: &str,
        base_record: &serde_json::Value,
        base_event_seq: i64,
        scope: &CloudModelLaneScope,
    ) -> Result<Option<CloudModelLaneStoredRow>, SurrealStorageError> {
        let (kind, projection_field, consent_field, schema_id, status) = match base_kind {
            CloudModelLaneRecordKind::ConsentReceipt => (
                CloudModelLaneRecordKind::ConsentRevocation,
                "projection_plan_id",
                "consent_receipt_id",
                "hsk.model_lane_cloud_consent_receipt@2",
                "revoked",
            ),
            CloudModelLaneRecordKind::CloudLane => (
                CloudModelLaneRecordKind::LaneTerminal,
                "projection_plan_ref",
                "consent_receipt_ref",
                "hsk.model_lane_terminal@1",
                "cancelled",
            ),
            _ => return Ok(None),
        };
        let base_ref = |field| {
            base_record.get(field).and_then(serde_json::Value::as_str)
                .filter(|value| !value.trim().is_empty())
                .ok_or(SurrealStorageError::InvalidCloudModelLaneRecord {
                    reason: "cloud overlay base has no canonical projection or consent reference",
                })
        };
        let projection = base_ref(projection_field)?;
        let consent = base_ref(consent_field)?;
        let mut bindings = lookup_bindings(kind, aggregate_id, run_id, consent, scope);
        bindings.projection_plan_id = projection.to_owned();

        let mismatch = || SurrealStorageError::InvalidCloudModelLaneRecord {
            reason: "cloud overlay has no unique matching authority and receipt",
        };
        let snapshot = self.storage.with_data_operation(|database| Box::pin(async move {
            database.query_first::<CloudOverlaySnapshot, _>(
                "RETURN {
                    events: (SELECT VALUE record::id(id) FROM model_lane_cloud_event_ledger
                        WHERE aggregate_kind = $kind AND aggregate_id = $aggregate_id
                          AND owner_account_id = $owner_account_id AND actor_principal_id = $actor_principal_id
                          AND authenticated_session_id = $authenticated_session_id AND access_space_id = $access_space_id
                          AND workspace_id = $workspace_id LIMIT 2),
                    candidates: (SELECT record_json, event_id, event_seq, event_payload_json FROM model_lane_cloud_authority
                        WHERE kind = $kind AND aggregate_id = $aggregate_id AND run_id = $run_id
                          AND projection_plan_id = $projection_plan_id AND consent_receipt_id = $consent_receipt_id
                          AND owner_account_id = $owner_account_id AND actor_principal_id = $actor_principal_id
                          AND authenticated_session_id = $authenticated_session_id AND access_space_id = $access_space_id
                          AND workspace_id = $workspace_id LIMIT 2)
                };",
                bindings,
            ).await
        })).await?;
        let CloudOverlaySnapshot { events, candidates } = snapshot.ok_or_else(mismatch)?;
        if events.is_empty() && candidates.is_empty() {
            return Ok(None);
        }
        let [event_id] = events.as_slice() else {
            return Err(mismatch());
        };
        let [candidate]: [StoredRowValue; 1] = candidates.try_into().map_err(|_| mismatch())?;
        if &candidate.event_id != event_id || candidate.event_seq <= base_event_seq {
            return Err(mismatch());
        }

        let row = candidate.into();
        let payload = self.validate_actual_receipt(kind, aggregate_id, run_id, scope, &row).await?;
        let record = &payload["record"];
        let matches_terminal_state = if kind == CloudModelLaneRecordKind::LaneTerminal {
            record["lane_id"].as_str() == Some(aggregate_id)
                && record["failstate_code"].as_str() == Some("CX-MM-007")
                && payload["status"].as_str() == Some(status)
                && payload["lane_id"].as_str() == Some(aggregate_id)
                && payload["run_id"].as_str() == Some(run_id)
                && payload["consent_receipt_id"].as_str() == Some(consent)
                && terminal_overlay_matches_base(base_record, record, run_id, aggregate_id)
        } else {
            record["approved"].as_bool() == Some(false)
                && record["revocation_ref"].as_str().is_some_and(|reference| !reference.trim().is_empty())
                && payload["revocation_ref"] == record["revocation_ref"]
                && consent_overlay_matches_base(base_record, record, consent)
        };
        if payload["schema_id"].as_str() != Some(schema_id)
            || payload["dexterity_kernel"].as_str() != Some("Dexterity")
            || payload["reason_code"].as_str() != Some("CX-MM-007")
            || record["status"].as_str() != Some(status)
            || record["run_id"].as_str() != Some(run_id)
            || record[projection_field].as_str() != Some(projection)
            || record[consent_field].as_str() != Some(consent)
            || !matches_terminal_state
        {
            return Err(mismatch());
        }
        Ok(Some(row))
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
            let overlay = self.read_overlay(kind, aggregate, run, &record, row.event_seq, &scope).await?;
            result.push(overlay.unwrap_or(row));
        }
        Ok(result)
    }

}

fn immutable_overlay_fields_match(
    base: &serde_json::Value,
    overlay: &serde_json::Value,
    mutable_fields: &[&str],
    optional_fields: &[&str],
    empty_array_defaults: &[&str],
) -> bool {
    use serde_json::Value;

    let (Some(base), Some(overlay)) = (base.as_object(), overlay.as_object()) else {
        return false;
    };
    base.keys().chain(overlay.keys()).all(|field| {
        let (base, overlay) = (base.get(field), overlay.get(field));
        mutable_fields.contains(&field.as_str())
            || base == overlay
            || (optional_fields.contains(&field.as_str())
                && base.unwrap_or(&Value::Null) == overlay.unwrap_or(&Value::Null))
            || (empty_array_defaults.contains(&field.as_str())
                && [base, overlay].into_iter().all(|value| {
                    value.is_none_or(|value| value.as_array().is_some_and(Vec::is_empty))
                }))
    })
}

fn consent_overlay_matches_base(base: &serde_json::Value, overlay: &serde_json::Value, consent: &str) -> bool {
    use serde_json::{json, Value};

    let mutable = [
        "status", "approved", "revoked_at_utc", "revocation_ref", "revocation_input_hash",
        "diagnostic_payload", "consent_receipt_hash", "event_ledger_event_id", "event_ledger_seq",
        "event_stream_version", "transaction_seq",
    ];
    let optional = ["lane_id", "model_session_id", "provider_kind", "requested_model_id", "target_bindings_hash"];
    if !immutable_overlay_fields_match(base, overlay, &mutable, &optional, &["target_bindings"]) {
        return false;
    }

    let (Some(base_diagnostics), Some(body), Some(actor), Some(reason)) = (
        base.get("diagnostic_payload"),
        overlay.as_object(),
        overlay["revocation_ref"].as_str(),
        overlay["diagnostic_payload"]["revocation_reason"].as_str(),
    ) else {
        return false;
    };
    if actor.trim().is_empty() || reason.trim().is_empty()
        || !overlay["revoked_at_utc"].as_str().is_some_and(|value| chrono::DateTime::parse_from_rfc3339(value).is_ok())
        || overlay["event_stream_version"] != overlay["event_ledger_seq"]
        || overlay["transaction_seq"] != overlay["event_ledger_seq"]
    {
        return false;
    }
    let mut diagnostics = base_diagnostics.as_object().cloned().unwrap_or_default();
    diagnostics.extend([
        ("consent_status".to_owned(), json!("CX-MM-007")),
        ("revocation_reason".to_owned(), json!(reason)),
        ("revoked_by_ref".to_owned(), json!(actor)),
        ("storage_authority".to_owned(), json!("embedded_surrealdb")),
    ]);
    let input_hash = sha256_hex(&canonical_json_bytes(&json!({
        "consent_receipt_id": consent,
        "revoked_by_ref": actor,
        "reason": reason,
    })));
    let mut body = body.clone();
    for key in ["consent_receipt_hash", "event_ledger_event_id", "event_ledger_seq", "event_stream_version", "transaction_seq", "target_bindings_hash"] {
        body.remove(key);
    }
    let receipt_hash = sha256_hex(&canonical_json_bytes(&Value::Object(body)));
    overlay["diagnostic_payload"] == Value::Object(diagnostics)
        && overlay["revocation_input_hash"].as_str() == Some(input_hash.as_str())
        && overlay["consent_receipt_hash"].as_str() == Some(receipt_hash.as_str())
}

fn terminal_overlay_matches_base(base: &serde_json::Value, overlay: &serde_json::Value, run: &str, lane: &str) -> bool {
    let mutable = [
        "status", "recovery_state", "failstate_code", "reason_ref", "recovery_hint_ref", "last_runtime_status_ref",
        "event_ledger_event_id", "event_ledger_seq",
    ];
    let optional = [
        "engine_profile", "model_id", "effective_capability_snapshot_ref", "capability_negotiation_ref",
        "provider_feature_profile_ref", "requested_execution_policy_ref", "effective_execution_policy_ref",
        "projection_plan_ref", "consent_receipt_ref", "heartbeat_at_utc", "lease_expires_at_utc",
        "reclaim_after_utc", "cancellation_ref", "reclaim_policy_ref", "terminal_status_mapping_ref",
        "process_ownership_ref", "no_os_process_reason_ref", "backpressure_ref", "loop_counter_ref",
        "last_recovery_event_ref", "startup_failure_ref", "work_packet_id", "micro_task_id", "task_board_id",
        "locus_binding",
    ];
    immutable_overlay_fields_match(base, overlay, &mutable, &optional, &[])
        && matches!(base["status"].as_str(), Some("planned" | "ready" | "running" | "waiting" | "reclaimable"))
        && overlay["recovery_state"].as_str() == Some("terminal")
        && overlay["reason_ref"].as_str() == Some(format!("cloud-consent-revoked://dexterity/{run}/{lane}").as_str())
        && overlay["recovery_hint_ref"].as_str() == Some("usermanual://model-lane-cloud-projection-consent#recovery")
        && overlay["last_runtime_status_ref"].as_str() == Some(format!("runtime-status://dexterity/{lane}/cloud-consent-revoked").as_str())
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
        projection_plan_id: String::new(),
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
