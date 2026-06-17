import { EDITOR_COMMANDS, type EditorCommandDescriptor } from "./editor/editor_commands";
import {
  editorComponentCommands,
  type EditorComponentCommandDescriptor,
} from "./editor/editor_component_commands";

export type AppCommandKind = "app" | "editor";

export type AppCommandDescriptor = {
  id: string;
  kind: AppCommandKind;
  label: string;
  description?: string;
  keywords: string[];
  stableId: string;
  disabled?: boolean;
  editorCommandId?: string;
  editorQuery?: string;
};

type BuildAppCommandRegistryOptions = {
  searchText: string;
  editorCommandsEnabled: boolean;
};

function stableEditorCommandId(commandId: string): string {
  return commandId
    .trim()
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-+|-+$/g, "");
}

function editorCommandToAppCommand(
  command: EditorCommandDescriptor,
  enabled: boolean,
): AppCommandDescriptor {
  return {
    id: `editor.${command.id}`,
    kind: "editor",
    label: command.label,
    description: `Editor command${command.keyboardHint ? ` (${command.keyboardHint})` : ""}.`,
    keywords: [
      command.id,
      command.category,
      command.keyboardHint ?? "",
      ...command.keywords,
    ].filter(Boolean),
    stableId: `hs-editor-command-${stableEditorCommandId(command.id)}`,
    disabled: !enabled,
    editorCommandId: command.id,
    editorQuery: command.label,
  };
}

function editorComponentCommandToAppCommand(
  command: EditorComponentCommandDescriptor,
  enabled: boolean,
): AppCommandDescriptor {
  return {
    id: `editor.${command.id}`,
    kind: "editor",
    label: command.label,
    description: "Editor component command.",
    keywords: [command.id, ...command.keywords],
    stableId: `hs-editor-command-${stableEditorCommandId(command.id)}`,
    disabled: !enabled,
    editorCommandId: command.id,
    editorQuery: command.label,
  };
}

export function buildAppCommandRegistry({
  searchText,
  editorCommandsEnabled,
}: BuildAppCommandRegistryOptions): AppCommandDescriptor[] {
  const trimmedSearch = searchText.trim();
  return [
    {
      id: "usermanual.open",
      kind: "app",
      label: "UserManual: Open",
      description: "Open the in-app UserManual diagnostics tab.",
      keywords: ["manual", "usermanual", "help", "diagnostics", "usermanual.open"],
      stableId: "hs-usermanual-palette-open",
    },
    {
      id: "usermanual.search",
      kind: "app",
      label: "UserManual: Search",
      description: trimmedSearch
        ? `Search UserManual for "${trimmedSearch}".`
        : "Open UserManual search.",
      keywords: ["manual", "usermanual", "search", "help", "usermanual.search"],
      stableId: "hs-usermanual-palette-search",
    },
    ...EDITOR_COMMANDS.map((command) => editorCommandToAppCommand(command, editorCommandsEnabled)),
    ...editorComponentCommands({ includeSave: true }).map((command) =>
      editorComponentCommandToAppCommand(command, editorCommandsEnabled),
    ),
  ];
}

export function resolveEditorAppCommand(
  actions: readonly AppCommandDescriptor[],
  actionId: string,
): AppCommandDescriptor | null {
  const action = actions.find((candidate) => candidate.id === actionId);
  return action?.kind === "editor" ? action : null;
}
