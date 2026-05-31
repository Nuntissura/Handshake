import { useEffect, useState } from "react";
import { ViewModeToggle } from "./ViewModeToggle";
import type { ViewMode } from "../lib/viewMode";
import {
  ABOUT_INFO,
  SWARM_RECONCILE_INTERVAL_SETTING,
  SWARM_RESOURCE_POLL_INTERVAL_SETTING,
  TERMINAL_DEFAULT_SHELL_SETTING,
  TERMINAL_MAX_SCROLLBACK_SETTING,
  TERMINAL_OUTPUT_LOGGING_SETTING,
  THEME_SETTING,
  loadSwarmBoardDefaultOpen,
  saveSwarmBoardDefaultOpen,
  type NotYetWiredSetting,
} from "../lib/globalSettings";

type SettingsMenuProps = {
  isOpen: boolean;
  onClose: () => void;
  viewMode: ViewMode;
  onViewModeChange: (mode: ViewMode) => void;
  /**
   * Optional: called when the operator flips "Open Swarm Board on launch".
   * App.tsx owns the board disclosure's defaultOpen, so it may pass a handler to
   * react live. When omitted, the setting is still persisted to localStorage and
   * takes effect on next launch.
   */
  onSwarmBoardDefaultOpenChange?: (value: boolean) => void;
  /**
   * Optional: resets panes & drawers to defaults. App.tsx owns layout state, so
   * it provides this. When omitted the control renders disabled with a
   * "not yet wired" note rather than a dead no-op button.
   */
  onResetLayout?: () => void;
};

function NotYetWiredRow({ setting }: { setting: NotYetWiredSetting }) {
  return (
    <div className="settings-row" data-stable-id={`setting-${setting.id}`}>
      <div className="settings-row__main">
        <span className="settings-row__label">{setting.label}</span>
        <span className="settings-note">{setting.note}</span>
      </div>
      <input
        type="text"
        className="settings-row__control"
        value={setting.fixedValueLabel}
        readOnly
        disabled
        aria-label={`${setting.label} (not yet wired)`}
        data-stable-id={`setting-${setting.id}.control`}
      />
    </div>
  );
}

export function SettingsMenu({
  isOpen,
  onClose,
  viewMode,
  onViewModeChange,
  onSwarmBoardDefaultOpenChange,
  onResetLayout,
}: SettingsMenuProps) {
  // Seeded once from localStorage at mount. This menu is the only writer of the
  // board-default key, so the local state stays authoritative without a re-sync
  // effect on open.
  const [boardDefaultOpen, setBoardDefaultOpen] = useState<boolean>(() => loadSwarmBoardDefaultOpen());

  // Esc-to-close while the dialog is open.
  useEffect(() => {
    if (!isOpen) return;
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        event.stopPropagation();
        onClose();
      }
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [isOpen, onClose]);

  if (!isOpen) {
    return null;
  }

  const handleBoardDefaultOpenChange = (next: boolean) => {
    setBoardDefaultOpen(next);
    saveSwarmBoardDefaultOpen(next);
    onSwarmBoardDefaultOpenChange?.(next);
  };

  return (
    <>
      <div
        className="settings-menu__backdrop"
        onClick={onClose}
        data-stable-id="settings-menu.backdrop"
        aria-hidden="true"
      />
      <div
        className="ans001-drawer settings-menu"
        role="dialog"
        aria-modal="true"
        aria-label="Settings"
        data-stable-id="settings-menu"
        data-testid="settings-menu"
      >
        <div className="drawer-header">
          <div>
            <p className="drawer-eyebrow">Global</p>
            <h3 style={{ margin: 0 }}>Settings</h3>
          </div>
          <div className="drawer-actions" style={{ marginTop: 0 }}>
            <button
              type="button"
              className="secondary"
              onClick={onClose}
              data-stable-id="settings-menu.close"
              data-testid="settings-menu.close"
            >
              Close
            </button>
          </div>
        </div>

        {/* Appearance ------------------------------------------------------- */}
        <section className="settings-section" data-stable-id="settings-section-appearance">
          <h4 className="settings-section__title">Appearance</h4>

          <div className="settings-row" data-stable-id="setting-view-mode">
            <div className="settings-row__main">
              <span className="settings-row__label">View Mode</span>
              <span className="muted small">NSFW / SFW content visibility (persisted).</span>
            </div>
            <ViewModeToggle value={viewMode} onChange={onViewModeChange} />
          </div>

          <NotYetWiredRow setting={THEME_SETTING} />
        </section>

        {/* Swarm ------------------------------------------------------------ */}
        <section className="settings-section" data-stable-id="settings-section-swarm">
          <h4 className="settings-section__title">Swarm</h4>

          <NotYetWiredRow setting={SWARM_RECONCILE_INTERVAL_SETTING} />
          <NotYetWiredRow setting={SWARM_RESOURCE_POLL_INTERVAL_SETTING} />

          <div className="settings-row" data-stable-id="setting-swarm-board-default-open">
            <div className="settings-row__main">
              <span className="settings-row__label">Open Swarm Board on launch</span>
              <span className="muted small">
                Persisted. Board stays collapsed by default; enable to open it at startup.
              </span>
            </div>
            <label className="settings-row__control settings-checkbox">
              <input
                type="checkbox"
                checked={boardDefaultOpen}
                onChange={(event) => handleBoardDefaultOpenChange(event.target.checked)}
                data-stable-id="setting-swarm-board-default-open.control"
                data-testid="setting-swarm-board-default-open.control"
              />
              <span>{boardDefaultOpen ? "Open" : "Collapsed"}</span>
            </label>
          </div>
        </section>

        {/* Terminal --------------------------------------------------------- */}
        <section className="settings-section" data-stable-id="settings-section-terminal">
          <h4 className="settings-section__title">Terminal</h4>

          <NotYetWiredRow setting={TERMINAL_DEFAULT_SHELL_SETTING} />
          <NotYetWiredRow setting={TERMINAL_MAX_SCROLLBACK_SETTING} />
          <NotYetWiredRow setting={TERMINAL_OUTPUT_LOGGING_SETTING} />
        </section>

        {/* Layout ----------------------------------------------------------- */}
        <section className="settings-section" data-stable-id="settings-section-layout">
          <h4 className="settings-section__title">Layout</h4>

          <div className="settings-row" data-stable-id="setting-reset-layout">
            <div className="settings-row__main">
              <span className="settings-row__label">Reset layout</span>
              {onResetLayout ? (
                <span className="muted small">Restore panes &amp; drawers to their defaults.</span>
              ) : (
                <span className="settings-note">Not yet wired</span>
              )}
            </div>
            <button
              type="button"
              className="secondary settings-row__control"
              onClick={onResetLayout}
              disabled={!onResetLayout}
              data-stable-id="setting-reset-layout.control"
              data-testid="setting-reset-layout.control"
            >
              Reset panes &amp; drawers
            </button>
          </div>
        </section>

        {/* About ------------------------------------------------------------ */}
        <section className="settings-section" data-stable-id="settings-section-about">
          <h4 className="settings-section__title">About</h4>
          <div className="settings-about">
            <div>
              <span className="muted small">App</span> {ABOUT_INFO.appName}
            </div>
            <div>
              <span className="muted small">Version</span> {ABOUT_INFO.version}
            </div>
          </div>
        </section>
      </div>
    </>
  );
}
