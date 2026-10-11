-- Exact-scope embedded-SurrealDB ModelRuntime registry authority.
-- Shared schema owns workspaces and kernel_event_ledger. This extension uses
-- those canonical tables; it creates no parallel audit or storage island.

DEFINE TABLE OVERWRITE model_runtime_registry SCHEMAFULL PERMISSIONS
    FOR select WHERE fn::wp012_exact_loom_scope(owner_account_id, actor_principal_id, authenticated_session_id, access_space_id, record::id(workspace_id)) AND selection_revision = 1 AND selection_event_id = selection_created_event_id AND selection_event_id = selection_updated_event_id AND current_selection_fingerprint = mutation_fingerprint AND latest_mutation_fingerprint = mutation_fingerprint AND selection_event_id.payload.mutation_fingerprint = mutation_fingerprint AND selection_event_id.payload.registry_row_id = registry_row_id AND selection_event_id.payload.artifact_sha256 = artifact_sha256 AND selection_event_id.payload.selection.runtime_role = runtime_role AND selection_event_id.payload.selection.runtime_binding = runtime_binding AND selection_event_id.payload.selection.declared_capabilities = capabilities AND selection_event_id.payload.selection.provider = provider AND selection_event_id.payload.base_model_tag = base_model_tag AND lifecycle_state = 'active' AND selection_event_id.event_type = 'MODEL_RUNTIME_SELECTION_RECORDED' AND selection_event_id.source_component = 'model_runtime_registry' AND selection_event_id.aggregate_type = 'model_runtime_registry' AND selection_event_id.aggregate_id = registry_row_id AND selection_event_id.owner_account_id = owner_account_id AND selection_event_id.actor_principal_id = actor_principal_id AND selection_event_id.authenticated_session_id = authenticated_session_id AND selection_event_id.access_space_id = access_space_id AND selection_event_id.workspace_id = record::id(workspace_id) AND selection_event_id.payload.owner_account_id = owner_account_id AND selection_event_id.payload.actor_principal_id = actor_principal_id AND selection_event_id.payload.authenticated_session_id = authenticated_session_id AND selection_event_id.payload.access_space_id = access_space_id AND selection_event_id.payload.workspace_id = record::id(workspace_id) AND fn::wp012_api_receipt(selection_event_id.authority_resource_id, selection_event_id.authority_session_id, selection_event_id.authority_capability_id, selection_event_id.authority_action, selection_event_id.wsids, selection_event_id.event_type, selection_event_id.source_component, selection_event_id.aggregate_type, selection_event_id.aggregate_id, selection_event_id.payload, selection_event_id.actor_kind, selection_event_id.actor_id, false) AND fn::mt109_has_workspace_access(record::id(workspace_id), 'read', 'fs.read')
    FOR create WHERE fn::wp012_exact_loom_scope(owner_account_id, actor_principal_id, authenticated_session_id, access_space_id, record::id(workspace_id)) AND selection_revision = 1 AND selection_event_id = selection_created_event_id AND selection_event_id = selection_updated_event_id AND current_selection_fingerprint = mutation_fingerprint AND latest_mutation_fingerprint = mutation_fingerprint AND selection_event_id.payload.mutation_fingerprint = mutation_fingerprint AND selection_event_id.payload.registry_row_id = registry_row_id AND selection_event_id.payload.artifact_sha256 = artifact_sha256 AND selection_event_id.payload.selection.runtime_role = runtime_role AND selection_event_id.payload.selection.runtime_binding = runtime_binding AND selection_event_id.payload.selection.declared_capabilities = capabilities AND selection_event_id.payload.selection.provider = provider AND selection_event_id.payload.base_model_tag = base_model_tag AND lifecycle_state = 'active' AND selection_event_id.event_type = 'MODEL_RUNTIME_SELECTION_RECORDED' AND selection_event_id.source_component = 'model_runtime_registry' AND selection_event_id.aggregate_type = 'model_runtime_registry' AND selection_event_id.aggregate_id = registry_row_id AND selection_event_id.owner_account_id = owner_account_id AND selection_event_id.actor_principal_id = actor_principal_id AND selection_event_id.authenticated_session_id = authenticated_session_id AND selection_event_id.access_space_id = access_space_id AND selection_event_id.workspace_id = record::id(workspace_id) AND selection_event_id.payload.owner_account_id = owner_account_id AND selection_event_id.payload.actor_principal_id = actor_principal_id AND selection_event_id.payload.authenticated_session_id = authenticated_session_id AND selection_event_id.payload.access_space_id = access_space_id AND selection_event_id.payload.workspace_id = record::id(workspace_id) AND fn::wp012_api_receipt(selection_event_id.authority_resource_id, selection_event_id.authority_session_id, selection_event_id.authority_capability_id, selection_event_id.authority_action, selection_event_id.wsids, selection_event_id.event_type, selection_event_id.source_component, selection_event_id.aggregate_type, selection_event_id.aggregate_id, selection_event_id.payload, selection_event_id.actor_kind, selection_event_id.actor_id, false) AND fn::mt109_has_workspace_access(record::id(workspace_id), 'create', 'fs.write')
    FOR update, delete NONE;
