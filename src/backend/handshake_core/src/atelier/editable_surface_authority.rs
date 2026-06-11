//! Live-authority persistence for the self-improvement EditableSurfaces
//! (WP-KERNEL-005 MT-149).
//!
//! The two allow-listed editable surfaces persist their live values here:
//! ModelManual capsule section text and RetrievalPolicy parameters. The
//! production PG-backed surface providers ([`pg_model_manual_surface`] /
//! [`pg_retrieval_policy_surface`]) read snapshots from these tables and
//! write promotions through these methods, which mirror every
//! live-authority write into the Atelier EventLedger.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::Row;

use crate::memory::persistence_postgres::block_on;
use crate::memory::policy_table::CapsulePolicyTable;
use crate::memory::TaskType;
use crate::self_improve::editable_surface::{
    EditableSurfaceError, ModelManualSurface, RetrievalPolicySurface,
};
use crate::self_improve::iteration::PolicyParameterRef;

use super::{AtelierError, AtelierResult, AtelierStore};

pub mod editable_surface_event_family {
    pub const MODEL_MANUAL_SECTION_WRITTEN: &str =
        "atelier.editable_surface.model_manual_section_written";
    pub const RETRIEVAL_POLICY_WRITTEN: &str =
        "atelier.editable_surface.retrieval_policy_written";

    pub const ALL: &[&str] = &[MODEL_MANUAL_SECTION_WRITTEN, RETRIEVAL_POLICY_WRITTEN];
}

pub fn task_type_token(task_type: TaskType) -> &'static str {
    match task_type {
        TaskType::ValidatorHbrTestPacket => "validator_hbr_test_packet",
        TaskType::KernelBuilderMtImplementation => "kernel_builder_mt_implementation",
        TaskType::IntegrationValidatorBatchReview => "integration_validator_batch_review",
        TaskType::OperatorTriage => "operator_triage",
        TaskType::SwarmHarnessSession => "swarm_harness_session",
        TaskType::ProcessLedgerInspection => "process_ledger_inspection",
        TaskType::SelfImprovementLoopEval => "self_improvement_loop_eval",
        TaskType::GeneralRetrieval => "general_retrieval",
    }
}

pub fn policy_parameter_token(parameter: PolicyParameterRef) -> &'static str {
    match parameter {
        PolicyParameterRef::TopK => "top_k",
        PolicyParameterRef::CapsuleBudgetBytes => "capsule_budget_bytes",
    }
}

/// Persisted ModelManual capsule section row.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelManualSectionRecord {
    pub section_id: String,
    pub section_text: String,
    pub revision: i64,
    pub updated_by: String,
    pub updated_at_utc: DateTime<Utc>,
    pub created_at_utc: DateTime<Utc>,
}

/// Persisted RetrievalPolicy parameter row.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RetrievalPolicyValueRecord {
    pub task_type: TaskType,
    pub parameter: PolicyParameterRef,
    pub value: i64,
    pub updated_by: String,
    pub updated_at_utc: DateTime<Utc>,
}

impl AtelierStore {
    /// Read the live ModelManual section text, if persisted.
    pub async fn get_model_manual_section(
        &self,
        section_id: &str,
    ) -> AtelierResult<Option<ModelManualSectionRecord>> {
        let row = sqlx::query(
            r#"SELECT section_id, section_text, revision, updated_by,
                      updated_at_utc, created_at_utc
               FROM atelier_model_manual_section
               WHERE section_id = $1"#,
        )
        .bind(section_id)
        .fetch_optional(self.pool())
        .await?;
        row.map(|row| manual_section_from_row(&row)).transpose()
    }

    /// Write the live ModelManual section text (insert or revision bump)
    /// and mirror the write through the EventLedger. This is the single
    /// authority write path the PG surface provider's `promote` uses.
    pub async fn upsert_model_manual_section(
        &self,
        section_id: &str,
        section_text: &str,
        updated_by: &str,
    ) -> AtelierResult<ModelManualSectionRecord> {
        validate_trimmed("section_id", section_id)?;
        validate_trimmed("updated_by", updated_by)?;
        if section_text.trim().is_empty() {
            return Err(AtelierError::Validation(
                "section_text must not be empty".into(),
            ));
        }
        if section_text.len() > 1_048_576 {
            return Err(AtelierError::Validation(
                "section_text exceeds 1MiB cap".into(),
            ));
        }

        let mut tx = self.pool().begin().await?;
        let row = sqlx::query(
            r#"INSERT INTO atelier_model_manual_section (
                   section_id, section_text, revision, updated_by
               )
               VALUES ($1, $2, 1, $3)
               ON CONFLICT (section_id) DO UPDATE
               SET section_text = EXCLUDED.section_text,
                   revision = atelier_model_manual_section.revision + 1,
                   updated_by = EXCLUDED.updated_by,
                   updated_at_utc = NOW()
               RETURNING section_id, section_text, revision, updated_by,
                         updated_at_utc, created_at_utc"#,
        )
        .bind(section_id)
        .bind(section_text)
        .bind(updated_by)
        .fetch_one(&mut *tx)
        .await?;
        let record = manual_section_from_row(&row)?;

