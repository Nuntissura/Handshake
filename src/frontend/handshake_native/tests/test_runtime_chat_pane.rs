//! WP-KERNEL-012 MT-098: Runtime Chat pane beside the native editor work surface.

use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use egui_kittest::kittest::NodeT;
#[path = "native_gui_support/screenshot_harness.rs"]
mod screenshot_harness;
use screenshot_harness::ScreenshotHarness as Harness;
#[path = "native_gui_support/argus_surface_proof.rs"]
mod argus_surface_proof;
use argus_surface_proof::{prove_argus_surface, ArgusMutation};
use handshake_native::accessibility::{UiNodeBounds, UiTreeNode, UiTreeSnapshot};
use handshake_native::app::{HandshakeApp, HealthDisplayState};
use handshake_native::backend_client::HealthInfo;
use handshake_native::mcp::{
    dispatch_request, ActionChannel, McpRequest, ScreenshotError, SessionToken,
};
use handshake_native::pane_registry::PaneType;
use handshake_native::runtime_chat::{
    runtime_chat_turn_body_author_id, runtime_chat_turn_role_author_id, ChatRole, ChatSendError,
    RuntimeChatClient, RuntimeChatPanel, RUNTIME_CHAT_INPUT_AUTHOR_ID,
    RUNTIME_CHAT_PANEL_AUTHOR_ID, RUNTIME_CHAT_SEND_AUTHOR_ID, RUNTIME_CHAT_STATUS_AUTHOR_ID,
};
use handshake_native::theme::HsTheme;

static WGPU_SERIAL_GUARD: std::sync::Mutex<()> = std::sync::Mutex::new(());

const CHAT_PROBE_ACTIONS: &[egui::accesskit::Action] = &[
    egui::accesskit::Action::Click,
    egui::accesskit::Action::Focus,
    egui::accesskit::Action::SetValue,
];

fn chat_snapshot(harness: &Harness<'_, impl Sized>) -> UiTreeSnapshot {
    let mut children = Vec::new();
    for node in harness.root().children_recursive() {
        let access = node.accesskit_node();
        let node_id = access.id().0;
        let author_id = access.author_id().map(str::to_owned);
        children.push(UiTreeNode {
            id: author_id
                .clone()
                .unwrap_or_else(|| format!("node:{node_id}")),
            author_id,
            node_id,
            role: format!("{:?}", access.role()),
            label: access.label(),
            value: access.value(),
            disabled: access.is_disabled(),
            actions: CHAT_PROBE_ACTIONS
                .iter()
                .filter(|action| access.data().supports_action(**action))
                .map(|action| format!("{action:?}"))
                .collect(),
            bounds: None::<UiNodeBounds>,
            children: Vec::new(),
        });
    }
    let widget_count = children.len() + 1;
    UiTreeSnapshot {
        root: UiTreeNode {
            id: "node:runtime-chat-proof-root".to_owned(),
            author_id: None,
            node_id: 0,
            role: "Window".to_owned(),
            label: None,
            value: None,
            disabled: false,
            actions: Vec::new(),
            bounds: None,
            children,
        },
        captured_at_utc: "0.000000000Z".to_owned(),
        widget_count,
    }
}

fn mcp_request(method: &str, params: serde_json::Value) -> McpRequest {
    McpRequest {
        id: serde_json::json!(1),
        method: method.to_owned(),
        params,
        session_token: "runtime-chat-session".to_owned(),
    }
}

fn ok_app() -> HandshakeApp {
    HandshakeApp::with_health(HealthDisplayState::Ok(HealthInfo {
        status: "ok".to_owned(),
        db_status: "ok".to_owned(),
        migration_version: Some(1),
    }))
}

