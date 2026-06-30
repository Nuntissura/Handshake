//! WP-KERNEL-012 MT-101: native model-session launch from a workspace folder.
//!
//! The reachable native half is real `POST /jobs` job creation. Direct repo-folder-bound session spawn
//! with wrapper remains Tauri IPC-only (`kernel_swarm_spawn_session`), so the UI must surface an honest
//! `EndpointMissing` blocker and never fabricate a running model session.

use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::time::Duration;

use egui_kittest::{
    kittest::{NodeT, Queryable},
    Harness,
};
use handshake_native::app::{
    HandshakeApp, HealthDisplayState, ModelSessionLaunchDialogState,
    MODEL_SESSION_LAUNCH_DIALOG_AUTHOR_ID, MODEL_SESSION_LAUNCH_FOLDER_AUTHOR_ID,
    MODEL_SESSION_LAUNCH_INLINE_STATUS_AUTHOR_ID, MODEL_SESSION_LAUNCH_MODEL_AUTHOR_ID,
    MODEL_SESSION_LAUNCH_PROVIDER_AUTHOR_ID, MODEL_SESSION_LAUNCH_PROVIDER_CLOUD_AUTHOR_ID,
    MODEL_SESSION_LAUNCH_PROVIDER_LOCAL_AUTHOR_ID, MODEL_SESSION_LAUNCH_START_AUTHOR_ID,
    MODEL_SESSION_LAUNCH_STATUS_AUTHOR_ID, MODEL_SESSION_LAUNCH_WRAPPER_AUTHOR_ID,
};
use handshake_native::backend_client::{
    HealthInfo, HttpMethod, ModelSessionLaunchClient, ModelSessionLaunchError,
    ModelSessionLaunchRequest, ModelSessionProvider, MODEL_SESSION_DIRECT_SPAWN_PROBED_PATH,
    MODEL_SESSION_JOBS_PATH, MODEL_SESSION_LAUNCH_IPC_CHANNEL, MODEL_SESSION_LAUNCH_IPC_OWNER,
    MODEL_SESSION_PROTOCOL_ID,
};
use handshake_native::command_registry::{
    all_commands, effective_disabled, CommandKind, CMD_MODEL_SESSION_LAUNCH_WORKSPACE,
    MODEL_SESSION_LAUNCH_WORKSPACE_STABLE_ID,
};
use handshake_native::top_menu_bar::MENU_RUN_MODEL_SESSION_LAUNCH_AUTHOR_ID;

#[derive(Debug)]
struct CapturedRequest {
    request_line: String,
    body: serde_json::Value,
}

fn ok_app() -> HandshakeApp {
    HandshakeApp::with_health(HealthDisplayState::Ok(HealthInfo {
        status: "ok".to_owned(),
        db_status: "ok".to_owned(),
        migration_version: Some(1),
    }))
}

fn runtime() -> tokio::runtime::Runtime {
    tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .expect("test tokio runtime")
}

fn external_artifact_dir(subdir: &str) -> PathBuf {
    Path::new("../../../../Handshake_Artifacts/handshake-test").join(subdir)
}

static WGPU_SERIAL_GUARD: std::sync::Mutex<()> = std::sync::Mutex::new(());
fn wgpu_guard() -> std::sync::MutexGuard<'static, ()> {
    WGPU_SERIAL_GUARD.lock().unwrap_or_else(|p| p.into_inner())
}

fn live_author_nodes(harness: &Harness<'_, HandshakeApp>) -> Vec<(String, String, Option<String>)> {
    let mut found = Vec::new();
    for node in harness.root().children_recursive() {
        let ak = node.accesskit_node();
        if let Some(author_id) = ak.author_id() {
            found.push((author_id.to_owned(), format!("{:?}", ak.role()), ak.label()));
        }
    }
    found
}

fn capture_server(reply_body: &'static str) -> (String, std::thread::JoinHandle<CapturedRequest>) {
    capture_server_delayed(reply_body, Duration::ZERO)
}

