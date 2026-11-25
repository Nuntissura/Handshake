//! Submit, observe, wait and probe: the only way work reaches the owner's queue, and the only ways its end is seen.
//!
//! A submission is counted in flight before the queue takes it and stays counted until the owner observes its
//! fence, its loss or that it never reached the queue. Nothing here waits for the device except
//! [`crate::GpuClient::wait`], [`crate::GpuOwner::probe`] and [`crate::DeviceLease::wait_release`], which are bounded by
//! their caller and by [`MAX_WAIT`] and hold no lock meanwhile: settlement is asynchronous, and a client may observe
//! whenever it needs to know.

use super::{Live, OwnerShared};
use crate::device::Recording;
use crate::ledger::{Claim, LedgerClient, Submission};
use crate::outcome::{
    AuthorizeRefusal, DeviceFault, DeviceHealth, RecordAborted, SubmissionRefusal, SubmitRefusal,
};
use crate::records::SubmissionState;
use crate::vulkan::{Backoff, QueueFault, RecordedWork};
use std::sync::Arc;
use std::time::Duration;

/// Bound on one wait for the device ([`crate::GpuClient::wait`], [`crate::GpuOwner::probe`],
/// [`crate::DeviceLease::wait_release`], and the draining in [`crate::GpuOwner::close`] and
/// [`crate::GpuOwner::rebind_device`]).
pub const MAX_WAIT: Duration = Duration::from_secs(5);

/// A wait of `timeout`, within [`MAX_WAIT`].
pub(super) fn bounded(timeout: Duration) -> Backoff {
    Backoff::new(timeout.min(MAX_WAIT))
}

fn fault_of(fault: QueueFault, generation: u64) -> DeviceFault {
    match fault {
        QueueFault::Lost => DeviceFault::Lost { generation },
        QueueFault::OutOfMemory => DeviceFault::OutOfMemory,
        QueueFault::Driver(raw) => DeviceFault::Driver(raw),
    }
}

impl OwnerShared {
    /// The device of `live` answered with a fault. Only device loss marks the generation lost; it also ends the
    /// submissions of that generation once the device says, without being waited for, that none of them is still in
    /// use.
    pub(super) fn device_failed(&self, live: &Live, fault: QueueFault) -> DeviceFault {
        let typed = fault_of(fault, live.number);
        if fault == QueueFault::Lost {
            live.vulkan.lose();
            self.core.device_lost(live.number);
            if live.vulkan.confirm() {
                self.core.ledger().resolve_lost(live.number);
            }
        } else {
            self.core.device_fault(live.number, typed);
        }
        typed
    }

    /// Ask the device of `live`, without waiting, how far its queue got, and end every submission it completed.
    pub(super) fn look(&self, live: &Live) -> Result<(), DeviceFault> {
        match live.vulkan.advance() {
            Ok(value) => {
                self.core.ledger().resolve_through(live.number, value);
                Ok(())
            }
            Err(fault) => Err(self.device_failed(live, fault)),
        }
    }

    pub(super) fn poll(&self) -> Result<(), DeviceFault> {
        let result = match self.lock_slot().live.as_ref() {
            Some(live) => self.look(live),
            None => Err(DeviceFault::NoDevice),
        };
        self.reap();
        result
    }

    pub(super) fn probe(&self, timeout: Duration) -> Result<DeviceHealth, DeviceFault> {
        let live = self.lock_slot().live.clone();
        let result = match &live {
            Some(live) => self.probe_device(live, &mut bounded(timeout)),
            None => Err(DeviceFault::NoDevice),
        };
        // The device is not held any longer than the wait itself.
        drop(live);
        self.reap();
        result
    }

    fn probe_device(
        &self,
        live: &Live,
        backoff: &mut Backoff,
    ) -> Result<DeviceHealth, DeviceFault> {
        if live.vulkan.is_lost() {
            // Known already; this only ends submissions an earlier look could not.
            live.vulkan.drain(backoff);
        }
        match live.vulkan.wait_for(live.vulkan.last_value(), backoff) {
            Err(fault) => Err(self.device_failed(live, fault)),
            Ok(idle) => {
                self.look(live)?;
                Ok(if idle {
                    DeviceHealth::Idle
                } else {
                    DeviceHealth::Busy
                })
            }
        }
    }

    pub(super) fn submit<F>(
        &self,
        client: &LedgerClient,
        claim: &Claim,
        record: F,
    ) -> Result<Submission, SubmitRefusal>
    where
        F: FnOnce(&Recording) -> Result<(), RecordAborted>,
    {
        let result = self.submit_work(client, claim, record);
        // A recording held its device as well.
        self.reap();
        result
    }

