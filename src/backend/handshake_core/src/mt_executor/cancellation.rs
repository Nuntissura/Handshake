//! MT-186 MT cancellation primitive (cooperative + forced) with cleanup hooks.
//!
//! MT-186 Phase-2 remediation (spec §5.7.5; red_team minimum_control #1, #3):
//!
//!  - Forced cancellation now performs a **built-in ProcessOwnershipLedger
//!    reclaim** as a non-optional step. It does not depend on a caller having
//!    registered a cleanup hook — a hung executor that ignores the cooperative
//!    token is force-killed and its owned processes are reclaimed regardless.
//!    If no reclaim path is wired into the canceller, the forced path
//!    **fails loud** with a typed [`ForceCancelError::NoReclaimerConfigured`]
//!    rather than silently leaking an orphan process.
//!
//!  - A built-in **cooperative -> forced escalation timer** is provided by
//!    [`MtCanceller::request_with_force_after`]. It flips the cooperative
//!    token, awaits cooperative drain up to `force_after` (default 30s,
//!    sourced from [`MtCancellationConfig`] which a Work-Profile populates),
//!    then escalates to a forced kill + built-in reclaim **without the caller
//!    orchestrating the timer**.
//!
//!  - Forced cancellation marks the job/MT state `Cancelled` with
//!    `force_used = true` and emits `FR-EVT-MT-CANCEL-FORCED`. The event id is
//!    sourced from the real flight-recorder registry
//!    ([`crate::flight_recorder::fr_event_registry::FrEventId::MtCancelForced`])
//!    so it round-trips against the locked FR-event manifest; no `.GOV/`
//!    registry edit was required.
//!
//! The reclaim seam ([`ForcedCancelReclaimer`]) is a thin async trait so the
//! real [`crate::process_ledger::reclaim::Reclaim`] primitive can be wired in
//! without this module inventing a parallel reclaim path. A blanket adapter
//! ([`ReclaimForcedCancelAdapter`]) binds the real `Reclaim` +
//! `ReclaimTrigger::OperatorCancel` to the canceller.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use thiserror::Error;
use uuid::Uuid;

use super::job::MicroTaskJobId;
use crate::flight_recorder::fr_event_registry::FrEventId;
use crate::process_ledger::reclaim::{Reclaim, ReclaimReport, ReclaimTrigger};
use crate::process_ledger::ProcessLedgerError;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum MtCancellationReason {
    OperatorRequested { operator_id: String },
    SessionShutdown,
    BudgetExceeded,
    EscalationToHardGate,
    DependencyFailed { dep_job_id: Uuid },
}

/// Default cooperative -> forced escalation deadline (spec §5.7.5: 30s).
pub const DEFAULT_FORCE_AFTER_SECS: u64 = 30;

/// Cancellation timing/config. `force_after` is the cooperative drain window
/// before a forced kill is triggered by [`MtCanceller::request_with_force_after`].
///
/// Per spec §5.7.5 the window is per-Work-Profile configurable; a Work-Profile
/// populates this struct at session-spawn time. The default mirrors the spec
/// default of 30 seconds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MtCancellationConfig {
    pub force_after: Duration,
}

impl MtCancellationConfig {
    pub fn with_force_after(force_after: Duration) -> Self {
        Self { force_after }
    }

    /// Normalize a zero window back to the spec default so a mis-populated
    /// Work-Profile cannot disable the escalation timer entirely.
    pub fn normalized(self) -> Self {
        Self {
            force_after: if self.force_after.is_zero() {
                Duration::from_secs(DEFAULT_FORCE_AFTER_SECS)
            } else {
                self.force_after
            },
        }
    }
}

impl Default for MtCancellationConfig {
    fn default() -> Self {
        Self {
            force_after: Duration::from_secs(DEFAULT_FORCE_AFTER_SECS),
        }
    }
}

/// Cooperative cancellation token observable by the executor at safe checkpoints.
#[derive(Debug, Clone)]
pub struct MtCancellationToken {
    job_id: MicroTaskJobId,
    flag: Arc<AtomicBool>,
    reason: Arc<Mutex<Option<MtCancellationReason>>>,
}

impl MtCancellationToken {
    pub fn new(job_id: MicroTaskJobId) -> Self {
        Self {
            job_id,
            flag: Arc::new(AtomicBool::new(false)),
            reason: Arc::new(Mutex::new(None)),
        }
    }

    pub fn job_id(&self) -> MicroTaskJobId {
        self.job_id
    }

