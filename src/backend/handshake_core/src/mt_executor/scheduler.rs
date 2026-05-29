//! MT-187 MT queue starvation prevention (age-based priority + fair scheduling).
//!
//! Surface:
//!   - `FairScheduler::priority(...)` — pure-function priority scorer for
//!     local in-memory candidate lists (cheap, testable).
//!   - `FairScheduler::pick_next(...)` — in-memory tie-broken picker
//!     (FIFO on `created_at_utc` when priorities tie).
//!   - `FairScheduler::claim_next_priority(...)` — async Postgres-backed
//!     server-side priority claim using a single CTE that computes
//!     `base_tier_weight + age_boost - fairness_penalty` and atomically
//!     claims via an atomic `UPDATE ... FROM candidate ... RETURNING job_id`
//!     where the candidate row is selected with `FOR UPDATE SKIP LOCKED
//!     LIMIT 1`. This is the production claim path: no client-side re-rank
//!     race (red_team #1).
//!   - `StarvationGuard::check(...)` — in-memory monotonic guard (one signal
//!     per (job_id, crossing) per process).
//!   - `StarvationGuard::check_with_watermark(...)` — Postgres-backed
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
//!   #1 Server-side priority CTE in `claim_next_priority` — no in-memory
//!      re-rank between query and claim; the CTE computes priority and the
//!      outer SELECT picks the top row in the same transaction with
//!      FOR UPDATE SKIP LOCKED.
//!   #2 Starvation watermark `starvation_watermark_at_utc` on the
//!      `kernel_micro_task_job` row makes the metric monotonic across
//!      process restarts; `check_with_watermark` sets the column on first
//!      emission and skips subsequent crossings.
//!   #3 Fairness key is computed from `kernel_micro_task_job.claimed_at_utc`
//!      rows in the last 60s, not from an in-memory counter — survives
//!      restart and is correct under multiple parallel scheduler instances.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Postgres, Transaction};
use std::collections::HashMap;
use std::sync::Mutex;
use thiserror::Error;
use uuid::Uuid;

use super::job::{MicroTaskJob, MicroTaskJobId};

#[derive(Debug, Error)]
pub enum SchedulerError {
    #[error("sqlx error: {0}")]
    Sqlx(#[from] sqlx::Error),
    #[error("serde error: {0}")]
    Serde(#[from] serde_json::Error),
}

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

    /// Apply the MT-187 schema additions (starvation watermark column and
    /// the per-wp claim-window index). Idempotent; safe to call after
    /// `MicroTaskQueue::ensure_schema`.
    pub async fn ensure_schema(&self, pool: &PgPool) -> Result<(), SchedulerError> {
        sqlx::raw_sql(SCHEDULER_SCHEMA_SQL).execute(pool).await?;
        Ok(())
    }

    /// Pure-function priority computation. Mirrors the SQL CTE used by
    /// `claim_next_priority` so test assertions on `priority(...)` reflect
    /// the same ordering the DB will produce.
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

    /// Render the priority CTE + atomic-claim SQL used by
    /// `claim_next_priority`. Exposed so the test surface can assert
    /// red_team #1 (server-side priority, no client-side re-rank race)
    /// without needing a live Postgres.
    pub fn claim_next_priority_sql(&self) -> String {
        // The CTE materialises the priority for every queued row, the
        // candidate SELECT locks the base job row, and the outer UPDATE
        // performs the state transition in the same statement.
        //
        // Notes on shape:
        //  - `base_weight` mirrors `EscalationTier::base_weight()` exactly so
        //    the wire form (`t7b`, `t7b_alt`, ...) maps to the same numeric
        //    weight Rust uses.
        //  - `age_boost` = LEAST(EXTRACT(EPOCH FROM now() - created_at_utc) /
        //    60, age_boost_cap) * age_boost_per_minute.
        //  - `fairness_penalty` = LEAST(claims_per_wp_in_window *
        //    penalty_per_claim, penalty_cap), where `claims_per_wp_in_window`
        //    is computed from `kernel_micro_task_job.claimed_at_utc` rows in
        //    the last `fairness_window_secs` seconds (red_team #3).
        format!(
            r#"WITH base AS (
    SELECT
        job_id,
        wp_id,
        escalation_tier,
        created_at_utc,
        CASE escalation_tier
            WHEN 'hard_gate' THEN 1000
            WHEN 't32b'      THEN 100
            WHEN 't13b_alt'  THEN 80
            WHEN 't13b'      THEN 60
            WHEN 't7b_alt'   THEN 40
            WHEN 't7b'       THEN 20
            ELSE 0
        END AS base_weight,
        LEAST(
            FLOOR(EXTRACT(EPOCH FROM (NOW() - created_at_utc)) / 60)::int * {age_boost_per_minute},
            {age_boost_cap}
        ) AS age_boost
    FROM kernel_micro_task_job
    WHERE state = 'queued'
),
wp_claims AS (
    SELECT wp_id, COUNT(*)::int AS claims
    FROM kernel_micro_task_job
    WHERE claimed_at_utc IS NOT NULL
      AND claimed_at_utc > NOW() - INTERVAL '{fairness_window_secs} seconds'
    GROUP BY wp_id
),
scored AS (
    SELECT
        base.job_id,
        base.wp_id,
        base.created_at_utc,
        base.base_weight
        + base.age_boost
        - LEAST(
            COALESCE(wp_claims.claims, 0) * {fairness_penalty_per_claim},
            {fairness_penalty_cap}
          ) AS priority
    FROM base
    LEFT JOIN wp_claims USING (wp_id)
),
candidate AS (
    SELECT j.job_id
    FROM kernel_micro_task_job j
    JOIN scored ON scored.job_id = j.job_id
    WHERE j.state = 'queued'
    ORDER BY scored.priority DESC, scored.created_at_utc ASC
    FOR UPDATE SKIP LOCKED
    LIMIT 1
)
UPDATE kernel_micro_task_job j
SET state = 'claimed',
    claimed_by_session = $1,
    claimed_at_utc = $2,
    updated_at_utc = $2
FROM candidate
WHERE j.job_id = candidate.job_id
  AND j.state = 'queued'
RETURNING j.job_id"#,
            age_boost_per_minute = self.cfg.age_boost_per_minute,
            age_boost_cap = self.cfg.age_boost_cap,
            fairness_window_secs = self.cfg.fairness_window_secs,
            fairness_penalty_per_claim = self.cfg.fairness_penalty_per_claim,
            fairness_penalty_cap = self.cfg.fairness_penalty_cap,
        )
    }

