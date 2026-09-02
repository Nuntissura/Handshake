use surrealdb::types::{Datetime, RecordId, RecordIdKey, SurrealValue};

use super::SurrealStorage;
use crate::storage::{
    MergeBackArtifact, ModelSession, ModelSessionState, NewModelSession, NewSessionMessage,
    SessionCheckpoint, SessionMessage, SessionMessageRole, StorageError, StorageResult,
};

const MODEL_SESSIONS: &str = "model_sessions";
const SESSION_CHECKPOINTS: &str = "model_session_checkpoints";
const SESSION_MESSAGES: &str = "model_session_messages";

#[derive(SurrealValue)]
struct ModelSessionRow {
    id: RecordId,
    parent_session_id: Option<RecordId>,
    spawn_depth: i32,
    state: String,
    model_id: String,
    backend: String,
    parameter_class: String,
    role: String,
    wp_id: Option<String>,
    mt_id: Option<String>,
    work_profile_id: Option<String>,
    execution_mode: String,
    memory_policy: String,
    consent_receipt_id: Option<String>,
    capability_grants: Vec<String>,
    capability_token_ids: Option<Vec<String>>,
    job_id: Option<RecordId>,
    checkpoint_artifact_id: Option<String>,
    last_checkpoint_at: Option<Datetime>,
    checkpoint_count: i64,
    merge_back_artifact: Option<serde_json::Value>,
    agent: Option<String>,
    purpose: Option<String>,
    close_reason: Option<String>,
    closed_by_actor: Option<String>,
    closed_at: Option<Datetime>,
    created_at: Datetime,
    updated_at: Datetime,
}

#[derive(SurrealValue)]
struct SessionUpsertBindings {
    record: RecordId,
    parent_session: Option<RecordId>,
    spawn_depth: i32,
    state: String,
    model_id: String,
    backend: String,
    parameter_class: String,
    role: String,
    wp_id: Option<String>,
    mt_id: Option<String>,
    work_profile_id: Option<String>,
    execution_mode: String,
    memory_policy: String,
    consent_receipt_id: Option<String>,
    capability_grants: Vec<String>,
    capability_token_ids: Option<Vec<String>>,
    job: Option<RecordId>,
    checkpoint_artifact_id: Option<String>,
    last_checkpoint_at: Option<Datetime>,
    checkpoint_count: i64,
    agent: Option<String>,
    purpose: Option<String>,
    now: Datetime,
}

#[derive(SurrealValue)]
struct RecordBinding {
    record: RecordId,
}

#[derive(SurrealValue)]
struct JobBinding {
    job: RecordId,
}

#[derive(SurrealValue)]
struct SessionStateBindings {
    record: RecordId,
    state: String,
    job: Option<RecordId>,
    merge_back_artifact: Option<serde_json::Value>,
    now: Datetime,
}

#[derive(SurrealValue)]
struct SessionCloseBindings {
    record: RecordId,
    state: String,
    close_reason: String,
    actor: String,
    now: Datetime,
}

#[derive(Clone, SurrealValue)]
struct CheckpointRow {
    id: RecordId,
    session_id: RecordId,
    timestamp: Datetime,
    session_state_json: String,
    message_thread_tail_id: String,
    pending_tool_calls_json: String,
    checkpoint_artifact_id: String,
}

#[derive(SurrealValue)]
struct CheckpointBindings {
    record: RecordId,
    model_session: RecordId,
    timestamp: Datetime,
    session_state_json: String,
    message_thread_tail_id: String,
    pending_tool_calls_json: String,
    checkpoint_artifact_id: String,
}

#[derive(Clone, SurrealValue)]
struct MessageRow {
    id: RecordId,
    session_id: RecordId,
    role: String,
    content_hash: String,
    content_artifact_id: String,
    token_count: Option<i64>,
    redacted: bool,
    tool_call_id: Option<String>,
    attachments: Vec<String>,
    created_at: Datetime,
}

