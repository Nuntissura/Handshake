//! WP-KERNEL-004 cluster X.1 MT-182 Role Mailbox backpressure integration
//! tests.
//!
//! Spec-Realism Gate compliance:
//!  - Pure-Rust assertions exercise the public surface, the typed
//!    `BackpressureReceipt` round-trip, the deterministic `FixedClock` token
//!    bucket math, the FR-EVT-MAILBOX-BACKPRESSURE emission, and adversarial
//!    boundaries (zero-cap, max-cap, concurrent appenders racing the cap).
//!  - Postgres-backed assertions are `#[ignore]`-gated on `POSTGRES_TEST_URL`
//!    so default `cargo test` exits 0 against the pure-Rust assertions.
//!  - No `LiveXxxUnavailable` / `todo!()` / `unimplemented!()` paths.
//!
//! Adversarial coverage (per MT-182 `red_team.minimum_controls`,
//! `validator_focus`, and the implementation_notes test list):
//!   (a) send `inbox_cap` messages, expect the (cap+1)th to Deny with
//!       InboxFull (test: `mt_182_inbox_cap_257th_message_denies`).
//!   (b) send burst+1 in a single instant, expect the (burst+1)th to Deny
//!       with RateLimitExceeded (test:
//!       `mt_182_rate_limit_burst_plus_one_denies`).
//!   (c) wait one refill window, expect next send to Allow (test:
//!       `mt_182_refill_window_unblocks_via_injected_clock`).
//!   (d) 8 concurrent senders from tokio tasks — count exact Allow/Deny
//!       split matches token bucket math with the injected clock (test:
//!       `mt_182_8_concurrent_senders_exact_allow_deny_split`).
//!   (e) FR-EVT-MAILBOX-BACKPRESSURE emission verified via test stub
//!       FlightRecorder (tests:
//!       `mt_182_fr_emit_on_inbox_full_deny`,
//!       `mt_182_fr_emit_on_rate_limit_deny`,
//!       `mt_182_fr_emit_sampled_on_allow`).
//!   Adversarial extensions (operator clarification):
//!   - Zero-cap config always denies (`mt_182_zero_cap_starves_all_sends`).
//!   - Burst-zero config always denies on rate path
//!     (`mt_182_zero_burst_starves_rate_path`).
//!   - Max-cap config admits up to u32::MAX
//!     (`mt_182_max_cap_admits_below_u32_max`).
//!   - Config serde round-trip with `deny_unknown_fields`
//!     (`mt_182_config_serde_round_trip_rejects_unknown_keys`).
//!   - Receipt serde round-trip with `deny_unknown_fields`
//!     (`mt_182_receipt_serde_round_trip`).
//!   - Postgres-gated: actual cap enforcement against real MT-177
//!     `count_pending_messages_for_role` (test:
//!     `mt_182_postgres_check_via_repo_observes_pending`).

use async_trait::async_trait;
use chrono::Utc;
use handshake_core::flight_recorder::{
    EventFilter, FlightRecorder, FlightRecorderEvent, RecorderError,
};
use handshake_core::role_mailbox::RoleId;
use handshake_core::role_mailbox_v1::{
    backpressure::{
        BackpressureClock, BackpressureConfig, BackpressureDecision, BackpressureGuard,
        BackpressureReceipt, DenyReason, FixedClock,
    },
    message::MessageType,
    repo::RoleMailboxRepository,
    router::ExecutorKind,
    thread::{ClaimMode, LinkedRecordKind, ResponseAuthorityScope, RoleMailboxThread},
    TakeoverPolicy,
};
use std::sync::{Arc, Mutex};

// ----- test-stub FlightRecorder -----

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

impl InMemoryRecorder {
    /// Spin until the in-memory recorder has captured at least `expected`
    /// events. Used by the FR-emission tests because the production emit
    /// path is async-detached on the current tokio runtime.
    async fn wait_for_events(&self, expected: usize) {
        for _ in 0..200 {
            if self.events.lock().unwrap().len() >= expected {
                return;
            }
            tokio::time::sleep(std::time::Duration::from_millis(5)).await;
        }
        let n = self.events.lock().unwrap().len();
        panic!("recorder never reached {expected} events (saw {n})");
    }
}

// ===== Pure-Rust adversarial assertions =====

