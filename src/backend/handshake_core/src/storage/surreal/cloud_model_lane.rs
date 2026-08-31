use super::{SurrealStorage, SurrealStorageError};
use serde::{Deserialize, Serialize};
use std::fmt;
use surrealdb::types::SurrealValue;

const SCHEMA: &str = include_str!("cloud_model_lane_schema.surql");
const SCHEMA_STATE: &str = "\
DEFINE TABLE IF NOT EXISTS model_lane_cloud_schema_state SCHEMAFULL;\
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
        let record_id = stable_record_id(kind.as_str(), idempotency_key);
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
                            "BEGIN TRANSACTION;\
                             LET $existing = (SELECT record_json, event_id, event_seq, event_payload_json FROM type::record('model_lane_cloud_authority', $record_id) WHERE owner_account_id = $owner_account_id AND actor_principal_id = $actor_principal_id AND authenticated_session_id = $authenticated_session_id AND access_space_id = $access_space_id AND workspace_id = $workspace_id);\
                             IF array::len($existing) > 0 { RETURN $existing; } ELSE {\
                               LET $ledger = CREATE type::record('model_lane_cloud_event_ledger', $event_id) CONTENT { aggregate_kind: $kind, aggregate_id: $aggregate_id, run_id: $run_id, event_seq: $event_seq, payload_json: $event_payload_json, owner_account_id: $owner_account_id, actor_principal_id: $actor_principal_id, authenticated_session_id: $authenticated_session_id, access_space_id: $access_space_id, workspace_id: $workspace_id };\
                               RETURN CREATE type::record('model_lane_cloud_authority', $record_id) CONTENT { kind: $kind, aggregate_id: $aggregate_id, run_id: $run_id, projection_plan_id: $projection_plan_id, consent_receipt_id: $consent_receipt_id, idempotency_key: $idempotency_key, record_json: $record_json, event_id: $event_id, event_seq: $event_seq, event_payload_json: $event_payload_json, owner_account_id: $owner_account_id, actor_principal_id: $actor_principal_id, authenticated_session_id: $authenticated_session_id, access_space_id: $access_space_id, workspace_id: $workspace_id };\
                             };\
                             COMMIT TRANSACTION;",
                            bindings,
                            2,
                        )
                        .await
                })
            })
            .await?;
        rows.into_iter().next().map(Into::into).ok_or(
            SurrealStorageError::InvalidCloudModelLaneRecord {
                reason: "authority write returned no row",
            },
        )
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

    async fn query_rows(
        &self,
        statement: &'static str,
        bindings: ScopedLookupBindings,
    ) -> Result<Vec<CloudModelLaneStoredRow>, SurrealStorageError> {
        let rows = self
            .storage
            .with_data_operation(|database| {
                Box::pin(async move {
                    database
                        .query_values::<StoredRowValue, _>(statement, bindings)
                        .await
                })
            })
            .await?;
        Ok(rows.into_iter().map(Into::into).collect())
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

fn stable_record_id(kind: &str, idempotency_key: &str) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(b"handshake.model-lane.cloud-authority.v1\0");
    hasher.update(kind.as_bytes());
    hasher.update([0]);
    hasher.update(idempotency_key.as_bytes());
    hex::encode(hasher.finalize())
}
