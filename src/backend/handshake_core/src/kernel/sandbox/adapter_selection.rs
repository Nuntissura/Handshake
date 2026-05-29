//! Deterministic adapter selection with typed fallback evidence.
//!
//! `select_adapter(...)` picks an adapter from a registry given a tier
//! preference. When the preferred tier is UNSUPPORTED or BLOCKED, the selector
//! falls back to the next-best tier and records the fallback as a typed
//! `AdapterFallbackEvidenceV1` so audits and DCC can see the downgrade.
//!
//! Fallback chain:
//!   HardIsolation -> Process -> (none; return error)
//!   Wasm          -> Process -> (none; return error)
//!
//! Fallback is never silent: a non-empty `fallbacks` vec means the operator's
//! preferred tier was not available.

use serde::{Deserialize, Serialize};

use super::adapter::{AdapterError, AdapterIsolationTier, SandboxAdapter};
use super::hard_isolation::{HardIsolationAdapter, HardIsolationAvailability};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AdapterFallbackEvidenceV1 {
    pub requested_tier: AdapterIsolationTier,
    pub fell_back_to_tier: AdapterIsolationTier,
    pub selected_adapter_id: String,
    pub availability_state: String,
    pub reason: String,
}

pub struct SelectionResult<'a> {
    pub adapter: &'a dyn SandboxAdapter,
    pub fallbacks: Vec<AdapterFallbackEvidenceV1>,
}

/// Pluggable registry view: hard-isolation adapters first (so the selector can
/// probe them), then process-tier adapters.
pub struct AdapterRegistryView<'a> {
    pub hard_isolation: Vec<&'a dyn HardIsolationAdapter>,
    pub process_tier: Vec<&'a dyn SandboxAdapter>,
    pub wasm: Vec<&'a dyn SandboxAdapter>,
}

impl<'a> AdapterRegistryView<'a> {
    pub fn empty() -> Self {
        Self {
            hard_isolation: Vec::new(),
            process_tier: Vec::new(),
            wasm: Vec::new(),
        }
    }
}

pub fn select_adapter<'a>(
    registry: &'a AdapterRegistryView<'a>,
    preferred: AdapterIsolationTier,
) -> Result<SelectionResult<'a>, AdapterError> {
    let mut fallbacks: Vec<AdapterFallbackEvidenceV1> = Vec::new();

    let try_hard_isolation = || -> Option<&'a dyn SandboxAdapter> {
        registry
            .hard_isolation
            .iter()
            .find_map(|a| match a.probe_availability() {
                HardIsolationAvailability::Available { .. } => Some(a.as_sandbox_adapter()),
                _ => None,
            })
    };
    let try_process =
        || -> Option<&'a dyn SandboxAdapter> { registry.process_tier.first().copied() };
    let try_wasm = || -> Option<&'a dyn SandboxAdapter> { registry.wasm.first().copied() };

    let record_fallback = |fallbacks: &mut Vec<AdapterFallbackEvidenceV1>,
                           from: AdapterIsolationTier,
                           adapter: &dyn SandboxAdapter| {
        let to = adapter.kind().tier;
        let reason = match from {
            AdapterIsolationTier::HardIsolation => {
                "preferred HardIsolation tier was UNSUPPORTED/BLOCKED; falling back".to_string()
            }
            AdapterIsolationTier::Wasm => {
                "preferred Wasm tier had no available adapter; falling back to process".to_string()
            }
            AdapterIsolationTier::Process => "no fallback needed".to_string(),
        };
        let state = describe_unavailable_chain(registry, from);
        fallbacks.push(AdapterFallbackEvidenceV1 {
            requested_tier: from,
            fell_back_to_tier: to,
            selected_adapter_id: adapter.kind().id,
            availability_state: state,
            reason,
        });
    };

    match preferred {
        AdapterIsolationTier::HardIsolation => {
            if let Some(a) = try_hard_isolation() {
                return Ok(SelectionResult {
                    adapter: a,
                    fallbacks,
                });
            }
            if let Some(a) = try_process() {
                record_fallback(&mut fallbacks, AdapterIsolationTier::HardIsolation, a);
                return Ok(SelectionResult {
                    adapter: a,
                    fallbacks,
                });
            }
            Err(AdapterError::Unavailable(
                "no HardIsolation adapter available and no Process fallback registered".into(),
            ))
        }
        AdapterIsolationTier::Wasm => {
            if let Some(a) = try_wasm() {
                return Ok(SelectionResult {
                    adapter: a,
                    fallbacks,
                });
            }
            if let Some(a) = try_process() {
                record_fallback(&mut fallbacks, AdapterIsolationTier::Wasm, a);
                return Ok(SelectionResult {
                    adapter: a,
                    fallbacks,
                });
            }
            Err(AdapterError::Unavailable(
                "no Wasm adapter available and no Process fallback registered".into(),
            ))
        }
        AdapterIsolationTier::Process => match try_process() {
            Some(a) => Ok(SelectionResult {
                adapter: a,
                fallbacks,
            }),
            None => Err(AdapterError::Unavailable(
                "no Process-tier adapter registered".into(),
            )),
        },
    }
}

