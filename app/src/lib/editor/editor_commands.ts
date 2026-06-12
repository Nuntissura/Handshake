// WP-KERNEL-009 / MT-169 + MT-170 — editor command catalog.
//
// The single machine-readable catalog of every Handshake editor action, shared
// by the toolbar (MT-169) and the keyboard/command-palette (MT-170) so the two
// can never drift: the same command id drives a toolbar button, a palette row,
// and (where defined) a keyboard shortcut. Each command runs against a Tiptap
// Editor through a pure `run(editor)` thunk and reports `isActive(editor)` for
// toolbar highlighting — no hidden chat context, no global mutable state.
//
// Categories cover the §7.1.1.8 editor feature surface: formatting, headings,
// lists, tables, task lists, quotes/callouts, links (typed wikilinks), code
// (embedded Monaco), media/embeds, backlinks/graph, mentions/tags, and
// UserManual insertion. Commands whose target is provided interactively (a
// wikilink target, a media ref) expose `requiresArg` so the palette/toolbar can
// prompt before running.

import type { Editor } from "@tiptap/core";
import { makeCodeBlockAttrs } from "./code_block_serialization";
import { classifyWikilink } from "./wikilink";

export type EditorCommandCategory =
  | "format"
  | "block"
  | "list"
  | "table"
  | "link"
  | "code"
  | "embed"
  | "graph"
  | "mention"
  | "manual";

export interface EditorCommandArgSpec {
  /** Stable arg id. */
  id: string;
  /** Operator-facing label / prompt. */
  label: string;
  /** Placeholder example. */
  placeholder?: string;
}

export interface EditorCommandDescriptor {
  /** Stable command id (drives toolbar button + palette row + tests). */
  id: string;
  /** Operator-facing label. */
  label: string;
  category: EditorCommandCategory;
  /** Keyboard hint shown in the UI (display only; binding lives in keymap). */
  keyboardHint?: string;
  /** Search keywords for the command palette. */
  keywords: string[];
  /** Args the command needs (prompted before run); empty for direct actions. */
  args?: EditorCommandArgSpec[];
  /** Runs the command against the editor. `args` keyed by EditorCommandArgSpec.id. */
  run: (editor: Editor, args?: Record<string, string>) => boolean;
  /** Whether the command's mark/node is currently active (toolbar highlight). */
  isActive?: (editor: Editor) => boolean;
  /** Whether the command can run in the current state (enable/disable). */
  canRun?: (editor: Editor) => boolean;
}

/** True when the command needs interactive args before it can run. */
export function commandRequiresArg(cmd: EditorCommandDescriptor): boolean {
  return (cmd.args?.length ?? 0) > 0;
}

/**
 * The full catalog. Pure data + thunks; importing it does not touch the editor
 * or the network. The integrated editor binds these to toolbar buttons and the
 * command palette.
 */