DEFINE FIELD OVERWRITE registry_row_id ON TABLE model_runtime_registry TYPE string ASSERT $value = record::id($this.id);
DEFINE FIELD OVERWRITE schema_id ON TABLE model_runtime_registry TYPE 'hsk.model_runtime_registry.row@2';
DEFINE FIELD OVERWRITE owner_account_id ON TABLE model_runtime_registry TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE actor_principal_id ON TABLE model_runtime_registry TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE authenticated_session_id ON TABLE model_runtime_registry TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE access_space_id ON TABLE model_runtime_registry TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE workspace_id ON TABLE model_runtime_registry TYPE record<workspaces> ASSERT record::exists($value) REFERENCE ON DELETE CASCADE;
DEFINE FIELD OVERWRITE artifact_sha256 ON TABLE model_runtime_registry TYPE string ASSERT string::len($value) = 64;
DEFINE FIELD OVERWRITE artifact_locator ON TABLE model_runtime_registry TYPE string ASSERT $value = string::concat('artifact://sha256/', $this.artifact_sha256);
DEFINE FIELD OVERWRITE last_observed_runtime_model_id ON TABLE model_runtime_registry TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE runtime_binding ON TABLE model_runtime_registry TYPE 'candle' | 'llama_cpp';
DEFINE FIELD OVERWRITE runtime_role ON TABLE model_runtime_registry TYPE 'completion' | 'embedding';
DEFINE FIELD OVERWRITE capabilities_schema_id ON TABLE model_runtime_registry TYPE 'hsk.model_runtime.capabilities@1';
DEFINE FIELD OVERWRITE capabilities ON TABLE model_runtime_registry TYPE object FLEXIBLE;
DEFINE FIELD OVERWRITE capabilities.* ON TABLE model_runtime_registry TYPE any;
DEFINE FIELD OVERWRITE provider ON TABLE model_runtime_registry TYPE 'local' | 'external_compat' | 'byok_cloud' | 'official_cli';
DEFINE FIELD OVERWRITE base_model_tag ON TABLE model_runtime_registry TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE last_observed_by ON TABLE model_runtime_registry TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE embedding_space_id ON TABLE model_runtime_registry TYPE option<string> ASSERT ($this.runtime_role = 'embedding' AND $value != NONE AND string::trim($value) != '') OR ($this.runtime_role = 'completion' AND $value = NONE);
DEFINE FIELD OVERWRITE embedding_dimension ON TABLE model_runtime_registry TYPE option<int> ASSERT ($this.runtime_role = 'embedding' AND $value != NONE AND $value > 0) OR ($this.runtime_role = 'completion' AND $value = NONE);
DEFINE FIELD OVERWRITE lifecycle_state ON TABLE model_runtime_registry TYPE 'active' | 'stale' | 'revoked';
DEFINE FIELD OVERWRITE selection_revision ON TABLE model_runtime_registry TYPE int ASSERT $value > 0;
DEFINE FIELD OVERWRITE selection_event_id ON TABLE model_runtime_registry TYPE record<kernel_event_ledger> ASSERT record::exists($value) REFERENCE ON DELETE REJECT;
DEFINE FIELD OVERWRITE selection_created_event_id ON TABLE model_runtime_registry TYPE record<kernel_event_ledger> ASSERT record::exists($value) REFERENCE ON DELETE REJECT;
DEFINE FIELD OVERWRITE selection_updated_event_id ON TABLE model_runtime_registry TYPE record<kernel_event_ledger> ASSERT record::exists($value) REFERENCE ON DELETE REJECT;
DEFINE FIELD OVERWRITE current_selection_fingerprint ON TABLE model_runtime_registry TYPE string ASSERT string::len($value) = 64;
DEFINE FIELD OVERWRITE latest_mutation_fingerprint ON TABLE model_runtime_registry TYPE string ASSERT string::len($value) = 64;
DEFINE FIELD OVERWRITE mutation_fingerprint ON TABLE model_runtime_registry TYPE string ASSERT string::len($value) = 64;
DEFINE FIELD OVERWRITE last_rebind_request_fingerprint ON TABLE model_runtime_registry TYPE option<string> ASSERT $value = NONE OR string::len($value) = 64;
DEFINE FIELD OVERWRITE registered_at_utc ON TABLE model_runtime_registry TYPE datetime;
DEFINE FIELD OVERWRITE selection_created_at_utc ON TABLE model_runtime_registry TYPE datetime;
DEFINE FIELD OVERWRITE selection_updated_at_utc ON TABLE model_runtime_registry TYPE datetime;
DEFINE FIELD OVERWRITE last_observed_at_utc ON TABLE model_runtime_registry TYPE datetime;
DEFINE INDEX OVERWRITE pk_model_runtime_registry ON TABLE model_runtime_registry FIELDS registry_row_id UNIQUE;
DEFINE INDEX OVERWRITE uq_model_runtime_registry_scope_artifact ON TABLE model_runtime_registry FIELDS owner_account_id, actor_principal_id, authenticated_session_id, access_space_id, workspace_id, artifact_sha256 UNIQUE;
DEFINE INDEX OVERWRITE idx_model_runtime_registry_scope_role_state ON TABLE model_runtime_registry FIELDS owner_account_id, actor_principal_id, authenticated_session_id, access_space_id, workspace_id, runtime_role, lifecycle_state;
DEFINE INDEX OVERWRITE idx_model_runtime_registry_created_receipt ON TABLE model_runtime_registry FIELDS selection_created_event_id UNIQUE;
DEFINE INDEX OVERWRITE idx_model_runtime_registry_updated_receipt ON TABLE model_runtime_registry FIELDS selection_updated_event_id;

