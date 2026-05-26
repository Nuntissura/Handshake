use std::{collections::BTreeMap, sync::Arc};

use super::{
    windows_native_jail_unavailable_capabilities, AdapterCapabilities, AdapterId, SandboxAdapter,
    WindowsNativeJailAdapter, WINDOWS_NATIVE_JAIL_ADAPTER_ID, WINDOWS_NATIVE_JAIL_BACKEND_APPROVED,
};

#[derive(Clone)]
pub struct SandboxAdapterRegistry {
    adapters: BTreeMap<AdapterId, Arc<dyn SandboxAdapter>>,
    default_adapter_id: AdapterId,
    docker_explicit_opt_in: bool,
}

impl SandboxAdapterRegistry {
    pub fn new(default_adapter_id: AdapterId) -> Self {
        Self {
            adapters: BTreeMap::new(),
            default_adapter_id,
            docker_explicit_opt_in: false,
        }
    }

    pub fn register(&mut self, adapter: Arc<dyn SandboxAdapter>) {
        let adapter_id = adapter.capabilities().adapter_id;
        if self.adapters.contains_key(&adapter_id) {
            panic!("duplicate sandbox adapter registration: {adapter_id}");
        }
        self.adapters.insert(adapter_id, adapter);
    }

    pub fn set_docker_explicit_opt_in(&mut self, opt_in: bool) {
        self.docker_explicit_opt_in = opt_in;
    }

    pub fn docker_explicit_opt_in(&self) -> bool {
        self.docker_explicit_opt_in
    }

    pub fn list(&self) -> Vec<AdapterCapabilities> {
        self.adapters
            .values()
            .map(|adapter| registry_visible_capabilities(adapter.capabilities()))
            .collect()
    }

    pub fn get(&self, adapter_id: &AdapterId) -> Option<Arc<dyn SandboxAdapter>> {
        if adapter_id.as_str() == WINDOWS_NATIVE_JAIL_ADAPTER_ID
            && !WINDOWS_NATIVE_JAIL_BACKEND_APPROVED
        {
            return self
                .adapters
                .contains_key(adapter_id)
                .then(|| Arc::new(WindowsNativeJailAdapter::unavailable_for_current_host()) as _);
        }
        self.adapters.get(adapter_id).cloned()
    }

    pub fn default(&self) -> Arc<dyn SandboxAdapter> {
        self.get(&self.default_adapter_id).unwrap_or_else(|| {
            panic!(
                "default sandbox adapter not registered: {}",
                self.default_adapter_id
            )
        })
    }

    pub fn default_adapter_id(&self) -> &AdapterId {
        &self.default_adapter_id
    }
}

fn registry_visible_capabilities(capabilities: AdapterCapabilities) -> AdapterCapabilities {
    if capabilities.adapter_id.as_str() == WINDOWS_NATIVE_JAIL_ADAPTER_ID
        && !WINDOWS_NATIVE_JAIL_BACKEND_APPROVED
    {
        windows_native_jail_unavailable_capabilities()
    } else {
        capabilities
    }
}
