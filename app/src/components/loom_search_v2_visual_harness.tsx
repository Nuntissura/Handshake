// WP-KERNEL-009 MT-264 LoomSearchV2 — offline Playwright harness entry.
//
// Mounts the REAL LoomSearchV2Panel so an offline Playwright spec can prove the
// hybrid search GUI against a real browser with ZERO external network: the
// panel uses the REAL loomSearchV2 / createBlockView clients, and the spec
// intercepts those REST calls with page.route (a faithful in-memory model of
// the Postgres-native hybrid search). The harness records the block id passed
// to onOpenBlock so the spec can prove open-in-place (reference, not copy).

import "../App.css";

import { createElement as h } from "react";
import { createRoot } from "react-dom/client";

import { LoomSearchV2Panel } from "./LoomSearchV2Panel";

declare global {
  interface Window {
    __mt264HarnessReady?: boolean;
    __mt264Config?: { ws?: string };
    __mt264OpenedBlockId?: string | null;
  }
}

const cfg = window.__mt264Config ?? {};
const workspaceId = cfg.ws ?? "ws-mt264";
window.__mt264OpenedBlockId = null;

const root = document.getElementById("harness-root");
if (root) {
  createRoot(root).render(
    h(
      "div",
      { "data-testid": "mt264-harness", style: { padding: 12, width: 960 } },
      h(LoomSearchV2Panel, {
        workspaceId,
        onOpenBlock: (blockId: string) => {
          window.__mt264OpenedBlockId = blockId;
        },
      }),
    ),
  );
  window.__mt264HarnessReady = true;
}

export {};
