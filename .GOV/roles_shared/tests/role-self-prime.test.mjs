import assert from "node:assert/strict";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { spawnSync } from "node:child_process";
import test from "node:test";
import { fileURLToPath } from "node:url";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(__dirname, "../../..");
const selfPrimeScript = path.resolve(__dirname, "../scripts/session/role-self-prime.mjs");

function normalizePath(value) {
  return String(value || "").replace(/\\/g, "/");
}

function writeText(filePath, text = "") {
  fs.mkdirSync(path.dirname(filePath), { recursive: true });
  fs.writeFileSync(filePath, text, "utf8");
}

function createFixture() {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), "hsk-role-self-prime-"));
  const govRoot = path.join(root, ".GOV");
  const govRuntimeRoot = path.join(root, "gov_runtime");
  const wpId = "WP-TEST-SELF-PRIME-v1";
  const packetPath = path.join(govRoot, "task_packets", wpId, "packet.md");
  const commDir = path.join(govRuntimeRoot, "roles_shared", "WP_COMMUNICATIONS", wpId);
  const runtimeStatusPath = path.join(commDir, "RUNTIME_STATUS.json");
  const receiptsPath = path.join(commDir, "RECEIPTS.jsonl");
  const notificationsPath = path.join(commDir, "NOTIFICATIONS.jsonl");
  const cursorPath = path.join(commDir, "NOTIFICATION_CURSOR.json");

  writeText(packetPath, [
    `- WP_ID: ${wpId}`,
    "- **Status:** In Progress",
    "- WORKFLOW_LANE: ORCHESTRATOR_MANAGED",
    "- PACKET_FORMAT_VERSION: 2026-03-22",
    "- COMMUNICATION_CONTRACT: DIRECT_REVIEW_V1",
    "- COMMUNICATION_HEALTH_GATE: HANDOFF_VERDICT_BLOCKING",
    `- WP_RUNTIME_STATUS_FILE: ${normalizePath(runtimeStatusPath)}`,
    `- WP_RECEIPTS_FILE: ${normalizePath(receiptsPath)}`,
  ].join("\n"));

  writeText(path.join(govRoot, "task_packets", wpId, "MT-001.md"), [
    "# MT-001: Self-prime fixture [CX-SELF-001]",
    "",
    "## METADATA",
    `- WP_ID: ${wpId}`,
    "- MT_ID: MT-001",
    "- CLAUSE: Self-prime fixture [CX-SELF-001]",
    "- CODE_SURFACES: src/example.rs",
    "- EXPECTED_TESTS: cargo test self_prime_fixture -- --exact",
    "- DEPENDS_ON: NONE",
  ].join("\n"));

  writeText(runtimeStatusPath, `${JSON.stringify({
    schema_version: "wp_runtime_status@1",
    wp_id: wpId,
    task_packet: normalizePath(packetPath),
    communication_dir: normalizePath(commDir),
    thread_file: normalizePath(path.join(commDir, "THREAD.md")),
    runtime_status_file: normalizePath(runtimeStatusPath),
    receipts_file: normalizePath(receiptsPath),
    workflow_lane: "ORCHESTRATOR_MANAGED",
    runtime_status: "working",
    current_phase: "IMPLEMENTATION",
    current_task_board_status: "IN_PROGRESS",
    current_milestone: "MICROTASK",
    next_expected_actor: "CODER",
    next_expected_session: "CODER:WP-TEST-SELF-PRIME-v1",
    waiting_on: "CODER_INTENT",
    waiting_on_session: "CODER:WP-TEST-SELF-PRIME-v1",
    active_role_sessions: [
      { role: "CODER", session_id: "CODER:WP-TEST-SELF-PRIME-v1", state: "working", last_heartbeat_at: "2099-01-01T10:00:00Z" },
    ],
    open_review_items: [],
    current_coder_revision_cycle: 0,
    current_validator_review_cycle: 0,
    current_relay_escalation_cycle: 0,
  }, null, 2)}\n`);
  writeText(receiptsPath, "");
  writeText(notificationsPath, "");
  writeText(cursorPath, `${JSON.stringify({ schema_version: "wp_notification_cursor@1", cursors: {} }, null, 2)}\n`);
  writeText(path.join(commDir, "THREAD.md"), "# thread\n");
  writeText(path.join(govRuntimeRoot, "roles_shared", "ROLE_SESSION_REGISTRY.json"), `${JSON.stringify({ sessions: [] }, null, 2)}\n`);

  return { root, govRoot, govRuntimeRoot, wpId };
}

function runSelfPrime(fixture, role, extraArgs = []) {
  return spawnSync(process.execPath, [
    selfPrimeScript,
    "--role", role,
    "--wp-id", fixture.wpId,
    "--mt-id", "MT-001",
    "--session-id", `${role}:${fixture.wpId}`,
    ...extraArgs,
  ], {
    cwd: repoRoot,
    env: {
      ...process.env,
      HANDSHAKE_GOV_ROOT: fixture.govRoot,
      HANDSHAKE_GOV_RUNTIME_ROOT: fixture.govRuntimeRoot,
    },
    encoding: "utf8",
  });
}

test("role-self-prime builds effective prompts for governed implementation roles", () => {
  const fixture = createFixture();
  for (const role of ["CODER", "WP_VALIDATOR", "INTEGRATION_VALIDATOR"]) {
    const result = runSelfPrime(fixture, role);
    assert.equal(result.status, 0, `${role}\nSTDOUT:\n${result.stdout}\nSTDERR:\n${result.stderr}`);
    assert.match(result.stdout, /ROLE_SELF_PRIME \[RGF-246\]/);
    assert.match(result.stdout, new RegExp(`- ROLE: ${role}`));
    assert.match(result.stdout, /SOURCE_PRIORITY: terminal_closeout_record -> packet_projection -> mt_board -> runtime_status -> repomem\/governance_memory/);
    assert.match(result.stdout, /ACTIVE_LANE_BRIEF \[CX-LANE-001\]/);
    assert.match(result.stdout, /ROLE_SELF_PRIME_EFFECTIVE_PROMPT:/);
    assert.match(result.stdout, new RegExp(`ROLE LOCK: You are the ${role}`));
    assert.match(result.stdout, /SESSION_OPEN \(MANDATORY\): Before any governed mutation/i);
  }
});

test("role-self-prime PreCompact writes a fresh prompt prefix to the summary file", () => {
  const fixture = createFixture();
  const summaryPath = path.join(fixture.root, "compact-summary.md");
  const result = runSelfPrime(fixture, "CODER", [
    "--event", "PreCompact",
    "--write-summary", summaryPath,
    "--json",
  ]);

  assert.equal(result.status, 0, result.stderr || result.stdout);
  const payload = JSON.parse(result.stdout);
  assert.equal(payload.event, "PreCompact");
  assert.match(payload.prompt, /ROLE_SELF_PRIME \[RGF-246\]/);
  const summary = fs.readFileSync(summaryPath, "utf8");
  assert.match(summary, /ROLE_SELF_PRIME_PRECOMPACT \[RGF-246\]/);
  assert.match(summary, /ROLE_SELF_PRIME_EFFECTIVE_PROMPT:/);
});
