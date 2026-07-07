//! MT-015 — Cloud Models settings surface: Argus (out-of-process) proofs.
//!
//! Drives the REAL `HandshakeApp` headlessly via egui_kittest (AccessKit enabled — the same TreeUpdate
//! the out-of-process Windows UIA adapter reads) and proves the operator cloud-model access surface:
//!
//! * every BYOK + CLI-bridge control is addressable by a stable per-provider author_id in the LIVE tree;
//! * Gemini is NEVER offered (no `gemini*` author_id or provider row);
//! * a BYOK key typed into the password field is CLEARED from the shell buffer after Save is dispatched
//!   (the key never lingers in the UI — the security-critical native invariant);
//! * a CLI-bridge Log-in click records the provider's OWN official login command (operator-initiated),
//!   with the terminal launch suppressed so no console steals focus during the headless run.
//!
//! The backend leak proof (key stored only in the OS keychain, never in the settings PUT / logs / FR /
//! EventLedger / audit rows) is `handshake_core/tests/cloud_byok_access_config_leak_tests.rs`.

use std::sync::Arc;

use egui_kittest::kittest::{NodeT, Queryable};
use egui_kittest::Harness;
use handshake_native::app::{HandshakeApp, HealthDisplayState};
use handshake_native::backend_client::HealthInfo;
use handshake_native::settings_dialog::{
    cloud_byok_key_egui_id, CloudAccessSnapshot, CloudByokRow, CloudCliRow,
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
/// Claude Code + Codex CLI bridges. NO Gemini.
fn seeded_snapshot() -> CloudAccessSnapshot {
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
                login_program: "claude".into(),
                login_args: vec!["/login".into()],
                hint: String::new(),
            },
            CloudCliRow {
                provider: "codex".into(),
                label: "GPT / Codex CLI".into(),
                login_program: "codex".into(),
                login_args: vec!["login".into()],
                hint: String::new(),
            },
        ],
    }
}

fn all_author_ids(harness: &Harness<'_, HandshakeApp>) -> Vec<String> {
    harness
        .root()
        .children_recursive()
        .filter_map(|node| node.accesskit_node().author_id().map(str::to_owned))
        .collect()
}

fn open_with_snapshot() -> Harness<'static, HandshakeApp> {
    let mut app = ok_app();
    app.set_settings_transport(Arc::new(StubSettingsTransport));
    app.set_cloud_snapshot_for_test(seeded_snapshot());

    let mut harness =
        Harness::builder().build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), app);
    harness.state_mut().open_settings();
    harness.run();
    harness.run();
    harness
}

/// Open the settings dialog with NO backend enumeration seeded and NO cloud-access client
/// (`with_health` leaves `cloud_access_client = None`), so the Cloud Models section must fall back to
/// the static seed rows (F10) rather than a backend enumeration.
fn open_without_snapshot() -> Harness<'static, HandshakeApp> {
    let mut app = ok_app();
    app.set_settings_transport(Arc::new(StubSettingsTransport));
    // Deliberately NO set_cloud_snapshot_for_test: the snapshot stays empty and no client can fetch one.

    let mut harness =
        Harness::builder().build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), app);
    harness.state_mut().open_settings();
    harness.run();
    harness.run();
    harness
}

// ── Every provider control is addressable + Gemini is never offered. ────────────────────────────────
#[test]
fn cloud_models_controls_are_addressable_and_gemini_is_never_offered() {
    let harness = open_with_snapshot();
    let ids = all_author_ids(&harness);

    for expected in [
        "settings.cloud.byok.anthropic.key",
        "settings.cloud.byok.anthropic.save",
        "settings.cloud.byok.openai.key",
        "settings.cloud.byok.openai.save",
        "settings.cloud.byok.openai.remove",
        "settings.cloud.cli.claude_code.login",
        "settings.cloud.cli.codex.login",
    ] {
        assert!(
            ids.iter().any(|id| id == expected),
            "author_id '{expected}' must be addressable in the live settings tree: {ids:?}"
        );
    }

    // Gemini is never offered anywhere in the surface.
    assert!(
        ids.iter().all(|id| !id.contains("gemini")),
        "no Gemini control may appear in the cloud models surface: {ids:?}"
    );
}

