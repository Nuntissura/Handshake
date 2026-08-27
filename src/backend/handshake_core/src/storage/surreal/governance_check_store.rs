use surrealdb::types::{Datetime, RecordId, RecordIdKey, SurrealValue};

use super::SurrealStorage;
use crate::storage::{
    GovernanceCheckRun, MutationMetadata, NewGovernanceCheckRun, StorageError, StorageResult,
};

const TABLE: &str = "governance_check_runs";

#[derive(SurrealValue)]
struct GovernanceCheckRunRow {
    id: RecordId,
    check_id: String,
    session_id: String,
    check_name: String,
    check_kind: String,
    descriptor_hash: String,
    result_status: String,
    checks_duration_ms: i64,
    evidence_artifact_id: Option<String>,
    evidence_artifact_content_hash: Option<String>,
    created_at: Datetime,
    updated_at: Datetime,
}

#[derive(SurrealValue)]
struct GovernanceCheckRunContent {
    check_id: String,
    session_id: String,
    check_name: String,
    check_kind: String,
    descriptor_hash: String,
    result_status: String,
    checks_duration_ms: i64,
    evidence_artifact_id: Option<String>,
    evidence_artifact_content_hash: Option<String>,
    last_job_id: Option<String>,
    last_workflow_id: Option<String>,
    last_actor_id: Option<String>,
    edit_event_id: String,
    last_actor_kind: String,
    created_at: Datetime,
    updated_at: Datetime,
}

#[derive(SurrealValue)]
struct CreateBindings {
    record: RecordId,
    content: GovernanceCheckRunContent,
}

#[derive(SurrealValue)]
struct SessionBinding {
    session_id: String,
}

pub(crate) async fn create_governance_check_run(
    storage: &SurrealStorage,
    run_id: uuid::Uuid,
    run: NewGovernanceCheckRun,
    metadata: MutationMetadata,
) -> StorageResult<GovernanceCheckRun> {
    validate_required(&run.check_name, "governance check name must not be blank")?;
    validate_required(&run.check_kind, "governance check kind must not be blank")?;
    validate_required(
        &run.descriptor_hash,
        "governance check descriptor hash must not be blank",
    )?;
    validate_required(
        &run.result_status,
        "governance check result status must not be blank",
    )?;
    if metadata.resource_id != run_id.to_string() {
        return Err(StorageError::Guard(
            "governance check mutation resource mismatch",
        ));
    }
    let checks_duration_ms = i64::try_from(run.checks_duration_ms)
        .map_err(|_| StorageError::Validation("governance check duration exceeds i64"))?;
    let bindings = CreateBindings {
        record: RecordId::new(TABLE, run_id.to_string()),
        content: GovernanceCheckRunContent {
            check_id: run.check_id.to_string(),
            session_id: run.session_id.to_string(),
            check_name: run.check_name,
            check_kind: run.check_kind,
            descriptor_hash: run.descriptor_hash,
            result_status: run.result_status,
            checks_duration_ms,
            evidence_artifact_id: run.evidence_artifact_id,
            evidence_artifact_content_hash: run.evidence_artifact_content_hash,
            last_job_id: metadata.job_id.map(|id| id.to_string()),
            last_workflow_id: metadata.workflow_id.map(|id| id.to_string()),
            last_actor_id: metadata.actor_id,
            edit_event_id: metadata.edit_event_id.to_string(),
            last_actor_kind: metadata.actor_kind.as_str().to_owned(),
            created_at: Datetime::from(metadata.timestamp),
            updated_at: Datetime::from(metadata.timestamp),
        },
    };
    let rows: Vec<GovernanceCheckRunRow> = storage
        .with_data_operation(move |database| {
            Box::pin(async move {
                database
                    .query_values_at(
                        "BEGIN TRANSACTION; CREATE $record CONTENT $content; COMMIT TRANSACTION; SELECT id, check_id, session_id, check_name, check_kind, descriptor_hash, result_status, checks_duration_ms, evidence_artifact_id, evidence_artifact_content_hash, created_at, updated_at FROM $record;",
                        bindings,
                        3,
                    )
                    .await
            })
        })
        .await
        .map_err(StorageError::from)?;
    rows.into_iter()
        .next()
        .map(row_to_domain)
        .transpose()?
        .ok_or_else(|| {
            StorageError::Database("governance check run create returned no row".to_owned())
        })
}

pub(crate) async fn list_governance_check_runs(
    storage: &SurrealStorage,
    session_id: uuid::Uuid,
) -> StorageResult<Vec<GovernanceCheckRun>> {
    let rows: Vec<GovernanceCheckRunRow> = storage
        .with_data_operation(move |database| {
            Box::pin(async move {
                database
                    .query_values(
                        "SELECT id, check_id, session_id, check_name, check_kind, descriptor_hash, result_status, checks_duration_ms, evidence_artifact_id, evidence_artifact_content_hash, created_at, updated_at FROM governance_check_runs WHERE session_id = $session_id ORDER BY created_at ASC, id ASC;",
                        SessionBinding {
                            session_id: session_id.to_string(),
                        },
                    )
                    .await
            })
        })
        .await
        .map_err(StorageError::from)?;
    rows.into_iter().map(row_to_domain).collect()
}

