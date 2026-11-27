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

/// Dependency keys of every `[dependencies]`-style table of a manifest (target-specific and dev included).
fn dependency_keys(manifest: &str) -> Vec<String> {
    let mut in_dependencies = false;
    let mut keys = Vec::new();
    for line in manifest.lines() {
        let line = line.trim();
        if line.starts_with('[') {
            in_dependencies = line.trim_matches(['[', ']']).ends_with("dependencies");
            continue;
        }
        if !in_dependencies || line.starts_with('#') || line.is_empty() {
            continue;
        }
        if let Some((key, _)) = line.split_once('=') {
            if !key.contains(' ') || key.trim_end() == key.trim() {
                keys.push(key.trim().to_string());
            }
        }
    }
    keys
}

/// Path values of dependencies that point at the host crate directory (anything but a sibling support crate).
fn host_path_dependencies(manifest: &str) -> Vec<String> {
    manifest
        .lines()
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

/// Source with `//` comments removed. A `//` inside a string literal is cut too, which can only hide text, and no
/// banned token sits behind one in this file (the positive control below proves the scan itself works).
fn code_only(source: &str) -> String {
    source
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
