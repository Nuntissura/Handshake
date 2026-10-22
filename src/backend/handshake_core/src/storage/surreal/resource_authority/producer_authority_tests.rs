use super::*;
use axum::{body::{to_bytes, Body}, http::{Request, StatusCode}, Router};
use crate::api::MountedRequestExt;
use crate::api::workspace_activation_readiness::{MountedApi, MountedApiState};
use crate::process_ledger::{
    production_process_sandbox_registry_async, LedgerDrainJoinOutcome, ProcessReclaimRuntime,
    ReclaimResourceScope,
};
use crate::storage::tests::{embedded_test_backend, EmbeddedTestBackend};
use crate::AccountBootstrapState;
use crate::model_runtime::cloud::secrets_vault::{OsKeychainSecretsVault, SecretsVault, SecretsVaultError};
use crate::swarm_orchestration::resource_scope::{AccessSpaceRef, ActorPrincipalId, AuthenticatedSessionRef, OwnerAccountId, ResourceScope, WorkspaceScopeRef};
use crate::swarm_orchestration::model_lane::{ModelLaneLocusBinding, ModelLaneRecoveryState, ModelLaneRunCanonicalSet, ModelLaneRunRecord, ModelLaneStore, NewModelLaneRun};
use futures::FutureExt;
use serde_json::{json, Value};
use std::{ffi::OsString, future::Future, panic::AssertUnwindSafe, pin::Pin, sync::Arc, time::Duration};
use tokio::sync::{mpsc, RwLock};
use uuid::Uuid;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error + Send + Sync>>;
type Case<'a> = Pin<Box<dyn Future<Output = TestResult> + 'a>>;
const BOUND: Duration = Duration::from_secs(30);

// Core LIB execution must be isolated from every other native-binding environment mutator.
struct Fixture {
    backend: Option<EmbeddedTestBackend>,
    bootstrap: AccountBootstrapState,
    router: Router,
    activation: mpsc::Receiver<crate::api::authority::WorkspaceActivationRequest>,
    mounted: MountedApiState,
    channel: String,
    previous_binding: Option<OsString>,
    binding_installed: bool,
    session: Option<Value>,
    extra_sessions: Vec<(String, String)>,
    logged_out_sessions: Vec<String>,
    session_exchange_uncertain: bool,
    workspace: Option<String>,
    runtime: Option<ProcessReclaimRuntime>,
    runtime_start_uncertain: bool,
    runtime_drained: bool,
    settlement: Option<tokio::task::JoinHandle<Result<(), (StatusCode, axum::Json<Value>)>>>,
    revocation: Option<tokio::task::JoinHandle<Result<(), ResourceAuthorityError>>>,
    revocation_uncertain: bool,
    prepared_latch: Option<Arc<ActivationPreparedTestLatch>>,
    original_operation: Option<Uuid>,
    activation_unsettled: bool,
    producer_run: Option<ProducerRunRecovery>,
}

type ProducerRunInvocation = Pin<Box<dyn Future<Output = ProducerRunCompletion>>>;

struct ProducerRunRecovery {
    capture: Arc<ProducerSourceFactsCapture>,
    expected: NewModelLaneRun,
    baseline: ModelLaneRunCanonicalSet,
    canonical: Option<ModelLaneRunCanonicalSet>,
    invocation: Option<ProducerRunInvocation>,
    outcome: Option<ProducerRunCompletion>,
    transaction_acknowledged: bool,
    panic_payload: Option<Box<dyn std::any::Any + Send>>,
}

#[derive(surrealdb::types::SurrealValue)]
struct OpenRows {
    runtime: Uuid, account: String, principal: String, session: String, space: String, workspace: String,
}

impl Fixture {
    async fn open() -> TestResult<Self> {
        let backend = embedded_test_backend().await?;
        let bootstrap = AccountBootstrapState::new(
            backend.database.clone(), backend.storage.clone(),
            Arc::new(crate::capabilities::CapabilityRegistry::new()),
        );
        let (tx, activation) = mpsc::channel(4);
        let mounted = Arc::new(RwLock::new(None));
        let router = crate::api::authority::bootstrap_routes(bootstrap.clone(), tx)
            .merge(crate::api::workspaces::bootstrap_routes(bootstrap.clone()))
            .layer(axum::Extension(mounted.clone()));
        Ok(Self {
            backend: Some(backend), bootstrap, router, activation, mounted,
            channel: hex::encode(Sha256::digest(Uuid::now_v7().as_bytes())),
            previous_binding: None, binding_installed: false, session: None, extra_sessions: Vec::new(),
            logged_out_sessions: Vec::new(),
            session_exchange_uncertain: false,
            workspace: None, runtime: None, runtime_start_uncertain: false, runtime_drained: false,
            settlement: None, revocation: None, revocation_uncertain: false,
            prepared_latch: None, original_operation: None,
            activation_unsettled: false, producer_run: None,
        })
    }

    async fn authenticate(&mut self) -> TestResult {
        let path = self.backend.as_ref().unwrap().data_dir.join("producer-native-binding.json");
        std::fs::write(&path, serde_json::to_vec(
            &crate::api::stage::current_process_native_session_binding(&self.channel),
        )?)?;
        self.previous_binding = std::env::var_os("HANDSHAKE_STAGE_BINDING_FILE");
        std::env::set_var("HANDSHAKE_STAGE_BINDING_FILE", path);
        self.binding_installed = true;
        let credentials = json!({
            "account_name": "isolated producer authority operator",
            "password": "isolated producer authority password without egress",
        });
        let (status, setup) = self.http("POST", "/authority/setup", None, credentials.clone()).await?;
        assert_eq!(status, StatusCode::OK, "{setup}");
        let (status, login) = self.http("POST", "/authority/login", None, credentials).await?;
        assert_eq!(status, StatusCode::OK, "{login}");
        self.session_exchange_uncertain = true;
        let (status, session) = self.http("POST", "/authority/session", None, json!({
            "account_id": field(&login, "account_id")?,
            "principal_id": field(&login, "principal_id")?,
            "access_space_id": field(&login, "access_space_id")?,
            "authentication_token": field(&login, "token")?,
        })).await?;
        assert_eq!(status, StatusCode::OK, "{session}");
        self.session = Some(session);
        self.session_exchange_uncertain = false;
        let actual = self.bootstrap.surreal.authenticate_local_session(
            self.token(), &self.channel_hash(),
        ).await?;
        assert_eq!(actual.session_id, field(self.session.as_ref().unwrap(), "session_id")?);
        self.workspace = Some(self.create_workspace().await?);
        Ok(())
    }

