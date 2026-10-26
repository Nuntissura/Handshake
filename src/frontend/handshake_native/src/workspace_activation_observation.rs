use std::sync::Arc;

use handshake_account_activation::{WorkspaceActivationExpected, WorkspaceActivationObservation};
use handshake_runtime_instance::EmbeddedRuntimeInstanceDescriptor;

use crate::local_account::AuthenticatedContext;

pub(crate) struct MountedReadinessCapture {
    account: Arc<AuthenticatedContext>,
    workspace_id: String,
    operation_id: uuid::Uuid,
    generation: u64,
}

impl MountedReadinessCapture {
    pub(crate) fn new(
        account: Arc<AuthenticatedContext>,
        workspace_id: String,
        operation_id: uuid::Uuid,
        generation: u64,
    ) -> Result<Self, WorkspaceActivationObservationError> {
        if !account.is_active() || workspace_id.trim().is_empty() || operation_id.is_nil() {
            return Err(WorkspaceActivationObservationError::ContextChanged);
        }
        Ok(Self {
            account,
            workspace_id,
            operation_id,
            generation,
        })
    }

    pub(crate) fn is_current(
        &self,
        account: &Arc<AuthenticatedContext>,
        workspace_id: &str,
        operation_id: uuid::Uuid,
        generation: u64,
    ) -> bool {
        self.account.is_active()
            && account.is_active()
            && Arc::ptr_eq(&self.account, account)
            && self.account.account_id == account.account_id
            && self.account.principal_id == account.principal_id
            && self.account.session_id == account.session_id
            && self.account.access_space_id == account.access_space_id
            && self.workspace_id == workspace_id
            && self.operation_id == operation_id
            && self.generation == generation
    }

    pub(crate) async fn fetch(
        &self,
        client: &reqwest::Client,
        backend: &str,
    ) -> Result<
        WorkspaceActivationObservation<EmbeddedRuntimeInstanceDescriptor>,
        WorkspaceActivationObservationError,
    > {
        fetch_workspace_activation_observation(
            client,
            backend,
            self.account.clone(),
            &self.workspace_id,
            self.operation_id,
        )
        .await
    }
}

/// Read-only host binding for the retained real activation. No credential is serialized.
pub(crate) struct WorkspaceActivationReadback {
    capture: MountedReadinessCapture,
    backend: String,
    in_flight: tokio::sync::Mutex<()>,
}

impl WorkspaceActivationReadback {
    pub(crate) fn new(
        account: Arc<AuthenticatedContext>,
        workspace_id: String,
        operation_id: uuid::Uuid,
        generation: u64,
        backend: String,
    ) -> Result<Self, WorkspaceActivationObservationError> {
        Ok(Self {
            capture: MountedReadinessCapture::new(account, workspace_id, operation_id, generation)?,
            backend,
            in_flight: tokio::sync::Mutex::new(()),
        })
    }

    pub(crate) fn matches_owner(&self, account: &Arc<AuthenticatedContext>, workspace_id: &str) -> bool {
        self.capture.is_current(account, workspace_id, self.capture.operation_id, self.capture.generation)
    }

    pub(crate) fn matches_original(
        &self, account: &Arc<AuthenticatedContext>, workspace_id: &str,
        operation_id: uuid::Uuid, generation: u64, backend: &str,
    ) -> bool {
        self.capture.is_current(account, workspace_id, operation_id, generation) && self.backend == backend
    }

    pub(crate) async fn fetch(&self) -> serde_json::Value {
        let Ok(_in_flight) = self.in_flight.try_lock() else {
            return serde_json::json!({"status":"busy"});
        };
        let result = tokio::time::timeout(std::time::Duration::from_secs(5), self.capture.fetch(
            &crate::backend_client::shared_http_client(), &self.backend,
        )).await;
        let observation = match result {
            Err(_) => return serde_json::json!({"status":"error","reason":"timeout"}),
            Ok(Err(error)) => {
                let (reason, status) = match error {
                    WorkspaceActivationObservationError::ContextChanged => ("context_changed", None),
                    WorkspaceActivationObservationError::Request(_) => ("request_unavailable", None),
                    WorkspaceActivationObservationError::Transport(_) => ("transport", None),
                    WorkspaceActivationObservationError::Status(status) => ("http_status", Some(status.as_u16())),
                    WorkspaceActivationObservationError::InvalidObservation => ("invalid_observation", None),
                };
                return serde_json::json!({"status":"error","reason":reason,"http_status":status});
            }
            Ok(Ok(observation)) => observation,
        };
        let Ok(observation) = observation.to_value() else {
            return serde_json::json!({"status":"error","reason":"encoding"});
        };
        let account = &self.capture.account;
        serde_json::json!({
            "status":"observed", "source":"fresh_canonical_get", "cached":false,
            "host_generation":self.capture.generation,
            "expected":{
                "operation_id":self.capture.operation_id,
                "owner_account_id":account.account_id,
                "actor_principal_id":account.principal_id,
                "authenticated_session_id":account.session_id,
                "access_space_id":account.access_space_id,
                "workspace_id":self.capture.workspace_id,
            },
            "observation":observation,
        })
    }
}

