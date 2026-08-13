//! Ignored production-binary Argus socket proof.
//!
//! This target is intentionally outside the default suite. It opens real native windows and requires
//! managed PostgreSQL plus a Palmistry-ready `handshake_core` on `127.0.0.1:37501`. Before running,
//! set `HANDSHAKE_ARGUS_LIVE_BACKEND_READY=1` and point `HANDSHAKE_DIAGNOSTICS_DIR` at the existing
//! absolute directory shared by the backend, Palmistry, and the native child.
//!
//! The spawn/discovery/receipt/redaction plumbing lives in the shared
//! `argus_socket_support/live_socket.rs` module so this proof and
//! `test_argus_production_socket_live_surfaces.rs` drive the production transport through ONE
//! implementation instead of drifting copies. The assertions below are unchanged.

#![cfg(target_os = "windows")]

use std::process::Command;
use std::time::{Duration, Instant};

use base64::Engine as _;
use sha2::{Digest, Sha256};

use handshake_native::pane_registry::PaneType;
use handshake_native::swarm_lane_diagnostics::{scoped_author_id, SURFACE_AUTHOR_ID};

#[path = "argus_socket_support/live_socket.rs"]
mod live_socket;

use live_socket::{
    assert_success, assert_visual_png, contains_author_id, discover_binding, list_has_window,
    pane_id_hosting, proof_dir, request_child_close, require_palmistry_ready_backend,
    wait_for_author_id_between, wait_for_window, ArgusClient, ChildGuard, LiveApp, SURFACE_TIMEOUT,
};

const PRIVACY_OWNER_AUTHOR_ID: &str = "swarm-lane-diagnostics.privacy.owner-account";
const PRIVACY_PRINCIPAL_AUTHOR_ID: &str = "swarm-lane-diagnostics.privacy.actor-principal";
const PRIVACY_SESSION_AUTHOR_ID: &str = "swarm-lane-diagnostics.privacy.authenticated-session";
const PRIVACY_ACCESS_SPACE_AUTHOR_ID: &str = "swarm-lane-diagnostics.privacy.access-space";
const PRIVACY_WORKSPACE_AUTHOR_ID: &str = "swarm-lane-diagnostics.privacy.workspace";

#[ignore = "LIVE production socket MT-008 E2E: opens a native diagnostics pane/pop-out and requires \
            managed PostgreSQL, Palmistry-ready handshake_core on 127.0.0.1:37501, \
            HANDSHAKE_ARGUS_LIVE_BACKEND_READY=1, HANDSHAKE_MT008_ARGUS_PROOF_NONCE, and a shared \
            HANDSHAKE_DIAGNOSTICS_DIR"]
