// WP-KERNEL-009 / MT-251 - Monaco construction defaults.
//
// Pins the shared createConfiguredEditor options that make the embedded code
// editor behave like an operator-facing power editor instead of relying on
// Monaco's defaults changing under us.

import { beforeEach, describe, expect, it, vi } from "vitest";

const monacoDouble = vi.hoisted(() => {
  const state = {
    createdEditorOptions: null as null | Record<string, unknown>,
  };
  return {
    state,
    reset() {
      state.createdEditorOptions = null;
    },
  };
});

vi.mock("monaco-editor/esm/vs/editor/editor.worker?worker", () => ({ default: class WorkerDouble {} }));
vi.mock("monaco-editor/esm/vs/language/json/json.worker?worker", () => ({ default: class WorkerDouble {} }));
vi.mock("monaco-editor/esm/vs/language/css/css.worker?worker", () => ({ default: class WorkerDouble {} }));
vi.mock("monaco-editor/esm/vs/language/html/html.worker?worker", () => ({ default: class WorkerDouble {} }));
vi.mock("monaco-editor/esm/vs/language/typescript/ts.worker?worker", () => ({ default: class WorkerDouble {} }));

vi.mock("monaco-editor", () => {
  const disposable = { dispose() {} };
  return {
    editor: {
      defineTheme: () => {},
      create: (_container: HTMLElement, options: Record<string, unknown>) => {
        monacoDouble.state.createdEditorOptions = options;
        return { dispose() {} };
      },
      createDiffEditor: () => ({ dispose() {} }),
    },
    languages: {
      CompletionItemKind: {
        Class: 1,
        Enum: 2,
        Field: 3,
        Function: 4,
        Module: 5,
        Variable: 6,
      },
      registerCompletionItemProvider: () => disposable,
      registerHoverProvider: () => disposable,
      registerDefinitionProvider: () => disposable,
      registerReferenceProvider: () => disposable,
      registerDocumentFormattingEditProvider: () => disposable,
      registerCodeLensProvider: () => disposable,
      registerInlayHintsProvider: () => disposable,
      InlayHintKind: { Type: 1 },
      typescript: {},
    },
    Range: class Range {
      constructor(
        public startLineNumber: number,
        public startColumn: number,
        public endLineNumber: number,
        public endColumn: number,
      ) {}
    },
  };
});

describe("createConfiguredEditor (MT-251 Monaco editing parity)", () => {
  beforeEach(() => {
    monacoDouble.reset();
  });

  it("enables multi-cursor and column-selection behavior for real code editors", async () => {
    const { createConfiguredEditor } = await import("./setup");
    const container = document.createElement("div");

    createConfiguredEditor({ container, value: "a\nb\nc", language: "typescript" });

    expect(monacoDouble.state.createdEditorOptions).toMatchObject({
      columnSelection: true,
      multiCursorModifier: "alt",
      multiCursorPaste: "spread",
      multiCursorMergeOverlapping: true,
    });
  });
});
