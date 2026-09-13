use std::time::Duration;

use super::resource_authority::{
    AuthorizationRequest, RecordUserScope, ResourceAction, ResourceAuthorityError, ResourceGrant,
    ResourceGrantSpec, ResourceKind,
};
use super::SurrealDatabase;
use crate::storage::tests::embedded_test_backend;
use crate::storage::{Database, NewWorkspace, WriteContext};
use surrealdb::types::{RecordId, SurrealValue, Value};

#[derive(Clone, SurrealValue)]
struct OperationalProbeBindings {
    table: String,
    record_id: String,
    sentinel_id: String,
    workspace: RecordId,
    workspace_key: String,
    authority_resource: RecordId,
    authority_session: RecordId,
    hash: String,
}

#[derive(SurrealValue)]
struct GrantPrincipalBinding {
    principal: RecordId,
}

struct DeniedOperationalCase {
    label: &'static str,
    scope: RecordUserScope,
}

const OPERATIONAL_SENTINEL_ID: &str = "mt109-v12-sentinel";
const OPERATIONAL_HASH: &str = "0000000000000000000000000000000000000000000000000000000000000000";
const WORKSPACE_RESOURCE_CAPABILITIES: &[&str] = &[
    "fr.read",
    "fr.ingest.runtime_chat",
    "fr.ingest.native_editor",
    "memory.read",
    "memory.propose",
    "memory.review",
    "memory.commit",
];

async fn grant_existing_route_resources_to(
    storage: &super::SurrealStorage,
    resource_owner: &super::resource_authority::ProvisionedPrincipal,
    grantee: &super::resource_authority::ProvisionedPrincipal,
    workspace_id: &str,
    action_override: Option<ResourceAction>,
    capability_override: Option<&str>,
    delegation_chain: Option<&[String]>,
) -> Result<Vec<ResourceGrant>, ResourceAuthorityError> {
    let mut grants = Vec::with_capacity(PROTECTED_RESOURCE_ACTIONS.len() + 1);
    let workspace = storage
        .authorize_protected_resource(AuthorizationRequest {
            session_token: resource_owner.session.token.clone(),
            channel_binding_hash: Some("direct-negative-binding".to_owned()),
            capability_id: "fr.read".to_owned(),
            resource_kind: ResourceKind::Workspace,
            external_resource_id: workspace_id.to_owned(),
            action: ResourceAction::Read,
        })
        .await?;
    grants.push(
        storage
            .grant_resource(
                &grantee.identity.account_id,
                &grantee.identity.access_space_id,
                ResourceGrantSpec {
                    principal_id: grantee.identity.principal_id.clone(),
                    resource_id: workspace.resource_id,
                    actions: action_override.map_or_else(
                        || {
                            vec![
                                ResourceAction::Read,
                                ResourceAction::Create,
                                ResourceAction::Update,
                            ]
                        },
                        |action| vec![action],
                    ),
                    capability_ids: capability_override.map_or_else(
                        || {
                            WORKSPACE_RESOURCE_CAPABILITIES
                                .iter()
                                .map(|capability| (*capability).to_owned())
                                .collect()
                        },
                        |capability| vec![capability.to_owned()],
                    ),
                    expires_at: Some(grantee.session.expires_at),
                    delegation_chain: delegation_chain.unwrap_or_default().to_vec(),
                },
            )
            .await?,
    );
    for case in PROTECTED_RESOURCE_ACTIONS {
        let decision = storage
            .authorize_protected_resource(matrix_request(
                &resource_owner.session.token,
                "direct-negative-binding",
                *case,
                workspace_id,
            ))
            .await?;
        grants.push(
            storage
                .grant_resource(
                    &grantee.identity.account_id,
                    &grantee.identity.access_space_id,
                    ResourceGrantSpec {
                        principal_id: grantee.identity.principal_id.clone(),
                        resource_id: decision.resource_id,
                        actions: vec![action_override.unwrap_or(case.action)],
                        capability_ids: vec![capability_override
                            .unwrap_or(case.capability)
                            .to_owned()],
                        expires_at: Some(grantee.session.expires_at),
                        delegation_chain: delegation_chain.unwrap_or_default().to_vec(),
                    },
                )
                .await?,
        );
    }
    Ok(grants)
}

#[derive(SurrealValue)]
struct MissingResourceGrantBindings {
    grant: RecordId,
    account: RecordId,
    principal: RecordId,
    space: RecordId,
    missing_resource: RecordId,
    actions: Vec<String>,
    capabilities: Vec<String>,
    delegation_chain: Vec<String>,
}

async fn grant_missing_resource_relation_to(
    storage: &super::SurrealStorage,
    principal: &super::resource_authority::ProvisionedPrincipal,
) -> Result<(), Box<dyn std::error::Error>> {
    let principal_id = principal.identity.principal_id.clone();
    let bindings = MissingResourceGrantBindings {
        grant: RecordId::new("resource_grants", "mt109-v14-missing-resource-grant"),
        account: RecordId::new("local_accounts", principal.identity.account_id.clone()),
        principal: RecordId::new("principals", principal_id.clone()),
        space: RecordId::new("access_spaces", principal.identity.access_space_id.clone()),
        missing_resource: RecordId::new(
            "protected_resources",
            "mt109-v14-deliberately-absent-resource",
        ),
        actions: ["read", "create", "update"].map(str::to_owned).to_vec(),
        capabilities: WORKSPACE_RESOURCE_CAPABILITIES
            .iter()
            .map(|capability| (*capability).to_owned())
            .collect(),
        delegation_chain: vec![principal_id],
    };
    storage
        .with_data_operation(move |database| {
            Box::pin(async move {
                database
                    .query_values::<Value, _>(
                        "CREATE $grant SET account_id = $account, principal_id = $principal, access_space_id = $space, resource_id = $missing_resource, actions = $actions, capability_ids = $capabilities, delegation_chain = $delegation_chain, status = 'active', grant_version = 1, policy_version = 1, expires_at = NONE, revoked_at = NONE, created_at = time::now(), updated_at = time::now();",
                        bindings,
                    )
                    .await
                    .map(|_| ())
            })
        })
        .await?;
    Ok(())
}

async fn clear_principal_grant_field(
    storage: &super::SurrealStorage,
    principal_id: &str,
    field: &'static str,
) -> Result<(), Box<dyn std::error::Error>> {
    let principal = RecordId::new("principals", principal_id);
    storage
        .with_data_operation(move |database| {
            Box::pin(async move {
                let statement = match field {
                    "actions" => {
                        "UPDATE resource_grants SET actions = [] WHERE principal_id = $principal;"
                    }
                    "capability_ids" => "UPDATE resource_grants SET capability_ids = [] WHERE principal_id = $principal;",
                    _ => unreachable!("unsupported grant field"),
                };
                database
                    .query_values::<Value, _>(statement, GrantPrincipalBinding { principal })
                    .await
                    .map(|_| ())
            })
        })
        .await?;
    Ok(())
}

async fn provision_direct_negative_principal(
    storage: &super::SurrealStorage,
    principal_key: &str,
    capabilities: &[String],
) -> Result<super::resource_authority::ProvisionedPrincipal, ResourceAuthorityError> {
    storage
        .provision_principal(
            "direct-negative-account",
            principal_key,
            "human_account",
            principal_key,
            "Operator",
            capabilities,
            "direct-negative-space-a",
            Some("direct-negative-binding"),
            Duration::from_secs(300),
        )
        .await
}

fn direct_negative_scope(
    principal: &super::resource_authority::ProvisionedPrincipal,
    resource_id: impl Into<String>,
    capability_id: impl Into<String>,
    action: ResourceAction,
) -> RecordUserScope {
    RecordUserScope {
        session_token: principal.session.token.clone(),
        channel_binding_hash: Some("direct-negative-binding".to_owned()),
        resource_id: resource_id.into(),
        session_id: principal.session.session_id.clone(),
        capability_id: capability_id.into(),
        action,
    }
}

async fn privileged_operational_rows(
    storage: &super::SurrealStorage,
    statement: &'static str,
    bindings: OperationalProbeBindings,
) -> Result<Vec<Value>, Box<dyn std::error::Error>> {
    Ok(storage
        .with_data_operation(move |database| {
            Box::pin(async move { database.query_values::<Value, _>(statement, bindings).await })
        })
        .await?)
}

async fn seed_operational_sentinels(
    storage: &super::SurrealStorage,
    workspace_id: &str,
    authority_resource_id: &str,
    authority_session_id: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let bindings = OperationalProbeBindings {
        table: String::new(),
        record_id: String::new(),
        sentinel_id: OPERATIONAL_SENTINEL_ID.to_owned(),
        workspace: RecordId::new("workspaces", workspace_id),
        workspace_key: workspace_id.to_owned(),
        authority_resource: RecordId::new("protected_resources", authority_resource_id),
        authority_session: RecordId::new("authenticated_sessions", authority_session_id),
        hash: OPERATIONAL_HASH.to_owned(),
    };
    privileged_operational_rows(
        storage,
        "UPSERT type::record('kernel_event_ledger', $sentinel_id) CONTENT { event_id: $sentinel_id, event_version: 'v1', kernel_task_run_id: 'mt109-v12', session_run_id: 'mt109-v12', aggregate_type: 'native_editor', aggregate_id: $workspace_key, idempotency_key: $sentinel_id, event_type: 'MT109_V12_SENTINEL', actor_kind: 'SYSTEM', actor_id: 'mt109-v12', payload_hash: $hash, source_component: 'mt109-v12-test', payload: { marker: 'sentinel' }, wsids: [$workspace_key], authority_resource_id: $authority_resource, authority_session_id: $authority_session, authority_capability_id: 'fr.read', authority_action: 'read' } RETURN AFTER; \
         UPSERT type::record('fems_memory_packs', $sentinel_id) CONTENT { pack_id: $sentinel_id, workspace_id: $workspace, scope_key: 'mt109-v12', pack: { items: [] }, generated_at: time::now() } RETURN AFTER; \
         UPSERT type::record('fems_memory_proposals', $sentinel_id) CONTENT { proposal_id: $sentinel_id, request_id: $sentinel_id, workspace_id: $workspace, document_id: 'mt109-v12-document', selection_start: 0, selection_end: 1, content_hash: $hash, memory_class: 'episodic', status: 'pending_review', review_gated: true, proposal: { marker: 'sentinel' } } RETURN AFTER; \
         UPSERT type::record('fems_memory_items', $sentinel_id) CONTENT { memory_id: $sentinel_id, workspace_id: $workspace, item: { marker: 'sentinel' } } RETURN AFTER; \
         UPSERT type::record('fems_memory_commit_reports', $sentinel_id) CONTENT { commit_id: $sentinel_id, workspace_id: $workspace, proposal_id: type::record('fems_memory_proposals', $sentinel_id), memory_id: type::record('fems_memory_items', $sentinel_id), report: { marker: 'sentinel' }, report_hash: $hash, created_at: time::now() } RETURN AFTER; \
         UPSERT type::record('fems_memory_commit_fr_outbox', $sentinel_id) CONTENT { event_id: $sentinel_id, workspace_id: $workspace, proposal_id: type::record('fems_memory_proposals', $sentinel_id), commit_id: type::record('fems_memory_commit_reports', $sentinel_id), event_code: 'FR-EVT-MEM-003', event: { marker: 'sentinel' }, event_hash: $hash, created_at: time::now(), published_at: NONE, attempt_count: 0, last_error: NONE, last_error_at: NONE, quarantined_at: NONE } RETURN AFTER; \
         UPSERT type::record('fems_memory_lifecycle_fr_outbox', $sentinel_id) CONTENT { event_id: $sentinel_id, workspace_id: $workspace, proposal_id: type::record('fems_memory_proposals', $sentinel_id), event_code: 'FR-EVT-MEM-001', event: { marker: 'sentinel' }, event_hash: $hash, created_at: time::now(), published_at: NONE, attempt_count: 0, last_error: NONE, last_error_at: NONE, quarantined_at: NONE } RETURN AFTER; \
         UPSERT type::record('fems_workspace_write_anchors', $sentinel_id) CONTENT { anchor_key: $sentinel_id, workspace_key: $workspace_key, claim_nonce: 'mt109-v12-sentinel' } RETURN AFTER;",
        bindings,
    )
    .await?;
    Ok(())
}

