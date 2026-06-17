// WP-KERNEL-009 / MT-258 - Loom transclusion visual fixture.
//
// Mounts the real RichTextEditor with a loomTransclusion node and a workspace
// embed context. The Playwright proof fulfills the transclusion read-through
// endpoint and the source-document save endpoint at the browser route layer, so
// the fixture exercises the real NodeView:
//   - read-through rendering of the SOURCE content,
//   - "Edit source" routing the save to the SOURCE document,
//   - the host doc onChange JSON staying copy-free (only the atom node).

import { StrictMode, useCallback, useEffect } from "react";
import { createRoot } from "react-dom/client";
import type { JSONContent } from "@tiptap/core";
import { RichTextEditor } from "../components/RichTextEditor";
import { EDITOR_DEBUG_ENABLE_KEY } from "../lib/editor/visual_debug";
import "../App.css";

(globalThis as Record<string, unknown>)[EDITOR_DEBUG_ENABLE_KEY] = true;

declare global {
  interface Window {
    __LOOM_TRANSCLUSION_HARNESS__?: {
      docJson: JSONContent;
    };
  }
}

// The HOST document persists ONLY the atom node carrying the source block id.
const INITIAL_DOC: JSONContent = {
  type: "doc",
  content: [
    {
      type: "heading",
      attrs: { level: 1 },
      content: [{ type: "text", text: "Host document" }],
    },
    {
      type: "paragraph",
      content: [{ type: "text", text: "Host preamble paragraph." }],
    },
    {
      type: "loomTransclusion",
      attrs: { refValue: "block-source" },
    },
  ],
};

function HarnessShell() {
  const onChange = useCallback((next: JSONContent) => {
    window.__LOOM_TRANSCLUSION_HARNESS__ = { docJson: next };
  }, []);

  // Publish the INITIAL host doc immediately. Editing a transclusion routes to
  // the SOURCE document (a nested editor), so the host editor's onChange never
  // fires — the spec still needs the host JSON to assert the NO-COPY invariant.
  useEffect(() => {
    window.__LOOM_TRANSCLUSION_HARNESS__ = { docJson: INITIAL_DOC };
  }, []);

  return (
    <main data-testid="loom-transclusion-harness-root" className="loom-transclusion-harness">
      <div data-testid="loom-transclusion-capture" className="loom-transclusion-harness__capture">
        <RichTextEditor
          initialContent={INITIAL_DOC}
          onChange={onChange}
          embedContext={{ workspaceId: "ws-mt258-transclusion" }}
          documentTitle="MT-258 Loom transclusion host"
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
