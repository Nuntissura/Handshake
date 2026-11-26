//! The owner's queue: record, submit and observe, with one fence per submission.
//!
//! Two locks, so that a driver that stalls in `vkQueueSubmit` or `vkQueuePresentKHR` keeps nothing but other
//! submissions and presents waiting (MT-203 E3-1):
//! - The lane ([`Lane`]) is the external synchronization of the queue that `vkQueueSubmit` and `vkQueuePresentKHR`
//!   require. It is held across those calls and for nothing else.
//! - The state ([`QueueState`]): the highest value given, the objects of work that may still run, and the marks
//!   sealed, lost and drained. It is held only briefly and never across a call that hands work to the device. A
//!   submission is counted in it (`submitting`) before the call and moved to the pending list after it, so a queue that
//!   is sealed meanwhile is not drained while it runs, and sealing never waits for it.
//!
//! # Nothing here waits inside Vulkan
//!
//! A driver can ignore the timeout of `vkWaitSemaphores` (seen on Adreno X2-85, driver 0x8036f000), and
//! `vkDeviceWaitIdle` and `vkQueueWaitIdle` have none. None of them is called, nor is `vkWaitForFences`. Whoever waits
//! looks with `vkGetFenceStatus`, which reports a state and takes no timeout, sleeps and looks again until its own
//! deadline ([`Backoff`]). Why that is enough (Vulkan specification):
//! - The fence of a submission is signalled after everything earlier in submission order ("Fence Signaling"), so it
//!   stands for all of it.
//! - On a lost device, repeated `vkGetFenceStatus` ends in `VK_SUCCESS` or `VK_ERROR_DEVICE_LOST` (`vkGetFenceStatus`),
//!   and either says that the work is no longer in use ("Lost Device"). `vkWaitSemaphores` without a timeout and the
//!   counter of a timeline semaphore come with no such promise.
//! - For the same reason a fence that is never submitted shows the loss of an idle device: on a healthy one it stays
//!   unsignalled for ever. That is the sentinel of a generation.

use super::backoff::Backoff;
use super::fault::Call;
use super::VulkanGeneration;
use ash::vk::{self, Handle};
use std::sync::{Arc, MutexGuard, PoisonError};

/// A fault of the device, before the owner attaches the generation number.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum QueueFault {
    /// `VK_ERROR_DEVICE_LOST`, or a sentinel that is signalled, and nothing else.
    Lost,
    OutOfMemory,
    Driver(i32),
}

impl QueueFault {
    pub(super) fn of(result: vk::Result) -> Self {
        match result {
            vk::Result::ERROR_DEVICE_LOST => Self::Lost,
            vk::Result::ERROR_OUT_OF_HOST_MEMORY | vk::Result::ERROR_OUT_OF_DEVICE_MEMORY => {
                Self::OutOfMemory
            }
            other => Self::Driver(other.as_raw()),
        }
    }
}

/// Why a submission did not reach the queue, or reached it only on a lost device.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SubmitFault {
    /// The queue takes nothing more (a close, or an earlier loss). Nothing was handed to the device.
    Sealed,
    /// The device answered. With [`QueueFault::Lost`] the work may have reached the queue and stays pending until the
    /// lost device is quiescent; with every other fault the queue did not take it (Vulkan requires a failing submit
    /// that is not device loss to leave all state unaffected).
    Device(QueueFault),
}

/// Evidence that the queue takes nothing more and that nothing it took is still in use by the device.
pub(crate) struct Quiescent {
    _private: (),
}

/// The Vulkan objects of one submission.
#[derive(Clone, Copy)]
pub(super) struct WorkObjects {
    pool: vk::CommandPool,
    /// Signalled when the submission has ended.
    fence: vk::Fence,
}

impl WorkObjects {
    /// # Safety
    /// The objects are `device`'s, nothing refers to them any more, and the submission that uses them was never taken
    /// by the queue or is no longer in use by the device.
    pub(super) unsafe fn destroy(self, device: &ash::Device) {
        // SAFETY: the caller's. Destroying the pool frees its command buffer in whatever state recording left it; a
        // null fence is ignored.
        unsafe {
            device.destroy_fence(self.fence, None);
            device.destroy_command_pool(self.pool, None);
        }
    }
}

