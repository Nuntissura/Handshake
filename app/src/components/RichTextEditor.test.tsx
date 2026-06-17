// WP-KERNEL-009 / MT-169..174 — integrated RichTextEditor component tests.
//
// Mounts the REAL integrated editor and proves: the toolbar runs commands and
// reflects active state (MT-169); the command palette opens, filters, and runs
// (MT-170); the arg prompt collects a typed-link target (MT-169/170); the
// visual-debug selectors + counts update (MT-172); the toolbar exposes
// role/aria for accessibility (MT-173); a typed backend error renders inline,
// never blanking the surface (MT-174); and a fatal extension failure degrades to
// a non-blank notice (MT-174).

import { render, screen, waitFor, fireEvent } from "@testing-library/react";
import { act, type ComponentProps } from "react";
import { describe, it, expect, vi } from "vitest";
import { RichTextEditor } from "./RichTextEditor";
import type { JSONContent } from "@tiptap/core";
import type { Editor } from "@tiptap/core";
import { NodeSelection } from "@tiptap/pm/state";

function renderEditor(props?: Partial<ComponentProps<typeof RichTextEditor>>) {
  const onChange = vi.fn();
  const initial: JSONContent = { type: "doc", content: [{ type: "paragraph", content: [{ type: "text", text: "hello" }] }] };
  return {
    onChange,
    ...render(<RichTextEditor initialContent={initial} onChange={onChange} {...props} />),
  };
}

