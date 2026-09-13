DEFINE TABLE IF NOT EXISTS local_accounts TYPE NORMAL SCHEMAFULL
  PERMISSIONS FOR select WHERE id = $auth.account_id FOR create, update, delete NONE;
DEFINE FIELD IF NOT EXISTS account_key ON local_accounts TYPE string ASSERT string::len($value) > 0 PERMISSIONS NONE;
DEFINE FIELD IF NOT EXISTS account_role ON local_accounts TYPE string ASSERT $value IN ['Owner', 'Admin', 'Member'];
DEFINE FIELD IF NOT EXISTS status ON local_accounts TYPE string ASSERT $value IN ['enabled', 'disabled'];
DEFINE FIELD IF NOT EXISTS revocation_epoch ON local_accounts TYPE int ASSERT $value >= 0;
DEFINE FIELD IF NOT EXISTS policy_version ON local_accounts TYPE int ASSERT $value >= 1;
DEFINE FIELD IF NOT EXISTS created_at ON local_accounts TYPE datetime;
DEFINE FIELD IF NOT EXISTS updated_at ON local_accounts TYPE datetime;
DEFINE INDEX IF NOT EXISTS local_accounts_account_key_unique ON local_accounts FIELDS account_key UNIQUE;

DEFINE TABLE IF NOT EXISTS principals TYPE NORMAL SCHEMAFULL
  PERMISSIONS FOR select WHERE id = $auth.principal_id FOR create, update, delete NONE;
DEFINE FIELD IF NOT EXISTS principal_key ON principals TYPE string ASSERT string::len($value) > 0 PERMISSIONS NONE;
DEFINE FIELD IF NOT EXISTS account_id ON principals TYPE record<local_accounts>;
DEFINE FIELD IF NOT EXISTS principal_kind ON principals TYPE string ASSERT $value IN ['human_account', 'local_model', 'remote_model', 'spawned_agent', 'mcp_client', 'service_identity', 'visitor_pass', 'break_glass_operator'];
DEFINE FIELD IF NOT EXISTS actor_kind ON principals TYPE string ASSERT string::len($value) > 0;
DEFINE FIELD IF NOT EXISTS actor_id ON principals TYPE string ASSERT string::len($value) > 0;
DEFINE FIELD IF NOT EXISTS capability_profile_id ON principals TYPE string ASSERT string::len($value) > 0;
DEFINE FIELD IF NOT EXISTS delegated_capabilities ON principals TYPE array<string>;
DEFINE FIELD IF NOT EXISTS status ON principals TYPE string ASSERT $value IN ['enabled', 'disabled'];
DEFINE FIELD IF NOT EXISTS revocation_epoch ON principals TYPE int ASSERT $value >= 0;
DEFINE FIELD IF NOT EXISTS policy_version ON principals TYPE int ASSERT $value >= 1;
DEFINE FIELD IF NOT EXISTS created_at ON principals TYPE datetime;
DEFINE FIELD IF NOT EXISTS updated_at ON principals TYPE datetime;
DEFINE INDEX IF NOT EXISTS principals_principal_key_unique ON principals FIELDS principal_key UNIQUE;
DEFINE INDEX IF NOT EXISTS principals_account_idx ON principals FIELDS account_id;

DEFINE TABLE IF NOT EXISTS access_spaces TYPE NORMAL SCHEMAFULL
  PERMISSIONS FOR select WHERE account_id = $auth.account_id AND id = $auth.access_space_id FOR create, update, delete NONE;
DEFINE FIELD IF NOT EXISTS space_key ON access_spaces TYPE string ASSERT string::len($value) > 0 PERMISSIONS NONE;
DEFINE FIELD IF NOT EXISTS account_id ON access_spaces TYPE record<local_accounts>;
DEFINE FIELD IF NOT EXISTS name ON access_spaces TYPE string ASSERT string::len($value) > 0;
DEFINE FIELD IF NOT EXISTS status ON access_spaces TYPE string ASSERT $value IN ['active', 'disabled'];
DEFINE FIELD IF NOT EXISTS revocation_epoch ON access_spaces TYPE int ASSERT $value >= 0;
DEFINE FIELD IF NOT EXISTS policy_version ON access_spaces TYPE int ASSERT $value >= 1;
DEFINE FIELD IF NOT EXISTS created_at ON access_spaces TYPE datetime;
DEFINE FIELD IF NOT EXISTS updated_at ON access_spaces TYPE datetime;
DEFINE INDEX IF NOT EXISTS access_spaces_key_unique ON access_spaces FIELDS space_key UNIQUE;
DEFINE INDEX IF NOT EXISTS access_spaces_account_idx ON access_spaces FIELDS account_id;

