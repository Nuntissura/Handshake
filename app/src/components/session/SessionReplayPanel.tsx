import {
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
} from "react";

import { Disclosure } from "../common/Disclosure";
import {
  defaultSessionTranscriptIpc,
  defaultLiveTailIpc,
  entryStableKey,
  isLiveScrollbackEntry,
  TRANSCRIPT_KINDS,
  type AgentActivityEntry,
  type LiveTailIpc,
  type SessionSummary,
  type SessionTranscriptEntry,
  type SessionTranscriptIpc,
  type SessionTranscriptResponse,
  type SourceState,
  type SourceStatus,
  type TranscriptKind,
} from "../../lib/ipc/session_transcript";
import {
  eventInstanceKey,
  eventTerminalState,
} from "../../lib/ipc/swarm_runtime";

// WP-KERNEL-004 Session Replay review surface.
//
// Operator requirement: a UNIFIED per-session record the operator can reopen
// later — "go back and look when things go wrong or I forget". This is the AUDIT
// SUBSTRATE for Handshake self-hosting this repo's governance. It is NOT the main
// window: it is an off-main-window, collapsed-by-default + lazy <Disclosure>
// drawer (mirroring the Terminal drawer), reachable from the swarm board's
// per-lane "Review session" affordance.
//
// READ-ONLY review: no inputs, no stdin, no edit affordances anywhere. Left =
// the recorded-session index (kernel_session_list). Right = the selected
// session's consolidated, timestamp-ordered, typed timeline (chat turns +
// terminal output + FR lifecycle/inference + process rows), filterable by kind,
// scrollable, with HONEST per-source empty / unavailable states driven by the
// response `sourceStatus` — never fabricated rows.
//
// Testable with the IPC client injected (Tauri `invoke` is unavailable under
// jsdom): pass a fake `ipc`; production uses defaultSessionTranscriptIpc.

export interface SessionReplayPanelProps {
  /** Injectable IPC client (tests pass a recording stub). */
  ipc?: SessionTranscriptIpc;
  /**
   * Injectable live-tail seam (swarm + terminal push subscriptions + terminal
   * session lister). Tests pass deterministic fakes; production bridges to the
   * real swarm/terminal IPC. Omit to use {@link defaultLiveTailIpc}.
   */
  liveIpc?: LiveTailIpc;
  /** Start expanded. Defaults to collapsed-by-default (off-main-window). */
  defaultOpen?: boolean;
  /**
   * One-shot open driver forwarded to the host Disclosure. The board's "Review
   * session" affordance bumps this to reveal the drawer on demand.
   */
  openSignal?: number;
  /**
   * Optional: preselect this session id when the panel opens (board link). The
   * board affordance knows a swarm composite instance_id; the panel selects it
   * if the session index contains it. Re-applied whenever `openSignal` changes.
   */
  focusSessionId?: string | null;
}

const KIND_STYLE: Record<TranscriptKind, { label: string; bg: string; fg: string }> = {
  chat_turn: { label: "chat", bg: "#dbeafe", fg: "#1e3a8a" },
  agent_activity: { label: "agent", bg: "#cffafe", fg: "#155e75" },
  terminal_chunk: { label: "terminal", bg: "#dcfce7", fg: "#14532d" },
  fr_event: { label: "fr", bg: "#fef3c7", fg: "#78350f" },
  process: { label: "process", bg: "#ede9fe", fg: "#4c1d95" },
};

const SOURCE_EMPTY_MESSAGE: Record<TranscriptKind, string> = {
  chat_turn: "No chat turns recorded for this session.",
  agent_activity: "No structured agent activity captured for this session.",
  terminal_chunk: "No terminal output captured for this session.",
  fr_event: "No Flight Recorder events recorded for this session.",
  process: "No process activity recorded for this session.",
};

/**
 * Map a UI kind to the response sourceStatus key. `agent_activity` rides the
 * FR-derived `fr` source bucket (agent rows ARE Flight Recorder events); the
 * kind filter is the user-facing distinction.
 */
const KIND_TO_SOURCE: Record<TranscriptKind, keyof SessionTranscriptResponse["sourceStatus"]> = {
  chat_turn: "chat",
  agent_activity: "fr",
  terminal_chunk: "terminal",
  fr_event: "fr",
  process: "process",
};

function shortId(id: string, max = 22): string {
  return id.length > max ? `${id.slice(0, max)}…` : id;
}

function formatTs(ts: string): string {
  const d = new Date(ts);
  return Number.isNaN(d.getTime()) ? ts : d.toLocaleString();
}

/** Compact one-line summary of an FR payload for the collapsed row view. */
function payloadSummary(payload: unknown, max = 160): string {
  if (payload === null || payload === undefined) return "";
  let text: string;
  try {
    text = typeof payload === "string" ? payload : JSON.stringify(payload);
  } catch {
    text = String(payload);
  }
  return text.length > max ? `${text.slice(0, max)}…` : text;
}

/** One transcript row, typed + timestamped + expandable for raw FR payloads. */
function TranscriptRow({ entry }: { entry: SessionTranscriptEntry }) {
  const style = KIND_STYLE[entry.kind];
  return (
    <li
      className="session-replay__entry"
      data-testid={`session-replay-entry-${entry.seq}`}
      data-kind={entry.kind}
      style={{
        display: "grid",
        gridTemplateColumns: "84px 78px 1fr",
        gap: 8,
        padding: "6px 8px",
        borderBottom: "1px solid var(--hs-color-border, #e5e7eb)",
        fontSize: 12,
        alignItems: "start",
      }}
    >
      <span
        className="session-replay__entry-ts"
        style={{ color: "var(--hs-color-text-subtle)", whiteSpace: "nowrap", overflow: "hidden", textOverflow: "ellipsis" }}
        title={entry.ts}
      >
        {formatTs(entry.ts)}
      </span>
      <span
        className="session-replay__entry-kind"
        style={{
          fontSize: 10,
          padding: "1px 6px",
          borderRadius: 8,
          background: style.bg,
          color: style.fg,
          justifySelf: "start",
          whiteSpace: "nowrap",
        }}
      >
        {style.label}
      </span>
      <div style={{ minWidth: 0 }}>{renderEntryBody(entry)}</div>
    </li>
  );
}