export const EDITOR_COMMANDS: readonly EditorCommandDescriptor[] = [
  // --- Inline formatting ---
  {
    id: "format.bold",
    label: "Bold",
    category: "format",
    keyboardHint: "Mod-b",
    keywords: ["bold", "strong"],
    run: (e) => e.chain().focus().toggleBold().run(),
    isActive: (e) => e.isActive("bold"),
  },
  {
    id: "format.italic",
    label: "Italic",
    category: "format",
    keyboardHint: "Mod-i",
    keywords: ["italic", "emphasis"],
    run: (e) => e.chain().focus().toggleItalic().run(),
    isActive: (e) => e.isActive("italic"),
  },
  {
    id: "format.code",
    label: "Inline code",
    category: "format",
    keyboardHint: "Mod-e",
    keywords: ["code", "monospace", "inline"],
    run: (e) => e.chain().focus().toggleCode().run(),
    isActive: (e) => e.isActive("code"),
  },
  // --- Block structure ---
  {
    id: "block.h1",
    label: "Heading 1",
    category: "block",
    keywords: ["heading", "h1", "title"],
    run: (e) => e.chain().focus().toggleHeading({ level: 1 }).run(),
    isActive: (e) => e.isActive("heading", { level: 1 }),
  },
  {
    id: "block.h2",
    label: "Heading 2",
    category: "block",
    keywords: ["heading", "h2"],
    run: (e) => e.chain().focus().toggleHeading({ level: 2 }).run(),
    isActive: (e) => e.isActive("heading", { level: 2 }),
  },
  {
    id: "block.h3",
    label: "Heading 3",
    category: "block",
    keywords: ["heading", "h3"],
    run: (e) => e.chain().focus().toggleHeading({ level: 3 }).run(),
    isActive: (e) => e.isActive("heading", { level: 3 }),
  },
  {
    id: "block.paragraph",
    label: "Paragraph",
    category: "block",
    keywords: ["paragraph", "text", "body"],
    run: (e) => e.chain().focus().setParagraph().run(),
    isActive: (e) => e.isActive("paragraph"),
  },
  {
    id: "block.quote",
    label: "Block quote",
    category: "block",
    keywords: ["quote", "blockquote", "callout"],
    run: (e) => e.chain().focus().toggleBlockquote().run(),
    isActive: (e) => e.isActive("blockquote"),
  },
  // --- Lists ---
  {
    id: "list.bullet",
    label: "Bullet list",
    category: "list",
    keywords: ["bullet", "unordered", "list"],
    run: (e) => e.chain().focus().toggleBulletList().run(),
    isActive: (e) => e.isActive("bulletList"),
  },
  {
    id: "list.ordered",
    label: "Numbered list",
    category: "list",
    keywords: ["numbered", "ordered", "list"],
    run: (e) => e.chain().focus().toggleOrderedList().run(),
    isActive: (e) => e.isActive("orderedList"),
  },
  {
    id: "list.task",
    label: "Task list",
    category: "list",
    keywords: ["task", "todo", "checkbox", "checklist"],
    run: (e) => e.chain().focus().toggleTaskList().run(),
    isActive: (e) => e.isActive("taskList"),
  },
  // --- Tables ---
  {
    id: "table.insert",
    label: "Insert table",
    category: "table",
    keywords: ["table", "grid"],
    run: (e) => e.chain().focus().insertTable({ rows: 3, cols: 3, withHeaderRow: true }).run(),
  },
  // --- Code (embedded Monaco) ---
  {
    id: "code.insert",
    label: "Insert code block",
    category: "code",
    keyboardHint: "Mod-Alt-c",
    keywords: ["code", "monaco", "snippet", "fence"],
    args: [{ id: "language", label: "Language", placeholder: "typescript" }],
    run: (e, args) =>
      e
        .chain()
        .focus()
        .insertContent({
          type: "monacoCodeBlock",
          attrs: makeCodeBlockAttrs(args?.language ?? "plaintext", ""),
        })
        .run(),
    isActive: (e) => e.isActive("monacoCodeBlock"),
  },
  // --- Links (typed wikilinks) ---
  {
    id: "link.wikilink",
    label: "Insert link",
    category: "link",
    keyboardHint: "Mod-k",
    keywords: ["link", "wikilink", "note", "file", "wp", "spec", "reference"],
    args: [
      { id: "kind", label: "Link kind", placeholder: "wp / file / note / spec / symbol" },
      { id: "value", label: "Target", placeholder: "WP-KERNEL-009" },
      { id: "label", label: "Label (optional)", placeholder: "" },
    ],
    run: (e, args) => {
      const parsed = classifyWikilink(args?.kind ?? "", args?.value ?? "", args?.label);
      return e
        .chain()
        .focus()
        .insertContent({
          type: "hsLink",
          attrs: {
            refKind: parsed.refKind,
            refValue: parsed.refValue,
            label: parsed.label,
            resolved: parsed.resolved,
          },
        })
        .run();
    },
  },
  // --- Embeds (media/album/slideshow) ---
  {
    id: "embed.media",
    label: "Insert media embed",
    category: "embed",
    keywords: ["embed", "media", "image", "video", "album", "slideshow"],
    args: [
      { id: "kind", label: "Embed kind", placeholder: "images / album / video / HS_slideshow" },
      { id: "value", label: "Reference", placeholder: "album-id" },
    ],
    run: (e, args) => {
      const parsed = classifyWikilink(args?.kind ?? "", args?.value ?? "");
      return e
        .chain()
        .focus()
        .insertContent({
          type: "hsLink",
          attrs: {
            refKind: parsed.refKind,
            refValue: parsed.refValue,
            label: parsed.label,
            resolved: parsed.resolved,
          },
        })
        .run();
    },
  },
  // --- Graph / backlinks (typed link to another note) ---
  {
    id: "graph.backlink",
    label: "Link to note (backlink)",
    category: "graph",
    keywords: ["backlink", "graph", "note", "relate"],
    args: [{ id: "value", label: "Note", placeholder: "Runbook" }],
    run: (e, args) =>
      e
        .chain()
        .focus()
        .insertContent({
          type: "hsLink",
          attrs: { refKind: "note", refValue: args?.value ?? "", label: args?.value ?? "", resolved: true },
        })
        .run(),
  },
  // --- Mentions / tags ---
  {
    id: "mention.at",
    label: "Mention (@)",
    category: "mention",
    keywords: ["mention", "person", "agent", "@"],
    run: (e) => e.chain().focus().insertContent("@").run(),
  },
  {
    id: "mention.tag",
    label: "Tag (#)",
    category: "mention",
    keywords: ["tag", "label", "#", "knowledge"],
    run: (e) => e.chain().focus().insertContent("#").run(),
  },
  // --- UserManual insertion (typed link to the spec/manual) ---
  {
    id: "manual.insert",
    label: "Insert UserManual link",
    category: "manual",
    keywords: ["manual", "usermanual", "help", "spec", "docs"],
    args: [{ id: "value", label: "Manual anchor", placeholder: "7.1.1.8" }],
    run: (e, args) =>
      e
        .chain()
        .focus()
        .insertContent({
          type: "hsLink",
          attrs: { refKind: "spec", refValue: args?.value ?? "", label: `UserManual ${args?.value ?? ""}`, resolved: true },
        })
        .run(),
  },
];

/** Fast lookup of a command by id. */
export const EDITOR_COMMAND_BY_ID: ReadonlyMap<string, EditorCommandDescriptor> =
  new Map(EDITOR_COMMANDS.map((c) => [c.id, c]));

/**
 * Filters/ranks commands for the command palette by a query string (matches id,
 * label, and keywords; empty query returns all). Pure — no editor needed.
 */
export function filterEditorCommands(query: string): EditorCommandDescriptor[] {
  const q = query.trim().toLowerCase();
  if (q.length === 0) return [...EDITOR_COMMANDS];
  return EDITOR_COMMANDS.filter((c) => {
    if (c.id.toLowerCase().includes(q)) return true;
    if (c.label.toLowerCase().includes(q)) return true;
    return c.keywords.some((k) => k.toLowerCase().includes(q));
  });
}
