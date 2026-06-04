export type IsolationStrength = "weak" | "strong" | "very_strong";

export type GpuPassthrough = "none" | "nvidia_cuda" | "vendor_agnostic";

export type ThroughputClass = "low" | "medium" | "high";

// Master Spec v02.187 §3.5.5 isolation tiers (serde snake_case).
export type IsolationTier = "tier1_container" | "tier2_syscall" | "tier3_microvm";

export interface AdapterCapabilities {
  adapter_id: string;
  runtime_available: boolean;
  filesystem_isolation_strength: IsolationStrength;
  network_isolation_strength: IsolationStrength;
  gpu_passthrough: GpuPassthrough;
  stdio_throughput_class: ThroughputClass;
  win32_native_fidelity: boolean;
  cross_machine_portable: boolean;
  isolation_tier: IsolationTier;
  requires_nested_virt: boolean;
  supports_snapshot: boolean;
}
