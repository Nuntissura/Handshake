//! MT-007 breakpoint-set unit proofs (WP-KERNEL-012 — E1 code editor gutter).
//!
//! AC-002 / PT-002 (`cargo test -p handshake-native breakpoint_set`): toggle adds a breakpoint, a
//! second toggle of the same line removes it, and `contains()` reports the correct per-line state.
//! Also proves the panel-level breakpoint toggle publishes a typed `BreakpointEvent` onto the
//! debug-adapter channel (RISK-003 non-blocking publish), and the MC-004-style clear path.

use std::sync::Arc;

use handshake_native::code_editor::breakpoints::{BreakpointAction, BreakpointSet};
use handshake_native::code_editor::CodeEditorPanel;

#[test]
fn breakpoint_set_toggle_adds_then_removes() {
    let mut set = BreakpointSet::new();
    assert!(!set.contains(3), "line 3 has no breakpoint initially");
    // Toggle ON.
    assert_eq!(set.toggle(3), BreakpointAction::Set);
    assert!(
        set.contains(3),
        "line 3 has a breakpoint after the first toggle"
    );
    assert_eq!(set.len(), 1);
    // Toggle OFF (idempotent in pairs — AC-002).
    assert_eq!(set.toggle(3), BreakpointAction::Clear);
    assert!(
        !set.contains(3),
        "line 3 breakpoint removed after the second toggle"
    );
    assert!(set.is_empty());
}

#[test]
fn breakpoint_set_contains_is_independent_per_line() {
    let mut set = BreakpointSet::new();
    set.toggle(2);
    set.toggle(10);
    assert!(set.contains(2));
    assert!(set.contains(10));
    assert!(!set.contains(5), "an untoggled line has no breakpoint");
    let lines: Vec<usize> = set.iter().collect();
    assert_eq!(
        lines,
        vec![2, 10],
        "iter yields the live breakpoints ascending"
    );
}

#[test]
fn panel_toggle_breakpoint_publishes_event_to_dap_channel() {
    // The panel toggle is the surface the gutter click + a future keymap call. Each toggle publishes a
    // typed BreakpointEvent that a future debug-adapter (DAP) client consumes via subscribe_breakpoints.
    let panel = CodeEditorPanel::new("fn main() {\n    let x = 1;\n}\n", "rs");
    panel.set_file_path("src/main.rs");
    let rx = panel
        .subscribe_breakpoints()
        .expect("the breakpoint receiver is available before any subscriber takes it");

    // Toggle ON line 1 -> a Set event.
    assert_eq!(panel.toggle_breakpoint(1), BreakpointAction::Set);
    assert!(panel.is_breakpoint_set(1));
    // Toggle OFF line 1 -> a Clear event.
    assert_eq!(panel.toggle_breakpoint(1), BreakpointAction::Clear);
    assert!(!panel.is_breakpoint_set(1));

    // Both events are queued on the channel in order, each carrying the file path + line + action.
    let e1 = rx.recv().expect("first breakpoint event published");
    let e2 = rx.recv().expect("second breakpoint event published");
    assert_eq!(e1.file_path, "src/main.rs");
    assert_eq!(e1.line, 1);
    assert_eq!(e1.action, BreakpointAction::Set);
    assert_eq!(e2.line, 1);
    assert_eq!(e2.action, BreakpointAction::Clear);
}

#[test]
fn panel_breakpoint_publish_is_benign_after_receiver_dropped() {
    // RISK-003 / MC-003: with the DAP client absent (receiver dropped), a toggle must NOT block or
    // panic — the publish is discarded. (Until a subscriber takes the receiver it is parked inside the
    // panel; here we take it and drop it to simulate a disconnected DAP client.)
    let panel = CodeEditorPanel::new("a\nb\nc", "txt");
    let rx = panel.subscribe_breakpoints().expect("receiver available");
    drop(rx); // simulate the DAP client disconnecting
              // These must complete without blocking/panicking (the unbounded send().ok() discards the Err).
    assert_eq!(panel.toggle_breakpoint(0), BreakpointAction::Set);
    assert_eq!(panel.toggle_breakpoint(0), BreakpointAction::Clear);
    // A second subscribe returns None (the channel has a single consumer).
    assert!(panel.subscribe_breakpoints().is_none());
}

#[test]
fn panel_clear_breakpoints_resets_all() {
    let panel = Arc::new(CodeEditorPanel::new("x\ny\nz", "txt"));
    panel.toggle_breakpoint(0);
    panel.toggle_breakpoint(2);
    assert_eq!(panel.breakpoint_set().len(), 2);
    panel.clear_breakpoints();
    assert!(
        panel.breakpoint_set().is_empty(),
        "clear_breakpoints removes every breakpoint"
    );
}