#[derive(SurrealValue)]
struct MessageBindings {
    record: RecordId,
    model_session: RecordId,
    role: String,
    content_hash: String,
    content_artifact_id: String,
    token_count: Option<i64>,
    redacted: bool,
    tool_call_id: Option<String>,
    attachments: Vec<String>,
    now: Datetime,
}

pub(crate) async fn upsert_model_session(
    storage: &SurrealStorage,
    session: NewModelSession,
) -> StorageResult<ModelSession> {
    if session.session_id.trim().is_empty() || session.memory_policy.trim().is_empty() {
        return Err(StorageError::Validation(
            "session_id and memory_policy are required",
        ));
    }
    let now = chrono::Utc::now();
    let bindings = SessionUpsertBindings {
        record: RecordId::new(MODEL_SESSIONS, session.session_id),
        parent_session: session
            .parent_session_id
            .map(|id| RecordId::new(MODEL_SESSIONS, id)),
        spawn_depth: session.spawn_depth,
        state: session.state.as_str().to_owned(),
        model_id: session.model_id,
        backend: session.backend,
        parameter_class: session.parameter_class,
        role: session.role,
        wp_id: session.wp_id,
        mt_id: session.mt_id,
        work_profile_id: session.work_profile_id,
        execution_mode: session.execution_mode,
        memory_policy: session.memory_policy,
        consent_receipt_id: session.consent_receipt_id,
        capability_grants: session.capability_grants,
        capability_token_ids: session.capability_token_ids,
        job: session
            .job_id
            .map(|id| RecordId::new("ai_jobs", id.to_string())),
        checkpoint_artifact_id: session.checkpoint_artifact_id,
        last_checkpoint_at: session.last_checkpoint_at.map(Datetime::from),
        checkpoint_count: session.checkpoint_count,
        agent: session.agent,
        purpose: session.purpose,
        now: Datetime::from(now),
    };
    let rows: Vec<ModelSessionRow> = storage
        .with_data_operation(move |database| {
            Box::pin(async move {
                database
                    .query_values_at(
                        "BEGIN TRANSACTION; \
                         IF (SELECT VALUE memory_policy FROM $record)[0] != NONE \
                            AND (SELECT VALUE memory_policy FROM $record)[0] != $memory_policy { \
                            THROW 'HSK-MODEL-SESSION-MEMORY-POLICY-CONFLICT'; \
                         }; \
                         UPSERT $record SET parent_session_id = $parent_session, spawn_depth = $spawn_depth, \
                            state = $state, model_id = $model_id, backend = $backend, \
                            parameter_class = $parameter_class, role = $role, wp_id = $wp_id, mt_id = $mt_id, \
                            work_profile_id = $work_profile_id, execution_mode = $execution_mode, \
                            memory_policy = $memory_policy, consent_receipt_id = $consent_receipt_id, \
                            capability_grants = $capability_grants, capability_token_ids = $capability_token_ids, \
                            job_id = $job, checkpoint_artifact_id = $checkpoint_artifact_id, \
                            last_checkpoint_at = $last_checkpoint_at, checkpoint_count = $checkpoint_count, \
                            agent = $agent, purpose = $purpose, created_at = created_at ?? $now, updated_at = $now; \
                         COMMIT TRANSACTION; \
                         SELECT * FROM $record;",
                        bindings,
                        4,
                    )
                    .await
            })
        })
        .await
        .map_err(map_session_error)?;
    rows.into_iter()
        .next()
        .map(map_model_session)
        .transpose()?
        .ok_or_else(|| StorageError::Database("model session upsert returned no row".to_owned()))
}

pub(crate) async fn get_model_session(
    storage: &SurrealStorage,
    session_id: &str,
) -> StorageResult<ModelSession> {
    let row: Option<ModelSessionRow> = storage
        .with_data_operation({
            let session_id = session_id.to_owned();
            move |database| {
                Box::pin(async move { database.select_one(MODEL_SESSIONS, &session_id).await })
            }
        })
        .await
        .map_err(StorageError::from)?;
    row.map(map_model_session)
        .transpose()?
        .ok_or(StorageError::NotFound("model_session"))
}

