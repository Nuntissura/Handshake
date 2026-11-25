//! HBR-QUIET as a source scan: no call in this crate, its tests included, moves the Operator's focus or cursor or injects
//! input, only one file can show or order a window, and the only hook is the loop thread's own activation guard.

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
    "SetWindowsHookExA",
    "SetWinEventHook",
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

/// Hooks other than the guard's: global only, or seeing input.
const OTHER_HOOKS: &[&str] = &[
    "WH_KEYBOARD",
    "WH_KEYBOARD_LL",
    "WH_MOUSE",
    "WH_MOUSE_LL",
    "WH_JOURNALRECORD",
    "WH_JOURNALPLAYBACK",
    "WH_GETMESSAGE",
    "WH_CALLWNDPROC",
    "WH_CALLWNDPROCRET",
    "WH_SHELL",
    "WH_MSGFILTER",
    "WH_SYSMSGFILTER",
    "WH_FOREGROUNDIDLE",
    "WH_DEBUG",
];

/// The file of the activation guard, the only one that installs a hook.
const GUARD: &str = "src/platform/guard.rs";

/// Unit tests that show and order plain windows on a private desktop. Compiled for tests only.
const TEST_ONLY: &str = "src/platform/quiet_tests.rs";

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

fn source<'a>(sources: &'a [Source], path: &str) -> &'a Source {
    sources
        .iter()
        .find(|source| source.path == path)
        .unwrap_or_else(|| panic!("{path} is scanned"))
}

#[test]
fn nothing_moves_focus_or_cursor_or_injects_input() {
    let found = found(&every_source(), NEVER, |source, _, line| {
        source.path == LIBRARY_WRITER.0 && line.contains(LIBRARY_WRITER.1)
    });
    assert!(found.is_empty(), "{found:#?}");
}

#[test]
fn the_only_hook_is_the_loop_threads_activation_guard() {
    let sources = every_source();
    let installs = found(&sources, &["SetWindowsHookExW"], |_, _, _| false);
    assert!(!installs.is_empty(), "the scan sees the guard");
    assert!(
        installs.iter().all(|hit| hit.starts_with(&format!("{GUARD}:"))),
        "{installs:#?}"
    );
    let others = found(&sources, OTHER_HOOKS, |_, _, _| false);
    assert!(others.is_empty(), "{others:#?}");
    // The call: a CBT hook, for the calling thread, with no module.
    let guard = &source(&sources, GUARD).code;
    let call = guard
        .find("SetWindowsHookExW(")
        .map(|at| arguments(&guard[at + "SetWindowsHookExW".len()..]))
        .unwrap();
    let call: String = call.split_whitespace().collect();
    assert_eq!(call, "(WH_CBT,Some(on_cbt),0,GetCurrentThreadId())");
    assert!(!lines_with(guard, "UnhookWindowsHookEx").is_empty(), "and it is removed");
}

#[test]
fn only_one_file_shows_or_orders_windows() {
    let sources = below("src");
    let elsewhere = found(&sources, ONLY_IN_PLACE, |source, _, _| {
        source.path == "src/platform/place.rs" || source.path == TEST_ONLY
    });
    assert!(elsewhere.is_empty(), "{elsewhere:#?}");
    let activating = found(&sources, ACTIVATING, |source, _, _| source.path == TEST_ONLY);
    assert!(activating.is_empty(), "{activating:#?}");
    let place = source(&sources, "src/platform/place.rs");
    assert!(
        !lines_with(&place.code, "SetWindowPos").is_empty(),
        "the file is the one that does"
    );
    // The exception is compiled for tests only.
    let platform = &source(&sources, "src/platform/mod.rs").code;
    let declared: Vec<&str> = platform.lines().map(str::trim).collect();
    let at = declared
        .iter()
        .position(|line| *line == "mod quiet_tests;")
        .expect("declared");
    assert_eq!(declared[at - 1], "#[cfg(test)]");
}

#[test]
fn a_window_is_created_hidden_and_inactive() {
    let sources = below("src");
    let window = source(&sources, "src/platform/window.rs");
    assert_eq!(lines_with(&window.code, "create_window").len(), 1);
    for hidden in ["with_visible(false)", "with_active(false)"] {
        assert_eq!(window.code.matches(hidden).count(), 1, "{hidden}");
    }
    let shown = found(&sources, &["WS_VISIBLE"], |_, _, _| false);
    assert!(shown.is_empty(), "{shown:#?}");
}

