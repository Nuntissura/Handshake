//! WP-1 live orchestration debug console: a NON-AUTHORITATIVE observability tee.
//!
//! ## What this is (and is NOT)
//!
//! This is a lightweight, in-process **display/stream buffer** that mirrors WP-1
//! orchestration diagnostics — model-lane launch/status changes, model
//! invocations, resource/breaker/lease events, and (via the public
//! [`ConsoleBroadcast::publish`] API) cloud-access/CLI-bridge login, process
//! reclaim, and pane events — into a live text stream an operator or a headless
//! model can tail over Server-Sent Events (`GET /wp1/diagnostics/console/stream`).
//!
//! It is **never** the durable authority. The authoritative record of every WP-1
//! orchestration event remains PostgreSQL/EventLedger + the Flight Recorder
//! ([`crate::flight_recorder`]); this console is an ADDITIONAL, best-effort tee
//! that is safe to drop, lag, or lose without affecting durable state. The
//! canonical wiring is [`ConsoleSwarmSink`], which composes alongside the durable
//! [`crate::swarm_orchestration::events::FlightRecorderSwarmSink`] inside a
//! [`crate::swarm_orchestration::events::FanoutSwarmSink`] — the durable sink runs
//! FIRST and its terminal-persistence rejection still propagates to the producer;
//! the console tee runs after and can never fail the coordinator.
//!
//! ## Design (field pattern)
//!
//! The hub is a [`tokio::sync::broadcast`] channel plus a bounded recent-history
//! ring. This mirrors the existing [`crate::swarm_orchestration::events::BroadcastSwarmSink`]
//! (the operator-board live source) and the well-worn "snapshot + live deltas"
//! shape: a new subscriber replays the last N entries on connect, then follows
//! live. A slow subscriber that lags the ring observes a `Lagged` notice rather
//! than a silent gap. Because it is a display buffer, a single process-wide hub
//! ([`ConsoleBroadcast::shared`]) is the natural home — like a global tracing
//! broadcaster — so every emission point and every reader share one stream
//! without threading a new field through the 30+ `AppState` construction sites.

use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
};

#[cfg(feature = "test-utils")]
use std::sync::Condvar;

use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;

use crate::swarm_orchestration::events::{SwarmEvent, SwarmEventSink};
use crate::swarm_orchestration::resource_scope::ExactResourceScopeAttribution;
use crate::swarm_orchestration::state::ModelSessionState;

/// Schema id stamped onto the SSE `event:` name so consumers can version the
/// wire shape independently of the payload.
pub const CONSOLE_ENTRY_SCHEMA_ID: &str = "hsk.wp1_console_entry@1";

/// Default live broadcast ring capacity (entries buffered per slow subscriber
/// before it observes `Lagged`).
pub const DEFAULT_CHANNEL_CAPACITY: usize = 1024;

/// Default bounded recent-history replayed to a subscriber on connect.
pub const DEFAULT_HISTORY_CAPACITY: usize = 256;

/// Severity of a console entry (display hint only; never authority).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConsoleSeverity {
    Debug,
    Info,
    Warn,
    Error,
}

impl ConsoleSeverity {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Debug => "debug",
            Self::Info => "info",
            Self::Warn => "warn",
            Self::Error => "error",
        }
    }
}

/// Which WP-1 orchestration surface a console entry describes. Stable strings so
/// the native pane and headless readers can filter by category without parsing
/// the free-text detail.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConsoleCategory {
    /// A model lane was spawned / launched.
    ModelLaneLaunch,
    /// A model lane changed status (ready, generating, cancelled, completed, failed).
    ModelLaneStatus,
    /// A model invocation started or finished (the ModelLaneMessage-bearing path).
    ModelInvocation,
    /// Concurrency-permit allocation / eviction.
    Resource,
    /// A circuit breaker tripped.
    Breaker,
    /// A checkout lease expired.
    Lease,
    /// A spawn was rejected (budget/loop cap).
    SpawnRejected,
    /// A promotion decision (validator/operator verdict on a lane message).
    Promotion,
    /// Cloud-access / official-CLI-bridge login activity.
    CloudAccess,
    /// Process reclaim / START / STOP ownership-ledger activity.
    Process,
    /// Window / pane lifecycle activity.
    Pane,
    /// Anything else (generic system observability).
    System,
}

