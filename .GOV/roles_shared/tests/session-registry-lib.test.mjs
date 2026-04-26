import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import assert from "node:assert/strict";
import test from "node:test";
import { spawn } from "node:child_process";
import { fileURLToPath, pathToFileURL } from "node:url";
import {
  appendJsonlLine,
  defaultRegistry,
  enqueuePendingSessionControlRequest,
  getOrCreateSessionRecord,
  loadSessionRegistry,
  markSessionCommandQueued,
  markSessionCommandRunning,
  markSessionCommandResult,
  markCliEscalationUsed,
  markPluginResult,
  parseJsonlFile,
  registryBatchLaunchSummary,
  registrySessionSummary,
  resetBatchLaunchMode,
  saveSessionRegistry,
  sessionRegistryLockPath,
} from "../scripts/session/session-registry-lib.mjs";
import { SESSION_REGISTRY_FILE } from "../scripts/session/session-policy.mjs";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const sessionRegistryLibUrl = pathToFileURL(path.resolve(__dirname, "../scripts/session/session-registry-lib.mjs")).href;

function makeTempRepoRoot(prefix) {
  return fs.mkdtempSync(path.join(os.tmpdir(), prefix));
}

function removeTree(targetPath) {
  fs.rmSync(targetPath, { recursive: true, force: true });
}

function runNodeProcess(args) {
  return new Promise((resolve, reject) => {
    const child = spawn(process.execPath, args, {
      stdio: ["ignore", "pipe", "pipe"],
      env: { ...process.env },
    });
    let stderr = "";
    child.stderr.on("data", (chunk) => {
      stderr += chunk.toString();
    });
    child.on("error", reject);
    child.on("exit", (code) => {
      if (code === 0) {
        resolve();
        return;
      }
      reject(new Error(`child exited ${code}: ${stderr.trim()}`));
    });
  });
}

test("saveSessionRegistry removes stale temp siblings and leaves no lock residue", () => {
  const repoRoot = makeTempRepoRoot("handshake-session-registry-");
  try {
    const registryPath = path.resolve(repoRoot, SESSION_REGISTRY_FILE);
    fs.mkdirSync(path.dirname(registryPath), { recursive: true });
    const staleTempPath = `${registryPath}.12345.stale.tmp`;
    fs.writeFileSync(staleTempPath, "{\"stale\":true}\n", "utf8");
    const staleDate = new Date(Date.now() - (10 * 60 * 1000));
    fs.utimesSync(staleTempPath, staleDate, staleDate);

    saveSessionRegistry(repoRoot, defaultRegistry());

    assert.equal(fs.existsSync(registryPath), true, "registry file should be written");
    assert.equal(fs.existsSync(staleTempPath), false, "stale temp sibling should be cleaned up");
    assert.equal(fs.existsSync(sessionRegistryLockPath(repoRoot)), false, "lock file should not remain after save");
  } finally {
    removeTree(repoRoot);
  }
});

test("appendJsonlLine preserves all entries across concurrent writers", async () => {
  const repoRoot = makeTempRepoRoot("handshake-session-ledger-");
  const ledgerPath = path.join(repoRoot, "gov_runtime", "roles_shared", "SESSION_CONTROL_REQUESTS.jsonl");
  const writerCount = 4;
  const linesPerWriter = 25;

  try {
    const script = [
      "const [moduleUrl, ledgerPath, writerId, count] = process.argv.slice(1);",
      "const { appendJsonlLine } = await import(moduleUrl);",
      "for (let index = 0; index < Number(count); index += 1) {",
      "  appendJsonlLine(ledgerPath, { writer: Number(writerId), seq: index });",
      "}",
    ].join("\n");

    await Promise.all(
      Array.from({ length: writerCount }, (_, writerIndex) =>
        runNodeProcess([
          "--input-type=module",
          "-e",
          script,
          sessionRegistryLibUrl,
          ledgerPath,
          String(writerIndex),
          String(linesPerWriter),
        ]),
      ),
    );

    const rows = parseJsonlFile(ledgerPath);
    assert.equal(rows.length, writerCount * linesPerWriter, "all writer entries should persist");
    assert.equal(new Set(rows.map((row) => `${row.writer}:${row.seq}`)).size, rows.length, "entries should remain unique");
    assert.equal(fs.existsSync(`${ledgerPath}.lock`), false, "ledger lock file should not remain after concurrent appends");
  } finally {
    removeTree(repoRoot);
  }
});

