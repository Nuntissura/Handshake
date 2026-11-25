//! The fault seam of this crate's own unit tests: what a driver would answer, put in its place, and a gate that keeps
//! submitted work really pending. It exists in unit-test builds only. In every other build [`Faults`] is empty and
//! answers nothing, and no Vulkan type of it leaves the crate in either.

#[cfg(not(test))]
use ash::vk;

/// The device calls whose answer a unit test may replace.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Call {
    Submit,
    Counter,
    Wait,
    Idle,
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
}

#[cfg(test)]
pub(crate) use injected::{open_refused, refuse_open, Faults};

#[cfg(test)]
mod injected {
    use super::Call;
    use ash::vk;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::{Mutex, MutexGuard, PoisonError};

    static REFUSE_OPEN: AtomicBool = AtomicBool::new(false);

    /// Whether the next generations find no adapter.
    pub(crate) fn refuse_open(refuse: bool) {
        REFUSE_OPEN.store(refuse, Ordering::SeqCst);
    }

    pub(crate) fn open_refused() -> bool {
        REFUSE_OPEN.load(Ordering::SeqCst)
    }

    #[derive(Default)]
    struct Plan {
        /// The call that meets the loss; from then on the device is lost for every call.
        lose_at: Option<Call>,
        lost: bool,
        /// A lost device that never says that its work has ended.
        stalled: bool,
        refuse_submit: Option<vk::Result>,
        gate: Option<vk::Semaphore>,
    }

    #[derive(Default)]
    pub(crate) struct Faults {
        plan: Mutex<Plan>,
    }

    impl Faults {
        fn lock(&self) -> MutexGuard<'_, Plan> {
            self.plan.lock().unwrap_or_else(PoisonError::into_inner)
        }

        pub(crate) fn answer(&self, call: Call) -> Option<vk::Result> {
            let mut plan = self.lock();
            if plan.lose_at == Some(call) {
                plan.lose_at = None;
                plan.lost = true;
            }
            if call == Call::Submit {
                if let Some(result) = plan.refuse_submit.take() {
                    return Some(result);
                }
            }
            match (plan.lost, plan.stalled, call) {
                (false, _, _) => None,
                (true, true, Call::Wait) => Some(vk::Result::TIMEOUT),
                (true, true, Call::Idle) => Some(vk::Result::ERROR_OUT_OF_HOST_MEMORY),
                (true, _, _) => Some(vk::Result::ERROR_DEVICE_LOST),
            }
        }

        pub(crate) fn gate(&self) -> Option<vk::Semaphore> {
            self.lock().gate
        }

        /// The device is lost from the next `call` on.
        pub(crate) fn lose_at(&self, call: Call) {
            self.lock().lose_at = Some(call);
        }

        /// Whether the lost device withholds that its work has ended.
        pub(crate) fn stall(&self, stalled: bool) {
            self.lock().stalled = stalled;
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

        /// Before the device is destroyed: an injected loss is not a real one, so the real device may still run what
        /// it was given. Open the gate, wait for the real end and destroy the gate.
        pub(crate) fn finish(&self, device: &ash::Device) {
            self.open_gate(device);
            let gate = self.lock().gate.take();
            // SAFETY: called by the only holder of the device, with its queue sealed; nothing waits for the gate any
            // more once the device is idle.
            unsafe {
                let _ = device.device_wait_idle();
                if let Some(gate) = gate {
                    device.destroy_semaphore(gate, None);
                }
            }
        }
    }
}
