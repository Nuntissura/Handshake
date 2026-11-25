//! Windows: the window library and Win32, behind this crate's own types.
//!
//! The only unsafe code of the crate. Nothing in here is public; replacing the window library means replacing
//! `app`, `translate` and the handle inside `window`.
#![allow(unsafe_code)]

mod app;
mod clipboard;
mod drops;
mod loop_core;
mod monitors;
mod place;
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
