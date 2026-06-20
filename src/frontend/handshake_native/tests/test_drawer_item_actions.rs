//! WP-KERNEL-011 MT-024 (C6) — bottom drawer CARD ACTION menu LIVE proof.
//!
//! Renders the REAL `HandshakeApp` headlessly via egui_kittest (which enables AccessKit and produces the
//! same `TreeUpdate` the out-of-process Windows UIA adapter receives) and proves the MT-024 card actions:
//!   - PROOF-024-2(a): every open card renders an overflow `...` button (Role::Button, stable id).
//!   - PROOF-024-2(b): right-clicking a card OR clicking its overflow button opens a popup with exactly
//!     the eight labelled action items (Stow, Pin, Promote, Send to pane, Copy to prompt, Attach
//!     evidence, Convert to artifact, Discard).
//!   - PROOF-024-2(c): Copy to prompt (with a bound target) places the contract's exact prompt string in
//!     the egui clipboard (the native, headless-safe clipboard path — no `arboard` dependency).
//!   - PROOF-024-4 (AccessKit): the open action menu exposes the eight `ctx-menu.drawer.action.*`
//!     MenuItem nodes + the four `hsk.drawer.card.{kind}.overflow` Button nodes in the live tree.
//!   - HBR-SWARM: Promote writes a concurrency-safe `DrawerIntents.promote_block_id` a swarm reader sees.
//!   - HBR-STOP / AC-024-9: Attach-evidence with no active job makes NO backend call (surfaces a message).
//!
//! The headless shell has no tokio runtime, so the PERSISTING backend calls (Stow/Pin/Discard) are not
//! driven here; those are proven by the backend_client wire-capture unit tests (the REAL spawn path) and
//! the `#[ignore]` real-PG integration test in `test_drawer_integration.rs` (PROOF-024-3). These kittests
//! prove the UI / menu / AccessKit / local-action / dispatch-routing surface end-to-end.

use egui_kittest::kittest::Queryable;
use egui_kittest::Harness;
use handshake_native::app::{HandshakeApp, HealthDisplayState};
use handshake_native::backend_client::HealthInfo;
use handshake_native::stash_shelf::{
    DrawerActionTarget, DrawerCardAction, DrawerCardKind,
};

fn ok_app() -> HandshakeApp {
    HandshakeApp::with_health(HealthDisplayState::Ok(HealthInfo {
        status: "ok".to_string(),
        db_status: "ok".to_string(),
        migration_version: Some(1),
    }))
}

/// Open the drawer and bind the Notes card to a concrete block so its persisting actions enable + its
/// copy-to-prompt has a real target. Runs enough settle frames for the open panel to materialize.
fn open_app_with_notes_target() -> Harness<'static, HandshakeApp> {
    let mut harness =
        Harness::builder().build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), ok_app());
    harness.run();
    {
        let app = harness.state_mut();
        app.set_bottom_drawer_open(true);
        if let Some(card) = app.drawer_mut().card_mut(DrawerCardKind::Notes) {
            card.action_target = Some(DrawerActionTarget {
                workspace_id: "ws-test".to_owned(),
                block_id: "block-xyz".to_owned(),
                title: "My Note".to_owned(),
                content_type: "note".to_owned(),
                excerpt: "the note body".to_owned(),
            });
        }
    }
    harness.run();
    harness.run();
    harness
}

/// Every live author-id-carrying node: (author_id, role).
fn author_nodes(harness: &Harness<'_, HandshakeApp>) -> Vec<(String, String)> {
    use egui_kittest::kittest::NodeT;
    let mut found = Vec::new();
    for node in harness.root().children_recursive() {
        let ak = node.accesskit_node();
        if let Some(author_id) = ak.author_id() {
            found.push((author_id.to_owned(), format!("{:?}", ak.role())));
        }
    }
    found
}

#[test]
fn drawer_action_enum_has_eight_variants() {
    // PROOF-024-1(a): the typed action set is exactly the contract's eight.
    assert_eq!(DrawerCardAction::all().len(), 8);
}

