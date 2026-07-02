// WP-KERNEL-011 MT-030 — Focus-audit / quiet-operation guard (C8 proof; HBR-QUIET;
// GLOBAL-BUILD-046..054).
//
// This is the BINDING enforcement of the HBR-QUIET guarantee for the native egui shell: the app must
// NEVER steal OS focus, pop a foreground window, or hijack keyboard input during model-driven or
// background operation. It proves that guarantee by STATIC SOURCE AUDIT, not by spawning the app:
//
//   (a) BAN AUDIT — scans every `src/**/*.rs` (comments and string/char literals stripped, so a doc
//       comment that NAMES a banned API does not false-positive) for any call to a focus-stealing /
//       input-injecting Win32 API. Any real call outside the documented focus-safe allow-list is a
//       HARD FAIL.
//   (b) POSITIVE INVARIANTS — asserts the pop-out viewports use `with_active(false)` (no focus theft
//       on creation), that NO viewport requests `with_active(true)` / `with_focused(true)`, and that
//       the only Win32 surface (the screenshot capture) uses focus-safe `PrintWindow`/`BitBlt` over
//       an offscreen DC.
//   (c) ALLOW-LIST PROOF — the one disclosed allow-list item (MT-027 screenshot: PrintWindow /
//       PW_RENDERFULLCONTENT / BitBlt over an offscreen memory DC, focus-safe by construction) is
//       present and uses no banned API.
//
// WHY SOURCE-AUDIT, NOT A LIVE WINDOWS HOOK: a live WINEVENT_SYSTEM_FOREGROUND / WH_KEYBOARD_LL hook
// needs a real on-screen window and a windowing session, which the headless `cargo test` host does
// not provide. Such a hook would silently pass on "no events seen" — a FALSE PASS. A source audit
// that FAILS THE BUILD the moment a banned API is introduced is a stronger, gaming-resistant
// invariant and is GPU-FREE (never calls Harness::render, which crashes this headless host). This
// matches the prompt's GPU-FREE / DEFAULT-SUITE-GREEN mandate and the contract's HBR-QUIET intent.
//
// DEVIATIONS (recorded for the reviewer):
//   * PATH: the MT-030 contract names tests/native_gui/focus_audit_quiet.rs + keyboard_steal_audit.rs.
//     cargo derives an integration-test target name from a file directly in the crate `tests/` dir; a
//     tests/native_gui/ subdir does NOT register a `cargo test --test ...` target (same decision
//     documented in test_theme.rs / test_single_binary.rs). The two audits are combined into this one
//     `test_focus_audit_quiet` target.
//   * APPROACH: the contract body assumed spawning `cargo run --bin handshake-native`, installing real
//     Win32 hooks, and depending on `handshake_core` (src/backend, a forbidden_path here). The
//     governing prompt overrides that with a GPU-free source-scan / logical-assertion guard. No
//     backend dependency is added; no app process is spawned; nothing renders.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

/// The crate `src/` directory (the product source the audit governs).
fn src_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src")
}

/// Where the proof report is written: the external artifact root only (CX-212E).
fn artifacts_dir() -> PathBuf {
    if let Ok(root) = std::env::var("HANDSHAKE_ARTIFACTS_ROOT") {
        if !root.trim().is_empty() {
            return PathBuf::from(root)
                .join("handshake-test")
                .join("native_gui");
        }
    }
    if let Ok(dir) = std::env::var("HANDSHAKE_PROOF_ARTIFACT_DIR") {
        if !dir.trim().is_empty() {
            return PathBuf::from(dir);
        }
    }
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../../Handshake_Artifacts/handshake-test/native_gui")
}

/// Win32 APIs that steal OS focus, foreground a window, or inject input. A real call to any of these
/// from `src/frontend/**` is an HBR-QUIET violation unless gated behind a `ForegroundExceptionToken`
/// (there is no such call site today — the shell is focus-safe by construction). Kept in sync, by an
/// equality assertion below, with `quiet_mode::focus_guard::BANNED_FOCUS_APIS`.
const BANNED_FOCUS_APIS: &[&str] = &[
    "SetForegroundWindow",
    "BringWindowToTop",
    "SetActiveWindow",
    "SwitchToThisWindow",
    "AllowSetForegroundWindow",
    "keybd_event",
    "mouse_event",
    "SendInput",
];

/// One recorded violation: file + 1-based line + the banned API found in executable code.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct Violation {
    file: String,
    line: usize,
    api: String,
}

