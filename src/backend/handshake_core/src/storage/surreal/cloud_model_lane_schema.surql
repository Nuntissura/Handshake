DEFINE FUNCTION OVERWRITE fn::wp012_cloud_schema_access() {
    LET $scope = { owner_account_id: record::id($auth.account_id), actor_principal_id: record::id($auth.principal_id), authenticated_session_id: record::id($auth.id), access_space_id: record::id($auth.access_space_id), workspace_id: $wp012_original_workspace };
    RETURN fn::wp012_model_lane_original_operation($scope)
        AND (($wp012_original_action = 'read' AND $wp012_original_purpose = 'source_read')
            OR ($wp012_original_action = 'create' AND $wp012_original_purpose IN ['cloud_projection_plan', 'cloud_consent_receipt', 'cloud_run', 'cloud_lane', 'cloud_consent_denial']));
};

DEFINE FUNCTION OVERWRITE fn::wp012_cloud_source_access($row: object) {
    IF $row.kind NOT IN ['projection_plan', 'consent_receipt', 'cloud_run', 'cloud_lane', 'consent_denial']
        OR !fn::wp012_model_lane_original_operation($row) { RETURN false; };
    RETURN ($wp012_original_action = 'read' AND $wp012_original_purpose = 'source_read')
        OR ($wp012_original_action = 'create' AND $wp012_original_purpose IN ['cloud_projection_plan', 'cloud_consent_receipt', 'cloud_run', 'cloud_lane', 'cloud_consent_denial']);
};
DEFINE FUNCTION OVERWRITE fn::wp012_cloud_receipt_create($row: object) {
    LET $scoped = object::extend($row, { kind: $row.aggregate_kind });
    IF !fn::wp012_cloud_source_access($scoped)
        OR $wp012_original_action != 'create' OR ($row.aggregate_kind = 'consent_denial' AND $wp012_original_purpose != 'cloud_consent_denial') OR ($row.aggregate_kind = 'projection_plan' AND $wp012_original_purpose != 'cloud_projection_plan') OR ($row.aggregate_kind = 'consent_receipt' AND $wp012_original_purpose != 'cloud_consent_receipt') OR ($row.aggregate_kind IN ['cloud_run', 'cloud_lane'] AND $row.aggregate_kind != $wp012_original_purpose)
        OR !fn::wp012_context_coordination_fence($row, 'workspace', $row.workspace_id)
        OR $row.authority_resource_id != type::record('protected_resources', $wp012_original_resource)
        OR $row.authority_grant_id != type::record('resource_grants', $wp012_original_grant)
        OR $row.authority_session_id != $auth.id OR $row.authority_action != $wp012_original_action
        OR $row.authority_capability_id != $wp012_original_capability OR $row.authority_purpose != $wp012_original_purpose
        OR $row.event_id != record::id($row.id) OR $row.event_seq <= 0
        OR $row.payload_hash != crypto::sha256($row.payload_json)
        OR $row.allocation.event_seq != $row.event_seq OR $row.allocation.payload_json != $row.payload_json
        OR $row.allocation.payload_hash != $row.payload_hash { RETURN false; };
    LET $payload = encoding::json::decode($row.payload_json);
    LET $record = $payload.record;
    RETURN type::is_object($record) AND $payload.dexterity_kernel = 'Dexterity'
        AND $record.run_id = $row.run_id AND $record.event_ledger_event_id = $row.event_id
        AND $record.event_ledger_seq = $row.event_seq
        AND ($row.aggregate_kind NOT IN ['projection_plan', 'consent_receipt'] OR ($record.event_stream_version = $row.event_seq AND $record.transaction_seq = $row.event_seq))
        AND $record.event_ledger_stream_id = $row.event_ledger_stream_id
        AND (($row.aggregate_kind = 'projection_plan' AND $record.projection_plan_id = $row.aggregate_id AND $payload.schema_id = 'hsk.model_lane_cloud_projection_plan@2')
            OR ($row.aggregate_kind = 'consent_receipt' AND $record.consent_receipt_id = $row.aggregate_id AND $payload.schema_id = 'hsk.model_lane_cloud_consent_receipt@2')
            OR ($row.aggregate_kind = 'cloud_run' AND $record.run_id = $row.aggregate_id AND $payload.schema_id = 'hsk.model_lane_run@1')
            OR ($row.aggregate_kind = 'cloud_lane' AND $record.lane_id = $row.aggregate_id AND $payload.schema_id = 'hsk.model_lane@1')
            OR ($row.aggregate_kind = 'consent_denial' AND $record.lane_id = $row.aggregate_id AND $payload.schema_id = 'hsk.model_lane_cloud_consent_denial@1' AND $record.provider_call_attempted = false AND $record.partial_authority_state_created = false));
};
DEFINE FUNCTION OVERWRITE fn::wp012_cloud_authority_create($row: object) {
    IF !fn::wp012_cloud_source_access($row) OR $wp012_original_action != 'create'
        OR ($row.kind = 'consent_denial' AND $wp012_original_purpose != 'cloud_consent_denial') OR ($row.kind = 'projection_plan' AND $wp012_original_purpose != 'cloud_projection_plan') OR ($row.kind = 'consent_receipt' AND $wp012_original_purpose != 'cloud_consent_receipt') OR ($row.kind IN ['cloud_run', 'cloud_lane'] AND $row.kind != $wp012_original_purpose)
        OR !fn::wp012_context_coordination_fence($row, 'workspace', $row.workspace_id) { RETURN false; };
    LET $key = crypto::sha256(string::join(['handshake.model-lane.cloud-authority.v1', $row.kind, $row.idempotency_key, $row.owner_account_id, $row.actor_principal_id, $row.authenticated_session_id, $row.access_space_id, $row.workspace_id], '\u0000'));
    IF record::id($row.id) != $key { RETURN false; };
    LET $ledger = (SELECT * FROM ONLY $row.receipt_id);
    LET $record = encoding::json::decode($row.record_json);
    RETURN $ledger != NONE AND $row.event_id = $ledger.event_id AND $row.event_seq = $ledger.event_seq
        AND $row.kind = $ledger.aggregate_kind AND $row.aggregate_id = $ledger.aggregate_id AND $row.run_id = $ledger.run_id
        AND $row.event_payload_json = $ledger.payload_json AND $ledger.payload_hash = crypto::sha256($ledger.payload_json)
        AND fn::wp012_cloud_record_identity($row, $record) AND $record = encoding::json::decode($ledger.payload_json).record
        AND (($row.kind = 'cloud_lane' AND $row.idempotency_key = string::concat('model-lane:', $record.run_id, ':', $record.lane_id))
            OR ($row.kind != 'cloud_lane' AND $row.idempotency_key = $record.idempotency_key))
        AND $row.owner_account_id = $ledger.owner_account_id AND $row.actor_principal_id = $ledger.actor_principal_id
        AND $row.authenticated_session_id = $ledger.authenticated_session_id AND $row.access_space_id = $ledger.access_space_id
        AND $row.workspace_id = $ledger.workspace_id AND $row.authority_resource_id = $ledger.authority_resource_id
        AND $row.authority_grant_id = $ledger.authority_grant_id AND $row.authority_session_id = $ledger.authority_session_id
        AND $row.authority_action = $ledger.authority_action AND $row.authority_capability_id = $ledger.authority_capability_id
        AND $row.authority_purpose = $ledger.authority_purpose;
};
-- Only the original authenticated cloud operations may publish these immutable rows.
DEFINE TABLE OVERWRITE model_lane_cloud_authority SCHEMAFULL
    PERMISSIONS FOR select WHERE fn::wp012_cloud_authority_read($this)
    FOR create WHERE fn::wp012_cloud_authority_create($this)
    FOR update, delete NONE;
