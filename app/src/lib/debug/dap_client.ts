// WP-KERNEL-009 MT-254 DebugAdapterCore — frontend DAP client.
//
// Typed mirror of the backend `debug_adapter::protocol` shapes plus the
// adapter-registry + durable-breakpoint client surface AND the live session
// drive (launch/breakpoints/stack/scopes/variables/evaluate/step/continue/
// pause/terminate + the dap:// event stream). Everything is driven over the
// product HTTP API (axum @ 127.0.0.1:37501) — the SAME transport the rest of
// the UI speaks; there is no Tauri IPC bridge. This module owns the typed
// shapes, the honesty-gate validation, and the calls so the UI never invents
// adapter entries or fakes a breakpoint.

import {
  continueSession,
  evaluateInSession,
  getDebugAdapters,
  getDebugBreakpoints,
  getSessionScopes,
  getSessionStack,
  getSessionVariables,
  launchDebugSession,
  pauseSession,
  pollSessionEvents,
  setDebugBreakpoints,
  setSessionBreakpoints,
  stepSession,
  terminateDebugSession,
  type DebugAdapterDescriptor,
  type DebugAdapterKind,
  type DebugBreakpointInput,
  type DebugBreakpointRecord,
  type SessionBreakpoint,
  type SessionScope,
  type SessionStackFrame,
  type SessionVariable,
} from "../api";

/** Why a debuggee stopped (mirrors backend StoppedReason). */
export type StoppedReason =
  | "breakpoint"
  | "step"
  | "pause"
  | "entry"
  | "exception"
  | "other";

/** Stepping granularity (mirrors backend StepKind). */
export type StepKind = "over" | "into" | "out";

/** A breakpoint request against a source. */
export type SourceBreakpoint = {
  line: number;
  column?: number;
  condition?: string;
};

/** The adapter's verdict on a requested breakpoint. */
export type Breakpoint = {
  id: string;
  verified: boolean;
  line?: number;
  message?: string;
};

/** One frame of a paused call stack. */
export type StackFrame = {
  id: string;
  name: string;
  source?: string;
  line: number;
  column: number;
};

/** A variable scope within a paused frame. */
export type Scope = {
  name: string;
  variables_reference: string;
  expensive: boolean;
};

/** One variable (real runtime value). */
export type Variable = {
  name: string;
  value: string;
  type_name?: string;
  variables_reference: string;
};

/** A streamed dap:// lifecycle/output event. */
export type DebugEvent =
  | { kind: "stopped"; reason: StoppedReason; top_frame_line?: number; top_frame_source?: string }
  | { kind: "output"; category: string; output: string }
  | { kind: "continued" }
  | { kind: "terminated"; exit_code?: number };

/**
 * The runnable adapters. Throws if the backend ever returns a non-runnable
 * entry — the UI must NEVER show a dead adapter (negative-check guarantee).
 */
export async function loadRunnableAdapters(): Promise<DebugAdapterDescriptor[]> {
  const { adapters } = await getDebugAdapters();
  for (const adapter of adapters) {
    if (!adapter.runnable) {
      throw new Error(
        `debug adapter '${adapter.id}' is listed but not runnable; refusing to show a dead adapter`,
      );
    }
  }
  return adapters;
}

/** Load the durable breakpoints for a RichDocument. */
export async function loadBreakpoints(richDocumentId: string): Promise<DebugBreakpointRecord[]> {
  const { breakpoints } = await getDebugBreakpoints(richDocumentId);
  return breakpoints;
}

/** Persist the full breakpoint set for a RichDocument (PUT semantics). */
export async function persistBreakpoints(
  richDocumentId: string,
  workspaceId: string,
  breakpoints: DebugBreakpointInput[],
): Promise<DebugBreakpointRecord[]> {
  const { breakpoints: stored } = await setDebugBreakpoints(
    richDocumentId,
    workspaceId,
    breakpoints,
  );
  return stored;
}

/**
 * Toggle a breakpoint at (source_url, line) in the given set and return the new
 * set. Adding a breakpoint at a line removes a matching one (a true toggle).
 * This is the pure model the gutter uses before persisting.
 */
