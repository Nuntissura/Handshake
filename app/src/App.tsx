import "./App.css";
import {
  type CSSProperties,
  type JSX,
  type KeyboardEvent,
  type PointerEvent as ReactPointerEvent,
  useCallback,
  useEffect,
  useRef,
  useState,
} from "react";
import { WorkspaceSidebar } from "./components/WorkspaceSidebar";
import { DocumentView } from "./components/DocumentView";
import { CanvasView } from "./components/CanvasView";
import { DebugPanel } from "./components/DebugPanel";
import { KernelDccProjectionView } from "./components/KernelDccProjectionView";
import { InferenceLab } from "./components/inference_lab";
import { FontManagerView } from "./components/FontManagerView";
import { MediaDownloaderView } from "./components/MediaDownloaderView";
import { ModelRuntimePanel } from "./components/model_runtime_panel";
import { SwarmOperatorSurface } from "./components/swarm";
import { VisualDebuggerPanel } from "./components/visual_debugger";
import AtelierPanel from "./components/AtelierPanel";
import { SettingsMenu } from "./components/SettingsMenu";
import { loadSwarmBoardDefaultOpen } from "./lib/globalSettings";
import {
  getKernelDccProjection,
  triggerKernelDccAction,
  type BundleScopeInput,
  type KernelDccProjectionSurfaceV1,
  listWorkspaces,
} from "./lib/api";
import { AiJobsDrawer } from "./components/AiJobsDrawer";
import { ViewModeToggle } from "./components/ViewModeToggle";
import type { ViewMode } from "./lib/viewMode";
import { loadViewModeFromStorage, saveViewModeToStorage } from "./lib/viewMode";
import { SystemStatus } from "./components/SystemStatus";
import { FlightRecorderView } from "./components/FlightRecorderView";
import { CommandPalette, type CommandPaletteAction } from "./components/CommandPalette";
import { onHsLinkNavigate, type HsLinkNavigateDetail } from "./lib/editor/link_navigation";
import { UserManualPanel } from "./components/UserManualPanel";
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

type ModuleId = "MAIN" | "CKC" | "INGEST" | "STAGE" | "LAB" | "STUDIO";

type PaneTabId =
  | "workspace"
  | "media-downloader"
  | "fonts"
  | "flight-recorder"
  | "kernel-dcc"
  | "inference-lab"
  | "model-runtime"
  | "swarm"
  | "problems"
  | "jobs"
  | "timeline"
  | "user-manual"
  | "atelier"
  | "visual-debugger";

type PaneId = "pane-a" | "pane-b" | "pane-c" | "pane-d";

type ProjectItem = {
  id: string;
  name: string;
};

type PaneState = {
  id: PaneId;
  module: ModuleId;
  activeTab: PaneTabId;
  tabs: PaneTabId[];
  locked: boolean;
  projectRef: string;
};

type DragAxis = "vertical" | "horizontal";

type SplitWeights = {
  vertical: number;
  horizontal: number;
};

type LinkNavigationError = {
  refKind: string;
  refValue: string;
  label: string;
  message: string;
};

type LinkNavigationTarget = {
  refKind: string;
  refValue: string;
  label: string;
  surface: PaneTabId;
};

const TAB_LABEL_BY_ID: Record<PaneTabId, string> = {
  workspace: "Workspace",
  "media-downloader": "Media Downloader",
  fonts: "Fonts",
  "flight-recorder": "Flight Recorder",
  "kernel-dcc": "Kernel DCC",
  "inference-lab": "Inference Lab",
  "model-runtime": "Model Runtime",
  swarm: "Swarm",
  problems: "Problems",
  jobs: "Jobs",
  timeline: "Timeline",
  "user-manual": "UserManual",
  atelier: "Atelier",
  "visual-debugger": "Visual Debugger",
};

