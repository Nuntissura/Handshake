import assert from "node:assert/strict";
import test from "node:test";

import {
  collectWpIdsRequiringDedicatedWorktrees,
  wpRequiresDedicatedWorktreeMapping,
} from "../checks/worktree-concurrency-check.mjs";

test("packetless memory manager lanes do not require dedicated worktree mappings", () => {
  assert.equal(
    wpRequiresDedicatedWorktreeMapping({ role: "MEMORY_MANAGER", wpId: "WP-MEMORY-HYGIENE_2026-04-09T2115Z" }),
    false,
  );
  assert.equal(
    wpRequiresDedicatedWorktreeMapping({ role: "CODER", wpId: "WP-MEMORY-HYGIENE_2026-04-09T2115Z" }),
    false,
  );
  assert.equal(
    wpRequiresDedicatedWorktreeMapping({ role: "CODER", wpId: "WP-1-Workflow-Projection-Correlation-v1" }),
    true,
  );
});

test("prelaunch workflow authority roles do not require dedicated coder worktree mappings", () => {
  for (const role of ["ACTIVATION_MANAGER", "ORCHESTRATOR", "CLASSIC_ORCHESTRATOR"]) {
    assert.equal(
      wpRequiresDedicatedWorktreeMapping({ role, wpId: "WP-1-Postgres-Primary-Control-Plane-Foundation-v1" }),
      false,
      `${role} should not require a coder worktree before packet activation`,
    );
  }
  assert.equal(
    wpRequiresDedicatedWorktreeMapping({ role: "CODER", wpId: "WP-1-Postgres-Primary-Control-Plane-Foundation-v1" }),
    true,
  );
});

test("concurrency collection excludes active memory manager hygiene sessions", () => {
  const wpIds = collectWpIdsRequiringDedicatedWorktrees({
    inProgressWpIds: ["WP-1-Workflow-Projection-Correlation-v1"],
    repoRoot: process.cwd(),
    sessions: [
      {
        role: "MEMORY_MANAGER",
        wp_id: "WP-MEMORY-HYGIENE_2026-04-09T2115Z",
        runtime_state: "READY",
        local_worktree_dir: ".",
      },
      {
        role: "CODER",
        wp_id: "WP-1-Workflow-Projection-Correlation-v1",
        runtime_state: "READY",
        local_worktree_dir: ".",
      },
    ],
  });

  assert.deepEqual(wpIds, ["WP-1-Workflow-Projection-Correlation-v1"]);
});

test("concurrency collection excludes active activation manager prelaunch sessions unless task board is in progress", () => {
  const sessions = [
    {
      role: "ACTIVATION_MANAGER",
      wp_id: "WP-1-Postgres-Primary-Control-Plane-Foundation-v1",
      runtime_state: "COMMAND_RUNNING",
      local_worktree_dir: ".",
    },
  ];

  assert.deepEqual(
    collectWpIdsRequiringDedicatedWorktrees({
      inProgressWpIds: [],
      repoRoot: process.cwd(),
      sessions,
    }),
    [],
  );
  assert.deepEqual(
    collectWpIdsRequiringDedicatedWorktrees({
      inProgressWpIds: ["WP-1-Postgres-Primary-Control-Plane-Foundation-v1"],
      repoRoot: process.cwd(),
      sessions,
    }),
    ["WP-1-Postgres-Primary-Control-Plane-Foundation-v1"],
  );
});
