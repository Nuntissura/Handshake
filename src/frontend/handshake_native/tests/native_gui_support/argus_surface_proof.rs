//! Real localhost Argus JSON-RPC proof loop for one mounted native GUI surface.

#![allow(dead_code)]

use std::io::{Read, Write};
use std::sync::{Arc, Mutex};

use egui::accesskit;
use egui_kittest::kittest::NodeT;
use handshake_native::accessibility::{UiNodeBounds, UiTreeNode, UiTreeSnapshot};
use handshake_native::mcp::{
    ActionChannel, ScreenshotError, ScreenshotResult, SessionToken, SwarmMcpServer,
};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;

use crate::screenshot_harness::screenshot_marker;
use crate::screenshot_harness::ScreenshotHarness;

const PROBE_ACTIONS: &[accesskit::Action] = &[
    accesskit::Action::Click,
    accesskit::Action::Focus,
    accesskit::Action::SetValue,
    accesskit::Action::ReplaceSelectedText,
    accesskit::Action::ScrollIntoView,
];
const PROCESS_CORRELATION_ID_ENV: &str = "HANDSHAKE_PROOF_PROCESS_CORRELATION_ID";
const PROCESS_SCENARIO_ID_ENV: &str = "HANDSHAKE_PROOF_PROCESS_SCENARIO_ID";

const EXACT_ARGUS_SURFACES: [&str; 7] = [
    "diagnostics panel",
    "find bar",
    "formatting toolbar",
    "outline pane",
    "rich find/replace panel",
    "runtime chat pane",
    "slash menu",
];

#[derive(Clone, Copy)]
struct ExpectedArgusSurface {
    surface: &'static str,
    process_scenario: &'static str,
    test_binary: &'static str,
    test_name: &'static str,
    inspect_author_id: &'static str,
    mutation_method: &'static str,
    mutation_target: &'static str,
    reinspect_author_id: &'static str,
    reinspect_author_present: bool,
}

const EXPECTED_ARGUS_CONTRACTS: [ExpectedArgusSurface; 7] = [
    ExpectedArgusSurface {
        surface: "diagnostics panel",
        process_scenario: "diagnostics_panel",
        test_binary: "test_diagnostics_panel",
        test_name: "mt108_argus_diagnostics_panel_real_server_loop",
        inspect_author_id: handshake_native::diagnostics::DIAGNOSTICS_PANEL_AUTHOR_ID,
        mutation_method: handshake_native::mcp::ARGUS_CLICK_METHOD,
        mutation_target:
            handshake_native::visual_debugger::WORKSURFACE_INSPECTOR_DUMP_BUTTON_AUTHOR_ID,
        reinspect_author_id: handshake_native::diagnostics::DIAGNOSTICS_PANEL_AUTHOR_ID,
        reinspect_author_present: true,
    },
    ExpectedArgusSurface {
        surface: "find bar",
        process_scenario: "find_bar",
        test_binary: "test_find_bar_accesskit",
        test_name: "mt108_argus_find_bar_real_server_loop",
        inspect_author_id: handshake_native::code_editor::CODE_EDITOR_FIND_BAR_AUTHOR_ID,
        mutation_method: handshake_native::mcp::ARGUS_SET_VALUE_METHOD,
        mutation_target: handshake_native::code_editor::CODE_EDITOR_FIND_BAR_AUTHOR_ID,
        reinspect_author_id: handshake_native::code_editor::CODE_EDITOR_FIND_BAR_AUTHOR_ID,
        reinspect_author_present: true,
    },
    ExpectedArgusSurface {
        surface: "formatting toolbar",
        process_scenario: "formatting_toolbar",
        test_binary: "test_formatting_toolbar",
        test_name: "mt108_argus_formatting_toolbar_real_server_loop",
        inspect_author_id: "toolbar-btn-toggle_bold",
        mutation_method: handshake_native::mcp::ARGUS_CLICK_METHOD,
        mutation_target: "toolbar-btn-toggle_bold",
        reinspect_author_id: "toolbar-btn-toggle_bold",
        reinspect_author_present: true,
    },
    ExpectedArgusSurface {
        surface: "outline pane",
        process_scenario: "outline_pane",
        test_binary: "test_outline",
        test_name: "mt108_argus_outline_real_server_loop",
        inspect_author_id:
            handshake_native::rich_editor::outline_panel::OUTLINE_CONTAINER_AUTHOR_ID,
        mutation_method: handshake_native::mcp::ARGUS_CLICK_METHOD,
        mutation_target:
            handshake_native::manual_content_editors::MT108_ARGUS_OUTLINE_PROOF_AUTHOR_ID,
        reinspect_author_id:
            handshake_native::rich_editor::outline_panel::OUTLINE_CONTAINER_AUTHOR_ID,
        reinspect_author_present: true,
    },
    ExpectedArgusSurface {
        surface: "rich find/replace panel",
        process_scenario: "rich_find_replace",
        test_binary: "test_rich_find_replace",
        test_name: "mt108_argus_rich_find_replace_real_server_loop",
        inspect_author_id: handshake_native::rich_editor::find_replace::FIND_PANEL_AUTHOR_ID,
        mutation_method: handshake_native::mcp::ARGUS_SET_VALUE_METHOD,
        mutation_target: handshake_native::rich_editor::find_replace::FIND_INPUT_AUTHOR_ID,
        reinspect_author_id: handshake_native::rich_editor::find_replace::FIND_PANEL_AUTHOR_ID,
        reinspect_author_present: true,
    },
    ExpectedArgusSurface {
        surface: "runtime chat pane",
        process_scenario: "runtime_chat",
        test_binary: "test_runtime_chat_pane",
        test_name: "mt108_argus_runtime_chat_real_server_loop",
        inspect_author_id: handshake_native::runtime_chat::RUNTIME_CHAT_PANEL_AUTHOR_ID,
        mutation_method: handshake_native::mcp::ARGUS_SET_VALUE_METHOD,
        mutation_target: handshake_native::runtime_chat::RUNTIME_CHAT_INPUT_AUTHOR_ID,
        reinspect_author_id: handshake_native::runtime_chat::RUNTIME_CHAT_PANEL_AUTHOR_ID,
        reinspect_author_present: true,
    },
    ExpectedArgusSurface {
        surface: "slash menu",
        process_scenario: "slash_menu",
        test_binary: "test_slash_commands",
        test_name: "mt108_argus_slash_menu_real_server_loop",
        inspect_author_id: handshake_native::rich_editor::slash_commands::SLASH_MENU_AUTHOR_ID,
        mutation_method: handshake_native::mcp::ARGUS_CLICK_METHOD,
        mutation_target: "slash-item-paragraph",
        reinspect_author_id: handshake_native::rich_editor::slash_commands::SLASH_MENU_AUTHOR_ID,
        reinspect_author_present: false,
    },
];

#[derive(Clone, Copy, Debug)]
pub enum ArgusMutation<'a> {
    Click { target: &'a str },
    SetValue { target: &'a str, value: &'a str },
}

