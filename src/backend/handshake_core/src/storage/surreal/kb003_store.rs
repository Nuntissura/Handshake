use std::future::Future;

use chrono::{DateTime, SecondsFormat, Utc};
use surrealdb::types::{Datetime, RecordId, RecordIdKey, SurrealValue};

use super::SurrealStorage;
use crate::kernel::sandbox::cancellation::TerminalCause;
use crate::kernel::sandbox::no_sqlite_tripwire::AuthorityMode;
use crate::kernel::sandbox::policy::{CapabilityDecision, SandboxCapability, SandboxPolicyV1};
use crate::kernel::sandbox::run::{SandboxRunId, SandboxRunStatus, SandboxRunV1};
use crate::storage::kb003_storage::{
    Kb003Storage, Kb003StorageError, Kb003StorageResult, PromotionDecisionRowV1,
    PromotionReceiptRowV1, ReplayDurableBag, ValidationRunRowV1,
};

const POLICIES: &str = "kb003_sandbox_policies";
const RUNS: &str = "kb003_sandbox_runs";
const VALIDATIONS: &str = "kb003_validation_runs";
const DECISIONS: &str = "kb003_promotion_decisions";
const RECEIPTS: &str = "kb003_promotion_receipts";

#[derive(Clone)]
pub struct SurrealKb003Storage {
    storage: SurrealStorage,
}

impl SurrealKb003Storage {
    pub fn new(storage: SurrealStorage) -> Self {
        Self { storage }
    }

    pub fn embedded_store(&self) -> &SurrealStorage {
        &self.storage
    }

    pub async fn insert_sandbox_run_async(&self, run: SandboxRunV1) -> Kb003StorageResult<()> {
        insert_sandbox_run(&self.storage, run).await
    }

    pub async fn update_sandbox_run_status_async(
        &self,
        run_id: String,
        status: SandboxRunStatus,
    ) -> Kb003StorageResult<()> {
        update_sandbox_run_status(&self.storage, run_id, status).await
    }

    pub async fn insert_sandbox_policy_version_async(
        &self,
        policy: SandboxPolicyV1,
    ) -> Kb003StorageResult<()> {
        insert_sandbox_policy_version(&self.storage, policy).await
    }

    pub async fn insert_validation_run_async(
        &self,
        row: ValidationRunRowV1,
    ) -> Kb003StorageResult<()> {
        insert_validation_run(&self.storage, row).await
    }

    pub async fn insert_promotion_decision_async(
        &self,
        row: PromotionDecisionRowV1,
    ) -> Kb003StorageResult<()> {
        insert_promotion_decision(&self.storage, row).await
    }

    pub async fn insert_promotion_receipt_async(
        &self,
        row: PromotionReceiptRowV1,
    ) -> Kb003StorageResult<String> {
        insert_promotion_receipt(&self.storage, row).await
    }

    pub async fn load_run_for_replay_async(
        &self,
        run_id: String,
        policy_version_id: String,
    ) -> Kb003StorageResult<ReplayDurableBag> {
        load_run_for_replay(&self.storage, run_id, policy_version_id).await
    }

    fn block_on<T, F>(&self, future: F) -> Kb003StorageResult<T>
    where
        T: Send + 'static,
        F: Future<Output = Kb003StorageResult<T>> + Send + 'static,
    {
        match tokio::runtime::Handle::try_current() {
            Ok(handle) if handle.runtime_flavor() == tokio::runtime::RuntimeFlavor::MultiThread => {
                tokio::task::block_in_place(|| handle.block_on(future))
            }
            Ok(_) => std::thread::Builder::new()
                .name("handshake-kb003-surreal".to_owned())
                .spawn(move || {
                    tokio::runtime::Builder::new_current_thread()
                        .enable_all()
                        .build()
                        .map_err(|error| Kb003StorageError::Backend(error.to_string()))?
                        .block_on(future)
                })
                .map_err(|error| Kb003StorageError::Backend(error.to_string()))?
                .join()
                .map_err(|_| {
                    Kb003StorageError::Backend("KB003 embedded storage worker panicked".to_owned())
                })?,
            Err(_) => tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .map_err(|error| Kb003StorageError::Backend(error.to_string()))?
                .block_on(future),
        }
    }
}

impl Kb003Storage for SurrealKb003Storage {
    fn authority_mode(&self) -> AuthorityMode {
        AuthorityMode::SurrealPrimary
    }

    fn do_insert_sandbox_run(&mut self, run: &SandboxRunV1) -> Kb003StorageResult<()> {
        let adapter = self.clone();
        let run = run.clone();
        self.block_on(async move { adapter.insert_sandbox_run_async(run).await })
    }

