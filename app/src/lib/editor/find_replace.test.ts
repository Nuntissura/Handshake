// WP-KERNEL-009 / MT-244 — document-wide find/replace core tests.
//
// Proves against a REAL editor + schema (no mocks): the scan finds matches in
// prose AND inside embedded Monaco code blocks; case/whole-word/regex
// semantics match VS Code; regex replacement expands $1/$&; replace-all
// rewrites prose and code in ONE transaction (single undo step restores the
// exact document including code hashes); invalid regex is a typed error; the
// match cap truncates instead of hanging; zero-length regex matches cannot
// loop forever.

import { describe, it, expect } from "vitest";
import { Editor, type JSONContent } from "@tiptap/core";
import { buildHandshakeEditorExtensions } from "./build_editor_extensions";
import { makeCodeBlockAttrs, codeBlockHash } from "./code_block_serialization";
import {
  buildFindMatches,
  compileFindQuery,
  expandReplacement,
  applyReplaceAll,
  applyReplaceMatch,
  indexAfterSelection,
  matchOrderKey,
  selectMatch,
  MAX_MATCHES,
  type FindQuery,
  type CodeMatch,
  type ProseMatch,
} from "./find_replace";

function makeEditor(doc: JSONContent): Editor {
  return new Editor({ extensions: buildHandshakeEditorExtensions(), content: doc });
}

function query(term: string, overrides: Partial<FindQuery> = {}): FindQuery {
  return { term, caseSensitive: false, wholeWord: false, isRegex: false, ...overrides };
}

const DOC: JSONContent = {
  type: "doc",
  content: [
    { type: "paragraph", content: [{ type: "text", text: "Alpha alpha ALPHA alphabet" }] },
    { type: "monacoCodeBlock", attrs: makeCodeBlockAttrs("typescript", "const alpha = 'alpha';\nlet Alphabet = 1;") },
    { type: "paragraph", content: [{ type: "text", text: "tail alpha" }] },
  ],
};

describe("buildFindMatches (prose + code blocks)", () => {
  it("finds matches across prose AND code blocks in document order", () => {
    const editor = makeEditor(DOC);
    const scan = buildFindMatches(editor.state.doc, query("alpha"));
    expect(scan.error).toBeNull();
    // prose: Alpha, alpha, ALPHA, alphabet → 4; code: alpha, alpha, Alphabet → 3; tail: 1.
    expect(scan.matches).toHaveLength(8);
    const kinds = scan.matches.map((m) => m.kind);
    expect(kinds.filter((k) => k === "code")).toHaveLength(3);
    // Document order is non-decreasing.
    const keys = scan.matches.map(matchOrderKey);
    expect([...keys].sort((a, b) => a - b)).toEqual(keys);
    editor.destroy();
  });

  it("honors case sensitivity", () => {
    const editor = makeEditor(DOC);
    const scan = buildFindMatches(editor.state.doc, query("alpha", { caseSensitive: true }));
    // prose: alpha, alphabet; code: alpha, alpha; tail: alpha → 5 (not Alpha/ALPHA/Alphabet).
    expect(scan.matches).toHaveLength(5);
    editor.destroy();
  });

  it("honors whole-word matching across prose and code", () => {
    const editor = makeEditor(DOC);
    const scan = buildFindMatches(editor.state.doc, query("alpha", { wholeWord: true }));
    // Excludes "alphabet" (prose) and "Alphabet" (code): Alpha, alpha, ALPHA, code alpha ×2, tail alpha.
    expect(scan.matches).toHaveLength(6);
    for (const match of scan.matches) {
      expect(match.matchText.toLowerCase()).toBe("alpha");
    }
    editor.destroy();
  });

  it("supports regex with capture groups", () => {
    const editor = makeEditor(DOC);
    const scan = buildFindMatches(editor.state.doc, query("al(pha)(bet)?", { isRegex: true }));
    expect(scan.error).toBeNull();
    const withBet = scan.matches.filter((m) => m.groups[1] === "bet");
    expect(withBet.length).toBeGreaterThanOrEqual(1);
    editor.destroy();
  });

  it("returns a typed error for an invalid regex (no throw)", () => {
    const editor = makeEditor(DOC);
    const scan = buildFindMatches(editor.state.doc, query("(unclosed", { isRegex: true }));
    expect(scan.matches).toHaveLength(0);
    expect(scan.error).toMatch(/invalid regular expression/);
    editor.destroy();
  });

  it("guards zero-length regex matches (no infinite loop) and caps matches", () => {
    const editor = makeEditor({
      type: "doc",
      content: [{ type: "paragraph", content: [{ type: "text", text: "aaaa" }] }],
    });
    const zeroLength = buildFindMatches(editor.state.doc, query("x*", { isRegex: true }));
    expect(zeroLength.error).toBeNull(); // completed — did not hang
    expect(zeroLength.matches).toHaveLength(0);

    const longText = "ab ".repeat(MAX_MATCHES + 50);
    const editor2 = makeEditor({
      type: "doc",
      content: [{ type: "paragraph", content: [{ type: "text", text: longText }] }],
    });
    const capped = buildFindMatches(editor2.state.doc, query("ab"));
    expect(capped.matches.length).toBeLessThanOrEqual(MAX_MATCHES);
    expect(capped.truncated).toBe(true);
    editor.destroy();
    editor2.destroy();
  });

  it("rejects an over-long term with a typed error", () => {
    const compiled = compileFindQuery(query("x".repeat(2000)));
    expect("error" in compiled && compiled.error).toMatch(/longer than/);
  });
});