impl<'a> ArgusMutation<'a> {
    fn method(self) -> &'static str {
        match self {
            Self::Click { .. } => handshake_native::mcp::ARGUS_CLICK_METHOD,
            Self::SetValue { .. } => handshake_native::mcp::ARGUS_SET_VALUE_METHOD,
        }
    }

    fn target(self) -> &'a str {
        match self {
            Self::Click { target } | Self::SetValue { target, .. } => target,
        }
    }

    fn params(self) -> serde_json::Value {
        match self {
            Self::Click { target } => serde_json::json!({ "target": target }),
            Self::SetValue { target, value } => {
                serde_json::json!({ "target": target, "value": value })
            }
        }
    }
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
struct ArgusSurfaceEvidence {
    schema_id: String,
    run_id: String,
    outcome_id: String,
    surface: String,
    inspect_author_id: String,
    reinspect_author_id: String,
    mutation_method: String,
    mutation_target: String,
    client_session_id: String,
    agent_id: String,
    action_seq: u64,
    receipt_id: u64,
    receipt_status: String,
    reinspect_author_present: bool,
    observed_post_state: serde_json::Value,
    process_correlation_id: String,
    process_scenario_id: String,
    process_id: u32,
    screenshot_outcome_id: String,
    screenshot_scenario_id: String,
    screenshot_status: String,
    screenshot_frame_path: Option<String>,
    gpu_screenshot_enabled: bool,
}

#[derive(Clone, Debug, serde::Deserialize)]
struct ProcessIdentity {
    pid: u32,
    parent_pid: u32,
    start_time_utc: String,
    executable: String,
}

#[derive(Clone, Debug, serde::Deserialize)]
struct ExternalProcessReceipt {
    schema_id: String,
    run_id: String,
    outcome_id: String,
    process_correlation_id: String,
    child_pid: u32,
    owned_process_tree: Vec<ProcessIdentity>,
    test_process_pid: Option<u32>,
    test_process_start_time_utc: Option<String>,
    test_process_executable: Option<String>,
    child_started_at_utc: String,
    deadline_at_utc: String,
    deadline_seconds: u64,
    command_executable: String,
    command_arguments: Vec<String>,
    command_display: String,
    working_directory: String,
    scenario_id: String,
    status: String,
    exit_code: Option<i32>,
}