DEFINE TABLE IF NOT EXISTS authenticated_sessions TYPE NORMAL SCHEMAFULL
  PERMISSIONS FOR select WHERE id = $auth.id FOR create, update, delete NONE;
DEFINE FIELD IF NOT EXISTS account_id ON authenticated_sessions TYPE record<local_accounts>;
DEFINE FIELD IF NOT EXISTS principal_id ON authenticated_sessions TYPE record<principals>;
DEFINE FIELD IF NOT EXISTS access_space_id ON authenticated_sessions TYPE record<access_spaces>;
DEFINE FIELD IF NOT EXISTS token_hash ON authenticated_sessions TYPE string ASSERT string::len($value) = 64 PERMISSIONS NONE;
DEFINE FIELD IF NOT EXISTS channel_binding_hash ON authenticated_sessions TYPE option<string> PERMISSIONS NONE;
DEFINE FIELD IF NOT EXISTS authentication_strength ON authenticated_sessions TYPE string;
DEFINE FIELD IF NOT EXISTS delegated_capabilities ON authenticated_sessions TYPE array<string>;
DEFINE FIELD IF NOT EXISTS delegation_chain ON authenticated_sessions TYPE array<string>;
DEFINE FIELD IF NOT EXISTS account_revocation_epoch ON authenticated_sessions TYPE int;
DEFINE FIELD IF NOT EXISTS principal_revocation_epoch ON authenticated_sessions TYPE int;
DEFINE FIELD IF NOT EXISTS space_revocation_epoch ON authenticated_sessions TYPE int;
DEFINE FIELD IF NOT EXISTS policy_version ON authenticated_sessions TYPE int ASSERT $value >= 1;
DEFINE FIELD IF NOT EXISTS issued_at ON authenticated_sessions TYPE datetime;
DEFINE FIELD IF NOT EXISTS expires_at ON authenticated_sessions TYPE datetime;
DEFINE FIELD IF NOT EXISTS revoked_at ON authenticated_sessions TYPE option<datetime>;
DEFINE INDEX IF NOT EXISTS authenticated_sessions_token_hash_unique ON authenticated_sessions FIELDS token_hash UNIQUE;
DEFINE INDEX IF NOT EXISTS authenticated_sessions_principal_idx ON authenticated_sessions FIELDS principal_id;

DEFINE TABLE IF NOT EXISTS session_exchange_credentials TYPE NORMAL SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD IF NOT EXISTS account_id ON session_exchange_credentials TYPE record<local_accounts>;
DEFINE FIELD IF NOT EXISTS principal_id ON session_exchange_credentials TYPE record<principals>;
DEFINE FIELD IF NOT EXISTS access_space_id ON session_exchange_credentials TYPE record<access_spaces>;
DEFINE FIELD IF NOT EXISTS token_hash ON session_exchange_credentials TYPE string ASSERT string::len($value) = 64 PERMISSIONS NONE;
DEFINE FIELD IF NOT EXISTS status ON session_exchange_credentials TYPE string ASSERT $value IN ['active', 'consumed', 'revoked'];
DEFINE FIELD IF NOT EXISTS expires_at ON session_exchange_credentials TYPE datetime;
DEFINE FIELD IF NOT EXISTS revoked_at ON session_exchange_credentials TYPE option<datetime>;
DEFINE FIELD IF NOT EXISTS created_at ON session_exchange_credentials TYPE datetime;
DEFINE INDEX IF NOT EXISTS session_exchange_credentials_token_unique ON session_exchange_credentials FIELDS token_hash UNIQUE;
DEFINE INDEX IF NOT EXISTS session_exchange_credentials_principal_idx ON session_exchange_credentials FIELDS principal_id, access_space_id;

