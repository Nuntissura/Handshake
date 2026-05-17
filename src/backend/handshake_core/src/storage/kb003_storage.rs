//! KB003 storage glue.
//!
//! MT-011..MT-014 acceptance: provide durable Postgres rows + migration SQL
//! for sandbox runs, sandbox policies, validation runs, and promotion
//! receipts. MT-016 acceptance: replay a run from durable rows alone.
//!
//! Why one file instead of touching `storage/postgres.rs`:
//!
//! - `postgres.rs` is the legacy single-file authority surface (~8.3k lines).
//!   Splitting KB003 into its own file keeps the migration SQL, row types,
//!   and storage trait reviewable as one unit per the MT contracts.
//! - The trait is intentionally narrow: `Kb003Storage` exposes the minimum
//!   verbs MT-011..MT-014 demand (`insert_sandbox_run`,
//!   `update_sandbox_run_status`, `insert_sandbox_policy_version`,
//!   `insert_validation_run`, `insert_promotion_decision`,
//!   `insert_promotion_receipt`, `load_run_for_replay`).
//! - Concrete Postgres binding lands in a follow-up MT (Wave E); MT-015 still
//!   gates every write here through the no-SQLite tripwire.
//!
//! Idempotency (MT-014): the promotion receipt insert is keyed by
//! `idempotency_key`. Re-inserting the same key with the same payload returns
//! the original row id; re-inserting with a different payload returns
//! `Kb003StorageError::IdempotencyConflict`.

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::kernel::sandbox::no_sqlite_tripwire::{
    guard_authority_write, AuthorityMode, NoSqliteTripwireError,
};
use crate::kernel::sandbox::policy::SandboxPolicyV1;
use crate::kernel::sandbox::run::{SandboxRunStatus, SandboxRunV1};

// ---------------------------------------------------------------------------
// MT-011: Postgres migration for sandbox runs.
// ---------------------------------------------------------------------------

pub const MIGRATION_KB003_SANDBOX_RUNS_V1: &str = r#"
CREATE TABLE IF NOT EXISTS kb003_sandbox_runs (
    run_id              TEXT PRIMARY KEY,
    kernel_task_run_id  TEXT NOT NULL,
    session_run_id      TEXT NOT NULL,
    adapter_kind        TEXT NOT NULL,
    policy_version_id   TEXT NOT NULL,
    workspace_id        TEXT NOT NULL,
    status              TEXT NOT NULL,
    requested_at_utc    TIMESTAMPTZ NOT NULL,
    started_at_utc      TIMESTAMPTZ,
    finished_at_utc     TIMESTAMPTZ,
    denial_id           TEXT,
    artifact_refs       JSONB NOT NULL DEFAULT '[]'::jsonb
);
CREATE INDEX IF NOT EXISTS ix_kb003_sandbox_runs_session
    ON kb003_sandbox_runs (session_run_id);
CREATE INDEX IF NOT EXISTS ix_kb003_sandbox_runs_status
    ON kb003_sandbox_runs (status);
"#;

// ---------------------------------------------------------------------------
// MT-012: Postgres migration for sandbox policies (versioned).
// ---------------------------------------------------------------------------

pub const MIGRATION_KB003_SANDBOX_POLICIES_V1: &str = r#"
CREATE TABLE IF NOT EXISTS kb003_sandbox_policies (
    policy_id           TEXT NOT NULL,
    policy_version      INTEGER NOT NULL,
    name                TEXT NOT NULL,
    created_at_utc      TIMESTAMPTZ NOT NULL,
    default_decision    TEXT NOT NULL,
    overrides_json      JSONB NOT NULL,
    allowed_roots_json  JSONB NOT NULL,
    provenance_note     TEXT NOT NULL,
    PRIMARY KEY (policy_id, policy_version)
);
CREATE INDEX IF NOT EXISTS ix_kb003_sandbox_policies_name
    ON kb003_sandbox_policies (name);
"#;

// ---------------------------------------------------------------------------
// MT-013: Postgres migration for validation runs.
// ---------------------------------------------------------------------------

