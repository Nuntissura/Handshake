// WP-KERNEL-009 / MT-245 — Workbench outline + status-bar model tests.

import { describe, expect, it } from "vitest";
import { Editor, type JSONContent } from "@tiptap/core";
import { NodeSelection } from "@tiptap/pm/state";
import { buildHandshakeEditorExtensions } from "./build_editor_extensions";
import { makeCodeBlockAttrs } from "./code_block_serialization";
import {
  buildEditorOutline,
  buildEditorStatus,
  codeLineRange,
  type ChromeInspectableEditor,
} from "./editor_chrome";

function makeEditor(content: JSONContent): Editor {
  return new Editor({
    extensions: buildHandshakeEditorExtensions(),
    content,
  });
}

describe("editor chrome model (MT-245)", () => {
  it("builds a deterministic heading outline in document order", () => {
    const editor = makeEditor({
      type: "doc",
      content: [
        { type: "heading", attrs: { level: 1 }, content: [{ type: "text", text: "Runbook" }] },
        { type: "paragraph", content: [{ type: "text", text: "Body text" }] },
        { type: "heading", attrs: { level: 2 }, content: [{ type: "text", text: "Runbook" }] },
        { type: "heading", attrs: { level: 3 } },
      ],
    });

    const outline = buildEditorOutline(editor.state.doc);
    expect(outline.map((item) => [item.level, item.text])).toEqual([
      [1, "Runbook"],
      [2, "Runbook"],
      [3, "Untitled heading"],
    ]);
    expect(outline[0].id).not.toBe(outline[1].id);
    expect(outline[2].empty).toBe(true);
    expect(outline.every((item) => Number.isInteger(item.pos))).toBe(true);
    editor.destroy();
  });

  it("reports cursor position, word count, and focused code-block language", () => {
    const editor = makeEditor({
      type: "doc",
      content: [
        { type: "paragraph", content: [{ type: "text", text: "alpha beta" }] },
        {
          type: "monacoCodeBlock",
          attrs: makeCodeBlockAttrs("typescript", "const answer = 42;\nconsole.log(answer);"),
        },
      ],
    });
    const codePos = editor.state.doc.content.firstChild?.nodeSize ?? 1;
    editor.view.dispatch(editor.state.tr.setSelection(NodeSelection.create(editor.state.doc, codePos)));

    const status = buildEditorStatus(editor as unknown as ChromeInspectableEditor, {
      dirty: true,
      saving: false,
      blocked: false,
      backendErrorKind: null,
      lastSavedAt: null,
    });
    expect(status.cursor.line).toBeGreaterThanOrEqual(1);
    expect(status.cursor.column).toBeGreaterThanOrEqual(1);
    expect(status.wordCount).toBe(8);
    expect(status.focusedCodeLanguage).toBe("typescript");
    expect(status.saveState).toBe("dirty");
    editor.destroy();
  });

  it("uses an active Monaco cursor snapshot ahead of the prose selection for status", () => {
    const editor = makeEditor({
      type: "doc",
      content: [
        { type: "paragraph", content: [{ type: "text", text: "alpha beta" }] },
        {
          type: "monacoCodeBlock",
          attrs: makeCodeBlockAttrs("typescript", "const answer = 42;\nconsole.log(answer);"),
        },
      ],
    });
    const codePos = editor.state.doc.content.firstChild?.nodeSize ?? 1;
    editor.commands.setTextSelection(1);

    const status = buildEditorStatus(
      editor as unknown as ChromeInspectableEditor,
      {
        dirty: false,
        saving: false,
        blocked: false,
        backendErrorKind: null,
        lastSavedAt: null,
      },
      { focused: true, pos: codePos, language: "typescript", line: 2, column: 12 },
    );

    expect(status.cursor.line).toBe(2);
    expect(status.cursor.column).toBe(12);
    expect(status.focusedCodeLanguage).toBe("typescript");
    expect(status.focusedCodeNodePos).toBe(codePos);
    editor.destroy();
  });

  it("maps one-based code lines to character ranges and rejects invalid lines", () => {
    expect(codeLineRange("one\ntwo words\nthree", 1)).toEqual({ start: 0, end: 3 });
    expect(codeLineRange("one\ntwo words\nthree", 2)).toEqual({ start: 4, end: 13 });
    expect(codeLineRange("one\ntwo", 0)).toBeNull();
    expect(codeLineRange("one\ntwo", 3)).toBeNull();
    expect(codeLineRange("one\ntwo", Number.NaN)).toBeNull();
  });
});
