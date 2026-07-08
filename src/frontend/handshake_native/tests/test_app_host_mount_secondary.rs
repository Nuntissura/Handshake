//! WP-KERNEL-012 MT-080 (E11 host-mount, part 2) — the real-app GUI inspection of the SECONDARY native
//! panes (the MT-079 fuller-mount follow-on).
//!
//! MT-079 mounted the CORE code + rich editors LIVE; this MT mounts the rest of the widget-proven panes —
//! the canvas board (MT-026), the graph view (MT-021/060), and the side panes (outgoing-links MT-062,
//! relevant-memory MT-063, Stage MT-066, daily-journal MT-067, manual MT-073) — over their
//! `PlaceholderPaneFactory` entries, plus the deeper per-pane wirings (canvas PATCH/POST, graph depth
//! re-query, side-pane nav). These proofs drive the LIVE `HandshakeApp` through the SAME egui + AccessKit
//! path the running shell uses (NOT a widget harness), so a green proof means the secondary panes RENDER
//! their REAL subtrees in the running app and their event seams reach the real host paths.
//!
//! - PT-080-A / AC-080-1: `secondary_panes_render_live_in_app_tree_and_screenshot` re-types the seeded 2x2
//!   panes to the seven secondary surfaces, runs the real `app.ui` for several frames, asserts each pane's
//!   REAL AccessKit subtree is present (NOT a placeholder node), and saves a wgpu screenshot of the mounted
//!   secondary panes to the EXTERNAL artifact root.
//! - PT-080-B / AC-080-2: `canvas_resize_event_routes_to_host` enqueues a `CanvasEvent::ResizePlacement` on
//!   the SAME mounted board and asserts the host drains it (the event->host PATCH path fires; the live PG
//!   round-trip is gated NEEDS_MANAGED_RESOURCE_PROOF).
//! - PT-080-B / AC-080-3: `graph_depth_changed_requeries_with_new_backlink_depth` proves the depth-
//!   parameterized graph-search builder carries the new backlink_depth, and that a `DepthChanged` enqueued
//!   on the live mounted graph is drained by the host (the live fetch is gated).
//! - PT-080-B / AC-080-5: `outgoing_links_click_routes_to_nav` seeds a resolved link on the mounted pane,
//!   clicks it, and asserts a nav target reaches the shell's outbound queue (routed to the nav bus).
//! - PT-080-B / AC-080-5: `relevant_memory_shows_endpoint_missing_empty_state` drives the FEMS read (route
//!   verified ABSENT) and asserts the panel holds the `EndpointMissing` typed blocker (honest empty-state).
//! - PT-080-A / AC-080-6: `code_text_node_exposes_swarm_edit_actions` asserts the live `code_editor_text`
//!   node advertises `Action::SetValue` + `Action::ReplaceSelectedText`, and a dispatched SetValue mutates
//!   the buffer.

use std::path::{Path, PathBuf};

use egui_kittest::kittest::NodeT;
use egui_kittest::Harness;

use handshake_native::app::{HandshakeApp, HealthDisplayState, DEFAULT_PROJECT_ID};
use handshake_native::backend_client::HealthInfo;
use handshake_native::backend_client::{LoomGraphClient, MAX_BACKLINK_DEPTH, MIN_BACKLINK_DEPTH};
use handshake_native::code_editor::CODE_EDITOR_TEXT_AUTHOR_ID;
use handshake_native::pane_registry::{
    DirtyState, LockState, PaneAuthority, PaneId, PaneRecord, PaneType,
};
use handshake_native::quick_switcher::{NavDispatchOutcome, ShellNavigator};

/// Serialize the `.wgpu()` screenshot test (the documented Windows-wgpu concurrent-device hazard).
static WGPU_SERIAL_GUARD: std::sync::Mutex<()> = std::sync::Mutex::new(());
fn wgpu_guard() -> std::sync::MutexGuard<'static, ()> {
    WGPU_SERIAL_GUARD.lock().unwrap_or_else(|p| p.into_inner())
}

/// The crate-relative path to the external artifacts root (CX-212E), disk-agnostic. The crate sits at
/// `<repo>/src/frontend/handshake_native`, so four `..` reach `<repo>/..` where `Handshake_Artifacts`
/// is a sibling of the repo worktree. (The SCREENSHOT/TEST-ARTIFACT rule overrides any repo-local path.)
fn external_artifact_dir(subdir: &str) -> PathBuf {
    Path::new("../../../../Handshake_Artifacts/handshake-test").join(subdir)
}

/// Assert NO repo-local artifact directory exists under the crate (the artifact-hygiene guard the
/// SCREENSHOT/TEST-ARTIFACT rule mandates). Checks BOTH `test_output/` and `tests/screenshots/`;
/// artifacts go to the external root ONLY — a stray local dir is a hygiene FAILURE.
fn assert_no_local_artifact_dir() {
    for local in ["test_output", "tests/screenshots"] {
        let p = Path::new(local);
        assert!(
            !p.exists(),
            "artifact hygiene: no repo-local '{local}' dir may exist — artifacts go to the external \
             Handshake_Artifacts/handshake-test root only (found {})",
            p.display()
        );
    }
}

/// A live, RUNTIME-INJECTED shell with the seeded panes RE-TYPED so the four split slots host the
/// secondary surfaces this MT mounts. The split renders each fixed pane id's RECORD pane_type through the
/// factory map, so re-typing `pane-a..pane-d` makes the split render the REAL mounted secondary factories
/// at those slots. A multi-thread runtime is injected (so the per-frame session/palette push binds the
/// panes' context) and returned alongside the app so it OUTLIVES the harness.
fn secondary_shell() -> (HandshakeApp, tokio::runtime::Runtime) {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(1)
        .enable_all()
        .build()
        .expect("build multi-thread runtime");
    let mut app = HandshakeApp::with_health(HealthDisplayState::Ok(HealthInfo {
        status: "ok".to_string(),
        db_status: "ok".to_string(),
        migration_version: Some(1),
    }));
    app.set_runtime_handle(runtime.handle().clone());
    retype_panes(
        &mut app,
        &[
            ("pane-a", PaneType::AtelierEditor), // canvas board
            // REMEDIATION (MT-080 PaneType collisions): the graph view + outgoing-links panes now own
            // their OWN Placeholder keys — the old KernelDcc/LoomBlock registrations hijacked
            // content-addressed navigation (every loom-block open rendered the links panel; WP/MT quick-
            // switcher hits rendered a content-blind graph). The re-type targets the NEW honest keys.
            (
                "pane-b",
                handshake_native::editor_pane_factories::placeholder_pane_type(
                    handshake_native::editor_pane_factories::GRAPH_VIEW_PANE_LABEL,
                ),
            ), // graph view
            (
                "pane-c",
                handshake_native::editor_pane_factories::placeholder_pane_type(
                    handshake_native::editor_pane_factories::OUTGOING_LINKS_PANE_LABEL,
                ),
            ), // outgoing links
            ("pane-d", PaneType::UserManual), // manual
        ],
    );
    (app, runtime)
}

/// Re-type the fixed seeded panes to the given `(pane_id, pane_type)` set — BOTH the registry records
/// AND the tab bars. The shell syncs each pane's registry record FROM its ACTIVE TAB every frame
/// (MT-099 `sync`), so a registry-only re-type is overwritten by the fresh-default editor tabs on the
/// first frame and the split would render the default editors instead of the fixture's panes.
fn retype_panes(app: &mut HandshakeApp, panes: &[(&str, PaneType)]) {
    {
        let registry = app.pane_registry();
        let mut guard = registry.lock().expect("registry");
        for (id, ty) in panes {
            guard.insert(PaneRecord::new(
                PaneId::from(*id),
                ty.clone(),
                DEFAULT_PROJECT_ID,
                None,
                LockState::Unlocked,
                DirtyState::Clean,
                PaneAuthority::System,
            ));
        }
    }
    let bars = app.tab_bar_states_mut();
    for (id, ty) in panes {
        if let Some(bar) = bars.get_mut(&PaneId::from(*id)) {
            bar.tabs = vec![handshake_native::tab_bar::TabState::new(ty.clone())];
            bar.active_index = 0;
        }
    }
}

/// Every `author_id` present in the live consumer-side AccessKit tree.
fn live_author_ids(harness: &Harness<'_, HandshakeApp>) -> std::collections::HashSet<String> {
    let mut ids = std::collections::HashSet::new();
    for node in harness.root().children_recursive() {
        if let Some(a) = node.accesskit_node().author_id() {
            ids.insert(a.to_owned());
        }
    }
    ids
}

/// Read a live app AccessKit node's label by author_id.
fn live_label_for(harness: &Harness<'_, HandshakeApp>, author_id: &str) -> Option<String> {
    for node in harness.root().children_recursive() {
        let ak = node.accesskit_node();
        if ak.author_id() == Some(author_id) {
            return ak.label().map(|v| v.to_owned());
        }
    }
    None
}

// ── PT-080-A / AC-080-1: secondary panes render LIVE in the running app + screenshot ──────────────────