    pub fn is_cancelled(&self) -> bool {
        self.flag.load(Ordering::Acquire)
    }

    pub fn reason(&self) -> Option<MtCancellationReason> {
        self.reason.lock().unwrap().clone()
    }

    fn cancel(&self, reason: MtCancellationReason) -> bool {
        // Idempotent: only the first cancellation records the reason.
        let already = self.flag.swap(true, Ordering::AcqRel);
        if !already {
            *self.reason.lock().unwrap() = Some(reason);
            true
        } else {
            false
        }
    }
}

/// Cleanup hook trait. Hooks run in reverse-registration order on cancellation.
pub trait MtCancellationCleanupHook: Send + Sync {
    fn name(&self) -> &'static str;
    fn cleanup(&self, job_id: MicroTaskJobId) -> Result<(), String>;
}

/// Built-in forced-cancellation reclaim seam.
///
/// Forced cancellation reclaims any process the cancelled MT/session owns by
/// driving the real process-ledger reclaim path. The canceller owns one of
/// these; if it is absent the forced path fails loud rather than leaking.
///
/// The associated session id is the `parent_session_id` the
/// `ProcessOwnershipLedger` keys owned processes by. The canceller resolves a
/// job to its session via the registered token (sessions register the job's
/// session when they register the cancellation token), falling back to the
/// caller-supplied session on [`MtCanceller::force_cancel`].
#[async_trait]
pub trait ForcedCancelReclaimer: Send + Sync {
    /// Reclaim every process owned by `session_id`. Returns the reclaim record
    /// (process kill outcomes + a stop event per process) so the forced-cancel
    /// report can carry proof that no orphan survived.
    async fn reclaim_session(
        &self,
        session_id: &str,
    ) -> Result<ReclaimRecord, ProcessLedgerError>;
}

/// Proof-of-reclaim record returned by [`ForcedCancelReclaimer`]. A thin,
/// serializable projection of the real [`ReclaimReport`] so the forced-cancel
/// report is self-describing for diagnostics and tests without leaking the
/// full ledger types.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReclaimRecord {
    pub session_id: String,
    pub processes_reclaimed: u32,
    /// process_uuids that were killed + had a stop event written.
    pub reclaimed_process_uuids: Vec<Uuid>,
    pub total_duration_ms: u128,
}

impl From<ReclaimReport> for ReclaimRecord {
    fn from(report: ReclaimReport) -> Self {
        Self {
            session_id: report.session_id,
            processes_reclaimed: report.processes_reclaimed.len() as u32,
            reclaimed_process_uuids: report
                .processes_reclaimed
                .iter()
                .map(|p| p.process_uuid)
                .collect(),
            total_duration_ms: report.total_duration_ms,
        }
    }
}

/// Adapter binding the real [`Reclaim`] primitive to the forced-cancel seam.
///
/// This is the wiring that makes built-in reclaim use the *real* process-ledger
/// reclaim path (`active_processes_for_session` -> sandbox kill -> stop event)
/// rather than a parallel implementation. Forced MT cancellation is an operator
/// cancel from the ledger's perspective, hence [`ReclaimTrigger::OperatorCancel`].
pub struct ReclaimForcedCancelAdapter {
    reclaim: Arc<Reclaim>,
}

impl ReclaimForcedCancelAdapter {
    pub fn new(reclaim: Arc<Reclaim>) -> Self {
        Self { reclaim }
    }
}

#[async_trait]
impl ForcedCancelReclaimer for ReclaimForcedCancelAdapter {
    async fn reclaim_session(
        &self,
        session_id: &str,
    ) -> Result<ReclaimRecord, ProcessLedgerError> {
        let report = self
            .reclaim
            .run(session_id, ReclaimTrigger::OperatorCancel)
            .await?;
        Ok(report.into())
    }
}

/// Sink for the `FR-EVT-MT-CANCEL-FORCED` flight-recorder emission. Implementors
/// route the event into the flight recorder / event ledger. Kept as an
/// injectable trait so the cancellation primitive stays decoupled from the
/// async flight-recorder actor while still emitting the registry-locked event.
pub trait ForcedCancelEventSink: Send + Sync {
    fn emit_forced_cancel(&self, event: &ForcedCancelEvent);
}

/// Payload for `FR-EVT-MT-CANCEL-FORCED`. `event_id` is sourced from the real
/// FR-event registry so it cannot drift from the locked manifest.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ForcedCancelEvent {
    pub event_id: String,
    pub job_id: MicroTaskJobId,
    pub session_id: Option<String>,
    pub reason: MtCancellationReason,
    pub force_used: bool,
    pub processes_reclaimed: u32,
    pub hooks_invoked: u32,
}

