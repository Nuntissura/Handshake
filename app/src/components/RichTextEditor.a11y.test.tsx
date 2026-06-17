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
import { NodeSelection } from "@tiptap/pm/state";
import { registerCodeBlockFindHandle } from "../lib/editor/code_block_find_registry";

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
    // Valid combobox/listbox semantics. First option = first catalog command
    // (history.undo since iteration-3 L14).
    expect(input.getAttribute("role")).toBe("combobox");
    expect(input.getAttribute("aria-activedescendant")).toBe("palette-opt-history.undo");

    // ArrowDown moves the active option; aria-selected follows.
    await act(async () => {
      fireEvent.keyDown(input, { key: "ArrowDown" });
    });
    expect(input.getAttribute("aria-activedescendant")).toBe("palette-opt-history.redo");
    const redoOpt = document.getElementById("palette-opt-history.redo");
    expect(redoOpt?.getAttribute("aria-selected")).toBe("true");

    // ArrowUp wraps back; Enter runs the active command and closes the palette.
    await act(async () => {
      fireEvent.keyDown(input, { key: "ArrowUp" });
    });
    expect(input.getAttribute("aria-activedescendant")).toBe("palette-opt-history.undo");
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
    const allButtons = Array.from(toolbar.querySelectorAll("button"));
    const enabled = allButtons.filter((b) => !b.disabled);
    expect(enabled.length).toBeGreaterThan(3);
    // Exactly ONE button is in the tab order, and it is an ENABLED one (Undo
    // sits first but is truthfully disabled until the first edit — M11/L14).
    const tabStops = allButtons.filter((b) => b.tabIndex === 0);
    expect(tabStops).toHaveLength(1);
    expect(tabStops[0]).toBe(enabled[0]);
    expect(tabStops[0].disabled).toBe(false);

    // ArrowRight moves focus (and the roving tab stop) to the next ENABLED
    // control (disabled controls are skipped).
    await act(async () => {
      enabled[0].focus();
      fireEvent.keyDown(toolbar, { key: "ArrowRight" });
    });
    expect(document.activeElement).toBe(enabled[1]);
    await waitFor(() => {
      expect(enabled[1].tabIndex).toBe(0);
      expect(allButtons.filter((b) => b.tabIndex === 0)).toHaveLength(1);
    });
    // End jumps to the last enabled control; ArrowRight wraps to the first.
    await act(async () => {
      fireEvent.keyDown(toolbar, { key: "End" });
    });
    expect(document.activeElement).toBe(enabled[enabled.length - 1]);
    await act(async () => {
      fireEvent.keyDown(toolbar, { key: "ArrowRight" });
    });
    expect(document.activeElement).toBe(enabled[0]);
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
    // The palette lists AND runs the save command when a handler is wired.
    await act(async () => {
      fireEvent.click(screen.getByTestId("editor-open-palette"));
    });
    await act(async () => {
      fireEvent.change(screen.getByTestId("editor-command-palette-input"), { target: { value: "save doc" } });
    });
    await act(async () => {
      fireEvent.click(await screen.findByTestId("palette-cmd-editor.save"));
    });
    expect(onSaveRequested).toHaveBeenCalledTimes(4);
    await waitFor(() => {
      expect(screen.queryByTestId("editor-command-palette")).toBeNull();
    });
  });

  it("runs go-to-line from the palette against the focused code block and reports invalid lines (MT-245)", async () => {
    let editor: Editor | null = null;
    const reveals: Array<{ start: number; end: number }> = [];
    let codePos = -1;
    const unregister = registerCodeBlockFindHandle({
      getPos: () => codePos,
      reveal: (start, end) => reveals.push({ start, end }),
    });
    const withCode: JSONContent = {
      type: "doc",
      content: [
        { type: "paragraph", content: [{ type: "text", text: "before" }] },
        {
          type: "monacoCodeBlock",
          attrs: { language: "typescript", code: "one\ntwo words\nthree", contentHash: "" },
        },
      ],
    };
    await act(async () => {
      render(
        <RichTextEditor
          initialContent={withCode}
          onChange={() => {}}
          onEditorReady={(e) => {
            editor = e;
          }}
        />,
      );
    });
    await waitFor(() => expect(editor).toBeTruthy());
    editor!.state.doc.descendants((node, pos) => {
      if (node.type.name === "monacoCodeBlock") {
        codePos = pos;
        return false;
      }
      return true;
    });
    expect(codePos).toBeGreaterThanOrEqual(0);

    try {
      await act(async () => {
        editor!.view.dispatch(editor!.state.tr.setSelection(NodeSelection.create(editor!.state.doc, codePos)));
        fireEvent.click(screen.getByTestId("editor-open-palette"));
      });
      await act(async () => {
        fireEvent.change(screen.getByTestId("editor-command-palette-input"), { target: { value: "line" } });
        fireEvent.click(await screen.findByTestId("palette-cmd-navigate.gotoLine"));
      });
      await act(async () => {
        fireEvent.change(await screen.findByTestId("editor-arg-line"), { target: { value: "2" } });
        fireEvent.click(screen.getByTestId("editor-arg-confirm"));
      });
      expect(reveals).toEqual([{ start: 4, end: 13 }]);

      await act(async () => {
        fireEvent.click(screen.getByTestId("editor-open-palette"));
      });
      await act(async () => {
        fireEvent.change(screen.getByTestId("editor-command-palette-input"), { target: { value: "line" } });
        fireEvent.click(await screen.findByTestId("palette-cmd-navigate.gotoLine"));
      });
      await act(async () => {
        fireEvent.change(await screen.findByTestId("editor-arg-line"), { target: { value: "99" } });
        fireEvent.click(screen.getByTestId("editor-arg-confirm"));
      });
      const error = await screen.findByTestId("editor-go-to-line-error");
      expect(error.getAttribute("role")).toBe("alert");
      expect(error.textContent).toContain("99");
      expect(reveals).toHaveLength(1);
    } finally {
      unregister();
    }
  });

  it("does not reuse a stale code-block target after selection moves back to prose (MT-245)", async () => {
    let editor: Editor | null = null;
    let firstCodePos = -1;
    let secondCodePos = -1;
    const reveals: Array<{ pos: number; start: number; end: number }> = [];
    const unregisterFirst = registerCodeBlockFindHandle({
      getPos: () => firstCodePos,
      reveal: (start, end) => reveals.push({ pos: firstCodePos, start, end }),
    });
    const unregisterSecond = registerCodeBlockFindHandle({
      getPos: () => secondCodePos,
      reveal: (start, end) => reveals.push({ pos: secondCodePos, start, end }),
    });
    const withTwoBlocks: JSONContent = {
      type: "doc",
      content: [
        { type: "paragraph", content: [{ type: "text", text: "before" }] },
        { type: "monacoCodeBlock", attrs: { language: "typescript", code: "first\nblock", contentHash: "" } },
        { type: "paragraph", content: [{ type: "text", text: "middle prose" }] },
        { type: "monacoCodeBlock", attrs: { language: "typescript", code: "second\nblock", contentHash: "" } },
      ],
    };

    try {
      await act(async () => {
        render(
          <RichTextEditor
            initialContent={withTwoBlocks}
            onChange={() => {}}
            onEditorReady={(e) => {
              editor = e;
            }}
          />,
        );
      });
      await waitFor(() => expect(editor).toBeTruthy());
      let middleParagraphSelection = -1;
      editor!.state.doc.descendants((node, pos) => {
        if (node.type.name === "monacoCodeBlock") {
          if (firstCodePos < 0) firstCodePos = pos;
          else secondCodePos = pos;
        }
        if (node.type.name === "paragraph" && node.textContent === "middle prose") {
          middleParagraphSelection = pos + 1;
        }
        return true;
      });
      expect(firstCodePos).toBeGreaterThanOrEqual(0);
      expect(secondCodePos).toBeGreaterThanOrEqual(0);
      expect(middleParagraphSelection).toBeGreaterThanOrEqual(0);

      await act(async () => {
        editor!.view.dispatch(editor!.state.tr.setSelection(NodeSelection.create(editor!.state.doc, firstCodePos)));
      });
      await waitFor(() => {
        expect(screen.getByTestId("rich-text-editor-status-bar")).toHaveAttribute("data-code-language", "typescript");
      });

      await act(async () => {
        editor!.commands.setTextSelection(middleParagraphSelection);
      });
      await waitFor(() => {
        expect(screen.getByTestId("rich-text-editor-status-bar")).toHaveAttribute("data-code-language", "");
      });

      await act(async () => {
        fireEvent.click(screen.getByTestId("editor-open-palette"));
      });
      await act(async () => {
        fireEvent.change(screen.getByTestId("editor-command-palette-input"), { target: { value: "line" } });
        fireEvent.click(await screen.findByTestId("palette-cmd-navigate.gotoLine"));
      });
      await act(async () => {
        fireEvent.change(await screen.findByTestId("editor-arg-line"), { target: { value: "2" } });
        fireEvent.click(screen.getByTestId("editor-arg-confirm"));
      });

      const error = await screen.findByTestId("editor-go-to-line-error");
      expect(error.textContent).toContain("No focused code block");
      expect(reveals).toEqual([]);
    } finally {
      unregisterFirst();
      unregisterSecond();
    }
  });

  it("does not fall back to the only code block when prose has focus for Go to line (MT-245)", async () => {
    let editor: Editor | null = null;
    let codePos = -1;
    let proseSelection = -1;
    const reveals: Array<{ start: number; end: number }> = [];
    const unregister = registerCodeBlockFindHandle({
      getPos: () => codePos,
      reveal: (start, end) => reveals.push({ start, end }),
    });
    const withOneBlock: JSONContent = {
      type: "doc",
      content: [
        { type: "paragraph", content: [{ type: "text", text: "prose cursor stays here" }] },
        { type: "monacoCodeBlock", attrs: { language: "typescript", code: "first\nsecond", contentHash: "" } },
      ],
    };

    try {
      await act(async () => {
        render(
          <RichTextEditor
            initialContent={withOneBlock}
            onChange={() => {}}
            onEditorReady={(e) => {
              editor = e;
            }}
          />,
        );
      });
      await waitFor(() => expect(editor).toBeTruthy());
      editor!.state.doc.descendants((node, pos) => {
        if (node.type.name === "paragraph") proseSelection = pos + 1;
        if (node.type.name === "monacoCodeBlock") codePos = pos;
        return true;
      });
      expect(proseSelection).toBeGreaterThanOrEqual(0);
      expect(codePos).toBeGreaterThanOrEqual(0);

      await act(async () => {
        editor!.commands.setTextSelection(proseSelection);
      });
      await act(async () => {
        fireEvent.click(screen.getByTestId("editor-open-palette"));
      });
      await act(async () => {
        fireEvent.change(screen.getByTestId("editor-command-palette-input"), { target: { value: "line" } });
        fireEvent.click(await screen.findByTestId("palette-cmd-navigate.gotoLine"));
      });
      await act(async () => {
        fireEvent.change(await screen.findByTestId("editor-arg-line"), { target: { value: "2" } });
        fireEvent.click(screen.getByTestId("editor-arg-confirm"));
      });

      const error = await screen.findByTestId("editor-go-to-line-error");
      expect(error.textContent).toContain("No focused code block");
      expect(reveals).toEqual([]);
    } finally {
      unregister();
    }
  });

  it("uses Monaco focus and cursor events for status language and line column (MT-245)", async () => {
    let editor: Editor | null = null;
    let codePos = -1;
    const withCode: JSONContent = {
      type: "doc",
      content: [
        { type: "paragraph", content: [{ type: "text", text: "before" }] },
        { type: "monacoCodeBlock", attrs: { language: "typescript", code: "one\ntwo", contentHash: "" } },
      ],
    };

    await act(async () => {
      render(
        <RichTextEditor
          initialContent={withCode}
          onChange={() => {}}
          onEditorReady={(e) => {
            editor = e;
          }}
        />,
      );
    });
    await waitFor(() => expect(editor).toBeTruthy());
    editor!.state.doc.descendants((node, pos) => {
      if (node.type.name === "monacoCodeBlock") codePos = pos;
      return codePos < 0;
    });
    expect(codePos).toBeGreaterThanOrEqual(0);

    await act(async () => {
      fireEvent(
        screen.getByTestId("monaco-code-block"),
        new CustomEvent("handshake:monaco-code-block-status", {
          bubbles: true,
          detail: { focused: true, pos: codePos, language: "typescript", line: 2, column: 4 },
        }),
      );
    });

    await waitFor(() => {
      expect(screen.getByTestId("rich-text-editor-status-bar")).toHaveAttribute("data-code-language", "typescript");
      expect(screen.getByTestId("rich-text-editor-status-bar")).toHaveAttribute("data-cursor-line", "2");
      expect(screen.getByTestId("rich-text-editor-status-bar")).toHaveAttribute("data-cursor-column", "4");
      expect(screen.getByTestId("rich-text-editor-status-cursor")).toHaveTextContent("Ln 2, Col 4");
      expect(screen.getByTestId("rich-text-editor-status-language")).toHaveTextContent("typescript");
    });

    await act(async () => {
      fireEvent(
        screen.getByTestId("monaco-code-block"),
        new CustomEvent("handshake:monaco-code-block-status", {
          bubbles: true,
          detail: { focused: false, pos: codePos, language: "typescript", line: 2, column: 4 },
        }),
      );
    });

    await waitFor(() => {
      expect(screen.getByTestId("rich-text-editor-status-bar")).toHaveAttribute("data-code-language", "");
      expect(screen.getByTestId("rich-text-editor-status-language")).toHaveTextContent("Prose");
    });
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
