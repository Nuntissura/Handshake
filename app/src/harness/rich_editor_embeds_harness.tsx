// WP-KERNEL-009 / MT-244 — embeds + export + find/replace offline proof harness.
//
// A REAL product surface (built by vite.harness.config.ts from the same source
// tree) that mounts the INTEGRATED RichTextEditor with:
//   - typed media embeds: [[HS_images:img-ok]], [[video:vid-ok]],
//     [[HS_slideshow:s1,s2]], an UNRESOLVABLE [[HS_images:missing-asset]]
//     (typed error-state proof), and a non-media [[wp:…]] chip,
//   - an embedded Monaco code block containing the find target text,
//   - prose containing the find/replace target text.
//
// The embed context points at THIS page's own loopback origin: the offline
// Playwright spec (tests/dependency_policy/editor_embeds_export_find.spec.ts)
// serves the REAL Handshake asset route shapes
// (/workspaces/:ws/assets/:id[/content]) from the same 127.0.0.1 server that
// serves the built harness, with real bytes (a pngjs-encoded PNG and a
// MediaRecorder-recorded WebM) — REAL fetches over real HTTP, zero mocks in
// the product code, zero external requests (network kill-switch).
//
// The workspace id is fixed (`ws-embed-proof`) so the spec's asset server and
// this page agree without hidden coupling.

import { StrictMode, useState } from "react";
import { createRoot } from "react-dom/client";
import type { JSONContent } from "@tiptap/core";
import { RichTextEditor } from "../components/RichTextEditor";
import { makeCodeBlockAttrs } from "../lib/editor/code_block_serialization";
import { EDITOR_DEBUG_ENABLE_KEY } from "../lib/editor/visual_debug";
import "monaco-editor/min/vs/editor/editor.main.css";

// Iteration-3 M15: the debug payload is OFF in production bundles by default;
// this proof harness opts in so the offline Playwright spec can read
// __HS_EDITOR_DEBUG__ from the production-built bundle.
(globalThis as Record<string, unknown>)[EDITOR_DEBUG_ENABLE_KEY] = true;

export const EMBED_PROOF_WORKSPACE = "ws-embed-proof";

function link(refKind: string, refValue: string): JSONContent {
  return { type: "hsLink", attrs: { refKind, refValue, label: refValue, resolved: true } };
}

const INITIAL_DOC: JSONContent = {
  type: "doc",
  content: [
    { type: "heading", attrs: { level: 1 }, content: [{ type: "text", text: "CKC embed + export + find proof" }] },
    { type: "paragraph", content: [{ type: "text", text: "alpha beta alpha gamma" }] },
    { type: "paragraph", content: [{ type: "text", text: "Picture: " }, link("images", "img-ok")] },
    { type: "paragraph", content: [{ type: "text", text: "Video: " }, link("video", "vid-ok")] },
    { type: "paragraph", content: [{ type: "text", text: "Slideshow: " }, link("slideshow", "s1,s2")] },
    { type: "paragraph", content: [{ type: "text", text: "Broken: " }, link("images", "missing-asset")] },
    { type: "paragraph", content: [{ type: "text", text: "Chip: " }, link("wp", "WP-KERNEL-009")] },
    { type: "monacoCodeBlock", attrs: makeCodeBlockAttrs("typescript", "const alpha = 'alpha';") },
  ],
};

function HarnessShell() {
  const [doc, setDoc] = useState<JSONContent>(INITIAL_DOC);
  // The embed context resolves against THIS loopback origin — the spec's
  // server implements the real Handshake asset routes there.
  const [embedContext] = useState(() => ({
    workspaceId: EMBED_PROOF_WORKSPACE,
    apiBaseUrl: window.location.origin,
  }));

  return (
    <div data-testid="rich-editor-embeds-harness-root" style={{ padding: 16 }}>
      <h1 style={{ fontSize: 16 }}>Handshake embeds/export/find offline harness</h1>
      <RichTextEditor
        initialContent={doc}
        onChange={setDoc}
        embedContext={embedContext}
        documentTitle="Embed Proof Document"
      />
    </div>
  );
}

const root = createRoot(document.getElementById("root")!);
root.render(
  <StrictMode>
    <HarnessShell />
  </StrictMode>,
);
