use std::sync::Arc;

use tracing::warn;

use crate::process_ledger::LedgerBatcher;

use super::{
    AdapterId, CloudHypervisorAdapter, CloudHypervisorConfig, DockerAdapter, DockerConfig,
    GvisorAdapter, GvisorConfig, LedgerDecorator, SandboxAdapter, SandboxAdapterError,
    SandboxAdapterRegistry, SandboxSettings, Wsl2PodmanAdapter, Wsl2PodmanConfig,
    CLOUD_HYPERVISOR_ADAPTER_ID, DOCKER_ADAPTER_ID, GVISOR_ADAPTER_ID,
    WINDOWS_NATIVE_JAIL_ADAPTER_ID, WSL2_PODMAN_ADAPTER_ID,
};

pub const WINDOWS_NATIVE_JAIL_MT045_DECISION_RECORD: &str =
    ".GOV/roles_shared/records/sandbox/WP-KERNEL-004-windows-native-jail-crate-license-audit.json";

pub fn build_default_registry() -> Result<SandboxAdapterRegistry, SandboxAdapterError> {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|error| SandboxAdapterError::AdapterUnavailable {
            adapter_id: AdapterId::new(WSL2_PODMAN_ADAPTER_ID),
            reason: format!("sandbox bootstrap runtime unavailable: {error}"),
        })?;
    runtime.block_on(build_default_registry_async())
}

pub async fn build_default_registry_async() -> Result<SandboxAdapterRegistry, SandboxAdapterError> {
    let mut adapters: Vec<Arc<dyn SandboxAdapter>> = Vec::new();

    match Wsl2PodmanAdapter::try_new(Wsl2PodmanConfig::default()).await {
        Ok(adapter) => adapters.push(Arc::new(adapter)),
        Err(error) => warn!(
            adapter_id = WSL2_PODMAN_ADAPTER_ID,
            error = %error,
            "skipping unavailable sandbox adapter during bootstrap"
        ),
    }

    match super::WindowsNativeJailAdapter::try_new().await {
        Ok(adapter) => adapters.push(Arc::new(adapter)),
        Err(error) => warn!(
            adapter_id = WINDOWS_NATIVE_JAIL_ADAPTER_ID,
            decision_record = WINDOWS_NATIVE_JAIL_MT045_DECISION_RECORD,
            error = %error,
            "skipping unavailable sandbox adapter during bootstrap"
        ),
    }

    match DockerAdapter::try_new(DockerConfig::default()).await {
        Ok(adapter) => adapters.push(Arc::new(adapter)),
        Err(error) => warn!(
            adapter_id = DOCKER_ADAPTER_ID,
            error = %error,
            "skipping unavailable sandbox adapter during bootstrap"
        ),
    }

    // Tier-2 syscall-isolation (gVisor / runsc) sandbox. Available only on WSL2
    // hosts where runsc can actually start a sandbox; try_new performs a real
    // availability probe (binary present + a live smoke sandbox) and returns
    // AdapterUnavailable elsewhere so this block silently skips. It is never the
    // implicit default adapter; selection reaches it only via a Tier-2
    // isolation requirement.
    match GvisorAdapter::try_new(GvisorConfig::default()).await {
        Ok(adapter) => adapters.push(Arc::new(adapter)),
        Err(error) => warn!(
            adapter_id = GVISOR_ADAPTER_ID,
            error = %error,
            "skipping unavailable sandbox adapter during bootstrap"
        ),
    }

    // Tier-3 hardware-virtualized microVM. Available only on WSL2 + KVM hosts;
    // try_new performs a real availability probe and returns AdapterUnavailable
    // elsewhere so this block silently skips. It is never the default adapter;
    // selection reaches it only via a Tier-3 isolation requirement.
    match CloudHypervisorAdapter::try_new(CloudHypervisorConfig::default()).await {
        Ok(adapter) => adapters.push(Arc::new(adapter)),
        Err(error) => warn!(
            adapter_id = CLOUD_HYPERVISOR_ADAPTER_ID,
            error = %error,
            "skipping unavailable sandbox adapter during bootstrap"
        ),
    }

    let settings = SandboxSettings {
        docker_explicit_opt_in: docker_explicit_opt_in_from_env(),
        ..SandboxSettings::default()
    };

    build_registry_from_adapters(
        settings.default_adapter.adapter_id(),
        adapters,
        settings.docker_explicit_opt_in,
    )
}