/// Strip Rust line-comments (`// ...`, including `///` / `//!` doc comments), block comments
/// (`/* ... */`, including `/** */` / `/*! */`), and string / char literal CONTENT from one logical
/// view of the source, so the ban audit only sees executable code. Returns the source with comment
/// and literal bytes replaced by spaces (line structure preserved for accurate line numbers).
///
/// This is deliberately a conservative lexer: it understands `//`, `/* */` (non-nested, which is what
/// Rust uses outside of nested block comments — we treat the first `*/` as the end, which is safe for
/// this audit because a banned API name inside a comment is blanked either way), `"..."` with `\"`
/// escapes, raw strings `r#"..."#`, and `'.'` char literals. It is not a full Rust parser, but it is
/// sufficient to prevent the doc-comment / string false-positives that matter here, and it never
/// blanks executable identifiers.
fn strip_comments_and_literals(src: &str) -> String {
    let bytes = src.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        let b = bytes[i];
        let next = bytes.get(i + 1).copied();

        // Line comment: // ... to end of line.
        if b == b'/' && next == Some(b'/') {
            while i < bytes.len() && bytes[i] != b'\n' {
                out.push(if bytes[i] == b'\n' { b'\n' } else { b' ' });
                i += 1;
            }
            continue;
        }
        // Block comment: /* ... */ (treat first */ as end).
        if b == b'/' && next == Some(b'*') {
            out.push(b' ');
            out.push(b' ');
            i += 2;
            while i < bytes.len() {
                if bytes[i] == b'*' && bytes.get(i + 1).copied() == Some(b'/') {
                    out.push(b' ');
                    out.push(b' ');
                    i += 2;
                    break;
                }
                out.push(if bytes[i] == b'\n' { b'\n' } else { b' ' });
                i += 1;
            }
            continue;
        }
        // Raw string: r"..." or r#"..."# / r##"..."## ...
        if b == b'r' && (next == Some(b'"') || next == Some(b'#')) {
            let mut j = i + 1;
            let mut hashes = 0;
            while j < bytes.len() && bytes[j] == b'#' {
                hashes += 1;
                j += 1;
            }
            if j < bytes.len() && bytes[j] == b'"' {
                // It is a raw string. Copy `r`, the hashes and the opening quote as-is (they are not
                // identifiers we audit), then blank content until the matching `"###...`.
                out.push(b'r');
                out.extend(std::iter::repeat_n(b'#', hashes));
                out.push(b'"');
                j += 1; // past opening quote
                loop {
                    if j >= bytes.len() {
                        break;
                    }
                    if bytes[j] == b'"' {
                        // check for `hashes` following '#'
                        let mut k = j + 1;
                        let mut cnt = 0;
                        while k < bytes.len() && bytes[k] == b'#' && cnt < hashes {
                            cnt += 1;
                            k += 1;
                        }
                        if cnt == hashes {
                            out.push(b'"');
                            out.extend(std::iter::repeat_n(b'#', hashes));
                            j = k;
                            break;
                        }
                    }
                    out.push(if bytes[j] == b'\n' { b'\n' } else { b' ' });
                    j += 1;
                }
                i = j;
                continue;
            }
            // Not a raw string (just an identifier starting with `r`); fall through.
        }
        // Normal string literal: "..." with \" escapes.
        if b == b'"' {
            out.push(b'"');
            i += 1;
            while i < bytes.len() {
                if bytes[i] == b'\\' {
                    out.push(b' ');
                    if i + 1 < bytes.len() {
                        out.push(if bytes[i + 1] == b'\n' { b'\n' } else { b' ' });
                    }
                    i += 2;
                    continue;
                }
                if bytes[i] == b'"' {
                    out.push(b'"');
                    i += 1;
                    break;
                }
                out.push(if bytes[i] == b'\n' { b'\n' } else { b' ' });
                i += 1;
            }
            continue;
        }
        // Char literal: '.' or '\n' etc. Only treat as a literal when it looks like one (avoids
        // mangling lifetimes like 'a — but a lifetime never contains a banned API name anyway).
        if b == b'\'' {
            // Lookahead: `'x'` or `'\x'`.
            let is_escaped = next == Some(b'\\');
            let close_at = if is_escaped { i + 3 } else { i + 2 };
            if bytes.get(close_at).copied() == Some(b'\'') {
                out.push(b'\'');
                out.extend(std::iter::repeat_n(b' ', close_at - i - 1));
                out.push(b'\'');
                i = close_at + 1;
                continue;
            }
            // Otherwise it is a lifetime tick; copy it.
        }

        out.push(b);
        i += 1;
    }
    String::from_utf8(out).expect("stripping preserves valid utf-8 (ascii replacements only)")
}

/// Recursively collect every `.rs` file under `dir`.
fn collect_rs_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let entries =
        std::fs::read_dir(dir).unwrap_or_else(|e| panic!("read_dir {} failed: {e}", dir.display()));
    for entry in entries {
        let entry = entry.expect("dir entry");
        let path = entry.path();
        if path.is_dir() {
            collect_rs_files(&path, out);
        } else if path.extension().and_then(|s| s.to_str()) == Some("rs") {
            out.push(path);
        }
    }
}

