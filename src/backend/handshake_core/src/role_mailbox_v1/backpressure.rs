//! MT-182 Mailbox backpressure (per-role inbox cap + token-bucket rate limit)
//! with FR-EVT-MAILBOX-BACKPRESSURE degraded-mode receipt.
//!
//! Per spec v02.186 §05-security-and-observability.md §5.7.3 bypass-on-overflow
//! pattern (the same shape used by ProcessOwnershipLedger / FR-EVT-LEDGER-OVERFLOW):
//! when a role inbox is saturated or a per-role rate limit trips, the caller
//! receives a typed `BackpressureDecision::Deny` carrying a
//! [`BackpressureReceipt`] (degraded-mode envelope) and the guard emits a
//! `FR-EVT-MAILBOX-BACKPRESSURE` Flight Recorder event. The message is NEVER
//! silently dropped: callers must propagate the Deny back to the sender so the
//! sender can back off, retry, or surface the saturation to the operator.
//!
//! Configuration is loaded from the `kernel.role_mailbox.backpressure` section
//! of the Work Profile (forward-compat for the KERNEL-005 profile layer); until
//! the profile reads land, the [`BackpressureConfig::default`] constants are
//! used (inbox_cap = 256, tokens_per_second = 8, burst_capacity = 32). Defaults
//! are documented in code and link to the spec line cited above.
//!
//! Red-team minimum controls (see MT-182.json `red_team.minimum_controls`):
//!  1. Sampled FR-EVT-MAILBOX-BACKPRESSURE Allow-path emissions are supported
//!     via [`BackpressureGuard::with_allow_sample_rate`] so saturation trends
//!     surface before the cap trips.
//!  2. The token bucket consumes time via the injectable [`BackpressureClock`]
//!     trait. Production wires [`SystemClock`]; tests inject a deterministic
//!     [`FixedClock`] so race + boundary assertions are reproducible.
//!  3. The default config constants are linked to spec §5.7.3 in this module's
//!     header and on [`BackpressureConfig::default`].

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use thiserror::Error;
use uuid::Uuid;

use crate::flight_recorder::fr_event_registry::FrEventId;
use crate::flight_recorder::{
    FlightRecorder, FlightRecorderActor, FlightRecorderEvent, FlightRecorderEventType,
    RecorderError,
};
use crate::role_mailbox::RoleId;

use super::repo::{MailboxError, RoleMailboxRepository};

/// Per-role inbox cap + per-role rate-limit configuration.
///
/// Defaults align with spec v02.186 §05-security-and-observability.md §5.7.3
/// (bypass-on-overflow). `inbox_cap = 256` matches the default mailbox depth
/// budget per role. `tokens_per_second = 8` + `burst_capacity = 32` matches
/// the default send-side budget for a single role under the MT-182 contract.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BackpressureConfig {
    /// Maximum simultaneously-pending messages (delivery_state in
    /// {queued, delivered}) for a single `to_role`. When the observed pending
    /// count reaches `inbox_cap`, new sends to that role return a typed
    /// `Deny { reason: InboxFull, ... }`.
    pub inbox_cap: u32,
    /// Token bucket refill rate (tokens per real-clock second). Setting this
    /// to 0 disables refill — used by `zero_rate_starves_all_sends` to assert
    /// the degenerate boundary.
    pub tokens_per_second: u32,
    /// Maximum burst the token bucket can absorb at instantaneous t=0. Also
    /// the initial-fill amount on bucket creation. Setting this to 0 disables
    /// the rate-limit path entirely (every check via the bucket denies).
    pub burst_capacity: u32,
}

impl Default for BackpressureConfig {
    /// Spec §5.7.3 default budgets.
    fn default() -> Self {
        Self {
            inbox_cap: 256,
            tokens_per_second: 8,
            burst_capacity: 32,
        }
    }
}

/// Typed deny reason recorded in the degraded-mode receipt and in the
/// FR-EVT-MAILBOX-BACKPRESSURE `policy` field.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DenyReason {
    /// Pending message count reached `inbox_cap`.
    InboxFull,
    /// Token bucket exhausted for this role.
    RateLimitExceeded,
}