DEFINE TABLE IF NOT EXISTS protected_resources TYPE NORMAL SCHEMAFULL
  PERMISSIONS FOR select WHERE
    lifecycle_state = 'active'
    AND owner_account_id = $auth.account_id
    AND access_space_id = $auth.access_space_id
    AND owner_account_id.status = 'enabled'
    AND created_by_principal_id.status = 'enabled'
    AND access_space_id.status = 'active'
    AND array::len((SELECT id FROM resource_grants WHERE resource_id = $parent.id AND status = 'active' AND revoked_at = NONE AND (expires_at = NONE OR expires_at > time::now()) AND account_id = $auth.account_id AND principal_id = $auth.principal_id AND access_space_id = $auth.access_space_id)) > 0,
  FOR create, update, delete NONE;
DEFINE FIELD IF NOT EXISTS resource_kind ON protected_resources TYPE string ASSERT string::len($value) > 0;
DEFINE FIELD IF NOT EXISTS external_resource_id ON protected_resources TYPE string ASSERT string::len($value) > 0 PERMISSIONS NONE;
DEFINE FIELD IF NOT EXISTS owner_account_id ON protected_resources TYPE record<local_accounts>;
DEFINE FIELD IF NOT EXISTS created_by_principal_id ON protected_resources TYPE record<principals>;
DEFINE FIELD IF NOT EXISTS created_in_session_id ON protected_resources TYPE option<record<authenticated_sessions>>;
DEFINE FIELD IF NOT EXISTS access_space_id ON protected_resources TYPE record<access_spaces>;
DEFINE FIELD IF NOT EXISTS parent_resource_id ON protected_resources TYPE option<record<protected_resources>>;
DEFINE FIELD IF NOT EXISTS schema_version ON protected_resources TYPE int ASSERT $value >= 1;
DEFINE FIELD IF NOT EXISTS lifecycle_state ON protected_resources TYPE string ASSERT $value IN ['active', 'quarantined', 'deleted'];
DEFINE FIELD IF NOT EXISTS policy_version ON protected_resources TYPE int ASSERT $value >= 1;
DEFINE FIELD IF NOT EXISTS classification ON protected_resources TYPE string ASSERT string::len($value) > 0;
DEFINE FIELD IF NOT EXISTS storage_locator_hash ON protected_resources TYPE string ASSERT string::len($value) = 64 PERMISSIONS NONE;
DEFINE FIELD IF NOT EXISTS created_at ON protected_resources TYPE datetime;
DEFINE FIELD IF NOT EXISTS updated_at ON protected_resources TYPE datetime;
DEFINE INDEX IF NOT EXISTS protected_resources_external_unique ON protected_resources FIELDS resource_kind, external_resource_id UNIQUE;
DEFINE INDEX IF NOT EXISTS protected_resources_owner_idx ON protected_resources FIELDS owner_account_id, access_space_id;

DEFINE TABLE IF NOT EXISTS resource_grants TYPE NORMAL SCHEMAFULL
  PERMISSIONS FOR select WHERE
    status = 'active'
    AND account_id = $auth.account_id
    AND principal_id = $auth.principal_id
    AND access_space_id = $auth.access_space_id,
  FOR create, update, delete NONE;
DEFINE FIELD IF NOT EXISTS account_id ON resource_grants TYPE record<local_accounts>;
DEFINE FIELD IF NOT EXISTS principal_id ON resource_grants TYPE record<principals>;
DEFINE FIELD IF NOT EXISTS access_space_id ON resource_grants TYPE record<access_spaces>;
DEFINE FIELD IF NOT EXISTS resource_id ON resource_grants TYPE record<protected_resources>;
DEFINE FIELD IF NOT EXISTS actions ON resource_grants TYPE array<string>;
DEFINE FIELD IF NOT EXISTS capability_ids ON resource_grants TYPE array<string>;
DEFINE FIELD IF NOT EXISTS delegation_chain ON resource_grants TYPE array<string>;
DEFINE FIELD IF NOT EXISTS status ON resource_grants TYPE string ASSERT $value IN ['active', 'revoked'];
DEFINE FIELD IF NOT EXISTS grant_version ON resource_grants TYPE int ASSERT $value >= 1;
DEFINE FIELD IF NOT EXISTS policy_version ON resource_grants TYPE int ASSERT $value >= 1;
DEFINE FIELD IF NOT EXISTS expires_at ON resource_grants TYPE option<datetime>;
DEFINE FIELD IF NOT EXISTS revoked_at ON resource_grants TYPE option<datetime>;
DEFINE FIELD IF NOT EXISTS created_at ON resource_grants TYPE datetime;
DEFINE FIELD IF NOT EXISTS updated_at ON resource_grants TYPE datetime;
DEFINE INDEX IF NOT EXISTS resource_grants_exact_idx ON resource_grants FIELDS account_id, principal_id, access_space_id, resource_id;

