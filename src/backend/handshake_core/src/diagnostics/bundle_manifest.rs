//! WP-KERNEL-005 MT-141: Kernel Diagnostic Bundle Manifest schema + builder.
//!
//! The kernel-level diagnostic bundle manifest is the single record a
//! no-context model reads to isolate a failure without re-running it. It is
//! distinct from MT-112's pose/ComfyUI failure bundle
//! (`atelier::comfy::DiagnosticBundle`), which is scoped to one workflow run;
//! this manifest covers any kernel failure subject (job, session run, build,
//! workflow transition, capability call, ...).
//!
//! A manifest carries:
//!   * what failed (`subject_kind` + portable `subject_ref`),
//!   * a one-line `failure_summary` and stable `error_taxonomy` token,
//!   * a canonical [`DiagnosticSeverity`],
//!   * ordered evidence [`DiagnosticBundleSection`]s (diagnostics,
//!     event-ledger, state-probe, logs, environment, artifacts), each with a
//!     portable `content_ref` and/or inline `content_json`,
//!   * deterministic `reproduction_steps`,
//!   * ordered `isolation_hints` (what to check first).
//!
//! Storage authority is PostgreSQL only (table
//! `kernel_diagnostic_bundle_manifest`, migration 0120, applied by the kernel
//! migration runner `Database::run_migrations`). Recording a manifest emits
//! the `kernel.diagnostics.bundle_manifest_recorded` EventLedger family on the
//! `kernel_diagnostic_bundle_manifest` aggregate.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::Row;
use uuid::Uuid;

use crate::atelier::{reject_legacy_runtime_ref, AtelierError, AtelierResult, AtelierStore};

use super::DiagnosticSeverity;

/// Stable schema id stamped on every persisted manifest row.
pub const KERNEL_DIAGNOSTIC_BUNDLE_MANIFEST_SCHEMA: &str =
    "hsk.kernel.diagnostic_bundle_manifest@1";

pub mod kernel_diagnostic_bundle_event_family {
    pub const BUNDLE_MANIFEST_RECORDED: &str = "kernel.diagnostics.bundle_manifest_recorded";

    pub const ALL: &[&str] = &[BUNDLE_MANIFEST_RECORDED];
}

pub use kernel_diagnostic_bundle_event_family::BUNDLE_MANIFEST_RECORDED;

/// What kind of evidence a manifest section carries.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DiagnosticBundleSectionKind {
    /// Structured diagnostics (fingerprints, problem groups).
    Diagnostics,
    /// EventLedger evidence (aggregate ids, event families).
    EventLedger,
    /// State-probe snapshots of runtime state.
    StateProbe,
    /// Log evidence behind a portable content_ref.
    Logs,
    /// Environment / version pins in effect at failure.
    Environment,
    /// Artifacts involved in the failed operation.
    Artifacts,
}

impl DiagnosticBundleSectionKind {
    pub fn as_token(self) -> &'static str {
        match self {
            DiagnosticBundleSectionKind::Diagnostics => "DIAGNOSTICS",
            DiagnosticBundleSectionKind::EventLedger => "EVENT_LEDGER",
            DiagnosticBundleSectionKind::StateProbe => "STATE_PROBE",
            DiagnosticBundleSectionKind::Logs => "LOGS",
            DiagnosticBundleSectionKind::Environment => "ENVIRONMENT",
            DiagnosticBundleSectionKind::Artifacts => "ARTIFACTS",
        }
    }

    pub fn from_token(token: &str) -> AtelierResult<Self> {
        match token {
            "DIAGNOSTICS" => Ok(DiagnosticBundleSectionKind::Diagnostics),
            "EVENT_LEDGER" => Ok(DiagnosticBundleSectionKind::EventLedger),
            "STATE_PROBE" => Ok(DiagnosticBundleSectionKind::StateProbe),
            "LOGS" => Ok(DiagnosticBundleSectionKind::Logs),
            "ENVIRONMENT" => Ok(DiagnosticBundleSectionKind::Environment),
            "ARTIFACTS" => Ok(DiagnosticBundleSectionKind::Artifacts),
            other => Err(AtelierError::Validation(format!(
                "unknown diagnostic bundle section kind: {other}"
            ))),
        }
    }
}

/// One ordered evidence section inside a kernel diagnostic bundle manifest.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct DiagnosticBundleSection {
    /// Unique (within the manifest) stable section token.
    pub section_id: String,
    pub kind: DiagnosticBundleSectionKind,
    pub title: String,
    /// Portable ref to the section's evidence body (e.g. `artifact://...`).
    /// Never a machine-local path, `.GOV` ref, or SQLite ref.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content_ref: Option<String>,
    /// Inline structured evidence (JSON object or array).
    pub content_json: Value,
    /// Number of evidence items the section carries (rows, events, files...).
    pub item_count: i64,
}

