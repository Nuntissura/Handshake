use std::{
    sync::{Arc, OnceLock},
    time::Duration,
};

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use surrealdb::types::{RecordId, SurrealValue, Value as SurrealDataValue};
use thiserror::Error;
use tokio::{
    task::JoinHandle,
    time::{self, MissedTickBehavior},
};
use uuid::Uuid;

use super::{
    LedgerEvent, LedgerEventKind, ProcessEngineKind, ProcessLedgerError, ProcessLedgerStore,
    ProcessStop, SurrealProcessLedgerStore,
};

/// Sentinel written into `stop_reason` when the claim finds the column empty.
///
/// It is also the marker the follow-up STOP upsert overwrites, and — together
/// with the bound claim timestamp — the precise guard the compensating
/// un-claim uses.
const RECLAIM_CLAIM_STOP_REASON: &str = "reclaim_claimed";
const RECLAIM_KILLED_STOP_REASON: &str = "reclaim_killed";
const SURREAL_CLAIM_MAX_ATTEMPTS: usize = 10;
const SURREAL_CLAIM_BACKOFF_CAP_MS: u64 = 32;
static RECLAIM_BOOT_OWNER_ID: OnceLock<Uuid> = OnceLock::new();

fn reclaim_boot_owner_id() -> Uuid {
    *RECLAIM_BOOT_OWNER_ID.get_or_init(Uuid::now_v7)
}

fn is_surreal_retryable_transaction_conflict(
    error: &crate::storage::surreal::SurrealStorageError,
) -> bool {
    matches!(
        error,
        crate::storage::surreal::SurrealStorageError::Database(source)
            if source
                .to_string()
                .contains("Transaction conflict: Resource busy. This transaction can be retried")
    )
}

fn surreal_claim_retry_delay(seed: Uuid, failed_attempt: usize) -> Duration {
    let exponential_cap = 1_u64
        .checked_shl(failed_attempt.min(5) as u32)
        .unwrap_or(SURREAL_CLAIM_BACKOFF_CAP_MS)
        .min(SURREAL_CLAIM_BACKOFF_CAP_MS);
    let seed = seed.as_u128();
    let mut mixed = (seed as u64)
        ^ ((seed >> 64) as u64)
        ^ (failed_attempt as u64).wrapping_mul(0x9e37_79b9_7f4a_7c15);
    mixed ^= mixed >> 30;
    mixed = mixed.wrapping_mul(0xbf58_476d_1ce4_e5b9);
    mixed ^= mixed >> 27;
    mixed = mixed.wrapping_mul(0x94d0_49bb_1331_11eb);
    mixed ^= mixed >> 31;
    Duration::from_millis(mixed % (exponential_cap + 1))
}

/// MT-008: atomically claim the active (un-stopped) rows for a session.
///
/// The original PostgreSQL form was `SELECT ... FOR UPDATE` executed in a
/// transaction that was committed immediately, releasing the row locks *before*
/// the reclaim decision (sandbox kill + stop-event write) ran. Two concurrent
/// reclaims could therefore both read the same active rows and both act on
/// them, double-killing a process and writing duplicate STOP rows (or, under
/// interleaving, missing a row).
///
/// The exclusion must cover the rows being reclaimed for the *whole* reclaim
/// decision, so the read-modify-write is collapsed into ONE statement. SurrealDB
/// runs a single statement in its own transaction, so this `UPDATE ... WHERE
/// stopped_at = NONE ... RETURN BEFORE` is the direct equivalent of the
/// `UPDATE ... WHERE stopped_at IS NULL ... RETURNING` it replaces: it claims
/// each matching row by stamping `stopped_at`, and a concurrent reclaim's
/// identical statement then matches zero of those rows because `stopped_at` is
/// no longer `NONE`. Claiming and excluding happen in the same commit, so two
/// racing reclaims cannot both observe a row as active.
///
/// Two details are load-bearing:
///
/// * The claim timestamp is BOUND from the caller rather than produced by
///   `time::now()` server-side. Knowing the exact stamp we wrote is what lets
///   the compensating un-claim below target only rows this claim actually took:
///   if a real STOP landed in between it overwrote `stopped_at`, the guard no
///   longer matches, and the row is correctly left alone.
/// * `RETURN BEFORE` yields the pre-claim row. Every field the caller needs is
///   an identity field the UPDATE does not touch, so BEFORE and AFTER are
///   identical for them — but BEFORE additionally carries the original
///   `stop_reason`, which the compensation restores verbatim.
/// * The same UPDATE persists the exact prior terminal pair under a reserved
///   metadata key before replacing it. If the returned row is too malformed to
///   identify, compensation re-reads these per-record receipts; it never resets
///   a whole session to an invented `NONE/NONE` state.
///
/// The claim is durable the instant the statement commits; the kill outcome /
/// exit_code is refined afterward by the normal STOP upsert, which overwrites
/// the sentinel `stopped_at` with the real stop time. The owner suffix in
/// `stop_reason` is a process-boot token, not a time lease: every store handle in
/// one live process shares it, so a slow kill can never be stolen merely because
/// time elapsed. A restarted process has a different suffix and can immediately
/// recover the prior boot's abandoned sentinel.
pub const SURREAL_ACTIVE_RECLAIM_CLAIM_QUERY: &str = "UPDATE kernel_process_lifecycle SET \
     metadata.__handshake_internal_reclaim_durable_receipt_v1 = IF stop_reason = $killed_reason { \
         NONE \
     } ELSE { { \
         receipt_kind: 'handshake_reclaim_durable_receipt_v1', \
         previous_stopped_at: stopped_at, \
         previous_stop_reason: stop_reason \
     } }, \
     stopped_at = IF stop_reason = $killed_reason { stopped_at } ELSE { $claimed_at }, \
     stop_reason = IF stop_reason = $killed_reason { stop_reason } ELSE { $claim_reason } \
     WHERE parent_session_id = $session_id AND ( \
         stopped_at = NONE OR ( \
             exit_code = NONE AND ( \
                 (string::starts_with(stop_reason, $claim_prefix) \
                  AND stop_reason != $claim_reason) \
                 OR string::starts_with(stop_reason, $killed_prefix) \
             ) \
         ) \
     ) RETURN BEFORE;";

