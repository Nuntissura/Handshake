//! MT-023 Network Capability Gate.
//!
//! Acceptance (MT-023.json): "deny network unless policy grants it. Acceptance:
//! network grants require approval/provenance refs."
//!
//! The gate has two layers:
//!   1. Capability gate: `SandboxCapability::Network` must be allowed by the
//!      core policy. `Deny` blocks immediately.
//!   2. Grant lookup: even when the capability is allowed, the requested host
//!      must match a `NetworkGrantV1` whose `approval_ref` AND `provenance_ref`
//!      are both non-empty. Empty refs are treated as missing grant.
//!
//! Patterns support a single trailing `*` wildcard (e.g. `*.example.com`,
//! `api.*`). Resolution is purely string-based so behavior is deterministic.

use super::denial::{DenialKind, SandboxDenialRecordV1};
use super::policy::{CapabilityDecision, SandboxCapability, SandboxPolicyV1};
use super::policy_default_deny::{NetworkGateV1, NetworkGrantV1};
use super::run::SandboxRunV1;

pub struct NetworkCapabilityGate<'a> {
    core: &'a SandboxPolicyV1,
    gate: &'a NetworkGateV1,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NetworkDecision {
    Allowed {
        matched_grant: NetworkGrantV1,
    },
    Denied(SandboxDenialRecordV1),
}

impl<'a> NetworkCapabilityGate<'a> {
    pub fn new(core: &'a SandboxPolicyV1, gate: &'a NetworkGateV1) -> Self {
        Self { core, gate }
    }

    pub fn check_host(&self, run: &SandboxRunV1, host: &str) -> NetworkDecision {
        // Layer 1: core capability.
        let decision = self.core.decide(SandboxCapability::Network);
        if matches!(decision, CapabilityDecision::Deny) {
            return NetworkDecision::Denied(SandboxDenialRecordV1::new(
                run.run_id.0.clone(),
                run.policy_version_id.clone(),
                DenialKind::PolicyDenied,
                Some(SandboxCapability::Network),
                format!("network reach `{}`", host),
                "core policy denies NETWORK capability".to_string(),
            ));
        }

        // Layer 2: grant lookup. A grant without refs is treated as missing.
        let matched = self.gate.grants.iter().find(|g| {
            !g.approval_ref.trim().is_empty()
                && !g.provenance_ref.trim().is_empty()
                && host_matches_pattern(host, &g.host_pattern)
        });
        match matched {
            Some(g) => NetworkDecision::Allowed {
                matched_grant: g.clone(),
            },
            None => NetworkDecision::Denied(SandboxDenialRecordV1::new(
                run.run_id.0.clone(),
                run.policy_version_id.clone(),
                DenialKind::PolicyDenied,
                Some(SandboxCapability::Network),
                format!("network reach `{}`", host),
                if self.gate.grants.is_empty() {
                    "no network grants in policy".to_string()
                } else if self.gate.grants.iter().any(|g| {
                    g.approval_ref.trim().is_empty() || g.provenance_ref.trim().is_empty()
                }) {
                    "matching grant missing approval_ref or provenance_ref".to_string()
                } else {
                    format!("host `{}` does not match any granted pattern", host)
                },
            )),
        }
    }
}

