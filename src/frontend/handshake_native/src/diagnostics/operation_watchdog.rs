//! MT-105 generalized operation stall watchdog.
//!
//! Operations that can hang register a closed numeric [`OperationCode`] plus a bounded progress
//! deadline. The returned [`OperationHandle`] ticks on progress and deregisters on completion/drop.
//! A dedicated poll thread observes monotonic timestamps only; it never waits on the operation it
//! supervises. Stalls emit one typed `StalledOperation` event through the MT-082 recorder.

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicU8, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, OnceLock};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use handshake_diag_ring::DiagEvent;

const OPERATION_STATE_ACTIVE: u8 = 0;
const OPERATION_STATE_STALLING: u8 = 1;
const OPERATION_STATE_STALLED: u8 = 2;
const OPERATION_STATE_COMPLETED: u8 = 3;

/// Default watchdog scan cadence. A stalled operation is observed within its configured deadline plus
/// at most this poll interval when the production poll thread is running.
pub const OPERATION_WATCHDOG_POLL_INTERVAL: Duration = Duration::from_millis(250);

/// Backend HTTP operations are expected to advance or finish quickly. This bound is longer than the
/// MT-088 connect timeout and shorter than the 5s request timeout, so a TCP-accepted-but-silent
/// backend becomes visible before the reqwest request timeout finishes.
pub const BACKEND_OPERATION_STALL_DEADLINE: Duration = Duration::from_secs(2);

/// Closed allowlist of operation kinds. The discriminant is the only operation identity written into
/// the diagnostic event; names, command lines, arguments, and paths never enter the ring payload.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum OperationCode {
    BackendCall = 1,
    ChildProcess = 2,
    ToolRun = 3,
    TerminalSession = 4,
    ModelSession = 5,
}

impl OperationCode {
    #[inline]
    pub const fn as_u64(self) -> u64 {
        self as u64
    }
}

/// Typed payload produced when an operation crosses from healthy to stalled.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StalledOperationReport {
    pub operation_id: u64,
    pub operation_code: OperationCode,
    pub elapsed_ms: u64,
    pub last_progress_ms: u64,
    pub timestamp_nanos: u64,
}

#[derive(Debug)]
struct OperationSlot {
    operation_id: u64,
    operation_code: OperationCode,
    started_nanos: u64,
    last_progress_nanos: AtomicU64,
    deadline_ms: u64,
    progress_interval_ms: u64,
    state: AtomicU8,
}

#[derive(Debug)]
struct OperationWatchdogInner {
    operations: Mutex<HashMap<u64, Arc<OperationSlot>>>,
    next_operation_id: AtomicU64,
    active_stalled_count: AtomicUsize,
    poll_interval: Duration,
    clock_started_at: Instant,
    stop_thread: AtomicBool,
    thread_running: AtomicBool,
}

/// Cheap cloneable watchdog handle. The process-global instance is available through
/// [`global_operation_watchdog`], while tests can construct isolated instances with [`Self::new`].
#[derive(Debug, Clone)]
pub struct OperationWatchdog {
    inner: Arc<OperationWatchdogInner>,
}

impl Default for OperationWatchdog {
    fn default() -> Self {
        Self::new(OPERATION_WATCHDOG_POLL_INTERVAL)
    }
}

impl OperationWatchdog {
    pub fn new(poll_interval: Duration) -> Self {
        Self {
            inner: Arc::new(OperationWatchdogInner {
                operations: Mutex::new(HashMap::new()),
                next_operation_id: AtomicU64::new(1),
                active_stalled_count: AtomicUsize::new(0),
                poll_interval,
                clock_started_at: Instant::now(),
                stop_thread: AtomicBool::new(false),
                thread_running: AtomicBool::new(false),
            }),
        }
    }

