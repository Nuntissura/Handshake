//! The fault seam of this crate's own unit tests. It exists in unit-test builds only. In every other build it answers
//! nothing, holds nothing and replaces nothing, and no Vulkan type of it leaves the crate in either.
//!
//! What it does in unit tests:
//! - It gives single answers of the driver in its place. An injected loss is lost for every call from there on.
//! - A gate keeps submitted work really pending.
//! - Every Vulkan call that waits for the device is replaced by one that ignores its timeout, whatever the machine's
//!   driver does, so no wait of this crate can rely on one.
//! - The creation of a device is held until the test lets it go on, or refused.
//! - A submission can be held inside the call that hands it to the driver, as a driver that stalls there would hold it.
//! - The next memory object can be over the driver's budget, and invalidations of mapped memory are counted.

#[cfg(not(test))]
use crate::outcome::Unavailable;
#[cfg(not(test))]
use ash::vk;

/// The device calls whose answer a unit test may replace.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Call {
    Submit,
    /// The state of a submission's fence.
    Fence,
    /// The state of the fence nothing is submitted with.
    Sentinel,
}

#[cfg(not(test))]
#[derive(Default)]
pub(crate) struct Faults {
    _none: (),
}

#[cfg(not(test))]
impl Faults {
    pub(super) const fn answer(&self, _call: Call) -> Option<vk::Result> {
        None
    }

    /// A semaphore every submission waits for.
    pub(super) const fn gate(&self) -> Option<vk::Semaphore> {
        None
    }

    /// Whether the next memory object is over the driver's budget whatever the driver says.
    pub(super) const fn over_budget(&self) -> bool {
        false
    }

    /// A range of mapped memory was invalidated.
    pub(super) const fn invalidated(&self) {}
}

/// A device is about to be created.
#[cfg(not(test))]
pub(super) const fn opening() -> Result<(), Unavailable> {
    Ok(())
}

/// `device` as this crate calls it.
#[cfg(not(test))]
pub(super) fn table(_instance: &ash::Instance, device: ash::Device) -> ash::Device {
    device
}

#[cfg(test)]
pub(super) use creation::opening;
#[cfg(test)]
pub(crate) use creation::{hold_open, refuse_open};
#[cfg(test)]
pub(super) use deaf::table;
#[cfg(test)]
pub(crate) use deaf::waits;
#[cfg(test)]
pub(crate) use injected::{AfterLoss, Faults};

#[cfg(test)]
mod injected {
    use super::Call;
    use ash::vk;
    use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
    use std::sync::{Condvar, Mutex, MutexGuard, PoisonError};
    use std::time::Duration;

    /// A test that never lets a held submission go fails by what it finds then.
    const STALL_LIMIT: Duration = Duration::from_secs(60);

    /// What a lost device says from the call after the one that met the loss. Vulkan allows each of these answers.
    #[derive(Clone, Copy, Debug)]
    pub(crate) struct AfterLoss {
        pub(crate) fences: vk::Result,
        pub(crate) sentinel: vk::Result,
    }

    impl AfterLoss {
        /// That it is lost.
        pub(crate) const LOST: Self = Self {
            fences: vk::Result::ERROR_DEVICE_LOST,
            sentinel: vk::Result::ERROR_DEVICE_LOST,
        };
        /// As if its work ran for ever and nothing had happened.
        pub(crate) const SILENT: Self = Self {
            fences: vk::Result::NOT_READY,
            sentinel: vk::Result::NOT_READY,
        };
        /// As if its work had ended and nothing had happened.
        pub(crate) const FINISHED: Self = Self {
            fences: vk::Result::SUCCESS,
            sentinel: vk::Result::NOT_READY,
        };
    }

    struct Plan {
        /// The call that meets the loss; from then on the device is lost for every call.
        lose_at: Option<Call>,
        lost: bool,
        after: AfterLoss,
        refuse_submit: Option<vk::Result>,
        gate: Option<vk::Semaphore>,
        /// The queue's last submission met the loss here and was never given to the real queue.
        withheld: bool,
        /// Submissions are held in the driver call until this is cleared.
        stall: bool,
        /// Submissions held now.
        stalled: u32,
    }

    impl Default for Plan {
        fn default() -> Self {
            Self {
                lose_at: None,
                lost: false,
                after: AfterLoss::LOST,
                refuse_submit: None,
                gate: None,
                withheld: false,
                stall: false,
                stalled: 0,
            }
        }
    }

    #[derive(Default)]
    pub(crate) struct Faults {
        plan: Mutex<Plan>,
        changed: Condvar,
        /// The next memory object is over budget.
        over_budget: AtomicBool,
        invalidations: AtomicU64,
    }

    impl Faults {
        fn lock(&self) -> MutexGuard<'_, Plan> {
            self.plan.lock().unwrap_or_else(PoisonError::into_inner)
        }

        /// Whether the next memory object is over the driver's budget; answered once.
        pub(crate) fn over_budget(&self) -> bool {
            self.over_budget.swap(false, Ordering::SeqCst)
        }

