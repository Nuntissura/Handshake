import { readFileSync } from "node:fs";
import { beforeEach, vi } from "vitest";

vi.mock("@excalidraw/excalidraw", () => ({
  __esModule: true,
  Excalidraw: () => null,
}));

vi.mock("./components/WorkspaceSidebar", () => ({
  WorkspaceSidebar: ({
    activeWorkspaceId,
    onSelectWorkspace,
    selectedDocumentId,
    selectedCanvasId,
    onSelectDocument,
    onSelectCanvas,
    onOpenLoomBlock,
  }: {
    activeWorkspaceId?: string | null;
    onSelectWorkspace?: (id: string) => void;
    selectedDocumentId: string | null;
    selectedCanvasId: string | null;
    onSelectDocument: (id: string | null) => void;
    onSelectCanvas: (id: string | null) => void;
    onOpenLoomBlock?: (blockId: string) => void;
  }) => (
    <div
      data-testid="workspace-sidebar"
      data-active-workspace-id={activeWorkspaceId ?? ""}
      data-selected-document-id={selectedDocumentId ?? ""}
      data-selected-canvas-id={selectedCanvasId ?? ""}
    >
      <button
        type="button"
        data-testid="workspace-sidebar.select-project-aux"
        onClick={() => onSelectWorkspace?.("project-aux")}
      >
        Project Aux
      </button>
      <button type="button" data-testid="workspace-sidebar.open-doc-alpha" onClick={() => onSelectDocument("doc-alpha")}>
        Doc Alpha
      </button>
      <button type="button" data-testid="workspace-sidebar.open-doc-beta" onClick={() => onSelectDocument("doc-beta")}>
        Doc Beta
      </button>
      <button type="button" data-testid="workspace-sidebar.open-canvas-alpha" onClick={() => onSelectCanvas("canvas-alpha")}>
        Canvas Alpha
      </button>
      <button
        type="button"
        data-testid="workspace-sidebar.open-bookmark-block-alpha"
        onClick={() => onOpenLoomBlock?.("block-alpha")}
      >
        Bookmark Alpha
      </button>
    </div>
  ),
}));

vi.mock("./components/DocumentView", () => ({
  DocumentView: ({
    documentId,
    onDeleted,
    onDirtyChange,
    commandPaletteRequest,
    findRequest,
  }: {
    documentId: string;
    onDeleted: () => void;
    onDirtyChange?: (dirty: boolean) => void;
    commandPaletteRequest?: { paneId: string; requestId: number; query: string } | null;
    findRequest?: {
      requestId: number;
      query: string;
      caseSensitive: boolean;
      wholeWord: boolean;
      isRegex: boolean;
    } | null;
  }) => (
    <div data-testid={`document-view.${documentId}`} data-document-id={documentId}>
      <button type="button" data-testid={`document-view.${documentId}.mark-dirty`} onClick={() => onDirtyChange?.(true)}>
        Mark dirty
      </button>
      <button type="button" data-testid={`document-view.${documentId}.mark-clean`} onClick={() => onDirtyChange?.(false)}>
        Mark clean
      </button>
      <button type="button" data-testid={`document-view.${documentId}.delete`} onClick={onDeleted}>
        Delete
      </button>
      {commandPaletteRequest ? (
        <div
          data-testid={`document-view.${documentId}.command-palette-request`}
          data-pane-id={commandPaletteRequest.paneId}
          data-request-id={String(commandPaletteRequest.requestId)}
          data-query={commandPaletteRequest.query}
        >
          Editor command palette requested
        </div>
      ) : null}
      {findRequest ? (
        <div
          data-testid={`document-view.${documentId}.find-request`}
          data-request-id={String(findRequest.requestId)}
          data-query={findRequest.query}
          data-case-sensitive={String(findRequest.caseSensitive)}
          data-whole-word={String(findRequest.wholeWord)}
          data-regex={String(findRequest.isRegex)}
        >
          Editor find requested
        </div>
      ) : null}
      Document {documentId}
    </div>
  ),
}));

vi.mock("./components/CanvasView", () => ({
  CanvasView: ({ canvasId, onDeleted }: { canvasId: string; onDeleted: () => void }) => (
    <div data-testid={`canvas-view.${canvasId}`} data-canvas-id={canvasId}>
      <button type="button" data-testid={`canvas-view.${canvasId}.delete`} onClick={onDeleted}>
        Delete
      </button>
      Canvas {canvasId}
    </div>
  ),
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
  getCodeSymbol: vi.fn(async () => ({
    symbol: {
      symbol_entity_id: "KEN-symbol-alpha",
      symbol_key: "rust:src/backend/graph_search.rs#GraphSearchAlpha",
      display_name: "GraphSearchAlpha",
      symbol_kind: "function",
      owning_wp: "WP-KERNEL-009",
      primary_source_id: "src-backend-graph-search-rs",
      lifecycle_state: "active",
      definition: {
        span_id: "span-alpha",
        source_id: "src-backend-graph-search-rs",
        line_start: 12,
        line_end: 34,
        range_start: 400,
        range_end: 900,
        section_path: "GraphSearchAlpha",
      },
      staleness: { status: "fresh" },
    },
    nav_receipt_event_id: "EVT-symbol-get",
    quiet_background_work_receipt_id: "quiet-symbol-get",
  })),
  getLoomWikiProjection: vi.fn(async () => ({
    projection_id: "KWP-alpha",
    workspace_id: "w1",
    title: "GraphSearchAlpha Wiki Page",
    source_block_ids: ["block-alpha"],
    rendered_content: "GraphSearchAlpha wiki rendered content.",
    staleness_hash: "hash-alpha",
    rebuild_status: "fresh",
    page_type: "concept",
    compile_stamp: null,
    page_links: [],
    created_at: "2026-06-15T00:00:00Z",
    updated_at: "2026-06-15T00:00:00Z",
    staleness_verdict: { status: "fresh" },
  })),
  getLoomBlock: vi.fn(async (_workspaceId: string, blockId: string) => {
    if (blockId === "file-alpha") {
      return {
        block_id: "file-alpha",
        workspace_id: "w1",
        content_type: "file",
        document_id: null,
        asset_id: null,
        title: "GraphSearchAlpha source file",
        original_filename: "graph-search-alpha.md",
        content_hash: "hash-file-alpha",
        pinned: false,
        favorite: false,
        pin_order: null,
        journal_date: null,
        created_at: "2026-06-15T00:00:00Z",
        updated_at: "2026-06-15T00:00:00Z",
        imported_at: null,
        derived: {
          full_text_index: "GraphSearchAlpha appears in a source file block.",
          backlink_count: 0,
          mention_count: 1,
          tag_count: 0,
          preview_status: "none",
        },
      };
    }
    if (blockId === "tag-alpha") {
      return {
        block_id: "tag-alpha",
        workspace_id: "w1",
        content_type: "tag_hub",
        document_id: null,
        asset_id: null,
        title: "GraphSearchAlpha tag hub",
        original_filename: null,
        content_hash: "hash-tag-alpha",
        pinned: false,
        favorite: false,
        pin_order: null,
        journal_date: null,
        created_at: "2026-06-15T00:00:00Z",
        updated_at: "2026-06-15T00:00:00Z",
        imported_at: null,
        derived: {
          full_text_index: "GraphSearchAlpha appears in a tag hub block.",
          backlink_count: 1,
          mention_count: 0,
          tag_count: 2,
          preview_status: "none",
        },
      };
    }
    return {
      block_id: "block-alpha",
      workspace_id: "w1",
      content_type: "note",
      document_id: null,
      asset_id: null,
      title: "GraphSearchAlpha standalone Loom note",
      original_filename: null,
      content_hash: "hash-alpha",
      pinned: false,
      favorite: true,
      pin_order: null,
      journal_date: null,
      created_at: "2026-06-15T00:00:00Z",
      updated_at: "2026-06-15T00:00:00Z",
      imported_at: null,
      derived: {
        full_text_index: "GraphSearchAlpha joins notes to code and manuals.",
        backlink_count: 1,
        mention_count: 2,
        tag_count: 3,
        preview_status: "none",
      },
    };
  }),
  openDailyJournal: vi.fn(async (workspaceId: string, journalDate: string) => ({
    block_id: `journal-${journalDate}`,
    workspace_id: workspaceId,
    content_type: "journal",
    document_id: null,
    asset_id: null,
    title: `Daily Note ${journalDate}`,
    original_filename: null,
    content_hash: null,
    pinned: false,
    favorite: false,
    pin_order: null,
    journal_date: journalDate,
    created_at: `${journalDate}T00:00:00Z`,
    updated_at: `${journalDate}T00:00:00Z`,
    imported_at: null,
    derived: {
      full_text_index: `# Daily Note ${journalDate}\n\n`,
      backlink_count: 0,
      mention_count: 0,
      tag_count: 0,
      preview_status: "none",
    },
  })),
  getWorkbenchLayoutState: vi.fn(async (workspaceId: string) => ({
    workspace_id: workspaceId,
    layout_state: null,
    updated_at: null,
    event_ledger_event_id: null,
  })),
  saveWorkbenchLayoutState: vi.fn(async (workspaceId: string, layoutState: Record<string, unknown>) => ({
    workspace_id: workspaceId,
    layout_state: layoutState,
    updated_at: "2026-06-15T00:00:00Z",
    event_ledger_event_id: "EVT-workbench-layout",
  })),
  getWorkspaceSettingsState: vi.fn(async (workspaceId: string) => ({
    workspace_id: workspaceId,
    settings_state: null,
    updated_at: null,
    event_ledger_event_id: null,
  })),
  saveWorkspaceSettingsState: vi.fn(async (workspaceId: string, settingsState: Record<string, unknown>) => ({
    workspace_id: workspaceId,
    settings_state: settingsState,
    updated_at: "2026-06-15T00:00:00Z",
    event_ledger_event_id: "EVT-workspace-settings",
  })),
  listQuickSwitcherRecents: vi.fn(async () => []),
  recordQuickSwitcherRecent: vi.fn(async (workspaceId: string, hit: {
    source_kind: string;
    ref_id: string;
    result_kind: string;
    title: string;
    excerpt?: string;
    metadata?: unknown;
  }) => ({
    workspace_id: workspaceId,
    hit_key: `${hit.source_kind}:${hit.ref_id}`,
    source_kind: hit.source_kind,
    ref_id: hit.ref_id,
    result_kind: hit.result_kind,
    title: hit.title,
    excerpt: hit.excerpt ?? "",
    metadata: hit.metadata ?? {},
    selected_count: 1,
    selected_at: "2026-06-15T00:00:00Z",
    event_ledger_event_id: "EVT-quick-switcher-recent",
  })),
  searchLoomGraph: vi.fn(async () => [
    {
      result_kind: "loom_block",
      source_kind: "loom_block",
      ref_id: "block-alpha",
      title: "GraphSearchAlpha Loom note",
      excerpt: "GraphSearchAlpha joins notes to code and manuals.",
      score: 4.2,
      metadata: { content_type: "note" },
    },
    {
      result_kind: "loom_block",
      source_kind: "file",
      ref_id: "file-alpha",
      title: "GraphSearchAlpha source file",
      excerpt: "Open the indexed file block for GraphSearchAlpha.",
      score: 4.1,
      metadata: { content_type: "file" },
    },
    {
      result_kind: "loom_block",
      source_kind: "tag_hub",
      ref_id: "tag-alpha",
      title: "GraphSearchAlpha tag hub",
      excerpt: "Open the tag hub block for GraphSearchAlpha.",
      score: 4.0,
      metadata: { content_type: "tag_hub" },
    },
    {
      result_kind: "knowledge_entity",
      source_kind: "document",
      ref_id: "KRD-00000000000000000000000000000001",
      title: "GraphSearchAlpha standalone document",
      excerpt: "Open the standalone RichDocument for GraphSearchAlpha.",
      score: 3.9,
      metadata: { rich_document_id: "KRD-00000000000000000000000000000001" },
    },
    {
      result_kind: "knowledge_entity",
      source_kind: "symbol",
      ref_id: "rust:src/backend/graph_search.rs#GraphSearchAlpha",
      title: "GraphSearchAlpha",
      excerpt: "Symbol from the project knowledge index.",
      score: 3.6,
      metadata: {},
    },
    {
      result_kind: "knowledge_entity",
      source_kind: "work_packet",
      ref_id: "WP-KERNEL-009-GraphSearchAlpha",
      title: "GraphSearchAlpha work packet",
      excerpt: "Work packet hit.",
      score: 3.2,
      metadata: {},
    },
    {
      result_kind: "knowledge_entity",
      source_kind: "micro_task",
      ref_id: "MT-186-GraphSearchAlpha",
      title: "GraphSearchAlpha microtask",
      excerpt: "Microtask hit.",
      score: 3.1,
      metadata: {},
    },
    {
      result_kind: "wiki_page",
      source_kind: "wiki_page",
      ref_id: "KWP-alpha",
      title: "GraphSearchAlpha Wiki Page",
      excerpt: "Open the compiled project wiki projection for GraphSearchAlpha.",
      score: 2.9,
      metadata: { projection_id: "KWP-alpha", page_type: "concept" },
    },
    {
      result_kind: "user_manual_page",
      source_kind: "user_manual_page",
      ref_id: "graph-search-alpha",
      title: "GraphSearchAlpha UserManual page",
      excerpt: "Open the UserManual page for GraphSearchAlpha.",
      score: 2.8,
      metadata: { page_slug: "graph-search-alpha" },
    },
  ]),
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
  getSourceControlStatus: vi.fn(async () => ({
    repo_root: "D:\\Projects\\Handshake Repo",
    branch: "main",
    entries: [],
  })),
  getSourceControlDiff: vi.fn(async () => ({
    path: "src/main.rs",
    scope: "worktree",
    patch: "",
  })),
  stageSourceControlPaths: vi.fn(async () => ({ operation: "stage", paths: [] })),
  unstageSourceControlPaths: vi.fn(async () => ({ operation: "unstage", paths: [] })),
  discardSourceControlPaths: vi.fn(async () => ({ operation: "discard", paths: [] })),
  commitSourceControl: vi.fn(async () => ({ id: "a".repeat(40), message: "commit" })),
  listSourceControlBranches: vi.fn(async () => []),
  createSourceControlBranch: vi.fn(async () => ({ operation: "create_branch", paths: ["main"] })),
  switchSourceControlBranch: vi.fn(async () => ({ operation: "switch_branch", paths: ["main"] })),
  getSourceControlLog: vi.fn(async () => ({ entries: [] })),
  getSourceControlBlame: vi.fn(async () => ({ path: "src/main.rs", lines: [] })),
  loadRichDocumentHistory: vi.fn(async () => ({
    rich_document_id: "KRD-00000000000000000000000000000001",
    current_version: 1,
    authority_label: "PostgreSQL/EventLedger",
    owner_actor_kind: null,
    owner_actor_id: null,
    versions: [],
  })),
  createDiagnostic: vi.fn(async () => ({})),
}));

