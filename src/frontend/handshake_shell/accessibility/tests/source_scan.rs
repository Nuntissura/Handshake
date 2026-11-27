//! AC214-1, AC214-7 and HBR-QUIET as source scans of this crate, with the window crate's scanner (comments and
//! literals blanked, so naming a call is not making one): no platform or toolkit type in the interface, only AccessKit
//! core; Win32 and the adapter in `src/platform.rs` alone, with every unsafe block; no call anywhere, tests included,
//! that activates, orders, shows or focuses a window or injects input; a request made only inside the crate; every loop
//! of a test on a private desktop; the adapter is the asset's revision, and no banned crate is locked.

#[path = "../../window/tests/source/mod.rs"]
mod source;

use source::{below, code_only, lines_with, Source};

/// Not in the crate's code outside `src/platform.rs`; nor is a path into the `windows` crate ([`windows_paths`]).
const PLATFORM: &[&str] = &[
    "windows_sys",
    "accesskit_windows",
    "HWND",
    "HINSTANCE",
    "LPARAM",
    "WPARAM",
    "unsafe",
    "isize",
];

/// Nowhere in the crate's code.
const TOOLKITS: &[&str] = &[
    "winit",
    "ash",
    "vk",
    "raw_window_handle",
    "egui",
    "eframe",
    "wgpu",
    "glow",
    "kumihimo",
    "accesskit_consumer",
    "accesskit_winit",
];

