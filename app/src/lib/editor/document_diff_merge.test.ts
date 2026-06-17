import { describe, expect, it } from "vitest";
import { makeCodeBlockAttrs } from "./code_block_serialization";
import {
  applyRichDocumentMergePlan,
  buildRichDocumentDiff,
  planRichDocumentMerge,
} from "./document_diff_merge";

function doc(content: unknown[]) {
  return { type: "doc", content };
}

function paragraph(text: string) {
  return { type: "paragraph", content: [{ type: "text", text }] };
}

function codeBlock(language: string, code: string) {
  return { type: "monacoCodeBlock", attrs: makeCodeBlockAttrs(language, code) };
}

describe("document diff and merge model (MT-247)", () => {
  it("builds block-level prose diffs and Monaco-ready code diff pairs", () => {
    const before = doc([
      paragraph("Intro"),
      codeBlock("ts", "const count = 1;"),
      paragraph("Keep me"),
    ]);
    const after = doc([
      paragraph("Intro changed"),
      codeBlock("typescript", "const count = 2;"),
      paragraph("Keep me"),
      paragraph("New tail"),
    ]);

    const diff = buildRichDocumentDiff({ left: before, right: after });

    expect(diff.blocks.map((block) => block.status)).toEqual([
      "modified",
      "modified",
      "unchanged",
      "added",
    ]);
    expect(diff.blocks[0]).toMatchObject({
      kind: "prose",
      leftText: "Intro",
      rightText: "Intro changed",
    });
    expect(diff.blocks[1]).toMatchObject({
      kind: "code",
      leftCode: { language: "typescript", code: "const count = 1;" },
      rightCode: { language: "typescript", code: "const count = 2;" },
    });
  });

  it("aligns unchanged blocks around an inserted middle block", () => {
    const before = doc([paragraph("Alpha"), paragraph("Charlie")]);
    const after = doc([paragraph("Alpha"), paragraph("Bravo"), paragraph("Charlie")]);

    const diff = buildRichDocumentDiff({ left: before, right: after });

    expect(diff.blocks.map((block) => [block.status, block.leftText, block.rightText])).toEqual([
      ["unchanged", "Alpha", "Alpha"],
      ["added", undefined, "Bravo"],
      ["unchanged", "Charlie", "Charlie"],
    ]);
  });

  it("requires explicit conflict choices and preserves unrelated local edits", () => {
    const base = doc([paragraph("Heading"), paragraph("Stable body")]);
    const local = doc([paragraph("Heading from local"), paragraph("Local unsaved body")]);
    const remote = doc([paragraph("Heading from remote"), paragraph("Stable body")]);

    const plan = planRichDocumentMerge({ base, local, remote });

    expect(plan.blocks).toMatchObject([
      {
        blockIndex: 0,
        status: "conflict",
        localText: "Heading from local",
        remoteText: "Heading from remote",
      },
      {
        blockIndex: 1,
        status: "local_only",
        localText: "Local unsaved body",
      },
    ]);
    expect(() => applyRichDocumentMergePlan(plan, {})).toThrow(/unresolved conflict at block 0/i);

    const merged = applyRichDocumentMergePlan(plan, { 0: "remote" });

    expect(merged).toEqual(doc([paragraph("Heading from remote"), paragraph("Local unsaved body")]));
  });
});
