DEFINE TABLE OVERWRITE palmistry_durable_verifier SCHEMAFULL PERMISSIONS FOR select WHERE fn::wp012_palmistry_watcher_verifier($this) OR fn::wp012_palmistry_watcher_read_verifier($this) FOR create WHERE fn::wp012_palmistry_watcher_verifier($this) AND fn::wp012_palmistry_admission_row($this, 'verifier') FOR update, delete NONE;
DEFINE FIELD OVERWRITE session_id ON TABLE palmistry_durable_verifier TYPE uuid;
DEFINE FIELD OVERWRITE launch_nonce ON TABLE palmistry_durable_verifier TYPE uuid;
DEFINE FIELD OVERWRITE parent_pid ON TABLE palmistry_durable_verifier TYPE int ASSERT $value > 0;
DEFINE FIELD OVERWRITE watcher_pid ON TABLE palmistry_durable_verifier TYPE int ASSERT $value > 0;
DEFINE FIELD OVERWRITE watcher_creation_time_100ns ON TABLE palmistry_durable_verifier TYPE int ASSERT $value >= 0;
DEFINE FIELD OVERWRITE process_uuid ON TABLE palmistry_durable_verifier TYPE uuid;
DEFINE FIELD OVERWRITE executable_sha256 ON TABLE palmistry_durable_verifier TYPE string ASSERT string::len($value) = 64;
DEFINE FIELD OVERWRITE verifying_key_hex ON TABLE palmistry_durable_verifier TYPE string ASSERT string::len($value) = 64;
DEFINE FIELD OVERWRITE retired_at ON TABLE palmistry_durable_verifier TYPE option<datetime>;
DEFINE FIELD OVERWRITE event_ledger_event_id ON TABLE palmistry_durable_verifier TYPE record<kernel_event_ledger> ASSERT record::exists($value) REFERENCE ON DELETE REJECT;
DEFINE FIELD OVERWRITE owner_account_id ON TABLE palmistry_durable_verifier TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE actor_principal_id ON TABLE palmistry_durable_verifier TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE authenticated_session_id ON TABLE palmistry_durable_verifier TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE access_space_id ON TABLE palmistry_durable_verifier TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE workspace_id ON TABLE palmistry_durable_verifier TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE storage_authority ON TABLE palmistry_durable_verifier TYPE 'embedded_surrealdb' DEFAULT 'embedded_surrealdb';
DEFINE FIELD OVERWRITE created_at ON TABLE palmistry_durable_verifier TYPE datetime DEFAULT time::now();
DEFINE INDEX OVERWRITE palmistry_verifier_scope_identity_unique ON TABLE palmistry_durable_verifier FIELDS owner_account_id, actor_principal_id, authenticated_session_id, access_space_id, workspace_id, session_id, launch_nonce, process_uuid UNIQUE;
DEFINE INDEX OVERWRITE palmistry_verifier_scope_session_active_idx ON TABLE palmistry_durable_verifier FIELDS owner_account_id, actor_principal_id, authenticated_session_id, access_space_id, workspace_id, session_id, retired_at;

DEFINE TABLE OVERWRITE palmistry_watcher_custodian SCHEMAFULL
    PERMISSIONS FOR select WHERE fn::wp012_palmistry_watcher_custodian($this) OR fn::wp012_palmistry_watcher_read_custodian($this) FOR create, update, delete NONE;