        /// The next memory object is over budget, as a driver that says so would have it.
        pub(crate) fn refuse_budget_once(&self) {
            self.over_budget.store(true, Ordering::SeqCst);
        }

        pub(crate) fn invalidated(&self) {
            self.invalidations.fetch_add(1, Ordering::SeqCst);
        }

        /// Ranges of mapped memory invalidated on this generation.
        pub(crate) fn invalidations(&self) -> u64 {
            self.invalidations.load(Ordering::SeqCst)
        }

        pub(crate) fn answer(&self, call: Call) -> Option<vk::Result> {
            let mut plan = self.lock();
            if call == Call::Submit && plan.stall {
                plan.stalled += 1;
                self.changed.notify_all();
                plan = self
                    .changed
                    .wait_timeout_while(plan, STALL_LIMIT, |plan| plan.stall)
                    .unwrap_or_else(PoisonError::into_inner)
                    .0;
                plan.stalled -= 1;
            }
            let meets = plan.lose_at == Some(call);
            if meets {
                plan.lose_at = None;
                plan.lost = true;
            }
            if call == Call::Submit {
                if let Some(result) = plan.refuse_submit.take() {
                    return Some(result);
                }
            }
            if !plan.lost {
                return None;
            }
            Some(match call {
                Call::Submit => {
                    plan.withheld = true;
                    vk::Result::ERROR_DEVICE_LOST
                }
                _ if meets => vk::Result::ERROR_DEVICE_LOST,
                Call::Fence => plan.after.fences,
                Call::Sentinel => plan.after.sentinel,
            })
        }

        /// Whether the queue's last submission was never given to the real queue.
        pub(crate) fn withheld_last(&self) -> bool {
            self.lock().withheld
        }

        pub(crate) fn gate(&self) -> Option<vk::Semaphore> {
            self.lock().gate
        }

        /// The device is lost from the next `call` on.
        pub(crate) fn lose_at(&self, call: Call) {
            self.lock().lose_at = Some(call);
        }

        /// The device is lost, and what it says from now on is `after`.
        pub(crate) fn lose(&self, after: AfterLoss) {
            let mut plan = self.lock();
            plan.lost = true;
            plan.after = after;
        }

        /// What the device says once it is lost, from now on.
        pub(crate) fn after_loss(&self, after: AfterLoss) {
            self.lock().after = after;
        }

        /// From now on every submission is held inside the driver call until [`Self::release_submits`].
        pub(crate) fn stall_submits(&self) {
            self.lock().stall = true;
        }

        pub(crate) fn release_submits(&self) {
            self.lock().stall = false;
            self.changed.notify_all();
        }

        /// Until a submission is held. `false` when none came.
        pub(crate) fn is_stalled(&self) -> bool {
            let plan = self.lock();
            let (plan, _) = self
                .changed
                .wait_timeout_while(plan, STALL_LIMIT, |plan| plan.stalled == 0)
                .unwrap_or_else(PoisonError::into_inner);
            plan.stalled > 0
        }

        /// The queue refuses the next submission with `result`.
        pub(crate) fn refuse_submit(&self, result: vk::Result) {
            self.lock().refuse_submit = Some(result);
        }

        /// From now on every submission waits for [`Self::open_gate`]: it is really pending until then.
        pub(crate) fn close_gate(&self, device: &ash::Device) {
            let mut kind = vk::SemaphoreTypeCreateInfo::default()
                .semaphore_type(vk::SemaphoreType::TIMELINE)
                .initial_value(0);
            let create = vk::SemaphoreCreateInfo::default().push_next(&mut kind);
            let mut plan = self.lock();
            assert!(plan.gate.is_none(), "one gate per generation");
            // SAFETY: valid device with `timelineSemaphore` enabled.
            let gate = unsafe { device.create_semaphore(&create, None) }.expect("gate semaphore");
            plan.gate = Some(gate);
        }

        /// Let the held submissions run.
        pub(crate) fn open_gate(&self, device: &ash::Device) {
            let Some(gate) = self.gate() else { return };
            // SAFETY: valid device and timeline semaphore; the query has no other precondition.
            let opened = unsafe { device.get_semaphore_counter_value(gate) }.unwrap_or(0) >= 1;
            if !opened {
                let signal = vk::SemaphoreSignalInfo::default().semaphore(gate).value(1);
                // SAFETY: the value is above the current one and nothing else signals the gate.
                unsafe { device.signal_semaphore(&signal) }.expect("gate opens");
            }
        }

        /// Every submission that waited for the gate has ended.
        pub(crate) fn destroy_gate(&self, device: &ash::Device) {
            if let Some(gate) = self.lock().gate.take() {
                // SAFETY: called by the only holder of the device, after all its work has ended.
                unsafe { device.destroy_semaphore(gate, None) };
            }
        }
    }
}

/// A driver that ignores timeouts. The machine's own may or may not.
#[cfg(test)]
mod deaf {
    use ash::vk;
    use std::ffi::{c_void, CStr};
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::Duration;