const MODULE_DEFINITIONS: {
  id: ModuleId;
  label: string;
  dataId: string;
  tabs: PaneTabId[];
  defaultTab: PaneTabId;
}[] = [
  {
    id: "MAIN",
    label: "MAIN",
    dataId: "module-main",
    tabs: ["workspace", "user-manual", "problems", "jobs", "timeline"],
    defaultTab: "workspace",
  },
  {
    id: "CKC",
    label: "CKC",
    dataId: "module-ckc",
    tabs: ["atelier", "kernel-dcc", "user-manual", "problems", "jobs", "timeline"],
    defaultTab: "atelier",
  },
  {
    id: "INGEST",
    label: "INGEST",
    dataId: "module-ingest",
    tabs: ["media-downloader", "fonts", "flight-recorder", "visual-debugger", "problems"],
    defaultTab: "media-downloader",
  },
  {
    id: "STAGE",
    label: "STAGE",
    dataId: "module-stage",
    tabs: ["fonts", "inference-lab", "visual-debugger", "flight-recorder", "problems"],
    defaultTab: "fonts",
  },
  {
    id: "LAB",
    label: "LAB",
    dataId: "module-lab",
    tabs: ["inference-lab", "model-runtime", "swarm", "fonts", "kernel-dcc", "user-manual"],
    defaultTab: "inference-lab",
  },
  {
    id: "STUDIO",
    label: "STUDIO",
    dataId: "module-studio",
    tabs: ["model-runtime", "swarm", "inference-lab", "fonts", "kernel-dcc", "user-manual"],
    defaultTab: "model-runtime",
  },
];

const DEFAULT_PANES: PaneState[] = [
  {
    id: "pane-a",
    module: "MAIN",
    activeTab: "workspace",
    tabs: ["workspace", "user-manual", "problems", "jobs", "timeline"],
    locked: false,
    projectRef: "project-main",
  },
  {
    id: "pane-b",
    module: "CKC",
    activeTab: "inference-lab",
    tabs: ["inference-lab", "kernel-dcc", "problems"],
    locked: false,
    projectRef: "project-main",
  },
  {
    id: "pane-c",
    module: "INGEST",
    activeTab: "media-downloader",
    tabs: ["media-downloader", "fonts", "flight-recorder"],
    locked: false,
    projectRef: "project-main",
  },
  {
    id: "pane-d",
    module: "STAGE",
    activeTab: "fonts",
    tabs: ["fonts", "inference-lab", "media-downloader"],
    locked: false,
    projectRef: "project-main",
  },
];

const DEFAULT_PROJECTS: ProjectItem[] = [{ id: "project-main", name: "Project Main" }];

const SPLIT_MIN = 0.2;
const SPLIT_MAX = 0.8;
const SPLIT_STEP = 0.05;
const USERMANUAL_DIAGNOSTICS_TAB_STABLE_ID = "hs-usermanual-diagnostics-tab";

const clampSplit = (value: number) => Math.max(SPLIT_MIN, Math.min(SPLIT_MAX, value));

const uniqueTabs = (tabs: PaneTabId[]) => [...new Set(tabs)];

const DIRECT_DOCUMENT_KINDS = new Set(["doc", "document", "rich_document", "rich-document", "note"]);
const WORKSPACE_SURFACE_LINK_KINDS = new Set(["file", "folder"]);
const MANUAL_SEARCH_LINK_KINDS = new Set(["spec", "symbol"]);

function directDocumentTarget(detail: HsLinkNavigateDetail): string | null {
  const refKind = detail.refKind.trim().toLowerCase();
  const refValue = detail.refValue.trim();
  if (!refValue) return null;
  if (DIRECT_DOCUMENT_KINDS.has(refKind) && refValue.startsWith("KRD-")) return refValue;
  return null;
}

function wpTarget(detail: HsLinkNavigateDetail): boolean {
  return detail.refKind.trim().toLowerCase() === "wp" && detail.refValue.trim().startsWith("WP-");
}

