use crate::model_runtime::cloud::secrets_vault::{
    load_member_enrollment_recipient_credential,
    MemberEnrollmentRecipientCredential,
    MemberRecipientCredentialError,
};
use crate::storage::surreal::{SurrealStorage, SurrealStorageError};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

#[derive(Debug, thiserror::Error)]
pub(crate) enum MemberRecipientStartupError {
    #[error("member recipient provisioning schema initialization failed")]
    Schema(#[source] SurrealStorageError),
    #[error("member recipient credential initialization failed")]
    Credential(#[source] MemberRecipientCredentialError),
}

pub(crate) struct MemberRecipientStartupReady<R, E> {
    credential: Arc<MemberEnrollmentRecipientCredential>,
    recipient: Result<R, E>,
}

impl<R, E> MemberRecipientStartupReady<R, E> {
    pub(crate) fn credential(&self) -> &Arc<MemberEnrollmentRecipientCredential> {
        &self.credential
    }

    pub(crate) fn recipient(&self) -> Result<&R, &E> {
        self.recipient.as_ref()
    }
}

type MemberRecipientStartupOutcome<R, E> =
    Result<MemberRecipientStartupReady<R, E>, MemberRecipientStartupError>;

type MemberRecipientStartupFuture<R, E> =
    Pin<Box<dyn Future<Output = MemberRecipientStartupOutcome<R, E>> + 'static>>;

pub(crate) enum MemberRecipientStartupObservation<'a, R, E> {
    Pending,
    Complete(&'a MemberRecipientStartupOutcome<R, E>),
}

#[must_use]
pub(crate) struct MemberRecipientStartupBuilder<R, E> {
    _storage: SurrealStorage,
    initialization: Option<MemberRecipientStartupFuture<R, E>>,
    outcome: Option<MemberRecipientStartupOutcome<R, E>>,
}

impl<R: 'static, E: 'static> MemberRecipientStartupBuilder<R, E> {
    pub(crate) fn new<F, FactoryFuture>(
        storage: SurrealStorage,
        factory: F,
    ) -> Self
    where
        F: FnOnce(Arc<MemberEnrollmentRecipientCredential>) -> FactoryFuture + 'static,
        FactoryFuture: Future<Output = Result<R, E>> + 'static,
    {
        let retained_storage = storage.clone();
        let initialization = Box::pin(async move {
            storage.install_member_recipient_provisioning_schema().await
                .map_err(MemberRecipientStartupError::Schema)?;
            let credential = load_member_enrollment_recipient_credential(&storage).await
                .map_err(MemberRecipientStartupError::Credential)?;
            let recipient = factory(Arc::clone(&credential)).await;
            Ok(MemberRecipientStartupReady { credential, recipient })
        });
        Self {
            _storage: retained_storage,
            initialization: Some(initialization),
            outcome: None,
        }
    }

    pub(crate) fn ready(&self) -> Option<&MemberRecipientStartupReady<R, E>> {
        self.outcome.as_ref().and_then(|outcome| outcome.as_ref().ok())
    }

    pub(crate) async fn observe(&mut self) -> &MemberRecipientStartupOutcome<R, E> {
        if let Some(initialization) = self.initialization.as_mut() {
            let outcome = initialization.as_mut().await;
            self.outcome = Some(outcome);
            self.initialization = None;
        }
        self.outcome.as_ref().expect("initializer or original outcome is always retained")
    }

    pub(crate) async fn observe_until(
        &mut self,
        original_deadline: tokio::time::Instant,
    ) -> MemberRecipientStartupObservation<'_, R, E> {
        match tokio::time::timeout_at(original_deadline, self.observe()).await {
            Ok(outcome) => MemberRecipientStartupObservation::Complete(outcome),
            Err(_) => MemberRecipientStartupObservation::Pending,
        }
    }
}

pub(crate) type MemberRecipientStartup = MemberRecipientStartupBuilder<
    Arc<crate::storage::surreal::resource_authority::MemberRecipientRedemptionRuntime>,
    crate::storage::surreal::resource_authority::ResourceAuthorityError,
>;

pub(crate) fn member_recipient_startup(storage: SurrealStorage) -> MemberRecipientStartup {
    let factory_storage = storage.clone();
    MemberRecipientStartupBuilder::new(storage, move |credential| async move {
        crate::storage::surreal::resource_authority::MemberRecipientRedemptionRuntime::new(
            factory_storage,
            credential,
        ).map(Arc::new)
    })
}

use crate::storage::surreal::resource_authority::{
    MemberRecipientFailure, MemberRecipientOperationRequest, MemberRecipientOperationResult,
    MemberRecipientOutcome, MemberRecipientRedemptionController, MemberRecipientRedemptionRuntime,
};

type RecipientReport = Vec<Arc<Result<MemberRecipientOutcome, MemberRecipientFailure>>>;
type RecipientOperationReport = Vec<Arc<MemberRecipientOperationResult>>;

struct RecipientShutdown {
    requests: Vec<Arc<MemberRecipientOperationRequest>>,
    controllers: Vec<Arc<MemberRecipientRedemptionController>>,
}

pub(super) struct MemberRecipientLifecycle {
    runtime: Arc<MemberRecipientRedemptionRuntime>,
    shutdown: std::sync::Mutex<Option<RecipientShutdown>>,
    operation_reports: tokio::sync::Mutex<Vec<RecipientOperationReport>>,
    reports: tokio::sync::Mutex<Vec<RecipientReport>>,
}

impl MemberRecipientLifecycle {
    pub(super) fn new(runtime: Arc<MemberRecipientRedemptionRuntime>) -> Self {
        Self {
            runtime,
            shutdown: std::sync::Mutex::new(None),
            operation_reports: tokio::sync::Mutex::new(Vec::new()),
            reports: tokio::sync::Mutex::new(Vec::new()),
        }
    }

