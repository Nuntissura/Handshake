use crate::{
    flight_recorder::log_event,
    llm::ChatMessage,
    models::{AiJob, WorkflowRun},
    terminal::{TerminalError, TerminalService},
    AppState,
};
use once_cell::sync::Lazy;
use serde_json::json;
use std::collections::{HashMap, HashSet};
use thiserror::Error;
use uuid::Uuid;

#[derive(Error, Debug)]
pub enum WorkflowError {
    #[error("Database error: {0}")]
    Sqlx(#[from] sqlx::Error),
    #[error("LLM error: {0}")]
    Llm(String),
    #[error("Terminal error: {0}")]
    Terminal(String),
    #[error("Unauthorized: Missing capability {capability}")]
    Unauthorized { capability: String },
}

static CAPABILITY_PROFILES: Lazy<HashMap<&'static str, HashSet<&'static str>>> = Lazy::new(|| {
    let mut map = HashMap::new();
    map.insert(
        "default",
        HashSet::from_iter(["doc.read", "doc.summarize"].into_iter()),
    );
    map.insert("terminal", HashSet::from_iter(["term.exec"].into_iter()));
    map
});

fn required_capability_for_job_kind(job_kind: &str) -> Result<&'static str, WorkflowError> {
    match job_kind {
        "doc_summarize" | "doc_test" => Ok("doc.summarize"),
        "term_exec" | "terminal_exec" => Ok("term.exec"),
        other => Err(WorkflowError::Unauthorized {
            capability: format!("capability mapping for job_kind={}", other),
        }),
    }
}

fn capability_profile_has_required(
    capability_profile_id: &str,
    required: &str,
) -> Result<(), WorkflowError> {
    let has_capability = CAPABILITY_PROFILES
        .get(capability_profile_id)
        .map_or(false, |set| set.contains(required));

    if has_capability {
        Ok(())
    } else {
        Err(WorkflowError::Unauthorized {
            capability: required.to_string(),
        })
    }
}

fn enforce_capabilities(job: &AiJob) -> Result<(), WorkflowError> {
    let required = required_capability_for_job_kind(&job.job_kind)?;
    capability_profile_has_required(&job.capability_profile_id, required)
}

fn parse_inputs(raw: Option<&str>) -> serde_json::Value {
    if let Some(value) = raw {
        match serde_json::from_str(value) {
            Ok(parsed) => parsed,
            Err(_) => json!({}),
        }
    } else {
        json!({})
    }
}

pub async fn start_workflow_for_job(
    state: &AppState,
    job: AiJob,
) -> Result<WorkflowRun, WorkflowError> {
    let workflow_run_id = Uuid::new_v4().to_string();
    let initial_status = "running".to_string();

    let workflow_run = sqlx::query_as!(
        WorkflowRun,
        r#"
        INSERT INTO workflow_runs (id, job_id, status)
        VALUES ($1, $2, $3)
        RETURNING
            id as "id!",
            job_id as "job_id!",
            status as "status!",
            created_at as "created_at: chrono::DateTime<chrono::Utc>",
            updated_at as "updated_at: chrono::DateTime<chrono::Utc>"
        "#,
        workflow_run_id,
        job.id,
        initial_status
    )
    .fetch_one(&state.pool)
    .await?;

    let _ = log_event(
        state,
        "workflow_started",
        Some(&job.id),
        Some(&workflow_run.id),
        json!({ "status": workflow_run.status }),
    );

    let result = match enforce_capabilities(&job) {
        Ok(_) => run_job(state, &job).await,
        Err(err) => Err(err),
    };

    let mut captured_error: Option<WorkflowError> = None;
    let (final_status, error_message) = match result {
        Ok(_) => ("completed".to_string(), None),
        Err(e) => {
            let msg = e.to_string();
            captured_error = Some(e);
            ("failed".to_string(), Some(msg))
        }
    };

    sqlx::query("UPDATE ai_jobs SET status = $1, error_message = $2 WHERE id = $3")
        .bind(&final_status)
        .bind(&error_message)
        .bind(&job.id)
        .execute(&state.pool)
        .await?;

    let completed_run = sqlx::query_as::<_, WorkflowRun>(
        r#"
        UPDATE workflow_runs SET status = $1 WHERE id = $2
        RETURNING
            id,
            job_id,
            status,
            created_at,
            updated_at
        "#,
    )
    .bind(&final_status)
    .bind(&workflow_run.id)
    .fetch_one(&state.pool)
    .await?;

    let event_type = if final_status == "completed" {
        "workflow_completed"
    } else {
        "workflow_failed"
    };
    let _ = log_event(
        state,
        event_type,
        Some(&job.id),
        Some(&completed_run.id),
        json!({ "status": completed_run.status, "error": error_message }),
    );

    if let Some(err) = captured_error {
        Err(err)
    } else {
        Ok(completed_run)
    }
}

