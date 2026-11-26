//! AC214-1, HBR-HOST-001 and HBR-QUIET as source scans of this crate, with the window crate's own scanner (comments and
//! literals blanked, so naming a call is not making one): no window library, graphics binding or Win32 type in the
//! crate's code; no call that moves focus, the cursor or injects input anywhere; every loop of a test on a private
//! desktop; the example never run by a test and never without its flag; no banned crate in the lock.

#[path = "../../window/tests/source/mod.rs"]
mod source;

use source::{below, code_only, lines_with, Source};

/// Not in this crate's code at all: both come through the two Handshake crates, privately.
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
    "HINSTANCE",
    "unsafe",
];

/// The window crate's ban (its `tests/test_focus_ban.rs`), for every file here, tests and the example included.
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

/// The flag that alone lets the example open a window on the Operator's desktop.
const FLAG: &str = "--show-on-my-desktop";

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
fn no_window_library_or_graphics_binding_in_the_crates_code() {
    let sources = below("src");
    assert!(sources.len() >= 9, "the scan reads the crate");
    let stripped: Vec<Source> = sources
        .into_iter()
        .map(|source| Source {
            code: source.code.replace("cfg(windows)", "").replace("not(windows)", ""),
            path: source.path,
        })
        .collect();
    let found = lines(&stripped, LIBRARIES);
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
    assert_eq!(named, ["handshake_shell_window", "handshake-vulkan-renderer"]);
}

#[test]
fn nothing_moves_focus_or_cursor_or_injects_input() {
    let sources: Vec<Source> = below("src")
        .into_iter()
        .chain(below("tests"))
        .chain(below("examples"))
        .collect();
    let found = lines(&sources, NEVER);
    assert!(found.is_empty(), "{found:#?}");
    let control = Source {
        path: "tests/sample.rs".to_owned(),
        code: code_only("unsafe { SetWindowPos(w, 0, 0, 0, 0, 0, 0) }; // SetFocus(w)"),
    };
    assert_eq!(lines(&[control], NEVER).len(), 1, "the scan finds a call, not a mention");
}

/// Lines with a call of `run_script` outside the parentheses of an `on_private_desktop(` call.
fn loops_outside_a_private_desktop(code: &str) -> Vec<usize> {
    let spans: Vec<(usize, usize)> = code
        .match_indices("on_private_desktop(")
        .map(|(at, word)| (at + word.len() - 1, closing(code, at + word.len() - 1)))
        .collect();
    code.match_indices("run_script(")
        .map(|(at, _)| at)
        .filter(|at| {
            let before = &code[..*at];
            let joined = before
                .chars()
                .next_back()
                .is_some_and(|c| c.is_alphanumeric() || c == '_');
            !joined && !before.trim_end().ends_with("fn")
        })
        .filter(|at| !spans.iter().any(|(from, to)| (*from..*to).contains(at)))
        .map(|at| code[..at].matches('\n').count() + 1)
        .collect()
}

/// The offset after the parenthesis that closes the one at `open`.
fn closing(code: &str, open: usize) -> usize {
    let mut depth = 0;
    for (at, c) in code[open..].char_indices() {
        match c {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 {
                    return open + at + 1;
                }
            }
            _ => {}
        }
    }
    code.len()
}

#[test]
fn every_loop_of_a_test_runs_on_a_private_desktop() {
    let tests = below("tests");
    let mut loops = 0;
    for test in &tests {
        let outside = loops_outside_a_private_desktop(&test.code);
        assert!(outside.is_empty(), "{}:{outside:?}", test.path);
        loops += test.code.matches("run_script(").count();
        // The loop only through the window crate's `run_script`, which checks its desktop; desktops only through its
        // support.
        let direct = lines(
            std::slice::from_ref(test),
            &["handshake_shell_window::run", "SetThreadDesktop", "CreateDesktopW"],
        );
        assert!(direct.is_empty(), "{direct:#?}");
        assert!(!test.code.contains(FLAG) && !test.code.contains("first_frame"), "{}", test.path);
    }
    assert!(loops >= 2, "the scan sees the loops: {loops}");
    let inside = code_only("fn a() {\n    on_private_desktop(L, || {\n        run_script(c, e, w)\n    });\n}\n");
    assert!(loops_outside_a_private_desktop(&inside).is_empty());
    let outside = code_only("fn a() {\n    on_private_desktop(L, || {});\n    run_script(c, e, w);\n}\n");
    assert_eq!(loops_outside_a_private_desktop(&outside), [3], "the scan sees one outside");
}

#[test]
fn the_example_opens_nothing_without_its_flag() {
    let example = below("examples")
        .into_iter()
        .find(|source| source.path == "examples/first_frame.rs")
        .expect("the example");
    let raw = include_str!("../examples/first_frame.rs");
    assert!(raw.contains(&format!("\"{FLAG}\"")), "the flag is spelled out");
    // One loop, after the one check of the flag, which returns without it.
    let runs = lines_with(&example.code, "run");
    let check = lines_with(&example.code, "SHOW_FLAG");
    assert_eq!(runs.len(), 1, "{runs:?}");
    assert!(check.len() >= 2, "declared and checked: {check:?}");
    assert!(check[1] < runs[0], "the flag is checked before the loop runs");
    // Quiet, and never activated: no launch authority asked for or used.
    assert!(lines_with(&example.code, "take_launch_source").is_empty());
    assert!(!lines_with(&example.code, "Quiet").is_empty());
    assert!(example.code.contains("launched_by_operator: false"));
}

#[test]
fn no_banned_crate_is_locked() {
    let lock = include_str!("../../Cargo.lock");
    for name in [
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
    ] {
        assert!(!lock.contains(&format!("name = \"{name}\"")), "{name} is locked");
    }
    for name in ["handshake-vulkan-renderer", "handshake_shell_surface", "ash"] {
        assert!(lock.contains(&format!("name = \"{name}\"")), "{name} is not locked");
    }
}
