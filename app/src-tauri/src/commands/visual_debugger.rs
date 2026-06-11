//! NATIVE, FOCUS-SAFE visual-capture core for Handshake's built-in visual
//! debugger.
//!
//! Unlike `crate::visual_debug` (which speaks CDP over a `--remote-debugging-port`
//! WebSocket), this module drives the **in-process** WebView2 control directly via
//! `ICoreWebView2::CallDevToolsProtocolMethod`. It reaches the live
//! `ICoreWebView2` through `tauri::WebviewWindow::with_webview`, whose closure runs
//! on the **main (UI) thread**. Results are marshalled back out to the calling
//! async task over a `std::sync::mpsc` channel so the command never blocks the UI
//! thread.
//!
//! # Research-grounded constraints
//! WebView2 is NOT an off-screen renderer: when the host window is hidden or
//! minimized the compositor produces BLANK frames for `CapturePreview`. We
//! therefore capture PIXELS exclusively via CDP `Page.captureScreenshot` with
//! `{"fromSurface": true, "captureBeyondViewport": true}`. `fromSurface` reads the
//! compositor surface, so it works when the window is off-screen or occluded by
//! other windows. DOM / AX / console all run over CDP and are render-state
//! independent (headless-ok).
//!
//! # Focus safety (HBR-QUIET, [GLOBAL-BUILD-QUIET])
//! The capture path NEVER invokes the Win32 foreground-activation API,
//! `ShowWindow(SW_SHOW)`, or Tauri's window focus-activation method, and never
//! otherwise activates / re-orders the window. The
//! default behavior captures the app's CURRENT window state without changing its
//! activation, focus, or Z-order at all (`fromSurface` makes that possible). The
//! sole Win32 helper provided here — [`ensure_visible_no_activate`] — is an
//! explicit, opt-in escape hatch that only ever uses no-activate flags
//! (`WS_EX_NOACTIVATE` + `ShowWindow(SW_SHOWNOACTIVATE)` +
//! `SetWindowPos(.., SWP_NOACTIVATE | SWP_NOZORDER | SWP_NOMOVE | SWP_NOSIZE)`);
//! it is not invoked by any of the three commands and is gated behind a `cfg`
//! `windows` build.
//!
//! # Crate versions (matched to the transitive `wry` 0.53 / `webview2-com` 0.38)
//! `app/src-tauri/Cargo.lock` already pulls `webview2-com 0.38.0` (via `wry`
//! 0.53.5) which itself depends on `windows 0.61.3` + `windows-core 0.61.2`. We
//! pin `webview2-com = "0.38"` and `windows = "0.61"` so the `ICoreWebView2*`
//! COM interface types, `HRESULT`/`PCWSTR` string types, and the
//! `windows_core::Result` returned by the completion/event handlers are the SAME
//! monomorphized types `tauri::webview::PlatformWebview::controller()` hands us.
//! A version skew (e.g. `windows 0.62`) would produce two incompatible
//! `ICoreWebView2Controller` types and fail to compile.

#[cfg(windows)]
mod imp {
    use std::collections::VecDeque;
    use std::sync::{mpsc, Arc, Mutex};
    use std::time::Duration;

    use base64::{engine::general_purpose, Engine as _};
    use serde::Serialize;
    use serde_json::{json, Value};
    use tauri::{AppHandle, Manager, Runtime, State};
    use time::format_description::well_known::Rfc3339;

    use webview2_com::Microsoft::Web::WebView2::Win32::{
        ICoreWebView2, ICoreWebView2DevToolsProtocolEventReceivedEventArgs,
    };
    use webview2_com::{
        take_pwstr, CallDevToolsProtocolMethodCompletedHandler,
        DevToolsProtocolEventReceivedEventHandler,
    };
    use windows::core::{HRESULT, PCWSTR, PWSTR};
    use windows::Win32::Foundation::HWND;
    use windows::Win32::UI::WindowsAndMessaging::{
        GetAncestor, GetWindowLongPtrW, IsWindowVisible, SetWindowLongPtrW, SetWindowPos,
        ShowWindow, GA_ROOT, GWL_EXSTYLE, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SWP_NOZORDER,
        SW_SHOWNOACTIVATE, WS_EX_NOACTIVATE,
    };

