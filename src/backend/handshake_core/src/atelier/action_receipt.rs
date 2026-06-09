//! Generic model-visible action receipts (WP-KERNEL-005 MT-139).
//!
//! This schema records every model-visible operation by action id, params hash,
//! actor/session, timing, status, and refs. Raw params are never persisted.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use sqlx::Row;
use uuid::Uuid;

use crate::kernel::action_catalog::kernel002_action_catalog;

use super::{reject_legacy_runtime_ref, AtelierError, AtelierResult, AtelierStore};

pub mod action_receipt_event_family {
    pub const ACTION_RECEIPT_RECORDED: &str = "atelier.action_receipt.recorded";

    pub const ALL: &[&str] = &[ACTION_RECEIPT_RECORDED];
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ActionReceiptStatus {
    Succeeded,
    Failed,
    Rejected,
}

impl ActionReceiptStatus {
    pub fn as_token(self) -> &'static str {
        match self {
            ActionReceiptStatus::Succeeded => "succeeded",
            ActionReceiptStatus::Failed => "failed",
            ActionReceiptStatus::Rejected => "rejected",
        }
    }

    pub fn from_token(token: &str) -> AtelierResult<Self> {
        match token {
            "succeeded" => Ok(ActionReceiptStatus::Succeeded),
            "failed" => Ok(ActionReceiptStatus::Failed),
            "rejected" => Ok(ActionReceiptStatus::Rejected),
            other => Err(AtelierError::Validation(format!(
                "unknown action receipt status: {other}"
            ))),
        }
    }
}

#[derive(Clone, Debug)]
pub struct NewActionReceipt {
    pub action_id: String,
    pub actor_kind: String,
    pub actor_id: String,
    pub session_id: String,
    pub params: Value,
    pub started_at_utc: DateTime<Utc>,
    pub completed_at_utc: DateTime<Utc>,
    pub status: ActionReceiptStatus,
    pub target_refs: Vec<String>,
    pub evidence_refs: Vec<String>,
    pub result_refs: Vec<String>,
    pub error_class: Option<String>,
    pub recovery_hint: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ActionReceipt {
    pub receipt_id: Uuid,
    pub action_id: String,
    pub params_sha256: String,
    pub actor_kind: String,
    pub actor_id: String,
    pub session_id: String,
    pub started_at_utc: DateTime<Utc>,
    pub completed_at_utc: DateTime<Utc>,
    pub status: ActionReceiptStatus,
    pub target_refs: Vec<String>,
    pub evidence_refs: Vec<String>,
    pub result_refs: Vec<String>,
    pub error_class: Option<String>,
    pub recovery_hint: Option<String>,
    pub created_at_utc: DateTime<Utc>,
}

impl AtelierStore {
    pub async fn record_action_receipt(
        &self,
        input: &NewActionReceipt,
    ) -> AtelierResult<ActionReceipt> {
        validate_action_receipt(input)?;
        let params_sha256 = params_sha256(&input.params)?;
        let receipt_id = Uuid::now_v7();

        let target_refs = serde_json::to_value(&input.target_refs)
            .map_err(|err| AtelierError::Validation(err.to_string()))?;
        let evidence_refs = serde_json::to_value(&input.evidence_refs)
            .map_err(|err| AtelierError::Validation(err.to_string()))?;
        let result_refs = serde_json::to_value(&input.result_refs)
            .map_err(|err| AtelierError::Validation(err.to_string()))?;

        let mut tx = self.pool().begin().await?;
        let row = sqlx::query(
            r#"INSERT INTO atelier_action_receipt (
                   receipt_id, action_id, params_sha256, actor_kind, actor_id,
                   session_id, started_at_utc, completed_at_utc, status,
                   target_refs, evidence_refs, result_refs, error_class,
                   recovery_hint
               )
               VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9,
                       $10::jsonb, $11::jsonb, $12::jsonb, $13, $14)
               RETURNING receipt_id, action_id, params_sha256, actor_kind,
                         actor_id, session_id, started_at_utc, completed_at_utc,
                         status, target_refs, evidence_refs, result_refs,
                         error_class, recovery_hint, created_at_utc"#,
        )
        .bind(receipt_id)
        .bind(&input.action_id)
        .bind(&params_sha256)
        .bind(&input.actor_kind)
        .bind(&input.actor_id)
        .bind(&input.session_id)
        .bind(input.started_at_utc)
        .bind(input.completed_at_utc)
        .bind(input.status.as_token())
        .bind(target_refs)
        .bind(evidence_refs)
        .bind(result_refs)
        .bind(input.error_class.as_deref())
        .bind(input.recovery_hint.as_deref())
        .fetch_one(&mut *tx)
        .await?;
        let receipt = action_receipt_from_row(&row)?;

