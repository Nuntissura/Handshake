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
    /// (WP-KERNEL-005 MT-171..MT-175).
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
// WP-KERNEL-005 MT-171..MT-175: diagnostics "validation matrix".
//
// Each row asserts that one required check on a model-workflow diagnostic
// surface is REQUIRED, COVERED, or DEFERRED. This is the typed runtime surface
// that turns "diagnostics are covered" into a real PostgreSQL row + EventLedger
// event, never governance markdown. The catalog
// [`diagnostics_validation_matrix_catalog`] carries the real check rows for all
// five MT areas, grounded in the existing product modules they cite.
// ---------------------------------------------------------------------------

/// Matrix kind, one per MT area (171..175). Used to group + filter rows.
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
    if let Some(evidence_ref) = &new.evidence_ref {
        // Reuse the canonical .GOV/SQLite/localhost/local-path rejection boundary.
        reject_legacy_runtime_ref("evidence_ref", evidence_ref)?;
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

/// The real diagnostics validation-matrix rows for all five MT areas.
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
    ]
}

impl AtelierStore {
    /// Record one diagnostics validation-matrix row (MT-171..MT-175).
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