fn capture_server_delayed(
    reply_body: &'static str,
    response_delay: Duration,
) -> (String, std::thread::JoinHandle<CapturedRequest>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind capture server");
    let base_url = format!("http://{}", listener.local_addr().expect("local addr"));
    let join = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept one request");
        stream
            .set_read_timeout(Some(Duration::from_secs(5)))
            .expect("stream read timeout");
        let mut buf = Vec::new();
        let mut chunk = [0_u8; 1024];
        loop {
            let n = stream.read(&mut chunk).expect("read request");
            if n == 0 {
                break;
            }
            buf.extend_from_slice(&chunk[..n]);
            if buf.windows(4).any(|w| w == b"\r\n\r\n") {
                let headers = String::from_utf8_lossy(&buf);
                let len = headers
                    .lines()
                    .find_map(|line| {
                        let (name, value) = line.split_once(':')?;
                        name.eq_ignore_ascii_case("content-length")
                            .then(|| value.trim().parse::<usize>().ok())
                            .flatten()
                    })
                    .unwrap_or(0);
                let header_end = buf
                    .windows(4)
                    .position(|w| w == b"\r\n\r\n")
                    .map(|idx| idx + 4)
                    .unwrap();
                while buf.len().saturating_sub(header_end) < len {
                    let n = stream.read(&mut chunk).expect("read body");
                    if n == 0 {
                        break;
                    }
                    buf.extend_from_slice(&chunk[..n]);
                }
                break;
            }
        }
        let raw = String::from_utf8(buf).expect("utf8 http request");
        let (head, body_raw) = raw.split_once("\r\n\r\n").expect("http split");
        let request_line = head.lines().next().unwrap_or_default().to_owned();
        let body = serde_json::from_str(body_raw).expect("json body");
        let response = format!(
            "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{}",
            reply_body.len(),
            reply_body
        );
        std::thread::sleep(response_delay);
        stream
            .write_all(response.as_bytes())
            .expect("write response");
        CapturedRequest { request_line, body }
    });
    (base_url, join)
}

#[test]
fn model_session_launch_dialog_renders_and_screenshots() {
    let _guard = wgpu_guard();
    let mut app = ok_app();
    app.set_model_session_launch_dialog_for_test(ModelSessionLaunchDialogState {
        provider: ModelSessionProvider::Local,
        workspace_folder: "D:/Projects/Handshake/repo".to_owned(),
        model_id: "qwen2.5-coder:7b".to_owned(),
        wrapper: "repo-folder-wrapper-v1".to_owned(),
    });

    let mut harness = Harness::builder()
        .with_size(egui::vec2(900.0, 700.0))
        .wgpu()
        .build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), app);
    harness.run();
    harness.run();
    let nodes = live_author_nodes(&harness);
    assert!(
        nodes.iter().any(
            |(author_id, role, _)| author_id == MODEL_SESSION_LAUNCH_DIALOG_AUTHOR_ID
                && role == "Dialog"
        ),
        "model-session launch dialog is live before screenshot: {nodes:?}"
    );
    let (_, _, inline_label) = nodes
        .iter()
        .find(|(author_id, _, _)| author_id == MODEL_SESSION_LAUNCH_INLINE_STATUS_AUTHOR_ID)
        .expect("inline status node is live");
    assert_eq!(
        inline_label.as_deref(),
        Some("Model session: ready to issue POST /jobs"),
        "visible ready status and AccessKit status stay in sync"
    );

    let out_dir = external_artifact_dir("wp-kernel-012-mt-101");
    let _ = std::fs::create_dir_all(&out_dir);
    match harness.render() {
        Ok(image) => {
            let (w, h) = (image.width(), image.height());
            assert!(w > 0 && h > 0, "rendered image is non-empty");
            let out_path = out_dir.join("model_session_launch_dialog.png");
            let saved = image.save(&out_path).is_ok();
            let abs = std::fs::canonicalize(&out_path).unwrap_or(out_path.clone());
            println!(
                "MT-101 model-session dialog screenshot: {w}x{h}, saved={saved} ({})",
                abs.display()
            );
            assert!(
                saved,
                "HBR-VIS: the model-session launch dialog screenshot PNG saved"
            );
        }
        Err(e) => {
            println!(
                "BLOCKER(non-fatal): MT-101 model-session dialog screenshot render unavailable \
                 (no wgpu adapter): {e}. The widget-id and wire proofs still stand."
            );
        }
    }
}

