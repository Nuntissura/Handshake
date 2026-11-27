//! The Windows adapter: the asset's accesskit_windows 0.29.2 (assets rev 8eadc5b5, Kumihimo third-party), attached as
//! accesskit_winit 0.29.2 attaches it on Windows (`platform_impl/windows.rs`): a `SubclassingAdapter` on the window
//! before it is first shown, `update_if_active`, then the queued events raised. Its procedure goes in front of the
//! window crate's hook (comctl32 `SetWindowSubclass`) and answers `WM_GETOBJECT` only; it notes `WM_SETFOCUS` and
//! `WM_KILLFOCUS` and passes everything on. All unsafe code of the crate is here: two reads before the adapter is made,
//! and the window property the adapter leaves behind when it fails.

use std::collections::BTreeSet;
use std::ffi::c_void;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::{Mutex, PoisonError};

use accesskit::TreeUpdate;
use accesskit_windows::{SubclassingAdapter, HWND};
use handshake_shell_window::NativeWindowToken;
use windows_sys::Win32::System::Threading::GetCurrentThreadId;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    GetPropW, GetWindowThreadProcessId, IsWindowVisible, RemovePropW,
};

use crate::hub::{AttachRefusal, TreeSource};
use crate::request::ActionSink;

/// The windows of this process that have an adapter: one each, since a second panics inside the adapter
/// (accesskit_windows 0.29.2 `subclass.rs`, `install`).
static ATTACHED: Mutex<BTreeSet<isize>> = Mutex::new(BTreeSet::new());

/// The name of the window property the adapter keeps its state pointer under (accesskit_windows 0.29.2 `subclass.rs`,
/// `PROP_NAME`). The adapter sets it, then takes the window's procedure; when the second step fails it panics and its
/// state is freed, with the property left pointing at it (MT-214 E7-F9, asset request RQ-AKW-01 asks the asset to set
/// the property last).
const ADAPTER_PROPERTY: &str = "AccessKitAdapter";

fn property_name() -> Vec<u16> {
    ADAPTER_PROPERTY.encode_utf16().chain([0]).collect()
}

/// Make the adapter with `make`, on the window's thread. A window that has the property already is refused before
/// anything is made (the adapter panics on it, and a cleanup would take another owner's property). When `make`
/// panics, the property it may have set is removed, since this call saw it was not there before: nothing of the
/// adapter is left on the window. The procedure is never replaced by a failing install (`SetWindowLongPtrW` is its
/// last call).
fn guarded<T>(hwnd: isize, make: impl FnOnce() -> T) -> Result<T, AttachRefusal> {
    let name = property_name();
    // SAFETY: a read of a window the caller's token keeps alive, with a terminated name.
    if unsafe { GetPropW(hwnd, name.as_ptr()) } != 0 {
        return Err(AttachRefusal::AlreadyAttached);
    }
    catch_unwind(AssertUnwindSafe(make)).map_err(|_| {
        // SAFETY: as above; the property is ours to remove, it was not set before the attempt.
        unsafe { RemovePropW(hwnd, name.as_ptr()) };
        AttachRefusal::Adapter
    })
}

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
        // It panics instead of failing (window property or procedure refused); the property it left is removed.
        let adapter = guarded(hwnd, || {
            SubclassingAdapter::new(HWND(hwnd as *mut c_void), source, sink)
        })?;
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

/// The window crate's test support: a private desktop under the watches (HBR-QUIET-001), as the integration tests use.
#[cfg(test)]
#[path = "../../window/tests/support/mod.rs"]
mod support;

#[cfg(test)]
mod tests {
    use std::cell::Cell;
    use std::time::Duration;

    use windows_sys::Win32::UI::WindowsAndMessaging::{
        CreateWindowExW, DestroyWindow, SetPropW, HWND_MESSAGE,
    };

    use super::support::on_private_desktop;
    use super::*;

    /// A message-only window of this thread (a thread on a private desktop): never shown, never active, no input.
    struct Hidden(isize);

    impl Hidden {
        fn new() -> Self {
            let class: Vec<u16> = "STATIC".encode_utf16().chain([0]).collect();
            // SAFETY: a message-only window of the class every system has, destroyed on this thread below.
            let hwnd = unsafe {
                CreateWindowExW(
                    0,
                    class.as_ptr(),
                    std::ptr::null(),
                    0,
                    0,
                    0,
                    0,
                    0,
                    HWND_MESSAGE,
                    0,
                    0,
                    std::ptr::null(),
                )
            };
            assert_ne!(hwnd, 0, "a message-only window");
            Self(hwnd)
        }

        fn property(&self) -> isize {
            let name = property_name();
            // SAFETY: a window of this thread, a terminated name.
            unsafe { GetPropW(self.0, name.as_ptr()) }
        }

        fn set_property(&self, value: isize) {
            let name = property_name();
            // SAFETY: as above.
            assert_ne!(unsafe { SetPropW(self.0, name.as_ptr(), value) }, 0);
        }
    }

    impl Drop for Hidden {
        fn drop(&mut self) {
            let name = property_name();
            // SAFETY: as above; a property is removed before its window goes.
            unsafe {
                RemovePropW(self.0, name.as_ptr());
                DestroyWindow(self.0);
            }
        }
    }

    /// MT-214 E7-F9, mutation caught: a failed install leaving the adapter's window property on the window, pointing
    /// at state that was freed; or a cleanup that takes a property that was there before.
    /// MT-214 E8-F3: the window is made on a private desktop, like every window of every other test.
    #[test]
    fn a_failed_install_leaves_no_property_and_another_owners_is_not_touched() {
        on_private_desktop(Duration::from_secs(60), a_failed_install_on_this_desktop);
    }

    fn a_failed_install_on_this_desktop() {
        let window = Hidden::new();
        let failed: Result<(), AttachRefusal> = guarded(window.0, || {
            window.set_property(0x00DE_AD00);
            panic!("the install failed after its property was set");
        });
        assert_eq!(failed, Err(AttachRefusal::Adapter));
        assert_eq!(window.property(), 0, "the property of the failed install is gone");

        assert_eq!(guarded(window.0, || 7), Ok(7));
        assert_eq!(window.property(), 0, "a made adapter is no business of guarded");

        window.set_property(0x1234);
        let made = Cell::new(false);
        let refused = guarded(window.0, || made.set(true));
        assert_eq!(refused, Err(AttachRefusal::AlreadyAttached));
        assert!(!made.get(), "nothing was made over a window that has the property");
        assert_eq!(window.property(), 0x1234, "the other owner's property stays");
    }
}