#[test]
fn mt108_argus_runtime_chat_real_server_loop() {
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1400.0, 900.0))
        .build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), ok_app());
    harness.run_steps(4);

    prove_argus_surface(
        &mut harness,
        "runtime chat pane",
        RUNTIME_CHAT_PANEL_AUTHOR_ID,
        ArgusMutation::SetValue {
            target: RUNTIME_CHAT_INPUT_AUTHOR_ID,
            value: "canonical Argus runtime draft",
        },
        RUNTIME_CHAT_PANEL_AUTHOR_ID,
        true,
        |harness| {
            let input_value = harness
                .root()
                .children_recursive()
                .find(|node| {
                    node.accesskit_node().author_id() == Some(RUNTIME_CHAT_INPUT_AUTHOR_ID)
                })
                .and_then(|node| node.accesskit_node().value());
            if input_value.as_deref() != Some("canonical Argus runtime draft") {
                return Err(format!(
                    "expected runtime draft value, observed {input_value:?}"
                ));
            }
            Ok(serde_json::json!({ "input_value": input_value }))
        },
    );
}

fn external_artifact_dir(subdir: &str) -> PathBuf {
    Path::new("../../../../Handshake_Artifacts/handshake-test").join(subdir)
}

fn live_author_nodes(
    harness: &Harness<'_, HandshakeApp>,
) -> HashMap<String, (String, Option<String>, bool)> {
    let mut found = HashMap::new();
    for node in harness.root().children_recursive() {
        let ak = node.accesskit_node();
        if let Some(author_id) = ak.author_id() {
            found.insert(
                author_id.to_owned(),
                (format!("{:?}", ak.role()), ak.label(), ak.is_disabled()),
            );
        }
    }
    found
}

fn rect_for(harness: &Harness<'_, HandshakeApp>, author_id: &str) -> egui::Rect {
    harness
        .root()
        .children_recursive()
        .find(|node| node.accesskit_node().author_id() == Some(author_id))
        .unwrap_or_else(|| panic!("missing node {author_id}"))
        .rect()
}

struct ChatProbeServer {
    base_url: String,
    addr: SocketAddr,
    request_rx: mpsc::Receiver<Result<String, String>>,
    shutdown: Arc<AtomicBool>,
    thread: Option<thread::JoinHandle<()>>,
}

#[derive(Clone, Copy)]
struct ChatProbeResponse {
    status_line: &'static str,
    content_type: Option<&'static str>,
    body: &'static str,
    delay: Duration,
}

impl ChatProbeResponse {
    const fn bare(status_line: &'static str) -> Self {
        Self {
            status_line,
            content_type: None,
            body: "",
            delay: Duration::ZERO,
        }
    }
}

impl ChatProbeServer {
    fn base_url(&self) -> &str {
        &self.base_url
    }

    fn recv_request(&self) -> String {
        self.request_rx
            .recv_timeout(Duration::from_secs(2))
            .expect("controlled Runtime Chat server publishes a bounded result")
            .expect("controlled Runtime Chat server captured a complete request")
    }

    fn join(mut self) {
        if let Some(thread) = self.thread.take() {
            thread.join().expect("controlled Runtime Chat server exits");
        }
    }
}

impl Drop for ChatProbeServer {
    fn drop(&mut self) {
        self.shutdown.store(true, Ordering::Release);
        // Wake a nonblocking accept loop immediately. If the real client is already connected, the
        // stream's short read timeout observes shutdown and exits within the same bounded teardown.
        let _ = TcpStream::connect_timeout(&self.addr, Duration::from_millis(100));
        if let Some(thread) = self.thread.take() {
            thread
                .join()
                .expect("controlled Runtime Chat server teardown joins its thread");
        }
    }
}

fn spawn_chat_probe_server(status_line: &'static str) -> ChatProbeServer {
    spawn_chat_probe_server_with_body_and_delay(status_line, None, "", Duration::ZERO)
}