    pub(super) fn begin_shutdown(&self) {
        let mut shutdown = self.shutdown.lock().unwrap_or_else(|error| error.into_inner());
        if shutdown.is_none() {
            let controllers = self.runtime.begin_shutdown();
            let requests = self.runtime.retained_operation_requests();
            *shutdown = Some(RecipientShutdown { requests, controllers });
        }
    }

    pub(super) async fn drain(&self) -> bool {
        self.begin_shutdown();
        let (requests, controllers) = {
            let shutdown = self.shutdown.lock().unwrap_or_else(|error| error.into_inner());
            let shutdown = shutdown.as_ref().expect("shutdown inventory is retained");
            (shutdown.requests.clone(), shutdown.controllers.clone())
        };
        let operation_report = futures::future::join_all(requests.iter().map(|owner| owner.wait())).await;
        let operations_settled = operation_report.iter().all(|outcome| outcome.as_ref().is_ok());
        self.operation_reports.lock().await.push(operation_report);
        let report = futures::future::join_all(controllers.iter().map(|owner| owner.wait())).await;
        let settled = operations_settled && report.iter().all(|outcome| outcome.as_ref().is_ok());
        self.reports.lock().await.push(report);
        settled
    }
}
use crate::storage::surreal::resource_authority::{
    MemberRevocationResult, MemberRevocationRuntime, MemberRevocationSeedResult, MemberRevocationShutdownInventory,
};

type RevocationShutdown = Result<MemberRevocationShutdownInventory, crate::storage::surreal::resource_authority::ResourceAuthorityError>;
type RevocationReport = Vec<Arc<MemberRevocationResult>>;
type RevocationSeedReport = Vec<Arc<MemberRevocationSeedResult>>;

pub(super) struct MemberRevocationLifecycle {
    runtime: Arc<MemberRevocationRuntime>,
    shutdown: std::sync::Mutex<Option<RevocationShutdown>>,
    seed_reports: tokio::sync::Mutex<Vec<RevocationSeedReport>>,
    reports: tokio::sync::Mutex<Vec<RevocationReport>>,
}

impl MemberRevocationLifecycle {
    pub(super) fn new(runtime: Arc<MemberRevocationRuntime>) -> Self {
        Self {
            runtime,
            shutdown: std::sync::Mutex::new(None),
            seed_reports: tokio::sync::Mutex::new(Vec::new()),
            reports: tokio::sync::Mutex::new(Vec::new()),
        }
    }

