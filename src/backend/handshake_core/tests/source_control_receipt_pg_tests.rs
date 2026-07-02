// WP-KERNEL-009 / MT-253 — real-PostgreSQL EventLedger receipt proof.
//
// Drives source-control write ops through the REAL PG-backed recorder
// (`source_control::kernel_event_recorder`, the same one `routes()` uses) against
// an isolated PostgreSQL schema, then reads the appended kernel events back from
// the EventLedger by aggregate. This closes the gap where the earlier API test
// used a recording test double instead of real durable state.

use std::{net::SocketAddr, path::Path, process::Command};

use axum::Router;
use handshake_core::api::source_control as source_control_api;
use handshake_core::source_control::SourceControlRepository;
use handshake_core::storage::{tests::postgres_backend_from_env, Database, StorageError};
use serde_json::{json, Value};
use std::sync::Arc;
use tempfile::tempdir;
use tokio::net::TcpListener;

async fn postgres_or_blocked() -> Arc<dyn Database> {
    match postgres_backend_from_env().await {
        Ok(db) => db,
        Err(err) => panic!("failed to init postgres backend: {err:?}"),
    }
}

#[tokio::test]
#[ignore = "requires real PostgreSQL; auto-resolves POSTGRES_TEST_URL > DATABASE_URL > managed PostgreSQL; run with `cargo test -- --ignored`"]
async fn source_control_stage_and_commit_append_real_event_ledger_receipts() {
    let db = postgres_or_blocked().await;

    // Real temp git repo with a committed file plus a pending modification.
    let repo_dir = tempdir().expect("temp git repo");
    init_repo(repo_dir.path());
    write(repo_dir.path(), "tracked.txt", "alpha\n");
    git(repo_dir.path(), &["add", "tracked.txt"]);
    git(repo_dir.path(), &["commit", "-m", "seed commit"]);
    write(repo_dir.path(), "tracked.txt", "alpha\nbeta\n");

    let repo = SourceControlRepository::open(repo_dir.path()).expect("open repo");
    let repo_root_id = repo.root().to_string_lossy().replace('\\', "/");

    // The SAME PG-backed recorder the product wires in routes().
    let recorder = source_control_api::kernel_event_recorder(db.clone());
    let (base, _server) =
        start_server(source_control_api::routes_with_event_recorder(recorder)).await;
    let http = reqwest::Client::new();
    let repo_path = repo_dir.path().to_string_lossy();

    let stage: Value = http
        .post(format!("{base}/source-control/stage"))
        .json(&json!({"repo_path": repo_path.as_ref(), "paths": ["tracked.txt"]}))
        .send()
        .await
        .expect("stage request")
        .error_for_status()
        .expect("stage response")
        .json()
        .await
        .expect("stage json");
    let stage_receipt_id = stage["event_ledger_event_id"]
        .as_str()
        .expect("stage receipt id")
        .to_string();
    assert!(
        stage_receipt_id.starts_with("KE-"),
        "got {stage_receipt_id}"
    );

    let commit: Value = http
        .post(format!("{base}/source-control/commit"))
        .json(&json!({"repo_path": repo_path.as_ref(), "message": "mt253 receipt commit"}))
        .send()
        .await
        .expect("commit request")
        .error_for_status()
        .expect("commit response")
        .json()
        .await
        .expect("commit json");
    let commit_receipt_id = commit["event_ledger_event_id"]
        .as_str()
        .expect("commit receipt id")
        .to_string();
    let commit_sha = commit["id"].as_str().expect("commit sha").to_string();
    assert_eq!(commit_sha.len(), 40);

    // Read the appended kernel events back from REAL PostgreSQL by aggregate.
    let events = db
        .list_kernel_events_for_aggregate("source_control_repo", &repo_root_id)
        .await
        .expect("list source-control kernel events");

    let operations: Vec<String> = events
        .iter()
        .filter_map(|event| {
            event
                .payload
                .get("operation")
                .and_then(Value::as_str)
                .map(str::to_string)
        })
        .collect();
    assert_eq!(operations, ["stage", "commit"]);

    // The receipt ids returned to the API caller must match the durable events.
    let durable_ids: Vec<String> = events.iter().map(|event| event.event_id.clone()).collect();
    assert!(
        durable_ids.contains(&stage_receipt_id),
        "stage receipt {stage_receipt_id} not durable in EventLedger: {durable_ids:?}"
    );
    assert!(
        durable_ids.contains(&commit_receipt_id),
        "commit receipt {commit_receipt_id} not durable in EventLedger: {durable_ids:?}"
    );

    let commit_event = events
        .iter()
        .find(|event| event.event_id == commit_receipt_id)
        .expect("commit event present");
    assert_eq!(
        commit_event.event_type.as_str(),
        "SOURCE_CONTROL_OPERATION_RECORDED"
    );
    assert_eq!(commit_event.payload["operation"], "commit");
    assert_eq!(
        commit_event.payload["commit_message"],
        "mt253 receipt commit"
    );
    assert_eq!(commit_event.payload["phase"], "pre_git_write");
    assert_eq!(
        commit_event.payload["authority_source"],
        "postgres_event_ledger"
    );

    // The commit truly landed in real git log with the correct message.
    let head_message = run_git_stdout(repo_dir.path(), &["log", "-1", "--pretty=%s"]);
    assert_eq!(head_message.trim(), "mt253 receipt commit");
    let head_sha = run_git_stdout(repo_dir.path(), &["rev-parse", "HEAD"]);
    assert_eq!(head_sha.trim(), commit_sha);
}

async fn start_server(app: Router) -> (String, tokio::task::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind test server");
    let addr: SocketAddr = listener.local_addr().expect("server addr");
    let server = tokio::spawn(async move {
        axum::serve(listener, app)
            .await
            .expect("serve source control api");
    });
    (format!("http://{addr}"), server)
}

fn init_repo(path: &Path) {
    git(path, &["init", "-b", "main"]);
    git(path, &["config", "user.name", "Handshake MT253"]);
    git(path, &["config", "user.email", "mt253@handshake.invalid"]);
    git(path, &["config", "core.autocrlf", "false"]);
}

fn write(root: &Path, relative: &str, contents: &str) {
    let path = root.join(relative);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("create parent");
    }
    std::fs::write(path, contents).expect("write fixture");
}

fn git(path: &Path, args: &[&str]) {
    let output = Command::new("git")
        .arg("-C")
        .arg(path)
        .args(["-c", "core.longpaths=true"])
        .args(args)
        .output()
        .expect("run git");
    assert!(
        output.status.success(),
        "git {:?} failed\nstdout:\n{}\nstderr:\n{}",
        args,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn run_git_stdout(path: &Path, args: &[&str]) -> String {
    let output = Command::new("git")
        .arg("-C")
        .arg(path)
        .args(["-c", "core.longpaths=true"])
        .args(args)
        .output()
        .expect("run git");
    assert!(output.status.success(), "git {args:?} failed");
    String::from_utf8_lossy(&output.stdout).to_string()
}
