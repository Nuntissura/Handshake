use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Canonical executable check result status.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "status", content = "details", rename_all = "snake_case")]
pub enum CheckResult {
    Pass(CheckPassDetails),
    Fail(CheckFailDetails),
    Blocked(CheckBlockedDetails),
    AdvisoryOnly(CheckAdvisoryOnlyDetails),
    Unsupported(CheckUnsupportedDetails),
}

/// Check descriptor consumed by the check runner.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CheckDescriptor {
    pub check_id: Uuid,
    pub name: String,
    pub check_kind: String,
    #[serde(default)]
    pub profile: Option<String>,
    #[serde(default)]
    pub tool_id: Option<String>,
    #[serde(default)]
    pub timeout_ms: Option<u64>,
    #[serde(default)]
    pub required_capabilities: Vec<String>,
    #[serde(default)]
    pub parameters: BTreeMap<String, String>,
}

impl CheckDescriptor {
    pub fn new(
        check_id: Uuid,
        name: impl Into<String>,
        check_kind: impl Into<String>,
    ) -> Self {
        Self {
            check_id,
            name: name.into(),
            check_kind: check_kind.into(),
            profile: None,
            tool_id: None,
            timeout_ms: None,
            required_capabilities: Vec::new(),
            parameters: BTreeMap::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CheckPassDetails {
    pub summary: String,
    #[serde(default)]
    pub evidence_artifact_id: Option<String>,
    #[serde(default)]
    pub checks_passed: usize,
    #[serde(default)]
    pub duration_ms: Option<u64>,
}

impl CheckPassDetails {
    pub fn with_summary(summary: impl Into<String>) -> Self {
        Self {
            summary: summary.into(),
            evidence_artifact_id: None,
            checks_passed: 0,
            duration_ms: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CheckFailDetails {
    pub reason: String,
    #[serde(default)]
    pub failed_checks: Vec<String>,
    #[serde(default)]
    pub remediation: Option<String>,
    #[serde(default)]
    pub checks_failed: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CheckBlockedDetails {
    pub reason: String,
    #[serde(default)]
    pub missing_capabilities: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CheckAdvisoryOnlyDetails {
    pub note: String,
    #[serde(default)]
    pub advisories: Vec<String>,
    #[serde(default)]
    pub evidence_artifact_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CheckUnsupportedDetails {
    pub check_kind: String,
    pub reason: String,
    #[serde(default)]
    pub remediation: Option<String>,
    #[serde(default)]
    pub supported_kinds: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn check_result_roundtrips_all_variants() -> Result<(), Box<dyn std::error::Error>> {
        let pass = CheckResult::Pass(CheckPassDetails::with_summary("all checks passed"));
        let fail = CheckResult::Fail(CheckFailDetails {
            reason: "violations found".to_string(),
            failed_checks: vec!["rule-a".to_string(), "rule-b".to_string()],
            remediation: Some("Fix failing rules".to_string()),
            checks_failed: 2,
        });
        let blocked = CheckResult::Blocked(CheckBlockedDetails {
            reason: "missing capability governance.check.run".to_string(),
            missing_capabilities: vec!["governance.check.run".to_string()],
        });
        let advisory = CheckResult::AdvisoryOnly(CheckAdvisoryOnlyDetails {
            note: "best effort check".to_string(),
            advisories: vec!["policy not enforced".to_string()],
            evidence_artifact_id: Some("art-123".to_string()),
        });
        let unsupported = CheckResult::Unsupported(CheckUnsupportedDetails {
            check_kind: "external_tool".to_string(),
            reason: "not supported in this runtime".to_string(),
            remediation: Some("enable adapter module".to_string()),
            supported_kinds: vec!["native".to_string(), "policy".to_string()],
        });

        let values = vec![
            ("pass", pass),
            ("fail", fail),
            ("blocked", blocked),
            ("advisory_only", advisory),
            ("unsupported", unsupported),
        ];

        for (expected_tag, result) in values {
            let raw = serde_json::to_string(&result)?;
            let value = serde_json::from_str::<serde_json::Value>(&raw)?;
            assert_eq!(value["status"], json!(expected_tag));
            let repaired: CheckResult = serde_json::from_str(&raw)?;
            assert_eq!(repaired, result);
        }

        Ok(())
    }

    #[test]
    fn check_descriptor_has_reasonable_default_values() -> Result<(), Box<dyn std::error::Error>> {
        let raw = json!({
            "check_id": Uuid::new_v4().to_string(),
            "name": "unit",
            "check_kind": "native"
        })
        .to_string();

        let descriptor: CheckDescriptor = serde_json::from_str(&raw)?;
        assert_eq!(descriptor.required_capabilities.len(), 0);
        assert_eq!(descriptor.parameters.len(), 0);
        assert!(descriptor.timeout_ms.is_none());
        assert!(descriptor.profile.is_none());
        assert!(descriptor.tool_id.is_none());

        let roundtrip: CheckDescriptor = serde_json::from_str(&serde_json::to_string(&descriptor)?)?;
        assert_eq!(descriptor, roundtrip);
        Ok(())
    }
}
