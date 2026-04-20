import assert from "node:assert/strict";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { spawnSync } from "node:child_process";
import test from "node:test";

function normalizePath(value) {
  return String(value || "").replace(/\\/g, "/");
}

function writeText(filePath, text = "") {
  fs.mkdirSync(path.dirname(filePath), { recursive: true });
  fs.writeFileSync(filePath, text, "utf8");
}

test("generate-post-run-audit-skeleton reads task board status from the packet governance root", () => {
  const repoRoot = fs.mkdtempSync(path.join(os.tmpdir(), "hsk-audit-skeleton-"));
  const govRoot = path.join(repoRoot, ".GOV");
  const govRuntimeRoot = path.join(repoRoot, "gov_runtime");
  const wpId = "WP-1-Test-v1";
  const packetPath = path.join(govRoot, "task_packets", wpId, "packet.md");
  const commDir = path.join(govRuntimeRoot, "roles_shared", "WP_COMMUNICATIONS", wpId);
  const runtimeStatusPath = path.join(commDir, "RUNTIME_STATUS.json");
  const receiptsPath = path.join(commDir, "RECEIPTS.jsonl");
  const threadPath = path.join(commDir, "THREAD.md");
  const outputPath = path.join(repoRoot, "audit.md");

  try {
    writeText(
      path.join(govRoot, "roles_shared", "records", "TASK_BOARD.md"),
      "# Board\n\n## Done\n- **[WP-1-Test-v1]** - [VALIDATED]\n",
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
        "- **Status:** Validated (PASS)",
      ].join("\n"),
    );
    writeText(
      runtimeStatusPath,
      `${JSON.stringify({
        schema_version: "wp_runtime_status@1",
        wp_id: wpId,
        current_packet_status: "Validated (PASS)",
        runtime_status: "completed",
        current_phase: "CLOSED",
        next_expected_actor: "NONE",
        open_review_items: [],
        last_event_at: "2026-03-31T10:00:00Z",
      }, null, 2)}\n`,
    );
    writeText(receiptsPath, "\n");
    writeText(threadPath, "# thread\n");

    const result = spawnSync(
      process.execPath,
      [".GOV/roles_shared/scripts/audit/generate-post-run-audit-skeleton.mjs", wpId, "--output", outputPath],
      {
        cwd: path.resolve("D:/Projects/LLM projects/Handshake/Handshake Worktrees/wt-gov-kernel"),
        env: {
          ...process.env,
          HANDSHAKE_GOV_ROOT: govRoot,
          HANDSHAKE_GOV_RUNTIME_ROOT: govRuntimeRoot,
        },
        encoding: "utf8",
      },
    );

    assert.equal(result.status, 0, result.stderr || result.stdout);
    const audit = fs.readFileSync(outputPath, "utf8");
    assert.match(audit, /- TASK_BOARD_STATUS: VALIDATED/);
  } finally {
    fs.rmSync(repoRoot, { recursive: true, force: true });
  }
});

