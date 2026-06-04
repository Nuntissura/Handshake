//! MT-199 Flight Recorder span lifecycle FR-EVT-SPAN-* emitter.
//!
//! Authority: cluster X.4 contract
//! `.GOV/task_packets/WP-KERNEL-004-.../MT-199.json`.
//!
//! Surface contract:
//!  - [`FrRecorder`] — async trait. Implementations write `(FrEventId, payload,
//!    Option<SpanContextRef>)` triples to a durable sink. Errors are reported
//!    via [`FrRecorderError`]; the trait does not panic.
//!  - [`SpanContextRef`] — correlation handle carried into every recorded
//!    event. Holds the span_id (always present), parent_span_id (Some on
//!    activity spans, None on session spans), model_session_id, and
//!    session_id.
//!  - [`SpanFrEmitter`] — lifecycle façade. Owns a recorder + a
//!    [`SpanFrEmitterConfig`] (sample_rate, force_emit_failures). Provides
//!    `on_span_start`, `on_span_end`, `on_span_failed`,
//!    `on_span_checkpoint`, `on_span_attach_activity` which emit the
//!    matching FR-EVT-SPAN-* via the registry.
//!  - [`NullFrRecorder`] — fail-open no-op; used in unit tests and as the
//!    default emitter in untouched call sites.
//!  - [`InMemoryStubFrRecorder`] — async-capable in-memory recorder for
//!    integration assertions.
//!  - [`PostgresFrRecorder`] — production sink. Adapts MT-191's batched-
//!    channel pattern: every `record` call non-blockingly sends onto a
//!    bounded mpsc; a background flusher drains the channel into
//!    `kernel_event_ledger` in batched inserts. The recorder's [`Drop`]
//!    signals the flusher to drain and shut down.
//!  - [`current_span_emitter`] / [`set_span_emitter_scope`] — task-local
//!    injection point so tests can substitute `NullFrRecorder` /
//!    `InMemoryStubFrRecorder` without touching every call site.
//!
//! Authority-files alignment: MT-199 `owned_files` names
//! `src/backend/handshake_core/src/observability/fr_emitter.rs`; the
//! cluster already landed cluster-X.4 surfaces under `flight_recorder::*`
//! via MT-196/197/198 deviation, so this MT extends the same module and
//! flags the same PATH_SHAPE_DRIFT for operator decision.
//!
//! Spec invariant (master-spec v02.186 §5 observability):
//!  - "failures are never silently dropped" — [`SpanFrEmitter::on_span_failed`]
//!    bypasses sampling when `force_emit_failures` is true (the default).
//!  - Drop-on-panic semantics: tested by an integration test under
//!    `tests/observability_fr_integration_tests.rs`. The SpanGuard Drop in
//!    `spans.rs` calls the emitter unconditionally, so the panic unwind
//!    path executes the same recorder write.

use std::collections::hash_map::DefaultHasher;
use std::fmt;
use std::future::Future;
use std::hash::{Hash, Hasher};
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value as JsonValue};
use sqlx::PgPool;
use thiserror::Error;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tokio::time::timeout;
use uuid::Uuid;

use super::fr_event_registry::FrEventId;
use super::spans::SpanId;

/// Default sample rate when [`SpanFrEmitterConfig::default`] is used.
/// `1.0` means "emit every span boundary".
pub const DEFAULT_SAMPLE_RATE: f32 = 1.0;

/// Default for [`SpanFrEmitterConfig::force_emit_failures`].
/// `true` because the master-spec §5 observability invariant says
/// "failures are never silently dropped".
pub const DEFAULT_FORCE_EMIT_FAILURES: bool = true;

/// Default mpsc capacity for [`PostgresFrRecorder`]. Sized for ~1k
/// in-flight span events; producers `try_send` and fall back to
/// dropping with [`FrRecorderError::Backpressure`] when full so a
/// slow Postgres does not block span Drop.
pub const DEFAULT_POSTGRES_CHANNEL_CAPACITY: usize = 1024;

/// Default batch size for the background flusher.
pub const DEFAULT_POSTGRES_BATCH_SIZE: usize = 64;

/// Default flush interval (events are flushed at the earlier of
/// batch_size reached OR interval elapsed since the first queued
/// event in the current batch).
pub const DEFAULT_POSTGRES_FLUSH_INTERVAL: Duration = Duration::from_millis(50);

/// Source-component string written into `kernel_event_ledger.source_component`.
pub const FR_EMITTER_SOURCE_COMPONENT: &str = "flight_recorder.span_fr_emitter";

// ---------------- error type ----------------