async fn seed_operational_probe_dependencies(
    storage: &super::SurrealStorage,
    bindings: OperationalProbeBindings,
) -> Result<(), Box<dyn std::error::Error>> {
    privileged_operational_rows(
        storage,
        "UPSERT type::record('fems_memory_proposals', $record_id + '-proposal-dep') CONTENT { proposal_id: $record_id + '-proposal-dep', request_id: $record_id + '-proposal-dep', workspace_id: $workspace, document_id: 'mt109-v12-document', selection_start: 0, selection_end: 1, content_hash: $hash, memory_class: 'episodic', status: 'pending_review', review_gated: true, proposal: { marker: 'dependency' } } RETURN AFTER; \
         UPSERT type::record('fems_memory_items', $record_id + '-item-dep') CONTENT { memory_id: $record_id + '-item-dep', workspace_id: $workspace, item: { marker: 'dependency' } } RETURN AFTER; \
         UPSERT type::record('fems_memory_commit_reports', $record_id + '-report-dep') CONTENT { commit_id: $record_id + '-report-dep', workspace_id: $workspace, proposal_id: type::record('fems_memory_proposals', $record_id + '-proposal-dep'), memory_id: type::record('fems_memory_items', $record_id + '-item-dep'), report: { marker: 'dependency' }, report_hash: $hash, created_at: time::now() } RETURN AFTER;",
        bindings,
    )
    .await?;
    Ok(())
}

async fn assert_record_user_operation_has_zero_effect(
    storage: &super::SurrealStorage,
    scope: RecordUserScope,
    statement: &'static str,
    bindings: OperationalProbeBindings,
) -> Result<(), Box<dyn std::error::Error>> {
    let query_bindings = bindings.clone();
    let before_rows = privileged_operational_rows(
        storage,
        "SELECT * FROM type::table($table) ORDER BY id;",
        bindings.clone(),
    )
    .await?;
    let sentinel_before = privileged_operational_rows(
        storage,
        "SELECT * FROM type::record($table, $sentinel_id);",
        bindings.clone(),
    )
    .await?;
    assert_eq!(
        sentinel_before.len(),
        1,
        "{} denial probe has no privileged sentinel row",
        bindings.table
    );
    if bindings.record_id != bindings.sentinel_id {
        let candidate_before = privileged_operational_rows(
            storage,
            "SELECT * FROM type::record($table, $record_id);",
            bindings.clone(),
        )
        .await?;
        assert!(
            candidate_before.is_empty(),
            "{} denial candidate already exists",
            bindings.table
        );
    }
    let result = storage
        .with_record_user_scope(
            scope,
            storage.with_data_operation(move |database| {
                Box::pin(async move {
                    database
                        .query_values::<Value, _>(statement, query_bindings)
                        .await
                })
            }),
        )
        .await;
    match result {
        Ok(rows) => assert!(
            rows.is_empty(),
            "record-user operation changed or exposed a foreign row"
        ),
        Err(error) => {
            let error = error.to_string().to_ascii_lowercase();
            assert!(
                error.contains("permission")
                    || error.contains("not allowed")
                    || error.contains("auth")
                    || error.contains("denied"),
                "probe failed for a reason other than authorization: {error}"
            );
        }
    }
    let after_rows = privileged_operational_rows(
        storage,
        "SELECT * FROM type::table($table) ORDER BY id;",
        bindings.clone(),
    )
    .await?;
    let sentinel_after = privileged_operational_rows(
        storage,
        "SELECT * FROM type::record($table, $sentinel_id);",
        bindings.clone(),
    )
    .await?;
    assert_eq!(
        after_rows, before_rows,
        "{} denial changed privileged table state",
        bindings.table
    );
    assert_eq!(
        sentinel_after, sentinel_before,
        "{} denial changed or deleted its sentinel",
        bindings.table
    );
    if bindings.record_id != bindings.sentinel_id {
        let candidate_after = privileged_operational_rows(
            storage,
            "SELECT * FROM type::record($table, $record_id);",
            bindings.clone(),
        )
        .await?;
        assert!(
            candidate_after.is_empty(),
            "{} denial created candidate residue",
            bindings.table
        );
    }
    Ok(())
}

async fn assert_record_user_select_matches_privileged(
    storage: &super::SurrealStorage,
    scope: RecordUserScope,
    bindings: OperationalProbeBindings,
) -> Result<(), Box<dyn std::error::Error>> {
    let expected = privileged_operational_rows(
        storage,
        "SELECT * FROM type::record($table, $sentinel_id);",
        bindings.clone(),
    )
    .await?;
    assert_eq!(expected.len(), 1, "authorized probe sentinel is missing");
    let actual = storage
        .with_record_user_scope(
            scope,
            storage.with_data_operation(move |database| {
                Box::pin(async move {
                    database
                        .query_values::<Value, _>(
                            "SELECT * FROM type::record($table, $sentinel_id);",
                            bindings,
                        )
                        .await
                })
            }),
        )
        .await?;
    assert_eq!(actual, expected, "authorized record-user read drifted");
    Ok(())
}

async fn assert_record_user_update_matches_privileged(
    storage: &super::SurrealStorage,
    scope: RecordUserScope,
    statement: &'static str,
    bindings: OperationalProbeBindings,
) -> Result<(), Box<dyn std::error::Error>> {
    let readback_bindings = bindings.clone();
    let before = privileged_operational_rows(
        storage,
        "SELECT * FROM type::record($table, $sentinel_id);",
        bindings.clone(),
    )
    .await?;
    assert_eq!(before.len(), 1, "authorized update sentinel is missing");
    let returned = storage
        .with_record_user_scope(
            scope,
            storage.with_data_operation(move |database| {
                Box::pin(
                    async move { database.query_values::<Value, _>(statement, bindings).await },
                )
            }),
        )
        .await?;
    assert_eq!(returned.len(), 1, "authorized update returned no exact row");
    let after = privileged_operational_rows(
        storage,
        "SELECT * FROM type::record($table, $sentinel_id);",
        readback_bindings,
    )
    .await?;
    assert_ne!(after, before, "authorized update made no data change");
    assert_eq!(after, returned, "authorized update/readback diverged");
    Ok(())
}

async fn assert_record_user_create_matches_privileged(
    storage: &super::SurrealStorage,
    scope: RecordUserScope,
    statement: &'static str,
    bindings: OperationalProbeBindings,
) -> Result<(), Box<dyn std::error::Error>> {
    let readback_bindings = bindings.clone();
    let before = privileged_operational_rows(
        storage,
        "SELECT * FROM type::record($table, $record_id);",
        bindings.clone(),
    )
    .await?;
    assert!(
        before.is_empty(),
        "authorized create candidate already exists"
    );
    let returned = storage
        .with_record_user_scope(
            scope,
            storage.with_data_operation(move |database| {
                Box::pin(
                    async move { database.query_values::<Value, _>(statement, bindings).await },
                )
            }),
        )
        .await?;
    assert_eq!(returned.len(), 1, "authorized create returned no exact row");
    let after = privileged_operational_rows(
        storage,
        "SELECT * FROM type::record($table, $record_id);",
        readback_bindings,
    )
    .await?;
    assert_eq!(after, returned, "authorized create/readback diverged");
    Ok(())
}

#[tokio::test]
async fn production_resource_authority_schema_bootstraps_on_embedded_surrealdb_3_2(
) -> Result<(), Box<dyn std::error::Error>> {
    let backend = embedded_test_backend().await?;
    backend
        .storage
        .bootstrap_resource_authority_schema()
        .await?;
    Ok(())
}

fn request(
    token: &str,
    channel_binding_hash: Option<&str>,
    capability_id: &str,
    workspace_id: &str,
) -> AuthorizationRequest {
    AuthorizationRequest {
        session_token: token.to_owned(),
        channel_binding_hash: channel_binding_hash.map(str::to_owned),
        capability_id: capability_id.to_owned(),
        resource_kind: ResourceKind::FlightRecorder,
        external_resource_id: workspace_id.to_owned(),
        action: ResourceAction::Read,
    }
}

#[tokio::test]
async fn exact_capability_and_grant_are_both_required_and_revocation_is_immediate(
) -> Result<(), Box<dyn std::error::Error>> {
    let backend = embedded_test_backend().await?;
    let storage = &backend.storage;
    let owner = storage
        .provision_principal(
            "authority-test-account-a",
            "authority-test-principal-a",
            "human_account",
            "authority-test-actor-a",
            "Operator",
            &["fr.read".to_owned()],
            "authority-test-space-a",
            Some("binding-a"),
            Duration::from_secs(300),
        )
        .await?;
    let resource = storage
        .register_protected_resource(
            &owner.identity,
            ResourceKind::FlightRecorder,
            "authority-workspace-a",
            None,
            "account_private",
        )
        .await?;

    let no_grant = storage
        .authorize_protected_resource(request(
            &owner.session.token,
            Some("binding-a"),
            "fr.read",
            "authority-workspace-a",
        ))
        .await;
    assert!(matches!(
        no_grant,
        Err(ResourceAuthorityError::Denied { .. })
    ));

    let grant = storage
        .grant_resource(
            &owner.identity.account_id,
            &owner.identity.access_space_id,
            ResourceGrantSpec {
                principal_id: owner.identity.principal_id.clone(),
                resource_id: resource.resource_id,
                actions: vec![ResourceAction::Read],
                capability_ids: vec!["fr.read".to_owned()],
                expires_at: Some(owner.session.expires_at),
                delegation_chain: Vec::new(),
            },
        )
        .await?;
    let allowed = storage
        .authorize_protected_resource(request(
            &owner.session.token,
            Some("binding-a"),
            "fr.read",
            "authority-workspace-a",
        ))
        .await?;
    assert_eq!(allowed.account_id, owner.identity.account_id);
    assert_eq!(allowed.principal_id, owner.identity.principal_id);

    for denied in [
        request(
            &owner.session.token,
            Some("binding-forged"),
            "fr.read",
            "authority-workspace-a",
        ),
        request(
            &owner.session.token,
            Some("binding-a"),
            "memory.read",
            "authority-workspace-a",
        ),
        request(
            &owner.session.token,
            Some("binding-a"),
            "fr.read",
            "authority-workspace-forged",
        ),
    ] {
        assert!(matches!(
            storage.authorize_protected_resource(denied).await,
            Err(ResourceAuthorityError::Denied { .. })
        ));
    }

    storage.revoke_grant(&grant.grant_id).await?;
    assert!(matches!(
        storage
            .authorize_protected_resource(request(
                &owner.session.token,
                Some("binding-a"),
                "fr.read",
                "authority-workspace-a",
            ))
            .await,
        Err(ResourceAuthorityError::Denied { .. })
    ));
    Ok(())
}

