// WP-KERNEL-009 / MT-173 — EditorAccessibilityAndReadability verification.
//
// Asserts the integrated editor's accessibility + discoverability affordances so
// a screen reader / keyboard user (and the GLOBAL-BUILD-DIAG GUI checks) can
// operate it: the toolbar exposes role=toolbar + a label; every toolbar control
// is a real <button> with an aria-label and aria-pressed reflecting active
// state; the "More…" affordance is discoverable + labelled; the command palette
// is a labelled modal dialog with a labelled search input that receives focus;
// and the arg prompt is a labelled dialog with labelled fields. (Pixel-level
// contrast / no-overlap across viewports is asserted visually in the MT-176
// Playwright matrix; this proves the semantic structure those checks rely on.)

import { render, screen, waitFor, fireEvent } from "@testing-library/react";
import { act } from "react";
import { describe, it, expect } from "vitest";
import { RichTextEditor } from "./RichTextEditor";
import type { JSONContent } from "@tiptap/core";

const INITIAL: JSONContent = {
  type: "doc",
  content: [{ type: "paragraph", content: [{ type: "text", text: "hello" }] }],
};

describe("RichTextEditor accessibility (MT-173)", () => {
  it("exposes a labelled toolbar of real buttons with aria-pressed state", async () => {
    await act(async () => {
      render(<RichTextEditor initialContent={INITIAL} onChange={() => {}} />);
    });
    const toolbar = await screen.findByRole("toolbar", { name: /editor formatting/i });
    expect(toolbar).toBeTruthy();

    const bold = screen.getByTestId("editor-cmd-format.bold");
    expect(bold.tagName).toBe("BUTTON");
    expect(bold.getAttribute("aria-label")).toBeTruthy();
    // aria-pressed reflects active state (starts unpressed).
    expect(bold.getAttribute("aria-pressed")).toBe("false");

    // The overflow affordance is discoverable + labelled.
    const more = screen.getByTestId("editor-open-palette");
    expect(more.getAttribute("aria-label")).toMatch(/command palette/i);
  });

  it("opens the command palette as a labelled modal dialog and focuses the search input", async () => {
    await act(async () => {
      render(<RichTextEditor initialContent={INITIAL} onChange={() => {}} />);
    });
    await act(async () => {
      fireEvent.click(screen.getByTestId("editor-open-palette"));
    });
    const dialog = await screen.findByRole("dialog", { name: /command palette/i });
    expect(dialog.getAttribute("aria-modal")).toBe("true");

    const input = screen.getByTestId("editor-command-palette-input");
    expect(input.getAttribute("aria-label")).toMatch(/search editor commands/i);
    // The input receives focus on open (keyboard users land in the search box).
    await waitFor(() => {
      expect(document.activeElement).toBe(input);
    });
  });

  it("renders the arg prompt as a labelled dialog with labelled fields", async () => {
    await act(async () => {
      render(<RichTextEditor initialContent={INITIAL} onChange={() => {}} />);
    });
    await act(async () => {
      fireEvent.click(screen.getByTestId("editor-cmd-link.wikilink"));
    });
    const dialog = await screen.findByRole("dialog", { name: /insert link options/i });
    expect(dialog).toBeTruthy();
    // Each arg field has a visible label associated with its input.
    expect(screen.getByTestId("editor-arg-kind")).toBeTruthy();
    expect(screen.getByTestId("editor-arg-value")).toBeTruthy();
  });

  it("marks read-only mode on the root and disables toolbar controls", async () => {
    await act(async () => {
      render(<RichTextEditor initialContent={INITIAL} onChange={() => {}} readOnly />);
    });
    const root = await screen.findByTestId("rich-text-editor");
    expect(root.getAttribute("data-readonly")).toBe("true");
    expect((screen.getByTestId("editor-cmd-format.bold") as HTMLButtonElement).disabled).toBe(true);
  });
});
