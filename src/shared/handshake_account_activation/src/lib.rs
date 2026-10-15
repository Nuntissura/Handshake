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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ActivationOriginalEnvelope {
    pub operation_id: uuid::Uuid,
    pub owner_account_id: String,
    pub actor_principal_id: String,
    pub authenticated_session_id: String,
    pub access_space_id: String,
    pub workspace_id: String,
    pub resource_id: String,
    pub grant_id: String,
    pub grant_version: i64,
    pub policy_version: i64,
    pub policy_versions: [i64; 6],
    pub revocation_epochs: [i64; 6],
    pub actor_kind: String,
    pub actor_id: String,
    pub actions: Vec<String>,
    pub capability_ids: Vec<String>,
    pub delegation_chain: Vec<String>,
    pub session_capabilities: Vec<String>,
    pub principal_capabilities: Vec<String>,
    pub capability_profile_id: String,
    pub statuses: [String; 5],
    pub resource_kind: String,
    pub capability_id: String,
    pub action: String,
    pub purpose: String,
    pub grant_expires_at: Option<String>,
    pub session_expires_at: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ActivationPublicationWitness<R> {
    pub operation_id: uuid::Uuid,
    pub original_envelope: ActivationOriginalEnvelope,
    pub actual_runtime: R,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case", deny_unknown_fields)]
pub enum WorkspaceActivationReadiness<R> {
    Current { descriptor: R },
    Unavailable,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case", deny_unknown_fields)]
pub enum WorkspaceActivationOutcome<R> {
    Applied {
        result: WorkspaceActivationAcknowledgement,
        publication_witness: ActivationPublicationWitness<R>,
    },
    Unresolved,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkspaceActivationObservation<R> {
    pub schema_id: String,
    pub operation_id: uuid::Uuid,
    pub owner_account_id: String,
    pub actor_principal_id: String,
    pub authenticated_session_id: String,
    pub access_space_id: String,
    pub workspace_id: String,
    pub readiness: WorkspaceActivationReadiness<R>,
    pub outcome: WorkspaceActivationOutcome<R>,
}

#[derive(Debug)]
pub enum WorkspaceActivationObservationError {
    InvalidSchema,
    InvalidObservation,
    Mismatch,
}

impl<R: Serialize> WorkspaceActivationObservation<R> {
    pub fn to_value(&self) -> Result<serde_json::Value, serde_json::Error> {
        let readiness = match &self.readiness {
            WorkspaceActivationReadiness::Current { descriptor } => {
                serde_json::json!({"status": "current", "descriptor": descriptor})
            }
            WorkspaceActivationReadiness::Unavailable => serde_json::json!({"status": "unavailable"}),
        };
        let outcome = match &self.outcome {
            WorkspaceActivationOutcome::Applied { result, publication_witness } => {
                let result = serialize_workspace_activation_ack(
                    result, "hsk.workspace_activation@1", serde_json::value::Serializer,
                )?;
                serde_json::json!({
                    "status": "applied", "result": result,
                    "publication_witness": {
                        "operation_id": publication_witness.operation_id,
                        "original_envelope": publication_witness.original_envelope,
                        "actual_runtime": publication_witness.actual_runtime,
                    },
                })
            }
            WorkspaceActivationOutcome::Unresolved => serde_json::json!({"status": "unresolved"}),
        };
        Ok(serde_json::json!({
            "schema_id": self.schema_id,
            "operation_id": self.operation_id,
            "owner_account_id": self.owner_account_id,
            "actor_principal_id": self.actor_principal_id,
            "authenticated_session_id": self.authenticated_session_id,
            "access_space_id": self.access_space_id,
            "workspace_id": self.workspace_id,
            "readiness": readiness, "outcome": outcome,
        }))
    }
}

pub fn validate_activation_runtime_descriptor(value: &serde_json::Value) -> bool {
    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Descriptor {
        runtime_instance_id: uuid::Uuid,
        runtime_host_scope_id: String,
        runtime_lease_protocol: String,
        runtime_lease_address: std::net::IpAddr,
        runtime_lease_port: u16,
    }
    let Ok(descriptor) = serde_json::from_value::<Descriptor>(value.clone()) else {
        return false;
    };
    !descriptor.runtime_instance_id.is_nil()
        && !descriptor.runtime_host_scope_id.trim().is_empty()
        && descriptor.runtime_lease_protocol == "hsk.embedded_runtime.loopback_udp_exclusive@1"
        && descriptor.runtime_lease_address.is_loopback()
        && descriptor.runtime_lease_port != 0
}

pub fn parse_workspace_activation_observation<R: serde::de::DeserializeOwned>(
    value: serde_json::Value,
    expected: &WorkspaceActivationExpected<'_>,
) -> Result<WorkspaceActivationObservation<R>, WorkspaceActivationObservationError> {
    use WorkspaceActivationObservationError::{InvalidObservation, InvalidSchema, Mismatch};
    if value["schema_id"].as_str() != Some("hsk.workspace_activation.observation@1") {
        return Err(InvalidSchema);
    }
    let observation: WorkspaceActivationObservation<R> =
        serde_json::from_value(value.clone()).map_err(|_| InvalidObservation)?;
    if observation.operation_id != expected.operation_id
        || observation.owner_account_id != expected.owner_account_id
        || observation.actor_principal_id != expected.actor_principal_id
        || observation.authenticated_session_id != expected.authenticated_session_id
        || observation.access_space_id != expected.access_space_id
        || observation.workspace_id != expected.workspace_id
    {
        return Err(Mismatch);
    }
    if matches!(&observation.readiness, WorkspaceActivationReadiness::Current { .. })
        && !validate_activation_runtime_descriptor(&value["readiness"]["descriptor"])
    {
        return Err(InvalidObservation);
    }
    if let WorkspaceActivationOutcome::Applied { publication_witness, .. } = &observation.outcome {
        parse_workspace_activation_ack(value["outcome"]["result"].clone(), expected)
            .map_err(|error| match error {
                WorkspaceActivationAcknowledgementError::Mismatch => Mismatch,
                _ => InvalidObservation,
            })?;
        let envelope = &publication_witness.original_envelope;
        if publication_witness.operation_id != expected.operation_id
            || envelope.operation_id != expected.operation_id
            || envelope.owner_account_id != expected.owner_account_id
            || envelope.actor_principal_id != expected.actor_principal_id
            || envelope.authenticated_session_id != expected.authenticated_session_id
            || envelope.access_space_id != expected.access_space_id
            || envelope.workspace_id != expected.workspace_id
        {
            return Err(Mismatch);
        }
        if !validate_activation_runtime_descriptor(&value["outcome"]["publication_witness"]["actual_runtime"]) {
            return Err(InvalidObservation);
        }
    }
    Ok(observation)
}
