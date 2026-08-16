//! WP-KERNEL-012 MT-045 (LC-06) CodeNavIndexApi: the workspace code-nav INDEX
//! pipeline. The native code editor's large-codebase proof (LC-06) POSTs a
//! machine-local `root_path` and reads back the total number of symbols the
//! index produced (`symbol_count`).
//!
//! PENDING SURREALDB PORT (WP-KERNEL-012 MT-137): this surface still binds
//! `sqlx` against the deleted relational backend and does not compile today.
//! Handshake's only database is the embedded SurrealDB store.
//!
//! This is NOT a thin adapter. It genuinely, over single-store/EventLedger
//! authority (no SQLite):
//!   1. registers `root_path` as a knowledge source root — the MT-081 root
//!      allowlist is enforced FAIL-CLOSED (a denied path is a typed 403, never a
//!      silent skip), then
//!   2. runs the REAL ingestion pass (`IngestionEngine::run_ingestion_pass`) —
//!      the SAME directory walker + per-root file allowlist the ingestion API
//!      uses — to ingest every eligible file as a `KnowledgeSource`, then
//!   3. runs the REAL code indexer (`knowledge_code_index::read_and_index` over
//!      `CodeIndexEngine::index_code_source`) on each ingested CODE/CONFIG
//!      source and sums `symbols_indexed`.
//!
//! Route (workspace-scoped, P2 identity):
//!   * `POST /workspaces/:workspace_id/code-nav/index {root_path}`
//!       -> `{symbol_count, files_indexed, files_failed, files_skipped, root_id,
//!            index_run_id}`
//!
//! Backend-navigation law (spec 2.3.13.11): indexing is a MUTATION, so the
//! identity headers are REQUIRED (400 otherwise) and thread into every ingestion
//! + code-index EventLedger receipt:
//!   * `x-hsk-actor-id`, `x-hsk-kernel-task-run-id`, `x-hsk-session-run-id`
//!   * (optional) `x-hsk-actor-kind` (default `system`), `x-hsk-correlation-id`

use std::path::PathBuf;
use std::sync::Arc;

use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    routing::post,
    Json, Router,
};
use futures_util::stream::{self, StreamExt};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::kernel::KernelActor;
use crate::knowledge_code_index::config_schema::detect_config_format;
use crate::knowledge_code_index::engine::{read_and_index, CodeIndexContext, CodeIndexEngine};
use crate::knowledge_code_index::parser::detect_code_language;
use crate::knowledge_code_index::CodeIndexError;
use crate::knowledge_ingestion::engine::{
    IngestionContext, IngestionEngine, RootRegistrationRequest,
};
use crate::knowledge_ingestion::backpressure::IngestionLimits;
use crate::knowledge_ingestion::IngestionError;
use crate::storage::knowledge::KnowledgeRootKind;
use crate::storage::knowledge::{KnowledgeIndexRunCounts, KnowledgeIndexRunOutcome};
use crate::storage::postgres::PostgresDatabase;
use crate::storage::configured_postgres_parallelism;
use crate::storage::StorageError;
use crate::AppState;

const HSK_HEADER_ACTOR_KIND: &str = "x-hsk-actor-kind";
const HSK_HEADER_ACTOR_ID: &str = "x-hsk-actor-id";
const HSK_HEADER_KERNEL_TASK_RUN_ID: &str = "x-hsk-kernel-task-run-id";
const HSK_HEADER_SESSION_RUN_ID: &str = "x-hsk-session-run-id";
const HSK_HEADER_CORRELATION_ID: &str = "x-hsk-correlation-id";

pub fn routes(state: AppState) -> Router {
    Router::new()
        .route(
            "/workspaces/:workspace_id/code-nav/index",
            post(index_workspace_code),
        )
        .with_state(state)
}

// ---------------------------------------------------------------------------
// Shared plumbing.
// ---------------------------------------------------------------------------

type ApiError = (StatusCode, Json<Value>);

fn header_str<'a>(headers: &'a HeaderMap, name: &str) -> Option<&'a str> {
    headers
        .get(name)
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty())
}

fn bad_request(detail: impl Into<String>) -> ApiError {
    (
        StatusCode::BAD_REQUEST,
        Json(json!({"error": "bad_request", "detail": detail.into()})),
    )
}

