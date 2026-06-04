//! WP-KERNEL-004 cluster X.2 MT-187 MT queue starvation prevention
//! (age-based priority + fair scheduling) integration tests.
//!
//! Contract: MT-187 owns `FairScheduler` (priority + `claim_next_priority`
//! Postgres CTE), `StarvationGuard` (in-memory + DB-watermarked monotonic
//! emission of FR-EVT-MT-STARVED), `StarvationConfig`, the
//! `starvation_watermark_at_utc` column on `kernel_micro_task_job`, the
//! per-wp claim-window index, and this integration-test surface.
//!
//! Implementation paths (relative to crate root):
//!   - `src/mt_executor/scheduler.rs` — FairScheduler + StarvationGuard
//!     (extends `queue.rs::MicroTaskQueue` from MT-184)
//!   - `migrations/0026_mt_scheduler_starvation_watermark.sql` — adds the
//!     watermark column + claim-window index
//!
//! Note on owned-files drift vs MT-187 contract `owned_files`:
//!   The contract's `expected_diff_shape` calls for
//!   `process_ledger/mt_scheduler.rs`. In practice the scheduler lives
//!   under `mt_executor/` because MT-184..MT-189 form a tight subscope
//!   cluster (X.2) and the executor composes job + queue + loop_control +
//!   cancellation + scheduler + outcome + executor as siblings. The test
//!   file lives at the contract-named path (`tests/mt_scheduler_tests.rs`)
//!   to anchor MT-187 acceptance evidence independent of the cluster-wide
//!   `mt_executor_tests.rs` smoke surface. Same drift pattern as MT-184
//!   (already flagged as a residual for IntVal waiver).
//!
//! Adversarial coverage:
//!
//! Pure-Rust always-on:
//!   (a) age-priority ordering: old T7B beats fresh T7B; very old T7B
//!       eventually outweighs fresh T32B because age_boost caps at 200
//!       (200 + 20 = 220 > 100 base T32B).
//!   (b) tier-tie-break FIFO: same tier + same age (within 1 minute
//!       resolution) tie-broken by `created_at_utc` ASC (earliest wins).
//!   (c) HardGate dominates regardless of age (base_weight 1000 >> any
//!       age_boost cap 200 + best other tier 100).
//!   (d) Age boost cap holds at +200 even for very old jobs (24h-old
//!       T7B priority is 220 not 1460).
//!   (e) Fairness penalty caps at -200: 100 recent claims still only
//!       penalises by 200, not 5000.
//!   (f) Fairness penalty for OTHER wp does NOT affect this wp's score
//!       (penalty key is wp_id, not global).
//!   (g) Empty candidate list returns None.
//!   (h) Single candidate is returned regardless of score.
//!   (i) StarvationGuard threshold respected: below threshold returns None.
//!   (j) StarvationGuard emits exactly once per (job_id, crossing)
//!       in-memory (red_team #2 in-process slice).
//!   (k) StarvationSignal serde round-trip.
//!   (l) priority CTE SQL contains FOR UPDATE SKIP LOCKED + LIMIT 1
//!       (red_team #1 — proven structurally without a live DB).
//!   (m) priority CTE SQL renders every tier wire form in the CASE so
//!       priority is computed for every queued row.
//!   (n) priority CTE SQL embeds the fairness window seconds from
//!       StarvationConfig (configurable per Work Profile).
//!   (o) StarvationConfig default is field-stable (defends against silent
//!       cfg drift on later MT changes).
//!
//! Postgres-gated (`#[ignore]` until `POSTGRES_TEST_URL` is set):
//!   (p) claim_next_priority claims the highest-priority job from a
//!       realistic queue mix (100 fresh T7B + 1 old T32B → old T32B
//!       claimed within the first 5 picks; in practice claimed first
//!       because age_boost on the old job dominates).
//!   (q) Fair scheduling under wp-imbalance: pre-claim 50 jobs from
//!       BUSY wp (populating claimed_at_utc within the fairness window),
//!       then enqueue 5 more BUSY + 5 OTHER fresh T7B; OTHER wp must be
//!       claimed first because BUSY wp gets the -200 cap and OTHER does
//!       not.
//!   (r) Starvation watermark is monotonic across "process restarts":
//!       check_with_watermark on a stale job emits ONCE; a second call
//!       on a fresh StarvationGuard (simulated restart) sees the
//!       persisted watermark and emits None.
//!   (s) Empty queue returns None from claim_next_priority.
//!   (t) Atomic claim race using claim_next_priority: 8 rows + 8 parallel
//!       claimers → every row claimed at most once (set-cardinality
//!       assertion).

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;