        self.record_event_in_tx(
            &mut tx,
            action_receipt_event_family::ACTION_RECEIPT_RECORDED,
            "atelier_action_receipt",
            &receipt.receipt_id.to_string(),
            serde_json::json!({
                "receipt_id": receipt.receipt_id,
                "action_id": receipt.action_id,
                "params_sha256": receipt.params_sha256,
                "actor_kind": receipt.actor_kind,
                "actor_id": receipt.actor_id,
                "session_id": receipt.session_id,
                "started_at_utc": receipt.started_at_utc,
                "completed_at_utc": receipt.completed_at_utc,
                "status": receipt.status.as_token(),
                "target_refs": receipt.target_refs,
                "evidence_refs": receipt.evidence_refs,
                "result_refs": receipt.result_refs,
                "error_class": receipt.error_class,
                "recovery_hint": receipt.recovery_hint,
                "schema": "hsk.atelier.action_receipt@1",
            }),
        )
        .await?;
        tx.commit().await?;
        Ok(receipt)
    }

    pub async fn get_action_receipt(&self, receipt_id: Uuid) -> AtelierResult<ActionReceipt> {
        let row = sqlx::query(
            r#"SELECT receipt_id, action_id, params_sha256, actor_kind,
                      actor_id, session_id, started_at_utc, completed_at_utc,
                      status, target_refs, evidence_refs, result_refs,
                      error_class, recovery_hint, created_at_utc
               FROM atelier_action_receipt
               WHERE receipt_id = $1"#,
        )
        .bind(receipt_id)
        .fetch_optional(self.pool())
        .await?;

        match row {
            Some(row) => action_receipt_from_row(&row),
            None => Err(AtelierError::NotFound(format!(
                "action receipt_id={receipt_id}"
            ))),
        }
    }
}

fn validate_action_receipt(input: &NewActionReceipt) -> AtelierResult<()> {
    validate_token("action_id", &input.action_id)?;
    validate_token("actor_kind", &input.actor_kind)?;
    validate_token("actor_id", &input.actor_id)?;
    validate_token("session_id", &input.session_id)?;
    if input.completed_at_utc < input.started_at_utc {
        return Err(AtelierError::Validation(
            "completed_at_utc must be >= started_at_utc".into(),
        ));
    }
    if kernel002_action_catalog()
        .action(&input.action_id)
        .is_none()
    {
        return Err(AtelierError::Validation(format!(
            "unknown model-visible action_id {}",
            input.action_id
        )));
    }
    validate_ref_list("target_refs", &input.target_refs)?;
    validate_ref_list("evidence_refs", &input.evidence_refs)?;
    validate_ref_list("result_refs", &input.result_refs)?;
    if input.status != ActionReceiptStatus::Succeeded {
        let missing_fields = [
            ("error_class", input.error_class.as_deref()),
            ("recovery_hint", input.recovery_hint.as_deref()),
        ]
        .into_iter()
        .filter_map(|(field, value)| match value {
            Some(value) if !value.trim().is_empty() && value == value.trim() => None,
            _ => Some(field),
        })
        .collect::<Vec<_>>();
        if !missing_fields.is_empty() {
            return Err(AtelierError::Validation(format!(
                "{} required for non-succeeded action receipts",
                missing_fields.join(", ")
            )));
        }
    }
    Ok(())
}

fn action_receipt_from_row(row: &sqlx::postgres::PgRow) -> AtelierResult<ActionReceipt> {
    let status: String = row.get("status");
    Ok(ActionReceipt {
        receipt_id: row.get("receipt_id"),
        action_id: row.get("action_id"),
        params_sha256: row.get("params_sha256"),
        actor_kind: row.get("actor_kind"),
        actor_id: row.get("actor_id"),
        session_id: row.get("session_id"),
        started_at_utc: row.get("started_at_utc"),
        completed_at_utc: row.get("completed_at_utc"),
        status: ActionReceiptStatus::from_token(&status)?,
        target_refs: jsonb_string_array(row.get("target_refs"))?,
        evidence_refs: jsonb_string_array(row.get("evidence_refs"))?,
        result_refs: jsonb_string_array(row.get("result_refs"))?,
        error_class: row.get("error_class"),
        recovery_hint: row.get("recovery_hint"),
        created_at_utc: row.get("created_at_utc"),
    })
}

fn params_sha256(params: &Value) -> AtelierResult<String> {
    let bytes =
        serde_json::to_vec(params).map_err(|err| AtelierError::Validation(err.to_string()))?;
    Ok(format!("sha256:{}", hex::encode(Sha256::digest(&bytes))))
}

fn validate_token(field: &str, value: &str) -> AtelierResult<()> {
    if value.trim().is_empty() || value.trim() != value {
        return Err(AtelierError::Validation(format!(
            "{field} must not be empty or padded"
        )));
    }
    Ok(())
}

fn validate_ref_list(field: &str, values: &[String]) -> AtelierResult<()> {
    if values.is_empty() {
        return Err(AtelierError::Validation(format!(
            "{field} must include at least one ref"
        )));
    }
    for value in values {
        reject_legacy_runtime_ref(field, value)?;
        if value.to_ascii_lowercase().contains("candidate") {
            return Err(AtelierError::Validation(format!(
                "{field} must cite a verified product ref, not a candidate name"
            )));
        }
    }
    Ok(())
}

fn jsonb_string_array(value: Value) -> AtelierResult<Vec<String>> {
    serde_json::from_value(value)
        .map_err(|err| AtelierError::Validation(format!("expected JSON string array: {err}")))
}
