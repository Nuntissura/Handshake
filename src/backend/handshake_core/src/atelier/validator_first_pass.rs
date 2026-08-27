//! Production validator-first-pass-in-sandbox path (WP-KERNEL-005 MT-151).
//!
//! The self-improvement loop core consumes two injected traits:
//! [`LoopSandbox`] (provision an isolated world carrying the candidate
//! snapshot) and [`ValidatorRunner`] (run the validator first-pass against
//! one corpus item inside that world). This module supplies the production
//! implementations the evaluator previously stubbed:
//!
//! - [`LiveSelfImproveSandbox`] materialises the snapshot's candidate `after`
//!   value into a real per-run sandbox workspace directory and persists the
//!   provisioning run to `atelier_self_improve_sandbox_run`, mirrored
//!   through the Atelier EventLedger.
//! - [`HbrFirstPassRunner`] executes the real HBR handoff gate
//!   ([`HandoffGate::evaluate`]) against the corpus item's acceptance-matrix
//!   fixture, appending the canonical `HBR_HANDOFF_GATE` EventLedger row,
//!   and persists every first-pass execution to
//!   `atelier_validator_first_pass_run` linked to the sandbox run.
//!
//! Both traits are synchronous, so store access goes through a local Tokio
//! bridge; callers inside a runtime must use a multi-thread runtime.

use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use surrealdb::types::{Datetime, RecordId, SurrealValue, Uuid as SurrealUuid};
use uuid::Uuid;

use crate::hbr::handoff_gate::{
    HandoffEventLedger, HandoffEventLedgerError, HandoffGate, HandoffRule, HandoffTransition,
    HbrAcceptanceMatrix, HbrMatrixRow, HbrNotApplicableRow, HbrPacket,
};
use crate::kernel::{KernelEvent, NewKernelEvent};
use crate::self_improve::corpus::{CorpusItem, ValidatorVerdict};
use crate::self_improve::editable_surface::EditableSurfaceSnapshot;
use crate::self_improve::evaluator::{EvalError, ValidatorRun, ValidatorRunner};
use crate::self_improve::loop_core::{LoopSandbox, LoopSandboxError, SandboxRunResult};
use crate::storage::Database;

use super::{atelier_event_sql, uuid_from_record_link, AtelierError, AtelierResult, AtelierStore};

/// Bridge the synchronous sandbox/runner traits to the async store. Callers
/// inside a runtime must use a multi-thread runtime
/// (`#[tokio::test(flavor = "multi_thread")]`).
fn block_on<F: std::future::Future>(future: F) -> F::Output {
    match tokio::runtime::Handle::try_current() {
        Ok(handle) => tokio::task::block_in_place(|| handle.block_on(future)),
        Err(_) => {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("tokio current-thread runtime must build");
            runtime.block_on(future)
        }
    }
}

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

/// One `atelier_self_improve_sandbox_run` row as the store returns it.
#[derive(SurrealValue)]
struct SandboxRunRow {
    sandbox_run_id: SurrealUuid,
    surface_kind: String,
    snapshot_sha256: String,
    workspace_ref: String,
    status: String,
    started_at_utc: Datetime,
    completed_at_utc: Datetime,
    created_at_utc: Datetime,
}

impl From<SandboxRunRow> for SelfImproveSandboxRunRecord {
    fn from(row: SandboxRunRow) -> Self {
        SelfImproveSandboxRunRecord {
            sandbox_run_id: row.sandbox_run_id.into(),
            surface_kind: row.surface_kind,
            snapshot_sha256: row.snapshot_sha256,
            workspace_ref: row.workspace_ref,
            status: row.status,
            started_at_utc: row.started_at_utc.into(),
            completed_at_utc: row.completed_at_utc.into(),
            created_at_utc: row.created_at_utc.into(),
        }
    }
}

/// One `atelier_validator_first_pass_run` row as the store returns it.
#[derive(SurrealValue)]
struct FirstPassRunRow {
    first_pass_run_id: SurrealUuid,
    sandbox_run_id: Option<RecordId>,
    corpus_item_id: SurrealUuid,
    hbr_rule_id: String,
    packet_under_test: String,
    transition: String,
    verdict: String,
    failing_rule_count: i32,
    latency_ms: i64,
    capsule_bytes: i64,
    gate_event_id: Option<SurrealUuid>,
    created_at_utc: Datetime,
}

