import { afterEach, describe, expect, it, vi } from "vitest";
import { getKernelDccProjection } from "./api";

describe("Kernel DCC API projection composition", () => {
  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it("fetches runtime-backed session spawn tree projection when the DCC surface omits it", async () => {
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
              session_id: "session-runtime-backed",
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
      )
      .mockResolvedValueOnce(
        jsonResponse({
          schema_id: "hsk.kernel.session_spawn_tree_dcc_projection@1",
          tree_id: "dcc-api-composition-test.session-spawn-tree",
          panel_id: "session-spawn-tree",
          visible_fields: ["SpawnHierarchy"],
          nodes: [
            {
              session_id: "session-runtime-backed",
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
          cascade_cancel_session_ids: ["session-runtime-backed"],
          announce_back_badge_count: 1,
          runtime_record_refs: ["runtime://session-spawn/session-runtime-backed"],
          mutates_runtime_records: false,
        }),
      );
    vi.stubGlobal("fetch", fetchMock);

    const surface = await getKernelDccProjection();

    expect(surface.spawn_tree_projection?.runtime_record_refs).toEqual([
      "runtime://session-spawn/session-runtime-backed",
    ]);
    expect(fetchMock).toHaveBeenCalledTimes(2);
    expect(fetchMock).toHaveBeenNthCalledWith(
      2,
      "http://127.0.0.1:37501/api/kernel/session_spawn_tree_dcc_projection",
      expect.objectContaining({
        method: "POST",
        body: expect.stringContaining("runtime://session-spawn/session-runtime-backed"),
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
