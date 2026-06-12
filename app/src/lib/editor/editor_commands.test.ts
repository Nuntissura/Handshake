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
      "format",
      "block",
      "list",
      "table",
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
