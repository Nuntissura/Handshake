//! Live-authority persistence for the self-improvement EditableSurfaces
//! (WP-KERNEL-005 MT-149).
//!
//! The two allow-listed editable surfaces persist their live values here:
//! ModelManual capsule section text and RetrievalPolicy parameters. The
//! production store-backed surface providers ([`live_model_manual_surface`] /
//! [`live_retrieval_policy_surface`]) read snapshots from these tables and
//! write promotions through these methods, which mirror every
//! live-authority write into the Atelier EventLedger.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use surrealdb::types::{Datetime, SurrealValue};

use crate::memory::policy_table::CapsulePolicyTable;
use crate::memory::TaskType;
use crate::self_improve::editable_surface::{
    EditableSurfaceError, ModelManualSurface, RetrievalPolicySurface,
};
use crate::self_improve::iteration::PolicyParameterRef;

use super::{atelier_event_sql, AtelierError, AtelierResult, AtelierStore};

pub mod editable_surface_event_family {
    pub const MODEL_MANUAL_SECTION_WRITTEN: &str =
        "atelier.editable_surface.model_manual_section_written";
    pub const RETRIEVAL_POLICY_WRITTEN: &str = "atelier.editable_surface.retrieval_policy_written";

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

/// One `atelier_model_manual_section` row as the store returns it.
#[derive(SurrealValue)]
struct ModelManualSectionRow {
    section_id: String,
    section_text: String,
    revision: i64,
    updated_by: String,
    updated_at_utc: Datetime,
    created_at_utc: Datetime,
}

impl From<ModelManualSectionRow> for ModelManualSectionRecord {
    fn from(row: ModelManualSectionRow) -> Self {
        ModelManualSectionRecord {
            section_id: row.section_id,
            section_text: row.section_text,
            revision: row.revision,
            updated_by: row.updated_by,
            updated_at_utc: row.updated_at_utc.into(),
            created_at_utc: row.created_at_utc.into(),
        }
    }
}

/// One `atelier_retrieval_policy` row as the store returns it.
#[derive(SurrealValue)]
struct RetrievalPolicyRow {
    value: i64,
    updated_by: String,
    updated_at_utc: Datetime,
}

#[derive(SurrealValue)]
struct SectionIdBinding {
    section_id: String,
}

#[derive(Clone, SurrealValue)]
struct UpsertManualSectionBindings {
    section_id: String,
    section_text: String,
    updated_by: String,
}

#[derive(SurrealValue)]
struct RetrievalPolicyKeyBindings {
    task_type: String,
    parameter: String,
}

#[derive(Clone, SurrealValue)]
struct UpsertRetrievalPolicyBindings {
    policy_id: String,
    task_type: String,
    parameter: String,
    value: i64,
    updated_by: String,
}

const GET_MODEL_MANUAL_SECTION_STATEMENT: &str =
    "SELECT section_id, section_text, revision, updated_by, updated_at_utc, created_at_utc \
     FROM atelier_model_manual_section WHERE section_id = $section_id LIMIT 1;";

/// Insert-or-revision-bump for one manual section, with the EventLedger
/// mirror in the same atomic statement. `revision` is NONE before the first
/// write, so `(revision ?? 0) + 1` yields 1 on insert and a bump on update;
/// `created_at_utc` comes from the schema default and survives updates.
const UPSERT_MODEL_MANUAL_SECTION_STATEMENT: &str = concat!(
    "RETURN { \
       LET $rid = type::record('atelier_model_manual_section', $domain.section_id); ",
    atelier_event_sql!(),
    " RETURN (UPSERT $rid SET \
         section_id = $domain.section_id, \
         section_text = $domain.section_text, \
         revision = (revision ?? 0) + 1, \
         updated_by = $domain.updated_by, \
         updated_at_utc = time::now() \
       RETURN AFTER)[0]; };"
);

const GET_RETRIEVAL_POLICY_STATEMENT: &str =
    "SELECT value, updated_by, updated_at_utc FROM atelier_retrieval_policy \
     WHERE task_type = $task_type AND parameter = $parameter LIMIT 1;";

const UPSERT_RETRIEVAL_POLICY_STATEMENT: &str = concat!(
    "RETURN { \
       LET $rid = type::record('atelier_retrieval_policy', $domain.policy_id); ",
    atelier_event_sql!(),
    " RETURN (UPSERT $rid SET \
         task_type = $domain.task_type, \
         parameter = $domain.parameter, \
         value = $domain.value, \
         updated_by = $domain.updated_by, \
         updated_at_utc = time::now() \
       RETURN AFTER)[0]; };"
);

impl AtelierStore {
    /// Read the live ModelManual section text, if persisted.
    pub async fn get_model_manual_section(
        &self,
        section_id: &str,
    ) -> AtelierResult<Option<ModelManualSectionRecord>> {
        let bindings = SectionIdBinding {
            section_id: section_id.to_owned(),
        };
        let row: Option<ModelManualSectionRow> = self
            .store()
            .with_data_operation(move |ctx| {
                Box::pin(async move {
                    ctx.query_first(GET_MODEL_MANUAL_SECTION_STATEMENT, bindings)
                        .await
                })
            })
            .await?;
        Ok(row.map(ModelManualSectionRecord::from))
    }

    /// Write the live ModelManual section text (insert or revision bump)
    /// and mirror the write through the EventLedger. This is the single
    /// authority write path the live surface provider's `promote` uses.
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