impl ConsoleCategory {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ModelLaneLaunch => "model_lane_launch",
            Self::ModelLaneStatus => "model_lane_status",
            Self::ModelInvocation => "model_invocation",
            Self::Resource => "resource",
            Self::Breaker => "breaker",
            Self::Lease => "lease",
            Self::SpawnRejected => "spawn_rejected",
            Self::Promotion => "promotion",
            Self::CloudAccess => "cloud_access",
            Self::Process => "process",
            Self::Pane => "pane",
            Self::System => "system",
        }
    }
}

/// One structured console entry. `seq` is a monotonic per-hub id assigned at
/// publish time — it gives the native pane a STABLE row identity (author_id) and
/// lets a reader dedupe the connect-time replay against the live tail.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConsoleEntry {
    pub seq: u64,
    pub ts_unix_ms: u64,
    pub severity: ConsoleSeverity,
    pub category: ConsoleCategory,
    pub subject: String,
    pub detail: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trace_id: Option<String>,
    /// Exact durable-resource attribution. Account-facing readers fail closed
    /// when this is absent; `None` is reserved for explicit system-only sinks.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resource_scope: Option<ExactResourceScopeAttribution>,
}

/// The parts of a console entry a publisher provides; the hub stamps `seq` +
/// `ts_unix_ms`. Kept separate from [`ConsoleEntry`] so a caller can never forge
/// the monotonic sequence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConsoleEntryDraft {
    pub severity: ConsoleSeverity,
    pub category: ConsoleCategory,
    pub subject: String,
    pub detail: String,
    pub trace_id: Option<String>,
    pub resource_scope: Option<ExactResourceScopeAttribution>,
}

impl ConsoleEntryDraft {
    pub fn new(
        severity: ConsoleSeverity,
        category: ConsoleCategory,
        subject: impl Into<String>,
        detail: impl Into<String>,
        trace_id: Option<String>,
    ) -> Self {
        Self {
            severity,
            category,
            subject: subject.into(),
            detail: detail.into(),
            trace_id,
            resource_scope: None,
        }
    }

    pub fn with_resource_scope(mut self, scope: ExactResourceScopeAttribution) -> Self {
        self.resource_scope = Some(scope);
        self
    }
}

struct ConsoleBroadcastInner {
    tx: broadcast::Sender<ConsoleEntry>,
    publish_state: Mutex<ConsolePublishState>,
    history_capacity: usize,
    #[cfg(feature = "test-utils")]
    recent_snapshot_gate: Mutex<Option<Arc<ConsoleRecentSnapshotGateInner>>>,
}

struct ConsolePublishState {
    history: VecDeque<ConsoleEntry>,
    next_seq: u64,
}

#[cfg(feature = "test-utils")]
struct ConsoleRecentSnapshotGateInner {
    state: Mutex<(bool, bool)>,
    changed: Condvar,
}

/// Deterministic integration-test seam for the atomic subscribe+snapshot
/// boundary. The route holds the publication mutex while blocked, so a queued
/// publisher cannot enter between receiver creation and history capture.
#[cfg(feature = "test-utils")]
#[derive(Clone)]
pub struct ConsoleRecentSnapshotGate {
    inner: Arc<ConsoleRecentSnapshotGateInner>,
}

#[cfg(feature = "test-utils")]
impl ConsoleRecentSnapshotGate {
    pub fn wait_until_blocked(&self) {
        let mut state = self.inner.state.lock().expect("console gate state");
        while !state.0 {
            state = self.inner.changed.wait(state).expect("console gate wait");
        }
    }

    pub fn release(&self) {
        let mut state = self.inner.state.lock().expect("console gate state");
        state.1 = true;
        self.inner.changed.notify_all();
    }
}

/// The live console hub: a `tokio::sync::broadcast` sender plus a bounded
/// recent-history ring. Cheap to [`Clone`] (shares one `Arc` inner), so the SSE
/// route, the [`ConsoleSwarmSink`] tee, and any direct publisher all address the
/// same stream.
#[derive(Clone)]
pub struct ConsoleBroadcast {
    inner: Arc<ConsoleBroadcastInner>,
}

