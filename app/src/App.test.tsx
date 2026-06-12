import { readFileSync } from "node:fs";
import { vi } from "vitest";

vi.mock("@excalidraw/excalidraw", () => ({
  __esModule: true,
  Excalidraw: () => null,
}));

vi.mock("./components/WorkspaceSidebar", () => ({
  WorkspaceSidebar: () => <div data-testid="workspace-sidebar">Workspace Sidebar</div>,
}));

vi.mock("./components/SystemStatus", () => ({
  SystemStatus: () => <div data-testid="system-status">Coordinator: OK</div>,
}));

vi.mock("./lib/api", () => ({
  listWorkspaces: vi.fn(async () => [
    {
      id: "w1",
      name: "Workspace 1",
      created_at: "2025-01-01T00:00:00Z",
      updated_at: "2025-01-01T00:00:00Z",
    },
  ]),
  createWorkspace: vi.fn(async (name: string) => ({
    id: "w-new",
    name,
    created_at: "2025-01-01T00:00:00Z",
    updated_at: "2025-01-01T00:00:00Z",
  })),
  listDocuments: vi.fn(async () => []),
  createDocument: vi.fn(async (workspaceId: string, title: string) => ({
    id: "doc-1",
    workspace_id: workspaceId,
    title,
    created_at: "2025-01-01T00:00:00Z",
    updated_at: "2025-01-01T00:00:00Z",
  })),
  listCanvases: vi.fn(async () => []),
  createCanvas: vi.fn(async (workspaceId: string, title: string) => ({
    id: "canvas-1",
    workspace_id: workspaceId,
    title,
    created_at: "2025-01-01T00:00:00Z",
    updated_at: "2025-01-01T00:00:00Z",
  })),
  getDocument: vi.fn(async () => ({
    id: "doc-1",
    workspace_id: "w1",
    title: "Doc 1",
    created_at: "2025-01-01T00:00:00Z",
    updated_at: "2025-01-01T00:00:00Z",
    blocks: [],
  })),
  updateDocumentBlocks: vi.fn(async () => []),
  getCanvas: vi.fn(async (id: string) => ({
    id,
    workspace_id: "w1",
    title: "Canvas 1",
    created_at: "2025-01-01T00:00:00Z",
    updated_at: "2025-01-01T00:00:00Z",
    nodes: [],
    edges: [],
  })),
  updateCanvasGraph: vi.fn(async (canvasId: string) => ({
    id: canvasId,
    workspace_id: "w1",
    title: "Canvas 1",
    created_at: "2025-01-01T00:00:00Z",
    updated_at: "2025-01-01T00:00:00Z",
    nodes: [],
    edges: [],
  })),
  getEvents: vi.fn(async () => []),
  // WP-KERNEL-005: AtelierPanel is wired into the CKC module pane and fetches
  // on mount; the explicit mock factory must cover its api surface or the
  // unmocked call throws inside the mount effect.
  getAtelierOverview: vi.fn(async () => ({ tables: [], event_families: [] })),
  getAtelierIntakeItems: vi.fn(async () => ({ batch: null, items: [] })),
  listAtelierCommandCorpus: vi.fn(async () => []),
  listAtelierIntakeBatches: vi.fn(async () => []),
  listAtelierStealthWindows: vi.fn(async () => []),
  openAtelierIntakeBatch: vi.fn(async () => ({})),
  listUserManualAccessPoints: vi.fn(async () => ({
    count: 3,
    access_points: [
      {
        access_point_id: "ap.diagnostics.manual_tab",
        host_surface: "diagnostics",
        entry_kind: "panel",
        target_page_slug: "manual-toc",
        ui_wiring_route: "/usermanual/pages/manual-toc",
        stable_element_id: "hs-usermanual-diagnostics-tab",
        note: "Diagnostics manual tab",
        target_resolves: true,
      },
      {
        access_point_id: "ap.command_palette.open_manual",
        host_surface: "command_palette",
        entry_kind: "command",
        target_page_slug: "manual-toc",
        ui_wiring_route: "/usermanual/pages/manual-toc",
        stable_element_id: "hs-usermanual-palette-open",
        note: "Palette open",
        target_resolves: true,
      },
      {
        access_point_id: "ap.command_palette.search_manual",
        host_surface: "command_palette",
        entry_kind: "command",
        target_page_slug: "manual-toc",
        ui_wiring_route: "/usermanual/search",
        stable_element_id: "hs-usermanual-palette-search",
        note: "Palette search",
        target_resolves: true,
      },
    ],
  })),
  listUserManualPages: vi.fn(async () => ({
    manual_version: "2.0.0",
    route_namespace: "/usermanual",
    count: 1,
    pages: [
      {
        slug: "manual-toc",
        title: "Manual TOC",
        page_kind: "guide",
        audience: "model",
        manual_version: "2.0.0",
        content_hash: "hash-manual-toc",
        status: "current",
        updated_at: "2026-06-12T00:00:00Z",
      },
    ],
  })),
  getUserManualPage: vi.fn(async () => ({
    page: {
      page_id: "page-manual-toc",
      slug: "manual-toc",
      title: "Manual TOC",
      page_kind: "guide",
      audience: "model",
      manual_version: "2.0.0",
      content_hash: "hash-manual-toc",
      status: "current",
      updated_at: "2026-06-12T00:00:00Z",
    },
    sections: [
      {
        section_id: "section-navigation",
        page_id: "page-manual-toc",
        position: 0,
        section_kind: "body",
        title: "Navigation",
        body_md: "Start with the page index, then open a task-sized guide.",
        body_json: null,
      },
    ],
    anchors: [],
    bootstrap_receipt_event_id: "evt-manual-opened",
    bootstrap_identity_used: true,
  })),
  searchUserManual: vi.fn(async () => ({
    query: "recovery",
    count: 1,
    results: [
      {
        result_kind: "section",
        result_ref: "section-recovery",
        page_slug: "manual-toc",
        title: "Manual recovery",
        excerpt: "Recover failed startup state.",
      },
    ],
  })),
  getKernelDccProjection: vi.fn(async () => ({
    schema_id: "hsk.kernel.dcc_mvp_runtime_surface@1",
    surface_id: "dcc-app-backend-test",
    folded_stub_id: "WP-1-Dev-Command-Center-MVP-v1",
    panels: [
      {
        panel_id: "panel-app-backend-test",
        kind: "WorkSelection",
        projection_only: true,
        source_refs: ["kernel.action_catalog"],
        visible_state_fields: ["wp_id"],
      },
    ],
    work_items: [
      {
        work_id: "work-app-backend-123",
        wp_id: "WP-KERNEL-002",
        mt_id: "MT-DCC-APP",
        status: "BACKEND_LOADED",
        worktree_id: "wt-app-backend",
        session_ids: ["session-app-backend"],
        proposal_ids: ["proposal-app-backend"],
        evidence_ids: ["evidence-app-backend"],
        allowed_action_ids: ["kernel.write_box.promote"],
      },
    ],
    worktrees: [
      {
        worktree_id: "wt-app-backend",
        path_ref: "worktree://app-backend",
        branch: "codex/app-backend",
        dirty: false,
        diff_ref: "evidence-app-backend",
        linked_work_ids: ["work-app-backend-123"],
      },
    ],
    sessions: [
      {
        session_id: "session-app-backend",
        role: "CODER",
        model_id: "gpt-test",
        backend: "codex",
        worktree_id: "wt-app-backend",
        wp_id: "WP-KERNEL-002",
        mt_id: "MT-DCC-APP",
        state: "ACTIVE",
      },
    ],
    proposals: [
      {
        proposal_id: "proposal-app-backend",
        work_id: "work-app-backend-123",
        action_id: "kernel.write_box.promote",
        status: "Approved",
        evidence_ids: ["evidence-app-backend"],
        approval_preview_id: "approval-app-backend",
      },
    ],
    evidence: [
      {
        evidence_id: "evidence-app-backend",
        kind: "ValidationOutput",
        evidence_ref: "validation://app-backend",
        work_id: "work-app-backend-123",
      },
    ],
    approval_previews: [
      {
        preview_id: "approval-app-backend",
        action_id: "kernel.write_box.promote",
        scope_options: ["Once"],
        requires_same_turn_approval: false,
        denied_failure_code: "APP_DENIED",
      },
    ],
    write_box_queue_rows: [
      {
        row_id: "write-box-row-app-backend",
        write_box_id: "wb-app-backend",
        work_id: "work-app-backend-123",
        kind: "Promotion",
        lifecycle_state: "Validated",
        actor_id: "actor-app-backend",
        target_refs: ["authority://app-backend"],
        validation_state: "Valid",
        denial_receipt_refs: [],
        promotion_receipt_refs: ["receipt://app-backend"],
        event_ledger_event_refs: [],
        stale_state_vector: false,
        stable_element_id: "dcc.write_box_queue.row.wb-app-backend",
      },
    ],
    direct_edit_denials: [
      {
        row_id: "direct-edit-denial-row-app-backend",
        denial_id: "denial-app-backend",
        work_id: "work-app-backend-123",
        actor_id: "actor-app-backend",
        target_ref: "authority://app-backend",
        attempted_action: "raw_authority_file_write",
        recovery_instruction: "Use a registered write-box action",
        ui_response_ref: "dcc://denials/app-backend",
        api_response_ref: "api://denials/app-backend",
        stable_element_id: "dcc.direct_edit_denial.row.app-backend",
      },
    ],
    promotion_previews: [
      {
        row_id: "promotion-preview-row-app-backend",
        preview_id: "promotion-app-backend",
        work_id: "work-app-backend-123",
        write_box_id: "wb-app-backend",
        promotion_target_ref: "authority://app-backend",
        request_event_ref: "eventledger://app-backend/requested",
        accepted_event_ref: null,
        rejected_event_ref: null,
        state_vector: "sv-app-backend",
        validation_check_summaries: ["promotion_gate_input_alignment: PASS"],
        idempotency_key: "promotion:bridge-app-backend:requested",
        expected_event_kinds: [
          "KernelCrdtPromotionRequestedV1",
          "KernelCrdtPromotionAcceptedV1",
        ],
        stale_risk: "None",
        freshness_badge_id: "freshness-app-backend",
        stable_element_id: "dcc.promotion_preview.row.wb-app-backend",
      },
    ],
    freshness_badges: [
      {
        badge_id: "freshness-app-backend",
        source_projection_id: "dcc-app-backend-projection",
        source_ref: "eventledger://app-backend",
        state_vector: "sv-app-backend",
        updated_at_ref: "eventledger://app-backend/latest",
        stale: false,
        stable_element_id: "dcc.freshness_badge.app-backend",
      },
    ],
    stable_element_ids: [
      {
        element_id: "dcc.write_box_queue.row.wb-app-backend",
        surface_id: "dcc-app-backend-test",
        element_kind: "write_box_queue_row",
        source_ref: "writebox://wb-app-backend",
      },
    ],
    catalog_action_refs: ["kernel.write_box.promote"],
    catalog_action_rows: [
      {
        action_id: "kernel.write_box.promote",
        target_authority_class: "EventLedgerAuthorityWrite",
        input_schema_id: "hsk.kernel.write_box_promote_input@1",
        result_schema_id: "hsk.kernel.write_box_promote_result@1",
        role_eligibility: ["CODER", "KERNEL_BUILDER"],
        capability_requirements: ["promotion.gate"],
        approval_posture: "RequiresPromotionGate",
        preview_behavior_summary:
          "Promote a validated write box through the promotion gate into authority state.",
        preview_panel_id: "dcc-promote-preview",
      },
    ],
    direct_authority_mutation_allowed: false,
    ungoverned_tool_execution_allowed: false,
    destructive_git_ops_require_same_turn_approval: true,
    flight_recorder_event_types: ["dcc.work.selected"],
    product_authority_refs: ["kernel.write_box.queue"],
    folded_source_refs: [".GOV/task_packets/stubs/WP-1-Dev-Command-Center-MVP-v1.contract.json"],
    spawn_tree_projection: {
      schema_id: "hsk.kernel.session_spawn_tree_dcc_projection@1",
      tree_id: "app-route-spawn-tree",
      panel_id: "session-spawn-tree",
      visible_fields: [
        "SpawnHierarchy",
        "ChildCounts",
        "SpawnDepth",
        "CascadeCancel",
        "SpawnMode",
        "AnnounceBackBadges",
      ],
      nodes: [
        {
          session_id: "session-app-backend",
          parent_session_id: null,
          role_id: "CODER",
          depth: 0,
          child_count: 0,
          active_child_count: 0,
          spawn_mode: "SessionPersistent",
          runtime_state: "Active",
          cascade_cancel_available: true,
          announce_back_badges: ["announce-back-ready"],
        },
      ],
      max_depth: 0,
      cascade_cancel_session_ids: ["session-app-backend"],
      announce_back_badge_count: 1,
      runtime_record_refs: ["runtime://session-spawn/session-app-backend"],
      mutates_runtime_records: false,
    },
  })),
  triggerKernelDccAction: vi.fn(async () => ({
    schema_id: "hsk.kernel.dcc_governed_action_trigger_result@1",
    work_id: "work-app-backend-123",
    action_id: "kernel.write_box.promote",
    triggered: true,
    catalog_checked: true,
    preview_checked: true,
    gate_enforced: true,
    approval_preview_id: "approval-app-backend",
    authority_effect: "EventLedgerAuthorityWrite",
    approval_posture: "RequiresPromotionGate",
    expected_write_box_kinds: ["PromotionBox"],
    receipt_ref: "receipt://kernel-dcc/action-trigger/work-app-backend-123/kernel.write_box.promote",
  })),
  createDiagnostic: vi.fn(async () => ({})),
}));