        self.record_event_in_tx(
            &mut tx,
            editable_surface_event_family::MODEL_MANUAL_SECTION_WRITTEN,
            "atelier_model_manual_section",
            &record.section_id,
            serde_json::json!({
                "section_id": record.section_id,
                "revision": record.revision,
                "section_text_sha256": sha256_hex(record.section_text.as_bytes()),
                "section_text_bytes": record.section_text.len(),
                "updated_by": record.updated_by,
                "schema": "hsk.atelier.model_manual_section@1",
            }),
        )
        .await?;
        tx.commit().await?;
        Ok(record)
    }

    /// Read the live RetrievalPolicy parameter value, if persisted. Callers
    /// fall back to `CapsulePolicyTable::default_policy_for` when `None`.
    pub async fn get_retrieval_policy_value(
        &self,
        task_type: TaskType,
        parameter: PolicyParameterRef,
    ) -> AtelierResult<Option<RetrievalPolicyValueRecord>> {
        let row = sqlx::query(
            r#"SELECT task_type, parameter, value, updated_by, updated_at_utc
               FROM atelier_retrieval_policy
               WHERE task_type = $1 AND parameter = $2"#,
        )
        .bind(task_type_token(task_type))
        .bind(policy_parameter_token(parameter))
        .fetch_optional(self.pool())
        .await?;
        match row {
            Some(row) => Ok(Some(RetrievalPolicyValueRecord {
                task_type,
                parameter,
                value: row.get("value"),
                updated_by: row.get("updated_by"),
                updated_at_utc: row.get("updated_at_utc"),
            })),
            None => Ok(None),
        }
    }

    /// Write the live RetrievalPolicy parameter value and mirror it through
    /// the EventLedger. Single authority write path for `promote`.
    pub async fn upsert_retrieval_policy_value(
        &self,
        task_type: TaskType,
        parameter: PolicyParameterRef,
        value: i64,
        updated_by: &str,
    ) -> AtelierResult<RetrievalPolicyValueRecord> {
        validate_trimmed("updated_by", updated_by)?;
        if value <= 0 {
            return Err(AtelierError::Validation(
                "retrieval policy value must be positive".into(),
            ));
        }

        let mut tx = self.pool().begin().await?;
        let row = sqlx::query(
            r#"INSERT INTO atelier_retrieval_policy (
                   task_type, parameter, value, updated_by
               )
               VALUES ($1, $2, $3, $4)
               ON CONFLICT (task_type, parameter) DO UPDATE
               SET value = EXCLUDED.value,
                   updated_by = EXCLUDED.updated_by,
                   updated_at_utc = NOW()
               RETURNING task_type, parameter, value, updated_by, updated_at_utc"#,
        )
        .bind(task_type_token(task_type))
        .bind(policy_parameter_token(parameter))
        .bind(value)
        .bind(updated_by)
        .fetch_one(&mut *tx)
        .await?;
        let record = RetrievalPolicyValueRecord {
            task_type,
            parameter,
            value: row.get("value"),
            updated_by: row.get("updated_by"),
            updated_at_utc: row.get("updated_at_utc"),
        };

        let aggregate_id = format!(
            "{}:{}",
            task_type_token(task_type),
            policy_parameter_token(parameter)
        );
        self.record_event_in_tx(
            &mut tx,
            editable_surface_event_family::RETRIEVAL_POLICY_WRITTEN,
            "atelier_retrieval_policy",
            &aggregate_id,
            serde_json::json!({
                "task_type": task_type_token(task_type),
                "parameter": policy_parameter_token(parameter),
                "value": record.value,
                "updated_by": record.updated_by,
                "schema": "hsk.atelier.retrieval_policy@1",
            }),
        )
        .await?;
        tx.commit().await?;
        Ok(record)
    }
}

