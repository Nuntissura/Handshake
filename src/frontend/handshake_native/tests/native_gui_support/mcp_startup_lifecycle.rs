//! Actual MCP startup and app transfer without a GPU, watcher, account or backend grant.

use std::io::{BufRead, BufReader, ErrorKind, Write};
use std::net::{SocketAddr, TcpStream};
use std::sync::Arc;
use std::time::Duration;

use egui_kittest::Harness;
use handshake_native::app::HandshakeApp;
use handshake_native::mcp::startup::NativeMcpStartup;
use handshake_native::mcp::{binding, McpBinding, ERR_INVALID_PARAMS, MAIN_WINDOW_ID};
use serde_json::{json, Value};

use super::{external_artifact_dir, host_startup_test_guard, EnvGuard};

const IO_TIMEOUT: Duration = Duration::from_secs(5);

struct TestAppData {
    _env: EnvGuard,
    _directory: tempfile::TempDir,
}

impl TestAppData {
    fn new() -> Self {
        let root = external_artifact_dir("wp-kernel-012-mt-212");
        std::fs::create_dir_all(&root).expect("create scoped startup-test artifact directory");
        let directory = tempfile::Builder::new()
            .prefix("mcp-startup-")
            .tempdir_in(root)
            .expect("create isolated MCP app-data directory");
        #[cfg(target_os = "windows")]
        let key = "LOCALAPPDATA";
        #[cfg(not(target_os = "windows"))]
        let key = "XDG_DATA_HOME";
        Self {
            _env: EnvGuard::set_path(key, directory.path()),
            _directory: directory,
        }
    }
}

struct Client {
    reader: BufReader<TcpStream>,
    next_id: u64,
}

impl Client {
    fn connect(binding: &McpBinding) -> Self {
        let address: SocketAddr = binding.tcp_addr.parse().expect("loopback socket address");
        let stream = TcpStream::connect_timeout(&address, IO_TIMEOUT)
            .expect("connect to actual prestarted MCP listener");
        stream
            .set_read_timeout(Some(IO_TIMEOUT))
            .expect("bound read wait");
        stream
            .set_write_timeout(Some(IO_TIMEOUT))
            .expect("bound write wait");
        Self {
            reader: BufReader::new(stream),
            next_id: 1,
        }
    }

    fn request(&mut self, binding: &McpBinding, method: &str, params: Value) -> Value {
        let id = self.next_id;
        self.next_id += 1;
        let request = json!({
            "jsonrpc": "2.0", "id": id, "method": method, "params": params,
            "session_token": binding.token, "agent_label": "mcp-startup-lifecycle",
            "client_session_id": "mcp-startup-lifecycle",
        });
        let mut line = serde_json::to_vec(&request).expect("serialize MCP request");
        line.push(b'\n');
        self.reader
            .get_mut()
            .write_all(&line)
            .expect("write actual TCP request");
        self.reader.get_mut().flush().expect("flush TCP request");
        let mut response = String::new();
        let received = self
            .reader
            .read_line(&mut response)
            .expect("bounded TCP response");
        assert!(received > 0, "MCP connection closed before its response");
        let response: Value = serde_json::from_str(&response).expect("JSON-RPC response");
        assert_eq!(response["id"], id, "response belongs to this request");
        response
    }

    fn assert_closed(&mut self, binding: &McpBinding) {
        let mut line = String::new();
        match self.reader.read_line(&mut line) {
            Ok(0) => {}
            Err(error)
                if matches!(
                    error.kind(),
                    ErrorKind::ConnectionReset | ErrorKind::ConnectionAborted
                ) => {}
            other => panic!("owned connection did not close after owner drop: {other:?}"),
        }
        let address: SocketAddr = binding.tcp_addr.parse().expect("loopback socket address");
        match TcpStream::connect_timeout(&address, IO_TIMEOUT) {
            Err(error) if error.kind() == ErrorKind::ConnectionRefused => {}
            other => panic!("owned listener still accepts or has an unexpected outcome: {other:?}"),
        }
        assert!(
            !binding::binding_path().exists(),
            "owned discovery binding removed"
        );
    }
}

fn assert_binding_current(expected: &McpBinding) {
    let bytes = std::fs::read(binding::binding_path()).expect("actual discovery binding exists");
    let current: McpBinding =
        serde_json::from_slice(&bytes).expect("parse actual discovery binding");
    assert!(
        current == *expected,
        "discovery token, endpoint and process identity remain unchanged"
    );
}