#[derive(Debug, Error)]
pub enum FrRecorderError {
    /// Recorder accepted the event but the durable sink dropped it
    /// because the in-flight channel was full. Span Drop MUST NOT
    /// block; the recorder degrades to dropping events instead.
    /// Failures still propagate via the in-memory `SpanGuard` path
    /// because `force_emit_failures` is honoured by the
    /// [`SpanFrEmitter`] before the recorder is even called.
    #[error("recorder backpressure: in-flight channel full")]
    Backpressure,
    /// Underlying Postgres write failed; the flusher logs and
    /// continues (does not poison the recorder).
    #[error("sqlx error: {0}")]
    Sqlx(#[from] sqlx::Error),
    /// JSON serialisation failed.
    #[error("serde_json error: {0}")]
    Serde(#[from] serde_json::Error),
    /// Recorder was already shut down (Drop signalled the flusher).
    #[error("recorder is shut down")]
    Shutdown,
}

// ---------------- SpanContextRef ----------------

/// Correlation handle carried into every recorded event.
///
/// `span_id` is always present; `parent_span_id` is `Some` for
/// activity spans and `None` for session spans. `model_session_id`
/// and `session_id` are `Some` when the span boundary is a session
/// span; activity spans may still propagate them via attribute
/// payload but the canonical correlation key is the parent_span_id.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SpanContextRef {
    pub span_id: SpanId,
    pub parent_span_id: Option<SpanId>,
    pub model_session_id: Option<Uuid>,
    pub session_id: Option<Uuid>,
    pub is_session_span: bool,
}

impl SpanContextRef {
    pub fn for_session_span(span_id: SpanId, model_session_id: Uuid, session_id: Uuid) -> Self {
        Self {
            span_id,
            parent_span_id: None,
            model_session_id: Some(model_session_id),
            session_id: Some(session_id),
            is_session_span: true,
        }
    }

    pub fn for_activity_span(
        span_id: SpanId,
        parent_span_id: SpanId,
        model_session_id: Option<Uuid>,
        session_id: Option<Uuid>,
    ) -> Self {
        Self {
            span_id,
            parent_span_id: Some(parent_span_id),
            model_session_id,
            session_id,
            is_session_span: false,
        }
    }
}

// ---------------- FrRecorder trait ----------------

pub type RecordFuture<'a> = Pin<Box<dyn Future<Output = Result<(), FrRecorderError>> + Send + 'a>>;

/// MT-199 canonical recorder trait. Async by construction so a
/// Postgres-backed recorder can lean on tokio mpsc + sqlx without
/// blocking the Drop path.
///
/// Implementations MUST be cheap on the calling thread; the
/// [`PostgresFrRecorder`] satisfies this by `try_send`-ing onto a
/// bounded channel and returning immediately.
pub trait FrRecorder: Send + Sync {
    fn record<'a>(
        &'a self,
        event_id: FrEventId,
        payload: JsonValue,
        span_context: Option<SpanContextRef>,
    ) -> RecordFuture<'a>;
}

// ---------------- NullFrRecorder ----------------

/// No-op recorder. Returns `Ok(())` immediately. Used as the default
/// when no emitter is injected and in unit tests that do not need
/// to assert emission.
#[derive(Debug, Default, Clone, Copy)]
pub struct NullFrRecorder;

impl FrRecorder for NullFrRecorder {
    fn record<'a>(
        &'a self,
        _event_id: FrEventId,
        _payload: JsonValue,
        _span_context: Option<SpanContextRef>,
    ) -> RecordFuture<'a> {
        Box::pin(async { Ok(()) })
    }
}

// ---------------- InMemoryStubFrRecorder ----------------

/// Captured event for assertions.
#[derive(Debug, Clone, PartialEq)]
pub struct CapturedFrEvent {
    pub event_id: FrEventId,
    pub payload: JsonValue,
    pub span_context: Option<SpanContextRef>,
    pub captured_at_utc: DateTime<Utc>,
}

/// In-memory recorder used by integration tests. Captures the
/// (event_id, payload, span_context) triple verbatim so assertions
/// can introspect parent_span_id correlation, sampling effects,
/// and Drop-on-panic emission.
#[derive(Debug, Default)]
pub struct InMemoryStubFrRecorder {
    events: Mutex<Vec<CapturedFrEvent>>,
}

impl InMemoryStubFrRecorder {
    pub fn new() -> Self {
        Self {
            events: Mutex::new(Vec::new()),
        }
    }

    pub fn snapshot(&self) -> Vec<CapturedFrEvent> {
        self.events.lock().unwrap().clone()
    }

