// WP-KERNEL-009 / MT-258 - Loom bookmarks visual fixture.
//
// Mounts the real WorkspaceSidebar and LoomBlockPanel together. Playwright
// fulfills the backend routes so add/remove/navigation flows exercise the
// production API wrappers and component event wiring without a live backend.

import { StrictMode, useState } from "react";
import { createRoot } from "react-dom/client";
import { LoomBlockPanel } from "../components/LoomBlockPanel";
import { WorkspaceSidebar } from "../components/WorkspaceSidebar";
import "../App.css";

const workspaceId = "w1";

function initialBlockId(): string {
  if (typeof window === "undefined") return "block-unpinned";
  const param = new URLSearchParams(window.location.search).get("block");
  return param && param.trim() ? param.trim() : "block-unpinned";
}

function HarnessShell() {
  const [selectedBlockId, setSelectedBlockId] = useState(initialBlockId);
  const [openLog, setOpenLog] = useState<string[]>([]);

  const openLoomBlock = (blockId: string) => {
    setSelectedBlockId(blockId);
    setOpenLog((prev) => [...prev, blockId]);
  };

  return (
    <main data-testid="loom-bookmarks-harness-root" className="loom-bookmarks-harness">
      <div data-testid="loom-bookmarks-capture" className="loom-bookmarks-harness__capture">
        <WorkspaceSidebar
          refreshKey={0}
          activeWorkspaceId={workspaceId}
          onSelectWorkspace={() => undefined}
          onSelectDocument={() => undefined}
          onSelectCanvas={() => undefined}
          onOpenLoomBlock={openLoomBlock}
          selectedDocumentId={null}
          selectedCanvasId={null}
          onWorkspaceDeleted={() => undefined}
        />
        <section className="loom-bookmarks-harness__panel" aria-label="Selected Loom block">
          <p className="app-eyebrow">Selected Block</p>
          <p data-testid="loom-bookmarks.selected-block">{selectedBlockId}</p>
          <p data-testid="loom-bookmarks.open-log">{openLog.join("|") || "none"}</p>
          <LoomBlockPanel workspaceId={workspaceId} blockId={selectedBlockId} />
        </section>
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
