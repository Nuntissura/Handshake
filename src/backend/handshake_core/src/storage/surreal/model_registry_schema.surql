-- Native registry operations retain their issued request, rather than acquiring
-- read/write authority from matching workspace strings or a different grant.
DEFINE FUNCTION OVERWRITE fn::wp012_model_registry_request($scope: object, $write: bool) {
    IF !fn::wp012_exact_loom_scope($scope.owner_account_id, $scope.actor_principal_id,
            $scope.authenticated_session_id, $scope.access_space_id, $scope.workspace_id)
        OR $wp012_original_session != $scope.authenticated_session_id
        OR $wp012_original_workspace != $scope.workspace_id
        OR $wp012_original_grant = NONE { RETURN false; };
    LET $resource = type::record('protected_resources', $wp012_original_resource);
    LET $grant = type::record('resource_grants', $wp012_original_grant);
    LET $write_request = $wp012_original_capability = 'fs.write' AND $wp012_original_action IN ['create', 'update'];
    IF ($write AND !$write_request)
        OR (!$write AND !$write_request AND !($wp012_original_capability = 'fs.read' AND $wp012_original_action = 'read'))
        OR $resource.resource_kind != 'workspace' OR $resource.external_resource_id != $scope.workspace_id
        OR $resource.lifecycle_state != 'active' OR $resource.created_by_principal_id.status != 'enabled'
        OR $resource.owner_account_id != $auth.account_id OR $resource.access_space_id != $auth.access_space_id
        OR $grant.resource_id != $resource OR $grant.account_id != $auth.account_id
        OR $grant.principal_id != $auth.principal_id OR $grant.access_space_id != $auth.access_space_id
        OR $grant.status != 'active' OR $grant.revoked_at != NONE
        OR ($grant.expires_at != NONE AND $grant.expires_at <= time::now())
        OR $grant.delegation_chain != $auth.delegation_chain
        OR $grant.policy_version < $resource.policy_version OR $grant.policy_version > $auth.policy_version
        OR $wp012_original_action NOT IN $grant.actions OR $wp012_original_capability NOT IN $grant.capability_ids { RETURN false; };
    RETURN fn::mt109_ledger_access($resource, $auth.id, $wp012_original_capability, $wp012_original_action)
        AND fn::mt109_has_workspace_access($scope.workspace_id, $wp012_original_action, $wp012_original_capability);
};

DEFINE FUNCTION OVERWRITE fn::wp012_model_registry_active_payload($payload: object, $event: string, $validate_target: bool) {
    IF array::sort(object::keys($payload)) != ['access_space_id', 'action', 'actor_principal_id',
            'authenticated_session_id', 'mutation_fingerprint', 'owner_account_id', 'previous_artifact_sha256',
            'previous_selection_revision', 'purpose', 'reason', 'runtime_role', 'schema_id', 'selection_revision',
            'target_artifact_sha256', 'workspace_id']
        OR $payload.schema_id != 'hsk.model_runtime.active_selection_event@1'
        OR !(($payload.purpose = 'application/default' AND $payload.runtime_role = 'completion')
            OR ($payload.purpose = 'embeddings/default' AND $payload.runtime_role = 'embedding'))
        OR !type::is_int($payload.selection_revision) OR $payload.selection_revision < 1
        OR string::len($payload.target_artifact_sha256) != 64 OR string::len($payload.mutation_fingerprint) != 64
        OR !type::is_string($payload.reason) OR string::trim($payload.reason) = '' { RETURN false; };
    IF $validate_target {
        IF !fn::wp012_model_registry_request($payload, false) { RETURN false; };
        LET $targets = SELECT runtime_role, lifecycle_state FROM model_runtime_registry
            WHERE owner_account_id = $payload.owner_account_id AND actor_principal_id = $payload.actor_principal_id
            AND authenticated_session_id = $payload.authenticated_session_id AND access_space_id = $payload.access_space_id
            AND workspace_id = type::record('workspaces', $payload.workspace_id)
            AND artifact_sha256 = $payload.target_artifact_sha256 LIMIT 2;
        IF array::len($targets) != 1 OR $targets[0].lifecycle_state != 'active'
            OR $targets[0].runtime_role != $payload.runtime_role { RETURN false; };
    };
    RETURN IF $event = 'MODEL_RUNTIME_SELECTION_RECORDED' {
        $payload.action = 'active_default_initialized' AND $payload.selection_revision = 1
            AND $payload.previous_artifact_sha256 = NULL AND $payload.previous_selection_revision = NULL
    } ELSE {
        $event = 'MODEL_RUNTIME_SELECTION_REBOUND' AND $payload.action = 'active_default_changed'
            AND type::is_int($payload.previous_selection_revision) AND $payload.previous_selection_revision > 0
            AND $payload.selection_revision = $payload.previous_selection_revision + 1
            AND string::len($payload.previous_artifact_sha256) = 64
            AND $payload.previous_artifact_sha256 != $payload.target_artifact_sha256
    };
};

