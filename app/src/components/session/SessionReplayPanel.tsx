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
  type ExportFormat,
  type LiveTailIpc,
  type SearchSnippet,
  type SessionExportResponse,
  type SessionSearchHit,
  type SessionSearchRequest,
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
  /**
   * ROI #3 STATE RECOVERY: invoked after a successful one-click Resume of a
   * recorded session. `newComposite` is the FRESH session's composite instance
   * id; `originSessionId` is the resumed-from session id (lineage). The host
   * routes focus into the workbench (chat + terminal + transcript) for the new
   * session. Absent => the panel still resumes (and refreshes the index) but
   * does not hand focus to a host surface.
   */
  onResumed?: (newComposite: string, originSessionId: string) => void;
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

/** The export formats, in display order, for the per-row format <select>. */
const EXPORT_FORMATS: { value: ExportFormat; label: string }[] = [
  { value: "both", label: "MD + JSON" },
  { value: "markdown", label: "Markdown" },
  { value: "json", label: "JSON" },
];

/** Human byte size (small files; whole KiB above 1024). */
function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  return `${(bytes / 1024).toFixed(1)} KiB`;
}

/**
 * A copy-to-clipboard affordance for a path/dir. Best-effort: when the clipboard
 * API is unavailable (older jsdom / non-secure context) the click is a no-op,
 * never a throw. Shows a brief "Copied" acknowledgement on success.
 */
function CopyButton({ value, testId, label = "Copy" }: { value: string; testId: string; label?: string }) {
  const [copied, setCopied] = useState(false);
  const copy = useCallback(() => {
    const clip = typeof navigator !== "undefined" ? navigator.clipboard : undefined;
    if (clip && typeof clip.writeText === "function") {
      void clip.writeText(value).then(
        () => {
          setCopied(true);
          window.setTimeout(() => setCopied(false), 1200);
        },
        () => {
          /* clipboard denied — leave the path visible/selectable, no throw. */
        },
      );
    }
  }, [value]);
  return (
    <button
      type="button"
      data-testid={testId}
      onClick={copy}
      title={`Copy ${value}`}
      style={{
        fontSize: 10,
        padding: "0 6px",
        borderRadius: 6,
        border: "1px solid var(--hs-color-border, #d1d5db)",
        background: "var(--hs-color-surface)",
        color: "var(--hs-color-text-subtle)",
        cursor: "pointer",
        whiteSpace: "nowrap",
        flex: "0 0 auto",
      }}
    >
      {copied ? "Copied" : label}
    </button>
  );
}

/**
 * ROI #5 EXPORT cluster: a per-row format <select> + Export button + honest
 * result/error surface. A SIBLING of the row-select button (never nested), so
 * the overlap-free hit-target discipline of the Resume affordance applies. The
 * written path is shown + copyable; redaction telemetry and an empty flag are
 * surfaced as trust chips; a backend error (e.g. `SESSION_NOT_FOUND:`) renders
 * verbatim, never swallowed. No shell-open in v1 (quiet — no foreground window).
 */
