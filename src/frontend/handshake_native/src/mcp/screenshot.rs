//! The screenshot adapter: turn a rendered RGBA frame into a base64 PNG result whose JSON shape
//! matches the React `VisualCaptureResult` (`{png_base64, width, height, captured_at_utc}`), so agent
//! code is portable between the old webview capture and the native shell.
//!
//! ## Two capture sources: live OS-window (production) and offscreen render (headless proof)
//!
//! The contract asks the production `screenshot` tool to grab the live OS window focus-safely. egui
//! 0.33 does NOT expose a programmatic frame read-back from inside the running `eframe` app (the wgpu
//! surface is presented to the OS, not handed to the app), so the production capture path uses an OS
//! window grab. The native shell ships a focus-safe Win32 `PrintWindow`/`BitBlt`-style adapter
//! ([`capture_window_by_title_and_pid`]) — it NEVER calls `SetForegroundWindow`/`BringWindowToTop` and
//! never changes Z-order (HBR-QUIET). That OS path needs a real on-screen window and a windowing
//! environment, so it is GENUINELY UNDRIVEABLE from this headless `cargo test` host and is disclosed as
//! such in the handoff; it is wired into the live app and exercised by the running binary.
//!
//! The headless proof path uses `egui_kittest`'s wgpu renderer (`Harness::render()`), which renders the
//! SAME frame to an offscreen texture and reads it back as an `image::RgbaImage` — focus-safe BY
//! CONSTRUCTION (no OS window). This is what the over-the-wire transport test and the `test_mcp_screenshot`
//! proof use to prove a real, decodable PNG flows through the `screenshot` tool.
//!
//! ## base64 via the `base64` crate
//!
//! [`encode_base64`] uses `base64::engine::general_purpose::STANDARD` (already in the locked graph) so
//! `png_base64` decodes with any standard base64 reader an agent already has.

use std::time::{SystemTime, UNIX_EPOCH};

/// A captured screenshot, ready to serialize to the `VisualCaptureResult`-compatible JSON shape.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScreenshotResult {
    /// The PNG bytes, base64-encoded (RFC 4648 STANDARD alphabet, with `=` padding).
    pub png_base64: String,
    /// Image width in pixels (> 0 for a real frame).
    pub width: u32,
    /// Image height in pixels (> 0 for a real frame).
    pub height: u32,
    /// RFC3339-ish UTC capture timestamp (`<unix_seconds>.<nanos>Z`), matching the snapshot's clock
    /// format (no chrono dependency).
    pub captured_at_utc: String,
}

impl ScreenshotResult {
    /// Project to the `VisualCaptureResult`-compatible JSON value (`{png_base64, width, height,
    /// captured_at_utc}`) the MCP `screenshot` tool returns.
    pub fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "png_base64": self.png_base64,
            "width": self.width,
            "height": self.height,
            "captured_at_utc": self.captured_at_utc,
        })
    }
}

/// A screenshot failure (the render path returned an error). Surfaced (never panicked) so the tool
/// layer returns a well-formed JSON-RPC error instead of bringing down the caller (red-team: never
/// panic on the model-vision path).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScreenshotError(pub String);

impl std::fmt::Display for ScreenshotError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "screenshot capture failed: {}", self.0)
    }
}

impl std::error::Error for ScreenshotError {}

/// Encode raw PNG bytes into a [`ScreenshotResult`] with the given dimensions and a fresh timestamp.
/// The render/encoding of the frame to PNG bytes is the caller's job (the live test uses
/// `Harness::render()` + the `image` PNG encoder, both already available); this builds the
/// transport-ready result so the tool layer and a future socket transport share one shape.
pub fn screenshot_from_png(png_bytes: &[u8], width: u32, height: u32) -> ScreenshotResult {
    ScreenshotResult {
        png_base64: encode_base64(png_bytes),
        width,
        height,
        captured_at_utc: now_utc_string(),
    }
}

/// The window title the production capture matches (the live shell sets this title in `main.rs`).
pub const HANDSHAKE_WINDOW_TITLE: &str = "Handshake";

/// Capture the live Handshake OS window into a [`ScreenshotResult`], focus-safely.
///
/// PRODUCTION path (the contract's live OS-window grab). Matches the window whose title is
/// [`HANDSHAKE_WINDOW_TITLE`] AND whose owning process id is THIS process (red-team: window-title
/// ambiguity — a multi-window dev session never captures another process's window). Uses Win32
/// `PrintWindow` with `PW_RENDERFULLCONTENT` over an off-screen memory DC; it NEVER calls
/// `SetForegroundWindow`/`BringWindowToTop` and never changes Z-order (HBR-QUIET).
///
/// On non-Windows builds, or when no matching window is found / GDI fails, returns a typed
/// [`ScreenshotError`] (never panics) so the tool layer replies with a well-formed JSON-RPC error.
///
/// This OS path needs a real on-screen window + windowing environment, so it is GENUINELY UNDRIVEABLE
/// from a headless `cargo test` host — the over-the-wire transport test uses the offscreen-render
/// closure instead (focus-safe by construction). Disclosed in the MT-027 handoff.
pub fn capture_handshake_window() -> Result<ScreenshotResult, ScreenshotError> {
    #[cfg(target_os = "windows")]
    {
        windows_capture::capture_window_by_title_and_pid(HANDSHAKE_WINDOW_TITLE, std::process::id())
    }
    #[cfg(not(target_os = "windows"))]
    {
        Err(ScreenshotError(
            "live OS-window capture is implemented for Windows; use the offscreen-render path on this OS"
                .to_owned(),
        ))
    }
}

