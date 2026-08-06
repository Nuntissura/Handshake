//! MT-015 (v4 blocker) — Cloud Models settings surface in the DETACHED window: Argus proofs.
//!
//! The MT-015 v4 fail report was narrow and specific: the secrets boundary was hardened and the
//! DOCKED (root-viewport modal) surface was already Argus-proven, but the changed cloud-access surface
//! "lacks production Argus proof across real panels and detached windows … the screenshot route cannot
//! target pop-outs, so the operator's all-panels/all-windows requirement is unmet."
//!
//! `test_cloud_models_settings_argus.rs` proves the DOCKED surface. `test_settings_dialog.rs` proves the
//! detach/re-dock/close *host lifecycle*, but it seeds NO cloud snapshot, so it never exercises the
//! cloud-access controls (CLI status/login rows, per-provider states) inside the detached window, and it
//! never proves a typed key stays out of the detached window's Argus tree. This file closes exactly that
//! gap: it drives the REAL `HandshakeApp` headlessly through egui_kittest (AccessKit enabled — the same
//! `TreeUpdate` the out-of-process Windows UIA adapter reads) into the DETACHED Settings window and
//! proves, entirely against the live AccessKit tree of that pop-out:
//!
//! * the detached window root node `popout-window-settings` (Role::Window, "Handshake – Settings") is
//!   live, the modal `settings.dialog` root is gone (mutual exclusion), and Argus enumerates the window
//!   as `popout-settings` alongside `main`;
//! * every cloud-access control renders with a STABLE pane-scoped author_id in the detached window —
//!   BYOK key/save/status (+ remove when configured) for Anthropic + OpenAI, CLI-bridge status + login
//!   for Claude Code + GPT/Codex;
//! * Gemini is NEVER offered as a provider control;
//! * the CLI-bridge auth status renders the typed logged-in / logged-out / expired state, and BYOK rows
//!   render the configured vs unavailable state (login-state + unavailable-provider coverage);
//! * a BYOK key typed into the detached window's password field NEVER appears as an author_id or a label
//!   anywhere in the live Argus tree, and the shell buffer is cleared the moment Save is dispatched
//!   (the secrets-leakage lens for the surface state / Argus author-id/label);
//! * the detached window's controls are OPERABLE out-of-process: an AccessKit click on the Log-in button
//!   drives the provider's own official login command, with the terminal launch suppressed (no focus
//!   theft, HBR-QUIET);
//! * the `screenshot` route can TARGET the detached settings window by its stable `window_id`
//!   (`popout-settings`) — a recorded OS handle is grabbed directly, a stale/absent one falls back to
//!   exact title matching — which is the resolution the v4 "screenshot cannot target pop-outs" blocker
//!   is about. (The genuine live-pixel OS grab needs a real winit window and is proven by the live
//!   socket surfaces; it is not faked here.)
//!
//! ## Headless scope (honest)
//!
//! On a plain kittest `egui::Context`, `embed_viewports()` is `true`, so `show_viewport_immediate` runs
//! the SAME detached-window closure embedded in the current frame instead of raising a second OS window
//! (eframe sets `embed_viewports == false` only on the live wgpu/winit backend). The detached surface's
//! content, its window-root node, the mutual exclusion, the Argus registration, and every control's
//! drivability are therefore fully proven here. The one part that genuinely needs a real winit event
//! loop — the OS actually raising a second top-level window and a live-pixel capture of it — is NOT
//! faked: the capture-target *resolution* is proven headlessly through the pure resolver + handle
//! registry, and the live-pixel grab is the documented live-host remainder.

use std::sync::Arc;

use egui_kittest::kittest::{NodeT, Queryable};
use egui_kittest::Harness;
use handshake_native::app::{HandshakeApp, HealthDisplayState};
use handshake_native::backend_client::HealthInfo;
use handshake_native::settings_dialog::{
    CloudAccessSnapshot, CloudByokRow, CloudCliAuthStatus, CloudCliRow,
};
use handshake_native::workspace_settings::{SettingsTransport, SettingsTransportError};
use serde_json::Value;