    fn do_update_sandbox_run_status(
        &mut self,
        run_id: &str,
        new_status: SandboxRunStatus,
    ) -> Kb003StorageResult<()> {
        let adapter = self.clone();
        let run_id = run_id.to_owned();
        self.block_on(async move {
            adapter
                .update_sandbox_run_status_async(run_id, new_status)
                .await
        })
    }

    fn do_insert_sandbox_policy_version(
        &mut self,
        policy: &SandboxPolicyV1,
    ) -> Kb003StorageResult<()> {
        let adapter = self.clone();
        let policy = policy.clone();
        self.block_on(async move { adapter.insert_sandbox_policy_version_async(policy).await })
    }

    fn do_insert_validation_run(&mut self, row: &ValidationRunRowV1) -> Kb003StorageResult<()> {
        let adapter = self.clone();
        let row = row.clone();
        self.block_on(async move { adapter.insert_validation_run_async(row).await })
    }

    fn do_insert_promotion_decision(
        &mut self,
        row: &PromotionDecisionRowV1,
    ) -> Kb003StorageResult<()> {
        let adapter = self.clone();
        let row = row.clone();
        self.block_on(async move { adapter.insert_promotion_decision_async(row).await })
    }

    fn do_insert_promotion_receipt(
        &mut self,
        row: &PromotionReceiptRowV1,
    ) -> Kb003StorageResult<String> {
        let adapter = self.clone();
        let row = row.clone();
        self.block_on(async move { adapter.insert_promotion_receipt_async(row).await })
    }

    fn load_run_for_replay(
        &self,
        run_id: &str,
        policy_version_id: &str,
    ) -> Kb003StorageResult<ReplayDurableBag> {
        let adapter = self.clone();
        let run_id = run_id.to_owned();
        let policy_version_id = policy_version_id.to_owned();
        self.block_on(async move {
            adapter
                .load_run_for_replay_async(run_id, policy_version_id)
                .await
        })
    }
}

#[derive(SurrealValue)]
struct PolicyWrite {
    record: RecordId,
    policy_version_id: String,
    policy_id: String,
    policy_version: i64,
    name: String,
    created_at_utc: Datetime,
    default_decision_json: String,
    overrides_json: String,
    allowed_roots_json: String,
    provenance_note: String,
}

#[derive(SurrealValue)]
struct PolicyRow {
    id: RecordId,
    policy_version_id: String,
    policy_id: String,
    policy_version: i64,
    name: String,
    created_at_utc: Datetime,
    default_decision_json: String,
    overrides_json: String,
    allowed_roots_json: String,
    provenance_note: String,
}

#[derive(SurrealValue)]
struct RunWrite {
    record: RecordId,
    run_id: String,
    kernel_task_run_id: String,
    session_run_id: String,
    adapter_kind: String,
    policy_version_id: RecordId,
    workspace_id: String,
    status: String,
    requested_at_utc: Datetime,
    started_at_utc: Option<Datetime>,
    finished_at_utc: Option<Datetime>,
    denial_id: Option<String>,
    artifact_refs: Vec<String>,
    terminal_cause: Option<String>,
    requested_capabilities: Vec<String>,
}

#[derive(SurrealValue)]
struct RunRow {
    id: RecordId,
    run_id: String,
    kernel_task_run_id: String,
    session_run_id: String,
    adapter_kind: String,
    policy_version_id: RecordId,
    workspace_id: String,
    status: String,
    requested_at_utc: Datetime,
    started_at_utc: Option<Datetime>,
    finished_at_utc: Option<Datetime>,
    denial_id: Option<String>,
    artifact_refs: Vec<String>,
    terminal_cause: Option<String>,
    requested_capabilities: Vec<String>,
}

#[derive(SurrealValue)]
struct StatusWrite {
    record: RecordId,
    status: String,
    now: Datetime,
}

#[derive(SurrealValue)]
struct ValidationWrite {
    record: RecordId,
    validation_run_id: String,
    sandbox_run_id: RecordId,
    descriptor_id: String,
    verdict: String,
    check_count: i64,
    failed_check_count: i64,
    report_artifact_ref: Option<String>,
    started_at_utc: Datetime,
    finished_at_utc: Datetime,
    summary_json: String,
}

#[derive(SurrealValue)]
struct ValidationRow {
    id: RecordId,
    validation_run_id: String,
    sandbox_run_id: RecordId,
    descriptor_id: String,
    verdict: String,
    check_count: i64,
    failed_check_count: i64,
    report_artifact_ref: Option<String>,
    started_at_utc: Datetime,
    finished_at_utc: Datetime,
    summary_json: String,
}

#[derive(SurrealValue)]
struct DecisionWrite {
    record: RecordId,
    decision_id: String,
    validation_run_id: RecordId,
    decision: String,
    rationale_short: String,
    decided_at_utc: Datetime,
}

