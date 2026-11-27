//! AC215-11: the launcher compiles without a GUI toolkit, a window library, a GPU API or the `handshake-native`
//! crate, and exposes no toolkit type. Source and manifest scans, each with a positive control that proves the
//! scan can fail. The resolved closure is proven separately by `cargo tree` (the eval log), not here.

const MANIFEST: &str = include_str!("../Cargo.toml");
const WORKSPACE_LOCK: &str = include_str!("../../Cargo.lock");
const LIB_RS: &str = include_str!("../src/lib.rs");

/// The crates MT-215 forbids anywhere in a host-port closure, plus the host crate itself.
const BANNED: &[&str] = &[
    "egui",
    "eframe",
    "egui-winit",
    "egui-wgpu",
    "egui_tiles",
    "epaint",
    "ecolor",
    "emath",
    "egui_kittest",
    "kumihimo-egui",
    "cabinet-egui",
    "cabinet-peak-egui",
    "handshake_context_menu_egui",
    "wgpu",
    "wgpu-core",
    "wgpu-hal",
    "glow",
    "handshake-native",
    "handshake_native",
];

/// Extra identifier roots that mark a toolkit or window library in source.
const BANNED_SOURCE_ROOTS: &[&str] = &["egui", "eframe", "wgpu", "glow", "winit", "kumihimo", "cabinet"];

fn normalized(name: &str) -> String {
    name.trim().trim_matches('"').replace('_', "-").to_ascii_lowercase()
}

fn is_banned_name(name: &str) -> bool {
    let name = normalized(name);
    BANNED.iter().any(|banned| normalized(banned) == name)
}

/// A manifest line without its `#` comment (a `#` inside a quoted string is kept).
fn strip_toml_comment(line: &str) -> &str {
    let mut in_string: Option<char> = None;
    for (at, c) in line.char_indices() {
        match (in_string, c) {
            (None, '"' | '\'') => in_string = Some(c),
            (Some(q), c) if c == q => in_string = None,
            (None, '#') => return &line[..at],
            _ => {}
        }
    }
    line
}

/// The dependency a `[...dependencies.<name>]` header names, or `None` for any other header.
/// Matches `[dependencies.x]`, `[dev-dependencies.x]`, `[build-dependencies.x]` and
/// `[target.'cfg(windows)'.dependencies.x]`, with a quoted or bare name.
fn named_dependency_header(header: &str) -> Option<String> {
    let inner = header.trim().trim_start_matches('[').trim_end_matches(']').trim();
    let marker = "dependencies.";
    let mut search_from = 0;
    while let Some(found) = inner[search_from..].find(marker) {
        let at = search_from + found;
        let boundary = at == 0 || matches!(inner.as_bytes()[at - 1], b'.' | b'-');
        if boundary {
            let name = inner[at + marker.len()..].trim().trim_matches(['"', '\'']);
            return Some(name.to_string());
        }
        search_from = at + marker.len();
    }
    None
}

/// Whether a header opens a plain `[...dependencies]` table (target-specific and dev included).
fn is_dependency_table_header(header: &str) -> bool {
    header.trim().trim_start_matches('[').trim_end_matches(']').trim().ends_with("dependencies")
}

/// The value of a `package = "..."` rename on this line (inline table or section line), if any.
fn package_rename(line: &str) -> Option<String> {
    let at = line.find("package")?;
    let before_ok = at == 0 || !line.as_bytes()[at - 1].is_ascii_alphanumeric() && line.as_bytes()[at - 1] != b'_';
    let rest = line[at + "package".len()..].trim_start();
    let rest = rest.strip_prefix('=')?.trim_start();
    let quote = rest.chars().next().filter(|c| *c == '"' || *c == '\'')?;
    let value = rest[1..].split(quote).next()?;
    before_ok.then(|| value.to_string())
}

