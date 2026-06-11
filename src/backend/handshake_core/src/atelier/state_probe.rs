//! Model-workflow state probe catalog (WP-KERNEL-005 MT-138).
//!
//! This is runtime product state, not governance paperwork. The catalog gives
//! no-context models a structured list of state probes that must be checked
//! before visual inspection, then persists that list in PostgreSQL and mirrors
//! the snapshot through the Atelier EventLedger family.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sqlx::Row;

use super::{reject_legacy_runtime_ref, AtelierError, AtelierResult, AtelierStore};

pub mod state_probe_event_family {
    pub const STATE_PROBE_CATALOG_RECORDED: &str = "atelier.state_probe.catalog_recorded";
    /// Emitted when one diagnostics validation-matrix row is recorded
    /// (WP-KERNEL-005 MT-171..MT-180, MT-182, MT-195).
    pub const DIAGNOSTICS_VALIDATION_ROW_RECORDED: &str =
        "atelier.state_probe.diagnostics_validation_row_recorded";

    pub const ALL: &[&str] = &[
        STATE_PROBE_CATALOG_RECORDED,
        DIAGNOSTICS_VALIDATION_ROW_RECORDED,
    ];
}

pub const MODEL_WORKFLOW_STATE_PROBE_CATALOG_ID: &str =
    "wp-kernel-005.model-workflow.state-probe-catalog@1";

pub const REQUIRED_STATE_PROBE_SURFACES: &[StateProbeSurface] = &[
    StateProbeSurface::Character,
    StateProbeSurface::Media,
    StateProbeSurface::Intake,
    StateProbeSurface::Collection,
    StateProbeSurface::Docs,
    StateProbeSurface::Moodboard,
    StateProbeSurface::Pose,
    StateProbeSurface::ComfyUiJob,
    StateProbeSurface::Session,
    StateProbeSurface::Errors,
];

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum StateProbeSurface {
    Character,
    Media,
    Intake,
    Collection,
    Docs,
    Moodboard,
    Pose,
    ComfyUiJob,
    Session,
    Errors,
}

impl StateProbeSurface {
    pub fn as_token(self) -> &'static str {
        match self {
            StateProbeSurface::Character => "character",
            StateProbeSurface::Media => "media",
            StateProbeSurface::Intake => "intake",
            StateProbeSurface::Collection => "collection",
            StateProbeSurface::Docs => "docs",
            StateProbeSurface::Moodboard => "moodboard",
            StateProbeSurface::Pose => "pose",
            StateProbeSurface::ComfyUiJob => "comfyui_job",
            StateProbeSurface::Session => "session",
            StateProbeSurface::Errors => "errors",
        }
    }

    pub fn from_token(token: &str) -> AtelierResult<Self> {
        match token {
            "character" => Ok(StateProbeSurface::Character),
            "media" => Ok(StateProbeSurface::Media),
            "intake" => Ok(StateProbeSurface::Intake),
            "collection" => Ok(StateProbeSurface::Collection),
            "docs" => Ok(StateProbeSurface::Docs),
            "moodboard" => Ok(StateProbeSurface::Moodboard),
            "pose" => Ok(StateProbeSurface::Pose),
            "comfyui_job" => Ok(StateProbeSurface::ComfyUiJob),
            "session" => Ok(StateProbeSurface::Session),
            "errors" => Ok(StateProbeSurface::Errors),
            other => Err(AtelierError::Validation(format!(
                "unknown state probe surface: {other}"
            ))),
        }
    }
}

#[derive(Clone, Debug)]
pub struct NewStateProbeCatalog {
    pub catalog_id: String,
    pub recorded_by: String,
    pub entries: Vec<NewStateProbeCatalogEntry>,
}

#[derive(Clone, Debug)]
pub struct NewStateProbeCatalogEntry {
    pub probe_id: String,
    pub surface: StateProbeSurface,
    pub probe_label: String,
    pub read_model: String,
    pub inspection_phase: String,
    pub required_before_visual_inspection: bool,
    pub status: String,
    pub probe_fields: Value,
    pub evidence_refs: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct StateProbeCatalog {
    pub catalog_id: String,
    pub entries: Vec<StateProbeCatalogEntry>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct StateProbeCatalogEntry {
    pub catalog_id: String,
    pub probe_id: String,
    pub surface: StateProbeSurface,
    pub probe_label: String,
    pub read_model: String,
    pub inspection_phase: String,
    pub required_before_visual_inspection: bool,
    pub status: String,
    pub probe_fields: Value,
    pub evidence_refs: Vec<String>,
    pub updated_at_utc: DateTime<Utc>,
}

pub fn model_workflow_state_probe_catalog(recorded_by: impl Into<String>) -> NewStateProbeCatalog {
    NewStateProbeCatalog {
        catalog_id: MODEL_WORKFLOW_STATE_PROBE_CATALOG_ID.to_string(),
        recorded_by: recorded_by.into(),
        entries: vec![
            state_probe_entry(
                "MT-138.character",
                StateProbeSurface::Character,
                "Character identity state",
                &[
                    "character_public_id",
                    "display_name",
                    "sheet_version_count",
                    "last_event_ledger_sequence",
                ],
                &[
                    "src/backend/handshake_core/src/atelier/core.rs",
                    "src/backend/handshake_core/tests/atelier_foundation_tests.rs",
                ],
            ),
            state_probe_entry(
                "MT-138.media",
                StateProbeSurface::Media,
                "Media asset and artifact state",
                &[
                    "asset_id",
                    "content_hash",
                    "artifact_manifest",
                    "review_status",
                    "last_event_ledger_sequence",
                ],
                &[
                    "src/backend/handshake_core/src/atelier/media.rs",
                    "src/backend/handshake_core/tests/atelier_media_artifact_tests.rs",
                ],
            ),
            state_probe_entry(
                "MT-138.intake",
                StateProbeSurface::Intake,
                "Intake batch and item state",
                &[
                    "batch_id",
                    "batch_status",
                    "item_counts_by_status",
                    "resume_cursor",
                    "last_event_ledger_sequence",
                ],
                &[
                    "src/backend/handshake_core/src/atelier/intake.rs",
                    "src/backend/handshake_core/tests/atelier_intake_folder_scan_tests.rs",
                ],
            ),
            state_probe_entry(
                "MT-138.collection",
                StateProbeSurface::Collection,
                "Collection and contact-sheet state",
                &[
                    "collection_id",
                    "media_count",
                    "contact_sheet_id",
                    "projection_freshness",
                    "last_event_ledger_sequence",
                ],
                &[
                    "src/backend/handshake_core/src/atelier/collections.rs",
                    "src/backend/handshake_core/tests/atelier_core_data_tests.rs",
                ],
            ),
            state_probe_entry(
                "MT-138.docs",
                StateProbeSurface::Docs,
                "Character document state",
                &[
                    "document_id",
                    "document_kind",
                    "version_count",
                    "latest_version_id",
                    "last_event_ledger_sequence",
                ],
                &[
                    "src/backend/handshake_core/src/atelier/documents.rs",
                    "src/backend/handshake_core/tests/atelier_core_data_tests.rs",
                ],
            ),
            state_probe_entry(
                "MT-138.moodboard",
                StateProbeSurface::Moodboard,
                "Moodboard layer and export state",
                &[
                    "moodboard_id",
                    "snapshot_version",
                    "layer_count",
                    "export_status",
                    "last_event_ledger_sequence",
                ],
                &[
                    "src/backend/handshake_core/src/atelier/moodboards.rs",
                    "src/backend/handshake_core/tests/atelier_core_data_tests.rs",
                ],
            ),
            state_probe_entry(
                "MT-138.pose",
                StateProbeSurface::Pose,
                "Pose rig calibration state",
                &[
                    "pose_rig_id",
                    "source_format",
                    "calibration_status",
                    "blocked_reason",
                    "last_event_ledger_sequence",
                ],
                &[
                    "src/backend/handshake_core/src/atelier/pose.rs",
                    "src/backend/handshake_core/tests/atelier_pose_tests.rs",
                ],
            ),
            state_probe_entry(
                "MT-138.comfyui-job",
                StateProbeSurface::ComfyUiJob,
                "ComfyUI job bridge state",
                &[
                    "adapter_id",
                    "workflow_ref",
                    "job_status",
                    "capability_profile_id",
                    "last_event_ledger_sequence",
                ],
                &[
                    "src/backend/handshake_core/src/atelier/comfy.rs",
                    "src/backend/handshake_core/tests/atelier_comfy_tests.rs",
                ],
            ),
            state_probe_entry(
                "MT-138.session",
                StateProbeSurface::Session,
                "Model workflow session state",
                &[
                    "session_id",
                    "actor_id",
                    "lease_status",
                    "last_heartbeat_at_utc",
                    "last_event_ledger_sequence",
                ],
                &[
                    "src/backend/handshake_core/src/kernel/session_broker.rs",
                    "src/backend/handshake_core/tests/session_checkpoint_tests.rs",
                ],
            ),
            state_probe_entry(
                "MT-138.errors",
                StateProbeSurface::Errors,
                "Diagnostic error state",
                &[
                    "diagnostic_id",
                    "error_class",
                    "recovery_hint",
                    "status",
                    "last_event_ledger_sequence",
                ],
                &[
                    "src/backend/handshake_core/src/diagnostics/mod.rs",
                    "src/backend/handshake_core/src/api/diagnostics.rs",
                ],
            ),
        ],
    }
}

fn state_probe_entry(
    probe_id: &str,
    surface: StateProbeSurface,
    probe_label: &str,
    fields: &[&str],
    evidence_refs: &[&str],
) -> NewStateProbeCatalogEntry {
    NewStateProbeCatalogEntry {
        probe_id: probe_id.to_string(),
        surface,
        probe_label: probe_label.to_string(),
        read_model: "postgres_event_ledger_projection".to_string(),
        inspection_phase: "pre_visual_inspection".to_string(),
        required_before_visual_inspection: true,
        status: "ready".to_string(),
        probe_fields: json!({
            "schema": "hsk.atelier.state_probe.fields@1",
            "fields": fields,
            "state_authority": "postgres",
            "event_authority": "kernel_event_ledger",
        }),
        evidence_refs: evidence_refs
            .iter()
            .map(|value| value.to_string())
            .collect(),
    }
}

impl AtelierStore {
    pub async fn record_state_probe_catalog(
        &self,
        input: &NewStateProbeCatalog,
    ) -> AtelierResult<StateProbeCatalog> {
        validate_state_probe_catalog(input)?;
        let required_before_visual_inspection_count = input
            .entries
            .iter()
            .filter(|entry| entry.required_before_visual_inspection)
            .count();

        let mut tx = self.pool().begin().await?;
        let probe_ids = input
            .entries
            .iter()
            .map(|entry| entry.probe_id.clone())
            .collect::<Vec<_>>();
        sqlx::query(
            r#"DELETE FROM atelier_state_probe_catalog_entry
               WHERE catalog_id = $1
                 AND NOT (probe_id = ANY($2::text[]))"#,
        )
        .bind(&input.catalog_id)
        .bind(&probe_ids)
        .execute(&mut *tx)
        .await?;

        for entry in &input.entries {
            let evidence_refs = serde_json::to_value(&entry.evidence_refs)
                .map_err(|err| AtelierError::Validation(err.to_string()))?;
            sqlx::query(
                r#"INSERT INTO atelier_state_probe_catalog_entry (
                       catalog_id, probe_id, surface, probe_label, read_model,
                       inspection_phase, required_before_visual_inspection, status,
                       probe_fields, evidence_refs, updated_at_utc
                   )
                   VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9::jsonb, $10::jsonb, NOW())
                   ON CONFLICT (catalog_id, probe_id) DO UPDATE SET
                       surface = EXCLUDED.surface,
                       probe_label = EXCLUDED.probe_label,
                       read_model = EXCLUDED.read_model,
                       inspection_phase = EXCLUDED.inspection_phase,
                       required_before_visual_inspection =
                           EXCLUDED.required_before_visual_inspection,
                       status = EXCLUDED.status,
                       probe_fields = EXCLUDED.probe_fields,
                       evidence_refs = EXCLUDED.evidence_refs,
                       updated_at_utc = NOW()"#,
            )
            .bind(&input.catalog_id)
            .bind(&entry.probe_id)
            .bind(entry.surface.as_token())
            .bind(&entry.probe_label)
            .bind(&entry.read_model)
            .bind(&entry.inspection_phase)
            .bind(entry.required_before_visual_inspection)
            .bind(&entry.status)
            .bind(&entry.probe_fields)
            .bind(evidence_refs)
            .execute(&mut *tx)
            .await?;
        }

        self.record_event_in_tx(
            &mut tx,
            state_probe_event_family::STATE_PROBE_CATALOG_RECORDED,
            "atelier_state_probe_catalog",
            &input.catalog_id,
            json!({
                "catalog_id": input.catalog_id,
                "recorded_by": input.recorded_by,
                "surface_count": input.entries.len(),
                "required_before_visual_inspection_count":
                    required_before_visual_inspection_count,
                "schema": "hsk.atelier.state_probe_catalog@1",
            }),
        )
        .await?;
        tx.commit().await?;

        self.get_state_probe_catalog(&input.catalog_id).await
    }

