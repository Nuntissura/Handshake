//! WP-KERNEL-012 MT-098 — editor + Runtime Chat default work surface.
//!
//! Fresh launch must seed only the native code editor, rich Notes editor, and Runtime Chat pane. Feature
//! panes stay registered so the operator/agents can open them later, but they are not visible in the
//! default work surface.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use egui_kittest::kittest::{NodeT, Queryable};
use egui_kittest::Harness;
use handshake_native::app::{HandshakeApp, HealthDisplayState};
use handshake_native::backend_client::HealthInfo;
use handshake_native::code_editor::CODE_EDITOR_TEXT_AUTHOR_ID;
use handshake_native::diagnostics::DIAGNOSTICS_PANEL_AUTHOR_ID;
use handshake_native::fems::RELEVANT_MEMORY_PANEL_AUTHOR_ID;
use handshake_native::graph::canvas_board::STATUS_AUTHOR_ID as CANVAS_STATUS_AUTHOR_ID;
use handshake_native::graph::daily_journal_panel::DAILY_JOURNAL_PANEL_AUTHOR_ID;
use handshake_native::manual_pane::MANUAL_PANE_AUTHOR_ID;
use handshake_native::pane_registry::{PaneId, PaneType};
use handshake_native::rich_editor::renderer::RICH_EDITOR_ROOT_AUTHOR_ID;
use handshake_native::rich_editor::wikilinks::outgoing_links_panel::PANEL_AUTHOR_ID as OUTGOING_LINKS_PANEL_AUTHOR_ID;
use handshake_native::runtime_chat::{
    RUNTIME_CHAT_INPUT_AUTHOR_ID, RUNTIME_CHAT_PANEL_AUTHOR_ID, RUNTIME_CHAT_SEND_AUTHOR_ID,
    RUNTIME_CHAT_STATUS_AUTHOR_ID,
};
use handshake_native::stage_pane::STAGE_PANE_AUTHOR_ID;

static WGPU_SERIAL_GUARD: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn ok_app() -> HandshakeApp {
    HandshakeApp::with_health(HealthDisplayState::Ok(HealthInfo {
        status: "ok".to_owned(),
        db_status: "ok".to_owned(),
        migration_version: Some(1),
    }))
}

fn pid(s: &str) -> PaneId {
    std::sync::Arc::from(s)
}

fn registry_pane_types(app: &HandshakeApp) -> HashMap<String, PaneType> {
    let registry = app.pane_registry();
    let guard = registry.lock().expect("registry");
    guard
        .iter()
        .map(|(id, record)| (id.to_string(), record.pane_type.clone()))
        .collect()
}

fn live_author_ids(harness: &Harness<'_, HandshakeApp>) -> HashSet<String> {
    let mut ids = HashSet::new();
    for node in harness.root().children_recursive() {
        if let Some(author_id) = node.accesskit_node().author_id() {
            ids.insert(author_id.to_owned());
        }
    }
    ids
}

fn external_artifact_dir(subdir: &str) -> PathBuf {
    Path::new("../../../../Handshake_Artifacts/handshake-test").join(subdir)
}

fn assert_no_local_artifact_dir() {
    for local in ["test_output", "tests/screenshots"] {
        assert!(
            !Path::new(local).exists(),
            "artifact hygiene: screenshots must go to the external Handshake_Artifacts root, not {local}"
        );
    }
}

