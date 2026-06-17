import { afterEach, describe, expect, it, vi } from "vitest";
import {
  acceptAtelierAiTagSuggestion,
  applyAtelierAiTagSuggestion,
  archiveAtelierDeletionTargets,
  getCodeSymbol,
  getCodeSymbolReferences,
  getCodeFileLens,
  getKernelDccProjection,
  getLoomBlock,
  getLoomWikiProjection,
  queryLoomView,
  getWorkbenchLayoutState,
  getWorkspaceSettingsState,
  getUserManualPage,
  getSourceControlDiff,
  getSourceControlStatus,
  importAtelierClipboardImage,
  listUserManualAccessPoints,
  listUserManualPages,
  listAtelierAiTagSuggestionsForCharacter,
  listAtelierFilesystemHealthFindings,
  listAtelierStealthWindows,
  listQuickSwitcherRecents,
  openDailyJournal,
  openAtelierIntakeBatch,
  previewAtelierDeletionImpact,
  recordAtelierAiTagSuggestion,
  recordAtelierUrlImageImport,
  recordQuickSwitcherRecent,
  rejectAtelierAiTagSuggestion,
  restoreAtelierDeletionTargets,
  runAtelierFilesystemHealthCheck,
  saveRichDocument,
  saveWorkbenchLayoutState,
  saveWorkspaceSettingsState,
  searchLoomGraph,
  searchUserManual,
  setLoomBlockPinOrder,
  stageSourceControlPaths,
  discardSourceControlPaths,
  commitSourceControl,
  createSourceControlBranch,
  getSourceControlBlame,
  getSourceControlLog,
  listSourceControlBranches,
  switchSourceControlBranch,
  lookupCodeSymbols,
  unstageSourceControlPaths,
  updateLoomBlock,
} from "./api";

describe("Kernel DCC API projection composition", () => {
  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it("composes session spawn tree projection only from backend runtime records", async () => {
    const fetchMock = vi
      .fn()
      .mockResolvedValueOnce(
        jsonResponse({
          schema_id: "hsk.kernel.dcc_mvp_runtime_surface@1",
          surface_id: "dcc-api-composition-test",
          folded_stub_id: "WP-1-Dev-Command-Center-MVP-v1",
          panels: [],
          work_items: [],
          worktrees: [],
          sessions: [
            {
              session_id: "session-child",
              role: "CODER",
              model_id: "gpt-test",
              backend: "codex",
              worktree_id: "wt-runtime-backed",
              wp_id: "WP-KERNEL-002",
              mt_id: "MT-043",
              state: "ACTIVE",
            },
            {
              session_id: "session-root",
              role: "ORCHESTRATOR",
              model_id: "gpt-test",
              backend: "codex",
              worktree_id: "wt-runtime-backed",
              wp_id: "WP-KERNEL-002",
              mt_id: "MT-043",
              state: "ACTIVE",
            },
          ],
          session_spawn_runtime_records: [
            {
              session_id: "session-child",
              parent_session_id: "session-root",
              role_id: "CODER",
              spawn_mode: "OneShot",
              runtime_state: "Active",
              cascade_cancel_supported: false,
              announce_back_badges: [
                {
                  badge_id: "badge-from-runtime-child",
                  session_id: "session-child",
                  label: "announce-back pending",
                  mailbox_route: "role-mailbox://session-child/announce-back",
                },
              ],
              runtime_record_ref: "runtime://session-spawn/runtime-record-child",
              flight_recorder_ref: "FR-EVT-SESSION-SPAWN-runtime-child",
            },
            {
              session_id: "session-root",
              parent_session_id: null,
              role_id: "ORCHESTRATOR",
              spawn_mode: "SessionPersistent",
              runtime_state: "Active",
              cascade_cancel_supported: true,
              announce_back_badges: [],
              runtime_record_ref: "runtime://session-spawn/runtime-record-root",
              flight_recorder_ref: "FR-EVT-SESSION-SPAWN-runtime-root",
            },
          ],
          proposals: [],
          evidence: [],
          approval_previews: [],
          write_box_queue_rows: [],
          direct_edit_denials: [],
          promotion_previews: [],
          freshness_badges: [],
          stable_element_ids: [],
          catalog_action_refs: [],
          direct_authority_mutation_allowed: false,
          ungoverned_tool_execution_allowed: false,
          destructive_git_ops_require_same_turn_approval: true,
          flight_recorder_event_types: [],
          product_authority_refs: [],
          folded_source_refs: [],
        }),
      )
      .mockResolvedValueOnce(
        jsonResponse({
          schema_id: "hsk.kernel.session_spawn_tree_dcc_projection@1",
          tree_id: "dcc-api-composition-test.session-spawn-tree",
          panel_id: "session-spawn-tree",
          visible_fields: ["SpawnHierarchy"],
          nodes: [
            {
              session_id: "session-root",
              parent_session_id: null,
              role_id: "ORCHESTRATOR",
              depth: 0,
              child_count: 1,
              active_child_count: 1,
              spawn_mode: "SessionPersistent",
              runtime_state: "Active",
              cascade_cancel_available: true,
              announce_back_badges: [],
            },
            {
              session_id: "session-child",
              parent_session_id: "session-root",
              role_id: "CODER",
              depth: 1,
              child_count: 0,
              active_child_count: 0,
              spawn_mode: "OneShot",
              runtime_state: "Active",
              cascade_cancel_available: false,
              announce_back_badges: ["announce-back pending"],
            },
          ],
          max_depth: 1,
          cascade_cancel_session_ids: ["session-root"],
          announce_back_badge_count: 1,
          runtime_record_refs: [
            "runtime://session-spawn/runtime-record-child",
            "runtime://session-spawn/runtime-record-root",
          ],
          mutates_runtime_records: false,
        }),
      );
    vi.stubGlobal("fetch", fetchMock);

    const surface = await getKernelDccProjection();

    expect(surface.spawn_tree_projection?.runtime_record_refs).toEqual([
      "runtime://session-spawn/runtime-record-child",
      "runtime://session-spawn/runtime-record-root",
    ]);
    expect(fetchMock).toHaveBeenCalledTimes(2);
    expect(fetchMock).toHaveBeenNthCalledWith(
      2,
      "http://127.0.0.1:37501/api/kernel/session_spawn_tree_dcc_projection",
      expect.objectContaining({
        method: "POST",
        body: expect.any(String),
      }),
    );
    const postedBody = JSON.parse(fetchMock.mock.calls[1][1].body as string);
    expect(postedBody.runtime_records).toEqual([
      expect.objectContaining({
        session_id: "session-child",
        parent_session_id: "session-root",
        spawn_mode: "OneShot",
        cascade_cancel_supported: false,
        runtime_record_ref: "runtime://session-spawn/runtime-record-child",
        flight_recorder_ref: "FR-EVT-SESSION-SPAWN-runtime-child",
        announce_back_badges: [
          expect.objectContaining({
            badge_id: "badge-from-runtime-child",
            label: "announce-back pending",
          }),
        ],
      }),
      expect.objectContaining({
        session_id: "session-root",
        parent_session_id: null,
        spawn_mode: "SessionPersistent",
        cascade_cancel_supported: true,
        runtime_record_ref: "runtime://session-spawn/runtime-record-root",
        flight_recorder_ref: "FR-EVT-SESSION-SPAWN-runtime-root",
      }),
    ]);
    expect(JSON.stringify(postedBody)).not.toContain("runtime://session-spawn/session-child");
  });

  it("does not synthesize flat spawn records when backend runtime records are absent", async () => {
    const fetchMock = vi.fn().mockResolvedValueOnce(
      jsonResponse({
        schema_id: "hsk.kernel.dcc_mvp_runtime_surface@1",
        surface_id: "dcc-api-composition-test",
        folded_stub_id: "WP-1-Dev-Command-Center-MVP-v1",
        panels: [],
        work_items: [],
        worktrees: [],
        sessions: [
          {
            session_id: "session-without-runtime-record",
            role: "CODER",
            model_id: "gpt-test",
            backend: "codex",
            worktree_id: "wt-runtime-backed",
            wp_id: "WP-KERNEL-002",
            mt_id: "MT-043",
            state: "ACTIVE",
          },
        ],
        proposals: [],
        evidence: [],
        approval_previews: [],
        write_box_queue_rows: [],
        direct_edit_denials: [],
        promotion_previews: [],
        freshness_badges: [],
        stable_element_ids: [],
        catalog_action_refs: [],
        direct_authority_mutation_allowed: false,
        ungoverned_tool_execution_allowed: false,
        destructive_git_ops_require_same_turn_approval: true,
        flight_recorder_event_types: [],
        product_authority_refs: [],
        folded_source_refs: [],
      }),
    );
    vi.stubGlobal("fetch", fetchMock);

    const surface = await getKernelDccProjection();

    expect(fetchMock).toHaveBeenCalledTimes(1);
    expect(surface.spawn_tree_projection).toBeNull();
  });
});