#[derive(SurrealValue)]
struct DecisionRow {
    id: RecordId,
    decision_id: String,
    validation_run_id: RecordId,
    decision: String,
    rationale_short: String,
    decided_at_utc: Datetime,
}

#[derive(SurrealValue)]
struct ReceiptWrite {
    record: RecordId,
    receipt_id: String,
    decision_id: RecordId,
    idempotency_key: String,
    payload_hash: String,
    artifact_ref: Option<String>,
    issued_at_utc: Datetime,
}

#[derive(SurrealValue)]
struct ReceiptRow {
    id: RecordId,
    receipt_id: String,
    decision_id: RecordId,
    idempotency_key: String,
    payload_hash: String,
    artifact_ref: Option<String>,
    issued_at_utc: Datetime,
}

#[derive(SurrealValue)]
struct RecordBinding {
    record: RecordId,
}

#[derive(SurrealValue)]
struct ReplayBinding {
    run: RecordId,
    policy: RecordId,
}

async fn insert_sandbox_policy_version(
    storage: &SurrealStorage,
    policy: SandboxPolicyV1,
) -> Kb003StorageResult<()> {
    policy
        .validate_grants()
        .map_err(|error| Kb003StorageError::Backend(error.to_string()))?;
    let policy_version_id = policy.version_id();
    let bindings = PolicyWrite {
        record: RecordId::new(POLICIES, policy_version_id.clone()),
        policy_version_id,
        policy_id: policy.policy_id,
        policy_version: i64::from(policy.policy_version),
        name: policy.name,
        created_at_utc: Datetime::from(policy.created_at_utc),
        default_decision_json: encode_json(&policy.default_decision)?,
        overrides_json: encode_json(&policy.overrides)?,
        allowed_roots_json: encode_json(&policy.allowed_workspace_roots)?,
        provenance_note: policy.provenance_note,
    };
    create_strict(
        storage,
        "BEGIN TRANSACTION; \
         IF (SELECT VALUE id FROM $record)[0] != NONE { THROW 'HSK-KB003-POLICY-DUPLICATE'; }; \
         CREATE $record CONTENT { policy_version_id: $policy_version_id, policy_id: $policy_id, \
           policy_version: $policy_version, name: $name, created_at_utc: $created_at_utc, \
           default_decision_json: $default_decision_json, overrides_json: $overrides_json, \
           allowed_roots_json: $allowed_roots_json, provenance_note: $provenance_note }; \
         COMMIT TRANSACTION;",
        bindings,
        2,
    )
    .await
}

async fn insert_sandbox_run(storage: &SurrealStorage, run: SandboxRunV1) -> Kb003StorageResult<()> {
    let run_id = run.run_id.0.clone();
    let bindings = RunWrite {
        record: RecordId::new(RUNS, run_id.clone()),
        run_id,
        kernel_task_run_id: run.kernel_task_run_id,
        session_run_id: run.session_run_id,
        adapter_kind: run.adapter_kind,
        policy_version_id: RecordId::new(POLICIES, run.policy_version_id),
        workspace_id: run.workspace_id,
        status: run.status.as_str().to_owned(),
        requested_at_utc: Datetime::from(run.requested_at_utc),
        started_at_utc: run.started_at_utc.map(Datetime::from),
        finished_at_utc: run.finished_at_utc.map(Datetime::from),
        denial_id: run.denial_id,
        artifact_refs: run.artifact_refs,
        terminal_cause: run.terminal_cause.map(terminal_cause_label),
        requested_capabilities: run
            .requested_capabilities
            .into_iter()
            .map(|capability| capability.as_str().to_owned())
            .collect(),
    };
    create_strict(
        storage,
        "BEGIN TRANSACTION; \
         IF (SELECT VALUE id FROM $record)[0] != NONE { THROW 'HSK-KB003-RUN-DUPLICATE'; }; \
         CREATE $record CONTENT { run_id: $run_id, kernel_task_run_id: $kernel_task_run_id, \
           session_run_id: $session_run_id, adapter_kind: $adapter_kind, \
           policy_version_id: $policy_version_id, workspace_id: $workspace_id, status: $status, \
           requested_at_utc: $requested_at_utc, started_at_utc: $started_at_utc, \
           finished_at_utc: $finished_at_utc, denial_id: $denial_id, artifact_refs: $artifact_refs, \
           terminal_cause: $terminal_cause, requested_capabilities: $requested_capabilities }; \
         COMMIT TRANSACTION;",
        bindings,
        2,
    )
    .await
}