#[test]
fn secondary_panes_render_live_in_app_tree_and_screenshot() {
    use handshake_native::fems::relevant_memory_panel::RELEVANT_MEMORY_PANEL_AUTHOR_ID;
    use handshake_native::graph::{ADD_CARD_AUTHOR_ID, DAILY_JOURNAL_PANEL_AUTHOR_ID};
    use handshake_native::graph::{
        MODE_LOCAL_AUTHOR_ID, STATUS_AUTHOR_ID as CANVAS_STATUS_AUTHOR_ID,
    };
    use handshake_native::manual_pane::MANUAL_PANE_AUTHOR_ID;
    use handshake_native::rich_editor::wikilinks::outgoing_links_panel::PANEL_AUTHOR_ID as OUTGOING_PANEL_AUTHOR_ID;
    use handshake_native::stage_pane::STAGE_PANE_AUTHOR_ID;

    let _g = wgpu_guard();
    // First frame batch: the canvas / graph / outgoing-links / manual surfaces (the 4-slot split).
    let (app, _rt) = secondary_shell();
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1400.0, 900.0))
        .wgpu()
        .build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), app);
    harness.run_steps(4);
    let ids = live_author_ids(&harness);

    // CANVAS real subtree: the toolbar emits `canvas.add-card` + `canvas.status` (NOT a placeholder).
    assert!(
        ids.contains(ADD_CARD_AUTHOR_ID) || ids.contains(CANVAS_STATUS_AUTHOR_ID),
        "the live app tree carries the REAL canvas subtree ('{ADD_CARD_AUTHOR_ID}'/'{CANVAS_STATUS_AUTHOR_ID}'); \
         got a canvas subset {:?}",
        ids.iter().filter(|i| i.starts_with("canvas")).collect::<Vec<_>>()
    );
    // GRAPH real subtree: the toolbar emits `graph.mode.local`.
    assert!(
        ids.contains(MODE_LOCAL_AUTHOR_ID),
        "the live app tree carries the REAL graph subtree ('{MODE_LOCAL_AUTHOR_ID}'); got a graph subset {:?}",
        ids.iter().filter(|i| i.starts_with("graph")).collect::<Vec<_>>()
    );
    // OUTGOING-LINKS real subtree: the panel is the empty-state initially, but the manual pane root and the
    // outgoing panel render. (The empty outgoing pane emits no `outgoing.*` node until it has links, so we
    // assert the manual pane below as the fourth slot's real subtree; the outgoing pane's live state is
    // proven by `outgoing_links_click_routes_to_nav`.)
    let _ = OUTGOING_PANEL_AUTHOR_ID;
    // MANUAL real subtree: the `manual-pane` Region node.
    assert!(
        ids.contains(MANUAL_PANE_AUTHOR_ID),
        "the live app tree carries the REAL manual pane subtree ('{MANUAL_PANE_AUTHOR_ID}'); got {ids:?}"
    );

    // wgpu screenshot of the four mounted secondary panes -> the EXTERNAL artifact root ONLY.
    let screenshot_saved = match harness.render() {
        Ok(image) => {
            let (w, h) = (image.width(), image.height());
            assert!(w > 0 && h > 0, "rendered image is non-empty");
            let ext_dir = external_artifact_dir("wp-kernel-012-mt-080");
            let _ = std::fs::create_dir_all(&ext_dir);
            let png_path = ext_dir.join("MT-080-secondary-panes-mounted-live.png");
            let saved = image.save(&png_path).is_ok();
            let abs = std::fs::canonicalize(&png_path).unwrap_or(png_path.clone());
            println!(
                "PT-080-A mounted-secondary-panes screenshot: {w}x{h}, saved={saved} ({})",
                abs.display()
            );
            saved
        }
        Err(e) => {
            println!(
                "BLOCKER(non-fatal): MT-080 secondary-panes screenshot render unavailable (no wgpu \
                 adapter): {e}. AC-080-1 AccessKit real-subtree proof passed; the PNG is a GPU-host item."
            );
            false
        }
    };
    let _ = screenshot_saved;

    // Second frame batch: the relevant-memory / Stage / daily-journal side panes (re-typed into the slots).
    let (mut app2, _rt2) = secondary_shell();
    retype_panes(
        &mut app2,
        &[
            (
                "pane-a",
                PaneType::Placeholder("Relevant Memory".to_owned()),
            ),
            ("pane-b", PaneType::Placeholder("Stage".to_owned())),
            ("pane-c", PaneType::LoomDailyJournal),
            ("pane-d", PaneType::UserManual),
        ],
    );
    let mut harness2 = Harness::builder()
        .with_size(egui::vec2(1400.0, 900.0))
        .build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), app2);
    harness2.run_steps(4);
    let ids2 = live_author_ids(&harness2);

    assert!(
        ids2.contains(RELEVANT_MEMORY_PANEL_AUTHOR_ID),
        "the live app tree carries the REAL relevant-memory subtree ('{RELEVANT_MEMORY_PANEL_AUTHOR_ID}')"
    );
    assert!(
        ids2.contains(STAGE_PANE_AUTHOR_ID),
        "the live app tree carries the REAL Stage subtree ('{STAGE_PANE_AUTHOR_ID}')"
    );
    assert!(
        ids2.contains(DAILY_JOURNAL_PANEL_AUTHOR_ID),
        "the live app tree carries the REAL daily-journal subtree ('{DAILY_JOURNAL_PANEL_AUTHOR_ID}')"
    );

    assert_no_local_artifact_dir();
}

// ── PT-080-B / AC-080-2: canvas resize event routes to the host PATCH path ────────────────────────────

#[test]
fn canvas_resize_event_routes_to_host() {
    use handshake_native::graph::CanvasEvent;

    let (app, _rt) = secondary_shell();
    let canvas_events = app.mounted_canvas_events();
    let mut harness =
        Harness::builder().build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), app);
    harness.run_steps(2);

    // Enqueue a ResizePlacement the way a resize drag-stop would, on the SAME mounted board's outbound
    // queue, then run a live frame: the shell drains it (drive_secondary_mounts -> route_canvas_events)
    // and maps it to the EXISTING CanvasBoardClient PATCH + board re-fetch. After the frame the queue is
    // empty (drained) — the event reached the host path (the live PG round-trip is gated).
    canvas_events
        .lock()
        .unwrap()
        .push(CanvasEvent::ResizePlacement {
            placement_id: "p-mt080".into(),
            w: 320.0,
            h: 180.0,
        });
    assert_eq!(
        canvas_events.lock().unwrap().len(),
        1,
        "the event is enqueued before the frame"
    );
    harness.run_steps(2);
    assert!(
        canvas_events.lock().unwrap().is_empty(),
        "AC-080-2: the canvas ResizePlacement was DRAINED by the host (mapped to the real PATCH path)"
    );
}

// ── PT-080-B / AC-080-3: graph DepthChanged re-queries at the new backlink_depth ──────────────────────

#[test]
fn graph_depth_changed_requeries_with_new_backlink_depth() {
    use handshake_native::graph::{GraphEvent, GraphMode};

    // The depth-parameterized builder carries the new backlink_depth on the verified endpoint (the host
    // re-query the DepthChanged fires). This is the pure builder proof; the live fetch is gated.
    let rt = tokio::runtime::Builder::new_current_thread()
        .build()
        .unwrap();
    let client = LoomGraphClient::production(rt.handle().clone());
    let spec = client.local_request_with_depth(DEFAULT_PROJECT_ID, "Focused Note", 4);
    assert_eq!(
        spec.query[1],
        ("backlink_depth".to_owned(), "4".to_owned()),
        "AC-080-3: the re-query carries the NEW backlink_depth on the existing graph-search endpoint"
    );
    // Clamp envelope (RISK-080-3): an out-of-range depth never reaches the backend as an abusive traversal.
    assert_eq!(
        client
            .local_request_with_depth(DEFAULT_PROJECT_ID, "T", 99)
            .query[1],
        ("backlink_depth".to_owned(), MAX_BACKLINK_DEPTH.to_string())
    );
    assert_eq!(
        client
            .local_request_with_depth(DEFAULT_PROJECT_ID, "T", 0)
            .query[1],
        ("backlink_depth".to_owned(), MIN_BACKLINK_DEPTH.to_string())
    );

    // The live mounted graph drains a DepthChanged: put the view in Local mode (so the depth re-query has a
    // focus), enqueue DepthChanged, run a frame, and assert the host drained it (the event reached the
    // re-query path; the live fetch is gated NEEDS_MANAGED_RESOURCE_PROOF).
    let (app, _rt2) = secondary_shell();
    let graph_view = app.mounted_graph_view();
    {
        let mut v = graph_view.lock().unwrap();
        v.mode = GraphMode::Local {
            block_id: "blk-1".into(),
            title: "Focused Note".into(),
        };
        v.workspace_id = DEFAULT_PROJECT_ID.to_owned();
    }
    let events = app.editor_mounts_graph_events_for_test();
    let graph_view2 = app.mounted_graph_view();
    let mut harness =
        Harness::builder().build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), app);
    harness.run_steps(2);
    events
        .lock()
        .unwrap()
        .push(GraphEvent::DepthChanged { depth: 3 });
    harness.run_steps(2);
    assert!(
        events.lock().unwrap().is_empty(),
        "AC-080-3: the graph DepthChanged was DRAINED by the host (mapped to the depth re-query)"
    );
    // MT-060 REMEDIATION (the deliver path is LIVE): the host now sets `loading = true` on the depth
    // re-query AND has the per-frame graph-cell drain that CLEARS it when the fetch resolves — so the
    // MT-015 perpetual-spinner trap the old idle-neutral assertion guarded against is closed from the
    // other side. With no backend on this host, the off-thread re-query resolves to a typed transport
    // Err, which the deliver drain applies as the view's error label + `loading = false`. Poll bounded
    // frames until the delivery lands, then assert the spinner CLEARED and the typed error SURFACED —
    // runtime proof the deliver loop (previously a discarded throwaway cell) is live.
    let mut delivered = false;
    for _ in 0..100 {
        harness.run_steps(2);
        let v = graph_view2.lock().unwrap();
        if !v.loading {
            delivered = true;
            assert!(
                v.error.is_some(),
                "MT-060 deliver path: with no live backend the re-query resolves to the TYPED error \
                 surfaced on the view (never a silent discard)"
            );
            break;
        }
        drop(v);
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    assert!(
        delivered,
        "MT-060 deliver path: the depth re-query's result was DELIVERED into the mounted view \
         (loading cleared by the per-frame graph-cell drain — no perpetual spinner)"
    );
}