import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import App from "./App";
import { FlightRecorderView } from "./components/FlightRecorderView";
import {
  createDiagnostic,
  getEvents,
  getKernelDccProjection,
  getUserManualPage,
  listUserManualAccessPoints,
  listUserManualPages,
  listWorkspaces,
  searchUserManual,
  type FlightEvent,
} from "./lib/api";

it("renders desktop shell header and shows coordinator status", () => {
  render(<App />);

  expect(screen.getByText(/Desktop Shell/i)).toBeInTheDocument();
  expect(screen.getByTestId("system-status")).toBeInTheDocument();
});

it("renders main window shell and stable layout hooks", () => {
  render(<App />);

  const mainWindow = screen.getByTestId("main-window");

  expect(mainWindow).toHaveAttribute("data-stable-layout", "main-window-v1");
  expect(mainWindow).toHaveAttribute("data-stable-id", "main-window");
  expect(mainWindow).toHaveAttribute("data-project-drawer-open", "true");
  expect(mainWindow).toHaveAttribute("data-file-drawer-open", "true");
  expect(mainWindow).toHaveAttribute("data-bottom-drawer-open", "true");
  expect(screen.getByTestId("module-rail")).toBeInTheDocument();
  expect(screen.getByTestId("file-drawer")).toBeInTheDocument();
  expect(screen.getByTestId("pane-grid")).toBeInTheDocument();
});

