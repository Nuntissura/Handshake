use handshake_account_activation::{
    ActivationOriginalEnvelope, WorkspaceActivationAcknowledgement, WorkspaceActivationExpected,
};
use serde::Serialize;
use surrealdb::types::{RecordId, SurrealValue};
use uuid::Uuid;

use super::{resource_authority::ResourceAuthorityError, SurrealTransactionContext};

#[derive(SurrealValue)]
struct OutcomeRow {
    envelope_json: String,
    state: String,
    result_json: Option<String>,
    witness_json: Option<String>,
}

#[derive(SurrealValue)]
struct Lookup {
    record: RecordId,
}

#[derive(SurrealValue)]
struct PreparationBindings {
    record: RecordId,
    envelope_json: String,
}

#[derive(SurrealValue)]
struct FinalizationBindings {
    record: RecordId,
    envelope_json: String,
    result_json: String,
    witness_json: String,
}

pub(crate) struct StoredActivationOutcome {
    envelope: ActivationOriginalEnvelope,
    applied: Option<CanonicalAppliedActivation>,
}

impl StoredActivationOutcome {
    pub(crate) fn envelope(&self) -> &ActivationOriginalEnvelope {
        &self.envelope
    }

    pub(crate) fn applied(&self) -> Option<&CanonicalAppliedActivation> {
        self.applied.as_ref()
    }
}

pub(crate) struct CanonicalAppliedActivation {
    envelope: ActivationOriginalEnvelope,
    result: serde_json::Value,
    publication_witness: serde_json::Value,
}

impl CanonicalAppliedActivation {
    pub(crate) fn envelope(&self) -> &ActivationOriginalEnvelope {
        &self.envelope
    }

    pub(crate) fn result(&self) -> &serde_json::Value {
        &self.result
    }

    pub(crate) fn publication_witness(&self) -> &serde_json::Value {
        &self.publication_witness
    }
}

pub(crate) enum ActivationPreparation {
    NewlyPrepared(NewlyPreparedActivation),
    Pending,
    Applied(CanonicalAppliedActivation),
}

pub(crate) struct NewlyPreparedActivation {
    envelope: ActivationOriginalEnvelope,
}

pub(crate) struct ProtectedActivationPublicationWitness {
    applied: CanonicalAppliedActivation,
}

impl NewlyPreparedActivation {
    /// The controller must observe the preparation transaction commit first and
    /// hold its original authority/lifecycle fence through this callback.
    pub(crate) fn publish_after_observed_commit<R, E, F>(
        self,
        runtime: &R,
        publication: F,
    ) -> Result<ProtectedActivationPublicationWitness, E>
    where
        R: Serialize,
        E: From<ResourceAuthorityError>,
        F: FnOnce() -> Result<(), E>,
    {
        let envelope = self.envelope;
        let acknowledgement = WorkspaceActivationAcknowledgement {
            operation_id: envelope.operation_id,
            owner_account_id: envelope.owner_account_id.clone(),
            actor_principal_id: envelope.actor_principal_id.clone(),
            authenticated_session_id: envelope.authenticated_session_id.clone(),
            access_space_id: envelope.access_space_id.clone(),
            workspace_id: envelope.workspace_id.clone(),
        };
        let result = handshake_account_activation::serialize_workspace_activation_ack(
            &acknowledgement,
            "hsk.workspace_activation@1",
            serde_json::value::Serializer,
        )
        .map_err(|_| E::from(invalid("activation result encoding failed")))?;
        let actual_runtime = serde_json::to_value(runtime)
            .map_err(|_| E::from(invalid("activation runtime encoding failed")))?;
        if !handshake_account_activation::validate_activation_runtime_descriptor(&actual_runtime) {
            return Err(E::from(invalid("invalid activation publication runtime")));
        }
        let publication_witness = serde_json::json!({
            "operation_id": envelope.operation_id,
            "original_envelope": &envelope,
            "actual_runtime": actual_runtime,
        });
        publication()?;
        Ok(ProtectedActivationPublicationWitness {
            applied: CanonicalAppliedActivation { envelope, result, publication_witness },
        })
    }
}

pub(crate) async fn read_activation(
    transaction: &SurrealTransactionContext<'_>,
    operation_id: Uuid,
) -> Result<Option<StoredActivationOutcome>, ResourceAuthorityError> {
    if operation_id.is_nil() {
        return Err(invalid("nil activation operation"));
    }
    let row = transaction
        .query_first::<OutcomeRow, _>(
            "SELECT envelope_json, state, result_json, witness_json FROM $record;",
            Lookup { record: record(operation_id) },
        )
        .await?;
    row.map(|row| decode_row(row, operation_id)).transpose()
}