DEFINE TABLE OVERWRITE model_runtime_active_selection SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE selection_id ON TABLE model_runtime_active_selection TYPE string ASSERT $value = record::id($this.id);
DEFINE FIELD OVERWRITE schema_id ON TABLE model_runtime_active_selection TYPE 'hsk.model_runtime.active_selection@1';
DEFINE FIELD OVERWRITE owner_account_id ON TABLE model_runtime_active_selection TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE actor_principal_id ON TABLE model_runtime_active_selection TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE authenticated_session_id ON TABLE model_runtime_active_selection TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE access_space_id ON TABLE model_runtime_active_selection TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE workspace_id ON TABLE model_runtime_active_selection TYPE record<workspaces> ASSERT record::exists($value) REFERENCE ON DELETE CASCADE;
DEFINE FIELD OVERWRITE purpose ON TABLE model_runtime_active_selection TYPE 'application/default' | 'embeddings/default';
DEFINE FIELD OVERWRITE runtime_role ON TABLE model_runtime_active_selection TYPE 'completion' | 'embedding' ASSERT ($this.purpose = 'application/default' AND $value = 'completion') OR ($this.purpose = 'embeddings/default' AND $value = 'embedding');
DEFINE FIELD OVERWRITE artifact_sha256 ON TABLE model_runtime_active_selection TYPE string ASSERT string::len($value) = 64;
DEFINE FIELD OVERWRITE lifecycle_state ON TABLE model_runtime_active_selection TYPE 'active' | 'stale' | 'revoked';
DEFINE FIELD OVERWRITE selection_revision ON TABLE model_runtime_active_selection TYPE int ASSERT $value > 0;
DEFINE FIELD OVERWRITE selection_created_event_id ON TABLE model_runtime_active_selection TYPE record<kernel_event_ledger> ASSERT record::exists($value) REFERENCE ON DELETE REJECT;
DEFINE FIELD OVERWRITE selection_updated_event_id ON TABLE model_runtime_active_selection TYPE record<kernel_event_ledger> ASSERT record::exists($value) REFERENCE ON DELETE REJECT;
DEFINE FIELD OVERWRITE latest_mutation_fingerprint ON TABLE model_runtime_active_selection TYPE string ASSERT string::len($value) = 64;
DEFINE FIELD OVERWRITE last_request_fingerprint ON TABLE model_runtime_active_selection TYPE option<string> ASSERT $value = NONE OR string::len($value) = 64;
DEFINE FIELD OVERWRITE selection_created_at_utc ON TABLE model_runtime_active_selection TYPE datetime;
DEFINE FIELD OVERWRITE selection_updated_at_utc ON TABLE model_runtime_active_selection TYPE datetime;
DEFINE INDEX OVERWRITE pk_model_runtime_active_selection ON TABLE model_runtime_active_selection FIELDS selection_id UNIQUE;
DEFINE INDEX OVERWRITE uq_model_runtime_active_scope_purpose ON TABLE model_runtime_active_selection FIELDS owner_account_id, actor_principal_id, authenticated_session_id, access_space_id, workspace_id, purpose UNIQUE;
DEFINE INDEX OVERWRITE idx_model_runtime_active_scope_state ON TABLE model_runtime_active_selection FIELDS owner_account_id, actor_principal_id, authenticated_session_id, access_space_id, workspace_id, lifecycle_state;
DEFINE INDEX OVERWRITE idx_model_runtime_active_created_receipt ON TABLE model_runtime_active_selection FIELDS selection_created_event_id UNIQUE;
DEFINE INDEX OVERWRITE idx_model_runtime_active_updated_receipt ON TABLE model_runtime_active_selection FIELDS selection_updated_event_id;