it("switches active project by project tab", async () => {
  vi.mocked(listWorkspaces).mockResolvedValueOnce([
    { id: "project-main", name: "Project Main", created_at: "2025-01-01T00:00:00Z", updated_at: "2025-01-01T00:00:00Z" },
    { id: "project-aux", name: "Project Aux", created_at: "2025-01-01T00:00:00Z", updated_at: "2025-01-01T00:00:00Z" },
  ]);

  render(<App />);
  const mainWindow = screen.getByTestId("main-window");

  fireEvent.click(await screen.findByTestId("project-project-aux"));
  expect(mainWindow).toHaveAttribute("data-active-project-id", "project-aux");
});

it("switches module context and keeps pane context wired", async () => {
  render(<App />);

  const mainWindow = screen.getByTestId("main-window");
  fireEvent.click(screen.getByTestId("module-ckc"));
  expect(mainWindow).toHaveAttribute("data-active-module", "CKC");

  // WP-KERNEL-005: the CKC module pane defaults to the Handshake-native
  // Atelier surface; the Kernel DCC projection stays reachable as a pane tab.
  expect(screen.getByTestId("pane-pane-a").getAttribute("data-pane-type")).toBe("atelier");

  fireEvent.click(await screen.findByTestId("pane-pane-a.tab.kernel-dcc"));
  expect(screen.getByTestId("pane-pane-a").getAttribute("data-pane-type")).toBe("kernel-dcc");
});

