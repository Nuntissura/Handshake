use std::any::Any;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::pin::Pin;
use std::task::{Context, Poll};

use futures_util::future::poll_fn;
use tokio::time::{timeout_at, Instant};

pub trait ExecutionDriver<R> {
    type Output;
    type Error;

    fn poll_advance(
        self: Pin<&mut Self>,
        resources: Pin<&mut R>,
        cx: &mut Context<'_>,
    ) -> Poll<Result<Self::Output, Self::Error>>;

    fn request_cancel(self: Pin<&mut Self>, resources: Pin<&mut R>);
}

pub enum ExecutionObservation<'a, O, E> {
    Pending,
    Returned(&'a Result<O, E>),
    PanickedUnknown,
}

#[derive(Debug, PartialEq, Eq)]
pub enum ExecutionAdvanceError {
    Deadline,
    PanickedUnknown,
}

/// Owns execution resources independently of any host finalizer.
/// A returned driver result does not establish host finalization.
pub struct OwnedExecution<R, D: ExecutionDriver<R>> {
    resources: Pin<Box<R>>,
    driver: Pin<Box<D>>,
    original_deadline: Instant,
    returned: Option<Result<D::Output, D::Error>>,
    cancel_requested: bool,
    panic_payload: Option<Box<dyn Any + Send + 'static>>,
}

impl<R, D: ExecutionDriver<R>> OwnedExecution<R, D> {
    pub fn new(resources: R, driver: D, original_deadline: Instant) -> Self {
        Self {
            resources: Box::pin(resources),
            driver: Box::pin(driver),
            original_deadline,
            returned: None,
            cancel_requested: false,
            panic_payload: None,
        }
    }

    pub fn original_deadline(&self) -> Instant {
        self.original_deadline
    }

    pub fn request_cancel(&mut self) -> Result<(), ExecutionAdvanceError> {
        if self.panic_payload.is_some() {
            return Err(ExecutionAdvanceError::PanickedUnknown);
        }
        if self.returned.is_some() || self.cancel_requested {
            return Ok(());
        }

        self.cancel_requested = true;
        match catch_unwind(AssertUnwindSafe(|| {
            self.driver
                .as_mut()
                .request_cancel(self.resources.as_mut());
        })) {
            Ok(()) => Ok(()),
            Err(payload) => {
                self.panic_payload = Some(payload);
                Err(ExecutionAdvanceError::PanickedUnknown)
            }
        }
    }

    pub async fn advance_until(
        &mut self,
    ) -> Result<ExecutionObservation<'_, D::Output, D::Error>, ExecutionAdvanceError> {
        if self.panic_payload.is_some() {
            return Err(ExecutionAdvanceError::PanickedUnknown);
        }
        if self.returned.is_some() {
            return Ok(self.observation());
        }

        let deadline = self.original_deadline;
        let progress = poll_fn(|cx| {
            match catch_unwind(AssertUnwindSafe(|| {
                self.driver
                    .as_mut()
                    .poll_advance(self.resources.as_mut(), cx)
            })) {
                Ok(Poll::Pending) => Poll::Pending,
                Ok(Poll::Ready(result)) => {
                    self.returned = Some(result);
                    Poll::Ready(Ok(()))
                }
                Err(payload) => {
                    self.panic_payload = Some(payload);
                    Poll::Ready(Err(ExecutionAdvanceError::PanickedUnknown))
                }
            }
        });

        match timeout_at(deadline, progress).await {
            Ok(result) => result?,
            Err(_) => return Err(ExecutionAdvanceError::Deadline),
        }

