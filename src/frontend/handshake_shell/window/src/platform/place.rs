//! The only place that shows, hides, orders, moves or destroys a window (tests/test_focus_ban.rs).
//!
//! Every call says whether it activates. Only [`show`] with a plan that activates does, and only `policy` makes one.
//! Every other call runs inside the activation guard's `still`, so whatever the operating system or the window library
//! would activate on the way is refused; an activating show names its window with `allowing`.

use std::cell::Cell;
use std::mem;

use windows_sys::Win32::Foundation::{BOOL, FALSE, HWND, LPARAM, RECT, TRUE};
use windows_sys::Win32::System::Threading::GetCurrentThreadId;
use windows_sys::Win32::UI::HiDpi::{AdjustWindowRectExForDpi, GetDpiForWindow};
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{GetActiveWindow, IsWindowEnabled};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    DestroyWindow, EnumThreadWindows, GetAncestor, GetClientRect, GetForegroundWindow,
    GetTopWindow, GetWindow, GetWindowLongPtrW, GetWindowRect, GetWindowThreadProcessId, IsIconic,
    IsWindow, IsWindowVisible, IsZoomed, SetWindowLongPtrW, SetWindowPos, ShowWindow,
    GA_ROOTOWNER, GWL_EXSTYLE, GWL_STYLE, GW_HWNDNEXT, GW_OWNER, HWND_BOTTOM, HWND_TOP,
    SET_WINDOW_POS_FLAGS,
    SHOW_WINDOW_CMD, SWP_FRAMECHANGED, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOOWNERZORDER, SWP_NOSIZE,
    SWP_NOZORDER, SWP_SHOWWINDOW, SW_HIDE, SW_MINIMIZE, SW_RESTORE, SW_SHOW, SW_SHOWMINNOACTIVE,
    SW_SHOWNOACTIVATE, WINDOWPOS, WS_EX_APPWINDOW, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW,
    WS_EX_TOPMOST,
};

use super::{guard, last_error};
use crate::command::{Refusal, WindowClass};
use crate::event::Display;
use crate::geometry::{PhysicalPosition, PhysicalRect, PhysicalSize, ScaleFactor};
use crate::policy::{self, Foreground, MinimizePlan, ShowPlan, ZPlace};

/// Never activates, never changes the order of owner windows.
const QUIETLY: SET_WINDOW_POS_FLAGS = SWP_NOACTIVATE | SWP_NOOWNERZORDER;
const IN_PLACE: SET_WINDOW_POS_FLAGS = SWP_NOMOVE | SWP_NOSIZE;

/// The longest owner chain followed.
const MAX_OWNERS: usize = 32;

/// The most top-level windows walked down the Z order.
const MAX_Z_ORDER: usize = 10_000;

/// A window as [`show`] and [`keep_quiet`] see it.
pub(super) struct Target<'a> {
    pub(super) hwnd: HWND,
    pub(super) class: WindowClass,
    /// Where the window goes in the Z order while a show is carried out.
    pub(super) order_during_show: &'a Cell<Option<HWND>>,
}

fn set_position(
    hwnd: HWND,
    after: HWND,
    position: PhysicalPosition,
    size: PhysicalSize,
    flags: SET_WINDOW_POS_FLAGS,
) -> Result<(), Refusal> {
    let extent = |pixels: u32| i32::try_from(pixels).unwrap_or(i32::MAX);
    // SAFETY: plain values; a window that is gone makes the call fail.
    let done = unsafe {
        SetWindowPos(
            hwnd,
            after,
            position.x,
            position.y,
            extent(size.width),
            extent(size.height),
            flags,
        )
    };
    if done == 0 {
        Err(Refusal::System(last_error()))
    } else {
        Ok(())
    }
}

fn set_order(hwnd: HWND, after: HWND, flags: SET_WINDOW_POS_FLAGS) -> Result<(), Refusal> {
    set_position(
        hwnd,
        after,
        PhysicalPosition::default(),
        PhysicalSize::default(),
        IN_PLACE | flags,
    )
}