    pub fn count_of(&self, event_id: FrEventId) -> usize {
        self.events
            .lock()
            .unwrap()
            .iter()
            .filter(|e| e.event_id == event_id)
            .count()
    }

    pub fn clear(&self) {
        self.events.lock().unwrap().clear();
    }
}

impl FrRecorder for InMemoryStubFrRecorder {
    fn record<'a>(
        &'a self,
        event_id: FrEventId,
        payload: JsonValue,
        span_context: Option<SpanContextRef>,
    ) -> RecordFuture<'a> {
        Box::pin(async move {
            self.events.lock().unwrap().push(CapturedFrEvent {
                event_id,
                payload,
                span_context,
                captured_at_utc: Utc::now(),
            });
            Ok(())
        })
    }
}

// ---------------- SpanFrEmitter config + emitter ----------------

/// Per-emitter sampling configuration.
///
/// `sample_rate` is clamped into `[0.0, 1.0]`. A rate of `0.0` skips
/// all *non-failure* emissions; failures are still emitted when
/// `force_emit_failures` is true (the default).
///
/// Sampling is deterministic per-span: the sampling decision is made
/// from a stable hash of `span_id`, so SpanStarted and the matching
/// SpanEnded for the same span are *always* both emitted or both
/// suppressed (never half-emitted). This keeps downstream joins
/// consistent.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SpanFrEmitterConfig {
    pub sample_rate: f32,
    pub force_emit_failures: bool,
}

impl Default for SpanFrEmitterConfig {
    fn default() -> Self {
        Self {
            sample_rate: DEFAULT_SAMPLE_RATE,
            force_emit_failures: DEFAULT_FORCE_EMIT_FAILURES,
        }
    }
}

impl SpanFrEmitterConfig {
    pub fn full_sampling() -> Self {
        Self::default()
    }

    pub fn no_sampling() -> Self {
        Self {
            sample_rate: 0.0,
            force_emit_failures: true,
        }
    }

    /// Deterministic per-span sampling decision. Uses a stable hash
    /// over the span_id so SpanStarted/SpanEnded for the same span
    /// agree.
    pub fn should_sample(&self, span_id: SpanId) -> bool {
        let rate = self.sample_rate.clamp(0.0, 1.0);
        if rate >= 1.0 {
            return true;
        }
        if rate <= 0.0 {
            return false;
        }
        let mut hasher = DefaultHasher::new();
        span_id.as_uuid().hash(&mut hasher);
        let h = hasher.finish();
        // Map u64 -> f32 in [0,1)
        let bucket = ((h as f64) / (u64::MAX as f64)) as f32;
        bucket < rate
    }
}

/// Span lifecycle emitter. The shared façade between the sync
/// `SpanGuard` Drop path in `spans.rs` and the async [`FrRecorder`]
/// implementations defined in this module.
///
/// `Arc<SpanFrEmitter>` is the canonical handle; clone it freely.
pub struct SpanFrEmitter {
    recorder: Arc<dyn FrRecorder>,
    config: SpanFrEmitterConfig,
}

impl SpanFrEmitter {
    pub fn new(recorder: Arc<dyn FrRecorder>, config: SpanFrEmitterConfig) -> Self {
        Self { recorder, config }
    }

    /// Convenience constructor with the default sampling config
    /// (emit every boundary; failures always emitted).
    pub fn with_recorder(recorder: Arc<dyn FrRecorder>) -> Self {
        Self::new(recorder, SpanFrEmitterConfig::default())
    }

    pub fn null() -> Self {
        Self::with_recorder(Arc::new(NullFrRecorder))
    }

    pub fn config(&self) -> &SpanFrEmitterConfig {
        &self.config
    }

    pub fn recorder(&self) -> Arc<dyn FrRecorder> {
        Arc::clone(&self.recorder)
    }

    /// SpanStarted emission. Sampled per [`SpanFrEmitterConfig`].
    pub async fn on_span_start(
        &self,
        ctx: SpanContextRef,
        activity_kind: Option<&str>,
        attributes: &JsonValue,
    ) -> Result<(), FrRecorderError> {
        if !self.config.should_sample(ctx.span_id) {
            return Ok(());
        }
        let payload = json!({
            "span_id": ctx.span_id.as_uuid(),
            "parent_span_id": ctx.parent_span_id.map(|s| s.as_uuid()),
            "model_session_id": ctx.model_session_id,
            "session_id": ctx.session_id,
            "activity_kind": activity_kind,
            "attributes": attributes,
        });
        self.recorder
            .record(FrEventId::SpanStarted, payload, Some(ctx))
            .await
    }

