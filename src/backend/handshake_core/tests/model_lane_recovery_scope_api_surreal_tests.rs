//! MT-007 exact-scope recovery API denial and non-mutation proof.

mod surreal_test_store_support;

use handshake_core::storage::surreal::resource_authority::{
    AuthorizationRequest, ResourceAction, ResourceKind,
};
use handshake_core::test_harness::crdt_workspace::AuthenticatedModelLaneTestOperation;

use handshake_core::storage::surreal::{SurrealStorage};
use handshake_core::swarm_orchestration::model_lane::{
    ModelLaneError, ModelLaneRecoveryState, ModelLaneRecoveryStatus, ModelLaneStatus,
    ModelLaneStore, NewModelLaneRecoveryCheckpoint,
};
use handshake_core::swarm_orchestration::resource_scope::{
    AccessSpaceRef, ActorPrincipalId, AuthenticatedSessionRef, ExactResourceScopeAttribution,
    OwnerAccountId, ResourceAccessLifecycleRegistry, ResourceScope, ScopeDenied, WorkspaceScopeRef,
};
use handshake_core::test_harness::crdt_workspace::{
    build_surreal_admissible_crdt_posture, SurrealAdmissibleCrdtPosture,
};
use serde_json::json;
use surreal_test_store_support::IsolatedSurrealTestStore;

// Inputs come from production HTTP session exchange and workspace activation.
async fn execute_http_authenticated_recovery(
    storage: &SurrealStorage,
    exact: &ExactResourceScopeAttribution,
    session_token: &str,
    channel_binding_hash: &str,
    run_id: &str,
) -> Result<handshake_core::swarm_orchestration::model_lane::ModelLaneRecoveredRun, ModelLaneError> {
    let operation = AuthenticatedModelLaneTestOperation::authorize(
        storage.clone(),
        AuthorizationRequest {
            session_token: session_token.to_owned(),
            channel_binding_hash: Some(channel_binding_hash.to_owned()),
            capability_id: "model_lane.recovery.execute".to_owned(),
            resource_kind: ResourceKind::Workspace,
            external_resource_id: exact.workspace_id.as_str().to_owned(),
            action: ResourceAction::Update,
        },
        exact.clone(),
    )
    .await?;
    let scope = handshake_core::api::account_scope::ProductLocalResourceScope::from_exact(exact.clone())
        .map_err(|error| ModelLaneError::AuthorityDenied(error.to_string()))?;
    let store = ModelLaneStore::new_scoped_with_lifecycle(
        storage.clone(),
        scope.resource_scope(),
        storage.resource_access_lifecycle(),
    );
    operation.run(store.recover_run_after_restart(run_id)).await
}

async fn read_http_authenticated_recovery_state(
    storage: &SurrealStorage,
    exact: &ExactResourceScopeAttribution,
    session_token: &str,
    channel_binding_hash: &str,
    run_id: &str,
) -> Result<(
    handshake_core::swarm_orchestration::model_lane::ModelLaneReplay,
    Vec<handshake_core::swarm_orchestration::model_lane::ModelLaneScopedAuthorityReceipt>,
), ModelLaneError> {
    let operation = AuthenticatedModelLaneTestOperation::authorize(
        storage.clone(),
        AuthorizationRequest {
            session_token: session_token.to_owned(),
            channel_binding_hash: Some(channel_binding_hash.to_owned()),
            capability_id: "fs.read".to_owned(),
            resource_kind: ResourceKind::Workspace,
            external_resource_id: exact.workspace_id.as_str().to_owned(),
            action: ResourceAction::Read,
        },
        exact.clone(),
    )
    .await?;
    let scope = handshake_core::api::account_scope::ProductLocalResourceScope::from_exact(exact.clone())
        .map_err(|error| ModelLaneError::AuthorityDenied(error.to_string()))?;
    let store = ModelLaneStore::new_scoped_with_lifecycle(
        storage.clone(),
        scope.resource_scope(),
        storage.resource_access_lifecycle(),
    );
    operation.run(async {
        let replay = store.replay_run(run_id).await?;
        let receipts = store.test_scoped_authority_receipts(run_id, 256).await?;
        Ok((replay, receipts))
    }).await
}

