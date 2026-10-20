use std::sync::{Arc, Mutex, MutexGuard, Weak};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DispatchDisposition {
    Unmanaged,
    Owned,
    Terminal,
    Taken,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntryError {
    OwnerMismatch,
    AlreadyOwned,
    NotTerminal,
    Pinned,
    Taken,
    Unknown,
}

pub enum EntryPayloadRef<'a, P, T> {
    Present { payload: &'a P, observation: Option<&'a T> },
    Taken,
}

pub struct EntryUpdateRejected<T> {
    error: EntryError,
    incoming: T,
}

impl<T> EntryUpdateRejected<T> {
    pub fn into_parts(self) -> (EntryError, T) {
        (self.error, self.incoming)
    }
}

struct EntryInner<P, T> {
    payload: Option<P>,
    observation: Option<T>,
    disposition: DispatchDisposition,
    owner: Option<Weak<()>>,
}

impl<P, T> EntryInner<P, T> {
    fn refresh_owner(&mut self) -> Result<(), EntryError> {
        if matches!(self.disposition, DispatchDisposition::Owned | DispatchDisposition::Terminal)
            && self.owner.as_ref().and_then(Weak::upgrade).is_none()
        {
            self.disposition = DispatchDisposition::Unknown;
        }
        if self.disposition == DispatchDisposition::Unknown {
            Err(EntryError::Unknown)
        } else {
            Ok(())
        }
    }

    fn require_owner(&mut self, owner: &Arc<()>) -> Result<(), EntryError> {
        self.refresh_owner()?;
        if self.disposition == DispatchDisposition::Taken {
            return Err(EntryError::Taken);
        }
        match self.owner.as_ref().and_then(Weak::upgrade) {
            Some(current) if Arc::ptr_eq(&current, owner) => Ok(()),
            _ => Err(EntryError::OwnerMismatch),
        }
    }
}

pub struct Entry<P, T> {
    inner: Arc<Mutex<EntryInner<P, T>>>,
}

pub struct EntryPin<P, T> {
    inner: Arc<Mutex<EntryInner<P, T>>>,
}

pub struct EntryPayloadGuard<'a, P, T> {
    inner: MutexGuard<'a, EntryInner<P, T>>,
}

fn lock_payload<P, T>(
    inner: &Mutex<EntryInner<P, T>>,
) -> Result<EntryPayloadGuard<'_, P, T>, EntryError> {
    let mut inner = inner.lock().map_err(|_| EntryError::Unknown)?;
    inner.refresh_owner()?;
    Ok(EntryPayloadGuard { inner })
}

impl<P, T> Entry<P, T> {
    pub fn new(payload: P) -> Self {
        Self {
            inner: Arc::new(Mutex::new(EntryInner {
                payload: Some(payload),
                observation: None,
                disposition: DispatchDisposition::Unmanaged,
                owner: None,
            })),
        }
    }

    pub fn claim_existing(&self, owner: &Arc<()>) -> Result<EntryPin<P, T>, EntryError> {
        let mut inner = self.inner.lock().map_err(|_| EntryError::Unknown)?;
        inner.refresh_owner()?;
        match inner.disposition {
            DispatchDisposition::Unmanaged => {
                inner.owner = Some(Arc::downgrade(owner));
                inner.disposition = DispatchDisposition::Owned;
                Ok(EntryPin { inner: Arc::clone(&self.inner) })
            }
            DispatchDisposition::Taken => Err(EntryError::Taken),
            _ => Err(EntryError::AlreadyOwned),
        }
    }

    pub fn pin(&self, owner: &Arc<()>) -> Result<EntryPin<P, T>, EntryError> {
        let mut inner = self.inner.lock().map_err(|_| EntryError::Unknown)?;
        inner.require_owner(owner)?;
        Ok(EntryPin { inner: Arc::clone(&self.inner) })
    }

    pub fn payload(&self) -> Result<EntryPayloadGuard<'_, P, T>, EntryError> {
        lock_payload(&self.inner)
    }

    pub fn observe(
        &self,
        owner: &Arc<()>,
        incoming: T,
        terminal: bool,
    ) -> Result<Option<T>, EntryUpdateRejected<T>> {
        let mut inner = match self.inner.lock() {
            Ok(inner) => inner,
            Err(_) => return Err(EntryUpdateRejected { error: EntryError::Unknown, incoming }),
        };
        if let Err(error) = inner.require_owner(owner) {
            return Err(EntryUpdateRejected { error, incoming });
        }
        if inner.disposition == DispatchDisposition::Terminal {
            return Err(EntryUpdateRejected { error: EntryError::AlreadyOwned, incoming });
        }
        let previous = inner.observation.replace(incoming);
        if terminal {
            inner.disposition = DispatchDisposition::Terminal;
        }
        Ok(previous)
    }

    pub fn take_terminal(&mut self, owner: &Arc<()>) -> Result<(P, Option<T>), EntryError> {
        let mut inner = self.inner.lock().map_err(|_| EntryError::Unknown)?;
        inner.require_owner(owner)?;
        if inner.disposition != DispatchDisposition::Terminal {
            return Err(EntryError::NotTerminal);
        }
        if Arc::strong_count(&self.inner) != 1 {
            return Err(EntryError::Pinned);
        }
        let payload = match inner.payload.take() {
            Some(payload) => payload,
            None => {
                inner.disposition = DispatchDisposition::Unknown;
                return Err(EntryError::Unknown);
            }
        };
        let observation = inner.observation.take();
        inner.disposition = DispatchDisposition::Taken;
        Ok((payload, observation))
    }
}

