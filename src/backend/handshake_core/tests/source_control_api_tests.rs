use std::{
    fs,
    net::SocketAddr,
    path::Path,
    process::Command,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, Mutex,
    },
};

use async_trait::async_trait;
use axum::Router;
use handshake_core::api::source_control as source_control_api;
use reqwest::StatusCode;
use serde_json::{json, Value};
use tempfile::tempdir;
use tokio::net::TcpListener;

#[tokio::test]
async fn source_control_api_drives_status_diff_stage_commit_log_and_blame_against_real_git() {
    let repo_dir = tempdir().expect("temp git repo");
    init_repo(repo_dir.path());
    write(repo_dir.path(), "tracked.txt", "initial\n");
    git(repo_dir.path(), &["add", "tracked.txt"]);
    git(repo_dir.path(), &["commit", "-m", "initial commit"]);
    write(repo_dir.path(), "tracked.txt", "initial\nchanged\n");
    write(repo_dir.path(), "new.txt", "new file\n");

    let recorder = Arc::new(RecordingEventRecorder::default());
    let (base, _server) = start_server(source_control_api::routes_with_event_recorder(
        recorder.clone(),
    ))
    .await;
    let http = reqwest::Client::new();
    let repo_path = repo_dir.path().to_string_lossy();

    let status: Value = http
        .get(format!("{base}/source-control/status"))
        .query(&[("repo_path", repo_path.as_ref())])
        .send()
        .await
        .expect("status request")
        .error_for_status()
        .expect("status response")
        .json()
        .await
        .expect("status json");
    assert_eq!(status["branch"], "main");
    assert!(status["entries"]
        .as_array()
        .unwrap()
        .iter()
        .any(|entry| { entry["path"] == "tracked.txt" && entry["worktree"] == "modified" }));

    let diff: Value = http
        .get(format!("{base}/source-control/diff"))
        .query(&[
            ("repo_path", repo_path.as_ref()),
            ("path", "tracked.txt"),
            ("scope", "worktree"),
        ])
        .send()
        .await
        .expect("diff request")
        .error_for_status()
        .expect("diff response")
        .json()
        .await
        .expect("diff json");
    assert!(diff["patch"].as_str().unwrap().contains("+changed"));

    let stage: Value = http
        .post(format!("{base}/source-control/stage"))
        .json(&json!({"repo_path": repo_path.as_ref(), "paths": ["tracked.txt", "new.txt"]}))
        .send()
        .await
        .expect("stage request")
        .error_for_status()
        .expect("stage response")
        .json()
        .await
        .expect("stage json");
    assert_eq!(stage["operation"], "stage");
    assert_eq!(stage["paths"].as_array().unwrap().len(), 2);
    assert_event_ledger_receipt(&stage);

    let commit: Value = http
        .post(format!("{base}/source-control/commit"))
        .json(&json!({"repo_path": repo_path.as_ref(), "message": "source control route commit"}))
        .send()
        .await
        .expect("commit request")
        .error_for_status()
        .expect("commit response")
        .json()
        .await
        .expect("commit json");
    assert_eq!(commit["message"], "source control route commit");
    assert_eq!(commit["id"].as_str().unwrap().len(), 40);
    assert_event_ledger_receipt(&commit);

    let create_branch: Value = http
        .post(format!("{base}/source-control/branches"))
        .json(&json!({"repo_path": repo_path.as_ref(), "name": "route-branch"}))
        .send()
        .await
        .expect("create branch request")
        .error_for_status()
        .expect("create branch response")
        .json()
        .await
        .expect("create branch json");
    assert_eq!(create_branch["operation"], "create_branch");
    assert_eq!(create_branch["paths"][0], "route-branch");
    assert_event_ledger_receipt(&create_branch);

    let switch_branch: Value = http
        .post(format!("{base}/source-control/switch"))
        .json(&json!({"repo_path": repo_path.as_ref(), "name": "route-branch"}))
        .send()
        .await
        .expect("switch branch request")
        .error_for_status()
        .expect("switch branch response")
        .json()
        .await
        .expect("switch branch json");
    assert_eq!(switch_branch["operation"], "switch_branch");
    assert_eq!(switch_branch["paths"][0], "route-branch");
    assert_event_ledger_receipt(&switch_branch);

    let log: Value = http
        .get(format!("{base}/source-control/log"))
        .query(&[("repo_path", repo_path.as_ref()), ("limit", "5")])
        .send()
        .await
        .expect("log request")
        .error_for_status()
        .expect("log response")
        .json()
        .await
        .expect("log json");
    assert_eq!(log["entries"][0]["message"], "source control route commit");

    let blame: Value = http
        .get(format!("{base}/source-control/blame"))
        .query(&[("repo_path", repo_path.as_ref()), ("path", "tracked.txt")])
        .send()
        .await
        .expect("blame request")
        .error_for_status()
        .expect("blame response")
        .json()
        .await
        .expect("blame json");
    assert!(blame["lines"]
        .as_array()
        .unwrap()
        .iter()
        .any(|line| { line["content"] == "changed" && line["commit_id"] == commit["id"] }));

    let operations: Vec<String> = recorder
        .records()
        .iter()
        .map(|record| record.operation.clone())
        .collect();
    assert_eq!(
        operations,
        ["stage", "commit", "create_branch", "switch_branch"]
    );
}

