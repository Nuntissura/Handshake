//! Production validator-first-pass-in-sandbox path (WP-KERNEL-005 MT-151).
//!
//! The self-improvement loop core consumes two injected traits:
//! [`LoopSandbox`] (provision an isolated world carrying the candidate
//! snapshot) and [`ValidatorRunner`] (run the validator first-pass against
//! one corpus item inside that world). This module supplies the production
//! implementations the evaluator previously stubbed:
//!
//! - [`PgSelfImproveSandbox`] materialises the snapshot's candidate `after`
//!   value into a real per-run sandbox workspace directory and persists the
//!   provisioning run to `atelier_self_improve_sandbox_run` (migration
//!   0118), mirrored through the Atelier EventLedger.
//! - [`HbrFirstPassRunner`] executes the real HBR handoff gate
//!   ([`HandoffGate::evaluate`]) against the corpus item's acceptance-matrix
//!   fixture, appending the canonical `HBR_HANDOFF_GATE` EventLedger row,
//!   and persists every first-pass execution to
//!   `atelier_validator_first_pass_run` linked to the sandbox run.
//!
//! Both traits are synchronous, so PG access goes through the shared Tokio
//! bridge (`memory::persistence_postgres::block_on`); callers inside a
//! runtime must use a multi-thread runtime.

use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::Row;
use uuid::Uuid;

use crate::hbr::handoff_gate::{
    HandoffEventLedger, HandoffEventLedgerError, HandoffGate, HandoffRule, HandoffTransition,
    HbrAcceptanceMatrix, HbrMatrixRow, HbrNotApplicableRow, HbrPacket,
};
use crate::kernel::{KernelEvent, NewKernelEvent};
use crate::memory::persistence_postgres::block_on;
use crate::self_improve::corpus::{CorpusItem, ValidatorVerdict};
use crate::self_improve::editable_surface::EditableSurfaceSnapshot;
use crate::self_improve::evaluator::{EvalError, ValidatorRun, ValidatorRunner};
use crate::self_improve::loop_core::{LoopSandbox, LoopSandboxError, SandboxRunResult};
use crate::storage::Database;

use super::{AtelierError, AtelierResult, AtelierStore};

pub mod validator_first_pass_event_family {
    pub const SANDBOX_RUN_PROVISIONED: &str = "atelier.self_improve.sandbox_run_provisioned";
    pub const VALIDATOR_FIRST_PASS_RECORDED: &str =
        "atelier.self_improve.validator_first_pass_recorded";

    pub const ALL: &[&str] = &[SANDBOX_RUN_PROVISIONED, VALIDATOR_FIRST_PASS_RECORDED];
}

/// Sandbox run statuses persisted to `atelier_self_improve_sandbox_run`.
pub const SANDBOX_RUN_STATUS_PROVISIONED: &str = "provisioned";
pub const SANDBOX_RUN_STATUS_FAILED: &str = "failed";

fn verdict_token(verdict: ValidatorVerdict) -> &'static str {
    match verdict {
        ValidatorVerdict::Pass => "pass",
        ValidatorVerdict::Fail => "fail",
        ValidatorVerdict::Skip => "skip",
    }
}

fn verdict_from_token(token: &str) -> AtelierResult<ValidatorVerdict> {
    match token {
        "pass" => Ok(ValidatorVerdict::Pass),
        "fail" => Ok(ValidatorVerdict::Fail),
        "skip" => Ok(ValidatorVerdict::Skip),
        other => Err(AtelierError::Validation(format!(
            "unknown validator first-pass verdict: {other}"
        ))),
    }
}

/// Surface kind token persisted for a sandbox run.
pub fn snapshot_surface_kind(snapshot: &EditableSurfaceSnapshot) -> &'static str {
    match snapshot {
        EditableSurfaceSnapshot::ModelManual { .. } => "model_manual",
        EditableSurfaceSnapshot::RetrievalPolicy { .. } => "retrieval_policy",
    }
}

/// `sha256:<hex>` digest over the canonical snapshot JSON (the same shape
/// the evaluator hashes into `EvalResult.snapshot_hash`).
pub fn snapshot_sha256(snapshot: &EditableSurfaceSnapshot) -> AtelierResult<String> {
    use sha2::{Digest, Sha256};
    let bytes = serde_json::to_vec(snapshot)
        .map_err(|err| AtelierError::Validation(format!("snapshot not serializable: {err}")))?;
    Ok(format!("sha256:{}", hex::encode(Sha256::digest(&bytes))))
}

