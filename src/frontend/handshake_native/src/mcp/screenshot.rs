//! The screenshot adapter: turn a rendered RGBA frame into a base64 PNG result whose JSON shape
//! matches the React `VisualCaptureResult` (`{png_base64, width, height, captured_at_utc}`), so agent
//! code is portable between the old webview capture and the native shell.
//!
//! Production capture requests the registered live viewport's next rendered frame through egui's
//! `ViewportCommand::Screenshot` and receives its correlated `Event::Screenshot`. This reads the
//! actual GPU surface without focusing, raising, or replaying the window. GDI window capture remains
//! a legacy adapter; it cannot reliably read the client pixels of a presented wgpu surface.
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

/// Capture the registered main window's next actual renderer frame from a blocking worker.
/// A missing renderer, closed window, concurrent capture, or timeout returns a typed error.
/// No cached, headless, or GDI image replaces a missing live frame.
pub fn capture_handshake_window() -> Result<ScreenshotResult, ScreenshotError> {
    capture_live_frame(super::argus::MAIN_WINDOW_ID, None)
}

#[derive(Clone)]
struct LiveFrameWindow {
    window: super::argus::RegisteredWindow,
    context: egui::Context,
    ui_thread: std::thread::ThreadId,
}

#[derive(Debug)]
struct FrameCaptureToken(u64);

struct PendingFrame {
    window: super::argus::RegisteredWindow,
    reply: std::sync::mpsc::SyncSender<std::sync::Arc<egui::ColorImage>>,
}

#[derive(Default)]
struct LiveFrames {
    windows: std::collections::HashMap<String, LiveFrameWindow>,
    pending: std::collections::HashMap<u64, PendingFrame>,
}

static LIVE_FRAMES: std::sync::OnceLock<std::sync::Mutex<LiveFrames>> = std::sync::OnceLock::new();
static NEXT_FRAME_CAPTURE: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);

struct PendingFrameGuard(u64);

impl Drop for PendingFrameGuard {
    fn drop(&mut self) {
        LIVE_FRAMES.get_or_init(Default::default).lock().unwrap_or_else(|e| e.into_inner())
            .pending.remove(&self.0);
    }
}

/// Called only from real viewport frames, never from the isolated snapshot renderer.
pub(crate) fn update_live_frame_capture(
    windows: &super::argus::WindowSnapshotRegistry,
    window_id: &str,
    ctx: &egui::Context,
) {
    let Ok(window) = windows.get(window_id) else { return; };
    if window.descriptor.viewport_id != ctx.viewport_id()
        || !window.live.load(std::sync::atomic::Ordering::Acquire) { return; }
    let mut frames = LIVE_FRAMES.get_or_init(Default::default).lock().unwrap_or_else(|e| e.into_inner());
    frames.windows.retain(|_, entry| entry.window.live.load(std::sync::atomic::Ordering::Acquire));
    frames.windows.insert(window_id.to_owned(), LiveFrameWindow {
        window, context: ctx.clone(), ui_thread: std::thread::current().id(),
    });
    ctx.input(|input| {
        for event in &input.events {
            let egui::Event::Screenshot { viewport_id, user_data, image } = event else { continue; };
            let Some(token) = user_data.data.as_ref().and_then(|data| data.downcast_ref::<FrameCaptureToken>()) else { continue; };
            let Some(pending) = frames.pending.get(&token.0) else { continue; };
            if pending.window.descriptor.viewport_id == *viewport_id
                && pending.window.live.load(std::sync::atomic::Ordering::Acquire) {
                let _ = pending.reply.try_send(image.clone());
            }
        }
    });
}