impl ConsoleBroadcast {
    /// Build a fresh, isolated hub. Production uses [`Self::shared`]; tests use
    /// this to get an isolated instance with a deterministic history bound.
    pub fn new(channel_capacity: usize, history_capacity: usize) -> Self {
        let (tx, _rx) = broadcast::channel(channel_capacity.max(1));
        Self {
            inner: Arc::new(ConsoleBroadcastInner {
                tx,
                publish_state: Mutex::new(ConsolePublishState {
                    history: VecDeque::with_capacity(history_capacity.max(1)),
                    next_seq: 0,
                }),
                history_capacity: history_capacity.max(1),
                #[cfg(feature = "test-utils")]
                recent_snapshot_gate: Mutex::new(None),
            }),
        }
    }

    /// The process-wide shared hub. This is a NON-AUTHORITATIVE display buffer, so
    /// a single global instance (like a global tracing broadcaster) is the natural
    /// home: every WP-1 emission point tees into it and the SSE route reads it,
    /// without threading a new field through every `AppState` construction site.
    pub fn shared() -> Self {
        static SHARED: once_cell::sync::Lazy<ConsoleBroadcast> = once_cell::sync::Lazy::new(|| {
            ConsoleBroadcast::new(DEFAULT_CHANNEL_CAPACITY, DEFAULT_HISTORY_CAPACITY)
        });
        SHARED.clone()
    }

    /// Publish a drafted entry: stamp the monotonic `seq` + timestamp, append to
    /// the bounded history ring, and broadcast to live subscribers. Sequence
    /// allocation, history insertion, and live send share one short critical
    /// section so concurrent publishers cannot expose different orders through
    /// replay and live reads. Returns the stamped entry. A full ring drops the
    /// OLDEST for slow subscribers (who observe `Lagged`), and `send` with no
    /// subscribers is a no-op.
    pub fn publish(&self, draft: ConsoleEntryDraft) -> ConsoleEntry {
        let mut state = self
            .inner
            .publish_state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let seq = state.next_seq;
        state.next_seq = state.next_seq.wrapping_add(1);
        let entry = ConsoleEntry {
            seq,
            ts_unix_ms: now_unix_ms(),
            severity: draft.severity,
            category: draft.category,
            subject: draft.subject,
            detail: draft.detail,
            trace_id: draft.trace_id,
            resource_scope: draft.resource_scope,
        };
        if state.history.len() == self.inner.history_capacity {
            state.history.pop_front();
        }
        state.history.push_back(entry.clone());
        // Ignore the no-subscribers error: an open stream is not required for the
        // durable record, only for live observation.
        let _ = self.inner.tx.send(entry.clone());
        entry
    }

    /// Convenience publisher used by non-swarm emission points (cloud login,
    /// process reclaim, pane events).
    pub fn publish_parts(
        &self,
        severity: ConsoleSeverity,
        category: ConsoleCategory,
        subject: impl Into<String>,
        detail: impl Into<String>,
        trace_id: Option<String>,
    ) -> ConsoleEntry {
        self.publish(ConsoleEntryDraft::new(
            severity, category, subject, detail, trace_id,
        ))
    }

    /// Subscribe to the live tail (the SSE route / tests).
    pub fn subscribe(&self) -> broadcast::Receiver<ConsoleEntry> {
        self.inner.tx.subscribe()
    }

    /// Atomically establish a live receiver and capture its replay prefix.
    /// Holding the same mutex used by `publish` makes the history snapshot a
    /// strict prefix and the receiver a strict suffix, eliminating overlap,
    /// loss, and replay/live reordering at connection time.
    pub fn subscribe_with_recent(
        &self,
        limit: usize,
    ) -> (broadcast::Receiver<ConsoleEntry>, Vec<ConsoleEntry>) {
        let state = self
            .inner
            .publish_state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let rx = self.inner.tx.subscribe();
        #[cfg(feature = "test-utils")]
        self.wait_on_recent_snapshot_gate();
        let len = state.history.len();
        let start = len.saturating_sub(limit);
        let replay = state.history.iter().skip(start).cloned().collect();
        (rx, replay)
    }

    /// The bounded recent history, oldest-first, capped at `limit` (most recent).
    /// Replayed to a subscriber on connect.
    pub fn recent(&self, limit: usize) -> Vec<ConsoleEntry> {
        #[cfg(feature = "test-utils")]
        self.wait_on_recent_snapshot_gate();
        let state = self
            .inner
            .publish_state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let len = state.history.len();
        let start = len.saturating_sub(limit);
        state.history.iter().skip(start).cloned().collect()
    }

