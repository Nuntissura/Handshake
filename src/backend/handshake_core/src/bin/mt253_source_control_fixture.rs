// WP-KERNEL-009 / MT-253 — source-control panel real-backend fixture.
//
// Spins up a REAL temp git repository plus the real source-control REST surface
// (the embedded-Surreal KernelSourceControlEventRecorder, via `api::routes`)
// against an isolated store. The offline Playwright spec drives the built
// SourceControlPanel harness against this backend: status -> diff (Monaco) ->
// stage -> commit -> branch -> log -> blame, with EventLedger receipts written
// to the real SurrealDB EventLedger. A proof endpoint reads BOTH the real `git log` and the
// appended kernel events back so the spec can assert the commit truly landed.

use std::{net::SocketAddr, path::Path, process::Command, sync::Arc};

use axum::{
    extract::State,
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use handshake_core::{
    api,
    capabilities::CapabilityRegistry,
    flight_recorder::duckdb::DuckDbFlightRecorder,
    llm::DisabledLlmClient,
    source_control::SourceControlRepository,
    storage::{
        surreal::SurrealDatabase,
        tests::{embedded_test_backend, EmbeddedTestBackend},
        Database,
    },
    workflows::{SessionRegistry, SessionSchedulerConfig},
    AppState,
};
use serde::Serialize;
use serde_json::{json, Value};
use tokio::{net::TcpListener, sync::watch};
use tower_http::cors::{Any, CorsLayer};
use uuid::Uuid;

#[derive(Debug, Serialize)]
struct ReadyMessage {
    base_url: String,
    repo_path: String,
    repo_root_id: String,
    tracked_path: String,
    untracked_path: String,
}

#[derive(Debug, Serialize)]
struct ProofResponse {
    head_commit_id: String,
    head_commit_message: String,
    log_messages: Vec<String>,
    branches: Vec<String>,
    receipt_event_ids: Vec<String>,
    receipt_operations: Vec<String>,
    commit_receipt_payloads: Vec<Value>,
}

struct FixtureState {
    app: AppState,
    repo_path: String,
    repo_root_id: String,
}

type SharedFixture = Arc<FixtureState>;

#[tokio::main]
async fn main() {
    if let Err(error) = run().await {
        eprintln!("MT253_FIXTURE_ERROR {error}");
        std::process::exit(1);
    }
}

async fn run() -> Result<(), Box<dyn std::error::Error>> {
    if git_version().is_none() {
        println!("MT253_FIXTURE_SKIP git CLI not found");
        return Ok(());
    }

    let backend = embedded_test_backend().await?;
    let repo_dir_path =
        std::env::temp_dir().join(format!("mt253-source-control-{}", Uuid::now_v7().simple()));
    let body_result = run_server(&backend, &repo_dir_path).await;
    let store_cleanup_result = backend.close_and_remove().await;
    let repo_cleanup_result = remove_temp_git_repo(&repo_dir_path);
    let cleanup_result: Result<(), Box<dyn std::error::Error>> =
        match (store_cleanup_result, repo_cleanup_result) {
            (Ok(()), Ok(())) => Ok(()),
            (Err(store_error), Ok(())) => Err(Box::new(store_error)),
            (Ok(()), Err(repo_error)) => Err(Box::new(repo_error)),
            (Err(store_error), Err(repo_error)) => Err(std::io::Error::other(format!(
                "embedded-store cleanup failed: {store_error}; temp-git-repo cleanup also failed: {repo_error}"
            ))
            .into()),
        };

    match (body_result, cleanup_result) {
        (Ok(()), Ok(())) => Ok(()),
        (Err(body_error), Ok(())) => Err(body_error),
        (Ok(()), Err(cleanup_error)) => Err(cleanup_error),
        (Err(body_error), Err(cleanup_error)) => Err(std::io::Error::other(format!(
            "fixture body failed: {body_error}; fixture cleanup also failed: {cleanup_error}"
        ))
        .into()),
    }
}

async fn run_server(
    backend: &EmbeddedTestBackend,
    repo_dir_path: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    // Real temp git repo, seeded with one committed file and pending changes.
    // Created under the OS temp root (bins cannot use the dev-only tempfile crate).
    std::fs::create_dir_all(&repo_dir_path)?;
    seed_git_repo(&repo_dir_path)?;
    let repo_path = repo_dir_path.to_string_lossy().to_string();
    // Derive repo_root_id exactly as the source-control API does: open through
    // SourceControlRepository (git rev-parse --show-toplevel) then forward-slash.
    let repo = SourceControlRepository::open(&repo_path)
        .map_err(|err| format!("open temp repo failed: {err}"))?;
    let repo_root_id = repo.root().to_string_lossy().replace('\\', "/");

    let app = app_state_for(backend)?;
    let fixture: SharedFixture = Arc::new(FixtureState {
        app: app.clone(),
        repo_path: repo_path.clone(),
        repo_root_id: repo_root_id.clone(),
    });

    let (shutdown_tx, mut shutdown_rx) = watch::channel(false);
    let router = app_router(app, fixture).route(
        "/__fixture/shutdown",
        post(move || {
            let shutdown_tx = shutdown_tx.clone();
            async move {
                let _ = shutdown_tx.send(true);
            }
        }),
    );
    let listener = TcpListener::bind(SocketAddr::from(([127, 0, 0, 1], 0))).await?;
    let addr = listener.local_addr()?;

    let ready = ReadyMessage {
        base_url: format!("http://{addr}"),
        repo_path,
        repo_root_id,
        tracked_path: "tracked.txt".to_string(),
        untracked_path: "new.txt".to_string(),
    };
    println!("MT253_FIXTURE_READY {}", serde_json::to_string(&ready)?);

    axum::serve(listener, router)
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

fn remove_temp_git_repo(path: &Path) -> Result<(), std::io::Error> {
    match std::fs::remove_dir_all(path) {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(error),
    }
    if path.exists() {
        return Err(std::io::Error::other(format!(
            "temp git repo still exists after teardown: {}",
            path.display()
        )));
    }
    Ok(())
}

fn git_version() -> Option<String> {
    let output = Command::new("git").arg("--version").output().ok()?;
    output
        .status
        .success()
        .then(|| String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn seed_git_repo(root: &Path) -> Result<(), Box<dyn std::error::Error>> {
    run_git(root, &["init", "-b", "main"])?;
    run_git(root, &["config", "user.name", "Handshake MT253"])?;
    run_git(root, &["config", "user.email", "mt253@handshake.invalid"])?;
    run_git(root, &["config", "core.autocrlf", "false"])?;
    std::fs::write(root.join("tracked.txt"), "fn main() {\n    init();\n}\n")?;
    run_git(root, &["add", "tracked.txt"])?;
    run_git(root, &["commit", "-m", "seed: initial tracked file"])?;
    // Pending changes the panel will diff/stage/commit.
    std::fs::write(
        root.join("tracked.txt"),
        "fn main() {\n    init();\n    run();\n}\n",
    )?;
    std::fs::write(root.join("new.txt"), "untracked content\n")?;
    Ok(())
}

fn run_git(root: &Path, args: &[&str]) -> Result<String, Box<dyn std::error::Error>> {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["-c", "core.longpaths=true"])
        .args(args)
        .output()?;
    if !output.status.success() {
        return Err(format!(
            "git {:?} failed: {}",
            args,
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

fn app_state_for(backend: &EmbeddedTestBackend) -> Result<AppState, Box<dyn std::error::Error>> {
    let recorder = Arc::new(DuckDbFlightRecorder::new_in_memory(7)?);
    Ok(AppState {
        storage: backend.database.clone(),
        surreal: backend.storage.clone(),
        flight_recorder: recorder.clone(),
        diagnostics: recorder,
        llm_client: Arc::new(DisabledLlmClient::new(
            "mt253-source-control-fixture".to_string(),
            "fixture does not call an LLM".to_string(),
        )),
        capability_registry: Arc::new(CapabilityRegistry::new()),
        session_registry: Arc::new(SessionRegistry::new(SessionSchedulerConfig::default())),
    })
}

fn app_router(state: AppState, fixture: SharedFixture) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);
    // Real product source-control routes (SurrealDB-backed recorder) come from api::routes.
    let api_routes = api::routes(state.clone());
    Router::new()
        .route("/health", get(|| async { Json(json!({"status": "ok"})) }))
        .route("/mt253-fixture/proof", get(fixture_proof))
        .with_state(fixture)
        .merge(api_routes.clone())
        .nest("/api", api_routes)
        .layer(cors)
}

async fn fixture_proof(
    State(fixture): State<SharedFixture>,
) -> Result<Json<ProofResponse>, (StatusCode, String)> {
    let repo = Path::new(&fixture.repo_path);

    let head_commit_id = run_git(repo, &["rev-parse", "HEAD"])
        .map_err(internal)?
        .trim()
        .to_string();
    let head_commit_message = run_git(repo, &["log", "-1", "--pretty=%s"])
        .map_err(internal)?
        .trim()
        .to_string();
    let log_messages = run_git(repo, &["log", "--pretty=%s"])
        .map_err(internal)?
        .lines()
        .map(|line| line.trim().to_string())
        .filter(|line| !line.is_empty())
        .collect();
    let branches = run_git(repo, &["branch", "--format=%(refname:short)"])
        .map_err(internal)?
        .lines()
        .map(|line| line.trim().to_string())
        .filter(|line| !line.is_empty())
        .collect();

    let db = SurrealDatabase::new(fixture.app.surreal.clone());
    let events = db
        .list_kernel_events_for_aggregate("source_control_repo", &fixture.repo_root_id)
        .await
        .map_err(internal)?;
    let receipt_event_ids = events.iter().map(|e| e.event_id.clone()).collect();
    let receipt_operations = events
        .iter()
        .filter_map(|e| {
            e.payload
                .get("operation")
                .and_then(Value::as_str)
                .map(str::to_string)
        })
        .collect();
    let commit_receipt_payloads = events
        .iter()
        .filter(|e| e.payload.get("operation").and_then(Value::as_str) == Some("commit"))
        .map(|e| e.payload.clone())
        .collect();

    Ok(Json(ProofResponse {
        head_commit_id,
        head_commit_message,
        log_messages,
        branches,
        receipt_event_ids,
        receipt_operations,
        commit_receipt_payloads,
    }))
}

fn internal(error: impl std::fmt::Display) -> (StatusCode, String) {
    (StatusCode::INTERNAL_SERVER_ERROR, error.to_string())
}