fn capture_live_frame(
    window_id: &str,
    expected: Option<&super::argus::RegisteredWindow>,
) -> Result<ScreenshotResult, ScreenshotError> {
    let token = NEXT_FRAME_CAPTURE.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let (reply, receiver) = std::sync::mpsc::sync_channel(1);
    let entry = {
        let mut frames = LIVE_FRAMES.get_or_init(Default::default).lock().unwrap_or_else(|e| e.into_inner());
        let entry = frames.windows.get(window_id).cloned()
            .ok_or_else(|| ScreenshotError("Argus window has no registered live renderer".to_owned()))?;
        if !entry.window.live.load(std::sync::atomic::Ordering::Acquire)
            || expected.is_some_and(|window| window.epoch != entry.window.epoch
                || window.descriptor.viewport_id != entry.window.descriptor.viewport_id
                || window.descriptor.title != entry.window.descriptor.title
                || !std::sync::Arc::ptr_eq(&window.live, &entry.window.live)) {
            return Err(ScreenshotError("Argus capture window identity is stale".to_owned()));
        }
        if entry.ui_thread == std::thread::current().id() {
            return Err(ScreenshotError("live frame capture must be requested off the UI thread".to_owned()));
        }
        if frames.pending.values().any(|pending| pending.window.descriptor.window_id == window_id) {
            return Err(ScreenshotError("a live frame capture is already in progress for this window".to_owned()));
        }
        frames.pending.insert(token, PendingFrame { window: entry.window.clone(), reply });
        entry
    };
    let _pending = PendingFrameGuard(token);
    entry.context.send_viewport_cmd_to(entry.window.descriptor.viewport_id,
        egui::ViewportCommand::Screenshot(egui::UserData::new(FrameCaptureToken(token))));
    // Only the MCP blocking worker waits; the UI thread merely delivers the renderer's event.
    let result = receiver.recv_timeout(std::time::Duration::from_millis(1500));
    let image = result.map_err(|error| ScreenshotError(format!("live renderer frame unavailable: {error}")))?;
    if !entry.window.live.load(std::sync::atomic::Ordering::Acquire) {
        return Err(ScreenshotError("Argus capture window closed during capture".to_owned()));
    }
    let width = u32::try_from(image.size[0]).map_err(|_| ScreenshotError("frame width overflow".to_owned()))?;
    let height = u32::try_from(image.size[1]).map_err(|_| ScreenshotError("frame height overflow".to_owned()))?;
    if width == 0 || height == 0 { return Err(ScreenshotError("renderer returned an empty frame".to_owned())); }
    let rgba: Vec<u8> = image.pixels.iter().flat_map(|pixel| pixel.to_array()).collect();
    let mut png = Vec::new();
    use image::ImageEncoder;
    image::codecs::png::PngEncoder::new(&mut png)
        .write_image(&rgba, width, height, image::ExtendedColorType::Rgba8)
        .map_err(|error| ScreenshotError(format!("PNG encode failed: {error}")))?;
    Ok(screenshot_from_png(&png, width, height))
}

#[derive(Clone)]
struct RegisteredHandle {
    handle: isize,
    title: String,
    epoch: u64,
    viewport: egui::ViewportId,
    outer_rect_px: (i32, i32, i32, i32),
    live: std::sync::Arc<std::sync::atomic::AtomicBool>,
}

static WINDOW_HANDLES: std::sync::OnceLock<std::sync::Mutex<std::collections::HashMap<String, RegisteredHandle>>> = std::sync::OnceLock::new();

pub(crate) fn clear_registered_window_handle(window: &super::argus::RegisteredWindow) {
    let mut frames = LIVE_FRAMES.get_or_init(Default::default).lock().unwrap_or_else(|e| e.into_inner());
    if frames.windows.get(&window.descriptor.window_id).is_some_and(|entry|
        entry.window.epoch == window.epoch && std::sync::Arc::ptr_eq(&entry.window.live, &window.live)) {
        frames.windows.remove(&window.descriptor.window_id);
    }
    frames.pending.retain(|_, pending| !std::sync::Arc::ptr_eq(&pending.window.live, &window.live));
    drop(frames);
    let mut handles = WINDOW_HANDLES.get_or_init(Default::default).lock().unwrap_or_else(|e| e.into_inner());
    if handles.get(&window.descriptor.window_id).is_some_and(|handle|
        handle.epoch == window.epoch && handle.viewport == window.descriptor.viewport_id
            && std::sync::Arc::ptr_eq(&handle.live, &window.live)) {
        handles.remove(&window.descriptor.window_id);
    }
}

