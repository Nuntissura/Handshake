// WP-KERNEL-009 MT-254 DebugAdapterCore — debug session visual harness.
//
// Mounts the real DebugSidePanel (which mounts a real Monaco editor + breakpoint
// gutter and drives a live DebugSession over the product HTTP API). The offline
// Playwright spec sets window.__mt254DebugConfig from the fixture READY message,
// then drives the panel end-to-end against a REAL node debuggee.

import "../App.css";

import { createElement as h } from "react";
import { createRoot } from "react-dom/client";

import { DebugSidePanel } from "./DebugSidePanel";
import { setMonacoWorkerFactoryForTests } from "../lib/monaco/setup";

// In the offline Playwright harness the Vite `?worker` asset URLs
// (/assets/ts.worker-*.js) are not served by any origin, so Monaco's default
// getWorker would throw "not a valid URL". The breakpoint gutter + decorations
// the panel uses do NOT need language workers, so we install a same-origin
// blob worker (no network) to satisfy Monaco without spawning real language
// servers. This is the documented test seam, not a product code path.
const NOOP_WORKER_SOURCE = "self.onmessage = function () {};";
setMonacoWorkerFactoryForTests(() => {
  const blob = new Blob([NOOP_WORKER_SOURCE], { type: "application/javascript" });
  return new Worker(URL.createObjectURL(blob));
});

declare global {
  interface Window {
    __mt254DebugConfig?: {
      program: string;
      sourceUrl: string;
      sourceText: string;
    };
    __mt254HarnessMounted?: boolean;
  }
}

const config = window.__mt254DebugConfig;
const root = document.getElementById("harness-root");
if (root && config) {
  createRoot(root).render(
    h(DebugSidePanel, {
      adapter: "node",
      program: config.program,
      sourceUrl: config.sourceUrl,
      sourceText: config.sourceText,
      language: "javascript",
    }),
  );
  window.__mt254HarnessMounted = true;
}

export {};