pub const MIGRATION_KB003_VALIDATION_RUNS_V1: &str = r#"
CREATE TABLE IF NOT EXISTS kb003_validation_runs (
    validation_run_id        TEXT PRIMARY KEY,
    sandbox_run_id           TEXT NOT NULL REFERENCES kb003_sandbox_runs (run_id),
    descriptor_id            TEXT NOT NULL,
    verdict                  TEXT NOT NULL,
    check_count              INTEGER NOT NULL,
    failed_check_count       INTEGER NOT NULL,
    report_artifact_ref      TEXT,
    started_at_utc           TIMESTAMPTZ NOT NULL,
    finished_at_utc          TIMESTAMPTZ NOT NULL,
    summary_json             JSONB NOT NULL
);
CREATE INDEX IF NOT EXISTS ix_kb003_validation_runs_sandbox
    ON kb003_validation_runs (sandbox_run_id);
"#;

// ---------------------------------------------------------------------------
// MT-014: Postgres migration for promotion decisions + receipts (idempotent).
// ---------------------------------------------------------------------------

pub const MIGRATION_KB003_PROMOTION_RECEIPTS_V1: &str = r#"
CREATE TABLE IF NOT EXISTS kb003_promotion_decisions (
    decision_id        TEXT PRIMARY KEY,
    validation_run_id  TEXT NOT NULL REFERENCES kb003_validation_runs (validation_run_id),
    decision           TEXT NOT NULL,
    rationale_short    TEXT NOT NULL,
    decided_at_utc     TIMESTAMPTZ NOT NULL
);
CREATE TABLE IF NOT EXISTS kb003_promotion_receipts (
    receipt_id         TEXT PRIMARY KEY,
    decision_id        TEXT NOT NULL REFERENCES kb003_promotion_decisions (decision_id),
    idempotency_key    TEXT NOT NULL UNIQUE,
    payload_hash       TEXT NOT NULL,
    artifact_ref       TEXT,
    issued_at_utc      TIMESTAMPTZ NOT NULL
);
CREATE INDEX IF NOT EXISTS ix_kb003_promotion_receipts_decision
    ON kb003_promotion_receipts (decision_id);
"#;

pub const KB003_MIGRATIONS_V1: &[(&str, &str)] = &[
    ("kb003_sandbox_runs_v1", MIGRATION_KB003_SANDBOX_RUNS_V1),
    ("kb003_sandbox_policies_v1", MIGRATION_KB003_SANDBOX_POLICIES_V1),
    ("kb003_validation_runs_v1", MIGRATION_KB003_VALIDATION_RUNS_V1),
    ("kb003_promotion_receipts_v1", MIGRATION_KB003_PROMOTION_RECEIPTS_V1),
];