#[cfg(feature = "app-runtime")]
async fn prepare_http_recovery_workspace(
    client: &reqwest::Client, base_url: &str, channel_token: &str,
    account_name: &str, password: &str, workspace_name: &str,
    retained_session: &mut Option<String>, deadline: std::time::Instant,
) -> Result<handshake_core::api::account_scope::ProductLocalResourceScope,
    Box<dyn std::error::Error + Send + Sync>> {
    use reqwest::Method;
    let request = |method, path: &str| {
        client.request(method, format!("{}{path}", base_url.trim_end_matches('/')))
            .header("x-hsk-channel-binding-token", channel_token)
            .timeout(deadline.saturating_duration_since(std::time::Instant::now()))
    };
    let setup: serde_json::Value = request(Method::GET, "/authority/setup")
        .send().await?.error_for_status()?.json().await?;
    let credentials = json!({"account_name": account_name, "password": password});
    match setup.get("setup_required").and_then(serde_json::Value::as_bool) {
        Some(true) => {
            request(Method::POST, "/authority/setup").json(&credentials)
                .send().await?.error_for_status()?;
        }
        Some(false) => {}
        None => return Err(std::io::Error::other("missing actual setup status").into()),
    }
    let login: serde_json::Value = request(Method::POST, "/authority/login")
        .json(&credentials).send().await?.error_for_status()?.json().await?;
    let field = |value: &serde_json::Value, key: &str| {
        value.get(key).and_then(serde_json::Value::as_str)
            .filter(|value| !value.is_empty()).map(str::to_owned)
            .ok_or_else(|| std::io::Error::other(format!("missing actual response field {key}")))
    };
    let session: serde_json::Value = request(Method::POST, "/authority/session").json(&json!({
        "account_id": field(&login, "account_id")?,
        "principal_id": field(&login, "principal_id")?,
        "access_space_id": field(&login, "access_space_id")?,
        "authentication_token": field(&login, "token")?,
    })).send().await?.error_for_status()?.json().await?;
    let session_token = field(&session, "session_token")?;
    *retained_session = Some(session_token.clone());
    let authenticated = |method, path: &str| {
        request(method, path).header("x-hsk-session-token", &session_token)
    };
    let workspace: serde_json::Value = authenticated(Method::POST, "/workspaces")
        .json(&json!({"name": workspace_name}))
        .send().await?.error_for_status()?.json().await?;
    let workspace_id = field(&workspace, "id")?;
    let activation: serde_json::Value = authenticated(Method::POST, "/authority/session")
        .json(&json!({"workspace_id": workspace_id, "operation_id": uuid::Uuid::now_v7()}))
        .send().await?.error_for_status()?.json().await?;
    for (actual, expected) in [
        ("owner_account_id", "account_id"),
        ("actor_principal_id", "principal_id"),
        ("authenticated_session_id", "session_id"),
        ("access_space_id", "access_space_id"),
    ] {
        if field(&activation, actual)? != field(&session, expected)? {
            return Err(std::io::Error::other(format!("activation differs from session field {actual}")).into());
        }
    }
    if field(&activation, "workspace_id")? != workspace_id {
        return Err(std::io::Error::other("activation differs from created workspace").into());
    }
    let scope = handshake_core::api::account_scope::ProductLocalResourceScope::from_json(
        &json!({"schema_version": 1, "scope": {
            "owner_account_id": field(&activation, "owner_account_id")?,
            "actor_principal_id": field(&activation, "actor_principal_id")?,
            "authenticated_session_id": field(&activation, "authenticated_session_id")?,
            "access_space_id": field(&activation, "access_space_id")?,
            "workspace_id": workspace_id,
        }}).to_string(),
    )?;
    Ok(scope)
}