pub(crate) async fn get_model_session_by_job_id(
    storage: &SurrealStorage,
    job_id: uuid::Uuid,
) -> StorageResult<ModelSession> {
    let row: Option<ModelSessionRow> = storage
        .with_data_operation(move |database| {
            Box::pin(async move {
                database
                    .query_first(
                        "SELECT * FROM model_sessions WHERE job_id = $job LIMIT 1;",
                        JobBinding {
                            job: RecordId::new("ai_jobs", job_id.to_string()),
                        },
                    )
                    .await
            })
        })
        .await
        .map_err(StorageError::from)?;
    row.map(map_model_session)
        .transpose()?
        .ok_or(StorageError::NotFound("model_session"))
}

pub(crate) async fn update_model_session_state(
    storage: &SurrealStorage,
    session_id: &str,
    state: ModelSessionState,
    job_id: Option<uuid::Uuid>,
    merge_back_artifact: Option<MergeBackArtifact>,
) -> StorageResult<ModelSession> {
    let bindings = SessionStateBindings {
        record: RecordId::new(MODEL_SESSIONS, session_id.to_owned()),
        state: state.as_str().to_owned(),
        job: job_id.map(|id| RecordId::new("ai_jobs", id.to_string())),
        merge_back_artifact: merge_back_artifact.map(serde_json::to_value).transpose()?,
        now: Datetime::from(chrono::Utc::now()),
    };
    let row: Option<ModelSessionRow> = storage
        .with_data_operation(move |database| {
            Box::pin(async move {
                database
                    .query_first(
                        "UPDATE $record SET state = $state, job_id = $job ?? job_id, \
                         merge_back_artifact = $merge_back_artifact ?? merge_back_artifact, \
                         updated_at = $now RETURN AFTER;",
                        bindings,
                    )
                    .await
            })
        })
        .await
        .map_err(map_session_error)?;
    row.map(map_model_session)
        .transpose()?
        .ok_or(StorageError::NotFound("model_session"))
}

pub(crate) async fn close_model_session(
    storage: &SurrealStorage,
    session_id: &str,
    state: ModelSessionState,
    close_reason: &str,
    actor: &str,
) -> StorageResult<ModelSession> {
    if !state.is_terminal() {
        return Err(StorageError::Validation(
            "model session close state must be terminal",
        ));
    }
    if close_reason.trim().is_empty() || actor.trim().is_empty() {
        return Err(StorageError::Validation(
            "close_reason and actor are required",
        ));
    }
    let bindings = SessionCloseBindings {
        record: RecordId::new(MODEL_SESSIONS, session_id.to_owned()),
        state: state.as_str().to_owned(),
        close_reason: close_reason.to_owned(),
        actor: actor.to_owned(),
        now: Datetime::from(chrono::Utc::now()),
    };
    let row: Option<ModelSessionRow> = storage
        .with_data_operation(move |database| {
            Box::pin(async move {
                database
                    .query_first(
                        "UPDATE $record SET state = $state, close_reason = $close_reason, \
                         closed_by_actor = $actor, closed_at = $now, updated_at = $now RETURN AFTER;",
                        bindings,
                    )
                    .await
            })
        })
        .await
        .map_err(StorageError::from)?;
    row.map(map_model_session)
        .transpose()?
        .ok_or(StorageError::NotFound("model_session"))
}