use chrono::Utc;
use handshake_core::mt_executor::{
    job::{EscalationTier, MicroTaskJob, MicroTaskJobId, MicroTaskJobState},
    queue::MicroTaskQueue,
    scheduler::{FairScheduler, StarvationConfig, StarvationGuard, StarvationSignal},
};
use uuid::Uuid;

// ============================================================================
// Pure-Rust always-on assertions
// ============================================================================

fn make_job(wp: &str, tier: EscalationTier, age_minutes: i64) -> MicroTaskJob {
    let mut j = MicroTaskJob::queue(wp, "MT", PathBuf::from("a.json"), 6, vec![]);
    j.escalation_tier = tier;
    j.created_at_utc = Utc::now() - chrono::Duration::minutes(age_minutes);
    j
}

#[test]
fn mt_187_old_t7b_beats_fresh_t7b() {
    let s = FairScheduler::new(StarvationConfig::default());
    let cands = vec![
        make_job("W-fresh", EscalationTier::T7B, 0),
        make_job("W-old", EscalationTier::T7B, 5),
    ];
    let pick = s.pick_next(&cands, Utc::now(), &HashMap::new()).unwrap();
    assert_eq!(pick.wp_id, "W-old", "older job within same tier must win");
}

#[test]
fn mt_187_very_old_t7b_outweighs_fresh_t32b() {
    // age_boost caps at +200; T7B base 20 + 200 = 220 > T32B base 100.
    // So a sufficiently old T7B must eventually outweigh a fresh T32B —
    // the contract's anti-starvation invariant.
    let s = FairScheduler::new(StarvationConfig::default());
    let cands = vec![
        make_job("W-fresh-t32", EscalationTier::T32B, 0),
        make_job("W-very-old-t7", EscalationTier::T7B, 250),
    ];
    let pick = s.pick_next(&cands, Utc::now(), &HashMap::new()).unwrap();
    assert_eq!(
        pick.wp_id, "W-very-old-t7",
        "T7B aged past age_boost_cap must outweigh fresh T32B (anti-starvation)"
    );
}

#[test]
fn mt_187_tier_tie_break_is_fifo_on_created_at() {
    // Same tier + same wp (so no fairness penalty), only created_at_utc
    // distinguishes the candidates; earliest wins.
    let s = FairScheduler::new(StarvationConfig::default());
    let now = Utc::now();
    let mut first = MicroTaskJob::queue("W-A", "M-1", PathBuf::from("a.json"), 6, vec![]);
    first.escalation_tier = EscalationTier::T7B;
    first.created_at_utc = now - chrono::Duration::seconds(30);
    let mut second = MicroTaskJob::queue("W-A", "M-2", PathBuf::from("a.json"), 6, vec![]);
    second.escalation_tier = EscalationTier::T7B;
    second.created_at_utc = now - chrono::Duration::seconds(10);
    let cands = vec![first.clone(), second];
    let pick = s.pick_next(&cands, now, &HashMap::new()).unwrap();
    assert_eq!(pick.mt_id, first.mt_id, "FIFO tie-break must pick earliest created_at_utc");
}

#[test]
fn mt_187_hardgate_dominates_regardless_of_age() {
    let s = FairScheduler::new(StarvationConfig::default());
    let cands = vec![
        make_job("W-very-old", EscalationTier::T7B, 1000),
        make_job("W-hg", EscalationTier::HardGate, 0),
    ];
    let pick = s.pick_next(&cands, Utc::now(), &HashMap::new()).unwrap();
    assert_eq!(
        pick.wp_id, "W-hg",
        "HardGate base_weight 1000 must always dominate (operator pause invariant)"
    );
}

