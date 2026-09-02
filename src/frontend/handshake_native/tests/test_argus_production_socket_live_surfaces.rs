//! LIVE production-socket Argus proofs for the remaining WP-1 operator surfaces.
//!
//! Companion to `test_argus_production_socket_live.rs` (diagnostics pane + pane pop-out + main-window
//! Settings). This target covers the surfaces V4 failed for driving an in-process `egui_kittest`
//! Harness instead of the production transport:
//!
//! - **MT-014 ModelRuntime panel** — navigate through the real MODELS menu, inspect every Master-Spec
//!   row control/telemetry target, drive the refresh action with an applied/durable before-after
//!   receipt, assert honest empty/unavailable/dormant negative states, screenshot the main window, and
//!   then detach the pane into a real second OS window and inspect + capture + steer it there.
//! - **MT-012 Operator Chat** — inspect the picker/prompt/launch controls, set the prompt through
//!   `argus.set_value`, select live governed-session and no-OS subagent inventory rows, launch through
//!   the real backend, render its captured transcript, and prove docked/detached state continuity.
//! - **MT-015 Settings cloud access (MAIN window)** — reach Settings through the MODELS menu, assert
//!   the BYOK secret boundary structurally (no value, no `SetValue` action, refused writes) and prove
//!   the canary never appears in `list_widgets` / `argus.inspect` / `argus.screenshot` responses, plus
//!   login-state rows and unavailable-provider states.
//!
//! Every proof here drives the REAL `SwarmMcpServer` socket of a spawned production binary. There is
//! no in-process harness and no transport mock. Each test is `#[ignore]`d behind the same environment
//! gate as the existing live proof, because it opens real native windows and needs the injected
//! embedded SurrealDB plus a Palmistry-ready `handshake_core` on `127.0.0.1:37501`.
//!
//! Run serially (`-- --ignored --test-threads=1`): each test spawns its own production window.

#![cfg(target_os = "windows")]

use handshake_native::console_stream_pane::{
    CONSOLE_STREAM_STATUS_AUTHOR_ID, FILTER_ALL_AUTHOR_ID,
};
use handshake_native::model_runtime_panel::{
    empty_author_id, error_author_id, refresh_author_id, row_action_author_id,
    row_active_selection_author_id, row_adapter_author_id, row_artifact_path_author_id,
    row_audit_author_id, row_author_id, row_dormant_reason_author_id,
    row_engine_internals_author_id, row_engine_internals_expand_author_id, row_kv_cache_author_id,
    row_last_call_age_author_id, row_last_call_author_id, row_ledger_link_author_id,
    row_live_model_author_id, row_locator_author_id, row_lora_author_id, row_revision_author_id,
    row_role_author_id, row_sha_author_id, row_state_author_id, row_steering_author_id,
    row_switch_author_id, row_tokens_per_second_author_id, row_vram_author_id, status_author_id,
    surface_author_id, AUTHOR_ID_PREFIX,
};
use handshake_native::operator_chat_pane::{
    ERROR_AUTHOR_ID, LAUNCH_AUTHOR_ID, LAUNCH_STATUS_AUTHOR_ID, MODEL_PICKER_AUTHOR_ID,
    PROMPT_INPUT_AUTHOR_ID, REFRESH_MODELS_AUTHOR_ID, ROUTING_AUTHORITY_AUTHOR_ID,
    ROUTING_CANCEL_AUTHOR_ID, ROUTING_LIFECYCLE_AUTHOR_ID, ROUTING_RECOVER_AUTHOR_ID,
    ROUTING_REQUEST_AUTHOR_ID, SURFACE_AUTHOR_ID, TRANSCRIPT_AUTHOR_ID,
    WORKTREE_SELECTION_AUTHOR_ID,
};
use handshake_native::pane_registry::PaneType;
use handshake_native::popout_window::popout_window_author_id;
use handshake_native::settings_dialog::{
    cloud_byok_key_author_id, cloud_byok_remove_author_id, cloud_byok_save_author_id,
    cloud_byok_status_author_id, cloud_cli_login_author_id, cloud_cli_status_author_id,
    cloud_consent_posture_author_id, swarm_model_session_option_author_id, CLOSE_AUTHOR_ID,
    CLOUD_CONSENT_STATUS_AUTHOR_ID, SETTINGS_DIALOG_AUTHOR_ID, SETTINGS_POPOUT_AUTHOR_ID,
    SETTINGS_REDOCK_AUTHOR_ID, SETTINGS_SEARCH_AUTHOR_ID, SWARM_MODEL_SESSIONS_COMBO_AUTHOR_ID,
    SWARM_MODEL_SESSIONS_STATUS_AUTHOR_ID,
};

#[path = "argus_socket_support/live_socket.rs"]
mod live_socket;

