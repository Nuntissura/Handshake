import { invoke } from "@tauri-apps/api/core";

// IPC bindings for the WP-KERNEL-004 Integrated Terminal surface. These call the
// REAL managed TerminalRuntime backend commands registered in
// `app/src-tauri/src/lib.rs` (kernel_terminal_*). No mocks: create spawns a real
// PTY session, write/resize/close drive it, run_command is the one-shot
// capability-gated path, and subscribe() forwards the live `terminal://output`
// byte stream straight into xterm.
//
// LAW TERM-V1-SCOPE / TERM-INVARIANTS: AiJob sessions are inspect-only by
// default. This client never auto-wires stdin; callers must explicitly opt into
// interaction (the panel gates that behind a Take-control toggle), and the real
// authorization is enforced backend-side in kernel_terminal_write_stdin. The
// toggle is UX, not authorization.

/** Session classes, matching the backend TerminalSessionType serde. */
export type TerminalSessionType = "HumanDev" | "AiJob" | "PluginTool";

type BackendTerminalSessionType = "HUMAN_DEV" | "AI_JOB" | "PLUGIN_TOOL";

export interface TerminalSession {
  sessionId: string;
  sessionType: TerminalSessionType;
  /** Swarm swimlane binding (Jira board link). Null = ungrouped. */
  swarmId: string | null;
  /** Worktree binding, if any. */
  worktreeId: string | null;
  /** Source instance (e.g. cloud CLI bridge instance, sandbox adapter id). */
  instanceId: string | null;
  /** Human-facing title for the tab. */
  title: string;
  /** True once the PTY child has exited (read-only, no stdin). */
  exited: boolean;
  /** Exit code if exited, else null. */
  exitCode: number | null;
  /**
   * Whether the backend permits interactive stdin for this session AT ALL.
   * HumanDev sessions are interactive; AiJob capture sessions are read-only
   * unless an explicit capability grant flips this. The panel ALSO requires an
   * operator Take-control toggle before wiring stdin, but this flag is the
   * backend's authority signal and gates whether the toggle is even offered.
   */
  interactiveAllowed: boolean;
}

export interface TerminalContext {
  /** Backend-resolved workspace root for new HumanDev terminal cwd. */
  cwd: string;
  /** Optional shell override. Null means omit shell and let the backend resolve its platform default. */
  defaultShell: string | null;
}

export interface TerminalDiagnostics {
  /** Runtime-wide count of failed terminal Flight Recorder/EventLedger receipt writes. */
  receiptFailureCount: number;
}

export interface CreateSessionRequest {
  sessionType?: TerminalSessionType | null;
  /** Shell/program to launch (HumanDev). Backend default when omitted. */
  shell?: string | null;
  args?: string[];
  cwd?: string | null;
  rows?: number;
  cols?: number;
  swarmId?: string | null;
  worktreeId?: string | null;
  instanceId?: string | null;
  title?: string | null;
  capabilityScope?: string[];
}

/** A scrollback snapshot: raw captured bytes (base64) up to the backend cap. */
export interface ScrollbackSnapshot {
  sessionId: string;
  /** Latest seq represented by this snapshot (for delta re-baselining). */
  seq: number;
  /** Base64 of the raw captured byte buffer (already scrollback-capped). */
  chunkBase64: string;
  /** True when the capped scrollback dropped older bytes (truncation marker). */
  truncated: boolean;
}

/** One-shot run_command result (capability-gated, redacted, timeout-bounded). */
export interface RunCommandResult {
  sessionId: string;
  exitCode: number | null;
  /** Combined stdout/stderr after secret redaction. */
  output: string;
  timedOut: boolean;
}

interface SessionInfoIpc {
  sessionId: string;
  kind?: string;
  sessionType: string;
  swarmId: string | null;
  worktreeId: string | null;
  instanceId: string | null;
  title?: string | null;
  exited?: boolean;
  exitCode?: number | null;
  interactiveAllowed?: boolean;
  interactiveAuthorized?: boolean;
}

interface RunCommandResultIpc {
  sessionId: string;
  exitCode: number;
  timedOut?: boolean;
  outputBase64: string;
}

function toBackendSessionType(sessionType: TerminalSessionType | null | undefined): BackendTerminalSessionType | undefined {
  switch (sessionType) {
    case "HumanDev":
      return "HUMAN_DEV";
    case "AiJob":
      return "AI_JOB";
    case "PluginTool":
      return "PLUGIN_TOOL";
    default:
      return undefined;
  }
}

function fromBackendSessionType(sessionType: string): TerminalSessionType {
  switch (sessionType) {
    case "HUMAN_DEV":
    case "HumanDev":
      return "HumanDev";
    case "AI_JOB":
    case "AiJob":
      return "AiJob";
    case "PLUGIN_TOOL":
    case "PluginTool":
      return "PluginTool";
    default:
      return "PluginTool";
  }
}