/// New sandbox provisioning run to persist.
#[derive(Clone, Debug)]
pub struct NewSelfImproveSandboxRun {
    pub sandbox_run_id: Uuid,
    pub surface_kind: String,
    pub snapshot_sha256: String,
    pub workspace_ref: String,
    pub status: String,
    pub started_at_utc: DateTime<Utc>,
    pub completed_at_utc: DateTime<Utc>,
}

/// Persisted sandbox provisioning run row.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SelfImproveSandboxRunRecord {
    pub sandbox_run_id: Uuid,
    pub surface_kind: String,
    pub snapshot_sha256: String,
    pub workspace_ref: String,
    pub status: String,
    pub started_at_utc: DateTime<Utc>,
    pub completed_at_utc: DateTime<Utc>,
    pub created_at_utc: DateTime<Utc>,
}

/// New validator first-pass execution to persist.
#[derive(Clone, Debug)]
pub struct NewValidatorFirstPassRun {
    pub sandbox_run_id: Option<Uuid>,
    pub corpus_item_id: Uuid,
    pub hbr_rule_id: String,
    pub packet_under_test: String,
    pub transition: String,
    pub verdict: ValidatorVerdict,
    pub failing_rule_count: i32,
    pub latency_ms: i64,
    pub capsule_bytes: i64,
    pub gate_event_id: Option<Uuid>,
}

/// Persisted validator first-pass execution row.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValidatorFirstPassRunRecord {
    pub first_pass_run_id: Uuid,
    pub sandbox_run_id: Option<Uuid>,
    pub corpus_item_id: Uuid,
    pub hbr_rule_id: String,
    pub packet_under_test: String,
    pub transition: String,
    pub verdict: ValidatorVerdict,
    pub failing_rule_count: i32,
    pub latency_ms: i64,
    pub capsule_bytes: i64,
    pub gate_event_id: Option<Uuid>,
    pub created_at_utc: DateTime<Utc>,
}

impl AtelierStore {
    /// Persist a sandbox provisioning run and mirror it through the
    /// EventLedger.
    pub async fn record_self_improve_sandbox_run(
        &self,
        run: &NewSelfImproveSandboxRun,
    ) -> AtelierResult<SelfImproveSandboxRunRecord> {
        let mut tx = self.pool().begin().await?;
        let row = sqlx::query(
            r#"INSERT INTO atelier_self_improve_sandbox_run (
                   sandbox_run_id, surface_kind, snapshot_sha256,
                   workspace_ref, status, started_at_utc, completed_at_utc
               )
               VALUES ($1, $2, $3, $4, $5, $6, $7)
               RETURNING sandbox_run_id, surface_kind, snapshot_sha256,
                         workspace_ref, status, started_at_utc,
                         completed_at_utc, created_at_utc"#,
        )
        .bind(run.sandbox_run_id)
        .bind(&run.surface_kind)
        .bind(&run.snapshot_sha256)
        .bind(&run.workspace_ref)
        .bind(&run.status)
        .bind(run.started_at_utc)
        .bind(run.completed_at_utc)
        .fetch_one(&mut *tx)
        .await?;
        let record = sandbox_run_from_row(&row);