pub(crate) async fn create_session_checkpoint(
    storage: &SurrealStorage,
    checkpoint: SessionCheckpoint,
) -> StorageResult<SessionCheckpoint> {
    let candidate = checkpoint.clone();
    let bindings = CheckpointBindings {
        record: RecordId::new(SESSION_CHECKPOINTS, checkpoint.checkpoint_id),
        model_session: RecordId::new(MODEL_SESSIONS, checkpoint.session_id),
        timestamp: Datetime::from(checkpoint.timestamp),
        session_state_json: checkpoint.session_state_json,
        message_thread_tail_id: checkpoint.message_thread_tail_id,
        pending_tool_calls_json: checkpoint.pending_tool_calls_json,
        checkpoint_artifact_id: checkpoint.checkpoint_artifact_id,
    };
    let rows: Vec<CheckpointRow> = storage
        .with_data_operation(move |database| {
            Box::pin(async move {
                database
                    .query_values_at(
                        "BEGIN TRANSACTION; \
                         IF (SELECT VALUE id FROM $model_session)[0] = NONE { THROW 'HSK-MODEL-SESSION-MISSING'; }; \
                         IF (SELECT VALUE id FROM $record)[0] = NONE { \
                            CREATE $record SET session_id = $model_session, timestamp = $timestamp, \
                              session_state_json = $session_state_json, message_thread_tail_id = $message_thread_tail_id, \
                              pending_tool_calls_json = $pending_tool_calls_json, checkpoint_artifact_id = $checkpoint_artifact_id; \
                            UPDATE $model_session SET checkpoint_artifact_id = $checkpoint_artifact_id, \
                              last_checkpoint_at = $timestamp, checkpoint_count += 1, updated_at = $timestamp; \
                         }; \
                         COMMIT TRANSACTION; \
                         SELECT * FROM $record;",
                        bindings,
                        4,
                    )
                    .await
            })
        })
        .await
        .map_err(map_session_error)?;
    let stored = rows
        .into_iter()
        .next()
        .map(map_checkpoint)
        .transpose()?
        .ok_or_else(|| {
            StorageError::Database("session checkpoint create returned no row".to_owned())
        })?;
    if serde_json::to_value(&stored)? != serde_json::to_value(&candidate)? {
        return Err(StorageError::Conflict(
            "checkpoint id was reused with different content",
        ));
    }
    Ok(stored)
}

pub(crate) async fn get_latest_session_checkpoint(
    storage: &SurrealStorage,
    session_id: &str,
) -> StorageResult<SessionCheckpoint> {
    let row: Option<CheckpointRow> = storage
        .with_data_operation({
            let session_id = session_id.to_owned();
            move |database| {
                Box::pin(async move {
                    database
                        .query_first(
                            "SELECT * FROM model_session_checkpoints \
                             WHERE session_id = $record ORDER BY timestamp DESC, id DESC LIMIT 1;",
                            RecordBinding {
                                record: RecordId::new(MODEL_SESSIONS, session_id),
                            },
                        )
                        .await
                })
            }
        })
        .await
        .map_err(StorageError::from)?;
    row.map(map_checkpoint)
        .transpose()?
        .ok_or(StorageError::NotFound("session_checkpoint"))
}

pub(crate) async fn append_session_message(
    storage: &SurrealStorage,
    message: NewSessionMessage,
) -> StorageResult<SessionMessage> {
    let message_id = message
        .message_id
        .unwrap_or_else(|| uuid::Uuid::now_v7().to_string());
    let now = chrono::Utc::now();
    let candidate = SessionMessage {
        message_id: message_id.clone(),
        session_id: message.session_id.clone(),
        role: message.role.clone(),
        content_hash: message.content_hash.clone(),
        content_artifact_id: message.content_artifact_id.clone(),
        token_count: message.token_count,
        redacted: message.redacted,
        tool_call_id: message.tool_call_id.clone(),
        attachments: message.attachments.clone(),
        created_at: now,
    };
    let bindings = MessageBindings {
        record: RecordId::new(SESSION_MESSAGES, message_id),
        model_session: RecordId::new(MODEL_SESSIONS, message.session_id),
        role: message.role.as_str().to_owned(),
        content_hash: message.content_hash,
        content_artifact_id: message.content_artifact_id,
        token_count: message.token_count,
        redacted: message.redacted,
        tool_call_id: message.tool_call_id,
        attachments: message.attachments,
        now: Datetime::from(now),
    };
    let rows: Vec<MessageRow> = storage
        .with_data_operation(move |database| {
            Box::pin(async move {
                database
                    .query_values_at(
                        "BEGIN TRANSACTION; \
                         IF (SELECT VALUE id FROM $model_session)[0] = NONE { THROW 'HSK-MODEL-SESSION-MISSING'; }; \
                         IF (SELECT VALUE id FROM $record)[0] = NONE { \
                            CREATE $record SET session_id = $model_session, role = $role, content_hash = $content_hash, \
                              content_artifact_id = $content_artifact_id, token_count = $token_count, \
                              redacted = $redacted, tool_call_id = $tool_call_id, attachments = $attachments, \
                              created_at = $now; \
                         }; \
                         COMMIT TRANSACTION; \
                         SELECT * FROM $record;",
                        bindings,
                        4,
                    )
                    .await
            })
        })
        .await
        .map_err(map_session_error)?;
    let stored = rows
        .into_iter()
        .next()
        .map(map_message)
        .transpose()?
        .ok_or_else(|| {
            StorageError::Database("session message append returned no row".to_owned())
        })?;
    if !same_message_content(&stored, &candidate) {
        return Err(StorageError::Conflict(
            "session message id was reused with different content",
        ));
    }
    Ok(stored)
}