#[test]
fn model_session_launch_request_builds_real_jobs_post_with_explicit_fields() {
    let rt = runtime();
    let client = ModelSessionLaunchClient::new("http://127.0.0.1:37501", rt.handle().clone());
    let request = ModelSessionLaunchRequest::new(
        ModelSessionProvider::Local,
        "default-project",
        "D:/Projects/Handshake/repo",
        "qwen2.5-coder:7b",
        "repo-folder-wrapper-v1",
    );

    let spec = client.jobs_request(&request).expect("valid jobs request");
    assert_eq!(spec.method, HttpMethod::Post);
    assert_eq!(spec.url, "http://127.0.0.1:37501/jobs");
    let body = spec.body.expect("POST /jobs body");
    assert_eq!(body["job_kind"], serde_json::json!("model_run"));
    assert_eq!(
        body["protocol_id"],
        serde_json::json!(MODEL_SESSION_PROTOCOL_ID)
    );
    assert!(
        body.get("doc_id").is_none(),
        "folder-only launch must not invent a doc_id"
    );
    let inputs = &body["job_inputs"];
    assert_eq!(inputs["workspace_id"], serde_json::json!("default-project"));
    assert_eq!(
        inputs["workspace_folder"],
        serde_json::json!("D:/Projects/Handshake/repo")
    );
    assert_eq!(
        inputs["working_dir"],
        serde_json::json!("D:/Projects/Handshake/repo")
    );
    assert_eq!(inputs["model_provider"], serde_json::json!("local"));
    assert_eq!(inputs["model_id"], serde_json::json!("qwen2.5-coder:7b"));
    assert_eq!(inputs["backend"], serde_json::json!("local"));
    assert_eq!(
        inputs["wrapper"],
        serde_json::json!("repo-folder-wrapper-v1")
    );
    assert_eq!(
        inputs["wp_id"],
        serde_json::json!("WP-KERNEL-012-Native-Editors-Obsidian-VSCode-Parity-v1")
    );
    assert_eq!(inputs["mt_id"], serde_json::json!("MT-101"));
}

#[test]
fn model_session_direct_spawn_returns_endpoint_missing_without_fake_session() {
    let request = ModelSessionLaunchRequest::new(
        ModelSessionProvider::Cloud,
        "default-project",
        "D:/Projects/Handshake/repo",
        "gpt-5.4",
        "repo-folder-wrapper-v1",
    );

    let err = ModelSessionLaunchClient::direct_spawn_workspace("http://127.0.0.1:37501", &request)
        .expect_err("direct spawn is IPC-only, not a fake session");
    assert!(err.is_endpoint_missing());
    match err {
        ModelSessionLaunchError::EndpointMissing {
            probed_path,
            probed_url,
            ipc_channel,
            ipc_owner,
            request,
        } => {
            assert_eq!(probed_path, MODEL_SESSION_DIRECT_SPAWN_PROBED_PATH);
            assert_eq!(probed_url, "http://127.0.0.1:37501/swarm/sessions");
            assert_eq!(ipc_channel, MODEL_SESSION_LAUNCH_IPC_CHANNEL);
            assert_eq!(ipc_owner, MODEL_SESSION_LAUNCH_IPC_OWNER);
            assert_eq!(request.provider, ModelSessionProvider::Cloud);
            assert_eq!(request.worktree_id, None);
            assert_eq!(request.working_dir, "D:/Projects/Handshake/repo");
            assert_eq!(request.cloud_model_name.as_deref(), Some("gpt-5.4"));
            assert_eq!(request.local_model_id, None);
            assert_eq!(request.artifact_path, None);
            assert_eq!(request.sha256_expected, None);
            assert_eq!(request.runtime_binding, None);
        }
        other => panic!("expected EndpointMissing, got {other:?}"),
    }
}

