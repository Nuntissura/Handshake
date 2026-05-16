import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { vi } from "vitest";
import { KernelDccProjectionView } from "./KernelDccProjectionView";
import type { KernelDccProjectionSurfaceV1 } from "../lib/api";

const surface: KernelDccProjectionSurfaceV1 = {
  schema_id: "hsk.kernel.dcc_mvp_runtime_surface@1",
  surface_id: "dcc-backend-surface-test",
  folded_stub_id: "WP-1-Dev-Command-Center-MVP-v1",
  panels: [
    {
      panel_id: "panel-work-selection-test",
      kind: "WorkSelection",
      projection_only: true,
      source_refs: ["kernel.action_catalog"],
      visible_state_fields: ["wp_id", "proposal_id"],
    },
  ],
  work_items: [
    {
      work_id: "work-backend-123",
      wp_id: "WP-KERNEL-002",
      mt_id: "MT-DCC-123",
      status: "BACKEND_READY",
      worktree_id: "wt-backend-123",
      session_ids: ["session-backend-123"],
      proposal_ids: ["proposal-backend-123"],
      evidence_ids: ["evidence-backend-123"],
      allowed_action_ids: ["kernel.crdt_workspace.propose_patch"],
    },
  ],
  worktrees: [
    {
      worktree_id: "wt-backend-123",
      path_ref: "worktree://backend-test",
      branch: "codex/backend-test",
      dirty: true,
      diff_ref: "evidence-backend-123",
      linked_work_ids: ["work-backend-123"],
    },
  ],
  sessions: [
    {
      session_id: "session-backend-123",
      role: "CODER",
      model_id: "gpt-test",
      backend: "codex",
      worktree_id: "wt-backend-123",
      wp_id: "WP-KERNEL-002",
      mt_id: "MT-DCC-123",
      state: "ACTIVE",
    },
  ],
  proposals: [
    {
      proposal_id: "proposal-backend-123",
      work_id: "work-backend-123",
      action_id: "kernel.crdt_workspace.propose_patch",
      status: "AwaitingApproval",
      evidence_ids: ["evidence-backend-123"],
      approval_preview_id: "approval-backend-123",
    },
  ],
  evidence: [
    {
      evidence_id: "evidence-backend-123",
      kind: "DiffPatch",
      evidence_ref: "diff://backend-test",
      work_id: "work-backend-123",
    },
  ],
  approval_previews: [
    {
      preview_id: "approval-backend-123",
      action_id: "kernel.crdt_workspace.propose_patch",
      scope_options: ["Once", "Job"],
      requires_same_turn_approval: true,
      denied_failure_code: "BACKEND_DENIED",
    },
  ],
  catalog_action_refs: ["kernel.crdt_workspace.propose_patch"],
  catalog_action_rows: [
    {
      action_id: "kernel.crdt_workspace.propose_patch",
      target_authority_class: "PrePromotionEvidenceOnly",
      input_schema_id: "hsk.kernel.crdt_workspace_propose_patch_input@1",
      result_schema_id: "hsk.kernel.crdt_workspace_propose_patch_result@1",
      role_eligibility: ["CODER", "KERNEL_BUILDER"],
      capability_requirements: ["crdt_workspace.write"],
      approval_posture: "RequiresPromotionGate",
      preview_behavior_summary:
        "Stage a CRDT workspace patch for promotion-gate review without authority mutation.",
      preview_panel_id: "dcc-crdt-workspace-preview",
    },
  ],
  write_box_queue_rows: [
    {
      row_id: "write-box-row-backend-123",
      write_box_id: "wb-backend-123",
      work_id: "work-backend-123",
      kind: "CrdtWorkspace",
      lifecycle_state: "ReadyForValidation",
      actor_id: "actor-backend",
      target_refs: ["authority://backend-test"],
      validation_state: "Pending",
      denial_receipt_refs: [],
      promotion_receipt_refs: ["receipt://backend-test"],
      event_ledger_event_refs: [],
      stale_state_vector: false,
      stable_element_id: "dcc.write_box_queue.row.wb-backend-123",
    },
    {
      row_id: "write-box-row-backend-promoted",
      write_box_id: "wb-backend-promoted",
      work_id: "work-backend-123",
      kind: "Promotion",
      lifecycle_state: "Promoted",
      actor_id: "actor-backend",
      target_refs: ["authority://backend-test/promoted"],
      validation_state: "Valid",
      denial_receipt_refs: [],
      promotion_receipt_refs: ["receipt://backend-test/promoted"],
      event_ledger_event_refs: [
        "eventledger://backend/promotion-requested",
        "eventledger://backend/promotion-accepted",
      ],
      stale_state_vector: true,
      stable_element_id: "dcc.write_box_queue.row.wb-backend-promoted",
    },
  ],
  direct_edit_denials: [
    {
      row_id: "direct-edit-denial-row-backend-123",
      denial_id: "denial-backend-123",
      work_id: "work-backend-123",
      actor_id: "actor-backend",
      target_ref: "authority://backend-test",
      attempted_action: "raw_authority_file_write",
      recovery_instruction: "Use a registered write-box action",
      ui_response_ref: "dcc://denials/backend-123",
      api_response_ref: "api://denials/backend-123",
      stable_element_id: "dcc.direct_edit_denial.row.backend-123",
    },
  ],
  promotion_previews: [
    {
      row_id: "promotion-preview-row-backend-123",
      preview_id: "promotion-backend-123",
      work_id: "work-backend-123",
      write_box_id: "wb-backend-123",
      promotion_target_ref: "authority://backend-test",
      request_event_ref: "eventledger://backend/requested",
      accepted_event_ref: "eventledger://backend/accepted",
      rejected_event_ref: null,
      state_vector: "sv-promotion-backend",
      validation_check_summaries: [
        "promotion_gate_input_alignment: PASS",
        "crdt_state_vector_match: PASS",
      ],
      idempotency_key: "promotion:bridge-backend-123:requested",
      expected_event_kinds: [
        "KernelCrdtPromotionRequestedV1",
        "KernelCrdtPromotionAcceptedV1",
      ],
      stale_risk: "DuplicateIdempotency",
      freshness_badge_id: "freshness-backend-123",
      stable_element_id: "dcc.promotion_preview.row.wb-backend-123",
    },
  ],
  freshness_badges: [
    {
      badge_id: "freshness-backend-123",
      source_projection_id: "dcc-backend-projection",
      source_ref: "eventledger://backend",
      state_vector: "sv-backend-123",
      updated_at_ref: "eventledger://backend/latest",
      stale: false,
      stable_element_id: "dcc.freshness_badge.backend-123",
    },
  ],
  stable_element_ids: [
    {
      element_id: "dcc.write_box_queue.row.wb-backend-123",
      surface_id: "dcc-backend-surface-test",
      element_kind: "write_box_queue_row",
      source_ref: "writebox://wb-backend-123",
    },
  ],
  direct_authority_mutation_allowed: false,
  ungoverned_tool_execution_allowed: false,
  destructive_git_ops_require_same_turn_approval: true,
  flight_recorder_event_types: ["dcc.work.selected"],
  product_authority_refs: ["kernel.action_catalog"],
  folded_source_refs: [".GOV/task_packets/stubs/WP-1-Dev-Command-Center-MVP-v1.contract.json"],
  spawn_tree_projection: {
    schema_id: "hsk.kernel.session_spawn_tree_dcc_projection@1",
    tree_id: "spawn-tree-backend-123",
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
        session_id: "session-root-123",
        parent_session_id: null,
        role_id: "orchestrator",
        depth: 0,
        child_count: 1,
        active_child_count: 1,
        spawn_mode: "SessionPersistent",
        runtime_state: "Active",
        cascade_cancel_available: true,
        announce_back_badges: ["announce-back-ready"],
      },
      {
        session_id: "session-child-123",
        parent_session_id: "session-root-123",
        role_id: "coder",
        depth: 1,
        child_count: 0,
        active_child_count: 0,
        spawn_mode: "OneShot",
        runtime_state: "Active",
        cascade_cancel_available: false,
        announce_back_badges: [],
      },
    ],
    max_depth: 1,
    cascade_cancel_session_ids: ["session-root-123"],
    announce_back_badge_count: 1,
    runtime_record_refs: [
      "runtime://session-spawn/session-root-123",
      "runtime://session-spawn/session-child-123",
    ],
    mutates_runtime_records: false,
  },
};