    /// Ring-buffer capacity for the console/log/exception/network-error buffer.
    pub const CONSOLE_BUFFER_CAPACITY: usize = 2_048;
    /// How long a CDP call may take before the command gives up waiting on the
    /// main-thread closure / completion handler.
    const CDP_CALL_TIMEOUT: Duration = Duration::from_secs(15);

    // ---- Public result shapes (the IPC return types) ---------------------------

    #[derive(Debug, Clone, Serialize)]
    pub struct VisualCaptureResult {
        pub png_base64: String,
        pub width: u32,
        pub height: u32,
        pub captured_at_utc: String,
    }

    #[derive(Debug, Clone, Serialize)]
    pub struct AxBounds {
        pub x: f64,
        pub y: f64,
        pub w: f64,
        pub h: f64,
    }

    #[derive(Debug, Clone, Serialize)]
    pub struct AxNode {
        pub id: String,
        pub role: String,
        pub name: String,
        pub bounds: Option<AxBounds>,
    }

    #[derive(Debug, Clone, Serialize)]
    pub struct AxTreeResult {
        pub nodes: Vec<AxNode>,
        pub frame_id: Option<String>,
    }

    #[derive(Debug, Clone, Serialize)]
    pub struct ConsoleEntry {
        /// `console` | `exception` | `log` | `network_error`
        pub kind: String,
        /// CDP level (`log`/`warning`/`error`/`info`/`debug`) where applicable.
        pub level: String,
        pub text: String,
        pub url: Option<String>,
        /// CDP timestamp (ms since epoch as a float) when present.
        pub timestamp: Option<f64>,
        /// Wall-clock RFC3339 time the entry was buffered.
        pub received_at_utc: String,
    }

    // ---- Managed state: the shared console ring buffer -------------------------

    /// Tauri-managed state holding the buffered console/exception/log/network
    /// entries. Wired ONCE at startup via [`register_visual_debug_event_receivers`]
    /// and drained by [`visual_debug_console`].
    #[derive(Clone)]
    pub struct VisualDebugConsoleBuffer {
        inner: Arc<Mutex<VecDeque<ConsoleEntry>>>,
    }

    impl Default for VisualDebugConsoleBuffer {
        fn default() -> Self {
            Self {
                inner: Arc::new(Mutex::new(VecDeque::with_capacity(CONSOLE_BUFFER_CAPACITY))),
            }
        }
    }

    impl VisualDebugConsoleBuffer {
        fn push(&self, entry: ConsoleEntry) {
            if let Ok(mut buf) = self.inner.lock() {
                if buf.len() >= CONSOLE_BUFFER_CAPACITY {
                    buf.pop_front();
                }
                buf.push_back(entry);
            }
        }

        fn drain(&self) -> Vec<ConsoleEntry> {
            self.inner
                .lock()
                .map(|mut buf| buf.drain(..).collect())
                .unwrap_or_default()
        }

        fn snapshot(&self) -> Vec<ConsoleEntry> {
            self.inner
                .lock()
                .map(|buf| buf.iter().cloned().collect())
                .unwrap_or_default()
        }
    }

    // ---- Helpers ---------------------------------------------------------------

    fn now_rfc3339() -> String {
        time::OffsetDateTime::now_utc()
            .format(&Rfc3339)
            .unwrap_or_else(|_| "1970-01-01T00:00:00Z".to_string())
    }

    /// Encode a Rust `&str` as a NUL-terminated UTF-16 buffer for `PCWSTR`.
    fn to_pcwstr(value: &str) -> Vec<u16> {
        value.encode_utf16().chain(std::iter::once(0)).collect()
    }

    /// Read a `PCWSTR` (NUL-terminated UTF-16) into a Rust `String` WITHOUT
    /// freeing it. Used for the `CallDevToolsProtocolMethod` completion handler's
    /// `returnObjectAsJson`, which is owned by WebView2 (not CoTaskMem-allocated by
    /// us), so it must be copied but never freed here.
    unsafe fn pcwstr_to_string(ptr: PCWSTR) -> String {
        if ptr.0.is_null() {
            return String::new();
        }
        ptr.to_string().unwrap_or_default()
    }

    fn resolve_window<R: Runtime>(
        app: &AppHandle<R>,
        window_label: Option<String>,
    ) -> Result<tauri::WebviewWindow<R>, String> {
        let label = window_label.unwrap_or_else(|| "main".to_string());
        app.get_webview_window(&label)
            .ok_or_else(|| format!("visual debugger: no webview window with label \"{label}\""))
    }

