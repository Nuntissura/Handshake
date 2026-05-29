//! MT-184 MicroTaskQueue — Postgres-backed atomic claim with SKIP LOCKED.

use chrono::Utc;
use sqlx::PgPool;
use std::path::PathBuf;
use thiserror::Error;
use uuid::Uuid;

use super::job::{EscalationStep, EscalationTier, MicroTaskJob, MicroTaskJobId, MicroTaskJobState};

#[derive(Debug, Error)]
pub enum QueueError {
    #[error("sqlx error: {0}")]
    Sqlx(#[from] sqlx::Error),
    #[error("serde error: {0}")]
    Serde(#[from] serde_json::Error),
    #[error("invalid escalation transition")]
    InvalidEscalation,
    #[error("job not found")]
    NotFound,
    #[error("hardgated job rejects further claims")]
    HardGated,
    #[error("HardGate transition requires a decision_request mailbox receipt")]
    HardGateMailboxRequired,
}

pub struct MicroTaskQueue {
    pool: PgPool,
}

impl MicroTaskQueue {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    pub async fn ensure_schema(&self) -> Result<(), QueueError> {
        sqlx::raw_sql(SCHEMA_SQL).execute(&self.pool).await?;
        Ok(())
    }

    pub async fn enqueue(&self, job: &MicroTaskJob) -> Result<(), QueueError> {
        sqlx::query(
            r#"INSERT INTO kernel_micro_task_job
               (job_id, wp_id, mt_id, mt_contract_path, iteration_n, max_iterations,
                escalation_tier, escalation_history, task_tags, lora_id,
                mailbox_thread_id, state, claimed_by_session, claimed_at_utc,
                created_at_utc, updated_at_utc)
               VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16)"#,
        )
        .bind(job.job_id.as_uuid())
        .bind(&job.wp_id)
        .bind(&job.mt_id)
        .bind(job.mt_contract_path.to_string_lossy().to_string())
        .bind(job.iteration_n as i64)
        .bind(job.max_iterations as i64)
        .bind(job.escalation_tier.as_str())
        .bind(serde_json::to_value(&job.escalation_history)?)
        .bind(serde_json::to_value(&job.task_tags)?)
        .bind(job.lora_id.as_deref())
        .bind(job.mailbox_thread_id)
        .bind(job.state.as_str())
        .bind(job.claimed_by_session)
        .bind(job.claimed_at_utc)
        .bind(job.created_at_utc)
        .bind(job.updated_at_utc)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Atomically claim the next queued job. Uses `SELECT ... FOR UPDATE SKIP
    /// LOCKED LIMIT 1` so parallel claimers do not race.
    pub async fn claim_next(&self, session_id: Uuid) -> Result<Option<MicroTaskJobId>, QueueError> {
        let mut tx = self.pool.begin().await?;
        let row: Option<(Uuid,)> = sqlx::query_as(
            r#"SELECT job_id FROM kernel_micro_task_job
               WHERE state = 'queued'
               ORDER BY created_at_utc ASC
               FOR UPDATE SKIP LOCKED LIMIT 1"#,
        )
        .fetch_optional(&mut *tx)
        .await?;
        let Some((id,)) = row else {
            tx.commit().await?;
            return Ok(None);
        };
        sqlx::query(
            r#"UPDATE kernel_micro_task_job
               SET state = 'claimed', claimed_by_session = $1, claimed_at_utc = $2, updated_at_utc = $2
               WHERE job_id = $3"#,
        )
        .bind(session_id)
        .bind(Utc::now())
        .bind(id)
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(Some(MicroTaskJobId(id)))
    }

    pub async fn update_state(
        &self,
        job_id: MicroTaskJobId,
        new_state: MicroTaskJobState,
        transition_reason: Option<String>,
    ) -> Result<(), QueueError> {
        let mut tx = self.pool.begin().await?;
        let row: Option<(String,)> = sqlx::query_as(
            r#"SELECT state FROM kernel_micro_task_job WHERE job_id = $1 FOR UPDATE"#,
        )
        .bind(job_id.as_uuid())
        .fetch_optional(&mut *tx)
        .await?;
        let Some((current,)) = row else {
            return Err(QueueError::NotFound);
        };
        if current == "hard_gated" && new_state != MicroTaskJobState::HardGated {
            return Err(QueueError::HardGated);
        }
        let now = Utc::now();
        sqlx::query(
            r#"UPDATE kernel_micro_task_job
               SET state = $1, updated_at_utc = $2,
                   transition_reason = COALESCE($3, transition_reason)
               WHERE job_id = $4"#,
        )
        .bind(new_state.as_str())
        .bind(now)
        .bind(transition_reason)
        .bind(job_id.as_uuid())
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(())
    }

    pub async fn escalate(
        &self,
        job_id: MicroTaskJobId,
        new_tier: EscalationTier,
        reason: String,
    ) -> Result<EscalationStep, QueueError> {
        self.escalate_inner(job_id, new_tier, reason, None, None)
            .await
    }

    pub async fn escalate_with_lora(
        &self,
        job_id: MicroTaskJobId,
        new_tier: EscalationTier,
        reason: String,
        lora_id: Option<String>,
    ) -> Result<EscalationStep, QueueError> {
        self.escalate_inner(job_id, new_tier, reason, lora_id, None)
            .await
    }

    pub async fn hard_gate_after_mailbox_post(
        &self,
        job_id: MicroTaskJobId,
        reason: String,
        decision_request_message_id: Uuid,
    ) -> Result<EscalationStep, QueueError> {
        self.escalate_inner(
            job_id,
            EscalationTier::HardGate,
            reason,
            None,
            Some(decision_request_message_id),
        )
        .await
    }

    async fn escalate_inner(
        &self,
        job_id: MicroTaskJobId,
        new_tier: EscalationTier,
        reason: String,
        lora_id: Option<String>,
        hardgate_decision_request_message_id: Option<Uuid>,
    ) -> Result<EscalationStep, QueueError> {
        let mut tx = self.pool.begin().await?;
        let row: Option<(String, serde_json::Value)> = sqlx::query_as(
            r#"SELECT escalation_tier, escalation_history FROM kernel_micro_task_job
               WHERE job_id = $1 FOR UPDATE"#,
        )
        .bind(job_id.as_uuid())
        .fetch_optional(&mut *tx)
        .await?;
        let Some((current_tier_s, history)) = row else {
            return Err(QueueError::NotFound);
        };
        let current_tier: EscalationTier =
            serde_json::from_value(serde_json::Value::String(current_tier_s))?;
        // Monotonic: new tier must equal current.next() unless re-issuing same.
        let allowed = match current_tier.next() {
            Some(n) => n == new_tier,
            None => false,
        };
        if !allowed {
            return Err(QueueError::InvalidEscalation);
        }
        if matches!(new_tier, EscalationTier::HardGate)
            && hardgate_decision_request_message_id.is_none()
        {
            return Err(QueueError::HardGateMailboxRequired);
        }
        let step = EscalationStep {
            from_tier: current_tier,
            to_tier: new_tier,
            reason: reason.clone(),
            recorded_at_utc: Utc::now(),
        };
        let mut history_vec: Vec<EscalationStep> = serde_json::from_value(history)?;
        history_vec.push(step.clone());
        let new_state = if matches!(new_tier, EscalationTier::HardGate) {
            "hard_gated"
        } else {
            "escalated"
        };
        let transition_reason = hardgate_decision_request_message_id
            .map(|message_id| format!("{reason}; decision_request_message_id={message_id}"))
            .unwrap_or_else(|| reason.clone());
        sqlx::query(
            r#"UPDATE kernel_micro_task_job
               SET escalation_tier = $1, escalation_history = $2, state = $3,
                   lora_id = COALESCE($4, lora_id),
                   transition_reason = $5,
                   updated_at_utc = $6
               WHERE job_id = $7"#,
        )
        .bind(new_tier.as_str())
        .bind(serde_json::to_value(&history_vec)?)
        .bind(new_state)
        .bind(lora_id.as_deref())
        .bind(transition_reason)
        .bind(Utc::now())
        .bind(job_id.as_uuid())
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(step)
    }

    pub async fn get_state(
        &self,
        job_id: MicroTaskJobId,
    ) -> Result<Option<MicroTaskJobState>, QueueError> {
        let row: Option<(String,)> =
            sqlx::query_as("SELECT state FROM kernel_micro_task_job WHERE job_id = $1")
                .bind(job_id.as_uuid())
                .fetch_optional(&self.pool)
                .await?;
        let Some((s,)) = row else { return Ok(None) };
        let st: MicroTaskJobState = serde_json::from_value(serde_json::Value::String(s))?;
        Ok(Some(st))
    }

    pub async fn get_job(
        &self,
        job_id: MicroTaskJobId,
    ) -> Result<Option<MicroTaskJob>, QueueError> {
        let row: Option<(
            Uuid,
            String,
            String,
            String,
            i64,
            i64,
            String,
            serde_json::Value,
            serde_json::Value,
            Option<String>,
            Option<Uuid>,
            String,
            Option<Uuid>,
            Option<chrono::DateTime<chrono::Utc>>,
            chrono::DateTime<chrono::Utc>,
            chrono::DateTime<chrono::Utc>,
        )> = sqlx::query_as(
            r#"SELECT job_id, wp_id, mt_id, mt_contract_path, iteration_n, max_iterations,
                       escalation_tier, escalation_history, task_tags, lora_id,
                       mailbox_thread_id, state, claimed_by_session, claimed_at_utc,
                       created_at_utc, updated_at_utc
               FROM kernel_micro_task_job WHERE job_id = $1"#,
        )
        .bind(job_id.as_uuid())
        .fetch_optional(&self.pool)
        .await?;
        let Some(r) = row else { return Ok(None) };
        let tier: EscalationTier = serde_json::from_value(serde_json::Value::String(r.6))?;
        let history: Vec<EscalationStep> = serde_json::from_value(r.7)?;
        let task_tags: Vec<String> = serde_json::from_value(r.8)?;
        let state: MicroTaskJobState = serde_json::from_value(serde_json::Value::String(r.11))?;
        Ok(Some(MicroTaskJob {
            job_id: MicroTaskJobId(r.0),
            wp_id: r.1,
            mt_id: r.2,
            mt_contract_path: PathBuf::from(r.3),
            iteration_n: r.4 as u32,
            max_iterations: r.5 as u32,
            escalation_tier: tier,
            escalation_history: history,
            task_tags,
            lora_id: r.9,
            mailbox_thread_id: r.10,
            state,
            claimed_by_session: r.12,
            claimed_at_utc: r.13,
            created_at_utc: r.14,
            updated_at_utc: r.15,
            completion_signal: None,
            progress_artifact_ref: None,
            run_ledger_ref: None,
        }))
    }
}

pub const SCHEMA_SQL: &str = include_str!("../../migrations/0023_micro_task_job_queue.sql");