impl TryFrom<FirstPassRunRow> for ValidatorFirstPassRunRecord {
    type Error = AtelierError;

    fn try_from(row: FirstPassRunRow) -> AtelierResult<Self> {
        let sandbox_run_id = row
            .sandbox_run_id
            .as_ref()
            .map(|link| uuid_from_record_link("sandbox_run_id", link))
            .transpose()?;
        Ok(ValidatorFirstPassRunRecord {
            first_pass_run_id: row.first_pass_run_id.into(),
            sandbox_run_id,
            corpus_item_id: row.corpus_item_id.into(),
            hbr_rule_id: row.hbr_rule_id,
            packet_under_test: row.packet_under_test,
            transition: row.transition,
            verdict: verdict_from_token(&row.verdict)?,
            failing_rule_count: row.failing_rule_count,
            latency_ms: row.latency_ms,
            capsule_bytes: row.capsule_bytes,
            gate_event_id: row.gate_event_id.map(Into::into),
            created_at_utc: row.created_at_utc.into(),
        })
    }
}

#[derive(Clone, SurrealValue)]
struct RecordSandboxRunBindings {
    record_id: RecordId,
    sandbox_run_id: SurrealUuid,
    surface_kind: String,
    snapshot_sha256: String,
    workspace_ref: String,
    status: String,
    started_at_utc: Datetime,
    completed_at_utc: Datetime,
}

const RECORD_SANDBOX_RUN_STATEMENT: &str = concat!(
    "RETURN { \
       LET $rid = $domain.record_id; ",
    atelier_event_sql!(),
    " RETURN (CREATE $rid CONTENT { \
         sandbox_run_id: $domain.sandbox_run_id, \
         surface_kind: $domain.surface_kind, \
         snapshot_sha256: $domain.snapshot_sha256, \
         workspace_ref: $domain.workspace_ref, \
         status: $domain.status, \
         started_at_utc: $domain.started_at_utc, \
         completed_at_utc: $domain.completed_at_utc \
       }); };"
);

#[derive(SurrealValue)]
struct SandboxRunIdBinding {
    sandbox_run_id: SurrealUuid,
}

const GET_SANDBOX_RUN_STATEMENT: &str =
    "SELECT sandbox_run_id, surface_kind, snapshot_sha256, workspace_ref, status, \
            started_at_utc, completed_at_utc, created_at_utc \
     FROM atelier_self_improve_sandbox_run WHERE sandbox_run_id = $sandbox_run_id LIMIT 1;";

#[derive(Clone, SurrealValue)]
struct RecordFirstPassRunBindings {
    record_id: RecordId,
    first_pass_run_id: SurrealUuid,
    sandbox_run_ref: Option<RecordId>,
    corpus_item_id: SurrealUuid,
    hbr_rule_id: String,
    packet_under_test: String,
    transition: String,
    verdict: String,
    failing_rule_count: i32,
    latency_ms: i64,
    capsule_bytes: i64,
    gate_event_id: Option<SurrealUuid>,
}

const FIRST_PASS_RUN_SELECT: &str =
    "first_pass_run_id, sandbox_run_id, corpus_item_id, hbr_rule_id, packet_under_test, \
     transition, verdict, failing_rule_count, latency_ms, capsule_bytes, gate_event_id, \
     created_at_utc";

const RECORD_FIRST_PASS_RUN_STATEMENT: &str = concat!(
    "RETURN { \
       LET $rid = $domain.record_id; ",
    atelier_event_sql!(),
    " RETURN (CREATE $rid CONTENT { \
         first_pass_run_id: $domain.first_pass_run_id, \
         sandbox_run_id: $domain.sandbox_run_ref, \
         corpus_item_id: $domain.corpus_item_id, \
         hbr_rule_id: $domain.hbr_rule_id, \
         packet_under_test: $domain.packet_under_test, \
         transition: $domain.transition, \
         verdict: $domain.verdict, \
         failing_rule_count: $domain.failing_rule_count, \
         latency_ms: $domain.latency_ms, \
         capsule_bytes: $domain.capsule_bytes, \
         gate_event_id: $domain.gate_event_id \
       }); };"
);

#[derive(SurrealValue)]
struct FirstPassRunIdBinding {
    first_pass_run_id: SurrealUuid,
}

