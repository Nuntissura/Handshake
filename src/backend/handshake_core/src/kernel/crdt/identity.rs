use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CrdtAuthorityLinksV1 {
    pub work_item_id: String,
    pub action_trace_id: String,
    pub artifact_proposal_id: String,
    pub role_mailbox_thread_id: String,
    pub dcc_projection_id: String,
    pub event_ledger_stream_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CrdtWorkspaceIdentityV1 {
    pub schema_id: String,
    pub workspace_id: String,
    pub document_id: String,
    pub crdt_document_id: String,
    pub actor_id: String,
    pub actor_kind: String,
    pub crdt_site_id: String,
    pub crdt_client_id: String,
    pub document_schema_id: String,
    pub authority_links: CrdtAuthorityLinksV1,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CrdtWorkspaceIdentityValidationError {
    pub field: &'static str,
    pub message: &'static str,
}

pub fn validate_crdt_workspace_identity(
    identity: &CrdtWorkspaceIdentityV1,
) -> Result<(), Vec<CrdtWorkspaceIdentityValidationError>> {
    let mut errors = Vec::new();

    require_non_empty(&mut errors, "schema_id", &identity.schema_id);
    require_non_empty(&mut errors, "workspace_id", &identity.workspace_id);
    require_non_empty(&mut errors, "document_id", &identity.document_id);
    require_non_empty(&mut errors, "crdt_document_id", &identity.crdt_document_id);
    require_non_empty(&mut errors, "actor_id", &identity.actor_id);
    require_non_empty(&mut errors, "actor_kind", &identity.actor_kind);
    require_non_empty(&mut errors, "crdt_site_id", &identity.crdt_site_id);
    require_non_empty(&mut errors, "crdt_client_id", &identity.crdt_client_id);
    require_non_empty(
        &mut errors,
        "document_schema_id",
        &identity.document_schema_id,
    );
    require_non_empty(
        &mut errors,
        "authority_links.work_item_id",
        &identity.authority_links.work_item_id,
    );
    require_non_empty(
        &mut errors,
        "authority_links.action_trace_id",
        &identity.authority_links.action_trace_id,
    );
    require_non_empty(
        &mut errors,
        "authority_links.artifact_proposal_id",
        &identity.authority_links.artifact_proposal_id,
    );
    require_non_empty(
        &mut errors,
        "authority_links.role_mailbox_thread_id",
        &identity.authority_links.role_mailbox_thread_id,
    );
    require_non_empty(
        &mut errors,
        "authority_links.dcc_projection_id",
        &identity.authority_links.dcc_projection_id,
    );
    require_non_empty(
        &mut errors,
        "authority_links.event_ledger_stream_id",
        &identity.authority_links.event_ledger_stream_id,
    );

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

fn require_non_empty(
    errors: &mut Vec<CrdtWorkspaceIdentityValidationError>,
    field: &'static str,
    value: &str,
) {
    if value.trim().is_empty() {
        errors.push(CrdtWorkspaceIdentityValidationError {
            field,
            message: "value must not be empty",
        });
    }
}
