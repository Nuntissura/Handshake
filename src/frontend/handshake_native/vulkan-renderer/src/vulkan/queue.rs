//! The owner's queue: record, submit and observe, with one timeline semaphore as the fence of every submission.
//!
//! The queue, the last signalled value and the command pools of work that may still run sit behind one lock, which
//! is the queue's external synchronization. Nothing outside this module touches the queue. No wait for the device
//! runs with that lock held.

use super::fault::Call;
use super::VulkanGeneration;
use ash::vk::{self, Handle};
use std::sync::atomic::Ordering;
use std::sync::{Arc, MutexGuard, PoisonError};

/// A fault of the device, before the owner attaches the generation number.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum QueueFault {
    /// `VK_ERROR_DEVICE_LOST`, and nothing else.
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

struct PendingWork {
    signal: u64,
    pool: vk::CommandPool,
}

pub(super) struct QueueState {
    queue: vk::Queue,
    /// Highest value a submission signals.
    last_signal: u64,
    /// Work whose fence value has not been observed, in signal order.
    pending: Vec<PendingWork>,
    /// Nothing is submitted any more.
    sealed: bool,
    /// Sealed, and observed to be no longer in use by the device. Both are for good.
    drained: bool,
}

impl QueueState {
    pub(super) fn new(queue: vk::Queue) -> Self {
        Self {
            queue,
            last_signal: 0,
            pending: Vec::new(),
            sealed: false,
            drained: false,
        }
    }
}

/// The owner's fence: a timeline semaphore starting at 0.
pub(super) fn create_timeline(device: &ash::Device) -> Result<vk::Semaphore, vk::Result> {
    let mut kind = vk::SemaphoreTypeCreateInfo::default()
        .semaphore_type(vk::SemaphoreType::TIMELINE)
        .initial_value(0);
    let create = vk::SemaphoreCreateInfo::default().push_next(&mut kind);
    // SAFETY: valid device with `timelineSemaphore` enabled; the chained struct lives for the call.
    unsafe { device.create_semaphore(&create, None) }
}

/// A command buffer in a pool of its own, being recorded or ready to submit. Dropping it before it was submitted
/// destroys the pool.
pub(crate) struct RecordedWork {
    generation: Arc<VulkanGeneration>,
    /// `None` once the queue took the work.
    pool: Option<vk::CommandPool>,
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
        // From here on every early return drops `work`, which destroys the pool.
        let mut work = Self {
            generation,
            pool: Some(pool),
            commands: vk::CommandBuffer::null(),
        };
        let allocate = vk::CommandBufferAllocateInfo::default()
            .command_pool(pool)
            .level(vk::CommandBufferLevel::PRIMARY)
            .command_buffer_count(1);
        let begin = vk::CommandBufferBeginInfo::default()
            .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);
        let device = &work.generation.device;
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

    /// End recording and submit, signalling `signal` on the fence. `signal` must be above every value signalled
    /// before on this generation.
    ///
    /// With [`QueueFault::Lost`] the work may have reached the queue: its pool is kept until the generation is
    /// destroyed. With every other fault the queue did not take it (Vulkan requires a failing submit that is not
    /// device loss to leave all state unaffected) and the pool is destroyed.
    pub(crate) fn submit(mut self, signal: u64) -> Result<(), QueueFault> {
        let device = &self.generation.device;
        // SAFETY: `begin` put the buffer in the recording state and its pool is used by this value only. A client
        // that breaks the recording rule of `Recording` does so through its own `unsafe` calls.
        unsafe { device.end_command_buffer(self.commands) }.map_err(QueueFault::of)?;
        let mut state = self.generation.lock_queue();
        if state.sealed {
            // Only a lost generation is sealed while work can still be brought to it.
            return Err(QueueFault::Lost);
        }
        if signal <= state.last_signal {
            // The owner hands out values in order; a value out of order is never put on the fence.
            return Err(QueueFault::Driver(vk::Result::ERROR_UNKNOWN.as_raw()));
        }
        let buffers = [self.commands];
        let semaphores = [self.generation.timeline];
        let values = [signal];
        // Waits for nothing outside unit tests.
        let gate = self.generation.faults.gate();
        let waits = gate.as_slice();
        let wait_values = &[1u64][..waits.len()];
        let wait_stages = &[vk::PipelineStageFlags::ALL_COMMANDS][..waits.len()];
        let mut timeline = vk::TimelineSemaphoreSubmitInfo::default()
            .wait_semaphore_values(wait_values)
            .signal_semaphore_values(&values);
        let submit = vk::SubmitInfo::default()
            .wait_semaphores(waits)
            .wait_dst_stage_mask(wait_stages)
            .command_buffers(&buffers)
            .signal_semaphores(&semaphores)
            .push_next(&mut timeline);
        let result = match self.generation.faults.answer(Call::Submit) {
            Some(result) => Err(result),
            // SAFETY: the buffer is executable, primary, of the queue's family and never submitted before; the
            // fence value is above every earlier one; the queue lock is held, so access to the queue is
            // synchronized; all referenced arrays outlive the call.
            None => unsafe { device.queue_submit(state.queue, &[submit], vk::Fence::null()) },
        };
        match result {
            Err(error) if error != vk::Result::ERROR_DEVICE_LOST => Err(QueueFault::of(error)),
            accepted => {
                state.last_signal = signal;
                // The pool now belongs to the pending list.
                if let Some(pool) = self.pool.take() {
                    state.pending.push(PendingWork { signal, pool });
                }
                accepted.map_err(QueueFault::of)
            }
        }
    }
}

