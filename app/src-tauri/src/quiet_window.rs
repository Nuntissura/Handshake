use std::path::PathBuf;

use tauri::webview::{WebviewWindow, WebviewWindowBuilder};
use tauri::{AppHandle, Runtime, Url, WebviewUrl};

pub struct QuietWindowBuilder<'a, R: Runtime> {
    builder: WebviewWindowBuilder<'a, R, AppHandle<R>>,
}

impl<'a, R: Runtime> QuietWindowBuilder<'a, R> {
    #[allow(clippy::disallowed_methods)]
    pub fn new<L: Into<String>>(app: &'a AppHandle<R>, label: L, url: WebviewUrl) -> Self {
        let builder = WebviewWindowBuilder::new(app, label, url)
            .visible(false)
            .focused(false)
            .focusable(false)
            .skip_taskbar(true)
            .always_on_bottom(true)
            .decorations(false);
        Self { builder }
    }

    pub fn title<S: Into<String>>(self, title: S) -> Self {
        Self {
            builder: self.builder.title(title),
        }
    }

    pub fn data_directory(self, data_directory: PathBuf) -> Self {
        Self {
            builder: self.builder.data_directory(data_directory),
        }
    }

    pub fn on_navigation<F>(self, handler: F) -> Self
    where
        F: Fn(&Url) -> bool + Send + 'static,
    {
        Self {
            builder: self.builder.on_navigation(handler),
        }
    }

    pub fn build(self) -> tauri::Result<WebviewWindow<R>> {
        self.builder.build()
    }
}

/// Builder for the HBR-QUIET-004 foreground-exception window (spec §6.6.7).
///
/// This is the ONLY sanctioned visible+focused+foreground window in Handshake.
/// Unlike [`QuietWindowBuilder`] (which forces every window off-screen and
/// unfocused), this builder deliberately creates a SHOWN, FOCUSED, bounded
/// window. It is gated behind the `ForegroundException` declaration + operator
/// warning surface in `handshake_core::operator_foreground::foreground_exception`
/// and must never be used for ordinary model-driven work. It lives here, next to
/// `QuietWindowBuilder`, so the `clippy::disallowed_methods` exemption for
/// `WebviewWindowBuilder::new` stays scoped to this one file.
pub struct ForegroundExceptionWindowBuilder<'a, R: Runtime> {
    builder: WebviewWindowBuilder<'a, R, AppHandle<R>>,
}

impl<'a, R: Runtime> ForegroundExceptionWindowBuilder<'a, R> {
    #[allow(clippy::disallowed_methods)]
    pub fn new<L: Into<String>>(app: &'a AppHandle<R>, label: L, url: WebviewUrl) -> Self {
        // Deliberately visible + focused: spec §6.6.7 requires a SHOWN, FOCUSED,
        // bounded controlled window. The bound (timeout + auto-dismiss) is
        // enforced by ControlledWindow in handshake_core, not here.
        let builder = WebviewWindowBuilder::new(app, label, url)
            .visible(true)
            .focused(true)
            .focusable(true)
            .skip_taskbar(false)
            .always_on_top(true)
            .decorations(true)
            .inner_size(640.0, 360.0);
        Self { builder }
    }

    pub fn title<S: Into<String>>(self, title: S) -> Self {
        Self {
            builder: self.builder.title(title),
        }
    }

    pub fn inner_size(self, width: f64, height: f64) -> Self {
        Self {
            builder: self.builder.inner_size(width, height),
        }
    }

    pub fn build(self) -> tauri::Result<WebviewWindow<R>> {
        let window = self.builder.build()?;
        // Belt-and-suspenders: ensure the window is actually shown and focused
        // even on platforms where the builder hints are advisory.
        window.show()?;
        window.set_focus()?;
        Ok(window)
    }
}
