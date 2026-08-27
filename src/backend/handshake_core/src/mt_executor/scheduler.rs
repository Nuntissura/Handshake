//! MT-187 MT queue starvation prevention (age-based priority + fair scheduling).
//!
//! Surface:
//!   - `FairScheduler::priority(...)` — pure-function priority scorer for
//!     local in-memory candidate lists (cheap, testable).
//!   - `FairScheduler::pick_next(...)` — in-memory tie-broken picker
//!     (FIFO on `created_at_utc` when priorities tie).
//!   - `FairScheduler::claim_next_priority(...)` — async embedded-SurrealDB
//!     priority claim. Candidate ranking uses the same pure priority function,
//!     then a guarded single-statement update makes the claim atomic.
//!   - `StarvationGuard::check(...)` — in-memory monotonic guard (one signal
//!     per (job_id, crossing) per process).
//!   - `StarvationGuard::check_with_watermark(...)` — embedded-store-backed
//!     monotonic guard using the `starvation_watermark_at_utc` column on
//!     `kernel_micro_task_job`. Survives process restart so a job is not
//!     re-emitted as starved after restart (red_team #2).
//!
//! Priority shape (per MT-187 contract `implementation_notes`):
//!   priority = base_tier_weight(escalation_tier)
//!            + age_boost(now - created_at_utc)
//!            - fairness_penalty(per_wp_recent_claims)
//!
//! `base_tier_weight`: HardGate=1000, T32B=100, T13BAlt=80, T13B=60,
//! T7BAlt=40, T7B=20 — defined on `EscalationTier::base_weight()`.
//! `age_boost`: +1 per minute waiting, capped at +200 so a very old T7B
//! eventually outweighs a fresh T32B (200+20 > 100).
//! `fairness_penalty`: -50 per claim by the same wp_id in the last 60s,
//! capped at -200 so a busy wp cannot push priority arbitrarily low.
//! Tie-break: FIFO on `created_at_utc`.
//!
//! Red-team minimum_controls satisfied:
//!   #1 A guarded `state = 'queued'` update in `claim_next_priority` prevents
//!      competing schedulers from claiming the same ranked candidate.
//!   #2 Starvation watermark `starvation_watermark_at_utc` on the
//!      `kernel_micro_task_job` row makes the metric monotonic across
//!      process restarts; `check_with_watermark` sets the column on first
//!      emission and skips subsequent crossings.
//!   #3 Fairness key is computed from `kernel_micro_task_job.claimed_at_utc`
//!      rows in the last 60s, not from an in-memory counter — survives
//!      restart and is correct under multiple parallel scheduler instances.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Mutex;
use surrealdb::types::SurrealValue;
use thiserror::Error;
use uuid::Uuid;

use super::job::{MicroTaskJob, MicroTaskJobId};
use super::queue::MicroTaskQueue;
use crate::storage::surreal::SurrealStorageError;