/// Releases one row claimed by [`SURREAL_ACTIVE_RECLAIM_CLAIM_QUERY`].
///
/// `stopped_at = $claimed_at` is the whole safety of this statement: it matches
/// only a row still carrying the exact sentinel this claim wrote. A row whose
/// STOP has since landed has a different `stopped_at` and is skipped.
const SURREAL_ACTIVE_RECLAIM_RELEASE_QUERY: &str = "UPDATE kernel_process_lifecycle SET \
     stopped_at = $previous_stopped_at, stop_reason = $stop_reason, \
     metadata.__handshake_internal_reclaim_durable_receipt_v1 = NONE \
     WHERE id = $record AND stopped_at = $claimed_at \
       AND stop_reason = $claim_reason RETURN AFTER;";

const SURREAL_ACTIVE_RECLAIM_OUTSTANDING_SESSION_QUERY: &str = "SELECT VALUE id FROM \
     kernel_process_lifecycle WHERE parent_session_id = $session_id \
     AND stopped_at = $claimed_at AND stop_reason = $claim_reason;";

const SURREAL_ACTIVE_RECLAIM_DURABLE_RECEIPTS_QUERY: &str = "SELECT id, \
     metadata.__handshake_internal_reclaim_durable_receipt_v1.receipt_kind AS receipt_kind, \
     metadata.__handshake_internal_reclaim_durable_receipt_v1.previous_stopped_at AS previous_stopped_at, \
     metadata.__handshake_internal_reclaim_durable_receipt_v1.previous_stop_reason AS previous_stop_reason \
     FROM kernel_process_lifecycle WHERE parent_session_id = $session_id \
     AND stopped_at = $claimed_at AND stop_reason = $claim_reason;";

const SURREAL_ACTIVE_RECLAIM_SAME_OWNER_QUERY: &str = "SELECT VALUE id FROM \
     kernel_process_lifecycle WHERE parent_session_id = $session_id \
     AND exit_code = NONE AND stop_reason = $claim_reason;";

const SURREAL_ACTIVE_RECLAIM_ABANDON_QUERY: &str = "UPDATE kernel_process_lifecycle SET \
     stopped_at = $previous_stopped_at, stop_reason = $previous_stop_reason, \
     metadata.__handshake_internal_reclaim_durable_receipt_v1 = NONE \
     WHERE id = $record AND stopped_at = $claimed_at \
        AND exit_code = NONE AND stop_reason = $claim_reason RETURN AFTER;";

const SURREAL_ACTIVE_RECLAIM_CANONICAL_READ_QUERY: &str = "SELECT stopped_at, stop_reason, \
     exit_code FROM ONLY $record;";

const RECLAIM_RECEIPT_METADATA_KEY: &str = "__handshake_internal_reclaim_receipt_v1";
const RECLAIM_DURABLE_RECEIPT_METADATA_KEY: &str =
    "__handshake_internal_reclaim_durable_receipt_v1";
const RECLAIM_DURABLE_RECEIPT_KIND: &str = "handshake_reclaim_durable_receipt_v1";

const SURREAL_ACTIVE_RECLAIM_MARK_KILLED_QUERY: &str = "UPDATE kernel_process_lifecycle SET \
     stop_reason = $killed_reason \
     WHERE process_uuid = $process_uuid AND stopped_at = $claimed_at \
       AND exit_code = NONE AND stop_reason = $claim_reason RETURN VALUE process_uuid;";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReclaimTrigger {
    Close,
    Failure,
    Stale,
    OperatorCancel,
}

impl ReclaimTrigger {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Close => "close",
            Self::Failure => "failure",
            Self::Stale => "stale",
            Self::OperatorCancel => "operator_cancel",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReclaimableProcess {
    pub process_uuid: Uuid,
    pub os_pid: Option<u32>,
    pub parent_session_id: String,
    pub parent_process_id: Option<Uuid>,
    pub sandbox_adapter_id: Option<String>,
    pub sandbox_internal_id: Option<String>,
    pub engine_kind: ProcessEngineKind,
    pub started_at: DateTime<Utc>,
    pub model_artifact_sha256: Option<String>,
    pub work_profile_id: Option<String>,
    pub owner_role: String,
    pub owner_wp: Option<String>,
    pub role_id: Option<String>,
    pub wp_id: Option<String>,
    pub mt_id: Option<String>,
    pub sandbox_capabilities_snapshot: serde_json::Value,
    pub metadata_jsonb: serde_json::Value,
    /// Exact timestamp written by the atomic reclaim claim. The terminal STOP
    /// must present this stamp before it may replace the durable sentinel.
    pub reclaim_claimed_at: DateTime<Utc>,
    /// Exact owner-qualified sentinel reason that must still be durable when
    /// the terminal reclaim STOP is applied.
    pub reclaim_expected_reason: String,
    /// Exact owner-qualified cleanup-completed sentinel that may replace the
    /// claim reason before terminalization.
    pub reclaim_expected_killed_reason: String,
    /// `true` when this claim re-acquired a durable post-cleanup marker from a
    /// prior attempt. The external cleanup must not be repeated; only the exact
    /// terminal STOP remains.
    pub reclaim_cleanup_completed: bool,
}

impl ReclaimableProcess {
    fn reclaim_receipt(&self) -> Result<ReclaimPriorState, ProcessLedgerError> {
        let receipt = self
            .metadata_jsonb
            .as_object()
            .and_then(|metadata| metadata.get(RECLAIM_RECEIPT_METADATA_KEY))
            .ok_or_else(|| {
                ProcessLedgerError::Store(format!(
                    "missing exact reclaim receipt for process {}",
                    self.process_uuid
                ))
            })?;
        serde_json::from_value(receipt.clone()).map_err(|error| {
            ProcessLedgerError::Store(format!(
                "invalid exact reclaim receipt for process {}: {error}",
                self.process_uuid
            ))
        })
    }

    fn metadata_without_reclaim_receipt(&self) -> serde_json::Value {
        let mut metadata = self.metadata_jsonb.clone();
        if let Some(object) = metadata.as_object_mut() {
            let is_our_receipt = object
                .get(RECLAIM_RECEIPT_METADATA_KEY)
                .and_then(|value| serde_json::from_value::<ReclaimPriorState>(value.clone()).ok())
                .is_some_and(|receipt| receipt.record_process_uuid == self.process_uuid);
            if is_our_receipt {
                object.remove(RECLAIM_RECEIPT_METADATA_KEY);
            }
        }
        metadata
    }