impl DenyReason {
    /// Canonical wire string emitted in the FR-EVT payload `policy` field.
    pub fn as_str(self) -> &'static str {
        match self {
            DenyReason::InboxFull => "inbox_full",
            DenyReason::RateLimitExceeded => "rate_limit_exceeded",
        }
    }
}

/// Top-level decision shape returned from [`BackpressureGuard::check`].
///
/// `Allow` means the caller may proceed to append the message to the role
/// inbox. `Deny` carries a [`BackpressureReceipt`] the caller MUST surface
/// back to the sender — silent drops violate the MT-182 contract and the
/// bypass-on-overflow spec pattern.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "outcome", rename_all = "snake_case")]
pub enum BackpressureDecision {
    Allow,
    Deny {
        reason: DenyReason,
        retry_after_secs: u32,
        observed_pending: u32,
    },
}

/// Degraded-mode receipt envelope returned on `Deny`. The receipt is the
/// caller-visible product surface (sender sees a typed object, not a silent
/// drop). It mirrors the FR-EVT-MAILBOX-BACKPRESSURE payload one-to-one so
/// observability and reply surfaces stay in sync.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BackpressureReceipt {
    pub schema_version: String,
    pub receipt_id: Uuid,
    pub role_id: String,
    pub reason: DenyReason,
    pub retry_after_secs: u32,
    pub observed_pending: u32,
    pub observed_at_utc: DateTime<Utc>,
    /// Wire-stable canonical event id (matches FrEventId::MailboxBackpressure).
    pub fr_event_id: String,
}

impl BackpressureReceipt {
    pub const SCHEMA_VERSION: &'static str = "hsk.role_mailbox.backpressure_receipt@1";

    pub fn from_decision(
        role_id: &RoleId,
        decision: &BackpressureDecision,
        now: DateTime<Utc>,
    ) -> Option<Self> {
        match decision {
            BackpressureDecision::Allow => None,
            BackpressureDecision::Deny {
                reason,
                retry_after_secs,
                observed_pending,
            } => Some(Self {
                schema_version: Self::SCHEMA_VERSION.to_string(),
                receipt_id: Uuid::now_v7(),
                role_id: role_id.to_string(),
                reason: *reason,
                retry_after_secs: *retry_after_secs,
                observed_pending: *observed_pending,
                observed_at_utc: now,
                fr_event_id: FrEventId::MailboxBackpressure.as_str().to_string(),
            }),
        }
    }
}

/// Errors surfaced by [`BackpressureGuard::check_via_repo`] when the pending
/// count query against the MT-177 repo fails.
#[derive(Debug, Error)]
pub enum BackpressureError {
    #[error("inbox query failed: {0}")]
    InboxQuery(#[from] MailboxError),
}

/// Injectable clock for the token bucket. Production wires
/// [`SystemClock`]; tests use [`FixedClock`] to make race + boundary
/// assertions deterministic.
pub trait BackpressureClock: Send + Sync {
    fn now(&self) -> DateTime<Utc>;
}

/// Production clock — calls `Utc::now`.
#[derive(Debug, Default, Clone)]
pub struct SystemClock;

impl BackpressureClock for SystemClock {
    fn now(&self) -> DateTime<Utc> {
        Utc::now()
    }
}

/// Deterministic clock for tests. The held instant can be advanced via
/// [`FixedClock::set`]; cloning shares the inner cell so multiple subsystems
/// see the same advance.
#[derive(Debug, Clone)]
pub struct FixedClock {
    inner: Arc<Mutex<DateTime<Utc>>>,
}

impl FixedClock {
    pub fn new(t0: DateTime<Utc>) -> Self {
        Self {
            inner: Arc::new(Mutex::new(t0)),
        }
    }

    pub fn set(&self, t: DateTime<Utc>) {
        *self.inner.lock().unwrap() = t;
    }

