import "../App.css";

import { createElement as h } from "react";
import { createRoot } from "react-dom/client";

import { WorkspaceSearchPanel } from "./WorkspaceSearchPanel";
import type { EditorFindOptions } from "../lib/editor/editor_find_request";

declare global {
  interface Window {
    __workspaceSearchOpenLog?: string[];
  }
}

const root = document.getElementById("harness-root");
if (root) {
  window.__workspaceSearchOpenLog = [];
  createRoot(root).render(
    h(WorkspaceSearchPanel, {
      open: true,
      workspaceId: "w1",
      onClose: () => {
        window.__workspaceSearchOpenLog?.push("closed");
      },
      onOpenDocument: (documentId: string, findOptions?: EditorFindOptions) => {
        const request = findOptions
          ? `${findOptions.query}:${findOptions.caseSensitive}:${findOptions.wholeWord}:${findOptions.isRegex}`
          : "none";
        window.__workspaceSearchOpenLog?.push(`document:${documentId}:${request}`);
      },
      onOpenLoomBlock: (blockId: string) => {
        window.__workspaceSearchOpenLog?.push(`loom:${blockId}`);
      },
    }),
  );
}

export {};
