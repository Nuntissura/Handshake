// WP-KERNEL-009 / MT-169 + MT-170 — editor command catalog.
//
// The single machine-readable catalog of every Handshake editor action, shared
// by the toolbar (MT-169) and the keyboard/command-palette (MT-170) so the two
// can never drift: the same command id drives a toolbar button, a palette row,
// and (where defined) a keyboard shortcut. Each command runs against a Tiptap
// Editor through a pure `run(editor)` thunk and reports `isActive(editor)` for
// toolbar highlighting — no hidden chat context, no global mutable state.
//
// Categories cover the §7.1.1.8 editor feature surface: history, formatting,
// headings, lists, tables (+ structural table editing), task lists,
// quotes/callouts, links (typed wikilinks), code (embedded Monaco),
// media/embeds, backlinks/graph, mentions/tags, and UserManual insertion.
// Commands whose target is provided interactively (a wikilink target, a media
// ref, a mention) expose `requiresArg` so the palette/toolbar can prompt
// before running.

import type { Editor, JSONContent } from "@tiptap/core";
import { NodeSelection } from "@tiptap/pm/state";
import { makeCodeBlockAttrs } from "./code_block_serialization";
import { insertNoteTemplate } from "./editor_note_templates";
import { insertProseSnippet, snippetDefaultText } from "./editor_snippets";
import {
  addCurrentProseMultiRange,
  canAddCurrentProseMultiRange,
  clearProseMultiRanges,
  getProseMultiRangeState,
} from "../tiptap/prose_multi_range_selection";
import { classifyWikilink } from "./wikilink";

export type EditorCommandCategory =
  | "history"
  | "format"
  | "block"
  | "list"
  | "table"
  | "tableEdit"
  | "link"
  | "code"
  | "snippet"
  | "template"
  | "selection"
  | "embed"
  | "graph"
  | "mention"
  | "manual";

/**
 * Iteration-3 M11: NodeSelection-safe insertion. insertContent over a
 * NodeSelection REPLACES the selected node — running "insert code block" while
 * a code block was selected destroyed it. Inserting AFTER the selected node is
 * the non-destructive interpretation (VS Code parity: inserting a block while
 * a block is selected appends next to it).
 */
function insertSafely(e: Editor, content: JSONContent | JSONContent[]): boolean {
  const selection = e.state.selection;
  if (selection instanceof NodeSelection) {
    return e.chain().focus().insertContentAt(selection.to, content).run();
  }
  return e.chain().focus().insertContent(content).run();
}

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
 * command palette. Iteration-3 M11: `canRun` is now populated so the toolbar
 * and palette can truthfully disable commands the current state cannot run.
 */