it("switches pane-local tabs within one pane", async () => {
  render(<App />);

  const jobsTab = await screen.findByTestId("pane-pane-a.tab.jobs");
  fireEvent.click(jobsTab);

  expect(jobsTab).toHaveClass("main-pane__tab--active");
});

it("updates split weights from splitters", async () => {
  render(<App />);

  const mainWindow = screen.getByTestId("main-window");
  const paneGrid = screen.getByTestId("pane-grid");
  const getBoundingClientRectSpy = vi.spyOn(paneGrid, "getBoundingClientRect").mockReturnValue({
    width: 1200,
    height: 900,
    top: 0,
    left: 0,
    right: 1200,
    bottom: 900,
    x: 0,
    y: 0,
    toJSON: () => ({}),
  } as DOMRect);

  try {
    const splitter = screen.getByTestId("main-window-splitter-vertical");
    fireEvent.pointerDown(splitter, { clientX: 600, clientY: 0, pointerId: 9 });
    fireEvent.pointerMove(window.document, { clientX: 700, clientY: 0, pointerId: 9 });
    fireEvent.pointerUp(window.document, { clientX: 700, clientY: 0, pointerId: 9 });

    const pointerChanged = mainWindow.getAttribute("data-split-weights");
    expect(pointerChanged).not.toEqual("0.500,0.550");

    fireEvent.keyDown(splitter, { key: "ArrowLeft" });
    await waitFor(() => expect(mainWindow).not.toHaveAttribute("data-split-weights", pointerChanged ?? ""));
  } finally {
    getBoundingClientRectSpy.mockRestore();
  }
});