#[cfg(feature = "app-runtime")]
async fn run_activated_http_recovery_case(
    state: &handshake_core::AppState, client: &reqwest::Client, base_url: &str,
    channel_token: &str, session_token: &str,
    scope: &handshake_core::api::account_scope::ProductLocalResourceScope,
    launch_request: &handshake_core::swarm_orchestration::operator_chat::OperatorChatLaunchRequest,
    checkpoint_id: &str, deadline: std::time::Instant,
    cancelled: &tokio::sync::watch::Receiver<bool>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    use reqwest::Method;
    use sha2::{Digest, Sha256};
    use handshake_core::swarm_orchestration::operator_chat::OperatorChatLaunched;
    let storage = &state.surreal;
    let workspace_id = scope.exact().workspace_id.as_str().to_owned();
    let authenticated = |method, path: &str| {
        client.request(method, format!("{}{path}", base_url.trim_end_matches('/')))
            .header("x-hsk-channel-binding-token", channel_token)
            .header("x-hsk-session-token", session_token)
            .timeout(deadline.saturating_duration_since(std::time::Instant::now()))
    };
    let exact = scope.exact();
    let channel_hash = hex::encode(Sha256::digest(channel_token.as_bytes()));
    if *cancelled.borrow() {
        return Err(std::io::Error::new(std::io::ErrorKind::Interrupted, "activated recovery consumer cancelled; settlement requires its owner").into());
    }
    let launched: OperatorChatLaunched = authenticated(Method::POST, "/operator-chat/launch")
        .json(launch_request).send().await?.error_for_status()?.json().await?;
    let (replay, _) = read_http_authenticated_recovery_state(
        storage, exact, &session_token, &channel_hash, &launched.run_id,
    ).await?;
    if replay.run.run_id != launched.run_id || replay.run.trace_id != launched.trace_id {
        return Err(std::io::Error::other("canonical replay differs from launched run or trace").into());
    }
    let lane = replay.lanes.iter().find(|lane| lane.lane_id == launched.lane_id)
        .ok_or_else(|| std::io::Error::other("actual launched lane missing from canonical replay"))?;
    let message = replay.messages.iter().filter(|message| message.from_lane_id == lane.lane_id)
        .max_by_key(|message| message.event_ledger_seq)
        .ok_or_else(|| std::io::Error::other("actual launched lane has no canonical message checkpoint source"))?;
    let required_provenance = |value: &Option<String>, name: &str| {
        value.as_ref().filter(|value| !value.is_empty()).cloned()
            .ok_or_else(|| std::io::Error::other(format!("canonical launch lacks {name}")))
    };
    if *cancelled.borrow() {
        return Err(std::io::Error::new(std::io::ErrorKind::Interrupted, "activated recovery consumer cancelled; settlement requires its owner").into());
    }
    let checkpoint = NewModelLaneRecoveryCheckpoint {
        checkpoint_id: checkpoint_id.to_owned(), run_id: replay.run.run_id.clone(),
        lane_id: Some(lane.lane_id.clone()), session_id: lane.session_id.clone(),
        model_session_id: lane.model_session_id.clone(), lane_status: lane.status.clone(),
        checkpoint_status: ModelLaneRecoveryStatus::Checkpointed,
        last_event_ledger_seq: message.event_ledger_seq,
        last_message_id: Some(message.message_id.clone()),
        open_payload_refs: vec![message.payload_ref.clone()], lease_id: None,
        idempotency_scope: format!("recovery:{}", replay.run.run_id),
        recovery_state: lane.recovery_state.clone(), recovery_event_ref: lane.last_recovery_event_ref.clone(),
        event_ledger_stream_id: replay.run.event_ledger_stream_id.clone(),
        work_packet_id: required_provenance(&message.work_packet_id, "work_packet_id")?,
        micro_task_id: required_provenance(&message.micro_task_id, "micro_task_id")?,
        task_board_id: required_provenance(&message.task_board_id, "task_board_id")?,
        owner_session: message.owner_session.clone(), idempotency_key: checkpoint_id.to_owned(),
        created_at_utc: chrono::Utc::now().to_rfc3339(),
        recovery_hint_ref: lane.recovery_hint_ref.clone(),
        diagnostic_payload: json!({"source_message_id": message.message_id,
            "source_event_ledger_event_id": message.event_ledger_event_id}),
    };
    let create = AuthenticatedModelLaneTestOperation::authorize(storage.clone(), AuthorizationRequest {
        session_token: session_token.to_owned(), channel_binding_hash: Some(channel_hash.clone()),
        capability_id: "fs.write".to_owned(), resource_kind: ResourceKind::Workspace,
        external_resource_id: workspace_id, action: ResourceAction::Create,
    }, exact.clone()).await?;
    let store = ModelLaneStore::new_scoped_with_lifecycle(
        storage.clone(), scope.resource_scope(), storage.resource_access_lifecycle(),
    );
    if *cancelled.borrow() {
        return Err(std::io::Error::new(std::io::ErrorKind::Interrupted, "activated recovery consumer cancelled; settlement requires its owner").into());
    }
    let recorded = create.run(store.record_recovery_checkpoint(checkpoint)).await?;
    if *cancelled.borrow() {
        return Err(std::io::Error::new(std::io::ErrorKind::Interrupted, "activated recovery consumer cancelled; settlement requires its owner").into());
    }
    let recovered = execute_http_authenticated_recovery(
        storage, exact, &session_token, &channel_hash, &launched.run_id,
    ).await?;
    if recovered.checkpoint != recorded {
        return Err(std::io::Error::other("recovery differs from recorded checkpoint").into());
    }
    let before = read_http_authenticated_recovery_state(
        storage, exact, &session_token, &channel_hash, &launched.run_id,
    ).await?;
    let path = format!("/swarm/model-lanes/navigation/recovery/{}", launched.run_id);
    if *cancelled.borrow() {
        return Err(std::io::Error::new(std::io::ErrorKind::Interrupted, "activated recovery consumer cancelled; settlement requires its owner").into());
    }
    let projection: serde_json::Value = authenticated(Method::GET, &path)
        .send().await?.error_for_status()?.json().await?;
    if projection["run"]["run_id"] != launched.run_id {
        return Err(std::io::Error::other("HTTP recovery projection differs from launched run").into());
    }
    let after = read_http_authenticated_recovery_state(
        storage, exact, &session_token, &channel_hash, &launched.run_id,
    ).await?;
    if before != after {
        return Err(std::io::Error::other("passive replay or receipts changed around recovery GET").into());
    }
    Ok(())
}