it("projects backend DCC panels, work state, evidence, approval previews, rows, and stable ids", () => {
  render(<KernelDccProjectionView surface={surface} />);

  expect(screen.getAllByText("kernel.crdt_workspace.propose_patch").length).toBeGreaterThan(0);
  expect(screen.getByText("panel-work-selection-test")).toBeInTheDocument();
  expect(screen.getAllByText("work-backend-123").length).toBeGreaterThan(0);
  expect(screen.getAllByText("wt-backend-123").length).toBeGreaterThan(0);
  expect(screen.getAllByText("session-backend-123").length).toBeGreaterThan(0);
  expect(screen.getAllByText("proposal-backend-123").length).toBeGreaterThan(0);
  expect(screen.getAllByText("evidence-backend-123").length).toBeGreaterThan(0);
  expect(screen.getAllByText("approval-backend-123").length).toBeGreaterThan(0);
  expect(screen.getAllByText("wb-backend-123").length).toBeGreaterThan(0);
  expect(screen.getByText("denial-backend-123")).toBeInTheDocument();
  expect(screen.getByText("promotion-backend-123")).toBeInTheDocument();
  expect(screen.getByText("sv-backend-123")).toBeInTheDocument();

  for (const stableId of [
    "dcc.write_box_queue.row.wb-backend-123",
    "dcc.direct_edit_denial.row.backend-123",
    "dcc.promotion_preview.row.wb-backend-123",
    "dcc.freshness_badge.backend-123",
  ]) {
    expect(document.querySelector(`[data-stable-id="${stableId}"]`)).not.toBeNull();
  }
});