DEFINE TABLE IF NOT EXISTS authorization_audit_events TYPE NORMAL SCHEMAFULL
  PERMISSIONS FOR select WHERE account_id = $auth.account_id AND principal_id = $auth.principal_id FOR create, update, delete NONE;
DEFINE FIELD IF NOT EXISTS decision_id ON authorization_audit_events TYPE string ASSERT string::len($value) > 0;
DEFINE FIELD IF NOT EXISTS account_id ON authorization_audit_events TYPE option<record<local_accounts>>;
DEFINE FIELD IF NOT EXISTS principal_id ON authorization_audit_events TYPE option<record<principals>>;
DEFINE FIELD IF NOT EXISTS session_id ON authorization_audit_events TYPE option<record<authenticated_sessions>>;
DEFINE FIELD IF NOT EXISTS access_space_id ON authorization_audit_events TYPE option<record<access_spaces>>;
DEFINE FIELD IF NOT EXISTS delegation_chain ON authorization_audit_events TYPE array<string>;
DEFINE FIELD IF NOT EXISTS resource_id ON authorization_audit_events TYPE option<record<protected_resources>>;
DEFINE FIELD IF NOT EXISTS requested_resource_hash ON authorization_audit_events TYPE string ASSERT string::len($value) = 64;
DEFINE FIELD IF NOT EXISTS resource_kind ON authorization_audit_events TYPE string;
DEFINE FIELD IF NOT EXISTS action ON authorization_audit_events TYPE string;
DEFINE FIELD IF NOT EXISTS capability_id ON authorization_audit_events TYPE string;
DEFINE FIELD IF NOT EXISTS result ON authorization_audit_events TYPE string ASSERT $value IN ['allow', 'deny', 'error'];
DEFINE FIELD IF NOT EXISTS policy_version ON authorization_audit_events TYPE option<int>;
DEFINE FIELD IF NOT EXISTS occurred_at ON authorization_audit_events TYPE datetime;
DEFINE INDEX IF NOT EXISTS authorization_audit_decision_unique ON authorization_audit_events FIELDS decision_id UNIQUE;

DEFINE ACCESS IF NOT EXISTS authenticated_session ON DATABASE TYPE RECORD
  SIGNIN (
    SELECT * FROM authenticated_sessions
    WHERE token_hash = $token_hash
      AND revoked_at = NONE
      AND expires_at > time::now()
      AND (channel_binding_hash = NONE OR channel_binding_hash = $channel_binding_hash)
      AND account_id.status = 'enabled'
      AND principal_id.status = 'enabled'
      AND access_space_id.status = 'active'
      AND access_space_id.account_id = account_id
      AND account_id.revocation_epoch = account_revocation_epoch
      AND principal_id.revocation_epoch = principal_revocation_epoch
      AND access_space_id.revocation_epoch = space_revocation_epoch
      AND account_id.policy_version <= policy_version
      AND principal_id.policy_version <= policy_version
      AND access_space_id.policy_version <= policy_version
    LIMIT 1
  )
  DURATION FOR TOKEN 5m, FOR SESSION 12h;

-- MT-109 V8: authority and protected data share the primary database. These helpers
-- re-check the live session, account, Principal, AccessSpace, exact ResourceGrant,
-- delegated capability, and revocation/policy epochs at the table boundary.
DEFINE FUNCTION OVERWRITE fn::mt109_live_session() {
    RETURN $auth != NONE
        AND $auth.revoked_at = NONE
        AND $auth.expires_at > time::now()
        AND $auth.account_id.status = 'enabled'
        AND $auth.principal_id.status = 'enabled'
        AND $auth.access_space_id.status = 'active'
        AND $auth.account_id.revocation_epoch = $auth.account_revocation_epoch
        AND $auth.principal_id.revocation_epoch = $auth.principal_revocation_epoch
        AND $auth.access_space_id.revocation_epoch = $auth.space_revocation_epoch
        AND $auth.account_id.policy_version <= $auth.policy_version
        AND $auth.principal_id.policy_version <= $auth.policy_version
        AND $auth.access_space_id.policy_version <= $auth.policy_version;
};

