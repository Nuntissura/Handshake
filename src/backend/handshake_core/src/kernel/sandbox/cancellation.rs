//! MT-027 Cancellation and Timeout.
//!
//! Acceptance (MT-027.json): "add cancellation and timeout handling.
//! Acceptance: cancelled runs cannot promote and have typed terminal state."
//!
//! Two cooperative primitives:
//!   * `CancellationToken` — thread-safe boolean flag a watcher can flip to
//!     request cancellation.
//!   * `TimeoutClock` — deterministic, injectable wall-time source (so tests
//!     can advance time without sleeping).
//!
//! `terminate_run` folds both into a single decision that drops the run into
//! the `Rejected` terminal state with a typed cause (`CancelledByOperator`,
//! `WallTimeoutExpired`, `CpuTimeoutExpired`). The terminal state plus the
//! `RunPromotionGuard` ensures cancelled or timed-out runs can never enter a
//! promotion path: `RunPromotionGuard::is_promotable(run)` returns `false` for
//! any non-`Completed` status, and the dedicated `assert_promotable` returns a
//! typed denial when called against a cancelled run.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use serde::{Deserialize, Serialize};

use super::denial::{DenialKind, SandboxDenialRecordV1};
use super::run::{SandboxRunStatus, SandboxRunV1};

#[derive(Debug, Clone, Default)]
pub struct CancellationToken {
    inner: Arc<AtomicBool>,
}

impl CancellationToken {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn cancel(&self) {
        self.inner.store(true, Ordering::SeqCst);
    }
    pub fn is_cancelled(&self) -> bool {
        self.inner.load(Ordering::SeqCst)
    }
}

/// Injectable monotonic clock. Production uses `Instant::now()`-backed
/// implementations; tests use `ManualClock` so deadlines fire deterministically.
pub trait TimeoutClock: Send + Sync {
    fn elapsed_since_start(&self) -> Duration;
}

pub struct ManualClock {
    elapsed: std::sync::Mutex<Duration>,
}

impl ManualClock {
    pub fn new(initial: Duration) -> Self {
        Self {
            elapsed: std::sync::Mutex::new(initial),
        }
    }
    pub fn advance(&self, delta: Duration) {
        let mut e = self.elapsed.lock().unwrap();
        *e += delta;
    }
    pub fn set(&self, value: Duration) {
        *self.elapsed.lock().unwrap() = value;
    }
}