/// Never, in the crate's code and its tests: the window crate's ban, and what injects input.
const NEVER: &[&str] = &[
    "SendInput",
    "keybd_event",
    "mouse_event",
    "InjectTouchInput",
    "InjectSyntheticPointerInput",
    "InjectMouseInput",
    "InjectKeyboardInput",
    "SetForegroundWindow",
    "BringWindowToTop",
    "SetActiveWindow",
    "SwitchToThisWindow",
    "AllowSetForegroundWindow",
    "LockSetForegroundWindow",
    "AttachThreadInput",
    "SetCursorPos",
    "ClipCursor",
    "SetWindowsHookExW",
    "SetWinEventHook",
    "FlashWindow",
    "FlashWindowEx",
    "SwitchDesktop",
    "OpenInputDesktop",
    "ShowWindow",
    "ShowWindowAsync",
    "SetWindowPos",
    "SetWindowPlacement",
    "MoveWindow",
    "DeferWindowPos",
    "OpenIcon",
    "CloseWindow",
    "SendMessageW",
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

/// Lines with a path into the `windows` crate (`windows::`); a field or method named `windows` is no such path.
fn windows_paths(sources: &[Source]) -> Vec<String> {
    let mut found = Vec::new();
    for source in sources {
        for (number, line) in source.code.lines().enumerate() {
            let path = line.match_indices("windows::").any(|(at, _)| {
                !line[..at]
                    .chars()
                    .next_back()
                    .is_some_and(|c| c.is_alphanumeric() || c == '_')
            });
            if path {
                found.push(format!("{}:{}", source.path, number + 1));
            }
        }
    }
    found
}

/// Lines that call the Win32 `SetFocus`, a free function. A UI Automation client's `element.SetFocus()` asks the
/// provider, which is what the tests prove queues a request and activates nothing.
fn win32_set_focus(sources: &[Source]) -> Vec<String> {
    let part_of_word = |c: char| c.is_alphanumeric() || c == '_';
    let mut found = Vec::new();
    for source in sources {
        for (number, line) in source.code.lines().enumerate() {
            let free = line.match_indices("SetFocus").any(|(at, word)| {
                let before = line[..at].trim_end();
                !before.ends_with('.')
                    && !line[..at].chars().next_back().is_some_and(part_of_word)
                    && !line[at + word.len()..].starts_with(part_of_word)
            });
            if free {
                found.push(format!("{}:{}", source.path, number + 1));
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

/// What a manifest has under the table `header`: its header on a line of its own (a comment that names the table is not
/// the table), up to the next table (MT-214 E7-F4).
fn table<'a>(manifest: &'a str, header: &str) -> Option<&'a str> {
    let mut at = 0;
    let mut start = None;
    for line in manifest.split_inclusive('\n') {
        let text = line.trim();
        match start {
            None if text == header => start = Some(at + line.len()),
            Some(from) if text.starts_with('[') => return Some(&manifest[from..at]),
            _ => {}
        }
        at += line.len();
    }
    start.map(|from| &manifest[from..])
}

/// The names of a dependency table of the manifest.
fn dependencies(header: &str) -> Vec<String> {
    let manifest = include_str!("../Cargo.toml");
    table(manifest, header)
        .unwrap_or_else(|| panic!("{header}"))
        .lines()
        .filter(|line| !line.starts_with('#') && !line.starts_with(' ') && !line.starts_with(']'))
        .filter_map(|line| line.split_once(" = ").map(|(name, _)| name.trim().to_owned()))
        .collect()
}

/// AC214-1: only AccessKit core types and the window and input crates' data types in the interface; Win32 and the
/// adapter only in `src/platform.rs`, which makes nothing public.
#[test]
fn no_platform_or_toolkit_type_outside_the_platform_module() {
    // The crate's code: its unit tests (`*_tests.rs`) read the consumer the adapter uses, which is no interface.
    let sources: Vec<Source> = below("src")
        .into_iter()
        .filter(|source| !source.path.ends_with("_tests.rs"))
        .collect();
    assert!(sources.len() >= 6, "the scan reads the crate");
    let (platform, rest): (Vec<Source>, Vec<Source>) =
        sources.into_iter().partition(|source| source.path == "src/platform.rs");
    assert_eq!(platform.len(), 1);
    let mut found = lines(&rest, PLATFORM);
    found.extend(windows_paths(&rest));
    assert!(found.is_empty(), "{found:#?}");
    let found = lines(&rest, TOOLKITS);
    assert!(found.is_empty(), "{found:#?}");
    let public: Vec<&str> = platform[0]
        .code
        .lines()
        .filter(|line| line.trim_start().starts_with("pub ") || line.contains(" pub "))
        .collect();
    assert!(public.is_empty(), "src/platform.rs makes nothing public: {public:#?}");
    assert!(!lines_with(&platform[0].code, "unsafe").is_empty(), "the scan sees the platform module's code");
    // The root passes on no loop, control or native window of the window crate.
    let root = source("src/lib.rs");
    for word in ["run", "Control", "NativeWindowToken", "Win32Window", "ShellHost", "LaneWaker"] {
        assert!(lines_with(&root.code, word).is_empty(), "lib.rs passes on {word}");
    }
    let control = vec![Source {
        path: "src/sample.rs".to_owned(),
        code: code_only(
            "pub fn window(&self) -> HWND { self.hwnd } // windows_sys\nuse winit::window::Window;\n\
             fn windows(&self) {}\nuse windows::Win32::Foundation::RECT;",
        ),
    }];
    assert_eq!(lines(&control, PLATFORM).len(), 1, "the scan finds a type, not a mention");
    assert_eq!(lines(&control, TOOLKITS).len(), 1);
    assert_eq!(windows_paths(&control), ["src/sample.rs:4"], "a path, not a method named windows");
    assert_eq!(dependencies("[dependencies]"), ["handshake_shell_window", "handshake_shell_input", "accesskit"]);
    assert_eq!(dependencies("[target.'cfg(windows)'.dependencies]"), ["accesskit_windows", "windows-sys"]);
}

/// HBR-QUIET-001/002, AC214-7: nothing in the crate or its tests activates, shows, orders or focuses a window, or
/// injects input; the tests' UI Automation client asks the provider only.
#[test]
fn nothing_activates_orders_or_focuses_a_window_or_injects_input() {
    let sources: Vec<Source> = below("src").into_iter().chain(below("tests")).collect();
    assert!(sources.len() >= 10, "the scan reads the crate and its tests");
    let found = lines(&sources, NEVER);
    assert!(found.is_empty(), "{found:#?}");
    let found = win32_set_focus(&sources);
    assert!(found.is_empty(), "{found:#?}");
    let control = vec![Source {
        path: "tests/sample.rs".to_owned(),
        code: code_only("unsafe { SetFocus(hwnd) }; element.SetFocus()?; // SendInput(1, &i, 40)\nShowWindow(h, 4);"),
    }];
    assert_eq!(lines(&control, NEVER).len(), 1, "the scan finds calls, not mentions");
    // E7-F8: asynchronous and placement forms of showing and moving a window.
    for call in ["ShowWindowAsync(h, 1);", "SetWindowPlacement(h, &p);", "MoveWindow(h, 0, 0, 1, 1, 1);"] {
        let sample = vec![Source { path: "tests/sample.rs".to_owned(), code: code_only(call) }];
        assert_eq!(lines(&sample, NEVER).len(), 1, "{call}");
    }
    assert_eq!(win32_set_focus(&control).len(), 1, "the Win32 call, not the provider's");
    let provider_only = vec![Source {
        path: "tests/sample.rs".to_owned(),
        code: code_only("root.SetFocus()?;\n    element\n        .SetFocus()\n"),
    }];
    assert!(win32_set_focus(&provider_only).is_empty());
}

/// AC214-7: a request's origin is set inside the crate only; no public field, and no public method makes one.
#[test]
fn a_request_and_its_origin_are_made_inside_the_crate_only() {
    let request = source("src/request.rs");
    let fields = block(&request.code, "pub struct AccessRequest {");
    let public: Vec<&str> = fields.lines().map(str::trim).filter(|line| line.starts_with("pub")).collect();
    assert!(public.is_empty(), "{public:?}");
    assert!(!lines_with(fields, "origin").is_empty(), "the scan reads the fields");
    // Every `impl AccessRequest` block of the crate, not the first alone (E7-F8).
    let sources: Vec<Source> = below("src")
        .into_iter()
        .filter(|source| !source.path.ends_with("_tests.rs"))
        .collect();
    let made = request_makers(&sources);
    assert!(made.is_empty(), "{made:#?}");
    let public: usize = sources
        .iter()
        .flat_map(|source| impl_blocks(&source.code, "AccessRequest"))
        .map(|methods| methods.matches("pub fn").count())
        .sum();
    assert!(public >= 8, "the scan reads the methods: {public}");
    // The adapter's requests are another program's, and nothing in the crate says the Operator's.
    let hub = source("src/hub.rs");
    assert!(!lines_with(&hub.code, "OtherProgram").is_empty());
    for source in below("src") {
        assert!(lines_with(&source.code, "OperatorDevice").is_empty(), "{}", source.path);
    }
    let sample = |code: &str| vec![Source { path: "src/sample.rs".to_owned(), code: code_only(code) }];
    let forged = sample("impl AccessRequest {\n    pub fn forge(origin: Origin) -> Self {\n    }\n}\n");
    assert_eq!(request_makers(&forged).len(), 1, "the scan sees a constructor");
    // A second block, after a clean one, with a constructor of any shape that takes an origin.
    let second = sample(
        "impl AccessRequest {\n    pub fn at(&self) -> u8 { 1 }\n}\nimpl AccessRequest {\n    pub fn with(origin: Origin, w: u8) -> AccessRequest {\n    }\n}\n",
    );
    assert_eq!(request_makers(&second).len(), 1, "the second block is read too");
    let taking = sample("impl AccessRequest {\n    pub fn set(&mut self, origin: Origin) {}\n    pub fn origin(&self) -> Origin { self.origin }\n}\n");
    assert_eq!(request_makers(&taking).len(), 1, "taking an origin, not returning one");
    assert_eq!(request_makers(&sample("impl From<u8> for AccessRequest {\n}\n")).len(), 1, "a trait impl");
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

/// HBR-QUIET-001: every window of a test is on a private desktop, through the window crate's support; so is the UI
/// Automation client's thread.
#[test]
fn every_loop_and_client_of_a_test_runs_on_a_private_desktop() {
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
                "std::thread::spawn",
                "thread::spawn",
                "thread::Builder",
                "thread::scope",
                "Builder",
            ],
        );
        assert!(direct.is_empty(), "a thread of a test is made by the support, on the loop's desktop: {direct:#?}");
    }
    assert!(loops >= 1, "the scan sees the loop");
    let outside = code_only("fn a() {\n    on_private_desktop(L, || {});\n    run_script(c, e, w);\n}\n");
    assert_eq!(loops_outside_a_private_desktop(&outside), [3], "the scan sees one outside");
    // E7-F8: a thread made another way is found as well.
    let words = ["thread::Builder", "thread::scope", "Builder"];
    for made in [
        "let h = std::thread::Builder::new().spawn(f);",
        "use std::thread::{self, Builder};",
        "std::thread::scope(|s| {});",
    ] {
        let sample = vec![Source { path: "tests/sample.rs".to_owned(), code: code_only(made) }];
        assert!(!lines(&sample, &words).is_empty(), "{made}");
    }
}

/// Whether `path` names `name`, by its last path segment: `AccessRequest`, `request::AccessRequest` and
/// `crate::request::AccessRequest` all do (E8-F2c).
fn names(path: &str, name: &str) -> bool {
    let mut path = path.trim();
    if path.starts_with('<') {
        // `impl<T> AccessRequest`: the generics come first.
        let mut depth = 0;
        let end = path.find(|c| {
            depth += i32::from(c == '<') - i32::from(c == '>');
            depth == 0
        });
        path = end.map_or(path, |end| path[end + 1..].trim());
    }
    path == name || path.ends_with(&format!("::{name}"))
}

/// Each `impl ... {` header of `code` (what follows `impl`, up to its `{`) with the offset of that brace.
fn impl_headers(code: &str) -> Vec<(&str, usize)> {
    let mut headers = Vec::new();
    for (at, _) in code.match_indices("impl") {
        let before_ok = code[..at].chars().next_back().is_none_or(|c| !(c.is_alphanumeric() || c == '_'));
        let rest = &code[at + 4..];
        if !before_ok || !rest.starts_with(|c: char| c.is_whitespace() || c == '<') {
            continue;
        }
        if let Some(open) = rest.find('{') {
            headers.push((&rest[..open], at + 4 + open));
        }
    }
    headers
}

/// The text of each inherent `impl <name> {` block in `code` (the name by path suffix), up to its closing brace.
fn impl_blocks<'a>(code: &'a str, name: &str) -> Vec<&'a str> {
    impl_headers(code)
        .into_iter()
        .filter(|(header, _)| !header.contains(" for ") && names(header, name))
        .map(|(_, open)| block(&code[open..], "{"))
        .collect()
}

