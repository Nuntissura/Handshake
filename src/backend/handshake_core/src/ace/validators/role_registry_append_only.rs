//! Role Registry Append-Only Validator (WP-1)
//!
//! Enforces Master Spec Addendum 3.3 invariants:
//! - `role_id` set is append-only (no removals)
//! - `contract_id` -> contract surface hash is immutable once published

use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::diagnostics::{
    DiagnosticError, DiagnosticInput, DiagnosticSeverity, DiagnosticSource, DiagnosticSurface,
    DiagnosticsStore, LinkConfidence,
};
use crate::storage::StorageError;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, PartialOrd, Ord)]
pub struct RoleId(pub String);

impl RoleId {
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, PartialOrd, Ord)]
pub struct DepartmentId(pub String);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RoleContractKind {
    Extract,
    Produce,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ContractSurfaceHash(pub [u8; 32]);

impl ContractSurfaceHash {
    pub fn as_hex(&self) -> String {
        hex::encode(self.0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RoleSpecEntry {
    pub role_id: RoleId,
    pub department_id: DepartmentId,
    pub display_name: String,
    #[serde(default)]
    pub aliases: Vec<RoleId>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RoleContractSurface {
    pub contract_id: String,
    pub role_id: RoleId,
    pub kind: RoleContractKind,
    pub version: String,
    pub schema_hash: ContractSurfaceHash,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct RoleRegistrySnapshot {
    pub roles: Vec<RoleSpecEntry>,
    pub contracts: Vec<RoleContractSurface>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RoleRegistryViolation {
    RoleIdRemoved { role_id: RoleId },
    ContractIdRemoved { contract_id: String },
    ContractSurfaceDrift {
        contract_id: String,
        expected_hash: ContractSurfaceHash,
        got_hash: ContractSurfaceHash,
    },
    DuplicateRoleId { role_id: RoleId },
    DuplicateContractId { contract_id: String },
    InvalidRoleId { role_id: RoleId },
    InvalidContractId { contract_id: String },
}

impl std::fmt::Display for RoleRegistryViolation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RoleRegistryViolation::RoleIdRemoved { role_id } => {
                write!(f, "role_id removed: {}", role_id.as_str())
            }
            RoleRegistryViolation::ContractIdRemoved { contract_id } => {
                write!(f, "contract_id removed: {}", contract_id)
            }
            RoleRegistryViolation::ContractSurfaceDrift {
                contract_id,
                expected_hash,
                got_hash,
            } => write!(
                f,
                "contract surface drift for {}: expected={} got={}",
                contract_id,
                expected_hash.as_hex(),
                got_hash.as_hex()
            ),
            RoleRegistryViolation::DuplicateRoleId { role_id } => {
                write!(f, "duplicate role_id: {}", role_id.as_str())
            }
            RoleRegistryViolation::DuplicateContractId { contract_id } => {
                write!(f, "duplicate contract_id: {}", contract_id)
            }
            RoleRegistryViolation::InvalidRoleId { role_id } => {
                write!(f, "invalid role_id: {}", role_id.as_str())
            }
            RoleRegistryViolation::InvalidContractId { contract_id } => {
                write!(f, "invalid contract_id: {}", contract_id)
            }
        }
    }
}

impl std::error::Error for RoleRegistryViolation {}

#[derive(Debug)]
pub enum RoleRegistryDiagnosticError {
    Diagnostic(DiagnosticError),
    Storage(StorageError),
}

impl From<DiagnosticError> for RoleRegistryDiagnosticError {
    fn from(err: DiagnosticError) -> Self {
        Self::Diagnostic(err)
    }
}

impl From<StorageError> for RoleRegistryDiagnosticError {
    fn from(err: StorageError) -> Self {
        Self::Storage(err)
    }
}

pub fn validate_append_only(
    baseline: &RoleRegistrySnapshot,
    current: &RoleRegistrySnapshot,
) -> Result<(), RoleRegistryViolation> {
    let mut current_role_ids = HashSet::<RoleId>::new();
    for role in &current.roles {
        if !is_valid_role_id(&role.role_id) {
            return Err(RoleRegistryViolation::InvalidRoleId {
                role_id: role.role_id.clone(),
            });
        }
        if !current_role_ids.insert(role.role_id.clone()) {
            return Err(RoleRegistryViolation::DuplicateRoleId {
                role_id: role.role_id.clone(),
            });
        }
    }

    let mut current_contract_ids = HashSet::<String>::new();
    let mut current_contract_by_id = HashMap::<String, ContractSurfaceHash>::new();
    for contract in &current.contracts {
        if !is_valid_contract_id(&contract.contract_id, &contract.role_id, contract.kind, &contract.version)
        {
            return Err(RoleRegistryViolation::InvalidContractId {
                contract_id: contract.contract_id.clone(),
            });
        }
        if !current_contract_ids.insert(contract.contract_id.clone()) {
            return Err(RoleRegistryViolation::DuplicateContractId {
                contract_id: contract.contract_id.clone(),
            });
        }
        current_contract_by_id.insert(contract.contract_id.clone(), contract.schema_hash);
    }

    let baseline_role_ids = baseline
        .roles
        .iter()
        .map(|r| r.role_id.clone())
        .collect::<HashSet<_>>();

    for role_id in baseline_role_ids {
        if !current_role_ids.contains(&role_id) {
            return Err(RoleRegistryViolation::RoleIdRemoved { role_id });
        }
    }

    for baseline_contract in &baseline.contracts {
        let current_hash = match current_contract_by_id.get(&baseline_contract.contract_id) {
            Some(hash) => *hash,
            None => {
                return Err(RoleRegistryViolation::ContractIdRemoved {
                    contract_id: baseline_contract.contract_id.clone(),
                });
            }
        };

        if current_hash != baseline_contract.schema_hash {
            return Err(RoleRegistryViolation::ContractSurfaceDrift {
                contract_id: baseline_contract.contract_id.clone(),
                expected_hash: baseline_contract.schema_hash,
                got_hash: current_hash,
            });
        }
    }

    Ok(())
}

pub async fn record_violation_diagnostic(
    store: &dyn DiagnosticsStore,
    job_id: Option<String>,
    workflow_id: Option<String>,
    violation: &RoleRegistryViolation,
) -> Result<Uuid, RoleRegistryDiagnosticError> {
    let mut tags = Vec::<String>::new();
    if let Some(workflow_id) = workflow_id.as_deref() {
        tags.push(format!("workflow_id:{}", workflow_id));
    }

    let input = DiagnosticInput {
        title: "Role registry append-only violation".to_string(),
        message: violation.to_string(),
        severity: DiagnosticSeverity::Error,
        source: DiagnosticSource::Validator,
        surface: DiagnosticSurface::System,
        tool: Some("role_registry_append_only".to_string()),
        code: None,
        tags: if tags.is_empty() { None } else { Some(tags) },
        wsid: None,
        job_id,
        model_id: None,
        actor: None,
        capability_id: None,
        policy_decision_id: None,
        locations: None,
        evidence_refs: None,
        link_confidence: LinkConfidence::Direct,
        status: None,
        count: None,
        first_seen: None,
        last_seen: None,
        timestamp: None,
        updated_at: None,
    };

    let diagnostic = input.into_diagnostic()?;
    let diagnostic_id = diagnostic.id;
    store.record_diagnostic(diagnostic).await?;
    Ok(diagnostic_id)
}

pub fn canonical_json_bytes(value: &Value) -> Vec<u8> {
    let mut out = String::new();
    write_canonical_json_value(&mut out, value);
    out.push('\n');
    out.into_bytes()
}

pub fn canonical_json_sha256(value: &Value) -> ContractSurfaceHash {
    let bytes = canonical_json_bytes(value);
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    let digest = hasher.finalize();
    let mut out = [0u8; 32];
    out.copy_from_slice(&digest);
    ContractSurfaceHash(out)
}

fn write_canonical_json_value(out: &mut String, value: &Value) {
    match value {
        Value::Null => out.push_str("null"),
        Value::Bool(v) => out.push_str(if *v { "true" } else { "false" }),
        Value::Number(num) => out.push_str(&num.to_string()),
        Value::String(s) => write_canonical_json_string(out, s),
        Value::Array(items) => {
            out.push('[');
            for (idx, item) in items.iter().enumerate() {
                if idx > 0 {
                    out.push(',');
                }
                write_canonical_json_value(out, item);
            }
            out.push(']');
        }
        Value::Object(map) => {
            out.push('{');
            let mut keys: Vec<&String> = map.keys().collect();
            keys.sort();
            for (idx, key) in keys.iter().enumerate() {
                if idx > 0 {
                    out.push(',');
                }
                write_canonical_json_string(out, key);
                out.push(':');
                if let Some(v) = map.get(*key) {
                    write_canonical_json_value(out, v);
                } else {
                    out.push_str("null");
                }
            }
            out.push('}');
        }
    }
}

fn write_canonical_json_string(out: &mut String, value: &str) {
    out.push('"');
    for ch in value.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\u{08}' => out.push_str("\\b"),
            '\u{0C}' => out.push_str("\\f"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => {
                out.push_str(&format!("\\u{:04X}", c as u32));
            }
            c if (c as u32) <= 0x7F => out.push(c),
            c if (c as u32) <= 0xFFFF => {
                out.push_str(&format!("\\u{:04X}", c as u32));
            }
            c => {
                let code = (c as u32) - 0x1_0000;
                let high = 0xD800 + ((code >> 10) & 0x3FF);
                let low = 0xDC00 + (code & 0x3FF);
                out.push_str(&format!("\\u{:04X}\\u{:04X}", high, low));
            }
        }
    }
    out.push('"');
}

fn is_valid_role_id(role_id: &RoleId) -> bool {
    let s = role_id.as_str();
    let mut chars = s.chars();
    let first = match chars.next() {
        Some(ch) => ch,
        None => return false,
    };
    if !first.is_ascii_lowercase() {
        return false;
    }
    for ch in chars {
        if ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '_' || ch == '-' {
            continue;
        }
        return false;
    }
    true
}

fn is_valid_contract_id(
    contract_id: &str,
    role_id: &RoleId,
    kind: RoleContractKind,
    version: &str,
) -> bool {
    let trimmed = contract_id.trim();
    let mut parts = trimmed.split(':');
    let tag = match parts.next() {
        Some(v) => v,
        None => return false,
    };
    let role_part = match parts.next() {
        Some(v) => v,
        None => return false,
    };
    let kind_part = match parts.next() {
        Some(v) => v,
        None => return false,
    };
    let ver_part = match parts.next() {
        Some(v) => v,
        None => return false,
    };

    if parts.next().is_some() {
        return false;
    }

    if tag != "ROLE" {
        return false;
    }

    if role_part != role_id.as_str() {
        return false;
    }

    let expected_kind = match kind {
        RoleContractKind::Extract => "X",
        RoleContractKind::Produce => "C",
    };

    if kind_part != expected_kind {
        return false;
    }

    if ver_part != version {
        return false;
    }

    if ver_part.is_empty() {
        return false;
    }

    let mut ver_chars = ver_part.chars();
    let first = match ver_chars.next() {
        Some(c) => c,
        None => return false,
    };
    if first < '1' || first > '9' {
        return false;
    }
    for ch in ver_chars {
        if !ch.is_ascii_digit() {
            return false;
        }
    }

    true
}