#[tokio::test]
async fn cross_account_and_capability_without_grant_fail_closed(
) -> Result<(), Box<dyn std::error::Error>> {
    let backend = embedded_test_backend().await?;
    let storage = &backend.storage;
    let owner = storage
        .provision_principal(
            "authority-test-account-owner",
            "authority-test-principal-owner",
            "human_account",
            "authority-owner",
            "Operator",
            &["fr.read".to_owned()],
            "authority-test-space-owner",
            None,
            Duration::from_secs(300),
        )
        .await?;
    storage
        .register_protected_resource(
            &owner.identity,
            ResourceKind::FlightRecorder,
            "authority-cross-account-workspace",
            None,
            "account_private",
        )
        .await?;
    let outsider = storage
        .provision_principal(
            "authority-test-account-outsider",
            "authority-test-principal-outsider",
            "human_account",
            "authority-outsider",
            "Operator",
            &["fr.read".to_owned()],
            "authority-test-space-outsider",
            None,
            Duration::from_secs(300),
        )
        .await?;
    assert!(storage
        .issue_authenticated_session(
            &owner.identity.account_id,
            &owner.identity.principal_id,
            &outsider.identity.access_space_id,
            None,
            Duration::from_secs(300),
        )
        .await
        .is_err());
    assert!(matches!(
        storage
            .authorize_protected_resource(request(
                &outsider.session.token,
                None,
                "fr.read",
                "authority-cross-account-workspace",
            ))
            .await,
        Err(ResourceAuthorityError::Denied { .. })
    ));
    Ok(())
}

#[tokio::test]
async fn revoked_session_and_disabled_account_fail_closed() -> Result<(), Box<dyn std::error::Error>>
{
    let backend = embedded_test_backend().await?;
    let storage = &backend.storage;
    let principal = storage.provision_local_operator(None).await?;
    let resource = storage
        .register_protected_resource(
            &principal.identity,
            ResourceKind::FlightRecorder,
            "authority-revocation-workspace",
            None,
            "account_private",
        )
        .await?;
    storage
        .grant_resource(
            &principal.identity.account_id,
            &principal.identity.access_space_id,
            ResourceGrantSpec {
                principal_id: principal.identity.principal_id.clone(),
                resource_id: resource.resource_id,
                actions: vec![ResourceAction::Read],
                capability_ids: vec!["fr.read".to_owned()],
                expires_at: Some(principal.session.expires_at),
                delegation_chain: Vec::new(),
            },
        )
        .await?;
    storage
        .authorize_protected_resource(request(
            &principal.session.token,
            None,
            "fr.read",
            "authority-revocation-workspace",
        ))
        .await?;
    storage
        .revoke_session(&principal.session.session_id)
        .await?;
    assert!(matches!(
        storage
            .authorize_protected_resource(request(
                &principal.session.token,
                None,
                "fr.read",
                "authority-revocation-workspace",
            ))
            .await,
        Err(ResourceAuthorityError::Denied { .. })
    ));
    let replacement = storage
        .issue_authenticated_session(
            &principal.identity.account_id,
            &principal.identity.principal_id,
            &principal.identity.access_space_id,
            None,
            Duration::from_secs(300),
        )
        .await?;
    storage
        .authorize_protected_resource(request(
            &replacement.token,
            None,
            "fr.read",
            "authority-revocation-workspace",
        ))
        .await?;
    storage
        .disable_account(&principal.identity.account_id)
        .await?;
    assert!(matches!(
        storage
            .authorize_protected_resource(request(
                &replacement.token,
                None,
                "fr.read",
                "authority-revocation-workspace",
            ))
            .await,
        Err(ResourceAuthorityError::Denied { .. })
    ));
    Ok(())
}

#[tokio::test]
async fn expired_session_fails_closed() -> Result<(), Box<dyn std::error::Error>> {
    let backend = embedded_test_backend().await?;
    let storage = &backend.storage;
    let principal = storage
        .provision_principal(
            "authority-expiry-account",
            "authority-expiry-principal",
            "human_account",
            "authority-expiry-actor",
            "Operator",
            &["fr.read".to_owned()],
            "authority-expiry-space",
            None,
            Duration::from_millis(1),
        )
        .await?;
    let resource = storage
        .register_protected_resource(
            &principal.identity,
            ResourceKind::FlightRecorder,
            "authority-expiry-workspace",
            None,
            "account_private",
        )
        .await?;
    storage
        .grant_resource(
            &principal.identity.account_id,
            &principal.identity.access_space_id,
            ResourceGrantSpec {
                principal_id: principal.identity.principal_id.clone(),
                resource_id: resource.resource_id,
                actions: vec![ResourceAction::Read],
                capability_ids: vec!["fr.read".to_owned()],
                expires_at: Some(principal.session.expires_at),
                delegation_chain: Vec::new(),
            },
        )
        .await?;
    tokio::time::sleep(Duration::from_millis(10)).await;
    assert!(matches!(
        storage
            .authorize_protected_resource(request(
                &principal.session.token,
                None,
                "fr.read",
                "authority-expiry-workspace",
            ))
            .await,
        Err(ResourceAuthorityError::Denied { .. })
    ));
    Ok(())
}

#[derive(Clone, Copy)]
struct ResourceAuthorityCase {
    resource_kind: ResourceKind,
    action: ResourceAction,
    capability: &'static str,
}

const PROTECTED_RESOURCE_ACTIONS: &[ResourceAuthorityCase] = &[
    ResourceAuthorityCase {
        resource_kind: ResourceKind::FlightRecorder,
        action: ResourceAction::Read,
        capability: "fr.read",
    },
    ResourceAuthorityCase {
        resource_kind: ResourceKind::FlightRecorder,
        action: ResourceAction::Create,
        capability: "fr.ingest.runtime_chat",
    },
    ResourceAuthorityCase {
        resource_kind: ResourceKind::FlightRecorder,
        action: ResourceAction::Create,
        capability: "fr.ingest.native_editor",
    },
    ResourceAuthorityCase {
        resource_kind: ResourceKind::MemoryPack,
        action: ResourceAction::Read,
        capability: "memory.read",
    },
    ResourceAuthorityCase {
        resource_kind: ResourceKind::MemoryProposal,
        action: ResourceAction::Read,
        capability: "memory.read",
    },
    ResourceAuthorityCase {
        resource_kind: ResourceKind::MemoryProposal,
        action: ResourceAction::Create,
        capability: "memory.propose",
    },
    ResourceAuthorityCase {
        resource_kind: ResourceKind::MemoryProposal,
        action: ResourceAction::Update,
        capability: "memory.review",
    },
    ResourceAuthorityCase {
        resource_kind: ResourceKind::MemoryProposal,
        action: ResourceAction::Update,
        capability: "memory.commit",
    },
    ResourceAuthorityCase {
        resource_kind: ResourceKind::MemoryPack,
        action: ResourceAction::Create,
        capability: "memory.commit",
    },
    ResourceAuthorityCase {
        resource_kind: ResourceKind::MemoryItem,
        action: ResourceAction::Create,
        capability: "memory.commit",
    },
    ResourceAuthorityCase {
        resource_kind: ResourceKind::MemoryCommitReport,
        action: ResourceAction::Create,
        capability: "memory.commit",
    },
    ResourceAuthorityCase {
        resource_kind: ResourceKind::MemoryCommitReport,
        action: ResourceAction::Read,
        capability: "memory.read",
    },
    ResourceAuthorityCase {
        resource_kind: ResourceKind::MemoryItemCount,
        action: ResourceAction::Read,
        capability: "memory.read",
    },
];

fn matrix_request(
    session_token: &str,
    binding: &str,
    case: ResourceAuthorityCase,
    workspace_id: &str,
) -> AuthorizationRequest {
    AuthorizationRequest {
        session_token: session_token.to_owned(),
        channel_binding_hash: Some(binding.to_owned()),
        capability_id: case.capability.to_owned(),
        resource_kind: case.resource_kind,
        external_resource_id: workspace_id.to_owned(),
        action: case.action,
    }
}

async fn grant_every_route(
    storage: &super::SurrealStorage,
    principal: &super::resource_authority::ProvisionedPrincipal,
    workspace_id: &str,
) -> Result<(), ResourceAuthorityError> {
    let workspace = storage
        .register_workspace_resource(&principal.identity, workspace_id)
        .await?;
    let flight_recorder = storage
        .register_protected_resource(
            &principal.identity,
            ResourceKind::FlightRecorder,
            workspace_id,
            None,
            "account_private",
        )
        .await?;
    let memory_pack = storage
        .register_protected_resource(
            &principal.identity,
            ResourceKind::MemoryPack,
            workspace_id,
            None,
            "account_private",
        )
        .await?;
    let memory_proposal = storage
        .register_protected_resource(
            &principal.identity,
            ResourceKind::MemoryProposal,
            workspace_id,
            None,
            "account_private",
        )
        .await?;
    let memory_report = storage
        .register_protected_resource(
            &principal.identity,
            ResourceKind::MemoryCommitReport,
            workspace_id,
            None,
            "account_private",
        )
        .await?;
    let memory_item = storage
        .register_protected_resource(
            &principal.identity,
            ResourceKind::MemoryItem,
            workspace_id,
            None,
            "account_private",
        )
        .await?;
    let memory_item_count = storage
        .register_protected_resource(
            &principal.identity,
            ResourceKind::MemoryItemCount,
            workspace_id,
            None,
            "account_private",
        )
        .await?;
    storage
        .grant_resource(
            &principal.identity.account_id,
            &principal.identity.access_space_id,
            ResourceGrantSpec {
                principal_id: principal.identity.principal_id.clone(),
                resource_id: workspace.resource_id,
                actions: vec![
                    ResourceAction::Read,
                    ResourceAction::Create,
                    ResourceAction::Update,
                ],
                capability_ids: WORKSPACE_RESOURCE_CAPABILITIES
                    .iter()
                    .map(|capability| (*capability).to_owned())
                    .collect(),
                expires_at: Some(principal.session.expires_at),
                delegation_chain: Vec::new(),
            },
        )
        .await?;
    for case in PROTECTED_RESOURCE_ACTIONS {
        let resource_id = match case.resource_kind {
            ResourceKind::FlightRecorder => &flight_recorder.resource_id,
            ResourceKind::MemoryPack => &memory_pack.resource_id,
            ResourceKind::MemoryProposal => &memory_proposal.resource_id,
            ResourceKind::MemoryCommitReport => &memory_report.resource_id,
            ResourceKind::MemoryItem => &memory_item.resource_id,
            ResourceKind::MemoryItemCount => &memory_item_count.resource_id,
            ResourceKind::Workspace | ResourceKind::ReconciliationQueue => {
                unreachable!("not an ordinary route resource")
            }
        };
        storage
            .grant_resource(
                &principal.identity.account_id,
                &principal.identity.access_space_id,
                ResourceGrantSpec {
                    principal_id: principal.identity.principal_id.clone(),
                    resource_id: resource_id.clone(),
                    actions: vec![case.action],
                    capability_ids: vec![case.capability.to_owned()],
                    expires_at: Some(principal.session.expires_at),
                    delegation_chain: Vec::new(),
                },
            )
            .await?;
    }
    Ok(())
}

