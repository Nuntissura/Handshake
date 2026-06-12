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
}