describe("Atelier API actor context", () => {
  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it("sends actor context when listing stealth windows", async () => {
    const fetchMock = vi.fn().mockResolvedValueOnce(jsonResponse([]));
    vi.stubGlobal("fetch", fetchMock);

    await listAtelierStealthWindows({ actor_id: "operator-alpha" });

    expect(fetchMock).toHaveBeenCalledWith(
      "http://127.0.0.1:37501/atelier/stealth/windows",
      expect.objectContaining({
        method: "GET",
        headers: expect.objectContaining({
          "x-hsk-actor-id": "operator-alpha",
        }),
      }),
    );
  });

  it("exposes AI tag suggestion lifecycle routes with explicit actor attribution", async () => {
    const fetchMock = vi
      .fn()
      .mockResolvedValueOnce(jsonResponse({ suggestion_id: "suggestion-1", status: "proposed" }))
      .mockResolvedValueOnce(jsonResponse([{ suggestion_id: "suggestion-1", status: "proposed" }]))
      .mockResolvedValueOnce(jsonResponse({ suggestion_id: "suggestion-1", status: "accepted" }))
      .mockResolvedValueOnce(jsonResponse({ suggestion_id: "suggestion-2", status: "rejected" }))
      .mockResolvedValueOnce(jsonResponse({ suggestion_id: "suggestion-1", status: "applied" }));
    vi.stubGlobal("fetch", fetchMock);

    await recordAtelierAiTagSuggestion({
      character_internal_id: "character-1",
      asset_id: null,
      tag_text: "cinematic lighting",
      confidence: 0.91,
      model_receipt_ref: "receipt://atelier/model/1",
      tool_receipt_ref: "receipt://atelier/tool/1",
      suggested_by: "model-worker",
    });
    await listAtelierAiTagSuggestionsForCharacter("character-1");
    await acceptAtelierAiTagSuggestion(
      "suggestion-1",
      { actor_id: "operator-alpha" },
      { reason: "matches image" },
    );
    await rejectAtelierAiTagSuggestion(
      "suggestion-2",
      { actor_id: "operator-beta" },
      { reason: "does not match image" },
    );
    await applyAtelierAiTagSuggestion("suggestion-1", { actor_id: "operator-alpha" });

    expect(fetchMock).toHaveBeenNthCalledWith(
      1,
      "http://127.0.0.1:37501/atelier/ai-tag-suggestions",
      expect.objectContaining({
        method: "POST",
        body: expect.stringContaining("cinematic lighting"),
      }),
    );
    expect(fetchMock).toHaveBeenNthCalledWith(
      2,
      "http://127.0.0.1:37501/atelier/ai-tag-suggestions/characters/character-1",
      expect.objectContaining({ method: "GET" }),
    );
    expect(fetchMock).toHaveBeenNthCalledWith(
      3,
      "http://127.0.0.1:37501/atelier/ai-tag-suggestions/suggestion-1/accept",
      expect.objectContaining({
        method: "POST",
        headers: expect.objectContaining({
          "x-hsk-actor-id": "operator-alpha",
        }),
        body: expect.stringContaining("matches image"),
      }),
    );
    expect(fetchMock).toHaveBeenNthCalledWith(
      4,
      "http://127.0.0.1:37501/atelier/ai-tag-suggestions/suggestion-2/reject",
      expect.objectContaining({
        method: "POST",
        headers: expect.objectContaining({
          "x-hsk-actor-id": "operator-beta",
        }),
        body: expect.stringContaining("does not match image"),
      }),
    );
    expect(fetchMock).toHaveBeenNthCalledWith(
      5,
      "http://127.0.0.1:37501/atelier/ai-tag-suggestions/suggestion-1/apply",
      expect.objectContaining({
        method: "POST",
        headers: expect.objectContaining({
          "x-hsk-actor-id": "operator-alpha",
        }),
      }),
    );
  });

  it("posts the full Atelier intake batch contract", async () => {
    const fetchMock = vi.fn().mockResolvedValueOnce(
      jsonResponse({
        batch_id: "batch-1",
        idempotency_key: "intake-contract-1",
        source_label: "Linked source",
        source_ref: "source://atelier/intake/source-1",
        mode: "sourcing_run",
        profile_mode: "character_linked",
        target_character_id: "00000000-0000-0000-0000-000000000001",
        target_sheet_version_id: "00000000-0000-0000-0000-000000000002",
        target_collection_id: "00000000-0000-0000-0000-000000000003",
        status: "open",
        resume_cursor: "cursor://atelier/intake/intake-contract-1",
        resumed_at_utc: null,
        created_at_utc: "2026-06-08T00:00:00Z",
      }),
    );
    vi.stubGlobal("fetch", fetchMock);

    await openAtelierIntakeBatch({
      idempotency_key: "intake-contract-1",
      source_label: "Linked source",
      source_ref: "source://atelier/intake/source-1",
      mode: "sourcing_run",
      profile_mode: "character_linked",
      target_character_id: "00000000-0000-0000-0000-000000000001",
      target_sheet_version_id: "00000000-0000-0000-0000-000000000002",
      target_collection_id: "00000000-0000-0000-0000-000000000003",
      resume_cursor: "cursor://atelier/intake/intake-contract-1",
    });

    expect(fetchMock).toHaveBeenCalledWith(
      "http://127.0.0.1:37501/atelier/intake/batches",
      expect.objectContaining({
        method: "POST",
        body: expect.any(String),
      }),
    );
    expect(JSON.parse(fetchMock.mock.calls[0][1].body as string)).toEqual({
      idempotency_key: "intake-contract-1",
      source_label: "Linked source",
      source_ref: "source://atelier/intake/source-1",
      mode: "sourcing_run",
      profile_mode: "character_linked",
      target_character_id: "00000000-0000-0000-0000-000000000001",
      target_sheet_version_id: "00000000-0000-0000-0000-000000000002",
      target_collection_id: "00000000-0000-0000-0000-000000000003",
      resume_cursor: "cursor://atelier/intake/intake-contract-1",
    });
  });

  it("exposes filesystem health diagnostics without implicit repair", async () => {
    const fetchMock = vi
      .fn()
      .mockResolvedValueOnce(
        jsonResponse({
          check: {
            check_id: "check-1",
            requested_by: "operator-alpha",
            scope_label: "gallery",
            summary: { auto_resync: false, auto_delete: false },
            created_at_utc: "2026-06-08T00:00:00Z",
          },
          findings: [{ finding_id: "finding-1", finding_kind: "missing_thumbnail" }],
        }),
      )
      .mockResolvedValueOnce(jsonResponse([{ finding_id: "finding-1" }]));
    vi.stubGlobal("fetch", fetchMock);

    await runAtelierFilesystemHealthCheck(
      { actor_id: "operator-alpha" },
      { scope_label: "gallery" },
    );
    await listAtelierFilesystemHealthFindings("check-1");

    expect(fetchMock).toHaveBeenNthCalledWith(
      1,
      "http://127.0.0.1:37501/atelier/filesystem-health/checks",
      expect.objectContaining({
        method: "POST",
        headers: expect.objectContaining({
          "x-hsk-actor-id": "operator-alpha",
        }),
        body: expect.stringContaining("gallery"),
      }),
    );
    expect(fetchMock).toHaveBeenNthCalledWith(
      2,
      "http://127.0.0.1:37501/atelier/filesystem-health/checks/check-1/findings",
      expect.objectContaining({ method: "GET" }),
    );
  });

  it("exposes recoverable deletion preview, archive, and restore routes", async () => {
    const fetchMock = vi
      .fn()
      .mockResolvedValueOnce(jsonResponse({ target_count: 2, would_archive_count: 2 }))
      .mockResolvedValueOnce(jsonResponse({ operation: "archive_deletion_targets" }))
      .mockResolvedValueOnce(jsonResponse({ operation: "restore_deletion_targets" }));
    vi.stubGlobal("fetch", fetchMock);

    const targets = [
      { target_type: "media_asset" as const, target_id: "asset-1" },
      { target_type: "sheet_version" as const, target_id: "sheet-1" },
    ];
    await previewAtelierDeletionImpact(
      { actor_id: "operator-alpha" },
      { targets, reason: "preview cleanup" },
    );
    await archiveAtelierDeletionTargets(
      { actor_id: "operator-alpha" },
      { targets, reason: "archive cleanup" },
    );
    await restoreAtelierDeletionTargets(
      { actor_id: "operator-beta" },
      { targets, reason: "restore cleanup" },
    );

    expect(fetchMock).toHaveBeenNthCalledWith(
      1,
      "http://127.0.0.1:37501/atelier/deletion/impact-preview",
      expect.objectContaining({
        method: "POST",
        headers: expect.objectContaining({
          "x-hsk-actor-id": "operator-alpha",
        }),
        body: expect.stringContaining("preview cleanup"),
      }),
    );
    expect(fetchMock).toHaveBeenNthCalledWith(
      2,
      "http://127.0.0.1:37501/atelier/deletion/archive",
      expect.objectContaining({
        method: "POST",
        headers: expect.objectContaining({
          "x-hsk-actor-id": "operator-alpha",
        }),
        body: expect.stringContaining("archive cleanup"),
      }),
    );
    expect(fetchMock).toHaveBeenNthCalledWith(
      3,
      "http://127.0.0.1:37501/atelier/deletion/restore",
      expect.objectContaining({
        method: "POST",
        headers: expect.objectContaining({
          "x-hsk-actor-id": "operator-beta",
        }),
        body: expect.stringContaining("restore cleanup"),
      }),
    );
  });

  it("exposes clipboard and URL image import routes with actor attribution", async () => {
    const fetchMock = vi
      .fn()
      .mockResolvedValueOnce(jsonResponse({ source_kind: "clipboard", status: "materialized" }))
      .mockResolvedValueOnce(jsonResponse({ source_kind: "url", status: "queued" }));
    vi.stubGlobal("fetch", fetchMock);

    await importAtelierClipboardImage(
      { actor_id: "operator-import" },
      {
        idempotency_key: "clipboard-1",
        mime: "image/png",
        content_hash: "sha256:abc",
        byte_len: 10,
        artifact_ref: "artifact://.handshake/artifacts/L1/00000000-0000-0000-0000-000000000001/payload",
        source_application: "system-clipboard",
      },
    );
    await recordAtelierUrlImageImport(
      { actor_id: "operator-import" },
      {
        idempotency_key: "url-1",
        source_url: "https://example.com/image.png",
        expected_mime: "image/png",
        source_label: "example",
        capability_profile_id: "MediaDownloader",
        capability_grant_ref: "capgrant://media_downloader/MediaDownloader/evidence-1",
      },
    );

    expect(fetchMock).toHaveBeenNthCalledWith(
      1,
      "http://127.0.0.1:37501/atelier/image-import/clipboard",
      expect.objectContaining({
        method: "POST",
        headers: expect.objectContaining({
          "x-hsk-actor-id": "operator-import",
        }),
        body: expect.stringContaining("system-clipboard"),
      }),
    );
    expect(fetchMock).toHaveBeenNthCalledWith(
      2,
      "http://127.0.0.1:37501/atelier/image-import/url",
      expect.objectContaining({
        method: "POST",
        headers: expect.objectContaining({
          "x-hsk-actor-id": "operator-import",
        }),
        body: expect.stringContaining("capgrant://media_downloader/MediaDownloader/evidence-1"),
      }),
    );
  });
});

