//! WP-1 MT-012: Operator chat/launch work-surface backend engine.
//!
//! This module is the net-new binding the MT-012 contract calls for: it re-homes
//! the WP-KERNEL-004 CLI-wrapper capture ([`AgentActivity`], produced by
//! [`parse_agent_activity_line`]) onto WP-1 ModelLane authority. Every captured
//! *completed* activity block becomes ONE typed [`NewModelLaneMessage`] recorded
//! under a live [`ModelLaneRun`](super::model_lane) via
//! [`ModelLaneStore::record_message`] (EventLedger authority) AND one Flight
//! Recorder `FR-EVT-AGENT-*` business event. The operator's own turn is persisted
//! as a `HUMAN_OPERATOR` ModelLane message.
//!
//! Launch resolves ONLY through sanctioned [`SwarmCoordinator`] authority:
//! process-backed lanes use `spawn_session`, and SUBAGENT uses the no-OS subagent
//! lane helper (both fail closed without a `ModelLaneStore`). Selection of a
//! model/worktree is recorded as a distinct auditable decision through the MT-014
//! [`ModelCatalog::record_selection_decision`] primitive.
//!
//! Message-kind mapping (contract F1, spec 4.3.9.2.5):
//! * `AgentActivity::ToolCall`                 -> `ModelLaneMessageKind::ToolRequest`
//! * `AgentActivity::Text` rendered tool_result -> `ModelLaneMessageKind::ToolResult`
//! * operator prompt / model answer / thought  -> `ModelLaneMessageKind::Status`,
//!   discriminated by `diagnostic_payload.activity_kind`
//!   (`tool_call|thinking|text|other`, from [`AgentActivityKind::label`]).

use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

use chrono::{Duration, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::flight_recorder::events_agent_activity::agent_activity_event;
use crate::flight_recorder::{FlightRecorder, RecorderError};
use crate::model_runtime::catalog::ModelCatalog;
use crate::model_runtime::cloud::{
    parse_agent_activity_line, AgentActivity, AgentActivityKind, CliKind, CliOutputFormat,
};
use crate::model_runtime::{
    CancellationToken, FinishReason, GenPrompt, GenerateRequest, ModelId, ProviderKind,
    SamplingParams,
};
use crate::process_ledger::RetainedLedgerBatcher;

use crate::terminal::redaction::{PatternRedactor, SecretRedactor};
use futures::StreamExt;

use super::error::SwarmError;
use super::ids::{ByokCloudProvider, ModelInstanceId, SpawnRequest};
use super::model_lane::{
    dexterity_spawn_model_session_id, CloudExportDelegation, DexterityLaunchAdapterKind,
    DexterityLaunchAdapterRequest, DexterityLaunchContract, LaunchAuthority, ModelLaneAuthority,
    ModelLaneCloudConsentReceiptRecord, ModelLaneCloudConsentReceiptStatus,
    ModelLaneCloudConsentScope, ModelLaneCloudExportPosture, ModelLaneCloudProjectionPlanRecord,
    ModelLaneCloudProjectionPlanStatus, ModelLaneCloudRetentionPolicy, ModelLaneDiagnosticTier,
    ModelLaneDiagnosticTierState, ModelLaneError, ModelLaneKind, ModelLaneLocusBinding,
    ModelLaneMessageKind, ModelLaneProviderKind, ModelLaneRecord, ModelLaneRecoveryState,
    ModelLaneRoutingMetadata, ModelLaneRunRecord, ModelLaneStatus, ModelLaneStore, ModelLaneTarget,
    NewModelLane, NewModelLaneCloudConsentReceipt, NewModelLaneCloudProjectionPlan,
    NewModelLaneContextBundleArtifactBinding, NewModelLaneDiagnosticTierStatus,
    NewModelLaneMessage, RuntimeBinding,
};
use super::resource_scope::{AccountBoundAuthority, ActorPrincipalId, ExactResourceScopeAttribution};
use super::routing::ModelLaneRoutingAuthority;
use super::routing_execution::{
    ModelLaneRoutingDispatchBatch, ModelLaneRoutingExecutionContext,
    ModelLaneRoutingExecutionState, ModelLaneRoutingStageLaunch,
};
use super::SwarmCoordinator;

#[cfg(feature = "runtime-full")]
pub use paired_initial::{
    OperatorChatAdmissionSlot, OperatorChatInitialSettlement, OperatorChatInitialState,
    OperatorChatObservationError, OperatorChatRejectedWriter, OperatorChatSubmission,
    OperatorChatWriterResult, OwnedOperatorChatOperation,
    AwaitingParent, OperatorChatSubmissionError, OperatorChatParentObservation,
    OperatorChatSecondOwnerRejected,
};

#[cfg(feature = "runtime-full")]
pub(crate) use paired_initial::{OperatorChatEngineOwnerInstallReason, OperatorChatEngineOwnerInstallRejected, OperatorChatInitialOwnersRef, OperatorChatFirstOwnerRef};

#[cfg(feature = "runtime-full")]
mod paired_initial {
    use std::panic::{catch_unwind, AssertUnwindSafe};
    use std::sync::Arc;

    use futures::FutureExt;
    use tokio::time::Instant;
    use uuid::Uuid;

    use crate::jobs::{JobAdmission, JobAdmissionError};
    use crate::storage::{
        JobCreateCommitOutcome, JobInitialObservation, JobInitialWriterResult, JobRejectedObservation,
        JobSubmissionSettlement,
    };
    use crate::workflows::{OwnedOperatorChatEngineOwner, SessionRegistry, WorkflowReserveError};
    use crate::AppState;

    pub enum OperatorChatAdmissionSlot { First, Second }

    pub enum OperatorChatSubmission<'a> {
        NotSubmitted,
        WriterInstalled,
        ReturnedError(&'a JobAdmissionError),
        PanickedUnknown,
    }

    #[derive(Clone, Copy)]
    pub enum OperatorChatInitialState { NotSubmitted, Pending, Observed }

    pub enum OperatorChatObservationError { PanickedUnknown }

    pub enum OperatorChatWriterResult<'a> {
        WriteReturnedCreated,
        WriteReturnedAlreadyCreated,
        AdmissionError(&'a JobAdmissionError),
        JoinError(&'a tokio::task::JoinError),
    }

    pub enum OperatorChatRejectedWriter<'a> {
        Pending { reason: handshake_job_admission::SubmissionStateError },
        Observed {
            reason: handshake_job_admission::SubmissionStateError,
            outcome: OperatorChatWriterResult<'a>,
        },
    }

    #[derive(Clone, Copy)]
    pub(crate) enum OperatorChatEngineOwnerInstallReason {
        AlreadyInstalled,
        OwnerMismatch,
        PanickedUnknown,
    }

    #[must_use = "retain this incoming owner and the original operation on rejection"]
    pub(crate) struct OperatorChatEngineOwnerInstallRejected {
        _incoming: OwnedOperatorChatEngineOwner,
        reason: OperatorChatEngineOwnerInstallReason,
    }

    impl OperatorChatEngineOwnerInstallRejected {
        pub(crate) fn reason(&self) -> OperatorChatEngineOwnerInstallReason {
            self.reason
        }
    }

    enum Submission {
        NotSubmitted,
        Returned(Result<(), JobAdmissionError>),
        PanickedUnknown { _payload: Box<dyn std::any::Any + Send> },
    }

    impl Submission {
        fn view(&self) -> OperatorChatSubmission<'_> {
            match self {
                Self::NotSubmitted => OperatorChatSubmission::NotSubmitted,
                Self::Returned(Ok(())) => OperatorChatSubmission::WriterInstalled,
                Self::Returned(Err(error)) => OperatorChatSubmission::ReturnedError(error),
                Self::PanickedUnknown { .. } => OperatorChatSubmission::PanickedUnknown,
            }
        }
    }

    struct InitialOwners<S = JobAdmission> {
        first: JobAdmission,
        second: S,
        state: Arc<AppState>,
        _registry: Arc<SessionRegistry>,
        original_deadline: Instant,
    }

    pub(crate) struct OperatorChatInitialOwnersRef<'a> {
        state: &'a Arc<AppState>,
        registry: &'a Arc<SessionRegistry>,
        first: &'a JobAdmission,
        second: &'a JobAdmission,
        original_deadline: Instant,
    }

    impl OperatorChatInitialOwnersRef<'_> {
        pub(crate) fn state(&self) -> &Arc<AppState> { self.state }
        pub(crate) fn registry(&self) -> &Arc<SessionRegistry> { self.registry }
        pub(crate) fn first_admission(&self) -> &JobAdmission { self.first }
        pub(crate) fn second_admission(&self) -> &JobAdmission { self.second }
        pub(crate) fn original_deadline(&self) -> Instant { self.original_deadline }
    }

    #[must_use = "retain the original operation while either initial writer is unsettled"]
    pub struct OwnedOperatorChatOperation<S = JobAdmission> {
        owners: InitialOwners<S>,
        workflow_engine_owner: Option<OwnedOperatorChatEngineOwner>,
        _workflow_engine_owner_install_rejected: Option<OperatorChatEngineOwnerInstallRejected>,
        first_submission: Submission,
        second_submission: Submission,
        first_observation_panicked: bool,
        second_observation_panicked: bool,
        _first_observation_panic: Option<Box<dyn std::any::Any + Send>>,
        _second_observation_panic: Option<Box<dyn std::any::Any + Send>>,
    }

    impl OwnedOperatorChatOperation {
        pub fn prepare(
            first: JobAdmission,
            second: JobAdmission,
            state: Arc<AppState>,
            original_deadline: Instant,
        ) -> Self {
            let registry = state.session_registry.clone();
            let owners = InitialOwners {
                first, second, state, _registry: registry, original_deadline,
            };
            Self {
                owners,
                first_submission: Submission::NotSubmitted,
                second_submission: Submission::NotSubmitted,
                workflow_engine_owner: None,
                _workflow_engine_owner_install_rejected: None,
                first_observation_panicked: false,
                second_observation_panicked: false,
                _first_observation_panic: None,
                _second_observation_panic: None,
            }
        }

        pub fn begin(
            first: JobAdmission,
            second: JobAdmission,
            state: Arc<AppState>,
            original_deadline: Instant,
        ) -> Self {
            let mut operation = Self::prepare(first, second, state, original_deadline);
            Self::submit_initial(
                &mut operation.owners,
                &mut operation.first_submission,
                &mut operation.second_submission,
            );
            operation
        }

        fn submit_initial(
            owners: &mut InitialOwners,
            first_submission: &mut Submission,
            second_submission: &mut Submission,
        ) {
            if matches!(first_submission, Submission::NotSubmitted) {
                *first_submission = match catch_unwind(AssertUnwindSafe(|| {
                    owners.first.submit(owners.state.clone())
                })) {
                    Ok(result) => Submission::Returned(result),
                    Err(payload) => Submission::PanickedUnknown { _payload: payload },
                };
            }
            if matches!(second_submission, Submission::NotSubmitted) {
                *second_submission = match catch_unwind(AssertUnwindSafe(|| {
                    owners.second.submit(owners.state.clone())
                })) {
                    Ok(result) => Submission::Returned(result),
                    Err(payload) => Submission::PanickedUnknown { _payload: payload },
                };
            }
        }

        pub(crate) fn prepare_with_workflow_engine_owner(
            first: JobAdmission,
            second: JobAdmission,
            state: Arc<AppState>,
            original_deadline: Instant,
        ) -> Self {
            let mut operation = Self::prepare(first, second, state, original_deadline);
            let incoming = OwnedOperatorChatEngineOwner::from_initial_owners(operation.initial_owners());
            if let Err(rejected) = operation.install_workflow_engine_owner(incoming) {
                operation._workflow_engine_owner_install_rejected = Some(rejected);
            }
            operation
        }

        pub(crate) async fn submit_prepared(&mut self) -> Result<(), WorkflowReserveError> {
            let Self {
                owners, workflow_engine_owner, first_submission, second_submission, ..
            } = self;
            let owner = workflow_engine_owner.as_mut().ok_or(WorkflowReserveError::OwnerNotInstalled)?;
            if !matches!(first_submission, Submission::NotSubmitted)
                || !matches!(second_submission, Submission::NotSubmitted)
            {
                return Err(WorkflowReserveError::AlreadyConsumed);
            }
            let initial = OperatorChatInitialOwnersRef {
                state: &owners.state,
                registry: &owners._registry,
                first: &owners.first,
                second: &owners.second,
                original_deadline: owners.original_deadline,
            };
            let access = owner.reserve_initial_submission(initial).await?;
            Self::submit_initial(owners, first_submission, second_submission);
            drop(access);
            Ok(())
        }

        pub(crate) fn initial_owners(&self) -> OperatorChatInitialOwnersRef<'_> {
            OperatorChatInitialOwnersRef {
                state: &self.owners.state,
                registry: &self.owners._registry,
                first: &self.owners.first,
                second: &self.owners.second,
                original_deadline: self.owners.original_deadline,
            }
        }

        pub(crate) fn install_workflow_engine_owner(
            &mut self,
            incoming: OwnedOperatorChatEngineOwner,
        ) -> Result<(), OperatorChatEngineOwnerInstallRejected> {
            if self.workflow_engine_owner.is_some() {
                return Err(OperatorChatEngineOwnerInstallRejected {
                    _incoming: incoming,
                    reason: OperatorChatEngineOwnerInstallReason::AlreadyInstalled,
                });
            }
            let verification = catch_unwind(AssertUnwindSafe(|| {
                incoming.matches_initial_owners(
                    &self.owners.state,
                    &self.owners._registry,
                    &self.owners.first,
                    &self.owners.second,
                    self.owners.original_deadline,
                )
            }));
            let reason = match verification {
                Ok(true) => {
                    self.workflow_engine_owner = Some(incoming);
                    return Ok(());
                }
                Ok(false) => OperatorChatEngineOwnerInstallReason::OwnerMismatch,
                Err(_) => OperatorChatEngineOwnerInstallReason::PanickedUnknown,
            };
            Err(OperatorChatEngineOwnerInstallRejected { _incoming: incoming, reason })
        }

        pub(crate) fn workflow_engine_owner_mut(&mut self) -> Option<&mut OwnedOperatorChatEngineOwner> {
            self.workflow_engine_owner.as_mut()
        }

        pub fn submission(&self, slot: OperatorChatAdmissionSlot) -> OperatorChatSubmission<'_> {
            match slot {
                OperatorChatAdmissionSlot::First => self.first_submission.view(),
                OperatorChatAdmissionSlot::Second => self.second_submission.view(),
            }
        }

        pub fn job_id(&self, slot: OperatorChatAdmissionSlot) -> Uuid {
            match slot {
                OperatorChatAdmissionSlot::First => self.owners.first.job_id(),
                OperatorChatAdmissionSlot::Second => self.owners.second.job_id(),
            }
        }

        pub async fn settle_initial<'a>(&'a mut self) -> OperatorChatInitialSettlement<'a> {
            let first_state = match AssertUnwindSafe(self.owners.first.settle(self.owners.original_deadline))
                .catch_unwind().await {
                Ok(JobSubmissionSettlement::NotSubmitted) => OperatorChatInitialState::NotSubmitted,
                Ok(JobSubmissionSettlement::Pending) => OperatorChatInitialState::Pending,
                Ok(JobSubmissionSettlement::Observed(_)) => OperatorChatInitialState::Observed,
                Err(_) => {
                    self.first_observation_panicked = true;
                    OperatorChatInitialState::Pending
                }
            };
            let second_state = match AssertUnwindSafe(self.owners.second.settle(self.owners.original_deadline))
                .catch_unwind().await {
                Ok(JobSubmissionSettlement::NotSubmitted) => OperatorChatInitialState::NotSubmitted,
                Ok(JobSubmissionSettlement::Pending) => OperatorChatInitialState::Pending,
                Ok(JobSubmissionSettlement::Observed(_)) => OperatorChatInitialState::Observed,
                Err(_) => {
                    self.second_observation_panicked = true;
                    OperatorChatInitialState::Pending
                }
            };
            OperatorChatInitialSettlement {
                first_state, second_state,
                first: self.owners.first.initial_result(),
                second: self.owners.second.initial_result(),
                first_submission: &self.first_submission,
                second_submission: &self.second_submission,
                first_observation_panicked: self.first_observation_panicked,
                second_observation_panicked: self.second_observation_panicked,
            }
        }
    }

    pub struct OperatorChatInitialSettlement<'a> {
        first_state: OperatorChatInitialState,
        second_state: OperatorChatInitialState,
        first: JobInitialObservation<'a>,
        second: JobInitialObservation<'a>,
        first_submission: &'a Submission,
        second_submission: &'a Submission,
        first_observation_panicked: bool,
        second_observation_panicked: bool,
    }

    impl OperatorChatInitialSettlement<'_> {
        fn observation(&self, slot: OperatorChatAdmissionSlot) -> &JobInitialObservation<'_> {
            match slot {
                OperatorChatAdmissionSlot::First => &self.first,
                OperatorChatAdmissionSlot::Second => &self.second,
            }
        }

        pub fn state(&self, slot: OperatorChatAdmissionSlot) -> OperatorChatInitialState {
            match slot {
                OperatorChatAdmissionSlot::First => self.first_state,
                OperatorChatAdmissionSlot::Second => self.second_state,
            }
        }

        pub fn submission(&self, slot: OperatorChatAdmissionSlot) -> OperatorChatSubmission<'_> {
            match slot {
                OperatorChatAdmissionSlot::First => self.first_submission.view(),
                OperatorChatAdmissionSlot::Second => self.second_submission.view(),
            }
        }

        pub fn observation_error(&self, slot: OperatorChatAdmissionSlot) -> Option<OperatorChatObservationError> {
            let panicked = match slot {
                OperatorChatAdmissionSlot::First => self.first_observation_panicked,
                OperatorChatAdmissionSlot::Second => self.second_observation_panicked,
            };
            panicked.then_some(OperatorChatObservationError::PanickedUnknown)
        }

        pub fn primary(&self, slot: OperatorChatAdmissionSlot) -> Option<OperatorChatWriterResult<'_>> {
            self.observation(slot).primary.map(writer_result)
        }

        pub fn rejected(&self, slot: OperatorChatAdmissionSlot) -> Option<OperatorChatRejectedWriter<'_>> {
            let observation = self.observation(slot);
            observation.rejected.as_ref().map(|rejected| match rejected {
                JobRejectedObservation::Pending { reason } => OperatorChatRejectedWriter::Pending { reason: *reason },
                JobRejectedObservation::Observed { reason, outcome } => OperatorChatRejectedWriter::Observed {
                    reason: *reason, outcome: writer_result(outcome),
                },
            })
        }
    }


    pub struct AwaitingParent { _private: () }

    #[derive(Clone, Copy, Debug)]
    pub enum OperatorChatSubmissionError {
        OwnerNotInstalled, OwnerMismatch, Deadline, AlreadyConsumed, RegistryUnknown, PanickedUnknown,
    }

    impl From<WorkflowReserveError> for OperatorChatSubmissionError {
        fn from(reason: WorkflowReserveError) -> Self {
            match reason {
                WorkflowReserveError::OwnerNotInstalled => Self::OwnerNotInstalled,
                WorkflowReserveError::OwnerMismatch => Self::OwnerMismatch,
                WorkflowReserveError::Deadline => Self::Deadline,
                WorkflowReserveError::AlreadyConsumed => Self::AlreadyConsumed,
                WorkflowReserveError::RegistryUnknown => Self::RegistryUnknown,
                WorkflowReserveError::PanickedUnknown => Self::PanickedUnknown,
            }
        }
    }

    pub(crate) struct OperatorChatFirstOwnerRef<'a> {
        state: &'a Arc<AppState>,
        registry: &'a Arc<SessionRegistry>,
        first: &'a JobAdmission,
        original_deadline: Instant,
    }

    impl OperatorChatFirstOwnerRef<'_> {
        pub(crate) fn state(&self) -> &Arc<AppState> { self.state }
        pub(crate) fn registry(&self) -> &Arc<SessionRegistry> { self.registry }
        pub(crate) fn first_admission(&self) -> &JobAdmission { self.first }
        pub(crate) fn original_deadline(&self) -> Instant { self.original_deadline }
    }

    impl<S> OwnedOperatorChatOperation<S> {
        pub(crate) fn first_owner(&self) -> OperatorChatFirstOwnerRef<'_> {
            OperatorChatFirstOwnerRef {
                state: &self.owners.state,
                registry: &self.owners._registry,
                first: &self.owners.first,
                original_deadline: self.owners.original_deadline,
            }
        }

        pub fn first_job_id(&self) -> Uuid { self.owners.first.job_id() }

        pub fn first_submission(&self) -> OperatorChatSubmission<'_> {
            self.first_submission.view()
        }
    }

    impl OwnedOperatorChatOperation<AwaitingParent> {
        pub async fn submit_first_prepared(&mut self) -> Result<(), OperatorChatSubmissionError> {
            let Self { owners, workflow_engine_owner, first_submission, .. } = self;
            let owner = workflow_engine_owner.as_mut()
                .ok_or(OperatorChatSubmissionError::OwnerNotInstalled)?;
            if !matches!(first_submission, Submission::NotSubmitted) {
                return Err(OperatorChatSubmissionError::AlreadyConsumed);
            }
            let initial = OperatorChatFirstOwnerRef {
                state: &owners.state,
                registry: &owners._registry,
                first: &owners.first,
                original_deadline: owners.original_deadline,
            };
            let access = owner.reserve_first_submission(initial).await?;
            *first_submission = match catch_unwind(AssertUnwindSafe(|| {
                owners.first.submit(owners.state.clone())
            })) {
                Ok(result) => Submission::Returned(result),
                Err(payload) => Submission::PanickedUnknown { _payload: payload },
            };
            drop(access);
            Ok(())
        }

        pub(crate) async fn bind_second(
            &mut self,
            second: &JobAdmission,
        ) -> Result<(), OperatorChatSubmissionError> {
            let Self { owners, workflow_engine_owner, .. } = self;
            let owner = workflow_engine_owner.as_mut()
                .ok_or(OperatorChatSubmissionError::OwnerNotInstalled)?;
            let first = OperatorChatFirstOwnerRef {
                state: &owners.state,
                registry: &owners._registry,
                first: &owners.first,
                original_deadline: owners.original_deadline,
            };
            owner.extend_second_owner(first, second).await.map_err(Into::into)
        }
    }

    impl OwnedOperatorChatOperation {
        pub async fn submit_second_prepared(&mut self) -> Result<(), OperatorChatSubmissionError> {
            let Self { owners, workflow_engine_owner, second_submission, .. } = self;
            let owner = workflow_engine_owner.as_mut()
                .ok_or(OperatorChatSubmissionError::OwnerNotInstalled)?;
            if !matches!(second_submission, Submission::NotSubmitted) {
                return Err(OperatorChatSubmissionError::AlreadyConsumed);
            }
            let initial = OperatorChatInitialOwnersRef {
                state: &owners.state,
                registry: &owners._registry,
                first: &owners.first,
                second: &owners.second,
                original_deadline: owners.original_deadline,
            };
            let access = owner.reserve_second_submission(initial).await?;
            *second_submission = match catch_unwind(AssertUnwindSafe(|| {
                owners.second.submit(owners.state.clone())
            })) {
                Ok(result) => Submission::Returned(result),
                Err(payload) => Submission::PanickedUnknown { _payload: payload },
            };
            drop(access);
            Ok(())
        }
    }

    pub struct OperatorChatParentObservation {
        inner: crate::workflows::OperatorChatParentObservation,
    }

    impl OperatorChatParentObservation {
        pub fn session_id(&self) -> &str { self.inner.session_id() }
        pub fn spawn_depth(&self) -> i32 { self.inner.spawn_depth() }
        pub fn is_active(&self) -> bool { self.inner.is_active() }
    }

    impl OwnedOperatorChatOperation<AwaitingParent> {
        pub(crate) fn prepare_first_with_workflow_engine_owner(
            first: JobAdmission,
            state: Arc<AppState>,
            original_deadline: Instant,
        ) -> Self {
            let registry = state.session_registry.clone();
            let mut operation = Self {
                owners: InitialOwners {
                    first, second: AwaitingParent { _private: () },
                    state, _registry: registry, original_deadline,
                },
                workflow_engine_owner: None,
                _workflow_engine_owner_install_rejected: None,
                first_submission: Submission::NotSubmitted,
                second_submission: Submission::NotSubmitted,
                first_observation_panicked: false,
                second_observation_panicked: false,
                _first_observation_panic: None,
                _second_observation_panic: None,
            };
            let incoming = OwnedOperatorChatEngineOwner::from_first_owner(operation.first_owner());
            let verification = catch_unwind(AssertUnwindSafe(|| {
                incoming.matches_first_owner(operation.first_owner())
            }));
            match verification {
                Ok(true) => operation.workflow_engine_owner = Some(incoming),
                other => {
                    let reason = match other {
                        Ok(false) => OperatorChatEngineOwnerInstallReason::OwnerMismatch,
                        Err(_) => OperatorChatEngineOwnerInstallReason::PanickedUnknown,
                        Ok(true) => unreachable!("successful verification handled above"),
                    };
                    operation._workflow_engine_owner_install_rejected = Some(
                        OperatorChatEngineOwnerInstallRejected { _incoming: incoming, reason },
                    );
                }
            }
            operation
        }

        pub async fn first_parent_observation(
            &mut self,
        ) -> Result<OperatorChatParentObservation, OperatorChatSubmissionError> {
            if self.first_observation_panicked {
                return Err(OperatorChatSubmissionError::PanickedUnknown);
            }
            match AssertUnwindSafe(self.owners.first.settle(self.owners.original_deadline))
                .catch_unwind().await
            {
                Ok(JobSubmissionSettlement::Observed(observation))
                    if observation.rejected.is_none()
                        && matches!(observation.primary, Some(Ok(Ok(
                            JobCreateCommitOutcome::Created(_) | JobCreateCommitOutcome::AlreadyCreated(_)
                        )))) => {}
                Ok(_) => return Err(OperatorChatSubmissionError::RegistryUnknown),
                Err(payload) => {
                    self.first_observation_panicked = true;
                    self._first_observation_panic = Some(payload);
                    return Err(OperatorChatSubmissionError::PanickedUnknown);
                }
            }
            let Self { owners, workflow_engine_owner, .. } = self;
            let owner = workflow_engine_owner.as_mut()
                .ok_or(OperatorChatSubmissionError::OwnerNotInstalled)?;
            let first = OperatorChatFirstOwnerRef {
                state: &owners.state,
                registry: &owners._registry,
                first: &owners.first,
                original_deadline: owners.original_deadline,
            };
            owner.first_parent_observation(first).await
                .map(|inner| OperatorChatParentObservation { inner }).map_err(Into::into)
        }
    }

    impl<S> OwnedOperatorChatOperation<S> {

        pub fn request_cancel(&mut self) -> Result<(), OperatorChatSubmissionError> {
            self.workflow_engine_owner.as_mut()
                .ok_or(OperatorChatSubmissionError::OwnerNotInstalled)?
                .request_cancel().map_err(Into::into)
        }

    }

    #[must_use = "retain both the original parent operation and actual incoming admission on rejection"]
    pub struct OperatorChatSecondOwnerRejected {
        operation: OwnedOperatorChatOperation<AwaitingParent>,
        second: JobAdmission,
        reason: OperatorChatSubmissionError,
        _verification_panic: Option<Box<dyn std::any::Any + Send>>,
    }

    impl OperatorChatSecondOwnerRejected {
        pub fn reason(&self) -> OperatorChatSubmissionError { self.reason }

        pub fn operation_mut(&mut self) -> &mut OwnedOperatorChatOperation<AwaitingParent> {
            &mut self.operation
        }

        pub(crate) fn into_parts(self) -> (
            OwnedOperatorChatOperation<AwaitingParent>, JobAdmission,
            OperatorChatSubmissionError, Option<Box<dyn std::any::Any + Send>>,
        ) {
            (self.operation, self.second, self.reason, self._verification_panic)
        }
    }

    impl OwnedOperatorChatOperation<AwaitingParent> {
        pub fn extend_second(
            self,
            second: JobAdmission,
        ) -> Result<OwnedOperatorChatOperation, OperatorChatSecondOwnerRejected> {
            let verification = catch_unwind(AssertUnwindSafe(|| {
                self.workflow_engine_owner.as_ref().map(|owner| owner.matches_initial_owners(
                    &self.owners.state, &self.owners._registry,
                    &self.owners.first, &second, self.owners.original_deadline,
                ))
            }));
            let (reason, payload) = match verification {
                Ok(Some(true)) => (None, None),
                Ok(Some(false)) => (Some(OperatorChatSubmissionError::OwnerMismatch), None),
                Ok(None) => (Some(OperatorChatSubmissionError::OwnerNotInstalled), None),
                Err(payload) => (Some(OperatorChatSubmissionError::PanickedUnknown), Some(payload)),
            };
            if let Some(reason) = reason {
                return Err(OperatorChatSecondOwnerRejected {
                    operation: self, second, reason, _verification_panic: payload,
                });
            }
            let Self {
                owners, workflow_engine_owner, _workflow_engine_owner_install_rejected,
                first_submission, second_submission,
                first_observation_panicked, second_observation_panicked,
                _first_observation_panic, _second_observation_panic,
            } = self;
            let InitialOwners { first, state, _registry, original_deadline, .. } = owners;
            Ok(OwnedOperatorChatOperation {
                owners: InitialOwners { first, second, state, _registry, original_deadline },
                workflow_engine_owner, _workflow_engine_owner_install_rejected,
                first_submission, second_submission,
                first_observation_panicked, second_observation_panicked,
                _first_observation_panic, _second_observation_panic,
            })
        }
    }


    async fn settle_owned_admission(
        admission: &mut JobAdmission,
        submission: &Submission,
        original_deadline: Instant,
        panicked: &mut bool,
        payload: &mut Option<Box<dyn std::any::Any + Send>>,
    ) -> bool {
        if *panicked { return false; }
        match AssertUnwindSafe(admission.settle(original_deadline)).catch_unwind().await {
            Ok(JobSubmissionSettlement::NotSubmitted) => !matches!(submission, Submission::PanickedUnknown { .. }),
            Ok(JobSubmissionSettlement::Observed(_)) => true,
            Ok(JobSubmissionSettlement::Pending) => false,
            Err(cause) => {
                *panicked = true;
                *payload = Some(cause);
                false
            }
        }
    }

    impl OwnedOperatorChatOperation<AwaitingParent> {
        pub async fn drain(&mut self) -> Result<(), OperatorChatSubmissionError> {
            let settled = settle_owned_admission(
                &mut self.owners.first, &self.first_submission, self.owners.original_deadline,
                &mut self.first_observation_panicked, &mut self._first_observation_panic,
            ).await;
            self.workflow_engine_owner.as_mut()
                .ok_or(OperatorChatSubmissionError::OwnerNotInstalled)?
                .drain().await?;
            if settled { Ok(()) } else { Err(OperatorChatSubmissionError::RegistryUnknown) }
        }
    }

    impl OwnedOperatorChatOperation {
        pub async fn drain(&mut self) -> Result<(), OperatorChatSubmissionError> {
            let first_settled = settle_owned_admission(
                &mut self.owners.first, &self.first_submission, self.owners.original_deadline,
                &mut self.first_observation_panicked, &mut self._first_observation_panic,
            ).await;
            let second_settled = settle_owned_admission(
                &mut self.owners.second, &self.second_submission, self.owners.original_deadline,
                &mut self.second_observation_panicked, &mut self._second_observation_panic,
            ).await;
            self.workflow_engine_owner.as_mut()
                .ok_or(OperatorChatSubmissionError::OwnerNotInstalled)?
                .drain().await?;
            if first_settled && second_settled { Ok(()) }
            else { Err(OperatorChatSubmissionError::RegistryUnknown) }
        }
    }

    fn writer_result(result: &JobInitialWriterResult) -> OperatorChatWriterResult<'_> {
        match result {
            Ok(Ok(JobCreateCommitOutcome::Created(_))) => OperatorChatWriterResult::WriteReturnedCreated,
            Ok(Ok(JobCreateCommitOutcome::AlreadyCreated(_))) => OperatorChatWriterResult::WriteReturnedAlreadyCreated,
            Ok(Err(error)) => OperatorChatWriterResult::AdmissionError(error),
            Err(error) => OperatorChatWriterResult::JoinError(error),
        }
    }
}

