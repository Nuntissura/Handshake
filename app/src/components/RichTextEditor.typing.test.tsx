// WP-KERNEL-009 iteration-3 hardening — REAL typing behavior of the integrated
// editor (adversarial findings H1/H3).
//
// H1 (echo-loop caret teleport): RichDocumentView-style parents store the
// onChange JSON and pass it straight back as initialContent. Tiptap 3.13's
// setContent is tr.replaceWith(0, size) with no equality guard, so without an
// echo guard every keystroke re-parses the document and teleports the caret to
// the document end (proven headless by the adversarial review: caret 6 -> 12/13
// after setContent(identical JSON)). These tests type THROUGH an echoing parent
// at the ProseMirror view level (insertText transactions — the same path real
// keydown input takes) and assert the caret stays put keystroke after keystroke.
//
// H3 (Monaco keystroke interception): the editor keymap listens on view.dom, so
// chords typed inside an embedded Monaco code block bubble into the prose
// handler (Mod-Alt-c / Mod-k are unbound in standalone Monaco). With the block
// node-selected, a dispatched insertContent REPLACES the code block. The keymap
// must ignore key events originating inside the code-block host except the
// explicitly global chords (command palette, save).

import { render, screen, waitFor, fireEvent } from "@testing-library/react";
import { act, useState } from "react";
import { describe, it, expect } from "vitest";
import type { Editor, JSONContent } from "@tiptap/core";
import { RichTextEditor } from "./RichTextEditor";

const INITIAL: JSONContent = {
  type: "doc",
  content: [
    { type: "paragraph", content: [{ type: "text", text: "hello world" }] },
  ],
};

/**
 * Reproduces the RichDocumentView / harness wiring the adversarial review
 * flagged: the parent stores the latest onChange JSON in state and passes the
 * SAME object back down as initialContent on the next render.
 */
function EchoParent({ onEditor }: { onEditor: (e: Editor) => void }) {
  const [doc, setDoc] = useState<JSONContent>(INITIAL);
  return <RichTextEditor initialContent={doc} onChange={setDoc} onEditorReady={onEditor} />;
}

function docText(editor: Editor): string {
  return editor.state.doc.textBetween(0, editor.state.doc.content.size, "\n");
}

async function mountEcho(): Promise<Editor> {
  let editor: Editor | null = null;
  await act(async () => {
    render(<EchoParent onEditor={(e) => (editor = e)} />);
  });
  await waitFor(() => expect(editor).toBeTruthy());
  return editor!;
}

describe("RichTextEditor real typing (iteration-3 H1)", () => {
  it("keeps the caret in place while typing mid-document through an echoing parent", async () => {
    const editor = await mountEcho();

    // Caret after "hello" — PM position 6 inside "hello world".
    await act(async () => {
      editor.commands.setTextSelection(6);
    });
    expect(editor.state.selection.from).toBe(6);

    // Type three characters the way real input lands: insertText transactions
    // dispatched through the view. After EACH keystroke the caret must advance
    // by exactly one (the echo-loop defect teleports it to the document end).
    const chars = ["X", "Y", "Z"];
    for (let i = 0; i < chars.length; i++) {
      await act(async () => {
        editor.view.dispatch(editor.state.tr.insertText(chars[i]));
      });
      expect(editor.state.selection.from, `caret after keystroke ${i + 1}`).toBe(7 + i);
    }

    expect(docText(editor)).toBe("helloXYZ world");
  });

  it("applies a genuinely external document update (reload / conflict resolution)", async () => {
    let editor: Editor | null = null;
    const external: JSONContent = {
      type: "doc",
      content: [
        { type: "paragraph", content: [{ type: "text", text: "reloaded from authority" }] },
      ],
    };

    const { rerender } = render(
      <RichTextEditor
        initialContent={INITIAL}
        onChange={() => {}}
        onEditorReady={(e) => (editor = e)}
      />,
    );
    await waitFor(() => expect(editor).toBeTruthy());
    expect(docText(editor!)).toBe("hello world");

    // A brand-new object with different content = a real external update.
    await act(async () => {
      rerender(
        <RichTextEditor
          initialContent={external}
          onChange={() => {}}
          onEditorReady={(e) => (editor = e)}
        />,
      );
    });
    expect(docText(editor!)).toBe("reloaded from authority");
  });

  it("does not reset the caret when a parent re-render passes a structurally identical doc", async () => {
    let editor: Editor | null = null;
    const { rerender } = render(
      <RichTextEditor
        initialContent={INITIAL}
        onChange={() => {}}
        onEditorReady={(e) => (editor = e)}
      />,
    );
    await waitFor(() => expect(editor).toBeTruthy());
    await act(async () => {
      editor!.commands.setTextSelection(6);
    });

    // A deep clone (e.g. a backend round-trip that did not change anything) must
    // not replace the document out from under the caret.
    const clone = JSON.parse(JSON.stringify(editor!.getJSON())) as JSONContent;
    await act(async () => {
      rerender(
        <RichTextEditor
          initialContent={clone}
          onChange={() => {}}
          onEditorReady={(e) => (editor = e)}
        />,
      );
    });
    expect(editor!.state.selection.from).toBe(6);
    expect(docText(editor!)).toBe("hello world");
  });
});

