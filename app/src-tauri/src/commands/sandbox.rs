use std::sync::Arc;

use handshake_core::sandbox::{AdapterCapabilities, AdapterId, SandboxAdapterRegistry};
use tauri::State;

pub const KERNEL_SANDBOX_LIST_ADAPTERS_IPC_CHANNEL: &str = "kernel_sandbox_list_adapters";
pub const KERNEL_SANDBOX_CAPABILITIES_IPC_CHANNEL: &str = "kernel_sandbox_capabilities";

#[tauri::command]
pub async fn kernel_sandbox_list_adapters(
    registry: State<'_, Arc<SandboxAdapterRegistry>>,
) -> Result<Vec<AdapterCapabilities>, String> {
    let _ = KERNEL_SANDBOX_LIST_ADAPTERS_IPC_CHANNEL;
    Ok(list_adapters(registry.inner().as_ref()))
}

#[tauri::command(rename_all = "snake_case")]
pub async fn kernel_sandbox_capabilities(
    adapter_id: String,
    registry: State<'_, Arc<SandboxAdapterRegistry>>,
) -> Result<AdapterCapabilities, String> {
    let _ = KERNEL_SANDBOX_CAPABILITIES_IPC_CHANNEL;
    adapter_capabilities(&adapter_id, registry.inner().as_ref())
}

pub fn list_adapters(registry: &SandboxAdapterRegistry) -> Vec<AdapterCapabilities> {
    registry.list()
}

pub fn adapter_capabilities(
    adapter_id: &str,
    registry: &SandboxAdapterRegistry,
) -> Result<AdapterCapabilities, String> {
    let adapter_id = AdapterId::new(adapter_id.trim());
    registry
        .get(&adapter_id)
        .map(|adapter| adapter.capabilities())
        .ok_or_else(|| format!("sandbox adapter not registered: {adapter_id}"))
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use handshake_core::sandbox::{
        AdapterId, GpuPassthrough, SandboxAdapterRegistry, Wsl2PodmanAdapter, Wsl2PodmanConfig,
        WSL2_PODMAN_ADAPTER_ID,
    };

    use super::{adapter_capabilities, list_adapters};

    #[test]
    fn list_adapters_returns_owned_capabilities() {
        let registry = registry();

        let adapters = list_adapters(&registry);

        assert_eq!(adapters.len(), 1);
        assert_eq!(adapters[0].adapter_id.as_str(), WSL2_PODMAN_ADAPTER_ID);
    }

    #[test]
    fn capabilities_returns_one_adapter_or_typed_string_error() {
        let registry = registry();

        let capabilities =
            adapter_capabilities(WSL2_PODMAN_ADAPTER_ID, &registry).expect("known adapter");
        assert_eq!(capabilities.adapter_id.as_str(), WSL2_PODMAN_ADAPTER_ID);

        let error = adapter_capabilities("missing_adapter", &registry).expect_err("missing");
        assert_eq!(error, "sandbox adapter not registered: missing_adapter");
    }

    fn registry() -> SandboxAdapterRegistry {
        let mut registry = SandboxAdapterRegistry::new(AdapterId::new(WSL2_PODMAN_ADAPTER_ID));
        registry.register(Arc::new(Wsl2PodmanAdapter::with_config_and_gpu_for_tests(
            Wsl2PodmanConfig::new("test-distro", "wsl"),
            GpuPassthrough::None,
        )));
        registry
    }
}