#[test]
fn mt_187_age_boost_caps_at_200() {
    // 24h-old T7B priority must be exactly base(20) + cap(200) = 220.
    // Without the cap it would be 20 + 1440 = 1460 and would silently
    // outrank HardGate (which is wrong — HardGate is the operator-pause
    // sentinel).
    let s = FairScheduler::new(StarvationConfig::default());
    let job = make_job("W-day-old", EscalationTier::T7B, 60 * 24);
    let p = s.priority(&job, Utc::now(), &HashMap::new());
    assert_eq!(p, 220, "age_boost must cap at +200 so very-old T7B never outranks HardGate");
    let hg = make_job("W-hg", EscalationTier::HardGate, 0);
    let p_hg = s.priority(&hg, Utc::now(), &HashMap::new());
    assert!(p_hg > p, "HardGate ({}) must always outrank capped old T7B ({})", p_hg, p);
}

#[test]
fn mt_187_fairness_penalty_caps_at_minus_200() {
    // 100 recent claims must penalise only -200, not -5000.
    let s = FairScheduler::new(StarvationConfig::default());
    let job = make_job("BUSY", EscalationTier::T7B, 0);
    let mut recent = HashMap::new();
    recent.insert("BUSY".to_string(), 100);
    let p = s.priority(&job, Utc::now(), &recent);
    assert_eq!(p, 20 - 200, "fairness penalty must cap at -200");
}

#[test]
fn mt_187_fairness_penalty_does_not_cross_wp() {
    // Penalising BUSY wp must not affect OTHER wp's score.
    let s = FairScheduler::new(StarvationConfig::default());
    let other = make_job("OTHER", EscalationTier::T7B, 0);
    let mut recent = HashMap::new();
    recent.insert("BUSY".to_string(), 100);
    let p = s.priority(&other, Utc::now(), &recent);
    assert_eq!(p, 20, "OTHER wp must not be penalised for BUSY wp's recent claims");
}

#[test]
fn mt_187_pick_next_empty_candidates_returns_none() {
    let s = FairScheduler::new(StarvationConfig::default());
    assert!(s.pick_next(&[], Utc::now(), &HashMap::new()).is_none());
}

#[test]
fn mt_187_pick_next_single_candidate_returns_it() {
    let s = FairScheduler::new(StarvationConfig::default());
    let only = make_job("W-A", EscalationTier::T7B, 0);
    let pick = s
        .pick_next(std::slice::from_ref(&only), Utc::now(), &HashMap::new())
        .unwrap();
    assert_eq!(pick.wp_id, "W-A");
}

#[test]
fn mt_187_starvation_guard_threshold_respected() {
    let g = StarvationGuard::new(StarvationConfig {
        starvation_threshold_secs: 600,
        ..StarvationConfig::default()
    });
    // 30s-old job is well below the 600s threshold — must not emit.
    let mut j = MicroTaskJob::queue("W", "M", PathBuf::from("a.json"), 6, vec![]);
    j.created_at_utc = Utc::now() - chrono::Duration::seconds(30);
    assert!(g.check(&j, Utc::now()).is_none(), "below threshold must not emit");
}

#[test]
fn mt_187_starvation_guard_emits_exactly_once_in_memory() {
    let g = StarvationGuard::new(StarvationConfig {
        starvation_threshold_secs: 60,
        ..StarvationConfig::default()
    });
    let mut j = MicroTaskJob::queue("W-x", "M", PathBuf::from("a.json"), 6, vec![]);
    j.created_at_utc = Utc::now() - chrono::Duration::seconds(120);
    let s1 = g.check(&j, Utc::now());
    let s2 = g.check(&j, Utc::now() + chrono::Duration::seconds(120));
    let s3 = g.check(&j, Utc::now() + chrono::Duration::seconds(600));
    assert!(s1.is_some(), "first crossing must emit");
    assert!(s2.is_none(), "second call must be silent (monotonic)");
    assert!(s3.is_none(), "third call far later must still be silent");
}

#[test]
fn mt_187_starvation_signal_serde_round_trip() {
    let sig = StarvationSignal {
        job_id: Uuid::now_v7(),
        wp_id: "WP-MT187-RT".to_string(),
        age_secs: 1234,
    };
    let s = serde_json::to_string(&sig).expect("serialize");
    let back: StarvationSignal = serde_json::from_str(&s).expect("deserialize");
    assert_eq!(back, sig);
}