fn show_command(hwnd: HWND, command: SHOW_WINDOW_CMD) {
    // SAFETY: plain values. The result is the earlier visibility, not success.
    unsafe { ShowWindow(hwnd, command) };
}

/// A show ends a minimised state, and with `to_normal` a maximised one: the window gets another size and position.
pub(super) fn changes_size(hwnd: HWND, to_normal: bool) -> bool {
    // SAFETY: plain values, read only. Also true of a hidden window.
    unsafe { IsIconic(hwnd) != 0 || (to_normal && IsZoomed(hwnd) != 0) }
}

/// Show a window as planned; `to_normal` also ends minimised and maximised.
pub(super) fn show(target: &Target<'_>, plan: ShowPlan, to_normal: bool) -> Result<(), Refusal> {
    let hwnd = target.hwnd;
    let leaves_size = changes_size(hwnd, to_normal);
    if plan.activate {
        return guard::allowing(hwnd, || {
            if leaves_size {
                show_command(hwnd, SW_RESTORE);
            } else {
                show_command(hwnd, SW_SHOW);
            }
            // The show commands do nothing to a window that is already shown.
            set_order(hwnd, HWND_TOP, 0)
        });
    }
    guard::still(|| {
        let after = insert_after(hwnd, plan.place);
        // In force during the calls below, so the window is at no moment above its place (see `keep_quiet`).
        target.order_during_show.set(after);
        if leaves_size {
            show_command(hwnd, SW_SHOWNOACTIVATE);
        }
        let order = after.map_or(SWP_NOZORDER, |_| 0);
        let done = set_order(
            hwnd,
            after.unwrap_or(HWND_TOP),
            QUIETLY | SWP_SHOWWINDOW | order,
        );
        target.order_during_show.set(None);
        done
    })
}

/// Right behind the foreground window is behind the whole owner chain it belongs to: an owned window is always above
/// its owner, so a window put between them would cover the owner.
fn insert_after(hwnd: HWND, place: ZPlace) -> Option<HWND> {
    match place {
        ZPlace::Keep => None,
        ZPlace::Bottom => Some(HWND_BOTTOM),
        ZPlace::BehindForeground => {
            let foreground = foreground_window();
            let below = root_owner(foreground);
            let usable = foreground != 0
                && below != hwnd
                && !owns(hwnd, foreground)
                && !is_topmost(foreground)
                && !is_topmost(below);
            Some(if usable { below } else { HWND_BOTTOM })
        }
    }
}

pub(super) fn hide(hwnd: HWND) {
    guard::still(|| show_command(hwnd, SW_HIDE));
}

/// Neither plan hands the activation to another window of this thread: the guard refuses it inside `still`. With
/// [`MinimizePlan::ActivateNext`] the operating system may still activate another program's window.
pub(super) fn minimize(hwnd: HWND, plan: MinimizePlan) {
    let command = match plan {
        MinimizePlan::ActivateNext => SW_MINIMIZE,
        MinimizePlan::ActivateNothing => SW_SHOWMINNOACTIVE,
    };
    guard::still(|| show_command(hwnd, command));
}

pub(super) fn move_to(hwnd: HWND, position: PhysicalPosition) -> Result<(), Refusal> {
    guard::still(|| {
        set_position(
            hwnd,
            HWND_TOP,
            position,
            PhysicalSize::default(),
            QUIETLY | SWP_NOSIZE | SWP_NOZORDER,
        )
    })
}

/// Give the client area this size. The frame around it depends on the style and the scale.
pub(super) fn resize(hwnd: HWND, inner: PhysicalSize) -> Result<(), Refusal> {
    let extent = |pixels: u32| i32::try_from(pixels).map_err(|_| Refusal::InvalidGeometry);
    let mut frame = RECT {
        left: 0,
        top: 0,
        right: extent(inner.width)?,
        bottom: extent(inner.height)?,
    };
    // SAFETY: plain values and a rectangle that outlives the call. Styles are 32 bits wide.
    let adjusted = unsafe {
        AdjustWindowRectExForDpi(
            &mut frame,
            GetWindowLongPtrW(hwnd, GWL_STYLE) as u32,
            FALSE,
            GetWindowLongPtrW(hwnd, GWL_EXSTYLE) as u32,
            GetDpiForWindow(hwnd),
        )
    };
    if adjusted == 0 {
        return Err(Refusal::System(last_error()));
    }
    guard::still(|| {
        set_position(
            hwnd,
            HWND_TOP,
            PhysicalPosition::default(),
            rect(frame).size,
            QUIETLY | SWP_NOMOVE | SWP_NOZORDER,
        )
    })
}

