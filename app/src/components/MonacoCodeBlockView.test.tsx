// WP-KERNEL-009 / MT-165 — MonacoEmbeddedCodeBlock NodeView (React mount).
//
// Mounts a REAL @tiptap/react editor whose document contains a monacoCodeBlock,
// so the React NodeView (MonacoCodeBlockView) actually renders. Monaco's full
// editor needs real browser layout, so under jsdom the NodeView either renders
// the Monaco host or degrades to the editable textarea fallback — EITHER WAY the
// stable wrapper + language selector + hash selectors must render (never a blank
// surface), and the code/language attrs must be present. The live Monaco mount +
// offline-worker proof is the MT-176 Playwright spec.

import { render, screen, waitFor } from "@testing-library/react";
import { act } from "react";
import { describe, it, expect } from "vitest";
import { EditorContent, useEditor } from "@tiptap/react";
import StarterKit from "@tiptap/starter-kit";
import { MonacoCodeBlockNode } from "../lib/tiptap/monaco_code_block_node";
import { makeCodeBlockAttrs } from "../lib/editor/code_block_serialization";

function Harness({ language, code }: { language: string; code: string }) {
  const attrs = makeCodeBlockAttrs(language, code);
  const editor = useEditor({
    extensions: [StarterKit.configure({ heading: { levels: [1, 2, 3] } }), MonacoCodeBlockNode],
    content: {
      type: "doc",
      content: [{ type: "monacoCodeBlock", attrs }],
    },
  });
  if (!editor) return null;
  return <EditorContent editor={editor} />;
}

describe("MonacoCodeBlockView (MT-165 React NodeView)", () => {
  it("renders the stable code-block wrapper with language + hash selectors", async () => {
    await act(async () => {
      render(<Harness language="ts" code={"const x = 1;"} />);
    });

    const block = await screen.findByTestId("monaco-code-block");
    // Normalized language id is reflected on the stable selector.
    expect(block.getAttribute("data-language")).toBe("typescript");
    // Round-trip hash is present (non-empty).
    expect(block.getAttribute("data-rt-hash")?.length).toBeGreaterThan(0);

    // The language picker renders over the curated registry.
    const picker = await screen.findByTestId("monaco-code-block-language");
    expect(picker).toBeTruthy();
    expect((picker as HTMLSelectElement).value).toBe("typescript");

    // Either Monaco mounted OR the degraded fallback is present — never blank.
    await waitFor(() => {
      const mounted = block.getAttribute("data-monaco-mounted") === "true";
      const degraded = block.getAttribute("data-degraded") === "true";
      expect(mounted || degraded).toBe(true);
    });
  });

  it("exposes the editable code in the fallback when Monaco cannot mount", async () => {
    await act(async () => {
      render(<Harness language="rust" code={"fn main() {}"} />);
    });
    const block = await screen.findByTestId("monaco-code-block");
    await waitFor(() => {
      expect(block.getAttribute("data-degraded") === "true" || block.getAttribute("data-monaco-mounted") === "true").toBe(true);
    });
    // When degraded, the fallback textarea must carry the code (no data loss).
    if (block.getAttribute("data-degraded") === "true") {
      const fallback = screen.getByTestId("monaco-code-block-fallback") as HTMLTextAreaElement;
      expect(fallback.value).toBe("fn main() {}");
    }
  });
});