DEFINE FIELD IF NOT EXISTS kind ON model_lane_cloud_authority TYPE string;
DEFINE FIELD IF NOT EXISTS aggregate_id ON model_lane_cloud_authority TYPE string;
DEFINE FIELD IF NOT EXISTS run_id ON model_lane_cloud_authority TYPE string;
DEFINE FIELD IF NOT EXISTS projection_plan_id ON model_lane_cloud_authority TYPE string;
DEFINE FIELD IF NOT EXISTS consent_receipt_id ON model_lane_cloud_authority TYPE string;
DEFINE FIELD IF NOT EXISTS idempotency_key ON model_lane_cloud_authority TYPE string;
DEFINE FIELD IF NOT EXISTS record_json ON model_lane_cloud_authority TYPE string;
DEFINE FIELD IF NOT EXISTS event_id ON model_lane_cloud_authority TYPE string;
DEFINE FIELD IF NOT EXISTS event_seq ON model_lane_cloud_authority TYPE int;
DEFINE FIELD IF NOT EXISTS event_payload_json ON model_lane_cloud_authority TYPE string;
DEFINE FIELD IF NOT EXISTS owner_account_id ON model_lane_cloud_authority TYPE string;
DEFINE FIELD IF NOT EXISTS actor_principal_id ON model_lane_cloud_authority TYPE string;
DEFINE FIELD IF NOT EXISTS authenticated_session_id ON model_lane_cloud_authority TYPE string;
DEFINE FIELD IF NOT EXISTS access_space_id ON model_lane_cloud_authority TYPE string;
DEFINE FIELD IF NOT EXISTS workspace_id ON model_lane_cloud_authority TYPE string;
DEFINE INDEX IF NOT EXISTS model_lane_cloud_authority_kind_aggregate_scope ON model_lane_cloud_authority FIELDS kind, aggregate_id, owner_account_id, actor_principal_id, authenticated_session_id, access_space_id, workspace_id UNIQUE;
DEFINE INDEX IF NOT EXISTS model_lane_cloud_authority_run_scope ON model_lane_cloud_authority FIELDS kind, run_id, owner_account_id, actor_principal_id, authenticated_session_id, access_space_id, workspace_id;
DEFINE INDEX IF NOT EXISTS model_lane_cloud_authority_consent_scope ON model_lane_cloud_authority FIELDS kind, consent_receipt_id, owner_account_id, actor_principal_id, authenticated_session_id, access_space_id, workspace_id;

