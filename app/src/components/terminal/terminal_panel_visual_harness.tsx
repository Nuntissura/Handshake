// Real-component mount harness for the Playwright visual matrix.
//
// MEDIUM-defect remediation: the visual gate previously asserted readability
// against a HAND-AUTHORED static HTML fixture (a mockup of the panel DOM),
// violating the operator's "No mockups / test via Handshake's own tools" rule.
//
// This harness mounts the GENUINE <TerminalPanel> React component (the same
// module shipped in the app) into a real Chromium page via react-dom. xterm
// needs a canvas/WebGL surface that is awkward under the screenshot harness, so
// we inject a fake `renderTerminal` that produces a faithful, deterministic
// terminal SURFACE (monospace output block) — but everything else (the
// Disclosure host, lazy/collapsed gating, the swarm-grouped tablist, the AiJob
// read-only badge + honest-disabled Take-control gate, the active-session
// toolbar) is the REAL component's own render output, not hand-written HTML.
//
// A fake in-memory TerminalIpc supplies a deterministic captured-session list so
// the layout is stable for the PNG baseline. The bundle is built at Playwright
// global-setup time by esbuild and injected into the page; the spec then drives
// the real component (open the disclosure, assert readability against the
// genuine render).
import { createElement as h, type ReactNode } from "react";
import { createRoot } from "react-dom/client";

import { TerminalPanel } from "./TerminalPanel";
import type {
  ScrollbackSnapshot,
  TerminalIpc,
  TerminalSession,
} from "../../lib/ipc/terminal";

const SESSIONS: TerminalSession[] = [
  {
    sessionId: "s1",
    sessionType: "AiJob",
    swarmId: "alpha",
    worktreeId: null,
    instanceId: "official-cli-0",
    title: "captured cloud cli",
    exited: false,
    exitCode: null,
    // Inspect-only: backend has NOT granted interaction -> Take-control is
    // honestly DISABLED in the real render (TERM-INVARIANTS at the surface).
    interactiveAllowed: false,
  },
  {
    sessionId: "s2",
    sessionType: "HumanDev",
    swarmId: "alpha",
    worktreeId: null,
    instanceId: null,
    title: "dev shell",
    exited: false,
    exitCode: null,
    interactiveAllowed: true,
  },
];

const SCROLLBACK: ScrollbackSnapshot = {
  sessionId: "s1",
  seq: 0,
  chunkBase64: "",
  truncated: false,
};

const fakeIpc: TerminalIpc = {
  createSession: async () => SESSIONS[0],
  writeStdin: async () => {},
  resizeSession: async () => {},
  closeSession: async () => {},
  listSessions: async () => SESSIONS,
  scrollback: async () => SCROLLBACK,
  subscribe: async () => () => {},
};

// Deterministic terminal surface stand-in for xterm (canvas-free) so the PNG
// baseline is stable. This is the ONLY hand-rendered piece; it is injected via
// the component's own `renderTerminal` seam (the same seam the unit tests use),
// not a replacement for the panel DOM.
function fakeRenderTerminal({
  session,
}: {
  session: TerminalSession;
  readOnly: boolean;
  ipc: TerminalIpc;
}): ReactNode {
  return h(
    "div",
    {
      "data-testid": `fake-term-${session.sessionId}`,
      style: {
        background: "#000",
        color: "#e5e7eb",
        fontFamily: "ui-monospace, Consolas, monospace",
        fontSize: 13,
        padding: 8,
        minHeight: 240,
        whiteSpace: "pre-wrap",
      },
    },
    "$ official-cli run --task build\ncompiling handshake_core ... ok\nsee app/src-tauri/src/lib.rs:42",
  );
}

declare global {
  interface Window {
    // Set by the spec BEFORE the bundle runs to choose the mount mode:
    //   "open"      -> render the real panel expanded (readability/baseline)
    //   "collapsed" -> render the real panel collapsed-by-default (lazy gate)
    __HARNESS_MODE__?: "open" | "collapsed";
  }
}

const root = document.getElementById("harness-root");
if (root) {
  const open = window.__HARNESS_MODE__ !== "collapsed";
  createRoot(root).render(
    h(TerminalPanel, {
      ipc: fakeIpc,
      renderTerminal: fakeRenderTerminal,
      // `open` mode mounts expanded so the visual readability assertions see the
      // real opened panel body; `collapsed` mode proves the genuine component is
      // collapsed-by-default + lazy (body not mounted) using the real Disclosure
      // host, not a hand-authored fixture.
      defaultOpen: open,
    }),
  );
}

export {};
