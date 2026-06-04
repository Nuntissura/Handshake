use std::fs;
use std::path::Path;

use serde::Deserialize;
use thiserror::Error;

use super::applicability::HbrApplicability;

const EXPECTED_SCHEMA: &str = "handshake.build_rules@1";
const EXPECTED_NAME: &str = "HANDSHAKE_BUILD_RULES";

#[derive(Debug, Error)]
pub enum HbrError {
    #[error("failed to read HBR registry: {0}")]
    Io(#[from] std::io::Error),
    #[error("failed to parse HBR registry JSON: {0}")]
    Json(#[from] serde_json::Error),
    #[error("invalid HBR registry {field}: expected {expected}, got {actual}")]
    InvalidRegistryField {
        field: &'static str,
        expected: &'static str,
        actual: String,
    },
    #[error("invalid HBR registry version: {version}")]
    InvalidVersion { version: String },
}

#[derive(Debug, Clone, Deserialize)]
struct HbrRegistryDocument {
    schema: String,
    name: String,
    version: String,
    rules: Vec<HbrRule>,
}

#[derive(Debug, Clone)]
pub struct HbrRegistry {
    pub schema: String,
    pub name: String,
    pub version: String,
    rules: Vec<HbrRule>,
}

impl HbrRegistry {
    pub fn load_from_path(path: &Path) -> Result<Self, HbrError> {
        let raw = fs::read_to_string(path)?;
        let document: HbrRegistryDocument = serde_json::from_str(&raw)?;
        if document.schema != EXPECTED_SCHEMA {
            return Err(HbrError::InvalidRegistryField {
                field: "schema",
                expected: EXPECTED_SCHEMA,
                actual: document.schema,
            });
        }
        if document.name != EXPECTED_NAME {
            return Err(HbrError::InvalidRegistryField {
                field: "name",
                expected: EXPECTED_NAME,
                actual: document.name,
            });
        }
        if !is_semver_core(&document.version) {
            return Err(HbrError::InvalidVersion {
                version: document.version,
            });
        }

        Ok(Self {
            schema: document.schema,
            name: document.name,
            version: document.version,
            rules: document.rules,
        })
    }

    pub fn active_rules(&self) -> impl Iterator<Item = &HbrRule> {
        self.rules
            .iter()
            .filter(|rule| rule.status.eq_ignore_ascii_case("ACTIVE"))
    }

    pub fn rule(&self, id: &str) -> Option<&HbrRule> {
        self.rules.iter().find(|rule| rule.id == id)
    }

    pub fn rules(&self) -> &[HbrRule] {
        &self.rules
    }
}

fn is_semver_core(value: &str) -> bool {
    let mut parts = value.splitn(3, '.');
    let major = parts.next().unwrap_or_default();
    let minor = parts.next().unwrap_or_default();
    let patch_and_suffix = parts.next().unwrap_or_default();
    if major.is_empty() || minor.is_empty() || patch_and_suffix.is_empty() {
        return false;
    }
    if !major.chars().all(|ch| ch.is_ascii_digit()) || !minor.chars().all(|ch| ch.is_ascii_digit())
    {
        return false;
    }
    let patch = patch_and_suffix
        .split(['-', '+'])
        .next()
        .unwrap_or_default();
    !patch.is_empty() && patch.chars().all(|ch| ch.is_ascii_digit())
}

#[derive(Debug, Clone, Copy, Deserialize, Eq, Hash, PartialEq)]
pub enum HbrPillar {
    #[serde(rename = "INT")]
    Int,
    #[serde(rename = "SWARM")]
    Swarm,
    #[serde(rename = "VIS")]
    Vis,
    #[serde(rename = "QUIET")]
    Quiet,
    #[serde(rename = "MAN")]
    Man,
    #[serde(rename = "STOP")]
    Stop,
}

#[derive(Debug, Clone, Deserialize)]
pub struct HbrRule {
    pub id: String,
    pub pillar: HbrPillar,
    pub status: String,
    pub novelty: String,
    pub trigger: String,
    pub applicability: HbrApplicability,
    pub required_action: String,
    pub evidence_kind: String,
    pub citations: Vec<HbrCitation>,
    pub not_applicable_when: Vec<String>,
    pub deferred_until: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct HbrCitation {
    pub kind: String,
    pub anchor: String,
}
