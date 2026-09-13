use std::time::Duration;

use super::resource_authority::{
    AuthorizationRequest, RecordUserScope, ResourceAction, ResourceAuthorityError,
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
    workspace: RecordId,
    workspace_key: String,
}

async fn assert_record_user_operation_has_zero_effect(
    storage: &super::SurrealStorage,
    scope: RecordUserScope,
    statement: &'static str,
    bindings: OperationalProbeBindings,
) -> Result<(), Box<dyn std::error::Error>> {
    let result = storage
        .with_record_user_scope(
            scope,
            storage.with_data_operation(move |database| {
                Box::pin(
                    async move { database.query_values::<Value, _>(statement, bindings).await },
                )
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
    let workspace = RecordId::new("workspaces", foreign_workspace.id.as_str());

    assert_record_user_operation_has_zero_effect(
        storage,
        scope.clone(),
        "SELECT * FROM workspaces WHERE id = $workspace;",
        OperationalProbeBindings {
            table: "workspaces".to_owned(),
            record_id: "foreign-workspace-probe".to_owned(),
            workspace: workspace.clone(),
            workspace_key: foreign_workspace.id.clone(),
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
                workspace: workspace.clone(),
                workspace_key: foreign_workspace.id.clone(),
            },
        )
        .await?;
    }

    for (table, statements) in [
        (
            "kernel_event_ledger",
            [
                "SELECT * FROM kernel_event_ledger WHERE array::contains(wsids, $workspace_key);",
                "CREATE type::record($table, $record_id) SET wsids = [$workspace_key] RETURN AFTER;",
                "UPSERT type::record($table, $record_id) SET wsids = [$workspace_key] RETURN AFTER;",
                "UPDATE kernel_event_ledger SET wsids = [$workspace_key] WHERE array::contains(wsids, $workspace_key) RETURN AFTER;",
                "DELETE kernel_event_ledger WHERE array::contains(wsids, $workspace_key) RETURN BEFORE;",
            ],
        ),
        (
            "fems_memory_packs",
            [
                "SELECT * FROM fems_memory_packs WHERE workspace_id = $workspace;",
                "CREATE type::record($table, $record_id) SET workspace_id = $workspace RETURN AFTER;",
                "UPSERT type::record($table, $record_id) SET workspace_id = $workspace RETURN AFTER;",
                "UPDATE fems_memory_packs SET workspace_id = $workspace WHERE workspace_id = $workspace RETURN AFTER;",
                "DELETE fems_memory_packs WHERE workspace_id = $workspace RETURN BEFORE;",
            ],
        ),
        (
            "fems_memory_proposals",
            [
                "SELECT * FROM fems_memory_proposals WHERE workspace_id = $workspace;",
                "CREATE type::record($table, $record_id) SET workspace_id = $workspace RETURN AFTER;",
                "UPSERT type::record($table, $record_id) SET workspace_id = $workspace RETURN AFTER;",
                "UPDATE fems_memory_proposals SET workspace_id = $workspace WHERE workspace_id = $workspace RETURN AFTER;",
                "DELETE fems_memory_proposals WHERE workspace_id = $workspace RETURN BEFORE;",
            ],
        ),
        (
            "fems_memory_items",
            [
                "SELECT * FROM fems_memory_items WHERE workspace_id = $workspace;",
                "CREATE type::record($table, $record_id) SET workspace_id = $workspace RETURN AFTER;",
                "UPSERT type::record($table, $record_id) SET workspace_id = $workspace RETURN AFTER;",
                "UPDATE fems_memory_items SET workspace_id = $workspace WHERE workspace_id = $workspace RETURN AFTER;",
                "DELETE fems_memory_items WHERE workspace_id = $workspace RETURN BEFORE;",
            ],
        ),
        (
            "fems_memory_commit_reports",
            [
                "SELECT * FROM fems_memory_commit_reports WHERE workspace_id = $workspace;",
                "CREATE type::record($table, $record_id) SET workspace_id = $workspace RETURN AFTER;",
                "UPSERT type::record($table, $record_id) SET workspace_id = $workspace RETURN AFTER;",
                "UPDATE fems_memory_commit_reports SET workspace_id = $workspace WHERE workspace_id = $workspace RETURN AFTER;",
                "DELETE fems_memory_commit_reports WHERE workspace_id = $workspace RETURN BEFORE;",
            ],
        ),
        (
            "fems_memory_commit_fr_outbox",
            [
                "SELECT * FROM fems_memory_commit_fr_outbox WHERE workspace_id = $workspace;",
                "CREATE type::record($table, $record_id) SET workspace_id = $workspace RETURN AFTER;",
                "UPSERT type::record($table, $record_id) SET workspace_id = $workspace RETURN AFTER;",
                "UPDATE fems_memory_commit_fr_outbox SET workspace_id = $workspace WHERE workspace_id = $workspace RETURN AFTER;",
                "DELETE fems_memory_commit_fr_outbox WHERE workspace_id = $workspace RETURN BEFORE;",
            ],
        ),
        (
            "fems_memory_lifecycle_fr_outbox",
            [
                "SELECT * FROM fems_memory_lifecycle_fr_outbox WHERE workspace_id = $workspace;",
                "CREATE type::record($table, $record_id) SET workspace_id = $workspace RETURN AFTER;",
                "UPSERT type::record($table, $record_id) SET workspace_id = $workspace RETURN AFTER;",
                "UPDATE fems_memory_lifecycle_fr_outbox SET workspace_id = $workspace WHERE workspace_id = $workspace RETURN AFTER;",
                "DELETE fems_memory_lifecycle_fr_outbox WHERE workspace_id = $workspace RETURN BEFORE;",
            ],
        ),
        (
            "fems_workspace_write_anchors",
            [
                "SELECT * FROM fems_workspace_write_anchors WHERE workspace_key = $workspace_key;",
                "CREATE type::record($table, $record_id) SET workspace_key = $workspace_key RETURN AFTER;",
                "UPSERT type::record($table, $record_id) SET workspace_key = $workspace_key RETURN AFTER;",
                "UPDATE fems_workspace_write_anchors SET workspace_key = $workspace_key WHERE workspace_key = $workspace_key RETURN AFTER;",
                "DELETE fems_workspace_write_anchors WHERE workspace_key = $workspace_key RETURN BEFORE;",
            ],
        ),
    ] {
        let bindings = OperationalProbeBindings {
            table: table.to_owned(),
            record_id: format!("mt109-foreign-{}", table.replace('_', "-")),
            workspace: workspace.clone(),
            workspace_key: foreign_workspace.id.clone(),
        };
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
    let member_without_grant = storage
        .provision_principal(
            "direct-negative-account",
            "direct-negative-member",
            "human_account",
            "direct-negative-member",
            "Operator",
            &capabilities,
            "direct-negative-space-a",
            Some("direct-negative-binding"),
            Duration::from_secs(300),
        )
        .await?;
    assert_eq!(
        member_without_grant.identity.account_id,
        owner.identity.account_id
    );
    assert_eq!(
        member_without_grant.identity.access_space_id,
        owner.identity.access_space_id
    );
    let grant_without_capability = storage
        .provision_principal(
            "direct-negative-account",
            "direct-negative-grant-without-capability",
            "human_account",
            "direct-negative-grant-without-capability",
            "Operator",
            &[],
            "direct-negative-space-a",
            Some("direct-negative-binding"),
            Duration::from_secs(300),
        )
        .await?;
    grant_every_route(storage, &grant_without_capability, &workspace.id).await?;
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

    let denied_scopes = [
        (
            "same-account-member-without-grant",
            member_without_grant.session.token,
        ),
        (
            "grant-without-delegated-capability",
            grant_without_capability.session.token,
        ),
        ("wrong-access-space", wrong_space.session.token),
        ("cross-account", foreign.session.token),
        ("revoked-session", revoked.session.token),
        ("expired-session", expired.session.token),
        ("forged-session", "f".repeat(64)),
    ];
    let probes = [
        (
            "workspaces",
            "SELECT * FROM workspaces WHERE id = $workspace;",
            "UPDATE workspaces SET name = 'bulk bypass' WHERE id = $workspace RETURN AFTER;",
        ),
        (
            "kernel_event_ledger",
            "SELECT * FROM kernel_event_ledger WHERE array::contains(wsids, $workspace_key);",
            "UPDATE kernel_event_ledger SET wsids = [$workspace_key] WHERE array::contains(wsids, $workspace_key) RETURN AFTER;",
        ),
        (
            "fems_memory_packs",
            "SELECT * FROM fems_memory_packs WHERE workspace_id = $workspace;",
            "UPDATE fems_memory_packs SET workspace_id = $workspace WHERE workspace_id = $workspace RETURN AFTER;",
        ),
        (
            "fems_memory_proposals",
            "SELECT * FROM fems_memory_proposals WHERE workspace_id = $workspace;",
            "UPDATE fems_memory_proposals SET workspace_id = $workspace WHERE workspace_id = $workspace RETURN AFTER;",
        ),
        (
            "fems_memory_items",
            "SELECT * FROM fems_memory_items WHERE workspace_id = $workspace;",
            "UPDATE fems_memory_items SET workspace_id = $workspace WHERE workspace_id = $workspace RETURN AFTER;",
        ),
        (
            "fems_memory_commit_reports",
            "SELECT * FROM fems_memory_commit_reports WHERE workspace_id = $workspace;",
            "UPDATE fems_memory_commit_reports SET workspace_id = $workspace WHERE workspace_id = $workspace RETURN AFTER;",
        ),
        (
            "fems_memory_commit_fr_outbox",
            "SELECT * FROM fems_memory_commit_fr_outbox WHERE workspace_id = $workspace;",
            "UPDATE fems_memory_commit_fr_outbox SET workspace_id = $workspace WHERE workspace_id = $workspace RETURN AFTER;",
        ),
        (
            "fems_memory_lifecycle_fr_outbox",
            "SELECT * FROM fems_memory_lifecycle_fr_outbox WHERE workspace_id = $workspace;",
            "UPDATE fems_memory_lifecycle_fr_outbox SET workspace_id = $workspace WHERE workspace_id = $workspace RETURN AFTER;",
        ),
        (
            "fems_workspace_write_anchors",
            "SELECT * FROM fems_workspace_write_anchors WHERE workspace_key = $workspace_key;",
            "UPDATE fems_workspace_write_anchors SET workspace_key = $workspace_key WHERE workspace_key = $workspace_key RETURN AFTER;",
        ),
    ];
    for (label, token) in denied_scopes {
        let scope = RecordUserScope {
            session_token: token,
            channel_binding_hash: Some("direct-negative-binding".to_owned()),
            resource_id: owner.identity.principal_id.clone(),
            session_id: owner.session.session_id.clone(),
            capability_id: "forged.capability".to_owned(),
            action: ResourceAction::Delete,
        };
        for (table, read, bulk_or_recovery) in probes {
            let bindings = OperationalProbeBindings {
                table: table.to_owned(),
                record_id: format!("{label}-{}", table.replace('_', "-")),
                workspace: RecordId::new("workspaces", workspace.id.as_str()),
                workspace_key: workspace.id.clone(),
            };
            for statement in [read, bulk_or_recovery] {
                assert_record_user_operation_has_zero_effect(
                    storage,
                    scope.clone(),
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