describe("UserManual API client", () => {
  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it("fetches the manual page index from the canonical /api/usermanual route", async () => {
    const fetchMock = vi.fn().mockResolvedValueOnce(
      jsonResponse({
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
      }),
    );
    vi.stubGlobal("fetch", fetchMock);

    const result = await listUserManualPages({ audience: "model", limit: 25 });

    expect(result.pages[0].slug).toBe("manual-toc");
    expect(fetchMock).toHaveBeenCalledWith(
      "http://127.0.0.1:37501/api/usermanual/pages?audience=model&limit=25",
      expect.objectContaining({ method: "GET" }),
    );
  });

  it("fetches one manual page by slug with encoded path parameters", async () => {
    const fetchMock = vi.fn().mockResolvedValueOnce(
      jsonResponse({
        page: { slug: "manual-toc", title: "Manual TOC", manual_version: "2.0.0" },
        sections: [{ section_id: "sec-1", title: "Navigation", body_md: "Start here." }],
        anchors: [{ anchor_id: "a-1", anchor_kind: "http_route", anchor_value: "/usermanual/pages" }],
        bootstrap_receipt_event_id: "evt-user-manual-opened",
        bootstrap_identity_used: true,
      }),
    );
    vi.stubGlobal("fetch", fetchMock);

    const result = await getUserManualPage("manual/toc");

    expect(result.bootstrap_identity_used).toBe(true);
    expect(fetchMock).toHaveBeenCalledWith(
      "http://127.0.0.1:37501/api/usermanual/pages/manual%2Ftoc",
      expect.objectContaining({ method: "GET" }),
    );
  });

  it("searches the manual through the backend search route", async () => {
    const fetchMock = vi.fn().mockResolvedValueOnce(
      jsonResponse({
        query: "recovery",
        count: 1,
        results: [
          {
            result_kind: "section",
            result_ref: "sec-recovery",
            page_slug: "state-recovery-guide",
            title: "State recovery",
            excerpt: "Recover failed startup state.",
          },
        ],
      }),
    );
    vi.stubGlobal("fetch", fetchMock);

    const result = await searchUserManual("recovery", 10);

    expect(result.results[0].page_slug).toBe("state-recovery-guide");
    expect(fetchMock).toHaveBeenCalledWith(
      "http://127.0.0.1:37501/api/usermanual/search?q=recovery&limit=10",
      expect.objectContaining({ method: "GET" }),
    );
  });

  it("lists backend-declared in-app access points with stable element ids", async () => {
    const fetchMock = vi.fn().mockResolvedValueOnce(
      jsonResponse({
        count: 2,
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
      }),
    );
    vi.stubGlobal("fetch", fetchMock);

    const result = await listUserManualAccessPoints();

    expect(result.access_points.map((point) => point.stable_element_id)).toEqual([
      "hs-usermanual-diagnostics-tab",
      "hs-usermanual-palette-search",
    ]);
    expect(fetchMock).toHaveBeenCalledWith(
      "http://127.0.0.1:37501/api/usermanual/access-points",
      expect.objectContaining({ method: "GET" }),
    );
  });
});