DEFINE FIELD OVERWRITE registration_operation_id ON TABLE palmistry_watcher_custodian TYPE uuid;
DEFINE FIELD OVERWRITE process_uuid ON TABLE palmistry_watcher_custodian TYPE uuid;
DEFINE FIELD OVERWRITE diagnostic_session_id ON TABLE palmistry_watcher_custodian TYPE uuid;
DEFINE FIELD OVERWRITE launch_nonce ON TABLE palmistry_watcher_custodian TYPE uuid;
DEFINE FIELD OVERWRITE parent_pid ON TABLE palmistry_watcher_custodian TYPE int;
DEFINE FIELD OVERWRITE watcher_pid ON TABLE palmistry_watcher_custodian TYPE int;
DEFINE FIELD OVERWRITE parent_creation_time_100ns ON TABLE palmistry_watcher_custodian TYPE int;
DEFINE FIELD OVERWRITE watcher_creation_time_100ns ON TABLE palmistry_watcher_custodian TYPE int;
DEFINE FIELD OVERWRITE executable_sha256 ON TABLE palmistry_watcher_custodian TYPE string;
DEFINE FIELD OVERWRITE verifying_key_hex ON TABLE palmistry_watcher_custodian TYPE string;
DEFINE FIELD OVERWRITE original_started_at ON TABLE palmistry_watcher_custodian TYPE datetime;
DEFINE FIELD OVERWRITE original_start_hash ON TABLE palmistry_watcher_custodian TYPE string;
DEFINE FIELD OVERWRITE verifier_record_id ON TABLE palmistry_watcher_custodian TYPE string;
DEFINE FIELD OVERWRITE owner_account_id ON TABLE palmistry_watcher_custodian TYPE string;
DEFINE FIELD OVERWRITE actor_principal_id ON TABLE palmistry_watcher_custodian TYPE string;
DEFINE FIELD OVERWRITE authenticated_session_id ON TABLE palmistry_watcher_custodian TYPE string;
DEFINE FIELD OVERWRITE access_space_id ON TABLE palmistry_watcher_custodian TYPE string;
DEFINE FIELD OVERWRITE workspace_id ON TABLE palmistry_watcher_custodian TYPE string;
DEFINE FIELD OVERWRITE authority_resource_id ON TABLE palmistry_watcher_custodian TYPE record<protected_resources>;
DEFINE FIELD OVERWRITE authority_parent_resource_id ON TABLE palmistry_watcher_custodian TYPE record<protected_resources>;
DEFINE FIELD OVERWRITE original_start ON TABLE palmistry_watcher_custodian TYPE object FLEXIBLE;
DEFINE FIELD OVERWRITE original_start.* ON TABLE palmistry_watcher_custodian TYPE any;
DEFINE FIELD OVERWRITE signed_native_registration_evidence ON TABLE palmistry_watcher_custodian TYPE object FLEXIBLE;
DEFINE FIELD OVERWRITE signed_native_registration_evidence.* ON TABLE palmistry_watcher_custodian TYPE any;
DEFINE INDEX OVERWRITE palmistry_watcher_custodian_identity_unique ON TABLE palmistry_watcher_custodian FIELDS process_uuid UNIQUE;
DEFINE INDEX OVERWRITE palmistry_watcher_custodian_registration_unique ON TABLE palmistry_watcher_custodian FIELDS registration_operation_id UNIQUE;

DEFINE TABLE OVERWRITE palmistry_watcher_adoption SCHEMAFULL
    PERMISSIONS FOR select WHERE fn::wp012_palmistry_watcher_adoption($this, false) OR fn::wp012_palmistry_watcher_read_custodian($this) FOR create WHERE fn::wp012_palmistry_watcher_adoption($this, true) FOR update, delete NONE;
