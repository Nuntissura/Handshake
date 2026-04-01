import assert from "node:assert/strict";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import test from "node:test";
import { defaultRegistry } from "../scripts/session/session-registry-lib.mjs";
import {
  SESSION_CONTROL_BROKER_STATE_FILE,
  SESSION_CONTROL_OUTPUT_DIR,
  SESSION_CONTROL_REQUESTS_FILE,
  SESSION_CONTROL_RESULTS_FILE,
  SESSION_REGISTRY_FILE,
} from "../scripts/session/session-policy.mjs";
import {
  buildSessionControlRequest,
} from "../scripts/session/session-control-lib.mjs";
import { settleRecoverableSessionControlResults } from "../scripts/session/session-control-self-settle-lib.mjs";

function writeJson(filePath, value) {
  fs.mkdirSync(path.dirname(filePath), { recursive: true });
  fs.writeFileSync(filePath, `${JSON.stringify(value, null, 2)}\n`, "utf8");
}

function appendJsonl(filePath, value) {
  fs.mkdirSync(path.dirname(filePath), { recursive: true });
  fs.appendFileSync(filePath, `${JSON.stringify(value)}\n`, "utf8");
}

function readJsonl(filePath) {
  if (!fs.existsSync(filePath)) return [];
  return fs.readFileSync(filePath, "utf8")
    .split(/\r?\n/)
    .map((line) => line.trim())
    .filter(Boolean)
    .map((line) => JSON.parse(line));
}

function tempRepoRoot() {
  const base = fs.mkdtempSync(path.join(os.tmpdir(), "handshake-self-settle-"));
  const repoRoot = path.join(base, "repo");
  fs.mkdirSync(repoRoot, { recursive: true });
  return repoRoot;
}

function seedRegistry(repoRoot, session) {
  const registry = defaultRegistry();
  registry.sessions.push(session);
  writeJson(path.resolve(repoRoot, SESSION_REGISTRY_FILE), registry);
}

function requestFixture(repoRoot, {
  commandId,
  wpId = "WP-TEST-RUNTIME-v1",
  role = "CODER",
  sessionKey = "CODER:WP-TEST-RUNTIME-v1",
  prompt = "Run just coder-next WP-TEST-RUNTIME-v1",
  outputJsonlFile = path.join(SESSION_CONTROL_OUTPUT_DIR, "CODER_WP-TEST-RUNTIME-v1", `${commandId}.jsonl`),
} = {}) {
  return buildSessionControlRequest({
    commandId,
    commandKind: "SEND_PROMPT",
    wpId,
    role,
    sessionKey,
    localBranch: "feat/WP-TEST-RUNTIME-v1",
    localWorktreeDir: "../wtc-test-runtime-v1",
    absWorktreeDir: path.resolve(repoRoot, "../wtc-test-runtime-v1"),
    selectedModel: "gpt-5.4",
    prompt,
    threadId: "thread-1",
    summary: "Steer CODER session for WP-TEST-RUNTIME-v1",
    outputJsonlFile,
  });
}

test("self-settlement writes a FAILED result when broker.rejected exists without a result row", () => {
  const repoRoot = tempRepoRoot();
  const commandId = "cmd-rejected";
  const request = requestFixture(repoRoot, { commandId });
  appendJsonl(path.resolve(repoRoot, SESSION_CONTROL_REQUESTS_FILE), request);
  seedRegistry(repoRoot, {
    session_key: request.session_key,
    session_id: "coder:wp-test-runtime-v1",
    wp_id: request.wp_id,
    role: request.role,
    local_branch: request.local_branch,
    local_worktree_dir: request.local_worktree_dir,
    session_thread_id: "thread-1",
    startup_proof_state: "READY",
    last_command_id: "",
    last_command_status: "NONE",
    last_command_output_file: "",
    runtime_state: "READY",
  });
  appendJsonl(path.resolve(repoRoot, request.output_jsonl_file), {
    timestamp: "2026-03-25T00:00:00.000Z",
    type: "broker.rejected",
    reason: "Concurrent governed run already active for CODER:WP-TEST-RUNTIME-v1",
  });

  const reconciliation = settleRecoverableSessionControlResults(repoRoot, {
    brokerState: { active_runs: [] },
  });

  assert.equal(reconciliation.settled.length, 1);
  const results = readJsonl(path.resolve(repoRoot, SESSION_CONTROL_RESULTS_FILE));
  assert.equal(results.length, 1);
  assert.equal(results[0].command_id, commandId);
  assert.equal(results[0].status, "FAILED");
  assert.match(results[0].summary, /Concurrent governed run already active/i);

  const outputEvents = readJsonl(path.resolve(repoRoot, request.output_jsonl_file));
  assert.ok(outputEvents.some((event) => event.type === "broker.self_settle"));
});