fn ok_app() -> HandshakeApp {
    HandshakeApp::with_health(HealthDisplayState::Ok(HealthInfo {
        status: "ok".to_string(),
        db_status: "ok".to_string(),
        migration_version: Some(1),
    }))
}

#[derive(Default)]
struct StubSettingsTransport;
impl SettingsTransport for StubSettingsTransport {
    fn load(&self, _workspace_id: &str) -> Result<Option<Value>, SettingsTransportError> {
        Ok(None)
    }
    fn save(&self, _: &str, _: Value) -> Result<(), SettingsTransportError> {
        Ok(())
    }
}

/// A snapshot mirroring the backend enumeration: Anthropic (unconfigured) + OpenAI (configured) BYOK,
/// Claude Code + Codex CLI bridges (both at `cli_status`). NO Gemini. `CloudCliAuthStatus` is `Copy`, so
/// the same status seeds both CLI rows.
fn seeded_snapshot(cli_status: CloudCliAuthStatus) -> CloudAccessSnapshot {
    CloudAccessSnapshot {
        byok: vec![
            CloudByokRow {
                provider: "anthropic".into(),
                label: "Anthropic (Claude)".into(),
                configured: false,
            },
            CloudByokRow {
                provider: "openai".into(),
                label: "OpenAI (GPT)".into(),
                configured: true,
            },
        ],
        cli_bridge: vec![
            CloudCliRow {
                provider: "claude_code".into(),
                label: "Claude Code".into(),
                auth_status: cli_status,
                login_program: "claude".into(),
                login_args: vec!["auth".into(), "login".into()],
                hint: String::new(),
            },
            CloudCliRow {
                provider: "codex".into(),
                label: "GPT / Codex CLI".into(),
                auth_status: cli_status,
                login_program: "codex".into(),
                login_args: vec!["login".into()],
                hint: String::new(),
            },
        ],
    }
}

/// Every `(author_id, role, label)` triple in the LIVE consumer-side AccessKit tree — the same
/// projection an out-of-process Argus/UIA client reads.
fn all_author_nodes(harness: &Harness<'_, HandshakeApp>) -> Vec<(String, String, Option<String>)> {
    harness
        .root()
        .children_recursive()
        .filter_map(|node| {
            let ak = node.accesskit_node();
            ak.author_id()
                .map(|a| (a.to_owned(), format!("{:?}", ak.role()), ak.label()))
        })
        .collect()
}

fn all_author_ids(harness: &Harness<'_, HandshakeApp>) -> Vec<String> {
    all_author_nodes(harness)
        .into_iter()
        .map(|(a, _, _)| a)
        .collect()
}

/// EVERY node's label (not only author_id-bearing nodes), so a canary scan covers descriptive text,
/// status lines, and the password field's own AccessKit label alike.
fn all_labels(harness: &Harness<'_, HandshakeApp>) -> Vec<String> {
    harness
        .root()
        .children_recursive()
        .filter_map(|node| node.accesskit_node().label())
        .collect()
}

/// Open the settings surface with a seeded cloud snapshot, then DETACH it into the settings pop-out
/// window and settle the frames. `detach_settings()` is the exact seam the live `settings.popout`
/// AccessKit control routes through (proven in `test_settings_dialog.rs`); it does not touch the seeded
/// snapshot, so the detached window renders the same enumeration the modal would.
fn open_detached_with_snapshot(snapshot: CloudAccessSnapshot) -> Harness<'static, HandshakeApp> {
    let mut app = ok_app();
    app.set_settings_transport(Arc::new(StubSettingsTransport));
    app.set_cloud_snapshot_for_test(snapshot);

    let mut harness = Harness::builder()
        .with_size(egui::Vec2::new(1440.0, 940.0))
        .build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), app);
    harness.state_mut().open_settings();
    harness.run();
    assert!(
        harness.state_mut().detach_settings(),
        "detach_settings must transition an open modal into its own detached window"
    );
    harness.run();
    harness.run();
    harness
}