fn spawn_chat_probe_server_with_body_and_delay(
    status_line: &'static str,
    content_type: Option<&'static str>,
    body: &'static str,
    response_delay: Duration,
) -> ChatProbeServer {
    spawn_chat_probe_server_with_responses(vec![ChatProbeResponse {
        status_line,
        content_type,
        body,
        delay: response_delay,
    }])
}

fn spawn_absent_chat_probe_server() -> ChatProbeServer {
    spawn_chat_probe_server_with_responses(vec![
        ChatProbeResponse::bare("404 Not Found"),
        ChatProbeResponse::bare("404 Not Found"),
    ])
}

fn spawn_chat_probe_server_with_responses(responses: Vec<ChatProbeResponse>) -> ChatProbeServer {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind controlled Runtime Chat server");
    listener
        .set_nonblocking(true)
        .expect("controlled server uses bounded nonblocking accept");
    let addr = listener.local_addr().expect("server address");
    let base_url = format!("http://{addr}");
    let (request_tx, request_rx) = mpsc::channel();
    let shutdown = Arc::new(AtomicBool::new(false));
    let thread_shutdown = Arc::clone(&shutdown);
    let server_thread = thread::spawn(move || {
        for response in responses {
            let accept_deadline = Instant::now() + Duration::from_secs(2);
            let mut stream = loop {
                if thread_shutdown.load(Ordering::Acquire) {
                    return;
                }
                match listener.accept() {
                    Ok((stream, _)) => break stream,
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        if Instant::now() >= accept_deadline {
                            let _ =
                                request_tx
                                    .send(Err("controlled Runtime Chat server accept timed out"
                                        .to_owned()));
                            return;
                        }
                        thread::sleep(Duration::from_millis(5));
                    }
                    Err(error) => {
                        let _ = request_tx.send(Err(format!(
                            "controlled Runtime Chat server accept failed: {error}"
                        )));
                        return;
                    }
                }
            };
            if thread_shutdown.load(Ordering::Acquire) {
                return;
            }
            stream
                .set_read_timeout(Some(Duration::from_millis(100)))
                .expect("bound request read timeout");
            let mut request = Vec::new();
            let mut chunk = [0_u8; 2048];
            let read_deadline = Instant::now() + Duration::from_secs(2);
            loop {
                if thread_shutdown.load(Ordering::Acquire) {
                    return;
                }
                let read = match stream.read(&mut chunk) {
                    Ok(read) => read,
                    Err(error)
                        if matches!(
                            error.kind(),
                            std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
                        ) =>
                    {
                        if Instant::now() >= read_deadline {
                            let _ = request_tx.send(Err(
                                "controlled Runtime Chat request read timed out".to_owned(),
                            ));
                            return;
                        }
                        continue;
                    }
                    Err(error) => {
                        let _ = request_tx.send(Err(format!(
                            "controlled Runtime Chat request read failed: {error}"
                        )));
                        return;
                    }
                };
                if read == 0 {
                    let _ = request_tx.send(Err(
                        "controlled Runtime Chat client closed before a complete request"
                            .to_owned(),
                    ));
                    return;
                }
                request.extend_from_slice(&chunk[..read]);
                let Some(header_end) = request.windows(4).position(|w| w == b"\r\n\r\n") else {
                    continue;
                };
                let headers = String::from_utf8_lossy(&request[..header_end]);
                let content_length = headers
                    .lines()
                    .find_map(|line| {
                        let (name, value) = line.split_once(':')?;
                        name.eq_ignore_ascii_case("content-length")
                            .then(|| value.trim().parse::<usize>().ok())
                            .flatten()
                    })
                    .unwrap_or(0);
                if request.len() >= header_end + 4 + content_length {
                    break;
                }
            }
            let request = match String::from_utf8(request) {
                Ok(request) => request,
                Err(error) => {
                    let _ =
                        request_tx.send(Err(format!("Runtime Chat request is not UTF-8: {error}")));
                    return;
                }
            };
            if request_tx.send(Ok(request)).is_err() {
                return;
            }
            let response_at = Instant::now() + response.delay;
            while Instant::now() < response_at {
                if thread_shutdown.load(Ordering::Acquire) {
                    return;
                }
                thread::sleep(Duration::from_millis(5));
            }
            let content_type_header = response
                .content_type
                .map(|value| format!("Content-Type: {value}\r\n"))
                .unwrap_or_default();
            let _ = write!(
                stream,
                "HTTP/1.1 {}\r\n{content_type_header}Content-Length: {}\r\nConnection: close\r\n\r\n{}",
                response.status_line,
                response.body.len(),
                response.body
            );
            let _ = stream.flush();
        }
    });
    ChatProbeServer {
        base_url,
        addr,
        request_rx,
        shutdown,
        thread: Some(server_thread),
    }
}