    /// Run a single synchronous CDP method against the in-process WebView2 and
    /// return its `returnObjectAsJson`.
    ///
    /// `with_webview` hands us the live `ICoreWebView2` on the **main thread**;
    /// `CallDevToolsProtocolMethod` is itself async (its result arrives on the
    /// completion handler). We bridge BOTH hops back to this (async-task) thread
    /// with `std::sync::mpsc` channels so nothing blocks the UI thread.
    fn call_cdp<R: Runtime>(
        window: &tauri::WebviewWindow<R>,
        method: &str,
        params: &Value,
    ) -> Result<Value, String> {
        let method_owned = method.to_string();
        let params_owned = serde_json::to_string(params).map_err(|e| e.to_string())?;

        // Channel #1: main-thread closure -> here (did the CDP call dispatch?).
        let (dispatch_tx, dispatch_rx) = mpsc::channel::<Result<(), String>>();
        // Channel #2: completion handler -> here (the JSON result).
        let (result_tx, result_rx) = mpsc::channel::<Result<String, String>>();

        window
            .with_webview(move |platform| {
                // SAFETY: we are on the WebView2 UI thread; the controller and its
                // `ICoreWebView2` are valid for the duration of this closure. We do
                // NOT touch activation/focus/Z-order here.
                let dispatch: Result<(), String> = unsafe {
                    let controller = platform.controller();
                    match controller.CoreWebView2() {
                        Err(e) => Err(e.to_string()),
                        Ok(core) => {
                            let core: ICoreWebView2 = core;
                            let method_w = to_pcwstr(&method_owned);
                            let params_w = to_pcwstr(&params_owned);

                            let result_tx = result_tx.clone();
                            let handler =
                                CallDevToolsProtocolMethodCompletedHandler::create(Box::new(
                                    move |error: ::windows::core::Result<()>,
                                          return_object_as_json: String| {
                                        match error {
                                            Ok(()) => {
                                                let _ = result_tx.send(Ok(return_object_as_json));
                                            }
                                            Err(e) => {
                                                let _ = result_tx.send(Err(format!(
                                                    "CallDevToolsProtocolMethod failed: {e}"
                                                )));
                                            }
                                        }
                                        Ok(())
                                    },
                                ));

                            core.CallDevToolsProtocolMethod(
                                PCWSTR(method_w.as_ptr()),
                                PCWSTR(params_w.as_ptr()),
                                &handler,
                            )
                            .map_err(|e| e.to_string())
                        }
                    }
                };
                let _ = dispatch_tx.send(dispatch);
            })
            .map_err(|e| format!("with_webview dispatch failed: {e}"))?;

        // Wait for the main-thread closure to confirm the call dispatched.
        match dispatch_rx.recv_timeout(CDP_CALL_TIMEOUT) {
            Ok(Ok(())) => {}
            Ok(Err(e)) => return Err(e),
            Err(_) => return Err("visual debugger: timed out dispatching CDP call".to_string()),
        }

        // Wait for the completion handler to deliver the JSON result.
        let json = match result_rx.recv_timeout(CDP_CALL_TIMEOUT) {
            Ok(Ok(json)) => json,
            Ok(Err(e)) => return Err(e),
            Err(_) => return Err(format!("visual debugger: timed out awaiting {method} result")),
        };

        if json.trim().is_empty() {
            return Ok(Value::Null);
        }
        serde_json::from_str(&json).map_err(|e| format!("{method} returned invalid JSON: {e}"))
    }

    // ---- 1. Screenshot ---------------------------------------------------------

    pub fn visual_debug_capture_impl<R: Runtime>(
        app: AppHandle<R>,
        window_label: Option<String>,
    ) -> Result<VisualCaptureResult, String> {
        let window = resolve_window(&app, window_label)?;

        // Render-state-independent pixel capture: `fromSurface` reads the
        // compositor surface so this works off-screen / behind other windows
        // WITHOUT activating the app.
        let params = json!({
            "format": "png",
            "fromSurface": true,
            "captureBeyondViewport": true
        });
        let result = call_cdp(&window, "Page.captureScreenshot", &params)?;

        let data = result
            .get("data")
            .and_then(Value::as_str)
            .ok_or_else(|| "Page.captureScreenshot response missing `data`".to_string())?;
        let png = general_purpose::STANDARD
            .decode(data)
            .map_err(|e| format!("screenshot base64 decode failed: {e}"))?;
        if !png.starts_with(b"\x89PNG\r\n\x1a\n") {
            return Err("Page.captureScreenshot did not return a PNG".to_string());
        }
        let (width, height) = png_dimensions(&png).unwrap_or((0, 0));

        Ok(VisualCaptureResult {
            png_base64: data.to_string(),
            width,
            height,
            captured_at_utc: now_rfc3339(),
        })
    }