DEFINE FUNCTION OVERWRITE fn::wp012_model_registry_active_event($event: object, $row: object) {
    RETURN $event.event_id = record::id($event.id)
        AND $event.kernel_task_run_id = 'MODELREG-ACTIVE-' + $event.payload.mutation_fingerprint
        AND $event.session_run_id = $row.authenticated_session_id
        AND $event.idempotency_key = 'model-runtime-active:' + $event.payload.mutation_fingerprint
        AND $event.event_version = 'kernel_event_v1' AND $event.source_component = 'model_runtime_registry'
        AND $event.aggregate_type = 'model_runtime_active_selection' AND $event.aggregate_id = $row.purpose
        AND $event.payload.purpose = $row.purpose AND $event.payload.runtime_role = $row.runtime_role
        AND $event.owner_account_id = $row.owner_account_id AND $event.actor_principal_id = $row.actor_principal_id
        AND $event.authenticated_session_id = $row.authenticated_session_id AND $event.access_space_id = $row.access_space_id
        AND $event.workspace_id = record::id($row.workspace_id)
        AND $event.payload.owner_account_id = $row.owner_account_id AND $event.payload.actor_principal_id = $row.actor_principal_id
        AND $event.payload.authenticated_session_id = $row.authenticated_session_id AND $event.payload.access_space_id = $row.access_space_id
        AND $event.payload.workspace_id = record::id($row.workspace_id)
        AND $event.actor_kind = $event.authority_session_id.principal_id.actor_kind
        AND $event.actor_id = $event.authority_session_id.principal_id.actor_id
        AND $event.authority_session_id = type::record('authenticated_sessions', $row.authenticated_session_id)
        AND $event.authority_resource_id.resource_kind = 'workspace'
        AND $event.authority_resource_id.external_resource_id = record::id($row.workspace_id)
        AND $event.authority_capability_id = 'fs.write' AND $event.authority_action IN ['create', 'update']
        AND $event.wsids = [record::id($row.workspace_id)]
        AND $event.payload_hash = crypto::sha256(encoding::json::encode($event.payload))
        AND fn::wp012_model_registry_active_payload($event.payload, $event.event_type, false);
};