function bytesToBase64(bytes: Uint8Array): string {
  let binary = "";
  for (const byte of bytes) {
    binary += String.fromCharCode(byte);
  }
  return btoa(binary);
}

function normalizeCreateSessionRequest(request: CreateSessionRequest) {
  return {
    ...request,
    sessionType: toBackendSessionType(request.sessionType),
  };
}

function normalizeInteractiveAllowed(
  raw: SessionInfoIpc,
  sessionType: TerminalSessionType,
  exited: boolean,
): boolean {
  if (exited) return false;
  if (raw.interactiveAllowed !== undefined) return raw.interactiveAllowed;
  if (sessionType === "HumanDev") return true;

  const kind = raw.kind?.toUpperCase();
  if (kind === "INTERACTIVE") return true;
  if (kind === "CAPTURE") return false;

  return raw.interactiveAuthorized ?? false;
}

function normalizeSession(raw: SessionInfoIpc): TerminalSession {
  const sessionType = fromBackendSessionType(raw.sessionType);
  const exited = raw.exited ?? false;
  return {
    sessionId: raw.sessionId,
    sessionType,
    swarmId: raw.swarmId ?? null,
    worktreeId: raw.worktreeId ?? null,
    instanceId: raw.instanceId ?? null,
    title: raw.title ?? raw.sessionId,
    exited,
    exitCode: raw.exitCode ?? null,
    interactiveAllowed: normalizeInteractiveAllowed(raw, sessionType, exited),
  };
}

// ---------------------------------------------------------------------------
// Command wrappers (the 7 kernel_terminal_* commands).
// ---------------------------------------------------------------------------

export async function createSession(request: CreateSessionRequest): Promise<TerminalSession> {
  const raw = await invoke<SessionInfoIpc>("kernel_terminal_create_session", {
    req: normalizeCreateSessionRequest(request),
  });
  return normalizeSession(raw);
}

/**
 * Write operator keystrokes to a session's stdin. The backend rejects this for
 * read-only / capability-denied sessions (TERM-INVARIANTS: AI must not type into
 * human terminals, and AiJob capture sessions are inspect-only by default). The
 * panel only calls this after the Take-control gate resolves ok, but the gate is
 * not the authority — this call is.
 */
export async function writeStdin(
  sessionId: string,
  data: string,
  options: { asAi?: boolean } = {},
): Promise<void> {
  await invoke("kernel_terminal_write_stdin", {
    sessionId,
    dataBase64: bytesToBase64(new TextEncoder().encode(data)),
    asAi: options.asAi ?? false,
  });
}

export async function authorizeInteractive(sessionId: string): Promise<void> {
  await invoke("kernel_terminal_authorize_interactive", { sessionId });
}

export async function resizeSession(sessionId: string, cols: number, rows: number): Promise<void> {
  await invoke("kernel_terminal_resize", { sessionId, cols, rows });
}

export async function closeSession(sessionId: string): Promise<void> {
  await invoke("kernel_terminal_close_session", { sessionId });
}

export async function listSessions(): Promise<TerminalSession[]> {
  const raw = await invoke<SessionInfoIpc[]>("kernel_terminal_list_sessions");
  return raw.map(normalizeSession);
}

export async function getContext(): Promise<TerminalContext> {
  return invoke<TerminalContext>("kernel_terminal_context");
}

export async function getDiagnostics(): Promise<TerminalDiagnostics> {
  return invoke<TerminalDiagnostics>("kernel_terminal_diagnostics");
}

export async function runCommand(
  request: {
    command: string;
    args?: string[];
    cwd?: string | null;
    timeoutMs?: number;
    swarmId?: string | null;
    capabilityScope?: string[];
  },
): Promise<RunCommandResult> {
  const raw = await invoke<RunCommandResultIpc>("kernel_terminal_run_command", {
    req: {
      shell: request.command,
      args: request.args ?? [],
      cwd: request.cwd,
      timeoutMs: request.timeoutMs,
      swarmId: request.swarmId,
      capabilityScope: request.capabilityScope ?? [],
    },
  });
  return {
    sessionId: raw.sessionId,
    exitCode: raw.exitCode,
    output: new TextDecoder().decode(decodeChunk(raw.outputBase64)),
    timedOut: raw.timedOut ?? raw.exitCode === -1,
  };
}

export async function scrollback(sessionId: string): Promise<ScrollbackSnapshot> {
  const chunkBase64 = await invoke<string>("kernel_terminal_scrollback", { sessionId });
  return {
    sessionId,
    seq: 0,
    chunkBase64,
    truncated: false,
  };
}

// ---------------------------------------------------------------------------
// Live output stream. Mirrors subscribeBoardEvents in swarm_runtime.ts: the
// backend forwarder emits `terminal://output` {session_id, seq, chunk_base64},
// `terminal://exit` {session_id, exit_code}, and `terminal://resync`
// {session_id, dropped} when the bounded broadcast lags. A seq gap on a session
// OR a resync event means we missed bytes -> the caller refetches scrollback.
// ---------------------------------------------------------------------------