    /// Parse width/height from a PNG IHDR (bytes 16..24, big-endian). Avoids an
    /// image-decode dependency just to report dimensions.
    fn png_dimensions(png: &[u8]) -> Option<(u32, u32)> {
        if png.len() < 24 {
            return None;
        }
        let width = u32::from_be_bytes([png[16], png[17], png[18], png[19]]);
        let height = u32::from_be_bytes([png[20], png[21], png[22], png[23]]);
        Some((width, height))
    }

    // ---- 2. Accessibility tree -------------------------------------------------

    pub fn visual_debug_ax_tree_impl<R: Runtime>(
        app: AppHandle<R>,
        window_label: Option<String>,
    ) -> Result<AxTreeResult, String> {
        let window = resolve_window(&app, window_label)?;

        // Accessibility.getFullAXTree requires the Accessibility domain enabled.
        let _ = call_cdp(&window, "Accessibility.enable", &json!({}))?;
        let ax = call_cdp(&window, "Accessibility.getFullAXTree", &json!({}))?;

        // DOMSnapshot gives layout bounds keyed by backendNodeId. We join AX nodes
        // to layout rects on backendNodeId.
        let snapshot = call_cdp(
            &window,
            "DOMSnapshot.captureSnapshot",
            &json!({ "computedStyles": [], "includeDOMRects": true }),
        )
        .unwrap_or(Value::Null);

        let bounds_by_backend = build_backend_bounds(&snapshot);

        let frame_id = ax
            .get("nodes")
            .and_then(Value::as_array)
            .and_then(|nodes| nodes.iter().find_map(|n| node_frame_id(n)));

        let mut out = Vec::new();
        if let Some(nodes) = ax.get("nodes").and_then(Value::as_array) {
            for node in nodes {
                if node
                    .get("ignored")
                    .and_then(Value::as_bool)
                    .unwrap_or(false)
                {
                    continue;
                }
                let id = node_id_string(node.get("nodeId"));
                let role = node
                    .get("role")
                    .and_then(|r| r.get("value"))
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_string();
                let name = node
                    .get("name")
                    .and_then(|n| n.get("value"))
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_string();
                let bounds = node
                    .get("backendDOMNodeId")
                    .and_then(value_as_i64)
                    .and_then(|backend| bounds_by_backend.get(&backend).cloned());

                if id.is_empty() && role.is_empty() && name.is_empty() {
                    continue;
                }
                out.push(AxNode {
                    id,
                    role,
                    name,
                    bounds,
                });
            }
        }

        Ok(AxTreeResult {
            nodes: out,
            frame_id,
        })
    }

    /// Edge 148+ may surface `frameId` either at the tree root or per node, and
    /// node ids as either ints or strings. Read it defensively from common spots.
    fn node_frame_id(node: &Value) -> Option<String> {
        node.get("frameId")
            .and_then(Value::as_str)
            .map(str::to_string)
    }

    /// Node ids changed from string to occasionally-integer across CDP/Edge
    /// versions; accept either and normalize to `String`.
    fn node_id_string(value: Option<&Value>) -> String {
        match value {
            Some(Value::String(s)) => s.clone(),
            Some(Value::Number(n)) => n.to_string(),
            _ => String::new(),
        }
    }

    fn value_as_i64(value: &Value) -> Option<i64> {
        value
            .as_i64()
            .or_else(|| value.as_u64().and_then(|v| i64::try_from(v).ok()))
            .or_else(|| value.as_str().and_then(|s| s.parse::<i64>().ok()))
    }

