//! Ignored production-binary Argus socket proof.
//!
//! This target is intentionally outside the default suite. It opens real native windows and requires
//! managed PostgreSQL plus a Palmistry-ready `handshake_core` on `127.0.0.1:37501`. Before running,
//! set `HANDSHAKE_ARGUS_LIVE_BACKEND_READY=1` and point `HANDSHAKE_DIAGNOSTICS_DIR` at the existing
//! absolute directory shared by the backend, Palmistry, and the native child.

#![cfg(target_os = "windows")]

use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::process::{Child, Command};
use std::time::{Duration, Instant};

use base64::Engine as _;
use sha2::{Digest, Sha256};

#[derive(serde::Deserialize)]
struct DiscoveredBinding {
    tcp_addr: String,
    token: String,
    pid: u32,
}

struct ChildGuard(Child);

impl Drop for ChildGuard {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

struct ArgusClient {
    addr: String,
    token: String,
    next_id: u64,
    agent_token: Option<String>,
    transcript: Vec<serde_json::Value>,
}

impl ArgusClient {
    fn call(&mut self, method: &str, params: serde_json::Value) -> serde_json::Value {
        let token = self.token.clone();
        self.call_with_credentials(method, params, &token, "production-socket-live")
    }

    fn call_with_credentials(
        &mut self,
        method: &str,
        params: serde_json::Value,
        token: &str,
        agent_label: &str,
    ) -> serde_json::Value {
        let request = serde_json::json!({
            "jsonrpc": "2.0",
            "id": self.next_id,
            "method": method,
            "params": params,
            "session_token": token,
            "agent_token": self.agent_token.as_deref(),
            "agent_label": agent_label,
        });
        self.next_id += 1;
        let response = rpc(&self.addr, &request)
            .unwrap_or_else(|error| panic!("{method} transport failed: {error}"));
        self.transcript.push(serde_json::json!({
            "request": redact_request_for_proof(&request),
            "response": response,
        }));
        response
    }

    fn authenticate_agent(&mut self) -> String {
        let response = self.call_with_credentials(
            "argus.authenticate_agent",
            serde_json::json!({}),
            &self.token.clone(),
            "production-socket-live",
        );
        assert_success(&response, "argus.authenticate_agent");
        let agent_id = response["result"]["agent_id"]
            .as_str()
            .expect("broker returned agent_id")
            .to_owned();
        self.agent_token = Some(
            response["result"]["agent_token"]
                .as_str()
                .expect("broker returned agent_token")
                .to_owned(),
        );
        agent_id
    }

    fn inspect(&mut self, window_id: &str) -> serde_json::Value {
        let response = self.call("argus.inspect", serde_json::json!({"window_id": window_id}));
        assert_success(&response, "argus.inspect");
        response["result"].clone()
    }

    fn mutation(
        &mut self,
        method: &str,
        window_id: &str,
        author_id: &str,
        extra: Option<(&str, serde_json::Value)>,
    ) -> serde_json::Value {
        let before = self.inspect(window_id);
        let revision = before["revision"]
            .as_u64()
            .expect("inspect revision is numeric");
        let mut params = serde_json::Map::from_iter([
            (
                "window_id".to_owned(),
                serde_json::Value::String(window_id.to_owned()),
            ),
            (
                "author_id".to_owned(),
                serde_json::Value::String(author_id.to_owned()),
            ),
            (
                "expected_snapshot_revision".to_owned(),
                serde_json::Value::from(revision),
            ),
        ]);
        if let Some((key, value)) = extra {
            params.insert(key.to_owned(), value);
        }
        let response = self.call(method, serde_json::Value::Object(params));
        assert_applied_durable(&response, revision, method);
        response
    }
}

fn redact_request_for_proof(request: &serde_json::Value) -> serde_json::Value {
    let mut redacted = request.clone();
    if let Some(object) = redacted.as_object_mut() {
        if object.contains_key("session_token") {
            object.insert(
                "session_token".to_owned(),
                serde_json::Value::String("[REDACTED]".to_owned()),
            );
        }
        if object.contains_key("agent_token") {
            object.insert(
                "agent_token".to_owned(),
                serde_json::Value::String("[REDACTED]".to_owned()),
            );
        }
        if let Some(params) = object
            .get_mut("params")
            .and_then(serde_json::Value::as_object_mut)
        {
            let sensitive = params
                .get("author_id")
                .and_then(serde_json::Value::as_str)
                .is_some_and(handshake_native::accessibility::is_sensitive_author_id);
            if sensitive && params.contains_key("value") {
                params.insert(
                    "value".to_owned(),
                    serde_json::Value::String("[REDACTED]".to_owned()),
                );
            }
        }
    }
    redacted
}

fn assert_visual_png(png: &[u8], context: &str) {
    let image = image::load_from_memory(png)
        .unwrap_or_else(|error| panic!("{context} was not a decodable image: {error}"))
        .to_rgba8();
    let mut colors = std::collections::HashSet::new();
    let mut visible_nonblack = false;
    for pixel in image.pixels() {
        colors.insert(pixel.0);
        visible_nonblack |= pixel[3] != 0 && (pixel[0] > 8 || pixel[1] > 8 || pixel[2] > 8);
        if colors.len() > 4 && visible_nonblack {
            return;
        }
    }
    panic!(
        "{context} was blank/uniform: {} distinct colors, visible_nonblack={visible_nonblack}",
        colors.len()
    );
}

fn rpc(addr: &str, request: &serde_json::Value) -> std::io::Result<serde_json::Value> {
    let stream = TcpStream::connect(addr)?;
    stream.set_read_timeout(Some(Duration::from_secs(15)))?;
    let mut writer = stream.try_clone()?;
    serde_json::to_writer(&mut writer, request)
        .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidData, error))?;
    writer.write_all(b"\n")?;
    writer.flush()?;
    let mut reader = BufReader::new(stream);
    let mut line = String::new();
    reader.read_line(&mut line)?;
    serde_json::from_str(line.trim())
        .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidData, error))
}

