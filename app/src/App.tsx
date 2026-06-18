import "./App.css";
import {
  type CSSProperties,
  type DragEvent as ReactDragEvent,
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
  getWorkbenchLayoutState,
  getKernelDccProjection,
  getWorkspaceSettingsState,
  saveWorkbenchLayoutState,
  saveWorkspaceSettingsState,
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
import { CommandPalette } from "./components/CommandPalette";
import { UserManualPanel } from "./components/UserManualPanel";
import { CodeSymbolPanel } from "./components/CodeSymbolPanel";
import { SourceControlPanel } from "./components/SourceControlPanel";
import { LoomBlockPanel } from "./components/LoomBlockPanel";
import { LoomDailyJournalPanel } from "./components/LoomDailyJournalPanel";
import { LoomWikiPagePanel } from "./components/LoomWikiPagePanel";
import { LoomAiReviewPanel } from "./components/LoomAiReviewPanel";
import { QuickSwitcher } from "./components/QuickSwitcher";
import { WorkspaceSearchPanel } from "./components/WorkspaceSearchPanel";
import { LoomSearchV2Panel } from "./components/LoomSearchV2Panel";
import { buildAppCommandRegistry, resolveEditorAppCommand } from "./lib/app_command_registry";
import type { EditorCommandPaletteRequest } from "./lib/editor/editor_command_palette_request";
import type { EditorFindOptions, EditorFindRequest } from "./lib/editor/editor_find_request";
import { onHsLinkNavigate, resolveHsLinkTarget } from "./lib/editor/link_navigation";
import {
  defaultWorkspaceSettingsState,
  keyboardEventMatchesChord,
  normalizeWorkspaceSettingsState,
  type WorkspaceSettingsState,
} from "./lib/workspaceSettings";
import { configureHandshakeCodeIntelligence } from "./lib/monaco/code_intelligence";
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
  | "code-symbol"
  | "source-control"
  | "loom-daily-journal"
  | "loom-block"
  | "loom-wiki-page"
  | "atelier"
  | "visual-debugger";

type PaneId = "pane-a" | "pane-b" | "pane-c" | "pane-d";

type ProjectItem = {
  id: string;
  name: string;
};

type OpenDocumentTab = {
  documentId: string;
  pinned: boolean;
  dirty: boolean;
};

type EditorFindTargetRequest = {
  paneId: PaneId;
  documentId: string;
  request: EditorFindRequest;
};

type PaneState = {
  id: PaneId;
  module: ModuleId;
  activeTab: PaneTabId;
  tabs: PaneTabId[];
  locked: boolean;
  projectRef: string;
  activeDocumentId: string | null;
  activeCanvasId: string | null;
  openDocuments: OpenDocumentTab[];
};

type DragAxis = "vertical" | "horizontal";

type SplitWeights = {
  vertical: number;
  horizontal: number;
};

type WorkbenchLayoutState = {
  schema_id: typeof WORKBENCH_LAYOUT_SCHEMA_ID;
  activePaneId: PaneId;
  activeModule: ModuleId;
  splitWeights: SplitWeights;
  drawers: {
    project: boolean;
    file: boolean;
    bottom: boolean;
  };
  panes: PaneState[];
};

type WorkbenchLayoutPersistenceStatus = {
  state: "loading" | "ready" | "load-error" | "save-pending" | "save-error";
  message: string;
};

type WorkspaceSettingsPersistenceStatus = WorkbenchLayoutPersistenceStatus;

type KernelDccFocusTarget = {
  wpId?: string | null;
  mtId?: string | null;
};

const WORKBENCH_LAYOUT_SCHEMA_ID = "hsk.workbench_layout_state@1";
const PANE_IDS: PaneId[] = ["pane-a", "pane-b", "pane-c", "pane-d"];
const DEFAULT_SPLIT_WEIGHTS: SplitWeights = { vertical: 0.5, horizontal: 0.55 };
const WORKBENCH_LAYOUT_READY_STATUS: WorkbenchLayoutPersistenceStatus = {
  state: "ready",
  message: "Layout saved",
};
const WORKSPACE_SETTINGS_READY_STATUS: WorkspaceSettingsPersistenceStatus = {
  state: "ready",
  message: "Settings saved",
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
  "code-symbol": "Code Symbol",
  "source-control": "Source Control",
  "loom-daily-journal": "Journal",
  "loom-block": "Loom Block",
  "loom-wiki-page": "Wiki Page",
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
    tabs: [
      "workspace",
      "loom-daily-journal",
      "loom-block",
      "loom-wiki-page",
      "user-manual",
      "problems",
      "jobs",
      "timeline",
    ],
    defaultTab: "workspace",
  },
  {
    id: "CKC",
    label: "CKC",
    dataId: "module-ckc",
    tabs: [
      "atelier",
      "kernel-dcc",
      "code-symbol",
      "source-control",
      "loom-daily-journal",
      "loom-block",
      "loom-wiki-page",
      "user-manual",
      "problems",
      "jobs",
      "timeline",
    ],
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
    tabs: ["workspace", "loom-daily-journal", "user-manual", "problems", "jobs", "timeline"],
    locked: false,
    projectRef: "project-main",
    activeDocumentId: null,
    activeCanvasId: null,
    openDocuments: [],
  },
  {
    id: "pane-b",
    module: "CKC",
    activeTab: "inference-lab",
    tabs: ["inference-lab", "kernel-dcc", "problems"],
    locked: false,
    projectRef: "project-main",
    activeDocumentId: null,
    activeCanvasId: null,
    openDocuments: [],
  },
  {
    id: "pane-c",
    module: "INGEST",
    activeTab: "media-downloader",
    tabs: ["media-downloader", "fonts", "flight-recorder"],
    locked: false,
    projectRef: "project-main",
    activeDocumentId: null,
    activeCanvasId: null,
    openDocuments: [],
  },
  {
    id: "pane-d",
    module: "STAGE",
    activeTab: "fonts",
    tabs: ["fonts", "inference-lab", "media-downloader"],
    locked: false,
    projectRef: "project-main",
    activeDocumentId: null,
    activeCanvasId: null,
    openDocuments: [],
  },
];

const DEFAULT_PROJECTS: ProjectItem[] = [{ id: "project-main", name: "Project Main" }];

const SPLIT_MIN = 0.2;
const SPLIT_MAX = 0.8;
const SPLIT_STEP = 0.05;
const WORKBENCH_LAYOUT_SAVE_RETRY_DELAYS_MS = [50, 100, 200] as const;
const USERMANUAL_DIAGNOSTICS_TAB_STABLE_ID = "hs-usermanual-diagnostics-tab";
const DOCUMENT_TAB_DRAG_MIME = "application/x-handshake-document-tab";

const clampSplit = (value: number) => Math.max(SPLIT_MIN, Math.min(SPLIT_MAX, value));

const uniqueTabs = (tabs: PaneTabId[]) => [...new Set(tabs)];

const paneIdForKeyboardNavigation = (currentPaneId: PaneId, key: string): PaneId => {
  if (key === "ArrowRight") {
    return currentPaneId === "pane-a" ? "pane-b" : currentPaneId === "pane-c" ? "pane-d" : currentPaneId;
  }
  if (key === "ArrowLeft") {
    return currentPaneId === "pane-b" ? "pane-a" : currentPaneId === "pane-d" ? "pane-c" : currentPaneId;
  }
  if (key === "ArrowDown") {
    return currentPaneId === "pane-a" ? "pane-c" : currentPaneId === "pane-b" ? "pane-d" : currentPaneId;
  }
  if (key === "ArrowUp") {
    return currentPaneId === "pane-c" ? "pane-a" : currentPaneId === "pane-d" ? "pane-b" : currentPaneId;
  }
  return currentPaneId;
};

