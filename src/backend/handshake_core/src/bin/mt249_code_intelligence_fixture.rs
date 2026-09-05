use std::{net::SocketAddr, sync::Arc};

use axum::{
    routing::{get, post},
    Json, Router,
};
use handshake_core::{
    api,
    capabilities::CapabilityRegistry,
    flight_recorder::duckdb::DuckDbFlightRecorder,
    kernel::KernelActor,
    knowledge_code_index::{
        engine::{CodeIndexContext, CodeIndexEngine},
        parser::{CodeLanguage, CodeParserAdapter},
    },
    llm::{CompletionRequest, CompletionResponse, LlmClient, LlmError, ModelProfile, TokenUsage},
    storage::{
        knowledge::{
            KnowledgeIndexingEligibility, KnowledgeRootKind, KnowledgeStore, NewKnowledgeSourceRoot,
        },
        surreal::SurrealDatabase,
        tests::{embedded_test_backend, EmbeddedTestBackend},
        Database, NewWorkspace, WriteContext,
    },
    workflows::{SessionRegistry, SessionSchedulerConfig},
    AppState,
};
use serde::Serialize;
use serde_json::json;
use tokio::{net::TcpListener, sync::watch};
use tower_http::cors::{Any, CorsLayer};
use uuid::Uuid;

const RUST_SRC: &str = r#"
/// Adds two numbers.
pub fn add(a: i32, b: i32) -> i32 { a + b }

pub fn caller() -> i32 { add(1, 2) }

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn adds() { assert_eq!(add(1, 2), 3); }
}
"#;

#[derive(Serialize)]
struct ReadyMessage {
    base_url: String,
    workspace_id: String,
    symbol_entity_id: String,
    content_hash: String,
    parser_version: String,
}

struct NoopLlmClient {
    profile: ModelProfile,
}

#[async_trait::async_trait]
impl LlmClient for NoopLlmClient {
    async fn completion(&self, _req: CompletionRequest) -> Result<CompletionResponse, LlmError> {
        Ok(CompletionResponse {
            text: String::new(),
            usage: TokenUsage {
                prompt_tokens: 0,
                completion_tokens: 0,
                total_tokens: 0,
            },
            latency_ms: 0,
        })
    }

    fn profile(&self) -> &ModelProfile {
        &self.profile
    }
}

#[tokio::main]
async fn main() {
    if let Err(error) = run().await {
        eprintln!("MT249_FIXTURE_ERROR {error}");
        std::process::exit(1);
    }
}

async fn run() -> Result<(), Box<dyn std::error::Error>> {
    let backend = embedded_test_backend().await?;
    let body_result = run_server(&backend).await;
    let cleanup_result = backend.close_and_remove().await;

    match (body_result, cleanup_result) {
        (Ok(()), Ok(())) => Ok(()),
        (Err(body_error), Ok(())) => Err(body_error),
        (Ok(()), Err(cleanup_error)) => Err(Box::new(cleanup_error)),
        (Err(body_error), Err(cleanup_error)) => Err(std::io::Error::other(format!(
            "fixture body failed: {body_error}; embedded-store cleanup also failed: {cleanup_error}"
        ))
        .into()),
    }
}

