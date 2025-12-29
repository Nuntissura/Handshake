import React from "react";

type Props = {
  percent: number;
  steps: { label: string; done: boolean; active?: boolean }[];
  onCancel?: () => void;
};

export const DebugBundleProgress: React.FC<Props> = ({ percent, steps, onCancel }) => {
  return (
    <div className="content-card">
      <div className="card-header">
        <h3>Exporting Debug Bundle...</h3>
        <p className="muted">We are collecting evidence, applying redaction, and creating a deterministic ZIP.</p>
      </div>
      <div className="progress-bar">
        <div className="progress-bar__fill" style={{ width: `${percent}%` }} />
      </div>
      <ul className="progress-steps">
        {steps.map((step) => (
          <li key={step.label} className={step.done ? "done" : step.active ? "active" : ""}>
            {step.done ? "✔" : step.active ? "…" : "○"} {step.label}
          </li>
        ))}
      </ul>
      <div className="drawer-actions">
        <button className="secondary" onClick={onCancel} disabled={!onCancel}>
          Cancel
        </button>
      </div>
    </div>
  );
};
