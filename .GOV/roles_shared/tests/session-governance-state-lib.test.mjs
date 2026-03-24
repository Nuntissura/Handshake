import assert from "node:assert/strict";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import test from "node:test";
import { evaluateSessionGovernanceState } from "../scripts/session/session-governance-state-lib.mjs";

function writeFile(filePath, content) {
  fs.mkdirSync(path.dirname(filePath), { recursive: true });
  fs.writeFileSync(filePath, content, "utf8");
}

function withTempRepo(fn) {
  const repoRoot = fs.mkdtempSync(path.join(os.tmpdir(), "hsk-session-governance-"));
  try {
    writeFile(
      path.join(repoRoot, ".GOV", "roles_shared", "records", "TASK_BOARD.md"),
      "# Board\n\n## Ready for Dev\n- **[WP-1-Example-v1]** - [READY_FOR_DEV]\n\n## Superseded\n- **[WP-1-Old-v1]** - [SUPERSEDED]\n",
    );
    writeFile(
      path.join(repoRoot, ".GOV", "task_packets", "WP-1-Example-v1.md"),
      "# Task Packet\n\n- **Status:** Ready for Dev\n",
    );
    writeFile(
      path.join(repoRoot, ".GOV", "task_packets", "WP-1-Old-v1.md"),
      "# Task Packet\n\n- **Status:** Done\n",
    );
    writeFile(path.join(repoRoot, "wt-example", ".gitkeep"), "");
    fn(repoRoot);
  } finally {
    fs.rmSync(repoRoot, { recursive: true, force: true });
  }
}

test("evaluateSessionGovernanceState allows launch and steering for active packets with an existing worktree", () => {
  withTempRepo((repoRoot) => {
    const result = evaluateSessionGovernanceState(repoRoot, {
      wp_id: "WP-1-Example-v1",
      local_worktree_dir: "wt-example",
    });

    assert.equal(result.launchAllowed, true);
    assert.equal(result.steeringAllowed, true);
    assert.equal(result.taskBoardStatus, "READY_FOR_DEV");
    assert.equal(result.localWorktreeExists, true);
  });
});

test("evaluateSessionGovernanceState blocks steering when the assigned worktree is missing", () => {
  withTempRepo((repoRoot) => {
    const result = evaluateSessionGovernanceState(repoRoot, {
      wp_id: "WP-1-Example-v1",
      local_worktree_dir: "wt-missing",
    });

    assert.equal(result.launchAllowed, true);
    assert.equal(result.steeringAllowed, false);
    assert.match(result.steeringBlockers.join(" "), /assigned worktree is missing/i);
  });
});

test("evaluateSessionGovernanceState blocks launch and steering for terminal task-board states", () => {
  withTempRepo((repoRoot) => {
    const result = evaluateSessionGovernanceState(repoRoot, {
      wp_id: "WP-1-Old-v1",
      local_worktree_dir: "wt-old",
    });

    assert.equal(result.launchAllowed, false);
    assert.equal(result.steeringAllowed, false);
    assert.equal(result.taskBoardStatus, "SUPERSEDED");
    assert.match(result.launchBlockers.join(" "), /task board status is terminal/i);
  });
});