/// Every dependency identity a manifest declares: table keys, `[dependencies.<name>]` headers, dotted keys
/// (`egui.workspace = true`) and `package = "<name>"` renames. `#` comments are ignored.
fn dependency_keys(manifest: &str) -> Vec<String> {
    #[derive(PartialEq)]
    enum Table {
        Other,
        Plain,
        Named,
    }
    let mut table = Table::Other;
    let mut keys = Vec::new();
    for raw in manifest.lines() {
        let line = strip_toml_comment(raw).trim();
        if line.is_empty() {
            continue;
        }
        if line.starts_with('[') {
            if let Some(name) = named_dependency_header(line) {
                keys.push(name);
                table = Table::Named;
            } else if is_dependency_table_header(line) {
                table = Table::Plain;
            } else {
                table = Table::Other;
            }
            continue;
        }
        match table {
            Table::Other => {}
            Table::Plain => {
                if let Some((key, _)) = line.split_once('=') {
                    let key = key.trim().trim_matches('"');
                    keys.push(key.split('.').next().unwrap_or(key).trim_matches('"').to_string());
                }
                if let Some(renamed) = package_rename(line) {
                    keys.push(renamed);
                }
            }
            Table::Named => {
                if let Some(renamed) = package_rename(line) {
                    keys.push(renamed);
                }
            }
        }
    }
    keys
}

/// Path values of dependencies that point at the host crate directory (anything but a sibling support crate).
fn host_path_dependencies(manifest: &str) -> Vec<String> {
    manifest
        .lines()
        .map(strip_toml_comment)
        .filter(|line| line.contains("path ="))
        .filter_map(|line| line.split("path =").nth(1))
        .map(|rest| rest.trim().trim_start_matches('"').split('"').next().unwrap_or("").to_string())
        .filter(|path| {
            let path = path.replace('\\', "/");
            let ends_at_host_root = path.trim_end_matches('/').ends_with("handshake_native");
            let enters_host_src = path.contains("handshake_native/src");
            ends_at_host_root || enters_host_src
        })
        .collect()
}

fn banned_manifest_hits(manifest: &str) -> Vec<String> {
    let mut hits: Vec<String> = dependency_keys(manifest)
        .into_iter()
        .filter(|key| is_banned_name(key))
        .collect();
    hits.extend(host_path_dependencies(manifest));
    hits
}

fn banned_lock_hits(lock: &str) -> Vec<String> {
    lock.lines()
        .filter_map(|line| line.strip_prefix("name = "))
        .filter(|name| is_banned_name(name))
        .map(str::to_string)
        .collect()
}

/// Source with `/* */` block comments and `//` comments removed. A `//` inside a string literal is cut too, which
/// can only hide text, and no banned token sits behind one in this file (the positive controls below prove the
/// scans work).
fn code_only(source: &str) -> String {
    let mut without_blocks = String::with_capacity(source.len());
    let mut rest = source;
    while let Some(open) = rest.find("/*") {
        without_blocks.push_str(&rest[..open]);
        match rest[open + 2..].find("*/") {
            Some(close) => {
                // Keep the line count stable.
                without_blocks.push_str(&"\n".repeat(rest[open..open + 2 + close].matches('\n').count()));
                rest = &rest[open + 2 + close + 2..];
            }
            None => {
                rest = "";
            }
        }
    }
    without_blocks.push_str(rest);
    without_blocks
        .lines()
        .map(|line| line.split("//").next().unwrap_or(""))
        .collect::<Vec<_>>()
        .join("\n")
}

fn banned_source_tokens(source: &str) -> Vec<String> {
    let code = code_only(source);
    let mut hits = Vec::new();
    for token in code.split(|c: char| !(c.is_ascii_alphanumeric() || c == '_' || c == '-')) {
        let lower = token.to_ascii_lowercase();
        let root = lower.split(['_', '-']).next().unwrap_or("");
        if BANNED_SOURCE_ROOTS.contains(&root) || is_banned_name(&lower) {
            hits.push(token.to_string());
        }
    }
    hits
}

#[test]
fn manifest_has_no_toolkit_window_gpu_or_host_crate() {
    let hits = banned_manifest_hits(MANIFEST);
    assert!(hits.is_empty(), "banned manifest dependency: {hits:?}");
    let keys = dependency_keys(MANIFEST);
    assert!(
        keys.iter().any(|key| key == "handshake-diag-ring")
            && keys.iter().any(|key| key == "palmistry-control-protocol"),
        "the scan must see the real dependency table, got {keys:?}"
    );
}

#[test]
fn workspace_lock_has_no_banned_package() {
    let hits = banned_lock_hits(WORKSPACE_LOCK);
    assert!(hits.is_empty(), "banned package in the resolved lock: {hits:?}");
    assert!(WORKSPACE_LOCK.contains("name = \"palmistry_launch\""), "the lock must list this crate");
}

#[test]
fn source_names_no_toolkit_window_gpu_or_host_type() {
    let hits = banned_source_tokens(LIB_RS);
    assert!(hits.is_empty(), "banned token in launcher source: {hits:?}");
    assert!(code_only(LIB_RS).contains("pub trait PalmistryLaunchSession"));
}

