//! MT-107: Abliteration output review gate.
//!
//! Reviews an abliterated model artifact's provenance and artifact-level
//! metadata before any downstream Skill Bank registration path can
//! reference it. Failed reviews quarantine by moving the artifact into a
//! quarantine directory; no artifact bytes are deleted by this module.

use std::{
    collections::HashSet,
    fs,
    path::{Path, PathBuf},
};

use chrono::DateTime;
use thiserror::Error;

use super::{
    abliterate::{AbliterationProvenance, ABLITERATION_TOOL_VERSION},
    content_review::ContentReviewConfig,
    pii_patterns::scan as scan_pii,
};

pub const ABLITERATION_REVIEW_EVENT_ID: &str = "FR-EVT-DISTILL-ABLITERATE-REVIEW";
pub const SKILL_BANK_REGISTER_ABLITERATED_MODEL_ACTION: &str =
    "kernel.skill_bank.register_abliterated_model";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AbliterationOutputReviewConfig {
    pub license_allowlist: HashSet<String>,
    /// Optional quarantine root. When absent, failures move to
    /// `<artifact parent>/.quarantine/<artifact file name>`.
    pub quarantine_root: Option<PathBuf>,
}

impl Default for AbliterationOutputReviewConfig {
    fn default() -> Self {
        Self {
            license_allowlist: ContentReviewConfig::defaults().license_allowlist,
            quarantine_root: None,
        }
    }
}

