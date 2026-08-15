//! WP-1 live orchestration debug console pane — headless egui_kittest proof.
//!
//! Opens the pane through the real app menu (MODELS -> Open Console), asserts the
//! streamed console entries render with STABLE `console_row_{index}` AccessKit
//! author_ids (the reused DebugConsole widget contract), and that the display
//! filter narrows the visible rows. Headless: egui_kittest embeds viewports, so
//! no live GUI host is needed.

use egui_kittest::kittest::{NodeT, Queryable};
use egui_kittest::Harness;
use handshake_native::app::{HandshakeApp, HealthDisplayState};
use handshake_native::backend_client::HealthInfo;
use handshake_native::console_stream_pane::{
    ConsoleStreamEntry, FILTER_ALL_AUTHOR_ID, FILTER_ERRORS_AUTHOR_ID,
};
use handshake_native::debug_console::console_row_author_id;
use handshake_native::pane_registry::{PaneId, PaneType};
use handshake_native::popout_window::{popout_title_for, popout_window_author_id};

fn entry(
    seq: u64,
    severity: &str,
    category: &str,
    subject: &str,
    detail: &str,
) -> ConsoleStreamEntry {
    ConsoleStreamEntry {
        seq,
        ts_unix_ms: 0,
        severity: severity.to_string(),
        category: category.to_string(),
        subject: subject.to_string(),
        detail: detail.to_string(),
        trace_id: None,
    }
}

fn app_with_entries(entries: Vec<ConsoleStreamEntry>) -> HandshakeApp {
    let mut app = HandshakeApp::with_health(HealthDisplayState::Ok(HealthInfo {
        status: "ok".to_string(),
        db_status: "ok".to_string(),
        migration_version: Some(1),
    }));
    app.set_wp1_console_entries_for_test(entries);
    app
}

fn live_author_ids(harness: &Harness<'_, HandshakeApp>) -> Vec<String> {
    harness
        .root()
        .children_recursive()
        .filter_map(|n| n.accesskit_node().author_id().map(str::to_owned))
        .collect()
}

fn node_by_author<'a>(
    harness: &'a Harness<'_, HandshakeApp>,
    author_id: &str,
) -> egui_kittest::Node<'a> {
    harness
        .root()
        .children_recursive()
        .find(|n| n.accesskit_node().author_id() == Some(author_id))
        .unwrap_or_else(|| {
            panic!(
                "{author_id} missing from live tree: {:?}",
                live_author_ids(harness)
            )
        })
}

fn open_console(harness: &mut Harness<'_, HandshakeApp>) {
    harness.run();
    harness.get_by_label("MODELS").click();
    harness.run();
    harness.get_by_label("Open Console").click();
    harness.run();
    // One extra frame so the pane drains its delivery buffer into the console.
    harness.run();
}

#[test]
fn wp1_console_pane_renders_streamed_entries_with_stable_author_ids_and_filters() {
    let entries = vec![
        entry(0, "info", "model_lane_launch", "lane-1", "lane spawned"),
        entry(1, "info", "model_lane_status", "lane-1", "lane ready"),
        entry(
            2,
            "error",
            "model_lane_status",
            "lane-1",
            "lane failed: boom",
        ),
    ];
    let mut harness = Harness::builder().build_state(
        |ctx, a: &mut HandshakeApp| a.ui(ctx),
        app_with_entries(entries),
    );
    open_console(&mut harness);

    // The MODELS -> Open Console menu opened a native WP-1 console tab.
    assert!(
        harness.state().tab_bar_states().values().any(|bar| bar
            .tabs
            .iter()
            .any(|tab| tab.pane_type == PaneType::Wp1OrchestrationConsole)),
        "Open Console opened a native Wp1OrchestrationConsole tab"
    );

    // All three streamed entries rendered with stable console_row_{index} author_ids.
    let authors = live_author_ids(&harness);
    for index in 0..3 {
        assert!(
            authors.iter().any(|id| id == &console_row_author_id(index)),
            "streamed entry {index} rendered with stable author_id {}: {authors:?}",
            console_row_author_id(index)
        );
    }

    // Filter to Errors: only the error row (original index 2) stays; the two info
    // rows (0, 1) are filtered out. The author_id stays the ORIGINAL index, so the
    // row identity is stable across filter changes.
    node_by_author(&harness, FILTER_ERRORS_AUTHOR_ID).click_accesskit();
    harness.run();
    let filtered = live_author_ids(&harness);
    assert!(
        filtered.iter().any(|id| id == &console_row_author_id(2)),
        "error row remains after filtering to Errors: {filtered:?}"
    );
    assert!(
        !filtered.iter().any(|id| id == &console_row_author_id(0)),
        "info row 0 is filtered out"
    );
    assert!(
        !filtered.iter().any(|id| id == &console_row_author_id(1)),
        "info row 1 is filtered out"
    );

    // Show All restores every row.
    node_by_author(&harness, FILTER_ALL_AUTHOR_ID).click_accesskit();
    harness.run();
    let restored = live_author_ids(&harness);
    for index in 0..3 {
        assert!(
            restored
                .iter()
                .any(|id| id == &console_row_author_id(index)),
            "Show All restores row {index}: {restored:?}"
        );
    }
}

#[test]
fn wp1_console_active_tab_keeps_its_factory_and_title_when_host_pane_is_popped_out() {
    let mut harness = Harness::builder().build_state(
        |ctx, app: &mut HandshakeApp| app.ui(ctx),
        app_with_entries(Vec::new()),
    );
    open_console(&mut harness);

    let pane_id: PaneId = harness
        .state()
        .tab_bar_states()
        .iter()
        .find_map(|(pane_id, bar)| {
            bar.active()
                .is_some_and(|tab| tab.pane_type == PaneType::Wp1OrchestrationConsole)
                .then(|| pane_id.clone())
        })
        .expect("Open Console leaves the console tab active in one pane");

    harness.state_mut().request_pop_out(pane_id.clone());
    harness.run();
    harness.run();

    let authors = live_author_ids(&harness);
    assert!(
        authors.iter().any(|id| id == FILTER_ALL_AUTHOR_ID),
        "detached active console tab must render the console factory, not the pane record's original factory: {authors:?}"
    );
    let window_author_id = popout_window_author_id(pane_id.as_ref());
    let window = node_by_author(&harness, &window_author_id);
    let expected_title = popout_title_for(&PaneType::Wp1OrchestrationConsole.label());
    assert_eq!(
        window.accesskit_node().label(),
        Some(expected_title),
        "detached window title must describe the active console surface"
    );
}