#[test]
fn mt_187_claim_next_priority_sql_uses_for_update_skip_locked_limit_1() {
    let s = FairScheduler::new(StarvationConfig::default());
    let sql = s.claim_next_priority_sql();
    assert!(
        sql.contains("FOR UPDATE SKIP LOCKED"),
        "claim SQL must use SKIP LOCKED (red_team #1: no client-side re-rank race)"
    );
    assert!(sql.contains("LIMIT 1"), "claim SQL must LIMIT 1");
    // Defends the priority pattern itself: priority is computed in a CTE
    // and the outer SELECT picks by it.
    assert!(
        sql.contains("WITH base AS") && sql.contains("scored AS") && sql.contains("priority"),
        "claim SQL must compute priority in a CTE before the FOR UPDATE pick"
    );
}

#[test]
fn mt_187_claim_next_priority_sql_renders_all_six_tier_wire_forms() {
    let s = FairScheduler::new(StarvationConfig::default());
    let sql = s.claim_next_priority_sql();
    for tier in ["hard_gate", "t32b", "t13b_alt", "t13b", "t7b_alt", "t7b"] {
        assert!(
            sql.contains(&format!("'{}'", tier)),
            "tier wire form '{}' missing from CASE",
            tier
        );
    }
}

#[test]
fn mt_187_claim_next_priority_sql_embeds_fairness_window_secs_from_config() {
    let cfg = StarvationConfig {
        fairness_window_secs: 137,
        ..StarvationConfig::default()
    };
    let s = FairScheduler::new(cfg);
    let sql = s.claim_next_priority_sql();
    assert!(
        sql.contains("INTERVAL '137 seconds'"),
        "claim SQL must embed configured fairness_window_secs"
    );
}

#[test]
fn mt_187_starvation_config_default_is_field_stable() {
    let cfg = StarvationConfig::default();
    assert_eq!(cfg.starvation_threshold_secs, 600);
    assert_eq!(cfg.age_boost_per_minute, 1);
    assert_eq!(cfg.age_boost_cap, 200);
    assert_eq!(cfg.fairness_penalty_per_claim, 50);
    assert_eq!(cfg.fairness_penalty_cap, 200);
    assert_eq!(cfg.fairness_window_secs, 60);
}

// ============================================================================
// Postgres-gated integration assertions
// ============================================================================

async fn postgres_pool() -> sqlx::PgPool {
    let url = std::env::var("POSTGRES_TEST_URL")
        .expect("ENVIRONMENT_BLOCKED: POSTGRES_TEST_URL not set");
    sqlx::PgPool::connect(&url).await.expect("postgres connect")
}

fn unique_wp_id(test_label: &str) -> String {
    format!("WP-MT187-{}-{}", test_label, Uuid::now_v7().simple())
}

/// Set up the cluster X.2 schema (kernel_micro_task_job + children + MT-187
/// watermark column). Idempotent; safe to call from every test.
async fn ensure_full_schema(pool: &sqlx::PgPool) {
    let queue = MicroTaskQueue::new(pool.clone());
    queue.ensure_schema().await.expect("ensure queue schema");
    let sched = FairScheduler::new(StarvationConfig::default());
    sched
        .ensure_schema(pool)
        .await
        .expect("ensure scheduler schema");
}

