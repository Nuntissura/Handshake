// WP-KERNEL-009 MT-254 DebugAdapterCore — debug side panel.
//
// The operator-facing debug surface, rendered INSIDE Handshake. It mounts a
// REAL Monaco editor showing the script source with the breakpoint gutter
// (glyph-margin toggle via the pure helpers in lib/monaco/breakpoint_gutter),
// and drives a live DebugSession over the product HTTP API against a REAL
// debuggee process:
//
//   * the adapter picker shows ONLY runnable adapters (honesty gate),
//   * clicking the glyph margin toggles a breakpoint (real CDP binding on launch),
//   * Launch starts the real process; on the breakpoint hit the panel renders the
//     paused call stack + the real local variables, the current-stop line is
//     decorated in Monaco, and the embedded DebugConsole can evaluate in the frame,
//   * Step / Continue / Terminate drive the real session.
//
// No mock: stack frames, variables, eval results, and the exit code all come
// from the running debuggee. The gutter helpers are wired onto a real Monaco
// instance so their decorations/toggle render in a browser.

import { useCallback, useEffect, useMemo, useRef, useState } from "react";

import { createConfiguredEditor, monaco } from "../lib/monaco/setup";
import {
  BREAKPOINT_GLYPH_CLASS,
  BREAKPOINT_UNVERIFIED_GLYPH_CLASS,
  CURRENT_STOP_LINE_CLASS,
  buildBreakpointDecorations,
  buildCurrentStopDecoration,
  resolveGutterToggleLine,
  type GutterBreakpoint,
} from "../lib/monaco/breakpoint_gutter";
import {
  DebugSession,
  launchSession,
  loadRunnableAdapters,
  type StackFrame,
  type Variable,
} from "../lib/debug/dap_client";
import type { DebugAdapterDescriptor, DebugAdapterKind } from "../lib/api";
import { DebugConsole, type DebugConsoleEntry } from "./DebugConsole";

type Editor = monaco.editor.IStandaloneCodeEditor;

export type DebugSidePanelProps = {
  /** Adapter to launch (defaults to the first runnable adapter). */
  adapter?: DebugAdapterKind;
  /** Absolute path to the program to debug. */
  program: string;
  /** Source url the script is reported under (for breakpoint binding + gutter). */
  sourceUrl: string;
  /** The source text shown in the gutter editor. */
  sourceText: string;
  /** Monaco language id for the source editor. */
  language?: string;
};