#[tokio::test]
async fn two_account_two_space_resource_broker_and_record_user_boundary_fail_closed(
) -> Result<(), Box<dyn std::error::Error>> {
    let backend = embedded_test_backend().await?;
    let storage = &backend.storage;
    let database = SurrealDatabase::new(storage.clone());
    let workspace_a = database
        .create_workspace(
            &WriteContext::human(Some("matrix-a".to_owned())),
            NewWorkspace {
                name: "MT-109 matrix account A".to_owned(),
            },
        )
        .await?;
    let workspace_b = database
        .create_workspace(
            &WriteContext::human(Some("matrix-b".to_owned())),
            NewWorkspace {
                name: "MT-109 matrix account B".to_owned(),
            },
        )
        .await?;
    let capabilities = [
        "fr.read",
        "fr.ingest.runtime_chat",
        "fr.ingest.native_editor",
        "memory.read",
        "memory.propose",
        "memory.review",
        "memory.commit",
    ]
    .map(str::to_owned);
    let account_a = storage
        .provision_principal(
            "matrix-account-a",
            "matrix-principal-a",
            "human_account",
            "matrix-actor-a",
            "Operator",
            &capabilities,
            "matrix-space-a",
            Some("matrix-binding-a"),
            Duration::from_secs(300),
        )
        .await?;
    let account_b = storage
        .provision_principal(
            "matrix-account-b",
            "matrix-principal-b",
            "human_account",
            "matrix-actor-b",
            "Operator",
            &capabilities,
            "matrix-space-b",
            Some("matrix-binding-b"),
            Duration::from_secs(300),
        )
        .await?;
    grant_every_route(storage, &account_a, &workspace_a.id).await?;
    grant_every_route(storage, &account_b, &workspace_b.id).await?;

    for case in PROTECTED_RESOURCE_ACTIONS {
        let allowed = storage
            .authorize_protected_resource(matrix_request(
                &account_a.session.token,
                "matrix-binding-a",
                *case,
                &workspace_a.id,
            ))
            .await
            .unwrap_or_else(|error| panic!("resource action should be authorized: {error}"));
        assert_eq!(
            allowed.account_id, account_a.identity.account_id,
            "resource action account attribution"
        );
        assert!(
            matches!(
                storage
                    .authorize_protected_resource(matrix_request(
                        &account_a.session.token,
                        "matrix-binding-a",
                        *case,
                        &workspace_b.id
                    ))
                    .await,
                Err(ResourceAuthorityError::Denied { .. })
            ),
            "forged cross-account selector"
        );
        assert!(
            matches!(
                storage
                    .authorize_protected_resource(matrix_request(
                        &account_b.session.token,
                        "matrix-binding-b",
                        *case,
                        &workspace_a.id
                    ))
                    .await,
                Err(ResourceAuthorityError::Denied { .. })
            ),
            "cross-account session"
        );
        let mut wrong_capability = matrix_request(
            &account_a.session.token,
            "matrix-binding-a",
            *case,
            &workspace_a.id,
        );
        wrong_capability.capability_id = "forged.capability".to_owned();
        assert!(
            matches!(
                storage.authorize_protected_resource(wrong_capability).await,
                Err(ResourceAuthorityError::Denied { .. })
            ),
            "forged capability"
        );
        let mut wrong_action = matrix_request(
            &account_a.session.token,
            "matrix-binding-a",
            *case,
            &workspace_a.id,
        );
        wrong_action.action = if case.action == ResourceAction::Read {
            ResourceAction::Create
        } else {
            ResourceAction::Read
        };
        assert!(
            matches!(
                storage.authorize_protected_resource(wrong_action).await,
                Err(ResourceAuthorityError::Denied { .. })
            ),
            "capability/grant action asymmetry"
        );
    }

    let decision = storage
        .authorize_protected_resource(matrix_request(
            &account_a.session.token,
            "matrix-binding-a",
            PROTECTED_RESOURCE_ACTIONS[0],
            &workspace_a.id,
        ))
        .await?;
    let visible = storage
        .with_record_user_scope(
            RecordUserScope {
                session_token: account_a.session.token.clone(),
                channel_binding_hash: Some("matrix-binding-a".to_owned()),
                resource_id: decision.resource_id,
                session_id: decision.session_id,
                capability_id: "fr.read".to_owned(),
                action: ResourceAction::Read,
            },
            database.list_workspaces(),
        )
        .await?;
    assert!(visible
        .iter()
        .any(|workspace| workspace.id == workspace_a.id));
    assert!(
        !visible
            .iter()
            .any(|workspace| workspace.id == workspace_b.id),
        "record-user direct enumeration exposed another account"
    );
    Ok(())
}