// ── A typed BYOK key is CLEARED from the shell buffer after Save is dispatched. ──────────────────────
#[test]
fn typing_and_saving_a_byok_key_clears_the_ui_buffer() {
    let mut harness = open_with_snapshot();

    // The buffer starts empty.
    assert!(harness.state().cloud_key_draft_is_empty("openai"));

    // Focus + type a key into the OpenAI password field (addressed by author_id).
    harness
        .query_all_by(|n: &egui_kittest::kittest::AccessKitNode<'_>| {
            n.author_id() == Some("settings.cloud.byok.openai.key")
        })
        .next()
        .expect("openai key input addressable")
        .focus();
    harness.run();
    harness
        .query_all_by(|n: &egui_kittest::kittest::AccessKitNode<'_>| {
            n.author_id() == Some("settings.cloud.byok.openai.key")
        })
        .next()
        .expect("openai key input addressable")
        .type_text("sk-argus-canary-key");
    harness.run();
    assert!(
        !harness.state().cloud_key_draft_is_empty("openai"),
        "the typed key is buffered before Save"
    );
    // The Save button IS addressable in the live tree (Argus can find + click it out-of-process).
    assert!(
        all_author_ids(&harness)
            .iter()
            .any(|id| id == "settings.cloud.byok.openai.save"),
        "openai save button is addressable"
    );

    // Click the LIVE Save button by its stable author_id through AccessKit so the proof exercises
    // `save.clicked()` inside `render_cloud_models_body`, then `drive_settings_dialog` applies the
    // emitted outcome. The shell takes + clears the buffer immediately — the security invariant is that
    // the key never lingers in the UI. (No backend in this headless shell, so the store itself reports
    // unreachable, but the buffer is cleared regardless.)
    harness
        .query_all_by(|n: &egui_kittest::kittest::AccessKitNode<'_>| {
            n.author_id() == Some("settings.cloud.byok.openai.save")
        })
        .next()
        .expect("openai save button addressable")
        .click_accesskit();
    harness.run();
    harness.run();

    assert!(
        harness.state().cloud_key_draft_is_empty("openai"),
        "the key buffer MUST be cleared immediately after Save is dispatched"
    );
    // A status message is surfaced (no backend here, so it reports unreachable — but a message exists).
    assert!(
        harness.state().cloud_models().message("openai").is_some(),
        "a status message is shown after a save attempt"
    );
}

// ── A CLI-bridge Log-in records the provider's OWN official login command (operator-initiated). ──────
//
// This exercises the live button's click -> outcome -> launch wiring through `login.clicked()` and
// `drive_settings_dialog`. The terminal spawn stays suppressed (with_health default) so no console
// steals focus.
#[test]
fn cli_bridge_login_records_the_official_command_without_stealing_focus() {
    let mut harness = open_with_snapshot();
    // The login button IS addressable in the live tree (Argus can find + click it out-of-process).
    let ids = all_author_ids(&harness);
    assert!(ids
        .iter()
        .any(|id| id == "settings.cloud.cli.claude_code.login"));

    // No launch recorded before the click; the terminal spawn is suppressed in the headless shell.
    assert!(harness.state().last_cli_login_launch().is_none());

    harness
        .query_all_by(|n: &egui_kittest::kittest::AccessKitNode<'_>| {
            n.author_id() == Some("settings.cloud.cli.claude_code.login")
        })
        .next()
        .expect("Claude Code login button addressable")
        .click_accesskit();
    harness.run();
    harness.run();

    let launch = harness
        .state()
        .last_cli_login_launch()
        .cloned()
        .expect("a CLI login launch was recorded");
    assert_eq!(
        launch.0, "claude",
        "launched the provider's OWN official CLI"
    );
    assert_eq!(launch.1, vec!["/login".to_string()]);
}

// ── F10: BYOK key entry renders even when the backend enumeration is unreachable. ────────────────────
//
// Before the fix, `render_cloud_models_body` gated ALL key-entry rows on a non-empty backend snapshot,
// so with `cloud_access_client = None` (backend unreachable) it showed only a "Loading… backend not
// reachable" note and NO fields — an operator could never enter a BYOK key offline. The section now
// seeds the STATIC provider rows client-side, so the key field + Save ALWAYS render.
#[test]
fn cloud_models_key_entry_renders_when_backend_unreachable() {
    let harness = open_without_snapshot();
    let ids = all_author_ids(&harness);

    for expected in [
        "settings.cloud.byok.anthropic.key",
        "settings.cloud.byok.anthropic.save",
        "settings.cloud.byok.openai.key",
        "settings.cloud.byok.openai.save",
    ] {
        assert!(
            ids.iter().any(|id| id == expected),
            "static seed row '{expected}' must render when the backend is unreachable: {ids:?}"
        );
    }
    // Gemini is still never offered, even in the static seed.
    assert!(
        ids.iter().all(|id| !id.contains("gemini")),
        "no Gemini control may appear in the static seed rows: {ids:?}"
    );
}