#[test]
fn prestarted_mcp_rejects_unpublished_operations_and_cleans_abandoned_creator() {
    let _host_guard = host_startup_test_guard();
    let _app_data = TestAppData::new();
    let startup = NativeMcpStartup::start();
    let binding = startup
        .binding()
        .expect("actual startup server bound")
        .clone();
    assert_binding_current(&binding);
    let mut client = Client::connect(&binding);

    let listing = client.request(&binding, "argus.list_windows", json!({}));
    let windows = listing["result"]["windows"]
        .as_array()
        .expect("window listing");
    assert_eq!(windows.len(), 1);
    assert_eq!(windows[0]["window_id"], MAIN_WINDOW_ID);
    assert_eq!(windows[0]["snapshot_available"], false);
    assert_eq!(windows[0]["revision"], 0);

    let placeholder = client.request(&binding, "argus.inspect", json!({}));
    assert_eq!(placeholder["result"]["widget_count"], 1);
    assert_eq!(placeholder["result"]["root"]["actions"], json!([]));
    assert_eq!(placeholder["result"]["root"]["children"], json!([]));
    assert!(placeholder["result"]["viewport"].is_null());

    for method in ["argus.inspect", "argus.click", "argus.set_value"] {
        let response = client.request(
            &binding,
            method,
            json!({
                "window_id": MAIN_WINDOW_ID, "expected_revision": 0,
                "target": "shell.chrome.theme-toggle", "value": "unavailable",
            }),
        );
        assert_eq!(response["error"]["code"], ERR_INVALID_PARAMS);
        assert_eq!(
            response["error"]["message"],
            "Argus window has no published live snapshot"
        );
        assert!(response.get("result").is_none());
    }
    let screenshot = client.request(&binding, "argus.screenshot", json!({}));
    assert_eq!(screenshot["error"]["code"], ERR_INVALID_PARAMS);
    assert_eq!(
        screenshot["error"]["message"],
        "Argus window has no published live snapshot"
    );
    assert!(screenshot.get("result").is_none());

    // This exercises abandoning the actual creator type before invocation, not a GPU failure.
    let creator: eframe::AppCreator<'static> = Box::new(move |cc| {
        Ok(Box::new(HandshakeApp::new_with_prestarted_mcp(
            cc, None, startup,
        )))
    });
    drop(creator);
    client.assert_closed(&binding);
}

#[test]
fn prestarted_mcp_transfers_live_connection_to_headless_app_and_cleans_on_drop() {
    let _host_guard = host_startup_test_guard();
    let _app_data = TestAppData::new();
    let startup = NativeMcpStartup::start();
    let binding = startup
        .binding()
        .expect("actual startup server bound")
        .clone();
    let mut client = Client::connect(&binding);
    let before = client.request(&binding, "argus.list_windows", json!({}));
    assert_eq!(before["result"]["windows"][0]["snapshot_available"], false);

    let mut harness: Harness<HandshakeApp> = Harness::builder().build_eframe(move |cc| {
        assert!(
            cc.wgpu_render_state.is_none(),
            "this ownership test must not create a GPU"
        );
        HandshakeApp::new_with_prestarted_mcp(cc, None, startup)
    });
    harness.run_steps(1);
    let app = harness.state();
    let server = app.mcp_server().expect("app owns the prestarted server");
    assert!(
        server.binding() == &binding,
        "app retains the original server binding"
    );
    assert!(
        app.mcp_token().matches(&binding.token),
        "app retains the startup token"
    );
    assert!(Arc::ptr_eq(
        &app.mcp_snapshot_slot(),
        &server.safety().snapshot
    ));
    assert!(Arc::ptr_eq(
        &app.mcp_action_channel(),
        &server.safety().channel
    ));
    assert_eq!(
        app.mcp_action_channel()
            .lock()
            .expect("action channel")
            .pending(),
        0
    );
    assert_binding_current(&binding);

    // The connection accepted by the startup runtime must survive consumption of its holder.
    let listing = client.request(&binding, "argus.list_windows", json!({}));
    let main = listing["result"]["windows"]
        .as_array()
        .expect("window listing")
        .iter()
        .find(|window| window["window_id"] == MAIN_WINDOW_ID)
        .expect("main window");
    assert_eq!(main["snapshot_available"], true);
    let revision = main["revision"]
        .as_u64()
        .expect("live publication revision");
    assert!(revision > 0);
    let snapshot = client.request(
        &binding,
        "argus.inspect",
        json!({
            "window_id": MAIN_WINDOW_ID, "expected_revision": revision,
        }),
    );
    assert!(
        snapshot.get("error").is_none(),
        "live inspect failed: {snapshot}"
    );
    assert!(
        snapshot["result"]["snapshot"]["widget_count"]
            .as_u64()
            .expect("live widget count")
            > 1
    );

    drop(harness);
    client.assert_closed(&binding);
}
