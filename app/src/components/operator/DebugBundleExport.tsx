import React, { useEffect, useMemo, useState } from "react";
import { openPath } from "@tauri-apps/plugin-opener";
import {
  BundleExportRequest,
  BundleExportResponse,
  BundleScopeInput,
  BundleStatus,
  exportDebugBundle,
  getBundleStatus,
} from "../../lib/api";
import { DebugBundleProgress } from "./DebugBundleProgress";
import { DebugBundleComplete } from "./DebugBundleComplete";

type Stage = "form" | "progress" | "complete";

type Props = {
  isOpen: boolean;
  defaultScope?: BundleScopeInput;
  onClose: () => void;
};

type InnerProps = {
  defaultScope?: BundleScopeInput;
  onClose: () => void;
};

function scopeKeyForScope(scope?: BundleScopeInput): string {
  if (!scope) return "default";
  switch (scope.kind) {
    case "job":
      return `job:${scope.job_id}`;
    case "problem":
      return `problem:${scope.problem_id}`;
    case "workspace":
      return `workspace:${scope.wsid}`;
    case "time_window":
      return `time_window:${scope.time_range?.start ?? ""}:${scope.time_range?.end ?? ""}:${scope.wsid ?? ""}`;
    default:
      return "unknown";
  }
}

function toDateTimeLocal(iso?: string): string {
  if (!iso) return "";
  const date = new Date(iso);
  if (Number.isNaN(date.getTime())) return "";
  const pad = (value: number) => value.toString().padStart(2, "0");
  return `${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())}T${pad(date.getHours())}:${pad(date.getMinutes())}`;
}

function toIsoFromLocal(value: string): string {
  const date = new Date(value);
  return Number.isNaN(date.getTime()) ? value : date.toISOString();
}

function formatBytes(bytes: number): string {
  if (!Number.isFinite(bytes) || bytes <= 0) return "n/a";
  const kb = 1024;
  const mb = kb * 1024;
  if (bytes >= mb) return `${(bytes / mb).toFixed(1)} MB`;
  if (bytes >= kb) return `${(bytes / kb).toFixed(1)} KB`;
  return `${bytes} B`;
}

function buildProgressSteps(percent: number) {
  const thresholds = [
    { label: "Collecting job metadata", doneAt: 20 },
    { label: "Collecting diagnostics", doneAt: 35 },
    { label: "Extracting Flight Recorder events...", doneAt: 50 },
    { label: "Applying redaction", doneAt: 65 },
    { label: "Generating coder prompt", doneAt: 80 },
    { label: "Creating ZIP", doneAt: 95 },
  ];

  const doneFlags = thresholds.map((step) => percent >= step.doneAt);
  const activeIndex = doneFlags.findIndex((done) => !done);

  return thresholds.map((step, idx) => ({
    label: step.label,
    done: doneFlags[idx],
    active: activeIndex === idx,
  }));
}

