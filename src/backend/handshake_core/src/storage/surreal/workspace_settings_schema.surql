-- MT-021 embedded workspace-settings authority.
-- This schema is intentionally disjoint from the generated compatibility table in schema.surql.
-- Every durable identity, read, and mutation is scoped by all five ResourceScope dimensions.

DEFINE TABLE IF NOT EXISTS workspace_settings_schema_state SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD IF NOT EXISTS schema_version ON TABLE workspace_settings_schema_state TYPE string;
DEFINE FIELD IF NOT EXISTS schema_revision ON TABLE workspace_settings_schema_state TYPE int;
DEFINE FIELD IF NOT EXISTS apply_state ON TABLE workspace_settings_schema_state TYPE string;

DEFINE TABLE OVERWRITE workspace_settings_authority SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE owner_account_id ON TABLE workspace_settings_authority TYPE string ASSERT string::len($value) > 0;
DEFINE FIELD OVERWRITE actor_principal_id ON TABLE workspace_settings_authority TYPE string ASSERT string::len($value) > 0;
DEFINE FIELD OVERWRITE authenticated_session_id ON TABLE workspace_settings_authority TYPE string ASSERT string::len($value) > 0;
DEFINE FIELD OVERWRITE access_space_id ON TABLE workspace_settings_authority TYPE string ASSERT string::len($value) > 0;
DEFINE FIELD OVERWRITE workspace_id ON TABLE workspace_settings_authority TYPE string ASSERT string::len(string::trim($value)) > 0;
DEFINE FIELD OVERWRITE settings_state ON TABLE workspace_settings_authority TYPE object FLEXIBLE ASSERT $value.schema_id = 'hsk.workspace_settings_state@1';
DEFINE FIELD OVERWRITE settings_state.* ON TABLE workspace_settings_authority TYPE any;
DEFINE FIELD OVERWRITE generation ON TABLE workspace_settings_authority TYPE int ASSERT $value >= 1;
DEFINE FIELD OVERWRITE updated_at ON TABLE workspace_settings_authority TYPE datetime DEFAULT time::now();
DEFINE FIELD OVERWRITE event_ledger_event_id ON TABLE workspace_settings_authority TYPE record<kernel_event_ledger> ASSERT record::exists($value) REFERENCE ON DELETE REJECT;
DEFINE FIELD OVERWRITE storage_authority ON TABLE workspace_settings_authority TYPE string VALUE 'embedded_surrealdb' ASSERT $value = 'embedded_surrealdb';
DEFINE INDEX OVERWRITE uq_workspace_settings_exact_scope ON TABLE workspace_settings_authority FIELDS owner_account_id, actor_principal_id, authenticated_session_id, access_space_id, workspace_id UNIQUE;
DEFINE INDEX OVERWRITE idx_workspace_settings_event ON TABLE workspace_settings_authority FIELDS event_ledger_event_id;

-- Mutation receipts append to the canonical kernel_event_ledger. This module
-- deliberately defines no domain-local ledger or replay island.