    /// Build a `backendNodeId -> bounds` map from a `DOMSnapshot.captureSnapshot`
    /// result. The snapshot stores layout in parallel arrays under
    /// `documents[].layout` (`nodeIndex`, `bounds`) joined to
    /// `documents[].nodes.backendNodeId`.
    fn build_backend_bounds(snapshot: &Value) -> std::collections::HashMap<i64, AxBounds> {
        let mut map = std::collections::HashMap::new();
        let Some(documents) = snapshot.get("documents").and_then(Value::as_array) else {
            return map;
        };
        for doc in documents {
            let backend_ids = doc
                .get("nodes")
                .and_then(|n| n.get("backendNodeId"))
                .and_then(Value::as_array);
            let layout = doc.get("layout");
            let (Some(backend_ids), Some(layout)) = (backend_ids, layout) else {
                continue;
            };
            let node_index = layout.get("nodeIndex").and_then(Value::as_array);
            let bounds = layout.get("bounds").and_then(Value::as_array);
            let (Some(node_index), Some(bounds)) = (node_index, bounds) else {
                continue;
            };
            for (i, ni) in node_index.iter().enumerate() {
                let Some(node_idx) = value_as_i64(ni) else {
                    continue;
                };
                let Some(backend) = backend_ids
                    .get(node_idx as usize)
                    .and_then(value_as_i64)
                else {
                    continue;
                };
                if let Some(rect) = bounds.get(i).and_then(Value::as_array) {
                    if rect.len() >= 4 {
                        let f = |idx: usize| rect[idx].as_f64().unwrap_or(0.0);
                        map.insert(
                            backend,
                            AxBounds {
                                x: f(0),
                                y: f(1),
                                w: f(2),
                                h: f(3),
                            },
                        );
                    }
                }
            }
        }
        map
    }

    // ---- 3. Console buffer -----------------------------------------------------

    pub fn visual_debug_console_impl(
        buffer: State<'_, VisualDebugConsoleBuffer>,
    ) -> Result<Vec<ConsoleEntry>, String> {
        // Draining returns the buffered entries and clears them so the next call
        // only sees new activity (matches a "tail since last poll" model).
        Ok(buffer.drain())
    }

    /// Snapshot WITHOUT draining (used by tests / non-consuming inspectors).
    #[allow(dead_code)]
    pub fn console_snapshot(buffer: &VisualDebugConsoleBuffer) -> Vec<ConsoleEntry> {
        buffer.snapshot()
    }

    // ---- Startup wiring: enable domains + add event receivers ------------------

    /// Enable the CDP domains and register the DevTools event receivers that feed
    /// the shared console ring buffer. Call ONCE from Tauri `setup` after the main
    /// window exists, passing the managed [`VisualDebugConsoleBuffer`].
    ///
    /// Does NOT activate/focus the window — only enables domains and subscribes to
    /// events through the in-process `ICoreWebView2`.
    pub fn register_visual_debug_event_receivers<R: Runtime>(
        window: &tauri::WebviewWindow<R>,
        buffer: VisualDebugConsoleBuffer,
    ) -> Result<(), String> {
        // Enable Runtime / Log / Network once so the events start flowing. These
        // are fire-and-forget; failures are surfaced but non-fatal so app startup
        // can continue with an empty buffer.
        for (method, params) in [
            ("Runtime.enable", json!({})),
            ("Log.enable", json!({})),
            ("Network.enable", json!({})),
        ] {
            if let Err(e) = call_cdp(window, method, &params) {
                eprintln!("visual debugger: {method} failed during setup: {e}");
            }
        }

        // Subscribe to the four event streams. Each receiver pushes into the shared
        // ring buffer. We add them on the main thread via `with_webview`.
        let events = [
            ("Runtime.consoleAPICalled", "console"),
            ("Runtime.exceptionThrown", "exception"),
            ("Log.entryAdded", "log"),
            ("Network.loadingFailed", "network_error"),
        ];

        let buffer_for_closure = buffer;
        window
            .with_webview(move |platform| {
                // SAFETY: main (UI) thread; the controller + ICoreWebView2 are live
                // for the closure body. No activation/focus/Z-order calls. The work
                // is wrapped in an inner closure returning `windows::core::Result`
                // so the COM `?` operators propagate to it (not to the outer
                // `with_webview` closure, which must return `()`).
                let wire = || -> windows::core::Result<()> {
                    unsafe {
                        let controller = platform.controller();
                        let core = controller.CoreWebView2()?;
                        for (event_name, kind) in events {
                            let event_w = to_pcwstr(event_name);
                            let receiver =
                                core.GetDevToolsProtocolEventReceiver(PCWSTR(event_w.as_ptr()))?;
                            let buf = buffer_for_closure.clone();
                            let kind_owned = kind.to_string();
                            let mut token: i64 = 0;
                            let handler =
                                DevToolsProtocolEventReceivedEventHandler::create(Box::new(
                                    move |_sender: Option<ICoreWebView2>,
                                          args: Option<
                                        ICoreWebView2DevToolsProtocolEventReceivedEventArgs,
                                    >| {
                                        if let Some(args) = args {
                                            let mut raw = PWSTR::null();
                                            // ParameterObjectAsJson writes a COM-allocated
                                            // (CoTaskMemAlloc) PWSTR; `take_pwstr` copies it into a
                                            // String and frees the COM buffer (no leak per event).
                                            if unsafe { args.ParameterObjectAsJson(&mut raw) }.is_ok()
                                            {
                                                let json = take_pwstr(raw);
                                                if let Some(entry) =
                                                    parse_event(&kind_owned, &json)
                                                {
                                                    buf.push(entry);
                                                }
                                            }
                                        }
                                        Ok(())
                                    },
                                ));
                            receiver.add_DevToolsProtocolEventReceived(&handler, &mut token)?;
                            // The receiver + token live for app lifetime (we never
                            // remove them); leaking the token is intentional.
                            let _ = token;
                        }
                        Ok(())
                    }
                };
                if let Err(e) = wire() {
                    eprintln!("visual debugger: event receiver wiring failed: {e}");
                }
            })
            .map_err(|e| format!("visual debugger: with_webview wiring failed: {e}"))?;

        Ok(())
    }