    fn token(&self) -> &str { self.session.as_ref().unwrap()["session_token"].as_str().unwrap() }
    fn channel_hash(&self) -> String { sha256_hex(self.channel.as_bytes()) }
    fn credentials(&self) -> RoleAuthorityAuthInput {
        RoleAuthorityAuthInput::new(self.token().to_owned(), self.channel_hash())
    }
    fn request(&self, workspace: &str, capability: &str, action: ResourceAction) -> AuthorizationRequest {
        AuthorizationRequest {
            session_token: self.token().to_owned(), channel_binding_hash: Some(self.channel_hash()),
            capability_id: capability.to_owned(), resource_kind: ResourceKind::Workspace,
            external_resource_id: workspace.to_owned(), action,
        }
    }
    async fn http(&self, method: &str, path: &str, token: Option<&str>, value: Value)
        -> TestResult<(StatusCode, Value)>
    {
        http(&self.router, &self.channel, method, path, token, value).await
    }
    async fn create_workspace(&self) -> TestResult<String> {
        let (status, value) = self.http("POST", "/workspaces", Some(self.token()),
            json!({"name": format!("isolated-{}", Uuid::now_v7())})).await?;
        assert_eq!(status, StatusCode::CREATED, "{value}");
        Ok(field(&value, "id")?.to_owned())
    }
    async fn another_durable_session(&mut self) -> TestResult<String> {
        let (status, credential) = self.http("POST", "/authority/login", None, json!({
            "account_name": "isolated producer authority operator",
            "password": "isolated producer authority password without egress",
        })).await?;
        assert_eq!(status, StatusCode::OK, "{credential}");
        self.session_exchange_uncertain = true;
        let (status, issued) = self.http("POST", "/authority/session", None, json!({
            "account_id": field(&credential, "account_id")?, "principal_id": field(&credential, "principal_id")?,
            "access_space_id": field(&credential, "access_space_id")?, "authentication_token": field(&credential, "token")?,
        })).await?;
        assert_eq!(status, StatusCode::OK, "{issued}");
        let token = field(&issued, "session_token")?.to_owned();
        let session_id = field(&issued, "session_id")?.to_owned();
        self.extra_sessions.push((token.clone(), session_id));
        self.session_exchange_uncertain = false;
        assert_ne!(field(&issued, "session_id")?, field(self.session.as_ref().unwrap(), "session_id")?);
        self.bootstrap.surreal.authenticate_local_session(&token, &self.channel_hash()).await?;
        Ok(token)
    }
    async fn observe(&self, operation: Uuid) -> TestResult<(StatusCode, Value)> {
        self.http("GET", &format!("/authority/workspaces/{}/activation/{operation}",
            self.workspace.as_ref().unwrap()), Some(self.token()), Value::Null).await
    }
    async fn canonical(&self, operation: Uuid) -> TestResult<Value> {
        self.bootstrap.surreal.read_activation_and_publish(
            self.token(), &self.channel_hash(), operation, self.workspace.as_ref().unwrap(),
            &self.bootstrap.capability_registry, |observation| {
                Ok(match observation.outcome {
                    None => Value::Null,
                    Some(stored) => json!({
                        "envelope": stored.envelope(),
                        "result": stored.applied().map(|applied| applied.result()),
                        "witness": stored.applied().map(|applied| applied.publication_witness()),
                    }),
                })
            },
        ).await.map_err(|error| match error {
            ActivationObservationError::Authentication => "canonical authenticated read: authentication failed".into(),
            ActivationObservationError::NotFound => "canonical authenticated read: activation not found".into(),
            ActivationObservationError::Unavailable => "canonical authenticated read: activation unavailable".into(),
        })
    }
    async fn start_runtime(&mut self, request: &crate::api::authority::WorkspaceActivationRequest) -> TestResult {
        let exact = request.scope().exact();
        let reclaim = ReclaimResourceScope::try_from_stored(
            &exact.owner_account_id.to_string(), &exact.actor_principal_id.to_string(),
            &exact.authenticated_session_id.to_string(), exact.workspace_id.as_str(),
            &exact.access_space_id.to_string(),
        )?;
        self.runtime_start_uncertain = true;
        let runtime = ProcessReclaimRuntime::production(
            self.bootstrap.surreal.clone(), reclaim, None,
            production_process_sandbox_registry_async().await?,
            format!("producer-authority-{}", Uuid::now_v7()), BOUND,
        ).await?;
        self.runtime = Some(runtime);
        self.runtime_drained = false;
        self.runtime_start_uncertain = false;
        Ok(())
    }
    async fn assert_zero_open_rows(&self, runtime: &ProcessReclaimRuntime) -> TestResult {
        let session = self.session.as_ref().unwrap();
        let bindings = OpenRows {
            runtime: runtime.runtime_instance().instance_id,
            account: field(session, "account_id")?.into(),
            principal: field(session, "principal_id")?.into(),
            session: field(session, "session_id")?.into(),
            space: field(session, "access_space_id")?.into(),
            workspace: self.workspace.clone().unwrap(),
        };
        let count = self.bootstrap.surreal.with_data_operation(move |database| Box::pin(async move {
            database.query_first::<i64, _>(
                "RETURN array::len(SELECT VALUE id FROM kernel_process_lifecycle WHERE stopped_at = NONE AND owner_runtime_instance_id = $runtime AND owner_account_id = $account AND actor_principal_id = $principal AND authenticated_session_id = $session AND access_space_id = $space AND workspace_id = $workspace);",
                bindings,
            ).await
        })).await?.ok_or("missing independent exact-scope open-row result")?;
        assert_eq!(count, 0, "original runtime still owns open exact-scope lifecycle rows");
        Ok(())
    }
    fn producer_store(&self, capture: &ProducerSourceFactsCapture, source_index: usize) -> TestResult<ModelLaneStore> {
        let source = capture.sources.get(source_index).ok_or("missing original producer source")?;
        let decision = &source.decision;
        let scope = ResourceScope::new(
            OwnerAccountId::from_uuid(Uuid::parse_str(&decision.account_id)?),
            ActorPrincipalId::from_uuid(Uuid::parse_str(&decision.principal_id)?),
        )
        .with_session(AuthenticatedSessionRef::from_uuid(Uuid::parse_str(&decision.session_id)?))
        .with_access_space(AccessSpaceRef::from_uuid(Uuid::parse_str(&decision.access_space_id)?))
        .with_workspace(WorkspaceScopeRef::new(source.request.external_resource_id.clone())?);
        Ok(ModelLaneStore::new_scoped_with_lifecycle(
            self.bootstrap.surreal.clone(), scope, capture.lifecycle.clone(),
        ))
    }

