// WP-KERNEL-009 / MT-253 — offline source-control panel proof harness.
//
// Mounts the REAL SourceControlPanel (same component App.tsx wires into the
// `source-control` tab) so the offline Playwright spec drives status -> diff
// (real Monaco diff editor) -> stage -> commit -> branch -> log -> blame against
// a REAL temp git repo served by the mt253_source_control_fixture backend.
//
// The panel calls the fixed API base (lib/api BASE_URL); the Playwright spec
// rewrites those requests to the fixture's dynamic port. The harness only reads
// `?repo_path=` from the query so the panel starts pointed at the temp repo.

import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { SourceControlPanel } from "../components/SourceControlPanel";
import "../App.css";
import "monaco-editor/min/vs/editor/editor.main.css";

declare global {
  interface Window {
    __HS_SOURCE_CONTROL_HARNESS__?: { repoPath: string };
  }
}

const params = new URLSearchParams(window.location.search);
const repoPath = params.get("repo_path") ?? "";
window.__HS_SOURCE_CONTROL_HARNESS__ = { repoPath };

const root = createRoot(document.getElementById("root")!);
root.render(
  <StrictMode>
    <div data-testid="source-control-harness-root" style={{ padding: 16 }}>
      <SourceControlPanel initialRepoPath={repoPath} />
    </div>
  </StrictMode>,
);