export function DebugSidePanel({
  adapter,
  program,
  sourceUrl,
  sourceText,
  language = "javascript",
}: DebugSidePanelProps) {
  const hostRef = useRef<HTMLDivElement>(null);
  const editorRef = useRef<Editor | null>(null);
  const decorationsRef = useRef<string[]>([]);
  const sessionRef = useRef<DebugSession | null>(null);
  const pollTimerRef = useRef<ReturnType<typeof setInterval> | null>(null);

  const [adapters, setAdapters] = useState<DebugAdapterDescriptor[]>([]);
  const [selectedAdapter, setSelectedAdapter] = useState<DebugAdapterKind | "">(adapter ?? "");
  const [breakpointLines, setBreakpointLines] = useState<number[]>([]);
  const [verifiedLines, setVerifiedLines] = useState<Set<number>>(new Set());
  const [currentStopLine, setCurrentStopLine] = useState<number | null>(null);
  const [status, setStatus] = useState<"idle" | "running" | "paused" | "terminated">("idle");
  const [frames, setFrames] = useState<StackFrame[]>([]);
  const [variables, setVariables] = useState<Variable[]>([]);
  const [consoleEntries, setConsoleEntries] = useState<DebugConsoleEntry[]>([]);
  const [exitCode, setExitCode] = useState<number | null>(null);
  const [error, setError] = useState<string | null>(null);

  const topFrame = frames[0];

  // Load the runnable adapters (honesty gate: throws if any are non-runnable).
  useEffect(() => {
    let cancelled = false;
    loadRunnableAdapters()
      .then((loaded) => {
        if (cancelled) return;
        setAdapters(loaded);
        if (!adapter && loaded.length > 0) setSelectedAdapter(loaded[0].kind);
      })
      .catch((cause) => {
        if (!cancelled) setError(cause instanceof Error ? cause.message : String(cause));
      });
    return () => {
      cancelled = true;
    };
  }, [adapter]);

  // Mount the REAL Monaco editor with the breakpoint gutter.
  useEffect(() => {
    const host = hostRef.current;
    if (!host || editorRef.current) return;
    let instance: Editor | null = null;
    try {
      instance = createConfiguredEditor({
        container: host,
        value: sourceText,
        language,
        readOnly: true,
        glyphMargin: true,
        lineNumbers: "on",
        minimap: { enabled: false },
        fontSize: 13,
        scrollBeyondLastLine: false,
      });
      editorRef.current = instance;
      // Glyph-margin click toggles a breakpoint at that line (pure resolver).
      const mouseDown = instance.onMouseDown((event) => {
        const line = resolveGutterToggleLine({
          type: event.target.type,
          position: event.target.position
            ? { lineNumber: event.target.position.lineNumber }
            : null,
        });
        if (line === null) return;
        setBreakpointLines((current) =>
          current.includes(line)
            ? current.filter((l) => l !== line)
            : [...current, line].sort((a, b) => a - b),
        );
      });
      return () => {
        mouseDown.dispose();
        instance?.dispose();
        editorRef.current = null;
      };
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : String(cause));
      return;
    }
    // Mount once.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  // Re-render gutter decorations whenever breakpoints / current stop change.
  useEffect(() => {
    const instance = editorRef.current;
    if (!instance) return;
    const gutterBreakpoints: GutterBreakpoint[] = breakpointLines.map((line) => ({
      line,
      verified: verifiedLines.has(line),
    }));
    const next = [
      ...buildBreakpointDecorations(monaco, gutterBreakpoints),
      ...buildCurrentStopDecoration(monaco, currentStopLine),
    ];
    decorationsRef.current = instance.deltaDecorations(decorationsRef.current, next);
  }, [breakpointLines, verifiedLines, currentStopLine]);

  const appendConsole = useCallback((entry: DebugConsoleEntry) => {
    setConsoleEntries((entries) => [...entries, entry]);
  }, []);

  // Refresh the paused call stack + local variables from the live session.
  const refreshPausedState = useCallback(async (session: DebugSession) => {
    const stack = await session.stack();
    setFrames(stack);
    const top = stack[0];
    setCurrentStopLine(top ? top.line : null);
    if (top) {
      const scopes = await session.scopes(top.id);
      const local = scopes.find((scope) => scope.name === "local") ?? scopes[0];
      if (local) {
        setVariables(await session.variables(local.variables_reference));
      } else {
        setVariables([]);
      }
    }
  }, []);

  // Drain dap events; surface output + react to terminate.
  const drainEvents = useCallback(async () => {
    const session = sessionRef.current;
    if (!session) return;
    const events = await session.pollEvents();
    for (const event of events) {
      if (event.kind === "output") {
        appendConsole({ kind: "output", category: event.category, text: event.output.trimEnd() });
      } else if (event.kind === "terminated") {
        setStatus("terminated");
        setExitCode(event.exit_code ?? null);
        setCurrentStopLine(null);
        setFrames([]);
        setVariables([]);
        if (pollTimerRef.current) {
          clearInterval(pollTimerRef.current);
          pollTimerRef.current = null;
        }
      }
    }
  }, [appendConsole]);

  const launch = useCallback(async () => {
    if (!selectedAdapter) return;
    setError(null);
    setExitCode(null);
    setConsoleEntries([]);
    try {
      const session = await launchSession({ adapter: selectedAdapter, program });
      sessionRef.current = session;
      // Bind the operator's breakpoints (REAL CDP verified verdicts).
      const bound = await session.setBreakpoints(
        sourceUrl,
        breakpointLines.map((line) => ({ line })),
      );
      setVerifiedLines(new Set(bound.filter((b) => b.verified).map((b) => b.line ?? 0)));
      // Begin polling the event stream.
      pollTimerRef.current = setInterval(() => {
        void drainEvents();
      }, 150);
      // Resume from the entry pause; run into the first breakpoint.
      setStatus("running");
      await session.continue();
      // Wait until a breakpoint stop is observed (poll the stack).
      await waitForPause(session);
      setStatus("paused");
      await refreshPausedState(session);
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : String(cause));
    }
  }, [selectedAdapter, program, sourceUrl, breakpointLines, drainEvents, refreshPausedState]);

  const stepOver = useCallback(async () => {
    const session = sessionRef.current;
    if (!session || status !== "paused") return;
    try {
      await session.step("over");
      await refreshPausedState(session);
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : String(cause));
    }
  }, [status, refreshPausedState]);

  const resume = useCallback(async () => {
    const session = sessionRef.current;
    if (!session || status !== "paused") return;
    setStatus("running");
    setCurrentStopLine(null);
    try {
      await session.continue();
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : String(cause));
    }
  }, [status]);

  const evaluate = useCallback(
    async (expression: string) => {
      const session = sessionRef.current;
      const frameId = topFrame?.id;
      if (!session || !frameId) return;
      appendConsole({ kind: "input", text: expression });
      try {
        const result = await session.evaluate(frameId, expression);
        appendConsole({ kind: "result", text: result });
      } catch (cause) {
        appendConsole({ kind: "error", text: cause instanceof Error ? cause.message : String(cause) });
      }
    },
    [topFrame, appendConsole],
  );

  const terminate = useCallback(async () => {
    const session = sessionRef.current;
    if (!session) return;
    try {
      const code = await session.terminate();
      setExitCode(code ?? null);
      setStatus("terminated");
      setCurrentStopLine(null);
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : String(cause));
    } finally {
      if (pollTimerRef.current) {
        clearInterval(pollTimerRef.current);
        pollTimerRef.current = null;
      }
    }
  }, []);

  useEffect(() => {
    return () => {
      if (pollTimerRef.current) clearInterval(pollTimerRef.current);
    };
  }, []);

  const adapterOptions = useMemo(
    () =>
      adapters.map((descriptor) => (
        <option key={descriptor.id} value={descriptor.kind}>
          {descriptor.display_name}
        </option>
      )),
    [adapters],
  );

  return (
    <section className="debug-side-panel" data-testid="debug-side-panel" aria-label="Debugger">
      <header className="debug-side-panel__header">
        <span className="debug-side-panel__title">Debugger</span>
        <select
          className="debug-side-panel__adapter"
          data-testid="debug-side-panel.adapter"
          value={selectedAdapter}
          onChange={(event) => setSelectedAdapter(event.target.value as DebugAdapterKind)}
          disabled={status === "running" || status === "paused"}
        >
          {adapterOptions}
        </select>
        <button
          type="button"
          data-testid="debug-side-panel.launch"
          onClick={() => void launch()}
          disabled={!selectedAdapter || status === "running" || status === "paused"}
        >
          Launch
        </button>
        <button
          type="button"
          data-testid="debug-side-panel.step-over"
          onClick={() => void stepOver()}
          disabled={status !== "paused"}
        >
          Step Over
        </button>
        <button
          type="button"
          data-testid="debug-side-panel.continue"
          onClick={() => void resume()}
          disabled={status !== "paused"}
        >
          Continue
        </button>
        <button
          type="button"
          data-testid="debug-side-panel.terminate"
          onClick={() => void terminate()}
          disabled={status === "idle" || status === "terminated"}
        >
          Terminate
        </button>
        <span className="debug-side-panel__status" data-testid="debug-side-panel.status" data-status={status}>
          {status}
          {status === "terminated" && exitCode !== null ? ` (exit ${exitCode})` : ""}
        </span>
      </header>

      {error && (
        <div className="debug-side-panel__error" data-testid="debug-side-panel.error" role="alert">
          {error}
        </div>
      )}

      <div className="debug-side-panel__body">
        <div
          ref={hostRef}
          data-testid="debug-side-panel.editor"
          className="debug-side-panel__editor"
          style={{ minHeight: 180, height: 180 }}
        />

        <div className="debug-side-panel__inspect">
          <div className="debug-side-panel__stack" data-testid="debug-side-panel.stack">
            <header>Call Stack</header>
            <ol>
              {frames.map((frame) => (
                <li
                  key={frame.id}
                  data-testid={`debug-side-panel.frame.${frame.name}`}
                  data-line={frame.line}
                >
                  {frame.name} @ {frame.line}
                </li>
              ))}
            </ol>
          </div>
          <div className="debug-side-panel__variables" data-testid="debug-side-panel.variables">
            <header>Variables</header>
            <ul>
              {variables.map((variable) => (
                <li
                  key={variable.name}
                  data-testid={`debug-side-panel.var.${variable.name}`}
                  data-value={variable.value}
                >
                  <span className="debug-side-panel__var-name">{variable.name}</span>
                  <span className="debug-side-panel__var-value">{variable.value}</span>
                </li>
              ))}
            </ul>
          </div>
        </div>

        <DebugConsole
          entries={consoleEntries}
          canEvaluate={status === "paused" && Boolean(topFrame)}
          onEvaluate={evaluate}
        />
      </div>
    </section>
  );
}

/**
 * Poll the live session until it reports a paused frame (the breakpoint hit).
 * The session pushes a Stopped event, but the stack endpoint is the
 * authoritative paused-state query the panel renders from.
 */
async function waitForPause(session: DebugSession, timeoutMs = 20_000): Promise<void> {
  const deadline = Date.now() + timeoutMs;
  while (Date.now() < deadline) {
    try {
      const frames = await session.stack();
      if (frames.length > 0) return;
    } catch {
      // 409 NotPaused — keep polling until the breakpoint binds.
    }
    await new Promise((resolve) => setTimeout(resolve, 100));
  }
  throw new Error("timed out waiting for the debuggee to pause at a breakpoint");
}

// Re-export the gutter CSS class names so the harness/visual layer can assert
// the decorations are present without importing the gutter module directly.
export const DEBUG_GUTTER_CLASSES = {
  breakpoint: BREAKPOINT_GLYPH_CLASS,
  breakpointUnverified: BREAKPOINT_UNVERIFIED_GLYPH_CLASS,
  currentStop: CURRENT_STOP_LINE_CLASS,
};
