import "@testing-library/jest-dom";

// WP-KERNEL-009 / MT-165 — jsdom polyfills so the bundled monaco-editor module
// can be IMPORTED under jsdom (its top-level code touches a few DOM APIs jsdom
// omits). This lets the embedded-code-block node's schema/serialization be unit
// tested without pulling a real browser; a full Monaco MOUNT still needs a real
// DOM (proven in the MT-176 Playwright offline spec), so these are import-time
// shims only, not a fake editor.
if (typeof document !== "undefined") {
  const doc = document as Document & {
    queryCommandSupported?: (commandId: string) => boolean;
  };
  if (typeof doc.queryCommandSupported !== "function") {
    doc.queryCommandSupported = () => false;
  }
}
if (typeof window !== "undefined") {
  const win = window as Window & {
    matchMedia?: (query: string) => MediaQueryList;
    ResizeObserver?: typeof ResizeObserver;
  };
  if (typeof win.matchMedia !== "function") {
    win.matchMedia = (query: string) =>
      ({
        matches: false,
        media: query,
        onchange: null,
        addListener: () => {},
        removeListener: () => {},
        addEventListener: () => {},
        removeEventListener: () => {},
        dispatchEvent: () => false,
      }) as unknown as MediaQueryList;
  }
  if (typeof win.ResizeObserver !== "function") {
    win.ResizeObserver = class {
      observe(): void {}
      unobserve(): void {}
      disconnect(): void {}
    } as unknown as typeof ResizeObserver;
  }

  // WP-KERNEL-009 iteration-3 (L25): jsdom's Range implements neither
  // getClientRects nor getBoundingClientRect, but ProseMirror's scroll-into-view
  // path (EditorView.coordsAtPos -> singleRect) calls them after every command
  // dispatch that scrolls (focus/insert/find). The resulting TypeError escaped
  // the dispatch as an UNCAUGHT exception — tests stayed green while dispatch
  // crashed mid-flight (false-positive risk flagged by the adversarial review).
  // Zero-rect polyfills keep the dispatch path intact under jsdom; real
  // coordinates are exercised in the Playwright browser lane.
  if (typeof Range !== "undefined") {
    const rangeProto = Range.prototype as Range & {
      getClientRects?: () => DOMRectList;
      getBoundingClientRect?: () => DOMRect;
    };
    if (typeof rangeProto.getClientRects !== "function") {
      rangeProto.getClientRects = () => [] as unknown as DOMRectList;
    }
    if (typeof rangeProto.getBoundingClientRect !== "function") {
      rangeProto.getBoundingClientRect = () =>
        ({ x: 0, y: 0, top: 0, left: 0, right: 0, bottom: 0, width: 0, height: 0, toJSON: () => ({}) }) as DOMRect;
    }
  }

  // Monaco's clipboard service calls navigator.clipboard.{write,read*}; jsdom
  // omits the Clipboard API, so its body-level paste/copy handlers throw
  // "Cannot read properties of undefined (reading 'write')" when a code-block
  // editor is mounted. Provide a no-op clipboard stub so the embedded editor's
  // schema/UI can be tested under jsdom (real clipboard behavior is exercised in
  // the MT-176 Playwright spec).
  const nav = win.navigator as Navigator & { clipboard?: unknown };
  if (!nav.clipboard) {
    Object.defineProperty(nav, "clipboard", {
      configurable: true,
      value: {
        write: async () => {},
        writeText: async () => {},
        read: async () => [],
        readText: async () => "",
      },
    });
  }
  // Monaco constructs `new ClipboardItem(...)` in its body copy handler; jsdom
  // omits the class. Provide a minimal stub so the handler does not throw a
  // synchronous ReferenceError on synthetic clicks during code-block tests.
  const g = globalThis as Record<string, unknown>;
  if (typeof g.ClipboardItem === "undefined") {
    g.ClipboardItem = class {
      constructor(public items: Record<string, unknown>) {}
    };
  }

  // WP-KERNEL-009 / MT-165 — Monaco schedules async clipboard/layout work that
  // it CANCELS when an editor is disposed; under jsdom (no canvas, fast
  // teardown) those cancellations surface as unhandled "Canceled" rejections.
  // They are benign Monaco lifecycle noise, NOT product errors. Swallow ONLY
  // Monaco's cancellation error so a real unhandled rejection still fails the
  // suite. (Full Monaco behavior is proven in the MT-176 Playwright spec.)
  win.addEventListener("unhandledrejection", (event) => {
    if (isMonacoCancellation((event as PromiseRejectionEvent).reason)) {
      event.preventDefault();
    }
  });
}

/** True for Monaco's benign disposal "Canceled" rejection (see note above). */
function isMonacoCancellation(reason: unknown): boolean {
  if (typeof reason === "string") return reason === "Canceled";
  if (!reason || typeof reason !== "object") return false;
  const r = reason as { name?: unknown; message?: unknown };
  return r.name === "Canceled" || r.name === "CancellationError" || r.message === "Canceled";
}

// Vitest surfaces Node-level unhandled rejections too; swallow ONLY Monaco's
// cancellation noise at the process boundary so a genuine unhandled rejection
// still fails the suite.
const nodeProcess = (
  globalThis as {
    process?: { on?: (event: string, listener: (reason: unknown) => void) => void };
  }
).process;
if (nodeProcess && typeof nodeProcess.on === "function") {
  nodeProcess.on("unhandledRejection", (reason: unknown) => {
    if (!isMonacoCancellation(reason)) throw reason;
  });
}