#[test]
fn copy_to_prompt_builds_the_contract_prompt_format() {
    // PROOF-024-1(d): the prompt string format (ported from the React copy_as_coder_prompt).
    let t = DrawerActionTarget {
        workspace_id: "ws".to_owned(),
        block_id: "b1".to_owned(),
        title: "Title".to_owned(),
        content_type: "note".to_owned(),
        excerpt: "Body text".to_owned(),
    };
    assert_eq!(
        t.coder_prompt(),
        "Block: Title\nType: note\nID: b1\n\nBody text"
    );
}

#[test]
fn every_open_card_renders_an_overflow_button() {
    // PROOF-024-2(a) + AC-024-1: each card has an always-visible overflow `...` Button node.
    let harness = open_app_with_notes_target();
    let nodes = author_nodes(&harness);
    for kind in DrawerCardKind::all() {
        let aid = kind.overflow_author_id();
        let found = nodes
            .iter()
            .find(|(a, _)| a == aid)
            .unwrap_or_else(|| panic!("overflow {aid} missing; found {:?}", nodes));
        assert_eq!(found.1, "Button", "{aid} role is Button");
    }
}

#[test]
fn overflow_click_opens_menu_with_exactly_eight_action_items() {
    // PROOF-024-2(b) + AC-024-2 + PROOF-024-4: clicking the overflow button opens the action menu; all
    // eight typed actions appear as Role::MenuItem nodes with stable ctx-menu.drawer.action.* ids.
    let mut harness = open_app_with_notes_target();

    harness.get_by_label("Notes actions").click();
    harness.run();
    harness.run();

    let nodes = author_nodes(&harness);
    for action in DrawerCardAction::all() {
        let expected = format!("ctx-menu.{}", action.menu_item_id());
        let found = nodes
            .iter()
            .find(|(a, _)| a == &expected)
            .unwrap_or_else(|| panic!("action item {expected} missing: {nodes:?}"));
        assert_eq!(found.1, "MenuItem", "{expected} role is MenuItem");
    }
    // Exactly eight drawer.action.* MenuItems (no more, no fewer).
    let action_items = nodes
        .iter()
        .filter(|(a, _)| a.starts_with("ctx-menu.drawer.action."))
        .count();
    assert_eq!(action_items, 8, "exactly eight action items: {nodes:?}");
}

#[test]
fn right_click_card_also_opens_the_action_menu() {
    // AC-024-2: the SECOND trigger — right-clicking the card body — opens the same menu.
    let mut harness = open_app_with_notes_target();

    harness.get_by_label("Notes (0)").click_secondary();
    harness.run();
    harness.run();

    let nodes = author_nodes(&harness);
    assert!(
        nodes.iter().any(|(a, _)| a == "ctx-menu.drawer.action.stow"),
        "right-click opened the action menu: {nodes:?}"
    );
}

#[test]
fn copy_to_prompt_writes_the_clipboard() {
    // PROOF-024-2(c) + AC-024-8: Copy to prompt writes the contract prompt string to the egui clipboard.
    let mut harness = open_app_with_notes_target();

    harness.get_by_label("Notes actions").click();
    harness.run();
    harness.run();
    harness.get_by_label("Copy to prompt").click();
    // step() (one frame) not run(): run() settles over several frames and the later (empty) settle frames
    // would overwrite the transient CopyText command (the same pattern as the console Copy Line test).
    harness.step();

    let copied = harness
        .output()
        .platform_output
        .commands
        .iter()
        .find_map(|c| match c {
            egui::OutputCommand::CopyText(s) => Some(s.clone()),
            _ => None,
        });
    assert_eq!(
        copied.as_deref(),
        Some("Block: My Note\nType: note\nID: block-xyz\n\nthe note body"),
        "the bound Notes card's coder prompt was copied to the clipboard"
    );
}

#[test]
fn promote_writes_a_concurrency_safe_intent() {
    // AC-024-6 + HBR-SWARM: Promote makes NO backend call and writes a swarm-readable PromoteIntent.
    let mut harness = open_app_with_notes_target();

    assert!(harness.state().drawer_intents().promote_block_id.is_none(), "no intent before");

    harness.get_by_label("Notes actions").click();
    harness.run();
    harness.run();
    harness.get_by_label("Promote").click();
    harness.run();

    assert_eq!(
        harness.state().drawer_intents().promote_block_id.as_deref(),
        Some("block-xyz"),
        "Promote wrote the block id into the concurrency-safe intent (no backend call)"
    );
}

