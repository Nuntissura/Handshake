// WP-KERNEL-009 / MT-164 — TiptapAutoCodeBlockRules tests.
//
// Proves (1) the pure detection turns fenced openers, fenced regions, and
// indented blobs into the right language + code, reversibly; (2) the extension
// registers the type-as-you-go input rule and the slash/prose commands against a
// real editor; (3) the paste handler inserts Monaco code block node(s) for
// pasted code and declines non-code. Uses a REAL @tiptap/core Editor.

import { describe, it, expect } from "vitest";
import { Editor } from "@tiptap/core";
import StarterKit from "@tiptap/starter-kit";
import { MonacoCodeBlockNode } from "./monaco_code_block_node";
import {
  AutoCodeBlockRules,
  codeBlocksFromPaste,
  handleCodeBlockPaste,
} from "./auto_code_block_rules";
import {
  detectFenceOpener,
  detectFencedBlocks,
  detectIndentedCodeBlock,
  codeBlockToFenced,
} from "../editor/auto_code_block";

describe("auto-code-block detection (MT-164)", () => {
  it("detects a fenced opener and normalizes the language", () => {
    expect(detectFenceOpener("```ts ")).toBe("typescript");
    expect(detectFenceOpener("``` ")).toBe("plaintext");
    expect(detectFenceOpener("not a fence")).toBeNull();
  });

  it("extracts fenced regions from a pasted blob in order", () => {
    const blob = "intro\n```ts\nconst a = 1;\n```\nmid\n```python\nx = 2\n```\n";
    const blocks = detectFencedBlocks(blob);
    expect(blocks).toHaveLength(2);
    expect(blocks[0]).toEqual({ language: "typescript", code: "const a = 1;" });
    expect(blocks[1]).toEqual({ language: "python", code: "x = 2" });
  });

  it("detects a 4-space / tab indented code block and de-indents it", () => {
    const indented = "    line one\n    line two";
    expect(detectIndentedCodeBlock(indented)).toBe("line one\nline two");
    const tabbed = "\tfoo\n\tbar";
    expect(detectIndentedCodeBlock(tabbed)).toBe("foo\nbar");
    // Not indented → null.
    expect(detectIndentedCodeBlock("plain text\nmore")).toBeNull();
    // Single line below the min threshold → null.
    expect(detectIndentedCodeBlock("    one")).toBeNull();
  });

  it("reverses a code block back to a fenced markdown string (round-trip)", () => {
    expect(codeBlockToFenced("rust", "fn main() {}")).toBe("```rust\nfn main() {}\n```");
  });

  it("codeBlocksFromPaste prefers fenced, falls back to indented, else none", () => {
    expect(codeBlocksFromPaste("```go\nx\n```")).toEqual([{ language: "go", code: "x" }]);
    expect(codeBlocksFromPaste("    a\n    b")).toEqual([{ language: "plaintext", code: "a\nb" }]);
    expect(codeBlocksFromPaste("just prose")).toEqual([]);
  });
});

function makeEditor(): Editor {
  return new Editor({
    extensions: [
      StarterKit.configure({ heading: { levels: [1, 2, 3] } }),
      MonacoCodeBlockNode,
      AutoCodeBlockRules,
    ],
    content: { type: "doc", content: [{ type: "paragraph" }] },
  });
}

function findNode(
  json: { type?: string; attrs?: Record<string, unknown>; content?: unknown[] },
  type: string,
): { type?: string; attrs?: Record<string, unknown> } | null {
  if (json.type === type) return json;
  for (const child of json.content ?? []) {
    const found = findNode(child as typeof json, type);
    if (found) return found;
  }
  return null;
}