DEFINE TABLE OVERWRITE model_lane_cloud_event_ledger SCHEMAFULL
    PERMISSIONS FOR select WHERE fn::wp012_cloud_source_access(object::extend($this, { kind: $this.aggregate_kind }))
        OR fn::wp012_cloud_checkpoint_dependency_read(object::extend($this, { kind: $this.aggregate_kind }))
    FOR create WHERE fn::wp012_cloud_receipt_create($this)
    FOR update, delete NONE;
DEFINE FIELD IF NOT EXISTS aggregate_kind ON model_lane_cloud_event_ledger TYPE string;
DEFINE FIELD IF NOT EXISTS aggregate_id ON model_lane_cloud_event_ledger TYPE string;
DEFINE FIELD IF NOT EXISTS run_id ON model_lane_cloud_event_ledger TYPE string;
DEFINE FIELD IF NOT EXISTS event_seq ON model_lane_cloud_event_ledger TYPE int;
DEFINE FIELD IF NOT EXISTS payload_json ON model_lane_cloud_event_ledger TYPE string;
DEFINE FIELD IF NOT EXISTS owner_account_id ON model_lane_cloud_event_ledger TYPE string;
DEFINE FIELD IF NOT EXISTS actor_principal_id ON model_lane_cloud_event_ledger TYPE string;
DEFINE FIELD IF NOT EXISTS authenticated_session_id ON model_lane_cloud_event_ledger TYPE string;
DEFINE FIELD IF NOT EXISTS access_space_id ON model_lane_cloud_event_ledger TYPE string;
DEFINE FIELD IF NOT EXISTS workspace_id ON model_lane_cloud_event_ledger TYPE string;
DEFINE INDEX OVERWRITE model_lane_cloud_event_ledger_aggregate ON model_lane_cloud_event_ledger FIELDS aggregate_kind, aggregate_id, owner_account_id, actor_principal_id, authenticated_session_id, access_space_id, workspace_id, event_seq UNIQUE;
DEFINE INDEX IF NOT EXISTS model_lane_cloud_event_ledger_run_scope ON model_lane_cloud_event_ledger FIELDS run_id, owner_account_id, actor_principal_id, authenticated_session_id, access_space_id, workspace_id, event_seq;

