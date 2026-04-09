import test from "node:test";
import assert from "node:assert/strict";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";

import {
  activeOrchestratorCandidates,
  buildPostWorkCommand,
  comparePrepareAgainstPacketTruth,
  inferWpIdFromPrepare,
  isTerminalTaskBoardStatus,
  normalizeVerdict,
  parseExplicitCoderHandoffRange,
  resolveCommittedCoderHandoffRange,
  workflowStartReadinessState,
} from "../scripts/lib/role-resume-utils.mjs";

const REPO_ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../../../../handshake_main");

test("comparePrepareAgainstPacketTruth accepts matching packet and PREPARE authority", () => {
  const packet = [
    "- WORKFLOW_LANE: ORCHESTRATOR_MANAGED",
    "- EXECUTION_OWNER: CODER_A",
    "- LOCAL_BRANCH: feat/WP-1-Example-v1",
    "- LOCAL_WORKTREE_DIR: ../wtc-example-v1",
  ].join("\n");
  const prepare = {
    workflow_lane: "ORCHESTRATOR_MANAGED",
    execution_lane: "CODER_A",
    branch: "feat/WP-1-Example-v1",
    worktree_dir: "../wtc-example-v1",
  };

  const result = comparePrepareAgainstPacketTruth(packet, prepare, REPO_ROOT);

  assert.equal(result.ok, true);
  assert.deepEqual(result.issues, []);
});

test("comparePrepareAgainstPacketTruth accepts normalized execution owner aliases", () => {
  const packet = [
    "- WORKFLOW_LANE: ORCHESTRATOR_MANAGED",
    "- EXECUTION_OWNER: CODER_A",
    "- LOCAL_BRANCH: feat/WP-1-Example-v1",
    "- LOCAL_WORKTREE_DIR: ../wtc-example-v1",
  ].join("\n");
  const prepare = {
    workflow_lane: "ORCHESTRATOR_MANAGED",
    execution_lane: "Coder-A",
    branch: "feat/WP-1-Example-v1",
    worktree_dir: "../wtc-example-v1",
  };

  const result = comparePrepareAgainstPacketTruth(packet, prepare, REPO_ROOT);

  assert.equal(result.ok, true);
  assert.deepEqual(result.issues, []);
});

test("comparePrepareAgainstPacketTruth flags packet/PREPARE authority drift", () => {
  const packet = [
    "- WORKFLOW_LANE: ORCHESTRATOR_MANAGED",
    "- EXECUTION_OWNER: CODER_A",
    "- LOCAL_BRANCH: feat/WP-1-Example-v1",
    "- LOCAL_WORKTREE_DIR: ../wtc-example-v1",
  ].join("\n");
  const prepare = {
    workflow_lane: "MANUAL_RELAY",
    execution_lane: "CODER_B",
    branch: "feat/WP-1-Other-v1",
    worktree_dir: "../wtc-other-v1",
  };

  const result = comparePrepareAgainstPacketTruth(packet, prepare, REPO_ROOT);

  assert.equal(result.ok, false);
  assert.deepEqual(result.issues, [
    "Official packet WORKFLOW_LANE conflicts with PREPARE: expected ORCHESTRATOR_MANAGED, got MANUAL_RELAY",
    "Official packet EXECUTION_OWNER conflicts with PREPARE: expected CODER_A, got CODER_B",
    "Official packet LOCAL_BRANCH conflicts with PREPARE: expected feat/WP-1-Example-v1, got feat/WP-1-Other-v1",
    "Official packet LOCAL_WORKTREE_DIR conflicts with PREPARE: expected ../wtc-example-v1, got ../wtc-other-v1",
  ]);
});

test("parseExplicitCoderHandoffRange prefers the latest fixed packet range and ignores HEAD-based commands", () => {
  const packet = [
    "- MERGE_BASE_SHA: facce56f879d4ee990f62566b12a8b26d8bc61d7",
    "- Ran: `just phase-check HANDOFF WP-1-Example-v1 CODER --range facce56f..HEAD`",
    "- Ran: `just phase-check HANDOFF WP-1-Example-v1 CODER --range bf3e7f81..4ba26a4`",
    "- COMMAND: `just phase-check HANDOFF WP-1-Example-v1 CODER --range deadbee..feed123`",
  ].join("\n");

  assert.deepEqual(parseExplicitCoderHandoffRange(packet, "WP-1-Example-v1"), {
    baseRev: "deadbee",
    headRev: "feed123",
  });
});