#[test]
fn model_session_direct_spawn_records_local_replay_gaps_without_fake_runtime_binding() {
    let request = ModelSessionLaunchRequest::new(
        ModelSessionProvider::Local,
        "default-project",
        "D:/Projects/Handshake/repo",
        "qwen2.5-coder:7b",
        "repo-folder-wrapper-v1",
    );

    let err = ModelSessionLaunchClient::direct_spawn_workspace("http://127.0.0.1:37501", &request)
        .expect_err("direct spawn is IPC-only, not a fake session");
    match err {
        ModelSessionLaunchError::EndpointMissing { request, .. } => {
            assert_eq!(request.local_model_id.as_deref(), Some("qwen2.5-coder:7b"));
            assert_eq!(request.cloud_model_name, None);
            assert_eq!(request.artifact_path, None);
            assert_eq!(request.sha256_expected, None);
            assert_eq!(
                request.runtime_binding, None,
                "runtime_binding is a runtime adapter, not the operator's model id"
            );
        }
        other => panic!("expected EndpointMissing, got {other:?}"),
    }
}

#[test]
fn model_session_launch_command_is_addressable_and_enabled() {
    let row = all_commands()
        .iter()
        .find(|cmd| cmd.id == CMD_MODEL_SESSION_LAUNCH_WORKSPACE)
        .expect("model-session launch command is present");

    assert_eq!(row.kind, CommandKind::App);
    assert_eq!(row.stable_id, MODEL_SESSION_LAUNCH_WORKSPACE_STABLE_ID);
    assert_eq!(row.label, "Model Session: Launch in Workspace Folder");
    assert!(!row.disabled);
    assert!(!effective_disabled(row, true));
    assert!(row.description.contains("POST /jobs"));
    assert!(row.description.contains("EndpointMissing"));
    assert!(row.description.contains("Tauri IPC-only"));
}

#[test]
fn run_menu_model_session_launch_opens_compact_dialog_with_stable_ids() {
    let mut harness =
        Harness::builder().build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), ok_app());
    harness.run();

    harness.get_by_label("RUN").click();
    harness.run();
    let menu_nodes = live_author_nodes(&harness);
    assert!(
        menu_nodes.iter().any(|(author_id, role, _)| author_id
            == MENU_RUN_MODEL_SESSION_LAUNCH_AUTHOR_ID
            && role == "MenuItem"),
        "RUN menu exposes model-session launch leaf: {menu_nodes:?}"
    );

    harness
        .get_by_label("Launch Model Session in Workspace Folder")
        .click();
    harness.run();
    harness.run();

    assert!(harness.state().model_session_launch_dialog_open_for_test());
    let nodes = live_author_nodes(&harness);
    for id in [
        MODEL_SESSION_LAUNCH_DIALOG_AUTHOR_ID,
        MODEL_SESSION_LAUNCH_PROVIDER_AUTHOR_ID,
        MODEL_SESSION_LAUNCH_FOLDER_AUTHOR_ID,
        MODEL_SESSION_LAUNCH_MODEL_AUTHOR_ID,
        MODEL_SESSION_LAUNCH_WRAPPER_AUTHOR_ID,
        MODEL_SESSION_LAUNCH_START_AUTHOR_ID,
        MODEL_SESSION_LAUNCH_INLINE_STATUS_AUTHOR_ID,
    ] {
        assert!(
            nodes.iter().any(|(author_id, _, _)| author_id == id),
            "{id} must be live in the compact dialog: {nodes:?}"
        );
    }

    harness.get_by_label("Provider Local").click();
    harness.run();
    let nodes = live_author_nodes(&harness);
    for id in [
        MODEL_SESSION_LAUNCH_PROVIDER_LOCAL_AUTHOR_ID,
        MODEL_SESSION_LAUNCH_PROVIDER_CLOUD_AUTHOR_ID,
    ] {
        assert!(
            nodes.iter().any(|(author_id, _, _)| author_id == id),
            "{id} must be exposed while the provider picker is open: {nodes:?}"
        );
    }
}

