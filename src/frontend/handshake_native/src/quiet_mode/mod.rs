//! WP-KERNEL-011 MT-030 — quiet-operation (HBR-QUIET) subsystem.
//!
//! Holds the focus-audit / quiet-operation guard that anchors the HBR-QUIET guarantee: the native
//! shell never steals OS focus, foregrounds a window, or hijacks keyboard input during automated or
//! background operation. See [`focus_guard`] for the contract and the `ForegroundExceptionToken`
//! choke point; the enforcing source audit lives in `tests/test_focus_audit_quiet.rs`.

pub mod focus_guard;
