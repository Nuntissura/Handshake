import assert from "node:assert/strict";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { spawnSync } from "node:child_process";
import test from "node:test";
import { fileURLToPath } from "node:url";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(__dirname, "../../../..");
const taskBoardSetScript = path.resolve(repoRoot, ".GOV/roles/orchestrator/scripts/task-board-set.mjs");

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
  currentPacketStatus = "In Progress",
  currentTaskBoardStatus = "IN_PROGRESS",
  runtimeStatus = "working",
  currentPhase = "IMPLEMENTATION",
  nextExpectedActor = "CODER",
  nextExpectedSession = "coder-1",
  waitingOn = "CODER_HANDOFF",
  waitingOnSession = "coder-1",
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
    current_packet_status: currentPacketStatus,
    main_containment_status: "NOT_STARTED",
    merged_main_commit: null,
    main_containment_verified_at_utc: null,
    runtime_status: runtimeStatus,
    current_phase: currentPhase,
    current_task_board_status: currentTaskBoardStatus,
    current_milestone: "MICROTASK",
    last_milestone_sync_at: "2099-01-01T10:00:00Z",
    next_expected_actor: nextExpectedActor,
    next_expected_session: nextExpectedSession,
    waiting_on: waitingOn,
    waiting_on_session: waitingOnSession,
    validator_trigger: "NONE",
    validator_trigger_reason: null,
    attention_required: false,
    ready_for_validation: false,
    ready_for_validation_reason: null,
    current_branch: `feat/${wpId}`,
    current_worktree_dir: "../wtc-test",
    current_files_touched: [],
    active_role_sessions: [
      {
        role: "CODER",
        authority_kind: "PRIMARY_CODER",
        validator_role_kind: null,
        session_id: "coder-1",
        state: "working",
        worktree_dir: "../wtc-test",
        last_heartbeat_at: "2099-01-01T10:00:00Z",
      },
    ],
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
    ...(executionState ? { execution_state: executionState } : {}),
  };
}