    pub async fn get_state_probe_catalog(
        &self,
        catalog_id: &str,
    ) -> AtelierResult<StateProbeCatalog> {
        let catalog_id = validate_token("catalog_id", catalog_id)?;
        let rows = sqlx::query(
            r#"SELECT catalog_id, probe_id, surface, probe_label, read_model,
                      inspection_phase, required_before_visual_inspection, status,
                      probe_fields, evidence_refs, updated_at_utc
               FROM atelier_state_probe_catalog_entry
               WHERE catalog_id = $1
               ORDER BY surface, probe_id"#,
        )
        .bind(&catalog_id)
        .fetch_all(self.pool())
        .await?;

        if rows.is_empty() {
            return Err(AtelierError::NotFound(format!(
                "state probe catalog_id={catalog_id}"
            )));
        }

        let mut entries = Vec::with_capacity(rows.len());
        for row in rows {
            entries.push(StateProbeCatalogEntry {
                catalog_id: row.get("catalog_id"),
                probe_id: row.get("probe_id"),
                surface: StateProbeSurface::from_token(row.get("surface"))?,
                probe_label: row.get("probe_label"),
                read_model: row.get("read_model"),
                inspection_phase: row.get("inspection_phase"),
                required_before_visual_inspection: row.get("required_before_visual_inspection"),
                status: row.get("status"),
                probe_fields: row.get("probe_fields"),
                evidence_refs: jsonb_string_array(row.get("evidence_refs"))?,
                updated_at_utc: row.get("updated_at_utc"),
            });
        }

        Ok(StateProbeCatalog {
            catalog_id,
            entries,
        })
    }
}

fn validate_state_probe_catalog(input: &NewStateProbeCatalog) -> AtelierResult<()> {
    validate_token("catalog_id", &input.catalog_id)?;
    validate_token("recorded_by", &input.recorded_by)?;

    let mut probe_ids = std::collections::HashSet::new();
    let mut surfaces = std::collections::HashSet::new();
    for entry in &input.entries {
        validate_token("probe_id", &entry.probe_id)?;
        validate_token("probe_label", &entry.probe_label)?;
        validate_token("read_model", &entry.read_model)?;
        validate_token("inspection_phase", &entry.inspection_phase)?;
        validate_token("status", &entry.status)?;
        let expected_probe_id = expected_probe_id(entry.surface);
        if entry.probe_id != expected_probe_id {
            return Err(AtelierError::Validation(format!(
                "state probe surface {} must use exact probe_id {}",
                entry.surface.as_token(),
                expected_probe_id
            )));
        }
        if entry.read_model != "postgres_event_ledger_projection" {
            return Err(AtelierError::Validation(format!(
                "{} must use read_model postgres_event_ledger_projection",
                entry.probe_id
            )));
        }
        if entry.inspection_phase != "pre_visual_inspection" {
            return Err(AtelierError::Validation(
                "state probes must be marked pre_visual_inspection".into(),
            ));
        }
        if !entry.required_before_visual_inspection {
            return Err(AtelierError::Validation(format!(
                "{} must be required before visual inspection",
                entry.probe_id
            )));
        }
        if entry.status != "ready" {
            return Err(AtelierError::Validation(format!(
                "{} must have ready status",
                entry.probe_id
            )));
        }
        if !probe_ids.insert(entry.probe_id.as_str()) {
            return Err(AtelierError::Validation(
                "state probe catalog probe_id values must be unique".into(),
            ));
        }
        if !surfaces.insert(entry.surface) {
            return Err(AtelierError::Validation(format!(
                "state probe surface {} must be unique",
                entry.surface.as_token()
            )));
        }
        validate_probe_fields(
            &entry.probe_id,
            &entry.probe_fields,
            expected_probe_fields(entry.surface),
        )?;
        validate_ref_list("state_probe.evidence_refs", &entry.evidence_refs)?;
        let expected_refs = expected_evidence_refs(entry.surface);
        if entry.evidence_refs != expected_refs {
            return Err(AtelierError::Validation(format!(
                "{} must cite exact product evidence refs",
                entry.probe_id
            )));
        }
    }

    for required in REQUIRED_STATE_PROBE_SURFACES {
        if !surfaces.contains(required) {
            return Err(AtelierError::Validation(format!(
                "state probe catalog missing required surface {}",
                required.as_token()
            )));
        }
    }
    if input.entries.len() != REQUIRED_STATE_PROBE_SURFACES.len() {
        return Err(AtelierError::Validation(format!(
            "state probe catalog must include exactly {} required surfaces",
            REQUIRED_STATE_PROBE_SURFACES.len()
        )));
    }

    Ok(())
}

fn validate_probe_fields(
    probe_id: &str,
    value: &Value,
    expected_fields: Vec<String>,
) -> AtelierResult<()> {
    if value.get("schema").and_then(Value::as_str) != Some("hsk.atelier.state_probe.fields@1") {
        return Err(AtelierError::Validation(format!(
            "{probe_id} must use hsk.atelier.state_probe.fields@1"
        )));
    }
    if value.get("state_authority").and_then(Value::as_str) != Some("postgres") {
        return Err(AtelierError::Validation(format!(
            "{probe_id} must use postgres state_authority"
        )));
    }
    if value.get("event_authority").and_then(Value::as_str) != Some("kernel_event_ledger") {
        return Err(AtelierError::Validation(format!(
            "{probe_id} must use kernel_event_ledger event_authority"
        )));
    }
    let Some(fields) = value.get("fields").and_then(Value::as_array) else {
        return Err(AtelierError::Validation(format!(
            "{probe_id} must expose a fields array"
        )));
    };
    if fields.is_empty() {
        return Err(AtelierError::Validation(format!(
            "{probe_id} fields array must not be empty"
        )));
    }
    for field in fields {
        let Some(field) = field.as_str() else {
            return Err(AtelierError::Validation(format!(
                "{probe_id} fields must be strings"
            )));
        };
        validate_token("state_probe.field", field)?;
    }
    let fields = fields
        .iter()
        .map(|field| field.as_str().unwrap_or_default().to_string())
        .collect::<Vec<_>>();
    if fields != expected_fields {
        return Err(AtelierError::Validation(format!(
            "{probe_id} must expose exact MT-138 probe fields"
        )));
    }
    Ok(())
}

fn expected_probe_id(surface: StateProbeSurface) -> &'static str {
    match surface {
        StateProbeSurface::Character => "MT-138.character",
        StateProbeSurface::Media => "MT-138.media",
        StateProbeSurface::Intake => "MT-138.intake",
        StateProbeSurface::Collection => "MT-138.collection",
        StateProbeSurface::Docs => "MT-138.docs",
        StateProbeSurface::Moodboard => "MT-138.moodboard",
        StateProbeSurface::Pose => "MT-138.pose",
        StateProbeSurface::ComfyUiJob => "MT-138.comfyui-job",
        StateProbeSurface::Session => "MT-138.session",
        StateProbeSurface::Errors => "MT-138.errors",
    }
}

fn expected_probe_fields(surface: StateProbeSurface) -> Vec<String> {
    match surface {
        StateProbeSurface::Character => vec![
            "character_public_id",
            "display_name",
            "sheet_version_count",
            "last_event_ledger_sequence",
        ],
        StateProbeSurface::Media => vec![
            "asset_id",
            "content_hash",
            "artifact_manifest",
            "review_status",
            "last_event_ledger_sequence",
        ],
        StateProbeSurface::Intake => vec![
            "batch_id",
            "batch_status",
            "item_counts_by_status",
            "resume_cursor",
            "last_event_ledger_sequence",
        ],
        StateProbeSurface::Collection => vec![
            "collection_id",
            "media_count",
            "contact_sheet_id",
            "projection_freshness",
            "last_event_ledger_sequence",
        ],
        StateProbeSurface::Docs => vec![
            "document_id",
            "document_kind",
            "version_count",
            "latest_version_id",
            "last_event_ledger_sequence",
        ],
        StateProbeSurface::Moodboard => vec![
            "moodboard_id",
            "snapshot_version",
            "layer_count",
            "export_status",
            "last_event_ledger_sequence",
        ],
        StateProbeSurface::Pose => vec![
            "pose_rig_id",
            "source_format",
            "calibration_status",
            "blocked_reason",
            "last_event_ledger_sequence",
        ],
        StateProbeSurface::ComfyUiJob => vec![
            "adapter_id",
            "workflow_ref",
            "job_status",
            "capability_profile_id",
            "last_event_ledger_sequence",
        ],
        StateProbeSurface::Session => vec![
            "session_id",
            "actor_id",
            "lease_status",
            "last_heartbeat_at_utc",
            "last_event_ledger_sequence",
        ],
        StateProbeSurface::Errors => vec![
            "diagnostic_id",
            "error_class",
            "recovery_hint",
            "status",
            "last_event_ledger_sequence",
        ],
    }
    .into_iter()
    .map(str::to_string)
    .collect()
}

