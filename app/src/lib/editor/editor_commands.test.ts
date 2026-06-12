// WP-KERNEL-009 / MT-169 — editor command catalog tests.
//
// Proves the command catalog is well-formed (unique ids, every command runs and
// reports active state), covers the §7.1.1.8 feature categories, and actually
// mutates a REAL editor (formatting toggles, code-block insert, typed wikilink
// insert, table insert). The palette filter is also covered (shared with MT-170).

import { describe, it, expect } from "vitest";
import { Editor } from "@tiptap/core";
import { buildHandshakeEditorExtensions } from "./build_editor_extensions";
import {
  EDITOR_COMMANDS,
  EDITOR_COMMAND_BY_ID,
  filterEditorCommands,
  commandRequiresArg,
  type EditorCommandCategory,
} from "./editor_commands";

function makeEditor(): Editor {
  return new Editor({
    extensions: buildHandshakeEditorExtensions(),
    content: { type: "doc", content: [{ type: "paragraph", content: [{ type: "text", text: "hello world" }] }] },
  });
}

function findNode(
  json: { type?: string; content?: unknown[] },
  type: string,
): boolean {
  if (json.type === type) return true;
  return (json.content ?? []).some((c) => findNode(c as typeof json, type));
}

describe("editor command catalog (MT-169)", () => {
  it("has unique command ids and a lookup map", () => {
    const ids = EDITOR_COMMANDS.map((c) => c.id);
    expect(new Set(ids).size).toBe(ids.length);
    expect(EDITOR_COMMAND_BY_ID.get("format.bold")?.label).toBe("Bold");
  });

  it("covers the full editor feature surface (categories)", () => {
    const categories = new Set(EDITOR_COMMANDS.map((c) => c.category));
    const required: EditorCommandCategory[] = [
      "history",
      "format",
      "block",
      "list",
      "table",
      "tableEdit",
      "link",
      "code",
      "embed",
      "graph",
      "mention",
      "manual",
    ];
    for (const cat of required) expect(categories.has(cat)).toBe(true);
  });

  it("toggles bold on a real editor and reports active state", () => {
    const editor = makeEditor();
    editor.commands.selectAll();
    const bold = EDITOR_COMMAND_BY_ID.get("format.bold")!;
    expect(bold.isActive?.(editor)).toBe(false);
    bold.run(editor);
    expect(bold.isActive?.(editor)).toBe(true);
    editor.destroy();
  });

  it("inserts an embedded Monaco code block via the code command (with arg)", () => {
    const editor = makeEditor();
    const code = EDITOR_COMMAND_BY_ID.get("code.insert")!;
    expect(commandRequiresArg(code)).toBe(true);
    code.run(editor, { language: "rust" });
    expect(findNode(editor.getJSON(), "monacoCodeBlock")).toBe(true);
    editor.destroy();
  });

  it("inserts a typed wikilink via the link command", () => {
    const editor = makeEditor();
    const link = EDITOR_COMMAND_BY_ID.get("link.wikilink")!;
    link.run(editor, { kind: "wp", value: "WP-KERNEL-009" });
    expect(findNode(editor.getJSON(), "hsLink")).toBe(true);
    editor.destroy();
  });

  it("inserts a table via the table command", () => {
    const editor = makeEditor();
    EDITOR_COMMAND_BY_ID.get("table.insert")!.run(editor);
    expect(findNode(editor.getJSON(), "table")).toBe(true);
    editor.destroy();
  });

  it("filters commands for the palette by id, label, and keywords", () => {
    expect(filterEditorCommands("bold").some((c) => c.id === "format.bold")).toBe(true);
    expect(filterEditorCommands("checkbox").some((c) => c.id === "list.task")).toBe(true);
    expect(filterEditorCommands("monaco").some((c) => c.id === "code.insert")).toBe(true);
    expect(filterEditorCommands("").length).toBe(EDITOR_COMMANDS.length);
    expect(filterEditorCommands("zzzznotacommand").length).toBe(0);
  });
});