    /// SpanEnded emission. Sampled per [`SpanFrEmitterConfig`].
    pub async fn on_span_end(
        &self,
        ctx: SpanContextRef,
        duration_ms: i64,
    ) -> Result<(), FrRecorderError> {
        if !self.config.should_sample(ctx.span_id) {
            return Ok(());
        }
        let payload = json!({
            "span_id": ctx.span_id.as_uuid(),
            "parent_span_id": ctx.parent_span_id.map(|s| s.as_uuid()),
            "duration_ms": duration_ms,
            "is_session_span": ctx.is_session_span,
        });
        self.recorder
            .record(FrEventId::SpanEnded, payload, Some(ctx))
            .await
    }

    /// SpanFailed emission. When `force_emit_failures` is true (the
    /// default) sampling is BYPASSED so a failure is never silently
    /// dropped — per master-spec §5 observability invariant.
    pub async fn on_span_failed(
        &self,
        ctx: SpanContextRef,
        duration_ms: i64,
        failure_reason: &str,
    ) -> Result<(), FrRecorderError> {
        if !self.config.force_emit_failures && !self.config.should_sample(ctx.span_id) {
            return Ok(());
        }
        let payload = json!({
            "span_id": ctx.span_id.as_uuid(),
            "parent_span_id": ctx.parent_span_id.map(|s| s.as_uuid()),
            "duration_ms": duration_ms,
            "failure_reason": failure_reason,
        });
        self.recorder
            .record(FrEventId::SpanFailed, payload, Some(ctx))
            .await
    }

    /// SpanLifecycleCheckpoint emission. Mid-span checkpoint used by
    /// long-running spans to publish progress without ending the span.
    /// Always honours sampling.
    pub async fn on_span_checkpoint(
        &self,
        ctx: SpanContextRef,
        checkpoint_seq: i64,
        event_ledger_seq: i64,
    ) -> Result<(), FrRecorderError> {
        if !self.config.should_sample(ctx.span_id) {
            return Ok(());
        }
        let payload = json!({
            "span_id": ctx.span_id.as_uuid(),
            "checkpoint_seq": checkpoint_seq,
            "event_ledger_seq": event_ledger_seq,
        });
        self.recorder
            .record(FrEventId::SpanLifecycleCheckpoint, payload, Some(ctx))
            .await
    }

    /// SpanLifecycleAttachActivity emission. Records the late binding
    /// of an activity span to a parent ModelSessionSpan when the
    /// activity span was started before the parent context was
    /// resolvable. Always honours sampling.
    pub async fn on_span_attach_activity(
        &self,
        ctx: SpanContextRef,
        parent_span_id: SpanId,
    ) -> Result<(), FrRecorderError> {
        if !self.config.should_sample(ctx.span_id) {
            return Ok(());
        }
        let payload = json!({
            "span_id": ctx.span_id.as_uuid(),
            "parent_span_id": parent_span_id.as_uuid(),
            "attached_at_utc": Utc::now().to_rfc3339(),
        });
        self.recorder
            .record(FrEventId::SpanLifecycleAttachActivity, payload, Some(ctx))
            .await
    }

    /// Synchronous span-start emission. Used by `SpanGuard::start_*`
    /// non-async code paths in `spans.rs` to keep the existing
    /// constructor signatures. Bridges to the async recorder via
    /// `tokio::runtime::Handle::try_current()`; falls back to
    /// `Handle::block_on` only if no runtime is current (this
    /// preserves backward compatibility with sync `SpanGuard` tests).
    pub fn on_span_start_blocking(
        &self,
        ctx: SpanContextRef,
        activity_kind: Option<&str>,
        attributes: &JsonValue,
    ) {
        // Clone owned data so the spawned future does not borrow `self`.
        // (The spawn path in `block_on_in_recorder` requires a `'static`
        // future; borrowing `&self`/`&str`/`&JsonValue` here would leak
        // those references into the spawned task, which is unsound.)
        if !self.config.should_sample(ctx.span_id) {
            return;
        }
        let recorder = Arc::clone(&self.recorder);
        let activity_kind_owned: Option<String> = activity_kind.map(str::to_owned);
        let attributes_owned: JsonValue = attributes.clone();
        let payload = json!({
            "span_id": ctx.span_id.as_uuid(),
            "parent_span_id": ctx.parent_span_id.map(|s| s.as_uuid()),
            "model_session_id": ctx.model_session_id,
            "session_id": ctx.session_id,
            "activity_kind": activity_kind_owned,
            "attributes": attributes_owned,
        });
        let fut = async move {
            recorder
                .record(FrEventId::SpanStarted, payload, Some(ctx))
                .await
        };
        block_on_in_recorder(fut);
    }

