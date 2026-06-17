import { EXPORT_FORMATS } from "./export_formats";
import { FIND_OPEN_ACTION, REPLACE_OPEN_ACTION, SAVE_ACTION } from "./editor_keymap";

export interface EditorComponentCommandDescriptor {
  id: string;
  label: string;
  keywords: string[];
}

export const EDITOR_COMPONENT_COMMANDS: readonly EditorComponentCommandDescriptor[] = [
  { id: FIND_OPEN_ACTION, label: "Find in document", keywords: ["find", "search", "match"] },
  { id: REPLACE_OPEN_ACTION, label: "Find and replace", keywords: ["replace", "find", "substitute"] },
  ...EXPORT_FORMATS.map((format) => ({
    id: `export.${format.id}`,
    label: `Export: ${format.label}`,
    keywords: ["export", "save", "download", format.extension, format.id],
  })),
];

export const EDITOR_SAVE_COMPONENT_COMMAND: EditorComponentCommandDescriptor = {
  id: SAVE_ACTION,
  label: "Save document",
  keywords: ["save", "persist", "write"],
};

export function editorComponentCommands(options: {
  includeSave: boolean;
}): readonly EditorComponentCommandDescriptor[] {
  return options.includeSave
    ? [...EDITOR_COMPONENT_COMMANDS, EDITOR_SAVE_COMPONENT_COMMAND]
    : EDITOR_COMPONENT_COMMANDS;
}

export function filterEditorComponentCommands(
  query: string,
  commands: readonly EditorComponentCommandDescriptor[],
): EditorComponentCommandDescriptor[] {
  const q = query.trim().toLowerCase();
  if (q.length === 0) return [...commands];
  return commands.filter(
    (cmd) =>
      cmd.id.toLowerCase().includes(q) ||
      cmd.label.toLowerCase().includes(q) ||
      cmd.keywords.some((keyword) => keyword.toLowerCase().includes(q)),
  );
}