struct PendingWork {
    value: u64,
    objects: WorkObjects,
}

pub(super) struct QueueState {
    /// Highest value the queue was given.
    pub(super) last_value: u64,
    /// Value up to which every submission was seen completed.
    observed: u64,
    /// Work whose end was not seen, in submission order.
    pending: Vec<PendingWork>,
    /// Submissions between the check of `sealed` and their place in `pending`: in `vkQueueSubmit` now.
    submitting: u32,
    /// Command buffers being recorded or submitted.
    pub(super) recording: u32,
    /// Nothing is submitted any more.
    sealed: bool,
    /// The device is known to be lost: what its fences say of the work that was pending then is not that it completed.
    lost: bool,
    /// Sealed, and seen to be no longer in use by the device. All three are for good.
    drained: bool,
}

impl QueueState {
    pub(super) fn new() -> Self {
        Self {
            last_value: 0,
            observed: 0,
            pending: Vec::new(),
            submitting: 0,
            recording: 0,
            sealed: false,
            lost: false,
            drained: false,
        }
    }

    fn lose(&mut self) {
        self.sealed = true;
        self.lost = true;
    }

    /// Nothing that was handed to the device can still be in use by it, as far as the pending list knows.
    fn empty(&self) -> bool {
        self.pending.is_empty() && self.submitting == 0
    }
}

/// What the resources of a generation need of its queue: how far the device got and whether it may still run work.
#[derive(Clone, Copy, Debug)]
pub(crate) struct QueueView {
    pub(crate) observed: u64,
    pub(crate) recording: u32,
    pub(crate) lost: bool,
    pub(crate) drained: bool,
}

/// The queue, behind its own lock: the external synchronization `vkQueueSubmit` and `vkQueuePresentKHR` require.
pub(crate) struct Lane {
    pub(super) queue: vk::Queue,
}

/// The lane, held: submissions and presents run one at a time and in the order the owner hands out their values.
pub(crate) struct LaneGuard<'a> {
    pub(super) guard: MutexGuard<'a, Lane>,
}

/// Binary semaphores one submission waits for and signals (presentation).
#[derive(Clone, Copy, Default)]
pub(crate) struct SubmitSync<'a> {
    pub(crate) waits: &'a [vk::Semaphore],
    pub(crate) signals: &'a [vk::Semaphore],
}

/// A fence that nothing is submitted with yet.
pub(super) fn create_fence(device: &ash::Device) -> Result<vk::Fence, vk::Result> {
    // SAFETY: valid device; the create info has no pointer to outlive.
    unsafe { device.create_fence(&vk::FenceCreateInfo::default(), None) }
}

/// A command buffer in a pool of its own, being recorded or ready to submit. Dropping it before it was submitted
/// destroys its objects.
pub(crate) struct RecordedWork {
    generation: Arc<VulkanGeneration>,
    /// `None` once the queue took the work.
    objects: Option<WorkObjects>,
    commands: vk::CommandBuffer,
}

