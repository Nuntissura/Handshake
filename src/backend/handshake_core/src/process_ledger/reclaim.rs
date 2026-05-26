use std::{sync::Arc, time::Duration};

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::Row;
use thiserror::Error;
use tokio::{
    task::JoinHandle,
    time::{self, MissedTickBehavior},
};
use uuid::Uuid;

use super::{
    LedgerEventKind, PostgresProcessLedgerStore, ProcessEngineKind, ProcessLedgerError,
    ProcessLedgerWriter, ProcessStop,
};

pub const POSTGRES_ACTIVE_RECLAIM_QUERY_SQL: &str = r#"
SELECT
    process_uuid::text AS process_uuid,
    os_pid,
    parent_session_id,
    parent_process_id::text AS parent_process_id,
    sandbox_adapter_id,
    sandbox_internal_id,
    engine_kind,
    started_at,
    model_artifact_sha256,
    work_profile_id,
    owner_role,
    owner_wp,
    role_id,
    wp_id,
    mt_id,
    sandbox_capabilities_snapshot::text AS sandbox_capabilities_snapshot,
    metadata_jsonb::text AS metadata_jsonb
FROM kernel_process_lifecycle
WHERE parent_session_id = $1
  AND stopped_at IS NULL
FOR UPDATE
"#;

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

#[async_trait]
impl ReclaimProcessStore for PostgresProcessLedgerStore {
    async fn active_processes_for_session(
        &self,
        session_id: &str,
    ) -> Result<Vec<ReclaimableProcess>, ProcessLedgerError> {
        let mut tx = self.pool().begin().await?;
        let rows = sqlx::query(POSTGRES_ACTIVE_RECLAIM_QUERY_SQL)
            .bind(session_id)
            .fetch_all(&mut *tx)
            .await?;
        tx.commit().await?;

        rows.into_iter()
            .map(|row| {
                let process_uuid_raw: String = row.get("process_uuid");
                let engine_kind_raw: String = row.get("engine_kind");
                Ok(ReclaimableProcess {
                    process_uuid: Uuid::parse_str(&process_uuid_raw).map_err(|error| {
                        ProcessLedgerError::Store(format!(
                            "invalid process_uuid in reclaim query: {error}"
                        ))
                    })?,
                    os_pid: row
                        .try_get::<Option<i64>, _>("os_pid")
                        .map_err(ProcessLedgerError::from)?
                        .map(pg_pid_to_u32)
                        .transpose()?,
                    parent_session_id: row.get("parent_session_id"),
                    parent_process_id: row
                        .try_get::<Option<String>, _>("parent_process_id")
                        .map_err(ProcessLedgerError::from)?
                        .map(|raw| {
                            Uuid::parse_str(&raw).map_err(|error| {
                                ProcessLedgerError::Store(format!(
                                    "invalid parent_process_id in reclaim query: {error}"
                                ))
                            })
                        })
                        .transpose()?,
                    sandbox_adapter_id: row.get("sandbox_adapter_id"),
                    sandbox_internal_id: row.get("sandbox_internal_id"),
                    engine_kind: ProcessEngineKind::try_from(engine_kind_raw.as_str())
                        .map_err(ProcessLedgerError::Store)?,
                    started_at: row.get("started_at"),
                    model_artifact_sha256: row.get("model_artifact_sha256"),
                    work_profile_id: row.get("work_profile_id"),
                    owner_role: row.get("owner_role"),
                    owner_wp: row.get("owner_wp"),
                    role_id: row.get("role_id"),
                    wp_id: row.get("wp_id"),
                    mt_id: row.get("mt_id"),
                    sandbox_capabilities_snapshot: json_text_column(
                        &row,
                        "sandbox_capabilities_snapshot",
                    )?,
                    metadata_jsonb: json_text_column(&row, "metadata_jsonb")?,
                })
            })
            .collect()
    }
}

fn json_text_column(
    row: &sqlx::postgres::PgRow,
    column: &str,
) -> Result<serde_json::Value, ProcessLedgerError> {
    let raw = row
        .try_get::<Option<String>, _>(column)
        .map_err(ProcessLedgerError::from)?
        .unwrap_or_else(|| "{}".to_string());
    serde_json::from_str(&raw).map_err(|error| {
        ProcessLedgerError::Store(format!("invalid JSONB column {column}: {error}"))
    })
}

fn pg_pid_to_u32(value: i64) -> Result<u32, ProcessLedgerError> {
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
