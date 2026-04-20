import assert from "node:assert/strict";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { spawnSync } from "node:child_process";
import test from "node:test";
import { fileURLToPath } from "node:url";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(__dirname, "../../..");
const closeoutScript = path.resolve(repoRoot, ".GOV/roles_shared/scripts/wp/wp-closeout-format.mjs");

function normalizePath(value) {
  return String(value || "").replace(/\\/g, "/");
}

function writeText(filePath, text = "") {
  fs.mkdirSync(path.dirname(filePath), { recursive: true });
  fs.writeFileSync(filePath, text, "utf8");
}

function buildRuntimeStatus({
  wpId,
  packetPath,
  commDir,
  runtimeStatusPath,
  receiptsPath,
  threadPath,
  executionState = null,
} = {}) {
  return {
    schema_version: "wp_runtime_status@1",
    wp_id: wpId,
    base_wp_id: wpId.replace(/-v\d+$/i, ""),
    task_packet: normalizePath(packetPath),
    communication_dir: normalizePath(commDir),
    thread_file: normalizePath(threadPath),
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
    current_task_board_status: "IN_PROGRESS",
    current_milestone: "MICROTASK",
    last_milestone_sync_at: "2099-01-01T10:00:00Z",
    next_expected_actor: "CODER",
    next_expected_session: "coder-1",
    waiting_on: "CODER_HANDOFF",
    waiting_on_session: "coder-1",
    validator_trigger: "NONE",
    validator_trigger_reason: null,
    attention_required: false,
    ready_for_validation: false,
    ready_for_validation_reason: null,
    current_branch: `feat/${wpId}`,
    current_worktree_dir: "../wtc-test",
    current_files_touched: [],
    active_role_sessions: [],
    open_review_items: [],
    route_anchor_state: null,
    route_anchor_kind: null,
    route_anchor_correlation_id: null,
    route_anchor_target_role: null,
    route_anchor_target_session: null,
    authoritative_review_receipt_kind: null,
    authoritative_review_correlation_id: null,
    authoritative_review_actor_session: null,
    authoritative_review_target_session: null,
    authoritative_review_round: null,
    committed_handoff_base_sha: null,
    committed_handoff_head_sha: null,
    committed_handoff_range_source: null,
    last_event: "receipt_validator_review",
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
    ...(executionState ? { execution_state: executionState } : {}),
  };
}

test("wp-closeout-format updates packet closeout fields and syncs runtime publication truth", () => {
  const tempRoot = fs.mkdtempSync(path.join(os.tmpdir(), "hsk-closeout-format-"));
  const govRoot = path.join(tempRoot, ".GOV");
  const govRuntimeRoot = path.join(tempRoot, "gov_runtime");
  const wpId = "WP-TEST-CLOSEOUT-v1";
  const packetPath = path.join(govRoot, "task_packets", wpId, "packet.md");
  const commDir = path.join(govRuntimeRoot, "roles_shared", "WP_COMMUNICATIONS", wpId);
  const runtimeStatusPath = path.join(commDir, "RUNTIME_STATUS.json");
  const receiptsPath = path.join(commDir, "RECEIPTS.jsonl");
  const threadPath = path.join(commDir, "THREAD.md");
  const mergedSha = "0123456789abcdef0123456789abcdef01234567";

  try {
    writeText(
      packetPath,
      [
        `- WP_RUNTIME_STATUS_FILE: ${normalizePath(runtimeStatusPath)}`,
        `- WP_RECEIPTS_FILE: ${normalizePath(receiptsPath)}`,
        `- WP_THREAD_FILE: ${normalizePath(threadPath)}`,
        `- WP_COMMUNICATION_DIR: ${normalizePath(commDir)}`,
        "- PACKET_FORMAT_VERSION: 2026-03-29",
        "- WORKFLOW_LANE: ORCHESTRATOR_MANAGED",
        "- **Status:** In Progress",
        "- MAIN_CONTAINMENT_STATUS: NOT_STARTED",
        "- MERGED_MAIN_COMMIT: NONE",
        "- MAIN_CONTAINMENT_VERIFIED_AT_UTC: NONE",
        "- CURRENT_MAIN_COMPATIBILITY_STATUS: NOT_RUN",
        "",
        "Verdict: PENDING",
        "",
        "CODER_STATUS: UNPROVEN | VALIDATOR_STATUS: PENDING",
      ].join("\n"),
    );
    writeText(
      runtimeStatusPath,
      `${JSON.stringify(buildRuntimeStatus({
        wpId,
        packetPath,
        commDir,
        runtimeStatusPath,
        receiptsPath,
        threadPath,
        executionState: {
          schema_version: "wp_execution_state@1",
          authority: {
            packet_status: "Validated (PASS)",
            task_board_status: "DONE_VALIDATED",
            runtime_status: "completed",
            phase: "STATUS_SYNC",
            next_expected_actor: "NONE",
            next_expected_session: null,
            waiting_on: "CLOSED",
            waiting_on_session: null,
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
      }), null, 2)}\n`,
    );
    writeText(receiptsPath, "\n");
    writeText(threadPath, "# thread\n");

    const result = spawnSync(process.execPath, [closeoutScript, wpId, mergedSha], {
      cwd: repoRoot,
      env: {
        ...process.env,
        HANDSHAKE_GOV_ROOT: govRoot,
        HANDSHAKE_GOV_RUNTIME_ROOT: govRuntimeRoot,
      },
      encoding: "utf8",
    });

    assert.equal(result.status, 0, result.stderr || result.stdout);
    const packet = fs.readFileSync(packetPath, "utf8");
    assert.match(packet, /- \*\*Status:\*\* Validated \(PASS\)/);
    assert.match(packet, /- MAIN_CONTAINMENT_STATUS: CONTAINED_IN_MAIN/);
    assert.match(packet, new RegExp(`- MERGED_MAIN_COMMIT: ${mergedSha}`));
    assert.match(packet, /- CURRENT_MAIN_COMPATIBILITY_STATUS: COMPATIBLE/);
    assert.match(packet, /Verdict: PASS/);
    assert.match(packet, /CODER_STATUS: PROVED \| VALIDATOR_STATUS: CONFIRMED/);

    const runtime = JSON.parse(fs.readFileSync(runtimeStatusPath, "utf8"));
    assert.equal(runtime.current_packet_status, "Validated (PASS)");
    assert.equal(runtime.current_task_board_status, "DONE_VALIDATED");
    assert.equal(runtime.main_containment_status, "CONTAINED_IN_MAIN");
    assert.equal(runtime.merged_main_commit, mergedSha);
    assert.equal(runtime.execution_state.authority.packet_status, "Validated (PASS)");
    assert.equal(runtime.execution_state.authority.main_containment_status, "CONTAINED_IN_MAIN");
    assert.equal(runtime.execution_state.authority.merged_main_commit, mergedSha);
    assert.match(result.stdout, /Runtime synced:/);
  } finally {
    fs.rmSync(tempRoot, { recursive: true, force: true });
  }
});