pub(crate) async fn fetch_workspace_activation_observation(
    _client: &reqwest::Client,
    backend: &str,
    context: Arc<AuthenticatedContext>,
    workspace_id: &str,
    operation_id: uuid::Uuid,
) -> Result<
    WorkspaceActivationObservation<EmbeddedRuntimeInstanceDescriptor>,
    WorkspaceActivationObservationError,
> {
    if !context.is_active() || workspace_id.trim().is_empty() || operation_id.is_nil() {
        return Err(WorkspaceActivationObservationError::ContextChanged);
    }
    let client = crate::backend_client::shared_http_client();
    let mut url = reqwest::Url::parse(backend)
        .map_err(|error| WorkspaceActivationObservationError::Request(error.to_string()))?;
    url.set_query(None);
    url.set_fragment(None);
    let operation = operation_id.to_string();
    url.path_segments_mut()
        .map_err(|_| {
            WorkspaceActivationObservationError::Request("Invalid backend URL".to_owned())
        })?
        .pop_if_empty()
        .extend(["authority", "workspaces", workspace_id, "activation", &operation]);
    let request = context
        .authorize(client.get(url))
        .map_err(WorkspaceActivationObservationError::Request)?;
    let response = client
        .execute(request)
        .await
        .map_err(|error| WorkspaceActivationObservationError::Transport(error.to_string()))?;
    let status = response.status();
    context.observe_status(status);
    if !context.is_active() {
        return Err(WorkspaceActivationObservationError::ContextChanged);
    }
    if !status.is_success() {
        return Err(WorkspaceActivationObservationError::Status(status));
    }
    let value = response
        .json()
        .await
        .map_err(|_| WorkspaceActivationObservationError::InvalidObservation)?;
    decode_workspace_activation_observation(value, &context, workspace_id, operation_id)
}

pub(crate) fn decode_workspace_activation_observation(
    value: serde_json::Value,
    context: &AuthenticatedContext,
    workspace_id: &str,
    operation_id: uuid::Uuid,
) -> Result<
    WorkspaceActivationObservation<EmbeddedRuntimeInstanceDescriptor>,
    WorkspaceActivationObservationError,
> {
    if !context.is_active() || workspace_id.trim().is_empty() || operation_id.is_nil() {
        return Err(WorkspaceActivationObservationError::ContextChanged);
    }
    let observation = handshake_account_activation::parse_workspace_activation_observation(
        value,
        &WorkspaceActivationExpected {
            operation_id,
            owner_account_id: &context.account_id,
            actor_principal_id: &context.principal_id,
            authenticated_session_id: &context.session_id,
            access_space_id: &context.access_space_id,
            workspace_id,
        },
    )
    .map_err(|_| WorkspaceActivationObservationError::InvalidObservation)?;
    if !context.is_active() {
        return Err(WorkspaceActivationObservationError::ContextChanged);
    }
    Ok(observation)
}

#[derive(Debug)]
pub(crate) enum WorkspaceActivationObservationError {
    ContextChanged,
    Request(String),
    Transport(String),
    Status(reqwest::StatusCode),
    InvalidObservation,
}

impl std::fmt::Display for WorkspaceActivationObservationError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ContextChanged => formatter.write_str("Workspace activation context changed"),
            Self::Request(message) | Self::Transport(message) => formatter.write_str(message),
            Self::Status(status) => write!(formatter, "Workspace activation observation unavailable ({status})"),
            Self::InvalidObservation => formatter.write_str("Invalid workspace activation observation"),
        }
    }
}
