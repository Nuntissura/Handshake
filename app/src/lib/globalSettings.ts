// Global settings persistence helpers.
//
// This module owns the localStorage keys + load/save logic for the genuinely
// wired global settings so that both the Settings menu (writer) and App.tsx
// (reader, e.g. for the Swarm Board disclosure's defaultOpen) share one source
// of truth and cannot drift.
//
// Settings that are NOT yet backed by real state (reconcile interval, resource
// poll interval, theme) intentionally live here only as constant descriptors so
// the menu can render them honestly as "not yet wired" without faking a setter.

export const SWARM_BOARD_DEFAULT_OPEN_STORAGE_KEY = "handshake.swarm.board_default_open";

/** Default for "Open Swarm Board on launch" — FALSE to honor collapsed-by-default. */
export const SWARM_BOARD_DEFAULT_OPEN_FALLBACK = false;

function loadBoolean(key: string, fallback: boolean): boolean {
  try {
    const raw = localStorage.getItem(key);
    if (raw === null) return fallback;
    return raw === "true";
  } catch {
    return fallback;
  }
}

function saveBoolean(key: string, value: boolean): void {
  try {
    localStorage.setItem(key, value ? "true" : "false");
  } catch {
    // Best-effort persistence only.
  }
}

/** Read whether the Swarm Board disclosure should default to open on launch. */
export function loadSwarmBoardDefaultOpen(): boolean {
  return loadBoolean(SWARM_BOARD_DEFAULT_OPEN_STORAGE_KEY, SWARM_BOARD_DEFAULT_OPEN_FALLBACK);
}

/** Persist whether the Swarm Board disclosure should default to open on launch. */
export function saveSwarmBoardDefaultOpen(value: boolean): void {
  saveBoolean(SWARM_BOARD_DEFAULT_OPEN_STORAGE_KEY, value);
}

/**
 * Descriptor for a setting that exists in the UI but has no backing setter yet.
 * Rendered with a clear "not yet wired" affordance so nothing looks falsely
 * functional. `fixedValueLabel` is the value the control is currently pinned to.
 */
export type NotYetWiredSetting = {
  readonly id: string;
  readonly label: string;
  readonly fixedValueLabel: string;
  readonly note: string;
};

/** Swarm board auto-reconcile cadence — hardcoded 10s const in SwarmBoard.useSwarmBoard. */
export const SWARM_RECONCILE_INTERVAL_SETTING: NotYetWiredSetting = {
  id: "swarm-reconcile-interval",
  label: "Swarm board auto-reconcile interval",
  fixedValueLabel: "10s",
  note: "Not yet wired — fixed at 10s",
};

/** Swarm resource poll cadence — hardcoded POLL_INTERVAL_MS=1500 const in SwarmControlRoom. */
export const SWARM_RESOURCE_POLL_INTERVAL_SETTING: NotYetWiredSetting = {
  id: "swarm-resource-poll-interval",
  label: "Swarm resource poll interval",
  fixedValueLabel: "1.5s",
  note: "Not yet wired — fixed at 1.5s",
};

/** Theme — App.css ships a single light :root token set; no dark/color-scheme switch exists. */
export const THEME_SETTING: NotYetWiredSetting = {
  id: "theme",
  label: "Theme / appearance",
  fixedValueLabel: "Light (only theme available)",
  note: "Not yet wired — light is the only theme",
};

// --- Terminal (WP-KERNEL-004) -------------------------------------------------
// These three settings describe the integrated terminal panel but are not yet
// wired to a persisted setter / backend config, so they render as honest
// "not yet wired" rows. When the backend TerminalRuntime exposes config
// (default shell, scrollback cap, output-logging policy), promote each to a real
// load/save pair like the swarm-board-default-open setting above.

/** Default shell launched for new HumanDev sessions — backend-config-owned. */
export const TERMINAL_DEFAULT_SHELL_SETTING: NotYetWiredSetting = {
  id: "terminal-default-shell",
  label: "Terminal default shell",
  fixedValueLabel: "System default (backend-chosen)",
  note: "Not yet wired — backend picks the shell",
};

/** Max xterm scrollback lines (flood bound). Fixed in TerminalView for now. */
export const TERMINAL_MAX_SCROLLBACK_SETTING: NotYetWiredSetting = {
  id: "terminal-max-scrollback",
  label: "Terminal max scrollback",
  fixedValueLabel: "5000 lines",
  note: "Not yet wired — fixed at 5000 lines",
};

/** Output-logging policy (Flight Recorder capture of terminal output). */
export const TERMINAL_OUTPUT_LOGGING_SETTING: NotYetWiredSetting = {
  id: "terminal-output-logging",
  label: "Terminal output logging policy",
  fixedValueLabel: "Capture to Flight Recorder (redacted)",
  note: "Not yet wired — backend redacts + records captured output",
};

/**
 * About / build info. No build/version string is surfaced anywhere in the app
 * recon, so we honestly report "n/a" rather than inventing one.
 */
export const ABOUT_INFO = {
  appName: "Handshake",
  version: "n/a",
} as const;