DEFINE FUNCTION OVERWRITE fn::wp012_model_registry_active_row($row: object) {
    LET $key = 'MRAS-' + crypto::sha256(string::join([$row.owner_account_id, $row.actor_principal_id,
        $row.authenticated_session_id, $row.access_space_id, record::id($row.workspace_id), $row.purpose], '\u0000'));
    LET $created = (SELECT * FROM ONLY $row.selection_created_event_id);
    LET $latest = (SELECT * FROM ONLY $row.selection_updated_event_id);
    RETURN $row.selection_id = $key AND record::id($row.id) = $key
        AND fn::wp012_model_registry_active_event($created, $row)
        AND fn::wp012_model_registry_active_event($latest, $row)
        AND fn::wp012_model_registry_active_payload($latest.payload, $latest.event_type, true)
        AND $created.event_type = 'MODEL_RUNTIME_SELECTION_RECORDED' AND $created.payload.selection_revision = 1
        AND $latest.payload.selection_revision = $row.selection_revision
        AND $latest.payload.target_artifact_sha256 = $row.artifact_sha256
        AND $latest.payload.mutation_fingerprint = $row.latest_mutation_fingerprint
        AND $row.selection_created_at_utc = $created.created_at AND $row.selection_updated_at_utc = $latest.created_at
        AND IF $row.selection_revision = 1 {
            $created.id = $latest.id AND $row.last_request_fingerprint = NONE
        } ELSE {
            $latest.event_type = 'MODEL_RUNTIME_SELECTION_REBOUND' AND $latest.correlation_id = record::id($created.id)
                AND $row.last_request_fingerprint = $row.latest_mutation_fingerprint
        };
};

-- This is read authority for actual model-owned lifecycle rows. No process
-- control or lifecycle write permission is added.
DEFINE FUNCTION OVERWRITE fn::wp012_model_registry_process_read($row: object) {
    IF !(($wp012_original_action = 'read' AND $wp012_original_capability = 'fs.read')
            OR ($wp012_original_action = 'update' AND $wp012_original_capability = 'fs.write'
                AND $auth.principal_id.actor_kind = 'operator'))
        OR !fn::wp012_model_registry_request($row, false)
        OR $row.process_uuid = NONE OR record::id($row.id) != <string>$row.process_uuid
        OR string::len($row.model_artifact_sha256) != 64
        OR $row.owner_runtime_instance_id = NONE OR string::trim($row.owner_host_scope_id) = ''
        OR string::trim($row.owner_lease_schema_id) = '' OR string::trim($row.owner_lease_protocol) = ''
        OR string::trim($row.owner_lease_address) = '' OR $row.owner_lease_port < 1 OR $row.owner_lease_port > 65535
        OR $row.metadata.owner_account_id != $row.owner_account_id
        OR $row.metadata.actor_principal_id != $row.actor_principal_id
        OR $row.metadata.authenticated_session_id != $row.authenticated_session_id
        OR $row.metadata.access_space_id != $row.access_space_id OR $row.metadata.workspace_id != $row.workspace_id { RETURN false; };
    RETURN ($row.metadata.source = 'wp1_mt013_embedded_model_load' AND $row.owner_role = 'handshake-embedded-default'
        AND $row.engine_kind IN ['llamacpp', 'candle'] AND $row.os_pid = NONE AND $row.metadata.in_process = true
        AND $row.metadata.os_pid_absent_reason = 'in_process_library_load_no_os_process'
        AND $row.metadata.model_id = <string>$row.process_uuid AND $row.metadata.identity_contract_violation = NONE
        AND $row.metadata.runtime_instance_schema_id = $row.owner_lease_schema_id
        AND $row.metadata.runtime_instance_id = <string>$row.owner_runtime_instance_id
        AND $row.metadata.runtime_host_scope_id = $row.owner_host_scope_id
        AND $row.metadata.runtime_lease_protocol = $row.owner_lease_protocol
        AND $row.metadata.runtime_lease_address = $row.owner_lease_address
        AND $row.metadata.runtime_lease_port = $row.owner_lease_port)
        OR ($row.engine_kind = 'genie_x' AND $row.owner_role = 'handshake-configured-model'
            AND $row.os_pid = NONE AND $row.metadata.runtime_binding = 'genie_x'
            AND $row.metadata.lifecycle_kind = 'in_process_genie_x_configured_adapter'
            AND $row.metadata.execution_model = 'on_demand_one_shot_no_residency_claim'
            AND $row.metadata.model_id = <string>$row.process_uuid)
        OR (type::is_string($row.metadata.model_id) AND string::trim($row.metadata.model_id) != ''
            AND $row.metadata.process_engine_kind = $row.engine_kind
            AND (($row.metadata.runtime_binding = 'llama_cpp' AND $row.engine_kind = 'llamacpp')
                OR ($row.metadata.runtime_binding = 'candle' AND $row.engine_kind = 'candle')));
};