    async fn start_producer_run(
        &mut self,
        capture: Arc<ProducerSourceFactsCapture>,
        expected: NewModelLaneRun,
    ) -> TestResult {
        if self.producer_run.is_some() {
            return Err("original producer operation still owns the fixture".into());
        }
        let store = self.producer_store(&capture, 0)?;
        let storage = self.bootstrap.surreal.clone();
        let baseline = storage.replay_producer_source_run(&capture, 1, &store, &expected).await?;
        if baseline.run.is_some() {
            return Err("original producer run already exists before its Create invocation".into());
        }
        let worker_capture = Arc::clone(&capture);
        let input = expected.clone();
        let invocation = Box::pin(async move {
            storage.record_producer_source_run(&worker_capture, 0, store, input).await
        });
        // Own the original future and fence before its first poll. A canceled
        // borrowing phase leaves both here for the same fixture's cleanup.
        self.producer_run = Some(ProducerRunRecovery {
            capture, expected, baseline, canonical: None, invocation: Some(invocation), outcome: None,
            transaction_acknowledged: false, panic_payload: None,
        });
        Ok(())
    }

    async fn await_producer_run(&mut self) -> TestResult {
        let recovery = self.producer_run.as_mut().ok_or("missing original producer operation")?;
        if recovery.panic_payload.is_some() {
            return Err("original producer invocation panicked; retain complete fixture".into());
        }
        if let Some(invocation) = recovery.invocation.as_mut() {
            let result = AssertUnwindSafe(invocation).catch_unwind().await;
            // No await separates receipt of the original outcome from custody.
            // A caught panic consumes this invocation too; never poll it again.
            match result {
                Ok(outcome) => recovery.outcome = Some(outcome),
                Err(payload) => recovery.panic_payload = Some(payload),
            }
            recovery.invocation = None;
            if recovery.panic_payload.is_some() {
                return Err("original producer invocation panicked; retain complete fixture".into());
            }
        }
        Ok(())
    }

    fn compare_producer_canonical_set(
        recovery: &ProducerRunRecovery,
        observed: &ModelLaneRunCanonicalSet,
        committed: Option<&ModelLaneRunRecord>,
    ) {
        let baseline = &recovery.baseline;
        // This fixture creates only Runs. Nonzero runtime families, including
        // any lease state or orphan, cannot be accepted as unchanged cleanup.
        assert_eq!([
            baseline.lanes.len(), baseline.messages.len(), baseline.checkpoints.len(),
            baseline.recovery_events.len(), baseline.leases.len(),
        ], [0; 5], "original scope baseline has unresolved runtime records; retain complete fixture");
        assert_eq!([
            observed.lanes.len(), observed.messages.len(), observed.checkpoints.len(),
            observed.recovery_events.len(), observed.leases.len(),
        ], [0; 5], "original scope has unresolved runtime records; retain complete fixture");
        match committed {
            Some(committed) => {
                let run = observed.run.as_ref().expect("committed original run is missing");
                assert_eq!(run.inner, recovery.expected);
                assert_eq!(run, committed);
                assert!(!run.event_ledger_event_id.is_empty());
            }
            None => assert!(observed.run.is_none(), "failed invocation has an observed target run; retain complete fixture"),
        }
        // Every full raw row in all six families remains in these snapshots,
        // including unmatched rows and every lease state. Remove only the one
        // genuine returned Run when comparing a successful Create's delta.
        let target_rows = observed.scope_records.iter().filter(|(kind, row)| {
            committed.is_some() && kind.as_str() == "run"
                && row.aggregate_id == recovery.expected.run_id && row.run_id == recovery.expected.run_id
        }).collect::<Vec<_>>();
        assert_eq!(target_rows.len(), usize::from(committed.is_some()));
        if let Some(committed) = committed {
            assert_eq!(target_rows[0].1.event_id, committed.event_ledger_event_id);
            assert_eq!(target_rows[0].1.event_seq, committed.event_ledger_seq);
        }
        let prior_rows = observed.scope_records.iter().filter(|row| !target_rows.contains(row)).collect::<Vec<_>>();
        assert_eq!(prior_rows.len(), baseline.scope_records.len());
        for row in &baseline.scope_records {
            assert_eq!(prior_rows.iter().filter(|candidate| **candidate == row).count(),
                baseline.scope_records.iter().filter(|candidate| *candidate == row).count(),
                "canonical scope records changed outside the original Create delta");
        }
        assert_eq!(observed.other_runs.len(), baseline.other_runs.len());
        assert_eq!(observed.lanes.len(), baseline.lanes.len());
        assert_eq!(observed.messages.len(), baseline.messages.len());
        assert_eq!(observed.checkpoints.len(), baseline.checkpoints.len());
        assert_eq!(observed.recovery_events.len(), baseline.recovery_events.len());
        assert_eq!(observed.leases.len(), baseline.leases.len());
        let expected_event = committed.map(|run| run.event_ledger_event_id.as_str());
        let new_events = observed.ordered_ledger.iter().filter(|entry| Some(entry.event_id.as_str()) == expected_event)
            .collect::<Vec<_>>();
        assert_eq!(new_events.len(), usize::from(committed.is_some()));
        if let Some(committed) = committed { assert_eq!(new_events[0].event_seq, committed.event_ledger_seq); }
        let before_ledger = baseline.ordered_ledger.iter().map(|entry| (
            entry.event_id.as_str(), entry.event_seq, entry.event_stream_version, entry.transaction_seq,
            entry.aggregate_id.as_str(), entry.run_id.as_str(), entry.session_run_id.as_str(),
            entry.kernel_task_run_id.as_str(), entry.payload_hash.as_str(), entry.event_payload_json.as_str(),
        )).collect::<Vec<_>>();
        let after_ledger = observed.ordered_ledger.iter().filter(|entry| Some(entry.event_id.as_str()) != expected_event)
            .map(|entry| (
                entry.event_id.as_str(), entry.event_seq, entry.event_stream_version, entry.transaction_seq,
                entry.aggregate_id.as_str(), entry.run_id.as_str(), entry.session_run_id.as_str(),
                entry.kernel_task_run_id.as_str(), entry.payload_hash.as_str(), entry.event_payload_json.as_str(),
            )).collect::<Vec<_>>();
        assert_eq!(after_ledger, before_ledger, "complete ordered canonical ledger changed outside original Create");
    }

