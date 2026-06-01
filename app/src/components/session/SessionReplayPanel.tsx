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
  TRANSCRIPT_KINDS,
  type AgentActivityEntry,
  type SessionSummary,
  type SessionTranscriptEntry,
  type SessionTranscriptIpc,
  type SessionTranscriptResponse,
  type SourceState,
  type TranscriptKind,
} from "../../lib/ipc/session_transcript";

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
}: {
  selectedId: string | null;
  response: SessionTranscriptResponse | null;
  loading: boolean;
  error: string | null;
  activeKinds: Set<TranscriptKind>;
}) {
  // Client-side refilter for instant response (the server also filters by
  // `kinds`, but we refilter so toggling feels immediate before the refetch).
  const visible = useMemo(() => {
    if (!response) return [];
    return response.entries.filter((e) => activeKinds.has(e.kind));
  }, [response, activeKinds]);

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

      {loading && !response ? (
        <div style={{ color: "var(--hs-color-text-subtle)", fontSize: 13, padding: 12 }}>Loading transcript…</div>
      ) : null}

      <ul
        className="session-replay__entries"
        data-testid="session-replay-entries"
        style={{ listStyle: "none", margin: 0, padding: 0, overflow: "auto", maxHeight: 520, flex: 1 }}
      >
        {visible.map((entry) => (
          <TranscriptRow key={`${entry.kind}-${entry.seq}`} entry={entry} />
        ))}
      </ul>

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

/** The inner panel body, only mounted once the disclosure is first opened. */
function SessionReplayBody({
  ipc,
  focusSessionId,
  focusSignal,
}: {
  ipc: SessionTranscriptIpc;
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
    // loadTranscript awaits before any setState, same await-boundary rationale.
    void loadTranscript(selectedId, activeKindList);
  }, [selectedId, activeKindList, loadTranscript]);

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
          response={response}
          loading={transcriptLoading}
          error={transcriptError}
          activeKinds={activeKinds}
        />
      </div>
    </div>
  );
}

/**
 * The off-main-window Session Replay drawer. Collapsed-by-default + lazy: nothing
 * in the body (session index fetch, transcript fetch) mounts until first opened.
 */
export function SessionReplayPanel({
  ipc = defaultSessionTranscriptIpc,
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
      <SessionReplayBody ipc={ipc} focusSessionId={focusSessionId} focusSignal={openSignal} />
    </Disclosure>
  );
}