pub fn prove_argus_surface<'h, State, VerifyMutation>(
    harness: &mut ScreenshotHarness<'h, State>,
    surface: &str,
    inspect_author_id: &str,
    mutation: ArgusMutation<'_>,
    reinspect_author_id: &str,
    reinspect_author_present: bool,
    verify_mutation: VerifyMutation,
) where
    VerifyMutation: FnOnce(&mut ScreenshotHarness<'h, State>) -> Result<serde_json::Value, String>,
{
    let run_id = require_explicit_argus_run_id();
    let contract = EXPECTED_ARGUS_CONTRACTS
        .iter()
        .find(|contract| contract.surface == surface)
        .expect("proof surface is present in the exact MT-108 contract matrix");
    let process_correlation_id = require_process_env(PROCESS_CORRELATION_ID_ENV);
    let process_scenario_id = require_process_env(PROCESS_SCENARIO_ID_ENV);
    assert_eq!(process_scenario_id, contract.process_scenario);
    let before = snapshot_harness(harness);
    assert!(
        snapshot_has_author(&before, inspect_author_id),
        "{surface}: precondition author_id {inspect_author_id} is mounted"
    );

    let token = SessionToken::generate();
    let token_hex = token.as_hex().to_owned();
    let client_session_id = format!("mt108-{}", sanitize(surface));
    let snapshot = Arc::new(Mutex::new(before));
    let channel = Arc::new(Mutex::new(ActionChannel::new()));
    type CaptureReply = std::sync::mpsc::SyncSender<Result<ScreenshotResult, ScreenshotError>>;
    let (capture_request_tx, capture_request_rx) = std::sync::mpsc::sync_channel::<CaptureReply>(1);
    let capture = Arc::new(move || {
        let (reply_tx, reply_rx) = std::sync::mpsc::sync_channel(1);
        capture_request_tx.send(reply_tx).map_err(|error| {
            ScreenshotError(format!("Argus capture request channel closed: {error}"))
        })?;
        reply_rx
            .recv_timeout(std::time::Duration::from_secs(30))
            .map_err(|error| ScreenshotError(format!("Argus capture reply timed out: {error}")))?
    })
        as Arc<dyn Fn() -> Result<ScreenshotResult, ScreenshotError> + Send + Sync>;

    // The production binding contract resolves discovery through the platform app-data root. Tests
    // use the same binding implementation but redirect that root to this run's external proof area,
    // so they neither overwrite nor contend with a live Handshake process. The guard restores the
    // process environment after the server is dropped, including during panic unwinding.
    let binding_root = screenshot_marker::marker_dir()
        .join("argus-bindings")
        .join(sanitize(surface));
    let _app_data = ScopedAppData::install(binding_root);
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .expect("Argus proof runtime");
    let mut server = runtime
        .block_on(async {
            tokio::time::timeout(
                std::time::Duration::from_secs(10),
                SwarmMcpServer::bind(token, Arc::clone(&snapshot), Arc::clone(&channel), capture),
            )
            .await
        })
        .expect("real Argus localhost binding completed within 10s")
        .expect("real Argus localhost binding");
    let addr = server.tcp_addr().to_owned();

    let before_response = runtime.block_on(rpc_roundtrip(
        &addr,
        request(
            1,
            handshake_native::mcp::ARGUS_INSPECT_METHOD,
            serde_json::json!({}),
            &token_hex,
            &client_session_id,
        ),
    ));
    assert!(
        response_has_author(&before_response["result"], inspect_author_id),
        "{surface}: canonical argus.inspect returned the mounted surface"
    );

    let mutation_response = runtime.block_on(rpc_roundtrip(
        &addr,
        request(
            2,
            mutation.method(),
            mutation.params(),
            &token_hex,
            &client_session_id,
        ),
    ));
    assert_eq!(
        mutation_response["result"]["queued"],
        true,
        "{surface}: Argus mutation response must contain result.queued=true; response={}",
        serde_json::to_string(&mutation_response).unwrap_or_else(|error| {
            format!("<unable to serialize mutation response: {error}>")
        })
    );
    let receipt_id = mutation_response["result"]["receipt_id"]
        .as_u64()
        .expect("mutation response carries a real receipt_id");
    if std::env::var("HANDSHAKE_ARGUS_MATRIX_RUN_ID")
        .ok()
        .is_some_and(|run_id| !run_id.trim().is_empty())
    {
        std::env::set_var("HANDSHAKE_PROOF_ACTION_RECEIPT_ID", receipt_id.to_string());
    }
    let agent_id = mutation_response["result"]["agent_id"]
        .as_str()
        .expect("mutation response carries actual agent_id")
        .to_owned();
    assert!(
        agent_id.ends_with(&format!(":client:{client_session_id}")),
        "actual client_session_id attribution is retained: {agent_id}"
    );

    let events = {
        let live = snapshot.lock().unwrap_or_else(|p| p.into_inner()).clone();
        channel
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .drain_revalidated_into_events(&live)
    };
    assert!(
        !events.is_empty(),
        "{surface}: server mutation reached the shared action channel"
    );
    for event in events {
        harness.event(event);
    }
    harness.step();
    harness.step();

    let after = snapshot_harness(harness);
    {
        let mut shared = snapshot.lock().unwrap_or_else(|p| p.into_inner());
        *shared = after.clone();
    }
    channel
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .acknowledge_after_render(&after);

    let after_response = runtime.block_on(rpc_roundtrip(
        &addr,
        request(
            3,
            handshake_native::mcp::ARGUS_INSPECT_METHOD,
            serde_json::json!({}),
            &token_hex,
            &client_session_id,
        ),
    ));
    assert_eq!(
        response_has_author(&after_response["result"], reinspect_author_id),
        reinspect_author_present,
        "{surface}: fresh canonical argus.inspect observes the expected post-action surface state"
    );
    let receipt = after_response["result"]["action_receipts"]
        .as_array()
        .and_then(|receipts| {
            receipts
                .iter()
                .find(|receipt| receipt["receipt_id"].as_u64() == Some(receipt_id))
        })
        .expect("fresh argus.inspect returns the mutation receipt");
    let receipt_status = receipt["status"]
        .as_str()
        .expect("mutation receipt has a typed status")
        .to_owned();
    assert!(
        matches!(receipt_status.as_str(), "applied" | "indeterminate"),
        "{surface}: post-render receipt must be terminal and non-rejected, got {receipt_status}"
    );
    let observed_post_state = verify_mutation(harness)
        .unwrap_or_else(|error| panic!("{surface}: live mutation predicate failed: {error}"));
    assert!(
        !observed_post_state.is_null(),
        "{surface}: mutation predicate must return a concrete observed post-state"
    );

    let screenshot_request = request(
        4,
        handshake_native::mcp::ARGUS_SCREENSHOT_METHOD,
        serde_json::json!({}),
        &token_hex,
        &client_session_id,
    );
    let screenshot_addr = addr.clone();
    let screenshot_task = runtime.spawn(async move {
        rpc_roundtrip_bounded(
            &screenshot_addr,
            screenshot_request,
            std::time::Duration::from_secs(35),
        )
        .await
    });
    let capture_reply = capture_request_rx
        .recv_timeout(std::time::Duration::from_secs(5))
        .expect("argus.screenshot invoked the bound capture source within 5s");
    let rendered = capture_from_harness(harness);
    let screenshot_status = match &rendered {
        Ok(_) => "CAPTURED",
        Err(error) if error.0.contains("typed DEFERRED") => "DEFERRED",
        Err(_) => "BLOCKED",
    };
    let screenshot_outcome = harness
        .last_screenshot_outcome()
        .cloned()
        .expect("systemic screenshot harness retains the exact marker it durably wrote");
    assert_eq!(screenshot_outcome.run_id, run_id);
    assert_eq!(screenshot_outcome.status, screenshot_status);
    assert_eq!(
        screenshot_outcome.gpu_screenshot_enabled,
        screenshot_marker::gpu_screenshot_enabled()
    );
    capture_reply
        .send(rendered)
        .expect("return runtime screenshot outcome to the real Argus request");
    let screenshot_response = runtime
        .block_on(screenshot_task)
        .expect("join real argus.screenshot request task");
    if screenshot_status == "CAPTURED" {
        let png_base64 = screenshot_response["result"]["png_base64"]
            .as_str()
            .filter(|png| !png.is_empty())
            .expect("CAPTURED argus.screenshot carries PNG base64");
        let width = screenshot_response["result"]["width"]
            .as_u64()
            .expect("CAPTURED argus.screenshot width") as u32;
        let height = screenshot_response["result"]["height"]
            .as_u64()
            .expect("CAPTURED argus.screenshot height") as u32;
        assert!(width > 0 && height > 0);
        use base64::Engine as _;
        let png = base64::engine::general_purpose::STANDARD
            .decode(png_base64)
            .expect("CAPTURED argus.screenshot base64 decodes");
        let decoded = image::load_from_memory_with_format(&png, image::ImageFormat::Png)
            .expect("CAPTURED argus.screenshot is a decodable PNG");
        assert_eq!((decoded.width(), decoded.height()), (width, height));
    } else {
        let error = screenshot_response
            .get("error")
            .expect("non-CAPTURED argus.screenshot returns a typed tool error");
        assert!(
            error["message"]
                .as_str()
                .is_some_and(|message| message.contains("screenshot capture failed")),
            "{surface}: screenshot failure is the capture tool error, got {error}"
        );
        assert!(screenshot_response.get("result").is_none());
    }

    let actions = server.action_log().drain_log();
    let action = actions
        .iter()
        .find(|entry| entry.op_name == mutation.method() && entry.target_key == mutation.target())
        .expect("canonical mutation retained in the ActionLog");
    assert_eq!(action.agent_id, agent_id);

    write_evidence(&ArgusSurfaceEvidence {
        schema_id: "hsk.native_gui.argus_surface_evidence@4".to_owned(),
        run_id,
        outcome_id: format!(
            "pid{}:{}:{}:{}",
            std::process::id(),
            sanitize(surface),
            action.seq,
            now_nanos()
        ),
        surface: surface.to_owned(),
        inspect_author_id: inspect_author_id.to_owned(),
        reinspect_author_id: reinspect_author_id.to_owned(),
        mutation_method: mutation.method().to_owned(),
        mutation_target: mutation.target().to_owned(),
        client_session_id,
        agent_id,
        action_seq: action.seq,
        receipt_id,
        receipt_status,
        reinspect_author_present,
        observed_post_state,
        process_correlation_id,
        process_scenario_id,
        process_id: std::process::id(),
        screenshot_outcome_id: screenshot_outcome.outcome_id,
        screenshot_scenario_id: screenshot_outcome.scenario_id,
        screenshot_status: screenshot_status.to_owned(),
        screenshot_frame_path: screenshot_outcome.frame_path,
        gpu_screenshot_enabled: screenshot_marker::gpu_screenshot_enabled(),
    })
    .expect("durably write locked Argus seven-surface evidence");

    server.shutdown();
}

fn snapshot_harness<State>(harness: &ScreenshotHarness<'_, State>) -> UiTreeSnapshot {
    let children = harness
        .root()
        .children_recursive()
        .map(|node| {
            let ak = node.accesskit_node();
            let author_id = ak.author_id().map(str::to_owned);
            let node_id = ak.id().0;
            UiTreeNode {
                id: author_id
                    .clone()
                    .unwrap_or_else(|| format!("node:{node_id}")),
                author_id,
                node_id,
                role: format!("{:?}", ak.role()),
                label: ak.label(),
                value: ak.value(),
                disabled: ak.is_disabled(),
                actions: PROBE_ACTIONS
                    .iter()
                    .filter(|action| ak.data().supports_action(**action))
                    .map(|action| format!("{action:?}"))
                    .collect(),
                bounds: None::<UiNodeBounds>,
                children: Vec::new(),
            }
        })
        .collect::<Vec<_>>();
    UiTreeSnapshot {
        widget_count: children.len() + 1,
        root: UiTreeNode {
            id: "node:mt108-argus-root".to_owned(),
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
    }
}

fn snapshot_has_author(snapshot: &UiTreeSnapshot, author_id: &str) -> bool {
    snapshot
        .root
        .children
        .iter()
        .any(|node| node.author_id.as_deref() == Some(author_id))
}

fn response_has_author(value: &serde_json::Value, author_id: &str) -> bool {
    value.get("author_id").and_then(serde_json::Value::as_str) == Some(author_id)
        || value
            .get("children")
            .and_then(serde_json::Value::as_array)
            .is_some_and(|children| {
                children
                    .iter()
                    .any(|child| response_has_author(child, author_id))
            })
        || value
            .get("root")
            .is_some_and(|root| response_has_author(root, author_id))
}

fn capture_from_harness<State>(
    harness: &mut ScreenshotHarness<'_, State>,
) -> Result<ScreenshotResult, ScreenshotError> {
    use image::ImageEncoder;
    let image = harness.render().map_err(ScreenshotError)?;
    let (width, height) = (image.width(), image.height());
    let mut png = Vec::new();
    image::codecs::png::PngEncoder::new(&mut png)
        .write_image(
            image.as_raw(),
            width,
            height,
            image::ExtendedColorType::Rgba8,
        )
        .map_err(|error| ScreenshotError(format!("PNG encode failed: {error}")))?;
    Ok(handshake_native::mcp::screenshot::screenshot_from_png(
        &png, width, height,
    ))
}

async fn rpc_roundtrip(addr: &str, request: serde_json::Value) -> serde_json::Value {
    rpc_roundtrip_bounded(addr, request, std::time::Duration::from_secs(5)).await
}

async fn rpc_roundtrip_bounded(
    addr: &str,
    request: serde_json::Value,
    timeout: std::time::Duration,
) -> serde_json::Value {
    tokio::time::timeout(timeout, async move {
        let stream = TcpStream::connect(addr)
            .await
            .expect("connect real Argus binding");
        let (read_half, mut write_half) = tokio::io::split(stream);
        let mut reader = BufReader::new(read_half);
        let mut line = serde_json::to_string(&request).expect("serialize Argus request");
        line.push('\n');
        write_half
            .write_all(line.as_bytes())
            .await
            .expect("write Argus request");
        write_half.flush().await.expect("flush Argus request");
        let mut response = String::new();
        reader
            .read_line(&mut response)
            .await
            .expect("read Argus response");
        serde_json::from_str(response.trim()).expect("parse Argus response")
    })
    .await
    .expect("canonical Argus JSON-RPC roundtrip completed within its deadline")
}

fn request(
    id: u64,
    method: &str,
    params: serde_json::Value,
    token: &str,
    client_session_id: &str,
) -> serde_json::Value {
    serde_json::json!({
        "jsonrpc": "2.0",
        "id": id,
        "method": method,
        "params": params,
        "session_token": token,
        "client_session_id": client_session_id,
    })
}

fn require_explicit_argus_run_id() -> String {
    let configured = std::env::var(screenshot_marker::SCREENSHOT_RUN_ID_ENV)
        .ok()
        .filter(|value| !value.trim().is_empty())
        .expect("MT-108 Argus proof requires an explicit shared HANDSHAKE_SCREENSHOT_RUN_ID");
    let run_id = screenshot_marker::screenshot_run_id();
    assert!(
        !run_id.is_empty() && run_id != format!("pid-{}", std::process::id()),
        "configured Argus run id must sanitize to a non-empty shared id: {configured:?}"
    );
    run_id
}

fn require_process_env(name: &str) -> String {
    std::env::var(name)
        .ok()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| panic!("MT-108 Argus proof requires explicit supervisor context {name}"))
}

fn validate_argus_rows(
    rows: &[ArgusSurfaceEvidence],
    require_exact_seven: bool,
    expected_run_id: Option<&str>,
) -> std::io::Result<()> {
    let expected = EXACT_ARGUS_SURFACES
        .iter()
        .copied()
        .collect::<std::collections::HashSet<_>>();
    let mut surfaces = std::collections::HashSet::new();
    let mut outcomes = std::collections::HashSet::new();
    let run_id = rows
        .first()
        .map(|row| row.run_id.as_str())
        .unwrap_or_default();
    if run_id.trim().is_empty() {
        return Err(std::io::Error::other(
            "Argus aggregate run identity must be non-empty",
        ));
    }
    if expected_run_id.is_some_and(|expected| expected != run_id) {
        return Err(std::io::Error::other(format!(
            "Argus aggregate run id {run_id:?} does not match the supervisor run id {expected_run_id:?}"
        )));
    }
    let mut gpu_posture = None;
    for row in rows {
        if row.schema_id != "hsk.native_gui.argus_surface_evidence@4" || row.run_id != run_id {
            return Err(std::io::Error::other(
                "Argus aggregate mixes schema versions or run identities",
            ));
        }
        if !expected.contains(row.surface.as_str()) {
            return Err(std::io::Error::other(format!(
                "unexpected Argus aggregate surface {:?}",
                row.surface
            )));
        }
        let contract = EXPECTED_ARGUS_CONTRACTS
            .iter()
            .find(|contract| contract.surface == row.surface)
            .expect("EXACT_ARGUS_SURFACES and EXPECTED_ARGUS_CONTRACTS stay aligned");
        if row.outcome_id.trim().is_empty()
            || row.inspect_author_id.trim().is_empty()
            || row.reinspect_author_id.trim().is_empty()
            || row.mutation_target.trim().is_empty()
            || row.client_session_id.trim().is_empty()
            || row.agent_id.trim().is_empty()
            || row.process_correlation_id.trim().is_empty()
            || row.process_scenario_id.trim().is_empty()
            || row.process_id == 0
            || row.screenshot_outcome_id.trim().is_empty()
            || row.screenshot_scenario_id.trim().is_empty()
        {
            return Err(std::io::Error::other(format!(
                "Argus aggregate surface {:?} has an empty material identity/target field",
                row.surface
            )));
        }
        if row.inspect_author_id != contract.inspect_author_id
            || row.mutation_method != contract.mutation_method
            || row.mutation_target != contract.mutation_target
            || row.reinspect_author_id != contract.reinspect_author_id
            || row.reinspect_author_present != contract.reinspect_author_present
            || row.process_scenario_id != contract.process_scenario
        {
            return Err(std::io::Error::other(format!(
                "Argus aggregate surface {:?} has inspect/method/target/reinspection drift",
                row.surface
            )));
        }
        let expected_client_session_id = format!("mt108-{}", sanitize(&row.surface));
        if row.client_session_id != expected_client_session_id
            || !row
                .agent_id
                .ends_with(&format!(":client:{}", row.client_session_id))
        {
            return Err(std::io::Error::other(format!(
                "Argus aggregate surface {:?} has invalid client attribution",
                row.surface
            )));
        }
        if row.action_seq == 0
            || row.receipt_id == 0
            || !matches!(row.receipt_status.as_str(), "applied" | "indeterminate")
            || row
                .observed_post_state
                .as_object()
                .is_none_or(|state| state.is_empty())
        {
            return Err(std::io::Error::other(format!(
                "Argus aggregate surface {:?} lacks a terminal receipt or concrete post-state",
                row.surface
            )));
        }
        match row.screenshot_status.as_str() {
            "CAPTURED" if row.gpu_screenshot_enabled => {}
            "DEFERRED" if !row.gpu_screenshot_enabled => {}
            "BLOCKED" => {
                return Err(std::io::Error::other(format!(
                    "Argus aggregate surface {:?} is BLOCKED and cannot close proof",
                    row.surface
                )));
            }
            status => {
                return Err(std::io::Error::other(format!(
                    "Argus aggregate surface {:?} has invalid screenshot posture {status:?}",
                    row.surface
                )));
            }
        }
        match gpu_posture {
            Some(previous) if previous != row.gpu_screenshot_enabled => {
                return Err(std::io::Error::other(
                    "Argus aggregate mixes GPU and headless screenshot postures",
                ));
            }
            None => gpu_posture = Some(row.gpu_screenshot_enabled),
            Some(_) => {}
        }
        if !surfaces.insert(row.surface.as_str()) {
            return Err(std::io::Error::other(format!(
                "duplicate Argus aggregate surface {:?}",
                row.surface
            )));
        }
        if !outcomes.insert(row.outcome_id.as_str()) {
            return Err(std::io::Error::other(format!(
                "duplicate Argus aggregate outcome {:?}",
                row.outcome_id
            )));
        }
    }
    if rows.len() > EXACT_ARGUS_SURFACES.len() {
        return Err(std::io::Error::other(format!(
            "Argus aggregate has {} rows; expected at most seven",
            rows.len()
        )));
    }
    if require_exact_seven && (rows.len() != EXACT_ARGUS_SURFACES.len() || surfaces != expected) {
        return Err(std::io::Error::other(format!(
            "Argus aggregate must contain exactly seven unique surfaces; got {:?}",
            surfaces
        )));
    }
    Ok(())
}

fn read_jsonl<T: serde::de::DeserializeOwned>(path: &std::path::Path) -> std::io::Result<Vec<T>> {
    std::fs::read_to_string(path)?
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| serde_json::from_str::<T>(line).map_err(std::io::Error::other))
        .collect()
}

