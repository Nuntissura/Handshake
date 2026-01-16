import React, { useEffect, useMemo, useState } from "react";
import { openPath } from "@tauri-apps/plugin-opener";

import {
  AiJob,
  exportGovernancePack,
  getJob,
  GovernancePackExportRequest,
  GovernancePackExportResponse,
  GovernancePackInvariants,
} from "../../lib/api";

type Stage = "form" | "progress" | "complete";

type Props = {
  isOpen: boolean;
  onClose: () => void;
};

function isJobTerminalState(state: string): boolean {
  return ["completed", "completed_with_issues", "failed", "cancelled", "poisoned"].includes(state);
}

export const GovernancePackExport: React.FC<Props> = ({ isOpen, onClose }) => {
  const [stage, setStage] = useState<Stage>("form");
  const [jobId, setJobId] = useState<string | null>(null);
  const [job, setJob] = useState<AiJob | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [overwrite, setOverwrite] = useState(false);
  const [exportDir, setExportDir] = useState("");

  const [projectCode, setProjectCode] = useState("");
  const [projectDisplayName, setProjectDisplayName] = useState("");
  const [projectPrefix, setProjectPrefix] = useState("");
  const [issuePrefix, setIssuePrefix] = useState("");
  const [languageLayoutProfileId, setLanguageLayoutProfileId] = useState("");
  const [frontendRootDir, setFrontendRootDir] = useState("app");
  const [frontendSrcDir, setFrontendSrcDir] = useState("app/src");
  const [backendRootDir, setBackendRootDir] = useState("src/backend");
  const [backendCrateName, setBackendCrateName] = useState("");

  useEffect(() => {
    if (issuePrefix.trim().length === 0 && projectCode.trim().length > 0) {
      setIssuePrefix(projectCode.trim());
    }
    if (projectPrefix.trim().length === 0 && projectCode.trim().length > 0) {
      setProjectPrefix(projectCode.trim());
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [projectCode]);

  useEffect(() => {
    if (!jobId || stage !== "progress") return;
    let active = true;
    const interval = setInterval(() => {
      getJob(jobId)
        .then((next) => {
          if (!active) return;
          setJob(next);
          if (isJobTerminalState(next.state)) {
            setStage("complete");
          }
        })
        .catch((err) => {
          if (!active) return;
          setError(err instanceof Error ? err.message : "Failed to poll job");
        });
    }, 1500);

    return () => {
      active = false;
      clearInterval(interval);
    };
  }, [jobId, stage]);

  const exportSummary = useMemo(() => {
    return {
      exportDir: exportDir.trim(),
      overwrite,
      projectCode: projectCode.trim(),
      projectDisplayName: projectDisplayName.trim(),
      languageLayoutProfileId: languageLayoutProfileId.trim(),
    };
  }, [exportDir, languageLayoutProfileId, overwrite, projectCode, projectDisplayName]);

  const handleOpenFolder = async () => {
    const path = exportDir.trim();
    if (path.length === 0) return;
    try {
      await openPath(path);
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to open folder");
    }
  };

  const handleCopyPath = async () => {
    const path = exportDir.trim();
    if (path.length === 0) return;
    try {
      await navigator.clipboard.writeText(path);
    } catch {
      setError("Failed to copy path to clipboard");
    }
  };

  const startExport = async () => {
    setError(null);

    if (exportDir.trim().length === 0) {
      setError("export directory is required (absolute path)");
      return;
    }

    const required = [
      { key: "project_code", value: projectCode },
      { key: "project_display_name", value: projectDisplayName },
      { key: "issue_prefix", value: issuePrefix },
      { key: "language_layout_profile_id", value: languageLayoutProfileId },
      { key: "frontend_root_dir", value: frontendRootDir },
      { key: "frontend_src_dir", value: frontendSrcDir },
      { key: "backend_root_dir", value: backendRootDir },
      { key: "backend_crate_name", value: backendCrateName },
    ];
    const missing = required.find((field) => field.value.trim().length === 0);
    if (missing) {
      setError(`${missing.key} is required`);
      return;
    }

    const invariants: GovernancePackInvariants = {
      project_code: projectCode.trim(),
      project_display_name: projectDisplayName.trim(),
      project_prefix: projectPrefix.trim().length > 0 ? projectPrefix.trim() : undefined,
      issue_prefix: issuePrefix.trim(),
      language_layout_profile_id: languageLayoutProfileId.trim(),
      frontend_root_dir: frontendRootDir.trim(),
      frontend_src_dir: frontendSrcDir.trim(),
      backend_root_dir: backendRootDir.trim(),
      backend_crate_name: backendCrateName.trim(),
    };

    const request: GovernancePackExportRequest = {
      export_target: { type: "local_file", path: exportDir.trim() },
      overwrite,
      invariants,
    };

    setStage("progress");
    setJob(null);
    setJobId(null);

    try {
      const response: GovernancePackExportResponse = await exportGovernancePack(request);
      setJobId(response.export_job_id);
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to start export");
      setStage("form");
    }
  };

  const jobOutputs = job?.job_outputs as Record<string, unknown> | null | undefined;
  const materializedPaths = Array.isArray(jobOutputs?.materialized_paths)
    ? (jobOutputs?.materialized_paths as string[])
    : null;

  if (!isOpen) return null;

  return (
    <div className="modal-overlay">
      <div className="modal">
        <div className="modal-header">
          <h3>Export Governance Pack</h3>
          <button className="icon-button" onClick={onClose} aria-label="Close">
            X
          </button>
        </div>

        {stage === "form" && (
          <div className="modal-body">
            <p className="muted small">
              Exports the Master Spec Governance Pack Template Volume into a directory you choose. Requires an absolute LocalFile
              path.
            </p>

            <label className="stacked">
              <span>Export directory (absolute)</span>
              <input placeholder="C:\\path\\to\\repo" value={exportDir} onChange={(e) => setExportDir(e.target.value)} />
            </label>

            <label className="stacked">
              <span>Overwrite existing non-empty dir</span>
              <input type="checkbox" checked={overwrite} onChange={(e) => setOverwrite(e.target.checked)} />
            </label>

            <div className="stacked">
              <h4>Project invariants</h4>
              <p className="muted small">
                Used to resolve all <code>{"{{TOKEN}}"}</code> placeholders in templates.
              </p>
            </div>

            <div className="grid-two">
              <label className="stacked">
                <span>PROJECT_CODE</span>
                <input value={projectCode} onChange={(e) => setProjectCode(e.target.value)} placeholder="e.g. COOK" />
              </label>
              <label className="stacked">
                <span>PROJECT_DISPLAY_NAME</span>
                <input
                  value={projectDisplayName}
                  onChange={(e) => setProjectDisplayName(e.target.value)}
                  placeholder="e.g. Cooking App"
                />
              </label>
              <label className="stacked">
                <span>PROJECT_PREFIX</span>
                <input value={projectPrefix} onChange={(e) => setProjectPrefix(e.target.value)} placeholder="defaults to PROJECT_CODE" />
              </label>
              <label className="stacked">
                <span>ISSUE_PREFIX</span>
                <input value={issuePrefix} onChange={(e) => setIssuePrefix(e.target.value)} placeholder="used for issue tags" />
              </label>
              <label className="stacked">
                <span>LANGUAGE_LAYOUT_PROFILE_ID</span>
                <input
                  value={languageLayoutProfileId}
                  onChange={(e) => setLanguageLayoutProfileId(e.target.value)}
                  placeholder="e.g. rust-react"
                />
              </label>
              <label className="stacked">
                <span>FRONTEND_ROOT_DIR</span>
                <input value={frontendRootDir} onChange={(e) => setFrontendRootDir(e.target.value)} placeholder="e.g. app" />
              </label>
              <label className="stacked">
                <span>FRONTEND_SRC_DIR</span>
                <input value={frontendSrcDir} onChange={(e) => setFrontendSrcDir(e.target.value)} placeholder="e.g. app/src" />
              </label>
              <label className="stacked">
                <span>BACKEND_ROOT_DIR</span>
                <input value={backendRootDir} onChange={(e) => setBackendRootDir(e.target.value)} placeholder="e.g. src/backend" />
              </label>
              <label className="stacked">
                <span>BACKEND_CRATE_NAME</span>
                <input
                  value={backendCrateName}
                  onChange={(e) => setBackendCrateName(e.target.value)}
                  placeholder="e.g. my_core"
                />
              </label>
            </div>

            {error && <p className="muted">Error: {error}</p>}

            <div className="modal-actions">
              <button type="button" className="button" onClick={startExport}>
                Start export
              </button>
              <button type="button" className="button secondary" onClick={onClose}>
                Cancel
              </button>
            </div>
          </div>
        )}

        {stage === "progress" && (
          <div className="modal-body">
            <p className="muted small">Export in progress. This runs as a job and emits a Flight Recorder ExportRecord event.</p>
            <div className="stacked">
              <span className="muted">Job ID</span>
              <strong>{jobId ?? "starting..."}</strong>
            </div>
            <div className="stacked">
              <span className="muted">State</span>
              <strong>{job?.state ?? "queued"}</strong>
            </div>
            {error && <p className="muted">Error: {error}</p>}
            <div className="modal-actions">
              <button type="button" className="button secondary" onClick={onClose}>
                Close
              </button>
            </div>
          </div>
        )}

        {stage === "complete" && (
          <div className="modal-body">
            <p className="muted small">Export finished. Open the target directory to inspect the generated files.</p>
            <div className="stacked">
              <span className="muted">Export directory</span>
              <strong>{exportSummary.exportDir}</strong>
            </div>
            <div className="stacked">
              <span className="muted">Job state</span>
              <strong>{job?.state ?? "unknown"}</strong>
              {job?.error_message && <p className="muted">Error: {job.error_message}</p>}
            </div>
            {materializedPaths && (
              <div className="stacked">
                <span className="muted">Materialized paths</span>
                <div className="debug-panel__logbox">
                  {materializedPaths.slice(0, 12).map((p) => (
                    <pre key={p} className="debug-panel__line">
                      {p}
                    </pre>
                  ))}
                  {materializedPaths.length > 12 && <p className="muted small">…and {materializedPaths.length - 12} more</p>}
                </div>
              </div>
            )}
            <div className="modal-actions">
              <button type="button" className="button" onClick={handleOpenFolder}>
                Open folder
              </button>
              <button type="button" className="button secondary" onClick={handleCopyPath}>
                Copy path
              </button>
              <button type="button" className="button secondary" onClick={onClose}>
                Close
              </button>
            </div>
          </div>
        )}
      </div>
    </div>
  );
};