impl ForcedCancelEvent {
    fn new(
        job_id: MicroTaskJobId,
        session_id: Option<String>,
        reason: MtCancellationReason,
        processes_reclaimed: u32,
        hooks_invoked: u32,
    ) -> Self {
        Self {
            event_id: FrEventId::MtCancelForced.as_str().to_string(),
            job_id,
            session_id,
            reason,
            force_used: true,
            processes_reclaimed,
            hooks_invoked,
        }
    }
}

/// Terminal cancellation state recorded after a forced cancel. Mirrors the
/// `MicroTaskJobState::Cancelled` terminal state with the `force_used` flag the
/// queue persists alongside the transition.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CancelledJobState {
    pub job_id: MicroTaskJobId,
    /// Always `Cancelled` here; kept as a string to mirror the queue wire form
    /// (`MicroTaskJobState::Cancelled.as_str()`).
    pub state: String,
    pub force_used: bool,
    pub reason: MtCancellationReason,
}

/// Typed error surface for the forced-cancel path.
#[derive(Debug, Clone, Error, PartialEq, Eq)]
pub enum ForceCancelError {
    /// No [`ForcedCancelReclaimer`] is wired into the canceller. Forced
    /// cancellation refuses to proceed silently because doing so would leak any
    /// orphan process the cancelled MT owns (spec §5.7.5; red_team control #1).
    #[error(
        "forced cancellation of job {job_id} cannot reclaim owned processes: no reclaimer configured (would leak orphan)"
    )]
    NoReclaimerConfigured { job_id: MicroTaskJobId },

    /// The built-in reclaim path failed. The forced cancel is reported as failed
    /// loud so a hung executor's orphans are never assumed reclaimed. The
    /// underlying [`ProcessLedgerError`] is flattened to its display string so
    /// this error stays `Clone + PartialEq + Eq` (callers compare cancellation
    /// failures); the typed source is still surfaced verbatim in the message.
    #[error("forced cancellation reclaim failed for job {job_id}: {message}")]
    ReclaimFailed {
        job_id: MicroTaskJobId,
        message: String,
    },

    /// No session is known for the job (the token was never registered with a
    /// session). Reclaim is session-keyed, so without a session the built-in
    /// reclaim cannot run — fail loud instead of skipping it.
    #[error(
        "forced cancellation of job {job_id} has no associated session id; cannot run session-keyed reclaim"
    )]
    NoSessionForJob { job_id: MicroTaskJobId },
}

pub struct MtCanceller {
    tokens: Mutex<HashMap<MicroTaskJobId, MtCancellationToken>>,
    hooks: Mutex<HashMap<MicroTaskJobId, Vec<Arc<dyn MtCancellationCleanupHook>>>>,
    /// job_id -> parent_session_id used to key the built-in reclaim.
    sessions: Mutex<HashMap<MicroTaskJobId, String>>,
    reclaimer: Option<Arc<dyn ForcedCancelReclaimer>>,
    event_sink: Option<Arc<dyn ForcedCancelEventSink>>,
    config: MtCancellationConfig,
}

impl Default for MtCanceller {
    fn default() -> Self {
        Self::new()
    }
}

impl MtCanceller {
    /// Pure-mode canceller: cooperative + hook-chain only, no built-in reclaim.
    /// `force_cancel` on this instance fails loud (`NoReclaimerConfigured`).
    /// Retained for callers that only need the cooperative primitive.
    pub fn new() -> Self {
        Self {
            tokens: Mutex::new(HashMap::new()),
            hooks: Mutex::new(HashMap::new()),
            sessions: Mutex::new(HashMap::new()),
            reclaimer: None,
            event_sink: None,
            config: MtCancellationConfig::default(),
        }
    }

    /// Reclaim-enabled canceller. `force_cancel` performs the built-in
    /// process-ledger reclaim and emits `FR-EVT-MT-CANCEL-FORCED`.
    pub fn with_reclaim(
        reclaimer: Arc<dyn ForcedCancelReclaimer>,
        event_sink: Arc<dyn ForcedCancelEventSink>,
        config: MtCancellationConfig,
    ) -> Self {
        Self {
            tokens: Mutex::new(HashMap::new()),
            hooks: Mutex::new(HashMap::new()),
            sessions: Mutex::new(HashMap::new()),
            reclaimer: Some(reclaimer),
            event_sink: Some(event_sink),
            config: config.normalized(),
        }
    }

