//! HBR-QUIET as a source scan: no call in this crate, its tests included, moves the Operator's focus or cursor or injects
//! input, and only one file can show or order a window.

mod source;

use source::{below, code_only, lines_with, Source};

/// Never, anywhere: the product's ban (handshake_native quiet_mode::focus_guard and MT-215 AC215-8) and the calls of
/// the window library that do the same inside.
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
    "FlashWindow",
    "FlashWindowEx",
    "SwitchDesktop",
    // winit
    "focus_window",
    "set_minimized",
    "set_maximized",
    "set_fullscreen",
    "with_fullscreen",
    "with_maximized",
    "set_visible",
    "set_window_level",
    "with_window_level",
    "set_cursor_position",
    "set_cursor_grab",
    "request_user_attention",
    "drag_window",
    "drag_resize_window",
    "show_window_menu",
    "request_inner_size",
    "set_outer_position",
];

/// Only where each call is made from one decision.
const ONLY_IN_PLACE: &[&str] = &[
    "ShowWindow",
    "ShowWindowAsync",
    "SetWindowPos",
    "SetWindowPlacement",
    "MoveWindow",
    "BeginDeferWindowPos",
    "AnimateWindow",
];

/// Show commands that activate, or that let the process start decide.
const ACTIVATING: &[&str] = &[
    "SW_SHOWNORMAL",
    "SW_SHOWDEFAULT",
    "SW_SHOWMAXIMIZED",
    "SW_MAXIMIZE",
    "SW_FORCEMINIMIZE",
];

/// The one call of the window library's that has a name of ours, on the writer it hands out for a scale change.
const LIBRARY_WRITER: (&str, &str) = (
    "src/platform/translate.rs",
    "inner_size_writer.request_inner_size",
);

fn found(
    sources: &[Source],
    words: &[&str],
    allowed: impl Fn(&Source, &str, &str) -> bool,
) -> Vec<String> {
    let mut found = Vec::new();
    for source in sources {
        let lines: Vec<&str> = source.code.lines().collect();
        for word in words {
            for number in lines_with(&source.code, word) {
                if !allowed(source, word, lines[number - 1]) {
                    found.push(format!("{}:{number}: {word}", source.path));
                }
            }
        }
    }
    found
}

fn every_source() -> Vec<Source> {
    let sources: Vec<Source> = below("src").into_iter().chain(below("tests")).collect();
    assert!(sources.len() > 20, "the scan looks at the crate");
    sources
}

#[test]
fn nothing_moves_focus_or_cursor_or_injects_input() {
    let found = found(&every_source(), NEVER, |source, _, line| {
        source.path == LIBRARY_WRITER.0 && line.contains(LIBRARY_WRITER.1)
    });
    assert!(found.is_empty(), "{found:#?}");
}

#[test]
fn only_one_file_shows_or_orders_windows() {
    let sources = below("src");
    let elsewhere = found(&sources, ONLY_IN_PLACE, |source, _, _| {
        source.path == "src/platform/place.rs"
    });
    assert!(elsewhere.is_empty(), "{elsewhere:#?}");
    let activating = found(&sources, ACTIVATING, |_, _, _| false);
    assert!(activating.is_empty(), "{activating:#?}");
    let place = sources
        .iter()
        .find(|source| source.path == "src/platform/place.rs")
        .unwrap();
    assert!(
        !lines_with(&place.code, "SetWindowPos").is_empty(),
        "the file is the one that does"
    );
}

#[test]
fn a_window_is_created_hidden_and_inactive() {
    let sources = below("src");
    let window = sources
        .iter()
        .find(|source| source.path == "src/platform/window.rs")
        .unwrap();
    assert_eq!(lines_with(&window.code, "create_window").len(), 1);
    for hidden in ["with_visible(false)", "with_active(false)"] {
        assert_eq!(window.code.matches(hidden).count(), 1, "{hidden}");
    }
    let shown = found(&sources, &["WS_VISIBLE"], |_, _, _| false);
    assert!(shown.is_empty(), "{shown:#?}");
}

#[test]
fn every_test_that_runs_a_loop_runs_it_on_a_private_desktop() {
    for test in below("tests") {
        let runs = !lines_with(&test.code, "run_script").is_empty();
        if runs && test.path != "tests/support/mod.rs" {
            assert!(test.code.contains("on_private_desktop("), "{}", test.path);
        }
        let switches = ["SetThreadDesktop", "OpenInputDesktop", "OpenDesktopW"];
        let elsewhere = found(&[test], &switches, |source, word, _| {
            source.path == "tests/support/mod.rs" && word == "SetThreadDesktop"
        });
        assert!(elsewhere.is_empty(), "{elsewhere:#?}");
    }
}

#[test]
fn the_scan_finds_a_call_and_is_not_fooled_by_a_mention() {
    let source = |text: &str| Source {
        path: "src/sample.rs".to_owned(),
        code: code_only(text),
    };
    let hits = |text: &str| found(&[source(text)], NEVER, |_, _, _| false).len();
    assert_eq!(hits("unsafe { SetForegroundWindow(hwnd) };"), 1);
    assert_eq!(hits("let f = user32::SendInput;"), 1);
    assert_eq!(hits("window.focus_window();"), 1);
    assert_eq!(hits("// SetForegroundWindow(hwnd)"), 0);
    assert_eq!(hits("/* /* nested */ SendInput() */ let x = 1;"), 0);
    assert_eq!(hits("let s = \"SendInput \\\" SetFocus\";"), 0);
    assert_eq!(
        hits("let s = r#\"SendInput \" SetFocus\"#; SetFocus(w);"),
        1
    );
    assert_eq!(
        hits("let c = '\"'; SetFocus(w); let d = '\\''; SetFocus(w);"),
        2
    );
    assert_eq!(
        hits("let e = '\\\\'; SetFocus(w); let n = '\\n'; SetFocus(w);"),
        2
    );
    assert_eq!(hits("fn life<'a>(x: &'a str) { SetFocus(x) }"), 1);
    assert_eq!(hits("MySetFocusHelper(w); SetFocus2(w);"), 0, "other words");
}