        self.record_event_in_tx(
            &mut tx,
            validator_first_pass_event_family::SANDBOX_RUN_PROVISIONED,
            "atelier_self_improve_sandbox_run",
            &record.sandbox_run_id.to_string(),
            serde_json::json!({
                "sandbox_run_id": record.sandbox_run_id,
                "surface_kind": record.surface_kind,
                "snapshot_sha256": record.snapshot_sha256,
                "workspace_ref": record.workspace_ref,
                "status": record.status,
                "schema": "hsk.atelier.self_improve_sandbox_run@1",
            }),
        )
        .await?;
        tx.commit().await?;
        Ok(record)
    }

    /// Re-read a sandbox provisioning run.
    pub async fn get_self_improve_sandbox_run(
        &self,
        sandbox_run_id: Uuid,
    ) -> AtelierResult<SelfImproveSandboxRunRecord> {
        let row = sqlx::query(
            r#"SELECT sandbox_run_id, surface_kind, snapshot_sha256,
                      workspace_ref, status, started_at_utc,
                      completed_at_utc, created_at_utc
               FROM atelier_self_improve_sandbox_run
               WHERE sandbox_run_id = $1"#,
        )
        .bind(sandbox_run_id)
        .fetch_optional(self.pool())
        .await?;
        match row {
            Some(row) => Ok(sandbox_run_from_row(&row)),
            None => Err(AtelierError::NotFound(format!(
                "self-improve sandbox run sandbox_run_id={sandbox_run_id}"
            ))),
        }
    }

    /// Persist one validator first-pass execution and mirror it through
    /// the EventLedger.
    pub async fn record_validator_first_pass_run(
        &self,
        run: &NewValidatorFirstPassRun,
    ) -> AtelierResult<ValidatorFirstPassRunRecord> {
        let first_pass_run_id = Uuid::now_v7();
        let mut tx = self.pool().begin().await?;
        let row = sqlx::query(
            r#"INSERT INTO atelier_validator_first_pass_run (
                   first_pass_run_id, sandbox_run_id, corpus_item_id,
                   hbr_rule_id, packet_under_test, transition, verdict,
                   failing_rule_count, latency_ms, capsule_bytes,
                   gate_event_id
               )
               VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
               RETURNING first_pass_run_id, sandbox_run_id, corpus_item_id,
                         hbr_rule_id, packet_under_test, transition,
                         verdict, failing_rule_count, latency_ms,
                         capsule_bytes, gate_event_id, created_at_utc"#,
        )
        .bind(first_pass_run_id)
        .bind(run.sandbox_run_id)
        .bind(run.corpus_item_id)
        .bind(&run.hbr_rule_id)
        .bind(&run.packet_under_test)
        .bind(&run.transition)
        .bind(verdict_token(run.verdict))
        .bind(run.failing_rule_count)
        .bind(run.latency_ms)
        .bind(run.capsule_bytes)
        .bind(run.gate_event_id)
        .fetch_one(&mut *tx)
        .await?;
        let record = first_pass_run_from_row(&row)?;

        self.record_event_in_tx(
            &mut tx,
            validator_first_pass_event_family::VALIDATOR_FIRST_PASS_RECORDED,
            "atelier_validator_first_pass_run",
            &record.first_pass_run_id.to_string(),
            serde_json::json!({
                "first_pass_run_id": record.first_pass_run_id,
                "sandbox_run_id": record.sandbox_run_id,
                "corpus_item_id": record.corpus_item_id,
                "hbr_rule_id": record.hbr_rule_id,
                "packet_under_test": record.packet_under_test,
                "transition": record.transition,
                "verdict": verdict_token(record.verdict),
                "failing_rule_count": record.failing_rule_count,
                "latency_ms": record.latency_ms,
                "capsule_bytes": record.capsule_bytes,
                "gate_event_id": record.gate_event_id,
                "schema": "hsk.atelier.validator_first_pass_run@1",
            }),
        )
        .await?;
        tx.commit().await?;
        Ok(record)
    }

    /// Re-read one validator first-pass execution.
    pub async fn get_validator_first_pass_run(
        &self,
        first_pass_run_id: Uuid,
    ) -> AtelierResult<ValidatorFirstPassRunRecord> {
        let row = sqlx::query(
            r#"SELECT first_pass_run_id, sandbox_run_id, corpus_item_id,
                      hbr_rule_id, packet_under_test, transition, verdict,
                      failing_rule_count, latency_ms, capsule_bytes,
                      gate_event_id, created_at_utc
               FROM atelier_validator_first_pass_run
               WHERE first_pass_run_id = $1"#,
        )
        .bind(first_pass_run_id)
        .fetch_optional(self.pool())
        .await?;
        match row {
            Some(row) => first_pass_run_from_row(&row),
            None => Err(AtelierError::NotFound(format!(
                "validator first-pass run first_pass_run_id={first_pass_run_id}"
            ))),
        }
    }

    /// All first-pass executions linked to one sandbox run, oldest first.
    pub async fn list_validator_first_pass_runs_for_sandbox(
        &self,
        sandbox_run_id: Uuid,
    ) -> AtelierResult<Vec<ValidatorFirstPassRunRecord>> {
        let rows = sqlx::query(
            r#"SELECT first_pass_run_id, sandbox_run_id, corpus_item_id,
                      hbr_rule_id, packet_under_test, transition, verdict,
                      failing_rule_count, latency_ms, capsule_bytes,
                      gate_event_id, created_at_utc
               FROM atelier_validator_first_pass_run
               WHERE sandbox_run_id = $1
               ORDER BY created_at_utc ASC, first_pass_run_id ASC"#,
        )
        .bind(sandbox_run_id)
        .fetch_all(self.pool())
        .await?;
        rows.iter().map(first_pass_run_from_row).collect()
    }
}

