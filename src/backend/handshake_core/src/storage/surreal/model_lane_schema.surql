DEFINE TABLE IF NOT EXISTS model_lane_schema_state SCHEMAFULL;
DEFINE FIELD IF NOT EXISTS schema_version ON model_lane_schema_state TYPE string;
DEFINE FIELD IF NOT EXISTS schema_revision ON model_lane_schema_state TYPE int;
DEFINE FIELD IF NOT EXISTS apply_state ON model_lane_schema_state TYPE string;

DEFINE TABLE IF NOT EXISTS model_lane_recovery_order SCHEMAFULL;
DEFINE FIELD IF NOT EXISTS run_id ON model_lane_recovery_order TYPE string;
DEFINE FIELD IF NOT EXISTS next_value ON model_lane_recovery_order TYPE int DEFAULT 0;
DEFINE FIELD OVERWRITE owner_account_id ON model_lane_recovery_order TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE actor_principal_id ON model_lane_recovery_order TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE authenticated_session_id ON model_lane_recovery_order TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE access_space_id ON model_lane_recovery_order TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE workspace_id ON model_lane_recovery_order TYPE string ASSERT string::trim($value) != '';
DEFINE INDEX IF NOT EXISTS model_lane_recovery_order_scope ON model_lane_recovery_order FIELDS run_id, owner_account_id, actor_principal_id, authenticated_session_id, access_space_id, workspace_id UNIQUE;

DEFINE TABLE IF NOT EXISTS model_lane_authority SCHEMAFULL;
DEFINE FIELD IF NOT EXISTS record_kind ON model_lane_authority TYPE string;
DEFINE FIELD IF NOT EXISTS aggregate_id ON model_lane_authority TYPE string;
DEFINE FIELD IF NOT EXISTS run_id ON model_lane_authority TYPE string;
DEFINE FIELD IF NOT EXISTS idempotency_key ON model_lane_authority TYPE string;
DEFINE FIELD IF NOT EXISTS record_json ON model_lane_authority TYPE string;
DEFINE FIELD IF NOT EXISTS search_terms ON model_lane_authority TYPE array<string> DEFAULT [];
DEFINE FIELD IF NOT EXISTS event_id ON model_lane_authority TYPE string;
DEFINE FIELD IF NOT EXISTS event_ledger_event_id ON model_lane_authority TYPE record<kernel_event_ledger> ASSERT record::exists($value) REFERENCE ON DELETE REJECT;
DEFINE FIELD IF NOT EXISTS event_seq ON model_lane_authority TYPE int;
DEFINE FIELD IF NOT EXISTS event_stream_version ON model_lane_authority TYPE int;
DEFINE FIELD IF NOT EXISTS transaction_seq ON model_lane_authority TYPE int;
DEFINE FIELD IF NOT EXISTS ordering_seq ON model_lane_authority TYPE option<int>;
DEFINE FIELD OVERWRITE owner_account_id ON model_lane_authority TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE actor_principal_id ON model_lane_authority TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE authenticated_session_id ON model_lane_authority TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE access_space_id ON model_lane_authority TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE workspace_id ON model_lane_authority TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD IF NOT EXISTS created_at ON model_lane_authority TYPE datetime DEFAULT time::now();
DEFINE INDEX IF NOT EXISTS model_lane_authority_aggregate ON model_lane_authority FIELDS record_kind, aggregate_id, owner_account_id, actor_principal_id, authenticated_session_id, access_space_id, workspace_id UNIQUE;
DEFINE INDEX IF NOT EXISTS model_lane_authority_run ON model_lane_authority FIELDS record_kind, run_id, owner_account_id, workspace_id, event_seq;
DEFINE INDEX IF NOT EXISTS model_lane_authority_idempotency ON model_lane_authority FIELDS record_kind, idempotency_key, owner_account_id, actor_principal_id, authenticated_session_id, access_space_id, workspace_id UNIQUE;
DEFINE INDEX IF NOT EXISTS model_lane_authority_event ON model_lane_authority FIELDS event_id UNIQUE;
DEFINE INDEX IF NOT EXISTS model_lane_authority_kernel_event ON model_lane_authority FIELDS event_ledger_event_id;
DEFINE INDEX IF NOT EXISTS model_lane_authority_recovery_order ON model_lane_authority FIELDS record_kind, run_id, ordering_seq, owner_account_id, actor_principal_id, authenticated_session_id, access_space_id, workspace_id;

DEFINE TABLE IF NOT EXISTS model_lane_schema_registry SCHEMAFULL;
DEFINE FIELD IF NOT EXISTS schema_id ON model_lane_schema_registry TYPE string;
DEFINE FIELD IF NOT EXISTS schema_version ON model_lane_schema_registry TYPE int;
DEFINE FIELD IF NOT EXISTS record_kind ON model_lane_schema_registry TYPE string;
DEFINE FIELD IF NOT EXISTS table_name ON model_lane_schema_registry TYPE string;
DEFINE FIELD IF NOT EXISTS source_component ON model_lane_schema_registry TYPE string;
DEFINE FIELD OVERWRITE owner_account_id ON model_lane_schema_registry TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE actor_principal_id ON model_lane_schema_registry TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE authenticated_session_id ON model_lane_schema_registry TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE access_space_id ON model_lane_schema_registry TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE workspace_id ON model_lane_schema_registry TYPE string ASSERT string::trim($value) != '';
DEFINE INDEX IF NOT EXISTS model_lane_schema_registry_schema_id ON model_lane_schema_registry FIELDS schema_id, owner_account_id, actor_principal_id, authenticated_session_id, access_space_id, workspace_id UNIQUE;

DEFINE TABLE IF NOT EXISTS model_lane_routing_execution SCHEMAFULL;
DEFINE FIELD IF NOT EXISTS execution_id ON model_lane_routing_execution TYPE string;
DEFINE FIELD IF NOT EXISTS run_id ON model_lane_routing_execution TYPE string;
DEFINE FIELD IF NOT EXISTS revision ON model_lane_routing_execution TYPE int;
DEFINE FIELD IF NOT EXISTS context_hash ON model_lane_routing_execution TYPE string ASSERT string::len($value) = 64;
DEFINE FIELD IF NOT EXISTS record_json ON model_lane_routing_execution TYPE object FLEXIBLE;
DEFINE FIELD IF NOT EXISTS record_json.* ON model_lane_routing_execution TYPE any;
DEFINE FIELD IF NOT EXISTS event_ledger_event_id ON model_lane_routing_execution TYPE record<kernel_event_ledger> ASSERT record::exists($value) REFERENCE ON DELETE REJECT;
DEFINE FIELD OVERWRITE event_ledger_seq ON model_lane_routing_execution TYPE int ASSERT $value > 0;
DEFINE FIELD IF NOT EXISTS owner_account_id ON model_lane_routing_execution TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD IF NOT EXISTS actor_principal_id ON model_lane_routing_execution TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD IF NOT EXISTS authenticated_session_id ON model_lane_routing_execution TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD IF NOT EXISTS access_space_id ON model_lane_routing_execution TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD IF NOT EXISTS workspace_id ON model_lane_routing_execution TYPE string ASSERT string::trim($value) != '';
DEFINE INDEX IF NOT EXISTS model_lane_routing_execution_scope_identity ON model_lane_routing_execution FIELDS execution_id, owner_account_id, actor_principal_id, authenticated_session_id, access_space_id, workspace_id UNIQUE;
DEFINE INDEX IF NOT EXISTS model_lane_routing_execution_scope_run ON model_lane_routing_execution FIELDS run_id, owner_account_id, actor_principal_id, authenticated_session_id, access_space_id, workspace_id, event_ledger_seq;

DEFINE TABLE IF NOT EXISTS model_lane_routing_stage_attempt SCHEMAFULL;
DEFINE FIELD IF NOT EXISTS attempt_id ON model_lane_routing_stage_attempt TYPE string;
DEFINE FIELD IF NOT EXISTS execution_id ON model_lane_routing_stage_attempt TYPE string;
DEFINE FIELD IF NOT EXISTS run_id ON model_lane_routing_stage_attempt TYPE string;
DEFINE FIELD IF NOT EXISTS stage_id ON model_lane_routing_stage_attempt TYPE string;
DEFINE FIELD IF NOT EXISTS attempt ON model_lane_routing_stage_attempt TYPE int;
DEFINE FIELD IF NOT EXISTS state ON model_lane_routing_stage_attempt TYPE string;
DEFINE FIELD IF NOT EXISTS lease_owner ON model_lane_routing_stage_attempt TYPE option<string>;
DEFINE FIELD IF NOT EXISTS fencing_token ON model_lane_routing_stage_attempt TYPE option<string>;
DEFINE FIELD IF NOT EXISTS lease_expires_at_unix_ms ON model_lane_routing_stage_attempt TYPE option<int>;
DEFINE FIELD IF NOT EXISTS record_json ON model_lane_routing_stage_attempt TYPE object FLEXIBLE;
DEFINE FIELD IF NOT EXISTS record_json.* ON model_lane_routing_stage_attempt TYPE any;
DEFINE FIELD IF NOT EXISTS event_ledger_event_id ON model_lane_routing_stage_attempt TYPE record<kernel_event_ledger> ASSERT record::exists($value) REFERENCE ON DELETE REJECT;
DEFINE FIELD OVERWRITE event_ledger_seq ON model_lane_routing_stage_attempt TYPE int ASSERT $value > 0;
DEFINE FIELD IF NOT EXISTS owner_account_id ON model_lane_routing_stage_attempt TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD IF NOT EXISTS actor_principal_id ON model_lane_routing_stage_attempt TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD IF NOT EXISTS authenticated_session_id ON model_lane_routing_stage_attempt TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD IF NOT EXISTS access_space_id ON model_lane_routing_stage_attempt TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD IF NOT EXISTS workspace_id ON model_lane_routing_stage_attempt TYPE string ASSERT string::trim($value) != '';
DEFINE INDEX IF NOT EXISTS model_lane_routing_attempt_scope_identity ON model_lane_routing_stage_attempt FIELDS execution_id, stage_id, attempt, owner_account_id, actor_principal_id, authenticated_session_id, access_space_id, workspace_id UNIQUE;
DEFINE INDEX IF NOT EXISTS model_lane_routing_attempt_scope_run ON model_lane_routing_stage_attempt FIELDS run_id, owner_account_id, actor_principal_id, authenticated_session_id, access_space_id, workspace_id, event_ledger_seq;

DEFINE TABLE IF NOT EXISTS model_lane_routing_outbox SCHEMAFULL;
DEFINE FIELD IF NOT EXISTS command_id ON model_lane_routing_outbox TYPE string;
DEFINE FIELD IF NOT EXISTS execution_id ON model_lane_routing_outbox TYPE string;
DEFINE FIELD IF NOT EXISTS run_id ON model_lane_routing_outbox TYPE string;
DEFINE FIELD IF NOT EXISTS stage_id ON model_lane_routing_outbox TYPE string;
DEFINE FIELD IF NOT EXISTS attempt ON model_lane_routing_outbox TYPE int;
DEFINE FIELD IF NOT EXISTS status ON model_lane_routing_outbox TYPE string;
DEFINE FIELD IF NOT EXISTS lease_owner ON model_lane_routing_outbox TYPE option<string>;
DEFINE FIELD IF NOT EXISTS fencing_token ON model_lane_routing_outbox TYPE option<string>;
DEFINE FIELD IF NOT EXISTS lease_expires_at_unix_ms ON model_lane_routing_outbox TYPE option<int>;
DEFINE FIELD IF NOT EXISTS event_ledger_event_id ON model_lane_routing_outbox TYPE record<kernel_event_ledger> ASSERT record::exists($value) REFERENCE ON DELETE REJECT;
DEFINE FIELD OVERWRITE event_ledger_seq ON model_lane_routing_outbox TYPE int ASSERT $value > 0;
DEFINE FIELD IF NOT EXISTS created_at_unix_ms ON model_lane_routing_outbox TYPE int;
DEFINE FIELD IF NOT EXISTS updated_at_unix_ms ON model_lane_routing_outbox TYPE int;
DEFINE FIELD IF NOT EXISTS owner_account_id ON model_lane_routing_outbox TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD IF NOT EXISTS actor_principal_id ON model_lane_routing_outbox TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD IF NOT EXISTS authenticated_session_id ON model_lane_routing_outbox TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD IF NOT EXISTS access_space_id ON model_lane_routing_outbox TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD IF NOT EXISTS workspace_id ON model_lane_routing_outbox TYPE string ASSERT string::trim($value) != '';
DEFINE INDEX IF NOT EXISTS model_lane_routing_outbox_scope_identity ON model_lane_routing_outbox FIELDS command_id, owner_account_id, actor_principal_id, authenticated_session_id, access_space_id, workspace_id UNIQUE;
DEFINE INDEX IF NOT EXISTS model_lane_routing_outbox_scope_run ON model_lane_routing_outbox FIELDS run_id, owner_account_id, actor_principal_id, authenticated_session_id, access_space_id, workspace_id, event_ledger_seq;