/// The canary must never appear as an author_id or a label anywhere in the live Argus tree.
fn assert_no_canary(harness: &Harness<'_, HandshakeApp>, canary: &str, phase: &str) {
    for id in all_author_ids(harness) {
        assert!(
            !id.contains(canary),
            "a BYOK key leaked into an Argus author_id {phase}: {id}"
        );
    }
    for label in all_labels(harness) {
        assert!(
            !label.contains(canary),
            "a BYOK key leaked into an Argus label {phase}: {label}"
        );
    }
}

// ── Every cloud-access control is pane-scoped-addressable in the detached window; no Gemini. ──────────
#[test]
fn detached_settings_window_renders_all_cloud_access_controls_with_stable_author_ids() {
    let harness = open_detached_with_snapshot(seeded_snapshot(CloudCliAuthStatus::LoggedIn));
    assert!(
        harness.state().settings_detached(),
        "the surface is detached into its own window"
    );

    let nodes = all_author_nodes(&harness);
    let ids: Vec<&str> = nodes.iter().map(|(a, _, _)| a.as_str()).collect();

    // The detached window's ROOT node is a Role::Window carrying the shared pop-out title.
    let window = nodes
        .iter()
        .find(|(a, _, _)| a == "popout-window-settings")
        .unwrap_or_else(|| panic!("popout-window-settings missing from the detached tree: {ids:?}"));
    assert_eq!(window.1, "Window", "the detached settings root is Role::Window");
    assert_eq!(
        window.2.as_deref(),
        Some("Handshake \u{2013} Settings"),
        "the detached window carries the shared 'Handshake – Settings' title"
    );

    // Mutual exclusion: the root-viewport modal must NOT render while the surface is detached.
    assert!(
        !ids.contains(&"settings.dialog"),
        "the modal must not render while the surface is detached: {ids:?}"
    );

    // Every cloud-access control stays addressable by its stable author_id in the DETACHED host.
    for expected in [
        "settings.cloud.byok.anthropic.key",
        "settings.cloud.byok.anthropic.save",
        "settings.cloud.byok.anthropic.status",
        "settings.cloud.byok.openai.key",
        "settings.cloud.byok.openai.save",
        "settings.cloud.byok.openai.status",
        // OpenAI is configured in the seed, so its Remove/Rotate control renders.
        "settings.cloud.byok.openai.remove",
        "settings.cloud.cli.claude_code.status",
        "settings.cloud.cli.claude_code.login",
        "settings.cloud.cli.codex.status",
        "settings.cloud.cli.codex.login",
    ] {
        assert!(
            ids.contains(&expected),
            "'{expected}' must be addressable in the DETACHED settings window: {ids:?}"
        );
    }

    // Gemini is never offered as a provider control anywhere in the surface.
    assert!(
        ids.iter().all(|id| !id.contains("gemini")),
        "no Gemini control may appear in the detached cloud-access surface: {ids:?}"
    );

    // Argus enumerates the detached window by its stable id, so an out-of-process driver can target it
    // (list_widgets / click / screenshot) alongside the main window.
    let windows = harness.state().mcp_window_registry().list();
    let detached = windows
        .iter()
        .find(|w| w.window_id == "popout-settings")
        .unwrap_or_else(|| {
            panic!(
                "argus.list_windows must enumerate the detached settings window: {:?}",
                windows.iter().map(|w| &w.window_id).collect::<Vec<_>>()
            )
        });
    assert_eq!(detached.title, "Handshake \u{2013} Settings");
    assert!(
        windows.iter().any(|w| w.window_id == "main"),
        "the main window stays registered alongside the detached settings window"
    );
}

