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
import { describe, it, expect, vi } from "vitest";
import { RichTextEditor } from "./RichTextEditor";
import type { Editor, JSONContent } from "@tiptap/core";

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

describe("RichTextEditor keyboard accessibility (iteration-3 M12/M13/M16/M6/L16)", () => {
  it("navigates the palette with ArrowDown/Up and runs the active option with Enter (M12)", async () => {
    await act(async () => {
      render(<RichTextEditor initialContent={INITIAL} onChange={() => {}} />);
    });
    await act(async () => {
      fireEvent.click(screen.getByTestId("editor-open-palette"));
    });
    const input = await screen.findByTestId("editor-command-palette-input");
    // Valid combobox/listbox semantics.
    expect(input.getAttribute("role")).toBe("combobox");
    expect(input.getAttribute("aria-activedescendant")).toBe("palette-opt-format.bold");

    // ArrowDown moves the active option; aria-selected follows.
    await act(async () => {
      fireEvent.keyDown(input, { key: "ArrowDown" });
    });
    expect(input.getAttribute("aria-activedescendant")).toBe("palette-opt-format.italic");
    const italicOpt = document.getElementById("palette-opt-format.italic");
    expect(italicOpt?.getAttribute("aria-selected")).toBe("true");

    // ArrowUp wraps back; Enter runs the active command and closes the palette.
    await act(async () => {
      fireEvent.keyDown(input, { key: "ArrowUp" });
    });
    expect(input.getAttribute("aria-activedescendant")).toBe("palette-opt-format.bold");
    await act(async () => {
      fireEvent.keyDown(input, { key: "Enter" });
    });
    await waitFor(() => {
      expect(screen.queryByTestId("editor-command-palette")).toBeNull();
    });
  });

  it("restores focus to the opener when the palette closes (M13)", async () => {
    await act(async () => {
      render(<RichTextEditor initialContent={INITIAL} onChange={() => {}} />);
    });
    const opener = screen.getByTestId("editor-open-palette") as HTMLButtonElement;
    await act(async () => {
      opener.focus();
      fireEvent.click(opener);
    });
    const input = await screen.findByTestId("editor-command-palette-input");
    await waitFor(() => expect(document.activeElement).toBe(input));
    await act(async () => {
      fireEvent.keyDown(input, { key: "Escape" });
    });
    await waitFor(() => {
      expect(screen.queryByTestId("editor-command-palette")).toBeNull();
      expect(document.activeElement).toBe(opener);
    });
  });

  it("traps Tab inside the arg-prompt dialog and closes it with Escape (M13)", async () => {
    await act(async () => {
      render(<RichTextEditor initialContent={INITIAL} onChange={() => {}} />);
    });
    await act(async () => {
      fireEvent.click(screen.getByTestId("editor-cmd-link.wikilink"));
    });
    const dialog = await screen.findByTestId("editor-arg-prompt");
    // First field receives focus on open.
    await waitFor(() => {
      expect(document.activeElement).toBe(screen.getByTestId("editor-arg-kind"));
    });
    // Tab from the LAST focusable wraps to the first.
    const cancel = screen.getByTestId("editor-arg-cancel") as HTMLButtonElement;
    await act(async () => {
      cancel.focus();
      fireEvent.keyDown(dialog, { key: "Tab" });
    });
    expect(document.activeElement).toBe(screen.getByTestId("editor-arg-kind"));
    // Escape closes.
    await act(async () => {
      fireEvent.keyDown(dialog, { key: "Escape" });
    });
    expect(screen.queryByTestId("editor-arg-prompt")).toBeNull();
  });

  it("implements the ARIA toolbar pattern: one tab stop + arrow-key roving (M16)", async () => {
    await act(async () => {
      render(<RichTextEditor initialContent={INITIAL} onChange={() => {}} />);
    });
    const toolbar = await screen.findByTestId("rich-text-editor-toolbar");
    const buttons = Array.from(toolbar.querySelectorAll("button"));
    expect(buttons.length).toBeGreaterThan(3);
    // Exactly ONE button is in the tab order.
    expect(buttons.filter((b) => b.tabIndex === 0)).toHaveLength(1);
    expect(buttons[0].tabIndex).toBe(0);

    // ArrowRight moves focus (and the roving tab stop) to the next control.
    await act(async () => {
      buttons[0].focus();
      fireEvent.keyDown(toolbar, { key: "ArrowRight" });
    });
    expect(document.activeElement).toBe(buttons[1]);
    await waitFor(() => {
      expect(buttons[1].tabIndex).toBe(0);
      expect(buttons.filter((b) => b.tabIndex === 0)).toHaveLength(1);
    });
    // End jumps to the last control; ArrowRight wraps to the first.
    await act(async () => {
      fireEvent.keyDown(toolbar, { key: "End" });
    });
    expect(document.activeElement).toBe(buttons[buttons.length - 1]);
    await act(async () => {
      fireEvent.keyDown(toolbar, { key: "ArrowRight" });
    });
    expect(document.activeElement).toBe(buttons[0]);
  });

  it("fires onSaveRequested for Mod-s from prose AND from inside the code block, always preventDefault (L16)", async () => {
    const onSaveRequested = vi.fn();
    let editor: Editor | null = null;
    const withCode: JSONContent = {
      type: "doc",
      content: [
        { type: "paragraph", content: [{ type: "text", text: "prose" }] },
        { type: "monacoCodeBlock", attrs: { language: "typescript", code: "x", contentHash: "" } },
      ],
    };
    await act(async () => {
      render(
        <RichTextEditor
          initialContent={withCode}
          onChange={() => {}}
          onSaveRequested={onSaveRequested}
          onEditorReady={(e) => (editor = e)}
        />,
      );
    });
    await waitFor(() => expect(editor).toBeTruthy());
    const prose = screen
      .getByTestId("rich-text-editor-surface")
      .querySelector(".ProseMirror")!;
    await act(async () => {
      fireEvent.keyDown(prose, { key: "s", ctrlKey: true, bubbles: true });
    });
    expect(onSaveRequested).toHaveBeenCalledTimes(1);

    // Global escape hatch: Mod-s typed INSIDE the embedded code block saves too.
    const block = await screen.findByTestId("monaco-code-block");
    await act(async () => {
      fireEvent.keyDown(block, { key: "s", ctrlKey: true, bubbles: true });
    });
    expect(onSaveRequested).toHaveBeenCalledTimes(2);

    // The browser save dialog is always suppressed.
    const event = new KeyboardEvent("keydown", { key: "s", ctrlKey: true, bubbles: true, cancelable: true });
    await act(async () => {
      prose.dispatchEvent(event);
    });
    expect(event.defaultPrevented).toBe(true);
    // The palette lists the save command when a handler is wired.
    await act(async () => {
      fireEvent.click(screen.getByTestId("editor-open-palette"));
    });
    await act(async () => {
      fireEvent.change(screen.getByTestId("editor-command-palette-input"), { target: { value: "save doc" } });
    });
    expect(await screen.findByTestId("palette-cmd-editor.save")).toBeTruthy();
  });

  it("exits the embedded code block to the prose document with Escape (M6/M17, degraded path)", async () => {
    let editor: Editor | null = null;
    const withCode: JSONContent = {
      type: "doc",
      content: [
        { type: "paragraph", content: [{ type: "text", text: "before" }] },
        { type: "monacoCodeBlock", attrs: { language: "typescript", code: "x", contentHash: "" } },
        { type: "paragraph", content: [{ type: "text", text: "after" }] },
      ],
    };
    await act(async () => {
      render(
        <RichTextEditor
          initialContent={withCode}
          onChange={() => {}}
          onEditorReady={(e) => (editor = e)}
        />,
      );
    });
    await waitFor(() => expect(editor).toBeTruthy());
    const block = await screen.findByTestId("monaco-code-block");
    // jsdom cannot mount Monaco -> the degraded textarea renders; it carries
    // the same Escape exit contract as the Monaco keybinding.
    await waitFor(() => expect(block.getAttribute("data-degraded")).toBe("true"));
    expect(block.getAttribute("data-keyboard-exit")).toBe("escape");

    let blockPos = -1;
    editor!.state.doc.descendants((node, pos) => {
      if (node.type.name === "monacoCodeBlock") {
        blockPos = pos;
        return false;
      }
      return true;
    });
    const fallback = screen.getByTestId("monaco-code-block-fallback") as HTMLTextAreaElement;
    await act(async () => {
      fallback.focus();
      fireEvent.keyDown(fallback, { key: "Escape" });
    });
    // Caret lands right AFTER the block, in the prose document. (True FOCUS
    // movement needs a real browser — proven in the Playwright typing lane;
    // jsdom asserts the selection contract the focus call rides on.)
    await waitFor(() => {
      expect(editor!.state.selection.from).toBe(blockPos + 1);
    });
  });
});
