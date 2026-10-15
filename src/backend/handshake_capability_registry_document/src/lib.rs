use std::collections::HashSet;

use jsonschema::{Draft, JSONSchema};
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum CapabilityRegistryDocumentError {
    #[error("CAP_REG_SCHEMA_COMPILE: compile schema: {message}")]
    SchemaCompile { message: String },
    #[error("CAP_REG_SCHEMA_VALIDATE: {details}")]
    SchemaValidationFailed { details: String },
    #[error("CAP_REG_REGEX_COMPILE: section_ref regex compile failed: {message}")]
    RegexCompile { message: String },
    #[error("CAP_REG_INTEGRITY_DUPLICATE_CAPABILITY_ID: duplicate capability_id {capability_id}")]
    DuplicateCapabilityId { capability_id: String },
    #[error("CAP_REG_INTEGRITY_INVALID_SECTION_REF: invalid section_ref for {capability_id}: {section_ref}")]
    InvalidSectionRef {
        capability_id: String,
        section_ref: String,
    },
    #[error("CAP_REG_INTEGRITY_EMPTY_DISPLAY_NAME: display_name must be non-empty for {capability_id}")]
    EmptyDisplayName { capability_id: String },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CapabilityKind {
    Surface,
    Engine,
    Runtime,
    Integration,
    Model,
    Workflow,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum GovernanceMode {
    GovStrict,
    GovStandard,
    GovLight,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RiskClass {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CapabilityRegistryEntry {
    pub capability_id: String,
    pub kind: CapabilityKind,
    pub display_name: String,
    pub section_ref: String,
    pub required_capabilities: Vec<String>,
    pub default_governance_mode: GovernanceMode,
    pub risk_class: RiskClass,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CapabilityRegistryDocument {
    pub entries: Vec<CapabilityRegistryEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CapabilityRegistryDiff {
    pub previous_registry_sha256: Option<String>,
    pub next_registry_sha256: String,
    pub added: Vec<String>,
    pub removed: Vec<String>,
    pub changed: Vec<String>,
}

pub fn validate_schema(
    schema: &Value,
    instance: &Value,
) -> Result<(), CapabilityRegistryDocumentError> {
    let compiled = JSONSchema::options()
        .with_draft(Draft::Draft7)
        .compile(schema)
        .map_err(|source| CapabilityRegistryDocumentError::SchemaCompile {
            message: source.to_string(),
        })?;

    let result = compiled.validate(instance);
    if let Err(errors) = result {
        let mut messages: Vec<String> = errors.map(|e| e.to_string()).collect();
        messages.sort();
        let details = messages
            .into_iter()
            .map(|m| format!("- {m}"))
            .collect::<Vec<_>>()
            .join("\n");
        return Err(CapabilityRegistryDocumentError::SchemaValidationFailed { details });
    }
    Ok(())
}

pub fn validate_integrity(
    doc: &CapabilityRegistryDocument,
) -> Result<(), CapabilityRegistryDocumentError> {
    let section_re = Regex::new(r"^\d+(\.\d+)*$").map_err(|source| {
        CapabilityRegistryDocumentError::RegexCompile {
            message: source.to_string(),
        }
    })?;

    let mut seen = HashSet::new();
    for entry in &doc.entries {
        if !seen.insert(entry.capability_id.as_str()) {
            return Err(CapabilityRegistryDocumentError::DuplicateCapabilityId {
                capability_id: entry.capability_id.clone(),
            });
        }
        if !section_re.is_match(entry.section_ref.as_str()) {
            return Err(CapabilityRegistryDocumentError::InvalidSectionRef {
                capability_id: entry.capability_id.clone(),
                section_ref: entry.section_ref.clone(),
            });
        }
        if entry.display_name.trim().is_empty() {
            return Err(CapabilityRegistryDocumentError::EmptyDisplayName {
                capability_id: entry.capability_id.clone(),
            });
        }
    }

    Ok(())
}
