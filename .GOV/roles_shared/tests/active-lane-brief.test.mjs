import assert from "node:assert/strict";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { spawnSync } from "node:child_process";
import test from "node:test";
import { fileURLToPath } from "node:url";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(__dirname, "../../..");
const briefScript = path.resolve(__dirname, "../checks/active-lane-brief.mjs");

function normalizePath(value) {
  return String(value || "").replace(/\\/g, "/");
}

function writeText(filePath, text = "") {
  fs.mkdirSync(path.dirname(filePath), { recursive: true });
  fs.writeFileSync(filePath, text, "utf8");
}

function createFixture() {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), "hsk-active-lane-"));
  const govRoot = path.join(root, ".GOV");
  const govRuntimeRoot = path.join(root, "gov_runtime");
  const wpId = "WP-TEST-ACTIVE-LANE-v1";
  const packetPath = path.join(govRoot, "task_packets", wpId, "packet.md");
  const commDir = path.join(govRuntimeRoot, "roles_shared", "WP_COMMUNICATIONS", wpId);
  const runtimeStatusPath = path.join(commDir, "RUNTIME_STATUS.json");
  const receiptsPath = path.join(commDir, "RECEIPTS.jsonl");
  const notificationsPath = path.join(commDir, "NOTIFICATIONS.jsonl");
  const cursorPath = path.join(commDir, "NOTIFICATION_CURSOR.json");

  writeText(
    packetPath,
    [
      `- WP_RUNTIME_STATUS_FILE: ${normalizePath(runtimeStatusPath)}`,
      `- WP_RECEIPTS_FILE: ${normalizePath(receiptsPath)}`,
      `- WORKFLOW_LANE: ORCHESTRATOR_MANAGED`,
      `- PACKET_FORMAT_VERSION: 2026-03-22`,
      `- COMMUNICATION_CONTRACT: DIRECT_REVIEW_REQUIRED`,
      `- COMMUNICATION_HEALTH_GATE: WP_COMMUNICATION_HEALTH_V1`,
      `- **Status:** In Progress`,
    ].join("\n"),
  );

  writeText(
    runtimeStatusPath,
    `${JSON.stringify({
      schema_version: "wp_runtime_status@1",
      wp_id: wpId,
      base_wp_id: "WP-TEST-ACTIVE-LANE",
      task_packet: normalizePath(packetPath),
      communication_dir: normalizePath(commDir),
      thread_file: normalizePath(path.join(commDir, "THREAD.md")),
      runtime_status_file: normalizePath(runtimeStatusPath),
      receipts_file: normalizePath(receiptsPath),
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
      next_expected_actor: "CODER",
      next_expected_session: "coder-1",
      waiting_on: "CODER_INTENT",
      waiting_on_session: "coder-1",
      validator_trigger: "NONE",
      validator_trigger_reason: null,
      attention_required: false,
      ready_for_validation: false,
      ready_for_validation_reason: null,
      current_branch: "feat/WP-TEST-ACTIVE-LANE-v1",
      current_worktree_dir: "../wtc-test-active-lane-v1",
      current_files_touched: [],
      active_role_sessions: [
        { role: "CODER", session_id: "coder-1", state: "working", last_heartbeat_at: "2099-01-01T10:00:00Z" },
      ],
      open_review_items: [],
      last_event: "receipt_validator_kickoff",
      last_event_at: "2099-01-01T10:00:00Z",
      last_heartbeat_at: "2099-01-01T10:00:00Z",
      heartbeat_interval_minutes: 15,
      heartbeat_due_at: "2099-01-01T10:15:00Z",
      stale_after: "2099-01-01T10:45:00Z",
      max_coder_revision_cycles: 3,
      max_validator_review_cycles: 3,
      max_relay_escalation_cycles: 2,
      current_coder_revision_cycle: 0,
      current_validator_review_cycle: 0,
      current_relay_escalation_cycle: 0,
      last_backup_push_at: null,
      last_backup_push_sha: null,
    }, null, 2)}\n`,
  );

  writeText(
    receiptsPath,
    `${JSON.stringify({
      schema_version: "wp_receipt@1",
      timestamp_utc: "2099-01-01T10:00:00Z",
      wp_id: wpId,
      receipt_kind: "VALIDATOR_KICKOFF",
      actor_role: "WP_VALIDATOR",
      actor_session: "wpv-1",
      summary: "Kickoff complete",
      state_before: null,
      state_after: null,
      target_role: "CODER",
      target_session: "coder-1",
      correlation_id: "kick-1",
      requires_ack: true,
      ack_for: null,
      spec_anchor: null,
      packet_row_ref: null,
      workflow_invalidity_code: null,
      refs: [],
    })}\n`,
  );

  writeText(
    notificationsPath,
    `${JSON.stringify({
      schema_version: "wp_notification@1",
      timestamp_utc: "2099-01-01T10:00:01Z",
      wp_id: wpId,
      source_kind: "VALIDATOR_KICKOFF",
      source_role: "WP_VALIDATOR",
      source_session: "wpv-1",
      target_role: "CODER",
      target_session: "coder-1",
      correlation_id: "kick-1",
      summary: "Kickoff for coder",
    })}\n`,
  );
  writeText(cursorPath, `${JSON.stringify({ schema_version: "wp_notification_cursor@1", cursors: {} }, null, 2)}\n`);
  writeText(path.join(commDir, "THREAD.md"), "# thread\n");

  return { root, govRoot, govRuntimeRoot, wpId };
}

test("active-lane-brief reports compact authority and relay summary", () => {
  const fixture = createFixture();
  const result = spawnSync(process.execPath, [briefScript, "CODER", fixture.wpId, "--json"], {
    cwd: repoRoot,
    env: {
      ...process.env,
      HANDSHAKE_GOV_ROOT: fixture.govRoot,
      HANDSHAKE_GOV_RUNTIME_ROOT: fixture.govRuntimeRoot,
    },
    encoding: "utf8",
  });

  assert.equal(result.status, 0, result.stderr || result.stdout);
  const brief = JSON.parse(result.stdout);
  assert.equal(brief.role, "CODER");
  assert.match(brief.authority, /\.GOV\/codex\/Handshake_Codex_v1\.4\.md/);
  assert.equal(brief.notifications.pending_count, 1);
  assert.equal(brief.runtime.next_expected_actor, "CODER");
  assert.equal(brief.relay.status, "NORMAL");
  assert.match(brief.relay.summary, /Relay is healthy/i);
  assert.ok(brief.next_commands.some((entry) => entry.includes(`just check-notifications ${fixture.wpId} CODER`)));
});