fn validate_screenshot_rows(
    argus_rows: &[ArgusSurfaceEvidence],
    markers: &[screenshot_marker::ScreenshotMarker],
    expected_run_id: &str,
    run_dir: &std::path::Path,
) -> std::io::Result<()> {
    if markers.len() != EXPECTED_ARGUS_CONTRACTS.len() {
        return Err(std::io::Error::other(format!(
            "screenshot closure requires exactly seven marker rows; got {}",
            markers.len()
        )));
    }
    let canonical_run_dir = std::fs::canonicalize(run_dir)?;
    let mut outcomes = std::collections::HashSet::new();
    let mut scenarios = std::collections::HashSet::new();
    for marker in markers {
        if marker.schema_id != screenshot_marker::SCREENSHOT_MARKER_SCHEMA_ID
            || marker.run_id != expected_run_id
            || marker.mt_id != "MT-108"
            || marker.outcome_id.trim().is_empty()
        {
            return Err(std::io::Error::other(
                "screenshot marker has schema, run, MT, or outcome identity drift",
            ));
        }
        if !outcomes.insert(marker.outcome_id.as_str())
            || !scenarios.insert(marker.scenario_id.as_str())
        {
            return Err(std::io::Error::other(
                "screenshot closure contains a duplicate outcome or scenario",
            ));
        }
        let contract = EXPECTED_ARGUS_CONTRACTS
            .iter()
            .find(|contract| marker.scenario_id == format!("runtime:{}", contract.test_name))
            .ok_or_else(|| {
                std::io::Error::other(format!(
                    "unexpected screenshot scenario {:?}",
                    marker.scenario_id
                ))
            })?;
        let argus = argus_rows
            .iter()
            .find(|row| row.surface == contract.surface)
            .expect("exact Argus surface validation ran first");
        if argus.screenshot_outcome_id != marker.outcome_id
            || argus.screenshot_scenario_id != marker.scenario_id
            || argus.screenshot_frame_path != marker.frame_path
            || argus.gpu_screenshot_enabled != marker.gpu_screenshot_enabled
        {
            return Err(std::io::Error::other(format!(
                "surface {:?} does not correlate to its exact screenshot marker",
                argus.surface
            )));
        }
        match marker.status {
            screenshot_marker::ScreenshotStatus::Captured
                if argus.screenshot_status == "CAPTURED" && marker.gpu_screenshot_enabled =>
            {
                let frame = marker.frame_path.as_deref().ok_or_else(|| {
                    std::io::Error::other("CAPTURED screenshot marker has no frame path")
                })?;
                let canonical_frame = std::fs::canonicalize(frame)?;
                if !canonical_frame.starts_with(&canonical_run_dir) {
                    return Err(std::io::Error::other(format!(
                        "captured frame escaped the exact run directory: {}",
                        canonical_frame.display()
                    )));
                }
                let bytes = std::fs::read(&canonical_frame)?;
                let decoded = image::load_from_memory_with_format(&bytes, image::ImageFormat::Png)
                    .map_err(|error| {
                        std::io::Error::other(format!(
                            "captured frame does not decode as PNG at {}: {error}",
                            canonical_frame.display()
                        ))
                    })?;
                if decoded.width() == 0 || decoded.height() == 0 {
                    return Err(std::io::Error::other("captured PNG has zero dimensions"));
                }
            }
            screenshot_marker::ScreenshotStatus::Deferred
                if argus.screenshot_status == "DEFERRED"
                    && !marker.gpu_screenshot_enabled
                    && marker.frame_path.is_none() => {}
            screenshot_marker::ScreenshotStatus::Blocked => {
                return Err(std::io::Error::other(format!(
                    "surface {:?} has a BLOCKED screenshot marker",
                    argus.surface
                )));
            }
            _ => {
                return Err(std::io::Error::other(format!(
                    "surface {:?} has mismatched screenshot status/GPU/frame posture",
                    argus.surface
                )));
            }
        }
    }
    Ok(())
}