DEFINE TABLE IF NOT EXISTS model_lane_routing_extra_event_link SCHEMAFULL;
DEFINE FIELD IF NOT EXISTS execution_id ON model_lane_routing_extra_event_link TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD IF NOT EXISTS run_id ON model_lane_routing_extra_event_link TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD IF NOT EXISTS revision ON model_lane_routing_extra_event_link TYPE int ASSERT $value > 0;
DEFINE FIELD IF NOT EXISTS idempotency_key ON model_lane_routing_extra_event_link TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD IF NOT EXISTS event_ledger_event_id ON model_lane_routing_extra_event_link TYPE record<kernel_event_ledger> ASSERT record::exists($value) REFERENCE ON DELETE REJECT;
DEFINE FIELD OVERWRITE event_ledger_seq ON model_lane_routing_extra_event_link TYPE int ASSERT $value > 0;
DEFINE FIELD IF NOT EXISTS owner_account_id ON model_lane_routing_extra_event_link TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD IF NOT EXISTS actor_principal_id ON model_lane_routing_extra_event_link TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD IF NOT EXISTS authenticated_session_id ON model_lane_routing_extra_event_link TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD IF NOT EXISTS access_space_id ON model_lane_routing_extra_event_link TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD IF NOT EXISTS workspace_id ON model_lane_routing_extra_event_link TYPE string ASSERT string::trim($value) != '';
DEFINE INDEX IF NOT EXISTS model_lane_routing_extra_event_scope_identity ON model_lane_routing_extra_event_link FIELDS execution_id, revision, idempotency_key, owner_account_id, actor_principal_id, authenticated_session_id, access_space_id, workspace_id UNIQUE;
DEFINE INDEX IF NOT EXISTS model_lane_routing_extra_event_scope_replay ON model_lane_routing_extra_event_link FIELDS execution_id, revision, owner_account_id, actor_principal_id, authenticated_session_id, access_space_id, workspace_id, event_ledger_seq;

-- CRDT proposal application identity is consumed by the atomic ModelLane
-- message authority gate. These fields remain on the canonical embedded
-- SurrealDB proposal row; they are not a compatibility projection.
DEFINE FIELD IF NOT EXISTS applied_update_id ON knowledge_crdt_ai_edit_proposals TYPE option<string>;
DEFINE FIELD IF NOT EXISTS applied_update_sha256 ON knowledge_crdt_ai_edit_proposals TYPE option<string>;
DEFINE FIELD IF NOT EXISTS applied_event_id ON knowledge_crdt_ai_edit_proposals TYPE option<record<kernel_event_ledger>> ASSERT $value = NONE OR record::exists($value) REFERENCE ON DELETE REJECT;
DEFINE FIELD IF NOT EXISTS last_transition_event_id ON knowledge_crdt_ai_edit_proposals TYPE record<kernel_event_ledger>;

-- Every CRDT authority row is attributed directly to the same exact
-- five-field ResourceScope used by ModelLane. Missing legacy attribution is
-- intentionally unreadable by the scoped provider.
DEFINE FIELD IF NOT EXISTS owner_account_id ON kernel_crdt_updates TYPE string;
DEFINE FIELD IF NOT EXISTS actor_principal_id ON kernel_crdt_updates TYPE string;
DEFINE FIELD IF NOT EXISTS authenticated_session_id ON kernel_crdt_updates TYPE string;
DEFINE FIELD IF NOT EXISTS access_space_id ON kernel_crdt_updates TYPE string;
DEFINE INDEX IF NOT EXISTS model_lane_crdt_update_scope_ref ON kernel_crdt_updates FIELDS owner_account_id, actor_principal_id, authenticated_session_id, access_space_id, workspace_id, update_bytes_ref UNIQUE;
DEFINE INDEX IF NOT EXISTS model_lane_crdt_update_scope_sequence ON kernel_crdt_updates FIELDS owner_account_id, actor_principal_id, authenticated_session_id, access_space_id, workspace_id, document_id, crdt_document_id, update_seq UNIQUE;

DEFINE FIELD IF NOT EXISTS owner_account_id ON kernel_crdt_snapshots TYPE string;
DEFINE FIELD IF NOT EXISTS actor_principal_id ON kernel_crdt_snapshots TYPE string;
DEFINE FIELD IF NOT EXISTS authenticated_session_id ON kernel_crdt_snapshots TYPE string;
DEFINE FIELD IF NOT EXISTS access_space_id ON kernel_crdt_snapshots TYPE string;
DEFINE INDEX IF NOT EXISTS model_lane_crdt_snapshot_scope_ref ON kernel_crdt_snapshots FIELDS owner_account_id, actor_principal_id, authenticated_session_id, access_space_id, workspace_id, snapshot_bytes_ref UNIQUE;
DEFINE INDEX IF NOT EXISTS model_lane_crdt_snapshot_scope_sequence ON kernel_crdt_snapshots FIELDS owner_account_id, actor_principal_id, authenticated_session_id, access_space_id, workspace_id, document_id, crdt_document_id, covered_update_seq;

DEFINE FIELD IF NOT EXISTS owner_account_id ON knowledge_crdt_agent_lane_leases TYPE string;
DEFINE FIELD IF NOT EXISTS actor_principal_id ON knowledge_crdt_agent_lane_leases TYPE string;
DEFINE FIELD IF NOT EXISTS authenticated_session_id ON knowledge_crdt_agent_lane_leases TYPE string;
DEFINE FIELD IF NOT EXISTS access_space_id ON knowledge_crdt_agent_lane_leases TYPE string;
DEFINE FIELD IF NOT EXISTS workspace_id ON knowledge_crdt_agent_lane_leases TYPE string;
DEFINE FIELD IF NOT EXISTS document_id ON knowledge_crdt_agent_lane_leases TYPE option<string>;
DEFINE FIELD IF NOT EXISTS crdt_document_id ON knowledge_crdt_agent_lane_leases TYPE option<string>;
DEFINE FIELD IF NOT EXISTS recorded_event_id ON knowledge_crdt_agent_lane_leases TYPE record<kernel_event_ledger>;
DEFINE FIELD IF NOT EXISTS last_transition_event_id ON knowledge_crdt_agent_lane_leases TYPE record<kernel_event_ledger>;
DEFINE INDEX IF NOT EXISTS model_lane_crdt_lease_scope_identity ON knowledge_crdt_agent_lane_leases FIELDS owner_account_id, actor_principal_id, authenticated_session_id, access_space_id, workspace_id, lease_id UNIQUE;
DEFINE INDEX IF NOT EXISTS model_lane_crdt_lease_scope_document ON knowledge_crdt_agent_lane_leases FIELDS owner_account_id, actor_principal_id, authenticated_session_id, access_space_id, workspace_id, document_id, crdt_document_id, scope_kind, scope_id, expires_at_utc;

DEFINE FIELD IF NOT EXISTS owner_account_id ON knowledge_crdt_ai_edit_proposals TYPE string;
DEFINE FIELD IF NOT EXISTS actor_principal_id ON knowledge_crdt_ai_edit_proposals TYPE string;
DEFINE FIELD IF NOT EXISTS authenticated_session_id ON knowledge_crdt_ai_edit_proposals TYPE string;
DEFINE FIELD IF NOT EXISTS access_space_id ON knowledge_crdt_ai_edit_proposals TYPE string;
DEFINE INDEX IF NOT EXISTS model_lane_crdt_proposal_scope_identity ON knowledge_crdt_ai_edit_proposals FIELDS owner_account_id, actor_principal_id, authenticated_session_id, access_space_id, workspace_id, proposal_id UNIQUE;
DEFINE INDEX IF NOT EXISTS model_lane_crdt_proposal_scope_document ON knowledge_crdt_ai_edit_proposals FIELDS owner_account_id, actor_principal_id, authenticated_session_id, access_space_id, workspace_id, document_id, crdt_document_id, review_state, created_at_utc;
DEFINE INDEX IF NOT EXISTS model_lane_crdt_proposal_scope_applied_event ON knowledge_crdt_ai_edit_proposals FIELDS owner_account_id, actor_principal_id, authenticated_session_id, access_space_id, workspace_id, applied_event_id;

