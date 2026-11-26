//! Diagnostics that never reach the host's sink while a lock of the owner is held (MT-203 E3-2).
//!
//! Every event of the owner, its ledger and its handles goes through [`Outbox`]. An event that is emitted while the
//! emitting thread holds the owner's slot or close lock ([`Held`]), or a generation's lane ([`Quiet`]), waits in the
//! outbox until that thread let go of its last such lock, and is then delivered by that thread, or by another one that
//! is delivering at that moment: a sink may be called on any thread that emits events. Every other event is delivered
//! at once. Events are delivered in the order they were emitted, one at a time: a sink that calls back into the owner,
//! also one that emits more events that way, never meets a lock of the owner held by its own thread, and what it causes
//! is delivered after it returns.
//!
//! The windows' targets of destroyed surfaces take the same way ([`Outbox::bury`]): a target is dropped by a thread that
//! holds no lock of the owner, the one that destroyed its surface once it let go of its last one or another one that
//! delivers first, so whatever the window crate does in the drop never runs under one.

use crate::diag::{OwnerDiagnostics, OwnerEvent};
use crate::vulkan::Targets;
use std::cell::Cell;
use std::collections::VecDeque;
use std::ops::{Deref, DerefMut};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError, TryLockError};

thread_local! {
    /// Locks of the owner this thread holds now.
    static HELD: Cell<u32> = const { Cell::new(0) };
}

fn held() -> u32 {
    HELD.with(Cell::get)
}

/// The host's sink, behind the queue of what was emitted and not yet delivered.
pub(crate) struct Outbox {
    sink: Arc<dyn OwnerDiagnostics>,
    queue: Mutex<VecDeque<OwnerEvent>>,
    /// Taken by the one thread that delivers now. Never waited for.
    turn: Mutex<()>,
    /// Targets to drop. Held for a push or a take only.
    graves: Mutex<Targets>,
}

impl Outbox {
    pub(crate) fn new(sink: Arc<dyn OwnerDiagnostics>) -> Self {
        Self {
            sink,
            queue: Mutex::new(VecDeque::new()),
            turn: Mutex::new(()),
            graves: Mutex::new(Targets::new()),
        }
    }

    fn lock_queue(&self) -> MutexGuard<'_, VecDeque<OwnerEvent>> {
        self.queue.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn lock_graves(&self) -> MutexGuard<'_, Targets> {
        self.graves.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Drop `targets` as soon as this thread holds no lock of the owner: now, or when it lets go of its last one.
    pub(crate) fn bury(&self, targets: Targets) {
        if targets.is_empty() {
            return;
        }
        self.lock_graves().extend(targets);
        self.deliver();
    }

    /// Deliver what waits, unless this thread holds a lock of the owner or another delivery runs: that one, or the
    /// release of the last lock, delivers it then. Targets that wait are dropped first, with no lock held.
    pub(crate) fn deliver(&self) {
        if held() == 0 {
            let graves = std::mem::take(&mut *self.lock_graves());
            drop(graves);
        }
        while held() == 0 {
            let turn = match self.turn.try_lock() {
                Ok(turn) => turn,
                Err(TryLockError::Poisoned(poisoned)) => poisoned.into_inner(),
                // Another thread, or this one further up in a sink that called back.
                Err(TryLockError::WouldBlock) => return,
            };
            loop {
                // The queue lock is released before the sink is called.
                let next = self.lock_queue().pop_front();
                match next {
                    Some(event) => self.sink.record(event),
                    None => break,
                }
            }
            drop(turn);
            // An event that came while this thread was leaving would otherwise wait for the next one.
            if self.lock_queue().is_empty() {
                return;
            }
        }
    }

    /// Events that wait now.
    #[cfg(test)]
    pub(crate) fn waiting(&self) -> usize {
        self.lock_queue().len()
    }
}

impl OwnerDiagnostics for Outbox {
    fn record(&self, event: OwnerEvent) {
        self.lock_queue().push_back(event);
        self.deliver();
    }
}

/// A section in which this thread holds a lock that is not a [`Held`] one (a generation's lane): events wait as under
/// a held lock.
pub(crate) struct Quiet<'a> {
    outbox: &'a Outbox,
}

impl<'a> Quiet<'a> {
    pub(crate) fn begin(outbox: &'a Outbox) -> Self {
        HELD.with(|count| count.set(count.get().saturating_add(1)));
        Self { outbox }
    }
}

impl Drop for Quiet<'_> {
    fn drop(&mut self) {
        HELD.with(|count| count.set(count.get().saturating_sub(1)));
        self.outbox.deliver();
    }
}

/// A lock of the owner. While a thread holds one, the events it emits wait; the last one it lets go of delivers them.
pub(crate) struct Held<'a, T> {
    guard: Option<MutexGuard<'a, T>>,
    outbox: &'a Outbox,
}