/// The backend-navigation identity carried by the index mutation. Both the
/// ingestion pass and the code-index run receive it so their EventLedger
/// receipts are attributable to the same actor/session.
struct IndexIdentity {
    actor: KernelActor,
    kernel_task_run_id: String,
    session_run_id: String,
    correlation_id: Option<String>,
}

impl IndexIdentity {
    fn ingestion_context(&self) -> IngestionContext {
        IngestionContext {
            actor: self.actor.clone(),
            kernel_task_run_id: self.kernel_task_run_id.clone(),
            session_run_id: self.session_run_id.clone(),
            correlation_id: self.correlation_id.clone(),
        }
    }

    fn code_index_context(&self) -> CodeIndexContext {
        CodeIndexContext {
            actor: self.actor.clone(),
            kernel_task_run_id: self.kernel_task_run_id.clone(),
            session_run_id: self.session_run_id.clone(),
            correlation_id: self.correlation_id.clone(),
        }
    }
}

/// Build the index identity from the required mutation headers (400 if any is
/// missing) — mirrors `knowledge_ingestion::mutation_context`.
fn index_identity(headers: &HeaderMap) -> Result<IndexIdentity, ApiError> {
    let actor_id = header_str(headers, HSK_HEADER_ACTOR_ID)
        .ok_or_else(|| bad_request(format!("{HSK_HEADER_ACTOR_ID} header is required")))?
        .to_string();
    let kernel_task_run_id = header_str(headers, HSK_HEADER_KERNEL_TASK_RUN_ID)
        .ok_or_else(|| {
            bad_request(format!(
                "{HSK_HEADER_KERNEL_TASK_RUN_ID} header is required"
            ))
        })?
        .to_string();
    let session_run_id = header_str(headers, HSK_HEADER_SESSION_RUN_ID)
        .ok_or_else(|| bad_request(format!("{HSK_HEADER_SESSION_RUN_ID} header is required")))?
        .to_string();
    let actor = match header_str(headers, HSK_HEADER_ACTOR_KIND).unwrap_or("system") {
        "operator" => KernelActor::Operator(actor_id),
        "system" => KernelActor::System(actor_id),
        "session_broker" => KernelActor::SessionBroker(actor_id),
        "model_adapter" => KernelActor::ModelAdapter(actor_id),
        "toolgate" => KernelActor::ToolGate(actor_id),
        "validation_runner" => KernelActor::ValidationRunner(actor_id),
        "promotion_gate" => KernelActor::PromotionGate(actor_id),
        other => {
            return Err(bad_request(format!(
                "unknown {HSK_HEADER_ACTOR_KIND} '{other}'"
            )))
        }
    };
    Ok(IndexIdentity {
        actor,
        kernel_task_run_id,
        session_run_id,
        correlation_id: header_str(headers, HSK_HEADER_CORRELATION_ID).map(ToOwned::to_owned),
    })
}

/// Map an ingestion error to HTTP. A fail-closed allowlist denial is a 403 WITH
/// the durable decision id (never a silent skip).
fn ingestion_error(err: IngestionError) -> ApiError {
    match err {
        IngestionError::PolicyDenied {
            verdict,
            candidate_path,
            matched_pattern,
            decision_id,
        } => (
            StatusCode::FORBIDDEN,
            Json(json!({
                "error": "policy_denied",
                "verdict": verdict.as_str(),
                "candidate_path": candidate_path,
                "matched_pattern": matched_pattern,
                "decision_id": decision_id,
            })),
        ),
        IngestionError::Validation(detail) => bad_request(detail),
        IngestionError::Io { path, detail } => (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "io_error", "path": path, "detail": detail})),
        ),
        IngestionError::Storage(StorageError::NotFound(what)) => (
            StatusCode::NOT_FOUND,
            Json(json!({"error": "not_found", "detail": what})),
        ),
        other => {
            tracing::error!(
                target: "handshake_core::code_nav_index",
                error = %other,
                "code_nav_index_ingestion_error"
            );
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "internal_error"})),
            )
        }
    }
}

/// Map a code-index error to HTTP.
fn code_index_error(err: CodeIndexError) -> ApiError {
    match err {
        CodeIndexError::Validation(detail) => bad_request(detail),
        CodeIndexError::Storage(StorageError::NotFound(what)) => (
            StatusCode::NOT_FOUND,
            Json(json!({"error": "not_found", "detail": what})),
        ),
        other => {
            tracing::error!(
                target: "handshake_core::code_nav_index",
                error = %other,
                "code_nav_index_code_index_error"
            );
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "internal_error"})),
            )
        }
    }
}

