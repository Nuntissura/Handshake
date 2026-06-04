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

    let try_process =
        || -> Option<&'a dyn SandboxAdapter> { registry.process_tier.first().copied() };
    let try_wasm = || -> Option<&'a dyn SandboxAdapter> { registry.wasm.first().copied() };

    let record_fallback = |fallbacks: &mut Vec<AdapterFallbackEvidenceV1>,
                           from: AdapterIsolationTier,
                           adapter: &dyn SandboxAdapter,
                           availability_state: String| {
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
        fallbacks.push(AdapterFallbackEvidenceV1 {
            requested_tier: from,
            fell_back_to_tier: to,
            selected_adapter_id: adapter.kind().id,
            availability_state,
            reason,
        });
    };

    match preferred {
        AdapterIsolationTier::HardIsolation => {
            let hard_isolation_probes = probe_hard_isolation_once(registry);
            if let Some((a, _)) = hard_isolation_probes.iter().find(|(_, availability)| {
                matches!(availability, HardIsolationAvailability::Available { .. })
            }) {
                return Ok(SelectionResult {
                    adapter: a.as_sandbox_adapter(),
                    fallbacks,
                });
            }
            if let Some(a) = try_process() {
                record_fallback(
                    &mut fallbacks,
                    AdapterIsolationTier::HardIsolation,
                    a,
                    describe_hard_isolation_probes(&hard_isolation_probes),
                );
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
                record_fallback(
                    &mut fallbacks,
                    AdapterIsolationTier::Wasm,
                    a,
                    "no Wasm adapter registered".to_string(),
                );
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

fn probe_hard_isolation_once<'a>(
    registry: &'a AdapterRegistryView<'a>,
) -> Vec<(&'a dyn HardIsolationAdapter, HardIsolationAvailability)> {
    registry
        .hard_isolation
        .iter()
        .map(|adapter| (*adapter, adapter.probe_availability()))
        .collect()
}

fn describe_hard_isolation_probes(
    probes: &[(&dyn HardIsolationAdapter, HardIsolationAvailability)],
) -> String {
    if probes.is_empty() {
        return "no HardIsolation adapters registered".to_string();
    }
    probes
        .iter()
        .map(|(adapter, availability)| {
            format!("{}={}", adapter.kind().id, availability.short_label())
        })
        .collect::<Vec<_>>()
        .join(",")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    use crate::kernel::sandbox::adapter::{AdapterKind, AdapterRunOutcome};
    use crate::kernel::sandbox::hard_isolation_container::ContainerAdapterStub;
    use crate::kernel::sandbox::hard_isolation_microvm::MicroVmAdapterStub;
    use crate::kernel::sandbox::host_platform_probe::HostKind;
    use crate::kernel::sandbox::policy::SandboxPolicyV1;
    use crate::kernel::sandbox::policy_scoped_local::PolicyScopedLocalAdapter;
    use crate::kernel::sandbox::run::SandboxRunV1;
    use crate::kernel::sandbox::workspace::SandboxWorkspaceV1;

    struct CountingHardIsolationAdapter {
        probes: AtomicUsize,
    }

    impl CountingHardIsolationAdapter {
        fn new() -> Self {
            Self {
                probes: AtomicUsize::new(0),
            }
        }

        fn probes(&self) -> usize {
            self.probes.load(Ordering::SeqCst)
        }
    }

    impl SandboxAdapter for CountingHardIsolationAdapter {
        fn kind(&self) -> AdapterKind {
            AdapterKind {
                id: "counting_hard_isolation".to_string(),
                tier: AdapterIsolationTier::HardIsolation,
                version: 1,
                label: "counting hard-isolation probe".to_string(),
            }
        }

        fn run(
            &self,
            _run: &SandboxRunV1,
            _workspace: &SandboxWorkspaceV1,
            _policy: &SandboxPolicyV1,
        ) -> Result<AdapterRunOutcome, AdapterError> {
            Err(AdapterError::Unavailable(
                "counting adapter is probe-only".to_string(),
            ))
        }
    }

    impl HardIsolationAdapter for CountingHardIsolationAdapter {
        fn probe_availability(&self) -> HardIsolationAvailability {
            self.probes.fetch_add(1, Ordering::SeqCst);
            HardIsolationAvailability::Blocked {
                reason: "blocked for fallback evidence".to_string(),
                missing_dependency: "executor".to_string(),
            }
        }

        fn hard_isolation_tier_label(&self) -> &'static str {
            "counting"
        }

        fn as_sandbox_adapter(&self) -> &dyn SandboxAdapter {
            self
        }
    }

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
    fn hard_isolation_fallback_reuses_one_probe_result_for_selection_and_evidence() {
        let hard = CountingHardIsolationAdapter::new();
        let pol = SandboxPolicyV1::default_deny("baseline");
        let proc_adapter = PolicyScopedLocalAdapter::new(pol).unwrap();
        let registry = AdapterRegistryView {
            hard_isolation: vec![&hard],
            process_tier: vec![&proc_adapter],
            wasm: Vec::new(),
        };

        let r =
            select_adapter(&registry, AdapterIsolationTier::HardIsolation).expect("must fall back");

        assert_eq!(r.adapter.kind().tier, AdapterIsolationTier::Process);
        assert_eq!(hard.probes(), 1, "selection must not re-run runtime probes");
        assert_eq!(r.fallbacks.len(), 1);
        assert!(r.fallbacks[0]
            .availability_state
            .contains("counting_hard_isolation=BLOCKED"));
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