#[cfg(feature = "app-runtime")]
struct RecoveryRuntimeConsumer {
    prepared: std::cell::RefCell<Option<tokio::sync::oneshot::Receiver<(
        handshake_core::api::account_scope::ProductLocalResourceScope, String,
    )>>>,
    result: std::cell::RefCell<Option<tokio::sync::oneshot::Sender<Result<(), String>>>>,
    client: reqwest::Client,
    base_url: String,
    channel_token: String,
    launch_request: handshake_core::swarm_orchestration::operator_chat::OperatorChatLaunchRequest,
    checkpoint_id: String,
    deadline: std::time::Instant,
}

#[cfg(feature = "app-runtime")]
impl handshake_core::backend_runtime::RuntimeConsumer for RecoveryRuntimeConsumer {
    fn consume<'a>(
        &'a self, state: &'a handshake_core::AppState,
        mut cancelled: tokio::sync::watch::Receiver<bool>,
    ) -> handshake_core::backend_runtime::ConsumerFuture<'a> {
        Box::pin(async move {
            let prepared = self.prepared.borrow_mut().take()
                .ok_or_else(|| "recovery activation consumer was already used".to_owned())?;
            if *cancelled.borrow() {
                return Ok(());
            }
            let prepared = tokio::select! {
                result = prepared => match result { Ok(input) => input, Err(_) => return Ok(()) },
                _ = cancelled.changed() => return Ok(()),
            };
            let result = run_activated_http_recovery_case(
                state, &self.client, &self.base_url, &self.channel_token, &prepared.1,
                &prepared.0, &self.launch_request, &self.checkpoint_id, self.deadline, &cancelled,
            ).await.map_err(|error| error.to_string());
            if let Some(sender) = self.result.borrow_mut().take() {
                let _ = sender.send(result.clone());
            }
            result
        })
    }
}