fn assert_success(response: &serde_json::Value, operation: &str) {
    assert!(
        response.get("error").is_none() && response.get("result").is_some(),
        "{operation} returned a JSON-RPC error: {response}"
    );
}

fn assert_applied_durable(response: &serde_json::Value, before: u64, operation: &str) {
    assert_success(response, operation);
    let receipt = &response["result"];
    assert_eq!(receipt["status"], "applied", "{operation}: {receipt}");
    assert_eq!(receipt["before_revision"], before, "{operation}: {receipt}");
    assert!(
        receipt["after_revision"]
            .as_u64()
            .is_some_and(|after| after > before),
        "{operation} did not publish a newer revision: {receipt}"
    );
    assert!(
        receipt["evidence_ref"]
            .as_str()
            .is_some_and(|value| !value.is_empty()),
        "{operation} lacked durable evidence: {receipt}"
    );
    assert!(
        receipt["durability_error"].is_null(),
        "{operation} was applied but not durable: {receipt}"
    );
}

fn require_palmistry_ready_backend() -> PathBuf {
    assert_eq!(
        std::env::var("HANDSHAKE_ARGUS_LIVE_BACKEND_READY").as_deref(),
        Ok("1"),
        "set HANDSHAKE_ARGUS_LIVE_BACKEND_READY=1 only after managed PostgreSQL and the production \
         backend are ready for Palmistry"
    );
    let diagnostics_dir = PathBuf::from(
        std::env::var("HANDSHAKE_DIAGNOSTICS_DIR")
            .expect("HANDSHAKE_DIAGNOSTICS_DIR is required for the production Argus proof"),
    );
    assert!(
        diagnostics_dir.is_absolute() && diagnostics_dir.is_dir(),
        "HANDSHAKE_DIAGNOSTICS_DIR must be an existing absolute directory: {}",
        diagnostics_dir.display()
    );

    let mut stream = TcpStream::connect_timeout(
        &"127.0.0.1:37501".parse().expect("fixed backend address"),
        Duration::from_secs(3),
    )
    .expect("handshake_core is not accepting connections on 127.0.0.1:37501");
    stream
        .set_read_timeout(Some(Duration::from_secs(3)))
        .expect("set health timeout");
    stream
        .write_all(b"GET /health HTTP/1.1\r\nHost: 127.0.0.1:37501\r\nConnection: close\r\n\r\n")
        .expect("write backend health probe");
    let mut response = String::new();
    stream
        .read_to_string(&mut response)
        .expect("read backend health probe");
    assert!(
        response.starts_with("HTTP/1.1 200") || response.starts_with("HTTP/1.0 200"),
        "handshake_core /health was not ready: {}",
        response.lines().next().unwrap_or("<empty>")
    );
    diagnostics_dir
}

fn discover_binding(path: &Path, pid: u32, deadline: Instant) -> DiscoveredBinding {
    while Instant::now() < deadline {
        if let Ok(body) = std::fs::read_to_string(path) {
            if let Ok(binding) = serde_json::from_str::<DiscoveredBinding>(&body) {
                if binding.pid == pid && !binding.tcp_addr.is_empty() && !binding.token.is_empty() {
                    return binding;
                }
            }
        }
        std::thread::sleep(Duration::from_millis(200));
    }
    panic!(
        "production child pid {pid} did not publish its owned binding at {}",
        path.display()
    );
}

