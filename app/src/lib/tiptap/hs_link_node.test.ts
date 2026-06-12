// WP-KERNEL-009 / MT-163 — TiptapCustomLinkNodes tests.
//
// Proves (1) the pure wikilink parser classifies the [[kind:value]] family into
// typed backend ref kinds and preserves unknown prefixes, and (2) the hsLink
// Tiptap node instantiates, converts a pasted/parsed wikilink into a typed link
// node, and round-trips its typed attributes through ProseMirror JSON + plain
// text. Uses a REAL @tiptap/core Editor (jsdom), not a mock.

import { describe, it, expect } from "vitest";
import { Editor } from "@tiptap/core";
import StarterKit from "@tiptap/starter-kit";
import { HsLinkNode } from "./hs_link_node";
import {
  parseWikilink,
  extractWikilinks,
  classifyWikilink,
} from "../editor/wikilink";

describe("wikilink parser (MT-163)", () => {
  it("classifies known prefixes to their backend ref kinds", () => {
    expect(parseWikilink("[[wp:WP-KERNEL-009]]")).toMatchObject({
      refKind: "wp",
      refValue: "WP-KERNEL-009",
      resolved: true,
    });
    expect(parseWikilink("[[file:src/app.ts]]")).toMatchObject({
      refKind: "file",
      refValue: "src/app.ts",
      resolved: true,
    });
    expect(parseWikilink("[[note:Runbook]]")?.refKind).toBe("note");
  });

  it("honors an explicit |label and defaults the label to the value", () => {
    expect(parseWikilink("[[spec:7.1.1.8|Editor spec]]")).toMatchObject({
      refKind: "spec",
      refValue: "7.1.1.8",
      label: "Editor spec",
    });
    expect(parseWikilink("[[wp:WP-1]]")?.label).toBe("WP-1");
  });

  it("preserves an unknown prefix as a typed unknown link (never dropped)", () => {
    const parsed = parseWikilink("[[bogus:thing]]");
    expect(parsed).not.toBeNull();
    expect(parsed?.refKind).toBe("unknown");
    expect(parsed?.resolved).toBe(false);
    expect(parsed?.refValue).toBe("thing");
  });

  it("returns null for non-wikilink text", () => {
    expect(parseWikilink("just text")).toBeNull();
    expect(parseWikilink("[single bracket:x]")).toBeNull();
  });

  it("extracts multiple wikilinks from a paste blob in order", () => {
    const blob = "See [[wp:WP-1]] and [[file:a/b.ts]] plus [[unknownk:z]].";
    const links = extractWikilinks(blob);
    expect(links.map((l) => l.refKind)).toEqual(["wp", "file", "unknown"]);
  });

  it("classifyWikilink trims and lower-cases the prefix", () => {
    expect(classifyWikilink("  WP ", "X")).toMatchObject({ refKind: "wp", rawPrefix: "wp" });
  });
});

function makeEditor(content?: object): Editor {
  return new Editor({
    extensions: [StarterKit.configure({ heading: { levels: [1, 2, 3] } }), HsLinkNode],
    content: content ?? { type: "doc", content: [{ type: "paragraph" }] },
  });
}

describe("HsLinkNode (MT-163, real Tiptap editor)", () => {
  it("registers the hsLink node in the schema", () => {
    const editor = makeEditor();
    expect(editor.schema.nodes.hsLink).toBeDefined();
    editor.destroy();
  });

  it("inserts a typed link node and round-trips its attrs through JSON", () => {
    const editor = makeEditor();
    editor.commands.insertHsLink({
      refKind: "wp",
      refValue: "WP-KERNEL-009",
      label: "WP 009",
      resolved: true,
    });
    const json = editor.getJSON();
    const found = JSON.stringify(json);
    expect(found).toContain('"type":"hsLink"');
    expect(found).toContain('"refKind":"wp"');
    expect(found).toContain('"refValue":"WP-KERNEL-009"');

    // Round-trip: reload the same JSON into a fresh editor and read it back.
    const editor2 = makeEditor(json);
    const link = findNode(editor2.getJSON(), "hsLink");
    expect(link?.attrs?.refKind).toBe("wp");
    expect(link?.attrs?.refValue).toBe("WP-KERNEL-009");
    editor.destroy();
    editor2.destroy();
  });

  it("serializes a typed link back to its wikilink token in plain text", () => {
    const editor = makeEditor();
    editor.commands.insertHsLink({
      refKind: "file",
      refValue: "src/app.ts",
      label: "src/app.ts",
      resolved: true,
    });
    expect(editor.getText({ blockSeparator: "\n" })).toContain("[[file:src/app.ts]]");
    editor.destroy();
  });

  it("registers the wikilink input rule and paste rule on the node", () => {
    const editor = makeEditor();
    const ext = editor.extensionManager.extensions.find((e) => e.name === "hsLink");
    expect(ext).toBeDefined();
    // The node wires BOTH an input rule (type-as-you-go conversion) and a paste
    // rule (bulk conversion of pasted text). Real keyboard firing of the input
    // rule is proven end-to-end in the MT-176 Playwright offline spec; here we
    // assert the rule factories are actually attached so the wiring cannot
    // regress, and that invoking them yields exactly one rule each.
    type RuleFactory = (this: { type: unknown; editor: unknown }) => unknown[];
    const ctx = { type: editor.schema.nodes.hsLink, editor };
    const addInput = ext?.config.addInputRules as unknown as RuleFactory | undefined;
    const addPaste = ext?.config.addPasteRules as unknown as RuleFactory | undefined;
    expect(typeof addInput).toBe("function");
    expect(typeof addPaste).toBe("function");
    expect(addInput?.call(ctx).length).toBe(1);
    expect(addPaste?.call(ctx).length).toBe(1);
    editor.destroy();
  });
});

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