#[test]
fn palette_model_session_dispatch_opens_same_dialog() {
    let mut harness =
        Harness::builder().build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), ok_app());
    harness.run();

    assert!(
        harness
            .state_mut()
            .dispatch_palette_action_for_test(CMD_MODEL_SESSION_LAUNCH_WORKSPACE),
        "palette dispatch opens model-session launch dialog"
    );
    harness.run();
    assert!(harness.state().model_session_launch_dialog_open_for_test());
}

#[test]
fn launch_dialog_posts_jobs_on_the_wire_and_surfaces_honest_status() {
    let _guard = wgpu_guard();
    let rt = runtime();
    let (base_url, captured) = capture_server(r#"{"job_id":"job-mt101","status":"queued"}"#);
    let mut app = ok_app();
    app.set_backend_base_url_for_test(&base_url, rt.handle().clone());
    app.set_model_session_launch_dialog_for_test(ModelSessionLaunchDialogState {
        provider: ModelSessionProvider::Local,
        workspace_folder: "D:/Projects/Handshake/repo".to_owned(),
        model_id: "qwen2.5-coder:7b".to_owned(),
        wrapper: "repo-folder-wrapper-v1".to_owned(),
    });
    let mut harness = Harness::builder()
        .with_size(egui::vec2(900.0, 700.0))
        .wgpu()
        .build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), app);
    harness.run();
    harness.get_by_label("Launch model session").click();
    harness.run();

    let captured = captured.join().expect("captured POST /jobs");
    assert_eq!(
        captured.request_line,
        format!("POST {MODEL_SESSION_JOBS_PATH} HTTP/1.1")
    );
    assert_eq!(captured.body["job_kind"], serde_json::json!("model_run"));
    assert_eq!(
        captured.body["protocol_id"],
        serde_json::json!("protocol-default")
    );
    assert_eq!(
        captured.body["job_inputs"]["working_dir"],
        serde_json::json!("D:/Projects/Handshake/repo")
    );
    assert_eq!(
        captured.body["job_inputs"]["model_id"],
        serde_json::json!("qwen2.5-coder:7b")
    );
    assert_eq!(
        captured.body["job_inputs"]["wrapper"],
        serde_json::json!("repo-folder-wrapper-v1")
    );

    for _ in 0..20 {
        harness.run();
        if harness
            .state()
            .model_session_launch_status_for_test()
            .is_some_and(|s| s.contains("Model session: /jobs job job-mt101"))
        {
            break;
        }
        std::thread::sleep(Duration::from_millis(25));
    }

    let status = harness
        .state()
        .model_session_launch_status_for_test()
        .expect("model-session status exists");
    assert!(status.contains("Model session: /jobs job job-mt101"));
    assert!(status.contains("NEEDS_MANAGED_RESOURCE_PROOF"));
    assert!(status.contains("EndpointMissing kernel_swarm_spawn_session"));
    assert!(
        status.contains(&format!(
            "{base_url}{MODEL_SESSION_DIRECT_SPAWN_PROBED_PATH}"
        )),
        "state-recovery status preserves the injected direct-spawn probe URL"
    );
    assert!(
        !harness.state().model_session_launch_pending_for_test(),
        "pending flag clears after the /jobs result drains"
    );

    let nodes = live_author_nodes(&harness);
    let (_, role, label) = nodes
        .iter()
        .find(|(author_id, _, _)| author_id == MODEL_SESSION_LAUNCH_STATUS_AUTHOR_ID)
        .unwrap_or_else(|| {
            panic!(
                "model-session status node '{MODEL_SESSION_LAUNCH_STATUS_AUTHOR_ID}' must be live: {nodes:?}"
            )
        });
    assert_eq!(role, "Status");
    let label = label.as_deref().expect("status node label");
    assert!(label.contains("Model session: /jobs job job-mt101"));
    assert!(label.contains("EndpointMissing kernel_swarm_spawn_session"));

    match harness.render() {
        Ok(image) => {
            let out_dir = external_artifact_dir("wp-kernel-012-mt-101");
            let _ = std::fs::create_dir_all(&out_dir);
            let out_path = out_dir.join("model_session_launch_post_submit.png");
            let saved = image.save(&out_path).is_ok();
            let abs = std::fs::canonicalize(&out_path).unwrap_or(out_path.clone());
            println!(
                "MT-101 model-session post-submit screenshot saved={saved} ({})",
                abs.display()
            );
            assert!(
                saved,
                "HBR-VIS: post-submit model-session status screenshot PNG saved"
            );
        }
        Err(e) => {
            println!(
                "BLOCKER(non-fatal): MT-101 post-submit screenshot render unavailable \
                 (no wgpu adapter): {e}. The widget-id and wire proofs still stand."
            );
        }
    }
}