fn validate_required(value: &str, label: &'static str) -> StorageResult<()> {
    if value.trim().is_empty() {
        Err(StorageError::Validation(label))
    } else {
        Ok(())
    }
}

fn row_to_domain(row: GovernanceCheckRunRow) -> StorageResult<GovernanceCheckRun> {
    let checks_duration_ms = u64::try_from(row.checks_duration_ms).map_err(|_| {
        StorageError::Database("governance check run has a negative duration".to_owned())
    })?;
    Ok(GovernanceCheckRun {
        id: record_uuid(row.id, "invalid governance check run id")?,
        check_id: parse_uuid(&row.check_id, "invalid governance check id")?,
        session_id: parse_uuid(&row.session_id, "invalid governance check session id")?,
        check_name: row.check_name,
        check_kind: row.check_kind,
        descriptor_hash: row.descriptor_hash,
        result_status: row.result_status,
        checks_duration_ms,
        evidence_artifact_id: row.evidence_artifact_id,
        evidence_artifact_content_hash: row.evidence_artifact_content_hash,
        created_at: row.created_at.into_inner(),
        updated_at: row.updated_at.into_inner(),
    })
}

fn record_uuid(record: RecordId, label: &'static str) -> StorageResult<uuid::Uuid> {
    match record.key {
        RecordIdKey::String(value) => parse_uuid(&value, label),
        _ => Err(StorageError::Database(label.to_owned())),
    }
}

fn parse_uuid(value: &str, label: &'static str) -> StorageResult<uuid::Uuid> {
    uuid::Uuid::parse_str(value).map_err(|_| StorageError::Database(label.to_owned()))
}

#[cfg(test)]
mod tests {
    use super::super::{schema, SurrealDatabase, SurrealStorageConfig};
    use super::*;
    use crate::storage::{Database, WriteContext};

    async fn open(path: &std::path::Path) -> SurrealStorage {
        let storage = SurrealStorage::open(
            SurrealStorageConfig::with_path(path).expect("valid governance-check test path"),
        )
        .await
        .expect("open embedded governance-check store");
        schema::bootstrap_schema(&storage)
            .await
            .expect("bootstrap governance-check schema");
        storage
    }

    fn new_run(session_id: uuid::Uuid, name: &str, duration: u64) -> NewGovernanceCheckRun {
        NewGovernanceCheckRun {
            check_id: uuid::Uuid::now_v7(),
            session_id,
            check_name: name.to_owned(),
            check_kind: "unit".to_owned(),
            descriptor_hash: format!("descriptor-{name}"),
            result_status: "passed".to_owned(),
            checks_duration_ms: duration,
            evidence_artifact_id: Some(format!("artifact-{name}")),
            evidence_artifact_content_hash: Some(format!("hash-{name}")),
        }
    }

    #[tokio::test]
    async fn governance_check_runs_are_ordered_isolated_and_durable() {
        let directory = tempfile::tempdir().expect("temporary governance-check root");
        let path = directory.path().join("store");
        let storage = open(&path).await;
        let database = SurrealDatabase::new(storage.clone());
        let session_id = uuid::Uuid::now_v7();
        let first = database
            .create_governance_check_run(
                &WriteContext::system(Some("governance-check-test".to_owned())),
                new_run(session_id, "first", 11),
            )
            .await
            .expect("create first governance-check run");
        let second = database
            .create_governance_check_run(
                &WriteContext::system(Some("governance-check-test".to_owned())),
                new_run(session_id, "second", 22),
            )
            .await
            .expect("create second governance-check run");
        assert!(first.created_at <= second.created_at);
        assert_eq!(first.checks_duration_ms, 11);
        assert_eq!(
            second.evidence_artifact_id.as_deref(),
            Some("artifact-second")
        );

        let listed = database
            .list_governance_check_runs(session_id)
            .await
            .expect("list governance-check runs");
        assert_eq!(
            listed.iter().map(|run| run.id).collect::<Vec<_>>(),
            vec![first.id, second.id]
        );
        assert!(database
            .list_governance_check_runs(uuid::Uuid::now_v7())
            .await
            .expect("list missing session")
            .is_empty());
        assert!(matches!(
            database
                .create_governance_check_run(
                    &WriteContext::system(None),
                    NewGovernanceCheckRun {
                        check_name: "  ".to_owned(),
                        ..new_run(session_id, "invalid", 0)
                    },
                )
                .await,
            Err(StorageError::Validation(_))
        ));

        drop(database);
        storage
            .shutdown()
            .await
            .expect("close governance-check store");
        drop(storage);

        let reopened = open(&path).await;
        let reopened_database = SurrealDatabase::new(reopened.clone());
        let persisted = reopened_database
            .list_governance_check_runs(session_id)
            .await
            .expect("list reopened governance-check runs");
        assert_eq!(
            persisted.iter().map(|run| run.id).collect::<Vec<_>>(),
            vec![first.id, second.id]
        );
        assert_eq!(persisted[0].descriptor_hash, "descriptor-first");
        assert_eq!(persisted[1].checks_duration_ms, 22);
        drop(reopened_database);
        reopened
            .shutdown()
            .await
            .expect("close reopened governance-check store");
    }
}
