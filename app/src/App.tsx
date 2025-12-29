import "./App.css";
import { SystemStatus } from "./components/SystemStatus";
import { useState } from "react";
import { WorkspaceSidebar } from "./components/WorkspaceSidebar";
import { DocumentView } from "./components/DocumentView";
import { CanvasView } from "./components/CanvasView";
import { DebugPanel } from "./components/DebugPanel";
import { BundleScopeInput } from "./lib/api";

import { FlightRecorderView } from "./components/FlightRecorderView";
import {
  EvidenceDrawer,
  EvidenceSelection,
  DebugBundleExport,
  JobsView,
  ProblemsView,
  TimelineView,
} from "./components/operator";

function App() {
  const [selectedDocumentId, setSelectedDocumentId] = useState<string | null>(null);
  const [selectedCanvasId, setSelectedCanvasId] = useState<string | null>(null);
  const [activeView, setActiveView] = useState<
    "workspace" | "flight-recorder" | "problems" | "jobs" | "timeline"
  >("workspace");
  const [refreshKey, setRefreshKey] = useState<number>(0);
  const [selection, setSelection] = useState<EvidenceSelection | null>(null);
  const [exportOpen, setExportOpen] = useState(false);
  const [exportScope, setExportScope] = useState<BundleScopeInput | null>(null);

  return (
    <main className="app-shell">
      <div className="app-layout">
        <header className="app-header">
          <div>
            <p className="app-eyebrow">Handshake</p>
            <h1 className="app-title">Desktop Shell</h1>
            <p className="app-subtitle">Coordinator, workspaces, documents, and canvases.</p>
          </div>
          <div className="app-nav">
            <button 
              className={activeView === "workspace" ? "active" : ""} 
              onClick={() => setActiveView("workspace")}
            >
              Workspace
            </button>
            <button 
              className={activeView === "flight-recorder" ? "active" : ""} 
              onClick={() => setActiveView("flight-recorder")}
            >
              Flight Recorder
            </button>
            <button 
              className={activeView === "problems" ? "active" : ""} 
              onClick={() => setActiveView("problems")}
            >
              Problems
            </button>
            <button 
              className={activeView === "jobs" ? "active" : ""} 
              onClick={() => setActiveView("jobs")}
            >
              Jobs
            </button>
            <button 
              className={activeView === "timeline" ? "active" : ""} 
              onClick={() => setActiveView("timeline")}
            >
              Timeline
            </button>
          </div>
          <SystemStatus />
        </header>

        <div className="app-body">
          {activeView === "workspace" ? (
            <>
              <WorkspaceSidebar
                refreshKey={refreshKey}
                selectedDocumentId={selectedDocumentId}
                selectedCanvasId={selectedCanvasId}
                onSelectDocument={(id) => {
                  setSelectedDocumentId(id);
                  if (id !== null) setSelectedCanvasId(null);
                }}
                onSelectCanvas={(id) => {
                  setSelectedCanvasId(id);
                  if (id !== null) setSelectedDocumentId(null);
                }}
                onWorkspaceDeleted={() => {
                  setSelectedDocumentId(null);
                  setSelectedCanvasId(null);
                  setRefreshKey((k) => k + 1);
                }}
              />

              <div className="content-panel">
                <DebugPanel />
                <div className="content-main">
                  {selectedDocumentId ? (
                    <DocumentView
                      documentId={selectedDocumentId}
                      onDeleted={() => {
                        setSelectedDocumentId(null);
                      }}
                    />
                  ) : selectedCanvasId ? (
                    <CanvasView
                      canvasId={selectedCanvasId}
                      onDeleted={() => {
                        setSelectedCanvasId(null);
                      }}
                    />
                  ) : (
                    <div className="content-card">
                      <h2>Welcome</h2>
                      <p className="muted">
                        Select or create a workspace, then add documents or canvases to view their details.
                      </p>
                    </div>
                  )}
                </div>
              </div>
            </>
          ) : activeView === "flight-recorder" ? (
            <div className="content-panel content-panel--full">
              <FlightRecorderView />
            </div>
          ) : activeView === "problems" ? (
            <div className="content-panel content-panel--full">
              <ProblemsView onSelect={setSelection} />
            </div>
          ) : activeView === "jobs" ? (
            <div className="content-panel content-panel--full">
              <JobsView onSelect={setSelection} />
            </div>
          ) : (
            <div className="content-panel content-panel--full">
              <TimelineView onSelect={setSelection} />
            </div>
          )}
        </div>
      </div>
      <EvidenceDrawer
        selection={selection}
        onClose={() => setSelection(null)}
        onExport={(sel) => {
          let scope: BundleScopeInput | null = null;
          if (sel.kind === "diagnostic") {
            scope = { kind: "problem", problem_id: sel.diagnostic.id };
          } else if (sel.kind === "event") {
            scope = { kind: "job", job_id: sel.event.job_id ?? "" };
          }
          setExportScope(scope);
          setExportOpen(true);
        }}
      />
      {exportOpen && (
        <DebugBundleExport
          isOpen={exportOpen}
          defaultScope={exportScope ?? { kind: "job", job_id: "" }}
          onClose={() => setExportOpen(false)}
        />
      )}
    </main>
  );
}

export default App;