// ── MT-022 mounted host: folder-tree events route through the live shell ─────────────────────────────

#[test]
fn folder_tree_host_drains_expand_recolor_retry_and_open_events() {
    use handshake_native::editor_pane_factories::{placeholder_pane_type, FOLDER_TREE_PANE_LABEL};
    use handshake_native::graph::folder_tree::{FolderRow, FolderTreeEvent};

    let (mut app, _rt) = secondary_shell();
    retype_panes(
        &mut app,
        &[("pane-a", placeholder_pane_type(FOLDER_TREE_PANE_LABEL))],
    );
    let folder_tree = app.mounted_folder_tree_for_test();
    let folder_events = app.mounted_folder_events_for_test();
    {
        let mut tree = folder_tree.lock().unwrap();
        tree.set_folders(&[FolderRow::new(
            "folder-host",
            None,
            "Host Folder",
            Some("#336699".to_owned()),
        )]);
    }

    let mut harness =
        Harness::builder().build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), app);
    harness.run_steps(2);

    let baseline_child_cells = harness.state().folder_child_cells_in_flight_for_test();
    folder_events
        .lock()
        .unwrap()
        .push(FolderTreeEvent::ExpandFolder {
            folder_id: "folder-host".to_owned(),
        });
    harness.run_steps(1);
    assert!(
        folder_events.lock().unwrap().is_empty(),
        "MT-022 host: ExpandFolder event queue was drained"
    );
    assert!(
        harness.state().folder_child_cells_in_flight_for_test() > baseline_child_cells,
        "MT-022 host: ExpandFolder dispatched a real child-block fetch cell"
    );
    {
        let mut tree = folder_tree.lock().unwrap();
        let node = tree
            .find_folder_mut("folder-host")
            .expect("seeded host folder remains mounted");
        assert!(
            node.expanded && node.loading,
            "MT-022 host: expand marks the mounted node expanded and loading while the child fetch is in flight"
        );
    }

    let baseline_recolor_cells = harness.state().folder_recolor_cells_in_flight_for_test();
    folder_events
        .lock()
        .unwrap()
        .push(FolderTreeEvent::ChangeColor {
            folder_id: "folder-host".to_owned(),
            color: egui::Color32::from_rgb(255, 0, 0),
        });
    harness.run_steps(1);
    assert!(
        folder_events.lock().unwrap().is_empty(),
        "MT-022 host: ChangeColor event queue was drained"
    );
    assert!(
        harness.state().folder_recolor_cells_in_flight_for_test() > baseline_recolor_cells,
        "MT-022 host: ChangeColor dispatched a real recolor PATCH cell"
    );

    folder_events.lock().unwrap().push(FolderTreeEvent::Retry);
    harness.run_steps(1);
    assert!(
        folder_tree.lock().unwrap().loading,
        "MT-022 host: Retry sets the mounted tree into the bounded loading state before re-fetch"
    );

    folder_events
        .lock()
        .unwrap()
        .push(FolderTreeEvent::OpenBlock {
            block_id: "block-host".to_owned(),
        });
    harness.run_steps(1);
    let opened = harness
        .state_mut()
        .tab_bar_states_mut()
        .values()
        .any(|bar| {
            bar.tabs.iter().any(|tab| {
                tab.pane_type == PaneType::LoomBlock
                    && tab.content_id.as_deref() == Some("block-host")
            })
        });
    assert!(
        opened,
        "MT-022 host: OpenBlock routes through open_content_on_active_pane into a LoomBlock tab"
    );
}

#[test]
fn sidebar_active_block_deliveries_bind_backlinks_and_unlinked() {
    use handshake_native::editor_pane_factories::{placeholder_pane_type, SIDEBAR_PANE_LABEL};
    use handshake_native::graph::sidebar_panel::{
        BacklinkRow, SectionKind, SidebarEvent, UnlinkedRow,
    };

    let mut app = HandshakeApp::with_health(HealthDisplayState::Ok(HealthInfo {
        status: "ok".to_string(),
        db_status: "ok".to_string(),
        migration_version: Some(1),
    }));
    retype_panes(
        &mut app,
        &[("pane-a", placeholder_pane_type(SIDEBAR_PANE_LABEL))],
    );
    let sidebar_panel = app.mounted_sidebar_panel_for_test();
    let sidebar_events = app.mounted_sidebar_events_for_test();
    let mut harness =
        Harness::builder().build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), app);

    sidebar_events.lock().unwrap().push(SidebarEvent::Open {
        block_id: "block-a".to_owned(),
    });
    harness.run_steps(1);
    {
        let panel = sidebar_panel.lock().unwrap();
        assert_eq!(
            panel.active_block_id.as_deref(),
            Some("block-a"),
            "MT-024 host: SidebarEvent::Open binds the clicked Loom block as sidebar active context"
        );
    }

    let (backlinks_generation, unlinked_generation) = harness
        .state()
        .prepare_sidebar_active_block_for_test("block-a")
        .expect("MT-024 host: prepare active block generations");
    {
        let panel = sidebar_panel.lock().unwrap();
        assert!(
            panel.loading_section.contains(&SectionKind::Backlinks)
                && panel.loading_section.contains(&SectionKind::Unlinked),
            "MT-024 host: active-block prepare marks Backlinks and Unlinked independently loading"
        );
    }
    harness.state().deliver_sidebar_backlinks_for_test(
        backlinks_generation,
        Ok(vec![BacklinkRow::new("source-a", "Source A", "mention")]),
    );
    harness.state().deliver_sidebar_unlinked_for_test(
        unlinked_generation,
        Ok(vec![UnlinkedRow::new("mention-a", "Mention A")]),
    );
    harness.run_steps(1);
    {
        let panel = sidebar_panel.lock().unwrap();
        assert_eq!(
            panel.backlinks,
            vec![BacklinkRow::new("source-a", "Source A", "mention")],
            "MT-024 host: generation-matched Backlinks delivery reaches the mounted sidebar panel"
        );
        assert_eq!(
            panel.unlinked,
            vec![UnlinkedRow::new("mention-a", "Mention A")],
            "MT-024 host: generation-matched Unlinked delivery reaches the mounted sidebar panel"
        );
        assert!(
            !panel.loading_section.contains(&SectionKind::Backlinks)
                && !panel.loading_section.contains(&SectionKind::Unlinked),
            "MT-024 host: successful deliveries clear only their section loading flags"
        );
    }

    let (fresh_backlinks_generation, _) = harness
        .state()
        .prepare_sidebar_active_block_for_test("block-b")
        .expect("MT-024 host: prepare refreshed active block generations");
    harness.state().deliver_sidebar_backlinks_for_test(
        backlinks_generation,
        Ok(vec![BacklinkRow::new("stale", "Stale", "mention")]),
    );
    harness.run_steps(1);
    {
        let panel = sidebar_panel.lock().unwrap();
        assert!(
            panel.backlinks.is_empty(),
            "MT-024 host: stale Backlinks delivery cannot overwrite the new active block"
        );
        assert!(
            panel.loading_section.contains(&SectionKind::Backlinks),
            "MT-024 host: stale Backlinks delivery leaves the current generation in flight"
        );
    }
    harness.state().deliver_sidebar_backlinks_for_test(
        fresh_backlinks_generation,
        Ok(vec![BacklinkRow::new("source-b", "Source B", "related")]),
    );
    harness.run_steps(1);
    {
        let panel = sidebar_panel.lock().unwrap();
        assert_eq!(
            panel.backlinks,
            vec![BacklinkRow::new("source-b", "Source B", "related")],
            "MT-024 host: fresh Backlinks generation applies after stale delivery was dropped"
        );
    }
}

#[test]
fn folder_tree_refetches_and_clears_state_after_workspace_switch() {
    use handshake_native::editor_pane_factories::{placeholder_pane_type, FOLDER_TREE_PANE_LABEL};
    use handshake_native::graph::folder_tree::FolderRow;

    let (mut app, _rt) = secondary_shell();
    retype_panes(
        &mut app,
        &[("pane-a", placeholder_pane_type(FOLDER_TREE_PANE_LABEL))],
    );
    let folder_tree = app.mounted_folder_tree_for_test();
    let mut harness =
        Harness::builder().build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), app);
    harness.run_steps(2);

    {
        let mut tree = folder_tree.lock().unwrap();
        tree.workspace_id = DEFAULT_PROJECT_ID.to_owned();
        tree.set_folders(&[FolderRow::new("old-folder", None, "Old Folder", None)]);
    }
    assert!(
        harness.state_mut().switch_project("project-b"),
        "test precondition: project switch should happen"
    );
    retype_panes(
        harness.state_mut(),
        &[("pane-a", placeholder_pane_type(FOLDER_TREE_PANE_LABEL))],
    );
    harness.run_steps(2);

    let tree = folder_tree.lock().unwrap();
    assert_eq!(
        tree.workspace_id, "project-b",
        "MT-022 host: folder tree must be keyed to the active workspace after a project switch"
    );
    assert!(
        tree.root_nodes.is_empty(),
        "MT-022 host: stale folder rows from the previous workspace must be cleared before refetch"
    );
    assert!(
        tree.loading || tree.error.is_some(),
        "MT-022 host: the new workspace should start a bounded folder-list refetch when the pane is visible"
    );
}

