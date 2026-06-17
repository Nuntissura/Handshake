// WP-KERNEL-009 / MT-258 - data-driven note templates for the rich editor.
//
// Templates are structured data with a small explicit variable vocabulary.
// Insertion creates real ProseMirror nodes; no template text is evaluated as
// code and no markdown parser is used as an authority path.

import type { Editor, JSONContent } from "@tiptap/core";
import type { Node as ProseMirrorNode } from "@tiptap/pm/model";
import { NodeSelection, TextSelection } from "@tiptap/pm/state";
import { clearProseSnippetSession } from "./editor_snippets";

type NoteTemplateVariable = "date" | "title" | "cursor";

export type NoteTemplateBlock =
  | { kind: "heading"; level: 1 | 2 | 3; text: string }
  | { kind: "paragraph"; text: string };

export interface NoteTemplateDefinition {
  id: string;
  label: string;
  keywords: string[];
  blocks: readonly NoteTemplateBlock[];
}

export interface NoteTemplateVariables {
  title?: string | null;
  now?: Date;
}

export interface ExpandedNoteTemplateText {
  text: string;
  cursorOffset: number | null;
}

const CURSOR_MARKER = "\uE000HS_TEMPLATE_CURSOR\uE000";
const VARIABLE_PATTERN = /\{\{\s*(date|title|cursor)\s*\}\}/g;

export const NOTE_TEMPLATES: readonly NoteTemplateDefinition[] = [
  {
    id: "note.daily",
    label: "Daily note template",
    keywords: ["template", "daily template", "daily", "note", "date", "title", "cursor"],
    blocks: [
      { kind: "heading", level: 1, text: "{{title}}" },
      { kind: "paragraph", text: "Date: {{date}}" },
      { kind: "heading", level: 2, text: "Notes" },
      { kind: "paragraph", text: "{{cursor}}" },
      { kind: "heading", level: 2, text: "Links" },
      { kind: "paragraph", text: "" },
    ],
  },
];

export function noteTemplateById(id: string): NoteTemplateDefinition | null {
  return NOTE_TEMPLATES.find((template) => template.id === id) ?? null;
}

export function formatNoteTemplateDate(now = new Date()): string {
  const year = now.getFullYear();
  const month = String(now.getMonth() + 1).padStart(2, "0");
  const day = String(now.getDate()).padStart(2, "0");
  return `${year}-${month}-${day}`;
}

function variableValue(
  variable: NoteTemplateVariable,
  variables: NoteTemplateVariables,
  cursorReplacement: string,
): string {
  if (variable === "date") return formatNoteTemplateDate(variables.now);
  if (variable === "title") return variables.title?.trim() || "Untitled note";
  return cursorReplacement;
}

export function expandNoteTemplateText(
  templateText: string,
  variables: NoteTemplateVariables = {},
): ExpandedNoteTemplateText {
  return expandNoteTemplateTextWithCursor(templateText, variables, "");
}

function expandNoteTemplateTextWithCursor(
  templateText: string,
  variables: NoteTemplateVariables,
  cursorReplacement: string,
): ExpandedNoteTemplateText {
  let text = "";
  let cursor = 0;
  let cursorOffset: number | null = null;

  for (const match of templateText.matchAll(VARIABLE_PATTERN)) {
    const start = match.index ?? 0;
    text += templateText.slice(cursor, start);
    const variable = match[1] as NoteTemplateVariable;
    if (variable === "cursor" && cursorOffset === null) {
      cursorOffset = text.length;
    }
    text += variableValue(variable, variables, cursorReplacement);
    cursor = start + match[0].length;
  }

  text += templateText.slice(cursor);
  return { text, cursorOffset };
}

function blockToJson(
  block: NoteTemplateBlock,
  variables: NoteTemplateVariables,
  cursorReplacement: string,
): JSONContent {
  const { text } = expandNoteTemplateTextWithCursor(block.text, variables, cursorReplacement);
  const content = text.length > 0 ? [{ type: "text", text }] : undefined;
  if (block.kind === "heading") {
    return {
      type: "heading",
      attrs: { level: block.level },
      content,
    };
  }
  return { type: "paragraph", content };
}

export function renderNoteTemplateContent(
  templateId: string,
  variables: NoteTemplateVariables = {},
): JSONContent[] | null {
  const template = noteTemplateById(templateId);
  if (!template) return null;
  return template.blocks.map((block) => blockToJson(block, variables, ""));
}

function renderNoteTemplateContentForInsert(
  template: NoteTemplateDefinition,
  variables: NoteTemplateVariables,
): JSONContent[] {
  return template.blocks.map((block) => blockToJson(block, variables, CURSOR_MARKER));
}

function findTextStart(doc: ProseMirrorNode, text: string, preferredFrom: number): number | null {
  let bestFrom: number | null = null;
  let bestDistance = Number.POSITIVE_INFINITY;
  doc.descendants((node, pos) => {
    if (!node.isText) return true;
    const value = node.text ?? "";
    const index = value.indexOf(text);
    if (index < 0) return true;
    const from = pos + index;
    const distance = Math.abs(from - preferredFrom);
    if (distance < bestDistance) {
      bestFrom = from;
      bestDistance = distance;
    }
    return true;
  });
  return bestFrom;
}

export function insertNoteTemplate(
  editor: Editor,
  templateId: string,
  variables: NoteTemplateVariables = {},
): boolean {
  const template = noteTemplateById(templateId);
  if (!template || !editor.isEditable) return false;

  const content = renderNoteTemplateContentForInsert(template, variables);
  const selection = editor.state.selection;
  const insertAt =
    selection instanceof NodeSelection
      ? selection.to
      : { from: selection.from, to: selection.to };
  const preferredFrom = selection.from;
  const ran = editor
    .chain()
    .focus()
    .insertContentAt(insertAt, content)
    .command(({ tr, dispatch }) => {
      if (!dispatch) return true;
      const markerStart = findTextStart(tr.doc, CURSOR_MARKER, preferredFrom);
      if (markerStart === null) return true;
      tr.delete(markerStart, markerStart + CURSOR_MARKER.length);
      tr.setSelection(TextSelection.create(tr.doc, markerStart));
      tr.scrollIntoView();
      return true;
    })
    .run();

  if (ran) clearProseSnippetSession(editor);
  return ran;
}