export interface TerminalOutputEvent {
  sessionId: string;
  seq: number;
  /** Base64 of a raw byte chunk. NEVER decode as text: see decodeChunk. */
  chunkBase64: string;
}

export interface TerminalExitEvent {
  sessionId: string;
  exitCode: number | null;
}

export interface TerminalResyncEvent {
  sessionId: string;
  dropped: number;
}

/**
 * Decode a base64 output chunk to RAW BYTES.
 *
 * Terminal output carries ANSI/VT control sequences and multibyte UTF-8 that
 * MUST be handed to xterm as bytes (term.write(Uint8Array)); stringifying here
 * corrupts control sequences and splits multibyte codepoints. We therefore
 * decode base64 -> Uint8Array and never touch TextDecoder. Pure + dependency-free
 * so it is unit-testable under jsdom (which has atob).
 */
export function decodeChunk(chunkBase64: string): Uint8Array {
  // atob yields a binary string (one char per byte, code points 0..255).
  const binary = atob(chunkBase64);
  const out = new Uint8Array(binary.length);
  for (let i = 0; i < binary.length; i += 1) {
    out[i] = binary.charCodeAt(i) & 0xff;
  }
  return out;
}

/** Callbacks for the per-session subscription. */
export interface TerminalSubscription {
  /** Raw output bytes for this session (already base64-decoded). */
  onOutput: (sessionId: string, bytes: Uint8Array) => void;
  /** PTY child exited. */
  onExit: (sessionId: string, exitCode: number | null) => void;
  /**
   * We dropped bytes (seq gap or explicit resync). The caller should refetch
   * scrollback() and reset the xterm buffer. `reason` distinguishes the two.
   */
  onResync: (sessionId: string, info: { reason: "seq-gap" | "broadcast-lag"; dropped: number }) => void;
}

/**
 * Subscribe to the live terminal stream for ALL sessions and demux by
 * session_id. Tracks the last seq per session so a gap triggers onResync without
 * blindly applying a partial stream (the same drift-safety rule the swarm board
 * uses). Returns an unlisten function. Uses Tauri's event system (no polling).
 */
export async function subscribe(sub: TerminalSubscription): Promise<() => void> {
  const { listen } = await import("@tauri-apps/api/event");
  const lastSeq = new Map<string, number>();

  const unOutput = await listen<TerminalOutputEvent>("terminal://output", (e) => {
    const { sessionId, seq, chunkBase64 } = e.payload;
    const prev = lastSeq.get(sessionId);
    if (prev !== undefined && seq !== prev + 1) {
      // Gap: we missed bytes for this session. Re-baseline and ask for resync.
      lastSeq.set(sessionId, seq);
      sub.onResync(sessionId, { reason: "seq-gap", dropped: seq - prev - 1 });
      return;
    }
    lastSeq.set(sessionId, seq);
    sub.onOutput(sessionId, decodeChunk(chunkBase64));
  });

  const unExit = await listen<TerminalExitEvent>("terminal://exit", (e) => {
    sub.onExit(e.payload.sessionId, e.payload.exitCode);
  });

  const unResync = await listen<TerminalResyncEvent>("terminal://resync", (e) => {
    // Backend told us its bounded broadcast lagged for this session: force a
    // scrollback refetch and drop our seq baseline so the next output re-seeds.
    lastSeq.delete(e.payload.sessionId);
    sub.onResync(e.payload.sessionId, { reason: "broadcast-lag", dropped: e.payload.dropped });
  });

  return () => {
    unOutput();
    unExit();
    unResync();
  };
}

/**
 * The shape TerminalPanel depends on, so the panel can be rendered under jsdom
 * with a recording stub injected (Tauri `invoke` is unavailable there). The real
 * implementation is `defaultTerminalIpc` below; tests pass a fake.
 */
export interface TerminalIpc {
  getContext(): Promise<TerminalContext>;
  createSession(request: CreateSessionRequest): Promise<TerminalSession>;
  authorizeInteractive(sessionId: string): Promise<void>;
  writeStdin(sessionId: string, data: string): Promise<void>;
  resizeSession(sessionId: string, cols: number, rows: number): Promise<void>;
  closeSession(sessionId: string): Promise<void>;
  listSessions(): Promise<TerminalSession[]>;
  scrollback(sessionId: string): Promise<ScrollbackSnapshot>;
  subscribe(sub: TerminalSubscription): Promise<() => void>;
}

export const defaultTerminalIpc: TerminalIpc = {
  getContext,
  createSession,
  authorizeInteractive,
  writeStdin,
  resizeSession,
  closeSession,
  listSessions,
  scrollback,
  subscribe,
};