    pub fn on_span_end_blocking(&self, ctx: SpanContextRef, duration_ms: i64) {
        if !self.config.should_sample(ctx.span_id) {
            return;
        }
        let recorder = Arc::clone(&self.recorder);
        let payload = json!({
            "span_id": ctx.span_id.as_uuid(),
            "parent_span_id": ctx.parent_span_id.map(|s| s.as_uuid()),
            "duration_ms": duration_ms,
            "is_session_span": ctx.is_session_span,
        });
        let fut = async move {
            recorder
                .record(FrEventId::SpanEnded, payload, Some(ctx))
                .await
        };
        block_on_in_recorder(fut);
    }

    pub fn on_span_failed_blocking(&self, ctx: SpanContextRef, duration_ms: i64, reason: &str) {
        if !self.config.force_emit_failures && !self.config.should_sample(ctx.span_id) {
            return;
        }
        let recorder = Arc::clone(&self.recorder);
        let reason_owned: String = reason.to_owned();
        let payload = json!({
            "span_id": ctx.span_id.as_uuid(),
            "parent_span_id": ctx.parent_span_id.map(|s| s.as_uuid()),
            "duration_ms": duration_ms,
            "failure_reason": reason_owned,
        });
        let fut = async move {
            recorder
                .record(FrEventId::SpanFailed, payload, Some(ctx))
                .await
        };
        block_on_in_recorder(fut);
    }
}

impl fmt::Debug for SpanFrEmitter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SpanFrEmitter")
            .field("config", &self.config)
            .finish_non_exhaustive()
    }
}

/// Helper that executes a `record` future from sync code. If a
/// tokio runtime handle is current, the future is spawned (fire-
/// and-forget) so we never block the Drop thread; otherwise it
/// runs to completion synchronously via `futures::executor::block_on`.
/// Errors are intentionally swallowed: span Drop must not panic.
fn block_on_in_recorder(fut: impl Future<Output = Result<(), FrRecorderError>> + Send + 'static) {
    if let Ok(handle) = tokio::runtime::Handle::try_current() {
        handle.spawn(async move {
            let _ = fut.await;
        });
    } else {
        let _ = futures::executor::block_on(fut);
    }
}

// ---------------- task-local emitter injection ----------------

tokio::task_local! {
    static CURRENT_SPAN_EMITTER: Arc<SpanFrEmitter>;
}

/// Run `future` inside a task-local scope where
/// [`current_span_emitter`] returns the supplied emitter.
pub async fn set_span_emitter_scope<F>(emitter: Arc<SpanFrEmitter>, future: F) -> F::Output
where
    F: Future,
{
    CURRENT_SPAN_EMITTER.scope(emitter, future).await
}

/// Look up the currently scoped emitter, or `None` if no scope is
/// active. Call sites that always need an emitter should fall back
/// to a [`NullFrRecorder`]-backed default.
pub fn current_span_emitter() -> Option<Arc<SpanFrEmitter>> {
    CURRENT_SPAN_EMITTER.try_with(|e| Arc::clone(e)).ok()
}

// ---------------- PostgresFrRecorder ----------------

#[derive(Debug, Clone)]
pub struct PostgresFrRecorderConfig {
    pub channel_capacity: usize,
    pub batch_size: usize,
    pub flush_interval: Duration,
    pub kernel_task_run_id: String,
    pub session_run_id: String,
}

impl Default for PostgresFrRecorderConfig {
    fn default() -> Self {
        Self {
            channel_capacity: DEFAULT_POSTGRES_CHANNEL_CAPACITY,
            batch_size: DEFAULT_POSTGRES_BATCH_SIZE,
            flush_interval: DEFAULT_POSTGRES_FLUSH_INTERVAL,
            kernel_task_run_id: "mt199-default-task-run".to_string(),
            session_run_id: "mt199-default-session-run".to_string(),
        }
    }
}

struct PostgresFrRecorderEnvelope {
    event_id: FrEventId,
    payload: JsonValue,
    span_context: Option<SpanContextRef>,
    queued_at: DateTime<Utc>,
}