impl From<ContentReviewConfig> for AbliterationOutputReviewConfig {
    fn from(value: ContentReviewConfig) -> Self {
        Self {
            license_allowlist: value.license_allowlist,
            quarantine_root: Some(PathBuf::from(value.quarantine_root)),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AbliterationReviewFailure {
    MissingProvenanceField { field: &'static str },
    InvalidProvenanceField { field: &'static str, reason: String },
    UntaggableLicense,
    LicenseNotAllowed { license_tag: String },
    PiiDetected { kind: String, severity: String },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AbliterationReviewEvent {
    pub event_id: String,
    pub result: String,
    pub artifact_path: PathBuf,
    pub quarantine_path: Option<PathBuf>,
    pub reason_tags: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SkillBankAbliteratedModelRegistration {
    pub action: String,
    pub artifact_path: PathBuf,
    pub base_model_sha256: String,
    pub refusal_direction_sha256: String,
    pub license_tag: String,
}

pub type AbliteratedSkillBankModelRegistration = SkillBankAbliteratedModelRegistration;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ReviewResult {
    Pass {
        artifact_path: PathBuf,
        event: AbliterationReviewEvent,
        skill_bank_registration: SkillBankAbliteratedModelRegistration,
    },
    Fail {
        reasons: Vec<AbliterationReviewFailure>,
        quarantine_path: Option<PathBuf>,
        event: AbliterationReviewEvent,
    },
}

#[derive(Debug, Error)]
pub enum AbliterationReviewError {
    #[error("abliteration review I/O failed: {0}")]
    Io(String),
    #[error("abliteration artifact failed review")]
    ReviewFailed {
        reasons: Vec<AbliterationReviewFailure>,
        quarantine_path: Option<PathBuf>,
    },
}

impl AbliterationReviewError {
    pub fn reasons(&self) -> &[AbliterationReviewFailure] {
        match self {
            Self::ReviewFailed { reasons, .. } => reasons,
            Self::Io(_) => &[],
        }
    }

    pub fn quarantine_path(&self) -> Option<&Path> {
        match self {
            Self::ReviewFailed {
                quarantine_path, ..
            } => quarantine_path.as_deref(),
            Self::Io(_) => None,
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct AbliterationOutputReview {
    config: AbliterationOutputReviewConfig,
}

impl AbliterationOutputReview {
    pub fn new(config: AbliterationOutputReviewConfig) -> Self {
        Self { config }
    }

    pub fn check(
        &self,
        artifact_path: &Path,
        provenance: &AbliterationProvenance,
    ) -> Result<ReviewResult, AbliterationReviewError> {
        if !artifact_path.is_file() {
            return Err(AbliterationReviewError::Io(format!(
                "artifact not found: {}",
                artifact_path.display()
            )));
        }

        let reasons = review_failures(provenance, &self.config);
        if reasons.is_empty() {
            let registration = skill_bank_registration_token(artifact_path, provenance);
            let event = review_event(artifact_path, "Pass", None, &[]);
            return Ok(ReviewResult::Pass {
                artifact_path: artifact_path.to_path_buf(),
                event,
                skill_bank_registration: registration,
            });
        }

        let quarantine_path = self.quarantine_artifact(artifact_path)?;
        let event = review_event(artifact_path, "Fail", quarantine_path.as_deref(), &reasons);
        Ok(ReviewResult::Fail {
            reasons,
            quarantine_path,
            event,
        })
    }

    pub fn register_abliterated_model(
        &self,
        artifact_path: &Path,
        provenance: &AbliterationProvenance,
    ) -> Result<SkillBankAbliteratedModelRegistration, AbliterationReviewError> {
        match self.check(artifact_path, provenance)? {
            ReviewResult::Pass {
                skill_bank_registration,
                ..
            } => Ok(skill_bank_registration),
            ReviewResult::Fail {
                reasons,
                quarantine_path,
                ..
            } => Err(AbliterationReviewError::ReviewFailed {
                reasons,
                quarantine_path,
            }),
        }
    }

    fn quarantine_artifact(
        &self,
        artifact_path: &Path,
    ) -> Result<Option<PathBuf>, AbliterationReviewError> {
        if !artifact_path.exists() {
            return Ok(None);
        }

        let quarantine_root = match &self.config.quarantine_root {
            Some(root) if root.is_absolute() => root.clone(),
            Some(root) => artifact_path
                .parent()
                .unwrap_or_else(|| Path::new("."))
                .join(root),
            None => artifact_path
                .parent()
                .unwrap_or_else(|| Path::new("."))
                .join(".quarantine"),
        };
        fs::create_dir_all(&quarantine_root).map_err(|error| {
            AbliterationReviewError::Io(format!(
                "create quarantine dir {}: {error}",
                quarantine_root.display()
            ))
        })?;

        let file_name = artifact_path
            .file_name()
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| std::ffi::OsStr::new("abliterated-artifact"));
        let destination = unique_quarantine_path(&quarantine_root, file_name);
        fs::rename(artifact_path, &destination).map_err(|error| {
            AbliterationReviewError::Io(format!(
                "move {} to {}: {error}",
                artifact_path.display(),
                destination.display()
            ))
        })?;
        Ok(Some(destination))
    }
}

fn review_failures(
    provenance: &AbliterationProvenance,
    config: &AbliterationOutputReviewConfig,
) -> Vec<AbliterationReviewFailure> {
    let mut reasons = Vec::new();

    require_sha256(
        "base_model_sha256",
        &provenance.base_model_sha256,
        &mut reasons,
    );
    require_sha256(
        "refusal_direction_sha256",
        &provenance.refusal_direction_sha256,
        &mut reasons,
    );
    require_tool_version(&provenance.abliteration_tool_version, &mut reasons);
    require_timestamp(
        "abliterated_at_utc",
        &provenance.abliterated_at_utc,
        &mut reasons,
    );
    review_license(&provenance.license_tag, config, &mut reasons);
    require_non_empty(
        "operator_signature",
        &provenance.operator_signature,
        &mut reasons,
    );
    require_non_empty("provenance_note", &provenance.provenance_note, &mut reasons);

    if provenance.orthogonalised_weight_keys.is_empty() {
        reasons.push(AbliterationReviewFailure::MissingProvenanceField {
            field: "orthogonalised_weight_keys",
        });
    } else if provenance
        .orthogonalised_weight_keys
        .iter()
        .any(|key| key.trim().is_empty())
    {
        reasons.push(AbliterationReviewFailure::InvalidProvenanceField {
            field: "orthogonalised_weight_keys",
            reason: "contains an empty tensor key".to_string(),
        });
    }

    let metadata_text = provenance_metadata_text(provenance);
    for detection in scan_pii(&metadata_text) {
        reasons.push(AbliterationReviewFailure::PiiDetected {
            kind: detection.kind.label().to_string(),
            severity: format!("{:?}", detection.severity),
        });
    }

    reasons
}

fn require_non_empty(
    field: &'static str,
    value: &str,
    reasons: &mut Vec<AbliterationReviewFailure>,
) {
    if value.trim().is_empty() {
        reasons.push(AbliterationReviewFailure::MissingProvenanceField { field });
    }
}

fn require_sha256(field: &'static str, value: &str, reasons: &mut Vec<AbliterationReviewFailure>) {
    let value = value.trim();
    if value.is_empty() {
        reasons.push(AbliterationReviewFailure::MissingProvenanceField { field });
        return;
    }
    if value.len() != 64 || !value.chars().all(|ch| ch.is_ascii_hexdigit()) {
        reasons.push(AbliterationReviewFailure::InvalidProvenanceField {
            field,
            reason: "must be a 64-character SHA-256 hex digest".to_string(),
        });
    }
}

fn require_tool_version(value: &str, reasons: &mut Vec<AbliterationReviewFailure>) {
    let value = value.trim();
    if value.is_empty() {
        reasons.push(AbliterationReviewFailure::MissingProvenanceField {
            field: "abliteration_tool_version",
        });
        return;
    }
    if value != ABLITERATION_TOOL_VERSION {
        reasons.push(AbliterationReviewFailure::InvalidProvenanceField {
            field: "abliteration_tool_version",
            reason: format!("expected {ABLITERATION_TOOL_VERSION}, got {value}"),
        });
    }
}

fn require_timestamp(
    field: &'static str,
    value: &str,
    reasons: &mut Vec<AbliterationReviewFailure>,
) {
    let value = value.trim();
    if value.is_empty() {
        reasons.push(AbliterationReviewFailure::MissingProvenanceField { field });
        return;
    }
    if DateTime::parse_from_rfc3339(value).is_err() {
        reasons.push(AbliterationReviewFailure::InvalidProvenanceField {
            field,
            reason: "must be RFC3339 timestamp text".to_string(),
        });
    }
}

fn review_license(
    license_tag: &str,
    config: &AbliterationOutputReviewConfig,
    reasons: &mut Vec<AbliterationReviewFailure>,
) {
    let license_tag = license_tag.trim();
    if license_tag.is_empty() {
        reasons.push(AbliterationReviewFailure::UntaggableLicense);
        return;
    }
    if !config
        .license_allowlist
        .iter()
        .any(|allowed| allowed.eq_ignore_ascii_case(license_tag))
    {
        reasons.push(AbliterationReviewFailure::LicenseNotAllowed {
            license_tag: license_tag.to_string(),
        });
    }
}

fn provenance_metadata_text(provenance: &AbliterationProvenance) -> String {
    format!(
        "\
base_model_sha256={base_model_sha256}
refusal_direction_sha256={refusal_direction_sha256}
abliteration_tool_version={abliteration_tool_version}
abliterated_at_utc={abliterated_at_utc}
license_tag={license_tag}
operator_signature={operator_signature}
provenance_note={provenance_note}
orthogonalised_weight_keys={orthogonalised_weight_keys}
process_ledger_record_id={process_ledger_record_id}
",
        base_model_sha256 = provenance.base_model_sha256,
        refusal_direction_sha256 = provenance.refusal_direction_sha256,
        abliteration_tool_version = provenance.abliteration_tool_version,
        abliterated_at_utc = provenance.abliterated_at_utc,
        license_tag = provenance.license_tag,
        operator_signature = provenance.operator_signature,
        provenance_note = provenance.provenance_note,
        orthogonalised_weight_keys = provenance.orthogonalised_weight_keys.join(","),
        process_ledger_record_id = provenance.process_ledger_record_id.as_deref().unwrap_or(""),
    )
}

fn skill_bank_registration_token(
    artifact_path: &Path,
    provenance: &AbliterationProvenance,
) -> SkillBankAbliteratedModelRegistration {
    SkillBankAbliteratedModelRegistration {
        action: SKILL_BANK_REGISTER_ABLITERATED_MODEL_ACTION.to_string(),
        artifact_path: artifact_path.to_path_buf(),
        base_model_sha256: provenance.base_model_sha256.clone(),
        refusal_direction_sha256: provenance.refusal_direction_sha256.clone(),
        license_tag: provenance.license_tag.trim().to_string(),
    }
}

fn review_event(
    artifact_path: &Path,
    result: &str,
    quarantine_path: Option<&Path>,
    reasons: &[AbliterationReviewFailure],
) -> AbliterationReviewEvent {
    AbliterationReviewEvent {
        event_id: ABLITERATION_REVIEW_EVENT_ID.to_string(),
        result: result.to_string(),
        artifact_path: artifact_path.to_path_buf(),
        quarantine_path: quarantine_path.map(Path::to_path_buf),
        reason_tags: reasons.iter().map(reason_tag).collect(),
    }
}

fn reason_tag(reason: &AbliterationReviewFailure) -> String {
    match reason {
        AbliterationReviewFailure::MissingProvenanceField { field } => {
            format!("missing:{field}")
        }
        AbliterationReviewFailure::InvalidProvenanceField { field, .. } => {
            format!("invalid:{field}")
        }
        AbliterationReviewFailure::UntaggableLicense => "license:untaggable".to_string(),
        AbliterationReviewFailure::LicenseNotAllowed { license_tag } => {
            format!("license:not_allowed:{license_tag}")
        }
        AbliterationReviewFailure::PiiDetected { kind, severity } => {
            format!("pii:{kind}:{severity}")
        }
    }
}

fn unique_quarantine_path(root: &Path, file_name: &std::ffi::OsStr) -> PathBuf {
    let first = root.join(file_name);
    if !first.exists() {
        return first;
    }

    let file_name = Path::new(file_name);
    let stem = file_name
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("abliterated-artifact");
    let extension = file_name.extension().and_then(|value| value.to_str());

    for index in 1_u32.. {
        let candidate_name = match extension {
            Some(extension) => format!("{stem}.{index}.{extension}"),
            None => format!("{stem}.{index}"),
        };
        let candidate = root.join(candidate_name);
        if !candidate.exists() {
            return candidate;
        }
    }
    unreachable!("unbounded suffix search must return")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unique_quarantine_path_adds_suffix_when_destination_exists() {
        let tempdir = tempfile::tempdir().expect("tempdir");
        let existing = tempdir.path().join("artifact.safetensors");
        fs::write(&existing, b"existing").expect("write");
        let next =
            unique_quarantine_path(tempdir.path(), std::ffi::OsStr::new("artifact.safetensors"));
        assert_eq!(
            next.file_name().and_then(|v| v.to_str()),
            Some("artifact.1.safetensors")
        );
    }
}
