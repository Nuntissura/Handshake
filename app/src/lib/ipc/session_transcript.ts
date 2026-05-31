import { invoke } from "@tauri-apps/api/core";

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

/** The four lanes a transcript row can belong to (drives the UI kind filter). */
export type TranscriptKind = "chat_turn" | "fr_event" | "terminal_chunk" | "process";

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

export type SessionTranscriptEntry =
  | ChatTurnEntry
  | FrEventEntry
  | TerminalChunkEntry
  | ProcessEntry;

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

/** The four lanes, in stable UI display order, with human labels. */
export const TRANSCRIPT_KINDS: { kind: TranscriptKind; label: string }[] = [
  { kind: "chat_turn", label: "Chat" },
  { kind: "terminal_chunk", label: "Terminal" },
  { kind: "fr_event", label: "Flight Recorder" },
  { kind: "process", label: "Process" },
];