function renderEntryBody(entry: SessionTranscriptEntry) {
  switch (entry.kind) {
    case "chat_turn":
      return (
        <div className="session-replay__chat">
          <span style={{ fontWeight: 600 }}>{entry.modelRole || entry.role}</span>
          <div style={{ whiteSpace: "pre-wrap", wordBreak: "break-word", marginTop: 2 }}>
            {entry.content}
          </div>
        </div>
      );
    case "terminal_chunk":
      return (
        <div className="session-replay__terminal">
          {entry.command ? (
            <div style={{ fontFamily: "ui-monospace, Consolas, monospace", color: "#15803d" }}>
              $ {entry.command}
            </div>
          ) : null}
          {entry.text ? (
            <pre
              style={{
                margin: "2px 0 0",
                fontFamily: "ui-monospace, Consolas, monospace",
                whiteSpace: "pre-wrap",
                wordBreak: "break-word",
                background: "#0b1020",
                color: "#e5e7eb",
                padding: 6,
                borderRadius: 4,
                maxHeight: 220,
                overflow: "auto",
              }}
            >
              {entry.text}
            </pre>
          ) : null}
          {!entry.command && !entry.text ? (
            <span style={{ color: "var(--hs-color-text-subtle)" }}>
              {entry.frEvent ?? "terminal event"} ({shortId(entry.terminalSessionId, 12)})
            </span>
          ) : null}
        </div>
      );
    case "fr_event":
      return (
        <details className="session-replay__fr">
          <summary style={{ cursor: "pointer", listStyle: "revert" }}>
            <span style={{ fontWeight: 600 }}>{entry.frEvent ?? entry.eventType}</span>
            <span style={{ color: "var(--hs-color-text-subtle)", marginLeft: 6 }}>
              {entry.actor}
              {entry.modelId ? ` · ${entry.modelId}` : ""}
            </span>
            <span style={{ marginLeft: 6 }}>{payloadSummary(entry.payload)}</span>
          </summary>
          <pre
            style={{
              margin: "4px 0 0",
              fontSize: 11,
              whiteSpace: "pre-wrap",
              wordBreak: "break-word",
              background: "var(--hs-color-surface-muted, #f3f4f6)",
              padding: 6,
              borderRadius: 4,
              maxHeight: 220,
              overflow: "auto",
            }}
          >
            {JSON.stringify(entry.payload, null, 2)}
          </pre>
        </details>
      );
    case "agent_activity":
      return renderAgentActivityBody(entry);
    case "process":
      return (
        <div className="session-replay__process">
          <span style={{ fontWeight: 600 }}>{entry.phase}</span>
          {entry.processUuid ? (
            <span style={{ color: "var(--hs-color-text-subtle)", marginLeft: 6 }}>
              {shortId(entry.processUuid, 18)}
            </span>
          ) : null}
          {entry.modelId ? (
            <span style={{ color: "var(--hs-color-text-subtle)", marginLeft: 6 }}>{entry.modelId}</span>
          ) : null}
        </div>
      );
    default: {
      // Exhaustiveness guard: any new variant must be handled above.
      const _never: never = entry;
      return <span>{String(_never)}</span>;
    }
  }
}

/**
 * Render a structured agent-activity row (parsed from the agentic CLI's JSON
 * stream) distinctly per sub-kind:
 *   - tool_call: bold name + a collapsible <details> with the redacted args,
 *   - thinking:  muted, italic body (the model's visible reasoning),
 *   - text:      a normal pre-wrapped body (model-facing text),
 *   - other:     monospace raw body + a "raw" chip so the HONEST defensive
 *                fallback (an unrecognized/malformed CLI line, never dropped) is
 *                visually obvious to the operator.
 */
function renderAgentActivityBody(entry: AgentActivityEntry) {
  switch (entry.activityKind) {
    case "tool_call":
      return (
        <details className="session-replay__agent session-replay__agent--tool" data-agent-kind="tool_call">
          <summary style={{ cursor: "pointer", listStyle: "revert" }}>
            <span style={{ color: "#155e75", fontWeight: 600 }}>⚙ {entry.name || "tool"}</span>
            {entry.detail !== undefined && entry.detail !== null ? (
              <span style={{ color: "var(--hs-color-text-subtle)", marginLeft: 6 }}>
                {payloadSummary(entry.detail)}
              </span>
            ) : null}
          </summary>
          {entry.detail !== undefined && entry.detail !== null ? (
            <pre
              style={{
                margin: "4px 0 0",
                fontSize: 11,
                whiteSpace: "pre-wrap",
                wordBreak: "break-word",
                background: "var(--hs-color-surface-muted, #f3f4f6)",
                padding: 6,
                borderRadius: 4,
                maxHeight: 220,
                overflow: "auto",
              }}
            >
              {JSON.stringify(entry.detail, null, 2)}
            </pre>
          ) : null}
        </details>
      );
    case "thinking":
      return (
        <div
          className="session-replay__agent session-replay__agent--thinking"
          data-agent-kind="thinking"
          style={{
            color: "var(--hs-color-text-subtle)",
            fontStyle: "italic",
            whiteSpace: "pre-wrap",
            wordBreak: "break-word",
          }}
        >
          {entry.text}
        </div>
      );
    case "text":
      return (
        <div
          className="session-replay__agent session-replay__agent--text"
          data-agent-kind="text"
          style={{ whiteSpace: "pre-wrap", wordBreak: "break-word" }}
        >
          {entry.text}
        </div>
      );
    case "other":
    default:
      return (
        <div className="session-replay__agent session-replay__agent--other" data-agent-kind="other">
          <span
            style={{
              fontSize: 10,
              padding: "0 5px",
              borderRadius: 8,
              background: "var(--hs-color-surface-muted, #f3f4f6)",
              color: "var(--hs-color-text-subtle)",
              marginRight: 6,
            }}
          >
            raw
          </span>
          <span
            style={{
              fontFamily: "ui-monospace, Consolas, monospace",
              whiteSpace: "pre-wrap",
              wordBreak: "break-word",
            }}
          >
            {entry.text}
          </span>
        </div>
      );
  }
}

