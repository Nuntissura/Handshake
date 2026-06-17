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
import { Doc as YDoc } from "yjs";
import { formatNoteTemplateDate } from "../lib/editor/editor_note_templates";

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

  it("inserts a daily note template from the overflow menu with title/date variables (MT-258)", async () => {
    const expectedDate = formatNoteTemplateDate(new Date());
    await act(async () => {
      renderEditor();
    });

    await act(async () => {
      fireEvent.click(screen.getByTestId("editor-open-overflow"));
    });
    await act(async () => {
      fireEvent.click(screen.getByTestId("overflow-cmd-template.note.daily"));
    });

    const prompt = await screen.findByTestId("editor-arg-prompt");
    expect(prompt).toBeTruthy();
    await act(async () => {
      fireEvent.change(screen.getByTestId("editor-arg-title"), {
        target: { value: "MT-258 Daily" },
      });
      fireEvent.click(screen.getByTestId("editor-arg-confirm"));
    });

    await waitFor(() => {
      const surface = screen.getByTestId("rich-text-editor-surface");
      expect(surface.textContent).toContain("MT-258 Daily");
      expect(surface.textContent).toContain(`Date: ${expectedDate}`);
      expect(surface.textContent).toContain("Notes");
      expect(surface.textContent).toContain("Links");
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

  it("opens and filters the command palette from an external command request", async () => {
    await act(async () => {
      renderEditor({
        commandPaletteRequest: { paneId: "pane-a", requestId: 1, query: "Bold" },
      });
    });

    const palette = await screen.findByTestId("editor-command-palette");
    expect(palette).toBeTruthy();
    expect(screen.getByTestId("editor-command-palette-input")).toHaveValue("Bold");
    expect(screen.getByTestId("palette-cmd-format.bold")).toBeTruthy();
  });

  it("opens the existing find panel from an external find request and selects the match", async () => {
    await act(async () => {
      renderEditor({
        findRequest: {
          requestId: 1,
          query: "hello",
          caseSensitive: false,
          wholeWord: false,
          isRegex: false,
        },
      });
    });

    expect(await screen.findByTestId("find-panel")).toBeTruthy();
    expect(screen.getByTestId("find-input")).toHaveValue("hello");
    await waitFor(() => {
      expect(screen.getByTestId("find-panel")).toHaveAttribute("data-match-count", "1");
      expect(screen.getByTestId("find-panel")).toHaveAttribute("data-active-index", "0");
    });
  });

  it("leaves plain Mod-p for the app-level QuickSwitcher while Mod-Shift-p opens the editor palette", async () => {
    await act(async () => {
      renderEditor();
    });
    const surface = await screen.findByTestId("rich-text-editor-surface");
    const editorSurface = surface.querySelector(".ProseMirror") as HTMLElement;

    await act(async () => {
      fireEvent.keyDown(editorSurface, { key: "p", ctrlKey: true });
    });
    expect(screen.queryByTestId("editor-command-palette")).toBeNull();

    await act(async () => {
      fireEvent.keyDown(editorSurface, { key: "p", ctrlKey: true, shiftKey: true });
    });
    expect(await screen.findByTestId("editor-command-palette")).toBeTruthy();
  });

  it("toggles focus mode by hiding ordinary toolbar controls while keeping an exit control", async () => {
    await act(async () => {
      renderEditor();
    });

    const root = await screen.findByTestId("rich-text-editor");
    const toolbar = await screen.findByTestId("rich-text-editor-toolbar");
    const surface = await screen.findByTestId("rich-text-editor-surface");
    const bold = await screen.findByTestId("editor-cmd-format.bold");
    const toggle = await screen.findByTestId("editor-toggle-focus-mode");

    expect(root).toHaveAttribute("data-focus-mode", "false");
    expect(toolbar).toHaveAttribute("data-focus-mode", "false");
    expect(surface).toHaveAttribute("data-focus-mode", "false");
    expect(toggle).toHaveAttribute("aria-pressed", "false");

    await act(async () => {
      fireEvent.click(toggle);
    });

    expect(root).toHaveAttribute("data-focus-mode", "true");
    expect(toolbar).toHaveAttribute("data-focus-mode", "true");
    expect(surface).toHaveAttribute("data-focus-mode", "true");
    expect(toggle).toHaveAttribute("aria-pressed", "true");
    expect(bold).toHaveAttribute("data-focus-mode-hidden", "true");
    expect(bold).toHaveAttribute("tabindex", "-1");

    await act(async () => {
      fireEvent.click(toggle);
    });

    expect(root).toHaveAttribute("data-focus-mode", "false");
    expect(toolbar).toHaveAttribute("data-focus-mode", "false");
    expect(surface).toHaveAttribute("data-focus-mode", "false");
    expect(toggle).toHaveAttribute("aria-pressed", "false");
    expect(bold).toHaveAttribute("data-focus-mode-hidden", "false");
  });

  it("keeps focus mode free of command prompts and shortcut-opened transient panels", async () => {
    await act(async () => {
      renderEditor();
    });

    const surface = await screen.findByTestId("rich-text-editor-surface");
    const editorSurface = surface.querySelector(".ProseMirror") as HTMLElement;

    await act(async () => {
      fireEvent.click(screen.getByTestId("editor-open-palette"));
    });
    await act(async () => {
      fireEvent.change(screen.getByTestId("editor-command-palette-input"), { target: { value: "link" } });
    });
    await act(async () => {
      fireEvent.click(await screen.findByTestId("palette-cmd-link.wikilink"));
    });
    expect(await screen.findByTestId("editor-arg-prompt")).toBeTruthy();

    await act(async () => {
      fireEvent.click(screen.getByTestId("editor-toggle-focus-mode"));
    });
    expect(screen.queryByTestId("editor-arg-prompt")).toBeNull();
    expect(screen.queryByTestId("editor-command-palette")).toBeNull();

    await act(async () => {
      fireEvent.keyDown(editorSurface, { key: "k", ctrlKey: true });
      fireEvent.keyDown(editorSurface, { key: "c", ctrlKey: true, altKey: true });
      fireEvent.keyDown(editorSurface, { key: "p", ctrlKey: true, shiftKey: true });
    });

    expect(screen.queryByTestId("editor-arg-prompt")).toBeNull();
    expect(screen.queryByTestId("editor-command-palette")).toBeNull();
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

  it("keeps two mounted editors for the same Yjs document consistent (MT-246)", async () => {
    const collaborationDocument = new YDoc();
    const initial: JSONContent = {
      type: "doc",
      content: [{ type: "paragraph", content: [{ type: "text", text: "shared alpha" }] }],
    };
    let editorA: Editor | null = null;
    let editorB: Editor | null = null;

    await act(async () => {
      render(
        <>
          <RichTextEditor
            initialContent={initial}
            onChange={() => {}}
            onEditorReady={(editor) => {
              editorA = editor;
            }}
            collaborationDocument={collaborationDocument}
          />
          <RichTextEditor
            initialContent={initial}
            onChange={() => {}}
            onEditorReady={(editor) => {
              editorB = editor;
            }}
            collaborationDocument={collaborationDocument}
          />
        </>,
      );
    });

    await waitFor(() => {
      expect(editorA).toBeTruthy();
      expect(editorB).toBeTruthy();
    });

    await act(async () => {
      editorA!.commands.setContent({
        type: "doc",
        content: [{ type: "paragraph", content: [{ type: "text", text: "shared beta" }] }],
      });
    });

    await waitFor(() => {
      expect(editorB!.state.doc.textBetween(0, editorB!.state.doc.content.size, "\n")).toContain("shared beta");
    });
  });

  it("does not report initial collaborative hydration as a user edit (MT-246)", async () => {
    const collaborationDocument = new YDoc();
    const initial: JSONContent = {
      type: "doc",
      content: [{ type: "paragraph", content: [{ type: "text", text: "server clean" }] }],
    };
    const onChange = vi.fn();
    let editor: Editor | null = null;

    render(
      <RichTextEditor
        initialContent={initial}
        onChange={onChange}
        onEditorReady={(nextEditor) => {
          editor = nextEditor;
        }}
        collaborationDocument={collaborationDocument}
      />,
    );
    await waitFor(() => expect(editor).toBeTruthy());
    await waitFor(() => {
      expect(editor!.state.doc.textBetween(0, editor!.state.doc.content.size, "\n")).toContain("server clean");
    });

    expect(onChange).not.toHaveBeenCalled();
  });

  it("does not let a late-joining same-document editor overwrite shared CRDT edits with stale content (MT-246)", async () => {
    const collaborationDocument = new YDoc();
    const initial: JSONContent = {
      type: "doc",
      content: [{ type: "paragraph", content: [{ type: "text", text: "server alpha" }] }],
    };
    const localEdit: JSONContent = {
      type: "doc",
      content: [{ type: "paragraph", content: [{ type: "text", text: "local beta" }] }],
    };
    let editorA: Editor | null = null;
    let editorB: Editor | null = null;

    const { rerender } = render(
      <RichTextEditor
        initialContent={initial}
        onChange={() => {}}
        onEditorReady={(editor) => {
          editorA = editor;
        }}
        collaborationDocument={collaborationDocument}
      />,
    );
    await waitFor(() => expect(editorA).toBeTruthy());

    await act(async () => {
      editorA!.commands.setContent(localEdit);
    });
    await waitFor(() => {
      expect(editorA!.state.doc.textBetween(0, editorA!.state.doc.content.size, "\n")).toContain("local beta");
    });

    rerender(
      <>
        <RichTextEditor
          initialContent={initial}
          onChange={() => {}}
          onEditorReady={(editor) => {
            editorA = editor;
          }}
          collaborationDocument={collaborationDocument}
        />
        <RichTextEditor
          initialContent={initial}
          onChange={() => {}}
          onEditorReady={(editor) => {
            editorB = editor;
          }}
          collaborationDocument={collaborationDocument}
        />
      </>,
    );
    await waitFor(() => expect(editorB).toBeTruthy());

    await waitFor(() => {
      expect(editorA!.state.doc.textBetween(0, editorA!.state.doc.content.size, "\n")).toContain("local beta");
      expect(editorB!.state.doc.textBetween(0, editorB!.state.doc.content.size, "\n")).toContain("local beta");
    });
  });

  it("recreates the Tiptap instance when the CRDT collaboration document changes (MT-246)", async () => {
    const firstDocument = new YDoc({ guid: "mt246-first-crdt-doc" });
    const secondDocument = new YDoc({ guid: "mt246-second-crdt-doc" });
    const initial: JSONContent = {
      type: "doc",
      content: [{ type: "paragraph", content: [{ type: "text", text: "shared alpha" }] }],
    };
    let editor: Editor | null = null;

    const { rerender } = render(
      <RichTextEditor
        initialContent={initial}
        onChange={() => {}}
        onEditorReady={(nextEditor) => {
          editor = nextEditor;
        }}
        collaborationDocument={firstDocument}
      />,
    );
    await waitFor(() => expect(editor).toBeTruthy());
    const firstEditor = editor;

    rerender(
      <RichTextEditor
        initialContent={initial}
        onChange={() => {}}
        onEditorReady={(nextEditor) => {
          editor = nextEditor;
        }}
        collaborationDocument={secondDocument}
      />,
    );

    await waitFor(() => {
      expect(editor).toBeTruthy();
      expect(editor).not.toBe(firstEditor);
    });
  });

  it("applies an explicit collaborative reset without allowing ordinary late joins to replay stale content (MT-246)", async () => {
    const collaborationDocument = new YDoc();
    const initial: JSONContent = {
      type: "doc",
      content: [{ type: "paragraph", content: [{ type: "text", text: "server alpha" }] }],
    };
    const resetContent: JSONContent = {
      type: "doc",
      content: [{ type: "paragraph", content: [{ type: "text", text: "server gamma" }] }],
    };
    let editor: Editor | null = null;

    const { rerender } = render(
      <RichTextEditor
        initialContent={initial}
        onChange={() => {}}
        onEditorReady={(nextEditor) => {
          editor = nextEditor;
        }}
        collaborationDocument={collaborationDocument}
      />,
    );
    await waitFor(() => expect(editor).toBeTruthy());

    await act(async () => {
      editor!.commands.setContent({
        type: "doc",
        content: [{ type: "paragraph", content: [{ type: "text", text: "local beta" }] }],
      });
    });
    await waitFor(() => {
      expect(editor!.state.doc.textBetween(0, editor!.state.doc.content.size, "\n")).toContain("local beta");
    });

    rerender(
      <RichTextEditor
        initialContent={resetContent}
        onChange={() => {}}
        onEditorReady={(nextEditor) => {
          editor = nextEditor;
        }}
        collaborationDocument={collaborationDocument}
      />,
    );
    expect(editor!.state.doc.textBetween(0, editor!.state.doc.content.size, "\n")).toContain("local beta");

    rerender(
      <RichTextEditor
        initialContent={resetContent}
        onChange={() => {}}
        onEditorReady={(nextEditor) => {
          editor = nextEditor;
        }}
        collaborationDocument={collaborationDocument}
        collaborationResetToken={1}
      />,
    );
    await waitFor(() => {
      expect(editor!.state.doc.textBetween(0, editor!.state.doc.content.size, "\n")).toContain("server gamma");
    });
  });

  it("applies an explicit collaborative reset on first mount after a CRDT-key promotion remount (MT-246)", async () => {
    const collaborationDocument = new YDoc();
    const provisionalContent: JSONContent = {
      type: "doc",
      content: [{ type: "paragraph", content: [{ type: "text", text: "provisional stale" }] }],
    };
    const promotedAuthorityContent: JSONContent = {
      type: "doc",
      content: [{ type: "paragraph", content: [{ type: "text", text: "promoted server" }] }],
    };
    let editor: Editor | null = null;

    const firstRender = render(
      <RichTextEditor
        initialContent={provisionalContent}
        onChange={() => {}}
        onEditorReady={(nextEditor) => {
          editor = nextEditor;
        }}
        collaborationDocument={collaborationDocument}
      />,
    );
    await waitFor(() => expect(editor).toBeTruthy());
    await waitFor(() => {
      expect(editor!.state.doc.textBetween(0, editor!.state.doc.content.size, "\n")).toContain(
        "provisional stale",
      );
    });

    firstRender.unmount();
    editor = null;

    render(
      <RichTextEditor
        initialContent={promotedAuthorityContent}
        onChange={() => {}}
        onEditorReady={(nextEditor) => {
          editor = nextEditor;
        }}
        collaborationDocument={collaborationDocument}
        collaborationResetToken={1}
      />,
    );

    await waitFor(() => expect(editor).toBeTruthy());
    await waitFor(() => {
      const text = editor!.state.doc.textBetween(0, editor!.state.doc.content.size, "\n");
      expect(text).toContain("promoted server");
      expect(text).not.toContain("provisional stale");
    });
  });
});