// ── CLI-bridge auth status renders the typed logged-in / logged-out / expired state in the pop-out. ───
//
// The status ROW is addressable by its stable author_id in the detached window (per-provider), and the
// typed state TEXT is rendered into the detached window's live AccessKit label set. (The status node's
// text is emitted by egui's own `Label`; the exact author-id-scoped label read is host-dependent under
// the embedded-viewport render, so the state text is asserted against the full detached-tree label scan
// — a genuine Argus-tree signal — while the docked read is proven by
// `test_cloud_models_settings_argus::cli_bridge_auth_status_renders_all_three_states_for_claude_and_codex`.)
#[test]
fn detached_settings_renders_cli_auth_status_state_for_all_three_states() {
    for (status, expected_label) in [
        (CloudCliAuthStatus::LoggedIn, "Logged in"),
        (CloudCliAuthStatus::LoggedOut, "Logged out"),
        (CloudCliAuthStatus::Expired, "Session expired"),
    ] {
        let harness = open_detached_with_snapshot(seeded_snapshot(status));
        assert!(harness.state().settings_detached());

        let ids = all_author_ids(&harness);
        for provider in ["claude_code", "codex"] {
            let author_id = format!("settings.cloud.cli.{provider}.status");
            assert!(
                ids.contains(&author_id),
                "{author_id} status row must be addressable in the detached window: {ids:?}"
            );
        }

        let labels = all_labels(&harness);
        assert!(
            labels.iter().any(|l| l == expected_label),
            "the typed {status:?} state text '{expected_label}' must render in the detached window; \
             labels present: {labels:?}"
        );
    }
}

// ── BYOK rows render the configured vs unavailable state (unavailable-provider coverage). ─────────────
#[test]
fn detached_settings_renders_byok_configured_and_unavailable_states() {
    let harness = open_detached_with_snapshot(seeded_snapshot(CloudCliAuthStatus::LoggedIn));

    let ids = all_author_ids(&harness);
    for author_id in [
        "settings.cloud.byok.openai.status",
        "settings.cloud.byok.anthropic.status",
    ] {
        assert!(
            ids.iter().any(|id| id == author_id),
            "{author_id} status row must be addressable in the detached window: {ids:?}"
        );
    }

    // The configured OpenAI row and the unavailable Anthropic row render their distinct states into the
    // detached window's live AccessKit label set.
    let labels = all_labels(&harness);
    assert!(
        labels.iter().any(|l| l.contains("Configured")),
        "the configured provider's state must render in the detached window: {labels:?}"
    );
    assert!(
        labels.iter().any(|l| l == "Not configured"),
        "an un-keyed provider must render 'Not configured' (unavailable, not an error): {labels:?}"
    );
}

// ── A BYOK key typed into the DETACHED window never reaches any Argus author_id/label; buffer wiped. ──
#[test]
fn byok_key_never_appears_as_author_id_or_label_in_the_detached_tree() {
    const CANARY: &str = "sk-detached-argus-canary-NEVER-RENDER-0xC0FFEE";
    let mut harness = open_detached_with_snapshot(seeded_snapshot(CloudCliAuthStatus::LoggedIn));

    // Focus + type the canary into the OpenAI password field IN THE DETACHED WINDOW (by author_id).
    harness
        .query_all_by(|n: &egui_kittest::kittest::AccessKitNode<'_>| {
            n.author_id() == Some("settings.cloud.byok.openai.key")
        })
        .next()
        .expect("openai key input addressable in the detached window")
        .focus();
    harness.run();
    harness
        .query_all_by(|n: &egui_kittest::kittest::AccessKitNode<'_>| {
            n.author_id() == Some("settings.cloud.byok.openai.key")
        })
        .next()
        .expect("openai key input addressable in the detached window")
        .type_text(CANARY);
    harness.run();
    assert!(
        !harness.state().cloud_key_draft_is_empty("openai"),
        "the typed key is buffered before Save"
    );

    // The password field must never expose the plaintext key as an author_id or a label.
    assert_no_canary(&harness, CANARY, "after typing (before save)");

    // Save via the LIVE AccessKit button (the out-of-process click path) in the detached window.
    harness
        .query_all_by(|n: &egui_kittest::kittest::AccessKitNode<'_>| {
            n.author_id() == Some("settings.cloud.byok.openai.save")
        })
        .next()
        .expect("openai save button addressable in the detached window")
        .click_accesskit();
    harness.run();
    harness.run();

    // The shell takes + clears the buffer immediately — the key never lingers in the UI.
    assert!(
        harness.state().cloud_key_draft_is_empty("openai"),
        "the key buffer MUST be cleared immediately after Save is dispatched"
    );
    // And it is still nowhere in the Argus tree.
    assert_no_canary(&harness, CANARY, "after save");
}