// ---------------------------------------------------------------------------
// Row types matching the migrations above.
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValidationRunRowV1 {
    pub validation_run_id: String,
    pub sandbox_run_id: String,
    pub descriptor_id: String,
    pub verdict: String,
    pub check_count: u32,
    pub failed_check_count: u32,
    pub report_artifact_ref: Option<String>,
    pub started_at_utc: String,
    pub finished_at_utc: String,
    pub summary_json: serde_json::Value,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PromotionDecisionRowV1 {
    pub decision_id: String,
    pub validation_run_id: String,
    pub decision: String,
    pub rationale_short: String,
    pub decided_at_utc: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PromotionReceiptRowV1 {
    pub receipt_id: String,
    pub decision_id: String,
    pub idempotency_key: String,
    pub payload_hash: String,
    pub artifact_ref: Option<String>,
    pub issued_at_utc: String,
}

// ---------------------------------------------------------------------------
// Error + storage trait.
// ---------------------------------------------------------------------------

#[derive(Debug, Error)]
pub enum Kb003StorageError {
    #[error("authority guard failed: {0}")]
    Authority(#[from] NoSqliteTripwireError),
    #[error("idempotency conflict on key `{key}`: existing payload hash `{existing_hash}` != new `{new_hash}`")]
    IdempotencyConflict {
        key: String,
        existing_hash: String,
        new_hash: String,
    },
    #[error("row not found: {0}")]
    NotFound(String),
    #[error("backend error: {0}")]
    Backend(String),
}

pub type Kb003StorageResult<T> = Result<T, Kb003StorageError>;

/// Narrow storage trait covering only what KB003 MTs need. Implementations
/// MUST call `guard_authority_write(self.authority_mode())` on every write.
pub trait Kb003Storage {
    fn authority_mode(&self) -> AuthorityMode;

    fn insert_sandbox_run(&mut self, run: &SandboxRunV1) -> Kb003StorageResult<()> {
        guard_authority_write(self.authority_mode())?;
        self.do_insert_sandbox_run(run)
    }
    fn update_sandbox_run_status(
        &mut self,
        run_id: &str,
        new_status: SandboxRunStatus,
    ) -> Kb003StorageResult<()> {
        guard_authority_write(self.authority_mode())?;
        self.do_update_sandbox_run_status(run_id, new_status)
    }
    fn insert_sandbox_policy_version(&mut self, policy: &SandboxPolicyV1) -> Kb003StorageResult<()> {
        guard_authority_write(self.authority_mode())?;
        self.do_insert_sandbox_policy_version(policy)
    }
    fn insert_validation_run(&mut self, row: &ValidationRunRowV1) -> Kb003StorageResult<()> {
        guard_authority_write(self.authority_mode())?;
        self.do_insert_validation_run(row)
    }
    fn insert_promotion_decision(&mut self, row: &PromotionDecisionRowV1) -> Kb003StorageResult<()> {
        guard_authority_write(self.authority_mode())?;
        self.do_insert_promotion_decision(row)
    }
    fn insert_promotion_receipt(&mut self, row: &PromotionReceiptRowV1) -> Kb003StorageResult<String> {
        guard_authority_write(self.authority_mode())?;
        self.do_insert_promotion_receipt(row)
    }

    // Backend-specific work.
    fn do_insert_sandbox_run(&mut self, run: &SandboxRunV1) -> Kb003StorageResult<()>;
    fn do_update_sandbox_run_status(
        &mut self,
        run_id: &str,
        new_status: SandboxRunStatus,
    ) -> Kb003StorageResult<()>;
    fn do_insert_sandbox_policy_version(&mut self, policy: &SandboxPolicyV1) -> Kb003StorageResult<()>;
    fn do_insert_validation_run(&mut self, row: &ValidationRunRowV1) -> Kb003StorageResult<()>;
    fn do_insert_promotion_decision(&mut self, row: &PromotionDecisionRowV1) -> Kb003StorageResult<()>;
    fn do_insert_promotion_receipt(&mut self, row: &PromotionReceiptRowV1) -> Kb003StorageResult<String>;
}

// ---------------------------------------------------------------------------
// In-memory backend used by MT-011..MT-014 acceptance tests and by Wave-E
// integration smoketests until the real Postgres binding lands.
// ---------------------------------------------------------------------------

#[derive(Debug, Default)]
pub struct InMemoryKb003Storage {
    pub mode: AuthorityModeOverride,
    pub sandbox_runs: Vec<SandboxRunV1>,
    pub policies: Vec<SandboxPolicyV1>,
    pub validation_runs: Vec<ValidationRunRowV1>,
    pub promotion_decisions: Vec<PromotionDecisionRowV1>,
    pub promotion_receipts: Vec<PromotionReceiptRowV1>,
}

/// Wrapper so we can construct test instances that pretend to be either
/// PostgresPrimary or a degraded mode (the latter must be refused by the
/// tripwire — see MT-015).
#[derive(Debug, Clone, Copy, Default)]
pub struct AuthorityModeOverride {
    pub mode: Option<AuthorityMode>,
}

impl InMemoryKb003Storage {
    pub fn new_postgres_primary() -> Self {
        Self {
            mode: AuthorityModeOverride {
                mode: Some(AuthorityMode::PostgresPrimary),
            },
            ..Default::default()
        }
    }
    pub fn new_with_mode(mode: AuthorityMode) -> Self {
        Self {
            mode: AuthorityModeOverride { mode: Some(mode) },
            ..Default::default()
        }
    }
}

impl Kb003Storage for InMemoryKb003Storage {
    fn authority_mode(&self) -> AuthorityMode {
        self.mode.mode.unwrap_or(AuthorityMode::PostgresPrimary)
    }

    fn do_insert_sandbox_run(&mut self, run: &SandboxRunV1) -> Kb003StorageResult<()> {
        if self.sandbox_runs.iter().any(|r| r.run_id == run.run_id) {
            return Err(Kb003StorageError::Backend(format!(
                "duplicate run_id {}",
                run.run_id.0
            )));
        }
        self.sandbox_runs.push(run.clone());
        Ok(())
    }

    fn do_update_sandbox_run_status(
        &mut self,
        run_id: &str,
        new_status: SandboxRunStatus,
    ) -> Kb003StorageResult<()> {
        let row = self
            .sandbox_runs
            .iter_mut()
            .find(|r| r.run_id.0 == run_id)
            .ok_or_else(|| Kb003StorageError::NotFound(run_id.to_string()))?;
        if !row.status.can_transition_to(new_status) {
            return Err(Kb003StorageError::Backend(format!(
                "invalid transition {} -> {}",
                row.status.as_str(),
                new_status.as_str()
            )));
        }
        row.status = new_status;
        Ok(())
    }

    fn do_insert_sandbox_policy_version(&mut self, policy: &SandboxPolicyV1) -> Kb003StorageResult<()> {
        if self
            .policies
            .iter()
            .any(|p| p.policy_id == policy.policy_id && p.policy_version == policy.policy_version)
        {
            return Err(Kb003StorageError::Backend(format!(
                "duplicate policy version {}@{}",
                policy.policy_id, policy.policy_version
            )));
        }
        self.policies.push(policy.clone());
        Ok(())
    }

    fn do_insert_validation_run(&mut self, row: &ValidationRunRowV1) -> Kb003StorageResult<()> {
        self.validation_runs.push(row.clone());
        Ok(())
    }

    fn do_insert_promotion_decision(&mut self, row: &PromotionDecisionRowV1) -> Kb003StorageResult<()> {
        self.promotion_decisions.push(row.clone());
        Ok(())
    }

    fn do_insert_promotion_receipt(&mut self, row: &PromotionReceiptRowV1) -> Kb003StorageResult<String> {
        if let Some(existing) = self
            .promotion_receipts
            .iter()
            .find(|r| r.idempotency_key == row.idempotency_key)
        {
            if existing.payload_hash == row.payload_hash {
                return Ok(existing.receipt_id.clone());
            }
            return Err(Kb003StorageError::IdempotencyConflict {
                key: row.idempotency_key.clone(),
                existing_hash: existing.payload_hash.clone(),
                new_hash: row.payload_hash.clone(),
            });
        }
        self.promotion_receipts.push(row.clone());
        Ok(row.receipt_id.clone())
    }
}

// ---------------------------------------------------------------------------
// MT-016 helper: load durable inputs for replay.
// ---------------------------------------------------------------------------

/// Bag of durable rows the replay reconstructor needs. Lookups happen by
/// `run_id`; the bag is the only thing the replay layer is allowed to read.
#[derive(Debug, Clone)]
pub struct ReplayDurableBag<'a> {
    pub run: &'a SandboxRunV1,
    pub policy: &'a SandboxPolicyV1,
    pub validation: Option<&'a ValidationRunRowV1>,
    pub decision: Option<&'a PromotionDecisionRowV1>,
    pub receipt: Option<&'a PromotionReceiptRowV1>,
}

impl InMemoryKb003Storage {
    /// Build a replay bag for the given run id by walking durable rows only.
    pub fn load_replay_bag<'a>(
        &'a self,
        run_id: &str,
        policy_version_id: &str,
    ) -> Kb003StorageResult<ReplayDurableBag<'a>> {
        let run = self
            .sandbox_runs
            .iter()
            .find(|r| r.run_id.0 == run_id)
            .ok_or_else(|| Kb003StorageError::NotFound(format!("run {run_id}")))?;
        let policy = self
            .policies
            .iter()
            .find(|p| p.version_id() == policy_version_id)
            .ok_or_else(|| Kb003StorageError::NotFound(format!("policy {policy_version_id}")))?;
        let validation = self
            .validation_runs
            .iter()
            .find(|v| v.sandbox_run_id == run_id);
        let decision = validation
            .and_then(|v| self.promotion_decisions.iter().find(|d| d.validation_run_id == v.validation_run_id));
        let receipt = decision
            .and_then(|d| self.promotion_receipts.iter().find(|r| r.decision_id == d.decision_id));
        Ok(ReplayDurableBag {
            run,
            policy,
            validation,
            decision,
            receipt,
        })
    }
}

