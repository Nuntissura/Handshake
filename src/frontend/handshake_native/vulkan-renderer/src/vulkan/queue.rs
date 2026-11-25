//! The owner's queue: record, submit and observe, with one fence per submission.
//!
//! The queue, the highest value it was given and the objects of work that may still run sit behind one lock, which is
//! the queue's external synchronization. Nothing outside this module touches the queue.
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
    fn of(result: vk::Result) -> Self {
        match result {
            vk::Result::ERROR_DEVICE_LOST => Self::Lost,
            vk::Result::ERROR_OUT_OF_HOST_MEMORY | vk::Result::ERROR_OUT_OF_DEVICE_MEMORY => {
                Self::OutOfMemory
            }
            other => Self::Driver(other.as_raw()),
        }
    }
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
    queue: vk::Queue,
    /// Highest value the queue was given.
    last_value: u64,
    /// Value up to which every submission was seen completed.
    observed: u64,
    /// Work whose end was not seen, in submission order.
    pending: Vec<PendingWork>,
    /// Nothing is submitted any more.
    sealed: bool,
    /// The device is known to be lost: what its fences say of the work that was pending then is not that it completed.
    lost: bool,
    /// Sealed, and seen to be no longer in use by the device. All three are for good.
    drained: bool,
}

impl QueueState {
    pub(super) fn new(queue: vk::Queue) -> Self {
        Self {
            queue,
            last_value: 0,
            observed: 0,
            pending: Vec::new(),
            sealed: false,
            lost: false,
            drained: false,
        }
    }

    fn lose(&mut self) {
        self.sealed = true;
        self.lost = true;
    }
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
        // From here on every early return drops `work`, which destroys what there is.
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

    /// Whether this work was recorded on `generation`.
    pub(crate) fn is_on(&self, generation: &Arc<VulkanGeneration>) -> bool {
        Arc::ptr_eq(&self.generation, generation)
    }

    /// End recording and submit as `value`, which must be above every value given before on this generation.
    ///
    /// With [`QueueFault::Lost`] the work may have reached the queue: its objects are kept until the generation is
    /// destroyed. With every other fault the queue did not take it (Vulkan requires a failing submit that is not
    /// device loss to leave all state unaffected) and they are destroyed.
    pub(crate) fn submit(mut self, value: u64) -> Result<(), QueueFault> {
        let device = &self.generation.device;
        // SAFETY: `begin` put the buffer in the recording state and its pool is used by this value only. A client
        // that breaks the recording rule of `Recording` does so through its own `unsafe` calls.
        unsafe { device.end_command_buffer(self.commands) }.map_err(QueueFault::of)?;
        let mut state = self.generation.lock_queue();
        if state.sealed {
            // Only a lost generation is sealed while work can still be brought to it.
            return Err(QueueFault::Lost);
        }
        let objects = match self.objects {
            Some(objects) if value > state.last_value => objects,
            // The owner hands out values in order; a value out of order never reaches the queue.
            _ => return Err(QueueFault::Driver(vk::Result::ERROR_UNKNOWN.as_raw())),
        };
        let buffers = [self.commands];
        // Waits for nothing outside unit tests.
        let gate = self.generation.faults.gate();
        let waits = gate.as_slice();
        let wait_values = &[1u64][..waits.len()];
        let wait_stages = &[vk::PipelineStageFlags::ALL_COMMANDS][..waits.len()];
        let mut timeline =
            vk::TimelineSemaphoreSubmitInfo::default().wait_semaphore_values(wait_values);
        let submit = vk::SubmitInfo::default()
            .wait_semaphores(waits)
            .wait_dst_stage_mask(wait_stages)
            .command_buffers(&buffers)
            .push_next(&mut timeline);
        let result = match self.generation.faults.answer(Call::Submit) {
            Some(result) => Err(result),
            // SAFETY: the buffer is executable, primary, of the queue's family and never submitted before; the fence
            // is unsignalled and was never submitted; the queue lock is held, so access to the queue is
            // synchronized, and nobody else refers to the fence; all referenced arrays outlive the call.
            None => unsafe { device.queue_submit(state.queue, &[submit], objects.fence) },
        };
        match result {
            Err(error) if error != vk::Result::ERROR_DEVICE_LOST => Err(QueueFault::of(error)),
            accepted => {
                state.last_value = value;
                // The objects now belong to the pending list.
                self.objects = None;
                state.pending.push(PendingWork { value, objects });
                if accepted.is_err() {
                    state.lose();
                }
                accepted.map_err(QueueFault::of)
            }
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
    }
}

impl VulkanGeneration {
    fn lock_queue(&self) -> MutexGuard<'_, QueueState> {
        self.queue.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// The objects of all work, for the final destruction.
    pub(super) fn take_objects(&mut self) -> Vec<WorkObjects> {
        let state = self.queue.get_mut().unwrap_or_else(PoisonError::into_inner);
        std::mem::take(&mut state.pending)
            .into_iter()
            .map(|work| work.objects)
            .collect()
    }

    /// Whether `fence` is signalled. Reports a state and does not wait.
    fn is_signalled(&self, fence: vk::Fence, call: Call) -> Result<bool, QueueFault> {
        let result = match self.faults.answer(call) {
            Some(vk::Result::SUCCESS) => Ok(true),
            Some(vk::Result::NOT_READY) => Ok(false),
            Some(error) => Err(error),
            // SAFETY: valid device and a fence of it that lives until this returns: the caller holds the queue lock,
            // under which alone a fence leaves the pending list, or the fence is the sentinel. The query needs no
            // external synchronization.
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
        self.lock_queue().last_value
    }

    /// Look, without waiting, how far the queue got, and free the objects of finished work. Returns the value up to
    /// which every submission completed.
    ///
    /// A device that is known to be lost completes nothing any more, because its fences may then say anything
    /// (`vkGetFenceStatus`). Nor does a look that meets the loss, on a fence or on the sentinel, which is asked last.
    pub(crate) fn advance(&self) -> Result<u64, QueueFault> {
        let mut state = self.lock_queue();
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
        state.drained = state.sealed && state.pending.is_empty();
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

    /// Take nothing more. For good.
    pub(crate) fn seal(&self) {
        self.lock_queue().sealed = true;
    }

    /// The device was observed lost: take nothing more and call nothing completed any more. For good.
    pub(crate) fn lose(&self) {
        self.lock_queue().lose();
    }

    pub(crate) fn is_lost(&self) -> bool {
        self.lock_queue().lost
    }

    /// Evidence that nothing the sealed queue took is still in use by the device.
    pub(crate) fn evidence(&self) -> Option<Quiescent> {
        let state = self.lock_queue();
        (state.sealed && state.drained).then_some(Quiescent { _private: () })
    }

    pub(crate) fn is_drained(&self) -> bool {
        self.evidence().is_some()
    }

    /// Look, without waiting, whether everything the sealed queue took has ended.
    ///
    /// On a healthy device it has when all of it completed. On a lost one it has when every fence answers with
    /// success or device loss, which for this question is the same (Vulkan "Lost Device").
    pub(crate) fn confirm(&self) -> bool {
        if !self.is_lost() {
            // The result is in the marks.
            let _ = self.advance();
        }
        let mut state = self.lock_queue();
        if state.lost && !state.drained {
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
        if !self.lock_queue().sealed {
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
            let state = self.lock_queue();
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