    async fn reconcile_producer_run(&mut self) -> TestResult {
        if self.producer_run.is_none() { return Ok(()); }
        self.await_producer_run().await?;
        let recovery = self.producer_run.as_ref().unwrap();
        let store = self.producer_store(&recovery.capture, 1)?;
        let observed = self.bootstrap.surreal.replay_producer_source_run(
            &recovery.capture, 1, &store, &recovery.expected,
        ).await?;
        self.producer_run.as_mut().unwrap().canonical = Some(observed);
        let recovery = self.producer_run.as_ref().unwrap();
        let completion = recovery.outcome.as_ref()
            .ok_or("original producer outcome is unknown; retain complete fixture")?;
        let committed = match &completion.result {
            Ok(committed) => Some(committed),
            Err(failure) => {
                if failure.original_operation_id != recovery.capture.operation_id {
                    return Err("producer outcome differs from original operation; retain complete fixture".into());
                }
                None
            }
        };
        Self::compare_producer_canonical_set(recovery, recovery.canonical.as_ref().unwrap(), committed);
        let recovery = self.producer_run.as_mut().unwrap();
        let completion = recovery.outcome.as_mut().unwrap();
        // This consumes only the matching original transaction finalization
        // proof. Callback origin, cancel success, or absence cannot substitute.
        if !recovery.transaction_acknowledged {
            if !self.bootstrap.surreal.acknowledge_producer_run_completion(&recovery.capture, completion)? {
                return Err("original producer finalization is unproved; retain complete fixture".into());
            }
            // Record the actual consumed-proof step before any later phase.
            recovery.transaction_acknowledged = true;
        }
        if self.bootstrap.surreal.transaction_settlement_pending(recovery.capture.operation_id) {
            return Err("original producer Create remains unsettled after proof acknowledgment".into());
        }
        self.producer_run = None;
        Ok(())
    }

    fn assert_session_vault_absent(&self, session_id: &str) -> TestResult {
        let config = self.bootstrap.surreal.config();
        let identity = serde_json::to_vec(&(
            config.path().to_string_lossy(), config.namespace(), config.database(),
        ))?;
        let installation = hex::encode(Sha256::digest(identity));
        let vault = OsKeychainSecretsVault::new(format!("handshake-local-accounts-{installation}"));
        match vault.get(session_id) {
            Err(SecretsVaultError::NoSecretForLane(key)) if key == session_id => Ok(()),
            Ok(secret) => {
                drop(secret);
                Err("original session credential remains in OS vault; retain complete fixture".into())
            }
            Err(SecretsVaultError::KeychainBackend(_)) => {
                Err("OS keychain read failed; credential absence is unverified".into())
            }
            Err(_) => Err("original session credential absence is unverified".into()),
        }
    }

    async fn cleanup(&mut self) -> TestResult {
        if let Some(latch) = &self.prepared_latch { latch.release(); }
        if let Some(settlement) = self.settlement.as_mut() {
            let result = settlement.await;
            self.settlement.take();
            if matches!(result, Ok(Ok(()))) { self.activation_unsettled = false; }
        }
        if let Some(revocation) = self.revocation.as_mut() {
            let result = revocation.await;
            self.revocation.take();
            if matches!(&result, Ok(Ok(()))) { self.revocation_uncertain = false; }
            result??;
        }
        if self.activation_unsettled {
            if let Some(operation) = self.original_operation {
                let canonical = self.canonical(operation).await?;
                if !canonical["result"].is_null() && !canonical["witness"].is_null()
                    && !self.bootstrap.surreal.transaction_settlement_pending(operation)
                {
                    self.activation_unsettled = false;
                }
            }
        }
        if self.activation_unsettled || self.runtime_start_uncertain || self.session_exchange_uncertain
            || self.revocation_uncertain || self.settlement.is_some() {
            return Err("original activation owner is unresolved; retain complete fixture".into());
        }
        self.reconcile_producer_run().await?;
        if self.session.is_some() {
            let sessions = self.extra_sessions.iter().cloned()
                .chain(std::iter::once((self.token().to_owned(),
                    field(self.session.as_ref().unwrap(), "session_id")?.to_owned())))
                .collect::<Vec<_>>();
            for (token, session_id) in sessions {
                if !self.logged_out_sessions.contains(&token) {
                    let (status, value) = self.http("POST", "/authority/logout", Some(&token), Value::Null).await?;
                    assert_eq!(status, StatusCode::OK, "{value}");
                    // Preserve successful logout before the independent OS read.
                    self.logged_out_sessions.push(token);
                }
                // Recheck even previously logged-out sessions on every retry.
                self.assert_session_vault_absent(&session_id)?;
            }
        }
        *self.mounted.write().await = None;
        if let Some(runtime) = &self.runtime {
            if !self.runtime_drained {
                let report = runtime.shutdown_and_drain(BOUND).await;
                assert!(report.reclaim_task_quiesced, "{report:?}");
                assert!(matches!(&report.ledger, LedgerDrainJoinOutcome::Flushed | LedgerDrainJoinOutcome::AlreadyDrained));
                assert!(report.lease_released, "{report:?}");
                assert!(report.lease_retained_reason.is_none(), "{report:?}");
                self.runtime_drained = true;
            }
            self.assert_zero_open_rows(runtime).await?;
        }
        self.runtime = None;
        self.prepared_latch = None;
        if self.binding_installed {
            match self.previous_binding.take() {
                Some(value) => std::env::set_var("HANDSHAKE_STAGE_BINDING_FILE", value),
                None => std::env::remove_var("HANDSHAKE_STAGE_BINDING_FILE"),
            }
            self.binding_installed = false;
        }
        let backend = self.backend.as_ref().unwrap();
        // The canonical helper owns only another reference to the SAME storage.
        // Preserve the complete original backend/frame even if close/removal fails.
        crate::storage::tests::shutdown_and_remove_test_store(
            self.bootstrap.surreal.clone(), backend.data_dir.clone(),
        ).await?;
        assert!(!backend.data_dir.try_exists()?, "original backend directory survived cleanup");
        Ok(())
    }
}

struct OriginalFixtureFrame {
    runtime: tokio::runtime::Runtime,
    fixture: Fixture,
    finalized: bool,
}

impl OriginalFixtureFrame {
    fn finalize(&mut self) {
        while !self.finalized {
            let cleanup = std::panic::catch_unwind(AssertUnwindSafe(|| {
                self.runtime.block_on(AssertUnwindSafe(self.fixture.cleanup()).catch_unwind())
            }));
            if matches!(&cleanup, Ok(Ok(Ok(())))) {
                self.finalized = true;
                break;
            }
            // Diagnostics must not unwind past the owner of unresolved state.
            let _ = std::panic::catch_unwind(AssertUnwindSafe(|| {
                match &cleanup {
                    Ok(Ok(Err(error))) => eprintln!("original fixture remains pending: {error}"),
                    _ => eprintln!("original fixture remains pending after cleanup panic"),
                }
            }));
            std::thread::sleep(BOUND);
        }
    }
}

impl Drop for OriginalFixtureFrame {
    fn drop(&mut self) {
        // Any exit hands the SAME fixture and runtime to actual finalization;
        // unresolved state never falls through to field destructors.
        self.finalize();
    }
}

