//! App-layer real-window implementation of the HBR-QUIET-004 controlled
//! foreground-exception window (spec §6.6.7).
//!
//! `handshake_core` cannot depend on `tauri`, so it defines the
//! [`ControlledWindowSurface`] seam and the bounded-window state machine
//! (`ControlledWindow`) that enforces timeout + auto-dismiss. This module
//! implements that seam over a real `tauri::WebviewWindow`:
//!
//! 1. [`build_foreground_exception_window`] CREATES a SHOWN + FOCUSED bounded
//!    Tauri window via [`ForegroundExceptionWindowBuilder`], then wraps it in a
//!    [`TauriControlledWindowSurface`].
//! 2. The surface reports the window's live `is_visible` / `is_focused` state.
//! 3. `ControlledWindow::auto_dismiss_at_deadline` (in core) calls
//!    `surface.close()` at the deadline, which CLOSES the real window.
//!
//! This is the real wiring required to lift MT-019 / §6.6.7 above the previous
//! logging-only stub. The Integrate phase owns `lib.rs` registration; this
//! module exposes `pub` functions for it to call.

use std::sync::Arc;

use handshake_core::operator_foreground::foreground_exception::{
    ControlledWindow, ControlledWindowSurface, ForegroundExceptionError, ForegroundExceptionHandle,
};
use tauri::webview::WebviewWindow;
use tauri::{AppHandle, Manager, Runtime, Url, WebviewUrl};

use crate::quiet_window::ForegroundExceptionWindowBuilder;

/// Real-window surface backed by a live `tauri::WebviewWindow`.
///
/// `WebviewWindow`'s `is_visible` / `is_focused` / `close` are thread-safe and
/// callable from any thread (including the tokio executor that drives
/// `auto_dismiss_at_deadline`), so no main-thread hop is required for the seam.
pub struct TauriControlledWindowSurface<R: Runtime> {
    window: WebviewWindow<R>,
}

impl<R: Runtime> TauriControlledWindowSurface<R> {
    pub fn new(window: WebviewWindow<R>) -> Self {
        Self { window }
    }
}

/// Map a `tauri` query result on a controlled window into the bounded-window
/// state machine's view of liveness.
///
/// A window that has already been closed/destroyed no longer has a live
/// dispatcher, so Tauri returns [`tauri::Error::WindowNotFound`] /
/// [`tauri::Error::WebviewNotFound`]; those are normal terminal states and map
/// to `false` (the window is gone, hence not visible/focused). Any *other*
/// Tauri error means the query itself failed while the window was supposed to be
/// live (e.g. an IPC/runtime fault) — that must NOT be silently read as
/// "benign-invisible", because it could mask a window that failed to actually
/// show. We surface it as a [`ForegroundExceptionError::WindowSurface`] so the
/// fault is observable in the audit trail instead of being swallowed.
fn map_window_query(
    result: tauri::Result<bool>,
    query: &str,
) -> Result<bool, ForegroundExceptionError> {
    match result {
        Ok(value) => Ok(value),
        Err(tauri::Error::WindowNotFound) | Err(tauri::Error::WebviewNotFound) => Ok(false),
        Err(error) => Err(ForegroundExceptionError::WindowSurface(format!(
            "controlled window {query} query failed: {error}"
        ))),
    }
}

impl<R: Runtime> ControlledWindowSurface for TauriControlledWindowSurface<R> {
    fn is_visible(&self) -> Result<bool, ForegroundExceptionError> {
        map_window_query(self.window.is_visible(), "is_visible")
    }

    fn is_focused(&self) -> Result<bool, ForegroundExceptionError> {
        map_window_query(self.window.is_focused(), "is_focused")
    }

    fn close(&self) -> Result<(), ForegroundExceptionError> {
        // `close` consumes the request asynchronously inside Tauri; closing an
        // already-closed window returns an error we tolerate so dismissal is
        // idempotent-safe.
        match self.window.close() {
            Ok(()) => Ok(()),
            Err(tauri::Error::WindowNotFound) => Ok(()),
            Err(error) => Err(ForegroundExceptionError::WindowSurface(error.to_string())),
        }
    }
}

