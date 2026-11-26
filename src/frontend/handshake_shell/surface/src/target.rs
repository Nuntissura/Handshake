//! The window crate's native token as the GPU owner's surface target: the one place where the two meet.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use handshake_shell_window::{NativeWindowToken, WindowKey};
use handshake_vulkan_renderer::{NativeWindow, SurfaceTarget};

/// A shell window as the GPU owner makes a surface on it.
///
/// It holds a clone of the window's [`NativeWindowToken`], and the window crate destroys no window while a token
/// exists, so the window outlives the surface the owner keeps this target for (VUID-VkWin32SurfaceCreateInfoKHR-hwnd-01308:
/// `hwnd` must be a valid Win32 `HWND`). The owner drops it once, after it destroyed the surface: from then on the
/// window can be destroyed. It prints without the handles.
pub struct ShellWindowTarget {
    token: NativeWindowToken,
    released: Option<Arc<AtomicU64>>,
}

impl ShellWindowTarget {
    /// The target of the window `token` names.
    pub fn new(token: NativeWindowToken) -> Self {
        Self {
            token,
            released: None,
        }
    }

    /// The same, counting its own drop in `released`.
    pub(crate) fn counted(token: NativeWindowToken, released: Arc<AtomicU64>) -> Self {
        Self {
            token,
            released: Some(released),
        }
    }

    /// Which window.
    pub fn window(&self) -> WindowKey {
        self.token.window()
    }
}

impl SurfaceTarget for ShellWindowTarget {
    fn native_window(&self) -> NativeWindow {
        let window = self.token.win32();
        NativeWindow::Win32 {
            hwnd: window.hwnd,
            hinstance: window.hinstance,
        }
    }
}

impl Drop for ShellWindowTarget {
    fn drop(&mut self) {
        if let Some(released) = &self.released {
            released.fetch_add(1, Ordering::AcqRel);
        }
    }
}

impl std::fmt::Debug for ShellWindowTarget {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ShellWindowTarget(window {})", self.window().number())
    }
}
