//! Master Spec v02.187 §3.5 strong-isolation tier foundation tests.
//!
//! Covers the KERNEL-004 `sandbox/` trait-layer increment 1:
//!   - `IsolationTier::rank` ordering,
//!   - `TrustClass::default()` == `UntrustedAgent` (safe default),
//!   - `TrustClass::min_isolation_tier` mapping,
//!   - the three existing container adapters declare `Tier1Container`,
//!   - serde round-trip for both new enums.
//!
//! Selection-guard behaviour (Trusted selects Tier-1 OK; UntrustedAgent fails
//! loudly because only Tier-1 adapters exist) lives in
//! `sandbox_selection_tests.rs`.

use handshake_core::sandbox::{
    DockerAdapter, DockerConfig, GpuPassthrough, IsolationTier, SandboxAdapter, TrustClass,
    WindowsNativeJailAdapter, Wsl2PodmanAdapter, Wsl2PodmanConfig,
};

#[test]
fn isolation_tier_rank_is_strictly_ordered() {
    assert_eq!(IsolationTier::Tier1Container.rank(), 1);
    assert_eq!(IsolationTier::Tier2Syscall.rank(), 2);
    assert_eq!(IsolationTier::Tier3Microvm.rank(), 3);

    assert!(IsolationTier::Tier1Container.rank() < IsolationTier::Tier2Syscall.rank());
    assert!(IsolationTier::Tier2Syscall.rank() < IsolationTier::Tier3Microvm.rank());
}

#[test]
fn trust_class_default_is_untrusted_agent() {
    assert_eq!(TrustClass::default(), TrustClass::UntrustedAgent);
}

#[test]
fn trust_class_min_isolation_tier_mapping_matches_spec() {
    assert_eq!(
        TrustClass::Trusted.min_isolation_tier(),
        IsolationTier::Tier1Container
    );
    assert_eq!(
        TrustClass::Reviewed.min_isolation_tier(),
        IsolationTier::Tier1Container
    );
    assert_eq!(
        TrustClass::UntrustedAgent.min_isolation_tier(),
        IsolationTier::Tier3Microvm
    );
}

#[test]
fn existing_container_adapters_declare_tier1_container() {
    let docker = DockerAdapter::with_config_and_gpu_for_tests(
        DockerConfig::default(),
        GpuPassthrough::None,
    );
    assert_eq!(
        docker.capabilities().isolation_tier,
        IsolationTier::Tier1Container
    );

    let podman = Wsl2PodmanAdapter::with_config_and_gpu_for_tests(
        Wsl2PodmanConfig::default(),
        GpuPassthrough::None,
    );
    assert_eq!(
        podman.capabilities().isolation_tier,
        IsolationTier::Tier1Container
    );

    let windows_native = WindowsNativeJailAdapter::unavailable_for_current_host();
    assert_eq!(
        windows_native.capabilities().isolation_tier,
        IsolationTier::Tier1Container
    );
}

#[test]
fn isolation_tier_serde_round_trip_snake_case() {
    for (tier, expected) in [
        (IsolationTier::Tier1Container, "\"tier1_container\""),
        (IsolationTier::Tier2Syscall, "\"tier2_syscall\""),
        (IsolationTier::Tier3Microvm, "\"tier3_microvm\""),
    ] {
        let encoded = serde_json::to_string(&tier).expect("isolation tier serializes");
        assert_eq!(encoded, expected);
        let decoded: IsolationTier =
            serde_json::from_str(&encoded).expect("isolation tier deserializes");
        assert_eq!(decoded, tier);
    }

    assert!(
        serde_json::from_str::<IsolationTier>("\"tier4_unknown\"").is_err(),
        "unknown isolation tier variants must fail closed"
    );
}

#[test]
fn trust_class_serde_round_trip_snake_case() {
    for (class, expected) in [
        (TrustClass::Trusted, "\"trusted\""),
        (TrustClass::Reviewed, "\"reviewed\""),
        (TrustClass::UntrustedAgent, "\"untrusted_agent\""),
    ] {
        let encoded = serde_json::to_string(&class).expect("trust class serializes");
        assert_eq!(encoded, expected);
        let decoded: TrustClass = serde_json::from_str(&encoded).expect("trust class deserializes");
        assert_eq!(decoded, class);
    }

    assert!(
        serde_json::from_str::<TrustClass>("\"god_mode\"").is_err(),
        "unknown trust class variants must fail closed"
    );
}
