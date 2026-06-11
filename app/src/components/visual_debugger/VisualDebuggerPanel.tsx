import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { isTauri } from "@tauri-apps/api/core";
import {
  pngDataUrl,
  visualDebugAxTree,
  visualDebugCapture,
  visualDebugConsole,
  type VisualAxNode,
  type VisualCapture,
  type VisualConsoleEntry,
} from "../../lib/ipc/visual_debugger";

// In-app VISUAL DEBUGGER VIEWER PANEL (frontend) for Handshake's built-in
// visual debugger. A model OR a human co-author can visually inspect the running
// app WITHOUT popping it up or stealing focus: browse captures, inspect the
// element/AX tree (click a node to highlight its bounds over the capture), read
// console / exception / network errors, and diff two captures side-by-side.
//
// All transport is read-only inspection via the focus-safe in-process commands
// (visual_debug_capture / visual_debug_ax_tree / visual_debug_console), wrapped
// in `lib/ipc/visual_debugger.ts`. Nothing here activates the window.
//
// This is the co-authoring / Handshake-Stage seed.

type SubTab = "capture" | "tree" | "console" | "diff";

const SUB_TABS: { id: SubTab; label: string }[] = [
  { id: "capture", label: "Capture" },
  { id: "tree", label: "Element / AX Tree" },
  { id: "console", label: "Console" },
  { id: "diff", label: "Diff" },
];

const MAX_CAPTURES = 20;

function shortTime(rfc3339: string): string {
  const d = new Date(rfc3339);
  return Number.isNaN(d.getTime()) ? rfc3339 : d.toLocaleTimeString();
}

function levelColor(level: string): string {
  switch (level) {
    case "error":
      return "#ff6b6b";
    case "warning":
      return "#ffb454";
    case "info":
      return "#54a0ff";
    case "debug":
      return "#9aa0a6";
    default:
      return "#c8ccd0";
  }
}

/**
 * A capture image with an optional highlighted AX bounds box overlaid. The
 * overlay is positioned in the image's intrinsic pixel space and scaled to the
 * rendered size, so the box tracks the element regardless of zoom / container
 * width.
 */
function CaptureWithOverlay({
  capture,
  highlight,
  testId,
}: {
  capture: VisualCapture;
  highlight?: VisualAxNode | null;
  testId?: string;
}) {
  const imgRef = useRef<HTMLImageElement>(null);
  const [rendered, setRendered] = useState<{ w: number; h: number }>({ w: 0, h: 0 });

  const measure = useCallback(() => {
    const el = imgRef.current;
    if (el) setRendered({ w: el.clientWidth, h: el.clientHeight });
  }, []);

  useEffect(() => {
    measure();
    window.addEventListener("resize", measure);
    return () => window.removeEventListener("resize", measure);
  }, [measure, capture.id]);

  const scaleX = capture.width > 0 ? rendered.w / capture.width : 1;
  const scaleY = capture.height > 0 ? rendered.h / capture.height : 1;
  const b = highlight?.bounds ?? null;

  return (
    <div
      style={{ position: "relative", display: "inline-block", maxWidth: "100%" }}
      data-stable-id={testId}
      data-testid={testId}
    >
      <img
        ref={imgRef}
        src={capture.dataUrl}
        alt={`capture ${capture.id}`}
        onLoad={measure}
        style={{
          maxWidth: "100%",
          height: "auto",
          display: "block",
          border: "1px solid var(--hsk-border, #2a2f36)",
          borderRadius: 4,
        }}
      />
      {b && (
        <div
          aria-hidden
          data-stable-id="visual-debugger.overlay-box"
          style={{
            position: "absolute",
            left: b.x * scaleX,
            top: b.y * scaleY,
            width: Math.max(1, b.w * scaleX),
            height: Math.max(1, b.h * scaleY),
            border: "2px solid #54a0ff",
            background: "rgba(84, 160, 255, 0.18)",
            pointerEvents: "none",
            boxSizing: "border-box",
          }}
        />
      )}
    </div>
  );
}