/// The text from an opening parenthesis to its match.
fn arguments(code: &str) -> &str {
    let mut depth = 0;
    for (at, c) in code.char_indices() {
        match c {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 {
                    return &code[..=at];
                }
            }
            _ => {}
        }
    }
    code
}

/// Where `run_script` is called, by offset; not where it is defined or imported.
fn calls_of_run_script(code: &str) -> Vec<usize> {
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
        .collect()
}

/// Lines with a call of `run_script` that is not inside the closure of an `on_private_desktop(` call.
fn runs_outside_a_private_desktop(code: &str) -> Vec<usize> {
    let spans: Vec<(usize, usize)> = code
        .match_indices("on_private_desktop(")
        .map(|(at, word)| {
            let open = at + word.len() - 1;
            (open, open + arguments(&code[open..]).len())
        })
        .collect();
    let line_of = |offset: usize| code[..offset].matches('\n').count() + 1;
    calls_of_run_script(code)
        .into_iter()
        .filter(|offset| !spans.iter().any(|(from, to)| (*from..*to).contains(offset)))
        .map(line_of)
        .collect()
}

#[test]
fn every_test_that_runs_a_loop_runs_it_on_a_private_desktop() {
    let tests = below("tests");
    let mut runs = 0;
    for test in &tests {
        let outside = runs_outside_a_private_desktop(&test.code);
        assert!(outside.is_empty(), "{}:{outside:?}", test.path);
        runs += calls_of_run_script(&test.code).len();
        let switches = ["SetThreadDesktop", "OpenInputDesktop", "OpenDesktopW", "CreateDesktopW"];
        let elsewhere = found(std::slice::from_ref(test), &switches, |source, word, _| {
            source.path == "tests/support/mod.rs"
                && matches!(word, "SetThreadDesktop" | "CreateDesktopW")
        });
        assert!(elsewhere.is_empty(), "{elsewhere:#?}");
    }
    assert!(runs >= 7, "the scan sees the loops: {runs}");
    // In the crate's own source only its unit tests bind a thread to a desktop.
    let src = below("src");
    let bound = found(&src, &["SetThreadDesktop", "CreateDesktopW"], |source, _, _| {
        source.path == TEST_ONLY
    });
    assert!(bound.is_empty(), "{bound:#?}");
}

#[test]
fn the_private_desktop_scan_sees_a_loop_outside_one() {
    let inside = code_only(
        "fn a() {\n    on_private_desktop(LIMIT, || {\n        run_script(c, e, w)\n    });\n}\n",
    );
    assert!(runs_outside_a_private_desktop(&inside).is_empty());
    let one_of_two = code_only(
        "fn a() {\n    on_private_desktop(LIMIT, || { run_script(c, e, w) });\n    run_script(c, e, w);\n}\n",
    );
    assert_eq!(runs_outside_a_private_desktop(&one_of_two), [3]);
    let before = code_only("fn a() {\n    run_script(c, e, w);\n    on_private_desktop(LIMIT, || {});\n}\n");
    assert_eq!(runs_outside_a_private_desktop(&before), [2]);
}

#[test]
fn the_scan_finds_a_call_and_is_not_fooled_by_a_mention() {
    let source = |text: &str| Source {
        path: "src/sample.rs".to_owned(),
        code: code_only(text),
    };
    // The scan counts lines: a line with a call counts once, however many it holds.
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
    // After a character literal of a quote, and after one of an escaped quote, the scan still reads code.
    assert_eq!(hits("let c = '\"'; SetFocus(w);\nlet d = '\\''; SetFocus(w);"), 2);
    assert_eq!(hits("let c = '\"';\nlet d = '\\''; SetFocus(w);"), 1);
    assert_eq!(
        hits("let e = '\\\\'; SetFocus(w);\nlet n = '\\n'; SetFocus(w);"),
        2
    );
    assert_eq!(hits("fn life<'a>(x: &'a str) { SetFocus(x) }"), 1);
    assert_eq!(hits("MySetFocusHelper(w); SetFocus2(w);"), 0, "other words");
    assert_eq!(hits("SetFocus(a); SetFocus(b);"), 1, "one line");
}
