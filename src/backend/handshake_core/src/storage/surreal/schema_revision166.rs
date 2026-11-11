use super::*;

pub(super) const BASE_GENERATED: &str =
    "86d70b003a031c84163076b8c99bc8ca957696302907ce88b4723c617a5a944f";
pub(super) const PRODUCTION_GENERATED: &str =
    "29d1e0d7442159f01f69e3f7a2cc406baaa96475c3e302728e1f8c0f92b2689b";
pub(super) const PRODUCTION_SOURCE: &str =
    "40a646d1e860cc0bd59e319b0a2b973cd9f589d4d080dd0243ea2713505da900";
pub(super) const BASE_CATALOG: &str =
    "4ee81364add186ee33453775fcc2d0a1885906205aa2ee328d9a7055c7c67a0d";
pub(super) const PRODUCTION_CATALOG: &str =
    "aa37b7e044925e96794d0b71549f7c2fcf49c2b1e6cb47c8363c1241239c3f59";

pub(super) fn recognizes(
    state: &SchemaState,
    production: bool,
    config: &SurrealStorageConfig,
) -> bool {
    state.version == SCHEMA_VERSION
        && state.revision == PRE_ROUTING_REVISION
        && state.target_revision == PRE_ROUTING_REVISION
        && state.namespace == config.namespace()
        && state.database == config.database()
        && state.apply_state == "complete"
        && state.source_manifest_sha256
            == if production {
                PRODUCTION_SOURCE
            } else {
                SCHEMA_LINEAGE_SHA256
            }
        && state.generated_surql_sha256
            == if production {
                PRODUCTION_GENERATED
            } else {
                BASE_GENERATED
            }
        && if production {
            EXPECTED_PRODUCTION_SCHEMA_PIN.is_some_and(|pin| state.info_fingerprint_sha256 == pin.0)
        } else {
            state.info_fingerprint_sha256 == EXPECTED_SCHEMA_INFO_SHA256
        }
}

pub(super) fn predecessor_catalog_sha256(production: bool) -> Result<String, SurrealStorageError> {
    let entries = if production {
        production_catalog_entries()
    } else {
        compiled_schema_catalog_entries()
    }
    .map_err(SurrealStorageError::TransactionWorker)?;
    let mut entries = entries.into_iter().collect::<BTreeSet<_>>();
    for identity in BASE_ADDITIONS
        .iter()
        .chain(if production { MODULE_ADDITIONS } else { &[] }.iter())
    {
        if !entries.remove(*identity) {
            return Err(SurrealStorageError::TransactionWorker(format!(
                "first-pin catalog addition missing: {identity}"
            )));
        }
    }
    Ok(compute_catalog_hash(
        &entries.into_iter().collect::<Vec<_>>(),
    ))
}

pub(super) fn upgrade_statements(production: bool) -> String {
    let mut sources = vec![(SCHEMA, BASE_HEADERS)];
    if production {
        sources.push((
            include_str!("model_registry_schema.surql"),
            MODEL_REGISTRY_HEADERS,
        ));
    }
    let mut result = String::new();
    for (source, headers) in sources {
        for (start, end) in canonical_upgrade_spans(source, headers) {
            result.push_str(&source[start..end]);
            result.push('\n');
        }
    }
    result
}

pub(super) fn cloud_upgrade_statements(production: bool) -> String {
    if !production {
        return String::new();
    }
    let source = include_str!("cloud_model_lane_schema.surql");
    let mut result = String::new();
    for (start, end) in canonical_upgrade_spans(source, CLOUD_HEADERS) {
        result.push_str(&source[start..end]);
        result.push('\n');
    }
    result
}

