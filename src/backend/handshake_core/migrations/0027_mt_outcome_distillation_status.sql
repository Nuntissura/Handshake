-- WP-KERNEL-004 cluster X.2 MT-188: outcome recording + 6-level escalation
-- routing + DistillationCandidate emission hardening.
--
-- Red-team minimum_controls (MT-188 contract `red_team.minimum_controls`):
--   #2 DistillationCandidate Postgres table has a status column defaulting to
--      'LOGGING_ONLY_PHASE_1' to prevent accidental promotion before cluster C
--      pipeline lands. Implemented here as an additive ALTER ADD COLUMN with
--      IF NOT EXISTS so migration is replay-safe.
--
-- Adversarial coverage (MT-188 test surface `tests/mt_outcome_tests.rs`):
--   * "duplicate outcome on same job" — enforced by the (job_id, iteration_n)
--     UNIQUE constraint added below. Recorder writes return a typed conflict
--     error rather than silently writing a second outcome row for the same
--     iteration of the same job.
--   * "HardGate as terminal (no further escalation)" — the existing
--     queue.escalate() refuses to escalate past HardGate (queue.rs returns
--     QueueError::InvalidEscalation); this migration does not add a CHECK
--     because HardGate is recorded as a state, not as a continuation tier
--     in the chain; the application-level router test asserts the same.
--   * "forged outcome (wrong session)" — recorder validates
--     job.claimed_by_session matches the passed-in session_id before insert;
--     this migration adds no schema-level constraint because the
--     claimed_by_session column already exists on kernel_micro_task_job
--     and the validation is a Rust-level invariant (see
--     mt_outcome.rs::MtOutcomeRecorder::persist).
--
-- Schema additions are additive only; replay-safe (IF NOT EXISTS on every
-- DDL statement).

ALTER TABLE kernel_distillation_candidate
    ADD COLUMN IF NOT EXISTS status TEXT NOT NULL DEFAULT 'LOGGING_ONLY_PHASE_1';

-- Operator-rendered status column also surfaces tier_at_completion for cluster
-- C MT-XXX promotion-pipeline filter; add a composite index so the eventual
-- promotion query (status = 'LOGGING_ONLY_PHASE_1' AND tier_at_completion = ?)
-- does not full-scan once Phase 2 lands.
CREATE INDEX IF NOT EXISTS idx_kernel_distillation_candidate_status_tier
    ON kernel_distillation_candidate (status, tier_at_completion);

-- Defends the "duplicate outcome on same job" adversarial path: one outcome
-- row per (job_id, iteration_n) at the DB level so a concurrent recorder cannot
-- race past the application-level dedup check and write a second row for the
-- same iteration. The unique index is partial-safe: every existing row has
-- non-null iteration_n (BIGINT NOT NULL DEFAULT 0), so no backfill needed.
CREATE UNIQUE INDEX IF NOT EXISTS idx_kernel_mt_outcome_unique_job_iteration
    ON kernel_mt_outcome (job_id, iteration_n);

-- The kernel_mt_outcome.recorded_by_session column already exists (NOT NULL).
-- Application-level forged-outcome protection lives in
-- MtOutcomeRecorder::persist: it verifies the supplied session matches
-- kernel_micro_task_job.claimed_by_session before inserting. No schema
-- changes are needed for that invariant — flagged here so a future reader
-- knows where to find the assertion (mt_outcome.rs `verify_session_owns_job`).
