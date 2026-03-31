import assert from "node:assert/strict";
import fs from "node:fs";
import path from "node:path";
import test from "node:test";

const repoRoot = path.resolve(import.meta.dirname, "../../../..");

function read(relativePath) {
  return fs.readFileSync(path.join(repoRoot, relativePath), "utf8");
}

test("orchestrator runtime entrypoints use repo-root-safe resolution on governance files", () => {
  const expectations = [
    [".GOV/roles/orchestrator/scripts/orchestrator-next.mjs", "repoPathAbs("],
    [".GOV/roles/orchestrator/scripts/session-registry-status.mjs", "const repoRoot = REPO_ROOT;"],
    [".GOV/roles/orchestrator/scripts/task-board-set.mjs", "repoPathAbs("],
    [".GOV/roles/orchestrator/scripts/wp-traceability-set.mjs", "repoPathAbs("],
    [".GOV/roles/orchestrator/scripts/orchestrator-prepare-and-packet.mjs", "cwd: REPO_ROOT"],
    [".GOV/roles/orchestrator/scripts/operator-monitor-tui.mjs", "const CURRENT_WORKTREE_DIR = REPO_ROOT;"],
    [".GOV/operator/scripts/operator-viewport-tui.mjs", "roles/orchestrator/scripts/operator-monitor-tui.mjs"],
  ];
  for (const [relativePath, needle] of expectations) {
    assert.match(read(relativePath), new RegExp(needle.replace(/[.*+?^${}()|[\]\\]/g, "\\$&")));
  }
});
