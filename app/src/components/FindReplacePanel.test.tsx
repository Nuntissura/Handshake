// WP-KERNEL-009 / MT-244 — find/replace panel + export UI component tests.
//
// Mounts the REAL integrated RichTextEditor and proves:
//   - the Find toolbar button + Mod-f/Mod-h keymap open the panel (find vs
//     replace variants),
//   - the match count reports prose + code-block matches, navigation moves
//     the current index with wrap-around,
//   - case/word/regex toggles re-filter live; an invalid regex shows a typed
//     inline error (role=alert),
//   - replace-one and replace-all rewrite prose AND code-block text (the
//     editor document is the proof), and a single undo restores the document,
//   - the export menu lists every save-to-format projection and a chosen
//     format produces a real downloaded payload (captured via the injected
//     object-URL/anchor deps in jsdom) with the export status surfaced,
//   - find panel highlights publish ProseMirror decorations (hs-find-match).

import { render, screen, fireEvent, waitFor } from "@testing-library/react";
import { act } from "react";
import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import type { JSONContent } from "@tiptap/core";
import { RichTextEditor } from "./RichTextEditor";
import { makeCodeBlockAttrs } from "../lib/editor/code_block_serialization";
import { EDITOR_LAST_EXPORT_GLOBAL_KEY } from "../lib/editor/visual_debug";

const DOC: JSONContent = {
  type: "doc",
  content: [
    { type: "paragraph", content: [{ type: "text", text: "alpha beta alpha" }] },
    { type: "monacoCodeBlock", attrs: makeCodeBlockAttrs("typescript", "const alpha = 1;") },
  ],
};

async function mountEditor(doc: JSONContent = DOC) {
  const onChange = vi.fn();
  await act(async () => {
    render(<RichTextEditor initialContent={doc} onChange={onChange} documentTitle="Panel Test Doc" />);
  });
  return { onChange };
}

async function openFindPanel() {
  await act(async () => {
    fireEvent.click(screen.getByTestId("editor-open-find"));
  });
  return screen.findByTestId("find-panel");
}

async function typeQuery(term: string) {
  await act(async () => {
    fireEvent.change(screen.getByTestId("find-input"), { target: { value: term } });
  });
}