impl Drop for RecordedWork {
    fn drop(&mut self) {
        if let Some(pool) = self.pool.take() {
            // SAFETY: the queue never took this work (`submit` moves a taken pool out), so its buffer is not
            // pending; destroying the pool frees the buffer in whatever state recording left it.
            unsafe { self.generation.device.destroy_command_pool(pool, None) };
        }
    }
}

impl VulkanGeneration {
    fn lock_queue(&self) -> MutexGuard<'_, QueueState> {
        self.queue.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// The pools of all work, for the final destruction.
    pub(super) fn take_pools(&mut self) -> Vec<vk::CommandPool> {
        let state = self.queue.get_mut().unwrap_or_else(PoisonError::into_inner);
        std::mem::take(&mut state.pending)
            .into_iter()
            .map(|work| work.pool)
            .collect()
    }

    fn counter(&self) -> Result<u64, QueueFault> {
        match self.faults.answer(Call::Counter) {
            Some(result) => Err(result),
            // SAFETY: valid device and timeline semaphore; the query needs no external synchronization.
            None => unsafe { self.device.get_semaphore_counter_value(self.timeline) },
        }
        .map_err(QueueFault::of)
    }

    /// Highest value a submission of this generation signals.
    pub(crate) fn last_signal(&self) -> u64 {
        self.lock_queue().last_signal
    }

    /// Read the fence without blocking and free the command pools of finished work. Returns the value up to which
    /// every submission completed.
    pub(crate) fn advance(&self) -> Result<u64, QueueFault> {
        let mut state = self.lock_queue();
        let value = self.counter()?;
        if value > state.last_signal {
            // No submission signals such a value, so it says nothing about what finished. Report device loss when
            // that is why; otherwise nothing is observed and everything stays pending.
            let last = state.last_signal;
            drop(state);
            self.wait_for(last, 0)?;
            return Err(QueueFault::Driver(vk::Result::ERROR_UNKNOWN.as_raw()));
        }
        let (finished, running): (Vec<PendingWork>, Vec<PendingWork>) =
            std::mem::take(&mut state.pending)
                .into_iter()
                .partition(|work| work.signal <= value);
        state.pending = running;
        drop(state);
        for work in finished {
            // SAFETY: the fence passed this work's value, so its command buffer is no longer pending; the pool
            // left the pending list above and nothing else refers to it.
            unsafe { self.device.destroy_command_pool(work.pool, None) };
        }
        Ok(value)
    }

