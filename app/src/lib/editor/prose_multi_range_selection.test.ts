// WP-KERNEL-009 / MT-251 — prose multi-range simultaneous edit tests.

import { describe, expect, it } from "vitest";
import { Editor } from "@tiptap/core";
import { buildHandshakeEditorExtensions } from "./build_editor_extensions";
import {
  addProseMultiRange,
  applyProseMultiRangeText,
  clearProseMultiRanges,
  getProseMultiRangeState,
} from "../tiptap/prose_multi_range_selection";

function makeEditor(): Editor {
  return new Editor({
    extensions: buildHandshakeEditorExtensions(),
    content: {
      type: "doc",
      content: [
        {
          type: "paragraph",
          content: [{ type: "text", text: "alpha beta gamma beta" }],
        },
      ],
    },
  });
}

function docText(editor: Editor): string {
  return editor.state.doc.textBetween(0, editor.state.doc.content.size, "\n");
}

function textRange(editor: Editor, needle: string, occurrence = 0): { from: number; to: number } {
  let seen = 0;
  let found: { from: number; to: number } | null = null;
  editor.state.doc.descendants((node, pos) => {
    if (!node.isText || !node.text || found) return true;
    let index = node.text.indexOf(needle);
    while (index >= 0) {
      if (seen === occurrence) {
        found = { from: pos + index, to: pos + index + needle.length };
        return false;
      }
      seen += 1;
      index = node.text.indexOf(needle, index + needle.length);
    }
    return true;
  });
  expect(found, `expected to find ${needle} occurrence ${occurrence}`).not.toBeNull();
  return found!;
}

describe("prose multi-range selection (MT-251 / EXT-MC-001)", () => {
  it("applies one text edit to noncontiguous prose ranges in a single undoable transaction", () => {
    const editor = makeEditor();

    expect(addProseMultiRange(editor, textRange(editor, "beta", 0))).toBe(true);
    expect(addProseMultiRange(editor, textRange(editor, "beta", 1))).toBe(true);
    expect(getProseMultiRangeState(editor).ranges).toHaveLength(2);

    expect(applyProseMultiRangeText(editor, "B")).toBe(true);
    expect(docText(editor)).toBe("alpha B gamma B");
    expect(getProseMultiRangeState(editor).ranges).toEqual([
      { from: 8, to: 8 },
      { from: 16, to: 16 },
    ]);

    expect(editor.commands.undo()).toBe(true);
    expect(docText(editor)).toBe("alpha beta gamma beta");
    editor.destroy();
  });

  it("declines empty or overlapping multi-range edits without mutating the document", () => {
    const editor = makeEditor();
    const before = editor.getJSON();

    expect(applyProseMultiRangeText(editor, "X")).toBe(false);
    expect(addProseMultiRange(editor, { from: 2, to: 2 })).toBe(false);
    expect(addProseMultiRange(editor, { from: 8, to: 12 })).toBe(true);
    expect(addProseMultiRange(editor, { from: 10, to: 16 })).toBe(false);
    expect(getProseMultiRangeState(editor).ranges).toEqual([{ from: 8, to: 12 }]);
    clearProseMultiRanges(editor);
    expect(getProseMultiRangeState(editor).ranges).toEqual([]);
    expect(editor.getJSON()).toEqual(before);
    editor.destroy();
  });
});