async fn update_sandbox_run_status(
    storage: &SurrealStorage,
    run_id: String,
    status: SandboxRunStatus,
) -> Kb003StorageResult<()> {
    let bindings = StatusWrite {
        record: RecordId::new(RUNS, run_id),
        status: status.as_str().to_owned(),
        now: Datetime::from(Utc::now()),
    };
    let rows: Vec<RunRow> = storage
        .with_data_operation(move |database| {
            Box::pin(async move {
                database
                    .query_values_at(
                        "BEGIN TRANSACTION; \
                         LET $current = SELECT * FROM ONLY $record; \
                         IF $current = NONE { THROW 'HSK-KB003-RUN-MISSING'; }; \
                         IF ((($current.status = 'REQUESTED') AND ($status IN ['STARTED', 'REJECTED'])) \
                           OR (($current.status = 'STARTED') AND ($status IN ['COMPLETED', 'REJECTED']))) = false \
                         { THROW 'HSK-KB003-RUN-INVALID-TRANSITION'; }; \
                         UPDATE $record SET status = $status, \
                           started_at_utc = IF $status = 'STARTED' AND started_at_utc = NONE { $now } ELSE { started_at_utc }, \
                           finished_at_utc = IF $status IN ['COMPLETED', 'REJECTED'] { $now } ELSE { finished_at_utc } \
                           RETURN AFTER; \
                         COMMIT TRANSACTION;",
                        bindings,
                        4,
                    )
                    .await
            })
        })
        .await
        .map_err(map_error)?;
    if rows.len() == 1 {
        Ok(())
    } else {
        Err(Kb003StorageError::Backend(
            "KB003 run update returned an unexpected row count".to_owned(),
        ))
    }
}

async fn insert_validation_run(
    storage: &SurrealStorage,
    row: ValidationRunRowV1,
) -> Kb003StorageResult<()> {
    if row.failed_check_count > row.check_count {
        return Err(Kb003StorageError::Backend(
            "failed_check_count exceeds check_count".to_owned(),
        ));
    }
    let bindings = ValidationWrite {
        record: RecordId::new(VALIDATIONS, row.validation_run_id.clone()),
        validation_run_id: row.validation_run_id,
        sandbox_run_id: RecordId::new(RUNS, row.sandbox_run_id),
        descriptor_id: row.descriptor_id,
        verdict: row.verdict,
        check_count: i64::from(row.check_count),
        failed_check_count: i64::from(row.failed_check_count),
        report_artifact_ref: row.report_artifact_ref,
        started_at_utc: parse_datetime(&row.started_at_utc)?,
        finished_at_utc: parse_datetime(&row.finished_at_utc)?,
        summary_json: encode_json(&row.summary_json)?,
    };
    create_strict(
        storage,
        "BEGIN TRANSACTION; \
         IF (SELECT VALUE id FROM $record)[0] != NONE { THROW 'HSK-KB003-VALIDATION-DUPLICATE'; }; \
         CREATE $record CONTENT { validation_run_id: $validation_run_id, sandbox_run_id: $sandbox_run_id, \
           descriptor_id: $descriptor_id, verdict: $verdict, check_count: $check_count, \
           failed_check_count: $failed_check_count, report_artifact_ref: $report_artifact_ref, \
           started_at_utc: $started_at_utc, finished_at_utc: $finished_at_utc, summary_json: $summary_json }; \
         COMMIT TRANSACTION;",
        bindings,
        2,
    )
    .await
}

async fn insert_promotion_decision(
    storage: &SurrealStorage,
    row: PromotionDecisionRowV1,
) -> Kb003StorageResult<()> {
    let bindings = DecisionWrite {
        record: RecordId::new(DECISIONS, row.decision_id.clone()),
        decision_id: row.decision_id,
        validation_run_id: RecordId::new(VALIDATIONS, row.validation_run_id),
        decision: row.decision,
        rationale_short: row.rationale_short,
        decided_at_utc: parse_datetime(&row.decided_at_utc)?,
    };
    create_strict(
        storage,
        "BEGIN TRANSACTION; \
         IF (SELECT VALUE id FROM $record)[0] != NONE { THROW 'HSK-KB003-DECISION-DUPLICATE'; }; \
         CREATE $record CONTENT { decision_id: $decision_id, validation_run_id: $validation_run_id, \
           decision: $decision, rationale_short: $rationale_short, decided_at_utc: $decided_at_utc }; \
         COMMIT TRANSACTION;",
        bindings,
        2,
    )
    .await
}

