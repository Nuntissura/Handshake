use std::fmt;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

use crate::basis::CheckedJobCreation;

#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum SubmissionStateError {
    #[error("a writer is already attached")]
    WriterAlreadyAttached,
    #[error("the writer outcome is already cached")]
    WriterAlreadyCompleted,
}

#[must_use = "the rejected writer remains owned by this value"]
pub struct AttachRejected<H> {
    reason: SubmissionStateError,
    writer: H,
}

impl<H> AttachRejected<H> {
    pub fn reason(&self) -> SubmissionStateError {
        self.reason
    }

    pub fn writer(&self) -> &H {
        &self.writer
    }

    pub fn into_writer(self) -> H {
        self.writer
    }

    pub fn into_parts(self) -> (SubmissionStateError, H) {
        (self.reason, self.writer)
    }
}

impl<H> fmt::Debug for AttachRejected<H> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("AttachRejected")
            .field("reason", &self.reason)
            .finish_non_exhaustive()
    }
}

pub enum JobCreationView<'a, W> {
    Prepared {
        creation: &'a CheckedJobCreation,
        cancellation_requested: bool,
    },
    Submitted {
        creation: &'a CheckedJobCreation,
        cancellation_requested: bool,
    },
    Ready {
        creation: &'a CheckedJobCreation,
        outcome: &'a W,
        cancellation_requested: bool,
    },
}

/// Future readiness describes this writer only; it does not establish engine or resource closure.
#[must_use = "the caller owns the writer and must retain the state for its required lifetime"]
pub struct JobCreationState<H: Future<Output = W> + Unpin, W> {
    creation: CheckedJobCreation,
    writer: Option<H>,
    outcome: Option<W>,
    cancellation_requested: bool,
}

impl<H: Future<Output = W> + Unpin, W> JobCreationState<H, W> {
    pub fn new(creation: CheckedJobCreation) -> Self {
        Self {
            creation,
            writer: None,
            outcome: None,
            cancellation_requested: false,
        }
    }

    pub fn creation(&self) -> &CheckedJobCreation {
        &self.creation
    }

    pub fn attach_writer(&mut self, writer: H) -> Result<(), AttachRejected<H>> {
        let reason = if self.outcome.is_some() {
            Some(SubmissionStateError::WriterAlreadyCompleted)
        } else if self.writer.is_some() {
            Some(SubmissionStateError::WriterAlreadyAttached)
        } else {
            None
        };
        if let Some(reason) = reason {
            return Err(AttachRejected { reason, writer });
        }
        self.writer = Some(writer);
        Ok(())
    }

    pub fn request_cancel(&mut self) -> bool {
        let first_request = !self.cancellation_requested;
        self.cancellation_requested = true;
        first_request
    }

    pub fn cancellation_requested(&self) -> bool {
        self.cancellation_requested
    }

    pub fn outcome(&self) -> Option<&W> {
        self.outcome.as_ref()
    }

