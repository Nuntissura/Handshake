use std::time::{Duration, Instant};

use handshake_swarm_breaker::breaker::*;
use handshake_swarm_breaker::SwarmErrorClass;

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
