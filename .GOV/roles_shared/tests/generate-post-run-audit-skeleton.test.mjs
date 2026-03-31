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