describe("RichTextEditor keystroke routing (iteration-3 H3)", () => {
  async function mountWithCodeBlock(): Promise<{ editor: Editor }> {
    let editor: Editor | null = null;
    const initial: JSONContent = {
      type: "doc",
      content: [
        { type: "paragraph", content: [{ type: "text", text: "prose before" }] },
        {
          type: "monacoCodeBlock",
          attrs: { language: "typescript", code: "const x = 1;", contentHash: "" },
        },
      ],
    };
    await act(async () => {
      render(
        <RichTextEditor
          initialContent={initial}
          onChange={() => {}}
          onEditorReady={(e) => (editor = e)}
        />,
      );
    });
    await waitFor(() => expect(editor).toBeTruthy());
    await screen.findByTestId("monaco-code-block");
    return { editor: editor! };
  }

  it("ignores editor chords that originate inside the embedded code block", async () => {
    const { editor } = await mountWithCodeBlock();
    const block = screen.getByTestId("monaco-code-block");
    // Node-select the code block (the destructive precondition from the review:
    // insertContent over a NodeSelection REPLACES the selected node).
    let blockPos = -1;
    editor.state.doc.descendants((node, pos) => {
      if (node.type.name === "monacoCodeBlock") {
        blockPos = pos;
        return false;
      }
      return true;
    });
    expect(blockPos).toBeGreaterThanOrEqual(0);
    await act(async () => {
      editor.commands.setNodeSelection(blockPos);
    });

    const before = editor.getJSON();
    // Mod-Alt-c (insert code block) dispatched from INSIDE the code block — the
    // chord belongs to the code editor context and must be ignored by the prose
    // keymap (no arg prompt, no node replacement).
    await act(async () => {
      fireEvent.keyDown(block, { key: "c", ctrlKey: true, altKey: true, bubbles: true });
    });
    expect(screen.queryByTestId("editor-arg-prompt")).toBeNull();
    expect(editor.getJSON()).toEqual(before);

    // Mod-k (insert link) from inside the block: same containment.
    await act(async () => {
      fireEvent.keyDown(block, { key: "k", ctrlKey: true, bubbles: true });
    });
    expect(screen.queryByTestId("editor-arg-prompt")).toBeNull();
    expect(editor.getJSON()).toEqual(before);
  });

  it("contains StarterKit's native prose chords when they originate inside the code block", async () => {
    const { editor } = await mountWithCodeBlock();
    const block = screen.getByTestId("monaco-code-block");
    // Caret sits in the PROSE paragraph while the key event comes from the code
    // island (the focus/selection split the review flagged). Without the PM-level
    // guard, StarterKit's Mod-Alt-1 keymap would retitle the prose paragraph.
    await act(async () => {
      editor.commands.setTextSelection(3);
    });
    const before = editor.getJSON();
    await act(async () => {
      fireEvent.keyDown(block, { key: "1", ctrlKey: true, altKey: true, bubbles: true });
    });
    expect(editor.getJSON()).toEqual(before);
    // Sanity: the same chord from the prose surface DOES work natively.
    const prose = screen.getByTestId("rich-text-editor-surface").querySelector(".ProseMirror")!;
    await act(async () => {
      fireEvent.keyDown(prose, { key: "1", ctrlKey: true, altKey: true, bubbles: true });
    });
    expect(editor.isActive("heading", { level: 1 })).toBe(true);
  });

  it("still opens the command palette (global chord) from inside the code block", async () => {
    await mountWithCodeBlock();
    const block = screen.getByTestId("monaco-code-block");
    await act(async () => {
      fireEvent.keyDown(block, { key: "p", ctrlKey: true, shiftKey: true, bubbles: true });
    });
    expect(await screen.findByTestId("editor-command-palette")).toBeTruthy();
  });

  it("keeps handling chords from the prose surface: Mod-Alt-c inserts an EMBEDDED Monaco block", async () => {
    const { editor } = await mountWithCodeBlock();
    // Caret in the prose paragraph; dispatch Mod-Alt-c from the prose DOM.
    await act(async () => {
      editor.commands.setTextSelection(3);
    });
    const countBlocks = () => {
      let count = 0;
      editor.state.doc.descendants((node) => {
        if (node.type.name === "monacoCodeBlock") count += 1;
        return true;
      });
      return count;
    };
    expect(countBlocks()).toBe(1);
    const prose = screen.getByTestId("rich-text-editor-surface").querySelector(".ProseMirror")!;
    await act(async () => {
      fireEvent.keyDown(prose, { key: "c", ctrlKey: true, altKey: true, bubbles: true });
    });
    // The keystroke guard owns the chord: ONE new monacoCodeBlock (StarterKit's
    // plain codeBlock toggle, which natively binds the same chord, must NOT win
    // and must not double-fire).
    expect(countBlocks()).toBe(2);
    let plainCodeBlocks = 0;
    editor.state.doc.descendants((node) => {
      if (node.type.name === "codeBlock") plainCodeBlocks += 1;
      return true;
    });
    expect(plainCodeBlocks).toBe(0);
  });

  it("keeps handling prompt-backed chords from the prose surface (Mod-k insert link)", async () => {
    const { editor } = await mountWithCodeBlock();
    await act(async () => {
      editor.commands.setTextSelection(3);
    });
    const prose = screen.getByTestId("rich-text-editor-surface").querySelector(".ProseMirror")!;
    await act(async () => {
      fireEvent.keyDown(prose, { key: "k", ctrlKey: true, bubbles: true });
    });
    // link.wikilink requires args — the arg prompt opening proves the component
    // keymap still routes prose-origin chords.
    expect(await screen.findByTestId("editor-arg-prompt")).toBeTruthy();
  });

  it("does not double-dispatch chords already handled natively by ProseMirror", async () => {
    const { editor } = await mountWithCodeBlock();
    await act(async () => {
      editor.commands.setTextSelection({ from: 1, to: 6 });
    });
    const surface = screen.getByTestId("rich-text-editor-surface");
    const prose = surface.querySelector(".ProseMirror") ?? surface;
    // Simulate StarterKit's native keymap having handled Mod-b (it calls
    // preventDefault). The component handler must then NOT toggle bold again —
    // a double dispatch would undo the native toggle on every keystroke.
    await act(async () => {
      const event = new KeyboardEvent("keydown", {
        key: "b",
        ctrlKey: true,
        bubbles: true,
        cancelable: true,
      });
      event.preventDefault();
      prose.dispatchEvent(event);
    });
    // Bold must NOT be active: the only dispatch was the (simulated) native one,
    // whose effect is outside this synthetic event. If the component handler had
    // run toggleBold, the mark would now be active.
    expect(editor.isActive("bold")).toBe(false);
  });
});
