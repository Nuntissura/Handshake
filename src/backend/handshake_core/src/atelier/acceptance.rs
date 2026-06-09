//! Product acceptance constraints learned from the legacy package/release/data-root
//! flow (MT-051). These checks are runtime-callable product guards, not repo
//! governance: they reject machine-local roots, non-relocatable artifact refs,
//! and repo `dist/` release targets before an atelier workflow persists them.

use serde::{Deserialize, Serialize};

use super::{reject_legacy_runtime_ref, AtelierError, AtelierResult};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct AtelierAcceptanceConstraints {
    pub data_root_ref: String,
    pub artifact_root_ref: String,
    pub release_output_ref: String,
}

impl AtelierAcceptanceConstraints {
    pub fn validate(&self) -> AtelierResult<()> {
        validate_data_root_ref(&self.data_root_ref)?;
        validate_artifact_root_ref(&self.artifact_root_ref)?;
        validate_release_output_ref(&self.release_output_ref)?;
        Ok(())
    }
}

fn require_trimmed(field: &str, value: &str) -> AtelierResult<()> {
    let trimmed = value.trim();
    if trimmed.is_empty() || trimmed != value {
        return Err(AtelierError::Validation(format!(
            "{field} must not be empty or padded"
        )));
    }
    Ok(())
}

fn validate_data_root_ref(value: &str) -> AtelierResult<()> {
    require_trimmed("data_root_ref", value)?;
    reject_legacy_runtime_ref("data_root_ref", value)?;
    let normalized = value.to_ascii_lowercase().replace('\\', "/");
    if !normalized.starts_with("data-root:") || normalized.contains("..") {
        return Err(AtelierError::Validation(
            "data_root_ref must be a portable data-root: ref without traversal".into(),
        ));
    }
    Ok(())
}

fn validate_artifact_root_ref(value: &str) -> AtelierResult<()> {
    require_trimmed("artifact_root_ref", value)?;
    reject_legacy_runtime_ref("artifact_root_ref", value)?;
    let normalized = value.to_ascii_lowercase().replace('\\', "/");
    let is_artifact_store_root = normalized == "artifact-store://.handshake/artifacts";
    let is_artifact_payload = normalized.starts_with("artifact://.handshake/artifacts/")
        && normalized.ends_with("/payload");
    if !is_artifact_store_root && !is_artifact_payload {
        return Err(AtelierError::Validation(
            "artifact_root_ref must be a relocatable Handshake ArtifactStore ref".into(),
        ));
    }
    Ok(())
}

fn validate_release_output_ref(value: &str) -> AtelierResult<()> {
    require_trimmed("release_output_ref", value)?;
    reject_legacy_runtime_ref("release_output_ref", value)?;
    let normalized = value.to_ascii_lowercase().replace('\\', "/");
    let contains_repo_dist = normalized == "dist"
        || normalized.starts_with("dist/")
        || normalized.ends_with("/dist")
        || normalized.contains("/dist/")
        || normalized.starts_with("repo://dist");
    if !normalized.starts_with("release://") || normalized.contains("..") || contains_repo_dist {
        return Err(AtelierError::Validation(
            "release_output_ref must be a release:// ref outside repo dist/ paths".into(),
        ));
    }
    Ok(())
}
