import { useEffect, useMemo, useState } from "react";
import { ViewModeToggle } from "./ViewModeToggle";
import { CliBridgeConfigPanel } from "./CliBridgeConfigPanel";
import type { ViewMode } from "../lib/viewMode";
import type { CliBridgeConfigIpc } from "../lib/ipc/cli_bridge_config";
import {
  APP_KEYBINDING_ACTIONS,
  defaultWorkspaceSettingsState,
  findKeybindingConflicts,
  keybindingLabelForConflict,
  normalizeChordInput,
  normalizeWorkspaceSettingsState,
  type AppKeybindingActionId,
  type WorkspaceSettingsState,
  type WorkspaceTheme,
} from "../lib/workspaceSettings";
import {
  ABOUT_INFO,
  SWARM_RECONCILE_INTERVAL_SETTING,
  SWARM_RESOURCE_POLL_INTERVAL_SETTING,
  TERMINAL_DEFAULT_SHELL_SETTING,
  TERMINAL_MAX_SCROLLBACK_SETTING,
  TERMINAL_OUTPUT_LOGGING_SETTING,
  loadSwarmBoardDefaultOpen,
  saveSwarmBoardDefaultOpen,
  type NotYetWiredSetting,
} from "../lib/globalSettings";

type SettingsMenuProps = {
  isOpen: boolean;
  onClose: () => void;
  viewMode: ViewMode;
  onViewModeChange: (mode: ViewMode) => void;
  workspaceSettings?: WorkspaceSettingsState;
  onWorkspaceSettingsChange?: (settings: WorkspaceSettingsState) => void;
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
  /**
   * Optional: injected IPC client for the embedded CLI-bridge config panel.
   * Production omits it (the panel uses the real Tauri-backed default). Tests
   * pass a controlled mock so the panel can mount without `@tauri-apps/api`'s
   * `invoke` rejecting under jsdom (which otherwise floods act() warnings).
   */
  cliBridgeIpc?: CliBridgeConfigIpc;
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

function settingMatchesQuery(query: string, terms: readonly string[]) {
  if (!query) {
    return true;
  }
  return terms.join(" ").toLowerCase().includes(query);
}

export function SettingsMenu({
  isOpen,
  onClose,
  viewMode,
  onViewModeChange,
  workspaceSettings,
  onWorkspaceSettingsChange,
  onSwarmBoardDefaultOpenChange,
  onResetLayout,
  cliBridgeIpc,
}: SettingsMenuProps) {
  // Seeded once from localStorage at mount. This menu is the only writer of the
  // board-default key, so the local state stays authoritative without a re-sync
  // effect on open.
  const [boardDefaultOpen, setBoardDefaultOpen] = useState<boolean>(() => loadSwarmBoardDefaultOpen());
  const [settingsQuery, setSettingsQuery] = useState("");
  const effectiveWorkspaceSettings = workspaceSettings ?? defaultWorkspaceSettingsState(viewMode, boardDefaultOpen);
  const [draftKeybindings, setDraftKeybindings] = useState(effectiveWorkspaceSettings.keybindings);
  const boardDefaultOpenValue = workspaceSettings?.settings.swarm_board_default_open ?? boardDefaultOpen;
  const query = settingsQuery.trim().toLowerCase();
  const workspaceSettingsConnected = Boolean(onWorkspaceSettingsChange);
  const keybindingSyncKey = JSON.stringify(effectiveWorkspaceSettings.keybindings);
  const keybindingConflicts = useMemo(
    () => findKeybindingConflicts(draftKeybindings),
    [draftKeybindings],
  );
  const hasKeybindingConflict = keybindingConflicts.length > 0;

  useEffect(() => {
    setDraftKeybindings(effectiveWorkspaceSettings.keybindings);
    // eslint-disable-next-line react-hooks/exhaustive-deps -- keybindingSyncKey intentionally narrows object identity churn.
  }, [keybindingSyncKey]);

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
    emitWorkspaceSettings({
      ...effectiveWorkspaceSettings,
      settings: {
        ...effectiveWorkspaceSettings.settings,
        swarm_board_default_open: next,
      },
    });
  };

  const emitWorkspaceSettings = (next: WorkspaceSettingsState) => {
    onWorkspaceSettingsChange?.(normalizeWorkspaceSettingsState(next, effectiveWorkspaceSettings));
  };

  const handleViewModeChange = (next: ViewMode) => {
    onViewModeChange(next);
    emitWorkspaceSettings({
      ...effectiveWorkspaceSettings,
      settings: {
        ...effectiveWorkspaceSettings.settings,
        view_mode: next,
      },
    });
  };

  const handleThemeChange = (nextTheme: WorkspaceTheme) => {
    emitWorkspaceSettings({
      ...effectiveWorkspaceSettings,
      theme: nextTheme,
    });
  };

  const handleKeybindingChange = (actionId: AppKeybindingActionId, value: string) => {
    const nextKeybindings = {
      ...draftKeybindings,
      [actionId]: value,
    };
    setDraftKeybindings(nextKeybindings);
    const normalizedKeybindings = {
      ...effectiveWorkspaceSettings.keybindings,
      [actionId]: normalizeChordInput(value),
    };
    if (findKeybindingConflicts(normalizedKeybindings).length > 0) {
      return;
    }
    emitWorkspaceSettings({
      ...effectiveWorkspaceSettings,
      keybindings: normalizedKeybindings,
    });
  };

  const handleKeybindingReset = (actionId: AppKeybindingActionId, defaultChord: string) => {
    const nextKeybindings = {
      ...draftKeybindings,
      [actionId]: defaultChord,
    };
    setDraftKeybindings(nextKeybindings);
    const normalizedKeybindings = {
      ...effectiveWorkspaceSettings.keybindings,
      [actionId]: defaultChord,
    };
    if (findKeybindingConflicts(normalizedKeybindings).length > 0) {
      return;
    }
    emitWorkspaceSettings({
      ...effectiveWorkspaceSettings,
      keybindings: normalizedKeybindings,
    });
  };

  const showAppearanceSection =
    settingMatchesQuery(query, ["appearance", "theme", "light", "dark", "view", "mode", "sfw", "nsfw"]);
  const showThemeRow = settingMatchesQuery(query, ["appearance", "theme", "light", "dark"]);
  const showViewModeRow = settingMatchesQuery(query, ["appearance", "view", "mode", "sfw", "nsfw"]);
  const visibleKeybindingActions = APP_KEYBINDING_ACTIONS.filter((action) =>
    settingMatchesQuery(query, ["keybinding", "shortcut", action.label, action.description, ...action.keywords]),
  );
  const showKeybindingsSection =
    visibleKeybindingActions.length > 0 ||
    settingMatchesQuery(query, ["keybinding", "keybindings", "shortcut", "shortcuts"]);
  const showSwarmSection = settingMatchesQuery(query, ["swarm", "board", "reconcile", "resource", "poll"]);
  const showCliBridgeSection = settingMatchesQuery(query, ["cli", "bridge", "official", "swarm", "lane"]);
  const showTerminalSection = settingMatchesQuery(query, ["terminal", "shell", "scrollback", "logging"]);
  const showLayoutSection = settingMatchesQuery(query, ["layout", "reset", "panes", "drawers"]);
  const showAboutSection = settingMatchesQuery(query, ["about", "app", "version"]);

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

        <label className="settings-search">
          <span className="muted small">Search settings</span>
          <input
            type="search"
            value={settingsQuery}
            onChange={(event) => setSettingsQuery(event.target.value)}
            placeholder="Theme, quick switcher, terminal..."
            data-testid="settings-menu.search"
          />
        </label>

        {/* Appearance ------------------------------------------------------- */}
        {showAppearanceSection ? (
        <section className="settings-section" data-stable-id="settings-section-appearance">
          <h4 className="settings-section__title">Appearance</h4>

          {showViewModeRow ? (
          <div className="settings-row" data-stable-id="setting-view-mode" data-testid="setting-view-mode">
            <div className="settings-row__main">
              <span className="settings-row__label">View Mode</span>
              <span className="muted small">NSFW / SFW content visibility (persisted).</span>
            </div>
            <ViewModeToggle value={viewMode} onChange={handleViewModeChange} />
          </div>
          ) : null}

          {showThemeRow ? (
          <div className="settings-row" data-stable-id="setting-theme" data-testid="setting-theme">
            <div className="settings-row__main">
              <span className="settings-row__label">Theme / appearance</span>
              <span className="muted small">Workspace-scoped light or dark shell theme.</span>
            </div>
            <select
              className="settings-row__control"
              aria-label="Theme / appearance"
              value={effectiveWorkspaceSettings.theme}
              onChange={(event) => handleThemeChange(event.target.value as WorkspaceTheme)}
              disabled={!workspaceSettingsConnected}
              data-stable-id="setting-theme.control"
              data-testid="setting-theme.control"
            >
              <option value="light">Light</option>
              <option value="dark">Dark</option>
            </select>
          </div>
          ) : null}
        </section>
        ) : null}

        {/* Keybindings ------------------------------------------------------ */}
        {showKeybindingsSection ? (
        <section className="settings-section" data-stable-id="settings-section-keybindings">
          <h4 className="settings-section__title">Keybindings</h4>
          {hasKeybindingConflict ? (
            <p className="settings-note" role="alert" data-testid="settings-keybinding-conflict">
              {keybindingConflicts
                .map(
                  (conflict) =>
                    `${keybindingLabelForConflict(conflict.actionLabels)} both use ${conflict.chord}.`,
                )
                .join(" ")}
            </p>
          ) : null}
          {visibleKeybindingActions.map((action) => (
            <div
              key={action.id}
              className="settings-row"
              data-stable-id={`setting-keybinding-${action.id}`}
              data-testid={`setting-keybinding-${action.id}`}
            >
              <div className="settings-row__main">
                <span className="settings-row__label">{action.label}</span>
                <span className="muted small">{action.description}</span>
              </div>
              <div className="settings-row__compound-control">
                <input
                  type="text"
                  className="settings-row__control"
                  value={draftKeybindings[action.id] ?? action.defaultChord}
                  onChange={(event) => handleKeybindingChange(action.id, event.target.value)}
                  disabled={!workspaceSettingsConnected}
                  aria-label={`${action.label} keybinding`}
                  data-testid={`setting-keybinding-${action.id}.control`}
                />
                <button
                  type="button"
                  className="secondary"
                  onClick={() => handleKeybindingReset(action.id, action.defaultChord)}
                  disabled={!workspaceSettingsConnected}
                  data-testid={`setting-keybinding-${action.id}.reset`}
                >
                  Reset
                </button>
              </div>
            </div>
          ))}
        </section>
        ) : null}

        {/* Swarm ------------------------------------------------------------ */}
        {showSwarmSection ? (
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
                checked={boardDefaultOpenValue}
                onChange={(event) => handleBoardDefaultOpenChange(event.target.checked)}
                data-stable-id="setting-swarm-board-default-open.control"
                data-testid="setting-swarm-board-default-open.control"
              />
              <span>{boardDefaultOpenValue ? "Open" : "Collapsed"}</span>
            </label>
          </div>
        </section>
        ) : null}

        {/* CLI Bridge ------------------------------------------------------- */}
        {showCliBridgeSection ? (
        <section
          className="settings-section"
          data-stable-id="settings-section-cli-bridge"
          data-testid="settings-section-cli-bridge"
        >
          <h4 className="settings-section__title">CLI Bridge (Official-CLI swarm lane)</h4>
          <CliBridgeConfigPanel ipc={cliBridgeIpc} />
        </section>
        ) : null}

        {/* Terminal --------------------------------------------------------- */}
        {showTerminalSection ? (
        <section className="settings-section" data-stable-id="settings-section-terminal">
          <h4 className="settings-section__title">Terminal</h4>

          <NotYetWiredRow setting={TERMINAL_DEFAULT_SHELL_SETTING} />
          <NotYetWiredRow setting={TERMINAL_MAX_SCROLLBACK_SETTING} />
          <NotYetWiredRow setting={TERMINAL_OUTPUT_LOGGING_SETTING} />
        </section>
        ) : null}

        {/* Layout ----------------------------------------------------------- */}
        {showLayoutSection ? (
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
        ) : null}

        {/* About ------------------------------------------------------------ */}
        {showAboutSection ? (
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
        ) : null}
      </div>
    </>
  );
}