    fn submit_work<F>(
        &self,
        client: &LedgerClient,
        claim: &Claim,
        record: F,
    ) -> Result<Submission, SubmitRefusal>
    where
        F: FnOnce(&Recording) -> Result<(), RecordAborted>,
    {
        let work = self.record(client, claim, record)?;
        let slot = self.lock_slot();
        let live = match slot.live.as_ref() {
            Some(live) if work.is_on(&live.vulkan) => live,
            other => {
                let refusal = match other {
                    Some(_) => AuthorizeRefusal::Lost {
                        generation: claim.generation(),
                    },
                    None => AuthorizeRefusal::DeviceUnavailable,
                };
                return Err(refuse(client, claim, SubmitRefusal::NotAuthorized(refusal)));
            }
        };
        // Under the slot lock, so values reach the queue in the order the ledger hands them out.
        let submission = client
            .begin_submission(claim)
            .map_err(SubmitRefusal::NotAuthorized)?;
        match work.submit(submission.sequence()) {
            Ok(()) => Ok(submission),
            Err(QueueFault::Lost) => {
                // The work may have reached the queue: it stays in flight until the lost device is quiescent.
                let fault = self.device_failed(live, QueueFault::Lost);
                Err(refuse(client, claim, SubmitRefusal::Device(fault)))
            }
            Err(fault) => {
                // The queue did not take it, so it ends here as having had no effect.
                let _ = self.core.ledger().submission_no_effect(&submission);
                let fault = self.device_failed(live, fault);
                Err(refuse(client, claim, SubmitRefusal::Device(fault)))
            }
        }
    }

    /// Authorize `claim` for one more submission, begin a command buffer on its generation and let the client record
    /// into it.
    fn record<F>(
        &self,
        client: &LedgerClient,
        claim: &Claim,
        record: F,
    ) -> Result<RecordedWork, SubmitRefusal>
    where
        F: FnOnce(&Recording) -> Result<(), RecordAborted>,
    {
        if client.is_in_flight_full(claim) {
            // Submissions whose handle was dropped may have ended since somebody last asked the device.
            let _ = self.poll();
        }
        let live = {
            let slot = self.lock_slot();
            client
                .authorize_submission(claim)
                .map_err(SubmitRefusal::NotAuthorized)?;
            match slot.live.as_ref() {
                Some(live) if live.number == claim.generation() => live.clone(),
                _ => {
                    let refusal = AuthorizeRefusal::DeviceUnavailable;
                    return Err(refuse(client, claim, SubmitRefusal::NotAuthorized(refusal)));
                }
            }
        };
        let work = match RecordedWork::begin(Arc::clone(&live.vulkan)) {
            Ok(work) => work,
            Err(fault) => {
                let fault = self.device_failed(&live, fault);
                return Err(refuse(client, claim, SubmitRefusal::Device(fault)));
            }
        };
        let number = live.number;
        // The recording holds the device now.
        drop(live);
        match record(&Recording::new(work.raw(), number)) {
            Ok(()) => Ok(work),
            Err(RecordAborted) => Err(refuse(client, claim, SubmitRefusal::RecordAborted)),
        }
    }

    pub(super) fn observe(
        &self,
        client: &LedgerClient,
        submission: &Submission,
    ) -> Result<SubmissionState, SubmissionRefusal> {
        if !client.owns(submission) {
            client.submission_refused(submission, SubmissionRefusal::Unknown);
            return Err(SubmissionRefusal::Unknown);
        }
        let known = submission.state();
        if known != SubmissionState::Pending {
            return Ok(known);
        }
        // Its device may be bound, lost and set aside, or that of a closed owner.
        let outcome = match self.lock_slot().generation(submission.generation()) {
            Some(live) => self.look(live),
            None => Err(DeviceFault::NoDevice),
        };
        self.reap();
        match (submission.state(), outcome) {
            // A loss that ended the submission is an answer, not a refusal.
            (state @ SubmissionState::Finished(_), _) | (state, Ok(())) => Ok(state),
            (SubmissionState::Pending, Err(fault)) => {
                let refusal = SubmissionRefusal::Device(fault);
                client.submission_refused(submission, refusal);
                Err(refusal)
            }
        }
    }

    pub(super) fn wait(
        &self,
        client: &LedgerClient,
        submission: &Submission,
        timeout: Duration,
    ) -> Result<SubmissionState, SubmissionRefusal> {
        if client.owns(submission) && submission.state() == SubmissionState::Pending {
            let live = self
                .lock_slot()
                .generation(submission.generation())
                .cloned();
            if let Some(live) = live {
                self.wait_for(&live, submission.sequence(), &mut bounded(timeout));
            }
        }
        self.observe(client, submission)
    }

    /// Wait until the submission `value` on the device of `live` has ended, or `backoff` ends. A loss is reported when
    /// it is met; what is left of the time is for the lost device to say that its work has ended.
    pub(super) fn wait_for(&self, live: &Live, value: u64, backoff: &mut Backoff) {
        if let Err(fault) = live.vulkan.wait_for(value, backoff) {
            self.device_failed(live, fault);
            if fault == QueueFault::Lost {
                live.vulkan.drain(backoff);
            }
        }
    }
}

fn refuse(client: &LedgerClient, claim: &Claim, refusal: SubmitRefusal) -> SubmitRefusal {
    client.submit_refused(claim, refusal);
    refusal
}