        Ok(self.observation())
    }

    pub fn observation(&self) -> ExecutionObservation<'_, D::Output, D::Error> {
        if self.panic_payload.is_some() {
            ExecutionObservation::PanickedUnknown
        } else if let Some(result) = self.returned.as_ref() {
            ExecutionObservation::Returned(result)
        } else {
            ExecutionObservation::Pending
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::{Cell, RefCell};
    use std::future::Future;
    use std::marker::PhantomPinned;
    use std::panic::panic_any;
    use std::rc::Rc;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;
    use std::time::Duration;

    use futures_util::task::noop_waker_ref;
    use tokio::runtime::{Builder, Runtime};

    #[derive(Clone, Copy)]
    enum Mode {
        Pending,
        Success,
        Error,
        PollPanic,
        CancelPanic,
    }

    struct DropSentinel(Rc<Cell<usize>>);

    impl Drop for DropSentinel {
        fn drop(&mut self) {
            self.0.set(self.0.get() + 1);
        }
    }

    struct Resources {
        _drop: DropSentinel,
        _pinned: PhantomPinned,
    }

    struct Output {
        value: usize,
        _drop: DropSentinel,
    }

    struct PanicPayload(Arc<AtomicUsize>);

    impl Drop for PanicPayload {
        fn drop(&mut self) {
            self.0.fetch_add(1, Ordering::SeqCst);
        }
    }

    struct State {
        mode: Cell<Mode>,
        polls: Cell<usize>,
        cancels: Cell<usize>,
        addresses: RefCell<Vec<(usize, usize)>>,
        resource_drops: Rc<Cell<usize>>,
        driver_drops: Rc<Cell<usize>>,
        output_drops: Rc<Cell<usize>>,
        panic_drops: Arc<AtomicUsize>,
    }

    impl State {
        fn new(mode: Mode) -> Rc<Self> {
            Rc::new(Self {
                mode: Cell::new(mode),
                polls: Cell::new(0),
                cancels: Cell::new(0),
                addresses: RefCell::new(Vec::new()),
                resource_drops: Rc::new(Cell::new(0)),
                driver_drops: Rc::new(Cell::new(0)),
                output_drops: Rc::new(Cell::new(0)),
                panic_drops: Arc::new(AtomicUsize::new(0)),
            })
        }

        fn assert_retained(&self) {
            assert_eq!(self.resource_drops.get(), 0);
            assert_eq!(self.driver_drops.get(), 0);
            assert_eq!(self.panic_drops.load(Ordering::SeqCst), 0);
        }

        fn assert_same_addresses(&self) {
            let addresses = self.addresses.borrow();
            assert!(!addresses.is_empty());
            assert!(addresses.iter().all(|address| *address == addresses[0]));
        }
    }

    struct Driver {
        state: Rc<State>,
        _drop: DropSentinel,
        _pinned: PhantomPinned,
    }

    impl ExecutionDriver<Resources> for Driver {
        type Output = Output;
        type Error = &'static str;

        fn poll_advance(
            self: Pin<&mut Self>,
            resources: Pin<&mut Resources>,
            _cx: &mut Context<'_>,
        ) -> Poll<Result<Self::Output, Self::Error>> {
            let this = self.as_ref().get_ref();
            let state = &this.state;
            state.addresses.borrow_mut().push((
                this as *const Driver as usize,
                resources.as_ref().get_ref() as *const Resources as usize,
            ));
            state.polls.set(state.polls.get() + 1);
            match state.mode.get() {
                Mode::Pending | Mode::CancelPanic => Poll::Pending,
                Mode::Success => Poll::Ready(Ok(Output {
                    value: 41,
                    _drop: DropSentinel(state.output_drops.clone()),
                })),
                Mode::Error => Poll::Ready(Err("driver error")),
                Mode::PollPanic => {
                    panic_any(PanicPayload(state.panic_drops.clone()));
                }
            }
        }

        fn request_cancel(self: Pin<&mut Self>, resources: Pin<&mut Resources>) {
            let this = self.as_ref().get_ref();
            let state = &this.state;
            state.addresses.borrow_mut().push((
                this as *const Driver as usize,
                resources.as_ref().get_ref() as *const Resources as usize,
            ));
            state.cancels.set(state.cancels.get() + 1);
            if matches!(state.mode.get(), Mode::CancelPanic) {
                panic_any(PanicPayload(state.panic_drops.clone()));
            }
        }
    }

    fn runtime() -> Runtime {
        Builder::new_current_thread().enable_time().build().unwrap()
    }

    fn execution(state: &Rc<State>, deadline: Instant) -> OwnedExecution<Resources, Driver> {
        OwnedExecution::new(
            Resources {
                _drop: DropSentinel(state.resource_drops.clone()),
                _pinned: PhantomPinned,
            },
            Driver {
                state: state.clone(),
                _drop: DropSentinel(state.driver_drops.clone()),
                _pinned: PhantomPinned,
            },
            deadline,
        )
    }

    #[test]
    fn dropped_borrow_and_cancel_preserve_pinned_holders_and_separate_finalizer() {
        let runtime = runtime();
        runtime.block_on(async {
            let state = State::new(Mode::Pending);
            let deadline = Instant::now() + Duration::from_secs(60);
            let mut owner = execution(&state, deadline);
            let finalizer_drops = Rc::new(Cell::new(0));
            let finalizer = DropSentinel(finalizer_drops.clone());

            {
                let mut advance = Box::pin(owner.advance_until());
                let mut cx = Context::from_waker(noop_waker_ref());
                assert!(advance.as_mut().poll(&mut cx).is_pending());
            }

            state.assert_retained();
            assert_eq!(owner.original_deadline(), deadline);
            assert!(matches!(owner.observation(), ExecutionObservation::Pending));
            owner.request_cancel().unwrap();
            owner.request_cancel().unwrap();
            assert_eq!(state.cancels.get(), 1);
            state.assert_retained();
            assert_eq!(owner.original_deadline(), deadline);

            state.mode.set(Mode::Success);
            assert!(matches!(
                owner.advance_until().await,
                Ok(ExecutionObservation::Returned(Ok(Output { value: 41, .. })))
            ));
            state.assert_same_addresses();
            state.assert_retained();
            assert_eq!(finalizer_drops.get(), 0);

            drop(owner);
            assert_eq!(state.resource_drops.get(), 1);
            assert_eq!(state.driver_drops.get(), 1);
            assert_eq!(state.output_drops.get(), 1);
            assert_eq!(finalizer_drops.get(), 0);
            drop(finalizer);
            assert_eq!(finalizer_drops.get(), 1);
        });
    }

    #[test]
    fn expiry_preserves_original_deadline_and_actual_holders() {
        let runtime = runtime();
        runtime.block_on(async {
            let state = State::new(Mode::Pending);
            let deadline = Instant::now() + Duration::from_millis(5);
            let mut owner = execution(&state, deadline);

            assert!(matches!(
                owner.advance_until().await,
                Err(ExecutionAdvanceError::Deadline)
            ));
            assert_eq!(owner.original_deadline(), deadline);
            assert!(matches!(owner.observation(), ExecutionObservation::Pending));
            state.assert_retained();

            owner.request_cancel().unwrap();
            assert!(matches!(
                owner.advance_until().await,
                Err(ExecutionAdvanceError::Deadline)
            ));
            assert_eq!(owner.original_deadline(), deadline);
            state.assert_retained();
            state.assert_same_addresses();

            drop(owner);
            assert_eq!(state.resource_drops.get(), 1);
            assert_eq!(state.driver_drops.get(), 1);
        });
    }

    #[test]
    fn returned_success_and_error_are_cached_without_driver_reentry() {
        let runtime = runtime();
        runtime.block_on(async {
            for mode in [Mode::Success, Mode::Error] {
                let state = State::new(mode);
                let deadline = Instant::now() + Duration::from_secs(60);
                let mut owner = execution(&state, deadline);

                let first = match owner.advance_until().await {
                    Ok(ExecutionObservation::Returned(result)) => {
                        match mode {
                            Mode::Success => {
                                assert!(matches!(result, Ok(Output { value: 41, .. })));
                            }
                            Mode::Error => assert!(matches!(result, Err("driver error"))),
                            _ => unreachable!(),
                        }
                        result as *const Result<Output, &'static str>
                    }
                    _ => panic!("expected a cached driver result"),
                };

                state.mode.set(Mode::PollPanic);
                owner.request_cancel().unwrap();
                let second = match owner.advance_until().await {
                    Ok(ExecutionObservation::Returned(result)) => {
                        result as *const Result<Output, &'static str>
                    }
                    _ => panic!("expected the same cached result"),
                };

                assert_eq!(first, second);
                assert_eq!(state.polls.get(), 1);
                assert_eq!(state.cancels.get(), 0);
                assert_eq!(owner.original_deadline(), deadline);
                state.assert_retained();
                assert_eq!(state.output_drops.get(), 0);

                drop(owner);
                assert_eq!(state.resource_drops.get(), 1);
                assert_eq!(state.driver_drops.get(), 1);
                assert_eq!(
                    state.output_drops.get(),
                    usize::from(matches!(mode, Mode::Success))
                );
            }
        });
    }

    #[test]
    fn poll_panic_keeps_original_payload_and_holders_without_reentry() {
        let runtime = runtime();
        runtime.block_on(async {
            let state = State::new(Mode::PollPanic);
            let deadline = Instant::now() + Duration::from_secs(60);
            let mut owner = execution(&state, deadline);

            assert!(matches!(
                owner.advance_until().await,
                Err(ExecutionAdvanceError::PanickedUnknown)
            ));
            assert!(matches!(
                owner.observation(),
                ExecutionObservation::PanickedUnknown
            ));
            let payload = owner
                .panic_payload
                .as_ref()
                .unwrap()
                .downcast_ref::<PanicPayload>()
                .unwrap();
            assert!(Arc::ptr_eq(&payload.0, &state.panic_drops));
            state.assert_retained();

            state.mode.set(Mode::Success);
            assert!(matches!(
                owner.advance_until().await,
                Err(ExecutionAdvanceError::PanickedUnknown)
            ));
            assert_eq!(
                owner.request_cancel(),
                Err(ExecutionAdvanceError::PanickedUnknown)
            );
            assert_eq!(state.polls.get(), 1);
            assert_eq!(state.cancels.get(), 0);
            assert_eq!(owner.original_deadline(), deadline);
            state.assert_retained();

            drop(owner);
            assert_eq!(state.resource_drops.get(), 1);
            assert_eq!(state.driver_drops.get(), 1);
            assert_eq!(state.panic_drops.load(Ordering::SeqCst), 1);
        });
    }

    #[test]
    fn cancel_panic_keeps_original_payload_and_holders_without_reentry() {
        let runtime = runtime();
        runtime.block_on(async {
            let state = State::new(Mode::CancelPanic);
            let deadline = Instant::now() + Duration::from_secs(60);
            let mut owner = execution(&state, deadline);

            {
                let mut advance = Box::pin(owner.advance_until());
                let mut cx = Context::from_waker(noop_waker_ref());
                assert!(advance.as_mut().poll(&mut cx).is_pending());
            }
            assert_eq!(
                owner.request_cancel(),
                Err(ExecutionAdvanceError::PanickedUnknown)
            );
            assert!(matches!(
                owner.observation(),
                ExecutionObservation::PanickedUnknown
            ));
            let payload = owner
                .panic_payload
                .as_ref()
                .unwrap()
                .downcast_ref::<PanicPayload>()
                .unwrap();
            assert!(Arc::ptr_eq(&payload.0, &state.panic_drops));
            state.assert_retained();

            state.mode.set(Mode::Success);
            assert!(matches!(
                owner.advance_until().await,
                Err(ExecutionAdvanceError::PanickedUnknown)
            ));
            assert_eq!(
                owner.request_cancel(),
                Err(ExecutionAdvanceError::PanickedUnknown)
            );
            assert_eq!(state.polls.get(), 1);
            assert_eq!(state.cancels.get(), 1);
            assert_eq!(owner.original_deadline(), deadline);
            state.assert_same_addresses();
            state.assert_retained();

            drop(owner);
            assert_eq!(state.resource_drops.get(), 1);
            assert_eq!(state.driver_drops.get(), 1);
            assert_eq!(state.panic_drops.load(Ordering::SeqCst), 1);
        });
    }
}
