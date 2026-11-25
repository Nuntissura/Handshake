//! The owner's queue: record, submit and observe, with one timeline semaphore as the fence of every submission.
//!
//! The queue, the last signalled value and the command pools of work that may still run sit behind one lock, which
//! is the queue's external synchronization. Nothing outside this module touches the queue.

use super::VulkanGeneration;
use ash::vk::{self, Handle};
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

/// See [`VulkanGeneration::quiesce`]. It holds for what was submitted before it was taken.
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
}

impl QueueState {
    pub(super) fn new(queue: vk::Queue) -> Self {
        Self {
            queue,
            last_signal: 0,
            pending: Vec::new(),
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
        if signal <= state.last_signal {
            // The owner hands out values in order; a value out of order is never put on the fence.
            return Err(QueueFault::Driver(vk::Result::ERROR_UNKNOWN.as_raw()));
        }
        let buffers = [self.commands];
        let semaphores = [self.generation.timeline];
        let values = [signal];
        let mut timeline =
            vk::TimelineSemaphoreSubmitInfo::default().signal_semaphore_values(&values);
        let submit = vk::SubmitInfo::default()
            .command_buffers(&buffers)
            .signal_semaphores(&semaphores)
            .push_next(&mut timeline);
        // SAFETY: the buffer is executable, primary, of the queue's family and never submitted before; the fence
        // value is above every earlier one; the queue lock is held, so access to the queue is synchronized; all
        // referenced arrays outlive the call.
        let result = unsafe { device.queue_submit(state.queue, &[submit], vk::Fence::null()) };
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

    /// Read the fence without blocking and free the command pools of finished work. Returns the value up to which
    /// every submission completed.
    pub(crate) fn advance(&self) -> Result<u64, QueueFault> {
        let mut state = self.lock_queue();
        // SAFETY: valid device and timeline semaphore; the query needs no external synchronization.
        let value = unsafe { self.device.get_semaphore_counter_value(self.timeline) }
            .map_err(QueueFault::of)?;
        if value > state.last_signal {
            // No submission signals such a value, so it says nothing about what finished. Report device loss when
            // that is why; otherwise nothing is observed and everything stays pending.
            // SAFETY: the queue lock is held, so access to the queue is synchronized.
            unsafe { self.device.device_wait_idle() }.map_err(QueueFault::of)?;
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

    /// Wait until the fence reaches `value` or `timeout_ns` passed. `false` on timeout.
    pub(crate) fn wait_for(&self, value: u64, timeout_ns: u64) -> Result<bool, QueueFault> {
        let semaphores = [self.timeline];
        let values = [value];
        let wait = vk::SemaphoreWaitInfo::default()
            .semaphores(&semaphores)
            .values(&values);
        // SAFETY: valid device and timeline semaphore; the referenced arrays outlive the call; the wait needs no
        // external synchronization.
        match unsafe { self.device.wait_semaphores(&wait, timeout_ns) } {
            Ok(()) => Ok(true),
            Err(vk::Result::TIMEOUT) => Ok(false),
            Err(error) => Err(QueueFault::of(error)),
        }
    }

    /// Wait until the device is idle. This is how the owner itself observes a lost device.
    pub(crate) fn wait_idle(&self) -> Result<(), QueueFault> {
        let _queue = self.lock_queue();
        // SAFETY: valid device; the queue lock is held, so access to the device's one queue is synchronized. The
        // wait returns in finite time even on a lost device (Vulkan "Lost Device").
        unsafe { self.device.device_wait_idle() }.map_err(QueueFault::of)
    }

    /// Evidence that nothing submitted so far is still in use by the device, or `None` when the device did not
    /// say so. Device loss is such evidence: for this question `VK_ERROR_DEVICE_LOST` is equivalent to success
    /// (Vulkan "Lost Device").
    pub(crate) fn quiesce(&self) -> Option<Quiescent> {
        match self.wait_idle() {
            Ok(()) | Err(QueueFault::Lost) => Some(Quiescent { _private: () }),
            Err(_) => None,
        }
    }
}