/// Scan all product source for real (non-comment, non-string) calls to any banned focus API.
fn scan_for_banned_apis() -> (Vec<Violation>, usize) {
    let mut files = Vec::new();
    collect_rs_files(&src_dir(), &mut files);
    files.sort();
    let scanned = files.len();

    let mut violations = Vec::new();
    for file in &files {
        let raw = std::fs::read_to_string(file)
            .unwrap_or_else(|e| panic!("read {} failed: {e}", file.display()));
        let code = strip_comments_and_literals(&raw);
        let rel = file
            .strip_prefix(PathBuf::from(env!("CARGO_MANIFEST_DIR")))
            .unwrap_or(file)
            .to_string_lossy()
            .replace('\\', "/");
        for (idx, line) in code.lines().enumerate() {
            for api in BANNED_FOCUS_APIS {
                // Match the API as a call token: the API name followed (allowing whitespace) by `(`,
                // OR used as a path segment. We require the bare identifier to appear; because
                // comments/strings are blanked, any hit is executable code referencing the symbol.
                if line_has_identifier(line, api) {
                    violations.push(Violation {
                        file: rel.clone(),
                        line: idx + 1,
                        api: (*api).to_string(),
                    });
                }
            }
        }
    }
    violations.sort();
    violations.dedup();
    (violations, scanned)
}

/// True if `line` contains `ident` as a standalone identifier (not as a substring of a longer
/// identifier). Word boundary = the surrounding char is not an identifier char (`[A-Za-z0-9_]`).
fn line_has_identifier(line: &str, ident: &str) -> bool {
    let bytes = line.as_bytes();
    let pat = ident.as_bytes();
    if pat.is_empty() {
        return false;
    }
    let mut i = 0;
    while i + pat.len() <= bytes.len() {
        if &bytes[i..i + pat.len()] == pat {
            let before_ok = i == 0 || !is_ident_byte(bytes[i - 1]);
            let after_idx = i + pat.len();
            let after_ok = after_idx >= bytes.len() || !is_ident_byte(bytes[after_idx]);
            if before_ok && after_ok {
                return true;
            }
        }
        i += 1;
    }
    false
}

fn is_ident_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

fn write_report(report: &serde_json::Value) {
    let dir = artifacts_dir();
    std::fs::create_dir_all(&dir)
        .unwrap_or_else(|e| panic!("create artifacts dir {} failed: {e}", dir.display()));
    let path = dir.join("focus_audit_quiet_report.json");
    let body = serde_json::to_string_pretty(report).expect("serialize report");
    std::fs::write(&path, body)
        .unwrap_or_else(|e| panic!("write report {} failed: {e}", path.display()));
    eprintln!("focus audit report written to {}", path.display());
}

/// CORE PROOF (AC-030-01 / AC-030-06 / AC-030-07 / CTRL-030-01): no banned focus/input-injection API
/// is called anywhere in the product source. Writes the report JSON with the empty
/// `handshake_owned_events` (= violations) array.
#[test]
fn no_focus_stealing_or_input_injection_api_in_source() {
    let (violations, scanned) = scan_for_banned_apis();

    let report = serde_json::json!({
        "run_id": format!("focus-audit-{}", std::process::id()),
        "audit_status": "audited",
        "audit_method": "static_source_scan_comments_and_literals_stripped",
        // Field name mirrors FocusAuditReport.handshake_owned_events (the foreground steals attributed
        // to the app). For a source audit, a "steal" is a banned-API call site in app code.
        "handshake_owned_events": violations
            .iter()
            .map(|v| serde_json::json!({"file": v.file, "line": v.line, "api": v.api}))
            .collect::<Vec<_>>(),
        "total_foreground_events": violations.len(),
        "files_scanned": scanned,
        "banned_apis": BANNED_FOCUS_APIS,
        "allow_list": [
            "mcp/screenshot.rs: PrintWindow(PW_RENDERFULLCONTENT)/BitBlt over an offscreen memory DC — focus-safe by construction (no Z-order/activation change)"
        ],
    });
    write_report(&report);

    assert!(
        scanned > 0,
        "scanned 0 source files — the audit walked an empty tree (false-pass guard)"
    );
    assert!(
        violations.is_empty(),
        "HBR-QUIET VIOLATION: focus-stealing / input-injection Win32 API called in product source \
         (not behind a ForegroundExceptionToken):\n{}",
        violations
            .iter()
            .map(|v| format!("  {}:{} -> {}", v.file, v.line, v.api))
            .collect::<Vec<_>>()
            .join("\n"),
    );

    println!(
        "PASS: scanned {scanned} source files; zero focus-stealing/input-injection API calls \
         ({} banned APIs checked)",
        BANNED_FOCUS_APIS.len()
    );
}

