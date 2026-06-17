import "../App.css";

import { createElement as h } from "react";
import { createRoot } from "react-dom/client";

import { BlockCollectionView } from "./BlockCollectionView";

declare global {
  interface Window {
    __hsBlockCollectionConfig?: { workspaceId: string; viewBlockId: string };
  }
}

const root = document.getElementById("harness-root");
const config = window.__hsBlockCollectionConfig;
if (root && config) {
  createRoot(root).render(
    h(BlockCollectionView, {
      workspaceId: config.workspaceId,
      viewBlockId: config.viewBlockId,
    }),
  );
}

export {};
