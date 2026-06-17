// WP-KERNEL-009 / MT-258 - Loom search-operators visual fixture.
//
// Mounts the real WorkspaceSearchPanel. The Playwright proof fulfills the real
// /loom/graph-search route, inspecting the query params the panel sends for
// each operator (tag:, path:/folder:, kind:, mention:) and returning a filtered
// result set, so the test proves typing an operator filters the RENDERED
// results in the production search UI (offline, zero external network).

import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { WorkspaceSearchPanel } from "../components/WorkspaceSearchPanel";
import type { EditorFindOptions } from "../lib/editor/editor_find_request";
import "../App.css";

declare global {
  interface Window {
    __loomSearchOpenLog?: string[];
  }
}

const workspaceId = "w1";

function HarnessShell() {
  window.__loomSearchOpenLog ??= [];
  return (
    <main data-testid="loom-search-operators-harness-root" className="loom-search-operators-harness">
      <div data-testid="loom-search-operators-capture" className="loom-search-operators-harness__capture">
        <WorkspaceSearchPanel
          open
          workspaceId={workspaceId}
          onClose={() => window.__loomSearchOpenLog?.push("closed")}
          onOpenDocument={(documentId: string, findOptions?: EditorFindOptions) =>
            window.__loomSearchOpenLog?.push(`document:${documentId}:${findOptions?.query ?? "none"}`)
          }
          onOpenLoomBlock={(blockId: string) => window.__loomSearchOpenLog?.push(`loom:${blockId}`)}
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