fn expected_evidence_refs(surface: StateProbeSurface) -> Vec<String> {
    match surface {
        StateProbeSurface::Character => vec![
            "src/backend/handshake_core/src/atelier/core.rs",
            "src/backend/handshake_core/tests/atelier_foundation_tests.rs",
        ],
        StateProbeSurface::Media => vec![
            "src/backend/handshake_core/src/atelier/media.rs",
            "src/backend/handshake_core/tests/atelier_media_artifact_tests.rs",
        ],
        StateProbeSurface::Intake => vec![
            "src/backend/handshake_core/src/atelier/intake.rs",
            "src/backend/handshake_core/tests/atelier_intake_folder_scan_tests.rs",
        ],
        StateProbeSurface::Collection => vec![
            "src/backend/handshake_core/src/atelier/collections.rs",
            "src/backend/handshake_core/tests/atelier_core_data_tests.rs",
        ],
        StateProbeSurface::Docs => vec![
            "src/backend/handshake_core/src/atelier/documents.rs",
            "src/backend/handshake_core/tests/atelier_core_data_tests.rs",
        ],
        StateProbeSurface::Moodboard => vec![
            "src/backend/handshake_core/src/atelier/moodboards.rs",
            "src/backend/handshake_core/tests/atelier_core_data_tests.rs",
        ],
        StateProbeSurface::Pose => vec![
            "src/backend/handshake_core/src/atelier/pose.rs",
            "src/backend/handshake_core/tests/atelier_pose_tests.rs",
        ],
        StateProbeSurface::ComfyUiJob => vec![
            "src/backend/handshake_core/src/atelier/comfy.rs",
            "src/backend/handshake_core/tests/atelier_comfy_tests.rs",
        ],
        StateProbeSurface::Session => vec![
            "src/backend/handshake_core/src/kernel/session_broker.rs",
            "src/backend/handshake_core/tests/session_checkpoint_tests.rs",
        ],
        StateProbeSurface::Errors => vec![
            "src/backend/handshake_core/src/diagnostics/mod.rs",
            "src/backend/handshake_core/src/api/diagnostics.rs",
        ],
    }
    .into_iter()
    .map(str::to_string)
    .collect()
}

fn validate_token(field: &str, value: &str) -> AtelierResult<String> {
    if value.trim().is_empty() || value.trim() != value {
        return Err(AtelierError::Validation(format!(
            "{field} must not be empty or padded"
        )));
    }
    Ok(value.to_string())
}

