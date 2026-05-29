use std::{fs, path::Path};

use regex::Regex;
use serde_json::Value;

fn repo_root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .and_then(Path::parent)
        .expect("handshake_core lives under src/backend/handshake_core")
}

fn read_repo_file(relative: &str) -> String {
    fs::read_to_string(repo_root().join(relative))
        .unwrap_or_else(|error| panic!("failed to read {relative}: {error}"))
}

#[test]
fn tauri_main_window_defaults_are_quiet() {
    let raw = read_repo_file("app/src-tauri/tauri.conf.json");
    let config: Value = serde_json::from_str(&raw).expect("tauri.conf.json parses");
    let windows = config["app"]["windows"]
        .as_array()
        .expect("app.windows array exists");
    let main = windows
        .iter()
        .find(|window| window["label"].as_str().unwrap_or("main") == "main")
        .expect("main window config exists");

    assert_eq!(main["visible"], false);
    assert_eq!(main["focus"], false);
    assert_eq!(main["focusable"], false);
    assert_eq!(main["skipTaskbar"], true);
    assert_eq!(main["alwaysOnBottom"], true);
    assert_eq!(main["decorations"], false);
}

#[test]
fn quiet_window_builder_forces_all_hbr_quiet_flags_and_hides_raw_builder() {
    let quiet_window = read_repo_file("app/src-tauri/src/quiet_window.rs");

    for required in [
        "WebviewWindowBuilder::new",
        ".visible(false)",
        ".focused(false)",
        ".focusable(false)",
        ".skip_taskbar(true)",
        ".always_on_bottom(true)",
        ".decorations(false)",
        "pub struct QuietWindowBuilder",
        "builder:",
    ] {
        assert!(
            quiet_window.contains(required),
            "quiet_window.rs missing required fragment: {required}"
        );
    }

    assert!(
        !quiet_window.contains("pub fn visible")
            && !quiet_window.contains("pub fn focused")
            && !quiet_window.contains("pub fn focusable")
            && !quiet_window.contains("pub fn skip_taskbar")
            && !quiet_window.contains("pub fn always_on_bottom")
            && !quiet_window.contains("pub fn decorations"),
        "QuietWindowBuilder must not expose mutators that invert quiet flags"
    );
}

#[test]
fn clippy_policy_bans_raw_webview_builder_outside_quiet_wrapper() {
    let clippy = read_repo_file("clippy.toml");
    assert!(clippy.contains("tauri::webview::WebviewWindowBuilder::new"));
    assert!(clippy.contains("tauri::window::WindowBuilder::new"));
    assert!(clippy.contains("Use QuietWindowBuilder"));

    let app_lib = read_repo_file("app/src-tauri/src/lib.rs");
    assert!(app_lib.contains("#![deny(clippy::disallowed_methods)]"));

    let quiet_window = read_repo_file("app/src-tauri/src/quiet_window.rs");
    assert!(quiet_window.contains("#[allow(clippy::disallowed_methods)]"));
}

#[test]
fn raw_tauri_window_creation_and_focus_are_banned_outside_escape_hatches() {
    let app_src = repo_root().join("app/src-tauri/src");
    let raw_webview_builder = Regex::new(r"\bWebviewWindowBuilder::new").expect("regex");
    let raw_window_builder = Regex::new(r"\bWindowBuilder::new").expect("regex");
    let mut violations = Vec::new();
    for entry in fs::read_dir(&app_src).expect("app/src-tauri/src exists") {
        let entry = entry.expect("dir entry");
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("rs") {
            continue;
        }
        let relative = path
            .strip_prefix(repo_root())
            .expect("path under repo root")
            .to_string_lossy()
            .replace('\\', "/");
        if relative == "app/src-tauri/src/quiet_window.rs" {
            continue;
        }
        let text = fs::read_to_string(&path).expect("read Rust source");
        if raw_webview_builder.is_match(&text) {
            violations.push(format!("{relative}: raw WebviewWindowBuilder::new"));
        }
        if raw_window_builder.is_match(&text) {
            violations.push(format!("{relative}: raw WindowBuilder::new"));
        }
        if text.contains(".set_focus(") {
            violations.push(format!("{relative}: direct set_focus"));
        }
    }

    assert!(
        violations.is_empty(),
        "HBR-QUIET-001 source violations:\n{}",
        violations.join("\n")
    );
}

#[test]
fn operator_foreground_escape_hatch_is_documented_but_inert() {
    let module = read_repo_file("src/backend/handshake_core/src/operator_foreground/mod.rs");
    assert!(module.contains("HBR-QUIET-001"));
    assert!(module.contains("operator foreground exception"));
    assert!(module.contains("MT-019"));
    assert!(
        !module.contains("WebviewWindowBuilder::new") && !module.contains("set_focus"),
        "operator_foreground placeholder must not create or focus windows in MT-014"
    );
}