fn expected_cargo_arguments(contract: &ExpectedArgusSurface) -> Vec<String> {
    [
        "test",
        "--test",
        contract.test_binary,
        contract.test_name,
        "--",
        "--exact",
        "--nocapture",
    ]
    .into_iter()
    .map(str::to_owned)
    .collect()
}

fn verifier_cargo_arguments() -> Vec<String> {
    [
        "test",
        "--test",
        "test_mt108_argus_aggregate",
        "mt108_verify_argus_evidence_exact_seven",
        "--",
        "--ignored",
        "--exact",
        "--nocapture",
    ]
    .into_iter()
    .map(str::to_owned)
    .collect()
}

fn validate_process_rows(
    rows: &[ExternalProcessReceipt],
    argus_rows: &[ArgusSurfaceEvidence],
    expected_run_id: &str,
) -> std::io::Result<()> {
    let expected_row_count = EXPECTED_ARGUS_CONTRACTS.len() * 2 + 1;
    if rows.len() != expected_row_count {
        return Err(std::io::Error::other(format!(
            "verifier requires fourteen completed surface lifecycle rows plus its own STARTED row; got {}",
            rows.len()
        )));
    }
    let mut outcomes = std::collections::HashSet::new();
    for row in rows {
        if row.schema_id != "hsk.native_gui.external_process_receipt@2"
            || row.run_id != expected_run_id
            || row.outcome_id.trim().is_empty()
            || row.process_correlation_id.trim().is_empty()
            || row.child_pid == 0
            || row.owned_process_tree.is_empty()
            || row.child_started_at_utc.trim().is_empty()
            || row.deadline_at_utc.trim().is_empty()
            || row.deadline_seconds == 0
            || row.command_executable != "cargo"
            || row.command_display.trim().is_empty()
            || !std::path::Path::new(&row.working_directory).is_absolute()
            || !outcomes.insert(row.outcome_id.as_str())
        {
            return Err(std::io::Error::other(
                "external process receipt has schema/run/process/command/lifecycle identity drift",
            ));
        }
        let root_identity = row
            .owned_process_tree
            .iter()
            .find(|identity| identity.pid == row.child_pid)
            .ok_or_else(|| {
                std::io::Error::other(format!(
                    "process receipt {:?} does not include its exact supervised root identity",
                    row.scenario_id
                ))
            })?;
        if root_identity.start_time_utc.trim().is_empty()
            || root_identity.executable.trim().is_empty()
            || root_identity.start_time_utc != row.child_started_at_utc
        {
            return Err(std::io::Error::other(format!(
                "process receipt {:?} root PID/start-time identity does not match child_started_at_utc",
                row.scenario_id
            )));
        }
        if row.status == "COMPLETED"
            && (row.test_process_pid.is_none()
                || row
                    .test_process_start_time_utc
                    .as_deref()
                    .is_none_or(str::is_empty)
                || row
                    .test_process_executable
                    .as_deref()
                    .is_none_or(str::is_empty))
        {
            return Err(std::io::Error::other(format!(
                "completed surface process {:?} lacks exact test executable PID/start-time proof",
                row.scenario_id
            )));
        }
        if let (Some(test_pid), Some(test_start), Some(test_executable)) = (
            row.test_process_pid,
            row.test_process_start_time_utc.as_deref(),
            row.test_process_executable.as_deref(),
        ) {
            if !row.owned_process_tree.iter().any(|identity| {
                identity.pid == test_pid
                    && identity.start_time_utc == test_start
                    && identity.executable == test_executable
            }) {
                return Err(std::io::Error::other(format!(
                    "process receipt {:?} test executable is not an exact PID/start-time member of its owned tree",
                    row.scenario_id
                )));
            }
        }
        if !matches!(row.status.as_str(), "STARTED" | "COMPLETED") {
            return Err(std::io::Error::other(format!(
                "external process status {:?} cannot close proof",
                row.status
            )));
        }
    }

    for contract in &EXPECTED_ARGUS_CONTRACTS {
        validate_completed_process_lifecycle(
            rows,
            contract.process_scenario,
            &expected_cargo_arguments(contract),
        )?;
        let started = rows
            .iter()
            .find(|row| row.scenario_id == contract.process_scenario && row.status == "STARTED")
            .expect("completed lifecycle validation found one STARTED receipt");
        let argus = argus_rows
            .iter()
            .find(|row| row.surface == contract.surface)
            .ok_or_else(|| {
                std::io::Error::other(format!(
                    "process scenario {:?} has no exact Argus surface row",
                    contract.process_scenario
                ))
            })?;
        if argus.process_scenario_id != started.scenario_id
            || argus.process_correlation_id != started.process_correlation_id
        {
            return Err(std::io::Error::other(format!(
                "surface {:?} does not correlate to its exact supervised process scenario/correlation",
                contract.surface
            )));
        }
        let completed = rows
            .iter()
            .find(|row| row.scenario_id == contract.process_scenario && row.status == "COMPLETED")
            .expect("completed lifecycle validation found one COMPLETED receipt");
        let test_pid = completed.test_process_pid.ok_or_else(|| {
            std::io::Error::other(format!(
                "surface {:?} completed lifecycle lacks the test executable PID",
                contract.surface
            ))
        })?;
        if argus.process_id != test_pid
            || !completed.owned_process_tree.iter().any(|identity| {
                identity.pid == argus.process_id
                    && Some(identity.start_time_utc.as_str())
                        == completed.test_process_start_time_utc.as_deref()
            })
        {
            return Err(std::io::Error::other(format!(
                "surface {:?} Argus process PID {} is not the exact supervised test executable identity",
                contract.surface, argus.process_id
            )));
        }
    }

    let verifier = rows
        .iter()
        .filter(|row| row.scenario_id == "exact_seven_verifier")
        .collect::<Vec<_>>();
    if verifier.len() != 1
        || verifier[0].status != "STARTED"
        || verifier[0].exit_code.is_some()
        || verifier[0].command_arguments != verifier_cargo_arguments()
    {
        return Err(std::io::Error::other(
            "running exact-seven verifier must have exactly one correlated STARTED receipt",
        ));
    }
    Ok(())
}

