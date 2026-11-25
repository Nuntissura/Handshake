//! HBR-HOST-001 and AC214-1 as a source scan: the window library and Win32 appear in `src/platform` only, nothing of
//! `src/platform` is public, and no banned crate is a dependency.

mod source;

use source::{below, lines_with};

const LIBRARIES: &[&str] = &[
    "winit",
    "windows_sys",
    "raw_window_handle",
    "accesskit",
    "accesskit_windows",
    "ash",
    "egui",
    "eframe",
    "wgpu",
    "HWND",
    "HINSTANCE",
    "HMONITOR",
];

#[test]
fn library_types_appear_below_platform_only() {
    let sources = below("src");
    for source in sources
        .iter()
        .filter(|source| !source.path.starts_with("src/platform/"))
    {
        // `#[cfg(windows)]` is no library.
        let code = source
            .code
            .replace("cfg(windows)", "")
            .replace("not(windows)", "");
        for word in LIBRARIES {
            let lines = lines_with(&code, word);
            assert!(lines.is_empty(), "{}:{lines:?}: {word}", source.path);
        }
    }
    let inside = |word| {
        sources
            .iter()
            .filter(|source| source.path.starts_with("src/platform/"))
            .any(|source| !lines_with(&source.code, word).is_empty())
    };
    assert!(
        inside("winit") && inside("HWND"),
        "the scan reads the files that use them"
    );
}

#[test]
fn nothing_below_platform_is_public() {
    for source in below("src")
        .iter()
        .filter(|source| source.path.starts_with("src/platform/"))
    {
        for (number, line) in source.code.lines().enumerate() {
            let line = line.trim_start();
            assert!(
                !line.starts_with("pub "),
                "{}:{}: {line}",
                source.path,
                number + 1
            );
        }
    }
    let sources = below("src");
    let root = sources
        .iter()
        .find(|source| source.path == "src/lib.rs")
        .unwrap();
    let exports: Vec<&str> = root
        .code
        .lines()
        .filter(|line| line.contains("platform"))
        .map(str::trim)
        .collect();
    assert_eq!(exports, ["mod platform;"]);
}

#[test]
fn no_banned_crate_is_a_dependency() {
    let manifest = include_str!("../Cargo.toml");
    let lock = include_str!("../../Cargo.lock");
    let banned = [
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
    ];
    for name in banned {
        assert!(
            !lock.contains(&format!("name = \"{name}\"")),
            "{name} is locked"
        );
        assert!(
            !manifest.contains(&format!("\n{name} =")),
            "{name} is named"
        );
    }
    assert!(
        lock.contains("name = \"winit\"\nversion = \"0.30.13\"")
            || lock.contains("name = \"winit\"\r\nversion = \"0.30.13\"")
    );
}
