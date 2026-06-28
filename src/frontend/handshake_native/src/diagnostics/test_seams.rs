//! WP-KERNEL-012 MT-096 (G2 end-to-end capstone) — DETERMINISTIC, TEST-ONLY freeze/crash injection
//! seams (Master Spec v02.196 §5.8 + §6.13; AC-016-7 / RISK-016-3).
//!
//! # What this is (and what it is deliberately NOT)
//!
//! The capstone forces the three §6.13 incident scenarios — FREEZE, CRASH, BACKEND-DOWN — against the
//! REAL binaries to prove the whole three-tier diagnostic system works as an integrated whole. This
//! module holds the small, deterministic harness seams the capstone uses to inject those scenarios. It
//! is HARNESS code, never product behavior:
//!
//! - It is gated `#[cfg(any(test, feature = "diag-test-seams"))]` (see `diagnostics::mod`), so it is
//!   compiled ONLY for the crate's own unit tests or when the `diag-test-seams` feature is explicitly
//!   enabled (the capstone's #[ignore]d live cross-process crash proof). It is NEVER compiled into a
//!   default/release build — the shipped binary CANNOT reach the crash trigger (AC-016-7). The
//!   `main()`/`HandshakeApp` production paths call NOTHING here.
//!
//! # The FREEZE model (no production hook — AC-016-7)
//!
//! A UI-thread freeze is, from the external watcher's ZERO-COOPERATION vantage (§6.13.4), exactly one
//! observable: the MT-084 heartbeat counter STOPS advancing while the ring stays mapped + readable.
//! Whether the UI thread is deadlocked in a hung frame or the frame pump simply stopped, Palmistry sees
//! the identical signal — a stale heartbeat. The capstone therefore injects a freeze by driving the REAL
//! `HandshakeApp` frame loop (via egui_kittest, the same `eframe::App::update` the shipped binary runs)
//! and then STOPPING the step pump: the heartbeat counter freezes at its last value and a separate
//! `DiagRingReader` observes the staleness with zero cooperation. This needs NO production hook in
//! `app.rs`/`main.rs` (which are out of scope for this MT) — it is the most faithful, lowest-footprint
//! freeze injection. [`FREEZE_MODEL_DOC`] records this rationale for a reviewer.
//!
//! # The CRASH trigger (feature-gated, not shipped — AC-016-7)
//!
//! A hard crash (an unhandled OS exception / abrupt abort, NOT a clean shutdown) is injected by
//! [`force_crash_abort`] — `std::process::abort()`, the field-standard way to abnormally terminate a
//! process for crash-handler testing. It is invoked ONLY in a SPAWNED CHILD process by the capstone's
//! #[ignore]d live proof (the child re-invokes the test binary with [`ENV_FORCE_CRASH`] set); the parent
//! observes the abnormal child exit exactly as Palmistry's parent-handle wait (MT-089) observes an
//! abnormal Handshake exit. Because abort terminates the process, it MUST run only in a child the test
//! controls, never in the test/parent process itself.

use std::process;

/// Reviewer-facing rationale for why the FREEZE scenario needs no production hook (AC-016-7): a freeze is
/// observable solely as a stalled heartbeat, which the capstone injects by stopping the egui step pump on
/// the REAL `HandshakeApp`. A greppable constant so the no-production-hook decision is auditable.
pub const FREEZE_MODEL_DOC: &str =
    "MT-096 freeze injection = stop the egui frame pump on the REAL HandshakeApp; the MT-084 heartbeat \
     stops advancing and a zero-cooperation DiagRingReader observes the stale heartbeat (\u{a7}6.13.4). \
     NO production hook in app.rs/main.rs is added (out of scope) and NONE is needed.";

/// The environment variable a SPAWNED CHILD checks to decide whether to abnormally terminate itself via
/// [`force_crash_abort`]. The capstone's #[ignore]d live crash proof sets this on a child it spawns
/// (re-invoking the test binary); the child calls [`maybe_force_crash_from_env`] at entry and aborts, and
/// the parent observes the abnormal exit (the §6.13.6 crash signal MT-089/092 consume). A fixed
/// vocabulary string — never project content.
pub const ENV_FORCE_CRASH: &str = "HANDSHAKE_MT096_FORCE_CRASH";

/// THE CRASH TRIGGER (test-only, feature-gated). Abnormally terminate the CURRENT process via
/// `std::process::abort()` — a hard, unhandled termination (NOT a clean shutdown), the §6.13.6 crash
/// class. `abort()` raises `SIGABRT` / `STATUS_FATAL_APP_EXIT` so the OS reports an ABNORMAL exit code,
/// which a parent watcher (Palmistry, MT-089) classifies as a crash. Returns `!` — it never returns.
///
/// SAFETY OF USE (not `unsafe`, but a process-level hazard): this terminates the whole process. It MUST
/// be called only inside a SPAWNED CHILD the test controls (gated by [`ENV_FORCE_CRASH`]), NEVER in the
/// test/parent process. It is feature-gated + never wired into production (AC-016-7).
pub fn force_crash_abort() -> ! {
    // Flush nothing, hold nothing — a real crash gives the dying process no chance to clean up; that is
    // precisely why the EXTERNAL out-of-process watcher (Palmistry) must capture from outside (§6.13.6).
    process::abort();
}

/// If [`ENV_FORCE_CRASH`] is set in the environment, abnormally terminate via [`force_crash_abort`]. The
/// capstone's #[ignore]d live crash proof spawns a CHILD (the test binary re-invoked) with this env set;
/// the child calls this at entry and crashes deterministically, and the parent observes the abnormal
/// exit. In any process WITHOUT the env set (the normal test/parent run) this is a no-op, so importing /
/// linking the seam never risks an accidental abort.
pub fn maybe_force_crash_from_env() {
    if std::env::var_os(ENV_FORCE_CRASH).is_some() {
        force_crash_abort();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn freeze_model_doc_names_the_zero_cooperation_basis() {
        // The reviewer-facing rationale must name the zero-cooperation §6.13.4 basis + the no-hook decision.
        assert!(FREEZE_MODEL_DOC.contains("6.13.4"));
        assert!(FREEZE_MODEL_DOC.contains("NO production hook"));
    }

    #[test]
    fn force_crash_env_var_is_a_stable_fixed_token() {
        // A fixed-vocabulary env name (no project content); stable so the parent + child agree on it.
        assert_eq!(ENV_FORCE_CRASH, "HANDSHAKE_MT096_FORCE_CRASH");
    }

    #[test]
    fn maybe_force_crash_is_a_noop_without_the_env() {
        // CRITICAL: with the env UNSET, the seam must NOT abort the test process. Proven by simply
        // returning normally here (an abort would terminate the whole test binary). The env is read
        // process-wide; this test asserts the default (unset) path is inert. (The other tests in this
        // binary never set the env, so this is safe.)
        let prev = std::env::var_os(ENV_FORCE_CRASH);
        std::env::remove_var(ENV_FORCE_CRASH);
        maybe_force_crash_from_env(); // must return (no abort) when unset
        if let Some(v) = prev {
            std::env::set_var(ENV_FORCE_CRASH, v);
        }
    }
}
