import { createElement as h } from "react";
import { createRoot } from "react-dom/client";

import { TerminalView } from "./TerminalView";
import type { ScrollbackSnapshot, TerminalSubscription } from "../../lib/ipc/terminal";

declare global {
  interface Window {
    __TERMINAL_VIEW_HARNESS_MODE__?: "interactive" | "readonly";
    __terminalInputLog?: string[];
  }
}

function encodeAscii(value: string): string {
  return btoa(value);
}

const SCROLLBACK: ScrollbackSnapshot = {
  sessionId: "xterm-proof",
  seq: 0,
  chunkBase64: encodeAscii("HANDSHAKE_XTERM_SCROLLBACK\r\n$ "),
  truncated: false,
};

async function fetchScrollback(sessionId: string): Promise<ScrollbackSnapshot> {
  return { ...SCROLLBACK, sessionId };
}

async function subscribe(sub: TerminalSubscription): Promise<() => void> {
  void sub;
  return () => {};
}

const root = document.getElementById("harness-root");
if (root) {
  const readOnly = window.__TERMINAL_VIEW_HARNESS_MODE__ === "readonly";
  window.__terminalInputLog = [];
  createRoot(root).render(
    h(TerminalView, {
      sessionId: "xterm-proof",
      readOnly,
      fetchScrollback,
      subscribe,
      onInput: (data: string) => {
        window.__terminalInputLog?.push(data);
      },
      onResize: () => {},
    }),
  );
}

export {};