    /// Map a CDP event JSON payload to a [`ConsoleEntry`] for the given kind.
    fn parse_event(kind: &str, json: &str) -> Option<ConsoleEntry> {
        let params: Value = serde_json::from_str(json).ok()?;
        let received_at_utc = now_rfc3339();
        let timestamp = params.get("timestamp").and_then(Value::as_f64);

        let entry = match kind {
            "console" => {
                let level = params
                    .get("type")
                    .and_then(Value::as_str)
                    .unwrap_or("log")
                    .to_string();
                let text = params
                    .get("args")
                    .and_then(Value::as_array)
                    .map(|args| {
                        args.iter()
                            .filter_map(remote_object_text)
                            .collect::<Vec<_>>()
                            .join(" ")
                    })
                    .filter(|s| !s.trim().is_empty())
                    .unwrap_or_else(|| format!("console.{level}"));
                ConsoleEntry {
                    kind: kind.to_string(),
                    level,
                    text,
                    url: None,
                    timestamp,
                    received_at_utc,
                }
            }
            "exception" => {
                let details = params.get("exceptionDetails");
                let text = details
                    .and_then(|d| d.get("text").and_then(Value::as_str))
                    .or_else(|| {
                        details.and_then(|d| {
                            d.get("exception")
                                .and_then(|e| e.get("description"))
                                .and_then(Value::as_str)
                        })
                    })
                    .unwrap_or("uncaught exception")
                    .to_string();
                let url = details
                    .and_then(|d| d.get("url"))
                    .and_then(Value::as_str)
                    .map(str::to_string);
                ConsoleEntry {
                    kind: kind.to_string(),
                    level: "error".to_string(),
                    text,
                    url,
                    timestamp,
                    received_at_utc,
                }
            }
            "log" => {
                let log_entry = params.get("entry");
                let level = log_entry
                    .and_then(|e| e.get("level"))
                    .and_then(Value::as_str)
                    .unwrap_or("info")
                    .to_string();
                let text = log_entry
                    .and_then(|e| e.get("text"))
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_string();
                let url = log_entry
                    .and_then(|e| e.get("url"))
                    .and_then(Value::as_str)
                    .map(str::to_string);
                let ts = log_entry
                    .and_then(|e| e.get("timestamp"))
                    .and_then(Value::as_f64)
                    .or(timestamp);
                ConsoleEntry {
                    kind: kind.to_string(),
                    level,
                    text,
                    url,
                    timestamp: ts,
                    received_at_utc,
                }
            }
            "network_error" => {
                let text = params
                    .get("errorText")
                    .and_then(Value::as_str)
                    .unwrap_or("network loading failed")
                    .to_string();
                ConsoleEntry {
                    kind: kind.to_string(),
                    level: "error".to_string(),
                    text,
                    url: None,
                    timestamp,
                    received_at_utc,
                }
            }
            _ => return None,
        };
        Some(entry)
    }