-- Coordination uses each original operation envelope; it grants no business-row authority.
DEFINE FUNCTION OVERWRITE fn::wp012_context_coordination_key($row: object) {
    RETURN crypto::sha256(type::string([$row.owner_account_id, $row.actor_principal_id,
        $row.authenticated_session_id, $row.access_space_id, $row.workspace_id,
        $row.domain_kind, $row.domain_id]));
};
DEFINE FUNCTION OVERWRITE fn::wp012_model_lane_operation_matrix($kind: string, $purpose: string, $action: string) {
    RETURN ($kind = 'workspace' AND $purpose IN ['consume', 'source_read'] AND $action = 'read')
        OR ($kind = 'workspace' AND $purpose IN ['artifact_binding', 'handoff', 'guarded_message', 'run_create', 'lane_create', 'workspace_claim', 'index_run_claim', 'cloud_projection_plan', 'cloud_consent_receipt', 'cloud_run', 'cloud_lane', 'cloud_consent_denial'] AND $action = 'create')
        OR ($kind = 'rich_document' AND $purpose IN ['crdt_update', 'crdt_snapshot', 'document_claim', 'proposal_record', 'proposal_decide', 'proposal_apply'] AND $action = 'update')
        OR ($kind IN ['workspace', 'rich_document'] AND $purpose IN ['lease_renew', 'lease_release', 'lease_expiry'] AND $action = 'update');
};
DEFINE FUNCTION OVERWRITE fn::wp012_context_coordination_purpose($purpose: string, $action: string) {
    RETURN fn::wp012_model_lane_operation_matrix('workspace', $purpose, $action)
        OR fn::wp012_model_lane_operation_matrix('rich_document', $purpose, $action);
};
DEFINE FUNCTION OVERWRITE fn::wp012_model_lane_resource_domain($resource: record<protected_resources>, $workspace: string, $kind: string, $subject: string) {
    IF $resource.resource_kind != $kind OR $resource.external_resource_id != $subject
        OR $resource.owner_account_id != $auth.account_id OR $resource.access_space_id != $auth.access_space_id
        OR $resource.lifecycle_state != 'active' OR $resource.created_by_principal_id.status != 'enabled' { RETURN NONE; };
    IF $kind = 'workspace' { RETURN IF $subject = $workspace { $resource } ELSE { NONE }; };
    LET $parent = $resource.parent_resource_id;
    RETURN IF $kind = 'rich_document' AND $parent != NONE AND $parent.resource_kind = 'workspace'
        AND $parent.external_resource_id = $workspace AND $parent.owner_account_id = $auth.account_id
        AND $parent.access_space_id = $auth.access_space_id AND $parent.lifecycle_state = 'active'
        AND $parent.created_by_principal_id.status = 'enabled' { $parent } ELSE { NONE };
};
DEFINE FUNCTION OVERWRITE fn::wp012_model_lane_original_grant($resource: record<protected_resources>, $grant: record<resource_grants>, $capability: string, $action: string) {
    RETURN fn::mt109_live_session() AND $grant.resource_id = $resource
        AND $grant.account_id = $auth.account_id AND $grant.principal_id = $auth.principal_id
        AND $grant.access_space_id = $auth.access_space_id AND $grant.status = 'active'
        AND $grant.revoked_at = NONE AND ($grant.expires_at = NONE OR $grant.expires_at > time::now())
        AND $grant.delegation_chain = $auth.delegation_chain AND $grant.actions CONTAINS $action
        AND $grant.capability_ids CONTAINS $capability AND $resource.policy_version <= $grant.policy_version
        AND $grant.policy_version <= $auth.policy_version
        AND fn::mt109_ledger_access($resource, $auth.id, $capability, $action);
};
DEFINE FUNCTION OVERWRITE fn::wp012_context_coordination_domain($row: object) {
    IF !fn::mt109_live_session() OR $row.owner_account_id != record::id($auth.account_id)
        OR $row.actor_principal_id != record::id($auth.principal_id)
        OR $row.authenticated_session_id != record::id($auth.id)
        OR $row.access_space_id != record::id($auth.access_space_id)
        OR $row.authority_session_id != $auth.id
        OR $row.domain_key != fn::wp012_context_coordination_key($row) { RETURN false; };
    LET $resource = $row.canonical_workspace;
    IF $resource.resource_kind != 'workspace' OR $resource.external_resource_id != $row.workspace_id
        OR $resource.owner_account_id != $auth.account_id OR $resource.access_space_id != $auth.access_space_id
        OR $resource.lifecycle_state != 'active' OR $resource.created_by_principal_id.status != 'enabled' { RETURN false; };
    IF $row.domain_kind = 'workspace' {
        RETURN $row.domain_id = $row.workspace_id AND $row.canonical_run = NONE AND $row.canonical_lane = NONE;
    };
    LET $run = $row.canonical_run;
    IF $run = NONE OR $run.record_kind != 'run' OR $run.aggregate_id != $row.run_id
        OR $run.owner_account_id != $row.owner_account_id OR $run.actor_principal_id != $row.actor_principal_id
        OR $run.authenticated_session_id != $row.authenticated_session_id OR $run.access_space_id != $row.access_space_id
        OR $run.workspace_id != $row.workspace_id OR $run.event_id != record::id($run.event_ledger_event_id)
        OR $run.event_seq != $run.event_ledger_event_id.event_sequence
        OR $run.event_ledger_event_id.owner_account_id != $row.owner_account_id
        OR $run.event_ledger_event_id.actor_principal_id != $row.actor_principal_id
        OR $run.event_ledger_event_id.authenticated_session_id != $row.authenticated_session_id
        OR $run.event_ledger_event_id.access_space_id != $row.access_space_id
        OR $run.event_ledger_event_id.workspace_id != $row.workspace_id { RETURN false; };
    IF $row.domain_kind = 'run' { RETURN $row.domain_id = $row.run_id AND $row.canonical_lane = NONE; };
    LET $lane = $row.canonical_lane;
    RETURN $row.domain_kind = 'lane' AND $lane != NONE AND $lane.record_kind = 'lane'
        AND $lane.aggregate_id = $row.domain_id AND $lane.run_id = $row.run_id
        AND $lane.owner_account_id = $row.owner_account_id AND $lane.actor_principal_id = $row.actor_principal_id
        AND $lane.authenticated_session_id = $row.authenticated_session_id AND $lane.access_space_id = $row.access_space_id
        AND $lane.workspace_id = $row.workspace_id AND $lane.event_id = record::id($lane.event_ledger_event_id)
        AND $lane.event_seq = $lane.event_ledger_event_id.event_sequence
        AND $lane.event_ledger_event_id.owner_account_id = $row.owner_account_id
        AND $lane.event_ledger_event_id.actor_principal_id = $row.actor_principal_id
        AND $lane.event_ledger_event_id.authenticated_session_id = $row.authenticated_session_id
        AND $lane.event_ledger_event_id.access_space_id = $row.access_space_id
        AND $lane.event_ledger_event_id.workspace_id = $row.workspace_id;
};
DEFINE FUNCTION OVERWRITE fn::wp012_context_coordination_envelope($row: object, $resource: record<protected_resources>, $grant: record<resource_grants>, $capability: string, $action: string, $purpose: string, $kind: string, $subject: string) {
    RETURN fn::wp012_context_coordination_domain($row)
        AND fn::wp012_model_lane_operation_matrix($kind, $purpose, $action)
        AND fn::wp012_model_lane_resource_domain($resource, $row.workspace_id, $kind, $subject) = $row.canonical_workspace
        AND fn::wp012_model_lane_original_grant($resource, $grant, $capability, $action)
        AND ($kind != 'workspace' OR fn::mt109_has_workspace_access($row.workspace_id, $action, $capability));
};
DEFINE FUNCTION OVERWRITE fn::wp012_context_coordination_candidate($row: object, $resource: record<protected_resources>, $grant: record<resource_grants>, $capability: string, $action: string, $purpose: string) {
    RETURN fn::wp012_context_coordination_envelope($row, $resource, $grant, $capability, $action, $purpose, $subject_kind, $subject_id);
};
DEFINE FUNCTION OVERWRITE fn::wp012_context_coordination_holder($row: object) {
    RETURN fn::wp012_context_coordination_envelope($row, $row.authority_resource_id, $row.authority_grant_id,
        $row.authority_capability_id, $row.authority_action, $row.authority_purpose,
        $row.authority_subject_kind, $row.authority_subject_id);
};
DEFINE FUNCTION OVERWRITE fn::wp012_context_coordination_candidate_is_holder($row: object, $grant: record<resource_grants>, $capability: string, $action: string, $purpose: string) {
    RETURN $grant = $row.authority_grant_id AND $grant.resource_id = $row.authority_resource_id
        AND $row.authority_subject_kind = $subject_kind AND $row.authority_subject_id = $subject_id
        AND $capability = $row.authority_capability_id
        AND $action = $row.authority_action AND $purpose = $row.authority_purpose
        AND fn::wp012_context_coordination_holder($row);
};
DEFINE TABLE OVERWRITE model_lane_context_coordination SCHEMAFULL
    PERMISSIONS FOR select WHERE fn::wp012_context_coordination_candidate($this, type::record('protected_resources', $resource_id), type::record('resource_grants', $grant_id), $capability_id, $coordination_action, $coordination_purpose)
    FOR create WHERE fn::wp012_context_coordination_candidate($this, type::record('protected_resources', $resource_id), type::record('resource_grants', $grant_id), $capability_id, $coordination_action, $coordination_purpose) AND fn::wp012_context_coordination_candidate_is_holder($this, type::record('resource_grants', $grant_id), $capability_id, $coordination_action, $coordination_purpose) AND state = 'held'
        AND generation = 1 AND revision = 1 AND admitted_at <= time::now()
        AND deadline > admitted_at AND deadline <= admitted_at + 5m AND released_at = NONE
        AND token_proof = NONE AND validated_admission_at = NONE
    FOR update WHERE fn::wp012_context_coordination_candidate($this, type::record('protected_resources', $resource_id), type::record('resource_grants', $grant_id), $capability_id, $coordination_action, $coordination_purpose)
    FOR delete NONE;
DEFINE FIELD OVERWRITE domain_key ON model_lane_context_coordination TYPE string ASSERT $value = record::id($this.id);
DEFINE FIELD OVERWRITE domain_kind ON model_lane_context_coordination TYPE 'run' | 'lane' | 'workspace';
DEFINE FIELD OVERWRITE domain_id ON model_lane_context_coordination TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE run_id ON model_lane_context_coordination TYPE option<string>;
DEFINE FIELD OVERWRITE canonical_run ON model_lane_context_coordination TYPE option<record<model_lane_authority>>;
DEFINE FIELD OVERWRITE canonical_lane ON model_lane_context_coordination TYPE option<record<model_lane_authority>>;
DEFINE FIELD OVERWRITE owner_account_id ON model_lane_context_coordination TYPE string;
DEFINE FIELD OVERWRITE actor_principal_id ON model_lane_context_coordination TYPE string;
DEFINE FIELD OVERWRITE authenticated_session_id ON model_lane_context_coordination TYPE string;
DEFINE FIELD OVERWRITE access_space_id ON model_lane_context_coordination TYPE string;
DEFINE FIELD OVERWRITE workspace_id ON model_lane_context_coordination TYPE string;
DEFINE FIELD OVERWRITE canonical_workspace ON model_lane_context_coordination TYPE record<protected_resources>;
DEFINE FIELD OVERWRITE authority_subject_kind ON model_lane_context_coordination TYPE 'workspace' | 'rich_document';
DEFINE FIELD OVERWRITE authority_subject_id ON model_lane_context_coordination TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE authority_resource_id ON model_lane_context_coordination TYPE record<protected_resources>;
DEFINE FIELD OVERWRITE authority_grant_id ON model_lane_context_coordination TYPE record<resource_grants>;
DEFINE FIELD OVERWRITE authority_session_id ON model_lane_context_coordination TYPE record<authenticated_sessions>;
DEFINE FIELD OVERWRITE authority_capability_id ON model_lane_context_coordination TYPE string;
DEFINE FIELD OVERWRITE authority_action ON model_lane_context_coordination TYPE 'read' | 'create' | 'update';
DEFINE FIELD OVERWRITE authority_purpose ON model_lane_context_coordination TYPE 'consume' | 'artifact_binding' | 'handoff' | 'guarded_message' | 'document_claim' | 'workspace_claim' | 'index_run_claim' | 'lease_renew' | 'lease_release' | 'lease_expiry' | 'crdt_update' | 'crdt_snapshot' | 'proposal_record' | 'proposal_decide' | 'proposal_apply' | 'cloud_projection_plan' | 'cloud_consent_receipt' | 'cloud_run' | 'cloud_lane' | 'cloud_consent_denial';
DEFINE FIELD OVERWRITE state ON model_lane_context_coordination TYPE 'held' | 'released';
DEFINE FIELD OVERWRITE holder_operation_id ON model_lane_context_coordination TYPE uuid;
DEFINE FIELD OVERWRITE generation ON model_lane_context_coordination TYPE int ASSERT $value > 0;
DEFINE FIELD OVERWRITE revision ON model_lane_context_coordination TYPE int ASSERT $value > 0;
DEFINE FIELD OVERWRITE token_hash ON model_lane_context_coordination TYPE string ASSERT string::len($value) = 64 PERMISSIONS FOR select NONE;
DEFINE FIELD OVERWRITE token_proof ON model_lane_context_coordination TYPE option<string> PERMISSIONS FOR select NONE;
DEFINE FIELD OVERWRITE admitted_at ON model_lane_context_coordination TYPE datetime
    VALUE IF $before = NONE OR (($before.state = 'released' OR $before.deadline <= time::now()) AND $this.state = 'held') {
        time::now()
    } ELSE { $before.admitted_at };
DEFINE FIELD OVERWRITE validated_admission_at ON model_lane_context_coordination TYPE option<datetime>
    VALUE IF $before = NONE OR $this.generation != $before.generation { NONE }
        ELSE IF $before.validated_admission_at = NONE AND $value != NONE { time::now() }
        ELSE { $before.validated_admission_at };
DEFINE FIELD OVERWRITE deadline ON model_lane_context_coordination TYPE datetime;
DEFINE FIELD OVERWRITE released_at ON model_lane_context_coordination TYPE option<datetime>
    VALUE IF $this.state = 'released' { time::now() } ELSE { NONE };
DEFINE INDEX OVERWRITE model_lane_context_coordination_domain ON model_lane_context_coordination
    FIELDS owner_account_id, actor_principal_id, authenticated_session_id, access_space_id, workspace_id, domain_kind, domain_id UNIQUE;
