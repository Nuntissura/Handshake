//! A lost device and its recovery (MT-214 AC214-9): every window's surface is released at once, then, on the presenter's
//! own timer, the owner replaces the device, each window's claim moves onto it, a surface and swapchain are made on it
//! and each window is asked for a frame. Nothing waits for the device, a refusal that can pass is looked at again a
//! bounded number of times (`recovery.rs`), and when no device comes back nothing else renders: the application is told
//! ([`DeviceEvent::GaveUp`]) and ends with a typed outcome.
//!
//! The owner's rules are followed as they are written (MT-203): the surface is an owner resource of the claim, so it is
//! let go before the claim moves (`RebindRefusal::ResourcesHeld`); the owner marks a generation lost only from what it
//! observes, so the presenter asks it to look (`GpuOwner::poll`) before it asks to rebind; a device that clients still
//! hold is set aside by the owner and destroyed with its last holder, which is not waited for.

use std::sync::Arc;
use std::time::Instant;

use handshake_shell_window::Control;
use handshake_vulkan_renderer::{DeviceRebindRefusal, RebindRefusal};

use super::Core;
use crate::decide::Need;
use crate::error::PresentError;
use crate::recovery::{DeviceEvent, DeviceState, GaveUp, Loss, Next, Recovery};
use crate::slot::{Capture, Slot};
use crate::target::ShellWindowTarget;