    /// Pause the next replay snapshot after the route has subscribed. This is
    /// compiled only for real integration tests and has no production state or
    /// behavior when `test-utils` is disabled.
    #[cfg(feature = "test-utils")]
    pub fn arm_recent_snapshot_gate_for_tests(&self) -> ConsoleRecentSnapshotGate {
        let gate = ConsoleRecentSnapshotGate {
            inner: Arc::new(ConsoleRecentSnapshotGateInner {
                state: Mutex::new((false, false)),
                changed: Condvar::new(),
            }),
        };
        *self
            .inner
            .recent_snapshot_gate
            .lock()
            .expect("console recent gate") = Some(Arc::clone(&gate.inner));
        gate
    }

    #[cfg(feature = "test-utils")]
    fn wait_on_recent_snapshot_gate(&self) {
        if let Some(gate) = self
            .inner
            .recent_snapshot_gate
            .lock()
            .expect("console recent gate")
            .take()
        {
            let mut state = gate.state.lock().expect("console gate state");
            state.0 = true;
            gate.changed.notify_all();
            while !state.1 {
                state = gate.changed.wait(state).expect("console gate wait");
            }
        }
    }

    /// Current live subscriber count (0 = nobody tailing; publish is a cheap no-op).
    pub fn receiver_count(&self) -> usize {
        self.inner.tx.receiver_count()
    }
}

/// A [`SwarmEventSink`] that tees each [`SwarmEvent`] into a [`ConsoleBroadcast`]
/// as a structured [`ConsoleEntry`]. NON-AUTHORITATIVE: `emit` always returns
/// `Ok(())` so it can never fail the coordinator; the durable Flight Recorder
/// sink is the authority. Compose it AFTER the durable sink in a
/// [`crate::swarm_orchestration::events::FanoutSwarmSink`].
pub struct ConsoleSwarmSink {
    hub: ConsoleBroadcast,
    resource_scope: Option<ExactResourceScopeAttribution>,
}

impl ConsoleSwarmSink {
    /// Explicit system-only sink. Entries emitted here are deliberately
    /// invisible to account-facing SSE readers because they carry no scope.
    pub fn new(hub: ConsoleBroadcast) -> Self {
        Self {
            hub,
            resource_scope: None,
        }
    }

    pub fn new_scoped(
        hub: ConsoleBroadcast,
        resource_scope: ExactResourceScopeAttribution,
    ) -> Self {
        Self {
            hub,
            resource_scope: Some(resource_scope),
        }
    }

    /// The production tee: bound to the process-wide shared hub.
    pub fn shared() -> Self {
        Self::new(ConsoleBroadcast::shared())
    }
}

impl SwarmEventSink for ConsoleSwarmSink {
    fn emit(&self, event: SwarmEvent) -> Result<(), String> {
        let mut draft = console_draft_for_swarm_event(&event);
        draft.resource_scope = self.resource_scope.clone();
        self.hub.publish(draft);
        Ok(())
    }
}