fn validate_completed_process_lifecycle(
    rows: &[ExternalProcessReceipt],
    scenario: &str,
    expected_arguments: &[String],
) -> std::io::Result<()> {
    let lifecycle = rows
        .iter()
        .filter(|row| row.scenario_id == scenario)
        .collect::<Vec<_>>();
    let started = lifecycle
        .iter()
        .filter(|row| row.status == "STARTED")
        .collect::<Vec<_>>();
    let completed = lifecycle
        .iter()
        .filter(|row| row.status == "COMPLETED")
        .collect::<Vec<_>>();
    if lifecycle.len() != 2
        || started.len() != 1
        || completed.len() != 1
        || started[0].process_correlation_id != completed[0].process_correlation_id
        || started[0].child_pid != completed[0].child_pid
        || started[0].exit_code.is_some()
        || completed[0].exit_code != Some(0)
        || started[0].command_arguments != expected_arguments
        || completed[0].command_arguments != expected_arguments
    {
        return Err(std::io::Error::other(format!(
            "scenario {scenario:?} lacks an exact correlated STARTED/COMPLETED zero-exit lifecycle"
        )));
    }
    Ok(())
}

pub fn verify_argus_evidence_exact_seven() -> std::io::Result<()> {
    let expected_run_id = require_explicit_argus_run_id();
    let run_dir = screenshot_marker::marker_dir();
    let argus_rows =
        read_jsonl::<ArgusSurfaceEvidence>(&run_dir.join("argus-seven-surface.jsonl"))?;
    let screenshot_rows = read_jsonl::<screenshot_marker::ScreenshotMarker>(
        &run_dir.join("screenshot_marker.jsonl"),
    )?;
    let process_rows =
        read_jsonl::<ExternalProcessReceipt>(&run_dir.join("external_process_receipts.jsonl"))?;
    validate_argus_rows(&argus_rows, true, Some(&expected_run_id))?;
    validate_screenshot_rows(&argus_rows, &screenshot_rows, &expected_run_id, &run_dir)?;
    validate_process_rows(&process_rows, &argus_rows, &expected_run_id)
}

fn write_evidence(row: &ArgusSurfaceEvidence) -> std::io::Result<()> {
    let dir = screenshot_marker::marker_dir();
    std::fs::create_dir_all(&dir)?;
    let path = dir.join("argus-seven-surface.jsonl");
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .read(true)
        .append(true)
        .open(path)?;
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
    loop {
        match file.try_lock() {
            Ok(()) => break,
            Err(std::fs::TryLockError::WouldBlock) if std::time::Instant::now() < deadline => {
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
            Err(error) => {
                return Err(std::io::Error::other(format!(
                    "lock shared Argus evidence within 2s: {error}"
                )));
            }
        }
    }
    let mut existing = String::new();
    file.read_to_string(&mut existing)?;
    let mut rows = existing
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            serde_json::from_str::<ArgusSurfaceEvidence>(line).map_err(std::io::Error::other)
        })
        .collect::<std::io::Result<Vec<_>>>()?;
    rows.push(row.clone());
    validate_argus_rows(
        &rows,
        rows.len() == EXACT_ARGUS_SURFACES.len(),
        Some(&row.run_id),
    )?;

    let mut bytes = serde_json::to_vec(row).map_err(std::io::Error::other)?;
    bytes.push(b'\n');
    file.write_all(&bytes)?;
    file.sync_all()?;
    Ok(())
}

struct ScopedAppData {
    variable: &'static str,
    previous: Option<std::ffi::OsString>,
    root: std::path::PathBuf,
}

impl ScopedAppData {
    fn install(root: std::path::PathBuf) -> Self {
        std::fs::create_dir_all(&root).expect("create isolated Argus binding root");
        #[cfg(target_os = "windows")]
        let variable = "LOCALAPPDATA";
        #[cfg(not(target_os = "windows"))]
        let variable = "XDG_DATA_HOME";
        let previous = std::env::var_os(variable);
        std::env::set_var(variable, &root);
        Self {
            variable,
            previous,
            root,
        }
    }
}

impl Drop for ScopedAppData {
    fn drop(&mut self) {
        match self.previous.take() {
            Some(value) => std::env::set_var(self.variable, value),
            None => std::env::remove_var(self.variable),
        }
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

fn sanitize(value: &str) -> String {
    value
        .chars()
        .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '-' })
        .collect::<String>()
        .trim_matches('-')
        .to_owned()
}

fn now_nanos() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0)
}

#[cfg(test)]
mod aggregate_tests {
    use super::*;

    const UNIT_RUN_ID: &str = "mt108-exact-seven-unit";