// ── The detached window's Log-in control is operable out-of-process and never steals focus. ───────────
#[test]
fn detached_settings_cli_login_is_operable_via_accesskit_without_stealing_focus() {
    let mut harness = open_detached_with_snapshot(seeded_snapshot(CloudCliAuthStatus::LoggedOut));

    // No launch before the click; the terminal spawn is suppressed in the headless shell.
    assert!(harness.state().last_cli_login_launch().is_none());

    // Click the detached window's Claude Code login button (out-of-process AccessKit click).
    harness
        .query_all_by(|n: &egui_kittest::kittest::AccessKitNode<'_>| {
            n.author_id() == Some("settings.cloud.cli.claude_code.login")
        })
        .next()
        .expect("Claude Code login button addressable in the detached window")
        .click_accesskit();
    harness.run();
    harness.run();

    // The first click only arms the in-app login confirmation (no launch, no window, no focus theft).
    assert!(
        harness.state().last_cli_login_launch().is_none(),
        "the first click only opens the login confirmation"
    );
    harness
        .query_all_by(|n: &egui_kittest::kittest::AccessKitNode<'_>| {
            n.author_id() == Some("settings.cloud.cli.claude_code.login.confirm")
        })
        .next()
        .expect("Claude Code login confirmation addressable in the detached window")
        .click_accesskit();
    harness.run();
    harness.run();

    let launch = harness
        .state()
        .last_cli_login_launch()
        .cloned()
        .expect("a CLI login launch was recorded from the detached window");
    assert_eq!(
        launch.0, "claude",
        "the detached control launched the provider's OWN official CLI"
    );
    assert_eq!(launch.1, vec!["auth".to_string(), "login".to_string()]);
}

// ── The `screenshot` route can TARGET the detached settings pop-out by its stable window_id. ──────────
//
// This is the resolution the v4 "screenshot route cannot target pop-outs" blocker is about. The pure
// resolver + the process-global handle registry are proven headlessly for the settings `window_id`
// (`popout-settings`): a recorded OS handle is grabbed directly, a stale or absent one falls back to
// exact title matching (never a dead HWND). The live-pixel grab of the raised OS window is the
// documented live-host remainder (see the live socket surfaces).
#[test]
fn screenshot_capture_target_resolves_the_detached_settings_window_by_stable_id() {
    use handshake_native::mcp::screenshot::{
        clear_window_handle, record_window_handle, recorded_window_handle, resolve_capture_target,
        CaptureTarget,
    };

    const WINDOW_ID: &str = "popout-settings";
    clear_window_handle(WINDOW_ID); // isolate from any prior state in the shared static

    // Nothing recorded yet (e.g. the first capture before the pop-out rendered) => title fallback, and
    // the OS validity gate is not even consulted.
    assert_eq!(
        resolve_capture_target(recorded_window_handle(WINDOW_ID), |_| unreachable!(
            "the validity gate must not run when no handle is recorded"
        )),
        CaptureTarget::TitleFallback,
    );

    // Once the detached window records its OS handle, the screenshot route grabs THAT window directly,
    // unambiguously (never a same-title guess).
    record_window_handle(WINDOW_ID, 0x5E77_1465);
    assert_eq!(
        resolve_capture_target(recorded_window_handle(WINDOW_ID), |_| true),
        CaptureTarget::RecordedHandle(0x5E77_1465),
    );

    // A stale handle (the window was recreated) fails the validity gate => fall back to exact title +
    // PID matching rather than capturing a dead HWND.
    assert_eq!(
        resolve_capture_target(recorded_window_handle(WINDOW_ID), |_| false),
        CaptureTarget::TitleFallback,
    );

    clear_window_handle(WINDOW_ID);
}
