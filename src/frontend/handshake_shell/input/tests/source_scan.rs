//! AC214-1, AC214-5, AC214-6, HBR-HOST-001 and HBR-QUIET as source scans of this crate, with the window crate's own
//! scanner (comments and literals blanked, so naming a call is not making one): no window library, graphics binding,
//! Win32 type or unsafe code in the crate's code, and no dependency but the window crate; no call anywhere, tests
//! included, that injects input, sets a composition, moves focus or the cursor; an input event's origin and stamp can
//! be set only inside the crate; the state kept for the input method holds no composed text; every loop of a test on a
//! private desktop; no banned crate in the lock.

#[path = "../../window/tests/source/mod.rs"]
mod source;

use source::{below, code_only, lines_with, Source};

/// Not in this crate's code at all: they come through the window crate, privately.
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

/// Never, in the crate's code and its tests: what injects input or a composition, and the window crate's ban
/// (`tests/test_focus_ban.rs`).
const NEVER: &[&str] = &[
    "SendInput",
    "keybd_event",
    "mouse_event",
    "InjectTouchInput",
    "InjectSyntheticPointerInput",
    "InjectMouseInput",
    "InjectKeyboardInput",
    "CreateSyntheticPointerDevice",
    "ImmSetCompositionStringW",
    "ImmSetCompositionStringA",
    "ImmNotifyIME",
    "SetForegroundWindow",
    "BringWindowToTop",
    "SetActiveWindow",
    "SwitchToThisWindow",
    "AllowSetForegroundWindow",
    "LockSetForegroundWindow",
    "SetFocus",
    "AttachThreadInput",
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
    "SendMessageW",
    "focus_window",
    "request_user_attention",
    "set_cursor_position",
];

/// What a composition consists of. None of it in the state the crate keeps (MT-198 KG-03 keeps it).
const COMPOSED: &[&str] = &[
    "String",
    "TypedText",
    "str",
    "char",
    "Vec",
    "Box",
    "Cow",
    "Range",
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

fn source(path: &str) -> Source {
    below("src")
        .into_iter()
        .find(|source| source.path == path)
        .unwrap_or_else(|| panic!("{path} is read"))
}

/// What `head`, which ends with an opening brace, opens in `code`, up to its closing brace.
fn block<'a>(code: &'a str, head: &str) -> &'a str {
    let start = code.find(head).unwrap_or_else(|| panic!("{head}")) + head.len();
    let mut depth = 1;
    for (at, c) in code[start..].char_indices() {
        match c {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return &code[start..start + at];
                }
            }
            _ => {}
        }
    }
    panic!("{head} has no end");
}

/// The fields of the struct named `name` in `code`.
fn body<'a>(code: &'a str, name: &str) -> &'a str {
    block(code, &format!("struct {name} {{"))
}

#[test]
fn no_window_library_or_graphics_binding_in_the_crates_code() {
    let sources: Vec<Source> = below("src")
        .into_iter()
        .map(|source| Source {
            code: source
                .code
                .replace("cfg(windows)", "")
                .replace("not(windows)", ""),
            path: source.path,
        })
        .collect();
    assert!(sources.len() >= 6, "the scan reads the crate");
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
    assert_eq!(named, ["handshake_shell_window"]);
    // Only data types of the window crate are passed on: not its loop, its control, or a native window.
    let root = source("src/lib.rs");
    for word in [
        "run",
        "Control",
        "NativeWindowToken",
        "Win32Window",
        "ShellHost",
    ] {
        assert!(
            lines_with(&root.code, word).is_empty(),
            "lib.rs passes on {word}"
        );
    }
}

/// HBR-QUIET-002, AC214-5: no injection of input or of a composition, no focus or cursor moved, anywhere.
#[test]
fn nothing_injects_input_or_a_composition_or_moves_focus_or_cursor() {
    let sources: Vec<Source> = below("src").into_iter().chain(below("tests")).collect();
    assert!(sources.len() >= 8, "the scan reads the crate and its tests");
    let found = lines(&sources, NEVER);
    assert!(found.is_empty(), "{found:#?}");
    let control = Source {
        path: "tests/sample.rs".to_owned(),
        code: code_only("unsafe { SendInput(1, &i, 40) }; // keybd_event(0, 0, 0, 0)\nImmNotifyIME(h, 0x15, 1, 0);"),
    };
    assert_eq!(
        lines(&[control], NEVER).len(),
        2,
        "the scan finds calls, not mentions"
    );
}