#[tokio::test]
async fn direct_record_user_foreign_table_operations_are_default_deny(
) -> Result<(), Box<dyn std::error::Error>> {
    let backend = embedded_test_backend().await?;
    let storage = &backend.storage;
    let database = SurrealDatabase::new(storage.clone());
    let own_workspace = database
        .create_workspace(
            &WriteContext::human(Some("direct-record-user-own".to_owned())),
            NewWorkspace {
                name: "MT-109 direct own workspace".to_owned(),
            },
        )
        .await?;
    let foreign_workspace = database
        .create_workspace(
            &WriteContext::human(Some("direct-record-user-foreign".to_owned())),
            NewWorkspace {
                name: "MT-109 direct foreign workspace".to_owned(),
            },
        )
        .await?;
    let capabilities = [
        "fr.read",
        "fr.ingest.native_editor",
        "memory.read",
        "memory.propose",
        "memory.review",
        "memory.commit",
    ]
    .map(str::to_owned);
    let principal = storage
        .provision_principal(
            "direct-record-user-account",
            "direct-record-user-principal",
            "human_account",
            "direct-record-user-actor",
            "Operator",
            &capabilities,
            "direct-record-user-space",
            Some("direct-record-user-binding"),
            Duration::from_secs(300),
        )
        .await?;
    grant_every_route(storage, &principal, &own_workspace.id).await?;
    let decision = storage
        .authorize_protected_resource(AuthorizationRequest {
            session_token: principal.session.token.clone(),
            channel_binding_hash: Some("direct-record-user-binding".to_owned()),
            capability_id: "memory.read".to_owned(),
            resource_kind: ResourceKind::MemoryProposal,
            external_resource_id: own_workspace.id.clone(),
            action: ResourceAction::Read,
        })
        .await?;
    let scope = RecordUserScope {
        session_token: principal.session.token.clone(),
        channel_binding_hash: Some("direct-record-user-binding".to_owned()),
        resource_id: decision.resource_id,
        session_id: decision.session_id,
        capability_id: "memory.read".to_owned(),
        action: ResourceAction::Read,
    };
    let foreign_resource = storage
        .register_protected_resource(
            &principal.identity,
            ResourceKind::FlightRecorder,
            &foreign_workspace.id,
            None,
            "account_private",
        )
        .await?;
    seed_operational_sentinels(
        storage,
        &foreign_workspace.id,
        &foreign_resource.resource_id,
        &principal.session.session_id,
    )
    .await?;
    let workspace = RecordId::new("workspaces", foreign_workspace.id.as_str());

    assert_record_user_operation_has_zero_effect(
        storage,
        scope.clone(),
        "SELECT * FROM workspaces WHERE id = $workspace;",
        OperationalProbeBindings {
            table: "workspaces".to_owned(),
            record_id: "foreign-workspace-probe".to_owned(),
            sentinel_id: foreign_workspace.id.clone(),
            workspace: workspace.clone(),
            workspace_key: foreign_workspace.id.clone(),
            authority_resource: RecordId::new(
                "protected_resources",
                foreign_resource.resource_id.clone(),
            ),
            authority_session: RecordId::new(
                "authenticated_sessions",
                principal.session.session_id.clone(),
            ),
            hash: OPERATIONAL_HASH.to_owned(),
        },
    )
    .await?;
    for statement in [
        "CREATE type::record('workspaces', $record_id) SET name = 'foreign create probe' RETURN AFTER;",
        "UPSERT type::record('workspaces', $record_id) SET name = 'foreign upsert probe' RETURN AFTER;",
        "UPDATE $workspace SET name = 'foreign update probe' RETURN AFTER;",
        "DELETE $workspace RETURN BEFORE;",
    ] {
        assert_record_user_operation_has_zero_effect(
            storage,
            scope.clone(),
            statement,
            OperationalProbeBindings {
                table: "workspaces".to_owned(),
                record_id: "mt109-foreign-workspace-write".to_owned(),
                sentinel_id: foreign_workspace.id.clone(),
                workspace: workspace.clone(),
                workspace_key: foreign_workspace.id.clone(),
                authority_resource: RecordId::new(
                    "protected_resources",
                    foreign_resource.resource_id.clone(),
                ),
                authority_session: RecordId::new(
                    "authenticated_sessions",
                    principal.session.session_id.clone(),
                ),
                hash: OPERATIONAL_HASH.to_owned(),
            },
        )
        .await?;
    }

    for (table, statements) in [
        (
            "kernel_event_ledger",
            [
                "SELECT * FROM kernel_event_ledger WHERE array::includes(wsids, $workspace_key);",
                "CREATE type::record($table, $record_id) CONTENT { event_id: $record_id, event_version: 'v1', kernel_task_run_id: 'mt109-v12', session_run_id: 'mt109-v12', aggregate_type: 'native_editor', aggregate_id: $workspace_key, idempotency_key: $record_id, event_type: 'MT109_V12_DENIED', actor_kind: 'SYSTEM', actor_id: 'mt109-v12', payload_hash: $hash, source_component: 'mt109-v12-test', payload: { marker: 'denied' }, wsids: [$workspace_key], authority_resource_id: $authority_resource, authority_session_id: $authority_session, authority_capability_id: 'fr.ingest.native_editor', authority_action: 'create' } RETURN AFTER;",
                "UPSERT type::record($table, $record_id) CONTENT { event_id: $record_id, event_version: 'v1', kernel_task_run_id: 'mt109-v12', session_run_id: 'mt109-v12', aggregate_type: 'native_editor', aggregate_id: $workspace_key, idempotency_key: $record_id, event_type: 'MT109_V12_DENIED', actor_kind: 'SYSTEM', actor_id: 'mt109-v12', payload_hash: $hash, source_component: 'mt109-v12-test', payload: { marker: 'denied' }, wsids: [$workspace_key], authority_resource_id: $authority_resource, authority_session_id: $authority_session, authority_capability_id: 'fr.ingest.native_editor', authority_action: 'create' } RETURN AFTER;",
                "UPDATE kernel_event_ledger SET payload = { marker: 'denied-update' } WHERE array::includes(wsids, $workspace_key) RETURN AFTER;",
                "DELETE kernel_event_ledger WHERE array::includes(wsids, $workspace_key) RETURN BEFORE;",
            ],
        ),
        (
            "fems_memory_packs",
            [
                "SELECT * FROM fems_memory_packs WHERE workspace_id = $workspace;",
                "CREATE type::record($table, $record_id) CONTENT { pack_id: $record_id, workspace_id: $workspace, scope_key: $record_id, pack: { marker: 'denied-create' }, generated_at: time::now() } RETURN AFTER;",
                "UPSERT type::record($table, $record_id) CONTENT { pack_id: $record_id, workspace_id: $workspace, scope_key: $record_id, pack: { marker: 'denied-upsert-new' }, generated_at: time::now() } RETURN AFTER;",
                "UPDATE fems_memory_packs SET workspace_id = $workspace WHERE workspace_id = $workspace RETURN AFTER;",
                "DELETE fems_memory_packs WHERE workspace_id = $workspace RETURN BEFORE;",
            ],
        ),
        (
            "fems_memory_proposals",
            [
                "SELECT * FROM fems_memory_proposals WHERE workspace_id = $workspace;",
                "CREATE type::record($table, $record_id) CONTENT { proposal_id: $record_id, request_id: $record_id, workspace_id: $workspace, document_id: 'mt109-v12-document', selection_start: 0, selection_end: 1, content_hash: $hash, memory_class: 'episodic', status: 'pending_review', review_gated: true, proposal: { marker: 'denied-create' } } RETURN AFTER;",
                "UPSERT type::record($table, $record_id) CONTENT { proposal_id: $record_id, request_id: $record_id, workspace_id: $workspace, document_id: 'mt109-v12-document', selection_start: 0, selection_end: 1, content_hash: $hash, memory_class: 'episodic', status: 'pending_review', review_gated: true, proposal: { marker: 'denied-upsert-new' } } RETURN AFTER;",
                "UPDATE fems_memory_proposals SET workspace_id = $workspace WHERE workspace_id = $workspace RETURN AFTER;",
                "DELETE fems_memory_proposals WHERE workspace_id = $workspace RETURN BEFORE;",
            ],
        ),
        (
            "fems_memory_items",
            [
                "SELECT * FROM fems_memory_items WHERE workspace_id = $workspace;",
                "CREATE type::record($table, $record_id) CONTENT { memory_id: $record_id, workspace_id: $workspace, item: { marker: 'denied-create' } } RETURN AFTER;",
                "UPSERT type::record($table, $record_id) CONTENT { memory_id: $record_id, workspace_id: $workspace, item: { marker: 'denied-upsert-new' } } RETURN AFTER;",
                "UPDATE fems_memory_items SET workspace_id = $workspace WHERE workspace_id = $workspace RETURN AFTER;",
                "DELETE fems_memory_items WHERE workspace_id = $workspace RETURN BEFORE;",
            ],
        ),
        (
            "fems_memory_commit_reports",
            [
                "SELECT * FROM fems_memory_commit_reports WHERE workspace_id = $workspace;",
                "CREATE type::record($table, $record_id) CONTENT { commit_id: $record_id, workspace_id: $workspace, proposal_id: type::record('fems_memory_proposals', $record_id + '-proposal-dep'), memory_id: type::record('fems_memory_items', $record_id + '-item-dep'), report: { marker: 'denied-create' }, report_hash: $hash, created_at: time::now() } RETURN AFTER;",
                "UPSERT type::record($table, $record_id) CONTENT { commit_id: $record_id, workspace_id: $workspace, proposal_id: type::record('fems_memory_proposals', $record_id + '-proposal-dep'), memory_id: type::record('fems_memory_items', $record_id + '-item-dep'), report: { marker: 'denied-upsert-new' }, report_hash: $hash, created_at: time::now() } RETURN AFTER;",
                "UPDATE fems_memory_commit_reports SET workspace_id = $workspace WHERE workspace_id = $workspace RETURN AFTER;",
                "DELETE fems_memory_commit_reports WHERE workspace_id = $workspace RETURN BEFORE;",
            ],
        ),
        (
            "fems_memory_commit_fr_outbox",
            [
                "SELECT * FROM fems_memory_commit_fr_outbox WHERE workspace_id = $workspace;",
                "CREATE type::record($table, $record_id) CONTENT { event_id: $record_id, workspace_id: $workspace, proposal_id: type::record('fems_memory_proposals', $record_id + '-proposal-dep'), commit_id: type::record('fems_memory_commit_reports', $record_id + '-report-dep'), event_code: 'FR-EVT-MEM-003', event: { marker: 'denied-create' }, event_hash: $hash, created_at: time::now(), published_at: NONE, attempt_count: 0, last_error: NONE, last_error_at: NONE, quarantined_at: NONE } RETURN AFTER;",
                "UPSERT type::record($table, $record_id) CONTENT { event_id: $record_id, workspace_id: $workspace, proposal_id: type::record('fems_memory_proposals', $record_id + '-proposal-dep'), commit_id: type::record('fems_memory_commit_reports', $record_id + '-report-dep'), event_code: 'FR-EVT-MEM-003', event: { marker: 'denied-upsert-new' }, event_hash: $hash, created_at: time::now(), published_at: NONE, attempt_count: 0, last_error: NONE, last_error_at: NONE, quarantined_at: NONE } RETURN AFTER;",
                "UPDATE fems_memory_commit_fr_outbox SET workspace_id = $workspace WHERE workspace_id = $workspace RETURN AFTER;",
                "DELETE fems_memory_commit_fr_outbox WHERE workspace_id = $workspace RETURN BEFORE;",
            ],
        ),
        (
            "fems_memory_lifecycle_fr_outbox",
            [
                "SELECT * FROM fems_memory_lifecycle_fr_outbox WHERE workspace_id = $workspace;",
                "CREATE type::record($table, $record_id) CONTENT { event_id: $record_id, workspace_id: $workspace, proposal_id: type::record('fems_memory_proposals', $record_id + '-proposal-dep'), event_code: 'FR-EVT-MEM-001', event: { marker: 'denied-create' }, event_hash: $hash, created_at: time::now(), published_at: NONE, attempt_count: 0, last_error: NONE, last_error_at: NONE, quarantined_at: NONE } RETURN AFTER;",
                "UPSERT type::record($table, $record_id) CONTENT { event_id: $record_id, workspace_id: $workspace, proposal_id: type::record('fems_memory_proposals', $record_id + '-proposal-dep'), event_code: 'FR-EVT-MEM-001', event: { marker: 'denied-upsert-new' }, event_hash: $hash, created_at: time::now(), published_at: NONE, attempt_count: 0, last_error: NONE, last_error_at: NONE, quarantined_at: NONE } RETURN AFTER;",
                "UPDATE fems_memory_lifecycle_fr_outbox SET workspace_id = $workspace WHERE workspace_id = $workspace RETURN AFTER;",
                "DELETE fems_memory_lifecycle_fr_outbox WHERE workspace_id = $workspace RETURN BEFORE;",
            ],
        ),
        (
            "fems_workspace_write_anchors",
            [
                "SELECT * FROM fems_workspace_write_anchors WHERE workspace_key = $workspace_key;",
                "CREATE type::record($table, $record_id) CONTENT { anchor_key: $record_id, workspace_key: $workspace_key, claim_nonce: 'denied-create' } RETURN AFTER;",
                "UPSERT type::record($table, $record_id) CONTENT { anchor_key: $record_id, workspace_key: $workspace_key, claim_nonce: 'denied-upsert-new' } RETURN AFTER;",
                "UPDATE fems_workspace_write_anchors SET workspace_key = $workspace_key WHERE workspace_key = $workspace_key RETURN AFTER;",
                "DELETE fems_workspace_write_anchors WHERE workspace_key = $workspace_key RETURN BEFORE;",
            ],
        ),
    ] {
        let bindings = OperationalProbeBindings {
            table: table.to_owned(),
            record_id: format!("mt109-foreign-{}", table.replace('_', "-")),
            sentinel_id: OPERATIONAL_SENTINEL_ID.to_owned(),
            workspace: workspace.clone(),
            workspace_key: foreign_workspace.id.clone(),
            authority_resource: RecordId::new(
                "protected_resources",
                foreign_resource.resource_id.clone(),
            ),
            authority_session: RecordId::new(
                "authenticated_sessions",
                principal.session.session_id.clone(),
            ),
            hash: OPERATIONAL_HASH.to_owned(),
        };
        seed_operational_probe_dependencies(storage, bindings.clone()).await?;
        for statement in statements {
            assert_record_user_operation_has_zero_effect(
                storage,
                scope.clone(),
                statement,
                bindings.clone(),
            )
            .await?;
        }
    }
    Ok(())
}