use live_socket::{
    assert_bytes_exclude, assert_not_applied, collect_author_ids, contains_author_id,
    decode_verified_capture, node_by_author_id, node_is_disabled, node_supports, node_text,
    pane_id_hosting, require_node, wait_for_author_id, wait_for_author_id_between, wait_for_window,
    LiveApp, SURFACE_TIMEOUT,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{GetForegroundWindow, GetWindowThreadProcessId};

/// Every status string the production ModelRuntime panel can render. A live proof must recognise the
/// state it observed instead of accepting any text, so a blank/placeholder status fails.
const HONEST_REGISTRY_STATUS: &[&str] = &[
    "No registry projection loaded.",
    "Loading durable model registry",
    "Refreshing durable model registry",
    "Refreshing stale registry snapshot",
    "STALE registry snapshot",
];

/// The exact login states the Settings CLI-bridge rows may report (`CloudCliAuthStatus::label`).
const HONEST_CLI_LOGIN_STATES: &[&str] = &[
    "Logged in",
    "Logged out",
    "Session expired",
    "Status unavailable",
];

/// The exact BYOK configuration states the Settings rows may report.
const HONEST_BYOK_STATES: &[&str] = &[
    "Configured — key stored in the OS keychain",
    "Status unknown — backend not reachable",
    "Not configured",
];

fn assert_honest_registry_status(label: &str) {
    let counted = label.contains("live |") && label.contains("dormant |");
    assert!(
        counted || HONEST_REGISTRY_STATUS.iter().any(|s| label.contains(s)),
        "ModelRuntime status is not one of the production states: `{label}`"
    );
}

fn registry_status_is_in_flight(status: &str) -> bool {
    status.contains("Loading durable model registry") || status.starts_with("Refreshing")
}

/// Poll the live ModelRuntime panel until it has settled out of an in-flight fetch, so a proof
/// asserts against a real terminal state (rows, an empty registry, or a stated error) and never
/// against a transient loading frame.
fn wait_for_settled_registry(
    app: &mut LiveApp,
    window_id: &str,
    pane_id: &str,
) -> serde_json::Value {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(45);
    let mut last = serde_json::Value::Null;
    while std::time::Instant::now() < deadline {
        last = app.client.poll_inspect(window_id);
        let status = node_text(require_node(
            &last["snapshot"]["root"],
            &status_author_id(pane_id),
        ));
        if !registry_status_is_in_flight(&status) {
            // Re-read through the recorded path so the asserted observation is in the transcript,
            // keeping the polled one if the panel started another fetch in between.
            let recorded = app.client.inspect(window_id);
            let recorded_status = node_text(require_node(
                &recorded["snapshot"]["root"],
                &status_author_id(pane_id),
            ));
            return if registry_status_is_in_flight(&recorded_status) {
                last
            } else {
                recorded
            };
        }
        std::thread::sleep(std::time::Duration::from_millis(200));
    }
    panic!(
        "the live ModelRuntime registry never settled in window `{window_id}`; last status: `{}`",
        node_text(require_node(
            &last["snapshot"]["root"],
            &status_author_id(pane_id)
        ))
    );
}

fn foreground_process_id() -> u32 {
    let window = unsafe { GetForegroundWindow() };
    if window.is_null() {
        return 0;
    }
    let mut pid = 0;
    unsafe {
        GetWindowThreadProcessId(window, &mut pid);
    }
    pid
}

fn wait_for_enabled_author_prefix(
    app: &mut LiveApp,
    window_id: &str,
    prefix: &str,
) -> (String, serde_json::Value) {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(45);
    let mut last = serde_json::Value::Null;
    while std::time::Instant::now() < deadline {
        last = app.client.poll_inspect(window_id);
        let root = &last["snapshot"]["root"];
        let mut candidates = collect_author_ids(root)
            .into_iter()
            .filter(|author_id| author_id.starts_with(prefix))
            .collect::<Vec<_>>();
        candidates.sort();
        if let Some(author_id) = candidates.into_iter().find(|author_id| {
            node_by_author_id(root, author_id).is_some_and(|node| !node_is_disabled(node))
        }) {
            let recorded = app.client.inspect(window_id);
            if node_by_author_id(&recorded["snapshot"]["root"], &author_id)
                .is_some_and(|node| !node_is_disabled(node))
            {
                return (author_id, recorded);
            }
        }
        std::thread::sleep(std::time::Duration::from_millis(200));
    }
    panic!(
        "no enabled author_id with prefix `{prefix}` appeared in `{window_id}`; last ids: {:?}",
        collect_author_ids(&last["snapshot"]["root"])
    );
}

fn wait_for_operator_chat_launch_enabled(app: &mut LiveApp) -> serde_json::Value {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(45);
    let mut last = serde_json::Value::Null;
    while std::time::Instant::now() < deadline {
        last = app.client.poll_inspect("main");
        if node_by_author_id(&last["snapshot"]["root"], LAUNCH_AUTHOR_ID)
            .is_some_and(|node| !node_is_disabled(node))
        {
            let recorded = app.client.inspect("main");
            if node_by_author_id(&recorded["snapshot"]["root"], LAUNCH_AUTHOR_ID)
                .is_some_and(|node| !node_is_disabled(node))
            {
                return recorded;
            }
        }
        std::thread::sleep(std::time::Duration::from_millis(200));
    }
    panic!(
        "Operator Chat launch never became enabled; last surface: {}",
        serde_json::to_string(&last["snapshot"]["root"]).unwrap_or_default()
    );
}

fn wait_for_operator_chat_transcript(app: &mut LiveApp) -> serde_json::Value {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(60);
    let mut last = serde_json::Value::Null;
    while std::time::Instant::now() < deadline {
        last = app.client.poll_inspect("main");
        let root = &last["snapshot"]["root"];
        if let Some(error) = node_by_author_id(root, ERROR_AUTHOR_ID) {
            panic!(
                "real Operator Chat launch failed closed: {}",
                node_text(error)
            );
        }
        let has_transcript = collect_author_ids(root).iter().any(|author_id| {
            author_id.starts_with("operator-chat.transcript.message.")
                || author_id.starts_with("operator-chat.transcript.row.")
        });
        if contains_author_id(root, LAUNCH_STATUS_AUTHOR_ID) && has_transcript {
            let recorded = app.client.inspect("main");
            let recorded_root = &recorded["snapshot"]["root"];
            if contains_author_id(recorded_root, LAUNCH_STATUS_AUTHOR_ID)
                && collect_author_ids(recorded_root).iter().any(|author_id| {
                    author_id.starts_with("operator-chat.transcript.message.")
                        || author_id.starts_with("operator-chat.transcript.row.")
                })
            {
                return recorded;
            }
        }
        std::thread::sleep(std::time::Duration::from_millis(200));
    }
    panic!(
        "real Operator Chat launch produced no transcript; last surface: {}",
        serde_json::to_string(&last["snapshot"]["root"]).unwrap_or_default()
    );
}

fn is_lower_hex_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

/// Every registry row target the WP-1 MT-014 contract requires the panel to expose, asserted against
/// a LIVE snapshot taken over the production socket. Returns the artifact hashes it verified.
///
/// Also enforces the honest-negative contract: a row is either LIVE with a live model id or DORMANT
/// with a stated reason (never both, never neither), an unavailable telemetry field must disclose a
/// reason, and a disabled action must say why it is disabled.
fn assert_registry_rows_are_honest(
    root: &serde_json::Value,
    pane_id: &str,
    context: &str,
) -> Vec<String> {
    for author_id in [
        surface_author_id(pane_id),
        status_author_id(pane_id),
        refresh_author_id(pane_id),
    ] {
        assert!(
            contains_author_id(root, &author_id),
            "{context}: ModelRuntime landmark {author_id} is missing from the live snapshot"
        );
    }
    assert_honest_registry_status(&node_text(require_node(root, &status_author_id(pane_id))));

    let row_prefix = format!("{AUTHOR_ID_PREFIX}.{pane_id}.row.");
    let mut artifacts = collect_author_ids(root)
        .into_iter()
        .filter_map(|author_id| {
            author_id
                .strip_prefix(&row_prefix)
                .filter(|rest| is_lower_hex_sha256(rest))
                .map(str::to_owned)
        })
        .collect::<Vec<_>>();
    artifacts.sort();
    artifacts.dedup();

    if artifacts.is_empty() {
        // Honest empty/unavailable state: the panel must SAY it has nothing (or why), never render
        // an empty body that reads like a healthy registry.
        let status = node_text(require_node(root, &status_author_id(pane_id)));
        assert!(
            contains_author_id(root, &empty_author_id(pane_id))
                || contains_author_id(root, &error_author_id(pane_id))
                || status.contains("No registry projection loaded."),
            "{context}: a ModelRuntime registry with no rows must state its empty, unloaded or error \
             reason; observed status `{status}`"
        );
        if let Some(error) = node_by_author_id(root, &error_author_id(pane_id)) {
            assert!(
                !node_text(error).trim().is_empty(),
                "{context}: the ModelRuntime error state must state a reason"
            );
        }
        return artifacts;
    }

    for artifact in &artifacts {
        for author_id in [
            row_author_id(pane_id, artifact),
            row_state_author_id(pane_id, artifact),
            row_adapter_author_id(pane_id, artifact),
            row_role_author_id(pane_id, artifact),
            row_revision_author_id(pane_id, artifact),
            row_sha_author_id(pane_id, artifact),
            row_locator_author_id(pane_id, artifact),
            row_artifact_path_author_id(pane_id, artifact),
            row_kv_cache_author_id(pane_id, artifact),
            row_lora_author_id(pane_id, artifact),
            row_steering_author_id(pane_id, artifact),
            row_tokens_per_second_author_id(pane_id, artifact),
            row_vram_author_id(pane_id, artifact),
            row_last_call_author_id(pane_id, artifact),
            row_last_call_age_author_id(pane_id, artifact),
            row_engine_internals_author_id(pane_id, artifact),
            row_ledger_link_author_id(pane_id, artifact),
            row_audit_author_id(pane_id, artifact),
            row_action_author_id(pane_id, artifact, "quiesce"),
            row_action_author_id(pane_id, artifact, "unload"),
            row_action_author_id(pane_id, artifact, "adapter-swap"),
            row_action_author_id(pane_id, artifact, "inspect-internals"),
        ] {
            assert!(
                contains_author_id(root, &author_id),
                "{context}: required ModelRuntime target {author_id} is missing"
            );
        }

        // A row is LIVE with a concrete runtime id, or DORMANT with a stated reason — never both.
        let live = contains_author_id(root, &row_live_model_author_id(pane_id, artifact));
        let dormant = contains_author_id(root, &row_dormant_reason_author_id(pane_id, artifact));
        assert!(
            live ^ dormant,
            "{context}: row {artifact} must be exactly one of live ({live}) or dormant ({dormant})"
        );
        if !live {
            assert!(
                !contains_author_id(root, &row_switch_author_id(pane_id, artifact)),
                "{context}: a dormant row must not offer a default-model switch"
            );
        }
        if contains_author_id(root, &row_active_selection_author_id(pane_id, artifact)) {
            assert!(
                !contains_author_id(root, &row_switch_author_id(pane_id, artifact)),
                "{context}: the active default row must not also offer a switch to itself"
            );
        }

        // Every telemetry field renders text, and an unavailable one discloses WHY.
        for author_id in [
            row_artifact_path_author_id(pane_id, artifact),
            row_kv_cache_author_id(pane_id, artifact),
            row_lora_author_id(pane_id, artifact),
            row_steering_author_id(pane_id, artifact),
            row_tokens_per_second_author_id(pane_id, artifact),
            row_vram_author_id(pane_id, artifact),
            row_last_call_author_id(pane_id, artifact),
            row_last_call_age_author_id(pane_id, artifact),
            row_engine_internals_author_id(pane_id, artifact),
            row_ledger_link_author_id(pane_id, artifact),
        ] {
            let label = node_text(require_node(root, &author_id));
            assert!(
                !label.trim().is_empty(),
                "{context}: {author_id} rendered no operator-readable text"
            );
            if label.contains("unavailable") {
                let reason = label
                    .split_once("unavailable (")
                    .map(|(_, rest)| rest.trim_end_matches(')').trim())
                    .unwrap_or_default();
                assert!(
                    !reason.is_empty(),
                    "{context}: {author_id} claims unavailable without a reason: `{label}`"
                );
            }
        }

        // A disabled runtime action must disclose why it is disabled (never a silent dead control).
        for action in ["quiesce", "unload", "adapter-swap", "inspect-internals"] {
            let author_id = row_action_author_id(pane_id, artifact, action);
            let node = require_node(root, &author_id);
            if node_is_disabled(node) {
                let label = node_text(node);
                assert!(
                    label.contains("unavailable ("),
                    "{context}: disabled action {author_id} hides its reason: `{label}`"
                );
            }
        }

        // Engine internals: an available payload must offer its read-only drilldown.
        let internals = node_text(require_node(
            root,
            &row_engine_internals_author_id(pane_id, artifact),
        ));
        if internals.contains("available") && !internals.contains("unavailable") {
            assert!(
                contains_author_id(
                    root,
                    &row_engine_internals_expand_author_id(pane_id, artifact)
                ),
                "{context}: available engine internals must expose their expand target"
            );
        }
    }
    artifacts
}

#[ignore = "LIVE production socket E2E: opens native main/pop-out windows and requires injected \
            embedded SurrealDB, Palmistry-ready handshake_core on 127.0.0.1:37501, \
            HANDSHAKE_ARGUS_LIVE_BACKEND_READY=1, and a shared HANDSHAKE_DIAGNOSTICS_DIR"]
#[test]
fn production_socket_model_runtime_rows_refresh_receipt_negative_states_and_detached_window() {
    let mut app = LiveApp::start("model_runtime");

    // ── Operator navigation: the WP leaves live under the top-level MODELS menu ─────────────────
    app.open_models_menu_leaf("menu.models.model-runtime");
    let discovered_surface = wait_for_author_id_between(
        &mut app.client,
        "main",
        &format!("{AUTHOR_ID_PREFIX}."),
        ".surface",
        SURFACE_TIMEOUT,
    );
    let pane_id = discovered_surface
        .strip_prefix(&format!("{AUTHOR_ID_PREFIX}."))
        .and_then(|rest| rest.strip_suffix(".surface"))
        .expect("ModelRuntime surface author_id is `<prefix>.<pane_id>.surface`")
        .to_owned();
    assert_eq!(discovered_surface, surface_author_id(&pane_id));

    let opened = wait_for_settled_registry(&mut app, "main", &pane_id);
    let root = opened["snapshot"]["root"].clone();
    assert_eq!(
        pane_id_hosting(&root, &PaneType::ModelRuntime.label()),
        pane_id,
        "the pane-scoped ModelRuntime author_ids must belong to the pane hosting the surface"
    );
    let docked_artifacts = assert_registry_rows_are_honest(&root, &pane_id, "docked main window");

    // ── Safe mutating action with before/after receipts ──────────────────────────────────────────
    // Refresh re-reads the durable registry; it never mutates model state, so it is safe with or
    // without a loaded model. The receipt must be applied, attributed, revision-advancing and
    // durable — the exact contract an in-process Harness could never prove.
    let status_before = node_text(require_node(&root, &status_author_id(&pane_id)));
    let refresh_receipt = app.client.mutation_on_live_surface(
        "argus.click",
        "main",
        &refresh_author_id(&pane_id),
        None,
    );
    let refreshed = wait_for_settled_registry(&mut app, "main", &pane_id);
    let refreshed_root = refreshed["snapshot"]["root"].clone();
    let status_after = node_text(require_node(&refreshed_root, &status_author_id(&pane_id)));
    assert_honest_registry_status(&status_after);
    assert_registry_rows_are_honest(&refreshed_root, &pane_id, "after live refresh");

    // ── Visual evidence of the docked panel ─────────────────────────────────────────────────────
    let main_shot = app.client.screenshot("main");
    let main_png = decode_verified_capture(
        &main_shot,
        "main",
        app.child_pid,
        "ModelRuntime docked main-window capture",
    );

    // ── Detached window: the same pane in a real second OS window ───────────────────────────────
    let popout_window_id = app.pop_out_pane(&pane_id);
    wait_for_author_id(
        &mut app.client,
        &popout_window_id,
        &surface_author_id(&pane_id),
        SURFACE_TIMEOUT,
    );
    let detached = wait_for_settled_registry(&mut app, &popout_window_id, &pane_id);
    let detached_root = detached["snapshot"]["root"].clone();
    assert!(
        contains_author_id(&detached_root, &popout_window_author_id(&pane_id)),
        "the detached window must carry its stable window-root target"
    );
    let detached_artifacts =
        assert_registry_rows_are_honest(&detached_root, &pane_id, "detached pop-out window");

    // While detached, the main window shows the placeholder + merge-back, NOT a second copy of the
    // panel, so a stable author_id can never be ambiguous across windows.
    let main_while_detached = app.client.inspect("main");
    let main_while_detached_root = main_while_detached["snapshot"]["root"].clone();
    assert!(
        !contains_author_id(&main_while_detached_root, &surface_author_id(&pane_id)),
        "a detached pane must not still render inside the main window"
    );
    assert!(
        contains_author_id(
            &main_while_detached_root,
            &handshake_native::popout_window::merge_back_author_id(&pane_id)
        ),
        "the main window must expose the merge-back control for the detached pane"
    );

    let detached_shot = app.client.screenshot(&popout_window_id);
    let detached_png = decode_verified_capture(
        &detached_shot,
        &popout_window_id,
        app.child_pid,
        "ModelRuntime detached-window capture",
    );

    // The detached window must be STEERABLE, not merely visible/capturable.
    let detached_refresh = app.client.mutation_on_live_surface(
        "argus.click",
        &popout_window_id,
        &refresh_author_id(&pane_id),
        None,
    );
    let detached_after_refresh = wait_for_settled_registry(&mut app, &popout_window_id, &pane_id);
    assert_registry_rows_are_honest(
        &detached_after_refresh["snapshot"]["root"],
        &pane_id,
        "detached window after its own refresh",
    );

    app.merge_back_pane(&pane_id);
    wait_for_author_id(
        &mut app.client,
        "main",
        &surface_author_id(&pane_id),
        SURFACE_TIMEOUT,
    );
    let remerged = wait_for_settled_registry(&mut app, "main", &pane_id);
    assert_registry_rows_are_honest(
        &remerged["snapshot"]["root"],
        &pane_id,
        "after merge-back into the main window",
    );

    // ── Proof artifacts ─────────────────────────────────────────────────────────────────────────
    app.write_proof_artifact("argus_production_socket_model_runtime_main.png", &main_png);
    app.write_proof_artifact(
        "argus_production_socket_model_runtime_popout.png",
        &detached_png,
    );
    let transcript = app.client.assert_transcript_is_secret_free(&[]);
    app.write_proof_artifact(
        "argus_production_socket_model_runtime_transcript.json",
        &transcript,
    );
    let provenance = serde_json::json!({
        "schema_id": "handshake.argus.production_socket_model_runtime_provenance@1",
        "mt_id": "MT-014",
        "child_pid": app.child_pid,
        "authenticated_agent_id": app.authenticated_agent_id,
        "navigation": ["menu-models", "menu.models.model-runtime"],
        "pane_id": pane_id,
        "docked_row_artifacts": docked_artifacts,
        "detached_row_artifacts": detached_artifacts,
        "status_before_refresh": status_before,
        "status_after_refresh": status_after,
        "docked_refresh_receipt": {
            "status": refresh_receipt["result"]["status"],
            "before_revision": refresh_receipt["result"]["before_revision"],
            "after_revision": refresh_receipt["result"]["after_revision"],
            "evidence_ref": refresh_receipt["result"]["evidence_ref"],
            "agent_id": refresh_receipt["result"]["agent_id"],
            "agent_label": refresh_receipt["result"]["agent_label"],
        },
        "detached_refresh_receipt": {
            "window_id": popout_window_id,
            "status": detached_refresh["result"]["status"],
            "before_revision": detached_refresh["result"]["before_revision"],
            "after_revision": detached_refresh["result"]["after_revision"],
            "evidence_ref": detached_refresh["result"]["evidence_ref"],
        },
        "captures": [
            {
                "artifact": "argus_production_socket_model_runtime_main.png",
                "window_id": main_shot["result"]["window_id"],
                "pid": main_shot["result"]["pid"],
                "width": main_shot["result"]["width"],
                "height": main_shot["result"]["height"],
                "captured_at_utc": main_shot["result"]["captured_at_utc"],
                "sha256": main_shot["result"]["sha256"],
            },
            {
                "artifact": "argus_production_socket_model_runtime_popout.png",
                "window_id": detached_shot["result"]["window_id"],
                "pid": detached_shot["result"]["pid"],
                "width": detached_shot["result"]["width"],
                "height": detached_shot["result"]["height"],
                "captured_at_utc": detached_shot["result"]["captured_at_utc"],
                "sha256": detached_shot["result"]["sha256"],
            }
        ],
    });
    app.write_proof_artifact(
        "argus_production_socket_model_runtime_provenance.json",
        &serde_json::to_vec_pretty(&provenance).expect("serialize ModelRuntime provenance"),
    );

    app.shutdown();
}

#[ignore = "LIVE production socket E2E: opens native main/pop-out windows and requires injected \
            embedded SurrealDB, Palmistry-ready handshake_core on 127.0.0.1:37501, \
            HANDSHAKE_ARGUS_LIVE_BACKEND_READY=1, and a shared HANDSHAKE_DIAGNOSTICS_DIR"]
#[test]
fn operator_chat_live_argus_docked_and_detached_launch_transcript() {
    let foreground_before = foreground_process_id();
    let mut app = LiveApp::start("operator_chat");

    app.open_models_menu_leaf("menu.models.operator-chat");
    let opened = wait_for_author_id(&mut app.client, "main", SURFACE_AUTHOR_ID, SURFACE_TIMEOUT);
    let root = opened["snapshot"]["root"].clone();
    let pane_id = pane_id_hosting(&root, &PaneType::OperatorChatLaunch.label());

    // ── Every operator control is Argus-visible on the production surface ───────────────────────
    for author_id in [
        SURFACE_AUTHOR_ID,
        MODEL_PICKER_AUTHOR_ID,
        REFRESH_MODELS_AUTHOR_ID,
        WORKTREE_SELECTION_AUTHOR_ID,
        PROMPT_INPUT_AUTHOR_ID,
        LAUNCH_AUTHOR_ID,
        TRANSCRIPT_AUTHOR_ID,
        ROUTING_REQUEST_AUTHOR_ID,
        ROUTING_LIFECYCLE_AUTHOR_ID,
        ROUTING_RECOVER_AUTHOR_ID,
        ROUTING_CANCEL_AUTHOR_ID,
        ROUTING_AUTHORITY_AUTHOR_ID,
    ] {
        assert!(
            contains_author_id(&root, author_id),
            "Operator Chat control {author_id} is not visible over the production socket"
        );
    }

    let worktree = std::fs::canonicalize(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.."),
    )
    .expect("resolve the production Handshake worktree")
    .to_string_lossy()
    .into_owned();
    let worktree_node = require_node(&root, WORKTREE_SELECTION_AUTHOR_ID);
    assert!(
        node_supports(worktree_node, "SetValue"),
        "the production worktree seam must be socket-settable without a native picker"
    );
    let worktree_receipt = app.client.mutation_on_live_surface(
        "argus.set_value",
        "main",
        WORKTREE_SELECTION_AUTHOR_ID,
        Some(("value", serde_json::Value::String(worktree.clone()))),
    );

    // ── Prompt entry: real set_value with an applied receipt and a re-observed value ────────────
    let prompt_before = require_node(&root, PROMPT_INPUT_AUTHOR_ID);
    let prompt_value_before = prompt_before
        .get("value")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default()
        .to_owned();
    let docked_prompt = "production-socket-operator-chat-docked-prompt";
    assert_ne!(
        prompt_value_before, docked_prompt,
        "the prompt already held the proof marker before the proof wrote it"
    );
    let prompt_receipt = app.client.mutation_on_live_surface(
        "argus.set_value",
        "main",
        PROMPT_INPUT_AUTHOR_ID,
        Some(("value", serde_json::Value::String(docked_prompt.to_owned()))),
    );
    let after_prompt = app.client.inspect("main");
    let after_prompt_root = after_prompt["snapshot"]["root"].clone();
    assert_eq!(
        require_node(&after_prompt_root, PROMPT_INPUT_AUTHOR_ID)
            .get("value")
            .and_then(serde_json::Value::as_str),
        Some(docked_prompt),
        "the operator prompt did not carry the value the production socket set"
    );
    assert_eq!(
        require_node(&after_prompt_root, WORKTREE_SELECTION_AUTHOR_ID)
            .get("value")
            .and_then(serde_json::Value::as_str),
        Some(worktree.as_str()),
        "the focus-safe worktree selection was not re-observed through the production socket"
    );

    app.client
        .mutation_on_live_surface("argus.click", "main", REFRESH_MODELS_AUTHOR_ID, None);
    let (session_author_id, session_inventory) =
        wait_for_enabled_author_prefix(&mut app, "main", "operator-chat.session.");
    let session_label = node_text(require_node(
        &session_inventory["snapshot"]["root"],
        &session_author_id,
    ));
    assert!(
        !session_label.trim().is_empty(),
        "the selected governed session must have a live operator-readable label"
    );
    let (model_author_id, model_inventory) =
        wait_for_enabled_author_prefix(&mut app, "main", "operator-chat.model.subagent.");
    let model_label = node_text(require_node(
        &model_inventory["snapshot"]["root"],
        &model_author_id,
    ));
    assert!(
        model_label.contains("SUBAGENT") && model_label.contains("available"),
        "the live no-OS subagent row must be honestly selectable: `{model_label}`"
    );
    app.client
        .mutation_on_live_surface("argus.click", "main", &session_author_id, None);
    app.client
        .mutation_on_live_surface("argus.click", "main", &model_author_id, None);

    // ── Real backend launch + captured transcript ───────────────────────────────────────────────
    let launch_ready = wait_for_operator_chat_launch_enabled(&mut app);
    assert!(
        !node_is_disabled(require_node(
            &launch_ready["snapshot"]["root"],
            LAUNCH_AUTHOR_ID,
        )),
        "live governed session + subagent + worktree + prompt must enable launch"
    );
    let launch_receipt =
        app.client
            .mutation_on_live_surface("argus.click", "main", LAUNCH_AUTHOR_ID, None);
    let launched = wait_for_operator_chat_transcript(&mut app);
    let launched_root = launched["snapshot"]["root"].clone();
    let launch_status = node_text(require_node(&launched_root, LAUNCH_STATUS_AUTHOR_ID));
    assert!(
        launch_status.starts_with("launched run ")
            && launch_status.contains("lane ")
            && launch_status.contains("instance "),
        "launch status must carry real backend run/lane/instance identity: `{launch_status}`"
    );
    let transcript_author_ids = collect_author_ids(&launched_root)
        .into_iter()
        .filter(|author_id| {
            author_id.starts_with("operator-chat.transcript.message.")
                || author_id.starts_with("operator-chat.transcript.row.")
        })
        .collect::<Vec<_>>();
    assert!(
        !transcript_author_ids.is_empty(),
        "the production launch must render backend-captured transcript rows"
    );
    let transcript_text = node_text(require_node(&launched_root, TRANSCRIPT_AUTHOR_ID));
    assert!(
        transcript_author_ids.iter().all(|author_id| {
            node_by_author_id(&launched_root, author_id)
                .is_some_and(|node| !node_text(node).trim().is_empty())
        }),
        "every captured transcript row must expose nonblank production text: `{transcript_text}`"
    );

    let docked_shot = app.client.screenshot("main");
    let docked_png = decode_verified_capture(
        &docked_shot,
        "main",
        app.child_pid,
        "Operator Chat docked main-window capture",
    );

    // ── Detached window: capture, inspect, and steer the popped-out pane ────────────────────────
    let popout_window_id = app.pop_out_pane(&pane_id);
    let detached = wait_for_author_id(
        &mut app.client,
        &popout_window_id,
        SURFACE_AUTHOR_ID,
        SURFACE_TIMEOUT,
    );
    let detached_root = detached["snapshot"]["root"].clone();
    assert!(
        contains_author_id(&detached_root, &popout_window_author_id(&pane_id)),
        "the detached Operator Chat window must carry its stable window-root target"
    );
    for author_id in [
        MODEL_PICKER_AUTHOR_ID,
        WORKTREE_SELECTION_AUTHOR_ID,
        PROMPT_INPUT_AUTHOR_ID,
        LAUNCH_AUTHOR_ID,
        TRANSCRIPT_AUTHOR_ID,
    ] {
        assert!(
            contains_author_id(&detached_root, author_id),
            "detached Operator Chat lost control {author_id}"
        );
    }
    for author_id in [&session_author_id, &model_author_id]
        .into_iter()
        .chain(transcript_author_ids.iter())
    {
        assert!(
            contains_author_id(&detached_root, author_id),
            "detached Operator Chat lost live state target {author_id}"
        );
    }
    assert_eq!(
        require_node(&detached_root, PROMPT_INPUT_AUTHOR_ID)
            .get("value")
            .and_then(serde_json::Value::as_str),
        Some(docked_prompt),
        "the detached window must render the SAME live pane state, not a fresh empty copy"
    );
    assert_eq!(
        require_node(&detached_root, WORKTREE_SELECTION_AUTHOR_ID)
            .get("value")
            .and_then(serde_json::Value::as_str),
        Some(worktree.as_str()),
        "detaching must preserve the injected production worktree"
    );
    assert_eq!(
        node_text(require_node(&detached_root, LAUNCH_STATUS_AUTHOR_ID)),
        launch_status,
        "detached rendering must preserve the real backend launch identity"
    );

    let detached_shot = app.client.screenshot(&popout_window_id);
    let detached_png = decode_verified_capture(
        &detached_shot,
        &popout_window_id,
        app.child_pid,
        "Operator Chat detached-window capture",
    );

    let detached_prompt = "production-socket-operator-chat-detached-prompt";
    let detached_receipt = app.client.mutation_on_live_surface(
        "argus.set_value",
        &popout_window_id,
        PROMPT_INPUT_AUTHOR_ID,
        Some((
            "value",
            serde_json::Value::String(detached_prompt.to_owned()),
        )),
    );
    let detached_after = app.client.inspect(&popout_window_id);
    assert_eq!(
        require_node(&detached_after["snapshot"]["root"], PROMPT_INPUT_AUTHOR_ID)
            .get("value")
            .and_then(serde_json::Value::as_str),
        Some(detached_prompt),
        "the detached Operator Chat window is not steerable over the production socket"
    );

    app.merge_back_pane(&pane_id);
    let remerged = wait_for_author_id(&mut app.client, "main", SURFACE_AUTHOR_ID, SURFACE_TIMEOUT);
    assert_eq!(
        require_node(&remerged["snapshot"]["root"], PROMPT_INPUT_AUTHOR_ID)
            .get("value")
            .and_then(serde_json::Value::as_str),
        Some(detached_prompt),
        "merging back must preserve the live pane state the detached window edited"
    );
    assert_eq!(
        require_node(&remerged["snapshot"]["root"], WORKTREE_SELECTION_AUTHOR_ID,)
            .get("value")
            .and_then(serde_json::Value::as_str),
        Some(worktree.as_str()),
        "merging back must preserve the focus-safe worktree selection"
    );
    for author_id in &transcript_author_ids {
        assert!(
            contains_author_id(&remerged["snapshot"]["root"], author_id),
            "merge-back lost captured transcript target {author_id}"
        );
    }

    let foreground_after = foreground_process_id();
    assert_ne!(
        foreground_before, 0,
        "MT-012 focus proof requires an identifiable foreground owner"
    );
    assert_eq!(
        foreground_after, foreground_before,
        "socket-driven Operator Chat launch changed the operator's foreground owner"
    );
    assert_ne!(
        foreground_after, app.child_pid,
        "socket-driven Operator Chat proof must not activate the production child"
    );

    app.write_proof_artifact(
        "argus_production_socket_operator_chat_main.png",
        &docked_png,
    );
    app.write_proof_artifact(
        "argus_production_socket_operator_chat_popout.png",
        &detached_png,
    );
    let transcript = app.client.assert_transcript_is_secret_free(&[]);
    app.write_proof_artifact(
        "argus_production_socket_operator_chat_transcript.json",
        &transcript,
    );
    let provenance = serde_json::json!({
        "schema_id": "handshake.argus.production_socket_operator_chat_provenance@1",
        "mt_id": "MT-012",
        "child_pid": app.child_pid,
        "authenticated_agent_id": app.authenticated_agent_id,
        "navigation": ["menu-models", "menu.models.operator-chat"],
        "pane_id": pane_id,
        "foreground_pid_before": foreground_before,
        "foreground_pid_after": foreground_after,
        "worktree_selection": {
            "author_id": WORKTREE_SELECTION_AUTHOR_ID,
            "status": worktree_receipt["result"]["status"],
            "before_revision": worktree_receipt["result"]["before_revision"],
            "after_revision": worktree_receipt["result"]["after_revision"],
            "evidence_ref": worktree_receipt["result"]["evidence_ref"],
        },
        "live_inventory": {
            "session_author_id": session_author_id,
            "session_label": session_label,
            "model_author_id": model_author_id,
            "model_label": model_label,
        },
        "docked_prompt_receipt": {
            "status": prompt_receipt["result"]["status"],
            "before_revision": prompt_receipt["result"]["before_revision"],
            "after_revision": prompt_receipt["result"]["after_revision"],
            "evidence_ref": prompt_receipt["result"]["evidence_ref"],
            "agent_id": prompt_receipt["result"]["agent_id"],
            "agent_label": prompt_receipt["result"]["agent_label"],
        },
        "detached_prompt_receipt": {
            "window_id": popout_window_id,
            "status": detached_receipt["result"]["status"],
            "before_revision": detached_receipt["result"]["before_revision"],
            "after_revision": detached_receipt["result"]["after_revision"],
            "evidence_ref": detached_receipt["result"]["evidence_ref"],
        },
        "real_backend_launch": {
            "control": LAUNCH_AUTHOR_ID,
            "click_status": launch_receipt["result"]["status"],
            "before_revision": launch_receipt["result"]["before_revision"],
            "after_revision": launch_receipt["result"]["after_revision"],
            "evidence_ref": launch_receipt["result"]["evidence_ref"],
            "launch_status": launch_status,
            "transcript_author_ids": transcript_author_ids,
        },
        "captures": [
            {
                "artifact": "argus_production_socket_operator_chat_main.png",
                "window_id": docked_shot["result"]["window_id"],
                "pid": docked_shot["result"]["pid"],
                "width": docked_shot["result"]["width"],
                "height": docked_shot["result"]["height"],
                "captured_at_utc": docked_shot["result"]["captured_at_utc"],
                "sha256": docked_shot["result"]["sha256"],
            },
            {
                "artifact": "argus_production_socket_operator_chat_popout.png",
                "window_id": detached_shot["result"]["window_id"],
                "pid": detached_shot["result"]["pid"],
                "width": detached_shot["result"]["width"],
                "height": detached_shot["result"]["height"],
                "captured_at_utc": detached_shot["result"]["captured_at_utc"],
                "sha256": detached_shot["result"]["sha256"],
            }
        ],
    });
    app.write_proof_artifact(
        "argus_production_socket_operator_chat_provenance.json",
        &serde_json::to_vec_pretty(&provenance).expect("serialize Operator Chat provenance"),
    );

    app.shutdown();
}

#[ignore = "LIVE production socket E2E: opens native main/pop-out windows and requires injected \
            embedded SurrealDB, Palmistry-ready handshake_core on 127.0.0.1:37501, \
            HANDSHAKE_ARGUS_LIVE_BACKEND_READY=1, and a shared HANDSHAKE_DIAGNOSTICS_DIR"]
#[test]
fn production_socket_settings_cloud_access_login_states_and_no_secret_disclosure() {
    let mut app = LiveApp::start("settings_cloud");
    let canary = "production-socket-settings-secret-canary";

    // Reach Settings through the MODELS menu (the WP navigation path), not only HELP.
    app.open_models_menu_leaf("menu.models.settings");
    wait_for_author_id(
        &mut app.client,
        "main",
        SETTINGS_DIALOG_AUTHOR_ID,
        SURFACE_TIMEOUT,
    );
    // The CLI-bridge rows are rendered from the backend's non-secret provider enumeration, which the
    // shell fetches asynchronously. Wait for that authority to arrive so the login-state assertions
    // below read the real enumeration instead of the pre-fetch frame; if it never arrives, the proof
    // fails loudly rather than silently skipping the login-state coverage.
    let opened = wait_for_author_id(
        &mut app.client,
        "main",
        &cloud_cli_status_author_id("claude_code"),
        SURFACE_TIMEOUT,
    );
    let root = opened["snapshot"]["root"].clone();

    // ── BYOK rows: present, addressable, and structurally non-disclosing ────────────────────────
    for provider in ["openai", "anthropic"] {
        for author_id in [
            cloud_byok_key_author_id(provider),
            cloud_byok_status_author_id(provider),
            cloud_byok_save_author_id(provider),
            cloud_byok_remove_author_id(provider),
        ] {
            assert!(
                contains_author_id(&root, &author_id),
                "Settings cloud surface omitted {author_id}"
            );
        }

        // The key input is addressable for visibility/click proof, but the generic Argus surface may
        // neither PROJECT its value nor OFFER a write path: key material crosses only the dedicated
        // OS-keychain route.
        let key_node = require_node(&root, &cloud_byok_key_author_id(provider));
        assert!(
            key_node.get("value").is_none() || key_node["value"].is_null(),
            "the {provider} BYOK key input projected a value into the Argus snapshot: {key_node}"
        );
        assert!(
            !node_supports(key_node, "SetValue"),
            "the {provider} BYOK key input advertised a generic SetValue write path"
        );

        // Honest configuration state, and an unconfigured provider cannot offer key removal.
        let status_label = node_text(require_node(&root, &cloud_byok_status_author_id(provider)));
        assert!(
            HONEST_BYOK_STATES
                .iter()
                .any(|state| status_label.contains(state)),
            "{provider} BYOK status is not one of the production states: `{status_label}`"
        );
        let configured = status_label.contains("Configured — key stored in the OS keychain");
        let remove_node = require_node(&root, &cloud_byok_remove_author_id(provider));
        assert_eq!(
            node_is_disabled(remove_node),
            !configured,
            "{provider} remove/rotate enablement disagrees with its reported state `{status_label}`"
        );
    }

    // Gemini is never offered as a provider.
    assert!(
        !collect_author_ids(&root)
            .iter()
            .any(|author_id| author_id.contains("gemini")),
        "the cloud-access surface must never offer a Gemini provider"
    );

    // ── CLI-bridge login-state rows ─────────────────────────────────────────────────────────────
    for provider in ["claude_code", "codex"] {
        for author_id in [
            cloud_cli_status_author_id(provider),
            cloud_cli_login_author_id(provider),
        ] {
            assert!(
                contains_author_id(&root, &author_id),
                "Settings cloud surface omitted {author_id}"
            );
        }
        let login_state = node_text(require_node(&root, &cloud_cli_status_author_id(provider)));
        assert!(
            HONEST_CLI_LOGIN_STATES
                .iter()
                .any(|state| login_state == *state),
            "{provider} login state is not one of the production states: `{login_state}`"
        );
    }

    // ── The generic write path is refused for secret-bearing inputs, without echoing the value ──
    let (settings_revision, denial) = app.client.attempt_mutation(
        "argus.set_value",
        "main",
        &cloud_byok_key_author_id("openai"),
        Some(("value", serde_json::Value::String(canary.to_owned()))),
    );
    assert!(
        denial.get("error").is_some(),
        "a secret-bearing input accepted a generic Argus set_value: {denial}"
    );
    assert!(
        !denial.to_string().contains(canary),
        "the refusal echoed the secret value"
    );
    let after_denial = app.client.inspect("main");
    let after_denial_root = after_denial["snapshot"]["root"].clone();
    // The refused write had NO effect on the secret-bearing control. The invariant is asserted on the
    // control itself rather than on the window revision, because a live shell legitimately republishes
    // its snapshot for unrelated reasons (backend health polling), and a proof must not read that as a
    // secret-write side effect.
    let key_after_denial = require_node(&after_denial_root, &cloud_byok_key_author_id("openai"));
    assert!(
        key_after_denial.get("value").is_none() || key_after_denial["value"].is_null(),
        "the refused secret write left a projected value on the key input: {key_after_denial}"
    );
    assert!(
        !node_supports(key_after_denial, "SetValue"),
        "the refused secret write opened a generic write path on the key input"
    );
    assert!(
        !serde_json::to_string(&after_denial_root)
            .expect("serialize Settings tree")
            .contains(canary),
        "the Settings snapshot disclosed the canary"
    );

    // ── No disclosure through ANY read surface: the compat alias, the canonical inspect, the
    //    window listing, and the real screenshot response (headers AND pixels). ─────────────────
    let alias_read = app.client.inspect_via("list_widgets", "main");
    assert!(
        !alias_read.to_string().contains(canary),
        "the list_widgets compatibility alias disclosed the canary"
    );
    for author_id in [
        SETTINGS_DIALOG_AUTHOR_ID.to_owned(),
        cloud_byok_key_author_id("openai"),
        cloud_byok_status_author_id("openai"),
        cloud_cli_status_author_id("claude_code"),
    ] {
        assert!(
            contains_author_id(&alias_read["result"]["snapshot"]["root"], &author_id),
            "the list_widgets alias lost the Settings landmark {author_id}"
        );
    }
    let window_list = app.client.call("argus.list_windows", serde_json::json!({}));
    assert!(
        !window_list.to_string().contains(canary),
        "argus.list_windows disclosed the canary"
    );

    let settings_shot = app.client.screenshot("main");
    assert!(
        !settings_shot.to_string().contains(canary),
        "the screenshot response disclosed the canary"
    );
    let settings_png = decode_verified_capture(
        &settings_shot,
        "main",
        app.child_pid,
        "Settings cloud-access main-window capture",
    );
    assert_bytes_exclude(&settings_png, canary, "Settings-open PNG bytes");

    // The landmarks were on screen for the capture, before AND after it (not an earlier frame).
    let after_capture = app.client.inspect("main");
    let after_capture_root = after_capture["snapshot"]["root"].clone();
    for provider in ["openai", "anthropic"] {
        assert!(
            contains_author_id(&after_capture_root, &cloud_byok_status_author_id(provider)),
            "a Settings landmark vanished across the visual capture"
        );
    }
    assert!(
        !serde_json::to_string(&after_capture_root)
            .expect("serialize post-capture Settings tree")
            .contains(canary),
        "the post-capture Settings snapshot disclosed the canary"
    );

    app.client
        .mutation_on_live_surface("argus.click", "main", CLOSE_AUTHOR_ID, None);

    app.write_proof_artifact(
        "argus_production_socket_settings_cloud_main.png",
        &settings_png,
    );
    let transcript = app.client.assert_transcript_is_secret_free(&[canary]);
    app.write_proof_artifact(
        "argus_production_socket_settings_cloud_transcript.json",
        &transcript,
    );
    let provenance = serde_json::json!({
        "schema_id": "handshake.argus.production_socket_settings_cloud_provenance@1",
        "mt_id": "MT-015",
        "child_pid": app.child_pid,
        "authenticated_agent_id": app.authenticated_agent_id,
        "navigation": ["menu-models", "menu.models.settings"],
        "settings_revision_before_refused_write": settings_revision,
        "no_secret_disclosure": {
            "byok_key_value_projected": false,
            "byok_key_set_value_action_offered": false,
            "generic_set_value_refused": true,
            "canary_absent_from_inspect": true,
            "canary_absent_from_list_widgets_alias": true,
            "canary_absent_from_list_windows": true,
            "canary_absent_from_screenshot_response_and_pixels": true,
            "canary_absent_from_transcript": true,
        },
        "capture": {
            "artifact": "argus_production_socket_settings_cloud_main.png",
            "window_id": settings_shot["result"]["window_id"],
            "pid": settings_shot["result"]["pid"],
            "width": settings_shot["result"]["width"],
            "height": settings_shot["result"]["height"],
            "captured_at_utc": settings_shot["result"]["captured_at_utc"],
            "sha256": settings_shot["result"]["sha256"],
        },
        "follow_up": "A DETACHED Settings window proof is intentionally out of scope here; that \
                      surface is being implemented in parallel and needs its own live proof.",
    });
    app.write_proof_artifact(
        "argus_production_socket_settings_cloud_provenance.json",
        &serde_json::to_vec_pretty(&provenance).expect("serialize Settings provenance"),
    );

    app.shutdown();
}

/// WP-1 MT-021 V5 remediation: exercise the exact operator surfaces through the authenticated
/// production Argus/Palmistry path. This is deliberately separate from the deterministic Settings
/// tests: the acceptance gap was evidence from the running shell in both window hosts, not another
/// in-process assertion over the same widget code.
#[ignore = "LIVE production socket E2E: opens native main/pop-out windows and requires injected \
            embedded SurrealDB, Palmistry-ready handshake_core on 127.0.0.1:37501, \
            HANDSHAKE_ARGUS_LIVE_BACKEND_READY=1, and a shared HANDSHAKE_DIAGNOSTICS_DIR"]
#[test]
fn mt021_production_settings_consent_concurrency_and_console_main_detached() {
    let mut app = LiveApp::start("mt021_settings_console");

    let cold_navigation_started = std::time::Instant::now();
    app.open_models_menu_leaf("menu.models.settings");
    let cold_navigation_receipt_round_trip_ms =
        cold_navigation_started.elapsed().as_millis() as u64;
    wait_for_author_id(
        &mut app.client,
        "main",
        SETTINGS_DIALOG_AUTHOR_ID,
        SURFACE_TIMEOUT,
    );

    // Narrow the real Settings list to the two concurrency controls. The model-session row must
    // report backend truth; loading/unavailable text is not accepted for this live proof.
    app.client.mutation_on_live_surface(
        "argus.set_value",
        "main",
        SETTINGS_SEARCH_AUTHOR_ID,
        Some(("value", serde_json::json!("concurrency"))),
    );
    let coordinator_deadline = std::time::Instant::now() + SURFACE_TIMEOUT;
    let mut last_coordinator_status = "not rendered".to_owned();
    let concurrency = loop {
        let observed = app.client.poll_inspect("main");
        let root = &observed["snapshot"]["root"];
        if contains_author_id(root, SWARM_MODEL_SESSIONS_COMBO_AUTHOR_ID) {
            if let Some(status) = node_by_author_id(root, SWARM_MODEL_SESSIONS_STATUS_AUTHOR_ID) {
                last_coordinator_status = node_text(status);
                if last_coordinator_status.starts_with("Requested:")
                    && last_coordinator_status.contains("In force:")
                    && last_coordinator_status.contains("Fully applied:")
                    && last_coordinator_status.contains("Live sessions:")
                {
                    break observed;
                }
            }
        }
        assert!(
            std::time::Instant::now() < coordinator_deadline,
            "model-session control never loaded coordinator truth: `{last_coordinator_status}`"
        );
        std::thread::sleep(std::time::Duration::from_millis(200));
    };
    let root = &concurrency["snapshot"]["root"];
    let status_before = node_text(require_node(root, SWARM_MODEL_SESSIONS_STATUS_AUTHOR_ID));
    assert!(
        status_before.starts_with("Requested:")
            && status_before.contains("In force:")
            && status_before.contains("Fully applied:")
            && status_before.contains("Live sessions:"),
        "model-session control did not render live coordinator truth: `{status_before}`"
    );
    let original_requested = status_before
        .strip_prefix("Requested: ")
        .and_then(|value| value.split_whitespace().next())
        .and_then(|value| value.parse::<usize>().ok())
        .expect("live coordinator status exposes its numeric requested cap");
    let current_text = node_text(require_node(root, SWARM_MODEL_SESSIONS_COMBO_AUTHOR_ID));
    assert_eq!(
        current_text.trim(),
        original_requested.to_string(),
        "ComboBox selection must reflect the backend's requested cap"
    );
    let target = if original_requested == 2 { 4 } else { 2 };

    // Change the cap through the actual ComboBox popup. The post-action status can only reach the
    // requested value after the frontend PUT has traversed the production backend route.
    app.client.mutation_on_live_surface(
        "argus.click",
        "main",
        SWARM_MODEL_SESSIONS_COMBO_AUTHOR_ID,
        None,
    );
    let option_id = swarm_model_session_option_author_id(target);
    wait_for_author_id(&mut app.client, "main", &option_id, SURFACE_TIMEOUT);
    app.client
        .mutation_on_live_surface("argus.click", "main", &option_id, None);

    let deadline = std::time::Instant::now() + SURFACE_TIMEOUT;
    let status_after = loop {
        let observed = app.client.poll_inspect("main");
        if let Some(node) = node_by_author_id(
            &observed["snapshot"]["root"],
            SWARM_MODEL_SESSIONS_STATUS_AUTHOR_ID,
        ) {
            let text = node_text(node);
            if text.contains(&format!("Requested: {target}"))
                && text.contains("In force:")
                && !text.contains("loading")
                && !text.contains("unavailable")
            {
                break text;
            }
        }
        assert!(
            std::time::Instant::now() < deadline,
            "live coordinator status never reflected requested cap {target}"
        );
        std::thread::sleep(std::time::Duration::from_millis(200));
    };
    let main_concurrency_shot = app.client.screenshot("main");
    let main_concurrency_png = decode_verified_capture(
        &main_concurrency_shot,
        "main",
        app.child_pid,
        "MT-021 Settings concurrency main-window capture",
    );

    // Switch the same live surface to Cloud Models and prove the exact explicit-unavailable posture.
    // The state is label-only: it offers no action capable of granting or widening access.
    app.client.mutation_on_live_surface(
        "argus.set_value",
        "main",
        SETTINGS_SEARCH_AUTHOR_ID,
        Some(("value", serde_json::json!("cloud"))),
    );
    // The summary and static BYOK posture rows render before the asynchronous provider enumeration
    // returns.  The production backend probes both official CLIs while serving that enumeration, so
    // observing only the summary is not proof that the configured CLI lanes have arrived.  Wait for
    // every contract lane before taking the single snapshot used for the assertions and capture.
    for provider in ["openai", "anthropic", "claude_code", "codex"] {
        wait_for_author_id(
            &mut app.client,
            "main",
            &cloud_consent_posture_author_id(provider),
            SURFACE_TIMEOUT,
        );
    }
    let cloud = wait_for_author_id(
        &mut app.client,
        "main",
        CLOUD_CONSENT_STATUS_AUTHOR_ID,
        SURFACE_TIMEOUT,
    );
    let cloud_root = &cloud["snapshot"]["root"];
    let consent_summary = node_text(require_node(cloud_root, CLOUD_CONSENT_STATUS_AUTHOR_ID));
    assert!(
        consent_summary.contains("NOT WIRED")
            && consent_summary.contains("Nothing here grants, widens, or records consent"),
        "consent summary fabricated or widened posture: `{consent_summary}`"
    );
    for provider in ["openai", "anthropic", "claude_code", "codex"] {
        let author_id = cloud_consent_posture_author_id(provider);
        let row = require_node(cloud_root, &author_id);
        let text = node_text(row);
        assert!(
            text.contains("NOT WIRED") && text.contains("no posture is shown and none is assumed"),
            "{provider} posture was not explicitly unavailable: `{text}`"
        );
        assert!(
            !node_supports(row, "Click") && !node_supports(row, "SetValue"),
            "{provider} posture exposed an authority-widening action: {row}"
        );
        for forbidden in [
            "account_id",
            "access_space_id",
            "project_id",
            "resource_id",
            "artifact://",
        ] {
            assert!(
                !text.contains(forbidden),
                "{provider} posture leaked restricted metadata marker `{forbidden}`: `{text}`"
            );
        }
    }
    let main_cloud_shot = app.client.screenshot("main");
    let main_cloud_png = decode_verified_capture(
        &main_cloud_shot,
        "main",
        app.child_pid,
        "MT-021 Settings consent main-window capture",
    );

    // Move the same Settings state into its real detached OS window. Inspect cloud posture there,
    // then change the persisted search query and inspect the live concurrency status in that host.
    app.client
        .mutation_on_live_surface("argus.click", "main", SETTINGS_POPOUT_AUTHOR_ID, None);
    wait_for_window(&mut app.client, "popout-settings", true);
    let detached_cloud = wait_for_author_id(
        &mut app.client,
        "popout-settings",
        CLOUD_CONSENT_STATUS_AUTHOR_ID,
        SURFACE_TIMEOUT,
    );
    assert!(
        node_text(require_node(
            &detached_cloud["snapshot"]["root"],
            CLOUD_CONSENT_STATUS_AUTHOR_ID,
        ))
        .contains("NOT WIRED"),
        "detached Settings lost the explicit consent posture"
    );
    let detached_cloud_shot = app.client.screenshot("popout-settings");
    let detached_cloud_png = decode_verified_capture(
        &detached_cloud_shot,
        "popout-settings",
        app.child_pid,
        "MT-021 Settings consent detached-window capture",
    );

    app.client.mutation_on_live_surface(
        "argus.set_value",
        "popout-settings",
        SETTINGS_SEARCH_AUTHOR_ID,
        Some(("value", serde_json::json!("concurrency"))),
    );
    let detached_concurrency = wait_for_author_id(
        &mut app.client,
        "popout-settings",
        SWARM_MODEL_SESSIONS_STATUS_AUTHOR_ID,
        SURFACE_TIMEOUT,
    );
    let detached_status = node_text(require_node(
        &detached_concurrency["snapshot"]["root"],
        SWARM_MODEL_SESSIONS_STATUS_AUTHOR_ID,
    ));
    assert!(
        detached_status.contains(&format!("Requested: {target}"))
            && detached_status.contains("In force:"),
        "detached Settings did not preserve live coordinator truth: `{detached_status}`"
    );
    let detached_concurrency_shot = app.client.screenshot("popout-settings");
    let detached_concurrency_png = decode_verified_capture(
        &detached_concurrency_shot,
        "popout-settings",
        app.child_pid,
        "MT-021 Settings concurrency detached-window capture",
    );
    app.client.mutation_on_live_surface(
        "argus.click",
        "popout-settings",
        SETTINGS_REDOCK_AUTHOR_ID,
        None,
    );
    wait_for_window(&mut app.client, "popout-settings", false);

    // Restore the persisted coordinator setting through the same operator/backend path used for
    // the proof. A live E2E must not leave the shared product runtime in a different policy state.
    app.client.mutation_on_live_surface(
        "argus.click",
        "main",
        SWARM_MODEL_SESSIONS_COMBO_AUTHOR_ID,
        None,
    );
    let original_option_id = swarm_model_session_option_author_id(original_requested);
    wait_for_author_id(
        &mut app.client,
        "main",
        &original_option_id,
        SURFACE_TIMEOUT,
    );
    app.client
        .mutation_on_live_surface("argus.click", "main", &original_option_id, None);
    let restore_deadline = std::time::Instant::now() + SURFACE_TIMEOUT;
    let restored_status = loop {
        let observed = app.client.poll_inspect("main");
        if let Some(node) = node_by_author_id(
            &observed["snapshot"]["root"],
            SWARM_MODEL_SESSIONS_STATUS_AUTHOR_ID,
        ) {
            let text = node_text(node);
            if text.contains(&format!("Requested: {original_requested}"))
                && text.contains("In force:")
                && !text.contains("loading")
                && !text.contains("unavailable")
            {
                break text;
            }
        }
        assert!(
            std::time::Instant::now() < restore_deadline,
            "live coordinator status never restored requested cap {original_requested}"
        );
        std::thread::sleep(std::time::Duration::from_millis(200));
    };
    app.client
        .mutation_on_live_surface("argus.click", "main", CLOSE_AUTHOR_ID, None);

    // Finally use the operator-facing MODELS leaf this MT added, then inspect/capture the console in
    // main and detached windows. Clicking its filter proves the console is steerable, not decorative.
    app.open_models_menu_leaf("menu.models.wp1-orchestration-console");
    let console = wait_for_author_id(
        &mut app.client,
        "main",
        FILTER_ALL_AUTHOR_ID,
        SURFACE_TIMEOUT,
    );
    let console_pane_id = pane_id_hosting(
        &console["snapshot"]["root"],
        &PaneType::Wp1OrchestrationConsole.label(),
    );
    let stream_deadline = std::time::Instant::now() + SURFACE_TIMEOUT;
    let stream_status = loop {
        let observed = app.client.poll_inspect("main");
        if let Some(status) = node_by_author_id(
            &observed["snapshot"]["root"],
            CONSOLE_STREAM_STATUS_AUTHOR_ID,
        ) {
            let text = node_text(status);
            if text.starts_with("Live stream connected") {
                break text;
            }
        }
        assert!(
            std::time::Instant::now() < stream_deadline,
            "WP-1 console never reported a connected production SSE stream"
        );
        std::thread::sleep(std::time::Duration::from_millis(200));
    };
    assert!(
        stream_status.contains("event sequence")
            || stream_status.contains("waiting for orchestration events"),
        "connected console must report an event cursor or an honest empty state: `{stream_status}`"
    );
    app.client
        .mutation_on_live_surface("argus.click", "main", FILTER_ALL_AUTHOR_ID, None);
    let console_main_shot = app.client.screenshot("main");
    let console_main_png = decode_verified_capture(
        &console_main_shot,
        "main",
        app.child_pid,
        "MT-021 Orchestration Console main-window capture",
    );
    let console_window = app.pop_out_pane(&console_pane_id);
    wait_for_author_id(
        &mut app.client,
        &console_window,
        FILTER_ALL_AUTHOR_ID,
        SURFACE_TIMEOUT,
    );
    let console_detached_shot = app.client.screenshot(&console_window);
    let console_detached_png = decode_verified_capture(
        &console_detached_shot,
        &console_window,
        app.child_pid,
        "MT-021 Orchestration Console detached-window capture",
    );
    app.merge_back_pane(&console_pane_id);

    for (name, bytes) in [
        (
            "argus_mt021_settings_concurrency_main.png",
            main_concurrency_png,
        ),
        ("argus_mt021_settings_consent_main.png", main_cloud_png),
        (
            "argus_mt021_settings_consent_detached.png",
            detached_cloud_png,
        ),
        (
            "argus_mt021_settings_concurrency_detached.png",
            detached_concurrency_png,
        ),
        ("argus_mt021_console_main.png", console_main_png),
        ("argus_mt021_console_detached.png", console_detached_png),
    ] {
        app.write_proof_artifact(name, &bytes);
    }
    let transcript = app.client.assert_transcript_is_secret_free(&[]);
    app.write_proof_artifact("argus_mt021_settings_console_transcript.json", &transcript);
    let provenance = serde_json::json!({
        "schema_id": "handshake.argus.mt021_production_provenance@1",
        "mt_id": "MT-021",
        "child_pid": app.child_pid,
        "authenticated_agent_id": app.authenticated_agent_id,
        "cold_optional_flight_recorder": {
            "measurement": "first operator navigation with two applied durable Argus receipts",
            "round_trip_ms": cold_navigation_receipt_round_trip_ms,
        },
        "navigation": [
            "menu-models",
            "menu.models.settings",
            "settings.popout",
            "settings.redock",
            "menu.models.wp1-orchestration-console",
            "ctx-menu.pane.pop_out",
        ],
        "coordinator": {
            "status_before": status_before,
            "original_requested": original_requested,
            "requested_target": target,
            "status_after": status_after,
            "detached_status": detached_status,
            "restored_status": restored_status,
        },
        "privacy": {
            "consent_summary": consent_summary,
            "explicit_unavailable": true,
            "authority_widening_action_offered": false,
            "restricted_metadata_markers_absent": true,
        },
        "captures": [
            "argus_mt021_settings_concurrency_main.png",
            "argus_mt021_settings_consent_main.png",
            "argus_mt021_settings_consent_detached.png",
            "argus_mt021_settings_concurrency_detached.png",
            "argus_mt021_console_main.png",
            "argus_mt021_console_detached.png",
        ],
    });
    app.write_proof_artifact(
        "argus_mt021_settings_console_provenance.json",
        &serde_json::to_vec_pretty(&provenance).expect("serialize MT-021 provenance"),
    );

    app.shutdown();
}