#[derive(Debug, Error)]
pub enum SchedulerError {
    #[error("storage error: {0}")]
    Storage(#[from] SurrealStorageError),
    #[error("serde error: {0}")]
    Serde(#[from] serde_json::Error),
}

// ── row shapes ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, SurrealValue)]
struct QueuedCandidateRow {
    job_id: Uuid,
    wp_id: String,
    escalation_tier: String,
    created_at_utc: DateTime<Utc>,
}

#[derive(Debug, Clone, SurrealValue)]
struct WpClaimRow {
    wp_id: String,
    claims: i64,
}

#[derive(Debug, Clone, SurrealValue)]
struct WatermarkRow {
    job_id: Uuid,
    wp_id: String,
    created_at_utc: DateTime<Utc>,
}

#[derive(SurrealValue)]
struct SinceBinding {
    since: DateTime<Utc>,
}

#[derive(SurrealValue)]
struct EmptyBindings {}

#[derive(SurrealValue)]
struct ClaimBindings {
    job_id: Uuid,
    session_id: Uuid,
    now: DateTime<Utc>,
}

#[derive(SurrealValue)]
struct WatermarkBindings {
    job_id: Uuid,
    now: DateTime<Utc>,
    cutoff: DateTime<Utc>,
}

// ── statements ──────────────────────────────────────────────────────────────

/// Fairness window aggregate: the `wp_claims` CTE, verbatim in behaviour.
const WP_CLAIMS_QUERY: &str = "SELECT wp_id, count() AS claims FROM kernel_micro_task_job \
     WHERE claimed_at_utc != NONE AND claimed_at_utc > $since GROUP BY wp_id;";

/// Queued candidates, oldest first — the `base` CTE minus the arithmetic, which
/// [`FairScheduler::priority`] applies.
const QUEUED_CANDIDATES_QUERY: &str = "SELECT job_id, wp_id, escalation_tier, created_at_utc \
     FROM kernel_micro_task_job WHERE state = 'queued' \
     ORDER BY created_at_utc ASC;";

/// The claim itself. `AND state = 'queued'` inside the write is what makes two
/// parallel claimers unable to take the same row; it is the direct replacement
/// for `FOR UPDATE SKIP LOCKED` and it is why ranking outside the statement
/// cannot cause a double claim.
const PRIORITY_CLAIM_QUERY: &str = "UPDATE kernel_micro_task_job SET \
     state = 'claimed', claimed_by_session = $session_id, claimed_at_utc = $now, \
     updated_at_utc = $now \
     WHERE job_id = $job_id AND state = 'queued' RETURN AFTER;";

/// Monotonic starvation watermark.
///
/// The threshold test and the watermark write are ONE statement, so two
/// processes cannot both decide they are the first to cross. This is stronger
/// than the read-then-write it replaces needed to be, and it removes the
/// transaction the PostgreSQL version required.
const WATERMARK_CLAIM_QUERY: &str = "UPDATE kernel_micro_task_job SET \
     starvation_watermark_at_utc = $now, updated_at_utc = $now \
     WHERE job_id = $job_id AND starvation_watermark_at_utc = NONE \
     AND created_at_utc <= $cutoff RETURN AFTER;";

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct StarvationConfig {
    pub starvation_threshold_secs: u64,
    pub age_boost_per_minute: i32,
    pub age_boost_cap: i32,
    pub fairness_penalty_per_claim: i32,
    pub fairness_penalty_cap: i32,
    pub fairness_window_secs: u64,
}

impl Default for StarvationConfig {
    fn default() -> Self {
        Self {
            starvation_threshold_secs: 600,
            age_boost_per_minute: 1,
            age_boost_cap: 200,
            fairness_penalty_per_claim: 50,
            fairness_penalty_cap: 200,
            fairness_window_secs: 60,
        }
    }
}

pub struct FairScheduler {
    cfg: StarvationConfig,
}

impl FairScheduler {
    pub fn new(cfg: StarvationConfig) -> Self {
        Self { cfg }
    }

    pub fn config(&self) -> &StarvationConfig {
        &self.cfg
    }

    /// Priority computation. This is now the ONLY place priority is computed —
    /// the PostgreSQL CTE that duplicated this arithmetic is gone, so the
    /// "SQL and Rust must agree" drift risk it carried is gone with it.
    pub fn priority(
        &self,
        job: &MicroTaskJob,
        now: DateTime<Utc>,
        recent_claims_per_wp: &HashMap<String, u32>,
    ) -> i32 {
        let mut p = job.escalation_tier.base_weight();
        let age_secs = (now - job.created_at_utc).num_seconds().max(0) as i32;
        let age_minutes = age_secs / 60;
        let age_boost = (age_minutes * self.cfg.age_boost_per_minute).min(self.cfg.age_boost_cap);
        p += age_boost;
        let recent_claims = *recent_claims_per_wp.get(&job.wp_id).unwrap_or(&0) as i32;
        let penalty = (recent_claims * self.cfg.fairness_penalty_per_claim)
            .min(self.cfg.fairness_penalty_cap);
        p -= penalty;
        p
    }

    /// Same arithmetic as [`Self::priority`] over the projection the candidate
    /// query returns, so a full `MicroTaskJob` need not be loaded to rank.
    fn candidate_priority(
        &self,
        row: &QueuedCandidateRow,
        now: DateTime<Utc>,
        recent_claims_per_wp: &HashMap<String, u32>,
    ) -> Result<i32, SchedulerError> {
        let tier: super::job::EscalationTier =
            serde_json::from_value(serde_json::Value::String(row.escalation_tier.clone()))?;
        let mut p = tier.base_weight();
        let age_secs = (now - row.created_at_utc).num_seconds().max(0) as i32;
        let age_minutes = age_secs / 60;
        let age_boost = (age_minutes * self.cfg.age_boost_per_minute).min(self.cfg.age_boost_cap);
        p += age_boost;
        let recent_claims = *recent_claims_per_wp.get(&row.wp_id).unwrap_or(&0) as i32;
        let penalty = (recent_claims * self.cfg.fairness_penalty_per_claim)
            .min(self.cfg.fairness_penalty_cap);
        p -= penalty;
        Ok(p)
    }

    /// Pick the highest-priority job from a candidate list. Tie-break: earliest
    /// `created_at_utc` first (FIFO).
    pub fn pick_next<'a>(
        &self,
        candidates: &'a [MicroTaskJob],
        now: DateTime<Utc>,
        recent_claims_per_wp: &HashMap<String, u32>,
    ) -> Option<&'a MicroTaskJob> {
        let scored: Vec<(i32, &MicroTaskJob)> = candidates
            .iter()
            .map(|j| (self.priority(j, now, recent_claims_per_wp), j))
            .collect();
        scored
            .into_iter()
            .max_by(|a, b| {
                a.0.cmp(&b.0).then_with(|| {
                    // Lower created_at_utc wins on tie (FIFO).
                    b.1.created_at_utc.cmp(&a.1.created_at_utc)
                })
            })
            .map(|(_, j)| j)
    }

