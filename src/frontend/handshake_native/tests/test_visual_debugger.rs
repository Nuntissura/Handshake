//! WP-KERNEL-012 MT-102 — Visual Debugger worksurface/window structure inspector.
//!
//! These tests drive the intended runtime seam before implementation: a Settings-hosted diagnostic
//! action captures the live pane registry, AccessKit widget tree, layout snapshot, and an honest
//! headless screenshot marker into a JSON artifact outside the product tree.

use std::fs;
use std::path::{Path, PathBuf};

use egui::accesskit;
use handshake_native::accessibility::collect_ui_tree_snapshot;
use handshake_native::app::{HandshakeApp, HealthDisplayState};
use handshake_native::backend_client::HealthInfo;
use handshake_native::settings_diagnostics_section::{
    DiagnosticsSettingsView, DIAGNOSTICS_SEARCH_KEYWORDS,
};
use handshake_native::settings_dialog::SettingsOutcome;
use handshake_native::visual_debugger::{
    default_artifact_root, PaneLocation, ScreenshotEvidence, WorksurfaceInspector,
    WorksurfaceSnapshot, WORKSURFACE_DIAG_EVENT_CODE, WORKSURFACE_INSPECTOR_DUMP_BUTTON_AUTHOR_ID,
    WORKSURFACE_SNAPSHOT_SCHEMA_ID,
};

fn ok_app() -> HandshakeApp {
    HandshakeApp::with_health(HealthDisplayState::Ok(HealthInfo {
        status: "ok".to_string(),
        db_status: "ok".to_string(),
        migration_version: Some(1),
    }))
}

fn artifact_dir(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("..")
        .join("..")
        .join("Handshake_Artifacts")
        .join("handshake-test")
        .join("wp-kernel-012-mt-102")
        .join(name)
}

fn assert_under(path: &Path, root: &Path) {
    let path = path
        .canonicalize()
        .unwrap_or_else(|e| panic!("canonicalize {}: {e}", path.display()));
    let root = root
        .canonicalize()
        .unwrap_or_else(|e| panic!("canonicalize {}: {e}", root.display()));
    assert!(
        path.starts_with(&root),
        "{} must stay under external artifact root {}",
        path.display(),
        root.display()
    );
}