test("resolveCommittedCoderHandoffRange falls back to MERGE_BASE_SHA when no fixed packet range exists", () => {
  const packet = [
    "- MERGE_BASE_SHA: facce56f879d4ee990f62566b12a8b26d8bc61d7",
    "- Ran: `just phase-check HANDOFF WP-1-Example-v1 CODER --range facce56f..HEAD`",
  ].join("\n");

  assert.deepEqual(resolveCommittedCoderHandoffRange(packet, "WP-1-Example-v1"), {
    baseRev: "facce56f879d4ee990f62566b12a8b26d8bc61d7",
    headRev: "HEAD",
    source: "PACKET_MERGE_BASE",
  });
});

test("buildPostWorkCommand prefers the packet explicit committed handoff range over MERGE_BASE_SHA", () => {
  const packet = [
    "- MERGE_BASE_SHA: facce56f879d4ee990f62566b12a8b26d8bc61d7",
    "- Ran: `just phase-check HANDOFF WP-1-Example-v1 CODER --range bf3e7f81..4ba26a4`",
  ].join("\n");

  assert.equal(
    buildPostWorkCommand("WP-1-Example-v1", packet),
    "just phase-check HANDOFF WP-1-Example-v1 CODER --range bf3e7f81..4ba26a4",
  );
});

test("terminal board helpers treat ABANDONED as a first-class terminal state", () => {
  assert.equal(isTerminalTaskBoardStatus("ABANDONED"), true);
  assert.equal(normalizeVerdict("abandoned"), "ABANDONED");
});

test("repo-local resume helpers ignore foreign HANDSHAKE_GOV_ROOT when evaluating another repo root", () => {
  const repoRoot = fs.mkdtempSync(path.join(os.tmpdir(), "hsk-role-resume-"));
  const foreignGovRoot = fs.mkdtempSync(path.join(os.tmpdir(), "hsk-foreign-gov-"));
  try {
    fs.mkdirSync(path.join(repoRoot, ".GOV", "roles_shared", "records"), { recursive: true });
    fs.mkdirSync(path.join(repoRoot, ".GOV", "task_packets"), { recursive: true });
    fs.writeFileSync(
      path.join(repoRoot, ".GOV", "roles_shared", "records", "TASK_BOARD.md"),
      "# Board\n\n## Done\n- **[WP-1-Test-v1]** - [SUPERSEDED]\n",
      "utf8",
    );
    fs.writeFileSync(
      path.join(repoRoot, ".GOV", "task_packets", "WP-1-Test-v1.md"),
      "# Packet\n\n- **Status:** Done\n",
      "utf8",
    );

    const probe = [
      "import { packetPathAtRepo, taskBoardEntriesAtRepo } from './.GOV/roles_shared/scripts/lib/role-resume-utils.mjs';",
      `const repoRoot = ${JSON.stringify(repoRoot)};`,
      "console.log(JSON.stringify({",
      "  packetPathAtRepo: packetPathAtRepo('WP-1-Test-v1', repoRoot),",
      "  taskBoardEntriesAtRepo: taskBoardEntriesAtRepo(repoRoot),",
      "}, null, 2));",
    ].join("\n");
    const result = spawnSync(process.execPath, ["-e", probe], {
      cwd: path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../../.."),
      env: {
        ...process.env,
        HANDSHAKE_GOV_ROOT: foreignGovRoot,
      },
      encoding: "utf8",
    });

    assert.equal(result.status, 0, result.stderr || result.stdout);
    const parsed = JSON.parse(result.stdout);
    assert.equal(parsed.packetPathAtRepo, path.join(repoRoot, ".GOV", "task_packets", "WP-1-Test-v1.md"));
    assert.deepEqual(parsed.taskBoardEntriesAtRepo, [{ wpId: "WP-1-Test-v1", status: "SUPERSEDED" }]);
  } finally {
    fs.rmSync(repoRoot, { recursive: true, force: true });
    fs.rmSync(foreignGovRoot, { recursive: true, force: true });
  }
});

test("activeOrchestratorCandidates and inferWpIdFromPrepare honor the evaluated repo root task board", () => {
  const repoRoot = fs.mkdtempSync(path.join(os.tmpdir(), "hsk-role-resume-board-"));
  try {
    fs.mkdirSync(path.join(repoRoot, ".GOV", "roles_shared", "records"), { recursive: true });
    fs.writeFileSync(
      path.join(repoRoot, ".GOV", "roles_shared", "records", "TASK_BOARD.md"),
      "# Board\n\n## Done\n- **[WP-1-Resume-Locality-v1]** - [SUPERSEDED]\n",
      "utf8",
    );

    const logs = [
      {
        wpId: "WP-1-Resume-Locality-v1",
        type: "PREPARE",
        timestamp: new Date().toISOString(),
        branch: "feat/WP-1-Resume-Locality-v1",
        worktree_dir: repoRoot,
      },
    ];

    const candidates = activeOrchestratorCandidates(logs, repoRoot);
    assert.deepEqual(candidates, []);

    const inferred = inferWpIdFromPrepare(logs, {
      branch: "feat/WP-1-Resume-Locality-v1",
      topLevel: repoRoot,
    }, repoRoot);
    assert.equal(inferred.wpId, null);
    assert.deepEqual(inferred.candidates, []);
  } finally {
    fs.rmSync(repoRoot, { recursive: true, force: true });
  }
});