const stableIdPart = (value: string): string => {
  const stable = value
    .trim()
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-+|-+$/g, "");
  return stable || "item";
};

const uniqueOpenDocumentTabs = (documents: OpenDocumentTab[]): OpenDocumentTab[] => {
  const seen = new Set<string>();
  const unique: OpenDocumentTab[] = [];
  for (const document of documents) {
    if (seen.has(document.documentId)) {
      continue;
    }
    seen.add(document.documentId);
    unique.push(document);
  }
  return unique;
};

const normalizePaneDocuments = (
  pane: PaneState,
): Pick<PaneState, "activeDocumentId" | "activeCanvasId" | "openDocuments"> => {
  const openDocuments = uniqueOpenDocumentTabs(pane.openDocuments);
  const activeDocumentId =
    pane.activeCanvasId !== null
      ? null
      : pane.activeDocumentId !== null &&
          openDocuments.some((document) => document.documentId === pane.activeDocumentId)
      ? pane.activeDocumentId
      : (openDocuments[0]?.documentId ?? null);
  return {
    activeDocumentId,
    activeCanvasId: activeDocumentId ? null : pane.activeCanvasId,
    openDocuments,
  };
};

const panesForProject = (projectId: string): PaneState[] =>
  DEFAULT_PANES.map((pane) => ({ ...pane, openDocuments: [...pane.openDocuments], projectRef: projectId }));

const isRecord = (value: unknown): value is Record<string, unknown> =>
  typeof value === "object" && value !== null && !Array.isArray(value);

const isPaneId = (value: unknown): value is PaneId =>
  typeof value === "string" && PANE_IDS.includes(value as PaneId);

const isModuleId = (value: unknown): value is ModuleId =>
  typeof value === "string" && MODULE_DEFINITIONS.some((module) => module.id === value);

const isPaneTabId = (value: unknown): value is PaneTabId =>
  typeof value === "string" && Object.prototype.hasOwnProperty.call(TAB_LABEL_BY_ID, value);

const parseSplitWeight = (value: unknown) =>
  typeof value === "number" && Number.isFinite(value) ? clampSplit(value) : null;

const buildWorkbenchLayoutState = ({
  activePaneId,
  activeModule,
  splitWeights,
  drawers,
  panes,
}: Omit<WorkbenchLayoutState, "schema_id">): WorkbenchLayoutState => ({
  schema_id: WORKBENCH_LAYOUT_SCHEMA_ID,
  activePaneId,
  activeModule,
  splitWeights: {
    vertical: splitWeights.vertical,
    horizontal: splitWeights.horizontal,
  },
  drawers: {
    project: drawers.project,
    file: drawers.file,
    bottom: drawers.bottom,
  },
  panes: panes.map((pane) => ({
    ...pane,
    tabs: uniqueTabs(pane.tabs),
    ...normalizePaneDocuments(pane),
  })),
});

const defaultWorkbenchLayoutState = (projectId: string) =>
  buildWorkbenchLayoutState({
    activePaneId: "pane-a",
    activeModule: "MAIN",
    splitWeights: DEFAULT_SPLIT_WEIGHTS,
    drawers: { project: true, file: true, bottom: true },
    panes: panesForProject(projectId),
  });

const serializeWorkbenchLayoutState = (layout: WorkbenchLayoutState) => JSON.stringify(layout);

const parseWorkbenchLayoutPane = (value: unknown): PaneState | null => {
  if (!isRecord(value)) {
    return null;
  }
  const { id, module, activeTab, tabs, locked, projectRef, activeDocumentId, activeCanvasId, openDocuments } = value;
  if (!isPaneId(id) || !isModuleId(module) || !isPaneTabId(activeTab)) {
    return null;
  }
  if (!Array.isArray(tabs) || typeof locked !== "boolean" || typeof projectRef !== "string") {
    return null;
  }
  const validTabs = tabs.filter(isPaneTabId);
  if (validTabs.length !== tabs.length) {
    return null;
  }
  let parsedActiveDocumentId: string | null = null;
  if (activeDocumentId !== undefined && activeDocumentId !== null) {
    if (typeof activeDocumentId !== "string" || activeDocumentId.trim().length === 0) {
      return null;
    }
    parsedActiveDocumentId = activeDocumentId;
  }
  let parsedActiveCanvasId: string | null = null;
  if (activeCanvasId !== undefined && activeCanvasId !== null) {
    if (typeof activeCanvasId !== "string" || activeCanvasId.trim().length === 0) {
      return null;
    }
    parsedActiveCanvasId = activeCanvasId;
  }
  let parsedOpenDocuments: OpenDocumentTab[] = [];
  if (openDocuments !== undefined) {
    if (!Array.isArray(openDocuments)) {
      return null;
    }
    for (const document of openDocuments) {
      if (!isRecord(document)) {
        return null;
      }
      const { documentId, pinned, dirty } = document;
      if (typeof documentId !== "string" || documentId.trim().length === 0) {
        return null;
      }
      if (pinned !== undefined && typeof pinned !== "boolean") {
        return null;
      }
      if (dirty !== undefined && typeof dirty !== "boolean") {
        return null;
      }
      parsedOpenDocuments.push({ documentId, pinned: pinned ?? false, dirty: dirty ?? false });
    }
  }
  parsedOpenDocuments = uniqueOpenDocumentTabs(parsedOpenDocuments);
  if (parsedActiveDocumentId && !parsedOpenDocuments.some((document) => document.documentId === parsedActiveDocumentId)) {
    parsedOpenDocuments.unshift({ documentId: parsedActiveDocumentId, pinned: false, dirty: false });
  }
  return {
    id,
    module,
    activeTab,
    tabs: uniqueTabs([activeTab, ...validTabs]),
    locked,
    projectRef,
    activeDocumentId: parsedActiveDocumentId,
    activeCanvasId: parsedActiveDocumentId ? null : parsedActiveCanvasId,
    openDocuments: parsedOpenDocuments,
  };
};

const parseWorkbenchLayoutState = (value: unknown): WorkbenchLayoutState | null => {
  if (!isRecord(value) || value.schema_id !== WORKBENCH_LAYOUT_SCHEMA_ID) {
    return null;
  }
  if (!isPaneId(value.activePaneId) || !isModuleId(value.activeModule)) {
    return null;
  }
  if (!isRecord(value.splitWeights) || !isRecord(value.drawers) || !Array.isArray(value.panes)) {
    return null;
  }
  const vertical = parseSplitWeight(value.splitWeights.vertical);
  const horizontal = parseSplitWeight(value.splitWeights.horizontal);
  if (vertical === null || horizontal === null) {
    return null;
  }
  const { project, file, bottom } = value.drawers;
  if (typeof project !== "boolean" || typeof file !== "boolean" || typeof bottom !== "boolean") {
    return null;
  }
  const panes = value.panes.map(parseWorkbenchLayoutPane);
  if (panes.some((pane) => pane === null)) {
    return null;
  }
  const normalizedPanes = panes as PaneState[];
  const paneIds = new Set(normalizedPanes.map((pane) => pane.id));
  if (normalizedPanes.length !== PANE_IDS.length || !PANE_IDS.every((id) => paneIds.has(id))) {
    return null;
  }
  const orderedPanes = PANE_IDS.map((id) => normalizedPanes.find((pane) => pane.id === id)).filter(
    (pane): pane is PaneState => pane !== undefined,
  );
  return buildWorkbenchLayoutState({
    activePaneId: value.activePaneId,
    activeModule: value.activeModule,
    splitWeights: { vertical, horizontal },
    drawers: { project, file, bottom },
    panes: orderedPanes,
  });
};

