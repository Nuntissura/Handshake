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
        panic!("selftest must print a JSON verdict; got stdout={stdout:?} stderr={stderr:?} err={e}")
    });

    // The selftest exits 0 ONLY when every stage held: the callback ran, the simulated exception was
    // handled, and a non-empty minidump was captured out-of-process.
    assert_eq!(
        output.status.code(),
        Some(0),
        "AC-012-4: the crash-client selftest must pass (json={json}, stderr={stderr})"
    );
    assert_eq!(json["ok"], serde_json::Value::Bool(true), "verdict ok must be true: {json}");
    assert_eq!(
        json["callback_ran"], serde_json::Value::Bool(true),
        "AC-012-4: the crash-handler callback MUST have run on the (simulated) exception: {json}"
    );
    assert_eq!(
        json["sim_handled"], serde_json::Value::Bool(true),
        "AC-012-4: the callback must report it HANDLED the crash (it signalled the client): {json}"
    );
    assert_eq!(
        json["minidump_captured"], serde_json::Value::Bool(true),
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
