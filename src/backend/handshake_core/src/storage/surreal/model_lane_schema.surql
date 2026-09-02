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