#[test]
fn mt008_production_socket_diagnostics_scope_and_detached_capture() {
    let proof_nonce = std::env::var("HANDSHAKE_MT008_ARGUS_PROOF_NONCE")
        .expect("MT-008 live proof requires a fresh HANDSHAKE_MT008_ARGUS_PROOF_NONCE");
    assert!(
        !proof_nonce.trim().is_empty(),
        "MT-008 live proof nonce must not be blank"
    );

    let mut app = LiveApp::start("mt008_diagnostics");
    app.open_models_menu_leaf("menu.models.swarm-lane-diagnostics");
    let discovered_surface = wait_for_author_id_between(
        &mut app.client,
        "main",
        &format!("{SURFACE_AUTHOR_ID}.pane."),
        "",
        SURFACE_TIMEOUT,
    );
    let opened = app.client.inspect("main");
    let pane_id = pane_id_hosting(
        &opened["snapshot"]["root"],
        &PaneType::SwarmLaneDiagnostics.label(),
    );
    assert_eq!(
        discovered_surface,
        scoped_author_id(&pane_id, SURFACE_AUTHOR_ID),
        "the live diagnostics surface must belong to its actual pane"
    );

    for author_id in [
        PRIVACY_OWNER_AUTHOR_ID,
        PRIVACY_PRINCIPAL_AUTHOR_ID,
        PRIVACY_SESSION_AUTHOR_ID,
        PRIVACY_ACCESS_SPACE_AUTHOR_ID,
        PRIVACY_WORKSPACE_AUTHOR_ID,
    ] {
        let scoped = scoped_author_id(&pane_id, author_id);
        assert!(
            contains_author_id(&opened["snapshot"]["root"], &scoped),
            "live diagnostics omitted exact server-owned privacy landmark {scoped}"
        );
    }

    // The remainder of the live detached-window/artifact proof is intentionally added only after
    // this privacy-boundary RED is attributed. Keeping the first failure here makes it impossible
    // for generic pane/pop-out pixels to masquerade as MT-008 diagnostics evidence.
    drop(app);
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
        agent_id: None,
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
    let secret_canary = "production-socket-secret-canary";
    let settings_landmarks = [
        "settings.dialog",
        "settings.cloud.byok.openai.key",
        "settings.cloud.byok.openai.status",
        "settings.cloud.byok.openai.save",
        "settings.cloud.byok.anthropic.key",
        "settings.cloud.byok.anthropic.status",
        "settings.cloud.cli.claude_code.status",
        "settings.cloud.cli.codex.status",
    ];
    client.mutation("argus.click", "main", "menu-help", None);
    let help_menu = client.inspect("main");
    assert!(
        contains_author_id(&help_menu["snapshot"]["root"], "menu.help.settings"),
        "HELP menu did not expose Open Settings"
    );
    client.mutation("argus.click", "main", "menu.help.settings", None);
    let settings = client.inspect("main");
    for author_id in settings_landmarks {
        assert!(
            contains_author_id(&settings["snapshot"]["root"], author_id),
            "production Settings snapshot omitted {author_id}"
        );
    }
    let settings_revision = settings["revision"]
        .as_u64()
        .expect("Settings inspect revision is numeric");
    let secret_denial = client.call(
        "argus.set_value",
        serde_json::json!({
            "window_id": "main",
            "author_id": "settings.cloud.byok.openai.key",
            "expected_snapshot_revision": settings_revision,
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
    let settings_after_denial = client.inspect("main");
    assert_eq!(
        settings_after_denial["revision"].as_u64(),
        Some(settings_revision),
        "denied secret input unexpectedly advanced the Settings revision"
    );
    for author_id in settings_landmarks {
        assert!(
            contains_author_id(&settings_after_denial["snapshot"]["root"], author_id),
            "Settings landmark disappeared before visual capture: {author_id}"
        );
    }
    let settings_json = serde_json::to_string(&settings_after_denial["snapshot"]["root"])
        .expect("serialize Settings tree");
    assert!(
        !settings_json.contains(secret_canary),
        "Settings snapshot disclosed the BYOK canary"
    );

    // Bracket the targeted capture with canonical Settings snapshots. This proves the live main-window
    // PNG was captured while the Settings/cloud controls were rendered, not from an earlier frame.
    let settings_shot = client.call("argus.screenshot", serde_json::json!({"window_id": "main"}));
    assert_success(&settings_shot, "argus.screenshot(main Settings-open)");
    assert_eq!(settings_shot["result"]["window_id"], "main");
    assert_eq!(settings_shot["result"]["pid"], child_pid);
    assert!(
        settings_shot["result"]["width"]
            .as_u64()
            .is_some_and(|value| value > 0)
            && settings_shot["result"]["height"]
                .as_u64()
                .is_some_and(|value| value > 0),
        "Settings-open capture had zero dimensions"
    );
    assert!(
        !settings_shot.to_string().contains(secret_canary),
        "Settings-open screenshot response disclosed the BYOK canary"
    );
    let settings_png = base64::engine::general_purpose::STANDARD
        .decode(
            settings_shot["result"]["png_base64"]
                .as_str()
                .expect("Settings screenshot png_base64"),
        )
        .expect("decode Settings screenshot PNG");
    assert!(
        settings_png.starts_with(b"\x89PNG\r\n\x1a\n"),
        "Settings-open capture is not PNG"
    );
    assert_eq!(
        settings_shot["result"]["sha256"],
        format!("{:x}", Sha256::digest(&settings_png))
    );
    assert_visual_png(&settings_png, "Settings-open main-window capture");
    assert!(
        !settings_png
            .windows(secret_canary.len())
            .any(|window| window == secret_canary.as_bytes()),
        "Settings-open PNG bytes disclosed the BYOK canary"
    );
    let settings_after_capture = client.inspect("main");
    for author_id in settings_landmarks {
        assert!(
            contains_author_id(&settings_after_capture["snapshot"]["root"], author_id),
            "Settings landmark was not present after visual capture: {author_id}"
        );
    }
    assert!(
        !serde_json::to_string(&settings_after_capture["snapshot"]["root"])
            .expect("serialize post-capture Settings tree")
            .contains(secret_canary),
        "post-capture Settings snapshot disclosed the BYOK canary"
    );
    client.mutation("argus.click", "main", "settings.close", None);

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
        proof_dir.join("argus_production_socket_settings_open.png"),
        &settings_png,
    )
    .expect("write production Settings-open screenshot proof");
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
        &transcript,
    )
    .expect("write production socket transcript");
    let provenance = serde_json::json!({
        "schema_id": "handshake.argus.production_socket_provenance@1",
        "child_pid": child_pid,
        "authenticated_agent_id": authenticated_agent_id,
        "transcript": "argus_production_socket_transcript.json",
        "captures": [
            {
                "artifact": "argus_production_socket_main.png",
                "purpose": "main-window-before-settings",
                "window_id": screenshot["result"]["window_id"],
                "pid": screenshot["result"]["pid"],
                "width": screenshot["result"]["width"],
                "height": screenshot["result"]["height"],
                "captured_at_utc": screenshot["result"]["captured_at_utc"],
                "sha256": screenshot["result"]["sha256"],
            },
            {
                "artifact": "argus_production_socket_settings_open.png",
                "purpose": "main-window-settings-cloud-controls-open",
                "window_id": settings_shot["result"]["window_id"],
                "pid": settings_shot["result"]["pid"],
                "width": settings_shot["result"]["width"],
                "height": settings_shot["result"]["height"],
                "captured_at_utc": settings_shot["result"]["captured_at_utc"],
                "sha256": settings_shot["result"]["sha256"],
                "snapshot_revision_before_capture": settings_after_denial["revision"],
                "snapshot_revision_after_capture": settings_after_capture["revision"],
                "required_author_id_landmarks": settings_landmarks,
                "landmarks_present_before_and_after_capture": true,
                "sensitive_values_redacted": true,
            },
            {
                "artifact": "argus_production_socket_popout_pane_a.png",
                "purpose": "detached-window-targeting-and-capture",
                "window_id": popout_shot["result"]["window_id"],
                "pid": popout_shot["result"]["pid"],
                "width": popout_shot["result"]["width"],
                "height": popout_shot["result"]["height"],
                "captured_at_utc": popout_shot["result"]["captured_at_utc"],
                "sha256": popout_shot["result"]["sha256"],
            }
        ],
        "redaction": {
            "session_token_absent_from_transcript": true,
            "agent_token_absent_from_transcript": true,
            "sensitive_canary_absent_from_transcript_and_settings_capture": true,
        }
    });
    let provenance =
        serde_json::to_vec_pretty(&provenance).expect("serialize production proof provenance");
    let provenance_text = String::from_utf8_lossy(&provenance);
    assert!(
        !provenance_text.contains(client.token.as_str())
            && client
                .agent_token
                .as_deref()
                .is_none_or(|agent_token| !provenance_text.contains(agent_token))
            && !provenance_text.contains(secret_canary),
        "production proof provenance retained a secret"
    );
    std::fs::write(
        proof_dir.join("argus_production_socket_provenance.json"),
        provenance,
    )
    .expect("write production screenshot provenance");

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
