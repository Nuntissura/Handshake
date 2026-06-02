//! Failure-fingerprint circuit breaker.
//!
//! Keyed on **error identity** — a hash of `(error_class, truncated_message)`
//! — NOT on a per-task or per-instance id. The point is exactly that one
//! *systemic* failure (e.g. "model artifact sha256 mismatch", "CUDA OOM") will
//! present the *same* fingerprint across many sessions; once a fingerprint
//! crosses a threshold the breaker trips and SUPPRESSES further spawns/retries
//! carrying that fingerprint for a cooldown window. Without this, N retries of
//! a deterministically-failing spawn would drain the entire lifetime spawn
//! budget.
//!
//! Research basis: this is the standard three-state circuit-breaker
//! (Closed -> Open -> Half-Open) from Nygard's *Release It!* / resilience4j /
//! Polly, specialised so the *key* is the failure signature rather than the
//! downstream endpoint. resilience4j keys breakers by name; Polly by policy
//! instance; here the novel-but-field-aligned twist is signature-keyed
//! bucketing so heterogeneous work sharing one coordinator does not need one
//! breaker per task — a single coordinator-wide map of signature -> breaker
//! state absorbs correlated failures.

use std::collections::HashMap;
use std::time::{Duration, Instant};

use sha2::{Digest, Sha256};

use super::error::SwarmErrorClass;

/// Max bytes of the error detail folded into the fingerprint. Truncating keeps
/// signatures stable when a message carries a variable tail (ids, offsets)
/// while still distinguishing genuinely different failure messages.
pub const FINGERPRINT_DETAIL_TRUNCATE_BYTES: usize = 96;

/// A stable, opaque signature for a class of failure. `Display`/`as_str`
/// yields the hex digest so it can be logged and carried in
/// [`super::error::SwarmError::BreakerOpen`].
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct FailureFingerprint(String);

