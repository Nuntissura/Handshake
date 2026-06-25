//! WP-KERNEL-012 MT-070 (E11 — melt-together click-through): LIVE-shell proofs for the NavigationTarget
//! bus.
//!
//! These proofs drive the REAL `HandshakeApp` shell as the [`ShellNavigator`] (the same trait the quick
//! switcher and the MT-079 host-mount use) THROUGH the MT-070 [`navigation_bus`] layer, proving the thin
//! NavigationTarget layer DELEGATES to the existing nav substrate (RISK-070-4 — no forked bus) and routes
//! a target into the correct editor/pane addressed by a STABLE pane id (AC-070-3), with a typed
//! `PaneNotFound` guard (AC-070-6 / RISK-070-3) instead of a panic.
//!
//! - PT-070-4 / AC-070-3: `quick_switcher_target_routes_into_correct_pane_by_id` resolves a
//!   quick-switcher document hit to a NavigationTarget and dispatches it through `navigation_bus::dispatch`
//!   on the LIVE shell, asserting the focused/active pane id equals the target editor pane id (the shell's
//!   `open_navigator_tab` set `active_pane`), i.e. the bus routed the selection into the right pane.
//! - AC-070-6 (Ok): `code_symbol_target_opens_mounted_code_pane` dispatches an `EditorAtSymbol` target and
//!   asserts `Ok(())` (the MT-079 mounted code pane opened) — the bus delegates to the real open_* seam.
//! - AC-070-6 (Err): `unknown_pane_focus_returns_pane_not_found_never_panics` dispatches a `FocusPane` /
//!   `RevealNode` for an id no pane owns on a shell with NO tab-bar panes and asserts
//!   `Err(NavError::PaneNotFound)` — a typed guard, never a frame-thread panic (RISK-070-3 / MC-070-3).

use handshake_native::app::{HandshakeApp, HealthDisplayState, DEFAULT_PROJECT_ID};
use handshake_native::backend_client::HealthInfo;
use handshake_native::navigation_bus::{dispatch, NavError, NavigationTarget};
use handshake_native::pane_registry::{
    DirtyState, LockState, PaneAuthority, PaneId, PaneRecord, PaneType,
};
use handshake_native::quick_switcher::{resolve_open_target, LoomGraphSearchHit, QuickSwitcherTarget};

/// A live shell with the seeded 2x2 panes re-typed so the top-left slot hosts the code editor and the
/// top-right slot hosts the Notes/rich editor — the two MT-079-mounted editor surfaces a NavigationTarget
/// lands on. Mirrors the `test_app_host_mount.rs::editor_shell` setup so the bus dispatch hits the SAME
/// real mounted panes.
fn editor_shell() -> HandshakeApp {
    let app = HandshakeApp::with_health(HealthDisplayState::Ok(HealthInfo {
        status: "ok".to_string(),
        db_status: "ok".to_string(),
        migration_version: Some(1),
    }));
    let registry = app.pane_registry();
    let mut guard = registry.lock().expect("registry");
    guard.insert(PaneRecord::new(
        PaneId::from("pane-a"),
        PaneType::CodeSymbol,
        DEFAULT_PROJECT_ID,
        None,
        LockState::Unlocked,
        DirtyState::Clean,
        PaneAuthority::System,
    ));
    guard.insert(PaneRecord::new(
        PaneId::from("pane-b"),
        PaneType::LoomWikiPage,
        DEFAULT_PROJECT_ID,
        None,
        LockState::Unlocked,
        DirtyState::Clean,
        PaneAuthority::System,
    ));
    drop(guard);
    app
}

/// Build a `document` quick-switcher hit for `document_id` (a `KRD-`-prefixed rich-doc id), the exact hit
/// shape `resolve_open_target` maps to a `QuickSwitcherTarget::Document`.
fn document_hit(document_id: &str) -> LoomGraphSearchHit {
    LoomGraphSearchHit {
        result_kind: "knowledge_entity".to_owned(),
        source_kind: "document".to_owned(),
        ref_id: document_id.to_owned(),
        title: "Doc".to_owned(),
        excerpt: String::new(),
        block: serde_json::json!({}),
        score: 1.0,
        metadata: serde_json::json!({ "rich_document_id": document_id }),
    }
}

// ── PT-070-4 / AC-070-3: a quick-switcher selection routes into the correct pane by stable id ──────────