#[test]
fn launch_dialog_disables_duplicate_submit_while_jobs_request_is_pending() {
    let rt = runtime();
    let (base_url, captured) = capture_server_delayed(
        r#"{"job_id":"job-mt101","status":"queued"}"#,
        Duration::from_millis(250),
    );
    let mut app = ok_app();
    app.set_backend_base_url_for_test(&base_url, rt.handle().clone());
    app.set_model_session_launch_dialog_for_test(ModelSessionLaunchDialogState {
        provider: ModelSessionProvider::Local,
        workspace_folder: "D:/Projects/Handshake/repo".to_owned(),
        model_id: "qwen2.5-coder:7b".to_owned(),
        wrapper: "repo-folder-wrapper-v1".to_owned(),
    });
    let mut harness =
        Harness::builder().build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), app);
    harness.run();
    harness.get_by_label("Launch model session").click();
    harness.run();
    assert!(
        harness.state().model_session_launch_pending_for_test(),
        "first click sets the pending gate before the delayed /jobs response returns"
    );
    let status = harness
        .state()
        .model_session_launch_status_for_test()
        .expect("pending status exists");
    assert!(status.contains("POST /jobs pending"));

    harness.get_by_label("Launch model session").click();
    harness.run();
    assert!(
        harness.state().model_session_launch_pending_for_test(),
        "second click cannot clear or replace the pending launch"
    );

    let captured = captured.join().expect("captured one POST /jobs");
    assert_eq!(
        captured.request_line,
        format!("POST {MODEL_SESSION_JOBS_PATH} HTTP/1.1")
    );
}

#[test]
fn launch_dialog_rejects_jobs_response_without_job_id() {
    let rt = runtime();
    let (base_url, captured) = capture_server(r#"{"id":"workflow-only","status":"queued"}"#);
    let mut app = ok_app();
    app.set_backend_base_url_for_test(&base_url, rt.handle().clone());
    app.set_model_session_launch_dialog_for_test(ModelSessionLaunchDialogState {
        provider: ModelSessionProvider::Local,
        workspace_folder: "D:/Projects/Handshake/repo".to_owned(),
        model_id: "qwen2.5-coder:7b".to_owned(),
        wrapper: "repo-folder-wrapper-v1".to_owned(),
    });
    let mut harness =
        Harness::builder().build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), app);
    harness.run();
    harness.get_by_label("Launch model session").click();
    harness.run();
    let _ = captured.join().expect("captured POST /jobs");

    for _ in 0..20 {
        harness.run();
        if harness
            .state()
            .model_session_launch_status_for_test()
            .is_some_and(|s| s.contains("missing required job_id"))
        {
            break;
        }
        std::thread::sleep(Duration::from_millis(25));
    }

    let status = harness
        .state()
        .model_session_launch_status_for_test()
        .expect("model-session status exists");
    assert!(status.contains("POST /jobs failed"));
    assert!(status.contains("missing required job_id"));
    assert!(
        !status.contains("NEEDS_MANAGED_RESOURCE_PROOF"),
        "a malformed /jobs response is not accepted as job creation"
    );
    assert!(
        !harness.state().model_session_launch_pending_for_test(),
        "pending flag clears after parse failure drains"
    );
}
