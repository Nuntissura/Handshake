use serde_json::Value;

// Role Registry Append-Only Validator [WP-1]
//
// Enforces (per Master Spec Addendum 3.3 / 6.3.3.5.7.*):
// - role_id set is append-only vs baseline
// - contract_id set is append-only vs baseline
// - contract_id -> schema_json hash is immutable once published
// - additions are allowed

use std::collections::{BTreeMap, HashMap, HashSet};

use sha2::{Digest, Sha256};

use crate::diagnostics::{
    DiagnosticActor, DiagnosticInput, DiagnosticSeverity, DiagnosticSource, DiagnosticSurface,
    DiagnosticsStore, LinkConfidence,
};
use crate::storage::StorageError;

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct RoleId(pub String);

impl RoleId {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct DepartmentId(pub String);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RoleContractKind {
    Extract,
    Produce,
}

impl RoleContractKind {
    pub fn as_contract_kind_tag(&self) -> &'static str {
        match self {
            Self::Extract => "X",
            Self::Produce => "C",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoleSpecEntry {
    pub role_id: RoleId,
    pub department_id: DepartmentId,
    pub display_name: String,
    pub aliases: Vec<RoleId>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ContractSurfaceHash(pub [u8; 32]);

impl ContractSurfaceHash {
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    pub fn to_hex(self) -> String {
        hex::encode(self.0)
    }
}

impl std::fmt::Display for ContractSurfaceHash {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&hex::encode(self.0))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoleContractSurface {
    pub contract_id: String,
    pub role_id: RoleId,
    pub kind: RoleContractKind,
    pub version: String,
    pub schema_hash: ContractSurfaceHash,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RoleRegistrySnapshot {
    pub roles: Vec<RoleSpecEntry>,
    pub contracts: Vec<RoleContractSurface>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RoleRegistryViolation {
    RoleIdRemoved {
        role_id: RoleId,
    },
    ContractIdRemoved {
        contract_id: String,
    },
    ContractSurfaceDrift {
        contract_id: String,
        expected_hash: ContractSurfaceHash,
        got_hash: ContractSurfaceHash,
    },
    DuplicateRoleId {
        role_id: RoleId,
    },
    DuplicateContractId {
        contract_id: String,
    },
    InvalidRoleId {
        role_id: String,
    },
}

impl RoleRegistryViolation {
    pub fn diagnostic_code(&self) -> &'static str {
        match self {
            Self::RoleIdRemoved { .. } => "HSK-RR-001",
            Self::ContractIdRemoved { .. } => "HSK-RR-002",
            Self::ContractSurfaceDrift { .. } => "HSK-RR-003",
            Self::DuplicateRoleId { .. } => "HSK-RR-004",
            Self::DuplicateContractId { .. } => "HSK-RR-005",
            Self::InvalidRoleId { .. } => "HSK-RR-006",
        }
    }

    pub fn diagnostic_title(&self) -> &'static str {
        match self {
            Self::RoleIdRemoved { .. } => "Role registry append-only violation: role_id removed",
            Self::ContractIdRemoved { .. } => {
                "Role registry append-only violation: contract_id removed"
            }
            Self::ContractSurfaceDrift { .. } => {
                "Role registry append-only violation: contract surface drift"
            }
            Self::DuplicateRoleId { .. } => "Role registry invalid: duplicate role_id",
            Self::DuplicateContractId { .. } => "Role registry invalid: duplicate contract_id",
            Self::InvalidRoleId { .. } => "Role registry invalid: role_id",
        }
    }

    pub fn diagnostic_message(&self) -> String {
        match self {
            Self::RoleIdRemoved { role_id } => format!(
                "Previously-declared role_id is missing from current snapshot: {}",
                role_id.as_str()
            ),
            Self::ContractIdRemoved { contract_id } => format!(
                "Previously-declared contract_id is missing from current snapshot: {}",
                contract_id
            ),
            Self::ContractSurfaceDrift {
                contract_id,
                expected_hash,
                got_hash,
            } => format!(
                "schema_json changed for existing contract_id (expected_hash={}, got_hash={}): {}",
                expected_hash, got_hash, contract_id
            ),
            Self::DuplicateRoleId { role_id } => {
                format!(
                    "Duplicate role_id value present in snapshot: {}",
                    role_id.as_str()
                )
            }
            Self::DuplicateContractId { contract_id } => format!(
                "Duplicate contract_id value present in snapshot: {}",
                contract_id
            ),
            Self::InvalidRoleId { role_id } => {
                format!("Invalid role_id (expected non-empty string): {}", role_id)
            }
        }
    }
}

pub struct RoleRegistryAppendOnlyValidator;

impl RoleRegistryAppendOnlyValidator {
    pub fn validate(
        current: &RoleRegistrySnapshot,
        baseline: &RoleRegistrySnapshot,
    ) -> Result<(), RoleRegistryViolation> {
        let mut violations = Self::validate_all(current, baseline);
        if violations.is_empty() {
            return Ok(());
        }
        violations.sort_by(|a, b| a.diagnostic_code().cmp(b.diagnostic_code()));
        Err(violations.remove(0))
    }

    pub fn validate_all(
        current: &RoleRegistrySnapshot,
        baseline: &RoleRegistrySnapshot,
    ) -> Vec<RoleRegistryViolation> {
        let mut violations = Vec::new();

        violations.extend(Self::validate_snapshot_integrity(baseline));
        violations.extend(Self::validate_snapshot_integrity(current));

        // If the snapshots are malformed, prefer surfacing those violations first.
        if !violations.is_empty() {
            return violations;
        }

        let baseline_role_ids: HashSet<&str> =
            baseline.roles.iter().map(|r| r.role_id.as_str()).collect();
        let current_role_ids: HashSet<&str> =
            current.roles.iter().map(|r| r.role_id.as_str()).collect();

        let mut removed_roles: Vec<&str> = baseline_role_ids
            .difference(&current_role_ids)
            .copied()
            .collect();
        removed_roles.sort();
        for role_id in removed_roles {
            violations.push(RoleRegistryViolation::RoleIdRemoved {
                role_id: RoleId(role_id.to_string()),
            });
        }

        let baseline_contracts = Self::contract_map(baseline);
        let current_contracts = Self::contract_map(current);

        let mut baseline_contract_ids: Vec<&str> = baseline_contracts.keys().copied().collect();
        baseline_contract_ids.sort();
        for contract_id in baseline_contract_ids {
            let baseline_hash = baseline_contracts
                .get(contract_id)
                .copied()
                .unwrap_or(ContractSurfaceHash([0; 32]));
            match current_contracts.get(contract_id).copied() {
                None => violations.push(RoleRegistryViolation::ContractIdRemoved {
                    contract_id: contract_id.to_string(),
                }),
                Some(current_hash) if current_hash != baseline_hash => {
                    violations.push(RoleRegistryViolation::ContractSurfaceDrift {
                        contract_id: contract_id.to_string(),
                        expected_hash: baseline_hash,
                        got_hash: current_hash,
                    })
                }
                _ => {}
            }
        }

        violations
    }

    fn validate_snapshot_integrity(snapshot: &RoleRegistrySnapshot) -> Vec<RoleRegistryViolation> {
        let mut violations = Vec::new();

        let mut role_counts: BTreeMap<&str, usize> = BTreeMap::new();
        for role in &snapshot.roles {
            let role_id = role.role_id.as_str();
            if role_id.trim().is_empty() {
                violations.push(RoleRegistryViolation::InvalidRoleId {
                    role_id: role_id.to_string(),
                });
                continue;
            }
            *role_counts.entry(role_id).or_insert(0) += 1;
        }

        for (role_id, count) in role_counts {
            if count > 1 {
                violations.push(RoleRegistryViolation::DuplicateRoleId {
                    role_id: RoleId(role_id.to_string()),
                });
            }
        }

        let mut contract_counts: BTreeMap<&str, usize> = BTreeMap::new();
        for contract in &snapshot.contracts {
            *contract_counts
                .entry(contract.contract_id.as_str())
                .or_insert(0) += 1;
        }
        for (contract_id, count) in contract_counts {
            if count > 1 {
                violations.push(RoleRegistryViolation::DuplicateContractId {
                    contract_id: contract_id.to_string(),
                });
            }
        }

        violations
    }

    fn contract_map(snapshot: &RoleRegistrySnapshot) -> HashMap<&str, ContractSurfaceHash> {
        snapshot
            .contracts
            .iter()
            .map(|c| (c.contract_id.as_str(), c.schema_hash))
            .collect()
    }
}

pub fn canonical_json_sha256(value: &Value) -> ContractSurfaceHash {
    let bytes = canonical_json_bytes(value);
    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    let digest = hasher.finalize();
    let mut out = [0u8; 32];
    out.copy_from_slice(&digest);
    ContractSurfaceHash(out)
}

fn canonical_json_bytes(value: &Value) -> Vec<u8> {
    let mut out = String::new();
    write_canonical_json_value(&mut out, value);
    out.push('\n');
    out.into_bytes()
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

#[derive(Debug, Clone, Default)]
pub struct RoleRegistryDiagnosticContext {
    pub wsid: Option<String>,
    pub job_id: Option<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum RoleRegistryDiagnosticError {
    #[error(transparent)]
    Build(#[from] crate::diagnostics::DiagnosticError),
    #[error(transparent)]
    Store(#[from] StorageError),
}

pub async fn record_role_registry_violation_diagnostic(
    diagnostics: &dyn DiagnosticsStore,
    violation: &RoleRegistryViolation,
    ctx: RoleRegistryDiagnosticContext,
) -> Result<uuid::Uuid, RoleRegistryDiagnosticError> {
    let input = DiagnosticInput {
        title: violation.diagnostic_title().to_string(),
        message: violation.diagnostic_message(),
        severity: DiagnosticSeverity::Error,
        source: DiagnosticSource::Validator,
        surface: DiagnosticSurface::System,
        tool: Some("role_registry_append_only".to_string()),
        code: Some(violation.diagnostic_code().to_string()),
        tags: Some(vec![
            "wp:WP-1-Role-Registry-AppendOnly-v1".to_string(),
            "role_registry".to_string(),
            "append_only".to_string(),
        ]),
        wsid: ctx.wsid,
        job_id: ctx.job_id,
        model_id: None,
        actor: Some(DiagnosticActor::System),
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
    diagnostics.record_diagnostic(diagnostic).await?;
    Ok(diagnostic_id)
}