/// The one call to the input method that is made at all (MT-214 S3-Q1), with its exact shape: complete the window's
/// composition, so that its text is committed to the target it was typed at.
const COMPLETION: &str = "ImmNotifyIME(himc, NI_COMPOSITIONSTR, CPS_COMPLETE, 0)";

/// The window crate's one module that makes it.
const COMPLETION_MODULE: &str = "../window/src/platform/ime.rs";

/// What else asks the input method to do something with a composition: never, anywhere in the window crate.
const COMPOSITION_CALLS: &[&str] = &[
    "ImmSetCompositionStringW",
    "ImmSetCompositionStringA",
    "CPS_CANCEL",
    "CPS_CONVERT",
    "CPS_REVERT",
];

/// Every call of `ImmNotifyIME` in `sources`: where, and its text with the white space of its lines collapsed.
fn notify_calls(sources: &[Source]) -> Vec<(String, String)> {
    let mut calls = Vec::new();
    for source in sources {
        for (at, _) in source.code.match_indices("ImmNotifyIME(") {
            let rest = &source.code[at..];
            let end = rest.find(')').map_or(rest.len(), |end| end + 1);
            let call = rest[..end].split_whitespace().collect::<Vec<_>>().join(" ");
            calls.push((source.path.clone(), call));
        }
    }
    calls
}

/// MT-214 S3-Q1: this crate never calls the input method (the scan above), and the window crate does it exactly once:
/// `ImmNotifyIME` with `NI_COMPOSITIONSTR` and `CPS_COMPLETE`, in its `platform/ime.rs`, which turns a composition's
/// string into its result string. No other call there asks the input method to cancel, convert, revert or set a
/// composition. Fails if a second call, another shape (a cancel, a conversion) or another module appears.
#[test]
fn the_window_crate_makes_the_one_completion_call_and_no_other_composition_call() {
    let window = below("../window/src");
    assert!(window.len() >= 20, "the scan reads the window crate");
    assert_eq!(
        notify_calls(&window),
        [(COMPLETION_MODULE.to_owned(), COMPLETION.to_owned())],
        "one call, of this shape, in this module"
    );
    let found = lines(&window, COMPOSITION_CALLS);
    assert!(found.is_empty(), "{found:#?}");
    // The controls: a second call, another shape and another module are seen.
    let sample = |path: &str, code: &str| Source {
        path: path.to_owned(),
        code: code_only(code),
    };
    let twice = sample(
        COMPLETION_MODULE,
        "ImmNotifyIME(himc, NI_COMPOSITIONSTR, CPS_COMPLETE, 0);
ImmNotifyIME(himc, NI_COMPOSITIONSTR, CPS_COMPLETE, 0);",
    );
    assert_eq!(notify_calls(&[twice]).len(), 2, "a second call is seen");
    let elsewhere = sample(
        "../window/src/platform/window.rs",
        "ImmNotifyIME(himc, NI_COMPOSITIONSTR, CPS_COMPLETE, 0);",
    );
    assert_ne!(notify_calls(&[elsewhere])[0].0, COMPLETION_MODULE, "another module is seen");
    let cancel = sample(
        COMPLETION_MODULE,
        "ImmNotifyIME(
    himc,
    NI_COMPOSITIONSTR,
    CPS_CANCEL,
    0,
);",
    );
    assert_ne!(notify_calls(&[cancel])[0].1, COMPLETION, "a cancel is another shape");
    let set = [sample(COMPLETION_MODULE, "ImmSetCompositionStringW(h, 0, p, 1, q, 2);")];
    assert_eq!(lines(&set, COMPOSITION_CALLS).len(), 1);
}

