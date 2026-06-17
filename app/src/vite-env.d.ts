/// <reference types="vite/client" />

declare module "monaco-editor/esm/vs/editor/contrib/snippet/browser/snippetController2.js" {
  import type * as monaco from "monaco-editor";

  export const SnippetController2: {
    get(
      editor: monaco.editor.IStandaloneCodeEditor,
    ): {
      insert(template: string, opts?: { undoStopBefore?: boolean; undoStopAfter?: boolean }): void;
      isInSnippet(): boolean;
      next(): void;
      prev(): void;
    } | undefined;
  };
}