/// Put every window this thread has so far below all others, and keep it from being activated. The window library
/// makes one for itself, without any extent but visible, and new windows come in on top.
pub(super) fn lower_own_windows() {
    // SAFETY: the callback uses nothing but the window it is given.
    guard::still(|| unsafe { EnumThreadWindows(GetCurrentThreadId(), Some(lower), 0) });
}

unsafe extern "system" fn lower(hwnd: HWND, _: LPARAM) -> BOOL {
    guard::mark_quiet(hwnd);
    // A window that cannot be moved stays where it is; it has no extent.
    let _ = set_order(hwnd, HWND_BOTTOM, QUIETLY);
    TRUE
}

/// Returns whether it was destroyed. Destroying the active window activates no other window of this thread.
pub(super) fn destroy(hwnd: HWND) -> bool {
    // SAFETY: a plain value; fails for a window that is gone or not of this thread.
    guard::still(|| unsafe { DestroyWindow(hwnd) != 0 })
}

/// What a window was like before it was minimised, hidden or destroyed, for [`hand_on`].
pub(super) struct Before {
    removed: HWND,
    held_activation: bool,
    owner: HWND,
}

pub(super) fn before_removal(hwnd: HWND) -> Before {
    // SAFETY: plain values, read only; 0 where there is none.
    let (active, owner) = unsafe { (GetActiveWindow(), GetWindow(hwnd, GW_OWNER)) };
    Before {
        removed: hwnd,
        held_activation: active != 0 && (active == hwnd || owns(hwnd, active)),
        owner,
    }
}

/// Carry out [`policy::activation_after_removal`] once the removal is done, among `operators`: the windows of the
/// Operator's class of this loop. Returns whether one was activated.
///
/// The activation went nowhere when this thread has no active window left, or only one that is gone, hidden or
/// minimised; a dialog the removed window owns that is still shown keeps it.
pub(super) fn hand_on(before: &Before, approved: bool, operators: &[HWND]) -> bool {
    let removed = before.removed;
    let shown = |hwnd: HWND| {
        // SAFETY: plain values, read only; false for a window that is gone.
        unsafe { IsWindow(hwnd) != 0 && IsWindowVisible(hwnd) != 0 && IsIconic(hwnd) == 0 }
    };
    let usable = |hwnd: HWND| {
        hwnd != removed
            && operators.contains(&hwnd)
            && shown(hwnd)
            // SAFETY: a plain value, read only.
            && unsafe { IsWindowEnabled(hwnd) } != 0
    };
    // SAFETY: no arguments, read only.
    let (active, foreground) = unsafe { (GetActiveWindow(), GetForegroundWindow()) };
    let lost = before.held_activation && (active == 0 || active == removed || !shown(active));
    let other_program_in_front = foreground != 0 && !belongs_to_us(foreground);
    let owner = Some(before.owner).filter(|owner| *owner != 0 && usable(*owner));
    let next = policy::activation_after_removal(
        approved,
        lost,
        other_program_in_front,
        owner,
        top_down(operators).into_iter().filter(|hwnd| usable(*hwnd)),
    );
    let Some(next) = next else {
        return false;
    };
    // As an activating show does it: to the top, and only this window may be activated meanwhile.
    let done = guard::allowing(next, || set_order(next, HWND_TOP, 0));
    // SAFETY: no arguments, read only.
    done.is_ok() && unsafe { GetActiveWindow() } == next
}