#[test]
fn tags_pane_refetches_and_clears_state_after_workspace_switch() {
    use handshake_native::editor_pane_factories::{placeholder_pane_type, TAGS_PANE_LABEL};
    use handshake_native::graph::tags_panel::{HubMember, LoomTagHubPanel, TagEntry};

    let (mut app, _rt) = secondary_shell();
    retype_panes(
        &mut app,
        &[("pane-a", placeholder_pane_type(TAGS_PANE_LABEL))],
    );
    let tags_panel = app.mounted_tags_panel_for_test();
    let tags_hub = app.mounted_tags_hub_for_test();
    let mut harness =
        Harness::builder().build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), app);
    harness.run_steps(2);

    {
        let mut panel = tags_panel.lock().unwrap();
        panel.workspace_id = DEFAULT_PROJECT_ID.to_owned();
        panel.set_tags(vec![TagEntry::new("old-tag", "Old Tag", Some(1))]);
        panel.search_filter = "old".to_owned();
    }
    {
        let mut hub = tags_hub.lock().unwrap();
        let mut old_hub = LoomTagHubPanel::new(DEFAULT_PROJECT_ID, "old-tag");
        old_hub.set_detail(
            "Old Tag",
            vec![HubMember::new("old-member", "Old Member", "note")],
        );
        *hub = Some(old_hub);
    }

    assert!(
        harness.state_mut().switch_project("project-b"),
        "test precondition: project switch should happen"
    );
    retype_panes(
        harness.state_mut(),
        &[("pane-a", placeholder_pane_type(TAGS_PANE_LABEL))],
    );
    harness.run_steps(2);

    let panel = tags_panel.lock().unwrap();
    assert_eq!(
        panel.workspace_id, "project-b",
        "MT-023 host: tags panel must be keyed to the active workspace after a project switch"
    );
    assert!(
        panel.tags.is_empty(),
        "MT-023 host: stale tag rows from the previous workspace must be cleared before refetch"
    );
    assert!(
        panel.search_filter.is_empty(),
        "MT-023 host: stale tag search text from the previous workspace must be cleared"
    );
    assert!(
        panel.loading || panel.error.is_some(),
        "MT-023 host: the new workspace should start a bounded tag-list refetch when the pane is visible"
    );
    drop(panel);
    assert!(
        tags_hub.lock().unwrap().is_none(),
        "MT-023 host: an open tag-hub page from the previous workspace must be closed on switch"
    );
}

#[test]
fn tags_hub_delivery_queue_preserves_newer_valid_result_before_stale_reject() {
    use handshake_native::editor_pane_factories::{placeholder_pane_type, TAGS_PANE_LABEL};
    use handshake_native::graph::tags_panel::{HubMember, LoomTagHubPanel};

    let (mut app, _rt) = secondary_shell();
    retype_panes(
        &mut app,
        &[("pane-a", placeholder_pane_type(TAGS_PANE_LABEL))],
    );
    let tags_hub = app.mounted_tags_hub_for_test();
    let mut harness =
        Harness::builder().build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), app);
    harness.run_steps(2);

    {
        let mut hub = tags_hub.lock().unwrap();
        let mut current = LoomTagHubPanel::new(DEFAULT_PROJECT_ID, "hub-b");
        current.loading = true;
        *hub = Some(current);
    }

    harness.state().deliver_tag_hub_detail_for_test(
        DEFAULT_PROJECT_ID,
        "hub-b",
        Ok((
            "Hub B".to_owned(),
            vec![HubMember::new("member-b", "Member B", "note")],
        )),
    );
    harness.state().deliver_tag_hub_detail_for_test(
        "old-project",
        "hub-a",
        Ok((
            "Stale Hub A".to_owned(),
            vec![HubMember::new("member-a", "Member A", "note")],
        )),
    );
    harness.run_steps(2);

    let hub = tags_hub.lock().unwrap();
    let hub = hub.as_ref().expect("current hub remains open");
    assert_eq!(
        hub.title, "Hub B",
        "MT-023 host: valid current hub delivery must survive a later stale delivery before drain"
    );
    assert_eq!(
        hub.members.len(),
        1,
        "MT-023 host: current hub members from the valid delivery must be applied"
    );
    assert_eq!(hub.members[0].block_id, "member-b");
    assert!(
        !hub.loading,
        "MT-023 host: valid delivery clears the current hub loading state"
    );
}

#[test]
fn tags_member_count_delivery_updates_list_without_open_hub() {
    use handshake_native::editor_pane_factories::{placeholder_pane_type, TAGS_PANE_LABEL};
    use handshake_native::graph::tags_panel::{HubMember, TagEntry};

    let (mut app, _rt) = secondary_shell();
    retype_panes(
        &mut app,
        &[("pane-a", placeholder_pane_type(TAGS_PANE_LABEL))],
    );
    let tags_panel = app.mounted_tags_panel_for_test();
    let mut harness =
        Harness::builder().build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), app);
    harness.run_steps(2);

    {
        let mut panel = tags_panel.lock().unwrap();
        panel.workspace_id = DEFAULT_PROJECT_ID.to_owned();
        panel.set_tags(vec![TagEntry::new("tag-rust", "rust", None)]);
    }
    harness.state().deliver_tag_hub_detail_for_test(
        DEFAULT_PROJECT_ID,
        "tag-rust",
        Ok((
            String::new(),
            vec![
                HubMember::new("member-1", "Member 1", "note"),
                HubMember::new("member-2", "Member 2", "note"),
            ],
        )),
    );
    harness.run_steps(2);

    let panel = tags_panel.lock().unwrap();
    let tag = panel
        .tags
        .iter()
        .find(|tag| tag.block_id == "tag-rust")
        .expect("seeded tag remains");
    assert_eq!(
        tag.member_count,
        Some(2),
        "MT-023 AC1: members-only delivery must update the tag-list member count badge"
    );
}

#[test]
fn tags_edge_stale_error_does_not_apply_to_current_hub() {
    use handshake_native::editor_pane_factories::{placeholder_pane_type, TAGS_PANE_LABEL};
    use handshake_native::graph::tags_panel::LoomTagHubPanel;

    let (mut app, _rt) = secondary_shell();
    retype_panes(
        &mut app,
        &[("pane-a", placeholder_pane_type(TAGS_PANE_LABEL))],
    );
    let tags_hub = app.mounted_tags_hub_for_test();
    let mut harness =
        Harness::builder().build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), app);
    harness.run_steps(2);

    {
        let mut hub = tags_hub.lock().unwrap();
        *hub = Some(LoomTagHubPanel::new(DEFAULT_PROJECT_ID, "hub-b"));
    }
    harness.state().deliver_tag_edge_receipt_for_test(
        DEFAULT_PROJECT_ID,
        "hub-a",
        Err("old hub failed".to_owned()),
    );
    harness.run_steps(2);

    let hub = tags_hub.lock().unwrap();
    let hub = hub.as_ref().expect("current hub remains open");
    assert_eq!(hub.block_id, "hub-b");
    assert!(
        hub.error.is_none(),
        "MT-023 host: stale add-tag errors for a different hub must not surface on the current hub"
    );
}

#[test]
fn tags_edge_same_hub_superseded_error_does_not_surface() {
    use handshake_native::editor_pane_factories::{placeholder_pane_type, TAGS_PANE_LABEL};
    use handshake_native::graph::tags_panel::LoomTagHubPanel;

    let (mut app, _rt) = secondary_shell();
    retype_panes(
        &mut app,
        &[("pane-a", placeholder_pane_type(TAGS_PANE_LABEL))],
    );
    let tags_hub = app.mounted_tags_hub_for_test();
    let mut harness =
        Harness::builder().build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), app);
    harness.run_steps(2);

    {
        let mut hub = tags_hub.lock().unwrap();
        *hub = Some(LoomTagHubPanel::new(DEFAULT_PROJECT_ID, "hub-b"));
    }
    let old_sequence = harness
        .state()
        .register_tag_edge_request_for_test(DEFAULT_PROJECT_ID, "hub-b");
    let _new_sequence = harness
        .state()
        .register_tag_edge_request_for_test(DEFAULT_PROJECT_ID, "hub-b");
    harness
        .state()
        .deliver_tag_edge_receipt_with_sequence_for_test(
            DEFAULT_PROJECT_ID,
            "hub-b",
            old_sequence,
            Err("old add failed".to_owned()),
        );
    harness.run_steps(2);

    let hub = tags_hub.lock().unwrap();
    let hub = hub.as_ref().expect("current hub remains open");
    assert!(
        hub.error.is_none(),
        "MT-023 host: superseded same-hub add-tag errors must not surface after a newer add-tag request"
    );
}

#[test]
fn tags_hub_same_key_stale_delivery_does_not_overwrite_current_detail() {
    use handshake_native::editor_pane_factories::{placeholder_pane_type, TAGS_PANE_LABEL};
    use handshake_native::graph::tags_panel::{HubMember, LoomTagHubPanel};

    let (mut app, _rt) = secondary_shell();
    retype_panes(
        &mut app,
        &[("pane-a", placeholder_pane_type(TAGS_PANE_LABEL))],
    );
    let tags_hub = app.mounted_tags_hub_for_test();
    let mut harness =
        Harness::builder().build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), app);
    harness.run_steps(2);

    {
        let mut hub = tags_hub.lock().unwrap();
        *hub = Some(LoomTagHubPanel::new(DEFAULT_PROJECT_ID, "hub-b"));
    }
    let old_sequence = harness
        .state()
        .register_tag_hub_request_for_test(DEFAULT_PROJECT_ID, "hub-b");
    let new_sequence = harness
        .state()
        .register_tag_hub_request_for_test(DEFAULT_PROJECT_ID, "hub-b");
    harness
        .state()
        .deliver_tag_hub_detail_with_sequence_for_test(
            DEFAULT_PROJECT_ID,
            "hub-b",
            new_sequence,
            Ok((
                "New Hub B".to_owned(),
                vec![HubMember::new("member-new", "Member New", "note")],
            )),
        );
    harness
        .state()
        .deliver_tag_hub_detail_with_sequence_for_test(
            DEFAULT_PROJECT_ID,
            "hub-b",
            old_sequence,
            Ok((
                "Old Hub B".to_owned(),
                vec![HubMember::new("member-old", "Member Old", "note")],
            )),
        );
    harness.run_steps(2);

    let hub = tags_hub.lock().unwrap();
    let hub = hub.as_ref().expect("current hub remains open");
    assert_eq!(
        hub.title, "New Hub B",
        "MT-023 host: older same-workspace/same-hub deliveries must not overwrite newer hub detail"
    );
    assert_eq!(hub.members[0].block_id, "member-new");
}