import { act, fireEvent, render, screen, waitFor } from "@testing-library/react";
import App from "./App";
import { FlightRecorderView } from "./components/FlightRecorderView";
import { HS_LINK_NAVIGATE_EVENT } from "./lib/editor/link_navigation";
import {
  createDiagnostic,
  getCodeSymbol,
  getEvents,
  getKernelDccProjection,
  getLoomBlock,
  getLoomWikiProjection,
  openDailyJournal,
  getWorkbenchLayoutState,
  getWorkspaceSettingsState,
  getUserManualPage,
  listQuickSwitcherRecents,
  listUserManualAccessPoints,
  listUserManualPages,
  listWorkspaces,
  recordQuickSwitcherRecent,
  saveWorkbenchLayoutState,
  saveWorkspaceSettingsState,
  searchLoomGraph,
  searchUserManual,
  type FlightEvent,
  type QuickSwitcherRecent,
  type RecordQuickSwitcherRecentRequest,
} from "./lib/api";

function defaultQuickSwitcherRecent(
  workspaceId: string,
  hit: RecordQuickSwitcherRecentRequest,
): QuickSwitcherRecent {
  return {
    workspace_id: workspaceId,
    hit_key: `${hit.source_kind}:${hit.ref_id}`,
    source_kind: hit.source_kind,
    ref_id: hit.ref_id,
    result_kind: hit.result_kind,
    title: hit.title,
    excerpt: hit.excerpt ?? "",
    metadata: hit.metadata ?? {},
    selected_count: 1,
    selected_at: "2026-06-15T00:00:00Z",
    event_ledger_event_id: "EVT-quick-switcher-recent",
  };
}

function deferred<T>() {
  let resolve!: (value: T) => void;
  const promise = new Promise<T>((promiseResolve) => {
    resolve = promiseResolve;
  });
  return { promise, resolve };
}

function workbenchLayoutResponse(
  workspaceId: string,
  layoutState: Record<string, unknown>,
  eventId: string,
) {
  return {
    workspace_id: workspaceId,
    layout_state: layoutState,
    updated_at: "2026-06-15T00:00:00Z",
    event_ledger_event_id: eventId,
  };
}

beforeEach(() => {
  vi.mocked(listWorkspaces).mockReset();
  vi.mocked(listWorkspaces).mockResolvedValue([
    {
      id: "w1",
      name: "Workspace 1",
      created_at: "2025-01-01T00:00:00Z",
      updated_at: "2025-01-01T00:00:00Z",
    },
  ]);
  vi.mocked(getWorkbenchLayoutState).mockReset();
  vi.mocked(getWorkbenchLayoutState).mockImplementation(async (workspaceId) => ({
    workspace_id: workspaceId,
    layout_state: null,
    updated_at: null,
    event_ledger_event_id: null,
  }));
  vi.mocked(saveWorkbenchLayoutState).mockReset();
  vi.mocked(saveWorkbenchLayoutState).mockImplementation(async (workspaceId, layoutState) => ({
    workspace_id: workspaceId,
    layout_state: layoutState,
    updated_at: "2026-06-15T00:00:00Z",
    event_ledger_event_id: "EVT-workbench-layout",
  }));
  vi.mocked(getWorkspaceSettingsState).mockReset();
  vi.mocked(getWorkspaceSettingsState).mockImplementation(async (workspaceId) => ({
    workspace_id: workspaceId,
    settings_state: null,
    updated_at: null,
    event_ledger_event_id: null,
  }));
  vi.mocked(saveWorkspaceSettingsState).mockReset();
  vi.mocked(saveWorkspaceSettingsState).mockImplementation(async (workspaceId, settingsState) => ({
    workspace_id: workspaceId,
    settings_state: settingsState,
    updated_at: "2026-06-15T00:00:00Z",
    event_ledger_event_id: "EVT-workspace-settings",
  }));
  vi.mocked(listQuickSwitcherRecents).mockReset();
  vi.mocked(listQuickSwitcherRecents).mockResolvedValue([]);
  vi.mocked(recordQuickSwitcherRecent).mockReset();
  vi.mocked(recordQuickSwitcherRecent).mockImplementation(async (workspaceId, hit) =>
    defaultQuickSwitcherRecent(workspaceId, hit),
  );
  vi.mocked(openDailyJournal).mockReset();
  vi.mocked(openDailyJournal).mockImplementation(async (workspaceId: string, journalDate: string) => ({
    block_id: `journal-${journalDate}`,
    workspace_id: workspaceId,
    content_type: "journal",
    document_id: null,
    asset_id: null,
    title: `Daily Note ${journalDate}`,
    original_filename: null,
    content_hash: null,
    pinned: false,
    favorite: false,
    pin_order: null,
    journal_date: journalDate,
    created_at: `${journalDate}T00:00:00Z`,
    updated_at: `${journalDate}T00:00:00Z`,
    imported_at: null,
    derived: {
      full_text_index: `# Daily Note ${journalDate}\n\n`,
      backlink_count: 0,
      mention_count: 0,
      tag_count: 0,
      preview_status: "none",
    },
  }));
});

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

it("loads durable workspace settings and applies the selected theme to the shell", async () => {
  vi.mocked(getWorkspaceSettingsState).mockResolvedValue({
    workspace_id: "w1",
    settings_state: {
      schema_id: "hsk.workspace_settings_state@1",
      theme: "dark",
      custom_theme_tokens: {},
      keybindings: {
        "app.quick_switcher.open": "Alt-q",
        "app.command_palette.open": "Mod-Shift-p",
      },
      settings: {
        view_mode: "SFW",
        swarm_board_default_open: true,
      },
    },
    updated_at: "2026-06-15T00:00:00Z",
    event_ledger_event_id: "EVT-workspace-settings-load",
  });

  render(<App />);

  await waitFor(() => expect(getWorkspaceSettingsState).toHaveBeenCalledWith("w1"));
  expect(screen.getByTestId("main-window")).toHaveAttribute("data-theme", "dark");
});

it("saves theme changes through the durable workspace settings endpoint", async () => {
  render(<App />);

  await waitFor(() => expect(getWorkspaceSettingsState).toHaveBeenCalledWith("w1"));
  fireEvent.click(screen.getByTestId("settings-gear"));
  fireEvent.change(await screen.findByTestId("setting-theme.control"), { target: { value: "dark" } });

  await waitFor(() =>
    expect(saveWorkspaceSettingsState).toHaveBeenCalledWith(
      "w1",
      expect.objectContaining({
        schema_id: "hsk.workspace_settings_state@1",
        theme: "dark",
      }),
    ),
  );
  expect(screen.getByTestId("main-window")).toHaveAttribute("data-theme", "dark");
});

it("uses the durable app keybinding override for the quick switcher", async () => {
  vi.mocked(getWorkspaceSettingsState).mockResolvedValue({
    workspace_id: "w1",
    settings_state: {
      schema_id: "hsk.workspace_settings_state@1",
      theme: "light",
      custom_theme_tokens: {},
      keybindings: {
        "app.quick_switcher.open": "Alt-q",
        "app.command_palette.open": "Mod-Shift-p",
      },
      settings: {
        view_mode: "NSFW",
        swarm_board_default_open: false,
      },
    },
    updated_at: "2026-06-15T00:00:00Z",
    event_ledger_event_id: "EVT-workspace-settings-load",
  });

  render(<App />);

  await waitFor(() => expect(getWorkspaceSettingsState).toHaveBeenCalledWith("w1"));
  fireEvent.keyDown(window, { key: "p", ctrlKey: true });
  expect(screen.queryByTestId("quick-switcher.search")).not.toBeInTheDocument();

  fireEvent.keyDown(window, { key: "q", altKey: true });
  expect(await screen.findByTestId("quick-switcher.search")).toBeInTheDocument();
});