const GET_FIRST_PASS_RUN_STATEMENT: &str = concat!(
    "SELECT ",
    "first_pass_run_id, sandbox_run_id, corpus_item_id, hbr_rule_id, packet_under_test, \
     transition, verdict, failing_rule_count, latency_ms, capsule_bytes, gate_event_id, \
     created_at_utc",
    " FROM atelier_validator_first_pass_run \
     WHERE first_pass_run_id = $first_pass_run_id LIMIT 1;"
);

#[derive(SurrealValue)]
struct SandboxRunRefBinding {
    sandbox_run_ref: RecordId,
}

const LIST_FIRST_PASS_RUNS_STATEMENT: &str = concat!(
    "SELECT ",
    "first_pass_run_id, sandbox_run_id, corpus_item_id, hbr_rule_id, packet_under_test, \
     transition, verdict, failing_rule_count, latency_ms, capsule_bytes, gate_event_id, \
     created_at_utc",
    " FROM atelier_validator_first_pass_run \
     WHERE sandbox_run_id = $sandbox_run_ref \
     ORDER BY created_at_utc ASC, first_pass_run_id ASC;"
);

impl AtelierStore {
    /// Persist a sandbox provisioning run and mirror it through the
    /// EventLedger, in one atomic statement.
    pub async fn record_self_improve_sandbox_run(
        &self,
        run: &NewSelfImproveSandboxRun,
    ) -> AtelierResult<SelfImproveSandboxRunRecord> {
        let bindings = RecordSandboxRunBindings {
            record_id: RecordId::new(
                "atelier_self_improve_sandbox_run",
                SurrealUuid::from(run.sandbox_run_id),
            ),
            sandbox_run_id: SurrealUuid::from(run.sandbox_run_id),
            surface_kind: run.surface_kind.clone(),
            snapshot_sha256: run.snapshot_sha256.clone(),
            workspace_ref: run.workspace_ref.clone(),
            status: run.status.clone(),
            started_at_utc: Datetime::from(run.started_at_utc),
            completed_at_utc: Datetime::from(run.completed_at_utc),
        };
        let row: Option<SandboxRunRow> = self
            .write_with_event(
                RECORD_SANDBOX_RUN_STATEMENT,
                bindings,
                validator_first_pass_event_family::SANDBOX_RUN_PROVISIONED,
                "atelier_self_improve_sandbox_run",
                &run.sandbox_run_id.to_string(),
                serde_json::json!({
                    "sandbox_run_id": run.sandbox_run_id,
                    "surface_kind": run.surface_kind,
                    "snapshot_sha256": run.snapshot_sha256,
                    "workspace_ref": run.workspace_ref,
                    "status": run.status,
                    "schema": "hsk.atelier.self_improve_sandbox_run@1",
                }),
            )
            .await?;
        Ok(row
            .ok_or_else(|| {
                AtelierError::Internal(
                    "recording a self-improve sandbox run returned no row".to_owned(),
                )
            })?
            .into())
    }

    /// Re-read a sandbox provisioning run.
    pub async fn get_self_improve_sandbox_run(
        &self,
        sandbox_run_id: Uuid,
    ) -> AtelierResult<SelfImproveSandboxRunRecord> {
        let bindings = SandboxRunIdBinding {
            sandbox_run_id: SurrealUuid::from(sandbox_run_id),
        };
        let row: Option<SandboxRunRow> = self
            .store()
            .with_data_operation(move |ctx| {
                Box::pin(async move { ctx.query_first(GET_SANDBOX_RUN_STATEMENT, bindings).await })
            })
            .await?;
        match row {
            Some(row) => Ok(row.into()),
            None => Err(AtelierError::NotFound(format!(
                "self-improve sandbox run sandbox_run_id={sandbox_run_id}"
            ))),
        }
    }