describe("AutoCodeBlockRules extension (MT-164, real editor)", () => {
  it("registers the autoCodeBlockRules extension and a code-block input rule", () => {
    const editor = makeEditor();
    const ext = editor.extensionManager.extensions.find((e) => e.name === "autoCodeBlockRules");
    expect(ext).toBeDefined();
    type RuleFactory = (this: { editor: Editor }) => unknown[];
    const addInput = ext?.config.addInputRules as unknown as RuleFactory | undefined;
    expect(typeof addInput).toBe("function");
    expect(addInput?.call({ editor }).length).toBe(1);
    editor.destroy();
  });

  it("inserts a Monaco code block via the slash command", () => {
    const editor = makeEditor();
    editor.commands.insertCodeBlockFromSlash("rust");
    const node = findNode(editor.getJSON(), "monacoCodeBlock");
    expect(node?.attrs?.language).toBe("rust");
    editor.destroy();
  });

  it("converts selected prose into a code block (prose -> code)", () => {
    const editor = makeEditor();
    editor.commands.setContent({
      type: "doc",
      content: [{ type: "paragraph", content: [{ type: "text", text: "echo hello" }] }],
    });
    editor.commands.selectAll();
    editor.commands.proseToCodeBlock("shell");
    const node = findNode(editor.getJSON(), "monacoCodeBlock");
    expect(node?.attrs?.language).toBe("shell");
    expect(node?.attrs?.code).toContain("echo hello");
    editor.destroy();
  });

  it("handleCodeBlockPaste inserts code block(s) for pasted code and declines prose", () => {
    const editor = makeEditor();
    const handled = handleCodeBlockPaste(editor, "```json\n{\"a\":1}\n```");
    expect(handled).toBe(true);
    const node = findNode(editor.getJSON(), "monacoCodeBlock");
    expect(node?.attrs?.language).toBe("json");
    expect(node?.attrs?.code).toContain('"a":1');

    // Non-code paste is declined (returns false → default paste runs).
    const editor2 = makeEditor();
    expect(handleCodeBlockPaste(editor2, "just a sentence")).toBe(false);
    expect(findNode(editor2.getJSON(), "monacoCodeBlock")).toBeNull();
    editor.destroy();
    editor2.destroy();
  });

  it("paste is WIRED into the live editor via the registered handlePaste plugin (iteration-3 H6)", () => {
    const editor = makeEditor();
    // Drive the REAL registered plugin prop through the view's prop chain —
    // exactly how ProseMirror dispatches a paste event.
    const fenced = "```rust\nfn main() {}\n```";
    const fakeEvent = {
      clipboardData: { getData: (kind: string) => (kind === "text/plain" ? fenced : "") },
    } as unknown as ClipboardEvent;
    const handled = editor.view.someProp("handlePaste", (fn) =>
      fn(editor.view, fakeEvent, null as never),
    );
    expect(handled).toBe(true);
    const node = findNode(editor.getJSON(), "monacoCodeBlock");
    expect(node?.attrs?.language).toBe("rust");
    expect(node?.attrs?.code).toBe("fn main() {}");

    // A prose-only paste must fall through (no plugin claims it).
    const editor2 = makeEditor();
    const proseEvent = {
      clipboardData: {
        getData: (kind: string) => (kind === "text/plain" ? "plain words only" : ""),
      },
    } as unknown as ClipboardEvent;
    const fellThrough = editor2.view.someProp("handlePaste", (fn) =>
      fn(editor2.view, proseEvent, null as never),
    );
    expect(fellThrough ?? false).toBe(false);
    editor.destroy();
    editor2.destroy();
  });

  it("mixed prose+fence paste preserves the interleaved prose in order (iteration-3 H6)", () => {
    const editor = makeEditor();
    const blob = "Setup notes first.\n```shell\necho hi\n```\nAnd a closing remark.";
    expect(handleCodeBlockPaste(editor, blob)).toBe(true);
    const json = editor.getJSON();
    const kinds: string[] = [];
    const texts: string[] = [];
    for (const child of json.content ?? []) {
      kinds.push(child.type ?? "");
      if (child.type === "paragraph") {
        texts.push((child.content ?? []).map((c) => (c as { text?: string }).text ?? "").join(""));
      }
    }
    // Prose BEFORE the fence, the code block, prose AFTER — nothing dropped.
    expect(kinds).toContain("monacoCodeBlock");
    expect(texts).toContain("Setup notes first.");
    expect(texts).toContain("And a closing remark.");
    const codeIdx = kinds.indexOf("monacoCodeBlock");
    const beforeIdx = kinds.findIndex((k, i) => k === "paragraph" && texts.length > 0 && i < codeIdx);
    expect(beforeIdx).toBeGreaterThanOrEqual(0);
    expect(beforeIdx).toBeLessThan(codeIdx);
    editor.destroy();
  });

  it("codeToProse reverses a code block into paragraphs (iteration-3 M4 round-trip)", () => {
    const editor = makeEditor();
    editor.commands.setContent({
      type: "doc",
      content: [{ type: "paragraph", content: [{ type: "text", text: "line one" }] }],
    });
    editor.commands.selectAll();
    editor.commands.proseToCodeBlock("shell");
    // Find and node-select the produced block, then reverse it.
    let blockPos = -1;
    editor.state.doc.descendants((node, pos) => {
      if (node.type.name === "monacoCodeBlock") {
        blockPos = pos;
        return false;
      }
      return true;
    });
    expect(blockPos).toBeGreaterThanOrEqual(0);
    editor.commands.setNodeSelection(blockPos);
    expect(editor.commands.codeToProse()).toBe(true);
    const json = editor.getJSON();
    expect(findNode(json, "monacoCodeBlock")).toBeNull();
    const text = editor.state.doc.textBetween(0, editor.state.doc.content.size, "\n");
    expect(text).toContain("line one");
    editor.destroy();
  });

  it("codeToProse splits multi-line code into one paragraph per line and declines without a block", () => {
    const editor = makeEditor();
    editor.commands.insertCodeBlockFromSlash("python");
    let blockPos = -1;
    editor.state.doc.descendants((node, pos) => {
      if (node.type.name === "monacoCodeBlock") {
        blockPos = pos;
        return false;
      }
      return true;
    });
    editor.commands.setNodeSelection(blockPos);
    editor.commands.updateAttributes("monacoCodeBlock", {
      code: "a = 1\n\nb = 2",
    });
    editor.commands.setNodeSelection(blockPos);
    expect(editor.commands.codeToProse()).toBe(true);
    const text = editor.state.doc.textBetween(0, editor.state.doc.content.size, "\n");
    expect(text).toContain("a = 1");
    expect(text).toContain("b = 2");
    expect(findNode(editor.getJSON(), "monacoCodeBlock")).toBeNull();

    // No code block at the selection → declines.
    const editor2 = makeEditor();
    expect(editor2.commands.codeToProse()).toBe(false);
    editor.destroy();
    editor2.destroy();
  });
});