DEFINE EVENT OVERWRITE wp012_context_coordination_transition ON model_lane_context_coordination
WHEN $event = 'UPDATE'
THEN {
    IF !fn::wp012_context_coordination_domain($before) OR !fn::wp012_context_coordination_candidate($after, type::record('protected_resources', $resource_id), type::record('resource_grants', $grant_id), $capability_id, $coordination_action, $coordination_purpose) OR !fn::wp012_context_coordination_candidate_is_holder($after, type::record('resource_grants', $grant_id), $capability_id, $coordination_action, $coordination_purpose)
        OR $before.id != $after.id OR $before.domain_key != $after.domain_key
        OR $before.domain_kind != $after.domain_kind OR $before.domain_id != $after.domain_id
        OR $before.run_id != $after.run_id OR $before.canonical_run != $after.canonical_run
        OR $before.canonical_lane != $after.canonical_lane OR $before.owner_account_id != $after.owner_account_id
        OR $before.actor_principal_id != $after.actor_principal_id OR $before.authenticated_session_id != $after.authenticated_session_id
        OR $before.access_space_id != $after.access_space_id OR $before.workspace_id != $after.workspace_id
        OR $before.canonical_workspace != $after.canonical_workspace
        OR $before.authority_session_id != $after.authority_session_id OR $after.revision != $before.revision + 1
        OR $after.token_hash = $before.token_hash { THROW 'HSK-403-CONTEXT-COORDINATION'; };
    IF $before.generation = $after.generation AND $before.validated_admission_at != $after.validated_admission_at {
        IF $before.state != 'held' OR $after.state != 'held' OR $before.deadline <= time::now()
            OR $before.validated_admission_at != NONE OR $after.validated_admission_at = NONE
            OR $before.domain_kind != 'workspace' OR $before.authority_purpose != 'guarded_message'
            OR $before.generation != $after.generation { THROW 'HSK-403-CONTEXT-ADMISSION'; };
        LET $covering = SELECT * FROM knowledge_crdt_agent_lane_leases WHERE owner_account_id = $before.owner_account_id
            AND actor_principal_id = $before.actor_principal_id AND authenticated_session_id = $before.authenticated_session_id
            AND access_space_id = $before.access_space_id AND workspace_id = $before.workspace_id
            AND claimed_at_utc <= $after.validated_admission_at AND expires_at_utc > $after.validated_admission_at
            AND released_at_utc = NONE AND expired_at_utc = NONE
            AND ((scope_kind = 'workspace' AND scope_id = $before.workspace_id)
                OR (scope_kind = 'document' AND document_id = $admission_document_id
                    AND crdt_document_id = $admission_crdt_document_id AND scope_id = $admission_crdt_document_id));
        IF array::len($covering) != 1 OR !fn::wp012_model_lane_crdt_lease_identity($covering[0]) {
            THROW 'HSK-403-CONTEXT-ADMISSION';
        };
    };
    IF $before.state = 'held' AND $before.deadline > time::now() {
        IF !fn::wp012_context_coordination_holder($before)
            OR $before.authority_resource_id != $after.authority_resource_id
            OR $before.authority_subject_kind != $after.authority_subject_kind
            OR $before.authority_subject_id != $after.authority_subject_id
            OR $before.authority_grant_id != $after.authority_grant_id
            OR $before.authority_capability_id != $after.authority_capability_id
            OR $before.authority_action != $after.authority_action OR $before.authority_purpose != $after.authority_purpose
            OR $after.holder_operation_id != $before.holder_operation_id OR $after.generation != $before.generation
            OR $after.admitted_at != $before.admitted_at OR $after.token_proof = NONE
            OR crypto::sha256($after.token_proof) != $before.token_hash
            OR !($after.state IN ['held', 'released'])
            OR ($after.state = 'held' AND ($after.deadline <= time::now() OR $after.deadline > time::now() + 5m OR $after.released_at != NONE))
            OR ($after.state = 'released' AND ($after.deadline != $before.deadline OR $after.released_at = NONE OR $after.released_at > time::now())) {
            THROW 'HSK-403-CONTEXT-COORDINATION';
        };
    } ELSE {
        IF $after.state != 'held' OR $after.generation != $before.generation + 1
            OR $after.holder_operation_id = $before.holder_operation_id OR $after.token_proof != NONE
            OR $after.admitted_at > time::now() OR $after.deadline <= time::now()
            OR $after.deadline > time::now() + 5m OR $after.released_at != NONE {
            THROW 'HSK-403-CONTEXT-COORDINATION';
        };
    };
};

-- Proofs authorize only the held coordination domain, never the business mutation.
DEFINE FUNCTION OVERWRITE fn::wp012_context_coordination_fence($row: object, $kind: string, $identity: string) {
    IF !fn::wp012_model_lane_original_operation($row) OR !type::is_array($wp012_fences) { RETURN false; };
    LET $domain = {
        owner_account_id: $row.owner_account_id, actor_principal_id: $row.actor_principal_id,
        authenticated_session_id: $row.authenticated_session_id, access_space_id: $row.access_space_id,
        workspace_id: $row.workspace_id, domain_kind: $kind, domain_id: $identity
    };
    LET $key = fn::wp012_context_coordination_key($domain);
    LET $proofs = array::filter($wp012_fences, |$proof| $proof.domain_key = $key);
    IF array::len($proofs) != 1 { RETURN false; };
    LET $proof = $proofs[0];
    LET $gate = (SELECT * FROM ONLY type::record('model_lane_context_coordination', $key));
    RETURN fn::wp012_context_coordination_domain($gate)
        AND $gate.state = 'held' AND $gate.deadline > time::now()
        AND $gate.holder_operation_id = $proof.operation_id
        AND $gate.generation = $proof.generation AND $gate.revision = $proof.revision
        AND type::is_string($proof.token) AND crypto::sha256($proof.token) = $gate.token_hash
        AND $gate.authority_resource_id = type::record('protected_resources', $wp012_original_resource)
        AND $gate.authority_grant_id = type::record('resource_grants', $wp012_original_grant)
        AND $gate.authority_session_id = $auth.id
        AND $gate.authority_action = $wp012_original_action
        AND $gate.authority_capability_id = $wp012_original_capability
        AND $gate.authority_purpose = $wp012_original_purpose
        AND fn::wp012_context_coordination_holder($gate);
};

-- Catalog metadata is prepared administratively; runtime cannot seed or repair it.
DEFINE FUNCTION OVERWRITE fn::wp012_model_lane_original_operation($row: object) {
    IF !fn::mt109_live_session() OR $wp012_original_grant = NONE
        OR $wp012_original_workspace != $row.workspace_id
        OR $wp012_original_session != record::id($auth.id)
        OR $row.owner_account_id != record::id($auth.account_id)
        OR $row.actor_principal_id != record::id($auth.principal_id)
        OR $row.authenticated_session_id != record::id($auth.id)
        OR $row.access_space_id != record::id($auth.access_space_id) { RETURN false; };
    LET $resource = type::record('protected_resources', $wp012_original_resource);
    LET $grant = type::record('resource_grants', $wp012_original_grant);
    LET $kind = IF $wp012_original_subject_kind = NONE { 'workspace' } ELSE { $wp012_original_subject_kind };
    LET $subject = IF $wp012_original_subject_id = NONE { $row.workspace_id } ELSE { $wp012_original_subject_id };
    RETURN fn::wp012_model_lane_resource_domain($resource, $row.workspace_id, $kind, $subject) != NONE
        AND fn::wp012_model_lane_original_grant($resource, $grant, $wp012_original_capability, $wp012_original_action)
        AND fn::wp012_model_lane_operation_matrix($kind, $wp012_original_purpose, $wp012_original_action)
        AND ($wp012_original_purpose NOT IN ['proposal_record', 'proposal_decide', 'proposal_apply'] OR $wp012_original_capability = 'fs.write')
        AND ($kind != 'workspace' OR fn::mt109_has_workspace_access($row.workspace_id, $wp012_original_action, $wp012_original_capability));
};
DEFINE FUNCTION OVERWRITE fn::wp012_model_lane_catalog_read($row: object) {
    IF !fn::wp012_model_lane_original_operation($row)
        OR $row.record_kind != 'model_lane' OR $row.table_name != 'model_lane_authority'
        OR $row.source_component != 'swarm_orchestration::model_lane' { RETURN false; };
    RETURN $wp012_original_action = 'read'
        OR fn::wp012_context_coordination_purpose($wp012_original_purpose, $wp012_original_action);
};
DEFINE TABLE OVERWRITE model_lane_schema_registry SCHEMAFULL
    PERMISSIONS FOR select WHERE fn::wp012_model_lane_catalog_read($this)
    FOR create, update, delete NONE;

