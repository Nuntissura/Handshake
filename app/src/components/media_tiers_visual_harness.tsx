// WP-KERNEL-009 MT-259 MediaCacheTiers — offline Playwright harness entry.
//
// Mounts the REAL product media-embed renderer (HsLinkView.MediaEmbed) for the
// named GUI capabilities so an offline Playwright spec can prove them against a
// real browser with ZERO external network (page.route serves the bytes):
//   - a grid/album of many members loading tier=thumb (fluid scroll),
//   - a single image that upgrades to full-res only on click,
//   - a video whose src is the Range-capable content endpoint + a poster tier,
//   - the preview_status surface with a FAILED tier and a visible retry that
//     POSTs the retry endpoint.
//
// The embed resolves against the default runtime fetch (no injected fetchImpl),
// so every request is a real browser request that Playwright intercepts; this
// is what makes it a runtime GUI proof, not a unit test.

import "../App.css";

import { createElement as h } from "react";
import { createRoot } from "react-dom/client";

import { MediaEmbed } from "./HsLinkView";
import type { EmbedResolverContext } from "../lib/editor/embed_assets";

declare global {
  interface Window {
    __mt259HarnessReady?: boolean;
    __mt259Config?: {
      base?: string;
      ws?: string;
      ref?: string;
      kind?: string;
    };
  }
}

// The harness reads its scenario from an injected global (set by the Playwright
// spec before the bundle loads) so one bundle covers every case
// (kind=images|video|album, ref=<asset id | collection: id | comma list>).
const cfg = window.__mt259Config ?? {};
const apiBaseUrl = cfg.base ?? "http://127.0.0.1:38259";
const workspaceId = cfg.ws ?? "ws-mt259";
const refValue = cfg.ref ?? "";
const kindParam = cfg.kind ?? "images";

const context: EmbedResolverContext = { workspaceId, apiBaseUrl };

const root = document.getElementById("harness-root");
if (root) {
  createRoot(root).render(
    h(
      "div",
      { "data-testid": "mt259-harness", style: { padding: 12, width: 980 } },
      h(MediaEmbed, {
        kind: kindParam as never,
        refValue,
        label: "mt259",
        context,
      }),
    ),
  );
  window.__mt259HarnessReady = true;
}

export {};
