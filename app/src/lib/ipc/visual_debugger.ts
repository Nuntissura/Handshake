import { invoke } from "@tauri-apps/api/core";

// IPC bindings for the NATIVE, focus-safe visual debugger surface.
//
// PRIMARY transport: in-process WebView2 CDP commands registered in
// `app/src-tauri/src/commands/visual_debugger.rs` and wired in
// `app/src-tauri/src/lib.rs` (visual_debug_capture / visual_debug_ax_tree /
// visual_debug_console). These drive the LIVE `ICoreWebView2` directly, so they
// never pop a window, never steal focus, and never change Z-order
// ([GLOBAL-BUILD-QUIET], HBR-QUIET). `Page.captureScreenshot { fromSurface }`
// reads the compositor surface, so capture works even when the host window is
// off-screen or occluded.
//
// FALLBACK transport: the legacy CDP-over-websocket commands in
// `app/src-tauri/src/visual_debug.rs` (kernel_visual_debug_screenshot), used
// only if the in-process capture command is unavailable. The fallback returns
// raw PNG bytes (Vec<u8> -> number[]) which we re-encode to base64 so the rest
// of the panel sees a single shape.
//
// The wrappers are read-only inspection: nothing here writes app state or
// activates the window.

/** Layout bounds for an AX node, in CSS pixels relative to the captured page. */
export interface VisualAxBounds {
  x: number;
  y: number;
  w: number;
  h: number;
}

/** One accessibility-tree node (role / name / optional layout bounds). */
export interface VisualAxNode {
  /** Stable AX node id (string-normalized across CDP/Edge versions). */
  id: string;
  role: string;
  name: string;
  bounds: VisualAxBounds | null;
}

/** Result of `visual_debug_ax_tree`. */
export interface VisualAxTreeResult {
  nodes: VisualAxNode[];
  frame_id: string | null;
}

/** Result of `visual_debug_capture`: a base64 PNG plus its pixel dimensions. */
export interface VisualCaptureResult {
  /** base64-encoded PNG payload (no `data:` prefix). */
  png_base64: string;
  width: number;
  height: number;
  /** RFC3339 wall-clock time the capture was taken. */
  captured_at_utc: string;
}

/**
 * One buffered console / exception / log / network-error entry, drained from the
 * backend ring buffer. `kind` is `console | exception | log | network_error`.
 */
export interface VisualConsoleEntry {
  kind: "console" | "exception" | "log" | "network_error" | string;
  /** CDP level: log / warning / error / info / debug. */
  level: string;
  text: string;
  url: string | null;
  /** CDP timestamp (ms since epoch as a float) when present. */
  timestamp: number | null;
  /** Wall-clock RFC3339 time the entry was buffered. */
  received_at_utc: string;
}

/** A capture plus a client-side id, for the panel's capture list. */
export interface VisualCapture extends VisualCaptureResult {
  /** Client-assigned stable id (capture-N) used for selection / diff. */
  id: string;
  /** Ready-to-use `data:` URL derived from `png_base64`. */
  dataUrl: string;
}

const PNG_DATA_PREFIX = "data:image/png;base64,";

/** Build a `data:` URL for a base64 PNG payload. */
export function pngDataUrl(pngBase64: string): string {
  return `${PNG_DATA_PREFIX}${pngBase64}`;
}

/** Encode raw PNG bytes (legacy fallback shape) to a base64 string. */
function bytesToBase64(bytes: number[] | Uint8Array): string {
  const arr = bytes instanceof Uint8Array ? bytes : Uint8Array.from(bytes);
  let binary = "";
  const chunk = 0x8000;
  for (let i = 0; i < arr.length; i += chunk) {
    binary += String.fromCharCode(...arr.subarray(i, i + chunk));
  }
  return btoa(binary);
}

/** Read PNG width/height from the IHDR chunk (bytes 16..24, big-endian). */
function pngDimensions(bytes: Uint8Array): { width: number; height: number } {
  if (bytes.length < 24) return { width: 0, height: 0 };
  const dv = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength);
  return { width: dv.getUint32(16), height: dv.getUint32(20) };
}

/**
 * Capture a focus-safe PNG of the target webview. Tries the in-process
 * `visual_debug_capture` first; on failure falls back to the legacy
 * `kernel_visual_debug_screenshot` (full-page) and normalizes its byte payload
 * into the same {@link VisualCaptureResult} shape.
 */
export async function visualDebugCapture(windowLabel?: string): Promise<VisualCaptureResult> {
  try {
    return await invoke<VisualCaptureResult>("visual_debug_capture", { windowLabel });
  } catch (primaryError) {
    try {
      const bytes = await invoke<number[]>("kernel_visual_debug_screenshot", {
        scope: { kind: "full" },
      });
      const arr = Uint8Array.from(bytes);
      const { width, height } = pngDimensions(arr);
      return {
        png_base64: bytesToBase64(arr),
        width,
        height,
        captured_at_utc: new Date().toISOString(),
      };
    } catch (fallbackError) {
      const primary = primaryError instanceof Error ? primaryError.message : String(primaryError);
      const fallback =
        fallbackError instanceof Error ? fallbackError.message : String(fallbackError);
      throw new Error(`visual_debug_capture failed (${primary}); fallback failed (${fallback})`);
    }
  }
}

/** Fetch the compact accessibility tree (role / name / bounds per node). */
export async function visualDebugAxTree(windowLabel?: string): Promise<VisualAxTreeResult> {
  return invoke<VisualAxTreeResult>("visual_debug_ax_tree", { windowLabel });
}

/**
 * Drain and return the buffered console / exception / log / network-error
 * entries. Draining clears the backend buffer, so each call returns only new
 * activity since the previous call ("tail since last poll").
 */
export async function visualDebugConsole(): Promise<VisualConsoleEntry[]> {
  return invoke<VisualConsoleEntry[]>("visual_debug_console");
}
