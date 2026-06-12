// WP-KERNEL-009 iteration-3 hardening (H4/M10) — stale-language hash repro.
//
// The Monaco mount effect runs once, so its onDidChangeContent closure captured
// the MOUNT-TIME language. After the operator switched the block language,
// every keystroke minted contentHash against the OLD language — corrupting the
// MT-168 round-trip invariant the save path persists. jsdom cannot mount real
// Monaco, so this file mocks the single Monaco entry point with a minimal
// editor double that captures the change listener — letting the test drive the
// EXACT closure path: mount(ts) -> switch(json) -> type -> hash MUST be
// computed against json. The real-Monaco runtime proof runs in the Playwright
// typing spec (browser lane).

import { render, screen, waitFor, fireEvent } from "@testing-library/react";
import { act } from "react";
import { describe, it, expect, vi, beforeEach } from "vitest";
import { EditorContent, useEditor } from "@tiptap/react";
import StarterKit from "@tiptap/starter-kit";
import { MonacoCodeBlockNode } from "../lib/tiptap/monaco_code_block_node";
import { makeCodeBlockAttrs, codeBlockHash } from "../lib/editor/code_block_serialization";

const monacoDouble = vi.hoisted(() => {
  const state = {
    value: "",
    listeners: [] as Array<() => void>,
    setValueCalls: 0,
    pushEditCalls: 0,
    viewStateSaved: 0,
    viewStateRestored: 0,
  };
  return {
    state,
    reset() {
      state.value = "";
      state.listeners = [];
      state.setValueCalls = 0;
      state.pushEditCalls = 0;
      state.viewStateSaved = 0;
      state.viewStateRestored = 0;
    },
    /** Simulates the operator typing inside Monaco (model change + listeners). */
    type(next: string) {
      state.value = next;
      for (const cb of state.listeners) cb();
    },
  };
});

vi.mock("../lib/monaco/setup", () => {
  const model = {
    onDidChangeContent: (cb: () => void) => {
      monacoDouble.state.listeners.push(cb);
      return { dispose() {} };
    },
    getPositionAt: () => ({ lineNumber: 1, column: 1 }),
    getFullModelRange: () => ({ startLineNumber: 1, startColumn: 1, endLineNumber: 1, endColumn: 1 }),
    pushEditOperations: (
      _selections: unknown,
      edits: Array<{ text: string }>,
    ) => {
      monacoDouble.state.pushEditCalls += 1;
      monacoDouble.state.value = edits[0]?.text ?? "";
      return null;
    },
  };
  const instance = {
    getModel: () => model,
    getValue: () => monacoDouble.state.value,
    setValue: (v: string) => {
      monacoDouble.state.setValueCalls += 1;
      monacoDouble.state.value = v;
    },
    updateOptions: () => {},
    dispose: () => {},
    setSelection: () => {},
    revealRangeInCenterIfOutsideViewport: () => {},
    addCommand: () => null,
    saveViewState: () => {
      monacoDouble.state.viewStateSaved += 1;
      return { dummy: true };
    },
    restoreViewState: () => {
      monacoDouble.state.viewStateRestored += 1;
    },
  };
  return {
    createConfiguredEditor: ({ value }: { value: string }) => {
      monacoDouble.state.value = value;
      return instance;
    },
    monaco: { editor: { setModelLanguage: () => {} }, KeyCode: { Escape: 9 } },
  };
});

function Harness({
  language,
  code,
  onEditor,
}: {
  language: string;
  code: string;
  onEditor?: (editor: NonNullable<ReturnType<typeof useEditor>>) => void;
}) {
  const attrs = makeCodeBlockAttrs(language, code);
  const editor = useEditor({
    extensions: [StarterKit.configure({ heading: { levels: [1, 2, 3] } }), MonacoCodeBlockNode],
    content: { type: "doc", content: [{ type: "monacoCodeBlock", attrs }] },
  });
  if (!editor) return null;
  onEditor?.(editor);
  return <EditorContent editor={editor} />;
}