/// Win32 GDI window capture (focus-safe `PrintWindow`). Windows-only; gated behind `cfg(windows)` so
/// non-Windows builds never reference the Win32 APIs.
#[cfg(target_os = "windows")]
mod windows_capture {
    use super::{screenshot_from_png, ScreenshotError, ScreenshotResult};

    use windows_sys::Win32::Foundation::{HWND, LPARAM, RECT, TRUE};
    use windows_sys::Win32::Graphics::Gdi::{
        BitBlt, CreateCompatibleBitmap, CreateCompatibleDC, DeleteDC, DeleteObject, GetDC,
        GetDIBits, ReleaseDC, SelectObject, BITMAPINFO, BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS,
        HBITMAP, HDC, SRCCOPY,
    };
    // `PrintWindow` lives under `Win32::Storage::Xps` in windows-sys 0.61; `PW_RENDERFULLCONTENT` is in
    // `Win32::UI::WindowsAndMessaging`.
    use windows_sys::Win32::Storage::Xps::PrintWindow;
    use windows_sys::Win32::System::Threading::GetCurrentProcessId;
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        EnumWindows, GetWindowRect, GetWindowTextW, GetWindowThreadProcessId, IsWindowVisible,
        PW_RENDERFULLCONTENT,
    };

    /// Find the visible top-level window matching `title` owned by `pid`, then capture it focus-safely.
    pub fn capture_window_by_title_and_pid(
        title: &str,
        pid: u32,
    ) -> Result<ScreenshotResult, ScreenshotError> {
        let hwnd = find_window(title, pid).ok_or_else(|| {
            ScreenshotError(format!("no visible window titled '{title}' for pid {pid}"))
        })?;
        capture_hwnd(hwnd)
    }

    struct FindCtx {
        want_title: Vec<u16>,
        want_pid: u32,
        found: HWND,
    }

    /// Enumerate top-level windows, matching by exact title AND owning pid. Returns the first match.
    fn find_window(title: &str, pid: u32) -> Option<HWND> {
        let mut ctx = FindCtx {
            want_title: title.encode_utf16().collect(),
            want_pid: pid,
            found: std::ptr::null_mut(),
        };
        // SAFETY: EnumWindows calls `enum_proc` synchronously for each top-level window with our
        // &mut FindCtx as the lparam; the pointer is valid for the duration of the call.
        unsafe {
            EnumWindows(Some(enum_proc), &mut ctx as *mut FindCtx as LPARAM);
        }
        if ctx.found.is_null() {
            None
        } else {
            Some(ctx.found)
        }
    }

    unsafe extern "system" fn enum_proc(hwnd: HWND, lparam: LPARAM) -> i32 {
        let ctx = &mut *(lparam as *mut FindCtx);
        if IsWindowVisible(hwnd) == 0 {
            return TRUE; // keep enumerating
        }
        // Owning process id must match (red-team: never capture another process's window).
        let mut win_pid: u32 = 0;
        let _ = GetCurrentProcessId; // referenced so the import is used even if the path changes
        GetWindowThreadProcessId(hwnd, &mut win_pid);
        if win_pid != ctx.want_pid {
            return TRUE;
        }
        // Read the title and compare exactly.
        let mut buf = [0u16; 512];
        let len = GetWindowTextW(hwnd, buf.as_mut_ptr(), buf.len() as i32);
        if len > 0 {
            let got = &buf[..len as usize];
            if got == ctx.want_title.as_slice() {
                ctx.found = hwnd;
                return 0; // stop enumerating
            }
        }
        TRUE
    }

    /// Capture a specific HWND to PNG bytes via an off-screen memory DC + `PrintWindow`. Focus-safe:
    /// no foreground/Z-order change.
    fn capture_hwnd(hwnd: HWND) -> Result<ScreenshotResult, ScreenshotError> {
        // SAFETY: all handles are checked for null and released/deleted on every exit path below.
        unsafe {
            let mut rect: RECT = std::mem::zeroed();
            if GetWindowRect(hwnd, &mut rect) == 0 {
                return Err(ScreenshotError("GetWindowRect failed".to_owned()));
            }
            let width = (rect.right - rect.left).max(0);
            let height = (rect.bottom - rect.top).max(0);
            if width == 0 || height == 0 {
                return Err(ScreenshotError(
                    "window has zero area (minimized?)".to_owned(),
                ));
            }

            let window_dc: HDC = GetDC(hwnd);
            if window_dc.is_null() {
                return Err(ScreenshotError("GetDC failed".to_owned()));
            }
            let mem_dc: HDC = CreateCompatibleDC(window_dc);
            if mem_dc.is_null() {
                ReleaseDC(hwnd, window_dc);
                return Err(ScreenshotError("CreateCompatibleDC failed".to_owned()));
            }
            let bitmap: HBITMAP = CreateCompatibleBitmap(window_dc, width, height);
            if bitmap.is_null() {
                DeleteDC(mem_dc);
                ReleaseDC(hwnd, window_dc);
                return Err(ScreenshotError("CreateCompatibleBitmap failed".to_owned()));
            }
            let old = SelectObject(mem_dc, bitmap as _);

            // PrintWindow renders the window into the memory DC WITHOUT activating it (focus-safe).
            // PW_RENDERFULLCONTENT captures GPU-composited (wgpu) client content. Fall back to BitBlt
            // of the window DC if PrintWindow reports failure.
            let printed = PrintWindow(hwnd, mem_dc, PW_RENDERFULLCONTENT);
            if printed == 0 {
                let _ = BitBlt(mem_dc, 0, 0, width, height, window_dc, 0, 0, SRCCOPY);
            }

            // Read the bitmap bits out as top-down BGRA.
            let mut info: BITMAPINFO = std::mem::zeroed();
            info.bmiHeader.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as u32;
            info.bmiHeader.biWidth = width;
            info.bmiHeader.biHeight = -height; // negative => top-down rows
            info.bmiHeader.biPlanes = 1;
            info.bmiHeader.biBitCount = 32;
            info.bmiHeader.biCompression = BI_RGB;

            let pixel_count = (width as usize) * (height as usize);
            let mut bgra = vec![0u8; pixel_count * 4];
            let scanlines = GetDIBits(
                mem_dc,
                bitmap,
                0,
                height as u32,
                bgra.as_mut_ptr() as *mut _,
                &mut info,
                DIB_RGB_COLORS,
            );

            // Clean up GDI objects before encoding.
            SelectObject(mem_dc, old);
            DeleteObject(bitmap as _);
            DeleteDC(mem_dc);
            ReleaseDC(hwnd, window_dc);

            if scanlines == 0 {
                return Err(ScreenshotError("GetDIBits returned 0 scanlines".to_owned()));
            }

            // Convert BGRA -> RGBA in place.
            for px in bgra.chunks_exact_mut(4) {
                px.swap(0, 2);
            }

            // Encode to PNG via the `image` crate (already in the graph for the render path).
            let mut png_bytes: Vec<u8> = Vec::new();
            {
                use image::ImageEncoder;
                image::codecs::png::PngEncoder::new(&mut png_bytes)
                    .write_image(
                        &bgra,
                        width as u32,
                        height as u32,
                        image::ExtendedColorType::Rgba8,
                    )
                    .map_err(|e| ScreenshotError(format!("PNG encode failed: {e}")))?;
            }
            Ok(screenshot_from_png(&png_bytes, width as u32, height as u32))
        }
    }
}

