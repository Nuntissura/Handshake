// WP-KERNEL-009 / MT-258 — loomTransclusion node tests.
//
// Proves the transclusion node is a content-free atom that round-trips ONLY its
// { refValue } reference through ProseMirror JSON/HTML/text (the NO-COPY
// serialization invariant: the host document never persists the source body),
// and that the `![[block]]` input/paste rules are wired. Uses a REAL
// @tiptap/core Editor (jsdom), not a mock.

import { describe, it, expect } from "vitest";
import { Editor } from "@tiptap/core";
import StarterKit from "@tiptap/starter-kit";
import {
  LoomTransclusionNode,
  LOOM_TRANSCLUSION_REGEX,
} from "./loom_transclusion_node";

function makeEditor(content?: object): Editor {
  return new Editor({
    extensions: [StarterKit.configure({ heading: { levels: [1, 2, 3] } }), LoomTransclusionNode],
    content: content ?? { type: "doc", content: [{ type: "paragraph" }] },
  });
}

function findNode(json: unknown, type: string): { attrs?: Record<string, unknown>; content?: unknown } | null {
  const node = json as { type?: string; content?: unknown[]; attrs?: Record<string, unknown> };
  if (node?.type === type) return node;
  if (Array.isArray(node?.content)) {
    for (const child of node.content) {
      const found = findNode(child, type);
      if (found) return found;
    }
  }
  return null;
}

describe("LoomTransclusionNode (MT-258, real Tiptap editor)", () => {
  it("registers the loomTransclusion node in the schema", () => {
    const editor = makeEditor();
    expect(editor.schema.nodes.loomTransclusion).toBeDefined();
    editor.destroy();
  });

  it("inserts a content-free atom node carrying only refValue (NO-COPY)", () => {
    const editor = makeEditor();
    editor.commands.insertLoomTransclusion({ refValue: "block-source" });
    const json = editor.getJSON();
    const node = findNode(json, "loomTransclusion");
    expect(node).not.toBeNull();
    expect(node?.attrs?.refValue).toBe("block-source");
    // The persisted node is an atom: it has NO content array (no source body).
    expect(node?.content).toBeUndefined();
    editor.destroy();
  });

  it("round-trips the reference through JSON without absorbing any body", () => {
    const editor = makeEditor();
    editor.commands.insertLoomTransclusion({ refValue: "blk-42" });
    const json = editor.getJSON();
    const editor2 = makeEditor(json);
    const node = findNode(editor2.getJSON(), "loomTransclusion");
    expect(node?.attrs?.refValue).toBe("blk-42");
    expect(JSON.stringify(editor2.getJSON())).not.toContain('"text"');
    editor.destroy();
    editor2.destroy();
  });

  it("serializes the node to its ![[block]] token in plain text", () => {
    const editor = makeEditor();
    editor.commands.insertLoomTransclusion({ refValue: "block-source" });
    expect(editor.getText({ blockSeparator: "\n" })).toContain("![[block-source]]");
    editor.destroy();
  });

  it("renders content-free HTML carrying only the reference (no copied body)", () => {
    const editor = makeEditor();
    editor.commands.insertLoomTransclusion({ refValue: "block-source" });
    const html = editor.getHTML();
    expect(html).toContain('data-testid="loom-transclusion"');
    expect(html).toContain('data-ref-value="block-source"');
    editor.destroy();
  });

  it("parses persisted transclusion HTML back into the atom node", () => {
    const editor = makeEditor();
    editor.commands.insertContent(
      `<div data-testid="loom-transclusion" data-ref-value="blk-html"></div>`,
    );
    const node = findNode(editor.getJSON(), "loomTransclusion");
    expect(node?.attrs?.refValue).toBe("blk-html");
    editor.destroy();
  });

  it("wires the ![[block]] input + paste rules", () => {
    const editor = makeEditor();
    const ext = editor.extensionManager.extensions.find((e) => e.name === "loomTransclusion");
    expect(ext).toBeDefined();
    type RuleFactory = (this: { type: unknown; editor: unknown }) => unknown[];
    const ctx = { type: editor.schema.nodes.loomTransclusion, editor };
    const addInput = ext?.config.addInputRules as unknown as RuleFactory | undefined;
    const addPaste = ext?.config.addPasteRules as unknown as RuleFactory | undefined;
    expect(addInput?.call(ctx).length).toBe(1);
    expect(addPaste?.call(ctx).length).toBe(1);
    editor.destroy();
  });

  it("matches ![[block]] tokens but not plain [[wikilinks]]", () => {
    expect(LOOM_TRANSCLUSION_REGEX.test("![[block-1]]")).toBe(true);
    expect("![[block-1|label]]".match(LOOM_TRANSCLUSION_REGEX)?.[1]).toBe("block-1");
    expect(LOOM_TRANSCLUSION_REGEX.test("[[wp:WP-1]]")).toBe(false);
  });
});