#[test]
fn visual_debugger_live_dump_contains_panes_widgets_layout() {
    let mut app = ok_app();
    let root = artifact_dir("live-dump");
    fs::create_dir_all(&root).expect("create MT-102 artifact dir");

    let receipt = app
        .capture_worksurface_snapshot_to(&root)
        .expect("worksurface inspector writes JSON");
    assert_under(&receipt.path, &root);
    assert!(receipt.bytes > 0, "receipt records bytes written");

    let json = fs::read_to_string(&receipt.path).expect("read worksurface snapshot JSON");
    let snapshot: WorksurfaceSnapshot =
        serde_json::from_str(&json).expect("snapshot JSON deserializes");

    assert_eq!(snapshot.schema_id, WORKSURFACE_SNAPSHOT_SCHEMA_ID);
    assert_eq!(
        snapshot.pane_tree.len(),
        3,
        "fresh shell has the three native default panes"
    );
    assert!(!snapshot.capture_id.is_empty(), "capture id is recorded");
    assert!(snapshot.layout_tree.split_weights.vertical > 0.0);
    assert!(snapshot.layout_tree.split_weights.horizontal > 0.0);
    assert_eq!(snapshot.layout_tree.panes.len(), snapshot.pane_tree.len());
    assert_eq!(
        snapshot.layout_tree.tab_bars.len(),
        snapshot.pane_tree.len(),
        "each mounted pane has a tab bar in the layout tree"
    );

    let mut by_id = std::collections::BTreeMap::new();
    for pane in &snapshot.pane_tree {
        by_id.insert(pane.pane_id.as_str(), pane);
    }
    assert_eq!(by_id["pane-a"].pane_type, "CodeSymbol");
    assert_eq!(by_id["pane-b"].pane_type, "LoomWikiPage");
    assert_eq!(by_id["pane-c"].pane_type, "RuntimeChat");
    for pane in snapshot.pane_tree.iter() {
        assert!(
            pane.accesskit_node_id.is_some(),
            "{} must expose its registry AccessKit node id",
            pane.pane_id
        );
        assert_eq!(
            pane.location,
            PaneLocation::MainSplit,
            "{} is mounted in the main split by default",
            pane.pane_id
        );
        assert_ne!(
            pane.pane_type, "VisualDebugger",
            "the inspector must not mount itself as a worksurface pane"
        );
    }

    assert!(
        snapshot.widget_inventory.widget_count > 0,
        "widget inventory reflects the live AccessKit tree"
    );
    for author_id in ["pane-a", "tabbar-pane-a", "bottom-rail.input"] {
        assert!(
            snapshot
                .widget_inventory
                .nodes
                .iter()
                .any(|node| node.author_id.as_deref() == Some(author_id)),
            "widget inventory includes stable author_id {author_id}"
        );
    }

    match snapshot.screenshot {
        ScreenshotEvidence::Deferred { marker, reason } => {
            assert_eq!(marker, "screenshot_deferred_headless_gpu");
            assert!(
                reason.contains("headless"),
                "deferred screenshot marker must explain the headless limit"
            );
        }
        ScreenshotEvidence::Captured { path, .. } => {
            assert!(path.exists(), "captured screenshot path exists");
        }
    }

    assert_eq!(
        snapshot.internal_diagnostics.event_code,
        WORKSURFACE_DIAG_EVENT_CODE
    );
    assert_eq!(snapshot.internal_diagnostics.counter_a_name, "pane_count");
    assert_eq!(snapshot.internal_diagnostics.counter_a_value, 3);
    assert_eq!(snapshot.internal_diagnostics.counter_b_name, "widget_count");
    assert!(snapshot.internal_diagnostics.counter_b_value > 0);

    let value: serde_json::Value =
        serde_json::from_str(&json).expect("snapshot parses as generic JSON");
    assert!(
        value.pointer("/layout_tree/snapshot").is_none(),
        "inspector JSON must not embed the internal LayoutSnapshot serde shape"
    );
    assert!(
        value.pointer("/widget_inventory/tree/root").is_none(),
        "inspector JSON must not embed the internal UiTreeSnapshot serde shape"
    );
    for pointer in [
        "/pane_tree/0/pane_type",
        "/layout_tree/split_weights/vertical",
        "/layout_tree/tab_bars/0/tabs/0/label",
        "/widget_inventory/tree/id",
        "/internal_diagnostics/counter_a_name",
    ] {
        assert!(
            value.pointer(pointer).is_some(),
            "stable inspector JSON field {pointer} is present"
        );
    }
}

#[test]
fn visual_debugger_repeated_dumps_do_not_overwrite_and_reject_repo_local_roots() {
    let mut app = ok_app();
    let root = artifact_dir("repeat-dump");
    fs::create_dir_all(&root).expect("create MT-102 repeat artifact dir");

    let first = app
        .capture_worksurface_snapshot_to(&root)
        .expect("first worksurface inspector write");
    let second = app
        .capture_worksurface_snapshot_to(&root)
        .expect("second worksurface inspector write");

    assert_ne!(
        first.path, second.path,
        "back-to-back dumps must not overwrite the previous evidence file"
    );
    assert!(first.path.exists(), "first dump still exists");
    assert!(second.path.exists(), "second dump exists");

    let first_snapshot: WorksurfaceSnapshot =
        serde_json::from_str(&fs::read_to_string(&first.path).expect("read first snapshot"))
            .expect("first snapshot parses");
    let collision = WorksurfaceInspector::write_json(&first_snapshot, &root)
        .expect_err("duplicate capture IDs must not overwrite existing evidence");
    assert_eq!(collision.kind(), std::io::ErrorKind::AlreadyExists);

    let repo_local = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("mt102-local-artifacts")
        .join("nested");
    let err = app
        .capture_worksurface_snapshot_to(&repo_local)
        .expect_err("repo-local artifact roots are rejected before writes");
    assert_eq!(err.kind(), std::io::ErrorKind::InvalidInput);
    assert!(
        !repo_local.exists(),
        "repo-local artifact directory must not be created"
    );

    let cwd = std::env::current_dir().expect("test has a cwd");
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .and_then(Path::parent)
        .expect("manifest dir is under product repo")
        .to_path_buf();
    if cwd.starts_with(&repo_root) {
        let relative_repo_local = PathBuf::from("mt102-relative-local-artifacts");
        let err = app
            .capture_worksurface_snapshot_to(&relative_repo_local)
            .expect_err("relative repo-local artifact roots are rejected before writes");
        assert_eq!(err.kind(), std::io::ErrorKind::InvalidInput);
        assert!(
            !cwd.join(&relative_repo_local).exists(),
            "relative repo-local artifact directory must not be created"
        );
    }
}