describe("Loom graph search API client", () => {
  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it("queries graph search with the nine quick switcher source kinds", async () => {
    const fetchMock = vi.fn().mockResolvedValueOnce(
      jsonResponse([
        {
          result_kind: "loom_block",
          source_kind: "loom_block",
          ref_id: "block-alpha",
          title: "GraphSearchAlpha Loom note",
          excerpt: "GraphSearchAlpha joins notes to code and manuals.",
          score: 4.2,
          metadata: { content_type: "note" },
        },
      ]),
    );
    vi.stubGlobal("fetch", fetchMock);

    const result = await searchLoomGraph("w1", {
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
    });

    expect(result[0].source_kind).toBe("loom_block");
    expect(fetchMock).toHaveBeenCalledWith(
      "http://127.0.0.1:37501/workspaces/w1/loom/graph-search?q=GraphSearchAlpha&source_kinds=loom_block%2Cfile%2Ctag_hub%2Cdocument%2Csymbol%2Cwork_packet%2Cmicro_task%2Cuser_manual_page%2Cwiki_page&limit=25",
      expect.objectContaining({ method: "GET" }),
    );
  });

  it("passes exact workspace-search operators as backend query parameters", async () => {
    const fetchMock = vi.fn().mockResolvedValueOnce(jsonResponse([]));
    vi.stubGlobal("fetch", fetchMock);

    await searchLoomGraph("w1", {
      q: "Alpha.*Beta",
      limit: 500,
      offset: 500,
      sourceKinds: ["document"],
      tagIds: ["tag-1"],
      caseSensitive: true,
      wholeWord: true,
      isRegex: true,
      path: "src/app",
    });

    expect(fetchMock).toHaveBeenCalledWith(
      "http://127.0.0.1:37501/workspaces/w1/loom/graph-search?q=Alpha.*Beta&source_kinds=document&tag_ids=tag-1&case_sensitive=true&whole_word=true&regex=true&path=src%2Fapp&limit=500&offset=500",
      expect.objectContaining({ method: "GET" }),
    );
  });

  it("loads one Loom wiki projection by workspace and projection id", async () => {
    const fetchMock = vi.fn().mockResolvedValueOnce(
      jsonResponse({
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
      }),
    );
    vi.stubGlobal("fetch", fetchMock);

    const result = await getLoomWikiProjection("w1", "KWP-alpha");

    expect(result.projection_id).toBe("KWP-alpha");
    expect(result.staleness_verdict).toEqual({ status: "fresh" });
    expect(fetchMock).toHaveBeenCalledWith(
      "http://127.0.0.1:37501/workspaces/w1/loom/wiki/KWP-alpha",
      expect.objectContaining({ method: "GET" }),
    );
  });
});