test("batch launch mode flips after repeated plugin failures and can be reset", () => {
  const registry = defaultRegistry();
  const initialTerminalBatchId = registry.active_terminal_batch_id;
  const session = getOrCreateSessionRecord(registry, {
    wp_id: "WP-TEST",
    role: "CODER",
    local_branch: "feat/WP-TEST",
    local_worktree_dir: "../wtc-test",
    terminal_title: "CODER WP-TEST",
  });

  markPluginResult(registry, session, "req-1", "PLUGIN_FAILED", { error: "bridge failed once" });
  let batchSummary = registryBatchLaunchSummary(registry);
  assert.equal(batchSummary.launch_batch_mode, "PLUGIN_FIRST");
  assert.equal(batchSummary.launch_batch_plugin_failure_count, 1);

  markPluginResult(registry, session, "req-2", "PLUGIN_TIMED_OUT", { error: "bridge timed out twice" });
  batchSummary = registryBatchLaunchSummary(registry);
  assert.equal(batchSummary.launch_batch_mode, "CLI_ESCALATION_BATCH");
  assert.equal(batchSummary.launch_batch_plugin_failure_count, 2);
  assert.equal(Boolean(batchSummary.launch_batch_switched_at), true);
  assert.match(batchSummary.launch_batch_switch_reason, /plugin instability reached 2\/2/i);

  resetBatchLaunchMode(registry, "new governed batch");
  batchSummary = registryBatchLaunchSummary(registry);
  assert.equal(batchSummary.launch_batch_mode, "PLUGIN_FIRST");
  assert.equal(batchSummary.launch_batch_plugin_failure_count, 0);
  assert.equal(Boolean(batchSummary.launch_batch_last_reset_at), true);
  assert.equal(batchSummary.launch_batch_switch_reason, "new governed batch");
  assert.match(batchSummary.active_terminal_batch_id, /^TBATCH-/);
  assert.notEqual(batchSummary.active_terminal_batch_id, initialTerminalBatchId);
});

test("loadSessionRegistry derives a stable terminal batch id for legacy registries missing the new fields", () => {
  const repoRoot = makeTempRepoRoot("handshake-legacy-terminal-batch-");
  try {
    const registryPath = path.resolve(repoRoot, SESSION_REGISTRY_FILE);
    fs.mkdirSync(path.dirname(registryPath), { recursive: true });
    fs.writeFileSync(registryPath, JSON.stringify({
      ...defaultRegistry(),
      active_terminal_batch_id: undefined,
      active_terminal_batch_started_at: undefined,
      active_terminal_batch_last_rotated_at: undefined,
      active_terminal_batch_claimed_at: undefined,
      active_terminal_batch_reason: undefined,
      terminal_batch_scope: undefined,
    }, null, 2));

    const first = loadSessionRegistry(repoRoot).registry;
    const second = loadSessionRegistry(repoRoot).registry;
    assert.match(first.active_terminal_batch_id, /^TBATCH-/);
    assert.equal(first.active_terminal_batch_id, second.active_terminal_batch_id);
  } finally {
    removeTree(repoRoot);
  }
});

test("getOrCreateSessionRecord preserves original authority fields for existing sessions", () => {
  const registry = defaultRegistry();
  const session = getOrCreateSessionRecord(registry, {
    wp_id: "WP-TEST",
    role: "CODER",
    local_branch: "feat/WP-TEST-original",
    local_worktree_dir: "../wt-original",
    terminal_title: "CODER WP-TEST",
    requested_model: "gpt-5.4",
  });

  const reopened = getOrCreateSessionRecord(registry, {
    wp_id: "WP-TEST",
    role: "CODER",
    local_branch: "feat/WP-TEST-rewritten",
    local_worktree_dir: "../wt-rewritten",
    terminal_title: "CODER WP-TEST",
    requested_model: "gpt-5.2",
  });

  assert.equal(reopened, session);
  assert.equal(reopened.local_branch, "feat/WP-TEST-original");
  assert.equal(reopened.local_worktree_dir, "../wt-original");
  assert.equal(reopened.requested_model, "gpt-5.4");
});

test("CLI escalation keeps governance path context and marks startup as requested", () => {
  const registry = defaultRegistry();
  const session = getOrCreateSessionRecord(registry, {
    wp_id: "WP-TEST",
    role: "CODER",
    local_branch: "feat/WP-TEST",
    local_worktree_dir: "../wtc-test",
    terminal_title: "CODER WP-TEST",
  });

  markCliEscalationUsed(session, {
    hostKind: "SYSTEM_TERMINAL",
    terminalTitle: "CODER WP-TEST",
  });

  const summary = registrySessionSummary(session);
  assert.equal(summary.runtime_state, "CLI_ESCALATION_USED");
  assert.equal(summary.startup_proof_state, "START_REQUESTED");
  assert.equal(summary.local_branch, "feat/WP-TEST");
  assert.equal(summary.local_worktree_dir, "../wtc-test");
  assert.equal(summary.active_terminal_kind, "SYSTEM_TERMINAL");
});