fn wait_for_chat_delivery(panel: &Arc<Mutex<RuntimeChatPanel>>) -> ChatSendError {
    for _ in 0..200 {
        {
            let mut panel = panel.lock().unwrap_or_else(|error| error.into_inner());
            panel.drain_deliveries_for_test();
            if let Some(error) = panel.last_error_for_test() {
                return error.clone();
            }
        }
        thread::sleep(Duration::from_millis(5));
    }
    panic!("Runtime Chat transport did not deliver within one second");
}

#[test]
fn runtime_chat_real_post_maps_router_404_to_typed_endpoint_missing() {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("Runtime Chat test runtime");
    let server = spawn_absent_chat_probe_server();
    let client = RuntimeChatClient::new(server.base_url(), runtime.handle().clone());
    assert_eq!(
        client.probed_path(),
        "/chat",
        "RuntimeChatClient must follow the MT-defined handshake_core route convention"
    );
    assert_ne!(
        client.probed_path(),
        "/api/flight_recorder/runtime_chat_event",
        "Flight Recorder runtime-chat event ingestion is observability, not chat send/receive"
    );
    let panel = Arc::new(Mutex::new(RuntimeChatPanel::new(
        client,
        HsTheme::Dark.palette(),
    )));
    {
        let mut panel = panel.lock().expect("panel");
        panel.set_draft_for_test("   ");
        let empty = panel
            .send_current_message_for_test()
            .expect_err("empty panel send returns a typed validation blocker");
        assert!(empty.is_empty_message());
        panel.set_draft_for_test("hello from transport proof");
        panel
            .send_current_message_for_test()
            .expect("non-empty send dispatches an off-thread transport probe");
    }
    let request = server.recv_request();
    let capability_request = server.recv_request();
    server.join();
    let (head, body) = request
        .split_once("\r\n\r\n")
        .expect("captured HTTP request has headers and body");
    assert!(head.starts_with("POST /chat HTTP/1.1\r\n"), "{head}");
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(body).expect("probe JSON body"),
        serde_json::json!({ "prompt": "hello from transport proof" })
    );
    assert!(
        capability_request.starts_with("HSK-CAPABILITY /chat HTTP/1.1\r\n"),
        "404 classification must query the composed router, not infer absence from response bytes: {capability_request}"
    );

    let err = wait_for_chat_delivery(&panel);
    assert!(err.is_endpoint_missing());
    assert!(matches!(
        err,
        ChatSendError::EndpointMissing { ref probed_path } if probed_path == "/chat"
    ));
    let panel = panel.lock().expect("panel");
    assert!(
        !panel
            .turns_for_test()
            .iter()
            .any(|turn| turn.role == ChatRole::Assistant),
        "EndpointMissing must not synthesize an assistant response"
    );
    assert_eq!(
        panel
            .turns_for_test()
            .iter()
            .filter(|turn| turn.role == ChatRole::User)
            .count(),
        1,
        "the submitted operator message remains visible while no assistant reply is fabricated"
    );
}