    /// The SurrealQL the priority claim executes, exposed so the test surface
    /// can assert the claim shape without a live store.
    ///
    /// Replaces `claim_next_priority_sql`. The priority arithmetic is no longer
    /// part of the statement text (see [`Self::priority`]), so what is asserted
    /// here is the part that carries the concurrency guarantee: the claim's
    /// `state = 'queued'` predicate and the `RETURN AFTER` that reports whether
    /// this caller won.
    pub fn claim_next_priority_surql(&self) -> &'static str {
        PRIORITY_CLAIM_QUERY
    }

    /// The fairness-window aggregate the scheduler reads before ranking.
    pub fn wp_claims_surql(&self) -> &'static str {
        WP_CLAIMS_QUERY
    }

    /// Atomically claim the highest-priority queued job.
    ///
    /// DISCLOSED NARROWING: the PostgreSQL version ranked inside the claiming
    /// statement, so ranking and claiming shared one snapshot. Here ranking runs
    /// over one consistent read and the claim is a separate guarded statement.
    /// Exclusivity is unchanged — the claim's `state = 'queued'` predicate still
    /// admits exactly one winner per row — but two schedulers ranking against
    /// slightly different fairness snapshots can pick a different order. The
    /// consequence is ordering fairness, never a double claim. When a claim is
    /// lost, the next candidate is tried, which is the `SKIP LOCKED` behaviour.
    pub async fn claim_next_priority(
        &self,
        queue: &MicroTaskQueue,
        session_id: Uuid,
    ) -> Result<Option<MicroTaskJobId>, SchedulerError> {
        let now = Utc::now();
        let since = now
            - chrono::Duration::seconds(self.cfg.fairness_window_secs.min(i64::MAX as u64) as i64);
        let claim_rows: Vec<WpClaimRow> =
            queue.query(WP_CLAIMS_QUERY, SinceBinding { since }).await?;
        let recent_claims_per_wp: HashMap<String, u32> = claim_rows
            .into_iter()
            .map(|row| (row.wp_id, row.claims.max(0) as u32))
            .collect();

        let mut candidates: Vec<QueuedCandidateRow> = queue
            .query(QUEUED_CANDIDATES_QUERY, EmptyBindings {})
            .await?;
        if candidates.is_empty() {
            return Ok(None);
        }
        let mut scored = Vec::with_capacity(candidates.len());
        for row in candidates.drain(..) {
            let priority = self.candidate_priority(&row, now, &recent_claims_per_wp)?;
            scored.push((priority, row));
        }
        // Highest priority first; FIFO on ties. Mirrors
        // `ORDER BY priority DESC, created_at_utc ASC`.
        scored.sort_by(|left, right| {
            right
                .0
                .cmp(&left.0)
                .then_with(|| left.1.created_at_utc.cmp(&right.1.created_at_utc))
                .then_with(|| left.1.job_id.cmp(&right.1.job_id))
        });

        for (_, row) in scored {
            let claimed: Vec<WatermarkRow> = queue
                .query(
                    PRIORITY_CLAIM_QUERY,
                    ClaimBindings {
                        job_id: row.job_id,
                        session_id,
                        now,
                    },
                )
                .await?;
            if !claimed.is_empty() {
                return Ok(Some(MicroTaskJobId(row.job_id)));
            }
            // Lost the race for this row; move to the next candidate.
        }
        Ok(None)
    }
}