async fn insert_promotion_receipt(
    storage: &SurrealStorage,
    row: PromotionReceiptRowV1,
) -> Kb003StorageResult<String> {
    let new_hash = row.payload_hash.clone();
    let key = row.idempotency_key.clone();
    let bindings = ReceiptWrite {
        record: RecordId::new(RECEIPTS, row.receipt_id.clone()),
        receipt_id: row.receipt_id,
        decision_id: RecordId::new(DECISIONS, row.decision_id),
        idempotency_key: row.idempotency_key,
        payload_hash: row.payload_hash,
        artifact_ref: row.artifact_ref,
        issued_at_utc: parse_datetime(&row.issued_at_utc)?,
    };
    let result: Result<Vec<ReceiptRow>, super::SurrealStorageError> = storage
        .with_data_operation(move |database| {
            Box::pin(async move {
                database
                    .query_values_at(
                        "BEGIN TRANSACTION; \
                         LET $existing = SELECT * FROM kb003_promotion_receipts \
                           WHERE idempotency_key = $idempotency_key LIMIT 1; \
                         IF array::len($existing) != 0 AND $existing[0].payload_hash != $payload_hash \
                         { THROW 'HSK-KB003-IDEMPOTENCY-CONFLICT'; }; \
                         IF array::len($existing) = 0 { \
                           CREATE $record CONTENT { receipt_id: $receipt_id, decision_id: $decision_id, \
                             idempotency_key: $idempotency_key, payload_hash: $payload_hash, \
                             artifact_ref: $artifact_ref, issued_at_utc: $issued_at_utc }; \
                         }; \
                         COMMIT TRANSACTION; \
                         SELECT * FROM kb003_promotion_receipts WHERE idempotency_key = $idempotency_key LIMIT 1;",
                        bindings,
                        5,
                    )
                    .await
            })
        })
        .await;
    match result {
        Ok(rows) => rows
            .into_iter()
            .next()
            .map(|row| row.receipt_id)
            .ok_or_else(|| {
                Kb003StorageError::Backend(
                    "KB003 promotion receipt insert returned no row".to_owned(),
                )
            }),
        Err(error) => match find_receipt(storage, key.clone()).await? {
            Some(existing) if existing.payload_hash == new_hash => Ok(existing.receipt_id),
            Some(existing) => Err(Kb003StorageError::IdempotencyConflict {
                key,
                existing_hash: existing.payload_hash,
                new_hash,
            }),
            None => Err(map_error(error)),
        },
    }
}

async fn load_run_for_replay(
    storage: &SurrealStorage,
    run_id: String,
    policy_version_id: String,
) -> Kb003StorageResult<ReplayDurableBag> {
    let bindings = ReplayBinding {
        run: RecordId::new(RUNS, run_id.clone()),
        policy: RecordId::new(POLICIES, policy_version_id.clone()),
    };
    let queried_run_id = run_id.clone();
    let (run, policy, validation, decision, receipt) = storage
        .with_data_operation(move |database| {
            Box::pin(async move {
                let run: Option<RunRow> = database
                    .query_first(
                        "SELECT * FROM $record;",
                        RecordBinding {
                            record: bindings.run,
                        },
                    )
                    .await?;
                let policy: Option<PolicyRow> = database
                    .query_first(
                        "SELECT * FROM $record;",
                        RecordBinding {
                            record: bindings.policy,
                        },
                    )
                    .await?;
                let validation: Option<ValidationRow> = database
                    .query_first(
                        "SELECT * FROM kb003_validation_runs WHERE sandbox_run_id = $record \
                         ORDER BY started_at_utc DESC, validation_run_id DESC LIMIT 1;",
                        RecordBinding {
                            record: RecordId::new(RUNS, queried_run_id),
                        },
                    )
                    .await?;
                let decision: Option<DecisionRow> = if let Some(validation) = validation.as_ref() {
                    database
                        .query_first(
                            "SELECT * FROM kb003_promotion_decisions WHERE validation_run_id = $record \
                             ORDER BY decided_at_utc DESC, decision_id DESC LIMIT 1;",
                            RecordBinding {
                                record: validation.id.clone(),
                            },
                        )
                        .await?
                } else {
                    None
                };
                let receipt: Option<ReceiptRow> = if let Some(decision) = decision.as_ref() {
                    database
                        .query_first(
                            "SELECT * FROM kb003_promotion_receipts WHERE decision_id = $record \
                             ORDER BY issued_at_utc DESC, receipt_id DESC LIMIT 1;",
                            RecordBinding {
                                record: decision.id.clone(),
                            },
                        )
                        .await?
                } else {
                    None
                };
                Ok((run, policy, validation, decision, receipt))
            })
        })
        .await
        .map_err(map_error)?;

    let run = run
        .ok_or_else(|| Kb003StorageError::NotFound(format!("run {run_id}")))
        .and_then(run_from_row)?;
    if run.policy_version_id != policy_version_id {
        return Err(Kb003StorageError::Backend(format!(
            "run {run_id} links policy {}, not requested policy {policy_version_id}",
            run.policy_version_id
        )));
    }
    Ok(ReplayDurableBag {
        run,
        policy: policy
            .ok_or_else(|| Kb003StorageError::NotFound(format!("policy {policy_version_id}")))
            .and_then(policy_from_row)?,
        validation: validation.map(validation_from_row).transpose()?,
        decision: decision.map(decision_from_row).transpose()?,
        receipt: receipt.map(receipt_from_row).transpose()?,
    })
}