#[test]
fn default_seed_is_exactly_editor_panes_plus_runtime_chat() {
    let app = ok_app();
    let panes = registry_pane_types(&app);
    assert_eq!(
        panes.len(),
        3,
        "fresh default seeds exactly code, notes, and runtime chat panes: {panes:?}"
    );
    assert_eq!(panes.get("pane-a"), Some(&PaneType::CodeSymbol));
    assert_eq!(panes.get("pane-b"), Some(&PaneType::LoomWikiPage));
    assert_eq!(panes.get("pane-c"), Some(&PaneType::RuntimeChat));
    assert!(
        !panes.contains_key("pane-d"),
        "retired default pane-d is not seeded"
    );
    for stripped in [
        PaneType::Workspace,
        PaneType::InferenceLab,
        PaneType::MediaDownloader,
        PaneType::FontManager,
        PaneType::AtelierEditor,
        PaneType::KernelDcc,
        PaneType::LoomBlock,
        PaneType::LoomDailyJournal,
        PaneType::UserManual,
        PaneType::Placeholder("Stage".to_owned()),
        PaneType::Placeholder("Relevant Memory".to_owned()),
    ] {
        assert!(
            !panes.values().any(|ty| ty == &stripped),
            "stripped feature pane {stripped:?} must not be in the default seed"
        );
    }
}

#[test]
fn default_tab_bars_track_seeded_editor_and_chat_panes() {
    let app = ok_app();
    let bars = app.tab_bar_states();
    assert_eq!(bars.len(), 3, "one tab bar per default editor/chat pane");
    assert_eq!(bars.get(&pid("pane-a")).unwrap().tabs.len(), 1);
    assert_eq!(
        bars.get(&pid("pane-a")).unwrap().tabs[0].pane_type,
        PaneType::CodeSymbol
    );
    assert_eq!(bars.get(&pid("pane-b")).unwrap().tabs.len(), 1);
    assert_eq!(
        bars.get(&pid("pane-b")).unwrap().tabs[0].pane_type,
        PaneType::LoomWikiPage
    );
    assert_eq!(bars.get(&pid("pane-c")).unwrap().tabs.len(), 1);
    assert_eq!(
        bars.get(&pid("pane-c")).unwrap().tabs[0].pane_type,
        PaneType::RuntimeChat
    );
    assert!(
        !bars.contains_key(&pid("pane-d")),
        "no tabbar-pane-d in the default state"
    );
}

#[test]
fn stripped_feature_factories_remain_registered() {
    let app = ok_app();
    for pane_type in [
        PaneType::Workspace,
        PaneType::InferenceLab,
        PaneType::MediaDownloader,
        PaneType::FontManager,
        PaneType::AtelierEditor,
        PaneType::KernelDcc,
        PaneType::LoomBlock,
        PaneType::LoomDailyJournal,
        PaneType::UserManual,
        PaneType::Placeholder("Stage".to_owned()),
        PaneType::Placeholder("Relevant Memory".to_owned()),
        PaneType::CodeSymbol,
        PaneType::LoomWikiPage,
        PaneType::RuntimeChat,
    ] {
        assert!(
            app.pane_factory_registered(&pane_type),
            "factory registration preserved for {pane_type:?}"
        );
    }
}

#[test]
fn fresh_default_layout_snapshot_validates_and_round_trips() {
    let app = ok_app();
    let snapshot = app.capture_layout_snapshot();
    snapshot
        .validate()
        .expect("fresh MT-098 three-pane default snapshot validates");
    assert!(snapshot.panes.contains_key(&pid("pane-a")));
    assert!(snapshot.panes.contains_key(&pid("pane-b")));
    assert!(snapshot.panes.contains_key(&pid("pane-c")));
    assert!(!snapshot.panes.contains_key(&pid("pane-d")));

    let round_trip = handshake_native::layout_persistence::LayoutSnapshot::from_layout_state(
        snapshot.to_layout_state(),
    )
    .expect("fresh default snapshot round-trips through layout_state");
    assert_eq!(round_trip.panes.len(), 3);

    let mut missing_b = round_trip.clone();
    missing_b.panes.remove(&pid("pane-b"));
    assert!(
        matches!(
            missing_b.validate(),
            Err(handshake_native::layout_persistence::LayoutError::MissingPane { id }) if id == "pane-b"
        ),
        "missing pane-b remains a corrupt layout"
    );

    let mut stale_two_pane = round_trip.clone();
    stale_two_pane.panes.remove(&pid("pane-c"));
    assert!(
        matches!(
            stale_two_pane.validate(),
            Err(handshake_native::layout_persistence::LayoutError::MissingPane { id }) if id == "pane-c"
        ),
        "stale MT-097 two-pane layout must be rejected so Runtime Chat is restored"
    );
}