    pub fn config(&self) -> MtCancellationConfig {
        self.config
    }

    pub fn register(&self, job_id: MicroTaskJobId) -> MtCancellationToken {
        let mut tokens = self.tokens.lock().unwrap();
        if let Some(token) = tokens.get(&job_id).cloned() {
            return token;
        }
        let token = MtCancellationToken::new(job_id);
        tokens.insert(job_id, token.clone());
        token
    }

    /// Register the job together with the parent session id the
    /// `ProcessOwnershipLedger` keys its owned processes by. Required for the
    /// built-in forced-cancel reclaim to know what to reclaim.
    pub fn register_with_session(
        &self,
        job_id: MicroTaskJobId,
        session_id: impl Into<String>,
    ) -> MtCancellationToken {
        let token = self.register(job_id);
        self.sessions
            .lock()
            .unwrap()
            .insert(job_id, session_id.into());
        token
    }

    pub fn register_cleanup_hook(
        &self,
        job_id: MicroTaskJobId,
        hook: Arc<dyn MtCancellationCleanupHook>,
    ) {
        self.hooks
            .lock()
            .unwrap()
            .entry(job_id)
            .or_default()
            .push(hook);
    }

    /// Cooperative cancellation: flip the token; executor observes at next
    /// iteration boundary.
    pub fn request_cooperative(
        &self,
        job_id: MicroTaskJobId,
        reason: MtCancellationReason,
    ) -> bool {
        let tokens = self.tokens.lock().unwrap();
        match tokens.get(&job_id) {
            Some(t) => t.cancel(reason),
            None => false,
        }
    }

    fn session_for(&self, job_id: MicroTaskJobId) -> Option<String> {
        self.sessions.lock().unwrap().get(&job_id).cloned()
    }

    fn run_hooks(&self, job_id: MicroTaskJobId) -> (u32, Vec<HookFailure>) {
        let hooks = self
            .hooks
            .lock()
            .unwrap()
            .remove(&job_id)
            .unwrap_or_default();
        let mut errors = Vec::new();
        for hook in hooks.iter().rev() {
            if let Err(e) = hook.cleanup(job_id) {
                errors.push(HookFailure {
                    hook_name: hook.name().to_string(),
                    message: e,
                });
            }
        }
        (hooks.len() as u32, errors)
    }

    /// Forced cancellation (hook-chain only): run cleanup hooks in reverse
    /// order. Errors are captured but never abort the cancellation chain (per
    /// spec § cleanup chain robustness).
    ///
    /// NOTE: this path does **not** perform the built-in process-ledger reclaim.
    /// It is retained for the cooperative + hook-only primitive and for callers
    /// that have no process ownership to reclaim. For the spec §5.7.5 forced
    /// orphan-reclaim guarantee use [`MtCanceller::force_cancel`].
    pub fn force(&self, job_id: MicroTaskJobId, reason: MtCancellationReason) -> ForceCancelReport {
        let _ = self.request_cooperative(job_id, reason);
        let (hooks_invoked, errors) = self.run_hooks(job_id);
        ForceCancelReport {
            job_id,
            hooks_invoked,
            errors,
        }
    }