export const EDITOR_COMMANDS: readonly EditorCommandDescriptor[] = [
  // --- History (iteration-3 L14). Bound natively by StarterKit (Mod-z /
  // Mod-Shift-z); the hints are display-only so the chord is never dispatched
  // twice. canRun keeps the buttons truthfully disabled at stack edges.
  {
    id: "history.undo",
    label: "Undo",
    category: "history",
    keyboardHint: "Mod-z",
    keywords: ["undo", "revert", "back"],
    run: (e) => e.chain().focus().undo().run(),
    canRun: (e) => e.can().undo(),
  },
  {
    id: "history.redo",
    label: "Redo",
    category: "history",
    keyboardHint: "Mod-Shift-z",
    keywords: ["redo", "again", "forward"],
    run: (e) => e.chain().focus().redo().run(),
    canRun: (e) => e.can().redo(),
  },
  // --- Inline formatting ---
  {
    id: "format.bold",
    label: "Bold",
    category: "format",
    keyboardHint: "Mod-b",
    keywords: ["bold", "strong"],
    run: (e) => e.chain().focus().toggleBold().run(),
    isActive: (e) => e.isActive("bold"),
    canRun: (e) => e.can().chain().toggleBold().run(),
  },
  {
    id: "format.italic",
    label: "Italic",
    category: "format",
    keyboardHint: "Mod-i",
    keywords: ["italic", "emphasis"],
    run: (e) => e.chain().focus().toggleItalic().run(),
    isActive: (e) => e.isActive("italic"),
    canRun: (e) => e.can().chain().toggleItalic().run(),
  },
  {
    id: "format.code",
    label: "Inline code",
    category: "format",
    keyboardHint: "Mod-e",
    keywords: ["code", "monospace", "inline"],
    run: (e) => e.chain().focus().toggleCode().run(),
    isActive: (e) => e.isActive("code"),
    canRun: (e) => e.can().chain().toggleCode().run(),
  },
  // --- Block structure ---
  {
    id: "block.h1",
    label: "Heading 1",
    category: "block",
    keywords: ["heading", "h1", "title"],
    run: (e) => e.chain().focus().toggleHeading({ level: 1 }).run(),
    isActive: (e) => e.isActive("heading", { level: 1 }),
    canRun: (e) => e.can().chain().toggleHeading({ level: 1 }).run(),
  },
  {
    id: "block.h2",
    label: "Heading 2",
    category: "block",
    keywords: ["heading", "h2"],
    run: (e) => e.chain().focus().toggleHeading({ level: 2 }).run(),
    isActive: (e) => e.isActive("heading", { level: 2 }),
    canRun: (e) => e.can().chain().toggleHeading({ level: 2 }).run(),
  },
  {
    id: "block.h3",
    label: "Heading 3",
    category: "block",
    keywords: ["heading", "h3"],
    run: (e) => e.chain().focus().toggleHeading({ level: 3 }).run(),
    isActive: (e) => e.isActive("heading", { level: 3 }),
    canRun: (e) => e.can().chain().toggleHeading({ level: 3 }).run(),
  },
  {
    id: "block.paragraph",
    label: "Paragraph",
    category: "block",
    keywords: ["paragraph", "text", "body"],
    run: (e) => e.chain().focus().setParagraph().run(),
    isActive: (e) => e.isActive("paragraph"),
    canRun: (e) => e.can().chain().setParagraph().run(),
  },
  {
    id: "block.quote",
    label: "Block quote",
    category: "block",
    keywords: ["quote", "blockquote", "callout"],
    run: (e) => e.chain().focus().toggleBlockquote().run(),
    isActive: (e) => e.isActive("blockquote"),
    canRun: (e) => e.can().chain().toggleBlockquote().run(),
  },
  // --- Lists ---
  {
    id: "list.bullet",
    label: "Bullet list",
    category: "list",
    keywords: ["bullet", "unordered", "list"],
    run: (e) => e.chain().focus().toggleBulletList().run(),
    isActive: (e) => e.isActive("bulletList"),
    canRun: (e) => e.can().chain().toggleBulletList().run(),
  },
  {
    id: "list.ordered",
    label: "Numbered list",
    category: "list",
    keywords: ["numbered", "ordered", "list"],
    run: (e) => e.chain().focus().toggleOrderedList().run(),
    isActive: (e) => e.isActive("orderedList"),
    canRun: (e) => e.can().chain().toggleOrderedList().run(),
  },
  {
    id: "list.task",
    label: "Task list",
    category: "list",
    keywords: ["task", "todo", "checkbox", "checklist"],
    run: (e) => e.chain().focus().toggleTaskList().run(),
    isActive: (e) => e.isActive("taskList"),
    canRun: (e) => e.can().chain().toggleTaskList().run(),
  },
  // --- Tables ---
  {
    id: "table.insert",
    label: "Insert table",
    category: "table",
    keywords: ["table", "grid"],
    run: (e) => e.chain().focus().insertTable({ rows: 3, cols: 3, withHeaderRow: true }).run(),
    // M11: insertTable has no position-targeted variant; refuse on a
    // NodeSelection instead of replacing the selected node.
    canRun: (e) => !(e.state.selection instanceof NodeSelection),
  },
  // --- Table structure editing (iteration-3 L12) — enabled inside a table.
  {
    id: "table.addRowBefore",
    label: "Add row above",
    category: "tableEdit",
    keywords: ["table", "row", "insert", "above"],
    run: (e) => e.chain().focus().addRowBefore().run(),
    canRun: (e) => e.can().addRowBefore(),
  },
  {
    id: "table.addRowAfter",
    label: "Add row below",
    category: "tableEdit",
    keywords: ["table", "row", "insert", "below"],
    run: (e) => e.chain().focus().addRowAfter().run(),
    canRun: (e) => e.can().addRowAfter(),
  },
  {
    id: "table.addColumnBefore",
    label: "Add column left",
    category: "tableEdit",
    keywords: ["table", "column", "insert", "left"],
    run: (e) => e.chain().focus().addColumnBefore().run(),
    canRun: (e) => e.can().addColumnBefore(),
  },
  {
    id: "table.addColumnAfter",
    label: "Add column right",
    category: "tableEdit",
    keywords: ["table", "column", "insert", "right"],
    run: (e) => e.chain().focus().addColumnAfter().run(),
    canRun: (e) => e.can().addColumnAfter(),
  },
  {
    id: "table.deleteRow",
    label: "Delete row",
    category: "tableEdit",
    keywords: ["table", "row", "delete", "remove"],
    run: (e) => e.chain().focus().deleteRow().run(),
    canRun: (e) => e.can().deleteRow(),
  },
  {
    id: "table.deleteColumn",
    label: "Delete column",
    category: "tableEdit",
    keywords: ["table", "column", "delete", "remove"],
    run: (e) => e.chain().focus().deleteColumn().run(),
    canRun: (e) => e.can().deleteColumn(),
  },
  {
    id: "table.toggleHeaderRow",
    label: "Toggle header row",
    category: "tableEdit",
    keywords: ["table", "header", "row", "toggle"],
    run: (e) => e.chain().focus().toggleHeaderRow().run(),
    canRun: (e) => e.can().toggleHeaderRow(),
  },
  {
    id: "table.toggleHeaderColumn",
    label: "Toggle header column",
    category: "tableEdit",
    keywords: ["table", "header", "column", "toggle"],
    run: (e) => e.chain().focus().toggleHeaderColumn().run(),
    canRun: (e) => e.can().toggleHeaderColumn(),
  },
  {
    id: "table.toggleHeaderCell",
    label: "Toggle header cell",
    category: "tableEdit",
    keywords: ["table", "header", "cell", "toggle"],
    run: (e) => e.chain().focus().toggleHeaderCell().run(),
    canRun: (e) => e.can().toggleHeaderCell(),
  },
  {
    id: "table.mergeCells",
    label: "Merge cells",
    category: "tableEdit",
    keywords: ["table", "cell", "merge", "join"],
    run: (e) => e.chain().focus().mergeCells().run(),
    canRun: (e) => e.can().mergeCells(),
  },
  {
    id: "table.splitCell",
    label: "Split cell",
    category: "tableEdit",
    keywords: ["table", "cell", "split"],
    run: (e) => e.chain().focus().splitCell().run(),
    canRun: (e) => e.can().splitCell(),
  },
  {
    id: "table.delete",
    label: "Delete table",
    category: "tableEdit",
    keywords: ["table", "delete", "remove"],
    run: (e) => e.chain().focus().deleteTable().run(),
    canRun: (e) => e.can().deleteTable(),
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
      insertSafely(e, {
        type: "monacoCodeBlock",
        attrs: makeCodeBlockAttrs(args?.language ?? "plaintext", ""),
      }),
    isActive: (e) => e.isActive("monacoCodeBlock"),
  },
  // --- Snippets (MT-251): prose snippets keep tab-stop state in the editor
  // snippet helper; code snippets insert a Monaco block seeded from the same
  // definition, while in-block snippet tab-stops are handled by Monaco itself.
  {
    id: "snippet.prose.meeting",
    label: "Insert meeting snippet",
    category: "snippet",
    keywords: ["snippet", "meeting", "owner", "notes", "prose"],
    run: (e) => insertProseSnippet(e, "prose.meeting"),
    canRun: (e) => e.isEditable,
  },
  {
    id: "snippet.code.function",
    label: "Insert function snippet block",
    category: "snippet",
    keywords: ["snippet", "code", "function", "typescript"],
    run: (e) =>
      insertSafely(e, {
        type: "monacoCodeBlock",
        attrs: makeCodeBlockAttrs("typescript", snippetDefaultText("code.function") ?? ""),
      }),
    isActive: (e) => e.isActive("monacoCodeBlock"),
    canRun: (e) => e.isEditable,
  },
  // --- Note templates (MT-258): structured, data-only templates with explicit
  // date/title/cursor variables; no eval and no markdown authority path.
  {
    id: "template.note.daily",
    label: "Insert daily note template",
    category: "template",
    keywords: ["template", "daily template", "daily", "note", "date", "title", "cursor"],
    args: [{ id: "title", label: "Title", placeholder: "Daily project note" }],
    run: (e, args) => insertNoteTemplate(e, "note.daily", { title: args?.title }),
    canRun: (e) => e.isEditable,
  },
  // --- Prose multi-range editing (MT-251 / EXT-MC-001) ---
  {
    id: "selection.addRange",
    label: "Add selection range",
    category: "selection",
    keywords: ["selection", "multi", "range", "cursor", "prose"],
    run: (e) => addCurrentProseMultiRange(e),
    canRun: (e) => e.isEditable && canAddCurrentProseMultiRange(e),
  },
  {
    id: "selection.clearRanges",
    label: "Clear selection ranges",
    category: "selection",
    keywords: ["selection", "multi", "range", "clear", "prose"],
    run: (e) => clearProseMultiRanges(e),
    canRun: (e) => e.isEditable && getProseMultiRangeState(e).ranges.length > 0,
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
      return insertSafely(e, {
        type: "hsLink",
        attrs: {
          refKind: parsed.refKind,
          refValue: parsed.refValue,
          label: parsed.label,
          resolved: parsed.resolved,
        },
      });
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
      return insertSafely(e, {
        type: "hsLink",
        attrs: {
          refKind: parsed.refKind,
          refValue: parsed.refValue,
          label: parsed.label,
          resolved: parsed.resolved,
        },
      });
    },
  },
  // --- Graph / backlinks (typed link to another note) ---
  {
    id: "graph.backlink",
    label: "Link to note (backlink)",
    category: "graph",
    keywords: ["backlink", "graph", "note", "relate"],
    args: [{ id: "value", label: "Note", placeholder: "Runbook" }],
    run: (e, args) => {
      // Iteration-3 M3: classify through the shared wikilink rules instead of
      // hardcoding resolved:true, so resolution semantics live in ONE place.
      const parsed = classifyWikilink("note", args?.value ?? "");
      return insertSafely(e, {
        type: "hsLink",
        attrs: {
          refKind: parsed.refKind,
          refValue: parsed.refValue,
          label: parsed.label,
          resolved: parsed.resolved,
        },
      });
    },
  },
  // --- Mentions / tags (iteration-3 M1: create REAL mention/tag nodes — the
  // previous commands inserted a bare "@"/"#" character, so no mention or tag
  // node could ever be created from the UI). ---
  {
    id: "mention.at",
    label: "Mention (@)",
    category: "mention",
    keywords: ["mention", "person", "agent", "@"],
    args: [{ id: "value", label: "Who to mention", placeholder: "operator" }],
    run: (e, args) => {
      const value = (args?.value ?? "").trim();
      if (value.length === 0) return false;
      return insertSafely(e, [
        { type: "mention", attrs: { id: value, label: value } },
        { type: "text", text: " " },
      ]);
    },
  },
  {
    id: "mention.tag",
    label: "Tag (#)",
    category: "mention",
    keywords: ["tag", "label", "#", "knowledge"],
    args: [{ id: "value", label: "Tag name", placeholder: "runbook" }],
    run: (e, args) => {
      const value = (args?.value ?? "").trim();
      if (value.length === 0) return false;
      return insertSafely(e, [
        { type: "tagMention", attrs: { id: value, label: value } },
        { type: "text", text: " " },
      ]);
    },
  },
  // --- UserManual insertion (typed link to the spec/manual) ---
  {
    id: "manual.insert",
    label: "Insert UserManual link",
    category: "manual",
    keywords: ["manual", "usermanual", "help", "spec", "docs"],
    args: [{ id: "value", label: "Manual anchor", placeholder: "7.1.1.8" }],
    run: (e, args) => {
      const parsed = classifyWikilink("spec", args?.value ?? "", `UserManual ${args?.value ?? ""}`);
      return insertSafely(e, {
        type: "hsLink",
        attrs: {
          refKind: parsed.refKind,
          refValue: parsed.refValue,
          label: parsed.label,
          resolved: parsed.resolved,
        },
      });
    },
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