pub(crate) use super::coordinator::ValidatorProducerKey;

/// Stable surface id carried on every operator-chat capture message so
/// diagnostic/projection tooling can filter this work-surface without prose.
pub const OPERATOR_CHAT_SURFACE_ID: &str = "wp1_operator_chat_launch";

/// Adapter label stamped on the `FR-EVT-AGENT-*` events this surface emits.
pub const OPERATOR_CHAT_CLI_ADAPTER: &str = "operator_chat_cli_bridge";

/// Prefix that [`parse_agent_activity_line`] uses when it renders a CLI
/// `tool_result` content block as a [`AgentActivity::Text`]. Detecting it lets us
/// map a rendered tool_result to [`ModelLaneMessageKind::ToolResult`] per F1.
pub const RENDERED_TOOL_RESULT_PREFIX: &str = "[tool_result]";

/// How long an auto-issued operator-chat cloud ConsentReceipt stays valid.
///
/// # Why this is 12 hours and not 365 days
///
/// This receipt is minted by the launch path itself, for exactly ONE lane in ONE
/// run, and it authorizes sending operator context to a third-party provider.
/// The previous value was `now + 365 days`, which is longer than the resource it
/// authorizes can possibly live: a lane cannot outlive its run, a run cannot
/// outlive the coordinator process, and neither survives a machine restart. A
/// capability that outlives its subject by three orders of magnitude is a
/// dangling capability — after the lane is gone the receipt is still an
/// approved, unrevoked, replayable authorization for that provider/scope pair,
/// and every idempotent re-launch on the same run/lane ids re-uses it instead of
/// asking again.
///
/// The genuinely correct binding is lifetime-scoped consent, and that primitive
/// exists (`ModelLaneCloudConsentScope::SingleRun`, migration
/// `0353_model_lane_run_scoped_consent`), but it is a *scope* mechanism, not an
/// *expiry* mechanism: it says which lanes one grant covers, not when the grant
/// dies. There is no run-end timestamp available at mint time to bind
/// `valid_until_utc` to, so the honest construction is a bounded window that is
/// long enough to cover a real working session (a cloud lane can legitimately
/// run for hours) and short enough that an abandoned receipt expires on its own
/// rather than waiting for someone to notice it. 12 hours is that window: longer
/// than any single operator-chat turn or session, shorter than a day, and it
/// cannot silently survive to the next working day.
///
/// Revocation stays the immediate kill switch; expiry is the backstop for the
/// case where nobody revokes because nobody remembers the receipt exists.
pub const OPERATOR_CHAT_CLOUD_CONSENT_VALIDITY_HOURS: i64 = 12;

/// Clock-skew grace applied to `valid_from_utc` so a receipt minted on one clock
/// is not rejected as "not yet valid" by a check on a slightly earlier clock.
pub const OPERATOR_CHAT_CLOUD_CONSENT_BACKDATE_MINUTES: i64 = 5;

/// Recorded as `approved_by_ref` when the operator-chat launch path has no
/// authenticated account to bind the approval to.
///
/// It replaces `format!("operator://{}/cloud-selection", owner_session)`, which
/// claimed an operator approved the export when all it actually recorded was the
/// governance role label of the thing requesting the export. This value claims
/// nothing: it states that no authenticated account existed, and the typed
/// `approver` alongside it is [`AccountBoundAuthority::Unattributed`], which no
/// account-scoped gate will accept.
pub const OPERATOR_CHAT_UNATTRIBUTED_APPROVAL_REF: &str =
    "unattributed://operator-chat/no-authenticated-account";

/// Stable reason stamped on an unattributed operator-chat cloud approval so
/// every such row is enumerable by an auditor.
pub const OPERATOR_CHAT_UNATTRIBUTED_APPROVAL_REASON: &str =
    "OPERATOR_CHAT_CLOUD_LAUNCH_WITHOUT_AUTHENTICATED_ACCOUNT";