describe("RichTextEditor (MT-169..174)", () => {
  it("renders the toolbar with role + aria and runs a formatting command (MT-169/173)", async () => {
    await act(async () => {
      renderEditor();
    });
    const toolbar = await screen.findByTestId("rich-text-editor-toolbar");
    expect(toolbar.getAttribute("role")).toBe("toolbar");
    expect(toolbar.getAttribute("aria-label")).toBeTruthy();

    const bold = await screen.findByTestId("editor-cmd-format.bold");
    expect(bold.getAttribute("aria-pressed")).toBe("false");
    // Select all then bold.
    await act(async () => {
      fireEvent.click(screen.getByTestId("editor-cmd-block.h1"));
    });
    // h1 toggles active.
    await waitFor(() => {
      expect(screen.getByTestId("editor-cmd-block.h1").getAttribute("data-active")).toBe("true");
    });
  });

  it("inserts an embedded code block from the toolbar via the arg prompt (MT-169)", async () => {
    await act(async () => {
      renderEditor();
    });
    await act(async () => {
      fireEvent.click(screen.getByTestId("editor-cmd-code.insert"));
    });
    // Arg prompt appears asking for the language.
    const prompt = await screen.findByTestId("editor-arg-prompt");
    expect(prompt).toBeTruthy();
    const langInput = screen.getByTestId("editor-arg-language");
    await act(async () => {
      fireEvent.change(langInput, { target: { value: "rust" } });
      fireEvent.click(screen.getByTestId("editor-arg-confirm"));
    });
    // The document now has a code block → debug counter reflects it (MT-172).
    await waitFor(() => {
      expect(screen.getByTestId("rich-text-editor").getAttribute("data-code-block-count")).toBe("1");
    });
  });

  it("opens the command palette, filters, and runs a command (MT-170)", async () => {
    await act(async () => {
      renderEditor();
    });
    await act(async () => {
      fireEvent.click(screen.getByTestId("editor-open-palette"));
    });
    const palette = await screen.findByTestId("editor-command-palette");
    expect(palette).toBeTruthy();
    const input = screen.getByTestId("editor-command-palette-input");
    await act(async () => {
      fireEvent.change(input, { target: { value: "bullet" } });
    });
    // Filtered to the bullet-list command.
    await waitFor(() => {
      expect(screen.getByTestId("palette-cmd-list.bullet")).toBeTruthy();
    });
    await act(async () => {
      fireEvent.click(screen.getByTestId("palette-cmd-list.bullet"));
    });
    // Palette closes after running.
    await waitFor(() => {
      expect(screen.queryByTestId("editor-command-palette")).toBeNull();
    });
  });

  it("inserts a typed wikilink through the palette arg prompt (MT-170/163)", async () => {
    await act(async () => {
      renderEditor();
    });
    await act(async () => {
      fireEvent.click(screen.getByTestId("editor-open-palette"));
    });
    await act(async () => {
      fireEvent.change(screen.getByTestId("editor-command-palette-input"), { target: { value: "link" } });
    });
    await act(async () => {
      fireEvent.click(await screen.findByTestId("palette-cmd-link.wikilink"));
    });
    // Arg prompt for the link kind/value.
    await act(async () => {
      fireEvent.change(screen.getByTestId("editor-arg-kind"), { target: { value: "wp" } });
      fireEvent.change(screen.getByTestId("editor-arg-value"), { target: { value: "WP-KERNEL-009" } });
      fireEvent.click(screen.getByTestId("editor-arg-confirm"));
    });
    await waitFor(() => {
      expect(screen.getByTestId("rich-text-editor").getAttribute("data-link-count")).toBe("1");
    });
  });

  it("renders a typed backend error inline without blanking (MT-174)", async () => {
    await act(async () => {
      renderEditor({
        backendError: { kind: "conflict", message: "Version 3 expected, got 4.", hint: "Reload to merge." },
      });
    });
    const err = await screen.findByTestId("rich-text-editor-backend-error");
    expect(err.getAttribute("data-error-kind")).toBe("conflict");
    expect(err.getAttribute("role")).toBe("alert");
    expect(err.textContent).toContain("Version 3 expected");
    // The editor surface is still present (not blank).
    expect(screen.getByTestId("rich-text-editor-surface")).toBeTruthy();
  });

  it("opens a REAL overflow menu listing insert + table commands and runs one (iteration-3 L13/M1)", async () => {
    await act(async () => {
      renderEditor();
    });
    await act(async () => {
      fireEvent.click(screen.getByTestId("editor-open-overflow"));
    });
    const menu = await screen.findByTestId("rich-text-editor-overflow");
    expect(menu.getAttribute("role")).toBe("menu");
    // Insert commands and the table structure family are operator-reachable.
    expect(screen.getByTestId("overflow-cmd-mention.at")).toBeTruthy();
    expect(screen.getByTestId("overflow-cmd-table.addRowAfter")).toBeTruthy();
    // Table edit commands are truthfully disabled outside a table (M11).
    expect((screen.getByTestId("overflow-cmd-table.addRowAfter") as HTMLButtonElement).disabled).toBe(true);

    // Run the mention command through the menu: the arg prompt opens, and the
    // confirmed value creates a REAL mention node (M1).
    await act(async () => {
      fireEvent.click(screen.getByTestId("overflow-cmd-mention.at"));
    });
    await act(async () => {
      fireEvent.change(await screen.findByTestId("editor-arg-value"), {
        target: { value: "operator" },
      });
      fireEvent.click(screen.getByTestId("editor-arg-confirm"));
    });
    await waitFor(() => {
      const debug = (globalThis as Record<string, unknown>).__HS_EDITOR_DEBUG__ as
        | { nodeCounts?: Record<string, number> }
        | undefined;
      expect(debug?.nodeCounts?.mention ?? 0).toBe(1);
    });
  });

  it("disables undo until an edit exists, then undo/redo round-trips from the toolbar (iteration-3 L14)", async () => {
    await act(async () => {
      renderEditor();
    });
    const undo = (await screen.findByTestId("editor-cmd-history.undo")) as HTMLButtonElement;
    const redo = screen.getByTestId("editor-cmd-history.redo") as HTMLButtonElement;
    expect(undo.disabled).toBe(true);
    expect(redo.disabled).toBe(true);

    // Make an edit through the toolbar (h1), then undo it.
    await act(async () => {
      fireEvent.click(screen.getByTestId("editor-cmd-block.h1"));
    });
    await waitFor(() => expect((screen.getByTestId("editor-cmd-history.undo") as HTMLButtonElement).disabled).toBe(false));
    await act(async () => {
      fireEvent.click(screen.getByTestId("editor-cmd-history.undo"));
    });
    await waitFor(() => {
      expect(screen.getByTestId("editor-cmd-block.h1").getAttribute("data-active")).toBe("false");
      expect((screen.getByTestId("editor-cmd-history.redo") as HTMLButtonElement).disabled).toBe(false);
    });
    await act(async () => {
      fireEvent.click(screen.getByTestId("editor-cmd-history.redo"));
    });
    await waitFor(() => {
      expect(screen.getByTestId("editor-cmd-block.h1").getAttribute("data-active")).toBe("true");
    });
  });

  it("degrades to a non-blank notice when the extension set fails to build (MT-174)", async () => {
    await act(async () => {
      render(
        <RichTextEditor
          initialContent={null}
          onChange={() => {}}
          extensionFactory={() => {
            throw new Error("simulated extension init failure");
          }}
        />,
      );
    });
    // No blank: the fatal notice + dependency banner render.
    expect(await screen.findByTestId("rich-text-editor-fatal")).toBeTruthy();
    expect(screen.getByTestId("rich-text-editor").getAttribute("data-editor-degraded")).toBe("true");
  });

  it("renders a live heading outline and clicking an item moves the real editor selection (MT-245)", async () => {
    const scrollIntoView = vi.fn();
    const originalScrollIntoView = HTMLElement.prototype.scrollIntoView;
    HTMLElement.prototype.scrollIntoView = scrollIntoView;
    let editorRef: Editor | null = null;
    const doc: JSONContent = {
      type: "doc",
      content: [
        { type: "heading", attrs: { level: 1 }, content: [{ type: "text", text: "Runbook" }] },
        { type: "paragraph", content: [{ type: "text", text: "Body text" }] },
        { type: "heading", attrs: { level: 2 }, content: [{ type: "text", text: "Recovery" }] },
      ],
    };

    try {
      await act(async () => {
        renderEditor({
          initialContent: doc,
          onEditorReady: (editor) => {
            editorRef = editor;
          },
        });
      });

      const outline = await screen.findByTestId("rich-text-editor-outline");
      expect(outline).toHaveAttribute("data-outline-count", "2");
      const items = screen.getAllByTestId("rich-text-editor-outline-item");
      expect(items.map((item) => item.textContent)).toEqual(["Runbook", "Recovery"]);

      await act(async () => {
        fireEvent.click(items[1]);
      });
      await waitFor(() => {
        expect(editorRef?.state.selection.from).toBe(Number(items[1].getAttribute("data-selection-pos")));
        expect(scrollIntoView).toHaveBeenCalled();
      });
    } finally {
      HTMLElement.prototype.scrollIntoView = originalScrollIntoView;
    }
  });

  it("renders the MT-245 status bar from live editor state plus authority save state", async () => {
    let editorRef: Editor | null = null;
    const doc: JSONContent = {
      type: "doc",
      content: [
        { type: "paragraph", content: [{ type: "text", text: "alpha beta" }] },
        {
          type: "monacoCodeBlock",
          attrs: { language: "typescript", code: "const answer = 42;", contentHash: "hash" },
        },
      ],
    };
    await act(async () => {
      renderEditor({
        initialContent: doc,
        onEditorReady: (editor) => {
          editorRef = editor;
        },
        documentStatus: {
          dirty: true,
          saving: false,
          blocked: false,
          backendErrorKind: "conflict",
          lastSavedAt: "12:34:56",
        },
      });
    });

    const status = await screen.findByTestId("rich-text-editor-status-bar");
    expect(status).toHaveAttribute("data-save-state", "conflict");
    expect(status).toHaveAttribute("data-word-count", "5");
    expect(screen.getByTestId("rich-text-editor-status-save").textContent).toContain("Conflict");
    expect(screen.getByTestId("rich-text-editor-status-cursor").textContent).toMatch(/Ln \d+, Col \d+/);

    let codePos = -1;
    editorRef!.state.doc.descendants((node, pos) => {
      if (node.type.name === "monacoCodeBlock") {
        codePos = pos;
        return false;
      }
      return true;
    });
    expect(codePos).toBeGreaterThanOrEqual(0);
    await act(async () => {
      editorRef!.view.dispatch(
        editorRef!.state.tr.setSelection(NodeSelection.create(editorRef!.state.doc, codePos)),
      );
    });
    await waitFor(() => {
      expect(screen.getByTestId("rich-text-editor-status-language").textContent).toContain("typescript");
    });
  });
});
