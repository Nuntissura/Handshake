//! WP-KERNEL-011 MT-030 — Focus-audit / quiet-operation guard (HBR-QUIET; GLOBAL-BUILD-046..054).
//!
//! ## What this guards
//!
//! HBR-QUIET requires that the native shell NEVER steals OS focus, NEVER pops a window to the
//! foreground, and NEVER hijacks keyboard input during model-driven or background operation. This
//! module is the **runtime/compile-time anchor** for that guarantee; the **enforcement** lives in the
//! companion source-audit test `tests/test_focus_audit_quiet.rs`, which scans `src/` for any
//! focus-stealing Win32 API and fails the build if one appears outside the focus-safe allow-list.
//!
//! ## Why a source-audit (logical) guard, not a live Win32-hook spawn
//!
//! The shell is proven focus-safe **by construction**, so the strongest, most deterministic proof is
//! a static guarantee, not a live observation:
//!
//! - The native shell calls NO Win32 foreground/input-injection API at all. The only Win32 surface is
//!   the screenshot capture (`mcp::screenshot`), which uses `PrintWindow(PW_RENDERFULLCONTENT)` /
//!   `BitBlt` over an **offscreen** memory DC — it changes no Z-order and never activates a window
//!   (focus-safe by construction; the documented allow-list item).
//! - Detached pop-out windows are created with `egui::ViewportBuilder::with_active(false)`
//!   (`popout_window.rs`), so the OS does not raise them to the foreground or move focus on creation.
//! - The MCP steering channel's `focus` action maps to `accesskit::Action::Focus` — **in-app widget
//!   keyboard focus** for AccessKit navigation — never an OS-window foreground call.
//!
//! A live hook (WINEVENT_SYSTEM_FOREGROUND / WH_KEYBOARD_LL) is GPU- and on-screen-window-bound: it
//! requires a real visible window and a windowing session, which the headless `cargo test` host does
//! not provide, and the test would either silently pass on "no events" (a false PASS) or be skipped.
//! A source-scan that FAILS the build the moment a banned API is introduced is a stronger, gameable-
//! resistant invariant for the default test suite. This guard is GPU-free and never renders.
//!
//! ## The `ForegroundExceptionToken`
//!
//! If a future feature ever legitimately needs to foreground a window (e.g. an explicit operator
//! "bring Handshake to front" command), it must construct a [`ForegroundExceptionToken`] via
//! [`request_foreground_exception`] with a stated reason. The token is the single auditable choke
//! point: the source-audit allow-list keys on it, so foregrounding without a token fails the audit.

/// The set of Win32 APIs that steal OS focus, foreground a window, or inject input. Introducing a
/// call to any of these into `src/frontend/**` is an HBR-QUIET violation unless it is gated behind a
/// [`ForegroundExceptionToken`] and recorded in the audit allow-list.
///
/// This list is the single source of truth shared by this module's doc-contract and the source-audit
/// test (`tests/test_focus_audit_quiet.rs`), so the banned set cannot drift between the two.
pub const BANNED_FOCUS_APIS: &[&str] = &[
    "SetForegroundWindow",
    "BringWindowToTop",
    "SetActiveWindow",
    "SwitchToThisWindow",
    "AllowSetForegroundWindow",
    "keybd_event",
    "mouse_event",
    "SendInput",
];

/// A capability token proving that a foreground/focus activation was **explicitly requested** (e.g.
/// by an operator command), not silently performed during automated or background operation.
///
/// The only constructor is [`request_foreground_exception`], so the token cannot be forged elsewhere;
/// the source-audit allow-list keys on its presence. It carries the human-stated reason it was
/// granted, giving any future foreground call site an audit trail.
#[derive(Debug)]
pub struct ForegroundExceptionToken {
    reason: &'static str,
}

impl ForegroundExceptionToken {
    /// The human-stated reason the foreground exception was granted (audit trail).
    pub fn reason(&self) -> &'static str {
        self.reason
    }
}

/// Request an explicit foreground/focus-activation exception, recording the human reason.
///
/// This is the ONLY way to obtain a [`ForegroundExceptionToken`]. Any code path that foregrounds a
/// window or moves OS focus MUST hold a token obtained here; the source-audit (`test_focus_audit_quiet`)
/// allow-lists Win32 focus calls only when they are reached through a token. There is currently no
/// such call site in the shell — the shell is focus-safe by construction — so this exists as the
/// auditable choke point for any future operator-driven foreground feature.
pub fn request_foreground_exception(reason: &'static str) -> ForegroundExceptionToken {
    ForegroundExceptionToken { reason }
}

/// Install / assert the quiet-mode guarantee at startup.
///
/// Called from `main.rs` before the event loop. In **debug** builds it logs that the quiet-mode
/// invariant is active (the enforced invariant is the compile-time/source-audit ban — there is no
/// runtime focus call to intercept because the shell never makes one). In **release** builds it is a
/// no-op; the source-audit test is the binding gate.
///
/// Returns the count of APIs in the banned set so a caller (or a smoke test) can assert the guard was
/// wired with a non-empty ban list rather than a tautological no-op.
pub fn assert_quiet_mode_installed() -> usize {
    #[cfg(debug_assertions)]
    {
        tracing::debug!(
            banned_focus_apis = BANNED_FOCUS_APIS.len(),
            "HBR-QUIET: quiet-mode guard active; shell makes no Win32 foreground/input-injection call \
             (enforced by tests/test_focus_audit_quiet.rs source audit)"
        );
    }
    BANNED_FOCUS_APIS.len()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn banned_set_covers_the_focus_and_input_apis() {
        // Regression guard: the audit's ban list must include the foreground-steal trio AND the
        // keyboard/input-injection APIs. If someone trims this, the source audit weakens silently.
        for must in [
            "SetForegroundWindow",
            "BringWindowToTop",
            "SetActiveWindow",
            "keybd_event",
            "SendInput",
        ] {
            assert!(
                BANNED_FOCUS_APIS.contains(&must),
                "BANNED_FOCUS_APIS is missing {must} — HBR-QUIET audit would not catch it",
            );
        }
    }

    #[test]
    fn assert_quiet_mode_installed_reports_non_empty_ban_list() {
        assert_eq!(assert_quiet_mode_installed(), BANNED_FOCUS_APIS.len());
        assert!(assert_quiet_mode_installed() > 0, "ban list must be non-empty");
    }

    #[test]
    fn foreground_exception_carries_reason() {
        let tok = request_foreground_exception("operator: bring-to-front command");
        assert_eq!(tok.reason(), "operator: bring-to-front command");
    }
}
