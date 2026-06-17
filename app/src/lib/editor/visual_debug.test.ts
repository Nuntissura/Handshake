// WP-KERNEL-009 / MT-172 — EditorVisualDebugSelectors (debug payload) tests.
//
// Proves the debug snapshot reports node counts, embedded code blocks (language
// + hash + length), typed links, and selection from a REAL editor document — the
// machine-readable state a no-context model / the visual lane asserts against.

import { describe, it, expect, afterEach } from "vitest";
import { Editor } from "@tiptap/core";
import { buildHandshakeEditorExtensions } from "./build_editor_extensions";
import {
  buildEditorDebugSnapshot,
  isEditorDebugEnabled,
  publishEditorDebugSnapshot,
  EDITOR_STABLE_SELECTORS,
  EDITOR_DEBUG_ENABLE_KEY,
  EDITOR_DEBUG_GLOBAL_KEY,
  EDITOR_DEBUG_BY_ID_GLOBAL_KEY,
  type DebuggableEditor,
} from "./visual_debug";
import { makeCodeBlockAttrs } from "./code_block_serialization";

function makeEditor(): Editor {
  return new Editor({
    extensions: buildHandshakeEditorExtensions(),
    content: { type: "doc", content: [{ type: "paragraph", content: [{ type: "text", text: "hi" }] }] },
  });
}

describe("editor visual-debug snapshot (MT-172)", () => {
  it("reports node counts, code blocks, and links from the document", () => {
    const editor = makeEditor();
    editor
      .chain()
      .insertContent({ type: "monacoCodeBlock", attrs: makeCodeBlockAttrs("rust", "fn main() {}") })
      .insertContent({
        type: "hsLink",
        attrs: { refKind: "wp", refValue: "WP-1", label: "WP-1", resolved: true },
      })
      .run();

    const snap = buildEditorDebugSnapshot(editor as unknown as DebuggableEditor);
    expect(snap.codeBlocks).toHaveLength(1);
    expect(snap.codeBlocks[0].language).toBe("rust");
    expect(snap.codeBlocks[0].contentHash.length).toBeGreaterThan(0);
    expect(snap.codeBlocks[0].codeLength).toBe("fn main() {}".length);
    expect(snap.links).toHaveLength(1);
    expect(snap.links[0].refKind).toBe("wp");
    expect(snap.nodeCounts.paragraph).toBeGreaterThanOrEqual(1);
    expect(snap.editable).toBe(true);
    editor.destroy();
  });

  it("reports selection range", () => {
    const editor = makeEditor();
    editor.commands.selectAll();
    const snap = buildEditorDebugSnapshot(editor as unknown as DebuggableEditor);
    expect(snap.selection.empty).toBe(false);
    expect(snap.selection.to).toBeGreaterThan(snap.selection.from);
    editor.destroy();
  });

  it("documents the canonical stable selector set", () => {
    expect(EDITOR_STABLE_SELECTORS).toContain("rich-text-editor");
    expect(EDITOR_STABLE_SELECTORS).toContain("rich-text-editor-outline");
    expect(EDITOR_STABLE_SELECTORS).toContain("rich-text-editor-outline-item");
    expect(EDITOR_STABLE_SELECTORS).toContain("rich-text-editor-status-bar");
    expect(EDITOR_STABLE_SELECTORS).toContain("editor-go-to-line-prompt");
    expect(EDITOR_STABLE_SELECTORS).toContain("editor-go-to-line-error");
    expect(EDITOR_STABLE_SELECTORS).toContain("monaco-code-block");
    expect(EDITOR_STABLE_SELECTORS).toContain("hs-link");
    expect(EDITOR_STABLE_SELECTORS).toContain("hs-link-navigation-error");
    expect(EDITOR_STABLE_SELECTORS).toContain("rich-text-editor-backend-error");
  });
});

describe("debug gating + per-editor namespacing (iteration-3 M15/L19)", () => {
  afterEach(() => {
    const g = globalThis as Record<string, unknown>;
    delete g[EDITOR_DEBUG_ENABLE_KEY];
    delete g[EDITOR_DEBUG_GLOBAL_KEY];
    delete g[EDITOR_DEBUG_BY_ID_GLOBAL_KEY];
  });

  it("explicit enable/disable overrides; defaults ON outside production builds", () => {
    const g = globalThis as Record<string, unknown>;
    // Test env (vitest MODE=test) defaults to enabled.
    expect(isEditorDebugEnabled()).toBe(true);
    g[EDITOR_DEBUG_ENABLE_KEY] = false;
    expect(isEditorDebugEnabled()).toBe(false);
    g[EDITOR_DEBUG_ENABLE_KEY] = true;
    expect(isEditorDebugEnabled()).toBe(true);
  });

  it("publishes on both the last-writer global and the per-id namespace", () => {
    const editor = makeEditor();
    const snapshot = buildEditorDebugSnapshot(editor as unknown as DebuggableEditor);
    publishEditorDebugSnapshot(snapshot, "KRD-A");
    const second = makeEditor();
    const snapshot2 = buildEditorDebugSnapshot(second as unknown as DebuggableEditor);
    publishEditorDebugSnapshot(snapshot2, "KRD-B");

    const g = globalThis as Record<string, unknown>;
    // Last writer wins on the legacy global (back-compat for existing specs)...
    expect(g[EDITOR_DEBUG_GLOBAL_KEY]).toBe(snapshot2);
    // ...while both editors stay attributable through the id namespace.
    const byId = g[EDITOR_DEBUG_BY_ID_GLOBAL_KEY] as Record<string, unknown>;
    expect(byId["KRD-A"]).toBe(snapshot);
    expect(byId["KRD-B"]).toBe(snapshot2);
    editor.destroy();
    second.destroy();
  });
});