    /// Forced cancellation with **built-in process-ledger reclaim** (spec
    /// §5.7.5; red_team control #1 + #3).
    ///
    /// Steps, in order:
    ///   1. Flip the cooperative token (records `reason` if not already set).
    ///   2. Run user cleanup hooks in reverse-registration order (errors
    ///      captured, chain never aborts).
    ///   3. Run the **built-in** session-keyed reclaim against the real
    ///      process ledger. If no reclaimer is wired -> fail loud
    ///      ([`ForceCancelError::NoReclaimerConfigured`]); if reclaim itself
    ///      errors -> fail loud ([`ForceCancelError::ReclaimFailed`]). A hung
    ///      executor that ignored the cooperative token is killed + reclaimed
    ///      here regardless of whether any hook was registered.
    ///   4. Mark the job `Cancelled` with `force_used = true`.
    ///   5. Emit `FR-EVT-MT-CANCEL-FORCED` via the event sink.
    ///
    /// `session_override` lets a caller supply the session id when the job was
    /// registered without one; otherwise the session registered via
    /// [`MtCanceller::register_with_session`] is used.
    pub async fn force_cancel(
        &self,
        job_id: MicroTaskJobId,
        reason: MtCancellationReason,
        session_override: Option<String>,
    ) -> Result<ForcedCancelOutcome, ForceCancelError> {
        // (1) cooperative flip — preserve first reason.
        let _ = self.request_cooperative(job_id, reason.clone());

        // (2) user hooks (in addition to the built-in reclaim, never instead).
        let (hooks_invoked, hook_errors) = self.run_hooks(job_id);

        // (3) built-in reclaim — always runs, fail loud on absence/error.
        let reclaimer = self
            .reclaimer
            .as_ref()
            .ok_or(ForceCancelError::NoReclaimerConfigured { job_id })?;
        let session_id = session_override
            .or_else(|| self.session_for(job_id))
            .ok_or(ForceCancelError::NoSessionForJob { job_id })?;
        let reclaim_record = reclaimer.reclaim_session(&session_id).await.map_err(|source| {
            ForceCancelError::ReclaimFailed {
                job_id,
                message: source.to_string(),
            }
        })?;

        // (4) terminal state with force_used = true.
        let cancelled_state = CancelledJobState {
            job_id,
            state: super::job::MicroTaskJobState::Cancelled.as_str().to_string(),
            force_used: true,
            reason: reason.clone(),
        };

        // (5) FR-EVT-MT-CANCEL-FORCED emission (registry-locked id).
        let event = ForcedCancelEvent::new(
            job_id,
            Some(session_id.clone()),
            reason,
            reclaim_record.processes_reclaimed,
            hooks_invoked,
        );
        if let Some(sink) = self.event_sink.as_ref() {
            sink.emit_forced_cancel(&event);
        }

        Ok(ForcedCancelOutcome {
            report: ForceCancelReport {
                job_id,
                hooks_invoked,
                errors: hook_errors,
            },
            reclaim: reclaim_record,
            cancelled_state,
            event,
        })
    }

    /// Cooperative -> forced **escalation timer** (spec §5.7.5; red_team
    /// control #1). Flips the cooperative token, then polls for cooperative
    /// drain up to `force_after` (from [`MtCancellationConfig`], default 30s).
    /// If the executor drains cooperatively in time, returns `Cooperative`. If
    /// the deadline elapses (a hung executor ignoring the token), it
    /// **autonomously** escalates to [`MtCanceller::force_cancel`] — the caller
    /// does not drive the timer.
    ///
    /// `drained` is a predicate the caller supplies to report whether the
    /// executor has finished cooperative shutdown (e.g. the job row reached a
    /// terminal state, or an in-process barrier fired). It is polled on a short
    /// interval until it returns `true` or the deadline passes.
    pub async fn request_with_force_after<F>(
        &self,
        job_id: MicroTaskJobId,
        reason: MtCancellationReason,
        session_override: Option<String>,
        mut drained: F,
    ) -> Result<EscalationOutcome, ForceCancelError>
    where
        F: FnMut() -> bool + Send,
    {
        // Flip cooperative token first; give the executor the chance to drain.
        let _ = self.request_cooperative(job_id, reason.clone());

        let deadline = Instant::now() + self.config.force_after;
        // Poll on a short interval. Interval is capped so a very short
        // force_after (tests) still polls at least a few times.
        let poll = std::cmp::min(
            self.config.force_after / 10 + Duration::from_millis(1),
            Duration::from_millis(50),
        );
        loop {
            if drained() {
                return Ok(EscalationOutcome::Cooperative { job_id });
            }
            if Instant::now() >= deadline {
                break;
            }
            tokio::time::sleep(poll).await;
        }

        // Deadline elapsed without cooperative drain -> autonomous escalation.
        let outcome = self.force_cancel(job_id, reason, session_override).await?;
        Ok(EscalationOutcome::Forced(Box::new(outcome)))
    }
}

/// Result of a successful [`MtCanceller::force_cancel`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ForcedCancelOutcome {
    pub report: ForceCancelReport,
    pub reclaim: ReclaimRecord,
    pub cancelled_state: CancelledJobState,
    pub event: ForcedCancelEvent,
}