DEFINE FUNCTION OVERWRITE fn::wp012_model_registry_optional($actual: any, $expected: any) {
    RETURN IF $expected = NONE { $actual = NONE OR $actual = NULL } ELSE { $actual = $expected };
};

DEFINE FUNCTION OVERWRITE fn::wp012_model_registry_process_receipt_read($event: object) {
    IF $event.source_component != 'process_ledger' OR $event.aggregate_type != 'process_ownership'
        OR $event.event_version != 'hsk.process_ownership@1' OR $event.event_type NOT IN ['PROCESS_START', 'PROCESS_STOP']
        OR $event.event_id != record::id($event.id)
        OR $event.event_id != 'process-lifecycle-' + $event.aggregate_id + (IF $event.event_type = 'PROCESS_START' { '-start' } ELSE { '-stop' })
        OR $event.idempotency_key != 'process-lifecycle:' + $event.aggregate_id + (IF $event.event_type = 'PROCESS_START' { ':START' } ELSE { ':STOP' })
        OR $event.payload_hash != crypto::sha256(encoding::json::encode($event.payload)) { RETURN false; };
    LET $rows = SELECT * FROM type::record('kernel_process_lifecycle', $event.aggregate_id);
    IF array::len($rows) != 1 OR !fn::wp012_model_registry_process_read($rows[0]) { RETURN false; };
    LET $row = $rows[0];
    LET $owner = { runtime_instance_id: <string>$row.owner_runtime_instance_id, host_scope_id: $row.owner_host_scope_id,
        lease_schema_id: $row.owner_lease_schema_id, lease_protocol: $row.owner_lease_protocol,
        lease_address: $row.owner_lease_address, lease_port: $row.owner_lease_port };
    RETURN $row.event_ledger_event_id = $event.id AND $event.actor_kind = 'kernel_role' AND $event.actor_id = $row.owner_role
        AND $event.owner_account_id = $row.owner_account_id AND $event.actor_principal_id = $row.actor_principal_id
        AND $event.authenticated_session_id = $row.authenticated_session_id AND $event.access_space_id = $row.access_space_id
        AND $event.workspace_id = $row.workspace_id AND $event.payload.process_uuid = <string>$row.process_uuid
        AND $event.payload.model_artifact_sha256 = $row.model_artifact_sha256 AND $event.payload.runtime_owner = $owner
        AND $event.payload.engine_kind = $row.engine_kind AND $event.payload.owner_role = $row.owner_role
        AND type::is_string($event.payload.started_at) AND type::datetime($event.payload.started_at) = $row.started_at
        AND fn::wp012_model_registry_optional($event.payload.os_pid, $row.os_pid)
        AND fn::wp012_model_registry_optional($event.payload.parent_session_id, $row.parent_session_id)
        AND fn::wp012_model_registry_optional($event.payload.parent_process_id, IF $row.parent_process_id = NONE { NONE } ELSE { <string>$row.parent_process_id })
        AND fn::wp012_model_registry_optional($event.payload.sandbox_adapter_id, $row.sandbox_adapter_id)
        AND fn::wp012_model_registry_optional($event.payload.sandbox_internal_id, $row.sandbox_internal_id)
        AND fn::wp012_model_registry_optional($event.payload.owner_wp, $row.owner_wp)
        AND fn::wp012_model_registry_optional($event.payload.work_profile_id, $row.work_profile_id)
        AND $event.payload.metadata_jsonb = $row.metadata
        AND $event.kernel_task_run_id = (IF $row.parent_session_id = NONE { 'process-ledger' } ELSE { $row.parent_session_id })
        AND $event.session_run_id = $event.kernel_task_run_id
        AND IF $event.event_type = 'PROCESS_START' {
            $row.stopped_at = NONE AND $event.payload.event_kind = 'START'
        } ELSE {
            $row.stopped_at != NONE AND $event.payload.event_kind = 'STOP'
                AND type::is_string($event.payload.stopped_at) AND type::datetime($event.payload.stopped_at) = $row.stopped_at
                AND fn::wp012_model_registry_optional($event.payload.exit_code, $row.exit_code)
                AND fn::wp012_model_registry_optional($event.payload.stop_reason, $row.stop_reason)
        };
};