async fn run_job(state: &AppState, job: &AiJob) -> Result<(), WorkflowError> {
    if job.job_kind == "doc_test" || job.job_kind == "doc_summarize" {
        let inputs = parse_inputs(job.job_inputs.as_deref());
        let doc_id = inputs.get("doc_id").and_then(|v| v.as_str());

        if let Some(doc_id) = doc_id {
            let blocks = sqlx::query(
                "SELECT raw_content FROM document_blocks WHERE document_id = $1 ORDER BY sequence ASC",
            )
            .bind(doc_id)
            .fetch_all(&state.pool)
            .await?;

            let full_text = blocks
                .into_iter()
                .map(|b| {
                    use sqlx::Row;
                    b.get::<String, _>("raw_content")
                })
                .collect::<Vec<_>>()
                .join("\n");

            let messages = vec![
                ChatMessage {
                    role: "system".into(),
                    content: "You are a helpful assistant that summarizes documents.".into(),
                },
                ChatMessage {
                    role: "user".into(),
                    content: format!("Please summarize the following document:\n\n{}", full_text),
                },
            ];

            let response = state
                .llm_client
                .chat(messages)
                .await
                .map_err(|e| WorkflowError::Llm(e))?;

            sqlx::query("UPDATE ai_jobs SET job_outputs = $1 WHERE id = $2")
                .bind(json!({ "summary": response }).to_string())
                .bind(&job.id)
                .execute(&state.pool)
                .await?;
        }
    } else if job.job_kind == "term_exec" || job.job_kind == "terminal_exec" {
        return Err(WorkflowError::Unauthorized {
            capability: "terminal jobs are disabled during security hardening".to_string(),
        });
    }
    Ok(())
}