DEFINE INDEX OVERWRITE model_lane_cloud_authority_idempotency_scope ON model_lane_cloud_authority FIELDS kind, idempotency_key, owner_account_id, actor_principal_id, authenticated_session_id, access_space_id, workspace_id UNIQUE;
DEFINE FIELD OVERWRITE receipt_id ON model_lane_cloud_authority TYPE record<model_lane_cloud_event_ledger>;
DEFINE FIELD OVERWRITE event_id ON model_lane_cloud_event_ledger TYPE string ASSERT $value = record::id($this.id);
DEFINE FIELD OVERWRITE event_ledger_stream_id ON model_lane_cloud_event_ledger TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE payload_hash ON model_lane_cloud_event_ledger TYPE string ASSERT $value = crypto::sha256($this.payload_json);
DEFINE FIELD OVERWRITE authority_resource_id ON model_lane_cloud_authority TYPE record<protected_resources>;
DEFINE FIELD OVERWRITE authority_grant_id ON model_lane_cloud_authority TYPE record<resource_grants>;
DEFINE FIELD OVERWRITE authority_session_id ON model_lane_cloud_authority TYPE record<authenticated_sessions>;
DEFINE FIELD OVERWRITE authority_action ON model_lane_cloud_authority TYPE string;
DEFINE FIELD OVERWRITE authority_capability_id ON model_lane_cloud_authority TYPE string;
DEFINE FIELD OVERWRITE authority_purpose ON model_lane_cloud_authority TYPE string;
DEFINE FIELD OVERWRITE authority_resource_id ON model_lane_cloud_event_ledger TYPE record<protected_resources>;
DEFINE FIELD OVERWRITE authority_grant_id ON model_lane_cloud_event_ledger TYPE record<resource_grants>;
DEFINE FIELD OVERWRITE authority_session_id ON model_lane_cloud_event_ledger TYPE record<authenticated_sessions>;
DEFINE FIELD OVERWRITE authority_action ON model_lane_cloud_event_ledger TYPE string;
DEFINE FIELD OVERWRITE authority_capability_id ON model_lane_cloud_event_ledger TYPE string;
DEFINE FIELD OVERWRITE authority_purpose ON model_lane_cloud_event_ledger TYPE string;

