//! Canonical execution-policy reference resolution.
//!
//! Launch records store requested and effective references, but process
//! creation must never treat an arbitrary non-empty string as authority. This
//! registry is the shared fail-closed resolver used by Dexterity preflight and
//! the concrete Official-CLI process boundary.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};
use thiserror::Error;

use super::{
    AdapterCapabilities, AttachedNetworkMode, AttachedProcessSpec, IsolationTier, NetPolicy,
    RequiredCapability, ResourceLimits, ThroughputClass, TrustClass,
    HANDSHAKE_NATIVE_SANDBOX_ADAPTER_ID,
};

pub const EXECUTION_POLICY_SCHEMA_ID: &str = "hsk.resolved_execution_policy@1";
pub const EXECUTION_POLICY_SCHEMA_VERSION: u16 = 1;
pub const CLI_BRIDGE_POLICY_REVISION: u32 = 1;

pub const LOCAL_REQUESTED_EXECUTION_POLICY_REF: &str = "execution-policy://requested/local";
pub const LOCAL_EFFECTIVE_EXECUTION_POLICY_REF: &str = "execution-policy://effective/model_runtime";
pub const CLOUD_REQUESTED_EXECUTION_POLICY_REF: &str = "execution-policy://requested/cloud";
pub const CLOUD_EFFECTIVE_EXECUTION_POLICY_REF: &str = "execution-policy://effective/cloud_lane";
pub const CLI_BRIDGE_REQUESTED_EXECUTION_POLICY_REF: &str =
    "execution-policy://requested/cli_bridge";
pub const CLI_BRIDGE_EFFECTIVE_EXECUTION_POLICY_REF: &str =
    "execution-policy://effective/cli_bridge";
pub const HUMAN_REQUESTED_EXECUTION_POLICY_REF: &str = "execution-policy://requested/human";
pub const HUMAN_EFFECTIVE_EXECUTION_POLICY_REF: &str = "execution-policy://effective/operator";
pub const SUBAGENT_REQUESTED_EXECUTION_POLICY_REF: &str = "execution-policy://requested/subagent";
pub const SUBAGENT_EFFECTIVE_EXECUTION_POLICY_REF: &str =
    "execution-policy://effective/subagent_manager";
pub const VALIDATOR_REQUESTED_EXECUTION_POLICY_REF: &str = "execution-policy://requested/validator";
pub const VALIDATOR_EFFECTIVE_EXECUTION_POLICY_REF: &str =
    "execution-policy://effective/validator_runner";

/// Requested launch posture that must be bound to a registered policy before
/// executable inspection, sandbox selection, ledger START, or process spawn.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExecutionPolicyRequest {
    pub requested_ref: String,
    pub trust_class: TrustClass,
    pub isolation_tier: IsolationTier,
    pub required_capabilities: BTreeSet<RequiredCapability>,
    pub requested_net_policy: NetPolicy,
    pub effective_attached_network_mode: AttachedNetworkMode,
    pub resource_limits: ResourceLimits,
    pub startup_timeout_ms: u64,
}

/// Versioned, replayable policy resolution carried to the concrete attached
/// process boundary. This is the authority object; a free-form reference is
/// retained only as requested-input evidence.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResolvedExecutionPolicy {
    pub schema_id: String,
    pub schema_version: u16,
    pub policy_revision: u32,
    pub requested_ref: String,
    pub effective_ref: String,
    pub trust_class: TrustClass,
    pub isolation_tier: IsolationTier,
    pub required_capabilities: BTreeSet<RequiredCapability>,
    pub requested_net_policy: NetPolicy,
    pub effective_attached_network_mode: AttachedNetworkMode,
    pub resource_limits: ResourceLimits,
    pub startup_timeout_ms: u64,
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum ExecutionPolicyError {
    #[error("unknown, stale, or noncanonical execution-policy reference: {0}")]
    UnknownReference(String),
    #[error("execution-policy posture mismatch for {field}: expected {expected}, received {actual}")]
    PostureMismatch {
        field: &'static str,
        expected: String,
        actual: String,
    },
    #[error("execution-policy resource posture is invalid: {0}")]
    InvalidResourcePosture(String),
}