#[test]
fn public_api_takes_the_port_not_a_host_session_type() {
    let code = code_only(LIB_RS);
    for name in [
        "launch_palmistry",
        "launch_palmistry_at",
        "launch_palmistry_signed_at",
        "launch_palmistry_or_degrade",
    ] {
        let at = code
            .find(&format!("pub fn {name}<S: PalmistryLaunchSession + ?Sized>("))
            .unwrap_or_else(|| panic!("{name} must be generic over the port"));
        let signature = &code[at..at + code[at..].find("{\n").unwrap_or(400).min(900)];
        assert!(signature.contains("session: &S"), "{name} must take the port: {signature}");
    }
    assert!(!code.contains("DiagSession"), "the host session type must not be named here");
}

// ---- positive controls: each scan above must be able to fail ----

#[test]
fn control_manifest_scan_flags_each_banned_class() {
    for banned in BANNED {
        let manifest = format!("[dependencies]\n{banned} = \"1\"\n");
        assert_eq!(banned_manifest_hits(&manifest).len(), 1, "{banned} must be flagged");
    }
    let target_table = "[target.'cfg(windows)'.dependencies]\neframe = { version = \"=0.33.3\" }\n";
    assert_eq!(banned_manifest_hits(target_table), vec!["eframe".to_string()]);
    let host = "[dependencies]\nhost = { path = \"../../handshake_native\" }\n";
    assert_eq!(banned_manifest_hits(host).len(), 1, "a path into the host crate must be flagged");
    let host_src = "[dependencies]\nx = { path = \"../../handshake_native/src/mcp\" }\n";
    assert_eq!(banned_manifest_hits(host_src).len(), 1);
    let allowed = "[dependencies]\nhandshake-diag-ring = { path = \"../../handshake_native/diag_ring\" }\n";
    assert!(banned_manifest_hits(allowed).is_empty(), "the shared ring crate is allowed");
}

#[test]
fn control_lock_scan_flags_a_banned_package() {
    let lock = "[[package]]\nname = \"serde\"\nversion = \"1.0.229\"\n\n[[package]]\nname = \"wgpu-hal\"\nversion = \"27.0.0\"\n";
    assert_eq!(banned_lock_hits(lock), vec!["\"wgpu-hal\"".to_string()]);
    assert!(banned_lock_hits("[[package]]\nname = \"serde\"\nversion = \"1.0.229\"\n").is_empty());
}

#[test]
fn control_source_scan_flags_toolkit_types_and_ignores_comments() {
    let leaks = [
        "pub fn paint(ui: &mut egui::Ui) {}",
        "use eframe::App;",
        "pub struct S(wgpu::Device);",
        "use glow::HasContext;",
        "fn f(w: winit::window::Window) {}",
        "use handshake_native::diagnostics::DiagSession;",
        "use kumihimo_egui::Thing;",
    ];
    for leak in leaks {
        assert!(!banned_source_tokens(leak).is_empty(), "must flag: {leak}");
    }
    assert!(banned_source_tokens("// the egui host calls this\npub fn ok() {}").is_empty());
    assert!(banned_source_tokens("pub fn ok(a: &str) -> std::io::Result<()> { Ok(()) }").is_empty());
}

// ---- F9: table headers, package renames, comments ----

#[test]
fn control_manifest_scan_flags_table_headers_renames_and_dotted_keys() {
    let header = "[dependencies.egui]\nversion = \"0.33\"\n";
    assert_eq!(banned_manifest_hits(header), vec!["egui".to_string()], "a [dependencies.x] header must be flagged");
    let target_header = "[target.'cfg(windows)'.dependencies.wgpu]\nversion = \"27\"\n";
    assert_eq!(banned_manifest_hits(target_header), vec!["wgpu".to_string()]);
    let dev_header = "[dev-dependencies.\"egui_kittest\"]\nversion = \"0.33\"\n";
    assert_eq!(banned_manifest_hits(dev_header), vec!["egui_kittest".to_string()]);
    let inline_rename = "[dependencies]\nui = { package = \"eframe\", version = \"0.33\" }\n";
    assert!(
        banned_manifest_hits(inline_rename).contains(&"eframe".to_string()),
        "a package rename to a banned crate must be flagged"
    );
    let section_rename = "[dependencies.ui]\npackage = \"epaint\"\nversion = \"0.33\"\n";
    assert!(banned_manifest_hits(section_rename).contains(&"epaint".to_string()));
    let dotted = "[dependencies]\negui.workspace = true\n";
    assert_eq!(banned_manifest_hits(dotted), vec!["egui".to_string()], "a dotted key must be flagged");
    let trailing_comment = "[dependencies]\nglow = \"0.16\" # not harmless\n";
    assert_eq!(banned_manifest_hits(trailing_comment), vec!["glow".to_string()]);
}

