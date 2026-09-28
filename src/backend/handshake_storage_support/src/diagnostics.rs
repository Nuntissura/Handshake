//! Sanitized observations enabled only inside a document request scope.
use std::{
    future::IntoFuture,
    sync::atomic::{AtomicU64, Ordering},
    time::Instant,
};
tokio::task_local! { pub static DOCUMENT_REQUEST_ID: String; static PARENT_OBSERVATION: u64; }
static NEXT_OBSERVATION: AtomicU64 = AtomicU64::new(1);

struct Observation {
    request_id: String,
    phase: &'static str,
    id: u64,
    parent: u64,
    count: u64,
    started: Instant,
    completed: bool,
}
impl Observation {
    fn emit(&self, event: &'static str) {
        tracing::info!(target: "handshake_core::knowledge_documents_api",
            request_id = %self.request_id, phase = self.phase, observation_id = self.id,
            parent_observation_id = self.parent, count = self.count, event,
            elapsed_ms = self.started.elapsed().as_millis() as u64,
            "MT032_DOCUMENT_PHASE");
    }
}
impl Drop for Observation {
    fn drop(&mut self) {
        if !self.completed {
            self.emit("dropped");
        }
    }
}

/// `dropped` means no result was observed, not that engine work rolled back.
/// Captured correlation remains available when task-local scopes unwind.
pub async fn observe<T>(
    phase: &'static str,
    count: u64,
    future: impl IntoFuture<Output = T>,
    failed: impl FnOnce(&T) -> bool,
) -> T {
    let Ok(request_id) = DOCUMENT_REQUEST_ID.try_with(Clone::clone) else {
        return future.await;
    };
    let mut observation = Observation {
        request_id,
        phase,
        id: NEXT_OBSERVATION.fetch_add(1, Ordering::Relaxed),
        parent: PARENT_OBSERVATION.try_with(|id| *id).unwrap_or(0),
        count,
        started: Instant::now(),
        completed: false,
    };
    observation.emit("begin");
    let result = PARENT_OBSERVATION
        .scope(observation.id, future.into_future())
        .await;
    observation.emit(if failed(&result) { "error" } else { "end" });
    observation.completed = true;
    result
}
pub async fn observe_result<T, E>(
    phase: &'static str,
    future: impl IntoFuture<Output = Result<T, E>>,
) -> Result<T, E> {
    observe(phase, 0, future, Result::is_err).await
}

/// Per-statement SDK timings, emitted only after a query response exists.
pub struct StatementTimingContext {
    request_id: String,
    query_observation_id: u64,
}

/// Inner receipt timings returned by the same authenticated SurrealQL statement.
/// These use the engine's UTC wall clock, not a monotonic clock. Failed or dropped
/// queries return no timings; negative deltas are retained and marked invalid.
pub struct ReceiptTimingContext {
    request_id: String,
    receipt_observation_id: u64,
}
impl ReceiptTimingContext {
    pub fn capture() -> Option<Self> {
        Some(Self {
            request_id: DOCUMENT_REQUEST_ID.try_with(Clone::clone).ok()?,
            receipt_observation_id: PARENT_OBSERVATION.try_with(|id| *id).ok()?,
        })
    }

    pub fn emit(
        &self,
        replay: bool,
        lookup_performed: bool,
        lookup_elapsed_us: i64,
        operation_elapsed_us: i64,
    ) {
        tracing::info!(target: "handshake_core::knowledge_documents_api",
            request_id = %self.request_id, receipt_observation_id = self.receipt_observation_id,
            branch = if replay { "replay" } else { "create" }, clock = "surreal_wall",
            timing_valid = lookup_elapsed_us >= 0 && operation_elapsed_us >= 0,
            lookup_performed, lookup_elapsed_us, operation_elapsed_us,
            "MT032_DOCUMENT_RECEIPT");
    }
}
impl StatementTimingContext {
    pub fn capture() -> Option<Self> {
        Some(Self {
            request_id: DOCUMENT_REQUEST_ID.try_with(Clone::clone).ok()?,
            query_observation_id: PARENT_OBSERVATION.try_with(|id| *id).ok()?,
        })
    }
    pub fn emit(
        &self,
        index: usize,
        statement_count: usize,
        duration: Option<std::time::Duration>,
    ) {
        tracing::info!(target: "handshake_core::knowledge_documents_api",
            request_id = %self.request_id, query_observation_id = self.query_observation_id,
            statement_index = index as u64, statement_count = statement_count as u64,
            timing_available = duration.is_some(),
            execution_time_us = duration.map(|value| u64::try_from(value.as_micros()).unwrap_or(u64::MAX)).unwrap_or(0),
            "MT032_DOCUMENT_STATEMENT");
    }
}
