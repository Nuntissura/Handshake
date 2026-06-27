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

// WP-KERNEL-012 MT-085 (D2 — internal_diagnostics, Tier 2 §5.8.2/§5.8.4): per-frame frame-time
// tracking + the typed `SlowFrame` event + the p50/p95 stats the Diagnostics Panel (MT-087) reads.
// The in-process degradation signal BELOW a full freeze (a stutter that is not yet the ~5s freeze
// Palmistry watches). Wired into the live frame loop in `crate::app::HandshakeApp::update` (after
// `self.ui(ctx)`, measuring its WORK time so the MT-084 idle keep-alive is NOT mis-flagged as slow).
pub mod frame_timing;
pub mod gpu_info;
// WP-KERNEL-012 MT-087 (D3 — internal_diagnostics, Tier 2 §5.8.4 in-app Diagnostics Panel +
// §10.12.5 three-tier model): the egui widget that PROJECTS the live internal_diagnostics state
// (heartbeat MT-084 + frame-time MT-085 + resource/GPU MT-086 + last-N events MT-082 + an honest
// Tier-3 Palmistry empty-state until MT-093). Hosted as a Settings section (settings_diagnostics_section.rs),
// NOT a worksurface pane (operator steer 2026-06-27). Pure projection — holds no own authority (§5.8.4).
pub mod panel;
pub mod panic_hook;
pub mod recorder;
pub mod resource_counters;

// MT-085 re-exports so the panel + the app can `use crate::diagnostics::{FrameTimer, FrameStats, ...}`.
pub use frame_timing::{
    FrameStats, FrameTimer, FRAME_RING_CAPACITY, SLOW_FRAME_EMIT_DEBOUNCE, SLOW_FRAME_THRESHOLD,
};

// WP-KERNEL-012 MT-086 (D2 — internal_diagnostics, Tier 2 §5.8.2 resource counters) re-exports so the
// panel + the app can `use crate::diagnostics::{ResourceSampler, ResourceSample, GpuInfo, ...}`.
pub use gpu_info::GpuInfo;
pub use resource_counters::{ResourceSample, ResourceSampler, SAMPLE_INTERVAL};

// WP-KERNEL-012 MT-087 (D3 — §5.8.4 in-app Diagnostics Panel) re-exports so the Settings section + the
// app can `use crate::diagnostics::{DiagnosticsPanel, DiagnosticsView, ...}` without reaching into the
// `panel` submodule path.
pub use panel::{
    DiagnosticsPanel, DiagnosticsView, DIAGNOSTICS_EVENTS_AUTHOR_ID, DIAGNOSTICS_FRAME_AUTHOR_ID,
    DIAGNOSTICS_HEARTBEAT_AUTHOR_ID, DIAGNOSTICS_PALMISTRY_AUTHOR_ID, DIAGNOSTICS_PANEL_AUTHOR_ID,
    DIAGNOSTICS_RESOURCE_AUTHOR_ID, PANEL_EVENT_WINDOW,
};

// Public re-exports so any module can `use crate::diagnostics::{record, record_with, ...}` without
// reaching into the `recorder` submodule path.
pub use recorder::{
    dropped_count, has_ring_writer, heartbeat, install, record, record_with, snapshot_last_n,
    DiagSession, DiagnosticsRecorder, BUFFER_CAP,
};

// MT-083 durable-local-crash-record panic hook (Tier 2 §5.8.2). `install_panic_hook` is called in
// `main()` before `eframe::run_native`; the process-start session id helpers let the MT-081 ring reuse
// the SAME id the crash file is named with so Palmistry (Tier 3) correlates the crash file to the ring.
pub use panic_hook::{
    default_crash_dir, install_panic_hook, process_session_id, set_process_session_id,
};