DEFINE FIELD OVERWRITE operation_id ON TABLE palmistry_watcher_adoption TYPE uuid;
DEFINE FIELD OVERWRITE process_uuid ON TABLE palmistry_watcher_adoption TYPE uuid;
DEFINE FIELD OVERWRITE diagnostic_session_id ON TABLE palmistry_watcher_adoption TYPE uuid;
DEFINE FIELD OVERWRITE launch_nonce ON TABLE palmistry_watcher_adoption TYPE uuid;
DEFINE FIELD OVERWRITE original_started_at ON TABLE palmistry_watcher_adoption TYPE datetime;
DEFINE FIELD OVERWRITE original_custodian ON TABLE palmistry_watcher_adoption TYPE record<palmistry_watcher_custodian> ASSERT record::exists($value) REFERENCE ON DELETE REJECT;
DEFINE FIELD OVERWRITE verifier ON TABLE palmistry_watcher_adoption TYPE record<palmistry_durable_verifier> ASSERT record::exists($value) REFERENCE ON DELETE REJECT;
DEFINE FIELD OVERWRITE event_ledger_event_id ON TABLE palmistry_watcher_adoption TYPE record<kernel_event_ledger> ASSERT record::exists($value) REFERENCE ON DELETE REJECT;
DEFINE FIELD OVERWRITE input_hash ON TABLE palmistry_watcher_adoption TYPE string ASSERT string::len($value) = 64;
DEFINE FIELD OVERWRITE owner_account_id ON TABLE palmistry_watcher_adoption TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE actor_principal_id ON TABLE palmistry_watcher_adoption TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE authenticated_session_id ON TABLE palmistry_watcher_adoption TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE access_space_id ON TABLE palmistry_watcher_adoption TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE workspace_id ON TABLE palmistry_watcher_adoption TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE authority_resource_id ON TABLE palmistry_watcher_adoption TYPE record<protected_resources>;
DEFINE FIELD OVERWRITE authority_parent_resource_id ON TABLE palmistry_watcher_adoption TYPE record<protected_resources>;
DEFINE FIELD OVERWRITE authority_grant_id ON TABLE palmistry_watcher_adoption TYPE record<resource_grants>;
DEFINE FIELD OVERWRITE authority_session_id ON TABLE palmistry_watcher_adoption TYPE record<authenticated_sessions>;
DEFINE FIELD OVERWRITE authority_capability_id ON TABLE palmistry_watcher_adoption TYPE 'diagnostics.watcher.adopt_existing';
DEFINE FIELD OVERWRITE authority_action ON TABLE palmistry_watcher_adoption TYPE 'reconcile';
DEFINE INDEX OVERWRITE palmistry_watcher_adoption_operation_unique ON TABLE palmistry_watcher_adoption FIELDS operation_id UNIQUE;
DEFINE FIELD OVERWRITE start_record_id ON TABLE palmistry_watcher_adoption TYPE string;

-- Private access authenticates only; the sealed worker installs and consumes permits separately.
DEFINE TABLE OVERWRITE palmistry_watcher_admission_config SCHEMAFULL
    PERMISSIONS FOR select WHERE $access IN ['palmistry_watcher_admission', 'palmistry_watcher_permit_installer'] AND fn::mt109_live_session()
    FOR create, update, delete NONE;
DEFINE FIELD OVERWRITE key_version ON TABLE palmistry_watcher_admission_config TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE admission_verifier ON TABLE palmistry_watcher_admission_config TYPE string ASSERT string::len($value) = 64;
DEFINE FIELD OVERWRITE installer_verifier ON TABLE palmistry_watcher_admission_config TYPE string ASSERT string::len($value) = 64;
DEFINE FIELD OVERWRITE not_after ON TABLE palmistry_watcher_admission_config TYPE datetime;
DEFINE FIELD OVERWRITE revoked_at ON TABLE palmistry_watcher_admission_config TYPE option<datetime>;
DEFINE INDEX OVERWRITE palmistry_watcher_admission_config_version ON TABLE palmistry_watcher_admission_config FIELDS key_version UNIQUE;