#[test]
fn diagnostics_stays_in_settings_not_default_worksurface() {
    let mut harness =
        Harness::builder().build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), ok_app());
    harness.run_steps(2);
    let default_ids = live_author_ids(&harness);
    assert!(
        !default_ids.contains(DIAGNOSTICS_PANEL_AUTHOR_ID),
        "Diagnostics panel is not a default work-surface pane"
    );

    harness.state_mut().open_settings();
    harness.run();
    harness
        .get_by_label("Search settings")
        .type_text("diagnostics");
    harness.run_steps(2);
    let settings_ids = live_author_ids(&harness);
    assert!(
        settings_ids.contains(DIAGNOSTICS_PANEL_AUTHOR_ID),
        "Settings -> Diagnostics still renders the diagnostics panel"
    );
}

#[test]
fn live_default_tree_is_editors_plus_runtime_chat_and_screenshot() {
    let _guard = WGPU_SERIAL_GUARD.lock().unwrap_or_else(|p| p.into_inner());
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1400.0, 900.0))
        .wgpu()
        .build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), ok_app());
    harness.run_steps(4);

    let ids = live_author_ids(&harness);
    for expected in [
        "pane-a",
        "pane-b",
        "pane-c",
        "tabbar-pane-a",
        "tabbar-pane-b",
        "tabbar-pane-c",
        "tab-pane-a-0",
        "tab-pane-b-0",
        "tab-pane-c-0",
        CODE_EDITOR_TEXT_AUTHOR_ID,
        RICH_EDITOR_ROOT_AUTHOR_ID,
        RUNTIME_CHAT_PANEL_AUTHOR_ID,
        RUNTIME_CHAT_STATUS_AUTHOR_ID,
        RUNTIME_CHAT_INPUT_AUTHOR_ID,
        RUNTIME_CHAT_SEND_AUTHOR_ID,
    ] {
        assert!(
            ids.contains(expected),
            "fresh default tree must contain {expected}; got {:?}",
            ids.iter()
                .filter(|id| id.contains("pane") || id.contains("editor"))
                .collect::<Vec<_>>()
        );
    }
    for absent in [
        "pane-d",
        "tabbar-pane-d",
        "divider-horizontal",
        "atelier-side-panel",
        "atelier-side-panel.refresh",
        DIAGNOSTICS_PANEL_AUTHOR_ID,
        MANUAL_PANE_AUTHOR_ID,
        STAGE_PANE_AUTHOR_ID,
        RELEVANT_MEMORY_PANEL_AUTHOR_ID,
        DAILY_JOURNAL_PANEL_AUTHOR_ID,
        OUTGOING_LINKS_PANEL_AUTHOR_ID,
        CANVAS_STATUS_AUTHOR_ID,
    ] {
        assert!(
            !ids.contains(absent),
            "fresh default tree must not contain stripped surface {absent}"
        );
    }

    let image = harness
        .render()
        .expect("wgpu render succeeds for MT-098 default screenshot");
    assert!(
        image.width() > 0 && image.height() > 0,
        "non-empty screenshot"
    );
    let ext_dir = external_artifact_dir("wp-kernel-012-mt-098");
    std::fs::create_dir_all(&ext_dir).expect("create external artifact dir");
    let png_path = ext_dir.join("MT-098-editors-runtime-chat-default.png");
    image
        .save(&png_path)
        .expect("save MT-098 editor + runtime chat default screenshot");
    println!("MT-098 screenshot: {}", png_path.display());
    assert_no_local_artifact_dir();
}