    pub fn advance(&self, delta: chrono::Duration) {
        let mut g = self.inner.lock().unwrap();
        *g += delta;
    }
}

impl BackpressureClock for FixedClock {
    fn now(&self) -> DateTime<Utc> {
        *self.inner.lock().unwrap()
    }
}

#[derive(Debug, Clone)]
struct TokenBucket {
    tokens: f64,
    last_refill_utc: DateTime<Utc>,
}

impl TokenBucket {
    fn new(burst: u32, now: DateTime<Utc>) -> Self {
        Self {
            tokens: burst as f64,
            last_refill_utc: now,
        }
    }

    fn refill(&mut self, cfg: &BackpressureConfig, now: DateTime<Utc>) {
        let elapsed_secs = (now - self.last_refill_utc).num_milliseconds() as f64 / 1000.0;
        if elapsed_secs > 0.0 {
            let refill = elapsed_secs * cfg.tokens_per_second as f64;
            self.tokens = (self.tokens + refill).min(cfg.burst_capacity as f64);
            self.last_refill_utc = now;
        }
    }

    fn try_consume(&mut self, cfg: &BackpressureConfig, now: DateTime<Utc>) -> bool {
        self.refill(cfg, now);
        if self.tokens >= 1.0 {
            self.tokens -= 1.0;
            true
        } else {
            false
        }
    }

    fn retry_after_secs(&self, cfg: &BackpressureConfig) -> u32 {
        if cfg.tokens_per_second == 0 {
            return u32::MAX;
        }
        let needed = (1.0_f64 - self.tokens).max(0.0);
        // Ceil to the next whole second so the retry-after is always
        // strictly >= the wait the bucket actually requires.
        ((needed / cfg.tokens_per_second as f64).ceil() as u32).max(1)
    }
}

/// Per-role inbox cap + token-bucket rate-limit gate.
///
/// The guard is thread-safe via an internal `Mutex<HashMap<role_str,
/// TokenBucket>>` (one entry per observed role). Allocation is amortised:
/// the first send to a role allocates one bucket and reuses it thereafter.
///
/// Construct via [`BackpressureGuard::new`] for an in-process gate. To enable
/// FR-EVT-MAILBOX-BACKPRESSURE emission on Deny (and optional sampled Allow
/// emission per red_team #1), call [`BackpressureGuard::with_recorder`].
pub struct BackpressureGuard {
    cfg: BackpressureConfig,
    buckets: Mutex<HashMap<String, TokenBucket>>,
    /// FlightRecorder used to emit FR-EVT-MAILBOX-BACKPRESSURE. None disables
    /// emission entirely (the legacy in-process integration path).
    recorder: Option<Arc<dyn FlightRecorder>>,
    /// Allow-path FR sampling rate: emit one FR event per `allow_sample_rate`
    /// Allow decisions so trend monitoring is possible before the cap trips
    /// (red_team minimum_controls #1). 0 disables Allow-path emission.
    allow_sample_rate: u32,
    /// Monotonic Allow counter used for deterministic sampling.
    allow_counter: AtomicU64,
    /// Injectable clock — production wires `Arc<SystemClock>`, tests inject
    /// `Arc<FixedClock>` (red_team minimum_controls #2).
    clock: Arc<dyn BackpressureClock>,
}

impl BackpressureGuard {
    /// In-process gate with no FR emission and the default system clock.
    pub fn new(cfg: BackpressureConfig) -> Self {
        Self {
            cfg,
            buckets: Mutex::new(HashMap::new()),
            recorder: None,
            allow_sample_rate: 0,
            allow_counter: AtomicU64::new(0),
            clock: Arc::new(SystemClock),
        }
    }

    /// Inject a custom clock — required by tests that want deterministic
    /// token-bucket math (red_team minimum_controls #2).
    pub fn with_clock(mut self, clock: Arc<dyn BackpressureClock>) -> Self {
        self.clock = clock;
        self
    }

    /// Attach a FlightRecorder so Deny decisions emit
    /// FR-EVT-MAILBOX-BACKPRESSURE. Emission is async and non-blocking — a
    /// recorder failure does NOT change the Allow/Deny verdict and does NOT
    /// surface to the caller; the recorder is observability, not the
    /// decision authority.
    pub fn with_recorder(mut self, recorder: Arc<dyn FlightRecorder>) -> Self {
        self.recorder = Some(recorder);
        self
    }