#[test]
fn runtime_chat_non_fallback_responses_keep_distinct_typed_failures() {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("Runtime Chat typed-error runtime");
    for (status_line, expected_status, success_without_contract) in
        [("403 Forbidden", 403, false), ("204 No Content", 204, true)]
    {
        let server = spawn_chat_probe_server(status_line);
        let panel = Arc::new(Mutex::new(RuntimeChatPanel::new(
            RuntimeChatClient::new(server.base_url(), runtime.handle().clone()),
            HsTheme::Dark.palette(),
        )));
        {
            let mut panel = panel.lock().expect("panel");
            panel.set_draft_for_test(format!("status probe {expected_status}"));
            panel
                .send_current_message_for_test()
                .expect("typed-error probe dispatches");
        }
        let request = server.recv_request();
        server.join();
        assert!(request.starts_with("POST /chat HTTP/1.1\r\n"));
        let error = wait_for_chat_delivery(&panel);
        if success_without_contract {
            assert!(matches!(
                error,
                ChatSendError::ResponseContractMissing { status, .. } if status == expected_status
            ));
        } else {
            assert!(matches!(
                error,
                ChatSendError::HttpStatus { status, .. } if status == expected_status
            ));
        }
        assert!(
            !panel
                .lock()
                .expect("panel")
                .turns_for_test()
                .iter()
                .any(|turn| turn.role == ChatRole::Assistant),
            "neither rejection nor an undefined success body may fabricate assistant output"
        );
    }
}

#[test]
fn registered_route_bare_semantic_404_is_not_misclassified_as_missing_route() {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("Runtime Chat semantic-404 runtime");
    let server = spawn_chat_probe_server_with_responses(vec![
        // The real POST handler's semantic response is intentionally byte-identical to Axum's fallback.
        ChatProbeResponse::bare("404 Not Found"),
        // A composed router with a registered POST path rejects the unsupported capability method.
        ChatProbeResponse::bare("405 Method Not Allowed"),
    ]);
    let panel = Arc::new(Mutex::new(RuntimeChatPanel::new(
        RuntimeChatClient::new(server.base_url(), runtime.handle().clone()),
        HsTheme::Dark.palette(),
    )));
    panel
        .lock()
        .expect("panel")
        .set_draft_for_test("semantic 404 probe");
    panel
        .lock()
        .expect("panel")
        .send_current_message_for_test()
        .expect("semantic-404 probe dispatches");
    let request = server.recv_request();
    let capability_request = server.recv_request();
    server.join();
    assert!(request.starts_with("POST /chat HTTP/1.1\r\n"));
    assert!(
        capability_request.starts_with("HSK-CAPABILITY /chat HTTP/1.1\r\n"),
        "semantic 404 classification must use the composed-router capability signal"
    );

    let error = wait_for_chat_delivery(&panel);
    assert!(matches!(
        error,
        ChatSendError::HttpStatus { status: 404, .. }
    ));
}

#[test]
fn duplicate_send_and_stale_delivery_cannot_mutate_active_generation() {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("Runtime Chat correlation runtime");
    let server = spawn_chat_probe_server_with_body_and_delay(
        "404 Not Found",
        None,
        "",
        Duration::from_millis(500),
    );
    let mut panel = RuntimeChatPanel::new(
        RuntimeChatClient::new(server.base_url(), runtime.handle().clone()),
        HsTheme::Dark.palette(),
    );
    panel.set_draft_for_test("first send");
    panel
        .send_current_message_for_test()
        .expect("first send dispatches");
    let _request = server.recv_request();
    let generation = panel
        .active_send_generation_for_test()
        .expect("first send owns a generation");

    panel.set_draft_for_test("duplicate send");
    let duplicate = panel
        .send_current_message_for_test()
        .expect_err("state machine rejects a second send while the first is active");
    assert!(matches!(
        duplicate,
        ChatSendError::AlreadyInFlight { generation: active } if active == generation
    ));
    assert!(duplicate.is_already_in_flight());
    assert_eq!(
        panel
            .turns_for_test()
            .iter()
            .filter(|turn| turn.role == ChatRole::User)
            .count(),
        1,
        "a rejected duplicate does not append another user turn"
    );

    panel.inject_delivery_for_test(
        generation.wrapping_sub(1),
        Err(ChatSendError::HttpStatus {
            probed_path: "/chat".to_owned(),
            status: 500,
        }),
    );
    panel.drain_deliveries_for_test();
    assert!(
        panel.send_in_flight_for_test(),
        "a stale completion cannot clear the active generation"
    );
    assert!(
        panel.last_error_for_test().is_none(),
        "a stale completion cannot overwrite current visible error state"
    );

    panel.rebind_client(RuntimeChatClient::new(
        "http://127.0.0.1:9",
        runtime.handle().clone(),
    ));
    assert!(
        !panel.send_in_flight_for_test(),
        "transport rebind cancels and clears the task owned by the old binding"
    );
    drop(server);
}