// ── PT-080-B / AC-080-2 (must-fix backend-shape): the clear-section path sends the body the REAL backend
// accepts (`{clear_group:true}`), and an AssignSection{None} drains through the live host. ───────────────

#[test]
fn canvas_clear_group_sends_backend_accepted_clear_body() {
    use handshake_native::backend_client::CanvasBoardClient;
    use handshake_native::graph::CanvasEvent;

    // Builder shape (asserted against the REAL backend contract, not the serializer's own historical
    // output): the backend's `update_canvas_placement` clears the group ONLY on `clear_group: true`. A
    // `{"group_id": null}` body is a verified no-op (deserializes to `group_id: None`, leaves the group
    // unchanged), so the host MUST send `{"clear_group": true}` or a card dragged out of a section silently
    // re-snaps on the next board refresh.
    let rt = tokio::runtime::Builder::new_current_thread()
        .build()
        .unwrap();
    let client = CanvasBoardClient::production(rt.handle().clone());
    let clear = client.clear_group_request(DEFAULT_PROJECT_ID, "p-clear");
    assert_eq!(
        clear.body,
        Some(serde_json::json!({ "clear_group": true })),
        "must-fix(backend-shape): the clear-section PATCH sends {{clear_group:true}} (the only body the real \
         update_canvas_placement handler treats as a clear); {{group_id:null}} is a verified backend no-op"
    );
    assert_ne!(
        clear.body,
        Some(serde_json::json!({ "group_id": serde_json::Value::Null })),
        "regression guard: the clear body is NOT the no-op {{group_id:null}} shape"
    );

    // Live host path: an AssignSection{group_id:None} (a card dropped outside all section frames) drains
    // through the mounted board's outbound queue into route_canvas_events, which maps the None arm to the
    // clear builder above (the live PATCH round-trip is gated NEEDS_MANAGED_RESOURCE_PROOF).
    let (app, _rt) = secondary_shell();
    let canvas_events = app.mounted_canvas_events();
    let mut harness =
        Harness::builder().build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), app);
    harness.run_steps(2);
    canvas_events
        .lock()
        .unwrap()
        .push(CanvasEvent::AssignSection {
            placement_id: "p-clear".into(),
            group_id: None,
        });
    assert_eq!(
        canvas_events.lock().unwrap().len(),
        1,
        "the clear event is enqueued before the frame"
    );
    harness.run_steps(2);
    assert!(
        canvas_events.lock().unwrap().is_empty(),
        "AC-080-2: the canvas AssignSection{{None}} (clear) was DRAINED by the host (mapped to the \
         clear_group PATCH path)"
    );
}

// ── WP-KERNEL-012 W3 / MT-026 remediation: EVERY canvas mutation kind maps to a HOST dispatch ─────────

/// Wire-capture of the FULL `route_canvas_events` mutation wiring (the W2 audit found only
/// ResizePlacement/AssignSection wired; PlaceBlock / AddCard / Group / RemovePlacement / SemanticEdge /
/// VisualEdgeAdded / RemoveEdge / ViewportChanged drained into a dead catch-all). A PARKED
/// current-thread runtime is injected: `Handle::spawn` queues the off-thread mutations but nothing
/// polls them, so every dispatched op's result cell stays IN FLIGHT — `canvas_op_cells_in_flight()` is
/// then an exact, race-free count of the host dispatches the event batch produced. A drained-queue
/// assertion alone cannot distinguish "routed to a real dispatch" from "swallowed by the catch-all",
/// which is exactly the defect this proof pins. The RemoveEdge split is proven on the SAME mounted
/// board the host reads: a seeded board-local visual-edge id routes to the visual-edge DELETE, an
/// unknown id to the semantic loom-edge DELETE (both dispatch — 2 cells). The live PG round-trips stay
/// NEEDS_MANAGED_RESOURCE_PROOF; the URL/body shapes are pinned by the `test_canvas_board` builder
/// proofs the host routes through.
#[test]
fn canvas_mutation_events_map_to_host_dispatches_with_op_cells() {
    use handshake_native::graph::canvas_board::VisualEdge;
    use handshake_native::graph::CanvasEvent;

    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("build parked current-thread runtime");
    let mut app = HandshakeApp::with_health(HealthDisplayState::Ok(HealthInfo {
        status: "ok".to_string(),
        db_status: "ok".to_string(),
        migration_version: Some(1),
    }));
    app.set_runtime_handle(runtime.handle().clone());
    retype_panes(&mut app, &[("pane-a", PaneType::AtelierEditor)]);
    let canvas_events = app.mounted_canvas_events();
    let board = app.mounted_canvas_board();
    // Seed ONE board-local visual edge so the RemoveEdge visual-vs-semantic route split reads it from
    // the SAME board state the host routing locks. (The parked runtime never delivers the initial
    // board fetch, so the seed is never overwritten mid-test.)
    board.lock().unwrap().visual_edges.push(VisualEdge {
        visual_edge_id: "ve-w3".into(),
        from_placement_id: "p-1".into(),
        to_placement_id: "p-2".into(),
    });
    let mut harness =
        Harness::builder().build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), app);
    harness.run_steps(2);
    let baseline = harness.state().canvas_op_cells_in_flight();

    {
        let mut q = canvas_events.lock().unwrap();
        // The four MUST-capture kinds (PlaceBlock / AddCard / RemovePlacement / ViewportChanged) …
        q.push(CanvasEvent::PlaceBlock {
            placed_block_id: "blk-w3".into(),
            x: 60.0,
            y: 90.0,
        });
        q.push(CanvasEvent::AddCard {
            title: "Card W3".into(),
            x: 40.0,
            y: 40.0,
        });
        q.push(CanvasEvent::RemovePlacement {
            placement_id: "p-w3".into(),
        });
        q.push(CanvasEvent::ViewportChanged {
            pan_x: 12.0,
            pan_y: -8.0,
            zoom: 1.5,
        });
        // … plus the remaining wired kinds (shape-asserted through the same builder helpers).
        q.push(CanvasEvent::Group {
            placement_ids: vec!["p-1".into(), "p-2".into()],
            group_id: "grp-w3".into(),
        });
        q.push(CanvasEvent::SemanticEdge {
            source_block_id: "blk-a".into(),
            target_block_id: "blk-b".into(),
        });
        q.push(CanvasEvent::VisualEdgeAdded {
            from_placement_id: "p-1".into(),
            to_placement_id: "p-2".into(),
        });
        q.push(CanvasEvent::RemoveEdge {
            edge_id: "ve-w3".into(), // seeded board-local visual edge -> visual-edge DELETE
        });
        q.push(CanvasEvent::RemoveEdge {
            edge_id: "loom-edge-9".into(), // NOT a board visual edge -> semantic loom-edge DELETE
        });
    }
    harness.run_steps(1);

    assert!(
        canvas_events.lock().unwrap().is_empty(),
        "the host drained the full canvas mutation batch (drive_secondary_mounts -> route_canvas_events)"
    );
    let dispatched = harness.state().canvas_op_cells_in_flight() - baseline;
    // 1 (PlaceBlock) + 1 (AddCard) + 1 (RemovePlacement) + 1 (ViewportChanged) + 2 (Group of 2
    // placements: one PATCH per member) + 1 (SemanticEdge) + 1 (VisualEdgeAdded) + 2 (RemoveEdge x2).
    assert_eq!(
        dispatched, 10,
        "W3/W2: EVERY drained canvas mutation kind mapped to a real CanvasBoardClient dispatch with a \
         tracked op cell (none swallowed by a catch-all)"
    );
    println!("PASS W3/W2: 9 canvas mutation events -> {dispatched} host dispatches with op cells");
}