DEFINE FUNCTION OVERWRITE fn::mt109_has_grant($kind: string, $external: string, $action: string, $capability: string) {
    RETURN fn::mt109_live_session()
        AND (array::includes($auth.delegated_capabilities, '*') OR array::includes($auth.delegated_capabilities, $capability))
        AND array::len((SELECT id FROM resource_grants
            WHERE status = 'active'
              AND revoked_at = NONE
              AND (expires_at = NONE OR expires_at > time::now())
              AND account_id = $auth.account_id
              AND principal_id = $auth.principal_id
              AND access_space_id = $auth.access_space_id
              AND resource_id.lifecycle_state = 'active'
              AND resource_id.owner_account_id = $auth.account_id
              AND resource_id.access_space_id = $auth.access_space_id
              AND resource_id.resource_kind = $kind
              AND resource_id.external_resource_id = $external
              AND ($kind != 'reconciliation_queue' OR (principal_id.principal_kind = 'service_identity' AND principal_id.capability_profile_id = 'MT109Reconciler'))
              AND array::includes(actions, $action)
              AND array::includes(capability_ids, $capability)
              AND delegation_chain = $auth.delegation_chain)) > 0;
};

DEFINE FUNCTION OVERWRITE fn::mt109_has_workspace_access($external: string) {
    RETURN fn::mt109_live_session()
        AND array::len((SELECT id FROM resource_grants
            WHERE status = 'active'
              AND revoked_at = NONE
              AND (expires_at = NONE OR expires_at > time::now())
              AND account_id = $auth.account_id
              AND principal_id = $auth.principal_id
              AND access_space_id = $auth.access_space_id
              AND resource_id.lifecycle_state = 'active'
              AND resource_id.owner_account_id = $auth.account_id
              AND resource_id.access_space_id = $auth.access_space_id
              AND resource_id.external_resource_id = $external
              AND delegation_chain = $auth.delegation_chain)) > 0;
};

DEFINE FUNCTION OVERWRITE fn::mt109_ledger_access($resource: option<record<protected_resources>>, $session: option<record<authenticated_sessions>>, $capability: option<string>, $action: option<string>) {
    RETURN fn::mt109_live_session()
        AND $session = $auth.id
        AND $resource != NONE
        AND array::len((SELECT id FROM resource_grants
            WHERE status = 'active'
              AND revoked_at = NONE
              AND (expires_at = NONE OR expires_at > time::now())
              AND account_id = $auth.account_id
              AND principal_id = $auth.principal_id
              AND access_space_id = $auth.access_space_id
              AND resource_id = $resource
              AND resource_id.lifecycle_state = 'active'
              AND resource_id.owner_account_id = $auth.account_id
              AND resource_id.access_space_id = $auth.access_space_id
              AND $action != NONE
              AND array::includes(actions, $action)
              AND $capability != NONE
              AND array::includes(capability_ids, $capability)
              AND (array::includes($auth.delegated_capabilities, '*') OR array::includes($auth.delegated_capabilities, $capability))
              AND delegation_chain = $auth.delegation_chain)) > 0;
};

DEFINE TABLE OVERWRITE workspaces SCHEMAFULL
    PERMISSIONS FOR select WHERE fn::mt109_has_workspace_access(record::id(id)) FOR create, update, delete NONE;

DEFINE TABLE OVERWRITE kernel_event_ledger SCHEMAFULL
    PERMISSIONS FOR select WHERE fn::mt109_ledger_access(authority_resource_id, authority_session_id, authority_capability_id, 'read') OR fn::mt109_has_grant('reconciliation_queue', 'mt109-protected-reconciliation:' + type::string(wsids[0]), 'reconcile', 'fr.ingest.native_editor') OR fn::mt109_has_grant('reconciliation_queue', 'mt109-protected-reconciliation:' + type::string(wsids[0]), 'reconcile', 'memory.commit')
                FOR create WHERE fn::mt109_ledger_access(authority_resource_id, authority_session_id, authority_capability_id, authority_action) AND ((authority_action = 'reconcile' AND (fn::mt109_has_grant('reconciliation_queue', 'mt109-protected-reconciliation:' + type::string(wsids[0]), 'reconcile', 'fr.ingest.native_editor') OR fn::mt109_has_grant('reconciliation_queue', 'mt109-protected-reconciliation:' + type::string(wsids[0]), 'reconcile', 'memory.commit'))) OR (authority_action != 'reconcile' AND array::includes(wsids, authority_resource_id.external_resource_id)))
                FOR update, delete NONE;
