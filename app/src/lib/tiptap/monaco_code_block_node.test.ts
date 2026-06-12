// WP-KERNEL-009 / MT-165 — MonacoEmbeddedCodeBlock node tests (schema layer).
//
// Proves the monacoCodeBlock node registers in the schema, its insert command
// mints normalized + hashed attrs (MT-168 bridge), the attrs round-trip through
// ProseMirror JSON (load->reload), language can be changed, and renderText emits
// a fenced code block for plain-text/markdown projection. Uses a REAL
// @tiptap/core Editor (no React NodeView mount here — Monaco needs a real
// browser; the live mount + offline-worker proof is the MT-176 Playwright spec).
//
// addNodeView returns a React renderer that @tiptap/core never invokes (only
// @tiptap/react's EditorContent does), so the core Editor exercises the schema,
// commands, and serialization without a DOM-mounted Monaco.

import { describe, it, expect } from "vitest";
import { Editor } from "@tiptap/core";
import StarterKit from "@tiptap/starter-kit";
import { MonacoCodeBlockNode } from "./monaco_code_block_node";
import { verifyCodeBlockIntegrity } from "../editor/code_block_serialization";

function makeEditor(content?: object): Editor {
  return new Editor({
    extensions: [StarterKit.configure({ heading: { levels: [1, 2, 3] } }), MonacoCodeBlockNode],
    content: content ?? { type: "doc", content: [{ type: "paragraph" }] },
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

describe("MonacoCodeBlockNode (MT-165 schema layer)", () => {
  it("registers the monacoCodeBlock node in the schema", () => {
    const editor = makeEditor();
    expect(editor.schema.nodes.monacoCodeBlock).toBeDefined();
    editor.destroy();
  });

  it("inserts a code block with normalized language + matching round-trip hash", () => {
    const editor = makeEditor();
    editor.commands.insertMonacoCodeBlock({ language: "ts", code: "const x = 1;" });
    const node = findNode(editor.getJSON(), "monacoCodeBlock");
    expect(node).not.toBeNull();
    expect(node?.attrs?.language).toBe("typescript"); // normalized from alias
    expect(node?.attrs?.code).toBe("const x = 1;");
    expect(
      verifyCodeBlockIntegrity({
        language: String(node?.attrs?.language),
        code: String(node?.attrs?.code),
        contentHash: String(node?.attrs?.contentHash),
      }),
    ).toBe(true);
    editor.destroy();
  });

  it("round-trips code + language + hash through ProseMirror JSON (load -> reload)", () => {
    const editor = makeEditor();
    editor.commands.insertMonacoCodeBlock({ language: "rust", code: "fn main() {}" });
    const json = editor.getJSON();

    // Reload the exact JSON into a fresh editor and read the node back.
    const editor2 = makeEditor(json);
    const node = findNode(editor2.getJSON(), "monacoCodeBlock");
    expect(node?.attrs?.language).toBe("rust");
    expect(node?.attrs?.code).toBe("fn main() {}");
    expect(
      verifyCodeBlockIntegrity({
        language: String(node?.attrs?.language),
        code: String(node?.attrs?.code),
        contentHash: String(node?.attrs?.contentHash),
      }),
    ).toBe(true);
    editor.destroy();
    editor2.destroy();
  });

  it("changes the code block language and keeps the hash consistent", () => {
    const editor = makeEditor();
    editor.commands.insertMonacoCodeBlock({ language: "json", code: '{"a":1}' });
    // Place selection on the node, then change language.
    editor.commands.setNodeSelection(
      // The code block was inserted after the initial empty paragraph; select it.
      findCodeBlockPos(editor),
    );
    editor.commands.setMonacoCodeBlockLanguage("yaml");
    const node = findNode(editor.getJSON(), "monacoCodeBlock");
    expect(node?.attrs?.language).toBe("yaml");
    expect(
      verifyCodeBlockIntegrity({
        language: String(node?.attrs?.language),
        code: String(node?.attrs?.code),
        contentHash: String(node?.attrs?.contentHash),
      }),
    ).toBe(true);
    editor.destroy();
  });

  it("serializes to a fenced code block in plain text (projection round-trip)", () => {
    const editor = makeEditor();
    editor.commands.insertMonacoCodeBlock({ language: "python", code: "print('hi')" });
    const text = editor.getText({ blockSeparator: "\n" });
    expect(text).toContain("```python");
    expect(text).toContain("print('hi')");
    editor.destroy();
  });
});

/** Finds the document position of the monacoCodeBlock node (for selection). */
function findCodeBlockPos(editor: Editor): number {
  let pos = 0;
  editor.state.doc.descendants((node, nodePos) => {
    if (node.type.name === "monacoCodeBlock") {
      pos = nodePos;
      return false;
    }
    return true;
  });
  return pos;
}