DEFINE FUNCTION OVERWRITE fn::wp012_model_lane_source_kind($kind: string) {
    RETURN $kind IN ['run', 'lane', 'message', 'context_artifact', 'context_handoff'];
};
DEFINE FUNCTION OVERWRITE fn::wp012_model_lane_crdt_admission($row: object) {
    LET $record = encoding::json::decode($row.record_json);
    LET $binding = $record.crdt_authority_binding;
    IF $binding = NONE OR $binding = NULL { RETURN true; };
    IF !type::is_object($binding) OR $wp012_original_purpose != 'guarded_message'
        OR !fn::wp012_context_coordination_fence($row, 'workspace', $row.workspace_id)
        OR $binding.run_id != $row.run_id OR $binding.workspace_id != $row.workspace_id
        OR $binding.lane_id != $record.inner.from_lane_id { RETURN false; };
    LET $key = fn::wp012_context_coordination_key(object::extend($row, { domain_kind: 'workspace', domain_id: $row.workspace_id }));
    LET $gate = (SELECT * FROM ONLY type::record('model_lane_context_coordination', $key));
    LET $admitted = type::datetime($binding.lease_admitted_at_utc);
    IF $gate.validated_admission_at = NONE OR $admitted != $gate.validated_admission_at { RETURN false; };
    LET $covering = (SELECT * FROM knowledge_crdt_agent_lane_leases
        WHERE owner_account_id = $row.owner_account_id AND actor_principal_id = $row.actor_principal_id
        AND authenticated_session_id = $row.authenticated_session_id AND access_space_id = $row.access_space_id
        AND workspace_id = $row.workspace_id AND claimed_at_utc <= $admitted AND expires_at_utc > $admitted
        AND released_at_utc = NONE AND expired_at_utc = NONE
        AND ((scope_kind = 'workspace' AND scope_id = $row.workspace_id)
            OR (scope_kind = 'document' AND scope_id = $binding.crdt_document_id
                AND document_id = $binding.document_id AND crdt_document_id = $binding.crdt_document_id)));
    IF array::len($covering) != 1 { RETURN false; };
    LET $lease = $covering[0];
    IF $record.inner.kind = 'proposal' AND ($binding.crdt_proposal_ref = NONE OR $binding.crdt_proposal_ref = NULL) { RETURN false; };
    IF $binding.crdt_proposal_ref != NONE AND $binding.crdt_proposal_ref != NULL {
        LET $proposal_key = crypto::sha256(string::join(['handshake.model-lane.authority.v1', 'crdt_proposal', $binding.crdt_proposal_ref,
            $row.owner_account_id, $row.actor_principal_id, $row.authenticated_session_id,
            $row.access_space_id, $row.workspace_id], '\u0000'));
        LET $proposal = SELECT * FROM ONLY type::record('knowledge_crdt_ai_edit_proposals', $proposal_key);
        IF !fn::wp012_model_lane_proposal_identity($proposal) OR $proposal.review_state != 'approved'
            OR $proposal.document_id != $binding.document_id OR $proposal.crdt_document_id != $binding.crdt_document_id
            OR $proposal.actor_id != $binding.actor_id OR $proposal.actor_kind != $binding.actor_kind
            OR $proposal.session_id != $binding.crdt_session_id OR $proposal.correlation_id != $binding.crdt_trace_id
            OR $proposal.lease_id != $lease.id OR $proposal.applied_update_id != $binding.update_id
            OR $proposal.applied_update_sha256 != $proposal.diff_sha256
            OR $proposal.applied_event_id.payload.update_event_ledger_event_id != $binding.update_event_ledger_event_id
            OR !fn::wp012_model_lane_application_evidence($proposal,
                (SELECT * FROM ONLY type::record('kernel_crdt_updates', crypto::sha256(string::join([
                    'handshake.model-lane.authority.v1', 'crdt_update', $binding.update_id,
                    $row.owner_account_id, $row.actor_principal_id, $row.authenticated_session_id,
                    $row.access_space_id, $row.workspace_id], '\u0000')))),
                (SELECT * FROM ONLY $proposal.applied_event_id)) {
            RETURN false;
        };
    };
    RETURN fn::wp012_model_lane_crdt_lease_identity($lease)
        AND $lease.lease_id = $binding.lease_id AND $lease.lane_id = $binding.lane_id
        AND $lease.actor_id = $binding.actor_id AND $lease.actor_kind = $binding.actor_kind
        AND $lease.session_id = $binding.crdt_session_id AND $lease.correlation_id = $binding.lease_correlation_id
        AND $lease.correlation_id = $binding.crdt_trace_id
        AND $lease.scope_kind = $binding.lease_scope_kind AND $lease.scope_id = $binding.lease_scope_id
        AND $lease.claimed_at_utc = type::datetime($binding.lease_claimed_at_utc)
        AND $lease.expires_at_utc >= type::datetime($binding.lease_expires_at_utc);
};
DEFINE FUNCTION OVERWRITE fn::wp012_model_lane_source_access($row: object, $write: bool) {
    IF !fn::wp012_model_lane_original_operation($row)
        OR !fn::wp012_model_lane_source_kind($row.record_kind) { RETURN false; };
    IF $write AND $row.record_kind = 'message' AND !fn::wp012_model_lane_crdt_admission($row) { RETURN false; };
    IF $write AND $row.record_kind IN ['context_artifact', 'context_handoff'] {
        IF !fn::wp012_context_coordination_fence($row, 'run', $row.run_id) { RETURN false; };
        IF $row.record_kind = 'context_handoff' {
            LET $record = encoding::json::decode($row.record_json);
            IF !type::is_string($record.source_lane_id) OR !type::is_string($record.downstream_lane_id)
                OR !fn::wp012_context_coordination_fence($row, 'lane', $record.source_lane_id)
                OR !fn::wp012_context_coordination_fence($row, 'lane', $record.downstream_lane_id) { RETURN false; };
        };
    };
    IF !$write AND $wp012_original_action = 'read' { RETURN true; };
    IF !$write AND $wp012_original_action = 'update'
        AND $wp012_original_subject_kind = 'rich_document'
        AND $wp012_original_purpose IN ['crdt_update', 'crdt_snapshot', 'document_claim', 'lease_renew', 'lease_release', 'lease_expiry', 'proposal_record', 'proposal_decide', 'proposal_apply'] {
        RETURN $row.record_kind IN ['run', 'lane'];
    };
    IF $wp012_original_action = 'create' {
        IF $wp012_original_purpose = 'run_create' { RETURN $row.record_kind = 'run'; };
        IF $wp012_original_purpose = 'lane_create' { RETURN $row.record_kind = 'lane' OR (!$write AND $row.record_kind = 'run'); };
        IF !$write AND $wp012_original_purpose IN ['document_claim', 'workspace_claim', 'source_root_claim', 'index_run_claim'] { RETURN $row.record_kind IN ['run', 'lane']; };
        IF $wp012_original_purpose = 'guarded_message' { RETURN $row.record_kind = 'message' OR (!$write AND $row.record_kind IN ['run', 'lane']); };
        IF $wp012_original_purpose = 'artifact_binding' { RETURN $row.record_kind = 'context_artifact' OR (!$write AND $row.record_kind IN ['run', 'lane']); };
        IF $wp012_original_purpose = 'handoff' { RETURN $row.record_kind = 'context_handoff' OR (!$write AND $row.record_kind IN ['run', 'lane', 'message', 'context_artifact']); };
    };
    RETURN false;
};
DEFINE FUNCTION OVERWRITE fn::wp012_model_lane_source_identity($row: object) {
    IF !fn::wp012_model_lane_source_kind($row.record_kind)
        OR !type::is_string($row.aggregate_id) OR string::trim($row.aggregate_id) = ''
        OR !type::is_string($row.run_id) OR string::trim($row.run_id) = ''
        OR !type::is_string($row.idempotency_key) OR string::trim($row.idempotency_key) = '' { RETURN false; };
    LET $key = crypto::sha256(string::join([
        'handshake.model-lane.authority.v1', $row.record_kind, $row.aggregate_id,
        $row.owner_account_id, $row.actor_principal_id, $row.authenticated_session_id,
        $row.access_space_id, $row.workspace_id], '\u0000'));
    IF record::id($row.id) != $key { RETURN false; };
    LET $ledger = $row.event_ledger_event_id;
    IF $ledger = NONE OR $row.event_id != record::id($ledger)
        OR $ledger.event_id != $row.event_id OR $row.event_seq != $ledger.event_sequence
        OR $row.transaction_seq != $row.event_seq OR $row.event_stream_version != 1
        OR $ledger.source_component != 'model_lane'
        OR !($ledger.event_type = 'MODEL_LANE_AUTHORITY_RECORDED' OR ($row.record_kind = 'message' AND $ledger.event_type = 'MODEL_LANE_MESSAGE_RECORDED'))
        OR $ledger.aggregate_type != $row.record_kind OR $ledger.aggregate_id != $row.aggregate_id
        OR $ledger.kernel_task_run_id != $row.run_id OR $ledger.payload.record_kind != $row.record_kind
        OR $ledger.payload.run_id != $row.run_id OR $ledger.payload.event_stream_version != 1
        OR $ledger.owner_account_id != $row.owner_account_id OR $ledger.actor_principal_id != $row.actor_principal_id
        OR $ledger.authenticated_session_id != $row.authenticated_session_id OR $ledger.access_space_id != $row.access_space_id
        OR $ledger.workspace_id != $row.workspace_id
        OR crypto::sha256($ledger.payload.event_payload_json) != $ledger.payload_hash { RETURN false; };
    LET $payload = encoding::json::decode($ledger.payload.event_payload_json);
    LET $record = encoding::json::decode($row.record_json);
    LET $body = IF $row.record_kind = 'message' { $record.inner } ELSE { $record };
    RETURN $payload.dexterity_kernel = 'Dexterity' AND $payload.owner_account_id = $row.owner_account_id
        AND $payload.actor_principal_id = $row.actor_principal_id
        AND $payload.authenticated_session_id = $row.authenticated_session_id
        AND $payload.access_space_id = $row.access_space_id AND $payload.workspace_id = $row.workspace_id
        AND $payload.record = $record AND $body.run_id = $row.run_id
        AND type::is_string($body.event_ledger_stream_id) AND string::trim($body.event_ledger_stream_id) != ''
        AND $ledger.session_run_id = $body.event_ledger_stream_id;
};
DEFINE TABLE OVERWRITE model_lane_authority SCHEMAFULL
    PERMISSIONS FOR select WHERE fn::wp012_model_lane_source_access($this, false) AND fn::wp012_model_lane_source_identity($this)
    FOR create WHERE $wp012_original_action = 'create' AND fn::wp012_model_lane_source_access($this, true) AND fn::wp012_model_lane_source_identity($this)
    FOR update, delete NONE;

-- This producer branch preserves the original Document Update grant; it is not general content Read.
DEFINE FUNCTION OVERWRITE fn::wp012_model_lane_crdt_dependency_access($row: object) {
    IF !fn::wp012_model_lane_original_operation($row) OR $wp012_original_subject_kind != 'workspace'
        OR !($wp012_original_purpose IN ['handoff', 'consume', 'guarded_message', 'source_read']) { RETURN false; };
    IF $wp012_original_purpose = 'guarded_message' {
        RETURN fn::wp012_context_coordination_fence($row, 'workspace', $row.workspace_id)
            AND fn::wp012_model_lane_resource_domain($row.event_ledger_event_id.authority_resource_id,
                $row.workspace_id, 'rich_document', $row.document_id)
                = type::record('protected_resources', $wp012_original_resource);
    };
    LET $sources = SELECT * FROM model_lane_authority WHERE record_kind = IF $wp012_original_purpose = 'consume' { 'context_handoff' } ELSE { 'message' }
        AND owner_account_id = $row.owner_account_id AND actor_principal_id = $row.actor_principal_id
        AND authenticated_session_id = $row.authenticated_session_id AND access_space_id = $row.access_space_id
        AND workspace_id = $row.workspace_id;
    RETURN array::len(array::filter($sources, |$source| {
        LET $record = encoding::json::decode($source.record_json);
        LET $binding = IF $wp012_original_purpose = 'consume' { $record.crdt_payload } ELSE { $record.crdt_authority_binding };
        RETURN $binding != NONE AND $binding.document_id = $row.document_id
            AND ($wp012_original_purpose = 'consume' OR $binding.crdt_document_id = $row.crdt_document_id)
            AND fn::wp012_model_lane_source_identity($source)
            AND (($wp012_original_purpose = 'source_read' AND $wp012_original_action = 'read')
                OR (fn::wp012_context_coordination_fence($source, 'run', $source.run_id)
                    AND fn::wp012_context_coordination_fence($source, 'lane', IF $wp012_original_purpose = 'consume' { $record.downstream_lane_id } ELSE { $record.inner.from_lane_id })));
    })) > 0;
};
DEFINE FUNCTION OVERWRITE fn::wp012_model_lane_crdt_document_access($row: object, $write: bool, $snapshot: bool) {
    LET $dependency = !$write AND fn::wp012_model_lane_crdt_dependency_access($row);
    IF !fn::wp012_model_lane_original_operation($row)
        OR (!$dependency AND ($wp012_original_subject_kind != 'rich_document'
            OR $wp012_original_subject_id != $row.document_id OR $wp012_original_action != 'update'))
        OR $row.storage_authority != 'embedded_surrealdb'
        OR !type::is_string($row.crdt_document_id) OR string::trim($row.crdt_document_id) = '' { RETURN false; };
    IF $write {
        IF ($snapshot AND $wp012_original_purpose != 'crdt_snapshot')
            OR (!$snapshot AND $wp012_original_purpose != 'crdt_update') { RETURN false; };
    } ELSE IF !$dependency AND !($wp012_original_purpose IN ['crdt_update', 'crdt_snapshot', 'document_claim', 'lease_renew', 'lease_release', 'lease_expiry', 'proposal_record', 'proposal_decide', 'proposal_apply']) { RETURN false; };
    LET $kind = IF $snapshot { 'crdt_snapshot' } ELSE { 'crdt_update' };
    LET $identity = IF $snapshot { $row.snapshot_id } ELSE { $row.update_id };
    LET $key = crypto::sha256(string::join(['handshake.model-lane.authority.v1', $kind, $identity,
        $row.owner_account_id, $row.actor_principal_id, $row.authenticated_session_id,
        $row.access_space_id, $row.workspace_id], '\u0000'));
    IF record::id($row.id) != $key { RETURN false; };
    LET $ledger = $row.event_ledger_event_id;
    RETURN $ledger != NONE AND $ledger.source_component = 'model_lane_crdt'
        AND $ledger.aggregate_type = 'knowledge_crdt_document' AND $ledger.aggregate_id = $row.crdt_document_id
        AND $ledger.event_type = IF $snapshot { 'KNOWLEDGE_CRDT_SNAPSHOT_RECORDED' } ELSE { 'KNOWLEDGE_CRDT_UPDATE_RECORDED' }
        AND $ledger.owner_account_id = $row.owner_account_id AND $ledger.actor_principal_id = $row.actor_principal_id
        AND $ledger.authenticated_session_id = $row.authenticated_session_id
        AND $ledger.access_space_id = $row.access_space_id AND $ledger.workspace_id = $row.workspace_id
        AND $ledger.payload.document_id = $row.document_id AND $ledger.actor_id = $row.actor_id
        AND $ledger.actor_kind = $row.actor_kind
        AND $ledger.payload.crdt_document_id = $row.crdt_document_id
        AND $row.event_ledger_stream_id = string::concat('knowledge-crdt:', $row.crdt_document_id)
        AND (($snapshot AND $ledger.payload.snapshot_id = $row.snapshot_id
                AND $ledger.payload.snapshot_sha256 = $row.snapshot_sha256
                AND $ledger.payload.covered_update_seq = $row.covered_update_seq)
            OR (!$snapshot AND $ledger.payload.update_id = $row.update_id
                AND $ledger.payload.update_sha256 = $row.update_sha256 AND $ledger.payload.update_seq = $row.update_seq));
};
DEFINE TABLE OVERWRITE kernel_crdt_updates SCHEMAFULL
    PERMISSIONS FOR select WHERE fn::mt120_document_access(document_id, workspace_id, 'read', 'fs.read') OR fn::wp012_model_lane_crdt_document_access($this, false, false)
    FOR create WHERE (fn::mt120_document_access(document_id, workspace_id, 'update', 'fs.write') AND event_ledger_event_id.authority_session_id = $auth.id AND event_ledger_event_id.wsids = [workspace_id]) OR fn::wp012_model_lane_crdt_document_access($this, true, false)
    FOR update NONE FOR delete WHERE fn::mt109_has_workspace_access(workspace_id, 'delete', 'fs.write') AND owner_account_id = NONE;
DEFINE TABLE OVERWRITE kernel_crdt_snapshots SCHEMAFULL
    PERMISSIONS FOR select WHERE fn::wp012_model_lane_crdt_document_access($this, false, true)
    FOR create WHERE fn::wp012_model_lane_crdt_document_access($this, true, true)
    FOR update, delete NONE;