impl FailureFingerprint {
    /// Compute the fingerprint from an error class and free-form detail.
    /// The detail is truncated on a char boundary before hashing so a
    /// high-cardinality tail does not explode the signature space.
    pub fn compute(class: SwarmErrorClass, detail: &str) -> Self {
        let mut truncated = detail;
        if truncated.len() > FINGERPRINT_DETAIL_TRUNCATE_BYTES {
            // Find the largest char boundary <= the byte budget.
            let mut end = FINGERPRINT_DETAIL_TRUNCATE_BYTES;
            while end > 0 && !detail.is_char_boundary(end) {
                end -= 1;
            }
            truncated = &detail[..end];
        }
        let mut hasher = Sha256::new();
        hasher.update(class.as_str().as_bytes());
        hasher.update([0u8]); // domain separator between class and detail
        hasher.update(truncated.as_bytes());
        let digest = hasher.finalize();
        // 16 hex chars (64 bits) is ample to avoid collisions across the
        // handful of distinct failure classes a run will produce.
        let hex: String = digest.iter().take(8).map(|b| format!("{b:02x}")).collect();
        Self(hex)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for FailureFingerprint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BreakerState {
    Closed,
    Open,
    HalfOpen,
}

#[derive(Clone, Copy, Debug)]
pub struct BreakerConfig {
    /// Consecutive same-signature failures that trip the breaker.
    pub failure_threshold: u32,
    /// How long the breaker stays Open before allowing a single half-open
    /// probe.
    pub cooldown: Duration,
}

impl Default for BreakerConfig {
    fn default() -> Self {
        Self {
            failure_threshold: 5,
            cooldown: Duration::from_secs(30),
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct SignatureState {
    state: BreakerState,
    consecutive_failures: u32,
    opened_at: Option<Instant>,
}

impl Default for SignatureState {
    fn default() -> Self {
        Self {
            state: BreakerState::Closed,
            consecutive_failures: 0,
            opened_at: None,
        }
    }
}

/// Outcome of asking the breaker whether a spawn carrying a given fingerprint
/// may proceed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AdmitDecision {
    /// Breaker closed (or half-open probe permitted) — proceed.
    Admit,
    /// Breaker open — suppress, with remaining cooldown for the error.
    Suppress { cooldown_remaining_ms: u128 },
}

/// The signature-keyed circuit breaker. Not internally synchronised; the
/// coordinator owns it behind its own lock so trip decisions and the spawn
/// counter move together.
#[derive(Debug, Default)]
pub struct FailureFingerprintBreaker {
    config: BreakerConfigInner,
    signatures: HashMap<FailureFingerprint, SignatureState>,
}

#[derive(Debug, Clone, Copy)]
struct BreakerConfigInner {
    failure_threshold: u32,
    cooldown: Duration,
}

impl Default for BreakerConfigInner {
    fn default() -> Self {
        let c = BreakerConfig::default();
        Self {
            failure_threshold: c.failure_threshold,
            cooldown: c.cooldown,
        }
    }
}

impl FailureFingerprintBreaker {
    pub fn new(config: BreakerConfig) -> Self {
        Self {
            config: BreakerConfigInner {
                failure_threshold: config.failure_threshold.max(1),
                cooldown: config.cooldown,
            },
            signatures: HashMap::new(),
        }
    }

    /// Decide admission for a fingerprint at instant `now`. Open breakers
    /// transition to half-open once cooldown elapses and admit exactly one
    /// probe.
    pub fn admit(&mut self, fp: &FailureFingerprint, now: Instant) -> AdmitDecision {
        let entry = self.signatures.entry(fp.clone()).or_default();
        match entry.state {
            BreakerState::Closed | BreakerState::HalfOpen => AdmitDecision::Admit,
            BreakerState::Open => {
                let opened_at = entry.opened_at.unwrap_or(now);
                let elapsed = now.saturating_duration_since(opened_at);
                if elapsed >= self.config.cooldown {
                    // Cooldown elapsed: allow a single half-open probe.
                    entry.state = BreakerState::HalfOpen;
                    AdmitDecision::Admit
                } else {
                    AdmitDecision::Suppress {
                        cooldown_remaining_ms: (self.config.cooldown - elapsed).as_millis(),
                    }
                }
            }
        }
    }

    /// Record a success for a fingerprint, closing/healing the breaker.
    pub fn record_success(&mut self, fp: &FailureFingerprint) {
        let entry = self.signatures.entry(fp.clone()).or_default();
        entry.consecutive_failures = 0;
        entry.state = BreakerState::Closed;
        entry.opened_at = None;
    }

    /// Record a failure for a fingerprint at instant `now`. Returns `true` iff
    /// this call *tripped* the breaker (Closed/HalfOpen -> Open) so the caller
    /// can emit the BREAKER-TRIPPED event exactly once per trip.
    pub fn record_failure(&mut self, fp: &FailureFingerprint, now: Instant) -> bool {
        let threshold = self.config.failure_threshold;
        let entry = self.signatures.entry(fp.clone()).or_default();
        match entry.state {
            BreakerState::HalfOpen => {
                // Probe failed -> re-open immediately, reset the cooldown clock.
                entry.state = BreakerState::Open;
                entry.opened_at = Some(now);
                entry.consecutive_failures = entry.consecutive_failures.saturating_add(1);
                true
            }
            BreakerState::Open => {
                entry.consecutive_failures = entry.consecutive_failures.saturating_add(1);
                false
            }
            BreakerState::Closed => {
                entry.consecutive_failures = entry.consecutive_failures.saturating_add(1);
                if entry.consecutive_failures >= threshold {
                    entry.state = BreakerState::Open;
                    entry.opened_at = Some(now);
                    true
                } else {
                    false
                }
            }
        }
    }

    pub fn state_of(&self, fp: &FailureFingerprint) -> BreakerState {
        self.signatures
            .get(fp)
            .map(|s| s.state)
            .unwrap_or(BreakerState::Closed)
    }

    pub fn consecutive_failures(&self, fp: &FailureFingerprint) -> u32 {
        self.signatures
            .get(fp)
            .map(|s| s.consecutive_failures)
            .unwrap_or(0)
    }

    /// Number of signatures currently tracked. Used by the coordinator's
    /// unbounded-growth guard / tests.
    pub fn tracked_signatures(&self) -> usize {
        self.signatures.len()
    }

    /// Drop signature entries that are fully healed (Closed, no recorded
    /// failures) OR Open-but-cooled-down past `cooldown`. A healed/closed entry
    /// carries no state worth retaining — it is indistinguishable from the
    /// `or_default()` an `admit`/`record_failure` would re-create — so pruning
    /// it bounds the map without changing breaker behaviour. An Open entry that
    /// has sat past its cooldown is equivalent to Closed for admission (the next
    /// `admit` would half-open it anyway), so it is also safe to drop. Returns
    /// the number of entries removed.
    pub fn prune_settled(&mut self, now: Instant) -> usize {
        let cooldown = self.config.cooldown;
        let before = self.signatures.len();
        self.signatures.retain(|_, s| match s.state {
            BreakerState::Closed => s.consecutive_failures != 0,
            BreakerState::HalfOpen => true,
            BreakerState::Open => {
                let opened_at = s.opened_at.unwrap_or(now);
                now.saturating_duration_since(opened_at) < cooldown
            }
        });
        before - self.signatures.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fp(detail: &str) -> FailureFingerprint {
        FailureFingerprint::compute(SwarmErrorClass::FactoryFailed, detail)
    }

    #[test]
    fn same_class_and_detail_yield_same_fingerprint() {
        assert_eq!(fp("cuda oom"), fp("cuda oom"));
    }

    #[test]
    fn different_class_yields_different_fingerprint() {
        let a = FailureFingerprint::compute(SwarmErrorClass::FactoryFailed, "x");
        let b = FailureFingerprint::compute(SwarmErrorClass::ReclaimFailed, "x");
        assert_ne!(a, b);
    }

    #[test]
    fn truncation_collapses_variable_tail() {
        // Shared prefix must exceed FINGERPRINT_DETAIL_TRUNCATE_BYTES (96) so
        // that only the (truncated-away) tail differs.
        let head = "X".repeat(FINGERPRINT_DETAIL_TRUNCATE_BYTES + 4);
        let a = fp(&format!("{head}tail-A-12345"));
        let b = fp(&format!("{head}tail-B-67890"));
        // Both share the first 96 bytes, so they collapse to one signature.
        assert_eq!(a, b);
        // A genuinely different prefix (within the budget) must NOT collapse.
        let c = fp("totally different short message");
        assert_ne!(a, c);
    }

    #[test]
    fn trips_after_threshold_consecutive_failures() {
        let mut b = FailureFingerprintBreaker::new(BreakerConfig {
            failure_threshold: 3,
            cooldown: Duration::from_secs(10),
        });
        let s = fp("boom");
        let now = Instant::now();
        assert!(!b.record_failure(&s, now));
        assert!(!b.record_failure(&s, now));
        assert!(b.record_failure(&s, now)); // third trips
        assert_eq!(b.state_of(&s), BreakerState::Open);
        assert!(matches!(b.admit(&s, now), AdmitDecision::Suppress { .. }));
    }

    #[test]
    fn half_open_probe_after_cooldown() {
        let mut b = FailureFingerprintBreaker::new(BreakerConfig {
            failure_threshold: 1,
            cooldown: Duration::from_millis(50),
        });
        let s = fp("boom");
        let t0 = Instant::now();
        assert!(b.record_failure(&s, t0));
        // Still open before cooldown.
        assert!(matches!(b.admit(&s, t0), AdmitDecision::Suppress { .. }));
        let t1 = t0 + Duration::from_millis(60);
        // Cooldown elapsed -> half-open probe admitted.
        assert_eq!(b.admit(&s, t1), AdmitDecision::Admit);
        assert_eq!(b.state_of(&s), BreakerState::HalfOpen);
        // Success heals.
        b.record_success(&s);
        assert_eq!(b.state_of(&s), BreakerState::Closed);
    }
}