#[cfg(feature = "app-runtime")]
async fn run_composed_http_recovery_case(
    options: handshake_core::backend_runtime::RuntimeOptions,
    client: reqwest::Client, channel_token: String, account_name: String, password: String,
    workspace_name: String,
    launch_request: handshake_core::swarm_orchestration::operator_chat::OperatorChatLaunchRequest,
    checkpoint_id: String, deadline: std::time::Instant,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let address = options.listener.as_ref()
        .ok_or_else(|| std::io::Error::other("owned recovery listener is required"))?.local_addr()?;
    if !address.ip().is_loopback() {
        return Err(std::io::Error::other("recovery proof listener must be owned loopback").into());
    }
    let base_url = format!("http://{address}");
    let (prepared_tx, prepared_rx) = tokio::sync::oneshot::channel();
    let (result_tx, result_rx) = tokio::sync::oneshot::channel();
    let consumer = std::sync::Arc::new(RecoveryRuntimeConsumer {
        prepared: std::cell::RefCell::new(Some(prepared_rx)),
        result: std::cell::RefCell::new(Some(result_tx)),
        client: client.clone(), base_url: base_url.clone(), channel_token: channel_token.clone(),
        launch_request, checkpoint_id, deadline,
    });
    let local = tokio::task::LocalSet::new();
    local.run_until(async move {
        let mut owner = handshake_core::backend_runtime::RuntimeHandle::start(options, Some(consumer));
        let mut session_token = None;
        let case = tokio::time::timeout_at(tokio::time::Instant::from_std(deadline), async {
            let scope = prepare_http_recovery_workspace(
                &client, &base_url, &channel_token, &account_name, &password, &workspace_name,
                &mut session_token, deadline,
            ).await?;
            let token = session_token.as_ref()
                .ok_or_else(|| std::io::Error::other("actual exchanged session is missing"))?.clone();
            prepared_tx.send((scope, token))
                .map_err(|_| std::io::Error::other("activated production consumer is unavailable"))?;
            result_rx.await
                .map_err(|_| std::io::Error::other("recovery consumer result is unavailable"))?
                .map_err(|error| -> Box<dyn std::error::Error + Send + Sync> { std::io::Error::other(error).into() })
        }).await;
        let mut outcome: Result<(), Box<dyn std::error::Error + Send + Sync>> = match case {
            Ok(result) => result,
            Err(error) => Err(std::io::Error::new(std::io::ErrorKind::TimedOut,
                format!("recovery deadline elapsed; operation/session settlement UNKNOWN: {error}")).into()),
        };
        if let Some(token) = session_token {
            let logout = tokio::time::timeout_at(tokio::time::Instant::from_std(deadline), async {
                client.post(format!("{base_url}/authority/logout"))
                    .header("x-hsk-channel-binding-token", &channel_token)
                    .header("x-hsk-session-token", &token)
                    .timeout(deadline.saturating_duration_since(std::time::Instant::now()))
                    .send().await?.error_for_status()?;
                let denied = client.get(format!("{base_url}/authority/session"))
                    .header("x-hsk-channel-binding-token", &channel_token)
                    .header("x-hsk-session-token", &token)
                    .timeout(deadline.saturating_duration_since(std::time::Instant::now()))
                    .send().await?;
                if denied.status() != reqwest::StatusCode::FORBIDDEN {
                    return Err(std::io::Error::other("logged-out session remained readable").into());
                }
                Ok::<(), Box<dyn std::error::Error + Send + Sync>>(())
            }).await;
            let cleanup_error = match logout {
                Ok(Ok(())) => None,
                Ok(Err(error)) => Some(format!("session logout failed: {error}")),
                Err(error) => Some(format!("logout/settlement UNKNOWN: {error}")),
            };
            if let Some(cleanup) = cleanup_error {
                let message = match outcome {
                    Err(error) => format!("recovery case failed: {error}; {cleanup}"),
                    Ok(()) => cleanup,
                };
                outcome = Err(std::io::Error::other(message).into());
            }
        }
        if let Err(shutdown) = owner.shutdown_at(tokio::time::Instant::from_std(deadline)).await {
            let case_error = outcome.err().map(|error| error.to_string());
            let settlement = if owner.retirement_settled() {
                "runtime retirement settled; awaiting owner join"
            } else {
                "runtime settlement UNKNOWN; awaiting retained owner"
            };
            if let Some(error) = case_error.as_deref() {
                eprintln!("recovery case failed: {error}; {settlement}: {shutdown}");
            } else {
                eprintln!("{settlement}: {shutdown}");
            }
            // This keeps the LocalSet polled and the same live owner retained.
            let joined = owner.join_retained_owner().await;
            let settlement = if owner.retirement_settled() {
                "runtime retirement settled; owner joined"
            } else {
                "runtime settlement UNKNOWN; owner terminated"
            };
            let message = match case_error {
                Some(error) => format!("recovery case failed: {error}; {settlement}: {shutdown}"),
                None => format!("{settlement}: {shutdown}"),
            };
            let message = match joined {
                Ok(()) => message,
                Err(error) => format!("{message}; owner terminal failure: {error}"),
            };
            return Err(std::io::Error::other(message).into());
        }
        outcome
    }).await
}

