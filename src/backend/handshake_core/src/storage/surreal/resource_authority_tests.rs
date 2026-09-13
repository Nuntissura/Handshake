use std::time::Duration;

use super::resource_authority::{
    AuthorizationRequest, ResourceAction, ResourceAuthorityError, ResourceGrantSpec, ResourceKind,
};
use crate::storage::tests::embedded_test_backend;

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
    storage
        .revoke_session(&principal.session.session_id)
        .await?;
    assert!(matches!(
        storage
            .authorize_protected_resource(request(
                &principal.session.token,
                None,
                "fr.read",
                "unregistered",
            ))
            .await,
        Err(ResourceAuthorityError::Denied { .. })
    ));
    storage
        .disable_account(&principal.identity.account_id)
        .await?;
    Ok(())
}