fn sandbox_run_from_row(row: &sqlx::postgres::PgRow) -> SelfImproveSandboxRunRecord {
    SelfImproveSandboxRunRecord {
        sandbox_run_id: row.get("sandbox_run_id"),
        surface_kind: row.get("surface_kind"),
        snapshot_sha256: row.get("snapshot_sha256"),
        workspace_ref: row.get("workspace_ref"),
        status: row.get("status"),
        started_at_utc: row.get("started_at_utc"),
        completed_at_utc: row.get("completed_at_utc"),
        created_at_utc: row.get("created_at_utc"),
    }
}

fn first_pass_run_from_row(
    row: &sqlx::postgres::PgRow,
) -> AtelierResult<ValidatorFirstPassRunRecord> {
    let verdict: String = row.get("verdict");
    Ok(ValidatorFirstPassRunRecord {
        first_pass_run_id: row.get("first_pass_run_id"),
        sandbox_run_id: row.get("sandbox_run_id"),
        corpus_item_id: row.get("corpus_item_id"),
        hbr_rule_id: row.get("hbr_rule_id"),
        packet_under_test: row.get("packet_under_test"),
        transition: row.get("transition"),
        verdict: verdict_from_token(&verdict)?,
        failing_rule_count: row.get("failing_rule_count"),
        latency_ms: row.get("latency_ms"),
        capsule_bytes: row.get("capsule_bytes"),
        gate_event_id: row.get("gate_event_id"),
        created_at_utc: row.get("created_at_utc"),
    })
}

/// Shared slot linking the sandbox provisioning run to the first-pass
/// executions the evaluator performs inside it. [`PgSelfImproveSandbox`]
/// writes its persisted run id here; [`HbrFirstPassRunner`] reads it so
/// every `atelier_validator_first_pass_run` row carries the FK.
pub type SharedSandboxRunSlot = Arc<Mutex<Option<Uuid>>>;

/// Production [`LoopSandbox`]: provisions a real per-run sandbox workspace
/// directory carrying the candidate snapshot value, persists the run to
/// PostgreSQL, and mirrors it through the EventLedger.
pub struct PgSelfImproveSandbox {
    store: AtelierStore,
    sandbox_root: PathBuf,
    run_slot: SharedSandboxRunSlot,
}

impl PgSelfImproveSandbox {
    pub fn new(store: AtelierStore, sandbox_root: PathBuf) -> Self {
        Self {
            store,
            sandbox_root,
            run_slot: Arc::new(Mutex::new(None)),
        }
    }

    /// The slot a paired [`HbrFirstPassRunner`] uses to link its rows to
    /// the most recent sandbox run.
    pub fn run_slot(&self) -> SharedSandboxRunSlot {
        Arc::clone(&self.run_slot)
    }
}