it("resolves startup with no workspaces without hydrating a default workspace layout", async () => {
  vi.mocked(listWorkspaces).mockResolvedValueOnce([]);

  render(<App />);

  const mainWindow = screen.getByTestId("main-window");
  await waitFor(() => expect(listWorkspaces).toHaveBeenCalled());
  await waitFor(() => expect(screen.getByTestId("workspace-sidebar")).toHaveAttribute("data-active-workspace-id", ""));
  expect(mainWindow).toHaveAttribute("data-active-project-id", "");
  expect(getWorkbenchLayoutState).not.toHaveBeenCalled();
  vi.mocked(listQuickSwitcherRecents).mockClear();
  vi.mocked(searchLoomGraph).mockClear();

  fireEvent.keyDown(window, { key: "p", ctrlKey: true });
  const quickSearch = await screen.findByTestId("quick-switcher.search");
  fireEvent.change(quickSearch, { target: { value: "alpha" } });
  await act(async () => {
    await new Promise((resolve) => window.setTimeout(resolve, 250));
  });

  expect(listQuickSwitcherRecents).not.toHaveBeenCalled();
  expect(searchLoomGraph).not.toHaveBeenCalled();
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
  expect(screen.getByTestId("workspace-sidebar")).toHaveAttribute("data-active-workspace-id", "project-aux");
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

it("exposes source control as a CKC pane tab", async () => {
  render(<App />);
  const mainWindow = screen.getByTestId("main-window");

  await waitFor(() => expect(getWorkbenchLayoutState).toHaveBeenCalledWith("w1"));
  await waitFor(() => expect(mainWindow).toHaveAttribute("data-active-module", "MAIN"));

  await act(async () => {
    fireEvent.click(screen.getByTestId("module-ckc"));
  });
  await waitFor(() => expect(mainWindow).toHaveAttribute("data-active-module", "CKC"));
  const sourceControlTab = await screen.findByTestId("pane-pane-a.tab.source-control");
  await act(async () => {
    fireEvent.click(sourceControlTab);
  });

  expect(screen.getByTestId("pane-pane-a")).toHaveAttribute("data-pane-type", "source-control");
  expect(screen.getByTestId("source-control-panel")).toBeInTheDocument();
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

it("restores pane tabs, drawers, and split weights from durable workbench layout state", async () => {
  vi.mocked(getWorkbenchLayoutState).mockResolvedValueOnce({
    workspace_id: "w1",
    layout_state: {
      schema_id: "hsk.workbench_layout_state@1",
      activePaneId: "pane-b",
      activeModule: "CKC",
      splitWeights: { vertical: 0.63, horizontal: 0.47 },
      drawers: { project: true, file: false, bottom: false },
      panes: [
        {
          id: "pane-a",
          module: "MAIN",
          activeTab: "workspace",
          tabs: ["workspace", "user-manual"],
          locked: false,
          projectRef: "w1",
        },
        {
          id: "pane-b",
          module: "CKC",
          activeTab: "kernel-dcc",
          tabs: ["kernel-dcc", "problems"],
          locked: false,
          projectRef: "w1",
        },
        {
          id: "pane-c",
          module: "INGEST",
          activeTab: "flight-recorder",
          tabs: ["flight-recorder", "media-downloader"],
          locked: false,
          projectRef: "w1",
        },
        {
          id: "pane-d",
          module: "STAGE",
          activeTab: "fonts",
          tabs: ["fonts"],
          locked: false,
          projectRef: "w1",
        },
      ],
    },
    updated_at: "2026-06-15T00:00:00Z",
    event_ledger_event_id: "EVT-workbench-layout",
  });

  render(<App />);

  const mainWindow = screen.getByTestId("main-window");
  await waitFor(() => expect(mainWindow).toHaveAttribute("data-split-weights", "0.630,0.470"));
  expect(mainWindow).toHaveAttribute("data-active-module", "CKC");
  expect(mainWindow).toHaveAttribute("data-file-drawer-open", "false");
  expect(mainWindow).toHaveAttribute("data-bottom-drawer-open", "false");
  expect(screen.getByTestId("pane-pane-b")).toHaveAttribute("data-pane-active-tab", "kernel-dcc");
  expect(screen.getByTestId("pane-pane-c")).toHaveAttribute("data-pane-active-tab", "flight-recorder");
});

it("restores pane-local workspace document tabs from durable workbench layout state", async () => {
  vi.mocked(getWorkbenchLayoutState).mockResolvedValueOnce({
    workspace_id: "w1",
    layout_state: {
      schema_id: "hsk.workbench_layout_state@1",
      activePaneId: "pane-a",
      activeModule: "MAIN",
      splitWeights: { vertical: 0.5, horizontal: 0.55 },
      drawers: { project: true, file: true, bottom: true },
      panes: [
        {
          id: "pane-a",
          module: "MAIN",
          activeTab: "workspace",
          tabs: ["workspace", "user-manual"],
          locked: false,
          projectRef: "w1",
          activeDocumentId: "doc-beta",
          openDocuments: [
            { documentId: "doc-alpha", pinned: false, dirty: false },
            { documentId: "doc-beta", pinned: true, dirty: true },
          ],
        },
        {
          id: "pane-b",
          module: "CKC",
          activeTab: "kernel-dcc",
          tabs: ["kernel-dcc", "problems"],
          locked: false,
          projectRef: "w1",
          activeDocumentId: null,
          openDocuments: [],
        },
        {
          id: "pane-c",
          module: "INGEST",
          activeTab: "flight-recorder",
          tabs: ["flight-recorder", "media-downloader"],
          locked: false,
          projectRef: "w1",
          activeDocumentId: null,
          openDocuments: [],
        },
        {
          id: "pane-d",
          module: "STAGE",
          activeTab: "fonts",
          tabs: ["fonts"],
          locked: false,
          projectRef: "w1",
          activeDocumentId: null,
          openDocuments: [],
        },
      ],
    },
    updated_at: "2026-06-15T00:00:00Z",
    event_ledger_event_id: "EVT-workbench-layout",
  });

  render(<App />);

  const paneA = screen.getByTestId("pane-pane-a");
  await waitFor(() => expect(paneA).toHaveAttribute("data-pane-active-document-id", "doc-beta"));
  expect(paneA).toHaveAttribute("data-pane-open-document-count", "2");
  expect(screen.getByTestId("pane-pane-a.document-tab.doc-alpha")).toHaveAttribute("data-active", "false");
  expect(screen.getByTestId("pane-pane-a.document-tab.doc-beta")).toHaveAttribute("data-active", "true");
  expect(screen.getByTestId("pane-pane-a.document-tab.doc-beta")).toHaveAttribute("data-dirty", "true");
  expect(screen.getByTestId("pane-pane-a.document-tab.doc-beta")).toHaveAttribute("data-pinned", "true");
  expect(screen.getByTestId("document-view.doc-beta")).toBeInTheDocument();
});

it("restores partial pane-local document tab metadata with stable defaults", async () => {
  vi.mocked(getWorkbenchLayoutState).mockResolvedValueOnce({
    workspace_id: "w1",
    layout_state: {
      schema_id: "hsk.workbench_layout_state@1",
      activePaneId: "pane-a",
      activeModule: "MAIN",
      splitWeights: { vertical: 0.5, horizontal: 0.55 },
      drawers: { project: true, file: true, bottom: true },
      panes: [
        {
          id: "pane-a",
          module: "MAIN",
          activeTab: "workspace",
          tabs: ["workspace", "user-manual"],
          locked: false,
          projectRef: "w1",
          activeDocumentId: "doc-alpha",
          openDocuments: [{ documentId: "doc-alpha" }],
        },
        {
          id: "pane-b",
          module: "CKC",
          activeTab: "kernel-dcc",
          tabs: ["kernel-dcc", "problems"],
          locked: false,
          projectRef: "w1",
          activeDocumentId: null,
          openDocuments: [],
        },
        {
          id: "pane-c",
          module: "INGEST",
          activeTab: "flight-recorder",
          tabs: ["flight-recorder", "media-downloader"],
          locked: false,
          projectRef: "w1",
          activeDocumentId: null,
          openDocuments: [],
        },
        {
          id: "pane-d",
          module: "STAGE",
          activeTab: "fonts",
          tabs: ["fonts"],
          locked: false,
          projectRef: "w1",
          activeDocumentId: null,
          openDocuments: [],
        },
      ],
    },
    updated_at: "2026-06-15T00:00:00Z",
    event_ledger_event_id: "EVT-workbench-layout",
  });

  render(<App />);

  const restoredTab = await screen.findByTestId("pane-pane-a.document-tab.doc-alpha");
  expect(restoredTab).toHaveAttribute("data-active", "true");
  expect(restoredTab).toHaveAttribute("data-dirty", "false");
  expect(restoredTab).toHaveAttribute("data-pinned", "false");
  expect(screen.getByTestId("document-view.doc-alpha")).toBeInTheDocument();
});

it("canonicalizes restored pane order before rendering and splitter lock checks", async () => {
  vi.mocked(getWorkbenchLayoutState).mockResolvedValueOnce({
    workspace_id: "w1",
    layout_state: {
      schema_id: "hsk.workbench_layout_state@1",
      activePaneId: "pane-a",
      activeModule: "MAIN",
      splitWeights: { vertical: 0.5, horizontal: 0.55 },
      drawers: { project: true, file: true, bottom: true },
      panes: [
        {
          id: "pane-b",
          module: "CKC",
          activeTab: "kernel-dcc",
          tabs: ["kernel-dcc", "problems"],
          locked: true,
          projectRef: "w1",
          activeDocumentId: null,
          openDocuments: [],
        },
        {
          id: "pane-a",
          module: "MAIN",
          activeTab: "workspace",
          tabs: ["workspace", "user-manual"],
          locked: false,
          projectRef: "w1",
          activeDocumentId: null,
          openDocuments: [],
        },
        {
          id: "pane-c",
          module: "INGEST",
          activeTab: "flight-recorder",
          tabs: ["flight-recorder", "media-downloader"],
          locked: false,
          projectRef: "w1",
          activeDocumentId: null,
          openDocuments: [],
        },
        {
          id: "pane-d",
          module: "STAGE",
          activeTab: "fonts",
          tabs: ["fonts"],
          locked: false,
          projectRef: "w1",
          activeDocumentId: null,
          openDocuments: [],
        },
      ],
    },
    updated_at: "2026-06-15T00:00:00Z",
    event_ledger_event_id: "EVT-workbench-layout",
  });

  render(<App />);

  const paneGrid = screen.getByTestId("pane-grid");
  await waitFor(() => expect(screen.getByTestId("pane-pane-b")).toHaveAttribute("data-pane-locked", "true"));
  const renderedPaneIds = Array.from(paneGrid.querySelectorAll(".main-pane")).map((pane) =>
    pane.getAttribute("data-pane-id"),
  );
  expect(renderedPaneIds).toEqual(["pane-a", "pane-b", "pane-c", "pane-d"]);
  expect(screen.getByTestId("main-window-splitter-horizontal")).not.toBeDisabled();
});

it("keeps new project pane-local document state isolated when project layout hydration is pending", async () => {
  const projectAuxLayout = deferred<{
    workspace_id: string;
    layout_state: null;
    updated_at: null;
    event_ledger_event_id: null;
  }>();
  vi.mocked(listWorkspaces).mockResolvedValueOnce([
    { id: "w1", name: "Workspace 1", created_at: "2025-01-01T00:00:00Z", updated_at: "2025-01-01T00:00:00Z" },
    {
      id: "project-aux",
      name: "Project Aux",
      created_at: "2025-01-01T00:00:00Z",
      updated_at: "2025-01-01T00:00:00Z",
    },
  ]);
  vi.mocked(getWorkbenchLayoutState).mockImplementation(async (workspaceId) => {
    if (workspaceId === "project-aux") {
      return projectAuxLayout.promise;
    }
    return {
      workspace_id: workspaceId,
      layout_state: null,
      updated_at: null,
      event_ledger_event_id: null,
    };
  });

  render(<App />);

  await waitFor(() => expect(getWorkbenchLayoutState).toHaveBeenCalledWith("w1"));
  vi.mocked(saveWorkbenchLayoutState).mockClear();
  fireEvent.click(screen.getByTestId("workspace-sidebar.open-doc-alpha"));
  await waitFor(() => expect(screen.getByTestId("pane-pane-a")).toHaveAttribute("data-pane-active-document-id", "doc-alpha"));
  await waitFor(() => expect(saveWorkbenchLayoutState).toHaveBeenCalledWith("w1", expect.any(Object)));
  vi.mocked(saveWorkbenchLayoutState).mockClear();

  fireEvent.click(await screen.findByTestId("project-project-aux"));
  fireEvent.click(screen.getByTestId("workspace-sidebar.open-doc-beta"));
  projectAuxLayout.resolve({
    workspace_id: "project-aux",
    layout_state: null,
    updated_at: null,
    event_ledger_event_id: null,
  });

  const paneA = screen.getByTestId("pane-pane-a");
  await waitFor(() => expect(paneA).toHaveAttribute("data-pane-project-ref", "project-aux"));
  expect(paneA).toHaveAttribute("data-pane-active-document-id", "doc-beta");
  expect(screen.queryByTestId("pane-pane-a.document-tab.doc-alpha")).not.toBeInTheDocument();
  await waitFor(() => expect(saveWorkbenchLayoutState).toHaveBeenCalledWith("project-aux", expect.any(Object)));
  const projectAuxSave = vi.mocked(saveWorkbenchLayoutState).mock.calls.find((call) => call[0] === "project-aux")?.[1] as {
    panes?: Array<{
      id: string;
      projectRef: string;
      activeDocumentId?: string | null;
      openDocuments?: Array<{ documentId: string }>;
    }>;
  };
  const savedPaneA = projectAuxSave.panes?.find((pane) => pane.id === "pane-a");
  expect(savedPaneA?.projectRef).toBe("project-aux");
  expect(savedPaneA?.activeDocumentId).toBe("doc-beta");
  expect(savedPaneA?.openDocuments?.map((document) => document.documentId)).toEqual(["doc-beta"]);
});

it("keeps canvas selection pane-local across workspace panes", async () => {
  vi.mocked(getWorkbenchLayoutState).mockResolvedValueOnce({
    workspace_id: "w1",
    layout_state: {
      schema_id: "hsk.workbench_layout_state@1",
      activePaneId: "pane-a",
      activeModule: "MAIN",
      splitWeights: { vertical: 0.5, horizontal: 0.55 },
      drawers: { project: true, file: true, bottom: true },
      panes: [
        {
          id: "pane-a",
          module: "MAIN",
          activeTab: "workspace",
          tabs: ["workspace", "user-manual"],
          locked: false,
          projectRef: "w1",
          activeDocumentId: null,
          activeCanvasId: null,
          openDocuments: [],
        },
        {
          id: "pane-b",
          module: "MAIN",
          activeTab: "workspace",
          tabs: ["workspace", "problems"],
          locked: false,
          projectRef: "w1",
          activeDocumentId: null,
          activeCanvasId: null,
          openDocuments: [],
        },
        {
          id: "pane-c",
          module: "INGEST",
          activeTab: "flight-recorder",
          tabs: ["flight-recorder", "media-downloader"],
          locked: false,
          projectRef: "w1",
          activeDocumentId: null,
          activeCanvasId: null,
          openDocuments: [],
        },
        {
          id: "pane-d",
          module: "STAGE",
          activeTab: "fonts",
          tabs: ["fonts"],
          locked: false,
          projectRef: "w1",
          activeDocumentId: null,
          activeCanvasId: null,
          openDocuments: [],
        },
      ],
    },
    updated_at: "2026-06-15T00:00:00Z",
    event_ledger_event_id: "EVT-workbench-layout",
  });

  render(<App />);

  await waitFor(() => expect(screen.getByTestId("pane-pane-b")).toHaveAttribute("data-pane-active-tab", "workspace"));
  fireEvent.click(screen.getByTestId("workspace-sidebar.open-canvas-alpha"));

  expect(screen.getByTestId("pane-pane-a")).toHaveAttribute("data-pane-active-canvas-id", "canvas-alpha");
  expect(screen.getByTestId("pane-pane-b")).toHaveAttribute("data-pane-active-canvas-id", "");
  expect(screen.getAllByTestId("canvas-view.canvas-alpha")).toHaveLength(1);
});

it("reactivates an open document when a pane-local canvas is cleared and saves matching layout state", async () => {
  render(<App />);

  await waitFor(() => expect(getWorkbenchLayoutState).toHaveBeenCalledWith("w1"));
  vi.mocked(saveWorkbenchLayoutState).mockClear();

  fireEvent.click(screen.getByTestId("workspace-sidebar.open-doc-alpha"));
  fireEvent.click(screen.getByTestId("workspace-sidebar.open-canvas-alpha"));

  const paneA = screen.getByTestId("pane-pane-a");
  await waitFor(() => expect(paneA).toHaveAttribute("data-pane-active-canvas-id", "canvas-alpha"));
  await waitFor(() => expect(saveWorkbenchLayoutState).toHaveBeenCalled());
  vi.mocked(saveWorkbenchLayoutState).mockClear();

  fireEvent.click(screen.getByTestId("canvas-view.canvas-alpha.delete"));

  await waitFor(() => expect(paneA).toHaveAttribute("data-pane-active-document-id", "doc-alpha"));
  expect(paneA).toHaveAttribute("data-pane-active-canvas-id", "");
  expect(screen.getByTestId("document-view.doc-alpha")).toBeInTheDocument();
  await waitFor(() => expect(saveWorkbenchLayoutState).toHaveBeenCalled());
  const saveCalls = vi.mocked(saveWorkbenchLayoutState).mock.calls;
  const saved = saveCalls[saveCalls.length - 1]?.[1] as {
    panes?: Array<{
      id: string;
      activeDocumentId?: string | null;
      activeCanvasId?: string | null;
      openDocuments?: Array<{ documentId: string }>;
    }>;
  };
  const savedPaneA = saved.panes?.find((pane) => pane.id === "pane-a");
  expect(savedPaneA?.activeDocumentId).toBe("doc-alpha");
  expect(savedPaneA?.activeCanvasId).toBeNull();
  expect(savedPaneA?.openDocuments?.map((document) => document.documentId)).toEqual(["doc-alpha"]);
});

it("persists workbench layout changes and resets through the durable layout endpoint", async () => {
  render(<App />);

  await waitFor(() => expect(getWorkbenchLayoutState).toHaveBeenCalledWith("w1"));
  vi.mocked(saveWorkbenchLayoutState).mockClear();

  const splitter = screen.getByTestId("main-window-splitter-vertical");
  fireEvent.keyDown(splitter, { key: "ArrowRight" });

  await waitFor(() => expect(saveWorkbenchLayoutState).toHaveBeenCalled());
  const savedAfterSplitCalls = vi.mocked(saveWorkbenchLayoutState).mock.calls;
  const savedAfterSplit = savedAfterSplitCalls[savedAfterSplitCalls.length - 1]?.[1] as {
    splitWeights?: { vertical?: number; horizontal?: number };
  };
  expect(savedAfterSplit.splitWeights?.vertical).toBeCloseTo(0.55);

  fireEvent.click(screen.getByTestId("settings-gear"));
  const reset = await screen.findByTestId("setting-reset-layout.control");
  expect(reset).not.toBeDisabled();
  vi.mocked(saveWorkbenchLayoutState).mockClear();
  fireEvent.click(reset);

  await waitFor(() => expect(saveWorkbenchLayoutState).toHaveBeenCalled());
  const savedAfterResetCalls = vi.mocked(saveWorkbenchLayoutState).mock.calls;
  const savedAfterReset = savedAfterResetCalls[savedAfterResetCalls.length - 1]?.[1] as {
    splitWeights?: { vertical?: number; horizontal?: number };
    drawers?: { project?: boolean; file?: boolean; bottom?: boolean };
  };
  expect(savedAfterReset.splitWeights?.vertical).toBe(0.5);
  expect(savedAfterReset.splitWeights?.horizontal).toBe(0.55);
  expect(savedAfterReset.drawers).toEqual({ project: true, file: true, bottom: true });
});

it("serializes durable layout saves so late older requests cannot overwrite newer state", async () => {
  render(<App />);

  const mainWindow = screen.getByTestId("main-window");
  await waitFor(() => expect(mainWindow).toHaveAttribute("data-active-project-id", "w1"));
  await waitFor(() => expect(getWorkbenchLayoutState).toHaveBeenCalledWith("w1"));
  await act(async () => {
    await Promise.resolve();
    await Promise.resolve();
  });

  const firstSave = deferred<void>();
  vi.mocked(saveWorkbenchLayoutState).mockReset();
  vi.mocked(saveWorkbenchLayoutState)
    .mockImplementationOnce(async (workspaceId, layoutState) => {
      await firstSave.promise;
      return workbenchLayoutResponse(workspaceId, layoutState, "EVT-workbench-layout-first");
    })
    .mockImplementationOnce(async (workspaceId, layoutState) =>
      workbenchLayoutResponse(workspaceId, layoutState, "EVT-workbench-layout-latest"),
    );

  const splitter = screen.getByTestId("main-window-splitter-vertical");
  fireEvent.keyDown(splitter, { key: "ArrowRight" });
  await waitFor(() => expect(saveWorkbenchLayoutState).toHaveBeenCalledTimes(1));

  fireEvent.keyDown(splitter, { key: "ArrowRight" });
  await act(async () => {
    await Promise.resolve();
    await Promise.resolve();
  });
  expect(saveWorkbenchLayoutState).toHaveBeenCalledTimes(1);

  await act(async () => {
    firstSave.resolve();
    await firstSave.promise;
  });
  await waitFor(() => expect(saveWorkbenchLayoutState).toHaveBeenCalledTimes(2));

  const secondCall = vi.mocked(saveWorkbenchLayoutState).mock.calls[1]?.[1] as {
    splitWeights?: { vertical?: number };
  };
  expect(secondCall.splitWeights?.vertical).toBeCloseTo(0.6);
});

it("retries a failed durable layout save without another layout mutation", async () => {
  render(<App />);

  await waitFor(() => expect(getWorkbenchLayoutState).toHaveBeenCalledWith("w1"));
  await act(async () => {
    await Promise.resolve();
    await Promise.resolve();
  });
  vi.mocked(saveWorkbenchLayoutState).mockReset();
  vi.mocked(saveWorkbenchLayoutState)
    .mockRejectedValueOnce(new Error("transient layout save failure"))
    .mockImplementationOnce(async (workspaceId, layoutState) =>
      workbenchLayoutResponse(workspaceId, layoutState, "EVT-workbench-layout-retry"),
    );

  fireEvent.click(screen.getByTestId("workspace-sidebar.open-doc-alpha"));

  await waitFor(() => expect(saveWorkbenchLayoutState).toHaveBeenCalledTimes(1));
  await waitFor(() => expect(saveWorkbenchLayoutState).toHaveBeenCalledTimes(2));
  const retriedLayout = vi.mocked(saveWorkbenchLayoutState).mock.calls[1]?.[1] as {
    panes?: Array<{
      id: string;
      activeDocumentId?: string | null;
      openDocuments?: Array<{ documentId: string }>;
    }>;
  };
  const retriedPaneA = retriedLayout.panes?.find((pane) => pane.id === "pane-a");
  expect(retriedPaneA?.activeDocumentId).toBe("doc-alpha");
  expect(retriedPaneA?.openDocuments?.map((document) => document.documentId)).toEqual(["doc-alpha"]);
});

it("bounds durable layout save retries during a persistent backend failure", async () => {
  render(<App />);

  await waitFor(() => expect(getWorkbenchLayoutState).toHaveBeenCalledWith("w1"));
  await act(async () => {
    await Promise.resolve();
    await Promise.resolve();
  });
  vi.mocked(saveWorkbenchLayoutState).mockReset();
  vi.mocked(saveWorkbenchLayoutState).mockRejectedValue(new Error("persistent layout save failure"));

  fireEvent.click(screen.getByTestId("workspace-sidebar.open-doc-alpha"));

  await waitFor(() => expect(saveWorkbenchLayoutState).toHaveBeenCalledTimes(4));
  await act(async () => {
    await new Promise((resolve) => window.setTimeout(resolve, 300));
  });
  expect(saveWorkbenchLayoutState).toHaveBeenCalledTimes(4);
  await waitFor(() =>
    expect(screen.getByTestId("workbench-layout.status")).toHaveAttribute(
      "data-layout-persistence-state",
      "save-error",
    ),
  );
  expect(screen.getByTestId("workbench-layout.status")).toHaveTextContent("Layout changes are not saved");
});

it("surfaces durable layout load failures instead of silently treating local layout as saved", async () => {
  vi.mocked(getWorkbenchLayoutState).mockRejectedValueOnce(new Error("layout load unavailable"));

  render(<App />);

  await waitFor(() => expect(getWorkbenchLayoutState).toHaveBeenCalledWith("w1"));
  await waitFor(() =>
    expect(screen.getByTestId("workbench-layout.status")).toHaveAttribute(
      "data-layout-persistence-state",
      "load-error",
    ),
  );
  expect(screen.getByTestId("workbench-layout.status")).toHaveTextContent("Layout load failed");
});

it("surfaces malformed durable layout rows instead of silently replacing them with defaults", async () => {
  vi.mocked(getWorkbenchLayoutState).mockResolvedValueOnce({
    workspace_id: "w1",
    layout_state: {
      schema_id: "hsk.workbench_layout_state@1",
    },
    updated_at: "2026-06-15T00:00:00Z",
    event_ledger_event_id: "EVT-workbench-layout-malformed",
  });

  render(<App />);

  await waitFor(() => expect(getWorkbenchLayoutState).toHaveBeenCalledWith("w1"));
  await waitFor(() =>
    expect(screen.getByTestId("workbench-layout.status")).toHaveAttribute(
      "data-layout-persistence-state",
      "load-error",
    ),
  );
  expect(screen.getByTestId("workbench-layout.status")).toHaveTextContent("stored layout is not renderable");
  expect(saveWorkbenchLayoutState).not.toHaveBeenCalled();
});

it("opens workspace documents as pane-local tabs and persists the open document list", async () => {
  render(<App />);

  await waitFor(() => expect(getWorkbenchLayoutState).toHaveBeenCalledWith("w1"));
  vi.mocked(saveWorkbenchLayoutState).mockClear();

  fireEvent.click(screen.getByTestId("workspace-sidebar.open-doc-alpha"));

  const paneA = screen.getByTestId("pane-pane-a");
  expect(paneA).toHaveAttribute("data-pane-active-document-id", "doc-alpha");
  expect(screen.getByTestId("pane-pane-a.document-tab.doc-alpha")).toHaveAttribute("data-active", "true");
  expect(screen.getByTestId("document-view.doc-alpha")).toBeInTheDocument();

  await waitFor(() => expect(saveWorkbenchLayoutState).toHaveBeenCalled());
  const saveCalls = vi.mocked(saveWorkbenchLayoutState).mock.calls;
  const saved = saveCalls[saveCalls.length - 1]?.[1] as {
    panes?: Array<{
      id: string;
      activeDocumentId?: string | null;
      openDocuments?: Array<{ documentId: string; pinned: boolean; dirty: boolean }>;
    }>;
  };
  const savedPaneA = saved.panes?.find((pane) => pane.id === "pane-a");
  expect(savedPaneA?.activeDocumentId).toBe("doc-alpha");
  expect(savedPaneA?.openDocuments).toEqual([{ documentId: "doc-alpha", pinned: false, dirty: false }]);

  fireEvent.click(screen.getByTestId("pane-pane-b.tab.problems"));
  fireEvent.click(screen.getByTestId("workspace-sidebar.open-doc-beta"));

  const paneB = screen.getByTestId("pane-pane-b");
  expect(paneA).toHaveAttribute("data-pane-active-document-id", "doc-alpha");
  expect(paneB).toHaveAttribute("data-pane-type", "workspace");
  expect(paneB).toHaveAttribute("data-pane-active-document-id", "doc-beta");
  expect(screen.getByTestId("pane-pane-b.document-tab.doc-beta")).toHaveAttribute("data-active", "true");
  expect(screen.getByTestId("document-view.doc-beta")).toBeInTheDocument();

  fireEvent.click(screen.getByTestId("pane-pane-a.document-tab.doc-alpha.close"));

  expect(paneA).toHaveAttribute("data-pane-active-document-id", "");
  expect(screen.queryByTestId("pane-pane-a.document-tab.doc-alpha")).not.toBeInTheDocument();
  expect(paneB).toHaveAttribute("data-pane-active-document-id", "doc-beta");
});

it("pins and reorders pane-local document tabs in the durable layout", async () => {
  render(<App />);

  await waitFor(() => expect(getWorkbenchLayoutState).toHaveBeenCalledWith("w1"));
  vi.mocked(saveWorkbenchLayoutState).mockClear();

  fireEvent.click(screen.getByTestId("workspace-sidebar.open-doc-alpha"));
  fireEvent.click(screen.getByTestId("workspace-sidebar.open-doc-beta"));

  const tabs = screen.getByTestId("pane-pane-a.document-tabs");
  const documentTabOrder = () =>
    Array.from(tabs.querySelectorAll<HTMLElement>("[data-document-id]")).map((tab) => tab.dataset.documentId);
  expect(documentTabOrder()).toEqual([
    "doc-alpha",
    "doc-beta",
  ]);

  fireEvent.click(screen.getByTestId("pane-pane-a.document-tab.doc-beta.pin"));
  fireEvent.click(screen.getByTestId("pane-pane-a.document-tab.doc-beta.move-left"));

  expect(screen.getByTestId("pane-pane-a.document-tab.doc-beta")).toHaveAttribute("data-pinned", "true");
  expect(documentTabOrder()).toEqual([
    "doc-beta",
    "doc-alpha",
  ]);

  await waitFor(() => expect(saveWorkbenchLayoutState).toHaveBeenCalled());
  const saveCalls = vi.mocked(saveWorkbenchLayoutState).mock.calls;
  const saved = saveCalls[saveCalls.length - 1]?.[1] as {
    panes?: Array<{
      id: string;
      openDocuments?: Array<{ documentId: string; pinned: boolean; dirty: boolean }>;
    }>;
  };
  const savedPaneA = saved.panes?.find((pane) => pane.id === "pane-a");
  expect(savedPaneA?.openDocuments).toEqual([
    { documentId: "doc-beta", pinned: true, dirty: false },
    { documentId: "doc-alpha", pinned: false, dirty: false },
  ]);
});

it("drags pane-local document tabs between editor panes and persists the target pane", async () => {
  render(<App />);

  await waitFor(() => expect(getWorkbenchLayoutState).toHaveBeenCalledWith("w1"));
  vi.mocked(saveWorkbenchLayoutState).mockClear();

  fireEvent.click(screen.getByTestId("workspace-sidebar.open-doc-alpha"));
  fireEvent.click(screen.getByTestId("pane-pane-b.tab.problems"));
  fireEvent.click(screen.getByTestId("workspace-sidebar.open-doc-beta"));

  const draggedTab = screen.getByTestId("pane-pane-a.document-tab.doc-alpha");
  const paneBDropTarget = screen.getByTestId("pane-pane-b.document-tabs");
  const dragData = new Map<string, string>();
  const dataTransfer = {
    effectAllowed: "move",
    dropEffect: "move",
    setData: (type: string, value: string) => dragData.set(type, value),
    getData: (type: string) => dragData.get(type) ?? "",
  };

  fireEvent.dragStart(draggedTab, { dataTransfer });
  fireEvent.dragOver(paneBDropTarget, { dataTransfer });
  fireEvent.drop(paneBDropTarget, { dataTransfer });

  const paneA = screen.getByTestId("pane-pane-a");
  const paneB = screen.getByTestId("pane-pane-b");
  expect(screen.queryByTestId("pane-pane-a.document-tab.doc-alpha")).not.toBeInTheDocument();
  expect(paneA).toHaveAttribute("data-pane-active-document-id", "");
  expect(paneB).toHaveAttribute("data-pane-active-document-id", "doc-alpha");
  expect(screen.getByTestId("pane-pane-b.document-tab.doc-alpha")).toHaveAttribute("data-active", "true");

  await waitFor(() => expect(saveWorkbenchLayoutState).toHaveBeenCalled());
  const saveCalls = vi.mocked(saveWorkbenchLayoutState).mock.calls;
  const saved = saveCalls[saveCalls.length - 1]?.[1] as {
    panes?: Array<{
      id: string;
      activeDocumentId?: string | null;
      openDocuments?: Array<{ documentId: string; pinned: boolean; dirty: boolean }>;
    }>;
  };
  const savedPaneA = saved.panes?.find((pane) => pane.id === "pane-a");
  const savedPaneB = saved.panes?.find((pane) => pane.id === "pane-b");
  expect(savedPaneA?.openDocuments).toEqual([]);
  expect(savedPaneB?.activeDocumentId).toBe("doc-alpha");
  expect(savedPaneB?.openDocuments).toEqual([
    { documentId: "doc-beta", pinned: false, dirty: false },
    { documentId: "doc-alpha", pinned: false, dirty: false },
  ]);
});

it("preserves source document tab metadata when dragged onto a pane that already has the document", async () => {
  render(<App />);

  await waitFor(() => expect(getWorkbenchLayoutState).toHaveBeenCalledWith("w1"));
  vi.mocked(saveWorkbenchLayoutState).mockClear();

  fireEvent.click(screen.getByTestId("workspace-sidebar.open-doc-alpha"));
  fireEvent.click(screen.getByTestId("document-view.doc-alpha.mark-dirty"));
  fireEvent.click(screen.getByTestId("pane-pane-a.document-tab.doc-alpha.pin"));
  fireEvent.keyDown(window, { key: "ArrowRight", ctrlKey: true, altKey: true });
  fireEvent.click(screen.getByTestId("workspace-sidebar.open-doc-alpha"));

  const draggedTab = screen.getByTestId("pane-pane-a.document-tab.doc-alpha");
  const paneBDropTarget = screen.getByTestId("pane-pane-b.document-tabs");
  const dragData = new Map<string, string>();
  const dataTransfer = {
    effectAllowed: "move",
    dropEffect: "move",
    setData: (type: string, value: string) => dragData.set(type, value),
    getData: (type: string) => dragData.get(type) ?? "",
  };

  fireEvent.dragStart(draggedTab, { dataTransfer });
  fireEvent.dragOver(paneBDropTarget, { dataTransfer });
  fireEvent.drop(paneBDropTarget, { dataTransfer });

  expect(screen.queryByTestId("pane-pane-a.document-tab.doc-alpha")).not.toBeInTheDocument();
  const paneBDocAlpha = screen.getByTestId("pane-pane-b.document-tab.doc-alpha");
  expect(paneBDocAlpha).toHaveAttribute("data-dirty", "true");
  expect(paneBDocAlpha).toHaveAttribute("data-pinned", "true");

  await waitFor(() => expect(saveWorkbenchLayoutState).toHaveBeenCalled());
  const saveCalls = vi.mocked(saveWorkbenchLayoutState).mock.calls;
  const saved = saveCalls[saveCalls.length - 1]?.[1] as {
    panes?: Array<{
      id: string;
      activeDocumentId?: string | null;
      openDocuments?: Array<{ documentId: string; pinned: boolean; dirty: boolean }>;
    }>;
  };
  const savedPaneA = saved.panes?.find((pane) => pane.id === "pane-a");
  const savedPaneB = saved.panes?.find((pane) => pane.id === "pane-b");
  expect(savedPaneA?.openDocuments).toEqual([]);
  expect(savedPaneB?.activeDocumentId).toBe("doc-alpha");
  expect(savedPaneB?.openDocuments).toEqual([{ documentId: "doc-alpha", pinned: true, dirty: true }]);
});

it("ignores stale document tab drops without moving editor focus", async () => {
  render(<App />);

  await waitFor(() => expect(getWorkbenchLayoutState).toHaveBeenCalledWith("w1"));

  fireEvent.click(screen.getByTestId("workspace-sidebar.open-doc-alpha"));
  fireEvent.keyDown(window, { key: "ArrowRight", ctrlKey: true, altKey: true });
  fireEvent.click(screen.getByTestId("workspace-sidebar.open-doc-beta"));
  fireEvent.keyDown(window, { key: "ArrowLeft", ctrlKey: true, altKey: true });

  const mainWindow = screen.getByTestId("main-window");
  const paneB = screen.getByTestId("pane-pane-b");
  expect(mainWindow).toHaveAttribute("data-active-pane-id", "pane-a");
  expect(paneB).toHaveAttribute("data-pane-active-document-id", "doc-beta");
  vi.mocked(saveWorkbenchLayoutState).mockClear();

  const stalePayload = JSON.stringify({ paneId: "pane-a", documentId: "doc-stale" });
  const dataTransfer = {
    effectAllowed: "move",
    dropEffect: "move",
    types: ["application/x-handshake-document-tab"],
    getData: (type: string) => (type === "application/x-handshake-document-tab" ? stalePayload : ""),
    setData: vi.fn(),
  };

  fireEvent.dragOver(screen.getByTestId("pane-pane-b.document-tabs"), { dataTransfer });
  fireEvent.drop(screen.getByTestId("pane-pane-b.document-tabs"), { dataTransfer });

  expect(mainWindow).toHaveAttribute("data-active-pane-id", "pane-a");
  expect(paneB).toHaveAttribute("data-pane-active-document-id", "doc-beta");
  expect(saveWorkbenchLayoutState).not.toHaveBeenCalled();
});

it("reflects live document dirty state in pane-local tabs and durable layout", async () => {
  render(<App />);

  await waitFor(() => expect(getWorkbenchLayoutState).toHaveBeenCalledWith("w1"));
  vi.mocked(saveWorkbenchLayoutState).mockClear();

  fireEvent.click(screen.getByTestId("workspace-sidebar.open-doc-alpha"));
  const tab = screen.getByTestId("pane-pane-a.document-tab.doc-alpha");
  expect(tab).toHaveAttribute("data-dirty", "false");

  fireEvent.click(screen.getByTestId("document-view.doc-alpha.mark-dirty"));

  expect(tab).toHaveAttribute("data-dirty", "true");
  await waitFor(() => expect(saveWorkbenchLayoutState).toHaveBeenCalled());
  const saveCalls = vi.mocked(saveWorkbenchLayoutState).mock.calls;
  const saved = saveCalls[saveCalls.length - 1]?.[1] as {
    panes?: Array<{
      id: string;
      openDocuments?: Array<{ documentId: string; pinned: boolean; dirty: boolean }>;
    }>;
  };
  const savedPaneA = saved.panes?.find((pane) => pane.id === "pane-a");
  expect(savedPaneA?.openDocuments).toEqual([{ documentId: "doc-alpha", pinned: false, dirty: true }]);

  vi.mocked(saveWorkbenchLayoutState).mockClear();
  fireEvent.click(screen.getByTestId("document-view.doc-alpha.mark-clean"));

  expect(tab).toHaveAttribute("data-dirty", "false");
  await waitFor(() => expect(saveWorkbenchLayoutState).toHaveBeenCalled());
  const cleanSaveCalls = vi.mocked(saveWorkbenchLayoutState).mock.calls;
  const cleanSaved = cleanSaveCalls[cleanSaveCalls.length - 1]?.[1] as typeof saved;
  const cleanSavedPaneA = cleanSaved.panes?.find((pane) => pane.id === "pane-a");
  expect(cleanSavedPaneA?.openDocuments).toEqual([{ documentId: "doc-alpha", pinned: false, dirty: false }]);
});

it("guards dirty pane-local tab close with an explicit discard confirmation", async () => {
  const confirmSpy = vi.spyOn(window, "confirm");
  try {
    confirmSpy.mockReturnValueOnce(false).mockReturnValueOnce(true);
    render(<App />);

    await waitFor(() => expect(getWorkbenchLayoutState).toHaveBeenCalledWith("w1"));
    fireEvent.click(screen.getByTestId("workspace-sidebar.open-doc-alpha"));
    await screen.findByTestId("pane-pane-a.document-tab.doc-alpha");
    fireEvent.click(screen.getByTestId("document-view.doc-alpha.mark-dirty"));
    await waitFor(() =>
      expect(screen.getByTestId("pane-pane-a.document-tab.doc-alpha")).toHaveAttribute("data-dirty", "true"),
    );

    fireEvent.click(screen.getByTestId("pane-pane-a.document-tab.doc-alpha.close"));
    expect(confirmSpy).toHaveBeenCalledWith("Close doc-alpha and discard unsaved document changes?");
    expect(screen.getByTestId("pane-pane-a.document-tab.doc-alpha")).toBeInTheDocument();
    expect(screen.getByTestId("document-view.doc-alpha")).toBeInTheDocument();

    fireEvent.click(screen.getByTestId("pane-pane-a.document-tab.doc-alpha.close"));
    await waitFor(() => expect(screen.queryByTestId("pane-pane-a.document-tab.doc-alpha")).not.toBeInTheDocument());
    expect(screen.queryByTestId("document-view.doc-alpha")).not.toBeInTheDocument();
  } finally {
    confirmSpy.mockRestore();
  }
});

it("navigates editor groups by keyboard and routes workspace opens to the focused pane", async () => {
  render(<App />);

  await waitFor(() => expect(getWorkbenchLayoutState).toHaveBeenCalledWith("w1"));
  const mainWindow = screen.getByTestId("main-window");
  expect(mainWindow).toHaveAttribute("data-active-pane-id", "pane-a");
  expect(screen.getByTestId("pane-pane-a")).toHaveAttribute("data-pane-active", "true");

  fireEvent.keyDown(window, { key: "ArrowRight", ctrlKey: true, altKey: true });
  expect(mainWindow).toHaveAttribute("data-active-pane-id", "pane-b");
  expect(screen.getByTestId("pane-pane-b")).toHaveAttribute("data-pane-active", "true");

  fireEvent.click(screen.getByTestId("workspace-sidebar.open-doc-alpha"));
  expect(screen.getByTestId("pane-pane-b")).toHaveAttribute("data-pane-active-document-id", "doc-alpha");

  fireEvent.keyDown(window, { key: "ArrowDown", ctrlKey: true, altKey: true });
  expect(mainWindow).toHaveAttribute("data-active-pane-id", "pane-d");

  fireEvent.click(screen.getByTestId("workspace-sidebar.open-doc-beta"));
  expect(screen.getByTestId("pane-pane-d")).toHaveAttribute("data-pane-active-document-id", "doc-beta");
  expect(screen.getByTestId("pane-pane-b")).toHaveAttribute("data-pane-active-document-id", "doc-alpha");
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

it("opens the quick switcher with Ctrl+P, queries graph search, and opens UserManual page hits", async () => {
  vi.mocked(searchLoomGraph).mockClear();
  vi.mocked(getUserManualPage).mockClear();

  render(<App />);

  fireEvent.keyDown(window, { key: "p", ctrlKey: true });
  const quickSearch = await screen.findByTestId("quick-switcher.search");
  fireEvent.change(quickSearch, { target: { value: "GraphSearchAlpha" } });

  await waitFor(() =>
    expect(searchLoomGraph).toHaveBeenCalledWith("w1", {
      q: "GraphSearchAlpha",
      limit: 25,
      sourceKinds: [
        "loom_block",
        "file",
        "tag_hub",
        "document",
        "symbol",
        "work_packet",
        "micro_task",
        "user_manual_page",
        "wiki_page",
      ],
    }),
  );
  expect(await screen.findByTestId("quick-switcher.result.loom_block.block-alpha")).toHaveTextContent("Loom Block");
  expect(screen.getByTestId("quick-switcher.result.file.file-alpha")).toHaveTextContent("File");
  expect(screen.getByTestId("quick-switcher.result.tag_hub.tag-alpha")).toHaveTextContent("Tag Hub");
  expect(screen.getByTestId("quick-switcher.result.document.krd-00000000000000000000000000000001")).toHaveTextContent("Document");
  expect(screen.getByTestId("quick-switcher.result.symbol.rust-src-backend-graph-search-rs-graphsearchalpha")).toHaveTextContent("Symbol");
  expect(screen.getByTestId("quick-switcher.result.work_packet.wp-kernel-009-graphsearchalpha")).toHaveTextContent("Work Packet");
  expect(screen.getByTestId("quick-switcher.result.micro_task.mt-186-graphsearchalpha")).toHaveTextContent("Microtask");
  expect(screen.getByTestId("quick-switcher.result.wiki_page.kwp-alpha")).toHaveTextContent("Wiki Page");
  const manualHit = screen.getByTestId("quick-switcher.result.user_manual_page.graph-search-alpha");
  expect(manualHit).toHaveTextContent("UserManual Page");

  fireEvent.click(manualHit);

  expect(await screen.findByTestId("usermanual-panel")).toBeInTheDocument();
  expect(screen.getByTestId("pane-pane-a")).toHaveAttribute("data-pane-type", "user-manual");
  await waitFor(() => expect(getUserManualPage).toHaveBeenLastCalledWith("graph-search-alpha"));
});

it("opens workspace find-in-files from the bottom strip and queries graph search", async () => {
  vi.mocked(searchLoomGraph).mockClear();

  render(<App />);

  fireEvent.click(screen.getByTestId("workspace-search.open"));
  expect(await screen.findByTestId("workspace-search")).toBeInTheDocument();
  fireEvent.change(screen.getByTestId("workspace-search.query"), { target: { value: "GraphSearchAlpha" } });
  fireEvent.click(screen.getByTestId("workspace-search.search"));

  await waitFor(() =>
    expect(searchLoomGraph).toHaveBeenCalledWith("w1", {
      q: "GraphSearchAlpha",
      limit: 500,
      offset: 0,
      sourceKinds: undefined,
      tagIds: [],
      caseSensitive: false,
      wholeWord: false,
      isRegex: false,
      path: undefined,
    }),
  );
  expect(await screen.findByTestId("workspace-search.result.loom_block.block-alpha")).toHaveTextContent("Loom Block");

  fireEvent.click(screen.getByTestId("workspace-search.result.document.KRD-00000000000000000000000000000001"));
  const findRequest = await screen.findByTestId(
    "document-view.KRD-00000000000000000000000000000001.find-request",
  );
  expect(findRequest).toHaveAttribute("data-query", "GraphSearchAlpha");
  expect(findRequest).toHaveAttribute("data-case-sensitive", "false");
  expect(findRequest).toHaveAttribute("data-whole-word", "false");
  expect(findRequest).toHaveAttribute("data-regex", "false");
});

it("opens the quick switcher with Ctrl+P while focus is inside an editable field", async () => {
  render(<App />);

  const searchInput = screen.getByTestId("search-input");
  searchInput.focus();
  fireEvent.keyDown(searchInput, { key: "p", ctrlKey: true });

  expect(await screen.findByTestId("quick-switcher.search")).toBeInTheDocument();
});

it("opens quick switcher Loom block hits in a backend-backed block panel without requiring a document id", async () => {
  vi.mocked(searchLoomGraph).mockResolvedValueOnce([
    {
      result_kind: "loom_block",
      source_kind: "loom_block",
      ref_id: "block-alpha",
      title: "GraphSearchAlpha standalone Loom note",
      excerpt: "A standalone block without document linkage.",
      score: 4.2,
      metadata: { content_type: "note" },
    },
  ]);
  vi.mocked(getLoomBlock).mockClear();

  render(<App />);

  fireEvent.keyDown(window, { key: "p", ctrlKey: true });
  const quickSearch = await screen.findByTestId("quick-switcher.search");
  fireEvent.change(quickSearch, { target: { value: "GraphSearchAlpha" } });

  const loomHit = await screen.findByTestId("quick-switcher.result.loom_block.block-alpha");
  expect(loomHit).toHaveTextContent("Open Loom block");
  fireEvent.click(loomHit);

  await waitFor(() =>
    expect(screen.getByTestId("loom-block-panel")).toHaveTextContent("GraphSearchAlpha standalone Loom note"),
  );
  expect(screen.getByTestId("loom-block-panel")).toHaveTextContent("GraphSearchAlpha joins notes to code and manuals.");
  expect(screen.getByTestId("pane-pane-a")).toHaveAttribute("data-pane-type", "loom-block");
  await waitFor(() => expect(getLoomBlock).toHaveBeenCalledWith("w1", "block-alpha"));
});

it("opens Loom bookmark selections from the file drawer in the backend-backed block panel", async () => {
  vi.mocked(getLoomBlock).mockClear();

  render(<App />);

  fireEvent.click(await screen.findByTestId("workspace-sidebar.open-bookmark-block-alpha"));

  await waitFor(() =>
    expect(screen.getByTestId("loom-block-panel")).toHaveTextContent("GraphSearchAlpha standalone Loom note"),
  );
  expect(screen.getByTestId("pane-pane-a")).toHaveAttribute("data-pane-type", "loom-block");
  await waitFor(() => expect(getLoomBlock).toHaveBeenCalledWith("w1", "block-alpha"));
});

it("routes hsLink transclusion navigation to the Loom source without copying over the active document", async () => {
  vi.mocked(getLoomBlock).mockClear();

  render(<App />);

  fireEvent.click(await screen.findByTestId("workspace-sidebar.open-doc-alpha"));
  expect(await screen.findByTestId("document-view.doc-alpha")).toBeInTheDocument();
  expect(screen.getByTestId("pane-pane-a")).toHaveAttribute("data-pane-type", "workspace");
  expect(screen.getByTestId("pane-pane-a")).toHaveAttribute("data-pane-active-document-id", "doc-alpha");
  expect(screen.getByTestId("pane-pane-a")).toHaveAttribute("data-pane-open-document-count", "1");

  act(() => {
    window.dispatchEvent(
      new CustomEvent(HS_LINK_NAVIGATE_EVENT, {
        detail: {
          refKind: "note",
          refValue: "block-alpha",
          label: "GraphSearchAlpha standalone Loom note",
        },
      }),
    );
  });

  await waitFor(() =>
    expect(screen.getByTestId("loom-block-panel")).toHaveTextContent("GraphSearchAlpha standalone Loom note"),
  );
  expect(screen.getByTestId("pane-pane-a")).toHaveAttribute("data-pane-type", "loom-block");
  expect(screen.getByTestId("pane-pane-a")).toHaveAttribute("data-pane-active-document-id", "doc-alpha");
  expect(screen.getByTestId("pane-pane-a")).toHaveAttribute("data-pane-open-document-count", "1");
  fireEvent.click(screen.getByTestId("pane-pane-a.tab.workspace"));
  expect(screen.getByTestId("pane-pane-a")).toHaveAttribute("data-pane-type", "workspace");
  expect(screen.getByTestId("document-view.doc-alpha")).toBeInTheDocument();
  expect(screen.getByTestId("pane-pane-a.document-tab.doc-alpha")).toHaveAttribute("data-active", "true");
  await waitFor(() => expect(getLoomBlock).toHaveBeenCalledWith("w1", "block-alpha"));
});

it("opens the daily journal tab and auto-opens today's backend journal", async () => {
  render(<App />);

  fireEvent.click(await screen.findByTestId("pane-pane-a.tab.loom-daily-journal"));

  await waitFor(() => expect(screen.getByTestId("loom-daily-journal-panel")).toBeInTheDocument());
  expect(screen.getByTestId("pane-pane-a")).toHaveAttribute("data-pane-type", "loom-daily-journal");
  expect(openDailyJournal).toHaveBeenCalledWith("w1", expect.stringMatching(/^\d{4}-\d{2}-\d{2}$/));
});

it("opens quick switcher wiki page hits in a backend-backed wiki page panel", async () => {
  vi.mocked(searchLoomGraph).mockResolvedValueOnce([
    {
      result_kind: "wiki_page",
      source_kind: "wiki_page",
      ref_id: "KWP-alpha",
      title: "GraphSearchAlpha Wiki Page",
      excerpt: "A compiled project wiki projection.",
      score: 4.3,
      metadata: { projection_id: "KWP-alpha", page_type: "concept" },
    },
  ]);
  vi.mocked(getLoomWikiProjection).mockClear();

  render(<App />);

  fireEvent.keyDown(window, { key: "p", ctrlKey: true });
  const quickSearch = await screen.findByTestId("quick-switcher.search");
  fireEvent.change(quickSearch, { target: { value: "GraphSearchAlpha" } });

  const wikiHit = await screen.findByTestId("quick-switcher.result.wiki_page.kwp-alpha");
  expect(wikiHit).toHaveTextContent("Open wiki page");
  fireEvent.click(wikiHit);

  await waitFor(() => expect(screen.getByTestId("loom-wiki-page-panel")).toHaveTextContent("GraphSearchAlpha Wiki Page"));
  expect(screen.getByTestId("loom-wiki-page-panel")).toHaveTextContent("GraphSearchAlpha wiki rendered content.");
  expect(screen.getByTestId("pane-pane-a")).toHaveAttribute("data-pane-type", "loom-wiki-page");
  await waitFor(() => expect(getLoomWikiProjection).toHaveBeenCalledWith("w1", "KWP-alpha"));
});

it("opens quick switcher document-backed Loom hits in the source document tab", async () => {
  vi.mocked(searchLoomGraph).mockResolvedValueOnce([
    {
      result_kind: "loom_block",
      source_kind: "loom_block",
      ref_id: "block-doc-alpha",
      title: "GraphSearchAlpha document note",
      excerpt: "A Loom block linked to a source document.",
      block: { document_id: "doc-alpha" },
      score: 4.4,
      metadata: { content_type: "note" },
    },
  ]);
  vi.mocked(getLoomBlock).mockClear();

  render(<App />);

  fireEvent.keyDown(window, { key: "p", ctrlKey: true });
  const quickSearch = await screen.findByTestId("quick-switcher.search");
  fireEvent.change(quickSearch, { target: { value: "GraphSearchAlpha" } });

  const documentHit = await screen.findByTestId("quick-switcher.result.loom_block.block-doc-alpha");
  expect(documentHit).toHaveTextContent("Open source document");
  fireEvent.click(documentHit);

  expect(await screen.findByTestId("document-view.doc-alpha")).toBeInTheDocument();
  expect(screen.getByTestId("pane-pane-a")).toHaveAttribute("data-pane-type", "workspace");
  expect(screen.getByTestId("pane-pane-a")).toHaveAttribute("data-pane-active-document-id", "doc-alpha");
  expect(screen.getByTestId("pane-pane-a.document-tab.doc-alpha")).toHaveAttribute("data-active", "true");
  expect(getLoomBlock).not.toHaveBeenCalled();
});

it("opens quick switcher standalone document hits in the source document tab", async () => {
  vi.mocked(searchLoomGraph).mockResolvedValueOnce([
    {
      result_kind: "knowledge_entity",
      source_kind: "document",
      ref_id: "KRD-00000000000000000000000000000001",
      title: "GraphSearchAlpha standalone document",
      excerpt: "A standalone rich document.",
      score: 4.1,
      metadata: { rich_document_id: "KRD-00000000000000000000000000000001" },
    },
  ]);
  vi.mocked(getLoomBlock).mockClear();

  render(<App />);

  fireEvent.keyDown(window, { key: "p", ctrlKey: true });
  const quickSearch = await screen.findByTestId("quick-switcher.search");
  fireEvent.change(quickSearch, { target: { value: "GraphSearchAlpha" } });

  const documentHit = await screen.findByTestId(
    "quick-switcher.result.document.krd-00000000000000000000000000000001",
  );
  expect(documentHit).toHaveTextContent("Open document");
  fireEvent.click(documentHit);

  expect(await screen.findByTestId("document-view.KRD-00000000000000000000000000000001")).toBeInTheDocument();
  expect(screen.getByTestId("pane-pane-a")).toHaveAttribute("data-pane-type", "workspace");
  expect(screen.getByTestId("pane-pane-a")).toHaveAttribute(
    "data-pane-active-document-id",
    "KRD-00000000000000000000000000000001",
  );
  expect(getLoomBlock).not.toHaveBeenCalled();
});

it.each([
  ["file", "file-alpha", "GraphSearchAlpha source file", "Open file"],
  ["tag_hub", "tag-alpha", "GraphSearchAlpha tag hub", "Open tag hub"],
] as const)("opens quick switcher %s hits in the Loom block panel", async (sourceKind, refId, title, targetLabel) => {
  vi.mocked(searchLoomGraph).mockResolvedValueOnce([
    {
      result_kind: "loom_block",
      source_kind: sourceKind,
      ref_id: refId,
      title,
      excerpt: `${title} excerpt.`,
      score: 4.1,
      metadata: { content_type: sourceKind },
    },
  ]);
  vi.mocked(getLoomBlock).mockClear();

  render(<App />);

  fireEvent.keyDown(window, { key: "p", ctrlKey: true });
  const quickSearch = await screen.findByTestId("quick-switcher.search");
  fireEvent.change(quickSearch, { target: { value: "GraphSearchAlpha" } });

  const hit = await screen.findByTestId(`quick-switcher.result.${sourceKind}.${refId}`);
  expect(hit).toHaveTextContent(targetLabel);
  fireEvent.click(hit);

  await waitFor(() => expect(screen.getByTestId("loom-block-panel")).toHaveTextContent(title));
  expect(screen.getByTestId("pane-pane-a")).toHaveAttribute("data-pane-type", "loom-block");
  await waitFor(() => expect(getLoomBlock).toHaveBeenCalledWith("w1", refId));
});

it("opens quick switcher symbol hits in a backend-backed code symbol panel", async () => {
  vi.mocked(searchLoomGraph).mockResolvedValueOnce([
    {
      result_kind: "knowledge_entity",
      source_kind: "symbol",
      ref_id: "KEN-symbol-alpha",
      title: "GraphSearchAlpha",
      excerpt: "rust:src/backend/graph_search.rs#GraphSearchAlpha",
      score: 3.6,
      metadata: {
        authority_table: "knowledge_entities",
        entity_key: "rust:src/backend/graph_search.rs#GraphSearchAlpha",
      },
    },
  ]);
  vi.mocked(getCodeSymbol).mockClear();

  render(<App />);

  fireEvent.keyDown(window, { key: "p", ctrlKey: true });
  const quickSearch = await screen.findByTestId("quick-switcher.search");
  fireEvent.change(quickSearch, { target: { value: "GraphSearchAlpha" } });

  const symbolHit = await screen.findByTestId("quick-switcher.result.symbol.ken-symbol-alpha");
  expect(symbolHit).toHaveTextContent("Open code symbol");
  fireEvent.click(symbolHit);

  await waitFor(() => expect(screen.getByTestId("code-symbol-panel")).toHaveTextContent("GraphSearchAlpha"));
  expect(screen.getByTestId("code-symbol-panel")).toHaveTextContent("rust:src/backend/graph_search.rs#GraphSearchAlpha");
  expect(screen.getByTestId("pane-pane-a")).toHaveAttribute("data-pane-type", "code-symbol");
  await waitFor(() => expect(getCodeSymbol).toHaveBeenCalledWith("KEN-symbol-alpha"));
});

it("opens quick switcher work packet hits in Kernel DCC with the matching row focused", async () => {
  vi.mocked(searchLoomGraph).mockResolvedValueOnce([
    {
      result_kind: "knowledge_entity",
      source_kind: "work_packet",
      ref_id: "KEN-wp-app-backend",
      title: "WP-KERNEL-002",
      excerpt: "Work packet entity row.",
      score: 3.2,
      metadata: {
        authority_table: "knowledge_entities",
        entity_key: "wp:WP-KERNEL-002-GraphSearchAlpha",
      },
    },
  ]);

  render(<App />);

  fireEvent.keyDown(window, { key: "p", ctrlKey: true });
  const quickSearch = await screen.findByTestId("quick-switcher.search");
  fireEvent.change(quickSearch, { target: { value: "WP-KERNEL-002" } });

  const workPacketHit = await screen.findByTestId("quick-switcher.result.work_packet.ken-wp-app-backend");
  expect(workPacketHit).toHaveTextContent("Open Kernel DCC work packet");
  fireEvent.click(workPacketHit);

  expect(await screen.findByTestId("kernel-dcc-projection")).toBeInTheDocument();
  expect(screen.getByTestId("pane-pane-a")).toHaveAttribute("data-pane-type", "kernel-dcc");
  expect(await screen.findByTestId("dcc.work_selection.row.work-app-backend-123")).toHaveAttribute(
    "data-focused",
    "true",
  );
});

it("opens quick switcher microtask hits in Kernel DCC with the matching MT row focused", async () => {
  vi.mocked(searchLoomGraph).mockResolvedValueOnce([
    {
      result_kind: "knowledge_entity",
      source_kind: "micro_task",
      ref_id: "KEN-mt-app-backend",
      title: "MT-DCC-APP",
      excerpt: "Microtask entity row.",
      score: 3.1,
      metadata: {
        authority_table: "knowledge_entities",
        entity_key: "mt:MT-DCC-APP-GraphSearchAlpha",
        work_packet: "WP-KERNEL-002-GraphSearchAlpha",
      },
    },
  ]);

  render(<App />);

  fireEvent.keyDown(window, { key: "p", ctrlKey: true });
  const quickSearch = await screen.findByTestId("quick-switcher.search");
  fireEvent.change(quickSearch, { target: { value: "MT-DCC-APP" } });

  const microTaskHit = await screen.findByTestId("quick-switcher.result.micro_task.ken-mt-app-backend");
  expect(microTaskHit).toHaveTextContent("Open Kernel DCC microtask");
  fireEvent.click(microTaskHit);

  expect(await screen.findByTestId("kernel-dcc-projection")).toBeInTheDocument();
  expect(screen.getByTestId("pane-pane-a")).toHaveAttribute("data-pane-type", "kernel-dcc");
  expect(await screen.findByTestId("dcc.work_selection.row.work-app-backend-123")).toHaveAttribute(
    "data-focused",
    "true",
  );
});

it("keeps Ctrl+Shift+P routed to the app command palette instead of quick open", async () => {
  render(<App />);

  fireEvent.keyDown(window, { key: "p", ctrlKey: true, shiftKey: true });

  expect(await screen.findByRole("dialog", { name: "App commands" })).toBeInTheDocument();
  expect(screen.getByTestId("command-palette-action-hs-usermanual-palette-open")).toBeInTheDocument();
  expect(screen.queryByTestId("quick-switcher.search")).not.toBeInTheDocument();
});

it("routes app command palette editor catalog actions to the active rich document", async () => {
  vi.mocked(searchLoomGraph).mockResolvedValueOnce([
    {
      result_kind: "knowledge_entity",
      source_kind: "document",
      ref_id: "KRD-00000000000000000000000000000001",
      title: "Unified registry rich document",
      excerpt: "A rich document with editor command routing.",
      score: 4.1,
      metadata: { rich_document_id: "KRD-00000000000000000000000000000001" },
    },
  ]);
  render(<App />);

  fireEvent.keyDown(window, { key: "p", ctrlKey: true });
  const quickSearch = await screen.findByTestId("quick-switcher.search");
  fireEvent.change(quickSearch, { target: { value: "Unified registry" } });
  fireEvent.click(
    await screen.findByTestId("quick-switcher.result.document.krd-00000000000000000000000000000001"),
  );
  const documentView = await screen.findByTestId("document-view.KRD-00000000000000000000000000000001");
  expect(documentView).toBeInTheDocument();

  fireEvent.keyDown(documentView, { key: "p", ctrlKey: true, shiftKey: true });
  expect(await screen.findByRole("dialog", { name: "App commands" })).toBeInTheDocument();
  expect(screen.getByTestId("command-palette-action-hs-usermanual-palette-open")).toBeInTheDocument();
  expect(screen.queryByTestId("quick-switcher.search")).not.toBeInTheDocument();
  fireEvent.click(await screen.findByTestId("command-palette-action-hs-editor-command-format-bold"));

  expect(
    await screen.findByTestId(
      "document-view.KRD-00000000000000000000000000000001.command-palette-request",
    ),
  ).toHaveAttribute("data-query", "Bold");
  expect(screen.getByTestId("pane-pane-a")).toHaveAttribute(
    "data-pane-active-document-id",
    "KRD-00000000000000000000000000000001",
  );
  expect(screen.getByTestId("pane-pane-a")).toHaveAttribute("data-pane-type", "workspace");
});

it("routes app command palette editor component actions to the active rich document", async () => {
  vi.mocked(searchLoomGraph).mockResolvedValueOnce([
    {
      result_kind: "knowledge_entity",
      source_kind: "document",
      ref_id: "KRD-00000000000000000000000000000001",
      title: "Unified component command document",
      excerpt: "A rich document with component command routing.",
      score: 4.1,
      metadata: { rich_document_id: "KRD-00000000000000000000000000000001" },
    },
  ]);
  render(<App />);

  fireEvent.keyDown(window, { key: "p", ctrlKey: true });
  const quickSearch = await screen.findByTestId("quick-switcher.search");
  fireEvent.change(quickSearch, { target: { value: "Unified component" } });
  fireEvent.click(
    await screen.findByTestId("quick-switcher.result.document.krd-00000000000000000000000000000001"),
  );
  const documentView = await screen.findByTestId("document-view.KRD-00000000000000000000000000000001");

  fireEvent.keyDown(documentView, { key: "p", ctrlKey: true, shiftKey: true });
  expect(await screen.findByRole("dialog", { name: "App commands" })).toBeInTheDocument();
  expect(screen.getByTestId("command-palette-action-hs-editor-command-find-open")).toHaveTextContent(
    "Find in document",
  );
  expect(screen.getByTestId("command-palette-action-hs-editor-command-replace-open")).toHaveTextContent(
    "Find and replace",
  );
  expect(
    screen.getByTestId("command-palette-action-hs-editor-command-export-html-self-contained"),
  ).toHaveTextContent("Export: HTML");
  expect(screen.getByTestId("command-palette-action-hs-editor-command-editor-save")).toHaveTextContent(
    "Save document",
  );

  fireEvent.click(screen.getByTestId("command-palette-action-hs-editor-command-find-open"));

  expect(
    await screen.findByTestId(
      "document-view.KRD-00000000000000000000000000000001.command-palette-request",
    ),
  ).toHaveAttribute("data-query", "Find in document");
  expect(screen.getByTestId("pane-pane-a")).toHaveAttribute("data-pane-type", "workspace");
});

it("routes app command palette editor requests only to the active duplicate rich-document pane", async () => {
  const duplicateKrd = "KRD-00000000000000000000000000000002";
  vi.mocked(getWorkbenchLayoutState).mockResolvedValueOnce({
    workspace_id: "w1",
    layout_state: {
      schema_id: "hsk.workbench_layout_state@1",
      activePaneId: "pane-b",
      activeModule: "MAIN",
      splitWeights: { vertical: 0.5, horizontal: 0.55 },
      drawers: { project: true, file: true, bottom: true },
      panes: [
        {
          id: "pane-a",
          module: "MAIN",
          activeTab: "workspace",
          tabs: ["workspace"],
          locked: false,
          projectRef: "w1",
          activeDocumentId: duplicateKrd,
          activeCanvasId: null,
          openDocuments: [{ documentId: duplicateKrd }],
        },
        {
          id: "pane-b",
          module: "MAIN",
          activeTab: "workspace",
          tabs: ["workspace"],
          locked: false,
          projectRef: "w1",
          activeDocumentId: duplicateKrd,
          activeCanvasId: null,
          openDocuments: [{ documentId: duplicateKrd }],
        },
        {
          id: "pane-c",
          module: "INGEST",
          activeTab: "flight-recorder",
          tabs: ["flight-recorder"],
          locked: false,
          projectRef: "w1",
          activeDocumentId: null,
          activeCanvasId: null,
          openDocuments: [],
        },
        {
          id: "pane-d",
          module: "STAGE",
          activeTab: "fonts",
          tabs: ["fonts"],
          locked: false,
          projectRef: "w1",
          activeDocumentId: null,
          activeCanvasId: null,
          openDocuments: [],
        },
      ],
    },
    updated_at: "2026-06-15T00:00:00Z",
    event_ledger_event_id: "EVT-workbench-layout-duplicate-krd",
  });
  render(<App />);

  await waitFor(() => expect(screen.getByTestId("pane-pane-b")).toHaveAttribute("data-pane-active", "true"));
  expect(screen.getAllByTestId(`document-view.${duplicateKrd}`)).toHaveLength(2);

  fireEvent.keyDown(window, { key: "p", ctrlKey: true, shiftKey: true });
  fireEvent.click(await screen.findByTestId("command-palette-action-hs-editor-command-format-bold"));

  const request = screen.getByTestId(`document-view.${duplicateKrd}.command-palette-request`);
  expect(screen.getAllByTestId(`document-view.${duplicateKrd}.command-palette-request`)).toHaveLength(1);
  expect(request).toHaveAttribute("data-pane-id", "pane-b");
  expect(screen.getByTestId("pane-pane-b")).toContainElement(
    request,
  );
});

it("orders selected quick switcher hits before backend order when reopened", async () => {
  vi.mocked(searchLoomGraph).mockClear();
  const betaRecent = {
    workspace_id: "w1",
    hit_key: "user_manual_page:recent-beta",
    source_kind: "user_manual_page" as const,
    ref_id: "recent-beta",
    result_kind: "user_manual_page" as const,
    title: "Recent Beta",
    excerpt: "Persisted after selection.",
    metadata: { page_slug: "recent-beta" },
    selected_count: 1,
    selected_at: "2026-06-15T00:00:00Z",
    event_ledger_event_id: "EVT-quick-switcher-beta",
  };
  let durableRecentAvailable = false;
  vi.mocked(listQuickSwitcherRecents).mockImplementation(async () =>
    durableRecentAvailable ? [betaRecent] : [],
  );
  vi.mocked(searchLoomGraph).mockResolvedValue([
    {
      result_kind: "user_manual_page",
      source_kind: "user_manual_page",
      ref_id: "recent-alpha",
      title: "Recent Alpha",
      excerpt: "Backend returned this first.",
      score: 5,
      metadata: { page_slug: "recent-alpha" },
    },
    {
      result_kind: "user_manual_page",
      source_kind: "user_manual_page",
      ref_id: "recent-beta",
      title: "Recent Beta",
      excerpt: "Selected once, then promoted locally.",
      score: 4,
      metadata: { page_slug: "recent-beta" },
    },
  ]);

  render(<App />);

  fireEvent.keyDown(window, { key: "p", ctrlKey: true });
  const quickSearch = await screen.findByTestId("quick-switcher.search");
  await waitFor(() => expect(listQuickSwitcherRecents).toHaveBeenCalledWith("w1", 20));
  fireEvent.change(quickSearch, { target: { value: "Recent" } });
  expect(await screen.findByTestId("quick-switcher.result.user_manual_page.recent-alpha")).toHaveAttribute(
    "aria-selected",
    "true",
  );
  const betaHit = await screen.findByTestId("quick-switcher.result.user_manual_page.recent-beta");
  fireEvent.keyDown(quickSearch, { key: "ArrowDown" });
  expect(betaHit).toHaveAttribute("aria-selected", "true");
  fireEvent.keyDown(quickSearch, { key: "Enter" });
  await waitFor(() => expect(screen.queryByTestId("quick-switcher.search")).not.toBeInTheDocument());
  durableRecentAvailable = true;

  fireEvent.keyDown(window, { key: "p", ctrlKey: true });
  await waitFor(() => expect(listQuickSwitcherRecents).toHaveBeenCalledWith("w1", 20));
  const reopenedSearch = await screen.findByTestId("quick-switcher.search");
  fireEvent.change(reopenedSearch, { target: { value: "Recent" } });
  await screen.findByTestId("quick-switcher.result.user_manual_page.recent-alpha");
  await screen.findByTestId("quick-switcher.result.user_manual_page.recent-beta");

  await waitFor(() => {
    const rows = Array.from(
      screen.getByRole("listbox", { name: "Quick switcher results" }).querySelectorAll("[data-ref-id]"),
    );
    expect(rows.map((row) => row.getAttribute("data-ref-id"))).toEqual(["recent-beta", "recent-alpha"]);
  });
});

it("clears quick switcher query and results on every open before the next backend search", async () => {
  vi.mocked(searchLoomGraph).mockClear();
  vi.mocked(searchLoomGraph).mockResolvedValue([
    {
      result_kind: "user_manual_page",
      source_kind: "user_manual_page",
      ref_id: "recent-alpha",
      title: "Recent Alpha",
      excerpt: "Backend result from the previous query.",
      score: 5,
      metadata: { page_slug: "recent-alpha" },
    },
  ]);

  render(<App />);

  fireEvent.keyDown(window, { key: "p", ctrlKey: true });
  const firstSearch = await screen.findByTestId("quick-switcher.search");
  fireEvent.change(firstSearch, { target: { value: "Recent" } });
  await screen.findByTestId("quick-switcher.result.user_manual_page.recent-alpha");
  fireEvent.keyDown(firstSearch, { key: "Escape" });
  await waitFor(() => expect(screen.queryByTestId("quick-switcher.search")).not.toBeInTheDocument());

  fireEvent.keyDown(window, { key: "p", ctrlKey: true });
  const secondSearch = await screen.findByTestId("quick-switcher.search");
  expect(secondSearch).toHaveValue("");
  expect(screen.queryByTestId("quick-switcher.result.user_manual_page.recent-alpha")).not.toBeInTheDocument();
});

it("clears local quick switcher ranking when durable recents load empty", async () => {
  vi.mocked(searchLoomGraph).mockClear();
  vi.mocked(listQuickSwitcherRecents).mockResolvedValue([]);
  vi.mocked(searchLoomGraph).mockResolvedValue([
    {
      result_kind: "user_manual_page",
      source_kind: "user_manual_page",
      ref_id: "recent-alpha",
      title: "Recent Alpha",
      excerpt: "Backend returned this first.",
      score: 5,
      metadata: { page_slug: "recent-alpha" },
    },
    {
      result_kind: "user_manual_page",
      source_kind: "user_manual_page",
      ref_id: "recent-beta",
      title: "Recent Beta",
      excerpt: "Local selection should be cleared by empty durable state.",
      score: 4,
      metadata: { page_slug: "recent-beta" },
    },
  ]);

  render(<App />);

  fireEvent.keyDown(window, { key: "p", ctrlKey: true });
  const firstSearch = await screen.findByTestId("quick-switcher.search");
  await waitFor(() => expect(listQuickSwitcherRecents).toHaveBeenCalledTimes(1));
  fireEvent.change(firstSearch, { target: { value: "Recent" } });
  await screen.findByTestId("quick-switcher.result.user_manual_page.recent-alpha");
  const betaHit = await screen.findByTestId("quick-switcher.result.user_manual_page.recent-beta");
  fireEvent.keyDown(firstSearch, { key: "ArrowDown" });
  expect(betaHit).toHaveAttribute("aria-selected", "true");
  fireEvent.keyDown(firstSearch, { key: "Enter" });
  await waitFor(() => expect(screen.queryByTestId("quick-switcher.search")).not.toBeInTheDocument());
  vi.mocked(listQuickSwitcherRecents).mockClear();

  fireEvent.keyDown(window, { key: "p", ctrlKey: true });
  await waitFor(() => expect(listQuickSwitcherRecents).toHaveBeenCalledWith("w1", 20));
  const secondSearch = await screen.findByTestId("quick-switcher.search");
  fireEvent.change(secondSearch, { target: { value: "Recent" } });
  await screen.findByTestId("quick-switcher.result.user_manual_page.recent-alpha");
  await screen.findByTestId("quick-switcher.result.user_manual_page.recent-beta");

  const rows = Array.from(
    screen.getByRole("listbox", { name: "Quick switcher results" }).querySelectorAll("[data-ref-id]"),
  );
  expect(rows.map((row) => row.getAttribute("data-ref-id"))).toEqual(["recent-alpha", "recent-beta"]);
});

it("does not promote quick switcher recents locally when durable recording fails", async () => {
  vi.mocked(searchLoomGraph).mockClear();
  vi.mocked(listQuickSwitcherRecents)
    .mockResolvedValueOnce([])
    .mockRejectedValue(new Error("EventLedger recents unavailable"));
  vi.mocked(recordQuickSwitcherRecent).mockRejectedValue(new Error("EventLedger write failed"));
  vi.mocked(searchLoomGraph).mockResolvedValue([
    {
      result_kind: "user_manual_page",
      source_kind: "user_manual_page",
      ref_id: "recent-alpha",
      title: "Recent Alpha",
      excerpt: "Backend returned this first.",
      score: 5,
      metadata: { page_slug: "recent-alpha" },
    },
    {
      result_kind: "user_manual_page",
      source_kind: "user_manual_page",
      ref_id: "recent-beta",
      title: "Recent Beta",
      excerpt: "Record failure must not promote this locally.",
      score: 4,
      metadata: { page_slug: "recent-beta" },
    },
  ]);

  render(<App />);

  fireEvent.keyDown(window, { key: "p", ctrlKey: true });
  const firstSearch = await screen.findByTestId("quick-switcher.search");
  fireEvent.change(firstSearch, { target: { value: "Recent" } });
  await screen.findByTestId("quick-switcher.result.user_manual_page.recent-alpha");
  const betaHit = await screen.findByTestId("quick-switcher.result.user_manual_page.recent-beta");
  fireEvent.keyDown(firstSearch, { key: "ArrowDown" });
  expect(betaHit).toHaveAttribute("aria-selected", "true");
  fireEvent.keyDown(firstSearch, { key: "Enter" });
  await waitFor(() => expect(recordQuickSwitcherRecent).toHaveBeenCalled());
  await waitFor(() => expect(screen.queryByTestId("quick-switcher.search")).not.toBeInTheDocument());

  fireEvent.keyDown(window, { key: "p", ctrlKey: true });
  const secondSearch = await screen.findByTestId("quick-switcher.search");
  await waitFor(() => expect(listQuickSwitcherRecents).toHaveBeenCalledTimes(2));
  fireEvent.change(secondSearch, { target: { value: "Recent" } });
  await screen.findByTestId("quick-switcher.result.user_manual_page.recent-alpha");
  await screen.findByTestId("quick-switcher.result.user_manual_page.recent-beta");

  await waitFor(() => expect(screen.getByTestId("quick-switcher.status")).toHaveTextContent("durable recents unavailable"));
  const rows = Array.from(
    screen.getByRole("listbox", { name: "Quick switcher results" }).querySelectorAll("[data-ref-id]"),
  );
  expect(rows.map((row) => row.getAttribute("data-ref-id"))).toEqual(["recent-alpha", "recent-beta"]);
});

it("does not reuse a stale successful recent when durable recents fail on reopen", async () => {
  vi.mocked(searchLoomGraph).mockClear();
  vi.mocked(listQuickSwitcherRecents)
    .mockResolvedValueOnce([])
    .mockRejectedValueOnce(new Error("EventLedger recents unavailable"));
  vi.mocked(searchLoomGraph).mockResolvedValue([
    {
      result_kind: "user_manual_page",
      source_kind: "user_manual_page",
      ref_id: "recent-alpha",
      title: "Recent Alpha",
      excerpt: "Backend returned this first.",
      score: 5,
      metadata: { page_slug: "recent-alpha" },
    },
    {
      result_kind: "user_manual_page",
      source_kind: "user_manual_page",
      ref_id: "recent-beta",
      title: "Recent Beta",
      excerpt: "Previous selection must not leak into the next component instance.",
      score: 4,
      metadata: { page_slug: "recent-beta" },
    },
  ]);

  render(<App />);

  fireEvent.keyDown(window, { key: "p", ctrlKey: true });
  const firstSearch = await screen.findByTestId("quick-switcher.search");
  fireEvent.change(firstSearch, { target: { value: "Recent" } });
  await screen.findByTestId("quick-switcher.result.user_manual_page.recent-alpha");
  const betaHit = await screen.findByTestId("quick-switcher.result.user_manual_page.recent-beta");
  fireEvent.keyDown(firstSearch, { key: "ArrowDown" });
  expect(betaHit).toHaveAttribute("aria-selected", "true");
  fireEvent.keyDown(firstSearch, { key: "Enter" });
  await waitFor(() => expect(recordQuickSwitcherRecent).toHaveBeenCalled());
  await waitFor(() => expect(screen.queryByTestId("quick-switcher.search")).not.toBeInTheDocument());

  fireEvent.keyDown(window, { key: "p", ctrlKey: true });
  const secondSearch = await screen.findByTestId("quick-switcher.search");
  await waitFor(() => expect(listQuickSwitcherRecents).toHaveBeenCalledTimes(2));
  fireEvent.change(secondSearch, { target: { value: "Recent" } });
  await screen.findByTestId("quick-switcher.result.user_manual_page.recent-alpha");
  await screen.findByTestId("quick-switcher.result.user_manual_page.recent-beta");

  await waitFor(() => expect(screen.getByTestId("quick-switcher.status")).toHaveTextContent("durable recents unavailable"));
  const rows = Array.from(
    screen.getByRole("listbox", { name: "Quick switcher results" }).querySelectorAll("[data-ref-id]"),
  );
  expect(rows.map((row) => row.getAttribute("data-ref-id"))).toEqual(["recent-alpha", "recent-beta"]);
});

it("orders quick switcher hits from durable recents loaded on open", async () => {
  vi.mocked(searchLoomGraph).mockClear();
  vi.mocked(listQuickSwitcherRecents).mockResolvedValue([
    {
      workspace_id: "w1",
      hit_key: "user_manual_page:recent-beta",
      source_kind: "user_manual_page",
      ref_id: "recent-beta",
      result_kind: "user_manual_page",
      title: "Recent Beta",
      excerpt: "Persisted from a previous session.",
      metadata: { page_slug: "recent-beta" },
      selected_count: 2,
      selected_at: "2026-06-15T00:00:00Z",
      event_ledger_event_id: "EVT-quick-switcher-beta",
    },
  ]);
  vi.mocked(searchLoomGraph).mockResolvedValueOnce([
    {
      result_kind: "user_manual_page",
      source_kind: "user_manual_page",
      ref_id: "recent-alpha",
      title: "Recent Alpha",
      excerpt: "Backend returned this first.",
      score: 5,
      metadata: { page_slug: "recent-alpha" },
    },
    {
      result_kind: "user_manual_page",
      source_kind: "user_manual_page",
      ref_id: "recent-beta",
      title: "Recent Beta",
      excerpt: "Durable recent should promote this.",
      score: 4,
      metadata: { page_slug: "recent-beta" },
    },
  ]);

  render(<App />);

  fireEvent.keyDown(window, { key: "p", ctrlKey: true });
  const quickSearch = await screen.findByTestId("quick-switcher.search");
  await waitFor(() => expect(listQuickSwitcherRecents).toHaveBeenCalledWith("w1", 20));
  fireEvent.change(quickSearch, { target: { value: "Recent" } });
  await screen.findByTestId("quick-switcher.result.user_manual_page.recent-alpha");
  await screen.findByTestId("quick-switcher.result.user_manual_page.recent-beta");

  await waitFor(() => {
    const rows = Array.from(
      screen.getByRole("listbox", { name: "Quick switcher results" }).querySelectorAll("[data-ref-id]"),
    );
    expect(rows.map((row) => row.getAttribute("data-ref-id"))).toEqual(["recent-beta", "recent-alpha"]);
  });
});

it("records selected quick switcher hits through the durable recents API", async () => {
  vi.mocked(searchLoomGraph).mockClear();
  vi.mocked(searchLoomGraph).mockResolvedValueOnce([
    {
      result_kind: "user_manual_page",
      source_kind: "user_manual_page",
      ref_id: "recent-beta",
      title: "Recent Beta",
      excerpt: "Selected and persisted.",
      score: 4,
      metadata: { page_slug: "recent-beta" },
    },
  ]);

  render(<App />);

  fireEvent.keyDown(window, { key: "p", ctrlKey: true });
  const quickSearch = await screen.findByTestId("quick-switcher.search");
  fireEvent.change(quickSearch, { target: { value: "Recent" } });
  fireEvent.click(await screen.findByTestId("quick-switcher.result.user_manual_page.recent-beta"));

  await waitFor(() =>
    expect(recordQuickSwitcherRecent).toHaveBeenCalledWith(
      "w1",
      expect.objectContaining({
        result_kind: "user_manual_page",
        source_kind: "user_manual_page",
        ref_id: "recent-beta",
        title: "Recent Beta",
        excerpt: "Selected and persisted.",
        metadata: { page_slug: "recent-beta" },
      }),
    ),
  );
});

it("clears stale quick switcher results immediately when the query changes", async () => {
  vi.mocked(searchLoomGraph).mockClear();
  vi.mocked(searchLoomGraph)
    .mockResolvedValueOnce([
      {
        result_kind: "user_manual_page",
        source_kind: "user_manual_page",
        ref_id: "stale-alpha",
        title: "Stale Alpha",
        excerpt: "Old query result.",
        score: 5,
        metadata: { page_slug: "stale-alpha" },
      },
    ])
    .mockResolvedValueOnce([
      {
        result_kind: "user_manual_page",
        source_kind: "user_manual_page",
        ref_id: "fresh-alpha",
        title: "Fresh Alpha",
        excerpt: "New query result.",
        score: 5,
        metadata: { page_slug: "fresh-alpha" },
      },
    ]);

  render(<App />);

  fireEvent.keyDown(window, { key: "p", ctrlKey: true });
  const quickSearch = await screen.findByTestId("quick-switcher.search");
  fireEvent.change(quickSearch, { target: { value: "Stale" } });
  expect(await screen.findByTestId("quick-switcher.result.user_manual_page.stale-alpha")).toBeInTheDocument();

  fireEvent.change(quickSearch, { target: { value: "Fresh" } });

  expect(screen.queryByTestId("quick-switcher.result.user_manual_page.stale-alpha")).not.toBeInTheDocument();
  expect(await screen.findByTestId("quick-switcher.result.user_manual_page.fresh-alpha")).toBeInTheDocument();
});

it("ignores an in-flight stale quick switcher response after the query changes", async () => {
  vi.useFakeTimers();

  const staleResult = [
    {
      result_kind: "user_manual_page" as const,
      source_kind: "user_manual_page" as const,
      ref_id: "stale-alpha",
      title: "Stale Alpha",
      excerpt: "Old in-flight query result.",
      score: 5,
      metadata: { page_slug: "stale-alpha" },
    },
  ];
  const freshResult = [
    {
      result_kind: "user_manual_page" as const,
      source_kind: "user_manual_page" as const,
      ref_id: "fresh-alpha",
      title: "Fresh Alpha",
      excerpt: "New query result.",
      score: 5,
      metadata: { page_slug: "fresh-alpha" },
    },
  ];
  let resolveStale: (hits: typeof staleResult) => void = () => {};
  let resolveFresh: (hits: typeof freshResult) => void = () => {};
  vi.mocked(searchLoomGraph).mockClear();
  vi.mocked(searchLoomGraph)
    .mockReturnValueOnce(new Promise((resolve) => {
      resolveStale = resolve;
    }))
    .mockReturnValueOnce(new Promise((resolve) => {
      resolveFresh = resolve;
    }));

  try {
    render(<App />);
    await act(async () => {
      await Promise.resolve();
      await Promise.resolve();
      await Promise.resolve();
    });
    expect(screen.getByTestId("main-window")).toHaveAttribute("data-active-project-id", "w1");

    fireEvent.keyDown(window, { key: "p", ctrlKey: true });
    const quickSearch = screen.getByTestId("quick-switcher.search");
    fireEvent.change(quickSearch, { target: { value: "Stale" } });
    await act(async () => {
      vi.advanceTimersByTime(150);
    });
    expect(searchLoomGraph).toHaveBeenCalledWith(expect.any(String), expect.objectContaining({ q: "Stale" }));

    fireEvent.change(quickSearch, { target: { value: "Fresh" } });
    resolveStale(staleResult);
    await act(async () => {
      await Promise.resolve();
    });

    expect(screen.queryByTestId("quick-switcher.result.user_manual_page.stale-alpha")).not.toBeInTheDocument();

    await act(async () => {
      vi.advanceTimersByTime(150);
    });
    expect(searchLoomGraph).toHaveBeenCalledWith(expect.any(String), expect.objectContaining({ q: "Fresh" }));
    resolveFresh(freshResult);
    await act(async () => {
      await Promise.resolve();
    });

    expect(screen.getByTestId("quick-switcher.result.user_manual_page.fresh-alpha")).toBeInTheDocument();
  } finally {
    vi.useRealTimers();
  }
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

it("lets document editor actions wrap inside split editor panes", () => {
  const css = readFileSync("src/App.css", "utf8");

  expect(css).toMatch(/\.document-editor__header\s*\{[^}]*flex-wrap:\s*wrap;/);
  expect(css).toMatch(/\.document-editor__actions\s*\{[^}]*flex-wrap:\s*wrap;/);
  expect(css).toMatch(/\.document-editor__actions button\s*\{[^}]*max-width:\s*100%;/);
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