/// Operator Chat request for one immutable run-scoped cloud-consent grant.
/// The receipt is derived from the stored plan so its hash, policies, and
/// enumerated targets cannot diverge at this product boundary.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperatorChatSingleRunCloudConsentGrant {
    pub projection_plan: NewModelLaneCloudProjectionPlan,
    pub consent_receipt_id: String,
    pub approved_by_ref: String,
    pub approved_at_utc: String,
    pub valid_from_utc: String,
    pub valid_until_utc: String,
    pub consent_idempotency_key: String,
    pub diagnostic_payload: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperatorChatSingleRunCloudLaunchRequest {
    pub grant: OperatorChatSingleRunCloudConsentGrant,
    pub selections: Vec<OperatorChatSelection>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperatorChatSingleRunCloudLaunched {
    pub projection_plan_id: String,
    pub consent_receipt_id: String,
    pub instance_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperatorChatSingleRunCloudRevokeRequest {
    pub consent_receipt_id: String,
    pub revoked_by_ref: String,
    pub reason: String,
}

/// Errors surfaced by the operator chat/launch engine.
#[derive(Debug, thiserror::Error)]
pub enum OperatorChatError {
    #[error("model lane store error: {0}")]
    ModelLane(#[from] ModelLaneError),
    #[error("flight recorder error: {0}")]
    Recorder(#[from] RecorderError),
    #[error("swarm error: {0}")]
    Swarm(#[from] SwarmError),
    #[error("operator chat selection invalid: {0}")]
    Invalid(String),
    #[error("validator association readback failed: {0}")]
    ValidatorAssociationReadback(crate::storage::surreal::resource_authority::ResourceAuthorityError),
    #[error("validator association reservation rejected")]
    ValidatorAssociationReservation(Box<crate::storage::surreal::resource_authority::ValidatorAssociationReserveRejection>),
    #[error("validator association retained failure")]
    ValidatorAssociationRetained(Box<crate::storage::surreal::resource_authority::ValidatorAssociationRetainedFailure>),
    #[error("validator association routing attempt lacks its actual canonical evidence")]
    MissingCanonicalLaunchPlan(Box<MissingCanonicalLaunchPlan>),
}

/// Which lane kind the operator picked in the chat/launch pane.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OperatorChatLaneKind {
    /// Embedded local ModelRuntime lane (enumerated via [`ModelCatalog`]).
    Local,
    /// BYOK cloud lane (enumerated via the MT-015 cloud enumeration API).
    Cloud,
    /// Official CLI bridge lane (claude/codex) — the CLI-wrapper capture path.
    Cli,
    /// Runtime-owned subagent participant lane with no OS process spawned.
    Subagent,
}

/// One operator launch request assembled from the pane selection.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperatorChatSelection {
    #[serde(default, skip_deserializing, skip_serializing_if = "Option::is_none")]
    pub engine_profile_session: Option<crate::storage::EngineProfileSessionDeclarationV1>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub engine_profile: Option<crate::model_runtime::EngineProfileSnapshotV1>,
    pub lane_kind: OperatorChatLaneKind,
    /// Concrete model id the operator selected (local model id or cloud model name).
    pub model_id: String,
    /// For `Cloud`: the BYOK provider flavor (`anthropic` | `openai`).
    #[serde(default)]
    pub cloud_provider: Option<String>,
    /// For `Cli`: the exact official CLI provider row (`claude_code` | `codex`).
    #[serde(default)]
    pub cli_provider: Option<String>,
    /// Operator-selected folder/worktree path. Becomes the REAL CLI subprocess
    /// cwd (plumbed `SpawnRequest.working_dir -> CliBridgeConfig.working_dir`).
    pub working_dir: String,
    /// Optional worktree id attribution.
    #[serde(default)]
    pub worktree_id: Option<String>,
    /// The operator's prompt / message for this turn.
    pub prompt: String,
    /// Owner session identity (governs the process ledger + operator-lane authority).
    pub owner_session: String,
    /// Parent session that requested the launch.
    pub parent_session_id: String,
    /// Locus work-packet attribution (defaults to the operator-chat workspace).
    #[serde(default)]
    pub work_packet_id: Option<String>,
    /// Locus micro-task attribution (defaults to the operator turn).
    #[serde(default)]
    pub micro_task_id: Option<String>,
}

/// External operator-chat launch contract. The client supplies only the governed
/// owner id; trusted owner/parent lineage is resolved by the API from SessionRegistry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperatorChatLaunchRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub engine_profile: Option<crate::model_runtime::EngineProfileSnapshotV1>,
    pub lane_kind: OperatorChatLaneKind,
    pub model_id: String,
    #[serde(default)]
    pub cloud_provider: Option<String>,
    #[serde(default)]
    pub cli_provider: Option<String>,
    pub working_dir: String,
    #[serde(default)]
    pub worktree_id: Option<String>,
    pub prompt: String,
    pub owner_session_id: String,
    #[serde(default)]
    pub work_packet_id: Option<String>,
    #[serde(default)]
    pub micro_task_id: Option<String>,
}

impl OperatorChatLaunchRequest {
    pub fn into_governed_selection(
        self,
        owner_session: String,
        parent_session_id: String,
    ) -> OperatorChatSelection {
        OperatorChatSelection {
            engine_profile_session: None,
            engine_profile: self.engine_profile,
            lane_kind: self.lane_kind,
            model_id: self.model_id,
            cloud_provider: self.cloud_provider,
            cli_provider: self.cli_provider,
            working_dir: self.working_dir,
            worktree_id: self.worktree_id,
            prompt: self.prompt,
            owner_session,
            parent_session_id,
            work_packet_id: self.work_packet_id,
            micro_task_id: self.micro_task_id,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperatorChatRoutingStageRequest {
    pub stage_id: String,
    #[serde(default)]
    pub lane_id: Option<String>,
    #[serde(default)]
    pub selection: Option<OperatorChatSelection>,
    #[serde(default)]
    pub authority_lane_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperatorChatRoutingLifecycleRequest {
    pub execution_id: String,
    pub selecting_decision_id: String,
    pub authority: ModelLaneRoutingAuthority,
    pub context: ModelLaneRoutingExecutionContext,
    pub stages: Vec<OperatorChatRoutingStageRequest>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperatorChatRoutingCancelRequest {
    pub execution_id: String,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperatorChatRoutingAuthorityRequest {
    pub execution_id: String,
    pub stage_id: String,
    pub message_id: String,
    pub routing_request: OperatorChatRoutingLifecycleRequest,
}

impl OperatorChatSelection {
    fn work_packet(&self) -> &str {
        self.work_packet_id
            .as_deref()
            .filter(|s| !s.trim().is_empty())
            .unwrap_or("operator-chat-workspace")
    }

    fn micro_task(&self) -> &str {
        self.micro_task_id
            .as_deref()
            .filter(|s| !s.trim().is_empty())
            .unwrap_or("operator-chat-turn")
    }
}

/// Result of a successful operator launch: the persisted CLI ModelLane ids so the
/// pane can bind its transcript to the run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperatorChatLaunched {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub engine_profile: Option<crate::model_runtime::EngineProfileSnapshotV1>,
    pub instance_id: String,
    pub run_id: String,
    pub lane_id: String,
    pub trace_id: String,
    pub lane_kind: OperatorChatLaneKind,
    /// Number of typed [`NewModelLaneMessage`] rows captured from the launched
    /// runtime's REAL stdout during this turn (F1/F2 live loop). `0` for a lane
    /// whose runtime produced no structured activity or when the coordinator has
    /// no `ModelLaneStore` to persist into.
    #[serde(default)]
    pub captured_message_count: usize,
}

/// Max tokens the operator-chat turn requests from the launched runtime. The CLI
/// bridge lane ignores this (the subprocess runs to completion and the whole
/// stdout is drained), but the local/candle lanes honor it; a generous cap keeps
/// a real turn from being truncated mid-answer.
const OPERATOR_CHAT_MAX_TOKENS: u32 = 4096;

/// Map the operator-selected model id to the CLI dialect its stdout speaks, so
/// [`ModelLaneCaptureRecorder::capture_cli_stream`] parses the launched runtime's
/// real output with the correct parser (claude vs codex vs gemini vs generic).
pub fn cli_kind_for_model(model_id: &str) -> CliKind {
    let m = model_id.to_ascii_lowercase();
    if m.contains("claude") {
        CliKind::ClaudeCode
    } else if m.contains("codex") || m.contains("gpt") {
        CliKind::CodexCli
    } else if m.contains("gemini") {
        CliKind::GeminiCli
    } else {
        CliKind::Other
    }
}

/// Prefer the explicit CLI provider id the picker sent; fall back to the model
/// name only for legacy callers.
pub fn cli_kind_for_selection(selection: &OperatorChatSelection) -> CliKind {
    match selection.cli_provider.as_deref() {
        Some("claude_code" | "claude-code" | "claude") => CliKind::ClaudeCode,
        Some("codex" | "codex_cli" | "codex-cli") => CliKind::CodexCli,
        Some("gemini" | "gemini_cli" | "gemini-cli") => CliKind::GeminiCli,
        Some(_) | None => cli_kind_for_model(&selection.model_id),
    }
}

/// One pane-friendly transcript row projected from a captured
/// [`ModelLaneMessageRecord`] (F8 transcript render).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperatorChatTranscriptRow {
    /// `operator | thinking | tool_call | tool_result | text | other`.
    pub role: String,
    /// The captured turn text (thought / answer / tool summary).
    pub text: String,
    /// The typed lane message kind (`status | tool_request | tool_result | ...`).
    pub kind: String,
    pub message_id: String,
    pub ordered_index: u64,
}

/// Non-secret model inventory for the picker: local models (MT-014) + cloud
/// access rows (MT-015). Serializable so the enumeration route returns it verbatim.
#[derive(Debug, Clone, Serialize)]
pub struct OperatorChatModelInventory {
    pub inventory_source: &'static str,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub active_engine_profile: Option<crate::model_runtime::EngineProfileIdentityV1>,
    pub sessions: Vec<OperatorChatSessionRow>,
    pub local: Vec<OperatorChatModelRow>,
    pub cloud_byok: Vec<OperatorChatCloudRow>,
    pub cloud_cli_bridge: Vec<OperatorChatCloudRow>,
    pub subagents: Vec<OperatorChatSubagentRow>,
    pub excluded: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct OperatorChatSessionRow {
    #[serde(default)]
    pub job_id: Option<Uuid>,
    pub session_id: String,
    pub parent_session_id: Option<String>,
    pub label: String,
    /// `available` only when owner and registered parent are Active and governed.
    pub status: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct OperatorChatModelRow {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub engine_profile: Option<crate::model_runtime::EngineProfileSnapshotV1>,
    pub model_id: String,
    pub display_name: String,
    pub runtime_binding: String,
    pub ready: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct OperatorChatCloudRow {
    pub provider: String,
    pub model_id: String,
    pub label: String,
    /// `"configured"` | `"unavailable"` (fail-closed when a provider is absent).
    pub status: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct OperatorChatSubagentRow {
    pub role: String,
    pub model_id: String,
    pub label: String,
    /// `"available"` | `"unavailable"` (fail-closed when the manager is absent).
    pub status: String,
}

/// The plan for turning ONE captured activity into a typed lane message.
#[derive(Debug, Clone)]
pub struct CapturedMessagePlan {
    pub kind: ModelLaneMessageKind,
    /// `tool_call | thinking | text | other`, from [`AgentActivityKind::label`].
    pub activity_kind: &'static str,
    pub summary: String,
    pub payload: Value,
    /// Tool messages require at least one `tool_gate_decision_ref` (validator law).
    pub is_tool: bool,
}

fn is_rendered_tool_result(text: &str) -> bool {
    text.trim_start().starts_with(RENDERED_TOOL_RESULT_PREFIX)
}

fn short_summary(text: &str, fallback: &str) -> String {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return fallback.to_string();
    }
    let mut out: String = trimmed.chars().take(120).collect();
    if trimmed.chars().count() > 120 {
        out.push('…');
    }
    out
}

/// Map one captured [`AgentActivity`] to its lane-message plan (contract F1).
///
/// Pure + deterministic so the mapping is unit-testable without a database.
pub fn plan_from_activity(activity: &AgentActivity) -> CapturedMessagePlan {
    let activity_kind = activity.kind().label();
    match activity {
        AgentActivity::ToolCall {
            name,
            input,
            call_id,
        } => CapturedMessagePlan {
            kind: ModelLaneMessageKind::ToolRequest,
            activity_kind,
            summary: format!("tool_call:{name}"),
            payload: json!({
                "activity_kind": activity_kind,
                "name": name,
                "input": input,
                "call_id": call_id,
            }),
            is_tool: true,
        },
        AgentActivity::Text { text } if is_rendered_tool_result(text) => CapturedMessagePlan {
            kind: ModelLaneMessageKind::ToolResult,
            activity_kind,
            summary: short_summary(text, "tool_result"),
            payload: json!({
                "activity_kind": activity_kind,
                "rendered_tool_result": true,
                "text": text,
            }),
            is_tool: true,
        },
        AgentActivity::Text { text } => CapturedMessagePlan {
            kind: ModelLaneMessageKind::Status,
            activity_kind,
            summary: short_summary(text, "model_text"),
            payload: json!({ "activity_kind": activity_kind, "text": text }),
            is_tool: false,
        },
        AgentActivity::Thinking { text } => CapturedMessagePlan {
            kind: ModelLaneMessageKind::Status,
            activity_kind,
            summary: short_summary(text, "model_thought"),
            payload: json!({ "activity_kind": activity_kind, "text": text }),
            is_tool: false,
        },
        AgentActivity::Other { raw } => CapturedMessagePlan {
            kind: ModelLaneMessageKind::Status,
            activity_kind,
            summary: short_summary(raw, "cli_other"),
            payload: json!({ "activity_kind": activity_kind, "raw": raw }),
            is_tool: false,
        },
    }
}

fn message_kind_label(kind: &ModelLaneMessageKind) -> &'static str {
    match kind {
        ModelLaneMessageKind::Proposal => "proposal",
        ModelLaneMessageKind::Critique => "critique",
        ModelLaneMessageKind::ToolRequest => "tool_request",
        ModelLaneMessageKind::ToolResult => "tool_result",
        ModelLaneMessageKind::Status => "status",
        ModelLaneMessageKind::PromotionRequest => "promotion_request",
        ModelLaneMessageKind::Recovery => "recovery",
    }
}

fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

fn canonical_json_bytes(value: &Value) -> Vec<u8> {
    let mut output = String::new();
    write_canonical_json(&mut output, value);
    output.into_bytes()
}

fn write_canonical_json(output: &mut String, value: &Value) {
    match value {
        Value::Null => output.push_str("null"),
        Value::Bool(value) => output.push_str(if *value { "true" } else { "false" }),
        Value::Number(value) => output.push_str(&value.to_string()),
        Value::String(value) => {
            output.push('"');
            for ch in value.chars() {
                match ch {
                    '"' => output.push_str("\\\""),
                    '\\' => output.push_str("\\\\"),
                    '\n' => output.push_str("\\n"),
                    '\r' => output.push_str("\\r"),
                    '\t' => output.push_str("\\t"),
                    other => output.push(other),
                }
            }
            output.push('"');
        }
        Value::Array(values) => {
            output.push('[');
            for (idx, item) in values.iter().enumerate() {
                if idx > 0 {
                    output.push(',');
                }
                write_canonical_json(output, item);
            }
            output.push(']');
        }
        Value::Object(map) => {
            output.push('{');
            let mut keys: Vec<&String> = map.keys().collect();
            keys.sort();
            for (idx, key) in keys.into_iter().enumerate() {
                if idx > 0 {
                    output.push(',');
                }
                write_canonical_json(output, &Value::String(key.clone()));
                output.push(':');
                write_canonical_json(output, &map[key]);
            }
            output.push('}');
        }
    }
}

fn build_message_payload_binding(
    message: &NewModelLaneMessage,
    payload_json: Value,
) -> Result<NewModelLaneContextBundleArtifactBinding, OperatorChatError> {
    let work_packet_id = message.work_packet_id.clone().ok_or_else(|| {
        OperatorChatError::Invalid(format!(
            "message {} is missing work_packet_id required for payload artifact binding",
            message.message_id
        ))
    })?;
    let micro_task_id = message.micro_task_id.clone().ok_or_else(|| {
        OperatorChatError::Invalid(format!(
            "message {} is missing micro_task_id required for payload artifact binding",
            message.message_id
        ))
    })?;
    let task_board_id = message.task_board_id.clone().ok_or_else(|| {
        OperatorChatError::Invalid(format!(
            "message {} is missing task_board_id required for payload artifact binding",
            message.message_id
        ))
    })?;
    Ok(NewModelLaneContextBundleArtifactBinding {
        artifact_binding_id: format!("artifact-binding-{}", message.message_id),
        run_id: message.run_id.clone(),
        trace_id: message.trace_id.clone(),
        artifact_ref: message.payload_ref.clone(),
        artifact_sha256: message.payload_sha256.clone(),
        content_hash: message.payload_sha256.clone(),
        artifact_kind: "model_lane_message_payload".to_string(),
        artifact_manifest_ref: format!(
            "artifact-store://operator-chat/{}/{}.json",
            message.run_id, message.message_id
        ),
        artifact_payload_ref: message.payload_ref.clone(),
        payload_json,
        event_ledger_stream_id: message.event_ledger_stream_id.clone(),
        work_packet_id,
        micro_task_id,
        task_board_id,
        owner_session: message.owner_session.clone(),
        idempotency_key: format!(
            "operator-chat-artifact::{}::{}",
            message.run_id, message.message_id
        ),
        created_at_utc: Utc::now().to_rfc3339(),
        diagnostic_payload: json!({
            "flight_recorder": "ArtifactStore/EventLedger binding for operator-chat payload",
            "surface": OPERATOR_CHAT_SURFACE_ID,
            "message_id": &message.message_id,
            "payload_ref": &message.payload_ref,
        }),
    })
}

#[derive(Debug, Clone)]
struct CloudProjectionArtifactBindingPlan {
    run_id: String,
    trace_id: String,
    lane_id: String,
    event_ledger_stream_id: String,
    work_packet_id: String,
    micro_task_id: String,
    task_board_id: String,
    owner_session: String,
    cloud_input_artifact_ref: String,
    cloud_payload_artifact_ref: String,
    cloud_input_payload: Value,
    projected_payload: Value,
}

fn cloud_input_artifact_ref(run_id: &str) -> String {
    format!("artifact-store://operator-chat/{run_id}/cloud-input.json")
}

fn cloud_projection_payload_artifact_ref(run_id: &str) -> String {
    format!("artifact-store://operator-chat/{run_id}/cloud-projection-payload.json")
}

fn cloud_provider_kind_for_request(
    request: &SpawnRequest,
) -> Result<&'static str, OperatorChatError> {
    match request.byok_cloud_provider {
        Some(ByokCloudProvider::Anthropic) => Ok("anthropic"),
        Some(ByokCloudProvider::OpenAi) => Ok("openai"),
        None => Err(OperatorChatError::Invalid(
            "operator-chat cloud launch requires byok_cloud_provider".into(),
        )),
    }
}

fn cloud_requested_model_id(request: &SpawnRequest, contract: &DexterityLaunchContract) -> String {
    contract
        .candidate_model_ids
        .first()
        .cloned()
        .unwrap_or_else(|| {
            request
                .cloud_model_name
                .as_deref()
                .map(|model| format!("model://dexterity/byok_cloud/{model}"))
                .unwrap_or_else(|| request.instance_id.model_id.to_string())
        })
}

fn build_cloud_projected_payload(
    selection: &OperatorChatSelection,
    contract: &DexterityLaunchContract,
    provider_kind: &str,
    requested_model_id: &str,
    model_session_id: &str,
) -> Value {
    json!({
        "schema_id": "hsk.operator_chat_cloud_projection@1",
        "surface": OPERATOR_CHAT_SURFACE_ID,
        "run_id": &contract.run_id,
        "lane_id": &contract.lane_id,
        "trace_id": &contract.trace_id,
        "model_session_id": model_session_id,
        "provider_kind": provider_kind,
        "requested_model_id": requested_model_id,
        "prompt_sha256": sha256_hex(selection.prompt.as_bytes()),
        "working_dir": &selection.working_dir,
        "worktree_id": &selection.worktree_id,
        "context_bundle_id": &contract.context_bundle_id,
    })
}

fn build_cloud_input_payload(
    selection: &OperatorChatSelection,
    contract: &DexterityLaunchContract,
    provider_kind: &str,
    requested_model_id: &str,
    model_session_id: &str,
) -> Value {
    json!({
        "schema_id": "hsk.operator_chat_cloud_input@1",
        "surface": OPERATOR_CHAT_SURFACE_ID,
        "run_id": &contract.run_id,
        "lane_id": &contract.lane_id,
        "trace_id": &contract.trace_id,
        "model_session_id": model_session_id,
        "provider_kind": provider_kind,
        "requested_model_id": requested_model_id,
        "prompt_sha256": sha256_hex(selection.prompt.as_bytes()),
        "working_dir": &selection.working_dir,
        "worktree_id": &selection.worktree_id,
        "context_bundle_id": &contract.context_bundle_id,
        "projection_plan_ref": &contract.projection_plan_ref,
        "consent_receipt_ref": &contract.consent_receipt_ref,
        "payload_artifact_ref": cloud_projection_payload_artifact_ref(&contract.run_id),
    })
}

fn build_cloud_projection_artifact_binding_plan(
    selection: &OperatorChatSelection,
    request: &SpawnRequest,
) -> Result<Option<CloudProjectionArtifactBindingPlan>, OperatorChatError> {
    if request.provider != Some(ProviderKind::ByokCloud) {
        return Ok(None);
    }
    let contract = request.dexterity_launch.as_ref().ok_or_else(|| {
        OperatorChatError::Invalid(
            "operator-chat cloud launch requires a Dexterity launch contract".into(),
        )
    })?;
    let provider_kind = cloud_provider_kind_for_request(request)?;
    let requested_model_id = cloud_requested_model_id(request, contract);
    let model_session_id = dexterity_spawn_model_session_id(request);
    Ok(Some(CloudProjectionArtifactBindingPlan {
        run_id: contract.run_id.clone(),
        trace_id: contract.trace_id.clone(),
        lane_id: contract.lane_id.clone(),
        event_ledger_stream_id: contract.event_ledger_stream_id.clone(),
        work_packet_id: selection.work_packet().to_string(),
        micro_task_id: selection.micro_task().to_string(),
        task_board_id: contract.task_board_id.clone(),
        owner_session: selection.owner_session.clone(),
        cloud_input_artifact_ref: cloud_input_artifact_ref(&contract.run_id),
        cloud_payload_artifact_ref: cloud_projection_payload_artifact_ref(&contract.run_id),
        cloud_input_payload: build_cloud_input_payload(
            selection,
            contract,
            provider_kind,
            &requested_model_id,
            &model_session_id,
        ),
        projected_payload: build_cloud_projected_payload(
            selection,
            contract,
            provider_kind,
            &requested_model_id,
            &model_session_id,
        ),
    }))
}

/// Build a validation-passing [`NewModelLaneMessage`] under `run`/`lane` for a
/// captured activity. The message reuses the lane's locus binding so
/// `validate_locus_common` holds; `linked_span_contexts`, `routing`, and
/// `tool_gate_decision_refs` satisfy the ModelLaneStore message validators.
pub fn build_captured_message(
    run: &ModelLaneRunRecord,
    lane: &ModelLaneRecord,
    plan: &CapturedMessagePlan,
    ordered_index: u64,
    turn_role: &str,
) -> Result<NewModelLaneMessage, OperatorChatError> {
    let locus = lane.locus_binding.clone().ok_or_else(|| {
        OperatorChatError::Invalid(format!(
            "lane {} is missing a locus_binding required for capture",
            lane.lane_id
        ))
    })?;

    let payload_sha256 = sha256_hex(&canonical_json_bytes(&plan.payload));

    let message_id = format!("mlm-{}-{turn_role}-{ordered_index}", lane.lane_id);
    let message_span_id = format!("span-msg-{}-{turn_role}-{ordered_index}", lane.lane_id);
    // linked_span_contexts must be non-empty and MUST NOT contain message_span_id.
    let linked_span_contexts = vec![run.run_span_id.clone()];

    let tool_gate_decision_refs = if plan.is_tool {
        vec![format!(
            "toolgate://operator-chat/{}/observed/{ordered_index}",
            lane.lane_id
        )]
    } else {
        Vec::new()
    };

    let diagnostic_payload = json!({
        "activity_kind": plan.activity_kind,
        "message_kind": message_kind_label(&plan.kind),
        "surface": OPERATOR_CHAT_SURFACE_ID,
        "turn_role": turn_role,
        "ordered_index": ordered_index,
        "capture": plan.payload,
    });

    Ok(NewModelLaneMessage {
        message_id: message_id.clone(),
        run_id: run.run_id.clone(),
        trace_id: run.trace_id.clone(),
        message_span_id,
        parent_span_id: Some(lane.lane_span_id.clone()),
        linked_span_contexts,
        from_lane_id: lane.lane_id.clone(),
        to_lane: ModelLaneTarget::Coordinator,
        routing: Some(ModelLaneRoutingMetadata {
            target_role: "operator_chat_coordinator".to_string(),
            target_session: run.coordinator_session_id.clone(),
            correlation_id: run.run_id.clone(),
            requires_ack: false,
            ack_for: None,
        }),
        kind: plan.kind.clone(),
        payload_ref: format!("artifact://operator-chat/{}/{message_id}", run.run_id),
        payload_sha256,
        event_ledger_stream_id: run.event_ledger_stream_id.clone(),
        summary: plan.summary.clone(),
        authority: ModelLaneAuthority::Advisory,
        promotion_decision_id: None,
        promotion_gate_ref: None,
        promotion_receipt_ref: None,
        validator_verdict_ref: None,
        operator_decision_ref: None,
        promoted_artifact_ref: None,
        promoted_artifact_sha256: None,
        promoted_artifact_version: None,
        tool_gate_decision_refs,
        coordinator_session_id: locus.coordinator_session_id.clone(),
        work_packet_id: Some(locus.work_packet_id.clone()),
        micro_task_id: Some(locus.micro_task_id.clone()),
        task_board_id: locus.task_board_id.clone(),
        owner_session: locus.owner_session.clone(),
        locus_binding: Some(locus),
        idempotency_key: format!(
            "operator-chat::{}::{}::{turn_role}::{ordered_index}",
            run.run_id, lane.lane_id
        ),
        replay_order_key: format!("operator-chat-{ordered_index:020}"),
        replay_after_event_ledger_seq: None,
        proposal_ref: None,
        crdt_update_ref: None,
        crdt_base_snapshot_ref: None,
        crdt_state_vector: None,
        crdt_proposal_ref: None,
        crdt_stale_base_ref: None,
        failstate_code: None,
        reason_ref: None,
        recovery_hint_ref: None,
        created_at_utc: Utc::now().to_rfc3339(),
        diagnostic_payload,
    })
}

/// Records CLI-wrapper capture as typed ModelLane messages + Flight Recorder
/// business events. This is the core net-new binding of MT-012.
pub struct ModelLaneCaptureRecorder {
    store: ModelLaneStore,
    recorder: Arc<dyn FlightRecorder>,
    redactor: Arc<dyn SecretRedactor>,
}

impl ModelLaneCaptureRecorder {
    pub fn new(store: ModelLaneStore, recorder: Arc<dyn FlightRecorder>) -> Self {
        Self {
            store,
            recorder,
            redactor: Arc::new(PatternRedactor),
        }
    }

    pub fn with_redactor(mut self, redactor: Arc<dyn SecretRedactor>) -> Self {
        self.redactor = redactor;
        self
    }

    /// Record ONE captured activity. Persist the authoritative typed
    /// ModelLaneMessage/payload binding first, then emit the Flight Recorder
    /// `FR-EVT-AGENT-*` business event. A terminal lane denial therefore cannot
    /// leave a misleading Flight Recorder activity with no durable capture.
    pub async fn record_activity(
        &self,
        run: &ModelLaneRunRecord,
        lane: &ModelLaneRecord,
        model_id: ModelId,
        request_id: Uuid,
        ordered_index: u64,
        activity: &AgentActivity,
    ) -> Result<crate::swarm_orchestration::model_lane::ModelLaneMessageRecord, OperatorChatError>
    {
        self.record_activity_with_adapter(run, lane, model_id, request_id, ordered_index,
            activity, OPERATOR_CHAT_CLI_ADAPTER).await
    }

    async fn record_activity_with_adapter(
        &self,
        run: &ModelLaneRunRecord,
        lane: &ModelLaneRecord,
        model_id: ModelId,
        request_id: Uuid,
        ordered_index: u64,
        activity: &AgentActivity,
        adapter: &str,
    ) -> Result<crate::swarm_orchestration::model_lane::ModelLaneMessageRecord, OperatorChatError>
    {
        // Build both representations before either write, but persist ModelLane
        // authority first so cancellation/terminal guards fail closed without a
        // phantom activity event.
        let event = agent_activity_event(
            model_id,
            request_id,
            ordered_index,
            Some(lane.model_session_id.as_str()),
            adapter,
            activity,
            self.redactor.as_ref(),
        );
        let plan = plan_from_activity(activity);
        let message = build_captured_message(run, lane, &plan, ordered_index, "model")?;
        let binding = build_message_payload_binding(&message, plan.payload)?;
        let record = self
            .store
            .record_message_with_payload_binding(message, binding)
            .await?;
        self.recorder.record_event(event).await?;
        Ok(record)
    }

    /// Capture a whole CLI stdout stream: for every line, run the real
    /// [`parse_agent_activity_line`] and persist ONE ModelLaneMessage per
    /// *completed* activity block. Streaming delta / `item.updated` lines yield
    /// no activities, so there is exactly one message per completed block (F5).
    ///
    /// `start_index` is the first `ordered_index` to use (so successive capture
    /// calls on the same lane keep distinct, monotonic idempotency keys). Returns
    /// the persisted messages; the next free index is `start_index + records.len()`.
    pub async fn capture_cli_stream<I, S>(
        &self,
        run: &ModelLaneRunRecord,
        lane: &ModelLaneRecord,
        model_id: ModelId,
        request_id: Uuid,
        cli_kind: CliKind,
        start_index: u64,
        stdout_lines: I,
    ) -> Result<
        Vec<crate::swarm_orchestration::model_lane::ModelLaneMessageRecord>,
        OperatorChatError,
    >
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        let mut records = Vec::new();
        let mut ordered_index: u64 = start_index;
        for line in stdout_lines {
            for activity in parse_agent_activity_line(cli_kind, line.as_ref()) {
                let record = self
                    .record_activity(run, lane, model_id, request_id, ordered_index, &activity)
                    .await?;
                records.push(record);
                ordered_index += 1;
            }
        }
        Ok(records)
    }

    /// Persist the operator's own prompt as a typed message under a
    /// `HUMAN_OPERATOR` ModelLane (contract F1 / spec 1619). The prompt is a
    /// `Status` message discriminated by `diagnostic_payload.activity_kind="text"`.
    pub async fn record_operator_prompt(
        &self,
        operator_run: &ModelLaneRunRecord,
        operator_lane: &ModelLaneRecord,
        prompt: &str,
    ) -> Result<crate::swarm_orchestration::model_lane::ModelLaneMessageRecord, OperatorChatError>
    {
        let plan = CapturedMessagePlan {
            kind: ModelLaneMessageKind::Status,
            activity_kind: AgentActivityKind::Text.label(),
            summary: short_summary(prompt, "operator_prompt"),
            payload: json!({
                "activity_kind": AgentActivityKind::Text.label(),
                "turn_role": "operator",
                "text": prompt,
            }),
            is_tool: false,
        };
        let activity = AgentActivity::Text {
            text: prompt.to_string(),
        };
        let event = agent_activity_event(
            model_id_for_operator_prompt_event(operator_run, operator_lane),
            uuid_or_new(operator_run.trace_id.as_str()),
            0,
            Some(operator_lane.model_session_id.as_str()),
            OPERATOR_CHAT_CLI_ADAPTER,
            &activity,
            self.redactor.as_ref(),
        );
        let message = build_captured_message(operator_run, operator_lane, &plan, 0, "operator")?;
        let binding = build_message_payload_binding(&message, plan.payload)?;
        let record = self
            .store
            .record_message_with_payload_binding(message, binding)
            .await?;
        self.recorder.record_event(event).await?;
        Ok(record)
    }
}

fn model_id_for_operator_prompt_event(
    operator_run: &ModelLaneRunRecord,
    operator_lane: &ModelLaneRecord,
) -> ModelId {
    operator_lane
        .model_id
        .as_deref()
        .or(operator_run.selected_model_id.as_deref())
        .and_then(|value| Uuid::parse_str(value).ok())
        .map(ModelId::from)
        .unwrap_or_else(ModelId::new_v7)
}

fn uuid_or_new(value: &str) -> Uuid {
    Uuid::parse_str(value).unwrap_or_else(|_| Uuid::now_v7())
}

async fn record_hbr_int_009_tiers(
    store: &ModelLaneStore,
    run: &ModelLaneRunRecord,
) -> Result<(), OperatorChatError> {
    let work_packet_id = run.work_packet_id.clone().ok_or_else(|| {
        OperatorChatError::Invalid(format!(
            "run {} is missing work_packet_id required for HBR-INT-009 tier status",
            run.run_id
        ))
    })?;
    let micro_task_id = run.micro_task_id.clone().ok_or_else(|| {
        OperatorChatError::Invalid(format!(
            "run {} is missing micro_task_id required for HBR-INT-009 tier status",
            run.run_id
        ))
    })?;
    let task_board_id = run.task_board_id.clone().ok_or_else(|| {
        OperatorChatError::Invalid(format!(
            "run {} is missing task_board_id required for HBR-INT-009 tier status",
            run.run_id
        ))
    })?;

    // Write the COMPLETE run-level HBR-INT-009 triplet (one envelope per
    // ModelLaneRun, per the MT-011 run-level design). The launch service can
    // PROVE only its own EventLedger/Flight Recorder production, so it records
    // FlightRecorder=Wired. It cannot prove the native internal_diagnostics
    // producer or the authenticated Palmistry watcher from inside the launch
    // context: those tiers are produced and read back later through the
    // authenticated diagnostic observation/readback boundary
    // (`api/palmistry.rs`), which records them as Wired against the same run
    // with a later EventLedger seq. `diagnostic_tier_posture` selects the
    // latest state per tier, so those deferred_with_reason rows are cleanly
    // superseded (deferred -> wired) once the observation lands. Recording them
    // as deferred_with_reason (not Wired) is the honest launch-context state and
    // satisfies `validate_diagnostic_tier_posture`, which requires all three
    // tiers present, none Missing, and a non-null follow_up_ref on every
    // deferred tier.
    let rows: [(
        ModelLaneDiagnosticTier,
        ModelLaneDiagnosticTierState,
        &str,
        Option<String>,
    ); 3] = [
        (
            ModelLaneDiagnosticTier::FlightRecorder,
            ModelLaneDiagnosticTierState::Wired,
            "operator-chat launch/capture emitted this run's EventLedger row",
            None,
        ),
        (
            ModelLaneDiagnosticTier::InternalDiagnostics,
            ModelLaneDiagnosticTierState::DeferredWithReason,
            "native internal_diagnostics producer records this tier later through the authenticated diagnostic observation/readback boundary; deferred in the launch-service context which cannot prove the native producer",
            Some(format!(
                "internal-diagnostics://observation-readback/run/{}",
                run.run_id
            )),
        ),
        (
            ModelLaneDiagnosticTier::Palmistry,
            ModelLaneDiagnosticTierState::DeferredWithReason,
            "authenticated Palmistry watcher records this tier later through the diagnostic observation/readback boundary; deferred in the launch-service context which cannot prove the out-of-process watcher",
            Some(format!(
                "palmistry://observation-readback/run/{}",
                run.run_id
            )),
        ),
    ];

    for (tier, state, reason, follow_up_ref) in rows {
        let tier_label = tier.as_str();
        store
            .record_diagnostic_tier_status(NewModelLaneDiagnosticTierStatus {
                diagnostic_status_id: format!("diag-{}-{}", run.run_id, tier_label),
                behavior_id: "HBR-INT-009".to_string(),
                run_id: run.run_id.clone(),
                tier,
                state,
                reason: reason.to_string(),
                evidence_ref: format!("eventledger://kernel/{}", run.event_ledger_event_id),
                follow_up_ref,
                event_ledger_stream_id: run.event_ledger_stream_id.clone(),
                work_packet_id: work_packet_id.clone(),
                micro_task_id: micro_task_id.clone(),
                task_board_id: task_board_id.clone(),
                owner_session: run.owner_session.clone(),
                idempotency_key: format!(
                    "operator-chat-hbr-int-009::{}::{}",
                    run.run_id, tier_label
                ),
                diagnostic_payload: json!({
                    "surface": OPERATOR_CHAT_SURFACE_ID,
                    "behavior_id": "HBR-INT-009",
                    "tier": tier_label,
                    "state": state.as_str(),
                }),
            })
            .await?;
    }
    Ok(())
}

fn operator_lane_for_run(
    run: &ModelLaneRunRecord,
    selection: &OperatorChatSelection,
) -> Result<NewModelLane, OperatorChatError> {
    operator_lane_for_owner(run, &selection.owner_session)
}

fn operator_lane_for_owner(run: &ModelLaneRunRecord, owner_session: &str) -> Result<NewModelLane, OperatorChatError> {
    let work_packet_id = run.work_packet_id.clone().ok_or_else(|| {
        OperatorChatError::Invalid(format!(
            "run {} is missing work_packet_id required for operator prompt lane",
            run.run_id
        ))
    })?;
    let micro_task_id = run.micro_task_id.clone().ok_or_else(|| {
        OperatorChatError::Invalid(format!(
            "run {} is missing micro_task_id required for operator prompt lane",
            run.run_id
        ))
    })?;
    let task_board_id = run.task_board_id.clone().ok_or_else(|| {
        OperatorChatError::Invalid(format!(
            "run {} is missing task_board_id required for operator prompt lane",
            run.run_id
        ))
    })?;
    let lane_id = format!("dexterity-lane-human-operator-{}", run.run_id);
    let session_id = format!("operator-session-{}", run.run_id);
    let model_session_id = format!("operator-model-session-{}", run.run_id);
    let locus = ModelLaneLocusBinding {
        work_packet_id: work_packet_id.clone(),
        micro_task_id: micro_task_id.clone(),
        task_board_id: Some(task_board_id.clone()),
        coordinator_session_id: run.coordinator_session_id.clone(),
        session_id: session_id.clone(),
        model_session_id: model_session_id.clone(),
        owner_session: owner_session.to_owned(),
        locus_binding_ref: format!("locus://operator-chat/{}/{lane_id}", run.run_id),
    };
    Ok(NewModelLane {
        engine_profile: None,
        lane_id: lane_id.clone(),
        run_id: run.run_id.clone(),
        trace_id: run.trace_id.clone(),
        lane_span_id: format!("span-{lane_id}-lane"),
        event_ledger_stream_id: run.event_ledger_stream_id.clone(),
        kind: ModelLaneKind::HumanOperator,
        role: "operator".to_string(),
        backend: "human".to_string(),
        model_id: None,
        session_id,
        model_session_id,
        adapter_id: "operator_chat_human".to_string(),
        runtime_binding: RuntimeBinding::Human,
        launch_authority: LaunchAuthority::Operator,
        provider_kind: ModelLaneProviderKind::Human,
        capability_token_ids: vec!["capability://operator-chat/human-prompt".to_string()],
        effective_capability_snapshot_ref: Some(format!(
            "capability-snapshot://operator-chat/{lane_id}/human"
        )),
        capability_negotiation_ref: Some(format!(
            "capability-negotiation://operator-chat/{}/human",
            run.run_id
        )),
        provider_feature_profile_ref: Some("provider-feature-profile://human-operator".to_string()),
        requested_execution_policy_ref: Some("execution-policy://operator-chat/human".to_string()),
        effective_execution_policy_ref: Some("execution-policy://operator-chat/human".to_string()),
        projection_plan_ref: None,
        consent_receipt_ref: None,
        tool_gate_decision_refs: vec![format!(
            "toolgate://operator-chat/{}/human-prompt",
            run.run_id
        )],
        status: ModelLaneStatus::Ready,
        recovery_state: ModelLaneRecoveryState::Restartable,
        heartbeat_at_utc: Some(Utc::now().to_rfc3339()),
        lease_expires_at_utc: None,
        reclaim_after_utc: None,
        restart_generation: 0,
        cancellation_ref: Some(format!("cancel-token://operator-chat/{}/human", run.run_id)),
        reclaim_policy_ref: Some("reclaim-policy://operator-chat/human".to_string()),
        terminal_status_mapping_ref: Some("terminal-status://operator-chat/human".to_string()),
        process_ownership_ref: None,
        no_os_process_reason_ref: Some(format!("no-os://operator-chat/{}/human", run.run_id)),
        backpressure_ref: None,
        loop_counter_ref: Some(format!("loop-counter://operator-chat/{}/human", run.run_id)),
        last_runtime_status_ref: Some(format!(
            "runtime-status://operator-chat/{}/human-ready",
            run.run_id
        )),
        last_recovery_event_ref: None,
        failstate_code: None,
        startup_failure_ref: None,
        reason_ref: None,
        recovery_hint_ref: Some("usermanual://operator-chat-launch#human-operator".to_string()),
        work_packet_id: Some(work_packet_id),
        micro_task_id: Some(micro_task_id),
        task_board_id: Some(task_board_id),
        owner_session: owner_session.to_owned(),
        locus_binding: Some(locus),
    })
}

fn subagent_launch_request(selection: &OperatorChatSelection) -> DexterityLaunchAdapterRequest {
    let run_uuid = Uuid::now_v7();
    let lane_uuid = Uuid::now_v7();
    let run_id = format!("dexterity-run-subagent-{run_uuid}");
    let lane_id = format!("dexterity-lane-subagent-{lane_uuid}");
    let wp = selection.work_packet().to_string();
    let mt = selection.micro_task().to_string();
    let task_board_id = format!("task-board://operator-chat/{wp}");
    let session_id = format!("subagent-session-{lane_uuid}");
    let model_session_id = format!("subagent-model-session-{lane_uuid}");
    let candidate = selection.model_id.clone();
    DexterityLaunchAdapterRequest {
        adapter_kind: DexterityLaunchAdapterKind::Subagent,
        run_id: run_id.clone(),
        lane_id: lane_id.clone(),
        trace_id: format!("trace-{run_id}"),
        run_span_id: format!("span-{run_id}-run"),
        lane_span_id: format!("span-{lane_id}-lane"),
        coordinator_session_id: selection.parent_session_id.clone(),
        routing_policy: "dexterity_subagent".into(),
        context_bundle_id: format!(
            "context-bundle://operator-chat/{}",
            selection.parent_session_id
        ),
        event_ledger_stream_id: format!("event-ledger://operator-chat/{run_id}"),
        artifact_namespace: format!("artifact://operator-chat/{run_id}"),
        work_packet_id: Some(wp.clone()),
        micro_task_id: Some(mt.clone()),
        task_board_id: Some(task_board_id.clone()),
        owner_session: selection.owner_session.clone(),
        locus_binding_ref: format!("locus://operator-chat/{wp}/{mt}/{lane_id}"),
        role: "subagent_coder".into(),
        backend: None,
        adapter_id: None,
        model_id: Some(candidate.clone()),
        session_id,
        model_session_id,
        extra_capability_token_ids: vec![],
        requested_tool_capability_tokens: vec!["tool-capability://read-context".into()],
        effective_capability_snapshot_ref: None,
        capability_negotiation_ref: None,
        provider_feature_profile_ref: None,
        requested_execution_policy_ref: None,
        effective_execution_policy_ref: None,
        projection_plan_ref: None,
        consent_receipt_ref: None,
        tool_gate_decision_refs: vec![format!("toolgate://operator-chat/{lane_id}/read-context")],
        status: Some(ModelLaneStatus::Ready),
        heartbeat_at_utc: Some(Utc::now().to_rfc3339()),
        lease_expires_at_utc: None,
        reclaim_after_utc: None,
        restart_generation: 0,
        cancellation_ref: Some(format!("cancel-token://operator-chat/{lane_id}")),
        reclaim_policy_ref: Some("reclaim-policy://operator-chat/subagent".into()),
        terminal_status_mapping_ref: Some("terminal-status://operator-chat/subagent".into()),
        process_ownership_ref: None,
        no_os_process_reason_ref: None,
        backpressure_ref: None,
        loop_counter_ref: Some(format!("loop-counter://operator-chat/{lane_id}")),
        last_runtime_status_ref: Some(format!("runtime-status://operator-chat/{lane_id}/ready")),
        last_recovery_event_ref: None,
        startup_failure_code: None,
        startup_failure_ref: None,
        reason_ref: None,
        run_recovery_hint_ref: Some("usermanual://operator-chat-launch#subagent".into()),
        lane_recovery_hint_ref: Some("usermanual://operator-chat-launch#subagent".into()),
        memory_pack_ref: format!("memory-pack://operator-chat/{run_id}"),
        memory_pack_hash: sha256_hex(
            format!(
                "{}:{}:{}:{}",
                selection.owner_session,
                selection.parent_session_id,
                candidate,
                selection.working_dir
            )
            .as_bytes(),
        ),
        determinism_mode: "deterministic_replay".into(),
        budget_summary_ref: format!("budget://operator-chat/{run_id}"),
        selected_model_id: Some(candidate.clone()),
        candidate_model_ids: vec![candidate],
        procedural_review_status: "operator_chat_subagent_registry_normalized".into(),
        truncation_warning_ref: None,
        rejection_reason_refs: vec![],
    }
}

/// Force the CLI bridge into a JSON-stream output format so captured activities
/// are TYPED (`tool_call`/`thinking`/`text`) instead of a wall of `Other{raw}`
/// (contract F8). Idempotently ensures a `--output-format stream-json` flag pair
/// is present in the arg template and sets [`CliOutputFormat::JsonStream`].
pub fn force_json_stream_output(
    mut config: crate::model_runtime::cloud::CliBridgeConfig,
) -> crate::model_runtime::cloud::CliBridgeConfig {
    config.output_format = CliOutputFormat::JsonStream;
    let already = config
        .args_template
        .iter()
        .any(|arg| arg == "stream-json" || arg == "--json" || arg == "json-stream");
    if !already {
        config.args_template.push("--output-format".to_string());
        config.args_template.push("stream-json".to_string());
    }
    config
}

/// Launch service for the operator chat/launch surface. Owns the sanctioned
/// launch authority ([`SwarmCoordinator::spawn_session`]), the local model
/// catalog (MT-014), and the Flight Recorder (for the selection-decision audit).
#[derive(Clone)]
pub struct OperatorChatLaunchService {
    coordinator: Arc<SwarmCoordinator>,
    catalog: Option<Arc<ModelCatalog>>,
    recorder: Arc<dyn FlightRecorder>,
    instance_counter: Arc<AtomicU32>,
    _process_ledger_runtime: Option<RetainedLedgerBatcher>,
}

pub(crate) struct CapturedOperatorChatLaunch {
    scope: ExactResourceScopeAttribution,
    selection_json: Value,
    original_request_json: Value,
    original_request: SpawnRequest,
    original_selection: OperatorChatSelection,
}

impl CapturedOperatorChatLaunch {
    fn capture(
        scope: ExactResourceScopeAttribution,
        selection: &OperatorChatSelection,
        request: &SpawnRequest,
    ) -> Result<Arc<Self>, OperatorChatError> {
        let encode = |error: serde_json::Error| OperatorChatError::Invalid(
            format!("original operator-chat launch encoding failed: {error}")
        );
        Ok(Arc::new(Self {
            scope,
            selection_json: serde_json::to_value(selection).map_err(encode)?,
            original_request_json: serde_json::to_value(request).map_err(encode)?,
            original_request: request.clone(), original_selection: selection.clone(),
        }))
    }

    pub(crate) fn matches_request(&self, request: &SpawnRequest) -> bool {
        self.original_request == *request
    }

    pub(crate) fn scope(&self) -> &ExactResourceScopeAttribution { &self.scope }

    pub(crate) fn original_selection(&self) -> &OperatorChatSelection { &self.original_selection }
    pub(crate) fn selection_json(&self) -> &Value { &self.selection_json }

    pub(crate) fn original_request_json(&self) -> &Value {
        &self.original_request_json
    }
}

impl OperatorChatLaunchService {
    fn capture_original_launch(
        &self,
        selection: &OperatorChatSelection,
        request: &SpawnRequest,
        scope: Option<&ExactResourceScopeAttribution>,
    ) -> Result<Option<Arc<CapturedOperatorChatLaunch>>, OperatorChatError> {
        let Some(scope) = scope else { return Ok(None); };
        let store = self.coordinator.model_lane_store().ok_or_else(|| {
            OperatorChatError::Invalid("original launch source store unavailable".into())
        })?;
        if store.access().exact_read_scope() != Some(scope) {
            return Err(OperatorChatError::Invalid("original launch source scope differs".into()));
        }
        store.access().authorize_exact_request(scope).map_err(|_| {
            OperatorChatError::Invalid("original launch source authority unavailable".into())
        })?;
        CapturedOperatorChatLaunch::capture(scope.clone(), selection, request).map(Some)
    }

    pub(crate) async fn original_launch_for_registered_validator(
        &self,
        scope: &ExactResourceScopeAttribution,
        registered: &super::coordinator::RegisteredValidatorInstance,
    ) -> Result<Arc<super::coordinator::BoundOperatorChatLaunch>, OperatorChatError> {
        let store = self.coordinator.model_lane_store().ok_or_else(|| {
            OperatorChatError::Invalid("original launch source store unavailable".into())
        })?;
        if store.access().exact_read_scope() != Some(scope) {
            return Err(OperatorChatError::Invalid("original launch source scope differs".into()));
        }
        store.access().authorize_exact_request(scope).map_err(|_| {
            OperatorChatError::Invalid("original launch source authority unavailable".into())
        })?;
        let source = self.coordinator.original_operator_chat_launch(registered, scope)?;
        let contract = source.launch_contract().ok_or_else(|| {
            OperatorChatError::Invalid("original launch contract unavailable".into())
        })?;
        let replay = store.replay_run(&contract.run_id).await?;
        let lane = replay.lanes.iter().find(|lane| lane.lane_id == contract.lane_id)
            .ok_or_else(|| OperatorChatError::Invalid("original launch lane unavailable".into()))?;
        if replay.run.trace_id != contract.trace_id
            || replay.run.event_ledger_stream_id != contract.event_ledger_stream_id
            || lane.run_id != contract.run_id
            || lane.trace_id != contract.trace_id
            || lane.event_ledger_stream_id != contract.event_ledger_stream_id
            || lane.model_session_id != super::model_lane::dexterity_spawn_model_session_id(
                source.committed_request()
            )
        {
            return Err(OperatorChatError::Invalid("original launch canonical binding differs".into()));
        }
        store.access().authorize_exact_request(scope).map_err(|_| {
            OperatorChatError::Invalid("original launch source authority unavailable".into())
        })?;
        let current = self.coordinator.original_operator_chat_launch(registered, scope)?;
        if !Arc::ptr_eq(&source, &current) {
            return Err(OperatorChatError::Invalid("original launch incarnation changed".into()));
        }
        Ok(source)
    }


    fn require_exact_cloud_scope(
        &self,
        request_scope: Option<&ExactResourceScopeAttribution>,
    ) -> Result<ExactResourceScopeAttribution, OperatorChatError> {
        let store = self.coordinator.model_lane_store().ok_or_else(|| {
            OperatorChatError::Invalid(
                "RESOURCE_SCOPE_REQUIRED: operator-chat cloud authority requires a ModelLaneStore"
                    .into(),
            )
        })?;
        let store_scope = store.access().exact_read_scope().cloned().ok_or_else(|| {
            OperatorChatError::Invalid(
                "RESOURCE_SCOPE_REQUIRED: operator-chat cloud authority requires complete account/principal/session/AccessSpace/workspace scope"
                    .into(),
            )
        })?;
        if request_scope.is_some_and(|scope| scope != &store_scope) {
            return Err(OperatorChatError::Invalid(
                "RESOURCE_SCOPE_MISMATCH: HTTP cloud-launch scope differs from the durable ModelLaneStore scope"
                    .into(),
            ));
        }
        Ok(store_scope)
    }

    /// The live coordinator this service launches through.
    ///
    /// WP-1 MT-021 AC-3: the operator-facing swarm concurrency control must move
    /// the REAL model-session admission cap, and the coordinator singleton is
    /// only reachable from the API through this service. Exposed read-only so a
    /// route can call `set_max_concurrent`/`max_concurrent` without gaining any
    /// other coordinator authority.
    pub fn coordinator(&self) -> &Arc<SwarmCoordinator> {
        &self.coordinator
    }

    pub fn new(
        coordinator: Arc<SwarmCoordinator>,
        catalog: impl Into<Option<Arc<ModelCatalog>>>,
        recorder: Arc<dyn FlightRecorder>,
    ) -> Self {
        Self {
            coordinator,
            catalog: catalog.into(),
            recorder,
            instance_counter: Arc::new(AtomicU32::new(1)),
            _process_ledger_runtime: None,
        }
    }

    pub fn new_with_process_ledger_runtime(
        coordinator: Arc<SwarmCoordinator>,
        catalog: impl Into<Option<Arc<ModelCatalog>>>,
        recorder: Arc<dyn FlightRecorder>,
        process_ledger_runtime: RetainedLedgerBatcher,
    ) -> Self {
        Self {
            coordinator,
            catalog: catalog.into(),
            recorder,
            instance_counter: Arc::new(AtomicU32::new(1)),
            _process_ledger_runtime: Some(process_ledger_runtime),
        }
    }

    /// Persist a real Operator Chat `SingleRun` ProjectionPlan and its derived
    /// ConsentReceipt through ModelLane/EventLedger authority.
    #[cfg(feature = "runtime-full")]
    pub(crate) async fn grant_single_run_cloud_consent_authenticated(
        &self,
        bootstrap: &crate::AccountBootstrapState,
        authorization: crate::storage::surreal::resource_authority::AuthorizationRequest,
        request: OperatorChatSingleRunCloudConsentGrant,
    ) -> Result<(ModelLaneCloudProjectionPlanRecord, ModelLaneCloudConsentReceiptRecord), OperatorChatError> {
        use crate::storage::surreal::resource_authority::{
            RecordUserScope, ResourceAction, ResourceAuthorityError, ResourceKind,
        };
        use crate::storage::surreal::{with_model_lane_subject, ModelLaneOperationSubject, SurrealStorageError};

        let denied = || OperatorChatError::ModelLane(ModelLaneError::AuthorityDenied(
            "authenticated cloud consent authority unavailable".into(),
        ));
        let store = self.coordinator.model_lane_store().ok_or_else(denied)?;
        let storage = store.surreal_storage();
        let lifecycle = storage.resource_access_lifecycle();
        if bootstrap.surreal.resource_access_lifecycle() != lifecycle
            || store.access().lifecycle_authority() != Some(&lifecycle)
        {
            return Err(denied());
        }
        store.access().require_lifecycle_active().map_err(|_| denied())?;
        let exact = store.access().exact_read_scope().ok_or_else(denied)?;
        let write_scope = store.write_scope().ok_or_else(denied)?;
        let write_exact = ExactResourceScopeAttribution::try_from_resource_scope(write_scope)
            .map_err(|_| denied())?;
        if &write_exact != exact
            || authorization.resource_kind != ResourceKind::Workspace
            || authorization.capability_id != "fs.write"
            || authorization.action != ResourceAction::Create
            || authorization.external_resource_id != exact.workspace_id.as_str()
            || authorization.session_token.trim().is_empty()
            || authorization.channel_binding_hash.as_ref().is_none_or(|channel| channel.trim().is_empty())
        {
            return Err(denied());
        }
        store.access().authorize_exact_request(exact).map_err(|_| denied())?;
        let session_token = authorization.session_token.clone();
        let channel_binding_hash = authorization.channel_binding_hash.clone();
        let decision = bootstrap.surreal.authorize_protected_resource(authorization).await
            .map_err(|error| match error {
                ResourceAuthorityError::Storage(error) => OperatorChatError::ModelLane(ModelLaneError::Surreal(error)),
                ResourceAuthorityError::Entropy(_) => OperatorChatError::ModelLane(ModelLaneError::Surreal(
                    SurrealStorageError::ResourceAuthorityUnavailable,
                )),
                ResourceAuthorityError::Denied { .. } | ResourceAuthorityError::InvalidInput(_) => denied(),
            })?;
        if decision.account_id != exact.owner_account_id.to_string()
            || decision.principal_id != exact.actor_principal_id.to_string()
            || decision.session_id != exact.authenticated_session_id.to_string()
            || decision.access_space_id != exact.access_space_id.to_string()
            || decision.resource_id.is_empty()
            || decision.grant_id.is_empty()
            || !bootstrap.capability_registry.profile_can(&decision.capability_profile_id, "fs.write")
                .unwrap_or(false)
        {
            return Err(denied());
        }
        store.access().authorize_exact_request(exact).map_err(|_| denied())?;
        let record_user = RecordUserScope {
            grant_id: Some(decision.grant_id),
            workspace_id: Some(exact.workspace_id.as_str().to_owned()),
            session_token,
            channel_binding_hash,
            resource_id: decision.resource_id,
            session_id: decision.session_id,
            capability_id: "fs.write".to_owned(),
            action: ResourceAction::Create,
        };
        storage.with_record_user_scope(record_user, with_model_lane_subject(
            ModelLaneOperationSubject {
                kind: ResourceKind::Workspace,
                external_id: exact.workspace_id.as_str().to_owned(),
            },
            self.grant_single_run_cloud_consent(request),
        )).await
    }

    pub async fn grant_single_run_cloud_consent(
        &self,
        request: OperatorChatSingleRunCloudConsentGrant,
    ) -> Result<
        (
            ModelLaneCloudProjectionPlanRecord,
            ModelLaneCloudConsentReceiptRecord,
        ),
        OperatorChatError,
    > {
        self.require_exact_cloud_scope(None)?;
        if request.projection_plan.consent_scope != ModelLaneCloudConsentScope::SingleRun {
            return Err(OperatorChatError::Invalid(
                "operator-chat SingleRun grant requires consent_scope=single_run".into(),
            ));
        }
        let store = self.coordinator.model_lane_store().ok_or_else(|| {
            OperatorChatError::Invalid(
                "operator-chat SingleRun grant requires a ModelLaneStore".into(),
            )
        })?;

        // HBR-PRIV-005: identity is server-derived, never client-supplied. The
        // caller may propose a plan and a provenance label; it may not tell the
        // backend whose data is being exported or who approved it. Overriding
        // rather than validating is deliberate — there is no legitimate case
        // where a client knows a source scope the store does not.
        let approver = AccountBoundAuthority::from_access(store.access());
        let mut projection_plan = request.projection_plan;
        projection_plan.export_delegation.source_scope = approver.clone();
        projection_plan.export_delegation.authorization_receipt_ref =
            Some(request.consent_receipt_id.clone());
        let stored_plan = store.record_cloud_projection_plan(projection_plan).await?;
        let receipt = store
            .record_cloud_consent_receipt(NewModelLaneCloudConsentReceipt {
                consent_receipt_id: request.consent_receipt_id,
                projection_plan_id: stored_plan.projection_plan_id.clone(),
                projection_plan_hash: stored_plan.projection_plan_hash.clone(),
                run_id: stored_plan.run_id.clone(),
                trace_id: stored_plan.trace_id.clone(),
                lane_id: None,
                model_session_id: None,
                provider_kind: None,
                requested_model_id: None,
                scope_hash: stored_plan.scope_hash.clone(),
                consent_scope: ModelLaneCloudConsentScope::SingleRun,
                target_bindings: stored_plan.target_bindings.clone(),
                retention_policy: stored_plan.retention_policy.clone(),
                export_posture: stored_plan.export_posture.clone(),
                fan_out_targets: stored_plan.fan_out_targets.clone(),
                approved: true,
                approver,
                approved_by_ref: request.approved_by_ref,
                approved_at_utc: request.approved_at_utc,
                valid_from_utc: request.valid_from_utc,
                valid_until_utc: request.valid_until_utc,
                revoked_at_utc: None,
                revocation_ref: None,
                revocation_input_hash: None,
                status: ModelLaneCloudConsentReceiptStatus::Approved,
                event_ledger_stream_id: stored_plan.event_ledger_stream_id.clone(),
                work_packet_id: stored_plan.work_packet_id.clone(),
                micro_task_id: stored_plan.micro_task_id.clone(),
                task_board_id: stored_plan.task_board_id.clone(),
                owner_session: stored_plan.owner_session.clone(),
                idempotency_key: request.consent_idempotency_key,
                created_at_utc: stored_plan.created_at_utc.clone(),
                user_manual_behavior_ref: stored_plan.user_manual_behavior_ref.clone(),
                diagnostic_payload: request.diagnostic_payload,
            })
            .await?;
        Ok((stored_plan, receipt))
    }

    /// Revoke an Operator Chat cloud grant through the same atomic ModelLane
    /// cancellation path used by coordinator-managed lanes.
    pub async fn revoke_single_run_cloud_consent(
        &self,
        consent_receipt_id: &str,
        revoked_by_ref: &str,
        reason: &str,
    ) -> Result<Vec<ModelLaneRecord>, OperatorChatError> {
        self.require_exact_cloud_scope(None)?;
        Ok(self
            .coordinator
            .revoke_cloud_consent_receipt(consent_receipt_id, revoked_by_ref, reason)
            .await?)
    }

    /// Revoke cloud authority only when the HTTP extractor and durable store
    /// carry the same immutable five-dimensional resource scope.
    pub async fn revoke_single_run_cloud_consent_scoped(
        &self,
        consent_receipt_id: &str,
        revoked_by_ref: &str,
        reason: &str,
        request_scope: &ExactResourceScopeAttribution,
    ) -> Result<Vec<ModelLaneRecord>, OperatorChatError> {
        self.require_exact_cloud_scope(Some(request_scope))?;
        self.revoke_single_run_cloud_consent(consent_receipt_id, revoked_by_ref, reason)
            .await
    }

    /// Grant and consume SingleRun cloud authority through the production
    /// coordinator batch boundary. All targets preflight before any factory call.
    pub async fn launch_single_run_cloud_consent(
        &self,
        request: OperatorChatSingleRunCloudLaunchRequest,
    ) -> Result<OperatorChatSingleRunCloudLaunched, OperatorChatError> {
        self.launch_single_run_cloud_consent_for_scope(request, None)
            .await
    }

    /// Consume cloud authority only when the HTTP extractor and durable store
    /// carry the same immutable five-dimensional resource scope.
    pub async fn launch_single_run_cloud_consent_scoped(
        &self,
        request: OperatorChatSingleRunCloudLaunchRequest,
        request_scope: &ExactResourceScopeAttribution,
    ) -> Result<OperatorChatSingleRunCloudLaunched, OperatorChatError> {
        self.launch_single_run_cloud_consent_for_scope(request, Some(request_scope))
            .await
    }

    async fn launch_single_run_cloud_consent_for_scope(
        &self,
        mut request: OperatorChatSingleRunCloudLaunchRequest,
        request_scope: Option<&ExactResourceScopeAttribution>,
    ) -> Result<OperatorChatSingleRunCloudLaunched, OperatorChatError> {
        self.require_exact_cloud_scope(request_scope)?;
        if request.selections.len() < 2 {
            return Err(OperatorChatError::Invalid(
                "operator-chat SingleRun launch requires at least two selections".into(),
            ));
        }
        let mut spawn_requests = Vec::with_capacity(request.selections.len());
        let mut fan_out_targets = Vec::with_capacity(request.selections.len());
        for selection in &request.selections {
            if selection.lane_kind != OperatorChatLaneKind::Cloud {
                return Err(OperatorChatError::Invalid(
                    "operator-chat SingleRun launch accepts cloud selections only".into(),
                ));
            }
            let mut spawn = self.build_spawn_request(selection)?;
            cloud_provider_kind_for_request(&spawn)?;
            let contract = spawn.dexterity_launch.as_mut().ok_or_else(|| {
                OperatorChatError::Invalid(
                    "operator-chat SingleRun launch requires Dexterity contracts".into(),
                )
            })?;
            contract.run_id = request.grant.projection_plan.run_id.clone();
            contract.trace_id = request.grant.projection_plan.trace_id.clone();
            contract.run_span_id = format!("span-{}-run", request.grant.projection_plan.run_id);
            contract.event_ledger_stream_id =
                request.grant.projection_plan.event_ledger_stream_id.clone();
            contract.artifact_namespace = format!(
                "artifact://operator-chat/{}",
                request.grant.projection_plan.run_id
            );
            contract.memory_pack_ref = format!(
                "memory-pack://operator-chat/{}",
                request.grant.projection_plan.run_id
            );
            contract.memory_pack_hash = request.grant.projection_plan.payload_sha256.clone();
            contract.budget_summary_ref = format!(
                "budget://operator-chat/{}",
                request.grant.projection_plan.run_id
            );
            fan_out_targets.push(format!("provider-endpoint://{}", contract.adapter_id));
            spawn_requests.push(spawn);
        }
        request.grant.projection_plan.consent_scope = ModelLaneCloudConsentScope::SingleRun;
        request.grant.projection_plan.lane_id = None;
        request.grant.projection_plan.model_session_id = None;
        request.grant.projection_plan.provider_kind = None;
        request.grant.projection_plan.requested_model_id = None;
        // SingleRun is deliberately run-bound rather than lane-bound. Keep the
        // lane identity fields and target_bindings empty, while disclosing the
        // exact provider endpoints through the plan's fan-out/audience gate.
        request.grant.projection_plan.target_bindings.clear();
        request.grant.projection_plan.fan_out_targets = fan_out_targets;
        // HBR-PRIV-007: the SingleRun audience is exactly the enumerated launch
        // targets. Deriving it here (rather than trusting the caller's list)
        // means a broadcast grant cannot name an endpoint that is not one of the
        // lanes actually being launched under it.
        request
            .grant
            .projection_plan
            .export_delegation
            .audience_refs = request.grant.projection_plan.fan_out_targets.clone();
        let (plan, receipt) = self.grant_single_run_cloud_consent(request.grant).await?;
        for spawn in &mut spawn_requests {
            let contract = spawn.dexterity_launch.as_mut().ok_or_else(|| {
                OperatorChatError::Invalid(
                    "operator-chat SingleRun launch requires Dexterity contracts".into(),
                )
            })?;
            contract.projection_plan_ref = Some(plan.projection_plan_id.clone());
            contract.consent_receipt_ref = Some(receipt.consent_receipt_id.clone());
        }
        let instances = match self
            .coordinator
            .spawn_cloud_consent_batch(spawn_requests)
            .await
        {
            Ok(instances) => instances,
            Err(error) => {
                let _ = self
                    .coordinator
                    .revoke_cloud_consent_receipt(
                        &receipt.consent_receipt_id,
                        "operator-chat://single-run/failed-launch",
                        "SingleRun batch launch failed closed",
                    )
                    .await;
                return Err(error.into());
            }
        };
        Ok(OperatorChatSingleRunCloudLaunched {
            projection_plan_id: plan.projection_plan_id.clone(),
            consent_receipt_id: receipt.consent_receipt_id.clone(),
            instance_ids: instances
                .into_iter()
                .map(|instance| instance.to_string())
                .collect(),
        })
    }

    async fn routing_launches(
        &self,
        request: &OperatorChatRoutingLifecycleRequest,
    ) -> Result<Vec<ModelLaneRoutingStageLaunch>, OperatorChatError> {
        let store = self.coordinator.model_lane_store().ok_or_else(|| {
            OperatorChatError::Invalid("operator-chat routing requires a ModelLaneStore".into())
        })?;
        let replay = store.replay_run(&request.context.run_id).await?;
        self.routing_launches_from_canonical_run(request, &replay.run)
    }

    fn routing_launches_from_canonical_run(
        &self,
        request: &OperatorChatRoutingLifecycleRequest,
        run: &ModelLaneRunRecord,
    ) -> Result<Vec<ModelLaneRoutingStageLaunch>, OperatorChatError> {
        if run.trace_id != request.context.trace_id
            || run.run_span_id != request.context.run_span_id
            || run.coordinator_session_id != request.context.coordinator_session_id
            || run.work_packet_id.as_deref() != Some(request.context.work_packet_id.as_str())
            || run.micro_task_id.as_deref() != request.context.micro_task_id.as_deref()
            || run.task_board_id.as_deref() != Some(request.context.task_board_id.as_str())
            || run.owner_session != request.context.owner_session
            || run
                .locus_binding
                .as_ref()
                .map(|binding| binding.locus_binding_ref.as_str())
                != Some(request.context.locus_ref.as_str())
        {
            return Err(OperatorChatError::Invalid(
                "operator-chat routing context differs from canonical ModelLaneRun".into(),
            ));
        }
        let mut launches = Vec::with_capacity(request.stages.len());
        for stage in &request.stages {
            let spawn = if let Some(selection) = stage.selection.as_ref() {
                let lane_id = stage
                    .lane_id
                    .as_deref()
                    .filter(|value| !value.trim().is_empty())
                    .ok_or_else(|| {
                        OperatorChatError::Invalid(format!(
                            "routing model stage {} requires lane_id",
                            stage.stage_id
                        ))
                    })?;
                let mut spawn = self.build_spawn_request(selection)?;
                spawn.parent_session_id = request.context.coordinator_session_id.clone();
                spawn.owner_role = request.context.owner_session.clone();
                spawn.owner_wp = Some(request.context.work_packet_id.clone());
                spawn.wp_id = Some(request.context.work_packet_id.clone());
                spawn.mt_id = request.context.micro_task_id.clone();
                let contract = spawn.dexterity_launch.as_mut().ok_or_else(|| {
                    OperatorChatError::Invalid(format!(
                        "routing stage {} requires Dexterity launch contract",
                        stage.stage_id
                    ))
                })?;
                contract.lane_id = lane_id.to_string();
                contract.lane_span_id = format!("span-{lane_id}-lane");
                contract.run_id = run.run_id.clone();
                contract.trace_id = run.trace_id.clone();
                contract.run_span_id = run.run_span_id.clone();
                contract.routing_policy = run.routing_policy.clone();
                contract.context_bundle_id = run.context_bundle_id.clone();
                contract.event_ledger_stream_id = run.event_ledger_stream_id.clone();
                contract.artifact_namespace = run.artifact_namespace.clone();
                contract.task_board_id = run.task_board_id.clone().ok_or_else(|| {
                    OperatorChatError::Invalid(
                        "canonical ModelLaneRun requires task_board_id".into(),
                    )
                })?;
                contract.locus_binding_ref = request.context.locus_ref.clone();
                contract.projection_plan_ref = run.projection_plan_ref.clone();
                contract.consent_receipt_ref = request.authority.cloud_consent_receipt_ref.clone();
                contract.memory_pack_ref = run.memory_pack_ref.clone();
                contract.memory_pack_hash = run.memory_pack_hash.clone();
                contract.determinism_mode = run.determinism_mode.clone();
                contract.budget_summary_ref = run.budget_summary_ref.clone();
                contract.candidate_model_ids = run.candidate_model_ids.clone();
                let selected_model_id = spawn.instance_id.model_id.to_string();
                contract
                    .candidate_model_ids
                    .retain(|value| value != &selected_model_id);
                contract.candidate_model_ids.insert(0, selected_model_id);
                contract.effective_capability_snapshot_ref =
                    format!("capability-snapshot://operator-chat-routing/{lane_id}");
                contract.tool_gate_decision_refs = vec![format!(
                    "toolgate://operator-chat-routing/{lane_id}/generate"
                )];
                contract.procedural_review_status = run.procedural_review_status.clone();
                contract.truncation_warning_ref = run.truncation_warning_ref.clone();
                contract.rejection_reason_refs = run.rejection_reason_refs.clone();
                contract.run_recovery_hint_ref = run.recovery_hint_ref.clone();
                Some(spawn)
            } else {
                None
            };
            let generate_request = spawn.as_ref().map(|spawn| GenerateRequest {
                id: spawn.instance_id.model_id,
                prompt: GenPrompt::new(
                    stage
                        .selection
                        .as_ref()
                        .map(|selection| selection.prompt.clone())
                        .unwrap_or_default(),
                ),
                sampling: SamplingParams::default(),
                lora_overrides: vec![],
                steering_overrides: vec![],
                kv_prefix_handle: None,
                cancel: CancellationToken::new(),
                max_tokens: OPERATOR_CHAT_MAX_TOKENS,
                stop_sequences: vec![],
                speculative_mode: None,
                structured_decoding: None,
            });
            launches.push(ModelLaneRoutingStageLaunch {
                stage_id: stage.stage_id.clone(),
                expected_run_id: request.context.run_id.clone(),
                expected_lane_id: stage.lane_id.clone().unwrap_or_default(),
                expected_model_id: spawn
                    .as_ref()
                    .map(|spawn| spawn.instance_id.model_id.to_string())
                    .unwrap_or_default(),
                expected_provider: spawn.as_ref().and_then(|spawn| spawn.provider),
                request: spawn,
                generate_request,
                authority_lane_id: stage.authority_lane_id.clone(),
            });
        }
        Ok(launches)
    }

    pub async fn execute_routing_lifecycle(
        &self,
        request: OperatorChatRoutingLifecycleRequest,
    ) -> Result<ModelLaneRoutingDispatchBatch, OperatorChatError> {
        let launches = self.routing_launches(&request).await?;
        super::production_factory::execute_production_routing_lifecycle(
            &self.coordinator,
            &request.execution_id,
            &request.selecting_decision_id,
            &request.authority,
            request.context,
            launches,
        )
        .await
        .map_err(OperatorChatError::from)
    }

    pub async fn recover_routing_lifecycle(
        &self,
        request: OperatorChatRoutingLifecycleRequest,
    ) -> Result<ModelLaneRoutingDispatchBatch, OperatorChatError> {
        let launches = self.routing_launches(&request).await?;
        self.coordinator
            .recover_routing_execution(&request.execution_id, launches)
            .await
            .map_err(OperatorChatError::from)
    }

    #[cfg(feature = "runtime-full")]
    pub(crate) async fn complete_routing_authority(
        &self,
        bootstrap: &crate::AccountBootstrapState,
        authentication: crate::storage::surreal::resource_authority::RoleAuthorityAuthInput,
        request: OperatorChatRoutingAuthorityRequest,
    ) -> Result<ModelLaneRoutingDispatchBatch, OperatorChatError> {
        if request.execution_id != request.routing_request.execution_id {
            return Err(OperatorChatError::Invalid(
                "authority execution_id differs from routing request; no mutation performed".into(),
            ));
        }
        let store = self.coordinator.model_lane_store().ok_or_else(|| {
            OperatorChatError::Invalid(
                "authority completion requires a scoped ModelLaneStore".into(),
            )
        })?;
        let storage = &bootstrap.surreal;
        let capture = storage
            .capture_routing_authority_transaction(
                bootstrap,
                authentication,
                store,
                &request.execution_id,
                &request.stage_id,
                &request.message_id,
            )
            .await?;
        let read = storage.read_owned_routing_authority_capture(capture).await?;
        let finalization = storage.finalize_operator_routing_authority_capture(read).await?;
        let prepared = storage.prepare_owned_routing_authority_capture(finalization)?;
        let launches = match self.routing_launches_from_canonical_run(
            &request.routing_request,
            prepared.run(),
        ) {
            Ok(launches) => launches,
            Err(error) => {
                prepared.cancel().map_err(ModelLaneError::from)?;
                return Err(error);
            }
        };
        self.coordinator
            .complete_authority_and_resume_routing_lifecycle(
                prepared.into_permit(),
                &request.execution_id,
                &request.stage_id,
                &request.message_id,
                launches,
            )
            .await
            .map_err(|error| match error {
                super::coordinator::RoutingAuthorityCompletionError::ModelLane(error) =>
                    OperatorChatError::ModelLane(error),
                super::coordinator::RoutingAuthorityCompletionError::Swarm(error) =>
                    OperatorChatError::Swarm(error),
            })
    }

    pub async fn cancel_routing_lifecycle(
        &self,
        request: OperatorChatRoutingCancelRequest,
    ) -> Result<ModelLaneRoutingExecutionState, OperatorChatError> {
        self.coordinator
            .cancel_routing_execution(&request.execution_id, request.reason)
            .await
            .map_err(OperatorChatError::from)
    }

    /// Record the operator's model/worktree selection as a distinct auditable
    /// decision (spec 4.3.9.4.4). Wires the MT-014
    /// [`ModelCatalog::record_selection_decision`] primitive — this closes
    /// MT-014 MED-1. Distinct from launch.
    pub async fn record_selection(
        &self,
        selected_model_id: &str,
        actor: &str,
        reason: &str,
    ) -> Result<(), OperatorChatError> {
        let catalog = self.catalog.as_deref().ok_or_else(|| {
            OperatorChatError::Invalid(
                "operator-chat selection audit requires a ModelCatalog".to_string(),
            )
        })?;
        catalog
            .record_selection_decision(self.recorder.as_ref(), selected_model_id, actor, reason)
            .await
            .map_err(OperatorChatError::from)
    }

    /// Build the [`SpawnRequest`] for a selection (pure; unit-testable). Attaches a
    /// coordinator-generated Dexterity launch contract and plumbs the operator
    /// working_dir so a CLI lane truly runs there.
    pub fn build_spawn_request(
        &self,
        selection: &OperatorChatSelection,
    ) -> Result<SpawnRequest, OperatorChatError> {
        build_spawn_request_with_catalog(
            selection,
            self.next_instance(),
            self.catalog.as_deref(),
        )
    }

    fn next_instance(&self) -> u32 {
        self.instance_counter.fetch_add(1, Ordering::SeqCst)
    }

    async fn attach_cloud_launch_authority(
        &self,
        selection: &OperatorChatSelection,
        request: &mut SpawnRequest,
        request_scope: Option<&ExactResourceScopeAttribution>,
    ) -> Result<(), OperatorChatError> {
        if request.provider != Some(ProviderKind::ByokCloud) {
            return Ok(());
        }
        let request_scope = request_scope.ok_or_else(|| {
            OperatorChatError::Invalid(
                "RESOURCE_SCOPE_REQUIRED: operator-chat cloud launch requires the scoped product router"
                    .into(),
            )
        })?;
        self.require_exact_cloud_scope(Some(request_scope))?;
        let store = self.coordinator.model_lane_store().ok_or_else(|| {
            OperatorChatError::Invalid(
                "operator-chat cloud launch requires a ModelLaneStore for ProjectionPlan/ConsentReceipt authority"
                    .into(),
            )
        })?;
        let mut contract = request.dexterity_launch.clone().ok_or_else(|| {
            OperatorChatError::Invalid(
                "operator-chat cloud launch requires a Dexterity launch contract".into(),
            )
        })?;
        let provider_kind = cloud_provider_kind_for_request(request)?;
        let requested_model_id = cloud_requested_model_id(request, &contract);
        let model_session_id = dexterity_spawn_model_session_id(request);
        let now = Utc::now();
        let projection_plan_id = format!(
            "cloud-projection-plan://{}/{}",
            contract.run_id, contract.lane_id
        );
        let consent_receipt_id = format!(
            "cloud-consent-receipt://{}/{}",
            contract.run_id, contract.lane_id
        );
        let cloud_input_artifact_ref = cloud_input_artifact_ref(&contract.run_id);
        let cloud_payload_artifact_ref = cloud_projection_payload_artifact_ref(&contract.run_id);
        let projected_payload = build_cloud_projected_payload(
            selection,
            &contract,
            provider_kind,
            &requested_model_id,
            &model_session_id,
        );
        let payload_sha256 = sha256_hex(&canonical_json_bytes(&projected_payload));
        let scope_basis = json!({
            "run_id": &contract.run_id,
            "lane_id": &contract.lane_id,
            "model_session_id": &model_session_id,
            "provider_kind": provider_kind,
            "requested_model_id": &requested_model_id,
            "projection_plan_id": &projection_plan_id,
        });
        let scope_hash = sha256_hex(&canonical_json_bytes(&scope_basis));
        let fan_out_targets = vec![format!("provider://{provider_kind}/byok")];

        // HBR-PRIV-005/007. The approver is derived from the exact durable store
        // scope, after that scope has been matched to the HTTP extractor. It is
        // never accepted from `selection.owner_session`, which is a governance
        // role label rather than authenticated account authority.
        let approver = AccountBoundAuthority::from_access(store.access());
        let approved_by_ref = match &approver {
            AccountBoundAuthority::Account {
                owner_account_id, ..
            } => format!("account://{owner_account_id}/cloud-selection"),
            AccountBoundAuthority::Unattributed { .. } => {
                OPERATOR_CHAT_UNATTRIBUTED_APPROVAL_REF.to_string()
            }
        };
        let approver = match approver {
            account @ AccountBoundAuthority::Account { .. } => account,
            AccountBoundAuthority::Unattributed { .. } => {
                AccountBoundAuthority::unattributed(OPERATOR_CHAT_UNATTRIBUTED_APPROVAL_REASON)
            }
        };
        let stored_plan = store
            .record_cloud_projection_plan(NewModelLaneCloudProjectionPlan {
                projection_plan_id: projection_plan_id.clone(),
                run_id: contract.run_id.clone(),
                trace_id: contract.trace_id.clone(),
                lane_id: Some(contract.lane_id.clone()),
                model_session_id: Some(model_session_id.clone()),
                provider_kind: Some(provider_kind.to_string()),
                requested_model_id: Some(requested_model_id.clone()),
                scope_hash: scope_hash.clone(),
                source_artifact_refs: vec![
                    contract.context_bundle_id.clone(),
                    cloud_input_artifact_ref,
                ],
                payload_artifact_ref: cloud_payload_artifact_ref,
                payload_sha256,
                redaction_policy_ref: "redaction-policy://operator-chat/cloud-byok".to_string(),
                redaction_summary:
                    "operator-chat BYOK launch exports the selected prompt and redacted context only"
                        .to_string(),
                retention_policy: ModelLaneCloudRetentionPolicy::NoTrainingEphemeral,
                export_posture: ModelLaneCloudExportPosture::RedactedContextOnly,
                provider_profile_ref: format!("provider-profile://operator-chat/{provider_kind}"),
                fan_out_targets: fan_out_targets.clone(),
                // HBR-PRIV-007: the audience is exactly the disclosed fan-out —
                // this projection delegates to the BYOK provider endpoint and
                // nothing else — and the local visibility it is derived from is
                // the same account that approves it, so a remote export cannot
                // be broader than what that account can already see locally.
                export_delegation: CloudExportDelegation {
                    audience_refs: fan_out_targets.clone(),
                    source_scope: approver.clone(),
                    authorization_receipt_ref: Some(consent_receipt_id.clone()),
                },
                consent_scope: ModelLaneCloudConsentScope::SingleLane,
                target_bindings: vec![],
                status: ModelLaneCloudProjectionPlanStatus::Active,
                event_ledger_stream_id: contract.event_ledger_stream_id.clone(),
                work_packet_id: selection.work_packet().to_string(),
                micro_task_id: selection.micro_task().to_string(),
                task_board_id: contract.task_board_id.clone(),
                owner_session: selection.owner_session.clone(),
                idempotency_key: format!(
                    "operator-chat-cloud-projection::{}::{}",
                    contract.run_id, contract.lane_id
                ),
                created_at_utc: now.to_rfc3339(),
                user_manual_behavior_ref:
                    "usermanual://model-lane-cloud-projection-consent#launch".to_string(),
                diagnostic_payload: json!({
                    "flight_recorder": "EventLedger",
                    "internal_diagnostics": "deferred: operator-chat cloud authority is visible through ModelLane diagnostics",
                    "palmistry": "deferred: external watcher links by run_id/lane_id when available",
                    "surface": OPERATOR_CHAT_SURFACE_ID,
                    "payload": projected_payload,
                }),
            })
            .await?;
        let stored_receipt = store
            .record_cloud_consent_receipt(NewModelLaneCloudConsentReceipt {
                consent_receipt_id: consent_receipt_id.clone(),
                projection_plan_id: stored_plan.projection_plan_id.clone(),
                projection_plan_hash: stored_plan.projection_plan_hash.clone(),
                run_id: contract.run_id.clone(),
                trace_id: contract.trace_id.clone(),
                lane_id: Some(contract.lane_id.clone()),
                model_session_id: Some(model_session_id),
                provider_kind: Some(provider_kind.to_string()),
                requested_model_id: Some(requested_model_id),
                scope_hash,
                consent_scope: ModelLaneCloudConsentScope::SingleLane,
                target_bindings: vec![],
                retention_policy: ModelLaneCloudRetentionPolicy::NoTrainingEphemeral,
                export_posture: ModelLaneCloudExportPosture::RedactedContextOnly,
                fan_out_targets,
                approved: true,
                approver: approver.clone(),
                approved_by_ref,
                approved_at_utc: now.to_rfc3339(),
                valid_from_utc: (now
                    - Duration::minutes(OPERATOR_CHAT_CLOUD_CONSENT_BACKDATE_MINUTES))
                .to_rfc3339(),
                valid_until_utc: (now
                    + Duration::hours(OPERATOR_CHAT_CLOUD_CONSENT_VALIDITY_HOURS))
                .to_rfc3339(),
                revoked_at_utc: None,
                revocation_ref: None,
                revocation_input_hash: None,
                status: ModelLaneCloudConsentReceiptStatus::Approved,
                event_ledger_stream_id: contract.event_ledger_stream_id.clone(),
                work_packet_id: selection.work_packet().to_string(),
                micro_task_id: selection.micro_task().to_string(),
                task_board_id: contract.task_board_id.clone(),
                owner_session: selection.owner_session.clone(),
                idempotency_key: format!(
                    "operator-chat-cloud-consent::{}::{}",
                    contract.run_id, contract.lane_id
                ),
                created_at_utc: now.to_rfc3339(),
                user_manual_behavior_ref: "usermanual://model-lane-cloud-projection-consent#launch"
                    .to_string(),
                diagnostic_payload: json!({
                    "flight_recorder": "EventLedger",
                    "provider_call_attempted": false,
                    "surface": OPERATOR_CHAT_SURFACE_ID,
                    "selection_ref": format!("operator-chat-selection://{}", contract.run_id),
                    // Scoped diagnostic evidence (HBR-PRIV-005/008): the KIND of
                    // approval is visible without disclosing the account id, so
                    // an operator or a diagnostics surface can see at a glance
                    // that a receipt is unattributed rather than having to infer
                    // it from a string that used to look like an approval.
                    "approver_kind": if approver.is_account_bound() { "account_bound" } else { "unattributed" },
                    "consent_validity_hours": OPERATOR_CHAT_CLOUD_CONSENT_VALIDITY_HOURS,
                }),
            })
            .await?;
        contract.projection_plan_ref = Some(stored_plan.projection_plan_id.clone());
        contract.consent_receipt_ref = Some(stored_receipt.consent_receipt_id.clone());
        request.dexterity_launch = Some(contract);
        Ok(())
    }

    async fn record_cloud_projection_artifact_bindings(
        &self,
        plan: CloudProjectionArtifactBindingPlan,
    ) -> Result<(), OperatorChatError> {
        let store = self.coordinator.model_lane_store().ok_or_else(|| {
            OperatorChatError::Invalid(
                "operator-chat cloud launch requires a ModelLaneStore for ProjectionPlan artifact bindings"
                    .into(),
            )
        })?;
        let now = Utc::now().to_rfc3339();
        let input_sha256 = sha256_hex(&canonical_json_bytes(&plan.cloud_input_payload));
        let payload_sha256 = sha256_hex(&canonical_json_bytes(&plan.projected_payload));

        store
            .record_context_bundle_artifact_binding(NewModelLaneContextBundleArtifactBinding {
                artifact_binding_id: format!("artifact-binding-{}-cloud-input", plan.lane_id),
                run_id: plan.run_id.clone(),
                trace_id: plan.trace_id.clone(),
                artifact_ref: plan.cloud_input_artifact_ref.clone(),
                artifact_sha256: input_sha256.clone(),
                content_hash: input_sha256,
                artifact_kind: "operator_chat_cloud_input".to_string(),
                artifact_manifest_ref: format!(
                    "artifact-store://operator-chat/{}/cloud-input.manifest.json",
                    plan.run_id
                ),
                artifact_payload_ref: plan.cloud_input_artifact_ref.clone(),
                payload_json: plan.cloud_input_payload,
                event_ledger_stream_id: plan.event_ledger_stream_id.clone(),
                work_packet_id: plan.work_packet_id.clone(),
                micro_task_id: plan.micro_task_id.clone(),
                task_board_id: plan.task_board_id.clone(),
                owner_session: plan.owner_session.clone(),
                idempotency_key: format!(
                    "operator-chat-cloud-artifact::{}::{}::input",
                    plan.run_id, plan.lane_id
                ),
                created_at_utc: now.clone(),
                diagnostic_payload: json!({
                    "flight_recorder": "ArtifactStore/EventLedger binding for operator-chat cloud input",
                    "surface": OPERATOR_CHAT_SURFACE_ID,
                    "lane_id": &plan.lane_id,
                    "artifact_ref": &plan.cloud_input_artifact_ref,
                }),
            })
            .await?;

        store
            .record_context_bundle_artifact_binding(NewModelLaneContextBundleArtifactBinding {
                artifact_binding_id: format!(
                    "artifact-binding-{}-cloud-projection-payload",
                    plan.lane_id
                ),
                run_id: plan.run_id.clone(),
                trace_id: plan.trace_id.clone(),
                artifact_ref: plan.cloud_payload_artifact_ref.clone(),
                artifact_sha256: payload_sha256.clone(),
                content_hash: payload_sha256,
                artifact_kind: "operator_chat_cloud_projection_payload".to_string(),
                artifact_manifest_ref: format!(
                    "artifact-store://operator-chat/{}/cloud-projection-payload.manifest.json",
                    plan.run_id
                ),
                artifact_payload_ref: plan.cloud_payload_artifact_ref.clone(),
                payload_json: plan.projected_payload,
                event_ledger_stream_id: plan.event_ledger_stream_id,
                work_packet_id: plan.work_packet_id,
                micro_task_id: plan.micro_task_id,
                task_board_id: plan.task_board_id,
                owner_session: plan.owner_session,
                idempotency_key: format!(
                    "operator-chat-cloud-artifact::{}::{}::payload",
                    plan.run_id, plan.lane_id
                ),
                created_at_utc: now,
                diagnostic_payload: json!({
                    "flight_recorder": "ArtifactStore/EventLedger binding for operator-chat cloud projected payload",
                    "surface": OPERATOR_CHAT_SURFACE_ID,
                    "lane_id": &plan.lane_id,
                    "artifact_ref": &plan.cloud_payload_artifact_ref,
                }),
            })
            .await?;

        Ok(())
    }

    /// Launch a CLI/local/cloud lane for the operator selection through the
    /// sanctioned [`SwarmCoordinator::spawn_session`] authority, THEN drive the
    /// launched runtime with the operator's prompt and persist its REAL stdout as
    /// typed ModelLaneMessage rows (the F1/F2 live launch->capture loop). Fails
    /// closed if the coordinator has no `ModelLaneStore`.
    ///
    /// Assembled loop (no scaffolding): `spawn_session` persists the lane ->
    /// [`SwarmCoordinator::session_runtime`] hands back the exact runtime the
    /// coordinator just registered -> its `generate()` streams the launched CLI
    /// subprocess's live stdout -> that stdout is re-homed through
    /// [`ModelLaneCaptureRecorder::capture_cli_stream`] into ModelLaneMessage +
    /// Flight Recorder authority. The captured messages therefore ORIGINATE from
    /// the launched session's output, not a separately-authored vec.
    pub async fn launch(
        &self,
        selection: &OperatorChatSelection,
    ) -> Result<OperatorChatLaunched, OperatorChatError> {
        self.launch_for_scope(selection, None, None).await
    }

    /// Launch through an HTTP boundary whose exact resource attribution has
    /// already been extracted. Cloud selections require this path; local, CLI,
    /// human, and subagent selections remain usable through [`Self::launch`].
    pub async fn launch_scoped(
        &self,
        selection: &OperatorChatSelection,
        request_scope: &ExactResourceScopeAttribution,
    ) -> Result<OperatorChatLaunched, OperatorChatError> {
        self.launch_for_scope(selection, Some(request_scope), None).await
    }

    #[cfg(feature = "runtime-full")]
    pub(crate) async fn launch_authenticated_scoped(
        &self,
        selection: &OperatorChatSelection,
        request_scope: &ExactResourceScopeAttribution,
        bootstrap: &crate::AccountBootstrapState,
        authentication: crate::storage::surreal::resource_authority::RoleAuthorityAuthInput,
        local_execution_revisions: Option<(i64, i64)>,
    ) -> Result<OperatorChatLaunched, OperatorChatError> {
        let denied = || OperatorChatError::Invalid("authenticated scoped launch authority unavailable".into());
        let store = self.coordinator.model_lane_store().ok_or_else(denied)?;
        let lifecycle = bootstrap.surreal.resource_access_lifecycle();
        if store.surreal_storage().resource_access_lifecycle() != lifecycle
            || store.access().lifecycle_authority() != Some(&lifecycle)
            || store.access().exact_read_scope() != Some(request_scope)
        { return Err(denied()); }
        store.access().authorize_exact_request(request_scope).map_err(|_| denied())?;
        let profile_authority = profile_launch_authority(selection.engine_profile.as_ref().map(|snapshot| matches!(snapshot.profile.engine,
            handshake_dexterity::Engine::GenieXQairt | handshake_dexterity::Engine::GenieXLlamaCpp)));
        if let ProfileLaunchAuthority::HumanCreate { local_execution } = profile_authority {
            let authorization = authentication.workspace_creation_request(request_scope.workspace_id.as_str());
            let session_token = authorization.session_token.clone();
            let channel_binding_hash = authorization.channel_binding_hash.clone();
            let read_record_user = if local_execution {
                let mut read_authorization = authorization.clone();
                read_authorization.capability_id = "fs.read".into();
                read_authorization.action = crate::storage::surreal::resource_authority::ResourceAction::Read;
                let read_decision = bootstrap.surreal.authorize_protected_resource(read_authorization).await.map_err(|_| denied())?;
                if read_decision.account_id != request_scope.owner_account_id.to_string()
                    || read_decision.principal_id != request_scope.actor_principal_id.to_string()
                    || read_decision.session_id != request_scope.authenticated_session_id.to_string()
                    || read_decision.access_space_id != request_scope.access_space_id.to_string()
                    || read_decision.grant_id.is_empty() || read_decision.resource_id.is_empty()
                    || !bootstrap.capability_registry.profile_can(&read_decision.capability_profile_id, "fs.read").unwrap_or(false)
                { return Err(denied()); }
                let read_record_user = crate::storage::surreal::resource_authority::RecordUserScope {
                    grant_id: Some(read_decision.grant_id), workspace_id: Some(request_scope.workspace_id.to_string()),
                    session_token: session_token.clone(), channel_binding_hash: channel_binding_hash.clone(),
                    resource_id: read_decision.resource_id, session_id: read_decision.session_id,
                    capability_id: "fs.read".into(), action: crate::storage::surreal::resource_authority::ResourceAction::Read,
                };
                Some(read_record_user)
                } else { None };
            let decision = bootstrap.surreal.authorize_protected_resource(authorization).await.map_err(|_| denied())?;
            if decision.account_id != request_scope.owner_account_id.to_string()
                || decision.principal_id != request_scope.actor_principal_id.to_string()
                || decision.session_id != request_scope.authenticated_session_id.to_string()
                || decision.access_space_id != request_scope.access_space_id.to_string()
                || decision.grant_id.is_empty() || decision.resource_id.is_empty()
                || !bootstrap.capability_registry.profile_can(&decision.capability_profile_id, "fs.write").unwrap_or(false)
            { return Err(denied()); }
            let record_user = crate::storage::surreal::resource_authority::RecordUserScope {
                grant_id: Some(decision.grant_id), workspace_id: Some(request_scope.workspace_id.to_string()),
                session_token, channel_binding_hash, resource_id: decision.resource_id, session_id: decision.session_id,
                capability_id: "fs.write".into(), action: crate::storage::surreal::resource_authority::ResourceAction::Create,
            };
            let invocation_authority = if let Some(read_record_user) = read_record_user {
                let revisions = capture_initial_local_execution_revisions(
                    &bootstrap.surreal, &read_record_user, request_scope, local_execution_revisions,
                ).await?;
                Some(super::ids::LocalExecutionInvocationAuthority::authenticated(
                    request_scope.clone(), read_record_user, record_user.clone(),
                    revisions.expected_registry_revision, revisions.expected_selection_revision,
                    decision.capability_profile_id.clone(),
                ).map_err(|_| denied())?)
            } else { None };
            return bootstrap.surreal.with_record_user_scope(record_user.clone(), crate::storage::surreal::with_model_lane_subject(
                crate::storage::surreal::ModelLaneOperationSubject {
                    kind: crate::storage::surreal::resource_authority::ResourceKind::Workspace,
                    external_id: request_scope.workspace_id.to_string(),
                }, self.launch_for_scope(selection, Some(request_scope), invocation_authority),
            )).await;
        }
        bootstrap.surreal.authorize_validator_delegation_parent(
            &bootstrap.capability_registry, authentication, request_scope,
        ).await.map_err(|_| denied())?;
        self.launch_for_scope(selection, Some(request_scope), None).await
    }

    #[cfg(feature = "runtime-full")]
    pub(crate) async fn review_designated_validator(
        &self,
        bootstrap: &crate::AccountBootstrapState,
        auth: crate::storage::surreal::resource_authority::RoleAuthorityAuthInput,
        designation_record: &super::model_lane::WorkspaceValidatorRunnerDesignationRecord,
        operator_selector: super::model_lane::RoleEvidenceConsumeSelector,
        request: GenerateRequest,
        max_output_bytes: usize,
    ) -> super::model_lane::ModelLaneResult<super::model_lane::RoleEvidenceConsumeSelector> {
        use futures::StreamExt;
        use handshake_role_authority::domain as role;
        let denied = |error: String| ModelLaneError::AuthorityDenied(error);
        let designation = designation_record.request();
        let store = self.coordinator.model_lane_store()
            .ok_or_else(|| denied("validator review requires canonical storage".into()))?;
        if designation.run != operator_selector.expectation.run
            || designation.subject != operator_selector.expectation.subject
            || designation.expected != operator_selector.expectation.expected
            || operator_selector.designation_ref.is_some() || operator_selector.producer.is_some()
            || operator_selector.expectation.role != role::Role::Operator
            || operator_selector.expectation.purpose != role::Purpose::PriorSelection
            || store.access().exact_read_scope() != Some(&designation.scope)
            || operator_selector.expectation.receipt_scope != designation.scope
        { return Err(denied("validator review does not bind the canonical Operator prior".into())); }
        let authorization = auth.workspace_creation_request(designation.scope.workspace_id.as_str());
        let session_token = authorization.session_token.clone();
        let channel_binding_hash = authorization.channel_binding_hash.clone();
        let decision = bootstrap.surreal.authorize_protected_resource(authorization).await
            .map_err(|error| denied(error.to_string()))?;
        if decision.account_id != designation.scope.owner_account_id.to_string()
            || decision.principal_id != designation.scope.actor_principal_id.to_string()
            || decision.session_id != designation.scope.authenticated_session_id.to_string()
            || decision.access_space_id != designation.scope.access_space_id.to_string()
            || decision.resource_id.is_empty() || decision.grant_id.is_empty()
            || !bootstrap.capability_registry.profile_can(&decision.capability_profile_id, "fs.write").unwrap_or(false)
        { return Err(denied("validator review capture authority differs from Operator scope".into())); }
        let record_user = crate::storage::surreal::resource_authority::RecordUserScope {
            grant_id: Some(decision.grant_id), workspace_id: Some(designation.scope.workspace_id.to_string()),
            session_token, channel_binding_hash, resource_id: decision.resource_id, session_id: decision.session_id,
            capability_id: "fs.write".into(), action: crate::storage::surreal::resource_authority::ResourceAction::Create,
        };
        bootstrap.surreal.authorize_validator_delegation_parent(
            &bootstrap.capability_registry, auth.clone(), &designation.scope,
        ).await.map_err(|error| denied(error.to_string()))?;
        bootstrap.surreal.with_record_user_scope(record_user.clone(), crate::storage::surreal::with_model_lane_subject(
            crate::storage::surreal::ModelLaneOperationSubject {
                kind: crate::storage::surreal::resource_authority::ResourceKind::Workspace,
                external_id: designation.scope.workspace_id.to_string(),
            }, async {
        let registered = self.coordinator.registered_validator_instance_by_ref(&designation.runner.producer_instance_ref)
            .map_err(|error| denied(error.to_string()))?;
        let producer = self.coordinator.current_validator_producer(&registered)
            .map_err(|error| denied(error.to_string()))?
            .ok_or_else(|| denied("designated runtime has no authenticated producer".into()))?;
        if producer.registration_ref() != designation.runner.producer_registration_ref
            || producer.spawn_generation() != designation.runner.producer_spawn_generation
            || producer.registered_principal() != &designation.runner.producer_registered_principal
            || registered.parent_session_id() != designation.run.coordinator_session_id
        { return Err(denied("designation runtime incarnation changed".into())); }
        producer.revalidate(&bootstrap.surreal).await.map_err(|error| denied(error.to_string()))?;
        let runner_lane_id = self.coordinator.validator_producer_lane(&producer)
            .map_err(|error| denied(error.to_string()))?;
        let replay = store.replay_run(&designation.run.run_id).await?;
        let operator_lane = if let Some(lane) = replay.lanes.iter().find(|lane| lane.kind == ModelLaneKind::HumanOperator
            && lane.session_id == designation.scope.authenticated_session_id.to_string()) {
            lane.clone()
        } else {
            let mut lane = operator_lane_for_owner(&replay.run, &designation.run.owner_session)
                .map_err(|error| denied(error.to_string()))?;
            lane.lane_id = format!("operator-review-lane-{}", Uuid::now_v7());
            lane.lane_span_id = format!("span-{}", lane.lane_id);
            lane.session_id = designation.scope.authenticated_session_id.to_string();
            lane.model_session_id = lane.session_id.clone();
            if let Some(locus) = &mut lane.locus_binding {
                locus.session_id = lane.session_id.clone(); locus.model_session_id = lane.model_session_id.clone();
            }
            store.record_lane(lane).await?
        };
        let operation_id = Uuid::now_v7();
        let request_payload = json!({"text": request.prompt.as_str(), "turn_role": "operator",
            "activity_kind": "text", "validator_review_operation": operation_id.to_string()});
        let plan = CapturedMessagePlan { kind: ModelLaneMessageKind::Status,
            activity_kind: "text", summary: "Operator requested designated Validator review".into(),
            payload: request_payload.clone(), is_tool: false };
        let mut message_input = build_captured_message(&replay.run, &operator_lane, &plan, 0,
            &format!("validator-review-{operation_id}"))
            .map_err(|error| denied(error.to_string()))?;
        message_input.to_lane = ModelLaneTarget::Lane(runner_lane_id.clone());
        let payload_binding = build_message_payload_binding(&message_input, request_payload)
            .map_err(|error| denied(error.to_string()))?;
        let message = store.record_message_with_payload_binding(message_input, payload_binding).await?;
        let (mut stream, mut observation) = self.coordinator.run_registered_validator_review(
            &registered, request, designation.run.run_id.clone(),
            designation.run.coordinator_session_id.clone(), max_output_bytes,
        ).map_err(|error| denied(error.to_string()))?;
        while let Some(token) = stream.next().await {
            token.map_err(|error| denied(error.to_string()))?;
        }
        let observed = observation.take_output().map_err(|error| denied(error.to_string()))?;
        self.coordinator.with_current_validator_producer(&producer, Some(observed.terminal()), || Ok(()))
            .map_err(|error| denied(error.to_string()))?;
        let artifact_ref = format!("artifact-store://validator-review/{operation_id}/output");
        let manifest_ref = format!("artifact-store://validator-review/{operation_id}/manifest");
        let payload = Value::String(observed.output().to_owned());
        let artifact_sha = sha256_hex(&canonical_json_bytes(&payload));
        let binding = store.record_context_bundle_artifact_binding(NewModelLaneContextBundleArtifactBinding {
            artifact_binding_id: operation_id.to_string(), run_id: designation.run.run_id.clone(),
            trace_id: designation.run.trace_id.clone(), artifact_ref: artifact_ref.clone(),
            artifact_sha256: artifact_sha.clone(), content_hash: artifact_sha.clone(),
            artifact_kind: "validator_review_output".into(), artifact_manifest_ref: manifest_ref.clone(),
            artifact_payload_ref: artifact_ref.clone(), payload_json: payload,
            event_ledger_stream_id: replay.run.event_ledger_stream_id.clone(),
            work_packet_id: designation.run.work_packet_id.clone(), micro_task_id: designation.run.micro_task_id.clone(),
            task_board_id: designation.run.task_board_id.clone(), owner_session: designation.run.owner_session.clone(),
            idempotency_key: format!("validator-review-output::{operation_id}"), created_at_utc: Utc::now().to_rfc3339(),
            diagnostic_payload: json!({"invocation_trace_id": observed.terminal().trace_id().to_string()}),
        }).await?;
        let key_number = |number: i64| u64::try_from(number)
            .map_err(|_| denied("canonical review artifact event has invalid sequence".into()));
        let verdict = role::RecordValidatorVerdict {
            action: role::ActionKey { action_id: operation_id.to_string(), request_id: operation_id.to_string(),
                idempotency_key: format!("validator-review::{operation_id}") },
            scope: producer.exact_scope().clone(),
            designation_ref: role::DesignationRef::parse(&format!("eventledger://validator-designation/{}", designation_record.event().event_id))
                .map_err(|error| denied(error.to_string()))?,
            execution: role::ValidatorExecutionEvidence {
                runner_principal: producer.registered_principal().clone(), runner_lane_id,
                invocation_ref: observed.terminal().trace_id().to_string(), request_message_ref: message.message_id.clone(),
                execution_event: role::CanonicalEventKey { event_id: binding.event_ledger_event_id.clone(),
                    stream_id: binding.event_ledger_stream_id.clone(), event_seq: key_number(binding.event_ledger_seq)?,
                    transaction_seq: key_number(binding.transaction_seq)? },
                verdict_artifact: role::ArtifactSubject { artifact_ref, manifest_ref, version: binding.event_stream_version.to_string(),
                    sha256: role::ContentSha256::from_hex(&artifact_sha).map_err(|error| denied(error.to_string()))? },
            },
            run: designation.run.clone(), subject: designation.subject.clone(), purpose: role::Purpose::PriorSelection,
            expected: designation.expected.clone(), retention: designation.retention.clone(),
        };
        let issuance = bootstrap.surreal.capture_initial_validator_issuance(
            bootstrap, auth, self.coordinator.clone(), producer.clone(), observed, operator_selector.clone(), verdict, operation_id,
        ).await.map_err(|error| denied(error.to_string()))?;
        let receipt = bootstrap.surreal.consume_initial_validator_issuance(bootstrap, issuance, store).await?;
        Ok(super::model_lane::RoleEvidenceConsumeSelector {
            evidence_ref: role::RoleEvidenceRef::parse(&format!("eventledger://role-authority/{}", receipt.event.event_id))
                .map_err(|error| denied(error.to_string()))?,
            expectation: role::ResolutionExpectation { receipt_scope: receipt.body.scope.clone(), role: role::Role::Validator,
                run: receipt.body.run.clone(), subject: receipt.body.subject.clone(), purpose: receipt.body.purpose.clone(),
                expected: receipt.body.expected.clone() },
            designation_ref: receipt.body.designation.clone(), producer: Some(designation.runner.clone()),
        })
        })).await
    }

    #[cfg(feature = "runtime-full")]
    pub(crate) async fn prepare_designated_validator_runtime(
        &self,
        selection: &OperatorChatSelection,
        request_scope: &ExactResourceScopeAttribution,
        bootstrap: &crate::AccountBootstrapState,
        authentication: crate::storage::surreal::resource_authority::RoleAuthorityAuthInput,
        candidate_run: &ModelLaneRunRecord,
    ) -> Result<super::ModelInstanceId, OperatorChatError> {
        let denied = || OperatorChatError::Invalid("validator runtime preparation authority unavailable".into());
        let store = self.coordinator.model_lane_store().ok_or_else(denied)?;
        let lifecycle = bootstrap.surreal.resource_access_lifecycle();
        if selection.lane_kind == OperatorChatLaneKind::Subagent
            || store.surreal_storage().resource_access_lifecycle() != lifecycle
            || store.access().lifecycle_authority() != Some(&lifecycle)
            || store.access().exact_read_scope() != Some(request_scope)
        { return Err(denied()); }
        store.access().authorize_exact_request(request_scope).map_err(|_| denied())?;
        if candidate_run.owner_session != selection.owner_session
            || candidate_run.coordinator_session_id != selection.parent_session_id
            || candidate_run.work_packet_id.as_deref() != Some(selection.work_packet())
            || candidate_run.micro_task_id.as_deref() != Some(selection.micro_task())
        { return Err(denied()); }
        let authorization = authentication.workspace_creation_request(request_scope.workspace_id.as_str());
        let session_token = authorization.session_token.clone();
        let channel_binding_hash = authorization.channel_binding_hash.clone();
        let decision = bootstrap.surreal.authorize_protected_resource(authorization).await.map_err(|_| denied())?;
        if decision.account_id != request_scope.owner_account_id.to_string()
            || decision.principal_id != request_scope.actor_principal_id.to_string()
            || decision.session_id != request_scope.authenticated_session_id.to_string()
            || decision.access_space_id != request_scope.access_space_id.to_string()
            || decision.grant_id.is_empty() || decision.resource_id.is_empty()
            || !bootstrap.capability_registry.profile_can(&decision.capability_profile_id, "fs.write").unwrap_or(false)
        { return Err(denied()); }
        let record_user = crate::storage::surreal::resource_authority::RecordUserScope {
            grant_id: Some(decision.grant_id), workspace_id: Some(request_scope.workspace_id.to_string()),
            session_token, channel_binding_hash, resource_id: decision.resource_id, session_id: decision.session_id,
            capability_id: "fs.write".into(), action: crate::storage::surreal::resource_authority::ResourceAction::Create,
        };
        bootstrap.surreal.authorize_validator_delegation_parent(
            &bootstrap.capability_registry, authentication, request_scope,
        ).await.map_err(|_| denied())?;
        bootstrap.surreal.with_record_user_scope(record_user.clone(), crate::storage::surreal::with_model_lane_subject(
            crate::storage::surreal::ModelLaneOperationSubject {
                kind: crate::storage::surreal::resource_authority::ResourceKind::Workspace,
                external_id: request_scope.workspace_id.to_string(),
            }, async {
        let mut request = self.build_spawn_request(selection)?;
        let contract = request.dexterity_launch.as_mut().ok_or_else(denied)?;
        contract.run_id = candidate_run.run_id.clone();
        contract.trace_id = candidate_run.trace_id.clone();
        contract.run_span_id = candidate_run.run_span_id.clone();
        contract.context_bundle_id = candidate_run.context_bundle_id.clone();
        contract.event_ledger_stream_id = candidate_run.event_ledger_stream_id.clone();
        contract.artifact_namespace = candidate_run.artifact_namespace.clone();
        contract.routing_policy = candidate_run.routing_policy.clone();
        contract.task_board_id = candidate_run.task_board_id.clone().ok_or_else(denied)?;
        contract.locus_binding_ref = candidate_run.locus_binding.as_ref().ok_or_else(denied)?.locus_binding_ref.clone();
        self.attach_cloud_launch_authority(selection, &mut request, Some(request_scope)).await?;
        let plan = build_cloud_projection_artifact_binding_plan(selection, &request)?;
        let original = self.capture_original_launch(selection, &request, Some(request_scope))?;
        let instance_id = self.coordinator
            .spawn_session_with_original_operator_chat_launch(
                request, Some(candidate_run.clone()), original,
            ).await?;
        if let Some(plan) = plan {
            if let Err(error) = self.record_cloud_projection_artifact_bindings(plan).await {
                self.coordinator.cancel_session(instance_id,
                    format!("validator preparation artifact binding failed: {error}")).await?;
                return Err(error);
            }
        }
        Ok(instance_id)
        })).await
    }

    #[cfg(feature = "runtime-full")]
    pub(crate) async fn register_authenticated_validator_producer_for_designation(
        &self,
        bootstrap: &crate::AccountBootstrapState,
        auth: crate::storage::surreal::resource_authority::RoleAuthorityAuthInput,
        request: &super::model_lane::WorkspaceValidatorRunnerDesignationInput,
    ) -> Result<(
        Arc<super::coordinator::ValidatorProducerKey>,
        crate::storage::surreal::resource_authority::ValidatorAssociationIssuanceHandle,
    ), OperatorChatError> {
        let denied = || OperatorChatError::Invalid(
            "validator producer authority unavailable".into(),
        );
        let store = self.coordinator.model_lane_store().ok_or_else(denied)?;
        if store.surreal_storage().resource_access_lifecycle()
                != bootstrap.surreal.resource_access_lifecycle()
            || store.access().exact_read_scope() != Some(&request.scope)
            || store.access().lifecycle_authority()
                != Some(&bootstrap.surreal.resource_access_lifecycle())
        { return Err(denied()); }
        store.access().authorize_exact_request(&request.scope).map_err(|_| denied())?;
        let parent = bootstrap.surreal.authorize_validator_delegation_parent(
            &bootstrap.capability_registry, auth, &request.scope,
        ).await.map_err(OperatorChatError::ValidatorAssociationReadback)?;
        let registered = self.coordinator.registered_validator_instance_by_ref(
            &request.runner.instance_ref,
        )?;
        if registered.spawn_generation() != request.runner.spawn_generation
            || registered.parent_session_id() != request.run.coordinator_session_id
        { return Err(denied()); }
        let association = self.prepare_validator_association_invocation(
            request, &registered,
        ).await?;
        let input = crate::storage::surreal::resource_authority::ValidatorAssociationIssuanceInput::new(
            parent, registered.clone(), association, Arc::clone(&self.coordinator),
        );
        let mut issuance = bootstrap.surreal.reserve_validator_association_issuance(
            &bootstrap.capability_registry, input,
        ).map_err(|rejected| OperatorChatError::ValidatorAssociationReservation(Box::new(rejected)))?;
        // Fresh competing designation for an already assigned runtime cannot
        // mint another child. An exact original cached producer may reobserve.
        if let Some(existing) = self.coordinator.current_validator_producer(&registered)
            .map_err(|error| OperatorChatError::ValidatorAssociationRetained(Box::new(
                issuance.retain_runtime_failure(error),
            )))?
        {
            if !issuance.matches_cached_producer(existing.as_ref()) {
                return Err(OperatorChatError::ValidatorAssociationRetained(Box::new(
                    issuance.reject_existing_producer(),
                )));
            }
        }
        let seal = issuance.commit_and_readback(&bootstrap.surreal).await
            .map_err(|failure| OperatorChatError::ValidatorAssociationRetained(Box::new(failure)))?;
        let producer = self.coordinator.register_authenticated_validator_producer_arc(seal).await
            .map_err(|error| OperatorChatError::ValidatorAssociationRetained(Box::new(
                issuance.retain_registration_failure(error),
            )))?;
        // Final ordinary access lifecycle authorization after the last await.
        store.access().authorize_exact_request(&request.scope).map_err(|error| {
            OperatorChatError::ValidatorAssociationRetained(Box::new(
                issuance.retain_scope_failure(error),
            ))
        })?;
        self.coordinator.with_current_validator_producer(&producer, None, || Ok(()))
            .map_err(|error| OperatorChatError::ValidatorAssociationRetained(Box::new(
                issuance.retain_runtime_failure(error),
            )))?;
        Ok((producer, issuance))
    }

    #[cfg(feature = "runtime-full")]
    pub(crate) async fn designate_authenticated_validator(
        &self,
        bootstrap: &crate::AccountBootstrapState,
        authentication: crate::storage::surreal::resource_authority::RoleAuthorityAuthInput,
        input: super::model_lane::WorkspaceValidatorRunnerDesignationInput,
    ) -> Result<super::model_lane::WorkspaceValidatorRunnerDesignationRecord, OperatorChatError> {
        let denied = || OperatorChatError::Invalid(
            "explicit validator designation authority unavailable".into(),
        );
        let store = self.coordinator.model_lane_store().ok_or_else(denied)?;
        let storage = &bootstrap.surreal;
        if store.surreal_storage().resource_access_lifecycle() != storage.resource_access_lifecycle()
            || store.access().exact_read_scope() != Some(&input.scope)
            || store.access().lifecycle_authority() != Some(&storage.resource_access_lifecycle())
        { return Err(denied()); }
        store.access().authorize_exact_request(&input.scope).map_err(|_| denied())?;
        let (producer, mut issuance) = self.register_authenticated_validator_producer_for_designation(
            bootstrap, authentication, &input,
        ).await?;
        let request = self.complete_validator_runner_designation(&input, &producer)?;
        self.coordinator.with_current_validator_producer(&producer, None, || Ok(()))
            .map_err(|error| OperatorChatError::ValidatorAssociationRetained(Box::new(
                issuance.retain_runtime_failure(error),
            )))?;
        let result = store.designate_validator_runner_from_original_association(
            &mut issuance, request, std::sync::Arc::clone(&producer),
        ).await.map_err(|failure| OperatorChatError::ValidatorAssociationRetained(Box::new(failure)))?;
        producer.revalidate(storage).await.map_err(|error| {
            OperatorChatError::ValidatorAssociationRetained(Box::new(
                issuance.retain_public_authority_failure(error),
            ))
        })?;
        store.access().authorize_exact_request(&input.scope).map_err(|error| {
            OperatorChatError::ValidatorAssociationRetained(Box::new(
                issuance.retain_scope_failure(error),
            ))
        })?;
        self.coordinator.with_current_validator_producer(&producer, None, || Ok(()))
            .map_err(|error| OperatorChatError::ValidatorAssociationRetained(Box::new(
                issuance.retain_runtime_failure(error),
            )))?;
        Ok(result)
    }

    async fn launch_for_scope(
        &self,
        selection: &OperatorChatSelection,
        request_scope: Option<&ExactResourceScopeAttribution>,
        local_execution_authority: Option<super::ids::LocalExecutionInvocationAuthority>,
    ) -> Result<OperatorChatLaunched, OperatorChatError> {
        if selection.lane_kind == OperatorChatLaneKind::Subagent {
            return self.launch_subagent(selection).await;
        }
        let mut request = self.build_spawn_request(selection)?;
        request.local_execution_authority = local_execution_authority;
        self.attach_cloud_launch_authority(selection, &mut request, request_scope)
            .await?;
        let cloud_artifact_binding_plan =
            build_cloud_projection_artifact_binding_plan(selection, &request)?;
        let contract = request.dexterity_launch.clone().ok_or_else(|| {
            OperatorChatError::Invalid("missing dexterity launch contract".into())
        })?;
        #[cfg(all(feature = "test-utils", feature = "duckdb-flight-recorder"))]
        if let Some(control) = self.recorder.recorder_proof_control() {
            if let Some(scope) = request_scope.filter(|_| control.allocation_requested()) {
                control.before_session_spawn(crate::flight_recorder::shutdown_proof::AllocationPending {
                    pending_id: uuid::Uuid::now_v7(),
                    instance_id: request.instance_id.to_string(),
                    parent_session_id: request.parent_session_id.clone(),
                    scope: scope.clone(),
                    swarm_id: request.swarm_id.clone(),
                    worktree_id: request.worktree_id.clone(),
                    run_id: contract.run_id.to_string(),
                    lane_id: contract.lane_id.to_string(),
                    trace_id: contract.trace_id.to_string(),
                }).await;
            }
        }
        let original = self.capture_original_launch(selection, &request, request_scope)?;
        let instance_id = self.coordinator
            .spawn_session_with_original_operator_chat_launch(request, None, original)
            .await?;

        if let Some(plan) = cloud_artifact_binding_plan {
            if let Err(err) = self.record_cloud_projection_artifact_bindings(plan).await {
                let err_text = err.to_string();
                if let Err(cleanup_err) = self
                    .coordinator
                    .cancel_session(
                        instance_id,
                        format!("operator-chat cloud artifact binding failed: {err_text}"),
                    )
                    .await
                {
                    return Err(OperatorChatError::Invalid(format!(
                        "operator-chat cloud artifact binding failed ({err_text}); session cleanup failed: {cleanup_err}"
                    )));
                }
                return Err(err);
            }
        }

        // Drive the launched runtime and capture its real stdout as lane messages.
        let captured_result = self
            .drive_and_capture(selection, &contract.run_id, instance_id)
            .await;

        // Per-turn fresh-invocation model (contract F6): the turn is complete once
        // its stdout is captured, so complete the session to free its concurrency
        // permit + write the ledger STOP. Terminal ledger failure is part of the
        // runtime contract, so it is returned instead of hidden as a partial success.
        let captured_message_count = match captured_result {
            Ok(count) => {
                self.coordinator.complete_session(instance_id).await?;
                self.close_profile_session(selection, &contract.run_id, crate::storage::ModelSessionState::Completed, "completed").await?;
                count
            }
            Err(err) => {
                let err_text = err.to_string();
                match self
                    .coordinator
                    .cancel_session(
                        instance_id,
                        format!("operator-chat capture failed: {err_text}"),
                    )
                    .await
                {
                    Ok(()) => {},
                    Err(SwarmError::UnknownInstance(_)) if selection.engine_profile.is_none() => {},
                    Err(cleanup_err) => {
                        return Err(OperatorChatError::Invalid(format!(
                            "operator-chat capture failed ({err_text}); session cleanup failed: {cleanup_err}"
                        )));
                    }
                }
                self.close_profile_session(selection, &contract.run_id, crate::storage::ModelSessionState::Failed,
                    "ordinary profile invocation failed after owned cleanup").await?;
                return Err(err);
            }
        };

        Ok(OperatorChatLaunched {
            engine_profile: selection.engine_profile.clone(),
            instance_id: instance_id.to_string(),
            run_id: contract.run_id,
            lane_id: contract.lane_id,
            trace_id: contract.trace_id,
            lane_kind: selection.lane_kind,
            captured_message_count,
        })
    }

    async fn launch_subagent(
        &self,
        selection: &OperatorChatSelection,
    ) -> Result<OperatorChatLaunched, OperatorChatError> {
        if selection.model_id.trim().is_empty() {
            return Err(OperatorChatError::Invalid(
                "subagent lane requires a selected subagent role".into(),
            ));
        }
        if selection.working_dir.trim().is_empty() {
            return Err(OperatorChatError::Invalid(
                "operator must select a working directory / worktree before launch".into(),
            ));
        }
        if selection.prompt.trim().is_empty() {
            return Err(OperatorChatError::Invalid(
                "operator prompt is empty".into(),
            ));
        }
        let request = subagent_launch_request(selection);
        let (run, _lane, _manager) = self
            .coordinator
            .launch_operator_subagent_model_lane(request)
            .await?;
        let store = self.coordinator.model_lane_store().ok_or_else(|| {
            OperatorChatError::Invalid(
                "operator-chat subagent launch requires a ModelLaneStore".into(),
            )
        })?;
        let operator_lane = store
            .record_lane(operator_lane_for_run(&run, selection)?)
            .await?;
        record_hbr_int_009_tiers(&store, &run).await?;
        let capture = ModelLaneCaptureRecorder::new(store, self.recorder.clone());
        capture
            .record_operator_prompt(&run, &operator_lane, &selection.prompt)
            .await?;
        let run_id = run.run_id.clone();
        let lane_id = run
            .lane_ids
            .first()
            .cloned()
            .unwrap_or_else(|| "dexterity-lane-subagent-missing".to_string());
        let trace_id = run.trace_id.clone();
        Ok(OperatorChatLaunched {
            engine_profile: selection.engine_profile.clone(),
            instance_id: format!("no-os:{run_id}"),
            run_id,
            lane_id,
            trace_id,
            lane_kind: selection.lane_kind,
            captured_message_count: 0,
        })
    }

    /// Drive the just-spawned runtime with the operator prompt and persist its
    /// real stdout as ModelLaneMessage rows under the launched run. Returns the
    /// number of captured model messages. A coordinator without a `ModelLaneStore`,
    /// or a session that is not runnable, fails closed instead of returning a
    /// partial launch without capture authority.
    async fn close_profile_session(
        &self, selection: &OperatorChatSelection, run_id: &str,
        state: crate::storage::ModelSessionState, reason: &str,
    ) -> Result<(), OperatorChatError> {
        if selection.engine_profile.is_none() { return Ok(()); }
        let store = self.coordinator.model_lane_store()
            .ok_or_else(|| OperatorChatError::Invalid("profile session store unavailable".into()))?;
        let replay = store.replay_run(run_id).await?;
        let lane = replay.lanes.iter().find(|lane| lane.engine_profile == selection.engine_profile)
            .ok_or_else(|| OperatorChatError::Invalid("profile session canonical lane absent".into()))?;
        let database = crate::storage::surreal::SurrealDatabase::new(store.surreal_storage().clone());
        store.access().require_lifecycle_active()
            .map_err(|error| OperatorChatError::Invalid(error.to_string()))?;
        let authority = store.access().exact_read_scope()
            .ok_or_else(|| OperatorChatError::Invalid("profile session current authority absent".into()))?;
        crate::storage::Database::close_model_session(&database, &lane.model_session_id,
            state, reason, &format!("principal://{}", authority.actor_principal_id.as_uuid()))
            .await.map_err(|error| OperatorChatError::Invalid(error.to_string()))?;
        Ok(())
    }

    async fn drive_profile_completion(
        &self, selection: &OperatorChatSelection, run: &ModelLaneRunRecord, lane: &ModelLaneRecord,
        instance_id: ModelInstanceId, model_id: ModelId, request_id: Uuid,
        capture: &ModelLaneCaptureRecorder,
        snapshot: &crate::model_runtime::EngineProfileSnapshotV1,
    ) -> Result<usize, OperatorChatError> {
        let declaration = selection.engine_profile_session.as_ref()
            .ok_or_else(|| OperatorChatError::Invalid("profile session declaration absent".into()))?;
        declaration.validate().map_err(|error| OperatorChatError::Invalid(error.to_string()))?;
        if lane.engine_profile.as_ref() != Some(snapshot) || run.engine_profile.as_ref() != Some(snapshot)
            || lane.process_ownership_ref.as_deref().is_none_or(str::is_empty) {
            return Err(OperatorChatError::Invalid("profile session immutable runtime binding differs".into()));
        }
        let store = self.coordinator.model_lane_store()
            .ok_or_else(|| OperatorChatError::Invalid("profile session store unavailable".into()))?;
        let scope = store.access().exact_read_scope()
            .ok_or_else(|| OperatorChatError::Invalid("profile session exact scope absent".into()))?.clone();
        let database = crate::storage::surreal::SurrealDatabase::new(store.surreal_storage().clone());
        crate::storage::Database::upsert_model_session(&database, crate::storage::NewModelSession {
            engine_profile_binding: Some(crate::storage::EngineProfileSessionBindingV1 {
                declaration: declaration.clone(), run_id: run.run_id.clone(), lane_id: lane.lane_id.clone(),
                scope, run_event_id: run.event_ledger_event_id.clone(), lane_event_id: lane.event_ledger_event_id.clone(),
                snapshot: snapshot.clone(),
            }),
            session_id: lane.model_session_id.clone(), parent_session_id: Some(selection.parent_session_id.clone()),
            spawn_depth: declaration.spawn_depth, state: crate::storage::ModelSessionState::Created,
            model_id: snapshot.profile.model.id.clone(), backend: lane.backend.clone(),
            parameter_class: declaration.parameter_class.clone(), role: lane.role.clone(),
            wp_id: run.work_packet_id.clone(), mt_id: run.micro_task_id.clone(), work_profile_id: None,
            execution_mode: declaration.execution_mode.clone(), memory_policy: declaration.memory_policy.clone(),
            consent_receipt_id: None, capability_grants: vec![], capability_token_ids: None,
            job_id: None, checkpoint_artifact_id: None, last_checkpoint_at: None, checkpoint_count: 0,
            agent: Some(selection.owner_session.clone()), purpose: Some("ordinary_engine_profile_one_shot".into()),
        }).await.map_err(|error| OperatorChatError::Invalid(error.to_string()))?;
        let (client, actual_model_id, cancel, deadline) = self.coordinator.begin_profile_invocation(instance_id)?;
        if actual_model_id != model_id { return Err(OperatorChatError::Invalid("profile live model identity changed".into())); }
        crate::storage::Database::update_model_session_state(&database, &lane.model_session_id,
            crate::storage::ModelSessionState::Active, None).await
            .map_err(|error| OperatorChatError::Invalid(error.to_string()))?;
        // EPHEMERAL uses only the original prompt. This branch has no MemoryPack,
        // capsule, cached KV prefix, override or memory-write proposal path.
        let temperature = match &snapshot.profile.preset.sampling {
            handshake_dexterity::Sampling::Qairt { temperature, .. }
                | handshake_dexterity::Sampling::LlamaCpp { temperature, .. } => *temperature as f32,
        };
        let request = crate::llm::CompletionRequest::new(request_id, selection.prompt.clone(),
            actual_model_id.to_string()).with_max_tokens(snapshot.profile.preset.max_tokens)
            .with_temperature(temperature);
        let (mut future, control) = client.owned_completion(request, deadline).into_parts();
        let outcome = tokio::select! { biased;
            _ = cancel.cancelled() => { control.cancel(); Err(crate::llm::LlmError::ProviderError("profile invocation cancelled".into())) },
            _ = tokio::time::sleep_until(deadline) => { control.cancel(); Err(crate::llm::LlmError::ProviderError("profile invocation expired".into())) },
            result = &mut future => result,
        };
        drop(future);
        let drain = control.drain().await;
        if !matches!(drain, crate::llm::InvocationDrainOutcome::LocalResourcesSettled {
            remote_acknowledgement: crate::llm::RemoteAcknowledgement::NotApplicable, deadline_exceeded: false, ..
        }) { return Err(OperatorChatError::Invalid(format!("profile invocation native settlement unconfirmed: {drain:?}"))); }
        let response = outcome.map_err(|error| OperatorChatError::Invalid(error.to_string()))?;
        if cancel.is_cancelled() || tokio::time::Instant::now() >= deadline {
            return Err(OperatorChatError::Invalid("profile output arrived after its original lifetime".into()));
        }
        capture.record_activity_with_adapter(run, lane, model_id, request_id, 1,
            &AgentActivity::Text { text: response.text }, "genie_x").await?;
        Ok(1)
    }

    async fn drive_and_capture(
        &self,
        selection: &OperatorChatSelection,
        run_id: &str,
        instance_id: ModelInstanceId,
    ) -> Result<usize, OperatorChatError> {
        let store = self.coordinator.model_lane_store().ok_or_else(|| {
            OperatorChatError::Invalid("operator-chat launch requires a ModelLaneStore".into())
        })?;
        // Prepare the durable capture surfaces before driving the runtime. This
        // makes a completed activity block durable before the next stream poll,
        // so coordinator cancellation preserves that prefix and blocks only
        // genuinely late output.
        let replay = store
            .replay_run(run_id)
            .await
            .map_err(OperatorChatError::from)?;
        let run = replay.run;
        let Some(lane) = replay.lanes.into_iter().next() else {
            return Err(OperatorChatError::Invalid(format!(
                "operator-chat run {run_id} has no persisted model lane"
            )));
        };
        let operator_lane = store
            .record_lane(operator_lane_for_run(&run, selection)?)
            .await?;
        record_hbr_int_009_tiers(&store, &run).await?;
        let capture = ModelLaneCaptureRecorder::new(store, self.recorder.clone());
        capture
            .record_operator_prompt(&run, &operator_lane, &selection.prompt)
            .await?;

        let model_id = self
            .coordinator
            .session_model_id(instance_id)
            .ok_or_else(|| {
                OperatorChatError::Invalid(format!(
                    "operator-chat launch {instance_id} has no runnable session model"
                ))
            })?;
        let capture_request_id = Uuid::new_v4();
        if let Some(snapshot) = selection.engine_profile.as_ref() {
            return self.drive_profile_completion(selection, &run, &lane, instance_id,
                model_id, capture_request_id, &capture, snapshot).await;
        }
        // `generate_session` replaces this placeholder token with the
        // coordinator-owned session token before the runtime receives it.
        // Callers therefore cannot bypass the coordinator's terminal ledger
        // transition by supplying their own cancellation token.
        let request = GenerateRequest {
            id: model_id,
            prompt: GenPrompt::new(selection.prompt.clone()),
            sampling: SamplingParams::default(),
            lora_overrides: vec![],
            steering_overrides: vec![],
            kv_prefix_handle: None,
            cancel: CancellationToken::new(),
            max_tokens: OPERATOR_CHAT_MAX_TOKENS,
            stop_sequences: vec![],
            speculative_mode: None,
            structured_decoding: None,
        };
        let mut stream = self
            .coordinator
            .generate_session_managed(instance_id, request)?;
        // The coordinator-owned token this session is actually cancelled through.
        //
        // The in-band `FinishReason::Cancelled` marker is NOT a sufficient fence
        // on its own: output already buffered in the chunk channel can be
        // dequeued ahead of it, so whether a post-cancellation activity block
        // becomes durable would depend on which the executor polls first. Every
        // durable write below is therefore fenced on this token, which is the
        // same signal the runtime observes, so "only pre-cancel output is
        // durable" holds deterministically rather than by luck.
        let session_cancel = self.coordinator.session_cancel_token(instance_id);
        let cancellation_requested = || {
            session_cancel
                .as_ref()
                .is_some_and(|token| token.is_cancelled())
        };
        let mut stdout_buffer = String::new();
        let mut next_capture_index = 1;
        let mut captured_message_count = 0;
        let mut stream_error: Option<String> = None;
        let mut stream_cancelled = false;
        while let Some(item) = stream.next().await {
            match item {
                Ok(token) => {
                    if token.finish_reason == Some(FinishReason::Cancelled) {
                        stream_cancelled = true;
                    }
                    // Fence on the coordinator token, not just the in-band
                    // marker. An activity block that was already buffered when
                    // cancellation was requested arrives here AFTER the terminal
                    // boundary; persisting it would contradict the lane's
                    // Cancelled status and fabricate durable model output the
                    // operator never accepted.
                    if cancellation_requested() {
                        stream_cancelled = true;
                        stdout_buffer.clear();
                        continue;
                    }
                    if token.finish_reason.is_none() && !token.text.is_empty() {
                        stdout_buffer.push_str(&token.text);
                        while let Some(newline_index) = stdout_buffer.find('\n') {
                            let line = stdout_buffer[..newline_index]
                                .trim_end_matches('\r')
                                .to_string();
                            stdout_buffer.drain(..=newline_index);
                            let records = capture
                                .capture_cli_stream(
                                    &run,
                                    &lane,
                                    model_id,
                                    capture_request_id,
                                    cli_kind_for_selection(selection),
                                    next_capture_index,
                                    std::iter::once(line.as_str()),
                                )
                                .await?;
                            next_capture_index += records.len() as u64;
                            captured_message_count += records.len();
                        }
                    }
                }
                Err(err) => {
                    stream_error = Some(format!(
                        "operator-chat runtime stream failed for {}: {err}",
                        instance_id
                    ));
                    break;
                }
            }
        }

        // A final non-newline-terminated activity is valid stdout and must be
        // captured on normal/error completion. On a cooperative cancellation it
        // is intentionally not promoted: only newline-complete activity blocks
        // observed before the terminal boundary are durable.
        if !stream_cancelled && !cancellation_requested() && !stdout_buffer.trim().is_empty() {
            let records = capture
                .capture_cli_stream(
                    &run,
                    &lane,
                    model_id,
                    capture_request_id,
                    cli_kind_for_selection(selection),
                    next_capture_index,
                    std::iter::once(stdout_buffer.as_str()),
                )
                .await?;
            captured_message_count += records.len();
        }
        if let Some(err) = stream_error {
            return Err(OperatorChatError::Invalid(err));
        }
        if stream_cancelled {
            return Err(OperatorChatError::Invalid(format!(
                "operator-chat runtime stream cancelled for {instance_id}"
            )));
        }
        Ok(captured_message_count)
    }

    /// Project the captured ModelLaneMessage rows for a run into pane-friendly
    /// transcript rows (F8 transcript render). Reads EventLedger authority through
    /// the coordinator's `ModelLaneStore`; fails closed when no store is wired.
    pub async fn fetch_transcript(
        &self,
        run_id: &str,
    ) -> Result<Vec<OperatorChatTranscriptRow>, OperatorChatError> {
        let store = self.coordinator.model_lane_store().ok_or_else(|| {
            OperatorChatError::Invalid("operator-chat transcript requires a ModelLaneStore".into())
        })?;
        let replay = store
            .replay_run(run_id)
            .await
            .map_err(OperatorChatError::from)?;
        Ok(replay
            .messages
            .into_iter()
            .map(transcript_row_from_message)
            .collect())
    }
}

/// Project one captured message record into a transcript row: the `role` is the
/// `activity_kind` discriminator (`operator|thinking|tool_call|tool_result|text|
/// other`) and the `text` is the captured turn text (falling back to the summary).
fn transcript_row_from_message(
    msg: crate::swarm_orchestration::model_lane::ModelLaneMessageRecord,
) -> OperatorChatTranscriptRow {
    let diag = &msg.diagnostic_payload;
    let role = diag
        .get("turn_role")
        .and_then(|v| v.as_str())
        .filter(|s| *s == "operator")
        .map(|s| s.to_string())
        .or_else(|| {
            diag.get("activity_kind")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
        })
        .unwrap_or_else(|| "text".to_string());
    let capture = diag.get("capture");
    let text = capture
        .and_then(|c| c.get("text").or_else(|| c.get("raw")))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .unwrap_or_else(|| msg.summary.clone());
    let ordered_index = diag
        .get("ordered_index")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    OperatorChatTranscriptRow {
        role,
        text,
        kind: message_kind_label(&msg.kind).to_string(),
        message_id: msg.message_id.clone(),
        ordered_index,
    }
}

/// Build a [`SpawnRequest`] from an operator selection (pure). The launch resolves
/// provider/runtime from the lane kind, plumbs `working_dir`, and attaches the
/// Dexterity launch contract required by [`SwarmCoordinator::spawn_session`].
pub fn build_spawn_request(
    selection: &OperatorChatSelection,
    instance: u32,
) -> Result<SpawnRequest, OperatorChatError> {
    build_spawn_request_with_catalog(selection, instance, None)
}

fn build_spawn_request_with_catalog(
    selection: &OperatorChatSelection,
    instance: u32,
    catalog: Option<&ModelCatalog>,
) -> Result<SpawnRequest, OperatorChatError> {
    use crate::model_runtime::registry::RuntimeBinding as RuntimeAdapterBinding;

    if selection.working_dir.trim().is_empty() {
        return Err(OperatorChatError::Invalid(
            "operator must select a working directory / worktree before launch".into(),
        ));
    }
    if selection.prompt.trim().is_empty() {
        return Err(OperatorChatError::Invalid(
            "operator prompt is empty".into(),
        ));
    }

    let local_entry = if selection.lane_kind == OperatorChatLaneKind::Local {
        let catalog = catalog.ok_or_else(|| {
            OperatorChatError::Invalid(
                "local operator-chat launch requires a ModelCatalog".to_string(),
            )
        })?;
        let entry = if let Some(snapshot) = selection.engine_profile.as_ref() {
            catalog.entry_for_stable_anchor(&snapshot.profile.model.content_sha256)
        } else { catalog.entry(&selection.model_id) }.ok_or_else(|| {
            OperatorChatError::Invalid(format!(
                "local model_id '{}' is not registered in the live ModelCatalog",
                selection.model_id
            ))
        })?;
        if selection.engine_profile.is_some() && entry.runtime_binding != "genie_x" {
            return Err(OperatorChatError::Invalid("engine_profile_worker_registration_unavailable".into()));
        }
        if !entry.ready {
            return Err(OperatorChatError::Invalid(format!(
                "local model_id '{}' is registered but not ready",
                selection.model_id
            )));
        }
        if !entry.default_selectable
            || entry.runtime_role != crate::model_runtime::registry::ModelRuntimeRole::Completion
        {
            return Err(OperatorChatError::Invalid(format!(
                "local model_id '{}' has runtime role '{}' and is not eligible as the default completion model",
                selection.model_id,
                entry.runtime_role.as_str()
            )));
        }
        Some(entry)
    } else {
        None
    };
    let runtime_binding = match local_entry
        .as_ref()
        .map(|entry| entry.runtime_binding.as_str())
    {
        Some("candle") => RuntimeAdapterBinding::Candle,
        Some("llama_cpp") => RuntimeAdapterBinding::LlamaCpp,
        Some("genie_x") if selection.engine_profile.is_some() => RuntimeAdapterBinding::GenieX,
        Some(other) => {
            return Err(OperatorChatError::Invalid(format!(
                "local model_id '{}' has unsupported runtime_binding '{}'",
                selection.model_id, other
            )));
        }
        None => RuntimeAdapterBinding::Candle,
    };

    let instance_id = ModelInstanceId::new(ModelId::new_v7(), instance);
    let mut request = SpawnRequest::new(
        instance_id,
        runtime_binding,
        selection.owner_session.clone(),
        selection.parent_session_id.clone(),
    )
    .with_working_dir(selection.working_dir.clone())
    .with_wp(selection.work_packet().to_string())
    .with_mt(selection.micro_task().to_string());

    if let Some(worktree_id) = selection.worktree_id.clone() {
        request = request.with_worktree(worktree_id);
    }

    request = match selection.lane_kind {
        OperatorChatLaneKind::Local => {
            let entry = local_entry.expect("local entry resolved above");
            request.with_local_artifact(entry.artifact_path, entry.artifact_sha256)
        }
        OperatorChatLaneKind::Cli => {
            let provider = selection
                .cli_provider
                .clone()
                .unwrap_or_else(|| cli_provider_for_model(&selection.model_id).to_string());
            request
                .with_cloud_provider(ProviderKind::OfficialCli, selection.model_id.clone())
                .with_official_cli_provider(provider)
                .with_sandbox_posture(
                    crate::sandbox::TrustClass::Trusted,
                    crate::sandbox::IsolationTier::Tier1Container,
                    std::collections::BTreeSet::from([
                        crate::sandbox::RequiredCapability::HighStdioThroughput,
                    ]),
                    crate::sandbox::NetPolicy::HostInherited,
                    crate::sandbox::CLI_BRIDGE_REQUESTED_EXECUTION_POLICY_REF,
                )
        }
        OperatorChatLaneKind::Cloud => {
            let provider = match selection.cloud_provider.as_deref() {
                Some("anthropic") => ByokCloudProvider::Anthropic,
                Some("openai") => ByokCloudProvider::OpenAi,
                other => {
                    return Err(OperatorChatError::Invalid(format!(
                        "cloud lane requires cloud_provider anthropic|openai, got {other:?}"
                    )));
                }
            };
            request
                .with_cloud_provider(ProviderKind::ByokCloud, selection.model_id.clone())
                .with_byok_cloud_provider(provider)
        }
        OperatorChatLaneKind::Subagent => {
            return Err(OperatorChatError::Invalid(
                "subagent operator-chat launch uses the no-OS Dexterity path".into(),
            ));
        }
    };

    let mut contract =
        DexterityLaunchContract::from_spawn_request(&request).map_err(OperatorChatError::from)?;
    if let Some(snapshot) = selection.engine_profile.as_ref() {
        if selection.lane_kind != OperatorChatLaneKind::Local
            || snapshot.profile.model.id != selection.model_id
            || request.model_artifact_sha256.as_deref() != Some(snapshot.profile.model.content_sha256.as_str())
        { return Err(OperatorChatError::Invalid("engine profile model or artifact identity differs".into())); }
        snapshot.validate().map_err(|_| OperatorChatError::Invalid("engine profile fingerprint differs".into()))?;
        let declaration = selection.engine_profile_session.as_ref()
            .ok_or_else(|| OperatorChatError::Invalid("server-issued ordinary session policy required".into()))?;
        declaration.validate().map_err(|error| OperatorChatError::Invalid(error.to_string()))?;
        contract.engine_profile_session = Some(declaration.clone());
        contract.engine_profile = Some(snapshot.clone());
    }
    Ok(request.with_dexterity_launch(contract))
}

fn cli_provider_for_model(model_id: &str) -> &'static str {
    match cli_kind_for_model(model_id) {
        CliKind::ClaudeCode => "claude_code",
        CliKind::CodexCli => "codex",
        CliKind::GeminiCli | CliKind::Other => "codex",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_auto_issued_cloud_consent_cannot_outlive_a_working_day() {
        // Regression guard for the 365-day self-issued approval. If someone
        // widens this again, this test names the reason it was narrowed instead
        // of leaving the number to look arbitrary.
        assert!(
            OPERATOR_CHAT_CLOUD_CONSENT_VALIDITY_HOURS > 0,
            "an auto-issued consent must have a positive validity window"
        );
        assert!(
            OPERATOR_CHAT_CLOUD_CONSENT_VALIDITY_HOURS <= 24,
            "an auto-issued cloud-export approval must not survive into the next day: {OPERATOR_CHAT_CLOUD_CONSENT_VALIDITY_HOURS}h"
        );
        assert!(
            OPERATOR_CHAT_CLOUD_CONSENT_BACKDATE_MINUTES > 0
                && OPERATOR_CHAT_CLOUD_CONSENT_BACKDATE_MINUTES < 60,
            "the clock-skew grace must be a grace, not a second validity window"
        );
    }

    #[test]
    fn the_unattributed_approval_ref_does_not_claim_an_operator_approved() {
        // The replaced value was `operator://<role_label>/cloud-selection`, which
        // read as an operator approval. Its replacement must not.
        assert!(
            !OPERATOR_CHAT_UNATTRIBUTED_APPROVAL_REF.starts_with("operator://"),
            "an unattributed approval must not present itself as an operator approval"
        );
        assert!(OPERATOR_CHAT_UNATTRIBUTED_APPROVAL_REF.starts_with("unattributed://"));
    }

    #[test]
    fn tool_call_maps_to_tool_request_with_activity_kind() {
        let activity = AgentActivity::ToolCall {
            name: "Read".into(),
            input: json!({"path": "a.rs"}),
            call_id: Some("call-1".into()),
        };
        let plan = plan_from_activity(&activity);
        assert_eq!(plan.kind, ModelLaneMessageKind::ToolRequest);
        assert_eq!(plan.activity_kind, "tool_call");
        assert!(plan.is_tool);
    }

    #[test]
    fn rendered_tool_result_maps_to_tool_result() {
        let activity = AgentActivity::Text {
            text: "[tool_result] ok".into(),
        };
        let plan = plan_from_activity(&activity);
        assert_eq!(plan.kind, ModelLaneMessageKind::ToolResult);
        assert!(plan.is_tool);
    }

    #[test]
    fn thought_is_status_labelled_thinking_not_unlabelled() {
        let activity = AgentActivity::Thinking {
            text: "let me think".into(),
        };
        let plan = plan_from_activity(&activity);
        assert_eq!(plan.kind, ModelLaneMessageKind::Status);
        assert_eq!(plan.activity_kind, "thinking");
        assert!(!plan.is_tool);
    }

    #[test]
    fn model_text_is_status_labelled_text() {
        let plan = plan_from_activity(&AgentActivity::Text {
            text: "the answer is 42".into(),
        });
        assert_eq!(plan.kind, ModelLaneMessageKind::Status);
        assert_eq!(plan.activity_kind, "text");
    }

    #[test]
    fn other_is_status_labelled_other() {
        let plan = plan_from_activity(&AgentActivity::Other {
            raw: "weird".into(),
        });
        assert_eq!(plan.kind, ModelLaneMessageKind::Status);
        assert_eq!(plan.activity_kind, "other");
    }

    #[test]
    fn force_json_stream_sets_format_and_flag() {
        use crate::model_runtime::cloud::{CliBridgeConfig, CliKind, CliOutputFormat};
        use std::collections::HashMap;
        let config = CliBridgeConfig {
            cli_kind: CliKind::ClaudeCode,
            executable_path: "claude".into(),
            args_template: vec!["-p".into(), "{prompt}".into()],
            output_format: CliOutputFormat::RawText,
            env_vars: HashMap::new(),
            working_dir: None,
            timeout_seconds: 60,
        };
        let forced = force_json_stream_output(config);
        assert_eq!(forced.output_format, CliOutputFormat::JsonStream);
        assert!(forced.args_template.iter().any(|a| a == "stream-json"));
        // Idempotent: re-forcing does not duplicate the flag.
        let again = force_json_stream_output(forced);
        assert_eq!(
            again
                .args_template
                .iter()
                .filter(|a| *a == "stream-json")
                .count(),
            1
        );
    }

    #[test]
    fn build_spawn_request_cli_sets_provider_working_dir_and_contract() {
        let selection = OperatorChatSelection {
            engine_profile_session: None,
            engine_profile: None,
            lane_kind: OperatorChatLaneKind::Cli,
            model_id: "claude-sonnet-4".into(),
            cloud_provider: None,
            cli_provider: Some("claude_code".into()),
            working_dir: "D:/work/repo".into(),
            worktree_id: Some("wt-1".into()),
            prompt: "hello".into(),
            owner_session: "operator-1".into(),
            parent_session_id: "parent-1".into(),
            work_packet_id: None,
            micro_task_id: None,
        };
        let request = build_spawn_request(&selection, 7).expect("cli spawn request builds");
        assert_eq!(request.provider, Some(ProviderKind::OfficialCli));
        assert_eq!(request.working_dir.as_deref(), Some("D:/work/repo"));
        assert_eq!(request.cloud_model_name.as_deref(), Some("claude-sonnet-4"));
        assert_eq!(
            request.official_cli_provider.as_deref(),
            Some("claude_code")
        );
        assert!(request.dexterity_launch.is_some());
        assert_eq!(request.wp_id.as_deref(), Some("operator-chat-workspace"));
        assert_eq!(
            request.requested_trust_class,
            Some(crate::sandbox::TrustClass::Trusted)
        );
        assert_eq!(
            request.isolation_tier,
            Some(crate::sandbox::IsolationTier::Tier1Container)
        );
        assert_eq!(
            request.requested_net_policy,
            Some(crate::sandbox::NetPolicy::HostInherited)
        );
        // Assert against the CONSTANT, not a hand-written literal.
        //
        // This assertion previously demanded "execution-policy://operator-chat/official-cli",
        // a value that exists nowhere in src/ except this test. It was not merely stale: the
        // official CLI bridge REJECTS any requested ref that is not
        // CLI_BRIDGE_REQUESTED_EXECUTION_POLICY_REF (official_cli_bridge.rs
        // `if requested_ref != crate::sandbox::CLI_BRIDGE_REQUESTED_EXECUTION_POLICY_REF`),
        // so a spawn request carrying the old expected value would be refused by the product.
        // Satisfying the test as written would have BROKEN the operator-chat CLI lane.
        //
        // Verified pre-existing, not introduced by the 2026-08-04 lanes: the constant is
        // byte-identical at HEAD 50517aec and in the working tree, and neither this literal
        // nor the policy assignment appears in any uncommitted diff.
        //
        // Binding to the constant means a future rename moves both sides together instead of
        // silently re-breaking this test.
        assert_eq!(
            request.requested_execution_policy_ref.as_deref(),
            Some(crate::sandbox::CLI_BRIDGE_REQUESTED_EXECUTION_POLICY_REF)
        );
    }

    #[test]
    fn build_spawn_request_local_requires_catalog_resolution() {
        let selection = OperatorChatSelection {
            engine_profile_session: None,
            engine_profile: None,
            lane_kind: OperatorChatLaneKind::Local,
            model_id: Uuid::now_v7().to_string(),
            cloud_provider: None,
            cli_provider: None,
            working_dir: "D:/work/repo".into(),
            worktree_id: None,
            prompt: "hi".into(),
            owner_session: "op".into(),
            parent_session_id: "p".into(),
            work_packet_id: None,
            micro_task_id: None,
        };
        let err = build_spawn_request(&selection, 1)
            .expect_err("local build without catalog must fail closed");
        assert!(
            err.to_string().contains("requires a ModelCatalog"),
            "local model selection must not silently launch a generic local runtime: {err}"
        );
    }

    #[test]
    fn build_spawn_request_local_resolves_selected_artifact_identity() {
        let model_id = ModelId::new_v7();
        let mut registry = crate::model_runtime::ModelRegistry::default();
        registry
            .register(crate::model_runtime::ModelRegistration {
                model_id,
                artifact_path: "D:/models/local-model.gguf".into(),
                sha256: [9u8; 32],
                runtime_binding: crate::model_runtime::registry::RuntimeBinding::LlamaCpp,
                declared_capabilities: crate::model_runtime::ModelCapabilities::default(),
                base_model_tag: crate::model_runtime::BaseModelTag::new("Local Model"),
                registered_at_utc: Utc::now(),
                registered_by: crate::model_runtime::OperatorId::new("operator-test"),
                provider: ProviderKind::Local,
            })
            .expect("register local model");
        registry.mark_loaded(model_id).expect("mark model loaded");
        let catalog = ModelCatalog::from_registry(Arc::new(registry));
        let selection = OperatorChatSelection {
            engine_profile_session: None,
            engine_profile: None,
            lane_kind: OperatorChatLaneKind::Local,
            model_id: model_id.to_string(),
            cloud_provider: None,
            cli_provider: None,
            working_dir: "D:/work/repo".into(),
            worktree_id: Some("wt-local".into()),
            prompt: "hi".into(),
            owner_session: "op".into(),
            parent_session_id: "p".into(),
            work_packet_id: None,
            micro_task_id: None,
        };

        let request = build_spawn_request_with_catalog(&selection, 2, Some(catalog.as_ref()))
            .expect("local selection resolves through catalog");
        assert_eq!(request.provider, None);
        assert_eq!(
            request.runtime_binding,
            crate::model_runtime::registry::RuntimeBinding::LlamaCpp
        );
        assert_eq!(
            request.model_artifact_path.as_deref(),
            Some("D:/models/local-model.gguf")
        );
        assert_eq!(request.model_artifact_sha256, Some(hex::encode([9u8; 32])));
        assert_eq!(request.worktree_id.as_deref(), Some("wt-local"));
        assert!(request.dexterity_launch.is_some());
    }

    #[test]
    fn build_spawn_request_local_rejects_ready_embedding_role_catalog_bypass() {
        let model_id = ModelId::new_v7();
        let mut registry = crate::model_runtime::ModelRegistry::default();
        registry
            .register(crate::model_runtime::ModelRegistration {
                model_id,
                artifact_path: "D:/models/dedicated-embedding.gguf".into(),
                sha256: [10u8; 32],
                runtime_binding: crate::model_runtime::registry::RuntimeBinding::Candle,
                declared_capabilities: crate::model_runtime::ModelCapabilities {
                    supports_embedding: true,
                    embedding_dimension: Some(768),
                    ..crate::model_runtime::ModelCapabilities::default()
                },
                base_model_tag: crate::model_runtime::BaseModelTag::new(
                    "Dedicated Embedding Model",
                ),
                registered_at_utc: Utc::now(),
                registered_by: crate::model_runtime::OperatorId::new("operator-test"),
                provider: ProviderKind::Local,
            })
            .expect("register dedicated embedding model");
        registry
            .mark_loaded(model_id)
            .expect("mark embedding model READY");
        let catalog = ModelCatalog::from_registry_with_roles(
            Arc::new(registry),
            std::collections::HashMap::from([(
                model_id,
                crate::model_runtime::registry::ModelRuntimeRole::Embedding,
            )]),
        );
        let entry = catalog
            .entry(&model_id.to_string())
            .expect("READY embedding catalog entry");
        assert!(entry.ready);
        assert!(!entry.default_selectable);
        let selection = OperatorChatSelection {
            engine_profile_session: None,
            engine_profile: None,
            lane_kind: OperatorChatLaneKind::Local,
            model_id: model_id.to_string(),
            cloud_provider: None,
            cli_provider: None,
            working_dir: "D:/work/repo".into(),
            worktree_id: Some("wt-embedding-bypass".into()),
            prompt: "this direct launch must fail closed".into(),
            owner_session: "op".into(),
            parent_session_id: "p".into(),
            work_packet_id: None,
            micro_task_id: None,
        };

        let error = build_spawn_request_with_catalog(&selection, 3, Some(catalog.as_ref()))
            .expect_err("direct launch must reject a READY embedding-role catalog row");
        assert!(
            error
                .to_string()
                .contains("not eligible as the default completion model"),
            "unexpected role-boundary error: {error}"
        );
    }

    #[test]
    fn build_spawn_request_rejects_empty_working_dir() {
        let selection = OperatorChatSelection {
            engine_profile_session: None,
            engine_profile: None,
            lane_kind: OperatorChatLaneKind::Local,
            model_id: "m".into(),
            cloud_provider: None,
            cli_provider: None,
            working_dir: "  ".into(),
            worktree_id: None,
            prompt: "hi".into(),
            owner_session: "op".into(),
            parent_session_id: "p".into(),
            work_packet_id: None,
            micro_task_id: None,
        };
        assert!(matches!(
            build_spawn_request(&selection, 1),
            Err(OperatorChatError::Invalid(_))
        ));
    }
}

pub(crate) const VALIDATOR_ASSOCIATION_CAUSATION_SEMANTICS: &str =
    "designation_action_causes_association_issuance";

pub(crate) struct ValidatorAssociationInvocation {
    designation: super::model_lane::WorkspaceValidatorRunnerDesignationInput,
    source: Arc<super::coordinator::BoundOperatorChatLaunch>,
    selection_json: Value,
    committed_launch_json: Value,
    original_launch_json: Value,
    canonical_run_trace_json: Value,
    canonical_launch_plan: Option<super::routing_execution::ModelLaneRoutingExecutionState>,
}

impl ValidatorAssociationInvocation {
    pub(crate) fn causation_semantics(&self) -> &'static str {
        VALIDATOR_ASSOCIATION_CAUSATION_SEMANTICS
    }
    pub(crate) fn operation_id(&self) -> uuid::Uuid { self.designation.operation_id }
    pub(crate) fn request_id(&self) -> &str { &self.designation.action.request_id }
    pub(crate) fn idempotency_key(&self) -> &str { &self.designation.action.idempotency_key }
    pub(crate) fn causation_ref(&self) -> &str { &self.designation.action.action_id }
    pub(crate) fn designation(&self)
        -> &super::model_lane::WorkspaceValidatorRunnerDesignationInput { &self.designation }
    pub(crate) fn source(&self) -> &Arc<super::coordinator::BoundOperatorChatLaunch> { &self.source }
    pub(crate) fn selection_json(&self) -> &Value { &self.selection_json }
    pub(crate) fn committed_launch_json(&self) -> &Value { &self.committed_launch_json }
    pub(crate) fn original_launch_json(&self) -> &Value { &self.original_launch_json }
    pub(crate) fn canonical_run_trace_json(&self) -> &Value { &self.canonical_run_trace_json }
    pub(crate) fn canonical_launch_plan(&self)
        -> Option<&super::routing_execution::ModelLaneRoutingExecutionState> {
        self.canonical_launch_plan.as_ref()
    }
}

pub(crate) struct MissingCanonicalLaunchPlan {
    original_request: super::model_lane::WorkspaceValidatorRunnerDesignationInput,
    source: Arc<super::coordinator::BoundOperatorChatLaunch>,
}

impl MissingCanonicalLaunchPlan {
    pub(crate) fn into_parts(self) -> (
        super::model_lane::WorkspaceValidatorRunnerDesignationInput,
        Arc<super::coordinator::BoundOperatorChatLaunch>,
    ) {
        (self.original_request, self.source)
    }
}

impl std::fmt::Debug for MissingCanonicalLaunchPlan {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.debug_struct("MissingCanonicalLaunchPlan")
            .field("operation_id", &self.original_request.operation_id)
            .finish_non_exhaustive()
    }
}

impl OperatorChatLaunchService {
    #[cfg(feature = "runtime-full")]
    async fn prepare_validator_association_invocation(
    &self,
    request: &super::model_lane::WorkspaceValidatorRunnerDesignationInput,
    registered: &super::coordinator::RegisteredValidatorInstance,
) -> Result<ValidatorAssociationInvocation, OperatorChatError> {
    let denied = || OperatorChatError::Invalid("validator association source differs".into());
    if request.runner.instance_ref != registered.instance_id().to_string()
        || request.runner.spawn_generation != registered.spawn_generation()
        || request.run.coordinator_session_id != registered.parent_session_id()
    {
        return Err(denied());
    }
    let source = self.original_launch_for_registered_validator(
        &request.scope, registered,
    ).await?;
    let contract = source.launch_contract().ok_or_else(denied)?;
    let committed = source.committed_request();
    if request.run.run_id != contract.run_id
        || request.run.trace_id != contract.trace_id
        || request.run.coordinator_session_id != committed.parent_session_id
        || request.run.owner_session != committed.owner_role
        || committed.wp_id.as_deref() != Some(request.run.work_packet_id.as_str())
        || committed.mt_id.as_deref() != Some(request.run.micro_task_id.as_str())
        || request.run.task_board_id != contract.task_board_id
        || request.run.locus_binding_ref != contract.locus_binding_ref
    {
        return Err(denied());
    }
    let store = self.coordinator.model_lane_store().ok_or_else(denied)?;
    let replay = store.replay_run(&request.run.run_id).await?;
    if replay.run.trace_id != request.run.trace_id
        || replay.run.coordinator_session_id != request.run.coordinator_session_id
        || replay.run.owner_session != request.run.owner_session
        || replay.run.work_packet_id.as_deref() != Some(request.run.work_packet_id.as_str())
        || replay.run.micro_task_id.as_deref() != Some(request.run.micro_task_id.as_str())
        || replay.run.task_board_id.as_deref() != Some(request.run.task_board_id.as_str())
        || replay.run.event_ledger_stream_id != contract.event_ledger_stream_id
        || replay.run.locus_binding.as_ref().map(|locus| locus.locus_binding_ref.as_str())
            != Some(request.run.locus_binding_ref.as_str())
    {
        return Err(denied());
    }
    let lane = replay.lanes.iter().find(|lane| lane.lane_id == contract.lane_id)
        .ok_or_else(denied)?;
    if lane.run_id != request.run.run_id
        || lane.trace_id != request.run.trace_id
        || lane.event_ledger_stream_id != contract.event_ledger_stream_id
        || lane.restart_generation != contract.restart_generation
        || lane.model_session_id
            != super::model_lane::dexterity_spawn_model_session_id(committed)
    {
        return Err(denied());
    }
    let missing_plan = || OperatorChatError::MissingCanonicalLaunchPlan(Box::new(
        MissingCanonicalLaunchPlan {
            original_request: request.clone(),
            source: Arc::clone(&source),
        },
    ));
    // Optional actual routing provenance, never a prerequisite for a genuine
    // direct launch. Its whole original None is retained in committed JSON.
    let canonical_launch_plan = if let Some(attempt) = committed.routing_attempt.as_ref() {
        let canonical_launch_plan = store.routing_execution_snapshot(&attempt.execution_id)
            .await?.ok_or_else(missing_plan)?;
        let stage = canonical_launch_plan.stages.get(&attempt.stage_id).ok_or_else(missing_plan)?;
        let mut plans = canonical_launch_plan.canonical_launch_plan.iter()
            .filter(|plan| plan.stage_id == attempt.stage_id);
        let plan = plans.next().ok_or_else(missing_plan)?;
        if plans.next().is_some()
            || canonical_launch_plan.execution_id != attempt.execution_id
            || canonical_launch_plan.run_id != request.run.run_id
            || canonical_launch_plan.trace_id != request.run.trace_id
            || canonical_launch_plan.run_span_id != contract.run_span_id
            || canonical_launch_plan.coordinator_session_id != request.run.coordinator_session_id
            || canonical_launch_plan.locus_ref != request.run.locus_binding_ref
            || canonical_launch_plan.work_packet_id != request.run.work_packet_id
            || canonical_launch_plan.micro_task_id.as_deref() != Some(request.run.micro_task_id.as_str())
            || canonical_launch_plan.task_board_id != request.run.task_board_id
            || canonical_launch_plan.owner_session != request.run.owner_session
            || canonical_launch_plan.canonical_launch_plan_sha256 != request.run.launch_plan_hash.to_hex()
            || stage.stage_id != attempt.stage_id
            || stage.attempt != attempt.attempt
            || stage.instance_id.as_deref() != Some(registered.instance_id().to_string().as_str())
            || stage.expected_run_id != contract.run_id
            || stage.expected_lane_id != contract.lane_id
            || stage.lane_id.as_deref() != Some(contract.lane_id.as_str())
            || stage.expected_model_id != committed.instance_id.model_id.to_string()
            || stage.expected_provider != committed.provider
            || plan.dispatch_target != stage.dispatch_target
            || plan.lane_id.as_deref() != Some(contract.lane_id.as_str())
            || plan.model_id.as_deref() != Some(stage.expected_model_id.as_str())
            || plan.provider != committed.provider
        {
            return Err(denied());
        }
        Some(canonical_launch_plan)
    } else {
        None
    };
    let encode = |error: serde_json::Error| OperatorChatError::Invalid(
        format!("validator association original input encoding failed: {error}")
    );
    let committed_launch_json = source.committed_launch_json().map_err(encode)?;
    // Preserve the WHOLE actual protected replay, including messages and all
    // existing run/lane fields; the full original requested RunBinding remains
    // separately available in designation without a reconstructed projection.
    let canonical_run_trace_json = serde_json::to_value(&replay).map_err(encode)?;
    store.access().authorize_exact_request(&request.scope).map_err(|_| denied())?;
    let current = self.coordinator.original_operator_chat_launch(registered, &request.scope)?;
    if !Arc::ptr_eq(&source, &current) { return Err(denied()); }
    Ok(ValidatorAssociationInvocation {
        designation: request.clone(),
        selection_json: source.selection_json().clone(),
        original_launch_json: source.original_request_json().clone(),
        committed_launch_json,
        canonical_run_trace_json,
        canonical_launch_plan,
        source,
    })
}
    pub(crate) fn validator_runner_runtime_selection(
        &self, instance: super::ModelInstanceId,
    ) -> Result<super::model_lane::ValidatorRunnerRuntimeSelection, OperatorChatError> {
        let registered = self.coordinator.registered_validator_instance(instance)?;
        Ok(super::model_lane::ValidatorRunnerRuntimeSelection {
            instance_ref: registered.instance_id().to_string(),
            spawn_generation: registered.spawn_generation(),
        })
    }
    fn complete_validator_runner_designation(
        &self,
        input: &super::model_lane::WorkspaceValidatorRunnerDesignationInput,
        producer: &Arc<super::coordinator::ValidatorProducerKey>,
    ) -> Result<super::model_lane::WorkspaceValidatorRunnerDesignationRequest, OperatorChatError> {
        if producer.instance_id().to_string() != input.runner.instance_ref
            || producer.spawn_generation() != input.runner.spawn_generation
            || producer.delegating_scope() != &input.scope
        {
            return Err(OperatorChatError::Invalid(
                "server-issued Validator binding differs from selected runtime".into(),
            ));
        }
        Ok(super::model_lane::WorkspaceValidatorRunnerDesignationRequest {
            operation_id: input.operation_id,
            action: input.action.clone(),
            scope: input.scope.clone(),
            run: input.run.clone(),
            subject: input.subject.clone(),
            expected: input.expected.clone(),
            retention: input.retention.clone(),
            // These four facts exist ONLY after genuine successful delegated
            // child issuance/registered producer receipt. No incoming placeholder.
            runner: super::model_lane::ValidatorRunnerRegistrationBinding {
                producer_registration_ref: producer.registration_ref().to_owned(),
                producer_instance_ref: producer.instance_id().to_string(),
                producer_spawn_generation: producer.spawn_generation(),
                producer_registered_principal: producer.registered_principal().clone(),
            },
        })
    }
}

#[derive(Debug, PartialEq, Eq)]
enum ProfileLaunchAuthority {
    Delegation,
    HumanCreate { local_execution: bool },
}

fn profile_launch_authority(engine_is_genie_x: Option<bool>) -> ProfileLaunchAuthority {
    match engine_is_genie_x {
        Some(local_execution) => ProfileLaunchAuthority::HumanCreate { local_execution },
        None => ProfileLaunchAuthority::Delegation,
    }
}

#[cfg(test)]
mod profile_launch_authority_tests {
    use super::{profile_launch_authority, ProfileLaunchAuthority};

    #[test]
    fn human_profile_creation_does_not_depend_on_genie_x_supplier_requirements() {
        assert_eq!(profile_launch_authority(Some(false)),
            ProfileLaunchAuthority::HumanCreate { local_execution: false });
        assert_eq!(profile_launch_authority(Some(true)),
            ProfileLaunchAuthority::HumanCreate { local_execution: true });
        assert_eq!(profile_launch_authority(None), ProfileLaunchAuthority::Delegation);
    }
}
fn initial_local_execution_revision_request(
    workspace_id: &str,
    hints: Option<(i64,i64)>,
    records: Option<(&crate::preferences::PreferenceRecord,&crate::preferences::PreferenceRecord)>,
) -> Result<crate::preferences::LocalExecutionAuthorityRequest,OperatorChatError> {
    let denied = || OperatorChatError::Invalid("configured canonical WorkProfile revisions required".into());
    let (registry,selection) = match hints {
        Some(revisions) => revisions,
        None => {
            let (registry,selection) = records.ok_or_else(denied)?;
            for (record,key) in [(registry,crate::preferences::PREF_WORK_PROFILE_REGISTRY),
                (selection,crate::preferences::PREF_WORK_PROFILE_SELECTION)] {
                if record.preference_id != key || record.scope != "workspace" || record.scope_ref != workspace_id
                    || record.revision <= 0 || record.source != "operator" || record.value == record.default_value {
                    return Err(denied());
                }
            }
            (registry.revision,selection.revision)
        }
    };
    if workspace_id.trim().is_empty() || registry <= 0 || selection <= 0 { return Err(denied()); }
    Ok(crate::preferences::LocalExecutionAuthorityRequest {workspace_id:workspace_id.to_owned(),
        expected_registry_revision:registry,expected_selection_revision:selection})
}

#[cfg(feature = "runtime-full")]
async fn capture_initial_local_execution_revisions(
    storage: &crate::storage::surreal::SurrealStorage,
    read_scope: &crate::storage::surreal::resource_authority::RecordUserScope,
    scope: &ExactResourceScopeAttribution,
    hints: Option<(i64,i64)>,
) -> Result<crate::preferences::LocalExecutionAuthorityRequest,OperatorChatError> {
    use crate::preferences::{PreferenceScope,work_profile_preference_entry,PREF_WORK_PROFILE_REGISTRY,PREF_WORK_PROFILE_SELECTION};
    let denied = || OperatorChatError::Invalid("canonical WorkProfile revision capture rejected".into());
    let workspace = scope.workspace_id.to_string();
    let request = if hints.is_some() {
        initial_local_execution_revision_request(&workspace,hints,None)?
    } else {
        // Sequential reads select expected revisions only. The guarded snapshot below is authoritative.
        let preference_scope = PreferenceScope::workspace(&workspace);
        let registry_entry = work_profile_preference_entry(PREF_WORK_PROFILE_REGISTRY).ok_or_else(denied)?;
        let selection_entry = work_profile_preference_entry(PREF_WORK_PROFILE_SELECTION).ok_or_else(denied)?;
        let registry = storage.with_record_user_scope(read_scope.clone(),
            storage.preference_get(&preference_scope,&registry_entry)).await.map_err(|_| denied())?;
        let selection = storage.with_record_user_scope(read_scope.clone(),
            storage.preference_get(&preference_scope,&selection_entry)).await.map_err(|_| denied())?;
        initial_local_execution_revision_request(&workspace,None,Some((&registry,&selection)))?
    };
    let lease = storage.with_record_user_scope(read_scope.clone(),
        storage.local_execution_authority_snapshot(&request)).await.map_err(|_| denied())?;
    if lease.snapshot().owner_account_id != scope.owner_account_id.to_string()
        || lease.snapshot().actor_principal_id != scope.actor_principal_id.to_string() {
        return Err(denied());
    }
    // Capture once; a racing edit fails the gated expected-revision check without retry or rebase.
    drop(lease.release());
    Ok(request)
}
#[cfg(test)]
mod initial_local_execution_capture_contract {
    use super::*;
    use crate::preferences::{PreferenceRecord,PREF_WORK_PROFILE_REGISTRY,PREF_WORK_PROFILE_SELECTION};

    fn record(key: &str,revision: i64) -> PreferenceRecord {
        serde_json::from_value(json!({"schema_id":"fixture","preference_id":key,"namespace":"work-profiles",
            "value_type":"object","value":{"configured":true},"scope":"workspace","scope_ref":"fixture-workspace",
            "default_value":{},"source":"operator","revision":revision,"redaction_class":"public",
            "updated_by":"fixture-actor","event_ledger_event_id":"fixture-event"})).unwrap()
    }

    #[test]
    fn omitted_hints_capture_real_record_revisions_and_supplied_hints_never_rebase() {
        let registry = record(PREF_WORK_PROFILE_REGISTRY,7);
        let selection = record(PREF_WORK_PROFILE_SELECTION,9);
        let captured = initial_local_execution_revision_request("fixture-workspace",None,Some((&registry,&selection))).unwrap();
        assert_eq!(captured.expected_registry_revision,7);
        assert_eq!(captured.expected_selection_revision,9);
        // These stale hints remain stale for the authoritative guarded snapshot to reject.
        let hinted = initial_local_execution_revision_request("fixture-workspace",Some((1,2)),Some((&registry,&selection))).unwrap();
        assert_eq!(hinted.expected_registry_revision,1);
        assert_eq!(hinted.expected_selection_revision,2);
    }

    #[test]
    fn absent_default_and_foreign_prefilter_records_cannot_supply_revision_authority() {
        let registry = record(PREF_WORK_PROFILE_REGISTRY,7);
        let selection = record(PREF_WORK_PROFILE_SELECTION,9);
        assert!(initial_local_execution_revision_request("fixture-workspace",None,None).is_err());
        for changed in 0..4 {
            let mut invalid = selection.clone();
            match changed { 0 => invalid.revision=0,1 => invalid.source="default".into(),
                2 => invalid.scope_ref="foreign-workspace".into(),_ => invalid.value=invalid.default_value.clone() }
            assert!(initial_local_execution_revision_request("fixture-workspace",None,Some((&registry,&invalid))).is_err());
        }
    }
}