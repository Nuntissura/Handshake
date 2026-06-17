// WP-KERNEL-009 / MT-258 - Loom hover-preview visual fixture.
//
// Mounts the real RichTextEditor with a previewable Loom hsLink and a workspace
// context. The Playwright proof fulfills the Loom-block API call at the browser
// route layer, so the fixture exercises the real NodeView hover behavior without
// requiring a live backend.

import { StrictMode, useCallback } from "react";
import { createRoot } from "react-dom/client";
import type { JSONContent } from "@tiptap/core";
import { RichTextEditor } from "../components/RichTextEditor";
import { EDITOR_DEBUG_ENABLE_KEY } from "../lib/editor/visual_debug";
import "../App.css";

(globalThis as Record<string, unknown>)[EDITOR_DEBUG_ENABLE_KEY] = true;

declare global {
  interface Window {
    __LOOM_HOVER_PREVIEW_HARNESS__?: {
      docJson: JSONContent;
    };
  }
}

const INITIAL_DOC: JSONContent = {
  type: "doc",
  content: [
    {
      type: "heading",
      attrs: { level: 1 },
      content: [{ type: "text", text: "Loom hover preview fixture" }],
    },
    {
      type: "paragraph",
      content: [
        { type: "text", text: "Hover the Loom note chip: " },
        {
          type: "hsLink",
          attrs: {
            refKind: "note",
            refValue: "block-alpha",
            label: "Alpha Loom note",
            resolved: true,
          },
        },
        { type: "text", text: " and verify the preview stays readable." },
      ],
    },
  ],
};

function HarnessShell() {
  const onChange = useCallback((next: JSONContent) => {
    window.__LOOM_HOVER_PREVIEW_HARNESS__ = { docJson: next };
  }, []);

  return (
    <main data-testid="loom-hover-preview-harness-root" className="loom-hover-preview-harness">
      <div data-testid="loom-hover-preview-capture" className="loom-hover-preview-harness__capture">
        <RichTextEditor
          initialContent={INITIAL_DOC}
          onChange={onChange}
          embedContext={{ workspaceId: "ws-mt258-preview" }}
          documentTitle="MT-258 Loom hover preview"
        />
      </div>
    </main>
  );
}

const root = createRoot(document.getElementById("root")!);
root.render(
  <StrictMode>
    <HarnessShell />
  </StrictMode>,
);