it("keeps the app DCC surface projection-only", () => {
  render(<KernelDccProjectionView surface={surface} />);

  expect(screen.getByText("Projection only")).toBeInTheDocument();
  expect(screen.queryByRole("button", { name: /promote/i })).toBeNull();
  expect(screen.queryByRole("button", { name: /mutate/i })).toBeNull();
});

it("triggers governed catalog actions through the provided API path", async () => {
  const onTriggerCatalogAction = vi.fn(async () => ({
    schema_id: "hsk.kernel.dcc_governed_action_trigger_result@1" as const,
    work_id: "work-backend-123",
    action_id: "kernel.crdt_workspace.propose_patch",
    triggered: true,
    catalog_checked: true,
    preview_checked: true,
    gate_enforced: true,
    approval_preview_id: "approval-backend-123",
    authority_effect: "PrePromotionEvidenceOnly",
    approval_posture: "RequiresPromotionGate",
    expected_write_box_kinds: ["CRDTWorkspaceBox", "ProposalBox"],
    receipt_ref: "receipt://kernel-dcc/action-trigger/work-backend-123/kernel.crdt_workspace.propose_patch",
  }));

  render(<KernelDccProjectionView surface={surface} onTriggerCatalogAction={onTriggerCatalogAction} />);

  fireEvent.click(screen.getByRole("button", { name: /^trigger$/i }));

  await waitFor(() =>
    expect(onTriggerCatalogAction).toHaveBeenCalledWith({
      work_id: "work-backend-123",
      action_id: "kernel.crdt_workspace.propose_patch",
      approval_preview_id: "approval-backend-123",
      same_turn_approval: true,
    }),
  );
  expect(await screen.findByText(/Governed trigger accepted/i)).toBeInTheDocument();
  expect(screen.getByText(/gate enforced:\s*yes/i)).toBeInTheDocument();
});

it("does not fallback to the first work item for unallowed catalog actions", async () => {
  const onTriggerCatalogAction = vi.fn();
  const disallowedSurface: KernelDccProjectionSurfaceV1 = {
    ...surface,
    catalog_action_refs: ["kernel.unallowed.catalog_action"],
    catalog_action_rows: [
      {
        action_id: "kernel.unallowed.catalog_action",
        target_authority_class: "ProjectionOnly",
        input_schema_id: "hsk.kernel.unallowed_input@1",
        result_schema_id: "hsk.kernel.unallowed_result@1",
        role_eligibility: ["OPERATOR"],
        capability_requirements: ["operator.unallowed"],
        approval_posture: "Denied",
        preview_behavior_summary: "Denied action — should not be triggered by ordinary work items.",
        preview_panel_id: "dcc-unallowed-preview",
      },
    ],
    work_items: [
      {
        ...surface.work_items[0],
        allowed_action_ids: ["kernel.crdt_workspace.propose_patch"],
      },
    ],
    approval_previews: [],
  };

  render(<KernelDccProjectionView surface={disallowedSurface} onTriggerCatalogAction={onTriggerCatalogAction} />);

  fireEvent.click(screen.getByRole("button", { name: /^trigger$/i }));

  expect(onTriggerCatalogAction).not.toHaveBeenCalled();
  expect(await screen.findByRole("alert")).toHaveTextContent(
    "No selected DCC work item can trigger this catalog action",
  );
});