function ExportCluster({
  sessionId,
  format,
  exporting,
  result,
  error,
  onFormatChange,
  onExport,
}: {
  sessionId: string;
  format: ExportFormat;
  exporting: boolean;
  result: SessionExportResponse | null;
  error: string | null;
  onFormatChange: (id: string, format: ExportFormat) => void;
  onExport: (id: string, format: ExportFormat) => void;
}) {
  const firstPath = result && result.files.length > 0 ? result.files[0].path : null;
  const moreCount = result ? Math.max(0, result.files.length - 1) : 0;
  return (
    <div
      className="session-replay__export"
      data-testid={`session-replay-export-cluster-${sessionId}`}
      style={{ padding: "0 10px 8px 13px", display: "flex", flexDirection: "column", gap: 4 }}
    >
      <div style={{ display: "flex", gap: 6, alignItems: "center", flexWrap: "wrap" }}>
        <select
          data-testid={`session-replay-export-format-${sessionId}`}
          aria-label="Export format"
          value={format}
          disabled={exporting}
          onChange={(e) => onFormatChange(sessionId, e.target.value as ExportFormat)}
          style={{
            fontSize: 11,
            padding: "2px 6px",
            borderRadius: 6,
            border: "1px solid var(--hs-color-border, #d1d5db)",
            background: "var(--hs-color-surface)",
            color: "var(--hs-color-text)",
          }}
        >
          {EXPORT_FORMATS.map((f) => (
            <option key={f.value} value={f.value}>
              {f.label}
            </option>
          ))}
        </select>
        <button
          type="button"
          data-testid={`session-replay-export-${sessionId}`}
          data-stable-id={`session-replay.export.${sessionId}`}
          disabled={exporting}
          onClick={() => onExport(sessionId, format)}
          title="Export this session to a portable, secret-redacted file (markdown/json/both)"
          style={{
            fontSize: 11,
            padding: "2px 10px",
            borderRadius: 8,
            border: "1px solid #1e3a8a",
            background: exporting ? "var(--hs-color-surface)" : "#dbeafe",
            color: "#1e3a8a",
            cursor: exporting ? "not-allowed" : "pointer",
            opacity: exporting ? 0.6 : 1,
            fontWeight: 600,
          }}
        >
          {exporting ? "Exporting…" : "⤓ Export"}
        </button>
      </div>

      {result ? (
        <div
          data-testid={`session-replay-exported-${sessionId}`}
          style={{ display: "flex", flexDirection: "column", gap: 3, fontSize: 11, color: "#166534" }}
        >
          {firstPath ? (
            <div style={{ display: "flex", gap: 6, alignItems: "baseline", flexWrap: "wrap" }}>
              <span style={{ wordBreak: "break-all", minWidth: 0 }}>
                Exported → {firstPath}
                {result.files.length > 0 ? ` (${formatBytes(result.files[0].bytes)})` : ""}
                {moreCount > 0 ? ` +${moreCount} more` : ""}
              </span>
              <CopyButton value={firstPath} testId={`session-replay-export-copy-${sessionId}`} label="Copy path" />
            </div>
          ) : null}
          <div style={{ display: "flex", gap: 6, alignItems: "baseline", flexWrap: "wrap" }}>
            <span
              data-testid={`session-replay-export-folder-${sessionId}`}
              style={{ color: "var(--hs-color-text-subtle)", wordBreak: "break-all", minWidth: 0 }}
            >
              in {result.destDir}
            </span>
            <CopyButton value={result.destDir} testId={`session-replay-export-copy-folder-${sessionId}`} label="Copy folder" />
          </div>
          <div style={{ display: "flex", gap: 4, alignItems: "center", flexWrap: "wrap" }}>
            <span
              data-testid={`session-replay-export-redacted-${sessionId}`}
              title="Pattern-based secret redaction applied before writing"
              style={{
                fontSize: 10,
                padding: "0 6px",
                borderRadius: 8,
                background: result.redactedFieldCount > 0 ? "#fef3c7" : "var(--hs-color-surface-muted, #f3f4f6)",
                color: result.redactedFieldCount > 0 ? "#78350f" : "var(--hs-color-text-subtle)",
                whiteSpace: "nowrap",
              }}
            >
              {result.redactedFieldCount} secret{result.redactedFieldCount === 1 ? "" : "s"} redacted
            </span>
            {result.empty ? (
              <span
                data-testid={`session-replay-export-empty-${sessionId}`}
                title="The session had no recorded entries; an empty-but-valid file was written."
                style={{
                  fontSize: 10,
                  padding: "0 6px",
                  borderRadius: 8,
                  background: "var(--hs-color-surface-muted, #f3f4f6)",
                  color: "var(--hs-color-text-subtle)",
                  whiteSpace: "nowrap",
                }}
              >
                empty session
              </span>
            ) : null}
          </div>
        </div>
      ) : null}

      {error ? (
        <span
          data-testid={`session-replay-export-error-${sessionId}`}
          style={{ fontSize: 11, color: "#dc2626", wordBreak: "break-word" }}
        >
          {error}
        </span>
      ) : null}
    </div>
  );
}