const DebugBundleExportInner: React.FC<InnerProps> = ({ defaultScope, onClose }) => {
  const [redactionMode, setRedactionMode] = useState<BundleExportRequest["redaction_mode"]>("SAFE_DEFAULT");
  const [scope, setScope] = useState<BundleScopeInput>(() => defaultScope ?? { kind: "job", job_id: "" });
  const [stage, setStage] = useState<Stage>("form");
  const [bundleId, setBundleId] = useState<string | null>(null);
  const [bundleStatus, setBundleStatus] = useState<BundleStatus | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [progress, setProgress] = useState(0);

  useEffect(() => {
    if (!bundleId || stage !== "progress") return;
    let active = true;
    const interval = setInterval(() => {
      getBundleStatus(bundleId)
        .then((status) => {
          if (!active) return;
          setBundleStatus(status);
          if (status.status === "ready") {
            setProgress(100);
            setStage("complete");
          } else if (status.status === "expired") {
            setError("Bundle expired before it could be downloaded.");
            setStage("form");
          } else if (status.status === "failed") {
            setError(status.error ?? "Export failed");
            setStage("form");
          } else {
            setProgress((current) => Math.min(95, current + 5));
          }
        })
        .catch(() => {
          if (active) setError("Failed to poll bundle status");
        });
    }, 1500);

    return () => {
      active = false;
      clearInterval(interval);
    };
  }, [bundleId, stage]);

  const scopeSummary = useMemo(() => {
    switch (scope.kind) {
      case "problem":
        return `Problem: ${scope.problem_id}`;
      case "job":
        return `Job: ${scope.job_id}`;
      case "workspace":
        return `Workspace: ${scope.wsid}`;
      case "time_window":
        return `Time Window: ${scope.time_range?.start ?? ""} to ${scope.time_range?.end ?? ""}`;
      default:
        return "Scope not set";
    }
  }, [scope]);

  const bundlePath = bundleId ? `data/bundles/bundle-${bundleId}` : null;
  const manifestValue = bundleStatus?.manifest as Record<string, unknown> | null | undefined;
  const manifestFiles = Array.isArray(manifestValue?.files) ? (manifestValue?.files as Record<string, unknown>[]) : null;
  const fileCount = manifestFiles ? manifestFiles.length + 1 : 9;
  const sizeBytes = manifestFiles
    ? manifestFiles.reduce((sum, entry) => sum + (typeof entry.size_bytes === "number" ? entry.size_bytes : 0), 0)
    : 0;
  const sizeLabel = sizeBytes > 0 ? formatBytes(sizeBytes) : "n/a";
  const expiresAt = bundleStatus?.expires_at ?? null;

  const handleCopyPath = async () => {
    if (!bundlePath) return;
    try {
      await navigator.clipboard.writeText(bundlePath);
    } catch {
      setError("Failed to copy bundle path to clipboard.");
    }
  };

  const handleOpenFolder = async () => {
    if (!bundlePath) return;
    try {
      await openPath(bundlePath);
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to open bundle folder.");
    }
  };

  const startExport = async () => {
    const scopeError = (() => {
      switch (scope.kind) {
        case "job":
          return scope.job_id.trim().length === 0 ? "job_id is required for job scope" : null;
        case "problem":
          return scope.problem_id.trim().length === 0 ? "problem_id is required for problem scope" : null;
        case "workspace":
          return scope.wsid.trim().length === 0 ? "wsid is required for workspace scope" : null;
        case "time_window":
          if (!scope.time_range?.start || !scope.time_range?.end) return "start and end are required for time_window scope";
          return null;
        default:
          return "scope.kind is required";
      }
    })();
    if (scopeError) {
      setError(scopeError);
      return;
    }

    const payload: BundleExportRequest = { scope, redaction_mode: redactionMode };
    setError(null);
    setBundleStatus(null);
    setStage("progress");
    setProgress(10);
    try {
      const response: BundleExportResponse = await exportDebugBundle(payload);
      setBundleId(response.export_job_id);
      setProgress(20);
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to start export");
      setStage("form");
    }
  };

  return (
    <div className="modal-overlay">
      <div className="modal">
        <div className="modal-header">
          <h3>Export Debug Bundle</h3>
          <button className="icon-button" onClick={onClose} aria-label="Close">
            X
          </button>
        </div>

        {stage === "form" && (
          <div className="modal-body">
            <p className="muted small">Scope and redaction controls align with SAFE_DEFAULT requirements.</p>

            <div className="stacked">
              <span>Scope</span>
              <select
                value={scope.kind}
                onChange={(e) => {
                  const kind = e.target.value as BundleScopeInput["kind"];
                  if (kind === "job") setScope({ kind: "job", job_id: "" });
                  else if (kind === "problem") setScope({ kind: "problem", problem_id: "" });
                  else if (kind === "workspace") setScope({ kind: "workspace", wsid: "" });
                  else {
                    const now = new Date().toISOString();
                    setScope({ kind: "time_window", time_range: { start: now, end: now } });
                  }
                }}
              >
                <option value="job">Job</option>
                <option value="problem">Problem</option>
                <option value="time_window">Time Window</option>
                <option value="workspace">Workspace</option>
              </select>
              <span className="muted small">{scopeSummary}</span>
            </div>

            {scope.kind === "job" && (
              <label className="stacked">
                <span>Job ID</span>
                <input
                  placeholder="job uuid"
                  value={scope.job_id}
                  onChange={(e) => setScope({ kind: "job", job_id: e.target.value })}
                />
              </label>
            )}

            {scope.kind === "problem" && (
              <label className="stacked">
                <span>Problem (Diagnostic) ID</span>
                <input
                  placeholder="diagnostic uuid"
                  value={scope.problem_id}
                  onChange={(e) => setScope({ kind: "problem", problem_id: e.target.value })}
                />
              </label>
            )}

            {scope.kind === "workspace" && (
              <label className="stacked">
                <span>Workspace ID</span>
                <input
                  placeholder="wsid"
                  value={scope.wsid}
                  onChange={(e) => setScope({ kind: "workspace", wsid: e.target.value })}
                />
              </label>
            )}

            {scope.kind === "time_window" && (
              <>
                <label className="stacked">
                  <span>Start</span>
                  <input
                    type="datetime-local"
                    value={toDateTimeLocal(scope.time_range.start)}
                    onChange={(e) =>
                      setScope({
                        ...scope,
                        time_range: { ...scope.time_range, start: toIsoFromLocal(e.target.value) },
                      })
                    }
                  />
                </label>
                <label className="stacked">
                  <span>End</span>
                  <input
                    type="datetime-local"
                    value={toDateTimeLocal(scope.time_range.end)}
                    onChange={(e) =>
                      setScope({
                        ...scope,
                        time_range: { ...scope.time_range, end: toIsoFromLocal(e.target.value) },
                      })
                    }
                  />
                </label>
                <label className="stacked">
                  <span>Workspace ID (optional)</span>
                  <input
                    placeholder="wsid"
                    value={scope.wsid ?? ""}
                    onChange={(e) => setScope({ ...scope, wsid: e.target.value || undefined })}
                  />
                </label>
              </>
            )}

            <fieldset>
              <legend>Redaction Mode</legend>
              <label className="radio">
                <input
                  type="radio"
                  name="redaction"
                  value="SAFE_DEFAULT"
                  checked={redactionMode === "SAFE_DEFAULT"}
                  onChange={() => setRedactionMode("SAFE_DEFAULT")}
                />
                <div>
                  <strong>Safe Default</strong>
                  <p className="muted small">Removes secrets, PII, and absolute paths. Recommended for sharing.</p>
                </div>
              </label>
              <label className="radio">
                <input
                  type="radio"
                  name="redaction"
                  value="WORKSPACE"
                  checked={redactionMode === "WORKSPACE"}
                  onChange={() => setRedactionMode("WORKSPACE")}
                />
                <div>
                  <strong>Workspace</strong>
                  <p className="muted small">Includes workspace context; secrets still redacted.</p>
                </div>
              </label>
              <label className="radio">
                <input
                  type="radio"
                  name="redaction"
                  value="FULL_LOCAL"
                  checked={redactionMode === "FULL_LOCAL"}
                  onChange={() => setRedactionMode("FULL_LOCAL")}
                />
                <div>
                  <strong>Full Local</strong>
                  <p className="muted small">Full payloads; use only with explicit policy.</p>
                </div>
              </label>
            </fieldset>

            {error && <p className="error">{error}</p>}
          </div>
        )}

        {stage === "progress" && (
          <div className="modal-body">
            <DebugBundleProgress
              percent={progress}
              steps={buildProgressSteps(progress)}
              onCancel={onClose}
            />
          </div>
        )}

        {stage === "complete" && bundleId && (
          <div className="modal-body">
            <DebugBundleComplete
              bundleId={bundleId}
              bundlePath={bundlePath ?? undefined}
              expiresAt={expiresAt ?? undefined}
              fileCount={fileCount}
              sizeLabel={sizeLabel}
              onCopyPath={bundlePath ? handleCopyPath : undefined}
              onOpenFolder={bundlePath ? handleOpenFolder : undefined}
              onDone={onClose}
            />
            {error && <p className="error">{error}</p>}
          </div>
        )}

        {stage === "form" && (
          <div className="modal-footer">
            <button className="secondary" onClick={onClose}>
              Cancel
            </button>
            <button className="primary" onClick={startExport}>
              Export
            </button>
          </div>
        )}
      </div>
    </div>
  );
};

export const DebugBundleExport: React.FC<Props> = ({ isOpen, defaultScope, onClose }) => {
  if (!isOpen) return null;
  return <DebugBundleExportInner key={scopeKeyForScope(defaultScope)} defaultScope={defaultScope} onClose={onClose} />;
};