export function toggleBreakpoint(
  current: DebugBreakpointInput[],
  sourceUrl: string,
  line: number,
): DebugBreakpointInput[] {
  const existingIndex = current.findIndex(
    (b) => b.source_url === sourceUrl && b.line === line,
  );
  if (existingIndex >= 0) {
    return current.filter((_, i) => i !== existingIndex);
  }
  return [...current, { source_url: sourceUrl, line }];
}

/**
 * A live debug session handle. Drives launch/breakpoints/stack/scopes/
 * variables/evaluate/step/continue/pause/terminate over the product HTTP API
 * against a REAL debuggee process. There is no mock path.
 */
export class DebugSession {
  constructor(
    public readonly sessionId: string,
    public readonly adapter: string,
    /** The line the session is paused at after launch (entry stop), if known. */
    public topFrameLine: number | undefined,
  ) {}

  /** Bind breakpoints on the live session (REAL CDP binding; verified is real). */
  async setBreakpoints(
    source: string,
    breakpoints: SourceBreakpoint[],
  ): Promise<SessionBreakpoint[]> {
    const { breakpoints: bound } = await setSessionBreakpoints(this.sessionId, source, breakpoints);
    return bound;
  }

  /** The paused call stack. */
  async stack(): Promise<SessionStackFrame[]> {
    const { frames } = await getSessionStack(this.sessionId);
    return frames;
  }

  /** A paused frame's variable scopes. */
  async scopes(frameId: string): Promise<SessionScope[]> {
    const { scopes } = await getSessionScopes(this.sessionId, frameId);
    return scopes;
  }

  /** Real runtime variables behind a scope/object reference. */
  async variables(reference: string): Promise<SessionVariable[]> {
    const { variables } = await getSessionVariables(this.sessionId, reference);
    return variables;
  }

  /** Debug-console eval in the paused frame. */
  async evaluate(frameId: string, expression: string): Promise<string> {
    const { result } = await evaluateInSession(this.sessionId, frameId, expression);
    return result;
  }

  /** Step over/into/out; resolves once paused again, returning the new line. */
  async step(kind: StepKind): Promise<number | undefined> {
    const { top_frame_line } = await stepSession(this.sessionId, kind);
    this.topFrameLine = top_frame_line;
    return top_frame_line;
  }

  /** Resume execution. */
  async continue(): Promise<void> {
    await continueSession(this.sessionId);
  }

  /** Pause a running debuggee. */
  async pause(): Promise<void> {
    await pauseSession(this.sessionId);
  }

  /** Drain the dap events that arrived since the last poll. */
  async pollEvents(): Promise<DebugEvent[]> {
    const { events } = await pollSessionEvents(this.sessionId);
    return events.map(parseDebugEvent).filter((e): e is DebugEvent => e !== null);
  }

  /** Terminate the session; returns the real process exit code if known. */
  async terminate(): Promise<number | undefined> {
    const { exit_code } = await terminateDebugSession(this.sessionId);
    return exit_code;
  }
}

/** Launch a REAL debuggee and return a driveable [`DebugSession`]. */
export async function launchSession(input: {
  adapter: DebugAdapterKind;
  program: string;
  cwd?: string;
  runtimePath?: string;
}): Promise<DebugSession> {
  const launched = await launchDebugSession({
    adapter: input.adapter,
    program: input.program,
    cwd: input.cwd,
    runtime_path: input.runtimePath,
  });
  return new DebugSession(launched.session_id, launched.adapter, launched.top_frame_line);
}

/** Parse a raw dap event payload into a typed [`DebugEvent`]. */
export function parseDebugEvent(raw: unknown): DebugEvent | null {
  if (typeof raw !== "object" || raw === null) return null;
  const obj = raw as Record<string, unknown>;
  switch (obj.kind) {
    case "stopped":
      return {
        kind: "stopped",
        reason: (obj.reason as StoppedReason) ?? "other",
        top_frame_line: typeof obj.top_frame_line === "number" ? obj.top_frame_line : undefined,
        top_frame_source:
          typeof obj.top_frame_source === "string" ? obj.top_frame_source : undefined,
      };
    case "output":
      return {
        kind: "output",
        category: String(obj.category ?? "stdout"),
        output: String(obj.output ?? ""),
      };
    case "continued":
      return { kind: "continued" };
    case "terminated":
      return {
        kind: "terminated",
        exit_code: typeof obj.exit_code === "number" ? obj.exit_code : undefined,
      };
    default:
      return null;
  }
}
