//! The native window, for the GPU owner only.

use std::fmt;
use std::num::NonZeroIsize;
use std::sync::Arc;

use crate::event::WindowKey;

/// Names a native window for whoever creates a surface on it.
///
/// The window is not destroyed while one of its tokens exists, so a surface that keeps its token cannot outlive the
/// window (a Vulkan requirement). Any thread may hold one. It prints without the handle.
#[derive(Clone)]
pub struct NativeWindowToken {
    window: WindowKey,
    handles: Win32Window,
    hold: Arc<()>,
}

/// The two numbers a Win32 surface is created from. Valid while the token they were read from exists.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Win32Window {
    /// The window handle.
    pub hwnd: NonZeroIsize,
    /// The module the window belongs to.
    pub hinstance: NonZeroIsize,
}

impl fmt::Debug for Win32Window {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Win32Window")
    }
}

impl NativeWindowToken {
    pub(crate) fn new(window: WindowKey, handles: Win32Window, hold: Arc<()>) -> Self {
        Self {
            window,
            handles,
            hold,
        }
    }

    /// Which window.
    pub fn window(&self) -> WindowKey {
        self.window
    }

    /// The Win32 handles. A windowing system added later gets an accessor of its own.
    pub fn win32(&self) -> Win32Window {
        self.handles
    }

    /// How many tokens of this window exist. At least this one.
    pub fn tokens(&self) -> usize {
        Arc::strong_count(&self.hold).saturating_sub(1)
    }
}

impl fmt::Debug for NativeWindowToken {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "NativeWindowToken(window {})", self.window.number())
    }
}