impl TimeoutClock for ManualClock {
    fn elapsed_since_start(&self) -> Duration {
        *self.elapsed.lock().unwrap()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TerminalCause {
    CompletedOk,
    CancelledByOperator,
    WallTimeoutExpired,
    CpuTimeoutExpired,
}

/// Combined termination decision; mutates `run.status` to the appropriate
/// terminal state and returns a typed cause.
pub fn terminate_run(
    run: &mut SandboxRunV1,
    token: &CancellationToken,
    clock: &dyn TimeoutClock,
    wall_timeout: Option<Duration>,
    cpu_timeout: Option<Duration>,
    cpu_elapsed: Option<Duration>,
) -> TerminalCause {
    if run.status.is_terminal() {
        return match run.status {
            SandboxRunStatus::Completed => TerminalCause::CompletedOk,
            _ => TerminalCause::CancelledByOperator,
        };
    }
    if token.is_cancelled() {
        run.status = SandboxRunStatus::Rejected;
        run.finished_at_utc = Some(chrono::Utc::now());
        return TerminalCause::CancelledByOperator;
    }
    if let Some(timeout) = wall_timeout {
        if clock.elapsed_since_start() >= timeout {
            run.status = SandboxRunStatus::Rejected;
            run.finished_at_utc = Some(chrono::Utc::now());
            return TerminalCause::WallTimeoutExpired;
        }
    }
    if let (Some(timeout), Some(elapsed)) = (cpu_timeout, cpu_elapsed) {
        if elapsed >= timeout {
            run.status = SandboxRunStatus::Rejected;
            run.finished_at_utc = Some(chrono::Utc::now());
            return TerminalCause::CpuTimeoutExpired;
        }
    }
    TerminalCause::CompletedOk
}

/// Promotion guard: nothing that did not reach `Completed` may promote.
pub struct RunPromotionGuard;

impl RunPromotionGuard {
    pub fn is_promotable(run: &SandboxRunV1) -> bool {
        matches!(run.status, SandboxRunStatus::Completed)
    }

    pub fn assert_promotable(run: &SandboxRunV1) -> Result<(), SandboxDenialRecordV1> {
        if Self::is_promotable(run) {
            return Ok(());
        }
        Err(SandboxDenialRecordV1::new(
            run.run_id.0.clone(),
            run.policy_version_id.clone(),
            DenialKind::AuthorityModeRefused,
            None,
            format!("promote run `{}`", run.run_id.0),
            format!(
                "run status `{}` is not COMPLETED; promotion refused",
                run.status.as_str()
            ),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fresh_run() -> SandboxRunV1 {
        SandboxRunV1::new_requested("KTR-1", "SES-1", "x", "POL-1@1", "WSP-1")
    }

    #[test]
    fn cancellation_token_starts_unset() {
        let t = CancellationToken::new();
        assert!(!t.is_cancelled());
    }

    #[test]
    fn cancellation_drops_run_into_rejected_with_typed_cause() {
        let mut run = fresh_run();
        run.status = SandboxRunStatus::Started;
        let t = CancellationToken::new();
        t.cancel();
        let clock = ManualClock::new(Duration::from_millis(0));
        let cause = terminate_run(&mut run, &t, &clock, Some(Duration::from_secs(60)), None, None);
        assert_eq!(cause, TerminalCause::CancelledByOperator);
        assert_eq!(run.status, SandboxRunStatus::Rejected);
        assert!(run.finished_at_utc.is_some());
    }

    #[test]
    fn wall_timeout_drops_run_with_typed_cause() {
        let mut run = fresh_run();
        run.status = SandboxRunStatus::Started;
        let t = CancellationToken::new();
        let clock = ManualClock::new(Duration::from_secs(120));
        let cause = terminate_run(&mut run, &t, &clock, Some(Duration::from_secs(60)), None, None);
        assert_eq!(cause, TerminalCause::WallTimeoutExpired);
        assert_eq!(run.status, SandboxRunStatus::Rejected);
    }

    #[test]
    fn cpu_timeout_drops_run_with_typed_cause() {
        let mut run = fresh_run();
        run.status = SandboxRunStatus::Started;
        let t = CancellationToken::new();
        let clock = ManualClock::new(Duration::from_secs(0));
        let cause = terminate_run(
            &mut run,
            &t,
            &clock,
            None,
            Some(Duration::from_secs(10)),
            Some(Duration::from_secs(20)),
        );
        assert_eq!(cause, TerminalCause::CpuTimeoutExpired);
    }

    #[test]
    fn cancelled_run_cannot_promote() {
        let mut run = fresh_run();
        run.status = SandboxRunStatus::Rejected;
        assert!(!RunPromotionGuard::is_promotable(&run));
        let den = RunPromotionGuard::assert_promotable(&run).expect_err("must refuse promotion");
        assert_eq!(den.kind, DenialKind::AuthorityModeRefused);
        assert!(den.reason.contains("REJECTED"));
    }

    #[test]
    fn completed_run_may_promote() {
        let mut run = fresh_run();
        run.status = SandboxRunStatus::Completed;
        assert!(RunPromotionGuard::is_promotable(&run));
        RunPromotionGuard::assert_promotable(&run).expect("completed run promotes");
    }

    #[test]
    fn no_timeout_no_cancel_returns_completed_ok() {
        let mut run = fresh_run();
        run.status = SandboxRunStatus::Started;
        let t = CancellationToken::new();
        let clock = ManualClock::new(Duration::from_secs(5));
        let cause = terminate_run(&mut run, &t, &clock, Some(Duration::from_secs(60)), None, None);
        assert_eq!(cause, TerminalCause::CompletedOk);
        assert_eq!(run.status, SandboxRunStatus::Started);
    }
}