#[test]
fn control_manifest_scan_ignores_comments_and_allowed_renames() {
    assert!(banned_manifest_hits("[dependencies]\n# egui = \"0.33\"\n").is_empty());
    assert!(banned_manifest_hits("[dependencies]\n    # [dependencies.egui]\n").is_empty());
    assert!(banned_manifest_hits("[dependencies.serde]\nversion = \"1\"\n").is_empty());
    assert!(banned_manifest_hits("[dependencies]\nlog = { package = \"tracing\", version = \"0.1\" }\n").is_empty());
    assert!(banned_manifest_hits("[features]\negui = []\n").is_empty(), "a feature name is not a dependency");
    let keys = dependency_keys("[dependencies.serde]\nversion = \"1\"\n[dependencies]\nrand = \"0.9\"\n");
    assert_eq!(keys, vec!["serde".to_string(), "rand".to_string()]);
}

#[test]
fn control_source_scan_handles_block_comments() {
    assert!(banned_source_tokens("/* the egui host calls this */\npub fn ok() {}").is_empty());
    assert!(banned_source_tokens("/*\n multi-line egui\n note\n*/\npub fn ok() {}").is_empty());
    assert!(
        !banned_source_tokens("/* note */ fn f(ui: &mut egui::Ui) {}").is_empty(),
        "code after a block comment on the same line must still be scanned"
    );
    assert!(
        !banned_source_tokens("/* a */\nuse eframe::App;\n/* b */").is_empty(),
        "code between two block comments must still be scanned"
    );
}

// ---- F7: HBR-QUIET focus and foreground API ban (the moved launcher is no longer covered by the egui host audit) ----

/// Win32 and helper names that steal focus, foreground a window or inject input. Superset of the host's
/// `clippy.toml` `disallowed-methods` list and its `test_focus_audit_quiet` audit.
const BANNED_FOCUS_APIS: &[&str] = &[
    "SetForegroundWindow",
    "BringWindowToTop",
    "SetActiveWindow",
    "SwitchToThisWindow",
    "AllowSetForegroundWindow",
    "LockSetForegroundWindow",
    "SetFocus",
    "SetCapture",
    "FlashWindow",
    "FlashWindowEx",
    "ShowWindow",
    "keybd_event",
    "mouse_event",
    "SendInput",
    "AttachThreadInput",
    "AppActivate",
    "request_foreground_exception",
];

fn focus_api_hits(source: &str) -> Vec<String> {
    let code = code_only(source);
    code.split(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
        .filter(|token| BANNED_FOCUS_APIS.contains(token))
        .map(str::to_string)
        .collect()
}

#[test]
fn launcher_source_calls_no_focus_or_foreground_api() {
    let hits = focus_api_hits(LIB_RS);
    assert!(hits.is_empty(), "HBR-QUIET: focus or foreground API in the launcher: {hits:?}");
    // The scan reads the real file, and the launcher keeps its quiet spawn (no console, no inherited stdio).
    let code = code_only(LIB_RS);
    assert!(code.contains("creation_flags(CREATE_NO_WINDOW)") && code.contains("Stdio::null()"));
}

#[test]
fn control_focus_scan_flags_each_banned_api_and_ignores_comments() {
    for api in BANNED_FOCUS_APIS {
        assert_eq!(focus_api_hits(&format!("unsafe {{ {api}(hwnd) }};")), vec![api.to_string()], "{api} must be flagged");
        assert_eq!(
            focus_api_hits(&format!("windows_sys::Win32::UI::WindowsAndMessaging::{api}(0)")),
            vec![api.to_string()],
            "a qualified {api} call must be flagged"
        );
    }
    assert!(focus_api_hits("// SetForegroundWindow is banned\n/* SendInput too */\nfn ok() {}").is_empty());
    assert!(focus_api_hits("fn set_focus_later() { show_window_count(); }").is_empty(), "whole-token match only");
}