DEFINE FUNCTION OVERWRITE fn::wp012_palmistry_admission_signin_scope($session: record<authenticated_sessions>, $scope: object, $resource: record<protected_resources>, $parent: record<protected_resources>, $grant: record<resource_grants>) {
    IF $session.revoked_at != NONE OR $session.expires_at <= time::now()
        OR $session.account_id.status != 'enabled' OR $session.principal_id.status != 'enabled'
        OR $session.access_space_id.status != 'active' OR $session.principal_id.account_id != $session.account_id
        OR $session.access_space_id.account_id != $session.account_id
        OR $session.account_revocation_epoch != $session.account_id.revocation_epoch
        OR $session.principal_revocation_epoch != $session.principal_id.revocation_epoch
        OR $session.space_revocation_epoch != $session.access_space_id.revocation_epoch
        OR $session.policy_version < $session.account_id.policy_version OR $session.policy_version < $session.principal_id.policy_version
        OR $session.policy_version < $session.access_space_id.policy_version { RETURN false; };
    IF $session.principal_id.capability_profile_id != 'Operator'
        OR !($session.delegated_capabilities CONTAINS 'diagnostics.watcher.adopt_existing')
        OR !($session.principal_id.delegated_capabilities CONTAINS 'diagnostics.watcher.adopt_existing')
        OR record::id($session) != $scope.authenticated_session_id
        OR record::id($session.account_id) != $scope.owner_account_id
        OR record::id($session.principal_id) != $scope.actor_principal_id
        OR record::id($session.access_space_id) != $scope.access_space_id { RETURN false; };
    LET $parents = SELECT VALUE id FROM protected_resources WHERE id = $parent AND resource_kind = 'workspace'
        AND external_resource_id = $scope.workspace_id AND lifecycle_state = 'active'
        AND owner_account_id = $session.account_id AND access_space_id = $session.access_space_id LIMIT 2;
    IF array::len($parents) != 1 OR $resource.resource_kind != 'palmistry_watcher'
        OR $resource.lifecycle_state != 'active' OR $resource.parent_resource_id != $parent
        OR $resource.owner_account_id != $session.account_id OR $resource.access_space_id != $session.access_space_id
        OR $resource.created_by_principal_id != $session.principal_id { RETURN false; };
    LET $grants = SELECT VALUE id FROM resource_grants WHERE id = $grant AND resource_id = $resource
        AND account_id = $session.account_id AND principal_id = $session.principal_id
        AND access_space_id = $session.access_space_id AND status = 'active' AND revoked_at = NONE
        AND (expires_at = NONE OR expires_at > time::now()) AND actions CONTAINS 'reconcile'
        AND capability_ids CONTAINS 'diagnostics.watcher.adopt_existing'
        AND delegation_chain = $session.delegation_chain AND policy_version <= $session.policy_version
        AND policy_version >= $resource.policy_version AND policy_version >= $parent.policy_version LIMIT 2;
    RETURN array::len($grants) = 1;
};

DEFINE ACCESS OVERWRITE palmistry_watcher_admission ON DATABASE TYPE RECORD
    SIGNIN (SELECT * FROM authenticated_sessions WHERE id = type::record('authenticated_sessions', $scope.authenticated_session_id)
        AND token_hash = $token_hash AND channel_binding_hash = $channel_binding_hash
        AND channel_binding_hash != NONE
        AND fn::wp012_palmistry_admission_signin_scope(id, $scope, $resource, $parent, $grant)
        AND array::len(SELECT VALUE id FROM palmistry_watcher_admission_config WHERE key_version = $key_version
            AND revoked_at = NONE AND not_after > time::now()
            AND admission_verifier = crypto::sha256('handshake-watcher-admission-v1|palmistry_watcher_admission|' + $key_version + '|' + $installer_secret_hex) LIMIT 2) = 1 LIMIT 1)
    DURATION FOR TOKEN 5m, FOR SESSION 5m;
DEFINE ACCESS OVERWRITE palmistry_watcher_permit_installer ON DATABASE TYPE RECORD
    SIGNIN (SELECT * FROM authenticated_sessions WHERE id = type::record('authenticated_sessions', $scope.authenticated_session_id)
        AND token_hash = $token_hash AND channel_binding_hash = $channel_binding_hash
        AND channel_binding_hash != NONE
        AND fn::wp012_palmistry_admission_signin_scope(id, $scope, $resource, $parent, $grant)
        AND array::len(SELECT VALUE id FROM palmistry_watcher_admission_config WHERE key_version = $key_version
            AND revoked_at = NONE AND not_after > time::now()
            AND installer_verifier = crypto::sha256('handshake-watcher-admission-v1|palmistry_watcher_permit_installer|' + $key_version + '|' + $installer_secret_hex) LIMIT 2) = 1 LIMIT 1)
    DURATION FOR TOKEN 5m, FOR SESSION 5m;