test("generate-post-run-audit-skeleton prefers canonical execution publication status when runtime authority exists", () => {
  const repoRoot = fs.mkdtempSync(path.join(os.tmpdir(), "hsk-audit-skeleton-canonical-"));
  const govRoot = path.join(repoRoot, ".GOV");
  const govRuntimeRoot = path.join(repoRoot, "gov_runtime");
  const wpId = "WP-1-Test-v1";
  const packetPath = path.join(govRoot, "task_packets", wpId, "packet.md");
  const commDir = path.join(govRuntimeRoot, "roles_shared", "WP_COMMUNICATIONS", wpId);
  const runtimeStatusPath = path.join(commDir, "RUNTIME_STATUS.json");
  const receiptsPath = path.join(commDir, "RECEIPTS.jsonl");
  const threadPath = path.join(commDir, "THREAD.md");
  const outputPath = path.join(repoRoot, "audit.md");

  try {
    writeText(
      path.join(govRoot, "roles_shared", "records", "TASK_BOARD.md"),
      "# Board\n\n## Ready for Dev\n- **[WP-1-Test-v1]** - [READY_FOR_DEV]\n",
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
      `${JSON.stringify({
        schema_version: "wp_runtime_status@1",
        wp_id: wpId,
        current_packet_status: "In Progress",
        current_task_board_status: "IN_PROGRESS",
        runtime_status: "working",
        current_phase: "IMPLEMENTATION",
        next_expected_actor: "CODER",
        open_review_items: [],
        last_event_at: "2026-03-31T10:00:00Z",
        execution_state: {
          schema_version: "wp_execution_state@1",
          authority: {
            packet_status: "Validated (PASS)",
            task_board_status: "DONE_VALIDATED",
            runtime_status: "completed",
            phase: "STATUS_SYNC",
            next_expected_actor: "NONE",
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
      }, null, 2)}\n`,
    );
    writeText(receiptsPath, "\n");
    writeText(threadPath, "# thread\n");

    const result = spawnSync(
      process.execPath,
      [".GOV/roles_shared/scripts/audit/generate-post-run-audit-skeleton.mjs", wpId, "--output", outputPath],
      {
        cwd: path.resolve("D:/Projects/LLM projects/Handshake/Handshake Worktrees/wt-gov-kernel"),
        env: {
          ...process.env,
          HANDSHAKE_GOV_ROOT: govRoot,
          HANDSHAKE_GOV_RUNTIME_ROOT: govRuntimeRoot,
        },
        encoding: "utf8",
      },
    );

    assert.equal(result.status, 0, result.stderr || result.stdout);
    const audit = fs.readFileSync(outputPath, "utf8");
    assert.match(audit, /- TASK_BOARD_STATUS: DONE_VALIDATED/);
    assert.match(audit, /- PACKET_STATUS: Validated \(PASS\)/);
    assert.match(audit, /- RUNTIME_STATUS: completed/);
  } finally {
    fs.rmSync(repoRoot, { recursive: true, force: true });
  }
});

test("generate-post-run-audit-skeleton includes the last governed closeout action from validator gate provenance", () => {
  const repoRoot = fs.mkdtempSync(path.join(os.tmpdir(), "hsk-audit-skeleton-closeout-"));
  const govRoot = path.join(repoRoot, ".GOV");
  const govRuntimeRoot = path.join(repoRoot, "gov_runtime");
  const wpId = "WP-1-Test-v1";
  const packetPath = path.join(govRoot, "task_packets", wpId, "packet.md");
  const commDir = path.join(govRuntimeRoot, "roles_shared", "WP_COMMUNICATIONS", wpId);
  const runtimeStatusPath = path.join(commDir, "RUNTIME_STATUS.json");
  const receiptsPath = path.join(commDir, "RECEIPTS.jsonl");
  const threadPath = path.join(commDir, "THREAD.md");
  const gateStatePath = path.join(govRuntimeRoot, "roles_shared", "validator_gates", `${wpId}.json`);
  const outputPath = path.join(repoRoot, "audit.md");

  try {
    writeText(
      path.join(govRoot, "roles_shared", "records", "TASK_BOARD.md"),
      "# Board\n\n## Done\n- **[WP-1-Test-v1]** - [VALIDATED]\n",
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
        "- **Status:** Validated (PASS)",
      ].join("\n"),
    );
    writeText(
      runtimeStatusPath,
      `${JSON.stringify({
        schema_version: "wp_runtime_status@1",
        wp_id: wpId,
        current_packet_status: "Validated (PASS)",
        runtime_status: "completed",
        current_phase: "CLOSED",
        next_expected_actor: "NONE",
        open_review_items: [],
        last_event_at: "2026-03-31T10:00:00Z",
      }, null, 2)}\n`,
    );
    writeText(
      gateStatePath,
      `${JSON.stringify({
        closeout_sync_events: {
          [wpId]: [
            {
              timestamp_utc: "2026-04-20T10:15:00Z",
              mode: "CONTAINED_IN_MAIN",
              actor_role: "INTEGRATION_VALIDATOR",
              actor_session_id: "integration-validator-session",
              governed_action: {
                schema_id: "hsk.governed_action_result@1",
                schema_version: "governed_action_result_v1",
                action_id: "closeout-sync-action-1",
                processed_at: "2026-04-20T10:15:00Z",
                rule_id: "INTEGRATION_VALIDATOR_CLOSEOUT_SYNC_EXTERNAL_EXECUTE",
                action_kind: "EXTERNAL_EXECUTE",
                action_surface: "INTEGRATION_VALIDATOR_CLOSEOUT",
                command_kind: "CLOSEOUT_SYNC",
                command_id: "closeout-sync-action-1",
                session_key: "INTEGRATION_VALIDATOR:WP-1-Test-v1",
                wp_id: wpId,
                role: "INTEGRATION_VALIDATOR",
                status: "COMPLETED",
                outcome_state: "SETTLED",
                result_state: "SETTLED",
                resume_disposition: "CONSUME_RESULT",
                target_command_id: "",
                summary: "Integration Validator recorded closeout sync CONTAINED_IN_MAIN for WP-1-Test-v1.",
                error: "",
                metadata: {},
              },
            },
          ],
        },
      }, null, 2)}\n`,
    );
    writeText(receiptsPath, "\n");
    writeText(threadPath, "# thread\n");

    const result = spawnSync(
      process.execPath,
      [".GOV/roles_shared/scripts/audit/generate-post-run-audit-skeleton.mjs", wpId, "--output", outputPath],
      {
        cwd: path.resolve("D:/Projects/LLM projects/Handshake/Handshake Worktrees/wt-gov-kernel"),
        env: {
          ...process.env,
          HANDSHAKE_GOV_ROOT: govRoot,
          HANDSHAKE_GOV_RUNTIME_ROOT: govRuntimeRoot,
        },
        encoding: "utf8",
      },
    );

    assert.equal(result.status, 0, result.stderr || result.stdout);
    const audit = fs.readFileSync(outputPath, "utf8");
    assert.match(audit, /- LAST_GOVERNED_CLOSEOUT_ACTION: INTEGRATION_VALIDATOR_CLOSEOUT_SYNC_EXTERNAL_EXECUTE \| CONSUME_RESULT \| 2026-04-20T10:15:00Z/);
    assert.match(audit, /- LAST_CLOSEOUT_SYNC_MODE: CONTAINED_IN_MAIN/);
  } finally {
    fs.rmSync(repoRoot, { recursive: true, force: true });
  }
});