export function VisualDebuggerPanel() {
  const tauriAvailable = isTauri();

  const [subTab, setSubTab] = useState<SubTab>("capture");
  const [captures, setCaptures] = useState<VisualCapture[]>([]);
  const [selectedCaptureId, setSelectedCaptureId] = useState<string | null>(null);
  const [capturing, setCapturing] = useState(false);
  const [error, setError] = useState<string | null>(null);

  // AX tree
  const [axNodes, setAxNodes] = useState<VisualAxNode[]>([]);
  const [axLoading, setAxLoading] = useState(false);
  const [selectedAxId, setSelectedAxId] = useState<string | null>(null);
  const [axError, setAxError] = useState<string | null>(null);

  // Console
  const [consoleEntries, setConsoleEntries] = useState<VisualConsoleEntry[]>([]);
  const [consoleLoading, setConsoleLoading] = useState(false);
  const [consoleError, setConsoleError] = useState<string | null>(null);

  // Diff
  const [diffA, setDiffA] = useState<string | null>(null);
  const [diffB, setDiffB] = useState<string | null>(null);
  const [diffMode, setDiffMode] = useState<"side-by-side" | "overlay" | "pixel">("side-by-side");
  const [overlayShow, setOverlayShow] = useState<"a" | "b">("a");
  const diffCanvasRef = useRef<HTMLCanvasElement>(null);

  const captureCounter = useRef(0);

  const selectedCapture = useMemo(
    () => captures.find((c) => c.id === selectedCaptureId) ?? null,
    [captures, selectedCaptureId],
  );
  const selectedAxNode = useMemo(
    () => axNodes.find((n) => n.id === selectedAxId) ?? null,
    [axNodes, selectedAxId],
  );

  const doCapture = useCallback(async () => {
    setCapturing(true);
    setError(null);
    try {
      const result = await visualDebugCapture();
      captureCounter.current += 1;
      const capture: VisualCapture = {
        ...result,
        id: `capture-${captureCounter.current}`,
        dataUrl: pngDataUrl(result.png_base64),
      };
      setCaptures((prev) => [capture, ...prev].slice(0, MAX_CAPTURES));
      setSelectedCaptureId(capture.id);
      // Seed diff selectors as captures accrue.
      setDiffA((a) => a ?? capture.id);
      setDiffB((b) => (b === null && captures.length >= 1 ? captures[0].id : b));
    } catch (err) {
      setError(err instanceof Error ? err.message : "Capture failed");
    } finally {
      setCapturing(false);
    }
  }, [captures]);

  const loadAxTree = useCallback(async () => {
    setAxLoading(true);
    setAxError(null);
    try {
      const tree = await visualDebugAxTree();
      setAxNodes(tree.nodes);
      setSelectedAxId(null);
    } catch (err) {
      setAxNodes([]);
      setAxError(err instanceof Error ? err.message : "Failed to load AX tree");
    } finally {
      setAxLoading(false);
    }
  }, []);

  const loadConsole = useCallback(async () => {
    setConsoleLoading(true);
    setConsoleError(null);
    try {
      const entries = await visualDebugConsole();
      // Console drains on read; accumulate newest-first so the panel keeps
      // history across polls.
      setConsoleEntries((prev) => [...entries.reverse(), ...prev].slice(0, 1000));
    } catch (err) {
      setConsoleError(err instanceof Error ? err.message : "Failed to read console");
    } finally {
      setConsoleLoading(false);
    }
  }, []);

  // Pixel diff: draw A, then B with `difference` compositing onto a canvas.
  useEffect(() => {
    if (subTab !== "diff" || diffMode !== "pixel") return;
    const a = captures.find((c) => c.id === diffA);
    const b = captures.find((c) => c.id === diffB);
    const canvas = diffCanvasRef.current;
    if (!a || !b || !canvas) return;
    const ctx = canvas.getContext("2d");
    if (!ctx) return;
    const w = Math.min(a.width || 0, b.width || 0) || a.width || b.width;
    const h = Math.min(a.height || 0, b.height || 0) || a.height || b.height;
    if (w <= 0 || h <= 0) return;
    canvas.width = w;
    canvas.height = h;
    const imgA = new Image();
    const imgB = new Image();
    let loaded = 0;
    const onBoth = () => {
      if (++loaded < 2) return;
      ctx.clearRect(0, 0, w, h);
      ctx.globalCompositeOperation = "source-over";
      ctx.drawImage(imgA, 0, 0, w, h);
      ctx.globalCompositeOperation = "difference";
      ctx.drawImage(imgB, 0, 0, w, h);
      ctx.globalCompositeOperation = "source-over";
    };
    imgA.onload = onBoth;
    imgB.onload = onBoth;
    imgA.src = a.dataUrl;
    imgB.src = b.dataUrl;
  }, [subTab, diffMode, diffA, diffB, captures]);

  if (!tauriAvailable) {
    return (
      <div className="content-card" data-stable-id="visual-debugger.unavailable">
        <h2>Visual Debugger</h2>
        <p className="muted">
          The native visual debugger requires the Tauri desktop runtime (WebView2). It is
          unavailable in a plain browser context.
        </p>
      </div>
    );
  }

  const captureA = captures.find((c) => c.id === diffA) ?? null;
  const captureB = captures.find((c) => c.id === diffB) ?? null;

  return (
    <div className="content-card visual-debugger" data-stable-id="visual-debugger" data-testid="visual-debugger">
      <div
        style={{ display: "flex", alignItems: "center", justifyContent: "space-between", gap: 12 }}
      >
        <div>
          <h3 style={{ margin: 0 }}>Visual Debugger</h3>
          <p className="muted small" style={{ margin: "2px 0 0" }}>
            Focus-safe inspection of the running app (never activates / steals focus).
          </p>
        </div>
        <button
          type="button"
          className="debug-panel__refresh"
          onClick={doCapture}
          disabled={capturing}
          data-stable-id="visual-debugger.capture-button"
          data-testid="visual-debugger.capture-button"
        >
          {capturing ? "Capturing..." : "Capture"}
        </button>
      </div>

      {error && (
        <p className="muted" data-stable-id="visual-debugger.error">
          Error: {error}
        </p>
      )}

      <div
        className="main-pane__tabs"
        style={{ marginTop: 10, marginBottom: 10 }}
        data-stable-id="visual-debugger.subtabs"
      >
        {SUB_TABS.map((t) => (
          <button
            key={t.id}
            type="button"
            className={
              subTab === t.id ? "main-pane__tab main-pane__tab--active" : "main-pane__tab"
            }
            onClick={() => {
              setSubTab(t.id);
              if (t.id === "tree" && axNodes.length === 0 && !axLoading) void loadAxTree();
              if (t.id === "console" && consoleEntries.length === 0 && !consoleLoading)
                void loadConsole();
            }}
            data-stable-id={`visual-debugger.subtab.${t.id}`}
            data-testid={`visual-debugger.subtab.${t.id}`}
          >
            {t.label}
          </button>
        ))}
      </div>

      {/* Capture list (shared across sub-tabs) */}
      <div
        className="visual-debugger__captures"
        style={{ display: "flex", gap: 8, flexWrap: "wrap", marginBottom: 12 }}
        data-stable-id="visual-debugger.capture-list"
        data-testid="visual-debugger.capture-list"
      >
        {captures.length === 0 ? (
          <span className="muted small">No captures yet. Click Capture.</span>
        ) : (
          captures.map((c) => (
            <button
              key={c.id}
              type="button"
              className="main-button"
              onClick={() => setSelectedCaptureId(c.id)}
              aria-pressed={selectedCaptureId === c.id}
              style={{
                opacity: selectedCaptureId === c.id ? 1 : 0.7,
                borderColor: selectedCaptureId === c.id ? "#54a0ff" : undefined,
              }}
              data-stable-id={`visual-debugger.capture.${c.id}`}
              data-testid={`visual-debugger.capture.${c.id}`}
              data-capture-id={c.id}
            >
              {c.id} · {shortTime(c.captured_at_utc)} · {c.width}×{c.height}
            </button>
          ))
        )}
      </div>

      {subTab === "capture" && (
        <div data-stable-id="visual-debugger.capture-view">
          {selectedCapture ? (
            <CaptureWithOverlay capture={selectedCapture} testId="visual-debugger.capture-image" />
          ) : (
            <p className="muted">Select a capture above, or click Capture to take one.</p>
          )}
        </div>
      )}

      {subTab === "tree" && (
        <div
          data-stable-id="visual-debugger.tree-view"
          style={{ display: "grid", gridTemplateColumns: "minmax(280px, 40%) 1fr", gap: 12 }}
        >
          <div>
            <div className="drawer-actions" style={{ justifyContent: "flex-start", marginBottom: 8 }}>
              <button
                type="button"
                className="secondary"
                onClick={loadAxTree}
                disabled={axLoading}
                data-stable-id="visual-debugger.tree-refresh"
                data-testid="visual-debugger.tree-refresh"
              >
                {axLoading ? "Loading..." : "Refresh AX tree"}
              </button>
              <span className="muted small">{axNodes.length} nodes</span>
            </div>
            {axError && <p className="muted">Error: {axError}</p>}
            <div
              className="debug-panel__logbox"
              style={{ maxHeight: 420, overflow: "auto" }}
              data-stable-id="visual-debugger.tree-list"
              data-testid="visual-debugger.tree-list"
            >
              {axNodes.length === 0 ? (
                <p className="muted">No AX nodes. Click "Refresh AX tree".</p>
              ) : (
                axNodes.map((n) => (
                  <button
                    key={n.id || `${n.role}-${n.name}`}
                    type="button"
                    onClick={() => setSelectedAxId(n.id)}
                    title={n.bounds ? `${n.bounds.x},${n.bounds.y} ${n.bounds.w}×${n.bounds.h}` : "no bounds"}
                    style={{
                      display: "block",
                      width: "100%",
                      textAlign: "left",
                      padding: "3px 6px",
                      border: "none",
                      borderLeft:
                        selectedAxId === n.id ? "3px solid #54a0ff" : "3px solid transparent",
                      background: selectedAxId === n.id ? "rgba(84,160,255,0.12)" : "transparent",
                      color: "inherit",
                      cursor: n.bounds ? "pointer" : "default",
                      fontFamily: "var(--hsk-mono, monospace)",
                      fontSize: 12,
                    }}
                    data-stable-id={`visual-debugger.ax-node.${n.id}`}
                    data-testid={`visual-debugger.ax-node.${n.id}`}
                    data-ax-id={n.id}
                  >
                    <strong>{n.role || "—"}</strong>
                    {n.name ? ` · ${n.name}` : ""}
                    {n.bounds ? (
                      <span className="muted">
                        {" "}
                        · {Math.round(n.bounds.x)},{Math.round(n.bounds.y)} {Math.round(n.bounds.w)}×
                        {Math.round(n.bounds.h)}
                      </span>
                    ) : (
                      <span className="muted"> · no bounds</span>
                    )}
                  </button>
                ))
              )}
            </div>
          </div>
          <div data-stable-id="visual-debugger.tree-overlay">
            {selectedCapture ? (
              <CaptureWithOverlay
                capture={selectedCapture}
                highlight={selectedAxNode}
                testId="visual-debugger.tree-overlay-image"
              />
            ) : (
              <p className="muted">
                Capture the app first to overlay AX bounds on the screenshot.
              </p>
            )}
          </div>
        </div>
      )}

      {subTab === "console" && (
        <div data-stable-id="visual-debugger.console-view">
          <div className="drawer-actions" style={{ justifyContent: "flex-start", marginBottom: 8 }}>
            <button
              type="button"
              className="secondary"
              onClick={loadConsole}
              disabled={consoleLoading}
              data-stable-id="visual-debugger.console-refresh"
              data-testid="visual-debugger.console-refresh"
            >
              {consoleLoading ? "Polling..." : "Poll console"}
            </button>
            <button
              type="button"
              className="secondary"
              onClick={() => setConsoleEntries([])}
              data-stable-id="visual-debugger.console-clear"
            >
              Clear
            </button>
            <span className="muted small">{consoleEntries.length} entries</span>
          </div>
          {consoleError && <p className="muted">Error: {consoleError}</p>}
          <div
            className="debug-panel__logbox"
            style={{ maxHeight: 460, overflow: "auto" }}
            data-stable-id="visual-debugger.console-list"
            data-testid="visual-debugger.console-list"
          >
            {consoleEntries.length === 0 ? (
              <p className="muted">No console activity buffered. Click "Poll console".</p>
            ) : (
              consoleEntries.map((e, i) => (
                <pre
                  key={`${e.received_at_utc}-${i}`}
                  className="debug-panel__line"
                  style={{ whiteSpace: "pre-wrap", margin: 0, borderLeft: `3px solid ${levelColor(e.level)}`, paddingLeft: 6 }}
                  data-console-kind={e.kind}
                  data-console-level={e.level}
                >
                  <span className="muted">{shortTime(e.received_at_utc)}</span>{" "}
                  <strong style={{ color: levelColor(e.level) }}>
                    [{e.kind}/{e.level}]
                  </strong>{" "}
                  {e.text}
                  {e.url ? <span className="muted"> ({e.url})</span> : null}
                </pre>
              ))
            )}
          </div>
        </div>
      )}

      {subTab === "diff" && (
        <div data-stable-id="visual-debugger.diff-view">
          <div
            className="drawer-actions"
            style={{ justifyContent: "flex-start", marginBottom: 10, flexWrap: "wrap" }}
          >
            <label className="muted small">
              A:{" "}
              <select
                value={diffA ?? ""}
                onChange={(ev) => setDiffA(ev.target.value || null)}
                data-stable-id="visual-debugger.diff-select-a"
                data-testid="visual-debugger.diff-select-a"
              >
                <option value="">—</option>
                {captures.map((c) => (
                  <option key={c.id} value={c.id}>
                    {c.id} · {shortTime(c.captured_at_utc)}
                  </option>
                ))}
              </select>
            </label>
            <label className="muted small">
              B:{" "}
              <select
                value={diffB ?? ""}
                onChange={(ev) => setDiffB(ev.target.value || null)}
                data-stable-id="visual-debugger.diff-select-b"
                data-testid="visual-debugger.diff-select-b"
              >
                <option value="">—</option>
                {captures.map((c) => (
                  <option key={c.id} value={c.id}>
                    {c.id} · {shortTime(c.captured_at_utc)}
                  </option>
                ))}
              </select>
            </label>
            <div className="main-pane__tabs" data-stable-id="visual-debugger.diff-mode">
              {(["side-by-side", "overlay", "pixel"] as const).map((m) => (
                <button
                  key={m}
                  type="button"
                  className={diffMode === m ? "main-pane__tab main-pane__tab--active" : "main-pane__tab"}
                  onClick={() => setDiffMode(m)}
                  data-stable-id={`visual-debugger.diff-mode.${m}`}
                  data-testid={`visual-debugger.diff-mode.${m}`}
                >
                  {m === "side-by-side" ? "Side-by-side" : m === "overlay" ? "A/B overlay" : "Pixel diff"}
                </button>
              ))}
            </div>
          </div>

          {!captureA || !captureB ? (
            <p className="muted">Pick two captures (A and B) to diff.</p>
          ) : diffMode === "side-by-side" ? (
            <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 12 }}>
              <div data-stable-id="visual-debugger.diff-a">
                <p className="muted small" style={{ margin: "0 0 4px" }}>
                  A · {captureA.id} · {shortTime(captureA.captured_at_utc)}
                </p>
                <CaptureWithOverlay capture={captureA} testId="visual-debugger.diff-image-a" />
              </div>
              <div data-stable-id="visual-debugger.diff-b">
                <p className="muted small" style={{ margin: "0 0 4px" }}>
                  B · {captureB.id} · {shortTime(captureB.captured_at_utc)}
                </p>
                <CaptureWithOverlay capture={captureB} testId="visual-debugger.diff-image-b" />
              </div>
            </div>
          ) : diffMode === "overlay" ? (
            <div data-stable-id="visual-debugger.diff-overlay">
              <div className="drawer-actions" style={{ justifyContent: "flex-start", marginBottom: 8 }}>
                <button
                  type="button"
                  className="secondary"
                  onClick={() => setOverlayShow((s) => (s === "a" ? "b" : "a"))}
                  data-stable-id="visual-debugger.diff-overlay-toggle"
                  data-testid="visual-debugger.diff-overlay-toggle"
                >
                  Showing {overlayShow.toUpperCase()} — toggle
                </button>
              </div>
              <CaptureWithOverlay
                capture={overlayShow === "a" ? captureA : captureB}
                testId="visual-debugger.diff-overlay-image"
              />
            </div>
          ) : (
            <div data-stable-id="visual-debugger.diff-pixel">
              <p className="muted small" style={{ margin: "0 0 4px" }}>
                Pixel difference (bright = changed). Compared at{" "}
                {Math.min(captureA.width, captureB.width)}×{Math.min(captureA.height, captureB.height)}.
              </p>
              <canvas
                ref={diffCanvasRef}
                style={{
                  maxWidth: "100%",
                  height: "auto",
                  border: "1px solid var(--hsk-border, #2a2f36)",
                  borderRadius: 4,
                  background: "#000",
                }}
                data-stable-id="visual-debugger.diff-pixel-canvas"
                data-testid="visual-debugger.diff-pixel-canvas"
              />
            </div>
          )}
        </div>
      )}
    </div>
  );
}

export default VisualDebuggerPanel;
