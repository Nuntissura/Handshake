use std::{fs, path::PathBuf};

use handshake_core::sandbox::{
    AdapterCapabilities, AdapterId, GpuPassthrough, IsolationStrength, IsolationTier,
    ThroughputClass, WSL2_PODMAN_ADAPTER_ID,
};
use serde_json::json;

#[test]
fn sandbox_ipc_tests_tauri_bridge_registers_commands_and_state() {
    let repo = repo_root();
    let sandbox_rs =
        fs::read_to_string(repo.join("app/src-tauri/src/commands/sandbox.rs")).unwrap();
    let lib_rs = fs::read_to_string(repo.join("app/src-tauri/src/lib.rs")).unwrap();

    for command in [
        "kernel_sandbox_list_adapters",
        "kernel_sandbox_capabilities",
    ] {
        assert!(
            sandbox_rs.contains(&format!("pub async fn {command}")),
            "missing sandbox IPC command function {command}"
        );
        assert!(
            lib_rs.contains(&format!("commands::sandbox::{command}")),
            "missing invoke_handler registration for {command}"
        );
    }

    assert!(
        lib_rs.contains("mod commands") && lib_rs.contains("pub mod sandbox"),
        "app lib must declare the sandbox commands module"
    );
    assert!(
        lib_rs.contains("handshake_core::sandbox::build_default_registry()"),
        "app bootstrap must build the canonical sandbox registry"
    );
    assert!(
        lib_rs.contains(".manage(sandbox_registry)"),
        "app bootstrap must manage the sandbox registry as Tauri state"
    );
}

#[test]
fn sandbox_ipc_tests_payload_shape_matches_frontend_types() {
    let capabilities = AdapterCapabilities {
        adapter_id: AdapterId::new(WSL2_PODMAN_ADAPTER_ID),
        runtime_available: true,
        filesystem_isolation_strength: IsolationStrength::Strong,
        network_isolation_strength: IsolationStrength::Strong,
        gpu_passthrough: GpuPassthrough::None,
        stdio_throughput_class: ThroughputClass::High,
        win32_native_fidelity: false,
        cross_machine_portable: true,
        isolation_tier: IsolationTier::Tier1Container,
        requires_nested_virt: false,
        supports_snapshot: false,
        supports_persistent_exec: false,
        supports_warm_agent: false,
        supports_live_token_stream: false,
    };

    assert_eq!(
        serde_json::to_value(&capabilities).unwrap(),
        json!({
            "adapter_id": "wsl2_podman",
            "runtime_available": true,
            "filesystem_isolation_strength": "strong",
            "network_isolation_strength": "strong",
            "gpu_passthrough": "none",
            "stdio_throughput_class": "high",
            "win32_native_fidelity": false,
            "cross_machine_portable": true,
            "isolation_tier": "tier1_container",
            "requires_nested_virt": false,
            "supports_snapshot": false
        })
    );

    let frontend_types =
        fs::read_to_string(repo_root().join("app/src/components/diagnostics/sandbox/types.ts"))
            .unwrap();
    for field in [
        "adapter_id",
        "runtime_available",
        "filesystem_isolation_strength",
        "network_isolation_strength",
        "gpu_passthrough",
        "stdio_throughput_class",
        "win32_native_fidelity",
        "cross_machine_portable",
    ] {
        assert!(
            frontend_types.contains(field),
            "frontend AdapterCapabilities is missing backend field {field}"
        );
    }
}

fn repo_root() -> PathBuf {
    let mut current = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    loop {
        if current.join(".GOV").exists() {
            return current;
        }
        assert!(current.pop(), "repo root with .GOV not found");
    }
}