    /// Atomically claim the highest-priority queued job. Priority is
    /// computed server-side by a single CTE (no out-of-band re-ranking
    /// race); the claim uses `FOR UPDATE SKIP LOCKED LIMIT 1` so parallel
    /// claimers do not race on the same row.
    pub async fn claim_next_priority(
        &self,
        pool: &PgPool,
        session_id: Uuid,
    ) -> Result<Option<MicroTaskJobId>, SchedulerError> {
        let mut tx: Transaction<'_, Postgres> = pool.begin().await?;
        let now = Utc::now();
        let sql = self.claim_next_priority_sql();
        let row: Option<(Uuid,)> = sqlx::query_as(&sql)
            .bind(session_id)
            .bind(now)
            .fetch_optional(&mut *tx)
            .await?;
        let Some((id,)) = row else {
            tx.commit().await?;
            return Ok(None);
        };
        tx.commit().await?;
        Ok(Some(MicroTaskJobId(id)))
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

    /// Postgres-backed monotonic check. Sets the
    /// `starvation_watermark_at_utc` column on the
    /// `kernel_micro_task_job` row on first crossing; subsequent calls
    /// observe the watermark and return None.
    ///
    /// This satisfies red_team minimum_control #2: the metric is monotonic
    /// (no flapping) via `last_emitted_at_utc` watermark per job — and
    /// survives process restart because the watermark is durable in the
    /// DB row, not an in-memory map.
    pub async fn check_with_watermark(
        &self,
        pool: &PgPool,
        job_id: MicroTaskJobId,
        now: DateTime<Utc>,
    ) -> Result<Option<StarvationSignal>, SchedulerError> {
        let mut tx: Transaction<'_, Postgres> = pool.begin().await?;
        let row: Option<(Uuid, String, DateTime<Utc>, Option<DateTime<Utc>>)> = sqlx::query_as(
            r#"SELECT job_id, wp_id, created_at_utc, starvation_watermark_at_utc
               FROM kernel_micro_task_job
               WHERE job_id = $1
               FOR UPDATE"#,
        )
        .bind(job_id.as_uuid())
        .fetch_optional(&mut *tx)
        .await?;
        let Some((id, wp_id, created_at, watermark)) = row else {
            tx.commit().await?;
            return Ok(None);
        };
        let age = (now - created_at).num_seconds().max(0) as u64;
        if age < self.cfg.starvation_threshold_secs {
            tx.commit().await?;
            return Ok(None);
        }
        if watermark.is_some() {
            // Already emitted; honour the persisted watermark.
            tx.commit().await?;
            return Ok(None);
        }
        sqlx::query(
            r#"UPDATE kernel_micro_task_job
               SET starvation_watermark_at_utc = $1, updated_at_utc = $1
               WHERE job_id = $2"#,
        )
        .bind(now)
        .bind(id)
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(Some(StarvationSignal {
            job_id: id,
            wp_id,
            age_secs: age,
        }))
    }
}

pub const SCHEDULER_SCHEMA_SQL: &str =
    include_str!("../../migrations/0026_mt_scheduler_starvation_watermark.sql");

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mt_executor::job::EscalationTier;
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
    fn claim_next_priority_sql_uses_for_update_skip_locked_limit_1() {
        let s = FairScheduler::new(StarvationConfig::default());
        let sql = s.claim_next_priority_sql();
        assert!(
            sql.contains("FOR UPDATE SKIP LOCKED"),
            "claim SQL must use SKIP LOCKED (red_team #1)"
        );
        assert!(sql.contains("LIMIT 1"), "claim SQL must LIMIT 1");
    }

    #[test]
    fn claim_next_priority_sql_renders_all_six_tiers() {
        let s = FairScheduler::new(StarvationConfig::default());
        let sql = s.claim_next_priority_sql();
        // Every tier wire form must appear in the CASE so priority is
        // computed for every row regardless of tier.
        for tier in ["hard_gate", "t32b", "t13b_alt", "t13b", "t7b_alt", "t7b"] {
            assert!(
                sql.contains(&format!("'{}'", tier)),
                "tier {} missing from CASE",
                tier
            );
        }
    }
}