it("renders the action catalog viewer with authority class, approval posture, schemas, and preview metadata", () => {
  render(<KernelDccProjectionView surface={surface} />);

  expect(
    screen.getByTestId("dcc.action_catalog.row.kernel.crdt_workspace.propose_patch.target_authority_class"),
  ).toHaveTextContent("PrePromotionEvidenceOnly");
  expect(
    screen.getByTestId("dcc.action_catalog.row.kernel.crdt_workspace.propose_patch.approval_posture"),
  ).toHaveTextContent("RequiresPromotionGate");
  expect(
    screen.getByTestId("dcc.action_catalog.row.kernel.crdt_workspace.propose_patch.role_eligibility"),
  ).toHaveTextContent("CODER, KERNEL_BUILDER");
  expect(
    screen.getByTestId("dcc.action_catalog.row.kernel.crdt_workspace.propose_patch.capability_requirements"),
  ).toHaveTextContent("crdt_workspace.write");
  expect(
    screen.getByTestId("dcc.action_catalog.row.kernel.crdt_workspace.propose_patch.input_schema_id"),
  ).toHaveTextContent("hsk.kernel.crdt_workspace_propose_patch_input@1");
  expect(
    screen.getByTestId("dcc.action_catalog.row.kernel.crdt_workspace.propose_patch.result_schema_id"),
  ).toHaveTextContent("hsk.kernel.crdt_workspace_propose_patch_result@1");
  expect(
    screen.getByTestId("dcc.action_catalog.row.kernel.crdt_workspace.propose_patch.preview_panel_id"),
  ).toHaveTextContent("dcc-crdt-workspace-preview");
  expect(
    screen.getByTestId("dcc.action_catalog.row.kernel.crdt_workspace.propose_patch.preview_behavior_summary"),
  ).toHaveTextContent(/Stage a CRDT workspace patch/);
});

it("renders write box queue rows with EventLedger events and a stale state-vector badge for Promoted boxes", () => {
  render(<KernelDccProjectionView surface={surface} />);

  const eventLedgerCell = screen.getByTestId(
    "dcc.write_box_queue.row.wb-backend-promoted.event_ledger_event_refs",
  );
  expect(eventLedgerCell).toHaveTextContent("2");
  expect(eventLedgerCell).toHaveAttribute(
    "title",
    expect.stringContaining("eventledger://backend/promotion-accepted"),
  );

  const staleCell = screen.getByTestId(
    "dcc.write_box_queue.row.wb-backend-promoted.stale_state_vector",
  );
  expect(staleCell).toHaveTextContent("stale");
  expect(staleCell.className).toContain("stale");

  const freshCell = screen.getByTestId(
    "dcc.write_box_queue.row.wb-backend-123.stale_state_vector",
  );
  expect(freshCell).toHaveTextContent("fresh");
  expect(freshCell.className).not.toContain("stale");
});

it("renders promotion preview rows with state vector, idempotency key, expected events, and stale-risk class", () => {
  render(<KernelDccProjectionView surface={surface} />);

  expect(
    screen.getByTestId("dcc.promotion_preview.row.wb-backend-123.state_vector"),
  ).toHaveTextContent("sv-promotion-backend");
  expect(
    screen.getByTestId("dcc.promotion_preview.row.wb-backend-123.idempotency_key"),
  ).toHaveTextContent("promotion:bridge-backend-123:requested");
  expect(
    screen.getByTestId(
      "dcc.promotion_preview.row.wb-backend-123.validation_check_summaries",
    ),
  ).toHaveTextContent(/promotion_gate_input_alignment: PASS/);
  expect(
    screen.getByTestId("dcc.promotion_preview.row.wb-backend-123.expected_event_kinds"),
  ).toHaveTextContent("KernelCrdtPromotionRequestedV1, KernelCrdtPromotionAcceptedV1");
  expect(
    screen.getByTestId("dcc.promotion_preview.row.wb-backend-123.accepted_event_ref"),
  ).toHaveTextContent("eventledger://backend/accepted");
  expect(
    screen.getByTestId("dcc.promotion_preview.row.wb-backend-123.rejected_event_ref"),
  ).toHaveTextContent("pending");
  const staleRisk = screen.getByTestId(
    "dcc.promotion_preview.row.wb-backend-123.stale_risk",
  );
  expect(staleRisk).toHaveTextContent("DuplicateIdempotency");
  expect(staleRisk.className).toContain("stale");
});

it("renders session spawn hierarchy fields projected from runtime records", () => {
  render(<KernelDccProjectionView surface={surface} />);

  expect(screen.getByText("Session Spawn Tree")).toBeInTheDocument();
  expect(screen.getAllByText("session-root-123").length).toBeGreaterThan(0);
  expect(screen.getByText("session-child-123")).toBeInTheDocument();
  expect(screen.getByText("SessionPersistent")).toBeInTheDocument();
  expect(screen.getByText("OneShot")).toBeInTheDocument();
  expect(screen.getByText("announce-back-ready")).toBeInTheDocument();
  expect(screen.getByText(/max depth 1/i)).toBeInTheDocument();
  expect(screen.getByText(/Cascade cancel sessions: session-root-123/i)).toBeInTheDocument();
  expect(document.querySelector('[data-stable-id="dcc.session_spawn_tree.node.session-child-123"]')).not.toBeNull();
});