test("self-settlement mirrors session-registry terminal state when the result row is missing", () => {
  const repoRoot = tempRepoRoot();
  const commandId = "cmd-completed";
  const request = requestFixture(repoRoot, { commandId });
  appendJsonl(path.resolve(repoRoot, SESSION_CONTROL_REQUESTS_FILE), request);
  seedRegistry(repoRoot, {
    session_key: request.session_key,
    session_id: "coder:wp-test-runtime-v1",
    wp_id: request.wp_id,
    role: request.role,
    local_branch: request.local_branch,
    local_worktree_dir: request.local_worktree_dir,
    session_thread_id: "thread-1",
    startup_proof_state: "READY",
    last_command_id: commandId,
    last_command_status: "COMPLETED",
    last_command_output_file: request.output_jsonl_file,
    runtime_state: "READY",
  });

  const reconciliation = settleRecoverableSessionControlResults(repoRoot, {
    brokerState: { active_runs: [] },
  });

  assert.equal(reconciliation.settled.length, 1);
  const results = readJsonl(path.resolve(repoRoot, SESSION_CONTROL_RESULTS_FILE));
  assert.equal(results.length, 1);
  assert.equal(results[0].command_id, commandId);
  assert.equal(results[0].status, "COMPLETED");
  assert.match(results[0].summary, /Recovered missing terminal result from session registry state/i);
});

test("self-settlement prunes broker active runs that already have settled results", () => {
  const repoRoot = tempRepoRoot();
  const commandId = "cmd-stale-broker-run";
  const request = requestFixture(repoRoot, { commandId });
  appendJsonl(path.resolve(repoRoot, SESSION_CONTROL_REQUESTS_FILE), request);
  appendJsonl(path.resolve(repoRoot, SESSION_CONTROL_RESULTS_FILE), {
    schema_id: "hsk.session_control_result@1",
    schema_version: "session_control_result_v1",
    command_id: commandId,
    processed_at: "2026-04-01T00:00:00.000Z",
    command_kind: "SEND_PROMPT",
    session_key: request.session_key,
    wp_id: request.wp_id,
    role: request.role,
    status: "FAILED",
    thread_id: "thread-1",
    summary: "Recovered orphaned governed request.",
    output_jsonl_file: request.output_jsonl_file,
    last_agent_message: "",
    error: "orphaned run",
    duration_ms: 0,
    target_command_id: "",
    cancel_status: "",
    broker_build_id: "sha256:test",
  });
  writeJson(path.resolve(repoRoot, SESSION_CONTROL_BROKER_STATE_FILE), {
    schema_id: "hsk.session_control_broker_state@1",
    schema_version: "session_control_broker_state_v1",
    protocol: "HANDSHAKE_ACP_STDIO_V1",
    control_transport: "CODEX_EXEC_RESUME_JSON",
    host: "127.0.0.1",
    port: 65195,
    broker_pid: process.pid,
    started_at: "2026-04-01T00:00:00.000Z",
    updated_at: "2026-04-01T00:00:00.000Z",
    active_runs: [{
      command_id: commandId,
      session_key: request.session_key,
      wp_id: request.wp_id,
      role: request.role,
      command_kind: "SEND_PROMPT",
      child_pid: 1234,
      started_at: "2026-04-01T00:00:00.000Z",
      timeout_at: "2026-04-01T01:00:00.000Z",
      output_jsonl_file: request.output_jsonl_file,
      termination_reason: "",
    }],
    broker_build_id: "sha256:test",
    broker_auth_mode: "LOCAL_TOKEN_FILE_V1",
  });

  const reconciliation = settleRecoverableSessionControlResults(repoRoot, {
    brokerState: {
      active_runs: [{
        command_id: commandId,
      }],
    },
  });

  assert.ok(
    reconciliation.settled.some((entry) =>
      entry.command_id === commandId && entry.repair_reason === "stale_active_run_with_settled_result"),
  );

  const brokerState = JSON.parse(fs.readFileSync(path.resolve(repoRoot, SESSION_CONTROL_BROKER_STATE_FILE), "utf8"));
  assert.deepEqual(brokerState.active_runs, []);
});
