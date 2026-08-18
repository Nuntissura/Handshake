use std::{sync::Arc, time::Duration};

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use surrealdb::types::SurrealValue;
use thiserror::Error;
use tokio::{
    task::JoinHandle,
    time::{self, MissedTickBehavior},
};
use uuid::Uuid;

use super::{
    LedgerEventKind, ProcessEngineKind, ProcessLedgerError, ProcessLedgerWriter, ProcessStop,
    SurrealProcessLedgerStore,
};

/// Sentinel written into `stop_reason` when the claim finds the column empty.
///
/// It is also the marker the follow-up STOP upsert overwrites, and — together
/// with the bound claim timestamp — the precise guard the compensating
/// un-claim uses.
const RECLAIM_CLAIM_STOP_REASON: &str = "reclaim_claimed";

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
///
/// The claim is durable the instant the statement commits; the kill outcome /
/// exit_code is refined afterward by the normal STOP upsert, which overwrites
/// the sentinel `stopped_at` with the real stop time.
pub const SURREAL_ACTIVE_RECLAIM_CLAIM_QUERY: &str = "UPDATE kernel_process_lifecycle SET \
     stopped_at = $claimed_at, \
     stop_reason = IF stop_reason = NONE { $claim_reason } ELSE { stop_reason } \
     WHERE parent_session_id = $session_id AND stopped_at = NONE RETURN BEFORE;";

/// Releases one row claimed by [`SURREAL_ACTIVE_RECLAIM_CLAIM_QUERY`].
///
/// `stopped_at = $claimed_at` is the whole safety of this statement: it matches
/// only a row still carrying the exact sentinel this claim wrote. A row whose
/// STOP has since landed has a different `stopped_at` and is skipped.
const SURREAL_ACTIVE_RECLAIM_RELEASE_QUERY: &str = "UPDATE kernel_process_lifecycle SET \
     stopped_at = NONE, stop_reason = $stop_reason \
     WHERE process_uuid = $process_uuid AND stopped_at = $claimed_at RETURN AFTER;";

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
}

impl ReclaimableProcess {
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
            metadata_jsonb: self.metadata_jsonb.clone(),
        }
    }
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
}

pub trait SandboxKill: Send + Sync + 'static {
    fn kill(&self, process_uuid: Uuid) -> Result<(), KillError>;
}

pub trait ReclaimStopWriter: Send + Sync + 'static {
    fn append_reclaim_stop(&self, stop: ProcessStop) -> Result<(), ProcessLedgerError>;
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

    pub async fn run(
        &self,
        session_id: &str,
        trigger: ReclaimTrigger,
    ) -> Result<ReclaimReport, ProcessLedgerError> {
        let started = std::time::Instant::now();
        let active = self.store.active_processes_for_session(session_id).await?;
        let mut reclaimed = Vec::with_capacity(active.len());

        for process in active {
            let kill_result = match self.sandbox_kill.kill(process.process_uuid) {
                Ok(()) => KillOutcome::Killed,
                Err(error) => KillOutcome::Failed {
                    error: error.message().to_string(),
                },
            };
            self.stop_writer
                .append_reclaim_stop(process.reclaim_stop(-1))?;
            reclaimed.push(ReclaimedProcess {
                process_uuid: process.process_uuid,
                engine_kind: process.engine_kind,
                sandbox_adapter_id: process.sandbox_adapter_id,
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

impl ReclaimStopWriter for ProcessLedgerWriter {
    fn append_reclaim_stop(&self, stop: ProcessStop) -> Result<(), ProcessLedgerError> {
        self.append_stop(stop)
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

impl ClaimedRow {
    fn into_reclaimable(self, session_id: &str) -> Result<ReclaimableProcess, ProcessLedgerError> {
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
            metadata_jsonb: self.metadata,
        })
    }
}

#[derive(SurrealValue)]
struct ClaimBindings {
    session_id: String,
    claimed_at: DateTime<Utc>,
    claim_reason: String,
}

#[derive(SurrealValue)]
struct ReleaseBindings {
    process_uuid: Uuid,
    claimed_at: DateTime<Utc>,
    stop_reason: Option<String>,
}

#[async_trait]
impl ReclaimProcessStore for SurrealProcessLedgerStore {
    async fn active_processes_for_session(
        &self,
        session_id: &str,
    ) -> Result<Vec<ReclaimableProcess>, ProcessLedgerError> {
        // MT-008: the claim (exclusion + stopped_at mutation) is atomic inside
        // the single UPDATE ... RETURN BEFORE statement.
        let claimed_at = Utc::now();
        let bindings = ClaimBindings {
            session_id: session_id.to_owned(),
            claimed_at,
            claim_reason: RECLAIM_CLAIM_STOP_REASON.to_owned(),
        };
        let rows: Vec<ClaimedRow> = self
            .storage()
            .with_data_operation(move |database| {
                Box::pin(async move {
                    database
                        .query_values(SURREAL_ACTIVE_RECLAIM_CLAIM_QUERY, bindings)
                        .await
                })
            })
            .await
            .map_err(ProcessLedgerError::from)?;

        // The PostgreSQL version mapped rows inside the claiming transaction so
        // that a mapping failure rolled the claim back and left the rows
        // reclaimable. The seam commits the claim before Rust sees the rows, so
        // that intent is preserved by an explicit compensating release rather
        // than by a rollback. DISCLOSED NARROWING: a crash between the claim
        // commit and the release leaves the affected rows claimed-but-not-killed
        // instead of reclaimable. The release is precise (see the guard on
        // `SURREAL_ACTIVE_RECLAIM_RELEASE_QUERY`) and the two remaining fallible
        // conversions are both schema-constrained, so this path is not expected
        // to run at all.
        let mut reclaimable = Vec::with_capacity(rows.len());
        for (index, row) in rows.iter().enumerate() {
            match row.clone().into_reclaimable(session_id) {
                Ok(process) => reclaimable.push(process),
                Err(error) => {
                    self.release_claim(&rows[index..], claimed_at).await;
                    return Err(error);
                }
            }
        }
        Ok(reclaimable)
    }
}

impl SurrealProcessLedgerStore {
    /// Puts rows this claim took back into the reclaimable pool.
    ///
    /// Failures are swallowed deliberately: the caller is already returning the
    /// mapping error that triggered the release, and a release failure must not
    /// mask it. A row that fails to release stays claimed and is reported by the
    /// next staleness scan rather than being silently lost.
    async fn release_claim(&self, rows: &[ClaimedRow], claimed_at: DateTime<Utc>) {
        for row in rows {
            let bindings = ReleaseBindings {
                process_uuid: row.process_uuid,
                claimed_at,
                stop_reason: row.stop_reason.clone(),
            };
            let _ = self
                .storage()
                .with_data_operation(move |database| {
                    Box::pin(async move {
                        database
                            .execute_returning(SURREAL_ACTIVE_RECLAIM_RELEASE_QUERY, bindings)
                            .await
                    })
                })
                .await;
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