pub(crate) async fn list_session_messages(
    storage: &SurrealStorage,
    session_id: &str,
) -> StorageResult<Vec<SessionMessage>> {
    let rows: Vec<MessageRow> = storage
        .with_data_operation({
            let session_id = session_id.to_owned();
            move |database| {
                Box::pin(async move {
                    database
                        .query_values(
                            "SELECT * FROM model_session_messages WHERE session_id = $record \
                             ORDER BY created_at ASC, id ASC;",
                            RecordBinding {
                                record: RecordId::new(MODEL_SESSIONS, session_id),
                            },
                        )
                        .await
                })
            }
        })
        .await
        .map_err(StorageError::from)?;
    rows.into_iter().map(map_message).collect()
}

fn key(record: RecordId) -> StorageResult<String> {
    match record.key {
        RecordIdKey::String(value) => Ok(value),
        _ => Err(StorageError::Database(
            "model session record has a non-string id".to_owned(),
        )),
    }
}

fn map_model_session(row: ModelSessionRow) -> StorageResult<ModelSession> {
    Ok(ModelSession {
        session_id: key(row.id)?,
        parent_session_id: row.parent_session_id.map(key).transpose()?,
        spawn_depth: row.spawn_depth,
        state: ModelSessionState::try_from(row.state.as_str())?,
        model_id: row.model_id,
        backend: row.backend,
        parameter_class: row.parameter_class,
        role: row.role,
        wp_id: row.wp_id,
        mt_id: row.mt_id,
        work_profile_id: row.work_profile_id,
        execution_mode: row.execution_mode,
        memory_policy: row.memory_policy,
        consent_receipt_id: row.consent_receipt_id,
        capability_grants: row.capability_grants,
        capability_token_ids: row.capability_token_ids,
        job_id: row
            .job_id
            .map(key)
            .transpose()?
            .map(|id| uuid::Uuid::parse_str(&id))
            .transpose()
            .map_err(|_| StorageError::Validation("invalid model session job id"))?,
        checkpoint_artifact_id: row.checkpoint_artifact_id,
        last_checkpoint_at: row.last_checkpoint_at.map(|value| value.into_inner()),
        checkpoint_count: row.checkpoint_count,
        merge_back_artifact: row
            .merge_back_artifact
            .map(serde_json::from_value)
            .transpose()?,
        agent: row.agent,
        purpose: row.purpose,
        close_reason: row.close_reason,
        closed_by_actor: row.closed_by_actor,
        closed_at: row.closed_at.map(|value| value.into_inner()),
        created_at: row.created_at.into_inner(),
        updated_at: row.updated_at.into_inner(),
    })
}