const BASE_HEADERS: &[&str] = &[
    "DEFINE FUNCTION OVERWRITE fn::wp012_member_redemption_password_read(",
    "DEFINE FUNCTION OVERWRITE fn::wp012_engine_profile_session(",
    "DEFINE TABLE OVERWRITE model_sessions ",
    "DEFINE FIELD OVERWRITE engine_profile_binding ON TABLE model_sessions",
    "DEFINE EVENT OVERWRITE engine_profile_session_origin_guard ON TABLE model_sessions",
    "DEFINE TABLE OVERWRITE kernel_event_ledger ",
    "DEFINE FIELD OVERWRITE session_run_id ON TABLE kernel_event_ledger",
    "DEFINE FIELD OVERWRITE idempotency_key ON TABLE kernel_event_ledger",
    "DEFINE FIELD OVERWRITE correlation_id ON TABLE kernel_event_ledger",
    "DEFINE FIELD OVERWRITE payload_hash ON TABLE kernel_event_ledger",
    "DEFINE FIELD OVERWRITE payload ON TABLE kernel_event_ledger",
    "DEFINE FIELD OVERWRITE payload.* ON TABLE kernel_event_ledger",
    "DEFINE FUNCTION OVERWRITE fn::wp012_navigation_receipt_semantics(",
    "DEFINE FIELD OVERWRITE navigation_semantic_input ON TABLE kernel_event_ledger",
    "DEFINE FIELD OVERWRITE navigation_semantic_input.schema_id ON TABLE kernel_event_ledger",
    "DEFINE FIELD OVERWRITE navigation_semantic_input.proposal_id ON TABLE kernel_event_ledger",
    "DEFINE FIELD OVERWRITE navigation_semantic_input.diff_sha256 ON TABLE kernel_event_ledger",
    "DEFINE FIELD OVERWRITE navigation_semantic_input.document_id ON TABLE kernel_event_ledger",
    "DEFINE FIELD OVERWRITE navigation_semantic_input.crdt_document_id ON TABLE kernel_event_ledger",
    "DEFINE FIELD OVERWRITE navigation_semantic_input.recorded_event_id ON TABLE kernel_event_ledger",
    "DEFINE FIELD OVERWRITE navigation_semantic_input.reviewer_principal_id ON TABLE kernel_event_ledger",
    "DEFINE FIELD OVERWRITE navigation_semantic_input.reviewer_authenticated_session_id ON TABLE kernel_event_ledger",
    "DEFINE FIELD OVERWRITE navigation_semantic_input.reviewer_action ON TABLE kernel_event_ledger",
    "DEFINE FIELD OVERWRITE navigation_semantic_input.reviewer_capability_id ON TABLE kernel_event_ledger",
    "DEFINE FIELD OVERWRITE navigation_semantic_input.reviewer_resource_id ON TABLE kernel_event_ledger",
    "DEFINE FIELD OVERWRITE navigation_semantic_input.reviewer_grant_id ON TABLE kernel_event_ledger",
    "DEFINE FIELD OVERWRITE navigation_semantic_input.review_state ON TABLE kernel_event_ledger",
    "DEFINE FIELD OVERWRITE navigation_semantic_input.applied_update_id ON TABLE kernel_event_ledger",
    "DEFINE FIELD OVERWRITE navigation_semantic_input.applied_update_sha256 ON TABLE kernel_event_ledger",
    "DEFINE FIELD OVERWRITE navigation_semantic_input.approved_diff_sha256 ON TABLE kernel_event_ledger",
    "DEFINE FIELD OVERWRITE navigation_semantic_input.base_state_vector ON TABLE kernel_event_ledger",
    "DEFINE FIELD OVERWRITE navigation_semantic_input.decided_by ON TABLE kernel_event_ledger",
    "DEFINE FIELD OVERWRITE navigation_semantic_input.base_update_seq ON TABLE kernel_event_ledger",
    "DEFINE FIELD OVERWRITE navigation_semantic_input.application_evidence ON TABLE kernel_event_ledger",
    "DEFINE FIELD OVERWRITE navigation_semantic_input.application_evidence.schema_id ON TABLE kernel_event_ledger",
    "DEFINE FIELD OVERWRITE navigation_semantic_input.application_evidence.operation ON TABLE kernel_event_ledger",
    "DEFINE FIELD OVERWRITE navigation_semantic_input.application_evidence.proposal_id ON TABLE kernel_event_ledger",
    "DEFINE FIELD OVERWRITE navigation_semantic_input.application_evidence.document_id ON TABLE kernel_event_ledger",
    "DEFINE FIELD OVERWRITE navigation_semantic_input.application_evidence.crdt_document_id ON TABLE kernel_event_ledger",
    "DEFINE FIELD OVERWRITE navigation_semantic_input.application_evidence.recorded_event_id ON TABLE kernel_event_ledger",
    "DEFINE FIELD OVERWRITE navigation_semantic_input.application_evidence.approval_event_id ON TABLE kernel_event_ledger",
    "DEFINE FIELD OVERWRITE navigation_semantic_input.application_evidence.base_update_id ON TABLE kernel_event_ledger",
    "DEFINE FIELD OVERWRITE navigation_semantic_input.application_evidence.base_snapshot_id ON TABLE kernel_event_ledger",
    "DEFINE FIELD OVERWRITE navigation_semantic_input.application_evidence.base_snapshot_event_id ON TABLE kernel_event_ledger",
    "DEFINE FIELD OVERWRITE navigation_semantic_input.application_evidence.base_update_event_id ON TABLE kernel_event_ledger",
    "DEFINE FIELD OVERWRITE navigation_semantic_input.application_evidence.update_id ON TABLE kernel_event_ledger",
    "DEFINE FIELD OVERWRITE navigation_semantic_input.application_evidence.update_event_id ON TABLE kernel_event_ledger",
    "DEFINE FIELD OVERWRITE navigation_semantic_input.application_evidence.approved_diff_sha256 ON TABLE kernel_event_ledger",
    "DEFINE FIELD OVERWRITE navigation_semantic_input.application_evidence.yjs_update_sha256 ON TABLE kernel_event_ledger",
    "DEFINE FIELD OVERWRITE navigation_semantic_input.application_evidence.canonical_base_materialization_sha256 ON TABLE kernel_event_ledger",
    "DEFINE FIELD OVERWRITE navigation_semantic_input.application_evidence.canonical_post_materialization_sha256 ON TABLE kernel_event_ledger",
    "DEFINE FIELD OVERWRITE navigation_semantic_input.application_evidence.base_update_seq ON TABLE kernel_event_ledger",
    "DEFINE FIELD OVERWRITE navigation_semantic_input.application_evidence.update_seq ON TABLE kernel_event_ledger",
    "DEFINE FIELD OVERWRITE navigation_semantic_input.application_evidence.approval_event_sequence ON TABLE kernel_event_ledger",
    "DEFINE FIELD OVERWRITE navigation_semantic_input.application_evidence.update_event_sequence ON TABLE kernel_event_ledger",
    "DEFINE FIELD OVERWRITE navigation_semantic_input.application_evidence.path ON TABLE kernel_event_ledger",
    "DEFINE FIELD OVERWRITE created_at ON TABLE kernel_event_ledger",
    "DEFINE TABLE OVERWRITE kernel_process_lifecycle ",
    "DEFINE FIELD OVERWRITE engine_kind ON TABLE kernel_process_lifecycle",
    "DEFINE FUNCTION OVERWRITE fn::wp012_engine_profile_current_owner(",
    "DEFINE FUNCTION OVERWRITE fn::wp012_engine_profile_ledger_read(",
    "DEFINE FUNCTION OVERWRITE fn::wp012_engine_profile_snapshot(",
    "DEFINE FUNCTION OVERWRITE fn::wp012_engine_profile_transition(",
    "DEFINE FUNCTION OVERWRITE fn::wp012_api_receipt(",
    "DEFINE FIELD OVERWRITE authority_resource_id ON TABLE kernel_event_ledger",
    "DEFINE FIELD OVERWRITE authority_grant_id ON TABLE kernel_event_ledger",
    "DEFINE FIELD OVERWRITE authority_session_id ON TABLE kernel_event_ledger",
    "DEFINE FIELD OVERWRITE authority_capability_id ON TABLE kernel_event_ledger",
    "DEFINE FIELD OVERWRITE authority_action ON TABLE kernel_event_ledger",
    "DEFINE FUNCTION OVERWRITE fn::wp012_first_owner_workspace_sources(",
    "DEFINE FUNCTION OVERWRITE fn::mt120_creator_grant(",
    "DEFINE FUNCTION OVERWRITE fn::wp012_model_lane_receipt(",
    "DEFINE FIELD OVERWRITE authority_parent_resource_id ON TABLE kernel_event_ledger",
];

