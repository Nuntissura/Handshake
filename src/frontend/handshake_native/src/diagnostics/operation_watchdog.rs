//! MT-105 generalized operation stall watchdog.
//!
//! Operations that can hang register a closed numeric [`OperationCode`] plus a bounded progress
//! deadline. The returned [`OperationHandle`] ticks on progress and deregisters on completion/drop.
//! A dedicated poll thread observes monotonic timestamps only; it never waits on the operation it
//! supervises. Stalls emit one typed `StalledOperation` event through the MT-082 recorder.

use std::sync::{Arc, OnceLock};
use std::time::Duration;

use handshake_diag_ring::DiagEvent;

pub use handshake_internal_diagnostics::operation_watchdog::{
    OperationCode, OperationHandle, OperationWatchdog, OperationWatchdogThread,
    StalledOperationReport, OPERATION_WATCHDOG_POLL_INTERVAL,
};

/// Backend HTTP operations are expected to advance or finish quickly. This bound is longer than the
/// MT-088 connect timeout and shorter than the 5s request timeout, so a TCP-accepted-but-silent
/// backend becomes visible before the reqwest request timeout finishes.
pub const BACKEND_OPERATION_STALL_DEADLINE: Duration = Duration::from_secs(2);

/// Absolute lifetime ceiling for production backend operations registered with the watchdog.
///
/// The shared backend client has a five-second timeout for the health and layout requests that use
/// this registration. Matching that bound here means a request that keeps reporting synthetic
/// progress cannot evade diagnostics forever, while the shorter two-second progress-gap deadline
/// still reports a silent socket first.
pub const BACKEND_OPERATION_MAX_TOTAL_RUNTIME: Duration = Duration::from_secs(5);

pub fn emit_stalled_operation(report: StalledOperationReport) {
    crate::diagnostics::record(DiagEvent::stalled_operation(
        0,
        report.operation_id,
        report.operation_code.as_u64(),
        report.elapsed_ms,
        report.last_progress_ms,
        report.timestamp_nanos,
    ));
}

static GLOBAL_OPERATION_WATCHDOG: OnceLock<OperationWatchdog> = OnceLock::new();
static GLOBAL_OPERATION_WATCHDOG_THREAD: OnceLock<()> = OnceLock::new();

pub fn global_operation_watchdog() -> &'static OperationWatchdog {
    GLOBAL_OPERATION_WATCHDOG.get_or_init(OperationWatchdog::default)
}

/// Register a production backend operation with both diagnostic bounds enabled.
///
/// Keeping this in the diagnostics owner prevents individual app call sites from accidentally
/// reverting to the uncapped `register` API. The returned handle still completes/deregisters in the
/// normal RAII path.
pub fn register_backend_operation() -> OperationHandle {
    global_operation_watchdog().register_with_runtime_cap(
        OperationCode::BackendCall,
        BACKEND_OPERATION_STALL_DEADLINE,
        None,
        Some(BACKEND_OPERATION_MAX_TOTAL_RUNTIME),
    )
}

pub fn start_global_operation_watchdog() {
    GLOBAL_OPERATION_WATCHDOG_THREAD.get_or_init(|| {
        if let Some(thread) =
            global_operation_watchdog().start_poll_thread(Arc::new(emit_stalled_operation))
        {
            std::mem::forget(thread);
        }
    });
}

#[inline]
pub fn recent_stalled_operation_count(window: usize) -> usize {
    crate::diagnostics::snapshot_last_n(window)
        .iter()
        .filter(|event| {
            event.event_code == handshake_diag_ring::DiagEventCode::StalledOperation.as_u16()
        })
        .count()
}

#[inline]
pub fn active_stalled_operation_count() -> usize {
    global_operation_watchdog().active_stalled_count()
}
