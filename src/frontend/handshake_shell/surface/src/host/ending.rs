//! The end of windows and of the loop: closing a window, settling the claims of windows that are gone, and shutdown.

use std::cell::RefCell;
use std::rc::Rc;
use std::time::{Duration, Instant};

use handshake_shell_window::{Control, Refusal, WindowCommand, WindowKey};

use super::Core;
use crate::decide::{self, Need};
use crate::error::PresentError;
use crate::slot::{Settled, Slot};

impl Core {
    /// Settle the claims of windows that are gone. Returns whether one is still waiting.
    pub(super) fn settle_ending(&self) -> bool {
        let mut ending = self.ending.borrow_mut();
        ending.retain_mut(|slot| match slot.settle() {
            Settled::Now => {
                self.count(|report| report.claims_settled += 1);
                false
            }
            Settled::Nothing => false,
            Settled::Waiting => true,
            Settled::Refused(refusal) => {
                self.error(slot, PresentError::Settle(refusal));
                false
            }
        });
        !ending.is_empty()
    }

    /// One step of closing a window: settle its claim, and destroy it once its target is let go. Returns whether more
    /// is to come.
    pub(super) fn close_step(&self, control: &Control, slot: &mut Slot) -> bool {
        let mut more = false;
        match slot.settle() {
            Settled::Now => self.count(|report| report.claims_settled += 1),
            Settled::Nothing => {}
            Settled::Waiting => more = true,
            Settled::Refused(refusal) => self.error(slot, PresentError::Settle(refusal)),
        }
        if !slot.destroy_asked {
            let tokens = control
                .window_state(slot.key)
                .map_or(0, |state| state.native_tokens);
            if tokens == 0
                && control
                    .command(&self.source(), slot.key, WindowCommand::Destroy)
                    .is_ok()
            {
                slot.destroy_asked = true;
            } else {
                more = true;
            }
        }
        more
    }

    pub(crate) fn close_window(&self, control: &Control, key: WindowKey) -> Result<(), Refusal> {
        let Some(slot) = self.slot(key) else {
            return control.command(&self.source(), key, WindowCommand::Destroy);
        };
        let mut slot = slot.borrow_mut();
        if slot.closing {
            return Ok(());
        }
        slot.closing = true;
        if slot.release_surface() {
            self.count(|report| report.surfaces_released += 1);
        }
        self.schedule.borrow_mut().cancel(Need::Frame(key));
        let hidden = control.command(&self.source(), key, WindowCommand::Hide);
        let _ = self.owner.poll();
        if self.close_step(control, &mut slot) {
            self.want_looks();
        }
        hidden
    }

    /// After the application heard `Event::ShutdownBegan`: release every surface, settle the claims, close the owner
    /// and end the loop, now or once the owner let go of the last window target.
    pub(super) fn tear_down(&self, control: &Control) {
        if self.torn_down.replace(true) {
            return;
        }
        let slots = self.all_slots();
        for slot in &slots {
            let mut slot = slot.borrow_mut();
            if slot.release_surface() {
                self.count(|report| report.surfaces_released += 1);
            }
        }
        let _ = self.owner.poll();
        self.settle_all(&slots);
        self.settle_ending();
        // Bounded: the owner waits at most its own deadline for the queue (MT-203 `close`).
        let shutdown = self.owner.close();
        self.settle_all(&slots);
        self.settle_ending();
        let ending = std::mem::take(&mut *self.ending.borrow_mut());
        let abandoned = slots
            .iter()
            .filter(|slot| slot.borrow_mut().abandon_claim())
            .count()
            + ending.len();
        drop(ending);
        self.count(|report| report.claims_abandoned += abandoned as u64);
        self.count(|report| report.shutdown = Some(shutdown));
        // The clients go with the slots, before the loop ends.
        self.slots.borrow_mut().clear();
        drop(slots);
        self.schedule.borrow_mut().cancel(Need::Poll);
        self.try_finish(control);
    }

    pub(super) fn settle_all(&self, slots: &[Rc<RefCell<Slot>>]) {
        for slot in slots {
            let mut slot = slot.borrow_mut();
            match slot.settle() {
                Settled::Now => self.count(|report| report.claims_settled += 1),
                Settled::Refused(refusal) => self.error(&mut slot, PresentError::Settle(refusal)),
                Settled::Nothing | Settled::Waiting => {}
            }
        }
    }

    pub(super) fn try_finish(&self, control: &Control) {
        let code = self.exit_code.get().unwrap_or(0);
        match control.finish(code) {
            Ok(()) => {}
            Err(Refusal::NativeTokensHeld) => {
                self.count(|report| report.finish_refused += 1);
                // A device that was held or still ran work at the close is destroyed by a later look, with the
                // targets of its surfaces.
                let _ = self.owner.poll();
                let tries = self.finish_tries.get().saturating_add(1);
                self.finish_tries.set(tries);
                let delay = decide::poll_delay(tries).max(Duration::from_millis(10));
                self.schedule
                    .borrow_mut()
                    .arm(Need::Finish, Instant::now() + delay);
            }
            // Shutdown not begun, or the loop has ended: nothing to try again.
            Err(_) => {}
        }
    }
}