#[test]
fn mt_182_inbox_cap_257th_message_denies() {
    // (a) per implementation_notes: send 256 messages, expect 257th to Deny
    // with InboxFull. Default config matches the spec §5.7.3 budget.
    let cfg = BackpressureConfig::default();
    let g = BackpressureGuard::new(cfg);
    let now = Utc::now();
    // pending=255 still under the cap (256), still Allow at the inbox-cap
    // path (rate-limit not exercised because burst=32 so first call is fine).
    let d_under = g.check(&RoleId::Coder, 255, now);
    assert!(matches!(d_under, BackpressureDecision::Allow));
    // pending=256 reaches the cap → Deny with InboxFull regardless of bucket.
    let d_at = g.check(&RoleId::Coder, 256, now);
    match d_at {
        BackpressureDecision::Deny {
            reason,
            observed_pending,
            ..
        } => {
            assert_eq!(reason, DenyReason::InboxFull);
            assert_eq!(observed_pending, 256);
        }
        _ => panic!("expected Deny at cap, got {d_at:?}"),
    }
}

#[test]
fn mt_182_rate_limit_burst_plus_one_denies() {
    // (b) per implementation_notes: send 33 messages in 1 second, expect 33rd
    // to Deny with RateLimitExceeded. Default config: tokens_per_second=8,
    // burst_capacity=32. At instant t=0 the bucket admits exactly 32 immediate
    // Allows; the 33rd must Deny.
    let cfg = BackpressureConfig::default();
    let clock = Arc::new(FixedClock::new(Utc::now()));
    let g = BackpressureGuard::new(cfg).with_clock(clock.clone() as Arc<dyn BackpressureClock>);
    let now = clock.now();
    let mut allows = 0;
    let mut denies = 0;
    for _ in 0..33 {
        match g.check(&RoleId::Coder, 0, now) {
            BackpressureDecision::Allow => allows += 1,
            BackpressureDecision::Deny { reason, .. } => {
                denies += 1;
                assert_eq!(reason, DenyReason::RateLimitExceeded);
            }
        }
    }
    assert_eq!(allows, 32);
    assert_eq!(denies, 1);
}

#[test]
fn mt_182_refill_window_unblocks_via_injected_clock() {
    // (c) wait token-refill window, expect next send to Allow. We use the
    // injected FixedClock to advance deterministically; with
    // tokens_per_second=4 and burst=1, one second of elapsed time refills the
    // bucket to 4 tokens (clamped to burst=1).
    let clock = Arc::new(FixedClock::new(Utc::now()));
    let g = BackpressureGuard::new(BackpressureConfig {
        inbox_cap: 1000,
        tokens_per_second: 4,
        burst_capacity: 1,
    })
    .with_clock(clock.clone() as Arc<dyn BackpressureClock>);
    let t0 = clock.now();
    assert!(matches!(
        g.check(&RoleId::Coder, 0, t0),
        BackpressureDecision::Allow
    ));
    assert!(matches!(
        g.check(&RoleId::Coder, 0, t0),
        BackpressureDecision::Deny { .. }
    ));
    clock.advance(chrono::Duration::seconds(1));
    let t1 = clock.now();
    assert!(matches!(
        g.check(&RoleId::Coder, 0, t1),
        BackpressureDecision::Allow
    ));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 8)]
async fn mt_182_8_concurrent_senders_exact_allow_deny_split() {
    // (d) 8 concurrent senders racing the bucket. Burst=4 means exactly 4 of
    // the 8 must Allow at the shared instant; the other 4 must Deny with
    // RateLimitExceeded. With FixedClock the math is deterministic.
    let clock = Arc::new(FixedClock::new(Utc::now()));
    let g = Arc::new(
        BackpressureGuard::new(BackpressureConfig {
            inbox_cap: 1000,
            tokens_per_second: 1,
            burst_capacity: 4,
        })
        .with_clock(clock.clone() as Arc<dyn BackpressureClock>),
    );
    let now = clock.now();
    let mut handles = Vec::with_capacity(8);
    for _ in 0..8 {
        let g = g.clone();
        handles.push(tokio::spawn(async move { g.check(&RoleId::Coder, 0, now) }));
    }
    let mut allows = 0;
    let mut denies = 0;
    for h in handles {
        match h.await.unwrap() {
            BackpressureDecision::Allow => allows += 1,
            BackpressureDecision::Deny { reason, .. } => {
                denies += 1;
                assert_eq!(reason, DenyReason::RateLimitExceeded);
            }
        }
    }
    assert_eq!(
        allows, 4,
        "exactly burst=4 senders must win (got allows={allows}, denies={denies})"
    );
    assert_eq!(
        denies, 4,
        "the other four must Deny with RateLimitExceeded (got allows={allows}, denies={denies})"
    );
}