fn run_case(body: impl for<'a> FnOnce(&'a mut Fixture) -> Case<'a>) -> TestResult {
    let runtime = tokio::runtime::Builder::new_multi_thread().enable_all().build()?;
    let fixture = runtime.block_on(Fixture::open())?;
    let mut owner = OriginalFixtureFrame { runtime, fixture, finalized: false };
    let result = owner.runtime.block_on(AssertUnwindSafe(async {
        owner.fixture.authenticate().await?;
        body(&mut owner.fixture).await
    }).catch_unwind());
    owner.finalize();
    match result {
        Ok(result) => result,
        Err(payload) => std::panic::resume_unwind(payload),
    }
}

fn field<'a>(value: &'a Value, key: &str) -> TestResult<&'a str> {
    value[key].as_str().ok_or_else(|| format!("missing canonical {key}").into())
}

async fn http(router: &Router, channel: &str, method: &str, path: &str,
    token: Option<&str>, value: Value) -> TestResult<(StatusCode, Value)>
{
    let mut request = Request::builder().method(method).uri(path)
        .header("content-type", "application/json")
        .header(crate::api::stage::HSK_HEADER_CHANNEL_BINDING_TOKEN, channel);
    if let Some(token) = token { request = request.header("x-hsk-session-token", token); }
    let response = router.clone().oneshot(request.body(Body::from(serde_json::to_vec(&value)?))?).await?;
    let status = response.status();
    let bytes = to_bytes(response.into_body(), 1024 * 1024).await?;
    Ok((status, serde_json::from_slice(&bytes)?))
}

#[test]
fn producer_source_capture_keeps_ordered_real_grants_and_rejects_invalid_pairs() -> TestResult {
    run_case(|fixture| Box::pin(async move {
        let second = fixture.create_workspace().await?;
        let requests = vec![
            fixture.request(fixture.workspace.as_ref().unwrap(), "fs.read", ResourceAction::Read),
            fixture.request(&second, "fs.write", ResourceAction::Create),
        ];
        let operation = Uuid::now_v7();
        let captured = fixture.bootstrap.surreal.capture_producer_source_facts(
            operation, &fixture.bootstrap.capability_registry, fixture.credentials(), requests.clone(),
        ).await?;
        assert_eq!(captured.operation_id, operation);
        assert_eq!(captured.sources.len(), 2);
        assert_ne!(captured.sources[0].authority.resource_id, captured.sources[1].authority.resource_id);
        for (source, request) in captured.sources.iter().zip(&requests) {
            assert_eq!(&source.request, request);
            assert_eq!(source.authority.session_id, RecordId::new("authenticated_sessions", captured.session.session_id.clone()));
            assert_eq!(source.authority.account_id, RecordId::new("local_accounts", captured.session.identity.account_id.clone()));
            assert_eq!(source.authority.delegation_chain, captured.session.delegation_chain);
            assert!(!source.decision.grant_id.is_empty());
            assert_eq!(source.authority.grant_id, RecordId::new("resource_grants", source.decision.grant_id.clone()));
        }
        fixture.bootstrap.surreal.revalidate_producer_source_facts(&captured).await?;
        drop(captured);
        assert!(fixture.bootstrap.surreal.capture_producer_source_facts(
            Uuid::now_v7(), &fixture.bootstrap.capability_registry, fixture.credentials(), vec![],
        ).await.is_err());
        for mut request in requests {
            request.capability_id = "ungranted.producer.capability".into();
            assert!(fixture.bootstrap.surreal.capture_producer_source_facts(
                Uuid::now_v7(), &fixture.bootstrap.capability_registry, fixture.credentials(), vec![request],
            ).await.is_err());
        }
        let mut mixed = fixture.request(&second, "fs.read", ResourceAction::Read);
        mixed.session_token = Uuid::now_v7().to_string();
        assert!(fixture.bootstrap.surreal.capture_producer_source_facts(
            Uuid::now_v7(), &fixture.bootstrap.capability_registry, fixture.credentials(), vec![mixed],
        ).await.is_err());
        let foreign = fixture.request(&Uuid::now_v7().to_string(), "fs.read", ResourceAction::Read);
        assert!(fixture.bootstrap.surreal.capture_producer_source_facts(
            Uuid::now_v7(), &fixture.bootstrap.capability_registry, fixture.credentials(), vec![foreign],
        ).await.is_err());
        let run_operation = Uuid::now_v7();
        let current = Arc::new(fixture.bootstrap.surreal.capture_producer_source_facts(
            run_operation, &fixture.bootstrap.capability_registry, fixture.credentials(),
            vec![
                fixture.request(&second, "fs.write", ResourceAction::Create),
                fixture.request(&second, "fs.read", ResourceAction::Read),
            ],
        ).await?);
        let session_id = current.session.session_id.clone();
        let run_id = run_operation.to_string();
        let expected = NewModelLaneRun {
            run_id: run_id.clone(), trace_id: run_id.clone(), run_span_id: run_id.clone(),
            coordinator_session_id: session_id.clone(), routing_policy: "RoundRobin".into(),
            context_bundle_id: second.clone(), lane_ids: vec![format!("{run_id}-lane")],
            event_ledger_stream_id: run_id.clone(), artifact_namespace: run_id.clone(),
            projection_plan_ref: None, consent_receipt_ref: None,
            work_packet_id: Some("producer-authority-fixture".into()),
            micro_task_id: Some("existing-create-consumer".into()),
            task_board_id: Some("producer-authority-fixture".into()),
            owner_session: session_id.clone(), idempotency_key: run_id.clone(),
            replay_order_key: run_id.clone(), replay_after_event_ledger_seq: None,
            recovery_state: ModelLaneRecoveryState::Restartable, failstate_code: None,
            reason_ref: None, recovery_hint_ref: None,
            locus_binding: Some(ModelLaneLocusBinding {
                work_packet_id: "producer-authority-fixture".into(),
                micro_task_id: "existing-create-consumer".into(),
                task_board_id: Some("producer-authority-fixture".into()),
                coordinator_session_id: session_id.clone(), session_id: session_id.clone(),
                model_session_id: session_id.clone(), owner_session: session_id,
                locus_binding_ref: run_id.clone(),
            }),
            memory_pack_ref: run_id.clone(), memory_pack_hash: sha256_hex(run_id.as_bytes()),
            determinism_mode: "deterministic".into(), budget_summary_ref: run_id.clone(),
            selected_model_id: None, candidate_model_ids: vec!["producer-authority-fixture-model".into()],
            procedural_review_status: "pending".into(), truncation_warning_ref: None,
            rejection_reason_refs: Vec::new(),
        };
        let storage = fixture.bootstrap.surreal.clone();
        let store = fixture.producer_store(&current, 0)?;
        let (authority, subject) = storage.producer_source_run_authority(
            &current, 0, &store, ResourceAction::Create,
        ).await?;
        let decision = &current.sources[0].decision;
        let scope = crate::storage::surreal::ModelLaneScope {
            owner_account_id: decision.account_id.clone(),
            actor_principal_id: decision.principal_id.clone(),
            authenticated_session_id: decision.session_id.clone(),
            access_space_id: decision.access_space_id.clone(),
            workspace_id: second.clone(),
        };
        let provider = crate::storage::surreal::model_lane::SurrealModelLaneStore::initialize(
            storage.clone(),
        ).await?;
        assert!(provider.observe_current_context_guards().is_none());
        let callback_provider = provider.clone();
        crate::storage::surreal::with_model_lane_subject(subject, storage.with_record_user_scope(
            authority,
            provider.with_context_bundle_transaction(
                crate::storage::surreal::ContextCoordinationPurpose::RunCreate,
                expected.run_id.clone(), expected.lane_ids.clone(), scope,
                move || {
                    let constructed = callback_provider.observe_current_context_guards()
                        .expect("production guards must precede callback future construction");
                    assert!(constructed.belongs_to(&callback_provider));
                    async move {
                        let polled = callback_provider.observe_current_context_guards()
                            .expect("production guards must remain installed at first poll");
                        assert!(polled.belongs_to(&callback_provider));
                        assert!(constructed.same_guards_and_owner(&polled));
                        drop(polled);
                        drop(constructed);
                        Ok::<(), SurrealStorageError>(())
                    }
                },
            ),
        )).await?;
        assert!(provider.observe_current_context_guards().is_none());
        fixture.start_producer_run(Arc::clone(&current), expected.clone()).await?;
        drop(current);
        fixture.await_producer_run().await?;
        let completion = fixture.producer_run.as_ref().unwrap().outcome.as_ref()
            .ok_or("original producer Create outcome is unknown")?;
        let stored = match &completion.result {
            Ok(stored) => stored,
            Err(failure) => {
                let message = format!("original producer Create failed: {failure}");
                fixture.reconcile_producer_run().await?;
                return Err(message.into());
            }
        };
        assert_eq!(stored.inner, expected);
        assert!(!stored.event_ledger_event_id.is_empty());
        fixture.reconcile_producer_run().await?;

        let failed_operation = Uuid::now_v7();
        let failed_capture = Arc::new(fixture.bootstrap.surreal.capture_producer_source_facts(
            failed_operation, &fixture.bootstrap.capability_registry, fixture.credentials(),
            vec![
                fixture.request(&second, "fs.write", ResourceAction::Create),
                fixture.request(&second, "fs.read", ResourceAction::Read),
            ],
        ).await?);
        let mut invalid = expected;
        invalid.run_id = failed_operation.to_string();
        invalid.idempotency_key = failed_operation.to_string();
        invalid.candidate_model_ids.clear();
        fixture.start_producer_run(Arc::clone(&failed_capture), invalid).await?;
        drop(failed_capture);
        fixture.await_producer_run().await?;
        let recovery = fixture.producer_run.as_ref().unwrap();
        let completion = recovery.outcome.as_ref().ok_or("missing original callback outcome")?;
        let failure = completion.result.as_ref()
            .expect_err("empty model candidates must fail inside the genuine Create callback");
        assert_eq!(failure.original_operation_id, recovery.capture.operation_id);
        assert!(matches!(
            &failure.origin,
            ProducerRunFailureOrigin::OriginalCallback(
                crate::swarm_orchestration::model_lane::ModelLaneError::InvalidInput(message),
            ) if message == "candidate_model_ids must contain at least one model id"
        ));
        fixture.reconcile_producer_run().await?;
        assert!(fixture.producer_run.is_none(),
            "proved original rollback retained its transaction obligation");
        Ok(())
    }))
}