    fn row(surface: &str, index: usize) -> ArgusSurfaceEvidence {
        let contract = EXPECTED_ARGUS_CONTRACTS
            .iter()
            .find(|contract| contract.surface == surface)
            .unwrap();
        ArgusSurfaceEvidence {
            schema_id: "hsk.native_gui.argus_surface_evidence@4".to_owned(),
            run_id: UNIT_RUN_ID.to_owned(),
            outcome_id: format!("outcome-{index}"),
            surface: surface.to_owned(),
            inspect_author_id: contract.inspect_author_id.to_owned(),
            reinspect_author_id: contract.reinspect_author_id.to_owned(),
            mutation_method: contract.mutation_method.to_owned(),
            mutation_target: contract.mutation_target.to_owned(),
            client_session_id: format!("mt108-{}", sanitize(surface)),
            agent_id: format!("argus:client:mt108-{}", sanitize(surface)),
            action_seq: index as u64 + 1,
            receipt_id: index as u64 + 1,
            receipt_status: "applied".to_owned(),
            reinspect_author_present: contract.reinspect_author_present,
            observed_post_state: serde_json::json!({ "index": index }),
            process_correlation_id: format!("correlation-{}", contract.process_scenario),
            process_scenario_id: contract.process_scenario.to_owned(),
            process_id: index as u32 + 100,
            screenshot_outcome_id: format!("screenshot-{index}"),
            screenshot_scenario_id: format!("runtime:{}", contract.test_name),
            screenshot_status: "DEFERRED".to_owned(),
            screenshot_frame_path: None,
            gpu_screenshot_enabled: false,
        }
    }

    fn rows() -> Vec<ArgusSurfaceEvidence> {
        EXACT_ARGUS_SURFACES
            .iter()
            .enumerate()
            .map(|(index, surface)| row(surface, index))
            .collect()
    }

    fn markers(rows: &[ArgusSurfaceEvidence]) -> Vec<screenshot_marker::ScreenshotMarker> {
        rows.iter()
            .map(|row| screenshot_marker::ScreenshotMarker {
                schema_id: screenshot_marker::SCREENSHOT_MARKER_SCHEMA_ID.to_owned(),
                run_id: UNIT_RUN_ID.to_owned(),
                outcome_id: row.screenshot_outcome_id.clone(),
                mt_id: "MT-108".to_owned(),
                scenario_id: row.screenshot_scenario_id.clone(),
                source_sha: None,
                process_correlation_id: None,
                process_scenario_id: None,
                process_id: std::process::id(),
                action_receipt_id: Some(row.receipt_id),
                status: screenshot_marker::ScreenshotStatus::Deferred,
                reason: "unit headless proof".to_owned(),
                frame_path: None,
                gpu_screenshot_enabled: false,
                frame_width: None,
                frame_height: None,
                timestamp_nanos: 1,
            })
            .collect()
    }

    fn process_receipt(
        scenario: &str,
        status: &str,
        arguments: Vec<String>,
        index: usize,
    ) -> ExternalProcessReceipt {
        let cargo_pid = index as u32 + 200;
        let test_pid = index as u32 + 100;
        let cargo_start = format!("2026-07-19T00:00:{index:02}Z");
        let test_start = format!("2026-07-19T00:01:{index:02}Z");
        ExternalProcessReceipt {
            schema_id: "hsk.native_gui.external_process_receipt@2".to_owned(),
            run_id: UNIT_RUN_ID.to_owned(),
            outcome_id: format!("process-{scenario}-{status}-{index}"),
            process_correlation_id: format!("correlation-{scenario}"),
            child_pid: cargo_pid,
            owned_process_tree: vec![
                ProcessIdentity {
                    pid: cargo_pid,
                    parent_pid: 1,
                    start_time_utc: cargo_start.clone(),
                    executable: "cargo".to_owned(),
                },
                ProcessIdentity {
                    pid: test_pid,
                    parent_pid: cargo_pid,
                    start_time_utc: test_start.clone(),
                    executable: format!("test_{}.exe", scenario),
                },
            ],
            test_process_pid: (status == "COMPLETED").then_some(test_pid),
            test_process_start_time_utc: (status == "COMPLETED").then_some(test_start),
            test_process_executable: (status == "COMPLETED")
                .then(|| format!("test_{}.exe", scenario)),
            child_started_at_utc: cargo_start,
            deadline_at_utc: "2026-07-19T00:01:30Z".to_owned(),
            deadline_seconds: 90,
            command_executable: "cargo".to_owned(),
            command_arguments: arguments,
            command_display: "cargo exact arguments".to_owned(),
            working_directory: if cfg!(windows) {
                r"D:\worktree".to_owned()
            } else {
                "/worktree".to_owned()
            },
            scenario_id: scenario.to_owned(),
            status: status.to_owned(),
            exit_code: (status == "COMPLETED").then_some(0),
        }
    }

    fn process_rows() -> Vec<ExternalProcessReceipt> {
        let mut rows = Vec::new();
        for (index, contract) in EXPECTED_ARGUS_CONTRACTS.iter().enumerate() {
            let arguments = expected_cargo_arguments(contract);
            let mut started = process_receipt(
                contract.process_scenario,
                "STARTED",
                arguments.clone(),
                index,
            );
            started.exit_code = None;
            let mut completed =
                process_receipt(contract.process_scenario, "COMPLETED", arguments, index);
            completed.process_correlation_id = started.process_correlation_id.clone();
            completed.child_pid = started.child_pid;
            rows.extend([started, completed]);
        }
        rows.push(process_receipt(
            "exact_seven_verifier",
            "STARTED",
            verifier_cargo_arguments(),
            99,
        ));
        rows
    }

    #[test]
    fn exact_seven_aggregate_accepts_each_required_surface_once() {
        let rows = EXACT_ARGUS_SURFACES
            .iter()
            .enumerate()
            .map(|(index, surface)| row(surface, index))
            .collect::<Vec<_>>();
        validate_argus_rows(&rows, true, Some("mt108-exact-seven-unit"))
            .expect("the exact seven-surface aggregate is accepted");
    }

    #[test]
    fn aggregate_rejects_duplicate_surface_and_incomplete_finalization() {
        let mut duplicate = EXACT_ARGUS_SURFACES
            .iter()
            .enumerate()
            .map(|(index, surface)| row(surface, index))
            .collect::<Vec<_>>();
        duplicate[6].surface = duplicate[0].surface.clone();
        assert!(
            validate_argus_rows(&duplicate, true, Some("mt108-exact-seven-unit")).is_err(),
            "a duplicate surface cannot masquerade as exact-seven proof"
        );

        let mut duplicate_outcome = EXACT_ARGUS_SURFACES
            .iter()
            .enumerate()
            .map(|(index, surface)| row(surface, index))
            .collect::<Vec<_>>();
        duplicate_outcome[6].outcome_id = duplicate_outcome[0].outcome_id.clone();
        assert!(
            validate_argus_rows(&duplicate_outcome, true, Some("mt108-exact-seven-unit")).is_err(),
            "a duplicate outcome cannot masquerade as independent proof"
        );

        let incomplete = EXACT_ARGUS_SURFACES[..6]
            .iter()
            .enumerate()
            .map(|(index, surface)| row(surface, index))
            .collect::<Vec<_>>();
        assert!(
            validate_argus_rows(&incomplete, true, Some("mt108-exact-seven-unit")).is_err(),
            "six unique rows cannot finalize an exact-seven aggregate"
        );
    }