// ----- (e) FR-EVT-MAILBOX-BACKPRESSURE emission -----

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn mt_182_fr_emit_on_inbox_full_deny() {
    let recorder = Arc::new(InMemoryRecorder::default());
    let g = BackpressureGuard::new(BackpressureConfig {
        inbox_cap: 4,
        tokens_per_second: 8,
        burst_capacity: 32,
    })
    .with_recorder(recorder.clone());
    // pending=4 saturates the inbox; Deny path emits.
    let d = g.check(&RoleId::Coder, 4, Utc::now());
    assert!(matches!(
        d,
        BackpressureDecision::Deny {
            reason: DenyReason::InboxFull,
            ..
        }
    ));
    recorder.wait_for_events(1).await;
    let evs = recorder.events.lock().unwrap();
    assert_eq!(evs.len(), 1);
    let p = &evs[0].payload;
    assert_eq!(p["event_id"], "FR-EVT-MAILBOX-BACKPRESSURE");
    assert_eq!(p["policy"], "inbox_full");
    assert_eq!(p["outcome"], "deny");
    assert_eq!(p["role_id"], "coder");
    assert_eq!(p["queue_depth"], 4);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn mt_182_fr_emit_on_rate_limit_deny() {
    let recorder = Arc::new(InMemoryRecorder::default());
    let clock = Arc::new(FixedClock::new(Utc::now()));
    let g = BackpressureGuard::new(BackpressureConfig {
        inbox_cap: 1000,
        tokens_per_second: 1,
        burst_capacity: 1,
    })
    .with_recorder(recorder.clone())
    .with_clock(clock.clone() as Arc<dyn BackpressureClock>);
    let now = clock.now();
    // First Allow exhausts the burst.
    assert!(matches!(
        g.check(&RoleId::Coder, 0, now),
        BackpressureDecision::Allow
    ));
    // Second Deny with RateLimitExceeded emits FR.
    assert!(matches!(
        g.check(&RoleId::Coder, 0, now),
        BackpressureDecision::Deny {
            reason: DenyReason::RateLimitExceeded,
            ..
        }
    ));
    recorder.wait_for_events(1).await;
    let evs = recorder.events.lock().unwrap();
    // Allow-path is unsampled (rate=0 by default), so only the Deny shows.
    assert_eq!(evs.len(), 1);
    let p = &evs[0].payload;
    assert_eq!(p["policy"], "rate_limit_exceeded");
    assert_eq!(p["outcome"], "deny");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn mt_182_fr_emit_sampled_on_allow() {
    // Red-team minimum_controls #1: sampled Allow-path emissions so
    // saturation trends surface before the cap trips.
    let recorder = Arc::new(InMemoryRecorder::default());
    let g = BackpressureGuard::new(BackpressureConfig {
        inbox_cap: 1000,
        tokens_per_second: 1000,
        burst_capacity: 1000,
    })
    .with_recorder(recorder.clone())
    .with_allow_sample_rate(3);
    // 7 Allows under sample_rate=3 should emit at counters 0, 3, 6 → 3 events.
    for _ in 0..7 {
        assert!(matches!(
            g.check(&RoleId::Coder, 0, Utc::now()),
            BackpressureDecision::Allow
        ));
    }
    recorder.wait_for_events(3).await;
    let evs = recorder.events.lock().unwrap();
    assert_eq!(evs.len(), 3);
    for e in evs.iter() {
        assert_eq!(e.payload["outcome"], "allow");
        assert_eq!(e.payload["policy"], "allow_sampled");
    }
}

// ----- Adversarial extensions: zero-cap, max-cap, burst-zero, serde -----

#[test]
fn mt_182_zero_cap_starves_all_sends() {
    // inbox_cap=0 means *any* pending count >= cap → Deny. Even pending=0
    // satisfies `0 >= 0`, so the first send is rejected.
    let g = BackpressureGuard::new(BackpressureConfig {
        inbox_cap: 0,
        tokens_per_second: 100,
        burst_capacity: 100,
    });
    let d = g.check(&RoleId::Coder, 0, Utc::now());
    assert!(
        matches!(
            d,
            BackpressureDecision::Deny {
                reason: DenyReason::InboxFull,
                ..
            }
        ),
        "zero-cap must reject every send: got {d:?}"
    );
}

#[test]
fn mt_182_zero_burst_starves_rate_path() {
    // burst_capacity=0 means the bucket starts empty and must wait for
    // refill before the first send is admitted. At t=0 the bucket has zero
    // tokens; even with tokens_per_second=100 the first check at the same
    // instant denies.
    let g = BackpressureGuard::new(BackpressureConfig {
        inbox_cap: 1000,
        tokens_per_second: 100,
        burst_capacity: 0,
    });
    let d = g.check(&RoleId::Coder, 0, Utc::now());
    assert!(
        matches!(
            d,
            BackpressureDecision::Deny {
                reason: DenyReason::RateLimitExceeded,
                ..
            }
        ),
        "zero-burst must reject the first send at t=0: got {d:?}"
    );
}

#[test]
fn mt_182_max_cap_admits_below_u32_max() {
    // u32::MAX cap is the spec-allowed upper bound; sends below the cap
    // must Allow at the inbox-cap path.
    let g = BackpressureGuard::new(BackpressureConfig {
        inbox_cap: u32::MAX,
        tokens_per_second: 1000,
        burst_capacity: 1000,
    });
    let d = g.check(&RoleId::Coder, u32::MAX - 1, Utc::now());
    assert!(matches!(d, BackpressureDecision::Allow));
    let d_at = g.check(&RoleId::Coder, u32::MAX, Utc::now());
    assert!(matches!(
        d_at,
        BackpressureDecision::Deny {
            reason: DenyReason::InboxFull,
            ..
        }
    ));
}

#[test]
fn mt_182_config_serde_round_trip_rejects_unknown_keys() {
    let cfg = BackpressureConfig {
        inbox_cap: 10,
        tokens_per_second: 4,
        burst_capacity: 8,
    };
    let s = serde_json::to_string(&cfg).unwrap();
    let back: BackpressureConfig = serde_json::from_str(&s).unwrap();
    assert_eq!(back, cfg);
    // `deny_unknown_fields` semantics enforced via the struct attribute on
    // BackpressureConfig — adding an `unknown` key must fail to deserialise.
    let bad = r#"{"inbox_cap":1,"tokens_per_second":1,"burst_capacity":1,"unknown":"hello"}"#;
    let r: Result<BackpressureConfig, _> = serde_json::from_str(bad);
    assert!(
        r.is_err(),
        "deny_unknown_fields must reject extraneous keys, got {r:?}"
    );
}

#[test]
fn mt_182_receipt_serde_round_trip() {
    let d = BackpressureDecision::Deny {
        reason: DenyReason::InboxFull,
        retry_after_secs: 7,
        observed_pending: 256,
    };
    let r = BackpressureReceipt::from_decision(&RoleId::Validator, &d, Utc::now())
        .expect("Deny must produce a receipt");
    let s = serde_json::to_string(&r).unwrap();
    let back: BackpressureReceipt = serde_json::from_str(&s).unwrap();
    assert_eq!(back, r);
    // Receipt also enforces deny_unknown_fields.
    let bad = serde_json::to_value(&r)
        .map(|mut v| {
            v.as_object_mut()
                .unwrap()
                .insert("unknown".to_string(), serde_json::json!("xxx"));
            serde_json::to_string(&v).unwrap()
        })
        .unwrap();
    let parsed: Result<BackpressureReceipt, _> = serde_json::from_str(&bad);
    assert!(
        parsed.is_err(),
        "receipt deny_unknown_fields must reject extraneous keys"
    );
}

#[test]
fn mt_182_decision_serde_round_trip_discriminated_union() {
    // BackpressureDecision is a tagged enum (`outcome` key). Round-trip
    // both arms.
    let allow = BackpressureDecision::Allow;
    let s_allow = serde_json::to_string(&allow).unwrap();
    let back_allow: BackpressureDecision = serde_json::from_str(&s_allow).unwrap();
    assert_eq!(back_allow, allow);

    let deny = BackpressureDecision::Deny {
        reason: DenyReason::RateLimitExceeded,
        retry_after_secs: 3,
        observed_pending: 12,
    };
    let s_deny = serde_json::to_string(&deny).unwrap();
    let back_deny: BackpressureDecision = serde_json::from_str(&s_deny).unwrap();
    assert_eq!(back_deny, deny);
}

#[test]
fn mt_182_no_silent_drop_contract_decision_carries_reason_and_retry() {
    // validator_focus: "No silent drops — every saturation emits FR-EVT-
    // MAILBOX-BACKPRESSURE; backpressure decisions are observable in
    // metrics and Flight Recorder, never just suppressed." This test
    // asserts the typed surface alone is sufficient — the caller cannot
    // see Allow when the inbox cap is breached.
    let g = BackpressureGuard::new(BackpressureConfig {
        inbox_cap: 1,
        tokens_per_second: 100,
        burst_capacity: 100,
    });
    let d = g.check(&RoleId::Coder, 1, Utc::now());
    match d {
        BackpressureDecision::Deny {
            reason,
            retry_after_secs,
            observed_pending,
        } => {
            assert_eq!(reason, DenyReason::InboxFull);
            assert!(retry_after_secs >= 1, "retry_after_secs must be >= 1s");
            assert_eq!(observed_pending, 1);
        }
        BackpressureDecision::Allow => panic!("inbox at cap must Deny, never silently Allow"),
    }
}

#[test]
fn mt_182_advisory_role_id_bucket_keyed_on_advisory_id() {
    // The token bucket key must include the Advisory variant's inner id so
    // two distinct advisory roles do not share a bucket. We bucket by
    // `role_id.to_string()` which renders Advisory as `advisory:<id>`.
    let g = BackpressureGuard::new(BackpressureConfig {
        inbox_cap: 1000,
        tokens_per_second: 1,
        burst_capacity: 1,
    });
    let now = Utc::now();
    let scout = RoleId::Advisory("scout".to_string());
    let nomad = RoleId::Advisory("nomad".to_string());
    // Each role gets one burst-1 token.
    assert!(matches!(
        g.check(&scout, 0, now),
        BackpressureDecision::Allow
    ));
    assert!(matches!(
        g.check(&nomad, 0, now),
        BackpressureDecision::Allow
    ));
    // Second send to each must Deny on the rate path.
    assert!(matches!(
        g.check(&scout, 0, now),
        BackpressureDecision::Deny { .. }
    ));
    assert!(matches!(
        g.check(&nomad, 0, now),
        BackpressureDecision::Deny { .. }
    ));
}

#[test]
fn mt_182_default_config_matches_spec_5_7_3_budgets() {
    // Defaults are the spec §5.7.3 budgets; lock them in so a careless
    // refactor cannot silently drift the production values.
    let cfg = BackpressureConfig::default();
    assert_eq!(cfg.inbox_cap, 256);
    assert_eq!(cfg.tokens_per_second, 8);
    assert_eq!(cfg.burst_capacity, 32);
}

// ===== Postgres-gated integration tests =====
// Run with `cargo test --features test-utils --test
// role_mailbox_backpressure_tests -- --ignored` after exporting
// `POSTGRES_TEST_URL=postgres://user:pass@host/db`.

#[tokio::test]
#[ignore = "requires real PostgreSQL; auto-resolves POSTGRES_TEST_URL > DATABASE_URL > managed PostgreSQL; run with `cargo test -- --ignored`"]
async fn mt_182_postgres_check_via_repo_observes_pending() {
    let pool = postgres_pool().await;
    let repo = RoleMailboxRepository::new(pool);
    repo.ensure_schema().await.expect("schema");

    // Create a thread with cap=2, append three pending DelegateWork messages
    // addressed to Coder, then assert check_via_repo observes pending >=
    // cap and emits FR + returns a typed receipt.
    let thread = sample_open_thread();
    let id = thread.thread_id;
    repo.create_thread(thread).await.expect("create");
    for _ in 0..3 {
        repo.append_message(
            id,
            MessageType::DelegateWork,
            RoleId::Orchestrator,
            vec![RoleId::Coder],
            serde_json::json!({}),
        )
        .await
        .expect("append");
    }
    let recorder = Arc::new(InMemoryRecorder::default());
    let g = BackpressureGuard::new(BackpressureConfig {
        inbox_cap: 2,
        tokens_per_second: 100,
        burst_capacity: 100,
    })
    .with_recorder(recorder.clone());
    let (decision, receipt) = g
        .check_via_repo(&repo, &RoleId::Coder)
        .await
        .expect("check_via_repo");
    match decision {
        BackpressureDecision::Deny {
            reason,
            observed_pending,
            ..
        } => {
            assert_eq!(reason, DenyReason::InboxFull);
            assert!(
                observed_pending >= 2,
                "observed_pending must be >= cap (got {observed_pending})"
            );
        }
        _ => panic!("inbox at cap must Deny against real repo, got {decision:?}"),
    }
    let receipt = receipt.expect("Deny path must produce receipt");
    assert_eq!(receipt.fr_event_id, "FR-EVT-MAILBOX-BACKPRESSURE");
    assert_eq!(receipt.reason, DenyReason::InboxFull);
    // FR emission is async-detached; wait then assert.
    recorder.wait_for_events(1).await;
    let evs = recorder.events.lock().unwrap();
    assert_eq!(evs.len(), 1);
    assert_eq!(evs[0].payload["event_id"], "FR-EVT-MAILBOX-BACKPRESSURE");
}

#[tokio::test]
#[ignore = "requires real PostgreSQL; auto-resolves POSTGRES_TEST_URL > DATABASE_URL > managed PostgreSQL; run with `cargo test -- --ignored`"]
async fn mt_182_postgres_check_via_repo_allows_under_cap() {
    let pool = postgres_pool().await;
    let repo = RoleMailboxRepository::new(pool);
    repo.ensure_schema().await.expect("schema");

    // Create a thread, append one pending message. cap=100 so we are way
    // under — check_via_repo must Allow and return None receipt.
    let thread = sample_open_thread();
    let id = thread.thread_id;
    repo.create_thread(thread).await.expect("create");
    repo.append_message(
        id,
        MessageType::DelegateWork,
        RoleId::Orchestrator,
        vec![RoleId::Validator],
        serde_json::json!({}),
    )
    .await
    .expect("append");
    let g = BackpressureGuard::new(BackpressureConfig {
        inbox_cap: 100,
        tokens_per_second: 100,
        burst_capacity: 100,
    });
    let (decision, receipt) = g
        .check_via_repo(&repo, &RoleId::Validator)
        .await
        .expect("check_via_repo");
    assert!(matches!(decision, BackpressureDecision::Allow));
    assert!(receipt.is_none());
}

#[tokio::test]
#[ignore = "requires real PostgreSQL; auto-resolves POSTGRES_TEST_URL > DATABASE_URL > managed PostgreSQL; run with `cargo test -- --ignored`"]
async fn mt_182_postgres_check_via_repo_rate_limit_path() {
    let pool = postgres_pool().await;
    let repo = RoleMailboxRepository::new(pool);
    repo.ensure_schema().await.expect("schema");

    // No pending messages, but burst=1 means the second send within the
    // same instant must Deny on the rate path.
    let g = BackpressureGuard::new(BackpressureConfig {
        inbox_cap: 1000,
        tokens_per_second: 1,
        burst_capacity: 1,
    });
    let (d1, _) = g
        .check_via_repo(&repo, &RoleId::Coder)
        .await
        .expect("first check");
    assert!(matches!(d1, BackpressureDecision::Allow));
    let (d2, r2) = g
        .check_via_repo(&repo, &RoleId::Coder)
        .await
        .expect("second check");
    match d2 {
        BackpressureDecision::Deny { reason, .. } => {
            assert_eq!(reason, DenyReason::RateLimitExceeded);
        }
        _ => panic!("second instant send must Deny on rate path, got {d2:?}"),
    }
    assert!(r2.is_some(), "Deny must carry a receipt");
}

// ----- helpers -----

fn sample_open_thread() -> RoleMailboxThread {
    RoleMailboxThread::open(
        format!(
            "mt-182-bp-{}",
            Utc::now().timestamp_nanos_opt().unwrap_or(0)
        ),
        LinkedRecordKind::Wp,
        Some("WP-KERNEL-004".to_string()),
        vec![ExecutorKind::LocalSmallModel],
        ClaimMode::Exclusive,
        TakeoverPolicy::Never,
        ResponseAuthorityScope::LeaseHolder,
    )
}

async fn postgres_pool() -> sqlx::PgPool {
    let url = handshake_core::storage::tests::postgres_test_base_url()
        .await
        .expect("resolve real PostgreSQL for role_mailbox_backpressure_tests");
    sqlx::PgPool::connect(&url).await.expect("postgres connect")
}
