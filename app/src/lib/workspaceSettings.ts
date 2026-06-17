import type { ViewMode } from "./viewMode";

export const WORKSPACE_SETTINGS_SCHEMA_ID = "hsk.workspace_settings_state@1";

export type WorkspaceTheme = "light" | "dark";

export type AppKeybindingActionId = "app.quick_switcher.open" | "app.command_palette.open";

export type WorkspaceSettingsState = {
  schema_id: typeof WORKSPACE_SETTINGS_SCHEMA_ID;
  theme: WorkspaceTheme;
  custom_theme_tokens: Record<string, string>;
  keybindings: Record<AppKeybindingActionId, string>;
  settings: {
    view_mode: ViewMode;
    swarm_board_default_open: boolean;
  };
};

export type AppKeybindingDescriptor = {
  id: AppKeybindingActionId;
  label: string;
  description: string;
  defaultChord: string;
  keywords: readonly string[];
};

export const APP_KEYBINDING_ACTIONS: readonly AppKeybindingDescriptor[] = [
  {
    id: "app.quick_switcher.open",
    label: "Quick Switcher",
    description: "Open the workspace-wide quick switcher.",
    defaultChord: "Mod-p",
    keywords: ["quick", "switcher", "open", "workspace", "search"],
  },
  {
    id: "app.command_palette.open",
    label: "Command Palette",
    description: "Open app-level commands.",
    defaultChord: "Mod-Shift-p",
    keywords: ["command", "palette", "commands", "open"],
  },
] as const;

const APP_KEYBINDING_DEFAULTS = Object.fromEntries(
  APP_KEYBINDING_ACTIONS.map((action) => [action.id, action.defaultChord]),
) as Record<AppKeybindingActionId, string>;

export function defaultWorkspaceSettingsState(
  viewMode: ViewMode = "NSFW",
  swarmBoardDefaultOpen = false,
): WorkspaceSettingsState {
  return {
    schema_id: WORKSPACE_SETTINGS_SCHEMA_ID,
    theme: "light",
    custom_theme_tokens: {},
    keybindings: { ...APP_KEYBINDING_DEFAULTS },
    settings: {
      view_mode: viewMode,
      swarm_board_default_open: swarmBoardDefaultOpen,
    },
  };
}

export function normalizeChordInput(chord: string): string {
  const parts = chord
    .split("-")
    .map((part) => part.trim())
    .filter(Boolean);
  if (parts.length === 0) {
    return "";
  }

  const key = parts[parts.length - 1];
  const modifiers = new Set<string>();
  for (const part of parts.slice(0, -1)) {
    const normalized = part.toLowerCase();
    if (["mod", "cmd", "command", "meta", "ctrl", "control"].includes(normalized)) {
      modifiers.add("Mod");
    } else if (normalized === "shift") {
      modifiers.add("Shift");
    } else if (["alt", "option"].includes(normalized)) {
      modifiers.add("Alt");
    }
  }

  const ordered = ["Mod", "Alt", "Shift"].filter((modifier) => modifiers.has(modifier));
  const normalizedKey = key.length === 1 ? key.toLowerCase() : key;
  return [...ordered, normalizedKey].join("-");
}

export function chordFromKeyboardEvent(event: {
  key: string;
  ctrlKey?: boolean;
  metaKey?: boolean;
  shiftKey?: boolean;
  altKey?: boolean;
}): string {
  const parts: string[] = [];
  if (event.ctrlKey || event.metaKey) parts.push("Mod");
  if (event.altKey) parts.push("Alt");
  if (event.shiftKey) parts.push("Shift");
  parts.push(event.key.length === 1 ? event.key.toLowerCase() : event.key);
  return parts.join("-");
}

export function keyboardEventMatchesChord(
  event: {
    key: string;
    ctrlKey?: boolean;
    metaKey?: boolean;
    shiftKey?: boolean;
    altKey?: boolean;
  },
  chord: string,
): boolean {
  return normalizeChordInput(chordFromKeyboardEvent(event)) === normalizeChordInput(chord);
}

function isWorkspaceTheme(value: unknown): value is WorkspaceTheme {
  return value === "light" || value === "dark";
}

function normalizeCustomThemeTokens(value: unknown): Record<string, string> {
  if (!value || typeof value !== "object" || Array.isArray(value)) {
    return {};
  }
  return Object.fromEntries(
    Object.entries(value).filter((entry): entry is [string, string] => typeof entry[1] === "string"),
  );
}

export function normalizeWorkspaceSettingsState(
  value: unknown,
  fallback: WorkspaceSettingsState = defaultWorkspaceSettingsState(),
): WorkspaceSettingsState {
  if (!value || typeof value !== "object" || Array.isArray(value)) {
    return fallback;
  }
  const raw = value as Record<string, unknown>;
  if (raw.schema_id !== WORKSPACE_SETTINGS_SCHEMA_ID) {
    return fallback;
  }
  const rawKeybindings =
    raw.keybindings && typeof raw.keybindings === "object" && !Array.isArray(raw.keybindings)
      ? (raw.keybindings as Record<string, unknown>)
      : {};
  const rawSettings =
    raw.settings && typeof raw.settings === "object" && !Array.isArray(raw.settings)
      ? (raw.settings as Record<string, unknown>)
      : {};

  const keybindings = { ...fallback.keybindings };
  for (const action of APP_KEYBINDING_ACTIONS) {
    const candidate = rawKeybindings[action.id];
    if (typeof candidate === "string" && normalizeChordInput(candidate).length > 0) {
      keybindings[action.id] = normalizeChordInput(candidate);
    }
  }

  return {
    schema_id: WORKSPACE_SETTINGS_SCHEMA_ID,
    theme: isWorkspaceTheme(raw.theme) ? raw.theme : fallback.theme,
    custom_theme_tokens: normalizeCustomThemeTokens(raw.custom_theme_tokens),
    keybindings,
    settings: {
      view_mode: rawSettings.view_mode === "SFW" || rawSettings.view_mode === "NSFW"
        ? rawSettings.view_mode
        : fallback.settings.view_mode,
      swarm_board_default_open:
        typeof rawSettings.swarm_board_default_open === "boolean"
          ? rawSettings.swarm_board_default_open
          : fallback.settings.swarm_board_default_open,
    },
  };
}

export type KeybindingConflict = {
  chord: string;
  actionLabels: string[];
};

export function findKeybindingConflicts(
  keybindings: Record<AppKeybindingActionId, string>,
): KeybindingConflict[] {
  const labelsByChord = new Map<string, string[]>();
  for (const action of APP_KEYBINDING_ACTIONS) {
    const chord = normalizeChordInput(keybindings[action.id] ?? "");
    if (!chord) {
      continue;
    }
    labelsByChord.set(chord, [...(labelsByChord.get(chord) ?? []), action.label]);
  }
  return Array.from(labelsByChord.entries())
    .filter(([, labels]) => labels.length > 1)
    .map(([chord, actionLabels]) => ({ chord, actionLabels }));
}

export function keybindingLabelForConflict(labels: string[]): string {
  if (labels.length <= 1) return labels[0] ?? "";
  return `${labels.slice(0, -1).join(", ")} and ${labels[labels.length - 1]}`;
}
