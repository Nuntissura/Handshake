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

use unicode_normalization::UnicodeNormalization;

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
    Allowed { matched_grant: NetworkGrantV1 },
    Denied(SandboxDenialRecordV1),
}

/// Typed denials produced by host-shape validation before grant lookup runs.
/// These exist in addition to the policy-level `SandboxDenialRecordV1` so
/// callers wiring up grant builders can distinguish shape errors from policy
/// outcomes without parsing reason strings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NetworkGateDenial {
    HostShapeInvalid { reason: String },
}

/// Errors raised by the strict `NetworkGrant::build` constructor. The legacy
/// `NetworkGrantV1` struct (in `policy_default_deny`) remains the on-disk
/// schema; this wrapper enforces additional invariants (wildcard requires
/// approval, optional apex exclusion) at construction time so policies that
/// flow through it cannot land in inconsistent shapes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NetworkGrantBuildError {
    WildcardRequiresOperatorApproval,
    EmptyHostPattern,
    HostShapeInvalid { reason: String },
}

/// Strict grant builder. Once constructed, `NetworkGrant::into_v1` projects
/// back to the persistence type. `exclude_apex` controls whether
/// `*.example.com` also matches the bare apex `example.com` (default `false`
/// preserves the historical permissive behavior of `host_matches_pattern`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NetworkGrant {
    pub host_pattern: String,
    pub approval_ref: Option<String>,
    pub provenance_ref: Option<String>,
    pub exclude_apex: bool,
}

impl NetworkGrant {
    /// Validating constructor. Refuses bare `*` wildcards without an
    /// operator-supplied `approval_ref`, and runs the same NFKC + IDNA host
    /// normalisation used by `check_host` so policies cannot smuggle in a
    /// host pattern that visually masquerades as another.
    pub fn build(
        host_pattern: impl Into<String>,
        approval_ref: Option<String>,
        provenance_ref: Option<String>,
        exclude_apex: bool,
    ) -> Result<Self, NetworkGrantBuildError> {
        let pattern_raw = host_pattern.into();
        let pattern_trimmed = pattern_raw.trim();
        if pattern_trimmed.is_empty() {
            return Err(NetworkGrantBuildError::EmptyHostPattern);
        }
        if pattern_trimmed == "*"
            && approval_ref
                .as_ref()
                .map(|s| s.trim().is_empty())
                .unwrap_or(true)
        {
            return Err(NetworkGrantBuildError::WildcardRequiresOperatorApproval);
        }
        // Normalise the non-wildcard portion only.
        let normalised = normalize_host_pattern(pattern_trimmed)
            .map_err(|reason| NetworkGrantBuildError::HostShapeInvalid { reason })?;
        Ok(Self {
            host_pattern: normalised,
            approval_ref,
            provenance_ref,
            exclude_apex,
        })
    }

    pub fn into_v1(self) -> NetworkGrantV1 {
        NetworkGrantV1 {
            host_pattern: self.host_pattern,
            approval_ref: self.approval_ref.unwrap_or_default(),
            provenance_ref: self.provenance_ref.unwrap_or_default(),
        }
    }
}

impl<'a> NetworkCapabilityGate<'a> {
    pub fn new(core: &'a SandboxPolicyV1, gate: &'a NetworkGateV1) -> Self {
        Self { core, gate }
    }

    /// Check that the given `host` is permitted by the active policy + grants.
    ///
    /// Apex behavior: a wildcard grant of `*.example.com` matches the bare
    /// apex `example.com` by default. The strict-builder `NetworkGrant` type
    /// can opt out of that behavior via `exclude_apex`, but legacy
    /// `NetworkGrantV1` storage cannot express the flag and therefore retains
    /// permissive apex matching when consulted via this method.
    ///
    /// Host inputs are NFKC-normalised and IDNA-encoded to ASCII before any
    /// pattern comparison; visually-confusable Unicode hosts are matched at
    /// their canonical ASCII (Punycode) form. Host strings that fail shape
    /// validation (control characters, malformed labels, oversize labels)
    /// produce a typed denial without consulting any grant.
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

        // Layer 1b: host shape. NFKC + IDNA-to-ASCII normalise the host first;
        // anything that fails shape validation is a typed denial.
        let host_canonical = match normalize_host_input(host) {
            Ok(h) => h,
            Err(reason) => {
                return NetworkDecision::Denied(SandboxDenialRecordV1::new(
                    run.run_id.0.clone(),
                    run.policy_version_id.clone(),
                    DenialKind::PolicyDenied,
                    Some(SandboxCapability::Network),
                    format!("network reach `{}`", host),
                    format!("host shape invalid: {}", reason),
                ));
            }
        };