async fn find_receipt(
    storage: &SurrealStorage,
    idempotency_key: String,
) -> Kb003StorageResult<Option<ReceiptRow>> {
    #[derive(SurrealValue)]
    struct Binding {
        idempotency_key: String,
    }
    storage
        .with_data_operation(move |database| {
            Box::pin(async move {
                database
                    .query_first(
                        "SELECT * FROM kb003_promotion_receipts \
                         WHERE idempotency_key = $idempotency_key LIMIT 1;",
                        Binding { idempotency_key },
                    )
                    .await
            })
        })
        .await
        .map_err(map_error)
}

async fn create_strict<B>(
    storage: &SurrealStorage,
    statement: &'static str,
    bindings: B,
    result_index: usize,
) -> Kb003StorageResult<()>
where
    B: SurrealValue + Send + 'static,
{
    let rows: Vec<surrealdb::types::Value> = storage
        .with_data_operation(move |database| {
            Box::pin(async move {
                database
                    .query_values_at(statement, bindings, result_index)
                    .await
            })
        })
        .await
        .map_err(map_error)?;
    if rows.len() == 1 {
        Ok(())
    } else {
        Err(Kb003StorageError::Backend(
            "KB003 strict insert returned an unexpected row count".to_owned(),
        ))
    }
}

fn policy_from_row(row: PolicyRow) -> Kb003StorageResult<SandboxPolicyV1> {
    ensure_record(row.id, POLICIES, &row.policy_version_id)?;
    let policy_version = u32::try_from(row.policy_version)
        .map_err(|_| Kb003StorageError::Backend("invalid policy_version".to_owned()))?;
    Ok(SandboxPolicyV1 {
        policy_id: row.policy_id,
        policy_version,
        name: row.name,
        created_at_utc: row.created_at_utc.into_inner(),
        default_decision: decode_json::<CapabilityDecision>(&row.default_decision_json)?,
        overrides: decode_json(&row.overrides_json)?,
        allowed_workspace_roots: decode_json(&row.allowed_roots_json)?,
        provenance_note: row.provenance_note,
    })
}

fn run_from_row(row: RunRow) -> Kb003StorageResult<SandboxRunV1> {
    ensure_record(row.id, RUNS, &row.run_id)?;
    Ok(SandboxRunV1 {
        run_id: SandboxRunId(row.run_id),
        kernel_task_run_id: row.kernel_task_run_id,
        session_run_id: row.session_run_id,
        adapter_kind: row.adapter_kind,
        policy_version_id: record_key(row.policy_version_id, POLICIES)?,
        workspace_id: row.workspace_id,
        status: parse_status(&row.status)?,
        requested_at_utc: row.requested_at_utc.into_inner(),
        started_at_utc: row.started_at_utc.map(Datetime::into_inner),
        finished_at_utc: row.finished_at_utc.map(Datetime::into_inner),
        denial_id: row.denial_id,
        artifact_refs: row.artifact_refs,
        terminal_cause: row
            .terminal_cause
            .map(|value| parse_terminal_cause(&value))
            .transpose()?,
        requested_capabilities: row
            .requested_capabilities
            .into_iter()
            .map(|value| parse_capability(&value))
            .collect::<Kb003StorageResult<Vec<_>>>()?,
    })
}

fn validation_from_row(row: ValidationRow) -> Kb003StorageResult<ValidationRunRowV1> {
    ensure_record(row.id, VALIDATIONS, &row.validation_run_id)?;
    Ok(ValidationRunRowV1 {
        validation_run_id: row.validation_run_id,
        sandbox_run_id: record_key(row.sandbox_run_id, RUNS)?,
        descriptor_id: row.descriptor_id,
        verdict: row.verdict,
        check_count: u32::try_from(row.check_count)
            .map_err(|_| Kb003StorageError::Backend("invalid check_count".to_owned()))?,
        failed_check_count: u32::try_from(row.failed_check_count)
            .map_err(|_| Kb003StorageError::Backend("invalid failed_check_count".to_owned()))?,
        report_artifact_ref: row.report_artifact_ref,
        started_at_utc: format_datetime(row.started_at_utc.into_inner()),
        finished_at_utc: format_datetime(row.finished_at_utc.into_inner()),
        summary_json: decode_json(&row.summary_json)?,
    })
}