fn validate_ref_list(field: &str, values: &[String]) -> AtelierResult<()> {
    if values.is_empty() {
        return Err(AtelierError::Validation(format!(
            "{field} must include at least one product evidence ref"
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

// ---------------------------------------------------------------------------
// WP-KERNEL-005 MT-171..MT-180, MT-182, MT-195: diagnostics "validation matrix".
//
// Each row asserts that one required check on a model-workflow diagnostic
// surface is REQUIRED, COVERED, or DEFERRED. This is the typed runtime surface
// that turns "diagnostics are covered" into a real PostgreSQL row + EventLedger
// event, never governance markdown. The catalog
// [`diagnostics_validation_matrix_catalog`] carries the real check rows for all
// MT areas, grounded in the existing product modules they cite.
// ---------------------------------------------------------------------------

/// Matrix kind, one per MT area (171..180, 182, 195). Used to group + filter rows.
pub mod diagnostics_validation_matrix_kind {
    /// MT-171: model manual + action catalog.
    pub const MANUAL_ACTION_CATALOG: &str = "MT-171.manual-action-catalog";
    /// MT-172: session lease + heartbeat.
    pub const SESSION_LEASE_HEARTBEAT: &str = "MT-172.session-lease-heartbeat";
    /// MT-173: command log + error class + state probe.
    pub const COMMAND_LOG_ERROR_STATE_PROBE: &str = "MT-173.command-log-error-state-probe";
    /// MT-174: DCC + Flight Recorder projection.
    pub const DCC_FLIGHT_RECORDER: &str = "MT-174.dcc-flight-recorder";
    /// MT-175: visual evidence (capture, diff, ADR, STEER, comparison, retention).
    pub const VISUAL_EVIDENCE: &str = "MT-175.visual-evidence";
    /// MT-176: diagnostic bundle creation + contents.
    pub const DIAGNOSTIC_BUNDLE: &str = "MT-176.diagnostic-bundle";
    /// MT-177: local LLM config + local chat proposal boundaries.
    pub const LOCAL_LLM_CHAT_PROPOSAL: &str = "MT-177.local-llm-chat-proposal";
    /// MT-178: AI tagging proposal/apply behavior (no silent media mutation).
    pub const AI_TAGGING: &str = "MT-178.ai-tagging";
    /// MT-179: build diagnostics + packaging/release evidence (no repo-local
    /// output leakage).
    pub const BUILD_PACKAGE: &str = "MT-179.build-package";
    /// MT-180: non-intrusive automation, synthetic-input guards, parallel
    /// coordination.
    pub const SYNTHETIC_INPUT_PARALLEL_COORDINATION: &str =
        "MT-180.synthetic-input-parallel-coordination";
    /// MT-182: red-team automation-authority guards (NEGATIVE checks: hidden UI
    /// automation, focus steal, direct LLM execution, unbounded synthetic
    /// input). Rows of this kind MUST state a negative invariant.
    pub const RED_TEAM_AUTOMATION_AUTHORITY: &str = "MT-182.red-team-automation-authority";
    /// MT-194: reset/orphan validation matrix (rows persisted by the
    /// DCC/Flight-Recorder lane in `super::dcc_flight_recorder`).
    pub const RESET_ORPHAN: &str = "MT-194.reset-orphan";
    /// MT-195: stale README/spec detection + path portability (reject
    /// drive-letter / user-profile paths).
    pub const STALE_SOURCE_PATH_PORTABILITY: &str = "MT-195.stale-source-path-portability";

    /// All matrix kinds (parity/coverage helper).
    pub const ALL: &[&str] = &[
        MANUAL_ACTION_CATALOG,
        SESSION_LEASE_HEARTBEAT,
        COMMAND_LOG_ERROR_STATE_PROBE,
        DCC_FLIGHT_RECORDER,
        VISUAL_EVIDENCE,
        DIAGNOSTIC_BUNDLE,
        LOCAL_LLM_CHAT_PROPOSAL,
        AI_TAGGING,
        BUILD_PACKAGE,
        SYNTHETIC_INPUT_PARALLEL_COORDINATION,
        RED_TEAM_AUTOMATION_AUTHORITY,
        RESET_ORPHAN,
        STALE_SOURCE_PATH_PORTABILITY,
    ];
}

/// Coverage status of one diagnostics validation-matrix row.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DiagnosticsValidationStatus {
    /// The check is required but not yet proven covered.
    Required,
    /// The check is covered by a real product surface (evidence_ref expected).
    Covered,
    /// The check is intentionally deferred / carried forward.
    Deferred,
}

impl DiagnosticsValidationStatus {
    pub fn as_token(self) -> &'static str {
        match self {
            DiagnosticsValidationStatus::Required => "REQUIRED",
            DiagnosticsValidationStatus::Covered => "COVERED",
            DiagnosticsValidationStatus::Deferred => "DEFERRED",
        }
    }

    pub fn from_token(token: &str) -> AtelierResult<Self> {
        match token {
            "REQUIRED" => Ok(DiagnosticsValidationStatus::Required),
            "COVERED" => Ok(DiagnosticsValidationStatus::Covered),
            "DEFERRED" => Ok(DiagnosticsValidationStatus::Deferred),
            other => Err(AtelierError::Validation(format!(
                "unknown diagnostics validation status: {other}"
            ))),
        }
    }
}

/// Input to record one diagnostics validation-matrix row.
#[derive(Clone, Debug)]
pub struct NewDiagnosticsValidationRow {
    /// Stable kebab-case row id, unique in the matrix (the PK).
    pub row_id: String,
    /// MT-area grouping (see [`diagnostics_validation_matrix_kind`]).
    pub matrix_kind: String,
    /// The diagnostic surface this check belongs to (e.g. `model_manual`).
    pub surface: String,
    /// Stable check id within the surface (e.g. `model_manual.purpose`).
    pub check_id: String,
    /// Human-readable requirement statement. MUST be non-empty.
    pub requirement: String,
    pub status: DiagnosticsValidationStatus,
    /// Optional product evidence ref (no .GOV / SQLite / localhost / local path).
    pub evidence_ref: Option<String>,
}

/// Persisted diagnostics validation-matrix row.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct DiagnosticsValidationRow {
    pub row_id: String,
    pub matrix_kind: String,
    pub surface: String,
    pub check_id: String,
    pub requirement: String,
    pub status: DiagnosticsValidationStatus,
    pub evidence_ref: Option<String>,
    pub created_at_utc: DateTime<Utc>,
}

const DIAGNOSTICS_VALIDATION_ROW_COLUMNS: &str =
    "row_id, matrix_kind, surface, check_id, requirement, status, evidence_ref, created_at_utc";

fn diagnostics_validation_row_from_row(
    row: &sqlx::postgres::PgRow,
) -> AtelierResult<DiagnosticsValidationRow> {
    let status: String = row.get("status");
    Ok(DiagnosticsValidationRow {
        row_id: row.get("row_id"),
        matrix_kind: row.get("matrix_kind"),
        surface: row.get("surface"),
        check_id: row.get("check_id"),
        requirement: row.get("requirement"),
        status: DiagnosticsValidationStatus::from_token(&status)?,
        evidence_ref: row.get("evidence_ref"),
        created_at_utc: row.get("created_at_utc"),
    })
}

/// MT-182 probe: a red-team automation-authority row is only valid when its
/// requirement states a negative invariant (a prohibition), so an
/// automation-authority risk can never be smuggled in as a positive claim.
fn is_negative_invariant(requirement: &str) -> bool {
    let lower = requirement.to_ascii_lowercase();
    [
        "must not",
        "must never",
        "must deny",
        "must reject",
        "must refuse",
        "must be denied",
        "must be rejected",
    ]
    .iter()
    .any(|token| lower.contains(token))
}

/// MT-195 path-portability probe: reject machine-local path shapes.
///
/// Layers the user-profile rejection on top of the canonical
/// [`reject_legacy_runtime_ref`] boundary (which already rejects Windows
/// drive-letter paths, `file:` URLs, SQLite refs, `.GOV` refs, and
/// localhost/loopback authorities). A portable product ref must survive a
/// project move to another folder, disk, or machine.
pub fn reject_nonportable_path(field: &str, value: &str) -> AtelierResult<()> {
    reject_legacy_runtime_ref(field, value)?;
    let normalized = value.trim().to_ascii_lowercase().replace('\\', "/");
    let is_user_profile = normalized == "~"
        || normalized.starts_with("~/")
        || normalized.starts_with("/users/")
        || normalized.contains("/users/")
        || normalized.starts_with("/home/")
        || normalized.contains("/home/")
        || normalized.contains("%userprofile%")
        || normalized.contains("%appdata%")
        || normalized.contains("$home");
    if is_user_profile {
        return Err(AtelierError::Validation(format!(
            "{field} must be a portable product ref, not a user-profile path"
        )));
    }
    Ok(())
}

fn validate_diagnostics_validation_row(new: &NewDiagnosticsValidationRow) -> AtelierResult<()> {
    for (field, value) in [
        ("row_id", &new.row_id),
        ("matrix_kind", &new.matrix_kind),
        ("surface", &new.surface),
        ("check_id", &new.check_id),
    ] {
        if value.trim().is_empty() || value.trim() != value {
            return Err(AtelierError::Validation(format!(
                "diagnostics validation row {field} must be non-empty and unpadded"
            )));
        }
    }
    // Hard gate: a blank requirement is rejected so a check is never silent.
    if new.requirement.trim().is_empty() || new.requirement.trim() != new.requirement {
        return Err(AtelierError::Validation(
            "diagnostics validation row requirement must be non-empty and unpadded".into(),
        ));
    }
    // MT-182 hard gate: red-team automation-authority rows are NEGATIVE checks;
    // a positively-phrased "guard" row is rejected so a risk is never recorded
    // as if it were a feature.
    if new.matrix_kind == diagnostics_validation_matrix_kind::RED_TEAM_AUTOMATION_AUTHORITY
        && !is_negative_invariant(&new.requirement)
    {
        return Err(AtelierError::Validation(
            "red-team automation-authority rows must state a negative invariant \
             (e.g. `must not` / `must never` / `must deny`)"
                .into(),
        ));
    }
    if let Some(evidence_ref) = &new.evidence_ref {
        if new.matrix_kind == diagnostics_validation_matrix_kind::STALE_SOURCE_PATH_PORTABILITY {
            // MT-195 hard gate: portability rows must themselves cite portable
            // refs (no drive-letter, no user-profile path).
            reject_nonportable_path("evidence_ref", evidence_ref)?;
        } else {
            // Reuse the canonical .GOV/SQLite/localhost/local-path rejection boundary.
            reject_legacy_runtime_ref("evidence_ref", evidence_ref)?;
        }
    }
    Ok(())
}

/// One helper to build a COVERED row that cites a real product module path.
fn covered_row(
    row_id: &str,
    matrix_kind: &str,
    surface: &str,
    check_id: &str,
    requirement: &str,
    evidence_ref: &str,
) -> NewDiagnosticsValidationRow {
    NewDiagnosticsValidationRow {
        row_id: row_id.to_string(),
        matrix_kind: matrix_kind.to_string(),
        surface: surface.to_string(),
        check_id: check_id.to_string(),
        requirement: requirement.to_string(),
        status: DiagnosticsValidationStatus::Covered,
        evidence_ref: Some(evidence_ref.to_string()),
    }
}

/// The real diagnostics validation-matrix rows for all MT areas
/// (MT-171..MT-180, MT-182, MT-195).
///
/// Const-style data (mirrors [`source_evidence::core_data_source_evidence_matrix`]
/// and [`pose::pose_deferred_feature_catalog`]): every row names a real surface +
/// check and cites a product module that exists in the source tree, so a test can
/// persist this catalog and reload it to prove the runtime surface.
pub fn diagnostics_validation_matrix_catalog() -> Vec<NewDiagnosticsValidationRow> {
    use diagnostics_validation_matrix_kind as kind;
    vec![
        // ---- MT-171: model manual + action catalog.
        covered_row(
            "mt-171.model-manual.purpose",
            kind::MANUAL_ACTION_CATALOG,
            "model_manual",
            "model_manual.purpose",
            "The built-in model manual must state the product purpose for a no-context model.",
            "src/backend/handshake_core/src/kernel/model_manual.rs",
        ),
        covered_row(
            "mt-171.model-manual.core-workflows",
            kind::MANUAL_ACTION_CATALOG,
            "model_manual",
            "model_manual.core_workflows",
            "The model manual must describe core workflows, startup/run commands, and recovery steps.",
            "src/backend/handshake_core/src/model_manual/mod.rs",
        ),
        covered_row(
            "mt-171.action-catalog.actions-enumerated",
            kind::MANUAL_ACTION_CATALOG,
            "kernel_action_catalog",
            "action_catalog.actions_enumerated",
            "The kernel action catalog must enumerate the actions a model can invoke with stable ids.",
            "src/backend/handshake_core/src/kernel/action_catalog.rs",
        ),
        covered_row(
            "mt-171.action-catalog.inputs-outputs",
            kind::MANUAL_ACTION_CATALOG,
            "kernel_action_catalog",
            "action_catalog.inputs_outputs",
            "Each catalog action must document its expected inputs and outputs for a no-context model.",
            "src/backend/handshake_core/src/kernel/action_catalog.rs",
        ),
        // ---- MT-172: session lease + heartbeat.
        covered_row(
            "mt-172.session.lease-acquire",
            kind::SESSION_LEASE_HEARTBEAT,
            "session_broker",
            "session.lease_acquire",
            "A model session must acquire an attributable lease through the session broker before work.",
            "src/backend/handshake_core/src/kernel/session_broker.rs",
        ),
        covered_row(
            "mt-172.session.heartbeat",
            kind::SESSION_LEASE_HEARTBEAT,
            "session_broker",
            "session.heartbeat",
            "A held lease must record a heartbeat so a stale or dead session is observable.",
            "src/backend/handshake_core/src/kernel/session_broker.rs",
        ),
        covered_row(
            "mt-172.role-mailbox.lease-contract",
            kind::SESSION_LEASE_HEARTBEAT,
            "role_mailbox",
            "role_mailbox.lease_contract",
            "The role mailbox must enforce the lease contract so only the lease holder acts on a role.",
            "src/backend/handshake_core/src/kernel/role_mailbox_contract.rs",
        ),
        covered_row(
            "mt-172.role-mailbox.lease-persistence",
            kind::SESSION_LEASE_HEARTBEAT,
            "role_mailbox",
            "role_mailbox.lease_persistence",
            "Role-mailbox lease + heartbeat state must persist in PostgreSQL, not in-memory only.",
            "src/backend/handshake_core/src/role_mailbox_v1/repo.rs",
        ),
        // ---- MT-173: command log + error class + state probe.
        covered_row(
            "mt-173.command-log.recorded",
            kind::COMMAND_LOG_ERROR_STATE_PROBE,
            "diagnostics",
            "command_log.recorded",
            "Diagnostic command executions must be logged so a no-context model can replay them.",
            "src/backend/handshake_core/src/diagnostics/mod.rs",
        ),
        covered_row(
            "mt-173.error-class.classified",
            kind::COMMAND_LOG_ERROR_STATE_PROBE,
            "diagnostics",
            "error_class.classified",
            "Errors must be classified with a recovery hint so a model can isolate failures.",
            "src/backend/handshake_core/src/api/diagnostics.rs",
        ),
        covered_row(
            "mt-173.state-probe.pre-visual",
            kind::COMMAND_LOG_ERROR_STATE_PROBE,
            "state_probe",
            "state_probe.pre_visual",
            "Required state probes must be checkable before visual inspection from a typed catalog.",
            "src/backend/handshake_core/src/atelier/state_probe.rs",
        ),
        // ---- MT-174: DCC + Flight Recorder projection.
        covered_row(
            "mt-174.dcc.runtime-surface",
            kind::DCC_FLIGHT_RECORDER,
            "dcc",
            "dcc.runtime_surface",
            "The DCC runtime surface must expose structured diagnostic state to a no-context model.",
            "src/backend/handshake_core/src/kernel/dcc_mvp_runtime_surface.rs",
        ),
        covered_row(
            "mt-174.flight-recorder.projection",
            kind::DCC_FLIGHT_RECORDER,
            "flight_recorder",
            "flight_recorder.projection",
            "The Flight Recorder must project a replayable capture of runtime activity for diagnosis.",
            "src/backend/handshake_core/src/api/flight_recorder.rs",
        ),
        // ---- MT-175: visual evidence (capture, diff, ADR, STEER, comparison, retention).
        covered_row(
            "mt-175.visual.capture",
            kind::VISUAL_EVIDENCE,
            "visual_debugging_loop",
            "visual.capture",
            "Visual debugging must capture a product screenshot/snapshot as reproducible evidence.",
            "src/backend/handshake_core/src/kernel/product_screenshot_capture.rs",
        ),
        covered_row(
            "mt-175.visual.diff",
            kind::VISUAL_EVIDENCE,
            "visual_debugging_loop",
            "visual.diff",
            "Visual debugging must diff captures so visual regressions are detectable.",
            "src/backend/handshake_core/src/kernel/visual_debugging_loop.rs",
        ),
        covered_row(
            "mt-175.visual.adr",
            kind::VISUAL_EVIDENCE,
            "visual_debugging_loop",
            "visual.adr",
            "The visual loop must record an ADR (accept/defer/reject) decision on captured evidence.",
            "src/backend/handshake_core/src/kernel/visual_debugging_loop.rs",
        ),
        covered_row(
            "mt-175.visual.steer",
            kind::VISUAL_EVIDENCE,
            "visual_debugging_loop",
            "visual.steer",
            "The visual loop must support a STEER step so a model can correct the surface under test.",
            "src/backend/handshake_core/src/kernel/visual_debugging_loop.rs",
        ),
        covered_row(
            "mt-175.visual.comparison",
            kind::VISUAL_EVIDENCE,
            "visual_debugging_loop",
            "visual.comparison",
            "The visual loop must support a before/after comparison view over captured evidence.",
            "src/backend/handshake_core/src/kernel/visual_debugging_loop.rs",
        ),
        // ---- MT-176: diagnostic bundle creation + contents.
        covered_row(
            "mt-176.bundle.export-created",
            kind::DIAGNOSTIC_BUNDLE,
            "diagnostic_bundle",
            "bundle.export_created",
            "A diagnostic bundle must be creatable on demand through the bundle exporter so a \
             no-context model can hand off reproducible diagnosis state.",
            "src/backend/handshake_core/src/bundles/exporter.rs",
        ),
        covered_row(
            "mt-176.bundle.manifest-contents",
            kind::DIAGNOSTIC_BUNDLE,
            "diagnostic_bundle",
            "bundle.manifest_contents",
            "Every diagnostic bundle must carry a typed manifest enumerating its contents so the \
             bundle is inspectable without unpacking it blind.",
            "src/backend/handshake_core/src/bundles/schemas.rs",
        ),
        covered_row(
            "mt-176.bundle.secret-redaction",
            kind::DIAGNOSTIC_BUNDLE,
            "diagnostic_bundle",
            "bundle.secret_redaction",
            "Diagnostic bundle contents must pass secret redaction before export so credentials \
             never leave the workspace inside a bundle.",
            "src/backend/handshake_core/src/bundles/redactor.rs",
        ),
        covered_row(
            "mt-176.bundle.contents-validated",
            kind::DIAGNOSTIC_BUNDLE,
            "diagnostic_bundle",
            "bundle.contents_validated",
            "Diagnostic bundle contents must be validated against the bundle schema before the \
             bundle is accepted as diagnosis evidence.",
            "src/backend/handshake_core/src/bundles/validator.rs",
        ),
        // ---- MT-177: local LLM config + local chat proposal boundaries.
        covered_row(
            "mt-177.local-llm.config-registry",
            kind::LOCAL_LLM_CHAT_PROPOSAL,
            "local_llm",
            "local_llm.config_registry",
            "Local LLM model configuration must come from the typed model registry, never from \
             ad-hoc machine-local settings.",
            "src/backend/handshake_core/src/llm/registry.rs",
        ),
        covered_row(
            "mt-177.local-llm.local-routing",
            kind::LOCAL_LLM_CHAT_PROPOSAL,
            "local_llm",
            "local_llm.local_routing",
            "Local-first routing must select the local model tier and record the routing decision \
             through the Flight Recorder.",
            "src/backend/handshake_core/src/llm/local_router.rs",
        ),
        covered_row(
            "mt-177.chat-proposal.propose-only",
            kind::LOCAL_LLM_CHAT_PROPOSAL,
            "chat_proposal",
            "chat_proposal.propose_only",
            "Local chat output is a proposal boundary: applying it to authority surfaces must go \
             through the direct-edit guard, never direct execution.",
            "src/backend/handshake_core/src/kernel/direct_edit_guard.rs",
        ),
        covered_row(
            "mt-177.chat-proposal.cloud-escalation-consent",
            kind::LOCAL_LLM_CHAT_PROPOSAL,
            "chat_proposal",
            "chat_proposal.cloud_escalation_consent",
            "Escalating a local chat proposal to a cloud model must require explicit consent \
             through the cloud escalation guard.",
            "src/backend/handshake_core/src/llm/guard.rs",
        ),
        // ---- MT-178: AI tagging proposal/apply (no silent media mutation).
        covered_row(
            "mt-178.tagging.proposal-recorded",
            kind::AI_TAGGING,
            "ai_tagging",
            "ai_tagging.proposal_recorded",
            "AI tag suggestions must be recorded as typed proposals against media identity, never \
             applied silently on generation.",
            "src/backend/handshake_core/src/atelier/media.rs",
        ),
        covered_row(
            "mt-178.tagging.apply-gated",
            kind::AI_TAGGING,
            "ai_tagging",
            "ai_tagging.apply_gated",
            "Applying proposed tags must be an explicit governed mutation that validates the full \
             target set before writing.",
            "src/backend/handshake_core/src/atelier/bulk.rs",
        ),
        covered_row(
            "mt-178.tagging.no-silent-media-mutation",
            kind::AI_TAGGING,
            "ai_tagging",
            "ai_tagging.no_silent_media_mutation",
            "Applying tags must never mutate media bytes: bytes live in the ArtifactStore behind \
             a content hash, and tagging touches metadata rows only.",
            "src/backend/handshake_core/src/atelier/media.rs",
        ),
        covered_row(
            "mt-178.tagging.apply-evidence",
            kind::AI_TAGGING,
            "ai_tagging",
            "ai_tagging.apply_evidence",
            "Every tag apply must commit a durable receipt and an EventLedger event in the same \
             PostgreSQL transaction as the mutation.",
            "src/backend/handshake_core/src/atelier/bulk.rs",
        ),
        // ---- MT-179: build diagnostics + packaging/release evidence.
        covered_row(
            "mt-179.build.diagnostics-recorded",
            kind::BUILD_PACKAGE,
            "build_diagnostics",
            "build.diagnostics_recorded",
            "Build/delivery state claims must resolve to typed runtime-truth records, never to \
             prose claims about what was built.",
            "src/backend/handshake_core/src/kernel/software_delivery_runtime_truth.rs",
        ),
        covered_row(
            "mt-179.package.manifest-evidence",
            kind::BUILD_PACKAGE,
            "packaging",
            "package.manifest_evidence",
            "Packaged outputs must carry a manifest with per-file content hashes so a release \
             artifact is verifiable after transport.",
            "src/backend/handshake_core/src/bundles/zip.rs",
        ),
        covered_row(
            "mt-179.package.no-repo-local-leakage",
            kind::BUILD_PACKAGE,
            "packaging",
            "package.no_repo_local_leakage",
            "Export/release bytes must land in the governed ArtifactStore, never leak onto \
             repo-local or random filesystem paths.",
            "src/backend/handshake_core/src/atelier/exports.rs",
        ),
        covered_row(
            "mt-179.package.release-evidence",
            kind::BUILD_PACKAGE,
            "packaging",
            "package.release_evidence",
            "Each export/release must persist a durable request -> result -> manifest-entry graph \
             in PostgreSQL as release evidence.",
            "src/backend/handshake_core/src/atelier/exports.rs",
        ),
        // ---- MT-180: no-focus synthetic input + parallel coordination.
        covered_row(
            "mt-180.no-focus.stealth-window",
            kind::SYNTHETIC_INPUT_PARALLEL_COORDINATION,
            "no_focus_automation",
            "no_focus.stealth_window",
            "Model-driven reference windows must run under the stealth-window quiet flags so \
             automation never pops a foreground window.",
            "src/backend/handshake_core/src/atelier/stealth_window.rs",
        ),
        covered_row(
            "mt-180.no-focus.focus-audit",
            kind::SYNTHETIC_INPUT_PARALLEL_COORDINATION,
            "no_focus_automation",
            "no_focus.focus_audit",
            "Foreground/focus transitions caused by automation must be captured in the focus \
             audit ledger so focus steal is observable.",
            "src/backend/handshake_core/src/operator_foreground/focus_audit.rs",
        ),
        covered_row(
            "mt-180.synthetic-input.injection-flagged",
            kind::SYNTHETIC_INPUT_PARALLEL_COORDINATION,
            "synthetic_input",
            "synthetic_input.injection_flagged",
            "Synthetic keyboard input must be injected with the LLKHF_INJECTED flag and counted, \
             so injected events stay distinguishable from operator input.",
            "src/backend/handshake_core/src/operator_foreground/keyboard_inject_test.rs",
        ),
        covered_row(
            "mt-180.parallel.lease-coordination",
            kind::SYNTHETIC_INPUT_PARALLEL_COORDINATION,
            "parallel_coordination",
            "parallel.lease_coordination",
            "Parallel model work on a role must be coordinated through attributable claim leases \
             so concurrent sessions never collide silently.",
            "src/backend/handshake_core/src/kernel/role_mailbox_claim_lease.rs",
        ),
        covered_row(
            "mt-180.parallel.session-broker",
            kind::SYNTHETIC_INPUT_PARALLEL_COORDINATION,
            "parallel_coordination",
            "parallel.session_broker",
            "Parallel model sessions must be brokered so each session's actions stay observable, \
             attributable, and recoverable.",
            "src/backend/handshake_core/src/kernel/session_broker.rs",
        ),
        // ---- MT-182: red-team automation-authority guards (NEGATIVE checks).
        covered_row(
            "mt-182.red-team.hidden-ui-automation",
            kind::RED_TEAM_AUTOMATION_AUTHORITY,
            "automation_authority",
            "red_team.hidden_ui_automation",
            "Automation must never drive a hidden UI surface outside the governed stealth-window \
             registry; unregistered hidden automation must be rejected.",
            "src/backend/handshake_core/src/atelier/stealth_window.rs",
        ),
        covered_row(
            "mt-182.red-team.focus-steal",
            kind::RED_TEAM_AUTOMATION_AUTHORITY,
            "automation_authority",
            "red_team.focus_steal",
            "Automation must not steal operator focus: foreground exceptions are deny-by-default \
             and every focus transition must land in the focus audit ledger.",
            "src/backend/handshake_core/src/operator_foreground/focus_audit.rs",
        ),
        covered_row(
            "mt-182.red-team.direct-llm-execution",
            kind::RED_TEAM_AUTOMATION_AUTHORITY,
            "automation_authority",
            "red_team.direct_llm_execution",
            "An LLM must not execute direct edits on authority surfaces; direct-edit attempts \
             must be denied with a typed kernel denial.",
            "src/backend/handshake_core/src/kernel/direct_edit_guard.rs",
        ),
        covered_row(
            "mt-182.red-team.unbounded-synthetic-input",
            kind::RED_TEAM_AUTOMATION_AUTHORITY,
            "automation_authority",
            "red_team.unbounded_synthetic_input",
            "Synthetic input must never run unbounded: injected events must be flagged, counted, \
             and unflagged injection must be rejected.",
            "src/backend/handshake_core/src/operator_foreground/keyboard_inject_test.rs",
        ),
        // ---- MT-195: stale source detection + path portability.
        covered_row(
            "mt-195.stale-source.readme-drift",
            kind::STALE_SOURCE_PATH_PORTABILITY,
            "stale_source",
            "stale_source.readme_drift",
            "Stale README/spec claims must be detectable as typed spec-drift findings tying the \
             doc claim to the spec/code surface it points at.",
            "src/backend/handshake_core/src/atelier/state_probe.rs",
        ),
        covered_row(
            "mt-195.stale-source.mirror-sync",
            kind::STALE_SOURCE_PATH_PORTABILITY,
            "stale_source",
            "stale_source.mirror_sync",
            "Markdown mirrors must be drift-guarded against their machine-readable authority so a \
             stale projection is detected, not trusted.",
            "src/backend/handshake_core/src/kernel/markdown_mirror_sync_drift_guard.rs",
        ),
        covered_row(
            "mt-195.path-portability.drive-letter-rejected",
            kind::STALE_SOURCE_PATH_PORTABILITY,
            "path_portability",
            "path_portability.drive_letter_rejected",
            "Persisted refs must reject Windows drive-letter paths so a project move to another \
             disk cannot break stored evidence.",
            "src/backend/handshake_core/src/atelier/mod.rs",
        ),
        covered_row(
            "mt-195.path-portability.user-profile-rejected",
            kind::STALE_SOURCE_PATH_PORTABILITY,
            "path_portability",
            "path_portability.user_profile_rejected",
            "Persisted refs must reject user-profile paths (home directories, %USERPROFILE%) so \
             stored evidence stays machine-agnostic.",
            "src/backend/handshake_core/src/atelier/state_probe.rs",
        ),
    ]
}

impl AtelierStore {
    /// Record one diagnostics validation-matrix row (MT-171..MT-180, MT-182,
    /// MT-195).
    ///
    /// Idempotent on `row_id`: re-recording the same row updates the mutable
    /// fields and returns the row instead of erroring, so seeding the catalog
    /// twice is safe. A blank `requirement` is rejected with a `Validation`
    /// error so a check is never silent. Emits
    /// `DIAGNOSTICS_VALIDATION_ROW_RECORDED`.
    pub async fn record_diagnostics_validation_row(
        &self,
        new: &NewDiagnosticsValidationRow,
    ) -> AtelierResult<DiagnosticsValidationRow> {
        validate_diagnostics_validation_row(new)?;

        let mut tx = self.pool().begin().await?;
        let row = sqlx::query(&format!(
            r#"INSERT INTO atelier_diagnostics_validation_matrix
                 (row_id, matrix_kind, surface, check_id, requirement, status, evidence_ref)
               VALUES ($1, $2, $3, $4, $5, $6, $7)
               ON CONFLICT (row_id) DO UPDATE SET
                 matrix_kind = EXCLUDED.matrix_kind,
                 surface = EXCLUDED.surface,
                 check_id = EXCLUDED.check_id,
                 requirement = EXCLUDED.requirement,
                 status = EXCLUDED.status,
                 evidence_ref = EXCLUDED.evidence_ref
               RETURNING {DIAGNOSTICS_VALIDATION_ROW_COLUMNS}"#
        ))
        .bind(&new.row_id)
        .bind(&new.matrix_kind)
        .bind(&new.surface)
        .bind(&new.check_id)
        .bind(&new.requirement)
        .bind(new.status.as_token())
        .bind(&new.evidence_ref)
        .fetch_one(&mut *tx)
        .await?;

        let recorded = diagnostics_validation_row_from_row(&row)?;
        self.record_event_in_tx(
            &mut tx,
            state_probe_event_family::DIAGNOSTICS_VALIDATION_ROW_RECORDED,
            "atelier_diagnostics_validation_matrix",
            &recorded.row_id,
            json!({
                "row_id": recorded.row_id,
                "matrix_kind": recorded.matrix_kind,
                "surface": recorded.surface,
                "check_id": recorded.check_id,
                "requirement": recorded.requirement,
                "status": recorded.status.as_token(),
                "evidence_ref": recorded.evidence_ref,
                "schema": "hsk.atelier.diagnostics_validation_row@1",
            }),
        )
        .await?;
        tx.commit().await?;
        Ok(recorded)
    }

    /// List diagnostics validation-matrix rows, optionally filtered by
    /// `matrix_kind`. Ordered by `row_id` for deterministic, run-scoped tests.
    pub async fn list_diagnostics_validation_matrix(
        &self,
        matrix_kind: Option<&str>,
    ) -> AtelierResult<Vec<DiagnosticsValidationRow>> {
        let rows = match matrix_kind {
            Some(kind) => {
                sqlx::query(&format!(
                    r#"SELECT {DIAGNOSTICS_VALIDATION_ROW_COLUMNS}
                       FROM atelier_diagnostics_validation_matrix
                       WHERE matrix_kind = $1
                       ORDER BY row_id ASC"#
                ))
                .bind(kind)
                .fetch_all(self.pool())
                .await?
            }
            None => {
                sqlx::query(&format!(
                    r#"SELECT {DIAGNOSTICS_VALIDATION_ROW_COLUMNS}
                       FROM atelier_diagnostics_validation_matrix
                       ORDER BY row_id ASC"#
                ))
                .fetch_all(self.pool())
                .await?
            }
        };
        rows.iter().map(diagnostics_validation_row_from_row).collect()
    }
}

// ===========================================================================
// WP-KERNEL-005 MT-147 / MT-148 / MT-153 / MT-167: typed diagnostics projection
// surfaces.
//
// These are TYPED RUNTIME surfaces (PostgreSQL rows + EventLedger events), never
// governance markdown. Tables are created by migration
// `0115_atelier_diagnostics_projections.sql` (wired into
// `AtelierStore::ensure_schema`). Storage authority is
// PostgreSQL only (AtelierStore::pool()); SQLite is forbidden (MT-004).
// ===========================================================================

/// Event families for the MT-147/148/153/167 diagnostics projection surfaces.
///
/// Defined here so the parent module can fold these into
/// [`super::event_family::ALL`] (the orchestrator wires the fold-in after this
/// MT lands).
pub mod diagnostics_projection_event_family {
    /// A model work-state projection row was recorded (MT-147).
    pub const WORK_STATE_PROJECTION_RECORDED: &str =
        "atelier.state_probe.work_state_projection_recorded";
    /// A DCC panel projection row was recorded (MT-148).
    pub const DCC_PANEL_PROJECTION_RECORDED: &str =
        "atelier.state_probe.dcc_panel_projection_recorded";
    /// A screenshot artifact-storage row was recorded (MT-153).
    pub const SCREENSHOT_ARTIFACT_STORED: &str =
        "atelier.state_probe.screenshot_artifact_stored";
    /// A spec/README drift finding was recorded (MT-167).
    pub const SPEC_DRIFT_FINDING_RECORDED: &str =
        "atelier.state_probe.spec_drift_finding_recorded";

    /// All diagnostics-projection event families (parity/coverage helper).
    pub const ALL: &[&str] = &[
        WORK_STATE_PROJECTION_RECORDED,
        DCC_PANEL_PROJECTION_RECORDED,
        SCREENSHOT_ARTIFACT_STORED,
        SPEC_DRIFT_FINDING_RECORDED,
    ];
}

// ---------------------------------------------------------------------------
// MT-147: model work-state projection into Locus/MT surfaces.
// ---------------------------------------------------------------------------

/// Input to record one model work-state projection row (MT-147).
///
/// Projects atelier model work state (active MT, owner, status, plus the
/// optional blocker / receipts / next-action / evidence pointers) into a typed
/// diagnostics row a no-context model can read directly.
#[derive(Clone, Debug)]
pub struct NewWorkStateProjection {
    /// Stable projection id (the PK). MUST be non-empty / unpadded.
    pub projection_id: String,
    /// The microtask currently being worked (e.g. `MT-147`). Non-empty.
    pub active_mt: String,
    /// The owning session/actor of the active work. Non-empty.
    pub owner: String,
    /// Work status token (e.g. `READY_FOR_VALIDATION`). Non-empty.
    pub status: String,
    /// Optional current blocker description.
    pub blocker: Option<String>,
    /// Optional pointer to the receipts surface (governed ref).
    pub receipts_ref: Option<String>,
    /// Optional next-action hint for the model.
    pub next_action: Option<String>,
    /// Optional pointer to evidence (governed ref; no .GOV / SQLite / local path).
    pub evidence_ref: Option<String>,
}

/// Persisted model work-state projection row (MT-147).
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorkStateProjection {
    pub projection_id: String,
    pub active_mt: String,
    pub owner: String,
    pub status: String,
    pub blocker: Option<String>,
    pub receipts_ref: Option<String>,
    pub next_action: Option<String>,
    pub evidence_ref: Option<String>,
    pub created_at_utc: DateTime<Utc>,
}

const WORK_STATE_PROJECTION_COLUMNS: &str = "projection_id, active_mt, owner, status, blocker, \
                                             receipts_ref, next_action, evidence_ref, created_at_utc";

fn work_state_projection_from_row(row: &sqlx::postgres::PgRow) -> WorkStateProjection {
    WorkStateProjection {
        projection_id: row.get("projection_id"),
        active_mt: row.get("active_mt"),
        owner: row.get("owner"),
        status: row.get("status"),
        blocker: row.get("blocker"),
        receipts_ref: row.get("receipts_ref"),
        next_action: row.get("next_action"),
        evidence_ref: row.get("evidence_ref"),
        created_at_utc: row.get("created_at_utc"),
    }
}

fn validate_work_state_projection(new: &NewWorkStateProjection) -> AtelierResult<()> {
    for (field, value) in [
        ("projection_id", &new.projection_id),
        ("active_mt", &new.active_mt),
        ("owner", &new.owner),
        ("status", &new.status),
    ] {
        if value.trim().is_empty() || value.trim() != value {
            return Err(AtelierError::Validation(format!(
                "work state projection {field} must be non-empty and unpadded"
            )));
        }
    }
    // Reference-shaped fields, when present, must be governed refs (no .GOV /
    // SQLite / localhost / machine-local path).
    if let Some(receipts_ref) = &new.receipts_ref {
        reject_legacy_runtime_ref("receipts_ref", receipts_ref)?;
    }
    if let Some(evidence_ref) = &new.evidence_ref {
        reject_legacy_runtime_ref("evidence_ref", evidence_ref)?;
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// MT-148: DCC session/lease/command-log/recovery panel projections.
// ---------------------------------------------------------------------------

/// Kind of DCC (diagnostics control center) panel a projection row carries
/// (MT-148). Mirrors the `panel_kind` CHECK in migration 0115.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DccPanelKind {
    Session,
    Lease,
    CommandLog,
    Recovery,
}

impl DccPanelKind {
    pub fn as_token(self) -> &'static str {
        match self {
            DccPanelKind::Session => "SESSION",
            DccPanelKind::Lease => "LEASE",
            DccPanelKind::CommandLog => "COMMAND_LOG",
            DccPanelKind::Recovery => "RECOVERY",
        }
    }

    pub fn from_token(token: &str) -> AtelierResult<Self> {
        match token {
            "SESSION" => Ok(DccPanelKind::Session),
            "LEASE" => Ok(DccPanelKind::Lease),
            "COMMAND_LOG" => Ok(DccPanelKind::CommandLog),
            "RECOVERY" => Ok(DccPanelKind::Recovery),
            other => Err(AtelierError::Validation(format!(
                "unknown DCC panel kind: {other}"
            ))),
        }
    }

    /// All four panel kinds, for seeding/coverage.
    pub const ALL: &'static [DccPanelKind] = &[
        DccPanelKind::Session,
        DccPanelKind::Lease,
        DccPanelKind::CommandLog,
        DccPanelKind::Recovery,
    ];
}

/// Input to record one DCC panel projection row (MT-148).
#[derive(Clone, Debug)]
pub struct NewDccPanelProjection {
    /// Stable panel id (the PK). MUST be non-empty / unpadded.
    pub panel_id: String,
    pub panel_kind: DccPanelKind,
    /// Arbitrary typed panel state (session/lease/command-log/recovery payload).
    pub state_json: Value,
}

/// Persisted DCC panel projection row (MT-148).
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct DccPanelProjection {
    pub panel_id: String,
    pub panel_kind: DccPanelKind,
    pub state_json: Value,
    pub created_at_utc: DateTime<Utc>,
}

const DCC_PANEL_PROJECTION_COLUMNS: &str =
    "panel_id, panel_kind, state_json, created_at_utc";

fn dcc_panel_projection_from_row(row: &sqlx::postgres::PgRow) -> AtelierResult<DccPanelProjection> {
    let kind_token: String = row.get("panel_kind");
    Ok(DccPanelProjection {
        panel_id: row.get("panel_id"),
        panel_kind: DccPanelKind::from_token(&kind_token)?,
        state_json: row.get("state_json"),
        created_at_utc: row.get("created_at_utc"),
    })
}

// ---------------------------------------------------------------------------
// MT-153: screenshot artifact storage (governed artifact + metadata + retention).
// ---------------------------------------------------------------------------

/// Input to store a stealth screenshot capture as a governed, retained artifact
/// with diagnostic metadata (MT-153).
///
/// This EXTENDS the existing stealth capture receipt
/// ([`super::stealth_window::AtelierStore::record_stealth_capture`]): a capture
/// receipt proves a screenshot was produced, but carries no metadata or
/// retention policy. This row turns that capture into a described, retained
/// screenshot artifact (mime, dimensions, byte length, label, ttl, pinned),
/// referencing the governed ArtifactStore manifest id only (never raw pixels or
/// machine-local paths).
#[derive(Clone, Debug)]
pub struct NewScreenshotArtifactStorage {
    /// Stable storage id (the PK). MUST be non-empty / unpadded.
    pub storage_id: String,
    /// The stealth capture this storage row governs (one row per capture).
    pub capture_id: uuid::Uuid,
    /// Governed ArtifactStore manifest id of the screenshot (no path / SQLite / .GOV).
    pub artifact_manifest_id: String,
    /// Content hash of the stored screenshot payload. Non-empty.
    pub content_sha256: String,
    /// Image mime type (e.g. `image/png`). Non-empty.
    pub mime: String,
    pub width_px: Option<i32>,
    pub height_px: Option<i32>,
    pub byte_len: Option<i64>,
    /// Optional diagnostic label.
    pub label: Option<String>,
    /// Retention TTL in days; `None` = no automatic prune.
    pub retention_ttl_days: Option<i32>,
    /// Whether the artifact is pinned (exempt from retention prune).
    pub pinned: bool,
}

/// Persisted screenshot artifact-storage row (MT-153).
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ScreenshotArtifactStorage {
    pub storage_id: String,
    pub capture_id: uuid::Uuid,
    pub artifact_manifest_id: String,
    pub content_sha256: String,
    pub mime: String,
    pub width_px: Option<i32>,
    pub height_px: Option<i32>,
    pub byte_len: Option<i64>,
    pub label: Option<String>,
    pub retention_ttl_days: Option<i32>,
    pub pinned: bool,
    pub created_at_utc: DateTime<Utc>,
}

const SCREENSHOT_ARTIFACT_STORAGE_COLUMNS: &str =
    "storage_id, capture_id, artifact_manifest_id, content_sha256, mime, width_px, height_px, \
     byte_len, label, retention_ttl_days, pinned, created_at_utc";

fn screenshot_artifact_storage_from_row(row: &sqlx::postgres::PgRow) -> ScreenshotArtifactStorage {
    ScreenshotArtifactStorage {
        storage_id: row.get("storage_id"),
        capture_id: row.get("capture_id"),
        artifact_manifest_id: row.get("artifact_manifest_id"),
        content_sha256: row.get("content_sha256"),
        mime: row.get("mime"),
        width_px: row.get("width_px"),
        height_px: row.get("height_px"),
        byte_len: row.get("byte_len"),
        label: row.get("label"),
        retention_ttl_days: row.get("retention_ttl_days"),
        pinned: row.get("pinned"),
        created_at_utc: row.get("created_at_utc"),
    }
}

fn validate_screenshot_artifact_storage(
    new: &NewScreenshotArtifactStorage,
) -> AtelierResult<()> {
    for (field, value) in [
        ("storage_id", &new.storage_id),
        ("content_sha256", &new.content_sha256),
        ("mime", &new.mime),
    ] {
        if value.trim().is_empty() || value.trim() != value {
            return Err(AtelierError::Validation(format!(
                "screenshot artifact storage {field} must be non-empty and unpadded"
            )));
        }
    }
    // The artifact manifest id must be a governed, portable id (no .GOV / SQLite
    // / localhost / machine-local filesystem path).
    reject_legacy_runtime_ref("artifact_manifest_id", &new.artifact_manifest_id)?;
    if let Some(ttl) = new.retention_ttl_days {
        if ttl < 0 {
            return Err(AtelierError::Validation(
                "screenshot artifact storage retention_ttl_days must be >= 0".into(),
            ));
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// MT-167: stale README / spec drift detector.
// ---------------------------------------------------------------------------

/// Input to record one doc/spec drift finding (MT-167).
///
/// Generalizes the CKC README-vs-spec drift check: a finding ties a
/// doc-claimed pointer (`doc_ref`) to the spec/code surface it points at
/// (`spec_ref`), classifies the drift (`drift_kind`), and explains it
/// (`detail`).
#[derive(Clone, Debug)]
pub struct NewSpecDriftFinding {
    /// Stable finding id (the PK). MUST be non-empty / unpadded.
    pub finding_id: String,
    /// The doc/README surface that makes the claim. Non-empty.
    pub doc_ref: String,
    /// The spec/code surface the doc points at. Non-empty.
    pub spec_ref: String,
    /// Drift classification (e.g. `surface_mismatch`). Non-empty.
    pub drift_kind: String,
    /// Human-readable detail of the drift. Non-empty.
    pub detail: String,
}

/// Persisted doc/spec drift finding (MT-167).
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct SpecDriftFinding {
    pub finding_id: String,
    pub doc_ref: String,
    pub spec_ref: String,
    pub drift_kind: String,
    pub detail: String,
    pub created_at_utc: DateTime<Utc>,
}

const SPEC_DRIFT_FINDING_COLUMNS: &str =
    "finding_id, doc_ref, spec_ref, drift_kind, detail, created_at_utc";

fn spec_drift_finding_from_row(row: &sqlx::postgres::PgRow) -> SpecDriftFinding {
    SpecDriftFinding {
        finding_id: row.get("finding_id"),
        doc_ref: row.get("doc_ref"),
        spec_ref: row.get("spec_ref"),
        drift_kind: row.get("drift_kind"),
        detail: row.get("detail"),
        created_at_utc: row.get("created_at_utc"),
    }
}

fn validate_spec_drift_finding(new: &NewSpecDriftFinding) -> AtelierResult<()> {
    for (field, value) in [
        ("finding_id", &new.finding_id),
        ("doc_ref", &new.doc_ref),
        ("spec_ref", &new.spec_ref),
        ("drift_kind", &new.drift_kind),
        ("detail", &new.detail),
    ] {
        if value.trim().is_empty() || value.trim() != value {
            return Err(AtelierError::Validation(format!(
                "spec drift finding {field} must be non-empty and unpadded"
            )));
        }
    }
    Ok(())
}

impl AtelierStore {
    /// Record one model work-state projection row (MT-147).
    ///
    /// Idempotent on `projection_id`: re-recording updates the mutable fields
    /// and returns the row. Emits `WORK_STATE_PROJECTION_RECORDED`.
    pub async fn record_work_state_projection(
        &self,
        new: &NewWorkStateProjection,
    ) -> AtelierResult<WorkStateProjection> {
        validate_work_state_projection(new)?;

        let mut tx = self.pool().begin().await?;
        let row = sqlx::query(&format!(
            r#"INSERT INTO atelier_work_state_projection
                 (projection_id, active_mt, owner, status, blocker, receipts_ref,
                  next_action, evidence_ref)
               VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
               ON CONFLICT (projection_id) DO UPDATE SET
                 active_mt = EXCLUDED.active_mt,
                 owner = EXCLUDED.owner,
                 status = EXCLUDED.status,
                 blocker = EXCLUDED.blocker,
                 receipts_ref = EXCLUDED.receipts_ref,
                 next_action = EXCLUDED.next_action,
                 evidence_ref = EXCLUDED.evidence_ref
               RETURNING {WORK_STATE_PROJECTION_COLUMNS}"#
        ))
        .bind(&new.projection_id)
        .bind(&new.active_mt)
        .bind(&new.owner)
        .bind(&new.status)
        .bind(&new.blocker)
        .bind(&new.receipts_ref)
        .bind(&new.next_action)
        .bind(&new.evidence_ref)
        .fetch_one(&mut *tx)
        .await?;
        let recorded = work_state_projection_from_row(&row);

        self.record_event_in_tx(
            &mut tx,
            diagnostics_projection_event_family::WORK_STATE_PROJECTION_RECORDED,
            "atelier_work_state_projection",
            &recorded.projection_id,
            json!({
                "projection_id": recorded.projection_id,
                "active_mt": recorded.active_mt,
                "owner": recorded.owner,
                "status": recorded.status,
                "has_blocker": recorded.blocker.is_some(),
                "schema": "hsk.atelier.work_state_projection@1",
            }),
        )
        .await?;
        tx.commit().await?;
        Ok(recorded)
    }

    /// List model work-state projection rows, newest first.
    pub async fn list_work_state_projections(
        &self,
    ) -> AtelierResult<Vec<WorkStateProjection>> {
        let rows = sqlx::query(&format!(
            r#"SELECT {WORK_STATE_PROJECTION_COLUMNS}
               FROM atelier_work_state_projection
               ORDER BY created_at_utc DESC, projection_id ASC"#
        ))
        .fetch_all(self.pool())
        .await?;
        Ok(rows.iter().map(work_state_projection_from_row).collect())
    }

    /// Record one DCC panel projection row (MT-148).
    ///
    /// Idempotent on `panel_id`. Emits `DCC_PANEL_PROJECTION_RECORDED`.
    pub async fn record_dcc_panel_projection(
        &self,
        new: &NewDccPanelProjection,
    ) -> AtelierResult<DccPanelProjection> {
        if new.panel_id.trim().is_empty() || new.panel_id.trim() != new.panel_id {
            return Err(AtelierError::Validation(
                "dcc panel projection panel_id must be non-empty and unpadded".into(),
            ));
        }

        let mut tx = self.pool().begin().await?;
        let row = sqlx::query(&format!(
            r#"INSERT INTO atelier_dcc_panel_projection
                 (panel_id, panel_kind, state_json)
               VALUES ($1, $2, $3::jsonb)
               ON CONFLICT (panel_id) DO UPDATE SET
                 panel_kind = EXCLUDED.panel_kind,
                 state_json = EXCLUDED.state_json
               RETURNING {DCC_PANEL_PROJECTION_COLUMNS}"#
        ))
        .bind(&new.panel_id)
        .bind(new.panel_kind.as_token())
        .bind(&new.state_json)
        .fetch_one(&mut *tx)
        .await?;
        let recorded = dcc_panel_projection_from_row(&row)?;

        self.record_event_in_tx(
            &mut tx,
            diagnostics_projection_event_family::DCC_PANEL_PROJECTION_RECORDED,
            "atelier_dcc_panel_projection",
            &recorded.panel_id,
            json!({
                "panel_id": recorded.panel_id,
                "panel_kind": recorded.panel_kind.as_token(),
                "schema": "hsk.atelier.dcc_panel_projection@1",
            }),
        )
        .await?;
        tx.commit().await?;
        Ok(recorded)
    }

    /// List DCC panel projection rows for a given panel kind, newest first
    /// (MT-148).
    pub async fn list_dcc_panel_projections_by_kind(
        &self,
        panel_kind: DccPanelKind,
    ) -> AtelierResult<Vec<DccPanelProjection>> {
        let rows = sqlx::query(&format!(
            r#"SELECT {DCC_PANEL_PROJECTION_COLUMNS}
               FROM atelier_dcc_panel_projection
               WHERE panel_kind = $1
               ORDER BY created_at_utc DESC, panel_id ASC"#
        ))
        .bind(panel_kind.as_token())
        .fetch_all(self.pool())
        .await?;
        rows.iter().map(dcc_panel_projection_from_row).collect()
    }

    /// Store a stealth screenshot capture as a governed, retained artifact with
    /// metadata (MT-153).
    ///
    /// Idempotent on `capture_id` (one storage row per capture): re-recording
    /// updates the metadata/retention in place. The artifact manifest id is
    /// validated as a governed ref (no .GOV / SQLite / localhost / machine-local
    /// path); no raw pixels or paths are stored. Emits
    /// `SCREENSHOT_ARTIFACT_STORED`.
    pub async fn record_screenshot_artifact_storage(
        &self,
        new: &NewScreenshotArtifactStorage,
    ) -> AtelierResult<ScreenshotArtifactStorage> {
        validate_screenshot_artifact_storage(new)?;

        let mut tx = self.pool().begin().await?;
        let row = sqlx::query(&format!(
            r#"INSERT INTO atelier_screenshot_artifact_storage
                 (storage_id, capture_id, artifact_manifest_id, content_sha256, mime,
                  width_px, height_px, byte_len, label, retention_ttl_days, pinned)
               VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
               ON CONFLICT (capture_id) DO UPDATE SET
                 artifact_manifest_id = EXCLUDED.artifact_manifest_id,
                 content_sha256 = EXCLUDED.content_sha256,
                 mime = EXCLUDED.mime,
                 width_px = EXCLUDED.width_px,
                 height_px = EXCLUDED.height_px,
                 byte_len = EXCLUDED.byte_len,
                 label = EXCLUDED.label,
                 retention_ttl_days = EXCLUDED.retention_ttl_days,
                 pinned = EXCLUDED.pinned
               RETURNING {SCREENSHOT_ARTIFACT_STORAGE_COLUMNS}"#
        ))
        .bind(&new.storage_id)
        .bind(new.capture_id)
        .bind(&new.artifact_manifest_id)
        .bind(&new.content_sha256)
        .bind(&new.mime)
        .bind(new.width_px)
        .bind(new.height_px)
        .bind(new.byte_len)
        .bind(&new.label)
        .bind(new.retention_ttl_days)
        .bind(new.pinned)
        .fetch_one(&mut *tx)
        .await?;
        let recorded = screenshot_artifact_storage_from_row(&row);

        self.record_event_in_tx(
            &mut tx,
            diagnostics_projection_event_family::SCREENSHOT_ARTIFACT_STORED,
            "atelier_screenshot_artifact_storage",
            &recorded.storage_id,
            json!({
                "storage_id": recorded.storage_id,
                "capture_id": recorded.capture_id,
                "artifact_manifest_id": recorded.artifact_manifest_id,
                "content_sha256": recorded.content_sha256,
                "mime": recorded.mime,
                "retention_ttl_days": recorded.retention_ttl_days,
                "pinned": recorded.pinned,
                "schema": "hsk.atelier.screenshot_artifact_storage@1",
            }),
        )
        .await?;
        tx.commit().await?;
        Ok(recorded)
    }

    /// List screenshot artifact-storage rows for a window's captures, newest
    /// first (MT-153). Filtered by `capture_id` set is left to callers; this
    /// returns all storage rows ordered for deterministic, run-scoped tests.
    pub async fn list_screenshot_artifact_storage(
        &self,
    ) -> AtelierResult<Vec<ScreenshotArtifactStorage>> {
        let rows = sqlx::query(&format!(
            r#"SELECT {SCREENSHOT_ARTIFACT_STORAGE_COLUMNS}
               FROM atelier_screenshot_artifact_storage
               ORDER BY created_at_utc DESC, storage_id ASC"#
        ))
        .fetch_all(self.pool())
        .await?;
        Ok(rows
            .iter()
            .map(screenshot_artifact_storage_from_row)
            .collect())
    }

    /// Record one doc/spec drift finding (MT-167).
    ///
    /// Idempotent on `finding_id`. Emits `SPEC_DRIFT_FINDING_RECORDED`.
    pub async fn record_spec_drift_finding(
        &self,
        new: &NewSpecDriftFinding,
    ) -> AtelierResult<SpecDriftFinding> {
        validate_spec_drift_finding(new)?;

        let mut tx = self.pool().begin().await?;
        let row = sqlx::query(&format!(
            r#"INSERT INTO atelier_spec_drift_finding
                 (finding_id, doc_ref, spec_ref, drift_kind, detail)
               VALUES ($1, $2, $3, $4, $5)
               ON CONFLICT (finding_id) DO UPDATE SET
                 doc_ref = EXCLUDED.doc_ref,
                 spec_ref = EXCLUDED.spec_ref,
                 drift_kind = EXCLUDED.drift_kind,
                 detail = EXCLUDED.detail
               RETURNING {SPEC_DRIFT_FINDING_COLUMNS}"#
        ))
        .bind(&new.finding_id)
        .bind(&new.doc_ref)
        .bind(&new.spec_ref)
        .bind(&new.drift_kind)
        .bind(&new.detail)
        .fetch_one(&mut *tx)
        .await?;
        let recorded = spec_drift_finding_from_row(&row);

        self.record_event_in_tx(
            &mut tx,
            diagnostics_projection_event_family::SPEC_DRIFT_FINDING_RECORDED,
            "atelier_spec_drift_finding",
            &recorded.finding_id,
            json!({
                "finding_id": recorded.finding_id,
                "doc_ref": recorded.doc_ref,
                "spec_ref": recorded.spec_ref,
                "drift_kind": recorded.drift_kind,
                "schema": "hsk.atelier.spec_drift_finding@1",
            }),
        )
        .await?;
        tx.commit().await?;
        Ok(recorded)
    }

    /// List doc/spec drift findings, newest first (MT-167).
    pub async fn list_spec_drift_findings(&self) -> AtelierResult<Vec<SpecDriftFinding>> {
        let rows = sqlx::query(&format!(
            r#"SELECT {SPEC_DRIFT_FINDING_COLUMNS}
               FROM atelier_spec_drift_finding
               ORDER BY created_at_utc DESC, finding_id ASC"#
        ))
        .fetch_all(self.pool())
        .await?;
        Ok(rows.iter().map(spec_drift_finding_from_row).collect())
    }

    /// Detect doc/spec drift and record a finding ONLY when they differ (MT-167).
    ///
    /// Generalizes the CKC README-vs-spec check: compares a doc-claimed surface
    /// string against the actual spec/code surface string. When they match,
    /// returns `Ok(None)` and records nothing. When they differ, records a typed
    /// drift finding (with the supplied `drift_kind` and an auto-built `detail`
    /// describing the mismatch) and returns `Ok(Some(finding))`.
    pub async fn detect_and_record_spec_drift(
        &self,
        finding_id: &str,
        doc_ref: &str,
        spec_ref: &str,
        doc_surface: &str,
        code_surface: &str,
        drift_kind: &str,
    ) -> AtelierResult<Option<SpecDriftFinding>> {
        if doc_surface == code_surface {
            return Ok(None);
        }
        let detail = format!(
            "doc surface {doc_surface:?} (claimed at {doc_ref}) does not match \
             code/spec surface {code_surface:?} (at {spec_ref})"
        );
        let finding = self
            .record_spec_drift_finding(&NewSpecDriftFinding {
                finding_id: finding_id.to_string(),
                doc_ref: doc_ref.to_string(),
                spec_ref: spec_ref.to_string(),
                drift_kind: drift_kind.to_string(),
                detail,
            })
            .await?;
        Ok(Some(finding))
    }
}