#[test]
fn visual_debugger_settings_outcome_writes_external_artifact_and_status() {
    let mut app = ok_app();
    assert!(
        app.apply_settings_outcome_for_test(SettingsOutcome::WorksurfaceInspectorDumpRequested),
        "Settings outcome is handled by the shell"
    );
    let status = app
        .worksurface_inspector_last_dump()
        .expect("Settings outcome surfaces a visible last-dump status");
    assert!(
        status.contains("Wrote worksurface snapshot:"),
        "status reports a successful dump, got {status}"
    );
    let file_name = status
        .strip_prefix("Wrote worksurface snapshot: ")
        .and_then(|tail| tail.split(" (").next())
        .expect("status contains snapshot file name");
    assert!(
        file_name.starts_with("worksurface-snapshot-") && file_name.ends_with(".json"),
        "status stays compact and names the snapshot file, got {status}"
    );
    assert!(
        !status.contains(env!("CARGO_MANIFEST_DIR")),
        "status should not expose a noisy repo-local absolute path"
    );
    let path = default_artifact_root().join(file_name);
    assert!(path.exists(), "Settings outcome wrote {file_name}");
    assert!(
        !path.starts_with(PathBuf::from(env!("CARGO_MANIFEST_DIR"))),
        "Settings-triggered artifacts must not land under the product crate"
    );
}

#[test]
fn visual_debugger_hosted_in_settings_not_default_panes() {
    for term in [
        "visual",
        "debugger",
        "inspector",
        "worksurface",
        "widget",
        "layout",
    ] {
        assert!(
            DIAGNOSTICS_SEARCH_KEYWORDS.contains(&term),
            "Settings search must find the Visual Debugger by '{term}'"
        );
    }

    let app = ok_app();
    let diagnostics = app.diagnostics_view();
    let palette = app.current_theme().palette();

    let ctx = egui::Context::default();
    ctx.enable_accesskit();
    let output = ctx.run(egui::RawInput::default(), |ctx| {
        egui::CentralPanel::default().show(ctx, |ui| {
            let view = DiagnosticsSettingsView {
                diagnostics: &diagnostics,
                palette: &palette,
                worksurface_inspector_last_dump: None,
            };
            let outcome = handshake_native::settings_diagnostics_section::render(ui, &view);
            assert_eq!(
                outcome,
                handshake_native::settings_diagnostics_section::DiagnosticsSectionOutcome::None
            );
        });
    });
    let update: accesskit::TreeUpdate = output
        .platform_output
        .accesskit_update
        .expect("AccessKit update produced for Diagnostics section");
    let tree = collect_ui_tree_snapshot(&update);
    let dump_button = tree
        .find_by_author_id(WORKSURFACE_INSPECTOR_DUMP_BUTTON_AUTHOR_ID)
        .expect("Settings -> Diagnostics exposes the worksurface inspector dump button");
    assert_eq!(dump_button.role, "Button");
    assert!(
        dump_button.actions.iter().any(|a| a == "Click"),
        "dump button must be click-steerable by AccessKit"
    );

    let layout = app.capture_layout_snapshot();
    assert!(
        layout
            .panes
            .values()
            .all(|pane| format!("{:?}", pane.pane_type) != "VisualDebugger"),
        "Visual Debugger remains a hidden Settings diagnostic, not a default worksurface pane"
    );
}