impl LoopSandbox for PgSelfImproveSandbox {
    fn run(
        &self,
        snapshot: &EditableSurfaceSnapshot,
    ) -> Result<SandboxRunResult, LoopSandboxError> {
        let started_at_utc = Utc::now();
        let sandbox_run_id = Uuid::now_v7();
        let workspace =
            self.sandbox_root
                .join(format!("self-improve-sandbox-{sandbox_run_id}"));
        std::fs::create_dir_all(&workspace).map_err(|err| {
            LoopSandboxError::new(format!(
                "failed to provision sandbox workspace {}: {err}",
                workspace.display()
            ))
        })?;

        // Materialise the candidate `after` value into the isolated world
        // so the validator first-pass runs against the proposal, never the
        // live authority surface.
        match snapshot {
            EditableSurfaceSnapshot::ModelManual {
                manual_section_id,
                after_text,
                ..
            } => {
                let candidate = workspace.join("model_manual_section.txt");
                let body = format!("{manual_section_id}\n---\n{after_text}");
                std::fs::write(&candidate, body).map_err(|err| {
                    LoopSandboxError::new(format!(
                        "failed to materialise candidate manual section into {}: {err}",
                        candidate.display()
                    ))
                })?;
            }
            EditableSurfaceSnapshot::RetrievalPolicy {
                task_type,
                parameter,
                after_value,
                ..
            } => {
                let candidate = workspace.join("retrieval_policy.json");
                let body = serde_json::json!({
                    "task_type": task_type,
                    "parameter": parameter,
                    "candidate_value": after_value,
                });
                std::fs::write(&candidate, body.to_string()).map_err(|err| {
                    LoopSandboxError::new(format!(
                        "failed to materialise candidate retrieval policy into {}: {err}",
                        candidate.display()
                    ))
                })?;
            }
        }

        let snapshot_sha256 = snapshot_sha256(snapshot)
            .map_err(|err| LoopSandboxError::new(err.to_string()))?;
        let record = block_on(self.store.record_self_improve_sandbox_run(
            &NewSelfImproveSandboxRun {
                sandbox_run_id,
                surface_kind: snapshot_surface_kind(snapshot).to_string(),
                snapshot_sha256,
                workspace_ref: workspace.display().to_string(),
                status: SANDBOX_RUN_STATUS_PROVISIONED.to_string(),
                started_at_utc,
                completed_at_utc: Utc::now(),
            },
        ))
        .map_err(|err| {
            LoopSandboxError::new(format!("failed to persist sandbox run: {err}"))
        })?;

        *self
            .run_slot
            .lock()
            .expect("sandbox run slot lock poisoned") = Some(record.sandbox_run_id);
        Ok(SandboxRunResult {
            sandbox_run_id: record.sandbox_run_id,
        })
    }
}

