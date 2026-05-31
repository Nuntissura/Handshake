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

export interface CreateSessionRequest {
  sessionType: TerminalSessionType;
  /** Shell/program to launch (HumanDev). Backend default when omitted. */
  shell?: string | null;
  swarmId?: string | null;
  worktreeId?: string | null;
  title?: string | null;
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
  exitCode: number | null;
  /** Combined stdout/stderr after secret redaction. */
  output: string;
  timedOut: boolean;
}

// ---------------------------------------------------------------------------
// Command wrappers (the 7 kernel_terminal_* commands).
// ---------------------------------------------------------------------------

export async function createSession(request: CreateSessionRequest): Promise<TerminalSession> {
  return await invoke<TerminalSession>("kernel_terminal_create_session", { request });
}

/**
 * Write operator keystrokes to a session's stdin. The backend rejects this for
 * read-only / capability-denied sessions (TERM-INVARIANTS: AI must not type into
 * human terminals, and AiJob capture sessions are inspect-only by default). The
 * panel only calls this after the Take-control gate resolves ok, but the gate is
 * not the authority — this call is.
 */
export async function writeStdin(sessionId: string, data: string): Promise<void> {
  await invoke("kernel_terminal_write_stdin", { sessionId, data });
}

export async function resizeSession(sessionId: string, cols: number, rows: number): Promise<void> {
  await invoke("kernel_terminal_resize", { sessionId, cols, rows });
}

export async function closeSession(sessionId: string): Promise<void> {
  await invoke("kernel_terminal_close_session", { sessionId });
}

export async function listSessions(): Promise<TerminalSession[]> {
  return await invoke<TerminalSession[]>("kernel_terminal_list_sessions");
}

export async function runCommand(
  request: { command: string; args?: string[]; timeoutMs?: number; swarmId?: string | null },
): Promise<RunCommandResult> {
  return await invoke<RunCommandResult>("kernel_terminal_run_command", { request });
}

export async function scrollback(sessionId: string): Promise<ScrollbackSnapshot> {
  return await invoke<ScrollbackSnapshot>("kernel_terminal_scrollback", { sessionId });
}

// ---------------------------------------------------------------------------
// Live output stream. Mirrors subscribeBoardEvents in swarm_runtime.ts: the
// backend forwarder emits `terminal://output` {session_id, seq, chunk_base64},
// `terminal://exit` {session_id, exit_code}, and `terminal://resync`
// {session_id, dropped} when the bounded broadcast lags. A seq gap on a session
// OR a resync event means we missed bytes -> the caller refetches scrollback.
// ---------------------------------------------------------------------------

export interface TerminalOutputEvent {
  session_id: string;
  seq: number;
  /** Base64 of a raw byte chunk. NEVER decode as text: see decodeChunk. */
  chunk_base64: string;
}

export interface TerminalExitEvent {
  session_id: string;
  exit_code: number | null;
}

export interface TerminalResyncEvent {
  session_id: string;
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
    const { session_id, seq, chunk_base64 } = e.payload;
    const prev = lastSeq.get(session_id);
    if (prev !== undefined && seq !== prev + 1) {
      // Gap: we missed bytes for this session. Re-baseline and ask for resync.
      lastSeq.set(session_id, seq);
      sub.onResync(session_id, { reason: "seq-gap", dropped: seq - prev - 1 });
      return;
    }
    lastSeq.set(session_id, seq);
    sub.onOutput(session_id, decodeChunk(chunk_base64));
  });

  const unExit = await listen<TerminalExitEvent>("terminal://exit", (e) => {
    sub.onExit(e.payload.session_id, e.payload.exit_code);
  });

  const unResync = await listen<TerminalResyncEvent>("terminal://resync", (e) => {
    // Backend told us its bounded broadcast lagged for this session: force a
    // scrollback refetch and drop our seq baseline so the next output re-seeds.
    lastSeq.delete(e.payload.session_id);
    sub.onResync(e.payload.session_id, { reason: "broadcast-lag", dropped: e.payload.dropped });
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
  createSession(request: CreateSessionRequest): Promise<TerminalSession>;
  writeStdin(sessionId: string, data: string): Promise<void>;
  resizeSession(sessionId: string, cols: number, rows: number): Promise<void>;
  closeSession(sessionId: string): Promise<void>;
  listSessions(): Promise<TerminalSession[]>;
  scrollback(sessionId: string): Promise<ScrollbackSnapshot>;
  subscribe(sub: TerminalSubscription): Promise<() => void>;
}

export const defaultTerminalIpc: TerminalIpc = {
  createSession,
  writeStdin,
  resizeSession,
  closeSession,
  listSessions,
  scrollback,
  subscribe,
};