it("toggles bottom drawer state in DOM attributes", () => {
  render(<App />);

  const mainWindow = screen.getByTestId("main-window");
  const bottomToggle = screen.getByTestId("bottom-drawer.toggle");

  fireEvent.click(bottomToggle);
  expect(mainWindow).toHaveAttribute("data-bottom-drawer-open", "false");
  expect(screen.queryByTestId("search-status-region")).not.toBeInTheDocument();

  fireEvent.click(bottomToggle);
  expect(mainWindow).toHaveAttribute("data-bottom-drawer-open", "true");
  expect(screen.getByTestId("search-status-region")).toBeInTheDocument();
});

it("loads the backend Kernel DCC projection when module pane is set to CKC", async () => {
  vi.mocked(getKernelDccProjection).mockClear();

  render(<App />);

  fireEvent.click(screen.getByTestId("module-ckc"));

  expect(screen.getByTestId("main-window")).toHaveAttribute("data-active-module", "CKC");
  // WP-KERNEL-005: CKC defaults to the Atelier tab; select the Kernel DCC tab
  // explicitly before asserting the projection loads.
  fireEvent.click(await screen.findByTestId("pane-pane-a.tab.kernel-dcc"));
  expect((await screen.findAllByText("work-app-backend-123")).length).toBeGreaterThan(0);
  expect(screen.getByText("panel-app-backend-test")).toBeInTheDocument();
  expect(screen.getAllByText("wb-app-backend").length).toBeGreaterThan(0);
  expect(screen.getByText("Session Spawn Tree")).toBeInTheDocument();
  expect(screen.getAllByText("session-app-backend").length).toBeGreaterThan(0);
  expect(document.querySelector('[data-stable-id="dcc.session_spawn_tree.node.session-app-backend"]')).not.toBeNull();
  await waitFor(() => expect(vi.mocked(getKernelDccProjection)).toHaveBeenCalledTimes(1));
});

it("opens the UserManual diagnostics tab from the app command palette", async () => {
  vi.mocked(listUserManualAccessPoints).mockClear();
  vi.mocked(listUserManualPages).mockClear();
  vi.mocked(getUserManualPage).mockClear();

  render(<App />);

  fireEvent.click(screen.getByTestId("app-command-palette.open"));
  fireEvent.click(await screen.findByTestId("command-palette-action-hs-usermanual-palette-open"));

  const panel = await screen.findByTestId("usermanual-panel");
  expect(panel).toHaveAttribute("data-stable-id", "hs-usermanual-panel");
  expect(screen.getByTestId("pane-pane-a")).toHaveAttribute("data-pane-type", "user-manual");
  expect(screen.getByTestId("pane-pane-a.tab.user-manual")).toHaveAttribute(
    "data-stable-id",
    "hs-usermanual-diagnostics-tab",
  );
  expect(document.querySelectorAll('[data-stable-id="hs-usermanual-diagnostics-tab"]')).toHaveLength(1);
  expect(await screen.findByText("Manual TOC")).toBeInTheDocument();
  expect(listUserManualAccessPoints).toHaveBeenCalledTimes(1);
  expect(listUserManualPages).toHaveBeenCalledTimes(1);
  expect(getUserManualPage).toHaveBeenCalledWith("manual-toc");
});

it("puts the diagnostics stable selector on the UserManual tab entry point", () => {
  render(<App />);

  const tab = screen.getByTestId("pane-pane-a.tab.user-manual");

  expect(tab.tagName).toBe("BUTTON");
  expect(tab).toHaveAttribute("data-stable-id", "hs-usermanual-diagnostics-tab");
});