/// Input for recording a kernel diagnostic bundle manifest.
#[derive(Clone, Debug)]
pub struct NewDiagnosticBundleManifest {
    /// What kind of subject failed (e.g. `kernel_job`, `session_run`, `build`).
    pub subject_kind: String,
    /// Portable token identifying the failing subject.
    pub subject_ref: String,
    pub failure_summary: String,
    /// Stable error-taxonomy token classifying the failure.
    pub error_taxonomy: String,
    pub severity: DiagnosticSeverity,
    pub created_by: String,
    pub sections: Vec<DiagnosticBundleSection>,
    /// Deterministic steps a no-context model runs to reproduce the failure.
    pub reproduction_steps: Vec<String>,
    /// Ordered check-first hints for isolating the failure.
    pub isolation_hints: Vec<String>,
}

/// A persisted kernel diagnostic bundle manifest.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct DiagnosticBundleManifest {
    pub manifest_id: Uuid,
    pub schema_id: String,
    pub subject_kind: String,
    pub subject_ref: String,
    pub failure_summary: String,
    pub error_taxonomy: String,
    pub severity: DiagnosticSeverity,
    pub created_by: String,
    pub sections: Vec<DiagnosticBundleSection>,
    pub reproduction_steps: Vec<String>,
    pub isolation_hints: Vec<String>,
    pub created_at_utc: DateTime<Utc>,
}

fn require_token(field: &str, value: &str) -> AtelierResult<()> {
    if value.trim().is_empty() || value.trim() != value {
        return Err(AtelierError::Validation(format!(
            "{field} must not be empty or padded"
        )));
    }
    Ok(())
}

/// Reject machine-local / `.GOV` / SQLite refs anywhere inside inline section
/// content. Only string values that look like refs (contain a scheme) or carry
/// forbidden storage tokens are checked, so plain prose summaries stay legal.
fn reject_nonportable_strings_in_json(field: &str, value: &Value) -> AtelierResult<()> {
    match value {
        Value::String(text) => {
            let lower = text.to_ascii_lowercase();
            if text.contains("://") || lower.contains(".gov") || lower.contains("sqlite") {
                reject_legacy_runtime_ref(field, text)?;
            }
            Ok(())
        }
        Value::Array(items) => {
            for item in items {
                reject_nonportable_strings_in_json(field, item)?;
            }
            Ok(())
        }
        Value::Object(map) => {
            for item in map.values() {
                reject_nonportable_strings_in_json(field, item)?;
            }
            Ok(())
        }
        _ => Ok(()),
    }
}

/// Validate a manifest input so every persisted manifest is usable by a
/// no-context model: portable subject, at least one evidence section with
/// unique ids and portable refs, at least one reproduction step, and at least
/// one isolation hint.
pub fn validate_diagnostic_bundle_manifest(new: &NewDiagnosticBundleManifest) -> AtelierResult<()> {
    require_token("subject_kind", &new.subject_kind)?;
    reject_legacy_runtime_ref("diagnostic bundle manifest subject_ref", &new.subject_ref)?;
    require_token("failure_summary", &new.failure_summary)?;
    require_token("error_taxonomy", &new.error_taxonomy)?;
    require_token("created_by", &new.created_by)?;

    if new.sections.is_empty() {
        return Err(AtelierError::Validation(
            "diagnostic bundle manifest must include at least one evidence section".into(),
        ));
    }
    let section_ids: std::collections::HashSet<&str> = new
        .sections
        .iter()
        .map(|section| section.section_id.as_str())
        .collect();
    if section_ids.len() != new.sections.len() {
        return Err(AtelierError::Validation(
            "diagnostic bundle manifest section_id values must be unique".into(),
        ));
    }
    for section in &new.sections {
        require_token("section_id", &section.section_id)?;
        require_token("section title", &section.title)?;
        if let Some(content_ref) = section.content_ref.as_deref() {
            reject_legacy_runtime_ref("diagnostic bundle section content_ref", content_ref)?;
        }
        if !(section.content_json.is_object() || section.content_json.is_array()) {
            return Err(AtelierError::Validation(format!(
                "section {} content_json must be a JSON object or array",
                section.section_id
            )));
        }
        reject_nonportable_strings_in_json(
            "diagnostic bundle section content_json",
            &section.content_json,
        )?;
        if section.item_count < 0 {
            return Err(AtelierError::Validation(format!(
                "section {} item_count must not be negative",
                section.section_id
            )));
        }
        if section.content_ref.is_none()
            && section.content_json.as_object().is_some_and(|m| m.is_empty())
        {
            return Err(AtelierError::Validation(format!(
                "section {} must carry a content_ref or non-empty content_json",
                section.section_id
            )));
        }
    }

    if new.reproduction_steps.is_empty() {
        return Err(AtelierError::Validation(
            "diagnostic bundle manifest must include at least one reproduction step".into(),
        ));
    }
    for step in &new.reproduction_steps {
        require_token("reproduction step", step)?;
    }
    if new.isolation_hints.is_empty() {
        return Err(AtelierError::Validation(
            "diagnostic bundle manifest must include at least one isolation hint".into(),
        ));
    }
    for hint in &new.isolation_hints {
        require_token("isolation hint", hint)?;
    }
    Ok(())
}

