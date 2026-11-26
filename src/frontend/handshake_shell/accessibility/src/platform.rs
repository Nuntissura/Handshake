//! The Windows adapter: the asset's accesskit_windows 0.29.2 (assets rev 8eadc5b5, Kumihimo third-party), attached as
//! accesskit_winit 0.29.2 attaches it on Windows (`platform_impl/windows.rs`): a `SubclassingAdapter` on the window
//! before it is first shown, `update_if_active`, then the queued events raised. Its procedure goes in front of the
//! window crate's hook (comctl32 `SetWindowSubclass`) and answers `WM_GETOBJECT` only; it notes `WM_SETFOCUS` and
//! `WM_KILLFOCUS` and passes everything on. All unsafe code of the crate is here: two reads before the adapter is made.

use std::collections::BTreeSet;
use std::ffi::c_void;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::{Mutex, PoisonError};

use accesskit::TreeUpdate;
use accesskit_windows::{SubclassingAdapter, HWND};
use handshake_shell_window::NativeWindowToken;
use windows_sys::Win32::System::Threading::GetCurrentThreadId;
use windows_sys::Win32::UI::WindowsAndMessaging::{GetWindowThreadProcessId, IsWindowVisible};

use crate::hub::{AttachRefusal, TreeSource};
use crate::request::ActionSink;

/// The windows of this process that have an adapter: one each, since a second panics inside the adapter
/// (accesskit_windows 0.29.2 `subclass.rs`, `install`).
static ATTACHED: Mutex<BTreeSet<isize>> = Mutex::new(BTreeSet::new());

/// One window's adapter.
pub(crate) struct Attached {
    /// Dropped first: it gives the window its procedure back while the token still keeps the window.
    adapter: SubclassingAdapter,
    hwnd: isize,
    _token: NativeWindowToken,
}

impl Attached {
    /// Attach on the window's own thread, before it is first shown.
    pub(crate) fn new(
        token: NativeWindowToken,
        source: TreeSource,
        sink: ActionSink,
    ) -> Result<Self, AttachRefusal> {
        let hwnd = token.win32().hwnd.get();
        // SAFETY: reads of a window the token keeps alive; the process number is not asked for.
        let (thread, visible) = unsafe {
            (
                GetWindowThreadProcessId(hwnd, std::ptr::null_mut()),
                IsWindowVisible(hwnd),
            )
        };
        // SAFETY: no arguments.
        if thread != unsafe { GetCurrentThreadId() } {
            return Err(AttachRefusal::WrongThread);
        }
        if visible != 0 {
            return Err(AttachRefusal::AlreadyShown);
        }
        let mut attached = ATTACHED.lock().unwrap_or_else(PoisonError::into_inner);
        if attached.contains(&hwnd) {
            return Err(AttachRefusal::AlreadyAttached);
        }
        // It panics instead of failing (window property or procedure refused); then nothing of it was installed.
        let adapter = catch_unwind(AssertUnwindSafe(|| {
            SubclassingAdapter::new(HWND(hwnd as *mut c_void), source, sink)
        }))
        .map_err(|_| AttachRefusal::Adapter)?;
        attached.insert(hwnd);
        Ok(Self {
            adapter,
            hwnd,
            _token: token,
        })
    }

    /// Give the adapter `update` if an assistive tool activated it, and raise its events with nothing borrowed.
    /// `false` when the operating system refused an event: the adapter unwraps the result.
    pub(crate) fn update(&mut self, update: impl FnOnce() -> TreeUpdate) -> bool {
        match self.adapter.update_if_active(update) {
            Some(events) => catch_unwind(AssertUnwindSafe(|| events.raise())).is_ok(),
            None => true,
        }
    }
}

impl Drop for Attached {
    fn drop(&mut self) {
        ATTACHED
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .remove(&self.hwnd);
    }
}
