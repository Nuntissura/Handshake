import { afterEach, describe, expect, it, vi } from "vitest";
import {
  acceptAtelierAiTagSuggestion,
  applyAtelierAiTagSuggestion,
  archiveAtelierDeletionTargets,
  getKernelDccProjection,
  importAtelierClipboardImage,
  listAtelierAiTagSuggestionsForCharacter,
  listAtelierFilesystemHealthFindings,
  listAtelierStealthWindows,
  openAtelierIntakeBatch,
  previewAtelierDeletionImpact,
  recordAtelierAiTagSuggestion,
  recordAtelierUrlImageImport,
  rejectAtelierAiTagSuggestion,
  restoreAtelierDeletionTargets,
  runAtelierFilesystemHealthCheck,
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

function jsonResponse(body: unknown): Response {
  return {
    ok: true,
    status: 200,
    statusText: "OK",
    headers: new Headers({ "content-type": "application/json" }),
    text: async () => JSON.stringify(body),
  } as Response;
}