/** The session index (left rail). */
function SessionList({
  sessions,
  selectedId,
  onSelect,
  loading,
  error,
  onResume,
  resumingId,
  resumeError,
  resumedLineage,
  onExport,
  exportingId,
  exportFormatFor,
  onExportFormatChange,
  exportResult,
  exportError,
}: {
  sessions: SessionSummary[];
  selectedId: string | null;
  onSelect: (id: string) => void;
  loading: boolean;
  error: string | null;
  /** ROI #3: one-click resume of the row's recorded session (composite id). */
  onResume: (id: string) => void;
  /** The session id whose resume is in flight (disables its button), or null. */
  resumingId: string | null;
  /** The most recent resume error keyed by the row it came from, or null. */
  resumeError: { sessionId: string; message: string } | null;
  /** The most recent successful resume lineage (new + origin), or null. */
  resumedLineage: { newComposite: string; originSessionId: string } | null;
  /** ROI #5: export the row's recorded session to a file in the chosen format. */
  onExport: (id: string, format: ExportFormat) => void;
  /** The session id whose export is in flight (disables its button), or null. */
  exportingId: string | null;
  /** The chosen export format for a row (defaults to "both"). */
  exportFormatFor: (id: string) => ExportFormat;
  /** Update the chosen export format for a row. */
  onExportFormatChange: (id: string, format: ExportFormat) => void;
  /** The most recent successful export keyed by the row it came from, or null. */
  exportResult: { sessionId: string; response: SessionExportResponse } | null;
  /** The most recent export error keyed by the row it came from, or null. */
  exportError: { sessionId: string; message: string } | null;
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
          // ROI #3: a row is resumable only when the backend captured a spawn
          // template for it (swarm spawns). HONEST: chat sessions + pre-feature
          // spawns have resumable=false, so the Resume affordance is hidden.
          const resumable = s.resumable === true;
          const resuming = resumingId === s.sessionId;
          const rowResumeError =
            resumeError && resumeError.sessionId === s.sessionId ? resumeError.message : null;
          const rowLineage =
            resumedLineage && resumedLineage.originSessionId === s.sessionId
              ? resumedLineage
              : null;
          // ROI #5: export is available on EVERY recorded row (any recorded
          // session can be exported — unlike Resume, it is not gated on a
          // captured spawn template). Per-row in-flight + honest result/error.
          const exporting = exportingId === s.sessionId;
          const rowExportFormat = exportFormatFor(s.sessionId);
          const rowExportResult =
            exportResult && exportResult.sessionId === s.sessionId ? exportResult.response : null;
          const rowExportError =
            exportError && exportError.sessionId === s.sessionId ? exportError.message : null;
          return (
            <li
              key={s.sessionId}
              style={{
                borderLeft: selected ? "3px solid #2563eb" : "3px solid transparent",
                background: selected ? "#eff6ff" : "transparent",
              }}
            >
              <button
                type="button"
                data-testid={`session-replay-row-${s.sessionId}`}
                data-stable-id={`session-replay.row.${s.sessionId}`}
                data-selected={selected ? "true" : "false"}
                data-resumable={resumable ? "true" : "false"}
                aria-pressed={selected}
                onClick={() => onSelect(s.sessionId)}
                style={{
                  display: "block",
                  width: "100%",
                  textAlign: "left",
                  padding: "8px 10px",
                  border: "none",
                  background: "transparent",
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

              {/* ROI #3 STATE RECOVERY: the Resume affordance is a SIBLING of the
                  row-select button (never nested — no button-in-button). Rendered
                  only when the session is resumable (a captured spawn template
                  exists). One click re-spawns a fresh session carrying the
                  original's config via the SAME validated spawn path. */}
              {resumable ? (
                <div style={{ padding: "0 10px 8px 13px", display: "flex", flexDirection: "column", gap: 4 }}>
                  <button
                    type="button"
                    data-testid={`session-replay-resume-${s.sessionId}`}
                    data-stable-id={`session-replay.resume.${s.sessionId}`}
                    disabled={resuming}
                    onClick={() => onResume(s.sessionId)}
                    title="Re-spawn a fresh session with this session's recorded config (provider/model/artifact/worktree)"
                    style={{
                      alignSelf: "flex-start",
                      fontSize: 11,
                      padding: "2px 10px",
                      borderRadius: 8,
                      border: "1px solid #166534",
                      background: resuming ? "var(--hs-color-surface)" : "#dcfce7",
                      color: "#166534",
                      cursor: resuming ? "not-allowed" : "pointer",
                      opacity: resuming ? 0.6 : 1,
                      fontWeight: 600,
                    }}
                  >
                    {resuming ? "Resuming…" : "↻ Resume"}
                  </button>
                  {rowLineage ? (
                    <span
                      data-testid="session-replay-resumed-lineage"
                      data-new-composite={rowLineage.newComposite}
                      data-origin-session-id={rowLineage.originSessionId}
                      style={{ fontSize: 11, color: "#166534" }}
                    >
                      Resumed → {shortId(rowLineage.newComposite, 18)} (from {shortId(rowLineage.originSessionId, 14)})
                    </span>
                  ) : null}
                  {rowResumeError ? (
                    <span
                      data-testid={`session-replay-resume-error-${s.sessionId}`}
                      style={{ fontSize: 11, color: "#dc2626", wordBreak: "break-word" }}
                    >
                      {rowResumeError}
                    </span>
                  ) : null}
                </div>
              ) : null}

              {/* ROI #5 EXPORT: a per-row sibling cluster (never nested in the
                  row-select button). A format choice (markdown/json/both,
                  default both) + an Export button -> calls kernel_session_export
                  and surfaces the written path(s) with a copy affordance, plus
                  HONEST redaction telemetry + an empty chip + a verbatim error. */}
              <ExportCluster
                sessionId={s.sessionId}
                format={rowExportFormat}
                exporting={exporting}
                result={rowExportResult}
                error={rowExportError}
                onFormatChange={onExportFormatChange}
                onExport={onExport}
              />
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

// ===========================================================================
// ROI #4 RECALL: cross-session free-text search.
//
// A search box + structured filters (kind / worktree / time range) sits ABOVE
// the session index. Submitting calls kernel_session_search; the ranked hits
// (each with redacted snippets + a match-count badge) REPLACE the session list
// in the left rail. Clicking a hit drives the EXISTING select path (setSelectedId
// -> the post-hoc/live transcript machinery), opening that session's transcript
// with zero new plumbing. Search is ADDITIVE: when there are no results held
// (searchHits === null) the panel is byte-identical to before, so the list +
// post-hoc review + live tail are untouched.
//
// The backend already redacted every snippet; the only client-side emphasis is a
// cosmetic case-insensitive highlight of the matched term (no secret handling).
// ===========================================================================

/** RFC3339 from a `datetime-local` value (local wall-clock), or null. */
function localInputToRfc3339(value: string): string | null {
  if (!value) return null;
  const d = new Date(value);
  return Number.isNaN(d.getTime()) ? null : d.toISOString();
}

/**
 * Split `text` around case-insensitive occurrences of `term`, returning the
 * matched spans flagged so the caller can emphasize them. Purely cosmetic — the
 * snippet is already redacted; we never re-derive a secret here. Bounds the
 * number of segments so a pathological snippet can't blow up the render.
 */
function highlightSegments(text: string, term: string): { text: string; match: boolean }[] {
  const needle = term.trim().toLowerCase();
  if (!needle) return [{ text, match: false }];
  const out: { text: string; match: boolean }[] = [];
  const hay = text.toLowerCase();
  let i = 0;
  let guard = 0;
  while (i < text.length && guard < 200) {
    const at = hay.indexOf(needle, i);
    if (at === -1) {
      out.push({ text: text.slice(i), match: false });
      break;
    }
    if (at > i) out.push({ text: text.slice(i, at), match: false });
    out.push({ text: text.slice(at, at + needle.length), match: true });
    i = at + needle.length;
    guard += 1;
  }
  if (out.length === 0) out.push({ text, match: false });
  return out;
}

/** One redacted snippet line inside a hit: a kind chip + the highlighted text. */
function SnippetLine({ snippet, term }: { snippet: SearchSnippet; term: string }) {
  const kind = snippet.entryKind as TranscriptKind;
  const style = KIND_STYLE[kind];
  return (
    <div
      className="session-replay__search-snippet"
      data-snippet-kind={snippet.entryKind}
      style={{ display: "flex", gap: 6, alignItems: "baseline", marginTop: 3 }}
    >
      <span
        style={{
          fontSize: 9,
          padding: "0 5px",
          borderRadius: 8,
          background: style ? style.bg : "var(--hs-color-surface-muted, #f3f4f6)",
          color: style ? style.fg : "var(--hs-color-text-subtle)",
          whiteSpace: "nowrap",
          flex: "0 0 auto",
        }}
      >
        {style ? style.label : snippet.entryKind}
      </span>
      <span style={{ fontSize: 11, color: "var(--hs-color-text)", wordBreak: "break-word", minWidth: 0 }}>
        {highlightSegments(snippet.snippet, term).map((seg, idx) =>
          seg.match ? (
            <mark key={idx} style={{ background: "#fde68a", color: "inherit", padding: 0 }}>
              {seg.text}
            </mark>
          ) : (
            <span key={idx}>{seg.text}</span>
          ),
        )}
      </span>
    </div>
  );
}

/**
 * The search results (left rail), shown IN PLACE of the session list when a
 * search is active. Each hit shows title/kind/provider/worktree, a match-count
 * badge, and the capped redacted snippets. Clicking a hit selects that session
 * (the existing transcript-open path).
 */
function SessionSearchResults({
  hits,
  query,
  selectedId,
  onSelect,
  loading,
  error,
  truncated,
}: {
  hits: SessionSearchHit[];
  query: string;
  selectedId: string | null;
  onSelect: (id: string) => void;
  loading: boolean;
  error: string | null;
  truncated: boolean;
}) {
  return (
    <div
      className="session-replay__search-results"
      data-testid="session-replay-search-results"
      style={{
        width: 280,
        flex: "0 0 280px",
        borderRight: "1px solid var(--hs-color-border, #e5e7eb)",
        overflow: "auto",
        maxHeight: 520,
      }}
    >
      {error ? (
        <div
          data-testid="session-replay-search-error"
          style={{ color: "#dc2626", fontSize: 12, padding: 8 }}
        >
          Search error: {error}
        </div>
      ) : null}
      {!error && loading ? (
        <div style={{ color: "var(--hs-color-text-subtle)", fontSize: 13, padding: 8 }}>Searching…</div>
      ) : null}
      {!error && !loading && hits.length === 0 ? (
        <div
          data-testid="session-replay-search-empty"
          style={{ color: "var(--hs-color-text-subtle)", fontSize: 13, padding: 12 }}
        >
          No sessions match “{query}”.
        </div>
      ) : null}
      {!error && truncated && hits.length > 0 ? (
        <div
          data-testid="session-replay-search-truncated"
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
          more matches than shown — narrow the query
        </div>
      ) : null}
      <ul style={{ listStyle: "none", margin: 0, padding: 0 }}>
        {hits.map((hit) => {
          const selected = hit.sessionId === selectedId;
          return (
            <li
              key={hit.sessionId}
              style={{
                borderLeft: selected ? "3px solid #2563eb" : "3px solid transparent",
                background: selected ? "#eff6ff" : "transparent",
              }}
            >
              <button
                type="button"
                data-testid={`session-replay-search-hit-${hit.sessionId}`}
                data-stable-id={`session-replay.search-hit.${hit.sessionId}`}
                data-selected={selected ? "true" : "false"}
                aria-pressed={selected}
                onClick={() => onSelect(hit.sessionId)}
                style={{
                  display: "block",
                  width: "100%",
                  textAlign: "left",
                  padding: "8px 10px",
                  border: "none",
                  background: "transparent",
                  color: "var(--hs-color-text)",
                  cursor: "pointer",
                  fontSize: 12,
                }}
              >
                <div style={{ display: "flex", gap: 6, alignItems: "center" }}>
                  <span
                    style={{
                      fontWeight: 600,
                      overflow: "hidden",
                      textOverflow: "ellipsis",
                      whiteSpace: "nowrap",
                      minWidth: 0,
                    }}
                  >
                    {hit.title || shortId(hit.sessionId)}
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
                    {hit.kind}
                  </span>
                  <span
                    data-testid={`session-replay-search-matchcount-${hit.sessionId}`}
                    title={`${hit.matchCount} match${hit.matchCount === 1 ? "" : "es"}`}
                    style={{
                      marginLeft: "auto",
                      fontSize: 10,
                      fontWeight: 600,
                      padding: "0 6px",
                      borderRadius: 8,
                      background: "#dbeafe",
                      color: "#1e3a8a",
                      whiteSpace: "nowrap",
                    }}
                  >
                    {hit.matchCount} match{hit.matchCount === 1 ? "" : "es"}
                  </span>
                </div>
                {hit.provider || hit.modelId || hit.worktreeId ? (
                  <div
                    style={{
                      color: "var(--hs-color-text-subtle)",
                      fontSize: 11,
                      marginTop: 1,
                      overflow: "hidden",
                      textOverflow: "ellipsis",
                      whiteSpace: "nowrap",
                    }}
                  >
                    {[hit.provider, hit.modelId, hit.worktreeId].filter(Boolean).join(" · ")}
                  </div>
                ) : null}
                <div style={{ marginTop: 2 }}>
                  {hit.snippets.map((s, idx) => (
                    <SnippetLine key={idx} snippet={s} term={query} />
                  ))}
                </div>
              </button>
            </li>
          );
        })}
      </ul>
    </div>
  );
}

/**
 * The search box + structured filters above the session index. Owns its own
 * input/filter state; on submit it calls back with a built {@link
 * SessionSearchRequest}. Reuses the transcript kind-chip pattern + the distinct
 * worktree ids already loaded in the session index (zero extra IPC).
 */
function SessionSearchBar({
  worktrees,
  onSubmit,
  onClear,
  searching,
  active,
}: {
  /** Distinct worktree ids from the loaded session index (for the worktree select). */
  worktrees: string[];
  onSubmit: (req: SessionSearchRequest) => void;
  onClear: () => void;
  searching: boolean;
  /** True when a search is currently shown (enables the Clear button). */
  active: boolean;
}) {
  const [query, setQuery] = useState("");
  const [kinds, setKinds] = useState<Set<TranscriptKind>>(
    () => new Set(TRANSCRIPT_KINDS.map((k) => k.kind)),
  );
  const [worktree, setWorktree] = useState("");
  const [from, setFrom] = useState("");
  const [to, setTo] = useState("");

  const trimmed = query.trim();
  const canSubmit = trimmed.length > 0 && !searching;

  const toggleKind = useCallback((kind: TranscriptKind) => {
    setKinds((prev) => {
      const next = new Set(prev);
      // Never allow zero lanes — toggling off the last is a no-op (matches the
      // transcript filter discipline so the query is never an empty lane set).
      if (next.has(kind)) {
        if (next.size === 1) return prev;
        next.delete(kind);
      } else {
        next.add(kind);
      }
      return next;
    });
  }, []);

  const submit = useCallback(() => {
    if (!canSubmit) return;
    const allKinds = kinds.size === TRANSCRIPT_KINDS.length;
    onSubmit({
      query: trimmed,
      kinds: allKinds ? null : TRANSCRIPT_KINDS.map((k) => k.kind).filter((k) => kinds.has(k)),
      worktreeId: worktree || null,
      from: localInputToRfc3339(from),
      to: localInputToRfc3339(to),
    });
  }, [canSubmit, kinds, trimmed, worktree, from, to, onSubmit]);

  const clear = useCallback(() => {
    setQuery("");
    onClear();
  }, [onClear]);

  return (
    <div
      className="session-replay__search"
      data-testid="session-replay-search"
      role="search"
      aria-label="Search across recorded sessions"
      style={{
        display: "flex",
        flexDirection: "column",
        gap: 6,
        marginBottom: 8,
        padding: 8,
        border: "1px solid var(--hs-color-border, #e5e7eb)",
        borderRadius: 8,
        background: "var(--hs-color-surface-muted, #f9fafb)",
      }}
    >
      <div style={{ display: "flex", gap: 6, alignItems: "center", flexWrap: "wrap" }}>
        <input
          type="text"
          data-testid="session-replay-search-input"
          aria-label="Search query"
          placeholder="Search across sessions (chat, agent, terminal, FR)…"
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === "Enter") {
              e.preventDefault();
              submit();
            }
          }}
          style={{
            flex: "1 1 240px",
            minWidth: 0,
            fontSize: 12,
            padding: "4px 8px",
            borderRadius: 6,
            border: "1px solid var(--hs-color-border, #d1d5db)",
            background: "var(--hs-color-surface)",
            color: "var(--hs-color-text)",
          }}
        />
        <button
          type="button"
          data-testid="session-replay-search-submit"
          disabled={!canSubmit}
          onClick={submit}
          style={{
            fontSize: 12,
            padding: "4px 12px",
            borderRadius: 6,
            border: "1px solid #2563eb",
            background: canSubmit ? "#2563eb" : "var(--hs-color-surface)",
            color: canSubmit ? "#fff" : "var(--hs-color-text-subtle)",
            cursor: canSubmit ? "pointer" : "not-allowed",
            opacity: canSubmit ? 1 : 0.6,
            fontWeight: 600,
          }}
        >
          {searching ? "Searching…" : "Search"}
        </button>
        <button
          type="button"
          data-testid="session-replay-search-clear"
          disabled={!active && trimmed.length === 0}
          onClick={clear}
          title="Clear the search and restore the session index"
          style={{
            fontSize: 12,
            padding: "4px 10px",
            borderRadius: 6,
            border: "1px solid var(--hs-color-border, #d1d5db)",
            background: "var(--hs-color-surface)",
            color: "var(--hs-color-text-subtle)",
            cursor: !active && trimmed.length === 0 ? "not-allowed" : "pointer",
            opacity: !active && trimmed.length === 0 ? 0.5 : 1,
          }}
        >
          Clear
        </button>
      </div>

      {/* Structured filters: kind chips + worktree select + time range. */}
      <div
        className="session-replay__search-filters"
        data-testid="session-replay-search-filters"
        style={{ display: "flex", gap: 6, alignItems: "center", flexWrap: "wrap" }}
      >
        <span style={{ fontSize: 10, color: "var(--hs-color-text-subtle)", fontWeight: 600 }}>Lanes:</span>
        {TRANSCRIPT_KINDS.map((k) => {
          const on = kinds.has(k.kind);
          const style = KIND_STYLE[k.kind];
          return (
            <button
              key={k.kind}
              type="button"
              data-testid={`session-replay-search-kind-${k.kind}`}
              data-active={on ? "true" : "false"}
              aria-pressed={on}
              onClick={() => toggleKind(k.kind)}
              style={{
                fontSize: 10,
                padding: "1px 7px",
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

        <select
          data-testid="session-replay-search-worktree"
          aria-label="Filter by worktree"
          value={worktree}
          onChange={(e) => setWorktree(e.target.value)}
          style={{
            fontSize: 11,
            padding: "2px 6px",
            borderRadius: 6,
            border: "1px solid var(--hs-color-border, #d1d5db)",
            background: "var(--hs-color-surface)",
            color: "var(--hs-color-text)",
          }}
        >
          <option value="">All worktrees</option>
          {worktrees.map((w) => (
            <option key={w} value={w}>
              {w}
            </option>
          ))}
        </select>

        <label style={{ fontSize: 10, color: "var(--hs-color-text-subtle)", display: "flex", gap: 3, alignItems: "center" }}>
          From
          <input
            type="datetime-local"
            data-testid="session-replay-search-from"
            aria-label="From time"
            value={from}
            onChange={(e) => setFrom(e.target.value)}
            style={{
              fontSize: 11,
              padding: "1px 4px",
              borderRadius: 6,
              border: "1px solid var(--hs-color-border, #d1d5db)",
              background: "var(--hs-color-surface)",
              color: "var(--hs-color-text)",
            }}
          />
        </label>
        <label style={{ fontSize: 10, color: "var(--hs-color-text-subtle)", display: "flex", gap: 3, alignItems: "center" }}>
          To
          <input
            type="datetime-local"
            data-testid="session-replay-search-to"
            aria-label="To time"
            value={to}
            onChange={(e) => setTo(e.target.value)}
            style={{
              fontSize: 11,
              padding: "1px 4px",
              borderRadius: 6,
              border: "1px solid var(--hs-color-border, #d1d5db)",
              background: "var(--hs-color-surface)",
              color: "var(--hs-color-text)",
            }}
          />
        </label>
      </div>
    </div>
  );
}

/** The inner panel body, only mounted once the disclosure is first opened. */
function SessionReplayBody({
  ipc,
  liveIpc,
  focusSessionId,
  focusSignal,
  onResumed,
}: {
  ipc: SessionTranscriptIpc;
  liveIpc: LiveTailIpc;
  focusSessionId?: string | null;
  focusSignal?: number;
  onResumed?: (newComposite: string, originSessionId: string) => void;
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

  // ROI #3 STATE RECOVERY: resume affordance state. `resumingId` disables the
  // in-flight row's button; `resumeError` / `resumedLineage` are honest per-row
  // outcomes shown inline (mirroring the panel's other honest inline states).
  const [resumingId, setResumingId] = useState<string | null>(null);
  const [resumeError, setResumeError] = useState<{ sessionId: string; message: string } | null>(null);
  const [resumedLineage, setResumedLineage] = useState<{ newComposite: string; originSessionId: string } | null>(null);

  // ROI #5 EXPORT: per-row export state. `exportingId` disables the in-flight
  // row's button (single-flight); `exportFormats` holds each row's chosen format
  // (default "both"); `exportResult` / `exportError` are honest per-row outcomes
  // shown inline (mirroring the resume affordance's honest inline states).
  const [exportingId, setExportingId] = useState<string | null>(null);
  const [exportFormats, setExportFormats] = useState<Record<string, ExportFormat>>({});
  const [exportResult, setExportResult] = useState<{ sessionId: string; response: SessionExportResponse } | null>(null);
  const [exportError, setExportError] = useState<{ sessionId: string; message: string } | null>(null);

  // ROI #4 RECALL: cross-session search state. `searchHits === null` means
  // NOT-SEARCHING — the left rail shows the plain session index, so search is
  // purely additive. A non-null array (possibly empty) means a search is active
  // and its results REPLACE the index in the left rail. `searchQuery` is the
  // effective query the backend echoed (used for the empty-state + highlight).
  const [searchHits, setSearchHits] = useState<SessionSearchHit[] | null>(null);
  const [searchQuery, setSearchQuery] = useState("");
  const [searchTruncated, setSearchTruncated] = useState(false);
  const [searchLoading, setSearchLoading] = useState(false);
  const [searchError, setSearchError] = useState<string | null>(null);
  const searchGenRef = useRef(0);

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

  // ROI #3 STATE RECOVERY: one-click resume. Re-spawns a FRESH session via the
  // SAME validated backend spawn path (integrity gate + bounds preserved), then
  // reloads the index so the new session appears and hands focus to the host.
  // HONEST: a backend `SESSION_NOT_RESUMABLE:` (or any factory) error is shown
  // verbatim inline on the row, never swallowed. Single-flight per the disabled
  // button; the `resumingId` guard also blocks re-entrancy.
  const handleResume = useCallback(
    async (sessionId: string) => {
      if (resumingId) return;
      setResumingId(sessionId);
      setResumeError(null);
      try {
        const id = await ipc.resumeSession(sessionId);
        setResumedLineage({ newComposite: id.composite, originSessionId: sessionId });
        // Pull the new (and itself-resumable) session into the index.
        await loadSessions();
        onResumed?.(id.composite, sessionId);
      } catch (e) {
        setResumeError({ sessionId, message: e instanceof Error ? e.message : String(e) });
      } finally {
        setResumingId(null);
      }
    },
    [ipc, loadSessions, onResumed, resumingId],
  );

  // ROI #5 EXPORT: the chosen format for a row, defaulting to "both".
  const exportFormatFor = useCallback(
    (sessionId: string): ExportFormat => exportFormats[sessionId] ?? "both",
    [exportFormats],
  );
  const handleExportFormatChange = useCallback((sessionId: string, format: ExportFormat) => {
    setExportFormats((prev) => ({ ...prev, [sessionId]: format }));
  }, []);

  // ROI #5 EXPORT: export a recorded session to a secret-REDACTED file. Calls the
  // backend `kernel_session_export` (which reuses the aggregator + redactor + the
  // atomic temp+rename writer) and surfaces the written path(s) inline. HONEST: a
  // backend error (e.g. `SESSION_NOT_FOUND:` or an IO failure) is shown verbatim
  // on the row, never swallowed; an empty-but-valid export is flagged honestly.
  // Single-flight per the disabled button; the `exportingId` guard also blocks
  // re-entrancy. The result is keyed by the row so it never leaks onto another.
  const handleExport = useCallback(
    async (sessionId: string, format: ExportFormat) => {
      if (exportingId) return;
      setExportingId(sessionId);
      setExportError(null);
      // Clear a stale result from a previous export of this row so the operator
      // never sees an old path while a new export is in flight.
      setExportResult((prev) => (prev && prev.sessionId === sessionId ? null : prev));
      try {
        const response = await ipc.exportSession({ sessionId, format, destDir: null });
        setExportResult({ sessionId, response });
      } catch (e) {
        setExportError({ sessionId, message: e instanceof Error ? e.message : String(e) });
      } finally {
        setExportingId(null);
      }
    },
    [ipc, exportingId],
  );

  // ROI #4 RECALL: run a cross-session search. A monotonic generation guards
  // against a stale slow search landing after a newer one (or a Clear) — the same
  // discipline the live tail uses for cross-session contamination. HONEST: a
  // backend error (e.g. an empty-query rejection that slips past the disabled
  // button) is surfaced verbatim, never swallowed; an empty hit list renders the
  // distinct empty state rather than a fabricated row.
  const handleSearch = useCallback(
    async (req: SessionSearchRequest) => {
      const gen = (searchGenRef.current += 1);
      setSearchLoading(true);
      setSearchError(null);
      // Show the active (searching) rail immediately with the requested query.
      setSearchQuery(req.query);
      setSearchHits((prev) => prev ?? []);
      try {
        const res = await ipc.searchSessions(req);
        if (gen !== searchGenRef.current) return; // a newer search/clear won.
        setSearchHits(res.hits);
        setSearchQuery(res.query);
        setSearchTruncated(res.truncated);
      } catch (e) {
        if (gen !== searchGenRef.current) return;
        setSearchError(e instanceof Error ? e.message : String(e));
        setSearchHits([]);
      } finally {
        if (gen === searchGenRef.current) setSearchLoading(false);
      }
    },
    [ipc],
  );

  // Clear the search -> restore the plain session index (additive guarantee).
  // Bumps the generation so an in-flight search can't repopulate after clear.
  const handleClearSearch = useCallback(() => {
    searchGenRef.current += 1;
    setSearchHits(null);
    setSearchQuery("");
    setSearchTruncated(false);
    setSearchError(null);
    setSearchLoading(false);
  }, []);

  // Distinct worktree ids from the loaded index, for the worktree filter select
  // (zero extra IPC — reuse the summaries already fetched).
  const worktreeOptions = useMemo(() => {
    const set = new Set<string>();
    for (const s of sessions) if (s.worktreeId) set.add(s.worktreeId);
    return Array.from(set).sort();
  }, [sessions]);

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

      {/* ROI #4 RECALL: cross-session search box + filters above the index. */}
      <SessionSearchBar
        worktrees={worktreeOptions}
        onSubmit={handleSearch}
        onClear={handleClearSearch}
        searching={searchLoading}
        active={searchHits !== null}
      />

      <div className="session-replay__split" style={{ display: "flex", gap: 0, minWidth: 0 }}>
        {searchHits !== null ? (
          <SessionSearchResults
            hits={searchHits}
            query={searchQuery}
            selectedId={selectedId}
            onSelect={setSelectedId}
            loading={searchLoading}
            error={searchError}
            truncated={searchTruncated}
          />
        ) : (
          <SessionList
            sessions={sessions}
            selectedId={selectedId}
            onSelect={setSelectedId}
            loading={listLoading}
            error={listError}
            onResume={handleResume}
            resumingId={resumingId}
            resumeError={resumeError}
            resumedLineage={resumedLineage}
            onExport={handleExport}
            exportingId={exportingId}
            exportFormatFor={exportFormatFor}
            onExportFormatChange={handleExportFormatChange}
            exportResult={exportResult}
            exportError={exportError}
          />
        )}
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
  onResumed,
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
        onResumed={onResumed}
      />
    </Disclosure>
  );
}