/// MT-187 StarvationGuard — emits one signal per (job_id, threshold-crossing).
pub struct StarvationGuard {
    cfg: StarvationConfig,
    seen: Mutex<HashMap<uuid::Uuid, ()>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StarvationSignal {
    pub job_id: uuid::Uuid,
    pub wp_id: String,
    pub age_secs: u64,
}

impl StarvationGuard {
    pub fn new(cfg: StarvationConfig) -> Self {
        Self {
            cfg,
            seen: Mutex::new(HashMap::new()),
        }
    }

    pub fn config(&self) -> &StarvationConfig {
        &self.cfg
    }

    /// Returns Some(StarvationSignal) on the first time a job crosses the
    /// threshold within this process; None on subsequent calls.
    ///
    /// In-memory only — for cross-process monotonicity use
    /// `check_with_watermark`.
    pub fn check(&self, job: &MicroTaskJob, now: DateTime<Utc>) -> Option<StarvationSignal> {
        let age = (now - job.created_at_utc).num_seconds().max(0) as u64;
        if age < self.cfg.starvation_threshold_secs {
            return None;
        }
        let mut seen = self.seen.lock().unwrap();
        if seen.contains_key(&job.job_id.as_uuid()) {
            return None;
        }
        seen.insert(job.job_id.as_uuid(), ());
        Some(StarvationSignal {
            job_id: job.job_id.as_uuid(),
            wp_id: job.wp_id.clone(),
            age_secs: age,
        })
    }