DEFINE FUNCTION OVERWRITE fn::wp012_palmistry_admission_permit_current($permit: object) {
    IF !fn::mt109_live_session() OR $permit.expires_at <= time::now()
        OR $permit.a_auth != $auth.id OR $permit.a_token_id != <string>$auth.id
        OR $permit.owner_account_id != <string>record::id($auth.account_id)
        OR $permit.actor_principal_id != <string>record::id($auth.principal_id)
        OR $permit.authenticated_session_id != <string>record::id($auth.id)
        OR $permit.access_space_id != <string>record::id($auth.access_space_id)
        OR $permit.grant_version != $permit.authority_grant_id.grant_version { RETURN false; };
    LET $config = SELECT VALUE id FROM palmistry_watcher_admission_config WHERE key_version = $permit.key_version
        AND revoked_at = NONE AND not_after >= $permit.expires_at AND not_after > time::now() LIMIT 2;
    RETURN array::len($config) = 1 AND fn::wp012_palmistry_watcher_record($permit);
};
DEFINE FUNCTION OVERWRITE fn::wp012_palmistry_admission_actual_identity($permit: object) {
    RETURN $access = 'palmistry_watcher_admission' AND $token.AC = $access
        AND $token.ID = <string>$auth.id AND $token.jti = $permit.a_jti
        AND $permit.a_access = $access AND $permit.a_auth = $auth.id
        AND $permit.a_token_id = $token.ID AND fn::wp012_palmistry_admission_permit_current($permit);
};
DEFINE FUNCTION OVERWRITE fn::wp012_palmistry_admission_install($permit: object) {
    RETURN $access = 'palmistry_watcher_permit_installer' AND $token.AC = $access AND $token.ID = <string>$auth.id
        AND $permit.state = 'issued' AND $permit.purpose = 'adopt'
        AND $permit.a_access = 'palmistry_watcher_admission' AND string::trim($permit.a_jti) != ''
        AND $permit.operation_id = type::uuid($permit.manifest.operation_id)
        AND $permit.manifest.purpose = $permit.purpose AND $permit.manifest.schema_version = 1
        AND $permit.manifest.owner_account_id = $permit.owner_account_id
        AND $permit.manifest.actor_principal_id = $permit.actor_principal_id
        AND $permit.manifest.authenticated_session_id = $permit.authenticated_session_id
        AND $permit.manifest.access_space_id = $permit.access_space_id AND $permit.manifest.workspace_id = $permit.workspace_id
        AND type::datetime($permit.manifest.expires_at) = $permit.expires_at
        AND fn::wp012_palmistry_admission_permit_current($permit);
};
DEFINE TABLE OVERWRITE palmistry_watcher_operation_permit SCHEMAFULL
    PERMISSIONS FOR select WHERE ($access = 'palmistry_watcher_permit_installer' AND fn::wp012_palmistry_admission_permit_current($this))
        OR fn::wp012_palmistry_admission_actual_identity($this)
        OR fn::wp012_palmistry_watcher_custodian($this)
        OR fn::wp012_palmistry_watcher_read_custodian($this)
    FOR create WHERE fn::wp012_palmistry_admission_install($this)
    FOR update WHERE fn::wp012_palmistry_admission_actual_identity($this)
    FOR delete NONE;
