use super::*;

pub(super) const REVISION: i64 = 168;
const BASE_GENERATED: &str = "bbcdb4ecdf26c03d98ccd93533423049ef8b997cf09ad7b94ab2b201b140d0b8";
const PRODUCTION_GENERATED: &str = "2cc8334609a223e94040a2e0a333f6b7d55630deb0b30cd7047c173d15a01ef0";
const PRODUCTION_SOURCE: &str = "5a71ef4196c2710e3589d491789e6c0b13da7d9d0752a0c4a2bb6c575e6ad7df";
const BASE_CATALOG: (&str, usize, usize, usize) =
    ("fab1018a77de6cd9315d79dc72a933fb175761fcc783dd530e033f4a0aa36111", 311, 3987, 859);
const PRODUCTION_CATALOG: (&str, usize, usize, usize) =
    ("f70b0fd8fd7f1e04a52d520d8e8f7567ab8168383b0735207f61fc9a4295934d", 339, 4570, 936);

pub(super) const EVENT_IDENTITY: &str =
    "event:first_owner_administration_origins:wp012_first_owner_workspace_claim_guard";
const EVENT_HEADER: &str =
    "DEFINE EVENT OVERWRITE wp012_first_owner_workspace_claim_guard ON TABLE first_owner_administration_origins";

pub(super) fn receipt_pins(production: bool) -> (&'static str, &'static str, &'static str) {
    if production {
        (PRODUCTION_SOURCE, PRODUCTION_GENERATED, PRODUCTION_CATALOG.0)
    } else {
        (SCHEMA_LINEAGE_SHA256, BASE_GENERATED, BASE_CATALOG.0)
    }
}

pub(super) fn recognizes(
    state: &SchemaState,
    production: bool,
    config: &SurrealStorageConfig,
) -> bool {
    let (source, generated, info) = receipt_pins(production);
    state.version == SCHEMA_VERSION
        && state.revision == REVISION
        && state.target_revision == REVISION
        && state.namespace == config.namespace()
        && state.database == config.database()
        && state.apply_state == "complete"
        && state.source_manifest_sha256 == source
        && state.generated_surql_sha256 == generated
        && state.info_fingerprint_sha256 == info
}

pub(super) fn matches_observed_catalog(observed: &ObservedSchema, production: bool) -> bool {
    let (info, tables, fields, indexes) = if production { PRODUCTION_CATALOG } else { BASE_CATALOG };
    observed.info_fingerprint_sha256 == info
        && observed.tables_defined == tables
        && observed.fields_defined == fields
        && observed.indexes_defined == indexes
}

const UPGRADE_HEADERS: &[&str] = &[
    "DEFINE TABLE OVERWRITE workspaces ",
    "DEFINE EVENT OVERWRITE mt109_workspace_reconciliation_queue ON TABLE workspaces",
    "DEFINE TABLE OVERWRITE protected_resources ",
    "DEFINE FIELD OVERWRITE external_resource_id ON TABLE protected_resources ",
    "DEFINE FIELD OVERWRITE storage_locator_hash ON TABLE protected_resources ",
    "DEFINE FUNCTION OVERWRITE fn::wp012_member_recipient_metadata(",
    "DEFINE FUNCTION OVERWRITE fn::wp012_first_owner_workspace_claim(",
    "DEFINE TABLE OVERWRITE first_owner_administration_origins ",
    EVENT_HEADER,
    "DEFINE FUNCTION OVERWRITE fn::mt120_workspace_create(",
    "DEFINE FUNCTION OVERWRITE fn::mt120_resource_create(",
];

pub(super) fn append_upgrade_statements(mut prior: String) -> String {
    let mut delta = String::new();
    for (start, end) in canonical_upgrade_spans(SCHEMA, UPGRADE_HEADERS) {
        let statement = &SCHEMA[start..end];
        prior = prior.replace(statement, "");
        delta.push_str(statement);
        delta.push('\n');
    }
    prior.push('\n');
    prior.push_str(&delta);
    prior
}

pub(super) fn restore_shared_schema(mut source: String) -> String {
    if !source.contains(EVENT_HEADER) {
        return source;
    }
    for &(current, previous, count) in INVERSE {
        assert_eq!(source.matches(current).count(), count, "revision 168 source delta drifted");
        source = source.replace(current, previous);
    }
    source
}