    #[test]
    fn aggregate_rejects_blocked_or_materially_hollow_rows() {
        let baseline = || {
            EXACT_ARGUS_SURFACES
                .iter()
                .enumerate()
                .map(|(index, surface)| row(surface, index))
                .collect::<Vec<_>>()
        };

        let mut blocked = baseline();
        blocked[0].screenshot_status = "BLOCKED".to_owned();
        assert!(validate_argus_rows(&blocked, true, Some("mt108-exact-seven-unit")).is_err());

        let mut false_deferred = baseline();
        false_deferred[0].gpu_screenshot_enabled = true;
        assert!(
            validate_argus_rows(&false_deferred, true, Some("mt108-exact-seven-unit")).is_err()
        );

        let mut rejected = baseline();
        rejected[0].receipt_status = "rejected".to_owned();
        assert!(validate_argus_rows(&rejected, true, Some("mt108-exact-seven-unit")).is_err());

        let mut hollow = baseline();
        hollow[0].observed_post_state = serde_json::json!({});
        assert!(validate_argus_rows(&hollow, true, Some("mt108-exact-seven-unit")).is_err());

        let mut unattributed = baseline();
        unattributed[0].agent_id = "unattributed".to_owned();
        assert!(validate_argus_rows(&unattributed, true, Some("mt108-exact-seven-unit")).is_err());

        let mut wrong_method = baseline();
        wrong_method[0].mutation_method = "argus.set_value".to_owned();
        assert!(validate_argus_rows(&wrong_method, true, Some("mt108-exact-seven-unit")).is_err());

        let mut wrong_target = baseline();
        wrong_target[0].mutation_target = "different-target".to_owned();
        assert!(validate_argus_rows(&wrong_target, true, Some("mt108-exact-seven-unit")).is_err());

        let mut wrong_inspect = baseline();
        wrong_inspect[0].inspect_author_id = "different-inspect-root".to_owned();
        assert!(validate_argus_rows(&wrong_inspect, true, Some("mt108-exact-seven-unit")).is_err());

        let mut wrong_reinspect = baseline();
        wrong_reinspect[0].reinspect_author_id = "different-reinspect-root".to_owned();
        assert!(
            validate_argus_rows(&wrong_reinspect, true, Some("mt108-exact-seven-unit")).is_err()
        );
    }

    #[test]
    fn material_closure_correlates_exact_screenshot_markers() {
        let mut rows = rows();
        let mut markers = markers(&rows);
        let run_dir = std::env::temp_dir().join(format!(
            "hsk-mt108-material-closure-pid{}-{}",
            std::process::id(),
            now_nanos()
        ));
        std::fs::create_dir_all(&run_dir).unwrap();
        validate_screenshot_rows(&rows, &markers, UNIT_RUN_ID, &run_dir)
            .expect("exact correlated headless marker set closes screenshot material proof");

        let frame = run_dir.join("captured.png");
        image::RgbaImage::new(1, 1).save(&frame).unwrap();
        rows[0].screenshot_status = "CAPTURED".to_owned();
        rows[0].screenshot_frame_path = Some(frame.display().to_string());
        rows[0].gpu_screenshot_enabled = true;
        markers[0].status = screenshot_marker::ScreenshotStatus::Captured;
        markers[0].frame_path = Some(frame.display().to_string());
        markers[0].gpu_screenshot_enabled = true;
        validate_screenshot_rows(&rows, &markers, UNIT_RUN_ID, &run_dir)
            .expect("a captured frame is reopened and decoded during material closure");
        std::fs::write(&frame, b"not-a-png").unwrap();
        assert!(validate_screenshot_rows(&rows, &markers, UNIT_RUN_ID, &run_dir).is_err());

        rows[0].screenshot_status = "DEFERRED".to_owned();
        rows[0].screenshot_frame_path = None;
        rows[0].gpu_screenshot_enabled = false;
        markers[0].status = screenshot_marker::ScreenshotStatus::Deferred;
        markers[0].frame_path = None;
        markers[0].gpu_screenshot_enabled = false;

        let mut blocked = markers.clone();
        blocked[0].status = screenshot_marker::ScreenshotStatus::Blocked;
        assert!(validate_screenshot_rows(&rows, &blocked, UNIT_RUN_ID, &run_dir).is_err());

        let mut wrong_outcome = markers.clone();
        wrong_outcome[0].outcome_id = "wrong-outcome".to_owned();
        assert!(validate_screenshot_rows(&rows, &wrong_outcome, UNIT_RUN_ID, &run_dir).is_err());

        assert!(validate_screenshot_rows(&rows, &markers[..6], UNIT_RUN_ID, &run_dir).is_err());
        std::fs::remove_dir_all(run_dir).unwrap();
    }

    #[test]
    fn material_closure_requires_exact_successful_surface_process_lifecycles() {
        let argus = rows();
        let rows = process_rows();
        validate_process_rows(&rows, &argus, UNIT_RUN_ID)
            .expect("seven successful surface lifecycles plus verifier STARTED are exact");

        let mut failed = rows.clone();
        failed[1].status = "FAILED".to_owned();
        failed[1].exit_code = Some(1);
        assert!(validate_process_rows(&failed, &argus, UNIT_RUN_ID).is_err());

        let mut wrong_command = rows.clone();
        wrong_command[0]
            .command_arguments
            .push("--wrong".to_owned());
        assert!(validate_process_rows(&wrong_command, &argus, UNIT_RUN_ID).is_err());

        assert!(validate_process_rows(&rows[..rows.len() - 1], &argus, UNIT_RUN_ID).is_err());

        let mut wrong_process = argus.clone();
        wrong_process[0].process_id += 1;
        assert!(validate_process_rows(&rows, &wrong_process, UNIT_RUN_ID).is_err());

        let mut forged_test_identity = rows.clone();
        forged_test_identity[1].test_process_start_time_utc =
            Some("2026-07-19T09:59:59Z".to_owned());
        assert!(
            validate_process_rows(&forged_test_identity, &argus, UNIT_RUN_ID).is_err(),
            "a test PID with a mismatched start-time identity cannot close process proof"
        );

        let mut root_only = rows.clone();
        root_only[1].owned_process_tree.truncate(1);
        assert!(
            validate_process_rows(&root_only, &argus, UNIT_RUN_ID).is_err(),
            "a completed receipt without the exact test executable tree member cannot close proof"
        );
    }

    #[test]
    fn runner_inventory_errors_cannot_close_reclamation() {
        let runner = include_str!("../run_mt108_argus_proof.ps1");
        assert!(runner.contains("function Get-LiveOwnedProcessInventory"));
        assert!(runner.contains("InventoryHealthy = $errors.Count -eq 0"));
        assert!(runner.contains("process-tree reclamation inventory was indeterminate"));
        assert!(runner.contains("PROCESS_TREE_RECLAIM_FAILED"));
        assert!(runner.contains("$rootIdentity = Get-ProcessIdentityByPid -TargetPid $process.Id"));
        assert!(runner.contains("-ExpectedRootStartUtc $context.StartedAtUtc"));
        assert!(runner.contains("$process.WaitForExit(25)"));
        assert!(runner.contains("$correlationId.exit-code"));
        assert!(runner.contains("wrapper-owned Cargo exit-code sidecar"));
        assert!(runner.contains("[int]::TryParse("));
        assert!(runner.contains("$childStartedAt -lt $parentStartedAt"));
        assert!(runner.contains("Chronologically impossible"));
        assert!(runner.contains("ProcessInventoryErrors = @()"));
        assert!(runner.contains("owned process-tree capture was indeterminate"));
        assert!(runner.contains("$errors = @($ProcessContext.ProcessInventoryErrors)"));
        assert!(runner.contains("Capture one final process-table snapshot immediately after exit"));
        assert!(runner.contains("identity changed: expected start"));
        assert!(!runner.contains("ParentPid = 0"));
    }
}
