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

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

use super::ArgusWindowDescriptor;

// ── MT-008b: stable-identity OS-window handle registry ──────────────────────────────────────────────
//
// Matching an OS window by exact title + PID is ambiguous the moment two pop-out panes of the same
// module type share a title ("Handshake – Workspace"). Each Argus window nonetheless has a STABLE
// `window_id` (`popout-{pane_id}` / `main`). The shell records the OS window handle (HWND, stored as a
// portable `isize`) for each registered viewport at render time; the capture path then grabs THAT exact
// window instead of guessing which same-title HWND enumerated first. Title + PID remains the fallback
// only when no handle has been recorded (or a recorded handle has gone stale — e.g. the window was
// recreated), so an ambiguous-title pane becomes capturable while the existing main-window path is
// unchanged.

/// Process-global map from a stable Argus `window_id` to its recorded OS window handle (HWND as
/// `isize`). `isize` (not the raw `HWND` pointer) is stored so the value is `Send` across the UI thread
/// (which records) and the MCP server thread (which captures) — mirroring how the existing title-based
/// path already captures off the UI thread.
static WINDOW_HANDLE_REGISTRY: OnceLock<Mutex<HashMap<String, isize>>> = OnceLock::new();

fn window_handle_registry() -> &'static Mutex<HashMap<String, isize>> {
    WINDOW_HANDLE_REGISTRY.get_or_init(|| Mutex::new(HashMap::new()))
}

fn lock_registry() -> std::sync::MutexGuard<'static, HashMap<String, isize>> {
    window_handle_registry()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// Record (or overwrite) the OS window handle for a stable Argus `window_id`. A zero handle is ignored
/// (never a valid window). Called by the shell at viewport registration/render.
pub fn record_window_handle(window_id: &str, hwnd: isize) {
    if hwnd == 0 {
        return;
    }
    lock_registry().insert(window_id.to_owned(), hwnd);
}

/// Forget a recorded handle (e.g. when a pop-out merges back and its viewport is torn down), so a
/// later capture never grabs a dead HWND and instead falls back to title matching.
pub fn clear_window_handle(window_id: &str) {
    lock_registry().remove(window_id);
}

/// The recorded handle for a stable `window_id`, if any.
pub fn recorded_window_handle(window_id: &str) -> Option<isize> {
    lock_registry().get(window_id).copied()
}

/// The resolved source the capture path will use for a target window.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaptureTarget {
    /// A recorded HWND (`isize`) that passed the validity gate — captured directly, no title matching.
    RecordedHandle(isize),
    /// No usable recorded handle — fall back to exact title + PID enumeration.
    TitleFallback,
}