// Pinned legacy resolution seam; seeded handles never enter live registered capture.
static LEGACY_WINDOW_HANDLES: std::sync::OnceLock<std::sync::Mutex<std::collections::HashMap<String, isize>>> = std::sync::OnceLock::new();

pub fn record_window_handle(window_id: &str, hwnd: isize) {
    if hwnd != 0 {
        LEGACY_WINDOW_HANDLES.get_or_init(Default::default).lock().unwrap_or_else(|e| e.into_inner())
            .insert(window_id.to_owned(), hwnd);
    }
}

pub fn clear_window_handle(window_id: &str) {
    LEGACY_WINDOW_HANDLES.get_or_init(Default::default).lock().unwrap_or_else(|e| e.into_inner()).remove(window_id);
}

pub fn recorded_window_handle(window_id: &str) -> Option<isize> {
    LEGACY_WINDOW_HANDLES.get_or_init(Default::default).lock().unwrap_or_else(|e| e.into_inner()).get(window_id).copied()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaptureTarget { RecordedHandle(isize), TitleFallback }

pub fn resolve_capture_target(recorded: Option<isize>, is_valid: impl Fn(isize) -> bool) -> CaptureTarget {
    match recorded {
        Some(hwnd) if is_valid(hwnd) => CaptureTarget::RecordedHandle(hwnd),
        _ => CaptureTarget::TitleFallback,
    }
}

pub(crate) fn record_viewport_window_handle(
    windows: &super::argus::WindowSnapshotRegistry, window_id: &str, epoch: u64,
    ctx: &egui::Context, title: &str, outer_rect_px: Option<(i32, i32, i32, i32)>,
) -> Option<isize> {
    let window = windows.get(window_id).ok()?;
    let _channel = window.channel.lock().unwrap_or_else(|e| e.into_inner());
    if window.epoch != epoch || window.descriptor.viewport_id != ctx.viewport_id()
        || window.descriptor.title != title || !window.live.load(std::sync::atomic::Ordering::Acquire) {
        return None;
    }
    update_live_frame_capture(windows, window_id, ctx);
    let (left, top, right, bottom) = outer_rect_px?;
    if right <= left || bottom <= top { return None; }
    #[cfg(target_os = "windows")]
    {
        let hwnd = windows_capture::resolve_registered_window(title, std::process::id(), (left, top, right, bottom))?;
        let current = windows.get(window_id).ok()?;
        if current.epoch != epoch || !std::sync::Arc::ptr_eq(&current.live, &window.live)
            || !window.live.load(std::sync::atomic::Ordering::Acquire) { return None; }
        WINDOW_HANDLES.get_or_init(Default::default).lock().unwrap_or_else(|e| e.into_inner())
            .insert(window_id.to_owned(), RegisteredHandle { handle: hwnd, title: title.to_owned(),
                epoch, viewport: ctx.viewport_id(), outer_rect_px: (left, top, right, bottom), live: window.live.clone() });
        Some(hwnd)
    }
    #[cfg(not(target_os = "windows"))]
    { let _ = (left, top, right, bottom); None }
}

pub(crate) fn capture_registered_window(window: &super::argus::RegisteredWindow) -> Result<ScreenshotResult, ScreenshotError> {
    capture_live_frame(&window.descriptor.window_id, Some(window))
}

/// Win32 GDI window capture (focus-safe `PrintWindow`). Windows-only; gated behind `cfg(windows)` so
/// non-Windows builds never reference the Win32 APIs.
#[cfg(target_os = "windows")]
mod windows_capture {
    use super::{screenshot_from_png, ScreenshotError, ScreenshotResult};
    use std::sync::atomic::{AtomicBool, Ordering};

    use windows_sys::Win32::Foundation::{HWND, LPARAM, RECT, TRUE};
    use windows_sys::Win32::Graphics::Gdi::{
        BitBlt, CreateCompatibleBitmap, CreateCompatibleDC, DeleteDC, DeleteObject, GetWindowDC,
        GetDIBits, ReleaseDC, SelectObject, BITMAPINFO, BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS,
        HBITMAP, HDC, SRCCOPY,
    };
    use windows_sys::Win32::System::Threading::GetCurrentProcessId;
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        EnumWindows, GetWindowRect, GetWindowTextW, GetWindowThreadProcessId, IsWindowVisible,
        SendMessageTimeoutW, PRF_CHECKVISIBLE, PRF_CHILDREN, PRF_CLIENT, PRF_ERASEBKGND,
        PRF_NONCLIENT, PRF_OWNED, SMTO_ABORTIFHUNG, SMTO_ERRORONEXIT, WM_PRINT,
    };

    /// Prevent a stalled OS/driver capture from accumulating unbounded blocking workers. The async MCP
    /// server returns a typed timeout after two seconds; this guard remains held by the detached capture
    /// worker until the OS call actually unwinds, so later screenshots fail fast while all non-capture
    /// Swarm traffic continues normally.
    static CAPTURE_IN_FLIGHT: AtomicBool = AtomicBool::new(false);

    struct CaptureInFlightGuard;

    impl CaptureInFlightGuard {
        fn acquire() -> Result<Self, ScreenshotError> {
            CAPTURE_IN_FLIGHT
                .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
                .map(|_| Self)
                .map_err(|_| {
                    ScreenshotError("a previous screenshot capture is still in progress".to_owned())
                })
        }
    }

    impl Drop for CaptureInFlightGuard {
        fn drop(&mut self) {
            CAPTURE_IN_FLIGHT.store(false, Ordering::Release);
        }
    }

    /// Find the visible top-level window matching `title` owned by `pid`, then capture it focus-safely.
    pub fn capture_window_by_title_and_pid(
        title: &str,
        pid: u32,
    ) -> Result<ScreenshotResult, ScreenshotError> {
        let _in_flight = CaptureInFlightGuard::acquire()?;
        let hwnd = find_window(title, pid).ok_or_else(|| {
            ScreenshotError(format!("no visible window titled '{title}' for pid {pid}"))
        })?;
        capture_hwnd(hwnd)
    }

    struct FindCtx {
        want_title: Vec<u16>,
        want_pid: u32,
        found: HWND,
        matches: usize,
        rect: Option<(i32, i32, i32, i32)>,
    }

    /// Enumerate top-level windows, matching by exact title AND owning pid. Returns the first match.
    fn find_window(title: &str, pid: u32) -> Option<HWND> {
        let mut ctx = FindCtx {
            want_title: title.encode_utf16().collect(),
            want_pid: pid,
            found: std::ptr::null_mut(),
            matches: 0,
            rect: None,
        };
        // SAFETY: EnumWindows calls `enum_proc` synchronously for each top-level window with our
        // &mut FindCtx as the lparam; the pointer is valid for the duration of the call.
        unsafe {
            EnumWindows(Some(enum_proc), &mut ctx as *mut FindCtx as LPARAM);
        }
        if ctx.found.is_null() || ctx.matches != 1 {
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
                if let Some(wanted) = ctx.rect {
                    let mut rect: RECT = std::mem::zeroed();
                    if GetWindowRect(hwnd, &mut rect) == 0 || (rect.left, rect.top, rect.right, rect.bottom) != wanted {
                        return TRUE;
                    }
                }
                ctx.found = hwnd;
                ctx.matches += 1;
            }
        }
        TRUE
    }

    pub(super) fn resolve_registered_window(title: &str, pid: u32, rect: (i32, i32, i32, i32)) -> Option<isize> {
        let mut ctx = FindCtx { want_title: title.encode_utf16().collect(), want_pid: pid, found: std::ptr::null_mut(), matches: 0, rect: Some(rect) };
        unsafe { EnumWindows(Some(enum_proc), &mut ctx as *mut FindCtx as LPARAM); }
        if ctx.matches == 1 && !ctx.found.is_null() { Some(ctx.found as isize) } else { None }
    }

    pub(super) fn capture_registered_handle(handle: isize, title: &str, pid: u32, expected_rect: (i32, i32, i32, i32)) -> Result<ScreenshotResult, ScreenshotError> {
        let _in_flight = CaptureInFlightGuard::acquire()?;
        let hwnd = handle as HWND;
        let mut owner = 0;
        let mut buf = [0u16; 512];
        let mut rect: RECT = unsafe { std::mem::zeroed() };
        let valid = unsafe {
            GetWindowThreadProcessId(hwnd, &mut owner);
            let len = GetWindowTextW(hwnd, buf.as_mut_ptr(), buf.len() as i32);
            IsWindowVisible(hwnd) != 0 && owner == pid && len > 0
                && buf[..len as usize] == title.encode_utf16().collect::<Vec<_>>()
                && GetWindowRect(hwnd, &mut rect) != 0
                && (rect.left, rect.top, rect.right, rect.bottom) == expected_rect
        };
        if !valid { return Err(ScreenshotError("Argus recorded window is no longer owned by this process/descriptor".to_owned())); }
        capture_hwnd(hwnd)
    }

    /// Capture a specific HWND to PNG bytes via an off-screen memory DC + bounded `WM_PRINT`.
    /// Focus-safe: no foreground/Z-order change. `PrintWindow` is intentionally not used: Microsoft
    /// documents it as synchronous and potentially unbounded, which can stall the Swarm server's sole
    /// runtime worker when the target window does not answer. `SendMessageTimeoutW` preserves the same
    /// window-owned paint request with a hard one-second boundary, then falls back to `BitBlt`.
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

            let window_dc: HDC = GetWindowDC(hwnd);
            if window_dc.is_null() {
                return Err(ScreenshotError("GetWindowDC failed".to_owned()));
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
            if old.is_null() {
                DeleteObject(bitmap as _);
                DeleteDC(mem_dc);
                ReleaseDC(hwnd, window_dc);
                return Err(ScreenshotError("SelectObject failed".to_owned()));
            }

            // Ask the window to paint into the memory DC WITHOUT activating it. Unlike PrintWindow,
            // SendMessageTimeoutW cannot wedge this request forever: an unresponsive window is aborted
            // after one second. WM_PRINT is a system message, so cross-thread marshalling is supported.
            let print_flags = PRF_CHECKVISIBLE
                | PRF_NONCLIENT
                | PRF_CLIENT
                | PRF_ERASEBKGND
                | PRF_CHILDREN
                | PRF_OWNED;
            let mut paint_result = 0usize;
            let printed = SendMessageTimeoutW(
                hwnd,
                WM_PRINT,
                mem_dc as usize,
                print_flags as isize,
                SMTO_ABORTIFHUNG | SMTO_ERRORONEXIT,
                1_000,
                &mut paint_result,
            );
            if printed == 0
                && BitBlt(mem_dc, 0, 0, width, height, window_dc, 0, 0, SRCCOPY) == 0
            {
                SelectObject(mem_dc, old);
                DeleteDC(mem_dc);
                DeleteObject(bitmap as _);
                ReleaseDC(hwnd, window_dc);
                return Err(ScreenshotError("WM_PRINT and BitBlt capture failed".to_owned()));
            }

            // GetDIBits requires the bitmap to be deselected from every DC.
            if SelectObject(mem_dc, old).is_null() {
                DeleteDC(mem_dc);
                DeleteObject(bitmap as _);
                ReleaseDC(hwnd, window_dc);
                return Err(ScreenshotError("bitmap deselection failed".to_owned()));
            }

            // Read the bitmap bits out as top-down BGRX.
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
            DeleteObject(bitmap as _);
            DeleteDC(mem_dc);
            ReleaseDC(hwnd, window_dc);

            if scanlines == 0 {
                return Err(ScreenshotError("GetDIBits returned 0 scanlines".to_owned()));
            }

            // BI_RGB's fourth byte is unused; PNG needs an explicit opaque alpha.
            for px in bgra.chunks_exact_mut(4) {
                px.swap(0, 2);
                px[3] = 255;
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