    fn remote_object_text(value: &Value) -> Option<String> {
        value
            .get("value")
            .and_then(json_scalar_to_string)
            .or_else(|| {
                value
                    .get("description")
                    .and_then(Value::as_str)
                    .map(str::to_string)
            })
    }

    fn json_scalar_to_string(value: &Value) -> Option<String> {
        match value {
            Value::Null => Some("null".to_string()),
            Value::Bool(b) => Some(b.to_string()),
            Value::Number(n) => Some(n.to_string()),
            Value::String(s) => Some(s.clone()),
            other => Some(other.to_string()),
        }
    }

    // ---- Opt-in, no-activation visibility helper (NOT used by commands) --------

    /// Make a window's TOP-LEVEL host visible-but-quiet for compositor capture
    /// WITHOUT activating it, stealing focus, or changing Z-order.
    ///
    /// This is an explicit escape hatch for the rare case where `fromSurface`
    /// still yields blank frames (e.g. a fully minimized host). It is NOT invoked
    /// by any of the three commands; the default capture path touches nothing.
    ///
    /// Every Win32 call here uses no-activate semantics:
    /// - `WS_EX_NOACTIVATE` is OR-ed into the extended style so the window can
    ///   never be activated by user input.
    /// - `ShowWindow(.., SW_SHOWNOACTIVATE)` shows it WITHOUT activating.
    /// - `SetWindowPos(.., SWP_NOACTIVATE | SWP_NOZORDER | SWP_NOMOVE | SWP_NOSIZE)`
    ///   applies the new style without activating, re-ordering, moving, or resizing.
    #[allow(dead_code)]
    pub fn ensure_visible_no_activate<R: Runtime>(
        window: &tauri::WebviewWindow<R>,
    ) -> Result<(), String> {
        let (tx, rx) = mpsc::channel::<Result<(), String>>();
        window
            .with_webview(move |platform| {
                let result = unsafe {
                    let controller = platform.controller();
                    let mut child: HWND = HWND::default();
                    if controller.ParentWindow(&mut child).is_err() {
                        let _ = tx.send(Err("ParentWindow() failed".to_string()));
                        return;
                    }
                    // Walk to the top-level window so we toggle the host, not the
                    // WebView2 child.
                    let top = GetAncestor(child, GA_ROOT);
                    let hwnd = if top.0.is_null() { child } else { top };

                    if IsWindowVisible(hwnd).as_bool() {
                        let _ = tx.send(Ok(()));
                        return;
                    }

                    // OR in WS_EX_NOACTIVATE so the shown window can never steal
                    // activation.
                    let ex = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
                    let new_ex = ex | (WS_EX_NOACTIVATE.0 as isize);
                    SetWindowLongPtrW(hwnd, GWL_EXSTYLE, new_ex);

                    let _ = ShowWindow(hwnd, SW_SHOWNOACTIVATE);
                    let pos = SetWindowPos(
                        hwnd,
                        None,
                        0,
                        0,
                        0,
                        0,
                        SWP_NOACTIVATE | SWP_NOZORDER | SWP_NOMOVE | SWP_NOSIZE,
                    );
                    let _ = tx.send(pos.map_err(|e| e.to_string()));
                };
                let _ = result;
            })
            .map_err(|e| format!("with_webview failed: {e}"))?;
        rx.recv_timeout(CDP_CALL_TIMEOUT)
            .map_err(|_| "ensure_visible_no_activate timed out".to_string())?
    }
}

// ---- Cross-platform command surface ---------------------------------------

#[cfg(windows)]
pub use imp::{
    register_visual_debug_event_receivers, AxBounds, AxNode, AxTreeResult, ConsoleEntry,
    VisualCaptureResult, VisualDebugConsoleBuffer,
};

