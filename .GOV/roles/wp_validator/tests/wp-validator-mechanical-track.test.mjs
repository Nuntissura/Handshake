import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import test from "node:test";
import { fileURLToPath } from "node:url";
import { runMechanicalTrack } from "../scripts/wp-validator-mechanical-track.mjs";
import { ensureWpCommunications } from "../../../roles_shared/scripts/wp/ensure-wp-communications.mjs";
import { parseJsonFile, parseJsonlFile } from "../../../roles_shared/scripts/lib/wp-communications-lib.mjs";
import {
  governanceRuntimeAbsPath,
  repoRelativeGovernanceRuntimePath,
} from "../../../roles_shared/scripts/lib/runtime-paths.mjs";

const repoRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..", "..", "..", "..");
const taskBoardPath = path.join(repoRoot, ".GOV", "roles_shared", "records", "TASK_BOARD.md");
const buildOrderPath = path.join(repoRoot, ".GOV", "roles_shared", "records", "BUILD_ORDER.md");

function normalizePath(value) {
  return String(value || "").replace(/\\/g, "/");
}

function readOptional(filePath) {
  return fs.existsSync(filePath) ? fs.readFileSync(filePath, "utf8") : null;
}

function restoreOptional(filePath, text) {
  if (text === null) {
    fs.rmSync(filePath, { force: true });
    return;
  }
  fs.writeFileSync(filePath, text, "utf8");
}

function git(worktree, args) {
  return execFileSync("git", args, {
    cwd: worktree,
    encoding: "utf8",
    stdio: ["ignore", "pipe", "pipe"],
  }).trim();
}

function createGitWorktree() {
  const worktree = fs.mkdtempSync(path.join(os.tmpdir(), "hsk-mech-wt-"));
  fs.mkdirSync(path.join(worktree, "src"), { recursive: true });
  git(worktree, ["init"]);
  git(worktree, ["config", "user.email", "test@example.invalid"]);
  git(worktree, ["config", "user.name", "Mechanical Test"]);
  fs.writeFileSync(path.join(worktree, "src", "demo.rs"), "pub fn demo() {}\n", "utf8");
  git(worktree, ["add", "."]);
  git(worktree, ["commit", "-m", "initial"]);
  fs.writeFileSync(path.join(worktree, "src", "demo.rs"), "pub fn demo() { assert!(true); }\n", "utf8");
  git(worktree, ["add", "."]);
  git(worktree, ["commit", "-m", "feat: MT-001 demo"]);
  return worktree;
}

function writePacket({ wpId, packetDir, commDir, worktree }) {
  fs.mkdirSync(packetDir, { recursive: true });
  const commDirText = normalizePath(commDir);
  const worktreeText = normalizePath(worktree);
  fs.writeFileSync(
    path.join(packetDir, "packet.md"),
    [
      `# Task Packet: ${wpId}`,
      "",
      "- **Status:** In Progress",
      "",
      "## METADATA",
      `- WP_ID: ${wpId}`,
      `- BASE_WP_ID: ${wpId}`,
      `- WP_COMMUNICATION_DIR: ${commDirText}`,
      `- WP_RECEIPTS_FILE: ${normalizePath(path.join(commDir, "RECEIPTS.jsonl"))}`,
      `- WP_RUNTIME_STATUS_FILE: ${normalizePath(path.join(commDir, "RUNTIME_STATUS.json"))}`,
      `- WP_THREAD_FILE: ${normalizePath(path.join(commDir, "THREAD.md"))}`,
      "- EXECUTION_OWNER: CODER_A",
      "- WORKFLOW_AUTHORITY: ORCHESTRATOR",
      "- TECHNICAL_ADVISOR: WP_VALIDATOR",
      "- TECHNICAL_AUTHORITY: INTEGRATION_VALIDATOR",
      "- MERGE_AUTHORITY: INTEGRATION_VALIDATOR",
      "- LOCAL_BRANCH: feat/test-mechanical",
      `- LOCAL_WORKTREE_DIR: ${worktreeText}`,
      "- WORKFLOW_LANE: ORCHESTRATOR_MANAGED",
      "- AGENTIC_MODE: NO",
      "- PACKET_FORMAT_VERSION: 2026-04-27",
      "- COMMUNICATION_CONTRACT: DIRECT_REVIEW_V1",
      "- COMMUNICATION_HEALTH_GATE: HANDOFF_VERDICT_BLOCKING",
      "- GOVERNED_VALIDATOR_REPORT_PROFILE: SPLIT_DIFF_SCOPED_RIGOR_V3",
      "- IN_SCOPE_PATHS:",
      "  - src/",
      "",
      "## CURRENT_STATE (AUTHORITATIVE SNAPSHOT; MUTABLE)",
      "Verdict: PENDING",
      "Blockers: NONE",
      "Next: N/A",
      "",
      "## CLAUSE_CLOSURE_MATRIX",
      "- CLAUSE_ROWS:",
      "  - CLAUSE: CX-001 | CODE_SURFACES: src/demo.rs | TESTS: cargo test demo | EXAMPLES: NONE | DEBT_IDS: NONE | CODER_STATUS: PENDING | VALIDATOR_STATUS: PENDING",
    ].join("\n"),
    "utf8",
  );
  fs.writeFileSync(
    path.join(packetDir, "MT-001.md"),
    [
      "# MT-001: Demo",
      "",
      "## METADATA",
      `- WP_ID: ${wpId}`,
      "- MT_ID: MT-001",
      "- CLAUSE: [CX-001] Demo",
      "- CODE_SURFACES: src/demo.rs",
      "- EXPECTED_TESTS: cargo test demo",
      "- DEPENDS_ON: NONE",
    ].join("\n"),
    "utf8",
  );
}