function App() {
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
  const [kernelDccFocusTarget, setKernelDccFocusTarget] = useState<KernelDccFocusTarget | null>(null);
  const [codeSymbolEntityId, setCodeSymbolEntityId] = useState<string | null>(null);
  const [loomBlockTarget, setLoomBlockTarget] = useState<{ workspaceId: string; blockId: string } | null>(null);
  const [loomWikiPageTarget, setLoomWikiPageTarget] = useState<{
    workspaceId: string;
    projectionId: string;
  } | null>(null);

  const [projects, setProjects] = useState<ProjectItem[]>(DEFAULT_PROJECTS);
  const [activeProjectId, setActiveProjectId] = useState<string>(DEFAULT_PROJECTS[0].id);
  const [workspaceListResolved, setWorkspaceListResolved] = useState(false);
  const [activeModule, setActiveModule] = useState<ModuleId>("MAIN");
  const [panes, setPanes] = useState<PaneState[]>(DEFAULT_PANES);
  const [activePaneId, setActivePaneId] = useState<PaneId>("pane-a");
  const [searchText, setSearchText] = useState("");
  const [projectDrawerOpen, setProjectDrawerOpen] = useState(true);
  const [fileDrawerOpen, setFileDrawerOpen] = useState(true);
  const [bottomDrawerOpen, setBottomDrawerOpen] = useState(true);
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [appCommandPaletteOpen, setAppCommandPaletteOpen] = useState(false);
  // MT-245 (EXT-NAV-LINK-001): an unresolvable typed hsLink surfaces a typed,
  // visible error — never a silent no-op when no real surface can open it.
  const [linkNavigationError, setLinkNavigationError] = useState<{
    refKind: string;
    refValue: string;
    label: string;
    message: string;
  } | null>(null);
  const [quickSwitcherOpen, setQuickSwitcherOpen] = useState(false);
  const [workspaceSearchOpen, setWorkspaceSearchOpen] = useState(false);
  // MT-264: LoomSearchV2 -- hybrid (full-text + fuzzy + semantic) Postgres-native
  // search over the whole Loom corpus, blended with the Loom graph. Supersedes
  // the MT-258 workspace search panel's loom-block lane with a ranked,
  // highlighted, typo-tolerant, semantically-relevant surface.
  const [loomSearchV2Open, setLoomSearchV2Open] = useState(false);
  const [loomAiReviewOpen, setLoomAiReviewOpen] = useState(false);
  const [editorCommandPaletteRequest, setEditorCommandPaletteRequest] =
    useState<EditorCommandPaletteRequest | null>(null);
  const [editorFindRequest, setEditorFindRequest] = useState<EditorFindTargetRequest | null>(null);
  const [workbenchLayoutPersistenceStatus, setWorkbenchLayoutPersistenceStatus] =
    useState<WorkbenchLayoutPersistenceStatus>(WORKBENCH_LAYOUT_READY_STATUS);
  const [workspaceSettings, setWorkspaceSettings] = useState<WorkspaceSettingsState>(() =>
    defaultWorkspaceSettingsState(loadViewModeFromStorage(), loadSwarmBoardDefaultOpen()),
  );
  const [workspaceSettingsPersistenceStatus, setWorkspaceSettingsPersistenceStatus] =
    useState<WorkspaceSettingsPersistenceStatus>(WORKSPACE_SETTINGS_READY_STATUS);
  const [userManualSearchRequest, setUserManualSearchRequest] = useState<{
    query: string;
    slug: string | null;
    requestId: number;
  }>({ query: "", slug: null, requestId: 0 });
  const [splitWeights, setSplitWeights] = useState<SplitWeights>(DEFAULT_SPLIT_WEIGHTS);
  const [workbenchLayoutReadyVersion, setWorkbenchLayoutReadyVersion] = useState(0);
  const [workbenchLayoutSaveRetryVersion, setWorkbenchLayoutSaveRetryVersion] = useState(0);
  const workbenchLayoutHydratedRef = useRef(false);
  const workbenchLayoutLoadRef = useRef(0);
  const editorCommandPaletteRequestIdRef = useRef(0);
  const editorFindRequestIdRef = useRef(0);
  const lastSavedWorkbenchLayoutRef = useRef<string | null>(null);
  const pendingWorkbenchLayoutSaveRef = useRef<{
    workspaceId: string;
    layout: WorkbenchLayoutState;
    serialized: string;
  } | null>(null);
  const workbenchLayoutSaveDrainingRef = useRef(false);
  const workbenchLayoutSaveRetryTimerRef = useRef<number | null>(null);
  const currentWorkbenchLayoutRef = useRef<WorkbenchLayoutState>(
    defaultWorkbenchLayoutState(DEFAULT_PROJECTS[0].id),
  );
  const workbenchLayoutSaveRetryAttemptRef = useRef(0);
  const lastWorkbenchLayoutSaveAttemptSerializedRef = useRef<string | null>(null);
  const paneGridRef = useRef<HTMLDivElement>(null);
  const activePane = panes.find((pane) => pane.id === activePaneId) ?? panes[0];
  const activePaneDocumentId = activePane?.activeDocumentId ?? null;
  const activePaneCanvasId = activePane?.activeCanvasId ?? null;
  const activePaneHasRichDocument = activePaneDocumentId?.startsWith("KRD-") ?? false;
  const activeWorkspaceId =
    workspaceListResolved && activeProjectId.trim().length > 0 ? activeProjectId : null;

  const currentWorkbenchLayout = buildWorkbenchLayoutState({
    activePaneId,
    activeModule,
    splitWeights,
    drawers: {
      project: projectDrawerOpen,
      file: fileDrawerOpen,
      bottom: bottomDrawerOpen,
    },
    panes,
  });

  const queueWorkbenchLayoutSaveRetry = useCallback(() => {
    if (workbenchLayoutSaveRetryTimerRef.current !== null) {
      return true;
    }
    const retryDelay = WORKBENCH_LAYOUT_SAVE_RETRY_DELAYS_MS[workbenchLayoutSaveRetryAttemptRef.current];
    if (retryDelay === undefined) {
      return false;
    }
    workbenchLayoutSaveRetryAttemptRef.current += 1;
    workbenchLayoutSaveRetryTimerRef.current = window.setTimeout(() => {
      workbenchLayoutSaveRetryTimerRef.current = null;
      setWorkbenchLayoutSaveRetryVersion((current) => current + 1);
    }, retryDelay);
    return true;
  }, []);

  const startWorkbenchLayoutSaveDrain = useCallback(() => {
    if (workbenchLayoutSaveDrainingRef.current) {
      return;
    }

    workbenchLayoutSaveDrainingRef.current = true;
    void (async () => {
      try {
        while (pendingWorkbenchLayoutSaveRef.current) {
          const next = pendingWorkbenchLayoutSaveRef.current;
          pendingWorkbenchLayoutSaveRef.current = null;
          try {
            await saveWorkbenchLayoutState(next.workspaceId, next.layout);
            workbenchLayoutSaveRetryAttemptRef.current = 0;
            lastWorkbenchLayoutSaveAttemptSerializedRef.current = null;
            setWorkbenchLayoutPersistenceStatus(WORKBENCH_LAYOUT_READY_STATUS);
          } catch {
            if (lastSavedWorkbenchLayoutRef.current === next.serialized) {
              lastSavedWorkbenchLayoutRef.current = null;
            }
            const retryQueued = queueWorkbenchLayoutSaveRetry();
            setWorkbenchLayoutPersistenceStatus({
              state: retryQueued ? "save-pending" : "save-error",
              message: retryQueued
                ? "Layout save failed; retrying durable layout state"
                : "Layout changes are not saved; change the layout to retry",
            });
          }
        }
      } finally {
        workbenchLayoutSaveDrainingRef.current = false;
      }
    })();
  }, [queueWorkbenchLayoutSaveRetry]);

  useEffect(() => {
    saveViewModeToStorage(viewMode);
  }, [viewMode]);

  useEffect(
    () => () => {
      if (workbenchLayoutSaveRetryTimerRef.current !== null) {
        window.clearTimeout(workbenchLayoutSaveRetryTimerRef.current);
      }
    },
    [],
  );

  useEffect(() => {
    currentWorkbenchLayoutRef.current = currentWorkbenchLayout;
  }, [currentWorkbenchLayout]);

  useEffect(() => {
    listWorkspaces()
      .then((result) => {
        const mapped = result.map((workspace) => ({ id: workspace.id, name: workspace.name }));
        if (mapped.length === 0) {
          setProjects([]);
          setActiveProjectId("");
          setWorkspaceListResolved(true);
          return;
        }
        setProjects(mapped);
        setActiveProjectId((current) => (mapped.some((project) => project.id === current) ? current : mapped[0].id));
        setWorkspaceListResolved(true);
      })
      .catch(() => {
        setProjects((prev) => (prev.length > 0 ? prev : DEFAULT_PROJECTS));
        setWorkspaceListResolved(true);
      });
  }, []);

  useEffect(() => {
    if (!workspaceListResolved || !activeWorkspaceId) {
      setWorkspaceSettings(defaultWorkspaceSettingsState(loadViewModeFromStorage(), loadSwarmBoardDefaultOpen()));
      setWorkspaceSettingsPersistenceStatus(WORKSPACE_SETTINGS_READY_STATUS);
      return undefined;
    }

    let cancelled = false;
    const fallbackSettings = defaultWorkspaceSettingsState(
      loadViewModeFromStorage(),
      loadSwarmBoardDefaultOpen(),
    );
    setWorkspaceSettingsPersistenceStatus({
      state: "loading",
      message: "Loading durable settings",
    });

    getWorkspaceSettingsState(activeWorkspaceId)
      .then((record) => {
        if (cancelled) {
          return;
        }
        const nextSettings = normalizeWorkspaceSettingsState(record.settings_state, fallbackSettings);
        setWorkspaceSettings(nextSettings);
        setViewMode(nextSettings.settings.view_mode);
        setWorkspaceSettingsPersistenceStatus(WORKSPACE_SETTINGS_READY_STATUS);
      })
      .catch(() => {
        if (cancelled) {
          return;
        }
        setWorkspaceSettings(fallbackSettings);
        setWorkspaceSettingsPersistenceStatus({
          state: "load-error",
          message: "Settings load failed; local settings are not durable",
        });
      });

    return () => {
      cancelled = true;
    };
  }, [activeWorkspaceId, workspaceListResolved]);

  useEffect(() => {
    if (!workspaceListResolved || !activeWorkspaceId) {
      workbenchLayoutHydratedRef.current = false;
      lastSavedWorkbenchLayoutRef.current = null;
      return undefined;
    }

    let cancelled = false;
    const loadId = workbenchLayoutLoadRef.current + 1;
    workbenchLayoutLoadRef.current = loadId;
    workbenchLayoutHydratedRef.current = false;
    lastSavedWorkbenchLayoutRef.current = null;
    setWorkbenchLayoutPersistenceStatus({
      state: "loading",
      message: "Loading durable layout",
    });
    const baselineLayout = serializeWorkbenchLayoutState(currentWorkbenchLayoutRef.current);

    getWorkbenchLayoutState(activeWorkspaceId)
      .then((record) => {
        if (cancelled || workbenchLayoutLoadRef.current !== loadId) {
          return;
        }
        const hasStoredLayout = record.layout_state !== null && record.layout_state !== undefined;
        const restoredLayout = parseWorkbenchLayoutState(record.layout_state);
        if (hasStoredLayout && !restoredLayout) {
          lastSavedWorkbenchLayoutRef.current = baselineLayout;
          setWorkbenchLayoutPersistenceStatus({
            state: "load-error",
            message: "Layout load failed; stored layout is not renderable",
          });
          return;
        }
        const currentSerialized = serializeWorkbenchLayoutState(currentWorkbenchLayoutRef.current);
        if (currentSerialized !== baselineLayout) {
          lastSavedWorkbenchLayoutRef.current = restoredLayout
            ? serializeWorkbenchLayoutState(restoredLayout)
            : baselineLayout;
          return;
        }

        const nextLayout = restoredLayout ?? defaultWorkbenchLayoutState(activeWorkspaceId);
        setActivePaneId(nextLayout.activePaneId);
        setActiveModule(nextLayout.activeModule);
        setSplitWeights(nextLayout.splitWeights);
        setProjectDrawerOpen(nextLayout.drawers.project);
        setFileDrawerOpen(nextLayout.drawers.file);
        setBottomDrawerOpen(nextLayout.drawers.bottom);
        setPanes(nextLayout.panes);
        lastSavedWorkbenchLayoutRef.current = serializeWorkbenchLayoutState(nextLayout);
        setWorkbenchLayoutPersistenceStatus(WORKBENCH_LAYOUT_READY_STATUS);
      })
      .catch(() => {
        if (cancelled || workbenchLayoutLoadRef.current !== loadId) {
          return;
        }
        lastSavedWorkbenchLayoutRef.current = baselineLayout;
        setWorkbenchLayoutPersistenceStatus({
          state: "load-error",
          message: "Layout load failed; local layout is not durable",
        });
      })
      .finally(() => {
        if (!cancelled && workbenchLayoutLoadRef.current === loadId) {
          workbenchLayoutHydratedRef.current = true;
          setWorkbenchLayoutReadyVersion((current) => current + 1);
        }
      });

    return () => {
      cancelled = true;
    };
  }, [activeWorkspaceId, workspaceListResolved]);

  useEffect(() => {
    if (!workspaceListResolved || !activeWorkspaceId || !workbenchLayoutHydratedRef.current) {
      return;
    }

    const layout = currentWorkbenchLayoutRef.current;
    const serialized = serializeWorkbenchLayoutState(layout);
    if (lastSavedWorkbenchLayoutRef.current === serialized) {
      return;
    }
    lastSavedWorkbenchLayoutRef.current = serialized;
    if (lastWorkbenchLayoutSaveAttemptSerializedRef.current !== serialized) {
      workbenchLayoutSaveRetryAttemptRef.current = 0;
      lastWorkbenchLayoutSaveAttemptSerializedRef.current = serialized;
    }
    pendingWorkbenchLayoutSaveRef.current = {
      workspaceId: activeWorkspaceId,
      layout,
      serialized,
    };
    setWorkbenchLayoutPersistenceStatus({
      state: "save-pending",
      message: "Saving durable layout",
    });
    startWorkbenchLayoutSaveDrain();
  }, [
    activePaneId,
    activeModule,
    activeWorkspaceId,
    bottomDrawerOpen,
    fileDrawerOpen,
    panes,
    projectDrawerOpen,
    splitWeights,
    startWorkbenchLayoutSaveDrain,
    workbenchLayoutReadyVersion,
    workbenchLayoutSaveRetryVersion,
    workspaceListResolved,
  ]);

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
      const handle = window.setTimeout(() => loadKernelDccProjection(), 0);
      return () => window.clearTimeout(handle);
    }
    return undefined;
  }, [loadKernelDccProjection, panes, kernelDccError, kernelDccLoading, kernelDccSurface]);

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

  const addDocumentToPane = (pane: PaneState, documentId: string): PaneState => ({
    ...pane,
    activeTab: "workspace",
    tabs: uniqueTabs(["workspace", ...pane.tabs]),
    activeDocumentId: documentId,
    activeCanvasId: null,
    openDocuments: uniqueOpenDocumentTabs([
      ...pane.openDocuments,
      { documentId, pinned: false, dirty: false },
    ]),
  });

  const openWorkspaceDocument = useCallback((documentId: string, findOptions?: EditorFindOptions) => {
    const targetPaneId = activePaneId;
    setPanes((current) =>
      current.map((pane) => (pane.id === targetPaneId ? addDocumentToPane(pane, documentId) : pane)),
    );
    const query = findOptions?.query.trim() ?? "";
    if (query.length > 0 && findOptions) {
      editorFindRequestIdRef.current += 1;
      setEditorFindRequest({
        paneId: targetPaneId,
        documentId,
        request: {
          requestId: editorFindRequestIdRef.current,
          query,
          caseSensitive: findOptions.caseSensitive,
          wholeWord: findOptions.wholeWord,
          isRegex: findOptions.isRegex,
        },
      });
    }
  }, [activePaneId]);

  const activateDocumentInPane = (paneId: PaneId, documentId: string) => {
    setActivePaneId(paneId);
    setPanes((current) =>
      current.map((pane) =>
        pane.id === paneId
          ? {
              ...pane,
              activeTab: "workspace",
              tabs: uniqueTabs(["workspace", ...pane.tabs]),
              activeDocumentId: documentId,
              activeCanvasId: null,
            }
          : pane,
      ),
    );
  };

  const toggleDocumentPinnedInPane = (paneId: PaneId, documentId: string) => {
    setPanes((current) =>
      current.map((pane) =>
        pane.id === paneId
          ? {
              ...pane,
              openDocuments: pane.openDocuments.map((document) =>
                document.documentId === documentId ? { ...document, pinned: !document.pinned } : document,
              ),
            }
          : pane,
      ),
    );
  };

  const moveDocumentInPane = (paneId: PaneId, documentId: string, direction: -1 | 1) => {
    setPanes((current) =>
      current.map((pane) => {
        if (pane.id !== paneId) {
          return pane;
        }
        const currentIndex = pane.openDocuments.findIndex((document) => document.documentId === documentId);
        const nextIndex = currentIndex + direction;
        if (currentIndex < 0 || nextIndex < 0 || nextIndex >= pane.openDocuments.length) {
          return pane;
        }
        const openDocuments = [...pane.openDocuments];
        const [document] = openDocuments.splice(currentIndex, 1);
        openDocuments.splice(nextIndex, 0, document);
        return { ...pane, openDocuments };
      }),
    );
  };

  const setDocumentDirtyInPane = (paneId: PaneId, documentId: string, dirty: boolean) => {
    setPanes((current) => {
      let changed = false;
      const nextPanes = current.map((pane) => {
        if (pane.id !== paneId) {
          return pane;
        }
        const openDocuments = pane.openDocuments.map((document) => {
          if (document.documentId !== documentId || document.dirty === dirty) {
            return document;
          }
          changed = true;
          return { ...document, dirty };
        });
        return changed ? { ...pane, openDocuments } : pane;
      });
      return changed ? nextPanes : current;
    });
  };

  const moveDocumentBetweenPanes = (sourcePaneId: PaneId, targetPaneId: PaneId, documentId: string) => {
    if (sourcePaneId === targetPaneId) {
      return;
    }
    const sourcePane = panes.find((pane) => pane.id === sourcePaneId);
    const targetPane = panes.find((pane) => pane.id === targetPaneId);
    const sourceDocument = sourcePane?.openDocuments.find((document) => document.documentId === documentId);
    if (!sourcePane || !targetPane || !sourceDocument) {
      return;
    }
    setActivePaneId(targetPaneId);
    setPanes((current) => {
      const currentSourcePane = current.find((pane) => pane.id === sourcePaneId);
      const currentTargetPane = current.find((pane) => pane.id === targetPaneId);
      const currentSourceDocument = currentSourcePane?.openDocuments.find((document) => document.documentId === documentId);
      if (!currentSourcePane || !currentTargetPane || !currentSourceDocument) {
        return current;
      }
      const existingTargetDocument = currentTargetPane.openDocuments.find(
        (document) => document.documentId === documentId,
      );
      const targetDocument = existingTargetDocument
        ? {
            ...existingTargetDocument,
            dirty: existingTargetDocument.dirty || currentSourceDocument.dirty,
            pinned: existingTargetDocument.pinned || currentSourceDocument.pinned,
          }
        : currentSourceDocument;
      return current.map((pane) => {
        if (pane.id === sourcePaneId) {
          const openDocuments = pane.openDocuments.filter((document) => document.documentId !== documentId);
          const activeDocumentId =
            pane.activeDocumentId === documentId ? (openDocuments[0]?.documentId ?? null) : pane.activeDocumentId;
          return {
            ...pane,
            activeDocumentId,
            activeCanvasId: activeDocumentId ? null : pane.activeCanvasId,
            openDocuments,
          };
        }
        if (pane.id === targetPaneId) {
          return {
            ...pane,
            activeTab: "workspace",
            tabs: uniqueTabs(["workspace", ...pane.tabs]),
            activeDocumentId: documentId,
            activeCanvasId: null,
            openDocuments: uniqueOpenDocumentTabs([
              ...pane.openDocuments.filter((document) => document.documentId !== documentId),
              targetDocument,
            ]),
          };
        }
        return pane;
      });
    });
  };

  const handleDocumentTabDragStart =
    (paneId: PaneId, document: OpenDocumentTab) => (event: ReactDragEvent<HTMLDivElement>) => {
      event.dataTransfer.effectAllowed = "move";
      event.dataTransfer.setData(
        DOCUMENT_TAB_DRAG_MIME,
        JSON.stringify({ paneId, documentId: document.documentId }),
      );
    };

  const handleDocumentTabDrop = (targetPaneId: PaneId) => (event: ReactDragEvent<HTMLDivElement>) => {
    const rawPayload = event.dataTransfer.getData(DOCUMENT_TAB_DRAG_MIME);
    if (!rawPayload) {
      return;
    }
    event.preventDefault();
    try {
      const payload = JSON.parse(rawPayload) as { paneId?: unknown; documentId?: unknown };
      if (!isPaneId(payload.paneId) || typeof payload.documentId !== "string") {
        return;
      }
      moveDocumentBetweenPanes(payload.paneId, targetPaneId, payload.documentId);
    } catch {
      return;
    }
  };

  const closeDocumentInPane = (paneId: PaneId, documentId: string, options?: { force?: boolean }) => {
    const targetDocument = panes
      .find((pane) => pane.id === paneId)
      ?.openDocuments.find((document) => document.documentId === documentId);
    if (
      !options?.force &&
      targetDocument?.dirty &&
      !window.confirm(`Close ${documentId} and discard unsaved document changes?`)
    ) {
      return;
    }
    setPanes((current) =>
      current.map((pane) => {
        if (pane.id !== paneId) {
          return pane;
        }
        const nextOpenDocuments = pane.openDocuments.filter((document) => document.documentId !== documentId);
        const nextActiveDocumentId =
          pane.activeDocumentId === documentId ? (nextOpenDocuments[0]?.documentId ?? null) : pane.activeDocumentId;
        return {
          ...pane,
          activeDocumentId: nextActiveDocumentId,
          activeCanvasId: nextActiveDocumentId ? null : pane.activeCanvasId,
          openDocuments: nextOpenDocuments,
        };
      }),
    );
  };

  const openWorkspaceCanvas = (canvasId: string) => {
    setPanes((current) =>
      current.map((pane) =>
        pane.id === activePaneId
          ? {
              ...pane,
              activeTab: "workspace",
              tabs: uniqueTabs(["workspace", ...pane.tabs]),
              activeDocumentId: null,
              activeCanvasId: canvasId,
            }
          : pane,
      ),
    );
  };

  const clearCanvasInPane = (paneId: PaneId) => {
    setPanes((current) =>
      current.map((pane) => {
        if (pane.id !== paneId) {
          return pane;
        }
        const nextActiveDocumentId =
          pane.activeDocumentId &&
          pane.openDocuments.some((document) => document.documentId === pane.activeDocumentId)
            ? pane.activeDocumentId
            : (pane.openDocuments[0]?.documentId ?? null);
        return { ...pane, activeDocumentId: nextActiveDocumentId, activeCanvasId: null };
      }),
    );
  };

  const selectProject = (projectId: string) => {
    if (projectId === activeProjectId) {
      return;
    }
    const nextLayout = defaultWorkbenchLayoutState(projectId);
    setActiveProjectId(projectId);
    setActivePaneId(nextLayout.activePaneId);
    setActiveModule(nextLayout.activeModule);
    setSplitWeights(nextLayout.splitWeights);
    setProjectDrawerOpen(nextLayout.drawers.project);
    setFileDrawerOpen(nextLayout.drawers.file);
    setBottomDrawerOpen(nextLayout.drawers.bottom);
    setPanes(nextLayout.panes);
  };

  const openUserManualPane = useCallback((searchQuery?: string, pageSlug?: string) => {
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
    if (searchQuery !== undefined || pageSlug !== undefined) {
      setUserManualSearchRequest((current) => ({
        query: searchQuery ?? "",
        slug: pageSlug ?? null,
        requestId: current.requestId + 1,
      }));
    }
  }, [activePaneId]);

  const openCodeSymbolPane = useCallback((symbolEntityId: string) => {
    setCodeSymbolEntityId(symbolEntityId);
    setActiveTabForPane(activePaneId, "code-symbol");
  }, [activePaneId, setActiveTabForPane]);

  useEffect(() => {
    configureHandshakeCodeIntelligence({
      workspaceId: activeWorkspaceId,
      openCodeSymbol: (symbolEntityId: string) => {
        setCodeSymbolEntityId(symbolEntityId);
        setActiveTabForPane(activePaneId, "code-symbol");
      },
    });
  }, [activeWorkspaceId, activePaneId, setActiveTabForPane]);

  const openLoomBlockPane = useCallback((blockId: string) => {
    setLoomBlockTarget({ workspaceId: activeProjectId, blockId });
    setActiveTabForPane(activePaneId, "loom-block");
  }, [activePaneId, activeProjectId, setActiveTabForPane]);

  const openLoomWikiPagePane = useCallback((projectionId: string) => {
    setLoomWikiPageTarget({ workspaceId: activeProjectId, projectionId });
    setActiveTabForPane(activePaneId, "loom-wiki-page");
  }, [activePaneId, activeProjectId, setActiveTabForPane]);

  const openKernelDccWorkPacket = useCallback((wpId: string) => {
    setKernelDccFocusTarget({ wpId, mtId: null });
    setActiveTabForPane(activePaneId, "kernel-dcc");
  }, [activePaneId, setActiveTabForPane]);

  const openKernelDccMicroTask = useCallback((target: { mtId: string; wpId?: string | null }) => {
    setKernelDccFocusTarget({ wpId: target.wpId ?? null, mtId: target.mtId });
    setActiveTabForPane(activePaneId, "kernel-dcc");
  }, [activePaneId, setActiveTabForPane]);

  useEffect(
    () =>
      onHsLinkNavigate((detail) => {
        // MT-245 (EXT-NAV-LINK-001): the SAME pure resolver the offline proof
        // harness uses decides which in-app surface owns the typed link. A
        // resolvable target clears the error and opens the surface; an
        // unresolvable one surfaces a typed, visible error — never silent.
        const target = resolveHsLinkTarget(detail);
        if (target.kind === "error") {
          setLinkNavigationError({
            refKind: detail.refKind,
            refValue: detail.refValue,
            label: detail.label,
            message: target.message,
          });
          return;
        }
        setLinkNavigationError(null);
        switch (target.kind) {
          case "document":
            openWorkspaceDocument(target.refValue);
            return;
          case "loom":
            openLoomBlockPane(target.refValue);
            return;
          case "symbol":
            openCodeSymbolPane(target.refValue);
            return;
          case "wp":
            openKernelDccWorkPacket(target.refValue);
            return;
          case "mt":
            openKernelDccMicroTask({ mtId: target.refValue });
            return;
          case "wiki_page":
            openLoomWikiPagePane(target.refValue);
            return;
          case "user_manual":
            openUserManualPane(undefined, target.refValue);
            return;
        }
      }),
    [
      openCodeSymbolPane,
      openKernelDccMicroTask,
      openKernelDccWorkPacket,
      openLoomBlockPane,
      openLoomWikiPagePane,
      openUserManualPane,
      openWorkspaceDocument,
    ],
  );

  const resetWorkbenchLayout = () => {
    const nextLayout = defaultWorkbenchLayoutState(activeProjectId);
    setActivePaneId(nextLayout.activePaneId);
    setActiveModule(nextLayout.activeModule);
    setSplitWeights(nextLayout.splitWeights);
    setProjectDrawerOpen(nextLayout.drawers.project);
    setFileDrawerOpen(nextLayout.drawers.file);
    setBottomDrawerOpen(nextLayout.drawers.bottom);
    setPanes(nextLayout.panes);
  };

  const handleWorkspaceSettingsChange = useCallback(
    (next: WorkspaceSettingsState) => {
      const normalized = normalizeWorkspaceSettingsState(next, workspaceSettings);
      setWorkspaceSettings(normalized);
      setViewMode(normalized.settings.view_mode);

      if (!activeWorkspaceId) {
        setWorkspaceSettingsPersistenceStatus({
          state: "save-error",
          message: "Settings are not saved; no active workspace is selected",
        });
        return;
      }

      setWorkspaceSettingsPersistenceStatus({
        state: "save-pending",
        message: "Saving durable settings",
      });
      saveWorkspaceSettingsState(activeWorkspaceId, normalized)
        .then((record) => {
          const stored = normalizeWorkspaceSettingsState(record.settings_state, normalized);
          setWorkspaceSettings(stored);
          setViewMode(stored.settings.view_mode);
          setWorkspaceSettingsPersistenceStatus(WORKSPACE_SETTINGS_READY_STATUS);
        })
        .catch(() => {
          setWorkspaceSettingsPersistenceStatus({
            state: "save-error",
            message: "Settings changes are not saved; change a setting to retry",
          });
        });
    },
    [activeWorkspaceId, workspaceSettings],
  );

  const handleHeaderViewModeChange = (next: ViewMode) => {
    handleWorkspaceSettingsChange({
      ...workspaceSettings,
      settings: {
        ...workspaceSettings.settings,
        view_mode: next,
      },
    });
  };

  useEffect(() => {
    const handleAppCommandPaletteCapture = (event: globalThis.KeyboardEvent) => {
      if (event.defaultPrevented) {
        return;
      }
      const commandPaletteChord = workspaceSettings.keybindings["app.command_palette.open"];
      if (!keyboardEventMatchesChord(event, commandPaletteChord)) {
        return;
      }
      event.preventDefault();
      setAppCommandPaletteOpen(true);
    };

    const handleGlobalKeyDown = (event: globalThis.KeyboardEvent) => {
      if (
        event.ctrlKey &&
        event.altKey &&
        (event.key === "ArrowRight" ||
          event.key === "ArrowLeft" ||
          event.key === "ArrowDown" ||
          event.key === "ArrowUp")
      ) {
        event.preventDefault();
        setActivePaneId((current) => paneIdForKeyboardNavigation(current, event.key));
        return;
      }
      if (event.defaultPrevented) {
        return;
      }
      const commandPaletteChord = workspaceSettings.keybindings["app.command_palette.open"];
      const quickSwitcherChord = workspaceSettings.keybindings["app.quick_switcher.open"];
      if (keyboardEventMatchesChord(event, commandPaletteChord)) {
        event.preventDefault();
        setAppCommandPaletteOpen(true);
        return;
      }
      if (!keyboardEventMatchesChord(event, quickSwitcherChord)) {
        return;
      }
      event.preventDefault();
      setQuickSwitcherOpen(true);
    };

    window.addEventListener("keydown", handleAppCommandPaletteCapture, true);
    window.addEventListener("keydown", handleGlobalKeyDown);
    return () => {
      window.removeEventListener("keydown", handleAppCommandPaletteCapture, true);
      window.removeEventListener("keydown", handleGlobalKeyDown);
    };
  }, [workspaceSettings.keybindings]);

  const appCommandActions = buildAppCommandRegistry({
    searchText,
    editorCommandsEnabled: activePaneHasRichDocument,
  });

  const runAppCommand = (actionId: string) => {
    const editorCommand = resolveEditorAppCommand(appCommandActions, actionId);
    if (editorCommand) {
      if (!activePaneDocumentId || !activePaneHasRichDocument || !editorCommand.editorQuery) {
        return;
      }
      editorCommandPaletteRequestIdRef.current += 1;
      setActiveTabForPane(activePaneId, "workspace");
      setEditorCommandPaletteRequest({
        paneId: activePaneId,
        documentId: activePaneDocumentId,
        requestId: editorCommandPaletteRequestIdRef.current,
        query: editorCommand.editorQuery,
      });
      setAppCommandPaletteOpen(false);
      return;
    }

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
      const activeDocumentId = pane.activeDocumentId;
      content = (
        <>
          <DebugPanel />
          <div className="content-main">
            {pane.openDocuments.length > 0 ? (
              <div
                className="workspace-document-tabs"
                data-stable-id={`pane-${pane.id}-document-tabs`}
                data-testid={`pane-${pane.id}.document-tabs`}
                onDragOver={(event) => {
                  const dragTypes = Array.from(event.dataTransfer.types ?? []);
                  if (
                    dragTypes.includes(DOCUMENT_TAB_DRAG_MIME) ||
                    event.dataTransfer.getData(DOCUMENT_TAB_DRAG_MIME)
                  ) {
                    event.preventDefault();
                    event.dataTransfer.dropEffect = "move";
                  }
                }}
                onDrop={handleDocumentTabDrop(pane.id)}
              >
                {pane.openDocuments.map((document, index) => {
                  const documentStablePart = stableIdPart(document.documentId);
                  const tabId = `pane-${pane.id}.document-tab.${documentStablePart}`;
                  const isActive = activeDocumentId === document.documentId;
                  return (
                    <div
                      key={document.documentId}
                      className={
                        isActive
                          ? "workspace-document-tab workspace-document-tab--active"
                          : "workspace-document-tab"
                      }
                      data-active={isActive ? "true" : "false"}
                      data-document-id={document.documentId}
                      data-dirty={document.dirty ? "true" : "false"}
                      data-pinned={document.pinned ? "true" : "false"}
                      data-testid={tabId}
                      draggable
                      onDragStart={handleDocumentTabDragStart(pane.id, document)}
                    >
                      <button
                        type="button"
                        className="workspace-document-tab__activate"
                        onClick={() => activateDocumentInPane(pane.id, document.documentId)}
                        data-testid={`${tabId}.activate`}
                      >
                        {document.documentId}
                      </button>
                      {document.dirty ? <span className="workspace-document-tab__dirty" aria-label="Unsaved" /> : null}
                      <button
                        type="button"
                        className="workspace-document-tab__pin"
                        aria-label={`${document.pinned ? "Unpin" : "Pin"} ${document.documentId}`}
                        aria-pressed={document.pinned}
                        onClick={(event) => {
                          event.stopPropagation();
                          toggleDocumentPinnedInPane(pane.id, document.documentId);
                        }}
                        data-testid={`${tabId}.pin`}
                      >
                        Pin
                      </button>
                      <button
                        type="button"
                        className="workspace-document-tab__move"
                        aria-label={`Move ${document.documentId} left`}
                        disabled={index === 0}
                        onClick={(event) => {
                          event.stopPropagation();
                          moveDocumentInPane(pane.id, document.documentId, -1);
                        }}
                        data-testid={`${tabId}.move-left`}
                      >
                        &lt;
                      </button>
                      <button
                        type="button"
                        className="workspace-document-tab__move"
                        aria-label={`Move ${document.documentId} right`}
                        disabled={index === pane.openDocuments.length - 1}
                        onClick={(event) => {
                          event.stopPropagation();
                          moveDocumentInPane(pane.id, document.documentId, 1);
                        }}
                        data-testid={`${tabId}.move-right`}
                      >
                        &gt;
                      </button>
                      <button
                        type="button"
                        className="workspace-document-tab__close"
                        aria-label={`Close ${document.documentId}`}
                        onClick={(event) => {
                          event.stopPropagation();
                          closeDocumentInPane(pane.id, document.documentId);
                        }}
                        data-testid={`${tabId}.close`}
                      >
                        Close
                      </button>
                    </div>
                  );
                })}
              </div>
            ) : null}
            {activeDocumentId ? (
              <DocumentView
                documentId={activeDocumentId}
                onDirtyChange={(dirty) => setDocumentDirtyInPane(pane.id, activeDocumentId, dirty)}
                commandPaletteRequest={
                  editorCommandPaletteRequest?.paneId === pane.id &&
                  editorCommandPaletteRequest.documentId === activeDocumentId
                    ? editorCommandPaletteRequest
                    : null
                }
                findRequest={
                  editorFindRequest?.paneId === pane.id && editorFindRequest.documentId === activeDocumentId
                    ? editorFindRequest.request
                    : null
                }
                onDeleted={() => {
                  closeDocumentInPane(pane.id, activeDocumentId, { force: true });
                }}
              />
            ) : pane.activeCanvasId ? (
              <CanvasView
                canvasId={pane.activeCanvasId}
                onDeleted={() => {
                  clearCanvasInPane(pane.id);
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
            focusedWorkTarget={kernelDccFocusTarget}
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
          initialSlug={userManualSearchRequest.slug ?? undefined}
          initialSearchQuery={userManualSearchRequest.query}
          searchRequestId={userManualSearchRequest.requestId}
        />
      );
    } else if (pane.activeTab === "code-symbol") {
      content = codeSymbolEntityId ? (
        <CodeSymbolPanel symbolEntityId={codeSymbolEntityId} workspaceId={activeWorkspaceId} />
      ) : (
        <div className="content-card" data-testid="code-symbol-panel">
          <h2>Code Symbol</h2>
          <p className="muted">No code symbol selected.</p>
        </div>
      );
    } else if (pane.activeTab === "source-control") {
      content = <SourceControlPanel />;
    } else if (pane.activeTab === "loom-block") {
      content = loomBlockTarget ? (
        <LoomBlockPanel workspaceId={loomBlockTarget.workspaceId} blockId={loomBlockTarget.blockId} />
      ) : (
        <div className="content-card" data-testid="loom-block-panel">
          <h2>Loom Block</h2>
          <p className="muted">No Loom block selected.</p>
        </div>
      );
    } else if (pane.activeTab === "loom-daily-journal") {
      content = activeWorkspaceId ? (
        <LoomDailyJournalPanel workspaceId={activeWorkspaceId} />
      ) : (
        <div className="content-card" data-testid="loom-daily-journal-panel">
          <h2>Daily Journal</h2>
          <p className="muted">No workspace selected.</p>
        </div>
      );
    } else if (pane.activeTab === "loom-wiki-page") {
      content = loomWikiPageTarget ? (
        <LoomWikiPagePanel
          workspaceId={loomWikiPageTarget.workspaceId}
          projectionId={loomWikiPageTarget.projectionId}
        />
      ) : (
        <div className="content-card" data-testid="loom-wiki-page-panel">
          <h2>Wiki Page</h2>
          <p className="muted">No wiki page selected.</p>
        </div>
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
        data-pane-active-document-id={pane.activeDocumentId ?? ""}
        data-pane-active-canvas-id={pane.activeCanvasId ?? ""}
        data-pane-open-document-count={pane.openDocuments.length}
        data-pane-active={pane.id === activePaneId ? "true" : "false"}
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
      data-active-pane-id={activePaneId}
      data-active-module={activeModule}
      data-active-project-id={activeProjectId}
      data-project-drawer-open={projectDrawerOpen ? "true" : "false"}
      data-file-drawer-open={fileDrawerOpen ? "true" : "false"}
      data-bottom-drawer-open={bottomDrawerOpen ? "true" : "false"}
      data-split-weights={`${splitWeights.vertical.toFixed(3)},${splitWeights.horizontal.toFixed(3)}`}
      data-theme={workspaceSettings.theme}
      data-link-navigation-state={linkNavigationError ? "error" : "ok"}
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
            <ViewModeToggle value={viewMode} onChange={handleHeaderViewModeChange} />
            <SystemStatus />
            {workspaceSettingsPersistenceStatus.state !== "ready" ? (
              <p className="workspace-settings-status" data-testid="workspace-settings-status">
                {workspaceSettingsPersistenceStatus.message}
              </p>
            ) : null}
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
                          onClick={() => selectProject(project.id)}
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
                  activeWorkspaceId={activeWorkspaceId}
                  onSelectWorkspace={selectProject}
                  selectedDocumentId={activePaneDocumentId}
                  selectedCanvasId={activePaneCanvasId}
                  onSelectDocument={(id) => {
                    if (id !== null) {
                      openWorkspaceDocument(id);
                    }
                  }}
                  onSelectCanvas={(id) => {
                    if (id !== null) {
                      openWorkspaceCanvas(id);
                    } else {
                      clearCanvasInPane(activePaneId);
                    }
                  }}
                  onOpenLoomBlock={openLoomBlockPane}
                  onWorkspaceDeleted={() => {
                    setPanes((current) =>
                      current.map((pane) => ({
                        ...pane,
                        activeDocumentId: null,
                        activeCanvasId: null,
                        openDocuments: [],
                      })),
                    );
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
                <span
                  className={
                    workbenchLayoutPersistenceStatus.state === "ready"
                      ? "workbench-layout-status"
                      : "workbench-layout-status workbench-layout-status--attention"
                  }
                  data-stable-id="workbench-layout.status"
                  data-testid="workbench-layout.status"
                  data-layout-persistence-state={workbenchLayoutPersistenceStatus.state}
                >
                  {workbenchLayoutPersistenceStatus.message}
                </span>
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
                <button
                  type="button"
                  onClick={() => setQuickSwitcherOpen(true)}
                  data-stable-id="quick-switcher.open"
                  data-testid="quick-switcher.open"
                >
                  Quick Open
                </button>
                <button
                  type="button"
                  onClick={() => setWorkspaceSearchOpen((value) => !value)}
                  data-stable-id="workspace-search.open"
                  data-testid="workspace-search.open"
                >
                  Find in Files
                </button>
                <button
                  type="button"
                  onClick={() => setLoomSearchV2Open((value) => !value)}
                  data-stable-id="loom-search-v2.open"
                  data-testid="loom-search-v2.open"
                >
                  Loom Search
                </button>
                <button
                  type="button"
                  onClick={() => setLoomAiReviewOpen((value) => !value)}
                  data-stable-id="loom-ai-review.open"
                  data-testid="loom-ai-review.open"
                >
                  AI Loom Review
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
                  {workspaceSearchOpen ? (
                    <WorkspaceSearchPanel
                      open={true}
                      workspaceId={activeWorkspaceId}
                      onClose={() => setWorkspaceSearchOpen(false)}
                      onOpenDocument={openWorkspaceDocument}
                      onOpenLoomBlock={openLoomBlockPane}
                      onOpenCodeSymbol={openCodeSymbolPane}
                      onOpenMicroTask={openKernelDccMicroTask}
                      onOpenWorkPacket={openKernelDccWorkPacket}
                      onOpenUserManualPage={(slug) => openUserManualPane(undefined, slug)}
                      onOpenWikiPage={openLoomWikiPagePane}
                    />
                  ) : null}
                  {loomSearchV2Open ? (
                    <LoomSearchV2Panel
                      workspaceId={activeWorkspaceId}
                      onOpenBlock={openLoomBlockPane}
                    />
                  ) : null}
                  {loomAiReviewOpen && activeWorkspaceId ? (
                    <LoomAiReviewPanel
                      workspaceId={activeWorkspaceId}
                      onClose={() => setLoomAiReviewOpen(false)}
                    />
                  ) : null}
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
        workspaceSettings={workspaceSettings}
        onWorkspaceSettingsChange={handleWorkspaceSettingsChange}
        onResetLayout={resetWorkbenchLayout}
      />
      <CommandPalette
        open={appCommandPaletteOpen}
        title="App commands"
        actions={appCommandActions}
        onAction={runAppCommand}
        onClose={() => setAppCommandPaletteOpen(false)}
      />
      {quickSwitcherOpen ? (
        <QuickSwitcher
          open={true}
          workspaceId={activeWorkspaceId}
          onClose={() => setQuickSwitcherOpen(false)}
          onOpenCodeSymbol={openCodeSymbolPane}
          onOpenDocument={openWorkspaceDocument}
          onOpenLoomBlock={openLoomBlockPane}
          onOpenMicroTask={openKernelDccMicroTask}
          onOpenWorkPacket={openKernelDccWorkPacket}
          onOpenUserManualPage={(slug) => openUserManualPane(undefined, slug)}
          onOpenWikiPage={openLoomWikiPagePane}
        />
      ) : null}
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