const MODEL_REGISTRY_HEADERS: &[&str] = &[
    "DEFINE FUNCTION OVERWRITE fn::wp012_model_registry_request(",
    "DEFINE FUNCTION OVERWRITE fn::wp012_model_registry_active_payload(",
    "DEFINE FUNCTION OVERWRITE fn::wp012_model_registry_active_event(",
    "DEFINE FUNCTION OVERWRITE fn::wp012_model_registry_active_row(",
    "DEFINE FUNCTION OVERWRITE fn::wp012_model_registry_process_read(",
    "DEFINE FUNCTION OVERWRITE fn::wp012_model_registry_optional(",
    "DEFINE FUNCTION OVERWRITE fn::wp012_model_registry_process_receipt_read(",
    "DEFINE FUNCTION OVERWRITE fn::wp012_model_registry_registration_selection(",
    "DEFINE FUNCTION OVERWRITE fn::wp012_model_registry_rebind_payload(",
    "DEFINE FUNCTION OVERWRITE fn::wp012_model_registry_registration_event(",
    "DEFINE FUNCTION OVERWRITE fn::wp012_model_registry_registration_row(",
    "DEFINE TABLE OVERWRITE model_runtime_registry ",
    "DEFINE FIELD OVERWRITE runtime_binding ON TABLE model_runtime_registry",
    "DEFINE TABLE OVERWRITE model_runtime_active_selection ",
    "DEFINE FUNCTION OVERWRITE fn::wp012_engine_profile_row(",
    "DEFINE TABLE OVERWRITE engine_profile_registry ",
    "DEFINE FIELD OVERWRITE owner_account_id ON engine_profile_registry",
    "DEFINE FIELD OVERWRITE actor_principal_id ON engine_profile_registry",
    "DEFINE FIELD OVERWRITE authenticated_session_id ON engine_profile_registry",
    "DEFINE FIELD OVERWRITE access_space_id ON engine_profile_registry",
    "DEFINE FIELD OVERWRITE workspace_id ON engine_profile_registry",
    "DEFINE FIELD OVERWRITE data ON engine_profile_registry",
    "DEFINE FIELD OVERWRITE data.* ON engine_profile_registry",
    "DEFINE FIELD OVERWRITE receipt_id ON engine_profile_registry",
    "DEFINE INDEX OVERWRITE engine_profile_registry_scope ON engine_profile_registry",
    "DEFINE EVENT OVERWRITE engine_profile_registry_transition_guard\nON TABLE engine_profile_registry",
    "DEFINE EVENT OVERWRITE model_runtime_active_selection_transition_guard\nON TABLE model_runtime_active_selection",
    "DEFINE EVENT OVERWRITE model_runtime_registry_transition_guard\nON TABLE model_runtime_registry",
];