it("keeps the diagnostics stable selector canonical when UserManual opens in another pane", async () => {
  render(<App />);

  fireEvent.click(screen.getByTestId("pane-pane-b.tab.problems"));
  fireEvent.click(screen.getByTestId("app-command-palette.open"));
  fireEvent.click(await screen.findByTestId("command-palette-action-hs-usermanual-palette-open"));

  expect(screen.getByTestId("pane-pane-b")).toHaveAttribute("data-pane-type", "user-manual");
  expect(screen.getByTestId("pane-pane-a.tab.user-manual")).toHaveAttribute(
    "data-stable-id",
    "hs-usermanual-diagnostics-tab",
  );
  expect(screen.getByTestId("pane-pane-b.tab.user-manual")).toHaveAttribute(
    "data-stable-id",
    "pane-pane-b.tab.user-manual",
  );
  expect(document.querySelectorAll('[data-stable-id="hs-usermanual-diagnostics-tab"]')).toHaveLength(1);
});

it("runs the UserManual search command against the backend client using the bottom search text", async () => {
  vi.mocked(searchUserManual).mockClear();

  render(<App />);

  fireEvent.change(screen.getByTestId("search-input"), { target: { value: "recovery" } });
  fireEvent.click(screen.getByTestId("app-command-palette.open"));
  fireEvent.click(await screen.findByTestId("command-palette-action-hs-usermanual-palette-search"));

  expect(await screen.findByTestId("usermanual-search-results")).toHaveTextContent("Manual recovery");
  await waitFor(() => expect(searchUserManual).toHaveBeenCalledWith("recovery", 25));
});

it("declares a narrow viewport layout override that stacks the pane grid for UserManual readability", () => {
  const css = readFileSync("src/App.css", "utf8");

  expect(css).toMatch(
    /@media \(max-width: 700px\) \{[\s\S]*?\.main-pane-grid \{[\s\S]*?grid-template-columns: 1fr;[\s\S]*?grid-template-rows: none;/,
  );
  expect(css).toMatch(
    /@media \(max-width: 700px\) \{[\s\S]*?\.main-pane\[data-pane-id="pane-a"\],[\s\S]*?\.main-pane\[data-pane-id="pane-b"\],[\s\S]*?\.main-pane\[data-pane-id="pane-c"\],[\s\S]*?\.main-pane\[data-pane-id="pane-d"\] \{[\s\S]*?grid-column: 1;[\s\S]*?grid-row: auto;/,
  );
  expect(css).toMatch(
    /@media \(max-width: 700px\) \{[\s\S]*?\.main-divider \{[\s\S]*?display: none;/,
  );
  expect(css).toMatch(
    /@media \(max-width: 700px\) \{[\s\S]*?\.usermanual-search \.filter-actions \{[\s\S]*?flex-direction: column;/,
  );
});

describe("FlightRecorderView deep links", () => {
  let scrollIntoViewMock: ReturnType<typeof vi.fn>;

  const makeEvent = (eventId: string): FlightEvent => ({
    event_id: eventId,
    trace_id: "trace-1",
    timestamp: "2026-01-01T00:00:00Z",
    actor: "system",
    actor_id: "system",
    event_type: "system",
    wsids: [],
    payload: {},
  });

  beforeEach(() => {
    window.history.pushState({}, "", "/");
    scrollIntoViewMock = vi.fn();
    Object.defineProperty(HTMLElement.prototype, "scrollIntoView", {
      value: scrollIntoViewMock,
      writable: true,
      configurable: true,
    });

    vi.mocked(getEvents).mockClear();
    vi.mocked(createDiagnostic).mockClear();
  });

  it("focuses/selects event_id from URL by marking the row selected", async () => {
    const eventId = "evt-focus-1";
    window.history.pushState({}, "", `/?event_id=${encodeURIComponent(eventId)}`);

    vi.mocked(getEvents).mockResolvedValue([makeEvent(eventId)]);

    render(<FlightRecorderView />);

    const eventButton = await screen.findByRole("button", { name: eventId });
    const row = eventButton.closest("tr");
    expect(row).not.toBeNull();
    expect(row).toHaveClass("flight-recorder__row--selected");

    await waitFor(() => expect(scrollIntoViewMock).toHaveBeenCalled());
  });

  it("emits VAL-NAV-001 and shows a notice when event_id is not in returned results", async () => {
    const eventId = "evt-missing-1";
    window.history.pushState({}, "", `/?event_id=${encodeURIComponent(eventId)}`);

    vi.mocked(getEvents).mockResolvedValue([makeEvent("evt-other-1")]);

    render(<FlightRecorderView />);

    expect(
      await screen.findByText(/Event focus failed: event_id not present in returned events/i),
    ).toBeInTheDocument();

    await waitFor(() =>
      expect(vi.mocked(createDiagnostic)).toHaveBeenCalledWith(expect.objectContaining({ code: "VAL-NAV-001" })),
    );
  });
});