fn contains_author_id(node: &serde_json::Value, author_id: &str) -> bool {
    node.get("author_id").and_then(|value| value.as_str()) == Some(author_id)
        || node
            .get("children")
            .and_then(|value| value.as_array())
            .is_some_and(|children| {
                children
                    .iter()
                    .any(|child| contains_author_id(child, author_id))
            })
}

fn list_has_window(list_response: &serde_json::Value, window_id: &str) -> bool {
    list_response["result"]["windows"]
        .as_array()
        .is_some_and(|windows| {
            windows.iter().any(|window| {
                window["window_id"] == window_id
                    && window["snapshot_available"].as_bool() == Some(true)
            })
        })
}

fn list_contains_window(list_response: &serde_json::Value, window_id: &str) -> bool {
    list_response["result"]["windows"]
        .as_array()
        .is_some_and(|windows| {
            windows
                .iter()
                .any(|window| window["window_id"] == window_id)
        })
}

fn wait_for_window(client: &mut ArgusClient, window_id: &str, present: bool) {
    let deadline = Instant::now() + Duration::from_secs(15);
    while Instant::now() < deadline {
        let list = client.call("argus.list_windows", serde_json::json!({}));
        assert_success(&list, "argus.list_windows");
        let reached = if present {
            list_has_window(&list, window_id)
        } else {
            !list_contains_window(&list, window_id)
        };
        if reached {
            return;
        }
        std::thread::sleep(Duration::from_millis(200));
    }
    panic!("window {window_id} did not reach present={present}");
}

fn proof_dir() -> PathBuf {
    std::env::var("HANDSHAKE_PROOF_ARTIFACT_DIR")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../../../../Handshake_Artifacts/handshake-test/native_gui")
        })
}

fn request_child_close(pid: u32) {
    use windows_sys::Win32::Foundation::{BOOL, HWND, LPARAM};
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        EnumWindows, GetWindowThreadProcessId, PostMessageW, WM_CLOSE,
    };

    unsafe extern "system" fn close_owned_window(hwnd: HWND, lparam: LPARAM) -> BOOL {
        let wanted_pid = lparam as u32;
        let mut window_pid = 0u32;
        unsafe {
            GetWindowThreadProcessId(hwnd, &mut window_pid);
            if window_pid == wanted_pid {
                PostMessageW(hwnd, WM_CLOSE, 0, 0);
            }
        }
        1
    }

    unsafe {
        EnumWindows(Some(close_owned_window), pid as LPARAM);
    }
}

#[ignore = "LIVE production socket E2E: opens native main/pop-out windows and requires managed \
            PostgreSQL, Palmistry-ready handshake_core on 127.0.0.1:37501, \
            HANDSHAKE_ARGUS_LIVE_BACKEND_READY=1, and a shared HANDSHAKE_DIAGNOSTICS_DIR"]