-- Exact-scope embedded-SurrealDB ModelRuntime registry authority.
-- Shared schema owns workspaces and kernel_event_ledger. This extension uses
-- those canonical tables; it creates no parallel audit or storage island.

DEFINE FUNCTION OVERWRITE fn::wp012_model_registry_registration_selection($row: object) {
    RETURN { artifact_sha256: $row.artifact_sha256, runtime_binding: $row.runtime_binding,
        runtime_role: $row.runtime_role, capabilities_schema_id: $row.capabilities_schema_id,
        declared_capabilities: $row.capabilities, provider: $row.provider };
};

DEFINE FUNCTION OVERWRITE fn::wp012_model_registry_rebind_payload($payload: object) {
    IF array::sort(object::keys($payload)) != ['access_space_id', 'action', 'actor_principal_id', 'artifact_sha256',
            'authenticated_session_id', 'mutation_fingerprint', 'owner_account_id', 'previous_selection',
            'previous_selection_revision', 'reason', 'runtime_role', 'schema_id', 'selection_revision', 'target_selection', 'workspace_id']
        OR $payload.schema_id != 'hsk.model_runtime.selection_event@3'
        OR $payload.action != 'model_runtime_selection_rebound_after_verified_unload'
        OR string::len($payload.artifact_sha256) != 64 OR string::len($payload.mutation_fingerprint) != 64
        OR !type::is_int($payload.previous_selection_revision) OR $payload.previous_selection_revision < 1
        OR $payload.selection_revision != $payload.previous_selection_revision + 1
        OR !type::is_string($payload.reason) OR string::trim($payload.reason) = ''
        OR $payload.runtime_role NOT IN ['completion', 'embedding'] { RETURN false; };
    FOR $selection IN [$payload.previous_selection, $payload.target_selection] {
        IF array::sort(object::keys($selection)) != ['artifact_sha256', 'capabilities_schema_id', 'declared_capabilities', 'provider', 'runtime_binding', 'runtime_role']
            OR $selection.artifact_sha256 != $payload.artifact_sha256 OR $selection.runtime_role != $payload.runtime_role
            OR $selection.capabilities_schema_id != 'hsk.model_runtime.capabilities@1'
            OR $selection.runtime_binding NOT IN ['candle', 'llama_cpp', 'genie_x']
            OR $selection.provider NOT IN ['local', 'external_compat', 'byok_cloud', 'official_cli']
            OR !type::is_object($selection.declared_capabilities) { RETURN false; };
    };
    RETURN $payload.previous_selection != $payload.target_selection;
};

