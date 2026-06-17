// WP-KERNEL-009 / MT-251 — editor snippets shared by prose and code surfaces.

import type { Editor } from "@tiptap/core";
import type { Node as ProseMirrorNode } from "@tiptap/pm/model";
import { TextSelection, type Transaction } from "@tiptap/pm/state";

export type EditorSnippetScope = "prose" | "code";

export interface EditorSnippetDefinition {
  id: string;
  label: string;
  scope: EditorSnippetScope;
  template: string;
  keywords: string[];
  language?: string;
}

export interface SnippetTabStop {
  index: number;
  from: number;
  to: number;
}

export interface ExpandedSnippet {
  text: string;
  tabStops: SnippetTabStop[];
}

interface ActiveProseSnippetSession {
  snippetId: string;
  tabStops: SnippetTabStop[];
  activeIndex: number;
  dispose: () => void;
}

export const EDITOR_SNIPPETS: readonly EditorSnippetDefinition[] = [
  {
    id: "prose.meeting",
    label: "Meeting snippet",
    scope: "prose",
    template: "Meeting: ${1:Topic} / Owner: ${2:Owner} / Notes: ${0}",
    keywords: ["snippet", "meeting", "owner", "notes", "prose"],
  },
  {
    id: "code.function",
    label: "Function snippet",
    scope: "code",
    language: "typescript",
    template: "function ${1:name}(${2:args}) {\n\t${0}\n}",
    keywords: ["snippet", "code", "function", "typescript"],
  },
];

const activeProseSessions = new WeakMap<Editor, ActiveProseSnippetSession>();

export function editorSnippetById(id: string): EditorSnippetDefinition | null {
  return EDITOR_SNIPPETS.find((snippet) => snippet.id === id) ?? null;
}

export function expandSnippetTemplate(template: string): ExpandedSnippet {
  const tabStops: SnippetTabStop[] = [];
  let text = "";
  let cursor = 0;
  const placeholderPattern = /\$\{(\d+)(?::([^}]*))?\}|\$(\d+)/g;
  for (const match of template.matchAll(placeholderPattern)) {
    const start = match.index ?? 0;
    text += template.slice(cursor, start);
    const index = Number(match[1] ?? match[3]);
    const value = match[2] ?? "";
    const from = text.length;
    text += value;
    const to = text.length;
    tabStops.push({ index, from, to });
    cursor = start + match[0].length;
  }
  text += template.slice(cursor);

  tabStops.sort((a, b) => {
    if (a.index === 0 && b.index !== 0) return 1;
    if (b.index === 0 && a.index !== 0) return -1;
    return a.index - b.index || a.from - b.from;
  });

  return { text, tabStops };
}

export function snippetDefaultText(id: string): string | null {
  const snippet = editorSnippetById(id);
  return snippet ? expandSnippetTemplate(snippet.template).text : null;
}

export function monacoSnippetTemplateForId(id: string): string | null {
  const snippet = editorSnippetById(id);
  return snippet?.scope === "code" ? snippet.template : null;
}

export function getActiveProseSnippetSession(
  editor: Editor,
): Omit<ActiveProseSnippetSession, "dispose"> | null {
  const session = activeProseSessions.get(editor);
  if (!session) return null;
  return {
    snippetId: session.snippetId,
    tabStops: session.tabStops.map((stop) => ({ ...stop })),
    activeIndex: session.activeIndex,
  };
}

export function clearProseSnippetSession(editor: Editor): void {
  const existing = activeProseSessions.get(editor);
  existing?.dispose();
  activeProseSessions.delete(editor);
}

function installProseSnippetSession(editor: Editor, session: ActiveProseSnippetSession): void {
  clearProseSnippetSession(editor);
  const onTransaction = ({ transaction }: { transaction: Transaction }) => {
    if (!transaction.docChanged) return;
    const current = activeProseSessions.get(editor);
    if (current !== session) return;
    session.tabStops = session.tabStops.map((stop) => ({
      ...stop,
      from: transaction.mapping.map(stop.from, -1),
      to: transaction.mapping.map(stop.to, 1),
    }));
  };
  session.dispose = () => editor.off("transaction", onTransaction);
  activeProseSessions.set(editor, session);
  editor.on("transaction", onTransaction);
}

function findInsertedTextStart(
  doc: ProseMirrorNode,
  text: string,
  preferredFrom: number,
): number | null {
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

export function insertProseSnippet(editor: Editor, snippetId: string): boolean {
  const snippet = editorSnippetById(snippetId);
  if (!snippet || snippet.scope !== "prose") return false;
  const expanded = expandSnippetTemplate(snippet.template);
  if (expanded.text.length === 0) return false;

  let nextSession: ActiveProseSnippetSession | null = null;
  const ran = editor
    .chain()
    .focus()
    .command(({ tr, dispatch }) => {
      if (!dispatch) return true;
      const insertFrom = tr.selection.from;
      tr.insertText(expanded.text, tr.selection.from, tr.selection.to);
      const insertedTextStart = findInsertedTextStart(tr.doc, expanded.text, insertFrom) ?? insertFrom;
      const absoluteStops = expanded.tabStops.map((stop) => ({
        ...stop,
        from: insertedTextStart + stop.from,
        to: insertedTextStart + stop.to,
      }));
      const first = absoluteStops[0];
      if (first) {
        tr.setSelection(TextSelection.create(tr.doc, first.from, first.to));
        nextSession = {
          snippetId,
          tabStops: absoluteStops,
          activeIndex: 0,
          dispose: () => {},
        };
      }
      tr.scrollIntoView();
      return true;
    })
    .run();

  if (!ran) return false;
  if (nextSession) {
    installProseSnippetSession(editor, nextSession);
  } else {
    clearProseSnippetSession(editor);
  }
  return true;
}

export function moveToNextProseSnippetTabStop(editor: Editor, direction = 1): boolean {
  const session = activeProseSessions.get(editor);
  if (!session) return false;
  const nextIndex = session.activeIndex + (direction < 0 ? -1 : 1);
  if (nextIndex < 0 || nextIndex >= session.tabStops.length) return false;

  session.activeIndex = nextIndex;
  const stop = session.tabStops[nextIndex];
  const reachedFinalStop = stop.index === 0 || nextIndex === session.tabStops.length - 1;
  const selected = editor.commands.setTextSelection({ from: stop.from, to: stop.to });
  if (reachedFinalStop) clearProseSnippetSession(editor);
  return selected;
}
