use super::*;

pub(super) const REVISION: i64 = 171;
const BASE_GENERATED: &str = "ec2a2552e0abd613ef53f814b07e65673b167f58afb0f04ade1f50cb0885d651";
const PRODUCTION_GENERATED: &str = "a1af8b9349d6942e3d8b027382a7815fa166effb192f88fd37b6fe9708b68e5e";
const PRODUCTION_SOURCE: &str = "f8f592f0a32d63cad7732a3e2b6b6fb1af8bfc554d7a38de453bb1e8ba546458";
const BASE_CATALOG: (&str, usize, usize, usize) =
    ("bc7689225f3aa12a34032293dcd0811bfa92230d71b4f2cebd06f5c075bf7118", 311, 3987, 859);
const PRODUCTION_CATALOG: (&str, usize, usize, usize) =
    ("ad68e0eb25d5a8e3c4b97c9cc48c100823239d73881d29cf702616c18862e41f", 339, 4570, 936);

pub(super) fn receipt_pins(production: bool) -> (&'static str, &'static str, &'static str) {
    if production {
        (PRODUCTION_SOURCE, PRODUCTION_GENERATED, PRODUCTION_CATALOG.0)
    } else {
        (SCHEMA_LINEAGE_SHA256, BASE_GENERATED, BASE_CATALOG.0)
    }
}

pub(super) fn recognizes(state: &SchemaState, production: bool, config: &SurrealStorageConfig) -> bool {
    let (source, generated, info) = receipt_pins(production);
    state.version == SCHEMA_VERSION
        && state.revision == REVISION
        && state.target_revision == REVISION
        && state.namespace == config.namespace()
        && state.database == config.database()
        && state.source_manifest_sha256 == source
        && state.generated_surql_sha256 == generated
        && match state.apply_state.as_str() {
            "complete" => state.info_fingerprint_sha256 == info,
            "schema_applied" => state.info_fingerprint_sha256 == PENDING_SCHEMA_INFO_SHA256,
            _ => false,
        }
}

pub(super) fn matches_observed_catalog(observed: &ObservedSchema, production: bool) -> bool {
    let (info, tables, fields, indexes) = if production { PRODUCTION_CATALOG } else { BASE_CATALOG };
    observed.info_fingerprint_sha256 == info
        && observed.tables_defined == tables
        && observed.fields_defined == fields
        && observed.indexes_defined == indexes
}

const SHARED_BLOCK_BEGIN: &str = "-- WP012_REVISION_172_BEGIN\n";
const SHARED_BLOCK_END: &str = "-- WP012_REVISION_172_END\n\n";
const MODEL_LANE_RECEIPT_HEADER: &str = "DEFINE FUNCTION OVERWRITE fn::wp012_model_lane_receipt(";
const MODEL_LANE_SOURCE: &str = "model_lane";
const KNOWLEDGE_ENTITY_IDENTITY_INDEX: &str =
    "DEFINE INDEX OVERWRITE uq_knowledge_entities_identity ON TABLE knowledge_entities ";
const KNOWLEDGE_ENTITY_DUPLICATE_PREFLIGHT: &str = "\
LET $dups = (SELECT workspace_id, entity_kind, entity_key, count() AS c FROM knowledge_entities GROUP BY workspace_id, entity_kind, entity_key);\n\
IF array::len(array::filter($dups, |$d| $d.c > 1)) > 0 { THROW 'HSK-SCHEMA-172-KNOWLEDGE-ENTITY-DUPLICATE'; };\n";