#[test]
fn producer_source_capture_serializes_revocation_without_widening_other_grants() -> TestResult {
    run_case(|fixture| Box::pin(async move {
        let second = fixture.create_workspace().await?;
        let first_request = fixture.request(fixture.workspace.as_ref().unwrap(), "fs.read", ResourceAction::Read);
        let other_request = fixture.request(&second, "fs.read", ResourceAction::Read);
        let captured = fixture.bootstrap.surreal.capture_producer_source_facts(
            Uuid::now_v7(), &fixture.bootstrap.capability_registry, fixture.credentials(), vec![first_request.clone()],
        ).await?;
        let grant = captured.sources[0].decision.grant_id.clone();
        let storage = fixture.bootstrap.surreal.clone();
        fixture.revocation_uncertain = true;
        fixture.revocation = Some(tokio::spawn(async move {
            storage.revoke_grant(&grant).await
        }));
        tokio::time::timeout(BOUND, async {
            loop {
                assert!(!fixture.revocation.as_ref().unwrap().is_finished(), "revocation escaped held capture");
                // Tokio's fair RwLock blocks new readers behind a queued writer.
                // The capture still owns its read guard, so this cannot be an acquired writer.
                if fixture.bootstrap.surreal.inner.authority_publication_gate.try_read().is_err() { break; }
                tokio::task::yield_now().await;
            }
        }).await?;
        fixture.bootstrap.surreal.revalidate_producer_source_facts(&captured).await?;
        drop(captured);
        let completed = tokio::time::timeout(BOUND, fixture.revocation.as_mut().unwrap()).await?;
        fixture.revocation.take();
        completed??;
        fixture.revocation_uncertain = false;
        assert!(fixture.bootstrap.surreal.capture_producer_source_facts(
            Uuid::now_v7(), &fixture.bootstrap.capability_registry, fixture.credentials(), vec![first_request],
        ).await.is_err());
        let unrelated = fixture.bootstrap.surreal.capture_producer_source_facts(
            Uuid::now_v7(), &fixture.bootstrap.capability_registry, fixture.credentials(), vec![other_request],
        ).await?;
        fixture.bootstrap.surreal.revalidate_producer_source_facts(&unrelated).await?;
        drop(unrelated);
        Ok(())
    }))
}