test("workflowStartReadinessState loads gate logs from the evaluated repo runtime root", () => {
  const repoRoot = fs.mkdtempSync(path.join(os.tmpdir(), "hsk-role-resume-runtime-"));
  const foreignRuntimeRoot = fs.mkdtempSync(path.join(os.tmpdir(), "hsk-foreign-runtime-"));
  const wpId = "WP-1-Resume-Readiness-v1";
  const localBranch = `feat/${wpId}`;
  try {
    fs.mkdirSync(path.join(repoRoot, ".GOV", "roles_shared", "records"), { recursive: true });
    fs.mkdirSync(path.join(repoRoot, ".GOV", "task_packets"), { recursive: true });
    fs.mkdirSync(path.join(repoRoot, ".GOV", "spec"), { recursive: true });
    fs.writeFileSync(
      path.join(repoRoot, ".GOV", "roles_shared", "records", "TASK_BOARD.md"),
      `# Board\n\n## In Progress\n- **[${wpId}]** - [IN_PROGRESS]\n`,
      "utf8",
    );
    fs.writeFileSync(
      path.join(repoRoot, ".GOV", "task_packets", `${wpId}.md`),
      [
        "# Packet",
        "",
        "- **Status:** Done",
        "- WORKFLOW_LANE: ORCHESTRATOR_MANAGED",
        "- EXECUTION_OWNER: CODER_A",
        `- LOCAL_BRANCH: ${localBranch}`,
        "- LOCAL_WORKTREE_DIR: .",
      ].join("\n"),
      "utf8",
    );
    fs.writeFileSync(
      path.join(repoRoot, ".GOV", "spec", "SPEC_CURRENT.md"),
      "Handshake_Master_Spec_v02.179.md\n",
      "utf8",
    );
    fs.writeFileSync(
      path.join(repoRoot, ".GOV", "spec", "Handshake_Master_Spec_v02.179.md"),
      "# spec\n",
      "utf8",
    );
    fs.writeFileSync(
      path.join(repoRoot, ".GOV", "roles_shared", "records", "WP_TRACEABILITY_REGISTRY.md"),
      `| Base WP ID | Current Packet Path |\n| --- | --- |\n| WP-1-Resume-Readiness | .GOV/task_packets/${wpId}.md |\n`,
      "utf8",
    );
    spawnSync("git", ["init", "-b", localBranch], {
      cwd: repoRoot,
      stdio: "ignore",
    });
    fs.mkdirSync(path.join(repoRoot, "..", "gov_runtime", "roles_shared"), { recursive: true });
    fs.writeFileSync(
      path.join(repoRoot, "..", "gov_runtime", "roles_shared", "ORCHESTRATOR_GATES.json"),
      JSON.stringify({
        gate_logs: [
          {
            wpId,
            type: "PREPARE",
            timestamp: new Date().toISOString(),
            branch: localBranch,
            worktree_dir: ".",
            coder_id: "Coder-A",
            workflow_lane: "ORCHESTRATOR_MANAGED",
          },
        ],
      }, null, 2),
      "utf8",
    );

    const previousRuntimeRoot = process.env.HANDSHAKE_GOV_RUNTIME_ROOT;
    process.env.HANDSHAKE_GOV_RUNTIME_ROOT = foreignRuntimeRoot;
    try {
      const readiness = workflowStartReadinessState({ repoRoot });
      assert.equal(readiness.ok, true);
      assert.deepEqual(readiness.activeCandidateWpIds, [wpId]);
      assert.deepEqual(readiness.violations, []);
    } finally {
      if (previousRuntimeRoot === undefined) {
        delete process.env.HANDSHAKE_GOV_RUNTIME_ROOT;
      } else {
        process.env.HANDSHAKE_GOV_RUNTIME_ROOT = previousRuntimeRoot;
      }
    }
  } finally {
    fs.rmSync(repoRoot, { recursive: true, force: true });
    fs.rmSync(foreignRuntimeRoot, { recursive: true, force: true });
    fs.rmSync(path.join(repoRoot, "..", "gov_runtime"), { recursive: true, force: true });
  }
});