/// Map a [`SwarmEvent`] to a structured console draft. Pure + total so it is
/// unit-testable without a running coordinator.
pub fn console_draft_for_swarm_event(event: &SwarmEvent) -> ConsoleEntryDraft {
    match event {
        SwarmEvent::SessionSpawned {
            instance_id,
            parent_session_id,
            process_uuid,
            swarm_id,
            worktree_id,
        } => ConsoleEntryDraft::new(
            ConsoleSeverity::Info,
            ConsoleCategory::ModelLaneLaunch,
            instance_id.to_string(),
            format!(
                "lane spawned (parent={parent_session_id}, process={process_uuid}, swarm={}, worktree={})",
                swarm_id.as_deref().unwrap_or("none"),
                worktree_id.as_deref().unwrap_or("none"),
            ),
            None,
        ),
        SwarmEvent::SessionReady { instance_id } => ConsoleEntryDraft::new(
            ConsoleSeverity::Info,
            ConsoleCategory::ModelLaneStatus,
            instance_id.to_string(),
            "lane ready".to_string(),
            None,
        ),
        SwarmEvent::SessionStateChanged {
            instance_id,
            from,
            to,
        } => ConsoleEntryDraft::new(
            severity_for_state(*to),
            ConsoleCategory::ModelLaneStatus,
            instance_id.to_string(),
            format!("state {} -> {}", from.as_str(), to.as_str()),
            None,
        ),
        SwarmEvent::ModelInvocationStarted {
            instance_id,
            trace_id,
            run_id,
            session_id,
            max_tokens,
        } => ConsoleEntryDraft::new(
            ConsoleSeverity::Info,
            ConsoleCategory::ModelInvocation,
            instance_id.to_string(),
            format!("invocation started (run={run_id}, session={session_id}, max_tokens={max_tokens})"),
            Some(trace_id.to_string()),
        ),
        SwarmEvent::ModelInvocationFinished {
            instance_id,
            trace_id,
            run_id,
            session_id,
            outcome,
            generated_tokens,
            error,
        } => ConsoleEntryDraft::new(
            if outcome == "failed" {
                ConsoleSeverity::Error
            } else {
                ConsoleSeverity::Info
            },
            ConsoleCategory::ModelInvocation,
            instance_id.to_string(),
            format!(
                "invocation finished (run={run_id}, session={session_id}, outcome={outcome}, tokens={generated_tokens}, error={})",
                error.as_deref().map(redact_secrets).unwrap_or_else(|| "none".to_string()),
            ),
            Some(trace_id.to_string()),
        ),
        SwarmEvent::SessionCancelled {
            instance_id,
            reason,
            ..
        } => ConsoleEntryDraft::new(
            ConsoleSeverity::Warn,
            ConsoleCategory::ModelLaneStatus,
            instance_id.to_string(),
            format!("lane cancelled: {}", redact_secrets(reason)),
            None,
        ),
        SwarmEvent::SessionCompleted { instance_id, .. } => ConsoleEntryDraft::new(
            ConsoleSeverity::Info,
            ConsoleCategory::ModelLaneStatus,
            instance_id.to_string(),
            "lane completed".to_string(),
            None,
        ),
        SwarmEvent::SessionFailed {
            instance_id, error, ..
        } => ConsoleEntryDraft::new(
            ConsoleSeverity::Error,
            ConsoleCategory::ModelLaneStatus,
            instance_id.to_string(),
            format!("lane failed: {}", redact_secrets(error)),
            None,
        ),
        SwarmEvent::ResourceAllocated {
            instance_id,
            permits_in_use,
            permits_cap,
        } => ConsoleEntryDraft::new(
            ConsoleSeverity::Info,
            ConsoleCategory::Resource,
            instance_id.to_string(),
            format!("resource allocated ({permits_in_use}/{permits_cap} permits in use)"),
            None,
        ),
        SwarmEvent::ResourceEvicted {
            instance_id,
            terminal_state,
            ..
        } => ConsoleEntryDraft::new(
            ConsoleSeverity::Info,
            ConsoleCategory::Resource,
            instance_id.to_string(),
            format!("resource evicted (terminal state {})", terminal_state.as_str()),
            None,
        ),
        SwarmEvent::BreakerTripped {
            signature,
            consecutive_failures,
        } => ConsoleEntryDraft::new(
            ConsoleSeverity::Warn,
            ConsoleCategory::Breaker,
            signature.clone(),
            format!("circuit breaker tripped after {consecutive_failures} consecutive failures"),
            None,
        ),
        SwarmEvent::LeaseExpired { instance_id, owner } => ConsoleEntryDraft::new(
            ConsoleSeverity::Warn,
            ConsoleCategory::Lease,
            instance_id.to_string(),
            format!("checkout lease expired (owner={owner})"),
            None,
        ),
        SwarmEvent::SpawnRejected {
            instance_id,
            reason,
        } => ConsoleEntryDraft::new(
            ConsoleSeverity::Warn,
            ConsoleCategory::SpawnRejected,
            instance_id.to_string(),
            format!("spawn rejected: {}", redact_secrets(reason)),
            None,
        ),
    }
}