    pub fn view(&self) -> JobCreationView<'_, W> {
        let creation = &self.creation;
        let cancellation_requested = self.cancellation_requested;
        if let Some(outcome) = &self.outcome {
            JobCreationView::Ready {
                creation,
                outcome,
                cancellation_requested,
            }
        } else if self.writer.is_some() {
            JobCreationView::Submitted {
                creation,
                cancellation_requested,
            }
        } else {
            JobCreationView::Prepared {
                creation,
                cancellation_requested,
            }
        }
    }

    pub fn poll_writer(&mut self, context: &mut Context<'_>) -> Poll<&W> {
        if self.outcome.is_none() {
            if let Some(writer) = &mut self.writer {
                if let Poll::Ready(outcome) = Pin::new(writer).poll(context) {
                    self.outcome = Some(outcome);
                }
            }
        }
        match &self.outcome {
            Some(outcome) => Poll::Ready(outcome),
            None => Poll::Pending,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;
    use std::rc::Rc;
    use std::sync::Arc;
    use std::task::{Wake, Waker};

    use handshake_resource_scope::{
        AccessSpaceRef, ActorPrincipalId, AuthenticatedSessionRef, ExactResourceScopeAttribution,
        OwnerAccountId, ResourceScope, WorkspaceScopeRef,
    };
    use uuid::Uuid;

    use crate::basis::{CreationDescriptorInput, CreationEntityRef};
    use crate::identity::ProducerJobIdentity;

    fn creation() -> CheckedJobCreation {
        let scope = ResourceScope::new(
            OwnerAccountId::from_uuid(Uuid::now_v7()),
            ActorPrincipalId::from_uuid(Uuid::now_v7()),
        )
        .with_session(AuthenticatedSessionRef::from_uuid(Uuid::now_v7()))
        .with_access_space(AccessSpaceRef::from_uuid(Uuid::now_v7()))
        .with_workspace(WorkspaceScopeRef::new("workspace-test").unwrap());
        CheckedJobCreation::build(
            ProducerJobIdentity::allocate(true),
            ExactResourceScopeAttribution::try_from_resource_scope(&scope).unwrap(),
            CreationDescriptorInput {
                job_kind: "model_run".into(),
                protocol_id: "protocol-test".into(),
                profile_id: "profile-test".into(),
                capability_profile_id: "capability-test".into(),
                access_mode: "analysis_only".into(),
                safety_mode: "normal".into(),
                entity_refs: vec![CreationEntityRef {
                    entity_id: "workspace-test".into(),
                    entity_kind: "workspace".into(),
                }],
                planned_operations: Vec::new(),
                job_inputs: None,
            },
        )
        .unwrap()
    }

    struct NoopWake;

    impl Wake for NoopWake {
        fn wake(self: Arc<Self>) {}
    }

    struct Outcome {
        value: String,
    }

    struct ControlledWriter {
        ready: Rc<Cell<bool>>,
        polls: Rc<Cell<usize>>,
        drops: Rc<Cell<usize>>,
        outcome: Option<Outcome>,
    }

    impl Future for ControlledWriter {
        type Output = Outcome;

        fn poll(mut self: Pin<&mut Self>, _context: &mut Context<'_>) -> Poll<Self::Output> {
            self.polls.set(self.polls.get() + 1);
            assert!(self.outcome.is_some(), "writer was polled after Ready");
            if self.ready.get() {
                Poll::Ready(self.outcome.take().unwrap())
            } else {
                Poll::Pending
            }
        }
    }

    impl Drop for ControlledWriter {
        fn drop(&mut self) {
            self.drops.set(self.drops.get() + 1);
        }
    }

    fn writer(ready: Rc<Cell<bool>>, polls: Rc<Cell<usize>>, drops: Rc<Cell<usize>>) -> ControlledWriter {
        ControlledWriter {
            ready,
            polls,
            drops,
            outcome: Some(Outcome { value: "actual writer output".into() }),
        }
    }

    #[test]
    fn cancellation_keeps_pending_writer_then_caches_its_single_ready_output() {
        let ready = Rc::new(Cell::new(false));
        let polls = Rc::new(Cell::new(0));
        let drops = Rc::new(Cell::new(0));
        let checked = creation();
        let identity = checked.identity().clone();
        let mut state = JobCreationState::new(checked);
        assert!(matches!(state.view(), JobCreationView::Prepared { .. }));
        state.attach_writer(writer(ready.clone(), polls.clone(), drops.clone())).unwrap();
        let waker = Waker::from(Arc::new(NoopWake));
        let mut context = Context::from_waker(&waker);
        assert!(state.poll_writer(&mut context).is_pending());
        assert!(state.request_cancel());
        assert!(!state.request_cancel());
        assert!(matches!(state.view(), JobCreationView::Submitted { cancellation_requested: true, .. }));
        assert!(state.outcome().is_none());
        assert!(state.poll_writer(&mut context).is_pending());
        assert_eq!(drops.get(), 0);
        ready.set(true);
        let Poll::Ready(observed) = state.poll_writer(&mut context) else { panic!("actual writer should now be ready") };
        assert_eq!(observed.value, "actual writer output");
        let observed_address = observed as *const Outcome;
        assert_eq!(polls.get(), 3);
        for _ in 0..3 {
            let Poll::Ready(cached) = state.poll_writer(&mut context) else { panic!("cached writer output lost") };
            assert_eq!(cached as *const Outcome, observed_address);
        }
        assert_eq!(polls.get(), 3);
        assert_eq!(drops.get(), 0);
        assert_eq!(state.creation().identity(), &identity);
        assert!(matches!(state.view(), JobCreationView::Ready { cancellation_requested: true, .. }));
        drop(state);
        assert_eq!(drops.get(), 1);
    }

    #[test]
    fn duplicate_attachment_returns_the_rejected_writer_without_dropping_either() {
        let ready = Rc::new(Cell::new(false));
        let first_drops = Rc::new(Cell::new(0));
        let rejected_drops = Rc::new(Cell::new(0));
        let mut state = JobCreationState::new(creation());
        state.attach_writer(writer(ready.clone(), Rc::new(Cell::new(0)), first_drops.clone())).unwrap();
        let rejected = state.attach_writer(writer(ready, Rc::new(Cell::new(0)), rejected_drops.clone())).unwrap_err();
        assert_eq!(rejected.reason(), SubmissionStateError::WriterAlreadyAttached);
        assert_eq!(first_drops.get(), 0);
        assert_eq!(rejected_drops.get(), 0);
        assert!(Rc::ptr_eq(&rejected.writer().drops, &rejected_drops));
        let (reason, actual_writer) = rejected.into_parts();
        assert_eq!(reason, SubmissionStateError::WriterAlreadyAttached);
        assert_eq!(rejected_drops.get(), 0);
        drop(actual_writer);
        assert_eq!(rejected_drops.get(), 1);
        assert_eq!(first_drops.get(), 0);
        drop(state);
        assert_eq!(first_drops.get(), 1);
    }

    #[test]
    fn terminal_writer_error_is_cached_and_replacement_writer_is_returned() {
        let mut state = JobCreationState::new(creation());
        assert!(state.request_cancel());
        let waker = Waker::from(Arc::new(NoopWake));
        let mut context = Context::from_waker(&waker);
        assert!(state.poll_writer(&mut context).is_pending());
        state.attach_writer(std::future::ready(Err::<(), _>("writer failed"))).unwrap();
        assert_eq!(state.poll_writer(&mut context), Poll::Ready(&Err("writer failed")));
        let rejected = state.attach_writer(std::future::ready(Ok(()))).unwrap_err();
        assert_eq!(rejected.reason(), SubmissionStateError::WriterAlreadyCompleted);
        let mut actual_rejected_writer = rejected.into_writer();
        assert_eq!(Pin::new(&mut actual_rejected_writer).poll(&mut context), Poll::Ready(Ok(())));
        assert_eq!(state.poll_writer(&mut context), Poll::Ready(&Err("writer failed")));
        assert!(state.cancellation_requested());
    }
}