const CLOUD_HEADERS: &[&str] = &[
    "DEFINE TABLE OVERWRITE model_lane_cloud_event_ledger ",
    "DEFINE FUNCTION OVERWRITE fn::wp012_cloud_checkpoint_dependency_read(",
    "DEFINE FUNCTION OVERWRITE fn::wp012_cloud_checkpoint_denial_membership(",
    "DEFINE FUNCTION OVERWRITE fn::wp012_cloud_authority_read(",
];

const BASE_ADDITIONS: &[&str] = &[
    "event:model_sessions:engine_profile_session_origin_guard",
    "field:model_sessions:engine_profile_binding",
    "function:fn::wp012_engine_profile_current_owner",
    "function:fn::wp012_engine_profile_ledger_read",
    "function:fn::wp012_engine_profile_session",
    "function:fn::wp012_engine_profile_snapshot",
    "function:fn::wp012_engine_profile_transition",
];

const MODULE_ADDITIONS: &[&str] = &[
    "event:engine_profile_registry:engine_profile_registry_transition_guard",
    "event:model_lane_authority:wp012_run_attachment_update",
    "event:model_lane_profile_admission_state:profile_admission_state_preimage_guard",
    "event:model_runtime_active_selection:model_runtime_active_selection_transition_guard",
    "event:model_runtime_registry:model_runtime_registry_transition_guard",
    "field:engine_profile_registry:access_space_id",
    "field:engine_profile_registry:actor_principal_id",
    "field:engine_profile_registry:authenticated_session_id",
    "field:engine_profile_registry:data",
    "field:engine_profile_registry:data.*",
    "field:engine_profile_registry:owner_account_id",
    "field:engine_profile_registry:receipt_id",
    "field:engine_profile_registry:workspace_id",
    "field:model_lane_profile_admission_state:generation",
    "field:model_lane_profile_admission_state:origin_id",
    "field:model_lane_profile_admission_state:receipt_id",
    "field:model_lane_profile_admission_state:status",
    "function:fn::wp012_cloud_checkpoint_denial_membership",
    "function:fn::wp012_cloud_checkpoint_dependency_read",
    "function:fn::wp012_engine_profile_row",
    "function:fn::wp012_model_registry_active_event",
    "function:fn::wp012_model_registry_active_payload",
    "function:fn::wp012_model_registry_active_row",
    "function:fn::wp012_model_registry_optional",
    "function:fn::wp012_model_registry_process_read",
    "function:fn::wp012_model_registry_process_receipt_read",
    "function:fn::wp012_model_registry_rebind_payload",
    "function:fn::wp012_model_registry_registration_event",
    "function:fn::wp012_model_registry_registration_row",
    "function:fn::wp012_model_registry_registration_selection",
    "function:fn::wp012_model_registry_request",
    "function:fn::wp012_profile_admission_current_head",
    "function:fn::wp012_profile_admission_origin_piece",
    "function:fn::wp012_profile_admission_state_access",
    "function:fn::wp012_profile_admission_transition_record",
    "function:fn::wp012_profile_checkpoint_runtime",
    "function:fn::wp012_promotion_current",
    "function:fn::wp012_promotion_current_body",
    "function:fn::wp012_promotion_durable_body",
    "function:fn::wp012_promotion_durable_decision",
    "function:fn::wp012_promotion_frame",
    "function:fn::wp012_promotion_identity",
    "function:fn::wp012_promotion_ledger_identity",
    "function:fn::wp012_promotion_lineage",
    "function:fn::wp012_promotion_public_receipt",
    "function:fn::wp012_promotion_public_record",
    "function:fn::wp012_promotion_read_basis",
    "function:fn::wp012_promotion_receipt",
    "function:fn::wp012_promotion_role_selection",
    "function:fn::wp012_promotion_scope",
    "function:fn::wp012_promotion_scope_matches",
    "function:fn::wp012_promotion_selected_read",
    "function:fn::wp012_promotion_source_access",
    "function:fn::wp012_recovery_discovery_access",
    "function:fn::wp012_routing_ack_fence",
    "function:fn::wp012_routing_ack_read_receipt",
    "function:fn::wp012_routing_ack_read_source",
    "function:fn::wp012_routing_ack_receipt_read",
    "function:fn::wp012_routing_artifact_message",
    "function:fn::wp012_routing_authority_snapshot",
    "function:fn::wp012_routing_execution_projection",
    "function:fn::wp012_routing_extra_link_access",
    "function:fn::wp012_routing_extra_receipt",
    "function:fn::wp012_routing_model_lane_receipt",
    "function:fn::wp012_routing_original",
    "function:fn::wp012_routing_receipt",
    "function:fn::wp012_routing_record_access",
    "function:fn::wp012_routing_response_message",
    "function:fn::wp012_routing_role_receipt",
    "function:fn::wp012_routing_role_source_access",
    "function:fn::wp012_routing_selected",
    "function:fn::wp012_routing_source_access",
    "function:fn::wp012_routing_source_identity",
    "function:fn::wp012_routing_write_fence",
    "function:fn::wp012_run_attachment_authority",
    "function:fn::wp012_run_attachment_identity",
    "function:fn::wp012_run_attachment_merge",
    "function:fn::wp012_run_attachment_receipt",
    "index:engine_profile_registry:engine_profile_registry_scope",
    "table:engine_profile_registry",
    "table:model_lane_profile_admission_state",
];

