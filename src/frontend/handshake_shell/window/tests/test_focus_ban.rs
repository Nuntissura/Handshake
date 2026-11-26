//! HBR-QUIET as a source scan: no call in this crate, its tests included, moves the Operator's focus or cursor or injects
//! input, only one file can show or order a window, and the only hooks are the loop thread's own: its activation guard
//! and its watch of what it takes out of its own queue (MT-214 E3-F5), each for that thread alone. The clipboard tests'
//! window station is made fresh and never opened, and nothing writes a clipboard before its preflight passed (MT-214
//! E5-F1, E5-F1-S).

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

/// Hooks other than the loop thread's two: global only, or seeing input.
const OTHER_HOOKS: &[&str] = &[
    "WH_KEYBOARD",
    "WH_KEYBOARD_LL",
    "WH_MOUSE",
    "WH_MOUSE_LL",
    "WH_JOURNALRECORD",
    "WH_JOURNALPLAYBACK",
    "WH_CALLWNDPROC",
    "WH_CALLWNDPROCRET",
    "WH_SHELL",
    "WH_MSGFILTER",
    "WH_SYSMSGFILTER",
    "WH_FOREGROUNDIDLE",
    "WH_DEBUG",
];

/// The files that install a hook, and the one call each makes: a hook for the calling thread, with no module.
const HOOKS: [(&str, &str); 2] = [
    (
        "src/platform/guard.rs",
        "(WH_CBT,Some(on_cbt),0,GetCurrentThreadId())",
    ),
    (
        "src/platform/retrieval.rs",
        "(WH_GETMESSAGE,Some(on_get_message),0,GetCurrentThreadId())",
    ),
];

/// The one file that may name the message hook: it sees the loop thread's own queue only.
const MESSAGE_HOOK: (&str, &str) = ("src/platform/retrieval.rs", "WH_GETMESSAGE");

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
fn the_only_hooks_are_the_loop_threads_own() {
    let sources = every_source();
    let installs = found(&sources, &["SetWindowsHookExW"], |_, _, _| false);
    let in_hook_files = |hit: &String| HOOKS.iter().any(|(file, _)| hit.starts_with(&format!("{file}:")));
    assert!(installs.iter().all(in_hook_files), "{installs:#?}");
    for (file, expected) in HOOKS {
        assert!(
            installs.iter().any(|hit| hit.starts_with(&format!("{file}:"))),
            "the scan sees {file}: {installs:#?}"
        );
        // The call, once: for the calling thread, with no module.
        let code = &source(&sources, file).code;
        assert_eq!(code.matches("SetWindowsHookExW(").count(), 1, "{file}");
        let call = code
            .find("SetWindowsHookExW(")
            .map(|at| arguments(&code[at + "SetWindowsHookExW".len()..]))
            .unwrap();
        let call: String = call.split_whitespace().collect();
        assert_eq!(call, expected, "{file}");
        assert!(!lines_with(code, "UnhookWindowsHookEx").is_empty(), "{file}: and it is removed");
    }
    let others = found(&sources, OTHER_HOOKS, |_, _, _| false);
    assert!(others.is_empty(), "{others:#?}");
    let message_hooks = found(&sources, &[MESSAGE_HOOK.1], |source, _, _| {
        source.path == MESSAGE_HOOK.0
    });
    assert!(message_hooks.is_empty(), "{message_hooks:#?}");
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
    enclosed(code, '(', ')')
}

