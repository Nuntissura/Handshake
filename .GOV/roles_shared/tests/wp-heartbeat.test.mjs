import assert from "node:assert/strict";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { spawnSync } from "node:child_process";
import test from "node:test";
import { fileURLToPath } from "node:url";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const heartbeatScript = path.resolve(__dirname, "../scripts/wp/wp-heartbeat.mjs");
const repoRoot = path.resolve(__dirname, "../../..");

function normalizePath(value) {
  return String(value || "").replace(/\\/g, "/");
}

function makeFixtureRoot() {
  return fs.mkdtempSync(path.join(os.tmpdir(), "hsk-heartbeat-"));
}

function writeText(filePath, text = "") {
  fs.mkdirSync(path.dirname(filePath), { recursive: true });
  fs.writeFileSync(filePath, text, "utf8");
}

function runtimeFixture(wpId, packetPath, communicationDir) {
  return {
    schema_version: "wp_runtime_status@1",
    wp_id: wpId,
    base_wp_id: "WP-TEST-HEARTBEAT",
    task_packet: normalizePath(packetPath),
    communication_dir: normalizePath(communicationDir),
    thread_file: normalizePath(path.join(communicationDir, "THREAD.md")),
    runtime_status_file: normalizePath(path.join(communicationDir, "RUNTIME_STATUS.json")),
    receipts_file: normalizePath(path.join(communicationDir, "RECEIPTS.jsonl")),
    workflow_lane: "ORCHESTRATOR_MANAGED",
    execution_owner: "CODER_A",
    workflow_authority: "ORCHESTRATOR",
    technical_advisor: "WP_VALIDATOR",
    technical_authority: "INTEGRATION_VALIDATOR",
    merge_authority: "INTEGRATION_VALIDATOR",
    wp_validator_of_record: "wpv-1",
    integration_validator_of_record: "intval-1",
    secondary_validator_sessions: [],
    agentic_mode: "YES",
    current_packet_status: "In Progress",
    main_containment_status: "NOT_STARTED",
    merged_main_commit: null,
    main_containment_verified_at_utc: null,
    runtime_status: "working",
    current_phase: "IMPLEMENTATION",
    next_expected_actor: "WP_VALIDATOR",
    next_expected_session: "wpv-1",
    waiting_on: "WP_VALIDATOR_REVIEW",
    waiting_on_session: "wpv-1",
    validator_trigger: "HANDOFF_READY",
    validator_trigger_reason: "Coder handoff recorded; WP validator review required",
    attention_required: false,
    ready_for_validation: true,
    ready_for_validation_reason: "Coder handoff recorded; WP validator review required",
    current_branch: "feat/WP-TEST-HEARTBEAT-v1",
    current_worktree_dir: "../wtc-test-heartbeat-v1",
    current_files_touched: [],
    active_role_sessions: [
      {
        role: "CODER",
        session_id: "coder-1",
        authority_kind: "PRIMARY_CODER",
        validator_role_kind: null,
        worktree_dir: "../wtc-test-heartbeat-v1",
        state: "working",
        last_heartbeat_at: "2026-03-30T10:00:00Z",
      },
    ],
    open_review_items: [],
    last_event: "receipt_coder_handoff",
    last_event_at: "2026-03-30T10:00:00Z",
    last_heartbeat_at: "2026-03-30T10:00:00Z",
    heartbeat_interval_minutes: 15,
    heartbeat_due_at: "2026-03-30T10:15:00Z",
    stale_after: "2026-03-30T10:45:00Z",
    max_coder_revision_cycles: 3,
    max_validator_review_cycles: 3,
    max_relay_escalation_cycles: 2,
    current_coder_revision_cycle: 1,
    current_validator_review_cycle: 0,
    current_relay_escalation_cycle: 0,
    last_backup_push_at: null,
    last_backup_push_sha: null,
  };
}