DEFINE FUNCTION OVERWRITE fn::wp012_model_lane_crdt_lease_subject($row: object) {
    RETURN ($row.scope_kind = 'document' AND $wp012_original_subject_kind = 'rich_document'
            AND $row.document_id = $wp012_original_subject_id AND $row.scope_id = $row.crdt_document_id)
        OR ($row.scope_kind IN ['workspace', 'index_run'] AND $wp012_original_subject_kind = 'workspace'
            AND $wp012_original_subject_id = $row.workspace_id
            AND ($row.scope_kind != 'workspace' OR $row.scope_id = $row.workspace_id));
};
DEFINE FUNCTION OVERWRITE fn::wp012_model_lane_crdt_lease_identity($row: object) {
    LET $key = crypto::sha256(string::join(['handshake.model-lane.authority.v1', 'crdt_lease', $row.lease_id,
        $row.owner_account_id, $row.actor_principal_id, $row.authenticated_session_id,
        $row.access_space_id, $row.workspace_id], '\u0000'));
    LET $first = $row.recorded_event_id;
    LET $last = $row.last_transition_event_id;
    RETURN record::id($row.id) = $key AND $first != NONE AND $last != NONE
        AND $first.source_component = 'model_lane_crdt' AND $last.source_component = 'model_lane_crdt'
        AND $first.aggregate_type = 'knowledge_crdt_lease' AND $last.aggregate_type = 'knowledge_crdt_lease'
        AND $first.aggregate_id = $row.lease_id AND $last.aggregate_id = $row.lease_id
        AND $first.event_type = 'KNOWLEDGE_CRDT_LEASE_CLAIMED'
        AND $last.event_type IN ['KNOWLEDGE_CRDT_LEASE_CLAIMED', 'KNOWLEDGE_CRDT_LEASE_RENEWED', 'KNOWLEDGE_CRDT_LEASE_RELEASED', 'KNOWLEDGE_CRDT_LEASE_EXPIRED']
        AND $first.owner_account_id = $row.owner_account_id AND $last.owner_account_id = $row.owner_account_id
        AND $first.actor_principal_id = $row.actor_principal_id AND $last.actor_principal_id = $row.actor_principal_id
        AND $first.authenticated_session_id = $row.authenticated_session_id AND $last.authenticated_session_id = $row.authenticated_session_id
        AND $first.access_space_id = $row.access_space_id AND $last.access_space_id = $row.access_space_id
        AND $first.workspace_id = $row.workspace_id AND $last.workspace_id = $row.workspace_id
        AND $first.actor_id = $row.actor_id AND $last.actor_id = $row.actor_id
        AND $first.actor_kind = $row.actor_kind AND $last.actor_kind = $row.actor_kind
        AND $first.correlation_id = $row.correlation_id AND $last.correlation_id = $row.correlation_id
        AND $first.payload.lease_id = $row.lease_id AND $last.payload.lease_id = $row.lease_id
        AND $first.payload.lane_id = $row.lane_id AND $first.payload.actor_id = $row.actor_id
        AND $first.payload.session_id = $row.session_id
        AND $first.payload.scope_kind = $row.scope_kind AND $first.payload.scope_id = $row.scope_id
        AND ($row.scope_kind != 'document' OR ($first.payload.document_id = $row.document_id
            AND $first.payload.crdt_document_id = $row.crdt_document_id))
        AND $first.authority_resource_id = $row.authority_resource_id
        AND $first.authority_grant_id = $row.authority_grant_id
        AND $first.authority_action = $row.authority_action
        AND $first.authority_capability_id = $row.authority_capability_id
        AND $first.authority_session_id = type::record('authenticated_sessions', $row.authenticated_session_id)
        AND $row.authority_grant_id.resource_id = $row.authority_resource_id
        AND $row.authority_grant_id.account_id = type::record('local_accounts', $row.owner_account_id)
        AND $row.authority_grant_id.principal_id = type::record('principals', $row.actor_principal_id)
        AND $row.authority_grant_id.access_space_id = type::record('access_spaces', $row.access_space_id);
};
DEFINE FUNCTION OVERWRITE fn::wp012_model_lane_crdt_lease_access($row: object, $write: bool) {
    IF !fn::wp012_model_lane_original_operation($row) { RETURN false; };
    IF !$write AND $wp012_original_purpose IN ['consume', 'handoff', 'guarded_message', 'document_claim', 'workspace_claim', 'index_run_claim', 'crdt_update', 'crdt_snapshot', 'proposal_record', 'proposal_decide', 'proposal_apply'] {
        RETURN ($row.scope_kind = 'workspace' AND $row.scope_id = $row.workspace_id)
            OR ($row.scope_kind = 'document' AND $row.document_id != NONE AND $row.crdt_document_id = $row.scope_id
                AND ($wp012_original_subject_kind = 'workspace' OR $row.document_id = $wp012_original_subject_id))
            OR ($row.scope_kind = 'index_run' AND $wp012_original_purpose = 'index_run_claim');
    };
    RETURN fn::wp012_model_lane_crdt_lease_subject($row)
        AND fn::wp012_context_coordination_fence($row, 'workspace', $row.workspace_id)
        AND (($row.scope_kind = 'document' AND $wp012_original_purpose = 'document_claim')
            OR ($row.scope_kind = 'workspace' AND $wp012_original_purpose = 'workspace_claim')
            OR ($row.scope_kind = 'index_run' AND $wp012_original_purpose = 'index_run_claim')
            OR ($wp012_original_purpose IN ['lease_renew', 'lease_release', 'lease_expiry'] AND $wp012_original_action = 'update'));
};
DEFINE TABLE OVERWRITE knowledge_crdt_agent_lane_leases SCHEMAFULL
    PERMISSIONS FOR select WHERE fn::wp012_model_lane_crdt_lease_access($this, false) AND fn::wp012_model_lane_crdt_lease_identity($this)
    FOR create WHERE fn::wp012_model_lane_crdt_lease_access($this, true) AND fn::wp012_model_lane_crdt_lease_identity($this)
        AND transition_revision = 1 AND renewal_count = 0 AND released_at_utc = NONE AND expired_at_utc = NONE
        AND authority_resource_id = type::record('protected_resources', $wp012_original_resource)
        AND authority_grant_id = type::record('resource_grants', $wp012_original_grant)
        AND authority_action = $wp012_original_action AND authority_capability_id = $wp012_original_capability
        AND authority_subject_kind = $wp012_original_subject_kind AND authority_subject_id = $wp012_original_subject_id
    FOR update WHERE fn::wp012_model_lane_crdt_lease_access($this, true) AND fn::wp012_model_lane_crdt_lease_identity($this)
    FOR delete NONE;
DEFINE FIELD OVERWRITE lease_id ON knowledge_crdt_agent_lane_leases TYPE string ASSERT IF $this.owner_account_id = NONE AND $this.actor_principal_id = NONE AND $this.authenticated_session_id = NONE AND $this.access_space_id = NONE AND $this.workspace_id = NONE { $value = record::id($this.id) } ELSE { $this.owner_account_id != NONE AND $this.actor_principal_id != NONE AND $this.authenticated_session_id != NONE AND $this.access_space_id != NONE AND $this.workspace_id != NONE AND record::id($this.id) = crypto::sha256(string::join(['handshake.model-lane.authority.v1', 'crdt_lease', $value, $this.owner_account_id, $this.actor_principal_id, $this.authenticated_session_id, $this.access_space_id, $this.workspace_id], '\u0000')) };
DEFINE FIELD OVERWRITE authority_resource_id ON knowledge_crdt_agent_lane_leases TYPE option<record<protected_resources>>;
DEFINE FIELD OVERWRITE authority_grant_id ON knowledge_crdt_agent_lane_leases TYPE option<record<resource_grants>>;
DEFINE FIELD OVERWRITE authority_capability_id ON knowledge_crdt_agent_lane_leases TYPE option<string>;
DEFINE FIELD OVERWRITE authority_action ON knowledge_crdt_agent_lane_leases TYPE option<'create' | 'update'>;
DEFINE FIELD OVERWRITE authority_subject_kind ON knowledge_crdt_agent_lane_leases TYPE option<'workspace' | 'rich_document'>;
DEFINE FIELD OVERWRITE authority_subject_id ON knowledge_crdt_agent_lane_leases TYPE option<string>;
DEFINE FIELD OVERWRITE transition_revision ON knowledge_crdt_agent_lane_leases TYPE option<int> ASSERT $value = NONE OR $value > 0;

DEFINE EVENT OVERWRITE wp012_model_lane_crdt_lease_transition ON knowledge_crdt_agent_lane_leases
WHEN $event = 'UPDATE'
THEN {
    IF $before.owner_account_id = NONE AND $before.actor_principal_id = NONE AND $before.authenticated_session_id = NONE AND $before.access_space_id = NONE AND $before.workspace_id = NONE AND $after.owner_account_id = NONE AND $after.actor_principal_id = NONE AND $after.authenticated_session_id = NONE AND $after.access_space_id = NONE AND $after.workspace_id = NONE AND $before.authority_resource_id = NONE AND $after.authority_resource_id = NONE AND $before.authority_grant_id = NONE AND $after.authority_grant_id = NONE AND $before.transition_revision = NONE AND $after.transition_revision = NONE { RETURN NONE; };
    IF !fn::wp012_model_lane_crdt_lease_access($before, true)
        OR !fn::wp012_model_lane_crdt_lease_access($after, true)
        OR !fn::wp012_model_lane_crdt_lease_identity($before)
        OR !fn::wp012_model_lane_crdt_lease_identity($after)
        OR $before.released_at_utc != NONE OR $before.expired_at_utc != NONE
        OR $after.transition_revision != $before.transition_revision + 1
        OR $before.last_transition_event_id = $after.last_transition_event_id
        OR $before.id != $after.id OR $before.lease_id != $after.lease_id
        OR $before.owner_account_id != $after.owner_account_id OR $before.actor_principal_id != $after.actor_principal_id
        OR $before.authenticated_session_id != $after.authenticated_session_id OR $before.access_space_id != $after.access_space_id
        OR $before.workspace_id != $after.workspace_id OR $before.document_id != $after.document_id
        OR $before.crdt_document_id != $after.crdt_document_id OR $before.lane_id != $after.lane_id
        OR $before.actor_id != $after.actor_id OR $before.actor_kind != $after.actor_kind
        OR $before.session_id != $after.session_id OR $before.correlation_id != $after.correlation_id
        OR $before.scope_kind != $after.scope_kind OR $before.scope_id != $after.scope_id
        OR $before.claimed_at_utc != $after.claimed_at_utc OR $before.takeover_of != $after.takeover_of
        OR $before.recorded_event_id != $after.recorded_event_id
        OR $before.authority_resource_id != $after.authority_resource_id
        OR $before.authority_grant_id != $after.authority_grant_id
        OR $before.authority_capability_id != $after.authority_capability_id
        OR $before.authority_action != $after.authority_action
        OR $before.authority_subject_kind != $after.authority_subject_kind
        OR $before.authority_subject_id != $after.authority_subject_id
        OR $before.authority_resource_id != type::record('protected_resources', $wp012_original_resource)
        OR $before.authority_grant_id != type::record('resource_grants', $wp012_original_grant)
        OR $before.authority_subject_kind != $wp012_original_subject_kind
        OR $before.authority_subject_id != $wp012_original_subject_id
        OR !fn::wp012_model_lane_original_grant($before.authority_resource_id, $before.authority_grant_id, $before.authority_capability_id, $before.authority_action) {
        THROW 'HSK-403-MODEL-LANE-CRDT-LEASE-TRANSITION';
    };
    LET $receipt = $after.last_transition_event_id;
    IF $receipt.authority_resource_id != type::record('protected_resources', $wp012_original_resource)
        OR $receipt.authority_session_id != $auth.id OR $receipt.authority_action != $wp012_original_action
        OR $receipt.authority_capability_id != $wp012_original_capability {
        THROW 'HSK-403-MODEL-LANE-CRDT-LEASE-TRANSITION';
    };
    IF $wp012_original_purpose = 'lease_renew' {
        IF $receipt.event_type != 'KNOWLEDGE_CRDT_LEASE_RENEWED' OR $before.expires_at_utc <= time::now()
            OR $after.renewal_count != $before.renewal_count + 1 OR $after.expires_at_utc <= time::now()
            OR $after.expires_at_utc != $transition_at + duration::from_secs($ttl_seconds)
            OR $after.released_at_utc != NONE OR $after.expired_at_utc != NONE {
            THROW 'HSK-403-MODEL-LANE-CRDT-LEASE-TRANSITION';
        };
    } ELSE IF $wp012_original_purpose = 'lease_release' {
        IF $receipt.event_type != 'KNOWLEDGE_CRDT_LEASE_RELEASED' OR $after.released_at_utc = NONE
            OR $after.released_at_utc != $transition_at OR $after.released_at_utc > time::now() OR $after.expired_at_utc != NONE
            OR $after.expires_at_utc != $before.expires_at_utc OR $after.renewal_count != $before.renewal_count {
            THROW 'HSK-403-MODEL-LANE-CRDT-LEASE-TRANSITION';
        };
    } ELSE IF $wp012_original_purpose = 'lease_expiry' {
        IF $receipt.event_type != 'KNOWLEDGE_CRDT_LEASE_EXPIRED' OR $before.expires_at_utc > time::now()
            OR $after.expired_at_utc = NONE OR $after.expired_at_utc != $transition_at OR $after.expired_at_utc > time::now() OR $after.released_at_utc != NONE
            OR $after.expires_at_utc != $before.expires_at_utc OR $after.renewal_count != $before.renewal_count {
            THROW 'HSK-403-MODEL-LANE-CRDT-LEASE-TRANSITION';
        };
    } ELSE { THROW 'HSK-403-MODEL-LANE-CRDT-LEASE-TRANSITION'; };
};