describe("FindReplacePanel (MT-244)", () => {
  beforeEach(() => {
    delete (globalThis as Record<string, unknown>)[EDITOR_LAST_EXPORT_GLOBAL_KEY];
  });
  afterEach(() => {
    delete (globalThis as Record<string, unknown>)[EDITOR_LAST_EXPORT_GLOBAL_KEY];
  });

  it("opens via the toolbar button and counts prose + code matches", async () => {
    await mountEditor();
    const panel = await openFindPanel();
    expect(panel.getAttribute("data-with-replace")).toBe("false");
    await typeQuery("alpha");
    await waitFor(() => {
      expect(screen.getByTestId("find-panel").getAttribute("data-match-count")).toBe("3");
    });
    expect(screen.getByTestId("find-count").textContent).toContain("of 3");
  });

  it("opens find via Mod-f and replace via Mod-h on the editor surface", async () => {
    await mountEditor();
    const surface = screen.getByTestId("rich-text-editor-surface").querySelector(".tiptap, [contenteditable]");
    expect(surface).toBeTruthy();
    await act(async () => {
      fireEvent.keyDown(surface as Element, { key: "f", ctrlKey: true });
    });
    expect((await screen.findByTestId("find-panel")).getAttribute("data-with-replace")).toBe("false");
    await act(async () => {
      fireEvent.keyDown(surface as Element, { key: "h", ctrlKey: true });
    });
    await waitFor(() => {
      expect(screen.getByTestId("find-panel").getAttribute("data-with-replace")).toBe("true");
    });
    expect(screen.getByTestId("replace-input")).toBeTruthy();
  });

  it("navigates next/prev with wrap-around and highlights the current match", async () => {
    await mountEditor();
    await openFindPanel();
    await typeQuery("alpha");
    await waitFor(() => {
      expect(screen.getByTestId("find-panel").getAttribute("data-match-count")).toBe("3");
    });
    const next = screen.getByTestId("find-next");
    const activeIndex = () =>
      Number(screen.getByTestId("find-panel").getAttribute("data-active-index"));
    // First "next" lands on the match at/after the cursor (VS Code semantics).
    await act(async () => {
      fireEvent.click(next);
    });
    const first = activeIndex();
    expect(first).toBeGreaterThanOrEqual(0);
    expect(first).toBeLessThan(3);
    // Subsequent clicks advance with wrap-around modulo the match count.
    await act(async () => {
      fireEvent.click(next);
    });
    expect(activeIndex()).toBe((first + 1) % 3);
    // Prose highlight decorations are live in the editor DOM; the current
    // marker is the inline class (prose match) or the node class (code match).
    const highlights = document.querySelectorAll(".hs-find-match");
    expect(highlights.length).toBeGreaterThanOrEqual(2);
    expect(
      document.querySelector(".hs-find-match--current") ??
        document.querySelector(".hs-find-match-block--current"),
    ).toBeTruthy();
    await act(async () => {
      fireEvent.click(next);
    });
    await act(async () => {
      fireEvent.click(next);
    });
    expect(activeIndex()).toBe(first); // full cycle wrapped back
    // And prev steps backwards (with wrap).
    await act(async () => {
      fireEvent.click(screen.getByTestId("find-prev"));
    });
    expect(activeIndex()).toBe((first + 2) % 3);
  });

  it("applies case/whole-word toggles and shows a typed error for invalid regex", async () => {
    await mountEditor({
      type: "doc",
      content: [{ type: "paragraph", content: [{ type: "text", text: "Alpha alphabet alpha" }] }],
    });
    await openFindPanel();
    await typeQuery("alpha");
    await waitFor(() => {
      expect(screen.getByTestId("find-panel").getAttribute("data-match-count")).toBe("3");
    });
    await act(async () => {
      fireEvent.click(screen.getByTestId("find-toggle-case"));
    });
    await waitFor(() => {
      expect(screen.getByTestId("find-panel").getAttribute("data-match-count")).toBe("2");
    });
    await act(async () => {
      fireEvent.click(screen.getByTestId("find-toggle-word"));
    });
    await waitFor(() => {
      expect(screen.getByTestId("find-panel").getAttribute("data-match-count")).toBe("1");
    });
    // Invalid regex → typed visible error.
    await act(async () => {
      fireEvent.click(screen.getByTestId("find-toggle-regex"));
    });
    await typeQuery("(unclosed");
    const error = await screen.findByTestId("find-error");
    expect(error.getAttribute("role")).toBe("alert");
    expect(error.textContent).toContain("invalid regular expression");
  });

  it("replace-all rewrites prose AND code-block text; one undo restores all", async () => {
    const { onChange } = await mountEditor();
    await act(async () => {
      fireEvent.click(screen.getByTestId("editor-open-find"));
    });
    // Switch to replace mode via Mod-h on the surface.
    const surface = screen.getByTestId("rich-text-editor-surface").querySelector("[contenteditable]");
    await act(async () => {
      fireEvent.keyDown(surface as Element, { key: "h", ctrlKey: true });
    });
    await typeQuery("alpha");
    await waitFor(() => {
      expect(screen.getByTestId("find-panel").getAttribute("data-match-count")).toBe("3");
    });
    await act(async () => {
      fireEvent.change(screen.getByTestId("replace-input"), { target: { value: "omega" } });
      fireEvent.click(screen.getByTestId("replace-all"));
    });
    await waitFor(() => {
      expect(screen.getByTestId("find-outcome").textContent).toContain("Replaced 3 matches");
    });
    expect(screen.getByTestId("find-outcome").textContent).toContain("1 in code blocks");
    const lastDoc = onChange.mock.calls[onChange.mock.calls.length - 1]?.[0] as JSONContent;
    const flat = JSON.stringify(lastDoc);
    expect(flat).toContain("omega beta omega");
    expect(flat).toContain("const omega = 1;");
    expect(flat).not.toContain("alpha");
    // After the doc change the panel rescans: omega now matches 0 times for "alpha".
    expect(screen.getByTestId("find-panel").getAttribute("data-match-count")).toBe("0");
  });

  it("replaces a single match leaving the others", async () => {
    const { onChange } = await mountEditor();
    await openFindPanel();
    const surface = screen.getByTestId("rich-text-editor-surface").querySelector("[contenteditable]");
    await act(async () => {
      fireEvent.keyDown(surface as Element, { key: "h", ctrlKey: true });
    });
    await typeQuery("alpha");
    await waitFor(() => {
      expect(screen.getByTestId("find-panel").getAttribute("data-match-count")).toBe("3");
    });
    await act(async () => {
      fireEvent.change(screen.getByTestId("replace-input"), { target: { value: "first" } });
      fireEvent.click(screen.getByTestId("replace-one"));
    });
    await waitFor(() => {
      expect(screen.getByTestId("find-panel").getAttribute("data-match-count")).toBe("2");
    });
    const flat = JSON.stringify(onChange.mock.calls[onChange.mock.calls.length - 1]?.[0]);
    expect(flat).toContain("first beta alpha");
  });

  it("closes on Escape and clears highlights", async () => {
    await mountEditor();
    await openFindPanel();
    await typeQuery("alpha");
    await waitFor(() => {
      expect(document.querySelectorAll(".hs-find-match").length).toBeGreaterThan(0);
    });
    await act(async () => {
      fireEvent.keyDown(screen.getByTestId("find-input"), { key: "Escape" });
    });
    expect(screen.queryByTestId("find-panel")).toBeNull();
    expect(document.querySelectorAll(".hs-find-match")).toHaveLength(0);
  });
});