/// Pure resolution logic (OS-independent, unit-testable): prefer a recorded handle when it is present
/// AND still valid (`is_valid` is the injected OS validity gate — `IsWindow` + owning-PID in
/// production, a stub in tests); otherwise fall back to title matching. Keeping this a pure function of
/// `(recorded, is_valid)` is what lets the window-handle-based resolution be proven without a live GPU
/// or a real window.
pub fn resolve_capture_target(recorded: Option<isize>, is_valid: impl Fn(isize) -> bool) -> CaptureTarget {
    match recorded {
        Some(hwnd) if is_valid(hwnd) => CaptureTarget::RecordedHandle(hwnd),
        _ => CaptureTarget::TitleFallback,
    }
}

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
    /// SHA-256 of the decoded PNG bytes. This lets an agent correlate the payload with durable
    /// evidence without persisting a second copy of the image.
    pub sha256: String,
    /// Stable Argus window identity, populated by the production targeted capture path.
    pub window_id: Option<String>,
    /// Egui viewport identity associated with the registered window.
    pub viewport_id: Option<String>,
    /// Exact PID-scoped OS title used for capture.
    pub title: Option<String>,
    /// Owning process id used to fence the OS-window lookup.
    pub pid: Option<u32>,
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
            "sha256": self.sha256,
            "window_id": self.window_id,
            "viewport_id": self.viewport_id,
            "title": self.title,
            "pid": self.pid,
        })
    }

    fn with_window_metadata(mut self, window: &ArgusWindowDescriptor) -> Self {
        self.window_id = Some(window.window_id.clone());
        self.viewport_id = Some(window.viewport_id.clone());
        self.title = Some(window.title.clone());
        self.pid = Some(std::process::id());
        self
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
    use sha2::{Digest, Sha256};

    ScreenshotResult {
        png_base64: encode_base64(png_bytes),
        width,
        height,
        captured_at_utc: now_utc_string(),
        sha256: format!("{:x}", Sha256::digest(png_bytes)),
        window_id: None,
        viewport_id: None,
        title: None,
        pid: None,
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

/// Capture one registered Argus window. MT-008b: resolves by the STABLE `window_id` first — a recorded
/// OS window handle (HWND) is grabbed directly, so an ambiguous-title pop-out (two panes sharing a
/// module title) is captured unambiguously. Only when no valid handle is recorded does it fall back to
/// the exact title + PID enumeration (which still rejects ambiguous matches rather than guessing). The
/// main window (unique title, always registered) is unaffected either way.
pub fn capture_handshake_window_target(
    window: &ArgusWindowDescriptor,
) -> Result<ScreenshotResult, ScreenshotError> {
    #[cfg(target_os = "windows")]
    {
        let recorded = recorded_window_handle(&window.window_id);
        match resolve_capture_target(recorded, windows_capture::hwnd_is_capturable_for_this_process) {
            CaptureTarget::RecordedHandle(hwnd) => windows_capture::capture_recorded_hwnd(hwnd)
                .map(|capture| capture.with_window_metadata(window)),
            CaptureTarget::TitleFallback => {
                windows_capture::capture_window_by_title_and_pid(&window.title, std::process::id())
                    .map(|capture| capture.with_window_metadata(window))
            }
        }
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = window;
        Err(ScreenshotError(
            "live OS-window capture is implemented for Windows; use the offscreen-render path on this OS"
                .to_owned(),
        ))
    }
}

/// MT-008b: correlate a just-rendered viewport to its OS window and record the handle under its stable
/// `window_id`, so subsequent captures grab that exact window. Called by the shell inside a viewport's
/// render pass (where the viewport's own screen `outer_rect` is known). Cheap: skips enumeration when a
/// valid handle is already recorded. `outer_rect_px` is the viewport's outer screen rectangle in
/// PIXELS (`egui points * pixels_per_point`), used only to disambiguate when several windows share the
/// exact title; pass `None` when unknown (records only when the title+PID match is already unique).
/// No-op on non-Windows. Returns the recorded handle when one is now known.
pub fn record_viewport_window_handle(
    window_id: &str,
    title: &str,
    outer_rect_px: Option<(i32, i32, i32, i32)>,
) -> Option<isize> {
    #[cfg(target_os = "windows")]
    {
        // Fast path: a still-valid recorded handle needs no enumeration.
        if let Some(existing) = recorded_window_handle(window_id) {
            if windows_capture::hwnd_is_capturable_for_this_process(existing) {
                return Some(existing);
            }
        }
        let resolved = windows_capture::resolve_hwnd_by_title_and_geometry(
            title,
            std::process::id(),
            outer_rect_px,
        );
        if let Some(hwnd) = resolved {
            record_window_handle(window_id, hwnd);
        }
        resolved
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = (window_id, title, outer_rect_px);
        None
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
        EnumWindows, GetWindowRect, GetWindowTextW, GetWindowThreadProcessId, IsWindow,
        IsWindowVisible, PW_RENDERFULLCONTENT,
    };

    /// Find the visible top-level window matching `title` owned by `pid`, then capture it focus-safely.
    pub fn capture_window_by_title_and_pid(
        title: &str,
        pid: u32,
    ) -> Result<ScreenshotResult, ScreenshotError> {
        let hwnd = find_window(title, pid)?;
        capture_hwnd(hwnd)
    }

    /// MT-008b: whether a recorded handle is still a live, visible window owned by THIS process. The
    /// capture path uses this as the validity gate before trusting a recorded HWND; a window that was
    /// destroyed/recreated fails it and the caller falls back to title matching. Owning-PID is
    /// re-checked so a recycled handle value belonging to another process can never be captured.
    pub fn hwnd_is_capturable_for_this_process(hwnd_isize: isize) -> bool {
        let hwnd = hwnd_isize as HWND;
        // SAFETY: all three calls accept an arbitrary HWND and only READ window state; an invalid
        // handle returns 0 rather than faulting.
        unsafe {
            if IsWindow(hwnd) == 0 || IsWindowVisible(hwnd) == 0 {
                return false;
            }
            let mut win_pid: u32 = 0;
            GetWindowThreadProcessId(hwnd, &mut win_pid);
            win_pid == GetCurrentProcessId()
        }
    }

    /// MT-008b: capture a specific recorded HWND focus-safely, after re-validating it is still a window.
    pub fn capture_recorded_hwnd(hwnd_isize: isize) -> Result<ScreenshotResult, ScreenshotError> {
        let hwnd = hwnd_isize as HWND;
        // SAFETY: IsWindow only reads; an already-freed handle returns 0.
        if unsafe { IsWindow(hwnd) } == 0 {
            return Err(ScreenshotError(
                "recorded window handle is no longer a valid window".to_owned(),
            ));
        }
        capture_hwnd(hwnd)
    }

    /// MT-008b: resolve the OS window handle for `title` owned by `pid`. When exactly one visible
    /// window matches, return it. When several share the exact title (the ambiguous pop-out case),
    /// disambiguate by choosing the one whose top-left is nearest the caller-provided viewport
    /// `outer_rect_px` (its real screen position, from egui). Returns `None` if nothing matches, or if
    /// the match is ambiguous and no geometry hint was supplied (an honest "cannot decide" rather than
    /// guessing). The returned value is the HWND as `isize`.
    pub fn resolve_hwnd_by_title_and_geometry(
        title: &str,
        pid: u32,
        outer_rect_px: Option<(i32, i32, i32, i32)>,
    ) -> Option<isize> {
        let mut ctx = CollectCtx {
            want_title: title.encode_utf16().collect(),
            want_pid: pid,
            matches: Vec::new(),
        };
        // SAFETY: EnumWindows invokes `collect_enum_proc` synchronously with our &mut CollectCtx as the
        // lparam; the pointer is valid for the duration of the call.
        unsafe {
            EnumWindows(
                Some(collect_enum_proc),
                &mut ctx as *mut CollectCtx as LPARAM,
            );
        }
        match ctx.matches.len() {
            0 => None,
            1 => Some(ctx.matches[0].0 as isize),
            _ => {
                let (target_x, target_y) = match outer_rect_px {
                    Some((x, y, _, _)) => (x, y),
                    // Several same-title windows and no geometry hint: refuse to guess.
                    None => return None,
                };
                ctx.matches
                    .iter()
                    .min_by_key(|(_, rect)| {
                        let dx = (rect.left - target_x) as i64;
                        let dy = (rect.top - target_y) as i64;
                        dx * dx + dy * dy
                    })
                    .map(|(hwnd, _)| *hwnd as isize)
            }
        }
    }

    struct CollectCtx {
        want_title: Vec<u16>,
        want_pid: u32,
        matches: Vec<(HWND, RECT)>,
    }

    unsafe extern "system" fn collect_enum_proc(hwnd: HWND, lparam: LPARAM) -> i32 {
        let ctx = &mut *(lparam as *mut CollectCtx);
        if IsWindowVisible(hwnd) == 0 {
            return TRUE;
        }
        let mut win_pid: u32 = 0;
        GetWindowThreadProcessId(hwnd, &mut win_pid);
        if win_pid != ctx.want_pid {
            return TRUE;
        }
        let mut buf = [0u16; 512];
        let len = GetWindowTextW(hwnd, buf.as_mut_ptr(), buf.len() as i32);
        if len > 0 && &buf[..len as usize] == ctx.want_title.as_slice() {
            let mut rect: RECT = std::mem::zeroed();
            if GetWindowRect(hwnd, &mut rect) != 0 {
                ctx.matches.push((hwnd, rect));
            }
        }
        TRUE
    }

    struct FindCtx {
        want_title: Vec<u16>,
        want_pid: u32,
        found: HWND,
        match_count: usize,
    }

    /// Enumerate top-level windows, matching by exact title AND owning pid. Exactly one match is
    /// required: silently selecting the first of multiple same-title pop-outs would target the
    /// wrong viewport.
    fn find_window(title: &str, pid: u32) -> Result<HWND, ScreenshotError> {
        let mut ctx = FindCtx {
            want_title: title.encode_utf16().collect(),
            want_pid: pid,
            found: std::ptr::null_mut(),
            match_count: 0,
        };
        // SAFETY: EnumWindows calls `enum_proc` synchronously for each top-level window with our
        // &mut FindCtx as the lparam; the pointer is valid for the duration of the call.
        unsafe {
            EnumWindows(Some(enum_proc), &mut ctx as *mut FindCtx as LPARAM);
        }
        match ctx.match_count {
            0 => Err(ScreenshotError(format!(
                "no visible window titled '{title}' for pid {pid}"
            ))),
            1 => Ok(ctx.found),
            count => Err(ScreenshotError(format!(
                "ambiguous window target: {count} visible windows titled '{title}' for pid {pid}"
            ))),
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
                ctx.match_count += 1;
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
                let copied = BitBlt(mem_dc, 0, 0, width, height, window_dc, 0, 0, SRCCOPY);
                if copied == 0 {
                    SelectObject(mem_dc, old);
                    DeleteObject(bitmap as _);
                    DeleteDC(mem_dc);
                    ReleaseDC(hwnd, window_dc);
                    return Err(ScreenshotError(
                        "PrintWindow and BitBlt both failed".to_owned(),
                    ));
                }
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