DEFINE FIELD OVERWRITE active_scope_key ON knowledge_crdt_agent_lane_leases TYPE string
    VALUE IF $this.owner_account_id = NONE AND $this.actor_principal_id = NONE AND $this.authenticated_session_id = NONE AND $this.access_space_id = NONE AND $this.workspace_id = NONE {
        IF $this.released_at_utc = NONE { 'active:' + type::string($this.scope_kind) + ':' + $this.scope_id } ELSE { 'released:' + $this.lease_id }
    } ELSE IF $this.released_at_utc = NONE AND $this.expired_at_utc = NONE {
        crypto::sha256(string::join(['handshake.model-lane.crdt-lease.active.v1', $this.owner_account_id,
            $this.actor_principal_id, $this.authenticated_session_id, $this.access_space_id, $this.workspace_id,
            $this.scope_kind, $this.scope_id], '\u0000'))
    } ELSE {
        crypto::sha256(string::join(['handshake.model-lane.crdt-lease.closed.v1', $this.owner_account_id,
            $this.actor_principal_id, $this.authenticated_session_id, $this.access_space_id, $this.workspace_id,
            $this.lease_id], '\u0000'))
    };
DEFINE INDEX OVERWRITE pk_knowledge_crdt_agent_lane_leases ON knowledge_crdt_agent_lane_leases
    FIELDS owner_account_id, actor_principal_id, authenticated_session_id, access_space_id, workspace_id, lease_id UNIQUE;

-- Proposal editing retains the original document Update operation and immutable model provenance.
DEFINE FUNCTION OVERWRITE fn::wp012_model_lane_proposal_access($row: object, $write: bool) {
    IF !fn::wp012_model_lane_original_operation($row) { RETURN false; };
    IF $wp012_original_subject_kind = 'rich_document' AND $wp012_original_subject_id = $row.document_id
        AND $wp012_original_action = 'update' AND $wp012_original_capability = 'fs.write'
        AND $wp012_original_purpose IN ['proposal_record', 'proposal_decide', 'proposal_apply'] {
        RETURN !$write OR fn::wp012_context_coordination_fence($row, 'workspace', $row.workspace_id);
    };
    RETURN !$write AND fn::wp012_model_lane_crdt_dependency_access(object::extend($row, {
        event_ledger_event_id: $row.recorded_event_id
    }));
};
DEFINE FUNCTION OVERWRITE fn::wp012_model_lane_promotion_identity($row: object) {
    LET $requested = $row.promotion_requested_event_id;
    LET $accepted = $row.promotion_accepted_event_id;
    IF $requested = NONE OR $accepted = NONE OR $requested = $accepted
        OR $row.applied_event_id = NONE OR $row.applied_update_id = NONE
        OR $row.last_transition_event_id != $accepted OR $requested.causation_id != NONE
        OR $accepted.causation_id != record::id($requested)
        OR $requested.event_type != 'PROMOTION_REQUESTED' OR $accepted.event_type != 'PROMOTION_ACCEPTED'
        OR $requested.event_sequence <= $row.applied_event_id.event_sequence
        OR $accepted.event_sequence <= $requested.event_sequence
        OR $accepted.actor_id != $requested.actor_id OR $accepted.session_run_id != $requested.session_run_id
        OR $accepted.kernel_task_run_id != $requested.kernel_task_run_id
        OR $accepted.payload.review_state != 'promoted'
        OR $accepted.payload.applied_update_id != $row.applied_update_id
        OR $accepted.payload.applied_update_sha256 != $row.diff_sha256 { RETURN false; };
    RETURN array::len(array::filter([$requested, $accepted], |$receipt|
        $receipt.source_component != 'knowledge_crdt_ai_edit_proposal'
        OR $receipt.aggregate_type != 'knowledge_ai_edit_promotion' OR $receipt.aggregate_id != $row.proposal_id
        OR $receipt.event_version != 'kernel_event_v1' OR $receipt.event_id != record::id($receipt.id)
        OR $receipt.owner_account_id != $row.owner_account_id OR $receipt.actor_principal_id != $row.actor_principal_id
        OR $receipt.authenticated_session_id != $row.authenticated_session_id OR $receipt.access_space_id != $row.access_space_id
        OR $receipt.workspace_id != $row.workspace_id OR $receipt.wsids != [$row.workspace_id]
        OR $receipt.actor_kind != 'operator' OR $receipt.session_run_id != $receipt.authenticated_session_id
        OR $receipt.correlation_id != $row.correlation_id
        OR $receipt.payload.proposal_id != $row.proposal_id OR $receipt.payload.document_id != $row.document_id
        OR $receipt.payload.crdt_document_id != $row.crdt_document_id OR $receipt.payload.diff_sha256 != $row.diff_sha256
        OR $receipt.payload.recorded_event_id != record::id($row.recorded_event_id)
        OR $receipt.payload.decided_event_id != record::id($row.decided_event_id)
        OR $receipt.payload.applied_event_id != record::id($row.applied_event_id)
        OR $receipt.payload.application_update_id != $row.applied_update_id
        OR $receipt.payload.decided_by != $receipt.actor_id
        OR $receipt.payload.reviewer_principal_id != $receipt.actor_principal_id
        OR $receipt.payload.reviewer_authenticated_session_id != $receipt.authenticated_session_id
        OR $receipt.payload.reviewer_resource_id != record::id($receipt.authority_resource_id)
        OR $receipt.payload.reviewer_grant_id != record::id($receipt.authority_grant_id)
        OR $receipt.payload.reviewer_action != 'update' OR $receipt.authority_action != 'update'
        OR $receipt.payload.reviewer_capability_id != 'fs.write' OR $receipt.authority_capability_id != 'fs.write'
        OR $receipt.authority_session_id != type::record('authenticated_sessions', $receipt.authenticated_session_id)
        OR crypto::sha256(encoding::json::encode($receipt.payload)) != $receipt.payload_hash
    )) = 0 AND $requested.payload.base_update_seq = $row.base_update_seq
        AND $requested.payload.base_state_vector = $row.base_state_vector;
};
DEFINE FUNCTION OVERWRITE fn::wp012_model_lane_proposal_identity($row: object) {
    LET $key = crypto::sha256(string::join(['handshake.model-lane.authority.v1', 'crdt_proposal', $row.proposal_id,
        $row.owner_account_id, $row.actor_principal_id, $row.authenticated_session_id,
        $row.access_space_id, $row.workspace_id], '\u0000'));
    LET $first = $row.recorded_event_id;
    IF record::id($row.id) != $key OR $first = NONE
        OR $first.source_component != 'model_lane_crdt' OR $first.aggregate_type != 'knowledge_crdt_ai_edit_proposal'
        OR $first.aggregate_id != $row.proposal_id OR $first.event_type != 'AI_EDIT_PROPOSAL_RECORDED'
        OR $first.owner_account_id != $row.owner_account_id OR $first.actor_principal_id != $row.actor_principal_id
        OR $first.authenticated_session_id != $row.authenticated_session_id OR $first.access_space_id != $row.access_space_id
        OR $first.workspace_id != $row.workspace_id OR $first.actor_id != $row.actor_id OR $first.actor_kind != $row.actor_kind
        OR $first.session_run_id != $row.session_id OR $first.correlation_id != $row.correlation_id
        OR $first.payload.proposal_id != $row.proposal_id OR $first.payload.document_id != $row.document_id
        OR $first.payload.crdt_document_id != $row.crdt_document_id OR $first.payload.base_update_seq != $row.base_update_seq
        OR $first.payload.base_state_vector != $row.base_state_vector OR $first.payload.proposed_diff != $row.proposed_diff
        OR $first.payload.diff_sha256 != $row.diff_sha256 OR $first.payload.source_span_citations != $row.source_span_citations
        OR $first.payload.lease_id != $row.lease_id.lease_id
        OR crypto::sha256(encoding::json::encode($first.payload)) != $first.payload_hash
        OR !fn::wp012_model_lane_crdt_lease_identity($row.lease_id) { RETURN false; };
    IF $row.review_state = 'proposed' {
        RETURN $row.decided_event_id = NONE AND $row.applied_event_id = NONE
            AND $row.promotion_requested_event_id = NONE AND $row.promotion_accepted_event_id = NONE
            AND $row.last_transition_event_id = $first.id AND $row.decided_by = NONE;
    };
    LET $decision = $row.decided_event_id;
    IF $row.review_state NOT IN ['approved', 'rejected', 'promoted'] OR $decision = NONE
        OR $decision.source_component != 'model_lane_crdt' OR $decision.aggregate_type != 'knowledge_crdt_ai_edit_proposal'
        OR $decision.aggregate_id != $row.proposal_id OR $decision.event_type != 'AI_EDIT_PROPOSAL_DECIDED'
        OR $decision.actor_kind != 'operator' OR $decision.actor_id != $row.decided_by
        OR $decision.payload.review_state != IF $row.review_state = 'promoted' { 'approved' } ELSE { $row.review_state } OR $decision.payload.recorded_event_id != record::id($first)
        OR $decision.payload.diff_sha256 != $row.diff_sha256
        OR $decision.payload.reviewer_principal_id != $decision.actor_principal_id
        OR $decision.payload.reviewer_authenticated_session_id != $decision.authenticated_session_id
        OR $decision.payload.reviewer_resource_id != record::id($decision.authority_resource_id)
        OR $decision.payload.reviewer_grant_id != record::id($decision.authority_grant_id)
        OR $decision.payload.reviewer_action != $decision.authority_action
        OR $decision.payload.reviewer_capability_id != $decision.authority_capability_id
        OR $decision.owner_account_id != $row.owner_account_id OR $decision.actor_principal_id != $row.actor_principal_id
        OR $decision.authenticated_session_id != $row.authenticated_session_id OR $decision.access_space_id != $row.access_space_id
        OR $decision.workspace_id != $row.workspace_id OR $decision.session_run_id != $decision.authenticated_session_id
        OR crypto::sha256(encoding::json::encode($decision.payload)) != $decision.payload_hash { RETURN false; };
    IF $row.applied_event_id = NONE {
        RETURN $row.review_state != 'promoted' AND $row.promotion_requested_event_id = NONE AND $row.promotion_accepted_event_id = NONE
            AND $row.applied_update_id = NONE AND $row.applied_update_sha256 = NONE
            AND $row.last_transition_event_id = $decision;
    };
    LET $apply = $row.applied_event_id;
    RETURN $row.review_state IN ['approved', 'promoted'] AND $row.applied_update_sha256 = $row.diff_sha256
        AND $apply.source_component = 'model_lane_crdt' AND $apply.aggregate_type = 'knowledge_crdt_ai_edit_proposal'
        AND $apply.aggregate_id = $row.proposal_id AND $apply.event_type = 'AI_EDIT_PROPOSAL_DECIDED'
        AND $apply.actor_id = $row.actor_id AND $apply.actor_kind = $row.actor_kind
        AND $apply.owner_account_id = $row.owner_account_id AND $apply.actor_principal_id = $row.actor_principal_id
        AND $apply.authenticated_session_id = $row.authenticated_session_id AND $apply.access_space_id = $row.access_space_id
        AND $apply.workspace_id = $row.workspace_id AND $apply.session_run_id = $row.session_id
        AND $apply.payload.applied_update_id = $row.applied_update_id
        AND $apply.payload.applied_update_sha256 = $row.diff_sha256 AND $apply.payload.approved_diff_sha256 = $row.diff_sha256
        AND $apply.payload.recorded_event_id = record::id($first)
        AND $apply.payload.decided_event_id = record::id($decision)
        AND $apply.payload.base_update_seq = $row.base_update_seq
        AND $apply.payload.base_state_vector = $row.base_state_vector
        AND (($row.review_state = 'approved' AND $row.last_transition_event_id = $apply
                AND $row.promotion_requested_event_id = NONE AND $row.promotion_accepted_event_id = NONE)
            OR ($row.review_state = 'promoted' AND fn::wp012_model_lane_promotion_identity($row)))
        AND crypto::sha256(encoding::json::encode($apply.payload)) = $apply.payload_hash;
};
DEFINE TABLE OVERWRITE knowledge_crdt_ai_edit_proposals SCHEMAFULL
    PERMISSIONS FOR select WHERE fn::wp012_model_lane_proposal_access($this, false) AND fn::wp012_model_lane_proposal_identity($this)
    FOR create WHERE $wp012_original_purpose = 'proposal_record'
        AND fn::wp012_model_lane_proposal_access($this, true) AND fn::wp012_model_lane_proposal_identity($this)
        AND review_state = 'proposed' AND decided_event_id = NONE AND applied_event_id = NONE
    FOR update WHERE $wp012_original_purpose IN ['proposal_decide', 'proposal_apply']
        AND fn::wp012_model_lane_proposal_access($this, true) AND fn::wp012_model_lane_proposal_identity($this)
    FOR delete NONE;
