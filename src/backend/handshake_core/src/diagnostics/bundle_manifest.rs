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
//! Storage authority is the embedded SurrealDB table
//! `kernel_diagnostic_bundle_manifest`. Recording a manifest emits
//! the `kernel.diagnostics.bundle_manifest_recorded` EventLedger family on the
//! `kernel_diagnostic_bundle_manifest` aggregate.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use surrealdb::types::{Datetime, RecordId, SurrealValue, Uuid as SurrealUuid, Value};
use uuid::Uuid;

use crate::atelier::{
    atelier_event_sql, reject_legacy_runtime_ref, AtelierError, AtelierResult, AtelierStore,
};

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
    pub content_json: JsonValue,
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
fn reject_nonportable_strings_in_json(field: &str, value: &JsonValue) -> AtelierResult<()> {
    match value {
        JsonValue::String(text) => {
            let lower = text.to_ascii_lowercase();
            if text.contains("://") || lower.contains(".gov") || lower.contains("sqlite") {
                reject_legacy_runtime_ref(field, text)?;
            }
            Ok(())
        }
        JsonValue::Array(items) => {
            for item in items {
                reject_nonportable_strings_in_json(field, item)?;
            }
            Ok(())
        }
        JsonValue::Object(map) => {
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
            && section
                .content_json
                .as_object()
                .is_some_and(|m| m.is_empty())
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

#[derive(SurrealValue)]
struct DiagnosticManifestRow {
    manifest_id: SurrealUuid,
    schema_id: String,
    subject_kind: String,
    subject_ref: String,
    failure_summary: String,
    error_taxonomy: String,
    severity: String,
    created_by: String,
    sections_json: JsonValue,
    reproduction_json: Vec<String>,
    isolation_json: Vec<String>,
    created_at_utc: Datetime,
}

impl TryFrom<DiagnosticManifestRow> for DiagnosticBundleManifest {
    type Error = AtelierError;

    fn try_from(row: DiagnosticManifestRow) -> AtelierResult<Self> {
        let severity = row
            .severity
            .parse::<DiagnosticSeverity>()
            .map_err(|err| AtelierError::Validation(err.to_string()))?;
        let sections: Vec<DiagnosticBundleSection> = serde_json::from_value(row.sections_json)
            .map_err(|err| {
                AtelierError::Validation(format!("sections_json: invalid section payload: {err}"))
            })?;
        Ok(DiagnosticBundleManifest {
            manifest_id: row.manifest_id.into(),
            schema_id: row.schema_id,
            subject_kind: row.subject_kind,
            subject_ref: row.subject_ref,
            failure_summary: row.failure_summary,
            error_taxonomy: row.error_taxonomy,
            severity,
            created_by: row.created_by,
            sections,
            reproduction_steps: row.reproduction_json,
            isolation_hints: row.isolation_json,
            created_at_utc: row.created_at_utc.into(),
        })
    }
}

#[derive(Clone, SurrealValue)]
struct DiagnosticManifestBindings {
    record_id: RecordId,
    manifest_id: SurrealUuid,
    schema_id: String,
    subject_kind: String,
    subject_ref: String,
    failure_summary: String,
    error_taxonomy: String,
    severity: String,
    created_by: String,
    sections_json: Value,
    reproduction_json: Vec<String>,
    isolation_json: Vec<String>,
}

#[derive(SurrealValue)]
struct ManifestIdBinding {
    manifest_id: SurrealUuid,
}

#[derive(SurrealValue)]
struct ManifestSubjectBindings {
    subject_kind: String,
    subject_ref: String,
}

const RECORD_MANIFEST_STATEMENT: &str = concat!(
    "RETURN { LET $manifest = (CREATE $domain.record_id CONTENT { \
       manifest_id: $domain.manifest_id, schema_id: $domain.schema_id, \
       subject_kind: $domain.subject_kind, subject_ref: $domain.subject_ref, \
       failure_summary: $domain.failure_summary, error_taxonomy: $domain.error_taxonomy, \
       severity: $domain.severity, created_by: $domain.created_by, \
       sections_json: $domain.sections_json, reproduction_json: $domain.reproduction_json, \
       isolation_json: $domain.isolation_json } RETURN AFTER)[0]; ",
    atelier_event_sql!(),
    " RETURN $manifest; };"
);

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
        let manifest_id = Uuid::now_v7();
        let bindings = DiagnosticManifestBindings {
            record_id: RecordId::new(
                "kernel_diagnostic_bundle_manifest",
                SurrealUuid::from(manifest_id),
            ),
            manifest_id: manifest_id.into(),
            schema_id: KERNEL_DIAGNOSTIC_BUNDLE_MANIFEST_SCHEMA.to_owned(),
            subject_kind: new.subject_kind.clone(),
            subject_ref: new.subject_ref.clone(),
            failure_summary: new.failure_summary.clone(),
            error_taxonomy: new.error_taxonomy.clone(),
            severity: new.severity.as_str().to_owned(),
            created_by: new.created_by.clone(),
            sections_json: SurrealValue::into_value(sections_json),
            reproduction_json: new.reproduction_steps.clone(),
            isolation_json: new.isolation_hints.clone(),
        };
        let row: Option<DiagnosticManifestRow> = self
            .write_with_event(
                RECORD_MANIFEST_STATEMENT,
                bindings,
                kernel_diagnostic_bundle_event_family::BUNDLE_MANIFEST_RECORDED,
                "kernel_diagnostic_bundle_manifest",
                &manifest_id.to_string(),
                serde_json::json!({
                    "manifest_id": manifest_id,
                    "schema": KERNEL_DIAGNOSTIC_BUNDLE_MANIFEST_SCHEMA,
                    "subject_kind": new.subject_kind,
                    "subject_ref": new.subject_ref,
                    "error_taxonomy": new.error_taxonomy,
                    "severity": new.severity.as_str(),
                    "section_count": new.sections.len(),
                    "reproduction_step_count": new.reproduction_steps.len(),
                }),
            )
            .await?;
        row.ok_or_else(|| {
            AtelierError::Internal("recording diagnostic manifest returned no row".to_owned())
        })?
        .try_into()
    }

    /// Fetch a kernel diagnostic bundle manifest by id, if recorded.
    pub async fn get_kernel_diagnostic_bundle_manifest(
        &self,
        manifest_id: Uuid,
    ) -> AtelierResult<Option<DiagnosticBundleManifest>> {
        let row: Option<DiagnosticManifestRow> = self
            .store()
            .with_data_operation(move |ctx| {
                Box::pin(async move {
                    ctx.query_first(
                        "SELECT manifest_id, schema_id, subject_kind, subject_ref, \
                         failure_summary, error_taxonomy, severity, created_by, sections_json, \
                         reproduction_json, isolation_json, created_at_utc \
                         FROM kernel_diagnostic_bundle_manifest WHERE manifest_id = $manifest_id LIMIT 1;",
                        ManifestIdBinding { manifest_id: manifest_id.into() },
                    ).await
                })
            }).await?;
        row.map(TryInto::try_into).transpose()
    }

    /// List manifests for a failing subject, newest first, so a no-context
    /// model can find the latest failure evidence by subject token alone.
    pub async fn list_kernel_diagnostic_bundle_manifests_for_subject(
        &self,
        subject_kind: &str,
        subject_ref: &str,
    ) -> AtelierResult<Vec<DiagnosticBundleManifest>> {
        let bindings = ManifestSubjectBindings {
            subject_kind: subject_kind.to_owned(),
            subject_ref: subject_ref.to_owned(),
        };
        let rows: Vec<DiagnosticManifestRow> = self
            .store()
            .with_data_operation(move |ctx| {
                Box::pin(async move {
                    ctx.query_values(
                    "SELECT manifest_id, schema_id, subject_kind, subject_ref, failure_summary, \
                     error_taxonomy, severity, created_by, sections_json, reproduction_json, \
                     isolation_json, created_at_utc FROM kernel_diagnostic_bundle_manifest \
                     WHERE subject_kind = $subject_kind AND subject_ref = $subject_ref \
                     ORDER BY created_at_utc DESC, manifest_id DESC;",
                    bindings,
                ).await
                })
            })
            .await?;
        rows.into_iter().map(TryInto::try_into).collect()
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
