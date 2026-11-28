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

/// `code` without any whitespace, so the scans do not depend on how rustfmt breaks a line.
fn squeezed(code: &str) -> String {
    code.chars().filter(|c| !c.is_whitespace()).collect()
}

/// The text between the braces of the function whose signature starts with `signature`, in code only text.
fn body_of<'a>(code: &'a str, signature: &str) -> Option<&'a str> {
    let at = code.find(signature)?;
    let open = at + code[at..].find('{')?;
    let mut depth = 0;
    for (offset, c) in code[open..].char_indices() {
        match c {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(&code[open + 1..open + offset]);
                }
            }
            _ => {}
        }
    }
    None
}

/// What is wrong with the order of the shutdown in the text of `host/ending.rs` (MT-214 E9-F1): the proof of every
/// window's presents before any surface is released, the claims settled before the owner closes and again after, one
/// close. Read on the code of the file only: cut at its test module, comments and the content of literals blanked, so
/// a call named in a string, a comment or a test is not a call.
fn shutdown_problems(raw: &str) -> Vec<String> {
    let raw = raw.replace('\r', "");
    let code = code_only(raw.split("#[cfg(test)]").next().unwrap_or(""));
    let mut problems = Vec::new();
    // Each name must come after the one before it, in the squeezed body of the function.
    let mut in_order = |function: &str, names: &[&str]| {
        let Some(body) = body_of(&code, function) else {
            problems.push(format!("{function} is missing"));
            return;
        };
        let body = squeezed(body);
        let mut after = 0;
        for name in names {
            match body[after..].find(name) {
                Some(at) => after += at + name.len(),
                None => problems.push(format!("{function}: `{name}` is missing or out of order")),
            }
        }
    };
    in_order(
        "fn tear_down(",
        &[
            "self.prove_before_release(control,&slots)",
            "self.release(&mutslot)",
            "self.settle_before_close(&slots)",
            "self.owner.close()",
            "self.settle_all(&slots)",
            "report.shutdown=Some(shutdown)",
        ],
    );
    in_order(
        "fn prove_before_release(",
        &[
            "ifself.present_fences{return;}",
            "decide::clamp_prove_window(self.config.shutdown_prove_window)",
            "decide::proof_deadline(started,window)",
            "self.begin_proof(control,&mutslot,deadline)",
            "decide::wait_for(window.saturating_sub(started.elapsed()),decide::PROVE_STEP,",
            "self.owner.poll()",
            "self.prove_step(&mutslot)",
            "self.end_proof(&mutslot,false)",
            "decide::proof_gave_up(waited.gave_up,self.deadline_ends.get()-ended_at_deadline)",
            "report.shutdown_prove_gave_up=gave_up",
        ],
    );
    // A proof that ends at its own deadline is counted before it is ended (F3).
    in_order(
        "fn prove_step(",
        &["Instant::now()>=until", "self.deadline_ends.set(self.deadline_ends.get().saturating_add(1))", "self.end_proof(slot,false)"],
    );
    in_order(
        "fn settle_before_close(",
        &[
            "decide::settle_window(self.report().closes_unproven)",
            "decide::wait_for(window,decide::PROVE_STEP,",
            "self.owner.poll()",
            "self.settle_all(slots)",
            "self.settle_ending()",
        ],
    );
    in_order(
        "fn close_window(",
        &["self.begin_proof(control,&mutslot,decide::proof_deadline(Instant::now(),decide::PROVE_WINDOW))"],
    );
    let tear_down = body_of(&code, "fn tear_down(").map(squeezed).unwrap_or_default();
    if tear_down.matches("self.owner.close()").count() != 1 {
        problems.push("tear_down closes the owner other than once".to_owned());
    }
    problems
}

#[test]
fn a_shutdown_proves_before_it_releases_settles_before_the_owner_closes_and_again_after() {
    let raw = include_str!("../src/host/ending.rs");
    assert_eq!(shutdown_problems(raw), Vec::<String>::new());
    // The scan has teeth: each of these is the real text with one thing changed, and each is refused.
    let call = "        self.prove_before_release(control, &slots);
";
    let changed = |from: &str, to: &str| {
        let raw = raw.replace('\r', "");
        assert_eq!(raw.matches(from).count(), 1, "the control's text is in the file once: {from}");
        raw.replace(from, to)
    };
    let controls = [
        // Named in a string literal, in a comment, only in a test module, or after the release.
        ("a string literal", changed(call, "        let _ = \"self.prove_before_release(control, &slots)\";
")),
        ("a comment", changed(call, "        // self.prove_before_release(control, &slots);
")),
        (
            "a test module",
            changed(call, "") + "#[cfg(test)]
mod t {
    fn f() {
        self.prove_before_release(control, &slots);
    }
}
",
        ),
        (
            "after the release",
            changed(call, "").replace(
                "        self.settle_before_close(&slots);
",
                &format!("{call}        self.settle_before_close(&slots);
"),
            ),
        ),
        // The guard, the sweep of proofs that did not come, the settling and the single close.
        (
            "no guard",
            changed("        if self.present_fences {
            return;
        }
        // The one clock", "        // The one clock"),
        ),
        ("no sweep", changed("self.end_proof(&mut slot, false);", "let _ = &slot;")),
        (
            "settling in a literal",
            changed("let waiting = self.settle_all(slots);", "let waiting = \"self.settle_all(slots)\".is_empty();"),
        ),
        (
            "two closes",
            changed("let shutdown = self.owner.close();", "let _ = self.owner.close();
        let shutdown = self.owner.close();"),
        ),
        ("no settle window", changed("decide::settle_window(self.report().closes_unproven)", "decide::SETTLE_WINDOW")),
        (
            "the closing window's own bound",
            changed("decide::proof_deadline(Instant::now(), decide::PROVE_WINDOW)", "decide::proof_deadline(Instant::now(), decide::PROVE_STEP)"),
        ),
        ("an unclamped shutdown window", changed("decide::clamp_prove_window(self.config.shutdown_prove_window)", "self.config.shutdown_prove_window")),
        ("a give-up that only reads the shared wait", changed("decide::proof_gave_up(waited.gave_up, self.deadline_ends.get() - ended_at_deadline)", "waited.gave_up")),
        ("a deadline end that is not counted", changed("self.deadline_ends.set(self.deadline_ends.get().saturating_add(1));", "")),
    ];
    for (what, text) in controls {
        assert!(!shutdown_problems(&text).is_empty(), "{what} is refused");
    }
}

#[test]
fn the_shutdown_order_scan_reads_code_not_the_tests_comments_or_strings() {
    // The helpers of the scan itself, on small fixtures.
    let code = code_only("fn a() { let x = \"fn b() { }\"; } // fn c() { }
fn b() { one(); }
");
    assert_eq!(body_of(&code, "fn b(").map(str::trim), Some("one();"));
    assert_eq!(body_of(&code, "fn c("), None);
    assert_eq!(squeezed(" a 
	 b "), "ab");
    let mut fixture = String::from("fn tear_down(c: &Control) {
    self.owner.close();
}
");
    assert!(shutdown_problems(&fixture).len() >= 4, "a text without the order is refused");
    fixture.push_str("#[cfg(test)]
mod t {}
");
    assert!(!shutdown_problems(&fixture).is_empty());
}
