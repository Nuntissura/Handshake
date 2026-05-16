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
import { createDiagnostic, getEvents, getKernelDccProjection, type FlightEvent } from "./lib/api";

it("renders desktop shell header and shows coordinator status", () => {
  render(<App />);

  expect(screen.getByText(/Desktop Shell/i)).toBeInTheDocument();
  expect(screen.getByTestId("system-status")).toBeInTheDocument();
});

it("loads the backend Kernel DCC projection when opened", async () => {
  vi.mocked(getKernelDccProjection).mockClear();

  render(<App />);

  fireEvent.click(screen.getByRole("button", { name: /Kernel DCC/i }));

  expect((await screen.findAllByText("work-app-backend-123")).length).toBeGreaterThan(0);
  expect(screen.getByText("panel-app-backend-test")).toBeInTheDocument();
  expect(screen.getAllByText("wb-app-backend").length).toBeGreaterThan(0);
  expect(screen.getByText("Session Spawn Tree")).toBeInTheDocument();
  expect(screen.getAllByText("session-app-backend").length).toBeGreaterThan(0);
  expect(document.querySelector('[data-stable-id="dcc.session_spawn_tree.node.session-app-backend"]')).not.toBeNull();
  await waitFor(() => expect(vi.mocked(getKernelDccProjection)).toHaveBeenCalledTimes(1));
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