/// How one window's move onto the new device ended.
enum Moved {
    /// It has a surface on the new device and was asked for a frame.
    Drawing,
    /// The new generation was lost already.
    Lost(u64),
    /// Something that can pass keeps it from moving: the owner's key.
    Waiting(&'static str),
    /// It will not move: the owner's key. The window stays without frames.
    Failed(&'static str),
}

impl Core {
    pub(crate) fn device_state(&self) -> DeviceState {
        self.recovery.get().state()
    }

    pub(super) fn take_device_events(&self) -> Vec<DeviceEvent> {
        std::mem::take(&mut *self.device_events.borrow_mut())
    }

    fn tell(&self, event: DeviceEvent) {
        self.device_events.borrow_mut().push(event);
    }

    fn recover<R>(&self, change: impl FnOnce(&mut Recovery) -> R) -> R {
        let mut recovery = self.recovery.get();
        let answer = change(&mut recovery);
        self.recovery.set(recovery);
        self.count(|report| report.device = recovery.state());
        answer
    }

    /// A frame was presented: the losses in a row begin again.
    pub(super) fn note_presented(&self) {
        if self.device_state() == DeviceState::Healthy {
            self.recover(Recovery::presented);
        }
    }

    /// The owner reported the device lost, `generation` when the report named it. Every window's frame says so, but one
    /// recovery follows. Call it with no slot borrowed.
    pub(super) fn device_lost(&self, generation: Option<u64>) {
        match self.recover(|recovery| recovery.lost(generation)) {
            Loss::Already | Loss::Stale => {}
            Loss::Begin => {
                self.count(|report| report.device_losses += 1);
                self.release_for_loss();
                self.tell(DeviceEvent::Lost { generation });
                self.schedule.borrow_mut().arm(Need::Rebind, Instant::now());
            }
            Loss::GaveUp(why) => {
                self.count(|report| report.device_losses += 1);
                self.release_for_loss();
                self.tell(DeviceEvent::Lost { generation });
                self.gave_up(why);
            }
        }
    }

    /// Every window lets go of its surface; what cannot survive ends with a typed outcome.
    fn release_for_loss(&self) {
        for slot in self.all_slots() {
            let mut slot = slot.borrow_mut();
            let pending = matches!(slot.capture, Capture::Pending(_));
            if slot.lose() {
                self.count(|report| report.surfaces_released += 1);
            }
            if pending {
                self.count(|report| report.captures_lost += 1);
            }
            self.schedule.borrow_mut().cancel(Need::Frame(slot.key));
        }
    }

    fn gave_up(&self, why: GaveUp) {
        self.schedule.borrow_mut().cancel(Need::Rebind);
        self.tell(DeviceEvent::GaveUp(why));
    }

    /// Look again later, or give up when the looks are used.
    fn retry(&self, key: &'static str) {
        self.count(|report| report.rebind_looks += 1);
        match self.recover(|recovery| recovery.refused(key)) {
            Next::Retry(delay) => self
                .schedule
                .borrow_mut()
                .arm(Need::Rebind, Instant::now() + delay),
            Next::GaveUp(why) => self.gave_up(why),
        }
    }

    /// One try to replace the device and move the windows onto it.
    pub(super) fn rebind_step(&self, control: &Control) {
        if self.device_state() != DeviceState::Recovering {
            return;
        }
        // The owner marks a generation lost only from what it observes itself: let it look first.
        let _ = self.owner.poll();
        let generation = match self.owner.rebind_device() {
            Ok(generation) => {
                self.count(|report| report.device_rebinds += 1);
                generation
            }
            // Replaced already, or never lost for the owner: its bound generation is the one to move to.
            Err(DeviceRebindRefusal::NotLost) => match self.owner.device() {
                Some(bound) if !bound.lost => bound.generation,
                _ => return self.retry(DeviceRebindRefusal::NotLost.key()),
            },
            Err(DeviceRebindRefusal::Closed) => {
                let why = GaveUp::Rebind(DeviceRebindRefusal::Closed.key());
                self.recover(|recovery| recovery.give_up(why));
                return self.gave_up(why);
            }
            // A held device, work that has not ended, another call replacing it, or no adapter now: may pass.
            Err(refusal) => return self.retry(refusal.key()),
        };
        self.recover(|recovery| recovery.device_replaced(generation));
        self.move_windows(control, generation);
    }

    fn move_windows(&self, control: &Control, generation: u64) {
        let mut moved = 0_u64;
        let mut failed = 0_u64;
        let mut first_failure = None;
        let mut waiting = None;
        let mut lost_again = None;
        for slot in self.all_slots() {
            let mut slot = slot.borrow_mut();
            if !slot.lost || slot.closing {
                continue;
            }
            match self.move_window(control, &mut slot) {
                Moved::Drawing => moved += 1,
                Moved::Lost(generation) => lost_again = Some(generation),
                Moved::Waiting(key) => waiting = Some(key),
                Moved::Failed(key) => {
                    failed += 1;
                    first_failure = first_failure.or(Some(key));
                    // Not tried again: the window stays without frames, the others go on.
                    slot.lost = false;
                    self.count(|report| report.windows_unrecovered += 1);
                }
            }
        }
        self.count(|report| report.windows_rebound += moved);
        if let Some(lost) = lost_again {
            // The device that replaced it was lost as well: a new loss, which is bounded like the first.
            match self.recover(|recovery| recovery.lost(Some(lost))) {
                Loss::Begin => {
                    self.count(|report| report.device_losses += 1);
                    self.release_for_loss();
                    self.tell(DeviceEvent::Lost {
                        generation: Some(lost),
                    });
                    self.schedule.borrow_mut().arm(Need::Rebind, Instant::now());
                    return;
                }
                Loss::GaveUp(why) => {
                    self.count(|report| report.device_losses += 1);
                    self.release_for_loss();
                    return self.gave_up(why);
                }
                Loss::Already | Loss::Stale => {}
            }
        }
        if let Some(key) = waiting {
            return self.retry(key);
        }
        if let (Some(key), 0) = (first_failure, moved) {
            // No window can present on the device that is there: the program has no usable device.
            let why = GaveUp::Rebind(key);
            self.recover(|recovery| recovery.give_up(why));
            return self.gave_up(why);
        }
        debug_assert!(failed == 0 || moved > 0);
        self.recover(Recovery::recovered);
        self.tell(DeviceEvent::Rebound {
            generation,
            windows: moved,
        });
    }

    /// Move one window's claim onto the new device, make its surface there and ask for a frame.
    fn move_window(&self, control: &Control, slot: &mut Slot) -> Moved {
        if let Err(refusal) = slot.rebind() {
            return match refusal {
                RebindRefusal::Lost { generation } => Moved::Lost(generation),
                RebindRefusal::InFlight { .. }
                | RebindRefusal::LeaseOutstanding { .. }
                | RebindRefusal::MemoryHeld { .. }
                | RebindRefusal::ResourcesHeld { .. } => Moved::Waiting(refusal.key()),
                other => {
                    self.error(slot, PresentError::Rebind(other));
                    Moved::Failed(other.key())
                }
            };
        }
        let Some(token) = control.native_window(slot.key) else {
            self.error(slot, PresentError::NoToken);
            return Moved::Failed(PresentError::NoToken.code());
        };
        let target = ShellWindowTarget::counted(token, Arc::clone(&self.released));
        match slot.remake_surface(Box::new(target)) {
            Ok(()) => {
                self.count(|report| {
                    report.surfaces_made += 1;
                    report.surfaces_remade += 1;
                });
                let _ = control.request_frame(slot.key);
                Moved::Drawing
            }
            Err(error) => {
                self.error(slot, error);
                Moved::Failed(error.code())
            }
        }
    }
}