#[tokio::test]
async fn recovery_denies_each_scope_mismatch_stale_revoked_and_incomplete_context() {
    let mut isolated = IsolatedSurrealTestStore::create()
        .await
        .expect("allocate recovery scope API database");
    let storage = isolated.storage().clone();
    let scope = exact_scope("scope-api");
    let owner_lifecycle = active_lifecycle(&scope);
    let owner =
        ModelLaneStore::new_scoped_with_lifecycle(storage.clone(), scope.clone(), owner_lifecycle);
    let posture = build_surreal_admissible_crdt_posture(
        &owner,
        scope.workspace.as_ref().expect("workspace").as_str(),
        "scope-api",
    )
    .await
    .expect("seed production posture");
    let message = owner
        .record_message(posture.message.clone())
        .await
        .expect("record message");
    owner
        .record_recovery_checkpoint(checkpoint(&posture, &message))
        .await
        .expect("record checkpoint");
    let before = owner
        .test_scoped_authority_receipts(&posture.run_id, 64)
        .await
        .expect("baseline receipts");

    for foreign_scope in one_field_mismatches(&scope) {
        let foreign_lifecycle = active_lifecycle(&foreign_scope);
        let foreign = ModelLaneStore::new_scoped_with_lifecycle(
            storage.clone(),
            foreign_scope,
            foreign_lifecycle,
        );
        assert!(foreign
            .recover_run_after_restart(&posture.run_id)
            .await
            .is_err());
        assert!(foreign
            .recover_restartable_runs_at_boot()
            .await
            .expect("foreign boot scan is non-leaking")
            .is_empty());
        assert!(foreign
            .test_scoped_authority_receipts(&posture.run_id, 64)
            .await
            .expect("foreign receipt scan is non-leaking")
            .is_empty());
    }

    let exact = ExactResourceScopeAttribution::try_from_resource_scope(&scope)
        .expect("exact lifecycle tuple");
    let stale_lifecycle = active_lifecycle(&scope);
    let stale = ModelLaneStore::new_scoped_with_lifecycle(
        storage.clone(),
        scope.clone(),
        stale_lifecycle.clone(),
    );
    stale_lifecycle
        .mark_stale(&exact)
        .expect("mark exact tuple stale");
    let stale_error = stale
        .recover_run_after_restart(&posture.run_id)
        .await
        .expect_err("stale exact tuple must deny recovery");
    assert!(matches!(
        stale_error,
        ModelLaneError::ScopeDenied(ScopeDenied::LifecycleStale)
    ));

    let revoked_lifecycle = active_lifecycle(&scope);
    let revoked = ModelLaneStore::new_scoped_with_lifecycle(
        storage.clone(),
        scope.clone(),
        revoked_lifecycle.clone(),
    );
    revoked_lifecycle
        .revoke(&exact)
        .expect("revoke exact tuple");
    let revoked_error = revoked
        .recover_run_after_restart(&posture.run_id)
        .await
        .expect_err("revoked exact tuple must deny recovery");
    assert!(matches!(
        revoked_error,
        ModelLaneError::ScopeDenied(ScopeDenied::LifecycleRevoked)
    ));
    assert!(revoked_lifecycle.register_active(exact.clone()).is_err());
    let mut new_session_scope = scope.clone();
    new_session_scope.authenticated_session = Some(AuthenticatedSessionRef::mint());
    let new_session_exact =
        ExactResourceScopeAttribution::try_from_resource_scope(&new_session_scope)
            .expect("new exact session tuple");
    revoked_lifecycle
        .register_active(new_session_exact)
        .expect("a new authenticated session identity may register active");
    let mut incomplete_scope = scope.clone();
    incomplete_scope.workspace = None;
    let incomplete = ModelLaneStore::new_scoped(storage.clone(), incomplete_scope);
    assert!(incomplete
        .recover_run_after_restart(&posture.run_id)
        .await
        .is_err());

    let mut conflict = checkpoint(&posture, &message);
    conflict.session_id.push_str("-conflict");
    assert!(owner.record_recovery_checkpoint(conflict).await.is_err());
    assert_eq!(
        owner
            .test_scoped_authority_receipts(&posture.run_id, 64)
            .await
            .expect("denials preserve receipt count"),
        before
    );
    assert_eq!(
        owner
            .recover_run_after_restart(&posture.run_id)
            .await
            .expect("owner recovery survives denials")
            .checkpoint
            .checkpoint_id,
        "checkpoint-scope-api"
    );

    drop(owner);
    drop(storage);
    isolated
        .cleanup()
        .await
        .expect("cleanup recovery scope API");
}

