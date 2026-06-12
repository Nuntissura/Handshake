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
import { act } from "react";
import { describe, it, expect, vi } from "vitest";
import { RichTextEditor } from "./RichTextEditor";
import type { JSONContent } from "@tiptap/core";

function renderEditor(props?: Partial<React.ComponentProps<typeof RichTextEditor>>) {
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
});
