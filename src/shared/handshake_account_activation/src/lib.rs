use serde::{Deserialize, Serialize, Serializer};

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkspaceActivationInput {
    pub workspace_id: String,
    pub operation_id: uuid::Uuid,
}

#[derive(Debug, Clone, Deserialize)]
pub struct WorkspaceActivationAcknowledgement {
    pub operation_id: uuid::Uuid,
    pub owner_account_id: String,
    pub actor_principal_id: String,
    pub authenticated_session_id: String,
    pub access_space_id: String,
    pub workspace_id: String,
}

pub struct WorkspaceActivationExpected<'a> {
    pub operation_id: uuid::Uuid,
    pub owner_account_id: &'a str,
    pub actor_principal_id: &'a str,
    pub authenticated_session_id: &'a str,
    pub access_space_id: &'a str,
    pub workspace_id: &'a str,
}

#[derive(Debug)]
pub enum WorkspaceActivationAcknowledgementError {
    InvalidSchema,
    InvalidAcknowledgement,
    Mismatch,
}

pub fn parse_workspace_activation_ack(
    value: serde_json::Value,
    expected: &WorkspaceActivationExpected<'_>,
) -> Result<WorkspaceActivationAcknowledgement, WorkspaceActivationAcknowledgementError> {
    if value["schema_version"].as_str() != Some("hsk.workspace_activation@1") {
        return Err(WorkspaceActivationAcknowledgementError::InvalidSchema);
    }
    let acknowledgement: WorkspaceActivationAcknowledgement = serde_json::from_value(value)
        .map_err(|_| WorkspaceActivationAcknowledgementError::InvalidAcknowledgement)?;
    if acknowledgement.operation_id != expected.operation_id
        || acknowledgement.owner_account_id != expected.owner_account_id
        || acknowledgement.actor_principal_id != expected.actor_principal_id
        || acknowledgement.authenticated_session_id != expected.authenticated_session_id
        || acknowledgement.access_space_id != expected.access_space_id
        || acknowledgement.workspace_id != expected.workspace_id
    {
        return Err(WorkspaceActivationAcknowledgementError::Mismatch);
    }
    Ok(acknowledgement)
}

pub fn serialize_workspace_activation_ack<S: Serializer>(
    acknowledgement: &WorkspaceActivationAcknowledgement,
    schema_version: &str,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    #[derive(Serialize)]
    struct Envelope<'a> {
        schema_version: &'a str,
        operation_id: uuid::Uuid,
        owner_account_id: &'a str,
        actor_principal_id: &'a str,
        authenticated_session_id: &'a str,
        access_space_id: &'a str,
        workspace_id: &'a str,
    }

    Envelope {
        schema_version,
        operation_id: acknowledgement.operation_id,
        owner_account_id: &acknowledgement.owner_account_id,
        actor_principal_id: &acknowledgement.actor_principal_id,
        authenticated_session_id: &acknowledgement.authenticated_session_id,
        access_space_id: &acknowledgement.access_space_id,
        workspace_id: &acknowledgement.workspace_id,
    }
    .serialize(serializer)
}
