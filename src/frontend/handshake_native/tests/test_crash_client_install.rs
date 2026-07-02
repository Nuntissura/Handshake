//! MT-092 CLIENT-INSTALL PROOFS (§6.13.6 — the Handshake/CLIENT side of out-of-process crash capture).
//!
//! handshake-native is the CLIENT in the Embark crash pipeline; Palmistry is the SERVER/writer. These
//! tests prove:
//!
//! - **AC-012-4 / PT-012-C**: the crash-handler exception handler is INSTALLED and, on an unhandled OS
//!   exception, its callback SIGNALS the minidumper client (which makes the server write the dump
//!   out-of-process). Proven by driving the REAL compiled `handshake-native` binary with the headless
//!   `--crash-client-selftest` flag: it stands up a minidumper server in-process, installs the crash
//!   handler pointed at it, fires a SIMULATED exception (the crash-handler test seam — does NOT crash the
//!   process), and reports a machine-readable JSON verdict that the callback ran AND a real minidump was
//!   captured out-of-process. The binary's items are not importable from `tests/`, so we drive the
//!   compiled binary (the same proof shape MT-089 uses).
//! - **AC-012-7**: the TWO death-mode coverages are documented in the source — the MT-083 Rust-panic hook
//!   vs THIS MT's unhandled OS exception / SEH handler — so neither death mode is silently uncovered.
//! - **AC-012-4 source companion**: the production `main()` actually installs the OS exception handler
//!   (the `install_crash_client()` call) AND holds it alive for the process lifetime.

use std::path::PathBuf;
use std::process::{Command, Stdio};

/// Resolve the built `handshake-native` binary (cargo sets CARGO_BIN_EXE for integration tests of a
/// binary crate). This is the REAL binary under test.
fn handshake_native_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_handshake-native"))
}

// ===================================================================================================
// AC-012-4 / PT-012-C — the crash-handler is installed + its callback signals the minidumper client,
// and a real minidump is written OUT-OF-PROCESS. Driven through the compiled binary's headless seam.
// ===================================================================================================

#[test]
fn crash_client_install_signals_minidumper_and_dumps_out_of_process() {
    let output = Command::new(handshake_native_bin())
        .arg("--crash-client-selftest")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("run handshake-native --crash-client-selftest");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let json: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap_or_else(|e| {
        panic!(
            "selftest must print a JSON verdict; got stdout={stdout:?} stderr={stderr:?} err={e}"
        )
    });

    // The selftest exits 0 ONLY when every stage held: the callback ran, the simulated exception was
    // handled, and a non-empty minidump was captured out-of-process.
    assert_eq!(
        output.status.code(),
        Some(0),
        "AC-012-4: the crash-client selftest must pass (json={json}, stderr={stderr})"
    );
    assert_eq!(
        json["ok"],
        serde_json::Value::Bool(true),
        "verdict ok must be true: {json}"
    );
    assert_eq!(
        json["callback_ran"],
        serde_json::Value::Bool(true),
        "AC-012-4: the crash-handler callback MUST have run on the (simulated) exception: {json}"
    );
    assert_eq!(
        json["sim_handled"],
        serde_json::Value::Bool(true),
        "AC-012-4: the callback must report it HANDLED the crash (it signalled the client): {json}"
    );
    assert_eq!(
        json["minidump_captured"],
        serde_json::Value::Bool(true),
        "AC-012-4: a minidump must have been written OUT-OF-PROCESS by the server: {json}"
    );
    assert!(
        json["dump_len"].as_u64().unwrap_or(0) > 1024,
        "the out-of-process minidump must be a non-trivial real dump: {json}"
    );
}

// ===================================================================================================
// AC-012-7 — the two death modes are documented (MT-083 Rust panic vs MT-092 unhandled OS exception).
// Source-scan the binary entrypoint for the explicit two-death-mode documentation so neither is silently
// uncovered.
// ===================================================================================================

#[test]
fn two_death_modes_are_documented_in_main() {
    let src = include_str!("../src/main.rs");
    let lower = src.to_lowercase();
    // The OS-exception / SEH death mode (this MT).
    assert!(
        lower.contains("seh") || lower.contains("os exception") || lower.contains("unhandled os"),
        "AC-012-7: main.rs must document the unhandled OS exception / SEH death mode (MT-092)"
    );
    // The complement to the MT-083 Rust panic hook.
    assert!(
        lower.contains("mt-083") && lower.contains("panic"),
        "AC-012-7: main.rs must document that this complements the MT-083 Rust-panic hook (both modes)"
    );
    // The out-of-process invariant is named (the crashing process does not dump itself).
    assert!(
        lower.contains("out-of-process") || lower.contains("out of process"),
        "AC-012-7/§6.13.6: main.rs must name the out-of-process dump invariant"
    );
}

// ===================================================================================================
// AC-012-4 source companion — production main() installs the OS exception handler and HOLDS it alive.
// ===================================================================================================