/// `windows` as they are in the Z order, topmost first.
fn top_down(windows: &[HWND]) -> Vec<HWND> {
    let mut found = Vec::new();
    // SAFETY: plain values, read only: the top-level windows of this thread's desktop, from the top.
    let mut current = unsafe { GetTopWindow(0) };
    for _ in 0..MAX_Z_ORDER {
        if current == 0 || found.len() == windows.len() {
            break;
        }
        if windows.contains(&current) {
            found.push(current);
        }
        // SAFETY: as above; 0 below the last.
        current = unsafe { GetWindow(current, GW_HWNDNEXT) };
    }
    found
}

/// The window still exists.
pub(super) fn exists(hwnd: HWND) -> bool {
    // SAFETY: a plain value, read only.
    unsafe { IsWindow(hwnd) != 0 }
}

/// Size and position are the window's own: neither minimised nor maximised, also while hidden.
pub(super) fn is_normal_size(hwnd: HWND) -> bool {
    // SAFETY: plain values, read only.
    unsafe { IsIconic(hwnd) == 0 && IsZoomed(hwnd) == 0 }
}

/// Refuse unless size and position are the window's own.
pub(super) fn at_normal_size(hwnd: HWND) -> Result<(), Refusal> {
    if is_normal_size(hwnd) {
        Ok(())
    } else {
        Err(Refusal::NotAtNormalSize)
    }
}

pub(super) fn display(hwnd: HWND) -> Display {
    // SAFETY: plain values, read only.
    unsafe {
        if IsWindowVisible(hwnd) == 0 {
            Display::Hidden
        } else if IsIconic(hwnd) != 0 {
            Display::Minimized
        } else if IsZoomed(hwnd) != 0 {
            Display::Maximized
        } else {
            Display::Normal
        }
    }
}

pub(super) fn outer(hwnd: HWND) -> PhysicalRect {
    let mut area = empty();
    // SAFETY: the rectangle outlives the call; it stays empty when the window is gone.
    unsafe { GetWindowRect(hwnd, &mut area) };
    rect(area)
}

pub(super) fn inner(hwnd: HWND) -> PhysicalSize {
    let mut area = empty();
    // SAFETY: as in `outer`.
    unsafe { GetClientRect(hwnd, &mut area) };
    rect(area).size
}

pub(super) fn scale(hwnd: HWND) -> ScaleFactor {
    // SAFETY: a plain value; 0 when the window is gone.
    ScaleFactor::from_dpi(unsafe { GetDpiForWindow(hwnd) }).unwrap_or(ScaleFactor::ONE)
}

/// The foreground window. Where the desktop reports none (one that receives no input, or a moment in which the
/// activation moves), the active window of this thread stands for it, so a window of ours that is active is treated
/// as the one the Operator works in.
fn foreground_window() -> HWND {
    // SAFETY: no arguments, read only, both.
    unsafe {
        match GetForegroundWindow() {
            0 => GetActiveWindow(),
            foreground => foreground,
        }
    }
}

/// What is in the foreground, seen from `hwnd`. A window `hwnd` owns counts as `hwnd`: a dialog of the Operator's
/// window is where the Operator works.
pub(super) fn foreground(hwnd: HWND) -> Foreground {
    let foreground = foreground_window();
    if foreground == 0 {
        return Foreground::Nothing;
    }
    if foreground == hwnd || owns(hwnd, foreground) {
        return Foreground::This;
    }
    let root = root_owner(foreground);
    if belongs_to_us(foreground) || belongs_to_us(root) {
        Foreground::OurOther
    } else {
        Foreground::Other {
            topmost: is_topmost(foreground) || is_topmost(root),
        }
    }
}

/// `owner` is somewhere up the owner chain of `hwnd`.
fn owns(owner: HWND, hwnd: HWND) -> bool {
    let mut current = hwnd;
    for _ in 0..MAX_OWNERS {
        // SAFETY: plain values, read only; 0 at the end of the chain.
        current = unsafe { GetWindow(current, GW_OWNER) };
        if current == 0 {
            return false;
        }
        if current == owner {
            return true;
        }
    }
    false
}

/// The window at the end of the owner chain; the window itself without one.
fn root_owner(hwnd: HWND) -> HWND {
    // SAFETY: plain values, read only.
    match unsafe { GetAncestor(hwnd, GA_ROOTOWNER) } {
        0 => hwnd,
        root => root,
    }
}