// ---------------------------------------------------------------------------
// Handler.
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
struct IndexBody {
    /// The machine-local directory to index (runtime input for THIS run only —
    /// never stored authority; the registered root path is repo-relative empty
    /// so the anchor IS the root dir).
    root_path: String,
}

/// `POST /workspaces/:ws/code-nav/index` — walk `root_path`, ingest each
/// eligible file as a knowledge source, code-index each code/config source, and
/// return the total symbol count.
async fn index_workspace_code(
    State(state): State<AppState>,
    Path(workspace_id): Path<String>,
    headers: HeaderMap,
    Json(body): Json<IndexBody>,
) -> Result<Json<Value>, ApiError> {
    let identity = index_identity(&headers)?;
    let root_path = body.root_path.trim();
    if root_path.is_empty() {
        return Err(bad_request("root_path is required"));
    }
    let anchor = PathBuf::from(root_path);

    // One shared pooled handle backs both engines (no reconnect).
    let db = Arc::new(PostgresDatabase::new(state.postgres_pool.clone()));
    let ingestion = IngestionEngine::from_database(db.clone());
    let code_index = Arc::new(CodeIndexEngine::from_database(db));

    let ingest_ctx = identity.ingestion_context();
    let code_ctx = identity.code_index_context();

    // 1) Register the anchor as a project-repo root. `repo_relative_path = ""`
    //    means "the anchor itself is the root dir"; the MT-081 root allowlist is
    //    evaluated fail-closed here (a denied path returns the typed 403).
    let (root, _decision) = ingestion
        .register_root(
            &ingest_ctx,
            RootRegistrationRequest {
                workspace_id: workspace_id.clone(),
                display_name: format!("code-nav-index {root_path}"),
                root_kind: KnowledgeRootKind::ProjectRepo,
                repo_relative_path: String::new(),
                // Include-everything FILE allowlist; the walker still skips
                // .git/node_modules/target and non-code kinds are ignored below.
                file_allowlist_policy: json!({"include": ["**/*"], "exclude": []}),
                operator_approved: true,
            },
        )
        .await
        .map_err(ingestion_error)?;

    // 2) Run the REAL ingestion pass over the anchor — the shared walker +
    //    per-root file allowlist. Every eligible file becomes a KnowledgeSource.
    let (prepared, files_skipped) = ingestion
        .prepare_code_nav_files(&root, &anchor, &IngestionLimits::default())
        .await
        .map_err(ingestion_error)?;
    let files_ingested = prepared.len();
    let ingestion_run_token = format!("KIRUN-{}", uuid::Uuid::now_v7().simple());
    let persisted_sources = ingestion
        .persist_code_nav_batch(&ingest_ctx, &root, &ingestion_run_token, &prepared)
        .await
        .map_err(ingestion_error)?;

    // 3) Start a code-index run and index each ingested CODE/CONFIG source.
    let index_run_id = code_index
        .start_run(&code_ctx, &workspace_id, Some(root.root_id.as_str()))
        .await
        .map_err(code_index_error)?;

    // The ingestion pass remains ordered so source lifecycle and stale-source
    // detection retain their canonical semantics. Code indexing is independent
    // per source, however, and each operation uses the shared PostgreSQL pool.
    // Keep a small bounded fan-out so the route makes progress on the managed
    // pool without opening an unbounded task/connection storm.
    let index_inputs: Vec<(String, String)> = persisted_sources
        .clone()
        .into_iter()
        .filter_map(|(source_id, relative_path)| {
            if detect_code_language(&relative_path).is_none()
                && detect_config_format(&relative_path).is_none()
            {
                return None;
            }
            Some((source_id, relative_path))
        })
        .collect();
    let batch_attempt = code_index
        .try_index_prepared_batch(
            &code_ctx,
            &workspace_id,
            &prepared,
            &persisted_sources,
            &index_run_id,
        )
        .await;
    let indexed_results = match batch_attempt {
        Err(error) => {
            let mapped = code_index_error(error);
            if let Err(finish_error) = code_index
                .finish_run_with_retry(
                    &code_ctx,
                    &index_run_id,
                    KnowledgeIndexRunOutcome::Failed {
                        counts: KnowledgeIndexRunCounts::default(),
                        error_capture: json!({
                            "kind": "code_index_batch_failure",
                            "error": format!("{mapped:?}"),
                        }),
                    },
                )
                .await
            {
                tracing::error!(
                    index_run_id = %index_run_id,
                    error = ?finish_error,
                    "code_nav_index_batch_failure_terminalization_failed"
                );
            }
            return Err(mapped);
        }
        Ok(Some(outcomes)) => outcomes.into_iter().map(Ok).collect::<Vec<_>>(),
        Ok(None) => {
            let index_engine = code_index.clone();
            stream::iter(index_inputs)
                .map(|(source_id, relative_path)| {
                    let index_engine = index_engine.clone();
                    let code_ctx = code_ctx.clone();
                    let workspace_id = workspace_id.clone();
                    let anchor = anchor.clone();
                    let index_run_id = index_run_id.clone();
                    async move {
                        // `read_and_index` re-reads the file under the anchor and runs the
                        // AST/config indexer. A per-file read/parse failure is captured as
                        // a typed failed outcome and does not abort sibling work.
                        read_and_index(
                            &index_engine,
                            &code_ctx,
                            &workspace_id,
                            &source_id,
                            &relative_path,
                            &anchor,
                            Some(index_run_id.as_str()),
                        )
                        .await
                        .map_err(code_index_error)
                    }
                })
                .buffer_unordered(configured_postgres_parallelism())
                .collect::<Vec<_>>()
                .await
        }
    };

    // Drain every in-flight file even if one operation failed. This avoids
    // fail-fast cancellation leaving sibling writes half-completed while the
    // index-run row is still `started`.
    let mut indexed_outcomes = Vec::with_capacity(indexed_results.len());
    let mut first_error = None;
    let mut indexing_error_count = 0usize;
    for result in indexed_results {
        match result {
            Ok(outcome) => indexed_outcomes.push(outcome),
            Err(error) => {
                indexing_error_count += 1;
                first_error.get_or_insert(error);
            }
        }
    }

    let files_indexed = indexed_outcomes
        .iter()
        .filter(|outcome| !outcome.failed)
        .count();
    let files_failed = indexed_outcomes
        .iter()
        .filter(|outcome| outcome.failed)
        .count();
    let symbol_count = indexed_outcomes
        .iter()
        .map(|outcome| outcome.symbols_indexed)
        .sum::<usize>();

    let counts = KnowledgeIndexRunCounts {
        sources_seen: files_ingested as i32,
        sources_indexed: files_indexed as i32,
        spans_extracted: indexed_outcomes
            .iter()
            .map(|outcome| outcome.symbols_indexed + outcome.doc_passages_indexed)
            .sum::<usize>() as i32,
        entities_detected: indexed_outcomes
            .iter()
            .map(|outcome| outcome.symbols_indexed + outcome.doc_passages_indexed)
            .sum::<usize>() as i32,
        edges_written: indexed_outcomes
            .iter()
            .map(|outcome| outcome.edges_indexed)
            .sum::<usize>() as i32,
        claims_written: 0,
    };
    if let Some(error) = first_error {
        if let Err(finish_error) = code_index
            .finish_run_with_retry(
                &code_ctx,
                &index_run_id,
                KnowledgeIndexRunOutcome::Failed {
                    counts,
                    error_capture: json!({
                        "kind": "code_index_file_failure",
                        "files_completed": files_indexed,
                        "files_failed": files_failed + indexing_error_count,
                    }),
                },
            )
            .await
        {
            return Err(code_index_error(finish_error));
        }
        return Err(error);
    }
    code_index
        .finish_run_with_retry(
            &code_ctx,
            &index_run_id,
            KnowledgeIndexRunOutcome::Completed { counts },
        )
        .await
        .map_err(code_index_error)?;

    Ok(Json(json!({
        "symbol_count": symbol_count,
        "files_indexed": files_indexed,
        "files_failed": files_failed,
        "files_skipped": files_skipped,
        "files_ingested": files_ingested,
        "skipped_by_allowlist": files_skipped,
        "root_id": root.root_id,
        "index_run_id": index_run_id,
    })))
}