    /// Configure Allow-path FR sampling — emit one FR-EVT-MAILBOX-BACKPRESSURE
    /// event per `rate` Allow decisions (0 disables sampling). Used by the
    /// red-team minimum_controls #1 trend-observability path.
    pub fn with_allow_sample_rate(mut self, rate: u32) -> Self {
        self.allow_sample_rate = rate;
        self
    }

    pub fn config(&self) -> BackpressureConfig {
        self.cfg
    }

    /// Check whether a send is allowed for `role_id`. `pending_count` is the
    /// caller-supplied pending-message count (from MT-177
    /// [`RoleMailboxRepository::count_pending_messages_for_role`]).
    ///
    /// Returns a [`BackpressureDecision`]. On Deny, callers MUST construct a
    /// [`BackpressureReceipt`] via [`BackpressureReceipt::from_decision`] and
    /// surface it back to the sender (no silent drops, per MT-182 contract
    /// and spec §5.7.3). When a FlightRecorder is attached via
    /// [`BackpressureGuard::with_recorder`], the Deny path emits
    /// FR-EVT-MAILBOX-BACKPRESSURE automatically.
    pub fn check(
        &self,
        role_id: &RoleId,
        pending_count: u32,
        now: DateTime<Utc>,
    ) -> BackpressureDecision {
        // (1) Inbox-cap path takes precedence over rate-limit — a saturated
        // inbox must reject regardless of bucket state, because the cap is
        // about the consumer side (Postgres / executor) not the sender side.
        if pending_count >= self.cfg.inbox_cap {
            let decision = BackpressureDecision::Deny {
                reason: DenyReason::InboxFull,
                retry_after_secs: 5,
                observed_pending: pending_count,
            };
            self.maybe_emit_fr(role_id, &decision, now);
            return decision;
        }
        // (2) Token-bucket path. Bucket entries are keyed by canonical role
        // string (`role_id.to_string()`), so the Advisory variant's id is
        // included in the key.
        let key = role_id.to_string();
        let mut buckets = self.buckets.lock().expect("buckets mutex");
        let bucket = buckets
            .entry(key)
            .or_insert_with(|| TokenBucket::new(self.cfg.burst_capacity, now));
        let decision = if bucket.try_consume(&self.cfg, now) {
            BackpressureDecision::Allow
        } else {
            BackpressureDecision::Deny {
                reason: DenyReason::RateLimitExceeded,
                retry_after_secs: bucket.retry_after_secs(&self.cfg),
                observed_pending: pending_count,
            }
        };
        drop(buckets); // release before async emit path
        self.maybe_emit_fr(role_id, &decision, now);
        decision
    }

    /// Async wrapper that queries the MT-177 repo for pending-message count
    /// and then checks backpressure. Returns the typed receipt on Deny so
    /// callers can return it to the sender unchanged.
    pub async fn check_via_repo(
        &self,
        repo: &RoleMailboxRepository,
        role_id: &RoleId,
    ) -> Result<(BackpressureDecision, Option<BackpressureReceipt>), BackpressureError> {
        let pending = repo.count_pending_messages_for_role(role_id).await?;
        let now = self.clock.now();
        let decision = self.check(role_id, pending, now);
        let receipt = BackpressureReceipt::from_decision(role_id, &decision, now);
        Ok((decision, receipt))
    }