fn checkpoint(
    posture: &SurrealAdmissibleCrdtPosture,
    message: &handshake_core::swarm_orchestration::model_lane::ModelLaneMessageRecord,
) -> NewModelLaneRecoveryCheckpoint {
    NewModelLaneRecoveryCheckpoint {
        checkpoint_id: "checkpoint-scope-api".into(),
        run_id: posture.run_id.clone(),
        lane_id: Some(posture.lane_id.clone()),
        session_id: posture.session_id.clone(),
        model_session_id: posture.model_session_id.clone(),
        lane_status: ModelLaneStatus::Ready,
        checkpoint_status: ModelLaneRecoveryStatus::Checkpointed,
        last_event_ledger_seq: message.event_ledger_seq,
        last_message_id: Some(message.message_id.clone()),
        open_payload_refs: vec![message.payload_ref.clone()],
        lease_id: None,
        idempotency_scope: format!("recovery:{}:scope-api", posture.run_id),
        recovery_state: ModelLaneRecoveryState::Restartable,
        recovery_event_ref: Some("recovery-event://scope-api".into()),
        event_ledger_stream_id: message.event_ledger_stream_id.clone(),
        work_packet_id: "WP-1-Multi-Model-Orchestration-Lifecycle-Telemetry-v1".into(),
        micro_task_id: "MT-007".into(),
        task_board_id: "task-board://wp-1".into(),
        owner_session: message.owner_session.clone(),
        idempotency_key: "idem-checkpoint-scope-api".into(),
        created_at_utc: "2026-09-02T00:10:00Z".into(),
        recovery_hint_ref: Some("usermanual://model-lane/recovery".into()),
        diagnostic_payload: json!({"authority": "kernel_event_ledger"}),
    }
}

fn exact_scope(label: &str) -> ResourceScope {
    ResourceScope::new(OwnerAccountId::mint(), ActorPrincipalId::mint())
        .with_session(AuthenticatedSessionRef::mint())
        .with_access_space(AccessSpaceRef::mint())
        .with_workspace(WorkspaceScopeRef::new(format!("workspace-{label}")).expect("workspace"))
}

fn active_lifecycle(scope: &ResourceScope) -> ResourceAccessLifecycleRegistry {
    let lifecycle = ResourceAccessLifecycleRegistry::new();
    lifecycle
        .register_active(
            ExactResourceScopeAttribution::try_from_resource_scope(scope).expect("exact scope"),
        )
        .expect("register active exact scope");
    lifecycle
}

fn one_field_mismatches(scope: &ResourceScope) -> Vec<ResourceScope> {
    let mut owner = scope.clone();
    owner.owner_account_id = OwnerAccountId::mint();
    let mut actor = scope.clone();
    actor.actor_principal_id = ActorPrincipalId::mint();
    let mut session = scope.clone();
    session.authenticated_session = Some(AuthenticatedSessionRef::mint());
    let mut access = scope.clone();
    access.access_space = Some(AccessSpaceRef::mint());
    let mut workspace = scope.clone();
    workspace.workspace = Some(WorkspaceScopeRef::new("workspace-foreign").expect("workspace"));
    vec![owner, actor, session, access, workspace]
}