fn describe_unavailable_chain(
    registry: &AdapterRegistryView<'_>,
    from: AdapterIsolationTier,
) -> String {
    match from {
        AdapterIsolationTier::HardIsolation => {
            let states: Vec<String> = registry
                .hard_isolation
                .iter()
                .map(|a| format!("{}={}", a.kind().id, a.probe_availability().short_label()))
                .collect();
            if states.is_empty() {
                "no HardIsolation adapters registered".to_string()
            } else {
                states.join(",")
            }
        }
        AdapterIsolationTier::Wasm => "no Wasm adapter registered".to_string(),
        AdapterIsolationTier::Process => "process-tier requested".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kernel::sandbox::hard_isolation_container::ContainerAdapterStub;
    use crate::kernel::sandbox::hard_isolation_microvm::MicroVmAdapterStub;
    use crate::kernel::sandbox::host_platform_probe::HostKind;
    use crate::kernel::sandbox::policy::SandboxPolicyV1;
    use crate::kernel::sandbox::policy_scoped_local::PolicyScopedLocalAdapter;

    #[test]
    fn hard_isolation_request_falls_back_to_process_with_typed_evidence() {
        let container = ContainerAdapterStub::new();
        let microvm = MicroVmAdapterStub::with_forced_host(HostKind::Windows);
        let pol = SandboxPolicyV1::default_deny("baseline");
        let proc_adapter = PolicyScopedLocalAdapter::new(pol).unwrap();

        let registry = AdapterRegistryView {
            hard_isolation: vec![&container, &microvm],
            process_tier: vec![&proc_adapter],
            wasm: Vec::new(),
        };
        let r =
            select_adapter(&registry, AdapterIsolationTier::HardIsolation).expect("must fall back");
        assert_eq!(r.adapter.kind().tier, AdapterIsolationTier::Process);
        assert_eq!(
            r.fallbacks.len(),
            1,
            "fallback MUST be recorded as evidence"
        );
        let ev = &r.fallbacks[0];
        assert_eq!(ev.requested_tier, AdapterIsolationTier::HardIsolation);
        assert_eq!(ev.fell_back_to_tier, AdapterIsolationTier::Process);
        assert!(!ev.availability_state.is_empty());
        // Both HI adapters appear in the availability_state summary.
        assert!(ev.availability_state.contains("hard_isolation_container"));
        assert!(ev.availability_state.contains("hard_isolation_microvm"));
    }

    #[test]
    fn process_request_with_process_adapter_succeeds_without_fallback() {
        let pol = SandboxPolicyV1::default_deny("baseline");
        let proc_adapter = PolicyScopedLocalAdapter::new(pol).unwrap();
        let registry = AdapterRegistryView {
            hard_isolation: Vec::new(),
            process_tier: vec![&proc_adapter],
            wasm: Vec::new(),
        };
        let r = select_adapter(&registry, AdapterIsolationTier::Process).unwrap();
        assert!(r.fallbacks.is_empty());
        assert_eq!(r.adapter.kind().tier, AdapterIsolationTier::Process);
    }

    #[test]
    fn empty_registry_returns_unavailable_error() {
        // OK arm of select_adapter is SelectionResult<'_> which holds
        // `&dyn SandboxAdapter` (no Debug bound on trait). Pattern-match
        // instead of `unwrap_err()` to avoid the Debug requirement.
        let registry = AdapterRegistryView::empty();
        let result = select_adapter(&registry, AdapterIsolationTier::HardIsolation);
        match result {
            Ok(_) => panic!("expected select_adapter to return Err on empty registry"),
            Err(AdapterError::Unavailable(msg)) => assert!(msg.contains("HardIsolation")),
            Err(other) => panic!("expected Unavailable, got {:?}", other),
        }
    }

    #[test]
    fn wasm_request_falls_back_to_process_with_evidence() {
        let pol = SandboxPolicyV1::default_deny("baseline");
        let proc_adapter = PolicyScopedLocalAdapter::new(pol).unwrap();
        let registry = AdapterRegistryView {
            hard_isolation: Vec::new(),
            process_tier: vec![&proc_adapter],
            wasm: Vec::new(),
        };
        let r = select_adapter(&registry, AdapterIsolationTier::Wasm).unwrap();
        assert_eq!(r.fallbacks.len(), 1);
        assert_eq!(r.fallbacks[0].requested_tier, AdapterIsolationTier::Wasm);
    }
}