function App() {
  const [selectedDocumentId, setSelectedDocumentId] = useState<string | null>(null);
  const [selectedCanvasId, setSelectedCanvasId] = useState<string | null>(null);
  const [viewMode, setViewMode] = useState<ViewMode>(() => loadViewModeFromStorage());
  const [refreshKey, setRefreshKey] = useState<number>(0);
  const [selection, setSelection] = useState<EvidenceSelection | null>(null);
  const [exportScope, setExportScope] = useState<BundleScopeInput | null>(null);
  const [governancePackExportOpen, setGovernancePackExportOpen] = useState(false);
  const [focusJobId, setFocusJobId] = useState<string | null>(null);
  const [timelineNav, setTimelineNav] = useState<{ job_id?: string; wsid?: string; event_id?: string } | null>(
    null,
  );
  const [timelineWindow, setTimelineWindow] = useState<{ start: string; end: string; wsid?: string } | null>(
    null,
  );
  const [ans001TimelineOpen, setAns001TimelineOpen] = useState(false);
  const [kernelDccSurface, setKernelDccSurface] = useState<KernelDccProjectionSurfaceV1 | null>(null);
  const [kernelDccLoading, setKernelDccLoading] = useState(false);
  const [kernelDccError, setKernelDccError] = useState<string | null>(null);

  const [projects, setProjects] = useState<ProjectItem[]>(DEFAULT_PROJECTS);
  const [activeProjectId, setActiveProjectId] = useState<string>(DEFAULT_PROJECTS[0].id);
  const [activeModule, setActiveModule] = useState<ModuleId>("MAIN");
  const [panes, setPanes] = useState<PaneState[]>(DEFAULT_PANES);
  const [activePaneId, setActivePaneId] = useState<PaneId>("pane-a");
  const [searchText, setSearchText] = useState("");
  const [projectDrawerOpen, setProjectDrawerOpen] = useState(true);
  const [fileDrawerOpen, setFileDrawerOpen] = useState(true);
  const [bottomDrawerOpen, setBottomDrawerOpen] = useState(true);
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [appCommandPaletteOpen, setAppCommandPaletteOpen] = useState(false);
  const [linkNavigationError, setLinkNavigationError] = useState<LinkNavigationError | null>(null);
  const [linkNavigationTarget, setLinkNavigationTarget] = useState<LinkNavigationTarget | null>(null);
  const [userManualSearchRequest, setUserManualSearchRequest] = useState({ query: "", requestId: 0 });
  const [splitWeights, setSplitWeights] = useState<SplitWeights>({ vertical: 0.5, horizontal: 0.55 });
  const paneGridRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    saveViewModeToStorage(viewMode);
  }, [viewMode]);

  useEffect(() => {
    listWorkspaces()
      .then((result) => {
        const mapped = result.map((workspace) => ({ id: workspace.id, name: workspace.name }));
        if (mapped.length === 0) {
          return;
        }
        setProjects(mapped);
        setActiveProjectId((current) => (mapped.some((project) => project.id === current) ? current : mapped[0].id));
      })
      .catch(() => {
        setProjects((prev) => (prev.length > 0 ? prev : DEFAULT_PROJECTS));
      });
  }, []);

  useEffect(() => {
    queueMicrotask(() => {
      setPanes((current) =>
        current.map((pane) =>
          pane.projectRef === activeProjectId ? pane : { ...pane, projectRef: activeProjectId },
        ),
      );
    });
  }, [activeProjectId]);

  const loadKernelDccProjection = useCallback(() => {
    if (kernelDccSurface || kernelDccLoading) {
      return;
    }
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
  }, [kernelDccLoading, kernelDccSurface]);

  useEffect(() => {
    const hasKernelTab = panes.some((pane) => pane.activeTab === "kernel-dcc");
    if (hasKernelTab && !kernelDccSurface && !kernelDccLoading && !kernelDccError) {
      queueMicrotask(loadKernelDccProjection);
    }
  }, [panes, kernelDccError, kernelDccLoading, kernelDccSurface, loadKernelDccProjection]);

  const setActiveTabForPane = useCallback((paneId: PaneId, nextTab: PaneTabId) => {
    setActivePaneId(paneId);
    setPanes((current) =>
      current.map((pane) => {
        if (pane.id !== paneId) {
          return pane;
        }
        return {
          ...pane,
          activeTab: nextTab,
          tabs: uniqueTabs([nextTab, ...pane.tabs]),
        };
      }),
    );
  }, []);

  useEffect(() => {
    return onHsLinkNavigate((detail) => {
      const refKind = detail.refKind.trim().toLowerCase();
      const refValue = detail.refValue.trim();
      const fail = () => {
        setLinkNavigationTarget(null);
        setLinkNavigationError({
          refKind: detail.refKind,
          refValue: detail.refValue,
          label: detail.label,
          message: `Cannot resolve ${detail.refKind}:${detail.refValue} to a local Handshake surface.`,
        });
      };
      const markNavigated = (surface: PaneTabId) => {
        setLinkNavigationError(null);
        setLinkNavigationTarget({
          refKind,
          refValue,
          label: detail.label,
          surface,
        });
      };
      const targetDocumentId = directDocumentTarget(detail);
      if (targetDocumentId) {
        markNavigated("workspace");
        setSelectedCanvasId(null);
        setSelectedDocumentId(targetDocumentId);
        setActiveTabForPane(activePaneId, "workspace");
        return;
      }

      if (WORKSPACE_SURFACE_LINK_KINDS.has(refKind)) {
        markNavigated("workspace");
        setSelectedCanvasId(null);
        setSelectedDocumentId(null);
        setFileDrawerOpen(true);
        setActiveTabForPane(activePaneId, "workspace");
        return;
      }

      if (refKind === "project") {
        if (!projects.some((project) => project.id === refValue)) {
          fail();
          return;
        }
        markNavigated("workspace");
        setSelectedCanvasId(null);
        setSelectedDocumentId(null);
        setProjectDrawerOpen(true);
        setFileDrawerOpen(true);
        setActiveProjectId(refValue);
        setActiveTabForPane(activePaneId, "workspace");
        return;
      }

      if (wpTarget(detail)) {
        markNavigated("kernel-dcc");
        setActiveTabForPane(activePaneId, "kernel-dcc");
        return;
      }

      if (MANUAL_SEARCH_LINK_KINDS.has(refKind)) {
        markNavigated("user-manual");
        setPanes((current) =>
          current.map((pane) =>
            pane.id === activePaneId
              ? { ...pane, activeTab: "user-manual", tabs: uniqueTabs(["user-manual", ...pane.tabs]) }
              : pane,
          ),
        );
        setUserManualSearchRequest((current) => ({
          query: refValue,
          requestId: current.requestId + 1,
        }));
        return;
      }

      fail();
    });
  }, [activePaneId, projects, setActiveTabForPane]);

  const setModule = (moduleId: ModuleId) => {
    const nextDef = MODULE_DEFINITIONS.find((item) => item.id === moduleId);
    if (!nextDef) {
      return;
    }
    setActiveModule(moduleId);
    setPanes((current) =>
      current.map((pane) => {
        if (pane.id !== activePaneId) {
          return pane;
        }
        const nextTabs = uniqueTabs([nextDef.defaultTab, ...nextDef.tabs, ...pane.tabs]);
        return {
          ...pane,
          module: moduleId,
          activeTab: nextDef.defaultTab,
          tabs: nextTabs,
        };
      }),
    );
  };

  const openUserManualPane = (searchQuery?: string) => {
    setPanes((current) =>
      current.map((pane) => {
        if (pane.id !== activePaneId) {
          return pane;
        }
        return {
          ...pane,
          activeTab: "user-manual",
          tabs: uniqueTabs(["user-manual", ...pane.tabs]),
        };
      }),
    );
    if (searchQuery !== undefined) {
      setUserManualSearchRequest((current) => ({
        query: searchQuery,
        requestId: current.requestId + 1,
      }));
    }
  };

  const appCommandActions: CommandPaletteAction[] = [
    {
      id: "usermanual.open",
      label: "UserManual: Open",
      description: "Open the in-app UserManual diagnostics tab.",
      keywords: ["manual", "usermanual", "help", "diagnostics"],
      stableId: "hs-usermanual-palette-open",
    },
    {
      id: "usermanual.search",
      label: "UserManual: Search",
      description: searchText.trim()
        ? `Search UserManual for "${searchText.trim()}".`
        : "Open UserManual search.",
      keywords: ["manual", "usermanual", "search", "help"],
      stableId: "hs-usermanual-palette-search",
    },
  ];

  const runAppCommand = (actionId: string) => {
    if (actionId === "usermanual.open") {
      openUserManualPane();
      setAppCommandPaletteOpen(false);
      return;
    }
    if (actionId === "usermanual.search") {
      openUserManualPane(searchText.trim());
      setAppCommandPaletteOpen(false);
    }
  };

  const isSplitterLocked = (axis: DragAxis) =>
    axis === "vertical" ? panes[0].locked || panes[1].locked : panes[0].locked || panes[2].locked;

  const togglePaneLock = (paneId: PaneId) => {
    setPanes((current) =>
      current.map((pane) =>
        pane.id === paneId
          ? {
              ...pane,
              locked: !pane.locked,
            }
          : pane,
      ),
      );
  };

  const handleSplitDividerPointerDown =
    (axis: DragAxis) => (event: ReactPointerEvent<HTMLButtonElement>) => {
      if (isSplitterLocked(axis)) {
        return;
      }

      const trackRect = paneGridRef.current?.getBoundingClientRect();
      if (!trackRect) {
        return;
      }

      const trackSize = axis === "vertical" ? trackRect.width : trackRect.height;
      if (trackSize <= 0) {
        return;
      }

      const startClient = axis === "vertical" ? event.clientX : event.clientY;
      const startValue = axis === "vertical" ? splitWeights.vertical : splitWeights.horizontal;
      const pointerId = event.pointerId;
      const target = event.currentTarget;

      const onMove = (move: PointerEvent) => {
        if (move.pointerId !== pointerId) {
          return;
        }
        const delta =
          (axis === "vertical" ? move.clientX - startClient : move.clientY - startClient) / trackSize;
        const next = clampSplit(startValue + delta);
        setSplitWeights((current) =>
          axis === "vertical" ? { ...current, vertical: next } : { ...current, horizontal: next },
        );
      };

      const onUp = (up: PointerEvent) => {
        if (up.pointerId !== pointerId) {
          return;
        }
        window.removeEventListener("pointermove", onMove);
        window.removeEventListener("pointerup", onUp);
        target.releasePointerCapture?.(pointerId);
      };

      target.setPointerCapture?.(pointerId);
      window.addEventListener("pointermove", onMove);
      window.addEventListener("pointerup", onUp);
    };

  const handleSplitDividerKeyDown =
    (axis: DragAxis) => (event: KeyboardEvent<HTMLButtonElement>) => {
      if (isSplitterLocked(axis)) {
        return;
      }

      if (axis === "vertical" && event.key !== "ArrowLeft" && event.key !== "ArrowRight") {
        return;
      }
      if (axis === "horizontal" && event.key !== "ArrowUp" && event.key !== "ArrowDown") {
        return;
      }
      event.preventDefault();
      setSplitWeights((current) => {
        if (axis === "vertical") {
          const delta = event.key === "ArrowLeft" ? -SPLIT_STEP : SPLIT_STEP;
          return { ...current, vertical: clampSplit(current.vertical + delta) };
        }
        const delta = event.key === "ArrowUp" ? -SPLIT_STEP : SPLIT_STEP;
        return { ...current, horizontal: clampSplit(current.horizontal + delta) };
      });
    };

  const buildPane = (pane: PaneState) => {
    let content: JSX.Element;
    if (pane.activeTab === "workspace") {
      content = (
        <>
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
                <p className="muted">Select or create a workspace, then add documents or canvases.</p>
              </div>
            )}
          </div>
        </>
      );
    } else if (pane.activeTab === "media-downloader") {
      content = <MediaDownloaderView />;
    } else if (pane.activeTab === "fonts") {
      content = <FontManagerView />;
    } else if (pane.activeTab === "flight-recorder") {
      content = <FlightRecorderView />;
    } else if (pane.activeTab === "kernel-dcc") {
      if (kernelDccLoading) {
        content = (
          <div className="content-card" data-stable-id="kernel-dcc-projection-loading">
            <h2>Loading Kernel DCC projection...</h2>
          </div>
        );
      } else if (kernelDccError) {
        content = (
          <div className="content-card" data-stable-id="kernel-dcc-projection-error">
            <h2>Kernel DCC projection unavailable</h2>
            <p className="muted">{kernelDccError}</p>
          </div>
        );
      } else if (kernelDccSurface) {
        content = (
          <KernelDccProjectionView
            surface={kernelDccSurface}
            onTriggerCatalogAction={triggerKernelDccAction}
            data-testid="kernel-dcc-projection"
          />
        );
      } else {
        content = (
          <div className="content-card" data-stable-id="kernel-dcc-projection-unavailable">
            <h2>Kernel DCC projection unavailable</h2>
          </div>
        );
      }
    } else if (pane.activeTab === "inference-lab") {
      content = <InferenceLab />;
    } else if (pane.activeTab === "model-runtime") {
      content = <ModelRuntimePanel />;
    } else if (pane.activeTab === "swarm") {
      content = <SwarmOperatorSurface boardDefaultOpen={loadSwarmBoardDefaultOpen()} />;
    } else if (pane.activeTab === "problems") {
      content = <ProblemsView onSelect={setSelection} />;
    } else if (pane.activeTab === "jobs") {
      content = <JobsView onSelect={setSelection} focusJobId={focusJobId} />;
    } else if (pane.activeTab === "user-manual") {
      content = (
        <UserManualPanel
          initialSearchQuery={userManualSearchRequest.query}
          searchRequestId={userManualSearchRequest.requestId}
        />
      );
    } else if (pane.activeTab === "atelier") {
      content = <AtelierPanel />;
    } else if (pane.activeTab === "visual-debugger") {
      content = <VisualDebuggerPanel />;
    } else {
      content = (
        <TimelineView
          onSelect={setSelection}
          navigation={timelineNav}
          onTimeWindowChange={setTimelineWindow}
        />
      );
    }

    return (
      <section
        key={pane.id}
        className="main-pane"
        data-pane-id={pane.id}
        data-pane-type={pane.activeTab}
        data-pane-module={pane.module}
        data-pane-active-tab={pane.activeTab}
        data-pane-locked={pane.locked ? "true" : "false"}
        data-pane-lock={pane.locked ? "true" : "false"}
        data-pane-project-ref={pane.projectRef}
        data-stable-id={`pane-${pane.id}`}
        data-testid={`pane-${pane.id}`}
      >
        <div className="main-pane__header">
          <div className="main-pane__tabs" data-stable-id={`pane-${pane.id}-tabs`}>
            {pane.tabs.map((tab) => {
              const tabId = `pane-${pane.id}.tab.${tab}`;
              const tabStableId =
                tab === "user-manual" && pane.id === "pane-a"
                  ? USERMANUAL_DIAGNOSTICS_TAB_STABLE_ID
                  : tabId;
              return (
                <button
                  key={tabId}
                  type="button"
                  className={pane.activeTab === tab ? "main-pane__tab main-pane__tab--active" : "main-pane__tab"}
                  onClick={() => setActiveTabForPane(pane.id, tab)}
                  data-stable-id={tabStableId}
                  data-testid={tabId}
                  data-pane-tab={tab}
                >
                  {TAB_LABEL_BY_ID[tab]}
                </button>
              );
            })}
          </div>
          <button
            type="button"
            className="main-pane__lock"
            onClick={() => togglePaneLock(pane.id)}
            data-stable-id={`pane-${pane.id}-lock`}
          >
            {pane.locked ? "Unlock" : "Lock"}
          </button>
        </div>
        <div className="content-panel content-panel--full" data-stable-id={`pane-${pane.id}-content`}>
          {content}
        </div>
      </section>
    );
  };

  const activeProjectName = projects.find((project) => project.id === activeProjectId)?.name ?? activeProjectId;

  return (
    <main
      id="main-window"
      className="app-shell app-shell--main-v1"
      data-stable-layout="main-window-v1"
      data-stable-id="main-window"
      data-active-module={activeModule}
      data-active-project-id={activeProjectId}
      data-project-drawer-open={projectDrawerOpen ? "true" : "false"}
      data-file-drawer-open={fileDrawerOpen ? "true" : "false"}
      data-bottom-drawer-open={bottomDrawerOpen ? "true" : "false"}
      data-split-weights={`${splitWeights.vertical.toFixed(3)},${splitWeights.horizontal.toFixed(3)}`}
      data-link-navigation-state={
        linkNavigationError ? "error" : linkNavigationTarget || selectedDocumentId ? "navigated" : "idle"
      }
      data-link-navigation-kind={linkNavigationTarget?.refKind ?? ""}
      data-link-navigation-value={linkNavigationTarget?.refValue ?? ""}
      data-link-navigation-surface={linkNavigationTarget?.surface ?? ""}
      data-testid="main-window"
    >
      <div className="app-layout">
        <header className="app-header">
          <div>
            <p className="app-eyebrow">Handshake</p>
            <h1 className="app-title">Desktop Shell</h1>
            <p className="app-subtitle">Coordinator, workspaces, documents, and canvases.</p>
          </div>
          <div className="app-header-right">
            <ViewModeToggle value={viewMode} onChange={setViewMode} />
            <SystemStatus />
            <button
              type="button"
              className="settings-gear"
              aria-label="Open settings"
              aria-haspopup="dialog"
              onClick={() => setSettingsOpen(true)}
              data-stable-id="settings-gear"
              data-testid="settings-gear"
            >
              ⚙
            </button>
          </div>
        </header>

        {linkNavigationError && (
          <div
            className="hs-link-navigation-error"
            role="alert"
            data-testid="hs-link-navigation-error"
            data-ref-kind={linkNavigationError.refKind}
            data-ref-value={linkNavigationError.refValue}
          >
            {linkNavigationError.message}
          </div>
        )}

        <div className="main-window-surface">
          <aside
            className={`left-rail ${projectDrawerOpen ? "left-rail--open" : "left-rail--collapsed"}`}
            data-stable-id="module-rail"
            data-project-drawer-open={projectDrawerOpen ? "true" : "false"}
            data-testid="module-rail"
          >
            <button
              type="button"
              className="module-rail__toggle"
              onClick={() => setProjectDrawerOpen((value) => !value)}
              data-stable-id="project-drawer.toggle"
              data-testid="project-drawer.toggle"
            >
              {projectDrawerOpen ? "Hide Project Rail" : "Show Project Rail"}
            </button>

            {projectDrawerOpen ? (
              <>
                <section className="rail-section" data-stable-id="module-rail-sections">
                  <div className="rail-section__title">Modules</div>
                  <div className="module-buttons">
                    {MODULE_DEFINITIONS.map((moduleItem) => {
                      const moduleActive = activeModule === moduleItem.id;
                      return (
                        <button
                          key={moduleItem.id}
                          type="button"
                          className={`main-button${moduleActive ? " main-button--active" : ""}`}
                          onClick={() => setModule(moduleItem.id)}
                          data-stable-id={moduleItem.dataId}
                          data-testid={moduleItem.dataId}
                          data-stable-module={moduleItem.id}
                        >
                          {moduleItem.label}
                        </button>
                      );
                    })}
                  </div>
                </section>
                <section className="rail-section">
                  <div className="rail-section__title">Projects</div>
                  <div className="project-tabs" data-stable-id="project-tabs">
                    {projects.map((project) => {
                      const stableId = `project-${project.id}`;
                      return (
                        <button
                          key={project.id}
                          type="button"
                          className={`main-button${activeProjectId === project.id ? " main-button--active" : ""}`}
                          onClick={() => setActiveProjectId(project.id)}
                          data-stable-id={stableId}
                          data-testid={stableId}
                        >
                          {project.name}
                        </button>
                      );
                    })}
                  </div>
                </section>
              </>
            ) : null}
          </aside>

          <section className="app-main" data-stable-id="app-main">
            <div
              className={`file-drawer ${fileDrawerOpen ? "file-drawer--open" : "file-drawer--closed"}`}
              data-stable-id="file-drawer"
              data-file-drawer-open={fileDrawerOpen ? "true" : "false"}
              data-testid="file-drawer"
            >
              <button
                type="button"
                onClick={() => setFileDrawerOpen((value) => !value)}
                data-stable-id="file-drawer.toggle"
                data-testid="file-drawer.toggle"
              >
                {fileDrawerOpen ? "Hide Files" : "Show Files"}
              </button>
              {fileDrawerOpen ? (
                <WorkspaceSidebar
                  refreshKey={refreshKey}
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
                    setRefreshKey((k) => k + 1);
                  }}
                />
              ) : null}
            </div>

            <div className="main-workarea">
              <div
                className="main-pane-grid"
                ref={paneGridRef}
                style={
                  {
                    "--hsk-pane-vertical-split": `${splitWeights.vertical * 100}%`,
                    "--hsk-pane-horizontal-split": `${splitWeights.horizontal * 100}%`,
                  } as CSSProperties
                }
                data-stable-id="pane-grid"
                data-testid="pane-grid"
                data-split-state="pane-grid"
              >
                {buildPane(panes[0])}
                {buildPane(panes[1])}
                {buildPane(panes[2])}
                {buildPane(panes[3])}

                <button
                  type="button"
                  className="main-divider main-divider--vertical"
                  onPointerDown={handleSplitDividerPointerDown("vertical")}
                  onKeyDown={handleSplitDividerKeyDown("vertical")}
                  disabled={isSplitterLocked("vertical")}
                  aria-orientation="vertical"
                  role="separator"
                  aria-valuemin={SPLIT_MIN * 100}
                  aria-valuemax={SPLIT_MAX * 100}
                  aria-valuenow={Math.round(splitWeights.vertical * 100)}
                  data-divider-axis="vertical"
                  data-lock={isSplitterLocked("vertical") ? "true" : "false"}
                  data-stable-id="pane-divider.vertical"
                  data-testid="main-window-splitter-vertical"
                  data-divider-locked={isSplitterLocked("vertical")}
                  tabIndex={isSplitterLocked("vertical") ? -1 : 0}
                  aria-label="Vertical pane splitter"
                />

                <button
                  type="button"
                  className="main-divider main-divider--horizontal"
                  onPointerDown={handleSplitDividerPointerDown("horizontal")}
                  onKeyDown={handleSplitDividerKeyDown("horizontal")}
                  disabled={isSplitterLocked("horizontal")}
                  aria-orientation="horizontal"
                  role="separator"
                  aria-valuemin={SPLIT_MIN * 100}
                  aria-valuemax={SPLIT_MAX * 100}
                  aria-valuenow={Math.round(splitWeights.horizontal * 100)}
                  data-divider-axis="horizontal"
                  data-lock={isSplitterLocked("horizontal") ? "true" : "false"}
                  data-stable-id="pane-divider.horizontal"
                  data-testid="main-window-splitter-horizontal"
                  data-divider-locked={isSplitterLocked("horizontal")}
                  tabIndex={isSplitterLocked("horizontal") ? -1 : 0}
                  aria-label="Horizontal pane splitter"
                />
              </div>

              <div
                className="bottom-control-strip"
                data-stable-id="bottom-control-strip"
                data-bottom-drawer-open={bottomDrawerOpen ? "true" : "false"}
                data-testid="bottom-control-strip"
              >
                <span className="muted">Active Project: {activeProjectName}</span>
                <button
                  type="button"
                  onClick={() => setBottomDrawerOpen((value) => !value)}
                  data-stable-id="bottom-drawer.toggle"
                  data-testid="bottom-drawer.toggle"
                >
                  {bottomDrawerOpen ? "Collapse Bottom Drawer" : "Open Bottom Drawer"}
                </button>
                <button type="button" onClick={() => setGovernancePackExportOpen(true)}>
                  Gov Pack Export
                </button>
                <button type="button" onClick={() => setAns001TimelineOpen(true)}>
                  ANS-001 Timeline
                </button>
                <button
                  type="button"
                  onClick={() => setAppCommandPaletteOpen(true)}
                  data-stable-id="app-command-palette.open"
                  data-testid="app-command-palette.open"
                >
                  Commands
                </button>
              </div>

              {bottomDrawerOpen ? (
                <div
                  className="bottom-drawer"
                  data-stable-id="search-status-region"
                  data-testid="search-status-region"
                  data-pane-type="search-status"
                >
                  <input
                    data-stable-id="search-input"
                    data-testid="search-input"
                    type="search"
                    value={searchText}
                    onChange={(event) => setSearchText(event.target.value)}
                    placeholder="Search logs and events"
                    aria-label="Search"
                  />
                  <div className="muted">
                    Split Weights: {Math.round(splitWeights.vertical * 100)}% x {Math.round(splitWeights.horizontal * 100)}%
                  </div>
                </div>
              ) : null}
            </div>
          </section>
        </div>
      </div>

      <EvidenceDrawer
        selection={selection}
        onClose={() => setSelection(null)}
        timelineWindow={timelineWindow ?? undefined}
        onExportScope={(scope) => setExportScope(scope)}
        onNavigateToJob={(jobId) => {
          setActiveTabForPane(activePaneId, "jobs");
          setFocusJobId(jobId);
        }}
        onNavigateToTimeline={(nav) => {
          setActiveTabForPane(activePaneId, "timeline");
          setTimelineNav({ ...nav });
        }}
      />
      <Ans001TimelineDrawer isOpen={ans001TimelineOpen} onClose={() => setAns001TimelineOpen(false)} />
      <SettingsMenu
        isOpen={settingsOpen}
        onClose={() => setSettingsOpen(false)}
        viewMode={viewMode}
        onViewModeChange={setViewMode}
      />
      <CommandPalette
        open={appCommandPaletteOpen}
        title="App commands"
        actions={appCommandActions}
        onAction={runAppCommand}
        onClose={() => setAppCommandPaletteOpen(false)}
      />
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