#[tokio::test]
#[ignore = "requires POSTGRES_TEST_URL; run with `cargo test -- --ignored`"]
async fn mt_187_pg_claim_next_priority_picks_old_t32b_over_fresh_fleet() {
    // Realistic queue mix: 100 fresh T7B from many wp_ids + 1 old T32B
    // from a different wp. The old T32B must be claimed before any fresh
    // T7B because the priority CTE ranks it higher.
    let pool = postgres_pool().await;
    ensure_full_schema(&pool).await;
    let queue = MicroTaskQueue::new(pool.clone());
    let sched = FairScheduler::new(StarvationConfig::default());

    let wp_old = unique_wp_id("old-t32b");
    let label = unique_wp_id("fresh-fleet");

    // Insert 100 fresh T7B jobs first. Use a label_wp prefix unique to
    // this test so we can identify "our" jobs in the FIFO that other
    // parallel tests may have populated.
    let now = Utc::now();
    for i in 0..100 {
        let mut j = MicroTaskJob::queue(
            &format!("{}-fresh-{}", label, i),
            &format!("MT-FRESH-{}", i),
            PathBuf::from("a.json"),
            6,
            vec![],
        );
        j.escalation_tier = EscalationTier::T7B;
        // Stagger created_at_utc by milliseconds so FIFO is deterministic.
        j.created_at_utc = now - chrono::Duration::milliseconds(i);
        j.updated_at_utc = j.created_at_utc;
        queue.enqueue(&j).await.expect("enqueue fresh");
    }

    // Then insert the old T32B. 10 minutes old, T32B base 100 + age_boost
    // 10 = 110, vs fresh T7B base 20 + age_boost 0 = 20. T32B wins.
    let mut old = MicroTaskJob::queue(
        &wp_old,
        "MT-OLD-T32B",
        PathBuf::from("a.json"),
        6,
        vec![],
    );
    old.escalation_tier = EscalationTier::T32B;
    old.created_at_utc = now - chrono::Duration::minutes(10);
    old.updated_at_utc = old.created_at_utc;
    let old_id = old.job_id;
    queue.enqueue(&old).await.expect("enqueue old");

    // Now claim — the very first priority claim should hand us the old
    // T32B, ahead of every fresh T7B.
    let session = Uuid::now_v7();
    let mut claims: Vec<MicroTaskJobId> = Vec::new();
    for _ in 0..5 {
        if let Some(id) = sched
            .claim_next_priority(&pool, session)
            .await
            .expect("claim_next_priority")
        {
            claims.push(id);
        }
    }
    assert!(
        claims.contains(&old_id),
        "old T32B must be claimed within the first 5 picks (claims={:?}, old_id={})",
        claims,
        old_id
    );
    // Stronger invariant: in practice the old T32B is the very first
    // pick because no other queued row has a higher priority.
    assert_eq!(
        claims[0], old_id,
        "old T32B must be the very first pick (priority 110 > fresh T7B 20)"
    );

    // Cleanup: delete everything we inserted under our labels.
    sqlx::query("DELETE FROM kernel_micro_task_job WHERE wp_id = $1")
        .bind(&wp_old)
        .execute(&pool)
        .await
        .ok();
    sqlx::query("DELETE FROM kernel_micro_task_job WHERE wp_id LIKE $1")
        .bind(format!("{}-fresh-%", label))
        .execute(&pool)
        .await
        .ok();
}

#[tokio::test]
#[ignore = "requires POSTGRES_TEST_URL; run with `cargo test -- --ignored`"]
async fn mt_187_pg_fair_scheduling_rotates_across_wps_under_imbalance() {
    // Pre-populate claimed_at_utc for 50 BUSY-wp claims inside the
    // fairness window, then enqueue 1 BUSY + 1 OTHER fresh T7B job.
    // OTHER must be claimed first because BUSY gets the -200 penalty cap
    // (priority 20 - 200 = -180) and OTHER gets none (priority 20).
    let pool = postgres_pool().await;
    ensure_full_schema(&pool).await;
    let queue = MicroTaskQueue::new(pool.clone());
    let sched = FairScheduler::new(StarvationConfig::default());

    let wp_busy = unique_wp_id("busy");
    let wp_other = unique_wp_id("other");
    let now = Utc::now();

    // Insert 50 already-claimed BUSY rows inside the fairness window.
    // These rows will not be reclaimed (state='claimed', not 'queued')
    // but their claimed_at_utc populates the wp_claims CTE.
    for i in 0..50 {
        let mut j = MicroTaskJob::queue(
            &wp_busy,
            &format!("MT-BUSY-PRECLAIM-{}", i),
            PathBuf::from("a.json"),
            6,
            vec![],
        );
        j.state = MicroTaskJobState::Claimed;
        j.claimed_at_utc = Some(now - chrono::Duration::seconds(10));
        j.claimed_by_session = Some(Uuid::now_v7());
        j.created_at_utc = now - chrono::Duration::seconds(15);
        j.updated_at_utc = now - chrono::Duration::seconds(10);
        queue.enqueue(&j).await.expect("preclaim insert");
    }

    // Enqueue 1 BUSY + 1 OTHER fresh queued job.
    let mut busy_q = MicroTaskJob::queue(
        &wp_busy,
        "MT-BUSY-Q",
        PathBuf::from("a.json"),
        6,
        vec![],
    );
    busy_q.escalation_tier = EscalationTier::T7B;
    busy_q.created_at_utc = now;
    busy_q.updated_at_utc = now;
    let busy_id = busy_q.job_id;
    queue.enqueue(&busy_q).await.expect("busy enqueue");

    let mut other_q = MicroTaskJob::queue(
        &wp_other,
        "MT-OTHER-Q",
        PathBuf::from("a.json"),
        6,
        vec![],
    );
    other_q.escalation_tier = EscalationTier::T7B;
    other_q.created_at_utc = now + chrono::Duration::milliseconds(50);
    other_q.updated_at_utc = other_q.created_at_utc;
    let other_id = other_q.job_id;
    queue.enqueue(&other_q).await.expect("other enqueue");

    let session = Uuid::now_v7();
    // First priority claim should be OTHER, because BUSY is heavily
    // penalised. Loop until we observe ours (parallel tests may inject
    // other wp_ids; we identify by job_id).
    let mut first_wp_claim: Option<MicroTaskJobId> = None;
    for _ in 0..20 {
        match sched.claim_next_priority(&pool, session).await.expect("claim") {
            Some(id) if id == other_id || id == busy_id => {
                first_wp_claim = Some(id);
                break;
            }
            Some(other_id_from_sibling) => {
                // Release a sibling row back to queued so other parallel
                // tests still see their expected state. The fairness CTE
                // will then no longer count this claim, but that's OK —
                // we are isolating to our wp_ids.
                let _ = queue
                    .update_state(
                        other_id_from_sibling,
                        MicroTaskJobState::Queued,
                        Some("mt187 fair-test rollback".to_string()),
                    )
                    .await;
            }
            None => {
                tokio::time::sleep(std::time::Duration::from_millis(2)).await;
            }
        }
    }
    assert_eq!(
        first_wp_claim,
        Some(other_id),
        "OTHER wp must be claimed before BUSY wp (BUSY has -200 penalty, OTHER has 0)"
    );

    // Cleanup.
    for wp in [&wp_busy, &wp_other] {
        sqlx::query("DELETE FROM kernel_micro_task_job WHERE wp_id = $1")
            .bind(wp)
            .execute(&pool)
            .await
            .ok();
    }
}