DEFINE FUNCTION OVERWRITE fn::wp012_model_registry_registration_event($event: object, $row: object) {
    IF $event.event_id != record::id($event.id) OR $event.event_version != 'kernel_event_v1'
        OR $event.source_component != 'model_runtime_registry' OR $event.aggregate_type != 'model_runtime_registry'
        OR $event.aggregate_id != $row.registry_row_id OR $event.session_run_id != $row.authenticated_session_id
        OR $event.owner_account_id != $row.owner_account_id OR $event.actor_principal_id != $row.actor_principal_id
        OR $event.authenticated_session_id != $row.authenticated_session_id OR $event.access_space_id != $row.access_space_id
        OR $event.workspace_id != record::id($row.workspace_id)
        OR $event.payload.owner_account_id != $row.owner_account_id OR $event.payload.actor_principal_id != $row.actor_principal_id
        OR $event.payload.authenticated_session_id != $row.authenticated_session_id OR $event.payload.access_space_id != $row.access_space_id
        OR $event.payload.workspace_id != record::id($row.workspace_id)
        OR $event.payload.artifact_sha256 != $row.artifact_sha256
        OR $event.authority_session_id != type::record('authenticated_sessions', $row.authenticated_session_id)
        OR $event.actor_kind != $event.authority_session_id.principal_id.actor_kind
        OR $event.actor_id != $event.authority_session_id.principal_id.actor_id
        OR $event.authority_resource_id.resource_kind != 'workspace'
        OR $event.authority_resource_id.external_resource_id != record::id($row.workspace_id)
        OR $event.wsids != [record::id($row.workspace_id)] OR $event.authority_capability_id != 'fs.write'
        OR $event.payload_hash != crypto::sha256(encoding::json::encode($event.payload)) { RETURN false; };
    RETURN IF $event.event_type = 'MODEL_RUNTIME_SELECTION_RECORDED' {
        $event.authority_action = 'create' AND $event.payload.action = 'model_runtime_selection_recorded'
            AND $event.payload.schema_id = 'hsk.model_runtime.selection_event@3'
            AND $event.payload.registry_row_id = $row.registry_row_id AND $event.payload.selection_revision = 1
            AND $event.kernel_task_run_id = 'MODELREG-' + $event.payload.mutation_fingerprint
            AND $event.idempotency_key = 'model-runtime-registry:initial:' + $event.payload.mutation_fingerprint
            AND $event.causation_id = NONE AND $event.correlation_id = NONE
    } ELSE {
        $event.event_type = 'MODEL_RUNTIME_SELECTION_REBOUND' AND $event.authority_action = 'update'
            AND $event.actor_kind = 'operator' AND fn::wp012_model_registry_rebind_payload($event.payload)
            AND $event.kernel_task_run_id = 'MODELREG-REBIND-' + $event.payload.mutation_fingerprint
            AND $event.idempotency_key = 'model-runtime-registry:rebind:' + $event.payload.mutation_fingerprint
    };
};

DEFINE FUNCTION OVERWRITE fn::wp012_model_registry_registration_row($row: object) {
    LET $created = (SELECT * FROM ONLY $row.selection_created_event_id);
    LET $latest = (SELECT * FROM ONLY $row.selection_updated_event_id);
    LET $selection = fn::wp012_model_registry_registration_selection($row);
    LET $scope = { owner_account_id: $row.owner_account_id, actor_principal_id: $row.actor_principal_id,
        authenticated_session_id: $row.authenticated_session_id, access_space_id: $row.access_space_id,
        workspace_id: record::id($row.workspace_id) };
    IF $row.registry_row_id != record::id($row.id) OR $row.lifecycle_state != 'active'
        OR $row.selection_event_id != $row.selection_created_event_id
        OR !fn::wp012_model_registry_registration_event($created, $row)
        OR !fn::wp012_model_registry_registration_event($latest, $row)
        OR $created.event_type != 'MODEL_RUNTIME_SELECTION_RECORDED'
        OR $created.payload.mutation_fingerprint != $row.mutation_fingerprint
        OR $created.created_at != $row.selection_created_at_utc OR $latest.created_at != $row.selection_updated_at_utc
        OR $latest.payload.selection_revision != $row.selection_revision
        OR $latest.payload.mutation_fingerprint != $row.latest_mutation_fingerprint
        OR $row.current_selection_fingerprint != crypto::sha256(encoding::json::encode({ scope: $scope, selection: $selection })) { RETURN false; };
    RETURN IF $row.selection_revision = 1 {
        $created.id = $latest.id AND $created.payload.selection = $selection
            AND $row.current_selection_fingerprint = $row.mutation_fingerprint
            AND $row.latest_mutation_fingerprint = $row.mutation_fingerprint AND $row.last_rebind_request_fingerprint = NONE
    } ELSE {
        $latest.event_type = 'MODEL_RUNTIME_SELECTION_REBOUND' AND $latest.payload.target_selection = $selection
            AND $latest.payload.runtime_role = $row.runtime_role
            AND $latest.payload.previous_selection_revision = $row.selection_revision - 1
            AND $latest.correlation_id = record::id($created.id)
            AND type::is_string($latest.causation_id) AND string::trim($latest.causation_id) != ''
            AND $latest.causation_id != $latest.event_id AND $row.last_rebind_request_fingerprint = $row.latest_mutation_fingerprint
    };
};

