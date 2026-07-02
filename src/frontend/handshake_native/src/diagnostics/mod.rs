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
// WP-KERNEL-012 MT-105 (D2 — generalized operation/subprocess stall watchdog): per-operation
// deadline/progress-tick liveness monitoring. Emits the typed StalledOperation event through the
// MT-082 recorder on a dedicated poll thread; register/tick/complete never emit or block on work.
pub mod operation_watchdog;
// WP-KERNEL-012 MT-087 (D3 — internal_diagnostics, Tier 2 §5.8.4 in-app Diagnostics Panel +
// §10.12.5 three-tier model): the egui widget that PROJECTS the live internal_diagnostics state
// (heartbeat MT-084 + frame-time MT-085 + resource/GPU MT-086 + last-N events MT-082 + an honest
// Tier-3 Palmistry empty-state until MT-093). Hosted as a Settings section (settings_diagnostics_section.rs),
// NOT a worksurface pane (operator steer 2026-06-27). Pure projection — holds no own authority (§5.8.4).
pub mod panel;
pub mod panic_hook;
// WP-KERNEL-012 MT-094 (§6.13.3 — Palmistry launched WITH Handshake at startup): the Handshake-side
// LAUNCHER for the external Tier-3 watcher. Spawns the sibling `palmistry` process quietly (HBR-QUIET,
// CREATE_NO_WINDOW), completes the bounded startup IPC handshake over the MT-089 control socket, sends
// Shutdown on a clean exit, preserves the not-kill-on-job-close survives-parent-death inversion, and
// degrades gracefully (the watcher is supplementary — never blocks/crashes startup). Wired into `main()`
// BEFORE `eframe::run_native` (not `HandshakeApp::new`) so the kittest suite never spawns a palmistry child.
pub mod palmistry_launch;
pub mod recorder;
pub mod resource_counters;
// WP-KERNEL-012 MT-093 (§6.13.7 + §10.12.5): the Handshake-side READ seam for the freeze/crash records the
// external Palmistry watcher persisted to its durable survivor store. Feeds the Diagnostics Panel (MT-087)
// Tier-3 section so the §10.12.5 Tier-3 surface becomes POPULATED post-recovery (AC-013-6) instead of the
// honest empty-state MT-087 left. Reuse-via-FILE (the cross-process durable store) — handshake-native and
// the `palmistry` crate share no dependency edge; the FR is kept as-is.
pub mod survivor_forward;
// WP-KERNEL-012 MT-096 (G2 end-to-end capstone): TEST-ONLY freeze/crash injection seams. Gated behind
// `cfg(test)` (the crate's own unit tests) OR the `diag-test-seams` feature (the capstone's #[ignore]d
// live cross-process crash proof, which links the lib with that feature). It is NEVER compiled into a
// default/release build — the shipped binary cannot reach the crash trigger (AC-016-7). The whole module
// is a thin, deterministic harness seam, NOT product behavior; the production freeze/crash paths
// (heartbeat MT-084, panic hook MT-083, Palmistry MT-089+) are unchanged.
#[cfg(any(test, feature = "diag-test-seams"))]
pub mod test_seams;
pub use frame_timing::{
    FrameStats, FrameTimer, FRAME_RING_CAPACITY, SLOW_FRAME_EMIT_DEBOUNCE, SLOW_FRAME_THRESHOLD,
};

// WP-KERNEL-012 MT-086 (D2 — internal_diagnostics, Tier 2 §5.8.2 resource counters) re-exports so the
// panel + the app can `use crate::diagnostics::{ResourceSampler, ResourceSample, GpuInfo, ...}`.
pub use gpu_info::GpuInfo;
pub use operation_watchdog::{
    active_stalled_operation_count, global_operation_watchdog, recent_stalled_operation_count,
    start_global_operation_watchdog, OperationCode, OperationHandle, OperationWatchdog,
    OperationWatchdogThread, StalledOperationReport, BACKEND_OPERATION_STALL_DEADLINE,
    OPERATION_WATCHDOG_POLL_INTERVAL,
};
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

// WP-KERNEL-012 MT-093 (§6.13.7 / §10.12.5 Tier-3) re-exports so the shell + the panel can
// `use crate::diagnostics::{PalmistrySurvivorView, read_default_survivor_records, ...}` to feed the
// Tier-3 panel section with the forwarded freeze/crash records (AC-013-6).
pub use survivor_forward::{
    read_default_survivor_records, read_survivor_records, PalmistrySurvivorKind,
    PalmistrySurvivorView, ENV_PALMISTRY_SURVIVOR_DIR,
};

// WP-KERNEL-012 MT-094 (§6.13.3) re-exports so `main()` + the app can
// `use crate::diagnostics::{launch_palmistry_or_degrade, set_preinstalled_diag_session, ...}` without
// reaching into the `palmistry_launch` submodule path.
pub use palmistry_launch::{
    control_socket_name, crash_socket_path, drain_palmistry_child_watch_commands,
    enqueue_palmistry_child_deregister, enqueue_palmistry_child_liveness_file, launch_palmistry,
    launch_palmistry_at, launch_palmistry_or_degrade, resolve_palmistry_exe,
    set_preinstalled_diag_session, take_preinstalled_diag_session, PalmistryHandle,
    ShutdownOutcome, ENV_PALMISTRY_EXE, SPAWN_NOT_KILL_ON_JOB_CLOSE,
};
