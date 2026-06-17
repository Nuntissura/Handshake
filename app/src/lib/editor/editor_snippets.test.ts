// WP-KERNEL-009 / MT-251 — snippet expansion and tab-stop traversal tests.

import { describe, expect, it } from "vitest";
import { Editor } from "@tiptap/core";
import { buildHandshakeEditorExtensions } from "./build_editor_extensions";
import { EDITOR_COMMAND_BY_ID, filterEditorCommands } from "./editor_commands";
import {
  EDITOR_SNIPPETS,
  expandSnippetTemplate,
  getActiveProseSnippetSession,
  insertProseSnippet,
  monacoSnippetTemplateForId,
  moveToNextProseSnippetTabStop,
} from "./editor_snippets";

function makeEditor(): Editor {
  return new Editor({
    extensions: buildHandshakeEditorExtensions(),
    content: {
      type: "doc",
      content: [{ type: "paragraph", content: [{ type: "text", text: "hello" }] }],
    },
  });
}

function selectedText(editor: Editor): string {
  const { from, to } = editor.state.selection;
  return editor.state.doc.textBetween(from, to, "\n");
}

describe("editor snippets (MT-251)", () => {
  it("defines operator-reachable prose and code snippets", () => {
    expect(EDITOR_SNIPPETS.map((snippet) => snippet.id)).toEqual(
      expect.arrayContaining(["prose.meeting", "code.function"]),
    );
    expect(monacoSnippetTemplateForId("code.function")).toContain("${1:name}");
    expect(EDITOR_COMMAND_BY_ID.has("snippet.prose.meeting")).toBe(true);
    expect(filterEditorCommands("snippet").map((cmd) => cmd.id)).toContain("snippet.prose.meeting");
  });

  it("expands prose placeholders into plain text plus ordered tab-stop ranges", () => {
    const expanded = expandSnippetTemplate("Meeting: ${1:Topic} / Owner: ${2:Owner} / Notes: ${0}");

    expect(expanded.text).toBe("Meeting: Topic / Owner: Owner / Notes: ");
    expect(expanded.tabStops).toEqual([
      { index: 1, from: 9, to: 14 },
      { index: 2, from: 24, to: 29 },
      { index: 0, from: 39, to: 39 },
    ]);
  });

  it("inserts prose snippets and remaps later tab stops after placeholder edits", () => {
    const editor = makeEditor();
    editor.commands.selectAll();

    expect(insertProseSnippet(editor, "prose.meeting")).toBe(true);
    expect(selectedText(editor)).toBe("Topic");
    expect(getActiveProseSnippetSession(editor)?.snippetId).toBe("prose.meeting");

    editor.commands.insertContent("Roadmap");
    expect(moveToNextProseSnippetTabStop(editor)).toBe(true);
    expect(selectedText(editor)).toBe("Owner");

    editor.commands.insertContent("Ilja");
    expect(moveToNextProseSnippetTabStop(editor)).toBe(true);
    expect(editor.state.selection.empty).toBe(true);
    expect(getActiveProseSnippetSession(editor)).toBeNull();
    expect(editor.state.doc.textBetween(0, editor.state.doc.content.size, "\n")).toContain(
      "Meeting: Roadmap / Owner: Ilja / Notes: ",
    );
    editor.destroy();
  });

  it("declines invalid prose snippet requests without mutating the document", () => {
    const editor = makeEditor();
    const before = editor.getJSON();

    expect(insertProseSnippet(editor, "code.function")).toBe(false);
    expect(insertProseSnippet(editor, "missing.snippet")).toBe(false);
    expect(moveToNextProseSnippetTabStop(editor)).toBe(false);
    expect(monacoSnippetTemplateForId("prose.meeting")).toBeNull();
    expect(editor.getJSON()).toEqual(before);
    editor.destroy();
  });
});