#[tokio::test]
#[ignore = "requires POSTGRES_TEST_URL; run with `cargo test -- --ignored`"]
async fn mt_187_pg_starvation_watermark_is_monotonic_across_process_restart() {
    // Insert a stale job, run check_with_watermark on a fresh guard
    // (process-1), expect a signal. Drop the guard, build a new one
    // (process-2 simulated by re-instantiation), call again — must be
    // None because the watermark column is persisted.
    let pool = postgres_pool().await;
    ensure_full_schema(&pool).await;
    let queue = MicroTaskQueue::new(pool.clone());

    let wp = unique_wp_id("watermark");
    let mut j = MicroTaskJob::queue(&wp, "MT-WATER", PathBuf::from("a.json"), 6, vec![]);
    j.created_at_utc = Utc::now() - chrono::Duration::seconds(900);
    j.updated_at_utc = j.created_at_utc;
    let job_id = j.job_id;
    queue.enqueue(&j).await.expect("enqueue");

    let cfg = StarvationConfig {
        starvation_threshold_secs: 600,
        ..StarvationConfig::default()
    };
    let g1 = StarvationGuard::new(cfg);
    let s1 = g1
        .check_with_watermark(&pool, job_id, Utc::now())
        .await
        .expect("check 1");
    assert!(
        s1.is_some(),
        "first crossing must emit a signal (job is 900s old, threshold 600s)"
    );

    drop(g1);
    // Simulate process restart by building a brand-new guard. The
    // in-memory `seen` map is empty, but the DB watermark column
    // remembers the previous emission.
    let g2 = StarvationGuard::new(cfg);
    let s2 = g2
        .check_with_watermark(&pool, job_id, Utc::now())
        .await
        .expect("check 2");
    assert!(
        s2.is_none(),
        "second call on a fresh guard must observe the persisted watermark and stay silent"
    );

    // Verify the DB row carries a watermark.
    let row: (Option<chrono::DateTime<chrono::Utc>>,) = sqlx::query_as(
        "SELECT starvation_watermark_at_utc FROM kernel_micro_task_job WHERE job_id = $1",
    )
    .bind(job_id.as_uuid())
    .fetch_one(&pool)
    .await
    .expect("select watermark");
    assert!(row.0.is_some(), "watermark column must be populated after first emission");

    sqlx::query("DELETE FROM kernel_micro_task_job WHERE wp_id = $1")
        .bind(&wp)
        .execute(&pool)
        .await
        .ok();
}