    /// Register an operation. The deadline is interpreted as the maximum allowed gap since the last
    /// progress tick; `progress_interval` can provide a tighter gap. This reset-on-progress rule is
    /// what prevents long but healthy ticking operations from false-flagging.
    pub fn register(
        &self,
        operation_code: OperationCode,
        deadline: Duration,
        progress_interval: Option<Duration>,
    ) -> OperationHandle {
        let now = Instant::now();
        let operation_id = self.inner.next_operation_id.fetch_add(1, Ordering::Relaxed);
        let now_nanos = self.elapsed_nanos_at(now);
        let slot = Arc::new(OperationSlot {
            operation_id,
            operation_code,
            started_nanos: now_nanos,
            last_progress_nanos: AtomicU64::new(now_nanos),
            deadline_ms: duration_millis_u64(deadline),
            progress_interval_ms: progress_interval.map_or(0, duration_millis_u64),
            state: AtomicU8::new(OPERATION_STATE_ACTIVE),
        });
        self.lock_operations().insert(operation_id, slot.clone());
        OperationHandle {
            watchdog: self.clone(),
            operation_id,
            slot,
        }
    }

    /// Run one scan on the caller's thread and emit any newly stalled operations.
    pub fn poll_once(&self) -> usize {
        let now = Instant::now();
        let mut emitted = 0;
        while let Some(report) = self.take_next_stalled_report(now) {
            crate::diagnostics::record(DiagEvent::stalled_operation(
                0,
                report.operation_id,
                report.operation_code.as_u64(),
                report.elapsed_ms,
                report.last_progress_ms,
                report.timestamp_nanos,
            ));
            emitted += 1;
        }
        emitted
    }

    /// Start a dedicated polling thread for this watchdog. Returns `None` if this instance already has
    /// a poll thread running.
    pub fn start_poll_thread(&self) -> Option<OperationWatchdogThread> {
        if self.inner.thread_running.swap(true, Ordering::AcqRel) {
            return None;
        }
        self.inner.stop_thread.store(false, Ordering::Release);
        let watchdog = self.clone();
        let join = std::thread::spawn(move || {
            while !watchdog.inner.stop_thread.load(Ordering::Acquire) {
                std::thread::sleep(watchdog.inner.poll_interval);
                watchdog.poll_once();
            }
            watchdog
                .inner
                .thread_running
                .store(false, Ordering::Release);
        });
        Some(OperationWatchdogThread {
            watchdog: self.clone(),
            join: Some(join),
        })
    }

    pub fn active_stalled_count(&self) -> usize {
        self.inner.active_stalled_count.load(Ordering::Acquire)
    }

    fn take_next_stalled_report(&self, now: Instant) -> Option<StalledOperationReport> {
        let timestamp_nanos = self.elapsed_nanos();
        let now_nanos = self.elapsed_nanos_at(now);
        let mut operations = self.lock_operations();
        operations
            .retain(|_, slot| slot.state.load(Ordering::Acquire) != OPERATION_STATE_COMPLETED);
        for slot in operations.values() {
            if slot.state.load(Ordering::Acquire) != OPERATION_STATE_ACTIVE {
                continue;
            }
            let last_progress_nanos = slot.last_progress_nanos.load(Ordering::Acquire);
            let elapsed_ms = nanos_delta_to_millis(now_nanos, slot.started_nanos);
            let last_progress_ms = nanos_delta_to_millis(now_nanos, last_progress_nanos);
            let deadline_exceeded = last_progress_ms >= slot.deadline_ms;
            let progress_exceeded =
                slot.progress_interval_ms > 0 && last_progress_ms >= slot.progress_interval_ms;
            if deadline_exceeded || progress_exceeded {
                if slot
                    .state
                    .compare_exchange(
                        OPERATION_STATE_ACTIVE,
                        OPERATION_STATE_STALLING,
                        Ordering::AcqRel,
                        Ordering::Acquire,
                    )
                    .is_err()
                {
                    continue;
                }
                self.inner
                    .active_stalled_count
                    .fetch_add(1, Ordering::AcqRel);
                if slot
                    .state
                    .compare_exchange(
                        OPERATION_STATE_STALLING,
                        OPERATION_STATE_STALLED,
                        Ordering::AcqRel,
                        Ordering::Acquire,
                    )
                    .is_err()
                {
                    self.decrement_active_stalled_count();
                    continue;
                }
                return Some(StalledOperationReport {
                    operation_id: slot.operation_id,
                    operation_code: slot.operation_code,
                    elapsed_ms,
                    last_progress_ms,
                    timestamp_nanos,
                });
            }
        }
        None
    }

