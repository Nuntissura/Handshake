// WP-KERNEL-009 / MT-245 — Workbench outline + status-bar model.
//
// Pure helpers over a minimal ProseMirror/Tiptap shape. The React editor uses
// these for live chrome, and tests use them directly so outline/status behavior
// cannot degrade into display-only labels.

export interface ChromeInspectableNode {
  type: { name: string };
  attrs: Record<string, unknown>;
  text?: string;
  textContent?: string;
  nodeSize?: number;
}

export interface ChromeInspectableDoc {
  descendants(fn: (node: ChromeInspectableNode, pos: number) => boolean | void): void;
  textBetween?(from: number, to: number, blockSeparator?: string, leafText?: string): string;
}

export interface ChromeInspectableEditor {
  isEditable: boolean;
  state: {
    selection: { from: number; to: number; empty: boolean };
    doc: ChromeInspectableDoc;
  };
}

export interface EditorOutlineItem {
  id: string;
  level: number;
  text: string;
  pos: number;
  selectionPos: number;
  empty: boolean;
}

export interface DocumentChromeStatus {
  dirty: boolean;
  saving: boolean;
  blocked: boolean;
  backendErrorKind?: string | null;
  lastSavedAt?: string | null;
}

export type EditorSaveState = "saved" | "dirty" | "saving" | "blocked" | "conflict" | "error";

export interface EditorStatusSnapshot {
  cursor: { line: number; column: number; from: number; to: number };
  wordCount: number;
  focusedCodeLanguage: string | null;
  focusedCodeNodePos: number | null;
  saveState: EditorSaveState;
  editable: boolean;
  lastSavedAt: string | null;
}

export interface FocusedCodeBlock {
  pos: number;
  language: string;
  code: string;
}

export const MONACO_CODE_BLOCK_STATUS_EVENT = "handshake:monaco-code-block-status";

export interface MonacoCursorSnapshot {
  focused: boolean;
  pos: number;
  language: string;
  line: number;
  column: number;
}

const WORD_RE = /[\p{L}\p{N}_]+/gu;

export function buildEditorOutline(doc: ChromeInspectableDoc): EditorOutlineItem[] {
  const outline: EditorOutlineItem[] = [];
  doc.descendants((node, pos) => {
    if (node.type.name !== "heading") return true;
    const rawLevel = Number(node.attrs.level ?? node.attrs.heading_level ?? 1);
    const level = Number.isFinite(rawLevel) ? Math.max(1, Math.min(6, Math.trunc(rawLevel))) : 1;
    const rawText = String(node.textContent ?? "").replace(/\s+/g, " ").trim();
    const empty = rawText.length === 0;
    const text = empty ? "Untitled heading" : rawText;
    const index = outline.length + 1;
    outline.push({
      id: `heading-${index}-${slugify(text)}`,
      level,
      text,
      pos,
      selectionPos: pos + 1,
      empty,
    });
    return true;
  });
  return outline;
}

export function buildEditorStatus(
  editor: ChromeInspectableEditor,
  documentStatus?: DocumentChromeStatus,
  monacoCursor?: MonacoCursorSnapshot | null,
): EditorStatusSnapshot {
  const { from, to } = editor.state.selection;
  const prefix = editor.state.doc.textBetween?.(0, from, "\n", "\n") ?? "";
  const cursorLines = prefix.split("\n");
  const focusedCode = findFocusedCodeBlock(editor.state.doc, from);
  const focusedMonacoCode =
    monacoCursor?.focused ? findFocusedCodeBlock(editor.state.doc, monacoCursor.pos) : null;
  const rawMonacoLine = Math.trunc(monacoCursor?.line ?? 1);
  const rawMonacoColumn = Math.trunc(monacoCursor?.column ?? 1);
  const monacoLine = Number.isFinite(rawMonacoLine) && rawMonacoLine > 0 ? rawMonacoLine : 1;
  const monacoColumn = Number.isFinite(rawMonacoColumn) && rawMonacoColumn > 0 ? rawMonacoColumn : 1;
  return {
    cursor: {
      line: focusedMonacoCode ? monacoLine : Math.max(1, cursorLines.length),
      column: focusedMonacoCode
        ? monacoColumn
        : Math.max(1, (cursorLines[cursorLines.length - 1] ?? "").length + 1),
      from,
      to,
    },
    wordCount: countDocumentWords(editor.state.doc),
    focusedCodeLanguage: focusedMonacoCode?.language ?? focusedCode?.language ?? null,
    focusedCodeNodePos: focusedMonacoCode?.pos ?? focusedCode?.pos ?? null,
    saveState: saveStateFromDocumentStatus(documentStatus),
    editable: editor.isEditable,
    lastSavedAt: documentStatus?.lastSavedAt ?? null,
  };
}

export function findFocusedCodeBlock(
  doc: ChromeInspectableDoc,
  selectionFrom: number,
): FocusedCodeBlock | null {
  let focused: FocusedCodeBlock | null = null;
  doc.descendants((node, pos) => {
    if (focused || node.type.name !== "monacoCodeBlock") return true;
    const end = pos + Math.max(1, Number(node.nodeSize ?? 1));
    if (selectionFrom < pos || selectionFrom > end) return true;
    focused = {
      pos,
      language: String(node.attrs.language ?? "plaintext"),
      code: String(node.attrs.code ?? ""),
    };
    return false;
  });
  return focused;
}

export function findOnlyCodeBlock(doc: ChromeInspectableDoc): FocusedCodeBlock | null {
  const blocks: FocusedCodeBlock[] = [];
  doc.descendants((node, pos) => {
    if (node.type.name !== "monacoCodeBlock") return true;
    blocks.push({
      pos,
      language: String(node.attrs.language ?? "plaintext"),
      code: String(node.attrs.code ?? ""),
    });
    return blocks.length <= 1;
  });
  return blocks.length === 1 ? blocks[0] : null;
}

export function codeLineRange(code: string, oneBasedLine: number): { start: number; end: number } | null {
  if (!Number.isInteger(oneBasedLine) || oneBasedLine < 1) return null;
  const lines = code.split("\n");
  if (oneBasedLine > lines.length) return null;
  let start = 0;
  for (let i = 0; i < oneBasedLine - 1; i += 1) {
    start += lines[i].length + 1;
  }
  return { start, end: start + lines[oneBasedLine - 1].length };
}

export function saveStateFromDocumentStatus(status?: DocumentChromeStatus): EditorSaveState {
  if (!status) return "saved";
  if (status.saving) return "saving";
  if (status.blocked) return "blocked";
  if (status.backendErrorKind === "conflict") return "conflict";
  if (status.backendErrorKind) return "error";
  if (status.dirty) return "dirty";
  return "saved";
}

function countDocumentWords(doc: ChromeInspectableDoc): number {
  const parts: string[] = [];
  doc.descendants((node) => {
    if (node.type.name === "text") {
      parts.push(String(node.text ?? ""));
    } else if (node.type.name === "monacoCodeBlock") {
      parts.push(String(node.attrs.code ?? ""));
    } else if (node.type.name === "hsLink") {
      parts.push(String(node.attrs.label ?? node.attrs.refValue ?? ""));
    }
    return true;
  });
  return [...parts.join(" ").matchAll(WORD_RE)].length;
}

function slugify(value: string): string {
  const slug = value
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-+|-+$/g, "");
  return slug || "untitled";
}