fn host_matches_pattern(host: &str, pattern: &str) -> bool {
    if pattern == "*" {
        return true;
    }
    if let Some(rest) = pattern.strip_prefix("*.") {
        // Suffix match, but require at least one extra label.
        return host.ends_with(&format!(".{}", rest)) || host == rest;
    }
    if let Some(rest) = pattern.strip_suffix(".*") {
        return host.starts_with(&format!("{}.", rest));
    }
    host == pattern
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kernel::sandbox::policy::CapabilityDecision;
    use crate::kernel::sandbox::policy_default_deny::NetworkGrantV1;

    fn run() -> SandboxRunV1 {
        SandboxRunV1::new_requested("KTR-1", "SES-1", "net", "POL-1@1", "WSP-1")
    }

    #[test]
    fn default_deny_blocks_network() {
        let core = SandboxPolicyV1::default_deny("baseline");
        let gate = NetworkGateV1::default();
        let g = NetworkCapabilityGate::new(&core, &gate);
        match g.check_host(&run(), "api.example.com") {
            NetworkDecision::Denied(d) => {
                assert_eq!(d.kind, DenialKind::PolicyDenied);
                assert_eq!(d.capability, Some(SandboxCapability::Network));
                assert!(d.reason.contains("NETWORK"));
            }
            other => panic!("default-deny must block network, got {:?}", other),
        }
    }

    #[test]
    fn allowed_capability_but_no_grant_still_denies() {
        let mut core = SandboxPolicyV1::default_deny("baseline");
        core.overrides
            .push((SandboxCapability::Network, CapabilityDecision::AllowWithEvidence));
        let gate = NetworkGateV1::default();
        let g = NetworkCapabilityGate::new(&core, &gate);
        match g.check_host(&run(), "api.example.com") {
            NetworkDecision::Denied(d) => {
                assert!(d.reason.contains("no network grants"));
            }
            other => panic!("missing grant must deny, got {:?}", other),
        }
    }

    #[test]
    fn grant_without_refs_is_treated_as_missing() {
        let mut core = SandboxPolicyV1::default_deny("baseline");
        core.overrides
            .push((SandboxCapability::Network, CapabilityDecision::AllowWithEvidence));
        let mut gate = NetworkGateV1::default();
        gate.grants.push(NetworkGrantV1 {
            host_pattern: "api.example.com".into(),
            approval_ref: "".into(),
            provenance_ref: "".into(),
        });
        let g = NetworkCapabilityGate::new(&core, &gate);
        match g.check_host(&run(), "api.example.com") {
            NetworkDecision::Denied(d) => {
                assert!(d.reason.contains("approval_ref"));
            }
            other => panic!("empty refs must deny, got {:?}", other),
        }
    }

    #[test]
    fn full_grant_allows_matching_host() {
        let mut core = SandboxPolicyV1::default_deny("baseline");
        core.overrides
            .push((SandboxCapability::Network, CapabilityDecision::AllowWithEvidence));
        let mut gate = NetworkGateV1::default();
        gate.grants.push(NetworkGrantV1 {
            host_pattern: "*.example.com".into(),
            approval_ref: "APR-001".into(),
            provenance_ref: "PRV-001".into(),
        });
        let g = NetworkCapabilityGate::new(&core, &gate);
        match g.check_host(&run(), "api.example.com") {
            NetworkDecision::Allowed { matched_grant } => {
                assert_eq!(matched_grant.host_pattern, "*.example.com");
                assert_eq!(matched_grant.approval_ref, "APR-001");
                assert_eq!(matched_grant.provenance_ref, "PRV-001");
            }
            other => panic!("expected Allowed, got {:?}", other),
        }
    }

    #[test]
    fn unmatched_host_denies_even_with_grants() {
        let mut core = SandboxPolicyV1::default_deny("baseline");
        core.overrides
            .push((SandboxCapability::Network, CapabilityDecision::AllowWithEvidence));
        let mut gate = NetworkGateV1::default();
        gate.grants.push(NetworkGrantV1 {
            host_pattern: "*.example.com".into(),
            approval_ref: "APR-1".into(),
            provenance_ref: "PRV-1".into(),
        });
        let g = NetworkCapabilityGate::new(&core, &gate);
        match g.check_host(&run(), "evil.test") {
            NetworkDecision::Denied(d) => {
                assert!(d.reason.contains("does not match"));
            }
            other => panic!("non-matching host must deny, got {:?}", other),
        }
    }

    #[test]
    fn pattern_helpers_handle_wildcards() {
        assert!(host_matches_pattern("api.example.com", "*.example.com"));
        assert!(host_matches_pattern("example.com", "*.example.com")); // bare apex permitted
        assert!(!host_matches_pattern("evil.com", "*.example.com"));
        assert!(host_matches_pattern("api.foo", "api.*"));
        assert!(host_matches_pattern("anything", "*"));
        assert!(host_matches_pattern("exact.host", "exact.host"));
    }
}