/// Whether `code` has a trait impl for `name` (`impl X for name`, the name by path suffix).
fn has_trait_impl_for(code: &str, name: &str) -> bool {
    impl_headers(code)
        .into_iter()
        .any(|(header, _)| header.rsplit_once(" for ").is_some_and(|(_, target)| names(target, name)))
}

/// Signatures that make a request or take an origin, in any `impl AccessRequest` block of `sources`, and any impl of a
/// trait for the request (`From`, `Default`, ...): the crate alone makes a request, with its own origin (E7-F8).
fn request_makers(sources: &[Source]) -> Vec<String> {
    let mut found = Vec::new();
    for source in sources {
        if has_trait_impl_for(&source.code, "AccessRequest") {
            found.push(format!("{}: a trait impl for AccessRequest", source.path));
        }
        for methods in impl_blocks(&source.code, "AccessRequest") {
            for (at, _) in methods.match_indices("pub fn") {
                let signature = &methods[at..];
                let signature = &signature[..signature.find('{').unwrap_or(signature.len())];
                let params = signature.split("->").next().unwrap_or_default();
                let makes = ["Self", "AccessRequest"]
                    .iter()
                    .any(|made| !lines_with(signature, made).is_empty());
                if makes || !lines_with(params, "Origin").is_empty() {
                    found.push(format!("{}: {}", source.path, signature.trim()));
                }
            }
        }
    }
    found
}