    /// Wait until the fence reaches `value` or `timeout_ns` passed. `false` on timeout. With 0 it does not block.
    pub(crate) fn wait_for(&self, value: u64, timeout_ns: u64) -> Result<bool, QueueFault> {
        let semaphores = [self.timeline];
        let values = [value];
        let wait = vk::SemaphoreWaitInfo::default()
            .semaphores(&semaphores)
            .values(&values);
        let result = match self.faults.answer(Call::Wait) {
            Some(result) => Err(result),
            // SAFETY: valid device and timeline semaphore; the referenced arrays outlive the call; the wait needs
            // no external synchronization.
            None => unsafe { self.device.wait_semaphores(&wait, timeout_ns) },
        };
        match result {
            Ok(()) => Ok(true),
            Err(vk::Result::TIMEOUT) => Ok(false),
            Err(error) => Err(QueueFault::of(error)),
        }
    }

    /// Take nothing more. For good.
    pub(crate) fn seal(&self) {
        self.lock_queue().sealed = true;
    }

    /// Evidence that nothing the sealed queue took is still in use by the device.
    pub(crate) fn evidence(&self) -> Option<Quiescent> {
        let state = self.lock_queue();
        (state.sealed && state.drained).then_some(Quiescent { _private: () })
    }

    pub(crate) fn is_drained(&self) -> bool {
        self.evidence().is_some()
    }

    fn mark_drained(&self) {
        let mut state = self.lock_queue();
        state.drained = state.sealed;
    }

    /// Wait at most `timeout_ns`, with no lock held, for everything the sealed queue took. With 0 it does not block.
    ///
    /// Every submission signals the fence, so its last value stands for all of them. A lost device is drained as
    /// well: for the question whether work is still in use by the device, `VK_ERROR_DEVICE_LOST` is equivalent to
    /// success (Vulkan "Lost Device").
    pub(crate) fn drain(&self, timeout_ns: u64) -> Result<bool, QueueFault> {
        let last = {
            let state = self.lock_queue();
            if !state.sealed {
                return Ok(false);
            }
            if state.drained {
                return Ok(true);
            }
            state.last_signal
        };
        let result = self.wait_for(last, timeout_ns);
        if matches!(result, Ok(true) | Err(QueueFault::Lost)) {
            self.mark_drained();
        }
        result
    }

    /// Seal and drain a generation that was observed lost, with no lock held. Whether it is drained.
    ///
    /// A bounded wait on the fence comes first. Vulkan does not promise that such a wait ends on a lost device; it
    /// does promise that a wait for idleness does, so that one follows when the fence stays silent.
    pub(crate) fn drain_lost(&self, timeout_ns: u64) -> bool {
        self.seal();
        // The result is in the drained mark.
        let _ = self.drain(timeout_ns);
        self.is_drained() || self.wait_idle_of_lost()
    }

    fn wait_idle_of_lost(&self) -> bool {
        if self.idling.swap(true, Ordering::AcqRel) {
            // Another thread asks already; its answer counts when it has one.
            return self.is_drained();
        }
        let result = match self.faults.answer(Call::Idle) {
            Some(result) => Err(result),
            // SAFETY: valid device. Access to its one queue is synchronized without the queue lock: the queue is
            // sealed under that lock, so no thread submits to it any more, and `idling` lets one thread wait at a
            // time. The caller holds the generation, so it is not destroyed meanwhile. The wait returns in finite
            // time on a lost device (Vulkan "Lost Device").
            None => unsafe { self.device.device_wait_idle() },
        };
        self.idling.store(false, Ordering::Release);
        if matches!(
            result.map_err(QueueFault::of),
            Ok(()) | Err(QueueFault::Lost)
        ) {
            self.mark_drained();
        }
        self.is_drained()
    }
}