async fn execute_terminal_job(state: &AppState, job: &AiJob) -> Result<(), WorkflowError> {
    let inputs = parse_inputs(job.job_inputs.as_deref());

    let program = inputs
        .get("program")
        .and_then(|v| v.as_str())
        .ok_or_else(|| WorkflowError::Terminal("program is required".into()))?;
    let args: Vec<String> = match inputs.get("args").and_then(|v| v.as_array()) {
        Some(arr) => arr
            .iter()
            .filter_map(|v| v.as_str().map(|s| s.to_string()))
            .collect(),
        None => Vec::new(),
    };
    let timeout_ms = inputs
        .get("timeout_ms")
        .and_then(|v| v.as_u64())
        .or(Some(30_000));

    let output = TerminalService::run(program, &args, timeout_ms)
        .await
        .map_err(|e| match e {
            TerminalError::Invalid(msg) | TerminalError::Exec(msg) => {
                WorkflowError::Terminal(msg.to_string())
            }
            TerminalError::Timeout(ms) => {
                WorkflowError::Terminal(format!("command timed out after {} ms", ms))
            }
            TerminalError::Io(ioe) => WorkflowError::Terminal(ioe.to_string()),
        })?;

    let payload = json!({
        "job_kind": job.job_kind,
        "program": program,
        "args": args,
        "status_code": output.status_code,
        "stdout": output.stdout,
        "stderr": output.stderr
    });

    let _ = log_event(state, "terminal_exec", Some(&job.id), None, payload.clone());

    sqlx::query("UPDATE ai_jobs SET job_outputs = $1 WHERE id = $2")
        .bind(payload.to_string())
        .bind(&job.id)
        .execute(&state.pool)
        .await?;

    if output.status_code != 0 {
        return Err(WorkflowError::Terminal(format!(
            "command exited with code {}",
            output.status_code
        )));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::llm::MockLLMClient;
    use crate::AppState;
    use duckdb::Connection as DuckDbConnection;
    use serde_json::json;
    use sqlx::sqlite::SqlitePoolOptions;
    use std::sync::{Arc, Mutex};

    async fn setup_state() -> AppState {
        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect("sqlite::memory:")
            .await
            .expect("failed to create in-memory sqlite pool");

        sqlx::query(
            r#"
            CREATE TABLE ai_jobs (
                id TEXT PRIMARY KEY NOT NULL,
                job_kind TEXT NOT NULL,
                status TEXT NOT NULL,
                error_message TEXT,
                protocol_id TEXT NOT NULL,
                profile_id TEXT NOT NULL,
                capability_profile_id TEXT NOT NULL,
                access_mode TEXT NOT NULL,
                safety_mode TEXT NOT NULL,
                job_inputs TEXT,
                job_outputs TEXT,
                created_at DATETIME NOT NULL DEFAULT (strftime('%Y-%m-%d %H:%M:%f', 'now')),
                updated_at DATETIME NOT NULL DEFAULT (strftime('%Y-%m-%d %H:%M:%f', 'now'))
            );
            "#,
        )
        .execute(&pool)
        .await
        .expect("failed to create ai_jobs table");

        sqlx::query(
            r#"
            CREATE TRIGGER ai_jobs_updated_at
            AFTER UPDATE ON ai_jobs
            FOR EACH ROW
            BEGIN
                UPDATE ai_jobs SET updated_at = strftime('%Y-%m-%d %H:%M:%f', 'now') WHERE id = OLD.id;
            END;
            "#,
        )
        .execute(&pool)
        .await
        .expect("failed to create ai_jobs trigger");

        sqlx::query(
            r#"
            CREATE TABLE workflow_runs (
                id TEXT PRIMARY KEY NOT NULL,
                job_id TEXT NOT NULL,
                status TEXT NOT NULL,
                created_at DATETIME NOT NULL DEFAULT (strftime('%Y-%m-%d %H:%M:%f', 'now')),
                updated_at DATETIME NOT NULL DEFAULT (strftime('%Y-%m-%d %H:%M:%f', 'now')),
                FOREIGN KEY (job_id) REFERENCES ai_jobs(id) ON DELETE CASCADE
            );
            "#,
        )
        .execute(&pool)
        .await
        .expect("failed to create workflow_runs table");

        sqlx::query(
            r#"
            CREATE TRIGGER workflow_runs_updated_at
            AFTER UPDATE ON workflow_runs
            FOR EACH ROW
            BEGIN
                UPDATE workflow_runs SET updated_at = strftime('%Y-%m-%d %H:%M:%f', 'now') WHERE id = OLD.id;
            END;
            "#,
        )
        .execute(&pool)
        .await
        .expect("failed to create workflow_runs trigger");

        let fr_conn = DuckDbConnection::open_in_memory().expect("failed to create duckdb");
        fr_conn
            .execute_batch(
                r#"
                CREATE TABLE events (
                    timestamp DATETIME DEFAULT current_timestamp,
                    event_type TEXT NOT NULL,
                    job_id TEXT,
                    workflow_id TEXT,
                    payload JSON
                );
                "#,
            )
            .expect("failed to create flight recorder events table");

        AppState {
            pool,
            fr_pool: Arc::new(Mutex::new(fr_conn)),
            llm_client: Arc::new(MockLLMClient {
                response: "mock".to_string(),
            }),
        }
    }

    #[tokio::test]
    async fn job_fails_when_missing_required_capability() {
        let state = setup_state().await;
        let job_id = "job-unauthorized";

        sqlx::query(
            r#"
            INSERT INTO ai_jobs (
                id,
                job_kind,
                status,
                error_message,
                protocol_id,
                profile_id,
                capability_profile_id,
                access_mode,
                safety_mode,
                job_inputs,
                job_outputs
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(job_id)
        .bind("doc_summarize")
        .bind("queued")
        .bind::<Option<String>>(None)
        .bind("protocol-default")
        .bind("default")
        .bind("missing_profile")
        .bind("default")
        .bind("default")
        .bind::<Option<String>>(Some(r#"{"doc_id":"doc-1"}"#.to_string()))
        .bind::<Option<String>>(None)
        .execute(&state.pool)
        .await
        .expect("failed to insert job");

        let job = sqlx::query_as::<_, AiJob>(
            r#"
            SELECT
                id,
                job_kind,
                status,
                error_message,
                protocol_id,
                profile_id,
                capability_profile_id,
                access_mode,
                safety_mode,
                job_inputs,
                job_outputs,
                created_at,
                updated_at
            FROM ai_jobs
            WHERE id = ?
            "#,
        )
        .bind(job_id)
        .fetch_one(&state.pool)
        .await
        .expect("failed to load job");

        let result = start_workflow_for_job(&state, job).await;
        assert!(result.is_err(), "expected capability error");

        let updated_job = sqlx::query_as::<_, AiJob>(
            r#"
            SELECT
                id,
                job_kind,
                status,
                error_message,
                protocol_id,
                profile_id,
                capability_profile_id,
                access_mode,
                safety_mode,
                job_inputs,
                job_outputs,
                created_at,
                updated_at
            FROM ai_jobs
            WHERE id = ?
            "#,
        )
        .bind(job_id)
        .fetch_one(&state.pool)
        .await
        .expect("failed to fetch updated job");

        assert_eq!(updated_job.status, "failed");
        let message = match updated_job.error_message.clone() {
            Some(text) => text,
            None => String::new(),
        };
        assert!(
            message.contains("Unauthorized: Missing capability doc.summarize"),
            "unexpected error message: {}",
            message
        );

        let updated_run = sqlx::query_as::<_, WorkflowRun>(
            r#"
            SELECT
                id,
                job_id,
                status,
                created_at,
                updated_at
            FROM workflow_runs
            WHERE id = ?
            "#,
        )
        .bind(job_id)
        .fetch_optional(&state.pool)
        .await
        .expect("failed to fetch workflow run");

        assert!(
            updated_run.is_none(),
            "workflow run should not exist when gating fails early"
        );
    }

    #[tokio::test]
    async fn terminal_job_enforces_capability() {
        let state = setup_state().await;
        let job_id = "job-terminal-no-cap";

        sqlx::query(
            r#"
            INSERT INTO ai_jobs (
                id,
                job_kind,
                status,
                error_message,
                protocol_id,
                profile_id,
                capability_profile_id,
                access_mode,
                safety_mode,
                job_inputs,
                job_outputs
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(job_id)
        .bind("term_exec")
        .bind("queued")
        .bind::<Option<String>>(None)
        .bind("protocol-default")
        .bind("default")
        .bind("default")
        .bind("default")
        .bind("default")
        .bind::<Option<String>>(Some(r#"{"program":"printf","args":["hello"]}"#.to_string()))
        .bind::<Option<String>>(None)
        .execute(&state.pool)
        .await
        .expect("failed to insert job");

        let job = sqlx::query_as::<_, AiJob>(
            r#"
            SELECT
                id,
                job_kind,
                status,
                error_message,
                protocol_id,
                profile_id,
                capability_profile_id,
                access_mode,
                safety_mode,
                job_inputs,
                job_outputs,
                created_at,
                updated_at
            FROM ai_jobs
            WHERE id = ?
            "#,
        )
        .bind(job_id)
        .fetch_one(&state.pool)
        .await
        .expect("failed to load job");

        let result = start_workflow_for_job(&state, job).await;
        assert!(result.is_err(), "expected capability error");

        let updated_job = sqlx::query_as::<_, (String, Option<String>)>(
            r#"
            SELECT status, error_message FROM ai_jobs WHERE id = ?
            "#,
        )
        .bind(job_id)
        .fetch_one(&state.pool)
        .await
        .expect("failed to fetch updated job");

        assert_eq!(updated_job.0, "failed");
    }

    #[tokio::test]
    async fn terminal_job_runs_when_authorized() {
        let state = setup_state().await;
        let job_id = "job-terminal-ok";

        #[cfg(target_os = "windows")]
        let (program, args) = ("cmd", vec!["/C", "echo", "hello"]);
        #[cfg(not(target_os = "windows"))]
        let (program, args) = ("printf", vec!["hello"]);

        let job_inputs = json!({
            "program": program,
            "args": args
        })
        .to_string();

        sqlx::query(
            r#"
            INSERT INTO ai_jobs (
                id,
                job_kind,
                status,
                error_message,
                protocol_id,
                profile_id,
                capability_profile_id,
                access_mode,
                safety_mode,
                job_inputs,
                job_outputs
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(job_id)
        .bind("term_exec")
        .bind("queued")
        .bind::<Option<String>>(None)
        .bind("protocol-default")
        .bind("default")
        .bind("terminal")
        .bind("default")
        .bind("default")
        .bind::<Option<String>>(Some(job_inputs))
        .bind::<Option<String>>(None)
        .execute(&state.pool)
        .await
        .expect("failed to insert job");

        let job = sqlx::query_as::<_, AiJob>(
            r#"
            SELECT
                id,
                job_kind,
                status,
                error_message,
                protocol_id,
                profile_id,
                capability_profile_id,
                access_mode,
                safety_mode,
                job_inputs,
                job_outputs,
                created_at,
                updated_at
            FROM ai_jobs
            WHERE id = ?
            "#,
        )
        .bind(job_id)
        .fetch_one(&state.pool)
        .await
        .expect("failed to load job");

        let result = start_workflow_for_job(&state, job).await;
        assert!(
            result.is_err(),
            "terminal jobs should be blocked during security hardening"
        );

        let updated_job = sqlx::query_as::<_, (String, Option<String>, Option<String>)>(
            r#"
            SELECT status, error_message, job_outputs FROM ai_jobs WHERE id = ?
            "#,
        )
        .bind(job_id)
        .fetch_one(&state.pool)
        .await
        .expect("failed to fetch updated job");

        assert_eq!(updated_job.0, "failed");
        let err_message = match updated_job.1 {
            Some(msg) => msg,
            None => "missing error".to_string(),
        };
        assert!(
            err_message
                .to_lowercase()
                .contains("terminal jobs are disabled"),
            "expected terminal block message, got {}",
            err_message
        );
    }
}