describe("Quick switcher recents API client", () => {
  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it("lists durable quick switcher recents for one workspace", async () => {
    const fetchMock = vi.fn().mockResolvedValueOnce(
      jsonResponse([
        {
          workspace_id: "w1",
          hit_key: "user_manual_page:recent-beta",
          source_kind: "user_manual_page",
          ref_id: "recent-beta",
          result_kind: "user_manual_page",
          title: "Recent Beta",
          excerpt: "Persisted from EventLedger-backed recents.",
          metadata: { page_slug: "recent-beta" },
          selected_count: 2,
          selected_at: "2026-06-15T00:00:00Z",
          event_ledger_event_id: "EVT-quick-switcher-beta",
        },
      ]),
    );
    vi.stubGlobal("fetch", fetchMock);

    const result = await listQuickSwitcherRecents("w1", 20);

    expect(result[0].hit_key).toBe("user_manual_page:recent-beta");
    expect(fetchMock).toHaveBeenCalledWith(
      "http://127.0.0.1:37501/workspaces/w1/loom/quick-switcher/recents?limit=20",
      expect.objectContaining({ method: "GET" }),
    );
  });

  it("records selected quick switcher hits through the backend recents endpoint", async () => {
    const fetchMock = vi.fn().mockResolvedValueOnce(
      jsonResponse({
        workspace_id: "w1",
        hit_key: "user_manual_page:recent-beta",
        source_kind: "user_manual_page",
        ref_id: "recent-beta",
        result_kind: "user_manual_page",
        title: "Recent Beta",
        excerpt: "Selected from QuickSwitcher.",
        metadata: { page_slug: "recent-beta" },
        selected_count: 1,
        selected_at: "2026-06-15T00:00:00Z",
        event_ledger_event_id: "EVT-quick-switcher-beta",
      }),
    );
    vi.stubGlobal("fetch", fetchMock);

    const result = await recordQuickSwitcherRecent("w1", {
      result_kind: "user_manual_page",
      source_kind: "user_manual_page",
      ref_id: "recent-beta",
      title: "Recent Beta",
      excerpt: "Selected from QuickSwitcher.",
      metadata: { page_slug: "recent-beta" },
    });

    expect(result.event_ledger_event_id).toBe("EVT-quick-switcher-beta");
    expect(fetchMock).toHaveBeenCalledWith(
      "http://127.0.0.1:37501/workspaces/w1/loom/quick-switcher/recents",
      expect.objectContaining({
        method: "POST",
        body: JSON.stringify({
          result_kind: "user_manual_page",
          source_kind: "user_manual_page",
          ref_id: "recent-beta",
          title: "Recent Beta",
          excerpt: "Selected from QuickSwitcher.",
          metadata: { page_slug: "recent-beta" },
        }),
      }),
    );
  });
});

describe("Rich document API client", () => {
  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it("sends CRDT promotion metadata when saving a rich document", async () => {
    const fetchMock = vi.fn().mockResolvedValueOnce(
      jsonResponse({
        document: {
          rich_document_id: "KRD-api-save",
          workspace_id: "w1",
          document_id: null,
          title: "Runbook",
          schema_version: "rich_document_v1",
          doc_version: 2,
          content_json: { type: "doc", content: [] },
          content_sha256: "0".repeat(64),
          crdt_document_id: "KCRDT-api-save",
          crdt_snapshot_id: "snap-api-save",
          promotion_receipt_event_id: "EVT-api-save",
          projection_refs: [],
          project_ref: null,
          folder_ref: null,
          authority_label: "promoted",
          owner_actor_kind: null,
          owner_actor_id: null,
          created_at: "2026-06-15T00:00:00Z",
          updated_at: "2026-06-15T00:00:00Z",
        },
        save_receipt_event_id: "EVT-api-save-receipt",
        backlinks_persisted: 0,
        backlinks_skipped_reason: null,
      }),
    );
    vi.stubGlobal("fetch", fetchMock);

    const content = { type: "doc", content: [] };
    await saveRichDocument(
      "KRD-api-save",
      1,
      content,
      {
        actor_id: "operator",
        kernel_task_run_id: "KTR-api-save",
        session_run_id: "SR-api-save",
      },
      {
        crdt_document_id: "KCRDT-api-save",
        crdt_snapshot_id: "snap-api-save",
        promotion_receipt_event_id: "EVT-api-save",
      },
    );

    expect(fetchMock).toHaveBeenCalledWith(
      "http://127.0.0.1:37501/knowledge/documents/KRD-api-save/save",
      expect.objectContaining({
        method: "PUT",
        body: JSON.stringify({
          expected_version: 1,
          content_json: content,
          crdt_document_id: "KCRDT-api-save",
          crdt_snapshot_id: "snap-api-save",
          promotion_receipt_event_id: "EVT-api-save",
        }),
      }),
    );
  });
});

describe("Workbench layout state API client", () => {
  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it("loads the durable workbench layout state for one workspace", async () => {
    const fetchMock = vi.fn().mockResolvedValueOnce(
      jsonResponse({
        workspace_id: "w1",
        layout_state: {
          schema_id: "hsk.workbench_layout_state@1",
          activePaneId: "pane-b",
          activeModule: "CKC",
          splitWeights: { vertical: 0.62, horizontal: 0.44 },
          drawers: { project: true, file: false, bottom: true },
          panes: [
            { id: "pane-a", module: "MAIN", activeTab: "workspace", tabs: ["workspace"], locked: false, projectRef: "w1" },
            { id: "pane-b", module: "CKC", activeTab: "kernel-dcc", tabs: ["kernel-dcc", "workspace"], locked: false, projectRef: "w1" },
            { id: "pane-c", module: "INGEST", activeTab: "flight-recorder", tabs: ["flight-recorder"], locked: false, projectRef: "w1" },
            { id: "pane-d", module: "STAGE", activeTab: "fonts", tabs: ["fonts"], locked: false, projectRef: "w1" },
          ],
        },
        updated_at: "2026-06-15T00:00:00Z",
        event_ledger_event_id: "EVT-workbench-layout",
      }),
    );
    vi.stubGlobal("fetch", fetchMock);

    const result = await getWorkbenchLayoutState("w1");

    expect(result.layout_state?.schema_id).toBe("hsk.workbench_layout_state@1");
    expect(result.layout_state?.activePaneId).toBe("pane-b");
    expect(result.event_ledger_event_id).toBe("EVT-workbench-layout");
    expect(fetchMock).toHaveBeenCalledWith(
      "http://127.0.0.1:37501/workspaces/w1/workbench/layout",
      expect.objectContaining({ method: "GET" }),
    );
  });

  it("saves durable workbench layout state through the backend layout endpoint", async () => {
    const layoutState = {
      schema_id: "hsk.workbench_layout_state@1",
      activePaneId: "pane-c",
      activeModule: "INGEST",
      splitWeights: { vertical: 0.58, horizontal: 0.51 },
      drawers: { project: true, file: false, bottom: true },
      panes: [
        { id: "pane-a", module: "MAIN", activeTab: "workspace", tabs: ["workspace"], locked: false, projectRef: "w1" },
        { id: "pane-b", module: "CKC", activeTab: "kernel-dcc", tabs: ["kernel-dcc"], locked: false, projectRef: "w1" },
        { id: "pane-c", module: "INGEST", activeTab: "flight-recorder", tabs: ["flight-recorder"], locked: false, projectRef: "w1" },
        { id: "pane-d", module: "STAGE", activeTab: "fonts", tabs: ["fonts"], locked: false, projectRef: "w1" },
      ],
    };
    const fetchMock = vi.fn().mockResolvedValueOnce(
      jsonResponse({
        workspace_id: "w1",
        layout_state: layoutState,
        updated_at: "2026-06-15T00:05:00Z",
        event_ledger_event_id: "EVT-workbench-layout-save",
      }),
    );
    vi.stubGlobal("fetch", fetchMock);

    const result = await saveWorkbenchLayoutState("w1", layoutState);

    expect(result.layout_state).toEqual(layoutState);
    expect(fetchMock).toHaveBeenCalledWith(
      "http://127.0.0.1:37501/workspaces/w1/workbench/layout",
      expect.objectContaining({
        method: "PUT",
        body: JSON.stringify({ layout_state: layoutState }),
      }),
    );
  });
});

