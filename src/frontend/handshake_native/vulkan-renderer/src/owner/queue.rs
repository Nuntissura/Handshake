//! Submit, observe and wait: the only way work reaches the owner's queue.
//!
//! A submission is counted in flight before the queue takes it and stays counted until the owner observes its
//! fence, its loss or that it never reached the queue. Nothing here blocks except [`GpuOwner::wait`], which is
//! bounded by its caller: settlement is asynchronous, and a client may observe whenever it needs to know.

use super::{GpuOwner, Live};
use crate::device::Recording;
use crate::ledger::{Claim, Submission};
use crate::outcome::{
    AuthorizeRefusal, DeviceFault, RecordAborted, SubmissionRefusal, SubmitRefusal,
};
use crate::records::SubmissionState;
use crate::vulkan::{QueueFault, RecordedWork};
use std::sync::Arc;
use std::time::Duration;

/// Bound on one [`GpuOwner::wait`]. A wait on a semaphore is not promised to end on a lost device, and while it
/// runs the lost device cannot be replaced.
pub const MAX_WAIT: Duration = Duration::from_secs(5);

impl GpuOwner {
    /// Record work under `claim` and submit it. Returns the submission without waiting for it.
    ///
    /// `record` runs on this thread, before the owner's queue is touched and with no lock of the owner held. The
    /// claim is authorized before recording and again when the work is submitted, so a claim that is revoked, lost
    /// or settled while recording submits nothing.
    pub fn submit<F>(&self, claim: &Claim, record: F) -> Result<Submission, SubmitRefusal>
    where
        F: FnOnce(&Recording) -> Result<(), RecordAborted>,
    {
        let work = self.record(claim, record)?;
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
                return Err(self.refuse(SubmitRefusal::NotAuthorized(refusal)));
            }
        };
        // Under the slot lock, so fence values reach the queue in the order the ledger hands them out.
        let submission = self
            .core
            .begin_submission(claim)
            .map_err(SubmitRefusal::NotAuthorized)?;
        match work.submit(submission.sequence()) {
            Ok(()) => Ok(submission),
            Err(QueueFault::Lost) => {
                // The work may have reached the queue: it stays in flight until the lost device is quiescent.
                let fault = self.device_failed(live, QueueFault::Lost);
                Err(self.refuse(SubmitRefusal::Device(fault)))
            }
            Err(fault) => {
                // The queue did not take it, so it ends here as having had no effect.
                let _ = self.core.submission_no_effect(&submission);
                let fault = self.device_failed(live, fault);
                Err(self.refuse(SubmitRefusal::Device(fault)))
            }
        }
    }

    fn refuse(&self, refusal: SubmitRefusal) -> SubmitRefusal {
        self.core.submit_refused(refusal);
        refusal
    }

    /// Authorize `claim`, begin a command buffer on its generation and let the client record into it.
    fn record<F>(&self, claim: &Claim, record: F) -> Result<RecordedWork, SubmitRefusal>
    where
        F: FnOnce(&Recording) -> Result<(), RecordAborted>,
    {
        let (number, vulkan) = {
            let slot = self.lock_slot();
            self.core
                .authorize(claim)
                .map_err(SubmitRefusal::NotAuthorized)?;
            match slot.live.as_ref() {
                Some(live) if live.number == claim.generation() => {
                    (live.number, Arc::clone(&live.vulkan))
                }
                _ => {
                    let refusal = AuthorizeRefusal::DeviceUnavailable;
                    return Err(self.refuse(SubmitRefusal::NotAuthorized(refusal)));
                }
            }
        };
        let work = match RecordedWork::begin(vulkan) {
            Ok(work) => work,
            Err(fault) => {
                let fault = self.generation_failed(number, fault);
                return Err(self.refuse(SubmitRefusal::Device(fault)));
            }
        };
        match record(&Recording::new(work.raw(), number)) {
            Ok(()) => Ok(work),
            Err(RecordAborted) => Err(self.refuse(SubmitRefusal::RecordAborted)),
        }
    }

    /// Ask the device, without blocking, how far its fence got, and end every submission it passed.
    fn advance(&self, live: &Live) -> Result<(), DeviceFault> {
        match live.vulkan.advance() {
            Ok(value) => {
                self.core.resolve_through(live.number, value);
                Ok(())
            }
            Err(fault) => Err(self.device_failed(live, fault)),
        }
    }

    /// Where `submission` stands, asking the device when the owner does not know yet. Does not wait for work.
    ///
    /// A submission that was already observed answers from what was observed, also after its generation is gone.
    /// On a refusal it stays pending.
    pub fn observe(&self, submission: &Submission) -> Result<SubmissionState, SubmissionRefusal> {
        if !self.core.issued(submission) {
            self.core.submission_refused(SubmissionRefusal::Unknown);
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
                self.core.submission_refused(refusal);
                Err(refusal)
            }
        }
    }

    /// Wait at most `timeout`, and never longer than [`MAX_WAIT`], for `submission`, then answer as
    /// [`Self::observe`] does. Other clients are not blocked by the wait.
    pub fn wait(
        &self,
        submission: &Submission,
        timeout: Duration,
    ) -> Result<SubmissionState, SubmissionRefusal> {
        let vulkan = {
            let slot = self.lock_slot();
            match slot.live.as_ref() {
                Some(live)
                    if self.core.issued(submission)
                        && live.number == submission.generation()
                        && submission.state() == SubmissionState::Pending =>
                {
                    Some(Arc::clone(&live.vulkan))
                }
                _ => None,
            }
        };
        if let Some(vulkan) = vulkan {
            let timeout_ns = u64::try_from(timeout.min(MAX_WAIT).as_nanos()).unwrap_or(0);
            let waited = vulkan.wait_for(submission.sequence(), timeout_ns);
            // The device is not held any longer than the wait itself.
            drop(vulkan);
            if let Err(fault) = waited {
                self.generation_failed(submission.generation(), fault);
            }
        }
        self.observe(submission)
    }
}