#[test]
fn producer_source_capture_revalidates_real_grant_expiry_without_mutating_held_authority() -> TestResult {
    run_case(|fixture| Box::pin(async move {
        let session = fixture.bootstrap.surreal.authenticate_local_session(fixture.token(), &fixture.channel_hash()).await?;
        let external = Uuid::now_v7().to_string();
        let resource = fixture.bootstrap.surreal.register_workspace_resource(&session.identity, &external).await?;
        fixture.bootstrap.surreal.grant_resource(
            &session.identity.account_id, &session.identity.access_space_id,
            ResourceGrantSpec {
                principal_id: session.identity.principal_id.clone(), resource_id: resource.resource_id,
                actions: vec![ResourceAction::Read], capability_ids: vec!["fs.read".into()],
                expires_at: Some(Utc::now() + chrono::Duration::seconds(30)),
                delegation_chain: session.delegation_chain.clone(),
            },
        ).await?;
        let captured = fixture.bootstrap.surreal.capture_producer_source_facts(
            Uuid::now_v7(), &fixture.bootstrap.capability_registry, fixture.credentials(),
            vec![fixture.request(&external, "fs.read", ResourceAction::Read)],
        ).await?;
        let expiry = captured.sources[0].authority.grant_expires_at.clone().ok_or("actual grant expiry missing")?.into_inner();
        let remaining = (expiry - Utc::now()).to_std().unwrap_or_default();
        tokio::time::sleep(remaining + Duration::from_millis(100)).await;
        assert!(fixture.bootstrap.surreal.revalidate_producer_source_facts(&captured).await.is_err());
        drop(captured);
        Ok(())
    }))
}