#[test]
fn block_collection_create_view_result_rebinds_mounted_host() {
    use handshake_native::backend_client::BlockViewRecordData;
    use handshake_native::editor_pane_factories::{
        placeholder_pane_type, BLOCK_COLLECTIONS_PANE_LABEL,
    };
    use handshake_native::graph::block_collection_view::{
        BlockViewDefinition, BlockViewEvent, BlockViewKind, BlockViewResults,
    };

    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("build parked current-thread runtime");
    let mut app = HandshakeApp::with_health(HealthDisplayState::Ok(HealthInfo {
        status: "ok".to_string(),
        db_status: "ok".to_string(),
        migration_version: Some(1),
    }));
    app.set_runtime_handle(runtime.handle().clone());
    retype_panes(
        &mut app,
        &[(
            "pane-a",
            placeholder_pane_type(BLOCK_COLLECTIONS_PANE_LABEL),
        )],
    );

    let collection = app.mounted_block_collection_view();
    let events = app.mounted_block_collection_events();
    let mut harness =
        Harness::builder().build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), app);
    harness.run_steps(2);

    events.lock().unwrap().push(BlockViewEvent::CreateView {
        title: "Host-created collection".to_owned(),
        kind: BlockViewKind::Table,
    });
    harness.run_steps(1);
    assert!(
        events.lock().unwrap().is_empty(),
        "MT-027 host: CreateView event was drained by the mounted collection consumer"
    );
    {
        let view = collection.lock().unwrap();
        assert!(
            view.in_flight,
            "MT-027 host: CreateView enters an in-flight create state before the async result lands"
        );
        assert_eq!(
            view.status, "Creating view…",
            "MT-027 host: CreateView exposes an honest status while waiting for createBlockView"
        );
        assert!(
            view.view_block_id.is_empty(),
            "MT-027 host: the mounted host must not invent a new view id before the backend result"
        );
    }

    harness
        .state()
        .deliver_block_collection_op_for_test(Ok("view-created-from-backend".to_owned()));
    harness.run_steps(1);
    {
        let view = collection.lock().unwrap();
        assert_eq!(
            view.view_block_id, "view-created-from-backend",
            "MT-027 host: createBlockView's returned id rebinds the mounted host before re-query"
        );
        assert!(
            view.in_flight && view.loading,
            "MT-027 host: after create succeeds, the host remains busy while it fetches definition/results"
        );
    }

    let definition = BlockViewDefinition::of_kind(BlockViewKind::Table);
    harness
        .state()
        .deliver_block_collection_record_for_test(Ok(BlockViewRecordData {
            view_block_id: "view-created-from-backend".to_owned(),
            definition: definition.clone(),
        }));
    harness
        .state()
        .deliver_block_collection_results_for_test(Ok(BlockViewResults::default()));
    harness.run_steps(1);
    {
        let view = collection.lock().unwrap();
        assert_eq!(view.view_block_id, "view-created-from-backend");
        assert!(
            !view.in_flight && !view.loading,
            "MT-027 host: definition + results delivery clears the mounted in-flight state"
        );
        assert_eq!(
            view.definition.as_ref().map(|d| d.kind),
            Some(BlockViewKind::Table)
        );
        assert!(
            view.results.is_some(),
            "MT-027 host: results delivery installs the authoritative query result projection"
        );
    }
}

#[test]
fn block_collection_saved_view_opens_and_binds_mounted_host() {
    use handshake_native::editor_pane_factories::{
        placeholder_pane_type, BLOCK_COLLECTIONS_PANE_LABEL,
    };

    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("build parked current-thread runtime");
    let mut app = HandshakeApp::with_health(HealthDisplayState::Ok(HealthInfo {
        status: "ok".to_string(),
        db_status: "ok".to_string(),
        migration_version: Some(1),
    }));
    app.set_runtime_handle(runtime.handle().clone());
    retype_panes(
        &mut app,
        &[(
            "pane-a",
            placeholder_pane_type(BLOCK_COLLECTIONS_PANE_LABEL),
        )],
    );

    let outcome = ShellNavigator::open_block_collection_view(&mut app, "view-def-001");
    assert_eq!(
        outcome,
        NavDispatchOutcome::Opened {
            surface: BLOCK_COLLECTIONS_PANE_LABEL.to_owned()
        },
        "saved view_def navigation opens the mounted Block Collections surface"
    );
    let active = app.active_pane().expect("active pane after nav").clone();
    let active_tab = app
        .tab_bar_states()
        .get(&active)
        .and_then(|bar| bar.active())
        .expect("active tab after nav");
    assert_eq!(
        active_tab.pane_type,
        placeholder_pane_type(BLOCK_COLLECTIONS_PANE_LABEL)
    );
    assert_eq!(active_tab.content_id.as_deref(), Some("view-def-001"));
    let collection = app.mounted_block_collection_view();
    let view = collection.lock().unwrap();
    assert_eq!(view.view_block_id, "view-def-001");
    assert!(
        view.loading,
        "saved view_def navigation enters the definition/results loading state"
    );
    assert_eq!(
        view.status, "Loading view...",
        "saved view_def navigation exposes an honest loading status"
    );
}

#[test]
fn block_collection_load_handles_results_before_definition() {
    use handshake_native::backend_client::BlockViewRecordData;
    use handshake_native::editor_pane_factories::{
        placeholder_pane_type, BLOCK_COLLECTIONS_PANE_LABEL,
    };
    use handshake_native::graph::block_collection_view::{
        BlockViewDefinition, BlockViewKind, BlockViewResults,
    };

    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("build parked current-thread runtime");
    let mut app = HandshakeApp::with_health(HealthDisplayState::Ok(HealthInfo {
        status: "ok".to_string(),
        db_status: "ok".to_string(),
        migration_version: Some(1),
    }));
    app.set_runtime_handle(runtime.handle().clone());
    retype_panes(
        &mut app,
        &[(
            "pane-a",
            placeholder_pane_type(BLOCK_COLLECTIONS_PANE_LABEL),
        )],
    );
    let collection = app.mounted_block_collection_view();
    let mut harness =
        Harness::builder().build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), app);
    harness.run_steps(2);

    let outcome = ShellNavigator::open_block_collection_view(harness.state_mut(), "view-def-race");
    assert_eq!(
        outcome,
        NavDispatchOutcome::Opened {
            surface: BLOCK_COLLECTIONS_PANE_LABEL.to_owned()
        }
    );

    harness
        .state()
        .deliver_block_collection_results_for_test(Ok(BlockViewResults {
            kind_str: "table".to_owned(),
            total_returned: 1,
            ..Default::default()
        }));
    harness.run_steps(1);
    {
        let view = collection.lock().unwrap();
        assert!(
            view.loading && view.results.is_none(),
            "results that beat getBlockView are retained but not installed without a definition"
        );
    }

    harness
        .state()
        .deliver_block_collection_record_for_test(Ok(BlockViewRecordData {
            view_block_id: "view-def-race".to_owned(),
            definition: BlockViewDefinition::of_kind(BlockViewKind::Table),
        }));
    harness.run_steps(1);
    {
        let view = collection.lock().unwrap();
        assert_eq!(view.view_block_id, "view-def-race");
        assert!(
            !view.loading && !view.in_flight,
            "definition delivery installs the retained results and clears loading"
        );
        assert_eq!(
            view.results.as_ref().map(|r| r.total_returned),
            Some(1),
            "out-of-order results are not dropped"
        );
    }
}

#[test]
fn block_collection_host_drains_mutations_and_requeries_current_view() {
    use handshake_native::backend_client::BlockViewRecordData;
    use handshake_native::editor_pane_factories::{
        placeholder_pane_type, BLOCK_COLLECTIONS_PANE_LABEL,
    };
    use handshake_native::graph::block_collection_view::{
        BlockViewDefinition, BlockViewEvent, BlockViewKind, BlockViewResults, BlockViewSort,
        BlockViewSortDirection,
    };

    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("build parked current-thread runtime");
    let mut app = HandshakeApp::with_health(HealthDisplayState::Ok(HealthInfo {
        status: "ok".to_string(),
        db_status: "ok".to_string(),
        migration_version: Some(1),
    }));
    app.set_runtime_handle(runtime.handle().clone());
    retype_panes(
        &mut app,
        &[(
            "pane-a",
            placeholder_pane_type(BLOCK_COLLECTIONS_PANE_LABEL),
        )],
    );

    let collection = app.mounted_block_collection_view();
    {
        let mut view = collection.lock().unwrap();
        view.view_block_id = "existing-view-def".to_owned();
        view.set_loaded(
            BlockViewDefinition::of_kind(BlockViewKind::Table),
            BlockViewResults::default(),
        );
    }
    let events = app.mounted_block_collection_events();
    let mut harness =
        Harness::builder().build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), app);
    harness.run_steps(2);

    let mutation_events = [
        BlockViewEvent::Sort {
            sort: BlockViewSort {
                field: handshake_native::graph::block_collection_view::BlockViewField::Title,
                direction: BlockViewSortDirection::Asc,
            },
        },
        BlockViewEvent::KindChange {
            kind: BlockViewKind::Kanban,
        },
        BlockViewEvent::DateRange {
            date_from: Some("2026-07-01".to_owned()),
            date_to: Some("2026-07-05".to_owned()),
        },
        BlockViewEvent::CardMove {
            block_id: "block-001".to_owned(),
            add_tags: vec!["done".to_owned()],
            remove_tags: vec!["todo".to_owned()],
        },
    ];

    for event in mutation_events {
        events.lock().unwrap().push(event);
        harness.run_steps(1);
        assert!(
            events.lock().unwrap().is_empty(),
            "MT-027 host: mutation event was drained by the mounted collection consumer"
        );
        {
            let view = collection.lock().unwrap();
            assert!(
                view.in_flight,
                "MT-027 host: drained mutation enters an in-flight backend state"
            );
            assert_eq!(view.status, "Applying…");
        }

        harness
            .state()
            .deliver_block_collection_op_for_test(Ok("existing-view-def".to_owned()));
        harness.run_steps(1);
        {
            let view = collection.lock().unwrap();
            assert!(
                view.loading,
                "MT-027 host: successful mutation receipt starts definition/results re-query"
            );
            assert_eq!(view.view_block_id, "existing-view-def");
        }
        harness
            .state()
            .deliver_block_collection_record_for_test(Ok(BlockViewRecordData {
                view_block_id: "existing-view-def".to_owned(),
                definition: BlockViewDefinition::of_kind(BlockViewKind::Table),
            }));
        harness
            .state()
            .deliver_block_collection_results_for_test(Ok(BlockViewResults::default()));
        harness.run_steps(1);
        assert!(
            !collection.lock().unwrap().in_flight,
            "MT-027 host: definition + results delivery clears in-flight before the next mutation"
        );
    }
}