// ── MT-108 (MT-098 residual): LIVE type-and-click send observes EndpointMissing ────────────────────

#[test]
fn live_type_and_click_after_dropped_injected_runtime_uses_app_owned_worker() {
    // Drive the REAL mounted ChatPaneFactory inside HandshakeApp. This deliberately uses the runtime
    // owned by `with_health`; a dormant current-thread runtime would leave the visible status at Probing.
    let server = spawn_absent_chat_probe_server();
    let mut app = ok_app();
    app.set_runtime_chat_base_url_for_test(server.base_url());
    let injected_runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("caller-owned injected runtime");
    app.set_runtime_handle(injected_runtime.handle().clone());
    drop(injected_runtime);
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1400.0, 900.0))
        .build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), app);
    harness.run_steps(4);

    // Precondition: Send is disabled with an empty draft.
    assert!(
        live_author_nodes_ui(&harness)
            .get(RUNTIME_CHAT_SEND_AUTHOR_ID)
            .is_some_and(|disabled| *disabled),
        "Send starts disabled with an empty draft"
    );

    // Drive the canonical MCP `set_value` producer: resolve runtime-chat-input from the live snapshot,
    // enqueue the typed SetValue action through dispatch_request, and feed the real ActionChannel into
    // egui.
    let token = SessionToken::from_hex("runtime-chat-session");
    let mut channel = ActionChannel::new();
    let snapshot = chat_snapshot(&harness);
    let set_response = dispatch_request(
        &mcp_request(
            "argus.set_value",
            serde_json::json!({
                "target": RUNTIME_CHAT_INPUT_AUTHOR_ID,
                "value": "hello from the live UI"
            }),
        ),
        &token,
        &snapshot,
        &mut channel,
        || Err(ScreenshotError("not used".to_owned())),
    );
    assert_eq!(set_response.to_json()["result"]["queued"], true);
    assert_eq!(set_response.to_json()["result"]["action"], "SetValue");
    for event in channel.drain_into_events() {
        harness.event(event);
    }
    harness.run();
    harness.run();

    let input_value = harness
        .root()
        .children_recursive()
        .find(|node| node.accesskit_node().author_id() == Some(RUNTIME_CHAT_INPUT_AUTHOR_ID))
        .and_then(|node| node.accesskit_node().value());
    assert_eq!(
        input_value.as_deref(),
        Some("hello from the live UI"),
        "native SetValue updated the visible Runtime Chat input"
    );

    // A non-empty draft enabled the live Send button (the rendered UI reflects the message text).
    assert!(
        live_author_nodes_ui(&harness)
            .get(RUNTIME_CHAT_SEND_AUTHOR_ID)
            .is_some_and(|disabled| !*disabled),
        "a non-empty draft enabled the Send button"
    );

    // CLICK through the canonical MCP click_widget producer as well.
    let snapshot = chat_snapshot(&harness);
    let click_response = dispatch_request(
        &mcp_request(
            "argus.click",
            serde_json::json!({ "target": RUNTIME_CHAT_SEND_AUTHOR_ID }),
        ),
        &token,
        &snapshot,
        &mut channel,
        || Err(ScreenshotError("not used".to_owned())),
    );
    assert_eq!(click_response.to_json()["result"]["queued"], true);
    assert_eq!(click_response.to_json()["result"]["action"], "Click");
    for event in channel.drain_into_events() {
        harness.event(event);
    }
    harness.run();
    harness.run();

    let request = server.recv_request();
    let capability_request = server.recv_request();
    server.join();
    assert!(
        request.starts_with("POST /chat HTTP/1.1\r\n"),
        "live click must use the canonical planned route: {request}"
    );
    assert!(
        capability_request.starts_with("HSK-CAPABILITY /chat HTTP/1.1\r\n"),
        "mounted Runtime Chat must classify the 404 through the composed-router capability signal"
    );
    for _ in 0..200 {
        harness.run();
        let status = live_author_nodes(&harness)
            .get(RUNTIME_CHAT_STATUS_AUTHOR_ID)
            .and_then(|(_role, label, _disabled)| label.clone());
        if status.is_some_and(|label| label.contains("EndpointMissing")) {
            break;
        }
        thread::sleep(Duration::from_millis(5));
    }

    // The mounted pane completed on the app-owned runtime, returned to a stable non-spinner state, kept
    // the submitted user turn, and fabricated no assistant turn.
    let nodes = live_author_nodes(&harness);
    let status_label = nodes
        .get(RUNTIME_CHAT_STATUS_AUTHOR_ID)
        .and_then(|(_role, label, _disabled)| label.as_deref())
        .expect("mounted Runtime Chat status remains visible");
    assert!(
        status_label.contains("EndpointMissing") && status_label.contains("/chat"),
        "mounted click-send must surface the typed EndpointMissing blocker: {status_label}"
    );
    assert!(
        !status_label.contains("Probing"),
        "completed mounted send must not leave a perpetual spinner: {status_label}"
    );
    let role_one = harness
        .root()
        .children_recursive()
        .find(|node| {
            node.accesskit_node().author_id() == Some(runtime_chat_turn_role_author_id(1).as_str())
        })
        .and_then(|node| node.accesskit_node().label())
        .expect("submitted user turn has a stable transcript role node");
    let body_one = harness
        .root()
        .children_recursive()
        .find(|node| {
            node.accesskit_node().author_id() == Some(runtime_chat_turn_body_author_id(1).as_str())
        })
        .and_then(|node| node.accesskit_node().label())
        .expect("submitted user turn has a stable transcript body node");
    assert_eq!(role_one, "You:");
    assert_eq!(body_one, "hello from the live UI");
    assert!(
        harness.root().children_recursive().all(|node| {
            node.accesskit_node().author_id() != Some(runtime_chat_turn_role_author_id(2).as_str())
        }),
        "EndpointMissing must not fabricate an assistant turn"
    );
}