    /// How long a wait takes: far longer than every bound the tests check, and short enough for them to say so.
    const DEAF: Duration = Duration::from_secs(25);

    static WAITS: AtomicU64 = AtomicU64::new(0);

    /// How many calls that wait for the device were made in this process.
    pub(crate) fn waits() -> u64 {
        WAITS.load(Ordering::SeqCst)
    }

    fn wait() -> vk::Result {
        WAITS.fetch_add(1, Ordering::SeqCst);
        std::thread::sleep(DEAF);
        // No evidence of anything, so a test that does not look at the time fails as well.
        vk::Result::ERROR_OUT_OF_HOST_MEMORY
    }

    unsafe extern "system" fn wait_semaphores(
        _device: vk::Device,
        _info: *const vk::SemaphoreWaitInfo<'_>,
        _timeout: u64,
    ) -> vk::Result {
        wait()
    }

    unsafe extern "system" fn wait_for_fences(
        _device: vk::Device,
        _count: u32,
        _fences: *const vk::Fence,
        _all: vk::Bool32,
        _timeout: u64,
    ) -> vk::Result {
        wait()
    }

    unsafe extern "system" fn device_wait_idle(_device: vk::Device) -> vk::Result {
        wait()
    }

    unsafe extern "system" fn queue_wait_idle(_queue: vk::Queue) -> vk::Result {
        wait()
    }

    fn replacement(name: &CStr) -> Option<*const c_void> {
        Some(match name.to_bytes() {
            b"vkWaitSemaphores" | b"vkWaitSemaphoresKHR" => {
                wait_semaphores as vk::PFN_vkWaitSemaphores as *const c_void
            }
            b"vkWaitForFences" => wait_for_fences as vk::PFN_vkWaitForFences as *const c_void,
            b"vkDeviceWaitIdle" => device_wait_idle as vk::PFN_vkDeviceWaitIdle as *const c_void,
            b"vkQueueWaitIdle" => queue_wait_idle as vk::PFN_vkQueueWaitIdle as *const c_void,
            _ => return None,
        })
    }

    /// `device`, with every call that waits replaced.
    pub(in crate::vulkan) fn table(instance: &ash::Instance, device: ash::Device) -> ash::Device {
        let handle = device.handle();
        let load = |name: &CStr| {
            replacement(name).unwrap_or_else(|| {
                // SAFETY: `device` was created from `instance`, and the name is a C string.
                let address = unsafe { instance.get_device_proc_addr(handle, name.as_ptr()) };
                address.map_or(std::ptr::null(), |function| function as *const c_void)
            })
        };
        // SAFETY: every address is the device's own for its name, or one of the functions above, which have the
        // signature the specification gives that name.
        unsafe { ash::Device::load_with(load, handle) }
    }
}

/// What a test does to the creation of devices in its process.
#[cfg(test)]
mod creation {
    use crate::outcome::Unavailable;
    use std::sync::{Condvar, Mutex, MutexGuard, PoisonError};
    use std::time::Duration;

    /// A test that never lets go of a creation fails by what it finds then.
    const LIMIT: Duration = Duration::from_secs(60);

    struct Creation {
        refuse: bool,
        hold: bool,
        /// A creation waits for the test.
        waiting: bool,
    }

    static CREATION: Mutex<Creation> = Mutex::new(Creation {
        refuse: false,
        hold: false,
        waiting: false,
    });
    static CHANGED: Condvar = Condvar::new();

    fn lock() -> MutexGuard<'static, Creation> {
        CREATION.lock().unwrap_or_else(PoisonError::into_inner)
    }

    pub(in crate::vulkan) fn opening() -> Result<(), Unavailable> {
        let mut creation = lock();
        creation.waiting = creation.hold;
        CHANGED.notify_all();
        let (mut creation, _) = CHANGED
            .wait_timeout_while(creation, LIMIT, |creation| creation.hold)
            .unwrap_or_else(PoisonError::into_inner);
        creation.waiting = false;
        if creation.refuse {
            return Err(Unavailable::DeviceCreationRefused);
        }
        Ok(())
    }

    /// Whether the next generations find no adapter.
    pub(crate) fn refuse_open(refuse: bool) {
        lock().refuse = refuse;
    }

    /// Device creations wait until this is dropped.
    pub(crate) struct HeldOpen {
        _private: (),
    }

    pub(crate) fn hold_open() -> HeldOpen {
        lock().hold = true;
        HeldOpen { _private: () }
    }

    impl HeldOpen {
        /// Until a creation waits. `false` when none came.
        pub(crate) fn is_reached(&self) -> bool {
            let (creation, _) = CHANGED
                .wait_timeout_while(lock(), LIMIT, |creation| !creation.waiting)
                .unwrap_or_else(PoisonError::into_inner);
            creation.waiting
        }
    }

    impl Drop for HeldOpen {
        fn drop(&mut self) {
            lock().hold = false;
            CHANGED.notify_all();
        }
    }
}