fn decision_from_row(row: DecisionRow) -> Kb003StorageResult<PromotionDecisionRowV1> {
    ensure_record(row.id, DECISIONS, &row.decision_id)?;
    Ok(PromotionDecisionRowV1 {
        decision_id: row.decision_id,
        validation_run_id: record_key(row.validation_run_id, VALIDATIONS)?,
        decision: row.decision,
        rationale_short: row.rationale_short,
        decided_at_utc: format_datetime(row.decided_at_utc.into_inner()),
    })
}

fn receipt_from_row(row: ReceiptRow) -> Kb003StorageResult<PromotionReceiptRowV1> {
    ensure_record(row.id, RECEIPTS, &row.receipt_id)?;
    Ok(PromotionReceiptRowV1 {
        receipt_id: row.receipt_id,
        decision_id: record_key(row.decision_id, DECISIONS)?,
        idempotency_key: row.idempotency_key,
        payload_hash: row.payload_hash,
        artifact_ref: row.artifact_ref,
        issued_at_utc: format_datetime(row.issued_at_utc.into_inner()),
    })
}

fn parse_datetime(value: &str) -> Kb003StorageResult<Datetime> {
    DateTime::parse_from_rfc3339(value)
        .map(|value| Datetime::from(value.with_timezone(&Utc)))
        .map_err(|error| Kb003StorageError::Backend(format!("invalid RFC3339 datetime: {error}")))
}

fn format_datetime(value: DateTime<Utc>) -> String {
    value.to_rfc3339_opts(SecondsFormat::AutoSi, true)
}

fn encode_json<T: serde::Serialize>(value: &T) -> Kb003StorageResult<String> {
    serde_json::to_string(value).map_err(|error| Kb003StorageError::Backend(error.to_string()))
}

fn decode_json<T: serde::de::DeserializeOwned>(value: &str) -> Kb003StorageResult<T> {
    serde_json::from_str(value).map_err(|error| Kb003StorageError::Backend(error.to_string()))
}

fn ensure_record(record: RecordId, table: &'static str, expected: &str) -> Kb003StorageResult<()> {
    let actual = record_key(record, table)?;
    if actual == expected {
        Ok(())
    } else {
        Err(Kb003StorageError::Backend(format!(
            "{table} record key `{actual}` does not match alias `{expected}`"
        )))
    }
}

fn record_key(record: RecordId, table: &'static str) -> Kb003StorageResult<String> {
    if record.table.as_str() != table {
        return Err(Kb003StorageError::Backend(format!(
            "expected {table} record link, got {}",
            record.table.as_str()
        )));
    }
    match record.key {
        RecordIdKey::String(value) => Ok(value),
        _ => Err(Kb003StorageError::Backend(format!(
            "{table} record link is not a string key"
        ))),
    }
}

fn parse_status(value: &str) -> Kb003StorageResult<SandboxRunStatus> {
    match value {
        "REQUESTED" => Ok(SandboxRunStatus::Requested),
        "STARTED" => Ok(SandboxRunStatus::Started),
        "COMPLETED" => Ok(SandboxRunStatus::Completed),
        "REJECTED" => Ok(SandboxRunStatus::Rejected),
        _ => Err(Kb003StorageError::Backend(format!(
            "unknown KB003 run status `{value}`"
        ))),
    }
}

fn terminal_cause_label(value: TerminalCause) -> String {
    match value {
        TerminalCause::CompletedOk => "COMPLETED_OK",
        TerminalCause::CancelledByOperator => "CANCELLED_BY_OPERATOR",
        TerminalCause::WallTimeoutExpired => "WALL_TIMEOUT_EXPIRED",
        TerminalCause::CpuTimeoutExpired => "CPU_TIMEOUT_EXPIRED",
    }
    .to_owned()
}

fn parse_terminal_cause(value: &str) -> Kb003StorageResult<TerminalCause> {
    match value {
        "COMPLETED_OK" => Ok(TerminalCause::CompletedOk),
        "CANCELLED_BY_OPERATOR" => Ok(TerminalCause::CancelledByOperator),
        "WALL_TIMEOUT_EXPIRED" => Ok(TerminalCause::WallTimeoutExpired),
        "CPU_TIMEOUT_EXPIRED" => Ok(TerminalCause::CpuTimeoutExpired),
        _ => Err(Kb003StorageError::Backend(format!(
            "unknown KB003 terminal cause `{value}`"
        ))),
    }
}

fn parse_capability(value: &str) -> Kb003StorageResult<SandboxCapability> {
    match value {
        "FILESYSTEM_ESCAPE" => Ok(SandboxCapability::FilesystemEscape),
        "NETWORK" => Ok(SandboxCapability::Network),
        "PROCESS_SPAWN" => Ok(SandboxCapability::ProcessSpawn),
        "DEVICE" => Ok(SandboxCapability::Device),
        "ENVIRONMENT_LEAK" => Ok(SandboxCapability::EnvironmentLeak),
        "SECRET_READ" => Ok(SandboxCapability::SecretRead),
        _ => Err(Kb003StorageError::Backend(format!(
            "unknown KB003 capability `{value}`"
        ))),
    }
}

