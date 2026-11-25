//! The only place that shows, hides, orders, moves or destroys a window (tests/test_focus_ban.rs).
//!
//! Every call says whether it activates. Only [`show`] with a plan that activates does, and only `policy` makes one.

use std::mem;

use windows_sys::Win32::Foundation::{BOOL, FALSE, HWND, LPARAM, RECT, TRUE};
use windows_sys::Win32::System::Threading::GetCurrentThreadId;
use windows_sys::Win32::UI::HiDpi::{AdjustWindowRectExForDpi, GetDpiForWindow};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    DestroyWindow, EnumThreadWindows, GetClientRect, GetForegroundWindow, GetWindowLongPtrW,
    GetWindowRect, GetWindowThreadProcessId, IsIconic, IsWindowVisible, IsZoomed,
    SetWindowLongPtrW, SetWindowPos, ShowWindow, GWL_EXSTYLE, GWL_STYLE, HWND_BOTTOM, HWND_TOP,
    SET_WINDOW_POS_FLAGS, SHOW_WINDOW_CMD, SWP_FRAMECHANGED, SWP_NOACTIVATE, SWP_NOMOVE,
    SWP_NOOWNERZORDER, SWP_NOSIZE, SWP_NOZORDER, SWP_SHOWWINDOW, SW_HIDE, SW_MINIMIZE, SW_RESTORE,
    SW_SHOW, SW_SHOWMINNOACTIVE, SW_SHOWNOACTIVATE, WINDOWPOS, WS_EX_APPWINDOW, WS_EX_NOACTIVATE,
    WS_EX_TOOLWINDOW, WS_EX_TOPMOST,
};

use super::last_error;
use super::window::Window;
use crate::command::{Refusal, WindowClass};
use crate::event::Display;
use crate::geometry::{PhysicalPosition, PhysicalRect, PhysicalSize, ScaleFactor};
use crate::policy::{Foreground, MinimizePlan, ShowPlan, ZPlace};

/// Never activates, never changes the order of owner windows.
const QUIETLY: SET_WINDOW_POS_FLAGS = SWP_NOACTIVATE | SWP_NOOWNERZORDER;
const IN_PLACE: SET_WINDOW_POS_FLAGS = SWP_NOMOVE | SWP_NOSIZE;

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

/// Show a window as planned; `to_normal` also ends minimised and maximised.
pub(super) fn show(window: &Window, plan: ShowPlan, to_normal: bool) -> Result<(), Refusal> {
    let hwnd = window.hwnd;
    // SAFETY: plain values, read only. Also true of a hidden window.
    let leaves_size = unsafe { IsIconic(hwnd) != 0 || (to_normal && IsZoomed(hwnd) != 0) };
    if plan.activate {
        if leaves_size {
            show_command(hwnd, SW_RESTORE);
        } else {
            show_command(hwnd, SW_SHOW);
        }
        // The show commands do nothing to a window that is already shown.
        return set_order(hwnd, HWND_TOP, 0);
    }

    let after = insert_after(hwnd, plan.place);
    // In force during the calls below, so the window is at no moment above its place (see `keep_quiet`).
    window.order_during_show.set(after);
    if leaves_size {
        show_command(hwnd, SW_SHOWNOACTIVATE);
    }
    let order = after.map_or(SWP_NOZORDER, |_| 0);
    let done = set_order(
        hwnd,
        after.unwrap_or(HWND_TOP),
        QUIETLY | SWP_SHOWWINDOW | order,
    );
    window.order_during_show.set(None);
    done
}

fn insert_after(hwnd: HWND, place: ZPlace) -> Option<HWND> {
    match place {
        ZPlace::Keep => None,
        ZPlace::Bottom => Some(HWND_BOTTOM),
        ZPlace::BehindForeground => {
            // SAFETY: no arguments.
            let foreground = unsafe { GetForegroundWindow() };
            let usable = foreground != 0 && foreground != hwnd && !is_topmost(foreground);
            Some(if usable { foreground } else { HWND_BOTTOM })
        }
    }
}