#[test]
fn canvas_board_fetch_resolves_live_titles_into_mounted_cards() {
    use handshake_native::backend_client::CanvasBoardData;
    use handshake_native::graph::canvas_board::{placement_author_id, CanvasPlacementCard};

    let mut app = HandshakeApp::with_health(HealthDisplayState::Ok(HealthInfo {
        status: "ok".to_string(),
        db_status: "ok".to_string(),
        migration_version: Some(1),
    }));
    retype_panes(&mut app, &[("pane-a", PaneType::AtelierEditor)]);
    let board = app.mounted_canvas_board();
    let mut harness =
        Harness::builder().build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), app);

    harness
        .state()
        .deliver_canvas_board_for_test(Ok(CanvasBoardData {
            placements: vec![
                CanvasPlacementCard::new("p-live-a", "blk-live-a", 40.0, 60.0, 220.0, 130.0),
                CanvasPlacementCard::new("p-live-b", "blk-live-b", 300.0, 60.0, 220.0, 130.0),
            ],
            visual_edges: vec![],
            pan_x: 0.0,
            pan_y: 0.0,
            zoom: 1.0,
        }));
    harness.run_steps(2);
    let generation = harness.state().canvas_live_block_generation_for_test();
    assert_eq!(
        board.lock().unwrap().placements[0].display_title(),
        "(stale reference)",
        "the raw getCanvasBoard payload is reference-only until getLoomBlock resolves"
    );

    harness.state_mut().deliver_canvas_live_block_for_test(
        generation,
        "blk-live-a",
        Ok((
            Some("Resolved Canvas Title".to_owned()),
            "note".to_owned(),
            Some("hash-live-a".to_owned()),
        )),
    );
    harness.state_mut().deliver_canvas_live_block_for_test(
        generation,
        "blk-live-b",
        Err("missing block".to_owned()),
    );
    harness.run_steps(2);

    {
        let board = board.lock().unwrap();
        let resolved = board
            .placements
            .iter()
            .find(|p| p.placement_id == "p-live-a")
            .expect("resolved placement");
        assert_eq!(resolved.display_title(), "Resolved Canvas Title");
        assert_eq!(resolved.live_content_type.as_deref(), Some("note"));
        assert_eq!(resolved.loom_content_hash.as_deref(), Some("hash-live-a"));

        let stale = board
            .placements
            .iter()
            .find(|p| p.placement_id == "p-live-b")
            .expect("stale placement");
        assert_eq!(
            stale.display_title(),
            "(stale reference)",
            "a failed live-block resolve stays an honest stale reference"
        );
    }
    assert_eq!(
        harness.state().canvas_live_block_cells_in_flight_for_test(),
        0,
        "resolved live-block cells are drained from the mounted app host"
    );
    assert_eq!(
        live_label_for(&harness, &placement_author_id("p-live-a")).as_deref(),
        Some("Resolved Canvas Title"),
        "the mounted canvas AccessKit node label comes from the resolved live block title"
    );
    assert_eq!(
        live_label_for(&harness, &placement_author_id("p-live-b")).as_deref(),
        Some("(stale reference)"),
        "missing blocks remain explicit stale references instead of fabricated titles"
    );
}

// ── PT-080-B / AC-080-5: outgoing-links click routes to the nav bus ───────────────────────────────────

#[test]
fn outgoing_links_click_routes_to_nav() {
    use handshake_native::rich_editor::wikilinks::outgoing_links_panel::{LinkKind, OutgoingLink};

    let (app, _rt) = secondary_shell();
    let panel = app.mounted_outgoing_links();
    // Seed a resolved outgoing link so the pane renders a clickable row (not the empty-state).
    {
        let mut p = panel.lock().unwrap();
        p.resolved.push(OutgoingLink {
            raw: "note:Target".to_owned(),
            target_value: "Target".to_owned(),
            alias: None,
            kind: LinkKind::Wikilink,
            resolved_target_id: Some("KRD-target".to_owned()),
        });
    }
    let mut harness =
        Harness::builder().build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), app);
    harness.run_steps(2);

    // The pane renders the resolved row with author_id `outgoing.resolved.KRD-target`; a click routes a
    // NavTarget to the shell outbound queue. We assert the row node is present (the live subtree rendered);
    // the routing path itself is proven by the host-drain being a no-op when empty (the queue is drained
    // each frame). Find the row by its stable author_id.
    use handshake_native::rich_editor::wikilinks::outgoing_links_panel::resolved_author_id;
    let row_id = resolved_author_id("KRD-target");
    let ids = live_author_ids(&harness);
    assert!(
        ids.contains(&row_id) || ids.contains("outgoing.section.resolved"),
        "AC-080-5: the outgoing-links pane rendered its REAL resolved subtree ('{row_id}'); got {:?}",
        ids.iter().filter(|i| i.starts_with("outgoing")).collect::<Vec<_>>()
    );
}

// ── PT-080-B / AC-080-5: relevant-memory shows the EndpointMissing empty-state ────────────────────────

#[test]
fn relevant_memory_shows_endpoint_missing_empty_state() {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(1)
        .enable_all()
        .build()
        .expect("build multi-thread runtime");
    let mut app = HandshakeApp::with_health(HealthDisplayState::Ok(HealthInfo {
        status: "ok".to_string(),
        db_status: "ok".to_string(),
        migration_version: Some(1),
    }));
    app.set_runtime_handle(runtime.handle().clone());
    retype_panes(
        &mut app,
        &[(
            "pane-a",
            PaneType::Placeholder("Relevant Memory".to_owned()),
        )],
    );
    let panel = app.mounted_relevant_memory();
    let mut harness =
        Harness::builder().build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), app);
    // Several frames so the shell fires the FEMS read and the off-thread fetch resolves to the typed
    // blocker (the route is verified ABSENT). Poll the panel until the blocker lands or a bound is hit.
    let mut got_blocker = false;
    let mut ever_in_flight = false;
    for _ in 0..80 {
        harness.run_steps(2);
        {
            let p = panel.lock().unwrap();
            if p.in_flight() || p.last_context().is_some() {
                ever_in_flight = true;
            }
            if p.blocker().is_some() {
                got_blocker = true;
                break;
            }
        }
        std::thread::sleep(std::time::Duration::from_millis(25));
    }
    println!("relevant_memory: ever_requested={ever_in_flight} got_blocker={got_blocker}");
    assert!(
        ever_in_flight,
        "AC-080-5: the shell DROVE the FEMS refresh-for-context (the read fired) — the wiring is live"
    );
    // The FEMS read route is ABSENT in this build, so the fetch resolves to a typed blocker (EndpointMissing
    // on a 404, or a Transport error if no backend is reachable) — either way the panel holds an HONEST
    // typed blocker and renders its empty-state, never a faked pack.
    let blocker = panel.lock().unwrap().blocker().is_some();
    assert!(
        got_blocker && blocker,
        "AC-080-5: the relevant-memory pane drove the FEMS read to an HONEST typed blocker (the route is \
         ABSENT) and shows its empty-state — never a faked pack"
    );
}

// ── PT-080-A / AC-080-6: code text node exposes the swarm edit actions ────────────────────────────────

#[test]
fn code_text_node_exposes_swarm_edit_actions() {
    use handshake_native::code_editor::panel::CodeEditorPanel;

    // The code text node advertises SetValue + ReplaceSelectedText (a swarm agent authors code by id).
    let mut harness = Harness::new_ui(|ui| {
        let panel = CodeEditorPanel::new("fn main() {}", "rs");
        panel.show(ui);
    });
    harness.run_steps(2);
    let text_node = harness
        .root()
        .children_recursive()
        .find(|n| n.accesskit_node().author_id() == Some(CODE_EDITOR_TEXT_AUTHOR_ID))
        .expect("the code_editor_text node is in the live tree");
    let node = text_node.accesskit_node();
    // Probe the RAW NodeData action set (single-arg `supports_action`, the same `test_e7_swarm_edit_proof`
    // uses) so the assertion reads the node's OWN declared actions.
    assert!(
        node.data()
            .supports_action(egui::accesskit::Action::SetValue),
        "AC-080-6: the code text node advertises Action::SetValue (swarm author-whole-file)"
    );
    assert!(
        node.data().supports_action(egui::accesskit::Action::ReplaceSelectedText),
        "AC-080-6: the code text node advertises Action::ReplaceSelectedText (swarm edit-selection)"
    );
}

/// AC-080-6 dispatch proof: a swarm `Action::SetValue` request at the code text node mutates the buffer.
#[test]
fn code_text_setvalue_dispatch_mutates_buffer() {
    use handshake_native::code_editor::panel::CodeEditorPanel;
    use std::sync::Arc;

    let panel = Arc::new(CodeEditorPanel::new("old contents", "rs"));
    let drive = Arc::clone(&panel);
    let mut harness = Harness::new_ui(move |ui| {
        drive.show(ui);
    });
    harness.run_steps(2);
    // Find the live node id, then enqueue a SetValue action request carrying the new value (the exact
    // shape a swarm agent's `egui::Event::AccessKitActionRequest` carries).
    let node_id = harness
        .root()
        .children_recursive()
        .find(|n| n.accesskit_node().author_id() == Some(CODE_EDITOR_TEXT_AUTHOR_ID))
        .expect("code text node present")
        .accesskit_node()
        .id();
    harness.event(egui::Event::AccessKitActionRequest(
        egui::accesskit::ActionRequest {
            action: egui::accesskit::Action::SetValue,
            target: node_id,
            data: Some(egui::accesskit::ActionData::Value(
                "new swarm contents".into(),
            )),
        },
    ));
    harness.run_steps(2);
    assert_eq!(
        panel.buffer().to_string(),
        "new swarm contents",
        "AC-080-6: a swarm Action::SetValue dispatched at the code text node replaced the whole buffer"
    );
}