#[test]
fn main_installs_and_holds_the_crash_handler() {
    let src = include_str!("../src/main.rs");
    // The install is called from main() (not only in the selftest).
    assert!(
        src.contains("install_crash_client()"),
        "AC-012-4: main() must call install_crash_client() to install the OS exception handler"
    );
    // The returned handler is BOUND to a binding held for the process lifetime (dropping it would detach
    // the OS handler). The `_crash_handler` binding keeps it alive.
    assert!(
        src.contains("let _crash_handler = install_crash_client();"),
        "AC-012-4: the attached crash handler must be held alive for the process lifetime (a dropped \
         handler detaches the OS exception handler)"
    );
    // The handler attaches via the field-standard crash-handler crate (not a hand-rolled SEH filter).
    assert!(
        src.contains("crash_handler::CrashHandler::attach"),
        "the OS exception handler must be installed via the crash-handler crate's CrashHandler::attach"
    );
    // The callback requests an OUT-OF-PROCESS dump via the minidumper client (not a self-dump).
    assert!(
        src.contains("request_dump"),
        "the crash callback must call the minidumper client's request_dump (out-of-process), not \
         self-dump the crashing process"
    );
}

// ===================================================================================================
// MT-092/MT-094 remediation — the §6.13.6 CRASH-SOCKET RENDEZVOUS.
//
// The audited defect: HANDSHAKE_CRASH_SOCK was set NOWHERE, so `client=None` on every production run
// and the rich CrashContext -> out-of-process-minidump pipeline was DEAD. The fix derives the crash
// socket on BOTH sides from the SAME control-socket base name and arms the client AFTER the MT-094
// launcher brought Palmistry up. These tests pin the fix.
// ===================================================================================================

/// THE CROSS-CRATE WIRE TEST: the client-side derivation (`diagnostics::crash_socket_path`) and the
/// server-side derivation (`palmistry::crash_capture::crash_socket_path` — the REAL palmistry library,
/// a dev-dependency here) must be EQUAL for every control-socket shape the launcher can produce. A
/// drift on either side would silently leave the crash client unarmed (connect to a socket nothing
/// binds); this pin turns that silence into a test failure.
#[test]
fn crash_socket_derivation_is_pinned_equal_across_crates() {
    let cases = [
        // The production shape: control_socket_name(uuid-v4 session id).
        handshake_native::diagnostics::control_socket_name("3f2a9c44-6f6e-4d2b-9c1e-8f5a2b7c9d10"),
        // Simple names.
        "handshake-palmistry-abc".to_string(),
        "plain_token".to_string(),
        // Characters outside [A-Za-z0-9_-] must sanitize IDENTICALLY on both sides.
        "weird name/with:chars\\and.dots".to_string(),
        // Longer than the 40-char truncation budget: both sides must truncate identically.
        "x".repeat(120),
        format!("handshake-palmistry-{}", "y".repeat(100)),
        // Empty edge.
        String::new(),
    ];
    for control_socket in &cases {
        let client_side = handshake_native::diagnostics::crash_socket_path(control_socket);
        let server_side = palmistry::crash_capture::crash_socket_path(control_socket);
        assert_eq!(
            client_side, server_side,
            "the §6.13.6 crash-socket rendezvous derivation must be IDENTICAL on both sides for \
             control socket {control_socket:?} — a drift leaves the crash client silently unarmed"
        );
    }
}

/// The launcher-side half is actually WIRED: `launch_palmistry_at` derives the crash socket and the
/// handle carries it; `main()` arms the client AFTER the Palmistry launch (the late-connect step) — a
/// source-order scan mirroring the AC-014-1 proof shape (`main()` is a binary entrypoint whose items
/// are not importable from tests/).
#[test]
fn main_arms_the_crash_client_after_palmistry_launch() {
    let main_src = include_str!("../src/main.rs");
    let launcher_src = include_str!("../src/diagnostics/palmistry_launch.rs");

    // Strip full-line comments so the scan matches CODE, not the prose that names these APIs.
    let code_only = |src: &str| -> String {
        src.lines()
            .filter(|l| !l.trim_start().starts_with("//"))
            .collect::<Vec<_>>()
            .join("\n")
    };
    let main_code = code_only(main_src);
    let launcher_code = code_only(launcher_src);

    // The launcher derives the crash socket and the handle exposes it.
    assert!(
        launcher_code.contains("crash_socket_path(control_socket)"),
        "launch_palmistry_at must DERIVE the crash socket from the control socket (the launcher-side \
         half of the §6.13.6 rendezvous)"
    );
    assert!(
        launcher_code.contains("pub fn crash_socket(&self)"),
        "PalmistryHandle must expose the derived crash socket for the late-arm step"
    );

    // main() arms the client AFTER the launch (late-connect: the crash server must exist first).
    let launch_idx = main_code
        .find("launch_palmistry_or_degrade")
        .expect("main.rs must call the Palmistry launcher");
    let arm_idx = main_code
        .find("arm_crash_client_late(")
        .expect("main.rs must call arm_crash_client_late (the §6.13.6 late-connect step)");
    assert!(
        arm_idx > launch_idx,
        "the crash client must arm AFTER the Palmistry launch (the crash server binds the derived \
         socket during Palmistry startup; connecting before the launch can never succeed)"
    );
    // And the arm happens BEFORE the event loop starts (a crash during the session is covered).
    let run_native_idx = main_code
        .find("eframe::run_native")
        .expect("main.rs must call eframe::run_native");
    assert!(
        arm_idx < run_native_idx,
        "the crash client must be armed BEFORE eframe::run_native so the session is covered"
    );
}
