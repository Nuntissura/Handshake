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

test("evaluateSessionGovernanceState prefers canonical runtime projection when packet and board artifacts lag", () => {
  withTempRepo((repoRoot) => {
    const wpId = "WP-1-Example-v1";
    const runtimeStatusPath = path.join(repoRoot, "gov_runtime", "roles_shared", "WP_COMMUNICATIONS", wpId, "RUNTIME_STATUS.json");
    writeFile(
      path.join(repoRoot, ".GOV", "task_packets", `${wpId}.md`),
      [
        "# Task Packet",
        "",
        `- WP_RUNTIME_STATUS_FILE: ${path.relative(repoRoot, runtimeStatusPath).replace(/\\\\/g, "/")}`,
        "- **Status:** In Progress",
      ].join("\n"),
    );
    writeFile(
      runtimeStatusPath,
      JSON.stringify({
        schema_version: "wp_runtime_status@1",
        current_packet_status: "In Progress",
        current_task_board_status: "IN_PROGRESS",
        runtime_status: "working",
        current_phase: "IMPLEMENTATION",
        execution_state: {
          schema_version: "wp_execution_state@1",
          authority: {
            packet_status: "Validated (PASS)",
            task_board_status: "DONE_VALIDATED",
            runtime_status: "completed",
            phase: "STATUS_SYNC",
            route_anchor: {},
            review_anchor: {},
          },
          checkpoint_lineage: {
            schema_version: "wp_execution_checkpoint_lineage@1",
            latest_checkpoint_id: null,
            latest_checkpoint_at_utc: null,
            latest_checkpoint_kind: null,
            latest_restore_point_id: null,
            latest_checkpoint_fingerprint: null,
            checkpoint_count: 0,
            checkpoints: [],
          },
        },
      }, null, 2),
    );

    const result = evaluateSessionGovernanceState(repoRoot, {
      wp_id: wpId,
      local_worktree_dir: "wt-example",
    });

    assert.equal(result.packetStatusArtifact, "In Progress");
    assert.equal(result.taskBoardStatusArtifact, "READY_FOR_DEV");
    assert.equal(result.runtimePacketStatus, "Validated (PASS)");
    assert.equal(result.runtimeTaskBoardStatus, "DONE_VALIDATED");
    assert.equal(result.packetStatus, "Validated (PASS)");
    assert.equal(result.taskBoardStatus, "DONE_VALIDATED");
    assert.equal(result.packetProjectionDrift, true);
    assert.equal(result.taskBoardProjectionDrift, true);
    assert.equal(result.launchAllowed, false);
    assert.equal(result.terminalTaskBoardStatus, true);
  });
});