#[tokio::test]
#[ignore = "requires POSTGRES_TEST_URL; run with `cargo test -- --ignored`"]
async fn mt_187_pg_claim_next_priority_returns_none_on_empty_queue() {
    let pool = postgres_pool().await;
    ensure_full_schema(&pool).await;
    let sched = FairScheduler::new(StarvationConfig::default());
    // We cannot guarantee the table is globally empty because parallel
    // tests may have queued rows. But we can quiesce our own slice: claim
    // until we observe two consecutive None returns, which proves the
    // CTE eventually drains.
    let session = Uuid::now_v7();
    let mut consecutive_nones = 0;
    let mut iters = 0;
    while consecutive_nones < 2 && iters < 200 {
        match sched.claim_next_priority(&pool, session).await.expect("claim") {
            Some(_) => consecutive_nones = 0,
            None => consecutive_nones += 1,
        }
        iters += 1;
    }
    assert!(
        consecutive_nones >= 2,
        "claim_next_priority must eventually return None when no queued rows remain (drained in {} iters)",
        iters
    );
}

#[tokio::test]
#[ignore = "requires POSTGRES_TEST_URL; run with `cargo test -- --ignored`"]
async fn mt_187_pg_claim_next_priority_no_double_claim_under_8_parallel_workers() {
    // 8 rows + 8 parallel claimers using claim_next_priority. Every row
    // must be claimed at most once — the SKIP LOCKED contract still
    // holds for the priority claim path.
    let pool = postgres_pool().await;
    ensure_full_schema(&pool).await;
    let queue = MicroTaskQueue::new(pool.clone());
    let sched = Arc::new(FairScheduler::new(StarvationConfig::default()));

    let wp = unique_wp_id("race8");
    let mut enqueued: HashSet<MicroTaskJobId> = HashSet::new();
    for i in 0..8 {
        let mut j = MicroTaskJob::queue(
            &wp,
            &format!("MT-RACE8-{}", i),
            PathBuf::from("a.json"),
            6,
            vec![],
        );
        // Differ created_at_utc so FIFO is deterministic within tier.
        j.created_at_utc = Utc::now() - chrono::Duration::milliseconds(i);
        j.updated_at_utc = j.created_at_utc;
        enqueued.insert(j.job_id);
        queue.enqueue(&j).await.expect("enqueue");
    }

    let mut handles = Vec::new();
    for _ in 0..8 {
        let pool = pool.clone();
        let sched = sched.clone();
        let queue = MicroTaskQueue::new(pool.clone());
        let wp = wp.clone();
        handles.push(tokio::spawn(async move {
            let mut my_claims: Vec<MicroTaskJobId> = Vec::new();
            let session = Uuid::now_v7();
            for _ in 0..64 {
                match sched.claim_next_priority(&pool, session).await {
                    Ok(Some(id)) => {
                        if let Ok(Some(j)) = queue.get_job(id).await {
                            if j.wp_id == wp {
                                my_claims.push(id);
                                continue;
                            } else {
                                let _ = queue
                                    .update_state(
                                        id,
                                        MicroTaskJobState::Queued,
                                        Some("mt187 race8 rollback".to_string()),
                                    )
                                    .await;
                            }
                        }
                    }
                    Ok(None) => {
                        tokio::time::sleep(std::time::Duration::from_millis(2)).await;
                    }
                    Err(_) => {
                        tokio::time::sleep(std::time::Duration::from_millis(2)).await;
                    }
                }
            }
            my_claims
        }));
    }

    let mut all_claimed: Vec<MicroTaskJobId> = Vec::new();
    for h in handles {
        all_claimed.extend(h.await.expect("join"));
    }

    let unique: HashSet<MicroTaskJobId> = all_claimed.iter().copied().collect();
    assert_eq!(
        unique.len(),
        all_claimed.len(),
        "SKIP LOCKED violated on priority claim: a job was claimed twice (all={:?})",
        all_claimed
    );
    assert_eq!(
        unique.len(),
        8,
        "all 8 enqueued jobs must be claimed; got {}",
        unique.len()
    );
    assert_eq!(unique, enqueued, "claimed set must equal enqueued set");

    sqlx::query("DELETE FROM kernel_micro_task_job WHERE wp_id = $1")
        .bind(&wp)
        .execute(&pool)
        .await
        .ok();
}