pub fn build_registry_from_adapters(
    preferred_default_adapter_id: AdapterId,
    adapters: Vec<Arc<dyn SandboxAdapter>>,
    docker_explicit_opt_in: bool,
) -> Result<SandboxAdapterRegistry, SandboxAdapterError> {
    build_registry_from_adapters_with_ledger(
        preferred_default_adapter_id,
        adapters,
        docker_explicit_opt_in,
        None,
    )
}

pub fn build_registry_from_adapters_with_ledger(
    preferred_default_adapter_id: AdapterId,
    adapters: Vec<Arc<dyn SandboxAdapter>>,
    docker_explicit_opt_in: bool,
    ledger_batcher: Option<LedgerBatcher>,
) -> Result<SandboxAdapterRegistry, SandboxAdapterError> {
    if let Some(reason) = disallowed_default_adapter_reason(&preferred_default_adapter_id) {
        return Err(SandboxAdapterError::AdapterUnavailable {
            adapter_id: preferred_default_adapter_id,
            reason,
        });
    }

    let available_adapter_ids = adapters
        .iter()
        .map(|adapter| adapter.capabilities().adapter_id)
        .collect::<Vec<_>>();

    if available_adapter_ids.is_empty() {
        return Err(SandboxAdapterError::AdapterUnavailable {
            adapter_id: preferred_default_adapter_id,
            reason: "no sandbox adapters available during bootstrap".to_string(),
        });
    }

    let default_adapter_id = if available_adapter_ids
        .iter()
        .any(|adapter_id| adapter_id == &preferred_default_adapter_id)
    {
        preferred_default_adapter_id
    } else {
        let fallback = available_adapter_ids
            .iter()
            .find(|adapter_id| implicit_default_fallback_allowed(adapter_id))
            .ok_or_else(|| SandboxAdapterError::AdapterUnavailable {
                adapter_id: preferred_default_adapter_id.clone(),
                reason: format!(
                    "no implicit default sandbox adapter available during bootstrap; {DOCKER_ADAPTER_ID} remains compat-only and {WINDOWS_NATIVE_JAIL_ADAPTER_ID} requires explicit Win32-native selection"
                ),
            })?
            .clone();
        warn!(
            adapter_id = %fallback,
            "preferred sandbox adapter unavailable; using first available fallback as registry default"
        );
        fallback
    };

    let mut registry = SandboxAdapterRegistry::new(default_adapter_id);
    registry.set_docker_explicit_opt_in(docker_explicit_opt_in);
    for adapter in adapters {
        let adapter: Arc<dyn SandboxAdapter> = match &ledger_batcher {
            Some(ledger_batcher) => Arc::new(LedgerDecorator::new(adapter, ledger_batcher.clone())),
            None => adapter,
        };
        registry.register(adapter);
    }
    Ok(registry)
}

fn implicit_default_fallback_allowed(adapter_id: &AdapterId) -> bool {
    // cloud_hypervisor (Tier-3 microVM) and gvisor (Tier-2 syscall isolation)
    // are reached only via an explicit isolation-tier requirement; neither must
    // ever be silently chosen as the everyday implicit default, alongside docker
    // (compat-only) and the Win32-native jail (requires explicit selection).
    !matches!(
        adapter_id.as_str(),
        DOCKER_ADAPTER_ID
            | WINDOWS_NATIVE_JAIL_ADAPTER_ID
            | CLOUD_HYPERVISOR_ADAPTER_ID
            | GVISOR_ADAPTER_ID
    )
}

fn disallowed_default_adapter_reason(adapter_id: &AdapterId) -> Option<String> {
    match adapter_id.as_str() {
        DOCKER_ADAPTER_ID => Some(
            "docker cannot be the default sandbox adapter; select docker only through an explicit override plus opt-in"
                .to_string(),
        ),
        WINDOWS_NATIVE_JAIL_ADAPTER_ID => Some(format!(
            "{WINDOWS_NATIVE_JAIL_ADAPTER_ID} cannot be the default sandbox adapter; select it only through a Win32-native requirement or explicit work-profile override after MT-045 approval"
        )),
        _ => None,
    }
}

fn docker_explicit_opt_in_from_env() -> bool {
    std::env::var("HANDSHAKE_SANDBOX_DOCKER_EXPLICIT_OPT_IN")
        .map(|value| {
            matches!(
                value.trim().to_ascii_lowercase().as_str(),
                "1" | "true" | "yes" | "y" | "on"
            )
        })
        .unwrap_or(false)
}