test("task-board-set follows canonical execution authority and only refreshes runtime mirrors", () => {
  const tempRoot = fs.mkdtempSync(path.join(os.tmpdir(), "hsk-task-board-set-"));
  const govRoot = path.join(tempRoot, ".GOV");
  const govRuntimeRoot = path.join(tempRoot, "gov_runtime");
  const wpId = "WP-TEST-TASK-BOARD-v1";
  const packetPath = path.join(govRoot, "task_packets", wpId, "packet.md");
  const commDir = path.join(govRuntimeRoot, "roles_shared", "WP_COMMUNICATIONS", wpId);
  const runtimeStatusPath = path.join(commDir, "RUNTIME_STATUS.json");
  const receiptsPath = path.join(commDir, "RECEIPTS.jsonl");
  const threadPath = path.join(commDir, "THREAD.md");

  try {
    writeText(
      path.join(govRoot, "roles_shared", "records", "TASK_BOARD.md"),
      [
        "# Board",
        "",
        "## Ready for Dev",
        `- **[${wpId}]** - [READY_FOR_DEV]`,
        "",
        "## In Progress",
        "",
        "## Done",
        "",
      ].join("\n"),
    );
    writeText(
      packetPath,
      [
        `- WP_RUNTIME_STATUS_FILE: ${normalizePath(runtimeStatusPath)}`,
        `- WP_RECEIPTS_FILE: ${normalizePath(receiptsPath)}`,
        `- WP_THREAD_FILE: ${normalizePath(threadPath)}`,
        `- WP_COMMUNICATION_DIR: ${normalizePath(commDir)}`,
        "- PACKET_FORMAT_VERSION: 2026-03-29",
        "- WORKFLOW_LANE: ORCHESTRATOR_MANAGED",
        "- COMMUNICATION_CONTRACT: DIRECT_REVIEW_REQUIRED",
        "- COMMUNICATION_HEALTH_GATE: WP_COMMUNICATION_HEALTH_V1",
        "- **Status:** In Progress",
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
            milestone: "CONTAINMENT",
            next_expected_actor: "NONE",
            next_expected_session: null,
            waiting_on: "CLOSED",
            waiting_on_session: null,
            main_containment_status: "CONTAINED_IN_MAIN",
            merged_main_commit: "abc123def456",
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

    const result = spawnSync(process.execPath, [taskBoardSetScript, wpId, "DONE_VALIDATED"], {
      cwd: repoRoot,
      env: {
        ...process.env,
        HANDSHAKE_GOV_ROOT: govRoot,
        HANDSHAKE_GOV_RUNTIME_ROOT: govRuntimeRoot,
      },
      encoding: "utf8",
    });

    assert.equal(result.status, 0, result.stderr || result.stdout);
    const taskBoard = fs.readFileSync(path.join(govRoot, "roles_shared", "records", "TASK_BOARD.md"), "utf8");
    assert.match(taskBoard, new RegExp(`\\*\\*\\[${wpId}\\]\\*\\* - \\[VALIDATED\\]`));
    const runtime = JSON.parse(fs.readFileSync(runtimeStatusPath, "utf8"));
    assert.equal(runtime.current_packet_status, "Validated (PASS)");
    assert.equal(runtime.current_task_board_status, "DONE_VALIDATED");
    assert.equal(runtime.runtime_status, "completed");
    assert.equal(runtime.execution_state.authority.task_board_status, "DONE_VALIDATED");
    assert.match(result.stdout, /runtime_authority: canonical/);
  } finally {
    fs.rmSync(tempRoot, { recursive: true, force: true });
  }
});

test("task-board-set leaves board and runtime files untouched when the requested publication is already current", () => {
  const tempRoot = fs.mkdtempSync(path.join(os.tmpdir(), "hsk-task-board-set-noop-"));
  const govRoot = path.join(tempRoot, ".GOV");
  const govRuntimeRoot = path.join(tempRoot, "gov_runtime");
  const wpId = "WP-TEST-TASK-BOARD-NOOP-v1";
  const packetPath = path.join(govRoot, "task_packets", wpId, "packet.md");
  const commDir = path.join(govRuntimeRoot, "roles_shared", "WP_COMMUNICATIONS", wpId);
  const runtimeStatusPath = path.join(commDir, "RUNTIME_STATUS.json");
  const receiptsPath = path.join(commDir, "RECEIPTS.jsonl");
  const threadPath = path.join(commDir, "THREAD.md");
  const taskBoardPath = path.join(govRoot, "roles_shared", "records", "TASK_BOARD.md");

  try {
    writeText(
      taskBoardPath,
      [
        "# Board",
        "",
        "## Ready for Dev",
        "",
        "## In Progress",
        "",
        "## Done",
        `- **[${wpId}]** - [VALIDATED]`,
        "",
      ].join("\n"),
    );
    writeText(
      packetPath,
      [
        `- WP_RUNTIME_STATUS_FILE: ${normalizePath(runtimeStatusPath)}`,
        `- WP_RECEIPTS_FILE: ${normalizePath(receiptsPath)}`,
        `- WP_THREAD_FILE: ${normalizePath(threadPath)}`,
        `- WP_COMMUNICATION_DIR: ${normalizePath(commDir)}`,
        "- PACKET_FORMAT_VERSION: 2026-03-29",
        "- WORKFLOW_LANE: ORCHESTRATOR_MANAGED",
        "- COMMUNICATION_CONTRACT: DIRECT_REVIEW_REQUIRED",
        "- COMMUNICATION_HEALTH_GATE: WP_COMMUNICATION_HEALTH_V1",
        "- **Status:** In Progress",
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
        currentPacketStatus: "Validated (PASS)",
        currentTaskBoardStatus: "DONE_VALIDATED",
        runtimeStatus: "completed",
        currentPhase: "STATUS_SYNC",
        nextExpectedActor: "NONE",
        waitingOn: "CLOSED",
        executionState: {
          schema_version: "wp_execution_state@1",
          authority: {
            packet_status: "Validated (PASS)",
            task_board_status: "DONE_VALIDATED",
            runtime_status: "completed",
            phase: "STATUS_SYNC",
            milestone: "CONTAINMENT",
            next_expected_actor: "NONE",
            next_expected_session: null,
            waiting_on: "CLOSED",
            waiting_on_session: null,
            main_containment_status: "CONTAINED_IN_MAIN",
            merged_main_commit: "abc123def456",
            route_anchor: {},
            review_anchor: {},
          },
          checkpoint_lineage: {
            schema_version: "wp_execution_checkpoint_lineage@1",
            latest_checkpoint_id: "cp-0001-task_board_sync-20990101t100000z",
            latest_checkpoint_at_utc: "2099-01-01T10:00:00Z",
            latest_checkpoint_kind: "PACKET_SYNC",
            latest_restore_point_id: "cp-0001-task_board_sync-20990101t100000z",
            latest_checkpoint_fingerprint: "{\"packet_status\":\"Validated (PASS)\",\"task_board_status\":\"DONE_VALIDATED\",\"milestone\":\"CONTAINMENT\",\"phase\":\"STATUS_SYNC\",\"runtime_status\":\"completed\",\"next_expected_actor\":\"NONE\",\"next_expected_session\":null,\"waiting_on\":\"CLOSED\",\"waiting_on_session\":null,\"route_anchor_state\":null,\"route_anchor_kind\":null,\"route_anchor_correlation_id\":null,\"route_anchor_target_role\":null,\"route_anchor_target_session\":null,\"review_anchor_kind\":null,\"review_anchor_correlation_id\":null,\"committed_handoff_base_sha\":null,\"committed_handoff_head_sha\":null}",
            checkpoint_count: 1,
            checkpoints: [],
          },
        },
      }), null, 2)}\n`,
    );
    writeText(receiptsPath, "\n");
    writeText(threadPath, "# thread\n");

    const firstResult = spawnSync(process.execPath, [taskBoardSetScript, wpId, "DONE_VALIDATED"], {
      cwd: repoRoot,
      env: {
        ...process.env,
        HANDSHAKE_GOV_ROOT: govRoot,
        HANDSHAKE_GOV_RUNTIME_ROOT: govRuntimeRoot,
      },
      encoding: "utf8",
    });

    assert.equal(firstResult.status, 0, firstResult.stderr || firstResult.stdout);

    const frozenTime = new Date("2001-02-03T04:05:06.000Z");
    fs.utimesSync(taskBoardPath, frozenTime, frozenTime);
    fs.utimesSync(runtimeStatusPath, frozenTime, frozenTime);
    const beforeBoardMtime = fs.statSync(taskBoardPath).mtimeMs;
    const beforeRuntimeMtime = fs.statSync(runtimeStatusPath).mtimeMs;

    const result = spawnSync(process.execPath, [taskBoardSetScript, wpId, "DONE_VALIDATED"], {
      cwd: repoRoot,
      env: {
        ...process.env,
        HANDSHAKE_GOV_ROOT: govRoot,
        HANDSHAKE_GOV_RUNTIME_ROOT: govRuntimeRoot,
      },
      encoding: "utf8",
    });

    assert.equal(result.status, 0, result.stderr || result.stdout);
    assert.equal(fs.statSync(taskBoardPath).mtimeMs, beforeBoardMtime);
    assert.equal(fs.statSync(runtimeStatusPath).mtimeMs, beforeRuntimeMtime);
    assert.match(result.stdout, /task_board_change: no-op/);
    assert.match(result.stdout, /runtime_change: no-op/);
  } finally {
    fs.rmSync(tempRoot, { recursive: true, force: true });
  }
});

test("task-board-set allows older failed sibling to become superseded history", () => {
  const tempRoot = fs.mkdtempSync(path.join(os.tmpdir(), "hsk-task-board-set-superseded-"));
  const govRoot = path.join(tempRoot, ".GOV");
  const govRuntimeRoot = path.join(tempRoot, "gov_runtime");
  const oldWpId = "WP-TEST-SUPERSEDE-v1";
  const activeWpId = "WP-TEST-SUPERSEDE-v2";
  const baseWpId = "WP-TEST-SUPERSEDE";
  const oldPacketPath = path.join(govRoot, "task_packets", oldWpId, "packet.md");
  const activePacketPath = path.join(govRoot, "task_packets", activeWpId, "packet.md");
  const commDir = path.join(govRuntimeRoot, "roles_shared", "WP_COMMUNICATIONS", oldWpId);
  const runtimeStatusPath = path.join(commDir, "RUNTIME_STATUS.json");
  const receiptsPath = path.join(commDir, "RECEIPTS.jsonl");
  const threadPath = path.join(commDir, "THREAD.md");

  try {
    writeText(
      path.join(govRoot, "roles_shared", "records", "TASK_BOARD.md"),
      [
        "# Board",
        "",
        "## Ready for Dev",
        `- **[${activeWpId}]** - [READY_FOR_DEV]`,
        "",
        "## Done",
        `- **[${oldWpId}]** - [FAIL]`,
        "",
        "## Superseded",
        "",
      ].join("\n"),
    );
    writeText(
      path.join(govRoot, "roles_shared", "records", "WP_TRACEABILITY_REGISTRY.md"),
      [
        "| Base WP ID | Active Packet | Task Board | Notes |",
        "|---|---|---|---|",
        `| ${baseWpId} | .GOV/task_packets/${activeWpId}/packet.md | Ready for Dev | active remediation |`,
        "",
      ].join("\n"),
    );
    writeText(
      oldPacketPath,
      [
        `- WP_ID: ${oldWpId}`,
        `- BASE_WP_ID: ${baseWpId}`,
        `- WP_RUNTIME_STATUS_FILE: ${normalizePath(runtimeStatusPath)}`,
        `- WP_RECEIPTS_FILE: ${normalizePath(receiptsPath)}`,
        `- WP_THREAD_FILE: ${normalizePath(threadPath)}`,
        `- WP_COMMUNICATION_DIR: ${normalizePath(commDir)}`,
        "- PACKET_FORMAT_VERSION: 2026-03-29",
        "- WORKFLOW_LANE: ORCHESTRATOR_MANAGED",
        "- COMMUNICATION_CONTRACT: DIRECT_REVIEW_REQUIRED",
        "- COMMUNICATION_HEALTH_GATE: WP_COMMUNICATION_HEALTH_V1",
        "- **Status:** Validated (FAIL)",
      ].join("\n"),
    );
    writeText(
      activePacketPath,
      [
        `- WP_ID: ${activeWpId}`,
        `- BASE_WP_ID: ${baseWpId}`,
        "- **Status:** Ready for Dev",
      ].join("\n"),
    );
    writeText(
      runtimeStatusPath,
      `${JSON.stringify(buildRuntimeStatus({
        wpId: oldWpId,
        packetPath: oldPacketPath,
        commDir,
        runtimeStatusPath,
        receiptsPath,
        threadPath,
        currentPacketStatus: "Validated (FAIL)",
        currentTaskBoardStatus: "DONE_FAIL",
        runtimeStatus: "completed",
        currentPhase: "STATUS_SYNC",
        nextExpectedActor: "NONE",
        waitingOn: "CLOSED",
        executionState: {
          schema_version: "wp_execution_state@1",
          authority: {
            packet_status: "Validated (FAIL)",
            task_board_status: "DONE_FAIL",
            runtime_status: "completed",
            phase: "STATUS_SYNC",
            milestone: "FAIL",
            next_expected_actor: "NONE",
            next_expected_session: null,
            waiting_on: "CLOSED",
            waiting_on_session: null,
            main_containment_status: "NOT_REQUIRED",
            merged_main_commit: null,
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

    const result = spawnSync(process.execPath, [taskBoardSetScript, oldWpId, "SUPERSEDED"], {
      cwd: repoRoot,
      env: {
        ...process.env,
        HANDSHAKE_GOV_ROOT: govRoot,
        HANDSHAKE_GOV_RUNTIME_ROOT: govRuntimeRoot,
      },
      encoding: "utf8",
    });

    assert.equal(result.status, 0, result.stderr || result.stdout);
    const taskBoard = fs.readFileSync(path.join(govRoot, "roles_shared", "records", "TASK_BOARD.md"), "utf8");
    assert.match(taskBoard, new RegExp(`\\*\\*\\[${oldWpId}\\]\\*\\* - \\[SUPERSEDED\\]`));
    assert.doesNotMatch(taskBoard, new RegExp(`\\*\\*\\[${oldWpId}\\]\\*\\* - \\[FAIL\\]`));
    const runtime = JSON.parse(fs.readFileSync(runtimeStatusPath, "utf8"));
    assert.equal(runtime.current_packet_status, "Validated (FAIL)");
    assert.equal(runtime.current_task_board_status, "DONE_FAIL");
    assert.match(result.stdout, /superseded_history_projection: allowed/);
    assert.match(result.stdout, /runtime_authority: canonical/);
  } finally {
    fs.rmSync(tempRoot, { recursive: true, force: true });
  }
});