#[tokio::test]
async fn direct_record_user_negative_scope_bulk_outbox_and_recovery_matrix_is_default_deny(
) -> Result<(), Box<dyn std::error::Error>> {
    let backend = embedded_test_backend().await?;
    let storage = &backend.storage;
    let database = SurrealDatabase::new(storage.clone());
    let workspace = database
        .create_workspace(
            &WriteContext::human(Some("direct-negative-matrix".to_owned())),
            NewWorkspace {
                name: "MT-109 direct negative matrix".to_owned(),
            },
        )
        .await?;
    let capabilities = [
        "fr.read",
        "fr.ingest.runtime_chat",
        "fr.ingest.native_editor",
        "memory.read",
        "memory.propose",
        "memory.review",
        "memory.commit",
    ]
    .map(str::to_owned);
    let owner = storage
        .provision_principal(
            "direct-negative-account",
            "direct-negative-owner",
            "human_account",
            "direct-negative-owner",
            "Operator",
            &capabilities,
            "direct-negative-space-a",
            Some("direct-negative-binding"),
            Duration::from_secs(300),
        )
        .await?;
    grant_every_route(storage, &owner, &workspace.id).await?;
    let sentinel_decision = storage
        .authorize_protected_resource(request(
            &owner.session.token,
            Some("direct-negative-binding"),
            "fr.read",
            &workspace.id,
        ))
        .await?;
    seed_operational_sentinels(
        storage,
        &workspace.id,
        &sentinel_decision.resource_id,
        &sentinel_decision.session_id,
    )
    .await?;
    let member_without_grant =
        provision_direct_negative_principal(storage, "direct-negative-member", &capabilities)
            .await?;
    assert_eq!(
        member_without_grant.identity.account_id,
        owner.identity.account_id
    );
    assert_eq!(
        member_without_grant.identity.access_space_id,
        owner.identity.access_space_id
    );
    let capability_without_grant = provision_direct_negative_principal(
        storage,
        "direct-negative-capability-without-grant",
        &capabilities,
    )
    .await?;
    let irrelevant_resource_kind = provision_direct_negative_principal(
        storage,
        "direct-negative-irrelevant-resource-kind",
        &capabilities,
    )
    .await?;
    storage
        .grant_resource(
            &irrelevant_resource_kind.identity.account_id,
            &irrelevant_resource_kind.identity.access_space_id,
            ResourceGrantSpec {
                principal_id: irrelevant_resource_kind.identity.principal_id.clone(),
                resource_id: sentinel_decision.resource_id.clone(),
                actions: vec![ResourceAction::Read],
                capability_ids: vec!["fr.read".to_owned()],
                expires_at: Some(irrelevant_resource_kind.session.expires_at),
                delegation_chain: Vec::new(),
            },
        )
        .await?;
    let grant_without_delegated_capability = provision_direct_negative_principal(
        storage,
        "direct-negative-grant-without-delegated-capability",
        &[],
    )
    .await?;
    grant_existing_route_resources_to(
        storage,
        &owner,
        &grant_without_delegated_capability,
        &workspace.id,
        None,
        None,
        None,
    )
    .await?;
    let forged_capability = provision_direct_negative_principal(
        storage,
        "direct-negative-forged-capability",
        &["forged.capability".to_owned()],
    )
    .await?;
    grant_existing_route_resources_to(
        storage,
        &owner,
        &forged_capability,
        &workspace.id,
        None,
        None,
        None,
    )
    .await?;
    let wrong_action =
        provision_direct_negative_principal(storage, "direct-negative-wrong-action", &capabilities)
            .await?;
    grant_existing_route_resources_to(
        storage,
        &owner,
        &wrong_action,
        &workspace.id,
        Some(ResourceAction::Delete),
        None,
        None,
    )
    .await?;
    let missing_action = provision_direct_negative_principal(
        storage,
        "direct-negative-missing-action",
        &capabilities,
    )
    .await?;
    grant_existing_route_resources_to(
        storage,
        &owner,
        &missing_action,
        &workspace.id,
        None,
        None,
        None,
    )
    .await?;
    clear_principal_grant_field(storage, &missing_action.identity.principal_id, "actions").await?;
    let wrong_grant_capability = provision_direct_negative_principal(
        storage,
        "direct-negative-wrong-grant-capability",
        &capabilities,
    )
    .await?;
    grant_existing_route_resources_to(
        storage,
        &owner,
        &wrong_grant_capability,
        &workspace.id,
        None,
        Some("forged.capability"),
        None,
    )
    .await?;
    let missing_grant_capability = provision_direct_negative_principal(
        storage,
        "direct-negative-missing-grant-capability",
        &capabilities,
    )
    .await?;
    grant_existing_route_resources_to(
        storage,
        &owner,
        &missing_grant_capability,
        &workspace.id,
        None,
        None,
        None,
    )
    .await?;
    clear_principal_grant_field(
        storage,
        &missing_grant_capability.identity.principal_id,
        "capability_ids",
    )
    .await?;
    let mismatched_delegation = provision_direct_negative_principal(
        storage,
        "direct-negative-mismatched-delegation",
        &capabilities,
    )
    .await?;
    let mismatched_chain = ["not-the-session-principal".to_owned()];
    grant_existing_route_resources_to(
        storage,
        &owner,
        &mismatched_delegation,
        &workspace.id,
        None,
        None,
        Some(&mismatched_chain),
    )
    .await?;
    let revoked_grant = provision_direct_negative_principal(
        storage,
        "direct-negative-revoked-grant",
        &capabilities,
    )
    .await?;
    let revoked_grants = grant_existing_route_resources_to(
        storage,
        &owner,
        &revoked_grant,
        &workspace.id,
        None,
        None,
        None,
    )
    .await?;
    for grant in revoked_grants {
        storage.revoke_grant(&grant.grant_id).await?;
    }
    let wrong_space = storage
        .provision_principal(
            "direct-negative-account",
            "direct-negative-owner",
            "human_account",
            "direct-negative-owner",
            "Operator",
            &capabilities,
            "direct-negative-space-b",
            Some("direct-negative-binding"),
            Duration::from_secs(300),
        )
        .await?;
    assert_eq!(wrong_space.identity.account_id, owner.identity.account_id);
    assert_ne!(
        wrong_space.identity.access_space_id,
        owner.identity.access_space_id
    );
    grant_existing_route_resources_to(
        storage,
        &owner,
        &wrong_space,
        &workspace.id,
        None,
        None,
        None,
    )
    .await?;
    let wrong_resource_workspace = database
        .create_workspace(
            &WriteContext::human(Some("direct-negative-wrong-resource".to_owned())),
            NewWorkspace {
                name: "MT-109 wrong resource".to_owned(),
            },
        )
        .await?;
    let wrong_resource = storage
        .provision_principal(
            "direct-negative-account",
            "direct-negative-wrong-resource",
            "human_account",
            "direct-negative-wrong-resource",
            "Operator",
            &capabilities,
            "direct-negative-space-a",
            Some("direct-negative-binding"),
            Duration::from_secs(300),
        )
        .await?;
    grant_every_route(storage, &wrong_resource, &wrong_resource_workspace.id).await?;
    let missing_resource = provision_direct_negative_principal(
        storage,
        "direct-negative-missing-resource",
        &capabilities,
    )
    .await?;
    grant_missing_resource_relation_to(storage, &missing_resource).await?;
    let stale_space = storage
        .provision_principal(
            "direct-negative-account",
            "direct-negative-stale-space",
            "human_account",
            "direct-negative-stale-space",
            "Operator",
            &capabilities,
            "direct-negative-space-a",
            Some("direct-negative-binding"),
            Duration::from_secs(300),
        )
        .await?;
    grant_every_route(storage, &stale_space, &workspace.id).await?;
    storage
        .switch_session_access_space(
            &stale_space.session.session_id,
            &wrong_space.identity.access_space_id,
        )
        .await?;
    let foreign = storage
        .provision_principal(
            "direct-negative-foreign-account",
            "direct-negative-foreign-principal",
            "human_account",
            "direct-negative-foreign-principal",
            "Operator",
            &capabilities,
            "direct-negative-foreign-space",
            Some("direct-negative-binding"),
            Duration::from_secs(300),
        )
        .await?;
    grant_existing_route_resources_to(storage, &owner, &foreign, &workspace.id, None, None, None)
        .await?;
    let revoked = storage
        .provision_principal(
            "direct-negative-account",
            "direct-negative-revoked",
            "human_account",
            "direct-negative-revoked",
            "Operator",
            &capabilities,
            "direct-negative-space-a",
            Some("direct-negative-binding"),
            Duration::from_secs(300),
        )
        .await?;
    grant_every_route(storage, &revoked, &workspace.id).await?;
    storage.revoke_session(&revoked.session.session_id).await?;
    let expired = storage
        .provision_principal(
            "direct-negative-account",
            "direct-negative-expired",
            "human_account",
            "direct-negative-expired",
            "Operator",
            &capabilities,
            "direct-negative-space-a",
            Some("direct-negative-binding"),
            Duration::from_millis(1),
        )
        .await?;
    grant_every_route(storage, &expired, &workspace.id).await?;
    tokio::time::sleep(Duration::from_millis(5)).await;

    let valid_resource_id = sentinel_decision.resource_id.clone();
    let mut denied_scopes = vec![
        DeniedOperationalCase {
            label: "same-account-member-without-grant",
            scope: direct_negative_scope(
                &member_without_grant,
                valid_resource_id.clone(),
                "fr.read",
                ResourceAction::Read,
            ),
        },
        DeniedOperationalCase {
            label: "capability-without-grant",
            scope: direct_negative_scope(
                &capability_without_grant,
                valid_resource_id.clone(),
                "fr.read",
                ResourceAction::Read,
            ),
        },
        DeniedOperationalCase {
            label: "irrelevant-resource-kind",
            scope: direct_negative_scope(
                &irrelevant_resource_kind,
                valid_resource_id.clone(),
                "fr.read",
                ResourceAction::Read,
            ),
        },
        DeniedOperationalCase {
            label: "grant-without-delegated-capability",
            scope: direct_negative_scope(
                &grant_without_delegated_capability,
                valid_resource_id.clone(),
                "fr.read",
                ResourceAction::Read,
            ),
        },
        DeniedOperationalCase {
            label: "forged-capability",
            scope: direct_negative_scope(
                &forged_capability,
                valid_resource_id.clone(),
                "fr.read",
                ResourceAction::Read,
            ),
        },
        DeniedOperationalCase {
            label: "missing-action-grant",
            scope: direct_negative_scope(
                &missing_action,
                valid_resource_id.clone(),
                "fr.read",
                ResourceAction::Read,
            ),
        },
        DeniedOperationalCase {
            label: "wrong-action-grant",
            scope: direct_negative_scope(
                &wrong_action,
                valid_resource_id.clone(),
                "fr.read",
                ResourceAction::Read,
            ),
        },
        DeniedOperationalCase {
            label: "missing-capability-grant",
            scope: direct_negative_scope(
                &missing_grant_capability,
                valid_resource_id.clone(),
                "fr.read",
                ResourceAction::Read,
            ),
        },
        DeniedOperationalCase {
            label: "wrong-capability-grant",
            scope: direct_negative_scope(
                &wrong_grant_capability,
                valid_resource_id.clone(),
                "fr.read",
                ResourceAction::Read,
            ),
        },
        DeniedOperationalCase {
            label: "missing-resource",
            scope: direct_negative_scope(
                &missing_resource,
                valid_resource_id.clone(),
                "fr.read",
                ResourceAction::Read,
            ),
        },
        DeniedOperationalCase {
            label: "wrong-resource",
            scope: direct_negative_scope(
                &wrong_resource,
                valid_resource_id.clone(),
                "fr.read",
                ResourceAction::Read,
            ),
        },
        DeniedOperationalCase {
            label: "mismatched-delegation-chain",
            scope: direct_negative_scope(
                &mismatched_delegation,
                valid_resource_id.clone(),
                "fr.read",
                ResourceAction::Read,
            ),
        },
        DeniedOperationalCase {
            label: "revoked-resource-grant",
            scope: direct_negative_scope(
                &revoked_grant,
                valid_resource_id.clone(),
                "fr.read",
                ResourceAction::Read,
            ),
        },
        DeniedOperationalCase {
            label: "stale-post-space-switch",
            scope: direct_negative_scope(
                &stale_space,
                valid_resource_id.clone(),
                "fr.read",
                ResourceAction::Read,
            ),
        },
        DeniedOperationalCase {
            label: "wrong-access-space",
            scope: direct_negative_scope(
                &wrong_space,
                valid_resource_id.clone(),
                "fr.read",
                ResourceAction::Read,
            ),
        },
        DeniedOperationalCase {
            label: "cross-account",
            scope: direct_negative_scope(
                &foreign,
                valid_resource_id.clone(),
                "fr.read",
                ResourceAction::Read,
            ),
        },
        DeniedOperationalCase {
            label: "revoked-session",
            scope: direct_negative_scope(
                &revoked,
                valid_resource_id.clone(),
                "fr.read",
                ResourceAction::Read,
            ),
        },
        DeniedOperationalCase {
            label: "expired-session",
            scope: direct_negative_scope(
                &expired,
                valid_resource_id.clone(),
                "fr.read",
                ResourceAction::Read,
            ),
        },
    ];
    let mut forged_session_scope =
        direct_negative_scope(&owner, valid_resource_id, "fr.read", ResourceAction::Read);
    forged_session_scope.session_token = "f".repeat(64);
    denied_scopes.push(DeniedOperationalCase {
        label: "forged-session",
        scope: forged_session_scope,
    });
    let probes = [
        (
            "workspaces",
            [
                "SELECT * FROM type::record($table, $sentinel_id);",
                "CREATE type::record($table, $record_id) CONTENT { name: 'denied-create' } RETURN AFTER;",
                "UPSERT type::record($table, $record_id) CONTENT { name: 'denied-upsert-new' } RETURN AFTER;",
                "UPSERT type::record($table, $sentinel_id) CONTENT { name: 'denied-upsert-existing' } RETURN AFTER;",
                "UPDATE type::record($table, $sentinel_id) SET name = 'denied-update' RETURN AFTER;",
                "DELETE type::record($table, $sentinel_id) RETURN BEFORE;",
            ],
        ),
        (
            "kernel_event_ledger",
            [
                "SELECT * FROM type::record($table, $sentinel_id);",
                "CREATE type::record($table, $record_id) CONTENT { event_id: $record_id, event_version: 'v1', kernel_task_run_id: 'mt109-v12', session_run_id: 'mt109-v12', aggregate_type: 'native_editor', aggregate_id: $workspace_key, idempotency_key: $record_id, event_type: 'MT109_V12_DENIED', actor_kind: 'SYSTEM', actor_id: 'mt109-v12', payload_hash: $hash, source_component: 'mt109-v12-test', payload: { marker: 'denied' }, wsids: [$workspace_key], authority_resource_id: $authority_resource, authority_session_id: $authority_session, authority_capability_id: 'fr.ingest.native_editor', authority_action: 'create' } RETURN AFTER;",
                "UPSERT type::record($table, $record_id) CONTENT { event_id: $record_id, event_version: 'v1', kernel_task_run_id: 'mt109-v12', session_run_id: 'mt109-v12', aggregate_type: 'native_editor', aggregate_id: $workspace_key, idempotency_key: $record_id, event_type: 'MT109_V12_DENIED', actor_kind: 'SYSTEM', actor_id: 'mt109-v12', payload_hash: $hash, source_component: 'mt109-v12-test', payload: { marker: 'denied' }, wsids: [$workspace_key], authority_resource_id: $authority_resource, authority_session_id: $authority_session, authority_capability_id: 'fr.ingest.native_editor', authority_action: 'create' } RETURN AFTER;",
                "UPSERT type::record($table, $sentinel_id) SET payload = { marker: 'denied-upsert-existing' } RETURN AFTER;",
                "UPDATE type::record($table, $sentinel_id) SET payload = { marker: 'denied-update' } RETURN AFTER;",
                "DELETE type::record($table, $sentinel_id) RETURN BEFORE;",
            ],
        ),
        (
            "fems_memory_packs",
            [
                "SELECT * FROM type::record($table, $sentinel_id);",
                "CREATE type::record($table, $record_id) CONTENT { pack_id: $record_id, workspace_id: $workspace, scope_key: $record_id, pack: { marker: 'denied-create' }, generated_at: time::now() } RETURN AFTER;",
                "UPSERT type::record($table, $record_id) CONTENT { pack_id: $record_id, workspace_id: $workspace, scope_key: $record_id, pack: { marker: 'denied-upsert-new' }, generated_at: time::now() } RETURN AFTER;",
                "UPSERT type::record($table, $sentinel_id) SET scope_key = 'denied-upsert-existing' RETURN AFTER;",
                "UPDATE type::record($table, $sentinel_id) SET scope_key = 'denied-update' RETURN AFTER;",
                "DELETE type::record($table, $sentinel_id) RETURN BEFORE;",
            ],
        ),
        (
            "fems_memory_proposals",
            [
                "SELECT * FROM type::record($table, $sentinel_id);",
                "CREATE type::record($table, $record_id) CONTENT { proposal_id: $record_id, request_id: $record_id, workspace_id: $workspace, document_id: 'mt109-v12-document', selection_start: 0, selection_end: 1, content_hash: $hash, memory_class: 'episodic', status: 'pending_review', review_gated: true, proposal: { marker: 'denied-create' } } RETURN AFTER;",
                "UPSERT type::record($table, $record_id) CONTENT { proposal_id: $record_id, request_id: $record_id, workspace_id: $workspace, document_id: 'mt109-v12-document', selection_start: 0, selection_end: 1, content_hash: $hash, memory_class: 'episodic', status: 'pending_review', review_gated: true, proposal: { marker: 'denied-upsert-new' } } RETURN AFTER;",
                "UPSERT type::record($table, $sentinel_id) SET status = 'denied-upsert-existing' RETURN AFTER;",
                "UPDATE type::record($table, $sentinel_id) SET status = 'denied-update' RETURN AFTER;",
                "DELETE type::record($table, $sentinel_id) RETURN BEFORE;",
            ],
        ),
        (
            "fems_memory_items",
            [
                "SELECT * FROM type::record($table, $sentinel_id);",
                "CREATE type::record($table, $record_id) CONTENT { memory_id: $record_id, workspace_id: $workspace, item: { marker: 'denied-create' } } RETURN AFTER;",
                "UPSERT type::record($table, $record_id) CONTENT { memory_id: $record_id, workspace_id: $workspace, item: { marker: 'denied-upsert-new' } } RETURN AFTER;",
                "UPSERT type::record($table, $sentinel_id) SET item = { marker: 'denied-upsert-existing' } RETURN AFTER;",
                "UPDATE type::record($table, $sentinel_id) SET item = { marker: 'denied-update' } RETURN AFTER;",
                "DELETE type::record($table, $sentinel_id) RETURN BEFORE;",
            ],
        ),
        (
            "fems_memory_commit_reports",
            [
                "SELECT * FROM type::record($table, $sentinel_id);",
                "CREATE type::record($table, $record_id) CONTENT { commit_id: $record_id, workspace_id: $workspace, proposal_id: type::record('fems_memory_proposals', $record_id + '-proposal-dep'), memory_id: type::record('fems_memory_items', $record_id + '-item-dep'), report: { marker: 'denied-create' }, report_hash: $hash, created_at: time::now() } RETURN AFTER;",
                "UPSERT type::record($table, $record_id) CONTENT { commit_id: $record_id, workspace_id: $workspace, proposal_id: type::record('fems_memory_proposals', $record_id + '-proposal-dep'), memory_id: type::record('fems_memory_items', $record_id + '-item-dep'), report: { marker: 'denied-upsert-new' }, report_hash: $hash, created_at: time::now() } RETURN AFTER;",
                "UPSERT type::record($table, $sentinel_id) SET report = { marker: 'denied-upsert-existing' } RETURN AFTER;",
                "UPDATE type::record($table, $sentinel_id) SET report = { marker: 'denied-update' } RETURN AFTER;",
                "DELETE type::record($table, $sentinel_id) RETURN BEFORE;",
            ],
        ),
        (
            "fems_memory_commit_fr_outbox",
            [
                "SELECT * FROM type::record($table, $sentinel_id);",
                "CREATE type::record($table, $record_id) CONTENT { event_id: $record_id, workspace_id: $workspace, proposal_id: type::record('fems_memory_proposals', $record_id + '-proposal-dep'), commit_id: type::record('fems_memory_commit_reports', $record_id + '-report-dep'), event_code: 'FR-EVT-MEM-003', event: { marker: 'denied-create' }, event_hash: $hash, created_at: time::now(), published_at: NONE, attempt_count: 0, last_error: NONE, last_error_at: NONE, quarantined_at: NONE } RETURN AFTER;",
                "UPSERT type::record($table, $record_id) CONTENT { event_id: $record_id, workspace_id: $workspace, proposal_id: type::record('fems_memory_proposals', $record_id + '-proposal-dep'), commit_id: type::record('fems_memory_commit_reports', $record_id + '-report-dep'), event_code: 'FR-EVT-MEM-003', event: { marker: 'denied-upsert-new' }, event_hash: $hash, created_at: time::now(), published_at: NONE, attempt_count: 0, last_error: NONE, last_error_at: NONE, quarantined_at: NONE } RETURN AFTER;",
                "UPSERT type::record($table, $sentinel_id) SET attempt_count = 10 RETURN AFTER;",
                "UPDATE type::record($table, $sentinel_id) SET attempt_count = 11 RETURN AFTER;",
                "DELETE type::record($table, $sentinel_id) RETURN BEFORE;",
            ],
        ),
        (
            "fems_memory_lifecycle_fr_outbox",
            [
                "SELECT * FROM type::record($table, $sentinel_id);",
                "CREATE type::record($table, $record_id) CONTENT { event_id: $record_id, workspace_id: $workspace, proposal_id: type::record('fems_memory_proposals', $record_id + '-proposal-dep'), event_code: 'FR-EVT-MEM-001', event: { marker: 'denied-create' }, event_hash: $hash, created_at: time::now(), published_at: NONE, attempt_count: 0, last_error: NONE, last_error_at: NONE, quarantined_at: NONE } RETURN AFTER;",
                "UPSERT type::record($table, $record_id) CONTENT { event_id: $record_id, workspace_id: $workspace, proposal_id: type::record('fems_memory_proposals', $record_id + '-proposal-dep'), event_code: 'FR-EVT-MEM-001', event: { marker: 'denied-upsert-new' }, event_hash: $hash, created_at: time::now(), published_at: NONE, attempt_count: 0, last_error: NONE, last_error_at: NONE, quarantined_at: NONE } RETURN AFTER;",
                "UPSERT type::record($table, $sentinel_id) SET attempt_count = 10 RETURN AFTER;",
                "UPDATE type::record($table, $sentinel_id) SET attempt_count = 11 RETURN AFTER;",
                "DELETE type::record($table, $sentinel_id) RETURN BEFORE;",
            ],
        ),
        (
            "fems_workspace_write_anchors",
            [
                "SELECT * FROM type::record($table, $sentinel_id);",
                "CREATE type::record($table, $record_id) CONTENT { anchor_key: $record_id, workspace_key: $workspace_key, claim_nonce: 'denied-create' } RETURN AFTER;",
                "UPSERT type::record($table, $record_id) CONTENT { anchor_key: $record_id, workspace_key: $workspace_key, claim_nonce: 'denied-upsert-new' } RETURN AFTER;",
                "UPSERT type::record($table, $sentinel_id) SET claim_nonce = 'denied-upsert-existing' RETURN AFTER;",
                "UPDATE type::record($table, $sentinel_id) SET claim_nonce = 'denied-update' RETURN AFTER;",
                "DELETE type::record($table, $sentinel_id) RETURN BEFORE;",
            ],
        ),
    ];
    let authorized_scope = RecordUserScope {
        session_token: owner.session.token.clone(),
        channel_binding_hash: Some("direct-negative-binding".to_owned()),
        resource_id: sentinel_decision.resource_id.clone(),
        session_id: sentinel_decision.session_id.clone(),
        capability_id: "memory.commit".to_owned(),
        action: ResourceAction::Update,
    };
    for (table, statements) in &probes {
        let base_bindings = OperationalProbeBindings {
            table: (*table).to_owned(),
            record_id: format!("authorized-create-{}", table.replace('_', "-")),
            sentinel_id: if *table == "workspaces" {
                workspace.id.clone()
            } else {
                OPERATIONAL_SENTINEL_ID.to_owned()
            },
            workspace: RecordId::new("workspaces", workspace.id.as_str()),
            workspace_key: workspace.id.clone(),
            authority_resource: RecordId::new(
                "protected_resources",
                sentinel_decision.resource_id.clone(),
            ),
            authority_session: RecordId::new(
                "authenticated_sessions",
                sentinel_decision.session_id.clone(),
            ),
            hash: OPERATIONAL_HASH.to_owned(),
        };
        assert_record_user_select_matches_privileged(
            storage,
            authorized_scope.clone(),
            base_bindings.clone(),
        )
        .await?;

        if *table == "workspaces" {
            assert_record_user_operation_has_zero_effect(
                storage,
                authorized_scope.clone(),
                statements[1],
                base_bindings.clone(),
            )
            .await?;
        } else {
            seed_operational_probe_dependencies(storage, base_bindings.clone()).await?;
            assert_record_user_create_matches_privileged(
                storage,
                authorized_scope.clone(),
                statements[1],
                base_bindings.clone(),
            )
            .await?;
        }

        let mut upsert_new = base_bindings.clone();
        upsert_new.record_id = format!("authorized-upsert-new-{}", table.replace('_', "-"));
        if *table == "workspaces" {
            assert_record_user_operation_has_zero_effect(
                storage,
                authorized_scope.clone(),
                statements[2],
                upsert_new,
            )
            .await?;
        } else {
            seed_operational_probe_dependencies(storage, upsert_new.clone()).await?;
            assert_record_user_create_matches_privileged(
                storage,
                authorized_scope.clone(),
                statements[2],
                upsert_new,
            )
            .await?;
        }

        let mut existing = base_bindings.clone();
        existing.record_id = existing.sentinel_id.clone();
        if matches!(
            *table,
            "workspaces" | "kernel_event_ledger" | "fems_memory_commit_reports"
        ) {
            for statement in [statements[3], statements[4]] {
                assert_record_user_operation_has_zero_effect(
                    storage,
                    authorized_scope.clone(),
                    statement,
                    existing.clone(),
                )
                .await?;
            }
        } else {
            for statement in [statements[3], statements[4]] {
                assert_record_user_update_matches_privileged(
                    storage,
                    authorized_scope.clone(),
                    statement,
                    existing.clone(),
                )
                .await?;
            }
        }
        assert_record_user_operation_has_zero_effect(
            storage,
            authorized_scope.clone(),
            statements[5],
            existing,
        )
        .await?;
    }
    for denied_case in denied_scopes {
        for (table, statements) in &probes {
            let bindings = OperationalProbeBindings {
                table: (*table).to_owned(),
                record_id: format!("{}-{}", denied_case.label, table.replace('_', "-")),
                sentinel_id: if *table == "workspaces" {
                    workspace.id.clone()
                } else {
                    OPERATIONAL_SENTINEL_ID.to_owned()
                },
                workspace: RecordId::new("workspaces", workspace.id.as_str()),
                workspace_key: workspace.id.clone(),
                authority_resource: RecordId::new(
                    "protected_resources",
                    sentinel_decision.resource_id.clone(),
                ),
                authority_session: RecordId::new(
                    "authenticated_sessions",
                    sentinel_decision.session_id.clone(),
                ),
                hash: OPERATIONAL_HASH.to_owned(),
            };
            seed_operational_probe_dependencies(storage, bindings.clone()).await?;
            for statement in statements {
                assert_record_user_operation_has_zero_effect(
                    storage,
                    denied_case.scope.clone(),
                    statement,
                    bindings.clone(),
                )
                .await?;
            }
        }
    }
    Ok(())
}