const INVERSE: &[(&str, &str, usize)] = &[
    (r##"DEFINE FUNCTION OVERWRITE fn::wp012_member_recipient_metadata($kind: string, $id: record) {
    IF $access NOT IN ['member_enrollment_recipient_inspection', 'member_enrollment_recipient_installer', 'member_enrollment_recipient'] { RETURN false; };
    IF !fn::wp012_member_recipient_actor"##, r##"DEFINE FUNCTION OVERWRITE fn::wp012_member_recipient_metadata($kind: string, $id: record) {
    IF !fn::wp012_member_recipient_actor"##, 1),
    (r##"fn::mt120_workspace_create($row.created_in_session_id, $original_workspace, $original_resource)"##, r##"fn::mt120_workspace_create($row.created_in_session_id)"##, 1),
    (r##"DEFINE FUNCTION OVERWRITE fn::mt120_resource_create(
    $row: object, $original_workspace: option<string>, $original_resource: option<string>
) {"##, r##"DEFINE FUNCTION OVERWRITE fn::mt120_resource_create($row: object) {"##, 1),
    (r##"    IF $original_workspace = NONE OR $original_resource = NONE { RETURN false; };
    RETURN $origin.first_workspace_operation_id = $original_workspace
        AND $origin.first_workspace = type::record('workspaces', $original_workspace)
        AND $origin.first_workspace_resource = type::record('protected_resources', $original_resource)"##, r##"    RETURN $origin.first_workspace_operation_id = $wp012_original_workspace
        AND $origin.first_workspace = type::record('workspaces', $wp012_original_workspace)
        AND $origin.first_workspace_resource = type::record('protected_resources', $wp012_original_resource)"##, 1),
    (r##"DEFINE FUNCTION OVERWRITE fn::mt120_workspace_create(
    $session: option<record<authenticated_sessions>>,
    $original_workspace: option<string>, $original_resource: option<string>
) {"##, r##"DEFINE FUNCTION OVERWRITE fn::mt120_workspace_create($session: option<record<authenticated_sessions>>) {"##, 1),
    (r##"fn::mt120_resource_create($this, $wp012_original_workspace, $wp012_original_resource)"##, r##"fn::mt120_resource_create($this)"##, 3),
    (r##"fn::mt120_workspace_create($after.created_in_session_id, $wp012_original_workspace, $wp012_original_resource)"##, r##"fn::mt120_workspace_create($after.created_in_session_id)"##, 1),
    (r##"fn::mt120_workspace_create(created_in_session_id, $wp012_original_workspace, $wp012_original_resource)"##, r##"fn::mt120_workspace_create(created_in_session_id)"##, 1),
    (r##"    FOR update WHERE id = first_owner_administration_origins:primary AND fn::wp012_first_owner_workspace_actor($this);
DEFINE EVENT OVERWRITE wp012_first_owner_workspace_claim_guard ON TABLE first_owner_administration_origins
    WHEN $event = 'UPDATE' AND $auth != NONE
    THEN {
        IF !fn::wp012_first_owner_workspace_claim(
            $before, $after, {
                account_session: $account_session, account: $account, principal: $principal, space: $space,
                workspace: $workspace, external: $external, name: $name, actor: $actor, edit: $edit,
                resource: $resource, fr: $fr, workspace_grant: $workspace_grant, fr_grant: $fr_grant,
                capabilities: $capabilities, memory_surfaces: $memory_surfaces,
                operation_id: $operation_id, channel_hash: $channel_hash, deadline: $deadline,
                first_owner_workspace_receipt_read: false
            }, $wp012_original_workspace, $wp012_original_resource
        ) { THROW 'HSK-403-PROTECTED-RESOURCE'; };
    };
DEFINE FUNCTION OVERWRITE fn::wp012_first_owner_workspace_receipt("##, r##"    FOR update WHERE fn::wp012_first_owner_workspace_claim($before, $after);
DEFINE FUNCTION OVERWRITE fn::wp012_first_owner_workspace_receipt("##, 1),
    (r##"        AND $after.first_workspace_operation_id = $original_workspace
        AND $after.first_workspace = type::record('workspaces', $original_workspace)
        AND $after.first_workspace_resource = type::record('protected_resources', $original_resource)"##, r##"        AND $after.first_workspace_operation_id = $wp012_original_workspace
        AND $after.first_workspace = type::record('workspaces', $wp012_original_workspace)
        AND $after.first_workspace_resource = type::record('protected_resources', $wp012_original_resource)"##, 1),
    (r##"        AND $after.first_workspace_original = $original"##, r##"        AND $after.first_workspace_original != NONE
        AND $after.first_workspace_original = {
            account_session: $account_session, account: $account, principal: $principal, space: $space,
            workspace: $workspace, external: $external, name: $name, actor: $actor, edit: $edit,
            resource: $resource, fr: $fr, workspace_grant: $workspace_grant, fr_grant: $fr_grant,
            capabilities: $capabilities, memory_surfaces: $memory_surfaces,
            operation_id: $operation_id, channel_hash: $channel_hash, deadline: $deadline,
            first_owner_workspace_receipt_read: false
        }"##, 1),
    (r##"DEFINE FUNCTION OVERWRITE fn::wp012_first_owner_workspace_claim(
    $before: object, $after: object, $original: option<object>,
    $original_workspace: option<string>, $original_resource: option<string>
) {
    IF $original = NONE OR $original_workspace = NONE OR $original_resource = NONE { RETURN false; };
    RETURN fn::wp012_first_owner_workspace_actor($before)"##, r##"DEFINE FUNCTION OVERWRITE fn::wp012_first_owner_workspace_claim($before: object, $after: object) {
    RETURN fn::wp012_first_owner_workspace_actor($before)"##, 1),
];