describe("Workspace settings state API client", () => {
  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it("loads durable workspace settings state with its EventLedger receipt", async () => {
    const settingsState = {
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
    };
    const fetchMock = vi.fn().mockResolvedValueOnce(
      jsonResponse({
        workspace_id: "w1",
        settings_state: settingsState,
        updated_at: "2026-06-15T00:10:00Z",
        event_ledger_event_id: "EVT-workspace-settings",
      }),
    );
    vi.stubGlobal("fetch", fetchMock);

    const result = await getWorkspaceSettingsState("w1");

    expect(result.settings_state).toEqual(settingsState);
    expect(result.event_ledger_event_id).toBe("EVT-workspace-settings");
    expect(fetchMock).toHaveBeenCalledWith(
      "http://127.0.0.1:37501/workspaces/w1/settings",
      expect.objectContaining({ method: "GET" }),
    );
  });

  it("saves durable workspace settings state through the backend settings endpoint", async () => {
    const settingsState = {
      schema_id: "hsk.workspace_settings_state@1",
      theme: "light",
      custom_theme_tokens: {},
      keybindings: {
        "app.quick_switcher.open": "Mod-p",
        "app.command_palette.open": "Alt-c",
      },
      settings: {
        view_mode: "NSFW",
        swarm_board_default_open: false,
      },
    };
    const fetchMock = vi.fn().mockResolvedValueOnce(
      jsonResponse({
        workspace_id: "w1",
        settings_state: settingsState,
        updated_at: "2026-06-15T00:11:00Z",
        event_ledger_event_id: "EVT-workspace-settings-save",
      }),
    );
    vi.stubGlobal("fetch", fetchMock);

    const result = await saveWorkspaceSettingsState("w1", settingsState);

    expect(result.settings_state).toEqual(settingsState);
    expect(fetchMock).toHaveBeenCalledWith(
      "http://127.0.0.1:37501/workspaces/w1/settings",
      expect.objectContaining({
        method: "PUT",
        body: JSON.stringify({ settings_state: settingsState }),
      }),
    );
  });
});

describe("Loom block API client", () => {
  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it("fetches one Loom block by workspace and block id from the backend route", async () => {
    const fetchMock = vi.fn().mockResolvedValueOnce(
      jsonResponse({
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
      }),
    );
    vi.stubGlobal("fetch", fetchMock);

    const result = await getLoomBlock("w1", "block-alpha");

    expect(result.title).toBe("GraphSearchAlpha standalone Loom note");
    expect(fetchMock).toHaveBeenCalledWith(
      "http://127.0.0.1:37501/workspaces/w1/loom/blocks/block-alpha",
      expect.objectContaining({ method: "GET" }),
    );
  });

  it("patches Loom block properties through the backend update route", async () => {
    const fetchMock = vi.fn().mockResolvedValueOnce(
      jsonResponse({
        block_id: "block-alpha",
        workspace_id: "w1",
        content_type: "note",
        document_id: null,
        asset_id: null,
        title: "Updated Alpha",
        original_filename: null,
        content_hash: "hash-alpha",
        pinned: true,
        favorite: true,
        pin_order: null,
        journal_date: null,
        created_at: "2026-06-15T00:00:00Z",
        updated_at: "2026-06-16T00:00:00Z",
        imported_at: null,
        derived: {
          full_text_index: "GraphSearchAlpha joins notes to code and manuals.",
          backlink_count: 1,
          mention_count: 2,
          tag_count: 3,
          preview_status: "ready",
        },
      }),
    );
    vi.stubGlobal("fetch", fetchMock);

    const result = await updateLoomBlock("w1", "block-alpha", {
      title: "Updated Alpha",
      pinned: true,
      favorite: true,
    });

    expect(result.title).toBe("Updated Alpha");
    expect(result.pinned).toBe(true);
    expect(fetchMock).toHaveBeenCalledWith(
      "http://127.0.0.1:37501/workspaces/w1/loom/blocks/block-alpha",
      expect.objectContaining({
        method: "PATCH",
        body: JSON.stringify({
          title: "Updated Alpha",
          pinned: true,
          favorite: true,
        }),
      }),
    );
  });

  it("queries the reusable Loom pins view for bookmark surfaces", async () => {
    const fetchMock = vi.fn().mockResolvedValueOnce(
      jsonResponse({
        view_type: "pins",
        blocks: [
          {
            block_id: "block-alpha",
            workspace_id: "w1",
            content_type: "note",
            document_id: null,
            asset_id: null,
            title: "Pinned Alpha",
            original_filename: null,
            content_hash: "hash-alpha",
            pinned: true,
            favorite: false,
            pin_order: 0,
            journal_date: null,
            created_at: "2026-06-15T00:00:00Z",
            updated_at: "2026-06-16T00:00:00Z",
            imported_at: null,
            derived: {
              full_text_index: "Pinned alpha text",
              backlink_count: 1,
              mention_count: 2,
              tag_count: 3,
              preview_status: "ready",
            },
          },
        ],
      }),
    );
    vi.stubGlobal("fetch", fetchMock);

    const result = await queryLoomView("w1", "pins", {
      contentType: "note",
      limit: 50,
      offset: 5,
      tagIds: ["tag-alpha"],
    });

    expect(result.view_type).toBe("pins");
    if (result.view_type !== "pins") throw new Error("expected pins view response");
    expect(result.blocks[0].block_id).toBe("block-alpha");
    expect(fetchMock).toHaveBeenCalledWith(
      "http://127.0.0.1:37501/workspaces/w1/loom/views/pins?content_type=note&tag_ids=tag-alpha&limit=50&offset=5",
      expect.objectContaining({ method: "GET" }),
    );
  });

  it("clears Loom pin order through the dedicated pin-order route", async () => {
    const fetchMock = vi.fn().mockResolvedValueOnce(
      jsonResponse({
        block_id: "block-alpha",
        workspace_id: "w1",
        content_type: "note",
        document_id: null,
        asset_id: null,
        title: "Pinned Alpha",
        original_filename: null,
        content_hash: "hash-alpha",
        pinned: true,
        favorite: false,
        pin_order: null,
        journal_date: null,
        created_at: "2026-06-15T00:00:00Z",
        updated_at: "2026-06-16T00:00:00Z",
        imported_at: null,
        derived: {
          full_text_index: "Pinned alpha text",
          backlink_count: 1,
          mention_count: 2,
          tag_count: 3,
          preview_status: "ready",
        },
      }),
    );
    vi.stubGlobal("fetch", fetchMock);

    const result = await setLoomBlockPinOrder("w1", "block-alpha", null);

    expect(result.pin_order).toBeNull();
    expect(fetchMock).toHaveBeenCalledWith(
      "http://127.0.0.1:37501/workspaces/w1/loom/blocks/block-alpha/pin-order",
      expect.objectContaining({
        method: "PUT",
        body: JSON.stringify({ pin_order: null }),
      }),
    );
  });

  it("opens a daily journal block with a PUT to the date-addressed backend route", async () => {
    const fetchMock = vi.fn().mockResolvedValueOnce(
      jsonResponse({
        block_id: "journal-2026-06-16",
        workspace_id: "w1",
        content_type: "journal",
        document_id: null,
        asset_id: null,
        title: "Daily Note 2026-06-16",
        original_filename: null,
        content_hash: null,
        pinned: false,
        favorite: false,
        pin_order: null,
        journal_date: "2026-06-16",
        created_at: "2026-06-16T00:00:00Z",
        updated_at: "2026-06-16T00:00:00Z",
        imported_at: null,
        derived: {
          full_text_index: "# Daily Note 2026-06-16\n\n",
          backlink_count: 0,
          mention_count: 0,
          tag_count: 0,
          preview_status: "none",
        },
      }),
    );
    vi.stubGlobal("fetch", fetchMock);

    const result = await openDailyJournal("w1", "2026-06-16");

    expect(result.content_type).toBe("journal");
    expect(result.journal_date).toBe("2026-06-16");
    expect(fetchMock).toHaveBeenCalledWith(
      "http://127.0.0.1:37501/workspaces/w1/loom/journals/2026-06-16",
      expect.objectContaining({ method: "PUT" }),
    );
  });
});