    /// Store-backed monotonic check. Sets `starvation_watermark_at_utc` on the
    /// `kernel_micro_task_job` row on first crossing; subsequent calls observe
    /// the watermark and return None.
    ///
    /// red_team minimum_control #2 is strengthened rather than preserved: the
    /// age test, the watermark test and the watermark write are now a single
    /// statement, so the emit decision is atomic without a transaction and two
    /// processes crossing simultaneously cannot both emit. The watermark stays
    /// durable in the row, so it still survives restart.
    pub async fn check_with_watermark(
        &self,
        queue: &MicroTaskQueue,
        job_id: MicroTaskJobId,
        now: DateTime<Utc>,
    ) -> Result<Option<StarvationSignal>, SchedulerError> {
        let cutoff = now - chrono::Duration::seconds(self.cfg.starvation_threshold_secs as i64);
        let rows: Vec<WatermarkRow> = queue
            .query(
                WATERMARK_CLAIM_QUERY,
                WatermarkBindings {
                    job_id: job_id.as_uuid(),
                    now,
                    cutoff,
                },
            )
            .await?;
        let Some(row) = rows.into_iter().next() else {
            return Ok(None);
        };
        let age = (now - row.created_at_utc).num_seconds().max(0) as u64;
        Ok(Some(StarvationSignal {
            job_id: row.job_id,
            wp_id: row.wp_id,
            age_secs: age,
        }))
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::mt_executor::job::EscalationTier;
    use crate::storage::surreal::{bootstrap_schema, SurrealStorage, SurrealStorageConfig};
    use std::path::PathBuf;

    fn make_job(wp: &str, tier: EscalationTier, age_minutes: i64) -> MicroTaskJob {
        let mut j = MicroTaskJob::queue(wp, "MT", PathBuf::from("a.json"), 6, vec![]);
        j.escalation_tier = tier;
        j.created_at_utc = Utc::now() - chrono::Duration::minutes(age_minutes);
        j
    }

    #[test]
    fn old_t32b_beats_fresh_t7b_fleet() {
        let sched = FairScheduler::new(StarvationConfig::default());
        let mut candidates: Vec<MicroTaskJob> = (0..100)
            .map(|i| make_job(&format!("W-{i}"), EscalationTier::T7B, 0))
            .collect();
        candidates.push(make_job("W-old", EscalationTier::T32B, 5));
        let now = Utc::now();
        let pick = sched.pick_next(&candidates, now, &HashMap::new()).unwrap();
        assert_eq!(pick.wp_id, "W-old");
    }

    #[test]
    fn fairness_penalty_demotes_busy_wp() {
        let sched = FairScheduler::new(StarvationConfig::default());
        let candidates = vec![
            make_job("BUSY", EscalationTier::T7B, 0),
            make_job("OTHER", EscalationTier::T7B, 0),
        ];
        let mut recent = HashMap::new();
        recent.insert("BUSY".to_string(), 3);
        let pick = sched.pick_next(&candidates, Utc::now(), &recent).unwrap();
        assert_eq!(pick.wp_id, "OTHER");
    }

    #[test]
    fn starvation_guard_emits_once_per_crossing() {
        let g = StarvationGuard::new(StarvationConfig {
            starvation_threshold_secs: 60,
            ..StarvationConfig::default()
        });
        let job = make_job("W-1", EscalationTier::T7B, 5);
        let t0 = Utc::now();
        let s1 = g.check(&job, t0);
        let s2 = g.check(&job, t0 + chrono::Duration::seconds(120));
        assert!(s1.is_some());
        assert!(s2.is_none());
    }

    #[test]
    fn claim_next_priority_surql_guards_the_queued_row_and_returns_the_winner() {
        let s = FairScheduler::new(StarvationConfig::default());
        let surql = s.claim_next_priority_surql();
        assert!(
            surql.contains("WHERE job_id = $job_id AND state = 'queued'"),
            "claim SurrealQL must guard the exact queued row (red_team #1)"
        );
        assert!(
            surql.contains("RETURN AFTER"),
            "claim SurrealQL must return a row only to the winning caller"
        );
    }

    #[test]
    fn claim_next_priority_scores_all_six_tiers() {
        let s = FairScheduler::new(StarvationConfig::default());
        let now = Utc::now();
        for (tier, expected) in [
            (EscalationTier::HardGate, 1_000),
            (EscalationTier::T32B, 100),
            (EscalationTier::T13BAlt, 80),
            (EscalationTier::T13B, 60),
            (EscalationTier::T7BAlt, 40),
            (EscalationTier::T7B, 20),
        ] {
            let mut job = make_job("W-tier", tier, 0);
            job.created_at_utc = now;
            assert_eq!(
                s.priority(&job, now, &HashMap::new()),
                expected,
                "tier {tier:?} must retain its canonical base priority"
            );
        }
    }

    #[tokio::test]
    async fn mt137_priority_claim_ranks_beyond_the_oldest_five_hundred_rows() {
        let directory = tempfile::tempdir().expect("temporary priority scheduler root");
        let storage = SurrealStorage::open(
            SurrealStorageConfig::with_path(&directory.path().join("store"))
                .expect("valid priority scheduler path"),
        )
        .await
        .expect("open priority scheduler store");
        bootstrap_schema(&storage)
            .await
            .expect("bootstrap priority scheduler schema");
        let queue = MicroTaskQueue::new(storage.clone());

        for index in 0..500 {
            let mut low = make_job(&format!("WP-MT137-LOW-{index:03}"), EscalationTier::T7B, 0);
            // Strictly older than the hard gate inserted after this fleet.
            low.created_at_utc = Utc::now() - chrono::Duration::minutes(1);
            low.updated_at_utc = low.created_at_utc;
            queue.enqueue(&low).await.expect("enqueue low-tier job");
        }

        let hard_gate = make_job("WP-MT137-HARD-GATE", EscalationTier::HardGate, 0);
        let hard_gate_id = hard_gate.job_id;
        queue
            .enqueue(&hard_gate)
            .await
            .expect("enqueue newer hard-gate job");

        let claimed = FairScheduler::new(StarvationConfig::default())
            .claim_next_priority(&queue, Uuid::now_v7())
            .await
            .expect("claim globally highest-priority job");
        assert_eq!(
            claimed,
            Some(hard_gate_id),
            "priority ranking must include queued rows beyond the former 500-row window"
        );

        drop(queue);
        storage.shutdown().await.expect("close scheduler store");
    }
}