#[test]
fn workspace_activation_observation_http_preserves_original_uuid_prepared_unknown_applied_and_scope_nonmutation() -> TestResult {
    run_case(|fixture| Box::pin(async move {
        let operation = Uuid::now_v7();
        fixture.original_operation = Some(operation);
        assert_eq!(fixture.observe(operation).await?.0, StatusCode::SERVICE_UNAVAILABLE,
            "genuine missing lifecycle authorization must fail closed");
        let latch = fixture.bootstrap.surreal.install_activation_prepared_test_latch(operation, BOUND)?;
        fixture.prepared_latch = Some(latch.clone());
        let router = fixture.router.clone();
        let channel = fixture.channel.clone();
        let token = fixture.token().to_owned();
        let workspace = fixture.workspace.clone().unwrap();
        let mut post = Box::pin(http(&router, &channel, "POST", "/authority/session", Some(&token),
            json!({"workspace_id": workspace, "operation_id": operation})));
        let request = tokio::select! {
            result = &mut post => panic!("activation returned before real request dispatch: {result:?}"),
            request = fixture.activation.recv() => request.ok_or("activation receiver closed")?,
        };
        assert_eq!(request.operation_id(), operation);
        let before = fixture.observe(operation).await?;
        assert_eq!(before.0, StatusCode::OK);
        let initial: handshake_account_activation::WorkspaceActivationObservation<Value> = serde_json::from_value(before.1.clone())?;
        assert!(matches!(initial.outcome, handshake_account_activation::WorkspaceActivationOutcome::Unresolved));
        assert!(matches!(initial.readiness, handshake_account_activation::WorkspaceActivationReadiness::Unavailable));
        assert_eq!(initial.operation_id, operation);
        assert_eq!(initial.authenticated_session_id, field(fixture.session.as_ref().unwrap(), "session_id")?);
        assert_eq!(initial.owner_account_id, field(fixture.session.as_ref().unwrap(), "account_id")?);
        assert_eq!(initial.actor_principal_id, field(fixture.session.as_ref().unwrap(), "principal_id")?);
        assert_eq!(initial.access_space_id, field(fixture.session.as_ref().unwrap(), "access_space_id")?);
        assert_eq!(initial.workspace_id, fixture.workspace.as_ref().unwrap().as_str());
        assert_eq!(fixture.canonical(operation).await?, Value::Null);
        request.authorize_for_publication().await.map_err(|error| format!("publication authorization: {error:?}"))?;
        fixture.start_runtime(&request).await?;
        let runtime = fixture.runtime.as_ref().unwrap().clone();
        let descriptor = runtime.runtime_instance().clone();
        let actual_descriptor = serde_json::to_value(&descriptor)?;
        let exact = request.scope().exact().clone();
        let mounted = fixture.mounted.clone();
        let bootstrap = fixture.bootstrap.clone();
        let actual_router = fixture.router.clone();
        fixture.activation_unsettled = true;
        fixture.settlement = Some(tokio::spawn(async move {
            request.acknowledge_with(&descriptor, || {
                let mut slot = mounted.try_write().map_err(|_| (StatusCode::SERVICE_UNAVAILABLE, axum::Json(json!({"error":"mount_busy"}))))?;
                *slot = Some(MountedApi::new(actual_router, runtime, bootstrap, exact));
                Ok(())
            }).await
        }));
        tokio::select! {
            result = &mut post => panic!("activation ACK escaped committed Prepared latch: {result:?}"),
            result = latch.wait_prepared() => result?,
        }
        let prepared_first = fixture.observe(operation).await?;
        let prepared_second = fixture.observe(operation).await?;
        assert_eq!(prepared_first.0, StatusCode::OK);
        assert_eq!(prepared_first, prepared_second, "GET must not mutate canonical Prepared");
        let prepared: handshake_account_activation::WorkspaceActivationObservation<Value> = serde_json::from_value(prepared_first.1.clone())?;
        assert_eq!(prepared.operation_id, operation);
        assert!(matches!(prepared.outcome, handshake_account_activation::WorkspaceActivationOutcome::Unresolved));
        assert!(matches!(prepared.readiness, handshake_account_activation::WorkspaceActivationReadiness::Unavailable));
        let original_prepared = fixture.canonical(operation).await?;
        assert_eq!(original_prepared["envelope"]["operation_id"], operation.to_string());
        assert_eq!(original_prepared["result"], Value::Null);
        assert_eq!(original_prepared["witness"], Value::Null);
        assert_eq!(original_prepared, fixture.canonical(operation).await?);
        // Lose the HTTP ACK while the actual committed Prepared owner remains held.
        drop(post);
        assert_eq!(prepared_first, fixture.observe(operation).await?);
        latch.release();
        let completed = tokio::time::timeout(BOUND, fixture.settlement.as_mut().unwrap()).await?;
        fixture.settlement.take();
        completed?.map_err(|error| format!("activation settlement uncertain: {error:?}"))?;
        fixture.activation_unsettled = false;
        let applied = fixture.observe(operation).await?;
        assert_eq!(applied.0, StatusCode::OK);
        let observed: handshake_account_activation::WorkspaceActivationObservation<Value> = serde_json::from_value(applied.1.clone())?;
        let handshake_account_activation::WorkspaceActivationOutcome::Applied { result, publication_witness } = observed.outcome
            else { return Err("real protected publication did not produce canonical Applied".into()); };
        let handshake_account_activation::WorkspaceActivationReadiness::Current { descriptor } = observed.readiness
            else { return Err("real protected Mounted publication is unavailable".into()); };
        assert_eq!(descriptor, actual_descriptor);
        assert_eq!(result.operation_id, operation);
        assert_eq!(result.owner_account_id, initial.owner_account_id);
        assert_eq!(result.actor_principal_id, initial.actor_principal_id);
        assert_eq!(result.authenticated_session_id, initial.authenticated_session_id);
        assert_eq!(result.access_space_id, initial.access_space_id);
        assert_eq!(result.workspace_id, initial.workspace_id);
        assert_eq!(publication_witness.operation_id, operation);
        assert_eq!(publication_witness.actual_runtime, actual_descriptor);
        assert_eq!(serde_json::to_value(&publication_witness.original_envelope)?, original_prepared["envelope"]);
        let canonical_applied = fixture.canonical(operation).await?;
        assert_eq!(canonical_applied["envelope"], original_prepared["envelope"]);
        assert_eq!(canonical_applied["witness"], applied.1["outcome"]["publication_witness"]);
        assert_eq!(canonical_applied["result"]["operation_id"], operation.to_string());
        let expected = handshake_account_activation::WorkspaceActivationExpected {
            operation_id: operation, owner_account_id: &initial.owner_account_id,
            actor_principal_id: &initial.actor_principal_id, authenticated_session_id: &initial.authenticated_session_id,
            access_space_id: &initial.access_space_id, workspace_id: &initial.workspace_id,
        };
        handshake_account_activation::parse_workspace_activation_ack(canonical_applied["result"].clone(), &expected)
            .map_err(|error| format!("canonical ACK: {error:?}"))?;
        assert_eq!(applied, fixture.observe(operation).await?, "GET Applied remains nonmutating");
        let other_workspace = fixture.create_workspace().await?;
        let wrong_scope = fixture.http("GET", &format!("/authority/workspaces/{other_workspace}/activation/{operation}"),
            Some(fixture.token()), Value::Null).await?;
        assert_eq!(wrong_scope.0, StatusCode::NOT_FOUND);
        let other_session = fixture.another_durable_session().await?;
        let foreign_session = fixture.http("GET", &format!("/authority/workspaces/{workspace}/activation/{operation}"),
            Some(&other_session), Value::Null).await?;
        assert_eq!(foreign_session.0, StatusCode::NOT_FOUND, "valid other durable session cannot read original outcome");
        let invalid = fixture.http("GET", &format!("/authority/workspaces/{workspace}/activation/{operation}"),
            Some("invalid-session-token"), Value::Null).await?;
        assert!(matches!(invalid.0, StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN));
        assert_eq!(applied, fixture.observe(operation).await?);
        assert_eq!(canonical_applied, fixture.canonical(operation).await?);
        // Close only the drained runtime before reopening the SAME durable store/session.
        *fixture.mounted.write().await = None;
        let report = fixture.runtime.as_ref().unwrap().shutdown_and_drain(BOUND).await;
        assert!(report.reclaim_task_quiesced && report.lease_released, "{report:?}");
        assert!(matches!(&report.ledger, LedgerDrainJoinOutcome::Flushed | LedgerDrainJoinOutcome::AlreadyDrained));
        fixture.runtime_drained = true;
        fixture.assert_zero_open_rows(fixture.runtime.as_ref().unwrap()).await?;
        fixture.runtime = None;
        let reopened = fixture.backend.as_ref().unwrap().reopen_storage().await?;
        fixture.bootstrap.surreal = reopened;
        let (tx, receiver) = mpsc::channel(4);
        fixture.activation = receiver;
        fixture.router = crate::api::authority::bootstrap_routes(fixture.bootstrap.clone(), tx)
            .merge(crate::api::workspaces::bootstrap_routes(fixture.bootstrap.clone()))
            .layer(axum::Extension(fixture.mounted.clone()));
        assert_eq!(fixture.observe(operation).await?.0, StatusCode::SERVICE_UNAVAILABLE);
        // Reacquire authorization only through the genuine existing POST, after canonical
        // Applied was independently observed before close; preserve the SAME original UUID.
        let router = fixture.router.clone();
        let mut replay = Box::pin(http(&router, &channel, "POST", "/authority/session", Some(&token),
            json!({"workspace_id": workspace, "operation_id": operation})));
        let request = tokio::select! {
            result = &mut replay => panic!("reopen returned before genuine activation dispatch: {result:?}"),
            request = fixture.activation.recv() => request.ok_or("reopened activation receiver closed")?,
        };
        assert_eq!(request.operation_id(), operation);
        fixture.start_runtime(&request).await?;
        let runtime = fixture.runtime.as_ref().unwrap().clone();
        let descriptor = runtime.runtime_instance().clone();
        let mounted = fixture.mounted.clone();
        let bootstrap = fixture.bootstrap.clone();
        let actual_router = fixture.router.clone();
        let exact = request.scope().exact().clone();
        fixture.activation_unsettled = true;
        fixture.settlement = Some(tokio::spawn(async move {
            request.acknowledge_with(&descriptor, || {
                let mut slot = mounted.try_write().map_err(|_| (StatusCode::SERVICE_UNAVAILABLE, axum::Json(json!({"error":"mount_busy"}))))?;
                *slot = Some(MountedApi::new(actual_router, runtime, bootstrap, exact));
                Ok(())
            }).await
        }));
        let completed = tokio::time::timeout(BOUND, fixture.settlement.as_mut().unwrap()).await?;
        fixture.settlement.take();
        completed?.map_err(|error| format!("reopen Applied reconciliation: {error:?}"))?;
        fixture.activation_unsettled = false;
        let replay = tokio::time::timeout(BOUND, replay).await??;
        assert_eq!(replay.0, StatusCode::OK);
        handshake_account_activation::parse_workspace_activation_ack(replay.1, &expected)
            .map_err(|error| format!("reopened original ACK: {error:?}"))?;
        let reopened = fixture.observe(operation).await?;
        assert_eq!(reopened.0, StatusCode::OK);
        let reopened: handshake_account_activation::WorkspaceActivationObservation<Value> = serde_json::from_value(reopened.1)?;
        assert_eq!(reopened.operation_id, operation);
        assert!(matches!(reopened.readiness, handshake_account_activation::WorkspaceActivationReadiness::Unavailable));
        let handshake_account_activation::WorkspaceActivationOutcome::Applied { publication_witness: historical, .. } = reopened.outcome
            else { return Err("canonical Applied was lost on same-session reopen".into()); };
        assert_eq!(historical.operation_id, publication_witness.operation_id);
        assert_eq!(historical.original_envelope, publication_witness.original_envelope);
        assert_eq!(historical.actual_runtime, publication_witness.actual_runtime);
        assert_eq!(canonical_applied, fixture.canonical(operation).await?, "entire canonical envelope/result/witness survives reopen");
        Ok(())
    }))
}