fn belongs_to_us(hwnd: HWND) -> bool {
    let mut process = 0;
    // SAFETY: the number outlives the call; it stays 0 when the window is gone.
    unsafe { GetWindowThreadProcessId(hwnd, &mut process) };
    process == std::process::id()
}

fn is_topmost(hwnd: HWND) -> bool {
    // SAFETY: plain values, read only.
    unsafe { GetWindowLongPtrW(hwnd, GWL_EXSTYLE) as u32 & WS_EX_TOPMOST != 0 }
}

/// The extended style a window of this class has, whatever was asked for.
pub(super) fn class_style(class: WindowClass, style: u32) -> u32 {
    match class {
        WindowClass::Operator => style,
        WindowClass::Quiet => {
            (style & !(WS_EX_APPWINDOW | WS_EX_TOPMOST)) | WS_EX_NOACTIVATE | WS_EX_TOOLWINDOW
        }
    }
}

/// Give a new window its class. It is still hidden.
pub(super) fn apply_class(target: &Target<'_>) -> Result<(), Refusal> {
    if target.class == WindowClass::Operator {
        return Ok(());
    }
    guard::still(|| {
        // SAFETY: plain values. The window procedure hook keeps the class style in whatever is set.
        unsafe {
            let style = GetWindowLongPtrW(target.hwnd, GWL_EXSTYLE) as u32;
            SetWindowLongPtrW(
                target.hwnd,
                GWL_EXSTYLE,
                class_style(target.class, style) as isize,
            );
        }
        set_order(target.hwnd, HWND_BOTTOM, QUIETLY | SWP_FRAMECHANGED)
    })
}

/// Keep a position change, whoever makes it, within the window's class and the show that is being carried out. It
/// keeps the Z order; activation is the guard's.
pub(super) fn keep_quiet(target: &Target<'_>, change: &mut WINDOWPOS) {
    let quiet = target.class == WindowClass::Quiet;
    let during_show = target.order_during_show.get();
    if quiet || during_show.is_some() {
        change.flags |= SWP_NOACTIVATE;
    }
    let after = during_show.or(quiet.then_some(HWND_BOTTOM));
    let orders = change.flags & SWP_NOZORDER == 0 || change.flags & SWP_SHOWWINDOW != 0;
    if let (Some(after), true) = (after, orders) {
        change.hwndInsertAfter = after;
        change.flags &= !SWP_NOZORDER;
    }
}

fn empty() -> RECT {
    // SAFETY: four integers.
    unsafe { mem::zeroed() }
}

pub(super) fn rect(area: RECT) -> PhysicalRect {
    let extent = |from: i32, to: i32| u32::try_from(i64::from(to) - i64::from(from)).unwrap_or(0);
    PhysicalRect::new(
        area.left,
        area.top,
        extent(area.left, area.right),
        extent(area.top, area.bottom),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_quiet_window_keeps_its_style_whatever_is_asked_for() {
        let style = class_style(WindowClass::Quiet, WS_EX_APPWINDOW | WS_EX_TOPMOST | 0x10);
        assert_eq!(style & WS_EX_APPWINDOW, 0);
        assert_eq!(style & WS_EX_TOPMOST, 0);
        assert_eq!(style & WS_EX_NOACTIVATE, WS_EX_NOACTIVATE);
        assert_eq!(style & WS_EX_TOOLWINDOW, WS_EX_TOOLWINDOW);
        assert_eq!(style & 0x10, 0x10, "other bits are left alone");
        assert_eq!(
            class_style(WindowClass::Operator, WS_EX_APPWINDOW),
            WS_EX_APPWINDOW
        );
    }

    #[test]
    fn a_rectangle_with_crossed_edges_is_empty() {
        let crossed = RECT {
            left: 10,
            top: 10,
            right: 5,
            bottom: 20,
        };
        assert_eq!(rect(crossed).size, PhysicalSize::new(0, 10));
        let widest = RECT {
            left: i32::MIN,
            top: 0,
            right: i32::MAX,
            bottom: 1,
        };
        assert_eq!(rect(widest).size, PhysicalSize::new(u32::MAX, 1));
    }
}