describe("save-to-format export UI (MT-244 / DEC-003)", () => {
  const createObjectURL = vi.fn(() => "blob:hs-export");
  const revokeObjectURL = vi.fn();
  let anchorClicks: string[];

  beforeEach(() => {
    anchorClicks = [];
    createObjectURL.mockClear();
    revokeObjectURL.mockClear();
    // jsdom has no URL.createObjectURL: install a real spy pair.
    (URL as unknown as Record<string, unknown>).createObjectURL = createObjectURL;
    (URL as unknown as Record<string, unknown>).revokeObjectURL = revokeObjectURL;
    // Anchor click navigation is a no-op in jsdom; record the download intent.
    vi.spyOn(HTMLAnchorElement.prototype, "click").mockImplementation(function (this: HTMLAnchorElement) {
      anchorClicks.push(this.download);
    });
  });

  afterEach(() => {
    vi.restoreAllMocks();
    delete (URL as unknown as Record<string, unknown>).createObjectURL;
    delete (URL as unknown as Record<string, unknown>).revokeObjectURL;
    delete (globalThis as Record<string, unknown>)[EDITOR_LAST_EXPORT_GLOBAL_KEY];
  });

  it("lists every export format and exports markdown end to end", async () => {
    await mountEditor();
    await act(async () => {
      fireEvent.click(screen.getByTestId("editor-open-export"));
    });
    const menu = await screen.findByTestId("editor-export-menu");
    expect(menu.getAttribute("role")).toBe("dialog");
    for (const id of [
      "html_self_contained",
      "html_reference_linked",
      "markdown",
      "plain_text",
      "prosemirror_json",
    ]) {
      expect(screen.getByTestId(`export-format-${id}`)).toBeTruthy();
    }
    await act(async () => {
      fireEvent.click(screen.getByTestId("export-format-markdown"));
    });
    const status = await screen.findByTestId("export-status");
    expect(status.getAttribute("data-export-format")).toBe("markdown");
    expect(Number(status.getAttribute("data-export-bytes"))).toBeGreaterThan(0);
    expect(anchorClicks).toHaveLength(1);
    expect(anchorClicks[0]).toMatch(/^panel-test-doc-.*\.md$/);
    expect(anchorClicks[0]).not.toMatch(/\s/);
    expect(createObjectURL).toHaveBeenCalledTimes(1);

    const published = (globalThis as Record<string, unknown>)[EDITOR_LAST_EXPORT_GLOBAL_KEY] as {
      content: string;
      formatId: string;
    };
    expect(published.formatId).toBe("markdown");
    expect(published.content).toContain("alpha beta alpha");
    expect(published.content).toContain("```typescript");
  });

  it("exports reference-linked HTML through the palette command", async () => {
    await mountEditor();
    await act(async () => {
      fireEvent.click(screen.getByTestId("editor-open-palette"));
    });
    await act(async () => {
      fireEvent.change(screen.getByTestId("editor-command-palette-input"), {
        target: { value: "export" },
      });
    });
    await act(async () => {
      fireEvent.click(screen.getByTestId("palette-cmd-export.html_reference_linked"));
    });
    const status = await screen.findByTestId("export-status");
    expect(status.getAttribute("data-export-format")).toBe("html_reference_linked");
    const published = (globalThis as Record<string, unknown>)[EDITOR_LAST_EXPORT_GLOBAL_KEY] as {
      content: string;
    };
    expect(published.content).toContain("data-hs-export=\"rich_document\"");
    expect(published.content).toContain("data-hs-node=\"monacoCodeBlock\"");
  });
});