/// Redact likely secrets from provider-originated free text before it enters the
/// NON-authoritative console stream. The same text is already stored in the
/// authenticated Flight Recorder / EventLedger; this only scrubs the widened
/// loopback-stream exposure flagged by the WP-1 console adversarial review. UUIDs and
/// short ids (< 40 chars) are preserved so the stream stays diagnostic.
pub(crate) fn redact_secrets(text: &str) -> String {
    use once_cell::sync::Lazy;
    use regex::Regex;
    static BEARER: Lazy<Regex> =
        Lazy::new(|| Regex::new(r"(?i)\b(bearer|authorization:?)\s+\S+").unwrap());
    static QUERY: Lazy<Regex> = Lazy::new(|| {
        Regex::new(
            r"(?i)([?&](?:api[-_]?key|access[-_]?token|token|secret|password|sig|signature)=)[^&\s]+",
        )
        .unwrap()
    });
    static KEYISH: Lazy<Regex> = Lazy::new(|| {
        Regex::new(r"(?i)\b(?:sk|pk|rk|api|ghp|xox[baprs])[-_][A-Za-z0-9]{12,}\b").unwrap()
    });
    static LONGTOK: Lazy<Regex> = Lazy::new(|| Regex::new(r"\b[A-Za-z0-9_\-]{40,}\b").unwrap());
    let s = BEARER.replace_all(text, "$1 [redacted]");
    let s = QUERY.replace_all(&s, "$1[redacted]");
    let s = KEYISH.replace_all(&s, "[redacted-key]");
    LONGTOK.replace_all(&s, "[redacted-token]").into_owned()
}

fn severity_for_state(state: ModelSessionState) -> ConsoleSeverity {
    match state {
        ModelSessionState::Failed => ConsoleSeverity::Error,
        ModelSessionState::Cancelled => ConsoleSeverity::Warn,
        _ => ConsoleSeverity::Info,
    }
}