#[tokio::test]
async fn source_control_api_requires_discard_confirmation_and_rejects_bad_branch_names() {
    let repo_dir = tempdir().expect("temp git repo");
    init_repo(repo_dir.path());
    write(repo_dir.path(), "tracked.txt", "initial\n");
    git(repo_dir.path(), &["add", "tracked.txt"]);
    git(repo_dir.path(), &["commit", "-m", "initial commit"]);
    write(repo_dir.path(), "tracked.txt", "initial\nchanged\n");

    let recorder = Arc::new(RecordingEventRecorder::default());
    let (base, _server) = start_server(source_control_api::routes_with_event_recorder(
        recorder.clone(),
    ))
    .await;
    let http = reqwest::Client::new();
    let repo_path = repo_dir.path().to_string_lossy();

    let discard = http
        .post(format!("{base}/source-control/discard"))
        .json(
            &json!({"repo_path": repo_path.as_ref(), "paths": ["tracked.txt"], "confirmed": false}),
        )
        .send()
        .await
        .expect("discard request");
    assert_eq!(discard.status(), StatusCode::CONFLICT);
    assert_eq!(
        fs::read_to_string(repo_dir.path().join("tracked.txt")).expect("tracked content"),
        "initial\nchanged\n"
    );

    let branch = http
        .post(format!("{base}/source-control/branches"))
        .json(&json!({"repo_path": repo_path.as_ref(), "name": "@{-1}"}))
        .send()
        .await
        .expect("branch request");
    assert_eq!(branch.status(), StatusCode::BAD_REQUEST);
    assert!(
        recorder.records().is_empty(),
        "rejected source-control writes must not create EventLedger receipts"
    );
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
    git(path, &["config", "user.name", "Handshake Test"]);
    git(path, &["config", "user.email", "handshake@example.invalid"]);
    git(path, &["config", "core.autocrlf", "false"]);
}

fn write(root: &Path, relative: &str, contents: &str) {
    let path = root.join(relative);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("create parent");
    }
    fs::write(path, contents).expect("write fixture");
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

fn assert_event_ledger_receipt(response: &Value) {
    let event_id = response["event_ledger_event_id"]
        .as_str()
        .expect("write response includes EventLedger receipt id");
    assert!(
        event_id.starts_with("KE-"),
        "expected kernel EventLedger id, got {event_id}"
    );
}

#[derive(Default)]
struct RecordingEventRecorder {
    next_id: AtomicUsize,
    records: Mutex<Vec<source_control_api::SourceControlEventRecord>>,
}

impl RecordingEventRecorder {
    fn records(&self) -> Vec<source_control_api::SourceControlEventRecord> {
        self.records.lock().expect("records lock").clone()
    }
}

#[async_trait]
impl source_control_api::SourceControlEventRecorder for RecordingEventRecorder {
    async fn record(
        &self,
        record: source_control_api::SourceControlEventRecord,
    ) -> Result<String, source_control_api::SourceControlReceiptError> {
        self.records.lock().expect("records lock").push(record);
        let id = self.next_id.fetch_add(1, Ordering::SeqCst);
        Ok(format!("KE-source-control-api-test-{id}"))
    }
}