async fn run_server(backend: &EmbeddedTestBackend) -> Result<(), Box<dyn std::error::Error>> {
    let db = Arc::new(SurrealDatabase::new(backend.storage.clone()));

    let workspace_id = seed_code_fixture(db.clone()).await?;
    let symbol = db
        .lookup_code_symbols(&workspace_id, Some("add"), None, None, 5)
        .await?
        .into_iter()
        .find(|entity| entity.entity_key == "rust:src/lib.rs#add")
        .ok_or("seeded add symbol missing")?;
    let code_files = db.list_knowledge_code_files(&workspace_id).await?;
    let code_file = code_files.first().ok_or("seeded code file missing")?;
    db.mark_knowledge_code_file_stale(&code_file.code_file_id)
        .await?;

    let state = app_state_for(backend)?;
    let (shutdown_tx, mut shutdown_rx) = watch::channel(false);
    let app = app_router(state).route(
        "/__fixture/shutdown",
        post(move || {
            let shutdown_tx = shutdown_tx.clone();
            async move {
                let _ = shutdown_tx.send(true);
            }
        }),
    );
    let port = std::env::var("MT249_FIXTURE_PORT")
        .ok()
        .and_then(|value| value.parse::<u16>().ok())
        .unwrap_or(0);
    let listener = TcpListener::bind(SocketAddr::from(([127, 0, 0, 1], port))).await?;
    let addr = listener.local_addr()?;
    let ready = ReadyMessage {
        base_url: format!("http://{addr}"),
        workspace_id,
        symbol_entity_id: symbol.entity_id,
        content_hash: sha256_hex(RUST_SRC.as_bytes()),
        parser_version: CodeParserAdapter::new(CodeLanguage::Rust).parser_version(),
    };
    println!("MT249_FIXTURE_READY {}", serde_json::to_string(&ready)?);

    axum::serve(listener, app)
        .with_graceful_shutdown(async move {
            loop {
                if *shutdown_rx.borrow_and_update() {
                    break;
                }
                if shutdown_rx.changed().await.is_err() {
                    break;
                }
            }
        })
        .await?;
    Ok(())
}

async fn seed_code_fixture(db: Arc<SurrealDatabase>) -> Result<String, Box<dyn std::error::Error>> {
    let workspace = db
        .create_workspace(
            &WriteContext::human(None),
            NewWorkspace {
                name: format!("mt249-code-intel-{}", Uuid::now_v7()),
            },
        )
        .await?;
    let engine = CodeIndexEngine::new(db.clone());
    let context = CodeIndexContext {
        actor: KernelActor::System("mt249-code-intelligence-fixture".to_string()),
        kernel_task_run_id: "KTR-MT249-fixture".to_string(),
        session_run_id: "SR-MT249-fixture".to_string(),
        correlation_id: Some("CORR-MT249-fixture".to_string()),
    };
    let root = db
        .create_knowledge_source_root(NewKnowledgeSourceRoot {
            workspace_id: workspace.id.clone(),
            display_name: "mt249-code-fixture".to_string(),
            root_kind: KnowledgeRootKind::ProjectRepo,
            repo_relative_path: format!("mt249/{}", Uuid::now_v7().simple()),
            allowlist_policy: json!({"include": ["**/*"], "exclude": []}),
            indexing_eligibility: KnowledgeIndexingEligibility::Eligible,
        })
        .await?
        .root_id;
    let source_id = engine
        .register_code_source(&workspace.id, Some(&root), "src/lib.rs", RUST_SRC)
        .await?;
    engine
        .index_code_source(
            &context,
            &workspace.id,
            &source_id,
            "src/lib.rs",
            RUST_SRC,
            None,
        )
        .await?;
    Ok(workspace.id)
}

fn app_state_for(backend: &EmbeddedTestBackend) -> Result<AppState, Box<dyn std::error::Error>> {
    let recorder = Arc::new(DuckDbFlightRecorder::new_in_memory(7)?);
    Ok(AppState {
        storage: backend.database.clone(),
        surreal: backend.storage.clone(),
        flight_recorder: recorder.clone(),
        diagnostics: recorder,
        llm_client: Arc::new(NoopLlmClient {
            profile: ModelProfile::new("mt249-code-intelligence-fixture".to_string(), 4096),
        }),
        capability_registry: Arc::new(CapabilityRegistry::new()),
        session_registry: Arc::new(SessionRegistry::new(SessionSchedulerConfig::default())),
    })
}

fn app_router(state: AppState) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);
    let api_routes = api::routes(state.clone());
    Router::new()
        .route("/health", get(|| async { Json(json!({"status": "ok"})) }))
        .merge(api_routes.clone())
        .nest("/api", api_routes)
        .layer(cors)
}

use crate::storage::artifacts::sha256_hex;
