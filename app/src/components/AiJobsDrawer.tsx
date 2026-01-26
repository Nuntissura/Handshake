import { useEffect, useMemo, useState } from "react";
import { AiJob } from "../lib/api";
import { getSnapshot, removeJob, startPolling, subscribe } from "../state/aiJobs";
import { JobResultPanel } from "./JobResultPanel";

function jobKindLabel(kind: string): string {
  switch (kind) {
    case "doc_summarize":
      return "Summarize document";
    default:
      return kind;
  }
}

function statusTone(state: string | undefined): "success" | "error" | "neutral" {
  if (!state) return "neutral";
  switch (state) {
    case "completed":
      return "success";
    case "failed":
    case "poisoned":
      return "error";
    default:
      return "neutral";
  }
}

export function AiJobsDrawer() {
  const [open, setOpen] = useState(false);
  const [selectedJobId, setSelectedJobId] = useState<string | null>(null);
  const [snapshot, setSnapshot] = useState(getSnapshot());

  useEffect(() => {
    startPolling();
    return subscribe(setSnapshot);
  }, []);

  const activeCount = useMemo(() => {
    return snapshot.entries.filter((entry) => {
      const state = snapshot.jobsById[entry.jobId]?.state;
      return state === "queued" || state === "running";
    }).length;
  }, [snapshot.entries, snapshot.jobsById]);

  const selectedJob: AiJob | null = selectedJobId ? snapshot.jobsById[selectedJobId] ?? null : null;
  const selectedError = selectedJobId ? snapshot.errorsById[selectedJobId] ?? null : null;

  const badgeLabel =
    activeCount > 0 ? `${activeCount} active` : snapshot.entries.length > 0 ? `${snapshot.entries.length}` : null;

  if (!open) {
    return (
      <button
        type="button"
        className="ai-jobs-fab"
        onClick={() => setOpen(true)}
        aria-label="Open AI Jobs"
      >
        AI Jobs{badgeLabel ? ` (${badgeLabel})` : ""}
      </button>
    );
  }

  return (
    <aside className="ai-jobs-drawer" aria-label="AI Jobs">
      <div className="ai-jobs-drawer__header">
        <div>
          <p className="drawer-eyebrow">AI Jobs</p>
          <h3>Tracker</h3>
          <p className="muted small">Persists across documents and app reloads.</p>
        </div>
        <div className="ai-jobs-drawer__header-actions">
          <button
            type="button"
            className="secondary"
            onClick={() => {
              setSelectedJobId(null);
              setOpen(false);
            }}
          >
            Close
          </button>
        </div>
      </div>

      {snapshot.entries.length === 0 ? (
        <p className="muted">No tracked jobs yet.</p>
      ) : (
        <ul className="ai-jobs-drawer__list">
          {snapshot.entries.map((entry) => {
            const jobState = snapshot.jobsById[entry.jobId]?.state;
            const tone = statusTone(jobState);
            const isSelected = entry.jobId === selectedJobId;
            return (
              <li key={entry.jobId} className={isSelected ? "ai-jobs-drawer__item selected" : "ai-jobs-drawer__item"}>
                <button
                  type="button"
                  className="ai-jobs-drawer__item-main"
                  onClick={() => setSelectedJobId(entry.jobId)}
                >
                  <div className="ai-jobs-drawer__item-title">
                    <strong>{jobKindLabel(entry.jobKind)}</strong>
                    <span className={`status-pill ${tone}`}>{jobState ?? "loading"}</span>
                  </div>
                  <div className="ai-jobs-drawer__item-meta">
                    <span className="muted">
                      {entry.docTitle ? `${entry.docTitle} (${entry.docId})` : entry.docId}
                    </span>
                    <span className="muted small">{new Date(entry.createdAt).toLocaleString()}</span>
                  </div>
                </button>
                <button type="button" className="ai-jobs-drawer__item-remove" onClick={() => removeJob(entry.jobId)}>
                  Remove
                </button>
              </li>
            );
          })}
        </ul>
      )}

      {selectedJobId && (
        <div className="ai-jobs-drawer__detail">
          <JobResultPanel
            job={selectedJob}
            loading={!selectedJob && !selectedError}
            error={selectedError}
            onDismiss={() => setSelectedJobId(null)}
          />
        </div>
      )}
    </aside>
  );
}