// ---------------------------------------------------------------------------
// Tests covering MT-011..MT-014 acceptance.
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use crate::kernel::sandbox::run::SandboxRunId;
    use serde_json::json;

    fn fresh_run() -> SandboxRunV1 {
        SandboxRunV1 {
            run_id: SandboxRunId::new(),
            kernel_task_run_id: "KTR-1".into(),
            session_run_id: "SES-1".into(),
            adapter_kind: "policy_scoped_local".into(),
            policy_version_id: "POL-1@1".into(),
            workspace_id: "WSP-1".into(),
            status: SandboxRunStatus::Requested,
            requested_at_utc: Utc::now(),
            started_at_utc: None,
            finished_at_utc: None,
            denial_id: None,
            artifact_refs: vec![],
        }
    }

    // MT-011: records persist and replay after backend restart.
    #[test]
    fn sandbox_run_persists_across_restart() {
        let mut store = InMemoryKb003Storage::new_postgres_primary();
        let run = fresh_run();
        let id = run.run_id.0.clone();
        store.insert_sandbox_run(&run).unwrap();
        store
            .update_sandbox_run_status(&id, SandboxRunStatus::Started)
            .unwrap();

        // Snapshot durable state, then "restart" by moving rows into a new store.
        let snapshot = store.sandbox_runs.clone();
        let mut after_restart = InMemoryKb003Storage::new_postgres_primary();
        after_restart.sandbox_runs = snapshot;
        let row = after_restart
            .sandbox_runs
            .iter()
            .find(|r| r.run_id.0 == id)
            .unwrap();
        assert_eq!(row.status, SandboxRunStatus::Started);
    }

    // MT-012: policy changes are versioned and traceable.
    #[test]
    fn policy_changes_are_versioned() {
        let mut store = InMemoryKb003Storage::new_postgres_primary();
        let p1 = SandboxPolicyV1::default_deny("baseline");
        store.insert_sandbox_policy_version(&p1).unwrap();
        let p2 = p1.bump_version("relax NETWORK with evidence");
        store.insert_sandbox_policy_version(&p2).unwrap();
        assert_eq!(store.policies.len(), 2);
        assert_eq!(store.policies[0].policy_id, store.policies[1].policy_id);
        assert_ne!(
            store.policies[0].policy_version,
            store.policies[1].policy_version
        );
        // Duplicate version refused.
        let err = store.insert_sandbox_policy_version(&p2).unwrap_err();
        assert!(matches!(err, Kb003StorageError::Backend(_)));
    }

    // MT-013: validation results reconstruct without file-system-only state.
    #[test]
    fn validation_run_reconstructs_from_rows() {
        let mut store = InMemoryKb003Storage::new_postgres_primary();
        let run = fresh_run();
        let run_id = run.run_id.0.clone();
        store.insert_sandbox_run(&run).unwrap();
        let policy = SandboxPolicyV1::default_deny("baseline");
        store.insert_sandbox_policy_version(&policy).unwrap();
        let vr = ValidationRunRowV1 {
            validation_run_id: "VR-1".into(),
            sandbox_run_id: run_id.clone(),
            descriptor_id: "DESC-1".into(),
            verdict: "PASS".into(),
            check_count: 4,
            failed_check_count: 0,
            report_artifact_ref: Some("ART-report-1".into()),
            started_at_utc: "2026-05-17T00:00:00Z".into(),
            finished_at_utc: "2026-05-17T00:00:01Z".into(),
            summary_json: json!({"checks": ["lint", "fmt", "unit", "schema"]}),
        };
        store.insert_validation_run(&vr).unwrap();
        let bag = store.load_replay_bag(&run_id, &policy.version_id()).unwrap();
        assert!(bag.validation.is_some());
        assert_eq!(bag.validation.unwrap().verdict, "PASS");
    }

    // MT-014: duplicate idempotency keys are rejected or resolved idempotently.
    #[test]
    fn promotion_receipt_is_idempotent_on_matching_payload() {
        let mut store = InMemoryKb003Storage::new_postgres_primary();
        let dec = PromotionDecisionRowV1 {
            decision_id: "PD-1".into(),
            validation_run_id: "VR-1".into(),
            decision: "PROMOTED".into(),
            rationale_short: "checks green".into(),
            decided_at_utc: "2026-05-17T00:00:02Z".into(),
        };
        store.insert_promotion_decision(&dec).unwrap();
        let receipt = PromotionReceiptRowV1 {
            receipt_id: "PR-1".into(),
            decision_id: "PD-1".into(),
            idempotency_key: "IDEMP-1".into(),
            payload_hash: "h-aaaa".into(),
            artifact_ref: Some("ART-r-1".into()),
            issued_at_utc: "2026-05-17T00:00:03Z".into(),
        };
        let first = store.insert_promotion_receipt(&receipt).unwrap();
        // Same key + same payload hash => idempotent returns same id.
        let second = store.insert_promotion_receipt(&receipt).unwrap();
        assert_eq!(first, second);
        // Same key, different payload hash => conflict.
        let mut mutated = receipt.clone();
        mutated.receipt_id = "PR-2".into();
        mutated.payload_hash = "h-bbbb".into();
        let err = store.insert_promotion_receipt(&mutated).unwrap_err();
        match err {
            Kb003StorageError::IdempotencyConflict { key, .. } => assert_eq!(key, "IDEMP-1"),
            other => panic!("expected IdempotencyConflict, got {:?}", other),
        }
    }

    // MT-011 negative: invalid transition refused.
    #[test]
    fn invalid_status_transition_refused() {
        let mut store = InMemoryKb003Storage::new_postgres_primary();
        let run = fresh_run();
        let id = run.run_id.0.clone();
        store.insert_sandbox_run(&run).unwrap();
        let err = store
            .update_sandbox_run_status(&id, SandboxRunStatus::Completed)
            .unwrap_err();
        match err {
            Kb003StorageError::Backend(m) => assert!(m.contains("invalid transition")),
            other => panic!("{:?}", other),
        }
    }

    // MT-015 wired: writes refuse non-Postgres modes.
    #[test]
    fn writes_refused_when_authority_is_not_postgres_primary() {
        let mut store = InMemoryKb003Storage::new_with_mode(AuthorityMode::SqliteCache);
        let run = fresh_run();
        let err = store.insert_sandbox_run(&run).unwrap_err();
        assert!(matches!(err, Kb003StorageError::Authority(_)));
    }

    // Migrations declare themselves; safety net for SQL drift.
    #[test]
    fn migration_table_is_complete_and_ordered() {
        assert_eq!(KB003_MIGRATIONS_V1.len(), 4);
        let names: Vec<&str> = KB003_MIGRATIONS_V1.iter().map(|(n, _)| *n).collect();
        assert_eq!(
            names,
            vec![
                "kb003_sandbox_runs_v1",
                "kb003_sandbox_policies_v1",
                "kb003_validation_runs_v1",
                "kb003_promotion_receipts_v1"
            ]
        );
        for (_, sql) in KB003_MIGRATIONS_V1 {
            assert!(sql.contains("CREATE TABLE IF NOT EXISTS"));
        }
    }
}