fn manual_section_from_row(row: &sqlx::postgres::PgRow) -> AtelierResult<ModelManualSectionRecord> {
    Ok(ModelManualSectionRecord {
        section_id: row.get("section_id"),
        section_text: row.get("section_text"),
        revision: row.get("revision"),
        updated_by: row.get("updated_by"),
        updated_at_utc: row.get("updated_at_utc"),
        created_at_utc: row.get("created_at_utc"),
    })
}

fn validate_trimmed(field: &str, value: &str) -> AtelierResult<()> {
    if value.trim().is_empty() || value.trim() != value {
        return Err(AtelierError::Validation(format!(
            "{field} must not be empty or padded"
        )));
    }
    Ok(())
}

fn sha256_hex(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    format!("sha256:{}", hex::encode(Sha256::digest(bytes)))
}

fn surface_io(error: AtelierError) -> EditableSurfaceError {
    EditableSurfaceError::Io {
        message: error.to_string(),
    }
}

/// Production [`ModelManualSurface`] wired to the live PG authority table.
///
/// `snapshot` reads the persisted section text through
/// [`AtelierStore::get_model_manual_section`]; `promote` writes the gated
/// candidate through [`AtelierStore::upsert_model_manual_section`], the
/// single authority write path (revision bump + EventLedger mirror).
///
/// The provider closures bridge the sync `EditableSurfaceProvider` trait to
/// async PG via the shared Tokio bridge; callers inside a runtime must use
/// a multi-thread runtime (`#[tokio::test(flavor = "multi_thread")]`).
pub fn pg_model_manual_surface(
    store: AtelierStore,
    updated_by: String,
) -> ModelManualSurface<
    impl Fn(&str) -> Result<String, EditableSurfaceError>,
    impl Fn(&str, &str) -> Result<(), EditableSurfaceError>,
> {
    let read_store = store.clone();
    let write_store = store;
    ModelManualSurface::new(
        move |section_id: &str| {
            let record = block_on(read_store.get_model_manual_section(section_id))
                .map_err(surface_io)?;
            match record {
                Some(record) => Ok(record.section_text),
                None => Err(EditableSurfaceError::Io {
                    message: format!(
                        "model manual section {section_id} has no live authority row; \
                         seed it via upsert_model_manual_section before looping on it"
                    ),
                }),
            }
        },
        move |section_id: &str, new_text: &str| {
            block_on(write_store.upsert_model_manual_section(section_id, new_text, &updated_by))
                .map(|_| ())
                .map_err(surface_io)
        },
    )
}

/// Production [`RetrievalPolicySurface`] wired to the live PG authority
/// table.
///
/// `snapshot` reads the persisted parameter value through
/// [`AtelierStore::get_retrieval_policy_value`], falling back to
/// [`CapsulePolicyTable::default_policy_for`] when no live row exists yet;
/// `promote` writes the gated candidate through
/// [`AtelierStore::upsert_retrieval_policy_value`] (EventLedger mirror).
pub fn pg_retrieval_policy_surface(
    store: AtelierStore,
    updated_by: String,
) -> RetrievalPolicySurface<
    impl Fn(TaskType, PolicyParameterRef) -> Result<u64, EditableSurfaceError>,
    impl Fn(TaskType, PolicyParameterRef, u64) -> Result<(), EditableSurfaceError>,
> {
    let read_store = store.clone();
    let write_store = store;
    RetrievalPolicySurface::new(
        move |task_type: TaskType, parameter: PolicyParameterRef| {
            let record = block_on(read_store.get_retrieval_policy_value(task_type, parameter))
                .map_err(surface_io)?;
            match record {
                Some(record) => u64::try_from(record.value).map_err(|_| {
                    EditableSurfaceError::Io {
                        message: format!(
                            "persisted retrieval policy value {} is negative",
                            record.value
                        ),
                    }
                }),
                None => {
                    let default_policy = CapsulePolicyTable::default_policy_for(task_type);
                    Ok(match parameter {
                        PolicyParameterRef::TopK => u64::from(default_policy.top_k),
                        PolicyParameterRef::CapsuleBudgetBytes => {
                            default_policy.capsule_budget_bytes
                        }
                    })
                }
            }
        },
        move |task_type: TaskType, parameter: PolicyParameterRef, value: u64| {
            let value = i64::try_from(value).map_err(|_| EditableSurfaceError::Io {
                message: format!("retrieval policy value {value} exceeds i64 range"),
            })?;
            block_on(write_store.upsert_retrieval_policy_value(
                task_type,
                parameter,
                value,
                &updated_by,
            ))
            .map(|_| ())
            .map_err(surface_io)
        },
    )
}
