import "./App.css";
import { SystemStatus } from "./components/SystemStatus";
import { useCallback, useEffect, useRef, useState } from "react";
import { WorkspaceSidebar } from "./components/WorkspaceSidebar";
import { DocumentView } from "./components/DocumentView";
import { CanvasView } from "./components/CanvasView";
import { DebugPanel } from "./components/DebugPanel";
import { KernelDccProjectionView } from "./components/KernelDccProjectionView";
import { InferenceLab } from "./components/inference_lab";
import { FontManagerView } from "./components/FontManagerView";
import { MediaDownloaderView } from "./components/MediaDownloaderView";
import { ModelRuntimePanel } from "./components/model_runtime_panel";
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

const DEFAULT_SPLIT_WEIGHTS = {
  cols: [0.5, 0.5],
  rows: [0.5, 0.5],
} as const;

type ModuleId = "MAIN" | "CKC" | "INGEST" | "STAGE" | "LAB" | "STUDIO";
type PaneType =
  | "workspace"
  | "media-downloader"
  | "fonts"
  | "flight-recorder"
  | "kernel-dcc"
  | "inference-lab"
  | "model-runtime"
  | "problems"
  | "jobs"
  | "timeline";
type SplitOrientation = "vertical" | "horizontal";

type SplitWeights = {
  cols: [number, number];
  rows: [number, number];
};

type ProjectMetadata = {
  id: string;
  name: string;
  path: string;
};

type FileMetadata = {
  id: string;
  name: string;
};

type PaneState = {
  id: string;
  projectRef: string;
  tabs: readonly PaneType[];
  activeTab: PaneType;
  locked: boolean;
};

type SplitDragState = {
  orientation: SplitOrientation;
  pointerId: number;
  startX: number;
  startY: number;
  startWeights: SplitWeights;
  width: number;
  height: number;
} | null;

const projects: ProjectMetadata[] = [
  { id: "project-alpha", name: "Alpha", path: "Projects/Alpha" },
  { id: "project-beta", name: "Beta", path: "Projects/Beta" },
  { id: "project-gamma", name: "Gamma", path: "Projects/Gamma" },
];

const projectFiles: Record<string, FileMetadata[]> = {
  "project-alpha": [
    { id: "alpha-overview", name: "overview.md" },
    { id: "alpha-research", name: "research.md" },
  ],
  "project-beta": [
    { id: "beta-roadmap", name: "roadmap.md" },
    { id: "beta-ops", name: "ops-notes.md" },
  ],
  "project-gamma": [{ id: "gamma-review", name: "design-review.md" }],
};

const paneTabLabels: Record<PaneType, string> = {
  workspace: "Workspace",
  "media-downloader": "Media",
  fonts: "Fonts",
  "flight-recorder": "Flight",
  "kernel-dcc": "Kernel DCC",
  "inference-lab": "Inference",
  "model-runtime": "Model Runtime",
  problems: "Problems",
  jobs: "Jobs",
  timeline: "Timeline",
};

const moduleIds: ModuleId[] = ["MAIN", "CKC", "INGEST", "STAGE", "LAB", "STUDIO"];

const moduleToPreferredTabs: Record<ModuleId, PaneType[]> = {
  MAIN: ["workspace", "kernel-dcc", "inference-lab", "model-runtime"],
  CKC: ["kernel-dcc", "problems", "jobs", "timeline"],
  INGEST: ["media-downloader", "fonts", "jobs", "timeline"],
  STAGE: ["flight-recorder", "jobs", "timeline", "problems"],
  LAB: ["inference-lab", "model-runtime", "problems", "jobs"],
  STUDIO: ["model-runtime", "kernel-dcc", "timeline", "jobs"],
};

const paneDefaults: Omit<PaneState, "projectRef">[] = [
  {
    id: "pane-main-left",
    activeTab: "workspace",
    tabs: ["workspace", "kernel-dcc", "problems"] as const,
    locked: false,
  },
  {
    id: "pane-main-right",
    activeTab: "media-downloader",
    tabs: ["media-downloader", "fonts", "jobs"] as const,
    locked: false,
  },
  {
    id: "pane-bottom-left",
    activeTab: "inference-lab",
    tabs: ["inference-lab", "flight-recorder", "timeline"] as const,
    locked: false,
  },
  {
    id: "pane-bottom-right",
    activeTab: "model-runtime",
    tabs: ["model-runtime", "timeline", "problems"] as const,
    locked: false,
  },
];