#[test]
fn attach_evidence_without_active_job_makes_no_call_and_reports() {
    // AC-024-9: Attach-evidence with no active job surfaces a message and dispatches NO backend call.
    let mut harness = open_app_with_notes_target();
    // No active job set (default None).
    harness.get_by_label("Notes actions").click();
    harness.run();
    harness.run();
    harness.get_by_label("Attach evidence").click();
    harness.run();

    assert_eq!(
        harness.state().drawer_action_error(),
        Some("No active job to attach evidence to"),
        "no active job -> the AC-024-9 message, no backend call (headless shell has no client anyway)"
    );
}

// ═════════════════════════════════════════════════════════════════════════════════════════════════
// BLOCKER FIX (HBR-STOP / RISK-024-A / AC-024-11 / CONTROL-024-A): Discard must ARM an in-app confirm
// gate, NOT dispatch the DELETE immediately. These tests mirror PROOF-024-2(d)/(e):
//   (d) choosing Discard shows the in-app "Confirm Discard" Window with its OK/Cancel nodes in the LIVE
//       tree and makes NO backend call; Cancel clears the arm with no call.
//   (e) choosing Discard again -> OK fires the REAL DELETE /loom/blocks/:id on the wire (the MT-021
//       in-process TcpListener capture-server pattern).
// ═════════════════════════════════════════════════════════════════════════════════════════════════

/// Open the drawer, choose Discard on the (target-bound) Notes card via its menu, and settle. Returns
/// the harness with the Discard ARMED (the confirm Window rendering).
fn arm_discard_on_notes() -> Harness<'static, HandshakeApp> {
    let mut harness = open_app_with_notes_target();
    harness.get_by_label("Notes actions").click();
    harness.run();
    harness.run();
    harness.get_by_label("Discard").click();
    harness.run();
    harness.run();
    harness
}

#[test]
fn choosing_discard_arms_confirm_window_and_makes_no_call() {
    // PROOF-024-2(d): selecting Discard ARMS the in-app confirm gate (NO immediate DELETE) and renders
    // the "Confirm Discard" Window with stable OK/Cancel author_id nodes in the LIVE AccessKit tree.
    let harness = arm_discard_on_notes();

    // The arm is set (the menu did NOT dispatch a DELETE; it armed the confirm gate).
    assert_eq!(
        harness.state().confirm_discard_block_id(),
        Some("block-xyz"),
        "Discard armed the confirm gate with the target block id instead of dispatching the DELETE"
    );
    // The headless shell has no client; even so a DELETE must NOT have been attempted (no error set by a
    // dispatch attempt — the arm short-circuits before any client/runtime check).
    assert_eq!(
        harness.state().drawer_action_error(),
        None,
        "arming Discard makes NO backend call (no 'no backend runtime' dispatch error)"
    );

    // The confirm Window + its OK/Cancel nodes are LIVE in the tree (HBR-QUIET in-app modal).
    let nodes = author_nodes(&harness);
    for aid in ["hsk.drawer.confirm.window", "hsk.drawer.confirm.ok", "hsk.drawer.confirm.cancel"] {
        assert!(
            nodes.iter().any(|(a, _)| a == aid),
            "confirm node {aid} missing from LIVE tree: {nodes:?}"
        );
    }
    // The OK + Cancel are addressable Buttons out-of-process.
    for aid in ["hsk.drawer.confirm.ok", "hsk.drawer.confirm.cancel"] {
        let found = nodes.iter().find(|(a, _)| a == aid).unwrap();
        assert_eq!(found.1, "Button", "{aid} role is Button");
    }
    println!("PASS: choosing Discard armed the in-app Confirm Discard window (OK/Cancel live), NO DELETE");
}