enum Inverse {
    Previous(&'static str),
    Fragments(&'static [(&'static str, &'static str)]),
    Added,
}

const PRODUCTION_DECLARATIONS: &[(&str, &str, &[(&str, Inverse)])] = &[
    ("loom_search", include_str!("loom_search_schema.surql"), LOOM_SEARCH_DECLARATIONS),
    ("user_manual_knowledge", include_str!("user_manual_knowledge_schema.surql"), USER_MANUAL_DECLARATIONS),
    (MODEL_LANE_SOURCE, include_str!("model_lane_schema.surql"), MODEL_LANE_DECLARATIONS),
    ("cloud_model_lane", include_str!("cloud_model_lane_schema.surql"), CLOUD_MODEL_LANE_DECLARATIONS),
];

fn shared_block_span() -> (usize, usize) {
    let start = unique_offset(SCHEMA, SHARED_BLOCK_BEGIN).expect("revision 172 shared block begins");
    let end = unique_offset(SCHEMA, SHARED_BLOCK_END).expect("revision 172 shared block ends")
        + SHARED_BLOCK_END.len();
    (start, end)
}

pub(super) enum PriorUpgrade {
    None,
    ModelLane,
    MemberEnrollmentAndModelLane,
}

pub(super) fn upgrade_statements(production: bool) -> String {
    upgrade_statements_after(production, PriorUpgrade::None)
}

pub(super) fn upgrade_statements_after(production: bool, prior: PriorUpgrade) -> String {
    let shared_headers: Vec<_> = SHARED_DECLARATIONS
        .iter()
        .filter_map(|(header, _)| match prior {
            PriorUpgrade::None => Some(*header),
            PriorUpgrade::ModelLane if *header != MODEL_LANE_RECEIPT_HEADER => Some(*header),
            _ => None,
        })
        .collect();
    let mut shared_spans = canonical_upgrade_spans(SCHEMA, &shared_headers);
    let (block_start, block_end) = shared_block_span();
    assert!(
        shared_spans.range(block_start..block_end).next().is_none(),
        "revision 172 shared declaration is already in the marked block",
    );
    assert!(shared_spans.insert(block_start, block_end).is_none());

    let mut statements = String::new();
    for (start, end) in shared_spans {
        statements.push_str(&SCHEMA[start..end]);
        statements.push('\n');
    }
    if production {
        for &(name, source, declarations) in PRODUCTION_DECLARATIONS {
            if name == MODEL_LANE_SOURCE && !matches!(prior, PriorUpgrade::None) {
                continue;
            }
            for (start, end) in declaration_spans(source, declarations) {
                let statement = &source[start..end];
                if statement.starts_with(KNOWLEDGE_ENTITY_IDENTITY_INDEX) {
                    statements.push_str(KNOWLEDGE_ENTITY_DUPLICATE_PREFLIGHT);
                }
                statements.push_str(statement);
                statements.push('\n');
            }
        }
    }
    statements
}

fn declaration_spans(source: &str, declarations: &[(&str, Inverse)]) -> BTreeMap<usize, usize> {
    let headers: Vec<_> = declarations.iter().map(|(header, _)| *header).collect();
    canonical_upgrade_spans(source, &headers)
}

pub(super) fn restore_shared_schema(mut source: String) -> String {
    let (start, end) = shared_block_span();
    replace_once(&mut source, &SCHEMA[start..end], "");
    let permit_fields_start = unique_offset(
        SCHEMA,
        "DEFINE FIELD OVERWRITE original ON TABLE member_enrollment_redemption_permits ",
    ).expect("revision 172 permit fields begin");
    let permit_fields_end = unique_offset(
        SCHEMA,
        "DEFINE FUNCTION OVERWRITE fn::wp012_member_revoke_read(",
    ).expect("revision 172 permit fields end");
    let permit_fields = &SCHEMA[permit_fields_start..permit_fields_end];
    if let Some(offset) = unique_offset(&source, permit_fields) {
        source.replace_range(offset..offset + permit_fields.len(), "");
        let previous_offset = unique_offset(
            &source,
            "DEFINE FIELD OVERWRITE phc ON TABLE member_enrollment_redemption_permits ",
        ).expect("revision 171 permit field position");
        source.insert_str(previous_offset, permit_fields);
    }
    restore_declarations(&mut source, SCHEMA, SHARED_DECLARATIONS);
    source
}

pub(super) fn restore_production_sources(sources: &mut [(&str, String)]) {
    for &(name, canonical, declarations) in PRODUCTION_DECLARATIONS {
        let mut matching = sources.iter_mut().filter(|(module, _)| *module == name);
        let (_, source) = matching.next().expect("revision 171 production source is present");
        assert!(matching.next().is_none(), "revision 171 production source duplicated: {name}");
        restore_declarations(source, canonical, declarations);
    }
}

fn restore_declarations(source: &mut String, canonical: &str, declarations: &[(&str, Inverse)]) {
    for (header, inverse) in declarations {
        let spans = canonical_upgrade_spans(canonical, &[*header]);
        let (&start, &end) = spans.first_key_value().expect("revision 172 declaration is present");
        let revised = &canonical[start..end];
        match inverse {
            Inverse::Previous(previous) => replace_once(source, revised, previous),
            Inverse::Fragments(fragments) => {
                let mut previous = revised.to_owned();
                for &(from, to) in *fragments {
                    assert!(unique_offset(&previous, from).is_some(),
                        "revision 172 fragment missing from declaration: {header}");
                    replace_once(&mut previous, from, to);
                }
                replace_once(source, revised, &previous);
            }
            Inverse::Added => {
                assert!(canonical[end..].starts_with("\n\n"),
                    "revision 172 inserted declaration delimiter changed: {header}");
                replace_once(source, &canonical[start..end + 2], "");
            }
        }
    }
}

fn replace_once(source: &mut String, current: &str, previous: &str) {
    if let Some(offset) = unique_offset(source, current) {
        source.replace_range(offset..offset + current.len(), previous);
    }
}

fn unique_offset(source: &str, fragment: &str) -> Option<usize> {
    let mut matches = source.match_indices(fragment);
    let offset = matches.next().map(|(offset, _)| offset);
    assert!(matches.next().is_none(), "revision 172 source span duplicated: {fragment}");
    offset
}

const SHARED_DECLARATIONS: &[(&str, Inverse)] = &[
    (
        "DEFINE FIELD OVERWRITE original ON TABLE member_enrollment_redemption_permits ",
        Inverse::Previous("DEFINE FIELD OVERWRITE original ON TABLE member_enrollment_redemption_permits TYPE object;"),
    ),
    (
        "DEFINE FIELD OVERWRITE account_before ON TABLE member_enrollment_redemption_permits ",
        Inverse::Previous("DEFINE FIELD OVERWRITE account_before ON TABLE member_enrollment_redemption_permits TYPE object;"),
    ),
    (
        "DEFINE FIELD OVERWRITE principal_before ON TABLE member_enrollment_redemption_permits ",
        Inverse::Previous("DEFINE FIELD OVERWRITE principal_before ON TABLE member_enrollment_redemption_permits TYPE object;"),
    ),
    (
        "DEFINE FIELD OVERWRITE space_before ON TABLE member_enrollment_redemption_permits ",
        Inverse::Previous("DEFINE FIELD OVERWRITE space_before ON TABLE member_enrollment_redemption_permits TYPE object;"),
    ),
    (
        "DEFINE FUNCTION OVERWRITE fn::wp012_first_owner_workspace_receipt(",
        Inverse::Fragments(&[
            (r#"        AND $workspace.last_actor_kind = 'HUMAN'
        AND $row.setup_receipt.event_type = 'LOCAL_ACCOUNT_SETUP'
        AND $row.setup_receipt.event_sequence > 0 AND $row.setup_receipt.created_at = $row.created_at
"#, r#"        AND $workspace.last_actor_kind = 'HUMAN'
        AND record::exists($row.setup_receipt) AND $row.setup_receipt.event_type = 'LOCAL_ACCOUNT_SETUP'
        AND $row.setup_receipt.event_sequence > 0 AND $row.setup_receipt.created_at = $row.created_at
"#),
        ]),
    ),
    (
        MODEL_LANE_RECEIPT_HEADER,
        Inverse::Fragments(&[
            (r#"DEFINE FUNCTION OVERWRITE fn::wp012_model_lane_receipt($row: object, $write: bool) {
    IF $row.aggregate_type = 'session_cleanup_receipt' AND $wp012_original_purpose = 'cloud_consent_revocation' {
        IF $wp012_recovery_head_only = true OR $wp012_recovery_discovery_only = true
            OR $wp012_original_subject_kind != 'workspace' OR $wp012_original_action != 'update'
            OR $wp012_original_capability != 'fs.write' OR $wp012_original_subject_id != $row.workspace_id
            OR $record_kind != $row.aggregate_type OR $aggregate_id != $row.aggregate_id
            OR !fn::wp012_cloud_revocation_cleanup_event($row, $row, $row.kernel_task_run_id, $row.aggregate_id) { RETURN false; };
        LET $receipt = encoding::json::decode($row.payload.event_payload_json);
        IF !type::is_object($receipt)
            OR !fn::wp012_cloud_revocation_cleanup_witness($row, $row.kernel_task_run_id, $row.aggregate_id, $receipt, $row.event_sequence) { RETURN false; };
        LET $version = $row.payload.event_stream_version;
        IF !$write { RETURN $version != 1 OR $receipt.status = 'cleanup_pending'; };

        LET $key = crypto::sha256(string::join(['handshake.model-lane.authority.v1', 'session_cleanup_receipt', $row.aggregate_id,
            $row.owner_account_id, $row.actor_principal_id, $row.authenticated_session_id, $row.access_space_id, $row.workspace_id], '\u0000'));
        IF $record_id != $key OR $event_id != $row.event_id
            OR $record_json != $row.payload.event_payload_json OR $event_payload_json != $record_json { RETURN false; };
        LET $current = SELECT * FROM ONLY type::record('model_lane_authority', $key);
        IF $row.event_type = 'MODEL_LANE_AUTHORITY_RECORDED' {
            RETURN $current = NONE AND $version = 1 AND $run_id = $row.kernel_task_run_id AND $receipt.status = 'cleanup_pending';
        };
        IF $row.event_type != 'MODEL_LANE_AUTHORITY_REPLACED' OR $current = NONE
            OR !type::is_int($expected_event_stream_version) OR $expected_event_stream_version < 1
            OR $current.event_stream_version != $expected_event_stream_version OR $version != $expected_event_stream_version + 1
            OR $current.record_kind != $row.aggregate_type OR $current.aggregate_id != $row.aggregate_id OR $current.run_id != $row.kernel_task_run_id
            OR $current.owner_account_id != $row.owner_account_id OR $current.actor_principal_id != $row.actor_principal_id
            OR $current.authenticated_session_id != $row.authenticated_session_id
            OR $current.access_space_id != $row.access_space_id OR $current.workspace_id != $row.workspace_id
            OR $current.event_id != $row.causation_id OR $current.event_ledger_event_id != type::record('kernel_event_ledger', $row.causation_id)
            OR $current.event_seq >= $row.event_sequence
            OR !fn::wp012_cloud_revocation_cleanup_projection($current, 'read') { RETURN false; };
        RETURN fn::wp012_cloud_revocation_cleanup_transition(encoding::json::decode($current.record_json), $receipt);
    };
    IF !$write AND $wp012_recovery_head_only = true {
"#, r#"DEFINE FUNCTION OVERWRITE fn::wp012_model_lane_receipt($row: object, $write: bool) {
    IF !$write AND $wp012_recovery_head_only = true {
"#),
        ]),
    ),
];

const LOOM_SEARCH_DECLARATIONS: &[(&str, Inverse)] = &[
    (
        "DEFINE FIELD OVERWRITE owner_account_id ON TABLE loom_block_search_index ",
        Inverse::Previous("DEFINE FIELD OVERWRITE owner_account_id ON TABLE loom_block_search_index TYPE string ASSERT string::trim($value) != '';"),
    ),
    (
        "DEFINE FIELD OVERWRITE actor_principal_id ON TABLE loom_block_search_index ",
        Inverse::Previous("DEFINE FIELD OVERWRITE actor_principal_id ON TABLE loom_block_search_index TYPE string ASSERT string::trim($value) != '';"),
    ),
    (
        "DEFINE FIELD OVERWRITE authenticated_session_id ON TABLE loom_block_search_index ",
        Inverse::Previous("DEFINE FIELD OVERWRITE authenticated_session_id ON TABLE loom_block_search_index TYPE string ASSERT string::trim($value) != '';"),
    ),
    (
        "DEFINE FIELD OVERWRITE access_space_id ON TABLE loom_block_search_index ",
        Inverse::Previous("DEFINE FIELD OVERWRITE access_space_id ON TABLE loom_block_search_index TYPE string ASSERT string::trim($value) != '';"),
    ),
    (
        "DEFINE FIELD OVERWRITE source_content_hash ON TABLE loom_block_search_index ",
        Inverse::Previous("DEFINE FIELD OVERWRITE source_content_hash ON TABLE loom_block_search_index TYPE string ASSERT string::len($value) = 64;"),
    ),
    (
        "DEFINE FIELD OVERWRITE mutation_fingerprint ON TABLE loom_block_search_index ",
        Inverse::Previous("DEFINE FIELD OVERWRITE mutation_fingerprint ON TABLE loom_block_search_index TYPE string ASSERT string::len($value) = 64;"),
    ),
    (
        "DEFINE FIELD OVERWRITE index_event_id ON TABLE loom_block_search_index ",
        Inverse::Previous("DEFINE FIELD OVERWRITE index_event_id ON TABLE loom_block_search_index TYPE record<kernel_event_ledger> ASSERT record::exists($value) REFERENCE ON DELETE REJECT;"),
    ),
];

const USER_MANUAL_DECLARATIONS: &[(&str, Inverse)] = &[
    (
        "DEFINE FIELD OVERWRITE owner_account_id ON TABLE knowledge_entities ",
        Inverse::Previous("DEFINE FIELD OVERWRITE owner_account_id ON TABLE knowledge_entities TYPE string ASSERT string::trim($value) != '';"),
    ),
    (
        "DEFINE FIELD OVERWRITE actor_principal_id ON TABLE knowledge_entities ",
        Inverse::Previous("DEFINE FIELD OVERWRITE actor_principal_id ON TABLE knowledge_entities TYPE string ASSERT string::trim($value) != '';"),
    ),
    (
        "DEFINE FIELD OVERWRITE authenticated_session_id ON TABLE knowledge_entities ",
        Inverse::Previous("DEFINE FIELD OVERWRITE authenticated_session_id ON TABLE knowledge_entities TYPE string ASSERT string::trim($value) != '';"),
    ),
    (
        "DEFINE FIELD OVERWRITE access_space_id ON TABLE knowledge_entities ",
        Inverse::Previous("DEFINE FIELD OVERWRITE access_space_id ON TABLE knowledge_entities TYPE string ASSERT string::trim($value) != '';"),
    ),
    (
        "DEFINE FIELD OVERWRITE recorded_event_id ON TABLE knowledge_entities ",
        Inverse::Previous("DEFINE FIELD OVERWRITE recorded_event_id ON TABLE knowledge_entities TYPE record<kernel_event_ledger> ASSERT record::exists($value) REFERENCE ON DELETE REJECT;"),
    ),
    (
        "DEFINE FIELD OVERWRITE mutation_fingerprint ON TABLE knowledge_entities ",
        Inverse::Previous("DEFINE FIELD OVERWRITE mutation_fingerprint ON TABLE knowledge_entities TYPE string ASSERT string::len($value) = 64;"),
    ),
    (
        KNOWLEDGE_ENTITY_IDENTITY_INDEX,
        Inverse::Previous("DEFINE INDEX OVERWRITE uq_knowledge_entities_identity ON TABLE knowledge_entities FIELDS owner_account_id, actor_principal_id, authenticated_session_id, access_space_id, workspace_id, entity_kind, entity_key UNIQUE;"),
    ),
];

const MODEL_LANE_DECLARATIONS: &[(&str, Inverse)] = &[
    (
        "DEFINE TABLE OVERWRITE model_lane_authority ",
        Inverse::Fragments(&[
            (r#"        fn::wp012_validator_association_record($this, false)
    } ELSE IF record_kind = 'session_cleanup_receipt' AND $wp012_original_purpose = 'cloud_consent_revocation' {
        fn::wp012_cloud_revocation_cleanup_projection($this, 'read')
    } ELSE IF $wp012_role_evidence_capture != NONE {
"#, r#"        fn::wp012_validator_association_record($this, false)
    } ELSE IF $wp012_role_evidence_capture != NONE {
"#),
            (r#"        fn::wp012_validator_association_record($this, true)
    } ELSE IF record_kind = 'session_cleanup_receipt' AND $wp012_original_purpose = 'cloud_consent_revocation' {
        fn::wp012_cloud_revocation_cleanup_projection($this, 'create')
    } ELSE IF record_kind IN ['role_evidence', 'validator_designation', 'role_evidence_consumption'] {
"#, r#"        fn::wp012_validator_association_record($this, true)
    } ELSE IF record_kind IN ['role_evidence', 'validator_designation', 'role_evidence_consumption'] {
"#),
            (r#"    FOR update WHERE record_kind = 'run' AND fn::wp012_run_attachment_authority($this, true)
        OR (record_kind = 'session_cleanup_receipt' AND $wp012_original_purpose = 'cloud_consent_revocation'
            AND fn::wp012_cloud_revocation_cleanup_projection($this, 'update'))
    FOR delete NONE;"#, r#"    FOR update WHERE record_kind = 'run' AND fn::wp012_run_attachment_authority($this, true)
    FOR delete NONE;"#),
        ]),
    ),
    (
        "DEFINE FIELD OVERWRITE last_transition_event_id ON knowledge_crdt_ai_edit_proposals ",
        Inverse::Previous("DEFINE FIELD IF NOT EXISTS last_transition_event_id ON knowledge_crdt_ai_edit_proposals TYPE record<kernel_event_ledger>;"),
    ),
    (
        "DEFINE FIELD OVERWRITE owner_account_id ON kernel_crdt_updates ",
        Inverse::Previous("DEFINE FIELD IF NOT EXISTS owner_account_id ON kernel_crdt_updates TYPE string;"),
    ),
    (
        "DEFINE FIELD OVERWRITE actor_principal_id ON kernel_crdt_updates ",
        Inverse::Previous("DEFINE FIELD IF NOT EXISTS actor_principal_id ON kernel_crdt_updates TYPE string;"),
    ),
    (
        "DEFINE FIELD OVERWRITE authenticated_session_id ON kernel_crdt_updates ",
        Inverse::Previous("DEFINE FIELD IF NOT EXISTS authenticated_session_id ON kernel_crdt_updates TYPE string;"),
    ),
    (
        "DEFINE FIELD OVERWRITE access_space_id ON kernel_crdt_updates ",
        Inverse::Previous("DEFINE FIELD IF NOT EXISTS access_space_id ON kernel_crdt_updates TYPE string;"),
    ),
    (
        "DEFINE FIELD OVERWRITE owner_account_id ON kernel_crdt_snapshots ",
        Inverse::Previous("DEFINE FIELD IF NOT EXISTS owner_account_id ON kernel_crdt_snapshots TYPE string;"),
    ),
    (
        "DEFINE FIELD OVERWRITE actor_principal_id ON kernel_crdt_snapshots ",
        Inverse::Previous("DEFINE FIELD IF NOT EXISTS actor_principal_id ON kernel_crdt_snapshots TYPE string;"),
    ),
    (
        "DEFINE FIELD OVERWRITE authenticated_session_id ON kernel_crdt_snapshots ",
        Inverse::Previous("DEFINE FIELD IF NOT EXISTS authenticated_session_id ON kernel_crdt_snapshots TYPE string;"),
    ),
    (
        "DEFINE FIELD OVERWRITE access_space_id ON kernel_crdt_snapshots ",
        Inverse::Previous("DEFINE FIELD IF NOT EXISTS access_space_id ON kernel_crdt_snapshots TYPE string;"),
    ),
    (
        "DEFINE FIELD OVERWRITE owner_account_id ON knowledge_crdt_agent_lane_leases ",
        Inverse::Previous("DEFINE FIELD IF NOT EXISTS owner_account_id ON knowledge_crdt_agent_lane_leases TYPE string;"),
    ),
    (
        "DEFINE FIELD OVERWRITE actor_principal_id ON knowledge_crdt_agent_lane_leases ",
        Inverse::Previous("DEFINE FIELD IF NOT EXISTS actor_principal_id ON knowledge_crdt_agent_lane_leases TYPE string;"),
    ),
    (
        "DEFINE FIELD OVERWRITE authenticated_session_id ON knowledge_crdt_agent_lane_leases ",
        Inverse::Previous("DEFINE FIELD IF NOT EXISTS authenticated_session_id ON knowledge_crdt_agent_lane_leases TYPE string;"),
    ),
    (
        "DEFINE FIELD OVERWRITE access_space_id ON knowledge_crdt_agent_lane_leases ",
        Inverse::Previous("DEFINE FIELD IF NOT EXISTS access_space_id ON knowledge_crdt_agent_lane_leases TYPE string;"),
    ),
    (
        "DEFINE FIELD OVERWRITE workspace_id ON knowledge_crdt_agent_lane_leases ",
        Inverse::Previous("DEFINE FIELD IF NOT EXISTS workspace_id ON knowledge_crdt_agent_lane_leases TYPE string;"),
    ),
    (
        "DEFINE FIELD OVERWRITE recorded_event_id ON knowledge_crdt_agent_lane_leases ",
        Inverse::Previous("DEFINE FIELD IF NOT EXISTS recorded_event_id ON knowledge_crdt_agent_lane_leases TYPE record<kernel_event_ledger>;"),
    ),
    (
        "DEFINE FIELD OVERWRITE last_transition_event_id ON knowledge_crdt_agent_lane_leases ",
        Inverse::Previous("DEFINE FIELD IF NOT EXISTS last_transition_event_id ON knowledge_crdt_agent_lane_leases TYPE record<kernel_event_ledger>;"),
    ),
    (
        "DEFINE FIELD OVERWRITE owner_account_id ON knowledge_crdt_ai_edit_proposals ",
        Inverse::Previous("DEFINE FIELD IF NOT EXISTS owner_account_id ON knowledge_crdt_ai_edit_proposals TYPE string;"),
    ),
    (
        "DEFINE FIELD OVERWRITE actor_principal_id ON knowledge_crdt_ai_edit_proposals ",
        Inverse::Previous("DEFINE FIELD IF NOT EXISTS actor_principal_id ON knowledge_crdt_ai_edit_proposals TYPE string;"),
    ),
    (
        "DEFINE FIELD OVERWRITE authenticated_session_id ON knowledge_crdt_ai_edit_proposals ",
        Inverse::Previous("DEFINE FIELD IF NOT EXISTS authenticated_session_id ON knowledge_crdt_ai_edit_proposals TYPE string;"),
    ),
    (
        "DEFINE FIELD OVERWRITE access_space_id ON knowledge_crdt_ai_edit_proposals ",
        Inverse::Previous("DEFINE FIELD IF NOT EXISTS access_space_id ON knowledge_crdt_ai_edit_proposals TYPE string;"),
    ),
    (
        "DEFINE FUNCTION OVERWRITE fn::wp012_model_lane_operation_matrix(",
        Inverse::Fragments(&[
            (r#"        OR ($kind = 'workspace' AND $purpose = 'recovery_execute' AND $action = 'update')
        OR ($kind = 'workspace' AND $purpose = 'cloud_consent_revocation' AND $action = 'update')
        OR ($kind = 'workspace' AND $purpose IN ['routing_create', 'run_attachment'] AND $action = 'create')
"#, r#"        OR ($kind = 'workspace' AND $purpose = 'recovery_execute' AND $action = 'update')
        OR ($kind = 'workspace' AND $purpose IN ['routing_create', 'run_attachment'] AND $action = 'create')
"#),
        ]),
    ),
    (
        "DEFINE FIELD OVERWRITE authority_purpose ON model_lane_context_coordination ",
        Inverse::Previous("DEFINE FIELD OVERWRITE authority_purpose ON model_lane_context_coordination TYPE 'consume' | 'recovery_execute' | 'artifact_binding' | 'handoff' | 'guarded_message' | 'document_claim' | 'workspace_claim' | 'index_run_claim' | 'lease_renew' | 'lease_release' | 'lease_expiry' | 'crdt_update' | 'crdt_snapshot' | 'proposal_record' | 'proposal_decide' | 'proposal_apply' | 'cloud_projection_plan' | 'cloud_consent_receipt' | 'cloud_run' | 'cloud_lane' | 'cloud_consent_denial' | 'recovery_checkpoint_record';"),
    ),
    ("DEFINE FUNCTION OVERWRITE fn::wp012_cloud_revocation_cleanup_witness(", Inverse::Added),
    ("DEFINE FUNCTION OVERWRITE fn::wp012_cloud_revocation_cleanup_event(", Inverse::Added),
    ("DEFINE FUNCTION OVERWRITE fn::wp012_cloud_revocation_cleanup_transition(", Inverse::Added),
    ("DEFINE FUNCTION OVERWRITE fn::wp012_cloud_revocation_cleanup_projection(", Inverse::Added),
    (
        "DEFINE FIELD OVERWRITE lease_id ON knowledge_crdt_agent_lane_leases ",
        Inverse::Previous("DEFINE FIELD OVERWRITE lease_id ON knowledge_crdt_agent_lane_leases TYPE string ASSERT IF $this.owner_account_id = NONE AND $this.actor_principal_id = NONE AND $this.authenticated_session_id = NONE AND $this.access_space_id = NONE AND $this.workspace_id = NONE { (record::id($this.id) = $value) } ELSE { $this.owner_account_id != NONE AND $this.actor_principal_id != NONE AND $this.authenticated_session_id != NONE AND $this.access_space_id != NONE AND $this.workspace_id != NONE AND record::id($this.id) = crypto::sha256(string::join(['handshake.model-lane.authority.v1', 'crdt_lease', $value, $this.owner_account_id, $this.actor_principal_id, $this.authenticated_session_id, $this.access_space_id, $this.workspace_id], '\\u0000')) };"),
    ),
    (
        "DEFINE EVENT OVERWRITE wp012_model_lane_proposal_transition ON knowledge_crdt_ai_edit_proposals ",
        Inverse::Previous(r#"DEFINE EVENT OVERWRITE wp012_model_lane_proposal_transition ON knowledge_crdt_ai_edit_proposals WHEN $event = 'UPDATE' THEN {
    IF !fn::wp012_model_lane_proposal_access($before, true) OR !fn::wp012_model_lane_proposal_access($after, true)
        OR !fn::wp012_model_lane_proposal_identity($before) OR !fn::wp012_model_lane_proposal_identity($after) {
        THROW 'HSK-403-MODEL-LANE-PROPOSAL';
    };
    LET $promotion = $wp012_original_purpose = 'proposal_decide' AND $before.review_state = 'approved' AND $after.review_state = 'promoted';
    LET $mutable = IF $promotion {
        ['review_state', 'promotion_requested_event_id', 'promotion_accepted_event_id', 'last_transition_event_id']
    } ELSE IF $wp012_original_purpose = 'proposal_decide' {
        ['review_state', 'decided_by', 'decided_at_utc', 'decision_reason', 'decided_event_id', 'last_transition_event_id']
    } ELSE { ['applied_update_id', 'applied_update_sha256', 'applied_event_id', 'last_transition_event_id'] };
    LET $original = object::from_entries(array::filter(object::entries($before), |$entry| $entry[0] NOT IN $mutable));
    LET $next = object::from_entries(array::filter(object::entries($after), |$entry| $entry[0] NOT IN $mutable));
    IF $original != $next { THROW 'HSK-403-MODEL-LANE-PROPOSAL-IMMUTABLE'; };
    IF $promotion {
        LET $head = SELECT * FROM kernel_crdt_updates WHERE document_id = $before.document_id
            AND crdt_document_id = $before.crdt_document_id AND owner_account_id = $before.owner_account_id
            AND actor_principal_id = $before.actor_principal_id AND authenticated_session_id = $before.authenticated_session_id
            AND access_space_id = $before.access_space_id AND workspace_id = $before.workspace_id ORDER BY update_seq DESC LIMIT 1;
        IF $before.applied_event_id = NONE OR $before.promotion_requested_event_id != NONE OR $before.promotion_accepted_event_id != NONE
            OR $auth.principal_id.principal_kind NOT IN ['human_account', 'break_glass_operator']
            OR $auth.principal_id.status != 'enabled' OR $auth.principal_id.actor_kind != 'operator'
            OR $after.promotion_requested_event_id.actor_id != $auth.principal_id.actor_id
            OR $after.promotion_accepted_event_id.authority_session_id != $auth.id
            OR $after.promotion_accepted_event_id.authority_resource_id != type::record('protected_resources', $wp012_original_resource)
            OR $after.promotion_accepted_event_id.authority_grant_id != type::record('resource_grants', $wp012_original_grant)
            OR array::len($head) != 1 OR $head[0].update_id != $before.applied_update_id
            OR $head[0].update_seq != $before.base_update_seq + 1
            OR !fn::wp012_model_lane_application_evidence($before, $head[0], (SELECT * FROM ONLY $before.applied_event_id)) {
            THROW 'HSK-403-MODEL-LANE-PROPOSAL-PROMOTION';
        };
    } ELSE IF $wp012_original_purpose = 'proposal_decide' {
        IF $before.review_state != 'proposed' OR $after.review_state NOT IN ['approved', 'rejected']
            OR $before.decided_event_id != NONE OR $after.decided_event_id = NONE
            OR $auth.principal_id.principal_kind NOT IN ['human_account', 'break_glass_operator']
            OR $auth.principal_id.actor_kind != 'operator' OR $after.decided_by != $auth.principal_id.actor_id
            OR $after.decided_event_id.authority_session_id != $auth.id
            OR $after.decided_event_id.authority_resource_id != type::record('protected_resources', $wp012_original_resource)
            OR $after.decided_event_id.authority_grant_id != type::record('resource_grants', $wp012_original_grant)
            OR $after.decided_at_utc = NONE OR $after.decided_at_utc > time::now() {
            THROW 'HSK-403-MODEL-LANE-PROPOSAL-REVIEW';
        };
    } ELSE {
        IF $wp012_original_purpose != 'proposal_apply' OR $before.review_state != 'approved'
            OR $before.applied_update_id != NONE OR $after.applied_update_id = NONE
            OR $after.applied_update_sha256 != $before.diff_sha256 { THROW 'HSK-403-MODEL-LANE-PROPOSAL-APPLY'; };
        LET $updates = SELECT * FROM kernel_crdt_updates WHERE update_id = $after.applied_update_id
            AND owner_account_id = $before.owner_account_id AND actor_principal_id = $before.actor_principal_id
            AND authenticated_session_id = $before.authenticated_session_id AND access_space_id = $before.access_space_id
            AND workspace_id = $before.workspace_id AND document_id = $before.document_id AND crdt_document_id = $before.crdt_document_id;
        IF array::len($updates) != 1 OR !fn::wp012_model_lane_crdt_document_access($updates[0], false, false)
            OR $updates[0].actor_id != $before.actor_id OR $updates[0].actor_kind != $before.actor_kind
            OR $updates[0].session_id != $before.session_id OR $updates[0].trace_id != $before.correlation_id
            OR $updates[0].update_seq != $before.base_update_seq + 1 OR $updates[0].state_vector_before != $before.base_state_vector
            OR $after.applied_event_id.payload.yjs_update_sha256 != $updates[0].update_sha256
            OR $after.applied_event_id.payload.update_seq != $updates[0].update_seq
            OR $after.applied_event_id.payload.update_state_vector_before != $updates[0].state_vector_before
            OR $after.applied_event_id.payload.update_event_ledger_event_id != record::id($updates[0].event_ledger_event_id)
            OR $after.applied_event_id.payload.update_ledger_payload_hash != $updates[0].event_ledger_event_id.payload_hash
            OR !fn::wp012_model_lane_application_evidence($before, $updates[0], (SELECT * FROM ONLY $after.applied_event_id)) {
            THROW 'HSK-403-MODEL-LANE-PROPOSAL-APPLICATION';
        };
    };
};"#),
    ),
];

const CLOUD_MODEL_LANE_DECLARATIONS: &[(&str, Inverse)] = &[
    (
        "DEFINE FUNCTION OVERWRITE fn::wp012_cloud_schema_access(",
        Inverse::Previous(r#"DEFINE FUNCTION OVERWRITE fn::wp012_cloud_schema_access() {
    LET $scope = { owner_account_id: record::id($auth.account_id), actor_principal_id: record::id($auth.principal_id), authenticated_session_id: record::id($auth.id), access_space_id: record::id($auth.access_space_id), workspace_id: $wp012_original_workspace };
    RETURN fn::wp012_model_lane_original_operation($scope)
        AND (($wp012_original_action = 'read' AND $wp012_original_purpose = 'source_read')
            OR ($wp012_original_action = 'create' AND $wp012_original_purpose IN ['cloud_projection_plan', 'cloud_consent_receipt', 'cloud_run', 'cloud_lane', 'cloud_consent_denial']));
};"#),
    ),
    (
        "DEFINE FUNCTION OVERWRITE fn::wp012_cloud_source_access(",
        Inverse::Previous(r#"DEFINE FUNCTION OVERWRITE fn::wp012_cloud_source_access($row: object) {
    IF $row.kind NOT IN ['projection_plan', 'consent_receipt', 'cloud_run', 'cloud_lane', 'consent_denial']
        OR !fn::wp012_model_lane_original_operation($row) { RETURN false; };
    RETURN ($wp012_original_action = 'read' AND $wp012_original_purpose = 'source_read')
        OR ($wp012_original_action = 'create' AND $wp012_original_purpose IN ['cloud_projection_plan', 'cloud_consent_receipt', 'cloud_run', 'cloud_lane', 'cloud_consent_denial']);
};"#),
    ),
    (
        "DEFINE FUNCTION OVERWRITE fn::wp012_cloud_receipt_create(",
        Inverse::Previous(r#"DEFINE FUNCTION OVERWRITE fn::wp012_cloud_receipt_create($row: object) {
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
};"#),
    ),
    (
        "DEFINE FUNCTION OVERWRITE fn::wp012_cloud_authority_create(",
        Inverse::Previous(r#"DEFINE FUNCTION OVERWRITE fn::wp012_cloud_authority_create($row: object) {
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
};"#),
    ),
    (
        "DEFINE FUNCTION OVERWRITE fn::wp012_cloud_record_identity(",
        Inverse::Fragments(&[
            (r#"        OR $record.event_ledger_seq != $row.event_seq OR string::trim($record.event_ledger_stream_id) = ''
        OR ($row.kind IN ['projection_plan', 'consent_receipt', 'consent_revocation'] AND ($record.event_stream_version != $row.event_seq OR $record.transaction_seq != $row.event_seq)) { RETURN false; };
    // The pinned canonical consent_scope/target_bindings algorithm returns None.
    // Its derived field is omitted by serde; a supplied derived hash is noncanonical.
    IF $row.kind IN ['projection_plan', 'consent_receipt', 'consent_revocation'] AND $record.target_bindings_hash != NONE { RETURN false; };
    IF $row.kind = 'projection_plan' {
"#, r#"        OR $record.event_ledger_seq != $row.event_seq OR string::trim($record.event_ledger_stream_id) = ''
        OR ($row.kind IN ['projection_plan', 'consent_receipt'] AND ($record.event_stream_version != $row.event_seq OR $record.transaction_seq != $row.event_seq)) { RETURN false; };
    // The pinned canonical consent_scope/target_bindings algorithm returns None.
    // Its derived field is omitted by serde; a supplied derived hash is noncanonical.
    IF $row.kind IN ['projection_plan', 'consent_receipt'] AND $record.target_bindings_hash != NONE { RETURN false; };
    IF $row.kind = 'projection_plan' {
"#),
            (r#"    };
    IF $row.kind IN ['consent_receipt', 'consent_revocation'] {
        LET $body = object::from_entries(array::filter(object::entries($record), |$entry| $entry[0] NOT IN ['event_ledger_event_id', 'event_ledger_seq', 'event_stream_version', 'transaction_seq', 'target_bindings_hash', 'consent_receipt_hash']));
        IF $row.kind = 'consent_revocation' {
            RETURN $record.consent_receipt_id = $row.aggregate_id AND $row.consent_receipt_id = $row.aggregate_id
                AND $row.projection_plan_id = $record.projection_plan_id
                AND $record.status = 'revoked' AND $record.approved = false
                AND type::is_string($record.revocation_ref) AND string::trim($record.revocation_ref) != ''
                AND fn::wp012_cloud_attribution($record.approver, $row)
                AND $record.consent_receipt_hash = crypto::sha256(encoding::json::encode($body));
        };
        LET $plans = SELECT * FROM model_lane_cloud_authority WHERE kind = 'projection_plan' AND aggregate_id = $record.projection_plan_id
"#, r#"    };
    IF $row.kind = 'consent_receipt' {
        LET $body = object::from_entries(array::filter(object::entries($record), |$entry| $entry[0] NOT IN ['event_ledger_event_id', 'event_ledger_seq', 'event_stream_version', 'transaction_seq', 'target_bindings_hash', 'consent_receipt_hash']));
        LET $plans = SELECT * FROM model_lane_cloud_authority WHERE kind = 'projection_plan' AND aggregate_id = $record.projection_plan_id
"#),
            (r#"        LET $plan = encoding::json::decode($plans[0].record_json);
        LET $valid_from = type::datetime($record.valid_from_utc);
        LET $valid_until = type::datetime($record.valid_until_utc);
        RETURN $record.consent_receipt_id = $row.aggregate_id AND $row.consent_receipt_id = $row.aggregate_id
"#, r#"        LET $plan = encoding::json::decode($plans[0].record_json);
        RETURN $record.consent_receipt_id = $row.aggregate_id AND $row.consent_receipt_id = $row.aggregate_id
"#),
            (r#"            AND $record.retention_policy = $plan.retention_policy AND $record.fan_out_targets = $plan.fan_out_targets
            AND type::is_datetime($valid_from) AND type::is_datetime($valid_until)
            AND $valid_from < $valid_until AND $valid_from <= time::now()
            AND ($valid_until > time::now()
                OR ($wp012_original_subject_kind = 'workspace' AND $wp012_original_action = 'update'
                    AND $wp012_original_purpose = 'cloud_consent_revocation' AND fn::wp012_model_lane_original_operation($row)));
    };
"#, r#"            AND $record.retention_policy = $plan.retention_policy AND $record.fan_out_targets = $plan.fan_out_targets
            AND type::datetime($record.valid_from_utc) <= time::now() AND type::datetime($record.valid_until_utc) > time::now();
    };
"#),
            (r#"    };
    IF $row.kind = 'lane_terminal' {
        RETURN $record.lane_id = $row.aggregate_id AND $record.status = 'cancelled' AND $record.failstate_code = 'CX-MM-007'
            AND type::is_string($record.consent_receipt_ref) AND string::trim($record.consent_receipt_ref) != ''
            AND $row.consent_receipt_id = $record.consent_receipt_ref AND $row.projection_plan_id = $record.projection_plan_ref;
    };
    RETURN $row.kind = 'consent_denial' AND $record.lane_id = $row.aggregate_id
"#, r#"    };
    RETURN $row.kind = 'consent_denial' AND $record.lane_id = $row.aggregate_id
"#),
        ]),
    ),
    (
        "DEFINE FIELD OVERWRITE allocation ON model_lane_cloud_event_ledger ",
        Inverse::Previous(r#"DEFINE FIELD OVERWRITE allocation ON model_lane_cloud_event_ledger TYPE object FLEXIBLE VALUE {
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
} PERMISSIONS FOR select NONE;"#),
    ),
];