    pub(super) fn begin_shutdown(&self) {
        let mut shutdown = self.shutdown.lock().unwrap_or_else(|error| error.into_inner());
        if shutdown.is_none() {
            *shutdown = Some(self.runtime.begin_shutdown());
        }
    }

    pub(super) async fn drain(&self) -> bool {
        self.begin_shutdown();
        let inventory = {
            let shutdown = self.shutdown.lock().unwrap_or_else(|error| error.into_inner());
            match shutdown.as_ref().expect("revocation shutdown observation is retained") {
                Ok(inventory) => Some((inventory.seeds.clone(), inventory.effects.clone())),
                Err(_) => None,
            }
        };
        let Some((seeds, effects)) = inventory else { return false; };
        let seed_report = futures::future::join_all(seeds.iter().map(|owner| owner.wait())).await;
        let seeds_settled = seed_report.iter().all(|outcome| outcome.as_ref().is_ok());
        self.seed_reports.lock().await.push(seed_report);
        let report = futures::future::join_all(effects.iter().map(|owner| owner.wait())).await;
        let settled = seeds_settled && report.iter().all(|outcome| outcome.as_ref().is_ok());
        self.reports.lock().await.push(report);
        settled
    }
}
type MemberSessionDrain = Result<(), crate::storage::surreal::resource_authority::ResourceAuthorityError>;

pub(super) struct MemberSessionLifecycle {
    storage: SurrealStorage,
    reports: tokio::sync::Mutex<Vec<(MemberSessionDrain, MemberSessionDrain)>>,
}

impl MemberSessionLifecycle {
    pub(super) fn new(storage: SurrealStorage) -> Self {
        Self { storage, reports: tokio::sync::Mutex::new(Vec::new()) }
    }

    pub(super) async fn drain(&self) -> bool {
        let (password, exchange) = tokio::join!(
            self.storage.drain_member_password_login_operations(),
            self.storage.drain_member_session_exchange_operations(),
        );
        let settled = password.is_ok() && exchange.is_ok();
        self.reports.lock().await.push((password, exchange));
        settled
    }
}
use crate::api::authority::{
    MemberEnrollmentHttpRuntime, MemberEnrollmentHttpShutdownInventory,
    MemberEnrollmentHttpShutdownReport,
};

pub(super) struct MemberEnrollmentHttpLifecycle {
    runtime: Arc<MemberEnrollmentHttpRuntime>,
    shutdown: std::sync::Mutex<Option<Arc<MemberEnrollmentHttpShutdownInventory>>>,
    reports: tokio::sync::Mutex<Vec<Arc<MemberEnrollmentHttpShutdownReport>>>,
}

impl MemberEnrollmentHttpLifecycle {
    pub(super) fn new(runtime: Arc<MemberEnrollmentHttpRuntime>) -> Self {
        Self {
            runtime,
            shutdown: std::sync::Mutex::new(None),
            reports: tokio::sync::Mutex::new(Vec::new()),
        }
    }

    pub(super) fn begin_shutdown(&self) {
        let mut shutdown = self.shutdown.lock().unwrap_or_else(|error| error.into_inner());
        if shutdown.is_none() {
            *shutdown = Some(Arc::new(self.runtime.begin_shutdown()));
        }
    }

    pub(super) async fn drain(&self) -> bool {
        self.begin_shutdown();
        let inventory = {
            let shutdown = self.shutdown.lock().unwrap_or_else(|error| error.into_inner());
            Arc::clone(shutdown.as_ref().expect("HTTP shutdown inventory is retained"))
        };
        let report = inventory.drain().await;
        let complete = report.complete;
        self.reports.lock().await.push(report);
        complete
    }
}