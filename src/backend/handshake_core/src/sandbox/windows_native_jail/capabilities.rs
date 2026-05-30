use crate::sandbox::{
    AdapterCapabilities, AdapterId, GpuPassthrough, IsolationStrength, IsolationTier,
    ThroughputClass, WINDOWS_NATIVE_JAIL_ADAPTER_ID,
};

pub fn windows_native_jail_target_capabilities() -> AdapterCapabilities {
    AdapterCapabilities {
        adapter_id: AdapterId::new(WINDOWS_NATIVE_JAIL_ADAPTER_ID),
        runtime_available: false,
        filesystem_isolation_strength: IsolationStrength::VeryStrong,
        network_isolation_strength: IsolationStrength::Strong,
        // MT-046: WindowsNativeJailAdapter ships ZERO GPU plumbing (no D3D/DirectX/CUDA/
        // adapter-LUID code in windows_native_jail/). Declaring VendorAgnostic here was an
        // overclaim that violated the spec 3.5.4/3.6 truthful-declaration requirement and made
        // selection.rs (capability_satisfied / VendorAgnosticGpu) falsely match a GPU-required
        // job to this adapter, spawning it with no GPU access. Declare `None` until real GPU
        // passthrough is implemented. runtime_capabilities() inherits this value via struct update.
        gpu_passthrough: GpuPassthrough::None,
        stdio_throughput_class: ThroughputClass::Medium,
        win32_native_fidelity: true,
        cross_machine_portable: false,
        isolation_tier: IsolationTier::Tier1Container,
        requires_nested_virt: false,
        supports_snapshot: false,
    }
}

pub fn windows_native_jail_unavailable_capabilities() -> AdapterCapabilities {
    AdapterCapabilities {
        adapter_id: AdapterId::new(WINDOWS_NATIVE_JAIL_ADAPTER_ID),
        runtime_available: false,
        filesystem_isolation_strength: IsolationStrength::Weak,
        network_isolation_strength: IsolationStrength::Weak,
        gpu_passthrough: GpuPassthrough::None,
        stdio_throughput_class: ThroughputClass::Low,
        win32_native_fidelity: false,
        cross_machine_portable: false,
        isolation_tier: IsolationTier::Tier1Container,
        requires_nested_virt: false,
        supports_snapshot: false,
    }
}

pub fn windows_native_jail_runtime_capabilities() -> AdapterCapabilities {
    AdapterCapabilities {
        runtime_available: true,
        ..windows_native_jail_target_capabilities()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sandbox::RequiredCapability;

    /// Mirror of `selection::capability_satisfied` for the GPU arms (that fn is
    /// private to selection.rs). MT-046 proof must verify the *consume* semantics:
    /// a GPU-required capability must be unsatisfied by these caps.
    fn gpu_capability_satisfied(
        required: RequiredCapability,
        caps: &AdapterCapabilities,
    ) -> bool {
        match required {
            RequiredCapability::NvidiaCudaPassthrough => {
                caps.gpu_passthrough == GpuPassthrough::NvidiaCuda
            }
            RequiredCapability::VendorAgnosticGpu => caps.gpu_passthrough != GpuPassthrough::None,
            other => panic!("non-GPU capability {other:?} not covered by MT-046 proof"),
        }
    }

    #[test]
    fn mt046_target_capabilities_declare_no_gpu_passthrough() {
        // MT-046: ZERO GPU plumbing ships in windows_native_jail/, so the
        // truthful declaration is GpuPassthrough::None until real passthrough exists.
        let target = windows_native_jail_target_capabilities();
        assert_eq!(target.gpu_passthrough, GpuPassthrough::None);
    }

    #[test]
    fn mt046_runtime_capabilities_declare_no_gpu_passthrough() {
        // runtime_capabilities() inherits gpu_passthrough from target via struct update.
        let runtime = windows_native_jail_runtime_capabilities();
        assert_eq!(runtime.gpu_passthrough, GpuPassthrough::None);
    }

    #[test]
    fn mt046_gpu_required_job_no_longer_matches_native_jail_runtime() {
        // Proves selection.rs no longer matches a GPU-required job to this adapter:
        // with gpu_passthrough = None, both the VendorAgnosticGpu and
        // NvidiaCudaPassthrough arms of capability_satisfied evaluate to false.
        let runtime = windows_native_jail_runtime_capabilities();
        assert!(
            !gpu_capability_satisfied(RequiredCapability::VendorAgnosticGpu, &runtime),
            "VendorAgnosticGpu must be unsatisfied: adapter has no GPU plumbing"
        );
        assert!(
            !gpu_capability_satisfied(RequiredCapability::NvidiaCudaPassthrough, &runtime),
            "NvidiaCudaPassthrough must be unsatisfied: adapter has no GPU plumbing"
        );
    }
}