fn map_error(error: super::SurrealStorageError) -> Kb003StorageError {
    let message = error.to_string();
    if message.contains("HSK-KB003-RUN-MISSING") {
        Kb003StorageError::NotFound("sandbox run".to_owned())
    } else {
        Kb003StorageError::Backend(message)
    }
}

#[cfg(test)]
mod tests {
    use super::super::{schema, SurrealStorageConfig};
    use super::*;
    use crate::kernel::mte_authority_mutation_boundary::AuthorityMutationActor;
    use tempfile::TempDir;

    async fn open(path: &std::path::Path) -> SurrealStorage {
        let storage = SurrealStorage::open(
            SurrealStorageConfig::with_path(path).expect("valid KB003 test path"),
        )
        .await
        .expect("open embedded KB003 store");
        schema::bootstrap_schema(&storage)
            .await
            .expect("bootstrap embedded KB003 schema");
        storage
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn kb003_rows_and_idempotency_survive_close_reopen() {
        let directory = TempDir::new().expect("temporary KB003 root");
        let path = directory.path().join("store");
        let storage = open(&path).await;
        let mut adapter = SurrealKb003Storage::new(storage.clone());
        let policy = SandboxPolicyV1::default_deny("embedded");
        let policy_id = policy.version_id();
        adapter.insert_sandbox_policy_version(&policy).unwrap();
        let run = SandboxRunV1::new_requested("KTR-1", "SES-1", "local", &policy_id, "WSP-1");
        let run_id = run.run_id.0.clone();
        adapter.insert_sandbox_run(&run).unwrap();
        adapter
            .update_sandbox_run_status(&run_id, SandboxRunStatus::Started)
            .unwrap();
        let validation = ValidationRunRowV1 {
            validation_run_id: "VR-1".to_owned(),
            sandbox_run_id: run_id.clone(),
            descriptor_id: "DESC-1".to_owned(),
            verdict: "PASS".to_owned(),
            check_count: 1,
            failed_check_count: 0,
            report_artifact_ref: None,
            started_at_utc: "2026-05-17T00:00:00Z".to_owned(),
            finished_at_utc: "2026-05-17T00:00:01Z".to_owned(),
            summary_json: serde_json::json!({"green": true}),
        };
        adapter.insert_validation_run(&validation).unwrap();
        let decision = PromotionDecisionRowV1 {
            decision_id: "PD-1".to_owned(),
            validation_run_id: validation.validation_run_id.clone(),
            decision: "PROMOTED".to_owned(),
            rationale_short: "green".to_owned(),
            decided_at_utc: "2026-05-17T00:00:02Z".to_owned(),
        };
        adapter
            .insert_promotion_decision(&decision, AuthorityMutationActor::PromotionGate)
            .unwrap();
        let receipt = PromotionReceiptRowV1 {
            receipt_id: "PR-1".to_owned(),
            decision_id: decision.decision_id.clone(),
            idempotency_key: "IDEMP-1".to_owned(),
            payload_hash: "sha256:one".to_owned(),
            artifact_ref: None,
            issued_at_utc: "2026-05-17T00:00:03Z".to_owned(),
        };
        let first = adapter
            .insert_promotion_receipt(&receipt, AuthorityMutationActor::PromotionGate)
            .unwrap();
        let second = adapter
            .insert_promotion_receipt(&receipt, AuthorityMutationActor::PromotionGate)
            .unwrap();
        assert_eq!(first, second);
        let mut conflicting_receipt = receipt.clone();
        conflicting_receipt.receipt_id = "PR-2".to_owned();
        conflicting_receipt.payload_hash = "sha256:two".to_owned();
        assert!(matches!(
            adapter.insert_promotion_receipt(
                &conflicting_receipt,
                AuthorityMutationActor::PromotionGate,
            ),
            Err(Kb003StorageError::IdempotencyConflict { .. })
        ));
        drop(adapter);
        storage.shutdown().await.expect("close KB003 store");
        drop(storage);

        let reopened = open(&path).await;
        let adapter = SurrealKb003Storage::new(reopened.clone());
        let bag = adapter.load_run_for_replay(&run_id, &policy_id).unwrap();
        assert_eq!(bag.validation, Some(validation));
        assert_eq!(bag.decision, Some(decision));
        assert_eq!(bag.receipt, Some(receipt));
        reopened
            .shutdown()
            .await
            .expect("close reopened KB003 store");
    }
}