fn map_checkpoint(row: CheckpointRow) -> StorageResult<SessionCheckpoint> {
    Ok(SessionCheckpoint {
        checkpoint_id: key(row.id)?,
        session_id: key(row.session_id)?,
        timestamp: row.timestamp.into_inner(),
        session_state_json: row.session_state_json,
        message_thread_tail_id: row.message_thread_tail_id,
        pending_tool_calls_json: row.pending_tool_calls_json,
        checkpoint_artifact_id: row.checkpoint_artifact_id,
    })
}

fn map_message(row: MessageRow) -> StorageResult<SessionMessage> {
    Ok(SessionMessage {
        message_id: key(row.id)?,
        session_id: key(row.session_id)?,
        role: SessionMessageRole::try_from(row.role.as_str())?,
        content_hash: row.content_hash,
        content_artifact_id: row.content_artifact_id,
        token_count: row.token_count,
        redacted: row.redacted,
        tool_call_id: row.tool_call_id,
        attachments: row.attachments,
        created_at: row.created_at.into_inner(),
    })
}

fn same_message_content(left: &SessionMessage, right: &SessionMessage) -> bool {
    left.message_id == right.message_id
        && left.session_id == right.session_id
        && left.role == right.role
        && left.content_hash == right.content_hash
        && left.content_artifact_id == right.content_artifact_id
        && left.token_count == right.token_count
        && left.redacted == right.redacted
        && left.tool_call_id == right.tool_call_id
        && left.attachments == right.attachments
}

fn map_session_error(error: super::SurrealStorageError) -> StorageError {
    let message = error.to_string();
    if message.contains("HSK-MODEL-SESSION-MEMORY-POLICY-CONFLICT") {
        StorageError::Conflict("model session memory_policy is immutable")
    } else if message.contains("HSK-MODEL-SESSION-MISSING") {
        StorageError::NotFound("model_session")
    } else if message.contains("idx_model_sessions_job") {
        StorageError::Conflict("model session job_id already assigned")
    } else {
        StorageError::from(error)
    }
}

#[cfg(test)]
mod tests {
    use super::super::{schema, SurrealStorageConfig};
    use super::*;

    async fn open(path: &std::path::Path) -> SurrealStorage {
        let storage = SurrealStorage::open(
            SurrealStorageConfig::with_path(path).expect("valid embedded session test path"),
        )
        .await
        .expect("open embedded session store");
        schema::bootstrap_schema(&storage)
            .await
            .expect("bootstrap embedded session schema");
        storage
    }

    fn new_session(memory_policy: &str) -> NewModelSession {
        NewModelSession {
            session_id: "mt-137-model-session".to_owned(),
            parent_session_id: None,
            spawn_depth: 0,
            state: ModelSessionState::Created,
            model_id: "test-model".to_owned(),
            backend: "embedded-test".to_owned(),
            parameter_class: "test".to_owned(),
            role: "coder".to_owned(),
            wp_id: Some("WP-KERNEL-012".to_owned()),
            mt_id: Some("MT-137".to_owned()),
            work_profile_id: None,
            execution_mode: "test".to_owned(),
            memory_policy: memory_policy.to_owned(),
            consent_receipt_id: None,
            capability_grants: vec!["storage.read".to_owned()],
            capability_token_ids: None,
            job_id: None,
            checkpoint_artifact_id: None,
            last_checkpoint_at: None,
            checkpoint_count: 0,
            agent: Some("mt137-database-methods".to_owned()),
            purpose: Some("embedded durability proof".to_owned()),
        }
    }

    fn checkpoint(state: &str) -> SessionCheckpoint {
        SessionCheckpoint {
            checkpoint_id: "mt-137-checkpoint".to_owned(),
            session_id: "mt-137-model-session".to_owned(),
            timestamp: chrono::Utc::now(),
            session_state_json: state.to_owned(),
            message_thread_tail_id: "mt-137-message".to_owned(),
            pending_tool_calls_json: "[]".to_owned(),
            checkpoint_artifact_id: "artifact:mt-137-checkpoint".to_owned(),
        }
    }

