// WP-KERNEL-009 MT-260 AILoomJobs — offline Playwright harness entry.
//
// Mounts the REAL LoomAiReviewPanel so an offline Playwright spec can prove the
// confirm-to-promote review flow against a real browser with ZERO external
// network (page.route serves the loom-ai REST responses). One bundle covers
// every case; the spec drives the run/accept/reject/accept-all controls and
// asserts the panel reflects the (intercepted) backend authority transitions.

import "../App.css";

import { createElement as h } from "react";
import { createRoot } from "react-dom/client";

import { LoomAiReviewPanel } from "./LoomAiReviewPanel";

declare global {
  interface Window {
    __mt260HarnessReady?: boolean;
    __mt260Config?: { ws?: string };
  }
}

const cfg = window.__mt260Config ?? {};
const workspaceId = cfg.ws ?? "ws-mt260";

const root = document.getElementById("harness-root");
if (root) {
  createRoot(root).render(
    h(
      "div",
      { "data-testid": "mt260-harness", style: { padding: 12, width: 960 } },
      h(LoomAiReviewPanel, { workspaceId }),
    ),
  );
  window.__mt260HarnessReady = true;
}

export {};