#[test]
fn production_binary_argus_socket_inspect_click_set_screenshot_receipts_and_popout() {
    let diagnostics_dir = require_palmistry_ready_backend();
    let tmp = std::env::temp_dir().join(format!(
        "hsk_argus_production_socket_{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&tmp).expect("create isolated LOCALAPPDATA");
    let binding_path = tmp.join("handshake").join("swarm_mcp_binding.json");

    let child = Command::new(env!("CARGO_BIN_EXE_handshake-native"))
        .env("LOCALAPPDATA", &tmp)
        .env("HANDSHAKE_DIAGNOSTICS_DIR", &diagnostics_dir)
        .spawn()
        .expect("spawn production handshake-native binary");
    let child_pid = child.id();
    let mut child_guard = ChildGuard(child);
    let binding = discover_binding(
        &binding_path,
        child_pid,
        Instant::now() + Duration::from_secs(30),
    );
    let mut client = ArgusClient {
        addr: binding.tcp_addr,
        token: binding.token,
        next_id: 1,
        agent_token: None,
        transcript: Vec::new(),
    };
    let authenticated_agent_id = client.authenticate_agent();
    assert!(!authenticated_agent_id.is_empty());

    let windows = client.call("argus.list_windows", serde_json::json!({}));
    assert_success(&windows, "argus.list_windows");
    assert!(list_has_window(&windows, "main"), "main window not listed");

    let initial = client.inspect("main");
    assert!(contains_author_id(
        &initial["snapshot"]["root"],
        "shell.chrome.theme-toggle"
    ));
    assert!(contains_author_id(
        &initial["snapshot"]["root"],
        "bottom-rail.input"
    ));

    client.mutation("argus.click", "main", "shell.chrome.theme-toggle", None);
    client.mutation(
        "argus.set_value",
        "main",
        "bottom-rail.input",
        Some((
            "value",
            serde_json::Value::String("production-socket-proof".to_owned()),
        )),
    );
    let after_set = client.inspect("main");
    assert!(
        serde_json::to_string(&after_set["snapshot"]["root"])
            .expect("serialize tree")
            .contains("production-socket-proof"),
        "set value was not visible in the next canonical snapshot"
    );

    let screenshot = client.call("argus.screenshot", serde_json::json!({"window_id": "main"}));
    assert_success(&screenshot, "argus.screenshot(main)");
    let png = base64::engine::general_purpose::STANDARD
        .decode(
            screenshot["result"]["png_base64"]
                .as_str()
                .expect("screenshot png_base64"),
        )
        .expect("decode screenshot PNG");
    assert!(png.starts_with(b"\x89PNG\r\n\x1a\n"), "capture is not PNG");
    assert_eq!(
        screenshot["result"]["sha256"],
        format!("{:x}", Sha256::digest(&png))
    );
    assert_eq!(screenshot["result"]["window_id"], "main");
    assert_eq!(screenshot["result"]["pid"], child_pid);
    assert!(screenshot["result"]["width"]
        .as_u64()
        .is_some_and(|v| v > 0));
    assert!(screenshot["result"]["height"]
        .as_u64()
        .is_some_and(|v| v > 0));
    assert_visual_png(&png, "main-window capture");

    // Exercise the actual MT-015 Settings/cloud-access surface through the production socket.
    client.mutation("argus.click", "main", "menu-help", None);
    let help_menu = client.inspect("main");
    assert!(
        contains_author_id(&help_menu["snapshot"]["root"], "menu.help.settings"),
        "HELP menu did not expose Open Settings"
    );
    client.mutation("argus.click", "main", "menu.help.settings", None);
    let settings = client.inspect("main");
    for author_id in [
        "settings.dialog",
        "settings.cloud.byok.openai.key",
        "settings.cloud.byok.openai.status",
        "settings.cloud.byok.openai.save",
        "settings.cloud.byok.anthropic.key",
        "settings.cloud.byok.anthropic.status",
        "settings.cloud.cli.claude_code.status",
        "settings.cloud.cli.codex.status",
    ] {
        assert!(
            contains_author_id(&settings["snapshot"]["root"], author_id),
            "production Settings snapshot omitted {author_id}"
        );
    }
    let settings_json =
        serde_json::to_string(&settings["snapshot"]["root"]).expect("serialize Settings tree");
    assert!(
        !settings_json.contains("production-socket-secret-canary"),
        "Settings snapshot disclosed the BYOK canary"
    );

    // Canonical non-pointer context-menu opening, causal menu-item acknowledgement, and real pop-out.
    client.mutation(
        "argus.show_context_menu",
        "main",
        "pane-pane-a-header",
        None,
    );
    let menu_snapshot = client.inspect("main");
    assert!(
        contains_author_id(&menu_snapshot["snapshot"]["root"], "ctx-menu.pane.pop_out"),
        "pane context menu did not expose its stable pop-out item"
    );
    client.mutation("argus.click", "main", "ctx-menu.pane.pop_out", None);
    wait_for_window(&mut client, "popout-pane-a", true);
    let popout = client.inspect("popout-pane-a");
    assert!(contains_author_id(
        &popout["snapshot"]["root"],
        "popout-window-pane-a"
    ));
    let popout_shot = client.call(
        "argus.screenshot",
        serde_json::json!({"window_id": "popout-pane-a"}),
    );
    assert_success(&popout_shot, "argus.screenshot(popout-pane-a)");
    assert_eq!(popout_shot["result"]["window_id"], "popout-pane-a");
    assert_eq!(popout_shot["result"]["pid"], child_pid);
    assert!(
        popout_shot["result"]["title"]
            .as_str()
            .is_some_and(|title| !title.is_empty()),
        "pop-out capture lacked its exact OS title"
    );
    let popout_png = base64::engine::general_purpose::STANDARD
        .decode(
            popout_shot["result"]["png_base64"]
                .as_str()
                .expect("pop-out screenshot png_base64"),
        )
        .expect("decode pop-out screenshot PNG");
    assert!(
        popout_png.starts_with(b"\x89PNG\r\n\x1a\n"),
        "pop-out capture is not PNG"
    );
    assert_eq!(
        popout_shot["result"]["sha256"],
        format!("{:x}", Sha256::digest(&popout_png))
    );
    assert_visual_png(&popout_png, "detached-window capture");

    // A detached window must be steerable, not merely enumerable/capturable.
    client.mutation(
        "argus.show_context_menu",
        "popout-pane-a",
        "pane-pane-a-header",
        None,
    );
    let popout_after_action = client.inspect("popout-pane-a");
    assert!(
        contains_author_id(
            &popout_after_action["snapshot"]["root"],
            "ctx-menu.pane.lock"
        ) || contains_author_id(
            &popout_after_action["snapshot"]["root"],
            "ctx-menu.pane.pop_out"
        ),
        "detached-window mutation did not produce an observable newer snapshot"
    );

    client.mutation("argus.click", "main", "merge-back-pane-a", None);
    wait_for_window(&mut client, "popout-pane-a", false);

    // Protocol fences: each negative case must fail and therefore cannot be mistaken for an action.
    let bad_token = client.call_with_credentials(
        "argus.inspect",
        serde_json::json!({"window_id": "main"}),
        "not-the-session-token",
        "production-socket-live",
    );
    assert!(bad_token.get("error").is_some(), "bad token was accepted");
    let valid_token = client.token.clone();
    let missing_label = client.call_with_credentials(
        "argus.inspect",
        serde_json::json!({"window_id": "main"}),
        &valid_token,
        "",
    );
    assert!(
        missing_label.get("error").is_some(),
        "missing agent_label was accepted"
    );
    let wrong_window = client.call(
        "argus.inspect",
        serde_json::json!({"window_id": "does-not-exist"}),
    );
    assert!(
        wrong_window.get("error").is_some(),
        "unknown window was accepted"
    );
    let secret_canary = "production-socket-secret-canary";
    let secret_denial = client.call(
        "argus.set_value",
        serde_json::json!({
            "window_id": "main",
            "author_id": "settings.cloud.byok.openai.key",
            "value": secret_canary
        }),
    );
    assert!(
        secret_denial.get("error").is_some(),
        "secret-bearing input accepted generic Argus set_value"
    );
    assert!(
        !secret_denial.to_string().contains(secret_canary),
        "secret-bearing denial echoed its value"
    );
    let current_revision = client.inspect("main")["revision"]
        .as_u64()
        .expect("current main revision");
    let stale = client.call(
        "argus.click",
        serde_json::json!({
            "window_id": "main",
            "author_id": "shell.chrome.theme-toggle",
            "expected_snapshot_revision": current_revision.saturating_sub(1)
        }),
    );
    assert!(stale.get("error").is_some(), "stale revision was accepted");

    let proof_dir = proof_dir();
    std::fs::create_dir_all(&proof_dir).expect("create external proof directory");
    std::fs::write(proof_dir.join("argus_production_socket_main.png"), &png)
        .expect("write production screenshot proof");
    std::fs::write(
        proof_dir.join("argus_production_socket_popout_pane_a.png"),
        &popout_png,
    )
    .expect("write production pop-out screenshot proof");
    let transcript =
        serde_json::to_vec_pretty(&client.transcript).expect("serialize redacted transcript");
    assert!(
        !String::from_utf8_lossy(&transcript).contains(client.token.as_str()),
        "proof transcript retained the live session token"
    );
    assert!(
        client.agent_token.as_deref().is_none_or(|agent_token| {
            !String::from_utf8_lossy(&transcript).contains(agent_token)
        }),
        "proof transcript retained the broker-minted agent token"
    );
    assert!(
        !String::from_utf8_lossy(&transcript).contains(secret_canary),
        "proof transcript retained the sensitive-value canary"
    );
    std::fs::write(
        proof_dir.join("argus_production_socket_transcript.json"),
        transcript,
    )
    .expect("write production socket transcript");

    request_child_close(child_pid);
    let exit_deadline = Instant::now() + Duration::from_secs(10);
    while Instant::now() < exit_deadline {
        if child_guard
            .0
            .try_wait()
            .expect("poll production child")
            .is_some()
        {
            break;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    assert!(
        child_guard
            .0
            .try_wait()
            .expect("final production child poll")
            .is_some(),
        "production child did not exit after WM_CLOSE"
    );
    let deadline = Instant::now() + Duration::from_secs(5);
    while binding_path.exists() && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(100));
    }
    assert!(
        !binding_path.exists(),
        "owned binding survived production child shutdown"
    );
    let _ = std::fs::remove_dir_all(&tmp);
}