        // Layer 2: grant lookup. A grant without refs is treated as missing.
        let matched = self.gate.grants.iter().find(|g| {
            !g.approval_ref.trim().is_empty()
                && !g.provenance_ref.trim().is_empty()
                && host_matches_pattern_canonical(&host_canonical, &g.host_pattern, false)
        });
        match matched {
            Some(g) => NetworkDecision::Allowed {
                matched_grant: g.clone(),
            },
            None => {
                NetworkDecision::Denied(SandboxDenialRecordV1::new(
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
                ))
            }
        }
    }

    /// Strict variant: consults the typed `NetworkGrant` builder (which carries
    /// the `exclude_apex` flag) instead of the on-disk `NetworkGrantV1` list.
    /// Use this path when authority code holds grants built via
    /// `NetworkGrant::build`.
    pub fn check_host_against(
        &self,
        run: &SandboxRunV1,
        host: &str,
        grants: &[NetworkGrant],
    ) -> NetworkDecision {
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
        let host_canonical = match normalize_host_input(host) {
            Ok(h) => h,
            Err(reason) => {
                return NetworkDecision::Denied(SandboxDenialRecordV1::new(
                    run.run_id.0.clone(),
                    run.policy_version_id.clone(),
                    DenialKind::PolicyDenied,
                    Some(SandboxCapability::Network),
                    format!("network reach `{}`", host),
                    format!("host shape invalid: {}", reason),
                ));
            }
        };
        let matched = grants.iter().find(|g| {
            host_matches_pattern_canonical(&host_canonical, &g.host_pattern, g.exclude_apex)
        });
        match matched {
            Some(g) => NetworkDecision::Allowed {
                matched_grant: g.clone().into_v1(),
            },
            None => NetworkDecision::Denied(SandboxDenialRecordV1::new(
                run.run_id.0.clone(),
                run.policy_version_id.clone(),
                DenialKind::PolicyDenied,
                Some(SandboxCapability::Network),
                format!("network reach `{}`", host),
                format!("host `{}` does not match any typed grant", host),
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

/// Canonicalised pattern match. Both `host` and `pattern` are expected to
/// already be in canonical ASCII form (NFKC + IDNA-to-ASCII). The wildcard
/// portion (`*.` or `.*`) is recognised and the apex match for `*.example.com`
/// is suppressed when `exclude_apex` is true.
fn host_matches_pattern_canonical(host: &str, pattern: &str, exclude_apex: bool) -> bool {
    if pattern == "*" {
        return true;
    }
    if let Some(rest) = pattern.strip_prefix("*.") {
        let rest_canon = normalize_host_pattern_label(rest);
        if host.ends_with(&format!(".{}", rest_canon)) {
            return true;
        }
        if !exclude_apex && host == rest_canon {
            return true;
        }
        return false;
    }
    if let Some(rest) = pattern.strip_suffix(".*") {
        let rest_canon = normalize_host_pattern_label(rest);
        return host.starts_with(&format!("{}.", rest_canon));
    }
    host == pattern
}

/// NFKC + IDNA-to-ASCII normalise a host input. Returns the canonical ASCII
/// (Punycode) form. Rejects control characters, empty hosts, oversize labels,
/// and total lengths outside 1..=253 octets.
fn normalize_host_input(host: &str) -> Result<String, String> {
    if host.is_empty() {
        return Err("empty host".into());
    }
    if host.bytes().any(|b| matches!(b, 0x00..=0x1F | 0x7F)) {
        return Err("control character in host".into());
    }
    let nfkc: String = host.nfkc().collect();
    let ascii =
        idna::domain_to_ascii(&nfkc).map_err(|e| format!("IDNA encoding failed: {:?}", e))?;
    validate_ascii_host_shape(&ascii)?;
    Ok(ascii.to_ascii_lowercase())
}

/// Normalise the non-wildcard portion of a host pattern.
fn normalize_host_pattern(pattern: &str) -> Result<String, String> {
    if pattern == "*" {
        return Ok(pattern.to_string());
    }
    let (prefix, body, suffix) = if let Some(rest) = pattern.strip_prefix("*.") {
        ("*.", rest, "")
    } else if let Some(rest) = pattern.strip_suffix(".*") {
        ("", rest, ".*")
    } else {
        ("", pattern, "")
    };
    if body.is_empty() {
        return Err("empty host pattern body".into());
    }
    let normalised_body = normalize_host_input(body)?;
    Ok(format!("{prefix}{normalised_body}{suffix}"))
}

fn normalize_host_pattern_label(label: &str) -> String {
    normalize_host_input(label).unwrap_or_else(|_| label.to_ascii_lowercase())
}

fn validate_ascii_host_shape(host: &str) -> Result<(), String> {
    let len = host.len();
    if !(1..=253).contains(&len) {
        return Err(format!("host length {} outside 1..=253", len));
    }
    if host.starts_with('.') || host.ends_with('.') {
        return Err("host has leading or trailing dot".into());
    }
    for label in host.split('.') {
        if label.is_empty() {
            return Err("empty label".into());
        }
        if label.len() > 63 {
            return Err(format!("label `{}` exceeds 63 octets", label));
        }
        if !label.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
            return Err(format!("label `{}` has non-LDH characters", label));
        }
    }
    Ok(())
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
        core.overrides.push((
            SandboxCapability::Network,
            CapabilityDecision::Allow(crate::kernel::sandbox::policy::CapabilityGrant {
                capability: SandboxCapability::Network,
                evidence_ref: crate::kernel::sandbox::policy::CapabilityEvidenceRef::new(
                    "ART-net-test",
                ),
                approval_ref: Some(crate::kernel::sandbox::policy::OperatorApprovalRef::new(
                    "APR-net-test",
                )),
            }),
        ));
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
        core.overrides.push((
            SandboxCapability::Network,
            CapabilityDecision::Allow(crate::kernel::sandbox::policy::CapabilityGrant {
                capability: SandboxCapability::Network,
                evidence_ref: crate::kernel::sandbox::policy::CapabilityEvidenceRef::new(
                    "ART-net-test",
                ),
                approval_ref: Some(crate::kernel::sandbox::policy::OperatorApprovalRef::new(
                    "APR-net-test",
                )),
            }),
        ));
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
        core.overrides.push((
            SandboxCapability::Network,
            CapabilityDecision::Allow(crate::kernel::sandbox::policy::CapabilityGrant {
                capability: SandboxCapability::Network,
                evidence_ref: crate::kernel::sandbox::policy::CapabilityEvidenceRef::new(
                    "ART-net-test",
                ),
                approval_ref: Some(crate::kernel::sandbox::policy::OperatorApprovalRef::new(
                    "APR-net-test",
                )),
            }),
        ));
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
        core.overrides.push((
            SandboxCapability::Network,
            CapabilityDecision::Allow(crate::kernel::sandbox::policy::CapabilityGrant {
                capability: SandboxCapability::Network,
                evidence_ref: crate::kernel::sandbox::policy::CapabilityEvidenceRef::new(
                    "ART-net-test",
                ),
                approval_ref: Some(crate::kernel::sandbox::policy::OperatorApprovalRef::new(
                    "APR-net-test",
                )),
            }),
        ));
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
    fn wildcard_grant_without_approval_refused() {
        let err = NetworkGrant::build("*", None, Some("PRV-1".into()), false).unwrap_err();
        assert_eq!(
            err,
            NetworkGrantBuildError::WildcardRequiresOperatorApproval
        );
        let err2 =
            NetworkGrant::build("*", Some("".into()), Some("PRV-1".into()), false).unwrap_err();
        assert_eq!(
            err2,
            NetworkGrantBuildError::WildcardRequiresOperatorApproval
        );
        // With approval_ref present, wildcard is allowed.
        let ok =
            NetworkGrant::build("*", Some("APR-OP-1".into()), Some("PRV-1".into()), false).unwrap();
        assert_eq!(ok.host_pattern, "*");
    }

    #[test]
    fn cyrillic_homograph_host_normalized_and_matched() {
        // Construct "exаmple.com" with Cyrillic small `а` (U+0430). After NFKC
        // it stays Cyrillic; IDNA encodes to Punycode. A grant for the real
        // ASCII "example.com" must therefore NOT match the homograph host —
        // they canonicalise to different ASCII forms, which is the whole point.
        let mut core = SandboxPolicyV1::default_deny("baseline");
        core.overrides.push((
            SandboxCapability::Network,
            CapabilityDecision::Allow(crate::kernel::sandbox::policy::CapabilityGrant {
                capability: SandboxCapability::Network,
                evidence_ref: crate::kernel::sandbox::policy::CapabilityEvidenceRef::new(
                    "ART-net-test",
                ),
                approval_ref: Some(crate::kernel::sandbox::policy::OperatorApprovalRef::new(
                    "APR-net-test",
                )),
            }),
        ));
        let mut gate = NetworkGateV1::default();
        gate.grants.push(NetworkGrantV1 {
            host_pattern: "example.com".into(),
            approval_ref: "APR-1".into(),
            provenance_ref: "PRV-1".into(),
        });
        let g = NetworkCapabilityGate::new(&core, &gate);
        let homograph: String = ['e', 'x', '\u{0430}', 'm', 'p', 'l', 'e', '.', 'c', 'o', 'm']
            .iter()
            .collect();
        match g.check_host(&run(), &homograph) {
            NetworkDecision::Denied(_) => {}
            other => panic!(
                "Cyrillic homograph host must not match ASCII `example.com` grant, got {:?}",
                other
            ),
        }
        // The plain ASCII host still matches.
        match g.check_host(&run(), "example.com") {
            NetworkDecision::Allowed { matched_grant } => {
                assert_eq!(matched_grant.host_pattern, "example.com");
            }
            other => panic!("ASCII example.com must match, got {:?}", other),
        }
    }

    #[test]
    fn host_with_newline_rejected() {
        let mut core = SandboxPolicyV1::default_deny("baseline");
        core.overrides.push((
            SandboxCapability::Network,
            CapabilityDecision::Allow(crate::kernel::sandbox::policy::CapabilityGrant {
                capability: SandboxCapability::Network,
                evidence_ref: crate::kernel::sandbox::policy::CapabilityEvidenceRef::new(
                    "ART-net-test",
                ),
                approval_ref: Some(crate::kernel::sandbox::policy::OperatorApprovalRef::new(
                    "APR-net-test",
                )),
            }),
        ));
        let mut gate = NetworkGateV1::default();
        gate.grants.push(NetworkGrantV1 {
            host_pattern: "*".into(),
            approval_ref: "APR-1".into(),
            provenance_ref: "PRV-1".into(),
        });
        let g = NetworkCapabilityGate::new(&core, &gate);
        match g.check_host(&run(), "api.example.com\nGET / HTTP/1.1") {
            NetworkDecision::Denied(d) => {
                assert!(
                    d.reason.contains("host shape invalid"),
                    "expected host shape error, got reason: {}",
                    d.reason
                );
            }
            other => panic!("host with newline must be denied, got {:?}", other),
        }
    }

    #[test]
    fn apex_excluded_when_flag_set() {
        let mut core = SandboxPolicyV1::default_deny("baseline");
        core.overrides.push((
            SandboxCapability::Network,
            CapabilityDecision::Allow(crate::kernel::sandbox::policy::CapabilityGrant {
                capability: SandboxCapability::Network,
                evidence_ref: crate::kernel::sandbox::policy::CapabilityEvidenceRef::new(
                    "ART-net-test",
                ),
                approval_ref: Some(crate::kernel::sandbox::policy::OperatorApprovalRef::new(
                    "APR-net-test",
                )),
            }),
        ));
        let gate_v1 = NetworkGateV1::default();
        let g = NetworkCapabilityGate::new(&core, &gate_v1);
        let strict = vec![
            NetworkGrant::build(
                "*.example.com",
                Some("APR-1".into()),
                Some("PRV-1".into()),
                true,
            )
            .unwrap(),
        ];
        // Subdomain matches.
        match g.check_host_against(&run(), "api.example.com", &strict) {
            NetworkDecision::Allowed { .. } => {}
            other => panic!("subdomain must match strict grant, got {:?}", other),
        }
        // Apex must NOT match when exclude_apex=true.
        match g.check_host_against(&run(), "example.com", &strict) {
            NetworkDecision::Denied(_) => {}
            other => panic!("apex must be excluded when flag set, got {:?}", other),
        }
        // Sanity: with exclude_apex=false the apex does match.
        let permissive = vec![
            NetworkGrant::build(
                "*.example.com",
                Some("APR-1".into()),
                Some("PRV-1".into()),
                false,
            )
            .unwrap(),
        ];
        match g.check_host_against(&run(), "example.com", &permissive) {
            NetworkDecision::Allowed { .. } => {}
            other => panic!("apex must match when exclude_apex=false, got {:?}", other),
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