/// Create the real, shown, focused, bounded controlled window and attach it to
/// the `ControlledWindow` state machine from `handshake_core`.
///
/// On success a real Tauri window is already on screen and focused; the returned
/// [`ControlledWindow`] owns the timeout + auto-dismiss bound. Call
/// `controlled.auto_dismiss_at_deadline().await` (or `controlled.dismiss(..)`)
/// to tear the real window down.
pub fn build_foreground_exception_window<R: Runtime>(
    app: &AppHandle<R>,
    handle: &ForegroundExceptionHandle,
    label: impl Into<String>,
    url: impl Into<String>,
) -> Result<ControlledWindow, ForegroundExceptionError> {
    let label = label.into();
    let url = url.into();

    // Reuse an existing window with the same label if one is already open, so a
    // re-declared exception does not stack duplicate foreground windows.
    let webview_url = parse_webview_url(&url)?;
    let window = if let Some(existing) = app.get_webview_window(&label) {
        existing
            .show()
            .map_err(|error| ForegroundExceptionError::WindowSurface(error.to_string()))?;
        existing
            .set_focus()
            .map_err(|error| ForegroundExceptionError::WindowSurface(error.to_string()))?;
        existing
    } else {
        ForegroundExceptionWindowBuilder::new(app, label.clone(), webview_url)
            .title(format!("Handshake foreground exception: {}", label))
            .build()
            .map_err(|error| ForegroundExceptionError::WindowSurface(error.to_string()))?
    };

    let surface: Arc<dyn ControlledWindowSurface> =
        Arc::new(TauriControlledWindowSurface::new(window));

    handle.bounded_window_with_surface(label, url, surface)
}

fn parse_webview_url(url: &str) -> Result<WebviewUrl, ForegroundExceptionError> {
    let trimmed = url.trim();
    if trimmed.is_empty() {
        return Err(ForegroundExceptionError::InvalidWindowUrl);
    }
    // External http(s) URLs load over the network; everything else is treated as
    // an app-relative (bundled) path so the controlled window can show a local
    // operator-warning surface without a live server.
    if trimmed.starts_with("http://") || trimmed.starts_with("https://") {
        let parsed = Url::parse(trimmed)
            .map_err(|error| ForegroundExceptionError::WindowSurface(error.to_string()))?;
        Ok(WebviewUrl::External(parsed))
    } else {
        Ok(WebviewUrl::App(trimmed.trim_start_matches('/').into()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn map_window_query_passes_through_live_state() {
        assert_eq!(map_window_query(Ok(true), "is_visible").unwrap(), true);
        assert_eq!(map_window_query(Ok(false), "is_focused").unwrap(), false);
    }

    #[test]
    fn map_window_query_treats_closed_window_as_not_live() {
        // A closed/destroyed window has no live dispatcher; both the window and
        // webview not-found variants map to `false` (gone, not a fault).
        assert_eq!(
            map_window_query(Err(tauri::Error::WindowNotFound), "is_visible").unwrap(),
            false
        );
        assert_eq!(
            map_window_query(Err(tauri::Error::WebviewNotFound), "is_focused").unwrap(),
            false
        );
    }

    #[test]
    fn map_window_query_surfaces_unexpected_faults() {
        // A genuine query failure on a window that is supposed to be live must
        // NOT be swallowed as benign-invisible; it surfaces as WindowSurface so
        // a window that silently failed to display is observable in the audit
        // trail. `AssetNotFound` stands in for any non-not-found tauri::Error.
        let error = map_window_query(
            Err(tauri::Error::AssetNotFound("probe".to_string())),
            "is_visible",
        )
        .expect_err("unexpected query failure must be surfaced, not swallowed");
        match error {
            ForegroundExceptionError::WindowSurface(message) => {
                assert!(
                    message.contains("is_visible"),
                    "fault message must name the failing query: {message}"
                );
            }
            other => panic!("expected WindowSurface error, got {other:?}"),
        }
    }

    #[test]
    fn parse_webview_url_rejects_empty() {
        assert!(matches!(
            parse_webview_url("   "),
            Err(ForegroundExceptionError::InvalidWindowUrl)
        ));
    }

    #[test]
    fn parse_webview_url_classifies_external_and_app_paths() {
        assert!(matches!(
            parse_webview_url("https://example.test/warn"),
            Ok(WebviewUrl::External(_))
        ));
        match parse_webview_url("/operator/warn.html").expect("app path parses") {
            WebviewUrl::App(path) => assert_eq!(path, PathBuf::from("operator/warn.html")),
            other => panic!("expected App url, got {other:?}"),
        }
    }
}