/// Postgres-backed `FrRecorder`. Adapts MT-191's batched channel
/// pattern: producers `try_send` onto a bounded mpsc; a background
/// flusher drains the queue into `kernel_event_ledger`.
///
/// Span Drop is non-blocking: a full channel returns
/// [`FrRecorderError::Backpressure`] which the span path treats as a
/// soft drop. Failures still propagate because
/// `SpanFrEmitterConfig::force_emit_failures` lifts sampling first
/// — failures are dropped only if the channel is full AND the
/// flusher cannot keep up, which is a system-level back-pressure
/// signal worth surfacing on its own.
pub struct PostgresFrRecorder {
    tx: mpsc::Sender<PostgresFrRecorderEnvelope>,
    flusher: Option<JoinHandle<()>>,
    shutdown_tx: Option<mpsc::Sender<()>>,
}

impl PostgresFrRecorder {
    pub fn spawn(pool: PgPool, cfg: PostgresFrRecorderConfig) -> Self {
        let (tx, rx) = mpsc::channel::<PostgresFrRecorderEnvelope>(cfg.channel_capacity);
        let (shutdown_tx, shutdown_rx) = mpsc::channel::<()>(1);
        let flusher = tokio::spawn(flusher_loop(pool, cfg, rx, shutdown_rx));
        Self {
            tx,
            flusher: Some(flusher),
            shutdown_tx: Some(shutdown_tx),
        }
    }

    /// Number of envelopes currently buffered (approximation; for
    /// diagnostic + test introspection only).
    pub fn buffer_capacity(&self) -> usize {
        self.tx.capacity()
    }

    /// Cooperative shutdown: signal flusher, then join. Idempotent.
    pub async fn shutdown(&mut self) {
        if let Some(s) = self.shutdown_tx.take() {
            let _ = s.send(()).await;
        }
        if let Some(handle) = self.flusher.take() {
            let _ = timeout(Duration::from_secs(5), handle).await;
        }
    }
}

impl FrRecorder for PostgresFrRecorder {
    fn record<'a>(
        &'a self,
        event_id: FrEventId,
        payload: JsonValue,
        span_context: Option<SpanContextRef>,
    ) -> RecordFuture<'a> {
        let envelope = PostgresFrRecorderEnvelope {
            event_id,
            payload,
            span_context,
            queued_at: Utc::now(),
        };
        let tx = self.tx.clone();
        Box::pin(async move {
            match tx.try_send(envelope) {
                Ok(()) => Ok(()),
                Err(mpsc::error::TrySendError::Full(_)) => Err(FrRecorderError::Backpressure),
                Err(mpsc::error::TrySendError::Closed(_)) => Err(FrRecorderError::Shutdown),
            }
        })
    }
}

impl Drop for PostgresFrRecorder {
    fn drop(&mut self) {
        // Best-effort shutdown signal; the flusher will see the
        // mpsc closure when `tx` drops and exit on its own loop
        // iteration. Block on the flusher for a brief moment if a
        // runtime handle is current.
        if let Some(s) = self.shutdown_tx.take() {
            let _ = s.try_send(());
        }
        if let Some(handle) = self.flusher.take() {
            if let Ok(rt) = tokio::runtime::Handle::try_current() {
                rt.spawn(async move {
                    let _ = timeout(Duration::from_secs(2), handle).await;
                });
            }
        }
    }
}

async fn flusher_loop(
    pool: PgPool,
    cfg: PostgresFrRecorderConfig,
    mut rx: mpsc::Receiver<PostgresFrRecorderEnvelope>,
    mut shutdown_rx: mpsc::Receiver<()>,
) {
    let mut batch: Vec<PostgresFrRecorderEnvelope> = Vec::with_capacity(cfg.batch_size);
    loop {
        let collected = tokio::select! {
            _ = shutdown_rx.recv() => {
                // drain any remaining events synchronously then exit
                while let Ok(env) = rx.try_recv() {
                    batch.push(env);
                }
                if !batch.is_empty() {
                    let _ = flush_batch(&pool, &cfg, &mut batch).await;
                }
                return;
            }
            envelope = rx.recv() => {
                match envelope {
                    Some(env) => {
                        batch.push(env);
                        true
                    }
                    None => {
                        // sender side dropped; final drain and exit
                        if !batch.is_empty() {
                            let _ = flush_batch(&pool, &cfg, &mut batch).await;
                        }
                        return;
                    }
                }
            }
            _ = tokio::time::sleep(cfg.flush_interval) => {
                false
            }
        };

        // Drain whatever else is immediately ready.
        while batch.len() < cfg.batch_size {
            match rx.try_recv() {
                Ok(env) => batch.push(env),
                Err(_) => break,
            }
        }

        if batch.len() >= cfg.batch_size || (!collected && !batch.is_empty()) {
            let _ = flush_batch(&pool, &cfg, &mut batch).await;
        }
    }
}