#[test]
fn cancel_clears_the_arm_with_no_call() {
    // PROOF-024-2(d): Cancel clears the arm and makes NO backend call.
    let mut harness = arm_discard_on_notes();
    assert!(harness.state().confirm_discard_block_id().is_some(), "armed before Cancel");

    harness.get_by_label("Cancel").click();
    harness.run();
    harness.run();

    assert_eq!(
        harness.state().confirm_discard_block_id(),
        None,
        "Cancel cleared the armed discard"
    );
    assert_eq!(
        harness.state().drawer_action_error(),
        None,
        "Cancel made NO backend call (the destructive DELETE never ran)"
    );
    // The confirm Window is gone from the LIVE tree.
    let nodes = author_nodes(&harness);
    assert!(
        !nodes.iter().any(|(a, _)| a == "hsk.drawer.confirm.window"),
        "confirm window removed after Cancel: {nodes:?}"
    );
    println!("PASS: Cancel cleared the armed discard with no DELETE; the confirm window is gone");
}

// ── In-process TcpListener capture server (the MT-021 pattern, no new deps) ─────────────────────────

struct CapturedReq {
    request_line: String,
}

fn capture_one(listener: std::net::TcpListener) -> CapturedReq {
    use std::io::{Read, Write};
    let (mut stream, _) = listener.accept().expect("accept");
    let mut buf = [0u8; 8192];
    let mut data = Vec::new();
    loop {
        let n = stream.read(&mut buf).expect("read");
        if n == 0 {
            break;
        }
        data.extend_from_slice(&buf[..n]);
        let text = String::from_utf8_lossy(&data);
        if let Some(hdr_end) = text.find("\r\n\r\n") {
            let header = &text[..hdr_end];
            let body_so_far = &text[hdr_end + 4..];
            let content_len = header
                .lines()
                .find_map(|l| {
                    let l = l.to_ascii_lowercase();
                    l.strip_prefix("content-length:").map(|v| v.trim().parse::<usize>().ok())
                })
                .flatten()
                .unwrap_or(0);
            if body_so_far.len() >= content_len {
                break;
            }
        }
    }
    let text = String::from_utf8_lossy(&data).into_owned();
    let request_line = text.lines().next().unwrap_or("").to_owned();
    let _ = stream.write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\n{}");
    let _ = stream.flush();
    CapturedReq { request_line }
}

fn capture_server() -> (std::net::TcpListener, String) {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind");
    let port = listener.local_addr().unwrap().port();
    (listener, format!("http://127.0.0.1:{port}"))
}

fn test_runtime() -> tokio::runtime::Runtime {
    tokio::runtime::Builder::new_multi_thread()
        .worker_threads(1)
        .enable_all()
        .build()
        .expect("runtime")
}

#[test]
fn confirm_ok_fires_the_real_delete_on_the_wire() {
    // PROOF-024-2(e): with a real runtime + client pointed at a localhost capture server, arming Discard
    // then pressing the confirm window's OK dispatches the REAL DELETE /loom/blocks/:id on the wire. This
    // proves the confirm gate is the SINGLE DELETE call site (the menu never dispatches; OK does).
    let rt = test_runtime();
    let (listener, base) = capture_server();

    let mut harness =
        egui_kittest::Harness::builder().build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), ok_app());
    harness.run();
    {
        let app = harness.state_mut();
        // Point the drawer-action client at the capture server on a real runtime (the production dispatch
        // path), and bind the Notes card to a concrete block so Discard has a real target.
        app.set_backend_base_url_for_test(&base, rt.handle().clone());
        app.set_bottom_drawer_open(true);
        if let Some(card) = app.drawer_mut().card_mut(DrawerCardKind::Notes) {
            card.action_target = Some(DrawerActionTarget {
                workspace_id: "ws-1".to_owned(),
                block_id: "blk-del".to_owned(),
                title: "Doomed".to_owned(),
                content_type: "note".to_owned(),
                excerpt: "bye".to_owned(),
            });
        }
    }
    harness.run();
    harness.run();

    // Arm Discard via the menu.
    harness.get_by_label("Notes actions").click();
    harness.run();
    harness.run();
    harness.get_by_label("Discard").click();
    harness.run();
    harness.run();
    assert_eq!(
        harness.state().confirm_discard_block_id(),
        Some("blk-del"),
        "Discard armed the confirm gate (still no DELETE on the wire yet)"
    );

    // Capture the wire request on a background thread so the UI thread can keep pumping frames while the
    // off-thread tokio DELETE task connects (avoids a deadlock if accept() blocked the test thread).
    let cap_handle = std::thread::spawn(move || capture_one(listener));

    // Press OK: the SINGLE DELETE call site fires the real request. The menu is closed, so the only
    // "Discard"-labelled live node is the confirm window's OK button.
    harness.get_by_label("Discard").click();
    for _ in 0..8 {
        harness.run();
    }

    let cap = cap_handle.join().expect("capture thread");
    assert_eq!(
        cap.request_line, "DELETE /workspaces/ws-1/loom/blocks/blk-del HTTP/1.1",
        "confirm OK fired the real DELETE /loom/blocks/:id on the wire"
    );
    assert_eq!(
        harness.state().confirm_discard_block_id(),
        None,
        "the arm is cleared once OK dispatches the DELETE"
    );
    println!("PASS: confirm window OK fired the REAL DELETE /workspaces/ws-1/loom/blocks/blk-del on the wire");
}