function setupFixture(wpId) {
  const packetDir = path.join(repoRoot, ".GOV", "task_packets", wpId);
  const commDir = repoRelativeGovernanceRuntimePath("roles_shared", "WP_COMMUNICATIONS", wpId);
  const commDirAbs = governanceRuntimeAbsPath("roles_shared", "WP_COMMUNICATIONS", wpId);
  const worktree = createGitWorktree();
  const taskBoardBefore = readOptional(taskBoardPath);
  const buildOrderBefore = readOptional(buildOrderPath);
  fs.rmSync(commDirAbs, { recursive: true, force: true });
  writePacket({ wpId, packetDir, commDir, worktree });
  ensureWpCommunications({ wpId }, { assumeTransactionLock: true });
  return {
    packetDir,
    commDir,
    commDirAbs,
    worktree,
    taskBoardBefore,
    buildOrderBefore,
    cleanup() {
      fs.rmSync(packetDir, { recursive: true, force: true });
      fs.rmSync(commDirAbs, { recursive: true, force: true });
      fs.rmSync(worktree, { recursive: true, force: true });
      restoreOptional(taskBoardPath, taskBoardBefore);
      restoreOptional(buildOrderPath, buildOrderBefore);
    },
  };
}

test("mechanical track passes in-scope MT files and returns typed details", async () => {
  const wpId = "WP-TEST-MECHANICAL-PASS-v1";
  const fixture = setupFixture(wpId);
  try {
    const result = await runMechanicalTrack({
      wpId,
      mtId: "MT-001",
      range: "HEAD~1..HEAD",
      writeReceipt: false,
    });

    assert.equal(result.verdict, "PASS", JSON.stringify(result, null, 2));
    assert.equal(result.boundary_check_result.status, "PASS");
    assert.deepEqual(result.file_list_match_result.changed_files, ["src/demo.rs"]);
    assert.equal(result.mechanical_result, undefined);
  } finally {
    fixture.cleanup();
  }
});

test("mechanical track FAIL writes typed receipt, runtime route, and coder notification", async () => {
  const wpId = "WP-TEST-MECHANICAL-FAIL-v1";
  const fixture = setupFixture(wpId);
  try {
    const result = await runMechanicalTrack({
      wpId,
      mtId: "MT-001",
      changedFiles: ["src/outside.rs"],
      actorSession: "wpv-mechanical-test",
    });

    assert.equal(result.verdict, "FAIL");
    assert.equal(result.receipt_written, true);
    assert.ok(result.concerns.some((entry) => entry.key === "CHANGED_FILES_OUTSIDE_MT_CODE_SURFACES"));

    const receipts = parseJsonlFile(normalizePath(path.join(fixture.commDir, "RECEIPTS.jsonl")));
    const mechanical = receipts.find((entry) => entry.receipt_kind === "MT_VERDICT_MECHANICAL");
    assert.ok(mechanical);
    assert.equal(mechanical.mechanical_result.mt_id, "MT-001");
    assert.equal(mechanical.mechanical_result.verdict, "FAIL");
    assert.equal(mechanical.verb_body.track, "MECHANICAL");

    const runtime = parseJsonFile(normalizePath(path.join(fixture.commDir, "RUNTIME_STATUS.json")));
    assert.equal(runtime.route_anchor_kind, "MT_MECHANICAL_FAIL");
    assert.equal(runtime.next_expected_actor, "CODER");
    assert.equal(runtime.waiting_on, "MT_MECHANICAL_REMEDIATION");

    const notifications = parseJsonlFile(normalizePath(path.join(fixture.commDir, "NOTIFICATIONS.jsonl")));
    assert.ok(notifications.some((entry) =>
      entry.source_kind === "MT_VERDICT_MECHANICAL"
      && entry.target_role === "CODER"
    ));
  } finally {
    fixture.cleanup();
  }
});