/// The text from an opening bracket to its match.
fn enclosed(code: &str, open: char, close: char) -> &str {
    let mut depth = 0;
    for (at, c) in code.char_indices() {
        if c == open {
            depth += 1;
        } else if c == close {
            depth -= 1;
            if depth == 0 {
                return &code[..=at];
            }
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

/// The one file of the crate's own source that binds a thread to a desktop, and the desktop: the calling thread's, which
/// is the loop's (`platform/clipboard.rs`, the keeper thread of a write, MT-214 E4-F3).
const KEEPER: (&str, &str) = (
    "src/platform/clipboard.rs",
    "GetThreadDesktop(GetCurrentThreadId())",
);

/// The calls that run their closure on a private desktop; the second also on a window station of its own.
const PRIVATE: [&str; 2] = ["on_private_desktop(", "on_private_window_station("];

/// Lines with a call of `run_script` that is not inside the closure of a [`PRIVATE`] call.
fn runs_outside_a_private_desktop(code: &str) -> Vec<usize> {
    runs_outside(code, &PRIVATE)
}

/// Lines with a call of `run_script` that is not inside the arguments of one of `calls`.
fn runs_outside(code: &str, calls: &[&str]) -> Vec<usize> {
    let spans: Vec<(usize, usize)> = calls
        .iter()
        .flat_map(|call| code.match_indices(call))
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

/// The crate's `run` named in a test, by path: only `run_script` may call it, which checks its desktop too.
const DIRECT_RUN: &str = "handshake_shell_window::run";

/// Whether `code` imports the crate's `run`.
fn imports_run(code: &str) -> bool {
    code.match_indices("use handshake_shell_window::").any(|(at, _)| {
        let statement = &code[at..];
        let end = statement.find(';').unwrap_or(statement.len());
        !lines_with(&statement[..end], "run").is_empty()
    })
}

#[test]
fn every_test_that_runs_a_loop_runs_it_on_a_private_desktop() {
    let tests = below("tests");
    let mut runs = 0;
    for test in &tests {
        let outside = runs_outside_a_private_desktop(&test.code);
        assert!(outside.is_empty(), "{}:{outside:?}", test.path);
        let support = test.path == "tests/support/mod.rs";
        let direct = found(std::slice::from_ref(test), &[DIRECT_RUN], |_, _, _| support);
        assert!(direct.is_empty() && !imports_run(&test.code), "{}: {direct:?}", test.path);
        runs += calls_of_run_script(&test.code).len();
        let switches = [
            "SetThreadDesktop",
            "OpenInputDesktop",
            "OpenDesktopW",
            "CreateDesktopW",
            "OpenWindowStationW",
            "CreateWindowStationW",
            "SetProcessWindowStation",
        ];
        let elsewhere = found(std::slice::from_ref(test), &switches, |source, word, _| {
            source.path == "tests/support/mod.rs"
                && matches!(
                    word,
                    "SetThreadDesktop"
                        | "CreateDesktopW"
                        | "CreateWindowStationW"
                        | "SetProcessWindowStation"
                )
        });
        assert!(elsewhere.is_empty(), "{elsewhere:#?}");
    }
    assert!(runs >= 7, "the scan sees the loops: {runs}");
    // In the crate's own source only its unit tests bind a thread to a desktop, and the clipboard's keeper thread binds
    // itself to the loop thread's own desktop (MT-214 E4-F3); nothing makes a desktop or switches window stations.
    let src = below("src");
    let bound = found(&src, &["SetThreadDesktop", "CreateDesktopW"], |source, word, _| {
        source.path == TEST_ONLY || (source.path == KEEPER.0 && word == "SetThreadDesktop")
    });
    assert!(bound.is_empty(), "{bound:#?}");
    let keeper = &source(&src, KEEPER.0).code;
    assert_eq!(keeper.matches("SetThreadDesktop(").count(), 1, "one binding");
    assert_eq!(keeper.matches("GetThreadDesktop(").count(), 1, "of one desktop");
    assert_eq!(keeper.matches(KEEPER.1).count(), 1, "the loop thread's own");
    let stations = found(&src, &["CreateWindowStationW", "SetProcessWindowStation"], |_, _, _| false);
    assert!(stations.is_empty(), "{stations:#?}");
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
    let station = code_only(
        "fn a() {\n    on_private_window_station(LIMIT, || { run_script(c, e, w) });\n    run_script(c, e, w);\n}\n",
    );
    assert_eq!(runs_outside_a_private_desktop(&station), [3]);
    // The loop started without `run_script`.
    let direct = Source {
        path: "tests/sample.rs".to_owned(),
        code: code_only("let r = handshake_shell_window::run(c, h);\n"),
    };
    assert_eq!(found(&[direct], &[DIRECT_RUN], |_, _, _| false).len(), 1);
    assert!(imports_run(&code_only("use handshake_shell_window::{run, Control};\n")));
    assert!(imports_run(&code_only("use handshake_shell_window::{\n    Control,\n    run,\n};\n")));
    assert!(!imports_run(&code_only("use handshake_shell_window::{Control, RunReport};\nfn run() {}\n")));
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

/// The only test files that use the operating system's clipboard: the harness, then the clipboard test.
const HARNESS: [&str; 2] = ["tests/support/mod.rs", "tests/test_clipboard.rs"];

/// The arguments of the one call that makes a window station: no name and CWF_CREATE_ONLY, so it fails rather than open
/// a station that exists, the logon session's shared one among them (MT-214 E5-F1-S).
const FRESH_STATION: &str = "(ptr::null(),CWF_CREATE_ONLY,WINSTA_ALL_ACCESS,attributes)";

/// Calls that put something on a clipboard or take something off it.
const CLIPBOARD_WRITES: [&str; 4] = [
    "EmptyClipboard(",
    "SetClipboardData(",
    "write_clipboard(",
    "withdraw_clipboard(",
];

/// The check that passes on the fresh window station only, and only once its preflight passed
/// (`assert_clipboard_is_private` or `clipboard_is_private`).
const STATION_CHECK: &str = "clipboard_is_private(";

/// What can panic.
const PANICS: [&str; 7] = [
    "assert",
    "panic!",
    "unwrap(",
    "expect(",
    "unreachable!",
    "todo!",
    "unimplemented!",
];

fn is_word_end(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// The body of the first `fn name` in `code`, braces included.
fn body_of<'a>(code: &'a str, name: &str) -> Option<&'a str> {
    let named = format!("fn {name}");
    let after = code
        .match_indices(&named)
        .map(|(at, _)| at + named.len())
        .find(|end| code[*end..].starts_with(['(', '<']))?;
    let open = after + code[after..].find('{')?;
    Some(enclosed(&code[open..], '{', '}'))
}

/// Where each function body in `code` starts and ends, braces included.
fn bodies(code: &str) -> Vec<(usize, usize)> {
    code.match_indices("fn ")
        .filter(|(at, _)| !code[..*at].chars().next_back().is_some_and(is_word_end))
        .filter_map(|(at, _)| {
            let end = at + code[at..].find(['{', ';'])?;
            code[end..]
                .starts_with('{')
                .then(|| (end, end + enclosed(&code[end..], '{', '}').len()))
        })
        .collect()
}

/// Lines of `code` with a clipboard write that no [`STATION_CHECK`] earlier in its own function guards.
fn unchecked_writes(code: &str) -> Vec<usize> {
    let bodies = bodies(code);
    let mut lines: Vec<usize> = CLIPBOARD_WRITES
        .iter()
        .flat_map(|call| code.match_indices(call))
        .map(|(at, _)| at)
        .filter(|at| !code[..*at].chars().next_back().is_some_and(is_word_end))
        .filter(|at| {
            let inner = bodies
                .iter()
                .filter(|(from, to)| (*from..*to).contains(at))
                .max_by_key(|(from, _)| *from);
            !inner.is_some_and(|(from, _)| code[*from..*at].contains(STATION_CHECK))
        })
        .map(|at| code[..at].matches('\n').count() + 1)
        .collect();
    lines.sort_unstable();
    lines.dedup();
    lines
}

/// What is wrong with how the harness's code makes the clipboard tests' window station and lets them write: it must
/// make it with [`FRESH_STATION`] and never open one; switch to it and open its clipboard ("preflight") before it is
/// recorded as the tests' station, which [`STATION_CHECK`] reads; let nothing panic while the thread is the anonymous
/// user, and abort when it cannot stop being it; and run the test's work only after all that.
fn station_faults(support: &str) -> Vec<String> {
    let mut faults = Vec::new();
    let creates: Vec<String> = support
        .match_indices("CreateWindowStationW(")
        .map(|(at, word)| {
            let call = arguments(&support[at + word.len() - 1..]);
            call.split_whitespace().collect()
        })
        .collect();
    if creates != [FRESH_STATION] {
        faults.push(format!("CreateWindowStationW{creates:?}"));
    }
    let enter = body_of(support, "enter").unwrap_or_default();
    let order = [
        "created_as_anonymous(",
        "SetProcessWindowStation(",
        "preflight(",
        "*STATION.lock()",
    ];
    let at: Vec<Option<usize>> = order.iter().map(|word| enter.find(word)).collect();
    if at.iter().any(Option::is_none) || at.windows(2).any(|pair| pair[0] >= pair[1]) {
        faults.push(format!("enter: {order:?} at {at:?}"));
    }
    let records = support
        .match_indices("*STATION.lock()")
        .filter(|(at, _)| {
            support[*at..]
                .split(';')
                .next()
                .is_some_and(|statement| statement.contains("Some("))
        })
        .count();
    if records != 1 {
        faults.push(format!("the station is recorded as the tests' {records} times"));
    }
    let impersonated = body_of(support, "created_as_anonymous").and_then(|made| {
        let from = made.find("Anonymous::impersonate()")?;
        let to = from + made[from..].find("drop(anonymous)")?;
        Some(&made[from..to])
    });
    match impersonated {
        Some(span) if !PANICS.iter().any(|word| span.contains(word)) => {}
        _ => faults.push(format!("the impersonated span can panic: {impersonated:?}")),
    }
    let revert = support.find("impl Drop for Anonymous").and_then(|at| {
        let open = at + support[at..].find('{')?;
        Some(enclosed(&support[open..], '{', '}'))
    });
    match revert {
        Some(revert)
            if revert.contains("RevertToSelf()") && revert.contains("std::process::abort()") => {}
        _ => faults.push("a failed RevertToSelf does not abort".to_owned()),
    }
    let run = body_of(support, "on_private_window_station").unwrap_or_default();
    match (run.find("PrivateStation::enter()"), run.find("on_private_desktop(")) {
        (Some(entered), Some(work)) if entered < work => {}
        _ => faults.push("the work does not run after the station is entered".to_owned()),
    }
    faults
}

/// `code` with the first `from` after `anchor` replaced by `to`.
fn mutated(code: &str, anchor: &str, from: &str, to: &str) -> String {
    let at = code
        .find(anchor)
        .unwrap_or_else(|| panic!("{anchor:?} is in the code"));
    let at = at
        + code[at..]
            .find(from)
            .unwrap_or_else(|| panic!("{from:?} follows {anchor:?}"));
    format!("{}{to}{}", &code[..at], &code[at + from.len()..])
}

#[test]
fn the_clipboard_tests_make_a_fresh_station_and_write_only_after_its_preflight() {
    let sources = every_source();
    let faults = station_faults(&source(&sources, HARNESS[0]).code);
    assert!(faults.is_empty(), "{faults:#?}");
    // No other call makes a window station or opens one by name, the logon session's shared one included.
    let stations = [
        "CreateWindowStationW",
        "CreateWindowStationA",
        "OpenWindowStationW",
        "OpenWindowStationA",
    ];
    let elsewhere = found(&sources, &stations, |source, word, _| {
        source.path == HARNESS[0] && word == "CreateWindowStationW"
    });
    assert!(elsewhere.is_empty(), "{elsewhere:#?}");
    for path in HARNESS {
        let unchecked = unchecked_writes(&source(&sources, path).code);
        assert!(unchecked.is_empty(), "{path}:{unchecked:?}");
    }
    let clipboard_test = &source(&sources, HARNESS[1]).code;
    assert!(!calls_of_run_script(clipboard_test).is_empty(), "the scan sees its loop");
    let outside = runs_outside(clipboard_test, &["on_private_window_station("]);
    assert!(outside.is_empty(), "{}:{outside:?}", HARNESS[1]);
    // No other test opens or writes the operating system's clipboard.
    let os_clipboard = ["OpenClipboard", "EmptyClipboard", "SetClipboardData"];
    let elsewhere = found(&sources, &os_clipboard, |source, _, _| {
        source.path.starts_with("src/") || HARNESS.contains(&source.path.as_str())
    });
    assert!(elsewhere.is_empty(), "{elsewhere:#?}");
}

#[test]
fn the_clipboard_scan_catches_a_shared_station_and_a_write_before_the_preflight() {
    let sources = every_source();
    let support = &source(&sources, HARNESS[0]).code;
    let clipboard_test = &source(&sources, HARNESS[1]).code;
    let caught = |mutant: String| !station_faults(&mutant).is_empty();
    // The harness at 29763b430: the logon session's unnamed station, opened when it exists.
    assert!(caught(mutated(support, "CreateWindowStationW(", "CWF_CREATE_ONLY", "0")));
    // No preflight, or the station recorded as the tests' before it.
    assert!(caught(mutated(support, "fn enter(", "preflight(", "drop(")));
    let early = "*STATION.lock().unwrap() = Some(x); let shared";
    assert!(caught(mutated(support, "fn enter(", "let shared", early)));
    // Something that can panic while the thread is the anonymous user; a failed revert that lets it go on.
    let span = "assert_ne!(station, 0); drop(anonymous)";
    assert!(caught(mutated(support, "Anonymous::impersonate()", "drop(anonymous)", span)));
    assert!(caught(mutated(support, "impl Drop for Anonymous", "std::process::abort()", "()")));
    // The work before the station.
    let run = "fn on_private_window_station";
    assert!(caught(mutated(support, run, "PrivateStation::enter()", "PrivateStation::default()")));
    // A write without the check before it.
    let emptied = mutated(support, "impl Drop for EmptiedAtEnd", "if !clipboard_is_private()", "if false");
    assert!(!unchecked_writes(&emptied).is_empty());
    let put = mutated(clipboard_test, "fn put(", "assert_clipboard_is_private();", "");
    assert!(!unchecked_writes(&put).is_empty());
    let rendered = mutated(clipboard_test, "fn other_program(", "&& clipboard_is_private()", "");
    assert!(!unchecked_writes(&rendered).is_empty());
    // The clipboard test's loop on a desktop of the station the process had.
    let station = "on_private_window_station(";
    let moved = mutated(clipboard_test, "fn a_write_asks", station, "on_private_desktop(");
    assert_eq!(runs_outside(&moved, &[station]).len(), 1);
}
