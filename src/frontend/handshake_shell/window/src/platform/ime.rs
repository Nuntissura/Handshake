//! The one call to the input method that this crate makes itself, beside the window library's (MT-214 S3-Q1): complete
//! a composition, so that its text is committed to the target it was typed at, never cancelled or carried to another.
//!
//! Microsoft Learn, ImmNotifyIME: `NI_COMPOSITIONSTR` with `CPS_COMPLETE` "Set the composition string as the result
//! string". An input method that does so sends `WM_IME_COMPOSITION` with the result string, and the window library makes
//! a commit of it. Nothing else is asked of the input method here: no cancel, no conversion, no composition string set
//! (the input crate's source scan holds this file to this one call).

use windows_sys::Win32::Foundation::HWND;
use windows_sys::Win32::UI::Input::Ime::{
    ImmGetContext, ImmNotifyIME, ImmReleaseContext, CPS_COMPLETE, NI_COMPOSITIONSTR,
};

/// Ask the input method to complete the composition of `hwnd`, a window of the calling thread. What it sends the
/// window for that while this runs is handled before this returns; whether an input method sends it then or later is
/// its own (UNVERIFIED for real input methods: MT-214, the x64 gate). `false` when the window has no input context
/// (composition is off for it, or the system has no input method support), or the input method refused.
pub(super) fn complete(hwnd: HWND) -> bool {
    // SAFETY: a window of this thread; the context is released once, below.
    let himc = unsafe { ImmGetContext(hwnd) };
    if himc == 0 {
        return false;
    }
    // SAFETY: the context just taken, released after the call.
    let completed = unsafe { ImmNotifyIME(himc, NI_COMPOSITIONSTR, CPS_COMPLETE, 0) } != 0;
    // SAFETY: as taken above.
    unsafe { ImmReleaseContext(hwnd, himc) };
    completed
}
