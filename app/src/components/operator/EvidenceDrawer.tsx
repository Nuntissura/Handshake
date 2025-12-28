import React from "react";
import { Diagnostic, FlightEvent } from "../../lib/api";

export type EvidenceSelection =
  | { kind: "diagnostic"; diagnostic: Diagnostic }
  | { kind: "event"; event: FlightEvent };

type Props = {
  selection: EvidenceSelection | null;
  onClose: () => void;
  onExport?: (selection: EvidenceSelection) => void;
};

function redactMessage(message: string, visibleChars = 180): string {
  if (message.length <= visibleChars) return message;
  return `${message.slice(0, visibleChars)}... [redacted preview]`;
}

export const EvidenceDrawer: React.FC<Props> = ({ selection, onClose, onExport }) => {
  if (!selection) return null;

  const renderDiagnostic = (diagnostic: Diagnostic) => {
    return (
      <>
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
          <summary>Raw JSON (redacted by default)</summary>
          <pre>{JSON.stringify(diagnostic, null, 2)}</pre>
        </details>
        <div className="drawer-actions">
          <button className="secondary" onClick={onClose}>
            Close
          </button>
          <button
            className="primary"
            onClick={() => onExport && onExport(selection)}
            disabled={!onExport}
          >
            Export Debug Bundle
          </button>
        </div>
      </>
    );
  };

  const renderEvent = (event: FlightEvent) => {
    return (
      <>
        <div className="drawer-header">
          <div>
            <p className="drawer-eyebrow">Flight Recorder Event</p>
            <h3>{event.event_type}</h3>
            <p className="muted">Actor: {event.actor} · Trace: {event.trace_id}</p>
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
              <summary>View payload</summary>
              <pre>{JSON.stringify(event.payload, null, 2)}</pre>
            </details>
          </div>
        </div>
        <div className="drawer-actions">
          <button className="secondary" onClick={onClose}>
            Close
          </button>
          <button
            className="primary"
            onClick={() => onExport && onExport(selection)}
            disabled={!onExport}
          >
            Export Debug Bundle
          </button>
        </div>
      </>
    );
  };

  return (
    <aside className="evidence-drawer">
      {selection.kind === "diagnostic"
        ? renderDiagnostic(selection.diagnostic)
        : renderEvent(selection.event)}
    </aside>
  );
};
