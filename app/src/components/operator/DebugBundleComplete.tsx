import React from "react";

type Props = {
  bundleId: string;
  onCopyPath?: () => void;
  onOpenFolder?: () => void;
  onDone?: () => void;
  redactionsApplied?: number;
  fileCount?: number;
  sizeLabel?: string;
};

export const DebugBundleComplete: React.FC<Props> = ({
  bundleId,
  onCopyPath,
  onOpenFolder,
  onDone,
  redactionsApplied = 0,
  fileCount = 9,
  sizeLabel = "n/a",
}) => {
  return (
    <div className="content-card">
      <div className="card-header">
        <h3>Debug Bundle Ready</h3>
        <p className="muted">Bundle ID: {bundleId}</p>
      </div>
      <ul className="meta-list">
        <li>Files: {fileCount}</li>
        <li>Redactions applied: {redactionsApplied}</li>
        <li>Estimated size: {sizeLabel}</li>
      </ul>
      <p className="muted small">The bundle is stored locally and can be downloaded or shared.</p>
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
