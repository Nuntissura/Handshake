//! WP-KERNEL-012 MT-096 (G2 end-to-end capstone) — DETERMINISTIC, TEST-ONLY freeze/crash injection
//! seams (Master Spec v02.196 §5.8 + §6.13; AC-016-7 / RISK-016-3).
//!
//! # What this is (and what it is deliberately NOT)
//!
//! The capstone forces the §6.13 incident scenarios — FREEZE, CRASH, BACKEND-DOWN — against the REAL
//! binaries to prove the whole three-tier diagnostic system works as an integrated whole. This module
//! holds the harness-seam DOCUMENTATION for those injections. It is HARNESS material, never product
//! behavior:
//!
//! - It is gated `#[cfg(any(test, feature = "diag-test-seams"))]` (see `diagnostics::mod`), so it is
//!   compiled ONLY for the crate's own unit tests or when the `diag-test-seams` feature is explicitly
//!   enabled. It is NEVER compiled into a default/release build (AC-016-7). The `main()`/`HandshakeApp`
//!   production paths call NOTHING here.
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
//! # The CRASH injection (NO in-crate trigger — the orphan seam was REMOVED)
//!
//! MT-096 originally carried an `ENV_FORCE_CRASH` / `force_crash_abort()` (`std::process::abort()`)
//! trigger here, intended for a re-exec'd child in a live crash proof. That seam ended up an ORPHAN —
//! wired to no test anywhere — and the wave-2 remediation replaced it with a STRICTLY BETTER real-host
//! gate instead of wiring it: the palmistry-side live gate
//! (`palmistry/tests/test_end_to_end_live.rs::live_palmistry_captures_minidump_from_really_crashed_client`)
//! spawns a victim child that installs the REAL production crash client (minidumper client +
//! crash-handler, the exact `main.rs` shape) and then REALLY crashes with a genuine unhandled access
//! violation — a strictly more faithful §6.13.6 crash than `process::abort()` (which bypasses the SEH
//! unhandled-exception filter and can never exercise the CrashContext -> out-of-process-minidump path).
//! With the live gate owning crash injection end-to-end, an unreachable duplicate trigger in this crate
//! is pure risk surface (AC-016-7 wants the crash trigger UNREACHABLE in the shipped binary; no trigger
//! at all is the strongest form), so it was removed rather than kept "just in case".
//! [`CRASH_MODEL_DOC`] records this decision for a reviewer.

/// Reviewer-facing rationale for why the FREEZE scenario needs no production hook (AC-016-7): a freeze is
/// observable solely as a stalled heartbeat, which the capstone injects by stopping the egui step pump on
/// the REAL `HandshakeApp`. A greppable constant so the no-production-hook decision is auditable.
pub const FREEZE_MODEL_DOC: &str =
    "MT-096 freeze injection = stop the egui frame pump on the REAL HandshakeApp; the MT-084 heartbeat \
     stops advancing and a zero-cooperation DiagRingReader observes the stale heartbeat (\u{a7}6.13.4). \
     NO production hook in app.rs/main.rs is added (out of scope) and NONE is needed.";

/// Reviewer-facing rationale for why this crate carries NO crash trigger (AC-016-7): the live crash
/// injection is owned by the palmistry-side real-host gate, which really crashes a separate victim
/// process running the REAL production crash-client wiring (a genuine unhandled access violation ->
/// CrashContext -> out-of-process minidump, \u{a7}6.13.6). The former in-crate `ENV_FORCE_CRASH` abort
/// seam was an orphan (wired to no test) AND a weaker crash class (abort bypasses the SEH
/// unhandled-exception filter), so it was REMOVED — no trigger at all is the strongest AC-016-7 posture.
pub const CRASH_MODEL_DOC: &str =
    "MT-096 crash injection = the palmistry live gate's REALLY-crashed victim child (real unhandled \
     access violation through the real production crash client, \u{a7}6.13.6); handshake-native ships \
     NO crash trigger at all (the orphan ENV_FORCE_CRASH abort seam was removed \u{2014} AC-016-7).";

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
    fn crash_model_doc_names_the_live_gate_and_the_removed_orphan() {
        // The reviewer-facing rationale must name the §6.13.6 basis, the live-gate ownership, and the
        // removed orphan seam (so the removal decision stays auditable in code, not just in history).
        assert!(CRASH_MODEL_DOC.contains("6.13.6"));
        assert!(CRASH_MODEL_DOC.contains("ENV_FORCE_CRASH"));
        assert!(CRASH_MODEL_DOC.contains("NO crash trigger"));
    }
}