/// AC214-5: an input's origin and stamp are set inside the crate only. A public field would let a caller give a model's
/// input the Operator's origin.
#[test]
fn an_input_events_origin_and_stamp_cannot_be_set_outside_the_crate() {
    let event = source("src/event.rs");
    let fields = body(&event.code, "InputEvent");
    let public: Vec<&str> = fields
        .lines()
        .map(str::trim)
        .filter(|line| line.starts_with("pub"))
        .collect();
    assert!(public.is_empty(), "{public:?}");
    for field in ["origin", "stamp", "synthetic"] {
        assert!(
            !lines_with(fields, field).is_empty(),
            "the scan reads the fields: {field}"
        );
    }
    // Its constructors are the crate's own: no public method of it makes one.
    let methods = block(&event.code, "impl InputEvent {");
    let mut public = 0;
    for (at, _) in methods.match_indices("pub fn") {
        let signature = &methods[at..];
        let signature = &signature[..signature.find('{').unwrap_or(signature.len())];
        public += 1;
        for made in ["Self", "InputEvent"] {
            assert!(lines_with(signature, made).is_empty(), "{signature}");
        }
    }
    assert!(public >= 5, "the scan reads the methods: {public}");
    let control =
        code_only("impl InputEvent {\n    pub fn forge(origin: Origin) -> Self {\n    }\n}\n");
    let forged = block(&control, "impl InputEvent {");
    assert!(
        !lines_with(forged, "Self").is_empty(),
        "the scan sees a public constructor"
    );
    let control = code_only("pub struct InputEvent {\n    pub origin: Origin,\n}\n");
    assert_eq!(
        body(&control, "InputEvent")
            .lines()
            .filter(|line| line.trim().starts_with("pub"))
            .count(),
        1
    );
}

/// AC214-6: the composition is Kumihimo's; what the crate keeps per window and per open composition holds none of it.
#[test]
fn the_input_method_state_holds_no_composed_text() {
    let ime = source("src/ime.rs");
    for name in ["Open", "WindowText", "ImeRouter", "ImeCounts"] {
        let fields = body(&ime.code, name);
        let found: Vec<&str> = COMPOSED
            .iter()
            .copied()
            .filter(|word| !lines_with(fields, word).is_empty())
            .collect();
        assert!(found.is_empty(), "{name} keeps {found:?}");
    }
    let control = code_only("struct Open {\n    preedit: String,\n}\n");
    assert_eq!(
        lines_with(body(&control, "Open"), "String").len(),
        1,
        "the scan sees a kept text"
    );
    // The tracker keeps keys and buttons, never text.
    let tracker = source("src/tracker.rs");
    for name in ["InputTracker", "Held", "HeldKey"] {
        let fields = body(&tracker.code, name);
        for word in ["String", "TypedText", "ImeEvent", "Composition"] {
            assert!(lines_with(fields, word).is_empty(), "{name} keeps {word}");
        }
    }
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

/// HBR-QUIET-001: every window of a test is on a private desktop, through the window crate's support.
#[test]
fn every_loop_of_a_test_runs_on_a_private_desktop() {
    let mut loops = 0;
    for test in below("tests") {
        let outside = loops_outside_a_private_desktop(&test.code);
        assert!(outside.is_empty(), "{}:{outside:?}", test.path);
        loops += test.code.matches("run_script(").count();
        let direct = lines(
            std::slice::from_ref(&test),
            &[
                "handshake_shell_window::run",
                "SetThreadDesktop",
                "CreateDesktopW",
            ],
        );
        assert!(direct.is_empty(), "{direct:#?}");
    }
    assert!(loops >= 1, "the scan sees the loop");
    let outside =
        code_only("fn a() {\n    on_private_desktop(L, || {});\n    run_script(c, e, w);\n}\n");
    assert_eq!(
        loops_outside_a_private_desktop(&outside),
        [3],
        "the scan sees one outside"
    );
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
        assert!(
            !lock.contains(&format!("name = \"{name}\"")),
            "{name} is locked"
        );
    }
    assert!(
        lock.contains("name = \"handshake_shell_input\""),
        "the crate is locked"
    );
}
