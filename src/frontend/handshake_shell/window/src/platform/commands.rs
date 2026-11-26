//! What the host's commands do to a window: the policy decides, `place` carries it out.
//!
//! The Operator's authority is used up by the command that needed it (MT-214 W4): an activation, a minimise that hands
//! the activation on, or a clipboard write. A model command may not move, size, restore, hide, minimise or destroy the
//! window the Operator works in, nor show it back from minimised (R2-N1); a request to close it lets a model remove it
//! once, while the request is open. After a removal the Operator approved, the keyboard goes where
//! `policy::activation_after_removal` says (R2-N5).

use super::loop_core::Core;
use super::{monitors, place};
use crate::clipboard::ClipboardError;
use crate::command::{Refusal, WindowCommand};
use crate::event::WindowKey;
use crate::frame::{FrameOutcome, FrameProgress, FrameTicket};
use crate::input::WheelSettings;
use crate::monitor::{carried, Monitor};
use crate::origin::{Authority, CommandSource};
use crate::policy::{self, Foreground, MinimizePlan};

/// What `Control` offers for one window.
impl Core {
    pub(crate) fn command(
        &self,
        source: &CommandSource,
        window: WindowKey,
        command: WindowCommand,
    ) -> Result<(), Refusal> {
        let outcome = carry_out(self, source, window, command);
        self.counted(outcome)
    }

    pub(crate) fn monitors(&self) -> Vec<Monitor> {
        monitors::all()
    }

    pub(crate) fn wheel_settings(&self) -> WheelSettings {
        monitors::wheel_settings()
    }

    pub(crate) fn request_frame(&self, key: WindowKey) -> Result<(), Refusal> {
        let window = self.window_by_key(key).ok_or(Refusal::UnknownWindow)?;
        if window.frames.borrow_mut().request(self.now().nanos) {
            window.ask_for_paint();
        }
        Ok(())
    }

    pub(crate) fn frame_done(
        &self,
        key: WindowKey,
        ticket: FrameTicket,
        outcome: FrameOutcome,
    ) -> Result<(), Refusal> {
        let window = self.window_by_key(key).ok_or(Refusal::UnknownWindow)?;
        let again = window
            .frames
            .borrow_mut()
            .answer(ticket, outcome)
            .ok_or(Refusal::UnknownFrame)?;
        if again {
            window.ask_for_paint();
        }
        Ok(())
    }

    pub(crate) fn frame_progress(&self, key: WindowKey) -> Option<FrameProgress> {
        let window = self.window_by_key(key)?;
        let progress = window
            .frames
            .borrow()
            .progress(self.now().nanos, window.can_show());
        Some(progress)
    }
}

fn carry_out(
    core: &Core,
    source: &CommandSource,
    key: WindowKey,
    command: WindowCommand,
) -> Result<(), Refusal> {
    let window = core.window_by_key(key).ok_or(Refusal::UnknownWindow)?;
    let authority = core.authority_of(source);
    let hwnd = window.hwnd;
    let foreground = place::foreground(hwnd);
    let geometry = || policy::may_change_geometry(authority, foreground);
    match command {
        WindowCommand::Show { activate } | WindowCommand::Restore { activate } => {
            let to_normal = matches!(command, WindowCommand::Restore { .. });
            // A restore is geometry, and so is a show that brings a minimised window back (MT-214 R2-N1).
            let resizes = to_normal || place::changes_size(hwnd, false);
            let plan = policy::plan_show(window.class, activate, authority, foreground, resizes)?;
            if plan.activate && authority.is_operators() {
                core.use_up(source);
            }
            // Shown again: a close request from before is over.
            window.set_close_requested(None);
            place::show(&window.target(), plan, to_normal)
        }
        WindowCommand::Hide => {
            let approved = remove(core, &window, authority, foreground)?;
            let before = place::before_removal(hwnd);
            place::hide(hwnd);
            hand_on(core, source, authority, &before, approved);
            Ok(())
        }
        WindowCommand::Minimize => {
            let plan = policy::plan_minimize(window.class, authority, foreground)?;
            let approved = plan == MinimizePlan::ActivateNext;
            if approved && authority.is_operators() {
                core.use_up(source);
            }
            let before = place::before_removal(hwnd);
            place::minimize(hwnd, plan);
            hand_on(core, source, authority, &before, approved);
            Ok(())
        }
        WindowCommand::SetInnerSize(size) => {
            geometry()?;
            if !size.is_usable() {
                return Err(Refusal::InvalidGeometry);
            }
            place::at_normal_size(hwnd)?;
            let wanted = size.to_physical(window.scale.get());
            place::resize(hwnd, wanted)?;
            if place::inner(hwnd) == wanted {
                // Exactly what was asked for, and not what its whole pixels come back to.
                window.logical.set(size);
            }
            Ok(())
        }
        WindowCommand::MoveTo(position) => {
            geometry()?;
            place::at_normal_size(hwnd)?;
            place::move_to(hwnd, position)
        }
        WindowCommand::MoveToMonitor(target) => {
            geometry()?;
            place::at_normal_size(hwnd)?;
            let monitors = monitors::all();
            let find = |key| monitors.iter().find(|monitor| monitor.key == key);
            let to = find(target).ok_or(Refusal::UnknownMonitor)?;
            let outer = place::outer(hwnd);
            let position = match monitors::of_window(hwnd).and_then(find) {
                Some(from) => carried(outer, from, to),
                None => crate::monitor::centered(outer.size, to.work_area),
            };
            place::move_to(hwnd, position)
        }
        WindowCommand::SetCursor(shape) => {
            window.set_cursor(shape);
            Ok(())
        }
        WindowCommand::SetTitle(title) => {
            window.set_title(&title);
            Ok(())
        }
        WindowCommand::Destroy => {
            if window.tokens() > 0 {
                return Err(Refusal::NativeTokensHeld);
            }
            let approved = remove(core, &window, authority, foreground)?;
            let before = place::before_removal(hwnd);
            if !place::destroy(hwnd) {
                return Err(Refusal::System(super::last_error()));
            }
            hand_on(core, source, authority, &before, approved);
            Ok(())
        }
    }
}

/// Whether the window may be hidden or destroyed, and whether the Operator approved it: the Operator's own command, or
/// an open request to close it. A model that removes the Operator's window on a close request uses the request up.
fn remove(
    core: &Core,
    window: &super::window::Window,
    authority: Authority,
    foreground: Foreground,
) -> Result<bool, Refusal> {
    let open = window.close_open(core.now());
    policy::may_remove(authority, foreground, open)?;
    if authority == Authority::Model && foreground == Foreground::This {
        window.set_close_requested(None);
    }
    Ok(authority == Authority::Operator || open)
}

/// ACTIVATION_AFTER_REMOVAL (`policy::activation_after_removal`, MT-214 R2-N5). An activation on the Operator's
/// authority uses it up.
fn hand_on(
    core: &Core,
    source: &CommandSource,
    authority: Authority,
    before: &place::Before,
    approved: bool,
) {
    if place::hand_on(before, approved, &core.operator_windows()) && authority.is_operators() {
        core.use_up(source);
    }
}

/// Whether `source` may replace the clipboard now; using the Operator's authority for it uses it up.
pub(super) fn may_write_clipboard(core: &Core, source: &CommandSource) -> Result<(), ClipboardError> {
    let authority = core.authority_of(source);
    let foreground = place::foreground(core.service());
    if policy::may_write_clipboard(authority, foreground).is_err() {
        return Err(ClipboardError::NeedsOperator);
    }
    if authority.is_operators() {
        core.use_up(source);
    }
    Ok(())
}