function createHeartbeatFixture() {
  const root = makeFixtureRoot();
  const govRoot = path.join(root, ".GOV");
  const wpId = "WP-TEST-HEARTBEAT-v1";
  const packetDir = path.join(govRoot, "task_packets", wpId);
  const packetPath = path.join(packetDir, "packet.md");
  const govRuntimeRoot = path.join(root, "gov_runtime");
  const communicationDirAbs = path.join(govRuntimeRoot, "roles_shared", "WP_COMMUNICATIONS", wpId);
  const communicationDir = normalizePath(path.relative(repoRoot, communicationDirAbs));
  const runtimePathAbs = path.join(communicationDirAbs, "RUNTIME_STATUS.json");
  const receiptsPathAbs = path.join(communicationDirAbs, "RECEIPTS.jsonl");
  const threadPathAbs = path.join(communicationDirAbs, "THREAD.md");
  const runtimePath = normalizePath(path.relative(repoRoot, runtimePathAbs));
  const receiptsPath = normalizePath(path.relative(repoRoot, receiptsPathAbs));
  const threadPath = normalizePath(path.relative(repoRoot, threadPathAbs));

  writeText(
    packetPath,
    [
      `- WP_RUNTIME_STATUS_FILE: ${normalizePath(runtimePath)}`,
      `- WP_RECEIPTS_FILE: ${normalizePath(receiptsPath)}`,
      `- WP_THREAD_FILE: ${normalizePath(threadPath)}`,
      "- LOCAL_BRANCH: feat/WP-TEST-HEARTBEAT-v1",
      "- LOCAL_WORKTREE_DIR: ../wtc-test-heartbeat-v1",
      "- HEARTBEAT_INTERVAL_MINUTES: 15",
      "- STALE_AFTER_MINUTES: 45",
      "- WORKFLOW_LANE: ORCHESTRATOR_MANAGED",
      "- PACKET_FORMAT_VERSION: HYDRATED_RESEARCH_V1",
      "- COMMUNICATION_CONTRACT: DIRECT_REVIEW_REQUIRED",
      "- COMMUNICATION_HEALTH_GATE: WP_COMMUNICATION_HEALTH_V1",
      "- **Status:** In Progress",
    ].join("\n"),
  );
  writeText(threadPathAbs, "# thread\n");
  writeText(receiptsPathAbs, "");
  writeText(
    runtimePathAbs,
    `${JSON.stringify(runtimeFixture(wpId, normalizePath(path.relative(repoRoot, packetPath)), communicationDir), null, 2)}\n`,
  );

  return {
    root,
    govRoot,
    govRuntimeRoot,
    wpId,
    runtimePath: runtimePathAbs,
    receiptsPath: receiptsPathAbs,
  };
}

function runHeartbeat(fixture, args) {
  return spawnSync(process.execPath, [heartbeatScript, ...args], {
    cwd: repoRoot,
    env: {
      ...process.env,
      HANDSHAKE_GOV_ROOT: fixture.govRoot,
      HANDSHAKE_GOV_RUNTIME_ROOT: fixture.govRuntimeRoot,
    },
    encoding: "utf8",
  });
}

test("heartbeat preserves semantic route fields and validator readiness when assertions match", () => {
  const fixture = createHeartbeatFixture();
  const result = runHeartbeat(fixture, [
    fixture.wpId,
    "CODER",
    "coder-1",
    "IMPLEMENTATION",
    "working",
    "WP_VALIDATOR",
    "WP_VALIDATOR_REVIEW",
    "NONE",
    "heartbeat_coder",
    "../wtc-test-heartbeat-v1",
    "wpv-1",
    "wpv-1",
  ]);

  assert.equal(result.status, 0, result.stderr || result.stdout);
  const runtime = JSON.parse(fs.readFileSync(fixture.runtimePath, "utf8"));
  assert.equal(runtime.next_expected_actor, "WP_VALIDATOR");
  assert.equal(runtime.next_expected_session, "wpv-1");
  assert.equal(runtime.waiting_on, "WP_VALIDATOR_REVIEW");
  assert.equal(runtime.waiting_on_session, "wpv-1");
  assert.equal(runtime.validator_trigger, "HANDOFF_READY");
  assert.equal(runtime.ready_for_validation, true);

  const receipts = fs.readFileSync(fixture.receiptsPath, "utf8").trim().split(/\r?\n/).filter(Boolean).map((line) => JSON.parse(line));
  assert.equal(receipts.length, 1);
  assert.equal(receipts[0].receipt_kind, "HEARTBEAT");
  assert.equal(receipts[0].target_role, null);
  assert.match(receipts[0].summary, /heartbeat_validator_trigger=NONE/);
});

test("heartbeat rejects attempts to mutate semantic route fields", () => {
  const fixture = createHeartbeatFixture();
  const before = fs.readFileSync(fixture.runtimePath, "utf8");
  const result = runHeartbeat(fixture, [
    fixture.wpId,
    "CODER",
    "coder-1",
    "IMPLEMENTATION",
    "working",
    "INTEGRATION_VALIDATOR",
    "FINAL_REVIEW",
    "HANDOFF_READY",
    "heartbeat_coder",
    "../wtc-test-heartbeat-v1",
    "intval-1",
    "intval-1",
  ]);

  assert.notEqual(result.status, 0);
  assert.match(result.stderr, /Heartbeat is liveness-only/i);
  assert.equal(fs.readFileSync(fixture.runtimePath, "utf8"), before);
  assert.equal(fs.readFileSync(fixture.receiptsPath, "utf8"), "");
});