/** The session index (left rail). */
function SessionList({
  sessions,
  selectedId,
  onSelect,
  loading,
  error,
}: {
  sessions: SessionSummary[];
  selectedId: string | null;
  onSelect: (id: string) => void;
  loading: boolean;
  error: string | null;
}) {
  return (
    <div
      className="session-replay__list"
      data-testid="session-replay-list"
      style={{
        width: 280,
        flex: "0 0 280px",
        borderRight: "1px solid var(--hs-color-border, #e5e7eb)",
        overflow: "auto",
        maxHeight: 520,
      }}
    >
      {error ? (
        <div data-testid="session-replay-list-error" style={{ color: "#dc2626", fontSize: 12, padding: 8 }}>
          Session index error: {error}
        </div>
      ) : null}
      {!error && loading && sessions.length === 0 ? (
        <div style={{ color: "var(--hs-color-text-subtle)", fontSize: 13, padding: 8 }}>Loading sessions…</div>
      ) : null}
      {!error && !loading && sessions.length === 0 ? (
        <div data-testid="session-replay-list-empty" style={{ color: "var(--hs-color-text-subtle)", fontSize: 13, padding: 12 }}>
          No recorded sessions yet.
        </div>
      ) : null}
      <ul style={{ listStyle: "none", margin: 0, padding: 0 }}>
        {sessions.map((s) => {
          const selected = s.sessionId === selectedId;
          const total = s.counts.chat + s.counts.fr + s.counts.terminal + s.counts.process;
          return (
            <li key={s.sessionId}>
              <button
                type="button"
                data-testid={`session-replay-row-${s.sessionId}`}
                data-stable-id={`session-replay.row.${s.sessionId}`}
                data-selected={selected ? "true" : "false"}
                aria-pressed={selected}
                onClick={() => onSelect(s.sessionId)}
                style={{
                  display: "block",
                  width: "100%",
                  textAlign: "left",
                  padding: "8px 10px",
                  border: "none",
                  borderLeft: selected ? "3px solid #2563eb" : "3px solid transparent",
                  background: selected ? "#eff6ff" : "transparent",
                  color: "var(--hs-color-text)",
                  cursor: "pointer",
                  fontSize: 12,
                }}
              >
                <div style={{ display: "flex", gap: 6, alignItems: "center" }}>
                  <span style={{ fontWeight: 600, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>
                    {s.title || shortId(s.sessionId)}
                  </span>
                  <span
                    style={{
                      fontSize: 10,
                      padding: "0 5px",
                      borderRadius: 8,
                      background: "var(--hs-color-surface-muted, #f3f4f6)",
                      color: "var(--hs-color-text-subtle)",
                    }}
                  >
                    {s.kind}
                  </span>
                </div>
                <div style={{ display: "flex", gap: 8, marginTop: 2, color: "var(--hs-color-text-subtle)", fontSize: 11 }}>
                  {s.startedAt ? <span>{formatTs(s.startedAt)}</span> : <span>no timestamp</span>}
                  <span style={{ marginLeft: "auto" }}>{total} entr{total === 1 ? "y" : "ies"}</span>
                </div>
                {s.modelId || s.provider ? (
                  <div style={{ color: "var(--hs-color-text-subtle)", fontSize: 11, marginTop: 1, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>
                    {[s.provider, s.modelId].filter(Boolean).join(" · ")}
                  </div>
                ) : null}
              </button>
            </li>
          );
        })}
      </ul>
    </div>
  );
}

/** The consolidated timeline (right rail) for the selected session. */
function TranscriptTimeline({
  selectedId,
  response,
  loading,
  error,
  activeKinds,
  live = false,
  truncatedHead = false,
}: {
  selectedId: string | null;
  response: SessionTranscriptResponse | null;
  loading: boolean;
  error: string | null;
  activeKinds: Set<TranscriptKind>;
  /** Live tailing is active for this session (drives autoscroll affordances). */
  live?: boolean;
  /** Older live rows were trimmed from the head (memory cap) -> honest chip. */
  truncatedHead?: boolean;
}) {
  // Client-side refilter for instant response (the server also filters by
  // `kinds`, but we refilter so toggling feels immediate before the refetch).
  const visible = useMemo(() => {
    if (!response) return [];
    return response.entries.filter((e) => activeKinds.has(e.kind));
  }, [response, activeKinds]);

  // Autoscroll-to-latest with a pause-on-scroll-up affordance (log-tail UX).
  const listRef = useRef<HTMLUListElement | null>(null);
  const [autoscroll, setAutoscroll] = useState(true);
  const [newWhilePaused, setNewWhilePaused] = useState(0);
  const prevCountRef = useRef(0);

  // When new rows arrive: if pinned to the bottom, scroll down; else accumulate
  // the exact number of new rows for the "n new" resume-button counter.
  useEffect(() => {
    const el = listRef.current;
    const count = visible.length;
    const added = Math.max(0, count - prevCountRef.current);
    prevCountRef.current = count;
    if (!live || !el) return;
    if (autoscroll) {
      el.scrollTop = el.scrollHeight;
      if (newWhilePaused !== 0) setNewWhilePaused(0);
    } else if (added > 0) {
      setNewWhilePaused((n) => n + added);
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [visible.length, live, autoscroll]);

  const onScroll = useCallback(() => {
    const el = listRef.current;
    if (!el) return;
    const atBottom = el.scrollHeight - el.scrollTop - el.clientHeight < 24;
    setAutoscroll(atBottom);
    if (atBottom) setNewWhilePaused(0);
  }, []);

  const jumpToLatest = useCallback(() => {
    const el = listRef.current;
    if (el) el.scrollTop = el.scrollHeight;
    setAutoscroll(true);
    setNewWhilePaused(0);
  }, []);

  if (!selectedId) {
    return (
      <div
        className="session-replay__timeline"
        data-testid="session-replay-timeline"
        style={{ flex: 1, padding: 16, color: "var(--hs-color-text-subtle)", fontSize: 13 }}
      >
        Select a session to review its consolidated timeline.
      </div>
    );
  }

  return (
    <div
      className="session-replay__timeline"
      data-testid="session-replay-timeline"
      style={{ flex: 1, display: "flex", flexDirection: "column", minWidth: 0 }}
    >
      {error ? (
        <div data-testid="session-replay-timeline-error" style={{ color: "#dc2626", fontSize: 12, padding: 8 }}>
          Transcript error: {error}
        </div>
      ) : null}

      {response?.sourceStatus
        ? renderHonestyBanners(response)
        : null}

      {truncatedHead ? (
        <div
          data-testid="session-replay-live-truncated-head"
          style={{
            fontSize: 11,
            color: "#78350f",
            background: "#fef3c7",
            borderRadius: 8,
            padding: "1px 8px",
            margin: "6px 8px",
            alignSelf: "flex-start",
          }}
        >
          older live rows trimmed — toggle Live off to re-read the full transcript
        </div>
      ) : null}

      {loading && !response ? (
        <div style={{ color: "var(--hs-color-text-subtle)", fontSize: 13, padding: 12 }}>Loading transcript…</div>
      ) : null}

      <div style={{ position: "relative", flex: 1, minHeight: 0, display: "flex", flexDirection: "column" }}>
        <ul
          ref={listRef}
          className="session-replay__entries"
          data-testid="session-replay-entries"
          onScroll={onScroll}
          style={{ listStyle: "none", margin: 0, padding: 0, overflow: "auto", maxHeight: 520, flex: 1 }}
        >
          {visible.map((entry) => (
            <TranscriptRow key={entryStableKey(entry)} entry={entry} />
          ))}
        </ul>

        {live && !autoscroll ? (
          <button
            type="button"
            data-testid="session-replay-autoscroll-resume"
            onClick={jumpToLatest}
            style={{
              position: "absolute",
              right: 12,
              bottom: 12,
              fontSize: 11,
              padding: "3px 10px",
              borderRadius: 14,
              border: "1px solid #166534",
              background: "#dcfce7",
              color: "#166534",
              cursor: "pointer",
              fontWeight: 600,
              boxShadow: "0 1px 4px rgba(0,0,0,0.15)",
            }}
          >
            Jump to latest ▾{newWhilePaused > 0 ? ` (${newWhilePaused} new)` : ""}
          </button>
        ) : null}
      </div>

      {!loading && response && visible.length === 0
        ? renderEmptyLanes(response, activeKinds)
        : null}
    </div>
  );
}

/** Unavailable/truncated banners derived from sourceStatus (honesty rules). */
function renderHonestyBanners(response: SessionTranscriptResponse) {
  const unavailable = (Object.keys(response.sourceStatus) as (keyof typeof response.sourceStatus)[])
    .filter((k) => response.sourceStatus[k] === "unavailable");
  return (
    <>
      {unavailable.length > 0 ? (
        <div
          data-testid="session-replay-unavailable-banner"
          style={{
            fontSize: 12,
            color: "#92400e",
            background: "#fffbeb",
            border: "1px solid #fde68a",
            borderRadius: 4,
            padding: "4px 8px",
            margin: "6px 8px",
          }}
        >
          Some sources are unavailable for this session ({unavailable.join(", ")}); showing what is recorded.
        </div>
      ) : null}
      {response.truncated ? (
        <div
          data-testid="session-replay-truncated-chip"
          style={{
            fontSize: 11,
            color: "#78350f",
            background: "#fef3c7",
            borderRadius: 8,
            padding: "1px 8px",
            margin: "6px 8px",
            alignSelf: "flex-start",
          }}
        >
          results truncated
        </div>
      ) : null}
    </>
  );
}

/**
 * When the visible set is empty, show an honest per-lane message for each ACTIVE
 * lane whose source is empty (never a fabricated row). If an active lane has
 * data but is filtered out by another lane being off, this still reads honestly
 * because activeKinds is exactly the lanes the operator asked to see.
 */
function renderEmptyLanes(response: SessionTranscriptResponse, activeKinds: Set<TranscriptKind>) {
  const lanes = TRANSCRIPT_KINDS.filter((k) => activeKinds.has(k.kind));
  return (
    <div data-testid="session-replay-empty" style={{ padding: 12 }}>
      {lanes.map((k) => {
        const status: SourceState = response.sourceStatus[KIND_TO_SOURCE[k.kind]];
        const message =
          status === "unavailable"
            ? `${k.label}: source unavailable for this session.`
            : SOURCE_EMPTY_MESSAGE[k.kind];
        return (
          <div
            key={k.kind}
            data-testid={`session-replay-empty-${k.kind}`}
            style={{ color: "var(--hs-color-text-subtle)", fontSize: 13, padding: "2px 0" }}
          >
            {message}
          </div>
        );
      })}
    </div>
  );
}

// ===========================================================================
// Live tail mode.
//
// When Live is ON for a non-terminal swarm/agent session, the panel tails the
// SAME kernel_session_transcript_get aggregator: it subscribes to the existing
// swarm://event + terminal://output push streams (with their resync signals);
// on any event CORRELATED to the focused session it incrementally re-fetches the
// transcript window newer than the last-seen ts and merges by STABLE id (never
// seq, which is fetch-relative). It does NOT fork a second transcript model — the
// merged array is always a dedup'd projection of what the aggregator returns.
//
// Bounded: debounced (coalesce bursts), min-interval rate-capped, single-flight,
// memory-capped (drop oldest rows + their dedupe keys), and idle when no/terminal
// session (no fetch loop, no spinning). A seq gap / resync triggers a full
// refetch (never apply a partial stream blind — the board's drift-safety rule).
// ===========================================================================

/** Session kinds that genuinely stream live (swarm/agent). Chat is polled-only. */
const LIVE_STREAMING_KINDS = new Set(["swarm", "agent"]);
/** Coalesce a burst of events into one fetch. */
const TAIL_DEBOUNCE_MS = 250;
/** Hard floor between actual tail fetches (token-bucket). */
const TAIL_MIN_INTERVAL_MS = 500;
/** Slow visibility-gated reconcile net (mirrors SwarmBoard). */
const LIVE_RECONCILE_MS = 10_000;
/** Cap retained live rows so a multi-hour session can't grow unbounded. */
const LIVE_MAX_ENTRIES = 2000;

export type LiveStatus = "live" | "polled" | "ended" | "idle";

/** Honest live status for a session given its kind + terminal flag + toggle. */
function liveStatusFor(
  kind: string | undefined,
  terminal: boolean,
  live: boolean,
): LiveStatus {
  if (!live) return "idle";
  if (terminal) return "ended";
  if (kind && LIVE_STREAMING_KINDS.has(kind)) return "live";
  // Chat (UUID) sessions: no swarm/terminal event carries the chat UUID, so we
  // can only reconcile on the slow visibility-gated net. Labelled honestly.
  return "polled";
}

/** Newest entry ts in a list, or null. Entries are ts-ascending. */
function newestTs(entries: SessionTranscriptEntry[]): string | null {
  return entries.length > 0 ? entries[entries.length - 1].ts : null;
}

/**
 * Merge incoming tail entries into the held array by STABLE id, preserving
 * ts-ascending order. Live-scrollback singleton rows are REPLACED in place (one
 * rolling row per terminal session). Returns the new array + the new seen-id set.
 * Caps to LIVE_MAX_ENTRIES (drops oldest), reporting whether a trim occurred.
 */
function mergeTail(
  held: SessionTranscriptEntry[],
  incoming: SessionTranscriptEntry[],
): { entries: SessionTranscriptEntry[]; truncatedHead: boolean } {
  // Index held rows by stable key for O(1) replace/skip.
  const byKey = new Map<string, number>();
  const merged = held.slice();
  for (let i = 0; i < merged.length; i += 1) byKey.set(entryStableKey(merged[i]), i);

  for (const e of incoming) {
    const key = entryStableKey(e);
    const existing = byKey.get(key);
    if (existing !== undefined) {
      // Live-scrollback singleton: replace in place (rolling text, new ts).
      if (isLiveScrollbackEntry(e)) merged[existing] = e;
      // Any other duplicate (inclusive-cursor boundary re-return) is dropped.
      continue;
    }
    merged.push(e);
    byKey.set(key, merged.length - 1);
  }

  // Stable re-sort by ts so a replaced live-scrollback row (new ts) lands right.
  merged.sort((a, b) => (a.ts < b.ts ? -1 : a.ts > b.ts ? 1 : 0));

  let truncatedHead = false;
  let out = merged;
  if (out.length > LIVE_MAX_ENTRIES) {
    out = out.slice(out.length - LIVE_MAX_ENTRIES);
    truncatedHead = true;
  }
  return { entries: out, truncatedHead };
}

/**
 * Merge a tail fetch's sourceStatus into the held status WITHOUT spurious
 * downgrades: a narrow tail window can report a lane `empty` that the full load
 * showed `present`. Only a full refetch sets authoritative status; tail merges
 * never downgrade present -> empty.
 */
function mergeSourceStatus(prev: SourceStatus | null, next: SourceStatus): SourceStatus {
  if (!prev) return next;
  const out = { ...prev };
  (Object.keys(next) as (keyof SourceStatus)[]).forEach((k) => {
    // Upgrade to present, or to unavailable, but never drop present -> empty.
    if (next[k] === "present") out[k] = "present";
    else if (next[k] === "unavailable" && prev[k] !== "present") out[k] = "unavailable";
  });
  return out;
}

export interface LiveTailController {
  /** The live entries (dedup'd projection of the aggregator's full range). */
  entries: SessionTranscriptEntry[];
  sourceStatus: SourceStatus | null;
  truncated: boolean;
  /** True once any live row beyond the cap was trimmed from the head. */
  truncatedHead: boolean;
  status: LiveStatus;
  error: string | null;
  loading: boolean;
}

interface UseLiveTranscriptTailArgs {
  ipc: SessionTranscriptIpc;
  liveIpc: LiveTailIpc;
  /** The focused session id (composite for swarm/agent, UUID for chat), or null. */
  sessionId: string | null;
  /** The focused session's kind ("swarm" | "agent" | "chat"), if known. */
  sessionKind: string | undefined;
  /** Whether Live mode is enabled by the operator. */
  live: boolean;
  /** Active lane filter (so the tail fetch matches the post-hoc query). */
  activeKindList: TranscriptKind[];
  /** Notify the body when the focused session reaches a terminal state. */
  onTerminal?: () => void;
}

/**
 * Drive incremental tail re-fetches of kernel_session_transcript_get from the
 * existing swarm + terminal push streams. Returns the merged live entries +
 * honest status. Idle (no subscriptions, no fetches) when not live, no session,
 * or the session is terminal.
 */
export function useLiveTranscriptTail({
  ipc,
  liveIpc,
  sessionId,
  sessionKind,
  live,
  activeKindList,
  onTerminal,
}: UseLiveTranscriptTailArgs): LiveTailController {
  const [entries, setEntries] = useState<SessionTranscriptEntry[]>([]);
  const [sourceStatus, setSourceStatus] = useState<SourceStatus | null>(null);
  const [truncated, setTruncated] = useState(false);
  const [truncatedHead, setTruncatedHead] = useState(false);
  const [terminalReached, setTerminalReached] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);

  // Tail cursor = newest held ts. Refs so the event handlers read live values
  // without re-subscribing on every entry change.
  const lastTsRef = useRef<string | null>(null);
  const entriesRef = useRef<SessionTranscriptEntry[]>([]);
  const fetchInFlightRef = useRef(false);
  const pendingRef = useRef(false);
  const lastFetchAtRef = useRef(0);
  const debounceRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const intervalRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  // terminalSessionId -> instanceId (composite). Learned from listTerminalSessions.
  const termMapRef = useRef<Map<string, string | null>>(new Map());
  const termRefreshInFlightRef = useRef(false);
  const aliveRef = useRef(true);
  // Monotonic fetch generation: bumped whenever the focused session (or live
  // mode) changes. A fetch captures the generation at start and discards its
  // result if the generation has since changed — so a slow getTranscript for
  // session A that resolves AFTER the operator switched to session B can never
  // write A's entries into B's view (cross-session contamination in an audit
  // surface). The aliveRef guard alone is insufficient: the component stays
  // mounted across a session switch.
  const fetchGenRef = useRef(0);

  const isStreamingKind = sessionKind !== undefined && LIVE_STREAMING_KINDS.has(sessionKind);
  // Only swarm/agent sessions correlate to push events; chat is polled-only.
  const active = live && !!sessionId && !terminalReached;

  const kindsArg = useCallback(
    () => (activeKindList.length === TRANSCRIPT_KINDS.length ? null : activeKindList),
    [activeKindList],
  );

  // ----- the core fetch (tail or full). single-flight + rate-capped. ---------
  const runFetch = useCallback(
    async (full: boolean) => {
      if (!sessionId) return;
      if (fetchInFlightRef.current) {
        pendingRef.current = true;
        return;
      }
      fetchInFlightRef.current = true;
      const gen = fetchGenRef.current;
      setLoading(true);
      lastFetchAtRef.current = Date.now();
      try {
        const from = full ? null : lastTsRef.current;
        const res = await ipc.getTranscript({ sessionId, from, kinds: kindsArg() });
        // Discard if the component unmounted OR the focused session changed
        // while this fetch was in flight (stale cross-session result).
        if (!aliveRef.current || gen !== fetchGenRef.current) return;
        if (full) {
          // Full refetch replaces the held model wholesale + re-baselines.
          const capped = res.entries.length > LIVE_MAX_ENTRIES;
          const next = capped ? res.entries.slice(res.entries.length - LIVE_MAX_ENTRIES) : res.entries;
          entriesRef.current = next;
          lastTsRef.current = newestTs(next);
          setEntries(next);
          setSourceStatus(res.sourceStatus);
          setTruncated(res.truncated);
          setTruncatedHead(capped);
        } else {
          const { entries: next, truncatedHead: trimmed } = mergeTail(entriesRef.current, res.entries);
          entriesRef.current = next;
          lastTsRef.current = newestTs(next);
          setEntries(next);
          setSourceStatus((prev) => mergeSourceStatus(prev, res.sourceStatus));
          setTruncated((prev) => prev || res.truncated);
          if (trimmed) setTruncatedHead(true);
        }
        setError(null);
      } catch (e) {
        if (!aliveRef.current || gen !== fetchGenRef.current) return;
        setError(e instanceof Error ? e.message : String(e));
      } finally {
        fetchInFlightRef.current = false;
        if (aliveRef.current) setLoading(false);
        // Trailing-edge: if events (or a blocked fetch from a just-switched
        // session) arrived mid-fetch, fire exactly one more. NOT gated on gen —
        // scheduleTail always targets the CURRENT session, so this correctly
        // loads the new session after a switch-during-fetch; the stale WRITE
        // above is what gen-guards, not this re-schedule.
        if (pendingRef.current && aliveRef.current) {
          pendingRef.current = false;
          // Respect the rate cap on the trailing fetch too.
          scheduleTailRef.current?.();
        }
      }
    },
    [ipc, sessionId, kindsArg],
  );

  // ----- debounced + rate-capped tail scheduler -----------------------------
  const scheduleTailRef = useRef<(() => void) | null>(null);
  const scheduleFullRef = useRef<(() => void) | null>(null);

  const scheduleTail = useCallback(() => {
    if (!aliveRef.current) return;
    if (debounceRef.current) clearTimeout(debounceRef.current);
    const sinceLast = Date.now() - lastFetchAtRef.current;
    const wait = Math.max(TAIL_DEBOUNCE_MS, TAIL_MIN_INTERVAL_MS - sinceLast);
    debounceRef.current = setTimeout(() => {
      debounceRef.current = null;
      void runFetch(false);
    }, wait);
  }, [runFetch]);
  scheduleTailRef.current = scheduleTail;

  const scheduleFull = useCallback(() => {
    if (!aliveRef.current) return;
    if (debounceRef.current) clearTimeout(debounceRef.current);
    debounceRef.current = setTimeout(() => {
      debounceRef.current = null;
      void runFetch(true);
    }, TAIL_DEBOUNCE_MS);
  }, [runFetch]);
  scheduleFullRef.current = scheduleFull;

  // ----- terminal-session map refresh (terminalSessionId -> instanceId) ------
  const refreshTerminalMap = useCallback(async () => {
    if (termRefreshInFlightRef.current) return;
    termRefreshInFlightRef.current = true;
    try {
      const list = await liveIpc.listTerminalSessions();
      if (!aliveRef.current) return;
      const map = new Map<string, string | null>();
      for (const s of list) map.set(s.sessionId, s.instanceId);
      termMapRef.current = map;
    } catch {
      // Non-fatal: an unknown terminal id simply won't correlate this tick; the
      // 10s reconcile net + the next swarm event still keep the tail honest.
    } finally {
      termRefreshInFlightRef.current = false;
    }
  }, [liveIpc]);

  // ----- reset baseline when the focused session / live / kind changes -------
  useEffect(() => {
    // Invalidate any in-flight fetch from the previous session/mode so its
    // post-await write is discarded (see fetchGenRef).
    fetchGenRef.current += 1;
    entriesRef.current = [];
    lastTsRef.current = null;
    termMapRef.current = new Map();
    setEntries([]);
    setSourceStatus(null);
    setTruncated(false);
    setTruncatedHead(false);
    setTerminalReached(false);
    setError(null);
  }, [sessionId, live]);

  // ----- initial live load + subscriptions ----------------------------------
  useEffect(() => {
    aliveRef.current = true;
    if (!active) return;

    // Seed the live model with a full load so the tail cursor has a baseline.
    void runFetch(true);

    let unSwarm: (() => void) | undefined;
    let unTerm: (() => void) | undefined;

    // Only swarm/agent sessions correlate to push events. Chat sessions rely on
    // the visibility-gated reconcile net below (honest "polled" status).
    if (isStreamingKind) {
      void liveIpc
        .subscribeBoardEvents(
          (delta) => {
            const key = eventInstanceKey(delta.event);
            if (key !== sessionId) return;
            const term = eventTerminalState(delta.event);
            if (term && term.key === sessionId) {
              // Final tail to capture the closing rows, then flip to idle.
              void runFetch(false);
              setTerminalReached(true);
              onTerminal?.();
              return;
            }
            scheduleTail();
          },
          () => {
            // swarm resync: full refetch (never apply a partial stream blind).
            scheduleFull();
          },
        )
        .then((u) => {
          if (aliveRef.current) unSwarm = u;
          else u();
        });

      void refreshTerminalMap();
      void liveIpc
        .subscribeTerminal({
          onOutput: (termId) => {
            const mapped = termMapRef.current.get(termId);
            if (mapped === undefined) {
              // Unknown terminal id: learn it once, then a later tick correlates.
              void refreshTerminalMap();
              return;
            }
            if (mapped === sessionId) scheduleTail();
          },
          onExit: (termId) => {
            const mapped = termMapRef.current.get(termId);
            if (mapped === sessionId) {
              void runFetch(false);
              setTerminalReached(true);
              onTerminal?.();
            }
          },
          onResync: (termId) => {
            const mapped = termMapRef.current.get(termId);
            // Unknown id could be ours (map not yet learned) -> be safe + refetch.
            if (mapped === sessionId || mapped === undefined) scheduleFull();
          },
        })
        .then((u) => {
          if (aliveRef.current) unTerm = u;
          else u();
        });
    }

    // Slow visibility-gated reconcile net (Prefect WS-down pattern). Covers a
    // missed event with bounded cost; for chat sessions it is the ONLY signal.
    intervalRef.current = setInterval(() => {
      if (document.visibilityState === "visible") scheduleFull();
    }, LIVE_RECONCILE_MS);

    return () => {
      aliveRef.current = false;
      unSwarm?.();
      unTerm?.();
      if (debounceRef.current) clearTimeout(debounceRef.current);
      if (intervalRef.current) clearInterval(intervalRef.current);
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [active, isStreamingKind, sessionId]);

  // Re-baseline the tail cursor after a kind-filter change so the next tail
  // fetch uses the same lane set as the post-hoc query (a full refetch).
  const firstKindRef = useRef(true);
  useEffect(() => {
    if (firstKindRef.current) {
      firstKindRef.current = false;
      return;
    }
    if (active) scheduleFull();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [activeKindList]);

  const status = liveStatusFor(sessionKind, terminalReached, live);

  return { entries, sourceStatus, truncated, truncatedHead, status, error, loading };
}

/** The inner panel body, only mounted once the disclosure is first opened. */
function SessionReplayBody({
  ipc,
  liveIpc,
  focusSessionId,
  focusSignal,
}: {
  ipc: SessionTranscriptIpc;
  liveIpc: LiveTailIpc;
  focusSessionId?: string | null;
  focusSignal?: number;
}) {
  const [sessions, setSessions] = useState<SessionSummary[]>([]);
  const [listLoading, setListLoading] = useState(true);
  const [listError, setListError] = useState<string | null>(null);

  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [response, setResponse] = useState<SessionTranscriptResponse | null>(null);
  const [transcriptLoading, setTranscriptLoading] = useState(false);
  const [transcriptError, setTranscriptError] = useState<string | null>(null);

  // Live mode. Default ON for a streaming (swarm/agent) session, so opening an
  // active session lands directly in a live-tailing view; OFF/idle otherwise.
  // The operator can toggle it; selecting a new session re-applies the default.
  const [live, setLive] = useState(true);
  const liveUserSetRef = useRef(false);

  // Active kind filter (all on by default). Toggling re-queries the backend
  // (server-side filter) AND refilters client-side for instant response.
  const [activeKinds, setActiveKinds] = useState<Set<TranscriptKind>>(
    () => new Set(TRANSCRIPT_KINDS.map((k) => k.kind)),
  );

  // One-shot board focus guard, keyed by focusSignal (a repeat "Review session"
  // click re-arms it). setState happens only after the awaited list load, so we
  // never call setState synchronously in the effect body.
  const focusedSignal = useRef<number | undefined>(undefined);

  const loadSessions = useCallback(async () => {
    try {
      const list = await ipc.listSessions();
      setSessions(list);
      setListError(null);
      // Apply a one-shot board focus per focusSignal: select the requested
      // session id if the index contains it.
      if (focusedSignal.current !== focusSignal && focusSessionId) {
        const target = list.find((s) => s.sessionId === focusSessionId);
        if (target) {
          focusedSignal.current = focusSignal;
          setSelectedId(target.sessionId);
          return;
        }
      }
      // Otherwise drop a stale selection if it vanished from the index; never
      // auto-select a session — the operator picks what to review.
      setSelectedId((prev) => (prev && !list.some((s) => s.sessionId === prev) ? null : prev));
    } catch (e) {
      setListError(e instanceof Error ? e.message : String(e));
    } finally {
      setListLoading(false);
    }
  }, [ipc, focusSessionId, focusSignal]);

  useEffect(() => {
    // loadSessions awaits ipc.listSessions() BEFORE any setState, so this does
    // not synchronously update state in the effect body (mirrors TerminalPanel's
    // refresh) and the set-state-in-effect rule does not fire.
    void loadSessions();
  }, [loadSessions]);

  const activeKindList = useMemo(
    () => TRANSCRIPT_KINDS.map((k) => k.kind).filter((k) => activeKinds.has(k)),
    [activeKinds],
  );

  const selectedKind = useMemo(
    () => sessions.find((s) => s.sessionId === selectedId)?.kind,
    [sessions, selectedId],
  );
  const selectedIsStreaming =
    selectedKind !== undefined && LIVE_STREAMING_KINDS.has(selectedKind);

  // Selecting a new session re-applies the default Live state (ON for a
  // streaming kind) unless the operator explicitly overrode it for THIS session.
  useEffect(() => {
    liveUserSetRef.current = false;
    setLive(true);
  }, [selectedId]);

  // The live tail owns the entries when Live is on AND the session streams. For a
  // chat session (polled) or Live off, the post-hoc loadTranscript path drives.
  const liveOwnsEntries = live && selectedIsStreaming;

  const liveTail = useLiveTranscriptTail({
    ipc,
    liveIpc,
    sessionId: liveOwnsEntries ? selectedId : null,
    sessionKind: selectedKind,
    live,
    activeKindList,
  });

  const loadTranscript = useCallback(
    async (sessionId: string, kinds: TranscriptKind[]) => {
      setTranscriptLoading(true);
      try {
        const res = await ipc.getTranscript({
          sessionId,
          // Send the active kinds so the backend can filter server-side. When all
          // are on we send null (no restriction) to avoid an over-specified query.
          kinds: kinds.length === TRANSCRIPT_KINDS.length ? null : kinds,
        });
        setResponse(res);
        setTranscriptError(null);
      } catch (e) {
        setTranscriptError(e instanceof Error ? e.message : String(e));
        setResponse(null);
      } finally {
        setTranscriptLoading(false);
      }
    },
    [ipc],
  );

  useEffect(() => {
    if (!selectedId) {
      setResponse(null);
      return;
    }
    // When the live tail owns the entries (Live on + streaming session), it does
    // its OWN full load + tail re-fetches; the post-hoc path must not also fetch
    // (that would double-load and fight the live cursor).
    if (liveOwnsEntries) return;
    // loadTranscript awaits before any setState, same await-boundary rationale.
    void loadTranscript(selectedId, activeKindList);
  }, [selectedId, activeKindList, loadTranscript, liveOwnsEntries]);

  const toggleKind = useCallback((kind: TranscriptKind) => {
    setActiveKinds((prev) => {
      const next = new Set(prev);
      if (next.has(kind)) {
        // Never allow zero active lanes — keep at least one so the timeline is
        // not an ambiguous blank. Toggling off the last lane is a no-op.
        if (next.size === 1) return prev;
        next.delete(kind);
      } else {
        next.add(kind);
      }
      return next;
    });
  }, []);

  const toggleLive = useCallback(() => {
    liveUserSetRef.current = true;
    setLive((p) => !p);
  }, []);

  // The effective response the timeline renders. Live mode projects the live-tail
  // controller into the SAME SessionTranscriptResponse shape (single render path,
  // no forked timeline component). Otherwise the post-hoc fetch result.
  const effectiveResponse: SessionTranscriptResponse | null = useMemo(() => {
    if (liveOwnsEntries && selectedId) {
      if (!liveTail.sourceStatus && liveTail.entries.length === 0 && liveTail.loading) {
        return null; // still seeding the first live load -> show the loader
      }
      return {
        sessionId: selectedId,
        entries: liveTail.entries,
        sourceStatus:
          liveTail.sourceStatus ?? { chat: "empty", fr: "empty", terminal: "empty", process: "empty" },
        truncated: liveTail.truncated,
      };
    }
    return response;
  }, [liveOwnsEntries, selectedId, liveTail, response]);

  const effectiveLoading = liveOwnsEntries ? liveTail.loading : transcriptLoading;
  const effectiveError = liveOwnsEntries ? liveTail.error : transcriptError;
  // The honest status chip. Chat sessions are "polled"; streaming sessions are
  // "live"/"ended"; Live off (or no session) is "idle".
  const liveStatus: LiveStatus = !selectedId
    ? "idle"
    : liveOwnsEntries
      ? liveTail.status
      : liveStatusFor(selectedKind, false, live);

  return (
    <div className="session-replay" data-testid="session-replay-body">
      {/* Filter bar */}
      <div
        className="session-replay__filters"
        data-testid="session-replay-filters"
        role="group"
        aria-label="Filter transcript by kind"
        style={{ display: "flex", gap: 6, alignItems: "center", marginBottom: 8, flexWrap: "wrap" }}
      >
        <span style={{ fontSize: 11, color: "var(--hs-color-text-subtle)", fontWeight: 600 }}>Show:</span>
        {TRANSCRIPT_KINDS.map((k) => {
          const on = activeKinds.has(k.kind);
          const style = KIND_STYLE[k.kind];
          return (
            <button
              key={k.kind}
              type="button"
              data-testid={`session-replay-filter-${k.kind}`}
              data-active={on ? "true" : "false"}
              aria-pressed={on}
              onClick={() => toggleKind(k.kind)}
              style={{
                fontSize: 11,
                padding: "2px 8px",
                borderRadius: 8,
                border: on ? `1px solid ${style.fg}` : "1px solid var(--hs-color-border, #d1d5db)",
                background: on ? style.bg : "var(--hs-color-surface)",
                color: on ? style.fg : "var(--hs-color-text-subtle)",
                cursor: "pointer",
              }}
            >
              {k.label}
            </button>
          );
        })}

        {/* Live toggle + honest status chip. Pushed to the right of the filters. */}
        <div style={{ marginLeft: "auto", display: "flex", gap: 6, alignItems: "center" }}>
          <LiveStatusChip status={liveStatus} />
          <button
            type="button"
            data-testid="session-replay-live-toggle"
            data-active={live ? "true" : "false"}
            aria-pressed={live}
            disabled={!selectedId}
            onClick={toggleLive}
            title={
              !selectedId
                ? "Select a session to tail it live"
                : live
                  ? "Live tailing is ON (updates as the session runs)"
                  : "Live tailing is OFF (post-hoc review)"
            }
            style={{
              fontSize: 11,
              padding: "2px 10px",
              borderRadius: 8,
              border: live ? "1px solid #166534" : "1px solid var(--hs-color-border, #d1d5db)",
              background: live ? "#dcfce7" : "var(--hs-color-surface)",
              color: live ? "#166534" : "var(--hs-color-text-subtle)",
              cursor: selectedId ? "pointer" : "not-allowed",
              opacity: selectedId ? 1 : 0.5,
              fontWeight: 600,
            }}
          >
            {live ? "● Live" : "○ Live"}
          </button>
        </div>
      </div>

      <div className="session-replay__split" style={{ display: "flex", gap: 0, minWidth: 0 }}>
        <SessionList
          sessions={sessions}
          selectedId={selectedId}
          onSelect={setSelectedId}
          loading={listLoading}
          error={listError}
        />
        <TranscriptTimeline
          selectedId={selectedId}
          response={effectiveResponse}
          loading={effectiveLoading}
          error={effectiveError}
          activeKinds={activeKinds}
          live={liveOwnsEntries}
          truncatedHead={liveOwnsEntries ? liveTail.truncatedHead : false}
        />
      </div>
    </div>
  );
}

/** The honest live-status chip (live | polled | ended | idle) for tests + ops. */
function LiveStatusChip({ status }: { status: LiveStatus }) {
  const map: Record<LiveStatus, { label: string; bg: string; fg: string }> = {
    live: { label: "live", bg: "#dcfce7", fg: "#166534" },
    polled: { label: "live · polled", bg: "#fef9c3", fg: "#854d0e" },
    ended: { label: "live · ended", bg: "#f3f4f6", fg: "#6b7280" },
    idle: { label: "idle", bg: "#f3f4f6", fg: "#9ca3af" },
  };
  const s = map[status];
  return (
    <span
      data-testid="session-replay-live-status"
      data-status={status}
      style={{
        fontSize: 10,
        padding: "1px 7px",
        borderRadius: 8,
        background: s.bg,
        color: s.fg,
        whiteSpace: "nowrap",
        fontWeight: 600,
      }}
    >
      {s.label}
    </span>
  );
}

/**
 * The off-main-window Session Replay drawer. Collapsed-by-default + lazy: nothing
 * in the body (session index fetch, transcript fetch) mounts until first opened.
 */
export function SessionReplayPanel({
  ipc = defaultSessionTranscriptIpc,
  liveIpc = defaultLiveTailIpc,
  defaultOpen = false,
  openSignal,
  focusSessionId,
}: SessionReplayPanelProps) {
  return (
    <Disclosure
      id="session-replay"
      title="Session Replay"
      defaultOpen={defaultOpen}
      lazy
      openSignal={openSignal}
      data-testid="session-replay-panel"
    >
      <SessionReplayBody
        ipc={ipc}
        liveIpc={liveIpc}
        focusSessionId={focusSessionId}
        focusSignal={openSignal}
      />
    </Disclosure>
  );
}
