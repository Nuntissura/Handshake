import "../App.css";

import { createElement as h } from "react";
import { createRoot } from "react-dom/client";

import { LoomCanvasBoard } from "./LoomCanvasBoard";

declare global {
  interface Window {
    __hsLoomCanvasConfig?: { workspaceId: string; canvasBlockId: string };
  }
}

const root = document.getElementById("harness-root");
const config = window.__hsLoomCanvasConfig;
if (root && config) {
  createRoot(root).render(
    h(LoomCanvasBoard, {
      workspaceId: config.workspaceId,
      canvasBlockId: config.canvasBlockId,
    }),
  );
}

export {};