DEFINE TABLE OVERWRITE model_runtime_registry SCHEMAFULL PERMISSIONS
    FOR select WHERE fn::wp012_exact_loom_scope(owner_account_id, actor_principal_id, authenticated_session_id, access_space_id, record::id(workspace_id))
        AND fn::wp012_model_registry_registration_row($this)
        AND (fn::mt109_has_workspace_access(record::id(workspace_id), 'read', 'fs.read')
            OR ($wp012_original_action IN ['create', 'update'] AND $wp012_original_capability = 'fs.write'
                AND fn::wp012_model_registry_request(object::extend($this, { workspace_id: record::id(workspace_id) }), true)))
    FOR create WHERE $wp012_original_action = 'create' AND $wp012_original_capability = 'fs.write'
        AND fn::wp012_model_registry_request(object::extend($this, { workspace_id: record::id(workspace_id) }), true)
        AND selection_revision = 1 AND fn::wp012_model_registry_registration_row($this)
        AND selection_created_event_id.payload.base_model_tag = base_model_tag
        AND selection_created_event_id.payload.last_observed_by = last_observed_by
    FOR update WHERE fn::wp012_model_registry_request(object::extend($this, { workspace_id: record::id(workspace_id) }), true)
        AND ($wp012_original_action = 'create' OR ($wp012_original_action = 'update' AND $auth.principal_id.actor_kind = 'operator'))
        AND fn::wp012_model_registry_registration_row($this)
    FOR delete NONE;
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
DEFINE FIELD OVERWRITE runtime_binding ON TABLE model_runtime_registry TYPE 'candle' | 'llama_cpp' | 'genie_x';
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

DEFINE TABLE OVERWRITE model_runtime_active_selection SCHEMAFULL PERMISSIONS
    FOR select WHERE fn::wp012_model_registry_request(object::extend($this, { workspace_id: record::id(workspace_id) }), false)
        AND fn::wp012_model_registry_active_row($this)
    FOR create, update WHERE fn::wp012_model_registry_request(object::extend($this, { workspace_id: record::id(workspace_id) }), true)
        AND fn::wp012_model_registry_active_row($this)
    FOR delete NONE;
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

DEFINE EVENT OVERWRITE model_runtime_active_selection_transition_guard
ON TABLE model_runtime_active_selection
WHEN $event IN ['CREATE', 'UPDATE']
THEN {
    IF !fn::wp012_model_registry_active_row($after) { THROW 'MODEL_RUNTIME_ACTIVE_SELECTION_RECEIPT_INVALID'; };
    IF $event = 'CREATE' {
        IF $after.selection_revision != 1 { THROW 'MODEL_RUNTIME_ACTIVE_SELECTION_INITIAL_REVISION_INVALID'; };
    } ELSE {
        LET $receipt = $after.selection_updated_event_id;
        IF $before.id != $after.id OR $before.selection_id != $after.selection_id OR $before.schema_id != $after.schema_id
            OR $before.owner_account_id != $after.owner_account_id OR $before.actor_principal_id != $after.actor_principal_id
            OR $before.authenticated_session_id != $after.authenticated_session_id OR $before.access_space_id != $after.access_space_id
            OR $before.workspace_id != $after.workspace_id OR $before.purpose != $after.purpose OR $before.runtime_role != $after.runtime_role
            OR $before.lifecycle_state != 'active' OR $after.lifecycle_state != $before.lifecycle_state
            OR $after.selection_created_event_id != $before.selection_created_event_id
            OR $after.selection_created_at_utc != $before.selection_created_at_utc
            OR $after.selection_revision != $before.selection_revision + 1
            OR $after.artifact_sha256 = $before.artifact_sha256 OR $after.selection_updated_event_id = $before.selection_updated_event_id
            OR $receipt.payload.previous_selection_revision != $before.selection_revision
            OR $receipt.payload.previous_artifact_sha256 != $before.artifact_sha256
            OR $receipt.causation_id != record::id($before.selection_updated_event_id)
            OR $receipt.correlation_id != record::id($before.selection_created_event_id) {
            THROW 'MODEL_RUNTIME_ACTIVE_SELECTION_CAS_INVALID';
        };
    };
};