/// RFC3339-ish UTC timestamp without a chrono dependency: `<unix_seconds>.<9-digit-nanos>Z`. Same
/// format the MT-026 snapshot uses, so a reader can correlate a screenshot with a tree snapshot by time.
fn now_utc_string() -> String {
    match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(d) => format!("{}.{:09}Z", d.as_secs(), d.subsec_nanos()),
        Err(_) => "0.000000000Z".to_owned(),
    }
}

/// Standard (RFC 4648) base64 encode with `=` padding via the `base64` crate's STANDARD engine, so
/// `png_base64` decodes with any standard base64 reader.
pub fn encode_base64(bytes: &[u8]) -> String {
    use base64::Engine;
    base64::engine::general_purpose::STANDARD.encode(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base64_matches_known_vectors() {
        // RFC 4648 §10 test vectors.
        assert_eq!(encode_base64(b""), "");
        assert_eq!(encode_base64(b"f"), "Zg==");
        assert_eq!(encode_base64(b"fo"), "Zm8=");
        assert_eq!(encode_base64(b"foo"), "Zm9v");
        assert_eq!(encode_base64(b"foob"), "Zm9vYg==");
        assert_eq!(encode_base64(b"fooba"), "Zm9vYmE=");
        assert_eq!(encode_base64(b"foobar"), "Zm9vYmFy");
    }

    #[test]
    fn result_json_has_visual_capture_shape() {
        let r = screenshot_from_png(b"foobar", 320, 240);
        let v = r.to_json();
        assert_eq!(v["png_base64"], "Zm9vYmFy");
        assert_eq!(v["width"], 320);
        assert_eq!(v["height"], 240);
        assert!(v["captured_at_utc"].as_str().unwrap().ends_with('Z'));
    }
}