    fn elapsed_nanos(&self) -> u64 {
        duration_nanos_u64(self.inner.clock_started_at.elapsed())
    }

    fn elapsed_nanos_at(&self, instant: Instant) -> u64 {
        duration_nanos_u64(instant.saturating_duration_since(self.inner.clock_started_at))
    }

    fn lock_operations(&self) -> MutexGuard<'_, HashMap<u64, Arc<OperationSlot>>> {
        self.inner
            .operations
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    fn decrement_active_stalled_count(&self) {
        let _ = self.inner.active_stalled_count.fetch_update(
            Ordering::AcqRel,
            Ordering::Acquire,
            |count| count.checked_sub(1),
        );
    }
}

/// Operation lifetime guard. Dropping it deregisters the operation, so completed work cannot be
/// false-flagged by a later poll.
#[derive(Debug)]
pub struct OperationHandle {
    watchdog: OperationWatchdog,
    operation_id: u64,
    slot: Arc<OperationSlot>,
}

impl OperationHandle {
    #[inline]
    pub fn operation_id(&self) -> u64 {
        self.operation_id
    }

    #[inline]
    pub fn tick(&self) {
        if self.slot.state.load(Ordering::Acquire) == OPERATION_STATE_ACTIVE {
            self.slot
                .last_progress_nanos
                .store(self.watchdog.elapsed_nanos(), Ordering::Release);
        }
    }

    #[inline]
    pub fn complete(&self) {
        let previous = self
            .slot
            .state
            .swap(OPERATION_STATE_COMPLETED, Ordering::AcqRel);
        if previous == OPERATION_STATE_STALLED {
            self.watchdog.decrement_active_stalled_count();
        }
    }
}

impl Drop for OperationHandle {
    fn drop(&mut self) {
        self.complete();
    }
}

/// Stops and joins a non-global watchdog poll thread on drop. Production uses the process-global
/// fire-once thread; tests use this guard for isolated watchdog instances.
#[derive(Debug)]
pub struct OperationWatchdogThread {
    watchdog: OperationWatchdog,
    join: Option<JoinHandle<()>>,
}

impl Drop for OperationWatchdogThread {
    fn drop(&mut self) {
        self.watchdog
            .inner
            .stop_thread
            .store(true, Ordering::Release);
        if let Some(join) = self.join.take() {
            let _ = join.join();
        }
    }
}

static GLOBAL_OPERATION_WATCHDOG: OnceLock<OperationWatchdog> = OnceLock::new();
static GLOBAL_OPERATION_WATCHDOG_THREAD: OnceLock<()> = OnceLock::new();

pub fn global_operation_watchdog() -> &'static OperationWatchdog {
    GLOBAL_OPERATION_WATCHDOG.get_or_init(OperationWatchdog::default)
}

pub fn start_global_operation_watchdog() {
    GLOBAL_OPERATION_WATCHDOG_THREAD.get_or_init(|| {
        if let Some(thread) = global_operation_watchdog().start_poll_thread() {
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

#[inline]
fn duration_millis_u64(duration: Duration) -> u64 {
    u64::try_from(duration.as_millis()).unwrap_or(u64::MAX)
}

#[inline]
fn duration_nanos_u64(duration: Duration) -> u64 {
    u64::try_from(duration.as_nanos()).unwrap_or(u64::MAX)
}

#[inline]
fn nanos_delta_to_millis(now_nanos: u64, earlier_nanos: u64) -> u64 {
    now_nanos.saturating_sub(earlier_nanos) / 1_000_000
}