describe("replacement expansion (regex mode)", () => {
  it("expands $1..$9, $& and $$", () => {
    const match: ProseMatch = { kind: "prose", from: 1, to: 5, groups: ["pha", "bet"], matchText: "alphabet" };
    expect(expandReplacement("[$&]", match)).toBe("[alphabet]");
    expect(expandReplacement("$1-$2", match)).toBe("pha-bet");
    expect(expandReplacement("$$1", match)).toBe("$1");
    expect(expandReplacement("$9", match)).toBe("");
  });
});

describe("replace one / replace all (single transaction, undo integrity)", () => {
  it("replaces a prose match preserving the rest", () => {
    const editor = makeEditor(DOC);
    const scan = buildFindMatches(editor.state.doc, query("tail"));
    expect(scan.matches).toHaveLength(1);
    applyReplaceMatch(editor, scan.matches[0], "TAIL", false);
    expect(editor.state.doc.textBetween(0, editor.state.doc.content.size, "\n")).toContain("TAIL alpha");
    editor.destroy();
  });

  it("replaces inside the code block and recomputes the round-trip hash", () => {
    const editor = makeEditor(DOC);
    const scan = buildFindMatches(editor.state.doc, query("alpha", { caseSensitive: true }));
    const codeMatches = scan.matches.filter((m): m is CodeMatch => m.kind === "code");
    expect(codeMatches).toHaveLength(2);
    applyReplaceAll(editor, codeMatches, "omega", false);

    let codeAttrs: Record<string, unknown> | null = null;
    editor.state.doc.descendants((node) => {
      if (node.type.name === "monacoCodeBlock") codeAttrs = node.attrs as Record<string, unknown>;
      return true;
    });
    expect(codeAttrs).not.toBeNull();
    const attrs = codeAttrs as unknown as { language: string; code: string; contentHash: string };
    expect(attrs.code).toBe("const omega = 'omega';\nlet Alphabet = 1;");
    expect(attrs.contentHash).toBe(codeBlockHash(attrs.language, attrs.code));
    editor.destroy();
  });

  it("replace-all across prose + code is ONE undo step restoring the exact doc", () => {
    const editor = makeEditor(DOC);
    const before = editor.getJSON();
    const scan = buildFindMatches(editor.state.doc, query("alpha"));
    const outcome = applyReplaceAll(editor, scan.matches, "Ω", false);
    expect(outcome.replacedProse + outcome.replacedCode).toBe(8);

    const afterReplace = editor.getJSON();
    expect(JSON.stringify(afterReplace)).toContain("Ω");
    expect(JSON.stringify(afterReplace)).not.toMatch(/alpha/i);

    editor.commands.undo();
    expect(editor.getJSON()).toEqual(before);
    editor.destroy();
  });

  it("expands regex groups during replace-all", () => {
    const editor = makeEditor({
      type: "doc",
      content: [{ type: "paragraph", content: [{ type: "text", text: "call(1) call(2)" }] }],
    });
    const scan = buildFindMatches(editor.state.doc, query("call\\((\\d)\\)", { isRegex: true }));
    expect(scan.matches).toHaveLength(2);
    applyReplaceAll(editor, scan.matches, "invoke[$1]", true);
    expect(editor.getText()).toContain("invoke[1] invoke[2]");
    editor.destroy();
  });

  it("keeps the selection in-document after replace-all (no selection loss)", () => {
    const editor = makeEditor(DOC);
    editor.commands.setTextSelection(5);
    const scan = buildFindMatches(editor.state.doc, query("alpha"));
    applyReplaceAll(editor, scan.matches, "x", false);
    const { from, to } = editor.state.selection;
    expect(from).toBeGreaterThanOrEqual(0);
    expect(to).toBeLessThanOrEqual(editor.state.doc.content.size);
    editor.destroy();
  });
});

describe("navigation", () => {
  it("indexAfterSelection wraps to the first match", () => {
    const editor = makeEditor(DOC);
    const scan = buildFindMatches(editor.state.doc, query("alpha"));
    expect(indexAfterSelection(scan.matches, 0)).toBe(0);
    const lastKey = matchOrderKey(scan.matches[scan.matches.length - 1]);
    expect(indexAfterSelection(scan.matches, lastKey + 10)).toBe(0);
    expect(indexAfterSelection([], 0)).toBe(-1);
    editor.destroy();
  });

  it("selectMatch sets a text selection for prose and a node selection for code", () => {
    const editor = makeEditor(DOC);
    const scan = buildFindMatches(editor.state.doc, query("alpha"));
    const prose = scan.matches.find((m): m is ProseMatch => m.kind === "prose");
    const code = scan.matches.find((m): m is CodeMatch => m.kind === "code");
    selectMatch(editor, prose as ProseMatch);
    expect(editor.state.selection.from).toBe((prose as ProseMatch).from);
    expect(editor.state.selection.to).toBe((prose as ProseMatch).to);
    selectMatch(editor, code as CodeMatch);
    expect(editor.state.selection.from).toBe((code as CodeMatch).nodePos);
    editor.destroy();
  });
});