fn string_array_from_json(field: &str, value: Value) -> AtelierResult<Vec<String>> {
    serde_json::from_value(value).map_err(|err| {
        AtelierError::Validation(format!("{field}: expected JSON string array: {err}"))
    })
}

fn manifest_from_row(row: &sqlx::postgres::PgRow) -> AtelierResult<DiagnosticBundleManifest> {
    let severity_token: String = row.get("severity");
    let severity = severity_token
        .parse::<DiagnosticSeverity>()
        .map_err(|err| AtelierError::Validation(err.to_string()))?;
    let sections: Vec<DiagnosticBundleSection> =
        serde_json::from_value(row.get::<Value, _>("sections_json")).map_err(|err| {
            AtelierError::Validation(format!("sections_json: invalid section payload: {err}"))
        })?;
    Ok(DiagnosticBundleManifest {
        manifest_id: row.get("manifest_id"),
        schema_id: row.get("schema_id"),
        subject_kind: row.get("subject_kind"),
        subject_ref: row.get("subject_ref"),
        failure_summary: row.get("failure_summary"),
        error_taxonomy: row.get("error_taxonomy"),
        severity,
        created_by: row.get("created_by"),
        sections,
        reproduction_steps: string_array_from_json(
            "reproduction_json",
            row.get::<Value, _>("reproduction_json"),
        )?,
        isolation_hints: string_array_from_json(
            "isolation_json",
            row.get::<Value, _>("isolation_json"),
        )?,
        created_at_utc: row.get("created_at_utc"),
    })
}

impl AtelierStore {
    /// Validate and persist a kernel diagnostic bundle manifest, emitting the
    /// `kernel.diagnostics.bundle_manifest_recorded` EventLedger family in the
    /// same transaction.
    pub async fn record_kernel_diagnostic_bundle_manifest(
        &self,
        new: &NewDiagnosticBundleManifest,
    ) -> AtelierResult<DiagnosticBundleManifest> {
        validate_diagnostic_bundle_manifest(new)?;

        let sections_json = serde_json::to_value(&new.sections)
            .map_err(|err| AtelierError::Validation(err.to_string()))?;
        let reproduction_json = serde_json::to_value(&new.reproduction_steps)
            .map_err(|err| AtelierError::Validation(err.to_string()))?;
        let isolation_json = serde_json::to_value(&new.isolation_hints)
            .map_err(|err| AtelierError::Validation(err.to_string()))?;

        let mut tx = self.pool().begin().await?;
        let row = sqlx::query(
            r#"INSERT INTO kernel_diagnostic_bundle_manifest
                 (schema_id, subject_kind, subject_ref, failure_summary,
                  error_taxonomy, severity, created_by, sections_json,
                  reproduction_json, isolation_json)
               VALUES ($1, $2, $3, $4, $5, $6, $7, $8::jsonb, $9::jsonb, $10::jsonb)
               RETURNING manifest_id, schema_id, subject_kind, subject_ref,
                         failure_summary, error_taxonomy, severity, created_by,
                         sections_json, reproduction_json, isolation_json,
                         created_at_utc"#,
        )
        .bind(KERNEL_DIAGNOSTIC_BUNDLE_MANIFEST_SCHEMA)
        .bind(&new.subject_kind)
        .bind(&new.subject_ref)
        .bind(&new.failure_summary)
        .bind(&new.error_taxonomy)
        .bind(new.severity.as_str())
        .bind(&new.created_by)
        .bind(&sections_json)
        .bind(&reproduction_json)
        .bind(&isolation_json)
        .fetch_one(&mut *tx)
        .await?;
        let manifest = manifest_from_row(&row)?;

        self.record_event_in_tx(
            &mut tx,
            kernel_diagnostic_bundle_event_family::BUNDLE_MANIFEST_RECORDED,
            "kernel_diagnostic_bundle_manifest",
            &manifest.manifest_id.to_string(),
            serde_json::json!({
                "manifest_id": manifest.manifest_id,
                "schema": KERNEL_DIAGNOSTIC_BUNDLE_MANIFEST_SCHEMA,
                "subject_kind": manifest.subject_kind,
                "subject_ref": manifest.subject_ref,
                "error_taxonomy": manifest.error_taxonomy,
                "severity": manifest.severity.as_str(),
                "section_count": manifest.sections.len(),
                "reproduction_step_count": manifest.reproduction_steps.len(),
            }),
        )
        .await?;
        tx.commit().await?;
        Ok(manifest)
    }