test("new session records default ACP health fields and expose them through the registry summary", () => {
  const registry = defaultRegistry();
  const session = getOrCreateSessionRecord(registry, {
    wp_id: "WP-TEST",
    role: "WP_VALIDATOR",
    local_branch: "feat/WP-TEST",
    local_worktree_dir: "../wtv-test",
    terminal_title: "WP_VALIDATOR WP-TEST",
  });

  const summary = registrySessionSummary(session);
  assert.equal(summary.health_state, "UNKNOWN");
  assert.equal(summary.health_reason_code, "UNKNOWN");
  assert.equal(summary.health_source, "ACP_WATCHDOG_V1");
  assert.equal(summary.health_updated_at, "");
});

test("session command projection keeps a bounded governed action history alongside last_command fields", () => {
  const registry = defaultRegistry();
  const session = getOrCreateSessionRecord(registry, {
    wp_id: "WP-TEST",
    role: "CODER",
    local_branch: "feat/WP-TEST",
    local_worktree_dir: "../wtc-test",
    terminal_title: "CODER WP-TEST",
  });

  const command = {
    command_id: "33333333-3333-3333-3333-333333333333",
    command_kind: "SEND_PROMPT",
    created_at: "2026-04-20T10:00:00Z",
    summary: "Resume the coder lane.",
    output_jsonl_file: "gov_runtime/roles_shared/SESSION_CONTROL_OUTPUTS/CODER_WP-TEST/333.jsonl",
    governed_action: {
      schema_id: "hsk.governed_action_request@1",
      schema_version: "governed_action_request_v1",
      action_id: "33333333-3333-3333-3333-333333333333",
      requested_at: "2026-04-20T10:00:00Z",
      rule_id: "SESSION_CONTROL_SEND_PROMPT_EXTERNAL_EXECUTE",
      action_kind: "EXTERNAL_EXECUTE",
      action_surface: "SESSION_CONTROL",
      command_kind: "SEND_PROMPT",
      command_id: "33333333-3333-3333-3333-333333333333",
      created_by_role: "ORCHESTRATOR",
      session_key: "CODER:WP-TEST",
      wp_id: "WP-TEST",
      role: "CODER",
      target_command_id: "",
      reason_code: "SEND_PROMPT",
      summary: "Resume the coder lane.",
      resume_policy: "WAIT_FOR_TRANSPORT_RESULT",
      metadata: {},
    },
  };
  markSessionCommandQueued(session, command);
  markSessionCommandResult(session, {
    command_id: command.command_id,
    command_kind: command.command_kind,
    processed_at: "2026-04-20T10:01:00Z",
    status: "COMPLETED",
    outcome_state: "SETTLED",
    summary: "Coder lane resumed.",
    output_jsonl_file: command.output_jsonl_file,
    thread_id: "thread_test",
    governed_action: {
      schema_id: "hsk.governed_action_result@1",
      schema_version: "governed_action_result_v1",
      action_id: "33333333-3333-3333-3333-333333333333",
      processed_at: "2026-04-20T10:01:00Z",
      rule_id: "SESSION_CONTROL_SEND_PROMPT_EXTERNAL_EXECUTE",
      action_kind: "EXTERNAL_EXECUTE",
      action_surface: "SESSION_CONTROL",
      command_kind: "SEND_PROMPT",
      command_id: "33333333-3333-3333-3333-333333333333",
      session_key: "CODER:WP-TEST",
      wp_id: "WP-TEST",
      role: "CODER",
      status: "COMPLETED",
      outcome_state: "SETTLED",
      result_state: "SETTLED",
      resume_disposition: "CONSUME_RESULT",
      target_command_id: "",
      summary: "Coder lane resumed.",
      error: "",
      metadata: {},
    },
  });

  const summary = registrySessionSummary(session);
  assert.equal(summary.last_command_status, "COMPLETED");
  assert.equal(summary.last_governed_action.rule_id, "SESSION_CONTROL_SEND_PROMPT_EXTERNAL_EXECUTE");
  assert.equal(summary.last_governed_action.action_state, "SETTLED");
  assert.equal(summary.last_governed_action.outcome_state, "SETTLED");
  assert.equal(summary.last_governed_action.resume_disposition, "CONSUME_RESULT");
  assert.equal(summary.effective_governed_action.rule_id, "SESSION_CONTROL_SEND_PROMPT_EXTERNAL_EXECUTE");
  assert.equal(summary.effective_governed_action.command_kind, "SEND_PROMPT");
  assert.equal(summary.effective_governed_action.status, "COMPLETED");
  assert.equal(summary.effective_governed_action.outcome_state, "SETTLED");
  assert.equal(summary.effective_governed_action.source, "governed_action");
  assert.equal(summary.action_history.length, 1);
});