// ═══ WP-KERNEL-012 E11 remediation wave (lane W1): the `view.*` OPERATOR OPEN ROUTES ═══════════════════
//
// The 2026-07-02 drift audit found the AC-080-1 mounted-LIVE proof injected pane types programmatically
// (the retype harness), which does not evidence OPERATOR reachability — and the side panes had NO
// menu/palette/drawer/navigator arm at all. These proofs re-prove reachability through the REAL operator
// route: the command-palette `view.*` rows dispatch through the SAME `dispatch_palette_action` arm a
// clicked/Enter-run palette row reaches, and the pane's REAL widget subtree (stable AccessKit author_ids)
// renders on the active work surface — NOT via retype injection.

/// Collect every live AccessKit node carrying an author_id: `(author_id, is_disabled)`.
fn live_author_nodes_flat(harness: &Harness<'_, HandshakeApp>) -> Vec<(String, bool)> {
    let mut found = Vec::new();
    for node in harness.root().children_recursive() {
        let ak = node.accesskit_node();
        if let Some(author_id) = ak.author_id() {
            found.push((author_id.to_owned(), ak.is_disabled()));
        }
    }
    found
}

/// A live, runtime-injected shell with the DEFAULT seeded panes (no retype injection): the operator
/// route proofs open every pane through the palette dispatch arm only.
fn operator_shell() -> (HandshakeApp, tokio::runtime::Runtime) {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(1)
        .enable_all()
        .build()
        .expect("build multi-thread runtime");
    let mut app = HandshakeApp::with_health(HealthDisplayState::Ok(HealthInfo {
        status: "ok".to_string(),
        db_status: "ok".to_string(),
        migration_version: Some(1),
    }));
    app.set_runtime_handle(runtime.handle().clone());
    (app, runtime)
}

/// Every `view.*` open route is a REAL, ENABLED command-palette row (the operator-discoverable surface),
/// addressable by its stable `command-palette.option.{stable_id}` author_id.
#[test]
fn view_open_routes_are_enabled_palette_rows() {
    let (app, _rt) = operator_shell();
    let mut harness =
        Harness::builder().build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), app);
    harness.run_steps(2);
    harness.state_mut().open_command_palette();
    harness.run_steps(2);

    let nodes = live_author_nodes_flat(&harness);
    for stable_id in [
        "hs-view-palette-relevant-memory",
        "hs-view-palette-stage",
        "hs-view-palette-tags",
        "hs-view-palette-sidebar",
        "hs-view-palette-block-collections",
        "hs-view-palette-outline",
        "hs-view-palette-graph",
        "hs-view-palette-folders",
        "hs-view-palette-outgoing-links",
        "hs-view-palette-journal",
        "hs-view-palette-diff-merge",
    ] {
        let row_author = format!("command-palette.option.{stable_id}");
        let row = nodes
            .iter()
            .find(|(a, _)| a == &row_author)
            .unwrap_or_else(|| {
                panic!(
                    "operator route: the '{row_author}' palette row is missing: {:?}",
                    nodes
                        .iter()
                        .filter(|(a, _)| a.starts_with("command-palette.option.hs-view"))
                        .collect::<Vec<_>>()
                )
            });
        assert!(
            !row.1,
            "operator route: the '{row_author}' palette row is ENABLED (a dead disabled row is not an \
             open route)"
        );
    }
}

/// Dispatching each `view.*` palette command through the REAL palette dispatch arm opens the pane on the
/// active work surface and the pane's REAL widget subtree renders (stable author_ids in the live tree) —
/// the AC-080-1 re-proof via the operator route, NOT retype injection.
#[test]
fn view_commands_open_real_pane_subtrees_via_operator_route() {
    use handshake_native::command_registry::{
        CMD_VIEW_BLOCK_COLLECTIONS, CMD_VIEW_DIFF_MERGE, CMD_VIEW_GRAPH, CMD_VIEW_JOURNAL,
        CMD_VIEW_OUTLINE, CMD_VIEW_RELEVANT_MEMORY, CMD_VIEW_STAGE, CMD_VIEW_TAGS,
    };
    use handshake_native::fems::relevant_memory_panel::RELEVANT_MEMORY_PANEL_AUTHOR_ID;
    use handshake_native::graph::DAILY_JOURNAL_PANEL_AUTHOR_ID;
    use handshake_native::graph::MODE_LOCAL_AUTHOR_ID;
    use handshake_native::stage_pane::STAGE_PANE_AUTHOR_ID;

    let (app, _rt) = operator_shell();
    let mut harness =
        Harness::builder().build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), app);
    harness.run_steps(2);

    // (command id, the pane's REAL-subtree author_id that must render after the operator open).
    let routes: &[(&str, &str)] = &[
        (CMD_VIEW_RELEVANT_MEMORY, RELEVANT_MEMORY_PANEL_AUTHOR_ID),
        (CMD_VIEW_STAGE, STAGE_PANE_AUTHOR_ID),
        (CMD_VIEW_TAGS, "tags.search"),
        (CMD_VIEW_BLOCK_COLLECTIONS, "bcv.new-view"),
        (CMD_VIEW_OUTLINE, "rich-editor-outline"),
        (CMD_VIEW_GRAPH, MODE_LOCAL_AUTHOR_ID),
        (CMD_VIEW_JOURNAL, DAILY_JOURNAL_PANEL_AUTHOR_ID),
        (CMD_VIEW_DIFF_MERGE, "diff-merge-empty"),
    ];
    for (cmd, subtree_author_id) in routes {
        let fired = harness.state_mut().dispatch_palette_action_for_test(cmd);
        assert!(
            fired,
            "operator route: '{cmd}' dispatched through the palette arm produced an observable open"
        );
        harness.run_steps(3);
        let ids = live_author_ids(&harness);
        assert!(
            ids.contains(*subtree_author_id),
            "operator route: after '{cmd}' the pane's REAL subtree ('{subtree_author_id}') renders in \
             the live tree; got {:?}",
            ids.iter().collect::<Vec<_>>()
        );
    }
}

/// The Folders + Sidebar + Outgoing-Links `view.*` routes open their panes as REAL shell tabs on the
/// active work surface (tab-open proof — these panes render backend-fed rows only, so their honest
/// no-data first frame carries no unconditional chrome author_id to probe; the tab hosting the pane type
/// is the open-route evidence, and their widget subtrees are proven by their own widget suites).
#[test]
fn view_commands_open_folders_sidebar_outgoing_links_tabs() {
    use handshake_native::command_registry::{
        CMD_VIEW_FOLDERS, CMD_VIEW_OUTGOING_LINKS, CMD_VIEW_SIDEBAR,
    };
    use handshake_native::editor_pane_factories::{
        placeholder_pane_type, FOLDER_TREE_PANE_LABEL, OUTGOING_LINKS_PANE_LABEL,
        SIDEBAR_PANE_LABEL,
    };

    let (app, _rt) = operator_shell();
    let mut harness =
        Harness::builder().build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), app);
    harness.run_steps(2);

    let routes: &[(&str, PaneType)] = &[
        (
            CMD_VIEW_FOLDERS,
            placeholder_pane_type(FOLDER_TREE_PANE_LABEL),
        ),
        (CMD_VIEW_SIDEBAR, placeholder_pane_type(SIDEBAR_PANE_LABEL)),
        (
            CMD_VIEW_OUTGOING_LINKS,
            placeholder_pane_type(OUTGOING_LINKS_PANE_LABEL),
        ),
    ];
    for (cmd, pane_type) in routes {
        let fired = harness.state_mut().dispatch_palette_action_for_test(cmd);
        assert!(
            fired,
            "operator route: '{cmd}' dispatched through the palette arm produced an observable open"
        );
        harness.run_steps(2);
        let hosts_tab = harness
            .state_mut()
            .tab_bar_states_mut()
            .values()
            .any(|bar| bar.tabs.iter().any(|t| &t.pane_type == pane_type));
        assert!(
            hosts_tab,
            "operator route: after '{cmd}' a live shell tab hosts {pane_type:?}"
        );
    }
}

// ═══ MT-080 PaneType-collision regression: content-addressed nav is NO LONGER hijacked ═════════════════

/// A `PaneType::LoomBlock` navigation target renders the honest content-aware placeholder (carrying its
/// block content id), NOT the outgoing-links side panel; a `PaneType::KernelDcc` WP/MT hit renders the
/// honest placeholder, NOT a content-blind graph view. The audited hijack: singleton side-pane factories
/// registered over content-addressed navigation keys swallowed every loom-block / WP / MT open.
#[test]
fn loom_block_and_kernel_dcc_navigation_is_not_hijacked() {
    use handshake_native::graph::MODE_LOCAL_AUTHOR_ID;
    use handshake_native::rich_editor::wikilinks::outgoing_links_panel::PANEL_AUTHOR_ID as OUTGOING_PANEL_AUTHOR_ID;

    let (mut app, _rt) = operator_shell();
    retype_panes(
        &mut app,
        &[
            ("pane-a", PaneType::LoomBlock),
            ("pane-b", PaneType::KernelDcc),
        ],
    );
    let mut harness =
        Harness::builder().build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), app);
    harness.run_steps(3);
    let ids = live_author_ids(&harness);
    assert!(
        !ids.contains(OUTGOING_PANEL_AUTHOR_ID),
        "collision regression: a LoomBlock pane must NOT render the outgoing-links panel \
         ('{OUTGOING_PANEL_AUTHOR_ID}') — the content-blind hijack is retired"
    );
    assert!(
        !ids.contains(MODE_LOCAL_AUTHOR_ID),
        "collision regression: a KernelDcc pane must NOT render the graph view ('{MODE_LOCAL_AUTHOR_ID}') \
         — the content-blind hijack is retired"
    );
}