describe("MonacoCodeBlockView language/hash integrity (iteration-3 H4/M10)", () => {
  beforeEach(() => {
    monacoDouble.reset();
  });

  it("hashes keystrokes against the CURRENT language after a language switch", async () => {
    await act(async () => {
      render(<Harness language="typescript" code={"const x = 1;"} />);
    });
    const block = await screen.findByTestId("monaco-code-block");
    await waitFor(() => expect(block.getAttribute("data-monaco-mounted")).toBe("true"));

    // Operator switches the block language ts -> json.
    await act(async () => {
      fireEvent.change(screen.getByTestId("monaco-code-block-language"), {
        target: { value: "json" },
      });
    });
    await waitFor(() => expect(block.getAttribute("data-language")).toBe("json"));

    // Operator types INSIDE Monaco (the mount-once change listener fires).
    const typed = '{"a": 1}';
    await act(async () => {
      monacoDouble.type(typed);
    });

    // The persisted hash MUST be minted against the current language (json).
    // The pre-fix closure hashed against the mount-time language (typescript).
    await waitFor(() => {
      expect(block.getAttribute("data-rt-hash")).toBe(codeBlockHash("json", typed));
    });
    expect(block.getAttribute("data-rt-hash")).not.toBe(codeBlockHash("typescript", typed));
    expect(block.getAttribute("data-language")).toBe("json");
  });

  it("hashes keystrokes against the mount language when never switched", async () => {
    await act(async () => {
      render(<Harness language="rust" code={"fn main() {}"} />);
    });
    const block = await screen.findByTestId("monaco-code-block");
    await waitFor(() => expect(block.getAttribute("data-monaco-mounted")).toBe("true"));

    const typed = "fn main() { dbg!(1); }";
    await act(async () => {
      monacoDouble.type(typed);
    });
    await waitFor(() => {
      expect(block.getAttribute("data-rt-hash")).toBe(codeBlockHash("rust", typed));
    });
  });

  it("language switch alone re-mints the hash for the existing code", async () => {
    await act(async () => {
      render(<Harness language="typescript" code={"42"} />);
    });
    const block = await screen.findByTestId("monaco-code-block");
    await waitFor(() => expect(block.getAttribute("data-monaco-mounted")).toBe("true"));

    await act(async () => {
      fireEvent.change(screen.getByTestId("monaco-code-block-language"), {
        target: { value: "python" },
      });
    });
    await waitFor(() => {
      expect(block.getAttribute("data-rt-hash")).toBe(codeBlockHash("python", "42"));
    });
  });

  it("reconciles EXTERNAL code changes via pushEditOperations with view-state preserve, never setValue (iteration-3 M5)", async () => {
    type TiptapEditor = NonNullable<ReturnType<typeof useEditor>>;
    let editor: TiptapEditor | null = null;
    await act(async () => {
      render(
        <Harness language="typescript" code={"const a = 1;"} onEditor={(e) => (editor = e)} />,
      );
    });
    const block = await screen.findByTestId("monaco-code-block");
    await waitFor(() => expect(block.getAttribute("data-monaco-mounted")).toBe("true"));
    expect(editor).toBeTruthy();

    // External writer (undo/redo, collab, reload) replaces the code attr.
    await act(async () => {
      editor!.commands.command(({ tr, state }) => {
        let pos = -1;
        state.doc.descendants((node, p) => {
          if (node.type.name === "monacoCodeBlock") {
            pos = p;
            return false;
          }
          return true;
        });
        const node = state.doc.nodeAt(pos)!;
        tr.setNodeMarkup(pos, undefined, {
          ...node.attrs,
          ...makeCodeBlockAttrs("typescript", "const a = 2; // external"),
        });
        return true;
      });
    });

    await waitFor(() => {
      expect(monacoDouble.state.value).toBe("const a = 2; // external");
    });
    // The reconcile flowed through pushEditOperations (undo/caret preserved),
    // never the stack-destroying setValue, and the view state round-tripped.
    expect(monacoDouble.state.pushEditCalls).toBeGreaterThanOrEqual(1);
    expect(monacoDouble.state.setValueCalls).toBe(0);
    expect(monacoDouble.state.viewStateSaved).toBeGreaterThanOrEqual(1);
    expect(monacoDouble.state.viewStateRestored).toBeGreaterThanOrEqual(1);
  });
});
