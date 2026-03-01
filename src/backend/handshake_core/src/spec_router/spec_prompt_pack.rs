use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::llm::sha256_hex;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpecPromptPackV1 {
    pub schema_version: String,
    pub pack_id: String,
    pub description: String,
    pub target_job_kind: String,
    pub stable_prefix_sections: Vec<StablePrefixSectionV1>,
    pub variable_suffix_template_md: String,
    pub placeholders: Vec<PlaceholderV1>,
    pub required_outputs: Vec<RequiredOutputV1>,
    pub budgets: BudgetsV1,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StablePrefixSectionV1 {
    pub section_id: String,
    pub content_md: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlaceholderV1 {
    pub name: String,
    pub source: PlaceholderSourceV1,
    pub max_tokens: u32,
    pub required: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PlaceholderSourceV1 {
    PromptRef,
    CapabilitySnapshot,
    WorkflowContext,
    GovernanceMode,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequiredOutputV1 {
    pub artifact_kind: String,
    pub schema_ref: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BudgetsV1 {
    pub max_total_tokens: u32,
    pub max_prompt_excerpt_tokens: u32,
    pub max_capsule_tokens: u32,
    pub max_capability_table_tokens: u32,
}

#[derive(Debug, Clone)]
pub struct LoadedSpecPromptPack {
    pub pack: SpecPromptPackV1,
    pub pack_id: String,
    pub pack_sha256: String,
    pub raw_bytes: Vec<u8>,
}

#[derive(Debug, thiserror::Error)]
pub enum SpecPromptPackError {
    #[error("invalid pack id: {pack_id}")]
    InvalidPackId { pack_id: String },
    #[error("spec prompt pack not found: {path}")]
    NotFound { path: PathBuf },
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("json parse error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("unsupported schema_version: {found}")]
    UnsupportedSchemaVersion { found: String },
    #[error("pack_id mismatch: expected={expected} found={found}")]
    PackIdMismatch { expected: String, found: String },
    #[error("target_job_kind mismatch: expected={expected} found={found}")]
    TargetJobKindMismatch { expected: String, found: String },
}

fn validate_pack_id(pack_id: &str) -> Result<(), SpecPromptPackError> {
    let trimmed = pack_id.trim();
    if trimmed.is_empty() {
        return Err(SpecPromptPackError::InvalidPackId {
            pack_id: pack_id.to_string(),
        });
    }
    if trimmed.contains('/') || trimmed.contains('\\') || trimmed.split('/').any(|c| c == "..") {
        return Err(SpecPromptPackError::InvalidPackId {
            pack_id: pack_id.to_string(),
        });
    }
    Ok(())
}

pub fn spec_prompt_pack_path(workspace_root: &Path, pack_id: &str) -> PathBuf {
    workspace_root
        .join("assets")
        .join("spec_prompt_packs")
        .join(format!("{pack_id}.json"))
}

pub fn load_spec_prompt_pack(
    workspace_root: &Path,
    pack_id: &str,
) -> Result<LoadedSpecPromptPack, SpecPromptPackError> {
    validate_pack_id(pack_id)?;

    let path = spec_prompt_pack_path(workspace_root, pack_id);
    let raw_bytes = match fs::read(&path) {
        Ok(bytes) => bytes,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            return Err(SpecPromptPackError::NotFound { path });
        }
        Err(err) => return Err(SpecPromptPackError::Io(err)),
    };

    let pack_sha256 = sha256_hex(&raw_bytes);
    let pack: SpecPromptPackV1 = serde_json::from_slice(&raw_bytes)?;

    if pack.schema_version != "hsk.spec_prompt_pack@1" {
        return Err(SpecPromptPackError::UnsupportedSchemaVersion {
            found: pack.schema_version,
        });
    }
    if pack.pack_id != pack_id {
        return Err(SpecPromptPackError::PackIdMismatch {
            expected: pack_id.to_string(),
            found: pack.pack_id,
        });
    }
    if pack.target_job_kind != "spec_router" {
        return Err(SpecPromptPackError::TargetJobKindMismatch {
            expected: "spec_router".to_string(),
            found: pack.target_job_kind,
        });
    }

    Ok(LoadedSpecPromptPack {
        pack,
        pack_id: pack_id.to_string(),
        pack_sha256,
        raw_bytes,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn load_pack_hashes_exact_bytes() {
        let workspace_root = tempfile::tempdir().expect("tempdir");
        let pack_id = "spec_router_pack@1";
        let path = spec_prompt_pack_path(workspace_root.path(), pack_id);
        std::fs::create_dir_all(path.parent().expect("parent dir")).expect("create_dir_all");

        let raw_bytes = br#"{
  "schema_version":"hsk.spec_prompt_pack@1",
  "pack_id":"spec_router_pack@1",
  "description":"test pack",
  "target_job_kind":"spec_router",
  "stable_prefix_sections":[{"section_id":"SYSTEM_RULES","content_md":"RULES"}],
  "variable_suffix_template_md":"Hello",
  "placeholders":[],
  "required_outputs":[],
  "budgets":{"max_total_tokens":100,"max_prompt_excerpt_tokens":10,"max_capsule_tokens":10,"max_capability_table_tokens":10}
}
"#;
        std::fs::write(&path, raw_bytes).expect("write pack");

        let loaded = load_spec_prompt_pack(workspace_root.path(), pack_id).expect("load pack");
        assert_eq!(loaded.pack_id, pack_id);
        assert_eq!(loaded.pack.pack_id, pack_id);
        assert_eq!(loaded.pack_sha256, sha256_hex(raw_bytes));
        assert_eq!(loaded.raw_bytes.as_slice(), raw_bytes);
    }
}