/// AC-030-05 / consistency: the test's ban list and the product guard's ban list must be identical, so
/// the audit cannot drift away from the runtime guard's documented contract.
#[test]
fn ban_list_matches_product_guard() {
    use handshake_native::quiet_mode::focus_guard::BANNED_FOCUS_APIS as GUARD;
    let test_set: BTreeSet<&str> = BANNED_FOCUS_APIS.iter().copied().collect();
    let guard_set: BTreeSet<&str> = GUARD.iter().copied().collect();
    assert_eq!(
        test_set, guard_set,
        "the source-audit ban list and quiet_mode::focus_guard::BANNED_FOCUS_APIS have drifted apart",
    );
}

/// POSITIVE INVARIANT (HBR-QUIET): the detached pop-out viewports must be created with
/// `with_active(false)` (no focus theft on creation) and NO viewport may request
/// `with_active(true)` / `with_focused(true)`.
#[test]
fn popout_viewports_do_not_steal_focus() {
    let popout = src_dir().join("popout_window.rs");
    let raw = std::fs::read_to_string(&popout)
        .unwrap_or_else(|e| panic!("read {} failed: {e}", popout.display()));
    let code = strip_comments_and_literals(&raw);

    assert!(
        code.contains("with_active(false)"),
        "popout_window.rs no longer pins with_active(false) — detached windows could steal focus",
    );

    // No viewport anywhere may force activation/focus.
    let mut files = Vec::new();
    collect_rs_files(&src_dir(), &mut files);
    for file in &files {
        let raw = std::fs::read_to_string(file).unwrap();
        let code = strip_comments_and_literals(&raw);
        for forbidden in ["with_active(true)", "with_focused(true)"] {
            assert!(
                !code.contains(forbidden),
                "{} requests {forbidden} — HBR-QUIET forbids forcing window activation/focus",
                file.display(),
            );
        }
    }
    println!(
        "PASS: pop-out viewports use with_active(false); no with_active(true)/with_focused(true)"
    );
}

/// ALLOW-LIST PROOF (AC-030-04 intent): the ONE Win32 surface in the shell (the screenshot capture)
/// is the documented focus-safe allow-list item — it uses PrintWindow/BitBlt and references NO banned
/// API in executable code.
#[test]
fn screenshot_capture_is_focus_safe_allow_list_item() {
    let shot = src_dir().join("mcp").join("screenshot.rs");
    let raw = std::fs::read_to_string(&shot)
        .unwrap_or_else(|e| panic!("read {} failed: {e}", shot.display()));
    let code = strip_comments_and_literals(&raw);

    assert!(
        code.contains("PrintWindow"),
        "screenshot.rs no longer uses PrintWindow — verify the capture is still focus-safe",
    );
    for api in BANNED_FOCUS_APIS {
        assert!(
            !line_any_has_identifier(&code, api),
            "screenshot.rs (the focus-safe allow-list item) now calls banned API {api}",
        );
    }
    println!("PASS: screenshot capture uses focus-safe PrintWindow/BitBlt; no banned API in code");
}

fn line_any_has_identifier(code: &str, ident: &str) -> bool {
    code.lines().any(|l| line_has_identifier(l, ident))
}

// --- unit tests for the lexer itself (so the audit's own filtering is trustworthy) ---

#[test]
fn lexer_blanks_comment_and_string_mentions_but_keeps_code() {
    let src = r#"
// SetForegroundWindow in a line comment
/* BringWindowToTop in a block comment */
let s = "SetActiveWindow in a string";
fn real() { actually_call(); }
"#;
    let code = strip_comments_and_literals(src);
    assert!(!line_any_has_identifier(&code, "SetForegroundWindow"));
    assert!(!line_any_has_identifier(&code, "BringWindowToTop"));
    assert!(!line_any_has_identifier(&code, "SetActiveWindow"));
    assert!(line_any_has_identifier(&code, "actually_call"));
}

#[test]
fn lexer_keeps_real_call_outside_comment() {
    let src = "SetForegroundWindow(hwnd);";
    let code = strip_comments_and_literals(src);
    assert!(
        line_any_has_identifier(&code, "SetForegroundWindow"),
        "a real banned call must survive stripping so the audit catches it",
    );
}

#[test]
fn identifier_matcher_respects_word_boundaries() {
    assert!(line_has_identifier("foo SendInput(x)", "SendInput"));
    assert!(!line_has_identifier("MySendInputWrapper()", "SendInput"));
    assert!(!line_has_identifier("SendInputs(x)", "SendInput"));
}