#[tokio::test]
async fn explicit_session_credentials_are_one_time_revocable_and_channel_bound(
) -> Result<(), Box<dyn std::error::Error>> {
    let backend = embedded_test_backend().await?;
    let storage = &backend.storage;
    let principal = storage
        .provision_principal(
            "credential-account",
            "credential-principal",
            "human_account",
            "credential-actor",
            "Operator",
            &["fr.read".to_owned()],
            "credential-space",
            None,
            Duration::from_secs(300),
        )
        .await?;
    let credential = storage
        .provision_session_credential(&principal.identity, Duration::from_secs(300))
        .await?;
    let session = storage
        .exchange_session_credential(
            &principal.identity.account_id,
            &principal.identity.principal_id,
            &principal.identity.access_space_id,
            &credential.token,
            "credential-binding",
            Duration::from_secs(300),
        )
        .await?;
    assert_eq!(session.principal_id, principal.identity.principal_id);
    let resource = storage
        .register_protected_resource(
            &principal.identity,
            ResourceKind::FlightRecorder,
            "credential-workspace",
            None,
            "account_private",
        )
        .await?;
    storage
        .grant_resource(
            &principal.identity.account_id,
            &principal.identity.access_space_id,
            ResourceGrantSpec {
                principal_id: principal.identity.principal_id.clone(),
                resource_id: resource.resource_id,
                actions: vec![ResourceAction::Read],
                capability_ids: vec!["fr.read".to_owned()],
                expires_at: Some(session.expires_at),
                delegation_chain: Vec::new(),
            },
        )
        .await?;
    storage
        .authorize_protected_resource(request(
            &session.token,
            Some("credential-binding"),
            "fr.read",
            "credential-workspace",
        ))
        .await?;
    assert!(matches!(
        storage
            .authorize_protected_resource(request(
                &session.token,
                Some("forged-binding"),
                "fr.read",
                "credential-workspace"
            ))
            .await,
        Err(ResourceAuthorityError::Denied { .. })
    ));
    assert!(
        storage
            .exchange_session_credential(
                &principal.identity.account_id,
                &principal.identity.principal_id,
                &principal.identity.access_space_id,
                &credential.token,
                "credential-binding",
                Duration::from_secs(300)
            )
            .await
            .is_err(),
        "consumed credential was replayed"
    );

    let revoked = storage
        .provision_session_credential(&principal.identity, Duration::from_secs(300))
        .await?;
    storage
        .revoke_session_credential(&revoked.credential_id)
        .await?;
    assert!(storage
        .exchange_session_credential(
            &principal.identity.account_id,
            &principal.identity.principal_id,
            &principal.identity.access_space_id,
            &revoked.token,
            "credential-binding",
            Duration::from_secs(300)
        )
        .await
        .is_err());
    let expired = storage
        .provision_session_credential(&principal.identity, Duration::from_millis(1))
        .await?;
    tokio::time::sleep(Duration::from_millis(10)).await;
    assert!(storage
        .exchange_session_credential(
            &principal.identity.account_id,
            &principal.identity.principal_id,
            &principal.identity.access_space_id,
            &expired.token,
            "credential-binding",
            Duration::from_secs(300)
        )
        .await
        .is_err());
    let raced = storage
        .provision_session_credential(&principal.identity, Duration::from_secs(300))
        .await?;
    let (revoke_result, exchange_result) = tokio::join!(
        storage.revoke_session_credential(&raced.credential_id),
        storage.exchange_session_credential(
            &principal.identity.account_id,
            &principal.identity.principal_id,
            &principal.identity.access_space_id,
            &raced.token,
            "credential-binding",
            Duration::from_secs(300)
        ),
    );
    revoke_result?;
    if let Ok(raced_session) = exchange_result {
        storage.revoke_session(&raced_session.session_id).await?;
    }
    assert!(
        storage
            .exchange_session_credential(
                &principal.identity.account_id,
                &principal.identity.principal_id,
                &principal.identity.access_space_id,
                &raced.token,
                "credential-binding",
                Duration::from_secs(300)
            )
            .await
            .is_err(),
        "concurrent exchange left a replayable credential"
    );
    Ok(())
}

