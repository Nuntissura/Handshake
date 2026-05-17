//! MT-021 SandboxPolicy Default-Deny construction.
//!
//! Acceptance (MT-021.json): "implement default-deny policy construction.
//! Acceptance: omitted policy fields deny access."
//!
//! Batch B's `SandboxPolicyV1::default_deny(...)` covers the capability-decision
//! axis. This module extends default-deny to every *other* policy surface this
//! batch introduces (filesystem scope, network gate, process exec allowlist,
//! environment/secret redaction, resource caps) by providing a single
//! constructor that returns a fully-zeroed `SandboxPolicyExtensionsV1` plus a
//! deterministic audit function (`audit_default_deny`) that checks an existing
//! policy bundle and reports the first non-deny field.
//!
//! Downstream MTs (022..029) all read their sub-policy from
//! `SandboxPolicyExtensionsV1`. Any field a caller forgets to set deny-overrides
//! by construction.

use serde::{Deserialize, Serialize};

use super::policy::{CapabilityDecision, SandboxPolicyV1};

/// Sub-policy collection extending `SandboxPolicyV1` with the new surfaces this
/// batch needs. All fields are deny-by-default: empty allow-lists, no
/// granted-network entries, no exec descriptors, redaction enabled, caps zero
/// means "use deterministic safe defaults" (see MT-026).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct SandboxPolicyExtensionsV1 {
    pub filesystem: FilesystemScopeV1,
    pub network: NetworkGateV1,
    pub process_exec: ProcessExecAllowlistV1,
    pub redaction: EnvRedactionV1,
    pub resource_caps: ResourceCapsV1,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct FilesystemScopeV1 {
    /// Absolute or workspace-relative roots the sandbox may read from.
    /// Empty list = no reads permitted.
    pub read_roots: Vec<String>,
    /// Roots the sandbox may write to. Empty list = no writes permitted.
    pub write_roots: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct NetworkGateV1 {
    /// Approved network grants. Each carries an evidence reference so a grant
    /// without provenance is invalid. Empty list = no network permitted.
    pub grants: Vec<NetworkGrantV1>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NetworkGrantV1 {
    pub host_pattern: String,
    pub approval_ref: String,
    pub provenance_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct ProcessExecAllowlistV1 {
    /// Registered command descriptors. Empty list = no process execution
    /// permitted; raw shell strings are rejected at run time.
    pub commands: Vec<CommandDescriptorRefV1>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommandDescriptorRefV1 {
    pub descriptor_id: String,
    pub purpose_tag: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EnvRedactionV1 {
    /// Redaction is on by default; flipping this to `false` requires explicit
    /// caller intent recorded in `provenance_note` on the parent policy.
    pub enabled: bool,
    /// Extra patterns to redact beyond the built-in defaults.
    pub extra_patterns: Vec<String>,
}

impl Default for EnvRedactionV1 {
    fn default() -> Self {
        Self {
            enabled: true,
            extra_patterns: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct ResourceCapsV1 {
    pub wall_ms: Option<u64>,
    pub cpu_ms: Option<u64>,
    pub memory_bytes: Option<u64>,
    pub file_descriptors: Option<u32>,
    pub output_bytes: Option<u64>,
}

/// Combined policy bundle the rest of the sandbox uses.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SandboxPolicyBundleV1 {
    pub core: SandboxPolicyV1,
    pub extensions: SandboxPolicyExtensionsV1,
}

impl SandboxPolicyBundleV1 {
    /// Construct a fully default-deny bundle. Every omitted field is the deny
    /// state: empty FS roots, empty network grants, empty exec allowlist,
    /// redaction enabled, caps unset.
    pub fn default_deny(name: impl Into<String>) -> Self {
        Self {
            core: SandboxPolicyV1::default_deny(name),
            extensions: SandboxPolicyExtensionsV1::default(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DefaultDenyAudit {
    Ok,
    Violation { field: &'static str, detail: String },
}

/// Walk a bundle and verify every surface is in its deny-by-default state.
/// Returns the first violation found. Used by tests and by the policy ingest
/// path to catch callers that forget to set a deny default.
pub fn audit_default_deny(b: &SandboxPolicyBundleV1) -> DefaultDenyAudit {
    if b.core.default_decision != CapabilityDecision::Deny {
        return DefaultDenyAudit::Violation {
            field: "core.default_decision",
            detail: format!("{:?}", b.core.default_decision),
        };
    }
    if !b.core.overrides.is_empty() {
        return DefaultDenyAudit::Violation {
            field: "core.overrides",
            detail: format!(
                "default-deny bundle must not pre-grant any capability; got {} override(s)",
                b.core.overrides.len()
            ),
        };
    }
    if !b.extensions.filesystem.read_roots.is_empty() {
        return DefaultDenyAudit::Violation {
            field: "extensions.filesystem.read_roots",
            detail: format!(
                "default-deny bundle must list zero FS read roots; got {}",
                b.extensions.filesystem.read_roots.len()
            ),
        };
    }
    if !b.extensions.filesystem.write_roots.is_empty() {
        return DefaultDenyAudit::Violation {
            field: "extensions.filesystem.write_roots",
            detail: format!(
                "default-deny bundle must list zero FS write roots; got {}",
                b.extensions.filesystem.write_roots.len()
            ),
        };
    }
    if !b.extensions.network.grants.is_empty() {
        return DefaultDenyAudit::Violation {
            field: "extensions.network.grants",
            detail: "default-deny bundle must carry zero network grants".to_string(),
        };
    }
    if !b.extensions.process_exec.commands.is_empty() {
        return DefaultDenyAudit::Violation {
            field: "extensions.process_exec.commands",
            detail: "default-deny bundle must carry zero exec descriptors".to_string(),
        };
    }
    if !b.extensions.redaction.enabled {
        return DefaultDenyAudit::Violation {
            field: "extensions.redaction.enabled",
            detail: "redaction must be ON in default-deny bundle".to_string(),
        };
    }
    DefaultDenyAudit::Ok
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kernel::sandbox::policy::SandboxCapability;

    #[test]
    fn omitted_fields_deny_access() {
        let b = SandboxPolicyBundleV1::default_deny("baseline");
        // Core capability decisions deny everything.
        for cap in SandboxCapability::ALL {
            assert_eq!(b.core.decide(*cap), CapabilityDecision::Deny);
        }
        // Extension fields are empty / on as appropriate.
        assert!(b.extensions.filesystem.read_roots.is_empty());
        assert!(b.extensions.filesystem.write_roots.is_empty());
        assert!(b.extensions.network.grants.is_empty());
        assert!(b.extensions.process_exec.commands.is_empty());
        assert!(b.extensions.redaction.enabled);
        assert!(b.extensions.resource_caps.wall_ms.is_none());
        assert!(b.extensions.resource_caps.cpu_ms.is_none());
        assert!(b.extensions.resource_caps.memory_bytes.is_none());
        assert!(b.extensions.resource_caps.file_descriptors.is_none());
        assert!(b.extensions.resource_caps.output_bytes.is_none());
    }

    #[test]
    fn default_deny_bundle_passes_audit() {
        let b = SandboxPolicyBundleV1::default_deny("baseline");
        assert_eq!(audit_default_deny(&b), DefaultDenyAudit::Ok);
    }

    #[test]
    fn pregranted_network_fails_audit() {
        let mut b = SandboxPolicyBundleV1::default_deny("baseline");
        b.extensions.network.grants.push(NetworkGrantV1 {
            host_pattern: "*.example.com".into(),
            approval_ref: "APR-1".into(),
            provenance_ref: "PRV-1".into(),
        });
        match audit_default_deny(&b) {
            DefaultDenyAudit::Violation { field, .. } => {
                assert_eq!(field, "extensions.network.grants");
            }
            DefaultDenyAudit::Ok => panic!("audit must catch pre-granted network"),
        }
    }

    #[test]
    fn disabled_redaction_fails_audit() {
        let mut b = SandboxPolicyBundleV1::default_deny("baseline");
        b.extensions.redaction.enabled = false;
        assert!(matches!(
            audit_default_deny(&b),
            DefaultDenyAudit::Violation {
                field: "extensions.redaction.enabled",
                ..
            }
        ));
    }

    #[test]
    fn nonempty_overrides_fail_audit() {
        let mut b = SandboxPolicyBundleV1::default_deny("baseline");
        b.core
            .overrides
            .push((SandboxCapability::Network, CapabilityDecision::Allow));
        assert!(matches!(
            audit_default_deny(&b),
            DefaultDenyAudit::Violation {
                field: "core.overrides",
                ..
            }
        ));
    }
}