DEFINE FUNCTION OVERWRITE fn::wp012_model_lane_application_evidence($proposal: object, $update: object, $receipt: object) {
    LET $proof = $receipt.payload.application_evidence;
    LET $bases = SELECT * FROM kernel_crdt_updates WHERE update_id = $proof.base_update_id
        AND owner_account_id = $proposal.owner_account_id AND actor_principal_id = $proposal.actor_principal_id
        AND authenticated_session_id = $proposal.authenticated_session_id AND access_space_id = $proposal.access_space_id
        AND workspace_id = $proposal.workspace_id AND document_id = $proposal.document_id
        AND crdt_document_id = $proposal.crdt_document_id AND update_seq = $proposal.base_update_seq;
    LET $snapshots = SELECT * FROM kernel_crdt_snapshots WHERE snapshot_id = $proof.base_snapshot_id
        AND owner_account_id = $proposal.owner_account_id AND actor_principal_id = $proposal.actor_principal_id
        AND authenticated_session_id = $proposal.authenticated_session_id AND access_space_id = $proposal.access_space_id
        AND workspace_id = $proposal.workspace_id AND document_id = $proposal.document_id
        AND crdt_document_id = $proposal.crdt_document_id AND covered_update_seq = $proposal.base_update_seq;
    RETURN array::len($bases) = 1 AND array::len($snapshots) = 1
        AND $bases[0].state_vector_after = $proposal.base_state_vector
        AND $snapshots[0].state_vector = $proposal.base_state_vector
        AND $proof.base_update_event_id = record::id($bases[0].event_ledger_event_id)
        AND $proof.base_snapshot_event_id = record::id($snapshots[0].event_ledger_event_id)
        AND $bases[0].event_ledger_event_id.owner_account_id = $proposal.owner_account_id
        AND $bases[0].event_ledger_event_id.actor_principal_id = $proposal.actor_principal_id
        AND $bases[0].event_ledger_event_id.authenticated_session_id = $proposal.authenticated_session_id
        AND $bases[0].event_ledger_event_id.access_space_id = $proposal.access_space_id
        AND $bases[0].event_ledger_event_id.workspace_id = $proposal.workspace_id
        AND $snapshots[0].event_ledger_event_id.owner_account_id = $proposal.owner_account_id
        AND $snapshots[0].event_ledger_event_id.actor_principal_id = $proposal.actor_principal_id
        AND $snapshots[0].event_ledger_event_id.authenticated_session_id = $proposal.authenticated_session_id
        AND $snapshots[0].event_ledger_event_id.access_space_id = $proposal.access_space_id
        AND $snapshots[0].event_ledger_event_id.workspace_id = $proposal.workspace_id
        AND type::is_object($proof) AND $proof.schema_id = 'hsk.model_lane_proposal_application@1'
        AND $proof.operation = 'append_text' AND $proposal.proposed_diff.op = 'append_text'
        AND array::len($proposal.proposed_diff.path) = 1 AND $proof.path = $proposal.proposed_diff.path
        AND type::is_string($proof.path[0]) AND string::trim($proof.path[0]) != ''
        AND $proof.proposal_id = $proposal.proposal_id AND $proof.document_id = $proposal.document_id
        AND $proof.crdt_document_id = $proposal.crdt_document_id AND $proof.base_update_seq = $proposal.base_update_seq
        AND $proof.recorded_event_id = record::id($proposal.recorded_event_id)
        AND $proof.approval_event_id = record::id($proposal.decided_event_id)
        AND $proof.update_id = $update.update_id AND $proof.update_seq = $update.update_seq
        AND $proof.update_event_id = record::id($update.event_ledger_event_id)
        AND $proof.approved_diff_sha256 = $proposal.diff_sha256 AND $proof.yjs_update_sha256 = $update.update_sha256
        AND $proof.approval_event_sequence = $proposal.decided_event_id.event_sequence
        AND $proof.update_event_sequence = $update.event_ledger_event_id.event_sequence
        AND $proof.approval_event_sequence < $proof.update_event_sequence
        AND $receipt.event_sequence > $proof.update_event_sequence
        AND type::is_string($proof.canonical_base_materialization_sha256)
        AND string::matches($proof.canonical_base_materialization_sha256, '^[0-9a-f]{64}$')
        AND type::is_string($proof.canonical_post_materialization_sha256)
        AND string::matches($proof.canonical_post_materialization_sha256, '^[0-9a-f]{64}$')
        AND type::is_string($proof.base_update_id) AND type::is_string($proof.base_snapshot_id)
        AND type::is_string($proof.base_update_event_id) AND type::is_string($proof.base_snapshot_event_id);
};

DEFINE EVENT OVERWRITE wp012_model_lane_proposal_transition ON knowledge_crdt_ai_edit_proposals WHEN $event = 'UPDATE' THEN {
    IF !fn::wp012_model_lane_proposal_access($before, true) OR !fn::wp012_model_lane_proposal_access($after, true)
        OR !fn::wp012_model_lane_proposal_identity($before) OR !fn::wp012_model_lane_proposal_identity($after) {
        THROW 'HSK-403-MODEL-LANE-PROPOSAL';
    };
    LET $promotion = $wp012_original_purpose = 'proposal_decide' AND $before.review_state = 'approved' AND $after.review_state = 'promoted';
    LET $mutable = IF $promotion {
        ['review_state', 'promotion_requested_event_id', 'promotion_accepted_event_id', 'last_transition_event_id']
    } ELSE IF $wp012_original_purpose = 'proposal_decide' {
        ['review_state', 'decided_by', 'decided_at_utc', 'decision_reason', 'decided_event_id', 'last_transition_event_id']
    } ELSE { ['applied_update_id', 'applied_update_sha256', 'applied_event_id', 'last_transition_event_id'] };
    LET $original = object::from_entries(array::filter(object::entries($before), |$entry| $entry[0] NOT IN $mutable));
    LET $next = object::from_entries(array::filter(object::entries($after), |$entry| $entry[0] NOT IN $mutable));
    IF $original != $next { THROW 'HSK-403-MODEL-LANE-PROPOSAL-IMMUTABLE'; };
    IF $promotion {
        LET $head = SELECT * FROM kernel_crdt_updates WHERE document_id = $before.document_id
            AND crdt_document_id = $before.crdt_document_id AND owner_account_id = $before.owner_account_id
            AND actor_principal_id = $before.actor_principal_id AND authenticated_session_id = $before.authenticated_session_id
            AND access_space_id = $before.access_space_id AND workspace_id = $before.workspace_id ORDER BY update_seq DESC LIMIT 1;
        IF $before.applied_event_id = NONE OR $before.promotion_requested_event_id != NONE OR $before.promotion_accepted_event_id != NONE
            OR $auth.principal_id.principal_kind NOT IN ['human_account', 'break_glass_operator']
            OR $auth.principal_id.status != 'enabled' OR $auth.principal_id.actor_kind != 'operator'
            OR $after.promotion_requested_event_id.actor_id != $auth.principal_id.actor_id
            OR $after.promotion_accepted_event_id.authority_session_id != $auth.id
            OR $after.promotion_accepted_event_id.authority_resource_id != type::record('protected_resources', $wp012_original_resource)
            OR $after.promotion_accepted_event_id.authority_grant_id != type::record('resource_grants', $wp012_original_grant)
            OR array::len($head) != 1 OR $head[0].update_id != $before.applied_update_id
            OR $head[0].update_seq != $before.base_update_seq + 1
            OR !fn::wp012_model_lane_application_evidence($before, $head[0], (SELECT * FROM ONLY $before.applied_event_id)) {
            THROW 'HSK-403-MODEL-LANE-PROPOSAL-PROMOTION';
        };
    } ELSE IF $wp012_original_purpose = 'proposal_decide' {
        IF $before.review_state != 'proposed' OR $after.review_state NOT IN ['approved', 'rejected']
            OR $before.decided_event_id != NONE OR $after.decided_event_id = NONE
            OR $auth.principal_id.principal_kind NOT IN ['human_account', 'break_glass_operator']
            OR $auth.principal_id.actor_kind != 'operator' OR $after.decided_by != $auth.principal_id.actor_id
            OR $after.decided_event_id.authority_session_id != $auth.id
            OR $after.decided_event_id.authority_resource_id != type::record('protected_resources', $wp012_original_resource)
            OR $after.decided_event_id.authority_grant_id != type::record('resource_grants', $wp012_original_grant)
            OR $after.decided_at_utc = NONE OR $after.decided_at_utc > time::now() {
            THROW 'HSK-403-MODEL-LANE-PROPOSAL-REVIEW';
        };
    } ELSE {
        IF $wp012_original_purpose != 'proposal_apply' OR $before.review_state != 'approved'
            OR $before.applied_update_id != NONE OR $after.applied_update_id = NONE
            OR $after.applied_update_sha256 != $before.diff_sha256 { THROW 'HSK-403-MODEL-LANE-PROPOSAL-APPLY'; };
        LET $updates = SELECT * FROM kernel_crdt_updates WHERE update_id = $after.applied_update_id
            AND owner_account_id = $before.owner_account_id AND actor_principal_id = $before.actor_principal_id
            AND authenticated_session_id = $before.authenticated_session_id AND access_space_id = $before.access_space_id
            AND workspace_id = $before.workspace_id AND document_id = $before.document_id AND crdt_document_id = $before.crdt_document_id;
        IF array::len($updates) != 1 OR !fn::wp012_model_lane_crdt_document_access($updates[0], false, false)
            OR $updates[0].actor_id != $before.actor_id OR $updates[0].actor_kind != $before.actor_kind
            OR $updates[0].session_id != $before.session_id OR $updates[0].trace_id != $before.correlation_id
            OR $updates[0].update_seq != $before.base_update_seq + 1 OR $updates[0].state_vector_before != $before.base_state_vector
            OR $after.applied_event_id.payload.yjs_update_sha256 != $updates[0].update_sha256
            OR $after.applied_event_id.payload.update_seq != $updates[0].update_seq
            OR $after.applied_event_id.payload.update_state_vector_before != $updates[0].state_vector_before
            OR $after.applied_event_id.payload.update_event_ledger_event_id != record::id($updates[0].event_ledger_event_id)
            OR $after.applied_event_id.payload.update_ledger_payload_hash != $updates[0].event_ledger_event_id.payload_hash
            OR !fn::wp012_model_lane_application_evidence($before, $updates[0], (SELECT * FROM ONLY $after.applied_event_id)) {
            THROW 'HSK-403-MODEL-LANE-PROPOSAL-APPLICATION';
        };
    };
};
