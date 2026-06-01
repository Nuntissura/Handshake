import { invoke } from "@tauri-apps/api/core";

import {
  subscribeBoardEvents,
  type SwarmBoardDelta,
} from "./swarm_runtime";
import {
  defaultTerminalIpc,
  type TerminalSession,
  type TerminalSubscription,
} from "./terminal";

// IPC bindings for the WP-KERNEL-004 Unified Per-Session Record + Replay surface.
//
// GOAL (governance glue #1): give the operator a UNIFIED per-session record they
// can reopen later — "go back and look when things go wrong or I forget". These
// bindings call the REAL backend aggregator commands (kernel_session_list /
// kernel_session_transcript_get) registered in `app/src-tauri/src/lib.rs` by the
// Integrate phase. The transcript is DERIVED on read: the backend joins the four
// existing durable sources (chat.jsonl, Flight Recorder events, terminal capture,
// process-ledger-derived process rows) into ONE ordered timeline keyed by the
// session_id (the swarm composite instance_id `<model_id>#<instance>` for
// swarm/agent sessions, or the chat UUID for an operator-chat session).
//
// HONESTY: the response carries a per-source `sourceStatus` (present | empty |
// unavailable) so the panel can render honest empty/unavailable states instead of
// fabricating rows. No mocks here; the SessionTranscriptIpc interface is the
// injection seam so the panel is unit-testable under jsdom (where Tauri `invoke`
// is unavailable) — tests pass a fake, production uses defaultSessionTranscriptIpc.

// ---------------------------------------------------------------------------
// Transcript entry model. Mirrors the handshake_core::session_transcript
// `SessionTranscriptEntry` serde (tag = "kind", snake_case). Every variant
// carries a common `ts` (RFC3339 merge key) + a post-merge `seq` for stable
// scroll/test anchoring.
// ---------------------------------------------------------------------------

/**
 * The lanes a transcript row can belong to (drives the UI kind filter).
 *
 * `agent_activity` is the structured agentic-CLI lane (WP-KERNEL-004 ROI add):
 * the CLI bridge runtime, when its `output_format` is a JSON stream mode, parses
 * its own stdout JSONL into typed agent-activity (tool calls, visible thinking,
 * text) and emits `FR-EVT-AGENT-*` events that the session_transcript aggregator
 * classifies (by `event_id`) into `AgentActivityEntry` rows. It rides the SAME
 * FR-derived `fr` source bucket (these ARE Flight Recorder events); the kind
 * filter is the user-facing distinction. In RawText mode no agent rows exist —
 * the raw stdout flows to the terminal lane exactly as before (honest fallback).
 */
export type TranscriptKind =
  | "chat_turn"
  | "fr_event"
  | "terminal_chunk"
  | "process"
  | "agent_activity";

/** The structured agent-activity sub-kinds (mirrors backend `activityKind`). */
export type AgentActivityKind = "tool_call" | "thinking" | "text" | "other";

export interface ChatTurnEntry {
  kind: "chat_turn";
  /** RFC3339 timestamp (the merge key). */
  ts: string;
  /** Stable post-merge sequence index. */
  seq: number;
  /** "operator" | "assistant" | "system" etc. (chat record role). */
  role: string;
  /** The model role label if this turn came from a model session. */
  modelRole?: string | null;
  content: string;
  messageId: string;
}

export interface FrEventEntry {
  kind: "fr_event";
  ts: string;
  seq: number;
  /** FlightRecorder event_type (e.g. "system" | "llm_inference"). */
  eventType: string;
  /** The FR-EVT-* family tag from the payload, if present. */
  frEvent?: string | null;
  /** Actor that emitted the event. */
  actor: string;
  modelId?: string | null;
  /** Raw FR payload (JSON), shown expandable in the UI. */
  payload: unknown;
  eventId: string;
}

export interface TerminalChunkEntry {
  kind: "terminal_chunk";
  ts: string;
  seq: number;
  terminalSessionId: string;
  frEvent?: string | null;
  /** The command line, when the chunk is a terminal command event. */
  command?: string | null;
  /** Captured terminal text/output, when present. */
  text?: string | null;
}

export interface ProcessEntry {
  kind: "process";
  ts: string;
  seq: number;
  processUuid?: string | null;
  /** Lifecycle phase, e.g. "spawned" | "completed". */
  phase: string;
  modelId?: string | null;
  payload: unknown;
}