/// Capture a PNG screenshot of the target webview via in-process WebView2 CDP
/// `Page.captureScreenshot` (fromSurface + beyondViewport). Does NOT activate or
/// focus the window. Default target label is `"main"`.
#[cfg(windows)]
#[tauri::command]
pub async fn visual_debug_capture(
    app: tauri::AppHandle,
    window_label: Option<String>,
) -> Result<VisualCaptureResult, String> {
    // Run the blocking main-thread round-trip off the async worker so we don't
    // hold the IPC executor; the actual COM calls happen on the UI thread inside
    // `with_webview`.
    tauri::async_runtime::spawn_blocking(move || imp::visual_debug_capture_impl(app, window_label))
        .await
        .map_err(|e| e.to_string())?
}

/// Return a compact accessibility tree (`Accessibility.getFullAXTree` joined with
/// `DOMSnapshot.captureSnapshot` layout bounds on backendNodeId).
#[cfg(windows)]
#[tauri::command]
pub async fn visual_debug_ax_tree(
    app: tauri::AppHandle,
    window_label: Option<String>,
) -> Result<AxTreeResult, String> {
    tauri::async_runtime::spawn_blocking(move || imp::visual_debug_ax_tree_impl(app, window_label))
        .await
        .map_err(|e| e.to_string())?
}

/// Drain and return the buffered console / exception / log / network-error
/// entries collected by the startup-registered CDP event receivers.
#[cfg(windows)]
#[tauri::command]
pub fn visual_debug_console(
    buffer: tauri::State<'_, VisualDebugConsoleBuffer>,
) -> Result<Vec<ConsoleEntry>, String> {
    imp::visual_debug_console_impl(buffer)
}

// ---- Non-Windows stubs (keep the crate cross-platform) ---------------------

#[cfg(not(windows))]
mod stub {
    use serde::Serialize;

    #[derive(Debug, Clone, Serialize)]
    pub struct VisualCaptureResult {
        pub png_base64: String,
        pub width: u32,
        pub height: u32,
        pub captured_at_utc: String,
    }

    #[derive(Debug, Clone, Serialize)]
    pub struct AxBounds {
        pub x: f64,
        pub y: f64,
        pub w: f64,
        pub h: f64,
    }

    #[derive(Debug, Clone, Serialize)]
    pub struct AxNode {
        pub id: String,
        pub role: String,
        pub name: String,
        pub bounds: Option<AxBounds>,
    }

    #[derive(Debug, Clone, Serialize)]
    pub struct AxTreeResult {
        pub nodes: Vec<AxNode>,
        pub frame_id: Option<String>,
    }

    #[derive(Debug, Clone, Serialize)]
    pub struct ConsoleEntry {
        pub kind: String,
        pub level: String,
        pub text: String,
        pub url: Option<String>,
        pub timestamp: Option<f64>,
        pub received_at_utc: String,
    }

    /// Managed-state placeholder so `tauri::Builder::manage` calls compile on
    /// non-Windows hosts. It holds nothing.
    #[derive(Clone, Default)]
    pub struct VisualDebugConsoleBuffer;

    const NATIVE_ONLY: &str = "native visual capture is Windows/WebView2 only";

    pub fn register_visual_debug_event_receivers<R: tauri::Runtime>(
        _window: &tauri::WebviewWindow<R>,
        _buffer: VisualDebugConsoleBuffer,
    ) -> Result<(), String> {
        Err(NATIVE_ONLY.to_string())
    }
}

#[cfg(not(windows))]
pub use stub::{
    register_visual_debug_event_receivers, AxBounds, AxNode, AxTreeResult, ConsoleEntry,
    VisualCaptureResult, VisualDebugConsoleBuffer,
};

#[cfg(not(windows))]
#[tauri::command]
pub async fn visual_debug_capture(
    _app: tauri::AppHandle,
    _window_label: Option<String>,
) -> Result<VisualCaptureResult, String> {
    Err("native visual capture is Windows/WebView2 only".to_string())
}

#[cfg(not(windows))]
#[tauri::command]
pub async fn visual_debug_ax_tree(
    _app: tauri::AppHandle,
    _window_label: Option<String>,
) -> Result<AxTreeResult, String> {
    Err("native visual capture is Windows/WebView2 only".to_string())
}

#[cfg(not(windows))]
#[tauri::command]
pub fn visual_debug_console(
    _buffer: tauri::State<'_, VisualDebugConsoleBuffer>,
) -> Result<Vec<ConsoleEntry>, String> {
    Err("native visual capture is Windows/WebView2 only".to_string())
}