    /// Internal: emit FR-EVT-MAILBOX-BACKPRESSURE for the decision. The Deny
    /// path always emits; the Allow path emits once per `allow_sample_rate`
    /// decisions so the sampled trend per red_team minimum_controls #1 is
    /// available without flooding the recorder.
    fn maybe_emit_fr(&self, role_id: &RoleId, decision: &BackpressureDecision, now: DateTime<Utc>) {
        let Some(recorder) = self.recorder.clone() else {
            return;
        };
        let should_emit = match decision {
            BackpressureDecision::Deny { .. } => true,
            BackpressureDecision::Allow => {
                if self.allow_sample_rate == 0 {
                    false
                } else {
                    let counter = self.allow_counter.fetch_add(1, Ordering::Relaxed);
                    counter % self.allow_sample_rate as u64 == 0
                }
            }
        };
        if !should_emit {
            return;
        }
        let (outcome_str, policy, queue_depth, retry_after) = match decision {
            BackpressureDecision::Allow => ("allow", "allow_sampled", 0_u64, 0_u32),
            BackpressureDecision::Deny {
                reason,
                retry_after_secs,
                observed_pending,
            } => (
                "deny",
                reason.as_str(),
                *observed_pending as u64,
                *retry_after_secs,
            ),
        };
        // FR-EVT-MAILBOX-BACKPRESSURE schema fields per
        // .GOV/roles_shared/records/FR_EVENT_REGISTRY.json:
        //   { role_id: string, queue_depth: u64, policy: string }
        // The MT-182 contract widens the payload with the canonical FR id,
        // outcome, retry_after_secs, and observed_at_utc so downstream
        // diagnostics can distinguish Allow-sample emissions from Deny
        // emissions without re-deriving the policy from `policy` alone.
        let payload = json!({
            "schema_version": "hsk.fr.mailbox_backpressure@1",
            "event_id": FrEventId::MailboxBackpressure.as_str(),
            "role_id": role_id.to_string(),
            "queue_depth": queue_depth,
            "policy": policy,
            "outcome": outcome_str,
            "retry_after_secs": retry_after,
            "observed_at_utc": now.to_rfc3339(),
        });
        // Use FlightRecorderEventType::System as the legacy type-enum carrier;
        // the typed canonical id is stamped in the payload's `event_id` so the
        // FR_EVENT_REGISTRY taxonomy stays the authority. This mirrors the
        // MT-077 / MT-090 emission pattern for events that predate adoption
        // of the FrEventId enum in the legacy validator.
        let event = FlightRecorderEvent::new(
            FlightRecorderEventType::System,
            FlightRecorderActor::System,
            Uuid::now_v7(),
            payload,
        )
        .with_actor_id("role_mailbox_backpressure");
        // Best-effort emit. We do not block on the recorder, and we do not
        // surface emission failures to the caller — observability must not
        // change the decision-path verdict. The block_on path is acceptable
        // here because the in-process MT-189 caller is already on a tokio
        // runtime; if no runtime is present (e.g., a sync unit test), we
        // swallow the panic so the decision still propagates.
        let _ = try_block_on_emit(recorder, event);
    }
}

/// Best-effort sync wrapper for the async FlightRecorder::record_event call.
/// Returns Ok if the emit succeeds, Err otherwise — but the call site does
/// not propagate the error so a recorder failure cannot mask the decision.
fn try_block_on_emit(
    recorder: Arc<dyn FlightRecorder>,
    event: FlightRecorderEvent,
) -> Result<(), RecorderError> {
    match tokio::runtime::Handle::try_current() {
        Ok(handle) => {
            // We are on a tokio runtime; spawn a detached task. The decision
            // path has already returned to the caller by the time the emit
            // completes, which is the desired async-detached observability
            // shape.
            handle.spawn(async move {
                let _ = recorder.record_event(event).await;
            });
            Ok(())
        }
        Err(_) => {
            // No tokio runtime available (e.g., a pure-Rust sync test). Build
            // a minimal current-thread runtime, record, and tear down. This
            // path is bounded — it is only hit by unit tests; production
            // call sites always carry a runtime handle.
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .map_err(|e| RecorderError::SinkError(format!("runtime build: {e}")))?;
            rt.block_on(recorder.record_event(event))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::flight_recorder::EventFilter;
    use async_trait::async_trait;

    #[derive(Clone, Default)]
    struct InMemoryRecorder {
        events: Arc<Mutex<Vec<FlightRecorderEvent>>>,
    }

    #[async_trait]
    impl FlightRecorder for InMemoryRecorder {
        async fn record_event(&self, event: FlightRecorderEvent) -> Result<(), RecorderError> {
            self.events.lock().unwrap().push(event);
            Ok(())
        }
        async fn enforce_retention(&self) -> Result<u64, RecorderError> {
            Ok(0)
        }
        async fn list_events(
            &self,
            _filter: EventFilter,
        ) -> Result<Vec<FlightRecorderEvent>, RecorderError> {
            Ok(self.events.lock().unwrap().clone())
        }
    }

    #[test]
    fn allow_under_cap_and_burst() {
        let g = BackpressureGuard::new(BackpressureConfig::default());
        let d = g.check(&RoleId::Coder, 0, Utc::now());
        assert!(matches!(d, BackpressureDecision::Allow));
    }

    #[test]
    fn deny_when_inbox_full() {
        let g = BackpressureGuard::new(BackpressureConfig {
            inbox_cap: 10,
            tokens_per_second: 8,
            burst_capacity: 32,
        });
        let d = g.check(&RoleId::Coder, 10, Utc::now());
        match d {
            BackpressureDecision::Deny { reason, .. } => {
                assert_eq!(reason, DenyReason::InboxFull);
            }
            _ => panic!("expected Deny"),
        }
    }

    #[test]
    fn token_bucket_rate_limits() {
        let g = BackpressureGuard::new(BackpressureConfig {
            inbox_cap: 1000,
            tokens_per_second: 1,
            burst_capacity: 5,
        });
        let now = Utc::now();
        let role = RoleId::Coder;
        let mut allows = 0;
        let mut denies = 0;
        for _ in 0..10 {
            match g.check(&role, 0, now) {
                BackpressureDecision::Allow => allows += 1,
                BackpressureDecision::Deny { .. } => denies += 1,
            }
        }
        // Burst=5 immediate allows; rest denied at instant t.
        assert_eq!(allows, 5);
        assert_eq!(denies, 5);
    }

    #[test]
    fn refill_after_window_allows_again() {
        let g = BackpressureGuard::new(BackpressureConfig {
            inbox_cap: 1000,
            tokens_per_second: 4,
            burst_capacity: 1,
        });
        let t0 = Utc::now();
        assert!(matches!(
            g.check(&RoleId::Coder, 0, t0),
            BackpressureDecision::Allow
        ));
        assert!(matches!(
            g.check(&RoleId::Coder, 0, t0),
            BackpressureDecision::Deny { .. }
        ));
        let t1 = t0 + chrono::Duration::seconds(1);
        assert!(matches!(
            g.check(&RoleId::Coder, 0, t1),
            BackpressureDecision::Allow
        ));
    }

    #[test]
    fn receipt_from_allow_is_none() {
        let r = BackpressureReceipt::from_decision(
            &RoleId::Coder,
            &BackpressureDecision::Allow,
            Utc::now(),
        );
        assert!(r.is_none());
    }

    #[test]
    fn receipt_from_deny_carries_canonical_fr_id() {
        let now = Utc::now();
        let r = BackpressureReceipt::from_decision(
            &RoleId::Coder,
            &BackpressureDecision::Deny {
                reason: DenyReason::InboxFull,
                retry_after_secs: 5,
                observed_pending: 256,
            },
            now,
        )
        .expect("Deny should produce a receipt");
        assert_eq!(r.fr_event_id, "FR-EVT-MAILBOX-BACKPRESSURE");
        assert_eq!(r.role_id, "coder");
        assert_eq!(r.observed_pending, 256);
        assert_eq!(r.receipt_id.get_version_num(), 7);
    }

    #[test]
    fn fr_emit_on_deny_observable_via_recorder() {
        let recorder = Arc::new(InMemoryRecorder::default());
        let g = BackpressureGuard::new(BackpressureConfig {
            inbox_cap: 1,
            tokens_per_second: 1,
            burst_capacity: 1,
        })
        .with_recorder(recorder.clone());
        // pending=1 forces InboxFull immediately.
        let d = g.check(&RoleId::Coder, 1, Utc::now());
        assert!(matches!(
            d,
            BackpressureDecision::Deny {
                reason: DenyReason::InboxFull,
                ..
            }
        ));
        let evs = recorder.events.lock().unwrap();
        assert_eq!(evs.len(), 1);
        let payload = &evs[0].payload;
        assert_eq!(payload["event_id"], "FR-EVT-MAILBOX-BACKPRESSURE");
        assert_eq!(payload["policy"], "inbox_full");
        assert_eq!(payload["outcome"], "deny");
        assert_eq!(payload["role_id"], "coder");
    }
}