DEFINE FIELD OVERWRITE operation_id ON TABLE palmistry_watcher_operation_permit TYPE uuid;
DEFINE FIELD OVERWRITE purpose ON TABLE palmistry_watcher_operation_permit TYPE string ASSERT $value IN ['adopt', 'register'];
DEFINE FIELD OVERWRITE state ON TABLE palmistry_watcher_operation_permit TYPE string ASSERT $value IN ['issued', 'consuming', 'finalized'];
DEFINE FIELD OVERWRITE a_access ON TABLE palmistry_watcher_operation_permit TYPE 'palmistry_watcher_admission';
DEFINE FIELD OVERWRITE a_auth ON TABLE palmistry_watcher_operation_permit TYPE record<authenticated_sessions>;
DEFINE FIELD OVERWRITE a_token_id ON TABLE palmistry_watcher_operation_permit TYPE string;
DEFINE FIELD OVERWRITE a_jti ON TABLE palmistry_watcher_operation_permit TYPE string;
DEFINE FIELD OVERWRITE key_version ON TABLE palmistry_watcher_operation_permit TYPE string;
DEFINE FIELD OVERWRITE expires_at ON TABLE palmistry_watcher_operation_permit TYPE datetime;
DEFINE FIELD OVERWRITE manifest_hash ON TABLE palmistry_watcher_operation_permit TYPE string ASSERT string::len($value) = 64;
DEFINE FIELD OVERWRITE manifest ON TABLE palmistry_watcher_operation_permit TYPE object FLEXIBLE;
DEFINE FIELD OVERWRITE manifest.* ON TABLE palmistry_watcher_operation_permit TYPE any;
DEFINE FIELD OVERWRITE owner_account_id ON TABLE palmistry_watcher_operation_permit TYPE string;
DEFINE FIELD OVERWRITE actor_principal_id ON TABLE palmistry_watcher_operation_permit TYPE string;
DEFINE FIELD OVERWRITE authenticated_session_id ON TABLE palmistry_watcher_operation_permit TYPE string;
DEFINE FIELD OVERWRITE access_space_id ON TABLE palmistry_watcher_operation_permit TYPE string;
DEFINE FIELD OVERWRITE workspace_id ON TABLE palmistry_watcher_operation_permit TYPE string;
DEFINE FIELD OVERWRITE authority_resource_id ON TABLE palmistry_watcher_operation_permit TYPE record<protected_resources>;
DEFINE FIELD OVERWRITE authority_parent_resource_id ON TABLE palmistry_watcher_operation_permit TYPE record<protected_resources>;
DEFINE FIELD OVERWRITE authority_grant_id ON TABLE palmistry_watcher_operation_permit TYPE record<resource_grants>;
DEFINE FIELD OVERWRITE authority_session_id ON TABLE palmistry_watcher_operation_permit TYPE record<authenticated_sessions>;
DEFINE FIELD OVERWRITE authority_capability_id ON TABLE palmistry_watcher_operation_permit TYPE 'diagnostics.watcher.adopt_existing';
DEFINE FIELD OVERWRITE authority_action ON TABLE palmistry_watcher_operation_permit TYPE 'reconcile';
DEFINE FIELD OVERWRITE grant_version ON TABLE palmistry_watcher_operation_permit TYPE int;
DEFINE INDEX OVERWRITE palmistry_watcher_operation_permit_once ON TABLE palmistry_watcher_operation_permit FIELDS purpose, operation_id UNIQUE;
DEFINE INDEX OVERWRITE palmistry_watcher_operation_permit_identity ON TABLE palmistry_watcher_operation_permit FIELDS a_access, a_auth, a_jti UNIQUE;
DEFINE EVENT OVERWRITE palmistry_watcher_operation_permit_immutable ON TABLE palmistry_watcher_operation_permit WHEN $event = 'UPDATE' THEN {
    LET $before_body = object::from_entries(array::filter(object::entries($before), |$entry| { $entry[0] != 'state' }));
    LET $after_body = object::from_entries(array::filter(object::entries($after), |$entry| { $entry[0] != 'state' }));
    IF $before_body != $after_body OR !(($before.state = 'issued' AND $after.state = 'consuming') OR ($before.state = 'consuming' AND $after.state = 'finalized')) {
        THROW 'HSK-403-WATCHER-PERMIT-TRANSITION';
    };
};