impl ResolvedExecutionPolicy {
    /// Resolve the sole production Official-CLI policy. The policy is exact on
    /// trust, tier, capability, and network posture so invocation-supplied
    /// fields cannot silently redefine its meaning.
    pub fn resolve_official_cli(
        request: ExecutionPolicyRequest,
    ) -> Result<Self, ExecutionPolicyError> {
        let effective_ref = resolve_execution_policy_ref(&request.requested_ref).ok_or_else(|| {
            ExecutionPolicyError::UnknownReference(request.requested_ref.clone())
        })?;
        if request.requested_ref != CLI_BRIDGE_REQUESTED_EXECUTION_POLICY_REF
            || effective_ref != CLI_BRIDGE_EFFECTIVE_EXECUTION_POLICY_REF
        {
            return Err(ExecutionPolicyError::UnknownReference(
                request.requested_ref,
            ));
        }
        require_exact(
            "trust_class",
            TrustClass::Trusted,
            request.trust_class,
        )?;
        require_exact(
            "isolation_tier",
            IsolationTier::Tier1Container,
            request.isolation_tier,
        )?;
        let canonical_capabilities =
            BTreeSet::from([RequiredCapability::HighStdioThroughput]);
        require_exact(
            "required_capabilities",
            canonical_capabilities.clone(),
            request.required_capabilities.clone(),
        )?;
        require_exact(
            "requested_net_policy",
            NetPolicy::HostInherited,
            request.requested_net_policy.clone(),
        )?;
        require_exact(
            "effective_attached_network_mode",
            AttachedNetworkMode::OutboundInternetClient,
            request.effective_attached_network_mode,
        )?;
        if request.startup_timeout_ms == 0 {
            return Err(ExecutionPolicyError::InvalidResourcePosture(
                "startup_timeout_ms must be greater than zero".to_string(),
            ));
        }
        match request.resource_limits.timeout_ms {
            Some(timeout_ms) if timeout_ms > 0 => {}
            _ => {
                return Err(ExecutionPolicyError::InvalidResourcePosture(
                    "invocation timeout_ms must be present and greater than zero".to_string(),
                ));
            }
        }

        Ok(Self {
            schema_id: EXECUTION_POLICY_SCHEMA_ID.to_string(),
            schema_version: EXECUTION_POLICY_SCHEMA_VERSION,
            policy_revision: CLI_BRIDGE_POLICY_REVISION,
            requested_ref: request.requested_ref,
            effective_ref: effective_ref.to_string(),
            trust_class: request.trust_class,
            isolation_tier: request.isolation_tier,
            required_capabilities: request.required_capabilities,
            requested_net_policy: request.requested_net_policy,
            effective_attached_network_mode: request.effective_attached_network_mode,
            resource_limits: request.resource_limits,
            startup_timeout_ms: request.startup_timeout_ms,
        })
    }

    /// Bind the resolved policy to the exact attached spec before the adapter
    /// is invoked. Any drift between selection input and spawn input fails
    /// before the process side-effect boundary.
    pub fn validate_attached_spec(
        &self,
        spec: &AttachedProcessSpec,
    ) -> Result<(), ExecutionPolicyError> {
        require_exact(
            "effective_ref",
            self.effective_ref.as_str(),
            spec.execution_policy_ref.as_str(),
        )?;
        require_exact("trust_class", self.trust_class, spec.trust_class)?;
        require_exact(
            "isolation_tier",
            self.isolation_tier,
            spec.requested_isolation_tier,
        )?;
        require_exact(
            "required_capabilities",
            self.required_capabilities.clone(),
            spec.required_capabilities.clone(),
        )?;
        require_exact(
            "requested_net_policy",
            self.requested_net_policy.clone(),
            spec.requested_net_policy.clone(),
        )?;
        require_exact(
            "effective_attached_network_mode",
            self.effective_attached_network_mode,
            spec.network_mode,
        )?;
        require_exact(
            "resource_limits",
            self.resource_limits.clone(),
            spec.resource_limits.clone(),
        )?;
        require_exact(
            "startup_timeout_ms",
            self.startup_timeout_ms,
            spec.startup_timeout_ms,
        )
    }

    /// Bind the policy to the adapter's actual capability snapshot before
    /// START persistence or process creation.
    pub fn validate_adapter_capabilities(
        &self,
        capabilities: &AdapterCapabilities,
    ) -> Result<(), ExecutionPolicyError> {
        require_exact(
            "sandbox_adapter_id",
            HANDSHAKE_NATIVE_SANDBOX_ADAPTER_ID,
            capabilities.adapter_id.as_str(),
        )?;
        if !capabilities.runtime_available {
            return Err(ExecutionPolicyError::PostureMismatch {
                field: "runtime_available",
                expected: "true".to_string(),
                actual: "false".to_string(),
            });
        }
        if capabilities.isolation_tier.rank() < self.isolation_tier.rank() {
            return Err(ExecutionPolicyError::PostureMismatch {
                field: "adapter_isolation_tier",
                expected: format!(">= {:?}", self.isolation_tier),
                actual: format!("{:?}", capabilities.isolation_tier),
            });
        }
        if self
            .required_capabilities
            .contains(&RequiredCapability::HighStdioThroughput)
            && capabilities.stdio_throughput_class != ThroughputClass::High
        {
            return Err(ExecutionPolicyError::PostureMismatch {
                field: "stdio_throughput_class",
                expected: "High".to_string(),
                actual: format!("{:?}", capabilities.stdio_throughput_class),
            });
        }
        Ok(())
    }
}