test("generate-post-run-audit-skeleton loads session control state from the packet governance root instead of caller cwd", () => {
  const repoRoot = fs.mkdtempSync(path.join(os.tmpdir(), "hsk-audit-skeleton-"));
  const unrelatedCwd = fs.mkdtempSync(path.join(os.tmpdir(), "hsk-audit-cwd-"));
  const govRoot = path.join(repoRoot, ".GOV");
  const govRuntimeRoot = path.join(repoRoot, "gov_runtime");
  const wpId = "WP-1-Test-v1";
  const packetPath = path.join(govRoot, "task_packets", wpId, "packet.md");
  const commDir = path.join(govRuntimeRoot, "roles_shared", "WP_COMMUNICATIONS", wpId);
  const runtimeStatusPath = path.join(commDir, "RUNTIME_STATUS.json");
  const receiptsPath = path.join(commDir, "RECEIPTS.jsonl");
  const threadPath = path.join(commDir, "THREAD.md");
  const outputPath = path.join(repoRoot, "audit.md");
  const sessionRegistryPath = path.join(govRuntimeRoot, "roles_shared", "ROLE_SESSION_REGISTRY.json");
  const controlRequestsPath = path.join(govRuntimeRoot, "roles_shared", "SESSION_CONTROL_REQUESTS.jsonl");
  const controlResultsPath = path.join(govRuntimeRoot, "roles_shared", "SESSION_CONTROL_RESULTS.jsonl");

  try {
    writeText(
      path.join(govRoot, "roles_shared", "records", "TASK_BOARD.md"),
      "# Board\n\n## Done\n- **[WP-1-Test-v1]** - [VALIDATED]\n",
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
        "- **Status:** Validated (PASS)",
      ].join("\n"),
    );
    writeText(
      runtimeStatusPath,
      `${JSON.stringify({
        schema_version: "wp_runtime_status@1",
        wp_id: wpId,
        current_packet_status: "Validated (PASS)",
        runtime_status: "completed",
        current_phase: "CLOSED",
        next_expected_actor: "NONE",
        open_review_items: [],
        last_event_at: "2026-03-31T10:00:00Z",
      }, null, 2)}\n`,
    );
    writeText(
      sessionRegistryPath,
      `${JSON.stringify({
        schema_id: "hsk.role_session_registry@1",
        schema_version: "role_session_registry_v1",
        updated_at: "2026-03-31T10:05:00Z",
        sessions: [
          {
            session_key: "CODER:WP-1-Test-v1",
            role: "CODER",
            wp_id: wpId,
            runtime_state: "ACTIVE",
            active_host: "CLI",
            session_thread_id: "thread-1",
            last_command_kind: "SEND_PROMPT",
            last_command_status: "SUCCEEDED"
          }
        ]
      }, null, 2)}\n`,
    );
    writeText(controlRequestsPath, `${JSON.stringify({ wp_id: wpId, role: "CODER", command_kind: "START_SESSION", created_at: "2026-03-31T10:01:00Z" })}\n`);
    writeText(controlResultsPath, `${JSON.stringify({ wp_id: wpId, role: "CODER", command_kind: "START_SESSION", status: "SUCCEEDED", processed_at: "2026-03-31T10:02:00Z" })}\n`);
    writeText(receiptsPath, "\n");
    writeText(threadPath, "# thread\n");

    const result = spawnSync(
      process.execPath,
      [
        path.resolve("D:/Projects/LLM projects/Handshake/Handshake Worktrees/wt-gov-kernel/.GOV/roles_shared/scripts/audit/generate-post-run-audit-skeleton.mjs"),
        wpId,
        "--output",
        outputPath,
      ],
      {
        cwd: unrelatedCwd,
        env: {
          ...process.env,
          HANDSHAKE_GOV_ROOT: govRoot,
          HANDSHAKE_GOV_RUNTIME_ROOT: govRuntimeRoot,
        },
        encoding: "utf8",
      },
    );

    assert.equal(result.status, 0, result.stderr || result.stdout);
    const audit = fs.readFileSync(outputPath, "utf8");
    assert.match(audit, /- GOVERNED_SESSION_COUNT: 1/);
    assert.match(audit, /- CONTROL_REQUEST_COUNT: 1/);
    assert.match(audit, /- CONTROL_RESULT_COUNT: 1/);
  } finally {
    fs.rmSync(unrelatedCwd, { recursive: true, force: true });
    fs.rmSync(repoRoot, { recursive: true, force: true });
  }
});

