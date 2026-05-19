import "./App.css";
import { SystemStatus } from "./components/SystemStatus";
import { useEffect, useState } from "react";
import { WorkspaceSidebar } from "./components/WorkspaceSidebar";
import { DocumentView } from "./components/DocumentView";
import { CanvasView } from "./components/CanvasView";
import { DebugPanel } from "./components/DebugPanel";
import { KernelDccProjectionView } from "./components/KernelDccProjectionView";
import { InferenceLab } from "./components/inference_lab";
import { FontManagerView } from "./components/FontManagerView";
import { MediaDownloaderView } from "./components/MediaDownloaderView";
import {
  getKernelDccProjection,
  triggerKernelDccAction,
  type BundleScopeInput,
  type KernelDccProjectionSurfaceV1,
} from "./lib/api";
import { AiJobsDrawer } from "./components/AiJobsDrawer";
import { ViewModeToggle } from "./components/ViewModeToggle";
import type { ViewMode } from "./lib/viewMode";
import { loadViewModeFromStorage, saveViewModeToStorage } from "./lib/viewMode";

import { FlightRecorderView } from "./components/FlightRecorderView";
import {
  EvidenceDrawer,
  EvidenceSelection,
  DebugBundleExport,
  GovernancePackExport,
  JobsView,
  ProblemsView,
  TimelineView,
  Ans001TimelineDrawer,
} from "./components/operator";

