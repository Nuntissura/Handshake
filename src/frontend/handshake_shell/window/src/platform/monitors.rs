//! Monitors and the Operator's wheel settings, read from the operating system.

use std::mem;
use std::ptr;

use windows_sys::Win32::Foundation::{BOOL, HWND, LPARAM, RECT, TRUE};
use windows_sys::Win32::Graphics::Gdi::{
    EnumDisplayMonitors, GetMonitorInfoW, MonitorFromWindow, HDC, HMONITOR, MONITORINFO,
    MONITOR_DEFAULTTONULL,
};
use windows_sys::Win32::UI::HiDpi::{GetDpiForMonitor, MDT_EFFECTIVE_DPI};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    SystemParametersInfoW, MONITORINFOF_PRIMARY, SPI_GETWHEELSCROLLCHARS, SPI_GETWHEELSCROLLLINES,
    SYSTEM_PARAMETERS_INFO_ACTION,
};

use super::place;
use crate::geometry::ScaleFactor;
use crate::input::WheelSettings;
use crate::monitor::{Monitor, MonitorKey};

/// More than any desktop has; the list stays bounded.
const MAX_MONITORS: usize = 64;

/// A whole page per detent (winuser.h `WHEEL_PAGESCROLL`; the binding has no name for it).
const WHEEL_PAGESCROLL: u32 = u32::MAX;

pub(super) fn all() -> Vec<Monitor> {
    let mut found: Vec<HMONITOR> = Vec::new();
    // SAFETY: the callback runs inside the call only, with the list it is given.
    unsafe {
        EnumDisplayMonitors(
            0,
            ptr::null(),
            Some(collect),
            &mut found as *mut Vec<HMONITOR> as LPARAM,
        )
    };
    found.into_iter().filter_map(describe).collect()
}

unsafe extern "system" fn collect(monitor: HMONITOR, _: HDC, _: *mut RECT, list: LPARAM) -> BOOL {
    // SAFETY: the list `all` passed, which outlives the enumeration.
    let list = unsafe { &mut *(list as *mut Vec<HMONITOR>) };
    if list.len() < MAX_MONITORS {
        list.push(monitor);
    }
    TRUE
}

fn describe(monitor: HMONITOR) -> Option<Monitor> {
    let mut info = MONITORINFO {
        cbSize: mem::size_of::<MONITORINFO>() as u32,
        // SAFETY: integers and rectangles.
        ..unsafe { mem::zeroed() }
    };
    let (mut dpi_x, mut dpi_y) = (0, 0);
    // SAFETY: every pointer outlives its call. A monitor that is gone makes the first call fail.
    unsafe {
        if GetMonitorInfoW(monitor, &mut info) == 0 {
            return None;
        }
        // Leaves 0 when it fails, which is no scale.
        GetDpiForMonitor(monitor, MDT_EFFECTIVE_DPI, &mut dpi_x, &mut dpi_y);
    }
    Some(Monitor {
        key: key(monitor),
        bounds: place::rect(info.rcMonitor),
        work_area: place::rect(info.rcWork),
        scale: ScaleFactor::from_dpi(dpi_x).unwrap_or(ScaleFactor::ONE),
        primary: info.dwFlags & MONITORINFOF_PRIMARY != 0,
    })
}

fn key(monitor: HMONITOR) -> MonitorKey {
    MonitorKey(monitor as u64)
}

/// The monitor most of the window is on.
pub(super) fn of_window(hwnd: HWND) -> Option<MonitorKey> {
    // SAFETY: plain values, read only.
    let monitor = unsafe { MonitorFromWindow(hwnd, MONITOR_DEFAULTTONULL) };
    (monitor != 0).then(|| key(monitor))
}

pub(super) fn wheel_settings() -> WheelSettings {
    let lines = setting(SPI_GETWHEELSCROLLLINES).unwrap_or(3);
    WheelSettings {
        lines: (lines != WHEEL_PAGESCROLL).then_some(lines),
        characters: setting(SPI_GETWHEELSCROLLCHARS).unwrap_or(3),
    }
}

fn setting(which: SYSTEM_PARAMETERS_INFO_ACTION) -> Option<u32> {
    let mut value = 0u32;
    // SAFETY: both settings are one u32, and the call only reads.
    let read = unsafe { SystemParametersInfoW(which, 0, (&mut value as *mut u32).cast(), 0) };
    (read != 0).then_some(value)
}