// ── F3: a typed-but-unsaved BYOK key never lingers in egui memory across a dialog close. ─────────────
//
// The key edit buffer is a shell-owned `Zeroizing<String>`, and each key `TextEdit` keeps its own egui
// state (cursor + undo history) in egui memory. On close the shell buffer is wiped AND each key widget's
// egui state is reset, so neither the buffer nor the undo history (which snapshots the plaintext text)
// retains the key. This test types a canary, deterministically seeds the undo history with the canary
// (simulating egui's own undo checkpoint), closes the dialog, then proves the canary is ABSENT from the
// shell buffer, from the SERIALIZED persisted egui memory, and from the key widget's reset undo history.
#[test]
fn typed_byok_key_is_wiped_from_egui_memory_after_close() {
    const CANARY: &str = "sk-egui-mem-canary-NEVER-PERSIST-0xC0FFEE";
    let mut harness = open_with_snapshot();

    // Focus + type the canary into the OpenAI password field (addressed by author_id).
    harness
        .query_all_by(|n: &egui_kittest::kittest::AccessKitNode<'_>| {
            n.author_id() == Some("settings.cloud.byok.openai.key")
        })
        .next()
        .expect("openai key input addressable")
        .focus();
    harness.run();
    harness
        .query_all_by(|n: &egui_kittest::kittest::AccessKitNode<'_>| {
            n.author_id() == Some("settings.cloud.byok.openai.key")
        })
        .next()
        .expect("openai key input addressable")
        .type_text(CANARY);
    harness.run();
    assert!(
        !harness.state().cloud_key_draft_is_empty("openai"),
        "the typed key is buffered before close"
    );

    // Deterministically seed the key widget's egui undo history with the canary, mirroring the plaintext
    // snapshot egui itself keeps on edit. This makes the reset assertion below independent of undo-timing.
    let key_id = cloud_byok_key_egui_id("openai");
    {
        let mut state = egui::TextEdit::load_state(&harness.ctx, key_id).unwrap_or_default();
        let mut undoer = state.undoer();
        undoer.add_undo(&(egui::text::CCursorRange::default(), CANARY.to_string()));
        state.set_undoer(undoer);
        egui::TextEdit::store_state(&harness.ctx, key_id, state);
    }
    // Precondition: the persisted undo history really does hold the canary now.
    let before = egui::TextEdit::load_state(&harness.ctx, key_id).expect("key state present");
    assert!(
        serde_json::to_string(&before.undoer())
            .unwrap()
            .contains(CANARY),
        "precondition: the undo history holds the canary before close"
    );

    // Close the dialog via the Close button (a real dismiss path that runs `show()`).
    harness
        .query_all_by(|n: &egui_kittest::kittest::AccessKitNode<'_>| {
            n.author_id() == Some("settings.close")
        })
        .next()
        .expect("close button addressable")
        .click();
    harness.run();
    harness.run();

    // (1) The shell key buffer is wiped on close.
    assert!(
        harness.state().cloud_key_draft_is_empty("openai"),
        "the key buffer MUST be cleared on close"
    );

    // (2) The canary is absent from the SERIALIZED persisted egui memory (the IdTypeMap that would be
    // written to disk — DialogState and any persisted widget state live here). The app does NOT override
    // eframe::App::save, so this is the entire persisted footprint.
    let mem_json = harness
        .ctx
        .data(|d| serde_json::to_string(d).expect("serialize egui persisted memory"));
    assert!(
        !mem_json.contains(CANARY),
        "the key leaked into persisted egui memory: {mem_json}"
    );

    // (3) The key widget's egui state was reset: its undo history no longer snapshots the plaintext key.
    if let Some(state) = egui::TextEdit::load_state(&harness.ctx, key_id) {
        let undoer_json = serde_json::to_string(&state.undoer()).unwrap_or_default();
        assert!(
            !undoer_json.contains(CANARY),
            "the TextEdit undo history retained the key after close: {undoer_json}"
        );
    }
}