DEFINE FUNCTION OVERWRITE fn::wp012_cloud_attribution($scope: object, $row: object) {
    RETURN $scope.authority_kind = 'account' AND $scope.owner_account_id = $row.owner_account_id
        AND $scope.actor_principal_id = $row.actor_principal_id AND $scope.authenticated_session = $row.authenticated_session_id
        AND $scope.access_space = $row.access_space_id AND $scope.workspace = $row.workspace_id;
};
DEFINE FUNCTION OVERWRITE fn::wp012_cloud_record_identity($row: object, $record: object) {
    IF $record.run_id != $row.run_id OR $record.event_ledger_event_id != $row.event_id
        OR $record.event_ledger_seq != $row.event_seq OR string::trim($record.event_ledger_stream_id) = ''
        OR ($row.kind IN ['projection_plan', 'consent_receipt'] AND ($record.event_stream_version != $row.event_seq OR $record.transaction_seq != $row.event_seq)) { RETURN false; };
    // The pinned canonical consent_scope/target_bindings algorithm returns None.
    // Its derived field is omitted by serde; a supplied derived hash is noncanonical.
    IF $row.kind IN ['projection_plan', 'consent_receipt'] AND $record.target_bindings_hash != NONE { RETURN false; };
    IF $row.kind = 'projection_plan' {
        LET $body = object::from_entries(array::filter(object::entries($record), |$entry| $entry[0] NOT IN ['event_ledger_event_id', 'event_ledger_seq', 'event_stream_version', 'transaction_seq', 'target_bindings_hash', 'projection_plan_hash']));
        RETURN $record.projection_plan_id = $row.aggregate_id AND $row.projection_plan_id = $row.aggregate_id
            AND $record.status = 'active' AND fn::wp012_cloud_attribution($record.export_delegation.source_scope, $row)
            AND $record.projection_plan_hash = crypto::sha256(encoding::json::encode($body));
    };
    IF $row.kind = 'consent_receipt' {
        LET $body = object::from_entries(array::filter(object::entries($record), |$entry| $entry[0] NOT IN ['event_ledger_event_id', 'event_ledger_seq', 'event_stream_version', 'transaction_seq', 'target_bindings_hash', 'consent_receipt_hash']));
        LET $plans = SELECT * FROM model_lane_cloud_authority WHERE kind = 'projection_plan' AND aggregate_id = $record.projection_plan_id
            AND owner_account_id = $row.owner_account_id AND actor_principal_id = $row.actor_principal_id
            AND authenticated_session_id = $row.authenticated_session_id AND access_space_id = $row.access_space_id AND workspace_id = $row.workspace_id;
        IF array::len($plans) != 1 { RETURN false; };
        LET $plan = encoding::json::decode($plans[0].record_json);
        RETURN $record.consent_receipt_id = $row.aggregate_id AND $row.consent_receipt_id = $row.aggregate_id
            AND $row.projection_plan_id = $record.projection_plan_id AND $record.approved = true AND $record.status = 'approved'
            AND fn::wp012_cloud_attribution($record.approver, $row)
            AND $record.consent_receipt_hash = crypto::sha256(encoding::json::encode($body))
            AND $record.projection_plan_hash = $plan.projection_plan_hash AND $record.run_id = $plan.run_id
            AND $record.lane_id = $plan.lane_id AND $record.model_session_id = $plan.model_session_id
            AND $record.provider_kind = $plan.provider_kind AND $record.requested_model_id = $plan.requested_model_id
            AND $record.scope_hash = $plan.scope_hash AND $record.consent_scope = $plan.consent_scope
            AND $record.target_bindings = $plan.target_bindings AND $record.export_posture = $plan.export_posture
            AND $record.retention_policy = $plan.retention_policy AND $record.fan_out_targets = $plan.fan_out_targets
            AND type::datetime($record.valid_from_utc) <= time::now() AND type::datetime($record.valid_until_utc) > time::now();
    };
    IF $row.kind IN ['cloud_run', 'cloud_lane'] {
        LET $consents = SELECT * FROM model_lane_cloud_authority WHERE kind = 'consent_receipt' AND aggregate_id = $record.consent_receipt_ref
            AND owner_account_id = $row.owner_account_id AND actor_principal_id = $row.actor_principal_id
            AND authenticated_session_id = $row.authenticated_session_id AND access_space_id = $row.access_space_id AND workspace_id = $row.workspace_id;
        IF array::len($consents) != 1 { RETURN false; };
        LET $consent = encoding::json::decode($consents[0].record_json);
        IF $consent.run_id != $record.run_id OR $consent.projection_plan_id != $record.projection_plan_ref
            OR $row.projection_plan_id != $record.projection_plan_ref OR $row.consent_receipt_id != $record.consent_receipt_ref { RETURN false; };
        IF $row.kind = 'cloud_run' { RETURN $record.run_id = $row.aggregate_id AND $consent.lane_id IN $record.lane_ids; };
        RETURN $record.lane_id = $row.aggregate_id AND (($consent.consent_scope = 'single_lane'
            AND $record.lane_id = $consent.lane_id AND $record.model_session_id = $consent.model_session_id
            AND $record.provider_kind = $consent.provider_kind AND $record.model_id = $consent.requested_model_id)
            OR ($consent.consent_scope != 'single_lane' AND array::len(array::filter($consent.target_bindings, |$target|
                $target.lane_id = $record.lane_id AND $target.model_session_id = $record.model_session_id
                AND $target.provider_kind = $record.provider_kind AND $target.requested_model_id = $record.model_id
                AND $target.capability_snapshot_ref = $record.effective_capability_snapshot_ref
                AND $target.provider_endpoint_ref = $record.adapter_id)) = 1));
    };
    RETURN $row.kind = 'consent_denial' AND $record.lane_id = $row.aggregate_id
        AND $record.provider_call_attempted = false AND $record.partial_authority_state_created = false;
};
DEFINE FUNCTION OVERWRITE fn::wp012_cloud_checkpoint_dependency_read($row: object) {
    IF $row.kind != 'consent_denial' OR !fn::wp012_model_lane_original_operation($row)
        OR $wp012_original_subject_kind != 'workspace' { RETURN false; };
    IF !(($wp012_original_purpose = 'recovery_checkpoint_record' AND $wp012_original_action = 'create')
        OR ($wp012_original_purpose = 'recovery_execute' AND $wp012_original_action = 'update'
            AND $wp012_original_capability = 'model_lane.recovery.execute')) { RETURN false; };
    IF $wp012_original_purpose = 'recovery_checkpoint_record'
        AND type::is_int($expected_last_event_seq) AND $row.event_seq > $expected_last_event_seq {
        RETURN false;
    };
    IF !fn::wp012_context_coordination_fence($row, 'workspace', $row.workspace_id)
        OR !fn::wp012_context_coordination_fence($row, 'run', $row.run_id)
        OR !fn::wp012_context_coordination_fence($row, 'lane', $row.aggregate_id) {
        THROW 'HSK-CLOUD-CHECKPOINT-PROVENANCE-UNVERIFIABLE';
    };
    LET $lanes = SELECT * FROM model_lane_authority WHERE record_kind = 'lane'
        AND aggregate_id = $row.aggregate_id AND run_id = $row.run_id
        AND owner_account_id = $row.owner_account_id AND actor_principal_id = $row.actor_principal_id
        AND authenticated_session_id = $row.authenticated_session_id AND access_space_id = $row.access_space_id
        AND workspace_id = $row.workspace_id;
    IF array::len($lanes) != 1 { THROW 'HSK-CLOUD-CHECKPOINT-PROVENANCE-UNVERIFIABLE'; };
    LET $lane = encoding::json::decode($lanes[0].record_json);
    IF (type::is_string($lane.document_id) AND !fn::mt120_document_access($lane.document_id, $row.workspace_id, 'read', 'fs.read'))
        OR ($lane.crdt_authority_binding != NONE AND $lane.crdt_authority_binding != NULL
            AND ($lane.crdt_authority_binding.workspace_id != $row.workspace_id
                OR !fn::mt120_document_access($lane.crdt_authority_binding.document_id, $row.workspace_id, 'read', 'fs.read'))) {
        THROW 'HSK-CLOUD-CHECKPOINT-PROVENANCE-UNVERIFIABLE';
    };
    RETURN true;
};

