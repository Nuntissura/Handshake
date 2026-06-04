import { invoke } from "@tauri-apps/api/core";

import {
  getSpawnTemplate,
  resumeSession,
  subscribeBoardEvents,
  type SessionSpawnTemplate,
  type SwarmBoardDelta,
  type SwarmInstanceId,
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
  /**
   * Assigned VM/sandbox worktree recorded at spawn (the SwarmEvent `$.worktree_id`
   * the backend lifts into the summary). Mirrors `SessionSummary.worktree_id`
   * (backend serde `camelCase`) — previously missing on the TS side (drift).
   */
  worktreeId?: string | null;
  /**
   * ROI #3 STATE RECOVERY: true when a resume spawn template exists for this
   * session (a swarm spawn captured at spawn time). HONEST: false for chat (UUID)
   * sessions, swarm sessions spawned before the feature, and sessions whose
   * best-effort template write failed — those are NOT resumable, so the Resume
   * affordance is hidden/disabled. `#[serde(default)]` on the backend keeps older
   * payloads (no field) decoding as `false`.
   */
  resumable?: boolean;
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
// Cross-session search (ROI #4 "I-forget-something" recall). Mirrors the
// handshake_core::commands::session_transcript `kernel_session_search` serde
// (camelCase). The backend scans the SAME aggregator corpus (chat.jsonl + the FR
// DuckDB events) through the SAME correlation seam the transcript uses, runs each
// captured snippet through the SecretRedactor, bounds the result (session-hit
// limit + per-session snippet cap), and returns ranked SessionSearchHit[] each
// openable straight into that session's transcript via the EXISTING select path.
// HONEST: an empty/whitespace query rejects (never "match all"); a session is a
// hit only when a row the operator can actually open in the transcript matched.
// ---------------------------------------------------------------------------

/**
 * The entry-kind a matched snippet came from. A superset of {@link TranscriptKind}
 * — the backend tags FR/agent/terminal/process rows by the same lane vocabulary
 * the transcript uses, so a hit chip can reuse the transcript's KIND_STYLE.
 */
export type SearchEntryKind = TranscriptKind;

/** One matched, secret-REDACTED snippet within a session hit. */
export interface SearchSnippet {
  /** The lane the matched row belongs to (drives the snippet's kind chip). */
  entryKind: string;
  /** RFC3339 timestamp of the matched row, when known. */
  ts?: string | null;
  /** Matched text + a little context window, ALREADY secret-redacted by the backend. */
  snippet: string;
}

/** One ranked session that matched the search, with its capped snippets. */
export interface SessionSearchHit {
  sessionId: string;
  /** "chat" | "swarm" | "agent" (from the session summary). */
  kind: string;
  provider?: string | null;
  modelId?: string | null;
  worktreeId?: string | null;
  startedAt?: string | null;
  title?: string | null;
  /** Total matches across the session BEFORE the per-session snippet cap (honest). */
  matchCount: number;
  /** Capped (per-session) matched snippets, each redacted. */
  snippets: SearchSnippet[];
}

export interface SessionSearchResponse {
  hits: SessionSearchHit[];
  /** True if the session-hit limit (or candidate cap) clipped results. */
  truncated: boolean;
  /** The effective (trimmed) query the backend ran, echoed for the UI. */
  query: string;
}

/** The cross-session search request (mirrors the backend Tauri camelCase args). */
export interface SessionSearchRequest {
  /** Free text, case-insensitive. Empty/whitespace is rejected by the backend. */
  query: string;
  /** Restrict matched snippets to these lanes. Omit for all. */
  kinds?: TranscriptKind[] | null;
  /** Restrict candidate sessions to this worktree. */
  worktreeId?: string | null;
  /** RFC3339 inclusive lower bound on session activity. */
  from?: string | null;
  /** RFC3339 inclusive upper bound on session activity. */
  to?: string | null;
  /** Session-hit cap (backend default 50, hard max 200). */
  limit?: number | null;
}

// ---------------------------------------------------------------------------
// ROI #5 EXPORT: export a recorded session's consolidated record to a portable,
// secret-REDACTED file (markdown for humans + json for machines) for archive /
// handoff / sharing. Mirrors the handshake_core::commands::session_transcript
// `kernel_session_export` serde (camelCase). The backend REUSES the SAME
// aggregator (build_response) the transcript view uses + the SAME PatternRedactor
// the cross-session search trusts, then atomically writes the file(s) under a
// disk-agnostic dest dir (default <app_data_root>/exports). It returns the
// written path(s) + byte sizes so the panel can surface a copyable path. HONEST:
// a discoverable session with no entries writes an empty-but-valid file
// (`empty: true`); an UNKNOWN session id rejects with `SESSION_NOT_FOUND:`.
// ---------------------------------------------------------------------------

/** The export render format. `both` writes a markdown AND a json artifact. */
export type ExportFormat = "markdown" | "json" | "both";

/** One written artifact: which format, its absolute path, and its byte size. */
export interface ExportedFile {
  /** "markdown" | "json" (the artifact's format). */
  format: string;
  /** Absolute, copyable path the artifact landed at. */
  path: string;
  /** Byte size of the written artifact. */
  bytes: number;
}

/** The result of a session export (mirrors the backend `SessionExportResponse`). */
export interface SessionExportResponse {
  sessionId: string;
  /** The directory the file(s) landed in (absolute) — for an "open folder" hint. */
  destDir: string;
  files: ExportedFile[];
  /** True when the session had zero entries (an empty-but-valid file was written). */
  empty: boolean;
  /**
   * Total emitted text fields that matched a secret pattern and were masked —
   * an HONEST "N secrets redacted" trust signal (never the secret itself).
   */
  redactedFieldCount: number;
}

/** The export request (mirrors the backend Tauri camelCase args). */
export interface SessionExportRequest {
  sessionId: string;
  /** "markdown" | "json" | "both" (default "both" at the call site). */
  format: ExportFormat;
  /**
   * Optional operator-chosen destination dir. Omitted/null => the backend default
   * (`<app_data_root>/exports`). The operator owns where a share file lands.
   */
  destDir?: string | null;
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
 * Cross-session free-text search (ROI #4). Calls the REAL backend
 * `kernel_session_search` command with camelCase args matching the serde. The
 * backend rejects an empty/whitespace query (honest, not match-all), so callers
 * should gate the submit button accordingly.
 */
export async function searchSessions(
  request: SessionSearchRequest,
): Promise<SessionSearchResponse> {
  return await invoke<SessionSearchResponse>("kernel_session_search", {
    query: request.query,
    kinds: request.kinds ?? null,
    worktreeId: request.worktreeId ?? null,
    from: request.from ?? null,
    to: request.to ?? null,
    limit: request.limit ?? null,
  });
}

/**
 * ROI #5 EXPORT: export a recorded session to a portable, secret-REDACTED file
 * (markdown / json / both). Calls the REAL backend `kernel_session_export`
 * command with camelCase args matching the serde. The backend REUSES the
 * aggregator + the PatternRedactor (no secret leaks) and atomically writes the
 * file(s), returning the written path(s) + byte sizes. HONEST: an unknown
 * session id rejects with `SESSION_NOT_FOUND:`; a discoverable empty session
 * writes an empty-but-valid file (`empty: true`).
 */
export async function exportSession(
  request: SessionExportRequest,
): Promise<SessionExportResponse> {
  return await invoke<SessionExportResponse>("kernel_session_export", {
    sessionId: request.sessionId,
    format: request.format,
    destDir: request.destDir ?? null,
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
  /**
   * ROI #3 STATE RECOVERY: re-spawn a FRESH session carrying the recorded
   * session's captured config (provider/model/artifact/sha/worktree/working_dir/
   * isolation). Drives the SAME validated backend spawn path (integrity gate +
   * coordinator bounds preserved); the new session's lineage parent is `sessionId`.
   * Returns the new session's instance id. Rejects with `SESSION_NOT_RESUMABLE:`
   * when no template is stored (race vs. the `resumable` flag). Bridged to
   * `swarm_runtime.resumeSession` so the panel stays jsdom-testable via a fake.
   */
  resumeSession(sessionId: string): Promise<SwarmInstanceId>;
  /**
   * ROI #3 (edit-then-resume): read the stored spawn template so the host can
   * PREFILL the spawn form. Returns null when the session is not resumable.
   */
  getSpawnTemplate(sessionId: string): Promise<SessionSpawnTemplate | null>;
  /**
   * ROI #4 RECALL: cross-session free-text search. Returns ranked hits (with
   * redacted snippets) the operator can open straight into the session
   * transcript via the existing select path. Part of this seam so the panel +
   * harness stay jsdom-testable with an injected fake.
   */
  searchSessions(request: SessionSearchRequest): Promise<SessionSearchResponse>;
  /**
   * ROI #5 EXPORT: export the recorded session to a portable, secret-REDACTED
   * file (markdown / json / both) for archive / handoff / sharing. Returns the
   * written path(s) + byte sizes (+ redaction telemetry + an empty flag) the
   * panel surfaces honestly. Part of this seam so the panel + harness stay
   * jsdom-testable with an injected fake.
   */
  exportSession(request: SessionExportRequest): Promise<SessionExportResponse>;
}

export const defaultSessionTranscriptIpc: SessionTranscriptIpc = {
  listSessions,
  getTranscript,
  resumeSession,
  getSpawnTemplate,
  searchSessions,
  exportSession,
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
