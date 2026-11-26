//! Source scans with the window crate's scanner (comments and literals blanked, so naming a call is not making one):
//! one publisher of the ring's heartbeat slot across the shell and the host ports (MT-215 HB-7); no window library,
//! graphics binding, Win32 type or unsafe code in this crate's code, and only its three dependencies (AC214-1,
//! AC215-1); nothing here moves focus or the cursor or injects input (HBR-QUIET); every loop of a test on a private
//! desktop.

#[path = "../../window/tests/source/mod.rs"]
mod source;

use source::{below, code_only, lines_with, Source};

/// What writes the heartbeat slot or stands in for the port's beat: the ring's writer call, the native host's recorder
/// call and its per-frame bump.
const PUBLISHERS: &[&str] = &["write_heartbeat", "bump_heartbeat", "RecorderHeartbeat"];

/// Not in this crate's code at all: they come through the Handshake crates, privately.
const LIBRARIES: &[&str] = &[
    "winit",
    "ash",
    "vk",
    "windows_sys",
    "raw_window_handle",
    "accesskit",
    "egui",
    "eframe",
    "wgpu",
    "glow",
    "kumihimo",
    "HWND",
    "unsafe",
];

/// The window crate's ban (its `tests/test_focus_ban.rs`).
const NEVER: &[&str] = &[
    "SetForegroundWindow",
    "BringWindowToTop",
    "SetActiveWindow",
    "SwitchToThisWindow",
    "AllowSetForegroundWindow",
    "LockSetForegroundWindow",
    "SetFocus",
    "AttachThreadInput",
    "keybd_event",
    "mouse_event",
    "SendInput",
    "InjectTouchInput",
    "InjectSyntheticPointerInput",
    "SetCursorPos",
    "ClipCursor",
    "SetWindowsHookExW",
    "SetWindowsHookExA",
    "SetWinEventHook",
    "FlashWindow",
    "FlashWindowEx",
    "SwitchDesktop",
    "OpenInputDesktop",
    "ShowWindow",
    "SetWindowPos",
    "focus_window",
    "request_user_attention",
    "set_cursor_position",
];

fn lines(sources: &[Source], words: &[&str]) -> Vec<String> {
    let mut found = Vec::new();
    for source in sources {
        for word in words {
            for number in lines_with(&source.code, word) {
                found.push(format!("{}:{number}: {word}", source.path));
            }
        }
    }
    found
}

#[test]
fn the_heartbeat_slot_has_one_publisher_in_the_shell_and_the_ports() {
    let shell: Vec<Source> = ["src", "../window/src", "../surface/src"]
        .iter()
        .flat_map(|folder| below(folder))
        .collect();
    assert!(shell.len() >= 40, "the scan reads the shell: {}", shell.len());
    let found = lines(&shell, PUBLISHERS);
    assert!(found.is_empty(), "the shell writes the heartbeat past the port: {found:#?}");

    let port = below("../../handshake_host_ports/ui_heartbeat/src");
    assert!(port.len() >= 6, "the scan reads the port");
    let writes = lines(&port, &["write_heartbeat"]);
    assert_eq!(writes.len(), 2, "{writes:#?}");
    assert!(
        writes
            .iter()
            .all(|line| line.starts_with("../../handshake_host_ports/ui_heartbeat/src/sink.rs:")),
        "only the port's sinks write the slot: {writes:#?}"
    );

    let control = Source {
        path: "src/sample.rs".to_owned(),
        code: code_only("writer.write_heartbeat(1, 2); // write_heartbeat"),
    };
    assert_eq!(lines(&[control], PUBLISHERS).len(), 1, "the scan finds a call, not a mention");
}

#[test]
fn no_window_library_or_graphics_binding_in_the_crates_code() {
    let sources: Vec<Source> = below("src")
        .into_iter()
        .map(|source| Source {
            code: source.code.replace("cfg(windows)", "").replace("not(windows)", ""),
            path: source.path,
        })
        .collect();
    assert!(sources.len() >= 3, "the scan reads the crate");
    let found = lines(&sources, LIBRARIES);
    assert!(found.is_empty(), "{found:#?}");
    let manifest = include_str!("../Cargo.toml");
    let dependencies = manifest
        .split("[dependencies]")
        .nth(1)
        .and_then(|rest| rest.split("\n[").next())
        .expect("a dependency table");
    let named: Vec<&str> = dependencies
        .lines()
        .filter_map(|line| line.split_once(" = ").map(|(name, _)| name.trim()))
        .collect();
    assert_eq!(
        named,
        ["handshake_shell_window", "handshake_shell_surface", "ui_heartbeat"]
    );
}

#[test]
fn nothing_moves_focus_or_cursor_or_injects_input() {
    let sources: Vec<Source> = below("src").into_iter().chain(below("tests")).collect();
    let found = lines(&sources, NEVER);
    assert!(found.is_empty(), "{found:#?}");
}

#[test]
fn every_loop_of_a_test_runs_on_a_private_desktop() {
    let mut loops = 0;
    for test in below("tests") {
        let code = &test.code;
        let calls: Vec<usize> = code
            .match_indices("run_script(")
            .map(|(at, _)| at)
            .filter(|at| {
                let before = &code[..*at];
                !before
                    .chars()
                    .next_back()
                    .is_some_and(|c| c.is_alphanumeric() || c == '_')
                    && !before.trim_end().ends_with("fn")
            })
            .collect();
        let inside = code.find("on_private_desktop(");
        for at in &calls {
            assert!(
                inside.is_some_and(|start| start < *at),
                "{}: a loop before or without a private desktop",
                test.path
            );
        }
        loops += calls.len();
        let direct = lines(
            std::slice::from_ref(&test),
            &["handshake_shell_window::run", "SetThreadDesktop", "CreateDesktopW"],
        );
        assert!(direct.is_empty(), "{direct:#?}");
    }
    assert_eq!(loops, 2, "the scan sees both loops");
}
