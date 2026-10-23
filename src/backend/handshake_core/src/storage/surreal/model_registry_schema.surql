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


DEFINE FUNCTION OVERWRITE fn::wp012_engine_profile_row($row: object, $write: bool) {
    LET $event = $row.receipt_id;
    RETURN fn::wp012_engine_profile_current_owner($row.owner_account_id,
        $row.actor_principal_id, $row.access_space_id, record::id($row.workspace_id), false)
        AND $event != NONE AND $event.event_type = 'ENGINE_PROFILE_MUTATION_RECORDED'
        AND $event.source_component = 'engine_profiles'
        AND $event.aggregate_type = 'engine_profile_registry'
        AND $event.aggregate_id = record::id($row.id)
        AND $event.owner_account_id = $row.owner_account_id
        AND $event.actor_principal_id = $row.actor_principal_id
        AND $event.authenticated_session_id = $row.authenticated_session_id
        AND $event.access_space_id = $row.access_space_id
        AND $event.workspace_id = record::id($row.workspace_id)
        AND $event.payload.owner_account_id = $row.owner_account_id
        AND $event.payload.actor_principal_id = $row.actor_principal_id
        AND $event.payload.authenticated_session_id = $row.authenticated_session_id
        AND $event.payload.access_space_id = $row.access_space_id
        AND $event.payload.workspace_id = record::id($row.workspace_id)
        AND $event.payload.after = $row.data
        AND crypto::sha256(encoding::json::encode($event.payload)) = $event.payload_hash
        AND fn::wp012_engine_profile_ledger_read($event)
        AND IF $write {
            $row.authenticated_session_id = record::id($auth.id)
            AND $event.authority_session_id = $auth.id
            AND fn::mt109_ledger_access($event.authority_resource_id, $event.authority_session_id,
                $event.authority_capability_id, $event.authority_action)
            AND fn::mt109_has_workspace_access(record::id($row.workspace_id), 'update', 'fs.write')
        } ELSE {
            fn::mt109_has_workspace_access(record::id($row.workspace_id), 'read', 'fs.read')
        };
};
DEFINE TABLE OVERWRITE engine_profile_registry SCHEMAFULL PERMISSIONS
    FOR select WHERE fn::wp012_engine_profile_row($this, false)
    FOR create, update WHERE fn::wp012_engine_profile_row($this, true)
    FOR delete NONE;
DEFINE FIELD OVERWRITE owner_account_id ON engine_profile_registry TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE actor_principal_id ON engine_profile_registry TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE authenticated_session_id ON engine_profile_registry TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE access_space_id ON engine_profile_registry TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE workspace_id ON engine_profile_registry TYPE record<workspaces> ASSERT record::exists($value) REFERENCE ON DELETE CASCADE;
DEFINE FIELD OVERWRITE data ON engine_profile_registry TYPE object FLEXIBLE;
DEFINE FIELD OVERWRITE data.* ON engine_profile_registry TYPE any;
DEFINE FIELD OVERWRITE receipt_id ON engine_profile_registry TYPE record<kernel_event_ledger> ASSERT record::exists($value) REFERENCE ON DELETE REJECT;
DEFINE INDEX OVERWRITE engine_profile_registry_scope ON engine_profile_registry
    FIELDS owner_account_id, actor_principal_id, access_space_id, workspace_id UNIQUE;


DEFINE EVENT OVERWRITE engine_profile_registry_transition_guard
ON TABLE engine_profile_registry
WHEN $event IN ['CREATE', 'UPDATE']
THEN {
    IF !fn::wp012_engine_profile_row($after, true) {
        THROW 'ENGINE_PROFILE_TRANSITION_DENIED';
    };
    LET $payload = $after.receipt_id.payload;
    IF $after.authenticated_session_id != record::id($auth.id)
        OR $after.receipt_id.authority_session_id != $auth.id {
        THROW 'ENGINE_PROFILE_TRANSITION_DENIED';
    };
    IF $after.data != $payload.after {
        THROW 'ENGINE_PROFILE_TRANSITION_DENIED';
    };
    IF $event = 'CREATE' {
        IF $payload.before != NULL OR $after.data.registry_generation != 1 {
            THROW 'ENGINE_PROFILE_TRANSITION_DENIED';
        };
    } ELSE {
        IF $before.id != $after.id
            OR $before.owner_account_id != $after.owner_account_id
            OR $before.actor_principal_id != $after.actor_principal_id
            OR $before.access_space_id != $after.access_space_id
            OR $before.workspace_id != $after.workspace_id
            OR $before.receipt_id = $after.receipt_id
            OR $payload.before = NULL
            OR $before.data != $payload.before
            OR !type::is_int($before.data.registry_generation)
            OR !type::is_int($after.data.registry_generation)
            OR $after.data.registry_generation != $before.data.registry_generation + 1 {
            THROW 'ENGINE_PROFILE_TRANSITION_DENIED';
        };
    };
};
