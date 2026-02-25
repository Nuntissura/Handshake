import type { ViewMode } from "../lib/viewMode";

type ViewModeToggleProps = {
  value: ViewMode;
  onChange: (value: ViewMode) => void;
};

export function ViewModeToggle({ value, onChange }: ViewModeToggleProps) {
  return (
    <div className="view-mode-toggle" role="group" aria-label="View mode">
      <span className="view-mode-toggle__label">View mode</span>
      <button
        type="button"
        className={`view-mode-toggle__button ${value === "NSFW" ? "active" : ""}`}
        aria-pressed={value === "NSFW"}
        onClick={() => onChange("NSFW")}
      >
        NSFW
      </button>
      <button
        type="button"
        className={`view-mode-toggle__button ${value === "SFW" ? "active" : ""}`}
        aria-pressed={value === "SFW"}
        onClick={() => onChange("SFW")}
      >
        SFW
      </button>
    </div>
  );
}