/**
 * One structured agent-activity row parsed from the agentic CLI's JSON output
 * stream. Mirrors the backend `SessionTranscriptEntry::AgentActivity` serde
 * (tag = "agent_activity", camelCase fields). DEFENSIVE: an unrecognized or
 * malformed CLI line is never dropped — it arrives here as `activityKind:"other"`
 * with the raw line in `text`, so capture stays lossless.
 */
export interface AgentActivityEntry {
  kind: "agent_activity";
  ts: string;
  seq: number;
  /** "tool_call" | "thinking" | "text" | "other" (backend `activityKind`). */
  activityKind: AgentActivityKind;
  /** Tool name, present for `tool_call` rows (e.g. "Bash", "command_execution"). */
  name?: string | null;
  /** Redacted tool input/args (JSON), present for `tool_call` rows. */
  detail?: unknown;
  /** Body text for thinking/text/other rows (raw line for `other`). */
  text?: string | null;
  /** The FR-EVT-AGENT-* family tag this row was classified from. */
  eventId: string;
}

export type SessionTranscriptEntry =
  | ChatTurnEntry
  | FrEventEntry
  | TerminalChunkEntry
  | ProcessEntry
  | AgentActivityEntry;

/** Per-source availability so the UI can render emptiness honestly. */
export type SourceState = "present" | "empty" | "unavailable";

export interface SourceStatus {
  chat: SourceState;
  fr: SourceState;
  terminal: SourceState;
  process: SourceState;
}

export interface SessionTranscriptResponse {
  sessionId: string;
  entries: SessionTranscriptEntry[];
  sourceStatus: SourceStatus;
  /** True if any hard cap was applied while assembling the transcript. */
  truncated: boolean;
}

export interface SourceCounts {
  chat: number;
  fr: number;
  terminal: number;
  process: number;
}

/** One row in the session index (kernel_session_list). */
export interface SessionSummary {
  sessionId: string;
  /** "chat" | "swarm" | "agent". */
  kind: string;
  startedAt?: string | null;
  lastActivityAt?: string | null;
  modelId?: string | null;
  provider?: string | null;
  title?: string | null;
  counts: SourceCounts;
}

export interface TranscriptGetRequest {
  sessionId: string;
  /** RFC3339 inclusive lower bound. */
  from?: string | null;
  /** RFC3339 inclusive upper bound. */
  to?: string | null;
  /** Restrict to these lanes (server-side filter). Omit for all. */
  kinds?: TranscriptKind[] | null;
}

// ---------------------------------------------------------------------------
// Command wrappers.
// ---------------------------------------------------------------------------

export async function listSessions(): Promise<SessionSummary[]> {
  return await invoke<SessionSummary[]>("kernel_session_list");
}

export async function getTranscript(
  request: TranscriptGetRequest,
): Promise<SessionTranscriptResponse> {
  return await invoke<SessionTranscriptResponse>("kernel_session_transcript_get", {
    sessionId: request.sessionId,
    from: request.from ?? null,
    to: request.to ?? null,
    kinds: request.kinds ?? null,
  });
}

/**
 * The shape SessionReplayPanel depends on, so the panel can render under jsdom
 * with a recording stub injected (Tauri `invoke` is unavailable there). The real
 * implementation is `defaultSessionTranscriptIpc` below; tests pass a fake.
 */
export interface SessionTranscriptIpc {
  listSessions(): Promise<SessionSummary[]>;
  getTranscript(request: TranscriptGetRequest): Promise<SessionTranscriptResponse>;
}

export const defaultSessionTranscriptIpc: SessionTranscriptIpc = {
  listSessions,
  getTranscript,
};

// ---------------------------------------------------------------------------
// Live tail seam. The SessionReplayPanel "Live" mode subscribes to the EXISTING
// push streams (swarm://event + terminal://output, with their resync signals)
// and, on any event correlated to the focused session, incrementally re-fetches
// the transcript TAIL via the SAME getTranscript aggregator (the single source
// of truth — no forked client transcript model). This interface is the jsdom
// injection seam: tests pass fakes that drive the subscriptions deterministically
// (Tauri's event system + invoke are unavailable under jsdom).
// ---------------------------------------------------------------------------

/**
 * The push-stream subscriptions the live tail needs, plus a terminal-session
 * lister to resolve a `terminal://output` terminalSessionId -> the composite
 * instanceId (the transcript session_id). Default impls bridge to the real
 * swarm + terminal IPC; tests inject controllable fakes.
 */