/// Result of [`MtCanceller::request_with_force_after`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum EscalationOutcome {
    /// Executor drained cooperatively before `force_after` elapsed.
    Cooperative { job_id: MicroTaskJobId },
    /// Deadline elapsed; forced kill + built-in reclaim ran autonomously.
    Forced(Box<ForcedCancelOutcome>),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ForceCancelReport {
    pub job_id: MicroTaskJobId,
    pub hooks_invoked: u32,
    pub errors: Vec<HookFailure>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HookFailure {
    pub hook_name: String,
    pub message: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicU32;

    struct RecordingHook {
        name: &'static str,
        order: Arc<Mutex<Vec<&'static str>>>,
    }
    impl MtCancellationCleanupHook for RecordingHook {
        fn name(&self) -> &'static str {
            self.name
        }
        fn cleanup(&self, _job_id: MicroTaskJobId) -> Result<(), String> {
            self.order.lock().unwrap().push(self.name);
            Ok(())
        }
    }

    struct FailingHook {
        called: Arc<AtomicU32>,
    }
    impl MtCancellationCleanupHook for FailingHook {
        fn name(&self) -> &'static str {
            "failing"
        }
        fn cleanup(&self, _job_id: MicroTaskJobId) -> Result<(), String> {
            self.called.fetch_add(1, Ordering::SeqCst);
            Err("simulated failure".to_string())
        }
    }

    #[test]
    fn cooperative_cancellation_is_idempotent() {
        let c = MtCanceller::new();
        let id = MicroTaskJobId::new_v7();
        let _ = c.register(id);
        let r1 = c.request_cooperative(id, MtCancellationReason::SessionShutdown);
        let r2 = c.request_cooperative(id, MtCancellationReason::SessionShutdown);
        assert!(r1);
        assert!(!r2);
    }

    #[test]
    fn token_observes_cancellation() {
        let c = MtCanceller::new();
        let id = MicroTaskJobId::new_v7();
        let t = c.register(id);
        assert!(!t.is_cancelled());
        c.request_cooperative(id, MtCancellationReason::BudgetExceeded);
        assert!(t.is_cancelled());
    }

    #[test]
    fn register_preserves_existing_cancelled_token() {
        let c = MtCanceller::new();
        let id = MicroTaskJobId::new_v7();
        let first = c.register(id);
        assert!(c.request_cooperative(id, MtCancellationReason::SessionShutdown));

        let second = c.register(id);

        assert!(first.is_cancelled());
        assert!(second.is_cancelled());
        assert_eq!(second.reason(), Some(MtCancellationReason::SessionShutdown));
    }

    #[test]
    fn cleanup_hooks_run_in_reverse_order() {
        let c = MtCanceller::new();
        let id = MicroTaskJobId::new_v7();
        let _ = c.register(id);
        let order = Arc::new(Mutex::new(Vec::new()));
        c.register_cleanup_hook(
            id,
            Arc::new(RecordingHook {
                name: "first",
                order: Arc::clone(&order),
            }),
        );
        c.register_cleanup_hook(
            id,
            Arc::new(RecordingHook {
                name: "second",
                order: Arc::clone(&order),
            }),
        );
        c.register_cleanup_hook(
            id,
            Arc::new(RecordingHook {
                name: "third",
                order: Arc::clone(&order),
            }),
        );
        let _ = c.force(id, MtCancellationReason::SessionShutdown);
        let recorded = order.lock().unwrap().clone();
        assert_eq!(recorded, vec!["third", "second", "first"]);
    }

    #[test]
    fn cleanup_hook_error_does_not_abort_chain() {
        let c = MtCanceller::new();
        let id = MicroTaskJobId::new_v7();
        let _ = c.register(id);
        let calls = Arc::new(AtomicU32::new(0));
        c.register_cleanup_hook(
            id,
            Arc::new(FailingHook {
                called: Arc::clone(&calls),
            }),
        );
        c.register_cleanup_hook(
            id,
            Arc::new(FailingHook {
                called: Arc::clone(&calls),
            }),
        );
        let report = c.force(id, MtCancellationReason::SessionShutdown);
        assert_eq!(calls.load(Ordering::SeqCst), 2);
        assert_eq!(report.errors.len(), 2);
    }

    #[test]
    fn force_cancel_without_reclaimer_fails_loud() {
        // Pure-mode canceller has no reclaimer wired -> force_cancel must
        // refuse rather than silently leak an orphan.
        let c = MtCanceller::new();
        let id = MicroTaskJobId::new_v7();
        let _ = c.register_with_session(id, "sess-x");
        let rt = tokio::runtime::Builder::new_current_thread()
            .build()
            .unwrap();
        let err = rt
            .block_on(c.force_cancel(id, MtCancellationReason::SessionShutdown, None))
            .expect_err("must fail loud without reclaimer");
        assert_eq!(err, ForceCancelError::NoReclaimerConfigured { job_id: id });
    }
}
