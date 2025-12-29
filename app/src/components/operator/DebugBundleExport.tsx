import React, { useEffect, useMemo, useState } from "react";
import { BundleExportRequest, BundleExportResponse, BundleScopeInput, downloadBundle, exportDebugBundle, getBundleStatus } from "../../lib/api";
import { DebugBundleProgress } from "./DebugBundleProgress";
import { DebugBundleComplete } from "./DebugBundleComplete";

type Stage = "form" | "progress" | "complete";

type Props = {
  isOpen: boolean;
  defaultScope?: BundleScopeInput;
  onClose: () => void;
};

export const DebugBundleExport: React.FC<Props> = ({ isOpen, defaultScope, onClose }) => {
  const [redactionMode, setRedactionMode] = useState<BundleExportRequest["redaction_mode"]>("SAFE_DEFAULT");
  const [scope, setScope] = useState<BundleScopeInput>(defaultScope ?? { kind: "job", job_id: "" });
  const [stage, setStage] = useState<Stage>("form");
  const [bundleId, setBundleId] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [progress, setProgress] = useState(15);
  useEffect(() => {
    if (!bundleId || stage !== "progress") return;
    let active = true;
    const interval = setInterval(() => {
      getBundleStatus(bundleId)
        .then((status) => {
          if (!active) return;
          if (status.status === "ready") {
            setProgress(100);
            setStage("complete");
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
        return `Time Window: ${scope.time_range?.start ?? ""} → ${scope.time_range?.end ?? ""}`;
      default:
        return "Scope not set";
    }
  }, [scope]);

  if (!isOpen) return null;

  const startExport = async () => {
    const payload: BundleExportRequest = { scope, redaction_mode: redactionMode };
    setError(null);
    setStage("progress");
    try {
      const response: BundleExportResponse = await exportDebugBundle(payload);
      setBundleId(response.export_job_id);
      setProgress(35);
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to start export");
      setStage("form");
    }
  };

  const handleDownload = async () => {
    if (!bundleId) return;
    const blob = await downloadBundle(bundleId);
    const url = window.URL.createObjectURL(blob);
    const link = document.createElement("a");
    link.href = url;
    link.download = `${bundleId}.zip`;
    link.click();
    window.URL.revokeObjectURL(url);
  };

  return (
    <div className="modal-overlay">
      <div className="modal">
        <div className="modal-header">
          <h3>Export Debug Bundle</h3>
          <button className="icon-button" onClick={onClose} aria-label="Close">
            ×
          </button>
        </div>

        {stage === "form" && (
          <div className="modal-body">
            <p className="muted small">Scope and redaction controls align with SAFE_DEFAULT requirements.</p>

            <label className="stacked">
              <span>Scope</span>
              <input
                placeholder="job id"
                value={scope.kind === "job" ? scope.job_id ?? "" : ""}
                onChange={(e) => setScope({ kind: "job", job_id: e.target.value })}
              />
              <span className="muted small">{scopeSummary}</span>
            </label>

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
              steps={[
                { label: "Collecting metadata", done: progress > 25, active: progress <= 25 },
                { label: "Applying redaction", done: progress > 55, active: progress > 25 && progress <= 55 },
                { label: "Creating ZIP", done: progress > 85, active: progress > 55 && progress <= 85 },
              ]}
              onCancel={onClose}
            />
          </div>
        )}

        {stage === "complete" && bundleId && (
          <div className="modal-body">
            <DebugBundleComplete
              bundleId={bundleId}
              fileCount={9}
              sizeLabel="~n/a"
              onCopyPath={handleDownload}
              onOpenFolder={handleDownload}
              onDone={onClose}
            />
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
