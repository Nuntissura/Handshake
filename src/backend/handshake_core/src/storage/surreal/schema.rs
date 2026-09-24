use std::collections::{BTreeMap, BTreeSet};

use futures::{stream, StreamExt, TryStreamExt};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use sha2::{Digest, Sha256};
use surrealdb::types::{
    Array as SurrealArray, Datetime, Object as SurrealObject, RecordId, RecordIdKey, SurrealValue,
    Value as SurrealValueData,
};
use tokio::sync::Mutex;

use super::{
    SurrealAdminContext, SurrealStorage, SurrealStorageError, DEFAULT_DATABASE, DEFAULT_NAMESPACE,
};

pub const SCHEMA_VERSION: &str = "wp-kernel-012-surreal-v1";
pub const SCHEMA_REVISION: i64 = 160;
/// Exact revision-159 catalog before standalone LoomBlock update authorization.
const PRE_STANDALONE_LOOM_UPDATE_REVISION: i64 = 159;
const PRE_STANDALONE_LOOM_UPDATE_GENERATED_SHA256: &str =
    "d7cf6adbe590a6807a7ee495cd320add2dada1588d02c7bb90d91c6022abe178";
const PRE_STANDALONE_LOOM_UPDATE_INFO_SHA256: &str =
    "ef2a6a96a6a869b6bb39cbc3abc2b30491535c73cd36b5fb8f97d6f0bc8ee610";
/// Exact revision-158 catalog observed before the Canvas receipt visibility correction.
const PRE_CANVAS_RECEIPT_REVISION: i64 = 158;
const PRE_CANVAS_RECEIPT_GENERATED_SHA256: &str =
    "6dc618075e25af5ae13d61b7914ab07799c5142e9db479cc86016c864de59899";
const PRE_CANVAS_RECEIPT_INFO_SHA256: &str =
    "2d2f73504c6dc9efec83497ee7faa4b914fee19398540dbf93bda62fd0ef1b76";
const PRE_ACCOUNT_SETUP_REVISION: i64 = 157;
const PRE_ACCOUNT_SETUP_GENERATED_SHA256: &str =
    "e9a08258c5c0d86bfff6e4dcbdc4b086e0dafd4fa9eaec8735026c37088bcad1";
const PRE_ACCOUNT_SETUP_INFO_SHA256: &str =
    "e2a125fd980b463ac63fb857acd919786031d9dc04ce0d88bf9663ae0824944f";
const AUTHORITY_NONCE_EVENT_STATEMENTS: &str = r#"-- MT109_AUTHORITY_NONCE_EVENT_BEGIN
DEFINE EVENT OVERWRITE mt109_local_account_authorization_touch ON TABLE local_accounts
WHEN $event = 'UPDATE' AND $auth != NONE
THEN {
    IF $after.authorization_touch_nonce != ($before.authorization_touch_nonce ?? 0) + 1
        OR $before.account_key != $after.account_key
        OR $before.account_role != $after.account_role
        OR $before.status != $after.status
        OR $before.revocation_epoch != $after.revocation_epoch
        OR $before.policy_version != $after.policy_version
        OR $before.created_at != $after.created_at
        OR $before.updated_at != $after.updated_at
        OR $before.password_verifier != $after.password_verifier {
        THROW 'HSK-403-PROTECTED-RESOURCE';
    };
};
DEFINE EVENT OVERWRITE mt109_principal_authorization_touch ON TABLE principals
WHEN $event = 'UPDATE' AND $auth != NONE
THEN {
    IF $after.authorization_touch_nonce != ($before.authorization_touch_nonce ?? 0) + 1
        OR $before.principal_key != $after.principal_key
        OR $before.account_id != $after.account_id
        OR $before.principal_kind != $after.principal_kind
        OR $before.actor_kind != $after.actor_kind
        OR $before.actor_id != $after.actor_id
        OR $before.capability_profile_id != $after.capability_profile_id
        OR $before.delegated_capabilities != $after.delegated_capabilities
        OR $before.status != $after.status
        OR $before.revocation_epoch != $after.revocation_epoch
        OR $before.policy_version != $after.policy_version
        OR $before.created_at != $after.created_at
        OR $before.updated_at != $after.updated_at {
        THROW 'HSK-403-PROTECTED-RESOURCE';
    };
};
DEFINE EVENT OVERWRITE mt109_access_space_authorization_touch ON TABLE access_spaces
WHEN $event = 'UPDATE' AND $auth != NONE
THEN {
    IF $after.authorization_touch_nonce != ($before.authorization_touch_nonce ?? 0) + 1
        OR $before.space_key != $after.space_key
        OR $before.account_id != $after.account_id
        OR $before.name != $after.name
        OR $before.status != $after.status
        OR $before.revocation_epoch != $after.revocation_epoch
        OR $before.policy_version != $after.policy_version
        OR $before.created_at != $after.created_at
        OR $before.updated_at != $after.updated_at {
        THROW 'HSK-403-PROTECTED-RESOURCE';
    };
};
DEFINE EVENT OVERWRITE mt109_authenticated_session_authorization_touch ON TABLE authenticated_sessions
WHEN $event = 'UPDATE' AND $auth != NONE
THEN {
    IF $after.authorization_touch_nonce != ($before.authorization_touch_nonce ?? 0) + 1
        OR $before.account_id != $after.account_id
        OR $before.principal_id != $after.principal_id
        OR $before.access_space_id != $after.access_space_id
        OR $before.token_hash != $after.token_hash
        OR $before.channel_binding_hash != $after.channel_binding_hash
        OR $before.authentication_strength != $after.authentication_strength
        OR $before.delegated_capabilities != $after.delegated_capabilities
        OR $before.delegation_chain != $after.delegation_chain
        OR $before.account_revocation_epoch != $after.account_revocation_epoch
        OR $before.principal_revocation_epoch != $after.principal_revocation_epoch
        OR $before.space_revocation_epoch != $after.space_revocation_epoch
        OR $before.policy_version != $after.policy_version
        OR $before.issued_at != $after.issued_at
        OR $before.expires_at != $after.expires_at
        OR $before.revoked_at != $after.revoked_at {
        THROW 'HSK-403-PROTECTED-RESOURCE';
    };
};
DEFINE EVENT OVERWRITE mt109_protected_resource_authorization_touch ON TABLE protected_resources
WHEN $event = 'UPDATE' AND $auth != NONE
THEN {
    IF $after.authorization_touch_nonce != ($before.authorization_touch_nonce ?? 0) + 1
        OR $before.resource_kind != $after.resource_kind
        OR $before.external_resource_id != $after.external_resource_id
        OR $before.owner_account_id != $after.owner_account_id
        OR $before.created_by_principal_id != $after.created_by_principal_id
        OR $before.created_in_session_id != $after.created_in_session_id
        OR $before.access_space_id != $after.access_space_id
        OR $before.parent_resource_id != $after.parent_resource_id
        OR $before.schema_version != $after.schema_version
        OR $before.lifecycle_state != $after.lifecycle_state
        OR $before.policy_version != $after.policy_version
        OR $before.classification != $after.classification
        OR $before.storage_locator_hash != $after.storage_locator_hash
        OR $before.created_at != $after.created_at
        OR $before.updated_at != $after.updated_at
        OR $before.creator_grant_id != $after.creator_grant_id {
        THROW 'HSK-403-PROTECTED-RESOURCE';
    };
};
DEFINE EVENT OVERWRITE mt109_resource_grant_authorization_touch ON TABLE resource_grants
WHEN $event = 'UPDATE' AND $auth != NONE
THEN {
    IF $after.authorization_touch_nonce != ($before.authorization_touch_nonce ?? 0) + 1
        OR $before.account_id != $after.account_id
        OR $before.principal_id != $after.principal_id
        OR $before.access_space_id != $after.access_space_id
        OR $before.resource_id != $after.resource_id
        OR $before.actions != $after.actions
        OR $before.capability_ids != $after.capability_ids
        OR $before.delegation_chain != $after.delegation_chain
        OR $before.status != $after.status
        OR $before.grant_version != $after.grant_version
        OR $before.policy_version != $after.policy_version
        OR $before.expires_at != $after.expires_at
        OR $before.revoked_at != $after.revoked_at
        OR $before.created_at != $after.created_at
        OR $before.updated_at != $after.updated_at {
        THROW 'HSK-403-PROTECTED-RESOURCE';
    };
};
-- MT109_AUTHORITY_NONCE_EVENT_END
"#;
const AUTHORITY_NONCE_IMMUTABLE_FIELDS: &[(&str, &[&str])] = &[
    (
        "local_accounts",
        &[
            "account_key",
            "account_role",
            "status",
            "revocation_epoch",
            "policy_version",
            "created_at",
            "updated_at",
            "password_verifier",
        ],
    ),
    (
        "principals",
        &[
            "principal_key",
            "account_id",
            "principal_kind",
            "actor_kind",
            "actor_id",
            "capability_profile_id",
            "delegated_capabilities",
            "status",
            "revocation_epoch",
            "policy_version",
            "created_at",
            "updated_at",
        ],
    ),
    (
        "access_spaces",
        &[
            "space_key",
            "account_id",
            "name",
            "status",
            "revocation_epoch",
            "policy_version",
            "created_at",
            "updated_at",
        ],
    ),
    (
        "authenticated_sessions",
        &[
            "account_id",
            "principal_id",
            "access_space_id",
            "token_hash",
            "channel_binding_hash",
            "authentication_strength",
            "delegated_capabilities",
            "delegation_chain",
            "account_revocation_epoch",
            "principal_revocation_epoch",
            "space_revocation_epoch",
            "policy_version",
            "issued_at",
            "expires_at",
            "revoked_at",
        ],
    ),
    (
        "protected_resources",
        &[
            "resource_kind",
            "external_resource_id",
            "owner_account_id",
            "created_by_principal_id",
            "created_in_session_id",
            "access_space_id",
            "parent_resource_id",
            "schema_version",
            "lifecycle_state",
            "policy_version",
            "classification",
            "storage_locator_hash",
            "created_at",
            "updated_at",
            "creator_grant_id",
        ],
    ),
    (
        "resource_grants",
        &[
            "account_id",
            "principal_id",
            "access_space_id",
            "resource_id",
            "actions",
            "capability_ids",
            "delegation_chain",
            "status",
            "grant_version",
            "policy_version",
            "expires_at",
            "revoked_at",
            "created_at",
            "updated_at",
        ],
    ),
];
const MT120_RECORD_USER_UPDATE_GUARD_TABLES: &[&str] = &[
    "knowledge_source_roots",
    "knowledge_sources",
    "knowledge_index_runs",
    "knowledge_entities",
    "knowledge_edges",
    "knowledge_ingestion_repair_queue",
    "knowledge_code_files",
    "knowledge_code_repair_queue",
    "knowledge_rich_documents",
    "knowledge_rich_document_title_anchors",
    "knowledge_document_embeds",
    "knowledge_workbench_layout_states",
    "knowledge_workspace_settings_states",
    "knowledge_workspace_search_bookmark_states",
];
const MT120_RECORD_USER_UPDATE_GUARD_RESTORATIONS: &[(&str, &str, &str)] = &[
    ("knowledge_source_roots", "FOR update WHERE created_in_session_id != NONE AND created_in_session_id.account_id = $auth.account_id AND created_in_session_id.access_space_id = $auth.access_space_id AND fn::mt109_has_workspace_access(record::id(workspace_id), 'update', 'memory.propose')", "FOR update WHERE created_in_session_id != NONE AND created_in_session_id.account_id = $auth.account_id AND created_in_session_id.access_space_id = $auth.access_space_id AND fn::mt109_has_workspace_access(record::id(workspace_id), 'update', 'memory.propose') AND $before.workspace_id = $after.workspace_id AND $before.root_id = $after.root_id AND $before.repo_relative_path = $after.repo_relative_path"),
    ("knowledge_sources", "FOR update WHERE fn::mt109_has_grant('knowledge_source', source_id, 'update', 'memory.propose')", "FOR update WHERE fn::mt109_has_grant('knowledge_source', source_id, 'update', 'memory.propose') AND $before.source_id = $after.source_id AND $before.workspace_id = $after.workspace_id AND $before.root_id = $after.root_id AND $before.source_kind = $after.source_kind AND $before.relative_path = $after.relative_path"),
    ("knowledge_index_runs", "FOR update WHERE fn::mt120_index_run_access($this, true)", "FOR update WHERE fn::mt120_index_run_access($before, true) AND fn::mt120_index_run_access($after, true) AND $before.workspace_id = $after.workspace_id AND $before.root_id = $after.root_id AND $before.actor_id = $after.actor_id AND $before.actor_kind = $after.actor_kind AND $before.start_receipt_event_id = $after.start_receipt_event_id"),
    ("knowledge_entities", "FOR update WHERE fn::mt120_entity_write(id, primary_source_id, workspace_id)", "FOR update WHERE fn::mt120_entity_write(id, primary_source_id, workspace_id) AND fn::mt120_entity_write($before.id, $before.primary_source_id, $before.workspace_id) AND $before.workspace_id = $after.workspace_id AND $before.entity_kind = $after.entity_kind AND $before.entity_key = $after.entity_key"),
    ("knowledge_edges", "FOR update WHERE fn::mt120_edge_write(id, source_entity_id, target_entity_id, workspace_id)", "FOR update WHERE fn::mt120_edge_write(id, source_entity_id, target_entity_id, workspace_id) AND $before.workspace_id = $after.workspace_id AND $before.source_entity_id = $after.source_entity_id AND $before.target_entity_id = $after.target_entity_id AND $before.relationship_id = $after.relationship_id"),
    ("knowledge_ingestion_repair_queue", "FOR update WHERE fn::mt120_index_source_write(source_id, workspace_id)", "FOR update WHERE fn::mt120_index_source_write(source_id, workspace_id) AND $before.workspace_id = $after.workspace_id AND $before.source_id = $after.source_id"),
    ("knowledge_code_files", "FOR update WHERE fn::mt109_has_grant('knowledge_code_file', code_file_id, 'update', 'memory.propose')", "FOR update WHERE fn::mt109_has_grant('knowledge_code_file', code_file_id, 'update', 'memory.propose') AND $before.code_file_id = $after.code_file_id AND $before.workspace_id = $after.workspace_id AND $before.source_id = $after.source_id"),
    ("knowledge_code_repair_queue", "FOR update WHERE fn::mt120_index_source_write(source_id, workspace_id)", "FOR update WHERE fn::mt120_index_source_write(source_id, workspace_id) AND $before.workspace_id = $after.workspace_id AND $before.source_id = $after.source_id"),
    ("knowledge_rich_documents", "FOR update WHERE fn::mt120_document_access(rich_document_id, record::id(workspace_id), 'update', 'fs.write') OR fn::mt120_document_delete(rich_document_id, record::id(workspace_id))", "FOR update WHERE (fn::mt120_document_access(rich_document_id, record::id(workspace_id), 'update', 'fs.write') AND $before.deleted_at = NONE AND $after.deleted_at = NONE AND $before.deleted_receipt_event_id = $after.deleted_receipt_event_id) OR (fn::mt120_document_delete(rich_document_id, record::id(workspace_id)) AND $before.deleted_at = NONE AND $after.deleted_at != NONE AND $after.deleted_receipt_event_id != NONE AND $after.deleted_receipt_event_id.event_type = 'KNOWLEDGE_RICH_DOCUMENT_DELETED' AND $after.deleted_receipt_event_id.aggregate_id = rich_document_id AND $after.deleted_receipt_event_id.authority_session_id = $auth.id AND array::len($after.projection_refs) = array::len($before.projection_refs) + 1 AND array::slice($after.projection_refs, 0, array::len($before.projection_refs)) = $before.projection_refs AND $before.rich_document_id = $after.rich_document_id AND $before.workspace_id = $after.workspace_id AND $before.document_id = $after.document_id AND $before.title = $after.title AND $before.schema_version = $after.schema_version AND $before.doc_version = $after.doc_version AND $before.content_json = $after.content_json AND $before.content_sha256 = $after.content_sha256 AND $before.crdt_document_id = $after.crdt_document_id AND $before.crdt_snapshot_id = $after.crdt_snapshot_id AND $before.promotion_receipt_event_id = $after.promotion_receipt_event_id AND $before.project_ref = $after.project_ref AND $before.folder_ref = $after.folder_ref AND $before.authority_label = $after.authority_label AND $before.owner_actor_kind = $after.owner_actor_kind AND $before.owner_actor_id = $after.owner_actor_id AND $before.created_at = $after.created_at AND $before.created_in_session_id = $after.created_in_session_id)"),
    ("knowledge_rich_document_title_anchors", "FOR update WHERE (fn::mt120_document_access(last_rich_document_id, record::id(workspace_id), 'update', 'fs.write') OR fn::mt120_document_access(last_rich_document_id, record::id(workspace_id), 'delete', 'fs.write'))", "FOR update WHERE (fn::mt120_document_access(last_rich_document_id, record::id(workspace_id), 'update', 'fs.write') OR fn::mt120_document_access(last_rich_document_id, record::id(workspace_id), 'delete', 'fs.write')) AND $before.workspace_id = $after.workspace_id AND $before.title_key = $after.title_key AND $before.anchor_key = $after.anchor_key"),
    ("knowledge_document_embeds", "FOR update WHERE rich_document_id.deleted_at = NONE AND fn::mt120_document_access(record::id(rich_document_id), record::id(rich_document_id.workspace_id), 'update', 'fs.write')", "FOR update WHERE rich_document_id.deleted_at = NONE AND fn::mt120_document_access(record::id(rich_document_id), record::id(rich_document_id.workspace_id), 'update', 'fs.write') AND $before.rich_document_id = $after.rich_document_id"),
    ("knowledge_workbench_layout_states", "FOR update WHERE fn::mt109_has_workspace_access(record::id(workspace_id), 'update', 'fs.write')", "FOR update WHERE fn::mt109_has_workspace_access(record::id(workspace_id), 'update', 'fs.write') AND $before.workspace_id = $after.workspace_id"),
    ("knowledge_workspace_settings_states", "FOR update WHERE fn::mt109_has_workspace_access(record::id(workspace_id), 'update', 'fs.write')", "FOR update WHERE fn::mt109_has_workspace_access(record::id(workspace_id), 'update', 'fs.write') AND $before.workspace_id = $after.workspace_id"),
    ("knowledge_workspace_search_bookmark_states", "FOR update WHERE fn::mt109_has_workspace_access(record::id(workspace_id), 'update', 'fs.write')", "FOR update WHERE fn::mt109_has_workspace_access(record::id(workspace_id), 'update', 'fs.write') AND $before.workspace_id = $after.workspace_id"),
];
const LOCAL_ACCOUNT_SETUP_STATEMENTS: &str = r#"-- LOCAL_ACCOUNT_SETUP_BEGIN
DEFINE FIELD OVERWRITE password_verifier ON TABLE local_accounts TYPE option<string> PERMISSIONS NONE;
DEFINE TABLE OVERWRITE local_account_setup TYPE NORMAL SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE account_id ON TABLE local_account_setup TYPE record<local_accounts>;
DEFINE FIELD OVERWRITE principal_id ON TABLE local_account_setup TYPE record<principals>;
DEFINE FIELD OVERWRITE access_space_id ON TABLE local_account_setup TYPE record<access_spaces>;
DEFINE FIELD OVERWRITE created_at ON TABLE local_account_setup TYPE datetime;
-- LOCAL_ACCOUNT_SETUP_END
"#;

const MT120_DOCUMENT_ACCESS_BLOCK: &str = r#"-- MT120_DOCUMENT_ACCESS_BEGIN
DEFINE FUNCTION OVERWRITE fn::mt120_document_access($document: string, $workspace: string, $action: string, $capability: string) {
    RETURN fn::mt109_has_grant('rich_document', $document, $action, $capability)
        AND fn::mt109_has_workspace_access($workspace, 'read', 'fs.read')
        AND array::len((SELECT id FROM resource_grants
            WHERE status = 'active' AND revoked_at = NONE
              AND (expires_at = NONE OR expires_at > time::now())
              AND account_id = $auth.account_id AND principal_id = $auth.principal_id
              AND access_space_id = $auth.access_space_id
              AND resource_id.resource_kind = 'rich_document' AND resource_id.external_resource_id = $document
              AND resource_id.lifecycle_state = 'active' AND resource_id.owner_account_id = $auth.account_id
              AND resource_id.access_space_id = $auth.access_space_id AND resource_id.created_by_principal_id.status = 'enabled'
              AND resource_id.parent_resource_id.resource_kind = 'workspace'
              AND resource_id.parent_resource_id.external_resource_id = $workspace
              AND resource_id.parent_resource_id.lifecycle_state = 'active'
              AND resource_id.parent_resource_id.owner_account_id = $auth.account_id
              AND resource_id.parent_resource_id.access_space_id = $auth.access_space_id
              AND resource_id.parent_resource_id.created_by_principal_id.status = 'enabled'
              AND actions CONTAINS $action AND capability_ids CONTAINS $capability
              AND delegation_chain = $auth.delegation_chain)) > 0;
};
DEFINE FUNCTION OVERWRITE fn::mt120_loom_block_access($block: string, $workspace: string, $action: string, $capability: string) {
    RETURN fn::mt109_has_grant('loom_block', $block, $action, $capability)
        AND fn::mt109_has_workspace_access($workspace, 'read', 'fs.read')
        AND array::len((SELECT id FROM resource_grants
            WHERE status = 'active' AND revoked_at = NONE
              AND (expires_at = NONE OR expires_at > time::now())
              AND account_id = $auth.account_id AND principal_id = $auth.principal_id
              AND access_space_id = $auth.access_space_id
              AND resource_id.resource_kind = 'loom_block' AND resource_id.external_resource_id = $block
              AND resource_id.lifecycle_state = 'active' AND resource_id.owner_account_id = $auth.account_id
              AND resource_id.access_space_id = $auth.access_space_id AND resource_id.created_by_principal_id.status = 'enabled'
              AND resource_id.parent_resource_id.resource_kind = 'workspace'
              AND resource_id.parent_resource_id.external_resource_id = $workspace
              AND resource_id.parent_resource_id.lifecycle_state = 'active'
              AND resource_id.parent_resource_id.owner_account_id = $auth.account_id
              AND resource_id.parent_resource_id.access_space_id = $auth.access_space_id
              AND resource_id.parent_resource_id.created_by_principal_id.status = 'enabled'
              AND actions CONTAINS $action AND capability_ids CONTAINS $capability
              AND delegation_chain = $auth.delegation_chain)) > 0;
};
DEFINE FUNCTION OVERWRITE fn::mt120_loom_endpoint_access($block: option<record<loom_blocks>>, $workspace: string, $action: string, $capability: string) {
    IF $block = NONE OR $block.workspace_id != type::record('workspaces', $workspace) { RETURN false; };
    IF $block.source_rich_document_id = NONE {
        RETURN fn::mt120_loom_block_access(record::id($block), $workspace, $action, $capability);
    };
    RETURN record::id($block.source_rich_document_id) = record::id($block)
        AND fn::mt120_document_access(record::id($block), $workspace, $action, $capability);
};
DEFINE FUNCTION OVERWRITE fn::mt120_loom_receipt($resource: option<record<protected_resources>>, $session: option<record<authenticated_sessions>>, $capability: option<string>, $action: option<string>, $wsids: array<string>, $event: string, $source: string, $aggregate: string, $aggregate_id: string, $payload: object, $actor_kind: string, $actor_id: string, $write: bool) {
    IF !fn::mt109_live_session() OR $resource = NONE OR $session = NONE
        OR array::len($wsids) != 1 OR $payload.workspace_id != $wsids[0]
        OR $resource.owner_account_id != $auth.account_id OR $resource.access_space_id != $auth.access_space_id
        OR $actor_kind != $session.principal_id.actor_kind OR $actor_id != $session.principal_id.actor_id { RETURN false; };
    IF $resource.resource_kind = 'workspace' AND $resource.external_resource_id = $wsids[0]
        AND $capability = 'fs.write' AND $action = 'create'
        AND (($event = 'KNOWLEDGE_LOOM_BLOCK_INDEXED'
              AND $source = 'loom_block_knowledge_bridge' AND $aggregate = 'knowledge_loom_block'
              AND $aggregate_id = $payload.entity_id AND $payload.type = 'knowledge_loom_block_indexed'
              AND $payload.block_id != NONE AND $payload.content_type IN ['note', 'file', 'annotated_file', 'tag_hub', 'journal', 'canvas']
              AND type::record('loom_blocks', $payload.block_id).workspace_id = type::record('workspaces', $wsids[0])
              AND type::record('loom_blocks', $payload.block_id).created_in_session_id = $session)
             OR ($event = 'KNOWLEDGE_LOOM_CANVAS_BOARD_RECORDED'
                 AND $source = 'loom_canvas_board' AND $aggregate = 'loom_canvas_board'
                 AND $aggregate_id = $payload.block_id AND $payload.type = 'knowledge_loom_canvas_board_recorded'
                 AND $payload.op = 'create' AND type::record('loom_blocks', $payload.block_id).workspace_id = type::record('workspaces', $wsids[0])
                 AND type::record('loom_blocks', $payload.block_id).content_type = 'canvas'
                 AND type::record('loom_blocks', $payload.block_id).created_in_session_id = $session)) {
        RETURN ($write AND $session = $auth.id AND fn::mt109_ledger_access($resource, $session, $capability, $action)
                AND fn::mt109_has_workspace_access($wsids[0], 'create', 'fs.write'))
            OR (!$write AND fn::mt120_loom_block_access($payload.block_id, $wsids[0], 'read', 'fs.read'));
    };
    IF $resource.resource_kind = 'loom_block' AND $resource.external_resource_id = $payload.block_id
        AND $capability = 'fs.write' AND $action = 'update'
        AND $event = 'KNOWLEDGE_LOOM_BLOCK_MUTATED' AND $source = 'loom_block' AND $aggregate = 'loom_block'
        AND $aggregate_id = $payload.block_id AND $payload.type = 'knowledge_loom_block_mutated' AND $payload.operation = 'update'
        AND type::record('loom_blocks', $payload.block_id).workspace_id = type::record('workspaces', $wsids[0]) {
        RETURN ($write AND $session = $auth.id AND fn::mt109_ledger_access($resource, $session, $capability, $action)
                AND fn::mt120_loom_block_access($payload.block_id, $wsids[0], 'update', 'fs.write'))
            OR (!$write AND fn::mt120_loom_block_access($payload.block_id, $wsids[0], 'read', 'fs.read'));
    };
    IF (($event = 'KNOWLEDGE_LOOM_FOLDER_MUTATED' AND $source = 'loom_folder' AND $aggregate = 'loom_folder')
        OR ($event = 'KNOWLEDGE_LOOM_TAG_MUTATED' AND $source = 'loom_edge' AND $aggregate = 'loom_edge')
        OR ($event = 'KNOWLEDGE_LOOM_BLOCK_MUTATED' AND $source = 'loom_block' AND $aggregate = 'loom_block')
        OR ($event = 'KNOWLEDGE_LOOM_BLOCK_INDEXED' AND (($source = 'loom_block_knowledge_bridge' AND $aggregate = 'knowledge_loom_block')
            OR ($source = 'loom_search_v2' AND $aggregate = 'loom_block_search_index')))
        OR ($event = 'KNOWLEDGE_LOOM_WIKI_MUTATED' AND $source = 'loom_wiki' AND $aggregate = 'loom_wiki_overlay')
        OR ($event = 'KNOWLEDGE_PROJECTION_REBUILT' AND $source IN ['project_wiki_compiler', 'project_wiki_drift_checker']
            AND $aggregate = 'knowledge_wiki' AND $aggregate_id = $wsids[0])
        OR ($event = 'KNOWLEDGE_QUICK_SWITCHER_RECENT_RECORDED' AND $source = 'quick_switcher_recents' AND $aggregate = 'quick_switcher_recent')
        OR ($event IN ['AI_EDIT_PROPOSAL_RECORDED', 'AI_EDIT_PROPOSAL_DECIDED'] AND $source IN ['loom_ai_job', 'loom_ai_promotion'] AND $aggregate = 'loom_ai_suggestion')
        OR ($event IN ['PROMOTION_REQUESTED', 'PROMOTION_ACCEPTED', 'PROMOTION_REJECTED'] AND $source = 'loom_ai_promotion' AND $aggregate = 'loom_ai_promotion')
        OR ($event = 'KNOWLEDGE_LOOM_CANVAS_BOARD_RECORDED' AND $source = 'loom_canvas_board' AND $aggregate = 'loom_canvas_board'
            AND $payload.op = 'viewport' AND $resource.resource_kind = 'loom_block'))
        AND $capability = 'fs.write' AND $action IN ['create', 'update', 'delete'] {
        IF $resource.resource_kind = 'workspace' AND $resource.external_resource_id = $wsids[0] {
            RETURN ($write AND $session = $auth.id AND fn::mt109_ledger_access($resource, $session, $capability, $action)
                    AND fn::mt109_has_workspace_access($wsids[0], $action, 'fs.write'))
                OR (!$write AND fn::mt109_has_workspace_access($wsids[0], 'read', 'fs.read'));
        };
        IF $resource.resource_kind IN ['loom_block', 'rich_document'] AND $action = 'update'
            AND ($resource.external_resource_id = $payload.block_id OR $resource.external_resource_id = $payload.source_block_id) {
            LET $block = type::record('loom_blocks', $resource.external_resource_id);
            RETURN ($write AND $session = $auth.id AND fn::mt109_ledger_access($resource, $session, $capability, $action)
                    AND fn::mt120_loom_endpoint_access($block, $wsids[0], 'update', 'fs.write'))
                OR (!$write AND fn::mt120_loom_endpoint_access($block, $wsids[0], 'read', 'fs.read'));
        };
    };
    IF $resource.resource_kind != 'loom_block' OR $resource.external_resource_id != $payload.canvas_block_id
        OR $capability != 'fs.write' OR $action != 'update'
        OR $event != 'KNOWLEDGE_LOOM_CANVAS_BOARD_RECORDED' OR $source != 'loom_canvas_board'
        OR $aggregate != 'loom_canvas_placement' OR $aggregate_id != $payload.placement_id
        OR $payload.type NOT IN ['knowledge_loom_canvas_placement_recorded', 'knowledge_loom_canvas_placement_removed']
        OR (($payload.type = 'knowledge_loom_canvas_placement_recorded' AND $payload.op != 'create')
            OR ($payload.type = 'knowledge_loom_canvas_placement_removed' AND $payload.op != 'remove_placement'))
        OR $payload.canvas_block_id = NONE OR $payload.placed_block_id = NONE
        OR type::record('loom_blocks', $payload.canvas_block_id).workspace_id != type::record('workspaces', $wsids[0])
        OR type::record('loom_blocks', $payload.canvas_block_id).content_type != 'canvas'
        OR type::record('loom_blocks', $payload.placed_block_id).workspace_id != type::record('workspaces', $wsids[0]) { RETURN false; };
    IF $write AND array::len(SELECT id FROM loom_canvas_placements WHERE placement_id = $payload.placement_id
        AND workspace_id = type::record('workspaces', $wsids[0])
        AND canvas_block_id = type::record('loom_canvas_boards', $payload.canvas_block_id)
        AND placed_block_id = type::record('loom_blocks', $payload.placed_block_id)) != 1 { RETURN false; };
    RETURN ($write AND $session = $auth.id AND fn::mt109_ledger_access($resource, $session, $capability, $action)
            AND fn::mt120_loom_block_access($payload.canvas_block_id, $wsids[0], 'update', 'fs.write')
            AND (fn::mt120_loom_block_access($payload.placed_block_id, $wsids[0], 'read', 'fs.read') OR fn::mt120_document_access($payload.placed_block_id, $wsids[0], 'read', 'fs.read')))
        OR (!$write AND $session = $auth.id AND fn::mt109_ledger_access($resource, $session, $capability, $action)
            AND fn::mt120_loom_block_access($payload.canvas_block_id, $wsids[0], 'update', 'fs.write')
            AND (fn::mt120_loom_block_access($payload.placed_block_id, $wsids[0], 'read', 'fs.read') OR fn::mt120_document_access($payload.placed_block_id, $wsids[0], 'read', 'fs.read')))
        OR (!$write AND fn::mt120_loom_block_access($payload.canvas_block_id, $wsids[0], 'read', 'fs.read')
            AND (fn::mt120_loom_block_access($payload.placed_block_id, $wsids[0], 'read', 'fs.read') OR fn::mt120_document_access($payload.placed_block_id, $wsids[0], 'read', 'fs.read')));
};
DEFINE FUNCTION OVERWRITE fn::mt120_document_receipt($resource: option<record<protected_resources>>, $session: option<record<authenticated_sessions>>, $capability: option<string>, $action: option<string>, $wsids: array<string>, $event: string, $source: string, $aggregate: string, $document: string, $payload: object, $actor_kind: string, $actor_id: string, $write: bool) {
    RETURN fn::mt109_live_session() AND $resource != NONE AND $session != NONE
        AND $resource.resource_kind = 'rich_document' AND $resource.external_resource_id = $document
        AND $resource.owner_account_id = $auth.account_id AND $resource.access_space_id = $auth.access_space_id
        AND array::len($wsids) = 1 AND $resource.parent_resource_id.external_resource_id = $wsids[0]
        AND $capability = 'fs.write' AND $action IN ['create', 'update', 'delete']
        AND $aggregate = 'knowledge_rich_document' AND $source = 'knowledge_documents_api'
        AND $payload.workspace_id = $wsids[0]
        AND $payload.minted_by_principal = record::id($session.principal_id)
        AND $actor_kind = $session.principal_id.actor_kind AND $actor_id = $session.principal_id.actor_id
        AND (($action IN ['create', 'update'] AND $event = 'KNOWLEDGE_RICH_DOCUMENT_SAVED' AND $payload.event IN ['created', 'saved', 'imported', 'embed_repair', 'renamed', 'moved', 'batch'])
             OR ($action = 'update' AND $event = 'KNOWLEDGE_CRDT_RECOVERY_RECEIPT_RECORDED' AND $payload.event IN ['draft_saved', 'draft_cleared', 'draft_noop_cleared'])
             OR ($action = 'delete' AND $event = 'KNOWLEDGE_RICH_DOCUMENT_DELETED' AND $payload.event = 'deleted'))
        AND (($write AND $session = $auth.id
                  AND fn::mt109_ledger_access($resource, $session, $capability, $action)
                  AND fn::mt120_document_access($document, $wsids[0], $action, 'fs.write'))
             OR (!$write AND fn::mt120_document_access($document, $wsids[0], 'read', 'fs.read')));
};
DEFINE FUNCTION OVERWRITE fn::mt120_workspace_state_receipt($resource: option<record<protected_resources>>, $session: option<record<authenticated_sessions>>, $capability: option<string>, $action: option<string>, $wsids: array<string>, $event: string, $source: string, $aggregate: string, $workspace: string, $payload: object, $actor_kind: string, $actor_id: string, $write: bool) {
    RETURN fn::mt109_live_session() AND $resource != NONE AND $session != NONE
        AND $resource.resource_kind = 'workspace' AND $resource.external_resource_id = $workspace
        AND $resource.owner_account_id = $auth.account_id AND $resource.access_space_id = $auth.access_space_id
        AND array::len($wsids) = 1 AND $wsids[0] = $workspace AND $payload.workspace_id = $workspace
        AND $capability = 'fs.write' AND $action = 'update'
        AND $actor_kind = $session.principal_id.actor_kind AND $actor_id = $session.principal_id.actor_id
        AND (($event = 'KNOWLEDGE_WORKBENCH_LAYOUT_STATE_RECORDED' AND $source = 'workbench_layout_state' AND $aggregate = 'workbench_layout_state')
          OR ($event = 'KNOWLEDGE_WORKSPACE_SETTINGS_STATE_RECORDED' AND $source = 'workspace_settings_state' AND $aggregate = 'workspace_settings_state')
          OR ($event = 'KNOWLEDGE_WORKSPACE_SEARCH_BOOKMARK_STATE_RECORDED' AND $source = 'workspace_search_bookmark_state' AND $aggregate = 'workspace_search_bookmark_state'))
        AND (($write AND $session = $auth.id AND fn::mt109_ledger_access($resource, $session, $capability, $action)
              AND fn::mt109_has_workspace_access($workspace, 'update', 'fs.write'))
          OR (!$write AND fn::mt109_has_workspace_access($workspace, 'read', 'fs.read')));
};
-- MT120_DOCUMENT_ACCESS_END
"#;
const MT120_IDEMPOTENCY_BLOCK: &str = r#"-- MT120_IDEMPOTENCY_BEGIN
DEFINE FIELD OVERWRITE rich_document_id ON TABLE knowledge_idempotency_keys TYPE option<record<knowledge_rich_documents>>;
-- MT120_IDEMPOTENCY_END
"#;
const MT120_DOCUMENT_TABLE_UPGRADES: &[(&str, &str)] = &[
    (
        r#"DEFINE TABLE OVERWRITE knowledge_workspace_search_bookmark_states SCHEMAFULL PERMISSIONS NONE;"#,
        r#"DEFINE TABLE OVERWRITE knowledge_workspace_search_bookmark_states SCHEMAFULL
    PERMISSIONS FOR select WHERE fn::mt109_has_workspace_access(record::id(workspace_id), 'read', 'fs.read')
                FOR create WHERE fn::mt109_has_workspace_access(record::id(workspace_id), 'update', 'fs.write')
                FOR update WHERE fn::mt109_has_workspace_access(record::id(workspace_id), 'update', 'fs.write') AND $before.workspace_id = $after.workspace_id
                FOR delete NONE;"#,
    ),
    (
        r#"DEFINE TABLE OVERWRITE knowledge_workspace_settings_states SCHEMAFULL PERMISSIONS NONE;"#,
        r#"DEFINE TABLE OVERWRITE knowledge_workspace_settings_states SCHEMAFULL
    PERMISSIONS FOR select WHERE fn::mt109_has_workspace_access(record::id(workspace_id), 'read', 'fs.read')
                FOR create WHERE fn::mt109_has_workspace_access(record::id(workspace_id), 'update', 'fs.write')
                FOR update WHERE fn::mt109_has_workspace_access(record::id(workspace_id), 'update', 'fs.write') AND $before.workspace_id = $after.workspace_id
                FOR delete NONE;"#,
    ),
    (
        r#"DEFINE TABLE OVERWRITE knowledge_workbench_layout_states SCHEMAFULL PERMISSIONS NONE;"#,
        r#"DEFINE TABLE OVERWRITE knowledge_workbench_layout_states SCHEMAFULL
    PERMISSIONS FOR select WHERE fn::mt109_has_workspace_access(record::id(workspace_id), 'read', 'fs.read')
                FOR create WHERE fn::mt109_has_workspace_access(record::id(workspace_id), 'update', 'fs.write')
                FOR update WHERE fn::mt109_has_workspace_access(record::id(workspace_id), 'update', 'fs.write') AND $before.workspace_id = $after.workspace_id
                FOR delete NONE;"#,
    ),
    (
        r#"DEFINE TABLE OVERWRITE knowledge_rich_documents SCHEMAFULL
    PERMISSIONS FOR select WHERE deleted_at = NONE AND fn::mt109_source_read('rich_document', rich_document_id, record::id(workspace_id), 'workspace', record::id(workspace_id))
                FOR create, update, delete NONE;"#,
        r#"DEFINE TABLE OVERWRITE knowledge_rich_documents SCHEMAFULL
    PERMISSIONS FOR select WHERE (deleted_at = NONE AND (fn::mt109_source_read('rich_document', rich_document_id, record::id(workspace_id), 'workspace', record::id(workspace_id)) OR fn::mt120_document_access(rich_document_id, record::id(workspace_id), 'read', 'fs.read'))) OR (deleted_at != NONE AND fn::mt120_document_access(rich_document_id, record::id(workspace_id), 'delete', 'fs.write'))
                FOR create WHERE fn::mt120_document_access(rich_document_id, record::id(workspace_id), 'create', 'fs.write')
                FOR update WHERE fn::mt120_document_access(rich_document_id, record::id(workspace_id), 'update', 'fs.write')
                FOR delete NONE;"#,
    ),
    (
        r#"DEFINE TABLE OVERWRITE knowledge_rich_document_versions SCHEMAFULL PERMISSIONS NONE;"#,
        r#"DEFINE TABLE OVERWRITE knowledge_rich_document_versions SCHEMAFULL
    PERMISSIONS FOR select WHERE rich_document_id != NONE AND rich_document_id.deleted_at = NONE AND fn::mt120_document_access(record::id(rich_document_id), record::id(rich_document_id.workspace_id), 'read', 'fs.read')
                FOR create WHERE rich_document_id != NONE AND rich_document_id.deleted_at = NONE AND fn::mt120_document_access(record::id(rich_document_id), record::id(rich_document_id.workspace_id), 'update', 'fs.write')
                FOR update NONE
                FOR delete NONE;"#,
    ),
    (
        r#"DEFINE TABLE OVERWRITE knowledge_rich_document_drafts SCHEMAFULL PERMISSIONS NONE;"#,
        r#"DEFINE TABLE OVERWRITE knowledge_rich_document_drafts SCHEMAFULL
    PERMISSIONS FOR select WHERE rich_document_id != NONE AND rich_document_id.deleted_at = NONE AND workspace_id = rich_document_id.workspace_id AND fn::mt120_document_access(record::id(rich_document_id), record::id(rich_document_id.workspace_id), 'read', 'fs.read')
                FOR create WHERE rich_document_id != NONE AND rich_document_id.deleted_at = NONE AND workspace_id = rich_document_id.workspace_id AND fn::mt120_document_access(record::id(rich_document_id), record::id(rich_document_id.workspace_id), 'update', 'fs.write')
                FOR update WHERE rich_document_id != NONE AND rich_document_id.deleted_at = NONE AND workspace_id = rich_document_id.workspace_id AND fn::mt120_document_access(record::id(rich_document_id), record::id(rich_document_id.workspace_id), 'update', 'fs.write')
                FOR delete WHERE rich_document_id != NONE AND rich_document_id.deleted_at = NONE AND workspace_id = rich_document_id.workspace_id AND fn::mt120_document_access(record::id(rich_document_id), record::id(rich_document_id.workspace_id), 'update', 'fs.write');"#,
    ),
    (
        r#"DEFINE TABLE OVERWRITE knowledge_idempotency_keys SCHEMAFULL PERMISSIONS NONE;"#,
        r#"DEFINE TABLE OVERWRITE knowledge_idempotency_keys SCHEMAFULL
    PERMISSIONS FOR select WHERE operation_kind = 'rich_document_save' AND rich_document_id != NONE AND rich_document_id.workspace_id = workspace_id AND fn::mt120_document_access(record::id(rich_document_id), record::id(workspace_id), 'read', 'fs.read')
                FOR create WHERE operation_kind = 'rich_document_save' AND rich_document_id != NONE AND rich_document_id.workspace_id = workspace_id AND fn::mt120_document_access(record::id(rich_document_id), record::id(workspace_id), 'update', 'fs.write')
                FOR update NONE
                FOR delete NONE;"#,
    ),
    (
        r#"DEFINE TABLE OVERWRITE loom_blocks SCHEMAFULL
    PERMISSIONS FOR select WHERE (source_rich_document_id = NONE AND fn::mt109_source_read('loom_block', block_id, record::id(workspace_id), 'workspace', record::id(workspace_id))) OR (source_rich_document_id != NONE AND record::id(source_rich_document_id) = block_id AND content_type = 'note' AND fn::mt109_source_read('loom_block', block_id, record::id(workspace_id), 'rich_document', record::id(source_rich_document_id)) AND fn::mt109_source_read('rich_document', record::id(source_rich_document_id), record::id(workspace_id), 'workspace', record::id(workspace_id)) AND source_rich_document_id.workspace_id = workspace_id AND source_rich_document_id.deleted_at = NONE AND content_hash = source_rich_document_id.content_sha256)
                FOR create, update, delete NONE;"#,
        r#"DEFINE TABLE OVERWRITE loom_blocks SCHEMAFULL
    PERMISSIONS FOR select WHERE ((source_rich_document_id = NONE AND fn::mt109_source_read('loom_block', block_id, record::id(workspace_id), 'workspace', record::id(workspace_id))) OR (source_rich_document_id != NONE AND record::id(source_rich_document_id) = block_id AND content_type = 'note' AND fn::mt109_source_read('loom_block', block_id, record::id(workspace_id), 'rich_document', record::id(source_rich_document_id)) AND fn::mt109_source_read('rich_document', record::id(source_rich_document_id), record::id(workspace_id), 'workspace', record::id(workspace_id)) AND source_rich_document_id.workspace_id = workspace_id AND source_rich_document_id.deleted_at = NONE AND content_hash = source_rich_document_id.content_sha256)) OR (source_rich_document_id != NONE AND record::id(source_rich_document_id) = block_id AND content_type = 'note' AND source_rich_document_id.workspace_id = workspace_id AND source_rich_document_id.deleted_at = NONE AND content_hash = source_rich_document_id.content_sha256 AND fn::mt120_document_access(record::id(source_rich_document_id), record::id(workspace_id), 'read', 'fs.read'))
                FOR create WHERE source_rich_document_id != NONE AND record::id(source_rich_document_id) = block_id AND content_type = 'note' AND source_rich_document_id.workspace_id = workspace_id AND source_rich_document_id.deleted_at = NONE AND content_hash = source_rich_document_id.content_sha256 AND (fn::mt120_document_access(record::id(source_rich_document_id), record::id(workspace_id), 'create', 'fs.write') OR fn::mt120_document_access(record::id(source_rich_document_id), record::id(workspace_id), 'update', 'fs.write'))
                FOR update WHERE source_rich_document_id != NONE AND record::id(source_rich_document_id) = block_id AND content_type = 'note' AND source_rich_document_id.workspace_id = workspace_id AND source_rich_document_id.deleted_at = NONE AND content_hash = source_rich_document_id.content_sha256 AND fn::mt120_document_access(record::id(source_rich_document_id), record::id(workspace_id), 'update', 'fs.write')
                FOR delete NONE;"#,
    ),
    (
        r#"DEFINE TABLE OVERWRITE loom_block_search_index SCHEMAFULL PERMISSIONS NONE;"#,
        r#"DEFINE TABLE OVERWRITE loom_block_search_index SCHEMAFULL
    PERMISSIONS FOR select WHERE block_id.source_rich_document_id != NONE AND block_id.workspace_id = workspace_id AND block_id.source_rich_document_id.workspace_id = workspace_id AND block_id.source_rich_document_id.deleted_at = NONE AND fn::mt120_document_access(record::id(block_id.source_rich_document_id), record::id(workspace_id), 'read', 'fs.read')
                FOR create WHERE block_id.source_rich_document_id != NONE AND block_id.workspace_id = workspace_id AND block_id.source_rich_document_id.workspace_id = workspace_id AND block_id.source_rich_document_id.deleted_at = NONE AND (fn::mt120_document_access(record::id(block_id.source_rich_document_id), record::id(workspace_id), 'create', 'fs.write') OR fn::mt120_document_access(record::id(block_id.source_rich_document_id), record::id(workspace_id), 'update', 'fs.write'))
                FOR update WHERE block_id.source_rich_document_id != NONE AND block_id.workspace_id = workspace_id AND block_id.source_rich_document_id.workspace_id = workspace_id AND block_id.source_rich_document_id.deleted_at = NONE AND fn::mt120_document_access(record::id(block_id.source_rich_document_id), record::id(workspace_id), 'update', 'fs.write')
                FOR delete WHERE block_id.source_rich_document_id != NONE AND block_id.workspace_id = workspace_id AND block_id.source_rich_document_id.workspace_id = workspace_id AND block_id.source_rich_document_id.deleted_at = NONE AND fn::mt120_document_access(record::id(block_id.source_rich_document_id), record::id(workspace_id), 'update', 'fs.write');"#,
    ),
    (
        r#"DEFINE TABLE OVERWRITE knowledge_document_backlinks SCHEMAFULL PERMISSIONS NONE;"#,
        r#"DEFINE TABLE OVERWRITE knowledge_document_backlinks SCHEMAFULL
    PERMISSIONS FOR select WHERE source_document_id.workspace_id = workspace_id AND link_kind = 'wikilink' AND fn::mt120_document_access(record::id(source_document_id), record::id(workspace_id), 'read', 'fs.read') AND fn::mt120_document_access(target, record::id(workspace_id), 'read', 'fs.read')
                FOR create WHERE source_document_id.workspace_id = workspace_id AND link_kind = 'wikilink' AND fn::mt120_document_access(record::id(source_document_id), record::id(workspace_id), 'update', 'fs.write') AND fn::mt120_document_access(target, record::id(workspace_id), 'read', 'fs.read')
                FOR update WHERE source_document_id.workspace_id = workspace_id AND link_kind = 'wikilink' AND fn::mt120_document_access(record::id(source_document_id), record::id(workspace_id), 'update', 'fs.write') AND fn::mt120_document_access(target, record::id(workspace_id), 'read', 'fs.read')
                FOR delete WHERE source_document_id.workspace_id = workspace_id AND link_kind = 'wikilink' AND fn::mt120_document_access(record::id(source_document_id), record::id(workspace_id), 'update', 'fs.write') AND fn::mt120_document_access(target, record::id(workspace_id), 'read', 'fs.read');"#,
    ),
    (
        r#"DEFINE TABLE OVERWRITE loom_edges SCHEMAFULL PERMISSIONS NONE;"#,
        r#"DEFINE TABLE OVERWRITE loom_edges SCHEMAFULL
    PERMISSIONS FOR select WHERE source_document_id != NONE AND source_block_id.source_rich_document_id != NONE AND record::id(source_block_id.source_rich_document_id) = source_document_id AND source_block_id.workspace_id = workspace_id AND target_block_id.workspace_id = workspace_id AND target_block_id.source_rich_document_id != NONE AND edge_type = 'mention' AND last_actor_kind = 'SYSTEM' AND last_actor_id = 'knowledge_rich_document_backlink_projection' AND fn::mt120_document_access(source_document_id, record::id(workspace_id), 'read', 'fs.read') AND fn::mt120_document_access(record::id(target_block_id.source_rich_document_id), record::id(workspace_id), 'read', 'fs.read')
                FOR create WHERE source_document_id != NONE AND source_block_id.source_rich_document_id != NONE AND record::id(source_block_id.source_rich_document_id) = source_document_id AND source_block_id.workspace_id = workspace_id AND target_block_id.workspace_id = workspace_id AND target_block_id.source_rich_document_id != NONE AND edge_type = 'mention' AND last_actor_kind = 'SYSTEM' AND last_actor_id = 'knowledge_rich_document_backlink_projection' AND fn::mt120_document_access(source_document_id, record::id(workspace_id), 'update', 'fs.write') AND fn::mt120_document_access(record::id(target_block_id.source_rich_document_id), record::id(workspace_id), 'read', 'fs.read')
                FOR update WHERE source_document_id != NONE AND source_block_id.source_rich_document_id != NONE AND record::id(source_block_id.source_rich_document_id) = source_document_id AND source_block_id.workspace_id = workspace_id AND target_block_id.workspace_id = workspace_id AND target_block_id.source_rich_document_id != NONE AND edge_type = 'mention' AND last_actor_kind = 'SYSTEM' AND last_actor_id = 'knowledge_rich_document_backlink_projection' AND fn::mt120_document_access(source_document_id, record::id(workspace_id), 'update', 'fs.write') AND fn::mt120_document_access(record::id(target_block_id.source_rich_document_id), record::id(workspace_id), 'read', 'fs.read')
                FOR delete WHERE source_document_id != NONE AND source_block_id.source_rich_document_id != NONE AND record::id(source_block_id.source_rich_document_id) = source_document_id AND source_block_id.workspace_id = workspace_id AND target_block_id.workspace_id = workspace_id AND target_block_id.source_rich_document_id != NONE AND edge_type = 'mention' AND last_actor_kind = 'SYSTEM' AND last_actor_id = 'knowledge_rich_document_backlink_projection' AND fn::mt120_document_access(source_document_id, record::id(workspace_id), 'update', 'fs.write') AND fn::mt120_document_access(record::id(target_block_id.source_rich_document_id), record::id(workspace_id), 'read', 'fs.read');"#,
    ),
    (
        r#"DEFINE TABLE OVERWRITE kernel_event_ledger SCHEMAFULL
    PERMISSIONS FOR select WHERE fn::mt109_ledger_receipt(authority_capability_id, event_type, source_component, aggregate_type, payload, wsids) AND fn::mt109_ledger_reader(authority_resource_id, authority_session_id, authority_capability_id, authority_action, wsids)
                FOR create WHERE fn::mt109_ledger_receipt(authority_capability_id, event_type, source_component, aggregate_type, payload, wsids) AND fn::mt109_ledger_producer(authority_resource_id, authority_session_id, authority_capability_id, authority_action, wsids)
                FOR update, delete NONE;"#,
        r#"DEFINE TABLE OVERWRITE kernel_event_ledger SCHEMAFULL
    PERMISSIONS FOR select WHERE (fn::mt109_ledger_receipt(authority_capability_id, event_type, source_component, aggregate_type, payload, wsids) AND fn::mt109_ledger_reader(authority_resource_id, authority_session_id, authority_capability_id, authority_action, wsids)) OR fn::mt120_document_receipt(authority_resource_id, authority_session_id, authority_capability_id, authority_action, wsids, event_type, source_component, aggregate_type, aggregate_id, payload, actor_kind, actor_id, false) OR fn::mt120_workspace_state_receipt(authority_resource_id, authority_session_id, authority_capability_id, authority_action, wsids, event_type, source_component, aggregate_type, aggregate_id, payload, actor_kind, actor_id, false)
                FOR create WHERE (fn::mt109_ledger_receipt(authority_capability_id, event_type, source_component, aggregate_type, payload, wsids) AND fn::mt109_ledger_producer(authority_resource_id, authority_session_id, authority_capability_id, authority_action, wsids)) OR fn::mt120_document_receipt(authority_resource_id, authority_session_id, authority_capability_id, authority_action, wsids, event_type, source_component, aggregate_type, aggregate_id, payload, actor_kind, actor_id, true) OR fn::mt120_workspace_state_receipt(authority_resource_id, authority_session_id, authority_capability_id, authority_action, wsids, event_type, source_component, aggregate_type, aggregate_id, payload, actor_kind, actor_id, true)
                FOR update, delete NONE;"#,
    ),
    (
        r#"DEFINE TABLE OVERWRITE workspaces SCHEMAFULL
    PERMISSIONS FOR select WHERE fn::mt109_has_workspace_access(record::id(id), 'read', 'fr.read') OR fn::mt109_has_workspace_access(record::id(id), 'read', 'memory.read') OR fn::mt109_has_grant('reconciliation_queue', 'mt109-protected-reconciliation:' + record::id(id), 'reconcile', 'memory.commit') FOR create, update, delete NONE;"#,
        r#"DEFINE TABLE OVERWRITE workspaces SCHEMAFULL
    PERMISSIONS FOR select WHERE fn::mt109_has_workspace_access(record::id(id), 'read', 'fs.read') OR fn::mt109_has_workspace_access(record::id(id), 'read', 'fr.read') OR fn::mt109_has_workspace_access(record::id(id), 'read', 'memory.read') OR fn::mt109_has_grant('reconciliation_queue', 'mt109-protected-reconciliation:' + record::id(id), 'reconcile', 'memory.commit') FOR create, update, delete NONE;"#,
    ),
    (
        r#"DEFINE TABLE OVERWRITE knowledge_rich_document_title_anchors SCHEMAFULL PERMISSIONS NONE;"#,
        r#"DEFINE TABLE OVERWRITE knowledge_rich_document_title_anchors SCHEMAFULL
    PERMISSIONS FOR select WHERE fn::mt120_document_access(last_rich_document_id, record::id(workspace_id), 'read', 'fs.read')
                FOR create, update, delete NONE;"#,
    ),
    (
        r#"DEFINE TABLE OVERWRITE knowledge_document_embeds SCHEMAFULL PERMISSIONS NONE;"#,
        r#"DEFINE TABLE OVERWRITE knowledge_document_embeds SCHEMAFULL
    PERMISSIONS FOR select WHERE rich_document_id.deleted_at = NONE AND fn::mt120_document_access(record::id(rich_document_id), record::id(rich_document_id.workspace_id), 'read', 'fs.read')
                FOR create WHERE rich_document_id.deleted_at = NONE AND fn::mt120_document_access(record::id(rich_document_id), record::id(rich_document_id.workspace_id), 'update', 'fs.write')
                FOR update WHERE rich_document_id.deleted_at = NONE AND fn::mt120_document_access(record::id(rich_document_id), record::id(rich_document_id.workspace_id), 'update', 'fs.write') AND $before.rich_document_id = $after.rich_document_id
                FOR delete WHERE rich_document_id.deleted_at = NONE AND fn::mt120_document_access(record::id(rich_document_id), record::id(rich_document_id.workspace_id), 'update', 'fs.write');"#,
    ),
    (
        r#"DEFINE FUNCTION OVERWRITE fn::mt109_live_session() {
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
};"#,
        r#"DEFINE FUNCTION OVERWRITE fn::mt109_live_session() {
    RETURN $auth != NONE
        AND $auth.revoked_at = NONE
        AND $auth.expires_at > time::now()
        AND $auth.account_id.status = 'enabled'
        AND $auth.principal_id.status = 'enabled'
        AND $auth.access_space_id.status = 'active'
        AND $auth.principal_id.account_id = $auth.account_id
        AND $auth.access_space_id.account_id = $auth.account_id
        AND $auth.account_id.revocation_epoch = $auth.account_revocation_epoch
        AND $auth.principal_id.revocation_epoch = $auth.principal_revocation_epoch
        AND $auth.access_space_id.revocation_epoch = $auth.space_revocation_epoch
        AND $auth.account_id.policy_version <= $auth.policy_version
        AND $auth.principal_id.policy_version <= $auth.policy_version
        AND $auth.access_space_id.policy_version <= $auth.policy_version;
};"#,
    ),
    (
        r#"DEFINE FUNCTION OVERWRITE fn::mt109_has_grant($kind: string, $external: string, $action: string, $capability: string) {
    RETURN fn::mt109_live_session()
        AND (($auth.delegated_capabilities CONTAINS '*') OR ($auth.delegated_capabilities CONTAINS $capability))
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
              AND (actions CONTAINS $action)
              AND (capability_ids CONTAINS $capability)
              AND delegation_chain = $auth.delegation_chain)) > 0;
};"#,
        r#"DEFINE FUNCTION OVERWRITE fn::mt109_has_grant($kind: string, $external: string, $action: string, $capability: string) {
    RETURN fn::mt109_live_session()
        AND (($auth.delegated_capabilities CONTAINS '*') OR ($auth.delegated_capabilities CONTAINS $capability))
        AND (($auth.principal_id.delegated_capabilities CONTAINS '*') OR ($auth.principal_id.delegated_capabilities CONTAINS $capability))
        AND array::len((SELECT id FROM resource_grants
            WHERE status = 'active'
              AND revoked_at = NONE
              AND (expires_at = NONE OR expires_at > time::now())
              AND account_id = $auth.account_id
              AND principal_id = $auth.principal_id
              AND access_space_id = $auth.access_space_id
              AND resource_id.lifecycle_state = 'active'
              AND resource_id.created_by_principal_id.status = 'enabled'
              AND resource_id.policy_version <= policy_version
              AND policy_version <= $auth.policy_version
              AND resource_id.owner_account_id = $auth.account_id
              AND resource_id.access_space_id = $auth.access_space_id
              AND resource_id.resource_kind = $kind
              AND resource_id.external_resource_id = $external
              AND ($kind != 'reconciliation_queue' OR (principal_id.principal_kind = 'service_identity' AND principal_id.capability_profile_id = 'MT109Reconciler'))
              AND (actions CONTAINS $action)
              AND (capability_ids CONTAINS $capability)
              AND delegation_chain = $auth.delegation_chain)) > 0;
};"#,
    ),
    (
        r#"DEFINE FUNCTION OVERWRITE fn::mt109_has_workspace_access($external: string, $action: string, $capability: string) {
    RETURN fn::mt109_live_session()
        AND (($auth.delegated_capabilities CONTAINS '*') OR ($auth.delegated_capabilities CONTAINS $capability))
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
              AND resource_id.resource_kind = 'workspace'
              AND resource_id.external_resource_id = $external
              AND (actions CONTAINS $action)
              AND (capability_ids CONTAINS $capability)
              AND delegation_chain = $auth.delegation_chain)) > 0;
};"#,
        r#"DEFINE FUNCTION OVERWRITE fn::mt109_has_workspace_access($external: string, $action: string, $capability: string) {
    RETURN fn::mt109_live_session()
        AND (($auth.delegated_capabilities CONTAINS '*') OR ($auth.delegated_capabilities CONTAINS $capability))
        AND (($auth.principal_id.delegated_capabilities CONTAINS '*') OR ($auth.principal_id.delegated_capabilities CONTAINS $capability))
        AND array::len((SELECT id FROM resource_grants
            WHERE status = 'active'
              AND revoked_at = NONE
              AND (expires_at = NONE OR expires_at > time::now())
              AND account_id = $auth.account_id
              AND principal_id = $auth.principal_id
              AND access_space_id = $auth.access_space_id
              AND resource_id.lifecycle_state = 'active'
              AND resource_id.created_by_principal_id.status = 'enabled'
              AND resource_id.policy_version <= policy_version
              AND policy_version <= $auth.policy_version
              AND resource_id.owner_account_id = $auth.account_id
              AND resource_id.access_space_id = $auth.access_space_id
              AND resource_id.resource_kind = 'workspace'
              AND resource_id.external_resource_id = $external
              AND (actions CONTAINS $action)
              AND (capability_ids CONTAINS $capability)
              AND delegation_chain = $auth.delegation_chain)) > 0;
};"#,
    ),
    (
        r#"DEFINE TABLE OVERWRITE local_accounts TYPE NORMAL SCHEMAFULL
  PERMISSIONS FOR select WHERE id = $auth.account_id FOR create, update, delete NONE;"#,
        r#"DEFINE TABLE OVERWRITE local_accounts TYPE NORMAL SCHEMAFULL
  PERMISSIONS FOR select WHERE id = $auth.account_id FOR create NONE FOR update WHERE fn::mt109_live_session() AND id = $auth.account_id AND $after.authorization_touch_nonce = ($before.authorization_touch_nonce ?? 0) + 1 AND $before.account_key = $after.account_key AND $before.account_role = $after.account_role AND $before.status = $after.status AND $before.revocation_epoch = $after.revocation_epoch AND $before.policy_version = $after.policy_version AND $before.created_at = $after.created_at AND $before.updated_at = $after.updated_at AND $before.password_verifier = $after.password_verifier FOR delete NONE;"#,
    ),
    (
        r#"DEFINE TABLE OVERWRITE principals TYPE NORMAL SCHEMAFULL
  PERMISSIONS FOR select WHERE id = $auth.principal_id FOR create, update, delete NONE;"#,
        r#"DEFINE TABLE OVERWRITE principals TYPE NORMAL SCHEMAFULL
  PERMISSIONS FOR select WHERE id = $auth.principal_id FOR create NONE FOR update WHERE fn::mt109_live_session() AND id = $auth.principal_id AND $after.authorization_touch_nonce = ($before.authorization_touch_nonce ?? 0) + 1 AND $before.principal_key = $after.principal_key AND $before.account_id = $after.account_id AND $before.principal_kind = $after.principal_kind AND $before.actor_kind = $after.actor_kind AND $before.actor_id = $after.actor_id AND $before.capability_profile_id = $after.capability_profile_id AND $before.delegated_capabilities = $after.delegated_capabilities AND $before.status = $after.status AND $before.revocation_epoch = $after.revocation_epoch AND $before.policy_version = $after.policy_version AND $before.created_at = $after.created_at AND $before.updated_at = $after.updated_at FOR delete NONE;"#,
    ),
    (
        r#"DEFINE TABLE OVERWRITE access_spaces TYPE NORMAL SCHEMAFULL
  PERMISSIONS FOR select WHERE account_id = $auth.account_id AND id = $auth.access_space_id FOR create, update, delete NONE;"#,
        r#"DEFINE TABLE OVERWRITE access_spaces TYPE NORMAL SCHEMAFULL
  PERMISSIONS FOR select WHERE account_id = $auth.account_id AND id = $auth.access_space_id FOR create NONE FOR update WHERE fn::mt109_live_session() AND id = $auth.access_space_id AND $after.authorization_touch_nonce = ($before.authorization_touch_nonce ?? 0) + 1 AND $before.space_key = $after.space_key AND $before.account_id = $after.account_id AND $before.name = $after.name AND $before.status = $after.status AND $before.revocation_epoch = $after.revocation_epoch AND $before.policy_version = $after.policy_version AND $before.created_at = $after.created_at AND $before.updated_at = $after.updated_at FOR delete NONE;"#,
    ),
    (
        r#"DEFINE TABLE OVERWRITE authenticated_sessions TYPE NORMAL SCHEMAFULL
  PERMISSIONS FOR select WHERE id = $auth.id FOR create, update, delete NONE;"#,
        r#"DEFINE TABLE OVERWRITE authenticated_sessions TYPE NORMAL SCHEMAFULL
  PERMISSIONS FOR select WHERE id = $auth.id FOR create NONE FOR update WHERE fn::mt109_live_session() AND id = $auth.id AND $after.authorization_touch_nonce = ($before.authorization_touch_nonce ?? 0) + 1 AND $before.account_id = $after.account_id AND $before.principal_id = $after.principal_id AND $before.access_space_id = $after.access_space_id AND $before.token_hash = $after.token_hash AND $before.channel_binding_hash = $after.channel_binding_hash AND $before.authentication_strength = $after.authentication_strength AND $before.delegated_capabilities = $after.delegated_capabilities AND $before.delegation_chain = $after.delegation_chain AND $before.account_revocation_epoch = $after.account_revocation_epoch AND $before.principal_revocation_epoch = $after.principal_revocation_epoch AND $before.space_revocation_epoch = $after.space_revocation_epoch AND $before.policy_version = $after.policy_version AND $before.issued_at = $after.issued_at AND $before.expires_at = $after.expires_at AND $before.revoked_at = $after.revoked_at FOR delete NONE;"#,
    ),
    (
        r#"DEFINE TABLE OVERWRITE protected_resources TYPE NORMAL SCHEMAFULL
  PERMISSIONS FOR select WHERE
    lifecycle_state = 'active'
    AND owner_account_id = $auth.account_id
    AND access_space_id = $auth.access_space_id
    AND owner_account_id.status = 'enabled'
    AND created_by_principal_id.status = 'enabled'
    AND access_space_id.status = 'active'
    AND array::len((SELECT id FROM resource_grants WHERE resource_id = $parent.id AND status = 'active' AND revoked_at = NONE AND (expires_at = NONE OR expires_at > time::now()) AND account_id = $auth.account_id AND principal_id = $auth.principal_id AND access_space_id = $auth.access_space_id)) > 0,
  FOR create, update, delete NONE;"#,
        r#"DEFINE TABLE OVERWRITE protected_resources TYPE NORMAL SCHEMAFULL
  PERMISSIONS FOR select WHERE
    lifecycle_state = 'active'
    AND owner_account_id = $auth.account_id
    AND access_space_id = $auth.access_space_id
    AND owner_account_id.status = 'enabled'
    AND created_by_principal_id.status = 'enabled'
    AND access_space_id.status = 'active'
    AND array::len((SELECT id FROM resource_grants WHERE resource_id = $parent.id AND status = 'active' AND revoked_at = NONE AND (expires_at = NONE OR expires_at > time::now()) AND account_id = $auth.account_id AND principal_id = $auth.principal_id AND access_space_id = $auth.access_space_id)) > 0,
  FOR create WHERE fn::mt120_resource_create($this) FOR update WHERE fn::mt109_live_session() AND owner_account_id = $auth.account_id AND access_space_id = $auth.access_space_id AND fn::mt109_has_grant(resource_kind, external_resource_id, 'read', 'fs.read') AND $after.authorization_touch_nonce = ($before.authorization_touch_nonce ?? 0) + 1 AND $before.resource_kind = $after.resource_kind AND $before.external_resource_id = $after.external_resource_id AND $before.owner_account_id = $after.owner_account_id AND $before.created_by_principal_id = $after.created_by_principal_id AND $before.created_in_session_id = $after.created_in_session_id AND $before.access_space_id = $after.access_space_id AND $before.parent_resource_id = $after.parent_resource_id AND $before.schema_version = $after.schema_version AND $before.lifecycle_state = $after.lifecycle_state AND $before.policy_version = $after.policy_version AND $before.classification = $after.classification AND $before.storage_locator_hash = $after.storage_locator_hash AND $before.created_at = $after.created_at AND $before.updated_at = $after.updated_at AND $before.creator_grant_id = $after.creator_grant_id FOR delete NONE;"#,
    ),
    (
        r#"DEFINE TABLE OVERWRITE resource_grants TYPE NORMAL SCHEMAFULL
  PERMISSIONS FOR select WHERE
    status = 'active'
    AND account_id = $auth.account_id
    AND principal_id = $auth.principal_id
    AND access_space_id = $auth.access_space_id,
  FOR create, update, delete NONE;"#,
        r#"DEFINE TABLE OVERWRITE resource_grants TYPE NORMAL SCHEMAFULL
  PERMISSIONS FOR select WHERE
    status = 'active'
    AND account_id = $auth.account_id
    AND principal_id = $auth.principal_id
    AND access_space_id = $auth.access_space_id,
  FOR create WHERE fn::mt120_creator_grant($this) FOR update WHERE fn::mt109_live_session() AND account_id = $auth.account_id AND principal_id = $auth.principal_id AND access_space_id = $auth.access_space_id AND status = 'active' AND revoked_at = NONE AND (expires_at = NONE OR expires_at > time::now()) AND policy_version <= $auth.policy_version AND $after.authorization_touch_nonce = ($before.authorization_touch_nonce ?? 0) + 1 AND $before.account_id = $after.account_id AND $before.principal_id = $after.principal_id AND $before.access_space_id = $after.access_space_id AND $before.resource_id = $after.resource_id AND $before.actions = $after.actions AND $before.capability_ids = $after.capability_ids AND $before.delegation_chain = $after.delegation_chain AND $before.status = $after.status AND $before.grant_version = $after.grant_version AND $before.policy_version = $after.policy_version AND $before.expires_at = $after.expires_at AND $before.revoked_at = $after.revoked_at AND $before.created_at = $after.created_at AND $before.updated_at = $after.updated_at FOR delete NONE;"#,
    ),
    (
        r#"DEFINE FIELD OVERWRITE external_resource_id ON TABLE protected_resources TYPE string ASSERT string::len($value) > 0 PERMISSIONS NONE;"#,
        r#"DEFINE FIELD OVERWRITE external_resource_id ON TABLE protected_resources TYPE string ASSERT string::len($value) > 0 PERMISSIONS FOR select NONE FOR create WHERE fn::mt120_resource_create($this) FOR update NONE;"#,
    ),
    (
        r#"DEFINE FIELD OVERWRITE storage_locator_hash ON TABLE protected_resources TYPE string ASSERT string::len($value) = 64 PERMISSIONS NONE;"#,
        r#"DEFINE FIELD OVERWRITE storage_locator_hash ON TABLE protected_resources TYPE string ASSERT string::len($value) = 64 PERMISSIONS FOR select NONE FOR create WHERE fn::mt120_resource_create($this) FOR update NONE;"#,
    ),
    (
        r#"DEFINE TABLE OVERWRITE workspaces SCHEMAFULL
    PERMISSIONS FOR select WHERE fn::mt109_has_workspace_access(record::id(id), 'read', 'fs.read') OR fn::mt109_has_workspace_access(record::id(id), 'read', 'fr.read') OR fn::mt109_has_workspace_access(record::id(id), 'read', 'memory.read') OR fn::mt109_has_grant('reconciliation_queue', 'mt109-protected-reconciliation:' + record::id(id), 'reconcile', 'memory.commit') FOR create, update, delete NONE;"#,
        r#"DEFINE TABLE OVERWRITE workspaces SCHEMAFULL
    PERMISSIONS FOR select WHERE fn::mt109_has_workspace_access(record::id(id), 'read', 'fs.read') OR fn::mt109_has_workspace_access(record::id(id), 'read', 'fr.read') OR fn::mt109_has_workspace_access(record::id(id), 'read', 'memory.read') OR fn::mt109_has_grant('reconciliation_queue', 'mt109-protected-reconciliation:' + record::id(id), 'reconcile', 'memory.commit') FOR create WHERE fn::mt120_workspace_create(created_in_session_id) FOR update NONE FOR delete WHERE fn::mt120_workspace_delete(record::id(id));"#,
    ),
    (
        r#"DEFINE TABLE OVERWRITE knowledge_rich_documents SCHEMAFULL
    PERMISSIONS FOR select WHERE (deleted_at = NONE AND (fn::mt109_source_read('rich_document', rich_document_id, record::id(workspace_id), 'workspace', record::id(workspace_id)) OR fn::mt120_document_access(rich_document_id, record::id(workspace_id), 'read', 'fs.read'))) OR (deleted_at != NONE AND fn::mt120_document_access(rich_document_id, record::id(workspace_id), 'delete', 'fs.write'))
                FOR create WHERE fn::mt120_document_access(rich_document_id, record::id(workspace_id), 'create', 'fs.write')
                FOR update WHERE fn::mt120_document_access(rich_document_id, record::id(workspace_id), 'update', 'fs.write')
                FOR delete NONE;"#,
        r#"DEFINE TABLE OVERWRITE knowledge_rich_documents SCHEMAFULL
    PERMISSIONS FOR select WHERE (deleted_at = NONE AND (fn::mt109_source_read('rich_document', rich_document_id, record::id(workspace_id), 'workspace', record::id(workspace_id)) OR fn::mt120_document_access(rich_document_id, record::id(workspace_id), 'read', 'fs.read'))) OR (deleted_at != NONE AND fn::mt120_document_access(rich_document_id, record::id(workspace_id), 'delete', 'fs.write'))
                FOR create WHERE fn::mt109_live_session() AND created_in_session_id = $auth.id AND deleted_at = NONE AND fn::mt109_has_workspace_access(record::id(workspace_id), 'create', 'fs.write')
                FOR update WHERE fn::mt120_document_access(rich_document_id, record::id(workspace_id), 'update', 'fs.write')
                FOR delete NONE;"#,
    ),
    (
        r#"DEFINE TABLE OVERWRITE protected_resources TYPE NORMAL SCHEMAFULL
  PERMISSIONS FOR select WHERE
    lifecycle_state = 'active'
    AND owner_account_id = $auth.account_id
    AND access_space_id = $auth.access_space_id
    AND owner_account_id.status = 'enabled'
    AND created_by_principal_id.status = 'enabled'
    AND access_space_id.status = 'active'
    AND array::len((SELECT id FROM resource_grants WHERE resource_id = $parent.id AND status = 'active' AND revoked_at = NONE AND (expires_at = NONE OR expires_at > time::now()) AND account_id = $auth.account_id AND principal_id = $auth.principal_id AND access_space_id = $auth.access_space_id)) > 0,
  FOR create WHERE fn::mt120_resource_create($this) FOR update WHERE fn::mt109_live_session() AND owner_account_id = $auth.account_id AND access_space_id = $auth.access_space_id AND fn::mt109_has_grant(resource_kind, external_resource_id, 'read', 'fs.read') AND $after.authorization_touch_nonce = ($before.authorization_touch_nonce ?? 0) + 1 AND $before.resource_kind = $after.resource_kind AND $before.external_resource_id = $after.external_resource_id AND $before.owner_account_id = $after.owner_account_id AND $before.created_by_principal_id = $after.created_by_principal_id AND $before.created_in_session_id = $after.created_in_session_id AND $before.access_space_id = $after.access_space_id AND $before.parent_resource_id = $after.parent_resource_id AND $before.schema_version = $after.schema_version AND $before.lifecycle_state = $after.lifecycle_state AND $before.policy_version = $after.policy_version AND $before.classification = $after.classification AND $before.storage_locator_hash = $after.storage_locator_hash AND $before.created_at = $after.created_at AND $before.updated_at = $after.updated_at AND $before.creator_grant_id = $after.creator_grant_id FOR delete NONE;"#,
        r#"DEFINE TABLE OVERWRITE protected_resources TYPE NORMAL SCHEMAFULL
  PERMISSIONS FOR select WHERE
    lifecycle_state = 'active'
    AND owner_account_id = $auth.account_id
    AND access_space_id = $auth.access_space_id
    AND owner_account_id.status = 'enabled'
    AND created_by_principal_id.status = 'enabled'
    AND access_space_id.status = 'active'
    AND array::len((SELECT id FROM resource_grants WHERE resource_id = $parent.id AND status = 'active' AND revoked_at = NONE AND (expires_at = NONE OR expires_at > time::now()) AND account_id = $auth.account_id AND principal_id = $auth.principal_id AND access_space_id = $auth.access_space_id)) > 0,
  FOR create WHERE fn::mt120_resource_create($this) FOR update WHERE fn::mt109_live_session() AND owner_account_id = $auth.account_id AND access_space_id = $auth.access_space_id AND (fn::mt109_has_grant(resource_kind, external_resource_id, 'create', 'fs.write') OR fn::mt109_has_grant(resource_kind, external_resource_id, 'update', 'fs.write') OR fn::mt109_has_grant(resource_kind, external_resource_id, 'delete', 'fs.write') OR fn::mt109_has_grant(resource_kind, external_resource_id, 'read', 'fr.read')) AND $after.authorization_touch_nonce = ($before.authorization_touch_nonce ?? 0) + 1 AND $before.resource_kind = $after.resource_kind AND $before.external_resource_id = $after.external_resource_id AND $before.owner_account_id = $after.owner_account_id AND $before.created_by_principal_id = $after.created_by_principal_id AND $before.created_in_session_id = $after.created_in_session_id AND $before.access_space_id = $after.access_space_id AND $before.parent_resource_id = $after.parent_resource_id AND $before.schema_version = $after.schema_version AND $before.lifecycle_state = $after.lifecycle_state AND $before.policy_version = $after.policy_version AND $before.classification = $after.classification AND $before.storage_locator_hash = $after.storage_locator_hash AND $before.created_at = $after.created_at AND $before.updated_at = $after.updated_at AND $before.creator_grant_id = $after.creator_grant_id FOR delete NONE;"#,
    ),
    (
        r#"DEFINE TABLE OVERWRITE loom_blocks SCHEMAFULL
    PERMISSIONS FOR select WHERE ((source_rich_document_id = NONE AND fn::mt109_source_read('loom_block', block_id, record::id(workspace_id), 'workspace', record::id(workspace_id))) OR (source_rich_document_id != NONE AND record::id(source_rich_document_id) = block_id AND content_type = 'note' AND fn::mt109_source_read('loom_block', block_id, record::id(workspace_id), 'rich_document', record::id(source_rich_document_id)) AND fn::mt109_source_read('rich_document', record::id(source_rich_document_id), record::id(workspace_id), 'workspace', record::id(workspace_id)) AND source_rich_document_id.workspace_id = workspace_id AND source_rich_document_id.deleted_at = NONE AND content_hash = source_rich_document_id.content_sha256)) OR (source_rich_document_id != NONE AND record::id(source_rich_document_id) = block_id AND content_type = 'note' AND source_rich_document_id.workspace_id = workspace_id AND source_rich_document_id.deleted_at = NONE AND content_hash = source_rich_document_id.content_sha256 AND fn::mt120_document_access(record::id(source_rich_document_id), record::id(workspace_id), 'read', 'fs.read'))
                FOR create WHERE source_rich_document_id != NONE AND record::id(source_rich_document_id) = block_id AND content_type = 'note' AND source_rich_document_id.workspace_id = workspace_id AND source_rich_document_id.deleted_at = NONE AND content_hash = source_rich_document_id.content_sha256 AND (fn::mt120_document_access(record::id(source_rich_document_id), record::id(workspace_id), 'create', 'fs.write') OR fn::mt120_document_access(record::id(source_rich_document_id), record::id(workspace_id), 'update', 'fs.write'))
                FOR update WHERE source_rich_document_id != NONE AND record::id(source_rich_document_id) = block_id AND content_type = 'note' AND source_rich_document_id.workspace_id = workspace_id AND source_rich_document_id.deleted_at = NONE AND content_hash = source_rich_document_id.content_sha256 AND fn::mt120_document_access(record::id(source_rich_document_id), record::id(workspace_id), 'update', 'fs.write')
                FOR delete NONE;"#,
        r#"DEFINE TABLE OVERWRITE loom_blocks SCHEMAFULL
    PERMISSIONS FOR select WHERE ((source_rich_document_id = NONE AND fn::mt109_source_read('loom_block', block_id, record::id(workspace_id), 'workspace', record::id(workspace_id))) OR (source_rich_document_id != NONE AND record::id(source_rich_document_id) = block_id AND content_type = 'note' AND fn::mt109_source_read('loom_block', block_id, record::id(workspace_id), 'rich_document', record::id(source_rich_document_id)) AND fn::mt109_source_read('rich_document', record::id(source_rich_document_id), record::id(workspace_id), 'workspace', record::id(workspace_id)) AND source_rich_document_id.workspace_id = workspace_id AND source_rich_document_id.deleted_at = NONE AND content_hash = source_rich_document_id.content_sha256)) OR (source_rich_document_id != NONE AND record::id(source_rich_document_id) = block_id AND content_type = 'note' AND source_rich_document_id.workspace_id = workspace_id AND source_rich_document_id.deleted_at = NONE AND content_hash = source_rich_document_id.content_sha256 AND fn::mt120_document_access(record::id(source_rich_document_id), record::id(workspace_id), 'read', 'fs.read'))
                FOR create WHERE source_rich_document_id != NONE AND record::id(source_rich_document_id) = block_id AND content_type = 'note' AND source_rich_document_id.workspace_id = workspace_id AND source_rich_document_id.deleted_at = NONE AND content_hash = source_rich_document_id.content_sha256 AND (fn::mt120_document_access(record::id(source_rich_document_id), record::id(workspace_id), 'create', 'fs.write') OR fn::mt120_document_access(record::id(source_rich_document_id), record::id(workspace_id), 'update', 'fs.write'))
                FOR update WHERE source_rich_document_id != NONE AND record::id(source_rich_document_id) = block_id AND content_type = 'note' AND source_rich_document_id.workspace_id = workspace_id AND source_rich_document_id.deleted_at = NONE AND content_hash = source_rich_document_id.content_sha256 AND fn::mt120_document_access(record::id(source_rich_document_id), record::id(workspace_id), 'update', 'fs.write')
                FOR delete WHERE fn::mt120_workspace_delete(record::id(workspace_id));"#,
    ),
    (
        r#"DEFINE TABLE OVERWRITE loom_edges SCHEMAFULL
    PERMISSIONS FOR select WHERE source_document_id != NONE AND source_block_id.source_rich_document_id != NONE AND record::id(source_block_id.source_rich_document_id) = source_document_id AND source_block_id.workspace_id = workspace_id AND target_block_id.workspace_id = workspace_id AND target_block_id.source_rich_document_id != NONE AND edge_type = 'mention' AND last_actor_kind = 'SYSTEM' AND last_actor_id = 'knowledge_rich_document_backlink_projection' AND fn::mt120_document_access(source_document_id, record::id(workspace_id), 'read', 'fs.read') AND fn::mt120_document_access(record::id(target_block_id.source_rich_document_id), record::id(workspace_id), 'read', 'fs.read')
                FOR create WHERE source_document_id != NONE AND source_block_id.source_rich_document_id != NONE AND record::id(source_block_id.source_rich_document_id) = source_document_id AND source_block_id.workspace_id = workspace_id AND target_block_id.workspace_id = workspace_id AND target_block_id.source_rich_document_id != NONE AND edge_type = 'mention' AND last_actor_kind = 'SYSTEM' AND last_actor_id = 'knowledge_rich_document_backlink_projection' AND fn::mt120_document_access(source_document_id, record::id(workspace_id), 'update', 'fs.write') AND fn::mt120_document_access(record::id(target_block_id.source_rich_document_id), record::id(workspace_id), 'read', 'fs.read')
                FOR update WHERE source_document_id != NONE AND source_block_id.source_rich_document_id != NONE AND record::id(source_block_id.source_rich_document_id) = source_document_id AND source_block_id.workspace_id = workspace_id AND target_block_id.workspace_id = workspace_id AND target_block_id.source_rich_document_id != NONE AND edge_type = 'mention' AND last_actor_kind = 'SYSTEM' AND last_actor_id = 'knowledge_rich_document_backlink_projection' AND fn::mt120_document_access(source_document_id, record::id(workspace_id), 'update', 'fs.write') AND fn::mt120_document_access(record::id(target_block_id.source_rich_document_id), record::id(workspace_id), 'read', 'fs.read')
                FOR delete WHERE source_document_id != NONE AND source_block_id.source_rich_document_id != NONE AND record::id(source_block_id.source_rich_document_id) = source_document_id AND source_block_id.workspace_id = workspace_id AND target_block_id.workspace_id = workspace_id AND target_block_id.source_rich_document_id != NONE AND edge_type = 'mention' AND last_actor_kind = 'SYSTEM' AND last_actor_id = 'knowledge_rich_document_backlink_projection' AND fn::mt120_document_access(source_document_id, record::id(workspace_id), 'update', 'fs.write') AND fn::mt120_document_access(record::id(target_block_id.source_rich_document_id), record::id(workspace_id), 'read', 'fs.read');"#,
        r#"DEFINE TABLE OVERWRITE loom_edges SCHEMAFULL
    PERMISSIONS FOR select WHERE source_document_id != NONE AND source_block_id.source_rich_document_id != NONE AND record::id(source_block_id.source_rich_document_id) = source_document_id AND source_block_id.workspace_id = workspace_id AND target_block_id.workspace_id = workspace_id AND target_block_id.source_rich_document_id != NONE AND edge_type = 'mention' AND last_actor_kind = 'SYSTEM' AND last_actor_id = 'knowledge_rich_document_backlink_projection' AND fn::mt120_document_access(source_document_id, record::id(workspace_id), 'read', 'fs.read') AND fn::mt120_document_access(record::id(target_block_id.source_rich_document_id), record::id(workspace_id), 'read', 'fs.read')
                FOR create WHERE source_document_id != NONE AND source_block_id.source_rich_document_id != NONE AND record::id(source_block_id.source_rich_document_id) = source_document_id AND source_block_id.workspace_id = workspace_id AND target_block_id.workspace_id = workspace_id AND target_block_id.source_rich_document_id != NONE AND edge_type = 'mention' AND last_actor_kind = 'SYSTEM' AND last_actor_id = 'knowledge_rich_document_backlink_projection' AND fn::mt120_document_access(source_document_id, record::id(workspace_id), 'update', 'fs.write') AND fn::mt120_document_access(record::id(target_block_id.source_rich_document_id), record::id(workspace_id), 'read', 'fs.read')
                FOR update WHERE source_document_id != NONE AND source_block_id.source_rich_document_id != NONE AND record::id(source_block_id.source_rich_document_id) = source_document_id AND source_block_id.workspace_id = workspace_id AND target_block_id.workspace_id = workspace_id AND target_block_id.source_rich_document_id != NONE AND edge_type = 'mention' AND last_actor_kind = 'SYSTEM' AND last_actor_id = 'knowledge_rich_document_backlink_projection' AND fn::mt120_document_access(source_document_id, record::id(workspace_id), 'update', 'fs.write') AND fn::mt120_document_access(record::id(target_block_id.source_rich_document_id), record::id(workspace_id), 'read', 'fs.read')
                FOR delete WHERE source_document_id != NONE AND source_block_id.source_rich_document_id != NONE AND record::id(source_block_id.source_rich_document_id) = source_document_id AND source_block_id.workspace_id = workspace_id AND target_block_id.workspace_id = workspace_id AND target_block_id.source_rich_document_id != NONE AND edge_type = 'mention' AND last_actor_kind = 'SYSTEM' AND last_actor_id = 'knowledge_rich_document_backlink_projection' AND fn::mt120_document_access(source_document_id, record::id(workspace_id), 'update', 'fs.write') AND fn::mt120_document_access(record::id(target_block_id.source_rich_document_id), record::id(workspace_id), 'read', 'fs.read') OR fn::mt120_workspace_delete(record::id(workspace_id));"#,
    ),
    (
        r#"DEFINE TABLE OVERWRITE loom_block_search_index SCHEMAFULL
    PERMISSIONS FOR select WHERE block_id.source_rich_document_id != NONE AND block_id.workspace_id = workspace_id AND block_id.source_rich_document_id.workspace_id = workspace_id AND block_id.source_rich_document_id.deleted_at = NONE AND fn::mt120_document_access(record::id(block_id.source_rich_document_id), record::id(workspace_id), 'read', 'fs.read')
                FOR create WHERE block_id.source_rich_document_id != NONE AND block_id.workspace_id = workspace_id AND block_id.source_rich_document_id.workspace_id = workspace_id AND block_id.source_rich_document_id.deleted_at = NONE AND (fn::mt120_document_access(record::id(block_id.source_rich_document_id), record::id(workspace_id), 'create', 'fs.write') OR fn::mt120_document_access(record::id(block_id.source_rich_document_id), record::id(workspace_id), 'update', 'fs.write'))
                FOR update WHERE block_id.source_rich_document_id != NONE AND block_id.workspace_id = workspace_id AND block_id.source_rich_document_id.workspace_id = workspace_id AND block_id.source_rich_document_id.deleted_at = NONE AND fn::mt120_document_access(record::id(block_id.source_rich_document_id), record::id(workspace_id), 'update', 'fs.write')
                FOR delete WHERE block_id.source_rich_document_id != NONE AND block_id.workspace_id = workspace_id AND block_id.source_rich_document_id.workspace_id = workspace_id AND block_id.source_rich_document_id.deleted_at = NONE AND fn::mt120_document_access(record::id(block_id.source_rich_document_id), record::id(workspace_id), 'update', 'fs.write');"#,
        r#"DEFINE TABLE OVERWRITE loom_block_search_index SCHEMAFULL
    PERMISSIONS FOR select WHERE block_id.source_rich_document_id != NONE AND block_id.workspace_id = workspace_id AND block_id.source_rich_document_id.workspace_id = workspace_id AND block_id.source_rich_document_id.deleted_at = NONE AND fn::mt120_document_access(record::id(block_id.source_rich_document_id), record::id(workspace_id), 'read', 'fs.read')
                FOR create WHERE block_id.source_rich_document_id != NONE AND block_id.workspace_id = workspace_id AND block_id.source_rich_document_id.workspace_id = workspace_id AND block_id.source_rich_document_id.deleted_at = NONE AND (fn::mt120_document_access(record::id(block_id.source_rich_document_id), record::id(workspace_id), 'create', 'fs.write') OR fn::mt120_document_access(record::id(block_id.source_rich_document_id), record::id(workspace_id), 'update', 'fs.write'))
                FOR update WHERE block_id.source_rich_document_id != NONE AND block_id.workspace_id = workspace_id AND block_id.source_rich_document_id.workspace_id = workspace_id AND block_id.source_rich_document_id.deleted_at = NONE AND fn::mt120_document_access(record::id(block_id.source_rich_document_id), record::id(workspace_id), 'update', 'fs.write')
                FOR delete WHERE block_id.source_rich_document_id != NONE AND block_id.workspace_id = workspace_id AND block_id.source_rich_document_id.workspace_id = workspace_id AND block_id.source_rich_document_id.deleted_at = NONE AND fn::mt120_document_access(record::id(block_id.source_rich_document_id), record::id(workspace_id), 'update', 'fs.write') OR fn::mt120_workspace_delete(record::id(workspace_id));"#,
    ),
    (
        r#"DEFINE TABLE OVERWRITE loom_block_view_fr_outbox SCHEMAFULL PERMISSIONS NONE;"#,
        r#"DEFINE TABLE OVERWRITE loom_block_view_fr_outbox SCHEMAFULL PERMISSIONS FOR select, create, update NONE FOR delete WHERE fn::mt120_workspace_delete(record::id(workspace_id));"#,
    ),
    (
        r#"DEFINE TABLE OVERWRITE loom_canvas_visual_edges SCHEMAFULL PERMISSIONS NONE;"#,
        r#"DEFINE TABLE OVERWRITE loom_canvas_visual_edges SCHEMAFULL PERMISSIONS FOR select, create, update NONE FOR delete WHERE fn::mt120_workspace_delete(record::id(workspace_id));"#,
    ),
    (
        r#"DEFINE TABLE OVERWRITE loom_canvas_placements SCHEMAFULL PERMISSIONS NONE;"#,
        r#"DEFINE TABLE OVERWRITE loom_canvas_placements SCHEMAFULL PERMISSIONS FOR select, create, update NONE FOR delete WHERE fn::mt120_workspace_delete(record::id(workspace_id));"#,
    ),
    (
        r#"DEFINE TABLE OVERWRITE atelier_intake_item_loom_projection SCHEMAFULL PERMISSIONS NONE;"#,
        r#"DEFINE TABLE OVERWRITE atelier_intake_item_loom_projection SCHEMAFULL PERMISSIONS FOR select, create, update NONE FOR delete WHERE fn::mt120_workspace_delete(record::id(workspace_id));"#,
    ),
    (
        r#"DEFINE TABLE OVERWRITE knowledge_rich_documents SCHEMAFULL
    PERMISSIONS FOR select WHERE (deleted_at = NONE AND (fn::mt109_source_read('rich_document', rich_document_id, record::id(workspace_id), 'workspace', record::id(workspace_id)) OR fn::mt120_document_access(rich_document_id, record::id(workspace_id), 'read', 'fs.read'))) OR (deleted_at != NONE AND fn::mt120_document_access(rich_document_id, record::id(workspace_id), 'delete', 'fs.write'))
                FOR create WHERE fn::mt109_live_session() AND created_in_session_id = $auth.id AND deleted_at = NONE AND fn::mt109_has_workspace_access(record::id(workspace_id), 'create', 'fs.write')
                FOR update WHERE fn::mt120_document_access(rich_document_id, record::id(workspace_id), 'update', 'fs.write')
                FOR delete NONE;"#,
        r#"DEFINE TABLE OVERWRITE knowledge_rich_documents SCHEMAFULL
    PERMISSIONS FOR select WHERE (deleted_at = NONE AND (fn::mt109_source_read('rich_document', rich_document_id, record::id(workspace_id), 'workspace', record::id(workspace_id)) OR fn::mt120_document_access(rich_document_id, record::id(workspace_id), 'read', 'fs.read'))) OR (deleted_at != NONE AND fn::mt120_document_access(rich_document_id, record::id(workspace_id), 'delete', 'fs.write'))
                FOR create WHERE fn::mt109_live_session() AND created_in_session_id = $auth.id AND deleted_at = NONE AND fn::mt109_has_workspace_access(record::id(workspace_id), 'create', 'fs.write')
                FOR update WHERE (fn::mt120_document_access(rich_document_id, record::id(workspace_id), 'update', 'fs.write') AND $before.deleted_at = $after.deleted_at AND $before.deleted_receipt_event_id = $after.deleted_receipt_event_id) OR (fn::mt120_document_access(rich_document_id, record::id(workspace_id), 'delete', 'fs.write') AND $before.deleted_at = NONE AND $after.deleted_at != NONE AND $after.deleted_receipt_event_id != NONE AND $before.rich_document_id = $after.rich_document_id AND $before.workspace_id = $after.workspace_id AND $before.document_id = $after.document_id AND $before.title = $after.title AND $before.schema_version = $after.schema_version AND $before.doc_version = $after.doc_version AND $before.content_json = $after.content_json AND $before.content_sha256 = $after.content_sha256 AND $before.crdt_document_id = $after.crdt_document_id AND $before.crdt_snapshot_id = $after.crdt_snapshot_id AND $before.promotion_receipt_event_id = $after.promotion_receipt_event_id AND $before.project_ref = $after.project_ref AND $before.folder_ref = $after.folder_ref AND $before.authority_label = $after.authority_label AND $before.owner_actor_kind = $after.owner_actor_kind AND $before.owner_actor_id = $after.owner_actor_id AND $before.created_at = $after.created_at AND $before.created_in_session_id = $after.created_in_session_id)
                FOR delete NONE;"#,
    ),
    (
        r#"DEFINE TABLE OVERWRITE knowledge_rich_documents SCHEMAFULL
    PERMISSIONS FOR select WHERE (deleted_at = NONE AND (fn::mt109_source_read('rich_document', rich_document_id, record::id(workspace_id), 'workspace', record::id(workspace_id)) OR fn::mt120_document_access(rich_document_id, record::id(workspace_id), 'read', 'fs.read'))) OR (deleted_at != NONE AND fn::mt120_document_access(rich_document_id, record::id(workspace_id), 'delete', 'fs.write'))
                FOR create WHERE fn::mt109_live_session() AND created_in_session_id = $auth.id AND deleted_at = NONE AND fn::mt109_has_workspace_access(record::id(workspace_id), 'create', 'fs.write')
                FOR update WHERE (fn::mt120_document_access(rich_document_id, record::id(workspace_id), 'update', 'fs.write') AND $before.deleted_at = $after.deleted_at AND $before.deleted_receipt_event_id = $after.deleted_receipt_event_id) OR (fn::mt120_document_access(rich_document_id, record::id(workspace_id), 'delete', 'fs.write') AND $before.deleted_at = NONE AND $after.deleted_at != NONE AND $after.deleted_receipt_event_id != NONE AND $before.rich_document_id = $after.rich_document_id AND $before.workspace_id = $after.workspace_id AND $before.document_id = $after.document_id AND $before.title = $after.title AND $before.schema_version = $after.schema_version AND $before.doc_version = $after.doc_version AND $before.content_json = $after.content_json AND $before.content_sha256 = $after.content_sha256 AND $before.crdt_document_id = $after.crdt_document_id AND $before.crdt_snapshot_id = $after.crdt_snapshot_id AND $before.promotion_receipt_event_id = $after.promotion_receipt_event_id AND $before.project_ref = $after.project_ref AND $before.folder_ref = $after.folder_ref AND $before.authority_label = $after.authority_label AND $before.owner_actor_kind = $after.owner_actor_kind AND $before.owner_actor_id = $after.owner_actor_id AND $before.created_at = $after.created_at AND $before.created_in_session_id = $after.created_in_session_id)
                FOR delete NONE;"#,
        r#"DEFINE TABLE OVERWRITE knowledge_rich_documents SCHEMAFULL
    PERMISSIONS FOR select WHERE (deleted_at = NONE AND (fn::mt109_source_read('rich_document', rich_document_id, record::id(workspace_id), 'workspace', record::id(workspace_id)) OR fn::mt120_document_access(rich_document_id, record::id(workspace_id), 'read', 'fs.read'))) OR (deleted_at != NONE AND fn::mt120_document_access(rich_document_id, record::id(workspace_id), 'delete', 'fs.write'))
                FOR create WHERE fn::mt109_live_session() AND created_in_session_id = $auth.id AND deleted_at = NONE AND fn::mt109_has_workspace_access(record::id(workspace_id), 'create', 'fs.write')
                FOR update WHERE (fn::mt120_document_access(rich_document_id, record::id(workspace_id), 'update', 'fs.write') AND $before.deleted_at = $after.deleted_at AND $before.deleted_receipt_event_id = $after.deleted_receipt_event_id) OR (fn::mt120_document_delete(rich_document_id, record::id(workspace_id)) AND $before.deleted_at = NONE AND $after.deleted_at != NONE AND $after.deleted_receipt_event_id != NONE AND $before.rich_document_id = $after.rich_document_id AND $before.workspace_id = $after.workspace_id AND $before.document_id = $after.document_id AND $before.title = $after.title AND $before.schema_version = $after.schema_version AND $before.doc_version = $after.doc_version AND $before.content_json = $after.content_json AND $before.content_sha256 = $after.content_sha256 AND $before.crdt_document_id = $after.crdt_document_id AND $before.crdt_snapshot_id = $after.crdt_snapshot_id AND $before.promotion_receipt_event_id = $after.promotion_receipt_event_id AND $before.project_ref = $after.project_ref AND $before.folder_ref = $after.folder_ref AND $before.authority_label = $after.authority_label AND $before.owner_actor_kind = $after.owner_actor_kind AND $before.owner_actor_id = $after.owner_actor_id AND $before.created_at = $after.created_at AND $before.created_in_session_id = $after.created_in_session_id)
                FOR delete NONE;"#,
    ),
    (
        r#"DEFINE TABLE OVERWRITE loom_blocks SCHEMAFULL
    PERMISSIONS FOR select WHERE ((source_rich_document_id = NONE AND fn::mt109_source_read('loom_block', block_id, record::id(workspace_id), 'workspace', record::id(workspace_id))) OR (source_rich_document_id != NONE AND record::id(source_rich_document_id) = block_id AND content_type = 'note' AND fn::mt109_source_read('loom_block', block_id, record::id(workspace_id), 'rich_document', record::id(source_rich_document_id)) AND fn::mt109_source_read('rich_document', record::id(source_rich_document_id), record::id(workspace_id), 'workspace', record::id(workspace_id)) AND source_rich_document_id.workspace_id = workspace_id AND source_rich_document_id.deleted_at = NONE AND content_hash = source_rich_document_id.content_sha256)) OR (source_rich_document_id != NONE AND record::id(source_rich_document_id) = block_id AND content_type = 'note' AND source_rich_document_id.workspace_id = workspace_id AND source_rich_document_id.deleted_at = NONE AND content_hash = source_rich_document_id.content_sha256 AND fn::mt120_document_access(record::id(source_rich_document_id), record::id(workspace_id), 'read', 'fs.read'))
                FOR create WHERE source_rich_document_id != NONE AND record::id(source_rich_document_id) = block_id AND content_type = 'note' AND source_rich_document_id.workspace_id = workspace_id AND source_rich_document_id.deleted_at = NONE AND content_hash = source_rich_document_id.content_sha256 AND (fn::mt120_document_access(record::id(source_rich_document_id), record::id(workspace_id), 'create', 'fs.write') OR fn::mt120_document_access(record::id(source_rich_document_id), record::id(workspace_id), 'update', 'fs.write'))
                FOR update WHERE source_rich_document_id != NONE AND record::id(source_rich_document_id) = block_id AND content_type = 'note' AND source_rich_document_id.workspace_id = workspace_id AND source_rich_document_id.deleted_at = NONE AND content_hash = source_rich_document_id.content_sha256 AND fn::mt120_document_access(record::id(source_rich_document_id), record::id(workspace_id), 'update', 'fs.write')
                FOR delete WHERE fn::mt120_workspace_delete(record::id(workspace_id));"#,
        r#"DEFINE TABLE OVERWRITE loom_blocks SCHEMAFULL
    PERMISSIONS FOR select WHERE ((source_rich_document_id = NONE AND fn::mt109_source_read('loom_block', block_id, record::id(workspace_id), 'workspace', record::id(workspace_id))) OR (source_rich_document_id != NONE AND record::id(source_rich_document_id) = block_id AND content_type = 'note' AND fn::mt109_source_read('loom_block', block_id, record::id(workspace_id), 'rich_document', record::id(source_rich_document_id)) AND fn::mt109_source_read('rich_document', record::id(source_rich_document_id), record::id(workspace_id), 'workspace', record::id(workspace_id)) AND source_rich_document_id.workspace_id = workspace_id AND source_rich_document_id.deleted_at = NONE AND content_hash = source_rich_document_id.content_sha256)) OR (source_rich_document_id != NONE AND record::id(source_rich_document_id) = block_id AND content_type = 'note' AND source_rich_document_id.workspace_id = workspace_id AND source_rich_document_id.deleted_at = NONE AND content_hash = source_rich_document_id.content_sha256 AND fn::mt120_document_access(record::id(source_rich_document_id), record::id(workspace_id), 'read', 'fs.read'))
                FOR create WHERE source_rich_document_id != NONE AND record::id(source_rich_document_id) = block_id AND content_type = 'note' AND source_rich_document_id.workspace_id = workspace_id AND source_rich_document_id.deleted_at = NONE AND content_hash = source_rich_document_id.content_sha256 AND (fn::mt120_document_access(record::id(source_rich_document_id), record::id(workspace_id), 'create', 'fs.write') OR fn::mt120_document_access(record::id(source_rich_document_id), record::id(workspace_id), 'update', 'fs.write'))
                FOR update WHERE source_rich_document_id != NONE AND record::id(source_rich_document_id) = block_id AND content_type = 'note' AND source_rich_document_id.workspace_id = workspace_id AND source_rich_document_id.deleted_at = NONE AND content_hash = source_rich_document_id.content_sha256 AND fn::mt120_document_access(record::id(source_rich_document_id), record::id(workspace_id), 'update', 'fs.write')
                FOR delete WHERE (source_rich_document_id != NONE AND record::id(source_rich_document_id) = block_id AND source_rich_document_id.workspace_id = workspace_id AND content_type = 'note' AND fn::mt120_document_delete(block_id, record::id(workspace_id))) OR fn::mt120_workspace_delete(record::id(workspace_id));"#,
    ),
    (
        r#"DEFINE TABLE OVERWRITE loom_block_search_index SCHEMAFULL
    PERMISSIONS FOR select WHERE block_id.source_rich_document_id != NONE AND block_id.workspace_id = workspace_id AND block_id.source_rich_document_id.workspace_id = workspace_id AND block_id.source_rich_document_id.deleted_at = NONE AND fn::mt120_document_access(record::id(block_id.source_rich_document_id), record::id(workspace_id), 'read', 'fs.read')
                FOR create WHERE block_id.source_rich_document_id != NONE AND block_id.workspace_id = workspace_id AND block_id.source_rich_document_id.workspace_id = workspace_id AND block_id.source_rich_document_id.deleted_at = NONE AND (fn::mt120_document_access(record::id(block_id.source_rich_document_id), record::id(workspace_id), 'create', 'fs.write') OR fn::mt120_document_access(record::id(block_id.source_rich_document_id), record::id(workspace_id), 'update', 'fs.write'))
                FOR update WHERE block_id.source_rich_document_id != NONE AND block_id.workspace_id = workspace_id AND block_id.source_rich_document_id.workspace_id = workspace_id AND block_id.source_rich_document_id.deleted_at = NONE AND fn::mt120_document_access(record::id(block_id.source_rich_document_id), record::id(workspace_id), 'update', 'fs.write')
                FOR delete WHERE block_id.source_rich_document_id != NONE AND block_id.workspace_id = workspace_id AND block_id.source_rich_document_id.workspace_id = workspace_id AND block_id.source_rich_document_id.deleted_at = NONE AND fn::mt120_document_access(record::id(block_id.source_rich_document_id), record::id(workspace_id), 'update', 'fs.write') OR fn::mt120_workspace_delete(record::id(workspace_id));"#,
        r#"DEFINE TABLE OVERWRITE loom_block_search_index SCHEMAFULL
    PERMISSIONS FOR select WHERE block_id.source_rich_document_id != NONE AND block_id.workspace_id = workspace_id AND block_id.source_rich_document_id.workspace_id = workspace_id AND block_id.source_rich_document_id.deleted_at = NONE AND fn::mt120_document_access(record::id(block_id.source_rich_document_id), record::id(workspace_id), 'read', 'fs.read')
                FOR create WHERE block_id.source_rich_document_id != NONE AND block_id.workspace_id = workspace_id AND block_id.source_rich_document_id.workspace_id = workspace_id AND block_id.source_rich_document_id.deleted_at = NONE AND (fn::mt120_document_access(record::id(block_id.source_rich_document_id), record::id(workspace_id), 'create', 'fs.write') OR fn::mt120_document_access(record::id(block_id.source_rich_document_id), record::id(workspace_id), 'update', 'fs.write'))
                FOR update WHERE block_id.source_rich_document_id != NONE AND block_id.workspace_id = workspace_id AND block_id.source_rich_document_id.workspace_id = workspace_id AND block_id.source_rich_document_id.deleted_at = NONE AND fn::mt120_document_access(record::id(block_id.source_rich_document_id), record::id(workspace_id), 'update', 'fs.write')
                FOR delete WHERE block_id.source_rich_document_id != NONE AND block_id.workspace_id = workspace_id AND block_id.source_rich_document_id.workspace_id = workspace_id AND block_id.source_rich_document_id.deleted_at = NONE AND fn::mt120_document_access(record::id(block_id.source_rich_document_id), record::id(workspace_id), 'update', 'fs.write') OR fn::mt120_workspace_delete(record::id(workspace_id)) OR (block_id.source_rich_document_id != NONE AND block_id.workspace_id = workspace_id AND fn::mt120_document_delete(record::id(block_id.source_rich_document_id), record::id(workspace_id)));"#,
    ),
    (
        r#"DEFINE TABLE OVERWRITE knowledge_rich_document_title_anchors SCHEMAFULL
    PERMISSIONS FOR select WHERE fn::mt120_document_access(last_rich_document_id, record::id(workspace_id), 'read', 'fs.read')
                FOR create, update, delete NONE;"#,
        r#"DEFINE TABLE OVERWRITE knowledge_rich_document_title_anchors SCHEMAFULL
    PERMISSIONS FOR select WHERE fn::mt120_document_access(last_rich_document_id, record::id(workspace_id), 'read', 'fs.read')
                FOR create WHERE (fn::mt120_document_access(last_rich_document_id, record::id(workspace_id), 'update', 'fs.write') OR fn::mt120_document_access(last_rich_document_id, record::id(workspace_id), 'delete', 'fs.write')) FOR update WHERE (fn::mt120_document_access(last_rich_document_id, record::id(workspace_id), 'update', 'fs.write') OR fn::mt120_document_access(last_rich_document_id, record::id(workspace_id), 'delete', 'fs.write')) AND $before.workspace_id = $after.workspace_id AND $before.title_key = $after.title_key AND $before.anchor_key = $after.anchor_key FOR delete WHERE (fn::mt120_document_access(last_rich_document_id, record::id(workspace_id), 'update', 'fs.write') OR fn::mt120_document_access(last_rich_document_id, record::id(workspace_id), 'delete', 'fs.write'));"#,
    ),
    (
        r#"DEFINE TABLE OVERWRITE knowledge_spans SCHEMAFULL PERMISSIONS NONE;"#,
        r#"DEFINE TABLE OVERWRITE knowledge_spans SCHEMAFULL PERMISSIONS FOR select WHERE fn::mt120_index_source_read(source_id, source_id.workspace_id) FOR create, update, delete NONE;"#,
    ),
    (
        r#"DEFINE TABLE OVERWRITE knowledge_entities SCHEMAFULL PERMISSIONS NONE;"#,
        r#"DEFINE TABLE OVERWRITE knowledge_entities SCHEMAFULL PERMISSIONS FOR select WHERE fn::mt120_entity_read(id) FOR create, update, delete NONE;"#,
    ),
    (
        r#"DEFINE TABLE OVERWRITE knowledge_entity_spans SCHEMAFULL PERMISSIONS NONE;"#,
        r#"DEFINE TABLE OVERWRITE knowledge_entity_spans SCHEMAFULL PERMISSIONS FOR select WHERE fn::mt120_entity_read(entity_id) AND fn::mt120_index_source_read(span_id.source_id, entity_id.workspace_id) FOR create, update, delete NONE;"#,
    ),
    (
        r#"DEFINE TABLE OVERWRITE knowledge_edges SCHEMAFULL PERMISSIONS NONE;"#,
        r#"DEFINE TABLE OVERWRITE knowledge_edges SCHEMAFULL PERMISSIONS FOR select WHERE fn::mt120_edge_read(id) FOR create, update, delete NONE;"#,
    ),
    (
        r#"DEFINE TABLE OVERWRITE knowledge_edge_spans SCHEMAFULL PERMISSIONS NONE;"#,
        r#"DEFINE TABLE OVERWRITE knowledge_edge_spans SCHEMAFULL PERMISSIONS FOR select WHERE fn::mt120_edge_read(edge_id) AND fn::mt120_index_source_read(span_id.source_id, edge_id.workspace_id) FOR create, update, delete NONE;"#,
    ),
    (
        r#"DEFINE TABLE OVERWRITE kernel_event_ledger SCHEMAFULL
    PERMISSIONS FOR select WHERE (fn::mt109_ledger_receipt(authority_capability_id, event_type, source_component, aggregate_type, payload, wsids) AND fn::mt109_ledger_reader(authority_resource_id, authority_session_id, authority_capability_id, authority_action, wsids)) OR fn::mt120_document_receipt(authority_resource_id, authority_session_id, authority_capability_id, authority_action, wsids, event_type, source_component, aggregate_type, aggregate_id, payload, actor_kind, actor_id, false) OR fn::mt120_workspace_state_receipt(authority_resource_id, authority_session_id, authority_capability_id, authority_action, wsids, event_type, source_component, aggregate_type, aggregate_id, payload, actor_kind, actor_id, false)
                FOR create WHERE (fn::mt109_ledger_receipt(authority_capability_id, event_type, source_component, aggregate_type, payload, wsids) AND fn::mt109_ledger_producer(authority_resource_id, authority_session_id, authority_capability_id, authority_action, wsids)) OR fn::mt120_document_receipt(authority_resource_id, authority_session_id, authority_capability_id, authority_action, wsids, event_type, source_component, aggregate_type, aggregate_id, payload, actor_kind, actor_id, true) OR fn::mt120_workspace_state_receipt(authority_resource_id, authority_session_id, authority_capability_id, authority_action, wsids, event_type, source_component, aggregate_type, aggregate_id, payload, actor_kind, actor_id, true)
                FOR update, delete NONE;"#,
        r#"DEFINE TABLE OVERWRITE kernel_event_ledger SCHEMAFULL
    PERMISSIONS FOR select WHERE (fn::mt109_ledger_receipt(authority_capability_id, event_type, source_component, aggregate_type, payload, wsids) AND fn::mt109_ledger_reader(authority_resource_id, authority_session_id, authority_capability_id, authority_action, wsids)) OR fn::mt120_document_receipt(authority_resource_id, authority_session_id, authority_capability_id, authority_action, wsids, event_type, source_component, aggregate_type, aggregate_id, payload, actor_kind, actor_id, false) OR fn::mt120_workspace_state_receipt(authority_resource_id, authority_session_id, authority_capability_id, authority_action, wsids, event_type, source_component, aggregate_type, aggregate_id, payload, actor_kind, actor_id, false) OR fn::mt120_nav_receipt(authority_resource_id, authority_session_id, authority_capability_id, authority_action, wsids, event_type, source_component, aggregate_type, aggregate_id, payload, actor_kind, actor_id, session_run_id, false)
                FOR create WHERE (fn::mt109_ledger_receipt(authority_capability_id, event_type, source_component, aggregate_type, payload, wsids) AND fn::mt109_ledger_producer(authority_resource_id, authority_session_id, authority_capability_id, authority_action, wsids)) OR fn::mt120_document_receipt(authority_resource_id, authority_session_id, authority_capability_id, authority_action, wsids, event_type, source_component, aggregate_type, aggregate_id, payload, actor_kind, actor_id, true) OR fn::mt120_workspace_state_receipt(authority_resource_id, authority_session_id, authority_capability_id, authority_action, wsids, event_type, source_component, aggregate_type, aggregate_id, payload, actor_kind, actor_id, true) OR fn::mt120_nav_receipt(authority_resource_id, authority_session_id, authority_capability_id, authority_action, wsids, event_type, source_component, aggregate_type, aggregate_id, payload, actor_kind, actor_id, session_run_id, true)
                FOR update, delete NONE;"#,
    ),
    (
        r#"DEFINE TABLE OVERWRITE kernel_event_ledger SCHEMAFULL
    PERMISSIONS FOR select WHERE (fn::mt109_ledger_receipt(authority_capability_id, event_type, source_component, aggregate_type, payload, wsids) AND fn::mt109_ledger_reader(authority_resource_id, authority_session_id, authority_capability_id, authority_action, wsids)) OR fn::mt120_document_receipt(authority_resource_id, authority_session_id, authority_capability_id, authority_action, wsids, event_type, source_component, aggregate_type, aggregate_id, payload, actor_kind, actor_id, false) OR fn::mt120_workspace_state_receipt(authority_resource_id, authority_session_id, authority_capability_id, authority_action, wsids, event_type, source_component, aggregate_type, aggregate_id, payload, actor_kind, actor_id, false) OR fn::mt120_nav_receipt(authority_resource_id, authority_session_id, authority_capability_id, authority_action, wsids, event_type, source_component, aggregate_type, aggregate_id, payload, actor_kind, actor_id, session_run_id, false)
                FOR create WHERE (fn::mt109_ledger_receipt(authority_capability_id, event_type, source_component, aggregate_type, payload, wsids) AND fn::mt109_ledger_producer(authority_resource_id, authority_session_id, authority_capability_id, authority_action, wsids)) OR fn::mt120_document_receipt(authority_resource_id, authority_session_id, authority_capability_id, authority_action, wsids, event_type, source_component, aggregate_type, aggregate_id, payload, actor_kind, actor_id, true) OR fn::mt120_workspace_state_receipt(authority_resource_id, authority_session_id, authority_capability_id, authority_action, wsids, event_type, source_component, aggregate_type, aggregate_id, payload, actor_kind, actor_id, true) OR fn::mt120_nav_receipt(authority_resource_id, authority_session_id, authority_capability_id, authority_action, wsids, event_type, source_component, aggregate_type, aggregate_id, payload, actor_kind, actor_id, session_run_id, true)
                FOR update, delete NONE;"#,
        r#"DEFINE TABLE OVERWRITE kernel_event_ledger SCHEMAFULL
    PERMISSIONS FOR select WHERE (fn::mt109_ledger_receipt(authority_capability_id, event_type, source_component, aggregate_type, payload, wsids) AND fn::mt109_ledger_reader(authority_resource_id, authority_session_id, authority_capability_id, authority_action, wsids)) OR fn::mt120_document_receipt(authority_resource_id, authority_session_id, authority_capability_id, authority_action, wsids, event_type, source_component, aggregate_type, aggregate_id, payload, actor_kind, actor_id, false) OR fn::mt120_workspace_state_receipt(authority_resource_id, authority_session_id, authority_capability_id, authority_action, wsids, event_type, source_component, aggregate_type, aggregate_id, payload, actor_kind, actor_id, false) OR fn::mt120_nav_receipt(authority_resource_id, authority_session_id, authority_capability_id, authority_action, wsids, event_type, source_component, aggregate_type, aggregate_id, payload, actor_kind, actor_id, session_run_id, false) OR fn::mt120_nav_quiet_receipt(authority_resource_id, authority_session_id, authority_capability_id, authority_action, wsids, event_type, source_component, aggregate_type, aggregate_id, payload, actor_kind, actor_id, session_run_id, false)
                FOR create WHERE (fn::mt109_ledger_receipt(authority_capability_id, event_type, source_component, aggregate_type, payload, wsids) AND fn::mt109_ledger_producer(authority_resource_id, authority_session_id, authority_capability_id, authority_action, wsids)) OR fn::mt120_document_receipt(authority_resource_id, authority_session_id, authority_capability_id, authority_action, wsids, event_type, source_component, aggregate_type, aggregate_id, payload, actor_kind, actor_id, true) OR fn::mt120_workspace_state_receipt(authority_resource_id, authority_session_id, authority_capability_id, authority_action, wsids, event_type, source_component, aggregate_type, aggregate_id, payload, actor_kind, actor_id, true) OR fn::mt120_nav_receipt(authority_resource_id, authority_session_id, authority_capability_id, authority_action, wsids, event_type, source_component, aggregate_type, aggregate_id, payload, actor_kind, actor_id, session_run_id, true) OR fn::mt120_nav_quiet_receipt(authority_resource_id, authority_session_id, authority_capability_id, authority_action, wsids, event_type, source_component, aggregate_type, aggregate_id, payload, actor_kind, actor_id, session_run_id, true)
                FOR update, delete NONE;"#,
    ),
    (
        r#"DEFINE TABLE OVERWRITE knowledge_agent_quiet_background_work SCHEMAFULL PERMISSIONS NONE;"#,
        r#"DEFINE TABLE OVERWRITE knowledge_agent_quiet_background_work SCHEMAFULL PERMISSIONS FOR select WHERE fn::mt120_nav_quiet_row($this, false) FOR create WHERE fn::mt120_nav_quiet_row($this, true) FOR update, delete NONE;"#,
    ),
    (
        r#"DEFINE TABLE OVERWRITE resource_grants TYPE NORMAL SCHEMAFULL
  PERMISSIONS FOR select WHERE
    status = 'active'
    AND account_id = $auth.account_id
    AND principal_id = $auth.principal_id
    AND access_space_id = $auth.access_space_id,
  FOR create WHERE fn::mt120_creator_grant($this) FOR update WHERE fn::mt109_live_session() AND account_id = $auth.account_id AND principal_id = $auth.principal_id AND access_space_id = $auth.access_space_id AND status = 'active' AND revoked_at = NONE AND (expires_at = NONE OR expires_at > time::now()) AND policy_version <= $auth.policy_version AND $after.authorization_touch_nonce = ($before.authorization_touch_nonce ?? 0) + 1 AND $before.account_id = $after.account_id AND $before.principal_id = $after.principal_id AND $before.access_space_id = $after.access_space_id AND $before.resource_id = $after.resource_id AND $before.actions = $after.actions AND $before.capability_ids = $after.capability_ids AND $before.delegation_chain = $after.delegation_chain AND $before.status = $after.status AND $before.grant_version = $after.grant_version AND $before.policy_version = $after.policy_version AND $before.expires_at = $after.expires_at AND $before.revoked_at = $after.revoked_at AND $before.created_at = $after.created_at AND $before.updated_at = $after.updated_at FOR delete NONE;"#,
        r#"DEFINE TABLE OVERWRITE resource_grants TYPE NORMAL SCHEMAFULL
  PERMISSIONS FOR select WHERE
    status = 'active'
    AND account_id = $auth.account_id
    AND principal_id = $auth.principal_id
    AND access_space_id = $auth.access_space_id,
  FOR create WHERE fn::mt120_creator_grant($this) FOR update WHERE fn::mt109_live_session() AND account_id = $auth.account_id AND principal_id = $auth.principal_id AND access_space_id = $auth.access_space_id AND status = 'active' AND revoked_at = NONE AND (expires_at = NONE OR expires_at > time::now()) AND policy_version <= $auth.policy_version  AND delegation_chain = $auth.delegation_chain AND resource_id.lifecycle_state = 'active' AND resource_id.owner_account_id = $auth.account_id AND resource_id.access_space_id = $auth.access_space_id AND resource_id.created_by_principal_id.status = 'enabled' AND resource_id.policy_version <= policy_version AND ((capability_ids CONTAINS 'fs.write' AND (actions CONTAINS 'create' OR actions CONTAINS 'update' OR actions CONTAINS 'delete') AND ($auth.delegated_capabilities CONTAINS '*' OR $auth.delegated_capabilities CONTAINS 'fs.write') AND ($auth.principal_id.delegated_capabilities CONTAINS '*' OR $auth.principal_id.delegated_capabilities CONTAINS 'fs.write')) OR (capability_ids CONTAINS 'fr.read' AND actions CONTAINS 'read' AND ($auth.delegated_capabilities CONTAINS '*' OR $auth.delegated_capabilities CONTAINS 'fr.read') AND ($auth.principal_id.delegated_capabilities CONTAINS '*' OR $auth.principal_id.delegated_capabilities CONTAINS 'fr.read'))) AND $after.authorization_touch_nonce = ($before.authorization_touch_nonce ?? 0) + 1 AND $before.account_id = $after.account_id AND $before.principal_id = $after.principal_id AND $before.access_space_id = $after.access_space_id AND $before.resource_id = $after.resource_id AND $before.actions = $after.actions AND $before.capability_ids = $after.capability_ids AND $before.delegation_chain = $after.delegation_chain AND $before.status = $after.status AND $before.grant_version = $after.grant_version AND $before.policy_version = $after.policy_version AND $before.expires_at = $after.expires_at AND $before.revoked_at = $after.revoked_at AND $before.created_at = $after.created_at AND $before.updated_at = $after.updated_at FOR delete NONE;"#,
    ),
    (
        r#"DEFINE TABLE OVERWRITE knowledge_rich_documents SCHEMAFULL
    PERMISSIONS FOR select WHERE (deleted_at = NONE AND (fn::mt109_source_read('rich_document', rich_document_id, record::id(workspace_id), 'workspace', record::id(workspace_id)) OR fn::mt120_document_access(rich_document_id, record::id(workspace_id), 'read', 'fs.read'))) OR (deleted_at != NONE AND fn::mt120_document_access(rich_document_id, record::id(workspace_id), 'delete', 'fs.write'))
                FOR create WHERE fn::mt109_live_session() AND created_in_session_id = $auth.id AND deleted_at = NONE AND fn::mt109_has_workspace_access(record::id(workspace_id), 'create', 'fs.write')
                FOR update WHERE (fn::mt120_document_access(rich_document_id, record::id(workspace_id), 'update', 'fs.write') AND $before.deleted_at = $after.deleted_at AND $before.deleted_receipt_event_id = $after.deleted_receipt_event_id) OR (fn::mt120_document_delete(rich_document_id, record::id(workspace_id)) AND $before.deleted_at = NONE AND $after.deleted_at != NONE AND $after.deleted_receipt_event_id != NONE AND $before.rich_document_id = $after.rich_document_id AND $before.workspace_id = $after.workspace_id AND $before.document_id = $after.document_id AND $before.title = $after.title AND $before.schema_version = $after.schema_version AND $before.doc_version = $after.doc_version AND $before.content_json = $after.content_json AND $before.content_sha256 = $after.content_sha256 AND $before.crdt_document_id = $after.crdt_document_id AND $before.crdt_snapshot_id = $after.crdt_snapshot_id AND $before.promotion_receipt_event_id = $after.promotion_receipt_event_id AND $before.project_ref = $after.project_ref AND $before.folder_ref = $after.folder_ref AND $before.authority_label = $after.authority_label AND $before.owner_actor_kind = $after.owner_actor_kind AND $before.owner_actor_id = $after.owner_actor_id AND $before.created_at = $after.created_at AND $before.created_in_session_id = $after.created_in_session_id)
                FOR delete NONE;"#,
        r#"DEFINE TABLE OVERWRITE knowledge_rich_documents SCHEMAFULL
    PERMISSIONS FOR select WHERE (deleted_at = NONE AND (fn::mt109_source_read('rich_document', rich_document_id, record::id(workspace_id), 'workspace', record::id(workspace_id)) OR fn::mt120_document_access(rich_document_id, record::id(workspace_id), 'read', 'fs.read'))) OR (deleted_at != NONE AND fn::mt120_document_access(rich_document_id, record::id(workspace_id), 'delete', 'fs.write'))
                FOR create WHERE fn::mt109_live_session() AND created_in_session_id = $auth.id AND deleted_at = NONE AND fn::mt109_has_workspace_access(record::id(workspace_id), 'create', 'fs.write')
                FOR update WHERE (fn::mt120_document_access(rich_document_id, record::id(workspace_id), 'update', 'fs.write') AND $before.deleted_at = NONE AND $after.deleted_at = NONE AND $before.deleted_receipt_event_id = $after.deleted_receipt_event_id) OR (fn::mt120_document_delete(rich_document_id, record::id(workspace_id)) AND $before.deleted_at = NONE AND $after.deleted_at != NONE AND $after.deleted_receipt_event_id != NONE AND $after.deleted_receipt_event_id.event_type = 'KNOWLEDGE_RICH_DOCUMENT_DELETED' AND $after.deleted_receipt_event_id.aggregate_id = rich_document_id AND $after.deleted_receipt_event_id.authority_session_id = $auth.id AND array::len($after.projection_refs) = array::len($before.projection_refs) + 1 AND array::slice($after.projection_refs, 0, array::len($before.projection_refs)) = $before.projection_refs AND $before.rich_document_id = $after.rich_document_id AND $before.workspace_id = $after.workspace_id AND $before.document_id = $after.document_id AND $before.title = $after.title AND $before.schema_version = $after.schema_version AND $before.doc_version = $after.doc_version AND $before.content_json = $after.content_json AND $before.content_sha256 = $after.content_sha256 AND $before.crdt_document_id = $after.crdt_document_id AND $before.crdt_snapshot_id = $after.crdt_snapshot_id AND $before.promotion_receipt_event_id = $after.promotion_receipt_event_id AND $before.project_ref = $after.project_ref AND $before.folder_ref = $after.folder_ref AND $before.authority_label = $after.authority_label AND $before.owner_actor_kind = $after.owner_actor_kind AND $before.owner_actor_id = $after.owner_actor_id AND $before.created_at = $after.created_at AND $before.created_in_session_id = $after.created_in_session_id)
                FOR delete NONE;"#,
    ),
    (
        r#"DEFINE TABLE OVERWRITE knowledge_sources SCHEMAFULL
    PERMISSIONS FOR select WHERE source_kind = 'file' AND fn::mt109_source_read('knowledge_source', source_id, record::id(workspace_id), 'workspace', record::id(workspace_id))
                FOR create, update, delete NONE;"#,
        r#"DEFINE TABLE OVERWRITE knowledge_sources SCHEMAFULL
    PERMISSIONS FOR select WHERE source_kind = 'file' AND fn::mt109_source_read('knowledge_source', source_id, record::id(workspace_id), 'workspace', record::id(workspace_id))
                FOR create WHERE fn::mt109_live_session() AND created_in_session_id = $auth.id AND source_kind = 'file' AND (root_id = NONE OR root_id.workspace_id = workspace_id) AND fn::mt109_has_workspace_access(record::id(workspace_id), 'create', 'memory.propose') FOR update WHERE fn::mt109_has_grant('knowledge_source', source_id, 'update', 'memory.propose') AND $before.source_id = $after.source_id AND $before.workspace_id = $after.workspace_id AND $before.root_id = $after.root_id AND $before.source_kind = $after.source_kind AND $before.relative_path = $after.relative_path FOR delete NONE;"#,
    ),
    (
        r#"DEFINE TABLE OVERWRITE knowledge_code_files SCHEMAFULL
    PERMISSIONS FOR select WHERE fn::mt109_source_read('knowledge_code_file', code_file_id, record::id(workspace_id), 'knowledge_source', record::id(source_id)) AND fn::mt109_source_read('knowledge_source', record::id(source_id), record::id(workspace_id), 'workspace', record::id(workspace_id)) AND source_id.workspace_id = workspace_id AND source_id.source_kind = 'file'
                FOR create, update, delete NONE;"#,
        r#"DEFINE TABLE OVERWRITE knowledge_code_files SCHEMAFULL
    PERMISSIONS FOR select WHERE fn::mt109_source_read('knowledge_code_file', code_file_id, record::id(workspace_id), 'knowledge_source', record::id(source_id)) AND fn::mt109_source_read('knowledge_source', record::id(source_id), record::id(workspace_id), 'workspace', record::id(workspace_id)) AND source_id.workspace_id = workspace_id AND source_id.source_kind = 'file'
                FOR create WHERE fn::mt109_live_session() AND created_in_session_id = $auth.id AND source_id.workspace_id = workspace_id AND source_id.source_kind = 'file' AND fn::mt109_has_grant('knowledge_source', record::id(source_id), 'create', 'memory.propose') FOR update WHERE fn::mt109_has_grant('knowledge_code_file', code_file_id, 'update', 'memory.propose') AND $before.code_file_id = $after.code_file_id AND $before.workspace_id = $after.workspace_id AND $before.source_id = $after.source_id FOR delete NONE;"#,
    ),
    (
        r#"DEFINE TABLE OVERWRITE protected_resources TYPE NORMAL SCHEMAFULL
  PERMISSIONS FOR select WHERE
    lifecycle_state = 'active'
    AND owner_account_id = $auth.account_id
    AND access_space_id = $auth.access_space_id
    AND owner_account_id.status = 'enabled'
    AND created_by_principal_id.status = 'enabled'
    AND access_space_id.status = 'active'
    AND array::len((SELECT id FROM resource_grants WHERE resource_id = $parent.id AND status = 'active' AND revoked_at = NONE AND (expires_at = NONE OR expires_at > time::now()) AND account_id = $auth.account_id AND principal_id = $auth.principal_id AND access_space_id = $auth.access_space_id)) > 0,
  FOR create WHERE fn::mt120_resource_create($this) FOR update WHERE fn::mt109_live_session() AND owner_account_id = $auth.account_id AND access_space_id = $auth.access_space_id AND (fn::mt109_has_grant(resource_kind, external_resource_id, 'create', 'fs.write') OR fn::mt109_has_grant(resource_kind, external_resource_id, 'update', 'fs.write') OR fn::mt109_has_grant(resource_kind, external_resource_id, 'delete', 'fs.write') OR fn::mt109_has_grant(resource_kind, external_resource_id, 'read', 'fr.read')) AND $after.authorization_touch_nonce = ($before.authorization_touch_nonce ?? 0) + 1 AND $before.resource_kind = $after.resource_kind AND $before.external_resource_id = $after.external_resource_id AND $before.owner_account_id = $after.owner_account_id AND $before.created_by_principal_id = $after.created_by_principal_id AND $before.created_in_session_id = $after.created_in_session_id AND $before.access_space_id = $after.access_space_id AND $before.parent_resource_id = $after.parent_resource_id AND $before.schema_version = $after.schema_version AND $before.lifecycle_state = $after.lifecycle_state AND $before.policy_version = $after.policy_version AND $before.classification = $after.classification AND $before.storage_locator_hash = $after.storage_locator_hash AND $before.created_at = $after.created_at AND $before.updated_at = $after.updated_at AND $before.creator_grant_id = $after.creator_grant_id FOR delete NONE;"#,
        r#"DEFINE TABLE OVERWRITE protected_resources TYPE NORMAL SCHEMAFULL
  PERMISSIONS FOR select WHERE
    lifecycle_state = 'active'
    AND owner_account_id = $auth.account_id
    AND access_space_id = $auth.access_space_id
    AND owner_account_id.status = 'enabled'
    AND created_by_principal_id.status = 'enabled'
    AND access_space_id.status = 'active'
    AND array::len((SELECT id FROM resource_grants WHERE resource_id = $parent.id AND status = 'active' AND revoked_at = NONE AND (expires_at = NONE OR expires_at > time::now()) AND account_id = $auth.account_id AND principal_id = $auth.principal_id AND access_space_id = $auth.access_space_id)) > 0,
  FOR create WHERE fn::mt120_resource_create($this) FOR update WHERE fn::mt109_live_session() AND owner_account_id = $auth.account_id AND access_space_id = $auth.access_space_id AND (fn::mt109_has_grant(resource_kind, external_resource_id, 'create', 'fs.write') OR fn::mt109_has_grant(resource_kind, external_resource_id, 'update', 'fs.write') OR fn::mt109_has_grant(resource_kind, external_resource_id, 'delete', 'fs.write') OR fn::mt109_has_grant(resource_kind, external_resource_id, 'read', 'fr.read') OR fn::mt109_has_grant(resource_kind, external_resource_id, 'create', 'memory.propose') OR fn::mt109_has_grant(resource_kind, external_resource_id, 'update', 'memory.propose') OR fn::mt109_has_grant(resource_kind, external_resource_id, 'delete', 'memory.propose')) AND $after.authorization_touch_nonce = ($before.authorization_touch_nonce ?? 0) + 1 AND $before.resource_kind = $after.resource_kind AND $before.external_resource_id = $after.external_resource_id AND $before.owner_account_id = $after.owner_account_id AND $before.created_by_principal_id = $after.created_by_principal_id AND $before.created_in_session_id = $after.created_in_session_id AND $before.access_space_id = $after.access_space_id AND $before.parent_resource_id = $after.parent_resource_id AND $before.schema_version = $after.schema_version AND $before.lifecycle_state = $after.lifecycle_state AND $before.policy_version = $after.policy_version AND $before.classification = $after.classification AND $before.storage_locator_hash = $after.storage_locator_hash AND $before.created_at = $after.created_at AND $before.updated_at = $after.updated_at AND $before.creator_grant_id = $after.creator_grant_id FOR delete NONE;"#,
    ),
    (
        r#"DEFINE TABLE OVERWRITE resource_grants TYPE NORMAL SCHEMAFULL
  PERMISSIONS FOR select WHERE
    status = 'active'
    AND account_id = $auth.account_id
    AND principal_id = $auth.principal_id
    AND access_space_id = $auth.access_space_id,
  FOR create WHERE fn::mt120_creator_grant($this) FOR update WHERE fn::mt109_live_session() AND account_id = $auth.account_id AND principal_id = $auth.principal_id AND access_space_id = $auth.access_space_id AND status = 'active' AND revoked_at = NONE AND (expires_at = NONE OR expires_at > time::now()) AND policy_version <= $auth.policy_version  AND delegation_chain = $auth.delegation_chain AND resource_id.lifecycle_state = 'active' AND resource_id.owner_account_id = $auth.account_id AND resource_id.access_space_id = $auth.access_space_id AND resource_id.created_by_principal_id.status = 'enabled' AND resource_id.policy_version <= policy_version AND ((capability_ids CONTAINS 'fs.write' AND (actions CONTAINS 'create' OR actions CONTAINS 'update' OR actions CONTAINS 'delete') AND ($auth.delegated_capabilities CONTAINS '*' OR $auth.delegated_capabilities CONTAINS 'fs.write') AND ($auth.principal_id.delegated_capabilities CONTAINS '*' OR $auth.principal_id.delegated_capabilities CONTAINS 'fs.write')) OR (capability_ids CONTAINS 'fr.read' AND actions CONTAINS 'read' AND ($auth.delegated_capabilities CONTAINS '*' OR $auth.delegated_capabilities CONTAINS 'fr.read') AND ($auth.principal_id.delegated_capabilities CONTAINS '*' OR $auth.principal_id.delegated_capabilities CONTAINS 'fr.read'))) AND $after.authorization_touch_nonce = ($before.authorization_touch_nonce ?? 0) + 1 AND $before.account_id = $after.account_id AND $before.principal_id = $after.principal_id AND $before.access_space_id = $after.access_space_id AND $before.resource_id = $after.resource_id AND $before.actions = $after.actions AND $before.capability_ids = $after.capability_ids AND $before.delegation_chain = $after.delegation_chain AND $before.status = $after.status AND $before.grant_version = $after.grant_version AND $before.policy_version = $after.policy_version AND $before.expires_at = $after.expires_at AND $before.revoked_at = $after.revoked_at AND $before.created_at = $after.created_at AND $before.updated_at = $after.updated_at FOR delete NONE;"#,
        r#"DEFINE TABLE OVERWRITE resource_grants TYPE NORMAL SCHEMAFULL
  PERMISSIONS FOR select WHERE
    status = 'active'
    AND account_id = $auth.account_id
    AND principal_id = $auth.principal_id
    AND access_space_id = $auth.access_space_id,
  FOR create WHERE fn::mt120_creator_grant($this) FOR update WHERE fn::mt109_live_session() AND account_id = $auth.account_id AND principal_id = $auth.principal_id AND access_space_id = $auth.access_space_id AND status = 'active' AND revoked_at = NONE AND (expires_at = NONE OR expires_at > time::now()) AND policy_version <= $auth.policy_version  AND delegation_chain = $auth.delegation_chain AND resource_id.lifecycle_state = 'active' AND resource_id.owner_account_id = $auth.account_id AND resource_id.access_space_id = $auth.access_space_id AND resource_id.created_by_principal_id.status = 'enabled' AND resource_id.policy_version <= policy_version AND ((capability_ids CONTAINS 'fs.write' AND (actions CONTAINS 'create' OR actions CONTAINS 'update' OR actions CONTAINS 'delete') AND ($auth.delegated_capabilities CONTAINS '*' OR $auth.delegated_capabilities CONTAINS 'fs.write') AND ($auth.principal_id.delegated_capabilities CONTAINS '*' OR $auth.principal_id.delegated_capabilities CONTAINS 'fs.write')) OR (capability_ids CONTAINS 'memory.propose' AND (actions CONTAINS 'create' OR actions CONTAINS 'update' OR actions CONTAINS 'delete') AND ($auth.delegated_capabilities CONTAINS '*' OR $auth.delegated_capabilities CONTAINS 'memory.propose') AND ($auth.principal_id.delegated_capabilities CONTAINS '*' OR $auth.principal_id.delegated_capabilities CONTAINS 'memory.propose')) OR (capability_ids CONTAINS 'fr.read' AND actions CONTAINS 'read' AND ($auth.delegated_capabilities CONTAINS '*' OR $auth.delegated_capabilities CONTAINS 'fr.read') AND ($auth.principal_id.delegated_capabilities CONTAINS '*' OR $auth.principal_id.delegated_capabilities CONTAINS 'fr.read'))) AND $after.authorization_touch_nonce = ($before.authorization_touch_nonce ?? 0) + 1 AND $before.account_id = $after.account_id AND $before.principal_id = $after.principal_id AND $before.access_space_id = $after.access_space_id AND $before.resource_id = $after.resource_id AND $before.actions = $after.actions AND $before.capability_ids = $after.capability_ids AND $before.delegation_chain = $after.delegation_chain AND $before.status = $after.status AND $before.grant_version = $after.grant_version AND $before.policy_version = $after.policy_version AND $before.expires_at = $after.expires_at AND $before.revoked_at = $after.revoked_at AND $before.created_at = $after.created_at AND $before.updated_at = $after.updated_at FOR delete NONE;"#,
    ),
    (
        r#"DEFINE TABLE OVERWRITE knowledge_source_roots SCHEMAFULL PERMISSIONS NONE;"#,
        r#"DEFINE TABLE OVERWRITE knowledge_source_roots SCHEMAFULL PERMISSIONS FOR select WHERE created_in_session_id != NONE AND created_in_session_id.account_id = $auth.account_id AND created_in_session_id.access_space_id = $auth.access_space_id AND fn::mt109_has_workspace_access(record::id(workspace_id), 'read', 'memory.read') FOR create WHERE created_in_session_id = $auth.id AND fn::mt109_has_workspace_access(record::id(workspace_id), 'create', 'memory.propose') FOR update WHERE created_in_session_id != NONE AND created_in_session_id.account_id = $auth.account_id AND created_in_session_id.access_space_id = $auth.access_space_id AND fn::mt109_has_workspace_access(record::id(workspace_id), 'update', 'memory.propose') AND $before.workspace_id = $after.workspace_id AND $before.root_id = $after.root_id AND $before.repo_relative_path = $after.repo_relative_path FOR delete NONE;"#,
    ),
    (
        r#"DEFINE TABLE OVERWRITE knowledge_index_runs SCHEMAFULL PERMISSIONS NONE;"#,
        r#"DEFINE TABLE OVERWRITE knowledge_index_runs SCHEMAFULL PERMISSIONS FOR select WHERE fn::mt120_index_run_access($this, false) FOR create WHERE created_in_session_id = $auth.id AND fn::mt109_has_workspace_access(record::id(workspace_id), 'create', 'memory.propose') AND fn::mt120_index_run_access($this, true) AND actor_id = $auth.principal_id.actor_id AND actor_kind = $auth.principal_id.actor_kind AND start_receipt_event_id.authority_session_id = $auth.id FOR update WHERE fn::mt120_index_run_access($before, true) AND fn::mt120_index_run_access($after, true) AND $before.workspace_id = $after.workspace_id AND $before.root_id = $after.root_id AND $before.actor_id = $after.actor_id AND $before.actor_kind = $after.actor_kind AND $before.start_receipt_event_id = $after.start_receipt_event_id FOR delete NONE;"#,
    ),
    (
        r#"DEFINE TABLE OVERWRITE knowledge_spans SCHEMAFULL PERMISSIONS FOR select WHERE fn::mt120_index_source_read(source_id, source_id.workspace_id) FOR create, update, delete NONE;"#,
        r#"DEFINE TABLE OVERWRITE knowledge_spans SCHEMAFULL PERMISSIONS FOR select WHERE fn::mt120_index_source_read(source_id, source_id.workspace_id) FOR create WHERE fn::mt120_index_source_write(source_id, source_id.workspace_id) FOR update, delete NONE;"#,
    ),
    (
        r#"DEFINE TABLE OVERWRITE knowledge_entities SCHEMAFULL PERMISSIONS FOR select WHERE fn::mt120_entity_read(id) FOR create, update, delete NONE;"#,
        r#"DEFINE TABLE OVERWRITE knowledge_entities SCHEMAFULL PERMISSIONS FOR select WHERE fn::mt120_entity_read(id) FOR create WHERE fn::mt120_entity_write(id, primary_source_id, workspace_id) FOR update WHERE fn::mt120_entity_write(id, primary_source_id, workspace_id) AND fn::mt120_entity_write($before.id, $before.primary_source_id, $before.workspace_id) AND $before.workspace_id = $after.workspace_id AND $before.entity_kind = $after.entity_kind AND $before.entity_key = $after.entity_key FOR delete NONE;"#,
    ),
    (
        r#"DEFINE TABLE OVERWRITE knowledge_entity_spans SCHEMAFULL PERMISSIONS FOR select WHERE fn::mt120_entity_read(entity_id) AND fn::mt120_index_source_read(span_id.source_id, entity_id.workspace_id) FOR create, update, delete NONE;"#,
        r#"DEFINE TABLE OVERWRITE knowledge_entity_spans SCHEMAFULL PERMISSIONS FOR select WHERE fn::mt120_entity_read(entity_id) AND fn::mt120_index_source_read(span_id.source_id, entity_id.workspace_id) FOR create, delete WHERE fn::mt120_entity_write(entity_id, entity_id.primary_source_id, entity_id.workspace_id) AND fn::mt120_index_source_write(span_id.source_id, entity_id.workspace_id) FOR update NONE;"#,
    ),
    (
        r#"DEFINE TABLE OVERWRITE knowledge_edges SCHEMAFULL PERMISSIONS FOR select WHERE fn::mt120_edge_read(id) FOR create, update, delete NONE;"#,
        r#"DEFINE TABLE OVERWRITE knowledge_edges SCHEMAFULL PERMISSIONS FOR select WHERE fn::mt120_edge_read(id) FOR create WHERE fn::mt120_edge_write(id, source_entity_id, target_entity_id, workspace_id) FOR update WHERE fn::mt120_edge_write(id, source_entity_id, target_entity_id, workspace_id) AND $before.workspace_id = $after.workspace_id AND $before.source_entity_id = $after.source_entity_id AND $before.target_entity_id = $after.target_entity_id AND $before.relationship_id = $after.relationship_id FOR delete NONE;"#,
    ),
    (
        r#"DEFINE TABLE OVERWRITE knowledge_edge_spans SCHEMAFULL PERMISSIONS FOR select WHERE fn::mt120_edge_read(edge_id) AND fn::mt120_index_source_read(span_id.source_id, edge_id.workspace_id) FOR create, update, delete NONE;"#,
        r#"DEFINE TABLE OVERWRITE knowledge_edge_spans SCHEMAFULL PERMISSIONS FOR select WHERE fn::mt120_edge_read(edge_id) AND fn::mt120_index_source_read(span_id.source_id, edge_id.workspace_id) FOR create, delete WHERE fn::mt120_edge_write(edge_id, edge_id.source_entity_id, edge_id.target_entity_id, edge_id.workspace_id) AND fn::mt120_index_source_write(span_id.source_id, edge_id.workspace_id) FOR update NONE;"#,
    ),
    (
        r#"DEFINE TABLE OVERWRITE knowledge_ingestion_receipts SCHEMAFULL PERMISSIONS NONE;"#,
        r#"DEFINE TABLE OVERWRITE knowledge_ingestion_receipts SCHEMAFULL PERMISSIONS FOR select WHERE fn::mt120_index_source_read(source_id, workspace_id) FOR create WHERE fn::mt120_index_source_write(source_id, workspace_id) AND receipt_event_id != NONE AND receipt_event_id.authority_session_id = $auth.id AND receipt_event_id.payload.workspace_id = record::id(workspace_id) FOR update, delete NONE;"#,
    ),
    (
        r#"DEFINE TABLE OVERWRITE knowledge_ingestion_spans SCHEMAFULL PERMISSIONS NONE;"#,
        r#"DEFINE TABLE OVERWRITE knowledge_ingestion_spans SCHEMAFULL PERMISSIONS FOR select WHERE fn::mt120_index_source_read(source_id, workspace_id) AND receipt_id.source_id = source_id AND receipt_id.workspace_id = workspace_id FOR create WHERE fn::mt120_index_source_write(source_id, workspace_id) AND receipt_id.source_id = source_id AND receipt_id.workspace_id = workspace_id FOR update, delete NONE;"#,
    ),
    (
        r#"DEFINE TABLE OVERWRITE knowledge_ingestion_repair_queue SCHEMAFULL PERMISSIONS NONE;"#,
        r#"DEFINE TABLE OVERWRITE knowledge_ingestion_repair_queue SCHEMAFULL PERMISSIONS FOR select WHERE fn::mt120_index_source_read(source_id, workspace_id) FOR create WHERE fn::mt120_index_source_write(source_id, workspace_id) FOR update WHERE fn::mt120_index_source_write(source_id, workspace_id) AND $before.workspace_id = $after.workspace_id AND $before.source_id = $after.source_id FOR delete NONE;"#,
    ),
    (
        r#"DEFINE TABLE OVERWRITE knowledge_code_repair_queue SCHEMAFULL PERMISSIONS NONE;"#,
        r#"DEFINE TABLE OVERWRITE knowledge_code_repair_queue SCHEMAFULL PERMISSIONS FOR select WHERE fn::mt120_index_source_read(source_id, workspace_id) FOR create WHERE fn::mt120_index_source_write(source_id, workspace_id) FOR update WHERE fn::mt120_index_source_write(source_id, workspace_id) AND $before.workspace_id = $after.workspace_id AND $before.source_id = $after.source_id FOR delete NONE;"#,
    ),
    (
        r#"DEFINE TABLE OVERWRITE knowledge_ingestion_root_policies SCHEMAFULL PERMISSIONS NONE;"#,
        r#"DEFINE TABLE OVERWRITE knowledge_ingestion_root_policies SCHEMAFULL PERMISSIONS FOR select WHERE fn::mt109_has_workspace_access(record::id(workspace_id), 'create', 'memory.propose') FOR create, update, delete NONE;"#,
    ),
    (
        r#"DEFINE TABLE OVERWRITE knowledge_ingestion_policy_decisions SCHEMAFULL PERMISSIONS NONE;"#,
        r#"DEFINE TABLE OVERWRITE knowledge_ingestion_policy_decisions SCHEMAFULL PERMISSIONS FOR select WHERE fn::mt109_has_workspace_access(record::id(workspace_id), 'read', 'memory.read') AND receipt_event_id.authority_session_id.account_id = $auth.account_id AND receipt_event_id.authority_session_id.access_space_id = $auth.access_space_id FOR create WHERE fn::mt109_has_workspace_access(record::id(workspace_id), 'create', 'memory.propose') AND receipt_event_id.authority_session_id = $auth.id AND receipt_event_id.source_component = 'knowledge_ingestion' AND receipt_event_id.payload.kind = 'root_registration_policy_decision' AND receipt_event_id.payload.workspace_id = record::id(workspace_id) AND receipt_event_id.payload.candidate_path = candidate_path AND receipt_event_id.payload.verdict = verdict AND actor_id = $auth.principal_id.actor_id AND actor_kind = $auth.principal_id.actor_kind FOR update, delete NONE;"#,
    ),
    (
        r#"DEFINE TABLE OVERWRITE kernel_event_ledger SCHEMAFULL
    PERMISSIONS FOR select WHERE (fn::mt109_ledger_receipt(authority_capability_id, event_type, source_component, aggregate_type, payload, wsids) AND fn::mt109_ledger_reader(authority_resource_id, authority_session_id, authority_capability_id, authority_action, wsids)) OR fn::mt120_document_receipt(authority_resource_id, authority_session_id, authority_capability_id, authority_action, wsids, event_type, source_component, aggregate_type, aggregate_id, payload, actor_kind, actor_id, false) OR fn::mt120_workspace_state_receipt(authority_resource_id, authority_session_id, authority_capability_id, authority_action, wsids, event_type, source_component, aggregate_type, aggregate_id, payload, actor_kind, actor_id, false) OR fn::mt120_nav_receipt(authority_resource_id, authority_session_id, authority_capability_id, authority_action, wsids, event_type, source_component, aggregate_type, aggregate_id, payload, actor_kind, actor_id, session_run_id, false) OR fn::mt120_nav_quiet_receipt(authority_resource_id, authority_session_id, authority_capability_id, authority_action, wsids, event_type, source_component, aggregate_type, aggregate_id, payload, actor_kind, actor_id, session_run_id, false)
                FOR create WHERE (fn::mt109_ledger_receipt(authority_capability_id, event_type, source_component, aggregate_type, payload, wsids) AND fn::mt109_ledger_producer(authority_resource_id, authority_session_id, authority_capability_id, authority_action, wsids)) OR fn::mt120_document_receipt(authority_resource_id, authority_session_id, authority_capability_id, authority_action, wsids, event_type, source_component, aggregate_type, aggregate_id, payload, actor_kind, actor_id, true) OR fn::mt120_workspace_state_receipt(authority_resource_id, authority_session_id, authority_capability_id, authority_action, wsids, event_type, source_component, aggregate_type, aggregate_id, payload, actor_kind, actor_id, true) OR fn::mt120_nav_receipt(authority_resource_id, authority_session_id, authority_capability_id, authority_action, wsids, event_type, source_component, aggregate_type, aggregate_id, payload, actor_kind, actor_id, session_run_id, true) OR fn::mt120_nav_quiet_receipt(authority_resource_id, authority_session_id, authority_capability_id, authority_action, wsids, event_type, source_component, aggregate_type, aggregate_id, payload, actor_kind, actor_id, session_run_id, true)
                FOR update, delete NONE;"#,
        r#"DEFINE TABLE OVERWRITE kernel_event_ledger SCHEMAFULL
    PERMISSIONS FOR select WHERE (fn::mt109_ledger_receipt(authority_capability_id, event_type, source_component, aggregate_type, payload, wsids) AND fn::mt109_ledger_reader(authority_resource_id, authority_session_id, authority_capability_id, authority_action, wsids)) OR fn::mt120_document_receipt(authority_resource_id, authority_session_id, authority_capability_id, authority_action, wsids, event_type, source_component, aggregate_type, aggregate_id, payload, actor_kind, actor_id, false) OR fn::mt120_workspace_state_receipt(authority_resource_id, authority_session_id, authority_capability_id, authority_action, wsids, event_type, source_component, aggregate_type, aggregate_id, payload, actor_kind, actor_id, false) OR fn::mt120_nav_receipt(authority_resource_id, authority_session_id, authority_capability_id, authority_action, wsids, event_type, source_component, aggregate_type, aggregate_id, payload, actor_kind, actor_id, session_run_id, false) OR fn::mt120_nav_quiet_receipt(authority_resource_id, authority_session_id, authority_capability_id, authority_action, wsids, event_type, source_component, aggregate_type, aggregate_id, payload, actor_kind, actor_id, session_run_id, false) OR fn::mt120_index_receipt(authority_resource_id, authority_session_id, authority_capability_id, authority_action, wsids, event_type, source_component, aggregate_type, aggregate_id, payload, actor_kind, actor_id, session_run_id, false)
                FOR create WHERE (fn::mt109_ledger_receipt(authority_capability_id, event_type, source_component, aggregate_type, payload, wsids) AND fn::mt109_ledger_producer(authority_resource_id, authority_session_id, authority_capability_id, authority_action, wsids)) OR fn::mt120_document_receipt(authority_resource_id, authority_session_id, authority_capability_id, authority_action, wsids, event_type, source_component, aggregate_type, aggregate_id, payload, actor_kind, actor_id, true) OR fn::mt120_workspace_state_receipt(authority_resource_id, authority_session_id, authority_capability_id, authority_action, wsids, event_type, source_component, aggregate_type, aggregate_id, payload, actor_kind, actor_id, true) OR fn::mt120_nav_receipt(authority_resource_id, authority_session_id, authority_capability_id, authority_action, wsids, event_type, source_component, aggregate_type, aggregate_id, payload, actor_kind, actor_id, session_run_id, true) OR fn::mt120_nav_quiet_receipt(authority_resource_id, authority_session_id, authority_capability_id, authority_action, wsids, event_type, source_component, aggregate_type, aggregate_id, payload, actor_kind, actor_id, session_run_id, true) OR fn::mt120_index_receipt(authority_resource_id, authority_session_id, authority_capability_id, authority_action, wsids, event_type, source_component, aggregate_type, aggregate_id, payload, actor_kind, actor_id, session_run_id, true)
                FOR update, delete NONE;"#,
    ),
    (
        r#"DEFINE TABLE OVERWRITE knowledge_sources SCHEMAFULL
    PERMISSIONS FOR select WHERE source_kind = 'file' AND fn::mt109_source_read('knowledge_source', source_id, record::id(workspace_id), 'workspace', record::id(workspace_id))
                FOR create WHERE fn::mt109_live_session() AND created_in_session_id = $auth.id AND source_kind = 'file' AND (root_id = NONE OR root_id.workspace_id = workspace_id) AND fn::mt109_has_workspace_access(record::id(workspace_id), 'create', 'memory.propose') FOR update WHERE fn::mt109_has_grant('knowledge_source', source_id, 'update', 'memory.propose') AND $before.source_id = $after.source_id AND $before.workspace_id = $after.workspace_id AND $before.root_id = $after.root_id AND $before.source_kind = $after.source_kind AND $before.relative_path = $after.relative_path FOR delete NONE;"#,
        r#"DEFINE TABLE OVERWRITE knowledge_sources SCHEMAFULL
    PERMISSIONS FOR select WHERE source_kind = 'file' AND fn::mt109_source_read('knowledge_source', source_id, record::id(workspace_id), 'workspace', record::id(workspace_id))
                FOR create WHERE fn::mt109_live_session() AND created_in_session_id = $auth.id AND source_kind = 'file' AND (root_id = NONE OR (root_id.workspace_id = workspace_id AND root_id.created_in_session_id != NONE AND root_id.created_in_session_id.account_id = $auth.account_id AND root_id.created_in_session_id.access_space_id = $auth.access_space_id)) AND fn::mt109_has_workspace_access(record::id(workspace_id), 'create', 'memory.propose') FOR update WHERE fn::mt109_has_grant('knowledge_source', source_id, 'update', 'memory.propose') AND $before.source_id = $after.source_id AND $before.workspace_id = $after.workspace_id AND $before.root_id = $after.root_id AND $before.source_kind = $after.source_kind AND $before.relative_path = $after.relative_path FOR delete NONE;"#,
    ),
    (
        r#"DEFINE TABLE OVERWRITE knowledge_index_runs SCHEMAFULL PERMISSIONS FOR select WHERE fn::mt120_index_run_access($this, false) FOR create WHERE created_in_session_id = $auth.id AND fn::mt109_has_workspace_access(record::id(workspace_id), 'create', 'memory.propose') AND fn::mt120_index_run_access($this, true) AND actor_id = $auth.principal_id.actor_id AND actor_kind = $auth.principal_id.actor_kind AND start_receipt_event_id.authority_session_id = $auth.id FOR update WHERE fn::mt120_index_run_access($before, true) AND fn::mt120_index_run_access($after, true) AND $before.workspace_id = $after.workspace_id AND $before.root_id = $after.root_id AND $before.actor_id = $after.actor_id AND $before.actor_kind = $after.actor_kind AND $before.start_receipt_event_id = $after.start_receipt_event_id FOR delete NONE;"#,
        r#"DEFINE TABLE OVERWRITE knowledge_index_runs SCHEMAFULL PERMISSIONS FOR select WHERE fn::mt120_index_run_access($this, false) FOR create WHERE created_in_session_id = $auth.id AND (root_id = NONE OR (root_id.workspace_id = workspace_id AND root_id.created_in_session_id != NONE AND root_id.created_in_session_id.account_id = $auth.account_id AND root_id.created_in_session_id.access_space_id = $auth.access_space_id)) AND fn::mt109_has_workspace_access(record::id(workspace_id), 'create', 'memory.propose') AND fn::mt120_index_run_access($this, true) AND actor_id = $auth.principal_id.actor_id AND actor_kind = $auth.principal_id.actor_kind AND start_receipt_event_id.authority_session_id = $auth.id FOR update WHERE fn::mt120_index_run_access($before, true) AND fn::mt120_index_run_access($after, true) AND $before.workspace_id = $after.workspace_id AND $before.root_id = $after.root_id AND $before.actor_id = $after.actor_id AND $before.actor_kind = $after.actor_kind AND $before.start_receipt_event_id = $after.start_receipt_event_id FOR delete NONE;"#,
    ),
];
const RECORD_USER_PRODUCER_BLOCK: &str = r#"-- RECORD_USER_PRODUCER_BEGIN
DEFINE FIELD OVERWRITE created_in_session_id ON TABLE knowledge_sources TYPE option<record<authenticated_sessions>> PERMISSIONS FOR select, create FULL FOR update NONE;
DEFINE FIELD OVERWRITE created_in_session_id ON TABLE knowledge_code_files TYPE option<record<authenticated_sessions>> PERMISSIONS FOR select, create FULL FOR update NONE;
DEFINE FIELD OVERWRITE created_in_session_id ON TABLE workspaces TYPE option<record<authenticated_sessions>> PERMISSIONS FOR select, create FULL FOR update NONE;
DEFINE FIELD OVERWRITE created_in_session_id ON TABLE knowledge_rich_documents TYPE option<record<authenticated_sessions>> PERMISSIONS FOR select, create FULL FOR update NONE;
DEFINE FIELD OVERWRITE creator_grant_id ON TABLE protected_resources TYPE option<record<resource_grants>> PERMISSIONS FOR select, create FULL FOR update NONE;
DEFINE FIELD OVERWRITE authorization_touch_nonce ON TABLE authenticated_sessions TYPE int DEFAULT 0 ASSERT $value >= 0;
DEFINE FIELD OVERWRITE authorization_touch_nonce ON TABLE local_accounts TYPE int DEFAULT 0 ASSERT $value >= 0;
DEFINE FIELD OVERWRITE authorization_touch_nonce ON TABLE principals TYPE int DEFAULT 0 ASSERT $value >= 0;
DEFINE FIELD OVERWRITE authorization_touch_nonce ON TABLE access_spaces TYPE int DEFAULT 0 ASSERT $value >= 0;
DEFINE FIELD OVERWRITE authorization_touch_nonce ON TABLE protected_resources TYPE int DEFAULT 0 ASSERT $value >= 0;
DEFINE FIELD OVERWRITE authorization_touch_nonce ON TABLE resource_grants TYPE int DEFAULT 0 ASSERT $value >= 0;
DEFINE FUNCTION OVERWRITE fn::mt120_workspace_create($session: option<record<authenticated_sessions>>) {
    RETURN fn::mt109_live_session() AND $session = $auth.id
        AND $auth.account_id.account_role = 'Owner'
        AND $auth.principal_id.principal_kind = 'human_account'
        AND ($auth.delegated_capabilities CONTAINS '*' OR $auth.delegated_capabilities CONTAINS 'fs.write')
        AND ($auth.principal_id.delegated_capabilities CONTAINS '*' OR $auth.principal_id.delegated_capabilities CONTAINS 'fs.write');
};
DEFINE FUNCTION OVERWRITE fn::mt120_resource_create($row: object) {
    IF !fn::mt109_live_session() OR $row.created_in_session_id != $auth.id
        OR $row.owner_account_id != $auth.account_id OR $row.created_by_principal_id != $auth.principal_id
        OR $row.access_space_id != $auth.access_space_id OR $row.lifecycle_state != 'active'
        OR $row.classification != 'account_private' OR $row.schema_version != 1
        OR $row.policy_version != $auth.policy_version OR $row.creator_grant_id = NONE
        OR record::exists($row.creator_grant_id) { RETURN false; };
    IF $row.resource_kind = 'workspace' {
        RETURN fn::mt120_workspace_create($row.created_in_session_id)
            AND $row.parent_resource_id = NONE
            AND type::record('workspaces', $row.external_resource_id).created_in_session_id = $auth.id;
    };
    IF $row.resource_kind = 'rich_document' {
        LET $doc = type::record('knowledge_rich_documents', $row.external_resource_id);
        RETURN $doc.created_in_session_id = $auth.id
            AND $row.parent_resource_id.resource_kind = 'workspace'
            AND $row.parent_resource_id.external_resource_id = record::id($doc.workspace_id)
            AND $row.parent_resource_id.owner_account_id = $auth.account_id
            AND $row.parent_resource_id.access_space_id = $auth.access_space_id
            AND fn::mt109_has_workspace_access(record::id($doc.workspace_id), 'create', 'fs.write');
    };
    IF $row.resource_kind = 'loom_block' {
        LET $block = type::record('loom_blocks', $row.external_resource_id);
        RETURN $block.created_in_session_id = $auth.id AND $block.source_rich_document_id = NONE
            AND $block.content_type IN ['note', 'file', 'annotated_file', 'tag_hub', 'journal', 'canvas']
            AND $row.parent_resource_id.resource_kind = 'workspace'
            AND $row.parent_resource_id.external_resource_id = record::id($block.workspace_id)
            AND $row.parent_resource_id.owner_account_id = $auth.account_id
            AND $row.parent_resource_id.access_space_id = $auth.access_space_id
            AND fn::mt109_has_workspace_access(record::id($block.workspace_id), 'create', 'fs.write');
    };
    IF $row.resource_kind = 'knowledge_source' {
        LET $source = type::record('knowledge_sources', $row.external_resource_id);
        RETURN $source.created_in_session_id = $auth.id AND $source.source_kind IN ['file', 'rich_document']
            AND $row.parent_resource_id.resource_kind = 'workspace'
            AND $row.parent_resource_id.external_resource_id = record::id($source.workspace_id)
            AND $row.parent_resource_id.owner_account_id = $auth.account_id
            AND $row.parent_resource_id.access_space_id = $auth.access_space_id
            AND fn::mt109_has_workspace_access(record::id($source.workspace_id), 'create', 'memory.propose');
    };
    IF $row.resource_kind = 'knowledge_code_file' {
        LET $file = type::record('knowledge_code_files', $row.external_resource_id);
        RETURN $file.created_in_session_id = $auth.id AND $file.source_id.workspace_id = $file.workspace_id
            AND $file.source_id.source_kind = 'file' AND $row.parent_resource_id.resource_kind = 'knowledge_source'
            AND $row.parent_resource_id.external_resource_id = record::id($file.source_id)
            AND $row.parent_resource_id.owner_account_id = $auth.account_id
            AND $row.parent_resource_id.access_space_id = $auth.access_space_id
            AND fn::mt109_has_grant('knowledge_source', record::id($file.source_id), 'create', 'memory.propose');
    };
    RETURN $row.resource_kind IN ['flight_recorder', 'memory_pack', 'memory_proposal', 'memory_commit_report', 'memory_item', 'memory_item_count']
        AND $row.parent_resource_id.resource_kind = 'workspace'
        AND $row.parent_resource_id.created_in_session_id = $auth.id
        AND $row.parent_resource_id.owner_account_id = $auth.account_id
        AND $row.parent_resource_id.access_space_id = $auth.access_space_id
        AND $row.parent_resource_id.external_resource_id = $row.external_resource_id
        AND type::record('workspaces', $row.external_resource_id).created_in_session_id = $auth.id;
};
DEFINE FUNCTION OVERWRITE fn::mt120_creator_grant($row: object) {
    RETURN fn::mt109_live_session() AND $row.id = $row.resource_id.creator_grant_id
        AND $row.resource_id.created_in_session_id = $auth.id
        AND $row.resource_id.owner_account_id = $auth.account_id
        AND $row.resource_id.created_by_principal_id = $auth.principal_id
        AND $row.resource_id.access_space_id = $auth.access_space_id
        AND $row.resource_id.lifecycle_state = 'active'
        AND $row.account_id = $auth.account_id AND $row.principal_id = $auth.principal_id
        AND $row.access_space_id = $auth.access_space_id AND $row.delegation_chain = $auth.delegation_chain
        AND $row.status = 'active' AND $row.grant_version = 1
        AND $row.revoked_at = NONE AND $row.expires_at = NONE
        AND $row.resource_id.policy_version <= $row.policy_version AND $row.policy_version <= $auth.policy_version
        AND ($auth.delegated_capabilities CONTAINS '*' OR $auth.delegated_capabilities CONTAINSALL $row.capability_ids)
        AND ($auth.principal_id.delegated_capabilities CONTAINS '*' OR $auth.principal_id.delegated_capabilities CONTAINSALL $row.capability_ids)
        AND (($row.resource_id.resource_kind = 'workspace'
             AND ['create','read','update','delete'] CONTAINSALL $row.actions
             AND ['fs.read','fs.write','fr.read','fr.ingest.runtime_chat','fr.ingest.native_editor','memory.read','memory.propose','memory.review','memory.commit'] CONTAINSALL $row.capability_ids)
          OR ($row.resource_id.resource_kind IN ['memory_pack','memory_proposal','memory_commit_report','memory_item','memory_item_count'] AND ['create','read','update','delete'] CONTAINSALL $row.actions AND ['memory.read','memory.propose','memory.review','memory.commit','fs.write'] CONTAINSALL $row.capability_ids AND $row.actions CONTAINS 'delete' AND $row.capability_ids CONTAINS 'fs.write')
          OR ($row.resource_id.resource_kind = 'rich_document' AND ['create','read','update','delete'] CONTAINSALL $row.actions AND ['fs.read','fs.write'] CONTAINSALL $row.capability_ids)
          OR ($row.resource_id.resource_kind = 'loom_block' AND ['create','read','update','delete'] CONTAINSALL $row.actions AND ['fs.read','fs.write'] CONTAINSALL $row.capability_ids)
          OR ($row.resource_id.resource_kind IN ['knowledge_source','knowledge_code_file'] AND ['create','read','update','delete'] CONTAINSALL $row.actions AND ['memory.read','memory.propose','fs.write'] CONTAINSALL $row.capability_ids AND ((!(($row.actions CONTAINS 'delete') OR ($row.capability_ids CONTAINS 'fs.write'))) OR (($row.capability_ids CONTAINS 'fs.write') AND fn::mt109_has_grant($row.resource_id.parent_resource_id.resource_kind, $row.resource_id.parent_resource_id.external_resource_id, 'delete', 'fs.write'))))
          OR ($row.resource_id.resource_kind = 'flight_recorder' AND ['create','read'] CONTAINSALL $row.actions
             AND ['fr.read','fr.ingest.runtime_chat','fr.ingest.native_editor'] CONTAINSALL $row.capability_ids));
};
DEFINE FUNCTION OVERWRITE fn::mt120_workspace_delete($external: string) {
    IF !fn::mt109_has_workspace_access($external, 'delete', 'fs.write') { RETURN false; };
    LET $workspace = type::record('workspaces', $external);
    LET $account = $auth.account_id;
    LET $principal = $auth.principal_id;
    LET $space = $auth.access_space_id;
    LET $live = $auth;
    LET $resource = (SELECT VALUE id FROM protected_resources WHERE resource_kind = 'workspace'
        AND external_resource_id = $external AND owner_account_id = $account AND access_space_id = $space
        AND lifecycle_state = 'active')[0];
    IF $resource = NONE OR !record::exists($workspace) { RETURN false; };
LET $resources = SELECT * FROM protected_resources WHERE id = $resource
    OR parent_resource_id = $resource OR parent_resource_id.parent_resource_id = $resource;
FOR $owned IN $resources {
    IF $owned.owner_account_id != $account OR $owned.access_space_id != $space { RETURN false; };
    IF $owned.policy_version > $live.policy_version OR $owned.created_by_principal_id.status != 'enabled'
        OR $owned.lifecycle_state != 'active' { RETURN false; };
    IF $owned.resource_kind != 'flight_recorder' {
        LET $child_grants = SELECT id FROM resource_grants WHERE resource_id = $owned.id AND account_id = $account
            AND principal_id = $principal AND access_space_id = $space AND status = 'active' AND revoked_at = NONE
            AND (expires_at = NONE OR expires_at > time::now()) AND delegation_chain = $live.delegation_chain
            AND actions CONTAINS 'delete' AND capability_ids CONTAINS 'fs.write' AND resource_id.policy_version <= policy_version AND policy_version <= $live.policy_version;
        IF array::len($child_grants) = 0 { RETURN false; };
    } ELSE {
        IF $owned.parent_resource_id != $resource OR $owned.external_resource_id != $external { RETURN false; };
    };

    IF array::len(SELECT id FROM protected_resources WHERE parent_resource_id = $owned.id AND id NOT IN $resources.id) > 0 {
        RETURN false;
    };
};
FOR $document IN (SELECT rich_document_id FROM knowledge_rich_documents WHERE workspace_id = $workspace) {
    IF array::len(SELECT id FROM protected_resources WHERE resource_kind = 'rich_document'
        AND external_resource_id = $document.rich_document_id AND parent_resource_id = $resource
        AND owner_account_id = $account AND access_space_id = $space AND lifecycle_state = 'active') != 1 {
        RETURN false;
    };
};
FOR $block IN (SELECT block_id, source_rich_document_id, content_type, content_hash FROM loom_blocks WHERE workspace_id = $workspace) {
    IF $block.source_rich_document_id != NONE {
        IF record::id($block.source_rich_document_id) != $block.block_id OR $block.content_type != 'note'
            OR $block.source_rich_document_id.workspace_id != $workspace
            OR $block.content_hash != $block.source_rich_document_id.content_sha256
            OR array::len(SELECT id FROM protected_resources WHERE resource_kind = 'rich_document'
                AND external_resource_id = $block.block_id AND parent_resource_id = $resource
                AND owner_account_id = $account AND access_space_id = $space AND lifecycle_state = 'active') != 1 {
            RETURN false;
        };
    } ELSE {
        IF array::len(SELECT id FROM protected_resources WHERE resource_kind = 'loom_block' AND external_resource_id = $block.block_id
            AND id IN $resources.id AND owner_account_id = $account AND access_space_id = $space AND lifecycle_state = 'active') != 1 {
            RETURN false;
        };
    };
};
-- Legacy generic sources have no account-bound resource kind in this producer; never delete them by workspace membership alone.
IF array::len(SELECT id FROM documents WHERE workspace_id = $workspace) > 0
    OR array::len(SELECT id FROM assets WHERE workspace_id = $workspace) > 0
    OR array::len(SELECT id FROM canvases WHERE workspace_id = $workspace) > 0 {
    RETURN false;
};
IF array::len(SELECT id FROM atelier_intake_item_loom_projection WHERE workspace_id = $workspace) > 0 { RETURN false; };
IF array::len(SELECT id FROM ai_bronze_records WHERE workspace_id = $workspace) > 0 { RETURN false; };
IF array::len(SELECT id FROM ai_silver_records WHERE workspace_id = $workspace) > 0 { RETURN false; };
IF array::len(SELECT id FROM assets WHERE workspace_id = $workspace) > 0 { RETURN false; };
IF array::len(SELECT id FROM blocks WHERE document_id IN (SELECT VALUE id FROM documents WHERE workspace_id = $workspace)) > 0 { RETURN false; };
IF array::len(SELECT id FROM calendar_activity_spans WHERE workspace_id = $workspace) > 0 { RETURN false; };
IF array::len(SELECT id FROM calendar_events WHERE workspace_id = $workspace) > 0 { RETURN false; };
IF array::len(SELECT id FROM calendar_sources WHERE workspace_id = $workspace) > 0 { RETURN false; };
IF array::len(SELECT id FROM canvas_edges WHERE canvas_id IN (SELECT VALUE id FROM canvases WHERE workspace_id = $workspace)) > 0 { RETURN false; };
IF array::len(SELECT id FROM canvas_nodes WHERE canvas_id IN (SELECT VALUE id FROM canvases WHERE workspace_id = $workspace)) > 0 { RETURN false; };
IF array::len(SELECT id FROM canvases WHERE workspace_id = $workspace) > 0 { RETURN false; };
IF array::len(SELECT id FROM documents WHERE workspace_id = $workspace) > 0 { RETURN false; };
IF array::len(SELECT id FROM knowledge_claim_conflicts WHERE claim_id IN (SELECT VALUE id FROM knowledge_claims WHERE workspace_id = $workspace)) > 0 { RETURN false; };
IF array::len(SELECT id FROM knowledge_claim_spans WHERE claim_id IN (SELECT VALUE id FROM knowledge_claims WHERE workspace_id = $workspace)) > 0 { RETURN false; };
IF array::len(SELECT id FROM knowledge_claims WHERE workspace_id = $workspace) > 0 { RETURN false; };
IF array::len(SELECT id FROM knowledge_code_scip_imports WHERE workspace_id = $workspace) > 0 { RETURN false; };
IF array::len(SELECT id FROM knowledge_context_bundle_items WHERE bundle_id IN (SELECT VALUE id FROM knowledge_context_bundles WHERE workspace_id = $workspace)) > 0 { RETURN false; };
IF array::len(SELECT id FROM knowledge_context_bundles WHERE workspace_id = $workspace) > 0 { RETURN false; };
IF array::len(SELECT id FROM knowledge_debug_breakpoints WHERE workspace_id = $workspace) > 0 { RETURN false; };
IF array::len(SELECT id FROM knowledge_memory_bridge_decisions WHERE workspace_id = $workspace) > 0 { RETURN false; };
IF array::len(SELECT id FROM knowledge_memory_conflict_detection_findings WHERE job_id IN (SELECT VALUE id FROM knowledge_memory_conflict_detection_jobs WHERE workspace_id = $workspace)) > 0 { RETURN false; };
IF array::len(SELECT id FROM knowledge_memory_conflict_detection_jobs WHERE workspace_id = $workspace) > 0 { RETURN false; };
IF array::len(SELECT id FROM knowledge_memory_conflict_resolution_jobs WHERE workspace_id = $workspace) > 0 { RETURN false; };
IF array::len(SELECT id FROM knowledge_memory_facts WHERE workspace_id = $workspace) > 0 { RETURN false; };
IF array::len(SELECT id FROM knowledge_memory_ontology_aliases WHERE workspace_id = $workspace) > 0 { RETURN false; };
IF array::len(SELECT id FROM knowledge_memory_ontology_terms WHERE workspace_id = $workspace) > 0 { RETURN false; };
IF array::len(SELECT id FROM knowledge_memory_passages WHERE workspace_id = $workspace) > 0 { RETURN false; };
IF array::len(SELECT id FROM knowledge_passage_evidence WHERE passage_id IN (SELECT VALUE id FROM knowledge_memory_passages WHERE workspace_id = $workspace)) > 0 { RETURN false; };
IF array::len(SELECT id FROM knowledge_quick_switcher_recents WHERE workspace_id = $workspace) > 0 { RETURN false; };
IF array::len(SELECT id FROM knowledge_retrieval_traces WHERE workspace_id = $workspace) > 0 { RETURN false; };
IF array::len(SELECT id FROM knowledge_semantic_catalog_entries WHERE workspace_id = $workspace) > 0 { RETURN false; };
IF array::len(SELECT id FROM knowledge_wiki_projections WHERE workspace_id = $workspace) > 0 { RETURN false; };
IF array::len(SELECT id FROM loom_ai_suggestions WHERE workspace_id = $workspace) > 0 { RETURN false; };
IF array::len(SELECT id FROM loom_block_knowledge_bridge WHERE workspace_id = $workspace) > 0 { RETURN false; };
IF array::len(SELECT id FROM loom_canvas_boards WHERE workspace_id = $workspace) > 0 { RETURN false; };
IF array::len(SELECT id FROM loom_canvas_placements WHERE workspace_id = $workspace) > 0 { RETURN false; };
IF array::len(SELECT id FROM loom_canvas_visual_edges WHERE workspace_id = $workspace) > 0 { RETURN false; };
IF array::len(SELECT id FROM loom_collection_members WHERE collection_id IN (SELECT VALUE id FROM loom_collections WHERE workspace_id = $workspace)) > 0 { RETURN false; };
IF array::len(SELECT id FROM loom_collections WHERE workspace_id = $workspace) > 0 { RETURN false; };
IF array::len(SELECT id FROM loom_folder_members WHERE workspace_id = $workspace) > 0 { RETURN false; };
IF array::len(SELECT id FROM loom_folders WHERE workspace_id = $workspace) > 0 { RETURN false; };
IF array::len(SELECT id FROM loom_wiki_overlays WHERE workspace_id = $workspace) > 0 { RETURN false; };
IF array::len(SELECT id FROM media_asset_tiers WHERE workspace_id = $workspace) > 0 { RETURN false; };
IF array::len(SELECT id FROM stage_capture_artifacts WHERE workspace_id = $workspace) > 0 { RETURN false; };
IF array::len(SELECT id FROM documents WHERE workspace_id IN (SELECT VALUE id FROM workspaces WHERE id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };
IF array::len(SELECT id FROM blocks WHERE document_id IN (SELECT VALUE id FROM documents WHERE workspace_id = $workspace) AND (document_id IN (SELECT VALUE id FROM documents WHERE workspace_id = $workspace)) != true) > 0 { RETURN false; };
IF array::len(SELECT id FROM canvases WHERE workspace_id IN (SELECT VALUE id FROM workspaces WHERE id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };
IF array::len(SELECT id FROM canvas_nodes WHERE canvas_id IN (SELECT VALUE id FROM canvases WHERE workspace_id = $workspace) AND (canvas_id IN (SELECT VALUE id FROM canvases WHERE workspace_id = $workspace)) != true) > 0 { RETURN false; };
IF array::len(SELECT id FROM canvas_edges WHERE canvas_id IN (SELECT VALUE id FROM canvases WHERE workspace_id = $workspace) AND (canvas_id IN (SELECT VALUE id FROM canvases WHERE workspace_id = $workspace)) != true) > 0 { RETURN false; };
IF array::len(SELECT id FROM canvas_edges WHERE from_node_id IN (SELECT VALUE id FROM canvas_nodes WHERE canvas_id IN (SELECT VALUE id FROM canvases WHERE workspace_id = $workspace)) AND (canvas_id IN (SELECT VALUE id FROM canvases WHERE workspace_id = $workspace)) != true) > 0 { RETURN false; };
IF array::len(SELECT id FROM canvas_edges WHERE to_node_id IN (SELECT VALUE id FROM canvas_nodes WHERE canvas_id IN (SELECT VALUE id FROM canvases WHERE workspace_id = $workspace)) AND (canvas_id IN (SELECT VALUE id FROM canvases WHERE workspace_id = $workspace)) != true) > 0 { RETURN false; };
IF array::len(SELECT id FROM ai_bronze_records WHERE workspace_id IN (SELECT VALUE id FROM workspaces WHERE id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };
IF array::len(SELECT id FROM ai_silver_records WHERE workspace_id IN (SELECT VALUE id FROM workspaces WHERE id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };
IF array::len(SELECT id FROM ai_silver_records WHERE bronze_ref IN (SELECT VALUE id FROM ai_bronze_records WHERE workspace_id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };
IF array::len(SELECT id FROM assets WHERE workspace_id IN (SELECT VALUE id FROM workspaces WHERE id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };
IF array::len(SELECT id FROM loom_blocks WHERE workspace_id IN (SELECT VALUE id FROM workspaces WHERE id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };
IF array::len(SELECT id FROM loom_edges WHERE workspace_id IN (SELECT VALUE id FROM workspaces WHERE id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };
IF array::len(SELECT id FROM loom_edges WHERE source_block_id IN (SELECT VALUE id FROM loom_blocks WHERE workspace_id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };
IF array::len(SELECT id FROM loom_edges WHERE target_block_id IN (SELECT VALUE id FROM loom_blocks WHERE workspace_id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };
IF array::len(SELECT id FROM calendar_sources WHERE workspace_id IN (SELECT VALUE id FROM workspaces WHERE id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };
IF array::len(SELECT id FROM calendar_events WHERE workspace_id IN (SELECT VALUE id FROM workspaces WHERE id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };
IF array::len(SELECT id FROM calendar_events WHERE source_id IN (SELECT VALUE id FROM calendar_sources WHERE workspace_id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };
IF array::len(SELECT id FROM knowledge_source_roots WHERE workspace_id IN (SELECT VALUE id FROM workspaces WHERE id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };
IF array::len(SELECT id FROM knowledge_sources WHERE workspace_id IN (SELECT VALUE id FROM workspaces WHERE id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };
IF array::len(SELECT id FROM knowledge_sources WHERE root_id IN (SELECT VALUE id FROM knowledge_source_roots WHERE workspace_id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };
IF array::len(SELECT id FROM knowledge_sources WHERE asset_id IN (SELECT VALUE id FROM assets WHERE workspace_id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };
IF array::len(SELECT id FROM knowledge_sources WHERE loom_block_id IN (SELECT VALUE id FROM loom_blocks WHERE workspace_id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };
IF array::len(SELECT id FROM knowledge_sources WHERE document_id IN (SELECT VALUE id FROM documents WHERE workspace_id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };
IF array::len(SELECT id FROM knowledge_index_runs WHERE workspace_id IN (SELECT VALUE id FROM workspaces WHERE id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };
IF array::len(SELECT id FROM knowledge_spans WHERE source_id IN (SELECT VALUE id FROM knowledge_sources WHERE workspace_id = $workspace) AND (source_id IN (SELECT VALUE id FROM knowledge_sources WHERE workspace_id = $workspace)) != true) > 0 { RETURN false; };
IF array::len(SELECT id FROM knowledge_entities WHERE workspace_id IN (SELECT VALUE id FROM workspaces WHERE id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };
IF array::len(SELECT id FROM knowledge_entity_spans WHERE entity_id IN (SELECT VALUE id FROM knowledge_entities WHERE workspace_id = $workspace) AND (entity_id IN (SELECT VALUE id FROM knowledge_entities WHERE workspace_id = $workspace)) != true) > 0 { RETURN false; };
IF array::len(SELECT id FROM knowledge_edges WHERE workspace_id IN (SELECT VALUE id FROM workspaces WHERE id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };
IF array::len(SELECT id FROM knowledge_edges WHERE source_entity_id IN (SELECT VALUE id FROM knowledge_entities WHERE workspace_id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };
IF array::len(SELECT id FROM knowledge_edges WHERE target_entity_id IN (SELECT VALUE id FROM knowledge_entities WHERE workspace_id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };
IF array::len(SELECT id FROM knowledge_edge_spans WHERE edge_id IN (SELECT VALUE id FROM knowledge_edges WHERE workspace_id = $workspace) AND (edge_id IN (SELECT VALUE id FROM knowledge_edges WHERE workspace_id = $workspace)) != true) > 0 { RETURN false; };
IF array::len(SELECT id FROM knowledge_claims WHERE workspace_id IN (SELECT VALUE id FROM workspaces WHERE id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };
IF array::len(SELECT id FROM knowledge_claim_spans WHERE claim_id IN (SELECT VALUE id FROM knowledge_claims WHERE workspace_id = $workspace) AND (claim_id IN (SELECT VALUE id FROM knowledge_claims WHERE workspace_id = $workspace)) != true) > 0 { RETURN false; };
IF array::len(SELECT id FROM knowledge_claim_conflicts WHERE claim_id IN (SELECT VALUE id FROM knowledge_claims WHERE workspace_id = $workspace) AND (claim_id IN (SELECT VALUE id FROM knowledge_claims WHERE workspace_id = $workspace)) != true) > 0 { RETURN false; };
IF array::len(SELECT id FROM knowledge_claim_conflicts WHERE conflicting_claim_id IN (SELECT VALUE id FROM knowledge_claims WHERE workspace_id = $workspace) AND (claim_id IN (SELECT VALUE id FROM knowledge_claims WHERE workspace_id = $workspace)) != true) > 0 { RETURN false; };
IF array::len(SELECT id FROM knowledge_memory_passages WHERE workspace_id IN (SELECT VALUE id FROM workspaces WHERE id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };
IF array::len(SELECT id FROM knowledge_passage_evidence WHERE passage_id IN (SELECT VALUE id FROM knowledge_memory_passages WHERE workspace_id = $workspace) AND (passage_id IN (SELECT VALUE id FROM knowledge_memory_passages WHERE workspace_id = $workspace)) != true) > 0 { RETURN false; };
IF array::len(SELECT id FROM knowledge_passage_evidence WHERE source_id IN (SELECT VALUE id FROM knowledge_sources WHERE workspace_id = $workspace) AND (passage_id IN (SELECT VALUE id FROM knowledge_memory_passages WHERE workspace_id = $workspace)) != true) > 0 { RETURN false; };
IF array::len(SELECT id FROM knowledge_passage_evidence WHERE claim_id IN (SELECT VALUE id FROM knowledge_claims WHERE workspace_id = $workspace) AND (passage_id IN (SELECT VALUE id FROM knowledge_memory_passages WHERE workspace_id = $workspace)) != true) > 0 { RETURN false; };
IF array::len(SELECT id FROM knowledge_wiki_projections WHERE workspace_id IN (SELECT VALUE id FROM workspaces WHERE id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };
IF array::len(SELECT id FROM knowledge_rich_documents WHERE workspace_id IN (SELECT VALUE id FROM workspaces WHERE id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };
IF array::len(SELECT id FROM knowledge_rich_document_title_anchors WHERE workspace_id IN (SELECT VALUE id FROM workspaces WHERE id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };
IF array::len(SELECT id FROM knowledge_rich_document_versions WHERE rich_document_id IN (SELECT VALUE id FROM knowledge_rich_documents WHERE workspace_id = $workspace) AND (rich_document_id IN (SELECT VALUE id FROM knowledge_rich_documents WHERE workspace_id = $workspace)) != true) > 0 { RETURN false; };
IF array::len(SELECT id FROM knowledge_editor_code_nodes WHERE rich_document_id IN (SELECT VALUE id FROM knowledge_rich_documents WHERE workspace_id = $workspace) AND (rich_document_id IN (SELECT VALUE id FROM knowledge_rich_documents WHERE workspace_id = $workspace)) != true) > 0 { RETURN false; };
IF array::len(SELECT id FROM knowledge_context_bundles WHERE workspace_id IN (SELECT VALUE id FROM workspaces WHERE id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };
IF array::len(SELECT id FROM knowledge_context_bundle_items WHERE bundle_id IN (SELECT VALUE id FROM knowledge_context_bundles WHERE workspace_id = $workspace) AND (bundle_id IN (SELECT VALUE id FROM knowledge_context_bundles WHERE workspace_id = $workspace)) != true) > 0 { RETURN false; };
IF array::len(SELECT id FROM knowledge_retrieval_traces WHERE workspace_id IN (SELECT VALUE id FROM workspaces WHERE id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };
IF array::len(SELECT id FROM knowledge_idempotency_keys WHERE workspace_id IN (SELECT VALUE id FROM workspaces WHERE id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };
IF array::len(SELECT id FROM knowledge_ingestion_root_policies WHERE workspace_id IN (SELECT VALUE id FROM workspaces WHERE id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };
IF array::len(SELECT id FROM knowledge_ingestion_policy_decisions WHERE workspace_id IN (SELECT VALUE id FROM workspaces WHERE id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };
IF array::len(SELECT id FROM knowledge_ingestion_receipts WHERE workspace_id IN (SELECT VALUE id FROM workspaces WHERE id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };
IF array::len(SELECT id FROM knowledge_ingestion_receipts WHERE source_id IN (SELECT VALUE id FROM knowledge_sources WHERE workspace_id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };
IF array::len(SELECT id FROM knowledge_ingestion_spans WHERE workspace_id IN (SELECT VALUE id FROM workspaces WHERE id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };
IF array::len(SELECT id FROM knowledge_ingestion_spans WHERE source_id IN (SELECT VALUE id FROM knowledge_sources WHERE workspace_id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };
IF array::len(SELECT id FROM knowledge_ingestion_spans WHERE receipt_id IN (SELECT VALUE id FROM knowledge_ingestion_receipts WHERE workspace_id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };
IF array::len(SELECT id FROM knowledge_ingestion_repair_queue WHERE workspace_id IN (SELECT VALUE id FROM workspaces WHERE id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };
IF array::len(SELECT id FROM knowledge_ingestion_repair_queue WHERE source_id IN (SELECT VALUE id FROM knowledge_sources WHERE workspace_id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };
IF array::len(SELECT id FROM knowledge_code_files WHERE workspace_id IN (SELECT VALUE id FROM workspaces WHERE id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };
IF array::len(SELECT id FROM knowledge_code_files WHERE source_id IN (SELECT VALUE id FROM knowledge_sources WHERE workspace_id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };
IF array::len(SELECT id FROM knowledge_code_scip_imports WHERE workspace_id IN (SELECT VALUE id FROM workspaces WHERE id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };
IF array::len(SELECT id FROM knowledge_code_repair_queue WHERE workspace_id IN (SELECT VALUE id FROM workspaces WHERE id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };
IF array::len(SELECT id FROM knowledge_code_repair_queue WHERE source_id IN (SELECT VALUE id FROM knowledge_sources WHERE workspace_id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };
IF array::len(SELECT id FROM knowledge_memory_ontology_terms WHERE workspace_id IN (SELECT VALUE id FROM workspaces WHERE id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };
IF array::len(SELECT id FROM knowledge_memory_ontology_aliases WHERE term_id IN (SELECT VALUE id FROM knowledge_memory_ontology_terms WHERE workspace_id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };
IF array::len(SELECT id FROM knowledge_memory_ontology_aliases WHERE workspace_id IN (SELECT VALUE id FROM workspaces WHERE id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };
IF array::len(SELECT id FROM knowledge_memory_facts WHERE workspace_id IN (SELECT VALUE id FROM workspaces WHERE id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };
IF array::len(SELECT id FROM knowledge_memory_facts WHERE claim_id IN (SELECT VALUE id FROM knowledge_claims WHERE workspace_id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };
IF array::len(SELECT id FROM knowledge_memory_facts WHERE subject_entity_id IN (SELECT VALUE id FROM knowledge_entities WHERE workspace_id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };
IF array::len(SELECT id FROM knowledge_memory_facts WHERE object_entity_id IN (SELECT VALUE id FROM knowledge_entities WHERE workspace_id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };
IF array::len(SELECT id FROM knowledge_memory_conflict_detection_jobs WHERE workspace_id IN (SELECT VALUE id FROM workspaces WHERE id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };
IF array::len(SELECT id FROM knowledge_memory_conflict_detection_findings WHERE job_id IN (SELECT VALUE id FROM knowledge_memory_conflict_detection_jobs WHERE workspace_id = $workspace) AND (job_id IN (SELECT VALUE id FROM knowledge_memory_conflict_detection_jobs WHERE workspace_id = $workspace)) != true) > 0 { RETURN false; };
IF array::len(SELECT id FROM knowledge_memory_conflict_detection_findings WHERE conflict_id IN (SELECT VALUE id FROM knowledge_claim_conflicts WHERE claim_id IN (SELECT VALUE id FROM knowledge_claims WHERE workspace_id = $workspace)) AND (job_id IN (SELECT VALUE id FROM knowledge_memory_conflict_detection_jobs WHERE workspace_id = $workspace)) != true) > 0 { RETURN false; };
IF array::len(SELECT id FROM knowledge_memory_conflict_resolution_jobs WHERE workspace_id IN (SELECT VALUE id FROM workspaces WHERE id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };
IF array::len(SELECT id FROM knowledge_memory_conflict_resolution_jobs WHERE conflict_id IN (SELECT VALUE id FROM knowledge_claim_conflicts WHERE claim_id IN (SELECT VALUE id FROM knowledge_claims WHERE workspace_id = $workspace)) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };
IF array::len(SELECT id FROM knowledge_memory_bridge_decisions WHERE workspace_id IN (SELECT VALUE id FROM workspaces WHERE id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };
IF array::len(SELECT id FROM knowledge_memory_bridge_decisions WHERE entity_id_a IN (SELECT VALUE id FROM knowledge_entities WHERE workspace_id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };
IF array::len(SELECT id FROM knowledge_memory_bridge_decisions WHERE entity_id_b IN (SELECT VALUE id FROM knowledge_entities WHERE workspace_id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };
IF array::len(SELECT id FROM knowledge_semantic_catalog_entries WHERE workspace_id IN (SELECT VALUE id FROM workspaces WHERE id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };
IF array::len(SELECT id FROM knowledge_document_embeds WHERE rich_document_id IN (SELECT VALUE id FROM knowledge_rich_documents WHERE workspace_id = $workspace) AND (rich_document_id IN (SELECT VALUE id FROM knowledge_rich_documents WHERE workspace_id = $workspace)) != true) > 0 { RETURN false; };
IF array::len(SELECT id FROM knowledge_document_backlinks WHERE workspace_id IN (SELECT VALUE id FROM workspaces WHERE id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };
IF array::len(SELECT id FROM knowledge_document_backlinks WHERE source_document_id IN (SELECT VALUE id FROM knowledge_rich_documents WHERE workspace_id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };
IF array::len(SELECT id FROM loom_block_knowledge_bridge WHERE block_id IN (SELECT VALUE id FROM loom_blocks WHERE workspace_id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };
IF array::len(SELECT id FROM loom_block_knowledge_bridge WHERE workspace_id IN (SELECT VALUE id FROM workspaces WHERE id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };
IF array::len(SELECT id FROM loom_block_knowledge_bridge WHERE entity_id IN (SELECT VALUE id FROM knowledge_entities WHERE workspace_id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };
IF array::len(SELECT id FROM loom_folders WHERE workspace_id IN (SELECT VALUE id FROM workspaces WHERE id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };
IF array::len(SELECT id FROM loom_folders WHERE parent_folder_id IN (SELECT VALUE id FROM loom_folders WHERE workspace_id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };
IF array::len(SELECT id FROM loom_folder_members WHERE folder_id IN (SELECT VALUE id FROM loom_folders WHERE workspace_id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };
IF array::len(SELECT id FROM loom_folder_members WHERE block_id IN (SELECT VALUE id FROM loom_blocks WHERE workspace_id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };
IF array::len(SELECT id FROM loom_folder_members WHERE workspace_id IN (SELECT VALUE id FROM workspaces WHERE id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };
IF array::len(SELECT id FROM loom_wiki_overlays WHERE workspace_id IN (SELECT VALUE id FROM workspaces WHERE id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };
IF array::len(SELECT id FROM knowledge_quick_switcher_recents WHERE workspace_id IN (SELECT VALUE id FROM workspaces WHERE id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };
IF array::len(SELECT id FROM knowledge_workbench_layout_states WHERE workspace_id IN (SELECT VALUE id FROM workspaces WHERE id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };
IF array::len(SELECT id FROM knowledge_workspace_settings_states WHERE workspace_id IN (SELECT VALUE id FROM workspaces WHERE id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };
IF array::len(SELECT id FROM knowledge_rich_document_drafts WHERE rich_document_id IN (SELECT VALUE id FROM knowledge_rich_documents WHERE workspace_id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };
IF array::len(SELECT id FROM knowledge_rich_document_drafts WHERE workspace_id IN (SELECT VALUE id FROM workspaces WHERE id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };
IF array::len(SELECT id FROM knowledge_workspace_search_bookmark_states WHERE workspace_id IN (SELECT VALUE id FROM workspaces WHERE id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };
IF array::len(SELECT id FROM knowledge_debug_breakpoints WHERE rich_document_id IN (SELECT VALUE id FROM knowledge_rich_documents WHERE workspace_id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };
IF array::len(SELECT id FROM knowledge_debug_breakpoints WHERE workspace_id IN (SELECT VALUE id FROM workspaces WHERE id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };
IF array::len(SELECT id FROM media_asset_tiers WHERE workspace_id IN (SELECT VALUE id FROM workspaces WHERE id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };
IF array::len(SELECT id FROM media_asset_tiers WHERE asset_id IN (SELECT VALUE id FROM assets WHERE workspace_id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };
IF array::len(SELECT id FROM loom_collections WHERE workspace_id IN (SELECT VALUE id FROM workspaces WHERE id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };
IF array::len(SELECT id FROM loom_collection_members WHERE collection_id IN (SELECT VALUE id FROM loom_collections WHERE workspace_id = $workspace) AND (collection_id IN (SELECT VALUE id FROM loom_collections WHERE workspace_id = $workspace)) != true) > 0 { RETURN false; };
IF array::len(SELECT id FROM loom_collection_members WHERE asset_id IN (SELECT VALUE id FROM assets WHERE workspace_id = $workspace) AND (collection_id IN (SELECT VALUE id FROM loom_collections WHERE workspace_id = $workspace)) != true) > 0 { RETURN false; };
IF array::len(SELECT id FROM loom_ai_suggestions WHERE workspace_id IN (SELECT VALUE id FROM workspaces WHERE id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };
IF array::len(SELECT id FROM loom_canvas_boards WHERE block_id IN (SELECT VALUE id FROM loom_blocks WHERE workspace_id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };
IF array::len(SELECT id FROM loom_canvas_boards WHERE workspace_id IN (SELECT VALUE id FROM workspaces WHERE id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };
IF array::len(SELECT id FROM loom_canvas_placements WHERE canvas_block_id IN (SELECT VALUE id FROM loom_canvas_boards WHERE workspace_id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };
IF array::len(SELECT id FROM loom_canvas_placements WHERE workspace_id IN (SELECT VALUE id FROM workspaces WHERE id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };
IF array::len(SELECT id FROM loom_canvas_visual_edges WHERE canvas_block_id IN (SELECT VALUE id FROM loom_canvas_boards WHERE workspace_id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };
IF array::len(SELECT id FROM loom_canvas_visual_edges WHERE workspace_id IN (SELECT VALUE id FROM workspaces WHERE id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };
IF array::len(SELECT id FROM loom_canvas_visual_edges WHERE from_placement_id IN (SELECT VALUE id FROM loom_canvas_placements WHERE workspace_id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };
IF array::len(SELECT id FROM loom_canvas_visual_edges WHERE to_placement_id IN (SELECT VALUE id FROM loom_canvas_placements WHERE workspace_id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };
IF array::len(SELECT id FROM loom_block_search_index WHERE block_id IN (SELECT VALUE id FROM loom_blocks WHERE workspace_id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };
IF array::len(SELECT id FROM loom_block_search_index WHERE workspace_id IN (SELECT VALUE id FROM workspaces WHERE id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };
IF array::len(SELECT id FROM calendar_activity_spans WHERE workspace_id IN (SELECT VALUE id FROM workspaces WHERE id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };
IF array::len(SELECT id FROM stage_capture_artifacts WHERE workspace_id IN (SELECT VALUE id FROM workspaces WHERE id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };
IF array::len(SELECT id FROM fems_memory_packs WHERE workspace_id IN (SELECT VALUE id FROM workspaces WHERE id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };
IF array::len(SELECT id FROM fems_memory_proposals WHERE workspace_id IN (SELECT VALUE id FROM workspaces WHERE id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };
IF array::len(SELECT id FROM fems_memory_items WHERE workspace_id IN (SELECT VALUE id FROM workspaces WHERE id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };
IF array::len(SELECT id FROM fems_memory_commit_reports WHERE workspace_id IN (SELECT VALUE id FROM workspaces WHERE id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };
IF array::len(SELECT id FROM fems_memory_commit_reports WHERE proposal_id IN (SELECT VALUE id FROM fems_memory_proposals WHERE workspace_id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };
IF array::len(SELECT id FROM fems_memory_commit_fr_outbox WHERE workspace_id IN (SELECT VALUE id FROM workspaces WHERE id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };
IF array::len(SELECT id FROM fems_memory_commit_fr_outbox WHERE proposal_id IN (SELECT VALUE id FROM fems_memory_proposals WHERE workspace_id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };
IF array::len(SELECT id FROM fems_memory_commit_fr_outbox WHERE commit_id IN (SELECT VALUE id FROM fems_memory_commit_reports WHERE workspace_id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };
IF array::len(SELECT id FROM fems_memory_lifecycle_fr_outbox WHERE workspace_id IN (SELECT VALUE id FROM workspaces WHERE id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };
IF array::len(SELECT id FROM fems_memory_lifecycle_fr_outbox WHERE proposal_id IN (SELECT VALUE id FROM fems_memory_proposals WHERE workspace_id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };
IF array::len(SELECT id FROM loom_block_view_fr_outbox WHERE workspace_id IN (SELECT VALUE id FROM workspaces WHERE id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };
FOR $source IN (SELECT id FROM knowledge_sources WHERE workspace_id = $workspace) { IF array::len(SELECT id FROM protected_resources WHERE id IN $resources.id AND resource_kind = 'knowledge_source' AND external_resource_id = record::id($source.id) AND lifecycle_state = 'active') != 1 { RETURN false; }; };
FOR $source IN (SELECT id FROM knowledge_code_files WHERE workspace_id = $workspace) { IF array::len(SELECT id FROM protected_resources WHERE id IN $resources.id AND resource_kind = 'knowledge_code_file' AND external_resource_id = record::id($source.id) AND lifecycle_state = 'active') != 1 { RETURN false; }; };
FOR $source IN (SELECT id FROM fems_memory_packs WHERE workspace_id = $workspace) { IF array::len(SELECT id FROM protected_resources WHERE id IN $resources.id AND resource_kind = 'memory_pack' AND external_resource_id = $external AND lifecycle_state = 'active') != 1 { RETURN false; }; };
FOR $source IN (SELECT id FROM fems_memory_proposals WHERE workspace_id = $workspace) { IF array::len(SELECT id FROM protected_resources WHERE id IN $resources.id AND resource_kind = 'memory_proposal' AND external_resource_id = $external AND lifecycle_state = 'active') != 1 { RETURN false; }; };
FOR $source IN (SELECT id FROM fems_memory_items WHERE workspace_id = $workspace) { IF array::len(SELECT id FROM protected_resources WHERE id IN $resources.id AND resource_kind = 'memory_item' AND external_resource_id = $external AND lifecycle_state = 'active') != 1 { RETURN false; }; };
FOR $source IN (SELECT id FROM fems_memory_commit_reports WHERE workspace_id = $workspace) { IF array::len(SELECT id FROM protected_resources WHERE id IN $resources.id AND resource_kind = 'memory_commit_report' AND external_resource_id = $external AND lifecycle_state = 'active') != 1 { RETURN false; }; };
    FOR $row IN (SELECT created_in_session_id FROM knowledge_source_roots WHERE workspace_id = $workspace) { IF $row.created_in_session_id = NONE OR $row.created_in_session_id.account_id != $account OR $row.created_in_session_id.access_space_id != $space { RETURN false; }; };
    FOR $row IN (SELECT created_in_session_id FROM knowledge_index_runs WHERE workspace_id = $workspace) { IF $row.created_in_session_id = NONE OR $row.created_in_session_id.account_id != $account OR $row.created_in_session_id.access_space_id != $space { RETURN false; }; };
    FOR $run IN (SELECT scope FROM knowledge_index_runs WHERE workspace_id = $workspace) {
        IF $run.scope.source_ids = NONE { RETURN false; };
        FOR $key IN $run.scope.source_ids {
            IF type::record('knowledge_sources', $key).workspace_id != $workspace OR !fn::mt109_has_grant('knowledge_source', $key, 'delete', 'fs.write') { RETURN false; };
        };
    };
    FOR $entity IN (SELECT * FROM knowledge_entities WHERE workspace_id = $workspace) {
        IF $entity.entity_kind = 'loom_block' AND $entity.primary_source_id = NONE {
            IF array::len(SELECT id FROM protected_resources WHERE id IN $resources.id AND resource_kind IN ['loom_block','rich_document'] AND external_resource_id = $entity.entity_key AND lifecycle_state = 'active') != 1 { RETURN false; };
        } ELSE IF $entity.primary_source_id = NONE OR $entity.primary_source_id.workspace_id != $workspace OR !fn::mt109_has_grant('knowledge_source', record::id($entity.primary_source_id), 'delete', 'fs.write') { RETURN false; };
        FOR $link IN (SELECT span_id FROM knowledge_entity_spans WHERE entity_id = $entity.id) {
            IF $link.span_id.source_id.workspace_id != $workspace OR !fn::mt109_has_grant('knowledge_source', record::id($link.span_id.source_id), 'delete', 'fs.write') { RETURN false; };
        };
    };
    FOR $edge IN (SELECT * FROM knowledge_edges WHERE workspace_id = $workspace) {
        IF $edge.source_entity_id.workspace_id != $workspace OR $edge.target_entity_id.workspace_id != $workspace { RETURN false; };
        FOR $link IN (SELECT span_id FROM knowledge_edge_spans WHERE edge_id = $edge.id) {
            IF $link.span_id.source_id.workspace_id != $workspace OR !fn::mt109_has_grant('knowledge_source', record::id($link.span_id.source_id), 'delete', 'fs.write') { RETURN false; };
        };
    };
    IF array::len(SELECT id FROM knowledge_ingestion_receipts WHERE workspace_id = $workspace AND (source_id.workspace_id = $workspace) != true) > 0
        OR array::len(SELECT id FROM knowledge_ingestion_spans WHERE workspace_id = $workspace AND (source_id.workspace_id = $workspace AND receipt_id.workspace_id = $workspace AND receipt_id.source_id = source_id) != true) > 0
        OR array::len(SELECT id FROM knowledge_ingestion_repair_queue WHERE workspace_id = $workspace AND (source_id.workspace_id = $workspace) != true) > 0
        OR array::len(SELECT id FROM knowledge_code_repair_queue WHERE workspace_id = $workspace AND (source_id.workspace_id = $workspace) != true) > 0 { RETURN false; };
    RETURN true;
};
DEFINE FUNCTION OVERWRITE fn::mt120_document_delete($document: string, $workspace: string) {
    IF !fn::mt120_document_access($document, $workspace, 'delete', 'fs.write') { RETURN false; };
    LET $doc = type::record('knowledge_rich_documents', $document);
    LET $block = type::record('loom_blocks', $document);
    LET $ws = type::record('workspaces', $workspace);
    IF $doc.workspace_id != $ws { RETURN false; };
    IF array::len(SELECT id FROM knowledge_sources WHERE loom_block_id = $block) > 0
        OR array::len(SELECT id FROM loom_block_knowledge_bridge WHERE block_id = $block) > 0
        OR array::len(SELECT id FROM loom_folder_members WHERE block_id = $block) > 0
        OR array::len(SELECT id FROM loom_canvas_boards WHERE block_id = $block) > 0
        OR array::len(SELECT id FROM loom_canvas_placements WHERE placed_block_id = $block) > 0 { RETURN false; };
    IF array::len(SELECT id FROM loom_block_search_index WHERE block_id = $block AND (workspace_id = $ws) != true) > 0
        OR array::len(SELECT id FROM knowledge_rich_document_drafts WHERE rich_document_id = $doc AND (workspace_id = $ws) != true) > 0 { RETURN false; };
    FOR $edge IN (SELECT * FROM loom_edges WHERE source_block_id = $block OR target_block_id = $block) {
        IF $edge.workspace_id != $ws OR $edge.source_block_id.workspace_id != $ws OR $edge.target_block_id.workspace_id != $ws
            OR $edge.source_block_id.source_rich_document_id = NONE OR $edge.target_block_id.source_rich_document_id = NONE
            OR !fn::mt120_document_access(record::id($edge.source_block_id.source_rich_document_id), $workspace, 'update', 'fs.write')
            OR !fn::mt120_document_access(record::id($edge.target_block_id.source_rich_document_id), $workspace, 'update', 'fs.write') { RETURN false; };
    };
    FOR $backlink IN (SELECT * FROM knowledge_document_backlinks WHERE source_document_id = $doc OR target = $document OR target = $doc.title) {
        IF $backlink.workspace_id != $ws OR $backlink.source_document_id.workspace_id != $ws
            OR !fn::mt120_document_access(record::id($backlink.source_document_id), $workspace, 'update', 'fs.write') { RETURN false; };
    };
    FOR $source IN (SELECT * FROM knowledge_sources WHERE workspace_id = $ws AND source_kind = 'rich_document' AND provenance.rich_document_id = $document) {
        IF !fn::mt109_has_grant('knowledge_source', record::id($source.id), 'update', 'fs.write') { RETURN false; };
    };
    RETURN true;
};
DEFINE FUNCTION OVERWRITE fn::mt120_index_source_read($source: option<record<knowledge_sources>>, $workspace: record<workspaces>) {
    RETURN $source != NONE AND $source.workspace_id = $workspace AND $source.source_kind = 'file'
        AND fn::mt109_source_read('knowledge_source', record::id($source), record::id($workspace), 'workspace', record::id($workspace));
};
DEFINE FUNCTION OVERWRITE fn::mt120_entity_read($entity: record<knowledge_entities>) {
    IF !record::exists($entity) OR !fn::mt120_index_source_read($entity.primary_source_id, $entity.workspace_id) { RETURN false; };
    LET $evidence_rows = (SELECT span_id FROM knowledge_entity_spans WHERE entity_id = $entity);
    FOR $evidence IN $evidence_rows {
        IF !fn::mt120_index_source_read($evidence.span_id.source_id, $entity.workspace_id) { RETURN false; };
    };
    RETURN true;
};
DEFINE FUNCTION OVERWRITE fn::mt120_edge_read($edge: record<knowledge_edges>) {
    IF !record::exists($edge) OR $edge.source_entity_id.workspace_id != $edge.workspace_id
        OR $edge.target_entity_id.workspace_id != $edge.workspace_id
        OR !fn::mt120_entity_read($edge.source_entity_id) OR !fn::mt120_entity_read($edge.target_entity_id) { RETURN false; };
    LET $evidence_rows = (SELECT span_id FROM knowledge_edge_spans WHERE edge_id = $edge);
    FOR $evidence IN $evidence_rows {
        IF !fn::mt120_index_source_read($evidence.span_id.source_id, $edge.workspace_id) { RETURN false; };
    };
    RETURN true;
};
DEFINE FUNCTION OVERWRITE fn::mt120_nav_receipt($resource: option<record<protected_resources>>, $session: option<record<authenticated_sessions>>, $capability: option<string>, $action: option<string>, $wsids: array<string>, $event: string, $source: string, $aggregate: string, $query_kind: string, $payload: object, $actor_kind: string, $actor_id: string, $session_run: string, $write: bool) {
    IF !fn::mt109_live_session() OR $resource = NONE OR $session = NONE OR array::len($wsids) != 1
        OR $resource.resource_kind != 'workspace' OR $resource.external_resource_id != $wsids[0]
        OR $resource.owner_account_id != $auth.account_id OR $resource.access_space_id != $auth.access_space_id
        OR $event != 'KNOWLEDGE_RETRIEVAL_TRACE_RECORDED' OR $source != 'knowledge_code_nav' OR $aggregate != 'knowledge_code_nav'
        OR $query_kind NOT IN ['symbol_lookup','symbol_get','symbol_references','symbol_tests','symbol_spans','file_lens']
        OR $capability != 'memory.read' OR $action != 'read' OR $payload.kind != 'code_nav_query'
        OR $payload.query_kind != $query_kind OR $payload.workspace_id != $wsids[0]
        OR $payload.minted_by_principal != record::id($session.principal_id)
        OR $payload.account_id != record::id($session.account_id) OR $payload.access_space_id != record::id($session.access_space_id)
        OR $payload.delegation_chain != $session.delegation_chain OR $session.account_id != $auth.account_id
        OR $session.access_space_id != $auth.access_space_id OR $session_run != record::id($session)
        OR $actor_kind != $session.principal_id.actor_kind OR $actor_id != $session.principal_id.actor_id
        OR !fn::mt109_has_workspace_access($wsids[0], 'read', 'memory.read') { RETURN false; };
    IF $write AND ($session != $auth.id OR !fn::mt109_ledger_access($resource, $session, $capability, $action)) { RETURN false; };
    LET $witness = $payload.read_witnesses;
    IF $witness = NONE OR $witness.entity_ids = NONE OR $witness.edge_ids = NONE OR $witness.source_ids = NONE OR $witness.span_ids = NONE { RETURN false; };
    LET $workspace = type::record('workspaces', $wsids[0]);
    FOR $key IN $witness.entity_ids {
        LET $entity = type::record('knowledge_entities', $key);
        IF $entity.workspace_id != $workspace OR !fn::mt120_entity_read($entity) { RETURN false; };
    };
    FOR $key IN $witness.edge_ids {
        LET $edge = type::record('knowledge_edges', $key);
        IF $edge.workspace_id != $workspace OR !fn::mt120_edge_read($edge) { RETURN false; };
    };
    FOR $key IN $witness.source_ids {
        IF !fn::mt120_index_source_read(type::record('knowledge_sources', $key), $workspace) { RETURN false; };
    };
    FOR $key IN $witness.span_ids {
        LET $span = type::record('knowledge_spans', $key);
        IF !record::exists($span) OR !fn::mt120_index_source_read($span.source_id, $workspace) { RETURN false; };
    };
    IF $query_kind IN ['symbol_get','symbol_references','symbol_tests','symbol_spans']
        AND !($witness.entity_ids CONTAINS $payload.query.entity_id) { RETURN false; };
    IF $query_kind = 'symbol_lookup' AND $payload.query.matches > 0 AND array::len($witness.entity_ids) = 0 { RETURN false; };
    IF $query_kind = 'file_lens' AND $payload.query.entries > 0 AND (array::len($witness.entity_ids) = 0 OR array::len($witness.source_ids) = 0) { RETURN false; };
    IF $query_kind = 'symbol_spans' AND $payload.query.spans > 0 AND array::len($witness.span_ids) = 0 { RETURN false; };
    IF (($query_kind = 'symbol_tests' AND $payload.query.tests > 0) OR ($query_kind = 'symbol_references' AND ($payload.query.callers > 0 OR $payload.query.callees > 0))) AND array::len($witness.edge_ids) = 0 { RETURN false; };
    RETURN true;
};
DEFINE FUNCTION OVERWRITE fn::mt120_nav_quiet_receipt($resource: option<record<protected_resources>>, $session: option<record<authenticated_sessions>>, $capability: option<string>, $action: option<string>, $wsids: array<string>, $event: string, $source: string, $aggregate: string, $receipt: string, $payload: object, $actor_kind: string, $actor_id: string, $session_run: string, $write: bool) {
    IF !fn::mt109_live_session() OR $event != 'KNOWLEDGE_QUIET_BACKGROUND_WORK_RECORDED'
        OR $source != 'parallel_swarm_state_recovery' OR $aggregate != 'parallel_swarm_quiet_background_work'
        OR $payload.schema_id != 'hsk.parallel_swarm.quiet_background_work@1' OR $payload.work_kind != 'backend_navigation'
        OR $payload.receipt_id != $receipt OR $payload.evidence_ref != 'event://' + $payload.subject_id
        OR $session = NONE OR $resource = NONE OR $capability != 'memory.read' OR $action != 'read'
        OR array::len($wsids) != 1 OR $payload.workspace_id != $wsids[0]
        OR $resource.resource_kind != 'workspace' OR $resource.external_resource_id != $wsids[0]
        OR $resource.owner_account_id != $auth.account_id OR $resource.access_space_id != $auth.access_space_id
        OR $payload.minted_by_principal != record::id($session.principal_id)
        OR $payload.account_id != record::id($session.account_id) OR $payload.access_space_id != record::id($session.access_space_id)
        OR $payload.delegation_chain != $session.delegation_chain OR $session.account_id != $auth.account_id
        OR $session.access_space_id != $auth.access_space_id OR $session_run != record::id($session)
        OR $actor_kind != $session.principal_id.actor_kind OR $actor_id != $session.principal_id.actor_id { RETURN false; };
    IF $write AND ($session != $auth.id OR !fn::mt109_ledger_access($resource, $session, $capability, $action)) { RETURN false; };
    LET $nav = type::record('kernel_event_ledger', $payload.subject_id);
    RETURN record::exists($nav) AND $nav.wsids = $wsids AND $nav.authority_session_id = $session
        AND fn::mt120_nav_receipt($nav.authority_resource_id, $nav.authority_session_id, $nav.authority_capability_id, $nav.authority_action, $nav.wsids, $nav.event_type, $nav.source_component, $nav.aggregate_type, $nav.aggregate_id, $nav.payload, $nav.actor_kind, $nav.actor_id, $nav.session_run_id, false);
};
DEFINE FUNCTION OVERWRITE fn::mt120_nav_quiet_row($row: object, $write: bool) {
    LET $event = $row.event_ledger_event_id;
    RETURN $row.work_kind = 'backend_navigation' AND record::exists($event)
        AND $row.receipt_id = $event.aggregate_id AND $row.workspace_id = $event.payload.workspace_id
        AND $row.subject_id = $event.payload.subject_id AND $row.evidence_ref = $event.payload.evidence_ref
        AND $row.session_id = $event.session_run_id AND $row.actor_id = $event.actor_id
        AND $row.quiet_policy_jsonb = $event.payload.quiet_policy
        AND $row.wp_id = $event.payload.wp_id AND $row.mt_id = $event.payload.mt_id
        AND fn::mt120_nav_quiet_receipt($event.authority_resource_id, $event.authority_session_id, $event.authority_capability_id, $event.authority_action, $event.wsids, $event.event_type, $event.source_component, $event.aggregate_type, $event.aggregate_id, $event.payload, $event.actor_kind, $event.actor_id, $event.session_run_id, $write);
};
DEFINE FIELD OVERWRITE created_in_session_id ON TABLE knowledge_source_roots TYPE option<record<authenticated_sessions>> PERMISSIONS FOR select, create FULL FOR update NONE;
DEFINE FIELD OVERWRITE created_in_session_id ON TABLE knowledge_index_runs TYPE option<record<authenticated_sessions>> PERMISSIONS FOR select, create FULL FOR update NONE;
DEFINE FUNCTION OVERWRITE fn::mt120_index_source_write($source: option<record<knowledge_sources>>, $workspace: record<workspaces>) {
    RETURN $source != NONE AND $source.workspace_id = $workspace AND $source.source_kind = 'file'
        AND fn::mt109_has_grant('knowledge_source', record::id($source), 'update', 'memory.propose')
        AND fn::mt109_has_workspace_access(record::id($workspace), 'read', 'memory.read');
};
DEFINE FUNCTION OVERWRITE fn::mt120_entity_write($entity: record<knowledge_entities>, $source: option<record<knowledge_sources>>, $workspace: record<workspaces>) {
    IF !fn::mt120_index_source_write($source, $workspace) { RETURN false; };
    FOR $evidence IN (SELECT span_id FROM knowledge_entity_spans WHERE entity_id = $entity) {
        IF !fn::mt120_index_source_write($evidence.span_id.source_id, $workspace) { RETURN false; };
    };
    RETURN true;
};
DEFINE FUNCTION OVERWRITE fn::mt120_edge_write($edge: record<knowledge_edges>, $source: record<knowledge_entities>, $target: record<knowledge_entities>, $workspace: record<workspaces>) {
    IF $source.workspace_id != $workspace OR $target.workspace_id != $workspace
        OR !fn::mt120_entity_write($source, $source.primary_source_id, $workspace)
        OR !fn::mt120_entity_write($target, $target.primary_source_id, $workspace) { RETURN false; };
    FOR $evidence IN (SELECT span_id FROM knowledge_edge_spans WHERE edge_id = $edge) {
        IF !fn::mt120_index_source_write($evidence.span_id.source_id, $workspace) { RETURN false; };
    };
    RETURN true;
};
DEFINE FUNCTION OVERWRITE fn::mt120_index_run_access($row: object, $write: bool) {
    IF $row.created_in_session_id = NONE OR $row.created_in_session_id.account_id != $auth.account_id
        OR $row.created_in_session_id.access_space_id != $auth.access_space_id OR $row.scope.source_ids = NONE
        OR !fn::mt109_has_workspace_access(record::id($row.workspace_id), 'read', 'memory.read') { RETURN false; };
    IF $write AND !fn::mt109_has_workspace_access(record::id($row.workspace_id), 'update', 'memory.propose') { RETURN false; };
    FOR $key IN $row.scope.source_ids {
        LET $source = type::record('knowledge_sources', $key);
        IF !fn::mt120_index_source_read($source, $row.workspace_id) OR ($write AND !fn::mt120_index_source_write($source, $row.workspace_id)) { RETURN false; };
    };
    IF ($row.sources_seen > 0 OR $row.sources_indexed > 0 OR $row.spans_extracted > 0 OR $row.entities_detected > 0 OR $row.edges_written > 0) AND array::len($row.scope.source_ids) = 0 { RETURN false; };
    RETURN true;
};
DEFINE FUNCTION OVERWRITE fn::mt120_index_receipt($resource: option<record<protected_resources>>, $session: option<record<authenticated_sessions>>, $capability: option<string>, $action: option<string>, $wsids: array<string>, $event: string, $source: string, $aggregate: string, $aggregate_id: string, $payload: object, $actor_kind: string, $actor_id: string, $session_run: string, $write: bool) {
    IF !fn::mt109_live_session() OR $resource = NONE OR $session = NONE OR array::len($wsids) != 1
        OR $resource.resource_kind != 'workspace' OR $resource.external_resource_id != $wsids[0]
        OR $resource.owner_account_id != $auth.account_id OR $resource.access_space_id != $auth.access_space_id
        OR $capability != 'memory.propose' OR $action != 'create' OR $payload.workspace_id != $wsids[0]
        OR $payload.minted_by_principal != record::id($session.principal_id)
        OR $payload.account_id != record::id($session.account_id) OR $payload.access_space_id != record::id($session.access_space_id)
        OR $payload.delegation_chain != $session.delegation_chain OR $session.account_id != $auth.account_id
        OR $session.access_space_id != $auth.access_space_id OR $session_run != record::id($session)
        OR $actor_kind != $session.principal_id.actor_kind OR $actor_id != $session.principal_id.actor_id
        OR !fn::mt109_has_workspace_access($wsids[0], 'read', 'memory.read') { RETURN false; };
    IF $write AND ($session != $auth.id OR !fn::mt109_ledger_access($resource, $session, $capability, $action)) { RETURN false; };
    LET $workspace = type::record('workspaces', $wsids[0]);
    LET $kind = $payload.kind;
    IF $source = 'knowledge_ingestion' AND $event = 'VALIDATION_RECORDED' {
        IF $aggregate = 'knowledge_root_registration' AND $kind = 'root_registration_policy_decision' {
            RETURN fn::mt109_has_workspace_access($wsids[0], 'create', 'memory.propose');
        };
        IF $aggregate = 'knowledge_ingestion_batch' AND $kind = 'extraction_receipt_batch' AND $payload.run_token = $aggregate_id AND $payload.receipts != NONE {
            FOR $row IN $payload.receipts {
                LET $src = type::record('knowledge_sources', $row.source_id);
                IF $row.workspace_id != $wsids[0] OR !fn::mt120_index_source_read($src, $workspace)
                    OR ($write AND !fn::mt120_index_source_write($src, $workspace)) { RETURN false; };
            };
            RETURN true;
        };
        IF (($aggregate = 'knowledge_ingestion_receipt' AND $kind = 'extraction_receipt') OR ($aggregate = 'knowledge_source_lifecycle' AND $kind = 'source_stale_marked')) AND $payload.source_id = $aggregate_id {
            LET $src = type::record('knowledge_sources', $payload.source_id);
            RETURN fn::mt120_index_source_read($src, $workspace) AND (!$write OR fn::mt120_index_source_write($src, $workspace));
        };
        IF $aggregate != 'knowledge_ingestion_run' OR $kind NOT IN ['ingestion_run_started','ingestion_run_finished','ingestion_run_failed'] OR $payload.run_token != $aggregate_id { RETURN false; };
    } ELSE IF $source = 'knowledge_code_index' {
        IF $event = 'KNOWLEDGE_VALIDATION_RECORDED' AND $aggregate = 'knowledge_code_index_run' AND $kind = 'code_files_indexed_batch' AND $payload.index_run_id = $aggregate_id AND $payload.files != NONE {
            FOR $row IN $payload.files {
                LET $src = type::record('knowledge_sources', $row.source_id);
                IF $row.workspace_id != $wsids[0] OR !fn::mt120_index_source_read($src, $workspace)
                    OR ($write AND !fn::mt120_index_source_write($src, $workspace)) { RETURN false; };
            };
            RETURN array::len($payload.files) = $payload.file_count;
        };
        IF $event = 'KNOWLEDGE_VALIDATION_RECORDED' AND (($aggregate = 'knowledge_code_index_file' AND $kind IN ['code_file_indexed','code_file_parse_failed']) OR ($aggregate = 'knowledge_code_index_config' AND $kind = 'config_file_indexed')) AND $payload.source_id = $aggregate_id {
            LET $src = type::record('knowledge_sources', $payload.source_id);
            RETURN fn::mt120_index_source_read($src, $workspace) AND (!$write OR fn::mt120_index_source_write($src, $workspace));
        };
        IF $aggregate != 'knowledge_code_index_run' OR !(($event = 'KNOWLEDGE_INDEX_RUN_STARTED' AND $kind = 'code_index_run_started' AND $aggregate_id = $wsids[0]) OR ($event = 'KNOWLEDGE_INDEX_RUN_COMPLETED' AND $kind = 'code_index_run_completed' AND $aggregate_id = $payload.index_run_id) OR ($event = 'KNOWLEDGE_INDEX_RUN_FAILED' AND $kind = 'code_index_run_failed' AND $aggregate_id = $payload.index_run_id) OR ($event = 'KNOWLEDGE_INDEX_RUN_CANCELLED' AND $kind = 'code_index_run_cancelled' AND $aggregate_id = $payload.index_run_id)) { RETURN false; };
    } ELSE { RETURN false; };
    IF $payload.source_ids = NONE { RETURN false; };
    FOR $key IN $payload.source_ids {
        LET $src = type::record('knowledge_sources', $key);
        IF !fn::mt120_index_source_read($src, $workspace) OR ($write AND !fn::mt120_index_source_write($src, $workspace)) { RETURN false; };
    };
    IF $payload.files_ingested > 0 AND array::len($payload.source_ids) = 0 { RETURN false; };
    RETURN true;
};
-- RECORD_USER_PRODUCER_END
"#;
fn authority_nonce_stage_correction_statements() -> String {
    let mut statements = String::new();
    for (table, _) in AUTHORITY_NONCE_IMMUTABLE_FIELDS {
        let declaration = format!("DEFINE TABLE OVERWRITE {table} ");
        let start = SCHEMA
            .find(&declaration)
            .expect("authority table declaration must exist");
        let end = SCHEMA[start..]
            .find("\nDEFINE FIELD OVERWRITE ")
            .map(|offset| start + offset)
            .expect("authority table declaration must precede fields");
        statements.push_str(&SCHEMA[start..end]);
        statements.push('\n');
    }
    statements.push_str(AUTHORITY_NONCE_EVENT_STATEMENTS);
    statements
}

fn schema_table_definition_bounds(source: &str, table: &str) -> (usize, usize) {
    let declaration = format!("DEFINE TABLE OVERWRITE {table} ");
    let start = source
        .find(&declaration)
        .expect("stage-correct table declaration must exist");
    let end = source[start..]
        .find(";\n")
        .map(|offset| start + offset + 1)
        .expect("stage-correct table declaration must terminate");
    (start, end)
}

fn schema_event_definition_bounds(source: &str, event: &str) -> (usize, usize) {
    let declaration = format!("DEFINE EVENT OVERWRITE {event} ");
    let start = source
        .find(&declaration)
        .expect("Loom source-integrity event declaration must exist");
    let end = source[start..]
        .find("\n-- MT109_AUTHORITY_END")
        .map(|offset| start + offset)
        .expect("Loom source-integrity event declaration must terminate");
    (start, end)
}

fn schema_field_definition_bounds(source: &str, table: &str, field: &str) -> (usize, usize) {
    let declaration = format!("DEFINE FIELD OVERWRITE {field} ON TABLE {table} ");
    let start = source
        .find(&declaration)
        .expect("stage-correct field declaration must exist");
    let end = source[start..]
        .find(";\n")
        .map(|offset| start + offset + 1)
        .expect("stage-correct field declaration must terminate");
    (start, end)
}

fn schema_event_definition_bounds_until_next_table(source: &str, event: &str) -> (usize, usize) {
    let declaration = format!("DEFINE EVENT OVERWRITE {event} ");
    let start = source
        .find(&declaration)
        .expect("stage-correct event declaration must exist");
    let end = source[start..]
        .find("\nDEFINE TABLE OVERWRITE ")
        .map(|offset| start + offset)
        .expect("stage-correct event declaration must precede a table");
    (start, end)
}

fn mt120_record_user_update_guard_event_statements() -> &'static str {
    record_user_update_guard_event_block(SCHEMA)
}

fn record_user_update_guard_event_block(source: &str) -> &str {
    const BEGIN: &str = "-- MT120_RECORD_USER_UPDATE_GUARD_EVENT_BEGIN";
    const END: &str = "-- MT120_RECORD_USER_UPDATE_GUARD_EVENT_END";
    let start = source
        .find(BEGIN)
        .expect("record-user update event block must exist");
    let end = source[start..]
        .find(END)
        .map(|offset| start + offset + END.len())
        .expect("record-user update event block must terminate");
    &source[start..end]
}

fn mt120_record_user_update_stage_correction_statements() -> String {
    let mut statements = String::new();
    for table in MT120_RECORD_USER_UPDATE_GUARD_TABLES {
        let (start, end) = schema_table_definition_bounds(SCHEMA, table);
        statements.push_str(&SCHEMA[start..end]);
        statements.push('\n');
    }
    statements.push_str(mt120_record_user_update_guard_event_statements());
    statements.push('\n');
    statements
}

#[cfg(test)]
fn restore_pre_mt120_record_user_update_guards(mut source: String) -> String {
    for (table, current, previous) in MT120_RECORD_USER_UPDATE_GUARD_RESTORATIONS {
        let (start, end) = schema_table_definition_bounds(&source, table);
        let definition = &source[start..end];
        assert!(
            definition.contains(current),
            "stage-correct {table} table must retain its current-row update gate"
        );
        let restored = definition.replacen(current, previous, 1);
        source.replace_range(start..end, &restored);
    }
    source
}

#[cfg(test)]
fn restore_pre_account_authority_nonce_guards(mut source: String) -> String {
    for (table, fields) in AUTHORITY_NONCE_IMMUTABLE_FIELDS {
        let declaration = format!("DEFINE TABLE OVERWRITE {table} ");
        let start = source
            .find(&declaration)
            .expect("historical authority table declaration must exist");
        let end = source[start..]
            .find("\nDEFINE FIELD OVERWRITE ")
            .map(|offset| start + offset)
            .expect("historical authority table declaration must precede fields");
        let table_definition = &source[start..end];
        assert!(
            !table_definition.contains("$before.authorization_touch_nonce"),
            "stage-correct authority table must not retain transition checks"
        );
        let guard = format!(
            " AND $after.authorization_touch_nonce = ($before.authorization_touch_nonce ?? 0) + 1{}",
            fields
                .iter()
                .map(|field| format!(" AND $before.{field} = $after.{field}"))
                .collect::<String>(),
        );
        let marker = " FOR delete NONE;";
        let marker_offset = table_definition
            .rfind(marker)
            .expect("authority update permission must end in delete none");
        source.insert_str(start + marker_offset, &guard);
    }
    source
}

fn mt120_document_upgrade_statements() -> String {
    let mut statements = String::from(MT120_DOCUMENT_ACCESS_BLOCK);
    statements.push_str(MT120_IDEMPOTENCY_BLOCK);
    statements.push_str(RECORD_USER_PRODUCER_BLOCK);
    for (_, current) in MT120_DOCUMENT_TABLE_UPGRADES {
        statements.push_str(current);
        statements.push('\n');
    }
    statements.push_str(&authority_nonce_stage_correction_statements());
    statements.push('\n');
    statements.push_str(&mt120_record_user_update_stage_correction_statements());
    statements.push_str(&mt120_loom_bundle_table_statements());
    // Last, so the revision-160 deltas win over the stage-correction constants above.
    statements.push_str(&schema_delta_upgrade_statements());
    statements
}

const MT120_LOOM_BUNDLE_TABLES: [&str; 10] = [
    "protected_resources",
    "resource_grants",
    "loom_blocks",
    "kernel_event_ledger",
    "knowledge_entities",
    "loom_block_knowledge_bridge",
    "loom_canvas_boards",
    "loom_canvas_placements",
    "loom_canvas_visual_edges",
    "loom_block_search_index",
];

const MT120_LOOM_CREATED_SESSION_FIELD: &str =
    "DEFINE FIELD OVERWRITE created_in_session_id ON TABLE loom_blocks TYPE option<record<authenticated_sessions>> REFERENCE ON DELETE REJECT;\n";
const MT120_LOOM_CANVAS_PLACEMENT_RELATION_FIELDS: [&str; 3] =
    ["canvas_block_id", "workspace_id", "placed_block_id"];

fn mt120_loom_bundle_table_statements() -> String {
    let mut statements = String::new();
    for table in MT120_LOOM_BUNDLE_TABLES {
        let (start, end) = schema_table_definition_bounds(SCHEMA, table);
        statements.push_str(&SCHEMA[start..end]);
        statements.push('\n');
    }
    for field in MT120_LOOM_CANVAS_PLACEMENT_RELATION_FIELDS {
        let (start, end) = schema_field_definition_bounds(SCHEMA, "loom_canvas_placements", field);
        statements.push_str(&SCHEMA[start..end]);
        statements.push('\n');
    }
    statements.push_str(MT120_LOOM_CREATED_SESSION_FIELD);
    let (start, end) = schema_event_definition_bounds(SCHEMA, "mt109_loom_source_integrity");
    statements.push_str(&SCHEMA[start..end]);
    statements.push('\n');
    let (start, end) = schema_event_definition_bounds_until_next_table(
        SCHEMA,
        "enforce_loom_canvas_placement_integrity",
    );
    statements.push_str(&SCHEMA[start..end]);
    statements.push('\n');
    // MT-109 C2: Atelier projection delete, backlink link kinds and rich-document sources.
    for table in [
        "atelier_intake_item_loom_projection",
        "knowledge_document_backlinks",
        "knowledge_sources",
        // MT-109 C3: account-scoped Loom routes (folders, wiki, tags/edges, assets, collections,
        // AI suggestions, saved views, quick switcher) get least-privilege record-user permissions.
        "loom_edges",
        "loom_block_view_fr_outbox",
        "assets",
        "media_asset_tiers",
        "loom_folders",
        "loom_folder_members",
        "storage_graph_anchors",
        "knowledge_wiki_projections",
        "loom_wiki_overlays",
        "knowledge_quick_switcher_recents",
        "loom_collections",
        "loom_collection_members",
        "loom_ai_suggestions",
    ] {
        let (start, end) = schema_table_definition_bounds(SCHEMA, table);
        statements.push_str(&SCHEMA[start..end]);
        statements.push('\n');
    }
    statements
}

#[cfg(test)]
fn restore_pre_mt120_loom_bundle(mut source: String) -> String {
    source = source.replace(MT120_LOOM_CREATED_SESSION_FIELD, "");
    source = source.replace(
        "        IF $event = 'UPDATE' AND $after.created_in_session_id != $before.created_in_session_id {\n            THROW 'HSK-MT109-LOOM-CREATOR-SESSION-IMMUTABLE';\n        };\n",
        "",
    );
    source = source.replace(
        "            IF $source != NONE OR ($auth != NONE AND $after.created_in_session_id = NONE) { THROW 'HSK-MT109-LOOM-SOURCE-REQUIRED'; };",
        "            IF $source != NONE { THROW 'HSK-MT109-LOOM-SOURCE-REQUIRED'; };",
    );
    for table in MT120_LOOM_BUNDLE_TABLES {
        let (start, end) = schema_table_definition_bounds(&source, table);
        let current = &source[start..end];
        let restored = match table {
            "protected_resources" => current.replace(
                " OR fn::mt109_has_grant(resource_kind, external_resource_id, 'read', 'fs.read')",
                "",
            ),
            "resource_grants" => current.replace(
                " OR (capability_ids CONTAINS 'fs.read' AND actions CONTAINS 'read' AND ($auth.delegated_capabilities CONTAINS '*' OR $auth.delegated_capabilities CONTAINS 'fs.read') AND ($auth.principal_id.delegated_capabilities CONTAINS '*' OR $auth.principal_id.delegated_capabilities CONTAINS 'fs.read'))",
                "",
            ),
            "loom_blocks" => current
                .replace(
                    " OR (source_rich_document_id = NONE AND fn::mt120_loom_block_access(block_id, record::id(workspace_id), 'read', 'fs.read'))",
                    "",
                )
                .replace(
                    " OR (source_rich_document_id = NONE AND content_type IN ['note','file','annotated_file','tag_hub','journal','canvas'] AND $auth != NONE AND created_in_session_id = $auth.id AND fn::mt109_has_workspace_access(record::id(workspace_id), 'create', 'fs.write'))",
                    "",
                )
                .replace(
                    "FOR create WHERE (source_rich_document_id != NONE",
                    "FOR create WHERE source_rich_document_id != NONE",
                )
                .replace(
                    "record::id(workspace_id), 'update', 'fs.write')))\n                FOR update",
                    "record::id(workspace_id), 'update', 'fs.write'))\n                FOR update",
                )
                .replace(
                    " OR (source_rich_document_id = NONE AND fn::mt120_loom_block_access(block_id, record::id(workspace_id), 'delete', 'fs.write'))",
                    "",
                ),
            "kernel_event_ledger" => current.replace(
                " OR fn::mt120_loom_receipt(authority_resource_id, authority_session_id, authority_capability_id, authority_action, wsids, event_type, source_component, aggregate_type, aggregate_id, payload, actor_kind, actor_id, false)",
                "",
            ).replace(
                " OR fn::mt120_loom_receipt(authority_resource_id, authority_session_id, authority_capability_id, authority_action, wsids, event_type, source_component, aggregate_type, aggregate_id, payload, actor_kind, actor_id, true)",
                "",
            ),
            "knowledge_entities" => current.replace(
                " OR (entity_kind = 'loom_block' AND primary_source_id = NONE AND fn::mt120_loom_block_access(entity_key, record::id(workspace_id), 'read', 'fs.read'))",
                "",
            ).replace(
                " OR (entity_kind = 'loom_block' AND primary_source_id = NONE AND fn::mt120_loom_block_access(entity_key, record::id(workspace_id), 'create', 'fs.write'))",
                "",
            ),
            "loom_block_knowledge_bridge" | "loom_canvas_boards"
            | "loom_canvas_placements" | "loom_canvas_visual_edges" => {
                format!("DEFINE TABLE OVERWRITE {table} SCHEMAFULL PERMISSIONS NONE;")
            }
            "loom_block_search_index" => current.replace(
                " OR (block_id.source_rich_document_id = NONE AND fn::mt120_loom_block_access(record::id(block_id), record::id(workspace_id), 'read', 'fs.read'))",
                "",
            ).replace(
                " OR (block_id.source_rich_document_id = NONE AND fn::mt120_loom_block_access(record::id(block_id), record::id(workspace_id), 'create', 'fs.write'))",
                "",
            ).replace(
                "FOR select WHERE (block_id.source_rich_document_id != NONE",
                "FOR select WHERE block_id.source_rich_document_id != NONE",
            ).replace(
                "record::id(workspace_id), 'read', 'fs.read'))\n                FOR create",
                "record::id(workspace_id), 'read', 'fs.read')\n                FOR create",
            ).replace(
                "FOR create WHERE (block_id.source_rich_document_id != NONE",
                "FOR create WHERE block_id.source_rich_document_id != NONE",
            ).replace(
                "record::id(workspace_id), 'update', 'fs.write')))\n                FOR update",
                "record::id(workspace_id), 'update', 'fs.write'))\n                FOR update",
            ),
            _ => unreachable!("static Loom bundle table list is exhaustive"),
        };
        source.replace_range(start..end, &restored);
    }
    source = source
        .replace(
            "DEFINE FIELD OVERWRITE canvas_block_id ON TABLE loom_canvas_placements TYPE record<loom_canvas_boards> REFERENCE ON DELETE CASCADE;",
            "DEFINE FIELD OVERWRITE canvas_block_id ON TABLE loom_canvas_placements TYPE record<loom_canvas_boards> ASSERT record::exists($value) AND ($value.workspace_id = $this.workspace_id) AND ($value.workspace_id = $this.placed_block_id.workspace_id) REFERENCE ON DELETE CASCADE;",
        )
        .replace(
            "DEFINE FIELD OVERWRITE workspace_id ON TABLE loom_canvas_placements TYPE record<workspaces> REFERENCE ON DELETE CASCADE;",
            "DEFINE FIELD OVERWRITE workspace_id ON TABLE loom_canvas_placements TYPE record<workspaces> ASSERT record::exists($value) AND ($value = $this.canvas_block_id.workspace_id) AND ($value = $this.placed_block_id.workspace_id) REFERENCE ON DELETE CASCADE;",
        )
        .replace(
            "DEFINE FIELD OVERWRITE placed_block_id ON TABLE loom_canvas_placements TYPE record<loom_blocks> REFERENCE ON DELETE REJECT;",
            "DEFINE FIELD OVERWRITE placed_block_id ON TABLE loom_canvas_placements TYPE record<loom_blocks> ASSERT record::exists($value) AND ($value.workspace_id = $this.workspace_id) AND ($value.workspace_id = $this.canvas_block_id.workspace_id) REFERENCE ON DELETE REJECT;",
        );
    let (start, end) = schema_event_definition_bounds_until_next_table(
        &source,
        "enforce_loom_canvas_placement_integrity",
    );
    source.replace_range(start.saturating_sub(1)..end, "");
    source
}
/// MT-109 C2 schema deltas (workspace-delete Loom entities, loom_block update receipt, Canvas
/// visual-edge delete, Atelier projection delete, standalone search-index update). Predecessor
/// reconstructions revert them; upgrades re-emit them from [`SCHEMA`] via
/// [`schema_delta_upgrade_statements`].
const MT109_C2_SCHEMA_DELTAS: [(&str, &str); 11] = [
    ("        IF $entity.entity_kind = 'loom_block' AND $entity.primary_source_id = NONE {\n            IF array::len(SELECT id FROM protected_resources WHERE id IN $resources.id AND resource_kind IN ['loom_block','rich_document'] AND external_resource_id = $entity.entity_key AND lifecycle_state = 'active') != 1 { RETURN false; };\n        } ELSE IF $entity.primary_source_id = NONE OR $entity.primary_source_id.workspace_id != $workspace OR !fn::mt109_has_grant('knowledge_source', record::id($entity.primary_source_id), 'delete', 'fs.write') { RETURN false; };\n", "        IF $entity.primary_source_id = NONE OR $entity.primary_source_id.workspace_id != $workspace OR !fn::mt109_has_grant('knowledge_source', record::id($entity.primary_source_id), 'delete', 'fs.write') { RETURN false; };\n"),
    ("    IF $resource.resource_kind = 'loom_block' AND $resource.external_resource_id = $payload.block_id\n        AND $capability = 'fs.write' AND $action = 'update'\n        AND $event = 'KNOWLEDGE_LOOM_BLOCK_MUTATED' AND $source = 'loom_block' AND $aggregate = 'loom_block'\n        AND $aggregate_id = $payload.block_id AND $payload.type = 'knowledge_loom_block_mutated' AND $payload.operation = 'update'\n        AND type::record('loom_blocks', $payload.block_id).workspace_id = type::record('workspaces', $wsids[0]) {\n        RETURN ($write AND $session = $auth.id AND fn::mt109_ledger_access($resource, $session, $capability, $action)\n                AND fn::mt120_loom_block_access($payload.block_id, $wsids[0], 'update', 'fs.write'))\n            OR (!$write AND fn::mt120_loom_block_access($payload.block_id, $wsids[0], 'read', 'fs.read'));\n    };\n", ""),
    ("                FOR create, update NONE\n                FOR delete WHERE fn::mt120_loom_block_access(record::id(canvas_block_id), record::id(workspace_id), 'update', 'fs.write') OR fn::mt120_workspace_delete(record::id(workspace_id));\nDEFINE FIELD OVERWRITE visual_edge_id", "                FOR create, update NONE\n                FOR delete WHERE fn::mt120_workspace_delete(record::id(workspace_id));\nDEFINE FIELD OVERWRITE visual_edge_id"),
    ("DEFINE TABLE OVERWRITE atelier_intake_item_loom_projection SCHEMAFULL PERMISSIONS FOR select, create, update NONE FOR delete WHERE fn::mt109_has_workspace_access(record::id(workspace_id), 'delete', 'fs.write');\n", "DEFINE TABLE OVERWRITE atelier_intake_item_loom_projection SCHEMAFULL PERMISSIONS FOR select, create, update NONE FOR delete WHERE fn::mt120_workspace_delete(record::id(workspace_id));\n"),
    ("                FOR update WHERE (block_id.source_rich_document_id != NONE AND block_id.workspace_id = workspace_id AND block_id.source_rich_document_id.workspace_id = workspace_id AND block_id.source_rich_document_id.deleted_at = NONE AND fn::mt120_document_access(record::id(block_id.source_rich_document_id), record::id(workspace_id), 'update', 'fs.write')) OR (block_id.source_rich_document_id = NONE AND block_id.workspace_id = workspace_id AND fn::mt120_loom_block_access(record::id(block_id), record::id(workspace_id), 'update', 'fs.write'))\n                FOR delete WHERE block_id.source_rich_document_id != NONE", "                FOR update WHERE block_id.source_rich_document_id != NONE AND block_id.workspace_id = workspace_id AND block_id.source_rich_document_id.workspace_id = workspace_id AND block_id.source_rich_document_id.deleted_at = NONE AND fn::mt120_document_access(record::id(block_id.source_rich_document_id), record::id(workspace_id), 'update', 'fs.write')\n                FOR delete WHERE block_id.source_rich_document_id != NONE"),
    ("    RETURN $row.resource_kind IN ['flight_recorder', 'memory_pack', 'memory_proposal', 'memory_commit_report', 'memory_item', 'memory_item_count']\n        AND $row.parent_resource_id.resource_kind = 'workspace'", "    RETURN $row.resource_kind = 'flight_recorder'\n        AND $row.parent_resource_id.resource_kind = 'workspace'"),
    ("             AND ['fs.read','fs.write','fr.read','fr.ingest.runtime_chat','fr.ingest.native_editor','memory.read','memory.propose','memory.review','memory.commit'] CONTAINSALL $row.capability_ids)\n          OR ($row.resource_id.resource_kind IN ['memory_pack','memory_proposal','memory_commit_report','memory_item','memory_item_count'] AND ['create','read','update','delete'] CONTAINSALL $row.actions AND ['memory.read','memory.propose','memory.review','memory.commit','fs.write'] CONTAINSALL $row.capability_ids AND $row.actions CONTAINS 'delete' AND $row.capability_ids CONTAINS 'fs.write')\n", "             AND ['fs.read','fs.write','fr.read','fr.ingest.runtime_chat','fr.ingest.native_editor','memory.read','memory.propose'] CONTAINSALL $row.capability_ids)\n"),
    ("        RETURN $source.created_in_session_id = $auth.id AND $source.source_kind IN ['file', 'rich_document']\n", "        RETURN $source.created_in_session_id = $auth.id AND $source.source_kind = 'file'\n"),
    ("    PERMISSIONS FOR select WHERE source_kind IN ['file', 'rich_document'] AND fn::mt109_source_read('knowledge_source', source_id, record::id(workspace_id), 'workspace', record::id(workspace_id))\n                FOR create WHERE fn::mt109_live_session() AND created_in_session_id = $auth.id AND source_kind IN ['file', 'rich_document'] AND (root_id", "    PERMISSIONS FOR select WHERE source_kind = 'file' AND fn::mt109_source_read('knowledge_source', source_id, record::id(workspace_id), 'workspace', record::id(workspace_id))\n                FOR create WHERE fn::mt109_live_session() AND created_in_session_id = $auth.id AND source_kind = 'file' AND (root_id"),
    ("DEFINE TABLE OVERWRITE knowledge_document_backlinks SCHEMAFULL\n    PERMISSIONS FOR select WHERE source_document_id.workspace_id = workspace_id AND fn::mt120_document_access(record::id(source_document_id), record::id(workspace_id), 'read', 'fs.read') AND (fn::mt120_document_access(target, record::id(workspace_id), 'read', 'fs.read') OR !record::exists(type::record('knowledge_rich_documents', target)))\n                FOR create WHERE source_document_id.workspace_id = workspace_id AND fn::mt120_document_access(record::id(source_document_id), record::id(workspace_id), 'update', 'fs.write') AND (fn::mt120_document_access(target, record::id(workspace_id), 'read', 'fs.read') OR !record::exists(type::record('knowledge_rich_documents', target)))\n                FOR update WHERE source_document_id.workspace_id = workspace_id AND fn::mt120_document_access(record::id(source_document_id), record::id(workspace_id), 'update', 'fs.write') AND (fn::mt120_document_access(target, record::id(workspace_id), 'read', 'fs.read') OR !record::exists(type::record('knowledge_rich_documents', target)))\n                FOR delete WHERE source_document_id.workspace_id = workspace_id AND fn::mt120_document_access(record::id(source_document_id), record::id(workspace_id), 'update', 'fs.write') AND (fn::mt120_document_access(target, record::id(workspace_id), 'read', 'fs.read') OR !record::exists(type::record('knowledge_rich_documents', target)));\n", "DEFINE TABLE OVERWRITE knowledge_document_backlinks SCHEMAFULL\n    PERMISSIONS FOR select WHERE source_document_id.workspace_id = workspace_id AND link_kind = 'wikilink' AND fn::mt120_document_access(record::id(source_document_id), record::id(workspace_id), 'read', 'fs.read') AND fn::mt120_document_access(target, record::id(workspace_id), 'read', 'fs.read')\n                FOR create WHERE source_document_id.workspace_id = workspace_id AND link_kind = 'wikilink' AND fn::mt120_document_access(record::id(source_document_id), record::id(workspace_id), 'update', 'fs.write') AND fn::mt120_document_access(target, record::id(workspace_id), 'read', 'fs.read')\n                FOR update WHERE source_document_id.workspace_id = workspace_id AND link_kind = 'wikilink' AND fn::mt120_document_access(record::id(source_document_id), record::id(workspace_id), 'update', 'fs.write') AND fn::mt120_document_access(target, record::id(workspace_id), 'read', 'fs.read')\n                FOR delete WHERE source_document_id.workspace_id = workspace_id AND link_kind = 'wikilink' AND fn::mt120_document_access(record::id(source_document_id), record::id(workspace_id), 'update', 'fs.write') AND fn::mt120_document_access(target, record::id(workspace_id), 'read', 'fs.read');\n"),
    ("    RETURN ($write AND $session = $auth.id AND fn::mt109_ledger_access($resource, $session, $capability, $action)\n            AND fn::mt120_loom_block_access($payload.canvas_block_id, $wsids[0], 'update', 'fs.write')\n            AND (fn::mt120_loom_block_access($payload.placed_block_id, $wsids[0], 'read', 'fs.read') OR fn::mt120_document_access($payload.placed_block_id, $wsids[0], 'read', 'fs.read')))\n        OR (!$write AND $session = $auth.id AND fn::mt109_ledger_access($resource, $session, $capability, $action)\n            AND fn::mt120_loom_block_access($payload.canvas_block_id, $wsids[0], 'update', 'fs.write')\n            AND (fn::mt120_loom_block_access($payload.placed_block_id, $wsids[0], 'read', 'fs.read') OR fn::mt120_document_access($payload.placed_block_id, $wsids[0], 'read', 'fs.read')))\n        OR (!$write AND fn::mt120_loom_block_access($payload.canvas_block_id, $wsids[0], 'read', 'fs.read')\n            AND (fn::mt120_loom_block_access($payload.placed_block_id, $wsids[0], 'read', 'fs.read') OR fn::mt120_document_access($payload.placed_block_id, $wsids[0], 'read', 'fs.read')));\n", "    RETURN ($write AND $session = $auth.id AND fn::mt109_ledger_access($resource, $session, $capability, $action)\n            AND fn::mt120_loom_block_access($payload.canvas_block_id, $wsids[0], 'update', 'fs.write')\n            AND fn::mt120_loom_block_access($payload.placed_block_id, $wsids[0], 'read', 'fs.read'))\n        OR (!$write AND $session = $auth.id AND fn::mt109_ledger_access($resource, $session, $capability, $action)\n            AND fn::mt120_loom_block_access($payload.canvas_block_id, $wsids[0], 'update', 'fs.write')\n            AND fn::mt120_loom_block_access($payload.placed_block_id, $wsids[0], 'read', 'fs.read'))\n        OR (!$write AND fn::mt120_loom_block_access($payload.canvas_block_id, $wsids[0], 'read', 'fs.read')\n            AND fn::mt120_loom_block_access($payload.placed_block_id, $wsids[0], 'read', 'fs.read'));\n"),
];

/// MT-109 C3 schema deltas (Loom endpoint helper, workspace/block Loom receipts, record-user
/// permissions for the account-scoped Loom routes). Predecessor reconstructions revert them before
/// the C2 deltas; upgrades re-emit them from [`SCHEMA`] via [`schema_delta_upgrade_statements`].
const MT109_C3_SCHEMA_DELTAS: [(&str, &str); 22] = [
    ("DEFINE FUNCTION OVERWRITE fn::mt120_loom_endpoint_access($block: option<record<loom_blocks>>, $workspace: string, $action: string, $capability: string) {\n    IF $block = NONE OR $block.workspace_id != type::record('workspaces', $workspace) { RETURN false; };\n    IF $block.source_rich_document_id = NONE {\n        RETURN fn::mt120_loom_block_access(record::id($block), $workspace, $action, $capability);\n    };\n    RETURN record::id($block.source_rich_document_id) = record::id($block)\n        AND fn::mt120_document_access(record::id($block), $workspace, $action, $capability);\n};\nDEFINE FUNCTION OVERWRITE fn::mt120_loom_receipt($resource: ", "DEFINE FUNCTION OVERWRITE fn::mt120_loom_receipt($resource: "),
    ("    IF (($event = 'KNOWLEDGE_LOOM_FOLDER_MUTATED' AND $source = 'loom_folder' AND $aggregate = 'loom_folder')\n        OR ($event = 'KNOWLEDGE_LOOM_TAG_MUTATED' AND $source = 'loom_edge' AND $aggregate = 'loom_edge')\n        OR ($event = 'KNOWLEDGE_LOOM_BLOCK_MUTATED' AND $source = 'loom_block' AND $aggregate = 'loom_block')\n        OR ($event = 'KNOWLEDGE_LOOM_BLOCK_INDEXED' AND (($source = 'loom_block_knowledge_bridge' AND $aggregate = 'knowledge_loom_block')\n            OR ($source = 'loom_search_v2' AND $aggregate = 'loom_block_search_index')))\n        OR ($event = 'KNOWLEDGE_LOOM_WIKI_MUTATED' AND $source = 'loom_wiki' AND $aggregate = 'loom_wiki_overlay')\n        OR ($event = 'KNOWLEDGE_PROJECTION_REBUILT' AND $source IN ['project_wiki_compiler', 'project_wiki_drift_checker']\n            AND $aggregate = 'knowledge_wiki' AND $aggregate_id = $wsids[0])\n        OR ($event = 'KNOWLEDGE_QUICK_SWITCHER_RECENT_RECORDED' AND $source = 'quick_switcher_recents' AND $aggregate = 'quick_switcher_recent')\n        OR ($event IN ['AI_EDIT_PROPOSAL_RECORDED', 'AI_EDIT_PROPOSAL_DECIDED'] AND $source IN ['loom_ai_job', 'loom_ai_promotion'] AND $aggregate = 'loom_ai_suggestion')\n        OR ($event IN ['PROMOTION_REQUESTED', 'PROMOTION_ACCEPTED', 'PROMOTION_REJECTED'] AND $source = 'loom_ai_promotion' AND $aggregate = 'loom_ai_promotion')\n        OR ($event = 'KNOWLEDGE_LOOM_CANVAS_BOARD_RECORDED' AND $source = 'loom_canvas_board' AND $aggregate = 'loom_canvas_board'\n            AND $payload.op = 'viewport' AND $resource.resource_kind = 'loom_block'))\n        AND $capability = 'fs.write' AND $action IN ['create', 'update', 'delete'] {\n        IF $resource.resource_kind = 'workspace' AND $resource.external_resource_id = $wsids[0] {\n            RETURN ($write AND $session = $auth.id AND fn::mt109_ledger_access($resource, $session, $capability, $action)\n                    AND fn::mt109_has_workspace_access($wsids[0], $action, 'fs.write'))\n                OR (!$write AND fn::mt109_has_workspace_access($wsids[0], 'read', 'fs.read'));\n        };\n        IF $resource.resource_kind IN ['loom_block', 'rich_document'] AND $action = 'update'\n            AND ($resource.external_resource_id = $payload.block_id OR $resource.external_resource_id = $payload.source_block_id) {\n            LET $block = type::record('loom_blocks', $resource.external_resource_id);\n            RETURN ($write AND $session = $auth.id AND fn::mt109_ledger_access($resource, $session, $capability, $action)\n                    AND fn::mt120_loom_endpoint_access($block, $wsids[0], 'update', 'fs.write'))\n                OR (!$write AND fn::mt120_loom_endpoint_access($block, $wsids[0], 'read', 'fs.read'));\n        };\n    };\n    IF $resource.resource_kind != 'loom_block' OR $resource.external_resource_id != $payload.canvas_block_id\n        OR $capability != 'fs.write' OR $action != 'update'", "    IF $resource.resource_kind != 'loom_block' OR $resource.external_resource_id != $payload.canvas_block_id\n        OR $capability != 'fs.write' OR $action != 'update'"),
    ("DEFINE TABLE OVERWRITE loom_blocks SCHEMAFULL\n    PERMISSIONS FOR select WHERE ((source_rich_document_id = NONE AND fn::mt109_source_read('loom_block', block_id, record::id(workspace_id), 'workspace', record::id(workspace_id))) OR (source_rich_document_id != NONE AND record::id(source_rich_document_id) = block_id AND content_type = 'note' AND fn::mt109_source_read('loom_block', block_id, record::id(workspace_id), 'rich_document', record::id(source_rich_document_id)) AND fn::mt109_source_read('rich_document', record::id(source_rich_document_id), record::id(workspace_id), 'workspace', record::id(workspace_id)) AND source_rich_document_id.workspace_id = workspace_id AND source_rich_document_id.deleted_at = NONE AND content_hash = source_rich_document_id.content_sha256)) OR (source_rich_document_id != NONE AND record::id(source_rich_document_id) = block_id AND content_type = 'note' AND source_rich_document_id.workspace_id = workspace_id AND source_rich_document_id.deleted_at = NONE AND content_hash = source_rich_document_id.content_sha256 AND fn::mt120_document_access(record::id(source_rich_document_id), record::id(workspace_id), 'read', 'fs.read')) OR (source_rich_document_id = NONE AND fn::mt120_loom_block_access(block_id, record::id(workspace_id), 'read', 'fs.read')) OR (source_rich_document_id != NONE AND record::id(source_rich_document_id) = block_id AND content_type = 'note' AND source_rich_document_id.workspace_id = workspace_id AND source_rich_document_id.deleted_at = NONE AND fn::mt120_document_access(record::id(source_rich_document_id), record::id(workspace_id), 'update', 'fs.write')) OR (source_rich_document_id = NONE AND content_type = 'view_def' AND fn::mt109_has_workspace_access(record::id(workspace_id), 'read', 'fs.read'))\n                FOR create WHERE (source_rich_document_id != NONE AND record::id(source_rich_document_id) = block_id AND content_type = 'note' AND source_rich_document_id.workspace_id = workspace_id AND source_rich_document_id.deleted_at = NONE AND content_hash = source_rich_document_id.content_sha256 AND (fn::mt120_document_access(record::id(source_rich_document_id), record::id(workspace_id), 'create', 'fs.write') OR fn::mt120_document_access(record::id(source_rich_document_id), record::id(workspace_id), 'update', 'fs.write'))) OR (source_rich_document_id = NONE AND content_type IN ['note','file','annotated_file','tag_hub','journal','canvas'] AND $auth != NONE AND created_in_session_id = $auth.id AND fn::mt109_has_workspace_access(record::id(workspace_id), 'create', 'fs.write')) OR (source_rich_document_id = NONE AND content_type = 'view_def' AND $auth != NONE AND created_in_session_id = $auth.id AND fn::mt109_has_workspace_access(record::id(workspace_id), 'create', 'fs.write'))\n                FOR update WHERE (source_rich_document_id != NONE AND record::id(source_rich_document_id) = block_id AND content_type = 'note' AND source_rich_document_id.workspace_id = workspace_id AND source_rich_document_id.deleted_at = NONE AND content_hash = source_rich_document_id.content_sha256 AND fn::mt120_document_access(record::id(source_rich_document_id), record::id(workspace_id), 'update', 'fs.write')) OR (source_rich_document_id = NONE AND content_type IN ['note','file','annotated_file','tag_hub','journal','canvas'] AND fn::mt120_loom_block_access(block_id, record::id(workspace_id), 'update', 'fs.write')) OR (source_rich_document_id != NONE AND record::id(source_rich_document_id) = block_id AND content_type = 'note' AND source_rich_document_id.workspace_id = workspace_id AND source_rich_document_id.deleted_at = NONE AND fn::mt120_document_access(record::id(source_rich_document_id), record::id(workspace_id), 'update', 'fs.write')) OR (source_rich_document_id = NONE AND content_type = 'view_def' AND fn::mt109_has_workspace_access(record::id(workspace_id), 'update', 'fs.write'))\n                FOR delete WHERE (source_rich_document_id != NONE AND record::id(source_rich_document_id) = block_id AND source_rich_document_id.workspace_id = workspace_id AND content_type = 'note' AND fn::mt120_document_delete(block_id, record::id(workspace_id))) OR (source_rich_document_id = NONE AND fn::mt120_loom_block_access(block_id, record::id(workspace_id), 'delete', 'fs.write')) OR fn::mt120_workspace_delete(record::id(workspace_id)) OR (source_rich_document_id = NONE AND content_type = 'view_def' AND fn::mt109_has_workspace_access(record::id(workspace_id), 'delete', 'fs.write'));", "DEFINE TABLE OVERWRITE loom_blocks SCHEMAFULL\n    PERMISSIONS FOR select WHERE ((source_rich_document_id = NONE AND fn::mt109_source_read('loom_block', block_id, record::id(workspace_id), 'workspace', record::id(workspace_id))) OR (source_rich_document_id != NONE AND record::id(source_rich_document_id) = block_id AND content_type = 'note' AND fn::mt109_source_read('loom_block', block_id, record::id(workspace_id), 'rich_document', record::id(source_rich_document_id)) AND fn::mt109_source_read('rich_document', record::id(source_rich_document_id), record::id(workspace_id), 'workspace', record::id(workspace_id)) AND source_rich_document_id.workspace_id = workspace_id AND source_rich_document_id.deleted_at = NONE AND content_hash = source_rich_document_id.content_sha256)) OR (source_rich_document_id != NONE AND record::id(source_rich_document_id) = block_id AND content_type = 'note' AND source_rich_document_id.workspace_id = workspace_id AND source_rich_document_id.deleted_at = NONE AND content_hash = source_rich_document_id.content_sha256 AND fn::mt120_document_access(record::id(source_rich_document_id), record::id(workspace_id), 'read', 'fs.read')) OR (source_rich_document_id = NONE AND fn::mt120_loom_block_access(block_id, record::id(workspace_id), 'read', 'fs.read')) OR (source_rich_document_id != NONE AND record::id(source_rich_document_id) = block_id AND content_type = 'note' AND source_rich_document_id.workspace_id = workspace_id AND source_rich_document_id.deleted_at = NONE AND fn::mt120_document_access(record::id(source_rich_document_id), record::id(workspace_id), 'update', 'fs.write'))\n                FOR create WHERE (source_rich_document_id != NONE AND record::id(source_rich_document_id) = block_id AND content_type = 'note' AND source_rich_document_id.workspace_id = workspace_id AND source_rich_document_id.deleted_at = NONE AND content_hash = source_rich_document_id.content_sha256 AND (fn::mt120_document_access(record::id(source_rich_document_id), record::id(workspace_id), 'create', 'fs.write') OR fn::mt120_document_access(record::id(source_rich_document_id), record::id(workspace_id), 'update', 'fs.write'))) OR (source_rich_document_id = NONE AND content_type IN ['note','file','annotated_file','tag_hub','journal','canvas'] AND $auth != NONE AND created_in_session_id = $auth.id AND fn::mt109_has_workspace_access(record::id(workspace_id), 'create', 'fs.write'))\n                FOR update WHERE (source_rich_document_id != NONE AND record::id(source_rich_document_id) = block_id AND content_type = 'note' AND source_rich_document_id.workspace_id = workspace_id AND source_rich_document_id.deleted_at = NONE AND content_hash = source_rich_document_id.content_sha256 AND fn::mt120_document_access(record::id(source_rich_document_id), record::id(workspace_id), 'update', 'fs.write')) OR (source_rich_document_id = NONE AND content_type IN ['note','file','annotated_file','tag_hub','journal','canvas'] AND fn::mt120_loom_block_access(block_id, record::id(workspace_id), 'update', 'fs.write')) OR (source_rich_document_id != NONE AND record::id(source_rich_document_id) = block_id AND content_type = 'note' AND source_rich_document_id.workspace_id = workspace_id AND source_rich_document_id.deleted_at = NONE AND fn::mt120_document_access(record::id(source_rich_document_id), record::id(workspace_id), 'update', 'fs.write'))\n                FOR delete WHERE (source_rich_document_id != NONE AND record::id(source_rich_document_id) = block_id AND source_rich_document_id.workspace_id = workspace_id AND content_type = 'note' AND fn::mt120_document_delete(block_id, record::id(workspace_id))) OR (source_rich_document_id = NONE AND fn::mt120_loom_block_access(block_id, record::id(workspace_id), 'delete', 'fs.write')) OR fn::mt120_workspace_delete(record::id(workspace_id));"),
    ("DEFINE TABLE OVERWRITE loom_edges SCHEMAFULL\n    PERMISSIONS FOR select WHERE source_document_id != NONE AND source_block_id.source_rich_document_id != NONE AND record::id(source_block_id.source_rich_document_id) = source_document_id AND source_block_id.workspace_id = workspace_id AND target_block_id.workspace_id = workspace_id AND target_block_id.source_rich_document_id != NONE AND edge_type = 'mention' AND last_actor_kind = 'SYSTEM' AND last_actor_id = 'knowledge_rich_document_backlink_projection' AND fn::mt120_document_access(source_document_id, record::id(workspace_id), 'read', 'fs.read') AND fn::mt120_document_access(record::id(target_block_id.source_rich_document_id), record::id(workspace_id), 'read', 'fs.read') OR (last_actor_id != 'knowledge_rich_document_backlink_projection' AND source_block_id.workspace_id = workspace_id AND target_block_id.workspace_id = workspace_id AND fn::mt120_loom_endpoint_access(source_block_id, record::id(workspace_id), 'read', 'fs.read') AND fn::mt120_loom_endpoint_access(target_block_id, record::id(workspace_id), 'read', 'fs.read'))\n                FOR create WHERE source_document_id != NONE AND source_block_id.source_rich_document_id != NONE AND record::id(source_block_id.source_rich_document_id) = source_document_id AND source_block_id.workspace_id = workspace_id AND target_block_id.workspace_id = workspace_id AND target_block_id.source_rich_document_id != NONE AND edge_type = 'mention' AND last_actor_kind = 'SYSTEM' AND last_actor_id = 'knowledge_rich_document_backlink_projection' AND fn::mt120_document_access(source_document_id, record::id(workspace_id), 'update', 'fs.write') AND fn::mt120_document_access(record::id(target_block_id.source_rich_document_id), record::id(workspace_id), 'read', 'fs.read') OR (last_actor_id != 'knowledge_rich_document_backlink_projection' AND source_block_id.workspace_id = workspace_id AND target_block_id.workspace_id = workspace_id AND fn::mt120_loom_endpoint_access(source_block_id, record::id(workspace_id), 'update', 'fs.write') AND fn::mt120_loom_endpoint_access(target_block_id, record::id(workspace_id), 'read', 'fs.read'))\n                FOR update WHERE source_document_id != NONE AND source_block_id.source_rich_document_id != NONE AND record::id(source_block_id.source_rich_document_id) = source_document_id AND source_block_id.workspace_id = workspace_id AND target_block_id.workspace_id = workspace_id AND target_block_id.source_rich_document_id != NONE AND edge_type = 'mention' AND last_actor_kind = 'SYSTEM' AND last_actor_id = 'knowledge_rich_document_backlink_projection' AND fn::mt120_document_access(source_document_id, record::id(workspace_id), 'update', 'fs.write') AND fn::mt120_document_access(record::id(target_block_id.source_rich_document_id), record::id(workspace_id), 'read', 'fs.read') OR (last_actor_id != 'knowledge_rich_document_backlink_projection' AND source_block_id.workspace_id = workspace_id AND target_block_id.workspace_id = workspace_id AND fn::mt120_loom_endpoint_access(source_block_id, record::id(workspace_id), 'update', 'fs.write') AND fn::mt120_loom_endpoint_access(target_block_id, record::id(workspace_id), 'read', 'fs.read'))\n                FOR delete WHERE source_document_id != NONE AND source_block_id.source_rich_document_id != NONE AND record::id(source_block_id.source_rich_document_id) = source_document_id AND source_block_id.workspace_id = workspace_id AND target_block_id.workspace_id = workspace_id AND target_block_id.source_rich_document_id != NONE AND edge_type = 'mention' AND last_actor_kind = 'SYSTEM' AND last_actor_id = 'knowledge_rich_document_backlink_projection' AND fn::mt120_document_access(source_document_id, record::id(workspace_id), 'update', 'fs.write') AND fn::mt120_document_access(record::id(target_block_id.source_rich_document_id), record::id(workspace_id), 'read', 'fs.read') OR fn::mt120_workspace_delete(record::id(workspace_id)) OR (last_actor_id != 'knowledge_rich_document_backlink_projection' AND source_block_id.workspace_id = workspace_id AND target_block_id.workspace_id = workspace_id AND fn::mt120_loom_endpoint_access(source_block_id, record::id(workspace_id), 'update', 'fs.write') AND fn::mt120_loom_endpoint_access(target_block_id, record::id(workspace_id), 'read', 'fs.read'));", "DEFINE TABLE OVERWRITE loom_edges SCHEMAFULL\n    PERMISSIONS FOR select WHERE source_document_id != NONE AND source_block_id.source_rich_document_id != NONE AND record::id(source_block_id.source_rich_document_id) = source_document_id AND source_block_id.workspace_id = workspace_id AND target_block_id.workspace_id = workspace_id AND target_block_id.source_rich_document_id != NONE AND edge_type = 'mention' AND last_actor_kind = 'SYSTEM' AND last_actor_id = 'knowledge_rich_document_backlink_projection' AND fn::mt120_document_access(source_document_id, record::id(workspace_id), 'read', 'fs.read') AND fn::mt120_document_access(record::id(target_block_id.source_rich_document_id), record::id(workspace_id), 'read', 'fs.read')\n                FOR create WHERE source_document_id != NONE AND source_block_id.source_rich_document_id != NONE AND record::id(source_block_id.source_rich_document_id) = source_document_id AND source_block_id.workspace_id = workspace_id AND target_block_id.workspace_id = workspace_id AND target_block_id.source_rich_document_id != NONE AND edge_type = 'mention' AND last_actor_kind = 'SYSTEM' AND last_actor_id = 'knowledge_rich_document_backlink_projection' AND fn::mt120_document_access(source_document_id, record::id(workspace_id), 'update', 'fs.write') AND fn::mt120_document_access(record::id(target_block_id.source_rich_document_id), record::id(workspace_id), 'read', 'fs.read')\n                FOR update WHERE source_document_id != NONE AND source_block_id.source_rich_document_id != NONE AND record::id(source_block_id.source_rich_document_id) = source_document_id AND source_block_id.workspace_id = workspace_id AND target_block_id.workspace_id = workspace_id AND target_block_id.source_rich_document_id != NONE AND edge_type = 'mention' AND last_actor_kind = 'SYSTEM' AND last_actor_id = 'knowledge_rich_document_backlink_projection' AND fn::mt120_document_access(source_document_id, record::id(workspace_id), 'update', 'fs.write') AND fn::mt120_document_access(record::id(target_block_id.source_rich_document_id), record::id(workspace_id), 'read', 'fs.read')\n                FOR delete WHERE source_document_id != NONE AND source_block_id.source_rich_document_id != NONE AND record::id(source_block_id.source_rich_document_id) = source_document_id AND source_block_id.workspace_id = workspace_id AND target_block_id.workspace_id = workspace_id AND target_block_id.source_rich_document_id != NONE AND edge_type = 'mention' AND last_actor_kind = 'SYSTEM' AND last_actor_id = 'knowledge_rich_document_backlink_projection' AND fn::mt120_document_access(source_document_id, record::id(workspace_id), 'update', 'fs.write') AND fn::mt120_document_access(record::id(target_block_id.source_rich_document_id), record::id(workspace_id), 'read', 'fs.read') OR fn::mt120_workspace_delete(record::id(workspace_id));"),
    ("DEFINE TABLE OVERWRITE loom_block_search_index SCHEMAFULL\n    PERMISSIONS FOR select WHERE (block_id.source_rich_document_id != NONE AND block_id.workspace_id = workspace_id AND block_id.source_rich_document_id.workspace_id = workspace_id AND block_id.source_rich_document_id.deleted_at = NONE AND fn::mt120_document_access(record::id(block_id.source_rich_document_id), record::id(workspace_id), 'read', 'fs.read')) OR (block_id.source_rich_document_id = NONE AND fn::mt120_loom_block_access(record::id(block_id), record::id(workspace_id), 'read', 'fs.read')) OR (block_id.source_rich_document_id = NONE AND block_id.content_type = 'view_def' AND block_id.workspace_id = workspace_id AND fn::mt109_has_workspace_access(record::id(workspace_id), 'read', 'fs.read'))\n                FOR create WHERE (block_id.source_rich_document_id != NONE AND block_id.workspace_id = workspace_id AND block_id.source_rich_document_id.workspace_id = workspace_id AND block_id.source_rich_document_id.deleted_at = NONE AND (fn::mt120_document_access(record::id(block_id.source_rich_document_id), record::id(workspace_id), 'create', 'fs.write') OR fn::mt120_document_access(record::id(block_id.source_rich_document_id), record::id(workspace_id), 'update', 'fs.write'))) OR (block_id.source_rich_document_id = NONE AND fn::mt120_loom_block_access(record::id(block_id), record::id(workspace_id), 'create', 'fs.write')) OR (block_id.source_rich_document_id = NONE AND block_id.content_type = 'view_def' AND block_id.workspace_id = workspace_id AND fn::mt109_has_workspace_access(record::id(workspace_id), 'create', 'fs.write'))\n                FOR update WHERE (block_id.source_rich_document_id != NONE AND block_id.workspace_id = workspace_id AND block_id.source_rich_document_id.workspace_id = workspace_id AND block_id.source_rich_document_id.deleted_at = NONE AND fn::mt120_document_access(record::id(block_id.source_rich_document_id), record::id(workspace_id), 'update', 'fs.write')) OR (block_id.source_rich_document_id = NONE AND block_id.workspace_id = workspace_id AND fn::mt120_loom_block_access(record::id(block_id), record::id(workspace_id), 'update', 'fs.write')) OR (block_id.source_rich_document_id = NONE AND block_id.content_type = 'view_def' AND block_id.workspace_id = workspace_id AND fn::mt109_has_workspace_access(record::id(workspace_id), 'update', 'fs.write'))\n                FOR delete WHERE block_id.source_rich_document_id != NONE AND block_id.workspace_id = workspace_id AND block_id.source_rich_document_id.workspace_id = workspace_id AND block_id.source_rich_document_id.deleted_at = NONE AND fn::mt120_document_access(record::id(block_id.source_rich_document_id), record::id(workspace_id), 'update', 'fs.write') OR fn::mt120_workspace_delete(record::id(workspace_id)) OR (block_id.source_rich_document_id != NONE AND block_id.workspace_id = workspace_id AND fn::mt120_document_delete(record::id(block_id.source_rich_document_id), record::id(workspace_id)));", "DEFINE TABLE OVERWRITE loom_block_search_index SCHEMAFULL\n    PERMISSIONS FOR select WHERE (block_id.source_rich_document_id != NONE AND block_id.workspace_id = workspace_id AND block_id.source_rich_document_id.workspace_id = workspace_id AND block_id.source_rich_document_id.deleted_at = NONE AND fn::mt120_document_access(record::id(block_id.source_rich_document_id), record::id(workspace_id), 'read', 'fs.read')) OR (block_id.source_rich_document_id = NONE AND fn::mt120_loom_block_access(record::id(block_id), record::id(workspace_id), 'read', 'fs.read'))\n                FOR create WHERE (block_id.source_rich_document_id != NONE AND block_id.workspace_id = workspace_id AND block_id.source_rich_document_id.workspace_id = workspace_id AND block_id.source_rich_document_id.deleted_at = NONE AND (fn::mt120_document_access(record::id(block_id.source_rich_document_id), record::id(workspace_id), 'create', 'fs.write') OR fn::mt120_document_access(record::id(block_id.source_rich_document_id), record::id(workspace_id), 'update', 'fs.write'))) OR (block_id.source_rich_document_id = NONE AND fn::mt120_loom_block_access(record::id(block_id), record::id(workspace_id), 'create', 'fs.write'))\n                FOR update WHERE (block_id.source_rich_document_id != NONE AND block_id.workspace_id = workspace_id AND block_id.source_rich_document_id.workspace_id = workspace_id AND block_id.source_rich_document_id.deleted_at = NONE AND fn::mt120_document_access(record::id(block_id.source_rich_document_id), record::id(workspace_id), 'update', 'fs.write')) OR (block_id.source_rich_document_id = NONE AND block_id.workspace_id = workspace_id AND fn::mt120_loom_block_access(record::id(block_id), record::id(workspace_id), 'update', 'fs.write'))\n                FOR delete WHERE block_id.source_rich_document_id != NONE AND block_id.workspace_id = workspace_id AND block_id.source_rich_document_id.workspace_id = workspace_id AND block_id.source_rich_document_id.deleted_at = NONE AND fn::mt120_document_access(record::id(block_id.source_rich_document_id), record::id(workspace_id), 'update', 'fs.write') OR fn::mt120_workspace_delete(record::id(workspace_id)) OR (block_id.source_rich_document_id != NONE AND block_id.workspace_id = workspace_id AND fn::mt120_document_delete(record::id(block_id.source_rich_document_id), record::id(workspace_id)));"),
    ("DEFINE TABLE OVERWRITE loom_block_knowledge_bridge SCHEMAFULL\n    PERMISSIONS FOR select WHERE fn::mt120_loom_block_access(record::id(block_id), record::id(workspace_id), 'read', 'fs.read') OR (block_id.content_type = 'view_def' AND block_id.workspace_id = workspace_id AND fn::mt109_has_workspace_access(record::id(workspace_id), 'read', 'fs.read'))\n                FOR create WHERE block_id.workspace_id = workspace_id AND entity_id.workspace_id = workspace_id AND entity_id.entity_kind = 'loom_block' AND entity_id.entity_key = record::id(block_id) AND fn::mt120_loom_block_access(record::id(block_id), record::id(workspace_id), 'create', 'fs.write') OR (block_id.content_type = 'view_def' AND block_id.workspace_id = workspace_id AND entity_id.workspace_id = workspace_id AND entity_id.entity_kind = 'loom_block' AND entity_id.entity_key = record::id(block_id) AND fn::mt109_has_workspace_access(record::id(workspace_id), 'create', 'fs.write'))\n                FOR update NONE FOR delete NONE;", "DEFINE TABLE OVERWRITE loom_block_knowledge_bridge SCHEMAFULL\n    PERMISSIONS FOR select WHERE fn::mt120_loom_block_access(record::id(block_id), record::id(workspace_id), 'read', 'fs.read')\n                FOR create WHERE block_id.workspace_id = workspace_id AND entity_id.workspace_id = workspace_id AND entity_id.entity_kind = 'loom_block' AND entity_id.entity_key = record::id(block_id) AND fn::mt120_loom_block_access(record::id(block_id), record::id(workspace_id), 'create', 'fs.write')\n                FOR update NONE FOR delete NONE;"),
    ("DEFINE TABLE OVERWRITE knowledge_entities SCHEMAFULL PERMISSIONS FOR select WHERE fn::mt120_entity_read(id) OR (entity_kind = 'loom_block' AND primary_source_id = NONE AND fn::mt120_loom_block_access(entity_key, record::id(workspace_id), 'read', 'fs.read')) OR (entity_kind = 'loom_block' AND primary_source_id = NONE AND type::record('loom_blocks', entity_key).content_type = 'view_def' AND type::record('loom_blocks', entity_key).workspace_id = workspace_id AND fn::mt109_has_workspace_access(record::id(workspace_id), 'read', 'fs.read')) FOR create WHERE fn::mt120_entity_write(id, primary_source_id, workspace_id) OR (entity_kind = 'loom_block' AND primary_source_id = NONE AND fn::mt120_loom_block_access(entity_key, record::id(workspace_id), 'create', 'fs.write')) OR (entity_kind = 'loom_block' AND primary_source_id = NONE AND type::record('loom_blocks', entity_key).content_type = 'view_def' AND type::record('loom_blocks', entity_key).workspace_id = workspace_id AND fn::mt109_has_workspace_access(record::id(workspace_id), 'create', 'fs.write')) FOR update WHERE fn::mt120_entity_write(id, primary_source_id, workspace_id) FOR delete NONE;", "DEFINE TABLE OVERWRITE knowledge_entities SCHEMAFULL PERMISSIONS FOR select WHERE fn::mt120_entity_read(id) OR (entity_kind = 'loom_block' AND primary_source_id = NONE AND fn::mt120_loom_block_access(entity_key, record::id(workspace_id), 'read', 'fs.read')) FOR create WHERE fn::mt120_entity_write(id, primary_source_id, workspace_id) OR (entity_kind = 'loom_block' AND primary_source_id = NONE AND fn::mt120_loom_block_access(entity_key, record::id(workspace_id), 'create', 'fs.write')) FOR update WHERE fn::mt120_entity_write(id, primary_source_id, workspace_id) FOR delete NONE;"),
    ("DEFINE TABLE OVERWRITE loom_canvas_boards SCHEMAFULL\n    PERMISSIONS FOR select WHERE fn::mt120_loom_block_access(record::id(block_id), record::id(workspace_id), 'read', 'fs.read')\n                FOR create WHERE block_id.workspace_id = workspace_id AND block_id.content_type = 'canvas' AND fn::mt120_loom_block_access(record::id(block_id), record::id(workspace_id), 'create', 'fs.write')\n                FOR update WHERE block_id.workspace_id = workspace_id AND block_id.content_type = 'canvas' AND fn::mt120_loom_block_access(record::id(block_id), record::id(workspace_id), 'update', 'fs.write')\n                FOR delete WHERE block_id.workspace_id = workspace_id AND fn::mt120_loom_block_access(record::id(block_id), record::id(workspace_id), 'delete', 'fs.write');", "DEFINE TABLE OVERWRITE loom_canvas_boards SCHEMAFULL\n    PERMISSIONS FOR select WHERE fn::mt120_loom_block_access(record::id(block_id), record::id(workspace_id), 'read', 'fs.read')\n                FOR create WHERE block_id.workspace_id = workspace_id AND block_id.content_type = 'canvas' AND fn::mt120_loom_block_access(record::id(block_id), record::id(workspace_id), 'create', 'fs.write')\n                FOR update NONE\n                FOR delete WHERE block_id.workspace_id = workspace_id AND fn::mt120_loom_block_access(record::id(block_id), record::id(workspace_id), 'delete', 'fs.write');"),
    ("DEFINE TABLE OVERWRITE loom_canvas_placements SCHEMAFULL\n    PERMISSIONS FOR select WHERE (fn::mt120_loom_block_access(record::id(canvas_block_id), record::id(workspace_id), 'read', 'fs.read') OR fn::mt120_loom_block_access(record::id(canvas_block_id), record::id(workspace_id), 'update', 'fs.write')) AND (fn::mt120_loom_block_access(record::id(placed_block_id), record::id(workspace_id), 'read', 'fs.read') OR (placed_block_id.source_rich_document_id != NONE AND record::id(placed_block_id.source_rich_document_id) = record::id(placed_block_id) AND fn::mt120_document_access(record::id(placed_block_id), record::id(workspace_id), 'read', 'fs.read')))\n                FOR create WHERE fn::mt120_loom_block_access(record::id(canvas_block_id), record::id(workspace_id), 'update', 'fs.write') AND (fn::mt120_loom_block_access(record::id(placed_block_id), record::id(workspace_id), 'read', 'fs.read') OR (placed_block_id.source_rich_document_id != NONE AND record::id(placed_block_id.source_rich_document_id) = record::id(placed_block_id) AND fn::mt120_document_access(record::id(placed_block_id), record::id(workspace_id), 'read', 'fs.read')))\n                FOR update WHERE fn::mt120_loom_block_access(record::id(canvas_block_id), record::id(workspace_id), 'update', 'fs.write') AND (fn::mt120_loom_block_access(record::id(placed_block_id), record::id(workspace_id), 'read', 'fs.read') OR (placed_block_id.source_rich_document_id != NONE AND record::id(placed_block_id.source_rich_document_id) = record::id(placed_block_id) AND fn::mt120_document_access(record::id(placed_block_id), record::id(workspace_id), 'read', 'fs.read')))\n                FOR delete WHERE fn::mt120_workspace_delete(record::id(workspace_id)) OR (fn::mt120_loom_block_access(record::id(canvas_block_id), record::id(workspace_id), 'update', 'fs.write') AND (fn::mt120_loom_block_access(record::id(placed_block_id), record::id(workspace_id), 'read', 'fs.read') OR (placed_block_id.source_rich_document_id != NONE AND record::id(placed_block_id.source_rich_document_id) = record::id(placed_block_id) AND fn::mt120_document_access(record::id(placed_block_id), record::id(workspace_id), 'read', 'fs.read'))));", "DEFINE TABLE OVERWRITE loom_canvas_placements SCHEMAFULL\n    PERMISSIONS FOR select WHERE (fn::mt120_loom_block_access(record::id(canvas_block_id), record::id(workspace_id), 'read', 'fs.read') OR fn::mt120_loom_block_access(record::id(canvas_block_id), record::id(workspace_id), 'update', 'fs.write')) AND (fn::mt120_loom_block_access(record::id(placed_block_id), record::id(workspace_id), 'read', 'fs.read') OR (placed_block_id.source_rich_document_id != NONE AND record::id(placed_block_id.source_rich_document_id) = record::id(placed_block_id) AND fn::mt120_document_access(record::id(placed_block_id), record::id(workspace_id), 'read', 'fs.read')))\n                FOR create WHERE fn::mt120_loom_block_access(record::id(canvas_block_id), record::id(workspace_id), 'update', 'fs.write') AND (fn::mt120_loom_block_access(record::id(placed_block_id), record::id(workspace_id), 'read', 'fs.read') OR (placed_block_id.source_rich_document_id != NONE AND record::id(placed_block_id.source_rich_document_id) = record::id(placed_block_id) AND fn::mt120_document_access(record::id(placed_block_id), record::id(workspace_id), 'read', 'fs.read')))\n                FOR update NONE\n                FOR delete WHERE fn::mt120_workspace_delete(record::id(workspace_id)) OR (fn::mt120_loom_block_access(record::id(canvas_block_id), record::id(workspace_id), 'update', 'fs.write') AND (fn::mt120_loom_block_access(record::id(placed_block_id), record::id(workspace_id), 'read', 'fs.read') OR (placed_block_id.source_rich_document_id != NONE AND record::id(placed_block_id.source_rich_document_id) = record::id(placed_block_id) AND fn::mt120_document_access(record::id(placed_block_id), record::id(workspace_id), 'read', 'fs.read'))));"),
    ("DEFINE TABLE OVERWRITE loom_canvas_visual_edges SCHEMAFULL\n    PERMISSIONS FOR select WHERE fn::mt120_loom_block_access(record::id(canvas_block_id), record::id(workspace_id), 'read', 'fs.read') AND fn::mt120_loom_endpoint_access(from_placement_id.placed_block_id, record::id(workspace_id), 'read', 'fs.read') AND fn::mt120_loom_endpoint_access(to_placement_id.placed_block_id, record::id(workspace_id), 'read', 'fs.read')\n                FOR create WHERE fn::mt120_loom_block_access(record::id(canvas_block_id), record::id(workspace_id), 'update', 'fs.write') AND from_placement_id.canvas_block_id = canvas_block_id AND to_placement_id.canvas_block_id = canvas_block_id AND from_placement_id.workspace_id = workspace_id AND to_placement_id.workspace_id = workspace_id AND fn::mt120_loom_endpoint_access(from_placement_id.placed_block_id, record::id(workspace_id), 'read', 'fs.read') AND fn::mt120_loom_endpoint_access(to_placement_id.placed_block_id, record::id(workspace_id), 'read', 'fs.read')\n                FOR update NONE\n                FOR delete WHERE fn::mt120_loom_block_access(record::id(canvas_block_id), record::id(workspace_id), 'update', 'fs.write') OR fn::mt120_workspace_delete(record::id(workspace_id));", "DEFINE TABLE OVERWRITE loom_canvas_visual_edges SCHEMAFULL\n    PERMISSIONS FOR select WHERE fn::mt120_loom_block_access(record::id(canvas_block_id), record::id(workspace_id), 'read', 'fs.read') AND fn::mt120_loom_block_access(record::id(from_placement_id.placed_block_id), record::id(workspace_id), 'read', 'fs.read') AND fn::mt120_loom_block_access(record::id(to_placement_id.placed_block_id), record::id(workspace_id), 'read', 'fs.read')\n                FOR create, update NONE\n                FOR delete WHERE fn::mt120_loom_block_access(record::id(canvas_block_id), record::id(workspace_id), 'update', 'fs.write') OR fn::mt120_workspace_delete(record::id(workspace_id));"),
    ("DEFINE TABLE OVERWRITE loom_block_view_fr_outbox SCHEMAFULL PERMISSIONS FOR select WHERE block_id.workspace_id = workspace_id AND fn::mt109_has_workspace_access(record::id(workspace_id), 'read', 'fs.read') FOR create WHERE block_id.workspace_id = workspace_id AND block_id.content_type = 'view_def' AND (fn::mt109_has_workspace_access(record::id(workspace_id), 'create', 'fs.write') OR fn::mt109_has_workspace_access(record::id(workspace_id), 'update', 'fs.write')) FOR update WHERE block_id.workspace_id = workspace_id AND block_id.content_type = 'view_def' AND fn::mt109_has_workspace_access(record::id(workspace_id), 'update', 'fs.write') FOR delete WHERE fn::mt120_workspace_delete(record::id(workspace_id));", "DEFINE TABLE OVERWRITE loom_block_view_fr_outbox SCHEMAFULL PERMISSIONS FOR select, create, update NONE FOR delete WHERE fn::mt120_workspace_delete(record::id(workspace_id));"),
    ("DEFINE TABLE OVERWRITE assets SCHEMAFULL PERMISSIONS FOR select WHERE fn::mt109_has_workspace_access(record::id(workspace_id), 'read', 'fs.read') FOR create WHERE fn::mt109_has_workspace_access(record::id(workspace_id), 'create', 'fs.write') FOR update WHERE fn::mt109_has_workspace_access(record::id(workspace_id), 'update', 'fs.write') FOR delete WHERE fn::mt109_has_workspace_access(record::id(workspace_id), 'delete', 'fs.write') OR fn::mt120_workspace_delete(record::id(workspace_id));", "DEFINE TABLE OVERWRITE assets SCHEMAFULL PERMISSIONS NONE;"),
    ("DEFINE TABLE OVERWRITE media_asset_tiers SCHEMAFULL PERMISSIONS FOR select WHERE asset_id.workspace_id = workspace_id AND fn::mt109_has_workspace_access(record::id(workspace_id), 'read', 'fs.read') FOR create WHERE asset_id.workspace_id = workspace_id AND fn::mt109_has_workspace_access(record::id(workspace_id), 'update', 'fs.write') FOR update WHERE asset_id.workspace_id = workspace_id AND fn::mt109_has_workspace_access(record::id(workspace_id), 'update', 'fs.write') FOR delete WHERE fn::mt109_has_workspace_access(record::id(workspace_id), 'delete', 'fs.write') OR fn::mt120_workspace_delete(record::id(workspace_id));", "DEFINE TABLE OVERWRITE media_asset_tiers SCHEMAFULL PERMISSIONS NONE;"),
    ("DEFINE TABLE OVERWRITE loom_folders SCHEMAFULL PERMISSIONS FOR select WHERE fn::mt109_has_workspace_access(record::id(workspace_id), 'read', 'fs.read') FOR create WHERE (parent_folder_id = NONE OR parent_folder_id.workspace_id = workspace_id) AND fn::mt109_has_workspace_access(record::id(workspace_id), 'create', 'fs.write') FOR update WHERE (parent_folder_id = NONE OR parent_folder_id.workspace_id = workspace_id) AND fn::mt109_has_workspace_access(record::id(workspace_id), 'update', 'fs.write') FOR delete WHERE fn::mt109_has_workspace_access(record::id(workspace_id), 'delete', 'fs.write') OR fn::mt120_workspace_delete(record::id(workspace_id));", "DEFINE TABLE OVERWRITE loom_folders SCHEMAFULL PERMISSIONS NONE;"),
    ("DEFINE TABLE OVERWRITE loom_folder_members SCHEMAFULL PERMISSIONS FOR select WHERE folder_id.workspace_id = workspace_id AND fn::mt109_has_workspace_access(record::id(workspace_id), 'read', 'fs.read') AND fn::mt120_loom_endpoint_access(block_id, record::id(workspace_id), 'read', 'fs.read') FOR create WHERE folder_id.workspace_id = workspace_id AND fn::mt109_has_workspace_access(record::id(workspace_id), 'update', 'fs.write') AND fn::mt120_loom_endpoint_access(block_id, record::id(workspace_id), 'read', 'fs.read') FOR update WHERE folder_id.workspace_id = workspace_id AND fn::mt109_has_workspace_access(record::id(workspace_id), 'update', 'fs.write') AND fn::mt120_loom_endpoint_access(block_id, record::id(workspace_id), 'read', 'fs.read') FOR delete WHERE (folder_id.workspace_id = workspace_id AND fn::mt109_has_workspace_access(record::id(workspace_id), 'update', 'fs.write')) OR fn::mt120_workspace_delete(record::id(workspace_id));", "DEFINE TABLE OVERWRITE loom_folder_members SCHEMAFULL PERMISSIONS NONE;"),
    ("DEFINE TABLE OVERWRITE storage_graph_anchors SCHEMAFULL PERMISSIONS FOR select WHERE graph_kind = 'loom_folder_tree' AND anchor_key = 'loom_folder_tree|' + scope_key AND record::id(id) = anchor_key AND fn::mt109_has_workspace_access(scope_key, 'read', 'fs.read') FOR create, update WHERE graph_kind = 'loom_folder_tree' AND anchor_key = 'loom_folder_tree|' + scope_key AND record::id(id) = anchor_key AND fn::mt109_has_workspace_access(scope_key, 'update', 'fs.write') FOR delete NONE;", "DEFINE TABLE OVERWRITE storage_graph_anchors SCHEMAFULL PERMISSIONS NONE;"),
    ("DEFINE TABLE OVERWRITE knowledge_wiki_projections SCHEMAFULL PERMISSIONS FOR select WHERE fn::mt109_has_workspace_access(record::id(workspace_id), 'read', 'fs.read') FOR create WHERE fn::mt109_has_workspace_access(record::id(workspace_id), 'create', 'fs.write') FOR update WHERE fn::mt109_has_workspace_access(record::id(workspace_id), 'update', 'fs.write') FOR delete WHERE fn::mt109_has_workspace_access(record::id(workspace_id), 'delete', 'fs.write') OR fn::mt120_workspace_delete(record::id(workspace_id));", "DEFINE TABLE OVERWRITE knowledge_wiki_projections SCHEMAFULL PERMISSIONS NONE;"),
    ("DEFINE TABLE OVERWRITE loom_wiki_overlays SCHEMAFULL PERMISSIONS FOR select WHERE fn::mt109_has_workspace_access(record::id(workspace_id), 'read', 'fs.read') FOR create WHERE fn::mt109_has_workspace_access(record::id(workspace_id), 'create', 'fs.write') FOR update WHERE fn::mt109_has_workspace_access(record::id(workspace_id), 'update', 'fs.write') FOR delete WHERE fn::mt109_has_workspace_access(record::id(workspace_id), 'delete', 'fs.write') OR fn::mt120_workspace_delete(record::id(workspace_id));", "DEFINE TABLE OVERWRITE loom_wiki_overlays SCHEMAFULL PERMISSIONS NONE;"),
    ("DEFINE TABLE OVERWRITE knowledge_quick_switcher_recents SCHEMAFULL PERMISSIONS FOR select WHERE fn::mt109_has_workspace_access(record::id(workspace_id), 'read', 'fs.read') FOR create WHERE fn::mt109_has_workspace_access(record::id(workspace_id), 'update', 'fs.write') FOR update WHERE fn::mt109_has_workspace_access(record::id(workspace_id), 'update', 'fs.write') FOR delete WHERE fn::mt120_workspace_delete(record::id(workspace_id));", "DEFINE TABLE OVERWRITE knowledge_quick_switcher_recents SCHEMAFULL PERMISSIONS NONE;"),
    ("DEFINE TABLE OVERWRITE loom_collections SCHEMAFULL PERMISSIONS FOR select WHERE fn::mt109_has_workspace_access(record::id(workspace_id), 'read', 'fs.read') FOR create WHERE fn::mt109_has_workspace_access(record::id(workspace_id), 'create', 'fs.write') FOR update WHERE fn::mt109_has_workspace_access(record::id(workspace_id), 'update', 'fs.write') FOR delete WHERE fn::mt109_has_workspace_access(record::id(workspace_id), 'delete', 'fs.write') OR fn::mt120_workspace_delete(record::id(workspace_id));", "DEFINE TABLE OVERWRITE loom_collections SCHEMAFULL PERMISSIONS NONE;"),
    ("DEFINE TABLE OVERWRITE loom_collection_members SCHEMAFULL PERMISSIONS FOR select WHERE fn::mt109_has_workspace_access(record::id(collection_id.workspace_id), 'read', 'fs.read') FOR create, update WHERE asset_id.workspace_id = collection_id.workspace_id AND fn::mt109_has_workspace_access(record::id(collection_id.workspace_id), 'update', 'fs.write') FOR delete WHERE fn::mt109_has_workspace_access(record::id(collection_id.workspace_id), 'update', 'fs.write') OR fn::mt120_workspace_delete(record::id(collection_id.workspace_id));", "DEFINE TABLE OVERWRITE loom_collection_members SCHEMAFULL PERMISSIONS NONE;"),
    ("DEFINE TABLE OVERWRITE loom_ai_suggestions SCHEMAFULL PERMISSIONS FOR select WHERE fn::mt109_has_workspace_access(record::id(workspace_id), 'read', 'fs.read') FOR create WHERE fn::mt109_has_workspace_access(record::id(workspace_id), 'create', 'fs.write') FOR update WHERE fn::mt109_has_workspace_access(record::id(workspace_id), 'update', 'fs.write') FOR delete WHERE fn::mt120_workspace_delete(record::id(workspace_id));", "DEFINE TABLE OVERWRITE loom_ai_suggestions SCHEMAFULL PERMISSIONS NONE;"),
];

/// Revision-160 standalone LoomBlock update authorization, reverted after the MT-109 C3/C2 deltas
/// (text in its post-01df5ebf form; [`MT109_C1_SCHEMA_DELTAS`] then restores the revision-159 form).
const STANDALONE_LOOM_UPDATE_SCHEMA_DELTAS: [(&str, &str); 2] = [
    (
        "FOR update WHERE (source_rich_document_id != NONE AND record::id(source_rich_document_id) = block_id AND content_type = 'note' AND source_rich_document_id.workspace_id = workspace_id AND source_rich_document_id.deleted_at = NONE AND content_hash = source_rich_document_id.content_sha256 AND fn::mt120_document_access(record::id(source_rich_document_id), record::id(workspace_id), 'update', 'fs.write')) OR (source_rich_document_id = NONE AND content_type IN ['note','file','annotated_file','tag_hub','journal','canvas'] AND fn::mt120_loom_block_access(block_id, record::id(workspace_id), 'update', 'fs.write'))",
        "FOR update WHERE source_rich_document_id != NONE AND record::id(source_rich_document_id) = block_id AND content_type = 'note' AND source_rich_document_id.workspace_id = workspace_id AND source_rich_document_id.deleted_at = NONE AND content_hash = source_rich_document_id.content_sha256 AND fn::mt120_document_access(record::id(source_rich_document_id), record::id(workspace_id), 'update', 'fs.write')",
    ),
    (
        "        IF $event = 'UPDATE' AND $before.source_rich_document_id = NONE\n            AND ($after.source_rich_document_id != NONE OR $after.workspace_id != $before.workspace_id\n                OR $after.content_type != $before.content_type) {\n            THROW 'HSK-403-PROTECTED-RESOURCE';\n        };\n",
        "",
    ),
];
#[cfg(test)]
fn restore_pre_standalone_loom_update_schema(mut source: String) -> String {
    for (current, previous) in MT109_C3_SCHEMA_DELTAS {
        source = source.replace(current, previous);
    }
    for (current, previous) in MT109_C2_SCHEMA_DELTAS {
        source = source.replace(current, previous);
    }
    for (current, previous) in STANDALONE_LOOM_UPDATE_SCHEMA_DELTAS {
        source = source.replace(current, previous);
    }
    let (start, end) = schema_event_definition_bounds_until_next_table(
        &source,
        "mt109_workspace_reconciliation_queue",
    );
    source.replace_range(start..end, "");
    source
}
/// MT-109 C1 schema deltas committed after the revision-158/159 pins (456dc63a): edb54c0e
/// (workspace reconciliation bootstrap guard), c792b8a2 (Canvas board owner delete), 64ca7c82
/// (record-user-only reconciliation event) and 01df5ebf (every spec Loom content type). Hunks inside
/// the dropped `mt109_workspace_reconciliation_queue` event need no pair. Applied last, on the
/// output of [`restore_pre_standalone_loom_update_schema`], so the revision-157 chain in
/// [`pre_account_setup_schema`] keeps operating on the post-01df5ebf text it was written for.
const MT109_C1_SCHEMA_DELTAS: &[(&str, &str)] = &[
    ("    PERMISSIONS FOR select WHERE ((source_rich_document_id = NONE AND fn::mt109_source_read('loom_block', block_id, record::id(workspace_id), 'workspace', record::id(workspace_id))) OR (source_rich_document_id != NONE AND record::id(source_rich_document_id) = block_id AND content_type = 'note' AND fn::mt109_source_read('loom_block', block_id, record::id(workspace_id), 'rich_document', record::id(source_rich_document_id)) AND fn::mt109_source_read('rich_document', record::id(source_rich_document_id), record::id(workspace_id), 'workspace', record::id(workspace_id)) AND source_rich_document_id.workspace_id = workspace_id AND source_rich_document_id.deleted_at = NONE AND content_hash = source_rich_document_id.content_sha256)) OR (source_rich_document_id != NONE AND record::id(source_rich_document_id) = block_id AND content_type = 'note' AND source_rich_document_id.workspace_id = workspace_id AND source_rich_document_id.deleted_at = NONE AND content_hash = source_rich_document_id.content_sha256 AND fn::mt120_document_access(record::id(source_rich_document_id), record::id(workspace_id), 'read', 'fs.read')) OR (source_rich_document_id = NONE AND fn::mt120_loom_block_access(block_id, record::id(workspace_id), 'read', 'fs.read')) OR (source_rich_document_id != NONE AND record::id(source_rich_document_id) = block_id AND content_type = 'note' AND source_rich_document_id.workspace_id = workspace_id AND source_rich_document_id.deleted_at = NONE AND fn::mt120_document_access(record::id(source_rich_document_id), record::id(workspace_id), 'update', 'fs.write'))\n                FOR create WHERE (source_rich_document_id != NONE AND record::id(source_rich_document_id) = block_id AND content_type = 'note' AND source_rich_document_id.workspace_id = workspace_id AND source_rich_document_id.deleted_at = NONE AND content_hash = source_rich_document_id.content_sha256 AND (fn::mt120_document_access(record::id(source_rich_document_id), record::id(workspace_id), 'create', 'fs.write') OR fn::mt120_document_access(record::id(source_rich_document_id), record::id(workspace_id), 'update', 'fs.write'))) OR (source_rich_document_id = NONE AND content_type IN ['note','file','annotated_file','tag_hub','journal','canvas'] AND $auth != NONE AND created_in_session_id = $auth.id AND fn::mt109_has_workspace_access(record::id(workspace_id), 'create', 'fs.write'))\n                FOR update WHERE source_rich_document_id != NONE AND record::id(source_rich_document_id) = block_id AND content_type = 'note' AND source_rich_document_id.workspace_id = workspace_id AND source_rich_document_id.deleted_at = NONE AND content_hash = source_rich_document_id.content_sha256 AND fn::mt120_document_access(record::id(source_rich_document_id), record::id(workspace_id), 'update', 'fs.write') OR (source_rich_document_id != NONE AND record::id(source_rich_document_id) = block_id AND content_type = 'note' AND source_rich_document_id.workspace_id = workspace_id AND source_rich_document_id.deleted_at = NONE AND fn::mt120_document_access(record::id(source_rich_document_id), record::id(workspace_id), 'update', 'fs.write'))\n", "    PERMISSIONS FOR select WHERE ((source_rich_document_id = NONE AND fn::mt109_source_read('loom_block', block_id, record::id(workspace_id), 'workspace', record::id(workspace_id))) OR (source_rich_document_id != NONE AND record::id(source_rich_document_id) = block_id AND content_type = 'note' AND fn::mt109_source_read('loom_block', block_id, record::id(workspace_id), 'rich_document', record::id(source_rich_document_id)) AND fn::mt109_source_read('rich_document', record::id(source_rich_document_id), record::id(workspace_id), 'workspace', record::id(workspace_id)) AND source_rich_document_id.workspace_id = workspace_id AND source_rich_document_id.deleted_at = NONE AND content_hash = source_rich_document_id.content_sha256)) OR (source_rich_document_id != NONE AND record::id(source_rich_document_id) = block_id AND content_type = 'note' AND source_rich_document_id.workspace_id = workspace_id AND source_rich_document_id.deleted_at = NONE AND content_hash = source_rich_document_id.content_sha256 AND fn::mt120_document_access(record::id(source_rich_document_id), record::id(workspace_id), 'read', 'fs.read')) OR (source_rich_document_id = NONE AND fn::mt120_loom_block_access(block_id, record::id(workspace_id), 'read', 'fs.read'))\n                FOR create WHERE (source_rich_document_id != NONE AND record::id(source_rich_document_id) = block_id AND content_type = 'note' AND source_rich_document_id.workspace_id = workspace_id AND source_rich_document_id.deleted_at = NONE AND content_hash = source_rich_document_id.content_sha256 AND (fn::mt120_document_access(record::id(source_rich_document_id), record::id(workspace_id), 'create', 'fs.write') OR fn::mt120_document_access(record::id(source_rich_document_id), record::id(workspace_id), 'update', 'fs.write'))) OR (source_rich_document_id = NONE AND content_type IN ['note','canvas'] AND $auth != NONE AND created_in_session_id = $auth.id AND fn::mt109_has_workspace_access(record::id(workspace_id), 'create', 'fs.write'))\n                FOR update WHERE source_rich_document_id != NONE AND record::id(source_rich_document_id) = block_id AND content_type = 'note' AND source_rich_document_id.workspace_id = workspace_id AND source_rich_document_id.deleted_at = NONE AND content_hash = source_rich_document_id.content_sha256 AND fn::mt120_document_access(record::id(source_rich_document_id), record::id(workspace_id), 'update', 'fs.write')\n"),
    ("                FOR delete WHERE block_id.workspace_id = workspace_id AND fn::mt120_loom_block_access(record::id(block_id), record::id(workspace_id), 'delete', 'fs.write');\n", "                FOR delete NONE;\n"),
    ("    PERMISSIONS FOR select WHERE (fn::mt120_loom_block_access(record::id(canvas_block_id), record::id(workspace_id), 'read', 'fs.read') OR fn::mt120_loom_block_access(record::id(canvas_block_id), record::id(workspace_id), 'update', 'fs.write')) AND (fn::mt120_loom_block_access(record::id(placed_block_id), record::id(workspace_id), 'read', 'fs.read') OR (placed_block_id.source_rich_document_id != NONE AND record::id(placed_block_id.source_rich_document_id) = record::id(placed_block_id) AND fn::mt120_document_access(record::id(placed_block_id), record::id(workspace_id), 'read', 'fs.read')))\n                FOR create WHERE fn::mt120_loom_block_access(record::id(canvas_block_id), record::id(workspace_id), 'update', 'fs.write') AND (fn::mt120_loom_block_access(record::id(placed_block_id), record::id(workspace_id), 'read', 'fs.read') OR (placed_block_id.source_rich_document_id != NONE AND record::id(placed_block_id.source_rich_document_id) = record::id(placed_block_id) AND fn::mt120_document_access(record::id(placed_block_id), record::id(workspace_id), 'read', 'fs.read')))\n", "    PERMISSIONS FOR select WHERE (fn::mt120_loom_block_access(record::id(canvas_block_id), record::id(workspace_id), 'read', 'fs.read') OR fn::mt120_loom_block_access(record::id(canvas_block_id), record::id(workspace_id), 'update', 'fs.write')) AND fn::mt120_loom_block_access(record::id(placed_block_id), record::id(workspace_id), 'read', 'fs.read')\n                FOR create WHERE fn::mt120_loom_block_access(record::id(canvas_block_id), record::id(workspace_id), 'update', 'fs.write') AND fn::mt120_loom_block_access(record::id(placed_block_id), record::id(workspace_id), 'read', 'fs.read')\n"),
    ("                FOR delete WHERE fn::mt120_workspace_delete(record::id(workspace_id)) OR (fn::mt120_loom_block_access(record::id(canvas_block_id), record::id(workspace_id), 'update', 'fs.write') AND (fn::mt120_loom_block_access(record::id(placed_block_id), record::id(workspace_id), 'read', 'fs.read') OR (placed_block_id.source_rich_document_id != NONE AND record::id(placed_block_id.source_rich_document_id) = record::id(placed_block_id) AND fn::mt120_document_access(record::id(placed_block_id), record::id(workspace_id), 'read', 'fs.read'))));\n", "                FOR delete WHERE fn::mt120_workspace_delete(record::id(workspace_id)) OR (fn::mt120_loom_block_access(record::id(canvas_block_id), record::id(workspace_id), 'update', 'fs.write') AND fn::mt120_loom_block_access(record::id(placed_block_id), record::id(workspace_id), 'read', 'fs.read'));\n"),
    ("              AND $payload.block_id != NONE AND $payload.content_type IN ['note', 'file', 'annotated_file', 'tag_hub', 'journal', 'canvas']\n", "              AND $payload.block_id != NONE AND $payload.content_type IN ['note', 'canvas']\n"),
    ("            AND $block.content_type IN ['note', 'file', 'annotated_file', 'tag_hub', 'journal', 'canvas']\n", "            AND $block.content_type IN ['note', 'canvas']\n"),
];
/// MT-154 (C4) authority block appended to schema.surql; stripped whole by its markers.
const MT154_AUTHORITY_BLOCK_BEGIN: &str = "\n-- MT154_AUTHORITY_BEGIN\n";
const MT154_AUTHORITY_BLOCK_END: &str = "-- MT154_AUTHORITY_END\n";
/// MT-154 (C4) schema deltas outside the MT154 authority block, as (current, previous) pairs
/// against schema.surql at 0cfbff64. Reverted FIRST (before C3, C2, C1). Append new pairs here;
/// `Handshake_Artifacts/WP-KERNEL-012/MT-154/kb-c4/scripts/lineage_check.js` derives and verifies
/// them.
const MT154_SCHEMA_DELTAS: &[(&str, &str)] = &[
    ("-- MT-154 (02-system-architecture.md:2758/2773/2776, LM-RLS-001/002): legacy canvases are workspace-scoped;\n-- viewers read, members create/edit, admins delete; an owner's workspace delete may remove them.\nDEFINE TABLE OVERWRITE canvases SCHEMAFULL\n    PERMISSIONS FOR select WHERE fn::mt109_has_workspace_access(record::id(workspace_id), 'read', 'fs.read')\n                FOR create WHERE fn::mt109_has_workspace_access(record::id(workspace_id), 'create', 'fs.write')\n                FOR update WHERE fn::mt109_has_workspace_access(record::id(workspace_id), 'update', 'fs.write')\n                FOR delete WHERE fn::mt109_has_workspace_access(record::id(workspace_id), 'delete', 'fs.write')\n                    OR fn::mt120_workspace_delete(record::id(workspace_id));\n", "DEFINE TABLE OVERWRITE canvases SCHEMAFULL PERMISSIONS NONE;\n"),
    ("-- MT-154: canvas children inherit their canvas's workspace authority. Replacing a canvas graph is an edit\n-- of the canvas (PUT /canvases/:id authorizes update), so child rows are removable by update or delete.\nDEFINE TABLE OVERWRITE canvas_nodes SCHEMAFULL\n    PERMISSIONS FOR select WHERE fn::mt109_has_workspace_access(record::id(canvas_id.workspace_id), 'read', 'fs.read')\n                FOR create WHERE fn::mt109_has_workspace_access(record::id(canvas_id.workspace_id), 'update', 'fs.write')\n                FOR update WHERE fn::mt109_has_workspace_access(record::id(canvas_id.workspace_id), 'update', 'fs.write')\n                FOR delete WHERE fn::mt109_has_workspace_access(record::id(canvas_id.workspace_id), 'update', 'fs.write')\n                    OR fn::mt109_has_workspace_access(record::id(canvas_id.workspace_id), 'delete', 'fs.write')\n                    OR fn::mt120_workspace_delete(record::id(canvas_id.workspace_id));\n", "DEFINE TABLE OVERWRITE canvas_nodes SCHEMAFULL PERMISSIONS NONE;\n"),
    ("DEFINE TABLE OVERWRITE canvas_edges SCHEMAFULL\n    PERMISSIONS FOR select WHERE fn::mt109_has_workspace_access(record::id(canvas_id.workspace_id), 'read', 'fs.read')\n                FOR create WHERE fn::mt109_has_workspace_access(record::id(canvas_id.workspace_id), 'update', 'fs.write')\n                FOR update WHERE fn::mt109_has_workspace_access(record::id(canvas_id.workspace_id), 'update', 'fs.write')\n                FOR delete WHERE fn::mt109_has_workspace_access(record::id(canvas_id.workspace_id), 'update', 'fs.write')\n                    OR fn::mt109_has_workspace_access(record::id(canvas_id.workspace_id), 'delete', 'fs.write')\n                    OR fn::mt120_workspace_delete(record::id(canvas_id.workspace_id));\n", "DEFINE TABLE OVERWRITE canvas_edges SCHEMAFULL PERMISSIONS NONE;\n"),
    ("DEFINE TABLE OVERWRITE ai_jobs SCHEMAFULL PERMISSIONS FOR select WHERE fn::mt154_job_access(entity_refs, job_inputs, 'read', 'fs.read') OR fn::mt154_job_access(entity_refs, job_inputs, 'read', 'fr.read') FOR create WHERE fn::mt154_job_access(entity_refs, job_inputs, 'create', 'fs.write') OR (job_kind = 'debug_bundle_export' AND fn::mt154_job_access(entity_refs, job_inputs, 'read', 'fr.read')) FOR update WHERE fn::mt154_job_access(entity_refs, job_inputs, 'update', 'fs.write') OR (job_kind = 'debug_bundle_export' AND fn::mt154_job_access(entity_refs, job_inputs, 'read', 'fr.read')) FOR delete NONE;\n", "DEFINE TABLE OVERWRITE ai_jobs SCHEMAFULL PERMISSIONS NONE;\n"),
    ("-- MT-159 (02-system-architecture.md:2773; every job kind under the account session): a job-bound model\n-- session and its checkpoints/messages follow the job workspace grant (fn::mt154_job_access, as\n-- workflow_runs); unbound (job_id NONE) sessions stay root-only.\nDEFINE TABLE OVERWRITE model_sessions SCHEMAFULL PERMISSIONS FOR select WHERE fn::mt154_job_access(job_id.entity_refs, job_id.job_inputs, 'read', 'fs.read') OR fn::mt154_job_access(job_id.entity_refs, job_id.job_inputs, 'read', 'fr.read') FOR create, update WHERE fn::mt154_job_access(job_id.entity_refs, job_id.job_inputs, 'update', 'fs.write') FOR delete NONE;\n", "DEFINE TABLE OVERWRITE model_sessions SCHEMAFULL PERMISSIONS NONE;\n"),
    ("DEFINE TABLE OVERWRITE model_session_checkpoints SCHEMAFULL PERMISSIONS FOR select WHERE fn::mt154_job_access(session_id.job_id.entity_refs, session_id.job_id.job_inputs, 'read', 'fs.read') OR fn::mt154_job_access(session_id.job_id.entity_refs, session_id.job_id.job_inputs, 'read', 'fr.read') FOR create, update WHERE fn::mt154_job_access(session_id.job_id.entity_refs, session_id.job_id.job_inputs, 'update', 'fs.write') FOR delete NONE;\n", "DEFINE TABLE OVERWRITE model_session_checkpoints SCHEMAFULL PERMISSIONS NONE;\n"),
    ("DEFINE TABLE OVERWRITE model_session_messages SCHEMAFULL PERMISSIONS FOR select WHERE fn::mt154_job_access(session_id.job_id.entity_refs, session_id.job_id.job_inputs, 'read', 'fs.read') OR fn::mt154_job_access(session_id.job_id.entity_refs, session_id.job_id.job_inputs, 'read', 'fr.read') FOR create, update WHERE fn::mt154_job_access(session_id.job_id.entity_refs, session_id.job_id.job_inputs, 'update', 'fs.write') FOR delete NONE;\n", "DEFINE TABLE OVERWRITE model_session_messages SCHEMAFULL PERMISSIONS NONE;\n"),
    ("DEFINE TABLE OVERWRITE workflow_runs SCHEMAFULL PERMISSIONS FOR select WHERE fn::mt154_job_access(job_id.entity_refs, job_id.job_inputs, 'read', 'fs.read') OR fn::mt154_job_access(job_id.entity_refs, job_id.job_inputs, 'read', 'fr.read') FOR create, update WHERE fn::mt154_job_access(job_id.entity_refs, job_id.job_inputs, 'update', 'fs.write') OR (job_id.job_kind = 'debug_bundle_export' AND fn::mt154_job_access(job_id.entity_refs, job_id.job_inputs, 'read', 'fr.read')) FOR delete NONE;\n", "DEFINE TABLE OVERWRITE workflow_runs SCHEMAFULL PERMISSIONS NONE;\n"),
    ("DEFINE TABLE OVERWRITE workflow_node_executions SCHEMAFULL PERMISSIONS FOR select WHERE fn::mt154_job_access(workflow_run_id.job_id.entity_refs, workflow_run_id.job_id.job_inputs, 'read', 'fs.read') OR fn::mt154_job_access(workflow_run_id.job_id.entity_refs, workflow_run_id.job_id.job_inputs, 'read', 'fr.read') FOR create, update WHERE fn::mt154_job_access(workflow_run_id.job_id.entity_refs, workflow_run_id.job_id.job_inputs, 'update', 'fs.write') OR (workflow_run_id.job_id.job_kind = 'debug_bundle_export' AND fn::mt154_job_access(workflow_run_id.job_id.entity_refs, workflow_run_id.job_id.job_inputs, 'read', 'fr.read')) FOR delete NONE;\n", "DEFINE TABLE OVERWRITE workflow_node_executions SCHEMAFULL PERMISSIONS NONE;\n"),
    ("DEFINE FIELD OVERWRITE block_id ON TABLE loom_blocks TYPE string ASSERT $value = record::id($this.id) AND ($before = NONE OR fn::mt153_loom_identity_unchanged($this));\n", "DEFINE FIELD OVERWRITE block_id ON TABLE loom_blocks TYPE string ASSERT $value = record::id($this.id);\n"),
    ("-- MT-154 (02-system-architecture.md:2758/2773/2776, LM-RLS-001/002): calendar rows are workspace-scoped;\n-- viewers read, members create/edit, admins delete; an owner's workspace delete may remove them.\nDEFINE TABLE OVERWRITE calendar_sources SCHEMAFULL\n    PERMISSIONS FOR select WHERE fn::mt109_has_workspace_access(record::id(workspace_id), 'read', 'fs.read')\n                FOR create WHERE fn::mt109_has_workspace_access(record::id(workspace_id), 'create', 'fs.write')\n                FOR update WHERE fn::mt109_has_workspace_access(record::id(workspace_id), 'update', 'fs.write')\n                FOR delete WHERE fn::mt109_has_workspace_access(record::id(workspace_id), 'delete', 'fs.write')\n                    OR fn::mt120_workspace_delete(record::id(workspace_id));\n", "DEFINE TABLE OVERWRITE calendar_sources SCHEMAFULL PERMISSIONS NONE;\n"),
    ("DEFINE TABLE OVERWRITE calendar_events SCHEMAFULL\n    PERMISSIONS FOR select WHERE fn::mt109_has_workspace_access(record::id(workspace_id), 'read', 'fs.read')\n                FOR create WHERE fn::mt109_has_workspace_access(record::id(workspace_id), 'create', 'fs.write')\n                FOR update WHERE fn::mt109_has_workspace_access(record::id(workspace_id), 'update', 'fs.write')\n                FOR delete WHERE fn::mt109_has_workspace_access(record::id(workspace_id), 'delete', 'fs.write')\n                    OR fn::mt120_workspace_delete(record::id(workspace_id));\n", "DEFINE TABLE OVERWRITE calendar_events SCHEMAFULL PERMISSIONS NONE;\n"),
    ("-- MT-154 (D-154-3): account-owned; owner_account_id stamped from $auth at create (NONE for root/system\n-- writers, which a record user never sees).\nDEFINE TABLE OVERWRITE work_packets SCHEMAFULL PERMISSIONS FOR select WHERE fn::mt154_account_row(owner_account_id, 'fs.read') FOR create WHERE fn::mt154_account_row(owner_account_id, 'fs.write') AND fn::mt159_locus_key_is_own(id) FOR update, delete WHERE fn::mt154_account_row(owner_account_id, 'fs.write');\nDEFINE FIELD OVERWRITE owner_account_id ON TABLE work_packets TYPE option<record<local_accounts>> DEFAULT $auth.account_id READONLY;\nDEFINE FIELD OVERWRITE wp_id ON TABLE work_packets TYPE string ASSERT $value = fn::mt159_locus_key_id(record::id($this.id));\n", "DEFINE TABLE OVERWRITE work_packets SCHEMAFULL PERMISSIONS NONE;\nDEFINE FIELD OVERWRITE wp_id ON TABLE work_packets TYPE string ASSERT $value = record::id($this.id);\n"),
    ("DEFINE INDEX OVERWRITE pk_work_packets ON TABLE work_packets FIELDS owner_account_id, wp_id UNIQUE;\n", "DEFINE INDEX OVERWRITE pk_work_packets ON TABLE work_packets FIELDS wp_id UNIQUE;\n"),
    ("DEFINE TABLE OVERWRITE dependencies SCHEMAFULL PERMISSIONS FOR select WHERE fn::mt154_account_row(owner_account_id, 'fs.read') FOR create WHERE fn::mt154_account_row(owner_account_id, 'fs.write') AND from_wp_id.owner_account_id = owner_account_id AND to_wp_id.owner_account_id = owner_account_id AND fn::mt159_locus_key_is_own(id) FOR update WHERE fn::mt154_account_row(owner_account_id, 'fs.write') AND from_wp_id.owner_account_id = owner_account_id AND to_wp_id.owner_account_id = owner_account_id FOR delete WHERE fn::mt154_account_row(owner_account_id, 'fs.write');\nDEFINE FIELD OVERWRITE owner_account_id ON TABLE dependencies TYPE option<record<local_accounts>> DEFAULT $auth.account_id READONLY;\nDEFINE FIELD OVERWRITE dependency_id ON TABLE dependencies TYPE string ASSERT $value = fn::mt159_locus_key_id(record::id($this.id)) AND string::trim($value) != '';\n", "DEFINE TABLE OVERWRITE dependencies SCHEMAFULL PERMISSIONS NONE;\nDEFINE FIELD OVERWRITE dependency_id ON TABLE dependencies TYPE string ASSERT $value = record::id($this.id) AND string::trim($value) != '';\n"),
    ("DEFINE INDEX OVERWRITE pk_dependencies ON TABLE dependencies FIELDS owner_account_id, dependency_id UNIQUE;\n", "DEFINE INDEX OVERWRITE pk_dependencies ON TABLE dependencies FIELDS dependency_id UNIQUE;\n"),
    ("DEFINE TABLE OVERWRITE micro_tasks SCHEMAFULL PERMISSIONS FOR select WHERE fn::mt154_account_row(owner_account_id, 'fs.read') FOR create WHERE fn::mt154_account_row(owner_account_id, 'fs.write') AND fn::mt159_locus_key_is_own(id) FOR update, delete WHERE fn::mt154_account_row(owner_account_id, 'fs.write');\nDEFINE FIELD OVERWRITE owner_account_id ON TABLE micro_tasks TYPE option<record<local_accounts>> DEFAULT $auth.account_id READONLY;\nDEFINE FIELD OVERWRITE mt_id ON TABLE micro_tasks TYPE string ASSERT $value = fn::mt159_locus_key_id(record::id($this.id));\n", "DEFINE TABLE OVERWRITE micro_tasks SCHEMAFULL PERMISSIONS NONE;\nDEFINE FIELD OVERWRITE mt_id ON TABLE micro_tasks TYPE string ASSERT $value = record::id($this.id);\n"),
    ("DEFINE INDEX OVERWRITE pk_micro_tasks ON TABLE micro_tasks FIELDS owner_account_id, mt_id UNIQUE;\n", "DEFINE INDEX OVERWRITE pk_micro_tasks ON TABLE micro_tasks FIELDS mt_id UNIQUE;\n"),
    ("DEFINE TABLE OVERWRITE mt_iterations SCHEMAFULL PERMISSIONS FOR select WHERE fn::mt154_account_row(owner_account_id, 'fs.read') FOR create WHERE fn::mt154_account_row(owner_account_id, 'fs.write') AND mt_id.owner_account_id = owner_account_id AND fn::mt159_locus_key_is_own(id) FOR update WHERE fn::mt154_account_row(owner_account_id, 'fs.write') AND mt_id.owner_account_id = owner_account_id FOR delete WHERE fn::mt154_account_row(owner_account_id, 'fs.write');\nDEFINE FIELD OVERWRITE owner_account_id ON TABLE mt_iterations TYPE option<record<local_accounts>> DEFAULT $auth.account_id READONLY;\nDEFINE FIELD OVERWRITE iteration_id ON TABLE mt_iterations TYPE string ASSERT $value = fn::mt159_locus_key_id(record::id($this.id));\n", "DEFINE TABLE OVERWRITE mt_iterations SCHEMAFULL PERMISSIONS NONE;\nDEFINE FIELD OVERWRITE iteration_id ON TABLE mt_iterations TYPE string ASSERT $value = record::id($this.id);\n"),
    ("DEFINE INDEX OVERWRITE pk_mt_iterations ON TABLE mt_iterations FIELDS owner_account_id, iteration_id UNIQUE;\n", "DEFINE INDEX OVERWRITE pk_mt_iterations ON TABLE mt_iterations FIELDS iteration_id UNIQUE;\n"),
    ("DEFINE TABLE OVERWRITE kernel_event_ledger SCHEMAFULL\n    PERMISSIONS FOR select WHERE (fn::mt109_ledger_receipt(authority_capability_id, event_type, source_component, aggregate_type, payload, wsids) AND fn::mt109_ledger_reader(authority_resource_id, authority_session_id, authority_capability_id, authority_action, wsids)) OR fn::mt120_document_receipt(authority_resource_id, authority_session_id, authority_capability_id, authority_action, wsids, event_type, source_component, aggregate_type, aggregate_id, payload, actor_kind, actor_id, false) OR fn::mt120_workspace_state_receipt(authority_resource_id, authority_session_id, authority_capability_id, authority_action, wsids, event_type, source_component, aggregate_type, aggregate_id, payload, actor_kind, actor_id, false) OR fn::mt120_nav_receipt(authority_resource_id, authority_session_id, authority_capability_id, authority_action, wsids, event_type, source_component, aggregate_type, aggregate_id, payload, actor_kind, actor_id, session_run_id, false) OR fn::mt120_nav_quiet_receipt(authority_resource_id, authority_session_id, authority_capability_id, authority_action, wsids, event_type, source_component, aggregate_type, aggregate_id, payload, actor_kind, actor_id, session_run_id, false) OR fn::mt120_index_receipt(authority_resource_id, authority_session_id, authority_capability_id, authority_action, wsids, event_type, source_component, aggregate_type, aggregate_id, payload, actor_kind, actor_id, session_run_id, false) OR fn::mt120_loom_receipt(authority_resource_id, authority_session_id, authority_capability_id, authority_action, wsids, event_type, source_component, aggregate_type, aggregate_id, payload, actor_kind, actor_id, false) OR fn::mt154_workspace_receipt(authority_resource_id, authority_session_id, authority_capability_id, authority_action, wsids, event_type, source_component, aggregate_type, aggregate_id, payload, actor_kind, actor_id, false) OR fn::mt154_account_receipt(authority_resource_id, authority_session_id, authority_capability_id, authority_action, wsids, event_type, source_component, aggregate_type, aggregate_id, payload, actor_kind, actor_id, false) OR fn::mt157_breakpoint_receipt(authority_resource_id, authority_session_id, authority_capability_id, authority_action, wsids, event_type, source_component, aggregate_type, aggregate_id, payload, actor_kind, actor_id, false)\n                FOR create WHERE (fn::mt109_ledger_receipt(authority_capability_id, event_type, source_component, aggregate_type, payload, wsids) AND fn::mt109_ledger_producer(authority_resource_id, authority_session_id, authority_capability_id, authority_action, wsids)) OR fn::mt120_document_receipt(authority_resource_id, authority_session_id, authority_capability_id, authority_action, wsids, event_type, source_component, aggregate_type, aggregate_id, payload, actor_kind, actor_id, true) OR fn::mt120_workspace_state_receipt(authority_resource_id, authority_session_id, authority_capability_id, authority_action, wsids, event_type, source_component, aggregate_type, aggregate_id, payload, actor_kind, actor_id, true) OR fn::mt120_nav_receipt(authority_resource_id, authority_session_id, authority_capability_id, authority_action, wsids, event_type, source_component, aggregate_type, aggregate_id, payload, actor_kind, actor_id, session_run_id, true) OR fn::mt120_nav_quiet_receipt(authority_resource_id, authority_session_id, authority_capability_id, authority_action, wsids, event_type, source_component, aggregate_type, aggregate_id, payload, actor_kind, actor_id, session_run_id, true) OR fn::mt120_index_receipt(authority_resource_id, authority_session_id, authority_capability_id, authority_action, wsids, event_type, source_component, aggregate_type, aggregate_id, payload, actor_kind, actor_id, session_run_id, true) OR fn::mt120_loom_receipt(authority_resource_id, authority_session_id, authority_capability_id, authority_action, wsids, event_type, source_component, aggregate_type, aggregate_id, payload, actor_kind, actor_id, true) OR fn::mt154_workspace_receipt(authority_resource_id, authority_session_id, authority_capability_id, authority_action, wsids, event_type, source_component, aggregate_type, aggregate_id, payload, actor_kind, actor_id, true) OR fn::mt154_account_receipt(authority_resource_id, authority_session_id, authority_capability_id, authority_action, wsids, event_type, source_component, aggregate_type, aggregate_id, payload, actor_kind, actor_id, true) OR fn::mt157_breakpoint_receipt(authority_resource_id, authority_session_id, authority_capability_id, authority_action, wsids, event_type, source_component, aggregate_type, aggregate_id, payload, actor_kind, actor_id, true)\n                FOR update, delete NONE;\n", "DEFINE TABLE OVERWRITE kernel_event_ledger SCHEMAFULL\n    PERMISSIONS FOR select WHERE (fn::mt109_ledger_receipt(authority_capability_id, event_type, source_component, aggregate_type, payload, wsids) AND fn::mt109_ledger_reader(authority_resource_id, authority_session_id, authority_capability_id, authority_action, wsids)) OR fn::mt120_document_receipt(authority_resource_id, authority_session_id, authority_capability_id, authority_action, wsids, event_type, source_component, aggregate_type, aggregate_id, payload, actor_kind, actor_id, false) OR fn::mt120_workspace_state_receipt(authority_resource_id, authority_session_id, authority_capability_id, authority_action, wsids, event_type, source_component, aggregate_type, aggregate_id, payload, actor_kind, actor_id, false) OR fn::mt120_nav_receipt(authority_resource_id, authority_session_id, authority_capability_id, authority_action, wsids, event_type, source_component, aggregate_type, aggregate_id, payload, actor_kind, actor_id, session_run_id, false) OR fn::mt120_nav_quiet_receipt(authority_resource_id, authority_session_id, authority_capability_id, authority_action, wsids, event_type, source_component, aggregate_type, aggregate_id, payload, actor_kind, actor_id, session_run_id, false) OR fn::mt120_index_receipt(authority_resource_id, authority_session_id, authority_capability_id, authority_action, wsids, event_type, source_component, aggregate_type, aggregate_id, payload, actor_kind, actor_id, session_run_id, false) OR fn::mt120_loom_receipt(authority_resource_id, authority_session_id, authority_capability_id, authority_action, wsids, event_type, source_component, aggregate_type, aggregate_id, payload, actor_kind, actor_id, false)\n                FOR create WHERE (fn::mt109_ledger_receipt(authority_capability_id, event_type, source_component, aggregate_type, payload, wsids) AND fn::mt109_ledger_producer(authority_resource_id, authority_session_id, authority_capability_id, authority_action, wsids)) OR fn::mt120_document_receipt(authority_resource_id, authority_session_id, authority_capability_id, authority_action, wsids, event_type, source_component, aggregate_type, aggregate_id, payload, actor_kind, actor_id, true) OR fn::mt120_workspace_state_receipt(authority_resource_id, authority_session_id, authority_capability_id, authority_action, wsids, event_type, source_component, aggregate_type, aggregate_id, payload, actor_kind, actor_id, true) OR fn::mt120_nav_receipt(authority_resource_id, authority_session_id, authority_capability_id, authority_action, wsids, event_type, source_component, aggregate_type, aggregate_id, payload, actor_kind, actor_id, session_run_id, true) OR fn::mt120_nav_quiet_receipt(authority_resource_id, authority_session_id, authority_capability_id, authority_action, wsids, event_type, source_component, aggregate_type, aggregate_id, payload, actor_kind, actor_id, session_run_id, true) OR fn::mt120_index_receipt(authority_resource_id, authority_session_id, authority_capability_id, authority_action, wsids, event_type, source_component, aggregate_type, aggregate_id, payload, actor_kind, actor_id, session_run_id, true) OR fn::mt120_loom_receipt(authority_resource_id, authority_session_id, authority_capability_id, authority_action, wsids, event_type, source_component, aggregate_type, aggregate_id, payload, actor_kind, actor_id, true)\n                FOR update, delete NONE;\n"),
    ("DEFINE TABLE OVERWRITE kernel_crdt_updates SCHEMAFULL PERMISSIONS FOR select WHERE fn::mt120_document_access(document_id, workspace_id, 'read', 'fs.read') FOR create WHERE fn::mt120_document_access(document_id, workspace_id, 'update', 'fs.write') AND event_ledger_event_id.authority_session_id = $auth.id AND event_ledger_event_id.wsids = [workspace_id] FOR update NONE FOR delete WHERE fn::mt109_has_workspace_access(workspace_id, 'delete', 'fs.write');\n", "DEFINE TABLE OVERWRITE kernel_crdt_updates SCHEMAFULL PERMISSIONS NONE;\n"),
    ("-- MT-154 (D-154-3; Master Spec 02-system-architecture.md:2740/:2751): every atelier_* row is an\n-- account-owned ProtectedResource. owner_account_id is stamped from $auth at create (NONE for root/system\n-- writers, which no record user can see) and is READONLY; fn::mt154_account_row gates every operation.\nDEFINE TABLE OVERWRITE atelier_character SCHEMAFULL PERMISSIONS FOR select WHERE fn::mt154_account_row(owner_account_id, 'fs.read') FOR create, update, delete WHERE fn::mt154_account_row(owner_account_id, 'fs.write');\nDEFINE FIELD OVERWRITE owner_account_id ON TABLE atelier_character TYPE option<record<local_accounts>> DEFAULT $auth.account_id READONLY;\n", "DEFINE TABLE OVERWRITE atelier_character SCHEMAFULL PERMISSIONS NONE;\n"),
    ("DEFINE TABLE OVERWRITE atelier_sheet_version SCHEMAFULL PERMISSIONS FOR select WHERE fn::mt154_account_row(owner_account_id, 'fs.read') FOR create, update, delete WHERE fn::mt154_account_row(owner_account_id, 'fs.write');\nDEFINE FIELD OVERWRITE owner_account_id ON TABLE atelier_sheet_version TYPE option<record<local_accounts>> DEFAULT $auth.account_id READONLY;\n", "DEFINE TABLE OVERWRITE atelier_sheet_version SCHEMAFULL PERMISSIONS NONE;\n"),
    ("DEFINE TABLE OVERWRITE atelier_media_asset SCHEMAFULL PERMISSIONS FOR select WHERE fn::mt154_account_row(owner_account_id, 'fs.read') FOR create, update, delete WHERE fn::mt154_account_row(owner_account_id, 'fs.write');\nDEFINE FIELD OVERWRITE owner_account_id ON TABLE atelier_media_asset TYPE option<record<local_accounts>> DEFAULT $auth.account_id READONLY;\n", "DEFINE TABLE OVERWRITE atelier_media_asset SCHEMAFULL PERMISSIONS NONE;\n"),
    ("DEFINE TABLE OVERWRITE atelier_event SCHEMAFULL PERMISSIONS FOR select WHERE fn::mt154_account_row(owner_account_id, 'fs.read') FOR create, update, delete WHERE fn::mt154_account_row(owner_account_id, 'fs.write');\nDEFINE FIELD OVERWRITE owner_account_id ON TABLE atelier_event TYPE option<record<local_accounts>> DEFAULT $auth.account_id READONLY;\n", "DEFINE TABLE OVERWRITE atelier_event SCHEMAFULL PERMISSIONS NONE;\n"),
    ("DEFINE TABLE OVERWRITE atelier_intake_batch SCHEMAFULL PERMISSIONS FOR select WHERE fn::mt154_account_row(owner_account_id, 'fs.read') FOR create, update, delete WHERE fn::mt154_account_row(owner_account_id, 'fs.write');\nDEFINE FIELD OVERWRITE owner_account_id ON TABLE atelier_intake_batch TYPE option<record<local_accounts>> DEFAULT $auth.account_id READONLY;\n", "DEFINE TABLE OVERWRITE atelier_intake_batch SCHEMAFULL PERMISSIONS NONE;\n"),
    ("DEFINE TABLE OVERWRITE atelier_intake_item SCHEMAFULL PERMISSIONS FOR select WHERE fn::mt154_account_row(owner_account_id, 'fs.read') FOR create, update, delete WHERE fn::mt154_account_row(owner_account_id, 'fs.write');\nDEFINE FIELD OVERWRITE owner_account_id ON TABLE atelier_intake_item TYPE option<record<local_accounts>> DEFAULT $auth.account_id READONLY;\n", "DEFINE TABLE OVERWRITE atelier_intake_item SCHEMAFULL PERMISSIONS NONE;\n"),
    ("DEFINE TABLE OVERWRITE atelier_collection SCHEMAFULL PERMISSIONS FOR select WHERE fn::mt154_account_row(owner_account_id, 'fs.read') FOR create, update, delete WHERE fn::mt154_account_row(owner_account_id, 'fs.write');\nDEFINE FIELD OVERWRITE owner_account_id ON TABLE atelier_collection TYPE option<record<local_accounts>> DEFAULT $auth.account_id READONLY;\n", "DEFINE TABLE OVERWRITE atelier_collection SCHEMAFULL PERMISSIONS NONE;\n"),
    ("DEFINE TABLE OVERWRITE atelier_collection_item SCHEMAFULL PERMISSIONS FOR select WHERE fn::mt154_account_row(owner_account_id, 'fs.read') FOR create, update, delete WHERE fn::mt154_account_row(owner_account_id, 'fs.write');\nDEFINE FIELD OVERWRITE owner_account_id ON TABLE atelier_collection_item TYPE option<record<local_accounts>> DEFAULT $auth.account_id READONLY;\n", "DEFINE TABLE OVERWRITE atelier_collection_item SCHEMAFULL PERMISSIONS NONE;\n"),
    ("DEFINE TABLE OVERWRITE atelier_contact_sheet SCHEMAFULL PERMISSIONS FOR select WHERE fn::mt154_account_row(owner_account_id, 'fs.read') FOR create, update, delete WHERE fn::mt154_account_row(owner_account_id, 'fs.write');\nDEFINE FIELD OVERWRITE owner_account_id ON TABLE atelier_contact_sheet TYPE option<record<local_accounts>> DEFAULT $auth.account_id READONLY;\n", "DEFINE TABLE OVERWRITE atelier_contact_sheet SCHEMAFULL PERMISSIONS NONE;\n"),
    ("DEFINE TABLE OVERWRITE atelier_tag SCHEMAFULL PERMISSIONS FOR select WHERE fn::mt154_account_row(owner_account_id, 'fs.read') FOR create, update, delete WHERE fn::mt154_account_row(owner_account_id, 'fs.write');\nDEFINE FIELD OVERWRITE owner_account_id ON TABLE atelier_tag TYPE option<record<local_accounts>> DEFAULT $auth.account_id READONLY;\n", "DEFINE TABLE OVERWRITE atelier_tag SCHEMAFULL PERMISSIONS NONE;\n"),
    ("DEFINE TABLE OVERWRITE atelier_character_tag SCHEMAFULL PERMISSIONS FOR select WHERE fn::mt154_account_row(owner_account_id, 'fs.read') FOR create, update, delete WHERE fn::mt154_account_row(owner_account_id, 'fs.write');\nDEFINE FIELD OVERWRITE owner_account_id ON TABLE atelier_character_tag TYPE option<record<local_accounts>> DEFAULT $auth.account_id READONLY;\n", "DEFINE TABLE OVERWRITE atelier_character_tag SCHEMAFULL PERMISSIONS NONE;\n"),
    ("DEFINE TABLE OVERWRITE atelier_tag_rule SCHEMAFULL PERMISSIONS FOR select WHERE fn::mt154_account_row(owner_account_id, 'fs.read') FOR create, update, delete WHERE fn::mt154_account_row(owner_account_id, 'fs.write');\nDEFINE FIELD OVERWRITE owner_account_id ON TABLE atelier_tag_rule TYPE option<record<local_accounts>> DEFAULT $auth.account_id READONLY;\n", "DEFINE TABLE OVERWRITE atelier_tag_rule SCHEMAFULL PERMISSIONS NONE;\n"),
    ("DEFINE TABLE OVERWRITE atelier_similarity_projection SCHEMAFULL PERMISSIONS FOR select WHERE fn::mt154_account_row(owner_account_id, 'fs.read') FOR create, update, delete WHERE fn::mt154_account_row(owner_account_id, 'fs.write');\nDEFINE FIELD OVERWRITE owner_account_id ON TABLE atelier_similarity_projection TYPE option<record<local_accounts>> DEFAULT $auth.account_id READONLY;\n", "DEFINE TABLE OVERWRITE atelier_similarity_projection SCHEMAFULL PERMISSIONS NONE;\n"),
    ("DEFINE TABLE OVERWRITE atelier_export_request SCHEMAFULL PERMISSIONS FOR select WHERE fn::mt154_account_row(owner_account_id, 'fs.read') FOR create, update, delete WHERE fn::mt154_account_row(owner_account_id, 'fs.write');\nDEFINE FIELD OVERWRITE owner_account_id ON TABLE atelier_export_request TYPE option<record<local_accounts>> DEFAULT $auth.account_id READONLY;\n", "DEFINE TABLE OVERWRITE atelier_export_request SCHEMAFULL PERMISSIONS NONE;\n"),
    ("DEFINE TABLE OVERWRITE atelier_export_result SCHEMAFULL PERMISSIONS FOR select WHERE fn::mt154_account_row(owner_account_id, 'fs.read') FOR create, update, delete WHERE fn::mt154_account_row(owner_account_id, 'fs.write');\nDEFINE FIELD OVERWRITE owner_account_id ON TABLE atelier_export_result TYPE option<record<local_accounts>> DEFAULT $auth.account_id READONLY;\n", "DEFINE TABLE OVERWRITE atelier_export_result SCHEMAFULL PERMISSIONS NONE;\n"),
    ("DEFINE TABLE OVERWRITE atelier_export_manifest_entry SCHEMAFULL PERMISSIONS FOR select WHERE fn::mt154_account_row(owner_account_id, 'fs.read') FOR create, update, delete WHERE fn::mt154_account_row(owner_account_id, 'fs.write');\nDEFINE FIELD OVERWRITE owner_account_id ON TABLE atelier_export_manifest_entry TYPE option<record<local_accounts>> DEFAULT $auth.account_id READONLY;\n", "DEFINE TABLE OVERWRITE atelier_export_manifest_entry SCHEMAFULL PERMISSIONS NONE;\n"),
    ("DEFINE TABLE OVERWRITE atelier_media_annotation SCHEMAFULL PERMISSIONS FOR select WHERE fn::mt154_account_row(owner_account_id, 'fs.read') FOR create, update, delete WHERE fn::mt154_account_row(owner_account_id, 'fs.write');\nDEFINE FIELD OVERWRITE owner_account_id ON TABLE atelier_media_annotation TYPE option<record<local_accounts>> DEFAULT $auth.account_id READONLY;\n", "DEFINE TABLE OVERWRITE atelier_media_annotation SCHEMAFULL PERMISSIONS NONE;\n"),
    ("DEFINE TABLE OVERWRITE atelier_preference SCHEMAFULL PERMISSIONS FOR select WHERE fn::mt154_account_row(owner_account_id, 'fs.read') FOR create, update, delete WHERE fn::mt154_account_row(owner_account_id, 'fs.write');\nDEFINE FIELD OVERWRITE owner_account_id ON TABLE atelier_preference TYPE option<record<local_accounts>> DEFAULT $auth.account_id READONLY;\n", "DEFINE TABLE OVERWRITE atelier_preference SCHEMAFULL PERMISSIONS NONE;\n"),
    ("DEFINE TABLE OVERWRITE atelier_pose_rig SCHEMAFULL PERMISSIONS FOR select WHERE fn::mt154_account_row(owner_account_id, 'fs.read') FOR create, update, delete WHERE fn::mt154_account_row(owner_account_id, 'fs.write');\nDEFINE FIELD OVERWRITE owner_account_id ON TABLE atelier_pose_rig TYPE option<record<local_accounts>> DEFAULT $auth.account_id READONLY;\n", "DEFINE TABLE OVERWRITE atelier_pose_rig SCHEMAFULL PERMISSIONS NONE;\n"),
    ("DEFINE TABLE OVERWRITE atelier_pose_head_pose SCHEMAFULL PERMISSIONS FOR select WHERE fn::mt154_account_row(owner_account_id, 'fs.read') FOR create, update, delete WHERE fn::mt154_account_row(owner_account_id, 'fs.write');\nDEFINE FIELD OVERWRITE owner_account_id ON TABLE atelier_pose_head_pose TYPE option<record<local_accounts>> DEFAULT $auth.account_id READONLY;\n", "DEFINE TABLE OVERWRITE atelier_pose_head_pose SCHEMAFULL PERMISSIONS NONE;\n"),
    ("DEFINE TABLE OVERWRITE atelier_pose_calibration SCHEMAFULL PERMISSIONS FOR select WHERE fn::mt154_account_row(owner_account_id, 'fs.read') FOR create, update, delete WHERE fn::mt154_account_row(owner_account_id, 'fs.write');\nDEFINE FIELD OVERWRITE owner_account_id ON TABLE atelier_pose_calibration TYPE option<record<local_accounts>> DEFAULT $auth.account_id READONLY;\n", "DEFINE TABLE OVERWRITE atelier_pose_calibration SCHEMAFULL PERMISSIONS NONE;\n"),
    ("DEFINE TABLE OVERWRITE atelier_identity_profile SCHEMAFULL PERMISSIONS FOR select WHERE fn::mt154_account_row(owner_account_id, 'fs.read') FOR create, update, delete WHERE fn::mt154_account_row(owner_account_id, 'fs.write');\nDEFINE FIELD OVERWRITE owner_account_id ON TABLE atelier_identity_profile TYPE option<record<local_accounts>> DEFAULT $auth.account_id READONLY;\n", "DEFINE TABLE OVERWRITE atelier_identity_profile SCHEMAFULL PERMISSIONS NONE;\n"),
    ("DEFINE TABLE OVERWRITE atelier_comfy_bridge_probe SCHEMAFULL PERMISSIONS FOR select WHERE fn::mt154_account_row(owner_account_id, 'fs.read') FOR create, update, delete WHERE fn::mt154_account_row(owner_account_id, 'fs.write');\nDEFINE FIELD OVERWRITE owner_account_id ON TABLE atelier_comfy_bridge_probe TYPE option<record<local_accounts>> DEFAULT $auth.account_id READONLY;\n", "DEFINE TABLE OVERWRITE atelier_comfy_bridge_probe SCHEMAFULL PERMISSIONS NONE;\n"),
    ("DEFINE TABLE OVERWRITE atelier_comfy_capability_registration SCHEMAFULL PERMISSIONS FOR select WHERE fn::mt154_account_row(owner_account_id, 'fs.read') FOR create, update, delete WHERE fn::mt154_account_row(owner_account_id, 'fs.write');\nDEFINE FIELD OVERWRITE owner_account_id ON TABLE atelier_comfy_capability_registration TYPE option<record<local_accounts>> DEFAULT $auth.account_id READONLY;\n", "DEFINE TABLE OVERWRITE atelier_comfy_capability_registration SCHEMAFULL PERMISSIONS NONE;\n"),
    ("DEFINE TABLE OVERWRITE atelier_comfy_declared_output SCHEMAFULL PERMISSIONS FOR select WHERE fn::mt154_account_row(owner_account_id, 'fs.read') FOR create, update, delete WHERE fn::mt154_account_row(owner_account_id, 'fs.write');\nDEFINE FIELD OVERWRITE owner_account_id ON TABLE atelier_comfy_declared_output TYPE option<record<local_accounts>> DEFAULT $auth.account_id READONLY;\n", "DEFINE TABLE OVERWRITE atelier_comfy_declared_output SCHEMAFULL PERMISSIONS NONE;\n"),
    ("DEFINE TABLE OVERWRITE atelier_comfy_capability_reject SCHEMAFULL PERMISSIONS FOR select WHERE fn::mt154_account_row(owner_account_id, 'fs.read') FOR create, update, delete WHERE fn::mt154_account_row(owner_account_id, 'fs.write');\nDEFINE FIELD OVERWRITE owner_account_id ON TABLE atelier_comfy_capability_reject TYPE option<record<local_accounts>> DEFAULT $auth.account_id READONLY;\n", "DEFINE TABLE OVERWRITE atelier_comfy_capability_reject SCHEMAFULL PERMISSIONS NONE;\n"),
    ("DEFINE TABLE OVERWRITE atelier_comfy_intake_output SCHEMAFULL PERMISSIONS FOR select WHERE fn::mt154_account_row(owner_account_id, 'fs.read') FOR create, update, delete WHERE fn::mt154_account_row(owner_account_id, 'fs.write');\nDEFINE FIELD OVERWRITE owner_account_id ON TABLE atelier_comfy_intake_output TYPE option<record<local_accounts>> DEFAULT $auth.account_id READONLY;\n", "DEFINE TABLE OVERWRITE atelier_comfy_intake_output SCHEMAFULL PERMISSIONS NONE;\n"),
    ("DEFINE TABLE OVERWRITE atelier_comfy_fallback_marker SCHEMAFULL PERMISSIONS FOR select WHERE fn::mt154_account_row(owner_account_id, 'fs.read') FOR create, update, delete WHERE fn::mt154_account_row(owner_account_id, 'fs.write');\nDEFINE FIELD OVERWRITE owner_account_id ON TABLE atelier_comfy_fallback_marker TYPE option<record<local_accounts>> DEFAULT $auth.account_id READONLY;\n", "DEFINE TABLE OVERWRITE atelier_comfy_fallback_marker SCHEMAFULL PERMISSIONS NONE;\n"),
    ("DEFINE TABLE OVERWRITE atelier_sourcing_spec SCHEMAFULL PERMISSIONS FOR select WHERE fn::mt154_account_row(owner_account_id, 'fs.read') FOR create, update, delete WHERE fn::mt154_account_row(owner_account_id, 'fs.write');\nDEFINE FIELD OVERWRITE owner_account_id ON TABLE atelier_sourcing_spec TYPE option<record<local_accounts>> DEFAULT $auth.account_id READONLY;\n", "DEFINE TABLE OVERWRITE atelier_sourcing_spec SCHEMAFULL PERMISSIONS NONE;\n"),
    ("DEFINE TABLE OVERWRITE atelier_handler_version_matrix SCHEMAFULL PERMISSIONS FOR select WHERE fn::mt154_account_row(owner_account_id, 'fs.read') FOR create, update, delete WHERE fn::mt154_account_row(owner_account_id, 'fs.write');\nDEFINE FIELD OVERWRITE owner_account_id ON TABLE atelier_handler_version_matrix TYPE option<record<local_accounts>> DEFAULT $auth.account_id READONLY;\n", "DEFINE TABLE OVERWRITE atelier_handler_version_matrix SCHEMAFULL PERMISSIONS NONE;\n"),
    ("DEFINE TABLE OVERWRITE atelier_sourcing_binding_decision SCHEMAFULL PERMISSIONS FOR select WHERE fn::mt154_account_row(owner_account_id, 'fs.read') FOR create, update, delete WHERE fn::mt154_account_row(owner_account_id, 'fs.write');\nDEFINE FIELD OVERWRITE owner_account_id ON TABLE atelier_sourcing_binding_decision TYPE option<record<local_accounts>> DEFAULT $auth.account_id READONLY;\n", "DEFINE TABLE OVERWRITE atelier_sourcing_binding_decision SCHEMAFULL PERMISSIONS NONE;\n"),
    ("DEFINE TABLE OVERWRITE atelier_version_mismatch_receipt SCHEMAFULL PERMISSIONS FOR select WHERE fn::mt154_account_row(owner_account_id, 'fs.read') FOR create, update, delete WHERE fn::mt154_account_row(owner_account_id, 'fs.write');\nDEFINE FIELD OVERWRITE owner_account_id ON TABLE atelier_version_mismatch_receipt TYPE option<record<local_accounts>> DEFAULT $auth.account_id READONLY;\n", "DEFINE TABLE OVERWRITE atelier_version_mismatch_receipt SCHEMAFULL PERMISSIONS NONE;\n"),
    ("DEFINE TABLE OVERWRITE atelier_sourcing_ingestion_receipt SCHEMAFULL PERMISSIONS FOR select WHERE fn::mt154_account_row(owner_account_id, 'fs.read') FOR create, update, delete WHERE fn::mt154_account_row(owner_account_id, 'fs.write');\nDEFINE FIELD OVERWRITE owner_account_id ON TABLE atelier_sourcing_ingestion_receipt TYPE option<record<local_accounts>> DEFAULT $auth.account_id READONLY;\n", "DEFINE TABLE OVERWRITE atelier_sourcing_ingestion_receipt SCHEMAFULL PERMISSIONS NONE;\n"),
    ("DEFINE TABLE OVERWRITE atelier_media_probe_report SCHEMAFULL PERMISSIONS FOR select WHERE fn::mt154_account_row(owner_account_id, 'fs.read') FOR create, update, delete WHERE fn::mt154_account_row(owner_account_id, 'fs.write');\nDEFINE FIELD OVERWRITE owner_account_id ON TABLE atelier_media_probe_report TYPE option<record<local_accounts>> DEFAULT $auth.account_id READONLY;\n", "DEFINE TABLE OVERWRITE atelier_media_probe_report SCHEMAFULL PERMISSIONS NONE;\n"),
    ("DEFINE TABLE OVERWRITE atelier_transcript_artifact SCHEMAFULL PERMISSIONS FOR select WHERE fn::mt154_account_row(owner_account_id, 'fs.read') FOR create, update, delete WHERE fn::mt154_account_row(owner_account_id, 'fs.write');\nDEFINE FIELD OVERWRITE owner_account_id ON TABLE atelier_transcript_artifact TYPE option<record<local_accounts>> DEFAULT $auth.account_id READONLY;\n", "DEFINE TABLE OVERWRITE atelier_transcript_artifact SCHEMAFULL PERMISSIONS NONE;\n"),
    ("DEFINE TABLE OVERWRITE atelier_caption_artifact SCHEMAFULL PERMISSIONS FOR select WHERE fn::mt154_account_row(owner_account_id, 'fs.read') FOR create, update, delete WHERE fn::mt154_account_row(owner_account_id, 'fs.write');\nDEFINE FIELD OVERWRITE owner_account_id ON TABLE atelier_caption_artifact TYPE option<record<local_accounts>> DEFAULT $auth.account_id READONLY;\n", "DEFINE TABLE OVERWRITE atelier_caption_artifact SCHEMAFULL PERMISSIONS NONE;\n"),
    ("DEFINE TABLE OVERWRITE atelier_transcript_receipt SCHEMAFULL PERMISSIONS FOR select WHERE fn::mt154_account_row(owner_account_id, 'fs.read') FOR create, update, delete WHERE fn::mt154_account_row(owner_account_id, 'fs.write');\nDEFINE FIELD OVERWRITE owner_account_id ON TABLE atelier_transcript_receipt TYPE option<record<local_accounts>> DEFAULT $auth.account_id READONLY;\n", "DEFINE TABLE OVERWRITE atelier_transcript_receipt SCHEMAFULL PERMISSIONS NONE;\n"),
    ("DEFINE TABLE OVERWRITE atelier_md_output_root SCHEMAFULL PERMISSIONS FOR select WHERE fn::mt154_account_row(owner_account_id, 'fs.read') FOR create, update, delete WHERE fn::mt154_account_row(owner_account_id, 'fs.write');\nDEFINE FIELD OVERWRITE owner_account_id ON TABLE atelier_md_output_root TYPE option<record<local_accounts>> DEFAULT $auth.account_id READONLY;\n", "DEFINE TABLE OVERWRITE atelier_md_output_root SCHEMAFULL PERMISSIONS NONE;\n"),
    ("DEFINE TABLE OVERWRITE atelier_md_allowlist_policy SCHEMAFULL PERMISSIONS FOR select WHERE fn::mt154_account_row(owner_account_id, 'fs.read') FOR create, update, delete WHERE fn::mt154_account_row(owner_account_id, 'fs.write');\nDEFINE FIELD OVERWRITE owner_account_id ON TABLE atelier_md_allowlist_policy TYPE option<record<local_accounts>> DEFAULT $auth.account_id READONLY;\n", "DEFINE TABLE OVERWRITE atelier_md_allowlist_policy SCHEMAFULL PERMISSIONS NONE;\n"),
    ("DEFINE TABLE OVERWRITE atelier_md_auth_context SCHEMAFULL PERMISSIONS FOR select WHERE fn::mt154_account_row(owner_account_id, 'fs.read') FOR create, update, delete WHERE fn::mt154_account_row(owner_account_id, 'fs.write');\nDEFINE FIELD OVERWRITE owner_account_id ON TABLE atelier_md_auth_context TYPE option<record<local_accounts>> DEFAULT $auth.account_id READONLY;\n", "DEFINE TABLE OVERWRITE atelier_md_auth_context SCHEMAFULL PERMISSIONS NONE;\n"),
    ("DEFINE TABLE OVERWRITE atelier_md_download_session SCHEMAFULL PERMISSIONS FOR select WHERE fn::mt154_account_row(owner_account_id, 'fs.read') FOR create, update, delete WHERE fn::mt154_account_row(owner_account_id, 'fs.write');\nDEFINE FIELD OVERWRITE owner_account_id ON TABLE atelier_md_download_session TYPE option<record<local_accounts>> DEFAULT $auth.account_id READONLY;\n", "DEFINE TABLE OVERWRITE atelier_md_download_session SCHEMAFULL PERMISSIONS NONE;\n"),
    ("DEFINE TABLE OVERWRITE atelier_md_item_state SCHEMAFULL PERMISSIONS FOR select WHERE fn::mt154_account_row(owner_account_id, 'fs.read') FOR create, update, delete WHERE fn::mt154_account_row(owner_account_id, 'fs.write');\nDEFINE FIELD OVERWRITE owner_account_id ON TABLE atelier_md_item_state TYPE option<record<local_accounts>> DEFAULT $auth.account_id READONLY;\n", "DEFINE TABLE OVERWRITE atelier_md_item_state SCHEMAFULL PERMISSIONS NONE;\n"),
    ("DEFINE TABLE OVERWRITE atelier_md_checkpoint SCHEMAFULL PERMISSIONS FOR select WHERE fn::mt154_account_row(owner_account_id, 'fs.read') FOR create, update, delete WHERE fn::mt154_account_row(owner_account_id, 'fs.write');\nDEFINE FIELD OVERWRITE owner_account_id ON TABLE atelier_md_checkpoint TYPE option<record<local_accounts>> DEFAULT $auth.account_id READONLY;\n", "DEFINE TABLE OVERWRITE atelier_md_checkpoint SCHEMAFULL PERMISSIONS NONE;\n"),
    ("DEFINE TABLE OVERWRITE atelier_md_session_receipt SCHEMAFULL PERMISSIONS FOR select WHERE fn::mt154_account_row(owner_account_id, 'fs.read') FOR create, update, delete WHERE fn::mt154_account_row(owner_account_id, 'fs.write');\nDEFINE FIELD OVERWRITE owner_account_id ON TABLE atelier_md_session_receipt TYPE option<record<local_accounts>> DEFAULT $auth.account_id READONLY;\n", "DEFINE TABLE OVERWRITE atelier_md_session_receipt SCHEMAFULL PERMISSIONS NONE;\n"),
    ("DEFINE TABLE OVERWRITE atelier_command_corpus_entry SCHEMAFULL PERMISSIONS FOR select WHERE fn::mt154_account_catalog_read(owner_account_id) FOR create, update, delete WHERE fn::mt154_account_row(owner_account_id, 'fs.write');\nDEFINE FIELD OVERWRITE owner_account_id ON TABLE atelier_command_corpus_entry TYPE option<record<local_accounts>> DEFAULT $auth.account_id READONLY;\n", "DEFINE TABLE OVERWRITE atelier_command_corpus_entry SCHEMAFULL PERMISSIONS NONE;\n"),
    ("DEFINE TABLE OVERWRITE atelier_command_corpus_blocked SCHEMAFULL PERMISSIONS FOR select WHERE fn::mt154_account_catalog_read(owner_account_id) FOR create, update, delete WHERE fn::mt154_account_row(owner_account_id, 'fs.write');\nDEFINE FIELD OVERWRITE owner_account_id ON TABLE atelier_command_corpus_blocked TYPE option<record<local_accounts>> DEFAULT $auth.account_id READONLY;\n", "DEFINE TABLE OVERWRITE atelier_command_corpus_blocked SCHEMAFULL PERMISSIONS NONE;\n"),
    ("DEFINE TABLE OVERWRITE atelier_command_corpus_parity_report SCHEMAFULL PERMISSIONS FOR select WHERE fn::mt154_account_catalog_read(owner_account_id) FOR create, update, delete WHERE fn::mt154_account_row(owner_account_id, 'fs.write');\nDEFINE FIELD OVERWRITE owner_account_id ON TABLE atelier_command_corpus_parity_report TYPE option<record<local_accounts>> DEFAULT $auth.account_id READONLY;\n", "DEFINE TABLE OVERWRITE atelier_command_corpus_parity_report SCHEMAFULL PERMISSIONS NONE;\n"),
    ("DEFINE TABLE OVERWRITE atelier_stealth_window SCHEMAFULL PERMISSIONS FOR select WHERE fn::mt154_account_row(owner_account_id, 'fs.read') FOR create, update, delete WHERE fn::mt154_account_row(owner_account_id, 'fs.write');\nDEFINE FIELD OVERWRITE owner_account_id ON TABLE atelier_stealth_window TYPE option<record<local_accounts>> DEFAULT $auth.account_id READONLY;\n", "DEFINE TABLE OVERWRITE atelier_stealth_window SCHEMAFULL PERMISSIONS NONE;\n"),
    ("DEFINE TABLE OVERWRITE atelier_stealth_ref SCHEMAFULL PERMISSIONS FOR select WHERE fn::mt154_account_row(owner_account_id, 'fs.read') FOR create, update, delete WHERE fn::mt154_account_row(owner_account_id, 'fs.write');\nDEFINE FIELD OVERWRITE owner_account_id ON TABLE atelier_stealth_ref TYPE option<record<local_accounts>> DEFAULT $auth.account_id READONLY;\n", "DEFINE TABLE OVERWRITE atelier_stealth_ref SCHEMAFULL PERMISSIONS NONE;\n"),
    ("DEFINE TABLE OVERWRITE atelier_stealth_capture SCHEMAFULL PERMISSIONS FOR select WHERE fn::mt154_account_row(owner_account_id, 'fs.read') FOR create, update, delete WHERE fn::mt154_account_row(owner_account_id, 'fs.write');\nDEFINE FIELD OVERWRITE owner_account_id ON TABLE atelier_stealth_capture TYPE option<record<local_accounts>> DEFAULT $auth.account_id READONLY;\n", "DEFINE TABLE OVERWRITE atelier_stealth_capture SCHEMAFULL PERMISSIONS NONE;\n"),
    ("DEFINE TABLE OVERWRITE atelier_sheet_parse_snapshot SCHEMAFULL PERMISSIONS FOR select WHERE fn::mt154_account_row(owner_account_id, 'fs.read') FOR create, update, delete WHERE fn::mt154_account_row(owner_account_id, 'fs.write');\nDEFINE FIELD OVERWRITE owner_account_id ON TABLE atelier_sheet_parse_snapshot TYPE option<record<local_accounts>> DEFAULT $auth.account_id READONLY;\n", "DEFINE TABLE OVERWRITE atelier_sheet_parse_snapshot SCHEMAFULL PERMISSIONS NONE;\n"),
    ("DEFINE TABLE OVERWRITE atelier_bulk_operation_receipt SCHEMAFULL PERMISSIONS FOR select WHERE fn::mt154_account_row(owner_account_id, 'fs.read') FOR create, update, delete WHERE fn::mt154_account_row(owner_account_id, 'fs.write');\nDEFINE FIELD OVERWRITE owner_account_id ON TABLE atelier_bulk_operation_receipt TYPE option<record<local_accounts>> DEFAULT $auth.account_id READONLY;\n", "DEFINE TABLE OVERWRITE atelier_bulk_operation_receipt SCHEMAFULL PERMISSIONS NONE;\n"),
    ("DEFINE TABLE OVERWRITE atelier_trash_marker SCHEMAFULL PERMISSIONS FOR select WHERE fn::mt154_account_row(owner_account_id, 'fs.read') FOR create, update, delete WHERE fn::mt154_account_row(owner_account_id, 'fs.write');\nDEFINE FIELD OVERWRITE owner_account_id ON TABLE atelier_trash_marker TYPE option<record<local_accounts>> DEFAULT $auth.account_id READONLY;\n", "DEFINE TABLE OVERWRITE atelier_trash_marker SCHEMAFULL PERMISSIONS NONE;\n"),
    ("DEFINE TABLE OVERWRITE atelier_source_evidence_record SCHEMAFULL PERMISSIONS FOR select WHERE fn::mt154_account_row(owner_account_id, 'fs.read') FOR create, update, delete WHERE fn::mt154_account_row(owner_account_id, 'fs.write');\nDEFINE FIELD OVERWRITE owner_account_id ON TABLE atelier_source_evidence_record TYPE option<record<local_accounts>> DEFAULT $auth.account_id READONLY;\n", "DEFINE TABLE OVERWRITE atelier_source_evidence_record SCHEMAFULL PERMISSIONS NONE;\n"),
    ("DEFINE TABLE OVERWRITE atelier_anchor_verification_record SCHEMAFULL PERMISSIONS FOR select WHERE fn::mt154_account_row(owner_account_id, 'fs.read') FOR create, update, delete WHERE fn::mt154_account_row(owner_account_id, 'fs.write');\nDEFINE FIELD OVERWRITE owner_account_id ON TABLE atelier_anchor_verification_record TYPE option<record<local_accounts>> DEFAULT $auth.account_id READONLY;\n", "DEFINE TABLE OVERWRITE atelier_anchor_verification_record SCHEMAFULL PERMISSIONS NONE;\n"),
    ("DEFINE TABLE OVERWRITE atelier_media_review_metadata SCHEMAFULL PERMISSIONS FOR select WHERE fn::mt154_account_row(owner_account_id, 'fs.read') FOR create, update, delete WHERE fn::mt154_account_row(owner_account_id, 'fs.write');\nDEFINE FIELD OVERWRITE owner_account_id ON TABLE atelier_media_review_metadata TYPE option<record<local_accounts>> DEFAULT $auth.account_id READONLY;\n", "DEFINE TABLE OVERWRITE atelier_media_review_metadata SCHEMAFULL PERMISSIONS NONE;\n"),
    ("DEFINE TABLE OVERWRITE atelier_media_derivative SCHEMAFULL PERMISSIONS FOR select WHERE fn::mt154_account_row(owner_account_id, 'fs.read') FOR create, update, delete WHERE fn::mt154_account_row(owner_account_id, 'fs.write');\nDEFINE FIELD OVERWRITE owner_account_id ON TABLE atelier_media_derivative TYPE option<record<local_accounts>> DEFAULT $auth.account_id READONLY;\n", "DEFINE TABLE OVERWRITE atelier_media_derivative SCHEMAFULL PERMISSIONS NONE;\n"),
    ("DEFINE TABLE OVERWRITE atelier_similarity_rebuild_job SCHEMAFULL PERMISSIONS FOR select WHERE fn::mt154_account_row(owner_account_id, 'fs.read') FOR create, update, delete WHERE fn::mt154_account_row(owner_account_id, 'fs.write');\nDEFINE FIELD OVERWRITE owner_account_id ON TABLE atelier_similarity_rebuild_job TYPE option<record<local_accounts>> DEFAULT $auth.account_id READONLY;\n", "DEFINE TABLE OVERWRITE atelier_similarity_rebuild_job SCHEMAFULL PERMISSIONS NONE;\n"),
    ("DEFINE TABLE OVERWRITE atelier_ai_tag_suggestion SCHEMAFULL PERMISSIONS FOR select WHERE fn::mt154_account_row(owner_account_id, 'fs.read') FOR create, update, delete WHERE fn::mt154_account_row(owner_account_id, 'fs.write');\nDEFINE FIELD OVERWRITE owner_account_id ON TABLE atelier_ai_tag_suggestion TYPE option<record<local_accounts>> DEFAULT $auth.account_id READONLY;\n", "DEFINE TABLE OVERWRITE atelier_ai_tag_suggestion SCHEMAFULL PERMISSIONS NONE;\n"),
    ("DEFINE TABLE OVERWRITE atelier_media_sidecar SCHEMAFULL PERMISSIONS FOR select WHERE fn::mt154_account_row(owner_account_id, 'fs.read') FOR create, update, delete WHERE fn::mt154_account_row(owner_account_id, 'fs.write');\nDEFINE FIELD OVERWRITE owner_account_id ON TABLE atelier_media_sidecar TYPE option<record<local_accounts>> DEFAULT $auth.account_id READONLY;\n", "DEFINE TABLE OVERWRITE atelier_media_sidecar SCHEMAFULL PERMISSIONS NONE;\n"),
    ("DEFINE TABLE OVERWRITE atelier_filesystem_health_check SCHEMAFULL PERMISSIONS FOR select WHERE fn::mt154_account_row(owner_account_id, 'fs.read') FOR create, update, delete WHERE fn::mt154_account_row(owner_account_id, 'fs.write');\nDEFINE FIELD OVERWRITE owner_account_id ON TABLE atelier_filesystem_health_check TYPE option<record<local_accounts>> DEFAULT $auth.account_id READONLY;\n", "DEFINE TABLE OVERWRITE atelier_filesystem_health_check SCHEMAFULL PERMISSIONS NONE;\n"),
    ("DEFINE TABLE OVERWRITE atelier_filesystem_health_finding SCHEMAFULL PERMISSIONS FOR select WHERE fn::mt154_account_row(owner_account_id, 'fs.read') FOR create, update, delete WHERE fn::mt154_account_row(owner_account_id, 'fs.write');\nDEFINE FIELD OVERWRITE owner_account_id ON TABLE atelier_filesystem_health_finding TYPE option<record<local_accounts>> DEFAULT $auth.account_id READONLY;\n", "DEFINE TABLE OVERWRITE atelier_filesystem_health_finding SCHEMAFULL PERMISSIONS NONE;\n"),
    ("DEFINE TABLE OVERWRITE atelier_image_import_request SCHEMAFULL PERMISSIONS FOR select WHERE fn::mt154_account_row(owner_account_id, 'fs.read') FOR create, update, delete WHERE fn::mt154_account_row(owner_account_id, 'fs.write');\nDEFINE FIELD OVERWRITE owner_account_id ON TABLE atelier_image_import_request TYPE option<record<local_accounts>> DEFAULT $auth.account_id READONLY;\n", "DEFINE TABLE OVERWRITE atelier_image_import_request SCHEMAFULL PERMISSIONS NONE;\n"),
    ("DEFINE TABLE OVERWRITE atelier_media_source_provenance_ref SCHEMAFULL PERMISSIONS FOR select WHERE fn::mt154_account_row(owner_account_id, 'fs.read') FOR create, update, delete WHERE fn::mt154_account_row(owner_account_id, 'fs.write');\nDEFINE FIELD OVERWRITE owner_account_id ON TABLE atelier_media_source_provenance_ref TYPE option<record<local_accounts>> DEFAULT $auth.account_id READONLY;\n", "DEFINE TABLE OVERWRITE atelier_media_source_provenance_ref SCHEMAFULL PERMISSIONS NONE;\n"),
    ("DEFINE TABLE OVERWRITE atelier_intake_item_rejection_audit SCHEMAFULL PERMISSIONS FOR select WHERE fn::mt154_account_row(owner_account_id, 'fs.read') FOR create, update, delete WHERE fn::mt154_account_row(owner_account_id, 'fs.write');\nDEFINE FIELD OVERWRITE owner_account_id ON TABLE atelier_intake_item_rejection_audit TYPE option<record<local_accounts>> DEFAULT $auth.account_id READONLY;\n", "DEFINE TABLE OVERWRITE atelier_intake_item_rejection_audit SCHEMAFULL PERMISSIONS NONE;\n"),
    ("DEFINE TABLE OVERWRITE atelier_export_intake_link SCHEMAFULL PERMISSIONS FOR select WHERE fn::mt154_account_row(owner_account_id, 'fs.read') FOR create, update, delete WHERE fn::mt154_account_row(owner_account_id, 'fs.write');\nDEFINE FIELD OVERWRITE owner_account_id ON TABLE atelier_export_intake_link TYPE option<record<local_accounts>> DEFAULT $auth.account_id READONLY;\n", "DEFINE TABLE OVERWRITE atelier_export_intake_link SCHEMAFULL PERMISSIONS NONE;\n"),
    ("DEFINE TABLE OVERWRITE atelier_media_asset_tag SCHEMAFULL PERMISSIONS FOR select WHERE fn::mt154_account_row(owner_account_id, 'fs.read') FOR create, update, delete WHERE fn::mt154_account_row(owner_account_id, 'fs.write');\nDEFINE FIELD OVERWRITE owner_account_id ON TABLE atelier_media_asset_tag TYPE option<record<local_accounts>> DEFAULT $auth.account_id READONLY;\n", "DEFINE TABLE OVERWRITE atelier_media_asset_tag SCHEMAFULL PERMISSIONS NONE;\n"),
    ("DEFINE TABLE OVERWRITE atelier_collection_metadata_application SCHEMAFULL PERMISSIONS FOR select WHERE fn::mt154_account_row(owner_account_id, 'fs.read') FOR create, update, delete WHERE fn::mt154_account_row(owner_account_id, 'fs.write');\nDEFINE FIELD OVERWRITE owner_account_id ON TABLE atelier_collection_metadata_application TYPE option<record<local_accounts>> DEFAULT $auth.account_id READONLY;\n", "DEFINE TABLE OVERWRITE atelier_collection_metadata_application SCHEMAFULL PERMISSIONS NONE;\n"),
    ("DEFINE TABLE OVERWRITE atelier_contact_sheet_svg_artifact SCHEMAFULL PERMISSIONS FOR select WHERE fn::mt154_account_row(owner_account_id, 'fs.read') FOR create, update, delete WHERE fn::mt154_account_row(owner_account_id, 'fs.write');\nDEFINE FIELD OVERWRITE owner_account_id ON TABLE atelier_contact_sheet_svg_artifact TYPE option<record<local_accounts>> DEFAULT $auth.account_id READONLY;\n", "DEFINE TABLE OVERWRITE atelier_contact_sheet_svg_artifact SCHEMAFULL PERMISSIONS NONE;\n"),
    ("DEFINE TABLE OVERWRITE atelier_contact_sheet_raster_export_plan SCHEMAFULL PERMISSIONS FOR select WHERE fn::mt154_account_row(owner_account_id, 'fs.read') FOR create, update, delete WHERE fn::mt154_account_row(owner_account_id, 'fs.write');\nDEFINE FIELD OVERWRITE owner_account_id ON TABLE atelier_contact_sheet_raster_export_plan TYPE option<record<local_accounts>> DEFAULT $auth.account_id READONLY;\n", "DEFINE TABLE OVERWRITE atelier_contact_sheet_raster_export_plan SCHEMAFULL PERMISSIONS NONE;\n"),
    ("DEFINE TABLE OVERWRITE atelier_character_document SCHEMAFULL PERMISSIONS FOR select WHERE fn::mt154_account_row(owner_account_id, 'fs.read') FOR create, update, delete WHERE fn::mt154_account_row(owner_account_id, 'fs.write');\nDEFINE FIELD OVERWRITE owner_account_id ON TABLE atelier_character_document TYPE option<record<local_accounts>> DEFAULT $auth.account_id READONLY;\n", "DEFINE TABLE OVERWRITE atelier_character_document SCHEMAFULL PERMISSIONS NONE;\n"),
    ("DEFINE TABLE OVERWRITE atelier_character_document_version SCHEMAFULL PERMISSIONS FOR select WHERE fn::mt154_account_row(owner_account_id, 'fs.read') FOR create, update, delete WHERE fn::mt154_account_row(owner_account_id, 'fs.write');\nDEFINE FIELD OVERWRITE owner_account_id ON TABLE atelier_character_document_version TYPE option<record<local_accounts>> DEFAULT $auth.account_id READONLY;\n", "DEFINE TABLE OVERWRITE atelier_character_document_version SCHEMAFULL PERMISSIONS NONE;\n"),
    ("DEFINE TABLE OVERWRITE atelier_story_card SCHEMAFULL PERMISSIONS FOR select WHERE fn::mt154_account_row(owner_account_id, 'fs.read') FOR create, update, delete WHERE fn::mt154_account_row(owner_account_id, 'fs.write');\nDEFINE FIELD OVERWRITE owner_account_id ON TABLE atelier_story_card TYPE option<record<local_accounts>> DEFAULT $auth.account_id READONLY;\n", "DEFINE TABLE OVERWRITE atelier_story_card SCHEMAFULL PERMISSIONS NONE;\n"),
    ("DEFINE TABLE OVERWRITE atelier_story_beat SCHEMAFULL PERMISSIONS FOR select WHERE fn::mt154_account_row(owner_account_id, 'fs.read') FOR create, update, delete WHERE fn::mt154_account_row(owner_account_id, 'fs.write');\nDEFINE FIELD OVERWRITE owner_account_id ON TABLE atelier_story_beat TYPE option<record<local_accounts>> DEFAULT $auth.account_id READONLY;\n", "DEFINE TABLE OVERWRITE atelier_story_beat SCHEMAFULL PERMISSIONS NONE;\n"),
    ("DEFINE TABLE OVERWRITE atelier_character_script SCHEMAFULL PERMISSIONS FOR select WHERE fn::mt154_account_row(owner_account_id, 'fs.read') FOR create, update, delete WHERE fn::mt154_account_row(owner_account_id, 'fs.write');\nDEFINE FIELD OVERWRITE owner_account_id ON TABLE atelier_character_script TYPE option<record<local_accounts>> DEFAULT $auth.account_id READONLY;\n", "DEFINE TABLE OVERWRITE atelier_character_script SCHEMAFULL PERMISSIONS NONE;\n"),
    ("DEFINE TABLE OVERWRITE atelier_bracket_link_projection SCHEMAFULL PERMISSIONS FOR select WHERE fn::mt154_account_row(owner_account_id, 'fs.read') FOR create, update, delete WHERE fn::mt154_account_row(owner_account_id, 'fs.write');\nDEFINE FIELD OVERWRITE owner_account_id ON TABLE atelier_bracket_link_projection TYPE option<record<local_accounts>> DEFAULT $auth.account_id READONLY;\n", "DEFINE TABLE OVERWRITE atelier_bracket_link_projection SCHEMAFULL PERMISSIONS NONE;\n"),
    ("DEFINE TABLE OVERWRITE atelier_moodboard SCHEMAFULL PERMISSIONS FOR select WHERE fn::mt154_account_row(owner_account_id, 'fs.read') FOR create, update, delete WHERE fn::mt154_account_row(owner_account_id, 'fs.write');\nDEFINE FIELD OVERWRITE owner_account_id ON TABLE atelier_moodboard TYPE option<record<local_accounts>> DEFAULT $auth.account_id READONLY;\n", "DEFINE TABLE OVERWRITE atelier_moodboard SCHEMAFULL PERMISSIONS NONE;\n"),
    ("DEFINE TABLE OVERWRITE atelier_moodboard_operation_receipt SCHEMAFULL PERMISSIONS FOR select WHERE fn::mt154_account_row(owner_account_id, 'fs.read') FOR create, update, delete WHERE fn::mt154_account_row(owner_account_id, 'fs.write');\nDEFINE FIELD OVERWRITE owner_account_id ON TABLE atelier_moodboard_operation_receipt TYPE option<record<local_accounts>> DEFAULT $auth.account_id READONLY;\n", "DEFINE TABLE OVERWRITE atelier_moodboard_operation_receipt SCHEMAFULL PERMISSIONS NONE;\n"),
    ("DEFINE TABLE OVERWRITE atelier_moodboard_export_request SCHEMAFULL PERMISSIONS FOR select WHERE fn::mt154_account_row(owner_account_id, 'fs.read') FOR create, update, delete WHERE fn::mt154_account_row(owner_account_id, 'fs.write');\nDEFINE FIELD OVERWRITE owner_account_id ON TABLE atelier_moodboard_export_request TYPE option<record<local_accounts>> DEFAULT $auth.account_id READONLY;\n", "DEFINE TABLE OVERWRITE atelier_moodboard_export_request SCHEMAFULL PERMISSIONS NONE;\n"),
    ("DEFINE TABLE OVERWRITE atelier_character_relationship SCHEMAFULL PERMISSIONS FOR select WHERE fn::mt154_account_row(owner_account_id, 'fs.read') FOR create, update, delete WHERE fn::mt154_account_row(owner_account_id, 'fs.write');\nDEFINE FIELD OVERWRITE owner_account_id ON TABLE atelier_character_relationship TYPE option<record<local_accounts>> DEFAULT $auth.account_id READONLY;\n", "DEFINE TABLE OVERWRITE atelier_character_relationship SCHEMAFULL PERMISSIONS NONE;\n"),
    ("DEFINE TABLE OVERWRITE atelier_saved_search SCHEMAFULL PERMISSIONS FOR select WHERE fn::mt154_account_row(owner_account_id, 'fs.read') FOR create, update, delete WHERE fn::mt154_account_row(owner_account_id, 'fs.write');\nDEFINE FIELD OVERWRITE owner_account_id ON TABLE atelier_saved_search TYPE option<record<local_accounts>> DEFAULT $auth.account_id READONLY;\n", "DEFINE TABLE OVERWRITE atelier_saved_search SCHEMAFULL PERMISSIONS NONE;\n"),
    ("DEFINE TABLE OVERWRITE atelier_saved_search_retrieval_projection SCHEMAFULL PERMISSIONS FOR select WHERE fn::mt154_account_row(owner_account_id, 'fs.read') FOR create, update, delete WHERE fn::mt154_account_row(owner_account_id, 'fs.write');\nDEFINE FIELD OVERWRITE owner_account_id ON TABLE atelier_saved_search_retrieval_projection TYPE option<record<local_accounts>> DEFAULT $auth.account_id READONLY;\n", "DEFINE TABLE OVERWRITE atelier_saved_search_retrieval_projection SCHEMAFULL PERMISSIONS NONE;\n"),
    ("DEFINE TABLE OVERWRITE atelier_web_portfolio_export_request SCHEMAFULL PERMISSIONS FOR select WHERE fn::mt154_account_row(owner_account_id, 'fs.read') FOR create, update, delete WHERE fn::mt154_account_row(owner_account_id, 'fs.write');\nDEFINE FIELD OVERWRITE owner_account_id ON TABLE atelier_web_portfolio_export_request TYPE option<record<local_accounts>> DEFAULT $auth.account_id READONLY;\n", "DEFINE TABLE OVERWRITE atelier_web_portfolio_export_request SCHEMAFULL PERMISSIONS NONE;\n"),
    ("DEFINE TABLE OVERWRITE atelier_web_portfolio_export_result SCHEMAFULL PERMISSIONS FOR select WHERE fn::mt154_account_row(owner_account_id, 'fs.read') FOR create, update, delete WHERE fn::mt154_account_row(owner_account_id, 'fs.write');\nDEFINE FIELD OVERWRITE owner_account_id ON TABLE atelier_web_portfolio_export_result TYPE option<record<local_accounts>> DEFAULT $auth.account_id READONLY;\n", "DEFINE TABLE OVERWRITE atelier_web_portfolio_export_result SCHEMAFULL PERMISSIONS NONE;\n"),
    ("DEFINE TABLE OVERWRITE atelier_backup_manifest SCHEMAFULL PERMISSIONS FOR select WHERE fn::mt154_account_row(owner_account_id, 'fs.read') FOR create, update, delete WHERE fn::mt154_account_row(owner_account_id, 'fs.write');\nDEFINE FIELD OVERWRITE owner_account_id ON TABLE atelier_backup_manifest TYPE option<record<local_accounts>> DEFAULT $auth.account_id READONLY;\n", "DEFINE TABLE OVERWRITE atelier_backup_manifest SCHEMAFULL PERMISSIONS NONE;\n"),
    ("DEFINE TABLE OVERWRITE atelier_backup_restore_preflight SCHEMAFULL PERMISSIONS FOR select WHERE fn::mt154_account_row(owner_account_id, 'fs.read') FOR create, update, delete WHERE fn::mt154_account_row(owner_account_id, 'fs.write');\nDEFINE FIELD OVERWRITE owner_account_id ON TABLE atelier_backup_restore_preflight TYPE option<record<local_accounts>> DEFAULT $auth.account_id READONLY;\n", "DEFINE TABLE OVERWRITE atelier_backup_restore_preflight SCHEMAFULL PERMISSIONS NONE;\n"),
    ("DEFINE TABLE OVERWRITE atelier_state_probe_catalog_entry SCHEMAFULL PERMISSIONS FOR select WHERE fn::mt154_account_row(owner_account_id, 'fs.read') FOR create, update, delete WHERE fn::mt154_account_row(owner_account_id, 'fs.write');\nDEFINE FIELD OVERWRITE owner_account_id ON TABLE atelier_state_probe_catalog_entry TYPE option<record<local_accounts>> DEFAULT $auth.account_id READONLY;\n", "DEFINE TABLE OVERWRITE atelier_state_probe_catalog_entry SCHEMAFULL PERMISSIONS NONE;\n"),
    ("DEFINE TABLE OVERWRITE atelier_action_receipt SCHEMAFULL PERMISSIONS FOR select WHERE fn::mt154_account_row(owner_account_id, 'fs.read') FOR create, update, delete WHERE fn::mt154_account_row(owner_account_id, 'fs.write');\nDEFINE FIELD OVERWRITE owner_account_id ON TABLE atelier_action_receipt TYPE option<record<local_accounts>> DEFAULT $auth.account_id READONLY;\n", "DEFINE TABLE OVERWRITE atelier_action_receipt SCHEMAFULL PERMISSIONS NONE;\n"),
    ("DEFINE TABLE OVERWRITE atelier_reset_operation SCHEMAFULL PERMISSIONS FOR select WHERE fn::mt154_account_row(owner_account_id, 'fs.read') FOR create, update, delete WHERE fn::mt154_account_row(owner_account_id, 'fs.write');\nDEFINE FIELD OVERWRITE owner_account_id ON TABLE atelier_reset_operation TYPE option<record<local_accounts>> DEFAULT $auth.account_id READONLY;\n", "DEFINE TABLE OVERWRITE atelier_reset_operation SCHEMAFULL PERMISSIONS NONE;\n"),
    ("DEFINE TABLE OVERWRITE atelier_orphan_manifest SCHEMAFULL PERMISSIONS FOR select WHERE fn::mt154_account_row(owner_account_id, 'fs.read') FOR create, update, delete WHERE fn::mt154_account_row(owner_account_id, 'fs.write');\nDEFINE FIELD OVERWRITE owner_account_id ON TABLE atelier_orphan_manifest TYPE option<record<local_accounts>> DEFAULT $auth.account_id READONLY;\n", "DEFINE TABLE OVERWRITE atelier_orphan_manifest SCHEMAFULL PERMISSIONS NONE;\n"),
    ("DEFINE TABLE OVERWRITE atelier_orphan_manifest_item SCHEMAFULL PERMISSIONS FOR select WHERE fn::mt154_account_row(owner_account_id, 'fs.read') FOR create, update, delete WHERE fn::mt154_account_row(owner_account_id, 'fs.write');\nDEFINE FIELD OVERWRITE owner_account_id ON TABLE atelier_orphan_manifest_item TYPE option<record<local_accounts>> DEFAULT $auth.account_id READONLY;\n", "DEFINE TABLE OVERWRITE atelier_orphan_manifest_item SCHEMAFULL PERMISSIONS NONE;\n"),
    ("DEFINE TABLE OVERWRITE atelier_pose_sidecar SCHEMAFULL PERMISSIONS FOR select WHERE fn::mt154_account_row(owner_account_id, 'fs.read') FOR create, update, delete WHERE fn::mt154_account_row(owner_account_id, 'fs.write');\nDEFINE FIELD OVERWRITE owner_account_id ON TABLE atelier_pose_sidecar TYPE option<record<local_accounts>> DEFAULT $auth.account_id READONLY;\n", "DEFINE TABLE OVERWRITE atelier_pose_sidecar SCHEMAFULL PERMISSIONS NONE;\n"),
    ("DEFINE TABLE OVERWRITE atelier_pose_context_state SCHEMAFULL PERMISSIONS FOR select WHERE fn::mt154_account_row(owner_account_id, 'fs.read') FOR create, update, delete WHERE fn::mt154_account_row(owner_account_id, 'fs.write');\nDEFINE FIELD OVERWRITE owner_account_id ON TABLE atelier_pose_context_state TYPE option<record<local_accounts>> DEFAULT $auth.account_id READONLY;\n", "DEFINE TABLE OVERWRITE atelier_pose_context_state SCHEMAFULL PERMISSIONS NONE;\n"),
    ("DEFINE TABLE OVERWRITE atelier_pose_workspace_rig_state SCHEMAFULL PERMISSIONS FOR select WHERE fn::mt154_account_row(owner_account_id, 'fs.read') FOR create, update, delete WHERE fn::mt154_account_row(owner_account_id, 'fs.write');\nDEFINE FIELD OVERWRITE owner_account_id ON TABLE atelier_pose_workspace_rig_state TYPE option<record<local_accounts>> DEFAULT $auth.account_id READONLY;\n", "DEFINE TABLE OVERWRITE atelier_pose_workspace_rig_state SCHEMAFULL PERMISSIONS NONE;\n"),
    ("DEFINE TABLE OVERWRITE atelier_identity_crop_artifact SCHEMAFULL PERMISSIONS FOR select WHERE fn::mt154_account_row(owner_account_id, 'fs.read') FOR create, update, delete WHERE fn::mt154_account_row(owner_account_id, 'fs.write');\nDEFINE FIELD OVERWRITE owner_account_id ON TABLE atelier_identity_crop_artifact TYPE option<record<local_accounts>> DEFAULT $auth.account_id READONLY;\n", "DEFINE TABLE OVERWRITE atelier_identity_crop_artifact SCHEMAFULL PERMISSIONS NONE;\n"),
    ("DEFINE TABLE OVERWRITE atelier_comfy_workflow_receipt SCHEMAFULL PERMISSIONS FOR select WHERE fn::mt154_account_row(owner_account_id, 'fs.read') FOR create, update, delete WHERE fn::mt154_account_row(owner_account_id, 'fs.write');\nDEFINE FIELD OVERWRITE owner_account_id ON TABLE atelier_comfy_workflow_receipt TYPE option<record<local_accounts>> DEFAULT $auth.account_id READONLY;\n", "DEFINE TABLE OVERWRITE atelier_comfy_workflow_receipt SCHEMAFULL PERMISSIONS NONE;\n"),
    ("DEFINE TABLE OVERWRITE atelier_comfy_output_registration_failure SCHEMAFULL PERMISSIONS FOR select WHERE fn::mt154_account_row(owner_account_id, 'fs.read') FOR create, update, delete WHERE fn::mt154_account_row(owner_account_id, 'fs.write');\nDEFINE FIELD OVERWRITE owner_account_id ON TABLE atelier_comfy_output_registration_failure TYPE option<record<local_accounts>> DEFAULT $auth.account_id READONLY;\n", "DEFINE TABLE OVERWRITE atelier_comfy_output_registration_failure SCHEMAFULL PERMISSIONS NONE;\n"),
    ("DEFINE TABLE OVERWRITE atelier_pose_deferred_feature SCHEMAFULL PERMISSIONS FOR select WHERE fn::mt154_account_row(owner_account_id, 'fs.read') FOR create, update, delete WHERE fn::mt154_account_row(owner_account_id, 'fs.write');\nDEFINE FIELD OVERWRITE owner_account_id ON TABLE atelier_pose_deferred_feature TYPE option<record<local_accounts>> DEFAULT $auth.account_id READONLY;\n", "DEFINE TABLE OVERWRITE atelier_pose_deferred_feature SCHEMAFULL PERMISSIONS NONE;\n"),
    ("DEFINE TABLE OVERWRITE atelier_comfy_workflow_spec SCHEMAFULL PERMISSIONS FOR select WHERE fn::mt154_account_row(owner_account_id, 'fs.read') FOR create, update, delete WHERE fn::mt154_account_row(owner_account_id, 'fs.write');\nDEFINE FIELD OVERWRITE owner_account_id ON TABLE atelier_comfy_workflow_spec TYPE option<record<local_accounts>> DEFAULT $auth.account_id READONLY;\n", "DEFINE TABLE OVERWRITE atelier_comfy_workflow_spec SCHEMAFULL PERMISSIONS NONE;\n"),
    ("DEFINE TABLE OVERWRITE atelier_comfy_version_metadata SCHEMAFULL PERMISSIONS FOR select WHERE fn::mt154_account_row(owner_account_id, 'fs.read') FOR create, update, delete WHERE fn::mt154_account_row(owner_account_id, 'fs.write');\nDEFINE FIELD OVERWRITE owner_account_id ON TABLE atelier_comfy_version_metadata TYPE option<record<local_accounts>> DEFAULT $auth.account_id READONLY;\n", "DEFINE TABLE OVERWRITE atelier_comfy_version_metadata SCHEMAFULL PERMISSIONS NONE;\n"),
    ("DEFINE TABLE OVERWRITE atelier_comfy_job SCHEMAFULL PERMISSIONS FOR select WHERE fn::mt154_account_row(owner_account_id, 'fs.read') FOR create, update, delete WHERE fn::mt154_account_row(owner_account_id, 'fs.write');\nDEFINE FIELD OVERWRITE owner_account_id ON TABLE atelier_comfy_job TYPE option<record<local_accounts>> DEFAULT $auth.account_id READONLY;\n", "DEFINE TABLE OVERWRITE atelier_comfy_job SCHEMAFULL PERMISSIONS NONE;\n"),
    ("DEFINE TABLE OVERWRITE atelier_comfy_diagnostic_bundle SCHEMAFULL PERMISSIONS FOR select WHERE fn::mt154_account_row(owner_account_id, 'fs.read') FOR create, update, delete WHERE fn::mt154_account_row(owner_account_id, 'fs.write');\nDEFINE FIELD OVERWRITE owner_account_id ON TABLE atelier_comfy_diagnostic_bundle TYPE option<record<local_accounts>> DEFAULT $auth.account_id READONLY;\n", "DEFINE TABLE OVERWRITE atelier_comfy_diagnostic_bundle SCHEMAFULL PERMISSIONS NONE;\n"),
    ("DEFINE TABLE OVERWRITE atelier_diagnostics_validation_matrix SCHEMAFULL PERMISSIONS FOR select WHERE fn::mt154_account_row(owner_account_id, 'fs.read') FOR create, update, delete WHERE fn::mt154_account_row(owner_account_id, 'fs.write');\nDEFINE FIELD OVERWRITE owner_account_id ON TABLE atelier_diagnostics_validation_matrix TYPE option<record<local_accounts>> DEFAULT $auth.account_id READONLY;\n", "DEFINE TABLE OVERWRITE atelier_diagnostics_validation_matrix SCHEMAFULL PERMISSIONS NONE;\n"),
    ("DEFINE TABLE OVERWRITE atelier_diagnostics_error_taxonomy SCHEMAFULL PERMISSIONS FOR select WHERE fn::mt154_account_row(owner_account_id, 'fs.read') FOR create, update, delete WHERE fn::mt154_account_row(owner_account_id, 'fs.write');\nDEFINE FIELD OVERWRITE owner_account_id ON TABLE atelier_diagnostics_error_taxonomy TYPE option<record<local_accounts>> DEFAULT $auth.account_id READONLY;\n", "DEFINE TABLE OVERWRITE atelier_diagnostics_error_taxonomy SCHEMAFULL PERMISSIONS NONE;\n"),
    ("DEFINE TABLE OVERWRITE atelier_diagnostics_prompt_response_matrix SCHEMAFULL PERMISSIONS FOR select WHERE fn::mt154_account_row(owner_account_id, 'fs.read') FOR create, update, delete WHERE fn::mt154_account_row(owner_account_id, 'fs.write');\nDEFINE FIELD OVERWRITE owner_account_id ON TABLE atelier_diagnostics_prompt_response_matrix TYPE option<record<local_accounts>> DEFAULT $auth.account_id READONLY;\n", "DEFINE TABLE OVERWRITE atelier_diagnostics_prompt_response_matrix SCHEMAFULL PERMISSIONS NONE;\n"),
    ("DEFINE TABLE OVERWRITE atelier_command_log SCHEMAFULL PERMISSIONS FOR select WHERE fn::mt154_account_row(owner_account_id, 'fs.read') FOR create, update, delete WHERE fn::mt154_account_row(owner_account_id, 'fs.write');\nDEFINE FIELD OVERWRITE owner_account_id ON TABLE atelier_command_log TYPE option<record<local_accounts>> DEFAULT $auth.account_id READONLY;\n", "DEFINE TABLE OVERWRITE atelier_command_log SCHEMAFULL PERMISSIONS NONE;\n"),
    ("DEFINE TABLE OVERWRITE atelier_diagnostics_session SCHEMAFULL PERMISSIONS FOR select WHERE fn::mt154_account_row(owner_account_id, 'fs.read') FOR create, update, delete WHERE fn::mt154_account_row(owner_account_id, 'fs.write');\nDEFINE FIELD OVERWRITE owner_account_id ON TABLE atelier_diagnostics_session TYPE option<record<local_accounts>> DEFAULT $auth.account_id READONLY;\n", "DEFINE TABLE OVERWRITE atelier_diagnostics_session SCHEMAFULL PERMISSIONS NONE;\n"),
    ("DEFINE TABLE OVERWRITE atelier_model_config SCHEMAFULL PERMISSIONS FOR select WHERE fn::mt154_account_row(owner_account_id, 'fs.read') FOR create, update, delete WHERE fn::mt154_account_row(owner_account_id, 'fs.write');\nDEFINE FIELD OVERWRITE owner_account_id ON TABLE atelier_model_config TYPE option<record<local_accounts>> DEFAULT $auth.account_id READONLY;\n", "DEFINE TABLE OVERWRITE atelier_model_config SCHEMAFULL PERMISSIONS NONE;\n"),
    ("DEFINE TABLE OVERWRITE atelier_model_apply SCHEMAFULL PERMISSIONS FOR select WHERE fn::mt154_account_row(owner_account_id, 'fs.read') FOR create, update, delete WHERE fn::mt154_account_row(owner_account_id, 'fs.write');\nDEFINE FIELD OVERWRITE owner_account_id ON TABLE atelier_model_apply TYPE option<record<local_accounts>> DEFAULT $auth.account_id READONLY;\n", "DEFINE TABLE OVERWRITE atelier_model_apply SCHEMAFULL PERMISSIONS NONE;\n"),
    ("DEFINE TABLE OVERWRITE atelier_synthetic_input_guard SCHEMAFULL PERMISSIONS FOR select WHERE fn::mt154_account_row(owner_account_id, 'fs.read') FOR create, update, delete WHERE fn::mt154_account_row(owner_account_id, 'fs.write');\nDEFINE FIELD OVERWRITE owner_account_id ON TABLE atelier_synthetic_input_guard TYPE option<record<local_accounts>> DEFAULT $auth.account_id READONLY;\n", "DEFINE TABLE OVERWRITE atelier_synthetic_input_guard SCHEMAFULL PERMISSIONS NONE;\n"),
    ("DEFINE TABLE OVERWRITE atelier_work_state_projection SCHEMAFULL PERMISSIONS FOR select WHERE fn::mt154_account_row(owner_account_id, 'fs.read') FOR create, update, delete WHERE fn::mt154_account_row(owner_account_id, 'fs.write');\nDEFINE FIELD OVERWRITE owner_account_id ON TABLE atelier_work_state_projection TYPE option<record<local_accounts>> DEFAULT $auth.account_id READONLY;\n", "DEFINE TABLE OVERWRITE atelier_work_state_projection SCHEMAFULL PERMISSIONS NONE;\n"),
    ("DEFINE TABLE OVERWRITE atelier_dcc_panel_projection SCHEMAFULL PERMISSIONS FOR select WHERE fn::mt154_account_row(owner_account_id, 'fs.read') FOR create, update, delete WHERE fn::mt154_account_row(owner_account_id, 'fs.write');\nDEFINE FIELD OVERWRITE owner_account_id ON TABLE atelier_dcc_panel_projection TYPE option<record<local_accounts>> DEFAULT $auth.account_id READONLY;\n", "DEFINE TABLE OVERWRITE atelier_dcc_panel_projection SCHEMAFULL PERMISSIONS NONE;\n"),
    ("DEFINE TABLE OVERWRITE atelier_screenshot_artifact_storage SCHEMAFULL PERMISSIONS FOR select WHERE fn::mt154_account_row(owner_account_id, 'fs.read') FOR create, update, delete WHERE fn::mt154_account_row(owner_account_id, 'fs.write');\nDEFINE FIELD OVERWRITE owner_account_id ON TABLE atelier_screenshot_artifact_storage TYPE option<record<local_accounts>> DEFAULT $auth.account_id READONLY;\n", "DEFINE TABLE OVERWRITE atelier_screenshot_artifact_storage SCHEMAFULL PERMISSIONS NONE;\n"),
    ("DEFINE TABLE OVERWRITE atelier_spec_drift_finding SCHEMAFULL PERMISSIONS FOR select WHERE fn::mt154_account_row(owner_account_id, 'fs.read') FOR create, update, delete WHERE fn::mt154_account_row(owner_account_id, 'fs.write');\nDEFINE FIELD OVERWRITE owner_account_id ON TABLE atelier_spec_drift_finding TYPE option<record<local_accounts>> DEFAULT $auth.account_id READONLY;\n", "DEFINE TABLE OVERWRITE atelier_spec_drift_finding SCHEMAFULL PERMISSIONS NONE;\n"),
    ("DEFINE TABLE OVERWRITE atelier_dcc_workflow_panel_projection SCHEMAFULL PERMISSIONS FOR select WHERE fn::mt154_account_row(owner_account_id, 'fs.read') FOR create, update, delete WHERE fn::mt154_account_row(owner_account_id, 'fs.write');\nDEFINE FIELD OVERWRITE owner_account_id ON TABLE atelier_dcc_workflow_panel_projection TYPE option<record<local_accounts>> DEFAULT $auth.account_id READONLY;\n", "DEFINE TABLE OVERWRITE atelier_dcc_workflow_panel_projection SCHEMAFULL PERMISSIONS NONE;\n"),
    ("DEFINE TABLE OVERWRITE atelier_fr_workflow_event SCHEMAFULL PERMISSIONS FOR select WHERE fn::mt154_account_row(owner_account_id, 'fs.read') FOR create, update, delete WHERE fn::mt154_account_row(owner_account_id, 'fs.write');\nDEFINE FIELD OVERWRITE owner_account_id ON TABLE atelier_fr_workflow_event TYPE option<record<local_accounts>> DEFAULT $auth.account_id READONLY;\n", "DEFINE TABLE OVERWRITE atelier_fr_workflow_event SCHEMAFULL PERMISSIONS NONE;\n"),
    ("DEFINE TABLE OVERWRITE atelier_model_manual_section SCHEMAFULL PERMISSIONS FOR select WHERE fn::mt154_account_row(owner_account_id, 'fs.read') FOR create, update, delete WHERE fn::mt154_account_row(owner_account_id, 'fs.write');\nDEFINE FIELD OVERWRITE owner_account_id ON TABLE atelier_model_manual_section TYPE option<record<local_accounts>> DEFAULT $auth.account_id READONLY;\n", "DEFINE TABLE OVERWRITE atelier_model_manual_section SCHEMAFULL PERMISSIONS NONE;\n"),
    ("DEFINE TABLE OVERWRITE atelier_retrieval_policy SCHEMAFULL PERMISSIONS FOR select WHERE fn::mt154_account_row(owner_account_id, 'fs.read') FOR create, update, delete WHERE fn::mt154_account_row(owner_account_id, 'fs.write');\nDEFINE FIELD OVERWRITE owner_account_id ON TABLE atelier_retrieval_policy TYPE option<record<local_accounts>> DEFAULT $auth.account_id READONLY;\n", "DEFINE TABLE OVERWRITE atelier_retrieval_policy SCHEMAFULL PERMISSIONS NONE;\n"),
    ("DEFINE TABLE OVERWRITE atelier_self_improve_sandbox_run SCHEMAFULL PERMISSIONS FOR select WHERE fn::mt154_account_row(owner_account_id, 'fs.read') FOR create, update, delete WHERE fn::mt154_account_row(owner_account_id, 'fs.write');\nDEFINE FIELD OVERWRITE owner_account_id ON TABLE atelier_self_improve_sandbox_run TYPE option<record<local_accounts>> DEFAULT $auth.account_id READONLY;\n", "DEFINE TABLE OVERWRITE atelier_self_improve_sandbox_run SCHEMAFULL PERMISSIONS NONE;\n"),
    ("DEFINE TABLE OVERWRITE atelier_validator_first_pass_run SCHEMAFULL PERMISSIONS FOR select WHERE fn::mt154_account_row(owner_account_id, 'fs.read') FOR create, update, delete WHERE fn::mt154_account_row(owner_account_id, 'fs.write');\nDEFINE FIELD OVERWRITE owner_account_id ON TABLE atelier_validator_first_pass_run TYPE option<record<local_accounts>> DEFAULT $auth.account_id READONLY;\n", "DEFINE TABLE OVERWRITE atelier_validator_first_pass_run SCHEMAFULL PERMISSIONS NONE;\n"),
    ("DEFINE TABLE OVERWRITE atelier_model_coordination_lease SCHEMAFULL PERMISSIONS FOR select WHERE fn::mt154_account_row(owner_account_id, 'fs.read') FOR create, update, delete WHERE fn::mt154_account_row(owner_account_id, 'fs.write');\nDEFINE FIELD OVERWRITE owner_account_id ON TABLE atelier_model_coordination_lease TYPE option<record<local_accounts>> DEFAULT $auth.account_id READONLY;\n", "DEFINE TABLE OVERWRITE atelier_model_coordination_lease SCHEMAFULL PERMISSIONS NONE;\n"),
    ("DEFINE TABLE OVERWRITE atelier_model_manual_row_merge SCHEMAFULL PERMISSIONS FOR select WHERE fn::mt154_account_row(owner_account_id, 'fs.read') FOR create, update, delete WHERE fn::mt154_account_row(owner_account_id, 'fs.write');\nDEFINE FIELD OVERWRITE owner_account_id ON TABLE atelier_model_manual_row_merge TYPE option<record<local_accounts>> DEFAULT $auth.account_id READONLY;\n", "DEFINE TABLE OVERWRITE atelier_model_manual_row_merge SCHEMAFULL PERMISSIONS NONE;\n"),
    ("DEFINE TABLE OVERWRITE atelier_model_manual_drift_guard SCHEMAFULL PERMISSIONS FOR select WHERE fn::mt154_account_row(owner_account_id, 'fs.read') FOR create, update, delete WHERE fn::mt154_account_row(owner_account_id, 'fs.write');\nDEFINE FIELD OVERWRITE owner_account_id ON TABLE atelier_model_manual_drift_guard TYPE option<record<local_accounts>> DEFAULT $auth.account_id READONLY;\n", "DEFINE TABLE OVERWRITE atelier_model_manual_drift_guard SCHEMAFULL PERMISSIONS NONE;\n"),
    ("DEFINE TABLE OVERWRITE atelier_visual_steer_feedback SCHEMAFULL PERMISSIONS FOR select WHERE fn::mt154_account_row(owner_account_id, 'fs.read') FOR create, update, delete WHERE fn::mt154_account_row(owner_account_id, 'fs.write');\nDEFINE FIELD OVERWRITE owner_account_id ON TABLE atelier_visual_steer_feedback TYPE option<record<local_accounts>> DEFAULT $auth.account_id READONLY;\n", "DEFINE TABLE OVERWRITE atelier_visual_steer_feedback SCHEMAFULL PERMISSIONS NONE;\n"),
    ("DEFINE TABLE OVERWRITE knowledge_entities SCHEMAFULL PERMISSIONS FOR select WHERE fn::mt120_entity_read(id) OR (entity_kind = 'loom_block' AND primary_source_id = NONE AND fn::mt120_loom_block_access(entity_key, record::id(workspace_id), 'read', 'fs.read')) OR (entity_kind = 'loom_block' AND primary_source_id = NONE AND type::record('loom_blocks', entity_key).content_type = 'view_def' AND type::record('loom_blocks', entity_key).workspace_id = workspace_id AND fn::mt109_has_workspace_access(record::id(workspace_id), 'read', 'fs.read')) OR (entity_kind = 'loom_block' AND primary_source_id = NONE AND type::record('loom_blocks', entity_key).source_rich_document_id = type::record('knowledge_rich_documents', entity_key) AND type::record('knowledge_rich_documents', entity_key).workspace_id = workspace_id AND fn::mt120_document_access(entity_key, record::id(workspace_id), 'read', 'fs.read')) FOR create WHERE fn::mt120_entity_write(id, primary_source_id, workspace_id) OR (entity_kind = 'loom_block' AND primary_source_id = NONE AND fn::mt120_loom_block_access(entity_key, record::id(workspace_id), 'create', 'fs.write')) OR (entity_kind = 'loom_block' AND primary_source_id = NONE AND type::record('loom_blocks', entity_key).content_type = 'view_def' AND type::record('loom_blocks', entity_key).workspace_id = workspace_id AND fn::mt109_has_workspace_access(record::id(workspace_id), 'create', 'fs.write')) OR (entity_kind = 'loom_block' AND primary_source_id = NONE AND type::record('loom_blocks', entity_key).source_rich_document_id = type::record('knowledge_rich_documents', entity_key) AND type::record('knowledge_rich_documents', entity_key).workspace_id = workspace_id AND fn::mt120_document_access(entity_key, record::id(workspace_id), 'update', 'fs.write')) FOR update WHERE fn::mt120_entity_write(id, primary_source_id, workspace_id) FOR delete WHERE entity_kind = 'loom_block' AND primary_source_id = NONE AND fn::mt154_stage_card_projection_delete(entity_key, record::id(workspace_id)) AND fn::mt154_stage_compensation(entity_key, record::id(workspace_id)).entity_id = record::id(id);\n", "DEFINE TABLE OVERWRITE knowledge_entities SCHEMAFULL PERMISSIONS FOR select WHERE fn::mt120_entity_read(id) OR (entity_kind = 'loom_block' AND primary_source_id = NONE AND fn::mt120_loom_block_access(entity_key, record::id(workspace_id), 'read', 'fs.read')) OR (entity_kind = 'loom_block' AND primary_source_id = NONE AND type::record('loom_blocks', entity_key).content_type = 'view_def' AND type::record('loom_blocks', entity_key).workspace_id = workspace_id AND fn::mt109_has_workspace_access(record::id(workspace_id), 'read', 'fs.read')) FOR create WHERE fn::mt120_entity_write(id, primary_source_id, workspace_id) OR (entity_kind = 'loom_block' AND primary_source_id = NONE AND fn::mt120_loom_block_access(entity_key, record::id(workspace_id), 'create', 'fs.write')) OR (entity_kind = 'loom_block' AND primary_source_id = NONE AND type::record('loom_blocks', entity_key).content_type = 'view_def' AND type::record('loom_blocks', entity_key).workspace_id = workspace_id AND fn::mt109_has_workspace_access(record::id(workspace_id), 'create', 'fs.write')) FOR update WHERE fn::mt120_entity_write(id, primary_source_id, workspace_id) FOR delete NONE;\n"),
    ("DEFINE TABLE OVERWRITE knowledge_claims SCHEMAFULL PERMISSIONS FOR select WHERE fn::mt109_has_workspace_access(record::id(workspace_id), 'read', 'fs.read') FOR create, update, delete NONE;\n", "DEFINE TABLE OVERWRITE knowledge_claims SCHEMAFULL PERMISSIONS NONE;\n"),
    ("DEFINE TABLE OVERWRITE knowledge_claim_spans SCHEMAFULL PERMISSIONS FOR select WHERE fn::mt109_has_workspace_access(record::id(claim_id.workspace_id), 'read', 'fs.read') FOR create, update, delete NONE;\n", "DEFINE TABLE OVERWRITE knowledge_claim_spans SCHEMAFULL PERMISSIONS NONE;\n"),
    ("DEFINE TABLE OVERWRITE knowledge_claim_conflicts SCHEMAFULL PERMISSIONS FOR select WHERE claim_id.workspace_id = conflicting_claim_id.workspace_id AND fn::mt109_has_workspace_access(record::id(claim_id.workspace_id), 'read', 'fs.read') FOR create, update, delete NONE;\n", "DEFINE TABLE OVERWRITE knowledge_claim_conflicts SCHEMAFULL PERMISSIONS NONE;\n"),
    ("DEFINE TABLE OVERWRITE knowledge_memory_passages SCHEMAFULL PERMISSIONS FOR select WHERE fn::mt109_has_workspace_access(record::id(workspace_id), 'read', 'fs.read') FOR create, update, delete NONE;\n", "DEFINE TABLE OVERWRITE knowledge_memory_passages SCHEMAFULL PERMISSIONS NONE;\n"),
    ("DEFINE TABLE OVERWRITE knowledge_passage_evidence SCHEMAFULL PERMISSIONS FOR select WHERE fn::mt109_has_workspace_access(record::id(passage_id.workspace_id), 'read', 'fs.read') FOR create, update, delete NONE;\n", "DEFINE TABLE OVERWRITE knowledge_passage_evidence SCHEMAFULL PERMISSIONS NONE;\n"),
    ("DEFINE TABLE OVERWRITE knowledge_rich_documents SCHEMAFULL\n    PERMISSIONS FOR select WHERE (deleted_at = NONE AND (fn::mt109_source_read('rich_document', rich_document_id, record::id(workspace_id), 'workspace', record::id(workspace_id)) OR fn::mt120_document_access(rich_document_id, record::id(workspace_id), 'read', 'fs.read'))) OR (deleted_at != NONE AND fn::mt120_document_access(rich_document_id, record::id(workspace_id), 'delete', 'fs.write'))\n                FOR create WHERE fn::mt109_live_session() AND created_in_session_id = $auth.id AND deleted_at = NONE AND fn::mt109_has_workspace_access(record::id(workspace_id), 'create', 'fs.write')\n                FOR update WHERE fn::mt120_document_access(rich_document_id, record::id(workspace_id), 'update', 'fs.write') OR fn::mt120_document_delete(rich_document_id, record::id(workspace_id))\n                FOR delete WHERE fn::mt154_stage_card_projection_delete(rich_document_id, record::id(workspace_id));\n", "DEFINE TABLE OVERWRITE knowledge_rich_documents SCHEMAFULL\n    PERMISSIONS FOR select WHERE (deleted_at = NONE AND (fn::mt109_source_read('rich_document', rich_document_id, record::id(workspace_id), 'workspace', record::id(workspace_id)) OR fn::mt120_document_access(rich_document_id, record::id(workspace_id), 'read', 'fs.read'))) OR (deleted_at != NONE AND fn::mt120_document_access(rich_document_id, record::id(workspace_id), 'delete', 'fs.write'))\n                FOR create WHERE fn::mt109_live_session() AND created_in_session_id = $auth.id AND deleted_at = NONE AND fn::mt109_has_workspace_access(record::id(workspace_id), 'create', 'fs.write')\n                FOR update WHERE fn::mt120_document_access(rich_document_id, record::id(workspace_id), 'update', 'fs.write') OR fn::mt120_document_delete(rich_document_id, record::id(workspace_id))\n                FOR delete NONE;\n"),
    ("DEFINE TABLE OVERWRITE knowledge_context_bundles SCHEMAFULL PERMISSIONS FOR select WHERE fn::mt109_has_workspace_access(record::id(workspace_id), 'read', 'fs.read') FOR create WHERE fn::mt109_has_workspace_access(record::id(workspace_id), 'create', 'fs.write') AND build_receipt_event_id != NONE AND build_receipt_event_id.authority_session_id = $auth.id AND build_receipt_event_id.wsids = [record::id(workspace_id)] FOR update, delete NONE;\n", "DEFINE TABLE OVERWRITE knowledge_context_bundles SCHEMAFULL PERMISSIONS NONE;\n"),
    ("DEFINE TABLE OVERWRITE knowledge_context_bundle_items SCHEMAFULL PERMISSIONS FOR select WHERE fn::mt109_has_workspace_access(record::id(bundle_id.workspace_id), 'read', 'fs.read') FOR create WHERE fn::mt109_has_workspace_access(record::id(bundle_id.workspace_id), 'create', 'fs.write') AND bundle_id.build_receipt_event_id.authority_session_id = $auth.id FOR update, delete NONE;\n", "DEFINE TABLE OVERWRITE knowledge_context_bundle_items SCHEMAFULL PERMISSIONS NONE;\n"),
    ("DEFINE TABLE OVERWRITE knowledge_retrieval_traces SCHEMAFULL PERMISSIONS FOR select WHERE fn::mt109_has_workspace_access(record::id(workspace_id), 'read', 'fs.read') FOR create WHERE fn::mt109_has_workspace_access(record::id(workspace_id), 'create', 'fs.write') AND (bundle_id = NONE OR bundle_id.workspace_id = workspace_id) AND (trace_receipt_event_id = NONE OR trace_receipt_event_id.authority_session_id = $auth.id) FOR update WHERE fn::mt109_has_workspace_access(record::id(workspace_id), 'update', 'fs.write') AND (trace_receipt_event_id = NONE OR (trace_receipt_event_id.authority_session_id = $auth.id AND trace_receipt_event_id.wsids = [record::id(workspace_id)])) FOR delete NONE;\n", "DEFINE TABLE OVERWRITE knowledge_retrieval_traces SCHEMAFULL PERMISSIONS NONE;\n"),
    ("DEFINE TABLE OVERWRITE knowledge_crdt_denial_receipts SCHEMAFULL PERMISSIONS FOR select WHERE document_id != NONE AND fn::mt120_document_access(document_id, workspace_id, 'read', 'fs.read') FOR create WHERE document_id != NONE AND fn::mt120_document_access(document_id, workspace_id, 'update', 'fs.write') AND event_ledger_event_id.authority_session_id = $auth.id AND event_ledger_event_id.wsids = [workspace_id] FOR update, delete NONE;\n", "DEFINE TABLE OVERWRITE knowledge_crdt_denial_receipts SCHEMAFULL PERMISSIONS NONE;\n"),
    ("DEFINE TABLE OVERWRITE knowledge_memory_ontology_terms SCHEMAFULL PERMISSIONS FOR select WHERE fn::mt109_has_workspace_access(record::id(workspace_id), 'read', 'fs.read') FOR create, update, delete NONE;\n", "DEFINE TABLE OVERWRITE knowledge_memory_ontology_terms SCHEMAFULL PERMISSIONS NONE;\n"),
    ("DEFINE TABLE OVERWRITE knowledge_memory_ontology_aliases SCHEMAFULL PERMISSIONS FOR select WHERE term_id.workspace_id = workspace_id AND fn::mt109_has_workspace_access(record::id(workspace_id), 'read', 'fs.read') FOR create, update, delete NONE;\n", "DEFINE TABLE OVERWRITE knowledge_memory_ontology_aliases SCHEMAFULL PERMISSIONS NONE;\n"),
    ("DEFINE TABLE OVERWRITE knowledge_memory_facts SCHEMAFULL PERMISSIONS FOR select WHERE claim_id.workspace_id = workspace_id AND fn::mt109_has_workspace_access(record::id(workspace_id), 'read', 'fs.read') FOR create, update, delete NONE;\n", "DEFINE TABLE OVERWRITE knowledge_memory_facts SCHEMAFULL PERMISSIONS NONE;\n"),
    ("DEFINE TABLE OVERWRITE knowledge_semantic_catalog_entries SCHEMAFULL PERMISSIONS FOR select WHERE fn::mt109_has_workspace_access(record::id(workspace_id), 'read', 'fs.read') FOR create, update, delete NONE;\n", "DEFINE TABLE OVERWRITE knowledge_semantic_catalog_entries SCHEMAFULL PERMISSIONS NONE;\n"),
    ("DEFINE TABLE OVERWRITE loom_block_knowledge_bridge SCHEMAFULL\n    PERMISSIONS FOR select WHERE fn::mt120_loom_block_access(record::id(block_id), record::id(workspace_id), 'read', 'fs.read') OR (block_id.content_type = 'view_def' AND block_id.workspace_id = workspace_id AND fn::mt109_has_workspace_access(record::id(workspace_id), 'read', 'fs.read')) OR (block_id.source_rich_document_id != NONE AND record::id(block_id.source_rich_document_id) = record::id(block_id) AND block_id.workspace_id = workspace_id AND fn::mt120_document_access(record::id(block_id), record::id(workspace_id), 'read', 'fs.read'))\n                FOR create WHERE block_id.workspace_id = workspace_id AND entity_id.workspace_id = workspace_id AND entity_id.entity_kind = 'loom_block' AND entity_id.entity_key = record::id(block_id) AND fn::mt120_loom_block_access(record::id(block_id), record::id(workspace_id), 'create', 'fs.write') OR (block_id.content_type = 'view_def' AND block_id.workspace_id = workspace_id AND entity_id.workspace_id = workspace_id AND entity_id.entity_kind = 'loom_block' AND entity_id.entity_key = record::id(block_id) AND fn::mt109_has_workspace_access(record::id(workspace_id), 'create', 'fs.write')) OR (block_id.source_rich_document_id != NONE AND record::id(block_id.source_rich_document_id) = record::id(block_id) AND block_id.workspace_id = workspace_id AND entity_id.workspace_id = workspace_id AND entity_id.entity_kind = 'loom_block' AND entity_id.entity_key = record::id(block_id) AND fn::mt120_document_access(record::id(block_id), record::id(workspace_id), 'update', 'fs.write'))\n                FOR update NONE FOR delete WHERE block_id.source_rich_document_id != NONE AND record::id(block_id.source_rich_document_id) = record::id(block_id) AND fn::mt154_stage_card_projection_delete(record::id(block_id), record::id(workspace_id)) AND fn::mt154_stage_compensation(record::id(block_id), record::id(workspace_id)).entity_id = record::id(entity_id);\n", "DEFINE TABLE OVERWRITE loom_block_knowledge_bridge SCHEMAFULL\n    PERMISSIONS FOR select WHERE fn::mt120_loom_block_access(record::id(block_id), record::id(workspace_id), 'read', 'fs.read') OR (block_id.content_type = 'view_def' AND block_id.workspace_id = workspace_id AND fn::mt109_has_workspace_access(record::id(workspace_id), 'read', 'fs.read'))\n                FOR create WHERE block_id.workspace_id = workspace_id AND entity_id.workspace_id = workspace_id AND entity_id.entity_kind = 'loom_block' AND entity_id.entity_key = record::id(block_id) AND fn::mt120_loom_block_access(record::id(block_id), record::id(workspace_id), 'create', 'fs.write') OR (block_id.content_type = 'view_def' AND block_id.workspace_id = workspace_id AND entity_id.workspace_id = workspace_id AND entity_id.entity_kind = 'loom_block' AND entity_id.entity_key = record::id(block_id) AND fn::mt109_has_workspace_access(record::id(workspace_id), 'create', 'fs.write'))\n                FOR update NONE FOR delete NONE;\n"),
    ("DEFINE TABLE OVERWRITE storage_graph_anchors SCHEMAFULL PERMISSIONS FOR select WHERE (graph_kind = 'loom_folder_tree' AND anchor_key = 'loom_folder_tree|' + scope_key AND record::id(id) = anchor_key AND fn::mt109_has_workspace_access(scope_key, 'read', 'fs.read')) OR (graph_kind = 'work_packet_dependencies' AND scope_key != 'global' AND scope_key = fn::mt158_dependency_graph_scope() AND anchor_key = 'work_packet_dependencies|' + scope_key AND record::id(id) = anchor_key AND fn::mt154_account_row($auth.account_id, 'fs.read')) FOR create, update WHERE (graph_kind = 'loom_folder_tree' AND anchor_key = 'loom_folder_tree|' + scope_key AND record::id(id) = anchor_key AND fn::mt109_has_workspace_access(scope_key, 'update', 'fs.write')) OR (graph_kind = 'work_packet_dependencies' AND scope_key != 'global' AND scope_key = fn::mt158_dependency_graph_scope() AND anchor_key = 'work_packet_dependencies|' + scope_key AND record::id(id) = anchor_key AND fn::mt154_account_row($auth.account_id, 'fs.write')) FOR delete NONE;\n", "DEFINE TABLE OVERWRITE storage_graph_anchors SCHEMAFULL PERMISSIONS FOR select WHERE graph_kind = 'loom_folder_tree' AND anchor_key = 'loom_folder_tree|' + scope_key AND record::id(id) = anchor_key AND fn::mt109_has_workspace_access(scope_key, 'read', 'fs.read') FOR create, update WHERE graph_kind = 'loom_folder_tree' AND anchor_key = 'loom_folder_tree|' + scope_key AND record::id(id) = anchor_key AND fn::mt109_has_workspace_access(scope_key, 'update', 'fs.write') FOR delete NONE;\n"),
    ("-- MT-157 (Master Spec 02-system-architecture.md:2758/2773/2776, LM-RLS-002): record users read with the exact\n-- RichDocument read + fs.read grant; the replace-all PUT (DELETE + CREATE) needs update + fs.write; rows are never\n-- updated in place. Workspace/document deletes cascade through the REFERENCE fields below.\nDEFINE TABLE OVERWRITE knowledge_debug_breakpoints SCHEMAFULL\n    PERMISSIONS FOR select WHERE fn::mt120_document_access(record::id(rich_document_id), record::id(workspace_id), 'read', 'fs.read')\n                FOR create WHERE fn::mt120_document_access(record::id(rich_document_id), record::id(workspace_id), 'update', 'fs.write')\n                FOR update NONE\n                FOR delete WHERE fn::mt120_document_access(record::id(rich_document_id), record::id(workspace_id), 'update', 'fs.write') OR fn::mt120_workspace_delete(record::id(workspace_id));\n", "DEFINE TABLE OVERWRITE knowledge_debug_breakpoints SCHEMAFULL PERMISSIONS NONE;\n"),
    ("DEFINE TABLE OVERWRITE loom_canvas_placements SCHEMAFULL\n    PERMISSIONS FOR select WHERE (fn::mt120_loom_block_access(record::id(canvas_block_id), record::id(workspace_id), 'read', 'fs.read') OR fn::mt120_loom_block_access(record::id(canvas_block_id), record::id(workspace_id), 'update', 'fs.write')) AND (fn::mt120_loom_block_access(record::id(placed_block_id), record::id(workspace_id), 'read', 'fs.read') OR (placed_block_id.source_rich_document_id != NONE AND record::id(placed_block_id.source_rich_document_id) = record::id(placed_block_id) AND fn::mt120_document_access(record::id(placed_block_id), record::id(workspace_id), 'read', 'fs.read')))\n                FOR create WHERE fn::mt120_loom_block_access(record::id(canvas_block_id), record::id(workspace_id), 'update', 'fs.write') AND (fn::mt120_loom_block_access(record::id(placed_block_id), record::id(workspace_id), 'read', 'fs.read') OR (placed_block_id.source_rich_document_id != NONE AND record::id(placed_block_id.source_rich_document_id) = record::id(placed_block_id) AND fn::mt120_document_access(record::id(placed_block_id), record::id(workspace_id), 'read', 'fs.read')))\n                FOR update WHERE fn::mt120_loom_block_access(record::id(canvas_block_id), record::id(workspace_id), 'update', 'fs.write') AND (fn::mt120_loom_block_access(record::id(placed_block_id), record::id(workspace_id), 'read', 'fs.read') OR (placed_block_id.source_rich_document_id != NONE AND record::id(placed_block_id.source_rich_document_id) = record::id(placed_block_id) AND fn::mt120_document_access(record::id(placed_block_id), record::id(workspace_id), 'read', 'fs.read')))\n                FOR delete WHERE fn::mt120_workspace_delete(record::id(workspace_id)) OR (fn::mt120_loom_block_access(record::id(canvas_block_id), record::id(workspace_id), 'update', 'fs.write') AND (fn::mt120_loom_block_access(record::id(placed_block_id), record::id(workspace_id), 'read', 'fs.read') OR (placed_block_id.source_rich_document_id != NONE AND record::id(placed_block_id.source_rich_document_id) = record::id(placed_block_id) AND fn::mt120_document_access(record::id(placed_block_id), record::id(workspace_id), 'read', 'fs.read'))) AND fn::mt154_stage_placement_delete(placement_id, record::id(placed_block_id), record::id(workspace_id), stage_provenance_key));\n", "DEFINE TABLE OVERWRITE loom_canvas_placements SCHEMAFULL\n    PERMISSIONS FOR select WHERE (fn::mt120_loom_block_access(record::id(canvas_block_id), record::id(workspace_id), 'read', 'fs.read') OR fn::mt120_loom_block_access(record::id(canvas_block_id), record::id(workspace_id), 'update', 'fs.write')) AND (fn::mt120_loom_block_access(record::id(placed_block_id), record::id(workspace_id), 'read', 'fs.read') OR (placed_block_id.source_rich_document_id != NONE AND record::id(placed_block_id.source_rich_document_id) = record::id(placed_block_id) AND fn::mt120_document_access(record::id(placed_block_id), record::id(workspace_id), 'read', 'fs.read')))\n                FOR create WHERE fn::mt120_loom_block_access(record::id(canvas_block_id), record::id(workspace_id), 'update', 'fs.write') AND (fn::mt120_loom_block_access(record::id(placed_block_id), record::id(workspace_id), 'read', 'fs.read') OR (placed_block_id.source_rich_document_id != NONE AND record::id(placed_block_id.source_rich_document_id) = record::id(placed_block_id) AND fn::mt120_document_access(record::id(placed_block_id), record::id(workspace_id), 'read', 'fs.read')))\n                FOR update WHERE fn::mt120_loom_block_access(record::id(canvas_block_id), record::id(workspace_id), 'update', 'fs.write') AND (fn::mt120_loom_block_access(record::id(placed_block_id), record::id(workspace_id), 'read', 'fs.read') OR (placed_block_id.source_rich_document_id != NONE AND record::id(placed_block_id.source_rich_document_id) = record::id(placed_block_id) AND fn::mt120_document_access(record::id(placed_block_id), record::id(workspace_id), 'read', 'fs.read')))\n                FOR delete WHERE fn::mt120_workspace_delete(record::id(workspace_id)) OR (fn::mt120_loom_block_access(record::id(canvas_block_id), record::id(workspace_id), 'update', 'fs.write') AND (fn::mt120_loom_block_access(record::id(placed_block_id), record::id(workspace_id), 'read', 'fs.read') OR (placed_block_id.source_rich_document_id != NONE AND record::id(placed_block_id.source_rich_document_id) = record::id(placed_block_id) AND fn::mt120_document_access(record::id(placed_block_id), record::id(workspace_id), 'read', 'fs.read'))));\n"),
    ("-- MT-154: activity spans are workspace-scoped like the calendar rows they annotate.\nDEFINE TABLE OVERWRITE calendar_activity_spans SCHEMAFULL\n    PERMISSIONS FOR select WHERE fn::mt109_has_workspace_access(record::id(workspace_id), 'read', 'fs.read')\n                FOR create WHERE fn::mt109_has_workspace_access(record::id(workspace_id), 'create', 'fs.write')\n                FOR update WHERE fn::mt109_has_workspace_access(record::id(workspace_id), 'create', 'fs.write')\n                    OR fn::mt109_has_workspace_access(record::id(workspace_id), 'update', 'fs.write')\n                FOR delete WHERE fn::mt109_has_workspace_access(record::id(workspace_id), 'delete', 'fs.write')\n                    OR fn::mt120_workspace_delete(record::id(workspace_id));\n", "DEFINE TABLE OVERWRITE calendar_activity_spans SCHEMAFULL PERMISSIONS NONE;\n"),
    ("-- MT-154 AC-154-3 (Master Spec 02-system-architecture.md:2776, LM-RLS-001/002): workspace readers read,\n-- workspace members capture (the row's actor is the session principal), captures are immutable, and\n-- only a workspace deleter (or the owner's workspace delete) removes them.\nDEFINE TABLE OVERWRITE stage_capture_artifacts SCHEMAFULL\n    PERMISSIONS FOR select WHERE fn::mt109_has_workspace_access(record::id(workspace_id), 'read', 'fs.read')\n                FOR create WHERE fn::mt109_has_workspace_access(record::id(workspace_id), 'create', 'fs.write') AND $auth != NONE AND actor_id = $auth.principal_id.actor_id\n                FOR update NONE\n                FOR delete WHERE fn::mt120_workspace_delete(record::id(workspace_id)) OR fn::mt109_has_workspace_access(record::id(workspace_id), 'delete', 'fs.write');\n", "DEFINE TABLE OVERWRITE stage_capture_artifacts SCHEMAFULL PERMISSIONS NONE;\n"),
    ("-- MT-154 (AC-154-4, D-154-3): account-owned link; creating it additionally requires read on the exact\n-- target block (its loom_block resource or the same-id rich_document resource) so no account can pin\n-- another account's block. Delete stays with the workspace owner (workspace delete) or the link owner.\nDEFINE TABLE OVERWRITE atelier_intake_item_loom_projection SCHEMAFULL PERMISSIONS FOR select WHERE fn::mt154_account_row(owner_account_id, 'fs.read') FOR create WHERE fn::mt154_account_row(owner_account_id, 'fs.write') AND (fn::mt120_loom_block_access(record::id(loom_block_id), record::id(workspace_id), 'read', 'fs.read') OR fn::mt120_document_access(record::id(loom_block_id), record::id(workspace_id), 'read', 'fs.read')) FOR update NONE FOR delete WHERE fn::mt109_has_workspace_access(record::id(workspace_id), 'delete', 'fs.write') OR fn::mt154_account_row(owner_account_id, 'fs.write');\nDEFINE FIELD OVERWRITE owner_account_id ON TABLE atelier_intake_item_loom_projection TYPE option<record<local_accounts>> DEFAULT $auth.account_id READONLY;\n", "DEFINE TABLE OVERWRITE atelier_intake_item_loom_projection SCHEMAFULL PERMISSIONS FOR select, create, update NONE FOR delete WHERE fn::mt109_has_workspace_access(record::id(workspace_id), 'delete', 'fs.write');\n"),
    ("DEFINE TABLE OVERWRITE fems_workspace_write_anchors SCHEMAFULL\n    PERMISSIONS FOR select WHERE fn::mt109_has_workspace_access(workspace_key, 'read', 'memory.read') OR fn::mt109_has_workspace_access(workspace_key, 'create', 'memory.propose') OR fn::mt109_has_workspace_access(workspace_key, 'update', 'memory.review') OR fn::mt109_has_workspace_access(workspace_key, 'update', 'memory.commit')\n                FOR create WHERE fn::mt109_has_workspace_access(workspace_key, 'create', 'memory.propose') OR fn::mt109_has_workspace_access(workspace_key, 'create', 'memory.commit') OR fn::mt109_has_workspace_access(workspace_key, 'delete', 'fs.write')\n                FOR update WHERE fn::mt109_has_workspace_access(workspace_key, 'update', 'memory.review') OR fn::mt109_has_workspace_access(workspace_key, 'update', 'memory.commit') OR fn::mt109_has_workspace_access(workspace_key, 'delete', 'fs.write')\n                FOR delete WHERE fn::mt120_workspace_delete(workspace_key);\n", "DEFINE TABLE OVERWRITE fems_workspace_write_anchors SCHEMAFULL\n    PERMISSIONS FOR select WHERE fn::mt109_has_workspace_access(workspace_key, 'read', 'memory.read') OR fn::mt109_has_workspace_access(workspace_key, 'create', 'memory.propose') OR fn::mt109_has_workspace_access(workspace_key, 'update', 'memory.review') OR fn::mt109_has_workspace_access(workspace_key, 'update', 'memory.commit')\n                FOR create WHERE fn::mt109_has_workspace_access(workspace_key, 'create', 'memory.propose') OR fn::mt109_has_workspace_access(workspace_key, 'create', 'memory.commit')\n                FOR update WHERE fn::mt109_has_workspace_access(workspace_key, 'update', 'memory.review') OR fn::mt109_has_workspace_access(workspace_key, 'update', 'memory.commit')\n                FOR delete NONE;\n"),
    ("-- MT-154: the outbox row is bound to its workspace (string id) like the calendar mutation it records.\nDEFINE TABLE OVERWRITE calendar_mutation_outbox SCHEMAFULL\n    PERMISSIONS FOR select WHERE fn::mt109_has_workspace_access(workspace_id, 'read', 'fs.read')\n                FOR create WHERE fn::mt109_has_workspace_access(workspace_id, 'create', 'fs.write')\n                    OR fn::mt109_has_workspace_access(workspace_id, 'update', 'fs.write')\n                FOR update WHERE fn::mt109_has_workspace_access(workspace_id, 'update', 'fs.write')\n                FOR delete WHERE fn::mt109_has_workspace_access(workspace_id, 'delete', 'fs.write')\n                    OR fn::mt120_workspace_delete(workspace_id);\n", "DEFINE TABLE OVERWRITE calendar_mutation_outbox SCHEMAFULL PERMISSIONS NONE;\n"),
    ("-- MT-154 (AC-154-3, D-154-3): workspace-scope rows follow the workspace grant; global/surface rows are\n-- account-owned (owner_account_id stamped from $auth at create, never rewritten).\nDEFINE TABLE OVERWRITE preference_records SCHEMAFULL\n    PERMISSIONS FOR select WHERE (scope_kind = 'workspace' AND fn::mt109_has_workspace_access(scope_ref, 'read', 'fs.read'))\n                    OR (scope_kind != 'workspace' AND fn::mt154_account_row(owner_account_id, 'fs.read'))\n                FOR create, update WHERE (scope_kind = 'workspace' AND fn::mt109_has_workspace_access(scope_ref, 'update', 'fs.write'))\n                    OR (scope_kind != 'workspace' AND fn::mt154_account_row(owner_account_id, 'fs.write'))\n                FOR delete WHERE (scope_kind = 'workspace' AND fn::mt109_has_workspace_access(scope_ref, 'delete', 'fs.write'))\n                    OR (scope_kind != 'workspace' AND fn::mt154_account_row(owner_account_id, 'fs.write'));\nDEFINE FIELD OVERWRITE owner_account_id ON TABLE preference_records TYPE option<record<local_accounts>> DEFAULT $auth.account_id READONLY;\n", "DEFINE TABLE OVERWRITE preference_records SCHEMAFULL PERMISSIONS NONE;\n"),
    ("DEFINE TABLE OVERWRITE preference_change_receipts SCHEMAFULL\n    PERMISSIONS FOR select WHERE (scope_kind = 'workspace' AND fn::mt109_has_workspace_access(scope_ref, 'read', 'fs.read'))\n                    OR (scope_kind != 'workspace' AND fn::mt154_account_row(owner_account_id, 'fs.read'))\n                FOR create WHERE (scope_kind = 'workspace' AND fn::mt109_has_workspace_access(scope_ref, 'update', 'fs.write'))\n                    OR (scope_kind != 'workspace' AND fn::mt154_account_row(owner_account_id, 'fs.write'))\n                FOR update NONE\n                FOR delete WHERE (scope_kind = 'workspace' AND fn::mt109_has_workspace_access(scope_ref, 'delete', 'fs.write'))\n                    OR (scope_kind != 'workspace' AND fn::mt154_account_row(owner_account_id, 'fs.write'));\nDEFINE FIELD OVERWRITE owner_account_id ON TABLE preference_change_receipts TYPE option<record<local_accounts>> DEFAULT $auth.account_id READONLY;\n", "DEFINE TABLE OVERWRITE preference_change_receipts SCHEMAFULL PERMISSIONS NONE;\n"),
    ("DEFINE FUNCTION OVERWRITE fn::mt109_ledger_receipt($capability: option<string>, $event_type: string, $source: string, $aggregate: string, $payload: object, $wsids: array<string>) {\n    RETURN array::len($wsids) = 1 AND (\n        ($capability = 'fr.ingest.native_editor' AND $aggregate = 'native_editor_event'\n            AND $source = 'native_editor_fr_ingestion' AND $payload.envelope.workspace_id = $wsids[0]\n            AND (($payload.receipt_kind = 'native_editor_flight_recorder_pending' AND $event_type = 'FLIGHT_RECORDER_MIRROR_PENDING')\n                 OR ($payload.receipt_kind = 'native_editor_flight_recorder_recorded' AND $event_type = 'FLIGHT_RECORDER_MIRROR_RECORDED')))\n        OR ($capability = 'fr.ingest.runtime_chat' AND $aggregate = 'runtime_chat_event'\n            AND $source = 'runtime_chat_fr_ingestion' AND $payload.workspace_id = $wsids[0]\n            AND $payload.receipt_kind = 'runtime_chat_flight_recorder_recorded' AND $event_type = 'FLIGHT_RECORDER_MIRROR_RECORDED')\n        OR ($payload.workspace_id = $wsids[0] AND (\n            ($capability = 'memory.propose' AND $payload.receipt_kind = 'fems_memory_write_proposal'\n                AND $aggregate = 'fems_memory_proposal' AND $source = 'fems_memory_proposal_intake' AND $event_type = 'ARTIFACT_PROPOSED')\n            OR ($capability = 'memory.review' AND $payload.receipt_kind = 'fems_memory_write_review'\n                AND $aggregate = 'fems_memory_proposal' AND $source = 'fems_memory_proposal_review' AND $event_type IN ['PROMOTION_ACCEPTED', 'PROMOTION_REJECTED'])\n            OR ($capability = 'memory.commit' AND $payload.receipt_kind = 'fems_memory_write_committed'\n                AND $aggregate = 'fems_memory_commit' AND $source = 'fems_memory_proposal_commit' AND $event_type = 'ARTIFACT_STORED'))));\n};\n", "DEFINE FUNCTION OVERWRITE fn::mt109_ledger_receipt($capability: option<string>, $event_type: string, $source: string, $aggregate: string, $payload: object, $wsids: array<string>) {\n    RETURN array::len($wsids) = 1 AND (\n        ($capability = 'fr.ingest.native_editor' AND $aggregate = 'native_editor_event'\n            AND $source = 'native_editor_fr_ingestion' AND $payload.envelope.workspace_id = $wsids[0]\n            AND (($payload.receipt_kind = 'native_editor_flight_recorder_pending' AND $event_type = 'FLIGHT_RECORDER_MIRROR_PENDING')\n                 OR ($payload.receipt_kind = 'native_editor_flight_recorder_recorded' AND $event_type = 'FLIGHT_RECORDER_MIRROR_RECORDED')))\n        OR ($payload.workspace_id = $wsids[0] AND (\n            ($capability = 'memory.propose' AND $payload.receipt_kind = 'fems_memory_write_proposal'\n                AND $aggregate = 'fems_memory_proposal' AND $source = 'fems_memory_proposal_intake' AND $event_type = 'ARTIFACT_PROPOSED')\n            OR ($capability = 'memory.review' AND $payload.receipt_kind = 'fems_memory_write_review'\n                AND $aggregate = 'fems_memory_proposal' AND $source = 'fems_memory_proposal_review' AND $event_type IN ['PROMOTION_ACCEPTED', 'PROMOTION_REJECTED'])\n            OR ($capability = 'memory.commit' AND $payload.receipt_kind = 'fems_memory_write_committed'\n                AND $aggregate = 'fems_memory_commit' AND $source = 'fems_memory_proposal_commit' AND $event_type = 'ARTIFACT_STORED'))));\n};\n"),
    ("DEFINE FUNCTION OVERWRITE fn::mt109_ledger_producer($resource: option<record<protected_resources>>, $session: option<record<authenticated_sessions>>, $capability: option<string>, $action: option<string>, $wsids: array<string>) {\n    RETURN fn::mt109_ledger_workspace($resource, $wsids)\n        AND fn::mt109_ledger_access($resource, $session, $capability, $action)\n        AND (($resource.resource_kind = 'flight_recorder' AND $action = 'create' AND $capability IN ['fr.ingest.native_editor', 'fr.ingest.runtime_chat'])\n             OR ($resource.resource_kind = 'memory_proposal' AND (($action = 'create' AND $capability = 'memory.propose') OR ($action = 'update' AND $capability IN ['memory.review', 'memory.commit'])))\n             OR ($resource.resource_kind = 'reconciliation_queue' AND $action = 'reconcile' AND $capability IN ['fr.ingest.native_editor', 'memory.commit']));\n};\n", "DEFINE FUNCTION OVERWRITE fn::mt109_ledger_producer($resource: option<record<protected_resources>>, $session: option<record<authenticated_sessions>>, $capability: option<string>, $action: option<string>, $wsids: array<string>) {\n    RETURN fn::mt109_ledger_workspace($resource, $wsids)\n        AND fn::mt109_ledger_access($resource, $session, $capability, $action)\n        AND (($resource.resource_kind = 'flight_recorder' AND $action = 'create' AND $capability = 'fr.ingest.native_editor')\n             OR ($resource.resource_kind = 'memory_proposal' AND (($action = 'create' AND $capability = 'memory.propose') OR ($action = 'update' AND $capability IN ['memory.review', 'memory.commit'])))\n             OR ($resource.resource_kind = 'reconciliation_queue' AND $action = 'reconcile' AND $capability IN ['fr.ingest.native_editor', 'memory.commit']));\n};\n"),
    ("DEFINE FUNCTION OVERWRITE fn::mt109_ledger_reader($resource: option<record<protected_resources>>, $session: option<record<authenticated_sessions>>, $capability: option<string>, $action: option<string>, $wsids: array<string>) {\n    RETURN (fn::mt109_ledger_workspace($resource, $wsids)\n            AND (fn::mt109_ledger_producer($resource, $session, $capability, $action, $wsids)\n                 OR ($resource.resource_kind IN ['flight_recorder', 'reconciliation_queue'] AND $capability = 'fr.ingest.native_editor'\n                     AND fn::mt109_has_grant('flight_recorder', $wsids[0], 'read', 'fr.read'))\n                 OR ($resource.resource_kind = 'flight_recorder' AND $capability = 'fr.ingest.runtime_chat'\n                     AND fn::mt109_has_grant('flight_recorder', $wsids[0], 'read', 'fr.read'))\n                 OR ($resource.resource_kind IN ['memory_proposal', 'reconciliation_queue'] AND $capability IN ['memory.propose', 'memory.review', 'memory.commit']\n                     AND fn::mt109_has_grant('memory_proposal', $wsids[0], 'read', 'memory.read'))))\n        OR (fn::mt109_live_session() AND $resource != NONE AND array::len($wsids) = 1\n            AND $resource.lifecycle_state = 'active'\n            AND $resource.created_by_principal_id.status = 'enabled'\n            AND (($resource.resource_kind = 'flight_recorder' AND $resource.external_resource_id = $wsids[0]\n                  AND $capability = 'fr.ingest.native_editor'\n                  AND fn::mt109_has_grant('reconciliation_queue', 'mt109-protected-reconciliation:' + $wsids[0], 'reconcile', 'fr.ingest.native_editor'))\n                 OR ($resource.resource_kind = 'memory_proposal' AND $resource.external_resource_id = $wsids[0]\n                     AND $capability IN ['memory.propose', 'memory.review', 'memory.commit']\n                     AND fn::mt109_has_grant('reconciliation_queue', 'mt109-protected-reconciliation:' + $wsids[0], 'reconcile', 'memory.commit'))\n                 OR ($resource.resource_kind = 'reconciliation_queue'\n                     AND $resource.external_resource_id = 'mt109-protected-reconciliation:' + $wsids[0]\n                     AND (($capability = 'fr.ingest.native_editor'\n                           AND fn::mt109_has_grant('reconciliation_queue', 'mt109-protected-reconciliation:' + $wsids[0], 'reconcile', 'fr.ingest.native_editor'))\n                          OR ($capability IN ['memory.propose', 'memory.review', 'memory.commit']\n                              AND fn::mt109_has_grant('reconciliation_queue', 'mt109-protected-reconciliation:' + $wsids[0], 'reconcile', 'memory.commit'))))));\n};\n", "DEFINE FUNCTION OVERWRITE fn::mt109_ledger_reader($resource: option<record<protected_resources>>, $session: option<record<authenticated_sessions>>, $capability: option<string>, $action: option<string>, $wsids: array<string>) {\n    RETURN (fn::mt109_ledger_workspace($resource, $wsids)\n            AND (fn::mt109_ledger_producer($resource, $session, $capability, $action, $wsids)\n                 OR ($resource.resource_kind IN ['flight_recorder', 'reconciliation_queue'] AND $capability = 'fr.ingest.native_editor'\n                     AND fn::mt109_has_grant('flight_recorder', $wsids[0], 'read', 'fr.read'))\n                 OR ($resource.resource_kind IN ['memory_proposal', 'reconciliation_queue'] AND $capability IN ['memory.propose', 'memory.review', 'memory.commit']\n                     AND fn::mt109_has_grant('memory_proposal', $wsids[0], 'read', 'memory.read'))))\n        OR (fn::mt109_live_session() AND $resource != NONE AND array::len($wsids) = 1\n            AND $resource.lifecycle_state = 'active'\n            AND $resource.created_by_principal_id.status = 'enabled'\n            AND (($resource.resource_kind = 'flight_recorder' AND $resource.external_resource_id = $wsids[0]\n                  AND $capability = 'fr.ingest.native_editor'\n                  AND fn::mt109_has_grant('reconciliation_queue', 'mt109-protected-reconciliation:' + $wsids[0], 'reconcile', 'fr.ingest.native_editor'))\n                 OR ($resource.resource_kind = 'memory_proposal' AND $resource.external_resource_id = $wsids[0]\n                     AND $capability IN ['memory.propose', 'memory.review', 'memory.commit']\n                     AND fn::mt109_has_grant('reconciliation_queue', 'mt109-protected-reconciliation:' + $wsids[0], 'reconcile', 'memory.commit'))\n                 OR ($resource.resource_kind = 'reconciliation_queue'\n                     AND $resource.external_resource_id = 'mt109-protected-reconciliation:' + $wsids[0]\n                     AND (($capability = 'fr.ingest.native_editor'\n                           AND fn::mt109_has_grant('reconciliation_queue', 'mt109-protected-reconciliation:' + $wsids[0], 'reconcile', 'fr.ingest.native_editor'))\n                          OR ($capability IN ['memory.propose', 'memory.review', 'memory.commit']\n                              AND fn::mt109_has_grant('reconciliation_queue', 'mt109-protected-reconciliation:' + $wsids[0], 'reconcile', 'memory.commit'))))));\n};\n"),
    ("-- Legacy generic sources have no account-bound resource kind in this producer; never delete them by workspace membership alone.\nIF array::len(SELECT id FROM documents WHERE workspace_id = $workspace) > 0 {\n    RETURN false;\n};\n-- MT-153/MT-154/MT-157 (Operator decision 2026-09-22 extended as C2 did): rows of the account-scoped Loom,\n-- calendar, Stage, Canvas and breakpoint surfaces can only be created by holders of this workspace's grants,\n-- so the owner's delete removes them through their workspace REFERENCE ON DELETE CASCADE.\nIF array::len(SELECT id FROM atelier_intake_item_loom_projection WHERE workspace_id = $workspace) > 0 { RETURN false; };\nIF array::len(SELECT id FROM ai_bronze_records WHERE workspace_id = $workspace) > 0 { RETURN false; };\nIF array::len(SELECT id FROM ai_silver_records WHERE workspace_id = $workspace) > 0 { RETURN false; };\nIF array::len(SELECT id FROM blocks WHERE document_id IN (SELECT VALUE id FROM documents WHERE workspace_id = $workspace)) > 0 { RETURN false; };\nIF array::len(SELECT id FROM documents WHERE workspace_id = $workspace) > 0 { RETURN false; };\nIF array::len(SELECT id FROM knowledge_claim_conflicts WHERE claim_id IN (SELECT VALUE id FROM knowledge_claims WHERE workspace_id = $workspace)) > 0 { RETURN false; };\nIF array::len(SELECT id FROM knowledge_claim_spans WHERE claim_id IN (SELECT VALUE id FROM knowledge_claims WHERE workspace_id = $workspace)) > 0 { RETURN false; };\nIF array::len(SELECT id FROM knowledge_claims WHERE workspace_id = $workspace) > 0 { RETURN false; };\nIF array::len(SELECT id FROM knowledge_code_scip_imports WHERE workspace_id = $workspace) > 0 { RETURN false; };\nIF array::len(SELECT id FROM knowledge_context_bundle_items WHERE bundle_id IN (SELECT VALUE id FROM knowledge_context_bundles WHERE workspace_id = $workspace)) > 0 { RETURN false; };\nIF array::len(SELECT id FROM knowledge_context_bundles WHERE workspace_id = $workspace) > 0 { RETURN false; };\nIF array::len(SELECT id FROM knowledge_memory_bridge_decisions WHERE workspace_id = $workspace) > 0 { RETURN false; };\nIF array::len(SELECT id FROM knowledge_memory_conflict_detection_findings WHERE job_id IN (SELECT VALUE id FROM knowledge_memory_conflict_detection_jobs WHERE workspace_id = $workspace)) > 0 { RETURN false; };\nIF array::len(SELECT id FROM knowledge_memory_conflict_detection_jobs WHERE workspace_id = $workspace) > 0 { RETURN false; };\nIF array::len(SELECT id FROM knowledge_memory_conflict_resolution_jobs WHERE workspace_id = $workspace) > 0 { RETURN false; };\nIF array::len(SELECT id FROM knowledge_memory_facts WHERE workspace_id = $workspace) > 0 { RETURN false; };\nIF array::len(SELECT id FROM knowledge_memory_ontology_aliases WHERE workspace_id = $workspace) > 0 { RETURN false; };\nIF array::len(SELECT id FROM knowledge_memory_ontology_terms WHERE workspace_id = $workspace) > 0 { RETURN false; };\nIF array::len(SELECT id FROM knowledge_memory_passages WHERE workspace_id = $workspace) > 0 { RETURN false; };\nIF array::len(SELECT id FROM knowledge_passage_evidence WHERE passage_id IN (SELECT VALUE id FROM knowledge_memory_passages WHERE workspace_id = $workspace)) > 0 { RETURN false; };\nIF array::len(SELECT id FROM knowledge_retrieval_traces WHERE workspace_id = $workspace) > 0 { RETURN false; };\nIF array::len(SELECT id FROM knowledge_semantic_catalog_entries WHERE workspace_id = $workspace) > 0 { RETURN false; };\nIF array::len(SELECT id FROM loom_block_knowledge_bridge WHERE workspace_id = $workspace) > 0 { RETURN false; };\nIF array::len(SELECT id FROM loom_canvas_boards WHERE (workspace_id = $workspace OR block_id.workspace_id = $workspace) AND (workspace_id = $workspace AND block_id.workspace_id = $workspace) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM loom_canvas_placements WHERE (workspace_id = $workspace OR canvas_block_id.workspace_id = $workspace OR placed_block_id.workspace_id = $workspace) AND (workspace_id = $workspace AND canvas_block_id.workspace_id = $workspace AND placed_block_id.workspace_id = $workspace) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM loom_canvas_visual_edges WHERE (workspace_id = $workspace OR canvas_block_id.workspace_id = $workspace OR from_placement_id.workspace_id = $workspace OR to_placement_id.workspace_id = $workspace) AND (workspace_id = $workspace AND canvas_block_id.workspace_id = $workspace AND from_placement_id.workspace_id = $workspace AND to_placement_id.workspace_id = $workspace AND from_placement_id.canvas_block_id = canvas_block_id AND to_placement_id.canvas_block_id = canvas_block_id) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM documents WHERE workspace_id IN (SELECT VALUE id FROM workspaces WHERE id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM blocks WHERE document_id IN (SELECT VALUE id FROM documents WHERE workspace_id = $workspace) AND (document_id IN (SELECT VALUE id FROM documents WHERE workspace_id = $workspace)) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM canvases WHERE workspace_id IN (SELECT VALUE id FROM workspaces WHERE id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM canvas_nodes WHERE canvas_id IN (SELECT VALUE id FROM canvases WHERE workspace_id = $workspace) AND (canvas_id IN (SELECT VALUE id FROM canvases WHERE workspace_id = $workspace)) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM canvas_edges WHERE canvas_id IN (SELECT VALUE id FROM canvases WHERE workspace_id = $workspace) AND (canvas_id IN (SELECT VALUE id FROM canvases WHERE workspace_id = $workspace)) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM canvas_edges WHERE from_node_id IN (SELECT VALUE id FROM canvas_nodes WHERE canvas_id IN (SELECT VALUE id FROM canvases WHERE workspace_id = $workspace)) AND (canvas_id IN (SELECT VALUE id FROM canvases WHERE workspace_id = $workspace)) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM canvas_edges WHERE to_node_id IN (SELECT VALUE id FROM canvas_nodes WHERE canvas_id IN (SELECT VALUE id FROM canvases WHERE workspace_id = $workspace)) AND (canvas_id IN (SELECT VALUE id FROM canvases WHERE workspace_id = $workspace)) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM ai_bronze_records WHERE workspace_id IN (SELECT VALUE id FROM workspaces WHERE id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM ai_silver_records WHERE workspace_id IN (SELECT VALUE id FROM workspaces WHERE id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM ai_silver_records WHERE bronze_ref IN (SELECT VALUE id FROM ai_bronze_records WHERE workspace_id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM assets WHERE workspace_id IN (SELECT VALUE id FROM workspaces WHERE id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM loom_blocks WHERE workspace_id IN (SELECT VALUE id FROM workspaces WHERE id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM loom_edges WHERE workspace_id IN (SELECT VALUE id FROM workspaces WHERE id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM loom_edges WHERE source_block_id IN (SELECT VALUE id FROM loom_blocks WHERE workspace_id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM loom_edges WHERE target_block_id IN (SELECT VALUE id FROM loom_blocks WHERE workspace_id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM calendar_sources WHERE workspace_id IN (SELECT VALUE id FROM workspaces WHERE id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM calendar_events WHERE workspace_id IN (SELECT VALUE id FROM workspaces WHERE id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM calendar_events WHERE source_id IN (SELECT VALUE id FROM calendar_sources WHERE workspace_id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM knowledge_source_roots WHERE workspace_id IN (SELECT VALUE id FROM workspaces WHERE id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM knowledge_sources WHERE workspace_id IN (SELECT VALUE id FROM workspaces WHERE id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM knowledge_sources WHERE root_id IN (SELECT VALUE id FROM knowledge_source_roots WHERE workspace_id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM knowledge_sources WHERE asset_id IN (SELECT VALUE id FROM assets WHERE workspace_id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM knowledge_sources WHERE loom_block_id IN (SELECT VALUE id FROM loom_blocks WHERE workspace_id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM knowledge_sources WHERE document_id IN (SELECT VALUE id FROM documents WHERE workspace_id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM knowledge_index_runs WHERE workspace_id IN (SELECT VALUE id FROM workspaces WHERE id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM knowledge_spans WHERE source_id IN (SELECT VALUE id FROM knowledge_sources WHERE workspace_id = $workspace) AND (source_id IN (SELECT VALUE id FROM knowledge_sources WHERE workspace_id = $workspace)) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM knowledge_entities WHERE workspace_id IN (SELECT VALUE id FROM workspaces WHERE id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM knowledge_entity_spans WHERE entity_id IN (SELECT VALUE id FROM knowledge_entities WHERE workspace_id = $workspace) AND (entity_id IN (SELECT VALUE id FROM knowledge_entities WHERE workspace_id = $workspace)) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM knowledge_edges WHERE workspace_id IN (SELECT VALUE id FROM workspaces WHERE id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM knowledge_edges WHERE source_entity_id IN (SELECT VALUE id FROM knowledge_entities WHERE workspace_id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM knowledge_edges WHERE target_entity_id IN (SELECT VALUE id FROM knowledge_entities WHERE workspace_id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM knowledge_edge_spans WHERE edge_id IN (SELECT VALUE id FROM knowledge_edges WHERE workspace_id = $workspace) AND (edge_id IN (SELECT VALUE id FROM knowledge_edges WHERE workspace_id = $workspace)) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM knowledge_claims WHERE workspace_id IN (SELECT VALUE id FROM workspaces WHERE id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM knowledge_claim_spans WHERE claim_id IN (SELECT VALUE id FROM knowledge_claims WHERE workspace_id = $workspace) AND (claim_id IN (SELECT VALUE id FROM knowledge_claims WHERE workspace_id = $workspace)) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM knowledge_claim_conflicts WHERE claim_id IN (SELECT VALUE id FROM knowledge_claims WHERE workspace_id = $workspace) AND (claim_id IN (SELECT VALUE id FROM knowledge_claims WHERE workspace_id = $workspace)) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM knowledge_claim_conflicts WHERE conflicting_claim_id IN (SELECT VALUE id FROM knowledge_claims WHERE workspace_id = $workspace) AND (claim_id IN (SELECT VALUE id FROM knowledge_claims WHERE workspace_id = $workspace)) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM knowledge_memory_passages WHERE workspace_id IN (SELECT VALUE id FROM workspaces WHERE id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM knowledge_passage_evidence WHERE passage_id IN (SELECT VALUE id FROM knowledge_memory_passages WHERE workspace_id = $workspace) AND (passage_id IN (SELECT VALUE id FROM knowledge_memory_passages WHERE workspace_id = $workspace)) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM knowledge_passage_evidence WHERE source_id IN (SELECT VALUE id FROM knowledge_sources WHERE workspace_id = $workspace) AND (passage_id IN (SELECT VALUE id FROM knowledge_memory_passages WHERE workspace_id = $workspace)) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM knowledge_passage_evidence WHERE claim_id IN (SELECT VALUE id FROM knowledge_claims WHERE workspace_id = $workspace) AND (passage_id IN (SELECT VALUE id FROM knowledge_memory_passages WHERE workspace_id = $workspace)) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM knowledge_wiki_projections WHERE workspace_id IN (SELECT VALUE id FROM workspaces WHERE id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM knowledge_rich_documents WHERE workspace_id IN (SELECT VALUE id FROM workspaces WHERE id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM knowledge_rich_document_title_anchors WHERE workspace_id IN (SELECT VALUE id FROM workspaces WHERE id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM knowledge_rich_document_versions WHERE rich_document_id IN (SELECT VALUE id FROM knowledge_rich_documents WHERE workspace_id = $workspace) AND (rich_document_id IN (SELECT VALUE id FROM knowledge_rich_documents WHERE workspace_id = $workspace)) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM knowledge_editor_code_nodes WHERE rich_document_id IN (SELECT VALUE id FROM knowledge_rich_documents WHERE workspace_id = $workspace) AND (rich_document_id IN (SELECT VALUE id FROM knowledge_rich_documents WHERE workspace_id = $workspace)) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM knowledge_context_bundles WHERE workspace_id IN (SELECT VALUE id FROM workspaces WHERE id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM knowledge_context_bundle_items WHERE bundle_id IN (SELECT VALUE id FROM knowledge_context_bundles WHERE workspace_id = $workspace) AND (bundle_id IN (SELECT VALUE id FROM knowledge_context_bundles WHERE workspace_id = $workspace)) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM knowledge_retrieval_traces WHERE workspace_id IN (SELECT VALUE id FROM workspaces WHERE id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM knowledge_idempotency_keys WHERE workspace_id IN (SELECT VALUE id FROM workspaces WHERE id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM knowledge_ingestion_root_policies WHERE workspace_id IN (SELECT VALUE id FROM workspaces WHERE id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM knowledge_ingestion_policy_decisions WHERE workspace_id IN (SELECT VALUE id FROM workspaces WHERE id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM knowledge_ingestion_receipts WHERE workspace_id IN (SELECT VALUE id FROM workspaces WHERE id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM knowledge_ingestion_receipts WHERE source_id IN (SELECT VALUE id FROM knowledge_sources WHERE workspace_id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM knowledge_ingestion_spans WHERE workspace_id IN (SELECT VALUE id FROM workspaces WHERE id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM knowledge_ingestion_spans WHERE source_id IN (SELECT VALUE id FROM knowledge_sources WHERE workspace_id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM knowledge_ingestion_spans WHERE receipt_id IN (SELECT VALUE id FROM knowledge_ingestion_receipts WHERE workspace_id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM knowledge_ingestion_repair_queue WHERE workspace_id IN (SELECT VALUE id FROM workspaces WHERE id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM knowledge_ingestion_repair_queue WHERE source_id IN (SELECT VALUE id FROM knowledge_sources WHERE workspace_id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM knowledge_code_files WHERE workspace_id IN (SELECT VALUE id FROM workspaces WHERE id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM knowledge_code_files WHERE source_id IN (SELECT VALUE id FROM knowledge_sources WHERE workspace_id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM knowledge_code_scip_imports WHERE workspace_id IN (SELECT VALUE id FROM workspaces WHERE id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM knowledge_code_repair_queue WHERE workspace_id IN (SELECT VALUE id FROM workspaces WHERE id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM knowledge_code_repair_queue WHERE source_id IN (SELECT VALUE id FROM knowledge_sources WHERE workspace_id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM knowledge_memory_ontology_terms WHERE workspace_id IN (SELECT VALUE id FROM workspaces WHERE id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM knowledge_memory_ontology_aliases WHERE term_id IN (SELECT VALUE id FROM knowledge_memory_ontology_terms WHERE workspace_id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM knowledge_memory_ontology_aliases WHERE workspace_id IN (SELECT VALUE id FROM workspaces WHERE id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM knowledge_memory_facts WHERE workspace_id IN (SELECT VALUE id FROM workspaces WHERE id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM knowledge_memory_facts WHERE claim_id IN (SELECT VALUE id FROM knowledge_claims WHERE workspace_id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM knowledge_memory_facts WHERE subject_entity_id IN (SELECT VALUE id FROM knowledge_entities WHERE workspace_id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM knowledge_memory_facts WHERE object_entity_id IN (SELECT VALUE id FROM knowledge_entities WHERE workspace_id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM knowledge_memory_conflict_detection_jobs WHERE workspace_id IN (SELECT VALUE id FROM workspaces WHERE id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM knowledge_memory_conflict_detection_findings WHERE job_id IN (SELECT VALUE id FROM knowledge_memory_conflict_detection_jobs WHERE workspace_id = $workspace) AND (job_id IN (SELECT VALUE id FROM knowledge_memory_conflict_detection_jobs WHERE workspace_id = $workspace)) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM knowledge_memory_conflict_detection_findings WHERE conflict_id IN (SELECT VALUE id FROM knowledge_claim_conflicts WHERE claim_id IN (SELECT VALUE id FROM knowledge_claims WHERE workspace_id = $workspace)) AND (job_id IN (SELECT VALUE id FROM knowledge_memory_conflict_detection_jobs WHERE workspace_id = $workspace)) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM knowledge_memory_conflict_resolution_jobs WHERE workspace_id IN (SELECT VALUE id FROM workspaces WHERE id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM knowledge_memory_conflict_resolution_jobs WHERE conflict_id IN (SELECT VALUE id FROM knowledge_claim_conflicts WHERE claim_id IN (SELECT VALUE id FROM knowledge_claims WHERE workspace_id = $workspace)) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM knowledge_memory_bridge_decisions WHERE workspace_id IN (SELECT VALUE id FROM workspaces WHERE id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM knowledge_memory_bridge_decisions WHERE entity_id_a IN (SELECT VALUE id FROM knowledge_entities WHERE workspace_id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM knowledge_memory_bridge_decisions WHERE entity_id_b IN (SELECT VALUE id FROM knowledge_entities WHERE workspace_id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM knowledge_semantic_catalog_entries WHERE workspace_id IN (SELECT VALUE id FROM workspaces WHERE id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM knowledge_document_embeds WHERE rich_document_id IN (SELECT VALUE id FROM knowledge_rich_documents WHERE workspace_id = $workspace) AND (rich_document_id IN (SELECT VALUE id FROM knowledge_rich_documents WHERE workspace_id = $workspace)) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM knowledge_document_backlinks WHERE workspace_id IN (SELECT VALUE id FROM workspaces WHERE id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM knowledge_document_backlinks WHERE source_document_id IN (SELECT VALUE id FROM knowledge_rich_documents WHERE workspace_id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM loom_block_knowledge_bridge WHERE block_id IN (SELECT VALUE id FROM loom_blocks WHERE workspace_id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM loom_block_knowledge_bridge WHERE workspace_id IN (SELECT VALUE id FROM workspaces WHERE id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM loom_block_knowledge_bridge WHERE entity_id IN (SELECT VALUE id FROM knowledge_entities WHERE workspace_id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM loom_folders WHERE workspace_id IN (SELECT VALUE id FROM workspaces WHERE id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM loom_folders WHERE parent_folder_id IN (SELECT VALUE id FROM loom_folders WHERE workspace_id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM loom_folder_members WHERE folder_id IN (SELECT VALUE id FROM loom_folders WHERE workspace_id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM loom_folder_members WHERE block_id IN (SELECT VALUE id FROM loom_blocks WHERE workspace_id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM loom_folder_members WHERE workspace_id IN (SELECT VALUE id FROM workspaces WHERE id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM loom_wiki_overlays WHERE workspace_id IN (SELECT VALUE id FROM workspaces WHERE id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM knowledge_quick_switcher_recents WHERE workspace_id IN (SELECT VALUE id FROM workspaces WHERE id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM knowledge_workbench_layout_states WHERE workspace_id IN (SELECT VALUE id FROM workspaces WHERE id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM knowledge_workspace_settings_states WHERE workspace_id IN (SELECT VALUE id FROM workspaces WHERE id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM knowledge_rich_document_drafts WHERE rich_document_id IN (SELECT VALUE id FROM knowledge_rich_documents WHERE workspace_id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM knowledge_rich_document_drafts WHERE workspace_id IN (SELECT VALUE id FROM workspaces WHERE id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM knowledge_workspace_search_bookmark_states WHERE workspace_id IN (SELECT VALUE id FROM workspaces WHERE id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM knowledge_debug_breakpoints WHERE rich_document_id IN (SELECT VALUE id FROM knowledge_rich_documents WHERE workspace_id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM knowledge_debug_breakpoints WHERE workspace_id IN (SELECT VALUE id FROM workspaces WHERE id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM media_asset_tiers WHERE workspace_id IN (SELECT VALUE id FROM workspaces WHERE id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM media_asset_tiers WHERE asset_id IN (SELECT VALUE id FROM assets WHERE workspace_id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM loom_collections WHERE workspace_id IN (SELECT VALUE id FROM workspaces WHERE id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM loom_collection_members WHERE collection_id IN (SELECT VALUE id FROM loom_collections WHERE workspace_id = $workspace) AND (collection_id IN (SELECT VALUE id FROM loom_collections WHERE workspace_id = $workspace)) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM loom_collection_members WHERE asset_id IN (SELECT VALUE id FROM assets WHERE workspace_id = $workspace) AND (collection_id IN (SELECT VALUE id FROM loom_collections WHERE workspace_id = $workspace)) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM loom_ai_suggestions WHERE workspace_id IN (SELECT VALUE id FROM workspaces WHERE id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM loom_canvas_boards WHERE block_id IN (SELECT VALUE id FROM loom_blocks WHERE workspace_id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM loom_canvas_boards WHERE workspace_id IN (SELECT VALUE id FROM workspaces WHERE id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM loom_canvas_placements WHERE canvas_block_id IN (SELECT VALUE id FROM loom_canvas_boards WHERE workspace_id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM loom_canvas_placements WHERE workspace_id IN (SELECT VALUE id FROM workspaces WHERE id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM loom_canvas_visual_edges WHERE canvas_block_id IN (SELECT VALUE id FROM loom_canvas_boards WHERE workspace_id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM loom_canvas_visual_edges WHERE workspace_id IN (SELECT VALUE id FROM workspaces WHERE id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM loom_canvas_visual_edges WHERE from_placement_id IN (SELECT VALUE id FROM loom_canvas_placements WHERE workspace_id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM loom_canvas_visual_edges WHERE to_placement_id IN (SELECT VALUE id FROM loom_canvas_placements WHERE workspace_id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM loom_block_search_index WHERE block_id IN (SELECT VALUE id FROM loom_blocks WHERE workspace_id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM loom_block_search_index WHERE workspace_id IN (SELECT VALUE id FROM workspaces WHERE id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM calendar_activity_spans WHERE workspace_id IN (SELECT VALUE id FROM workspaces WHERE id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM stage_capture_artifacts WHERE workspace_id IN (SELECT VALUE id FROM workspaces WHERE id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM fems_memory_packs WHERE workspace_id IN (SELECT VALUE id FROM workspaces WHERE id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM fems_memory_proposals WHERE workspace_id IN (SELECT VALUE id FROM workspaces WHERE id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM fems_memory_items WHERE workspace_id IN (SELECT VALUE id FROM workspaces WHERE id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM fems_memory_commit_reports WHERE workspace_id IN (SELECT VALUE id FROM workspaces WHERE id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM fems_memory_commit_reports WHERE proposal_id IN (SELECT VALUE id FROM fems_memory_proposals WHERE workspace_id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM fems_memory_commit_fr_outbox WHERE workspace_id IN (SELECT VALUE id FROM workspaces WHERE id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM fems_memory_commit_fr_outbox WHERE proposal_id IN (SELECT VALUE id FROM fems_memory_proposals WHERE workspace_id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM fems_memory_commit_fr_outbox WHERE commit_id IN (SELECT VALUE id FROM fems_memory_commit_reports WHERE workspace_id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM fems_memory_lifecycle_fr_outbox WHERE workspace_id IN (SELECT VALUE id FROM workspaces WHERE id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM fems_memory_lifecycle_fr_outbox WHERE proposal_id IN (SELECT VALUE id FROM fems_memory_proposals WHERE workspace_id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM loom_block_view_fr_outbox WHERE workspace_id IN (SELECT VALUE id FROM workspaces WHERE id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };\nFOR $source IN (SELECT id FROM knowledge_sources WHERE workspace_id = $workspace) { IF array::len(SELECT id FROM protected_resources WHERE id IN $resources.id AND resource_kind = 'knowledge_source' AND external_resource_id = record::id($source.id) AND lifecycle_state = 'active') != 1 { RETURN false; }; };\nFOR $source IN (SELECT id FROM knowledge_code_files WHERE workspace_id = $workspace) { IF array::len(SELECT id FROM protected_resources WHERE id IN $resources.id AND resource_kind = 'knowledge_code_file' AND external_resource_id = record::id($source.id) AND lifecycle_state = 'active') != 1 { RETURN false; }; };\nFOR $source IN (SELECT id FROM fems_memory_packs WHERE workspace_id = $workspace) { IF array::len(SELECT id FROM protected_resources WHERE id IN $resources.id AND resource_kind = 'memory_pack' AND external_resource_id = $external AND lifecycle_state = 'active') != 1 { RETURN false; }; };\nFOR $source IN (SELECT id FROM fems_memory_proposals WHERE workspace_id = $workspace) { IF array::len(SELECT id FROM protected_resources WHERE id IN $resources.id AND resource_kind = 'memory_proposal' AND external_resource_id = $external AND lifecycle_state = 'active') != 1 { RETURN false; }; };\nFOR $source IN (SELECT id FROM fems_memory_items WHERE workspace_id = $workspace) { IF array::len(SELECT id FROM protected_resources WHERE id IN $resources.id AND resource_kind = 'memory_item' AND external_resource_id = $external AND lifecycle_state = 'active') != 1 { RETURN false; }; };\nFOR $source IN (SELECT id FROM fems_memory_commit_reports WHERE workspace_id = $workspace) { IF array::len(SELECT id FROM protected_resources WHERE id IN $resources.id AND resource_kind = 'memory_commit_report' AND external_resource_id = $external AND lifecycle_state = 'active') != 1 { RETURN false; }; };\n    FOR $row IN (SELECT created_in_session_id FROM knowledge_source_roots WHERE workspace_id = $workspace) { IF $row.created_in_session_id = NONE OR $row.created_in_session_id.account_id != $account OR $row.created_in_session_id.access_space_id != $space { RETURN false; }; };\n    FOR $row IN (SELECT created_in_session_id FROM knowledge_index_runs WHERE workspace_id = $workspace) { IF $row.created_in_session_id = NONE OR $row.created_in_session_id.account_id != $account OR $row.created_in_session_id.access_space_id != $space { RETURN false; }; };\n    FOR $run IN (SELECT scope FROM knowledge_index_runs WHERE workspace_id = $workspace) {\n        IF $run.scope.source_ids = NONE { RETURN false; };\n        FOR $key IN $run.scope.source_ids {\n            IF type::record('knowledge_sources', $key).workspace_id != $workspace OR !fn::mt109_has_grant('knowledge_source', $key, 'delete', 'fs.write') { RETURN false; };\n        };\n    };\n    FOR $entity IN (SELECT * FROM knowledge_entities WHERE workspace_id = $workspace) {\n        IF $entity.entity_kind = 'loom_block' AND $entity.primary_source_id = NONE {\n            IF array::len(SELECT id FROM protected_resources WHERE id IN $resources.id AND resource_kind IN ['loom_block','rich_document'] AND external_resource_id = $entity.entity_key AND lifecycle_state = 'active') != 1 { RETURN false; };\n        } ELSE IF $entity.primary_source_id = NONE OR $entity.primary_source_id.workspace_id != $workspace OR !fn::mt109_has_grant('knowledge_source', record::id($entity.primary_source_id), 'delete', 'fs.write') { RETURN false; };\n        FOR $link IN (SELECT span_id FROM knowledge_entity_spans WHERE entity_id = $entity.id) {\n            IF $link.span_id.source_id.workspace_id != $workspace OR !fn::mt109_has_grant('knowledge_source', record::id($link.span_id.source_id), 'delete', 'fs.write') { RETURN false; };\n        };\n    };\n    FOR $edge IN (SELECT * FROM knowledge_edges WHERE workspace_id = $workspace) {\n        IF $edge.source_entity_id.workspace_id != $workspace OR $edge.target_entity_id.workspace_id != $workspace { RETURN false; };\n        FOR $link IN (SELECT span_id FROM knowledge_edge_spans WHERE edge_id = $edge.id) {\n            IF $link.span_id.source_id.workspace_id != $workspace OR !fn::mt109_has_grant('knowledge_source', record::id($link.span_id.source_id), 'delete', 'fs.write') { RETURN false; };\n        };\n    };\n    IF array::len(SELECT id FROM knowledge_ingestion_receipts WHERE workspace_id = $workspace AND (source_id.workspace_id = $workspace) != true) > 0\n        OR array::len(SELECT id FROM knowledge_ingestion_spans WHERE workspace_id = $workspace AND (source_id.workspace_id = $workspace AND receipt_id.workspace_id = $workspace AND receipt_id.source_id = source_id) != true) > 0\n        OR array::len(SELECT id FROM knowledge_ingestion_repair_queue WHERE workspace_id = $workspace AND (source_id.workspace_id = $workspace) != true) > 0\n        OR array::len(SELECT id FROM knowledge_code_repair_queue WHERE workspace_id = $workspace AND (source_id.workspace_id = $workspace) != true) > 0 { RETURN false; };\n    RETURN true;\n};\n", "-- Legacy generic sources have no account-bound resource kind in this producer; never delete them by workspace membership alone.\nIF array::len(SELECT id FROM documents WHERE workspace_id = $workspace) > 0\n    OR array::len(SELECT id FROM assets WHERE workspace_id = $workspace) > 0\n    OR array::len(SELECT id FROM canvases WHERE workspace_id = $workspace) > 0 {\n    RETURN false;\n};\nIF array::len(SELECT id FROM atelier_intake_item_loom_projection WHERE workspace_id = $workspace) > 0 { RETURN false; };\nIF array::len(SELECT id FROM ai_bronze_records WHERE workspace_id = $workspace) > 0 { RETURN false; };\nIF array::len(SELECT id FROM ai_silver_records WHERE workspace_id = $workspace) > 0 { RETURN false; };\nIF array::len(SELECT id FROM assets WHERE workspace_id = $workspace) > 0 { RETURN false; };\nIF array::len(SELECT id FROM blocks WHERE document_id IN (SELECT VALUE id FROM documents WHERE workspace_id = $workspace)) > 0 { RETURN false; };\nIF array::len(SELECT id FROM calendar_activity_spans WHERE workspace_id = $workspace) > 0 { RETURN false; };\nIF array::len(SELECT id FROM calendar_events WHERE workspace_id = $workspace) > 0 { RETURN false; };\nIF array::len(SELECT id FROM calendar_sources WHERE workspace_id = $workspace) > 0 { RETURN false; };\nIF array::len(SELECT id FROM canvas_edges WHERE canvas_id IN (SELECT VALUE id FROM canvases WHERE workspace_id = $workspace)) > 0 { RETURN false; };\nIF array::len(SELECT id FROM canvas_nodes WHERE canvas_id IN (SELECT VALUE id FROM canvases WHERE workspace_id = $workspace)) > 0 { RETURN false; };\nIF array::len(SELECT id FROM canvases WHERE workspace_id = $workspace) > 0 { RETURN false; };\nIF array::len(SELECT id FROM documents WHERE workspace_id = $workspace) > 0 { RETURN false; };\nIF array::len(SELECT id FROM knowledge_claim_conflicts WHERE claim_id IN (SELECT VALUE id FROM knowledge_claims WHERE workspace_id = $workspace)) > 0 { RETURN false; };\nIF array::len(SELECT id FROM knowledge_claim_spans WHERE claim_id IN (SELECT VALUE id FROM knowledge_claims WHERE workspace_id = $workspace)) > 0 { RETURN false; };\nIF array::len(SELECT id FROM knowledge_claims WHERE workspace_id = $workspace) > 0 { RETURN false; };\nIF array::len(SELECT id FROM knowledge_code_scip_imports WHERE workspace_id = $workspace) > 0 { RETURN false; };\nIF array::len(SELECT id FROM knowledge_context_bundle_items WHERE bundle_id IN (SELECT VALUE id FROM knowledge_context_bundles WHERE workspace_id = $workspace)) > 0 { RETURN false; };\nIF array::len(SELECT id FROM knowledge_context_bundles WHERE workspace_id = $workspace) > 0 { RETURN false; };\nIF array::len(SELECT id FROM knowledge_debug_breakpoints WHERE workspace_id = $workspace) > 0 { RETURN false; };\nIF array::len(SELECT id FROM knowledge_memory_bridge_decisions WHERE workspace_id = $workspace) > 0 { RETURN false; };\nIF array::len(SELECT id FROM knowledge_memory_conflict_detection_findings WHERE job_id IN (SELECT VALUE id FROM knowledge_memory_conflict_detection_jobs WHERE workspace_id = $workspace)) > 0 { RETURN false; };\nIF array::len(SELECT id FROM knowledge_memory_conflict_detection_jobs WHERE workspace_id = $workspace) > 0 { RETURN false; };\nIF array::len(SELECT id FROM knowledge_memory_conflict_resolution_jobs WHERE workspace_id = $workspace) > 0 { RETURN false; };\nIF array::len(SELECT id FROM knowledge_memory_facts WHERE workspace_id = $workspace) > 0 { RETURN false; };\nIF array::len(SELECT id FROM knowledge_memory_ontology_aliases WHERE workspace_id = $workspace) > 0 { RETURN false; };\nIF array::len(SELECT id FROM knowledge_memory_ontology_terms WHERE workspace_id = $workspace) > 0 { RETURN false; };\nIF array::len(SELECT id FROM knowledge_memory_passages WHERE workspace_id = $workspace) > 0 { RETURN false; };\nIF array::len(SELECT id FROM knowledge_passage_evidence WHERE passage_id IN (SELECT VALUE id FROM knowledge_memory_passages WHERE workspace_id = $workspace)) > 0 { RETURN false; };\nIF array::len(SELECT id FROM knowledge_quick_switcher_recents WHERE workspace_id = $workspace) > 0 { RETURN false; };\nIF array::len(SELECT id FROM knowledge_retrieval_traces WHERE workspace_id = $workspace) > 0 { RETURN false; };\nIF array::len(SELECT id FROM knowledge_semantic_catalog_entries WHERE workspace_id = $workspace) > 0 { RETURN false; };\nIF array::len(SELECT id FROM knowledge_wiki_projections WHERE workspace_id = $workspace) > 0 { RETURN false; };\nIF array::len(SELECT id FROM loom_ai_suggestions WHERE workspace_id = $workspace) > 0 { RETURN false; };\nIF array::len(SELECT id FROM loom_block_knowledge_bridge WHERE workspace_id = $workspace) > 0 { RETURN false; };\nIF array::len(SELECT id FROM loom_canvas_boards WHERE workspace_id = $workspace) > 0 { RETURN false; };\nIF array::len(SELECT id FROM loom_canvas_placements WHERE workspace_id = $workspace) > 0 { RETURN false; };\nIF array::len(SELECT id FROM loom_canvas_visual_edges WHERE workspace_id = $workspace) > 0 { RETURN false; };\nIF array::len(SELECT id FROM loom_collection_members WHERE collection_id IN (SELECT VALUE id FROM loom_collections WHERE workspace_id = $workspace)) > 0 { RETURN false; };\nIF array::len(SELECT id FROM loom_collections WHERE workspace_id = $workspace) > 0 { RETURN false; };\nIF array::len(SELECT id FROM loom_folder_members WHERE workspace_id = $workspace) > 0 { RETURN false; };\nIF array::len(SELECT id FROM loom_folders WHERE workspace_id = $workspace) > 0 { RETURN false; };\nIF array::len(SELECT id FROM loom_wiki_overlays WHERE workspace_id = $workspace) > 0 { RETURN false; };\nIF array::len(SELECT id FROM media_asset_tiers WHERE workspace_id = $workspace) > 0 { RETURN false; };\nIF array::len(SELECT id FROM stage_capture_artifacts WHERE workspace_id = $workspace) > 0 { RETURN false; };\nIF array::len(SELECT id FROM documents WHERE workspace_id IN (SELECT VALUE id FROM workspaces WHERE id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM blocks WHERE document_id IN (SELECT VALUE id FROM documents WHERE workspace_id = $workspace) AND (document_id IN (SELECT VALUE id FROM documents WHERE workspace_id = $workspace)) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM canvases WHERE workspace_id IN (SELECT VALUE id FROM workspaces WHERE id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM canvas_nodes WHERE canvas_id IN (SELECT VALUE id FROM canvases WHERE workspace_id = $workspace) AND (canvas_id IN (SELECT VALUE id FROM canvases WHERE workspace_id = $workspace)) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM canvas_edges WHERE canvas_id IN (SELECT VALUE id FROM canvases WHERE workspace_id = $workspace) AND (canvas_id IN (SELECT VALUE id FROM canvases WHERE workspace_id = $workspace)) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM canvas_edges WHERE from_node_id IN (SELECT VALUE id FROM canvas_nodes WHERE canvas_id IN (SELECT VALUE id FROM canvases WHERE workspace_id = $workspace)) AND (canvas_id IN (SELECT VALUE id FROM canvases WHERE workspace_id = $workspace)) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM canvas_edges WHERE to_node_id IN (SELECT VALUE id FROM canvas_nodes WHERE canvas_id IN (SELECT VALUE id FROM canvases WHERE workspace_id = $workspace)) AND (canvas_id IN (SELECT VALUE id FROM canvases WHERE workspace_id = $workspace)) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM ai_bronze_records WHERE workspace_id IN (SELECT VALUE id FROM workspaces WHERE id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM ai_silver_records WHERE workspace_id IN (SELECT VALUE id FROM workspaces WHERE id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM ai_silver_records WHERE bronze_ref IN (SELECT VALUE id FROM ai_bronze_records WHERE workspace_id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM assets WHERE workspace_id IN (SELECT VALUE id FROM workspaces WHERE id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM loom_blocks WHERE workspace_id IN (SELECT VALUE id FROM workspaces WHERE id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM loom_edges WHERE workspace_id IN (SELECT VALUE id FROM workspaces WHERE id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM loom_edges WHERE source_block_id IN (SELECT VALUE id FROM loom_blocks WHERE workspace_id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM loom_edges WHERE target_block_id IN (SELECT VALUE id FROM loom_blocks WHERE workspace_id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM calendar_sources WHERE workspace_id IN (SELECT VALUE id FROM workspaces WHERE id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM calendar_events WHERE workspace_id IN (SELECT VALUE id FROM workspaces WHERE id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM calendar_events WHERE source_id IN (SELECT VALUE id FROM calendar_sources WHERE workspace_id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM knowledge_source_roots WHERE workspace_id IN (SELECT VALUE id FROM workspaces WHERE id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM knowledge_sources WHERE workspace_id IN (SELECT VALUE id FROM workspaces WHERE id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM knowledge_sources WHERE root_id IN (SELECT VALUE id FROM knowledge_source_roots WHERE workspace_id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM knowledge_sources WHERE asset_id IN (SELECT VALUE id FROM assets WHERE workspace_id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM knowledge_sources WHERE loom_block_id IN (SELECT VALUE id FROM loom_blocks WHERE workspace_id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM knowledge_sources WHERE document_id IN (SELECT VALUE id FROM documents WHERE workspace_id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM knowledge_index_runs WHERE workspace_id IN (SELECT VALUE id FROM workspaces WHERE id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM knowledge_spans WHERE source_id IN (SELECT VALUE id FROM knowledge_sources WHERE workspace_id = $workspace) AND (source_id IN (SELECT VALUE id FROM knowledge_sources WHERE workspace_id = $workspace)) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM knowledge_entities WHERE workspace_id IN (SELECT VALUE id FROM workspaces WHERE id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM knowledge_entity_spans WHERE entity_id IN (SELECT VALUE id FROM knowledge_entities WHERE workspace_id = $workspace) AND (entity_id IN (SELECT VALUE id FROM knowledge_entities WHERE workspace_id = $workspace)) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM knowledge_edges WHERE workspace_id IN (SELECT VALUE id FROM workspaces WHERE id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM knowledge_edges WHERE source_entity_id IN (SELECT VALUE id FROM knowledge_entities WHERE workspace_id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM knowledge_edges WHERE target_entity_id IN (SELECT VALUE id FROM knowledge_entities WHERE workspace_id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM knowledge_edge_spans WHERE edge_id IN (SELECT VALUE id FROM knowledge_edges WHERE workspace_id = $workspace) AND (edge_id IN (SELECT VALUE id FROM knowledge_edges WHERE workspace_id = $workspace)) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM knowledge_claims WHERE workspace_id IN (SELECT VALUE id FROM workspaces WHERE id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM knowledge_claim_spans WHERE claim_id IN (SELECT VALUE id FROM knowledge_claims WHERE workspace_id = $workspace) AND (claim_id IN (SELECT VALUE id FROM knowledge_claims WHERE workspace_id = $workspace)) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM knowledge_claim_conflicts WHERE claim_id IN (SELECT VALUE id FROM knowledge_claims WHERE workspace_id = $workspace) AND (claim_id IN (SELECT VALUE id FROM knowledge_claims WHERE workspace_id = $workspace)) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM knowledge_claim_conflicts WHERE conflicting_claim_id IN (SELECT VALUE id FROM knowledge_claims WHERE workspace_id = $workspace) AND (claim_id IN (SELECT VALUE id FROM knowledge_claims WHERE workspace_id = $workspace)) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM knowledge_memory_passages WHERE workspace_id IN (SELECT VALUE id FROM workspaces WHERE id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM knowledge_passage_evidence WHERE passage_id IN (SELECT VALUE id FROM knowledge_memory_passages WHERE workspace_id = $workspace) AND (passage_id IN (SELECT VALUE id FROM knowledge_memory_passages WHERE workspace_id = $workspace)) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM knowledge_passage_evidence WHERE source_id IN (SELECT VALUE id FROM knowledge_sources WHERE workspace_id = $workspace) AND (passage_id IN (SELECT VALUE id FROM knowledge_memory_passages WHERE workspace_id = $workspace)) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM knowledge_passage_evidence WHERE claim_id IN (SELECT VALUE id FROM knowledge_claims WHERE workspace_id = $workspace) AND (passage_id IN (SELECT VALUE id FROM knowledge_memory_passages WHERE workspace_id = $workspace)) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM knowledge_wiki_projections WHERE workspace_id IN (SELECT VALUE id FROM workspaces WHERE id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM knowledge_rich_documents WHERE workspace_id IN (SELECT VALUE id FROM workspaces WHERE id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM knowledge_rich_document_title_anchors WHERE workspace_id IN (SELECT VALUE id FROM workspaces WHERE id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM knowledge_rich_document_versions WHERE rich_document_id IN (SELECT VALUE id FROM knowledge_rich_documents WHERE workspace_id = $workspace) AND (rich_document_id IN (SELECT VALUE id FROM knowledge_rich_documents WHERE workspace_id = $workspace)) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM knowledge_editor_code_nodes WHERE rich_document_id IN (SELECT VALUE id FROM knowledge_rich_documents WHERE workspace_id = $workspace) AND (rich_document_id IN (SELECT VALUE id FROM knowledge_rich_documents WHERE workspace_id = $workspace)) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM knowledge_context_bundles WHERE workspace_id IN (SELECT VALUE id FROM workspaces WHERE id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM knowledge_context_bundle_items WHERE bundle_id IN (SELECT VALUE id FROM knowledge_context_bundles WHERE workspace_id = $workspace) AND (bundle_id IN (SELECT VALUE id FROM knowledge_context_bundles WHERE workspace_id = $workspace)) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM knowledge_retrieval_traces WHERE workspace_id IN (SELECT VALUE id FROM workspaces WHERE id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM knowledge_idempotency_keys WHERE workspace_id IN (SELECT VALUE id FROM workspaces WHERE id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM knowledge_ingestion_root_policies WHERE workspace_id IN (SELECT VALUE id FROM workspaces WHERE id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM knowledge_ingestion_policy_decisions WHERE workspace_id IN (SELECT VALUE id FROM workspaces WHERE id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM knowledge_ingestion_receipts WHERE workspace_id IN (SELECT VALUE id FROM workspaces WHERE id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM knowledge_ingestion_receipts WHERE source_id IN (SELECT VALUE id FROM knowledge_sources WHERE workspace_id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM knowledge_ingestion_spans WHERE workspace_id IN (SELECT VALUE id FROM workspaces WHERE id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM knowledge_ingestion_spans WHERE source_id IN (SELECT VALUE id FROM knowledge_sources WHERE workspace_id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM knowledge_ingestion_spans WHERE receipt_id IN (SELECT VALUE id FROM knowledge_ingestion_receipts WHERE workspace_id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM knowledge_ingestion_repair_queue WHERE workspace_id IN (SELECT VALUE id FROM workspaces WHERE id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM knowledge_ingestion_repair_queue WHERE source_id IN (SELECT VALUE id FROM knowledge_sources WHERE workspace_id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM knowledge_code_files WHERE workspace_id IN (SELECT VALUE id FROM workspaces WHERE id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM knowledge_code_files WHERE source_id IN (SELECT VALUE id FROM knowledge_sources WHERE workspace_id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM knowledge_code_scip_imports WHERE workspace_id IN (SELECT VALUE id FROM workspaces WHERE id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM knowledge_code_repair_queue WHERE workspace_id IN (SELECT VALUE id FROM workspaces WHERE id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM knowledge_code_repair_queue WHERE source_id IN (SELECT VALUE id FROM knowledge_sources WHERE workspace_id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM knowledge_memory_ontology_terms WHERE workspace_id IN (SELECT VALUE id FROM workspaces WHERE id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM knowledge_memory_ontology_aliases WHERE term_id IN (SELECT VALUE id FROM knowledge_memory_ontology_terms WHERE workspace_id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM knowledge_memory_ontology_aliases WHERE workspace_id IN (SELECT VALUE id FROM workspaces WHERE id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM knowledge_memory_facts WHERE workspace_id IN (SELECT VALUE id FROM workspaces WHERE id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM knowledge_memory_facts WHERE claim_id IN (SELECT VALUE id FROM knowledge_claims WHERE workspace_id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM knowledge_memory_facts WHERE subject_entity_id IN (SELECT VALUE id FROM knowledge_entities WHERE workspace_id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM knowledge_memory_facts WHERE object_entity_id IN (SELECT VALUE id FROM knowledge_entities WHERE workspace_id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM knowledge_memory_conflict_detection_jobs WHERE workspace_id IN (SELECT VALUE id FROM workspaces WHERE id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM knowledge_memory_conflict_detection_findings WHERE job_id IN (SELECT VALUE id FROM knowledge_memory_conflict_detection_jobs WHERE workspace_id = $workspace) AND (job_id IN (SELECT VALUE id FROM knowledge_memory_conflict_detection_jobs WHERE workspace_id = $workspace)) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM knowledge_memory_conflict_detection_findings WHERE conflict_id IN (SELECT VALUE id FROM knowledge_claim_conflicts WHERE claim_id IN (SELECT VALUE id FROM knowledge_claims WHERE workspace_id = $workspace)) AND (job_id IN (SELECT VALUE id FROM knowledge_memory_conflict_detection_jobs WHERE workspace_id = $workspace)) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM knowledge_memory_conflict_resolution_jobs WHERE workspace_id IN (SELECT VALUE id FROM workspaces WHERE id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM knowledge_memory_conflict_resolution_jobs WHERE conflict_id IN (SELECT VALUE id FROM knowledge_claim_conflicts WHERE claim_id IN (SELECT VALUE id FROM knowledge_claims WHERE workspace_id = $workspace)) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM knowledge_memory_bridge_decisions WHERE workspace_id IN (SELECT VALUE id FROM workspaces WHERE id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM knowledge_memory_bridge_decisions WHERE entity_id_a IN (SELECT VALUE id FROM knowledge_entities WHERE workspace_id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM knowledge_memory_bridge_decisions WHERE entity_id_b IN (SELECT VALUE id FROM knowledge_entities WHERE workspace_id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM knowledge_semantic_catalog_entries WHERE workspace_id IN (SELECT VALUE id FROM workspaces WHERE id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM knowledge_document_embeds WHERE rich_document_id IN (SELECT VALUE id FROM knowledge_rich_documents WHERE workspace_id = $workspace) AND (rich_document_id IN (SELECT VALUE id FROM knowledge_rich_documents WHERE workspace_id = $workspace)) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM knowledge_document_backlinks WHERE workspace_id IN (SELECT VALUE id FROM workspaces WHERE id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM knowledge_document_backlinks WHERE source_document_id IN (SELECT VALUE id FROM knowledge_rich_documents WHERE workspace_id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM loom_block_knowledge_bridge WHERE block_id IN (SELECT VALUE id FROM loom_blocks WHERE workspace_id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM loom_block_knowledge_bridge WHERE workspace_id IN (SELECT VALUE id FROM workspaces WHERE id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM loom_block_knowledge_bridge WHERE entity_id IN (SELECT VALUE id FROM knowledge_entities WHERE workspace_id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM loom_folders WHERE workspace_id IN (SELECT VALUE id FROM workspaces WHERE id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM loom_folders WHERE parent_folder_id IN (SELECT VALUE id FROM loom_folders WHERE workspace_id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM loom_folder_members WHERE folder_id IN (SELECT VALUE id FROM loom_folders WHERE workspace_id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM loom_folder_members WHERE block_id IN (SELECT VALUE id FROM loom_blocks WHERE workspace_id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM loom_folder_members WHERE workspace_id IN (SELECT VALUE id FROM workspaces WHERE id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM loom_wiki_overlays WHERE workspace_id IN (SELECT VALUE id FROM workspaces WHERE id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM knowledge_quick_switcher_recents WHERE workspace_id IN (SELECT VALUE id FROM workspaces WHERE id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM knowledge_workbench_layout_states WHERE workspace_id IN (SELECT VALUE id FROM workspaces WHERE id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM knowledge_workspace_settings_states WHERE workspace_id IN (SELECT VALUE id FROM workspaces WHERE id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM knowledge_rich_document_drafts WHERE rich_document_id IN (SELECT VALUE id FROM knowledge_rich_documents WHERE workspace_id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM knowledge_rich_document_drafts WHERE workspace_id IN (SELECT VALUE id FROM workspaces WHERE id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM knowledge_workspace_search_bookmark_states WHERE workspace_id IN (SELECT VALUE id FROM workspaces WHERE id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM knowledge_debug_breakpoints WHERE rich_document_id IN (SELECT VALUE id FROM knowledge_rich_documents WHERE workspace_id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM knowledge_debug_breakpoints WHERE workspace_id IN (SELECT VALUE id FROM workspaces WHERE id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM media_asset_tiers WHERE workspace_id IN (SELECT VALUE id FROM workspaces WHERE id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM media_asset_tiers WHERE asset_id IN (SELECT VALUE id FROM assets WHERE workspace_id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM loom_collections WHERE workspace_id IN (SELECT VALUE id FROM workspaces WHERE id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM loom_collection_members WHERE collection_id IN (SELECT VALUE id FROM loom_collections WHERE workspace_id = $workspace) AND (collection_id IN (SELECT VALUE id FROM loom_collections WHERE workspace_id = $workspace)) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM loom_collection_members WHERE asset_id IN (SELECT VALUE id FROM assets WHERE workspace_id = $workspace) AND (collection_id IN (SELECT VALUE id FROM loom_collections WHERE workspace_id = $workspace)) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM loom_ai_suggestions WHERE workspace_id IN (SELECT VALUE id FROM workspaces WHERE id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM loom_canvas_boards WHERE block_id IN (SELECT VALUE id FROM loom_blocks WHERE workspace_id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM loom_canvas_boards WHERE workspace_id IN (SELECT VALUE id FROM workspaces WHERE id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM loom_canvas_placements WHERE canvas_block_id IN (SELECT VALUE id FROM loom_canvas_boards WHERE workspace_id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM loom_canvas_placements WHERE workspace_id IN (SELECT VALUE id FROM workspaces WHERE id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM loom_canvas_visual_edges WHERE canvas_block_id IN (SELECT VALUE id FROM loom_canvas_boards WHERE workspace_id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM loom_canvas_visual_edges WHERE workspace_id IN (SELECT VALUE id FROM workspaces WHERE id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM loom_canvas_visual_edges WHERE from_placement_id IN (SELECT VALUE id FROM loom_canvas_placements WHERE workspace_id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM loom_canvas_visual_edges WHERE to_placement_id IN (SELECT VALUE id FROM loom_canvas_placements WHERE workspace_id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM loom_block_search_index WHERE block_id IN (SELECT VALUE id FROM loom_blocks WHERE workspace_id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM loom_block_search_index WHERE workspace_id IN (SELECT VALUE id FROM workspaces WHERE id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM calendar_activity_spans WHERE workspace_id IN (SELECT VALUE id FROM workspaces WHERE id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM stage_capture_artifacts WHERE workspace_id IN (SELECT VALUE id FROM workspaces WHERE id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM fems_memory_packs WHERE workspace_id IN (SELECT VALUE id FROM workspaces WHERE id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM fems_memory_proposals WHERE workspace_id IN (SELECT VALUE id FROM workspaces WHERE id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM fems_memory_items WHERE workspace_id IN (SELECT VALUE id FROM workspaces WHERE id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM fems_memory_commit_reports WHERE workspace_id IN (SELECT VALUE id FROM workspaces WHERE id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM fems_memory_commit_reports WHERE proposal_id IN (SELECT VALUE id FROM fems_memory_proposals WHERE workspace_id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM fems_memory_commit_fr_outbox WHERE workspace_id IN (SELECT VALUE id FROM workspaces WHERE id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM fems_memory_commit_fr_outbox WHERE proposal_id IN (SELECT VALUE id FROM fems_memory_proposals WHERE workspace_id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM fems_memory_commit_fr_outbox WHERE commit_id IN (SELECT VALUE id FROM fems_memory_commit_reports WHERE workspace_id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM fems_memory_lifecycle_fr_outbox WHERE workspace_id IN (SELECT VALUE id FROM workspaces WHERE id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM fems_memory_lifecycle_fr_outbox WHERE proposal_id IN (SELECT VALUE id FROM fems_memory_proposals WHERE workspace_id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };\nIF array::len(SELECT id FROM loom_block_view_fr_outbox WHERE workspace_id IN (SELECT VALUE id FROM workspaces WHERE id = $workspace) AND (workspace_id = $workspace) != true) > 0 { RETURN false; };\nFOR $source IN (SELECT id FROM knowledge_sources WHERE workspace_id = $workspace) { IF array::len(SELECT id FROM protected_resources WHERE id IN $resources.id AND resource_kind = 'knowledge_source' AND external_resource_id = record::id($source.id) AND lifecycle_state = 'active') != 1 { RETURN false; }; };\nFOR $source IN (SELECT id FROM knowledge_code_files WHERE workspace_id = $workspace) { IF array::len(SELECT id FROM protected_resources WHERE id IN $resources.id AND resource_kind = 'knowledge_code_file' AND external_resource_id = record::id($source.id) AND lifecycle_state = 'active') != 1 { RETURN false; }; };\nFOR $source IN (SELECT id FROM fems_memory_packs WHERE workspace_id = $workspace) { IF array::len(SELECT id FROM protected_resources WHERE id IN $resources.id AND resource_kind = 'memory_pack' AND external_resource_id = $external AND lifecycle_state = 'active') != 1 { RETURN false; }; };\nFOR $source IN (SELECT id FROM fems_memory_proposals WHERE workspace_id = $workspace) { IF array::len(SELECT id FROM protected_resources WHERE id IN $resources.id AND resource_kind = 'memory_proposal' AND external_resource_id = $external AND lifecycle_state = 'active') != 1 { RETURN false; }; };\nFOR $source IN (SELECT id FROM fems_memory_items WHERE workspace_id = $workspace) { IF array::len(SELECT id FROM protected_resources WHERE id IN $resources.id AND resource_kind = 'memory_item' AND external_resource_id = $external AND lifecycle_state = 'active') != 1 { RETURN false; }; };\nFOR $source IN (SELECT id FROM fems_memory_commit_reports WHERE workspace_id = $workspace) { IF array::len(SELECT id FROM protected_resources WHERE id IN $resources.id AND resource_kind = 'memory_commit_report' AND external_resource_id = $external AND lifecycle_state = 'active') != 1 { RETURN false; }; };\n    FOR $row IN (SELECT created_in_session_id FROM knowledge_source_roots WHERE workspace_id = $workspace) { IF $row.created_in_session_id = NONE OR $row.created_in_session_id.account_id != $account OR $row.created_in_session_id.access_space_id != $space { RETURN false; }; };\n    FOR $row IN (SELECT created_in_session_id FROM knowledge_index_runs WHERE workspace_id = $workspace) { IF $row.created_in_session_id = NONE OR $row.created_in_session_id.account_id != $account OR $row.created_in_session_id.access_space_id != $space { RETURN false; }; };\n    FOR $run IN (SELECT scope FROM knowledge_index_runs WHERE workspace_id = $workspace) {\n        IF $run.scope.source_ids = NONE { RETURN false; };\n        FOR $key IN $run.scope.source_ids {\n            IF type::record('knowledge_sources', $key).workspace_id != $workspace OR !fn::mt109_has_grant('knowledge_source', $key, 'delete', 'fs.write') { RETURN false; };\n        };\n    };\n    FOR $entity IN (SELECT * FROM knowledge_entities WHERE workspace_id = $workspace) {\n        IF $entity.entity_kind = 'loom_block' AND $entity.primary_source_id = NONE {\n            IF array::len(SELECT id FROM protected_resources WHERE id IN $resources.id AND resource_kind IN ['loom_block','rich_document'] AND external_resource_id = $entity.entity_key AND lifecycle_state = 'active') != 1 { RETURN false; };\n        } ELSE IF $entity.primary_source_id = NONE OR $entity.primary_source_id.workspace_id != $workspace OR !fn::mt109_has_grant('knowledge_source', record::id($entity.primary_source_id), 'delete', 'fs.write') { RETURN false; };\n        FOR $link IN (SELECT span_id FROM knowledge_entity_spans WHERE entity_id = $entity.id) {\n            IF $link.span_id.source_id.workspace_id != $workspace OR !fn::mt109_has_grant('knowledge_source', record::id($link.span_id.source_id), 'delete', 'fs.write') { RETURN false; };\n        };\n    };\n    FOR $edge IN (SELECT * FROM knowledge_edges WHERE workspace_id = $workspace) {\n        IF $edge.source_entity_id.workspace_id != $workspace OR $edge.target_entity_id.workspace_id != $workspace { RETURN false; };\n        FOR $link IN (SELECT span_id FROM knowledge_edge_spans WHERE edge_id = $edge.id) {\n            IF $link.span_id.source_id.workspace_id != $workspace OR !fn::mt109_has_grant('knowledge_source', record::id($link.span_id.source_id), 'delete', 'fs.write') { RETURN false; };\n        };\n    };\n    IF array::len(SELECT id FROM knowledge_ingestion_receipts WHERE workspace_id = $workspace AND (source_id.workspace_id = $workspace) != true) > 0\n        OR array::len(SELECT id FROM knowledge_ingestion_spans WHERE workspace_id = $workspace AND (source_id.workspace_id = $workspace AND receipt_id.workspace_id = $workspace AND receipt_id.source_id = source_id) != true) > 0\n        OR array::len(SELECT id FROM knowledge_ingestion_repair_queue WHERE workspace_id = $workspace AND (source_id.workspace_id = $workspace) != true) > 0\n        OR array::len(SELECT id FROM knowledge_code_repair_queue WHERE workspace_id = $workspace AND (source_id.workspace_id = $workspace) != true) > 0 { RETURN false; };\n    RETURN true;\n};\n"),
    ("DEFINE FUNCTION OVERWRITE fn::mt120_index_source_read($source: option<record<knowledge_sources>>, $workspace: record<workspaces>) {\n    RETURN $source != NONE AND $source.workspace_id = $workspace AND $source.source_kind IN ['file', 'rich_document']\n        AND fn::mt109_source_read('knowledge_source', record::id($source), record::id($workspace), 'workspace', record::id($workspace));\n};\n", "DEFINE FUNCTION OVERWRITE fn::mt120_index_source_read($source: option<record<knowledge_sources>>, $workspace: record<workspaces>) {\n    RETURN $source != NONE AND $source.workspace_id = $workspace AND $source.source_kind = 'file'\n        AND fn::mt109_source_read('knowledge_source', record::id($source), record::id($workspace), 'workspace', record::id($workspace));\n};\n"),
    ("DEFINE FUNCTION OVERWRITE fn::mt120_index_source_write($source: option<record<knowledge_sources>>, $workspace: record<workspaces>) {\n    RETURN $source != NONE AND $source.workspace_id = $workspace AND $source.source_kind IN ['file', 'rich_document']\n        AND fn::mt109_has_grant('knowledge_source', record::id($source), 'update', 'memory.propose')\n        AND fn::mt109_has_workspace_access(record::id($workspace), 'read', 'memory.read');\n};\n", "DEFINE FUNCTION OVERWRITE fn::mt120_index_source_write($source: option<record<knowledge_sources>>, $workspace: record<workspaces>) {\n    RETURN $source != NONE AND $source.workspace_id = $workspace AND $source.source_kind = 'file'\n        AND fn::mt109_has_grant('knowledge_source', record::id($source), 'update', 'memory.propose')\n        AND fn::mt109_has_workspace_access(record::id($workspace), 'read', 'memory.read');\n};\n"),
];
/// End (exclusive, just past the terminating `;`) of the top-level SurrealQL statement starting
/// at `start`: quoted text and `--` comments are skipped and `{}` nesting is tracked.
fn surql_statement_end(source: &str, start: usize) -> Option<usize> {
    let bytes = source.as_bytes();
    let mut depth = 0_i64;
    let mut quote: Option<u8> = None;
    let mut index = start;
    while index < bytes.len() {
        let byte = bytes[index];
        if let Some(delimiter) = quote {
            if byte == b'\\' {
                index += 2;
                continue;
            }
            if byte == delimiter {
                quote = None;
            }
        } else {
            match byte {
                b'\'' | b'"' => quote = Some(byte),
                b'-' if bytes.get(index + 1) == Some(&b'-') => {
                    index += source[index..].find('\n')?;
                }
                b'{' => depth += 1,
                b'}' => depth -= 1,
                b';' if depth == 0 => return Some(index + 1),
                _ => {}
            }
        }
        index += 1;
    }
    None
}

/// Offset of `line` in `source`: its unique full-line occurrence or, when it never fills a whole
/// line, its unique substring occurrence.
fn unique_line_offset(source: &str, line: &str) -> Option<usize> {
    let bytes = source.as_bytes();
    let (mut full, mut full_count, mut partial, mut partial_count) = (0, 0_usize, 0, 0_usize);
    for (offset, _) in source.match_indices(line) {
        partial = offset;
        partial_count += 1;
        let after = offset + line.len();
        if (offset == 0 || bytes[offset - 1] == b'\n')
            && (after == bytes.len() || bytes[after] == b'\n')
        {
            full = offset;
            full_count += 1;
        }
    }
    match (full_count, partial_count) {
        (1, _) => Some(full),
        (0, 1) => Some(partial),
        _ => None,
    }
}

/// Adds to `spans` (start -> end in [`SCHEMA`]) the complete `DEFINE ... OVERWRITE` statement
/// enclosing each uniquely located line of `delta`; returns how many lines were located.
fn schema_statements_enclosing(delta: &str, spans: &mut BTreeMap<usize, usize>) -> usize {
    let mut located = 0;
    for line in delta.split('\n') {
        if line.trim().len() < 8 {
            continue;
        }
        let Some(offset) = unique_line_offset(SCHEMA, line) else {
            continue;
        };
        let Some(head) = SCHEMA[..offset + line.len()].rfind("\nDEFINE ") else {
            continue;
        };
        let start = head + 1;
        let Some(end) = surql_statement_end(SCHEMA, start) else {
            continue;
        };
        let first_line = SCHEMA[start..].split('\n').next().unwrap_or_default();
        if end <= offset || !first_line.contains(" OVERWRITE ") {
            continue;
        }
        spans.insert(start, end);
        located += 1;
    }
    located
}

/// Every statement changed by the revision-160 deltas (MT-154, MT-109 C3/C2, standalone Loom update,
/// MT-109 C1), re-emitted verbatim from [`SCHEMA`] so an upgraded store reaches the exact current
/// catalog: the MT-154 authority block first, then the complete enclosing `DEFINE ... OVERWRITE`
/// statement of every delta pair, deduplicated, in schema order. A new `MT154_SCHEMA_DELTAS` pair is
/// re-emitted with no further code. Pairs whose text a later delta rewrote are covered by that later
/// pair. Only OVERWRITE definitions are emitted (no data statements).
fn schema_delta_upgrade_statements() -> String {
    let mut statements = String::new();
    let mut block = None;
    if let Some(begin) = SCHEMA.find(MT154_AUTHORITY_BLOCK_BEGIN) {
        let start = begin + 1;
        let end = SCHEMA[begin..]
            .find(MT154_AUTHORITY_BLOCK_END)
            .map(|offset| begin + offset + MT154_AUTHORITY_BLOCK_END.len())
            .expect("MT-154 authority block must terminate");
        statements.push_str(&SCHEMA[start..end]);
        block = Some(start..end);
    }
    let mut spans = BTreeMap::new();
    for deltas in [
        MT154_SCHEMA_DELTAS,
        &MT109_C3_SCHEMA_DELTAS[..],
        &MT109_C2_SCHEMA_DELTAS[..],
        &STANDALONE_LOOM_UPDATE_SCHEMA_DELTAS[..],
        MT109_C1_SCHEMA_DELTAS,
    ] {
        for (current, _) in deltas {
            schema_statements_enclosing(current, &mut spans);
        }
    }
    for (start, end) in spans {
        if block.as_ref().is_some_and(|range| range.contains(&start)) {
            continue;
        }
        statements.push_str(&SCHEMA[start..end]);
        statements.push('\n');
    }
    statements
}

#[cfg(test)]
fn restore_pre_mt154_schema(mut source: String) -> String {
    if let Some(start) = source.find(MT154_AUTHORITY_BLOCK_BEGIN) {
        let end = source[start..]
            .find(MT154_AUTHORITY_BLOCK_END)
            .map(|offset| start + offset + MT154_AUTHORITY_BLOCK_END.len())
            .expect("MT-154 authority block must terminate");
        source.replace_range(start..end, "");
    }
    for (current, previous) in MT154_SCHEMA_DELTAS {
        source = source.replace(current, previous);
    }
    source
}
/// Exact revision-159 catalog: the current schema with the MT-154, MT-109 C3/C2, standalone Loom
/// update and MT-109 C1 changes reverted.
#[cfg(test)]
fn pre_standalone_loom_update_schema() -> String {
    let mut predecessor =
        restore_pre_standalone_loom_update_schema(restore_pre_mt154_schema(SCHEMA.to_owned()));
    for (current, previous) in MT109_C1_SCHEMA_DELTAS {
        predecessor = predecessor.replace(current, previous);
    }
    predecessor
}

#[cfg(test)]
fn pre_canvas_receipt_schema() -> String {
    const CREATOR_RECEIPT_START: &str =
        "        OR (!$write AND $session = $auth.id AND fn::mt109_ledger_access";
    const READER_RECEIPT_START: &str = "        OR (!$write AND fn::mt120_loom_block_access";
    let mut predecessor = pre_standalone_loom_update_schema();
    let start = predecessor
        .find(CREATOR_RECEIPT_START)
        .expect("Canvas creator-session receipt branch must exist exactly once");
    assert_eq!(
        predecessor[start + CREATOR_RECEIPT_START.len()..]
            .matches(CREATOR_RECEIPT_START)
            .count(),
        0,
        "Canvas creator-session receipt branch must be unique"
    );
    let end = start
        + predecessor[start..]
            .find(READER_RECEIPT_START)
            .expect("Canvas reader receipt branch must follow creator-session branch");
    predecessor.replace_range(start..end, "");
    predecessor
}

/// The `loom_blocks` table statement exactly as the revision-157 pin (8d4ed30d) shipped it. The MT-109
/// C1-C3 Loom record-user permission rewrites (01df5ebf, ec34c88e, a1e18d79) have no revert pair in the
/// revision-157 chain, so [`pre_account_setup_schema`] restores the whole statement (kb-c5 emulation:
/// with it the chain reproduces PRE_ACCOUNT_SETUP_GENERATED_SHA256 byte for byte).
#[cfg(test)]
const PRE_ACCOUNT_SETUP_LOOM_BLOCKS_TABLE: &str = r#"DEFINE TABLE OVERWRITE loom_blocks SCHEMAFULL
    PERMISSIONS FOR select WHERE (source_rich_document_id = NONE AND fn::mt109_source_read('loom_block', block_id, record::id(workspace_id), 'workspace', record::id(workspace_id))) OR (source_rich_document_id != NONE AND record::id(source_rich_document_id) = block_id AND content_type = 'note' AND fn::mt109_source_read('loom_block', block_id, record::id(workspace_id), 'rich_document', record::id(source_rich_document_id)) AND fn::mt109_source_read('rich_document', record::id(source_rich_document_id), record::id(workspace_id), 'workspace', record::id(workspace_id)) AND source_rich_document_id.workspace_id = workspace_id AND source_rich_document_id.deleted_at = NONE AND content_hash = source_rich_document_id.content_sha256)
                FOR create, update, delete NONE;"#;

#[cfg(test)]
fn pre_account_setup_schema() -> String {
    // MT-154 is reverted first so every block removal below sees the 0cfbff64 text it matches.
    let source = restore_pre_mt154_schema(SCHEMA.to_owned());
    let record_user_update_guard_events = record_user_update_guard_event_block(&source).to_owned();
    let mut previous = source
        .replace(&format!("\n{LOCAL_ACCOUNT_SETUP_STATEMENTS}"), "")
        .replace(&format!("{MT120_DOCUMENT_ACCESS_BLOCK}\n"), "")
        .replace(MT120_IDEMPOTENCY_BLOCK, "")
        .replace(&format!("\n{RECORD_USER_PRODUCER_BLOCK}"), "")
        .replace(&format!("\n{AUTHORITY_NONCE_EVENT_STATEMENTS}"), "")
        .replace(&format!("\n{record_user_update_guard_events}\n"), "");
    previous = restore_pre_standalone_loom_update_schema(previous);
    previous = restore_pre_mt120_record_user_update_guards(previous);
    previous = restore_pre_account_authority_nonce_guards(previous);
    previous = restore_pre_mt120_loom_bundle(previous);
    for (old, current) in MT120_DOCUMENT_TABLE_UPGRADES.iter().rev() {
        previous = previous.replace(current, old);
    }
    let (start, end) = schema_table_definition_bounds(&previous, "loom_blocks");
    previous.replace_range(start..end, PRE_ACCOUNT_SETUP_LOOM_BLOCKS_TABLE);
    previous
}
/// Stable v1 lineage identifier retained so existing embedded stores remain readable after the
/// legacy schema-provenance corpus is removed. New source integrity is proven independently by
/// [`DECLARATIVE_SCHEMA_CATALOG_SHA256`] and [`GENERATED_SURREALQL_SHA256`].
pub const SCHEMA_LINEAGE_SHA256: &str =
    "225ed19c0259ef121867ca5da1995813db0c48ee0cbfaded2d871e47b50f7fc1";
// MT-142 re-pin: the predecessor artifact is derived from the current schema.surql
// (see `mt139_exact_predecessor_upgrade_preserves_data_and_restarts_current`), so it
// moves with the knowledge_rich_document_title_anchors block.
// MT-151 re-pin: moves again with loom_blocks.journal_key and storage_graph_anchors.
// MT-152 re-pin: moves again with fems_workspace_write_anchors, then with
// loom_folders.sibling_key / uq_loom_folders_sibling_key (I-152-2 sweep finding; offline
// recomputation, same derivation as the test).
const PREDECESSOR_GENERATED_SURREALQL_SHA256: &str =
    "cd6c96a495e65b24958c4583a3d9935645e25b765ff7403911e08c3c72804a7e";
// MT-142 re-pin: the synthesized predecessor store (derived from the current schema.surql
// with the retired registry field) now carries knowledge_rich_document_title_anchors.
// MT-151 re-pin: it now also carries journal_key and storage_graph_anchors, with table catalog
// ids stripped (run mt142-LIB-20260911T124916Z, HANDSHAKE_SURREAL_PREDECESSOR_INFO_FINGERPRINT_MISMATCH observed).
// MT-152 re-pin: it now also carries fems_workspace_write_anchors (run mt142-LIB-20260911T162250Z,
// HANDSHAKE_SURREAL_PREDECESSOR_INFO_FINGERPRINT_MISMATCH observed).
// MT-152 re-pin (I-152-2 sweep finding): it now also carries loom_folders.sibling_key and its
// UNIQUE index (run mt142-LIB-20260911T234013Z, HANDSHAKE_SURREAL_PREDECESSOR_INFO_FINGERPRINT_MISMATCH observed).
const PREDECESSOR_SCHEMA_INFO_SHA256: &str =
    "3d704adc7ddbf6ec802567aaa614cacedc97f76123b18b66e8ed6ecd1ba81cde";
const PREDECESSOR_KNOWLEDGE_REGISTRY_SHA256: &str =
    "1f8443486cd7101babb56dd6264ffcf08538a1eae24016d2155b19d5eb6370b4";
// MT-142 re-pin: schema.surql gained knowledge_rich_document_title_anchors.
// MT-151 re-pin: schema.surql gained loom_blocks.journal_key and storage_graph_anchors.
// MT-152 re-pin: schema.surql gained fems_workspace_write_anchors, then
// loom_folders.sibling_key / uq_loom_folders_sibling_key (I-152-2 sweep finding).
// MT-150 re-pin: schema.surql gained loom_edges.event_ledger_event_id / idx_loom_edges_event
// (previous value 0b9e32329d735477064393a1406825f94aa55db94f6256e3206f02166ae13dc6, the MT-109
// pin, retained as PRE_MT150_GENERATED_SURREALQL_SHA256).
// MT-141 R9 re-pin: schema.surql moved the composite provenance-ref constraint onto
// atelier_media_source_provenance_ref.asset_id (previous value
// 5cc902f3afe7691b07338a6170a16a35535e6ff40a1c119a4ccc65c3464c7779, the MT-150 pin, retained as
// PRE_MT141_GENERATED_SURREALQL_SHA256).
// MT-141 V2-R2 re-pin: schema.surql changed loom_blocks.pin_order (ASSERT dropped), widened the
// knowledge_quick_switcher_recents kind unions, and gained the two
// knowledge_crdt_ai_edit_proposals applied-binding fields, and loom_block_view_fr_outbox.block_id
// lost its cascading REFERENCE (previous value
// 05b36f65e0f2328d389c7ca460f2b9846b13d3be527d16dadb244be6f8e3bcfd, the MT-141 R9 pin; the
// R9 hop was never a released lineage, so the MT-150 pin remains the allowlisted predecessor).
// MT-109 C1-FDELETE re-pin (Operator decision 2026-09-22): loom_canvas_boards gained an owner
// delete permission so an authorized workspace delete can remove its Canvas boards (previous value
// 2c2af7bda13f220bcb00841ffdd439f3d0e1fc4f07df0a9ff8b64d68e73a891f; rev 160 unreleased, re-pinned in place).
// MT-109 C1V-WS-CREATE-403 re-pin: mt109_workspace_reconciliation_queue now fires only for
// record-user creates (`$auth != NONE`), so privileged/system workspace creation no longer throws
// (previous value 7544939ae1b81363773e6eb050797c81ce9aa3926ba7ba32109c940e0dd06b36).
// MT-109 round-4 re-pin (Master Spec LM-RLS-001 / §2.3.13.12.4): record-user Loom creation of all
// spec content types, rich-document projection placements, and writer access to a same-id
// projection across a save (previous value 01a3f6068b040794f3c670b919a46324d283da1374f09a72c3a98bf24bfe2653).
// MT-109 C2 re-pin: workspace delete accepts account Loom block entities; loom_block update receipt;
// Canvas visual-edge / Atelier projection delete; standalone search-index update; memory surfaces;
// rich-document knowledge sources; backlink link kinds; rich-document projection placement
// receipts (previous value
// 79444332fefc2fe7cf5950152bb7d29ddbbd0452f30a17f4abb277b6cbb6504a); kb-c2 runs 03/18.
// MT-109 C3 re-pin: record-user permissions for the account-scoped Loom routes, the Loom endpoint
// helper and workspace/block Loom receipts (previous value
// dcb91130af87c675f199db0afa9483fbbc60dc40b925870dfd0bcf1b340e1049); kb-c3 run 04.
// MT-154 C4 re-pin (batch MT-153..MT-157): record-user permissions for the account-scoped
// non-Loom surfaces, owner_account_id on account-global tables, job/workflow predicates, the Loom
// identity guard and the MT154_AUTHORITY block (previous value
// 41bbb694d0b7c45a5fcd95eb71f0010d1b28ca09440b9593f6f769bac8b55121); kb-c4 runs 26/28.
// MT-154/MT-158 re-pin: kernel_crdt_updates owner-delete permission (workspace delete cascade),
// mt_iterations/dependencies owner_account_id + permissions, the per-account dependency-graph
// anchor branch and fn::mt158_dependency_graph_scope (previous value
// ba67f200ef98758d62b2b307efae4efdc220d1ef07bf6ef44551ce389d57d271); kb-c4 run 82 (equals
// sha256 of schema.surql).
// MT-159 re-pin: owner-scoped Locus keys (key ASSERTs, (owner_account_id, id) indexes, create
// permission owner-key check, fn::mt159_locus_key_id / fn::mt159_locus_key_is_own) (previous
// value 35ce1b4d6578eedd5ef6f054f3b70432f94b2728fd9ae98cd4d35dd32e8974be); kb-c4 run 101.
// MT-159 re-pin: job-scoped record-user permissions on model_sessions / model_session_checkpoints /
// model_session_messages (previous value
// 27528c0e71735b81fbaa798dabb9e35b3805a16166db450d86fec4baaa32c38f); kb-c5 run 05 (equals sha256 of
// schema.surql). DECLARATIVE_SCHEMA_CATALOG_SHA256 is unchanged by this batch (kb-c5 run 05).
pub const GENERATED_SURREALQL_SHA256: &str =
    "d7f69584c809ae71dd25eddeecf5af626607ec24455dddb9bd2f67a0f35aecd1";
// MT-142 re-pin: catalog identities gained the knowledge_rich_document_title_anchors objects.
// MT-151 re-pin: catalog identities gained the journal_key field/index and the
// storage_graph_anchors objects.
// MT-152 re-pin: catalog identities gained the fems_workspace_write_anchors objects, then the
// loom_folders sibling_key field/index (I-152-2 sweep finding).
// MT-150 re-pin: catalog identities gained the loom_edges event_ledger_event_id field/index
// (previous value 9881bff3f6bd7d02797fb95c88ad51f1ae6f19777f89477e3285cc014c37d014, MT-109).
// MT-141 re-pin: catalog identities gained the atelier_saved_search_retrieval_projection objects and
// the changed atelier_media_source_provenance_ref.asset_id definition (previous value
// 70e5b64ba1141642e37bf7fff598b596f82cc7520cec40829a442fb2cced2754, MT-150).
// MT-141 V2-R2 re-pin: catalog identities changed for loom_blocks.pin_order, the
// knowledge_quick_switcher_recents kind unions, and gained the two ai_edit applied-binding
// fields (previous value b90f7345927316be15eb3f7bca0ba033df064d16326d37e3942677ecb84ce99c, the
// MT-141 R9 pin); observed by `declarative_schema_catalog_is_complete_and_content_sensitive`.
// MT-109 C3 re-pin: Loom record-user permissions and fn::mt120_loom_endpoint_access (previous
// 01e4c14cc6e3d75239c053974c5de2c4168c2ae50d76d3f418819ce7d7e993fd); kb-c3 run 04.
// MT-154/MT-158 re-pin (same batch as GENERATED_SURREALQL_SHA256; previous
// c6a1846bd23c9706a61feac9f267b6f32c77fb8f1b5faa2358d88b2d83a42436); kb-c4 run 82.
// MT-159 re-pin (previous 54b282b48cf22a2f02ae0d2964b659b01c53da3177c9e95ae5d70dce358fce7c); kb-c4
// run 101.
pub const DECLARATIVE_SCHEMA_CATALOG_SHA256: &str =
    "a50bd154872a5cffe14e3d1ffe8823852d70a76f4a7a2853ecd91736f461238c";
// MT-142 re-pin: the seed gained the rich_document_title_anchors registry row (63 rows).
pub const KNOWLEDGE_SCHEMA_REGISTRY_SEED_SHA256: &str =
    "64d0711c5273c6eb103c3d574b2f7ee98d9d0ebfd46e9c25ad65908b46573b75";
/// Fresh-engine STRUCTURE fingerprint captured with the product-locked SurrealDB 3.2.0
/// engine family after applying the generated schema to an absent RocksDB path.
// MT-142 re-pin: live STRUCTURE fingerprint with knowledge_rich_document_title_anchors applied.
// MT-151 re-pin: live STRUCTURE fingerprint with journal_key and storage_graph_anchors applied
// and engine table catalog ids stripped (see `inspect_schema`); run mt142-LIB-20260911T124916Z,
// `mt139_current_schema_info_pin_matches_fresh_mem_catalog`, and reached identically by the
// in-place MT-151 upgrade (`mt151_exact_mt142_pin_upgrade_materialises_journal_key_and_restarts_current`).
// MT-152 re-pin: live STRUCTURE fingerprint with fems_workspace_write_anchors applied; run
// mt142-LIB-20260911T160117Z, `mt139_current_schema_info_pin_matches_fresh_mem_catalog`, and
// reached identically by the in-place MT-152 upgrade
// (`mt152_exact_mt151_pin_upgrade_adds_fems_write_anchors_and_restarts_current`).
// MT-152 re-pin (I-152-2 sweep finding): loom_folders.sibling_key / uq_loom_folders_sibling_key
// applied; run mt142-LIB-20260911T233822Z, `mt139_current_schema_info_pin_matches_fresh_mem_catalog`
// observed, and reached identically by both in-place MT-152 upgrade proofs.
// MT-150 re-pin: live STRUCTURE fingerprint with loom_edges.event_ledger_event_id /
// idx_loom_edges_event applied (previous value
// f58198becbec2c5d922d98ae742596ba603566d53c6c8c5b90447aaf1c9c0384, the MT-109 pin, retained as
// PRE_MT150_SCHEMA_INFO_SHA256); observed by `mt139_current_schema_info_pin_matches_fresh_mem_catalog`
// and reached identically by the in-place MT-150 upgrade
// (`mt150_exact_mt109_pin_upgrade_adds_loom_edge_receipt_field_and_restarts_current`).
// MT-141 R9 re-pin: live STRUCTURE fingerprint with the provenance-ref constraint on
// atelier_media_source_provenance_ref.asset_id (previous value
// a4a7ef4c4f92e25186dcb4d0f331d22b150687b4102786d3a9a871028e7e93e7, the MT-150 pin, retained as
// PRE_MT141_SCHEMA_INFO_SHA256); observed by `mt139_current_schema_info_pin_matches_fresh_mem_catalog`
// and reached identically by the in-place MT-141 upgrade
// (`mt141_exact_mt150_pin_upgrade_moves_provenance_ref_constraint_and_restarts_current`).
// MT-141 V2-R2 re-pin: live STRUCTURE fingerprint with the pin_order ASSERT dropped, the
// quick-switcher kind unions widened, the ai_edit applied-binding fields applied and the
// block-view outbox `block_id` REFERENCE dropped (previous value
// 91ed6b88d18917d21bb31bfde164ba0d46c8e8f17e87f36f34ca8ff762f12ff6, the MT-141 R9 pin);
// observed by `mt139_current_schema_info_pin_matches_fresh_mem_catalog` (kb-v2 run 41,
// MT139_CURRENT_SCHEMA_INFO_SHA256).
// Revision 158 measured by the independent MT-109/wpv-v20 embedded-engine probes.
// MT-109 C1-FDELETE re-pin: loom_canvas_boards owner delete permission applied (previous value
// c12100ea03d08fe923149b1a128d7ebf1d30ac17d4ef7eb1d77e28d52f0191df); observed by
// `mt139_current_schema_info_pin_matches_fresh_mem_catalog` (kb-c1 run 42).
// MT-109 C1V-WS-CREATE-403 re-pin: record-user-only workspace reconciliation event (previous value
// 8955551a907913596f6751ac4a96ad4686b9c98889c293310900874cc06a1ca8); kb-c1 run 54.
// MT-109 round-4 re-pin (previous d06f44fd00b2525e93d55d4884a81d092a26c4e08b486da3b03b4671a035d3eb);
// kb-c1 run 76.
// MT-109 C2 re-pin (previous a99410f0c48491e110068e2d10d3ab0fc526073b6af749ce17afb85fe7e549f6); kb-c2 runs 03/18.
// MT-109 C3 re-pin (previous 13be68fc1edba9476e4cdd2db98f51b02ee33190683472fca3bb4cf57ee7225c); kb-c3 run 04.
// MT-154/MT-158 re-pin (previous ad5ff085fc07966c351da7cd56e2e423577d8ff58240d97a25ee5107048eb363);
// kb-c4 run 82.
// MT-159 re-pin (previous 5cb894682f37a18549316cc96e73167171ec21fa71e381da55f6d25d507e52f3);
// kb-c4 run 101.
// MT-159 re-pin: model_session* job-scoped permissions (previous
// 33f106de7ee1f2f5caa33fe7e2de5e61ba7d563d8064c727c89248dbc43fa86c); kb-c5 run 05.
pub const EXPECTED_SCHEMA_INFO_SHA256: &str =
    "0c59fb082fac2c52b79fc530c571794b8c933fde9305303ef11511093690a53a";
// MT-141 R9 re-pin: atelier_media_source_provenance_ref.asset_id definition changed (previous
// value 25cd85bc8267363891ef9bcece05b2e41b4aa0762e8384f86f4a1563e1d43585, MT-150).
// MT-141 re-pin (second hop): the atelier catalog gained atelier_saved_search_retrieval_projection
// (previous value 4d3f739296e5b59bd3962c0fab23180dd5e3363b8fc7ad277dc6ba17da9f1c63, the asset_id-only
// MT-141 pin; before that 25cd85bc8267363891ef9bcece05b2e41b4aa0762e8384f86f4a1563e1d43585, MT-150).
// MT-109 round-4 re-pin: fn::mt120_resource_create / fn::mt120_loom_receipt admit all spec Loom
// content types (previous ad696e28be444fd68b39f4c280400ae4914c17234ed0888794df75ed1de6483c).
// MT-109 C2 re-pin: Atelier projection delete follows the workspace delete grant (previous
// ed84249709a9ab9c3c7d4304859732fac8373d0001a27aa84685c319e6a6a04a).
// MT-109 C3 re-pin: fn::mt120_loom_endpoint_access and the Loom receipt branches (previous
// f8909f93910491ac4c95bb8f4b567dd2c572b6e08b5ca899fb0b223dbb2ca411); kb-c3 run 04.
const EXPECTED_ATELIER_CATALOG_SHA256: &str =
    "469a5fce3f0aac78dbc65638a6459da04877e0148ab9f2472524b5c6a9130e03";
const PENDING_SCHEMA_INFO_SHA256: &str =
    "0000000000000000000000000000000000000000000000000000000000000000";
/// Second allowlisted lineage (MT-142): every store bootstrapped at schema revision 157 before
/// `knowledge_rich_document_title_anchors` existed. These are the exact pre-MT-142 pins of
/// [`GENERATED_SURREALQL_SHA256`] and [`EXPECTED_SCHEMA_INFO_SHA256`]; such stores are upgraded
/// in place by `upgrade_pre_mt142_current` instead of failing closed.
const PRE_MT142_GENERATED_SURREALQL_SHA256: &str =
    "b4bcdbd16ffbbb3d9543f164f4226d3d952c841e80a7bd9c302b82cb15e3d4f9";
const PRE_MT142_SCHEMA_INFO_SHA256: &str =
    "685bc539ddd8864c773bb8bb599768570faa66a4ff17ee7ab24e10d6e2b2db41";
/// Third allowlisted lineage (MT-151): every store bootstrapped at the MT-142 pin, before
/// `loom_blocks.journal_key` / `uq_loom_blocks_journal_key` and `storage_graph_anchors` existed.
/// These are the exact MT-142 pins of [`GENERATED_SURREALQL_SHA256`] and
/// [`EXPECTED_SCHEMA_INFO_SHA256`]; such stores are upgraded in place by
/// `upgrade_pre_mt151_current`. Pre-MT-142 stores receive both upgrades in one transaction.
const PRE_MT151_GENERATED_SURREALQL_SHA256: &str =
    "ecfdca9826223629277a218c7f22a3d6aaabf0714274cf0358cc0ab5a8d562a0";
const PRE_MT151_SCHEMA_INFO_SHA256: &str =
    "e117afdb9a7ff9ded218b29a5741b5fbf2541170f0772e475475114fca42a994";
/// Fourth allowlisted lineage (MT-152): every store bootstrapped at the MT-151 pin, before
/// `fems_workspace_write_anchors` existed. These are the exact MT-151 pins of
/// [`GENERATED_SURREALQL_SHA256`] and [`EXPECTED_SCHEMA_INFO_SHA256`]; such stores are upgraded
/// in place by `upgrade_pre_mt152_current`. Pre-MT-142 and pre-MT-151 stores receive the
/// MT-152 statements inside their own upgrade transaction, because the finalize gate pins the
/// current fingerprint.
const PRE_MT152_GENERATED_SURREALQL_SHA256: &str =
    "a8bb72c7fd73c1a2ea0b9563ee2f0534bd9cd435d153975b61bf6794ff9a598a";
const PRE_MT152_SCHEMA_INFO_SHA256: &str =
    "294530f11ca454f1afba332ac9e70e909ab39cac12ce35ad661daff2fd0ff222";
/// Sixth allowlisted lineage (MT-150): every store bootstrapped at the MT-109 pin, before
/// `loom_edges.event_ledger_event_id` / `idx_loom_edges_event` existed. These are the exact
/// MT-109 pins of [`GENERATED_SURREALQL_SHA256`] and [`EXPECTED_SCHEMA_INFO_SHA256`]; such
/// stores are upgraded in place by `upgrade_pre_mt150_current`. Every older allowlisted
/// lineage receives the MT-150 statements inside its own upgrade transaction, because the
/// finalize gate pins the current fingerprint.
const PRE_MT150_GENERATED_SURREALQL_SHA256: &str =
    "0b9e32329d735477064393a1406825f94aa55db94f6256e3206f02166ae13dc6";
const PRE_MT150_SCHEMA_INFO_SHA256: &str =
    "f58198becbec2c5d922d98ae742596ba603566d53c6c8c5b90447aaf1c9c0384";
/// Seventh allowlisted lineage (MT-141 R9): every store bootstrapped at the MT-150 pin, before
/// the composite "at least one provenance ref" constraint moved from the optional `run_ref`
/// field (where the engine never evaluates an ASSERT for a NONE value, so an all-NONE row was
/// accepted) onto the always-evaluated required `asset_id` field of
/// `atelier_media_source_provenance_ref`. These are the exact MT-150 pins of
/// [`GENERATED_SURREALQL_SHA256`] and [`EXPECTED_SCHEMA_INFO_SHA256`]; such stores are upgraded
/// in place by `upgrade_pre_mt141_current`. Every older allowlisted lineage receives the MT-141
/// statement inside its own upgrade transaction, because the finalize gate pins the current
/// fingerprint.
const PRE_MT141_GENERATED_SURREALQL_SHA256: &str =
    "5cc902f3afe7691b07338a6170a16a35535e6ff40a1c119a4ccc65c3464c7779";
const PRE_MT141_SCHEMA_INFO_SHA256: &str =
    "a4a7ef4c4f92e25186dcb4d0f331d22b150687b4102786d3a9a871028e7e93e7";
/// MT-141 R9 upgrade statement: `DEFINE FIELD OVERWRITE` is idempotent, so re-defining
/// `asset_id` with the composite constraint is the whole delta. Must stay identical to
/// `schema.surql` (proven by `mt141_upgrade_statements_match_schema`). No backfill: every row the
/// product writer produced already carries at least one ref; a pre-existing all-NONE row would
/// be rejected on its next write, which is the constraint's purpose.
const MT141_SAVED_SEARCH_PROJECTION_BLOCK: &str = "\
DEFINE TABLE OVERWRITE atelier_saved_search_retrieval_projection SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE projection_id ON TABLE atelier_saved_search_retrieval_projection TYPE uuid ASSERT ($value = record::id($this.id));
DEFINE FIELD OVERWRITE saved_search_id ON TABLE atelier_saved_search_retrieval_projection TYPE string;
DEFINE FIELD OVERWRITE asset_id ON TABLE atelier_saved_search_retrieval_projection TYPE string;
DEFINE FIELD OVERWRITE content_hash ON TABLE atelier_saved_search_retrieval_projection TYPE string;
DEFINE FIELD OVERWRITE artifact_ref ON TABLE atelier_saved_search_retrieval_projection TYPE string;
DEFINE FIELD OVERWRITE jump_target ON TABLE atelier_saved_search_retrieval_projection TYPE string;
DEFINE FIELD OVERWRITE tags_json ON TABLE atelier_saved_search_retrieval_projection TYPE array DEFAULT [];
DEFINE FIELD OVERWRITE tags_json.* ON TABLE atelier_saved_search_retrieval_projection TYPE any;
DEFINE FIELD OVERWRITE favorite ON TABLE atelier_saved_search_retrieval_projection TYPE bool DEFAULT false;
DEFINE FIELD OVERWRITE rating ON TABLE atelier_saved_search_retrieval_projection TYPE int DEFAULT 0;
DEFINE FIELD OVERWRITE matched_color_hex ON TABLE atelier_saved_search_retrieval_projection TYPE option<string>;
DEFINE FIELD OVERWRITE content_tier ON TABLE atelier_saved_search_retrieval_projection TYPE option<string>;
DEFINE FIELD OVERWRITE view_mode ON TABLE atelier_saved_search_retrieval_projection TYPE 'NSFW' | 'SFW' DEFAULT 'NSFW';
DEFINE FIELD OVERWRITE created_at_utc ON TABLE atelier_saved_search_retrieval_projection TYPE datetime DEFAULT time::now();
DEFINE INDEX OVERWRITE pk_atelier_saved_search_retrieval_projection ON TABLE atelier_saved_search_retrieval_projection FIELDS projection_id UNIQUE;
DEFINE INDEX OVERWRITE uq_atelier_saved_search_retrieval_projection_1 ON TABLE atelier_saved_search_retrieval_projection FIELDS saved_search_id, asset_id UNIQUE;
DEFINE INDEX OVERWRITE idx_atelier_saved_search_retrieval_projection_search ON TABLE atelier_saved_search_retrieval_projection FIELDS saved_search_id;
";
const MT141_PROVENANCE_REF_ASSERT_LINE: &str = "\
DEFINE FIELD OVERWRITE asset_id ON TABLE atelier_media_source_provenance_ref TYPE record<atelier_media_asset> ASSERT (record::exists($value)) AND (record::id($value) = record::id($this.id)) AND ($this.source_url_ref != NONE OR $this.source_path_ref != NONE OR $this.source_note_ref != NONE OR $this.contact_sheet_ref != NONE OR $this.task_ref != NONE OR $this.run_ref != NONE) REFERENCE ON DELETE CASCADE;
";
/// MT-141 V2-R2 (validation_v2 V2-F02) lineage additions, each a `DEFINE FIELD OVERWRITE`
/// (idempotent, no backfill). Every line must stay identical to `schema.surql` (proven by
/// `mt141_upgrade_statements_match_schema`).
///
/// * `loom_blocks.pin_order` loses the `>= 0` ASSERT the port introduced: the storage contract
///   (`set_loom_block_pin_order(Option<i32>)`, MT-183) is a signed user-controlled ordinal and
///   "move to front" is written as a negative ordinal; the Master Spec is silent on the domain,
///   so the pre-port product behaviour is the authority and the ASSERT was the drift.
/// * `knowledge_quick_switcher_recents.{source_kind,result_kind}` are widened to the product's
///   typed `LoomSearchSourceKind` / `LoomSearchResultKind` unions (`storage/loom.rs`); the
///   schema literal unions were a stale subset and rejected `file` / `wiki_page` recents.
/// * `knowledge_crdt_ai_edit_proposals.{applied_update_id,applied_update_sha256}` are the
///   authority-hardening #5 binding columns the writer (`bind_applied_ai_edit_update`) sets and
///   the reader projects; they were lost in the port. The sha256 ASSERT is the former 0192
///   CHECK backstop: a bound hash must equal the approved `diff_sha256`.
const MT141_LOOM_PIN_ORDER_LINE: &str = "\
DEFINE FIELD OVERWRITE pin_order ON TABLE loom_blocks TYPE option<int>;
";
const MT141_QUICK_SWITCHER_SOURCE_KIND_LINE: &str = "\
DEFINE FIELD OVERWRITE source_kind ON TABLE knowledge_quick_switcher_recents TYPE 'loom_block' | 'file' | 'tag_hub' | 'document' | 'symbol' | 'work_packet' | 'micro_task' | 'user_manual_page' | 'wiki_page';
";
const MT141_QUICK_SWITCHER_RESULT_KIND_LINE: &str = "\
DEFINE FIELD OVERWRITE result_kind ON TABLE knowledge_quick_switcher_recents TYPE 'loom_block' | 'knowledge_entity' | 'user_manual_page' | 'wiki_page';
";
/// MT-027 (migration 0362, lost in the port): unpublished block-view audit intent must survive
/// block deletion, so the outbox row's `block_id` is a plain record link with no existence
/// assertion and no cascading reference.
const MT141_BLOCK_VIEW_OUTBOX_BLOCK_LINE: &str = "\
DEFINE FIELD OVERWRITE block_id ON TABLE loom_block_view_fr_outbox TYPE record<loom_blocks>;
";
const MT141_AI_EDIT_APPLIED_BINDING_LINES: &str = "\
DEFINE FIELD OVERWRITE applied_update_id ON TABLE knowledge_crdt_ai_edit_proposals TYPE option<string> ASSERT $value = NONE OR string::trim($value) != '';
DEFINE FIELD OVERWRITE applied_update_sha256 ON TABLE knowledge_crdt_ai_edit_proposals TYPE option<string> ASSERT $value = NONE OR ($value = $this.diff_sha256 AND $this.applied_update_id != NONE);
";
/// The complete MT-141 lineage delta: the provenance-ref `asset_id` line, the saved-search
/// retrieval projection block, and the V2-R2 field definitions.
fn mt141_upgrade_statements() -> String {
    format!(
        "{MT141_PROVENANCE_REF_ASSERT_LINE}{MT141_SAVED_SEARCH_PROJECTION_BLOCK}\
{MT141_LOOM_PIN_ORDER_LINE}{MT141_QUICK_SWITCHER_SOURCE_KIND_LINE}\
{MT141_QUICK_SWITCHER_RESULT_KIND_LINE}{MT141_BLOCK_VIEW_OUTBOX_BLOCK_LINE}\
{MT141_AI_EDIT_APPLIED_BINDING_LINES}"
    )
}
/// The MT-150-era `asset_id` definition, used only to reconstruct the exact MT-150 pin script
/// in tests (`mt141_pin_schema`, `mt150_pin_schema`).
#[cfg(test)]
const PRE_MT141_PROVENANCE_REF_ASSET_ID_LINE: &str = "\
DEFINE FIELD OVERWRITE asset_id ON TABLE atelier_media_source_provenance_ref TYPE record<atelier_media_asset> ASSERT (record::exists($value)) AND (record::id($value) = record::id($this.id)) REFERENCE ON DELETE CASCADE;
";
/// The MT-150-era definitions the V2-R2 lines replace, used only to reconstruct the exact
/// MT-150 pin script in tests (`mt141_pin_schema`).
#[cfg(test)]
const PRE_MT141_LOOM_PIN_ORDER_LINE: &str = "\
DEFINE FIELD OVERWRITE pin_order ON TABLE loom_blocks TYPE option<int> ASSERT $value = NONE OR $value >= 0;
";
#[cfg(test)]
const PRE_MT141_QUICK_SWITCHER_SOURCE_KIND_LINE: &str = "\
DEFINE FIELD OVERWRITE source_kind ON TABLE knowledge_quick_switcher_recents TYPE 'loom_block' | 'symbol' | 'work_packet' | 'micro_task' | 'user_manual_page';
";
#[cfg(test)]
const PRE_MT141_BLOCK_VIEW_OUTBOX_BLOCK_LINE: &str = "\
DEFINE FIELD OVERWRITE block_id ON TABLE loom_block_view_fr_outbox TYPE record<loom_blocks> ASSERT record::exists($value) REFERENCE ON DELETE CASCADE;
";
#[cfg(test)]
const PRE_MT141_QUICK_SWITCHER_RESULT_KIND_LINE: &str = "\
DEFINE FIELD OVERWRITE result_kind ON TABLE knowledge_quick_switcher_recents TYPE 'loom_block' | 'knowledge_entity' | 'user_manual_page';
";
/// MT-150 upgrade statements: the durable EventLedger receipt binding on `loom_edges`, so a
/// tag/mention edge create or delete carries the same atomic receipt linkage as `loom_blocks`
/// and `loom_folders`. Applied with the state update in one transaction on top of every
/// allowlisted predecessor lineage. Both statements must stay identical to `schema.surql`
/// (proven by `mt150_upgrade_statements_match_schema`). No backfill: edges written before the
/// field existed keep `NONE` (the field is optional, exactly like `loom_blocks`).
const MT150_LOOM_EDGE_RECEIPT_UPGRADE_STATEMENTS: &str = "\
DEFINE FIELD OVERWRITE event_ledger_event_id ON TABLE loom_edges TYPE option<record<kernel_event_ledger>> ASSERT $value = NONE OR record::exists($value) REFERENCE ON DELETE REJECT;
DEFINE INDEX OVERWRITE idx_loom_edges_event ON TABLE loom_edges FIELDS event_ledger_event_id;
";
/// MT-152 upgrade statements, applied with the state update in one transaction on top of every
/// allowlisted predecessor lineage. Every DDL statement must stay identical to `schema.surql`
/// (proven by `mt152_upgrade_statements_match_schema`). No backfill: the anchor rows are
/// created lazily by the first FEMS write or workspace delete per workspace.
const MT152_FEMS_WRITE_ANCHOR_UPGRADE_STATEMENTS: &str = "\
DEFINE TABLE OVERWRITE fems_workspace_write_anchors SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE anchor_key ON TABLE fems_workspace_write_anchors TYPE string ASSERT $value = record::id($this.id);
DEFINE FIELD OVERWRITE workspace_key ON TABLE fems_workspace_write_anchors TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE claim_nonce ON TABLE fems_workspace_write_anchors TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE updated_at ON TABLE fems_workspace_write_anchors TYPE datetime DEFAULT time::now();
DEFINE INDEX OVERWRITE pk_fems_workspace_write_anchors ON TABLE fems_workspace_write_anchors FIELDS anchor_key UNIQUE;
";
/// MT-152 (I-152-2 sweep finding): the stored `loom_folders.sibling_key` discriminator.
/// `uq_loom_folders_sibling_name` never rejected a duplicate ROOT name because the pinned
/// engine skips uniqueness for any tuple containing NONE
/// (`surrealdb-core-3.2.0/src/idx/index.rs:190-197`); MT-151's audit recorded that index as
/// covering the invariant. Phase one (own transaction, MT-151 `journal_key` pattern): define the
/// field, then `backfill_mt152_folder_sibling_keys` writes every existing row's key and
/// disambiguates pre-existing root duplicates with a stable `#dup<n>` suffix so an operator
/// store upgrades instead of failing at the index build; phase two builds the UNIQUE index in
/// the DDL transaction. Both statements must stay identical to `schema.surql` (proven by
/// `mt152_folder_sibling_key_statements_match_schema`).
const MT152_LOOM_FOLDER_SIBLING_KEY_FIELD_STATEMENTS: &str = "\
DEFINE FIELD OVERWRITE sibling_key ON TABLE loom_folders TYPE string ASSERT string::trim($value) != '';
";
const MT152_LOOM_FOLDER_SIBLING_KEY_INDEX_STATEMENTS: &str = "\
DEFINE INDEX OVERWRITE uq_loom_folders_sibling_key ON TABLE loom_folders FIELDS sibling_key UNIQUE;
";
/// First MT-151 upgrade phase, committed in its OWN transaction before the DDL transaction:
/// defines the computed `journal_key` and rewrites every existing journal block so the key is
/// materialised and COMMITTED before `uq_loom_blocks_journal_key` is built. The pinned engine
/// builds every `DEFINE INDEX` through its `IndexBuilder` in separate transactions
/// (`surrealdb-core-3.2.0/src/expr/statements/define/index.rs:235`,
/// `kvs/index/builder.rs:864-887`), so a backfill inside the same transaction as the index is
/// invisible to the build and pre-existing journal rows would stay unprotected (observed run
/// `mt142-LIB-20260911T125838Z`). Idempotent: re-running it after a crash is harmless.
const MT151_JOURNAL_KEY_MATERIALISE_STATEMENTS: &str = "\
DEFINE FIELD OVERWRITE journal_key ON TABLE loom_blocks TYPE option<string>
    VALUE IF $this.content_type = 'journal' AND $this.journal_date != NONE {
        type::string($this.workspace_id) + '|' + $this.journal_date
    } ELSE {
        NONE
    };
UPDATE loom_blocks SET updated_at = updated_at WHERE content_type = 'journal' AND journal_date != NONE RETURN NONE;
";
/// Second MT-151 upgrade phase, applied with the state update in one transaction on top of
/// every allowlisted predecessor lineage. Every DDL statement here and in
/// [`MT151_JOURNAL_KEY_MATERIALISE_STATEMENTS`] must stay identical to `schema.surql` (proven
/// by `mt151_upgrade_statements_match_schema`). A store that already holds two journal blocks
/// for one (workspace, date) fails closed at the index build rather than keeping an invariant
/// the index cannot honour.
const MT151_JOURNAL_KEY_AND_GRAPH_ANCHOR_UPGRADE_STATEMENTS: &str = "\
DEFINE INDEX OVERWRITE uq_loom_blocks_journal_key ON TABLE loom_blocks FIELDS journal_key UNIQUE;
DEFINE TABLE OVERWRITE storage_graph_anchors SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE anchor_key ON TABLE storage_graph_anchors TYPE string ASSERT $value = record::id($this.id);
DEFINE FIELD OVERWRITE graph_kind ON TABLE storage_graph_anchors TYPE 'loom_folder_tree' | 'work_packet_dependencies';
DEFINE FIELD OVERWRITE scope_key ON TABLE storage_graph_anchors TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE version ON TABLE storage_graph_anchors TYPE int ASSERT $value >= 1;
DEFINE FIELD OVERWRITE updated_at ON TABLE storage_graph_anchors TYPE datetime DEFAULT time::now();
DEFINE INDEX OVERWRITE pk_storage_graph_anchors ON TABLE storage_graph_anchors FIELDS anchor_key UNIQUE;
";
/// DDL and registry row MT-142 adds on top of both allowlisted predecessor lineages. Every DDL
/// line must stay byte-identical to the `knowledge_rich_document_title_anchors` block in
/// `schema.surql` (proven by `mt142_title_anchor_upgrade_statements_match_schema`).
const MT142_TITLE_ANCHOR_UPGRADE_STATEMENTS: &str = "\
DEFINE TABLE OVERWRITE knowledge_rich_document_title_anchors SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE anchor_key ON TABLE knowledge_rich_document_title_anchors TYPE string ASSERT $value = record::id($this.id);
DEFINE FIELD OVERWRITE workspace_id ON TABLE knowledge_rich_document_title_anchors TYPE record<workspaces> ASSERT record::exists($value) REFERENCE ON DELETE CASCADE;
DEFINE FIELD OVERWRITE title_key ON TABLE knowledge_rich_document_title_anchors TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE last_rich_document_id ON TABLE knowledge_rich_document_title_anchors TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE claim_nonce ON TABLE knowledge_rich_document_title_anchors TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE created_at ON TABLE knowledge_rich_document_title_anchors TYPE datetime DEFAULT time::now();
DEFINE FIELD OVERWRITE updated_at ON TABLE knowledge_rich_document_title_anchors TYPE datetime DEFAULT time::now();
DEFINE INDEX OVERWRITE pk_knowledge_rich_document_title_anchors ON TABLE knowledge_rich_document_title_anchors FIELDS anchor_key UNIQUE;
DEFINE INDEX OVERWRITE uq_knowledge_rich_document_title_anchors_identity ON TABLE knowledge_rich_document_title_anchors FIELDS workspace_id, title_key UNIQUE;
CREATE ONLY knowledge_schema_registry:rich_document_title_anchors CONTENT {
    family_key: 'rich_document_title_anchors',
    table_name: 'knowledge_rich_document_title_anchors',
    record_family: 'Support',
    authority_class: 'support',
    schema_source: $schema_source,
    wp_id: 'WP-KERNEL-012',
    mt_id: 'MT-142'
};
";

const SCHEMA: &str = include_str!("schema.surql");
#[cfg(test)]
const PRE_MT109_SCHEMA: &str = include_str!("schema_pre_mt109.surql");
const PRE_MT109_GENERATED_SURREALQL_SHA256: &str =
    "46eac57c4ac3e39acc9d18ac0a43fc62ec01461e8cf3b70b7e2711de2a59da10";
const PRE_MT109_AUTHORITY_INFO_SHA256: &str =
    "8803fbcc07ae64c671d6ecdc206f5a48efdf4d536444ff011d0db66d023672ca";
const PRE_MT109_SCHEMA_INFO_SHA256: &str =
    "bb515db9b0f18c4bc8b7c26cf0773cb9dbe5bb4343ba02bc9ea32218cfddd39d";
#[cfg(test)]
const PRE_MT109_AUTHORITY_SCHEMA: &str = include_str!("schema_pre_mt109_authority.surql");
const KNOWLEDGE_SCHEMA_REGISTRY_SEED: &str = include_str!("knowledge_schema_registry_seed.surql");
const DECLARATIVE_SCHEMA_CATALOG_DOMAIN: &[u8] =
    b"handshake.surreal.declarative-schema-catalog.v1\0";
const PREDECESSOR_KNOWLEDGE_REGISTRY_DOMAIN: &[u8] =
    b"handshake.surreal.predecessor-knowledge-schema-registry.v1\0";
/// Byte-exact SurrealQL projection of the 61 registry tuples independently extracted from the
/// deleted Git migration objects. It is compatibility data only; no deleted file is opened or
/// executed. The current-only 0343 registry row is intentionally absent and added by upgrade.
const PREDECESSOR_KNOWLEDGE_SCHEMA_REGISTRY_SEED: &str = r#"
BEGIN TRANSACTION;
FOR $registry IN [
    ['claim_conflicts', 'knowledge_claim_conflicts', 'KnowledgeClaim', 'authority', '0137_knowledge_claims.sql', 'WP-KERNEL-009', 'MT-056'],
    ['claim_spans', 'knowledge_claim_spans', 'KnowledgeClaim', 'authority', '0137_knowledge_claims.sql', 'WP-KERNEL-009', 'MT-056'],
    ['claims', 'knowledge_claims', 'KnowledgeClaim', 'authority', '0137_knowledge_claims.sql', 'WP-KERNEL-009', 'MT-056'],
    ['code_files', 'knowledge_code_files', 'KnowledgeSource', 'support', '0170_knowledge_code_files.sql', 'WP-KERNEL-009', 'MT-107'],
    ['code_repair_queue', 'knowledge_code_repair_queue', 'KnowledgeSource', 'support', '0230_knowledge_code_repair_queue.sql', 'WP-KERNEL-009', 'MT-108'],
    ['code_scip_imports', 'knowledge_code_scip_imports', 'KnowledgeEdge', 'support', '0171_knowledge_code_scip_imports.sql', 'WP-KERNEL-009', 'MT-105'],
    ['context_bundle_items', 'knowledge_context_bundle_items', 'Support', 'authority', '0141_knowledge_context_bundles.sql', 'WP-KERNEL-009', 'MT-060'],
    ['context_bundles', 'knowledge_context_bundles', 'Support', 'authority', '0141_knowledge_context_bundles.sql', 'WP-KERNEL-009', 'MT-060'],
    ['crdt_agent_lane_leases', 'knowledge_crdt_agent_lane_leases', 'AgentLaneLease', 'support', '0151_knowledge_crdt_agent_lane_leases.sql', 'WP-KERNEL-009', 'MT-076'],
    ['crdt_ai_edit_proposals', 'knowledge_crdt_ai_edit_proposals', 'AiEditProposal', 'support', '0154_knowledge_crdt_ai_edit_proposals.sql', 'WP-KERNEL-009', 'MT-074'],
    ['crdt_denial_receipts', 'knowledge_crdt_denial_receipts', 'CrdtDenialReceipt', 'support', '0150_knowledge_crdt_denial_receipts.sql', 'WP-KERNEL-009', 'MT-070'],
    ['crdt_graph_proposals', 'knowledge_crdt_graph_proposals', 'GraphMutationProposal', 'support', '0152_knowledge_crdt_graph_proposals.sql', 'WP-KERNEL-009', 'MT-068'],
    ['crdt_promoted_facts', 'knowledge_crdt_promoted_facts', 'KnowledgeClaim', 'authority', '0153_knowledge_crdt_promoted_facts.sql', 'WP-KERNEL-009', 'MT-069'],
    ['crdt_recovery_receipts', 'knowledge_crdt_recovery_receipts', 'CrdtRecoveryReceipt', 'support', '0155_knowledge_crdt_swarm_checkpoints.sql', 'WP-KERNEL-009', 'MT-079'],
    ['crdt_swarm_checkpoints', 'knowledge_crdt_swarm_checkpoints', 'SwarmCheckpoint', 'support', '0155_knowledge_crdt_swarm_checkpoints.sql', 'WP-KERNEL-009', 'MT-079'],
    ['debug_breakpoints', 'knowledge_debug_breakpoints', 'DebugBreakpoints', 'support', '0331_debug_breakpoints.sql', 'WP-KERNEL-009', 'MT-254'],
    ['document_backlinks', 'knowledge_document_backlinks', 'KnowledgeEdge', 'authority', '0282_knowledge_document_backlinks.sql', 'WP-KERNEL-009', 'MT-155'],
    ['document_embeds', 'knowledge_document_embeds', 'RichDocument', 'authority', '0281_knowledge_document_embeds.sql', 'WP-KERNEL-009', 'MT-152'],
    ['edge_spans', 'knowledge_edge_spans', 'KnowledgeEdge', 'authority', '0136_knowledge_edges.sql', 'WP-KERNEL-009', 'MT-054'],
    ['edges', 'knowledge_edges', 'KnowledgeEdge', 'authority', '0136_knowledge_edges.sql', 'WP-KERNEL-009', 'MT-054'],
    ['editor_code_nodes', 'knowledge_editor_code_nodes', 'EditorCodeNode', 'authority', '0140_knowledge_rich_documents.sql', 'WP-KERNEL-009', 'MT-059'],
    ['entities', 'knowledge_entities', 'KnowledgeEntity', 'authority', '0135_knowledge_entities.sql', 'WP-KERNEL-009', 'MT-053'],
    ['entity_spans', 'knowledge_entity_spans', 'KnowledgeEntity', 'authority', '0135_knowledge_entities.sql', 'WP-KERNEL-009', 'MT-053'],
    ['idempotency_keys', 'knowledge_idempotency_keys', 'Support', 'support', '0142_knowledge_idempotency_keys.sql', 'WP-KERNEL-009', 'MT-062'],
    ['index_runs', 'knowledge_index_runs', 'Support', 'authority', '0133_knowledge_index_runs.sql', 'WP-KERNEL-009', 'MT-052'],
    ['ingestion_kind_registry', 'knowledge_ingestion_kind_registry', 'KnowledgeSource', 'projection', '0161_knowledge_ingestion_kind_registry.sql', 'WP-KERNEL-009', 'MT-082'],
    ['ingestion_policy_decisions', 'knowledge_ingestion_policy_decisions', 'KnowledgeSource', 'support', '0160_knowledge_ingestion_policies.sql', 'WP-KERNEL-009', 'MT-081'],
    ['ingestion_receipts', 'knowledge_ingestion_receipts', 'KnowledgeSource', 'authority', '0162_knowledge_ingestion_receipts.sql', 'WP-KERNEL-009', 'MT-085'],
    ['ingestion_repair_queue', 'knowledge_ingestion_repair_queue', 'KnowledgeSource', 'authority', '0164_knowledge_ingestion_repair_queue.sql', 'WP-KERNEL-009', 'MT-094'],
    ['ingestion_root_policies', 'knowledge_ingestion_root_policies', 'KnowledgeSource', 'authority', '0160_knowledge_ingestion_policies.sql', 'WP-KERNEL-009', 'MT-081'],
    ['ingestion_spans', 'knowledge_ingestion_spans', 'KnowledgeSpan', 'authority', '0163_knowledge_ingestion_spans.sql', 'WP-KERNEL-009', 'MT-087'],
    ['memory_bridge_decisions', 'knowledge_memory_bridge_decisions', 'BridgeEdgeJob', 'authority', '0243_knowledge_memory_bridge_edges.sql', 'WP-KERNEL-009', 'MT-124'],
    ['memory_conflict_detection_findings', 'knowledge_memory_conflict_detection_findings', 'ConflictDetectionJob', 'authority', '0242_knowledge_memory_conflict_jobs.sql', 'WP-KERNEL-009', 'MT-122'],
    ['memory_conflict_detection_jobs', 'knowledge_memory_conflict_detection_jobs', 'ConflictDetectionJob', 'authority', '0242_knowledge_memory_conflict_jobs.sql', 'WP-KERNEL-009', 'MT-122'],
    ['memory_conflict_resolution_jobs', 'knowledge_memory_conflict_resolution_jobs', 'ConflictResolutionJob', 'authority', '0242_knowledge_memory_conflict_jobs.sql', 'WP-KERNEL-009', 'MT-123'],
    ['memory_facts', 'knowledge_memory_facts', 'MemoryFact', 'authority', '0241_knowledge_memory_facts.sql', 'WP-KERNEL-009', 'MT-114'],
    ['memory_ontology_aliases', 'knowledge_memory_ontology_aliases', 'MemoryOntology', 'authority', '0240_knowledge_memory_ontology.sql', 'WP-KERNEL-009', 'MT-113'],
    ['memory_ontology_terms', 'knowledge_memory_ontology_terms', 'MemoryOntology', 'authority', '0240_knowledge_memory_ontology.sql', 'WP-KERNEL-009', 'MT-113'],
    ['memory_passages', 'knowledge_memory_passages', 'MemoryPassage', 'authority', '0138_knowledge_memory_passages.sql', 'WP-KERNEL-009', 'MT-057'],
    ['parallel_indexing_lease_queue', 'knowledge_parallel_indexing_lease_queue', 'IndexingLease', 'support', '0311_parallel_swarm_state_recovery.sql', 'WP-KERNEL-009', 'MT-216'],
    ['parallel_swarm_checkpoints', 'knowledge_agent_state_recovery_checkpoints', 'SwarmCheckpoint', 'support', '0311_parallel_swarm_state_recovery.sql', 'WP-KERNEL-009', 'MT-213'],
    ['parallel_swarm_claims', 'knowledge_agent_worktree_claims', 'SwarmClaim', 'support', '0311_parallel_swarm_state_recovery.sql', 'WP-KERNEL-009', 'MT-210'],
    ['parallel_swarm_cloud_assistance_receipts', 'knowledge_agent_cloud_assistance_receipts', 'SwarmCloudAssistanceReceipt', 'support', '0314_parallel_swarm_cloud_assistance_receipts.sql', 'WP-KERNEL-009', 'MT-221'],
    ['parallel_swarm_handoffs', 'knowledge_agent_role_mailbox_handoffs', 'SwarmHandoff', 'support', '0311_parallel_swarm_state_recovery.sql', 'WP-KERNEL-009', 'MT-211'],
    ['parallel_swarm_quiet_background_work', 'knowledge_agent_quiet_background_work', 'SwarmQuietBackgroundWork', 'support', '0313_parallel_swarm_quiet_background_work.sql', 'WP-KERNEL-009', 'MT-219'],
    ['parallel_swarm_recovery_receipts', 'knowledge_agent_recovery_receipts', 'SwarmRecoveryReceipt', 'support', '0311_parallel_swarm_state_recovery.sql', 'WP-KERNEL-009', 'MT-214'],
    ['passage_evidence', 'knowledge_passage_evidence', 'MemoryPassage', 'authority', '0138_knowledge_memory_passages.sql', 'WP-KERNEL-009', 'MT-057'],
    ['quick_switcher_recents', 'knowledge_quick_switcher_recents', 'QuickSwitcherRecent', 'support', '0322_quick_switcher_recents.sql', 'WP-KERNEL-009', 'MT-256'],
    ['retrieval_traces', 'knowledge_retrieval_traces', 'RetrievalTrace', 'authority', '0141_knowledge_context_bundles.sql', 'WP-KERNEL-009', 'MT-060'],
    ['rich_document_drafts', 'knowledge_rich_document_drafts', 'RichDocumentDraftRecovery', 'support', '0328_rich_document_draft_recovery.sql', 'WP-KERNEL-009', 'MT-255'],
    ['rich_document_versions', 'knowledge_rich_document_versions', 'RichDocument', 'authority', '0140_knowledge_rich_documents.sql', 'WP-KERNEL-009', 'MT-059'],
    ['rich_documents', 'knowledge_rich_documents', 'RichDocument', 'authority', '0140_knowledge_rich_documents.sql', 'WP-KERNEL-009', 'MT-059'],
    ['schema_registry', 'knowledge_schema_registry', 'Support', 'support', '0130_knowledge_schema_namespace.sql', 'WP-KERNEL-009', 'MT-049'],
    ['semantic_catalog_entries', 'knowledge_semantic_catalog_entries', 'Support', 'authority', '0260_knowledge_semantic_catalog.sql', 'WP-KERNEL-009', 'MT-140'],
    ['source_roots', 'knowledge_source_roots', 'KnowledgeSource', 'authority', '0131_knowledge_source_roots.sql', 'WP-KERNEL-009', 'MT-050'],
    ['sources', 'knowledge_sources', 'KnowledgeSource', 'authority', '0132_knowledge_sources.sql', 'WP-KERNEL-009', 'MT-051'],
    ['spans', 'knowledge_spans', 'KnowledgeSpan', 'authority', '0134_knowledge_spans.sql', 'WP-KERNEL-009', 'MT-055'],
    ['wiki_projections', 'knowledge_wiki_projections', 'Projection', 'projection', '0139_knowledge_wiki_projections.sql', 'WP-KERNEL-009', 'MT-058'],
    ['workbench_layout_state', 'knowledge_workbench_layout_states', 'WorkbenchLayoutState', 'support', '0323_workbench_layout_state.sql', 'WP-KERNEL-009', 'MT-246'],
    ['workspace_search_bookmark_state', 'knowledge_workspace_search_bookmark_states', 'WorkspaceSearchBookmarkState', 'support', '0330_workspace_search_bookmark_state.sql', 'WP-KERNEL-009', 'MT-258'],
    ['workspace_settings_state', 'knowledge_workspace_settings_states', 'WorkspaceSettingsState', 'support', '0327_workspace_settings_state.sql', 'WP-KERNEL-009', 'MT-248'],
] {
    CREATE type::record('knowledge_schema_registry', $registry[0]) CONTENT {
        family_key: $registry[0], table_name: $registry[1], record_family: $registry[2],
        authority_class: $registry[3], migration_file: $registry[4],
        wp_id: $registry[5], mt_id: $registry[6]
    };
};
COMMIT TRANSACTION;
"#;
const BOOTSTRAP_STATE_TABLE: &str = "handshake_schema_state";
const BOOTSTRAP_STATE_ID: &str = "handshake_schema_state:primary";
const ATELIER_CATALOG_INFO_CONCURRENCY: usize = 8;
const ATELIER_REQUIRED_SEQUENCES: [&str; 2] =
    ["atelier_pose_context_state_seq", "kernel_event_sequence"];
const DATABASE_STRUCTURE_CATEGORIES: [&str; 12] = [
    "accesses",
    "analyzers",
    "apis",
    "buckets",
    "configs",
    "functions",
    "models",
    "modules",
    "params",
    "sequences",
    "tables",
    "users",
];
// MT-142 re-pin: +1 table (knowledge_rich_document_title_anchors), +7 fields,
// +2 indexes (pk + uq), +1 REFERENCE field, +1 record-id alias assertion.
// MT-151 re-pin: +1 table (storage_graph_anchors: +5 fields, +1 pk index, +1 record-id
// alias assertion) and loom_blocks.journal_key (+1 field, +1 uq index); no REFERENCE field.
// MT-152 re-pin: +1 table (fems_workspace_write_anchors: +4 fields, +1 pk index, +1 record-id
// alias assertion); no REFERENCE field.
// MT-150 re-pin: loom_edges.event_ledger_event_id (+1 field, +1 REFERENCE field, +1 explicit
// record::exists assertion) and idx_loom_edges_event (+1 named index); no new table.
const TABLE_DEFINITION_COUNT: usize = 294;
// MT-141 V2-R2 re-pin: +2 fields (knowledge_crdt_ai_edit_proposals.applied_update_id /
// applied_update_sha256); pin_order and the quick-switcher kind unions changed in place.
// MT-158 re-pin: +2 fields (mt_iterations.owner_account_id, dependencies.owner_account_id; D-154-3
// extension).
const SOURCE_FIELD_DEFINITION_COUNT: usize = 3358;
const FLEXIBLE_WILDCARD_FIELD_DEFINITION_COUNT: usize = 239;
const FLEXIBLE_FIELD_DEFINITION_COUNT: usize = 175;
const INTENTIONAL_UNION_ANY_FIELD_DEFINITIONS: [&str; 2] = [
    "DEFINE FIELD OVERWRITE capability_grants ON TABLE atelier_transcript_receipt TYPE any DEFAULT [];",
    "DEFINE FIELD OVERWRITE decisions ON TABLE knowledge_retrieval_traces TYPE any DEFAULT [];",
];
const AUTHORED_FIELD_DEFINITION_COUNT: usize =
    SOURCE_FIELD_DEFINITION_COUNT + FLEXIBLE_WILDCARD_FIELD_DEFINITION_COUNT;
// SurrealDB 3.2 persists one `field.*` subtype definition per non-Any typed collection nesting
// level. Structured INFO reads the full persisted field catalog, so these engine-generated
// definitions are part of the exact live schema even though they are not authored DEFINE lines.
const ENGINE_GENERATED_COLLECTION_SUBTYPE_FIELD_COUNT: usize = 55;
const FIELD_DEFINITION_COUNT: usize =
    AUTHORED_FIELD_DEFINITION_COUNT + ENGINE_GENERATED_COLLECTION_SUBTYPE_FIELD_COUNT;
const INDEX_DEFINITION_COUNT: usize = 816;
const EVENT_DEFINITION_COUNT: usize = 42;
const VIEW_DEFINITION_COUNT: usize = 2;
const SEQUENCE_DEFINITION_COUNT: usize = 2;
const ACCESS_DEFINITION_COUNT: usize = 1;
// MT-158 re-pin: +1 function (fn::mt158_dependency_graph_scope).
// MT-159: +2 functions (fn::mt159_locus_key_id, fn::mt159_locus_key_is_own).
const FUNCTION_DEFINITION_COUNT: usize = 55;
const SOURCE_TABLE_COUNT: usize = 291;
const SOURCE_VIEW_COUNT: usize = 2;
const SOURCE_NAMED_INDEX_COUNT: usize = 555;
const SURREAL_PRIMARY_KEY_INDEX_COUNT: usize = 260;
const SURREAL_BOOTSTRAP_STATE_TABLE_COUNT: usize = 1;
const SURREAL_BOOTSTRAP_STATE_INDEX_COUNT: usize = 1;
// MT-141 V2-R2: loom_block_view_fr_outbox.block_id lost its cascading REFERENCE (MT-027 0362).
// 407: DDL (non-comment) REFERENCE clauses in schema.surql at 0cfbff64 and after; the pinned 406
// predated one added REFERENCE field (kb-c5 static count, 2026-09-24).
const REFERENCE_FIELD_COUNT: usize = 407;
const EXPLICIT_REFERENCE_EXISTENCE_ASSERTION_COUNT: usize = 402;
const RECORD_ID_ALIAS_ASSERTION_COUNT: usize = 229;

static BOOTSTRAP_MUTEX: Mutex<()> = Mutex::const_new(());

/// Applies the canonical Atelier schema projection, together with the shared
/// EventLedger table and sequence that every Atelier mutation writes.
///
/// The projection is selected mechanically from the same compiled
/// `schema.surql` consumed by [`bootstrap_schema`]. It is a bounded production
/// bootstrap component, not a hand-maintained test schema. The shared bootstrap
/// mutex covers inspection and mutation, and the DDL is one transaction, so a
/// concurrent caller or failed statement cannot strand a partial projection.
/// Returns `true` only when this call installed the projection.
pub async fn bootstrap_atelier_schema(
    storage: &SurrealStorage,
) -> Result<bool, SurrealStorageError> {
    let _bootstrap_guard = BOOTSTRAP_MUTEX.lock().await;
    let ddl = atelier_schema_ddl();
    let expected = atelier_expected_catalog();
    storage
        .with_admin_operation(move |database| {
            Box::pin(async move {
                let present = atelier_table_definitions(&database).await?;
                let expected_atelier_tables = expected
                    .keys()
                    .filter(|table| table.starts_with("atelier_"))
                    .cloned()
                    .collect::<BTreeSet<_>>();
                let present_atelier_tables = present
                    .keys()
                    .filter(|table| table.starts_with("atelier_"))
                    .cloned()
                    .collect::<BTreeSet<_>>();

                if present_atelier_tables.is_empty() {
                    database
                        .query(format!(
                            "BEGIN TRANSACTION;\n{ddl}\nCOMMIT TRANSACTION;\n"
                        ))
                        .await?;
                    verify_atelier_catalog(&database, &expected).await?;
                    return Ok(true);
                }

                if present_atelier_tables != expected_atelier_tables {
                    return fail_closed(
                        &database,
                        format!(
                            "HANDSHAKE_ATELIER_SCHEMA_PARTIAL: expected={} present={} first_missing={} first_unexpected={}",
                            expected_atelier_tables.len(),
                            present_atelier_tables.len(),
                            expected_atelier_tables
                                .difference(&present_atelier_tables)
                                .next()
                                .map(String::as_str)
                                .unwrap_or("none"),
                            present_atelier_tables
                                .difference(&expected_atelier_tables)
                                .next()
                                .map(String::as_str)
                                .unwrap_or("none")
                        ),
                    )
                    .await;
                }

                verify_atelier_catalog(&database, &expected).await?;
                Ok(false)
            })
        })
        .await
}

#[derive(Debug, Default, PartialEq, Eq)]
struct AtelierTableDefinition {
    schemafull: bool,
    kind: String,
    is_view: bool,
}

#[derive(Debug, Default)]
struct ExpectedAtelierTable {
    definition: AtelierTableDefinition,
    fields: BTreeSet<String>,
    indexes: BTreeSet<String>,
    events: BTreeSet<String>,
}

fn atelier_expected_catalog() -> BTreeMap<String, ExpectedAtelierTable> {
    let mut catalog: BTreeMap<String, ExpectedAtelierTable> = BTreeMap::new();
    for line in atelier_schema_ddl().lines().map(str::trim_start) {
        if let Some(rest) = line.strip_prefix("DEFINE TABLE OVERWRITE ") {
            let table = rest.split_ascii_whitespace().next().unwrap_or_default();
            if !table.is_empty() {
                let expected = catalog.entry(table.to_owned()).or_default();
                expected.definition = AtelierTableDefinition {
                    schemafull: line.contains(" SCHEMAFULL"),
                    kind: if line.contains(" TYPE NORMAL") {
                        "NORMAL"
                    } else if line.contains(" TYPE RELATION") {
                        "RELATION"
                    } else if line.contains(" TYPE ANY") {
                        "ANY"
                    } else {
                        "NORMAL"
                    }
                    .to_owned(),
                    is_view: line.ends_with(" AS") || line.contains(" AS "),
                };
            }
            continue;
        }

        let (kind, rest) = if let Some(rest) = line.strip_prefix("DEFINE FIELD OVERWRITE ") {
            ("field", rest)
        } else if let Some(rest) = line.strip_prefix("DEFINE INDEX OVERWRITE ") {
            ("index", rest)
        } else if let Some(rest) = line.strip_prefix("DEFINE EVENT OVERWRITE ") {
            ("event", rest)
        } else {
            continue;
        };
        let Some((name, table_tail)) = rest.split_once(" ON TABLE ") else {
            continue;
        };
        let table = table_tail
            .split_ascii_whitespace()
            .next()
            .unwrap_or_default()
            .trim_matches('`');
        let expected = catalog.entry(table.to_owned()).or_default();
        match kind {
            "field" => {
                expected.fields.insert(name.to_owned());
            }
            "index" => {
                expected.indexes.insert(name.to_owned());
            }
            "event" => {
                expected.events.insert(name.to_owned());
            }
            _ => unreachable!(),
        }
    }
    catalog
}

async fn atelier_table_definitions(
    database: &SurrealAdminContext<'_>,
) -> Result<BTreeMap<String, AtelierTableDefinition>, SurrealStorageError> {
    let mut response = database.query("INFO FOR DB STRUCTURE;").await?;
    let info: SurrealValueData = response.take(0)?;
    match parse_table_definitions(&info) {
        Ok(definitions) => Ok(definitions),
        Err(reason) => fail_closed(database, reason).await,
    }
}

async fn verify_atelier_catalog(
    database: &SurrealAdminContext<'_>,
    expected: &BTreeMap<String, ExpectedAtelierTable>,
) -> Result<(), SurrealStorageError> {
    let expected_tables = expected.keys().cloned().collect::<BTreeSet<_>>();
    let expected_sequences = ATELIER_REQUIRED_SEQUENCES
        .iter()
        .map(|name| (*name).to_owned())
        .collect::<BTreeSet<_>>();
    verify_atelier_catalog_fingerprint(
        database,
        &expected_tables,
        &expected_sequences,
        EXPECTED_ATELIER_CATALOG_SHA256,
    )
    .await
}

#[derive(Serialize)]
struct AtelierCatalogInfoEnvelope {
    table_definitions: BTreeMap<String, SurrealValueData>,
    sequence_definitions: BTreeMap<String, SurrealValueData>,
    table_members: BTreeMap<String, SurrealValueData>,
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    access_definitions: BTreeMap<String, SurrealValueData>,
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    function_definitions: BTreeMap<String, SurrealValueData>,
}

async fn verify_atelier_catalog_fingerprint(
    database: &SurrealAdminContext<'_>,
    expected_tables: &BTreeSet<String>,
    expected_sequences: &BTreeSet<String>,
    expected_fingerprint: &str,
) -> Result<(), SurrealStorageError> {
    if expected_fingerprint.bytes().all(|byte| byte == b'0') {
        return fail_closed(
            database,
            "HANDSHAKE_ATELIER_SCHEMA_CATALOG_FINGERPRINT_UNPINNED".to_owned(),
        )
        .await;
    }
    let observed =
        inspect_atelier_catalog_fingerprint(database, expected_tables, expected_sequences).await?;
    if observed != expected_fingerprint {
        return fail_closed(
            database,
            format!(
                "HANDSHAKE_ATELIER_SCHEMA_CATALOG_FINGERPRINT_MISMATCH: expected={expected_fingerprint}; observed={observed}"
            ),
        )
        .await;
    }
    Ok(())
}

async fn inspect_atelier_catalog_fingerprint(
    database: &SurrealAdminContext<'_>,
    expected_tables: &BTreeSet<String>,
    expected_sequences: &BTreeSet<String>,
) -> Result<String, SurrealStorageError> {
    inspect_catalog_fingerprint(
        database,
        expected_tables,
        expected_sequences,
        CatalogInspectionScope::Atelier,
    )
    .await
}

#[derive(Clone, Copy)]
enum CatalogInspectionScope {
    Atelier,
    ExactDatabase,
}

async fn inspect_catalog_fingerprint(
    database: &SurrealAdminContext<'_>,
    expected_tables: &BTreeSet<String>,
    expected_sequences: &BTreeSet<String>,
    scope: CatalogInspectionScope,
) -> Result<String, SurrealStorageError> {
    let mut response = database.query("INFO FOR DB STRUCTURE;").await?;
    let database_info: SurrealValueData = response.take(0)?;
    let table_definitions = match parse_named_structures(&database_info, "tables") {
        Ok(definitions) => definitions,
        Err(reason) => return fail_closed(database, reason).await,
    };
    let sequence_definitions = match parse_named_structures(&database_info, "sequences") {
        Ok(definitions) => definitions,
        Err(reason) => return fail_closed(database, reason).await,
    };

    let authority_dependencies = expected_tables.contains("local_accounts");
    let relevant_tables = table_definitions
        .keys()
        .filter(|name| match scope {
            CatalogInspectionScope::Atelier => {
                name.starts_with("atelier_")
                    || *name == "kernel_event_ledger"
                    || (authority_dependencies && expected_tables.contains(*name))
            }
            CatalogInspectionScope::ExactDatabase => true,
        })
        .cloned()
        .collect::<BTreeSet<_>>();
    if &relevant_tables != expected_tables {
        return fail_closed(
            database,
            format!(
                "HANDSHAKE_ATELIER_SCHEMA_TABLE_SET_MISMATCH: expected={expected_tables:?} actual={relevant_tables:?}"
            ),
        )
        .await;
    }

    let relevant_sequences = sequence_definitions
        .keys()
        .filter(|name| match scope {
            CatalogInspectionScope::Atelier => {
                name.starts_with("atelier_") || *name == "kernel_event_sequence"
            }
            CatalogInspectionScope::ExactDatabase => true,
        })
        .cloned()
        .collect::<BTreeSet<_>>();
    if &relevant_sequences != expected_sequences {
        return fail_closed(
            database,
            format!(
                "HANDSHAKE_ATELIER_SCHEMA_SEQUENCE_SET_MISMATCH: expected={expected_sequences:?} actual={relevant_sequences:?}"
            ),
        )
        .await;
    }

    let table_members = stream::iter(expected_tables.iter().cloned().map(|table| async move {
        let mut response = database
            .query(format!("INFO FOR TABLE `{table}` STRUCTURE;"))
            .await?;
        let info: SurrealValueData = response.take(0)?;
        Ok::<_, SurrealStorageError>((table, canonicalize_info(info)))
    }))
    .buffer_unordered(ATELIER_CATALOG_INFO_CONCURRENCY)
    .try_collect::<Vec<_>>()
    .await?
    .into_iter()
    .collect::<BTreeMap<_, _>>();

    let mut access_definitions = BTreeMap::new();
    let mut function_definitions = BTreeMap::new();
    if authority_dependencies {
        for (category, expected_names, selected) in [
            (
                "accesses",
                authority_catalog_names("access"),
                &mut access_definitions,
            ),
            (
                "functions",
                {
                    // The exact bounded Loom catalog also carries the table-called functions
                    // (LOOM_RECEIPT_TEST_TABLE_FUNCTIONS) its DDL defines beside the authority core.
                    let mut names = authority_catalog_names("function");
                    if matches!(scope, CatalogInspectionScope::ExactDatabase) {
                        names.extend(
                            LOOM_RECEIPT_TEST_TABLE_FUNCTIONS
                                .iter()
                                .map(|name| (*name).to_owned()),
                        );
                    }
                    names
                },
                &mut function_definitions,
            ),
        ] {
            let actual = match parse_named_structures(&database_info, category) {
                Ok(definitions) => definitions,
                Err(reason) => return fail_closed(database, reason).await,
            };
            if matches!(scope, CatalogInspectionScope::ExactDatabase)
                && actual.keys().cloned().collect::<BTreeSet<_>>() != expected_names
            {
                return fail_closed(
                    database,
                    format!("HANDSHAKE_LOOM_SCHEMA_AUTHORITY_SET_MISMATCH: {category}"),
                )
                .await;
            }
            for name in expected_names {
                let Some(definition) = actual.get(&name) else {
                    return fail_closed(database, format!("HANDSHAKE_ATELIER_SCHEMA_AUTHORITY_DEPENDENCY_MISSING: {category}/{name}")).await;
                };
                selected.insert(name, canonicalize_info(definition.clone()));
            }
        }
    }
    let envelope = AtelierCatalogInfoEnvelope {
        access_definitions,
        function_definitions,
        table_definitions: expected_tables
            .iter()
            .filter_map(|name| {
                table_definitions
                    .get(name)
                    .cloned()
                    .map(strip_table_catalog_id)
                    .map(|definition| (name.clone(), canonicalize_info(definition)))
            })
            .collect(),
        sequence_definitions: expected_sequences
            .iter()
            .filter_map(|name| {
                sequence_definitions
                    .get(name)
                    .cloned()
                    .map(|definition| (name.clone(), canonicalize_info(definition)))
            })
            .collect(),
        table_members: table_members
            .into_iter()
            .map(|(name, info)| (name, strip_nested_table_catalog_ids(info)))
            .collect(),
    };
    let canonical_json = serde_json::to_string(&envelope)
        .expect("canonical Atelier structured INFO serializes losslessly");
    Ok(sha256_hex(canonical_json.as_bytes()))
}

fn atelier_schema_ddl() -> String {
    let mut ddl = Vec::new();
    let mut authority_members = Vec::new();
    let mut include_continuation = false;
    let mut include_authority_member_continuation = false;

    let authority = resource_authority_core_block();
    let authority_tables = authority
        .lines()
        .map(str::trim_start)
        .filter_map(|line| line.strip_prefix("DEFINE TABLE OVERWRITE "))
        .filter_map(|rest| rest.split_ascii_whitespace().next())
        .collect::<BTreeSet<_>>();
    let bounded_source = SCHEMA.replacen(authority, "", 1);
    for line in bounded_source.lines() {
        let trimmed = line.trim_start();
        let starts_atelier_statement = trimmed.starts_with("DEFINE TABLE OVERWRITE atelier_")
            || trimmed.starts_with("DEFINE SEQUENCE IF NOT EXISTS atelier_")
            || (trimmed.starts_with("DEFINE FIELD OVERWRITE ")
                && trimmed.contains(" ON TABLE atelier_"))
            || (trimmed.starts_with("DEFINE INDEX OVERWRITE ")
                && trimmed.contains(" ON TABLE atelier_"))
            || (trimmed.starts_with("DEFINE EVENT OVERWRITE ")
                && trimmed.contains(" ON TABLE atelier_"));
        let starts_event_ledger_dependency = trimmed
            .starts_with("DEFINE SEQUENCE IF NOT EXISTS kernel_event_sequence ")
            || trimmed.starts_with("DEFINE TABLE OVERWRITE kernel_event_ledger ")
            || ((trimmed.starts_with("DEFINE FIELD OVERWRITE ")
                || trimmed.starts_with("DEFINE INDEX OVERWRITE "))
                && trimmed.contains(" ON TABLE kernel_event_ledger"));
        let starts_authority_member = (trimmed.starts_with("DEFINE FIELD OVERWRITE ")
            || trimmed.starts_with("DEFINE INDEX OVERWRITE ")
            || trimmed.starts_with("DEFINE EVENT OVERWRITE "))
            && trimmed
                .split_once(" ON TABLE ")
                .and_then(|(_, table_tail)| table_tail.split_ascii_whitespace().next())
                .is_some_and(|table| authority_tables.contains(table.trim_matches('`')));

        if include_authority_member_continuation || starts_authority_member {
            authority_members.push(line);
            include_authority_member_continuation = !trimmed.ends_with(';');
        } else if include_continuation || starts_atelier_statement || starts_event_ledger_dependency
        {
            ddl.push(line);
            include_continuation = !trimmed.ends_with(';');
        }
    }

    let mut ddl = ddl.join("\n");
    ddl.push('\n');
    ddl.push_str(authority);
    ddl.push('\n');
    ddl.push_str(&authority_members.join("\n"));
    ddl.push('\n');
    ddl
}

/// Provisions the exact production-schema tables exercised by the focused Loom
/// mutation-receipt tests. Definitions are selected mechanically from the same
/// compiled `schema.surql` as production bootstrap; no test-owned DDL is used.
#[cfg(test)]
pub async fn bootstrap_loom_receipt_test_schema(
    storage: &SurrealStorage,
) -> Result<(), SurrealStorageError> {
    // FIFTH pin over the Loom receipt-test table set (not the whole schema). Re-pinned by
    // MT-109 (V17 F02): the value below is the catalog WITH MT-109's authority surface inside
    // this table set - `loom_blocks.source_rich_document_id`, the `mt109_loom_source_integrity`
    // event, the record-user SELECT permissions on `loom_blocks` / `knowledge_rich_documents`,
    // and the SCHEMAFULL `wsids` + `authority_*` fields on `kernel_event_ledger` - which MT-109
    // added without moving the pin (previous value 2d3490115ab484a75ef8896bff1152b95798f3c44
    // 37fd45404898172aa87b1c2, pinned by MT-152; validator run MT109-V17-PR-005/PR-006,
    // HANDSHAKE_LOOM_RECEIPT_TEST_SCHEMA_FINGERPRINT_MISMATCH observed).
    // FOURTH pin (MT-152, I-152-2) was the catalog WITH MT-151's `loom_blocks.journal_key` field
    // and `uq_loom_blocks_journal_key` index (previous value 77ab023e..., pinned at e9b81814).
    // SIXTH pin (MT-141 V2-R2): `loom_blocks.pin_order` lost its `>= 0` ASSERT (previous value
    // dc04737a586a4b743e727a2ff215a97e09ab7553df2ef33bc4d045f6a59b1676, the MT-109 pin; kb-v2
    // run 30, HANDSHAKE_LOOM_RECEIPT_TEST_SCHEMA_FINGERPRINT_MISMATCH / MT109_LOOM_CATALOG_SHA256
    // observed).
    // SEVENTH pin (MT-153, kb-c5 run 05): the bounded DDL now also defines
    // fn::mt153_loom_identity_unchanged, which the loom_blocks.block_id ASSERT calls (previous
    // value 8adc1dddc98f2fce6119e38f1689a617a84c6602e8909dd01be60641a8b49164).
    const EXPECTED_CATALOG_SHA256: &str =
        "efc3ecc6ceea3e2a2cb0b1deb717c48a3d79fe8ff7668a17b06fc8f2f10e61b7";
    let ddl = loom_receipt_test_schema_ddl();
    let expected_tables = loom_receipt_test_tables()
        .iter()
        .map(|table| (*table).to_owned())
        .collect::<BTreeSet<_>>();
    let expected_sequences = loom_receipt_test_sequences()
        .iter()
        .map(|sequence| (*sequence).to_owned())
        .collect::<BTreeSet<_>>();
    storage
        .with_admin_operation(move |database| {
            Box::pin(async move {
                database
                    .query(format!("BEGIN TRANSACTION;\n{ddl}\nCOMMIT TRANSACTION;\n"))
                    .await?;
                let present_tables = atelier_table_definitions(&database)
                    .await?
                    .into_keys()
                    .collect::<BTreeSet<_>>();
                if present_tables != expected_tables {
                    return fail_closed(
                        &database,
                        format!(
                            "HANDSHAKE_LOOM_RECEIPT_TEST_SCHEMA_TABLE_MISMATCH: expected={expected_tables:?}; observed={present_tables:?}"
                        ),
                    )
                    .await;
                }
                let observed = inspect_catalog_fingerprint(
                    &database,
                    &expected_tables,
                    &expected_sequences,
                    CatalogInspectionScope::ExactDatabase,
                )
                .await?;
                if observed != EXPECTED_CATALOG_SHA256 {
                    return fail_closed(
                        &database,
                        format!(
                            "HANDSHAKE_LOOM_RECEIPT_TEST_SCHEMA_FINGERPRINT_MISMATCH: expected={EXPECTED_CATALOG_SHA256}; observed={observed}"
                        ),
                    )
                    .await;
                }
                Ok(())
            })
        })
        .await
}

#[cfg(test)]
fn loom_receipt_test_tables() -> &'static [&'static str] {
    &[
        "workspaces",
        "loom_blocks",
        "loom_block_search_index",
        "loom_canvas_boards",
        "loom_canvas_placements",
        "loom_canvas_visual_edges",
        "kernel_event_ledger",
        "knowledge_rich_documents",
        "local_accounts",
        "local_account_setup",
        "principals",
        "access_spaces",
        "authenticated_sessions",
        "session_exchange_credentials",
        "protected_resources",
        "resource_grants",
        "authorization_audit_events",
    ]
}

/// Non-authority-core functions the Loom receipt-test tables call; part of that bounded catalog.
const LOOM_RECEIPT_TEST_TABLE_FUNCTIONS: [&str; 1] = ["mt153_loom_identity_unchanged"];

/// The whole `DEFINE FUNCTION OVERWRITE fn::<name>(` statement from the compiled SCHEMA.
#[cfg(test)]
fn schema_function_statement(name: &str) -> &'static str {
    let start = SCHEMA
        .find(&format!("DEFINE FUNCTION OVERWRITE fn::{name}("))
        .expect("canonical schema defines the Loom table function");
    let end = SCHEMA[start..]
        .find("\n};")
        .map(|offset| start + offset + "\n};".len())
        .expect("canonical schema function terminates");
    &SCHEMA[start..end]
}

#[cfg(test)]
fn loom_receipt_test_sequences() -> &'static [&'static str] {
    &["kernel_event_sequence"]
}

#[cfg(test)]
fn loom_receipt_test_schema_ddl() -> String {
    fn selected_table_statement<'a>(line: &'a str, tables: &[&str]) -> bool {
        let table = if let Some(rest) = line.strip_prefix("DEFINE TABLE OVERWRITE ") {
            rest.split_ascii_whitespace().next()
        } else if line.starts_with("DEFINE FIELD OVERWRITE ")
            || line.starts_with("DEFINE INDEX OVERWRITE ")
            || line.starts_with("DEFINE EVENT OVERWRITE ")
        {
            line.split_once(" ON TABLE ")
                .and_then(|(_, rest)| rest.split_ascii_whitespace().next())
        } else {
            None
        };
        table.is_some_and(|table| tables.contains(&table.trim_end_matches(';')))
    }

    fn selected_sequence_statement(line: &str, sequences: &[&str]) -> bool {
        line.strip_prefix("DEFINE SEQUENCE IF NOT EXISTS ")
            .and_then(|rest| rest.split_ascii_whitespace().next())
            .is_some_and(|sequence| sequences.contains(&sequence.trim_end_matches(';')))
    }

    let mut selected_blocks = vec![resource_authority_core_block()];
    for block in resource_authority_schema_blocks() {
        let first = block.trim_start().lines().next().unwrap_or_default();
        if block != resource_authority_core_block()
            && selected_table_statement(first, loom_receipt_test_tables())
        {
            selected_blocks.push(block);
        }
    }
    let (_, update_guard_block) = SCHEMA
        .split_once("-- MT120_RECORD_USER_UPDATE_GUARD_EVENT_BEGIN\n")
        .expect("MT120 update guard event block start");
    let (update_guard_block, _) = update_guard_block
        .split_once("-- MT120_RECORD_USER_UPDATE_GUARD_EVENT_END")
        .expect("MT120 update guard event block end");
    let selected_update_guard_events = format!("\n{update_guard_block}")
        .split("\nDEFINE EVENT OVERWRITE ")
        .skip(1)
        .map(|event| format!("DEFINE EVENT OVERWRITE {}", event.trim()))
        .filter(|event| selected_table_statement(event, loom_receipt_test_tables()))
        .collect::<Vec<_>>();
    let mut bounded_source = SCHEMA.to_owned();
    for block in &selected_blocks {
        bounded_source = bounded_source.replacen(block, "", 1);
    }
    bounded_source = bounded_source.replacen(
        &format!(
            "-- MT120_RECORD_USER_UPDATE_GUARD_EVENT_BEGIN\n{}-- MT120_RECORD_USER_UPDATE_GUARD_EVENT_END",
            update_guard_block
        ),
        "",
        1,
    );
    let mut ddl = selected_blocks
        .iter()
        .map(|block| (*block).to_owned())
        .collect::<Vec<_>>();
    let mut include_continuation = false;
    let mut include_event_continuation = false;
    for line in bounded_source.lines() {
        let trimmed = line.trim_start();
        let selected_event = trimmed.starts_with("DEFINE EVENT OVERWRITE ")
            && selected_table_statement(trimmed, loom_receipt_test_tables());
        if include_event_continuation
            || include_continuation
            || selected_event
            || selected_table_statement(trimmed, loom_receipt_test_tables())
            || selected_sequence_statement(trimmed, loom_receipt_test_sequences())
        {
            ddl.push(line.to_owned());
            if include_event_continuation {
                if trimmed == "};" || (trimmed.starts_with("THEN ") && trimmed.ends_with("};")) {
                    include_event_continuation = false;
                }
            } else if selected_event {
                include_event_continuation = trimmed != "};";
                include_continuation = false;
            } else {
                include_continuation = !trimmed.ends_with(';');
            }
        }
    }
    ddl.extend(selected_update_guard_events);
    // Functions outside the authority core that a selected table definition calls (MT-153:
    // `loom_blocks.block_id` ASSERT -> fn::mt153_loom_identity_unchanged), copied whole from SCHEMA.
    for name in LOOM_RECEIPT_TEST_TABLE_FUNCTIONS {
        ddl.push(schema_function_statement(name).to_owned());
    }
    let mut ddl = ddl.join("\n");
    ddl.push('\n');
    ddl
}

/// Provisions only the authoritative process-ledger schema wave for focused
/// restart/durability proofs. The DDL is sliced from the same compiled
/// `schema.surql` used by production bootstrap, so this test-support path
/// cannot drift into a hand-maintained substitute schema.
#[cfg(feature = "surreal-test-support")]
pub async fn bootstrap_mt137_process_ledger_test_schema(
    storage: &SurrealStorage,
) -> Result<(), SurrealStorageError> {
    const START: &str = "-- 0021_kernel_process_lifecycle";
    const END: &str = "-- 0022_role_mailbox_threads_messages";
    bootstrap_mt137_test_schema_slice(storage, START, END).await
}

/// Provisions the authoritative EventLedger and aggregate-query schema waves
/// needed by the focused MT-137 Flight Recorder append/reopen/read proof.
#[cfg(feature = "surreal-test-support")]
pub async fn bootstrap_mt137_flight_recorder_test_schema(
    storage: &SurrealStorage,
) -> Result<(), SurrealStorageError> {
    const START: &str = "-- 0018_kernel_event_ledger";
    const END: &str =
        "-- 0030-0059 Atelier schema projection. Historical legacy server backend backfill DML is";
    bootstrap_mt137_test_schema_slice(storage, START, END).await
}

#[cfg(feature = "surreal-test-support")]
async fn bootstrap_mt137_test_schema_slice(
    storage: &SurrealStorage,
    start: &'static str,
    end: &'static str,
) -> Result<(), SurrealStorageError> {
    let (_, after_start) = SCHEMA
        .split_once(start)
        .expect("compiled Surreal schema contains the focused MT-137 schema start");
    let (ddl, _) = after_start
        .split_once(end)
        .expect("compiled Surreal schema contains the focused MT-137 schema end");
    let ddl = ddl.to_owned();
    storage
        .with_admin_operation(move |database| {
            Box::pin(async move {
                database.query(ddl).await?;
                Ok(())
            })
        })
        .await
}

const TABLE_NAMES: [&str; TABLE_DEFINITION_COUNT] = [
    "access_spaces",
    "adapter_checkpoint",
    "ai_bronze_records",
    "ai_embedding_models",
    "ai_embedding_registry",
    "ai_job_mcp_fields",
    "ai_jobs",
    "ai_silver_records",
    "assets",
    "atelier_action_receipt",
    "atelier_ai_tag_suggestion",
    "atelier_anchor_verification_record",
    "atelier_backup_manifest",
    "atelier_backup_restore_preflight",
    "atelier_bracket_link_projection",
    "atelier_bulk_operation_receipt",
    "atelier_caption_artifact",
    "atelier_character",
    "atelier_character_document",
    "atelier_character_document_version",
    "atelier_character_relationship",
    "atelier_character_relationship_graph_projection",
    "atelier_character_script",
    "atelier_character_tag",
    "atelier_collection",
    "atelier_collection_item",
    "atelier_collection_metadata_application",
    "atelier_comfy_bridge_probe",
    "atelier_comfy_capability_registration",
    "atelier_comfy_capability_reject",
    "atelier_comfy_declared_output",
    "atelier_comfy_diagnostic_bundle",
    "atelier_comfy_fallback_marker",
    "atelier_comfy_intake_output",
    "atelier_comfy_job",
    "atelier_comfy_output_registration_failure",
    "atelier_comfy_version_metadata",
    "atelier_comfy_workflow_receipt",
    "atelier_comfy_workflow_spec",
    "atelier_command_corpus_blocked",
    "atelier_command_corpus_entry",
    "atelier_command_corpus_parity_report",
    "atelier_command_log",
    "atelier_contact_sheet",
    "atelier_contact_sheet_raster_export_plan",
    "atelier_contact_sheet_svg_artifact",
    "atelier_dcc_panel_projection",
    "atelier_dcc_workflow_panel_projection",
    "atelier_diagnostics_error_taxonomy",
    "atelier_diagnostics_prompt_response_matrix",
    "atelier_diagnostics_session",
    "atelier_diagnostics_validation_matrix",
    "atelier_event",
    "atelier_export_intake_link",
    "atelier_export_manifest_entry",
    "atelier_export_request",
    "atelier_export_result",
    "atelier_filesystem_health_check",
    "atelier_filesystem_health_finding",
    "atelier_fr_workflow_event",
    "atelier_handler_version_matrix",
    "atelier_identity_crop_artifact",
    "atelier_identity_profile",
    "atelier_image_import_request",
    "atelier_intake_batch",
    "atelier_intake_item",
    "atelier_intake_item_loom_projection",
    "atelier_intake_item_rejection_audit",
    "atelier_md_allowlist_policy",
    "atelier_md_auth_context",
    "atelier_md_checkpoint",
    "atelier_md_download_session",
    "atelier_md_item_state",
    "atelier_md_output_root",
    "atelier_md_session_receipt",
    "atelier_media_annotation",
    "atelier_media_asset",
    "atelier_media_asset_tag",
    "atelier_media_derivative",
    "atelier_media_probe_report",
    "atelier_media_review_metadata",
    "atelier_media_sidecar",
    "atelier_media_source_provenance_ref",
    "atelier_model_apply",
    "atelier_model_config",
    "atelier_model_coordination_lease",
    "atelier_model_manual_drift_guard",
    "atelier_model_manual_row_merge",
    "atelier_model_manual_section",
    "atelier_moodboard",
    "atelier_moodboard_export_request",
    "atelier_moodboard_operation_receipt",
    "atelier_orphan_manifest",
    "atelier_orphan_manifest_item",
    "atelier_pose_calibration",
    "atelier_pose_context_state",
    "atelier_pose_deferred_feature",
    "atelier_pose_head_pose",
    "atelier_pose_rig",
    "atelier_pose_sidecar",
    "atelier_pose_workspace_rig_state",
    "atelier_preference",
    "atelier_reset_operation",
    "atelier_retrieval_policy",
    "atelier_saved_search",
    "atelier_saved_search_retrieval_projection",
    "atelier_screenshot_artifact_storage",
    "atelier_self_improve_sandbox_run",
    "atelier_sheet_parse_snapshot",
    "atelier_sheet_version",
    "atelier_similarity_projection",
    "atelier_similarity_rebuild_job",
    "atelier_source_evidence_record",
    "atelier_sourcing_binding_decision",
    "atelier_sourcing_ingestion_receipt",
    "atelier_sourcing_spec",
    "atelier_spec_drift_finding",
    "atelier_state_probe_catalog_entry",
    "atelier_stealth_capture",
    "atelier_stealth_ref",
    "atelier_stealth_window",
    "atelier_story_beat",
    "atelier_story_card",
    "atelier_synthetic_input_guard",
    "atelier_tag",
    "atelier_tag_rule",
    "atelier_transcript_artifact",
    "atelier_transcript_receipt",
    "atelier_trash_marker",
    "atelier_validator_first_pass_run",
    "atelier_version_mismatch_receipt",
    "atelier_visual_steer_feedback",
    "atelier_web_portfolio_export_request",
    "atelier_web_portfolio_export_result",
    "atelier_work_state_projection",
    "authenticated_sessions",
    "authorization_audit_events",
    "blocks",
    "calendar_activity_spans",
    "calendar_events",
    "calendar_mutation_outbox",
    "calendar_sources",
    "canvas_edges",
    "canvas_nodes",
    "canvases",
    "dependencies",
    "distill_example",
    "distill_job",
    "documents",
    "eval_run",
    "fems_memory_commit_fr_outbox",
    "fems_memory_commit_reports",
    "fems_memory_items",
    "fems_memory_lifecycle_fr_outbox",
    "fems_memory_packs",
    "fems_memory_proposal_request_id_rekey",
    "fems_memory_proposals",
    "fems_workspace_write_anchors",
    "governance_check_runs",
    "handshake_schema_state",
    "kb003_promotion_decisions",
    "kb003_promotion_receipts",
    "kb003_sandbox_policies",
    "kb003_sandbox_runs",
    "kb003_validation_runs",
    "kernel_activity_span",
    "kernel_crdt_snapshots",
    "kernel_crdt_updates",
    "kernel_diagnostic_bundle_manifest",
    "kernel_distillation_candidate",
    "kernel_event_ledger",
    "kernel_idempotency_ledger",
    "kernel_micro_task_job",
    "kernel_model_session_span",
    "kernel_mt_loop_checkpoint",
    "kernel_mt_outcome",
    "kernel_process_lifecycle",
    "kernel_restart_resume_report",
    "kernel_session_checkpoint",
    "kernel_session_queue",
    "kernel_visual_diff_baseline",
    "kernel_visual_diff_request",
    "kernel_visual_diff_result",
    "knowledge_agent_cloud_assistance_receipts",
    "knowledge_agent_quiet_background_work",
    "knowledge_agent_recovery_receipts",
    "knowledge_agent_role_mailbox_handoffs",
    "knowledge_agent_state_recovery_checkpoints",
    "knowledge_agent_worktree_claims",
    "knowledge_claim_conflicts",
    "knowledge_claim_spans",
    "knowledge_claims",
    "knowledge_code_files",
    "knowledge_code_repair_queue",
    "knowledge_code_scip_imports",
    "knowledge_context_bundle_items",
    "knowledge_context_bundles",
    "knowledge_crdt_agent_lane_leases",
    "knowledge_crdt_ai_edit_proposals",
    "knowledge_crdt_denial_receipts",
    "knowledge_crdt_graph_proposals",
    "knowledge_crdt_promoted_facts",
    "knowledge_crdt_recovery_receipts",
    "knowledge_crdt_swarm_checkpoints",
    "knowledge_debug_breakpoints",
    "knowledge_document_backlinks",
    "knowledge_document_embeds",
    "knowledge_edge_spans",
    "knowledge_edges",
    "knowledge_editor_code_nodes",
    "knowledge_entities",
    "knowledge_entity_spans",
    "knowledge_idempotency_keys",
    "knowledge_index_runs",
    "knowledge_ingestion_kind_registry",
    "knowledge_ingestion_policy_decisions",
    "knowledge_ingestion_receipts",
    "knowledge_ingestion_repair_queue",
    "knowledge_ingestion_root_policies",
    "knowledge_ingestion_spans",
    "knowledge_memory_bridge_decisions",
    "knowledge_memory_conflict_detection_findings",
    "knowledge_memory_conflict_detection_jobs",
    "knowledge_memory_conflict_resolution_jobs",
    "knowledge_memory_facts",
    "knowledge_memory_ontology_aliases",
    "knowledge_memory_ontology_terms",
    "knowledge_memory_passages",
    "knowledge_parallel_indexing_lease_queue",
    "knowledge_passage_evidence",
    "knowledge_quick_switcher_recents",
    "knowledge_retrieval_traces",
    "knowledge_rich_document_drafts",
    "knowledge_rich_document_loom_projection_0343_state",
    "knowledge_rich_document_title_anchors",
    "knowledge_rich_document_versions",
    "knowledge_rich_documents",
    "knowledge_schema_registry",
    "knowledge_semantic_catalog_entries",
    "knowledge_source_roots",
    "knowledge_sources",
    "knowledge_spans",
    "knowledge_wiki_projections",
    "knowledge_workbench_layout_states",
    "knowledge_workspace_search_bookmark_states",
    "knowledge_workspace_settings_states",
    "local_accounts",
    "local_account_setup",
    "loom_ai_suggestions",
    "loom_block_knowledge_bridge",
    "loom_block_search_index",
    "loom_block_view_fr_outbox",
    "loom_blocks",
    "loom_canvas_boards",
    "loom_canvas_placements",
    "loom_canvas_visual_edges",
    "loom_collection_members",
    "loom_collections",
    "loom_edges",
    "loom_folder_members",
    "loom_folders",
    "loom_wiki_overlays",
    "media_asset_tiers",
    "micro_tasks",
    "model_session_checkpoints",
    "model_session_messages",
    "model_sessions",
    "mt_iterations",
    "preference_change_receipts",
    "preference_records",
    "principals",
    "protected_resources",
    "replay_candidates",
    "resource_grants",
    "role_mailbox_claim_lease",
    "role_mailbox_handoff_bundle",
    "role_mailbox_message",
    "role_mailbox_thread",
    "session_exchange_credentials",
    "skill_log_entry",
    "skill_log_file_ref",
    "stage_capture_artifacts",
    "storage_graph_anchors",
    "user_manual_anchors",
    "user_manual_feature_entries",
    "user_manual_legacy_aliases",
    "user_manual_pages",
    "user_manual_sections",
    "user_manual_tool_entries",
    "user_manual_versions",
    "work_packets",
    "workflow_node_executions",
    "workflow_runs",
    "workspaces",
];

/// Tables whose source `id` column is represented only by the Surreal record ID.
const RECORD_ID_ONLY_TABLES: [&str; 18] = [
    "workspaces",
    "documents",
    "blocks",
    "canvases",
    "canvas_nodes",
    "canvas_edges",
    "ai_jobs",
    "workflow_runs",
    "workflow_node_executions",
    "ai_embedding_registry",
    "calendar_sources",
    "calendar_events",
    "skill_log_entry",
    "skill_log_file_ref",
    "distill_job",
    "adapter_checkpoint",
    "eval_run",
    "preference_change_receipts",
];

/// Referenced targets that retain a domain-facing single-column key alias.
/// Each corresponding field ASSERTs equality with `record::id($this.id)`.
const REFERENCED_BUSINESS_KEY_ALIASES: [(&str, &str); 9] = [
    ("ai_bronze_records", "bronze_id"),
    ("assets", "asset_id"),
    ("loom_blocks", "block_id"),
    ("work_packets", "wp_id"),
    ("kernel_event_ledger", "event_id"),
    ("role_mailbox_thread", "thread_id"),
    ("role_mailbox_claim_lease", "lease_id"),
    ("kernel_micro_task_job", "job_id"),
    ("kernel_model_session_span", "span_id"),
];

#[derive(Debug, Clone, Deserialize, SurrealValue, PartialEq, Eq)]
struct SchemaState {
    version: String,
    revision: i64,
    namespace: String,
    database: String,
    source_manifest_sha256: String,
    generated_surql_sha256: String,
    info_fingerprint_sha256: String,
    apply_state: String,
    target_revision: i64,
}

#[derive(SurrealValue)]
struct BootstrapBindings {
    schema_version: String,
    schema_revision: i64,
    namespace: String,
    database: String,
    source_manifest_sha256: String,
    generated_surql_sha256: String,
}

#[derive(SurrealValue)]
struct FinalizeBindings {
    schema_version: String,
    schema_revision: i64,
    namespace: String,
    database: String,
    source_manifest_sha256: String,
    generated_surql_sha256: String,
    pending_info_fingerprint_sha256: String,
    info_fingerprint_sha256: String,
}

#[derive(SurrealValue)]
struct PredecessorUpgradeBindings {
    schema_version: String,
    predecessor_revision: i64,
    schema_revision: i64,
    namespace: String,
    database: String,
    source_manifest_sha256: String,
    predecessor_generated_surql_sha256: String,
    predecessor_info_fingerprint_sha256: String,
    generated_surql_sha256: String,
    pending_info_fingerprint_sha256: String,
    schema_source: String,
}

impl SchemaState {
    fn has_stable_v1_identity(&self) -> bool {
        self.version == SCHEMA_VERSION
            && self.revision == SCHEMA_REVISION
            && self.target_revision == SCHEMA_REVISION
            && self.namespace == DEFAULT_NAMESPACE
            && self.database == DEFAULT_DATABASE
            && self.source_manifest_sha256 == SCHEMA_LINEAGE_SHA256
    }

    fn is_schema_applied_current(&self) -> bool {
        self.has_stable_v1_identity()
            && self.generated_surql_sha256 == GENERATED_SURREALQL_SHA256
            && self.apply_state == "schema_applied"
            && self.info_fingerprint_sha256 == PENDING_SCHEMA_INFO_SHA256
    }

    fn is_exact_current(&self) -> bool {
        self.has_stable_v1_identity()
            && self.generated_surql_sha256 == GENERATED_SURREALQL_SHA256
            && self.apply_state == "complete"
            && self.info_fingerprint_sha256 == EXPECTED_SCHEMA_INFO_SHA256
    }

    fn has_pre_standalone_loom_update_identity(&self) -> bool {
        self.version == SCHEMA_VERSION
            && self.revision == PRE_STANDALONE_LOOM_UPDATE_REVISION
            && self.target_revision == PRE_STANDALONE_LOOM_UPDATE_REVISION
            && self.namespace == DEFAULT_NAMESPACE
            && self.database == DEFAULT_DATABASE
            && self.source_manifest_sha256 == SCHEMA_LINEAGE_SHA256
    }

    fn is_exact_pre_standalone_loom_update_current(&self) -> bool {
        self.has_pre_standalone_loom_update_identity()
            && self.generated_surql_sha256 == PRE_STANDALONE_LOOM_UPDATE_GENERATED_SHA256
            && self.apply_state == "complete"
            && self.info_fingerprint_sha256 == PRE_STANDALONE_LOOM_UPDATE_INFO_SHA256
    }

    fn has_pre_canvas_receipt_identity(&self) -> bool {
        self.version == SCHEMA_VERSION
            && self.revision == PRE_CANVAS_RECEIPT_REVISION
            && self.target_revision == PRE_CANVAS_RECEIPT_REVISION
            && self.namespace == DEFAULT_NAMESPACE
            && self.database == DEFAULT_DATABASE
            && self.source_manifest_sha256 == SCHEMA_LINEAGE_SHA256
    }

    fn is_exact_pre_canvas_receipt_current(&self) -> bool {
        self.has_pre_canvas_receipt_identity()
            && self.generated_surql_sha256 == PRE_CANVAS_RECEIPT_GENERATED_SHA256
            && self.apply_state == "complete"
            && self.info_fingerprint_sha256 == PRE_CANVAS_RECEIPT_INFO_SHA256
    }

    fn has_pre_account_setup_identity(&self) -> bool {
        self.version == SCHEMA_VERSION
            && self.revision == PRE_ACCOUNT_SETUP_REVISION
            && self.target_revision == PRE_ACCOUNT_SETUP_REVISION
            && self.namespace == DEFAULT_NAMESPACE
            && self.database == DEFAULT_DATABASE
            && self.source_manifest_sha256 == SCHEMA_LINEAGE_SHA256
    }

    fn is_exact_pre_account_setup(&self) -> bool {
        self.has_pre_account_setup_identity()
            && self.generated_surql_sha256 == PRE_ACCOUNT_SETUP_GENERATED_SHA256
            && self.apply_state == "complete"
            && self.info_fingerprint_sha256 == PRE_ACCOUNT_SETUP_INFO_SHA256
    }

    fn is_exact_pre_mt109_current(&self) -> bool {
        self.has_pre_account_setup_identity()
            && self.generated_surql_sha256 == PRE_MT109_GENERATED_SURREALQL_SHA256
            && self.apply_state == "complete"
            && self.info_fingerprint_sha256 == PRE_MT109_SCHEMA_INFO_SHA256
    }

    /// Exact MT-109 current lineage (revision 157 with the authority surface, before the
    /// MT-150 `loom_edges.event_ledger_event_id` receipt binding).
    fn is_exact_pre_mt150_current(&self) -> bool {
        self.has_pre_account_setup_identity()
            && self.generated_surql_sha256 == PRE_MT150_GENERATED_SURREALQL_SHA256
            && self.apply_state == "complete"
            && self.info_fingerprint_sha256 == PRE_MT150_SCHEMA_INFO_SHA256
    }

    /// Exact MT-150 current lineage (revision 157 with the loom_edges receipt binding, before
    /// the MT-141 R9 provenance-ref constraint moved onto `asset_id`).
    fn is_exact_pre_mt141_current(&self) -> bool {
        self.has_pre_account_setup_identity()
            && self.generated_surql_sha256 == PRE_MT141_GENERATED_SURREALQL_SHA256
            && self.apply_state == "complete"
            && self.info_fingerprint_sha256 == PRE_MT141_SCHEMA_INFO_SHA256
    }

    fn is_exact_supported_predecessor(&self) -> bool {
        self.has_pre_account_setup_identity()
            && self.generated_surql_sha256 == PREDECESSOR_GENERATED_SURREALQL_SHA256
            && self.apply_state == "complete"
            && self.info_fingerprint_sha256 == PREDECESSOR_SCHEMA_INFO_SHA256
    }

    /// Exact pre-MT-142 current lineage (revision 157 without the title-anchor table).
    fn is_exact_pre_mt142_current(&self) -> bool {
        self.has_pre_account_setup_identity()
            && self.generated_surql_sha256 == PRE_MT142_GENERATED_SURREALQL_SHA256
            && self.apply_state == "complete"
            && self.info_fingerprint_sha256 == PRE_MT142_SCHEMA_INFO_SHA256
    }

    /// Exact MT-142 current lineage (revision 157 with the title-anchor table, before the
    /// MT-151 journal key and graph anchors).
    fn is_exact_pre_mt151_current(&self) -> bool {
        self.has_pre_account_setup_identity()
            && self.generated_surql_sha256 == PRE_MT151_GENERATED_SURREALQL_SHA256
            && self.apply_state == "complete"
            && self.info_fingerprint_sha256 == PRE_MT151_SCHEMA_INFO_SHA256
    }

    /// Exact MT-151 current lineage (revision 157 with journal_key and graph anchors, before
    /// the MT-152 FEMS workspace write anchors).
    fn is_exact_pre_mt152_current(&self) -> bool {
        self.has_pre_account_setup_identity()
            && self.generated_surql_sha256 == PRE_MT152_GENERATED_SURREALQL_SHA256
            && self.apply_state == "complete"
            && self.info_fingerprint_sha256 == PRE_MT152_SCHEMA_INFO_SHA256
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SchemaBootstrapOutcome {
    InstalledFresh,
    ReusedExactCurrent,
    ResumedCurrentApply,
    UpgradedSupportedPredecessor,
}

impl SchemaBootstrapOutcome {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::InstalledFresh => "installed_fresh",
            Self::ReusedExactCurrent => "reused_exact_current",
            Self::ResumedCurrentApply => "resumed_current_apply",
            Self::UpgradedSupportedPredecessor => "upgraded_supported_predecessor",
        }
    }

    const fn reused_existing_schema(self) -> bool {
        !matches!(self, Self::InstalledFresh)
    }
}

/// Receipt derived from the durable state row and live INFO introspection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchemaBootstrapReport {
    pub schema_version: String,
    pub namespace: String,
    pub database: String,
    pub declarative_schema_files: usize,
    pub source_manifest_sha256: String,
    pub generated_surql_sha256: String,
    pub info_fingerprint_sha256: String,
    pub tables_defined: usize,
    pub fields_defined: usize,
    pub indexes_defined: usize,
    pub table_names: Vec<String>,
    pub outcome: SchemaBootstrapOutcome,
    /// Compatibility projection. Prefer [`SchemaBootstrapReport::outcome`] when mutation matters.
    pub reused_existing_schema: bool,
}

#[derive(Debug)]
struct ObservedSchema {
    info_fingerprint_sha256: String,
    tables_defined: usize,
    fields_defined: usize,
    indexes_defined: usize,
    table_names: Vec<String>,
}

#[derive(Serialize)]
struct CanonicalInfoEnvelope {
    database: SurrealValueData,
    tables: BTreeMap<String, SurrealValueData>,
}

#[derive(Debug, Clone, Deserialize, SurrealValue, PartialEq, Eq)]
struct KnowledgeSchemaRegistryMetadata {
    family_key: String,
    table_name: String,
    record_family: String,
    authority_class: String,
    schema_source: String,
    wp_id: String,
    mt_id: String,
}

#[derive(Debug, Clone, Deserialize, SurrealValue, PartialEq, Eq)]
struct PredecessorKnowledgeSchemaRegistryMetadata {
    family_key: String,
    table_name: String,
    record_family: String,
    authority_class: String,
    retired_source: String,
    wp_id: String,
    mt_id: String,
}

fn expected_knowledge_schema_registry_metadata(
) -> Result<Vec<KnowledgeSchemaRegistryMetadata>, String> {
    let mut rows = Vec::new();
    for line in KNOWLEDGE_SCHEMA_REGISTRY_SEED.lines() {
        let line = line.trim();
        if !line.starts_with("{ family_key:") {
            continue;
        }
        let values = line
            .split('\'')
            .skip(1)
            .step_by(2)
            .map(str::to_owned)
            .collect::<Vec<_>>();
        if values.len() != 7 {
            return Err(format!(
                "HANDSHAKE_SURREAL_KNOWLEDGE_REGISTRY_SEED_PARSE_FAILED: {line}"
            ));
        }
        rows.push(KnowledgeSchemaRegistryMetadata {
            family_key: values[0].clone(),
            table_name: values[1].clone(),
            record_family: values[2].clone(),
            authority_class: values[3].clone(),
            schema_source: values[4].clone(),
            wp_id: values[5].clone(),
            mt_id: values[6].clone(),
        });
    }
    rows.sort_by(|left, right| left.family_key.cmp(&right.family_key));
    // MT-142 re-pin: 61 historical rows + 0343 state row + rich_document_title_anchors row.
    if rows.len() != 63 {
        return Err(format!(
            "HANDSHAKE_SURREAL_KNOWLEDGE_REGISTRY_SEED_COUNT: expected=63 observed={}",
            rows.len()
        ));
    }
    Ok(rows)
}

fn expected_predecessor_registry_metadata(
) -> Result<Vec<PredecessorKnowledgeSchemaRegistryMetadata>, String> {
    let mut rows = Vec::new();
    let mut family_keys = BTreeSet::new();
    for line in PREDECESSOR_KNOWLEDGE_SCHEMA_REGISTRY_SEED.lines() {
        let line = line.trim();
        if !line.starts_with("['") {
            continue;
        }
        let values = line
            .split('\'')
            .skip(1)
            .step_by(2)
            .map(str::to_owned)
            .collect::<Vec<_>>();
        if values.len() != 7 {
            return Err(format!(
                "HANDSHAKE_SURREAL_PREDECESSOR_REGISTRY_SEED_PARSE_FAILED: {line}"
            ));
        }
        if !family_keys.insert(values[0].clone()) {
            return Err(format!(
                "HANDSHAKE_SURREAL_PREDECESSOR_REGISTRY_FAMILY_DUPLICATE: {}",
                values[0]
            ));
        }
        rows.push(PredecessorKnowledgeSchemaRegistryMetadata {
            family_key: values[0].clone(),
            table_name: values[1].clone(),
            record_family: values[2].clone(),
            authority_class: values[3].clone(),
            retired_source: values[4].clone(),
            wp_id: values[5].clone(),
            mt_id: values[6].clone(),
        });
    }
    rows.sort_by(|left, right| left.family_key.cmp(&right.family_key));
    if rows.len() != 61 {
        return Err(format!(
            "HANDSHAKE_SURREAL_PREDECESSOR_REGISTRY_SEED_COUNT: expected=61 observed={}",
            rows.len()
        ));
    }
    Ok(rows)
}

async fn read_knowledge_schema_registry_metadata(
    database: &SurrealAdminContext<'_>,
) -> Result<Vec<KnowledgeSchemaRegistryMetadata>, SurrealStorageError> {
    let mut response = database
        .query(
            "SELECT family_key, table_name, record_family, authority_class, schema_source, \
             wp_id, mt_id FROM knowledge_schema_registry ORDER BY family_key ASC;",
        )
        .await?;
    Ok(response.take(0)?)
}

fn compute_predecessor_registry_hash(
    rows: &[PredecessorKnowledgeSchemaRegistryMetadata],
) -> String {
    let mut sorted = rows.to_vec();
    sorted.sort_by(|left, right| left.family_key.cmp(&right.family_key));
    let mut hasher = Sha256::new();
    hasher.update(PREDECESSOR_KNOWLEDGE_REGISTRY_DOMAIN);
    for row in sorted {
        for field in [
            row.family_key.as_str(),
            row.table_name.as_str(),
            row.record_family.as_str(),
            row.authority_class.as_str(),
            row.retired_source.as_str(),
            row.wp_id.as_str(),
            row.mt_id.as_str(),
        ] {
            hasher.update((field.len() as u32).to_be_bytes());
            hasher.update(field.as_bytes());
        }
    }
    format!("{:x}", hasher.finalize())
}

async fn ensure_supported_predecessor_registry(
    database: &SurrealAdminContext<'_>,
) -> Result<(), SurrealStorageError> {
    let mut response = database
        .query(
            "SELECT family_key, table_name, record_family, authority_class, \
             migration_file AS retired_source, wp_id, mt_id \
             FROM knowledge_schema_registry ORDER BY family_key ASC;",
        )
        .await?;
    let observed: Vec<PredecessorKnowledgeSchemaRegistryMetadata> = response.take(0)?;
    if observed.len() != 61
        || observed.iter().any(|row| row.retired_source.is_empty())
        || compute_predecessor_registry_hash(&observed) != PREDECESSOR_KNOWLEDGE_REGISTRY_SHA256
    {
        return fail_closed(
            database,
            format!(
                "HANDSHAKE_SURREAL_PREDECESSOR_KNOWLEDGE_REGISTRY_DIVERGENT: rows={}; sha256={}",
                observed.len(),
                compute_predecessor_registry_hash(&observed)
            ),
        )
        .await;
    }
    Ok(())
}

async fn ensure_knowledge_schema_registry(
    database: &SurrealAdminContext<'_>,
) -> Result<(), SurrealStorageError> {
    let expected = match expected_knowledge_schema_registry_metadata() {
        Ok(expected) => expected,
        Err(reason) => return fail_closed(database, reason).await,
    };
    let observed = read_knowledge_schema_registry_metadata(database).await?;
    if observed == expected {
        return Ok(());
    }
    if !observed.is_empty() {
        return fail_closed(
            database,
            format!(
                "HANDSHAKE_SURREAL_KNOWLEDGE_REGISTRY_DIVERGENT: expected={expected:?}; observed={observed:?}"
            ),
        )
        .await;
    }
    database.query(KNOWLEDGE_SCHEMA_REGISTRY_SEED).await?;
    let seeded = read_knowledge_schema_registry_metadata(database).await?;
    if seeded != expected {
        return fail_closed(
            database,
            format!(
                "HANDSHAKE_SURREAL_KNOWLEDGE_REGISTRY_SEED_VERIFY_FAILED: expected={expected:?}; observed={seeded:?}"
            ),
        )
        .await;
    }
    Ok(())
}

/// The fresh-bootstrap execution plan of [`SCHEMA`], split into two queries.
struct FreshBootstrapScript {
    /// `BEGIN` ... every table, field, event, function and access definition ... `COMMIT`.
    definitions: String,
    /// Every `DEFINE INDEX` of the first transaction as its own statement (no explicit
    /// transaction), then the statements [`SCHEMA`] already runs after its transaction, then
    /// `BEGIN` ... the `schema_applied` receipt ... `COMMIT`.
    indexes_and_rest: String,
}

/// Splits the compiled [`SCHEMA`] for a fresh bootstrap (IV 2026-09-23, second zero-CPU hang root
/// cause): SurrealDB 3.2.0 runs an index builder per `DEFINE INDEX`
/// (surrealdb-core-3.2.0 `expr/statements/define/index.rs:229-235`), and the builder retries a
/// retryable transaction conflict every 100 ms without a bound (`kvs/index/builder.rs:822-832`)
/// while ~4,500 other definitions share its transaction (IV dump evidence: the builder's
/// `mark_durable_online` commit keeps retrying while an outer transaction holds its snapshot).
/// Every index therefore runs after the definitions transaction committed, each as its own
/// statement outside any explicit transaction (one implicit transaction and one build at a time),
/// on the still-empty tables (blocking, no `CONCURRENTLY`). The `schema_applied` receipt moves
/// into a final short transaction after every index is online and every post-transaction
/// statement ran. The compiled text, its hash and the declarative catalog are unchanged; only
/// execution is split. Statements are moved whole (continuation lines until `;`) and in source
/// order.
fn fresh_bootstrap_script(schema: &str) -> Result<FreshBootstrapScript, String> {
    const RECEIPT_START: &str = "UPSERT handshake_schema_state:primary SET";
    let lines = schema.lines().collect::<Vec<_>>();
    let begin = lines
        .iter()
        .position(|line| *line == "BEGIN TRANSACTION;")
        .ok_or("schema has no BEGIN TRANSACTION")?;
    let commit = lines
        .iter()
        .position(|line| *line == "COMMIT TRANSACTION;")
        .ok_or("schema has no COMMIT TRANSACTION")?;
    if commit <= begin {
        return Err("COMMIT precedes BEGIN".to_owned());
    }
    let mut definitions = lines[..commit]
        .iter()
        .map(|line| (*line).to_owned())
        .collect::<Vec<_>>();
    let mut kept = Vec::with_capacity(definitions.len());
    let mut indexes = Vec::new();
    let mut receipt = Vec::new();
    let mut continuation: Option<bool> = None; // Some(true) = index, Some(false) = receipt
    for (offset, line) in definitions.drain(..).enumerate() {
        let inside = offset > begin;
        let trimmed = line.trim_end();
        let target = match continuation {
            Some(kind) => Some(kind),
            None if inside && line.starts_with("DEFINE INDEX ") => Some(true),
            None if inside && line.starts_with(RECEIPT_START) => Some(false),
            None => None,
        };
        match target {
            Some(true) => indexes.push(line.clone()),
            Some(false) => receipt.push(line.clone()),
            None => kept.push(line.clone()),
        }
        continuation = match target {
            Some(kind) if !trimmed.ends_with(';') => Some(kind),
            _ => None,
        };
    }
    if continuation.is_some() {
        return Err("unterminated moved statement before COMMIT".to_owned());
    }
    if indexes.is_empty() || receipt.is_empty() {
        return Err(format!(
            "expected indexes and the schema_applied receipt inside the transaction (indexes={}, receipt_lines={})",
            indexes.len(),
            receipt.len()
        ));
    }
    let mut definitions = kept.join("\n");
    definitions.push_str("\nCOMMIT TRANSACTION;\n");
    let mut indexes_and_rest = indexes.join("\n");
    indexes_and_rest.push('\n');
    indexes_and_rest.push_str(&lines[commit + 1..].join("\n"));
    indexes_and_rest.push_str("\nBEGIN TRANSACTION;\n");
    indexes_and_rest.push_str(&receipt.join("\n"));
    indexes_and_rest.push_str("\nCOMMIT TRANSACTION;\n");
    Ok(FreshBootstrapScript {
        definitions,
        indexes_and_rest,
    })
}

/// True only for the interrupted fresh bootstrap: the live table set is exactly the compiled
/// [`TABLE_NAMES`] and the bootstrap receipt row does not exist. Any other state (empty, current,
/// predecessor, foreign) is left to [`read_context_and_state`], which fails closed as before.
async fn definitions_committed_without_receipt(
    database: &SurrealAdminContext<'_>,
) -> Result<bool, SurrealStorageError> {
    let mut response = database.query("INFO FOR DB STRUCTURE;").await?;
    let database_info: SurrealValueData = response.take(0)?;
    let Ok(tables) = parse_named_array(&database_info, "tables") else {
        return Ok(false);
    };
    let live = tables.iter().map(String::as_str).collect::<BTreeSet<_>>();
    let compiled = TABLE_NAMES.iter().copied().collect::<BTreeSet<_>>();
    if live != compiled {
        return Ok(false);
    }
    let mut receipt = database
        .query(format!("SELECT VALUE id FROM {BOOTSTRAP_STATE_TABLE};"))
        .await?;
    let rows: Vec<RecordId> = receipt.take(0)?;
    Ok(rows.is_empty())
}

/// Installs the sole declarative Surreal schema or verifies an exact-current schema.
///
/// V1 fails closed for every lower, divergent, or unknown lineage. One exact allowlisted
/// predecessor is upgraded transactionally from its retired registry field to the declarative
/// `schema_source` field; no deleted migration file is read or executed. The sole resumable
/// incomplete state is the exact-current `schema_applied` receipt written after committed DDL or
/// predecessor upgrade (on a fresh store it commits in a final short transaction after every
/// index; an interrupted fresh bootstrap resumes through
/// [`definitions_committed_without_receipt`]). It is finalized only after complete live INFO
/// matches the compiled fingerprint. A process-wide mutex serializes callers; each transaction
/// rechecks durable state before mutation. Exact-current restarts return before executing any
/// `OVERWRITE` statement.
///
/// MT-154 hardening: the whole bootstrap (mutex wait included) runs under
/// [`BOOTSTRAP_WATCHDOG`]; a bootstrap that never returns (observed: the commit coordinator blocked
/// in the OS file flush) becomes [`SurrealStorageError::BootstrapStalled`] instead of a silent hang.
/// The sync mode is unchanged.
pub async fn bootstrap_schema(
    storage: &SurrealStorage,
) -> Result<SchemaBootstrapReport, SurrealStorageError> {
    bounded_bootstrap(BOOTSTRAP_WATCHDOG, bootstrap_schema_unbounded(storage)).await
}

/// Generous on purpose: the fresh bootstrap is ~4,500 statements, and too-tight engine deadlines
/// already broke it on loaded disks (storage/surreal.rs, DEFAULT_ENGINE_QUERY_TIMEOUT notes). A
/// healthy bootstrap finishes far below this bound; only a stalled one reaches it.
const BOOTSTRAP_WATCHDOG: std::time::Duration = std::time::Duration::from_secs(900);

async fn bounded_bootstrap<T>(
    limit: std::time::Duration,
    bootstrap: impl std::future::Future<Output = Result<T, SurrealStorageError>>,
) -> Result<T, SurrealStorageError> {
    match tokio::time::timeout(limit, bootstrap).await {
        Ok(result) => result,
        Err(_elapsed) => {
            tracing::error!(
                target: "handshake_core",
                waited_ms = limit.as_millis() as u64,
                "schema bootstrap stalled (possible OS flush stall)"
            );
            Err(SurrealStorageError::BootstrapStalled {
                waited_ms: limit.as_millis(),
            })
        }
    }
}

async fn bootstrap_schema_unbounded(
    storage: &SurrealStorage,
) -> Result<SchemaBootstrapReport, SurrealStorageError> {
    let _bootstrap_guard = BOOTSTRAP_MUTEX.lock().await;
    let report = storage
        .with_admin_operation(|database| {
            Box::pin(async move {
                verify_compiled_manifest(&database).await?;
                let bindings = || BootstrapBindings {
                    schema_version: SCHEMA_VERSION.to_owned(),
                    schema_revision: SCHEMA_REVISION,
                    namespace: DEFAULT_NAMESPACE.to_owned(),
                    database: DEFAULT_DATABASE.to_owned(),
                    source_manifest_sha256: SCHEMA_LINEAGE_SHA256.to_owned(),
                    generated_surql_sha256: GENERATED_SURREALQL_SHA256.to_owned(),
                };
                let script = fresh_bootstrap_script(SCHEMA).map_err(|reason| {
                    SurrealStorageError::TransactionWorker(format!(
                        "HANDSHAKE_SURREAL_BOOTSTRAP_SCRIPT_INVALID: {reason}"
                    ))
                })?;
                // A crash after the definitions transaction committed but before the receipt
                // transaction did leaves the exact compiled table set with no receipt row. Only
                // that state resumes: the index statements, the post-transaction statements (all
                // OVERWRITE) and the receipt re-run, then the schema_applied resume path below
                // verifies the complete live INFO fingerprint before finalizing.
                if definitions_committed_without_receipt(&database).await? {
                    database
                        .query_bound(script.indexes_and_rest.clone(), bindings())
                        .await?;
                }
                let existing = read_context_and_state(&database).await?;
                let mut verified_observed = None;
                let outcome = match existing {
                    None => {
                        // The table/field/function transaction must commit before any index is
                        // defined: a failed first transaction (e.g. the not-empty guard) never
                        // reaches an index statement.
                        database
                            .query_bound(script.definitions, bindings())
                            .await?;
                        database.query_bound(script.indexes_and_rest, bindings()).await?;
                        let applied_state = match read_context_and_state(&database).await? {
                            Some(state) if state.is_schema_applied_current() => state,
                            Some(state) => {
                                return fail_closed(
                                    &database,
                                    format!(
                                        "HANDSHAKE_SURREAL_SCHEMA_APPLY_STATE_MISMATCH: {state:?}"
                                    ),
                                )
                                .await;
                            }
                            None => {
                                return fail_closed(
                                    &database,
                                    "HANDSHAKE_SURREAL_SCHEMA_APPLY_STATE_MISSING".to_owned(),
                                )
                                .await;
                            }
                        };
                        ensure_knowledge_schema_registry(&database).await?;
                        let observed = inspect_schema(&database).await?;
                        verify_expected_info_fingerprint(&database, &observed).await?;
                        finalize_schema_state(
                            &database,
                            &applied_state,
                            &observed.info_fingerprint_sha256,
                        )
                        .await?;
                        verified_observed = Some(observed);
                        SchemaBootstrapOutcome::InstalledFresh
                    }
                    Some(state) if state.is_schema_applied_current() => {
                        ensure_knowledge_schema_registry(&database).await?;
                        let observed = inspect_schema(&database).await?;
                        verify_expected_info_fingerprint(&database, &observed).await?;
                        finalize_schema_state(
                            &database,
                            &state,
                            &observed.info_fingerprint_sha256,
                        )
                        .await?;
                        verified_observed = Some(observed);
                        SchemaBootstrapOutcome::ResumedCurrentApply
                    }
                    Some(state) if state.is_exact_current() => {
                        ensure_knowledge_schema_registry(&database).await?;
                        SchemaBootstrapOutcome::ReusedExactCurrent
                    }
                    Some(state) if state.is_exact_pre_standalone_loom_update_current() => {
                        verified_observed =
                            Some(upgrade_pre_standalone_loom_update_current(&database, &state).await?);
                        SchemaBootstrapOutcome::UpgradedSupportedPredecessor
                    }
                    Some(state) if state.is_exact_pre_canvas_receipt_current() => {
                        verified_observed =
                            Some(upgrade_pre_canvas_receipt_current(&database, &state).await?);
                        SchemaBootstrapOutcome::UpgradedSupportedPredecessor
                    }
                    Some(state) if state.is_exact_pre_account_setup() => {
                        verified_observed = Some(upgrade_pre_account_setup(&database, &state).await?);
                        SchemaBootstrapOutcome::UpgradedSupportedPredecessor
                    }
                    Some(state) if state.is_exact_pre_mt141_current() => {
                        verified_observed =
                            Some(upgrade_pre_mt141_current(&database, &state).await?);
                        SchemaBootstrapOutcome::UpgradedSupportedPredecessor
                    }
                    Some(state) if state.is_exact_pre_mt150_current() => {
                        verified_observed =
                            Some(upgrade_pre_mt150_current(&database, &state).await?);
                        SchemaBootstrapOutcome::UpgradedSupportedPredecessor
                    }
                    Some(state) if state.is_exact_pre_mt109_current() => {
                        verified_observed = Some(upgrade_pre_mt109_current(&database, &state).await?);
                        SchemaBootstrapOutcome::UpgradedSupportedPredecessor
                    }
                    Some(state) if state.is_exact_supported_predecessor() => {
                        verified_observed =
                            Some(upgrade_supported_predecessor(&database, &state).await?);
                        SchemaBootstrapOutcome::UpgradedSupportedPredecessor
                    }
                    Some(state) if state.is_exact_pre_mt142_current() => {
                        verified_observed =
                            Some(upgrade_pre_mt142_current(&database, &state).await?);
                        SchemaBootstrapOutcome::UpgradedSupportedPredecessor
                    }
                    Some(state) if state.is_exact_pre_mt151_current() => {
                        verified_observed =
                            Some(upgrade_pre_mt151_current(&database, &state).await?);
                        SchemaBootstrapOutcome::UpgradedSupportedPredecessor
                    }
                    Some(state) if state.is_exact_pre_mt152_current() => {
                        verified_observed =
                            Some(upgrade_pre_mt152_current(&database, &state).await?);
                        SchemaBootstrapOutcome::UpgradedSupportedPredecessor
                    }
                    Some(state) => {
                        return fail_closed(
                            &database,
                            format!(
                                "HANDSHAKE_SURREAL_SCHEMA_UNSUPPORTED_LINEAGE: observed={state:?}; expected_revision={SCHEMA_REVISION}"
                            ),
                        )
                        .await;
                    }
                };

                let state = match read_context_and_state(&database).await? {
                    Some(state) if state.is_exact_current() => state,
                    Some(state) => {
                        return fail_closed(
                            &database,
                            format!(
                                "HANDSHAKE_SURREAL_SCHEMA_POST_APPLY_STATE_MISMATCH: {state:?}"
                            ),
                        )
                        .await;
                    }
                    None => {
                        return fail_closed(
                            &database,
                            "HANDSHAKE_SURREAL_SCHEMA_POST_APPLY_STATE_MISSING".to_owned(),
                        )
                        .await;
                    }
                };

                match verified_observed {
                    Some(observed) => {
                        report_from_observed(&database, state, observed, outcome)
                            .await
                    }
                    None => observe_schema(&database, state, outcome).await,
                }
            })
        })
        .await?;
    tracing::info!(
        target: "handshake_core",
        schema_bootstrap_outcome = report.outcome.as_str(),
        schema_version = %report.schema_version,
        generated_surql_sha256 = %report.generated_surql_sha256,
        info_fingerprint_sha256 = %report.info_fingerprint_sha256,
        "surreal_schema_bootstrap_complete"
    );
    Ok(report)
}

pub fn compute_generated_surql_sha256() -> String {
    sha256_hex(SCHEMA.as_bytes())
}

pub fn compute_declarative_schema_catalog_sha256() -> Result<String, String> {
    compiled_schema_catalog_entries().map(|entries| compute_catalog_hash(&entries))
}

pub fn compute_knowledge_schema_registry_seed_sha256() -> String {
    sha256_hex(KNOWLEDGE_SCHEMA_REGISTRY_SEED.as_bytes())
}

fn compiled_schema_catalog_entries() -> Result<Vec<String>, String> {
    let mut entries = BTreeSet::new();
    let mut tables = BTreeSet::new();
    let mut fields = 0usize;
    let mut indexes = 0usize;
    let mut events = 0usize;
    let mut views = 0usize;
    let mut sequences = 0usize;
    let mut accesses = 0usize;
    let mut functions = 0usize;

    for raw_line in SCHEMA.lines() {
        let line = raw_line.trim();
        let tokens = line.split_whitespace().collect::<Vec<_>>();
        let identity = match tokens.as_slice() {
            ["DEFINE", "TABLE", "OVERWRITE", name, ..] => {
                let name = name.trim_end_matches(';');
                tables.insert(name.to_owned());
                if line.contains(" TYPE NORMAL AS") {
                    views += 1;
                    insert_catalog_identity(&mut entries, format!("view:{name}"))?;
                }
                Some(format!("table:{name}"))
            }
            ["DEFINE", "FIELD", "OVERWRITE", name, "ON", "TABLE", table, ..] => {
                fields += 1;
                Some(format!(
                    "field:{}:{}",
                    table.trim_end_matches(';'),
                    name.trim_end_matches(';')
                ))
            }
            ["DEFINE", "INDEX", "OVERWRITE", name, "ON", "TABLE", table, ..] => {
                indexes += 1;
                Some(format!(
                    "index:{}:{}",
                    table.trim_end_matches(';'),
                    name.trim_end_matches(';')
                ))
            }
            ["DEFINE", "EVENT", "OVERWRITE", name, "ON", "TABLE", table, ..] => {
                events += 1;
                Some(format!(
                    "event:{}:{}",
                    table.trim_end_matches(';'),
                    name.trim_end_matches(';')
                ))
            }
            ["DEFINE", "SEQUENCE", "OVERWRITE", name, ..] => {
                sequences += 1;
                Some(format!("sequence:{}", name.trim_end_matches(';')))
            }
            ["DEFINE", "SEQUENCE", "IF", "NOT", "EXISTS", name, ..] => {
                sequences += 1;
                Some(format!("sequence:{}", name.trim_end_matches(';')))
            }
            ["DEFINE", "ACCESS", "OVERWRITE", name, "ON", "DATABASE", ..]
            | ["DEFINE", "ACCESS", "IF", "NOT", "EXISTS", name, "ON", "DATABASE", ..] => {
                accesses += 1;
                Some(format!("access:{}", name.trim_end_matches(';')))
            }
            ["DEFINE", "FUNCTION", "OVERWRITE", signature, ..] => {
                functions += 1;
                Some(format!(
                    "function:{}",
                    declarative_function_name(signature)?
                ))
            }
            ["DEFINE", "ACCESS" | "FUNCTION", ..] => {
                return Err(format!(
                    "unsupported declarative authority definition: {line}"
                ));
            }
            _ => None,
        };
        if let Some(identity) = identity {
            insert_catalog_identity(&mut entries, identity)?;
        }
    }

    let expected_tables = TABLE_NAMES
        .iter()
        .map(|name| (*name).to_owned())
        .collect::<BTreeSet<_>>();
    if tables != expected_tables {
        return Err(format!(
            "declarative table inventory differs from TABLE_NAMES: parsed={}; expected={}",
            tables.len(),
            expected_tables.len()
        ));
    }
    let observed_counts = (
        tables.len(),
        fields,
        indexes,
        events,
        views,
        sequences,
        accesses,
        functions,
    );
    let expected_counts = (
        TABLE_DEFINITION_COUNT,
        AUTHORED_FIELD_DEFINITION_COUNT,
        INDEX_DEFINITION_COUNT,
        EVENT_DEFINITION_COUNT,
        VIEW_DEFINITION_COUNT,
        SEQUENCE_DEFINITION_COUNT,
        ACCESS_DEFINITION_COUNT,
        FUNCTION_DEFINITION_COUNT,
    );
    if observed_counts != expected_counts {
        return Err(format!(
            "declarative schema catalog counts differ: observed={observed_counts:?}; expected={expected_counts:?}"
        ));
    }

    Ok(entries.into_iter().collect())
}

/// Cascade edges use the same strict declaration-token grammar as the catalog.
#[cfg(test)]
pub(super) fn workspace_cascade_edges() -> Result<Vec<(String, String, String)>, String> {
    compiled_schema_catalog_entries()?;
    let mut edges = Vec::new();
    for line in SCHEMA
        .lines()
        .filter(|line| line.contains("REFERENCE ON DELETE CASCADE"))
    {
        let tokens = line.split_whitespace().collect::<Vec<_>>();
        let ["DEFINE", "FIELD", "OVERWRITE", field, "ON", "TABLE", table, "TYPE", kind, ..] =
            tokens.as_slice()
        else {
            return Err("unsupported cascade declaration".to_owned());
        };
        let kind = kind
            .strip_prefix("option<")
            .and_then(|v| v.strip_suffix('>'))
            .unwrap_or(kind);
        let target = kind
            .strip_prefix("record<")
            .and_then(|v| v.strip_suffix('>'))
            .ok_or_else(|| "unsupported cascade target".to_owned())?;
        for name in [*field, *table, target] {
            if name.is_empty() || !name.bytes().all(|v| v.is_ascii_alphanumeric() || v == b'_') {
                return Err("unsupported cascade identifier".to_owned());
            }
        }
        edges.push((target.to_owned(), (*table).to_owned(), (*field).to_owned()));
    }
    Ok(edges)
}

fn declarative_function_name(signature: &str) -> Result<&str, String> {
    let (name, _) = signature
        .split_once('(')
        .ok_or_else(|| format!("function declaration lacks argument opener: {signature}"))?;
    if !name.starts_with("fn::") || name.len() == 4 {
        return Err(format!("invalid declarative function name: {signature}"));
    }
    Ok(name)
}

fn resource_authority_schema_blocks() -> Vec<&'static str> {
    let blocks = SCHEMA
        .split("-- MT109_AUTHORITY_BEGIN\n")
        .skip(1)
        .map(|tail| {
            tail.split_once("\n-- MT109_AUTHORITY_END")
                .expect("canonical authority block has an end marker")
                .0
        })
        .collect::<Vec<_>>();
    assert_eq!(
        blocks.len(),
        14,
        "canonical schema must contain all fourteen authority blocks"
    );
    assert!(
        blocks.iter().all(|block| !block.trim().is_empty()),
        "authority blocks must not be empty"
    );
    blocks
}

pub(super) fn resource_authority_schema_statements() -> String {
    format!(
        "{}\n{}",
        resource_authority_schema_blocks().join("\n"),
        RECORD_USER_PRODUCER_BLOCK
    )
}

/// The MT-154 authority block of the compiled SCHEMA (functions only, e.g. fn::mt154_workspace_receipt)
/// that the C4 kernel_event_ledger / Atelier / job permissions call. Bounded authority schemas append it
/// so a permission branch that reaches one of these functions evaluates instead of erroring.
pub(super) fn mt154_authority_function_block() -> &'static str {
    let start = SCHEMA
        .find(MT154_AUTHORITY_BLOCK_BEGIN)
        .expect("canonical schema carries the MT-154 authority block");
    let end = SCHEMA[start..]
        .find(MT154_AUTHORITY_BLOCK_END)
        .map(|offset| start + offset + MT154_AUTHORITY_BLOCK_END.len())
        .expect("MT-154 authority block must terminate");
    &SCHEMA[start..end]
}

fn resource_authority_upgrade_statements() -> String {
    format!(
        "{}\n{}",
        resource_authority_schema_statements(),
        MT109_LOOM_SOURCE_BACKFILL
    )
}

/// Every lineage older than the MT-109 pin receives the MT-109 authority delta AND the MT-150
/// `loom_edges` receipt binding in its own upgrade transaction, because the finalize gate pins
/// the current fingerprint (which includes both).
fn post_mt109_upgrade_statements() -> String {
    format!(
        "{}\n{}\n{}\n{}\n{}\n{}",
        resource_authority_upgrade_statements(),
        MT150_LOOM_EDGE_RECEIPT_UPGRADE_STATEMENTS,
        mt141_upgrade_statements(),
        mt120_document_upgrade_statements(),
        standalone_loom_update_upgrade_statement(),
        mt120_loom_receipt_upgrade_statement(),
    )
}

/// Reuse the declarative receipt function verbatim so fresh installs and every supported
/// predecessor carry the same permission predicate.
fn mt120_loom_receipt_upgrade_statement() -> String {
    const START: &str = "DEFINE FUNCTION OVERWRITE fn::mt120_loom_receipt(";
    const END: &str = "\nDEFINE FUNCTION OVERWRITE fn::mt120_document_receipt";
    let start = SCHEMA
        .find(START)
        .expect("current schema defines mt120 loom receipt");
    let receipt = &SCHEMA[start..];
    let end = receipt
        .find(END)
        .expect("mt120 loom receipt precedes document receipt");
    receipt[..end].to_owned()
}

/// Reuse the complete LoomBlock declaration so the current standalone-owner
/// update permission applies identically to fresh and upgraded stores.
fn standalone_loom_update_upgrade_statement() -> String {
    let (table_start, table_end) = schema_table_definition_bounds(SCHEMA, "loom_blocks");
    let (loom_event_start, loom_event_end) =
        schema_event_definition_bounds(SCHEMA, "mt109_loom_source_integrity");
    let (workspace_event_start, workspace_event_end) =
        schema_event_definition_bounds_until_next_table(
            SCHEMA,
            "mt109_workspace_reconciliation_queue",
        );
    format!(
        "{}\n{}\n{}",
        &SCHEMA[table_start..table_end],
        &SCHEMA[loom_event_start..loom_event_end],
        &SCHEMA[workspace_event_start..workspace_event_end],
    )
}

const MT109_LOOM_SOURCE_BACKFILL: &str = r#"
LET $documents = SELECT id, rich_document_id, workspace_id FROM knowledge_rich_documents;
IF array::len($documents) > 0 {
    FOR $document IN $documents {
        LET $blocks = SELECT * FROM loom_blocks
            WHERE id = type::record('loom_blocks', $document.rich_document_id) LIMIT 1;
        IF array::len($blocks) > 0 {
            LET $block = $blocks[0];
            IF $block.workspace_id != $document.workspace_id OR $block.content_type != 'note'
                OR ($block.source_rich_document_id != NONE AND $block.source_rich_document_id != $document.id) {
                THROW 'HSK-MT109-LOOM-SOURCE-IDENTITY';
            };
            UPDATE type::record('loom_blocks', $document.rich_document_id)
                SET source_rich_document_id = $document.id RETURN NONE;
        };
    };
};
"#;

fn resource_authority_core_block() -> &'static str {
    let blocks = resource_authority_schema_blocks()
        .into_iter()
        .filter(|block| {
            block
                .trim_start()
                .starts_with("DEFINE TABLE OVERWRITE local_accounts ")
        })
        .collect::<Vec<_>>();
    assert_eq!(
        blocks.len(),
        1,
        "exactly one canonical authority core block"
    );
    assert!(blocks[0].contains("DEFINE ACCESS IF NOT EXISTS authenticated_session ON DATABASE"));
    blocks[0]
}

fn authority_catalog_names(kind: &str) -> BTreeSet<String> {
    resource_authority_core_block()
        .lines()
        .filter_map(|line| {
            let tokens = line.split_ascii_whitespace().collect::<Vec<_>>();
            match (kind, tokens.as_slice()) {
                (
                    "access",
                    ["DEFINE", "ACCESS", "IF", "NOT", "EXISTS", name, "ON", "DATABASE", ..],
                ) => Some((*name).to_owned()),
                ("function", ["DEFINE", "FUNCTION", "OVERWRITE", signature, ..]) => Some(
                    declarative_function_name(signature)
                        .expect("canonical function signature")
                        .trim_start_matches("fn::")
                        .to_owned(),
                ),
                _ => None,
            }
        })
        .collect()
}

fn insert_catalog_identity(entries: &mut BTreeSet<String>, identity: String) -> Result<(), String> {
    if entries.insert(identity.clone()) {
        Ok(())
    } else {
        Err(format!("duplicate declarative schema identity: {identity}"))
    }
}

fn compute_catalog_hash(entries: &[String]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(DECLARATIVE_SCHEMA_CATALOG_DOMAIN);
    let mut sorted = entries.to_vec();
    sorted.sort();
    for identity in sorted {
        hasher.update((identity.len() as u32).to_be_bytes());
        hasher.update(identity.as_bytes());
    }
    format!("{:x}", hasher.finalize())
}

fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

#[cfg(test)]
fn generated_collection_subtype_field_count(schema: &str) -> usize {
    schema
        .lines()
        .filter(|line| line.starts_with("DEFINE FIELD OVERWRITE "))
        .map(|line| line.matches("array<").count() + line.matches("set<").count())
        .sum()
}

async fn verify_compiled_manifest(
    database: &SurrealAdminContext<'_>,
) -> Result<(), SurrealStorageError> {
    let catalog = match compute_declarative_schema_catalog_sha256() {
        Ok(catalog) => catalog,
        Err(reason) => {
            return fail_closed(
                database,
                format!("HANDSHAKE_SURREAL_DECLARATIVE_CATALOG_INVALID: {reason}"),
            )
            .await;
        }
    };
    let generated = compute_generated_surql_sha256();
    let registry_seed = compute_knowledge_schema_registry_seed_sha256();
    let predecessor_registry = match expected_predecessor_registry_metadata() {
        Ok(rows) => compute_predecessor_registry_hash(&rows),
        Err(reason) => {
            return fail_closed(
                database,
                format!("HANDSHAKE_SURREAL_PREDECESSOR_REGISTRY_MANIFEST_INVALID: {reason}"),
            )
            .await;
        }
    };
    if catalog != DECLARATIVE_SCHEMA_CATALOG_SHA256
        || generated != GENERATED_SURREALQL_SHA256
        || registry_seed != KNOWLEDGE_SCHEMA_REGISTRY_SEED_SHA256
        || predecessor_registry != PREDECESSOR_KNOWLEDGE_REGISTRY_SHA256
    {
        return fail_closed(
            database,
            format!(
                "HANDSHAKE_SURREAL_SCHEMA_COMPILED_MANIFEST_DRIFT: catalog={catalog}; generated={generated}; knowledge_registry_seed={registry_seed}; predecessor_registry={predecessor_registry}"
            ),
        )
        .await;
    }
    Ok(())
}

async fn read_context_and_state(
    database: &SurrealAdminContext<'_>,
) -> Result<Option<SchemaState>, SurrealStorageError> {
    let mut response = database
        .query("RETURN session::ns(); RETURN session::db(); INFO FOR DB STRUCTURE;")
        .await?;
    let namespace: Option<String> = response.take(0)?;
    let namespace = match namespace {
        Some(namespace) => namespace,
        None => {
            return fail_closed(
                database,
                "HANDSHAKE_SURREAL_SCHEMA_CONTEXT_NAMESPACE_MISSING".to_owned(),
            )
            .await;
        }
    };
    let selected_database: Option<String> = response.take(1)?;
    let selected_database = match selected_database {
        Some(selected_database) => selected_database,
        None => {
            return fail_closed(
                database,
                "HANDSHAKE_SURREAL_SCHEMA_CONTEXT_DATABASE_MISSING".to_owned(),
            )
            .await;
        }
    };
    if namespace != DEFAULT_NAMESPACE || selected_database != DEFAULT_DATABASE {
        return Err(SurrealStorageError::ContextMismatch {
            expected_namespace: DEFAULT_NAMESPACE.to_owned(),
            expected_database: DEFAULT_DATABASE.to_owned(),
            actual_namespace: namespace,
            actual_database: selected_database,
        });
    }

    let database_info: SurrealValueData = response.take(2)?;
    let mut nonempty_categories = Vec::new();
    for category in DATABASE_STRUCTURE_CATEGORIES {
        let count = match array_len(&database_info, category) {
            Ok(count) => count,
            Err(reason) => return fail_closed(database, reason).await,
        };
        if count != 0 {
            nonempty_categories.push(format!("{category}={count}"));
        }
    }
    let table_names = match parse_named_array(&database_info, "tables") {
        Ok(names) => names,
        Err(reason) => return fail_closed(database, reason).await,
    };
    if !table_names.iter().any(|name| name == BOOTSTRAP_STATE_TABLE) {
        if nonempty_categories.is_empty() {
            return Ok(None);
        }
        return fail_closed(
            database,
            format!(
                "HANDSHAKE_SURREAL_SCHEMA_DATABASE_NOT_EMPTY: missing_state_table; {}",
                nonempty_categories.join(",")
            ),
        )
        .await;
    }

    let mut state_response = database
        .query(format!("SELECT * FROM ONLY {BOOTSTRAP_STATE_ID};"))
        .await?;
    let state: Option<SchemaState> = state_response.take(0)?;
    match state {
        Some(state) => Ok(Some(state)),
        None => {
            fail_closed(
                database,
                "HANDSHAKE_SURREAL_SCHEMA_STATE_ROW_MISSING".to_owned(),
            )
            .await
        }
    }
}

async fn upgrade_supported_predecessor(
    database: &SurrealAdminContext<'_>,
    predecessor_state: &SchemaState,
) -> Result<ObservedSchema, SurrealStorageError> {
    if !predecessor_state.is_exact_supported_predecessor() {
        return fail_closed(
            database,
            "HANDSHAKE_SURREAL_PREDECESSOR_UPGRADE_PRECONDITION_FAILED".to_owned(),
        )
        .await;
    }
    ensure_supported_predecessor_registry(database).await?;
    let predecessor_observed = read_schema_catalog(database).await?;
    if predecessor_observed.info_fingerprint_sha256 != PREDECESSOR_SCHEMA_INFO_SHA256 {
        return fail_closed(
            database,
            format!(
                "HANDSHAKE_SURREAL_PREDECESSOR_INFO_FINGERPRINT_MISMATCH: expected={PREDECESSOR_SCHEMA_INFO_SHA256}; observed={}",
                predecessor_observed.info_fingerprint_sha256
            ),
        )
        .await;
    }

    database
        .query_bound(
            r#"
BEGIN TRANSACTION;
LET $current = SELECT * FROM ONLY handshake_schema_state:primary;
IF $current = NONE
    OR $current.version != $schema_version
    OR $current.revision != $predecessor_revision
    OR $current.target_revision != $predecessor_revision
    OR $current.namespace != $namespace
    OR $current.database != $database
    OR $current.source_manifest_sha256 != $source_manifest_sha256
    OR $current.generated_surql_sha256 != $predecessor_generated_surql_sha256
    OR $current.info_fingerprint_sha256 != $predecessor_info_fingerprint_sha256
    OR $current.apply_state != 'complete'
{
    THROW 'HANDSHAKE_SURREAL_PREDECESSOR_UPGRADE_STATE_CHANGED';
};
DEFINE FIELD OVERWRITE schema_source ON TABLE knowledge_schema_registry TYPE string;
UPDATE knowledge_schema_registry SET schema_source = $schema_source;
REMOVE FIELD migration_file ON TABLE knowledge_schema_registry;
CREATE ONLY knowledge_schema_registry:rich_document_loom_projection_0343_state CONTENT {
    family_key: 'rich_document_loom_projection_0343_state',
    table_name: 'knowledge_rich_document_loom_projection_0343_state',
    record_family: 'Support',
    authority_class: 'support',
    schema_source: $schema_source,
    wp_id: 'WP-KERNEL-012',
    mt_id: 'MT-032'
};
DEFINE TABLE OVERWRITE knowledge_rich_document_title_anchors SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE anchor_key ON TABLE knowledge_rich_document_title_anchors TYPE string ASSERT $value = record::id($this.id);
DEFINE FIELD OVERWRITE workspace_id ON TABLE knowledge_rich_document_title_anchors TYPE record<workspaces> ASSERT record::exists($value) REFERENCE ON DELETE CASCADE;
DEFINE FIELD OVERWRITE title_key ON TABLE knowledge_rich_document_title_anchors TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE last_rich_document_id ON TABLE knowledge_rich_document_title_anchors TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE claim_nonce ON TABLE knowledge_rich_document_title_anchors TYPE string ASSERT string::trim($value) != '';
DEFINE FIELD OVERWRITE created_at ON TABLE knowledge_rich_document_title_anchors TYPE datetime DEFAULT time::now();
DEFINE FIELD OVERWRITE updated_at ON TABLE knowledge_rich_document_title_anchors TYPE datetime DEFAULT time::now();
DEFINE INDEX OVERWRITE pk_knowledge_rich_document_title_anchors ON TABLE knowledge_rich_document_title_anchors FIELDS anchor_key UNIQUE;
DEFINE INDEX OVERWRITE uq_knowledge_rich_document_title_anchors_identity ON TABLE knowledge_rich_document_title_anchors FIELDS workspace_id, title_key UNIQUE;
CREATE ONLY knowledge_schema_registry:rich_document_title_anchors CONTENT {
    family_key: 'rich_document_title_anchors',
    table_name: 'knowledge_rich_document_title_anchors',
    record_family: 'Support',
    authority_class: 'support',
    schema_source: $schema_source,
    wp_id: 'WP-KERNEL-012',
    mt_id: 'MT-142'
};
UPDATE ONLY handshake_schema_state:primary SET
    revision = $schema_revision, target_revision = $schema_revision,
    generated_surql_sha256 = $generated_surql_sha256,
    info_fingerprint_sha256 = $pending_info_fingerprint_sha256,
    apply_state = 'schema_applied',
    updated_at = time::now();
COMMIT TRANSACTION;
"#.replace("UPDATE ONLY handshake_schema_state:primary SET", &format!("{}\nUPDATE ONLY handshake_schema_state:primary SET", post_mt109_upgrade_statements())).as_str(),
            PredecessorUpgradeBindings {
                schema_version: SCHEMA_VERSION.to_owned(),
                schema_revision: SCHEMA_REVISION,
                predecessor_revision: PRE_ACCOUNT_SETUP_REVISION,
                namespace: DEFAULT_NAMESPACE.to_owned(),
                database: DEFAULT_DATABASE.to_owned(),
                source_manifest_sha256: SCHEMA_LINEAGE_SHA256.to_owned(),
                predecessor_generated_surql_sha256: PREDECESSOR_GENERATED_SURREALQL_SHA256
                    .to_owned(),
                predecessor_info_fingerprint_sha256: PREDECESSOR_SCHEMA_INFO_SHA256.to_owned(),
                generated_surql_sha256: GENERATED_SURREALQL_SHA256.to_owned(),
                pending_info_fingerprint_sha256: PENDING_SCHEMA_INFO_SHA256.to_owned(),
                schema_source: "storage/surreal/schema.surql".to_owned(),
            },
        )
        .await?;

    let upgraded = match read_context_and_state(database).await? {
        Some(state) if state.is_schema_applied_current() => state,
        Some(state) => {
            return fail_closed(
                database,
                format!("HANDSHAKE_SURREAL_PREDECESSOR_UPGRADE_STATE_MISMATCH: {state:?}"),
            )
            .await;
        }
        None => {
            return fail_closed(
                database,
                "HANDSHAKE_SURREAL_PREDECESSOR_UPGRADE_STATE_MISSING".to_owned(),
            )
            .await;
        }
    };
    ensure_knowledge_schema_registry(database).await?;
    let observed = inspect_schema(database).await?;
    verify_expected_info_fingerprint(database, &observed).await?;
    finalize_schema_state(database, &upgraded, &observed.info_fingerprint_sha256).await?;
    match read_context_and_state(database).await? {
        Some(state) if state.is_exact_current() => Ok(observed),
        Some(state) => {
            fail_closed(
                database,
                format!("HANDSHAKE_SURREAL_PREDECESSOR_UPGRADE_FINAL_STATE_MISMATCH: {state:?}"),
            )
            .await
        }
        None => {
            fail_closed(
                database,
                "HANDSHAKE_SURREAL_PREDECESSOR_UPGRADE_FINAL_STATE_MISSING".to_owned(),
            )
            .await
        }
    }
}

/// MT-151 phase one for both allowlisted delta lineages: materialises `journal_key` on
/// existing journal rows in its own transaction, guarded by the exact predecessor state so it
/// never runs against any other lineage.
async fn materialise_mt151_journal_key(
    database: &SurrealAdminContext<'_>,
    predecessor_generated_surql_sha256: &str,
    predecessor_info_fingerprint_sha256: &str,
) -> Result<(), SurrealStorageError> {
    let materialise = format!(
        "BEGIN TRANSACTION;\n\
LET $current = SELECT * FROM ONLY handshake_schema_state:primary;\n\
IF $current = NONE\n\
    OR $current.version != $schema_version\n\
    OR $current.revision != $predecessor_revision\n\
    OR $current.target_revision != $predecessor_revision\n\
    OR $current.namespace != $namespace\n\
    OR $current.database != $database\n\
    OR $current.source_manifest_sha256 != $source_manifest_sha256\n\
    OR $current.generated_surql_sha256 != $predecessor_generated_surql_sha256\n\
    OR $current.info_fingerprint_sha256 != $predecessor_info_fingerprint_sha256\n\
    OR $current.apply_state != 'complete'\n\
{{\n\
    THROW 'HANDSHAKE_SURREAL_MT151_MATERIALISE_STATE_CHANGED';\n\
}};\n\
{MT151_JOURNAL_KEY_MATERIALISE_STATEMENTS}\
COMMIT TRANSACTION;\n"
    );
    database
        .query_bound(
            materialise.as_str(),
            PredecessorUpgradeBindings {
                schema_version: SCHEMA_VERSION.to_owned(),
                schema_revision: SCHEMA_REVISION,
                predecessor_revision: PRE_ACCOUNT_SETUP_REVISION,
                namespace: DEFAULT_NAMESPACE.to_owned(),
                database: DEFAULT_DATABASE.to_owned(),
                source_manifest_sha256: SCHEMA_LINEAGE_SHA256.to_owned(),
                predecessor_generated_surql_sha256: predecessor_generated_surql_sha256.to_owned(),
                predecessor_info_fingerprint_sha256: predecessor_info_fingerprint_sha256.to_owned(),
                generated_surql_sha256: GENERATED_SURREALQL_SHA256.to_owned(),
                pending_info_fingerprint_sha256: PENDING_SCHEMA_INFO_SHA256.to_owned(),
                schema_source: "storage/surreal/schema.surql".to_owned(),
            },
        )
        .await?;
    Ok(())
}

#[derive(SurrealValue)]
struct FolderSiblingKeyRow {
    id: RecordId,
    workspace_id: RecordId,
    parent_folder_id: Option<RecordId>,
    name: String,
    /// Selected only because the engine requires every ORDER BY idiom in the projection.
    #[allow(dead_code)]
    created_at: Datetime,
}

#[derive(SurrealValue)]
struct FolderSiblingKeyWrite {
    record: RecordId,
    sibling_key: String,
}

#[derive(SurrealValue)]
struct FolderSiblingKeyWrites {
    writes: Vec<FolderSiblingKeyWrite>,
}

fn record_id_key(record: &RecordId) -> String {
    match &record.key {
        RecordIdKey::String(value) => value.clone(),
        other => format!("{other:?}"),
    }
}

/// MT-152 phase one for every allowlisted predecessor lineage: defines `loom_folders.sibling_key`
/// in its own state-guarded transaction, then backfills every existing folder row. Rows are
/// visited in `(created_at, id)` order; the first holder of a key keeps it and each later
/// duplicate gets `<key>#dup<n>` (the visible `name` is never touched), with one structured
/// warning per collision naming every folder id, so the UNIQUE index built in phase two always
/// succeeds and the collision is visible rather than fatal. Idempotent: re-running after a crash
/// recomputes the same keys.
async fn materialise_mt152_folder_sibling_key(
    database: &SurrealAdminContext<'_>,
    predecessor_generated_surql_sha256: &str,
    predecessor_info_fingerprint_sha256: &str,
) -> Result<(), SurrealStorageError> {
    let define = format!(
        "BEGIN TRANSACTION;\n\
LET $current = SELECT * FROM ONLY handshake_schema_state:primary;\n\
IF $current = NONE\n\
    OR $current.version != $schema_version\n\
    OR $current.revision != $predecessor_revision\n\
    OR $current.target_revision != $predecessor_revision\n\
    OR $current.namespace != $namespace\n\
    OR $current.database != $database\n\
    OR $current.source_manifest_sha256 != $source_manifest_sha256\n\
    OR $current.generated_surql_sha256 != $predecessor_generated_surql_sha256\n\
    OR $current.info_fingerprint_sha256 != $predecessor_info_fingerprint_sha256\n\
    OR $current.apply_state != 'complete'\n\
{{\n\
    THROW 'HANDSHAKE_SURREAL_MT152_FOLDER_SIBLING_KEY_STATE_CHANGED';\n\
}};\n\
{MT152_LOOM_FOLDER_SIBLING_KEY_FIELD_STATEMENTS}\
COMMIT TRANSACTION;\n"
    );
    database
        .query_bound(
            define.as_str(),
            PredecessorUpgradeBindings {
                schema_version: SCHEMA_VERSION.to_owned(),
                schema_revision: SCHEMA_REVISION,
                predecessor_revision: PRE_ACCOUNT_SETUP_REVISION,
                namespace: DEFAULT_NAMESPACE.to_owned(),
                database: DEFAULT_DATABASE.to_owned(),
                source_manifest_sha256: SCHEMA_LINEAGE_SHA256.to_owned(),
                predecessor_generated_surql_sha256: predecessor_generated_surql_sha256.to_owned(),
                predecessor_info_fingerprint_sha256: predecessor_info_fingerprint_sha256.to_owned(),
                generated_surql_sha256: GENERATED_SURREALQL_SHA256.to_owned(),
                pending_info_fingerprint_sha256: PENDING_SCHEMA_INFO_SHA256.to_owned(),
                schema_source: "storage/surreal/schema.surql".to_owned(),
            },
        )
        .await?;
    backfill_mt152_folder_sibling_keys(database).await
}

async fn backfill_mt152_folder_sibling_keys(
    database: &SurrealAdminContext<'_>,
) -> Result<(), SurrealStorageError> {
    let mut response = database
        .query(
            "SELECT id, workspace_id, parent_folder_id, name, created_at FROM loom_folders \
             ORDER BY created_at ASC, id ASC;",
        )
        .await?;
    let rows: Vec<FolderSiblingKeyRow> = response.take(0)?;
    let mut holders: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut writes = Vec::with_capacity(rows.len());
    for row in rows {
        let workspace_id = record_id_key(&row.workspace_id);
        let parent_folder_id = row.parent_folder_id.as_ref().map(record_id_key);
        let base = super::loom_store::loom_folder_sibling_key(
            &workspace_id,
            parent_folder_id.as_deref(),
            &row.name,
        );
        let folder_id = record_id_key(&row.id);
        let seen = holders.entry(base.clone()).or_default();
        let sibling_key = if seen.is_empty() {
            base
        } else {
            format!("{base}#dup{}", seen.len())
        };
        seen.push(folder_id);
        writes.push(FolderSiblingKeyWrite {
            record: row.id,
            sibling_key,
        });
    }
    for (sibling_key, folder_ids) in holders.iter().filter(|(_, ids)| ids.len() > 1) {
        tracing::warn!(
            sibling_key = %sibling_key,
            folder_ids = ?folder_ids,
            kept = %folder_ids[0],
            "HANDSHAKE_SURREAL_LOOM_FOLDER_SIBLING_NAME_COLLISION: pre-existing folders share one sibling name; later duplicates keep their visible name and carry a '#dup<n>' sibling_key suffix (MT-152)"
        );
    }
    if writes.is_empty() {
        return Ok(());
    }
    database
        .query_bound(
            "BEGIN TRANSACTION; \
             FOR $write IN $writes { UPDATE $write.record SET sibling_key = $write.sibling_key RETURN NONE; }; \
             COMMIT TRANSACTION;",
            FolderSiblingKeyWrites { writes },
        )
        .await?;
    Ok(())
}

/// MT-142: upgrades an exact pre-MT-142 current store in place by adding only the
/// `knowledge_rich_document_title_anchors` table and its registry row inside one transaction
/// guarded by the exact prior state, then finalizes through the same fingerprint gate as every
/// other lineage. Application records are untouched. MT-151: the same transaction also applies
/// the MT-151 statements, because the finalize gate pins the current fingerprint.
async fn upgrade_pre_mt142_current(
    database: &SurrealAdminContext<'_>,
    previous_state: &SchemaState,
) -> Result<ObservedSchema, SurrealStorageError> {
    if !previous_state.is_exact_pre_mt142_current() {
        return fail_closed(
            database,
            "HANDSHAKE_SURREAL_PRE_MT142_UPGRADE_PRECONDITION_FAILED".to_owned(),
        )
        .await;
    }
    materialise_mt151_journal_key(
        database,
        PRE_MT142_GENERATED_SURREALQL_SHA256,
        PRE_MT142_SCHEMA_INFO_SHA256,
    )
    .await?;
    materialise_mt152_folder_sibling_key(
        database,
        PRE_MT142_GENERATED_SURREALQL_SHA256,
        PRE_MT142_SCHEMA_INFO_SHA256,
    )
    .await?;
    let authority_delta = post_mt109_upgrade_statements();
    let upgrade = format!(
        "BEGIN TRANSACTION;\n\
LET $current = SELECT * FROM ONLY handshake_schema_state:primary;\n\
IF $current = NONE\n\
    OR $current.version != $schema_version\n\
    OR $current.revision != $predecessor_revision\n\
    OR $current.target_revision != $predecessor_revision\n\
    OR $current.namespace != $namespace\n\
    OR $current.database != $database\n\
    OR $current.source_manifest_sha256 != $source_manifest_sha256\n\
    OR $current.generated_surql_sha256 != $predecessor_generated_surql_sha256\n\
    OR $current.info_fingerprint_sha256 != $predecessor_info_fingerprint_sha256\n\
    OR $current.apply_state != 'complete'\n\
{{\n\
    THROW 'HANDSHAKE_SURREAL_PRE_MT142_UPGRADE_STATE_CHANGED';\n\
}};\n\
{MT142_TITLE_ANCHOR_UPGRADE_STATEMENTS}\
{MT151_JOURNAL_KEY_AND_GRAPH_ANCHOR_UPGRADE_STATEMENTS}\
{MT152_FEMS_WRITE_ANCHOR_UPGRADE_STATEMENTS}\
{MT152_LOOM_FOLDER_SIBLING_KEY_INDEX_STATEMENTS}\
{authority_delta}\n\
UPDATE ONLY handshake_schema_state:primary SET\n\
    revision = $schema_revision, target_revision = $schema_revision,
    generated_surql_sha256 = $generated_surql_sha256,\n\
    info_fingerprint_sha256 = $pending_info_fingerprint_sha256,\n\
    apply_state = 'schema_applied',\n\
    updated_at = time::now();\n\
COMMIT TRANSACTION;\n"
    );
    database
        .query_bound(
            upgrade.as_str(),
            PredecessorUpgradeBindings {
                schema_version: SCHEMA_VERSION.to_owned(),
                schema_revision: SCHEMA_REVISION,
                predecessor_revision: PRE_ACCOUNT_SETUP_REVISION,
                namespace: DEFAULT_NAMESPACE.to_owned(),
                database: DEFAULT_DATABASE.to_owned(),
                source_manifest_sha256: SCHEMA_LINEAGE_SHA256.to_owned(),
                predecessor_generated_surql_sha256: PRE_MT142_GENERATED_SURREALQL_SHA256.to_owned(),
                predecessor_info_fingerprint_sha256: PRE_MT142_SCHEMA_INFO_SHA256.to_owned(),
                generated_surql_sha256: GENERATED_SURREALQL_SHA256.to_owned(),
                pending_info_fingerprint_sha256: PENDING_SCHEMA_INFO_SHA256.to_owned(),
                schema_source: "storage/surreal/schema.surql".to_owned(),
            },
        )
        .await?;

    let upgraded = match read_context_and_state(database).await? {
        Some(state) if state.is_schema_applied_current() => state,
        Some(state) => {
            return fail_closed(
                database,
                format!("HANDSHAKE_SURREAL_PRE_MT142_UPGRADE_STATE_MISMATCH: {state:?}"),
            )
            .await;
        }
        None => {
            return fail_closed(
                database,
                "HANDSHAKE_SURREAL_PRE_MT142_UPGRADE_STATE_MISSING".to_owned(),
            )
            .await;
        }
    };
    ensure_knowledge_schema_registry(database).await?;
    let observed = inspect_schema(database).await?;
    verify_expected_info_fingerprint(database, &observed).await?;
    finalize_schema_state(database, &upgraded, &observed.info_fingerprint_sha256).await?;
    match read_context_and_state(database).await? {
        Some(state) if state.is_exact_current() => Ok(observed),
        Some(state) => {
            fail_closed(
                database,
                format!("HANDSHAKE_SURREAL_PRE_MT142_UPGRADE_FINAL_STATE_MISMATCH: {state:?}"),
            )
            .await
        }
        None => {
            fail_closed(
                database,
                "HANDSHAKE_SURREAL_PRE_MT142_UPGRADE_FINAL_STATE_MISSING".to_owned(),
            )
            .await
        }
    }
}

/// MT-151: upgrades an exact MT-142 current store in place by adding `loom_blocks.journal_key`
/// with its UNIQUE index (materialising the key on existing journal rows first) and the
/// `storage_graph_anchors` table inside one transaction guarded by the exact prior state, then
/// finalizes through the same fingerprint gate as every other lineage. Every other application
/// record is untouched.
async fn upgrade_pre_mt151_current(
    database: &SurrealAdminContext<'_>,
    previous_state: &SchemaState,
) -> Result<ObservedSchema, SurrealStorageError> {
    if !previous_state.is_exact_pre_mt151_current() {
        return fail_closed(
            database,
            "HANDSHAKE_SURREAL_PRE_MT151_UPGRADE_PRECONDITION_FAILED".to_owned(),
        )
        .await;
    }
    materialise_mt151_journal_key(
        database,
        PRE_MT151_GENERATED_SURREALQL_SHA256,
        PRE_MT151_SCHEMA_INFO_SHA256,
    )
    .await?;
    materialise_mt152_folder_sibling_key(
        database,
        PRE_MT151_GENERATED_SURREALQL_SHA256,
        PRE_MT151_SCHEMA_INFO_SHA256,
    )
    .await?;
    let authority_delta = post_mt109_upgrade_statements();
    let upgrade = format!(
        "BEGIN TRANSACTION;\n\
LET $current = SELECT * FROM ONLY handshake_schema_state:primary;\n\
IF $current = NONE\n\
    OR $current.version != $schema_version\n\
    OR $current.revision != $predecessor_revision\n\
    OR $current.target_revision != $predecessor_revision\n\
    OR $current.namespace != $namespace\n\
    OR $current.database != $database\n\
    OR $current.source_manifest_sha256 != $source_manifest_sha256\n\
    OR $current.generated_surql_sha256 != $predecessor_generated_surql_sha256\n\
    OR $current.info_fingerprint_sha256 != $predecessor_info_fingerprint_sha256\n\
    OR $current.apply_state != 'complete'\n\
{{\n\
    THROW 'HANDSHAKE_SURREAL_PRE_MT151_UPGRADE_STATE_CHANGED';\n\
}};\n\
{MT151_JOURNAL_KEY_AND_GRAPH_ANCHOR_UPGRADE_STATEMENTS}\
{MT152_FEMS_WRITE_ANCHOR_UPGRADE_STATEMENTS}\
{MT152_LOOM_FOLDER_SIBLING_KEY_INDEX_STATEMENTS}\
{authority_delta}\n\
UPDATE ONLY handshake_schema_state:primary SET\n\
    revision = $schema_revision, target_revision = $schema_revision,
    generated_surql_sha256 = $generated_surql_sha256,\n\
    info_fingerprint_sha256 = $pending_info_fingerprint_sha256,\n\
    apply_state = 'schema_applied',\n\
    updated_at = time::now();\n\
COMMIT TRANSACTION;\n"
    );
    database
        .query_bound(
            upgrade.as_str(),
            PredecessorUpgradeBindings {
                schema_version: SCHEMA_VERSION.to_owned(),
                schema_revision: SCHEMA_REVISION,
                predecessor_revision: PRE_ACCOUNT_SETUP_REVISION,
                namespace: DEFAULT_NAMESPACE.to_owned(),
                database: DEFAULT_DATABASE.to_owned(),
                source_manifest_sha256: SCHEMA_LINEAGE_SHA256.to_owned(),
                predecessor_generated_surql_sha256: PRE_MT151_GENERATED_SURREALQL_SHA256.to_owned(),
                predecessor_info_fingerprint_sha256: PRE_MT151_SCHEMA_INFO_SHA256.to_owned(),
                generated_surql_sha256: GENERATED_SURREALQL_SHA256.to_owned(),
                pending_info_fingerprint_sha256: PENDING_SCHEMA_INFO_SHA256.to_owned(),
                schema_source: "storage/surreal/schema.surql".to_owned(),
            },
        )
        .await?;

    let upgraded = match read_context_and_state(database).await? {
        Some(state) if state.is_schema_applied_current() => state,
        Some(state) => {
            return fail_closed(
                database,
                format!("HANDSHAKE_SURREAL_PRE_MT151_UPGRADE_STATE_MISMATCH: {state:?}"),
            )
            .await;
        }
        None => {
            return fail_closed(
                database,
                "HANDSHAKE_SURREAL_PRE_MT151_UPGRADE_STATE_MISSING".to_owned(),
            )
            .await;
        }
    };
    ensure_knowledge_schema_registry(database).await?;
    let observed = inspect_schema(database).await?;
    verify_expected_info_fingerprint(database, &observed).await?;
    finalize_schema_state(database, &upgraded, &observed.info_fingerprint_sha256).await?;
    match read_context_and_state(database).await? {
        Some(state) if state.is_exact_current() => Ok(observed),
        Some(state) => {
            fail_closed(
                database,
                format!("HANDSHAKE_SURREAL_PRE_MT151_UPGRADE_FINAL_STATE_MISMATCH: {state:?}"),
            )
            .await
        }
        None => {
            fail_closed(
                database,
                "HANDSHAKE_SURREAL_PRE_MT151_UPGRADE_FINAL_STATE_MISSING".to_owned(),
            )
            .await
        }
    }
}

/// MT-152: upgrades an exact MT-151 current store in place by adding the
/// `fems_workspace_write_anchors` table inside one transaction guarded by the exact prior
/// state, then finalizes through the same fingerprint gate as every other lineage. Every
/// application record is untouched.
async fn upgrade_pre_mt152_current(
    database: &SurrealAdminContext<'_>,
    previous_state: &SchemaState,
) -> Result<ObservedSchema, SurrealStorageError> {
    if !previous_state.is_exact_pre_mt152_current() {
        return fail_closed(
            database,
            "HANDSHAKE_SURREAL_PRE_MT152_UPGRADE_PRECONDITION_FAILED".to_owned(),
        )
        .await;
    }
    materialise_mt152_folder_sibling_key(
        database,
        PRE_MT152_GENERATED_SURREALQL_SHA256,
        PRE_MT152_SCHEMA_INFO_SHA256,
    )
    .await?;
    let authority_delta = post_mt109_upgrade_statements();
    let upgrade = format!(
        "BEGIN TRANSACTION;\n\
LET $current = SELECT * FROM ONLY handshake_schema_state:primary;\n\
IF $current = NONE\n\
    OR $current.version != $schema_version\n\
    OR $current.revision != $predecessor_revision\n\
    OR $current.target_revision != $predecessor_revision\n\
    OR $current.namespace != $namespace\n\
    OR $current.database != $database\n\
    OR $current.source_manifest_sha256 != $source_manifest_sha256\n\
    OR $current.generated_surql_sha256 != $predecessor_generated_surql_sha256\n\
    OR $current.info_fingerprint_sha256 != $predecessor_info_fingerprint_sha256\n\
    OR $current.apply_state != 'complete'\n\
{{\n\
    THROW 'HANDSHAKE_SURREAL_PRE_MT152_UPGRADE_STATE_CHANGED';\n\
}};\n\
{MT152_FEMS_WRITE_ANCHOR_UPGRADE_STATEMENTS}\
{MT152_LOOM_FOLDER_SIBLING_KEY_INDEX_STATEMENTS}\
{authority_delta}\n\
UPDATE ONLY handshake_schema_state:primary SET\n\
    revision = $schema_revision, target_revision = $schema_revision,
    generated_surql_sha256 = $generated_surql_sha256,\n\
    info_fingerprint_sha256 = $pending_info_fingerprint_sha256,\n\
    apply_state = 'schema_applied',\n\
    updated_at = time::now();\n\
COMMIT TRANSACTION;\n"
    );
    database
        .query_bound(
            upgrade.as_str(),
            PredecessorUpgradeBindings {
                schema_version: SCHEMA_VERSION.to_owned(),
                schema_revision: SCHEMA_REVISION,
                predecessor_revision: PRE_ACCOUNT_SETUP_REVISION,
                namespace: DEFAULT_NAMESPACE.to_owned(),
                database: DEFAULT_DATABASE.to_owned(),
                source_manifest_sha256: SCHEMA_LINEAGE_SHA256.to_owned(),
                predecessor_generated_surql_sha256: PRE_MT152_GENERATED_SURREALQL_SHA256.to_owned(),
                predecessor_info_fingerprint_sha256: PRE_MT152_SCHEMA_INFO_SHA256.to_owned(),
                generated_surql_sha256: GENERATED_SURREALQL_SHA256.to_owned(),
                pending_info_fingerprint_sha256: PENDING_SCHEMA_INFO_SHA256.to_owned(),
                schema_source: "storage/surreal/schema.surql".to_owned(),
            },
        )
        .await?;

    let upgraded = match read_context_and_state(database).await? {
        Some(state) if state.is_schema_applied_current() => state,
        Some(state) => {
            return fail_closed(
                database,
                format!("HANDSHAKE_SURREAL_PRE_MT152_UPGRADE_STATE_MISMATCH: {state:?}"),
            )
            .await;
        }
        None => {
            return fail_closed(
                database,
                "HANDSHAKE_SURREAL_PRE_MT152_UPGRADE_STATE_MISSING".to_owned(),
            )
            .await;
        }
    };
    ensure_knowledge_schema_registry(database).await?;
    let observed = inspect_schema(database).await?;
    verify_expected_info_fingerprint(database, &observed).await?;
    finalize_schema_state(database, &upgraded, &observed.info_fingerprint_sha256).await?;
    match read_context_and_state(database).await? {
        Some(state) if state.is_exact_current() => Ok(observed),
        Some(state) => {
            fail_closed(
                database,
                format!("HANDSHAKE_SURREAL_PRE_MT152_UPGRADE_FINAL_STATE_MISMATCH: {state:?}"),
            )
            .await
        }
        None => {
            fail_closed(
                database,
                "HANDSHAKE_SURREAL_PRE_MT152_UPGRADE_FINAL_STATE_MISSING".to_owned(),
            )
            .await
        }
    }
}

/// MT-150: upgrades an exact MT-109 current store in place by adding
/// `loom_edges.event_ledger_event_id` and `idx_loom_edges_event` inside one transaction guarded
/// by the exact prior state, then finalizes through the same fingerprint gate as every other
/// lineage. Every application row is untouched; pre-existing edges keep `NONE`.
async fn upgrade_pre_mt150_current(
    database: &SurrealAdminContext<'_>,
    previous_state: &SchemaState,
) -> Result<ObservedSchema, SurrealStorageError> {
    if !previous_state.is_exact_pre_mt150_current() {
        return fail_closed(
            database,
            "HANDSHAKE_SURREAL_PRE_MT150_UPGRADE_PRECONDITION_FAILED".to_owned(),
        )
        .await;
    }
    let mt141_upgrade = mt141_upgrade_statements();
    let document_upgrade = mt120_document_upgrade_statements();
    let upgrade = format!(
        "BEGIN TRANSACTION;\n\
LET $current = SELECT * FROM ONLY handshake_schema_state:primary;\n\
IF $current = NONE\n\
    OR $current.version != $schema_version\n\
    OR $current.revision != $predecessor_revision\n\
    OR $current.target_revision != $predecessor_revision\n\
    OR $current.namespace != $namespace\n\
    OR $current.database != $database\n\
    OR $current.source_manifest_sha256 != $source_manifest_sha256\n\
    OR $current.generated_surql_sha256 != $predecessor_generated_surql_sha256\n\
    OR $current.info_fingerprint_sha256 != $predecessor_info_fingerprint_sha256\n\
    OR $current.apply_state != 'complete'\n\
{{\n\
    THROW 'HANDSHAKE_SURREAL_PRE_MT150_UPGRADE_STATE_CHANGED';\n\
}};\n\
{MT150_LOOM_EDGE_RECEIPT_UPGRADE_STATEMENTS}\
{mt141_upgrade}{LOCAL_ACCOUNT_SETUP_STATEMENTS}\
{document_upgrade}\
UPDATE ONLY handshake_schema_state:primary SET\n\
    revision = $schema_revision, target_revision = $schema_revision,
    generated_surql_sha256 = $generated_surql_sha256,\n\
    info_fingerprint_sha256 = $pending_info_fingerprint_sha256,\n\
    apply_state = 'schema_applied',\n\
    updated_at = time::now();\n\
COMMIT TRANSACTION;\n"
    );
    database
        .query_bound(
            upgrade.as_str(),
            PredecessorUpgradeBindings {
                schema_version: SCHEMA_VERSION.to_owned(),
                schema_revision: SCHEMA_REVISION,
                predecessor_revision: PRE_ACCOUNT_SETUP_REVISION,
                namespace: DEFAULT_NAMESPACE.to_owned(),
                database: DEFAULT_DATABASE.to_owned(),
                source_manifest_sha256: SCHEMA_LINEAGE_SHA256.to_owned(),
                predecessor_generated_surql_sha256: PRE_MT150_GENERATED_SURREALQL_SHA256.to_owned(),
                predecessor_info_fingerprint_sha256: PRE_MT150_SCHEMA_INFO_SHA256.to_owned(),
                generated_surql_sha256: GENERATED_SURREALQL_SHA256.to_owned(),
                pending_info_fingerprint_sha256: PENDING_SCHEMA_INFO_SHA256.to_owned(),
                schema_source: "storage/surreal/schema.surql".to_owned(),
            },
        )
        .await?;

    let upgraded = match read_context_and_state(database).await? {
        Some(state) if state.is_schema_applied_current() => state,
        Some(state) => {
            return fail_closed(
                database,
                format!("HANDSHAKE_SURREAL_PRE_MT150_UPGRADE_STATE_MISMATCH: {state:?}"),
            )
            .await;
        }
        None => {
            return fail_closed(
                database,
                "HANDSHAKE_SURREAL_PRE_MT150_UPGRADE_STATE_MISSING".to_owned(),
            )
            .await;
        }
    };
    ensure_knowledge_schema_registry(database).await?;
    let observed = inspect_schema(database).await?;
    verify_expected_info_fingerprint(database, &observed).await?;
    finalize_schema_state(database, &upgraded, &observed.info_fingerprint_sha256).await?;
    match read_context_and_state(database).await? {
        Some(state) if state.is_exact_current() => Ok(observed),
        Some(state) => {
            fail_closed(
                database,
                format!("HANDSHAKE_SURREAL_PRE_MT150_UPGRADE_FINAL_STATE_MISMATCH: {state:?}"),
            )
            .await
        }
        None => {
            fail_closed(
                database,
                "HANDSHAKE_SURREAL_PRE_MT150_UPGRADE_FINAL_STATE_MISSING".to_owned(),
            )
            .await
        }
    }
}

/// MT-141 R9: upgrades an exact MT-150 current store in place by re-defining
/// `atelier_media_source_provenance_ref.asset_id` with the composite provenance-ref constraint
/// inside one transaction guarded by the exact prior state, then finalizes through the same
/// fingerprint gate as every other lineage. Every application row is untouched.
async fn upgrade_pre_mt141_current(
    database: &SurrealAdminContext<'_>,
    previous_state: &SchemaState,
) -> Result<ObservedSchema, SurrealStorageError> {
    if !previous_state.is_exact_pre_mt141_current() {
        return fail_closed(
            database,
            "HANDSHAKE_SURREAL_PRE_MT141_UPGRADE_PRECONDITION_FAILED".to_owned(),
        )
        .await;
    }
    let mt141_upgrade = mt141_upgrade_statements();
    let document_upgrade = mt120_document_upgrade_statements();
    let upgrade = format!(
        "BEGIN TRANSACTION;\n\
LET $current = SELECT * FROM ONLY handshake_schema_state:primary;\n\
IF $current = NONE\n\
    OR $current.version != $schema_version\n\
    OR $current.revision != $predecessor_revision\n\
    OR $current.target_revision != $predecessor_revision\n\
    OR $current.namespace != $namespace\n\
    OR $current.database != $database\n\
    OR $current.source_manifest_sha256 != $source_manifest_sha256\n\
    OR $current.generated_surql_sha256 != $predecessor_generated_surql_sha256\n\
    OR $current.info_fingerprint_sha256 != $predecessor_info_fingerprint_sha256\n\
    OR $current.apply_state != 'complete'\n\
{{\n\
    THROW 'HANDSHAKE_SURREAL_PRE_MT141_UPGRADE_STATE_CHANGED';\n\
}};\n\
{mt141_upgrade}{LOCAL_ACCOUNT_SETUP_STATEMENTS}\
{document_upgrade}\
UPDATE ONLY handshake_schema_state:primary SET\n\
    revision = $schema_revision, target_revision = $schema_revision,
    generated_surql_sha256 = $generated_surql_sha256,\n\
    info_fingerprint_sha256 = $pending_info_fingerprint_sha256,\n\
    apply_state = 'schema_applied',\n\
    updated_at = time::now();\n\
COMMIT TRANSACTION;\n"
    );
    database
        .query_bound(
            upgrade.as_str(),
            PredecessorUpgradeBindings {
                schema_version: SCHEMA_VERSION.to_owned(),
                schema_revision: SCHEMA_REVISION,
                predecessor_revision: PRE_ACCOUNT_SETUP_REVISION,
                namespace: DEFAULT_NAMESPACE.to_owned(),
                database: DEFAULT_DATABASE.to_owned(),
                source_manifest_sha256: SCHEMA_LINEAGE_SHA256.to_owned(),
                predecessor_generated_surql_sha256: PRE_MT141_GENERATED_SURREALQL_SHA256.to_owned(),
                predecessor_info_fingerprint_sha256: PRE_MT141_SCHEMA_INFO_SHA256.to_owned(),
                generated_surql_sha256: GENERATED_SURREALQL_SHA256.to_owned(),
                pending_info_fingerprint_sha256: PENDING_SCHEMA_INFO_SHA256.to_owned(),
                schema_source: "storage/surreal/schema.surql".to_owned(),
            },
        )
        .await?;

    let upgraded = match read_context_and_state(database).await? {
        Some(state) if state.is_schema_applied_current() => state,
        Some(state) => {
            return fail_closed(
                database,
                format!("HANDSHAKE_SURREAL_PRE_MT141_UPGRADE_STATE_MISMATCH: {state:?}"),
            )
            .await;
        }
        None => {
            return fail_closed(
                database,
                "HANDSHAKE_SURREAL_PRE_MT141_UPGRADE_STATE_MISSING".to_owned(),
            )
            .await;
        }
    };
    ensure_knowledge_schema_registry(database).await?;
    let observed = inspect_schema(database).await?;
    verify_expected_info_fingerprint(database, &observed).await?;
    finalize_schema_state(database, &upgraded, &observed.info_fingerprint_sha256).await?;
    match read_context_and_state(database).await? {
        Some(state) if state.is_exact_current() => Ok(observed),
        Some(state) => {
            fail_closed(
                database,
                format!("HANDSHAKE_SURREAL_PRE_MT141_UPGRADE_FINAL_STATE_MISMATCH: {state:?}"),
            )
            .await
        }
        None => {
            fail_closed(
                database,
                "HANDSHAKE_SURREAL_PRE_MT141_UPGRADE_FINAL_STATE_MISSING".to_owned(),
            )
            .await
        }
    }
}

/// Revision 160 widens only standalone owned LoomBlock updates. The exact
/// revision-159 state is upgraded in place so existing stores receive the
/// same resource-grant predicate as fresh declarative installs.
async fn upgrade_pre_standalone_loom_update_current(
    database: &SurrealAdminContext<'_>,
    previous_state: &SchemaState,
) -> Result<ObservedSchema, SurrealStorageError> {
    if !previous_state.is_exact_pre_standalone_loom_update_current() {
        return fail_closed(
            database,
            "HANDSHAKE_SURREAL_PRE_STANDALONE_LOOM_UPDATE_PRECONDITION_FAILED".to_owned(),
        )
        .await;
    }
    let predecessor_observed = read_schema_catalog(database).await?;
    if predecessor_observed.info_fingerprint_sha256 != PRE_STANDALONE_LOOM_UPDATE_INFO_SHA256 {
        return fail_closed(
            database,
            format!(
                "HANDSHAKE_SURREAL_PRE_STANDALONE_LOOM_UPDATE_CATALOG_MISMATCH: expected={PRE_STANDALONE_LOOM_UPDATE_INFO_SHA256}; observed={}",
                predecessor_observed.info_fingerprint_sha256
            ),
        )
        .await;
    }
    let loom_upgrade = standalone_loom_update_upgrade_statement();
    let delta_upgrade = schema_delta_upgrade_statements();
    let upgrade = format!(
        "BEGIN TRANSACTION;\n\
LET $current = SELECT * FROM ONLY handshake_schema_state:primary;\n\
IF $current = NONE\n\
    OR $current.version != $schema_version\n\
    OR $current.revision != $predecessor_revision\n\
    OR $current.target_revision != $predecessor_revision\n\
    OR $current.namespace != $namespace\n\
    OR $current.database != $database\n\
    OR $current.source_manifest_sha256 != $source_manifest_sha256\n\
    OR $current.generated_surql_sha256 != $predecessor_generated_surql_sha256\n\
    OR $current.info_fingerprint_sha256 != $predecessor_info_fingerprint_sha256\n\
    OR $current.apply_state != 'complete'\n\
{{\n\
    THROW 'HANDSHAKE_SURREAL_PRE_STANDALONE_LOOM_UPDATE_STATE_CHANGED';\n\
}};\n\
{delta_upgrade}\n\
{loom_upgrade}\n\
UPDATE ONLY handshake_schema_state:primary SET\n\
    revision = $schema_revision, target_revision = $schema_revision,\n\
    generated_surql_sha256 = $generated_surql_sha256,\n\
    info_fingerprint_sha256 = $pending_info_fingerprint_sha256,\n\
    apply_state = 'schema_applied',\n\
    updated_at = time::now();\n\
COMMIT TRANSACTION;\n"
    );
    database
        .query_bound(
            upgrade.as_str(),
            PredecessorUpgradeBindings {
                schema_version: SCHEMA_VERSION.to_owned(),
                schema_revision: SCHEMA_REVISION,
                predecessor_revision: PRE_STANDALONE_LOOM_UPDATE_REVISION,
                namespace: DEFAULT_NAMESPACE.to_owned(),
                database: DEFAULT_DATABASE.to_owned(),
                source_manifest_sha256: SCHEMA_LINEAGE_SHA256.to_owned(),
                predecessor_generated_surql_sha256: PRE_STANDALONE_LOOM_UPDATE_GENERATED_SHA256
                    .to_owned(),
                predecessor_info_fingerprint_sha256: PRE_STANDALONE_LOOM_UPDATE_INFO_SHA256
                    .to_owned(),
                generated_surql_sha256: GENERATED_SURREALQL_SHA256.to_owned(),
                pending_info_fingerprint_sha256: PENDING_SCHEMA_INFO_SHA256.to_owned(),
                schema_source: "storage/surreal/schema.surql".to_owned(),
            },
        )
        .await?;

    let upgraded = match read_context_and_state(database).await? {
        Some(state) if state.is_schema_applied_current() => state,
        Some(state) => {
            return fail_closed(
                database,
                format!("HANDSHAKE_SURREAL_PRE_STANDALONE_LOOM_UPDATE_STATE_MISMATCH: {state:?}"),
            )
            .await;
        }
        None => {
            return fail_closed(
                database,
                "HANDSHAKE_SURREAL_PRE_STANDALONE_LOOM_UPDATE_STATE_MISSING".to_owned(),
            )
            .await;
        }
    };
    ensure_knowledge_schema_registry(database).await?;
    let observed = inspect_schema(database).await?;
    verify_expected_info_fingerprint(database, &observed).await?;
    finalize_schema_state(database, &upgraded, &observed.info_fingerprint_sha256).await?;
    match read_context_and_state(database).await? {
        Some(state) if state.is_exact_current() => Ok(observed),
        Some(state) => {
            fail_closed(
                database,
                format!(
                    "HANDSHAKE_SURREAL_PRE_STANDALONE_LOOM_UPDATE_FINAL_STATE_MISMATCH: {state:?}"
                ),
            )
            .await
        }
        None => {
            fail_closed(
                database,
                "HANDSHAKE_SURREAL_PRE_STANDALONE_LOOM_UPDATE_FINAL_STATE_MISSING".to_owned(),
            )
            .await
        }
    }
}

/// Revision 159 changes only the EventLedger receipt permission function. The exact revision-158
/// state is upgraded in place so existing stores receive the same narrow creator-session rule as
/// a fresh declarative install.
async fn upgrade_pre_canvas_receipt_current(
    database: &SurrealAdminContext<'_>,
    previous_state: &SchemaState,
) -> Result<ObservedSchema, SurrealStorageError> {
    if !previous_state.is_exact_pre_canvas_receipt_current() {
        return fail_closed(
            database,
            "HANDSHAKE_SURREAL_PRE_CANVAS_RECEIPT_UPGRADE_PRECONDITION_FAILED".to_owned(),
        )
        .await;
    }
    let predecessor_observed = read_schema_catalog(database).await?;
    if predecessor_observed.info_fingerprint_sha256 != PRE_CANVAS_RECEIPT_INFO_SHA256 {
        return fail_closed(
            database,
            format!(
                "HANDSHAKE_SURREAL_PRE_CANVAS_RECEIPT_CATALOG_MISMATCH: expected={PRE_CANVAS_RECEIPT_INFO_SHA256}; observed={}",
                predecessor_observed.info_fingerprint_sha256
            ),
        )
        .await;
    }
    let loom_update = standalone_loom_update_upgrade_statement();
    let receipt_upgrade = mt120_loom_receipt_upgrade_statement();
    let delta_upgrade = schema_delta_upgrade_statements();
    let upgrade = format!(
        "BEGIN TRANSACTION;\n\
LET $current = SELECT * FROM ONLY handshake_schema_state:primary;\n\
IF $current = NONE\n\
    OR $current.version != $schema_version\n\
    OR $current.revision != $predecessor_revision\n\
    OR $current.target_revision != $predecessor_revision\n\
    OR $current.namespace != $namespace\n\
    OR $current.database != $database\n\
    OR $current.source_manifest_sha256 != $source_manifest_sha256\n\
    OR $current.generated_surql_sha256 != $predecessor_generated_surql_sha256\n\
    OR $current.info_fingerprint_sha256 != $predecessor_info_fingerprint_sha256\n\
    OR $current.apply_state != 'complete'\n\
{{\n\
    THROW 'HANDSHAKE_SURREAL_PRE_CANVAS_RECEIPT_UPGRADE_STATE_CHANGED';\n\
}};\n\
{delta_upgrade}\n\
{loom_update}\n\
{receipt_upgrade}\n\
UPDATE ONLY handshake_schema_state:primary SET\n\
    revision = $schema_revision, target_revision = $schema_revision,\n\
    generated_surql_sha256 = $generated_surql_sha256,\n\
    info_fingerprint_sha256 = $pending_info_fingerprint_sha256,\n\
    apply_state = 'schema_applied',\n\
    updated_at = time::now();\n\
COMMIT TRANSACTION;\n"
    );
    database
        .query_bound(
            upgrade.as_str(),
            PredecessorUpgradeBindings {
                schema_version: SCHEMA_VERSION.to_owned(),
                schema_revision: SCHEMA_REVISION,
                predecessor_revision: PRE_CANVAS_RECEIPT_REVISION,
                namespace: DEFAULT_NAMESPACE.to_owned(),
                database: DEFAULT_DATABASE.to_owned(),
                source_manifest_sha256: SCHEMA_LINEAGE_SHA256.to_owned(),
                predecessor_generated_surql_sha256: PRE_CANVAS_RECEIPT_GENERATED_SHA256.to_owned(),
                predecessor_info_fingerprint_sha256: PRE_CANVAS_RECEIPT_INFO_SHA256.to_owned(),
                generated_surql_sha256: GENERATED_SURREALQL_SHA256.to_owned(),
                pending_info_fingerprint_sha256: PENDING_SCHEMA_INFO_SHA256.to_owned(),
                schema_source: "storage/surreal/schema.surql".to_owned(),
            },
        )
        .await?;

    let upgraded = match read_context_and_state(database).await? {
        Some(state) if state.is_schema_applied_current() => state,
        Some(state) => {
            return fail_closed(
                database,
                format!("HANDSHAKE_SURREAL_PRE_CANVAS_RECEIPT_UPGRADE_STATE_MISMATCH: {state:?}"),
            )
            .await;
        }
        None => {
            return fail_closed(
                database,
                "HANDSHAKE_SURREAL_PRE_CANVAS_RECEIPT_UPGRADE_STATE_MISSING".to_owned(),
            )
            .await;
        }
    };
    ensure_knowledge_schema_registry(database).await?;
    let observed = inspect_schema(database).await?;
    verify_expected_info_fingerprint(database, &observed).await?;
    finalize_schema_state(database, &upgraded, &observed.info_fingerprint_sha256).await?;
    match read_context_and_state(database).await? {
        Some(state) if state.is_exact_current() => Ok(observed),
        Some(state) => {
            fail_closed(
                database,
                format!(
                    "HANDSHAKE_SURREAL_PRE_CANVAS_RECEIPT_UPGRADE_FINAL_STATE_MISMATCH: {state:?}"
                ),
            )
            .await
        }
        None => {
            fail_closed(
                database,
                "HANDSHAKE_SURREAL_PRE_CANVAS_RECEIPT_UPGRADE_FINAL_STATE_MISSING".to_owned(),
            )
            .await
        }
    }
}

async fn upgrade_pre_account_setup(
    database: &SurrealAdminContext<'_>,
    previous_state: &SchemaState,
) -> Result<ObservedSchema, SurrealStorageError> {
    if !previous_state.is_exact_pre_account_setup() {
        return fail_closed(
            database,
            "HANDSHAKE_SURREAL_PRE_ACCOUNT_SETUP_UPGRADE_PRECONDITION_FAILED".to_owned(),
        )
        .await;
    }
    let mt141_upgrade = LOCAL_ACCOUNT_SETUP_STATEMENTS;
    let document_upgrade = mt120_document_upgrade_statements();
    let upgrade = format!(
        "BEGIN TRANSACTION;\n\
LET $current = SELECT * FROM ONLY handshake_schema_state:primary;\n\
IF $current = NONE\n\
    OR $current.version != $schema_version\n\
    OR $current.revision != $predecessor_revision\n\
    OR $current.target_revision != $predecessor_revision\n\
    OR $current.namespace != $namespace\n\
    OR $current.database != $database\n\
    OR $current.source_manifest_sha256 != $source_manifest_sha256\n\
    OR $current.generated_surql_sha256 != $predecessor_generated_surql_sha256\n\
    OR $current.info_fingerprint_sha256 != $predecessor_info_fingerprint_sha256\n\
    OR $current.apply_state != 'complete'\n\
{{\n\
    THROW 'HANDSHAKE_SURREAL_PRE_ACCOUNT_SETUP_UPGRADE_STATE_CHANGED';\n\
}};\n\
{mt141_upgrade}\
{document_upgrade}\
UPDATE ONLY handshake_schema_state:primary SET\n\
    revision = $schema_revision, target_revision = $schema_revision,
    generated_surql_sha256 = $generated_surql_sha256,\n\
    info_fingerprint_sha256 = $pending_info_fingerprint_sha256,\n\
    apply_state = 'schema_applied',\n\
    updated_at = time::now();\n\
COMMIT TRANSACTION;\n"
    );
    database
        .query_bound(
            upgrade.as_str(),
            PredecessorUpgradeBindings {
                schema_version: SCHEMA_VERSION.to_owned(),
                schema_revision: SCHEMA_REVISION,
                predecessor_revision: PRE_ACCOUNT_SETUP_REVISION,
                namespace: DEFAULT_NAMESPACE.to_owned(),
                database: DEFAULT_DATABASE.to_owned(),
                source_manifest_sha256: SCHEMA_LINEAGE_SHA256.to_owned(),
                predecessor_generated_surql_sha256: PRE_ACCOUNT_SETUP_GENERATED_SHA256.to_owned(),
                predecessor_info_fingerprint_sha256: PRE_ACCOUNT_SETUP_INFO_SHA256.to_owned(),
                generated_surql_sha256: GENERATED_SURREALQL_SHA256.to_owned(),
                pending_info_fingerprint_sha256: PENDING_SCHEMA_INFO_SHA256.to_owned(),
                schema_source: "storage/surreal/schema.surql".to_owned(),
            },
        )
        .await?;

    let upgraded = match read_context_and_state(database).await? {
        Some(state) if state.is_schema_applied_current() => state,
        Some(state) => {
            return fail_closed(
                database,
                format!("HANDSHAKE_SURREAL_PRE_ACCOUNT_SETUP_UPGRADE_STATE_MISMATCH: {state:?}"),
            )
            .await;
        }
        None => {
            return fail_closed(
                database,
                "HANDSHAKE_SURREAL_PRE_ACCOUNT_SETUP_UPGRADE_STATE_MISSING".to_owned(),
            )
            .await;
        }
    };
    ensure_knowledge_schema_registry(database).await?;
    let observed = inspect_schema(database).await?;
    verify_expected_info_fingerprint(database, &observed).await?;
    finalize_schema_state(database, &upgraded, &observed.info_fingerprint_sha256).await?;
    match read_context_and_state(database).await? {
        Some(state) if state.is_exact_current() => Ok(observed),
        Some(state) => {
            fail_closed(
                database,
                format!(
                    "HANDSHAKE_SURREAL_PRE_ACCOUNT_SETUP_UPGRADE_FINAL_STATE_MISMATCH: {state:?}"
                ),
            )
            .await
        }
        None => {
            fail_closed(
                database,
                "HANDSHAKE_SURREAL_PRE_ACCOUNT_SETUP_UPGRADE_FINAL_STATE_MISSING".to_owned(),
            )
            .await
        }
    }
}

fn mt109_authority_upgrade_bindings() -> PredecessorUpgradeBindings {
    PredecessorUpgradeBindings {
        schema_version: SCHEMA_VERSION.to_owned(),
        schema_revision: SCHEMA_REVISION,
        predecessor_revision: PRE_ACCOUNT_SETUP_REVISION,
        namespace: DEFAULT_NAMESPACE.to_owned(),
        database: DEFAULT_DATABASE.to_owned(),
        source_manifest_sha256: SCHEMA_LINEAGE_SHA256.to_owned(),
        predecessor_generated_surql_sha256: PRE_MT109_GENERATED_SURREALQL_SHA256.to_owned(),
        predecessor_info_fingerprint_sha256: PRE_MT109_SCHEMA_INFO_SHA256.to_owned(),
        generated_surql_sha256: GENERATED_SURREALQL_SHA256.to_owned(),
        pending_info_fingerprint_sha256: PENDING_SCHEMA_INFO_SHA256.to_owned(),
        schema_source: "storage/surreal/schema.surql".to_owned(),
    }
}

fn mt109_authority_upgrade_query() -> String {
    let authority_delta = post_mt109_upgrade_statements();
    format!(
        r#"
BEGIN TRANSACTION;
LET $current = SELECT * FROM ONLY handshake_schema_state:primary;
IF $current = NONE
    OR $current.version != $schema_version
    OR $current.revision != $predecessor_revision
    OR $current.target_revision != $predecessor_revision
    OR $current.namespace != $namespace
    OR $current.database != $database
    OR $current.source_manifest_sha256 != $source_manifest_sha256
    OR $current.generated_surql_sha256 != $predecessor_generated_surql_sha256
    OR $current.info_fingerprint_sha256 != $predecessor_info_fingerprint_sha256
    OR $current.apply_state != 'complete'
{{
    THROW 'HANDSHAKE_SURREAL_PRE_MT109_UPGRADE_STATE_CHANGED';
}};
{authority_delta}
UPDATE ONLY handshake_schema_state:primary SET
    revision = $schema_revision, target_revision = $schema_revision,
    generated_surql_sha256 = $generated_surql_sha256,
    info_fingerprint_sha256 = $pending_info_fingerprint_sha256,
    apply_state = 'schema_applied',
    updated_at = time::now();
COMMIT TRANSACTION;
"#
    )
}

async fn upgrade_pre_mt109_current(
    database: &SurrealAdminContext<'_>,
    previous_state: &SchemaState,
) -> Result<ObservedSchema, SurrealStorageError> {
    if !previous_state.is_exact_pre_mt109_current() {
        return fail_closed(
            database,
            "HANDSHAKE_SURREAL_PRE_MT109_UPGRADE_PRECONDITION_FAILED".to_owned(),
        )
        .await;
    }
    // The old receipt predates its out-of-band authority overlay. Accept only either complete
    // observed catalog, including every access/function/permission; the receipt alone is insufficient.
    let predecessor = read_schema_catalog(database).await?;
    if ![
        PRE_MT109_SCHEMA_INFO_SHA256,
        PRE_MT109_AUTHORITY_INFO_SHA256,
    ]
    .contains(&predecessor.info_fingerprint_sha256.as_str())
    {
        return fail_closed(
            database,
            format!(
                "HANDSHAKE_SURREAL_PRE_MT109_CATALOG_MISMATCH: observed={}",
                predecessor.info_fingerprint_sha256
            ),
        )
        .await;
    }
    database
        .query_bound(
            mt109_authority_upgrade_query().as_str(),
            mt109_authority_upgrade_bindings(),
        )
        .await?;
    let applied = match read_context_and_state(database).await? {
        Some(state) if state.is_schema_applied_current() => state,
        other => {
            return fail_closed(
                database,
                format!("HANDSHAKE_SURREAL_PRE_MT109_APPLY_STATE_MISMATCH: {other:?}"),
            )
            .await
        }
    };
    ensure_knowledge_schema_registry(database).await?;
    let observed = inspect_schema(database).await?;
    verify_expected_info_fingerprint(database, &observed).await?;
    finalize_schema_state(database, &applied, &observed.info_fingerprint_sha256).await?;
    Ok(observed)
}

async fn finalize_schema_state(
    database: &SurrealAdminContext<'_>,
    applied_state: &SchemaState,
    info_fingerprint_sha256: &str,
) -> Result<(), SurrealStorageError> {
    if !applied_state.is_schema_applied_current() || info_fingerprint_sha256.len() != 64 {
        return fail_closed(
            database,
            "HANDSHAKE_SURREAL_SCHEMA_FINALIZE_PRECONDITION_FAILED".to_owned(),
        )
        .await;
    }
    database
        .query_bound(
            r#"
BEGIN TRANSACTION;
LET $current = SELECT * FROM ONLY handshake_schema_state:primary;
IF $current = NONE
    OR $current.version != $schema_version
    OR $current.revision != $schema_revision
    OR $current.target_revision != $schema_revision
    OR $current.namespace != $namespace
    OR $current.database != $database
    OR $current.source_manifest_sha256 != $source_manifest_sha256
    OR $current.generated_surql_sha256 != $generated_surql_sha256
    OR $current.info_fingerprint_sha256 != $pending_info_fingerprint_sha256
    OR $current.apply_state != 'schema_applied'
{
    THROW 'HANDSHAKE_SURREAL_SCHEMA_FINALIZE_STATE_CHANGED';
};
UPDATE ONLY handshake_schema_state:primary SET
    info_fingerprint_sha256 = $info_fingerprint_sha256,
    apply_state = 'complete',
    updated_at = time::now();
COMMIT TRANSACTION;
"#,
            FinalizeBindings {
                schema_version: SCHEMA_VERSION.to_owned(),
                schema_revision: SCHEMA_REVISION,
                namespace: DEFAULT_NAMESPACE.to_owned(),
                database: DEFAULT_DATABASE.to_owned(),
                source_manifest_sha256: SCHEMA_LINEAGE_SHA256.to_owned(),
                generated_surql_sha256: GENERATED_SURREALQL_SHA256.to_owned(),
                pending_info_fingerprint_sha256: PENDING_SCHEMA_INFO_SHA256.to_owned(),
                info_fingerprint_sha256: info_fingerprint_sha256.to_owned(),
            },
        )
        .await?;
    Ok(())
}

async fn observe_schema(
    database: &SurrealAdminContext<'_>,
    state: SchemaState,
    outcome: SchemaBootstrapOutcome,
) -> Result<SchemaBootstrapReport, SurrealStorageError> {
    let observed = inspect_schema(database).await?;
    report_from_observed(database, state, observed, outcome).await
}

async fn report_from_observed(
    database: &SurrealAdminContext<'_>,
    state: SchemaState,
    observed: ObservedSchema,
    outcome: SchemaBootstrapOutcome,
) -> Result<SchemaBootstrapReport, SurrealStorageError> {
    verify_expected_info_fingerprint(database, &observed).await?;
    if observed.info_fingerprint_sha256 != state.info_fingerprint_sha256 {
        return fail_closed(
            database,
            format!(
                "HANDSHAKE_SURREAL_SCHEMA_INFO_FINGERPRINT_MISMATCH: expected={}; observed={}",
                state.info_fingerprint_sha256, observed.info_fingerprint_sha256
            ),
        )
        .await;
    }

    Ok(SchemaBootstrapReport {
        schema_version: state.version,
        namespace: state.namespace,
        database: state.database,
        declarative_schema_files: 1,
        source_manifest_sha256: state.source_manifest_sha256,
        generated_surql_sha256: state.generated_surql_sha256,
        info_fingerprint_sha256: state.info_fingerprint_sha256,
        tables_defined: observed.tables_defined,
        fields_defined: observed.fields_defined,
        indexes_defined: observed.indexes_defined,
        table_names: observed.table_names,
        outcome,
        reused_existing_schema: outcome.reused_existing_schema(),
    })
}

async fn verify_expected_info_fingerprint(
    database: &SurrealAdminContext<'_>,
    observed: &ObservedSchema,
) -> Result<(), SurrealStorageError> {
    if EXPECTED_SCHEMA_INFO_SHA256.bytes().all(|byte| byte == b'0') {
        return fail_closed(
            database,
            format!(
                "HANDSHAKE_SURREAL_SCHEMA_INFO_FINGERPRINT_UNPINNED: observed={}",
                observed.info_fingerprint_sha256
            ),
        )
        .await;
    }
    if observed.info_fingerprint_sha256 != EXPECTED_SCHEMA_INFO_SHA256 {
        return fail_closed(
            database,
            format!(
                "HANDSHAKE_SURREAL_SCHEMA_INFO_FINGERPRINT_MISMATCH: expected={EXPECTED_SCHEMA_INFO_SHA256}; observed={}",
                observed.info_fingerprint_sha256
            ),
        )
        .await;
    }
    Ok(())
}

async fn inspect_schema(
    database: &SurrealAdminContext<'_>,
) -> Result<ObservedSchema, SurrealStorageError> {
    let observed = read_schema_catalog(database).await?;
    let mut expected_names = TABLE_NAMES
        .iter()
        .map(|name| (*name).to_owned())
        .collect::<Vec<_>>();
    expected_names.sort();
    if observed.table_names != expected_names {
        return fail_closed(database, format!("HANDSHAKE_SURREAL_SCHEMA_TABLE_SET_MISMATCH: expected={expected_names:?}; observed={:?}", observed.table_names)).await;
    }
    if observed.fields_defined != FIELD_DEFINITION_COUNT
        || observed.indexes_defined != INDEX_DEFINITION_COUNT
    {
        return fail_closed(
            database,
            format!(
                "HANDSHAKE_SURREAL_SCHEMA_INFO_MISMATCH: tables={}; fields={}; indexes={}",
                observed.tables_defined, observed.fields_defined, observed.indexes_defined
            ),
        )
        .await;
    }
    Ok(observed)
}

async fn read_schema_catalog(
    database: &SurrealAdminContext<'_>,
) -> Result<ObservedSchema, SurrealStorageError> {
    let mut db_info_response = database.query("INFO FOR DB STRUCTURE;").await?;
    let db_info: SurrealValueData = db_info_response.take(0)?;
    for category in DATABASE_STRUCTURE_CATEGORIES {
        if let Err(reason) = array_len(&db_info, category) {
            return fail_closed(database, reason).await;
        }
    }
    let mut table_names = match parse_named_array(&db_info, "tables") {
        Ok(names) => names,
        Err(reason) => return fail_closed(database, reason).await,
    };
    table_names.sort();

    let mut fields_defined = 0usize;
    let mut indexes_defined = 0usize;
    let mut table_info_by_name = BTreeMap::new();
    let table_info_query = table_names
        .iter()
        .map(|table| format!("INFO FOR TABLE `{table}` STRUCTURE;"))
        .collect::<String>();
    let mut table_responses = database.query(table_info_query).await?;
    for (statement_index, table) in table_names.iter().enumerate() {
        let table_info: SurrealValueData = table_responses.take(statement_index)?;
        for category in ["events", "fields", "indexes", "lives", "tables"] {
            if let Err(reason) = array_len(&table_info, category) {
                return fail_closed(database, reason).await;
            }
        }
        fields_defined += match array_len(&table_info, "fields") {
            Ok(count) => count,
            Err(reason) => return fail_closed(database, reason).await,
        };
        indexes_defined += match array_len(&table_info, "indexes") {
            Ok(count) => count,
            Err(reason) => return fail_closed(database, reason).await,
        };
        table_info_by_name.insert(table.clone(), table_info);
    }

    Ok(ObservedSchema {
        info_fingerprint_sha256: canonical_catalog_fingerprint(db_info, table_info_by_name),
        tables_defined: table_names.len(),
        fields_defined,
        indexes_defined,
        table_names,
    })
}

pub(super) fn info_entry_name(value: &SurrealValueData) -> Option<&str> {
    let SurrealValueData::Object(object) = value else {
        return None;
    };
    let Some(SurrealValueData::String(name)) = object.get("name") else {
        return None;
    };
    Some(name)
}

/// The ONE definition of the live schema fingerprint: `INFO FOR DB STRUCTURE` plus every
/// table's `INFO FOR TABLE ... STRUCTURE`, canonicalised (named catalog entries sorted, index
/// column order kept) with the engine's table catalog ids stripped, serialised and hashed.
/// Bootstrap (`inspect_schema`) and the test inspector both pin
/// [`EXPECTED_SCHEMA_INFO_SHA256`] through this function, so they cannot disagree.
///
/// MT-151: table catalog ids (`tables[].id` in STRUCTURE output) are the engine's allocation
/// counter, not schema. A fresh apply numbers each table by its script position; a store
/// upgraded in place by a delta `DEFINE TABLE` allocates the next free id and every later
/// table keeps its old number, so a fingerprint that kept ids could never be reached by any
/// table-adding lineage (run mt142-LIB-20260911T124424Z: every drifting entry was
/// `tables[].id`, 233 fresh vs 300 upgraded for the added table). The MT-138 Atelier catalog
/// fingerprint already strips them for the same reason (`strip_table_catalog_id`).
pub(super) fn canonical_catalog_fingerprint(
    db_info: SurrealValueData,
    tables: BTreeMap<String, SurrealValueData>,
) -> String {
    let canonical = CanonicalInfoEnvelope {
        database: strip_nested_table_catalog_ids(db_info),
        tables: tables
            .into_iter()
            .map(|(name, info)| (name, strip_nested_table_catalog_ids(info)))
            .collect(),
    };
    let canonical_json =
        serde_json::to_string(&canonical).expect("canonical structured INFO serializes losslessly");
    sha256_hex(canonical_json.as_bytes())
}

pub(super) fn canonicalize_info(value: SurrealValueData) -> SurrealValueData {
    match value {
        SurrealValueData::Object(object) => {
            let mut canonical = SurrealObject::new();
            for (key, value) in object.into_inner() {
                canonical.insert(key, canonicalize_info(value));
            }
            SurrealValueData::Object(canonical)
        }
        SurrealValueData::Array(array) => {
            let mut canonical = array
                .into_vec()
                .into_iter()
                .map(canonicalize_info)
                .collect::<Vec<_>>();
            if canonical
                .iter()
                .all(|entry| info_entry_name(entry).is_some())
            {
                canonical.sort_by(|left, right| info_entry_name(left).cmp(&info_entry_name(right)));
            }
            SurrealValueData::Array(SurrealArray::from(canonical))
        }
        scalar => scalar,
    }
}

fn parse_table_definitions(
    value: &SurrealValueData,
) -> Result<BTreeMap<String, AtelierTableDefinition>, String> {
    let SurrealValueData::Object(object) = value else {
        return Err("HANDSHAKE_SURREAL_SCHEMA_INFO_INVALID: expected object".to_owned());
    };
    let Some(SurrealValueData::Array(tables)) = object.get("tables") else {
        return Err("HANDSHAKE_SURREAL_SCHEMA_INFO_INVALID: missing `tables` array".to_owned());
    };

    let mut definitions = BTreeMap::new();
    for entry in tables.iter() {
        let SurrealValueData::Object(table) = entry else {
            return Err(
                "HANDSHAKE_SURREAL_SCHEMA_INFO_INVALID: `tables` entry is not an object".to_owned(),
            );
        };
        let name = info_entry_name(entry)
            .map(|name| name.trim_matches('`').to_owned())
            .ok_or_else(|| {
                "HANDSHAKE_SURREAL_SCHEMA_INFO_INVALID: `tables` entry missing name".to_owned()
            })?;
        let Some(SurrealValueData::Bool(schemafull)) = table.get("schemafull") else {
            return Err(format!(
                "HANDSHAKE_SURREAL_SCHEMA_INFO_INVALID: table `{name}` missing schemafull"
            ));
        };
        let Some(SurrealValueData::Object(kind)) = table.get("kind") else {
            return Err(format!(
                "HANDSHAKE_SURREAL_SCHEMA_INFO_INVALID: table `{name}` missing kind object"
            ));
        };
        let Some(SurrealValueData::String(kind)) = kind.get("kind") else {
            return Err(format!(
                "HANDSHAKE_SURREAL_SCHEMA_INFO_INVALID: table `{name}` missing kind token"
            ));
        };
        let is_view = table
            .get("view")
            .is_some_and(|view| !matches!(view, SurrealValueData::None | SurrealValueData::Null));
        let definition = AtelierTableDefinition {
            schemafull: *schemafull,
            kind: kind.to_owned(),
            is_view,
        };
        if definitions.insert(name.clone(), definition).is_some() {
            return Err(format!(
                "HANDSHAKE_SURREAL_SCHEMA_INFO_INVALID: duplicate table `{name}`"
            ));
        }
    }
    Ok(definitions)
}

fn parse_named_structures(
    value: &SurrealValueData,
    key: &str,
) -> Result<BTreeMap<String, SurrealValueData>, String> {
    let SurrealValueData::Object(object) = value else {
        return Err("HANDSHAKE_SURREAL_SCHEMA_INFO_INVALID: expected object".to_owned());
    };
    let Some(SurrealValueData::Array(array)) = object.get(key) else {
        return Err(format!(
            "HANDSHAKE_SURREAL_SCHEMA_INFO_INVALID: missing `{key}` array"
        ));
    };
    let mut definitions = BTreeMap::new();
    for entry in array.iter() {
        let name = info_entry_name(entry)
            .map(|name| name.trim_matches('`').to_owned())
            .ok_or_else(|| {
                format!("HANDSHAKE_SURREAL_SCHEMA_INFO_INVALID: `{key}` entry missing name")
            })?;
        if definitions.insert(name.clone(), entry.clone()).is_some() {
            return Err(format!(
                "HANDSHAKE_SURREAL_SCHEMA_INFO_INVALID: duplicate `{key}` entry `{name}`"
            ));
        }
    }
    Ok(definitions)
}

fn strip_table_catalog_id(value: SurrealValueData) -> SurrealValueData {
    match value {
        SurrealValueData::Object(object) => {
            let mut object = object.into_inner();
            object.remove("id");
            SurrealValueData::Object(SurrealObject::from(object))
        }
        value => value,
    }
}

fn strip_nested_table_catalog_ids(value: SurrealValueData) -> SurrealValueData {
    match value {
        SurrealValueData::Object(object) => {
            let mut normalized = SurrealObject::new();
            for (key, value) in object.into_inner() {
                let value = if key == "tables" {
                    match value {
                        SurrealValueData::Array(array) => {
                            SurrealValueData::Array(SurrealArray::from(
                                array
                                    .into_vec()
                                    .into_iter()
                                    .map(strip_table_catalog_id)
                                    .collect::<Vec<_>>(),
                            ))
                        }
                        value => value,
                    }
                } else {
                    value
                };
                normalized.insert(key, canonicalize_info(value));
            }
            SurrealValueData::Object(normalized)
        }
        value => canonicalize_info(value),
    }
}

pub(super) fn parse_named_array(
    value: &SurrealValueData,
    key: &str,
) -> Result<Vec<String>, String> {
    let SurrealValueData::Object(object) = value else {
        return Err("HANDSHAKE_SURREAL_SCHEMA_INFO_INVALID: expected object".to_owned());
    };
    let Some(SurrealValueData::Array(array)) = object.get(key) else {
        return Err(format!(
            "HANDSHAKE_SURREAL_SCHEMA_INFO_INVALID: missing `{key}` array"
        ));
    };
    array
        .iter()
        .map(|entry| {
            info_entry_name(entry)
                .map(|name| name.trim_matches('`').to_owned())
                .ok_or_else(|| {
                    format!("HANDSHAKE_SURREAL_SCHEMA_INFO_INVALID: `{key}` entry missing name")
                })
        })
        .collect()
}

fn array_len(value: &SurrealValueData, key: &str) -> Result<usize, String> {
    let SurrealValueData::Object(object) = value else {
        return Err("HANDSHAKE_SURREAL_SCHEMA_INFO_INVALID: expected object".to_owned());
    };
    let Some(SurrealValueData::Array(array)) = object.get(key) else {
        return Err(format!(
            "HANDSHAKE_SURREAL_SCHEMA_INFO_INVALID: missing `{key}` array"
        ));
    };
    Ok(array.len())
}

async fn fail_closed<T>(
    database: &SurrealAdminContext<'_>,
    reason: String,
) -> Result<T, SurrealStorageError> {
    database
        .query_bound("THROW $reason;", ("reason", reason))
        .await?;
    unreachable!("THROW must fail closed")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::{
        surreal::{SurrealDatabase, SurrealStorage, SurrealStorageConfig},
        Database, EntityRef, JobMetrics, LoomFolderSortMode, LoomFolderUpdate, NewLoomFolder,
        OperationType, PlannedOperation, StorageError,
    };
    use surrealdb::{engine::local::Mem, Surreal};

    const MT138_MINIMAL_CATALOG_DDL: &str =
        "DEFINE SEQUENCE OVERWRITE atelier_mt138_catalog_seq BATCH 1 START 1; \
         DEFINE TABLE OVERWRITE atelier_mt138_catalog_probe SCHEMAFULL PERMISSIONS NONE; \
         DEFINE FIELD OVERWRITE value ON TABLE atelier_mt138_catalog_probe TYPE string; \
         DEFINE FIELD OVERWRITE marker ON TABLE atelier_mt138_catalog_probe TYPE string; \
         DEFINE INDEX OVERWRITE mt138_catalog_value ON TABLE atelier_mt138_catalog_probe FIELDS value UNIQUE; \
         DEFINE EVENT OVERWRITE mt138_catalog_event ON TABLE atelier_mt138_catalog_probe \
             WHEN $event = 'DELETE' \
             THEN { DELETE atelier_mt138_catalog_probe WHERE marker = $before.marker; }; \
         DEFINE TABLE OVERWRITE atelier_mt138_catalog_view TYPE NORMAL AS \
             SELECT `value` FROM atelier_mt138_catalog_probe PERMISSIONS NONE;";

    #[derive(SurrealValue)]
    struct NativeJsonBindings {
        entity_refs: JsonValue,
        planned_operations: JsonValue,
        metrics: JsonValue,
        job_inputs: JsonValue,
    }

    async fn open_test_storage(
        directory: &tempfile::TempDir,
    ) -> Result<SurrealStorage, SurrealStorageError> {
        SurrealStorage::open(
            SurrealStorageConfig::with_path(directory.path().join("store"))?
                .with_test_sync_from_env(),
        )
        .await
    }

    fn mt138_minimal_catalog_tables() -> BTreeSet<String> {
        BTreeSet::from([
            "atelier_mt138_catalog_probe".to_owned(),
            "atelier_mt138_catalog_view".to_owned(),
        ])
    }

    fn mt138_minimal_catalog_sequences() -> BTreeSet<String> {
        BTreeSet::from(["atelier_mt138_catalog_seq".to_owned()])
    }

    async fn mt138_minimal_catalog_query(
        storage: &SurrealStorage,
        statement: &'static str,
    ) -> Result<(), SurrealStorageError> {
        storage
            .with_admin_operation(move |database| {
                Box::pin(async move {
                    database.query(statement).await?;
                    Ok(())
                })
            })
            .await
    }

    async fn mt138_minimal_catalog_fingerprint(
        storage: &SurrealStorage,
    ) -> Result<String, SurrealStorageError> {
        storage
            .with_admin_operation(|database| {
                Box::pin(async move {
                    inspect_atelier_catalog_fingerprint(
                        &database,
                        &mt138_minimal_catalog_tables(),
                        &mt138_minimal_catalog_sequences(),
                    )
                    .await
                })
            })
            .await
    }

    async fn mt138_verify_minimal_catalog(
        storage: &SurrealStorage,
        expected_fingerprint: String,
    ) -> Result<(), SurrealStorageError> {
        storage
            .with_admin_operation(move |database| {
                Box::pin(async move {
                    verify_atelier_catalog_fingerprint(
                        &database,
                        &mt138_minimal_catalog_tables(),
                        &mt138_minimal_catalog_sequences(),
                        &expected_fingerprint,
                    )
                    .await
                })
            })
            .await
    }

    async fn mt138_mem_catalog_fingerprint(
        statement: String,
        expected_tables: &BTreeSet<String>,
        expected_sequences: &BTreeSet<String>,
    ) -> Result<String, SurrealStorageError> {
        let client = Surreal::new::<Mem>(()).await?;
        client
            .use_ns(DEFAULT_NAMESPACE)
            .use_db(DEFAULT_DATABASE)
            .await?;
        let database = SurrealAdminContext { client: &client };
        database.query(statement).await?;
        inspect_atelier_catalog_fingerprint(&database, expected_tables, expected_sequences).await
    }

    async fn mt138_canonical_mem_fingerprint(
        probe_dependencies: bool,
    ) -> Result<String, SurrealStorageError> {
        let expected_tables = atelier_expected_catalog()
            .into_keys()
            .collect::<BTreeSet<_>>();
        assert_eq!(expected_tables.len(), 136);
        let expected_sequences = ATELIER_REQUIRED_SEQUENCES
            .iter()
            .map(|name| (*name).to_owned())
            .collect::<BTreeSet<_>>();
        let client = Surreal::new::<Mem>(()).await?;
        client
            .use_ns(DEFAULT_NAMESPACE)
            .use_db(DEFAULT_DATABASE)
            .await?;
        let database = SurrealAdminContext { client: &client };
        database
            .query(format!(
                "BEGIN TRANSACTION;\n{}\nCOMMIT TRANSACTION;",
                atelier_schema_ddl()
            ))
            .await?;
        let fingerprint =
            inspect_atelier_catalog_fingerprint(&database, &expected_tables, &expected_sequences)
                .await?;
        eprintln!("EXPECTED_ATELIER_CATALOG_SHA256={fingerprint}");
        if probe_dependencies {
            assert_eq!(authority_catalog_names("access").len(), 1);
            // The authority core includes the existing document/workspace receipt helpers
            // plus mt120_loom_block_access, mt120_loom_endpoint_access (MT-109 C3) and
            // mt120_loom_receipt.
            assert_eq!(authority_catalog_names("function").len(), 16);
            let mut saved = database
                .query("RETURN (INFO FOR DB).functions.mt109_live_session;")
                .await?;
            let original = saved
                .take::<Option<String>>(0)?
                .expect("canonical authority function must exist");
            assert!(original.starts_with("DEFINE FUNCTION fn::mt109_live_session("));
            let restore = format!(
                "{};",
                original
                    .replacen("DEFINE FUNCTION ", "DEFINE FUNCTION OVERWRITE ", 1)
                    .trim_end_matches(';')
            );
            for (mutation, marker) in [
                (
                    "REMOVE FUNCTION fn::mt109_live_session;",
                    "AUTHORITY_DEPENDENCY_MISSING",
                ),
                (
                    "DEFINE FUNCTION OVERWRITE fn::mt109_live_session() { RETURN true; };",
                    "CATALOG_FINGERPRINT_MISMATCH",
                ),
            ] {
                database.query(mutation).await?;
                let error = verify_atelier_catalog_fingerprint(
                    &database,
                    &expected_tables,
                    &expected_sequences,
                    &fingerprint,
                )
                .await
                .expect_err("altered authority function must reject");
                assert!(
                    error.to_string().contains(marker),
                    "unexpected rejection: {error}"
                );
                database.query(restore.clone()).await?;
                verify_atelier_catalog_fingerprint(
                    &database,
                    &expected_tables,
                    &expected_sequences,
                    &fingerprint,
                )
                .await?;
            }
            database
                .query("ALTER ACCESS authenticated_session ON DATABASE DURATION FOR TOKEN 6m;")
                .await?;
            let error = verify_atelier_catalog_fingerprint(
                &database,
                &expected_tables,
                &expected_sequences,
                &fingerprint,
            )
            .await
            .expect_err("altered authority access must reject");
            assert!(error.to_string().contains("CATALOG_FINGERPRINT_MISMATCH"));
            database
                .query("ALTER ACCESS authenticated_session ON DATABASE DURATION FOR TOKEN 5m;")
                .await?;
            verify_atelier_catalog_fingerprint(
                &database,
                &expected_tables,
                &expected_sequences,
                &fingerprint,
            )
            .await?;
            database.query("DEFINE TABLE mt138_unrelated SCHEMAFULL; DEFINE FUNCTION fn::mt138_unrelated() { RETURN true; };").await?;
            verify_atelier_catalog_fingerprint(
                &database,
                &expected_tables,
                &expected_sequences,
                &fingerprint,
            )
            .await?;
            database
                .query("REMOVE ACCESS authenticated_session ON DATABASE;")
                .await?;
            let error = verify_atelier_catalog_fingerprint(
                &database,
                &expected_tables,
                &expected_sequences,
                &fingerprint,
            )
            .await
            .expect_err("missing authority access must reject");
            assert!(error.to_string().contains("AUTHORITY_DEPENDENCY_MISSING"));
        }
        Ok(fingerprint)
    }

    #[tokio::test]
    async fn mt138_catalog_fingerprint_is_backend_stable_between_mem_and_rocks() {
        tokio::time::timeout(std::time::Duration::from_secs(300), async {
            let directory = tempfile::tempdir().expect("create MT-138 backend parity directory");
            let rocks = open_test_storage(&directory)
                .await
                .expect("open MT-138 RocksDB parity store");
            mt138_minimal_catalog_query(&rocks, MT138_MINIMAL_CATALOG_DDL)
                .await
                .expect("create minimal RocksDB parity catalog");
            let rocks_fingerprint = mt138_minimal_catalog_fingerprint(&rocks)
                .await
                .expect("fingerprint minimal RocksDB catalog");
            let mem_fingerprint = mt138_mem_catalog_fingerprint(
                MT138_MINIMAL_CATALOG_DDL.to_owned(),
                &mt138_minimal_catalog_tables(),
                &mt138_minimal_catalog_sequences(),
            )
            .await
            .expect("fingerprint identical in-memory catalog");
            assert_eq!(
                rocks_fingerprint, mem_fingerprint,
                "normalized structured INFO must be storage-backend invariant"
            );
            rocks
                .shutdown()
                .await
                .expect("close MT-138 RocksDB parity store");
        })
        .await
        .expect("MT-138 Mem/Rocks fingerprint parity exceeded five minutes");
    }

    #[tokio::test]
    async fn mt138_minimal_real_rocks_catalog_rejects_adversarial_mutations() {
        tokio::time::timeout(std::time::Duration::from_secs(300), async {
            let directory = tempfile::tempdir().expect("create MT-138 minimal catalog directory");
            let storage = open_test_storage(&directory)
                .await
                .expect("open MT-138 minimal catalog store");
            mt138_minimal_catalog_query(&storage, MT138_MINIMAL_CATALOG_DDL)
            .await
            .expect("create exact minimal catalog");
            let expected_fingerprint = mt138_minimal_catalog_fingerprint(&storage)
                .await
                .expect("fingerprint exact minimal catalog");
            mt138_verify_minimal_catalog(&storage, expected_fingerprint.clone())
                .await
                .expect("accept exact minimal catalog");

            mt138_minimal_catalog_query(
                &storage,
                "DEFINE TABLE OVERWRITE atelier_mt138_catalog_rogue SCHEMAFULL PERMISSIONS NONE;",
            )
            .await
            .expect("create rogue table");
            let rogue_error = mt138_verify_minimal_catalog(&storage, expected_fingerprint.clone())
                .await
                .expect_err("reject rogue Atelier table");
            assert!(rogue_error.to_string().contains("TABLE_SET_MISMATCH"));
            mt138_minimal_catalog_query(
                &storage,
                "REMOVE TABLE atelier_mt138_catalog_rogue;",
            )
            .await
            .expect("remove rogue table");

            mt138_minimal_catalog_query(
                &storage,
                "DEFINE FIELD OVERWRITE attacker_extra ON TABLE atelier_mt138_catalog_probe TYPE string;",
            )
            .await
            .expect("create unexpected field");
            let field_error = mt138_verify_minimal_catalog(&storage, expected_fingerprint.clone())
                .await
                .expect_err("reject unexpected field");
            assert!(field_error.to_string().contains("CATALOG_FINGERPRINT_MISMATCH"));
            mt138_minimal_catalog_query(
                &storage,
                "REMOVE FIELD attacker_extra ON TABLE atelier_mt138_catalog_probe;",
            )
            .await
            .expect("remove unexpected field");

            mt138_minimal_catalog_query(
                &storage,
                "ALTER TABLE atelier_mt138_catalog_probe SCHEMALESS;",
            )
            .await
            .expect("make probe schemaless");
            let mode_error = mt138_verify_minimal_catalog(&storage, expected_fingerprint.clone())
                .await
                .expect_err("reject schemaless replacement");
            assert!(mode_error.to_string().contains("CATALOG_FINGERPRINT_MISMATCH"));
            mt138_minimal_catalog_query(
                &storage,
                "ALTER TABLE atelier_mt138_catalog_probe SCHEMAFULL;",
            )
            .await
            .expect("restore schemafull mode");

            mt138_minimal_catalog_query(
                &storage,
                "DEFINE TABLE OVERWRITE atelier_mt138_catalog_view TYPE NORMAL PERMISSIONS NONE;",
            )
            .await
            .expect("replace view with normal table");
            let view_error = mt138_verify_minimal_catalog(&storage, expected_fingerprint.clone())
                .await
                .expect_err("reject non-view replacement");
            assert!(view_error.to_string().contains("CATALOG_FINGERPRINT_MISMATCH"));
            mt138_minimal_catalog_query(
                &storage,
                "DEFINE TABLE OVERWRITE atelier_mt138_catalog_view TYPE NORMAL AS \
                     SELECT `value` FROM atelier_mt138_catalog_probe PERMISSIONS NONE;",
            )
            .await
            .expect("restore exact view");

            mt138_minimal_catalog_query(
                &storage,
                "DEFINE FIELD OVERWRITE value ON TABLE atelier_mt138_catalog_probe TYPE int;",
            )
            .await
            .expect("change existing field type");
            let field_type_error =
                mt138_verify_minimal_catalog(&storage, expected_fingerprint.clone())
                    .await
                    .expect_err("reject changed field type");
            assert!(field_type_error
                .to_string()
                .contains("CATALOG_FINGERPRINT_MISMATCH"));
            mt138_minimal_catalog_query(
                &storage,
                "DEFINE FIELD OVERWRITE value ON TABLE atelier_mt138_catalog_probe TYPE string;",
            )
            .await
            .expect("restore field type");

            mt138_minimal_catalog_query(
                &storage,
                "DEFINE INDEX OVERWRITE mt138_catalog_value ON TABLE atelier_mt138_catalog_probe FIELDS marker;",
            )
            .await
            .expect("change existing index columns and uniqueness");
            let index_error = mt138_verify_minimal_catalog(&storage, expected_fingerprint.clone())
                .await
                .expect_err("reject changed index definition");
            assert!(index_error
                .to_string()
                .contains("CATALOG_FINGERPRINT_MISMATCH"));
            mt138_minimal_catalog_query(
                &storage,
                "DEFINE INDEX OVERWRITE mt138_catalog_value ON TABLE atelier_mt138_catalog_probe FIELDS value UNIQUE;",
            )
            .await
            .expect("restore index definition");

            mt138_minimal_catalog_query(
                &storage,
                "DEFINE TABLE OVERWRITE atelier_mt138_catalog_probe SCHEMAFULL PERMISSIONS FULL;",
            )
            .await
            .expect("broaden table permissions");
            let permission_error =
                mt138_verify_minimal_catalog(&storage, expected_fingerprint.clone())
                    .await
                    .expect_err("reject changed table permissions");
            assert!(permission_error
                .to_string()
                .contains("CATALOG_FINGERPRINT_MISMATCH"));
            mt138_minimal_catalog_query(
                &storage,
                "DEFINE TABLE OVERWRITE atelier_mt138_catalog_probe SCHEMAFULL PERMISSIONS NONE;",
            )
            .await
            .expect("restore table permissions");

            mt138_minimal_catalog_query(
                &storage,
                "DEFINE TABLE OVERWRITE atelier_mt138_catalog_view TYPE NORMAL AS \
                     SELECT marker AS value FROM atelier_mt138_catalog_probe PERMISSIONS NONE;",
            )
            .await
            .expect("change view query while retaining view type");
            let view_query_error =
                mt138_verify_minimal_catalog(&storage, expected_fingerprint.clone())
                    .await
                    .expect_err("reject changed view query");
            assert!(view_query_error
                .to_string()
                .contains("CATALOG_FINGERPRINT_MISMATCH"));
            mt138_minimal_catalog_query(
                &storage,
                "DEFINE TABLE OVERWRITE atelier_mt138_catalog_view TYPE NORMAL AS \
                     SELECT `value` FROM atelier_mt138_catalog_probe PERMISSIONS NONE;",
            )
            .await
            .expect("restore view query");

            mt138_minimal_catalog_query(
                &storage,
                "DEFINE EVENT OVERWRITE mt138_catalog_event ON TABLE atelier_mt138_catalog_probe \
                     WHEN $event = 'CREATE' \
                     THEN { DELETE atelier_mt138_catalog_probe WHERE marker = $after.marker; };",
            )
            .await
            .expect("change event condition and action");
            let event_error = mt138_verify_minimal_catalog(&storage, expected_fingerprint.clone())
                .await
                .expect_err("reject changed event definition");
            assert!(event_error
                .to_string()
                .contains("CATALOG_FINGERPRINT_MISMATCH"));
            mt138_minimal_catalog_query(
                &storage,
                "DEFINE EVENT OVERWRITE mt138_catalog_event ON TABLE atelier_mt138_catalog_probe \
                     WHEN $event = 'DELETE' \
                     THEN { DELETE atelier_mt138_catalog_probe WHERE marker = $before.marker; };",
            )
            .await
            .expect("restore event definition");

            mt138_minimal_catalog_query(
                &storage,
                "DEFINE SEQUENCE OVERWRITE atelier_mt138_catalog_seq BATCH 2 START 1;",
            )
            .await
            .expect("change sequence definition");
            let sequence_definition_error =
                mt138_verify_minimal_catalog(&storage, expected_fingerprint.clone())
                    .await
                    .expect_err("reject changed sequence definition");
            assert!(sequence_definition_error
                .to_string()
                .contains("CATALOG_FINGERPRINT_MISMATCH"));
            mt138_minimal_catalog_query(
                &storage,
                "DEFINE SEQUENCE OVERWRITE atelier_mt138_catalog_seq BATCH 1 START 1;",
            )
            .await
            .expect("restore sequence definition");

            mt138_minimal_catalog_query(&storage, "REMOVE SEQUENCE atelier_mt138_catalog_seq;")
                .await
                .expect("remove required sequence");
            let missing_sequence_error =
                mt138_verify_minimal_catalog(&storage, expected_fingerprint.clone())
                    .await
                    .expect_err("reject missing required sequence");
            assert!(missing_sequence_error
                .to_string()
                .contains("SEQUENCE_SET_MISMATCH"));
            mt138_minimal_catalog_query(
                &storage,
                "DEFINE SEQUENCE OVERWRITE atelier_mt138_catalog_seq BATCH 1 START 1;",
            )
            .await
            .expect("restore required sequence");

            mt138_verify_minimal_catalog(&storage, expected_fingerprint)
                .await
                .expect("accept fully restored catalog");
            storage
                .shutdown()
                .await
                .expect("close MT-138 minimal catalog store");
        })
        .await
        .expect("MT-138 minimal real-Rocks catalog proof exceeded five minutes");
    }

    #[tokio::test]
    async fn mt138_canonical_atelier_catalog_fingerprint_matches_compiled_pin() {
        tokio::time::timeout(std::time::Duration::from_secs(120), async {
            let first = mt138_canonical_mem_fingerprint(true)
                .await
                .expect("generate first fresh canonical Atelier fingerprint");
            let second = mt138_canonical_mem_fingerprint(false)
                .await
                .expect("generate second fresh canonical Atelier fingerprint");
            assert_eq!(
                first, second,
                "fresh canonical stores must normalize identically"
            );
            assert_eq!(
                first, EXPECTED_ATELIER_CATALOG_SHA256,
                "compiled Atelier fingerprint pin must match a fresh canonical catalog"
            );
            eprintln!("EXPECTED_ATELIER_CATALOG_SHA256={first}");
        })
        .await
        .expect("MT-138 canonical fingerprint generation exceeded two minutes");
    }

    #[tokio::test]
    async fn mt138_full_schema_atelier_noop_matches_bounded_projection() {
        tokio::time::timeout(std::time::Duration::from_secs(300), async {
            let directory =
                tempfile::tempdir().expect("create full-schema Atelier parity directory");
            let storage = open_test_storage(&directory)
                .await
                .expect("open full-schema Atelier parity store");
            bootstrap_schema(&storage)
                .await
                .expect("apply canonical full schema before Atelier bootstrap");

            let expected_tables = atelier_expected_catalog()
                .into_keys()
                .collect::<BTreeSet<_>>();
            let expected_sequences = ATELIER_REQUIRED_SEQUENCES
                .iter()
                .map(|name| (*name).to_owned())
                .collect::<BTreeSet<_>>();
            let full_schema_fingerprint = storage
                .with_admin_operation(move |database| {
                    Box::pin(async move {
                        inspect_atelier_catalog_fingerprint(
                            &database,
                            &expected_tables,
                            &expected_sequences,
                        )
                        .await
                    })
                })
                .await
                .expect("inspect full-schema Atelier catalog");
            let bounded_projection_fingerprint = mt138_canonical_mem_fingerprint(false)
                .await
                .expect("inspect bounded Atelier projection catalog");
            eprintln!("FULL_SCHEMA_ATELIER_CATALOG_SHA256={full_schema_fingerprint}");
            eprintln!("BOUNDED_ATELIER_CATALOG_SHA256={bounded_projection_fingerprint}");
            assert_eq!(
                full_schema_fingerprint, bounded_projection_fingerprint,
                "full-schema no-op and bounded Atelier bootstrap must have identical catalogs"
            );

            let applied = bootstrap_atelier_schema(&storage)
                .await
                .expect("full canonical schema must already satisfy Atelier bootstrap");
            assert!(
                !applied,
                "Atelier bootstrap must be a no-op after the canonical full schema"
            );

            storage
                .shutdown()
                .await
                .expect("close full-schema Atelier parity store");
        })
        .await
        .expect("MT-138 full-schema Atelier parity exceeded five minutes");
    }

    /// Canonical STRUCTURE catalog of one live store, keyed like `inspect_schema` hashes it,
    /// so a fingerprint mismatch can be explained entry by entry instead of hash by hash.
    async fn canonical_catalog(
        database: &SurrealAdminContext<'_>,
    ) -> Result<BTreeMap<String, String>, SurrealStorageError> {
        let mut entries = BTreeMap::new();
        let mut db_info_response = database.query("INFO FOR DB STRUCTURE;").await?;
        let db_info: SurrealValueData = db_info_response.take(0)?;
        let mut table_names = parse_named_array(&db_info, "tables")
            .unwrap_or_else(|reason| panic!("invalid DB INFO: {reason}"));
        table_names.sort();
        entries.insert(
            "database".to_owned(),
            serde_json::to_string(&strip_nested_table_catalog_ids(db_info))
                .expect("db info serializes (same stripping as canonical_catalog_fingerprint)"),
        );
        for table in table_names {
            let mut response = database
                .query(format!("INFO FOR TABLE `{table}` STRUCTURE;"))
                .await?;
            let info: SurrealValueData = response.take(0)?;
            entries.insert(
                format!("table:{table}"),
                serde_json::to_string(&strip_nested_table_catalog_ids(info))
                    .expect("table info serializes"),
            );
        }
        Ok(entries)
    }

    /// Fresh in-memory apply of the current script: the reference every lineage must reach.
    async fn fresh_mem_catalog() -> BTreeMap<String, String> {
        let client = Surreal::new::<Mem>(()).await.expect("open memory store");
        client
            .use_ns(DEFAULT_NAMESPACE)
            .use_db(DEFAULT_DATABASE)
            .await
            .expect("select memory context");
        let database = SurrealAdminContext { client: &client };
        database
            .query_bound(
                SCHEMA,
                BootstrapBindings {
                    schema_version: SCHEMA_VERSION.to_owned(),
                    schema_revision: SCHEMA_REVISION,
                    namespace: DEFAULT_NAMESPACE.to_owned(),
                    database: DEFAULT_DATABASE.to_owned(),
                    source_manifest_sha256: SCHEMA_LINEAGE_SHA256.to_owned(),
                    generated_surql_sha256: GENERATED_SURREALQL_SHA256.to_owned(),
                },
            )
            .await
            .expect("apply current schema in memory");
        canonical_catalog(&database)
            .await
            .expect("inspect fresh memory catalog")
    }

    /// Prints every catalog entry that differs from the fresh reference.
    fn report_catalog_drift(
        label: &str,
        reference: &BTreeMap<String, String>,
        observed: &BTreeMap<String, String>,
    ) {
        for (key, expected) in reference {
            match observed.get(key) {
                Some(actual) if actual == expected => {}
                Some(actual) => {
                    eprintln!("{label} DRIFT {key}\n  fresh:    {expected}\n  observed: {actual}")
                }
                None => eprintln!("{label} MISSING {key}"),
            }
        }
        for key in observed.keys() {
            if !reference.contains_key(key) {
                eprintln!("{label} EXTRA {key}");
            }
        }
    }

    async fn mt109_schema_rows(
        storage: &SurrealStorage,
        with_authority: bool,
    ) -> Result<Vec<SurrealValueData>, SurrealStorageError> {
        storage
            .with_admin_operation(move |database| {
                Box::pin(async move {
                    let tables = if with_authority {
                        vec![
                            "local_accounts",
                            "principals",
                            "access_spaces",
                            "authenticated_sessions",
                            "session_exchange_credentials",
                            "protected_resources",
                            "resource_grants",
                            "authorization_audit_events",
                            "kernel_event_ledger",
                        ]
                    } else {
                        vec!["kernel_event_ledger"]
                    };
                    let mut rows = Vec::new();
                    for table in tables {
                        let mut result = database
                            .query(format!("SELECT * FROM {table} ORDER BY id;"))
                            .await?;
                        rows.push(result.take::<SurrealValueData>(0)?);
                    }
                    Ok(rows)
                })
            })
            .await
    }

    fn copy_mt109_migration_store(
        source: &std::path::Path,
        destination: &std::path::Path,
    ) -> std::io::Result<()> {
        for entry in std::fs::read_dir(source)? {
            let entry = entry?;
            let target = destination.join(entry.file_name());
            if entry.file_type()?.is_dir() {
                std::fs::create_dir_all(&target)?;
                copy_mt109_migration_store(&entry.path(), &target)?;
            } else {
                std::fs::copy(entry.path(), target)?;
            }
        }
        Ok(())
    }

    #[tokio::test]
    async fn mt109_exact_predecessor_migration_preserves_rows_keys_and_rolls_back() {
        use crate::storage::surreal::resource_authority::{
            AuthorizationRequest, ResourceAction, ResourceGrantSpec, ResourceKind, SigninParams,
        };
        use surrealdb::opt::auth::Record;
        for with_authority in [false, true] {
            let directory = tempfile::Builder::new()
                .prefix("mt109-migration-")
                .tempdir()
                .expect("migration proof directory");
            let rollback_directory = tempfile::Builder::new()
                .prefix("mt109-migration-rollback-")
                .tempdir()
                .expect("rollback migration proof directory");
            let directory_ref = &directory;
            let rollback_directory_ref = &rollback_directory;
            let storage = open_test_storage(directory_ref)
                .await
                .expect("open migration predecessor");
            let body = futures::FutureExt::catch_unwind(std::panic::AssertUnwindSafe(async move {
                storage.with_admin_operation(|database| Box::pin(async move {
                    database.query_bound(PRE_MT109_SCHEMA, BootstrapBindings {
                        schema_version: SCHEMA_VERSION.to_owned(), schema_revision: PRE_ACCOUNT_SETUP_REVISION,
                        namespace: DEFAULT_NAMESPACE.to_owned(), database: DEFAULT_DATABASE.to_owned(),
                        source_manifest_sha256: SCHEMA_LINEAGE_SHA256.to_owned(),
                        generated_surql_sha256: PRE_MT109_GENERATED_SURREALQL_SHA256.to_owned(),
                    }).await?;
                    ensure_knowledge_schema_registry(&database).await?;
                    let observed = read_schema_catalog(&database).await?;
                    assert_eq!(observed.info_fingerprint_sha256, PRE_MT109_SCHEMA_INFO_SHA256);
                    database.query(format!("UPDATE handshake_schema_state:primary SET apply_state = 'complete', info_fingerprint_sha256 = '{PRE_MT109_SCHEMA_INFO_SHA256}';")).await?;
                    if with_authority { database.query(PRE_MT109_AUTHORITY_SCHEMA).await?; }
                    Ok(())
                })).await?;
                let mut old_jwt = None;
                let mut session_id = None;
                if with_authority {
                    let capabilities = vec!["fr.read".to_owned(), "fr.ingest.native_editor".to_owned()];
                    let principal = storage.provision_principal("migration-account", "migration-principal", "human_account", "migration-actor", "Operator", &capabilities, "migration-space", None, std::time::Duration::from_secs(3600)).await?;
                    let resource = storage.register_protected_resource(&principal.identity, ResourceKind::FlightRecorder, "migration-workspace", None, "account_private").await?;
                    storage.grant_resource(&principal.identity.account_id, &principal.identity.access_space_id, ResourceGrantSpec {
                        principal_id: principal.identity.principal_id.clone(), resource_id: resource.resource_id,
                        actions: vec![ResourceAction::Read, ResourceAction::Create], capability_ids: capabilities,
                        expires_at: None, delegation_chain: vec![],
                    }).await?;
                    storage.provision_session_credential(&principal.identity, std::time::Duration::from_secs(3600)).await?;
                    storage.authorize_protected_resource(AuthorizationRequest {
                        session_token: principal.session.token.clone(), channel_binding_hash: None, capability_id: "fr.read".to_owned(),
                        resource_kind: ResourceKind::FlightRecorder, external_resource_id: "migration-workspace".to_owned(), action: ResourceAction::Read,
                    }).await?;
                    session_id = Some(principal.session.session_id.clone());
                    let token_hash = sha256_hex(principal.session.token.as_bytes());
                    old_jwt = Some(storage.with_lease(move |client| Box::pin(async move {
                        let ordinary = client.clone();
                        ordinary.use_ns(DEFAULT_NAMESPACE).use_db(DEFAULT_DATABASE).await?;
                        Ok(ordinary.signin(Record { namespace: DEFAULT_NAMESPACE.to_owned(), database: DEFAULT_DATABASE.to_owned(), access: "authenticated_session".to_owned(), params: SigninParams { token_hash, channel_binding_hash: None } }).await?)
                    })).await?);
                }
                for ordinal in 0..2 {
                    let event = crate::kernel::NewKernelEvent::builder("migration-task", "migration-session", crate::kernel::KernelEventType::ArtifactStored, crate::kernel::KernelActor::System("migration-proof".to_owned()))
                        .aggregate("migration", "migration-aggregate").idempotency_key(format!("migration-{ordinal}"))
                        .source_component("mt109-migration-proof").payload(serde_json::json!({"ordinal": ordinal})).build()?;
                    let (_, write) = super::super::event_ledger::prepare_event(event)?;
                    storage.with_admin_operation(move |database| Box::pin(async move {
                        let authority_fields = if with_authority { ", wsids: $wsids, authority_resource_id: $authority_resource_id, authority_session_id: $authority_session_id, authority_capability_id: $authority_capability_id, authority_action: $authority_action" } else { "" };
                        database.query_bound(format!("CREATE $record CONTENT {{ event_id: $event_id, event_version: $event_version, kernel_task_run_id: $kernel_task_run_id, session_run_id: $session_run_id, aggregate_type: $aggregate_type, aggregate_id: $aggregate_id, idempotency_key: $idempotency_key, event_type: $event_type, actor_kind: $actor_kind, actor_id: $actor_id, causation_id: $causation_id, correlation_id: $correlation_id, payload_hash: $payload_hash, source_component: $source_component, payload: $payload, created_at: $created_at{authority_fields} }} RETURN AFTER;"), write).await?;
                        Ok(())
                    })).await?;
                }
                let before = mt109_schema_rows(&storage, with_authority).await?;
                let (before_state, before_catalog) = storage.with_admin_operation(|database| Box::pin(async move {
                    Ok((read_context_and_state(&database).await?.expect("old receipt"), read_schema_catalog(&database).await?.info_fingerprint_sha256))
                })).await?;
                assert_eq!(before_catalog, if with_authority { PRE_MT109_AUTHORITY_INFO_SHA256 } else { PRE_MT109_SCHEMA_INFO_SHA256 });
                let mut malformed = before_state.clone();
                malformed.target_revision -= 1;
                assert!(!malformed.is_exact_pre_mt109_current());
                malformed = before_state.clone();
                malformed.apply_state = "unknown".to_owned();
                assert!(!malformed.is_exact_pre_mt109_current());
                for (field, assignments) in [("forward-revision", "revision = 158, target_revision = 158"), ("lower-revision", "revision = 156, target_revision = 156"), ("old-incomplete", "apply_state = 'schema_applied'")] {
                    storage.with_admin_operation(move |database| Box::pin(async move {
                        database.query(format!("UPDATE handshake_schema_state:primary SET {assignments};")).await?;
                        Ok(())
                    })).await?;
                    let rejected_state = storage.with_admin_operation(|database| Box::pin(async move {
                        Ok(read_context_and_state(&database).await?.expect("changed predecessor receipt"))
                    })).await?;
                    let rejection = bootstrap_schema(&storage).await.expect_err("changed predecessor state must fail closed");
                    assert!(rejection.to_string().contains("HANDSHAKE_SURREAL_SCHEMA_UNSUPPORTED_LINEAGE"), "changed {field} returned the wrong rejection: {rejection}");
                    assert_eq!(mt109_schema_rows(&storage, with_authority).await?, before);
                    storage.with_admin_operation(|database| Box::pin(async move {
                        assert_eq!(read_context_and_state(&database).await?.expect("rejected receipt"), rejected_state);
                        database.query("UPDATE handshake_schema_state:primary SET revision = 157, target_revision = 157, apply_state = 'complete';").await?;
                        Ok(())
                    })).await?;
                }
                storage.with_admin_operation(|database| Box::pin(async move {
                    database.query("DEFINE TABLE mt109_unknown_overlay SCHEMAFULL PERMISSIONS NONE;").await?;
                    Ok(())
                })).await?;
                let rejected_catalog = storage.with_admin_operation(|database| Box::pin(async move {
                    Ok(read_schema_catalog(&database).await?.info_fingerprint_sha256)
                })).await?;
                let rejection = bootstrap_schema(&storage).await.expect_err("unknown overlay must fail before DDL");
                assert!(rejection.to_string().contains("HANDSHAKE_SURREAL_PRE_MT109_CATALOG_MISMATCH"));
                assert_eq!(mt109_schema_rows(&storage, with_authority).await?, before);
                storage.with_admin_operation(|database| Box::pin(async move {
                    assert_eq!(read_schema_catalog(&database).await?.info_fingerprint_sha256, rejected_catalog);
                    database.query("REMOVE TABLE mt109_unknown_overlay;").await?;
                    Ok(())
                })).await?;
                let before_marker = storage.with_admin_operation(|database| Box::pin(async move {
                    let mut rows = database.query("SELECT * FROM ONLY handshake_schema_state:primary;").await?;
                    Ok(rows.take::<SurrealValueData>(0)?)
                })).await?;
                storage.shutdown().await?;
                drop(storage);
                copy_mt109_migration_store(
                    directory_ref.path(),
                    rollback_directory_ref.path(),
                )?;

                let upgraded = open_test_storage(directory_ref).await?;
                let upgrade = bootstrap_schema(&upgraded).await?;
                assert_eq!(upgrade.outcome, SchemaBootstrapOutcome::UpgradedSupportedPredecessor);
                assert_eq!(upgrade.info_fingerprint_sha256, EXPECTED_SCHEMA_INFO_SHA256);
                assert_eq!(mt109_schema_rows(&upgraded, with_authority).await?, before);
                upgraded.shutdown().await?;
                drop(upgraded);

                let current = open_test_storage(directory_ref).await?;
                let replay = bootstrap_schema(&current).await?;
                assert_eq!(replay.outcome, SchemaBootstrapOutcome::ReusedExactCurrent);
                assert_eq!(mt109_schema_rows(&current, with_authority).await?, before);
                if let Some(jwt) = old_jwt {
                    let expected_session = session_id.expect("old session identity");
                    current.with_lease(move |client| Box::pin(async move {
                        let ordinary = client.clone();
                        ordinary.use_ns(DEFAULT_NAMESPACE).use_db(DEFAULT_DATABASE).await?;
                        ordinary.authenticate(jwt).await?;
                        let mut response = ordinary.query("RETURN record::id($auth.id);").await?.check()?;
                        let observed: Option<String> = response.take(0)?;
                        assert_eq!(observed, Some(expected_session), "old JWT must retain its exact session after migration and reopen");
                        Ok(())
                    })).await?;
                }

                let rollback = open_test_storage(rollback_directory_ref).await?;
                let rollback_errors = rollback.with_admin_operation(|database| Box::pin(async move {
                    let injected = mt109_authority_upgrade_query().replace("UPDATE ONLY handshake_schema_state:primary SET", "THROW 'MT109_INJECTED_AUTHORITY_ROLLBACK'; UPDATE ONLY handshake_schema_state:primary SET");
                    let mut result = database.client.query(injected)
                        .bind(SurrealValue::into_value(mt109_authority_upgrade_bindings())).await?;
                    let mut errors = result.take_errors().into_iter().map(|(index, error)| (index, error.to_string())).collect::<Vec<_>>();
                    errors.sort_by_key(|(index, _)| *index);
                    Ok(errors)
                })).await?;
                let mut rollback_counts = std::collections::BTreeMap::new();
                for (_, error) in &rollback_errors {
                    *rollback_counts.entry(error.as_str()).or_insert(0usize) += 1;
                }
                eprintln!("MT109_ROLLBACK_ERROR_COUNTS {rollback_counts:?}");
                eprintln!(
                    "MT109_ROLLBACK_PRIMARY_ERRORS {:?}",
                    rollback_errors
                        .iter()
                        .filter(|(_, error)| {
                            error != "The query was not executed due to a failed transaction"
                        })
                        .collect::<Vec<_>>()
                );
                // Response index 176 (before MT-154) is the injected THROW immediately before the schema-state
                // UPDATE: three transaction/precondition statements, 144 authority statements,
                // two Loom backfill statements, the two MT-150 `loom_edges` receipt DDL
                // statements and the 25 MT-141 DDL statements (provenance-ref `asset_id` line, the
                // saved-search retrieval projection block and the six V2-R2 field lines) precede
                // it. The remaining tail statements must be cancelled.
                let primary_errors = rollback_errors
                    .iter()
                    .filter(|(_, error)| {
                        error != "The query was not executed due to a failed transaction"
                    })
                    .map(|(index, error)| (*index, error.as_str()))
                    .collect::<Vec<_>>();
                // Derived, not hard-coded (MT-154: schema_delta_upgrade_statements re-emits every
                // MT-154 delta inside this transaction, so the absolute index moves with each schema
                // batch). Only the schema-state UPDATE (cancelled) and the COMMIT may follow the
                // THROW: it must be the third-to-last reported result.
                let injected_index = rollback_errors
                    .iter()
                    .find(|(_, error)| error == "An error occurred: MT109_INJECTED_AUTHORITY_ROLLBACK")
                    .map(|(index, _)| *index)
                    .expect("injected rollback THROW must be reported");
                assert_eq!(
                    rollback_errors.last().map(|(index, _)| *index),
                    Some(injected_index + 2),
                    "the injected THROW must sit immediately before the schema-state UPDATE and COMMIT"
                );
                assert_eq!(
                    primary_errors,
                    vec![
                        (injected_index, "An error occurred: MT109_INJECTED_AUTHORITY_ROLLBACK"),
                        (injected_index + 1, "The query was not executed due to a cancelled transaction"),
                        (injected_index + 2, "Cannot COMMIT: the transaction was aborted due to a prior error"),
                    ],
                    "injected rollback did not fail at the exact pre-marker-update statement"
                );
                assert!(rollback_errors.iter().all(|(_, error)| matches!(error.as_str(),
                    "An error occurred: MT109_INJECTED_AUTHORITY_ROLLBACK"
                    | "The query was not executed due to a failed transaction"
                    | "The query was not executed due to a cancelled transaction"
                    | "Cannot COMMIT: the transaction was aborted due to a prior error"
                )), "unrelated transaction failure: {rollback_counts:?}");
                rollback.shutdown().await?;
                drop(rollback);

                let rollback_reopened = open_test_storage(rollback_directory_ref).await?;
                assert_eq!(mt109_schema_rows(&rollback_reopened, with_authority).await?, before);
                rollback_reopened.with_admin_operation(|database| Box::pin(async move {
                    let mut marker = database.query("SELECT * FROM ONLY handshake_schema_state:primary;").await?;
                    assert_eq!(marker.take::<SurrealValueData>(0)?, before_marker, "rollback changed complete marker including timestamps");
                    assert_eq!(read_schema_catalog(&database).await?.info_fingerprint_sha256, before_catalog);
                    assert_eq!(read_context_and_state(&database).await?.expect("rollback receipt"), before_state);
                    Ok(())
                })).await?;
                Ok::<(SurrealStorage, SurrealStorage), Box<dyn std::error::Error>>((
                    current,
                    rollback_reopened,
                ))
            })).await;
            let (current, rollback_reopened) = match body {
                Ok(Ok(handles)) => handles,
                Ok(Err(error)) => {
                    let path = directory.keep();
                    let rollback_path = rollback_directory.keep();
                    panic!(
                        "migration proof failed: {error}; stores preserved at {} and {}",
                        path.display(),
                        rollback_path.display()
                    );
                }
                Err(panic) => {
                    eprintln!(
                        "MT109_MIGRATION_STORES_PRESERVED {} {}",
                        directory.path().display(),
                        rollback_directory.path().display()
                    );
                    let _ = directory.keep();
                    let _ = rollback_directory.keep();
                    std::panic::resume_unwind(panic);
                }
            };
            let path = directory.keep();
            crate::storage::tests::shutdown_and_remove_test_store(current, path.clone())
                .await
                .expect("remove closed migration store through centralized cleanup");
            assert!(!path.exists());
            eprintln!("MT109_MIGRATION_STORE_REMOVED {}", path.display());
            let rollback_path = rollback_directory.keep();
            crate::storage::tests::shutdown_and_remove_test_store(
                rollback_reopened,
                rollback_path.clone(),
            )
            .await
            .expect("remove closed rollback migration store through centralized cleanup");
            assert!(!rollback_path.exists());
            eprintln!(
                "MT109_ROLLBACK_MIGRATION_STORE_REMOVED {}",
                rollback_path.display()
            );
        }
    }

    #[tokio::test]
    async fn mt109_loom_catalog_dependencies_are_complete_and_deterministic() {
        let tables = loom_receipt_test_tables()
            .iter()
            .map(|s| (*s).to_owned())
            .collect::<BTreeSet<_>>();
        let sequences = loom_receipt_test_sequences()
            .iter()
            .map(|s| (*s).to_owned())
            .collect::<BTreeSet<_>>();
        assert_eq!(tables.len(), 17);
        let mut fingerprints = Vec::new();
        for _ in 0..2 {
            let client = Surreal::new::<Mem>(())
                .await
                .expect("open bounded Loom catalog");
            client
                .use_ns(DEFAULT_NAMESPACE)
                .use_db(DEFAULT_DATABASE)
                .await
                .expect("select bounded context");
            let database = SurrealAdminContext { client: &client };
            database
                .query(loom_receipt_test_schema_ddl())
                .await
                .expect("apply canonical bounded Loom dependencies");
            let fingerprint = inspect_catalog_fingerprint(
                &database,
                &tables,
                &sequences,
                CatalogInspectionScope::ExactDatabase,
            )
            .await
            .expect("inspect complete bounded Loom catalog");
            database
                .query("REMOVE FUNCTION fn::mt109_source_read;")
                .await
                .expect("remove exact source dependency");
            let error = inspect_catalog_fingerprint(
                &database,
                &tables,
                &sequences,
                CatalogInspectionScope::ExactDatabase,
            )
            .await
            .expect_err("missing source dependency must reject");
            assert!(
                error
                    .to_string()
                    .contains("HANDSHAKE_LOOM_SCHEMA_AUTHORITY_SET_MISMATCH"),
                "{error}"
            );
            fingerprints.push(fingerprint);
        }
        assert_eq!(fingerprints[0], fingerprints[1]);
        eprintln!("MT109_LOOM_CATALOG_SHA256={}", fingerprints[0]);
        assert_eq!(
            fingerprints[0],
            "efc3ecc6ceea3e2a2cb0b1deb717c48a3d79fe8ff7668a17b06fc8f2f10e61b7"
        );
    }

    #[tokio::test]
    async fn mt109_authority_catalog_pins_are_deterministic() {
        assert_eq!(
            sha256_hex(PRE_MT109_SCHEMA.as_bytes()),
            PRE_MT109_GENERATED_SURREALQL_SHA256
        );
        let mut current_pins = Vec::new();
        let mut predecessor_pins = Vec::new();
        for current in [false, true] {
            for _ in 0..2 {
                let client = Surreal::new::<Mem>(())
                    .await
                    .expect("open isolated catalog store");
                client
                    .use_ns(DEFAULT_NAMESPACE)
                    .use_db(DEFAULT_DATABASE)
                    .await
                    .expect("select catalog context");
                let database = SurrealAdminContext { client: &client };
                database
                    .query_bound(
                        if current { SCHEMA } else { PRE_MT109_SCHEMA },
                        BootstrapBindings {
                            schema_version: SCHEMA_VERSION.to_owned(),
                            schema_revision: if current {
                                SCHEMA_REVISION
                            } else {
                                PRE_ACCOUNT_SETUP_REVISION
                            },
                            namespace: DEFAULT_NAMESPACE.to_owned(),
                            database: DEFAULT_DATABASE.to_owned(),
                            source_manifest_sha256: SCHEMA_LINEAGE_SHA256.to_owned(),
                            generated_surql_sha256: if current {
                                GENERATED_SURREALQL_SHA256
                            } else {
                                PRE_MT109_GENERATED_SURREALQL_SHA256
                            }
                            .to_owned(),
                        },
                    )
                    .await
                    .expect("apply catalog base");
                if !current {
                    database
                        .query(PRE_MT109_AUTHORITY_SCHEMA)
                        .await
                        .expect("apply committed predecessor authority");
                }
                let observed = if current {
                    inspect_schema(&database).await
                } else {
                    read_schema_catalog(&database).await
                }
                .expect("inspect complete authority catalog");
                if current {
                    current_pins.push(observed.info_fingerprint_sha256);
                } else {
                    predecessor_pins.push(observed.info_fingerprint_sha256);
                }
            }
        }
        assert_eq!(
            current_pins[0], current_pins[1],
            "fresh current catalog is nondeterministic"
        );
        assert_eq!(
            predecessor_pins[0], predecessor_pins[1],
            "committed overlay catalog is nondeterministic"
        );
        eprintln!("MT109_CURRENT_AUTHORITY_INFO_SHA256={}", current_pins[0]);
        eprintln!(
            "MT109_PREDECESSOR_AUTHORITY_INFO_SHA256={}",
            predecessor_pins[0]
        );
        assert_eq!(current_pins[0], EXPECTED_SCHEMA_INFO_SHA256);
        assert_eq!(predecessor_pins[0], PRE_MT109_AUTHORITY_INFO_SHA256);
    }

    #[tokio::test]
    async fn mt139_current_schema_info_pin_matches_fresh_mem_catalog() {
        let client = Surreal::new::<Mem>(()).await.expect("open memory store");
        client
            .use_ns(DEFAULT_NAMESPACE)
            .use_db(DEFAULT_DATABASE)
            .await
            .expect("select memory context");
        let database = SurrealAdminContext { client: &client };
        database
            .query_bound(
                SCHEMA,
                BootstrapBindings {
                    schema_version: SCHEMA_VERSION.to_owned(),
                    schema_revision: SCHEMA_REVISION,
                    namespace: DEFAULT_NAMESPACE.to_owned(),
                    database: DEFAULT_DATABASE.to_owned(),
                    source_manifest_sha256: SCHEMA_LINEAGE_SHA256.to_owned(),
                    generated_surql_sha256: GENERATED_SURREALQL_SHA256.to_owned(),
                },
            )
            .await
            .expect("apply current schema in memory");
        let observed = inspect_schema(&database)
            .await
            .expect("inspect current memory schema");
        eprintln!(
            "MT139_CURRENT_SCHEMA_INFO_SHA256={}",
            observed.info_fingerprint_sha256
        );
        assert_eq!(
            observed.info_fingerprint_sha256,
            EXPECTED_SCHEMA_INFO_SHA256
        );
    }

    #[tokio::test]
    async fn mt138_bounded_bootstrap_transaction_rolls_back_on_failure() {
        let directory = tempfile::tempdir().expect("create MT-138 atomicity directory");
        let storage = open_test_storage(&directory)
            .await
            .expect("open MT-138 atomicity store");
        let result: Result<(), SurrealStorageError> = storage
            .with_admin_operation(|database| {
                Box::pin(async move {
                    database
                        .query(
                            "BEGIN TRANSACTION; \
                             DEFINE TABLE OVERWRITE atelier_mt138_atomic_probe SCHEMAFULL; \
                             THROW 'MT138_INJECTED_BOOTSTRAP_FAILURE'; \
                             COMMIT TRANSACTION;",
                        )
                        .await?;
                    Ok(())
                })
            })
            .await;
        assert!(result.is_err());
        let tables: Vec<String> = storage
            .with_data_operation(|ctx| {
                Box::pin(async move {
                    ctx.query_values(
                        "RETURN array::sort(object::keys((INFO FOR DB).tables));",
                        (),
                    )
                    .await
                })
            })
            .await
            .expect("inspect MT-138 atomic rollback");
        assert!(!tables
            .iter()
            .any(|table| table == "atelier_mt138_atomic_probe"));
        storage.shutdown().await.expect("close atomicity store");
    }

    #[tokio::test]
    async fn mt138_structured_info_reports_reserved_value_field() {
        let directory = tempfile::tempdir().expect("create MT-138 field-info directory");
        let storage = open_test_storage(&directory)
            .await
            .expect("open MT-138 field-info store");
        let (fields, definitions) = storage
            .with_admin_operation(|database| {
                Box::pin(async move {
                    database
                        .query(
                            "DEFINE TABLE OVERWRITE atelier_mt138_field_probe SCHEMAFULL PERMISSIONS NONE; \
                             DEFINE FIELD OVERWRITE value ON TABLE atelier_mt138_field_probe TYPE string; \
                             DEFINE TABLE OVERWRITE atelier_mt138_view_probe TYPE NORMAL AS \
                                 SELECT `value` FROM atelier_mt138_field_probe PERMISSIONS NONE;",
                        )
                        .await?;
                    let mut response = database
                        .query("INFO FOR TABLE atelier_mt138_field_probe STRUCTURE;")
                        .await?;
                    let info: SurrealValueData = response.take(0)?;
                    let fields = parse_named_array(&info, "fields")
                        .unwrap_or_else(|reason| panic!("invalid field INFO: {reason}"));
                    let mut response = database.query("INFO FOR DB STRUCTURE;").await?;
                    let database_info: SurrealValueData = response.take(0)?;
                    let definitions = parse_table_definitions(&database_info)
                        .unwrap_or_else(|reason| panic!("invalid table INFO: {reason}"));
                    Ok((fields, definitions))
                })
            })
            .await
            .expect("read structured field catalog");

        assert!(
            fields.iter().any(|field| field == "value"),
            "structured INFO omitted the reserved-name field: {fields:?}"
        );
        assert_eq!(
            definitions.get("atelier_mt138_field_probe"),
            Some(&AtelierTableDefinition {
                schemafull: true,
                kind: "NORMAL".to_owned(),
                is_view: false,
            })
        );
        assert_eq!(
            definitions.get("atelier_mt138_view_probe"),
            Some(&AtelierTableDefinition {
                schemafull: false,
                kind: "NORMAL".to_owned(),
                is_view: true,
            })
        );
        storage.shutdown().await.expect("close field-info store");
    }

    async fn index_names(
        storage: &SurrealStorage,
        table: &'static str,
    ) -> Result<Vec<String>, SurrealStorageError> {
        storage
            .with_admin_operation(|database| {
                Box::pin(async move {
                    let mut response = database
                        .query(format!("INFO FOR TABLE `{table}` STRUCTURE;"))
                        .await?;
                    let info: SurrealValueData = response.take(0)?;
                    let mut names = parse_named_array(&info, "indexes")
                        .unwrap_or_else(|reason| panic!("invalid index INFO: {reason}"));
                    names.sort();
                    Ok(names)
                })
            })
            .await
    }

    #[test]
    fn declarative_schema_catalog_is_complete_and_content_sensitive() {
        let entries = compiled_schema_catalog_entries().expect("parse declarative schema catalog");
        assert_eq!(
            compute_catalog_hash(&entries),
            DECLARATIVE_SCHEMA_CATALOG_SHA256
        );
        assert_eq!(compute_generated_surql_sha256(), GENERATED_SURREALQL_SHA256);
        assert_eq!(
            compute_knowledge_schema_registry_seed_sha256(),
            KNOWLEDGE_SCHEMA_REGISTRY_SEED_SHA256
        );
        assert_eq!(
            entries
                .iter()
                .filter(|entry| entry.starts_with("table:"))
                .count(),
            TABLE_DEFINITION_COUNT
        );
        assert_eq!(
            entries
                .iter()
                .filter(|entry| entry.starts_with("field:"))
                .count(),
            AUTHORED_FIELD_DEFINITION_COUNT
        );
        assert_eq!(
            entries
                .iter()
                .filter(|entry| entry.starts_with("index:"))
                .count(),
            INDEX_DEFINITION_COUNT
        );
        assert_eq!(
            entries
                .iter()
                .filter(|entry| entry.starts_with("event:"))
                .count(),
            EVENT_DEFINITION_COUNT
        );
        assert_eq!(
            entries
                .iter()
                .filter(|entry| entry.starts_with("view:"))
                .count(),
            VIEW_DEFINITION_COUNT
        );
        assert_eq!(
            entries
                .iter()
                .filter(|entry| entry.starts_with("sequence:"))
                .count(),
            SEQUENCE_DEFINITION_COUNT
        );

        for (prefix, expected) in [
            ("access:", ACCESS_DEFINITION_COUNT),
            ("function:", FUNCTION_DEFINITION_COUNT),
        ] {
            assert_eq!(
                entries
                    .iter()
                    .filter(|entry| entry.starts_with(prefix))
                    .count(),
                expected
            );
        }
        assert!(entries.contains(&"access:authenticated_session".to_owned()));
        assert_eq!(
            declarative_function_name("fn::mt109_live_session()"),
            Ok("fn::mt109_live_session")
        );
        assert!(declarative_function_name("fn::missing_opener").is_err());
        assert!(declarative_function_name("fn::()").is_err());
        let mut duplicate = entries.iter().cloned().collect::<BTreeSet<_>>();
        assert!(
            insert_catalog_identity(&mut duplicate, "access:authenticated_session".to_owned())
                .is_err()
        );
        assert!(
            insert_catalog_identity(&mut duplicate, "function:fn::mt109_has_grant".to_owned())
                .is_err()
        );
        assert!(resource_authority_schema_statements()
            .contains("DEFINE ACCESS IF NOT EXISTS authenticated_session"));

        let mut reordered = entries.clone();
        reordered.reverse();
        assert_eq!(
            compute_catalog_hash(&reordered),
            DECLARATIVE_SCHEMA_CATALOG_SHA256
        );

        let mut altered = entries.clone();
        altered.push("table:attacker_rogue".to_owned());
        assert_ne!(
            compute_catalog_hash(&altered),
            DECLARATIVE_SCHEMA_CATALOG_SHA256
        );
        assert_ne!(
            sha256_hex(format!("{SCHEMA}\n").as_bytes()),
            GENERATED_SURREALQL_SHA256
        );
        assert_ne!(
            sha256_hex(format!("{KNOWLEDGE_SCHEMA_REGISTRY_SEED}\n").as_bytes()),
            KNOWLEDGE_SCHEMA_REGISTRY_SEED_SHA256
        );
    }

    #[test]
    fn canonical_info_sorts_named_catalog_entries_but_preserves_index_column_order() {
        let left = serde_json::json!({
            "indexes": [
                { "name": "z", "cols": ["first", "second"] },
                { "name": "a", "cols": ["only"] },
            ]
        });
        let reordered_catalog = serde_json::json!({
            "indexes": [
                { "name": "a", "cols": ["only"] },
                { "name": "z", "cols": ["first", "second"] },
            ]
        });
        let changed_index_order = serde_json::json!({
            "indexes": [
                { "name": "a", "cols": ["only"] },
                { "name": "z", "cols": ["second", "first"] },
            ]
        });

        assert_eq!(
            canonicalize_info(left.clone().into_value()),
            canonicalize_info(reordered_catalog.into_value())
        );
        assert_ne!(
            canonicalize_info(left.into_value()),
            canonicalize_info(changed_index_order.into_value())
        );
    }

    /// MT-142: the upgrade statements duplicate the schema.surql block on purpose (the fresh
    /// script is fresh-only); this pins them byte-for-byte to the declarative authority.
    #[test]
    fn mt142_title_anchor_upgrade_statements_match_schema() {
        let ddl_lines: Vec<&str> = MT142_TITLE_ANCHOR_UPGRADE_STATEMENTS
            .lines()
            .filter(|line| line.starts_with("DEFINE "))
            .collect();
        assert_eq!(ddl_lines.len(), 10);
        let predecessor = pre_account_setup_schema();
        for line in ddl_lines {
            assert!(
                predecessor.lines().any(|schema_line| schema_line == line),
                "MT-142 upgrade DDL drifted from schema.surql: {line}"
            );
        }
        assert!(KNOWLEDGE_SCHEMA_REGISTRY_SEED
            .contains("family_key: 'rich_document_title_anchors', table_name: 'knowledge_rich_document_title_anchors'"));
    }

    /// MT-151: same pin as `mt142_title_anchor_upgrade_statements_match_schema` for the
    /// The exact MT-152 `fems_workspace_write_anchors` block as it appears in `schema.surql`;
    /// removing it from the current script yields the byte-exact MT-151 pin.
    const MT152_FEMS_WRITE_ANCHORS_BLOCK: &str = concat!(
        "\n-- MT-152 fems_workspace_write_anchors: one write anchor per workspace, UPSERTed (with a\n",
        "-- fresh claim_nonce) by every FEMS transaction that creates a row referencing the workspace\n",
        "-- and by the workspace-delete transaction itself (UPSERT then DELETE, so the key is in its\n",
        "-- write set whether or not the row existed). The pinned engine detects conflicts only on\n",
        "-- keys a transaction writes, so a FEMS insert and a workspace delete that overlap in the\n",
        "-- engine now collide at commit instead of both committing with an orphan (MT-146 D-146-1,\n",
        "-- previously ordered only by FEMS_MUTATION_LOCK). Deliberately NOT a record<workspaces>\n",
        "-- reference: the delete removes it explicitly and no cascade scan is involved. A\n",
        "-- serialization device, not domain data.\n",
        "DEFINE TABLE OVERWRITE fems_workspace_write_anchors SCHEMAFULL PERMISSIONS NONE;\n",
        "DEFINE FIELD OVERWRITE anchor_key ON TABLE fems_workspace_write_anchors TYPE string ASSERT $value = record::id($this.id);\n",
        "DEFINE FIELD OVERWRITE workspace_key ON TABLE fems_workspace_write_anchors TYPE string ASSERT string::trim($value) != '';\n",
        "DEFINE FIELD OVERWRITE claim_nonce ON TABLE fems_workspace_write_anchors TYPE string ASSERT string::trim($value) != '';\n",
        "DEFINE FIELD OVERWRITE updated_at ON TABLE fems_workspace_write_anchors TYPE datetime DEFAULT time::now();\n",
        "DEFINE INDEX OVERWRITE pk_fems_workspace_write_anchors ON TABLE fems_workspace_write_anchors FIELDS anchor_key UNIQUE;\n",
    );

    /// The exact MT-152 `loom_folders.sibling_key` field block and index line as they appear in
    /// `schema.surql` (I-152-2 sweep finding); removed with the FEMS block to reach the MT-151 pin.
    const MT152_LOOM_FOLDER_SIBLING_KEY_BLOCK: &str = concat!(
        "-- MT-152 sibling_key: stored discriminator for sibling-name uniqueness covering ROOT folders.\n",
        "-- `uq_loom_folders_sibling_name` cannot: the engine skips uniqueness for any tuple containing\n",
        "-- NONE (surrealdb-core-3.2.0/src/idx/index.rs:190-197, NULL != NULL) and roots carry\n",
        "-- parent_folder_id = NONE. `workspace|parent-or-root|name`, set by every create, rename and\n",
        "-- re-parent write; the in-place upgrade backfills existing rows and disambiguates pre-existing\n",
        "-- root duplicates with a stable '#dup<n>' suffix on this key only (never on the visible name).\n",
        "DEFINE FIELD OVERWRITE sibling_key ON TABLE loom_folders TYPE string ASSERT string::trim($value) != '';\n",
    );
    const MT152_LOOM_FOLDER_SIBLING_KEY_INDEX_LINE: &str =
        "DEFINE INDEX OVERWRITE uq_loom_folders_sibling_key ON TABLE loom_folders FIELDS sibling_key UNIQUE;\n";

    /// The current script minus the MT-152 blocks: the exact MT-151 pin.
    fn mt151_pin_schema() -> String {
        for block in [
            MT152_FEMS_WRITE_ANCHORS_BLOCK,
            MT152_LOOM_FOLDER_SIBLING_KEY_BLOCK,
            MT152_LOOM_FOLDER_SIBLING_KEY_INDEX_LINE,
        ] {
            assert_eq!(
                PRE_MT109_SCHEMA.matches(block).count(),
                1,
                "MT-152 block drifted: {block}"
            );
        }
        let pinned = PRE_MT109_SCHEMA
            .replace(MT152_FEMS_WRITE_ANCHORS_BLOCK, "")
            .replace(MT152_LOOM_FOLDER_SIBLING_KEY_BLOCK, "")
            .replace(MT152_LOOM_FOLDER_SIBLING_KEY_INDEX_LINE, "");
        assert_eq!(
            sha256_hex(pinned.as_bytes()),
            PRE_MT152_GENERATED_SURREALQL_SHA256,
            "the pre-MT-152 allowlist must be exactly the current script minus the MT-152 block"
        );
        pinned
    }

    /// MT-152: the upgrade DDL is byte-identical (whitespace-normalised) to the fresh-script
    /// `fems_workspace_write_anchors` block, the table is in the inventory, and the lineage
    /// pins moved.
    #[test]
    fn mt152_upgrade_statements_match_schema() {
        fn statements(source: &str) -> Vec<String> {
            source
                .lines()
                .filter(|line| !line.trim_start().starts_with("--"))
                .collect::<Vec<_>>()
                .join("\n")
                .split(';')
                .map(|statement| statement.split_whitespace().collect::<Vec<_>>().join(" "))
                .filter(|statement| statement.starts_with("DEFINE "))
                .collect()
        }
        let upgrade = statements(MT152_FEMS_WRITE_ANCHOR_UPGRADE_STATEMENTS);
        assert_eq!(upgrade.len(), 6);
        assert_eq!(upgrade, statements(MT152_FEMS_WRITE_ANCHORS_BLOCK));
        let schema = statements(PRE_MT109_SCHEMA);
        for statement in &upgrade {
            assert!(
                schema
                    .iter()
                    .any(|schema_statement| schema_statement == statement),
                "MT-152 upgrade DDL drifted from schema.surql: {statement}"
            );
        }
        assert!(TABLE_NAMES.contains(&"fems_workspace_write_anchors"));
        assert!(!MT152_FEMS_WRITE_ANCHORS_BLOCK.contains("REFERENCE"));
        assert_ne!(
            PRE_MT152_GENERATED_SURREALQL_SHA256,
            GENERATED_SURREALQL_SHA256
        );
        assert_ne!(PRE_MT152_SCHEMA_INFO_SHA256, EXPECTED_SCHEMA_INFO_SHA256);
        // The MT-152 predecessor is the MT-151 current pin, so the two hops chain.
        assert_ne!(
            PRE_MT152_GENERATED_SURREALQL_SHA256,
            PRE_MT151_GENERATED_SURREALQL_SHA256
        );
        let _ = mt151_pin_schema();
    }

    /// MT-152 (I-152-2): the folder `sibling_key` field and UNIQUE index statements applied by
    /// the upgrade are byte-identical (whitespace-normalised) to `schema.surql`, and the field
    /// precedes the index there (the backfill must be committed before the index builds).
    #[test]
    fn mt152_folder_sibling_key_statements_match_schema() {
        fn statements(source: &str) -> Vec<String> {
            source
                .lines()
                .filter(|line| !line.trim_start().starts_with("--"))
                .collect::<Vec<_>>()
                .join("\n")
                .split(';')
                .map(|statement| statement.split_whitespace().collect::<Vec<_>>().join(" "))
                .filter(|statement| statement.starts_with("DEFINE "))
                .collect()
        }
        let field = statements(MT152_LOOM_FOLDER_SIBLING_KEY_FIELD_STATEMENTS);
        let index = statements(MT152_LOOM_FOLDER_SIBLING_KEY_INDEX_STATEMENTS);
        assert_eq!(field.len(), 1);
        assert_eq!(index.len(), 1);
        assert_eq!(field, statements(MT152_LOOM_FOLDER_SIBLING_KEY_BLOCK));
        assert_eq!(index, statements(MT152_LOOM_FOLDER_SIBLING_KEY_INDEX_LINE));
        let schema = statements(SCHEMA);
        let field_at = schema
            .iter()
            .position(|s| s == &field[0])
            .expect("field in schema");
        let index_at = schema
            .iter()
            .position(|s| s == &index[0])
            .expect("index in schema");
        assert!(
            field_at < index_at,
            "sibling_key field must precede its UNIQUE index"
        );
        assert_eq!(
            super::super::loom_store::loom_folder_sibling_key("ws-1", None, " Root "),
            "v1|w4:ws-1|r|n4:Root"
        );
        assert_eq!(
            super::super::loom_store::loom_folder_sibling_key("ws-1", Some("LFD-p"), "Child"),
            "v1|w4:ws-1|p5:LFD-p|n5:Child"
        );
    }

    /// MT-152 (I-152-2): a store at the exact MT-151 pin holding two ROOT folders with one name
    /// (legal there - the composite index never covered roots) and one nested folder is upgraded
    /// in place: every row gains `sibling_key`, the later duplicate carries the `#dup1` suffix on
    /// the key only, its visible name is untouched, and the UNIQUE index then rejects a third
    /// root with that name through the product path with the same typed conflict as a nested
    /// duplicate.
    #[tokio::test]
    async fn mt152_exact_mt151_pin_upgrade_backfills_folder_sibling_keys_and_rejects_root_duplicates(
    ) {
        let mt151_pin_schema = mt151_pin_schema();
        let directory = tempfile::tempdir().expect("temporary MT-151-pin store");
        let storage = open_test_storage(&directory)
            .await
            .expect("open MT-151-pin store");
        storage
            .with_admin_operation(|database| {
                Box::pin(async move {
                    database
                        .query_bound(
                            mt151_pin_schema.as_str(),
                            BootstrapBindings {
                                schema_version: SCHEMA_VERSION.to_owned(),
                                schema_revision: PRE_ACCOUNT_SETUP_REVISION,
                                namespace: DEFAULT_NAMESPACE.to_owned(),
                                database: DEFAULT_DATABASE.to_owned(),
                                source_manifest_sha256: SCHEMA_LINEAGE_SHA256.to_owned(),
                                generated_surql_sha256: PRE_MT152_GENERATED_SURREALQL_SHA256
                                    .to_owned(),
                            },
                        )
                        .await?;
                    ensure_knowledge_schema_registry(&database).await?;
                    database
                        .query(format!(
                            "UPDATE ONLY {BOOTSTRAP_STATE_ID} SET \
                             info_fingerprint_sha256 = '{PRE_MT152_SCHEMA_INFO_SHA256}', \
                             apply_state = 'complete', updated_at = time::now(); \
                             CREATE workspaces:mt152_folders CONTENT {{ name: 'folders' }}; \
                             CREATE kernel_event_ledger:mt152_folder_evt CONTENT {{ event_id: 'mt152_folder_evt', \
                             event_version: 'v1', kernel_task_run_id: 'run', session_run_id: 'session', \
                             aggregate_type: 'loom_folder', aggregate_id: 'seed', idempotency_key: 'mt152-folder-seed', \
                             event_type: 'seed', actor_kind: 'HUMAN', actor_id: 'test', payload_hash: 'seed', \
                             source_component: 'test', payload: {{ }} }}; \
                             CREATE loom_folders:mt152_root_a CONTENT {{ folder_id: 'mt152_root_a', \
                             workspace_id: workspaces:mt152_folders, parent_folder_id: NONE, name: 'Shared', \
                             event_ledger_event_id: kernel_event_ledger:mt152_folder_evt, \
                             created_at: d'2026-01-01T00:00:00Z' }}; \
                             CREATE loom_folders:mt152_root_b CONTENT {{ folder_id: 'mt152_root_b', \
                             workspace_id: workspaces:mt152_folders, parent_folder_id: NONE, name: 'Shared', \
                             event_ledger_event_id: kernel_event_ledger:mt152_folder_evt, \
                             created_at: d'2026-01-02T00:00:00Z' }}; \
                             CREATE loom_folders:mt152_child CONTENT {{ folder_id: 'mt152_child', \
                             workspace_id: workspaces:mt152_folders, parent_folder_id: loom_folders:mt152_root_a, \
                             name: 'Shared', event_ledger_event_id: kernel_event_ledger:mt152_folder_evt }};"
                        ))
                        .await?
                        .check()?;
                    Ok(())
                })
            })
            .await
            .expect("construct exact MT-151-pin store with duplicate root folders");
        storage.shutdown().await.expect("close MT-151-pin store");

        let reopened = open_test_storage(&directory)
            .await
            .expect("reopen MT-151-pin store");
        let upgraded = bootstrap_schema(&reopened)
            .await
            .expect("upgrade exact MT-151-pin store holding duplicate root folders");
        assert_eq!(
            upgraded.outcome,
            SchemaBootstrapOutcome::UpgradedSupportedPredecessor
        );
        assert_eq!(
            upgraded.info_fingerprint_sha256,
            EXPECTED_SCHEMA_INFO_SHA256
        );
        reopened
            .with_admin_operation(|database| {
                Box::pin(async move {
                    let mut keys = database
                        .query(
                            "SELECT VALUE sibling_key FROM loom_folders ORDER BY folder_id ASC; \
                             SELECT VALUE name FROM loom_folders ORDER BY folder_id ASC;",
                        )
                        .await?;
                    let sibling_keys: Vec<String> = keys.take(0)?;
                    let names: Vec<String> = keys.take(1)?;
                    assert_eq!(
                        sibling_keys,
                        vec![
                            "v1|w13:mt152_folders|p12:mt152_root_a|n6:Shared".to_owned(),
                            "v1|w13:mt152_folders|r|n6:Shared".to_owned(),
                            "v1|w13:mt152_folders|r|n6:Shared#dup1".to_owned(),
                        ]
                    );
                    assert_eq!(
                        names,
                        vec!["Shared"; 3],
                        "visible names are never rewritten"
                    );
                    Ok(())
                })
            })
            .await
            .expect("verify backfilled sibling keys");
        let db = SurrealDatabase::new(reopened.clone());
        let duplicate_root = db
            .create_loom_folder(
                "mt152_folders",
                NewLoomFolder {
                    folder_id: None,
                    workspace_id: "mt152_folders".to_owned(),
                    parent_folder_id: None,
                    name: "Shared".to_owned(),
                    color: None,
                    sort_mode: LoomFolderSortMode::UpdatedDesc,
                    sort_order: None,
                    project_ref: None,
                },
            )
            .await
            .expect_err("a third root 'Shared' must hit uq_loom_folders_sibling_key");
        assert!(
            matches!(
                duplicate_root,
                StorageError::Conflict("loom_folder_sibling_name")
            ),
            "root duplicate must surface the typed sibling-name conflict, got {duplicate_root}"
        );
        let renamed_into_collision = db
            .update_loom_folder(
                "mt152_folders",
                "mt152_child",
                LoomFolderUpdate {
                    parent_folder_id: Some(None),
                    ..LoomFolderUpdate::default()
                },
            )
            .await
            .expect_err("re-parenting the child to root under the taken name must be rejected");
        assert!(
            matches!(renamed_into_collision, StorageError::Conflict("loom_folder_sibling_name")),
            "re-parent into a root collision must surface the typed conflict, got {renamed_into_collision}"
        );
        let distinct = db
            .create_loom_folder(
                "mt152_folders",
                NewLoomFolder {
                    folder_id: None,
                    workspace_id: "mt152_folders".to_owned(),
                    parent_folder_id: None,
                    name: "Distinct".to_owned(),
                    color: None,
                    sort_mode: LoomFolderSortMode::UpdatedDesc,
                    sort_order: None,
                    project_ref: None,
                },
            )
            .await
            .expect("a distinct root name is still accepted after the upgrade");
        assert_eq!(distinct.name, "Distinct");
        reopened.shutdown().await.expect("close upgraded store");
    }

    /// journal-key and graph-anchor DDL; statements are compared whitespace-normalised because
    /// the fresh script and the upgrade both carry the multi-line `journal_key` VALUE verbatim.
    #[test]
    fn mt151_upgrade_statements_match_schema() {
        fn statements(source: &str) -> Vec<String> {
            source
                .lines()
                .filter(|line| !line.trim_start().starts_with("--"))
                .collect::<Vec<_>>()
                .join("\n")
                .split(';')
                .map(|statement| statement.split_whitespace().collect::<Vec<_>>().join(" "))
                .filter(|statement| statement.starts_with("DEFINE "))
                .collect()
        }
        let mut upgrade = statements(MT151_JOURNAL_KEY_MATERIALISE_STATEMENTS);
        upgrade.extend(statements(
            MT151_JOURNAL_KEY_AND_GRAPH_ANCHOR_UPGRADE_STATEMENTS,
        ));
        assert_eq!(upgrade.len(), 9);
        // Compared with the revision-157 script (after MT-151, before the MT-109 C1-C3 / MT-154
        // permission rewrites of storage_graph_anchors and loom_blocks); later waves re-emit their own
        // versions of these statements.
        let schema = statements(&pre_account_setup_schema());
        for statement in &upgrade {
            assert!(
                schema
                    .iter()
                    .any(|schema_statement| schema_statement == statement),
                "MT-151 upgrade DDL drifted from schema.surql: {statement}"
            );
        }
        assert!(MT151_JOURNAL_KEY_MATERIALISE_STATEMENTS.contains(
            "UPDATE loom_blocks SET updated_at = updated_at WHERE content_type = 'journal' AND journal_date != NONE RETURN NONE;"
        ));
        assert!(TABLE_NAMES.contains(&"storage_graph_anchors"));
        assert_ne!(
            PRE_MT151_GENERATED_SURREALQL_SHA256,
            GENERATED_SURREALQL_SHA256
        );
        assert_ne!(PRE_MT151_SCHEMA_INFO_SHA256, EXPECTED_SCHEMA_INFO_SHA256);
    }

    #[test]
    fn predecessor_registry_hash_rejects_changed_nonempty_retired_source() {
        let expected = expected_predecessor_registry_metadata()
            .expect("exact predecessor registry metadata must be complete");
        assert_eq!(
            compute_predecessor_registry_hash(&expected),
            PREDECESSOR_KNOWLEDGE_REGISTRY_SHA256
        );

        let mut tampered = expected;
        tampered[0].retired_source = "attacker-controlled-nonempty.sql".to_owned();
        assert_ne!(
            compute_predecessor_registry_hash(&tampered),
            PREDECESSOR_KNOWLEDGE_REGISTRY_SHA256
        );
    }

    #[test]
    fn schema_contract_is_wave_scoped_and_identity_safe() {
        assert_eq!(
            TABLE_DEFINITION_COUNT,
            SOURCE_TABLE_COUNT + SOURCE_VIEW_COUNT + SURREAL_BOOTSTRAP_STATE_TABLE_COUNT
        );
        assert_eq!(
            INDEX_DEFINITION_COUNT,
            SOURCE_NAMED_INDEX_COUNT
                + SURREAL_PRIMARY_KEY_INDEX_COUNT
                + SURREAL_BOOTSTRAP_STATE_INDEX_COUNT
        );
        assert_eq!(
            SCHEMA.matches("DEFINE TABLE OVERWRITE ").count(),
            TABLE_DEFINITION_COUNT
        );
        assert_eq!(
            SCHEMA.matches("DEFINE FIELD OVERWRITE ").count(),
            AUTHORED_FIELD_DEFINITION_COUNT
        );
        assert_eq!(
            SCHEMA.matches(" FLEXIBLE").count(),
            FLEXIBLE_FIELD_DEFINITION_COUNT
        );
        let mut expected_type_any_wildcards = std::collections::BTreeSet::new();
        for definition in SCHEMA.lines().filter(|line| {
            line.starts_with("DEFINE FIELD OVERWRITE ") && line.contains(" FLEXIBLE")
        }) {
            let parts = definition.split_whitespace().collect::<Vec<_>>();
            let field = parts[3];
            let table = parts[6];
            let collection_depth =
                definition.matches("array<").count() + definition.matches("set<").count();
            let wildcard = format!(
                "DEFINE FIELD OVERWRITE {field}{} ON TABLE {table} TYPE any;",
                ".*".repeat(collection_depth + 1)
            );
            assert!(
                expected_type_any_wildcards.insert(wildcard.clone()),
                "duplicate expected SCHEMAFULL wildcard: {wildcard}"
            );
            assert!(
                SCHEMA.lines().any(|line| line == wildcard),
                "missing SCHEMAFULL wildcard for {table}.{field}: {wildcard}"
            );
        }
        for definition in SCHEMA.lines().filter(|line| {
            line.starts_with("DEFINE FIELD OVERWRITE ")
                && (line.contains(" TYPE array;")
                    || line.contains(" TYPE array DEFAULT")
                    || line.contains(" TYPE option<array>;")
                    || line.contains(" TYPE option<array> DEFAULT"))
        }) {
            let parts = definition.split_whitespace().collect::<Vec<_>>();
            let field = parts[3];
            let table = parts[6];
            let wildcard = format!("DEFINE FIELD OVERWRITE {field}.* ON TABLE {table} TYPE any;");
            assert!(
                expected_type_any_wildcards.insert(wildcard.clone()),
                "duplicate expected untyped-array wildcard: {wildcard}"
            );
            assert!(
                SCHEMA.lines().any(|line| line == wildcard),
                "missing SCHEMAFULL wildcard for {table}.{field}: {wildcard}"
            );
        }
        for definition in INTENTIONAL_UNION_ANY_FIELD_DEFINITIONS {
            assert!(
                SCHEMA.lines().any(|line| line == definition),
                "missing intentional top-level TYPE any definition: {definition}"
            );
            let parts = definition.split_whitespace().collect::<Vec<_>>();
            let field = parts[3];
            let table = parts[6];
            let wildcard = format!("DEFINE FIELD OVERWRITE {field}.* ON TABLE {table} TYPE any;");
            assert!(
                expected_type_any_wildcards.insert(wildcard.clone()),
                "duplicate expected union-field wildcard: {wildcard}"
            );
            assert!(
                SCHEMA.lines().any(|line| line == wildcard),
                "missing SCHEMAFULL wildcard for intentional union field {table}.{field}: {wildcard}"
            );
        }
        assert_eq!(
            expected_type_any_wildcards.len(),
            FLEXIBLE_WILDCARD_FIELD_DEFINITION_COUNT
        );
        let type_any_definitions = SCHEMA
            .lines()
            .filter(|line| line.contains("TYPE any"))
            .collect::<Vec<_>>();
        assert_eq!(
            type_any_definitions.len(),
            FLEXIBLE_WILDCARD_FIELD_DEFINITION_COUNT
                + INTENTIONAL_UNION_ANY_FIELD_DEFINITIONS.len()
        );
        for definition in type_any_definitions {
            if INTENTIONAL_UNION_ANY_FIELD_DEFINITIONS.contains(&definition) {
                continue;
            }
            assert!(
                expected_type_any_wildcards.remove(definition),
                "unauthorized TYPE any definition: {definition}"
            );
        }
        assert!(
            expected_type_any_wildcards.is_empty(),
            "missing expected TYPE any wildcards: {expected_type_any_wildcards:?}"
        );
        assert_eq!(
            generated_collection_subtype_field_count(SCHEMA),
            ENGINE_GENERATED_COLLECTION_SUBTYPE_FIELD_COUNT
        );
        assert_eq!(
            FIELD_DEFINITION_COUNT,
            AUTHORED_FIELD_DEFINITION_COUNT + ENGINE_GENERATED_COLLECTION_SUBTYPE_FIELD_COUNT
        );
        assert!(!SCHEMA.contains("array<any>"));
        assert!(!SCHEMA.contains("set<any>"));
        assert_eq!(
            SCHEMA.matches("DEFINE INDEX OVERWRITE ").count(),
            INDEX_DEFINITION_COUNT
        );
        // Count REFERENCE clauses in DDL only: a SurrealQL comment may name the clause (MT-154,
        // schema.surql owner-delete comment) without defining a field.
        assert_eq!(
            SCHEMA
                .lines()
                .filter(|line| !line.trim_start().starts_with("--"))
                .map(|line| line.matches("REFERENCE ON DELETE ").count())
                .sum::<usize>(),
            REFERENCE_FIELD_COUNT
        );
        assert_eq!(
            SCHEMA.matches("record::exists($value)").count(),
            EXPLICIT_REFERENCE_EXISTENCE_ASSERTION_COUNT
        );
        assert_eq!(RECORD_ID_ONLY_TABLES.len(), 18);

        for (table, field) in REFERENCED_BUSINESS_KEY_ALIASES {
            let definition = SCHEMA
                .lines()
                .find(|line| {
                    line.starts_with(&format!(
                        "DEFINE FIELD OVERWRITE {field} ON TABLE {table} TYPE"
                    ))
                })
                .unwrap_or_else(|| panic!("missing business-key alias {table}.{field}"));
            assert!(definition.contains("ASSERT $value = record::id($this.id)"));
        }
        assert_eq!(
            SCHEMA.matches("record::id($this.id)").count(),
            RECORD_ID_ALIAS_ASSERTION_COUNT
        );
        for required_table in [
            "atelier_character",
            "atelier_source_evidence_record",
            "atelier_contact_sheet_raster_export_plan",
            "atelier_story_beat",
        ] {
            assert!(SCHEMA.contains(&format!(
                "DEFINE TABLE OVERWRITE {required_table} SCHEMAFULL PERMISSIONS NONE;"
            )));
        }
        assert!(SCHEMA.contains(
            "$value = type::record('atelier_source_evidence_record', [$this.matrix_id, record::id($value)[1]])"
        ));
        assert!(SCHEMA.contains("cascade_atelier_source_evidence_record"));
        assert!(!SCHEMA.contains("apply_state = 'applying'"));
        assert!(SCHEMA.contains("HANDSHAKE_SURREAL_SCHEMA_DATABASE_NOT_EMPTY"));
        for database_category in [
            "accesses",
            "analyzers",
            "apis",
            "buckets",
            "configs",
            "functions",
            "models",
            "modules",
            "params",
            "sequences",
            "tables",
            "users",
        ] {
            assert!(SCHEMA.contains(&format!(
                "array::len($existing_database.{database_category}) != 0"
            )));
        }
        assert!(SCHEMA.contains("generated_surql_sha256"));
        assert!(SCHEMA.contains("BEGIN TRANSACTION;"));
        assert!(SCHEMA.contains("COMMIT TRANSACTION;"));
        // No legacy server backend `jsonb` type token may survive the projection. Checked as a
        // whole identifier token, not a substring: source column NAMES such as
        // `attribution_jsonb` (migration 0311) are transcribed verbatim and are not
        // legacy server backend type syntax.
        let lowered = SCHEMA.to_ascii_lowercase();
        assert!(!lowered.contains("::jsonb"));
        assert!(!lowered
            .split(|character: char| !character.is_ascii_alphanumeric() && character != '_')
            .any(|token| token == "jsonb"));
    }

    #[tokio::test]
    async fn bootstrap_is_concurrent_restart_safe_and_receipt_is_live() {
        let directory = tempfile::Builder::new()
            .prefix("mt109-bootstrap-")
            .tempdir()
            .expect("temporary Surreal directory");
        let storage = open_test_storage(&directory)
            .await
            .expect("open fresh store");

        let mut reopened_storage = None;
        let body = futures::FutureExt::catch_unwind(std::panic::AssertUnwindSafe(async {
            let left = storage.clone();
            let right = storage.clone();
            let (left_report, right_report) =
                tokio::join!(bootstrap_schema(&left), bootstrap_schema(&right),);
            let left_report = left_report.expect("left bootstrap");
            let right_report = right_report.expect("right bootstrap");
            assert_ne!(
                left_report.reused_existing_schema,
                right_report.reused_existing_schema
            );
            for report in [&left_report, &right_report] {
                assert_eq!(report.schema_version, SCHEMA_VERSION);
                assert_eq!(report.source_manifest_sha256, SCHEMA_LINEAGE_SHA256);
                assert_eq!(report.generated_surql_sha256, GENERATED_SURREALQL_SHA256);
                assert_eq!(report.info_fingerprint_sha256.len(), 64);
                assert_eq!(report.tables_defined, TABLE_DEFINITION_COUNT);
                assert_eq!(report.fields_defined, FIELD_DEFINITION_COUNT);
                assert_eq!(report.indexes_defined, INDEX_DEFINITION_COUNT);
                assert_eq!(report.table_names.len(), TABLE_DEFINITION_COUNT);
            }
            let before_restart = index_names(&storage, "kernel_event_ledger")
                .await
                .expect("pre-restart INFO");
            storage.shutdown().await.expect("close first store");

            let reopened = open_test_storage(&directory).await.expect("reopen store");
            reopened_storage = Some(reopened.clone());
            let restarted = bootstrap_schema(&reopened)
                .await
                .expect("exact-current restart");
            assert!(restarted.reused_existing_schema);
            assert_eq!(
                before_restart,
                index_names(&reopened, "kernel_event_ledger")
                    .await
                    .expect("post-restart INFO")
            );
            reopened.shutdown().await.expect("close reopened store");
        }))
        .await;
        let mut cleanup_errors = Vec::new();
        if let Some(reopened) = reopened_storage.take() {
            if let Err(error) = reopened.shutdown().await {
                cleanup_errors.push(format!("reopened store shutdown: {error}"));
            }
            drop(reopened);
        }
        if let Err(error) = storage.shutdown().await {
            cleanup_errors.push(format!("original store shutdown: {error}"));
        }
        drop(storage);
        if !cleanup_errors.is_empty() {
            let path = directory.keep();
            panic!(
                "bootstrap proof cleanup failed; store preserved at {}: {}; body_failed={}",
                path.display(),
                cleanup_errors.join("; "),
                body.is_err()
            );
        }
        let path = directory.path().to_path_buf();
        directory
            .close()
            .expect("remove closed bootstrap proof store");
        assert!(
            !path.exists(),
            "bootstrap proof store survived cleanup: {}",
            path.display()
        );
        eprintln!("MT109_BOOTSTRAP_STORE_REMOVED {}", path.display());
        if let Err(panic) = body {
            std::panic::resume_unwind(panic);
        }
    }

    #[tokio::test]
    async fn mt139_exact_predecessor_upgrade_preserves_data_and_restarts_current() {
        const CURRENT_HEADER: &str =
            "-- This transaction is the sole declarative schema authority. Rust bootstrap\n\
-- code verifies these exact bytes, parses every declared object into the pinned\n\
-- semantic catalog, and compares the applied live-engine catalog fail-closed.";
        const PREDECESSOR_HEADER: &str =
            "-- This transaction is the bounded Surreal-native projection of the source\n\
-- wave enumerated by `SOURCE_WAVE_FILES` in schema.rs (migrations 0001-0129\n\
-- plus the selected 0130-0365 bands). Every table created by a forward\n\
-- migration in that enumeration is defined here; the source enumeration is the\n\
-- only authority for which migrations are in the wave.";

        let predecessor_schema = PRE_MT109_SCHEMA.replace(CURRENT_HEADER, PREDECESSOR_HEADER).replace(
            "DEFINE FIELD OVERWRITE schema_source ON TABLE knowledge_schema_registry TYPE string;",
            "DEFINE FIELD OVERWRITE migration_file ON TABLE knowledge_schema_registry TYPE string;",
        );
        assert_eq!(
            sha256_hex(predecessor_schema.as_bytes()),
            PREDECESSOR_GENERATED_SURREALQL_SHA256,
            "predecessor allowlist must be derived from the exact preceding artifact"
        );
        let directory = tempfile::tempdir().expect("temporary predecessor store");
        let storage = open_test_storage(&directory)
            .await
            .expect("open predecessor store");
        storage
            .with_admin_operation(|database| {
                Box::pin(async move {
                    database
                        .query_bound(
                            predecessor_schema.as_str(),
                            BootstrapBindings {
                                schema_version: SCHEMA_VERSION.to_owned(),
                                schema_revision: PRE_ACCOUNT_SETUP_REVISION,
                                namespace: DEFAULT_NAMESPACE.to_owned(),
                                database: DEFAULT_DATABASE.to_owned(),
                                source_manifest_sha256: SCHEMA_LINEAGE_SHA256.to_owned(),
                                generated_surql_sha256: PREDECESSOR_GENERATED_SURREALQL_SHA256
                                    .to_owned(),
                            },
                        )
                        .await?;
                    database
                        .query(PREDECESSOR_KNOWLEDGE_SCHEMA_REGISTRY_SEED)
                        .await?;
                    ensure_supported_predecessor_registry(&database).await?;
                    database
                        .query(format!(
                            "UPDATE ONLY {BOOTSTRAP_STATE_ID} SET \
                             info_fingerprint_sha256 = '{PREDECESSOR_SCHEMA_INFO_SHA256}', \
                             apply_state = 'complete', updated_at = time::now(); \
                             CREATE workspaces:mt139_predecessor CONTENT {{ name: 'sentinel' }};"
                        ))
                        .await?;
                    Ok(())
                })
            })
            .await
            .expect("construct exact predecessor store");
        storage.shutdown().await.expect("close predecessor store");

        let reopened = open_test_storage(&directory)
            .await
            .expect("reopen predecessor store");
        let upgraded = bootstrap_schema(&reopened)
            .await
            .expect("upgrade exact predecessor");
        assert!(upgraded.reused_existing_schema);
        assert_eq!(
            upgraded.outcome,
            SchemaBootstrapOutcome::UpgradedSupportedPredecessor
        );
        assert_eq!(upgraded.generated_surql_sha256, GENERATED_SURREALQL_SHA256);
        assert_eq!(
            upgraded.info_fingerprint_sha256,
            EXPECTED_SCHEMA_INFO_SHA256
        );
        reopened
            .with_admin_operation(|database| {
                Box::pin(async move {
                    let mut sentinel = database
                        .query("RETURN workspaces:mt139_predecessor.name;")
                        .await?;
                    let name: Option<String> = sentinel.take(0)?;
                    assert_eq!(name.as_deref(), Some("sentinel"));
                    ensure_knowledge_schema_registry(&database).await?;
                    let mut response = database
                        .query("INFO FOR TABLE knowledge_schema_registry STRUCTURE;")
                        .await?;
                    let info: SurrealValueData = response.take(0)?;
                    let fields = parse_named_array(&info, "fields")
                        .unwrap_or_else(|reason| panic!("invalid registry INFO: {reason}"));
                    assert!(fields.iter().any(|field| field == "schema_source"));
                    assert!(!fields.iter().any(|field| field == "migration_file"));
                    Ok(())
                })
            })
            .await
            .expect("verify upgraded data and registry");
        reopened.shutdown().await.expect("close upgraded store");

        let current = open_test_storage(&directory)
            .await
            .expect("reopen upgraded store");
        current
            .with_admin_operation(|database| {
                Box::pin(async move {
                    let state = read_context_and_state(&database)
                        .await?
                        .expect("upgraded state survives reopen");
                    assert!(state.is_exact_current());
                    ensure_knowledge_schema_registry(&database).await?;
                    let mut sentinel = database
                        .query("RETURN workspaces:mt139_predecessor.name;")
                        .await?;
                    let name: Option<String> = sentinel.take(0)?;
                    assert_eq!(name.as_deref(), Some("sentinel"));
                    Ok(())
                })
            })
            .await
            .expect("verify exact-current durable reopen after upgrade");
        current.shutdown().await.expect("close current store");
    }

    /// MT-151: a store at the exact MT-142 pin (the current script minus the MT-151 blocks,
    /// proven byte-exact against `PRE_MT151_GENERATED_SURREALQL_SHA256`) holding a journal
    /// block is upgraded in place: the journal key is materialised on the existing row, the
    /// UNIQUE index then rejects a second journal for that date, `storage_graph_anchors`
    /// exists, the live fingerprint is the current pin, and the state survives a reopen.
    #[tokio::test]
    async fn mt151_exact_mt142_pin_upgrade_materialises_journal_key_and_restarts_current() {
        const JOURNAL_KEY_BLOCK: &str = concat!(
            "-- MT-151 journal_key: stored discriminator for the journal get-or-create natural key\n",
            "-- (workspace, journal_date). NONE for every non-journal row, and the engine skips NONE\n",
            "-- tuples in UNIQUE indexes (surrealdb-core-3.2.0/src/idx/index.rs:193-197), so only\n",
            "-- journal blocks are constrained. Guards the invariant LOOM_MUTATION_LOCK alone used to\n",
            "-- hold, on every write path (get-or-create, create, update.journal_date).\n",
            "DEFINE FIELD OVERWRITE journal_key ON TABLE loom_blocks TYPE option<string>\n",
            "    VALUE IF $this.content_type = 'journal' AND $this.journal_date != NONE {\n",
            "        type::string($this.workspace_id) + '|' + $this.journal_date\n",
            "    } ELSE {\n",
            "        NONE\n",
            "    };\n",
        );
        const JOURNAL_INDEX_LINE: &str =
            "DEFINE INDEX OVERWRITE uq_loom_blocks_journal_key ON TABLE loom_blocks FIELDS journal_key UNIQUE;\n";
        const GRAPH_ANCHORS_BLOCK: &str = concat!(
            "\n-- MT-151 storage_graph_anchors: one version row per graph whose acyclicity is decided\n",
            "-- Rust-side (the Loom folder tree per workspace, the work-packet dependency graph).\n",
            "-- The deciding operation reads the version before its graph read and compare-and-sets\n",
            "-- it inside the committing transaction (THROW on mismatch, UPSERT to bump), so a\n",
            "-- decision against a stale graph fails closed and two writers that overlap in the\n",
            "-- engine collide on this one key at commit. A serialization device, not domain data.\n",
            "DEFINE TABLE OVERWRITE storage_graph_anchors SCHEMAFULL PERMISSIONS NONE;\n",
            "DEFINE FIELD OVERWRITE anchor_key ON TABLE storage_graph_anchors TYPE string ASSERT $value = record::id($this.id);\n",
            "DEFINE FIELD OVERWRITE graph_kind ON TABLE storage_graph_anchors TYPE 'loom_folder_tree' | 'work_packet_dependencies';\n",
            "DEFINE FIELD OVERWRITE scope_key ON TABLE storage_graph_anchors TYPE string ASSERT string::trim($value) != '';\n",
            "DEFINE FIELD OVERWRITE version ON TABLE storage_graph_anchors TYPE int ASSERT $value >= 1;\n",
            "DEFINE FIELD OVERWRITE updated_at ON TABLE storage_graph_anchors TYPE datetime DEFAULT time::now();\n",
            "DEFINE INDEX OVERWRITE pk_storage_graph_anchors ON TABLE storage_graph_anchors FIELDS anchor_key UNIQUE;\n",
        );
        for block in [JOURNAL_KEY_BLOCK, JOURNAL_INDEX_LINE, GRAPH_ANCHORS_BLOCK] {
            assert_eq!(
                PRE_MT109_SCHEMA.matches(block).count(),
                1,
                "MT-151 block drifted: {block}"
            );
        }
        // The MT-152 block sits on top of the MT-151 pin, so the MT-142 pin is the current
        // script minus both; the pre-MT-151 path now applies MT-151 and MT-152 together.
        let mt142_pin_schema = mt151_pin_schema()
            .replace(JOURNAL_KEY_BLOCK, "")
            .replace(JOURNAL_INDEX_LINE, "")
            .replace(GRAPH_ANCHORS_BLOCK, "");
        assert_eq!(
            sha256_hex(mt142_pin_schema.as_bytes()),
            PRE_MT151_GENERATED_SURREALQL_SHA256,
            "the pre-MT-151 allowlist must be exactly the current script minus the MT-151 and MT-152 blocks"
        );

        let directory = tempfile::tempdir().expect("temporary MT-142-pin store");
        let storage = open_test_storage(&directory)
            .await
            .expect("open MT-142-pin store");
        storage
            .with_admin_operation(|database| {
                Box::pin(async move {
                    database
                        .query_bound(
                            mt142_pin_schema.as_str(),
                            BootstrapBindings {
                                schema_version: SCHEMA_VERSION.to_owned(),
                                schema_revision: PRE_ACCOUNT_SETUP_REVISION,
                                namespace: DEFAULT_NAMESPACE.to_owned(),
                                database: DEFAULT_DATABASE.to_owned(),
                                source_manifest_sha256: SCHEMA_LINEAGE_SHA256.to_owned(),
                                generated_surql_sha256: PRE_MT151_GENERATED_SURREALQL_SHA256
                                    .to_owned(),
                            },
                        )
                        .await?;
                    ensure_knowledge_schema_registry(&database).await?;
                    database
                        .query(format!(
                            "UPDATE ONLY {BOOTSTRAP_STATE_ID} SET \
                             info_fingerprint_sha256 = '{PRE_MT151_SCHEMA_INFO_SHA256}', \
                             apply_state = 'complete', updated_at = time::now(); \
                             CREATE workspaces:mt151_pin CONTENT {{ name: 'sentinel' }}; \
                             CREATE loom_blocks:mt151_journal CONTENT {{ block_id: 'mt151_journal', \
                             workspace_id: workspaces:mt151_pin, content_type: 'journal', \
                             title: 'Daily Note 2026-09-11', journal_date: '2026-09-11' }};"
                        ))
                        .await?
                        .check()?;
                    Ok(())
                })
            })
            .await
            .expect("construct exact MT-142-pin store with a journal block");
        storage.shutdown().await.expect("close MT-142-pin store");

        let reopened = open_test_storage(&directory)
            .await
            .expect("reopen MT-142-pin store");
        let upgraded = match bootstrap_schema(&reopened).await {
            Ok(report) => report,
            Err(error) => {
                let reference = fresh_mem_catalog().await;
                let observed = reopened
                    .with_admin_operation(|database| {
                        Box::pin(async move { canonical_catalog(&database).await })
                    })
                    .await
                    .expect("inspect the failed upgrade");
                report_catalog_drift("MT151_UPGRADE", &reference, &observed);
                panic!("upgrade exact MT-142-pin store: {error}");
            }
        };
        assert!(upgraded.reused_existing_schema);
        assert_eq!(
            upgraded.outcome,
            SchemaBootstrapOutcome::UpgradedSupportedPredecessor
        );
        assert_eq!(upgraded.generated_surql_sha256, GENERATED_SURREALQL_SHA256);
        assert_eq!(
            upgraded.info_fingerprint_sha256,
            EXPECTED_SCHEMA_INFO_SHA256
        );
        reopened
            .with_admin_operation(|database| {
                Box::pin(async move {
                    let mut materialised = database
                        .query("RETURN loom_blocks:mt151_journal.journal_key;")
                        .await?;
                    let journal_key: Option<String> = materialised.take(0)?;
                    assert_eq!(
                        journal_key.as_deref(),
                        Some("workspaces:mt151_pin|2026-09-11"),
                        "the upgrade must materialise journal_key on the existing journal row"
                    );
                    let duplicate = database
                        .query(
                            "CREATE loom_blocks:mt151_duplicate CONTENT { block_id: 'mt151_duplicate', \
                             workspace_id: workspaces:mt151_pin, content_type: 'journal', \
                             title: 'Daily Note 2026-09-11', journal_date: '2026-09-11' };",
                        )
                        .await
                        .and_then(|response| Ok(response.check()?));
                    let rendered = duplicate
                        .err()
                        .map(|error| error.to_string())
                        .unwrap_or_default();
                    assert!(
                        rendered.contains("uq_loom_blocks_journal_key"),
                        "a second journal for the date must lose to the upgraded index: {rendered}"
                    );
                    let mut anchors = database
                        .query("INFO FOR TABLE storage_graph_anchors STRUCTURE;")
                        .await?;
                    let info: SurrealValueData = anchors.take(0)?;
                    let fields = parse_named_array(&info, "fields")
                        .unwrap_or_else(|reason| panic!("invalid anchors INFO: {reason}"));
                    assert!(fields.iter().any(|field| field == "version"));
                    Ok(())
                })
            })
            .await
            .expect("verify upgraded journal key, index and anchors table");
        reopened.shutdown().await.expect("close upgraded store");

        let current = open_test_storage(&directory)
            .await
            .expect("reopen upgraded store");
        current
            .with_admin_operation(|database| {
                Box::pin(async move {
                    let state = read_context_and_state(&database)
                        .await?
                        .expect("upgraded state survives reopen");
                    assert!(state.is_exact_current());
                    let mut sentinel = database.query("RETURN workspaces:mt151_pin.name;").await?;
                    let name: Option<String> = sentinel.take(0)?;
                    assert_eq!(name.as_deref(), Some("sentinel"));
                    Ok(())
                })
            })
            .await
            .expect("verify exact-current durable reopen after the MT-151 upgrade");
        current.shutdown().await.expect("close current store");
    }

    /// The revision-158 receipt schema upgrades only after both its durable marker and its
    /// live catalog match the observed predecessor. A changed catalog must remain untouched.
    #[tokio::test]
    async fn canvas_receipt_revision_158_upgrade_requires_exact_catalog_and_restarts_current() {
        const WRONG_PRE_CANVAS_GENERATED_SHA256: &str =
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";
        let previous_schema = pre_canvas_receipt_schema();
        assert_eq!(
            sha256_hex(previous_schema.as_bytes()),
            PRE_CANVAS_RECEIPT_GENERATED_SHA256,
            "the revision-158 predecessor must be exactly the current schema without the creator-session receipt read branch"
        );
        let directory = tempfile::tempdir().expect("temporary Canvas receipt predecessor");
        let storage = open_test_storage(&directory)
            .await
            .expect("open exact revision-158 predecessor store");
        storage
            .with_admin_operation(move |database| {
                Box::pin(async move {
                    database
                        .query_bound(
                            previous_schema.as_str(),
                            BootstrapBindings {
                                schema_version: SCHEMA_VERSION.to_owned(),
                                schema_revision: PRE_CANVAS_RECEIPT_REVISION,
                                namespace: DEFAULT_NAMESPACE.to_owned(),
                                database: DEFAULT_DATABASE.to_owned(),
                                source_manifest_sha256: SCHEMA_LINEAGE_SHA256.to_owned(),
                                generated_surql_sha256: PRE_CANVAS_RECEIPT_GENERATED_SHA256
                                    .to_owned(),
                            },
                        )
                        .await?;
                    ensure_knowledge_schema_registry(&database).await?;
                    assert_eq!(
                        read_schema_catalog(&database).await?.info_fingerprint_sha256,
                        PRE_CANVAS_RECEIPT_INFO_SHA256,
                        "the seeded predecessor must carry its observed catalog fingerprint"
                    );
                    database
                        .query(format!(
                            "UPDATE ONLY {BOOTSTRAP_STATE_ID} SET apply_state = 'complete', info_fingerprint_sha256 = '{PRE_CANVAS_RECEIPT_INFO_SHA256}';"
                        ))
                        .await?;
                    Ok(())
                })
            })
            .await
            .expect("seed exact complete revision-158 predecessor");

        storage
            .with_admin_operation(|database| {
                Box::pin(async move {
                    database
                        .query(format!(
                            "UPDATE ONLY handshake_schema_state:primary SET generated_surql_sha256 = '{WRONG_PRE_CANVAS_GENERATED_SHA256}';"
                        ))
                        .await?;
                    Ok(())
                })
            })
            .await
            .expect("tamper predecessor state for rejection proof");
        let state_rejection = bootstrap_schema(&storage)
            .await
            .expect_err("wrong revision-158 state must fail before DDL");
        assert!(state_rejection
            .to_string()
            .contains("HANDSHAKE_SURREAL_SCHEMA_UNSUPPORTED_LINEAGE"));
        storage
            .with_admin_operation(|database| {
                Box::pin(async move {
                    let state = read_context_and_state(&database)
                        .await?
                        .expect("rejected revision-158 state remains present");
                    assert_eq!(state.revision, PRE_CANVAS_RECEIPT_REVISION);
                    assert_eq!(
                        state.generated_surql_sha256,
                        WRONG_PRE_CANVAS_GENERATED_SHA256
                    );
                    assert_eq!(
                        read_schema_catalog(&database)
                            .await?
                            .info_fingerprint_sha256,
                        PRE_CANVAS_RECEIPT_INFO_SHA256,
                        "wrong state rejection must not alter the predecessor catalog"
                    );
                    Ok(())
                })
            })
            .await
            .expect("reread unchanged state rejection");
        storage
            .with_admin_operation(|database| {
                Box::pin(async move {
                    database
                        .query(format!(
                            "UPDATE ONLY handshake_schema_state:primary SET generated_surql_sha256 = '{PRE_CANVAS_RECEIPT_GENERATED_SHA256}'; \
                             DEFINE TABLE canvas_receipt_revision_158_unknown_overlay SCHEMAFULL PERMISSIONS NONE;"
                        ))
                        .await?;
                    Ok(())
                })
            })
            .await
            .expect("restore predecessor state and introduce catalog drift");
        let rejected_catalog = storage
            .with_admin_operation(|database| {
                Box::pin(async move {
                    Ok(read_schema_catalog(&database)
                        .await?
                        .info_fingerprint_sha256)
                })
            })
            .await
            .expect("read catalog before rejection");
        let catalog_rejection = bootstrap_schema(&storage)
            .await
            .expect_err("unknown revision-158 catalog drift must fail before receipt DDL");
        assert!(catalog_rejection
            .to_string()
            .contains("HANDSHAKE_SURREAL_PRE_CANVAS_RECEIPT_CATALOG_MISMATCH"));
        storage
            .with_admin_operation(|database| {
                Box::pin(async move {
                    let state = read_context_and_state(&database)
                        .await?
                        .expect("catalog-rejected revision-158 state remains present");
                    assert_eq!(state.revision, PRE_CANVAS_RECEIPT_REVISION);
                    assert_eq!(
                        state.generated_surql_sha256,
                        PRE_CANVAS_RECEIPT_GENERATED_SHA256
                    );
                    assert_eq!(
                        read_schema_catalog(&database)
                            .await?
                            .info_fingerprint_sha256,
                        rejected_catalog,
                        "catalog rejection must not alter the predecessor catalog"
                    );
                    Ok(())
                })
            })
            .await
            .expect("reread unchanged catalog rejection");
        storage
            .with_admin_operation(|database| {
                Box::pin(async move {
                    database
                        .query("REMOVE TABLE canvas_receipt_revision_158_unknown_overlay;")
                        .await?;
                    Ok(())
                })
            })
            .await
            .expect("restore exact revision-158 catalog");

        let upgraded = bootstrap_schema(&storage)
            .await
            .expect("upgrade exact revision-158 receipt predecessor");
        assert_eq!(
            upgraded.info_fingerprint_sha256,
            EXPECTED_SCHEMA_INFO_SHA256
        );
        storage
            .shutdown()
            .await
            .expect("close upgraded Canvas receipt predecessor store");
        let reopened = open_test_storage(&directory)
            .await
            .expect("reopen upgraded Canvas receipt store");
        let restarted = bootstrap_schema(&reopened)
            .await
            .expect("reuse current Canvas receipt schema after restart");
        assert_eq!(
            restarted.info_fingerprint_sha256,
            EXPECTED_SCHEMA_INFO_SHA256
        );
        reopened
            .shutdown()
            .await
            .expect("close restarted Canvas receipt store");
    }

    /// MT-154 hardening: a bootstrap that never returns becomes the typed `BootstrapStalled`
    /// error naming the bound, and a finished one passes its result through unchanged.
    #[tokio::test]
    async fn bounded_bootstrap_maps_a_stall_to_the_typed_error() {
        let limit = std::time::Duration::from_millis(20);
        let stalled = bounded_bootstrap(limit, std::future::pending::<Result<(), _>>()).await;
        match stalled {
            Err(SurrealStorageError::BootstrapStalled { waited_ms }) => assert_eq!(waited_ms, 20),
            other => panic!("expected BootstrapStalled, got {other:?}"),
        }
        let message = SurrealStorageError::BootstrapStalled { waited_ms: 20 }.to_string();
        assert!(message.contains("bootstrap stalled") && message.contains("OS flush"));
        let finished = bounded_bootstrap(limit, async { Ok::<_, SurrealStorageError>(7) }).await;
        assert_eq!(finished.expect("finished bootstrap passes through"), 7);
    }

    /// The fresh bootstrap runs every `DEFINE INDEX` of the schema transaction after the
    /// definitions transaction commits, each outside any explicit transaction, and writes the
    /// `schema_applied` receipt in a final short transaction; no line is lost or duplicated.
    #[test]
    fn fresh_bootstrap_script_runs_every_index_outside_an_explicit_transaction() {
        fn index_statements_inside_transactions(script: &str) -> usize {
            let mut inside = false;
            let mut count = 0;
            for line in script.lines() {
                match line {
                    "BEGIN TRANSACTION;" => inside = true,
                    "COMMIT TRANSACTION;" => inside = false,
                    _ if inside && line.starts_with("DEFINE INDEX ") => count += 1,
                    _ => {}
                }
            }
            count
        }
        let script = fresh_bootstrap_script(SCHEMA).expect("split compiled schema");
        assert!(
            !script.definitions.contains("\nDEFINE INDEX "),
            "the definitions transaction holds no index"
        );
        assert!(!script
            .definitions
            .contains("handshake_schema_state:primary"));
        assert_eq!(
            script.definitions.matches("COMMIT TRANSACTION;").count(),
            1,
            "the definitions query is exactly one transaction"
        );
        assert_eq!(
            index_statements_inside_transactions(&script.indexes_and_rest),
            0,
            "no index DDL runs inside BEGIN..COMMIT"
        );
        assert_eq!(
            script
                .indexes_and_rest
                .lines()
                .filter(|line| line.starts_with("DEFINE INDEX "))
                .count(),
            INDEX_DEFINITION_COUNT,
            "every index runs after the definitions transaction"
        );
        let (before_receipt, receipt_transaction) = script
            .indexes_and_rest
            .rsplit_once("BEGIN TRANSACTION;\n")
            .expect("final receipt transaction");
        assert!(
            receipt_transaction.starts_with("UPSERT handshake_schema_state:primary SET")
                && receipt_transaction
                    .trim_end()
                    .ends_with("COMMIT TRANSACTION;")
                && !receipt_transaction.contains("DEFINE "),
            "the final short transaction holds only the receipt: {receipt_transaction}"
        );
        let original_rest = SCHEMA
            .split_once("\nCOMMIT TRANSACTION;\n")
            .expect("schema transaction")
            .1;
        assert!(
            before_receipt
                .trim_end()
                .ends_with(original_rest.trim_end()),
            "the post-transaction statements run unchanged, before the receipt"
        );
        let mut before = SCHEMA.lines().collect::<Vec<_>>();
        let mut after = script
            .definitions
            .lines()
            .chain(script.indexes_and_rest.lines())
            .collect::<Vec<_>>();
        // The split adds exactly one BEGIN and one COMMIT (the receipt transaction).
        let added_begin = after
            .iter()
            .rposition(|line| *line == "BEGIN TRANSACTION;")
            .expect("receipt BEGIN");
        after.remove(added_begin);
        let added_commit = after
            .iter()
            .rposition(|line| *line == "COMMIT TRANSACTION;")
            .expect("receipt COMMIT");
        after.remove(added_commit);
        before.sort_unstable();
        after.sort_unstable();
        assert_eq!(
            before, after,
            "the split moves lines, it never adds or drops one"
        );
    }

    /// Every MT-154 delta pair is re-emitted on upgrade as its complete enclosing OVERWRITE
    /// statement, the MT-154 authority block leads, and nothing but OVERWRITE definitions (and
    /// comments) is emitted.
    #[test]
    fn schema_delta_upgrade_statements_re_emit_every_mt154_delta() {
        for (current, _) in MT154_SCHEMA_DELTAS {
            let mut spans = BTreeMap::new();
            assert!(
                schema_statements_enclosing(current, &mut spans) > 0,
                "MT-154 delta has no enclosing OVERWRITE statement in schema.surql: {current}"
            );
        }
        let statements = schema_delta_upgrade_statements();
        if SCHEMA.contains(MT154_AUTHORITY_BLOCK_BEGIN) {
            assert!(statements.starts_with("-- MT154_AUTHORITY_BEGIN\n"));
        }
        let mut offset = 0;
        while let Some(relative) = statements[offset..].find(|c: char| !c.is_whitespace()) {
            let start = offset + relative;
            if statements[start..].starts_with("--") {
                offset = start
                    + statements[start..]
                        .find('\n')
                        .expect("comment line terminates");
                continue;
            }
            let end = surql_statement_end(&statements, start).expect("statement terminates");
            let first_line = statements[start..].split('\n').next().unwrap_or_default();
            assert!(
                first_line.starts_with("DEFINE ") && first_line.contains(" OVERWRITE "),
                "upgrade re-emission must be OVERWRITE definitions only: {first_line}"
            );
            offset = end;
        }
    }

    #[tokio::test]
    async fn standalone_loom_revision_159_upgrade_requires_exact_catalog_and_restarts_current() {
        const WRONG_PRE_STANDALONE_LOOM_GENERATED_SHA256: &str =
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";
        let previous_schema = pre_standalone_loom_update_schema();
        assert_eq!(
            sha256_hex(previous_schema.as_bytes()),
            PRE_STANDALONE_LOOM_UPDATE_GENERATED_SHA256,
            "the revision-159 predecessor must be exactly the current schema without the standalone Loom update authorization"
        );
        let directory = tempfile::tempdir().expect("temporary standalone Loom predecessor");
        let storage = open_test_storage(&directory)
            .await
            .expect("open exact revision-159 predecessor store");
        storage
            .with_admin_operation(move |database| {
                Box::pin(async move {
                    database
                        .query_bound(
                            previous_schema.as_str(),
                            BootstrapBindings {
                                schema_version: SCHEMA_VERSION.to_owned(),
                                schema_revision: PRE_STANDALONE_LOOM_UPDATE_REVISION,
                                namespace: DEFAULT_NAMESPACE.to_owned(),
                                database: DEFAULT_DATABASE.to_owned(),
                                source_manifest_sha256: SCHEMA_LINEAGE_SHA256.to_owned(),
                                generated_surql_sha256: PRE_STANDALONE_LOOM_UPDATE_GENERATED_SHA256
                                    .to_owned(),
                            },
                        )
                        .await?;
                    ensure_knowledge_schema_registry(&database).await?;
                    assert_eq!(
                        read_schema_catalog(&database).await?.info_fingerprint_sha256,
                        PRE_STANDALONE_LOOM_UPDATE_INFO_SHA256,
                        "the seeded predecessor must carry its observed catalog fingerprint"
                    );
                    database
                        .query(format!(
                            "UPDATE ONLY {BOOTSTRAP_STATE_ID} SET apply_state = 'complete', info_fingerprint_sha256 = '{PRE_STANDALONE_LOOM_UPDATE_INFO_SHA256}';                              CREATE workspaces:revision159_sentinel CONTENT {{ name: 'revision159-sentinel' }};"
                        ))
                        .await?;
                    Ok(())
                })
            })
            .await
            .expect("seed exact complete revision-159 predecessor");

        storage
            .with_admin_operation(|database| {
                Box::pin(async move {
                    database
                        .query(format!(
                            "UPDATE ONLY handshake_schema_state:primary SET generated_surql_sha256 = '{WRONG_PRE_STANDALONE_LOOM_GENERATED_SHA256}';"
                        ))
                        .await?;
                    Ok(())
                })
            })
            .await
            .expect("tamper predecessor state for rejection proof");
        let state_rejection = bootstrap_schema(&storage)
            .await
            .expect_err("wrong revision-159 state must fail before DDL");
        assert!(state_rejection
            .to_string()
            .contains("HANDSHAKE_SURREAL_SCHEMA_UNSUPPORTED_LINEAGE"));
        storage
            .with_admin_operation(|database| {
                Box::pin(async move {
                    let state = read_context_and_state(&database)
                        .await?
                        .expect("rejected revision-159 state remains present");
                    assert_eq!(state.revision, PRE_STANDALONE_LOOM_UPDATE_REVISION);
                    assert_eq!(
                        state.generated_surql_sha256,
                        WRONG_PRE_STANDALONE_LOOM_GENERATED_SHA256
                    );
                    assert_eq!(
                        read_schema_catalog(&database)
                            .await?
                            .info_fingerprint_sha256,
                        PRE_STANDALONE_LOOM_UPDATE_INFO_SHA256,
                        "wrong state rejection must not alter the predecessor catalog"
                    );
                    Ok(())
                })
            })
            .await
            .expect("reread unchanged state rejection");
        storage
            .with_admin_operation(|database| {
                Box::pin(async move {
                    database
                        .query(format!(
                            "UPDATE ONLY handshake_schema_state:primary SET generated_surql_sha256 = '{PRE_STANDALONE_LOOM_UPDATE_GENERATED_SHA256}'; \
                             DEFINE TABLE standalone_loom_revision_159_unknown_overlay SCHEMAFULL PERMISSIONS NONE;"
                        ))
                        .await?;
                    Ok(())
                })
            })
            .await
            .expect("restore predecessor state and introduce catalog drift");
        let rejected_catalog = storage
            .with_admin_operation(|database| {
                Box::pin(async move {
                    Ok(read_schema_catalog(&database)
                        .await?
                        .info_fingerprint_sha256)
                })
            })
            .await
            .expect("read catalog before rejection");
        let catalog_rejection = bootstrap_schema(&storage)
            .await
            .expect_err("unknown revision-159 catalog drift must fail before receipt DDL");
        assert!(catalog_rejection
            .to_string()
            .contains("HANDSHAKE_SURREAL_PRE_STANDALONE_LOOM_UPDATE_CATALOG_MISMATCH"));
        storage
            .with_admin_operation(|database| {
                Box::pin(async move {
                    let state = read_context_and_state(&database)
                        .await?
                        .expect("catalog-rejected revision-159 state remains present");
                    assert_eq!(state.revision, PRE_STANDALONE_LOOM_UPDATE_REVISION);
                    assert_eq!(
                        state.generated_surql_sha256,
                        PRE_STANDALONE_LOOM_UPDATE_GENERATED_SHA256
                    );
                    assert_eq!(
                        read_schema_catalog(&database)
                            .await?
                            .info_fingerprint_sha256,
                        rejected_catalog,
                        "catalog rejection must not alter the predecessor catalog"
                    );
                    Ok(())
                })
            })
            .await
            .expect("reread unchanged catalog rejection");
        storage
            .with_admin_operation(|database| {
                Box::pin(async move {
                    database
                        .query("REMOVE TABLE standalone_loom_revision_159_unknown_overlay;")
                        .await?;
                    Ok(())
                })
            })
            .await
            .expect("restore exact revision-159 catalog");

        let upgraded = bootstrap_schema(&storage)
            .await
            .expect("upgrade exact revision-159 receipt predecessor");
        assert_eq!(
            upgraded.info_fingerprint_sha256,
            EXPECTED_SCHEMA_INFO_SHA256
        );
        assert_eq!(
            upgraded.outcome,
            SchemaBootstrapOutcome::UpgradedSupportedPredecessor
        );
        storage
            .with_admin_operation(|database| {
                Box::pin(async move {
                    let state = read_context_and_state(&database)
                        .await?
                        .expect("revision-160 state after exact upgrade");
                    assert_eq!(state.revision, SCHEMA_REVISION);
                    assert!(state.is_exact_current());
                    let mut sentinel = database
                        .query("RETURN workspaces:revision159_sentinel.name;")
                        .await?;
                    assert_eq!(
                        sentinel.take::<Option<String>>(0)?.as_deref(),
                        Some("revision159-sentinel")
                    );
                    Ok(())
                })
            })
            .await
            .expect("verify revision-160 current state after upgrade");
        storage
            .shutdown()
            .await
            .expect("close upgraded standalone Loom predecessor store");
        let reopened = open_test_storage(&directory)
            .await
            .expect("reopen upgraded standalone Loom store");
        let restarted = bootstrap_schema(&reopened)
            .await
            .expect("reuse current standalone Loom schema after restart");
        assert_eq!(
            restarted.info_fingerprint_sha256,
            EXPECTED_SCHEMA_INFO_SHA256
        );
        assert_eq!(
            restarted.outcome,
            SchemaBootstrapOutcome::ReusedExactCurrent
        );
        reopened
            .shutdown()
            .await
            .expect("close restarted standalone Loom store");
    }

    /// MT-141 R9: the exact MT-150 pin is the current script with the MT-150-era
    /// `atelier_media_source_provenance_ref.asset_id` definition restored, proven byte-exact
    /// against `PRE_MT141_GENERATED_SURREALQL_SHA256`.
    fn mt141_pin_schema() -> String {
        // The MT-141 lines are checked on the pre-MT-154 text: MT-154 rewrote the saved-search
        // projection table (owner_account_id, record-user permissions) after MT-141.
        let base = restore_pre_mt154_schema(SCHEMA.to_owned());
        assert_eq!(
            base.matches(MT141_PROVENANCE_REF_ASSERT_LINE).count(),
            1,
            "MT-141 line drifted: {MT141_PROVENANCE_REF_ASSERT_LINE}"
        );
        assert_eq!(
            base.matches(PRE_MT141_PROVENANCE_REF_ASSET_ID_LINE).count(),
            0
        );
        assert_eq!(base.matches(MT141_SAVED_SEARCH_PROJECTION_BLOCK).count(), 1);
        for (current, previous) in [
            (MT141_LOOM_PIN_ORDER_LINE, PRE_MT141_LOOM_PIN_ORDER_LINE),
            (
                MT141_QUICK_SWITCHER_SOURCE_KIND_LINE,
                PRE_MT141_QUICK_SWITCHER_SOURCE_KIND_LINE,
            ),
            (
                MT141_QUICK_SWITCHER_RESULT_KIND_LINE,
                PRE_MT141_QUICK_SWITCHER_RESULT_KIND_LINE,
            ),
            (
                MT141_BLOCK_VIEW_OUTBOX_BLOCK_LINE,
                PRE_MT141_BLOCK_VIEW_OUTBOX_BLOCK_LINE,
            ),
        ] {
            assert_eq!(
                base.matches(current).count(),
                1,
                "MT-141 line drifted: {current}"
            );
            assert_eq!(base.matches(previous).count(), 0);
        }
        assert_eq!(base.matches(MT141_AI_EDIT_APPLIED_BINDING_LINES).count(), 1);
        let pinned = pre_account_setup_schema()
            .replace(
                MT141_PROVENANCE_REF_ASSERT_LINE,
                PRE_MT141_PROVENANCE_REF_ASSET_ID_LINE,
            )
            .replace(MT141_SAVED_SEARCH_PROJECTION_BLOCK, "")
            .replace(MT141_LOOM_PIN_ORDER_LINE, PRE_MT141_LOOM_PIN_ORDER_LINE)
            .replace(
                MT141_QUICK_SWITCHER_SOURCE_KIND_LINE,
                PRE_MT141_QUICK_SWITCHER_SOURCE_KIND_LINE,
            )
            .replace(
                MT141_QUICK_SWITCHER_RESULT_KIND_LINE,
                PRE_MT141_QUICK_SWITCHER_RESULT_KIND_LINE,
            )
            .replace(
                MT141_BLOCK_VIEW_OUTBOX_BLOCK_LINE,
                PRE_MT141_BLOCK_VIEW_OUTBOX_BLOCK_LINE,
            )
            .replace(MT141_AI_EDIT_APPLIED_BINDING_LINES, "");
        assert_eq!(
            sha256_hex(pinned.as_bytes()),
            PRE_MT141_GENERATED_SURREALQL_SHA256,
            "the pre-MT-141 allowlist must be exactly the current script with the MT-150-era asset_id line"
        );
        pinned
    }

    /// MT-141 R9: the upgrade DDL is byte-identical (whitespace-normalised) to the fresh-script
    /// `asset_id` definition, it carries the composite constraint, every older lineage's upgrade
    /// query carries it, and the lineage pins moved.
    #[tokio::test]
    async fn local_account_setup_upgrade_preserves_revision_157_and_restarts_current() {
        let previous_schema = pre_account_setup_schema();
        assert_eq!(
            sha256_hex(previous_schema.as_bytes()),
            PRE_ACCOUNT_SETUP_GENERATED_SHA256
        );
        let directory = tempfile::tempdir().expect("temporary account setup predecessor");
        let storage = open_test_storage(&directory)
            .await
            .expect("open predecessor store");
        storage.with_admin_operation(move |database| Box::pin(async move {
            database.query_bound(previous_schema.as_str(), BootstrapBindings {
                schema_version: SCHEMA_VERSION.to_owned(),
                schema_revision: PRE_ACCOUNT_SETUP_REVISION,
                namespace: DEFAULT_NAMESPACE.to_owned(),
                database: DEFAULT_DATABASE.to_owned(),
                source_manifest_sha256: SCHEMA_LINEAGE_SHA256.to_owned(),
                generated_surql_sha256: PRE_ACCOUNT_SETUP_GENERATED_SHA256.to_owned(),
            }).await?;
            ensure_knowledge_schema_registry(&database).await?;
            assert_eq!(read_schema_catalog(&database).await?.info_fingerprint_sha256, PRE_ACCOUNT_SETUP_INFO_SHA256);
            database.query(format!("UPDATE handshake_schema_state:primary SET apply_state = 'complete', info_fingerprint_sha256 = '{PRE_ACCOUNT_SETUP_INFO_SHA256}'; CREATE local_accounts:legacy CONTENT {{ account_key: 'legacy', account_role: 'Member', status: 'enabled', revocation_epoch: 0, policy_version: 1, created_at: time::now(), updated_at: time::now() }};")).await?;
            database.query("CREATE principals:legacy CONTENT { principal_key: 'legacy', account_id: local_accounts:legacy, principal_kind: 'human_account', actor_kind: 'operator', actor_id: 'legacy', capability_profile_id: 'legacy', delegated_capabilities: ['fs.write'], status: 'enabled', revocation_epoch: 0, policy_version: 1, created_at: time::now(), updated_at: time::now() }; CREATE access_spaces:legacy CONTENT { space_key: 'legacy', account_id: local_accounts:legacy, name: 'legacy', status: 'active', revocation_epoch: 0, policy_version: 1, created_at: time::now(), updated_at: time::now() }; CREATE authenticated_sessions:legacy CONTENT { account_id: local_accounts:legacy, principal_id: principals:legacy, access_space_id: access_spaces:legacy, token_hash: crypto::sha256('revision157-nonce-proof'), channel_binding_hash: NONE, authentication_strength: 'test', delegated_capabilities: ['fs.write'], delegation_chain: [], account_revocation_epoch: 0, principal_revocation_epoch: 0, space_revocation_epoch: 0, policy_version: 1, issued_at: time::now(), expires_at: time::now() + 1h, revoked_at: NONE };").await?;
            Ok(())
        })).await.expect("seed exact revision 157 without password/setup credentials");
        let upgraded = bootstrap_schema(&storage)
            .await
            .expect("upgrade exact predecessor");
        assert_eq!(
            upgraded.info_fingerprint_sha256,
            EXPECTED_SCHEMA_INFO_SHA256
        );
        storage.with_admin_operation(|database| Box::pin(async move {
            let mut result = database.query("SELECT account_key, account_role, password_verifier FROM local_accounts; SELECT * FROM local_account_setup;").await?;
            #[derive(SurrealValue)]
            struct LegacyAccount {
                account_key: String,
                account_role: String,
                password_verifier: Option<String>,
            }
            let accounts: Vec<LegacyAccount> = result.take(0)?;
            let setups: Vec<SurrealValueData> = result.take(1)?;
            assert_eq!(accounts.len(), 1);
            assert_eq!(accounts[0].account_key, "legacy");
            assert_eq!(accounts[0].account_role, "Member");
            assert!(accounts[0].password_verifier.is_none());
            assert!(setups.is_empty(), "migration must not claim installation ownership");
            Ok(())
        })).await.expect("preserve account and no auto setup");
        storage.with_admin_operation(|database| Box::pin(async move {
            database.query(
                "CREATE protected_resources:legacy_resource CONTENT { resource_kind: 'workspace', external_resource_id: 'legacy-workspace', owner_account_id: local_accounts:legacy, created_by_principal_id: principals:legacy, created_in_session_id: authenticated_sessions:legacy, creator_grant_id: NONE, access_space_id: access_spaces:legacy, parent_resource_id: NONE, schema_version: 1, lifecycle_state: 'active', policy_version: 1, classification: 'account_private', storage_locator_hash: crypto::sha256('revision157-resource'), created_at: time::now(), updated_at: time::now() }; CREATE resource_grants:legacy_grant CONTENT { account_id: local_accounts:legacy, principal_id: principals:legacy, access_space_id: access_spaces:legacy, resource_id: protected_resources:legacy_resource, actions: ['read','create','update','delete'], capability_ids: ['fs.read','fs.write'], delegation_chain: [], status: 'active', grant_version: 1, policy_version: 1, expires_at: NONE, revoked_at: NONE, created_at: time::now(), updated_at: time::now() }; UPDATE protected_resources:legacy_resource SET creator_grant_id = resource_grants:legacy_grant;"
            ).await?.check()?;
            Ok(())
        })).await.expect("seed real protected resource and grant for record-user nonce proof");
        let scope = super::super::resource_authority::RecordUserScope {
            grant_id: Some("legacy_grant".to_owned()),
            workspace_id: Some("legacy-workspace".to_owned()),
            session_token: "revision157-nonce-proof".to_owned(),
            channel_binding_hash: None,
            resource_id: "nonce-only-proof".to_owned(),
            session_id: "legacy".to_owned(),
            capability_id: "fs.write".to_owned(),
            action: super::super::resource_authority::ResourceAction::Update,
        };
        storage.with_record_user_scope(scope, storage.with_data_operation(|database| Box::pin(async move {
            let authority_rows = [
                "local_accounts:legacy",
                "principals:legacy",
                "access_spaces:legacy",
                "authenticated_sessions:legacy",
                "protected_resources:legacy_resource",
                "resource_grants:legacy_grant",
            ];
            for row in authority_rows {
                let mut positive = database.client.query(format!(
                    "UPDATE {row} SET authorization_touch_nonce = (authorization_touch_nonce ?? 0) + 1 RETURN VALUE authorization_touch_nonce;"
                )).await?.check()?;
                let touched: Vec<i64> = positive.take(0)?;
                assert_eq!(touched, vec![1], "{row} accepts its first record-user nonce increment");
            }

            let tamper_cases = [
                ("local_accounts:legacy", "account_role = 'Owner'"),
                ("principals:legacy", "actor_id = 'tampered'"),
                ("access_spaces:legacy", "name = 'tampered'"),
                ("authenticated_sessions:legacy", "authentication_strength = 'tampered'"),
                ("protected_resources:legacy_resource", "classification = 'tampered'"),
                ("resource_grants:legacy_grant", "grant_version = 2"),
            ];
            for (row, mutation) in tamper_cases {
                let error = database.client.query(format!(
                    "UPDATE {row} SET authorization_touch_nonce = authorization_touch_nonce + 1, {mutation} RETURN NONE;"
                )).await?.check().expect_err("immutable authority tamper must fail");
                assert!(
                    error.to_string().contains("HSK-403-PROTECTED-RESOURCE"),
                    "{row} immutable tamper must report the authority guard: {error}"
                );
            }
            for row in authority_rows {
                let error = database.client.query(format!(
                    "UPDATE {row} SET authorization_touch_nonce = authorization_touch_nonce + 2 RETURN NONE;"
                )).await?.check().expect_err("non-unit authority nonce increment must fail");
                assert!(
                    error.to_string().contains("HSK-403-PROTECTED-RESOURCE"),
                    "{row} non-unit nonce increment must report the authority guard: {error}"
                );
            }

            let mut reread = database.client.query(
                "SELECT VALUE authorization_touch_nonce FROM local_accounts:legacy;                  SELECT VALUE authorization_touch_nonce FROM principals:legacy;                  SELECT VALUE authorization_touch_nonce FROM access_spaces:legacy;                  SELECT VALUE authorization_touch_nonce FROM authenticated_sessions:legacy;                  SELECT VALUE authorization_touch_nonce FROM protected_resources:legacy_resource;                  SELECT VALUE authorization_touch_nonce FROM resource_grants:legacy_grant;                  SELECT VALUE account_role FROM local_accounts:legacy;                  SELECT VALUE actor_id FROM principals:legacy;                  SELECT VALUE name FROM access_spaces:legacy;                  SELECT VALUE authentication_strength FROM authenticated_sessions:legacy;                  SELECT VALUE classification FROM protected_resources:legacy_resource;                  SELECT VALUE grant_version FROM resource_grants:legacy_grant;"
            ).await?.check()?;
            for statement_index in 0..6 {
                let nonce: Vec<i64> = reread.take(statement_index)?;
                assert_eq!(nonce, vec![1], "rejected tamper must not partially advance authority nonce");
            }
            let account_role: Vec<String> = reread.take(6)?;
            let actor_id: Vec<String> = reread.take(7)?;
            let space_name: Vec<String> = reread.take(8)?;
            let authentication_strength: Vec<String> = reread.take(9)?;
            let classification: Vec<String> = reread.take(10)?;
            let grant_version: Vec<i64> = reread.take(11)?;
            assert_eq!(account_role, vec!["Member"]);
            assert_eq!(actor_id, vec!["legacy"]);
            assert_eq!(space_name, vec!["legacy"]);
            assert_eq!(authentication_strength, vec!["test"]);
            assert_eq!(classification, vec!["account_private"]);
            assert_eq!(grant_version, vec![1]);
            Ok(())
        }))).await.expect("record-user nonce updates and immutable tamper rejection on preserved revision157 authority");
        storage.shutdown().await.expect("close upgraded store");
        let reopened = open_test_storage(&directory)
            .await
            .expect("reopen upgraded store");
        let report = bootstrap_schema(&reopened)
            .await
            .expect("reuse current after restart");
        assert_eq!(report.info_fingerprint_sha256, EXPECTED_SCHEMA_INFO_SHA256);
        reopened.shutdown().await.expect("close current store");
    }

    #[test]
    fn mt141_upgrade_statements_match_schema() {
        fn statements(source: &str) -> Vec<String> {
            source
                .lines()
                .filter(|line| !line.trim_start().starts_with("--"))
                .collect::<Vec<_>>()
                .join("\n")
                .split(';')
                .map(|statement| statement.split_whitespace().collect::<Vec<_>>().join(" "))
                .filter(|statement| statement.starts_with("DEFINE "))
                .collect()
        }
        let upgrade = statements(&mt141_upgrade_statements());
        // provenance asset_id + 18 saved-search statements + pin_order + 2 quick-switcher
        // kinds + the block-view outbox block_id + 2 applied-binding fields (V2-R2).
        assert_eq!(upgrade.len(), 1 + 18 + 1 + 2 + 1 + 2);
        // Compared with the pre-MT-154 text; MT-154 re-emits its own rewrite of these tables.
        let schema = statements(&restore_pre_mt154_schema(SCHEMA.to_owned()));
        for statement in &upgrade {
            assert!(
                schema.iter().any(|s| s == statement),
                "MT-141 upgrade DDL drifted from schema.surql: {statement}"
            );
        }
        assert!(TABLE_NAMES.contains(&"atelier_saved_search_retrieval_projection"));
        assert!(upgrade[0].contains(
            "ON TABLE atelier_media_source_provenance_ref TYPE record<atelier_media_asset>"
        ));
        for field in [
            "source_url_ref",
            "source_path_ref",
            "source_note_ref",
            "contact_sheet_ref",
            "task_ref",
            "run_ref",
        ] {
            assert!(
                upgrade[0].contains(&format!("$this.{field} != NONE")),
                "constraint names {field}"
            );
        }
        assert_eq!(
            PRE_MT109_SCHEMA
                .matches(MT141_PROVENANCE_REF_ASSERT_LINE)
                .count(),
            0
        );
        assert!(post_mt109_upgrade_statements().contains(&mt141_upgrade_statements()));
        assert!(mt109_authority_upgrade_query().contains(&mt141_upgrade_statements()));
        assert_ne!(
            PRE_MT141_GENERATED_SURREALQL_SHA256,
            GENERATED_SURREALQL_SHA256
        );
        assert_ne!(PRE_MT141_SCHEMA_INFO_SHA256, EXPECTED_SCHEMA_INFO_SHA256);
        // The MT-141 predecessor is the MT-150 current pin, so the hops chain.
        assert_ne!(
            PRE_MT141_GENERATED_SURREALQL_SHA256,
            PRE_MT150_GENERATED_SURREALQL_SHA256
        );
        let _ = mt141_pin_schema();
    }

    /// MT-141 R9: a store at the exact MT-150 pin (proven byte-exact against
    /// `PRE_MT141_GENERATED_SURREALQL_SHA256`) holding a media asset and a valid provenance row is
    /// upgraded in place: the row survives, the live fingerprint is the current pin, the
    /// constraint now rejects an all-NONE provenance row while still accepting a one-ref row, and
    /// the state survives a reopen.
    #[tokio::test]
    async fn mt141_exact_mt150_pin_upgrade_moves_provenance_ref_constraint_and_restarts_current() {
        let mt141_pin_schema = mt141_pin_schema();
        let directory = tempfile::tempdir().expect("temporary MT-150-pin store");
        let storage = open_test_storage(&directory)
            .await
            .expect("open MT-150-pin store");
        storage
            .with_admin_operation(|database| {
                Box::pin(async move {
                    database
                        .query_bound(
                            mt141_pin_schema.as_str(),
                            BootstrapBindings {
                                schema_version: SCHEMA_VERSION.to_owned(),
                                schema_revision: PRE_ACCOUNT_SETUP_REVISION,
                                namespace: DEFAULT_NAMESPACE.to_owned(),
                                database: DEFAULT_DATABASE.to_owned(),
                                source_manifest_sha256: SCHEMA_LINEAGE_SHA256.to_owned(),
                                generated_surql_sha256: PRE_MT141_GENERATED_SURREALQL_SHA256
                                    .to_owned(),
                            },
                        )
                        .await?;
                    ensure_knowledge_schema_registry(&database).await?;
                    let before = read_schema_catalog(&database).await?;
                    assert_eq!(
                        before.info_fingerprint_sha256, PRE_MT141_SCHEMA_INFO_SHA256,
                        "the synthesized MT-150-pin store must carry the exact MT-150 live fingerprint"
                    );
                    database
                        .query(format!(
                            "UPDATE ONLY {BOOTSTRAP_STATE_ID} SET \
                             info_fingerprint_sha256 = '{PRE_MT141_SCHEMA_INFO_SHA256}', \
                             apply_state = 'complete', updated_at = time::now(); \
                             CREATE atelier_media_asset:u'018f0000-0000-7000-8000-000000000141' CONTENT {{ \
                             asset_id: u'018f0000-0000-7000-8000-000000000141', content_hash: 'mt141-pin', \
                             mime: 'image/png', byte_len: 1, artifact_ref: 'artifact://mt141/pin' }}; \
                             CREATE atelier_media_source_provenance_ref:u'018f0000-0000-7000-8000-000000000141' CONTENT {{ \
                             asset_id: atelier_media_asset:u'018f0000-0000-7000-8000-000000000141', \
                             source_url_ref: 'https://example.invalid/mt141', updated_by: 'mt141-pin' }};"
                        ))
                        .await?
                        .check()?;
                    Ok(())
                })
            })
            .await
            .expect("construct exact MT-150-pin store with a media asset and a provenance row");
        storage.shutdown().await.expect("close MT-150-pin store");

        let reopened = open_test_storage(&directory)
            .await
            .expect("reopen MT-150-pin store");
        let upgraded = match bootstrap_schema(&reopened).await {
            Ok(report) => report,
            Err(error) => {
                let reference = fresh_mem_catalog().await;
                let observed = reopened
                    .with_admin_operation(|database| {
                        Box::pin(async move { canonical_catalog(&database).await })
                    })
                    .await
                    .expect("inspect the failed upgrade");
                report_catalog_drift("MT141_UPGRADE", &reference, &observed);
                panic!("upgrade exact MT-150-pin store: {error}");
            }
        };
        assert!(upgraded.reused_existing_schema);
        assert_eq!(
            upgraded.outcome,
            SchemaBootstrapOutcome::UpgradedSupportedPredecessor
        );
        assert_eq!(upgraded.generated_surql_sha256, GENERATED_SURREALQL_SHA256);
        assert_eq!(
            upgraded.info_fingerprint_sha256,
            EXPECTED_SCHEMA_INFO_SHA256
        );
        reopened
            .with_admin_operation(|database| {
                Box::pin(async move {
                    let mut survived = database
                        .query("RETURN atelier_media_source_provenance_ref:u'018f0000-0000-7000-8000-000000000141'.source_url_ref;")
                        .await?;
                    let url: Option<String> = survived.take(0)?;
                    assert_eq!(url.as_deref(), Some("https://example.invalid/mt141"));
                    database
                        .query(
                            "CREATE atelier_media_asset:u'018f0000-0000-7000-8000-000000000142' CONTENT {                              asset_id: u'018f0000-0000-7000-8000-000000000142', content_hash: 'mt141-pin-2',                              mime: 'image/png', byte_len: 1, artifact_ref: 'artifact://mt141/pin-2' };",
                        )
                        .await?
                        .check()?;
                    let rejected = match database
                        .query(
                            "CREATE atelier_media_source_provenance_ref:u'018f0000-0000-7000-8000-000000000142' CONTENT {                              asset_id: atelier_media_asset:u'018f0000-0000-7000-8000-000000000142', updated_by: 'mt141-pin' };",
                        )
                        .await
                    {
                        Ok(response) => response.check().map(|_| ()).map_err(|e| e.to_string()),
                        Err(error) => Err(error.to_string()),
                    };
                    let rejection =
                        rejected.expect_err("an all-NONE provenance row must be rejected after the upgrade");
                    assert!(
                        rejection.contains("$this.run_ref != NONE"),
                        "rejected by the moved constraint: {rejection}"
                    );
                    let accepted = match database
                        .query(
                            "CREATE atelier_media_source_provenance_ref:u'018f0000-0000-7000-8000-000000000142' CONTENT {                              asset_id: atelier_media_asset:u'018f0000-0000-7000-8000-000000000142',                              task_ref: 'task://mt141', updated_by: 'mt141-pin' };",
                        )
                        .await
                    {
                        Ok(response) => response.check().map(|_| ()).map_err(|e| e.to_string()),
                        Err(error) => Err(error.to_string()),
                    };
                    assert!(accepted.is_ok(), "a one-ref provenance row is accepted: {accepted:?}");
                    Ok(())
                })
            })
            .await
            .expect("inspect upgraded MT-150-pin store");
        reopened.shutdown().await.expect("close upgraded store");

        let restarted = open_test_storage(&directory)
            .await
            .expect("reopen upgraded store");
        let reused = bootstrap_schema(&restarted)
            .await
            .expect("bootstrap on the upgraded store");
        assert_eq!(reused.outcome, SchemaBootstrapOutcome::ReusedExactCurrent);
        restarted.shutdown().await.expect("close restarted store");
    }

    /// MT-150: the exact MT-109 pin is the current script minus the MT-150 `loom_edges`
    /// receipt block, proven byte-exact against `PRE_MT150_GENERATED_SURREALQL_SHA256`.
    fn mt150_pin_schema() -> String {
        assert_eq!(
            SCHEMA
                .matches(MT150_LOOM_EDGE_RECEIPT_UPGRADE_STATEMENTS)
                .count(),
            1,
            "MT-150 block drifted: {MT150_LOOM_EDGE_RECEIPT_UPGRADE_STATEMENTS}"
        );
        // The MT-141 R9 constraint move came after MT-150, so the MT-109 pin is the current
        // script minus the MT-150 block with the MT-150-era `asset_id` definition restored.
        let pinned = mt141_pin_schema().replace(MT150_LOOM_EDGE_RECEIPT_UPGRADE_STATEMENTS, "");
        assert_eq!(
            sha256_hex(pinned.as_bytes()),
            PRE_MT150_GENERATED_SURREALQL_SHA256,
            "the pre-MT-150 allowlist must be exactly the current script minus the MT-150 block"
        );
        pinned
    }

    /// MT-150: the upgrade DDL is byte-identical (whitespace-normalised) to the fresh-script
    /// `loom_edges.event_ledger_event_id` / `idx_loom_edges_event` lines, the field precedes its
    /// index, every older lineage's upgrade query carries the block, and the lineage pins moved.
    #[test]
    fn mt150_upgrade_statements_match_schema() {
        fn statements(source: &str) -> Vec<String> {
            source
                .lines()
                .filter(|line| !line.trim_start().starts_with("--"))
                .collect::<Vec<_>>()
                .join("\n")
                .split(';')
                .map(|statement| statement.split_whitespace().collect::<Vec<_>>().join(" "))
                .filter(|statement| statement.starts_with("DEFINE "))
                .collect()
        }
        let upgrade = statements(MT150_LOOM_EDGE_RECEIPT_UPGRADE_STATEMENTS);
        assert_eq!(upgrade.len(), 2);
        let schema = statements(SCHEMA);
        let field_at = schema
            .iter()
            .position(|s| s == &upgrade[0])
            .expect("MT-150 field in schema.surql");
        let index_at = schema
            .iter()
            .position(|s| s == &upgrade[1])
            .expect("MT-150 index in schema.surql");
        assert!(
            field_at < index_at,
            "event_ledger_event_id field must precede idx_loom_edges_event"
        );
        assert!(upgrade[0].contains("ON TABLE loom_edges TYPE option<record<kernel_event_ledger>>"));
        assert!(upgrade[0].contains("REFERENCE ON DELETE REJECT"));
        assert!(upgrade[1]
            .contains("idx_loom_edges_event ON TABLE loom_edges FIELDS event_ledger_event_id"));
        // The MT-109 pin (schema_pre_mt109.surql) predates the block, so it must NOT carry it.
        assert_eq!(
            PRE_MT109_SCHEMA
                .matches(MT150_LOOM_EDGE_RECEIPT_UPGRADE_STATEMENTS)
                .count(),
            0
        );
        assert!(
            post_mt109_upgrade_statements().contains(MT150_LOOM_EDGE_RECEIPT_UPGRADE_STATEMENTS)
        );
        assert!(
            mt109_authority_upgrade_query().contains(MT150_LOOM_EDGE_RECEIPT_UPGRADE_STATEMENTS)
        );
        assert_ne!(
            PRE_MT150_GENERATED_SURREALQL_SHA256,
            GENERATED_SURREALQL_SHA256
        );
        assert_ne!(PRE_MT150_SCHEMA_INFO_SHA256, EXPECTED_SCHEMA_INFO_SHA256);
        // The MT-150 predecessor is the MT-109 current pin, so the hops chain.
        assert_ne!(
            PRE_MT150_GENERATED_SURREALQL_SHA256,
            PRE_MT109_GENERATED_SURREALQL_SHA256
        );
        let _ = mt150_pin_schema();
    }

    /// MT-150: a store at the exact MT-109 pin (the current script minus the MT-150 block,
    /// proven byte-exact against `PRE_MT150_GENERATED_SURREALQL_SHA256`) holding a workspace, two
    /// blocks and a tag edge written before the receipt field existed is upgraded in place:
    /// `loom_edges.event_ledger_event_id` exists, the pre-existing edge survives with `NONE`, the
    /// live fingerprint is the current pin, and the state survives a reopen.
    #[tokio::test]
    async fn mt150_exact_mt109_pin_upgrade_adds_loom_edge_receipt_field_and_restarts_current() {
        let mt150_pin_schema = mt150_pin_schema();
        let directory = tempfile::tempdir().expect("temporary MT-109-pin store");
        let storage = open_test_storage(&directory)
            .await
            .expect("open MT-109-pin store");
        storage
            .with_admin_operation(|database| {
                Box::pin(async move {
                    database
                        .query_bound(
                            mt150_pin_schema.as_str(),
                            BootstrapBindings {
                                schema_version: SCHEMA_VERSION.to_owned(),
                                schema_revision: PRE_ACCOUNT_SETUP_REVISION,
                                namespace: DEFAULT_NAMESPACE.to_owned(),
                                database: DEFAULT_DATABASE.to_owned(),
                                source_manifest_sha256: SCHEMA_LINEAGE_SHA256.to_owned(),
                                generated_surql_sha256: PRE_MT150_GENERATED_SURREALQL_SHA256
                                    .to_owned(),
                            },
                        )
                        .await?;
                    ensure_knowledge_schema_registry(&database).await?;
                    let before = read_schema_catalog(&database).await?;
                    assert_eq!(
                        before.info_fingerprint_sha256, PRE_MT150_SCHEMA_INFO_SHA256,
                        "the synthesized MT-109-pin store must carry the exact MT-109 live fingerprint"
                    );
                    database
                        .query(format!(
                            "UPDATE ONLY {BOOTSTRAP_STATE_ID} SET \
                             info_fingerprint_sha256 = '{PRE_MT150_SCHEMA_INFO_SHA256}', \
                             apply_state = 'complete', updated_at = time::now(); \
                             CREATE workspaces:mt150_pin CONTENT {{ name: 'sentinel' }}; \
                             CREATE loom_blocks:mt150_src CONTENT {{ block_id: 'mt150_src', \
                             workspace_id: workspaces:mt150_pin, content_type: 'note', title: 'src' }}; \
                             CREATE loom_blocks:mt150_hub CONTENT {{ block_id: 'mt150_hub', \
                             workspace_id: workspaces:mt150_pin, content_type: 'note', title: 'hub' }}; \
                             CREATE loom_edges:mt150_edge CONTENT {{ edge_id: 'mt150_edge', \
                             workspace_id: workspaces:mt150_pin, source_block_id: loom_blocks:mt150_src, \
                             target_block_id: loom_blocks:mt150_hub, edge_type: 'tag', created_by: 'user' }};"
                        ))
                        .await?
                        .check()?;
                    Ok(())
                })
            })
            .await
            .expect("construct exact MT-109-pin store with a workspace, blocks and an edge");
        storage.shutdown().await.expect("close MT-109-pin store");

        let reopened = open_test_storage(&directory)
            .await
            .expect("reopen MT-109-pin store");
        let upgraded = match bootstrap_schema(&reopened).await {
            Ok(report) => report,
            Err(error) => {
                let reference = fresh_mem_catalog().await;
                let observed = reopened
                    .with_admin_operation(|database| {
                        Box::pin(async move { canonical_catalog(&database).await })
                    })
                    .await
                    .expect("inspect the failed upgrade");
                report_catalog_drift("MT150_UPGRADE", &reference, &observed);
                panic!("upgrade exact MT-109-pin store: {error}");
            }
        };
        assert!(upgraded.reused_existing_schema);
        assert_eq!(
            upgraded.outcome,
            SchemaBootstrapOutcome::UpgradedSupportedPredecessor
        );
        assert_eq!(upgraded.generated_surql_sha256, GENERATED_SURREALQL_SHA256);
        assert_eq!(
            upgraded.info_fingerprint_sha256,
            EXPECTED_SCHEMA_INFO_SHA256
        );
        reopened
            .with_admin_operation(|database| {
                Box::pin(async move {
                    let mut edges = database
                        .query("INFO FOR TABLE loom_edges STRUCTURE;")
                        .await?;
                    let info: SurrealValueData = edges.take(0)?;
                    let fields = parse_named_array(&info, "fields")
                        .unwrap_or_else(|reason| panic!("invalid loom_edges INFO: {reason}"));
                    assert!(fields.iter().any(|field| field == "event_ledger_event_id"));
                    let indexes = parse_named_array(&info, "indexes")
                        .unwrap_or_else(|reason| panic!("invalid loom_edges INFO: {reason}"));
                    assert!(indexes.iter().any(|index| index == "idx_loom_edges_event"));
                    let mut edge = database
                        .query(
                            "RETURN loom_edges:mt150_edge.edge_id; \
                             RETURN loom_edges:mt150_edge.event_ledger_event_id;",
                        )
                        .await?;
                    let edge_id: Option<String> = edge.take(0)?;
                    assert_eq!(edge_id.as_deref(), Some("mt150_edge"));
                    let receipt: Option<RecordId> = edge.take(1)?;
                    assert!(
                        receipt.is_none(),
                        "an edge written before MT-150 keeps NONE; no receipt is fabricated"
                    );
                    Ok(())
                })
            })
            .await
            .expect("inspect upgraded MT-109-pin store");
        reopened.shutdown().await.expect("close upgraded store");

        let restarted = open_test_storage(&directory)
            .await
            .expect("reopen upgraded store");
        let reused = bootstrap_schema(&restarted)
            .await
            .expect("bootstrap on the upgraded store");
        assert_eq!(reused.outcome, SchemaBootstrapOutcome::ReusedExactCurrent);
        restarted
            .with_admin_operation(|database| {
                Box::pin(async move {
                    let state = read_context_and_state(&database)
                        .await?
                        .expect("upgraded state survives reopen");
                    assert!(state.is_exact_current());
                    let mut sentinel = database.query("RETURN workspaces:mt150_pin.name;").await?;
                    let name: Option<String> = sentinel.take(0)?;
                    assert_eq!(name.as_deref(), Some("sentinel"));
                    Ok(())
                })
            })
            .await
            .expect("verify restarted upgraded store");
        restarted.shutdown().await.expect("close restarted store");
    }

    /// MT-152: a store at the exact MT-151 pin (the current script minus the MT-152 block,
    /// proven byte-exact against `PRE_MT152_GENERATED_SURREALQL_SHA256`) holding a workspace
    /// and a FEMS pack is upgraded in place: `fems_workspace_write_anchors` exists and accepts
    /// an UPSERT, the live fingerprint is the current pin, every application row survives, and
    /// the state survives a reopen.
    #[tokio::test]
    async fn mt152_exact_mt151_pin_upgrade_adds_fems_write_anchors_and_restarts_current() {
        let mt151_pin_schema = mt151_pin_schema();
        let directory = tempfile::tempdir().expect("temporary MT-151-pin store");
        let storage = open_test_storage(&directory)
            .await
            .expect("open MT-151-pin store");
        storage
            .with_admin_operation(|database| {
                Box::pin(async move {
                    database
                        .query_bound(
                            mt151_pin_schema.as_str(),
                            BootstrapBindings {
                                schema_version: SCHEMA_VERSION.to_owned(),
                                schema_revision: PRE_ACCOUNT_SETUP_REVISION,
                                namespace: DEFAULT_NAMESPACE.to_owned(),
                                database: DEFAULT_DATABASE.to_owned(),
                                source_manifest_sha256: SCHEMA_LINEAGE_SHA256.to_owned(),
                                generated_surql_sha256: PRE_MT152_GENERATED_SURREALQL_SHA256
                                    .to_owned(),
                            },
                        )
                        .await?;
                    ensure_knowledge_schema_registry(&database).await?;
                    database
                        .query(format!(
                            "UPDATE ONLY {BOOTSTRAP_STATE_ID} SET \
                             info_fingerprint_sha256 = '{PRE_MT152_SCHEMA_INFO_SHA256}', \
                             apply_state = 'complete', updated_at = time::now(); \
                             CREATE workspaces:mt152_pin CONTENT {{ name: 'sentinel' }}; \
                             CREATE fems_memory_packs:mt152_pack CONTENT {{ pack_id: 'mt152_pack', \
                             workspace_id: workspaces:mt152_pin, scope_key: '', pack: {{ v: 1 }}, \
                             generated_at: time::now() }};"
                        ))
                        .await?
                        .check()?;
                    Ok(())
                })
            })
            .await
            .expect("construct exact MT-151-pin store with a workspace and a FEMS pack");
        storage.shutdown().await.expect("close MT-151-pin store");

        let reopened = open_test_storage(&directory)
            .await
            .expect("reopen MT-151-pin store");
        let upgraded = match bootstrap_schema(&reopened).await {
            Ok(report) => report,
            Err(error) => {
                let reference = fresh_mem_catalog().await;
                let observed = reopened
                    .with_admin_operation(|database| {
                        Box::pin(async move { canonical_catalog(&database).await })
                    })
                    .await
                    .expect("inspect the failed upgrade");
                report_catalog_drift("MT152_UPGRADE", &reference, &observed);
                panic!("upgrade exact MT-151-pin store: {error}");
            }
        };
        assert!(upgraded.reused_existing_schema);
        assert_eq!(
            upgraded.outcome,
            SchemaBootstrapOutcome::UpgradedSupportedPredecessor
        );
        assert_eq!(upgraded.generated_surql_sha256, GENERATED_SURREALQL_SHA256);
        assert_eq!(
            upgraded.info_fingerprint_sha256,
            EXPECTED_SCHEMA_INFO_SHA256
        );
        reopened
            .with_admin_operation(|database| {
                Box::pin(async move {
                    let mut anchors = database
                        .query("INFO FOR TABLE fems_workspace_write_anchors STRUCTURE;")
                        .await?;
                    let info: SurrealValueData = anchors.take(0)?;
                    let fields = parse_named_array(&info, "fields")
                        .unwrap_or_else(|reason| panic!("invalid anchors INFO: {reason}"));
                    assert!(fields.iter().any(|field| field == "claim_nonce"));
                    database
                        .query(
                            "UPSERT fems_workspace_write_anchors:mt152_pin SET \
                             anchor_key = 'mt152_pin', workspace_key = 'mt152_pin', \
                             claim_nonce = 'nonce-1';",
                        )
                        .await?
                        .check()?;
                    let mut pack = database
                        .query("RETURN fems_memory_packs:mt152_pack.pack_id;")
                        .await?;
                    let pack_id: Option<String> = pack.take(0)?;
                    assert_eq!(pack_id.as_deref(), Some("mt152_pack"));
                    Ok(())
                })
            })
            .await
            .expect("verify upgraded anchors table and surviving rows");
        reopened.shutdown().await.expect("close upgraded store");

        let current = open_test_storage(&directory)
            .await
            .expect("reopen upgraded store");
        current
            .with_admin_operation(|database| {
                Box::pin(async move {
                    let state = read_context_and_state(&database)
                        .await?
                        .expect("upgraded state survives reopen");
                    assert!(state.is_exact_current());
                    let mut sentinel = database.query("RETURN workspaces:mt152_pin.name;").await?;
                    let name: Option<String> = sentinel.take(0)?;
                    assert_eq!(name.as_deref(), Some("sentinel"));
                    Ok(())
                })
            })
            .await
            .expect("verify exact-current durable reopen after the MT-152 upgrade");
        current.shutdown().await.expect("close current store");
    }

    #[tokio::test]
    async fn bootstrap_resumes_exact_current_schema_applied_state() {
        let directory = tempfile::tempdir().expect("temporary Surreal directory");
        let storage = open_test_storage(&directory)
            .await
            .expect("open fresh store");

        storage
            .with_admin_operation(|database| {
                Box::pin(async move {
                    database
                        .query_bound(
                            SCHEMA,
                            BootstrapBindings {
                                schema_version: SCHEMA_VERSION.to_owned(),
                                schema_revision: SCHEMA_REVISION,
                                namespace: DEFAULT_NAMESPACE.to_owned(),
                                database: DEFAULT_DATABASE.to_owned(),
                                source_manifest_sha256: SCHEMA_LINEAGE_SHA256.to_owned(),
                                generated_surql_sha256: GENERATED_SURREALQL_SHA256.to_owned(),
                            },
                        )
                        .await?;
                    let pending = read_context_and_state(&database)
                        .await?
                        .expect("schema transaction must write pending state");
                    assert!(pending.is_schema_applied_current());
                    Ok(())
                })
            })
            .await
            .expect("install schema without finalization");

        let resumed = bootstrap_schema(&storage)
            .await
            .expect("resume exact-current schema_applied state");
        assert!(resumed.reused_existing_schema);
        assert_eq!(resumed.info_fingerprint_sha256, EXPECTED_SCHEMA_INFO_SHA256);
        storage
            .with_admin_operation(|database| {
                Box::pin(async move {
                    let finalized = read_context_and_state(&database)
                        .await?
                        .expect("finalized state must exist");
                    assert!(finalized.is_exact_current());
                    Ok(())
                })
            })
            .await
            .expect("post-verify finalized state");
        storage.shutdown().await.expect("close store");
    }

    #[tokio::test]
    async fn fresh_bootstrap_rejects_and_preserves_preexisting_data() {
        let directory = tempfile::tempdir().expect("temporary Surreal directory");
        let storage = open_test_storage(&directory)
            .await
            .expect("open fresh store");
        storage
            .with_admin_operation(|database| {
                Box::pin(async move {
                    database
                        .query("CREATE preexisting:keep SET marker = 'untouched';")
                        .await?;
                    Ok(())
                })
            })
            .await
            .expect("seed pre-existing record");

        let error = bootstrap_schema(&storage)
            .await
            .expect_err("non-empty database must be rejected");
        assert!(
            error
                .to_string()
                .contains("HANDSHAKE_SURREAL_SCHEMA_DATABASE_NOT_EMPTY"),
            "unexpected non-empty database error: {error}"
        );
        storage
            .with_admin_operation(|database| {
                Box::pin(async move {
                    let mut response = database.query("RETURN preexisting:keep.marker;").await?;
                    let marker: Option<String> = response.take(0)?;
                    let marker = marker.expect("pre-existing marker must remain readable");
                    assert_eq!(marker, "untouched");
                    Ok(())
                })
            })
            .await
            .expect("pre-existing record remains intact");
        storage.shutdown().await.expect("close store");
    }

    #[tokio::test]
    async fn bootstrap_rejects_lower_or_divergent_lineage() {
        let directory = tempfile::tempdir().expect("temporary Surreal directory");
        let storage = open_test_storage(&directory)
            .await
            .expect("open fresh store");
        storage
            .with_admin_operation(|database| {
                Box::pin(async move {
                    database
                        .query(format!(
                            "DEFINE TABLE handshake_schema_state SCHEMALESS; \
                             CREATE handshake_schema_state:primary CONTENT {{ \
                               version: '{SCHEMA_VERSION}', revision: 28, \
                               namespace: '{DEFAULT_NAMESPACE}', database: '{DEFAULT_DATABASE}', \
                               source_manifest_sha256: '{SCHEMA_LINEAGE_SHA256}', \
                               generated_surql_sha256: '{GENERATED_SURREALQL_SHA256}', \
                               info_fingerprint_sha256: '0000000000000000000000000000000000000000000000000000000000000000', \
                               apply_state: 'complete', target_revision: 28 \
                             }};"
                        ))
                        .await?;
                    Ok(())
                })
            })
            .await
            .expect("seed lower lineage");

        let error = bootstrap_schema(&storage)
            .await
            .expect_err("lower lineage must fail closed");
        assert!(error
            .to_string()
            .contains("HANDSHAKE_SURREAL_SCHEMA_UNSUPPORTED_LINEAGE"));
        storage.shutdown().await.expect("close store");
    }

    #[tokio::test]
    async fn exact_current_bootstrap_rejects_complete_info_tampering() {
        let tamper_queries = [
            (
                "index definition",
                "DEFINE INDEX OVERWRITE idx_ai_jobs_gc ON TABLE ai_jobs FIELDS created_at, status, is_pinned;",
            ),
            ("sequence removal", "REMOVE SEQUENCE kernel_event_sequence;"),
            (
                "field assertion",
                "DEFINE FIELD OVERWRITE size_bytes ON TABLE assets TYPE int ASSERT $value >= -1;",
            ),
        ];

        for (label, tamper_query) in tamper_queries {
            let directory = tempfile::tempdir().expect("temporary Surreal directory");
            let storage = open_test_storage(&directory)
                .await
                .expect("open fresh store");
            bootstrap_schema(&storage).await.expect("bootstrap schema");
            storage
                .with_admin_operation(|database| {
                    Box::pin(async move {
                        database.query(tamper_query).await?;
                        Ok(())
                    })
                })
                .await
                .unwrap_or_else(|error| panic!("apply {label} tamper: {error}"));

            let error = match bootstrap_schema(&storage).await {
                Ok(_) => panic!("{label} tamper must be rejected"),
                Err(error) => error,
            };
            assert!(
                error
                    .to_string()
                    .contains("HANDSHAKE_SURREAL_SCHEMA_INFO_FINGERPRINT_MISMATCH"),
                "unexpected {label} verdict: {error}"
            );
            storage.shutdown().await.expect("close store");
        }
    }

    #[tokio::test]
    async fn native_json_fields_round_trip_real_domain_serialization_and_reject_wrong_shapes() {
        let directory = tempfile::tempdir().expect("temporary Surreal directory");
        let storage = open_test_storage(&directory)
            .await
            .expect("open fresh store");
        bootstrap_schema(&storage).await.expect("bootstrap schema");

        let metrics = JobMetrics::zero();
        let entity_refs = vec![EntityRef {
            entity_id: "document:serde".to_owned(),
            entity_kind: "document".to_owned(),
        }];
        let planned_operations = vec![PlannedOperation {
            op_type: OperationType::Read,
            target: entity_refs[0].clone(),
            description: Some("read representative document".to_owned()),
        }];
        let metrics_json = serde_json::to_value(&metrics).expect("serialize JobMetrics");
        let entity_refs_json =
            serde_json::to_value(&entity_refs).expect("serialize EntityRef list");
        let planned_operations_json =
            serde_json::to_value(&planned_operations).expect("serialize PlannedOperation list");
        let job_inputs_json = serde_json::json!({ "document_id": "serde" });
        let expected = serde_json::json!({
            "entity_refs": entity_refs_json,
            "planned_operations": planned_operations_json,
            "metrics": metrics_json,
            "job_inputs": job_inputs_json,
        });

        storage
            .with_admin_operation(|database| {
                Box::pin(async move {
                    let mut response = database
                        .query_bound(
                            "CREATE ai_jobs:json_roundtrip SET \
                               trace_id = '00000000-0000-0000-0000-000000000001', \
                               job_kind = 'manual_prompt', status = 'queued', \
                               protocol_id = 'test', profile_id = 'test', \
                               capability_profile_id = 'test', access_mode = 'read_only', \
                               safety_mode = 'strict', entity_refs = $entity_refs, \
                               planned_operations = $planned_operations, metrics = $metrics, \
                               job_inputs = $job_inputs; \
                             RETURN { \
                               entity_refs: ai_jobs:json_roundtrip.entity_refs, \
                               planned_operations: ai_jobs:json_roundtrip.planned_operations, \
                               metrics: ai_jobs:json_roundtrip.metrics, \
                               job_inputs: ai_jobs:json_roundtrip.job_inputs \
                             };",
                            NativeJsonBindings {
                                entity_refs: expected["entity_refs"].clone(),
                                planned_operations: expected["planned_operations"].clone(),
                                metrics: expected["metrics"].clone(),
                                job_inputs: expected["job_inputs"].clone(),
                            },
                        )
                        .await?;
                    let observed: Option<JsonValue> = response.take(1)?;
                    let observed = observed.expect("native JSON readback must exist");
                    assert_eq!(observed, expected);
                    let restored_metrics: JobMetrics =
                        serde_json::from_value(observed["metrics"].clone())
                            .expect("deserialize JobMetrics readback");
                    assert_eq!(
                        serde_json::to_value(restored_metrics).expect("reserialize JobMetrics"),
                        expected["metrics"]
                    );
                    Ok(())
                })
            })
            .await
            .expect("native JSON bind and readback");

        for (label, wrong_shape) in [
            (
                "metrics string",
                "UPDATE ai_jobs:json_roundtrip SET metrics = 'not-an-object';",
            ),
            (
                "entity refs object",
                "UPDATE ai_jobs:json_roundtrip SET entity_refs = {};",
            ),
            (
                "job inputs array",
                "UPDATE ai_jobs:json_roundtrip SET job_inputs = [];",
            ),
        ] {
            let result = storage
                .with_admin_operation(|database| {
                    Box::pin(async move {
                        database.query(wrong_shape).await?;
                        Ok(())
                    })
                })
                .await;
            assert!(result.is_err(), "{label} must fail SCHEMAFULL validation");
        }
        storage.shutdown().await.expect("close store");
    }

    #[tokio::test]
    async fn record_references_reject_orphans_and_preserve_identity_semantics() {
        let directory = tempfile::tempdir().expect("temporary Surreal directory");
        let storage = open_test_storage(&directory)
            .await
            .expect("open fresh store");
        bootstrap_schema(&storage).await.expect("bootstrap schema");

        let orphan = storage
            .with_admin_operation(|database| {
                Box::pin(async move {
                    database
                        .query(
                            "CREATE documents:orphan SET \
                             workspace_id = workspaces:missing, title = 'orphan';",
                        )
                        .await?;
                    Ok(())
                })
            })
            .await;
        assert!(
            orphan.is_err(),
            "required orphan reference must be rejected"
        );

        storage
            .with_admin_operation(|database| {
                Box::pin(async move {
                    let mut response = database
                        .query(
                            "CREATE workspaces:identity SET name = 'Identity'; \
                             CREATE documents:child SET workspace_id = workspaces:identity, title = 'Child'; \
                             CREATE blocks:grandchild SET document_id = documents:child, \
                               kind = 'paragraph', sequence = 0, raw_content = 'raw', \
                               display_content = 'display', derived_content = {}; \
                             RETURN documents:child.workspace_id.name; \
                             DELETE workspaces:identity; \
                             RETURN record::exists(documents:child); \
                             RETURN record::exists(blocks:grandchild);",
                        )
                        .await?;
                    let dereferenced_name: Option<String> = response.take(3)?;
                    let child_remains: Option<bool> = response.take(5)?;
                    let grandchild_remains: Option<bool> = response.take(6)?;
                    let dereferenced_name =
                        dereferenced_name.expect("dereferenced workspace name must exist");
                    let child_remains = child_remains.expect("child existence result must exist");
                    let grandchild_remains =
                        grandchild_remains.expect("grandchild existence result must exist");
                    assert_eq!(dereferenced_name, "Identity");
                    assert!(!child_remains, "cascade must remove the referring record");
                    assert!(
                        !grandchild_remains,
                        "multi-hop cascade must remove the grandchild record"
                    );
                    Ok(())
                })
            })
            .await
            .expect("identity, dereference, and delete behavior");

        storage
            .with_admin_operation(|database| {
                Box::pin(async move {
                    database
                        .query(
                            "CREATE work_packets:wp_identity SET \
                             wp_id = 'wp_identity', version = 1, title = 'Identity', \
                             status = 'ready', priority = 1, task_board_status = 'READY', \
                             reporter = 'test', created_at = 'now', updated_at = 'now', \
                             vector_clock = '{}', metadata = '{}';",
                        )
                        .await?;
                    Ok(())
                })
            })
            .await
            .expect("matching business-key alias");
        let identity_change = storage
            .with_admin_operation(|database| {
                Box::pin(async move {
                    database
                        .query("UPDATE work_packets:wp_identity SET wp_id = 'different';")
                        .await?;
                    Ok(())
                })
            })
            .await;
        assert!(
            identity_change.is_err(),
            "business-key alias must be immutable"
        );
        storage.shutdown().await.expect("close store");
    }

    #[tokio::test]
    async fn optional_unset_and_reject_self_references_enforce_delete_contracts() {
        let directory = tempfile::tempdir().expect("temporary Surreal directory");
        let storage = open_test_storage(&directory)
            .await
            .expect("open fresh store");
        bootstrap_schema(&storage).await.expect("bootstrap schema");

        storage
            .with_admin_operation(|database| {
                Box::pin(async move {
                    let mut response = database
                        .query(
                            "CREATE workspaces:unset_ws SET name = 'Unset'; \
                             CREATE assets:unset_asset SET asset_id = 'unset_asset', \
                               workspace_id = workspaces:unset_ws, kind = 'file', \
                               mime = 'text/plain', content_hash = 'unset-hash', size_bytes = 1; \
                             CREATE loom_blocks:unset_block SET block_id = 'unset_block', \
                               workspace_id = workspaces:unset_ws, content_type = 'file', \
                               asset_id = assets:unset_asset, derived_json = {}; \
                             DELETE assets:unset_asset; \
                             RETURN record::exists(loom_blocks:unset_block); \
                             RETURN loom_blocks:unset_block.asset_id = NONE;",
                        )
                        .await?;
                    let block_remains: Option<bool> = response.take(4)?;
                    let reference_was_unset: Option<bool> = response.take(5)?;
                    let block_remains = block_remains.expect("block existence result must exist");
                    let reference_was_unset =
                        reference_was_unset.expect("UNSET comparison result must exist");
                    assert!(block_remains);
                    assert!(reference_was_unset);
                    Ok(())
                })
            })
            .await
            .expect("optional reference ON DELETE UNSET");

        storage
            .with_admin_operation(|database| {
                Box::pin(async move {
                    database
                        .query(
                            "CREATE adapter_checkpoint:parent SET created_at = 'now', \
                               base_model_name = 'base', adapter_type = 'lora', rank_r = 8, \
                               alpha = 16, learning_rate = 0.001, precision = 'f16', \
                               path = 'parent'; \
                             CREATE adapter_checkpoint:child SET created_at = 'now', \
                               parent_checkpoint_id = adapter_checkpoint:parent, \
                               base_model_name = 'base', adapter_type = 'lora', rank_r = 8, \
                               alpha = 16, learning_rate = 0.001, precision = 'f16', \
                               path = 'child';",
                        )
                        .await?;
                    Ok(())
                })
            })
            .await
            .expect("valid adapter self-reference");
        let rejected_delete = storage
            .with_admin_operation(|database| {
                Box::pin(async move {
                    database.query("DELETE adapter_checkpoint:parent;").await?;
                    Ok(())
                })
            })
            .await;
        assert!(
            rejected_delete.is_err(),
            "REJECT must protect referenced parent"
        );
        let orphan_self_reference = storage
            .with_admin_operation(|database| {
                Box::pin(async move {
                    database
                        .query(
                            "CREATE adapter_checkpoint:orphan SET created_at = 'now', \
                               parent_checkpoint_id = adapter_checkpoint:missing, \
                               base_model_name = 'base', adapter_type = 'lora', rank_r = 8, \
                               alpha = 16, learning_rate = 0.001, precision = 'f16', \
                               path = 'orphan';",
                        )
                        .await?;
                    Ok(())
                })
            })
            .await;
        assert!(
            orphan_self_reference.is_err(),
            "self-reference must target an existing adapter"
        );
        storage.shutdown().await.expect("close store");
    }

    #[tokio::test]
    async fn uuid_backed_record_ids_reject_textual_identity_aliases() {
        const THREAD_UUID: &str = "018f0000-0000-7000-8000-000000000001";
        const MESSAGE_UUID: &str = "018f0000-0000-7000-8000-000000000002";
        const OTHER_UUID: &str = "018f0000-0000-7000-8000-000000000003";

        let directory = tempfile::tempdir().expect("temporary Surreal directory");
        let storage = open_test_storage(&directory)
            .await
            .expect("open fresh store");
        bootstrap_schema(&storage).await.expect("bootstrap schema");

        storage
            .with_admin_operation(|database| {
                Box::pin(async move {
                    let mut response = database
                        .query(format!(
                            "CREATE role_mailbox_thread:u'{THREAD_UUID}' SET \
                               thread_id = u'{THREAD_UUID}', title = 'Typed UUID', \
                               linked_record_kind = 'test', lifecycle_state = 'open', \
                               claim_mode = 'exclusive', takeover_policy = 'reject', \
                               response_authority_scope = 'thread'; \
                             CREATE role_mailbox_message:u'{MESSAGE_UUID}' SET \
                               message_id = u'{MESSAGE_UUID}', \
                               thread_id = role_mailbox_thread:u'{THREAD_UUID}', \
                               message_type = 'request', from_role = 'tester', \
                               delivery_state = 'queued', body = {{ purpose: 'uuid-proof' }}; \
                             RETURN record::id(role_mailbox_thread:u'{THREAD_UUID}');"
                        ))
                        .await?;
                    let observed_id: Option<uuid::Uuid> = response.take(2)?;
                    let observed_id = observed_id.expect("typed UUID record id must exist");
                    assert_eq!(observed_id.to_string(), THREAD_UUID);
                    Ok(())
                })
            })
            .await
            .expect("typed UUID record identity and reference");

        for (label, invalid_query) in [
            (
                "textual reference to UUID-backed target",
                format!(
                    "CREATE role_mailbox_message:u'{OTHER_UUID}' SET \
                       message_id = u'{OTHER_UUID}', \
                       thread_id = role_mailbox_thread:'{THREAD_UUID}', \
                       message_type = 'request', from_role = 'tester', \
                       delivery_state = 'queued', body = {{}};"
                ),
            ),
            (
                "textual record ID with typed UUID alias",
                format!(
                    "CREATE role_mailbox_thread:'{OTHER_UUID}' SET \
                       thread_id = u'{OTHER_UUID}', title = 'Wrong key kind', \
                       linked_record_kind = 'test', lifecycle_state = 'open', \
                       claim_mode = 'exclusive', takeover_policy = 'reject', \
                       response_authority_scope = 'thread';"
                ),
            ),
        ] {
            let result = storage
                .with_admin_operation(|database| {
                    Box::pin(async move {
                        database.query(invalid_query).await?;
                        Ok(())
                    })
                })
                .await;
            assert!(result.is_err(), "{label} must be rejected");
        }
        storage.shutdown().await.expect("close store");
    }
}