DEFINE FUNCTION OVERWRITE fn::wp012_cloud_checkpoint_denial_membership(
    $run_id: string, $event_ledger_stream_id: string, $expected_last_event_seq: int, $checkpoint_id: string,
    $owner_account_id: string, $actor_principal_id: string, $authenticated_session_id: string,
    $access_space_id: string, $workspace_id: string
) {
    LET $scope = { owner_account_id: $owner_account_id, actor_principal_id: $actor_principal_id,
        authenticated_session_id: $authenticated_session_id, access_space_id: $access_space_id, workspace_id: $workspace_id };
    IF $wp012_original_subject_kind != 'workspace' OR $wp012_original_action != 'create'
        OR $wp012_original_purpose != 'recovery_checkpoint_record'
        OR !fn::wp012_model_lane_original_operation($scope) OR $expected_last_event_seq <= 0
        OR string::trim($run_id) = '' OR string::trim($event_ledger_stream_id) = '' OR string::trim($checkpoint_id) = ''
        OR !fn::wp012_context_coordination_fence($scope, 'workspace', $workspace_id)
        OR !fn::wp012_context_coordination_fence($scope, 'run', $run_id) {
        THROW 'HSK-CLOUD-CHECKPOINT-PROVENANCE-UNVERIFIABLE';
    };
    LET $ledgers = SELECT * FROM model_lane_cloud_event_ledger WHERE aggregate_kind = 'consent_denial'
        AND run_id = $run_id AND owner_account_id = $owner_account_id AND actor_principal_id = $actor_principal_id
        AND authenticated_session_id = $authenticated_session_id AND access_space_id = $access_space_id
        AND workspace_id = $workspace_id AND event_seq <= $expected_last_event_seq ORDER BY event_seq ASC, event_id ASC;
    LET $receipts = array::map($ledgers, |$ledger| {
        LET $candidate = object::extend($ledger, { kind: $ledger.aggregate_kind });
        IF !fn::wp012_cloud_checkpoint_dependency_read($candidate) {
            THROW 'HSK-CLOUD-CHECKPOINT-PROVENANCE-UNVERIFIABLE';
        };
        LET $payload = encoding::json::decode($ledger.payload_json);
        LET $record = $payload.record;
        IF $ledger.event_id != record::id($ledger.id) OR $ledger.event_seq <= 0
            OR $ledger.payload_hash != crypto::sha256($ledger.payload_json)
            OR $ledger.event_ledger_stream_id != $event_ledger_stream_id
            OR !type::is_object($record) OR $payload.dexterity_kernel != 'Dexterity'
            OR $payload.schema_id != 'hsk.model_lane_cloud_consent_denial@1'
            OR $record.run_id != $run_id OR $record.lane_id != $ledger.aggregate_id
            OR $record.event_ledger_stream_id != $event_ledger_stream_id
            OR $record.event_ledger_event_id != $ledger.event_id OR $record.event_ledger_seq != $ledger.event_seq
            OR $record.provider_call_attempted != false OR $record.partial_authority_state_created != false {
            THROW 'HSK-CLOUD-CHECKPOINT-PROVENANCE-UNVERIFIABLE';
        };
        LET $authorities = SELECT * FROM model_lane_cloud_authority WHERE kind = 'consent_denial'
            AND event_id = $ledger.event_id AND run_id = $run_id
            AND owner_account_id = $owner_account_id AND actor_principal_id = $actor_principal_id
            AND authenticated_session_id = $authenticated_session_id AND access_space_id = $access_space_id
            AND workspace_id = $workspace_id;
        IF array::len($authorities) != 1 { THROW 'HSK-CLOUD-CHECKPOINT-PROVENANCE-UNVERIFIABLE'; };
        LET $authority = $authorities[0];
        IF $authority.receipt_id != $ledger.id OR $authority.aggregate_id != $ledger.aggregate_id
            OR $authority.event_seq != $ledger.event_seq OR $authority.event_payload_json != $ledger.payload_json
            OR encoding::json::decode($authority.record_json) != $record
            OR !fn::wp012_cloud_authority_read($authority) {
            THROW 'HSK-CLOUD-CHECKPOINT-PROVENANCE-UNVERIFIABLE';
        };
        RETURN { event_id: $ledger.event_id, event_seq: $ledger.event_seq, payload_hash: $ledger.payload_hash,
            run_id: $run_id, lane_id: $ledger.aggregate_id, event_ledger_stream_id: $ledger.event_ledger_stream_id,
            owner_account_id: $owner_account_id, actor_principal_id: $actor_principal_id,
            authenticated_session_id: $authenticated_session_id, access_space_id: $access_space_id, workspace_id: $workspace_id };
    });
    IF array::len(array::distinct(array::map($receipts, |$receipt| $receipt.event_id))) != array::len($receipts) {
        THROW 'HSK-CLOUD-CHECKPOINT-PROVENANCE-UNVERIFIABLE';
    };
    RETURN { schema_version: 1, observation: 'checkpoint_prewrite', checkpoint_id: $checkpoint_id, run_id: $run_id,
        event_ledger_stream_id: $event_ledger_stream_id, last_event_ledger_seq: $expected_last_event_seq,
        owner_account_id: $owner_account_id, actor_principal_id: $actor_principal_id,
        authenticated_session_id: $authenticated_session_id, access_space_id: $access_space_id, workspace_id: $workspace_id,
        receipts: $receipts };
};