describe("Source control API client", () => {
  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it("uses the typed local git backend routes for status, diff, stage, discard, and commit", async () => {
    const repoPath = "D:\\Projects\\Handshake Repo";
    const statusQuery = new URLSearchParams({ repo_path: repoPath }).toString();
    const diffQuery = new URLSearchParams({
      repo_path: repoPath,
      path: "src/main.rs",
      scope: "worktree",
    }).toString();
    const fetchMock = vi
      .fn()
      .mockResolvedValueOnce(
        jsonResponse({
          repo_root: repoPath,
          branch: "main",
          entries: [{ path: "src/main.rs", index: null, worktree: "modified" }],
        }),
      )
      .mockResolvedValueOnce(
        jsonResponse({
          path: "src/main.rs",
          scope: "worktree",
          patch: "@@ -1 +1\n+changed\n",
        }),
      )
      .mockResolvedValueOnce(jsonResponse({ operation: "stage", paths: ["src/main.rs"] }))
      .mockResolvedValueOnce(jsonResponse({ operation: "discard", paths: ["scratch"] }))
      .mockResolvedValueOnce(
        jsonResponse({
          id: "a".repeat(40),
          message: "source control commit",
        }),
      );
    vi.stubGlobal("fetch", fetchMock);

    const status = await getSourceControlStatus(repoPath);
    const diff = await getSourceControlDiff(repoPath, "src/main.rs", "worktree");
    const stage = await stageSourceControlPaths(repoPath, ["src/main.rs"]);
    const discard = await discardSourceControlPaths(repoPath, ["scratch"], true);
    const commit = await commitSourceControl(repoPath, "source control commit");

    expect(status.entries[0].worktree).toBe("modified");
    expect(diff.patch).toContain("+changed");
    expect(stage.operation).toBe("stage");
    expect(discard.operation).toBe("discard");
    expect(commit.id).toHaveLength(40);
    expect(fetchMock).toHaveBeenNthCalledWith(
      1,
      `http://127.0.0.1:37501/source-control/status?${statusQuery}`,
      expect.objectContaining({ method: "GET" }),
    );
    expect(fetchMock).toHaveBeenNthCalledWith(
      2,
      `http://127.0.0.1:37501/source-control/diff?${diffQuery}`,
      expect.objectContaining({ method: "GET" }),
    );
    expect(fetchMock).toHaveBeenNthCalledWith(
      3,
      "http://127.0.0.1:37501/source-control/stage",
      expect.objectContaining({
        method: "POST",
        body: JSON.stringify({ repo_path: repoPath, paths: ["src/main.rs"] }),
      }),
    );
    expect(fetchMock).toHaveBeenNthCalledWith(
      4,
      "http://127.0.0.1:37501/source-control/discard",
      expect.objectContaining({
        method: "POST",
        body: JSON.stringify({ repo_path: repoPath, paths: ["scratch"], confirmed: true }),
      }),
    );
    expect(fetchMock).toHaveBeenNthCalledWith(
      5,
      "http://127.0.0.1:37501/source-control/commit",
      expect.objectContaining({
        method: "POST",
        body: JSON.stringify({ repo_path: repoPath, message: "source control commit" }),
      }),
    );
  });

  it("uses the typed local git backend routes for unstage, branches, log, and blame", async () => {
    const repoPath = "D:\\Projects\\Handshake Repo";
    const repoQuery = new URLSearchParams({ repo_path: repoPath }).toString();
    const logQuery = new URLSearchParams({ repo_path: repoPath, limit: "5" }).toString();
    const blameQuery = new URLSearchParams({
      repo_path: repoPath,
      path: "src/main.rs",
    }).toString();
    const fetchMock = vi
      .fn()
      .mockResolvedValueOnce(jsonResponse({ operation: "unstage", paths: ["src/main.rs"] }))
      .mockResolvedValueOnce(
        jsonResponse([
          { name: "main", current: true, commit_id: "a".repeat(40) },
          { name: "feature/source-control", current: false, commit_id: "b".repeat(40) },
        ]),
      )
      .mockResolvedValueOnce(
        jsonResponse({ operation: "create_branch", paths: ["feature/source-control"] }),
      )
      .mockResolvedValueOnce(
        jsonResponse({ operation: "switch_branch", paths: ["main"] }),
      )
      .mockResolvedValueOnce(
        jsonResponse({
          entries: [{ id: "a".repeat(40), author: "Tester", timestamp: 1_700_000_000, message: "init" }],
        }),
      )
      .mockResolvedValueOnce(
        jsonResponse({
          path: "src/main.rs",
          lines: [{ line_number: 1, commit_id: "a".repeat(40), author: "Tester", content: "fn main() {}" }],
        }),
      );
    vi.stubGlobal("fetch", fetchMock);

    const unstage = await unstageSourceControlPaths(repoPath, ["src/main.rs"]);
    const branches = await listSourceControlBranches(repoPath);
    const created = await createSourceControlBranch(repoPath, "feature/source-control");
    const switched = await switchSourceControlBranch(repoPath, "main");
    const log = await getSourceControlLog(repoPath, 5);
    const blame = await getSourceControlBlame(repoPath, "src/main.rs");

    expect(unstage.operation).toBe("unstage");
    expect(branches[0].current).toBe(true);
    expect(created.operation).toBe("create_branch");
    expect(switched.operation).toBe("switch_branch");
    expect(log.entries[0].message).toBe("init");
    expect(blame.lines[0].line_number).toBe(1);
    expect(fetchMock).toHaveBeenNthCalledWith(
      1,
      "http://127.0.0.1:37501/source-control/unstage",
      expect.objectContaining({
        method: "POST",
        body: JSON.stringify({ repo_path: repoPath, paths: ["src/main.rs"] }),
      }),
    );
    expect(fetchMock).toHaveBeenNthCalledWith(
      2,
      `http://127.0.0.1:37501/source-control/branches?${repoQuery}`,
      expect.objectContaining({ method: "GET" }),
    );
    expect(fetchMock).toHaveBeenNthCalledWith(
      3,
      "http://127.0.0.1:37501/source-control/branches",
      expect.objectContaining({
        method: "POST",
        body: JSON.stringify({
          repo_path: repoPath,
          name: "feature/source-control",
        }),
      }),
    );
    expect(fetchMock).toHaveBeenNthCalledWith(
      4,
      "http://127.0.0.1:37501/source-control/switch",
      expect.objectContaining({
        method: "POST",
        body: JSON.stringify({ repo_path: repoPath, name: "main" }),
      }),
    );
    expect(fetchMock).toHaveBeenNthCalledWith(
      5,
      `http://127.0.0.1:37501/source-control/log?${logQuery}`,
      expect.objectContaining({ method: "GET" }),
    );
    expect(fetchMock).toHaveBeenNthCalledWith(
      6,
      `http://127.0.0.1:37501/source-control/blame?${blameQuery}`,
      expect.objectContaining({ method: "GET" }),
    );
  });
});