describe("command correctness hardening (iteration-3 M11/L12/L14/M1)", () => {
  function selectFirstCodeBlock(editor: Editor): number {
    let pos = -1;
    editor.state.doc.descendants((node, p) => {
      if (node.type.name === "monacoCodeBlock") {
        pos = p;
        return false;
      }
      return true;
    });
    editor.commands.setNodeSelection(pos);
    return pos;
  }

  function countNodes(editor: Editor, type: string): number {
    let count = 0;
    editor.state.doc.descendants((node) => {
      if (node.type.name === type) count += 1;
      return true;
    });
    return count;
  }

  it("M11: insert commands on a NodeSelection insert AFTER the node, never replace it", () => {
    const editor = makeEditor();
    EDITOR_COMMAND_BY_ID.get("code.insert")!.run(editor, { language: "rust" });
    expect(countNodes(editor, "monacoCodeBlock")).toBe(1);

    // Node-select the block, then run insert commands — the selected block
    // must SURVIVE every one of them.
    selectFirstCodeBlock(editor);
    EDITOR_COMMAND_BY_ID.get("code.insert")!.run(editor, { language: "python" });
    expect(countNodes(editor, "monacoCodeBlock")).toBe(2);

    selectFirstCodeBlock(editor);
    EDITOR_COMMAND_BY_ID.get("link.wikilink")!.run(editor, { kind: "wp", value: "WP-1" });
    expect(countNodes(editor, "monacoCodeBlock")).toBe(2);
    expect(countNodes(editor, "hsLink")).toBe(1);

    // table.insert has no position-targeted variant: canRun refuses instead.
    selectFirstCodeBlock(editor);
    expect(EDITOR_COMMAND_BY_ID.get("table.insert")!.canRun!(editor)).toBe(false);
    editor.destroy();
  });

  it("L14: undo/redo commands report truthful canRun and actually revert/reapply", () => {
    const editor = makeEditor();
    const undo = EDITOR_COMMAND_BY_ID.get("history.undo")!;
    const redo = EDITOR_COMMAND_BY_ID.get("history.redo")!;
    expect(undo.canRun!(editor)).toBe(false);
    expect(redo.canRun!(editor)).toBe(false);

    editor.commands.insertContent(" edited");
    expect(undo.canRun!(editor)).toBe(true);
    undo.run(editor);
    expect(editor.state.doc.textBetween(0, editor.state.doc.content.size, "\n")).toBe(
      "hello world",
    );
    expect(redo.canRun!(editor)).toBe(true);
    redo.run(editor);
    expect(editor.state.doc.textBetween(0, editor.state.doc.content.size, "\n")).toContain(
      "edited",
    );
    editor.destroy();
  });

  it("L12: table structure commands enable inside a table and edit it", () => {
    const editor = makeEditor();
    const addRow = EDITOR_COMMAND_BY_ID.get("table.addRowAfter")!;
    const deleteTable = EDITOR_COMMAND_BY_ID.get("table.delete")!;
    // Outside a table: truthfully disabled.
    expect(addRow.canRun!(editor)).toBe(false);
    expect(deleteTable.canRun!(editor)).toBe(false);

    EDITOR_COMMAND_BY_ID.get("table.insert")!.run(editor);
    // insertTable places the caret inside the new table.
    expect(addRow.canRun!(editor)).toBe(true);
    const rowsBefore = countNodes(editor, "tableRow");
    addRow.run(editor);
    expect(countNodes(editor, "tableRow")).toBe(rowsBefore + 1);

    expect(deleteTable.canRun!(editor)).toBe(true);
    deleteTable.run(editor);
    expect(countNodes(editor, "table")).toBe(0);
    editor.destroy();
  });

  it("M1: mention/tag commands create REAL mention and tagMention nodes", () => {
    const editor = makeEditor();
    const mention = EDITOR_COMMAND_BY_ID.get("mention.at")!;
    const tag = EDITOR_COMMAND_BY_ID.get("mention.tag")!;
    expect(commandRequiresArg(mention)).toBe(true);
    expect(commandRequiresArg(tag)).toBe(true);

    expect(mention.run(editor, { value: "operator" })).toBe(true);
    expect(tag.run(editor, { value: "runbook" })).toBe(true);
    expect(countNodes(editor, "mention")).toBe(1);
    expect(countNodes(editor, "tagMention")).toBe(1);

    // Empty values decline instead of inserting junk.
    expect(mention.run(editor, { value: "  " })).toBe(false);
    expect(tag.run(editor, {})).toBe(false);
    expect(countNodes(editor, "mention")).toBe(1);
    editor.destroy();
  });

  it("M3: graph/manual links classify through the shared wikilink rules", () => {
    const editor = makeEditor();
    EDITOR_COMMAND_BY_ID.get("graph.backlink")!.run(editor, { value: "Runbook" });
    EDITOR_COMMAND_BY_ID.get("manual.insert")!.run(editor, { value: "7.1.1.8" });
    const links: Array<Record<string, unknown>> = [];
    editor.state.doc.descendants((node) => {
      if (node.type.name === "hsLink") links.push(node.attrs);
      return true;
    });
    expect(links).toHaveLength(2);
    // Both kinds are known wikilink prefixes -> classified (not hardcoded).
    expect(links[0].refKind).toBe("note");
    expect(links[1].refKind).toBe("spec");
    editor.destroy();
  });
});
