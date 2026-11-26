//! Windows: the window library and Win32, behind this crate's own types.
//!
//! The only unsafe code of the crate. Nothing in here is public; replacing the window library means replacing
//! `app`, `translate` and the handle inside `window`.
#![allow(unsafe_code)]

mod app;
mod clipboard;
mod commands;
mod drops;
mod guard;
mod key_gate;
mod lanes;
mod loop_core;
mod monitors;
mod place;
#[cfg(test)]
mod quiet_tests;
mod service;
mod translate;
mod wake;
mod window;

pub(crate) use self::app::run;
pub(crate) use self::loop_core::Core;
pub(crate) use self::wake::Lane;

/// The calling thread's last error number.
fn last_error() -> u32 {
    // SAFETY: no arguments.
    unsafe { windows_sys::Win32::Foundation::GetLastError() }
}

/// The message the current window procedure handles was sent from another thread, of this process or another one
/// (InSendMessageEx: anything but `ISMEX_NOSEND`). A message this thread sends is a call, and reports nothing.
fn sent_by_another_thread() -> bool {
    use windows_sys::Win32::UI::WindowsAndMessaging::{InSendMessageEx, ISMEX_NOSEND};
    // SAFETY: the one argument is reserved and must be null.
    unsafe { InSendMessageEx(std::ptr::null()) != ISMEX_NOSEND }
}