        // Best-effort view of the revision the statement will assign, so the
        // event payload keeps carrying it. The row returned by the write is
        // the authoritative value.
        let expected_revision = self
            .get_model_manual_section(section_id)
            .await?
            .map(|record| record.revision + 1)
            .unwrap_or(1);

        let bindings = UpsertManualSectionBindings {
            section_id: section_id.to_owned(),
            section_text: section_text.to_owned(),
            updated_by: updated_by.to_owned(),
        };
        let row: Option<ModelManualSectionRow> = self
            .write_with_event(
                UPSERT_MODEL_MANUAL_SECTION_STATEMENT,
                bindings,
                editable_surface_event_family::MODEL_MANUAL_SECTION_WRITTEN,
                "atelier_model_manual_section",
                section_id,
                serde_json::json!({
                    "section_id": section_id,
                    "revision": expected_revision,
                    "section_text_sha256": sha256_hex(section_text.as_bytes()),
                    "section_text_bytes": section_text.len(),
                    "updated_by": updated_by,
                    "schema": "hsk.atelier.model_manual_section@1",
                }),
            )
            .await?;
        Ok(row
            .ok_or_else(|| {
                AtelierError::Internal(
                    "upserting a model manual section returned no row".to_owned(),
                )
            })?
            .into())
    }

    /// Read the live RetrievalPolicy parameter value, if persisted. Callers
    /// fall back to `CapsulePolicyTable::default_policy_for` when `None`.
    pub async fn get_retrieval_policy_value(
        &self,
        task_type: TaskType,
        parameter: PolicyParameterRef,
    ) -> AtelierResult<Option<RetrievalPolicyValueRecord>> {
        let bindings = RetrievalPolicyKeyBindings {
            task_type: task_type_token(task_type).to_owned(),
            parameter: policy_parameter_token(parameter).to_owned(),
        };
        let row: Option<RetrievalPolicyRow> = self
            .store()
            .with_data_operation(move |ctx| {
                Box::pin(async move {
                    ctx.query_first(GET_RETRIEVAL_POLICY_STATEMENT, bindings)
                        .await
                })
            })
            .await?;
        Ok(row.map(|row| RetrievalPolicyValueRecord {
            task_type,
            parameter,
            value: row.value,
            updated_by: row.updated_by,
            updated_at_utc: row.updated_at_utc.into(),
        }))
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

        let aggregate_id = format!(
            "{}:{}",
            task_type_token(task_type),
            policy_parameter_token(parameter)
        );
        let bindings = UpsertRetrievalPolicyBindings {
            policy_id: aggregate_id.clone(),
            task_type: task_type_token(task_type).to_owned(),
            parameter: policy_parameter_token(parameter).to_owned(),
            value,
            updated_by: updated_by.to_owned(),
        };
        let row: Option<RetrievalPolicyRow> = self
            .write_with_event(
                UPSERT_RETRIEVAL_POLICY_STATEMENT,
                bindings,
                editable_surface_event_family::RETRIEVAL_POLICY_WRITTEN,
                "atelier_retrieval_policy",
                &aggregate_id,
                serde_json::json!({
                    "task_type": task_type_token(task_type),
                    "parameter": policy_parameter_token(parameter),
                    "value": value,
                    "updated_by": updated_by,
                    "schema": "hsk.atelier.retrieval_policy@1",
                }),
            )
            .await?;
        let row = row.ok_or_else(|| {
            AtelierError::Internal("upserting a retrieval policy value returned no row".to_owned())
        })?;
        Ok(RetrievalPolicyValueRecord {
            task_type,
            parameter,
            value: row.value,
            updated_by: row.updated_by,
            updated_at_utc: row.updated_at_utc.into(),
        })
    }
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

/// Bridge the sync `EditableSurfaceProvider` trait to the async store.
/// Callers inside a runtime must use a multi-thread runtime
/// (`#[tokio::test(flavor = "multi_thread")]`).
fn block_on_surface<F: std::future::Future>(future: F) -> F::Output {
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

/// Production [`ModelManualSurface`] wired to the live embedded-store
/// authority table.
///
/// `snapshot` reads the persisted section text through
/// [`AtelierStore::get_model_manual_section`]; `promote` writes the gated
/// candidate through [`AtelierStore::upsert_model_manual_section`], the
/// single authority write path (revision bump + EventLedger mirror).
pub fn live_model_manual_surface(
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
            let record = block_on_surface(read_store.get_model_manual_section(section_id))
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
            block_on_surface(write_store.upsert_model_manual_section(
                section_id,
                new_text,
                &updated_by,
            ))
            .map(|_| ())
            .map_err(surface_io)
        },
    )
}

/// Production [`RetrievalPolicySurface`] wired to the live embedded-store
/// authority table.
///
/// `snapshot` reads the persisted parameter value through
/// [`AtelierStore::get_retrieval_policy_value`], falling back to
/// [`CapsulePolicyTable::default_policy_for`] when no live row exists yet;
/// `promote` writes the gated candidate through
/// [`AtelierStore::upsert_retrieval_policy_value`] (EventLedger mirror).
pub fn live_retrieval_policy_surface(
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
            let record =
                block_on_surface(read_store.get_retrieval_policy_value(task_type, parameter))
                    .map_err(surface_io)?;
            match record {
                Some(record) => u64::try_from(record.value).map_err(|_| EditableSurfaceError::Io {
                    message: format!(
                        "persisted retrieval policy value {} is negative",
                        record.value
                    ),
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
            block_on_surface(write_store.upsert_retrieval_policy_value(
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