impl<P, T> EntryPin<P, T> {
    pub fn payload(&self) -> Result<EntryPayloadGuard<'_, P, T>, EntryError> {
        lock_payload(&self.inner)
    }
}

impl<P, T> EntryPayloadGuard<'_, P, T> {
    /// References cannot outlive the borrow of this guard.
    ///
    /// ```compile_fail
    /// use handshake_workflow_lifecycle::{Entry, EntryPayloadRef};
    /// let entry = Entry::<String, ()>::new(String::from("payload"));
    /// let guard = entry.payload().unwrap();
    /// let payload = match guard.view() {
    ///     EntryPayloadRef::Present { payload, .. } => payload,
    ///     EntryPayloadRef::Taken => unreachable!(),
    /// };
    /// drop(guard);
    /// assert_eq!(payload, "payload");
    /// ```
    pub fn view(&self) -> EntryPayloadRef<'_, P, T> {
        match self.inner.payload.as_ref() {
            Some(payload) => EntryPayloadRef::Present {
                payload,
                observation: self.inner.observation.as_ref(),
            },
            None => EntryPayloadRef::Taken,
        }
    }

    pub fn dispatch_disposition(&self) -> DispatchDisposition {
        if matches!(self.inner.disposition, DispatchDisposition::Owned | DispatchDisposition::Terminal)
            && self.inner.owner.as_ref().and_then(Weak::upgrade).is_none()
        {
            DispatchDisposition::Unknown
        } else {
            self.inner.disposition
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;
    use std::panic::{catch_unwind, AssertUnwindSafe};
    use std::rc::Rc;

    struct Held {
        id: usize,
        drops: Rc<Cell<usize>>,
    }

    impl Drop for Held {
        fn drop(&mut self) {
            self.drops.set(self.drops.get() + 1);
        }
    }

    fn held(id: usize, drops: &Rc<Cell<usize>>) -> Held {
        Held { id, drops: Rc::clone(drops) }
    }

    #[test]
    fn pin_keeps_actual_payload_and_observation_after_entry_drop() {
        let payload_drops = Rc::new(Cell::new(0));
        let observation_drops = Rc::new(Cell::new(0));
        let owner = Arc::new(());
        let entry = Entry::new(held(1, &payload_drops));
        let pin = entry.claim_existing(&owner).unwrap();
        assert!(entry.observe(&owner, held(2, &observation_drops), false).is_ok());
        drop(entry);
        assert_eq!(payload_drops.get(), 0);
        assert_eq!(observation_drops.get(), 0);
        {
            let guard = pin.payload().unwrap();
            match guard.view() {
                EntryPayloadRef::Present { payload, observation: Some(observation) } => {
                    assert_eq!(payload.id, 1);
                    assert_eq!(observation.id, 2);
                }
                _ => panic!("actual resources missing"),
            }
        }
        drop(pin);
        assert_eq!(payload_drops.get(), 1);
        assert_eq!(observation_drops.get(), 1);
    }

    #[test]
    fn claim_preserves_identity_and_denies_repeat_wrong_and_expired_owners() {
        let owner = Arc::new(());
        let wrong = Arc::new(());
        let entry = Entry::<String, ()>::new(String::from("original"));
        let before = {
            let guard = entry.payload().unwrap();
            assert_eq!(guard.dispatch_disposition(), DispatchDisposition::Unmanaged);
            match guard.view() {
                EntryPayloadRef::Present { payload, .. } => payload as *const String,
                _ => panic!("payload missing"),
            }
        };
        let pin = entry.claim_existing(&owner).unwrap();
        {
            let guard = pin.payload().unwrap();
            assert_eq!(guard.dispatch_disposition(), DispatchDisposition::Owned);
            match guard.view() {
                EntryPayloadRef::Present { payload, .. } => assert_eq!(before, payload as *const String),
                _ => panic!("payload missing"),
            }
        }
        assert!(matches!(entry.claim_existing(&owner), Err(EntryError::AlreadyOwned)));
        assert!(matches!(entry.pin(&wrong), Err(EntryError::OwnerMismatch)));
        drop(owner);
        assert!(matches!(entry.pin(&wrong), Err(EntryError::Unknown)));
        assert!(matches!(entry.claim_existing(&wrong), Err(EntryError::Unknown)));
        assert!(matches!(pin.payload(), Err(EntryError::Unknown)));
    }

    #[test]
    fn observation_returns_whole_prior_and_rejects_whole_incoming() {
        let drops = Rc::new(Cell::new(0));
        let owner = Arc::new(());
        let wrong = Arc::new(());
        let entry = Entry::new(());
        let pin = entry.claim_existing(&owner).unwrap();
        assert!(matches!(entry.observe(&owner, held(1, &drops), false), Ok(None)));
        let prior = match entry.observe(&owner, held(2, &drops), true) {
            Ok(Some(prior)) => prior,
            _ => panic!("whole previous observation not returned"),
        };
        assert_eq!(prior.id, 1);
        assert_eq!(drops.get(), 0);
        let rejection = match entry.observe(&wrong, held(3, &drops), false) {
            Err(rejection) => rejection,
            _ => panic!("wrong owner accepted"),
        };
        let (error, incoming) = rejection.into_parts();
        assert_eq!(error, EntryError::OwnerMismatch);
        assert_eq!(incoming.id, 3);
        let terminal_rejection = match entry.observe(&owner, held(4, &drops), false) {
            Err(rejection) => rejection,
            _ => panic!("terminal observation overwritten"),
        };
        let (terminal_error, terminal_incoming) = terminal_rejection.into_parts();
        assert_eq!(terminal_error, EntryError::AlreadyOwned);
        assert_eq!(terminal_incoming.id, 4);
        assert_eq!(drops.get(), 0);
        {
            let guard = pin.payload().unwrap();
            assert_eq!(guard.dispatch_disposition(), DispatchDisposition::Terminal);
            match guard.view() {
                EntryPayloadRef::Present { observation: Some(current), .. } => assert_eq!(current.id, 2),
                _ => panic!("terminal observation lost"),
            }
        }
        drop((prior, incoming, terminal_incoming, pin, entry));
        assert_eq!(drops.get(), 4);
    }

    #[test]
    fn terminal_take_requires_owner_and_zero_pins_and_happens_once() {
        let drops = Rc::new(Cell::new(0));
        let owner = Arc::new(());
        let wrong = Arc::new(());
        let mut entry = Entry::new(held(1, &drops));
        let first = entry.claim_existing(&owner).unwrap();
        assert!(matches!(entry.take_terminal(&owner), Err(EntryError::NotTerminal)));
        assert!(entry.observe(&owner, held(2, &drops), true).is_ok());
        let second = entry.pin(&owner).unwrap();
        assert!(matches!(entry.take_terminal(&wrong), Err(EntryError::OwnerMismatch)));
        assert!(matches!(entry.take_terminal(&owner), Err(EntryError::Pinned)));
        drop(first);
        assert!(matches!(entry.take_terminal(&owner), Err(EntryError::Pinned)));
        drop(second);
        let (payload, observation) = entry.take_terminal(&owner).unwrap();
        assert_eq!(payload.id, 1);
        assert_eq!(observation.as_ref().unwrap().id, 2);
        assert_eq!(drops.get(), 0);
        assert!(matches!(entry.take_terminal(&owner), Err(EntryError::Taken)));
        assert!(matches!(entry.pin(&owner), Err(EntryError::Taken)));
        let guard = entry.payload().unwrap();
        assert!(matches!(guard.view(), EntryPayloadRef::Taken));
        assert_eq!(guard.dispatch_disposition(), DispatchDisposition::Taken);
        drop(guard);
        drop(entry);
        assert_eq!(drops.get(), 0);
        drop((payload, observation));
        assert_eq!(drops.get(), 2);
    }

    #[test]
    fn poisoned_mutex_retains_custody_and_rejects_whole_incoming() {
        let drops = Rc::new(Cell::new(0));
        let owner = Arc::new(());
        let mut entry = Entry::new(held(1, &drops));
        let pin = entry.claim_existing(&owner).unwrap();
        assert!(entry.observe(&owner, held(2, &drops), true).is_ok());
        let unwind = catch_unwind(AssertUnwindSafe(|| {
            let _guard = entry.inner.lock().unwrap();
            panic!("poison while preserving resources");
        }));
        assert!(unwind.is_err());
        assert!(matches!(entry.payload(), Err(EntryError::Unknown)));
        assert!(matches!(pin.payload(), Err(EntryError::Unknown)));
        assert!(matches!(entry.pin(&owner), Err(EntryError::Unknown)));
        assert!(matches!(entry.take_terminal(&owner), Err(EntryError::Unknown)));
        let rejected = match entry.observe(&owner, held(3, &drops), false) {
            Err(rejected) => rejected,
            _ => panic!("poisoned entry accepted update"),
        };
        let (error, incoming) = rejected.into_parts();
        assert_eq!(error, EntryError::Unknown);
        assert_eq!(incoming.id, 3);
        assert_eq!(drops.get(), 0);
        drop(entry);
        assert_eq!(drops.get(), 0);
        drop(pin);
        assert_eq!(drops.get(), 2);
        drop(incoming);
        assert_eq!(drops.get(), 3);
    }
}