/// Called only inside the server controller's original-authority transaction.
/// The returned owner is usable only after the controller observes its commit.
pub(crate) async fn prepare_activation(
    transaction: &SurrealTransactionContext<'_>,
    envelope: &ActivationOriginalEnvelope,
) -> Result<ActivationPreparation, ResourceAuthorityError> {
    if let Some(existing) = read_activation(transaction, envelope.operation_id).await? {
        if existing.envelope != *envelope {
            return Err(invalid("activation operation envelope conflict"));
        }
        return Ok(match existing.applied {
            Some(applied) => ActivationPreparation::Applied(applied),
            None => ActivationPreparation::Pending,
        });
    }
    let envelope_json = encode(envelope)?;
    let row = transaction
        .query_first::<OutcomeRow, _>(
            "CREATE $record SET envelope_json = $envelope_json, state = 'prepared', \
             result_json = NONE, witness_json = NONE, prepared_at = time::now(), \
             applied_at = NONE RETURN AFTER;",
            PreparationBindings { record: record(envelope.operation_id), envelope_json },
        )
        .await?
        .ok_or_else(|| invalid("activation preparation not observed"))?;
    let stored = decode_row(row, envelope.operation_id)?;
    if stored.envelope != *envelope || stored.applied.is_some() {
        return Err(invalid("activation preparation envelope conflict"));
    }
    Ok(ActivationPreparation::NewlyPrepared(NewlyPreparedActivation {
        envelope: envelope.clone(),
    }))
}

/// Wire witnesses cannot construct this permit. The controller must retain its
/// original authority checks through this transaction and observe its commit.
pub(crate) async fn finalize_activation(
    transaction: &SurrealTransactionContext<'_>,
    witness: ProtectedActivationPublicationWitness,
) -> Result<CanonicalAppliedActivation, ResourceAuthorityError> {
    let applied = witness.applied;
    let operation_id = applied.envelope.operation_id;
    let existing = read_activation(transaction, operation_id).await?
        .ok_or_else(|| invalid("activation preparation unavailable"))?;
    if existing.envelope != applied.envelope {
        return Err(invalid("activation operation envelope conflict"));
    }
    if let Some(stored) = existing.applied {
        if stored.result == applied.result && stored.publication_witness == applied.publication_witness {
            return Ok(stored);
        }
        return Err(invalid("activation terminal result conflict"));
    }
    let bindings = FinalizationBindings {
        record: record(operation_id),
        envelope_json: encode(&applied.envelope)?,
        result_json: encode(&applied.result)?,
        witness_json: encode(&applied.publication_witness)?,
    };
    let row = transaction
        .query_first::<OutcomeRow, _>(
            "UPDATE $record SET state = 'applied', result_json = $result_json, \
             witness_json = $witness_json, applied_at = time::now() \
             WHERE state = 'prepared' AND envelope_json = $envelope_json RETURN AFTER;",
            bindings,
        )
        .await?
        .ok_or_else(|| invalid("activation finalization not observed"))?;
    let stored = decode_row(row, operation_id)?.applied
        .ok_or_else(|| invalid("activation finalization remained pending"))?;
    if stored.envelope != applied.envelope
        || stored.result != applied.result
        || stored.publication_witness != applied.publication_witness
    {
        return Err(invalid("activation finalization result conflict"));
    }
    Ok(stored)
}

fn decode_row(row: OutcomeRow, operation_id: Uuid) -> Result<StoredActivationOutcome, ResourceAuthorityError> {
    let envelope: ActivationOriginalEnvelope = serde_json::from_str(&row.envelope_json)
        .map_err(|_| invalid("invalid activation envelope"))?;
    if envelope.operation_id != operation_id {
        return Err(invalid("activation operation envelope conflict"));
    }
    let applied = match row.state.as_str() {
        "prepared" if row.result_json.is_none() && row.witness_json.is_none() => None,
        "applied" => {
            let result: serde_json::Value = serde_json::from_str(row.result_json.as_deref()
                .ok_or_else(|| invalid("activation result absent"))?)
                .map_err(|_| invalid("invalid activation result"))?;
            let publication_witness: serde_json::Value = serde_json::from_str(row.witness_json.as_deref()
                .ok_or_else(|| invalid("activation publication witness absent"))?)
                .map_err(|_| invalid("invalid activation publication witness"))?;
            let expected = WorkspaceActivationExpected {
                operation_id,
                owner_account_id: &envelope.owner_account_id,
                actor_principal_id: &envelope.actor_principal_id,
                authenticated_session_id: &envelope.authenticated_session_id,
                access_space_id: &envelope.access_space_id,
                workspace_id: &envelope.workspace_id,
            };
            handshake_account_activation::parse_workspace_activation_ack(result.clone(), &expected)
                .map_err(|_| invalid("activation result envelope mismatch"))?;
            let witness: handshake_account_activation::ActivationPublicationWitness<serde_json::Value> =
                serde_json::from_value(publication_witness.clone())
                    .map_err(|_| invalid("invalid activation publication witness"))?;
            if witness.operation_id != operation_id || witness.original_envelope != envelope
                || !handshake_account_activation::validate_activation_runtime_descriptor(&witness.actual_runtime)
            {
                return Err(invalid("activation publication envelope mismatch"));
            }
            Some(CanonicalAppliedActivation { envelope: envelope.clone(), result, publication_witness })
        }
        _ => return Err(invalid("invalid activation settlement state")),
    };
    Ok(StoredActivationOutcome { envelope, applied })
}

fn encode<T: Serialize>(value: &T) -> Result<String, ResourceAuthorityError> {
    serde_json::to_string(value).map_err(|_| invalid("activation encoding failed"))
}

fn record(operation_id: Uuid) -> RecordId {
    RecordId::new("workspace_activation_outcome", operation_id.to_string())
}

fn invalid(message: &'static str) -> ResourceAuthorityError {
    ResourceAuthorityError::InvalidInput(message)
}