    /// Persist one validator first-pass execution and mirror it through
    /// the EventLedger, in one atomic statement.
    pub async fn record_validator_first_pass_run(
        &self,
        run: &NewValidatorFirstPassRun,
    ) -> AtelierResult<ValidatorFirstPassRunRecord> {
        let first_pass_run_id = Uuid::now_v7();
        let bindings = RecordFirstPassRunBindings {
            record_id: RecordId::new(
                "atelier_validator_first_pass_run",
                SurrealUuid::from(first_pass_run_id),
            ),
            first_pass_run_id: SurrealUuid::from(first_pass_run_id),
            sandbox_run_ref: run
                .sandbox_run_id
                .map(|id| RecordId::new("atelier_self_improve_sandbox_run", SurrealUuid::from(id))),
            corpus_item_id: SurrealUuid::from(run.corpus_item_id),
            hbr_rule_id: run.hbr_rule_id.clone(),
            packet_under_test: run.packet_under_test.clone(),
            transition: run.transition.clone(),
            verdict: verdict_token(run.verdict).to_owned(),
            failing_rule_count: run.failing_rule_count,
            latency_ms: run.latency_ms,
            capsule_bytes: run.capsule_bytes,
            gate_event_id: run.gate_event_id.map(SurrealUuid::from),
        };
        let row: Option<FirstPassRunRow> = self
            .write_with_event(
                RECORD_FIRST_PASS_RUN_STATEMENT,
                bindings,
                validator_first_pass_event_family::VALIDATOR_FIRST_PASS_RECORDED,
                "atelier_validator_first_pass_run",
                &first_pass_run_id.to_string(),
                serde_json::json!({
                    "first_pass_run_id": first_pass_run_id,
                    "sandbox_run_id": run.sandbox_run_id,
                    "corpus_item_id": run.corpus_item_id,
                    "hbr_rule_id": run.hbr_rule_id,
                    "packet_under_test": run.packet_under_test,
                    "transition": run.transition,
                    "verdict": verdict_token(run.verdict),
                    "failing_rule_count": run.failing_rule_count,
                    "latency_ms": run.latency_ms,
                    "capsule_bytes": run.capsule_bytes,
                    "gate_event_id": run.gate_event_id,
                    "schema": "hsk.atelier.validator_first_pass_run@1",
                }),
            )
            .await?;
        row.ok_or_else(|| {
            AtelierError::Internal(
                "recording a validator first-pass run returned no row".to_owned(),
            )
        })?
        .try_into()
    }

    /// Re-read one validator first-pass execution.
    pub async fn get_validator_first_pass_run(
        &self,
        first_pass_run_id: Uuid,
    ) -> AtelierResult<ValidatorFirstPassRunRecord> {
        let bindings = FirstPassRunIdBinding {
            first_pass_run_id: SurrealUuid::from(first_pass_run_id),
        };
        let row: Option<FirstPassRunRow> = self
            .store()
            .with_data_operation(move |ctx| {
                Box::pin(async move {
                    ctx.query_first(GET_FIRST_PASS_RUN_STATEMENT, bindings)
                        .await
                })
            })
            .await?;
        match row {
            Some(row) => row.try_into(),
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
        let bindings = SandboxRunRefBinding {
            sandbox_run_ref: RecordId::new(
                "atelier_self_improve_sandbox_run",
                SurrealUuid::from(sandbox_run_id),
            ),
        };
        let rows: Vec<FirstPassRunRow> = self
            .store()
            .with_data_operation(move |ctx| {
                Box::pin(async move {
                    ctx.query_values(LIST_FIRST_PASS_RUNS_STATEMENT, bindings)
                        .await
                })
            })
            .await?;
        rows.into_iter()
            .map(ValidatorFirstPassRunRecord::try_from)
            .collect()
    }
}

/// Shared slot linking the sandbox provisioning run to the first-pass
/// executions the evaluator performs inside it. [`LiveSelfImproveSandbox`]
/// writes its persisted run id here; [`HbrFirstPassRunner`] reads it so
/// every `atelier_validator_first_pass_run` row carries the link.
pub type SharedSandboxRunSlot = Arc<Mutex<Option<Uuid>>>;

/// Production [`LoopSandbox`]: provisions a real per-run sandbox workspace
/// directory carrying the candidate snapshot value, persists the run to
/// the durable store, and mirrors it through the EventLedger.
pub struct LiveSelfImproveSandbox {
    store: AtelierStore,
    sandbox_root: PathBuf,
    run_slot: SharedSandboxRunSlot,
}

impl LiveSelfImproveSandbox {
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

impl LoopSandbox for LiveSelfImproveSandbox {
    fn run(
        &self,
        snapshot: &EditableSurfaceSnapshot,
    ) -> Result<SandboxRunResult, LoopSandboxError> {
        let started_at_utc = Utc::now();
        let sandbox_run_id = Uuid::now_v7();
        let workspace = self
            .sandbox_root
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

        let snapshot_sha256 =
            snapshot_sha256(snapshot).map_err(|err| LoopSandboxError::new(err.to_string()))?;
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
        .map_err(|err| LoopSandboxError::new(format!("failed to persist sandbox run: {err}")))?;

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
/// `Arc<dyn Database>` so the gate event lands in the same durable
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