function App() {
  const [selectedDocumentId, setSelectedDocumentId] = useState<string | null>(null);
  const [selectedCanvasId, setSelectedCanvasId] = useState<string | null>(null);
  const [viewMode, setViewMode] = useState<ViewMode>(() => loadViewModeFromStorage());
  const [activeView, setActiveView] = useState<
    | "workspace"
    | "media-downloader"
    | "fonts"
    | "flight-recorder"
    | "kernel-dcc"
    | "inference-lab"
    | "problems"
    | "jobs"
    | "timeline"
  >("workspace");
  const [refreshKey, setRefreshKey] = useState<number>(0);
  const [selection, setSelection] = useState<EvidenceSelection | null>(null);
  const [exportScope, setExportScope] = useState<BundleScopeInput | null>(null);
  const [governancePackExportOpen, setGovernancePackExportOpen] = useState(false);
  const [focusJobId, setFocusJobId] = useState<string | null>(null);
  const [timelineNav, setTimelineNav] = useState<{ job_id?: string; wsid?: string; event_id?: string } | null>(null);
  const [timelineWindow, setTimelineWindow] = useState<{ start: string; end: string; wsid?: string } | null>(null);
  const [ans001TimelineOpen, setAns001TimelineOpen] = useState(false);
  const [kernelDccSurface, setKernelDccSurface] = useState<KernelDccProjectionSurfaceV1 | null>(null);
  const [kernelDccLoading, setKernelDccLoading] = useState(false);
  const [kernelDccError, setKernelDccError] = useState<string | null>(null);

  useEffect(() => {
    saveViewModeToStorage(viewMode);
  }, [viewMode]);

  const loadKernelDccProjection = () => {
    setKernelDccLoading(true);
    setKernelDccError(null);
    setKernelDccSurface(null);

    getKernelDccProjection()
      .then((surface) => {
        setKernelDccSurface(surface);
      })
      .catch((err) => {
        setKernelDccSurface(null);
        setKernelDccError(err instanceof Error ? err.message : "Failed to load Kernel DCC projection");
      })
      .finally(() => {
        setKernelDccLoading(false);
      });
  };

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
              className={activeView === "media-downloader" ? "active" : ""}
              onClick={() => setActiveView("media-downloader")}
            >
              Media Downloader
            </button>
            <button className={activeView === "fonts" ? "active" : ""} onClick={() => setActiveView("fonts")}>
              Fonts
            </button>
            <button 
              className={activeView === "flight-recorder" ? "active" : ""} 
              onClick={() => setActiveView("flight-recorder")}
            >
              Flight Recorder
            </button>
            <button
              className={activeView === "kernel-dcc" ? "active" : ""}
              onClick={() => {
                setActiveView("kernel-dcc");
                loadKernelDccProjection();
              }}
            >
              Kernel DCC
            </button>
            <button
              className={activeView === "inference-lab" ? "active" : ""}
              onClick={() => setActiveView("inference-lab")}
            >
              Inference Lab
            </button>
            <button
              className={activeView === "problems" ? "active" : ""}
              onClick={() => setActiveView("problems")}
            >
              Problems
            </button>
            <button 
              className={activeView === "jobs" ? "active" : ""} 
              onClick={() => {
                setFocusJobId(null);
                setActiveView("jobs");
              }}
            >
              Jobs
            </button>
            <button 
              className={activeView === "timeline" ? "active" : ""} 
              onClick={() => {
                setTimelineNav(null);
                setActiveView("timeline");
              }}
            >
              Timeline
            </button>
            <button onClick={() => setGovernancePackExportOpen(true)}>Gov Pack Export</button>
            <button onClick={() => setAns001TimelineOpen(true)}>ANS-001 Timeline</button>
          </div>
          <div className="app-header-right">
            <ViewModeToggle value={viewMode} onChange={setViewMode} />
            <SystemStatus />
          </div>
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
          ) : activeView === "fonts" ? (
            <div className="content-panel content-panel--full">
              <FontManagerView />
            </div>
          ) : activeView === "media-downloader" ? (
            <div className="content-panel content-panel--full">
              <MediaDownloaderView />
            </div>
          ) : activeView === "flight-recorder" ? (
            <div className="content-panel content-panel--full">
              <FlightRecorderView />
            </div>
          ) : activeView === "kernel-dcc" ? (
            <div className="content-panel content-panel--full">
              {kernelDccLoading ? (
                <div className="content-card">
                  <h2>Loading Kernel DCC projection...</h2>
                </div>
              ) : kernelDccError ? (
                <div className="content-card">
                  <h2>Kernel DCC projection unavailable</h2>
                  <p className="muted">{kernelDccError}</p>
                </div>
              ) : kernelDccSurface ? (
                <KernelDccProjectionView surface={kernelDccSurface} onTriggerCatalogAction={triggerKernelDccAction} />
              ) : (
                <div className="content-card">
                  <h2>Kernel DCC projection unavailable</h2>
                </div>
              )}
            </div>
          ) : activeView === "inference-lab" ? (
            <div className="content-panel content-panel--full">
              <InferenceLab />
            </div>
          ) : activeView === "problems" ? (
            <div className="content-panel content-panel--full">
              <ProblemsView onSelect={setSelection} />
            </div>
          ) : activeView === "jobs" ? (
            <div className="content-panel content-panel--full">
              <JobsView onSelect={setSelection} focusJobId={focusJobId} />
            </div>
          ) : (
            <div className="content-panel content-panel--full">
              <TimelineView
                onSelect={setSelection}
                navigation={timelineNav}
                onTimeWindowChange={setTimelineWindow}
              />
            </div>
          )}
        </div>
      </div>
      <EvidenceDrawer
        selection={selection}
        onClose={() => setSelection(null)}
        timelineWindow={timelineWindow ?? undefined}
        onExportScope={(scope) => setExportScope(scope)}
        onNavigateToJob={(jobId) => {
          setActiveView("jobs");
          setFocusJobId(jobId);
        }}
        onNavigateToTimeline={(nav) => {
          setActiveView("timeline");
          setTimelineNav({ ...nav });
        }}
      />
      <Ans001TimelineDrawer isOpen={ans001TimelineOpen} onClose={() => setAns001TimelineOpen(false)} />
      {exportScope && (
        <DebugBundleExport
          isOpen={true}
          defaultScope={exportScope}
          onClose={() => setExportScope(null)}
        />
      )}
      {governancePackExportOpen && (
        <GovernancePackExport
          isOpen={true}
          onClose={() => setGovernancePackExportOpen(false)}
        />
      )}
      <AiJobsDrawer />
    </main>
  );
}

export default App;
