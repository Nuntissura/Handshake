use std::sync::Arc;

use axum::{http::StatusCode, Json, Router};
use serde_json::Value;
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::{
    process_ledger::{EmbeddedRuntimeInstanceDescriptor, ProcessReclaimRuntime},
    storage::surreal::resource_authority::ActivationObservationError,
    swarm_orchestration::resource_scope::ExactResourceScopeAttribution,
    AccountBootstrapState,
};

pub(crate) type ApiError = (StatusCode, Json<Value>);
pub type MountedApiState = Arc<RwLock<Option<MountedApi>>>;

pub struct MountedApi {
    router: Router,
    runtime: ProcessReclaimRuntime,
    bootstrap: AccountBootstrapState,
    exact: ExactResourceScopeAttribution,
}

impl MountedApi {
    pub fn new(
        router: Router,
        runtime: ProcessReclaimRuntime,
        bootstrap: AccountBootstrapState,
        exact: ExactResourceScopeAttribution,
    ) -> Self {
        Self { router, runtime, bootstrap, exact }
    }

    pub fn router(&self) -> Router {
        self.router.clone()
    }
}

pub(crate) async fn observe_workspace_activation(
    mounted: &MountedApiState,
    bootstrap: &AccountBootstrapState,
    token: &str,
    channel_hash: &str,
    workspace_id: &str,
    operation_id: Uuid,
) -> Result<Json<Value>, ApiError> {
    let mount = mounted.read().await;
    if let Some(current) = mount.as_ref() {
        if !Arc::ptr_eq(&current.bootstrap.storage, &bootstrap.storage)
            || !Arc::ptr_eq(
                &current.bootstrap.capability_registry,
                &bootstrap.capability_registry,
            )
            || current.bootstrap.surreal.resource_access_lifecycle()
                != bootstrap.surreal.resource_access_lifecycle()
        {
            return Err(super::authority::activation_observation_error_response(
                ActivationObservationError::Unavailable,
            ));
        }
    }

    bootstrap
        .surreal
        .read_activation_and_publish(
            token,
            channel_hash,
            operation_id,
            workspace_id,
            &bootstrap.capability_registry,
            |observation| {
                let readiness = mount
                    .as_ref()
                    .filter(|current| &current.exact == observation.exact())
                    .map(|current| current.runtime.runtime_instance());
                observation.encode::<EmbeddedRuntimeInstanceDescriptor>(readiness)
            },
        )
        .await
        .map(Json)
        .map_err(super::authority::activation_observation_error_response)
}
