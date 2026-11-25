//! Submit, observe, wait and probe: the only way work reaches the owner's queue, and the only ways its end is seen.
//!
//! A submission is counted in flight before the queue takes it and stays counted until the owner observes its
//! fence, its loss or that it never reached the queue. Nothing here blocks on the device except
//! [`crate::GpuClient::wait`] and [`crate::GpuOwner::probe`], which are bounded by their caller and hold no lock
//! meanwhile: settlement is asynchronous, and a client may observe whenever it needs to know.

use super::{Live, OwnerShared};
use crate::device::Recording;
use crate::ledger::{Claim, LedgerClient, Submission};
use crate::outcome::{
    AuthorizeRefusal, DeviceFault, DeviceHealth, RecordAborted, SubmissionRefusal, SubmitRefusal,
};
use crate::records::SubmissionState;
use crate::vulkan::{QueueFault, RecordedWork};
use std::sync::Arc;
use std::time::Duration;

/// Bound on one wait for the device ([`crate::GpuClient::wait`], [`crate::GpuOwner::probe`], and the draining in
/// [`crate::GpuOwner::close`]). A wait on a semaphore is not promised to end on a lost device.
pub const MAX_WAIT: Duration = Duration::from_secs(5);

/// `timeout`, within [`MAX_WAIT`].
pub(super) fn bounded_ns(timeout: Duration) -> u64 {
    u64::try_from(timeout.min(MAX_WAIT).as_nanos()).unwrap_or(0)
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
    fn device_failed(&self, live: &Live, fault: QueueFault) -> DeviceFault {
        let typed = fault_of(fault, live.number);
        if fault == QueueFault::Lost {
            self.core.device_lost(live.number);
            live.vulkan.seal();
            // The result is in the drained mark.
            let _ = live.vulkan.drain(0);
            if live.vulkan.is_drained() {
                self.core.ledger().resolve_lost(live.number);
            }
        } else {
            self.core.device_fault(live.number, typed);
        }
        typed
    }

    /// [`Self::device_failed`] for generation `number`, when that is still the bound one.
    fn generation_failed(&self, number: u64, fault: QueueFault) -> DeviceFault {
        let slot = self.lock_slot();
        match slot.live.as_ref() {
            Some(live) if live.number == number => self.device_failed(live, fault),
            _ => fault_of(fault, number),
        }
    }

    /// Ask the device, without blocking, how far its fence got, and end every submission it passed.
    fn advance(&self, live: &Live) -> Result<(), DeviceFault> {
        match live.vulkan.advance() {
            Ok(value) => {
                self.core.ledger().resolve_through(live.number, value);
                Ok(())
            }
            Err(fault) => Err(self.device_failed(live, fault)),
        }
    }

    pub(super) fn poll(&self) -> Result<(), DeviceFault> {
        match self.lock_slot().live.as_ref() {
            Some(live) => self.advance(live),
            None => Err(DeviceFault::NoDevice),
        }
    }

    pub(super) fn probe(&self, timeout: Duration) -> Result<DeviceHealth, DeviceFault> {
        let (number, vulkan) = match self.lock_slot().live.as_ref() {
            Some(live) => (live.number, Arc::clone(&live.vulkan)),
            None => return Err(DeviceFault::NoDevice),
        };
        let timeout_ns = bounded_ns(timeout);
        if matches!(self.core.ledger().device(), Some(device) if device.lost) {
            // Known already; this only ends submissions an earlier observation could not.
            let _ = vulkan.drain(timeout_ns);
            drop(vulkan);
            return Err(self.generation_failed(number, QueueFault::Lost));
        }
        let waited = vulkan.wait_for(vulkan.last_signal(), timeout_ns);
        // The device is not held any longer than the wait itself.
        drop(vulkan);
        match waited {
            Err(fault) => Err(self.generation_failed(number, fault)),
            Ok(idle) => {
                self.poll()?;
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
        // Under the slot lock, so fence values reach the queue in the order the ledger hands them out.
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
        let (number, vulkan) = {
            let slot = self.lock_slot();
            client
                .authorize_submission(claim)
                .map_err(SubmitRefusal::NotAuthorized)?;
            match slot.live.as_ref() {
                Some(live) if live.number == claim.generation() => {
                    (live.number, Arc::clone(&live.vulkan))
                }
                _ => {
                    let refusal = AuthorizeRefusal::DeviceUnavailable;
                    return Err(refuse(client, claim, SubmitRefusal::NotAuthorized(refusal)));
                }
            }
        };
        let work = match RecordedWork::begin(vulkan) {
            Ok(work) => work,
            Err(fault) => {
                let fault = self.generation_failed(number, fault);
                return Err(refuse(client, claim, SubmitRefusal::Device(fault)));
            }
        };
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
        let slot = self.lock_slot();
        let known = submission.state();
        if known != SubmissionState::Pending {
            return Ok(known);
        }
        let outcome = match slot.live.as_ref() {
            Some(live) if live.number == submission.generation() => self.advance(live),
            _ => Err(DeviceFault::NoDevice),
        };
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
        let vulkan = match self.lock_slot().live.as_ref() {
            Some(live)
                if client.owns(submission)
                    && live.number == submission.generation()
                    && submission.state() == SubmissionState::Pending =>
            {
                Some(Arc::clone(&live.vulkan))
            }
            _ => None,
        };
        if let Some(vulkan) = vulkan {
            let waited = vulkan.wait_for(submission.sequence(), bounded_ns(timeout));
            // The device is not held any longer than the wait itself.
            drop(vulkan);
            if let Err(fault) = waited {
                self.generation_failed(submission.generation(), fault);
            }
        }
        self.observe(client, submission)
    }
}

fn refuse(client: &LedgerClient, claim: &Claim, refusal: SubmitRefusal) -> SubmitRefusal {
    client.submit_refused(claim, refusal);
    refusal
}