impl RecordedWork {
    /// Allocate and begin a one-time command buffer on `generation`.
    pub(crate) fn begin(generation: Arc<VulkanGeneration>) -> Result<Self, QueueFault> {
        let create = vk::CommandPoolCreateInfo::default()
            .flags(vk::CommandPoolCreateFlags::TRANSIENT)
            .queue_family_index(generation.identity.queue_family);
        // SAFETY: valid device; the family is the one its queue was created from.
        let pool = unsafe { generation.device.create_command_pool(&create, None) }
            .map_err(QueueFault::of)?;
        generation.lock_state().recording += 1;
        // From here on every early return drops `work`, which destroys what there is and ends the recording.
        let mut work = Self {
            generation,
            objects: Some(WorkObjects {
                pool,
                fence: vk::Fence::null(),
            }),
            commands: vk::CommandBuffer::null(),
        };
        let device = &work.generation.device;
        let fence = create_fence(device).map_err(QueueFault::of)?;
        work.objects = Some(WorkObjects { pool, fence });
        let allocate = vk::CommandBufferAllocateInfo::default()
            .command_pool(pool)
            .level(vk::CommandBufferLevel::PRIMARY)
            .command_buffer_count(1);
        let begin = vk::CommandBufferBeginInfo::default()
            .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);
        // SAFETY: the pool was created above and only this value uses it.
        let buffers =
            unsafe { device.allocate_command_buffers(&allocate) }.map_err(QueueFault::of)?;
        let commands = buffers.first().copied().ok_or(QueueFault::OutOfMemory)?;
        // SAFETY: the buffer was just allocated (initial state) from a pool only this value uses.
        unsafe { device.begin_command_buffer(commands, &begin) }.map_err(QueueFault::of)?;
        work.commands = commands;
        Ok(work)
    }

    /// The raw command buffer, for the client's recording call.
    pub(crate) fn raw(&self) -> u64 {
        self.commands.as_raw()
    }

    pub(crate) fn commands(&self) -> vk::CommandBuffer {
        self.commands
    }

    pub(crate) fn generation(&self) -> &Arc<VulkanGeneration> {
        &self.generation
    }

    /// Whether this work was recorded on `generation`.
    pub(crate) fn is_on(&self, generation: &Arc<VulkanGeneration>) -> bool {
        Arc::ptr_eq(&self.generation, generation)
    }

    /// End recording and submit as `value`, which must be above every value given before on this generation. The
    /// caller holds the lane of this generation; nothing else of the owner is locked meanwhile.
    pub(crate) fn submit(
        mut self,
        lane: &LaneGuard<'_>,
        value: u64,
        sync: SubmitSync<'_>,
    ) -> Result<(), SubmitFault> {
        let device = &self.generation.device;
        // SAFETY: `begin` put the buffer in the recording state and its pool is used by this value only. A client
        // that breaks the recording rule of `Recording` does so through its own `unsafe` calls.
        unsafe { device.end_command_buffer(self.commands) }
            .map_err(|result| SubmitFault::Device(QueueFault::of(result)))?;
        let objects = {
            let mut state = self.generation.lock_state();
            if state.sealed {
                return Err(SubmitFault::Sealed);
            }
            match self.objects {
                Some(objects) if value > state.last_value => {
                    state.submitting += 1;
                    objects
                }
                // The owner hands out values in order; a value out of order never reaches the queue.
                _ => {
                    let unknown = vk::Result::ERROR_UNKNOWN.as_raw();
                    return Err(SubmitFault::Device(QueueFault::Driver(unknown)));
                }
            }
        };
        let result = self.hand_over(lane, objects, sync);
        let mut state = self.generation.lock_state();
        state.submitting -= 1;
        match result {
            Err(error) if error != vk::Result::ERROR_DEVICE_LOST => {
                Err(SubmitFault::Device(QueueFault::of(error)))
            }
            accepted => {
                state.last_value = value;
                // The objects now belong to the pending list.
                self.objects = None;
                state.pending.push(PendingWork { value, objects });
                if accepted.is_err() {
                    state.lose();
                }
                accepted.map_err(|error| SubmitFault::Device(QueueFault::of(error)))
            }
        }
    }

    /// `vkQueueSubmit`, with no lock but the lane held.
    fn hand_over(
        &self,
        lane: &LaneGuard<'_>,
        objects: WorkObjects,
        sync: SubmitSync<'_>,
    ) -> Result<(), vk::Result> {
        let buffers = [self.commands];
        // The gate waits outside unit tests never; the binary semaphores of presentation wait with any value.
        let mut waits: Vec<vk::Semaphore> = sync.waits.to_vec();
        waits.extend(self.generation.faults.gate());
        let mut values = vec![0u64; sync.waits.len()];
        values.resize(waits.len(), 1);
        let stages = vec![vk::PipelineStageFlags::ALL_COMMANDS; waits.len()];
        let signal_values = vec![0u64; sync.signals.len()];
        let mut timeline = vk::TimelineSemaphoreSubmitInfo::default()
            .wait_semaphore_values(&values)
            .signal_semaphore_values(&signal_values);
        let submit = vk::SubmitInfo::default()
            .wait_semaphores(&waits)
            .wait_dst_stage_mask(&stages)
            .command_buffers(&buffers)
            .signal_semaphores(sync.signals)
            .push_next(&mut timeline);
        match self.generation.faults.answer(Call::Submit) {
            Some(result) => Err(result),
            // SAFETY: the buffer is executable, primary, of the queue's family and never submitted before; the fence
            // is unsignalled and was never submitted; the lane is held, so access to the queue is synchronized, and
            // nobody else refers to the fence; the semaphores are the caller's, unsignalled with no pending signal
            // (signals) or with a signal submitted and no other wait (waits); all referenced arrays outlive the call.
            None => unsafe {
                self.generation
                    .device
                    .queue_submit(lane.guard.queue, &[submit], objects.fence)
            },
        }
    }
}