/// AC214-1: the adapter is the asset's, at the revision the current application uses, and nothing banned is locked.
#[test]
fn the_adapter_is_the_assets_and_no_banned_crate_is_locked() {
    let lock = include_str!("../../Cargo.lock");
    let workspace = include_str!("../../Cargo.toml");
    const REVISION: &str = "8eadc5b57dc20d96bf29340f835505ad8ff7cbc8";
    // The table's own header line: a comment that names the table comes first in the workspace manifest (E7-F4).
    let patch = table(workspace, "[patch.crates-io]").expect("the workspace patches the adapter");
    assert!(patch.contains("accesskit_windows") && patch.contains(REVISION), "{patch}");
    let adapters: Vec<&str> = lock
        .split("[[package]]")
        .filter(|package| package.contains("name = \"accesskit_windows\""))
        .collect();
    assert_eq!(adapters.len(), 1, "one adapter: {adapters:#?}");
    assert!(
        adapters[0].contains(&format!("source = \"git+https://github.com/Nuntissura/assets.git?rev={REVISION}#{REVISION}\"")),
        "{}",
        adapters[0]
    );
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
        "accesskit_winit",
    ] {
        assert!(!lock.contains(&format!("name = \"{name}\"")), "{name} is locked");
    }
    assert!(lock.contains("name = \"handshake_shell_accessibility\""), "the crate is locked");
}

/// MT-214 E7-F4: a table is found by its header line, not by the first mention of its name, which the workspace
/// manifest has in a comment above it. Fails if the first mention is taken again.
#[test]
fn a_comment_naming_a_table_is_not_the_table() {
    let manifest = "# members stay out of [patch.crates-io] unless pinned\n[workspace]\nmembers = []\n\n  [patch.crates-io]\na = { git = \"x\" }\n[profile.dev]\nb = 1\n";
    let patch = table(manifest, "[patch.crates-io]").expect("the table");
    assert_eq!(patch, "a = { git = \"x\" }\n", "up to the next table");
    assert_eq!(table(manifest, "[workspace]"), Some("members = []\n\n"), "up to the next header");
    assert_eq!(table("# [patch.crates-io] only\n", "[patch.crates-io]"), None, "a comment alone is no table");
    assert_eq!(table("[a]\nx = 1\n", "[a]"), Some("x = 1\n"), "a table at the end");
}