function clamp(value: number, min: number, max: number): number {
  return Math.max(min, Math.min(max, value));
}

function serializeSplitWeights(weights: SplitWeights): string {
  return JSON.stringify({
    cols: weights.cols.map((value) => Number(value.toFixed(3))),
    rows: weights.rows.map((value) => Number(value.toFixed(3))),
  });
}

function App() {
  const [selectedDocumentId, setSelectedDocumentId] = useState<string | null>(null);
  const [selectedCanvasId, setSelectedCanvasId] = useState<string | null>(null);
  const [viewMode, setViewMode] = useState<ViewMode>(() => loadViewModeFromStorage());
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

  const [activeModule, setActiveModule] = useState<ModuleId>("MAIN");
  const [activeProjectId, setActiveProjectId] = useState(projects[0].id);
  const [splitWeights, setSplitWeights] = useState<SplitWeights>(DEFAULT_SPLIT_WEIGHTS);
  const [projectDrawerOpen, setProjectDrawerOpen] = useState(true);
  const [fileDrawerOpen, setFileDrawerOpen] = useState(true);
  const [bottomDrawerOpen, setBottomDrawerOpen] = useState(true);
  const [panes, setPanes] = useState<PaneState[]>(() =>
    paneDefaults.map((pane) => ({
      ...pane,
      projectRef: projects[0].id,
    })),
  );

  const paneAreaRef = useRef<HTMLDivElement>(null);
  const dragState = useRef<SplitDragState>(null);

  const activeProject = projects.find((project) => project.id === activeProjectId) ?? projects[0];
  const activeFiles = projectFiles[activeProjectId] ?? [];
  const isBottomDrawerOpen = String(bottomDrawerOpen);
  const isProjectDrawerOpen = String(projectDrawerOpen);
  const isFileDrawerOpen = String(fileDrawerOpen);

  useEffect(() => {
    saveViewModeToStorage(viewMode);
  }, [viewMode]);

  const loadKernelDccProjection = useCallback(() => {
    if (kernelDccLoading || kernelDccSurface) {
      return;
    }
    setKernelDccLoading(true);
    setKernelDccError(null);
    setKernelDccSurface(null);

    getKernelDccProjection()
      .then((surface) => {
        setKernelDccSurface(surface);
      })
      .catch((error) => {
        setKernelDccSurface(null);
        setKernelDccError(error instanceof Error ? error.message : "Failed to load Kernel DCC projection");
      })
      .finally(() => {
        setKernelDccLoading(false);
      });
  }, [kernelDccLoading, kernelDccSurface]);

  useEffect(() => {
    const needsKernelTab = panes.some((pane) => pane.activeTab === "kernel-dcc");
    if (needsKernelTab) {
      loadKernelDccProjection();
    }
  }, [loadKernelDccProjection, panes]);

  const setProject = (projectId: string) => {
    setActiveProjectId(projectId);
    setPanes((current) =>
      current.map((pane) => ({
        ...pane,
        projectRef: projectId,
      })),
    );
  };

  const setPaneTab = (paneId: string, tab: PaneType) => {
    setPanes((current) =>
      current.map((pane) => (pane.id === paneId && pane.tabs.includes(tab) ? { ...pane, activeTab: tab } : pane)),
    );
    if (tab === "kernel-dcc") {
      loadKernelDccProjection();
    }
  };

  const setModule = (moduleId: ModuleId) => {
    setActiveModule(moduleId);
    setPanes((current) =>
      current.map((pane, index) => {
        const requestedTab = moduleToPreferredTabs[moduleId][index];
        return pane.tabs.includes(requestedTab) ? { ...pane, activeTab: requestedTab } : pane;
      }),
    );
    if (moduleToPreferredTabs[moduleId].includes("kernel-dcc")) {
      loadKernelDccProjection();
    }
  };

  const handleSplitterPointerDown = (
    event: React.PointerEvent<HTMLDivElement>,
    orientation: SplitOrientation,
    locked: boolean,
  ) => {
    if (locked) {
      return;
    }
    const area = paneAreaRef.current;
    if (!area) {
      return;
    }
    const bounds = area.getBoundingClientRect();
    dragState.current = {
      orientation,
      pointerId: event.pointerId,
      startX: event.clientX,
      startY: event.clientY,
      startWeights: { ...splitWeights },
      width: bounds.width,
      height: bounds.height,
    };
    event.currentTarget.setPointerCapture?.(event.pointerId);
    event.preventDefault();
  };

  const handleSplitterPointerMove = (event: React.PointerEvent<HTMLDivElement>, orientation: SplitOrientation) => {
    const activeDrag = dragState.current;
    if (!activeDrag || activeDrag.orientation !== orientation) {
      return;
    }
    if (activeDrag.pointerId !== event.pointerId) {
      return;
    }
    event.preventDefault();

    if (orientation === "vertical") {
      const raw = activeDrag.startWeights.cols[0] + (event.clientX - activeDrag.startX) / Math.max(1, activeDrag.width);
      const nextLeft = clamp(raw, 0.18, 0.82);
      setSplitWeights((current) => ({
        ...current,
        cols: [Number(nextLeft.toFixed(3)), Number((1 - nextLeft).toFixed(3))],
      }));
      return;
    }

    const raw = activeDrag.startWeights.rows[0] + (event.clientY - activeDrag.startY) / Math.max(1, activeDrag.height);
    const nextTop = clamp(raw, 0.18, 0.82);
    setSplitWeights((current) => ({
      ...current,
      rows: [Number(nextTop.toFixed(3)), Number((1 - nextTop).toFixed(3))],
    }));
  };

  const handleSplitterPointerUp = (event: React.PointerEvent<HTMLDivElement>) => {
    const activeDrag = dragState.current;
    if (activeDrag && activeDrag.pointerId === event.pointerId) {
      event.currentTarget.releasePointerCapture?.(event.pointerId);
      dragState.current = null;
    }
  };

  const handleSplitterKey = (orientation: SplitOrientation, locked: boolean, event: React.KeyboardEvent<HTMLDivElement>) => {
    if (locked) {
      return;
    }
    const step = 0.03;

    if (orientation === "vertical") {
      if (event.key === "ArrowLeft") {
        setSplitWeights((current) => ({
          ...current,
          cols: [
            Number(clamp(current.cols[0] - step, 0.18, 0.82).toFixed(3)),
            Number(clamp(1 - clamp(current.cols[0] - step, 0.18, 0.82), 0.18, 0.82).toFixed(3)),
          ],
        }));
      } else if (event.key === "ArrowRight") {
        setSplitWeights((current) => ({
          ...current,
          cols: [
            Number(clamp(current.cols[0] + step, 0.18, 0.82).toFixed(3)),
            Number(clamp(1 - clamp(current.cols[0] + step, 0.18, 0.82), 0.18, 0.82).toFixed(3)),
          ],
        }));
      } else {
        return;
      }
      event.preventDefault();
      return;
    }

    if (event.key === "ArrowUp") {
      setSplitWeights((current) => ({
        ...current,
        rows: [
          Number(clamp(current.rows[0] - step, 0.18, 0.82).toFixed(3)),
          Number(clamp(1 - clamp(current.rows[0] - step, 0.18, 0.82), 0.18, 0.82).toFixed(3)),
        ],
      }));
      event.preventDefault();
      return;
    }

    if (event.key === "ArrowDown") {
      setSplitWeights((current) => ({
        ...current,
        rows: [
          Number(clamp(current.rows[0] + step, 0.18, 0.82).toFixed(3)),
          Number(clamp(1 - clamp(current.rows[0] + step, 0.18, 0.82), 0.18, 0.82).toFixed(3)),
        ],
      }));
      event.preventDefault();
    }
  };

  const isVerticalSplitterLocked = panes.some((pane) => pane.locked);
  const isHorizontalSplitterLocked = panes.some((pane) => pane.locked);

  const paneCells = [
    { style: { gridColumn: "1", gridRow: "1" }, title: "Top Left" },
    { style: { gridColumn: "2", gridRow: "1" }, title: "Top Right" },
    { style: { gridColumn: "1", gridRow: "2" }, title: "Bottom Left" },
    { style: { gridColumn: "2", gridRow: "2" }, title: "Bottom Right" },
  ] as const;

  const renderPaneContent = (pane: PaneState) => {
    if (pane.activeTab === "workspace") {
      return (
        <div className="main-workspace-layout">
          <WorkspaceSidebar
            refreshKey={0}
            selectedDocumentId={selectedDocumentId}
            selectedCanvasId={selectedCanvasId}
            onSelectDocument={(id) => {
              setSelectedDocumentId(id);
              if (id !== null) {
                setSelectedCanvasId(null);
              }
            }}
            onSelectCanvas={(id) => {
              setSelectedCanvasId(id);
              if (id !== null) {
                setSelectedDocumentId(null);
              }
            }}
            onWorkspaceDeleted={() => {
              setSelectedDocumentId(null);
              setSelectedCanvasId(null);
            }}
          />
          <div className="main-workspace-main">
            <DebugPanel />
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
                <h2>Workspace</h2>
                <p className="muted">Select or create content in the active project to begin.</p>
              </div>
            )}
          </div>
        </div>
      );
    }
    if (pane.activeTab === "media-downloader") {
      return (
        <div className="content-panel content-panel--full">
          <MediaDownloaderView />
        </div>
      );
    }
    if (pane.activeTab === "fonts") {
      return (
        <div className="content-panel content-panel--full">
          <FontManagerView />
        </div>
      );
    }
    if (pane.activeTab === "flight-recorder") {
      return (
        <div className="content-panel content-panel--full">
          <FlightRecorderView />
        </div>
      );
    }
    if (pane.activeTab === "kernel-dcc") {
      return (
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
      );
    }
    if (pane.activeTab === "inference-lab") {
      return (
        <div className="content-panel content-panel--full">
          <InferenceLab />
        </div>
      );
    }
    if (pane.activeTab === "model-runtime") {
      return (
        <div className="content-panel content-panel--full">
          <ModelRuntimePanel />
        </div>
      );
    }
    if (pane.activeTab === "problems") {
      return (
        <div className="content-panel content-panel--full">
          <ProblemsView onSelect={setSelection} />
        </div>
      );
    }
    if (pane.activeTab === "jobs") {
      return (
        <div className="content-panel content-panel--full">
          <JobsView onFocusJobId={focusJobId} onSelect={setSelection} />
        </div>
      );
    }
    return (
      <div className="content-panel content-panel--full">
        <TimelineView onSelect={setSelection} navigation={timelineNav} onTimeWindowChange={setTimelineWindow} />
      </div>
    );
  };

  return (
    <main
      className="app-shell app-shell--main-v1"
      data-testid="main-window-shell"
      data-stable-layout="v1"
      data-active-project-id={activeProjectId}
      data-split-weights={serializeSplitWeights(splitWeights)}
      data-project-drawer-open={isProjectDrawerOpen}
      data-file-drawer-open={isFileDrawerOpen}
      data-bottom-drawer-open={isBottomDrawerOpen}
      data-active-module={activeModule}
    >
      <header className="app-header">
        <div>
          <p className="app-eyebrow">Handshake</p>
          <h1 className="app-title">GUI Main Window</h1>
          <p className="app-subtitle">Module-aware panes, deterministic layout, and automation-stable DOM state.</p>
        </div>
        <div className="app-header-right">
          <ViewModeToggle value={viewMode} onChange={setViewMode} />
          <SystemStatus />
          <button onClick={() => setGovernancePackExportOpen(true)} data-testid="main-action.gov-pack-export">
            Gov Pack Export
          </button>
          <button onClick={() => setAns001TimelineOpen(true)} data-testid="main-action.ans001">
            ANS-001 Timeline
          </button>
        </div>
      </header>

      <div id="main-window" data-stable-id="main-window" className="main-window-surface">
        <aside className="main-left-rail">
          <section className="main-project-tabs" data-testid="project-tabs" data-stable-id="project-tabs">
            {projects.map((project) => (
              <button
                key={project.id}
                className={project.id === activeProjectId ? "main-project-tabs__button--active" : undefined}
                data-testid={`project-tab.${project.id}`}
                data-stable-id={`project-tab:${project.id}`}
                onClick={() => setProject(project.id)}
                type="button"
              >
                {project.name}
              </button>
            ))}
          </section>

          <section className={`main-drawer main-project-drawer ${isProjectDrawerOpen ? "" : "main-drawer--closed"}`} data-stable-id="drawer:project">
            <header className="main-drawer__header">
              <div className="main-drawer__title">Projects</div>
              <button
                type="button"
                data-testid="project-drawer.toggle"
                data-stable-id="drawer:project:toggle"
                onClick={() => setProjectDrawerOpen((value) => !value)}
              >
                {projectDrawerOpen ? "Hide" : "Show"}
              </button>
            </header>
            {projectDrawerOpen ? (
              <div className="main-drawer__body">
                {projects.map((project) => (
                  <button
                    key={`project-${project.id}`}
                    className="main-drawer__item"
                    data-testid={`project-drawer.item.${project.id}`}
                    data-stable-id={`drawer:project:${project.id}`}
                    onClick={() => setProject(project.id)}
                    type="button"
                  >
                    <span>{project.name}</span>
                    <span className="main-drawer__path">{project.path}</span>
                  </button>
                ))}
              </div>
            ) : null}
          </section>

          <section className={`main-drawer main-file-drawer ${isFileDrawerOpen ? "" : "main-drawer--closed"}`} data-stable-id="drawer:file">
            <header className="main-drawer__header">
              <div className="main-drawer__title">Project Files</div>
              <button
                type="button"
                data-testid="file-drawer.toggle"
                data-stable-id="drawer:file:toggle"
                onClick={() => setFileDrawerOpen((value) => !value)}
              >
                {fileDrawerOpen ? "Hide" : "Show"}
              </button>
            </header>
            {fileDrawerOpen ? (
              <div className="main-drawer__body">
                {activeFiles.length === 0 ? (
                  <div className="main-drawer__empty">No files in {activeProject.name}</div>
                ) : (
                  activeFiles.map((file) => (
                    <button
                      key={`file-${file.id}`}
                      className="main-drawer__item"
                      data-testid={`file-drawer.item.${file.id}`}
                      data-stable-id={`drawer:file:${file.id}`}
                      type="button"
                    >
                      {file.name}
                    </button>
                  ))
                )}
              </div>
            ) : null}
          </section>
        </aside>

        <section className="main-work-area">
          <nav className="main-module-rail" role="navigation" aria-label="Module rail">
            {moduleIds.map((moduleId) => (
              <button
                key={moduleId}
                type="button"
                data-testid={`module.${moduleId}`}
                data-stable-id={`module:${moduleId}`}
                className={moduleId === activeModule ? "main-module-rail__button--active" : undefined}
                onClick={() => setModule(moduleId)}
              >
                {moduleId}
              </button>
            ))}
          </nav>

          <div className="main-pane-area" ref={paneAreaRef}>
            <div
              className="main-pane-grid"
              data-testid="main-pane-grid"
              style={{
                gridTemplateColumns: `${(splitWeights.cols[0] * 100).toFixed(2)}% ${(splitWeights.cols[1] * 100).toFixed(2)}%`,
                gridTemplateRows: `${(splitWeights.rows[0] * 100).toFixed(2)}% ${(splitWeights.rows[1] * 100).toFixed(2)}%`,
              }}
            >
              {panes.map((pane, index) => (
                <section
                  key={pane.id}
                  className="main-pane"
                  data-pane-id={pane.id}
                  data-pane-type={pane.activeTab}
                  data-pane-active-tab={pane.activeTab}
                  data-pane-locked={pane.locked}
                  data-pane-project-ref={pane.projectRef}
                  data-stable-id={`pane:${pane.id}`}
                  data-testid={`main-pane.${pane.id}`}
                  style={paneCells[index].style as React.CSSProperties}
                >
                  <header className="main-pane__header">
                    <h2 className="main-pane__title">{paneCells[index].title}</h2>
                    <div className="main-pane__tabs" role="tablist" aria-label={`Pane ${pane.id} tabs`}>
                      {pane.tabs.map((tab) => (
                        <button
                          key={tab}
                          type="button"
                          role="tab"
                          aria-selected={tab === pane.activeTab}
                          data-testid={`pane-tab.${pane.id}.${tab}`}
                          data-stable-id={`pane-tab:${pane.id}:${tab}`}
                          className={`main-pane__tab ${pane.activeTab === tab ? "main-pane__tab--active" : ""}`}
                          onClick={() => setPaneTab(pane.id, tab)}
                        >
                          {paneTabLabels[tab]}
                        </button>
                      ))}
                    </div>
                    <span className="main-pane__tabmeta">Locked: {pane.locked ? "yes" : "no"}</span>
                  </header>
                  <div className="main-pane__content">{renderPaneContent(pane)}</div>
                </section>
              ))}
              <div
                className="main-pane-splitter main-pane-splitter--vertical"
                role="separator"
                aria-orientation="vertical"
                aria-valuemin={18}
                aria-valuemax={82}
                aria-valuenow={Math.round(splitWeights.cols[0] * 100)}
                tabIndex={0}
                data-testid="main-splitter.vertical"
                data-stable-id="splitter:vertical"
                data-lock={isVerticalSplitterLocked}
                style={{ left: `${splitWeights.cols[0] * 100}%` }}
                onPointerDown={(event) => handleSplitterPointerDown(event, "vertical", isVerticalSplitterLocked)}
                onPointerMove={(event) => handleSplitterPointerMove(event, "vertical")}
                onPointerUp={handleSplitterPointerUp}
                onKeyDown={(event) => handleSplitterKey("vertical", isVerticalSplitterLocked, event)}
              />
              <div
                className="main-pane-splitter main-pane-splitter--horizontal"
                role="separator"
                aria-orientation="horizontal"
                aria-valuemin={18}
                aria-valuemax={82}
                aria-valuenow={Math.round(splitWeights.rows[0] * 100)}
                tabIndex={0}
                data-testid="main-splitter.horizontal"
                data-stable-id="splitter:horizontal"
                data-lock={isHorizontalSplitterLocked}
                style={{ top: `${splitWeights.rows[0] * 100}%` }}
                onPointerDown={(event) => handleSplitterPointerDown(event, "horizontal", isHorizontalSplitterLocked)}
                onPointerMove={(event) => handleSplitterPointerMove(event, "horizontal")}
                onPointerUp={handleSplitterPointerUp}
                onKeyDown={(event) => handleSplitterKey("horizontal", isHorizontalSplitterLocked, event)}
              />
            </div>
          </div>

          <footer className="main-bottom-drawer" data-testid="main-bottom-drawer" data-stable-id="drawer:bottom">
            <header className="main-bottom-drawer__header">
              <button
                type="button"
                onClick={() => setBottomDrawerOpen((value) => !value)}
                data-testid="bottom-drawer.toggle"
                data-stable-id="drawer:bottom:toggle"
              >
                {bottomDrawerOpen ? "Hide Bottom Drawer" : "Open Bottom Drawer"}
              </button>
              <span>Bottom Drawer</span>
            </header>
            {bottomDrawerOpen ? (
              <div className="main-search-status" data-testid="search-status-region" data-stable-id="search-status-region">
                <input className="main-search-input" placeholder="Search..." data-testid="search-status.input" />
                <span className="main-status-chip">
                  Project: <strong>{activeProject.name}</strong>
                </span>
                <span className="main-status-chip">
                  Split: {Math.round(splitWeights.cols[0] * 100)}/{Math.round(splitWeights.rows[0] * 100)}
                </span>
              </div>
            ) : null}
          </footer>
        </section>
      </div>

      <EvidenceDrawer
        selection={selection}
        onClose={() => setSelection(null)}
        timelineWindow={timelineWindow ?? undefined}
        onExportScope={(scope) => setExportScope(scope)}
        onNavigateToJob={(jobId) => {
          setFocusJobId(jobId);
          setPanes((current) =>
            current.map((pane) =>
              pane.tabs.includes("jobs") ? { ...pane, activeTab: "jobs" as const, projectRef: activeProjectId } : pane,
            ),
          );
        }}
        onNavigateToTimeline={(navigation) => {
          setTimelineNav({ ...navigation });
          setPanes((current) =>
            current.map((pane) =>
              pane.tabs.includes("timeline")
                ? { ...pane, activeTab: "timeline" as const, projectRef: activeProjectId }
                : pane,
            ),
          );
        }}
      />
      <Ans001TimelineDrawer isOpen={ans001TimelineOpen} onClose={() => setAns001TimelineOpen(false)} />
      {exportScope && (
        <DebugBundleExport
          isOpen
          defaultScope={exportScope}
          onClose={() => setExportScope(null)}
        />
      )}
      {governancePackExportOpen && <GovernancePackExport isOpen onClose={() => setGovernancePackExportOpen(false)} />}
      <AiJobsDrawer />
    </main>
  );
}

export default App;