impl<'a, T> Held<'a, T> {
    /// Lock `mutex`, taking a poisoned lock over: what the owner guards stays consistent at every step that can panic.
    pub(crate) fn lock(mutex: &'a Mutex<T>, outbox: &'a Outbox) -> Self {
        let guard = mutex.lock().unwrap_or_else(PoisonError::into_inner);
        HELD.with(|count| count.set(count.get().saturating_add(1)));
        Self {
            guard: Some(guard),
            outbox,
        }
    }
}

impl<T> Deref for Held<'_, T> {
    type Target = T;

    fn deref(&self) -> &T {
        match &self.guard {
            Some(guard) => guard,
            // Only `drop` takes the guard.
            None => unreachable!("a held lock has its guard"),
        }
    }
}

impl<T> DerefMut for Held<'_, T> {
    fn deref_mut(&mut self) -> &mut T {
        match &mut self.guard {
            Some(guard) => guard,
            None => unreachable!("a held lock has its guard"),
        }
    }
}

impl<T> Drop for Held<'_, T> {
    fn drop(&mut self) {
        drop(self.guard.take());
        HELD.with(|count| count.set(count.get().saturating_sub(1)));
        self.outbox.deliver();
    }
}

#[cfg(test)]
mod tests {
    use super::{Held, Outbox};
    use crate::diag::{OwnerDiagnostics, OwnerEvent};
    use std::sync::{Arc, Mutex, OnceLock, PoisonError};

    fn event(generation: u64) -> OwnerEvent {
        OwnerEvent::VulkanDeviceLost { generation }
    }

    /// Records events, and for each of the first `again` it emits one more through the outbox, as a sink that calls
    /// back into the owner would.
    #[derive(Default)]
    struct Echo {
        seen: Mutex<Vec<u64>>,
        outbox: OnceLock<Arc<Outbox>>,
        lock: Mutex<()>,
    }

    impl OwnerDiagnostics for Echo {
        fn record(&self, event: OwnerEvent) {
            // A lock of the owner that the emitting thread held would make this wait for ever.
            let _no_lock_held = self.lock.try_lock().expect("delivered with no lock held");
            let OwnerEvent::VulkanDeviceLost { generation } = event else {
                return;
            };
            self.seen
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .push(generation);
            if generation < 3 {
                if let Some(outbox) = self.outbox.get() {
                    outbox.record(OwnerEvent::VulkanDeviceLost {
                        generation: generation + 10,
                    });
                }
            }
        }
    }

    fn rig() -> (Arc<Echo>, Arc<Outbox>) {
        let echo = Arc::new(Echo::default());
        let outbox = Arc::new(Outbox::new(echo.clone()));
        assert!(echo.outbox.set(Arc::clone(&outbox)).is_ok());
        (echo, outbox)
    }

    fn seen(echo: &Echo) -> Vec<u64> {
        echo.seen
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }

    #[test]
    fn an_event_under_a_lock_of_the_owner_waits_for_its_release() {
        let (echo, outbox) = rig();
        let slot = Mutex::new(());
        {
            let held = Held::lock(&slot, &outbox);
            // The sink takes the same lock as this thread, as `GpuOwner::poll` would.
            let lock = echo.lock.lock().expect("free");
            outbox.record(event(7));
            outbox.record(event(8));
            assert!(
                seen(&echo).is_empty(),
                "nothing is delivered under the lock"
            );
            assert_eq!(outbox.waiting(), 2);
            drop(lock);
            drop(held);
        }
        assert_eq!(seen(&echo), vec![7, 8]);
        assert_eq!(outbox.waiting(), 0);
    }

    #[test]
    fn nested_locks_deliver_with_the_last_one() {
        let (echo, outbox) = rig();
        let (slot, closed) = (Mutex::new(()), Mutex::new(()));
        let outer = Held::lock(&closed, &outbox);
        let inner = Held::lock(&slot, &outbox);
        outbox.record(event(5));
        drop(inner);
        assert!(seen(&echo).is_empty(), "the outer lock is still held");
        drop(outer);
        assert_eq!(seen(&echo), vec![5]);
    }

    #[test]
    fn what_a_sink_emits_is_delivered_after_it_in_order() {
        let (echo, outbox) = rig();
        outbox.record(event(1));
        outbox.record(event(2));
        // 1 echoes 11, 2 echoes 12; each after the event that caused it and none inside another delivery.
        assert_eq!(seen(&echo), vec![1, 11, 2, 12]);
    }

    #[test]
    fn events_of_many_threads_are_all_delivered() {
        let (echo, outbox) = rig();
        let threads: Vec<_> = (0..4)
            .map(|thread| {
                let outbox = Arc::clone(&outbox);
                std::thread::spawn(move || {
                    for index in 0..50 {
                        outbox.record(event(100 + thread * 50 + index));
                    }
                })
            })
            .collect();
        for thread in threads {
            thread.join().expect("ends");
        }
        let mut all = seen(&echo);
        all.sort_unstable();
        assert_eq!(all, (100..300).collect::<Vec<u64>>());
        assert_eq!(outbox.waiting(), 0);
    }
}
