//! Generic model-visible action receipts (WP-KERNEL-005 MT-139).
//!
//! This schema records every model-visible operation by action id, params hash,
//! actor/session, timing, status, and refs. Raw params are never persisted.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use surrealdb::types::{Datetime, RecordId, SurrealValue, Uuid as SurrealUuid};
use uuid::Uuid;

use crate::kernel::action_catalog::kernel002_action_catalog;

use super::{
    atelier_event_sql, reject_legacy_runtime_ref, AtelierError, AtelierResult, AtelierStore,
};

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

/// One `atelier_action_receipt` row as the store returns it.
#[derive(SurrealValue)]
struct ActionReceiptRow {
    receipt_id: SurrealUuid,
    action_id: String,
    params_sha256: String,
    actor_kind: String,
    actor_id: String,
    session_id: String,
    started_at_utc: Datetime,
    completed_at_utc: Datetime,
    status: String,
    target_refs: Vec<String>,
    evidence_refs: Vec<String>,
    result_refs: Vec<String>,
    error_class: Option<String>,
    recovery_hint: Option<String>,
    created_at_utc: Datetime,
}

impl TryFrom<ActionReceiptRow> for ActionReceipt {
    type Error = AtelierError;

    fn try_from(row: ActionReceiptRow) -> AtelierResult<Self> {
        Ok(ActionReceipt {
            receipt_id: row.receipt_id.into(),
            action_id: row.action_id,
            params_sha256: row.params_sha256,
            actor_kind: row.actor_kind,
            actor_id: row.actor_id,
            session_id: row.session_id,
            started_at_utc: row.started_at_utc.into(),
            completed_at_utc: row.completed_at_utc.into(),
            status: ActionReceiptStatus::from_token(&row.status)?,
            target_refs: row.target_refs,
            evidence_refs: row.evidence_refs,
            result_refs: row.result_refs,
            error_class: row.error_class,
            recovery_hint: row.recovery_hint,
            created_at_utc: row.created_at_utc.into(),
        })
    }
}

#[derive(Clone, SurrealValue)]
struct ActionReceiptBindings {
    record_id: RecordId,
    receipt_id: SurrealUuid,
    action_id: String,
    params_sha256: String,
    actor_kind: String,
    actor_id: String,
    session_id: String,
    started_at_utc: Datetime,
    completed_at_utc: Datetime,
    status: String,
    target_refs: Vec<String>,
    evidence_refs: Vec<String>,
    result_refs: Vec<String>,
    error_class: Option<String>,
    recovery_hint: Option<String>,
}

#[derive(SurrealValue)]
struct ReceiptIdBinding {
    receipt_id: SurrealUuid,
}

/// Write one action receipt and its event in the same atomic statement.
/// `created_at_utc` is stamped by the schema default.
const RECORD_ACTION_RECEIPT_STATEMENT: &str = concat!(
    "RETURN { \
       LET $rid = $domain.record_id; ",
    atelier_event_sql!(),
    " RETURN (CREATE $rid CONTENT { \
         receipt_id: $domain.receipt_id, \
         action_id: $domain.action_id, \
         params_sha256: $domain.params_sha256, \
         actor_kind: $domain.actor_kind, \
         actor_id: $domain.actor_id, \
         session_id: $domain.session_id, \
         started_at_utc: $domain.started_at_utc, \
         completed_at_utc: $domain.completed_at_utc, \
         status: $domain.status, \
         target_refs: $domain.target_refs, \
         evidence_refs: $domain.evidence_refs, \
         result_refs: $domain.result_refs, \
         error_class: $domain.error_class, \
         recovery_hint: $domain.recovery_hint \
       })[0]; };"
);

const GET_ACTION_RECEIPT_STATEMENT: &str =
    "SELECT receipt_id, action_id, params_sha256, actor_kind, actor_id, session_id, \
            started_at_utc, completed_at_utc, status, target_refs, evidence_refs, \
            result_refs, error_class, recovery_hint, created_at_utc \
     FROM atelier_action_receipt WHERE receipt_id = $receipt_id LIMIT 1;";

impl AtelierStore {
    pub async fn record_action_receipt(
        &self,
        input: &NewActionReceipt,
    ) -> AtelierResult<ActionReceipt> {
        validate_action_receipt(input)?;
        let params_sha256 = params_sha256(&input.params)?;
        let receipt_id = Uuid::now_v7();

        let bindings = ActionReceiptBindings {
            record_id: RecordId::new("atelier_action_receipt", SurrealUuid::from(receipt_id)),
            receipt_id: SurrealUuid::from(receipt_id),
            action_id: input.action_id.clone(),
            params_sha256: params_sha256.clone(),
            actor_kind: input.actor_kind.clone(),
            actor_id: input.actor_id.clone(),
            session_id: input.session_id.clone(),
            started_at_utc: Datetime::from(input.started_at_utc),
            completed_at_utc: Datetime::from(input.completed_at_utc),
            status: input.status.as_token().to_owned(),
            target_refs: input.target_refs.clone(),
            evidence_refs: input.evidence_refs.clone(),
            result_refs: input.result_refs.clone(),
            error_class: input.error_class.clone(),
            recovery_hint: input.recovery_hint.clone(),
        };
        let row: Option<ActionReceiptRow> = self
            .write_with_event(
                RECORD_ACTION_RECEIPT_STATEMENT,
                bindings,
                action_receipt_event_family::ACTION_RECEIPT_RECORDED,
                "atelier_action_receipt",
                &receipt_id.to_string(),
                serde_json::json!({
                    "receipt_id": receipt_id,
                    "action_id": input.action_id,
                    "params_sha256": params_sha256,
                    "actor_kind": input.actor_kind,
                    "actor_id": input.actor_id,
                    "session_id": input.session_id,
                    "started_at_utc": input.started_at_utc,
                    "completed_at_utc": input.completed_at_utc,
                    "status": input.status.as_token(),
                    "target_refs": input.target_refs,
                    "evidence_refs": input.evidence_refs,
                    "result_refs": input.result_refs,
                    "error_class": input.error_class,
                    "recovery_hint": input.recovery_hint,
                    "schema": "hsk.atelier.action_receipt@1",
                }),
            )
            .await?;
        row.ok_or_else(|| {
            AtelierError::Internal("recording an action receipt returned no row".to_owned())
        })?
        .try_into()
    }

    pub async fn get_action_receipt(&self, receipt_id: Uuid) -> AtelierResult<ActionReceipt> {
        let bindings = ReceiptIdBinding {
            receipt_id: SurrealUuid::from(receipt_id),
        };
        let row: Option<ActionReceiptRow> = self
            .store()
            .with_data_operation(move |ctx| {
                Box::pin(async move {
                    ctx.query_first(GET_ACTION_RECEIPT_STATEMENT, bindings)
                        .await
                })
            })
            .await?;
        match row {
            Some(row) => row.try_into(),
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