export interface LiveTailIpc {
  /** Subscribe to swarm board deltas + resync. Returns an unlisten fn. */
  subscribeBoardEvents(
    onDelta: (delta: SwarmBoardDelta) => void,
    onResync: (dropped: number) => void,
  ): Promise<() => void>;
  /** Subscribe to the terminal output/exit/resync stream. Returns an unlisten fn. */
  subscribeTerminal(sub: TerminalSubscription): Promise<() => void>;
  /** List live terminal sessions (to map terminalSessionId -> instanceId). */
  listTerminalSessions(): Promise<TerminalSession[]>;
}

export const defaultLiveTailIpc: LiveTailIpc = {
  subscribeBoardEvents,
  subscribeTerminal: (sub) => defaultTerminalIpc.subscribe(sub),
  listTerminalSessions: () => defaultTerminalIpc.listSessions(),
};

/**
 * A STABLE per-entry identity key for dedupe + React list keys across separate
 * transcript fetches (the live tail re-fetches overlapping windows).
 *
 * CRITICAL: the post-merge `seq` is assigned per-fetch via `enumerate()` in the
 * backend `merge_transcript` / `append_terminal_scrollback`, so it is NOT stable
 * across fetches — a tail fetch returns entries seq-numbered `0..n` for THAT
 * window. Keying by `seq` would cause duplicate rows and remounts. So:
 *   - chat_turn      -> the chat record `messageId` (durable).
 *   - fr_event       -> the FR UUIDv7 `eventId` (durable, globally unique).
 *   - agent_activity -> the FR UUIDv7 `eventId` (these ARE FR events).
 *   - terminal_chunk -> the serialized TerminalChunk carries NO event_id. The
 *     FR-DERIVED command rows are keyed by their stable content tuple
 *     (terminalSessionId + ts + frEvent/command); the LIVE-SCROLLBACK enrichment
 *     row (no frEvent, no command, ts = Utc::now() re-synthesized every fetch)
 *     is keyed ONLY by `terminal-live:<terminalSessionId>` so it is a single
 *     rolling row REPLACED in place each fetch, never an accumulating duplicate.
 *
 * {@link isLiveScrollbackEntry} identifies that replace-in-place singleton so the
 * live-tail merge can treat it specially.
 */
export function entryStableKey(e: SessionTranscriptEntry): string {
  switch (e.kind) {
    case "chat_turn":
      return `chat:${e.messageId}`;
    case "fr_event":
      return `fr:${e.eventId}`;
    case "agent_activity":
      return `agent:${e.eventId}`;
    case "process":
      // Process rows are FR-derived; the durable signal is the process uuid +
      // phase (a spawn and a completion share neither). Fall back to ts when the
      // uuid is absent so the key is never empty.
      return `process:${e.processUuid ?? "anon"}:${e.phase}:${e.ts}`;
    case "terminal_chunk": {
      const isLive =
        (e.frEvent === null || e.frEvent === undefined) &&
        (e.command === null || e.command === undefined);
      if (isLive) return `terminal-live:${e.terminalSessionId}`;
      // FR-derived terminal command/lifecycle row: stable content tuple.
      return `terminal:${e.terminalSessionId}:${e.ts}:${e.frEvent ?? ""}:${e.command ?? ""}`;
    }
    default: {
      const _never: never = e;
      return String(_never);
    }
  }
}

/**
 * True for the live-scrollback ENRICHMENT terminal row: a TerminalChunk with no
 * `frEvent` and no `command` (only rolling captured `text`), synthesized at
 * `Utc::now()` each fetch for a still-open capture session. This is the single
 * rolling tail row that must be REPLACED by key, not appended, on every re-fetch.
 */
export function isLiveScrollbackEntry(e: SessionTranscriptEntry): e is TerminalChunkEntry {
  return (
    e.kind === "terminal_chunk" &&
    (e.frEvent === null || e.frEvent === undefined) &&
    (e.command === null || e.command === undefined)
  );
}

/** The lanes, in stable UI display order, with human labels. */
export const TRANSCRIPT_KINDS: { kind: TranscriptKind; label: string }[] = [
  { kind: "chat_turn", label: "Chat" },
  { kind: "agent_activity", label: "Agent" },
  { kind: "terminal_chunk", label: "Terminal" },
  { kind: "fr_event", label: "Flight Recorder" },
  { kind: "process", label: "Process" },
];