#[test]
fn quick_switcher_target_routes_into_correct_pane_by_id() {
    let mut app = editor_shell();

    // The shell seeds pane-a..pane-d with tab bars; `module_target_pane` (the shell's nav landing pane)
    // is deterministically the lowest tab-bar pane id, "pane-a" (the re-typed code editor). Resolve the
    // MT-030 quick-switcher DOCUMENT hit to its typed target, then build the NavigationTarget the
    // quick-switcher confirm path dispatches.
    let hit = document_hit("KRD-routed-1");
    let qs_target = resolve_open_target(&hit);
    assert_eq!(
        qs_target,
        QuickSwitcherTarget::Document { document_id: "KRD-routed-1".to_owned() },
        "the document hit resolves to a Document target (the quick-switcher selection)",
    );
    let QuickSwitcherTarget::Document { document_id } = qs_target else {
        panic!("expected a Document target");
    };

    // Dispatch the OpenNote NavigationTarget through the MT-070 bus on the LIVE shell. The bus delegates
    // to ShellNavigator::open_document, which (MT-079) opens + FOCUSES the mounted Notes editor pane.
    let target = NavigationTarget::OpenNote { note_id: document_id };
    let result = dispatch(&mut app, &target);
    assert_eq!(result, Ok(()), "the bus routed the quick-switcher selection onto a real mounted pane");

    // AC-070-3: the bus routed the selection INTO the correct pane addressed by a STABLE id — the shell's
    // `open_navigator_tab` set `active_pane` to the landing pane id. Assert the focused pane id equals the
    // deterministic target pane id (the lowest tab-bar pane, "pane-a").
    assert_eq!(
        app.active_pane().map(|p| p.as_ref()),
        Some("pane-a"),
        "the focused/active pane id after dispatch equals the target pane id (routed by stable id)",
    );
}

// ── AC-070-6 (Ok path): an EditorAtSymbol target opens the mounted code pane via the real seam ─────────

#[test]
fn code_symbol_target_opens_mounted_code_pane() {
    let mut app = editor_shell();

    let target = NavigationTarget::EditorAtSymbol {
        pane_id: PaneId::from("pane-a"),
        symbol: "sym-entity-1".to_owned(),
    };
    let result = dispatch(&mut app, &target);
    assert_eq!(
        result,
        Ok(()),
        "EditorAtSymbol delegates to the MT-079-mounted code pane via ShellNavigator::open_code_symbol",
    );
    assert_eq!(
        app.active_pane().map(|p| p.as_ref()),
        Some("pane-a"),
        "the code-symbol target focused the code pane by stable id",
    );
}

// ── AC-070-6 (Err path) / RISK-070-3 / MC-070-3: unknown pane -> typed PaneNotFound, never a panic ─────

#[test]
fn unknown_pane_focus_returns_pane_not_found_never_panics() {
    // A shell with NO tab-bar panes at all: `module_target_pane` returns None, so every open_* seam
    // returns `NoTargetPane`, which the bus maps to the typed `PaneNotFound` (never a panic). This is the
    // "stale/closed pane id" + "empty work surface" path the red-team requires to be guarded.
    let mut app = HandshakeApp::with_health(HealthDisplayState::Ok(HealthInfo {
        status: "ok".to_string(),
        db_status: "ok".to_string(),
        migration_version: Some(1),
    }));
    // Drain every tab bar so there is no landing pane (simulate a fully-closed work surface): with no
    // tab-bar pane, the shell's `module_target_pane` returns None and every open_* seam yields
    // `NoTargetPane`, which the bus maps to the typed `PaneNotFound` guard.
    app.tab_bar_states_mut().clear();

    let focus_err = dispatch(
        &mut app,
        &NavigationTarget::FocusPane { pane_id: PaneId::from("pane-closed") },
    );
    assert!(
        matches!(&focus_err, Err(NavError::PaneNotFound { .. })),
        "FocusPane on a work surface with no landing pane -> PaneNotFound, got {focus_err:?}",
    );

    let reveal_err = dispatch(
        &mut app,
        &NavigationTarget::RevealNode {
            pane_id: PaneId::from("pane-closed"),
            node_id: "blk-stale".to_owned(),
        },
    );
    assert!(
        matches!(&reveal_err, Err(NavError::PaneNotFound { .. })),
        "RevealNode on a closed pane -> PaneNotFound, got {reveal_err:?}",
    );
}