impl Drop for RecordedWork {
    fn drop(&mut self) {
        if let Some(objects) = self.objects.take() {
            // SAFETY: the queue never took this work (`submit` gives the objects of taken work away), so its buffer
            // is not pending and its fence not in use, and only this value refers to them.
            unsafe { objects.destroy(&self.generation.device) };
        }
        self.generation.recording_ended();
    }
}

impl VulkanGeneration {
    pub(super) fn lock_state(&self) -> MutexGuard<'_, QueueState> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// The lane: waits only for another submission or present of this generation.
    pub(crate) fn lane(&self) -> LaneGuard<'_> {
        LaneGuard {
            guard: self.lane.lock().unwrap_or_else(PoisonError::into_inner),
        }
    }

    /// A recording ended (submitted or dropped). Resources released while it ran wait for it.
    fn recording_ended(&self) {
        let mut state = self.lock_state();
        state.recording = state.recording.saturating_sub(1);
        if state.recording == 0 {
            // Under the state lock, so no recording starts before the deferred releases have their value.
            self.resources.recordings_ended(state.last_value);
        }
    }

    /// What resources need to know of the queue.
    pub(crate) fn view(&self) -> QueueView {
        let state = self.lock_state();
        QueueView {
            observed: state.observed,
            recording: state.recording,
            lost: state.lost,
            drained: state.sealed && state.drained,
        }
    }

    /// The objects of all work, for the final destruction.
    pub(super) fn take_objects(&mut self) -> Vec<WorkObjects> {
        let state = self.state.get_mut().unwrap_or_else(PoisonError::into_inner);
        std::mem::take(&mut state.pending)
            .into_iter()
            .map(|work| work.objects)
            .collect()
    }

    /// Whether `fence` is signalled. Reports a state and does not wait.
    pub(super) fn is_signalled(&self, fence: vk::Fence, call: Call) -> Result<bool, QueueFault> {
        let result = match self.faults.answer(call) {
            Some(vk::Result::SUCCESS) => Ok(true),
            Some(vk::Result::NOT_READY) => Ok(false),
            Some(error) => Err(error),
            // SAFETY: valid device and a fence of it that lives until this returns: the caller holds the lock under
            // which alone that fence is destroyed, or the fence is the sentinel. The query needs no external
            // synchronization.
            None => unsafe { self.device.get_fence_status(fence) },
        };
        result.map_err(QueueFault::of)
    }

    /// Whether the device shows its loss on the fence that nothing is ever submitted with.
    fn sentinel(&self) -> Result<(), QueueFault> {
        match self.is_signalled(self.sentinel, Call::Sentinel) {
            Ok(false) => Ok(()),
            Ok(true) => Err(QueueFault::Lost),
            Err(fault) => Err(fault),
        }
    }

    /// How many of `pending`, from the first on, have ended on a device that shows no loss.
    fn ended(&self, pending: &[PendingWork]) -> Result<usize, QueueFault> {
        let mut ended = 0;
        for work in pending {
            // In submission order: after the first that has not ended, none has.
            if !self.is_signalled(work.objects.fence, Call::Fence)? {
                break;
            }
            ended += 1;
        }
        self.sentinel()?;
        Ok(ended)
    }

    /// Highest value the queue was given.
    pub(crate) fn last_value(&self) -> u64 {
        self.lock_state().last_value
    }

    /// Look, without waiting, how far the queue got, and free the objects of finished work. Returns the value up to
    /// which every submission completed.
    ///
    /// A device that is known to be lost completes nothing any more, because its fences may then say anything
    /// (`vkGetFenceStatus`). Nor does a look that meets the loss, on a fence or on the sentinel, which is asked last.
    pub(crate) fn advance(&self) -> Result<u64, QueueFault> {
        let mut state = self.lock_state();
        if state.lost {
            return Err(QueueFault::Lost);
        }
        let ended = match self.ended(&state.pending) {
            Ok(ended) => ended,
            Err(fault) => {
                if fault == QueueFault::Lost {
                    state.lose();
                }
                return Err(fault);
            }
        };
        let finished: Vec<PendingWork> = state.pending.drain(..ended).collect();
        if let Some(last) = finished.last() {
            state.observed = last.value;
        }
        state.drained = state.sealed && state.empty();
        let observed = state.observed;
        drop(state);
        for work in finished {
            // SAFETY: this work's fence is signalled and its device was not lost, so its submission has completed; it
            // left the pending list above and nothing else refers to its objects.
            unsafe { work.objects.destroy(&self.device) };
        }
        Ok(observed)
    }

    /// Wait until everything up to `value` is seen completed, or `backoff` ends. `false` when it ended first.
    pub(crate) fn wait_for(&self, value: u64, backoff: &mut Backoff) -> Result<bool, QueueFault> {
        loop {
            if self.advance()? >= value {
                return Ok(true);
            }
            if !backoff.pause() {
                return Ok(false);
            }
        }
    }

    /// Take nothing more. For good. Never waits for a submission that is in the driver now.
    pub(crate) fn seal(&self) {
        self.lock_state().sealed = true;
    }

    /// The device was observed lost: take nothing more and call nothing completed any more. For good.
    pub(crate) fn lose(&self) {
        self.lock_state().lose();
    }

    pub(crate) fn is_lost(&self) -> bool {
        self.lock_state().lost
    }

    /// Evidence that nothing the sealed queue took is still in use by the device.
    pub(crate) fn evidence(&self) -> Option<Quiescent> {
        let state = self.lock_state();
        (state.sealed && state.drained).then_some(Quiescent { _private: () })
    }

    pub(crate) fn is_drained(&self) -> bool {
        self.evidence().is_some()
    }

    /// Look, without waiting, whether everything the sealed queue took has ended.
    ///
    /// On a healthy device it has when all of it completed. On a lost one it has when every fence answers with
    /// success or device loss, which for this question is the same (Vulkan "Lost Device"). A submission that is in the
    /// driver now has not ended.
    pub(crate) fn confirm(&self) -> bool {
        if !self.is_lost() {
            // The result is in the marks.
            let _ = self.advance();
        }
        let mut state = self.lock_state();
        if state.lost && !state.drained && state.submitting == 0 {
            state.drained = state.pending.iter().all(|work| {
                matches!(
                    self.is_signalled(work.objects.fence, Call::Fence),
                    Ok(true) | Err(QueueFault::Lost)
                )
            });
        }
        state.sealed && state.drained
    }

    /// [`Self::confirm`], looking again until `backoff` ends. `false` when it ended first, or nothing is sealed.
    pub(crate) fn drain(&self, backoff: &mut Backoff) -> bool {
        if !self.lock_state().sealed {
            return false;
        }
        loop {
            if self.confirm() {
                return true;
            }
            if !backoff.pause() {
                return false;
            }
        }
    }
}

#[cfg(test)]
impl VulkanGeneration {
    /// Wait until the real device ended everything up to `value`, whatever is injected, without telling the owner.
    pub(crate) fn wait_really(&self, value: u64) {
        assert!(
            self.really_ended(value),
            "the real device still runs its work"
        );
    }

    fn really_ended(&self, value: u64) -> bool {
        let mut backoff = Backoff::new(std::time::Duration::from_secs(10));
        loop {
            let state = self.lock_state();
            let withheld = usize::from(self.faults.withheld_last());
            let given = state.pending.len().saturating_sub(withheld);
            let ended = state
                .pending
                .iter()
                .take(given)
                .filter(|work| work.value <= value)
                // SAFETY: as in `is_signalled`.
                .all(
                    |work| unsafe { self.device.get_fence_status(work.objects.fence) } == Ok(true),
                );
            drop(state);
            if ended || !backoff.pause() {
                return ended;
            }
        }
    }

    /// An injected loss is no real one: the real device ends what it was given before its objects go.
    pub(super) fn let_it_end(&self) {
        self.faults.open_gate(&self.device);
        assert!(
            self.really_ended(u64::MAX),
            "the real device still runs its work"
        );
        self.faults.destroy_gate(&self.device);
    }
}
