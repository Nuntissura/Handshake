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

test("evaluateSessionGovernanceState treats ABANDONED as a terminal task-board state", () => {
  withTempRepo((repoRoot) => {
    writeFile(
      path.join(repoRoot, ".GOV", "roles_shared", "records", "TASK_BOARD.md"),
      "# Board\n\n## Done\n- **[WP-1-Abandon-v1]** - [ABANDONED]\n",
    );
    writeFile(
      path.join(repoRoot, ".GOV", "task_packets", "WP-1-Abandon-v1.md"),
      "# Task Packet\n\n- **Status:** Validated (ABANDONED)\n",
    );

    const result = evaluateSessionGovernanceState(repoRoot, {
      wp_id: "WP-1-Abandon-v1",
      local_worktree_dir: "wt-abandon",
    });

    assert.equal(result.taskBoardStatus, "ABANDONED");
    assert.equal(result.terminalTaskBoardStatus, true);
    assert.equal(result.launchAllowed, false);
    assert.match(result.launchBlockers.join(" "), /task board status is terminal/i);
  });
});

test("evaluateSessionGovernanceState stays repo-local even when HANDSHAKE_GOV_ROOT points elsewhere", () => {
  withTempRepo((repoRoot) => {
    const foreignGovRoot = fs.mkdtempSync(path.join(os.tmpdir(), "hsk-foreign-gov-"));
    try {
      writeFile(
        path.join(repoRoot, ".GOV", "roles_shared", "records", "TASK_BOARD.md"),
        "# Board\n\n## Superseded\n- **[WP-1-Example-v1]** - [SUPERSEDED]\n",
      );
      writeFile(
        path.join(repoRoot, ".GOV", "task_packets", "WP-1-Example-v1.md"),
        "# Task Packet\n\n- **Status:** Done\n",
      );
      writeFile(
        path.join(foreignGovRoot, "task_packets", "WP-1-Example-v1.md"),
        "# Task Packet\n\n- **Status:** Ready for Dev\n",
      );

      const previousGovRoot = process.env.HANDSHAKE_GOV_ROOT;
      process.env.HANDSHAKE_GOV_ROOT = foreignGovRoot;
      try {
        const result = evaluateSessionGovernanceState(repoRoot, {
          wp_id: "WP-1-Example-v1",
          local_worktree_dir: "wt-old",
        });

        assert.equal(result.packetStatus, "Done");
        assert.equal(result.taskBoardStatus, "SUPERSEDED");
        assert.equal(result.terminalTaskBoardStatus, true);
        assert.equal(result.launchAllowed, false);
      } finally {
        if (previousGovRoot === undefined) {
          delete process.env.HANDSHAKE_GOV_ROOT;
        } else {
          process.env.HANDSHAKE_GOV_ROOT = previousGovRoot;
        }
      }
    } finally {
      fs.rmSync(foreignGovRoot, { recursive: true, force: true });
    }
  });
});