#[tokio::test]
async fn access_space_switch_is_same_account_only_and_drops_old_space_grants(
) -> Result<(), Box<dyn std::error::Error>> {
    let backend = embedded_test_backend().await?;
    let storage = &backend.storage;
    let original = storage
        .provision_principal(
            "switch-account",
            "switch-principal",
            "human_account",
            "switch-actor",
            "Operator",
            &["fr.read".to_owned()],
            "switch-space-a",
            None,
            Duration::from_secs(300),
        )
        .await?;
    let resource = storage
        .register_protected_resource(
            &original.identity,
            ResourceKind::FlightRecorder,
            "switch-workspace",
            None,
            "account_private",
        )
        .await?;
    storage
        .grant_resource(
            &original.identity.account_id,
            &original.identity.access_space_id,
            ResourceGrantSpec {
                principal_id: original.identity.principal_id.clone(),
                resource_id: resource.resource_id,
                actions: vec![ResourceAction::Read],
                capability_ids: vec!["fr.read".to_owned()],
                expires_at: Some(original.session.expires_at),
                delegation_chain: Vec::new(),
            },
        )
        .await?;
    storage
        .authorize_protected_resource(request(
            &original.session.token,
            None,
            "fr.read",
            "switch-workspace",
        ))
        .await?;
    let second_space = storage
        .provision_principal(
            "switch-account",
            "switch-principal",
            "human_account",
            "switch-actor",
            "Operator",
            &["fr.read".to_owned()],
            "switch-space-b",
            None,
            Duration::from_secs(300),
        )
        .await?;
    let outsider = storage
        .provision_principal(
            "switch-outsider-account",
            "switch-outsider-principal",
            "human_account",
            "switch-outsider",
            "Operator",
            &["fr.read".to_owned()],
            "switch-outsider-space",
            None,
            Duration::from_secs(300),
        )
        .await?;
    assert!(storage
        .switch_session_access_space(
            &original.session.session_id,
            &outsider.identity.access_space_id
        )
        .await
        .is_err());
    storage
        .switch_session_access_space(
            &original.session.session_id,
            &second_space.identity.access_space_id,
        )
        .await?;
    assert!(
        matches!(
            storage
                .authorize_protected_resource(request(
                    &original.session.token,
                    None,
                    "fr.read",
                    "switch-workspace"
                ))
                .await,
            Err(ResourceAuthorityError::Denied { .. })
        ),
        "old-Space grant survived the explicit switch"
    );
    Ok(())
}

#[tokio::test]
async fn grant_without_delegated_capability_is_denied() -> Result<(), Box<dyn std::error::Error>> {
    let backend = embedded_test_backend().await?;
    let storage = &backend.storage;
    let principal = storage
        .provision_principal(
            "asymmetry-account",
            "asymmetry-principal",
            "human_account",
            "asymmetry-actor",
            "Operator",
            &["memory.read".to_owned()],
            "asymmetry-space",
            None,
            Duration::from_secs(300),
        )
        .await?;
    let resource = storage
        .register_protected_resource(
            &principal.identity,
            ResourceKind::FlightRecorder,
            "asymmetry-workspace",
            None,
            "account_private",
        )
        .await?;
    storage
        .grant_resource(
            &principal.identity.account_id,
            &principal.identity.access_space_id,
            ResourceGrantSpec {
                principal_id: principal.identity.principal_id.clone(),
                resource_id: resource.resource_id,
                actions: vec![ResourceAction::Read],
                capability_ids: vec!["fr.read".to_owned()],
                expires_at: Some(principal.session.expires_at),
                delegation_chain: Vec::new(),
            },
        )
        .await?;
    assert!(matches!(
        storage
            .authorize_protected_resource(request(
                &principal.session.token,
                None,
                "fr.read",
                "asymmetry-workspace"
            ))
            .await,
        Err(ResourceAuthorityError::Denied { .. })
    ));
    Ok(())
}

#[tokio::test]
async fn reconciliation_requires_explicit_service_identity_and_workspace_grant(
) -> Result<(), Box<dyn std::error::Error>> {
    let backend = embedded_test_backend().await?;
    let storage = &backend.storage;
    assert!(
        storage.issue_reconciliation_session().await.is_err(),
        "runtime recovery minted an identity"
    );
    let workspace = "service-recovery-workspace".to_owned();
    let service = storage
        .provision_reconciliation_principal(std::slice::from_ref(&workspace), None)
        .await?;
    let issued = storage.issue_reconciliation_session().await?;
    assert_eq!(issued.identity.principal_id, service.identity.principal_id);
    storage
        .authorize_protected_resource(AuthorizationRequest {
            session_token: issued.session.token,
            channel_binding_hash: None,
            capability_id: "memory.commit".to_owned(),
            resource_kind: ResourceKind::ReconciliationQueue,
            external_resource_id: format!(
                "{}:{workspace}",
                super::resource_authority::RECONCILIATION_QUEUE_ID
            ),
            action: ResourceAction::Reconcile,
        })
        .await?;
    let human = storage
        .provision_principal(
            "recovery-human-account",
            "recovery-human-principal",
            "human_account",
            "recovery-human",
            super::resource_authority::RECONCILIATION_PROFILE_ID,
            &["memory.commit".to_owned()],
            "recovery-human-space",
            None,
            Duration::from_secs(300),
        )
        .await?;
    let queue = storage
        .register_protected_resource(
            &human.identity,
            ResourceKind::ReconciliationQueue,
            "human-recovery-queue",
            None,
            "restricted",
        )
        .await?;
    storage
        .grant_resource(
            &human.identity.account_id,
            &human.identity.access_space_id,
            ResourceGrantSpec {
                principal_id: human.identity.principal_id.clone(),
                resource_id: queue.resource_id,
                actions: vec![ResourceAction::Reconcile],
                capability_ids: vec!["memory.commit".to_owned()],
                expires_at: Some(human.session.expires_at),
                delegation_chain: Vec::new(),
            },
        )
        .await?;
    assert!(matches!(
        storage
            .authorize_protected_resource(AuthorizationRequest {
                session_token: human.session.token,
                channel_binding_hash: None,
                capability_id: "memory.commit".to_owned(),
                resource_kind: ResourceKind::ReconciliationQueue,
                external_resource_id: "human-recovery-queue".to_owned(),
                action: ResourceAction::Reconcile
            })
            .await,
        Err(ResourceAuthorityError::Denied { .. })
    ));
    Ok(())
}
