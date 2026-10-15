use std::collections::HashMap;
use std::time::{Duration, Instant};

pub use crate::failure::{FailureFingerprint, FINGERPRINT_DETAIL_TRUNCATE_BYTES};

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