fn now_unix_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| u64::try_from(duration.as_millis()).unwrap_or(u64::MAX))
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::swarm_orchestration::ids::ModelInstanceId;

    fn instance() -> ModelInstanceId {
        ModelInstanceId::new(crate::model_runtime::ModelId::new_v7(), 0)
    }

    #[test]
    fn redact_secrets_scrubs_tokens_but_keeps_ids() {
        // Bearer / Authorization material must not reach the loopback stream.
        let bearer = redact_secrets("401 Authorization: Bearer sk-abcdEFGH1234567890xyz");
        assert!(!bearer.contains("sk-abcdEFGH1234567890xyz"), "{bearer}");
        assert!(bearer.contains("[redacted"), "{bearer}");
        // URL query secrets.
        let q = redact_secrets(
            "GET https://api.example.com/v1?api_key=SUPERSECRETKEYVALUE1234&x=1 -> 500",
        );
        assert!(!q.contains("SUPERSECRETKEYVALUE1234"), "{q}");
        // Key-prefixed token.
        let k = redact_secrets("provider rejected key sk-0123456789abcdefABCDEF");
        assert!(!k.contains("0123456789abcdefABCDEF"), "{k}");
        // Long high-entropy token (>= 40 chars).
        let t = redact_secrets("token eyJhbGciOiJIUzI1NiIsInR5cCJ9AAAAAAAAAAAAAAAA leaked");
        assert!(t.contains("[redacted-token]"), "{t}");
        // A UUID (model/instance/run id) stays intact so the stream is still diagnostic.
        let id = "019fad78-453c-79b2-ac79-267206424bd6";
        assert_eq!(
            redact_secrets(&format!("lane failed: {id}")),
            format!("lane failed: {id}")
        );
    }

    #[test]
    fn publish_stamps_monotonic_seq_and_appends_history() {
        let hub = ConsoleBroadcast::new(16, 8);
        let a = hub.publish_parts(
            ConsoleSeverity::Info,
            ConsoleCategory::ModelLaneLaunch,
            "lane-1",
            "spawned",
            None,
        );
        let b = hub.publish_parts(
            ConsoleSeverity::Warn,
            ConsoleCategory::Breaker,
            "sig-1",
            "tripped",
            None,
        );
        assert_eq!(a.seq, 0);
        assert_eq!(b.seq, 1, "seq is monotonic across publishes");
        let recent = hub.recent(16);
        assert_eq!(recent.len(), 2);
        assert_eq!(recent[0], a, "history is oldest-first");
        assert_eq!(recent[1], b);
    }

    #[test]
    fn recent_history_is_bounded_to_capacity() {
        let hub = ConsoleBroadcast::new(64, 4);
        for i in 0..10 {
            hub.publish_parts(
                ConsoleSeverity::Info,
                ConsoleCategory::System,
                format!("subject-{i}"),
                format!("detail-{i}"),
                None,
            );
        }
        let recent = hub.recent(100);
        assert_eq!(recent.len(), 4, "ring keeps only the last N entries");
        // The oldest retained is entry #6 (0..10 published, capacity 4).
        assert_eq!(recent[0].seq, 6);
        assert_eq!(recent[3].seq, 9, "newest retained is the last published");
        // seq is still globally monotonic even though older entries were dropped.
        assert_eq!(recent[0].subject, "subject-6");
    }

    #[test]
    fn recent_limit_returns_the_most_recent_slice() {
        let hub = ConsoleBroadcast::new(64, 64);
        for i in 0..10 {
            hub.publish_parts(
                ConsoleSeverity::Info,
                ConsoleCategory::System,
                format!("s{i}"),
                "d",
                None,
            );
        }
        let recent = hub.recent(3);
        assert_eq!(recent.len(), 3);
        assert_eq!(recent[0].seq, 7);
        assert_eq!(recent[2].seq, 9);
    }

    #[tokio::test]
    async fn subscriber_receives_live_published_entries_in_order() {
        let hub = ConsoleBroadcast::new(16, 16);
        let mut rx = hub.subscribe();
        assert_eq!(hub.receiver_count(), 1);
        let first = hub.publish_parts(
            ConsoleSeverity::Info,
            ConsoleCategory::ModelLaneLaunch,
            "lane-9",
            "spawned",
            None,
        );
        let second = hub.publish_parts(
            ConsoleSeverity::Error,
            ConsoleCategory::ModelLaneStatus,
            "lane-9",
            "failed",
            None,
        );
        assert_eq!(rx.recv().await.expect("first live entry"), first);
        assert_eq!(rx.recv().await.expect("second live entry"), second);
    }

    #[test]
    fn console_sink_tees_swarm_event_into_hub() {
        let hub = ConsoleBroadcast::new(16, 16);
        let sink = ConsoleSwarmSink::new(hub.clone());
        let iid = instance();
        sink.emit(SwarmEvent::SessionReady { instance_id: iid })
            .expect("console tee never errors");
        sink.emit(SwarmEvent::SessionFailed {
            instance_id: iid,
            error: "boom".to_string(),
            event_id: None,
        })
        .expect("console tee never errors");
        let recent = hub.recent(16);
        assert_eq!(recent.len(), 2);
        assert_eq!(recent[0].category, ConsoleCategory::ModelLaneStatus);
        assert_eq!(recent[0].severity, ConsoleSeverity::Info);
        assert_eq!(recent[0].detail, "lane ready");
        assert_eq!(recent[1].severity, ConsoleSeverity::Error);
        assert!(recent[1].detail.contains("boom"));
    }

    #[test]
    fn invocation_events_carry_trace_id_and_category() {
        let started = console_draft_for_swarm_event(&SwarmEvent::ModelInvocationStarted {
            instance_id: instance(),
            trace_id: uuid::Uuid::now_v7(),
            run_id: "run-1".to_string(),
            session_id: "session-1".to_string(),
            max_tokens: 128,
        });
        assert_eq!(started.category, ConsoleCategory::ModelInvocation);
        assert!(
            started.trace_id.is_some(),
            "invocation carries the trace id"
        );

        let finished = console_draft_for_swarm_event(&SwarmEvent::ModelInvocationFinished {
            instance_id: instance(),
            trace_id: uuid::Uuid::now_v7(),
            run_id: "run-1".to_string(),
            session_id: "session-1".to_string(),
            outcome: "failed".to_string(),
            generated_tokens: 3,
            error: Some("provider failed".to_string()),
        });
        assert_eq!(finished.severity, ConsoleSeverity::Error);
        assert!(finished.detail.contains("provider failed"));
    }

    #[test]
    fn entry_round_trips_through_json() {
        let hub = ConsoleBroadcast::new(8, 8);
        let entry = hub.publish_parts(
            ConsoleSeverity::Warn,
            ConsoleCategory::Lease,
            "lane-x",
            "lease expired",
            Some("trace-1".to_string()),
        );
        let json = serde_json::to_string(&entry).expect("serialize");
        let back: ConsoleEntry = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(entry, back);
        // The wire form uses stable snake_case category/severity strings.
        assert!(json.contains("\"category\":\"lease\""));
        assert!(json.contains("\"severity\":\"warn\""));
    }
}