/// author_id -> is_disabled for every live node (a lightweight view for the send-enabled assertions).
fn live_author_nodes_ui(harness: &Harness<'_, impl Sized>) -> HashMap<String, bool> {
    let mut found = HashMap::new();
    for node in harness.root().children_recursive() {
        let ak = node.accesskit_node();
        if let Some(author_id) = ak.author_id() {
            found.insert(author_id.to_owned(), ak.is_disabled());
        }
    }
    found
}

#[test]
fn live_default_tree_contains_runtime_chat_beside_editors_and_screenshot() {
    let _guard = WGPU_SERIAL_GUARD.lock().unwrap_or_else(|p| p.into_inner());
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1400.0, 900.0))
        .wgpu()
        .build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), ok_app());
    harness.run_steps(4);

    let pane_types: HashMap<String, PaneType> = {
        let registry = harness.state().pane_registry();
        let guard = registry.lock().expect("registry");
        guard
            .iter()
            .map(|(id, record)| (id.to_string(), record.pane_type.clone()))
            .collect()
    };
    assert_eq!(pane_types.get("pane-a"), Some(&PaneType::CodeSymbol));
    assert_eq!(pane_types.get("pane-b"), Some(&PaneType::LoomWikiPage));
    assert_eq!(pane_types.get("pane-c"), Some(&PaneType::RuntimeChat));
    assert!(
        !pane_types.contains_key("pane-d"),
        "fresh MT-098 default stays minimal: no pane-d"
    );

    let nodes = live_author_nodes(&harness);
    for (author_id, role) in [
        ("pane-c", "Region"),
        ("tabbar-pane-c", "TabList"),
        ("tab-pane-c-0", "Tab"),
        (RUNTIME_CHAT_PANEL_AUTHOR_ID, "Region"),
        (RUNTIME_CHAT_STATUS_AUTHOR_ID, "Status"),
        (RUNTIME_CHAT_INPUT_AUTHOR_ID, "TextInput"),
        (RUNTIME_CHAT_SEND_AUTHOR_ID, "Button"),
    ] {
        let Some((actual_role, _label, _disabled)) = nodes.get(author_id) else {
            panic!(
                "missing live Runtime Chat author_id {author_id}; found {:?}",
                nodes.keys()
            );
        };
        assert_eq!(actual_role, role, "{author_id} role");
    }
    let status_label = nodes
        .get(RUNTIME_CHAT_STATUS_AUTHOR_ID)
        .and_then(|(_role, label, _disabled)| label.as_deref())
        .expect("runtime-chat-status label");
    assert!(
        status_label.contains("EndpointMissing") && status_label.contains("/chat"),
        "runtime-chat-status label must expose typed blocker and probed path: {status_label}"
    );
    assert_eq!(
        nodes
            .get(RUNTIME_CHAT_INPUT_AUTHOR_ID)
            .and_then(|(_role, label, _disabled)| label.as_deref()),
        Some("Runtime Chat message"),
        "runtime-chat-input has an explicit model-readable label"
    );
    assert!(
        nodes
            .get(RUNTIME_CHAT_SEND_AUTHOR_ID)
            .is_some_and(|(_role, _label, disabled)| *disabled),
        "runtime-chat-send is disabled until the draft has non-whitespace text"
    );
    assert!(
        !nodes.contains_key("divider-horizontal"),
        "three-column default should not expose a bottom-row divider"
    );

    let pane_a = rect_for(&harness, "pane-a");
    let pane_b = rect_for(&harness, "pane-b");
    let pane_c = rect_for(&harness, "pane-c");
    assert!(
        pane_a.center().x < pane_b.center().x && pane_b.center().x < pane_c.center().x,
        "Runtime Chat must be beside the editors left-to-right: a={pane_a:?}, b={pane_b:?}, c={pane_c:?}"
    );
    for (id, rect) in [("pane-a", pane_a), ("pane-b", pane_b), ("pane-c", pane_c)] {
        assert!(
            rect.width() > 100.0,
            "{id} is not starved horizontally: {rect:?}"
        );
        assert!(
            rect.height() > 300.0,
            "{id} is full-height enough: {rect:?}"
        );
    }

    let image = harness
        .render()
        .expect("wgpu render succeeds for MT-098 Runtime Chat screenshot");
    assert!(
        image.width() > 0 && image.height() > 0,
        "non-empty screenshot"
    );
    let ext_dir = external_artifact_dir("wp-kernel-012-mt-098");
    std::fs::create_dir_all(&ext_dir).expect("create external artifact dir");
    let png_path = ext_dir.join("MT-098-runtime-chat-default.png");
    image
        .save(&png_path)
        .expect("save MT-098 Runtime Chat screenshot");
    println!("MT-098 screenshot: {}", png_path.display());
}