DEFINE EVENT OVERWRITE model_runtime_registry_transition_guard
ON TABLE model_runtime_registry
WHEN $event IN ['CREATE', 'UPDATE']
THEN {
    IF !fn::wp012_model_registry_registration_row($after) { THROW 'MODEL_RUNTIME_REGISTRY_RECEIPT_INVALID'; };
    IF $event = 'CREATE' {
        IF $after.selection_revision != 1 { THROW 'MODEL_RUNTIME_REGISTRY_INITIAL_REVISION_INVALID'; };
    } ELSE IF $wp012_original_action = 'create' AND $wp012_original_capability = 'fs.write' {
        LET $allowed = ['base_model_tag', 'last_observed_at_utc', 'last_observed_by', 'last_observed_runtime_model_id'];
        LET $old_fixed = object::from_entries(array::filter(object::entries($before), |$entry| $entry[0] NOT IN $allowed));
        LET $new_fixed = object::from_entries(array::filter(object::entries($after), |$entry| $entry[0] NOT IN $allowed));
        IF $before.lifecycle_state != 'active' OR $old_fixed != $new_fixed {
            THROW 'MODEL_RUNTIME_REGISTRY_BOOT_REOBSERVATION_INVALID';
        };
    } ELSE {
        LET $allowed = ['capabilities', 'current_selection_fingerprint', 'last_rebind_request_fingerprint',
            'latest_mutation_fingerprint', 'provider', 'runtime_binding', 'selection_revision',
            'selection_updated_at_utc', 'selection_updated_event_id'];
        LET $old_fixed = object::from_entries(array::filter(object::entries($before), |$entry| $entry[0] NOT IN $allowed));
        LET $new_fixed = object::from_entries(array::filter(object::entries($after), |$entry| $entry[0] NOT IN $allowed));
        LET $receipt = (SELECT * FROM ONLY $after.selection_updated_event_id);
        IF $wp012_original_action != 'update' OR $wp012_original_capability != 'fs.write'
            OR $auth.principal_id.actor_kind != 'operator' OR $before.lifecycle_state != 'active'
            OR $old_fixed != $new_fixed OR $after.selection_revision != $before.selection_revision + 1
            OR $after.selection_updated_event_id = $before.selection_updated_event_id
            OR $receipt.event_type != 'MODEL_RUNTIME_SELECTION_REBOUND' OR $receipt.actor_kind != 'operator'
            OR $receipt.payload.previous_selection != fn::wp012_model_registry_registration_selection($before)
            OR $receipt.payload.target_selection != fn::wp012_model_registry_registration_selection($after)
            OR $receipt.payload.previous_selection_revision != $before.selection_revision
            OR $receipt.causation_id != record::id($before.selection_updated_event_id)
            OR $receipt.correlation_id != record::id($before.selection_created_event_id)
            OR $after.current_selection_fingerprint = $before.current_selection_fingerprint {
            THROW 'MODEL_RUNTIME_REGISTRY_REBIND_CAS_INVALID';
        };
    };
};
