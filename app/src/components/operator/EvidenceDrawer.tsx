import React, { useMemo, useState } from "react";
import { createDiagnostic, Diagnostic, FlightEvent, getJob } from "../../lib/api";

export type EvidenceSelection =
  | { kind: "diagnostic"; diagnostic: Diagnostic }
  | { kind: "event"; event: FlightEvent };

type Props = {
  selection: EvidenceSelection | null;
  onClose: () => void;
  onExport?: (selection: EvidenceSelection) => void;
  onNavigateToJob?: (jobId: string) => void;
  onNavigateToTimeline?: (nav: { job_id?: string; wsid?: string; event_id?: string }) => void;
};

type InnerProps = Omit<Props, "selection"> & { selection: EvidenceSelection };

function redactMessage(message: string, visibleChars = 180): string {
  if (message.length <= visibleChars) return message;
  return `${message.slice(0, visibleChars)}... [redacted preview]`;
}

function redactJsonValue(value: unknown, visibleChars = 180, depth = 0): unknown {
  const MAX_DEPTH = 6;
  const MAX_ARRAY = 50;
  if (depth > MAX_DEPTH) return "[redacted: depth limit]";

  if (typeof value === "string") return redactMessage(value, visibleChars);
  if (value === null || value === undefined) return value;
  if (typeof value !== "object") return value;
  if (Array.isArray(value)) return value.slice(0, MAX_ARRAY).map((v) => redactJsonValue(v, visibleChars, depth + 1));

  const obj = value as Record<string, unknown>;
  const out: Record<string, unknown> = {};
  for (const [key, val] of Object.entries(obj)) {
    if (/(token|secret|password|api[_-]?key)/i.test(key)) {
      out[key] = "[redacted]";
      continue;
    }
    out[key] = redactJsonValue(val, visibleChars, depth + 1);
  }
  return out;
}