    fn message(content_hash: &str) -> NewSessionMessage {
        NewSessionMessage {
            message_id: Some("mt-137-message".to_owned()),
            session_id: "mt-137-model-session".to_owned(),
            role: SessionMessageRole::Assistant,
            content_hash: content_hash.to_owned(),
            content_artifact_id: "artifact:mt-137-message".to_owned(),
            token_count: Some(7),
            redacted: false,
            tool_call_id: None,
            attachments: vec!["artifact:attachment".to_owned()],
        }
    }

    #[tokio::test]
    async fn session_checkpoint_message_and_close_survive_shutdown_reopen() {
        let directory = tempfile::tempdir().expect("temporary model-session root");
        let path = directory.path().join("store");
        let storage = open(&path).await;

        upsert_model_session(&storage, new_session("workspace_scoped"))
            .await
            .expect("create model session");
        let saved_checkpoint = checkpoint(r#"{"state":"active"}"#);
        create_session_checkpoint(&storage, saved_checkpoint.clone())
            .await
            .expect("create checkpoint");
        create_session_checkpoint(&storage, saved_checkpoint)
            .await
            .expect("exact checkpoint retry is idempotent");
        append_session_message(&storage, message("sha256:message"))
            .await
            .expect("append message");
        append_session_message(&storage, message("sha256:message"))
            .await
            .expect("exact message retry is idempotent");
        close_model_session(
            &storage,
            "mt-137-model-session",
            ModelSessionState::Completed,
            "work completed",
            "operator:test",
        )
        .await
        .expect("close model session");
        storage.shutdown().await.expect("close embedded store");
        drop(storage);

        let reopened = open(&path).await;
        let session = get_model_session(&reopened, "mt-137-model-session")
            .await
            .expect("read reopened model session");
        assert_eq!(session.state, ModelSessionState::Completed);
        assert_eq!(session.close_reason.as_deref(), Some("work completed"));
        assert_eq!(session.closed_by_actor.as_deref(), Some("operator:test"));
        assert!(session.closed_at.is_some());
        assert_eq!(session.checkpoint_count, 1);
        assert_eq!(
            get_latest_session_checkpoint(&reopened, "mt-137-model-session")
                .await
                .expect("read reopened checkpoint")
                .checkpoint_id,
            "mt-137-checkpoint"
        );
        assert_eq!(
            list_session_messages(&reopened, "mt-137-model-session")
                .await
                .expect("read reopened messages")
                .len(),
            1
        );
        reopened.shutdown().await.expect("close reopened store");
    }

    #[tokio::test]
    async fn immutable_session_and_idempotency_conflicts_fail_closed() {
        let directory = tempfile::tempdir().expect("temporary model-session root");
        let storage = open(&directory.path().join("store")).await;
        upsert_model_session(&storage, new_session("workspace_scoped"))
            .await
            .expect("create model session");

        let memory_policy_conflict =
            upsert_model_session(&storage, new_session("global_unbounded")).await;
        assert!(matches!(
            memory_policy_conflict,
            Err(StorageError::Conflict(_))
        ));

        create_session_checkpoint(&storage, checkpoint(r#"{"state":"active"}"#))
            .await
            .expect("create checkpoint");
        let checkpoint_conflict =
            create_session_checkpoint(&storage, checkpoint(r#"{"state":"different"}"#)).await;
        assert!(matches!(
            checkpoint_conflict,
            Err(StorageError::Conflict(_))
        ));

        append_session_message(&storage, message("sha256:original"))
            .await
            .expect("append message");
        let message_conflict = append_session_message(&storage, message("sha256:different")).await;
        assert!(matches!(message_conflict, Err(StorageError::Conflict(_))));
        assert!(matches!(
            close_model_session(
                &storage,
                "mt-137-model-session",
                ModelSessionState::Active,
                "not terminal",
                "operator:test",
            )
            .await,
            Err(StorageError::Validation(_))
        ));
        storage.shutdown().await.expect("close embedded store");
    }
}