    /// Fetch a kernel diagnostic bundle manifest by id, if recorded.
    pub async fn get_kernel_diagnostic_bundle_manifest(
        &self,
        manifest_id: Uuid,
    ) -> AtelierResult<Option<DiagnosticBundleManifest>> {
        let row = sqlx::query(
            r#"SELECT manifest_id, schema_id, subject_kind, subject_ref,
                      failure_summary, error_taxonomy, severity, created_by,
                      sections_json, reproduction_json, isolation_json,
                      created_at_utc
               FROM kernel_diagnostic_bundle_manifest
               WHERE manifest_id = $1"#,
        )
        .bind(manifest_id)
        .fetch_optional(self.pool())
        .await?;
        row.as_ref().map(manifest_from_row).transpose()
    }

    /// List manifests for a failing subject, newest first, so a no-context
    /// model can find the latest failure evidence by subject token alone.
    pub async fn list_kernel_diagnostic_bundle_manifests_for_subject(
        &self,
        subject_kind: &str,
        subject_ref: &str,
    ) -> AtelierResult<Vec<DiagnosticBundleManifest>> {
        let rows = sqlx::query(
            r#"SELECT manifest_id, schema_id, subject_kind, subject_ref,
                      failure_summary, error_taxonomy, severity, created_by,
                      sections_json, reproduction_json, isolation_json,
                      created_at_utc
               FROM kernel_diagnostic_bundle_manifest
               WHERE subject_kind = $1 AND subject_ref = $2
               ORDER BY created_at_utc DESC, manifest_id DESC"#,
        )
        .bind(subject_kind)
        .bind(subject_ref)
        .fetch_all(self.pool())
        .await?;
        rows.iter().map(manifest_from_row).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn sample_manifest() -> NewDiagnosticBundleManifest {
        NewDiagnosticBundleManifest {
            subject_kind: "kernel_job".to_string(),
            subject_ref: "kernel-job://run/sample".to_string(),
            failure_summary: "sample failure".to_string(),
            error_taxonomy: "kernel.sample_failure".to_string(),
            severity: DiagnosticSeverity::Error,
            created_by: "unit-test".to_string(),
            sections: vec![DiagnosticBundleSection {
                section_id: "diagnostics".to_string(),
                kind: DiagnosticBundleSectionKind::Diagnostics,
                title: "Open diagnostics".to_string(),
                content_ref: Some("artifact://diagnostics/sample".to_string()),
                content_json: json!({ "fingerprints": ["f1"] }),
                item_count: 1,
            }],
            reproduction_steps: vec!["cargo test sample".to_string()],
            isolation_hints: vec!["check the diagnostics section first".to_string()],
        }
    }

    #[test]
    fn validation_accepts_a_complete_manifest() {
        validate_diagnostic_bundle_manifest(&sample_manifest()).expect("valid manifest");
    }

    #[test]
    fn validation_rejects_gov_refs_and_missing_steps() {
        let mut gov_ref = sample_manifest();
        gov_ref.sections[0].content_ref = Some(".GOV/task_packets/WP-KERNEL-005".to_string());
        assert!(validate_diagnostic_bundle_manifest(&gov_ref).is_err());

        let mut inline_gov = sample_manifest();
        inline_gov.sections[0].content_json = json!({ "ref": "sqlite://local.db" });
        assert!(validate_diagnostic_bundle_manifest(&inline_gov).is_err());

        let mut no_steps = sample_manifest();
        no_steps.reproduction_steps.clear();
        assert!(validate_diagnostic_bundle_manifest(&no_steps).is_err());

        let mut no_sections = sample_manifest();
        no_sections.sections.clear();
        assert!(validate_diagnostic_bundle_manifest(&no_sections).is_err());
    }

    #[test]
    fn section_kind_tokens_round_trip() {
        for kind in [
            DiagnosticBundleSectionKind::Diagnostics,
            DiagnosticBundleSectionKind::EventLedger,
            DiagnosticBundleSectionKind::StateProbe,
            DiagnosticBundleSectionKind::Logs,
            DiagnosticBundleSectionKind::Environment,
            DiagnosticBundleSectionKind::Artifacts,
        ] {
            assert_eq!(
                DiagnosticBundleSectionKind::from_token(kind.as_token()).expect("token"),
                kind
            );
        }
    }
}
