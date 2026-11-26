//! Submit, observe, wait and probe: the only way work reaches the owner's queue, and the only ways its end is seen.
//!
//! A submission is counted in flight before the queue takes it and stays counted until the owner observes its
//! fence, its loss or that it never reached the queue. Nothing here waits for the device except
//! [`crate::GpuClient::wait`], [`crate::GpuOwner::probe`] and [`crate::DeviceLease::wait_release`], which are bounded by
//! their caller and by [`MAX_WAIT`] and hold no lock meanwhile: settlement is asynchronous, and a client may observe
//! whenever it needs to know.
//!
//! `vkQueueSubmit` runs with the generation's lane held and nothing else of the owner (MT-203 E3-1): a driver that
//! stalls in it keeps other submissions and presents waiting, and nothing that polls, observes, returns a lease,
//! closes or drops. Values reach the queue in the order the ledger hands them out because both happen under the lane.

use super::outbox::Quiet;
use super::{Live, OwnerShared};
use crate::device::Recording;
use crate::diag::{OwnerEvent, Subject};
use crate::ledger::{Claim, LedgerClient, OwnedEnd, Submission};
use crate::outcome::{
    AuthorizeRefusal, DeviceFault, DeviceHealth, RecordAborted, SubmissionRefusal, SubmitRefusal,
};
use crate::records::SubmissionState;
use crate::resource::FrameKey;
use crate::vulkan::{
    Backoff, PresentEnd, QueueFault, RecordedWork, SubmitFault, SubmitSync, Targets,
};
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

pub(super) fn fault_of(fault: QueueFault, generation: u64) -> DeviceFault {
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

    /// Ask the device of `live`, without waiting, how far its queue got, end every submission it completed, and
    /// destroy the resources, swapchains and surfaces that nothing may use any more.
    pub(super) fn look(&self, live: &Live) -> Result<(), DeviceFault> {
        let result = match live.vulkan.advance() {
            Ok(value) => {
                self.core.ledger().resolve_through(live.number, value);
                Ok(())
            }
            Err(fault) => Err(self.device_failed(live, fault)),
        };
        self.end_resources(live);
        result
    }

    /// Destroy what was released and can no longer be in use, and release its charge once. The targets of windows whose
    /// surfaces ended are dropped once this thread holds no lock of the owner.
    pub(super) fn end_resources(&self, live: &Live) {
        let mut ended: Vec<OwnedEnd> = live
            .vulkan
            .retire_ended()
            .into_iter()
            .map(|owned| OwnedEnd {
                claim: owned.claim,
                class: owned.class,
                bytes: owned.bytes,
            })
            .collect();
        let mut targets = Targets::new();
        for end in live.vulkan.retire_presentation() {
            match end {
                PresentEnd::Swapchain { claim, swapchain } => {
                    self.core.report(OwnerEvent::SwapchainEnded {
                        subject: Subject::claim(&claim),
                        swapchain,
                    });
                }
                PresentEnd::Surface { owned, target } => {
                    targets.extend(target);
                    ended.push(OwnedEnd {
                        claim: owned.claim,
                        class: owned.class,
                        bytes: owned.bytes,
                    });
                }
            }
        }
        self.core.ledger().owned_ended(&ended);
        self.outbox.bury(targets);
    }

    pub(super) fn poll(&self) -> Result<(), DeviceFault> {
        let live = self.lock_slot().live.clone();
        let result = match &live {
            Some(live) => self.look(live),
            None => Err(DeviceFault::NoDevice),
        };
        drop(live);
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
        F: FnOnce(&Recording<'_>) -> Result<(), RecordAborted>,
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
        F: FnOnce(&Recording<'_>) -> Result<(), RecordAborted>,
    {
        let work = self.record(client, claim, None, record)?;
        let generation = Arc::clone(work.generation());
        // Declared first, so it ends last: nothing reaches the sink while the lane is held.
        let _quiet = Quiet::begin(&self.outbox);
        let lane = generation.lane();
        let live = self.submitting_on(client, claim, &work)?;
        let submission = client
            .begin_submission(claim)
            .map_err(SubmitRefusal::NotAuthorized)?;
        let submitted = work.submit(&lane, submission.sequence(), SubmitSync::default());
        drop(lane);
        match submitted {
            Ok(()) => Ok(submission),
            Err(fault) => Err(self.submit_failed(client, claim, &live, &submission, fault)),
        }
    }

    /// The bound generation, when `work` was recorded on it. Asked with the lane held.
    pub(super) fn submitting_on(
        &self,
        client: &LedgerClient,
        claim: &Claim,
        work: &RecordedWork,
    ) -> Result<Live, SubmitRefusal> {
        let live = self.lock_slot().live.clone();
        match live {
            Some(live) if work.is_on(&live.vulkan) => Ok(live),
            other => {
                let refusal = match other {
                    Some(_) => AuthorizeRefusal::Lost {
                        generation: claim.generation(),
                    },
                    None => AuthorizeRefusal::DeviceUnavailable,
                };
                Err(refuse(client, claim, SubmitRefusal::NotAuthorized(refusal)))
            }
        }
    }

    /// What a submission that did not reach a healthy queue means for the ledger, and the refusal for its client.
    pub(super) fn submit_failed(
        &self,
        client: &LedgerClient,
        claim: &Claim,
        live: &Live,
        submission: &Submission,
        fault: SubmitFault,
    ) -> SubmitRefusal {
        let fault = match fault {
            SubmitFault::Sealed => {
                // Nothing was handed to the device.
                let _ = self.core.ledger().submission_no_effect(submission);
                if live.vulkan.is_lost() {
                    DeviceFault::Lost {
                        generation: live.number,
                    }
                } else {
                    DeviceFault::NoDevice
                }
            }
            // The work may have reached the queue: it stays in flight until the lost device is quiescent.
            SubmitFault::Device(QueueFault::Lost) => self.device_failed(live, QueueFault::Lost),
            SubmitFault::Device(fault) => {
                // The queue did not take it, so it ends here as having had no effect.
                let _ = self.core.ledger().submission_no_effect(submission);
                self.device_failed(live, fault)
            }
        };
        refuse(client, claim, SubmitRefusal::Device(fault))
    }

    /// Authorize `claim` for one more submission, begin a command buffer on its generation and let the client record
    /// into it. `frame` is the frame of a present, the only one the recording may use.
    pub(super) fn record<F>(
        &self,
        client: &LedgerClient,
        claim: &Claim,
        frame: Option<FrameKey>,
        record: F,
    ) -> Result<RecordedWork, SubmitRefusal>
    where
        F: FnOnce(&Recording<'_>) -> Result<(), RecordAborted>,
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
        let recording = Recording::new(&work, number, client.ledger_key(), client.seat(), frame);
        match record(&recording) {
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
        let live = self
            .lock_slot()
            .generation(submission.generation())
            .cloned();
        let outcome = match &live {
            Some(live) => self.look(live),
            None => Err(DeviceFault::NoDevice),
        };
        drop(live);
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

pub(super) fn refuse(
    client: &LedgerClient,
    claim: &Claim,
    refusal: SubmitRefusal,
) -> SubmitRefusal {
    client.submit_refused(claim, refusal);
    refusal
}