describe("Code symbol nav API client", () => {
  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it("fetches one code symbol by entity id from the backend nav route", async () => {
    const fetchMock = vi.fn().mockResolvedValueOnce(
      jsonResponse({
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
      }),
    );
    vi.stubGlobal("fetch", fetchMock);

    const result = await getCodeSymbol("KEN-symbol-alpha");

    expect(result.symbol.display_name).toBe("GraphSearchAlpha");
    expect(fetchMock).toHaveBeenCalledWith(
      "http://127.0.0.1:37501/knowledge/code/symbols/KEN-symbol-alpha",
      expect.objectContaining({
        method: "GET",
        headers: expect.objectContaining({
          "x-hsk-actor-kind": "model_adapter",
          "x-hsk-actor-id": "handshake-monaco-code-intelligence",
          "x-hsk-kernel-task-run-id": "MT-249-symbol-KEN-symbol-alpha",
          "x-hsk-session-run-id": "frontend-code-intelligence",
          "x-hsk-correlation-id": "frontend-code-intelligence-symbol-KEN-symbol-alpha",
        }),
      }),
    );
  });

  it("looks up code symbols by prefix for Monaco completions", async () => {
    const fetchMock = vi.fn().mockResolvedValueOnce(
      jsonResponse({
        workspace_id: "w1",
        matches: [
          {
            symbol_entity_id: "KEN-symbol-alpha",
            symbol_key: "rust:src/lib.rs#add",
            display_name: "add",
            symbol_kind: "function",
            lifecycle_state: "active",
            definition: null,
            staleness: { state: "fresh", fresh: true },
          },
        ],
        nav_receipt_event_id: "EVT-lookup",
        quiet_background_work_receipt_id: "quiet-lookup",
      }),
    );
    vi.stubGlobal("fetch", fetchMock);

    const result = await lookupCodeSymbols({ workspaceId: "w1", prefix: "ad", limit: 20 });

    expect(result.matches[0].display_name).toBe("add");
    expect(fetchMock).toHaveBeenCalledWith(
      "http://127.0.0.1:37501/knowledge/code/symbols?workspace_id=w1&prefix=ad&limit=20",
      expect.objectContaining({
        headers: expect.objectContaining({
          "x-hsk-actor-kind": "model_adapter",
          "x-hsk-kernel-task-run-id": "MT-249-lookup-w1",
        }),
      }),
    );
  });

  it("fetches code symbol references with backend nav identity headers", async () => {
    const fetchMock = vi.fn().mockResolvedValueOnce(
      jsonResponse({
        symbol_entity_id: "KEN-symbol-alpha",
        staleness: { state: "fresh", fresh: true },
        callers: [],
        callees: [],
        nav_receipt_event_id: "EVT-refs",
        quiet_background_work_receipt_id: "quiet-refs",
      }),
    );
    vi.stubGlobal("fetch", fetchMock);

    await getCodeSymbolReferences("KEN-symbol-alpha");

    expect(fetchMock).toHaveBeenCalledWith(
      "http://127.0.0.1:37501/knowledge/code/symbols/KEN-symbol-alpha/references",
      expect.objectContaining({
        headers: expect.objectContaining({
          "x-hsk-actor-kind": "model_adapter",
          "x-hsk-kernel-task-run-id": "MT-249-references-KEN-symbol-alpha",
        }),
      }),
    );
  });

  it("fetches the file lens payload with encoded path and staleness inputs", async () => {
    const fetchMock = vi.fn().mockResolvedValueOnce(
      jsonResponse({
        workspace_id: "w1",
        relative_path: "src/lib.rs",
        staleness: { state: "fresh", fresh: true },
        truncated: false,
        entries: [],
        nav_receipt_event_id: "EVT-lens",
        quiet_background_work_receipt_id: "quiet-lens",
      }),
    );
    vi.stubGlobal("fetch", fetchMock);

    await getCodeFileLens("w1", "src/lib.rs", "abc123", "tree-sitter-rust@1");

    expect(fetchMock).toHaveBeenCalledWith(
      "http://127.0.0.1:37501/knowledge/code/files/src%2Flib.rs/lens?workspace_id=w1&content_hash=abc123&parser_version=tree-sitter-rust%401",
      expect.objectContaining({
        headers: expect.objectContaining({
          "x-hsk-actor-kind": "model_adapter",
          "x-hsk-kernel-task-run-id": "MT-249-lens-w1",
        }),
      }),
    );
  });
});

function jsonResponse(body: unknown): Response {
  return {
    ok: true,
    status: 200,
    statusText: "OK",
    headers: new Headers({ "content-type": "application/json" }),
    text: async () => JSON.stringify(body),
  } as Response;
}