const EvidenceDrawerInner: React.FC<InnerProps> = ({
  selection,
  onClose,
  onExport,
  onNavigateToJob,
  onNavigateToTimeline,
}) => {
  const [showFullJson, setShowFullJson] = useState(false);

  const redactedSelection = useMemo(() => {
    if (selection.kind === "diagnostic") return redactJsonValue(selection.diagnostic);
    return { ...selection.event, payload: redactJsonValue(selection.event.payload) };
  }, [selection]);

  const emitNavFailure = async (action: string, reason: string) => {
    try {
      const wsid =
        selection.kind === "diagnostic"
          ? selection.diagnostic.wsid ?? null
          : selection.event.wsids?.[0] ?? null;
      const job_id =
        selection.kind === "diagnostic"
          ? selection.diagnostic.job_id ?? null
          : selection.event.job_id ?? null;

      await createDiagnostic({
        title: "VAL-NAV-001 Navigation failure",
        message: `${action}: ${reason}`,
        severity: "warning",
        source: "system",
        surface: "system",
        code: "VAL-NAV-001",
        wsid,
        job_id,
        evidence_refs: selection.kind === "event" ? { fr_event_ids: [selection.event.event_id] } : undefined,
        link_confidence: "unlinked",
      });
    } catch {
      // ignore emission failures
    }
  };

  const onCopyAsCoderPrompt = async () => {
    const context =
      selection.kind === "diagnostic"
        ? [
            `Diagnostic ID: ${selection.diagnostic.id}`,
            `Title: ${selection.diagnostic.title}`,
            `Severity: ${selection.diagnostic.severity}`,
            `Source: ${selection.diagnostic.source}`,
            `Surface: ${selection.diagnostic.surface}`,
            `WSID: ${selection.diagnostic.wsid ?? "n/a"}`,
            `Job ID: ${selection.diagnostic.job_id ?? "n/a"}`,
            `Fingerprint: ${selection.diagnostic.fingerprint}`,
          ]
        : [
            `Event ID: ${selection.event.event_id}`,
            `Event Type: ${selection.event.event_type}`,
            `Timestamp: ${selection.event.timestamp}`,
            `Actor: ${selection.event.actor}`,
            `WSIDs: ${selection.event.wsids.join(", ") || "n/a"}`,
            `Job ID: ${selection.event.job_id ?? "n/a"}`,
            `Trace ID: ${selection.event.trace_id}`,
          ];

    const evidenceJson = JSON.stringify(
      showFullJson
        ? selection.kind === "diagnostic"
          ? selection.diagnostic
          : selection.event
        : redactedSelection,
      null,
      2,
    );
    const prompt = `${context.join("\n")}\n\nEvidence JSON:\n${evidenceJson}\n`;

    try {
      await navigator.clipboard.writeText(prompt);
    } catch {
      await emitNavFailure("copy_as_coder_prompt", "clipboard write failed");
    }
  };

  const onOpenJob = async () => {
    const jobId =
      selection.kind === "diagnostic" ? selection.diagnostic.job_id : selection.event.job_id ?? null;
    if (!jobId) {
      await emitNavFailure("open_job", "missing job_id");
      return;
    }
    try {
      await getJob(jobId);
    } catch {
      await emitNavFailure("open_job", `job not found: ${jobId}`);
      return;
    }
    if (!onNavigateToJob) {
      await emitNavFailure("open_job", "no navigation handler configured");
      return;
    }
    onNavigateToJob(jobId);
  };

  const onOpenTimeline = async () => {
    if (!onNavigateToTimeline) {
      await emitNavFailure("open_timeline", "no navigation handler configured");
      return;
    }

    if (selection.kind === "event") {
      onNavigateToTimeline({
        event_id: selection.event.event_id,
        job_id: selection.event.job_id ?? undefined,
        wsid: selection.event.wsids?.[0],
      });
      return;
    }

    const job_id = selection.diagnostic.job_id ?? undefined;
    const wsid = selection.diagnostic.wsid ?? undefined;
    if (!job_id && !wsid) {
      await emitNavFailure("open_timeline", "missing job_id/wsid");
      return;
    }
    onNavigateToTimeline({ job_id, wsid });
  };

  if (selection.kind === "diagnostic") {
    const diagnostic = selection.diagnostic;
    return (
      <aside className="evidence-drawer">
        <div className="drawer-header">
          <div>
            <p className="drawer-eyebrow">Diagnostic</p>
            <h3>{diagnostic.title}</h3>
            <p className="muted">{redactMessage(diagnostic.message)}</p>
          </div>
          <div className="chip-row">
            <span className="chip chip--strong">{diagnostic.severity}</span>
            <span className="chip">{diagnostic.link_confidence}</span>
            {diagnostic.source && <span className="chip chip--ghost">{diagnostic.source}</span>}
          </div>
        </div>
        <div className="drawer-grid">
          <div>
            <h4>Correlation</h4>
            <ul className="meta-list">
              <li>Fingerprint: {diagnostic.fingerprint}</li>
              <li>Job: {diagnostic.job_id ?? "n/a"}</li>
              <li>Workspace: {diagnostic.wsid ?? "n/a"}</li>
              <li>Actor: {diagnostic.actor ?? "system"}</li>
              <li>Capability: {diagnostic.capability_id ?? "n/a"}</li>
              <li>Policy Decision: {diagnostic.policy_decision_id ?? "n/a"}</li>
              <li>Link Confidence: {diagnostic.link_confidence}</li>
            </ul>
          </div>
          <div>
            <h4>Evidence</h4>
            <ul className="meta-list">
              <li>FR events: {diagnostic.evidence_refs?.fr_event_ids?.join(", ") ?? "n/a"}</li>
              <li>Related jobs: {diagnostic.evidence_refs?.related_job_ids?.join(", ") ?? "n/a"}</li>
              <li>
                Artifact hashes:{" "}
                {diagnostic.evidence_refs?.artifact_hashes
                  ? JSON.stringify(diagnostic.evidence_refs.artifact_hashes)
                  : "n/a"}
              </li>
            </ul>
          </div>
        </div>
        <details className="drawer-raw">
          <summary>Raw JSON (redacted view)</summary>
          <div className="drawer-actions">
            <button className="secondary" onClick={() => setShowFullJson((v) => !v)}>
              {showFullJson ? "Hide full JSON" : "Show full JSON"}
            </button>
          </div>
          <pre>{JSON.stringify(showFullJson ? diagnostic : redactedSelection, null, 2)}</pre>
        </details>
        <div className="drawer-actions">
          <button className="secondary" onClick={onClose}>
            Close
          </button>
          <button className="secondary" onClick={onCopyAsCoderPrompt}>
            Copy as coder prompt
          </button>
          <button className="secondary" onClick={onOpenJob}>
            Open Job
          </button>
          <button className="secondary" onClick={onOpenTimeline}>
            Open Timeline
          </button>
          <button
            className="primary"
            onClick={() => onExport && onExport(selection)}
            disabled={!onExport}
          >
            Export Debug Bundle
          </button>
        </div>
      </aside>
    );
  }

  const event = selection.event;
  return (
    <aside className="evidence-drawer">
      <div className="drawer-header">
        <div>
          <p className="drawer-eyebrow">Flight Recorder Event</p>
          <h3>{event.event_type}</h3>
          <p className="muted">Actor: {event.actor} | Trace: {event.trace_id}</p>
        </div>
        <div className="chip-row">
          <span className="chip chip--ghost">Job: {event.job_id ?? "n/a"}</span>
          {event.wsids && event.wsids.length > 0 && <span className="chip">WSIDs: {event.wsids.join(", ")}</span>}
        </div>
      </div>
      <div className="drawer-grid">
        <div>
          <h4>Metadata</h4>
          <ul className="meta-list">
            <li>Event ID: {event.event_id}</li>
            <li>Timestamp: {new Date(event.timestamp).toLocaleString()}</li>
            <li>Actor ID: {event.actor_id}</li>
            <li>Workflow ID: {event.workflow_id ?? "n/a"}</li>
          </ul>
        </div>
        <div>
          <h4>Payload</h4>
          <details className="drawer-raw">
            <summary>Raw JSON (redacted view)</summary>
            <div className="drawer-actions">
              <button className="secondary" onClick={() => setShowFullJson((v) => !v)}>
                {showFullJson ? "Hide full JSON" : "Show full JSON"}
              </button>
            </div>
            <pre>{JSON.stringify(showFullJson ? event.payload : redactJsonValue(event.payload), null, 2)}</pre>
          </details>
        </div>
      </div>
      <div className="drawer-actions">
        <button className="secondary" onClick={onClose}>
          Close
        </button>
        <button className="secondary" onClick={onCopyAsCoderPrompt}>
          Copy as coder prompt
        </button>
        <button className="secondary" onClick={onOpenJob}>
          Open Job
        </button>
        <button className="secondary" onClick={onOpenTimeline}>
          Open Timeline
        </button>
        <button
          className="primary"
          onClick={() => onExport && onExport(selection)}
          disabled={!onExport}
        >
          Export Debug Bundle
        </button>
      </div>
    </aside>
  );
};

export const EvidenceDrawer: React.FC<Props> = (props) => {
  const { selection, ...rest } = props;
  if (!selection) return null;

  const selectionKey =
    selection.kind === "diagnostic" ? `diagnostic:${selection.diagnostic.id}` : `event:${selection.event.event_id}`;

  return <EvidenceDrawerInner key={selectionKey} selection={selection} {...rest} />;
};
