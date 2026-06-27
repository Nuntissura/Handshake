//! `internal_diagnostics` — Tier 2 of Handshake's three-tier diagnostic decoupling
//! (Master Spec v02.196 §5.8 internal_diagnostics + §5.8.2 "Open diagnostic-event API").
//!
//! This module owns the process-global diagnostics facade. Any feature in the running app — a backend
//! call site, a widget, a GUI action, a job bridge — calls [`record`] (or the ergonomic
//! [`record_with`]) and the typed [`DiagEvent`] is (a) appended to a bounded in-process ring the in-app
//! Diagnostics Panel (MT-087) reads via [`snapshot_last_n`], AND (b) written to the MT-081
//! shared-memory ring (when a writer has been [`install`]ed) so the external Palmistry watcher (Tier 3)
//! sees it with zero cooperation.
//!
//! # Tier placement
//!
//! - **Tier 1 (Flight Recorder)** — kept as-is; internal_diagnostics SUPPLEMENTS it (§5.8.6) and does
//!   NOT write to it here. Recovery-time forwarding of survived records into the Flight Recorder is
//!   MT-093, not this module.
//! - **Tier 2 (internal_diagnostics)** — THIS module. The open `record()` API + the in-process buffer.
//! - **Tier 3 (Palmistry)** — the external out-of-process watcher (MT-089+) that READS the MT-081 ring
//!   `record()` writes.
//!
//! # Wiring (no dead code)
//!
//! [`crate::app::HandshakeApp::new`] creates the MT-081 ring, calls [`install`] with the writer, then
//! records ONE live startup-marker event so a normal launch produces at least one real `DiagEvent` in
//! the ring + buffer with zero test scaffolding (AC-002-4 / the Spec-Realism anti-dead-code gate). The
//! fuller per-frame instrumentation (heartbeat, frame-time, resource counters, backend-unreachable) is
//! MT-084/005/006/007/008.
//!
//! # API design
//!
//! The public surface is FREE FUNCTIONS over a process-global ([`OnceLock`]), so any module can record
//! without threading a handle (§5.8.2 "any feature MAY call"). This is the standard Rust global-logger
//! / global-metrics pattern. There is deliberately NO public surface that accepts free text or a byte
//! blob: the API accepts only [`DiagEvent`] / the MT-081 typed enums, so the no-sensitive-data
//! allowlist (§5.8.3) is structural at the boundary (AC-002-5).
//!
//! [`OnceLock`]: std::sync::OnceLock

pub mod recorder;

// Public re-exports so any module can `use crate::diagnostics::{record, record_with, ...}` without
// reaching into the `recorder` submodule path.
pub use recorder::{
    dropped_count, has_ring_writer, install, record, record_with, snapshot_last_n, DiagSession,
    DiagnosticsRecorder, BUFFER_CAP,
};