test("session registry summary exposes pending queued control requests without replacing the effective action", () => {
  const registry = defaultRegistry();
  const session = getOrCreateSessionRecord(registry, {
    wp_id: "WP-TEST",
    role: "CODER",
    local_branch: "feat/WP-TEST",
    local_worktree_dir: "../wtc-test",
    terminal_title: "CODER WP-TEST",
  });

  session.last_command_id = "active-command";
  session.last_command_kind = "SEND_PROMPT";
  session.last_command_status = "RUNNING";
  session.last_command_summary = "Active governed prompt is still running.";

  enqueuePendingSessionControlRequest(session, {
    command_id: "queued-command",
    command_kind: "SEND_PROMPT",
    target_command_id: "",
    summary: "Queued follow-up prompt.",
    output_jsonl_file: "gov_runtime/roles_shared/SESSION_CONTROL_OUTPUTS/CODER_WP-TEST/queued.jsonl",
    busy_ingress_mode: "ENQUEUE_ON_BUSY",
    governed_action: {
      schema_id: "hsk.governed_action_request@1",
      schema_version: "governed_action_request_v1",
      action_id: "queued-command",
      requested_at: "2026-04-20T10:02:00Z",
      rule_id: "SESSION_CONTROL_SEND_PROMPT_EXTERNAL_EXECUTE",
      action_kind: "EXTERNAL_EXECUTE",
      action_surface: "SESSION_CONTROL",
      command_kind: "SEND_PROMPT",
      command_id: "queued-command",
      created_by_role: "ORCHESTRATOR",
      session_key: "CODER:WP-TEST",
      wp_id: "WP-TEST",
      role: "CODER",
      reason_code: "SEND_PROMPT",
      summary: "Queued follow-up prompt.",
      resume_policy: "WAIT_FOR_TRANSPORT_RESULT",
      metadata: {},
    },
  }, {
    queueReasonCode: "BUSY_ACTIVE_RUN",
    blockingCommandId: "active-command",
    queuedAt: "2026-04-20T10:02:00Z",
  });

  const summary = registrySessionSummary(session);
  assert.equal(summary.pending_control_queue_count, 1);
  assert.equal(summary.next_queued_control_request.command_id, "queued-command");
  assert.equal(summary.next_queued_control_request.outcome_state, "ACCEPTED_QUEUED");
  assert.equal(summary.next_queued_control_request.queue_reason_code, "BUSY_ACTIVE_RUN");
  assert.equal(summary.effective_governed_action.command_id, "active-command");
  assert.equal(summary.effective_governed_action.status, "RUNNING");
});

test("queued and running command projections now preserve distinct accepted outcome states", () => {
  const registry = defaultRegistry();
  const session = getOrCreateSessionRecord(registry, {
    wp_id: "WP-TEST",
    role: "CODER",
    local_branch: "feat/WP-TEST",
    local_worktree_dir: "../wtc-test",
    terminal_title: "CODER WP-TEST",
  });

  const queuedCommand = {
    command_id: "44444444-4444-4444-4444-444444444444",
    command_kind: "SEND_PROMPT",
    created_at: "2026-04-20T10:00:00Z",
    summary: "Queued governed follow-up.",
    output_jsonl_file: "gov_runtime/roles_shared/SESSION_CONTROL_OUTPUTS/CODER_WP-TEST/444.jsonl",
    governed_action: {
      schema_id: "hsk.governed_action_request@1",
      schema_version: "governed_action_request_v1",
      action_id: "44444444-4444-4444-4444-444444444444",
      requested_at: "2026-04-20T10:00:00Z",
      rule_id: "SESSION_CONTROL_SEND_PROMPT_EXTERNAL_EXECUTE",
      action_kind: "EXTERNAL_EXECUTE",
      action_surface: "SESSION_CONTROL",
      command_kind: "SEND_PROMPT",
      command_id: "44444444-4444-4444-4444-444444444444",
      created_by_role: "ORCHESTRATOR",
      session_key: "CODER:WP-TEST",
      wp_id: "WP-TEST",
      role: "CODER",
      reason_code: "SEND_PROMPT",
      summary: "Queued governed follow-up.",
      resume_policy: "WAIT_FOR_TRANSPORT_RESULT",
      metadata: {},
    },
  };

  markSessionCommandQueued(session, queuedCommand);
  let summary = registrySessionSummary(session);
  assert.equal(summary.last_governed_action.action_state, "ACCEPTED_QUEUED");
  assert.equal(summary.last_governed_action.outcome_state, "ACCEPTED_QUEUED");

  markSessionCommandRunning(session, queuedCommand);
  summary = registrySessionSummary(session);
  assert.equal(summary.last_governed_action.action_state, "ACCEPTED_RUNNING");
  assert.equal(summary.last_governed_action.outcome_state, "ACCEPTED_RUNNING");
  assert.equal(summary.effective_governed_action.outcome_state, "ACCEPTED_RUNNING");
});
