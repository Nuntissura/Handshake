import { useEffect, useRef } from "react";
import { Terminal } from "@xterm/xterm";
import { FitAddon } from "@xterm/addon-fit";
import { WebLinksAddon } from "@xterm/addon-web-links";
import { SearchAddon } from "@xterm/addon-search";
import "@xterm/xterm/css/xterm.css";

import type { ScrollbackSnapshot, TerminalSubscription } from "../../lib/ipc/terminal";
import { decodeChunk } from "../../lib/ipc/terminal";

// TerminalView: the ONLY module that touches xterm.js. Everything that needs a
// real canvas/WebGL surface lives here so the rest of the panel (tab logic,
// AiJob read-only gating, grouping) stays pure and unit-testable under jsdom,
// which has no canvas. The Playwright real-browser matrix exercises the genuine
// xterm render; vitest mocks this whole component.
//
// Hardening baked in here:
//  - scrollback cap (xterm `scrollback`) bounds renderer memory under a flood.
//  - FitAddon + a resize callback keep cols/rows synced to the backend PTY.
//  - WebLinksAddon linkifies file paths / urls (TERM-V1-SCOPE linkification).
//  - SearchAddon is wired so the panel can offer find-in-scrollback.
//  - stdin (onData) is wired ONLY when `readOnly` is false: TERM-INVARIANTS, the
//    AI/capture sessions never receive keystrokes unless the panel has resolved
//    the Take-control gate and the backend allows it.
//  - cleanup disposes the Terminal and calls the unlisten returned by subscribe,
//    so a collapsed/unmounted disclosure leaks neither the xterm instance nor
//    the IPC listener.

export interface TerminalViewProps {
  sessionId: string;
  /** When true, stdin is NOT wired (inspect-only). */
  readOnly: boolean;
  /** Max scrollback lines (flood bound + truncation behavior). */
  scrollbackLines?: number;
  /** Fetch the capped scrollback to (re)seed the buffer on mount + resync. */
  fetchScrollback: (sessionId: string) => Promise<ScrollbackSnapshot>;
  /** Subscribe to the live byte stream; returns an unlisten. */
  subscribe: (sub: TerminalSubscription) => Promise<() => void>;
  /** Send keystrokes to stdin (only invoked when !readOnly). */
  onInput?: (data: string) => void;
  /** Report new cols/rows after a fit so the panel can resize the PTY. */
  onResize?: (cols: number, rows: number) => void;
}

const DEFAULT_SCROLLBACK = 5000;
const TRUNCATION_MARKER = "\x1b[2m…(scrollback truncated)\x1b[0m\r\n";

export function TerminalView({
  sessionId,
  readOnly,
  scrollbackLines = DEFAULT_SCROLLBACK,
  fetchScrollback,
  subscribe,
  onInput,
  onResize,
}: TerminalViewProps) {
  const hostRef = useRef<HTMLDivElement | null>(null);

  useEffect(() => {
    const host = hostRef.current;
    if (!host) return;

    let alive = true;
    let unlisten: (() => void) | undefined;

    const term = new Terminal({
      scrollback: scrollbackLines,
      convertEol: false,
      disableStdin: readOnly,
      fontFamily: "ui-monospace, SFMono-Regular, Menlo, Consolas, monospace",
      fontSize: 13,
    });
    const fit = new FitAddon();
    const search = new SearchAddon();
    term.loadAddon(fit);
    term.loadAddon(new WebLinksAddon());
    term.loadAddon(search);
    term.open(host);
    try {
      fit.fit();
    } catch {
      // jsdom / zero-size host: fit is best-effort.
    }
    onResize?.(term.cols, term.rows);

    // stdin wiring is the TERM-INVARIANTS boundary: only when interactive.
    if (!readOnly && onInput) {
      term.onData((data) => onInput(data));
    }

    const seedFromScrollback = (snap: ScrollbackSnapshot) => {
      term.reset();
      if (snap.truncated) term.write(TRUNCATION_MARKER);
      if (snap.chunkBase64) term.write(decodeChunk(snap.chunkBase64));
    };

    void (async () => {
      try {
        const snap = await fetchScrollback(sessionId);
        if (!alive) return;
        seedFromScrollback(snap);
      } catch {
        // No scrollback yet (fresh session) is fine.
      }
    })();

    void subscribe({
      onOutput: (sid, bytes) => {
        if (sid !== sessionId) return;
        term.write(bytes);
      },
      onExit: (sid, code) => {
        if (sid !== sessionId) return;
        term.write(`\r\n\x1b[2m[process exited${code === null ? "" : ` with code ${code}`}]\x1b[0m\r\n`);
      },
      onResync: (sid) => {
        if (sid !== sessionId) return;
        // Reset then rewrite the capped scrollback so a dropped-bytes window
        // never leaves the buffer wedged on a partial control sequence.
        void (async () => {
          try {
            const snap = await fetchScrollback(sessionId);
            if (!alive) return;
            seedFromScrollback(snap);
          } catch {
            /* ignore */
          }
        })();
      },
    }).then((un) => {
      if (alive) unlisten = un;
      else un();
    });

    const onWindowResize = () => {
      try {
        fit.fit();
        onResize?.(term.cols, term.rows);
      } catch {
        /* best-effort */
      }
    };
    window.addEventListener("resize", onWindowResize);

    return () => {
      alive = false;
      window.removeEventListener("resize", onWindowResize);
      unlisten?.();
      term.dispose();
    };
    // sessionId/readOnly identify the session+capability; the callbacks are
    // stable per-session from the panel. Re-running on those is correct.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [sessionId, readOnly, scrollbackLines]);

  return (
    <div
      ref={hostRef}
      className="terminal-view"
      data-testid={`terminal-view-${sessionId}`}
      data-readonly={readOnly ? "true" : "false"}
      style={{ width: "100%", height: "100%", minHeight: 240 }}
    />
  );
}