DEFINE FUNCTION OVERWRITE fn::wp012_cloud_authority_read($row: object) {
    IF !(fn::wp012_cloud_source_access($row) OR fn::wp012_cloud_checkpoint_dependency_read($row)) { RETURN false; };
    LET $record = encoding::json::decode($row.record_json);
    LET $ledger = (SELECT * FROM ONLY $row.receipt_id);
    RETURN fn::wp012_cloud_record_identity($row, $record) AND $ledger != NONE
        AND $ledger.aggregate_kind = $row.kind AND $ledger.aggregate_id = $row.aggregate_id AND $ledger.run_id = $row.run_id
        AND $ledger.event_id = $row.event_id AND $ledger.event_seq = $row.event_seq
        AND $ledger.payload_json = $row.event_payload_json AND $ledger.payload_hash = crypto::sha256($ledger.payload_json)
        AND encoding::json::decode($ledger.payload_json).record = $record
        AND $ledger.owner_account_id = $row.owner_account_id AND $ledger.actor_principal_id = $row.actor_principal_id
        AND $ledger.authenticated_session_id = $row.authenticated_session_id AND $ledger.access_space_id = $row.access_space_id
        AND $ledger.workspace_id = $row.workspace_id;
};

DEFINE FIELD OVERWRITE allocation ON model_lane_cloud_event_ledger TYPE object FLEXIBLE VALUE {
    IF $input != NONE { THROW 'HSK-CLOUD-CALLER-ALLOCATION-DENIED'; };
    LET $prepared = encoding::json::decode($this.payload_json);
    LET $record = $prepared.record;
    IF !type::is_object($record) OR $record.event_ledger_event_id != NONE OR $record.event_ledger_seq != NONE
        OR $record.event_stream_version != NONE OR $record.transaction_seq != NONE { THROW 'HSK-CLOUD-CALLER-METADATA-DENIED'; };
    LET $seq = sequence::nextval('kernel_event_sequence');
    LET $metadata = object::extend({ event_ledger_event_id: record::id($this.id), event_ledger_seq: $seq },
        IF $this.aggregate_kind IN ['projection_plan', 'consent_receipt'] { { event_stream_version: $seq, transaction_seq: $seq } } ELSE { {} });
    LET $payload = encoding::json::encode(object::extend($prepared, { record: object::extend($record, $metadata) }));
    RETURN { event_seq: $seq, payload_json: $payload, payload_hash: crypto::sha256($payload) };
} PERMISSIONS FOR select NONE;
DEFINE FIELD OVERWRITE allocation.event_seq ON model_lane_cloud_event_ledger TYPE int;
DEFINE FIELD OVERWRITE allocation.payload_json ON model_lane_cloud_event_ledger TYPE string;
DEFINE FIELD OVERWRITE allocation.payload_hash ON model_lane_cloud_event_ledger TYPE string;
DEFINE FIELD OVERWRITE event_id ON model_lane_cloud_event_ledger TYPE string VALUE {
    IF $input != NONE { THROW 'HSK-CLOUD-CALLER-METADATA-DENIED'; }; RETURN record::id($this.id);
} ASSERT $value = record::id($this.id);
DEFINE FIELD OVERWRITE event_seq ON model_lane_cloud_event_ledger TYPE int VALUE {
    IF $input != NONE { THROW 'HSK-CLOUD-CALLER-SEQUENCE-DENIED'; }; RETURN $this.allocation.event_seq;
} ASSERT $value > 0 AND $value = $this.allocation.event_seq;
DEFINE FIELD OVERWRITE payload_json ON model_lane_cloud_event_ledger TYPE string VALUE $this.allocation.payload_json;
DEFINE FIELD OVERWRITE payload_hash ON model_lane_cloud_event_ledger TYPE string VALUE {
    IF $input != NONE { THROW 'HSK-CLOUD-CALLER-METADATA-DENIED'; }; RETURN $this.allocation.payload_hash;
} ASSERT $value = $this.allocation.payload_hash;