fn require_exact<T>(
    field: &'static str,
    expected: T,
    actual: T,
) -> Result<(), ExecutionPolicyError>
where
    T: PartialEq + std::fmt::Debug,
{
    if expected == actual {
        Ok(())
    } else {
        Err(ExecutionPolicyError::PostureMismatch {
            field,
            expected: format!("{expected:?}"),
            actual: format!("{actual:?}"),
        })
    }
}

/// Resolve an exact registered requested reference to the effective policy.
/// Whitespace, case changes, ad-hoc test references, and retired/unknown
/// references fail closed as `None`.
pub fn resolve_execution_policy_ref(requested_ref: &str) -> Option<&'static str> {
    match requested_ref {
        LOCAL_REQUESTED_EXECUTION_POLICY_REF => Some(LOCAL_EFFECTIVE_EXECUTION_POLICY_REF),
        CLOUD_REQUESTED_EXECUTION_POLICY_REF => Some(CLOUD_EFFECTIVE_EXECUTION_POLICY_REF),
        CLI_BRIDGE_REQUESTED_EXECUTION_POLICY_REF => {
            Some(CLI_BRIDGE_EFFECTIVE_EXECUTION_POLICY_REF)
        }
        HUMAN_REQUESTED_EXECUTION_POLICY_REF => Some(HUMAN_EFFECTIVE_EXECUTION_POLICY_REF),
        SUBAGENT_REQUESTED_EXECUTION_POLICY_REF => Some(SUBAGENT_EFFECTIVE_EXECUTION_POLICY_REF),
        VALIDATOR_REQUESTED_EXECUTION_POLICY_REF => Some(VALIDATOR_EFFECTIVE_EXECUTION_POLICY_REF),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_registered_reference_resolves() {
        assert_eq!(
            resolve_execution_policy_ref(CLI_BRIDGE_REQUESTED_EXECUTION_POLICY_REF),
            Some(CLI_BRIDGE_EFFECTIVE_EXECUTION_POLICY_REF)
        );
    }

    #[test]
    fn missing_unknown_stale_and_noncanonical_references_fail_closed() {
        for invalid in [
            "",
            "execution-policy://test/official-cli",
            "execution-policy://requested/retired-cli-v0",
            " execution-policy://requested/cli_bridge",
            "execution-policy://requested/CLI_BRIDGE",
        ] {
            assert_eq!(resolve_execution_policy_ref(invalid), None, "{invalid}");
        }
    }

    fn official_cli_request() -> ExecutionPolicyRequest {
        ExecutionPolicyRequest {
            requested_ref: CLI_BRIDGE_REQUESTED_EXECUTION_POLICY_REF.to_string(),
            trust_class: TrustClass::Trusted,
            isolation_tier: IsolationTier::Tier1Container,
            required_capabilities: BTreeSet::from([
                RequiredCapability::HighStdioThroughput,
            ]),
            requested_net_policy: NetPolicy::HostInherited,
            effective_attached_network_mode: AttachedNetworkMode::OutboundInternetClient,
            resource_limits: ResourceLimits {
                timeout_ms: Some(1_000),
                ..ResourceLimits::default()
            },
            startup_timeout_ms: 60_000,
        }
    }

    #[test]
    fn typed_official_cli_policy_rejects_posture_redefinition() {
        let mut request = official_cli_request();
        request.trust_class = TrustClass::Reviewed;
        assert!(matches!(
            ResolvedExecutionPolicy::resolve_official_cli(request),
            Err(ExecutionPolicyError::PostureMismatch {
                field: "trust_class",
                ..
            })
        ));

        let mut request = official_cli_request();
        request.requested_net_policy = NetPolicy::DenyAll;
        assert!(matches!(
            ResolvedExecutionPolicy::resolve_official_cli(request),
            Err(ExecutionPolicyError::PostureMismatch {
                field: "requested_net_policy",
                ..
            })
        ));

        let mut request = official_cli_request();
        request.required_capabilities.clear();
        assert!(matches!(
            ResolvedExecutionPolicy::resolve_official_cli(request),
            Err(ExecutionPolicyError::PostureMismatch {
                field: "required_capabilities",
                ..
            })
        ));
    }

    #[test]
    fn typed_official_cli_policy_preserves_version_and_resource_posture() {
        let resolved = ResolvedExecutionPolicy::resolve_official_cli(official_cli_request())
            .expect("canonical posture resolves");
        assert_eq!(resolved.schema_id, EXECUTION_POLICY_SCHEMA_ID);
        assert_eq!(resolved.schema_version, EXECUTION_POLICY_SCHEMA_VERSION);
        assert_eq!(resolved.policy_revision, CLI_BRIDGE_POLICY_REVISION);
        assert_eq!(resolved.resource_limits.timeout_ms, Some(1_000));
        assert_eq!(resolved.startup_timeout_ms, 60_000);
    }

}