test("generate-post-run-audit-skeleton emits typed failure-ledger and positive-control placeholders", () => {
  const repoRoot = fs.mkdtempSync(path.join(os.tmpdir(), "hsk-audit-skeleton-shape-"));
  const govRoot = path.join(repoRoot, ".GOV");
  const govRuntimeRoot = path.join(repoRoot, "gov_runtime");
  const wpId = "WP-1-Test-v1";
  const packetPath = path.join(govRoot, "task_packets", wpId, "packet.md");
  const commDir = path.join(govRuntimeRoot, "roles_shared", "WP_COMMUNICATIONS", wpId);
  const runtimeStatusPath = path.join(commDir, "RUNTIME_STATUS.json");
  const receiptsPath = path.join(commDir, "RECEIPTS.jsonl");
  const threadPath = path.join(commDir, "THREAD.md");
  const outputPath = path.join(repoRoot, "audit.md");

  try {
    writeText(
      path.join(govRoot, "roles_shared", "records", "TASK_BOARD.md"),
      "# Board\n\n## Done\n- **[WP-1-Test-v1]** - [VALIDATED]\n",
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
        "- **Status:** Validated (PASS)",
      ].join("\n"),
    );
    writeText(
      runtimeStatusPath,
      `${JSON.stringify({
        schema_version: "wp_runtime_status@1",
        wp_id: wpId,
        current_packet_status: "Validated (PASS)",
        runtime_status: "completed",
        current_phase: "CLOSED",
        next_expected_actor: "NONE",
        open_review_items: [],
        last_event_at: "2026-03-31T10:00:00Z",
      }, null, 2)}\n`,
    );
    writeText(receiptsPath, "\n");
    writeText(threadPath, "# thread\n");

    const result = spawnSync(
      process.execPath,
      [".GOV/roles_shared/scripts/audit/generate-post-run-audit-skeleton.mjs", wpId, "--output", outputPath],
      {
        cwd: path.resolve("D:/Projects/LLM projects/Handshake/Handshake Worktrees/wt-gov-kernel"),
        env: {
          ...process.env,
          HANDSHAKE_GOV_ROOT: govRoot,
          HANDSHAKE_GOV_RUNTIME_ROOT: govRuntimeRoot,
        },
        encoding: "utf8",
      },
    );

    assert.equal(result.status, 0, result.stderr || result.stdout);
    const audit = fs.readFileSync(outputPath, "utf8");
    assert.match(audit, /- ROLE_OWNER: SHARED/);
    assert.match(audit, /- SYSTEM_SCOPE: CONTROL_PLANE/);
    assert.match(audit, /- FAILURE_CLASS: UX_AMBIGUITY/);
    assert.match(audit, /- CONTROL_TYPE: REGRESSION_GUARD/);
    assert.match(audit, /- REGRESSION_GUARDS:/);
    assert.match(audit, /## Failure Classification Summary/);
  } finally {
    fs.rmSync(repoRoot, { recursive: true, force: true });
  }
});