DEFINE FIELD OVERWRITE authority_resource_id ON TABLE kernel_event_ledger TYPE option<record<protected_resources>> PERMISSIONS FULL;
DEFINE FIELD OVERWRITE authority_session_id ON TABLE kernel_event_ledger TYPE option<record<authenticated_sessions>> PERMISSIONS FULL;
DEFINE FIELD OVERWRITE authority_capability_id ON TABLE kernel_event_ledger TYPE option<string> PERMISSIONS FULL;
DEFINE FIELD OVERWRITE authority_action ON TABLE kernel_event_ledger TYPE option<string> PERMISSIONS FULL;
DEFINE FIELD OVERWRITE wsids ON TABLE kernel_event_ledger TYPE array<string> PERMISSIONS FULL;

DEFINE TABLE OVERWRITE fems_memory_packs SCHEMAFULL
    PERMISSIONS FOR select WHERE fn::mt109_has_grant('memory_pack', record::id(workspace_id), 'read', 'memory.read')
                FOR create, update WHERE fn::mt109_has_grant('memory_pack', record::id(workspace_id), 'create', 'memory.commit')
                FOR delete NONE;
DEFINE TABLE OVERWRITE fems_memory_proposals SCHEMAFULL
    PERMISSIONS FOR select WHERE fn::mt109_has_grant('memory_proposal', record::id(workspace_id), 'read', 'memory.read') OR fn::mt109_has_grant('reconciliation_queue', 'mt109-protected-reconciliation:' + record::id(workspace_id), 'reconcile', 'memory.commit')
                FOR create WHERE fn::mt109_has_grant('memory_proposal', record::id(workspace_id), 'create', 'memory.propose')
                FOR update WHERE fn::mt109_has_grant('memory_proposal', record::id(workspace_id), 'update', 'memory.review') OR fn::mt109_has_grant('memory_proposal', record::id(workspace_id), 'update', 'memory.commit')
                FOR delete NONE;
DEFINE TABLE OVERWRITE fems_memory_items SCHEMAFULL
    PERMISSIONS FOR select WHERE fn::mt109_has_grant('memory_item', record::id(workspace_id), 'read', 'memory.read') OR fn::mt109_has_grant('memory_item_count', record::id(workspace_id), 'read', 'memory.read')
                FOR create, update WHERE fn::mt109_has_grant('memory_item', record::id(workspace_id), 'create', 'memory.commit')
                FOR delete NONE;
DEFINE TABLE OVERWRITE fems_memory_commit_reports SCHEMAFULL
    PERMISSIONS FOR select WHERE fn::mt109_has_grant('memory_commit_report', record::id(workspace_id), 'read', 'memory.read') OR fn::mt109_has_grant('reconciliation_queue', 'mt109-protected-reconciliation:' + record::id(workspace_id), 'reconcile', 'memory.commit')
                FOR create WHERE fn::mt109_has_grant('memory_commit_report', record::id(workspace_id), 'create', 'memory.commit')
                FOR update, delete NONE;
DEFINE TABLE OVERWRITE fems_memory_commit_fr_outbox SCHEMAFULL
    PERMISSIONS FOR select, create, update WHERE fn::mt109_has_grant('memory_proposal', record::id(workspace_id), 'update', 'memory.commit') OR fn::mt109_has_grant('reconciliation_queue', 'mt109-protected-reconciliation:' + record::id(workspace_id), 'reconcile', 'memory.commit')
                FOR delete NONE;
DEFINE TABLE OVERWRITE fems_memory_lifecycle_fr_outbox SCHEMAFULL
    PERMISSIONS FOR select, create, update WHERE fn::mt109_has_grant('memory_proposal', record::id(workspace_id), 'create', 'memory.propose') OR fn::mt109_has_grant('memory_proposal', record::id(workspace_id), 'update', 'memory.review') OR fn::mt109_has_grant('memory_proposal', record::id(workspace_id), 'update', 'memory.commit') OR fn::mt109_has_grant('reconciliation_queue', 'mt109-protected-reconciliation:' + record::id(workspace_id), 'reconcile', 'memory.commit')
                FOR delete NONE;
DEFINE TABLE OVERWRITE fems_workspace_write_anchors SCHEMAFULL
    PERMISSIONS FOR select, create, update WHERE fn::mt109_has_workspace_access(workspace_key) FOR delete NONE;