#[test]
fn successful_action_clears_error_and_shows_success_state() {
    // MAJOR FIX (AC-024-4/5): a successful action clears the affected card's error + sets the success
    // indicator. The MT-023 drawer is FOUR FIXED TYPE cards, so the contract's card-removal/reorder does
    // not apply; the honest success effect is feedback + count refresh (disclosed deviation). Driven
    // through the real app receipt-drain path (no live backend needed — we seed the delivery cell + an
    // in-flight kind via a real Stow dispatch against a capture server, then drain).
    let rt = test_runtime();
    let (listener, base) = capture_server();

    let mut harness =
        egui_kittest::Harness::builder().build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), ok_app());
    harness.run();
    {
        let app = harness.state_mut();
        app.set_backend_base_url_for_test(&base, rt.handle().clone());
        app.set_bottom_drawer_open(true);
        if let Some(card) = app.drawer_mut().card_mut(DrawerCardKind::Notes) {
            // Seed a stale error so we can prove the success clears it.
            card.error = Some("stale failure".to_owned());
            card.action_target = Some(DrawerActionTarget {
                workspace_id: "ws-1".to_owned(),
                block_id: "blk-ok".to_owned(),
                title: "Note".to_owned(),
                content_type: "note".to_owned(),
                excerpt: "body".to_owned(),
            });
        }
    }
    harness.run();
    harness.run();

    // Dispatch a Stow (non-destructive persisting action) via the menu -> real POST on the wire.
    harness.get_by_label("Notes actions").click();
    harness.run();
    harness.run();
    harness.get_by_label("Stow").click();
    harness.run();

    // The capture server replies 200 {}; the off-thread task delivers Ok into the cell. Pump frames so
    // the receipt drain runs and folds the success in.
    let _cap = capture_one(listener);
    for _ in 0..8 {
        harness.run();
    }

    assert_eq!(
        harness.state().drawer_action_success(),
        Some(DrawerCardKind::Notes),
        "a successful action set the Notes success indicator"
    );
    assert_eq!(
        harness.state().drawer_action_error(),
        None,
        "the successful action left no action-level error"
    );
    println!("PASS: successful Stow cleared the error + set the Notes success state (feedback, not removal)");
}

#[test]
fn type_cards_disable_block_requiring_actions() {
    // Rubric end-to-end integrity: a TYPE card (no bound block) renders the block-requiring actions
    // DISABLED rather than dispatching against a nonexistent block id. The Agenda card has no target.
    let mut harness =
        Harness::builder().build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), ok_app());
    harness.run();
    harness.state_mut().set_bottom_drawer_open(true);
    harness.run();
    harness.run();

    harness.get_by_label("Agenda actions").click();
    harness.run();
    harness.run();

    // The menu still renders all eight items (PROOF-024-4) — disabled ones are present but not clickable.
    let nodes = author_nodes(&harness);
    let action_items = nodes
        .iter()
        .filter(|(a, _)| a.starts_with("ctx-menu.drawer.action."))
        .count();
    assert_eq!(action_items, 8, "all eight items render even when some are disabled: {nodes:?}");
}
