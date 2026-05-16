import { afterEach, describe, expect, it, vi } from "vitest";
import { getKernelDccProjection } from "./api";

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

function jsonResponse(body: unknown): Response {
  return {
    ok: true,
    status: 200,
    statusText: "OK",
    headers: new Headers({ "content-type": "application/json" }),
    text: async () => JSON.stringify(body),
  } as Response;
}