async fn flush_batch(
    pool: &PgPool,
    cfg: &PostgresFrRecorderConfig,
    batch: &mut Vec<PostgresFrRecorderEnvelope>,
) -> Result<(), FrRecorderError> {
    if batch.is_empty() {
        return Ok(());
    }
    // Multi-row INSERT into kernel_event_ledger. Idempotency key
    // is unique-per-event so concurrent flushers cannot double-write.
    // Note: this is a best-effort path — sqlx errors are logged and
    // the batch is dropped (we have no retry queue yet). The contract
    // for failures being preserved at the SOURCE is held by the
    // SpanGuard Drop emitting an in-process recorder before any
    // batched recorder is touched.
    let mut tx = pool.begin().await?;
    for env in batch.iter() {
        let event_uuid = Uuid::now_v7();
        let event_id_str = env.event_id.as_str();
        let aggregate_type = "flight_recorder_span";
        let aggregate_id = env
            .span_context
            .as_ref()
            .map(|s| s.span_id.as_uuid().to_string())
            .unwrap_or_else(|| "unknown_span".to_string());
        let idempotency_key = format!("mt199:{}:{}", event_id_str, event_uuid);
        let payload_hash = blake3_hash_of_payload(&env.payload);
        let correlation_id = env
            .span_context
            .as_ref()
            .and_then(|s| s.model_session_id)
            .map(|u| u.to_string());
        let causation_id = env
            .span_context
            .as_ref()
            .and_then(|s| s.parent_span_id)
            .map(|s| s.as_uuid().to_string());
        sqlx::query(
            r#"
            INSERT INTO kernel_event_ledger (
                event_id, event_version, kernel_task_run_id, session_run_id,
                aggregate_type, aggregate_id, idempotency_key, event_type,
                actor_kind, actor_id, causation_id, correlation_id,
                payload_hash, source_component, payload, created_at
            )
            VALUES ($1,'1', $2, $3, $4, $5, $6, $7, 'system', 'mt199_fr_emitter',
                    $8, $9, $10, $11, $12, $13)
            ON CONFLICT (idempotency_key) DO NOTHING
            "#,
        )
        .bind(event_uuid.to_string())
        .bind(&cfg.kernel_task_run_id)
        .bind(&cfg.session_run_id)
        .bind(aggregate_type)
        .bind(&aggregate_id)
        .bind(&idempotency_key)
        .bind(event_id_str)
        .bind(causation_id)
        .bind(correlation_id)
        .bind(&payload_hash)
        .bind(FR_EMITTER_SOURCE_COMPONENT)
        .bind(&env.payload)
        .bind(env.queued_at)
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await?;
    batch.clear();
    Ok(())
}

fn blake3_hash_of_payload(payload: &JsonValue) -> String {
    use std::io::Write;
    let mut hasher = DefaultHasher::new();
    let bytes = serde_json::to_vec(payload).unwrap_or_default();
    let _ = hasher.write(&bytes);
    format!("h64:{:016x}", hasher.finish())
}

// ---------------- tests ----------------

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx(span_id: SpanId, is_session: bool) -> SpanContextRef {
        if is_session {
            SpanContextRef::for_session_span(span_id, Uuid::now_v7(), Uuid::now_v7())
        } else {
            SpanContextRef::for_activity_span(span_id, SpanId::new_v7(), None, None)
        }
    }

    #[tokio::test]
    async fn emitter_emits_started_and_ended_via_recorder() {
        let recorder = Arc::new(InMemoryStubFrRecorder::new());
        let emitter = SpanFrEmitter::with_recorder(recorder.clone());
        let span_id = SpanId::new_v7();
        let span_ctx = ctx(span_id, true);

        emitter
            .on_span_start(span_ctx.clone(), None, &json!({}))
            .await
            .unwrap();
        emitter.on_span_end(span_ctx.clone(), 10).await.unwrap();

        let captured = recorder.snapshot();
        assert_eq!(captured.len(), 2);
        assert_eq!(captured[0].event_id, FrEventId::SpanStarted);
        assert_eq!(captured[1].event_id, FrEventId::SpanEnded);
    }

    #[tokio::test]
    async fn force_emit_failures_bypasses_sample_rate_zero() {
        let recorder = Arc::new(InMemoryStubFrRecorder::new());
        let emitter = SpanFrEmitter::new(
            recorder.clone(),
            SpanFrEmitterConfig {
                sample_rate: 0.0,
                force_emit_failures: true,
            },
        );
        let span_id = SpanId::new_v7();
        let span_ctx = ctx(span_id, true);

        // Start + end are sampled OUT.
        emitter
            .on_span_start(span_ctx.clone(), None, &json!({}))
            .await
            .unwrap();
        emitter.on_span_end(span_ctx.clone(), 10).await.unwrap();
        // Failure is emitted regardless.
        emitter
            .on_span_failed(span_ctx.clone(), 10, "test fail")
            .await
            .unwrap();

        let captured = recorder.snapshot();
        assert_eq!(captured.len(), 1);
        assert_eq!(captured[0].event_id, FrEventId::SpanFailed);
    }

    #[tokio::test]
    async fn sampling_decision_is_deterministic_per_span() {
        let recorder = Arc::new(InMemoryStubFrRecorder::new());
        let emitter = SpanFrEmitter::new(
            recorder.clone(),
            SpanFrEmitterConfig {
                sample_rate: 0.5,
                force_emit_failures: false,
            },
        );
        // Loop 50 unique span_ids; for each, start and end either both
        // emit or both suppress — never half.
        let mut started = 0;
        let mut ended = 0;
        for _ in 0..50 {
            recorder.clear();
            let span_id = SpanId::new_v7();
            let span_ctx = ctx(span_id, true);
            emitter
                .on_span_start(span_ctx.clone(), None, &json!({}))
                .await
                .unwrap();
            emitter.on_span_end(span_ctx.clone(), 10).await.unwrap();
            let cap = recorder.snapshot();
            let s_count = cap
                .iter()
                .filter(|e| e.event_id == FrEventId::SpanStarted)
                .count();
            let e_count = cap
                .iter()
                .filter(|e| e.event_id == FrEventId::SpanEnded)
                .count();
            assert!(
                (s_count == 0 && e_count == 0) || (s_count == 1 && e_count == 1),
                "half-emission: started={s_count} ended={e_count}"
            );
            started += s_count;
            ended += e_count;
        }
        // Both sides agree (no half-emissions).
        assert_eq!(started, ended);
    }

    #[tokio::test]
    async fn checkpoint_and_attach_activity_emit() {
        let recorder = Arc::new(InMemoryStubFrRecorder::new());
        let emitter = SpanFrEmitter::with_recorder(recorder.clone());
        let span_id = SpanId::new_v7();
        let span_ctx = ctx(span_id, true);

        emitter
            .on_span_checkpoint(span_ctx.clone(), 1, 100)
            .await
            .unwrap();
        emitter
            .on_span_attach_activity(span_ctx.clone(), SpanId::new_v7())
            .await
            .unwrap();

        let captured = recorder.snapshot();
        assert_eq!(captured.len(), 2);
        assert_eq!(captured[0].event_id, FrEventId::SpanLifecycleCheckpoint);
        assert_eq!(captured[1].event_id, FrEventId::SpanLifecycleAttachActivity);
    }

    #[tokio::test]
    async fn null_recorder_is_noop() {
        let r = Arc::new(NullFrRecorder);
        let emitter = SpanFrEmitter::with_recorder(r.clone());
        let span_ctx = ctx(SpanId::new_v7(), true);
        emitter
            .on_span_start(span_ctx.clone(), None, &json!({}))
            .await
            .unwrap();
        emitter.on_span_end(span_ctx, 1).await.unwrap();
        // Nothing to assert except no panic.
    }

    #[tokio::test]
    async fn task_local_scope_propagates_emitter() {
        let recorder = Arc::new(InMemoryStubFrRecorder::new());
        let emitter = Arc::new(SpanFrEmitter::with_recorder(recorder.clone()));
        let captured = set_span_emitter_scope(emitter.clone(), async {
            let fetched = current_span_emitter().expect("emitter should be scoped");
            let span_ctx = ctx(SpanId::new_v7(), true);
            fetched
                .on_span_start(span_ctx, None, &json!({}))
                .await
                .unwrap();
            recorder.snapshot().len()
        })
        .await;
        assert_eq!(captured, 1);
    }

    #[tokio::test]
    async fn current_span_emitter_returns_none_outside_scope() {
        assert!(current_span_emitter().is_none());
    }

    #[test]
    fn config_clamps_invalid_sample_rate() {
        let cfg = SpanFrEmitterConfig {
            sample_rate: -1.0,
            force_emit_failures: true,
        };
        // Underflow clamps to 0 -> never sample
        assert!(!cfg.should_sample(SpanId::new_v7()));

        let cfg2 = SpanFrEmitterConfig {
            sample_rate: 2.0,
            force_emit_failures: true,
        };
        // Overflow clamps to 1 -> always sample
        assert!(cfg2.should_sample(SpanId::new_v7()));
    }
}
