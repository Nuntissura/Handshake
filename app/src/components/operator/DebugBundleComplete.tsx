import React from "react";

type Props = {
  bundleId: string;
  bundlePath?: string;
  expiresAt?: string;
  onCopyPath?: () => void;
  onOpenFolder?: () => void;
  onDone?: () => void;
  redactionsApplied?: number;
  fileCount?: number;
  sizeLabel?: string;
};

export const DebugBundleComplete: React.FC<Props> = ({
  bundleId,
  bundlePath,
  expiresAt,
  onCopyPath,
  onOpenFolder,
  onDone,
  redactionsApplied,
  fileCount = 9,
  sizeLabel = "n/a",
}) => {
  const redactionsLabel = typeof redactionsApplied === "number" ? redactionsApplied.toString() : "n/a";
  const expiresLabel = expiresAt ? new Date(expiresAt).toLocaleString() : null;

  return (
    <div className="content-card">
      <div className="card-header">
        <h3>Debug Bundle Ready</h3>
        <p className="muted">Bundle ID: {bundleId}</p>
      </div>
      <ul className="meta-list">
        <li>Files: {fileCount}</li>
        <li>Redactions applied: {redactionsLabel}</li>
        <li>Estimated size: {sizeLabel}</li>
        {expiresLabel && <li>Expires at: {expiresLabel}</li>}
      </ul>
      {bundlePath ? (
        <p className="muted small">Path: {bundlePath}</p>
      ) : (
        <p className="muted small">The bundle is stored locally.</p>
      )}
      <div className="drawer-actions">
        <button className="secondary" onClick={onCopyPath} disabled={!onCopyPath}>
          Copy Path
        </button>
        <button className="secondary" onClick={onOpenFolder} disabled={!onOpenFolder}>
          Open Folder
        </button>
        <button className="primary" onClick={onDone}>
          Done
        </button>
      </div>
    </div>
  );
};