/// Acceptance-matrix fixture shape carried in [`CorpusItem::fixtures`] for
/// the validator first-pass. Mirrors the HBR handoff-gate inputs.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FirstPassFixture {
    pub transition: String,
    pub rules: Vec<FirstPassFixtureRule>,
    pub acceptance_matrix: FirstPassFixtureMatrix,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FirstPassFixtureRule {
    pub hbr_id: String,
    pub evidence_kind: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FirstPassFixtureMatrix {
    #[serde(default)]
    pub hbr: Vec<FirstPassFixtureMatrixRow>,
    #[serde(default)]
    pub hbr_not_applicable: Vec<FirstPassFixtureNaRow>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FirstPassFixtureMatrixRow {
    pub hbr_id: String,
    pub status: String,
    #[serde(default)]
    pub evidence_pointer: Option<String>,
    #[serde(default)]
    pub validator_verdict: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FirstPassFixtureNaRow {
    pub hbr_id: String,
    pub reason: String,
}

fn parse_transition(token: &str) -> Result<HandoffTransition, EvalError> {
    match token {
        "RefinementToCoder" => Ok(HandoffTransition::RefinementToCoder),
        "CoderToWpValidator" => Ok(HandoffTransition::CoderToWpValidator),
        "WpValidatorToIntegrationValidator" => {
            Ok(HandoffTransition::WpValidatorToIntegrationValidator)
        }
        "IntegrationValidatorToOrchestrator" => {
            Ok(HandoffTransition::IntegrationValidatorToOrchestrator)
        }
        other => Err(EvalError::ValidatorRunner {
            message: format!("unknown handoff transition in fixture: {other}"),
        }),
    }
}

/// EventLedger handle the handoff gate appends through. Wraps the shared
/// `Arc<dyn Database>` so the gate event lands in the same PostgreSQL
/// kernel EventLedger every other Atelier proof reads.
struct ArcDatabaseLedger(Arc<dyn Database>);

#[async_trait]
impl HandoffEventLedger for ArcDatabaseLedger {
    async fn append_handoff_event(
        &self,
        event: NewKernelEvent,
    ) -> Result<KernelEvent, HandoffEventLedgerError> {
        self.0
            .append_kernel_event(event)
            .await
            .map_err(|error| HandoffEventLedgerError::new(error.to_string()))
    }
}

/// Production [`ValidatorRunner`]: runs the real HBR handoff gate as the
/// validator first-pass for one corpus item, persists the execution row,
/// and returns the measured run.
pub struct HbrFirstPassRunner {
    store: AtelierStore,
    ledger: Arc<dyn Database>,
    sandbox_run: SharedSandboxRunSlot,
}

impl HbrFirstPassRunner {
    pub fn new(
        store: AtelierStore,
        ledger: Arc<dyn Database>,
        sandbox_run: SharedSandboxRunSlot,
    ) -> Self {
        Self {
            store,
            ledger,
            sandbox_run,
        }
    }
}

impl ValidatorRunner for HbrFirstPassRunner {
    fn run(
        &self,
        item: &CorpusItem,
        snapshot: &EditableSurfaceSnapshot,
    ) -> Result<ValidatorRun, EvalError> {
        let fixture: FirstPassFixture =
            serde_json::from_value(item.fixtures.clone()).map_err(|err| {
                EvalError::ValidatorRunner {
                    message: format!(
                        "corpus item {} carries no first-pass fixture: {err}",
                        item.id
                    ),
                }
            })?;
        let transition = parse_transition(&fixture.transition)?;
        let rules: Vec<HandoffRule> = fixture
            .rules
            .iter()
            .map(|rule| HandoffRule::new(rule.hbr_id.clone(), rule.evidence_kind.clone()))
            .collect();
        let packet = HbrPacket {
            wp_id: item.packet_under_test.clone(),
            acceptance_matrix: HbrAcceptanceMatrix {
                hbr: fixture
                    .acceptance_matrix
                    .hbr
                    .iter()
                    .map(|row| HbrMatrixRow {
                        hbr_id: row.hbr_id.clone(),
                        status: row.status.clone(),
                        evidence_pointer: row.evidence_pointer.clone(),
                        validator_verdict: row.validator_verdict.clone(),
                    })
                    .collect(),
                hbr_not_applicable: fixture
                    .acceptance_matrix
                    .hbr_not_applicable
                    .iter()
                    .map(|row| HbrNotApplicableRow {
                        hbr_id: row.hbr_id.clone(),
                        reason: row.reason.clone(),
                    })
                    .collect(),
            },
        };

        let gate = HandoffGate::new(ArcDatabaseLedger(Arc::clone(&self.ledger)), rules);
        let started = Instant::now();
        let outcome = block_on(gate.evaluate(&packet, transition));
        let latency_ms = u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX);

        let (verdict, failing_rule_count, gate_uuid) = match &outcome {
            Ok(evidence) => {
                let verdict = if evidence.evaluated_rules.is_empty() {
                    ValidatorVerdict::Skip
                } else {
                    ValidatorVerdict::Pass
                };
                (verdict, 0_i32, Some(evidence.gate_uuid))
            }
            Err(block) => (
                ValidatorVerdict::Fail,
                i32::try_from(block.failing_rules.len()).unwrap_or(i32::MAX),
                Some(block.gate_uuid),
            ),
        };

        let capsule_bytes = serde_json::to_vec(snapshot)
            .map(|bytes| bytes.len() as i64)
            .unwrap_or(0);
        let sandbox_run_id = *self
            .sandbox_run
            .lock()
            .expect("sandbox run slot lock poisoned");

        block_on(
            self.store
                .record_validator_first_pass_run(&NewValidatorFirstPassRun {
                    sandbox_run_id,
                    corpus_item_id: item.id,
                    hbr_rule_id: item.hbr_rule_id.clone(),
                    packet_under_test: item.packet_under_test.clone(),
                    transition: transition.as_str().to_string(),
                    verdict,
                    failing_rule_count,
                    latency_ms: i64::try_from(latency_ms).unwrap_or(i64::MAX),
                    capsule_bytes,
                    gate_event_id: gate_uuid,
                }),
        )
        .map_err(|err| EvalError::ValidatorRunner {
            message: format!("failed to persist validator first-pass run: {err}"),
        })?;

        Ok(ValidatorRun {
            verdict,
            latency_ms,
            capsule_bytes: u64::try_from(capsule_bytes).unwrap_or(0),
        })
    }
}