pub(super) fn hide(hwnd: HWND) {
    show_command(hwnd, SW_HIDE);
}

pub(super) fn minimize(hwnd: HWND, plan: MinimizePlan) {
    show_command(
        hwnd,
        match plan {
            MinimizePlan::ActivateNext => SW_MINIMIZE,
            MinimizePlan::ActivateNothing => SW_SHOWMINNOACTIVE,
        },
    );
}

pub(super) fn move_to(hwnd: HWND, position: PhysicalPosition) -> Result<(), Refusal> {
    set_position(
        hwnd,
        HWND_TOP,
        position,
        PhysicalSize::default(),
        QUIETLY | SWP_NOSIZE | SWP_NOZORDER,
    )
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
    set_position(
        hwnd,
        HWND_TOP,
        PhysicalPosition::default(),
        rect(frame).size,
        QUIETLY | SWP_NOMOVE | SWP_NOZORDER,
    )
}

/// Put every window this thread has so far below all others. The window library makes one for itself, without any
/// extent but visible, and new windows come in on top.
pub(super) fn lower_own_windows() {
    // SAFETY: the callback uses nothing but the window it is given.
    unsafe { EnumThreadWindows(GetCurrentThreadId(), Some(lower), 0) };
}

unsafe extern "system" fn lower(hwnd: HWND, _: LPARAM) -> BOOL {
    // A window that cannot be moved stays where it is; it has no extent.
    let _ = set_order(hwnd, HWND_BOTTOM, QUIETLY);
    TRUE
}

/// Returns whether it was destroyed.
pub(super) fn destroy(hwnd: HWND) -> bool {
    // SAFETY: a plain value; fails for a window that is gone or not of this thread.
    unsafe { DestroyWindow(hwnd) != 0 }
}

/// Refuse unless size and position are the window's own.
pub(super) fn at_normal_size(hwnd: HWND) -> Result<(), Refusal> {
    // SAFETY: plain values, read only.
    if unsafe { IsIconic(hwnd) != 0 || IsZoomed(hwnd) != 0 } {
        Err(Refusal::NotAtNormalSize)
    } else {
        Ok(())
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

/// What is in the foreground, seen from `hwnd`.
pub(super) fn foreground(hwnd: HWND) -> Foreground {
    // SAFETY: no arguments, read only.
    let foreground = unsafe { GetForegroundWindow() };
    if foreground == 0 {
        Foreground::Nothing
    } else if foreground == hwnd {
        Foreground::This
    } else if belongs_to_us(foreground) {
        Foreground::OurOther
    } else {
        Foreground::Other {
            topmost: is_topmost(foreground),
        }
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
        WindowClass::Quiet => (style & !WS_EX_APPWINDOW) | WS_EX_NOACTIVATE | WS_EX_TOOLWINDOW,
    }
}

/// Give a new window its class. It is still hidden.
pub(super) fn apply_class(window: &Window) -> Result<(), Refusal> {
    if window.class == WindowClass::Operator {
        return Ok(());
    }
    // SAFETY: plain values. The window procedure hook keeps the class style in whatever is set.
    unsafe {
        let style = GetWindowLongPtrW(window.hwnd, GWL_EXSTYLE) as u32;
        SetWindowLongPtrW(
            window.hwnd,
            GWL_EXSTYLE,
            class_style(window.class, style) as isize,
        );
    }
    set_order(window.hwnd, HWND_BOTTOM, QUIETLY | SWP_FRAMECHANGED)
}

/// Keep a position change, whoever makes it, within the window's class and the show that is being carried out.
pub(super) fn keep_quiet(window: &Window, change: &mut WINDOWPOS) {
    let quiet = window.class == WindowClass::Quiet;
    let during_show = window.order_during_show.get();
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
        let style = class_style(WindowClass::Quiet, WS_EX_APPWINDOW | 0x10);
        assert_eq!(style & WS_EX_APPWINDOW, 0);
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