test("generate-post-run-audit-skeleton live mode seeds a Brussels-local workflow dossier with ACP snapshot data", () => {
  const repoRoot = fs.mkdtempSync(path.join(os.tmpdir(), "hsk-live-review-"));
  const govRoot = path.join(repoRoot, ".GOV");
  const govRuntimeRoot = path.join(repoRoot, "gov_runtime");
  const wpId = "WP-1-Test-v1";
  const packetPath = path.join(govRoot, "task_packets", wpId, "packet.md");
  const commDir = path.join(govRuntimeRoot, "roles_shared", "WP_COMMUNICATIONS", wpId);
  const runtimeStatusPath = path.join(commDir, "RUNTIME_STATUS.json");
  const receiptsPath = path.join(commDir, "RECEIPTS.jsonl");
  const threadPath = path.join(commDir, "THREAD.md");
  const sessionRegistryPath = path.join(govRuntimeRoot, "roles_shared", "ROLE_SESSION_REGISTRY.json");
  const controlRequestsPath = path.join(govRuntimeRoot, "roles_shared", "SESSION_CONTROL_REQUESTS.jsonl");
  const controlResultsPath = path.join(govRuntimeRoot, "roles_shared", "SESSION_CONTROL_RESULTS.jsonl");
  const brokerStatePath = path.join(govRuntimeRoot, "roles_shared", "SESSION_CONTROL_BROKER_STATE.json");
  const sessionMarkerPath = path.join(govRuntimeRoot, "roles_shared", "CURRENT_REPOMEM_SESSION.json");

  try {
    writeText(
      path.join(govRoot, "roles_shared", "records", "TASK_BOARD.md"),
      "# Board\n\n## Ready\n- **[WP-1-Test-v1]** - [READY_FOR_DEV]\n",
    );
    writeText(
      packetPath,
      [
        `- WP_RUNTIME_STATUS_FILE: ${normalizePath(runtimeStatusPath)}`,
        `- WP_RECEIPTS_FILE: ${normalizePath(receiptsPath)}`,
        `- WP_THREAD_FILE: ${normalizePath(threadPath)}`,
        `- WP_COMMUNICATION_DIR: ${normalizePath(commDir)}`,
        "- PACKET_FORMAT_VERSION: 2026-04-06",
        "- WORKFLOW_LANE: ORCHESTRATOR_MANAGED",
        "- EXECUTION_OWNER: Coder-A",
        "- COMMUNICATION_CONTRACT: DIRECT_REVIEW_REQUIRED",
        "- COMMUNICATION_HEALTH_GATE: WP_COMMUNICATION_HEALTH_V1",
        "- **Status:** Ready for Dev",
      ].join("\n"),
    );
    writeText(
      runtimeStatusPath,
      `${JSON.stringify({
        schema_version: "wp_runtime_status@1",
        wp_id: wpId,
        current_packet_status: "Ready for Dev",
        runtime_status: "pending",
        current_phase: "STARTUP",
        next_expected_actor: "ACTIVATION_MANAGER",
        open_review_items: [],
        last_event_at: "2026-04-10T17:35:00Z",
      }, null, 2)}\n`,
    );
    writeText(
      sessionRegistryPath,
      `${JSON.stringify({
        schema_id: "hsk.role_session_registry@1",
        schema_version: "role_session_registry_v1",
        updated_at: "2026-04-10T17:36:00Z",
        sessions: [
          {
            session_key: "ACTIVATION_MANAGER:WP-1-Test-v1",
            role: "ACTIVATION_MANAGER",
            wp_id: wpId,
            runtime_state: "READY",
            active_host: "HANDSHAKE_ACP_BROKER",
            session_thread_id: "thread-live-1",
            last_command_kind: "START_SESSION",
            last_command_status: "SUCCEEDED"
          }
        ]
      }, null, 2)}\n`,
    );
    writeText(controlRequestsPath, `${JSON.stringify({ wp_id: wpId, role: "ACTIVATION_MANAGER", command_kind: "START_SESSION", created_at: "2026-04-10T17:36:30Z" })}\n`);
    writeText(controlResultsPath, `${JSON.stringify({ wp_id: wpId, role: "ACTIVATION_MANAGER", command_kind: "START_SESSION", status: "COMPLETED", processed_at: "2026-04-10T17:36:40Z" })}\n`);
    writeText(
      brokerStatePath,
      `${JSON.stringify({
        schema_id: "hsk.session_control_broker_state@1",
        schema_version: "session_control_broker_state_v1",
        broker_build_id: "test-build-id",
        broker_auth_mode: "LOCAL_TOKEN_FILE_V1",
        host: "127.0.0.1",
        port: 9876,
        broker_pid: 43210,
        updated_at: "2026-04-10T17:36:45Z",
        active_runs: [
          {
            command_id: "cmd-1",
            session_key: "ACTIVATION_MANAGER:WP-1-Test-v1",
            wp_id: wpId,
            role: "ACTIVATION_MANAGER",
            command_kind: "SEND_PROMPT",
            started_at: "2026-04-10T17:36:41Z",
            timeout_at: "2026-04-10T18:36:41Z"
          }
        ]
      }, null, 2)}\n`,
    );
    writeText(
      sessionMarkerPath,
      `${JSON.stringify({
        session_id: "ORCHESTRATOR-20260410-173700",
        role: "ORCHESTRATOR",
        opened_at: "2026-04-10T17:37:00Z",
        topic: "reduce workflow downtime, reduce governance document drift at close out, reduce token cost aggressively where possible",
      }, null, 2)}\n`,
    );
    writeText(receiptsPath, `${JSON.stringify({ receipt_kind: "WP-NOTIFICATION", timestamp_utc: "2026-04-10T17:36:50Z" })}\n`);
    writeText(threadPath, "# thread\n");
    writeText(path.join(govRoot, "task_packets", wpId, "MT-001.md"), "# MT-001\n");

    const result = spawnSync(
      process.execPath,
      [".GOV/roles_shared/scripts/audit/generate-post-run-audit-skeleton.mjs", wpId, "--mode", "live", "--auto-output"],
      {
        cwd: path.resolve("D:/Projects/LLM projects/Handshake/Handshake Worktrees/wt-gov-kernel"),
        env: {
          ...process.env,
          HANDSHAKE_GOV_ROOT: govRoot,
          HANDSHAKE_GOV_RUNTIME_ROOT: govRuntimeRoot,
        },
        encoding: "utf8",
      },
    );

    assert.equal(result.status, 0, result.stderr || result.stdout);
    const relativeOutputPath = result.stdout.trim();
    assert.match(relativeOutputPath, /\.GOV\/Audits\/smoketest\/DOSSIER_.*_WORKFLOW_DOSSIER\.md$/);
    const liveReview = fs.readFileSync(path.join(repoRoot, relativeOutputPath), "utf8");
    assert.match(liveReview, /# DOSSIER_\d{8}_TEST_WORKFLOW_DOSSIER/);
    assert.match(liveReview, /- WORKFLOW_DOSSIER_ID: WORKFLOW-DOSSIER-\d{8}-TEST/);
    assert.match(liveReview, /- DOCUMENT_KIND: LIVE_WORKFLOW_DOSSIER/);
    assert.match(liveReview, /- LIVE_REVIEW_STATUS: OPEN/);
    assert.match(liveReview, /- REPO_TIMEZONE: Europe\/Brussels/);
    assert.match(liveReview, /- SESSION_INTENTION: reduce workflow downtime, reduce governance document drift at close out, reduce token cost aggressively where possible/);
    assert.match(liveReview, /- BROKER_ACTIVE_RUN_COUNT: 1/);
    assert.match(liveReview, /- CONTROL_REQUEST_COUNT: 1/);
    assert.match(liveReview, /- CONTROL_RESULT_COUNT: 1/);
    assert.match(liveReview, /## Workflow Dossier Closeout Rubric/);
    assert.match(liveReview, /\| MT-001 \| <pending> \| NONE \| NOT_SENT \| N\/A \| N\/A \| NO \| 0 \|/);
  } finally {
    fs.rmSync(repoRoot, { recursive: true, force: true });
  }
});
