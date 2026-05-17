//! MT-017 Legacy compatibility blocker check.
//!
//! Acceptance (MT-017.json): "missing APIs produce BLOCKED with evidence, not
//! parallel implementations." Sandbox code that needs a prerequisite API (a
//! sibling kernel module, a storage path, a runtime feature flag) must call
//! `assert_or_block(...)` so that an absent prerequisite halts the run with a
//! typed BLOCKED record. Coders MUST NOT inline a parallel implementation
//! when the BLOCKED path fires.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlockedEvidenceV1 {
    pub blocker_id: String,
    pub missing_api: String,
    pub expected_module_path: String,
    pub upstream_packet: String,
    pub evidence_summary: String,
    pub remediation_hint: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CompatCheck {
    /// API present — caller may proceed.
    Present,
    /// API absent — caller MUST stop and surface the evidence.
    Blocked(BlockedEvidenceV1),
}

impl CompatCheck {
    pub fn is_blocked(&self) -> bool {
        matches!(self, CompatCheck::Blocked(_))
    }
}

/// Check a single prerequisite. `is_present` is a closure so call sites can
/// keep their dependency on the actual API local to their module.
pub fn assert_or_block<F: FnOnce() -> bool>(
    missing_api: impl Into<String>,
    expected_module_path: impl Into<String>,
    upstream_packet: impl Into<String>,
    remediation_hint: impl Into<String>,
    is_present: F,
) -> CompatCheck {
    let missing_api = missing_api.into();
    let expected_module_path = expected_module_path.into();
    let upstream_packet = upstream_packet.into();
    let remediation_hint = remediation_hint.into();
    if is_present() {
        CompatCheck::Present
    } else {
        let blocker_id = format!("BLK-{}-{}", upstream_packet, sanitise(&missing_api));
        let evidence_summary = format!(
            "Missing API `{}` at `{}`; blocked by upstream packet `{}`.",
            missing_api, expected_module_path, upstream_packet
        );
        CompatCheck::Blocked(BlockedEvidenceV1 {
            blocker_id,
            missing_api,
            expected_module_path,
            upstream_packet,
            evidence_summary,
            remediation_hint,
        })
    }
}

fn sanitise(s: &str) -> String {
    s.chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn present_api_returns_present() {
        let r = assert_or_block(
            "AdapterX::run",
            "kernel::sandbox::adapter",
            "WP-KERNEL-003",
            "implement AdapterX::run",
            || true,
        );
        assert_eq!(r, CompatCheck::Present);
        assert!(!r.is_blocked());
    }

    #[test]
    fn missing_api_returns_blocked_with_evidence() {
        let r = assert_or_block(
            "HardIsolationAdapter::spawn",
            "kernel::sandbox::adapter",
            "WP-KERNEL-003-C",
            "wait for hard-isolation MT batch",
            || false,
        );
        match r {
            CompatCheck::Blocked(e) => {
                assert!(e.blocker_id.starts_with("BLK-WP-KERNEL-003-C-"));
                assert!(e.evidence_summary.contains("HardIsolationAdapter::spawn"));
                assert!(e.evidence_summary.contains("kernel::sandbox::adapter"));
                assert_eq!(e.upstream_packet, "WP-KERNEL-003-C");
                assert!(!e.remediation_hint.is_empty());
            }
            other => panic!("expected Blocked, got {:?}", other),
        }
    }

    #[test]
    fn sanitise_keeps_alnum_only() {
        assert_eq!(sanitise("Foo::Bar-1"), "Foo__Bar_1");
    }
}