#[cfg(test)]
pub(super) const SOURCE_PINS: &[(&str, &str)] = &[
    (
        "shared",
        "86d70b003a031c84163076b8c99bc8ca957696302907ce88b4723c617a5a944f",
    ),
    (
        "model_registry",
        "e68d2abce933d715798a458a7aa70e0d09752707d3226178bafb2a39005955d6",
    ),
    (
        "loom_search",
        "55838a2b94e7eca696de04aadf4ce5b70b9a29562f6ee01f8b0e0f1f46d8bce8",
    ),
    (
        "model_lane_state",
        "529bfea18e6f6c3ecedcfe8406594fac95867e9300ce978faf508869e0358450",
    ),
    (
        "model_lane",
        "f486549c1344b93649b615ac2235fc7370c12be53a6a302be184fefdad6f6341",
    ),
    (
        "cloud_model_lane_state",
        "d55b974bf7a46de6b27a43b38f31b8bc99f5b2f882189203b440d3359cc18ce7",
    ),
    (
        "cloud_model_lane",
        "ec417603c3520794a829b2f95e03b570f371db8b2b743ddbb7c61146dfb69b77",
    ),
    (
        "process_ledger",
        "88fcf556e38da0e1b281ea6eb54a0a07bf22886296a67776c9a2c0e4d272a1a1",
    ),
    (
        "palmistry_state",
        "4ecef5ccae3e59e24161610114b995462388f3a0dfdb4f01f9b138a36f563b6f",
    ),
    (
        "palmistry",
        "55fbeb9c36c1bc9bdaf7b2f9d152bca37860f8b143d895202fee4ab494928f57",
    ),
    (
        "swarm_outbox_state",
        "06569d29f38fce05c16c439d1a20d8ceea4c0bcb627be306f8cab388e5cd4b51",
    ),
    (
        "swarm_outbox",
        "1436a9b91a2ccb6907d4f8c8742090d227708392aa969212f840ff9986f1042f",
    ),
    (
        "user_manual_knowledge",
        "abd49a97afbda36eec818a04fc08ffd5f51f88f8ff8e42ea19fcb7e937985805",
    ),
    (
        "workspace_settings_state",
        "16f1ad96ee279f6b18b8fab145f457b2485669da6f5c8c562aedd789b11ccc5e",
    ),
    (
        "workspace_settings",
        "e04efa94b38a2749903a4dd39f1aacd54f2a86b8834ffb27ad5937cf8bcb0d0c",
    ),
];
