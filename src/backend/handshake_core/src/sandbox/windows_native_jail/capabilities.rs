use crate::sandbox::{
    AdapterCapabilities, AdapterId, GpuPassthrough, IsolationStrength, ThroughputClass,
    WINDOWS_NATIVE_JAIL_ADAPTER_ID,
};

pub fn windows_native_jail_target_capabilities() -> AdapterCapabilities {
    AdapterCapabilities {
        adapter_id: AdapterId::new(WINDOWS_NATIVE_JAIL_ADAPTER_ID),
        runtime_available: false,
        filesystem_isolation_strength: IsolationStrength::VeryStrong,
        network_isolation_strength: IsolationStrength::Strong,
        gpu_passthrough: GpuPassthrough::VendorAgnostic,
        stdio_throughput_class: ThroughputClass::Medium,
        win32_native_fidelity: true,
        cross_machine_portable: false,
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
    }
}

pub fn windows_native_jail_runtime_capabilities() -> AdapterCapabilities {
    AdapterCapabilities {
        runtime_available: true,
        ..windows_native_jail_target_capabilities()
    }
}