    pub fn reclaim_stop(&self, exit_code: i32) -> ProcessStop {
        ProcessStop {
            process_uuid: self.process_uuid,
            os_pid: self.os_pid,
            parent_session_id: Some(self.parent_session_id.clone()),
            parent_process_id: self.parent_process_id,
            sandbox_adapter_id: self.sandbox_adapter_id.clone(),
            sandbox_internal_id: self.sandbox_internal_id.clone(),
            engine_kind: self.engine_kind,
            started_at: self.started_at,
            stopped_at: Utc::now(),
            exit_code: Some(exit_code),
            stop_reason: Some("reclaim".to_string()),
            model_artifact_sha256: self.model_artifact_sha256.clone(),
            work_profile_id: self.work_profile_id.clone(),
            owner_role: self.owner_role.clone(),
            owner_wp: self.owner_wp.clone(),
            role_id: self.role_id.clone(),
            wp_id: self.wp_id.clone(),
            mt_id: self.mt_id.clone(),
            sandbox_capabilities_snapshot: self.sandbox_capabilities_snapshot.clone(),
            metadata_jsonb: self.metadata_without_reclaim_receipt(),
            reclaim_claimed_at: Some(self.reclaim_claimed_at),
            reclaim_expected_reason: Some(self.reclaim_expected_reason.clone()),
            reclaim_expected_killed_reason: Some(self.reclaim_expected_killed_reason.clone()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct ReclaimPriorState {
    record: RecordId,
    record_process_uuid: Uuid,
    previous_stopped_at: Option<DateTime<Utc>>,
    previous_stop_reason: Option<String>,
}

#[derive(Debug, Clone, Error, PartialEq, Eq)]
#[error("{message}")]
pub struct KillError {
    message: String,
}

impl KillError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }

    pub fn message(&self) -> &str {
        &self.message
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum KillOutcome {
    Killed,
    Failed { error: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReclaimedProcess {
    pub process_uuid: Uuid,
    pub engine_kind: ProcessEngineKind,
    pub sandbox_adapter_id: Option<String>,
    pub kill_result: KillOutcome,
    pub stop_event_kind: LedgerEventKind,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReclaimReport {
    pub session_id: String,
    pub trigger: ReclaimTrigger,
    pub processes_reclaimed: Vec<ReclaimedProcess>,
    pub total_duration_ms: u128,
}

#[async_trait]
pub trait ReclaimProcessStore: Send + Sync + 'static {
    async fn active_processes_for_session(
        &self,
        session_id: &str,
    ) -> Result<Vec<ReclaimableProcess>, ProcessLedgerError>;

    /// Durably records that external cleanup succeeded before terminal STOP.
    /// A later claim may then finish the ledger transition without repeating
    /// the external kill/cleanup side effect.
    async fn mark_cleanup_completed(
        &self,
        process: &ReclaimableProcess,
    ) -> Result<(), ProcessLedgerError>;

    /// Release claims whose external cleanup has not been durably recorded, so
    /// the same boot can retry instead of waiting for process restart.
    async fn abandon(&self, processes: &[ReclaimableProcess]) -> Result<(), ProcessLedgerError>;
}

pub trait SandboxKill: Send + Sync + 'static {
    fn kill(&self, process_uuid: Uuid) -> Result<(), KillError>;
}

#[async_trait]
pub trait ReclaimStopWriter: Send + Sync + 'static {
    /// Persist the exact reclaim STOP before returning success. Queue acceptance
    /// is not sufficient because reclaim reports are durability receipts.
    async fn append_reclaim_stop(&self, stop: ProcessStop) -> Result<(), ProcessLedgerError>;
}

pub struct Reclaim {
    store: Arc<dyn ReclaimProcessStore>,
    sandbox_kill: Arc<dyn SandboxKill>,
    stop_writer: Arc<dyn ReclaimStopWriter>,
}

impl Reclaim {
    pub fn new<S, K, W>(store: Arc<S>, sandbox_kill: Arc<K>, stop_writer: Arc<W>) -> Self
    where
        S: ReclaimProcessStore,
        K: SandboxKill,
        W: ReclaimStopWriter,
    {
        Self {
            store,
            sandbox_kill,
            stop_writer,
        }
    }

    async fn release_after_failure(
        &self,
        claimed: &[ReclaimableProcess],
        cause: String,
    ) -> ProcessLedgerError {
        match self.store.abandon(claimed).await {
            Ok(()) => ProcessLedgerError::Store(cause),
            Err(release_error) => ProcessLedgerError::Store(format!(
                "{cause}; exact reclaim-claim release also failed: {release_error}"
            )),
        }
    }

    pub async fn run(
        &self,
        session_id: &str,
        trigger: ReclaimTrigger,
    ) -> Result<ReclaimReport, ProcessLedgerError> {
        let started = std::time::Instant::now();
        let active = self.store.active_processes_for_session(session_id).await?;
        let mut reclaimed = Vec::with_capacity(active.len());

        for (index, process) in active.iter().enumerate() {
            let kill_result = if process.reclaim_cleanup_completed {
                KillOutcome::Killed
            } else {
                match self.sandbox_kill.kill(process.process_uuid) {
                    Ok(()) => {
                        if let Err(error) = self.store.mark_cleanup_completed(process).await {
                            return Err(self
                                .release_after_failure(
                                    &active[index..],
                                    format!(
                                        "durable post-cleanup marker failed for process {}: {error}",
                                        process.process_uuid
                                    ),
                                )
                                .await);
                        }
                        KillOutcome::Killed
                    }
                    Err(error) => {
                        return Err(self
                            .release_after_failure(
                                &active[index..],
                                format!(
                                    "external reclaim cleanup failed for process {}: {}",
                                    process.process_uuid,
                                    error.message()
                                ),
                            )
                            .await);
                    }
                }
            };
            if let Err(error) = self
                .stop_writer
                .append_reclaim_stop(process.reclaim_stop(-1))
                .await
            {
                let release_from = if process.reclaim_cleanup_completed
                    || matches!(&kill_result, KillOutcome::Killed)
                {
                    index.saturating_add(1)
                } else {
                    index
                };
                return Err(self
                    .release_after_failure(
                        &active[release_from..],
                        format!(
                            "durable reclaim STOP failed for process {}: {error}",
                            process.process_uuid
                        ),
                    )
                    .await);
            }
            reclaimed.push(ReclaimedProcess {
                process_uuid: process.process_uuid,
                engine_kind: process.engine_kind,
                sandbox_adapter_id: process.sandbox_adapter_id.clone(),
                kill_result,
                stop_event_kind: LedgerEventKind::Stop,
            });
        }

        Ok(ReclaimReport {
            session_id: session_id.to_string(),
            trigger,
            processes_reclaimed: reclaimed,
            total_duration_ms: started.elapsed().as_millis(),
        })
    }
}

pub fn reclaim_handle<S, K, W>(store: Arc<S>, sandbox_kill: Arc<K>, stop_writer: Arc<W>) -> Reclaim
where
    S: ReclaimProcessStore,
    K: SandboxKill,
    W: ReclaimStopWriter,
{
    Reclaim::new(store, sandbox_kill, stop_writer)
}

#[async_trait]
impl ReclaimStopWriter for SurrealProcessLedgerStore {
    async fn append_reclaim_stop(&self, stop: ProcessStop) -> Result<(), ProcessLedgerError> {
        self.write_batch(vec![LedgerEvent::Stop(stop)]).await
    }
}

/// The projection the claim statement returns.
///
/// Field types mirror the `SCHEMAFULL` `kernel_process_lifecycle` definition, so
/// the conversions the PostgreSQL version had to perform by hand no longer
/// exist: `process_uuid`/`parent_process_id` arrive as native `uuid`, and the
/// two JSON columns arrive as `object FLEXIBLE` rather than text that had to be
/// re-parsed. Only `engine_kind` and `os_pid` still need a checked conversion,
/// and both are constrained by the schema (`engine_kind` is a literal union
/// matching [`ProcessEngineKind`] exactly; `os_pid` asserts `>= 0`).
#[derive(Debug, Clone, SurrealValue)]
struct ClaimedRow {
    process_uuid: Uuid,
    os_pid: Option<i64>,
    parent_session_id: Option<String>,
    parent_process_id: Option<Uuid>,
    sandbox_adapter_id: Option<String>,
    sandbox_internal_id: Option<String>,
    engine_kind: String,
    started_at: DateTime<Utc>,
    stopped_at: Option<DateTime<Utc>>,
    stop_reason: Option<String>,
    model_artifact_sha256: Option<String>,
    work_profile_id: Option<String>,
    owner_role: String,
    owner_wp: Option<String>,
    role_id: Option<String>,
    wp_id: Option<String>,
    mt_id: Option<String>,
    sandbox_capabilities_snapshot: Value,
    metadata: Value,
}

#[derive(Debug, Clone)]
struct ClaimReceipt {
    record: RecordId,
    previous_stopped_at: Option<DateTime<Utc>>,
    previous_stop_reason: Option<String>,
    raw_row: SurrealDataValue,
}

impl ClaimReceipt {
    fn from_raw(raw_row: SurrealDataValue) -> Result<Self, ProcessLedgerError> {
        let SurrealDataValue::Object(object) = &raw_row else {
            return Err(ProcessLedgerError::Store(
                "reclaim claim returned a non-object row".to_string(),
            ));
        };
        let record = object
            .get("id")
            .cloned()
            .ok_or_else(|| {
                ProcessLedgerError::Store(
                    "reclaim claim returned a row without its record id".to_string(),
                )
            })
            .and_then(|value| {
                RecordId::from_value(value).map_err(|error| {
                    ProcessLedgerError::Store(format!(
                        "reclaim claim returned an invalid record id: {error}"
                    ))
                })
            })?;
        let previous_stopped_at = Option::<DateTime<Utc>>::from_value(
            object
                .get("stopped_at")
                .cloned()
                .unwrap_or(SurrealDataValue::None),
        )
        .map_err(|error| {
            ProcessLedgerError::Store(format!(
                "reclaim claim returned an invalid prior stopped_at for {record:?}: {error}"
            ))
        })?;
        let previous_stop_reason = Option::<String>::from_value(
            object
                .get("stop_reason")
                .cloned()
                .unwrap_or(SurrealDataValue::None),
        )
        .map_err(|error| {
            ProcessLedgerError::Store(format!(
                "reclaim claim returned an invalid prior stop_reason for {record:?}: {error}"
            ))
        })?;
        Ok(Self {
            record,
            previous_stopped_at,
            previous_stop_reason,
            raw_row,
        })
    }

    fn decode(&self) -> Result<ClaimedRow, ProcessLedgerError> {
        ClaimedRow::from_value(self.raw_row.clone()).map_err(|error| {
            ProcessLedgerError::Store(format!(
                "reclaim claimed row {:?} failed typed decode: {error}",
                self.record
            ))
        })
    }
}

impl ClaimedRow {
    fn into_reclaimable(
        self,
        session_id: &str,
        reclaim_claimed_at: DateTime<Utc>,
        reclaim_expected_reason: String,
        reclaim_expected_killed_reason: String,
        record: RecordId,
    ) -> Result<ReclaimableProcess, ProcessLedgerError> {
        let reclaim_cleanup_completed = self.stop_reason.as_deref().is_some_and(|reason| {
            reason
                .strip_prefix(RECLAIM_KILLED_STOP_REASON)
                .is_some_and(|suffix| suffix.starts_with(':'))
        });
        let receipt = ReclaimPriorState {
            record,
            record_process_uuid: self.process_uuid,
            previous_stopped_at: self.stopped_at,
            previous_stop_reason: self.stop_reason.clone(),
        };
        let mut metadata = self.metadata;
        let metadata_object = metadata.as_object_mut().ok_or_else(|| {
            ProcessLedgerError::Store(format!(
                "reclaim metadata for process {} is not an object",
                self.process_uuid
            ))
        })?;
        if metadata_object.contains_key(RECLAIM_RECEIPT_METADATA_KEY) {
            return Err(ProcessLedgerError::Store(format!(
                "reclaim metadata for process {} uses reserved key {RECLAIM_RECEIPT_METADATA_KEY}",
                self.process_uuid
            )));
        }
        if let Some(durable_receipt) = metadata_object
            .get(RECLAIM_DURABLE_RECEIPT_METADATA_KEY)
            .cloned()
        {
            let receipt_kind = durable_receipt
                .as_object()
                .and_then(|receipt| receipt.get("receipt_kind"))
                .and_then(Value::as_str);
            if receipt_kind != Some(RECLAIM_DURABLE_RECEIPT_KIND) {
                return Err(ProcessLedgerError::Store(format!(
                    "reclaim metadata for process {} uses reserved key {RECLAIM_DURABLE_RECEIPT_METADATA_KEY}",
                    self.process_uuid
                )));
            }
            metadata_object.remove(RECLAIM_DURABLE_RECEIPT_METADATA_KEY);
        }
        metadata_object.insert(
            RECLAIM_RECEIPT_METADATA_KEY.to_owned(),
            serde_json::to_value(receipt).map_err(|error| {
                ProcessLedgerError::Store(format!(
                    "failed to retain exact reclaim receipt for process {}: {error}",
                    self.process_uuid
                ))
            })?,
        );
        Ok(ReclaimableProcess {
            process_uuid: self.process_uuid,
            os_pid: self.os_pid.map(os_pid_to_u32).transpose()?,
            // The claim's `WHERE parent_session_id = $session_id` makes this
            // provably equal to `session_id`; the fallback only covers the
            // schema's `option<string>` typing, it is not a guess.
            parent_session_id: self
                .parent_session_id
                .unwrap_or_else(|| session_id.to_owned()),
            parent_process_id: self.parent_process_id,
            sandbox_adapter_id: self.sandbox_adapter_id,
            sandbox_internal_id: self.sandbox_internal_id,
            engine_kind: ProcessEngineKind::try_from(self.engine_kind.as_str())
                .map_err(ProcessLedgerError::Store)?,
            started_at: self.started_at,
            model_artifact_sha256: self.model_artifact_sha256,
            work_profile_id: self.work_profile_id,
            owner_role: self.owner_role,
            owner_wp: self.owner_wp,
            role_id: self.role_id,
            wp_id: self.wp_id,
            mt_id: self.mt_id,
            sandbox_capabilities_snapshot: self.sandbox_capabilities_snapshot,
            metadata_jsonb: metadata,
            reclaim_claimed_at,
            reclaim_expected_reason,
            reclaim_expected_killed_reason,
            reclaim_cleanup_completed,
        })
    }
}

#[derive(Clone, SurrealValue)]
struct ClaimBindings {
    session_id: String,
    claimed_at: DateTime<Utc>,
    claim_prefix: String,
    killed_prefix: String,
    claim_reason: String,
    killed_reason: String,
}

#[derive(Clone, SurrealValue)]
struct ReleaseBindings {
    record: RecordId,
    claimed_at: DateTime<Utc>,
    previous_stopped_at: Option<DateTime<Utc>>,
    stop_reason: Option<String>,
    claim_reason: String,
}

#[derive(Clone, SurrealValue)]
struct ReleaseSessionBindings {
    session_id: String,
    claimed_at: DateTime<Utc>,
    claim_reason: String,
}

#[derive(Debug, Clone, SurrealValue)]
struct DurableClaimReceiptRow {
    id: RecordId,
    receipt_kind: String,
    previous_stopped_at: Option<DateTime<Utc>>,
    previous_stop_reason: Option<String>,
}

#[derive(Clone, SurrealValue)]
struct AbandonBindings {
    record: RecordId,
    claimed_at: DateTime<Utc>,
    claim_reason: String,
    previous_stopped_at: Option<DateTime<Utc>>,
    previous_stop_reason: Option<String>,
}

#[derive(Clone, SurrealValue)]
struct CanonicalReadBindings {
    record: RecordId,
}

#[derive(Debug, SurrealValue)]
struct CanonicalReclaimState {
    stopped_at: Option<DateTime<Utc>>,
    stop_reason: Option<String>,
    exit_code: Option<i64>,
}

#[derive(SurrealValue)]
struct MarkKilledBindings {
    process_uuid: Uuid,
    claimed_at: DateTime<Utc>,
    claim_reason: String,
    killed_reason: String,
}

#[async_trait]
impl ReclaimProcessStore for SurrealProcessLedgerStore {
    async fn active_processes_for_session(
        &self,
        session_id: &str,
    ) -> Result<Vec<ReclaimableProcess>, ProcessLedgerError> {
        self.active_processes_for_session_at_with_owner(
            session_id,
            Utc::now(),
            reclaim_boot_owner_id(),
        )
        .await
    }

    async fn mark_cleanup_completed(
        &self,
        process: &ReclaimableProcess,
    ) -> Result<(), ProcessLedgerError> {
        self.mark_reclaim_cleanup_completed(process).await
    }

    async fn abandon(&self, processes: &[ReclaimableProcess]) -> Result<(), ProcessLedgerError> {
        self.abandon_reclaim_claims(processes).await
    }
}

impl SurrealProcessLedgerStore {
    /// Atomically claims active rows and re-acquires only a sentinel owned by a
    /// different process boot. Explicit time and owner inputs make close/reopen
    /// and concurrent-owner proofs deterministic without sleeping.
    pub(crate) async fn active_processes_for_session_at_with_owner(
        &self,
        session_id: &str,
        claimed_at: DateTime<Utc>,
        owner_id: Uuid,
    ) -> Result<Vec<ReclaimableProcess>, ProcessLedgerError> {
        self.active_processes_for_session_at_with_owner_inner(
            session_id, claimed_at, owner_id, None,
        )
        .await
    }

    #[cfg(feature = "surreal-test-support")]
    pub(crate) async fn active_processes_for_session_with_raw_receipt_failure(
        &self,
        session_id: &str,
        claimed_at: DateTime<Utc>,
        owner_id: Uuid,
        corrupt_process_uuid: Uuid,
    ) -> Result<Vec<ReclaimableProcess>, ProcessLedgerError> {
        self.active_processes_for_session_at_with_owner_inner(
            session_id,
            claimed_at,
            owner_id,
            Some(corrupt_process_uuid),
        )
        .await
    }

    async fn active_processes_for_session_at_with_owner_inner(
        &self,
        session_id: &str,
        claimed_at: DateTime<Utc>,
        owner_id: Uuid,
        corrupt_raw_receipt_for: Option<Uuid>,
    ) -> Result<Vec<ReclaimableProcess>, ProcessLedgerError> {
        // MT-008: the claim (exclusion + stopped_at mutation) is atomic inside
        // the single UPDATE ... RETURN BEFORE statement.
        let bindings = ClaimBindings {
            session_id: session_id.to_owned(),
            claimed_at,
            claim_prefix: format!("{RECLAIM_CLAIM_STOP_REASON}:"),
            killed_prefix: format!("{RECLAIM_KILLED_STOP_REASON}:"),
            claim_reason: format!("{RECLAIM_CLAIM_STOP_REASON}:{owner_id}"),
            killed_reason: format!("{RECLAIM_KILLED_STOP_REASON}:{owner_id}"),
        };
        let retry_seed = Uuid::now_v7();
        let mut failed_attempt = 0;
        let mut raw_rows: Vec<SurrealDataValue> = loop {
            let attempt_bindings = bindings.clone();
            let outcome = self
                .storage()
                .with_data_operation(move |database| {
                    Box::pin(async move {
                        database
                            .query_values(SURREAL_ACTIVE_RECLAIM_CLAIM_QUERY, attempt_bindings)
                            .await
                    })
                })
                .await;
            match outcome {
                Ok(rows) => break rows,
                Err(error)
                    if failed_attempt + 1 < SURREAL_CLAIM_MAX_ATTEMPTS
                        && is_surreal_retryable_transaction_conflict(&error) =>
                {
                    tokio::time::sleep(surreal_claim_retry_delay(retry_seed, failed_attempt)).await;
                    failed_attempt += 1;
                }
                Err(error) => return Err(ProcessLedgerError::from(error)),
            }
        };

        #[cfg(feature = "surreal-test-support")]
        if let Some(process_uuid) = corrupt_raw_receipt_for {
            let raw_row = raw_rows
                .iter_mut()
                .find(|raw_row| {
                    let SurrealDataValue::Object(object) = raw_row else {
                        return false;
                    };
                    object
                        .get("process_uuid")
                        .cloned()
                        .and_then(|value| Uuid::from_value(value).ok())
                        == Some(process_uuid)
                })
                .ok_or_else(|| {
                    ProcessLedgerError::Store(format!(
                        "raw receipt failure injection target {process_uuid} was not claimed"
                    ))
                })?;
            *raw_row = SurrealDataValue::None;
        }
        #[cfg(not(feature = "surreal-test-support"))]
        let _ = corrupt_raw_receipt_for;

        // The PostgreSQL version mapped rows inside the claiming transaction so
        // that a mapping failure rolled the claim back and left the rows
        // reclaimable. The seam commits the claim before Rust sees the rows, so
        // that intent is preserved by an explicit compensating release rather
        // than by a rollback. A crash after the claim commit is recovered by
        // the different-process-boot branch in the same atomic claim query.
        // The release is precise (see its exact current-claim guard), and for a
        // re-acquired row it restores the prior sentinel rather than falsely
        // making that row active again.
        let receipts = match raw_rows
            .into_iter()
            .map(ClaimReceipt::from_raw)
            .collect::<Result<Vec<_>, _>>()
        {
            Ok(receipts) => receipts,
            Err(error) => {
                let release = self
                    .release_session_claim_from_durable_receipts(session_id, claimed_at, owner_id)
                    .await;
                return match release {
                    Ok(()) => Err(error),
                    Err(release_error) => Err(ProcessLedgerError::Store(format!(
                        "{error}; exact durable-receipt compensation also failed: {release_error}"
                    ))),
                };
            }
        };
        let same_owner: Vec<RecordId> = match self
            .storage()
            .with_data_operation({
                let bindings = bindings.clone();
                move |database| {
                    Box::pin(async move {
                        database
                            .query_values(SURREAL_ACTIVE_RECLAIM_SAME_OWNER_QUERY, bindings)
                            .await
                    })
                }
            })
            .await
        {
            Ok(rows) => rows,
            Err(read_error) => {
                let release = self.release_claim(&receipts, claimed_at, owner_id).await;
                return match release {
                    Ok(()) => Err(ProcessLedgerError::Store(format!(
                        "same-owner reclaim convergence read failed: {read_error}"
                    ))),
                    Err(release_error) => Err(ProcessLedgerError::Store(format!(
                        "same-owner reclaim convergence read failed: {read_error}; claim compensation failed: {release_error}"
                    ))),
                };
            }
        };
        let stale_same_owner = same_owner
            .into_iter()
            .filter(|record| !receipts.iter().any(|receipt| receipt.record == *record))
            .collect::<Vec<_>>();
        if !stale_same_owner.is_empty() {
            let error = ProcessLedgerError::Store(format!(
                "same-boot reclaim claims have not converged: {stale_same_owner:?}"
            ));
            return match self.release_claim(&receipts, claimed_at, owner_id).await {
                Ok(()) => Err(error),
                Err(release_error) => Err(ProcessLedgerError::Store(format!(
                    "{error}; current claim compensation failed: {release_error}"
                ))),
            };
        }
        let decoded = receipts
            .iter()
            .map(|receipt| {
                let effective_claimed_at = if receipt.previous_stop_reason.as_deref()
                    == Some(bindings.killed_reason.as_str())
                {
                    receipt.previous_stopped_at.ok_or_else(|| {
                        ProcessLedgerError::Store(format!(
                            "same-boot cleanup marker {:?} has no stopped_at guard",
                            receipt.record
                        ))
                    })?
                } else {
                    claimed_at
                };
                receipt.decode().and_then(|row| {
                    row.into_reclaimable(
                        session_id,
                        effective_claimed_at,
                        bindings.claim_reason.clone(),
                        bindings.killed_reason.clone(),
                        receipt.record.clone(),
                    )
                })
            })
            .collect::<Result<Vec<_>, _>>();
        match decoded {
            Ok(reclaimable) => Ok(reclaimable),
            Err(error) => {
                // The atomic statement claimed every returned row before any
                // Rust decoding began. A failure in any row must therefore
                // compensate the complete statement result, including rows
                // decoded before the malformed row.
                match self.release_claim(&receipts, claimed_at, owner_id).await {
                    Ok(()) => Err(error),
                    Err(release_error) => Err(ProcessLedgerError::Store(format!(
                        "{error}; claim compensation failed: {release_error}"
                    ))),
                }
            }
        }
    }

    async fn canonical_reclaim_state(
        &self,
        record: RecordId,
    ) -> Result<Option<CanonicalReclaimState>, ProcessLedgerError> {
        let bindings = CanonicalReadBindings {
            record: record.clone(),
        };
        let rows: Vec<CanonicalReclaimState> = self
            .storage()
            .with_data_operation(move |database| {
                Box::pin(async move {
                    database
                        .query_values(SURREAL_ACTIVE_RECLAIM_CANONICAL_READ_QUERY, bindings)
                        .await
                })
            })
            .await
            .map_err(ProcessLedgerError::from)?;
        match rows.len() {
            0 => Ok(None),
            1 => Ok(rows.into_iter().next()),
            count => Err(ProcessLedgerError::Store(format!(
                "canonical reclaim read returned {count} rows for {record:?}"
            ))),
        }
    }

    async fn verify_reclaim_release_converged(
        &self,
        record: RecordId,
        previous_stopped_at: Option<DateTime<Utc>>,
        previous_stop_reason: Option<&str>,
        current_claim_reason: &str,
    ) -> Result<(), ProcessLedgerError> {
        let Some(state) = self.canonical_reclaim_state(record.clone()).await? else {
            return Err(ProcessLedgerError::Store(format!(
                "canonical reclaim row {record:?} is missing"
            )));
        };
        if state.stopped_at == previous_stopped_at
            && state.stop_reason.as_deref() == previous_stop_reason
        {
            return Ok(());
        }
        // A current-boot sentinel is never convergence. In particular, it must
        // not make the next same-boot recovery's excluded claim look like a
        // successful zero-row recovery.
        if state.stop_reason.as_deref() == Some(current_claim_reason) {
            return Err(ProcessLedgerError::Store(format!(
                "{record:?} still carries current reclaim sentinel"
            )));
        }
        // A different durable stopped state proves that another writer has
        // advanced the row. Compensation must not overwrite that terminal or
        // post-cleanup state.
        if state.stopped_at.is_some() && (state.exit_code.is_some() || state.stop_reason.is_some())
        {
            return Ok(());
        }
        Err(ProcessLedgerError::Store(format!(
            "{record:?} did not converge to its pre-claim or a durable advanced state"
        )))
    }

    /// Puts rows this claim took back into the reclaimable pool.
    ///
    /// Transaction conflicts are retried with the same bounded jitter policy as
    /// the claim. Any exhausted/non-retryable failure is returned with the exact
    /// record id, so a failed compensation can never be mistaken for success.
    async fn release_claim(
        &self,
        receipts: &[ClaimReceipt],
        claimed_at: DateTime<Utc>,
        owner_id: Uuid,
    ) -> Result<(), ProcessLedgerError> {
        let mut failures = Vec::new();
        for receipt in receipts {
            let claim_reason = format!("{RECLAIM_CLAIM_STOP_REASON}:{owner_id}");
            let bindings = ReleaseBindings {
                record: receipt.record.clone(),
                claimed_at,
                previous_stopped_at: receipt.previous_stopped_at,
                stop_reason: receipt.previous_stop_reason.clone(),
                claim_reason: claim_reason.clone(),
            };
            let retry_seed = Uuid::now_v7();
            let mut failed_attempt = 0;
            loop {
                let attempt_bindings = bindings.clone();
                let outcome = self
                    .storage()
                    .with_data_operation(move |database| {
                        Box::pin(async move {
                            database
                                .execute_returning(
                                    SURREAL_ACTIVE_RECLAIM_RELEASE_QUERY,
                                    attempt_bindings,
                                )
                                .await
                        })
                    })
                    .await;
                match outcome {
                    Ok(1) => break,
                    Ok(0) => {
                        let convergence = self
                            .verify_reclaim_release_converged(
                                receipt.record.clone(),
                                receipt.previous_stopped_at,
                                receipt.previous_stop_reason.as_deref(),
                                &claim_reason,
                            )
                            .await;
                        match convergence {
                            Ok(()) => {}
                            Err(error) => failures.push(format!(
                                "{:?}: zero-row release did not converge: {error}",
                                receipt.record
                            )),
                        }
                        break;
                    }
                    Ok(count) => {
                        failures.push(format!(
                            "{:?} returned impossible affected-row count {count}",
                            receipt.record
                        ));
                        break;
                    }
                    Err(error)
                        if failed_attempt + 1 < SURREAL_CLAIM_MAX_ATTEMPTS
                            && is_surreal_retryable_transaction_conflict(&error) =>
                    {
                        tokio::time::sleep(surreal_claim_retry_delay(retry_seed, failed_attempt))
                            .await;
                        failed_attempt += 1;
                    }
                    Err(error) => {
                        let convergence = self
                            .verify_reclaim_release_converged(
                                receipt.record.clone(),
                                receipt.previous_stopped_at,
                                receipt.previous_stop_reason.as_deref(),
                                &claim_reason,
                            )
                            .await;
                        if let Err(convergence_error) = convergence {
                            failures.push(format!(
                                "{:?}: {error}; canonical convergence failed: {convergence_error}",
                                receipt.record
                            ));
                        }
                        break;
                    }
                }
            }
        }
        if failures.is_empty() {
            Ok(())
        } else {
            Err(ProcessLedgerError::Store(format!(
                "unreleased reclaim records: {}",
                failures.join(", ")
            )))
        }
    }

    /// Restores every row claimed by this invocation from the pre-claim receipt
    /// persisted atomically in that same row. If the receipt query or any
    /// receipt validation fails, the claims remain fail-closed; this path never
    /// substitutes `NONE/NONE` for an unknown prior state.
    async fn release_session_claim_from_durable_receipts(
        &self,
        session_id: &str,
        claimed_at: DateTime<Utc>,
        owner_id: Uuid,
    ) -> Result<(), ProcessLedgerError> {
        let bindings = ReleaseSessionBindings {
            session_id: session_id.to_owned(),
            claimed_at,
            claim_reason: format!("{RECLAIM_CLAIM_STOP_REASON}:{owner_id}"),
        };
        let durable_rows: Vec<DurableClaimReceiptRow> = self
            .storage()
            .with_data_operation({
                let bindings = bindings.clone();
                move |database| {
                    Box::pin(async move {
                        database
                            .query_values(SURREAL_ACTIVE_RECLAIM_DURABLE_RECEIPTS_QUERY, bindings)
                            .await
                    })
                }
            })
            .await
            .map_err(ProcessLedgerError::from)?;
        let mut receipts = Vec::with_capacity(durable_rows.len());
        for durable in durable_rows {
            if durable.receipt_kind != RECLAIM_DURABLE_RECEIPT_KIND {
                return Err(ProcessLedgerError::Store(format!(
                    "durable reclaim receipt for {:?} has unexpected kind {:?}",
                    durable.id, durable.receipt_kind
                )));
            }
            receipts.push(ClaimReceipt {
                record: durable.id,
                previous_stopped_at: durable.previous_stopped_at,
                previous_stop_reason: durable.previous_stop_reason,
                raw_row: SurrealDataValue::None,
            });
        }
        self.release_claim(&receipts, claimed_at, owner_id).await?;

        let outstanding: Vec<RecordId> = self
            .storage()
            .with_data_operation(move |database| {
                Box::pin(async move {
                    database
                        .query_values(SURREAL_ACTIVE_RECLAIM_OUTSTANDING_SESSION_QUERY, bindings)
                        .await
                })
            })
            .await
            .map_err(ProcessLedgerError::from)?;
        if outstanding.is_empty() {
            Ok(())
        } else {
            Err(ProcessLedgerError::Store(format!(
                "exact durable-receipt compensation left outstanding reclaim records: {outstanding:?}"
            )))
        }
    }

    /// Releases exact claims that could not be completed by an external kill or
    /// durable STOP. Exact timestamp + boot-owner guards prevent this recovery
    /// path from undoing a concurrent terminal event or a later boot's claim.
    pub(crate) async fn abandon_reclaim_claims(
        &self,
        processes: &[ReclaimableProcess],
    ) -> Result<(), ProcessLedgerError> {
        let owner_id = reclaim_boot_owner_id();
        let claim_reason = format!("{RECLAIM_CLAIM_STOP_REASON}:{owner_id}");
        let mut failures = Vec::new();
        for process in processes {
            let receipt = match process.reclaim_receipt() {
                Ok(receipt) if receipt.record_process_uuid == process.process_uuid => receipt,
                Ok(receipt) => {
                    failures.push(format!(
                        "kernel_process_lifecycle:{}: receipt identifies {}",
                        process.process_uuid, receipt.record_process_uuid
                    ));
                    continue;
                }
                Err(error) => {
                    failures.push(format!(
                        "kernel_process_lifecycle:{}: {error}",
                        process.process_uuid
                    ));
                    continue;
                }
            };
            let record = receipt.record.clone();
            let bindings = AbandonBindings {
                record: record.clone(),
                claimed_at: process.reclaim_claimed_at,
                claim_reason: claim_reason.clone(),
                previous_stopped_at: receipt.previous_stopped_at,
                previous_stop_reason: receipt.previous_stop_reason.clone(),
            };
            let retry_seed = Uuid::now_v7();
            let mut failed_attempt = 0;
            loop {
                let attempt_bindings = bindings.clone();
                let outcome = self
                    .storage()
                    .with_data_operation(move |database| {
                        Box::pin(async move {
                            database
                                .execute_returning(
                                    SURREAL_ACTIVE_RECLAIM_ABANDON_QUERY,
                                    attempt_bindings,
                                )
                                .await
                        })
                    })
                    .await;
                match outcome {
                    Ok(1) => break,
                    Ok(0) => {
                        if let Err(error) = self
                            .verify_reclaim_release_converged(
                                record.clone(),
                                receipt.previous_stopped_at,
                                receipt.previous_stop_reason.as_deref(),
                                &claim_reason,
                            )
                            .await
                        {
                            failures.push(format!(
                                "kernel_process_lifecycle:{}: zero-row abandon did not converge: {error}",
                                process.process_uuid
                            ));
                        }
                        break;
                    }
                    Ok(count) => {
                        failures.push(format!(
                            "kernel_process_lifecycle:{}: impossible affected-row count {count}",
                            process.process_uuid
                        ));
                        break;
                    }
                    Err(error)
                        if failed_attempt + 1 < SURREAL_CLAIM_MAX_ATTEMPTS
                            && is_surreal_retryable_transaction_conflict(&error) =>
                    {
                        tokio::time::sleep(surreal_claim_retry_delay(retry_seed, failed_attempt))
                            .await;
                        failed_attempt += 1;
                    }
                    Err(error) => {
                        if let Err(convergence_error) = self
                            .verify_reclaim_release_converged(
                                record.clone(),
                                receipt.previous_stopped_at,
                                receipt.previous_stop_reason.as_deref(),
                                &claim_reason,
                            )
                            .await
                        {
                            failures.push(format!(
                                "kernel_process_lifecycle:{}: {error}; canonical convergence failed: {convergence_error}",
                                process.process_uuid
                            ));
                        }
                        break;
                    }
                }
            }
        }
        if failures.is_empty() {
            Ok(())
        } else {
            Err(ProcessLedgerError::Store(format!(
                "unabandoned reclaim records: {}",
                failures.join(", ")
            )))
        }
    }

    /// Persist that external cleanup has succeeded before attempting the final
    /// STOP. A later boot can then finish the ledger transition without killing
    /// the same external identity again.
    pub(crate) async fn mark_reclaim_cleanup_completed(
        &self,
        process: &ReclaimableProcess,
    ) -> Result<(), ProcessLedgerError> {
        let owner_id = reclaim_boot_owner_id();
        let bindings = MarkKilledBindings {
            process_uuid: process.process_uuid,
            claimed_at: process.reclaim_claimed_at,
            claim_reason: format!("{RECLAIM_CLAIM_STOP_REASON}:{owner_id}"),
            killed_reason: format!("{RECLAIM_KILLED_STOP_REASON}:{owner_id}"),
        };
        let updated: Vec<Uuid> = self
            .storage()
            .with_data_operation(move |database| {
                Box::pin(async move {
                    database
                        .query_values(SURREAL_ACTIVE_RECLAIM_MARK_KILLED_QUERY, bindings)
                        .await
                })
            })
            .await
            .map_err(ProcessLedgerError::from)?;
        if updated == [process.process_uuid] {
            Ok(())
        } else {
            Err(ProcessLedgerError::Store(format!(
                "exact reclaim cleanup marker was not persisted for {}",
                process.process_uuid
            )))
        }
    }
}

fn os_pid_to_u32(value: i64) -> Result<u32, ProcessLedgerError> {
    u32::try_from(value)
        .map_err(|_| ProcessLedgerError::Store(format!("invalid os_pid in reclaim query: {value}")))
}

#[async_trait]
pub trait StaleSessionSource: Send + Sync + 'static {
    async fn stale_sessions(&self, ttl: Duration) -> Result<Vec<String>, ProcessLedgerError>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StalenessReclaimConfig {
    pub ttl: Duration,
    pub scan_interval: Duration,
}

impl StalenessReclaimConfig {
    pub fn normalized(self) -> Self {
        Self {
            ttl: if self.ttl.is_zero() {
                Duration::from_secs(300)
            } else {
                self.ttl
            },
            scan_interval: if self.scan_interval.is_zero() {
                Duration::from_secs(30)
            } else {
                self.scan_interval
            },
        }
    }
}

impl Default for StalenessReclaimConfig {
    fn default() -> Self {
        Self {
            ttl: Duration::from_secs(300),
            scan_interval: Duration::from_secs(30),
        }
    }
}

pub fn spawn_staleness_reclaim_task(
    reclaim: Arc<Reclaim>,
    stale_source: Arc<dyn StaleSessionSource>,
    config: StalenessReclaimConfig,
) -> JoinHandle<()> {
    let config = config.normalized();
    tokio::spawn(async move {
        let mut interval = time::interval(config.scan_interval);
        interval.set_missed_tick_behavior(MissedTickBehavior::Delay);
        loop {
            interval.tick().await;
            if let Ok(session_ids) = stale_source.stale_sessions(config.ttl).await {
                for session_id in session_ids {
                    let _ = reclaim.run(&session_id, ReclaimTrigger::Stale).await;
                }
            }
        }
    })
}
