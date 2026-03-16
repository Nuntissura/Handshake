#!/usr/bin/env node

import crypto from "node:crypto";
import fs from "node:fs";
import path from "node:path";
import { execFileSync } from "node:child_process";
import {
  assertOrchestratorLaunchAuthority,
  ensureSessionStateFiles,
  getOrCreateSessionRecord,
  loadSessionControlResults,
  loadSessionRegistry,
  saveSessionRegistry,
} from "../../../roles_shared/scripts/session/session-registry-lib.mjs";
import {
  buildSessionControlRequest,
  buildStartupPrompt,
  defaultSessionOutputFile,
  resolveRoleConfig,
  selectModel,
} from "../../../roles_shared/scripts/session/session-control-lib.mjs";
import { callHandshakeAcpMethod } from "../../../roles_shared/scripts/session/handshake-acp-client.mjs";
import {
  SESSION_CONTROL_RUN_STALE_GRACE_SECONDS,
  SESSION_CONTROL_RUN_TIMEOUT_SECONDS,
} from "../../../roles_shared/scripts/session/session-policy.mjs";

const commandKind = String(process.argv[2] || "").trim().toUpperCase();
const role = String(process.argv[3] || "").trim().toUpperCase();
const wpId = String(process.argv[4] || "").trim();
const promptArg = String(process.argv[5] || "").trim();
const requestedModel = String(process.argv[6] || "").trim().toUpperCase() || "PRIMARY";

function sleep(ms) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

async function waitForSettledResult(repoRoot, commandId, timeoutMs = 30000) {
  const startedAt = Date.now();
  while ((Date.now() - startedAt) < timeoutMs) {
    const settled = loadSessionControlResults(repoRoot).results.find((entry) => entry.command_id === commandId);
    if (settled) return settled;
    await sleep(250);
  }
  return null;
}

function fail(message) {
  console.error(`[SESSION_CONTROL] ${message}`);
  process.exit(1);
}

function runGit(args) {
  return execFileSync("git", args, { encoding: "utf8", stdio: ["ignore", "pipe", "pipe"] }).trim();
}

if (!["START_SESSION", "SEND_PROMPT", "CANCEL_SESSION", "CLOSE_SESSION"].includes(commandKind)) {
  fail("Usage: node .GOV/roles/orchestrator/scripts/session-control-command.mjs <START_SESSION|SEND_PROMPT|CANCEL_SESSION|CLOSE_SESSION> <CODER|WP_VALIDATOR|INTEGRATION_VALIDATOR> <WP_ID> [PROMPT] [PRIMARY|FALLBACK]");
}
if (!wpId || !wpId.startsWith("WP-")) {
  fail("WP_ID must start with WP-");
}

const roleConfig = resolveRoleConfig(role, wpId);
if (!roleConfig) fail(`Unknown role: ${role}`);

const repoRoot = runGit(["rev-parse", "--show-toplevel"]);
const currentBranch = runGit(["branch", "--show-current"]);
assertOrchestratorLaunchAuthority(currentBranch);
const absWorktreeDir = path.resolve(repoRoot, roleConfig.worktreeDir);

const selectedModel = selectModel(requestedModel);
ensureSessionStateFiles(repoRoot);
const { registry } = loadSessionRegistry(repoRoot);
let session = getOrCreateSessionRecord(registry, {
  wp_id: wpId,
  role,
  local_branch: roleConfig.branch,
  local_worktree_dir: roleConfig.worktreeDir,
  terminal_title: roleConfig.title,
  requested_model: selectedModel,
});

if (commandKind === "CANCEL_SESSION") {
  session = registry.sessions.find((entry) => entry.session_key === session.session_key) || session;
  const targetCommandId = (session.last_command_status === "RUNNING" ? session.last_command_id : "") || session.last_command_id || "";
  if (!targetCommandId) {
    console.log(`[SESSION_CONTROL] session_key=${session.session_key}`);
    console.log("[SESSION_CONTROL] status=not_running");
    console.log("[SESSION_CONTROL] reason=no governed command history is registered");
    process.exit(0);
  }

  const commandId = crypto.randomUUID();
  const request = buildSessionControlRequest({
    commandId,
    commandKind,
    wpId,
    role,
    sessionKey: session.session_key,
    localBranch: roleConfig.branch,
    localWorktreeDir: roleConfig.worktreeDir,
    absWorktreeDir,
    selectedModel: session.requested_model || selectedModel,
    prompt: `Cancel governed command ${targetCommandId} for ${role} ${wpId}.`,
    threadId: session.session_thread_id || "",
    summary: `Cancel governed ${role} session command ${targetCommandId} for ${wpId}`,
    outputJsonlFile: defaultSessionOutputFile(repoRoot, session.session_key, commandId),
    targetCommandId,
  });

  saveSessionRegistry(repoRoot, registry);

  let acpResponse;
  try {
    acpResponse = await callHandshakeAcpMethod({
      repoRoot,
      method: "session/cancel",
      params: { request },
      timeoutMs: 30000,
    });
  } catch (error) {
    const existingResult = loadSessionControlResults(repoRoot).results.find((entry) => entry.command_id === request.command_id);
    if (existingResult) {
      acpResponse = {
        result: {
          command_id: existingResult.command_id,
          session_id: existingResult.session_key,
          status: String(existingResult.cancel_status || existingResult.status || "").toLowerCase(),
          output_jsonl_file: existingResult.output_jsonl_file || request.output_jsonl_file,
          error: existingResult.error || "",
          run_id: existingResult.target_command_id || targetCommandId,
        },
      };
    } else {
      fail(`Broker cancel dispatch failed for ${request.command_id}: ${error.message || "Handshake ACP call failed"}`);
    }
  }

  const response = acpResponse.result || {};
  const settledCancel = await waitForSettledResult(repoRoot, request.command_id);
  if (!settledCancel) {
    fail(`Cancel request ${request.command_id} did not settle within 30s`);
  }

  console.log(`[SESSION_CONTROL] command_id=${request.command_id}`);
  console.log(`[SESSION_CONTROL] session_key=${session.session_key}`);
  console.log(`[SESSION_CONTROL] command_kind=${request.command_kind}`);
  console.log(`[SESSION_CONTROL] target_command_id=${targetCommandId}`);
  console.log(`[SESSION_CONTROL] broker_status=${response.status || "unknown"}`);
  console.log(`[SESSION_CONTROL] settled_status=${settledCancel.status}`);
  console.log(`[SESSION_CONTROL] cancel_status=${settledCancel.cancel_status || response.status || "unknown"}`);
  console.log(`[SESSION_CONTROL] output_jsonl=${settledCancel.output_jsonl_file || request.output_jsonl_file}`);
  if (settledCancel.summary) console.log(`[SESSION_CONTROL] summary=${settledCancel.summary}`);
  if (settledCancel.error) console.log(`[SESSION_CONTROL] error=${settledCancel.error}`);

  if ((settledCancel.cancel_status || response.status) === "cancellation_requested") {
    const settledTarget = await waitForSettledResult(repoRoot, targetCommandId);
    if (!settledTarget) {
      fail(`Cancellation requested for ${targetCommandId}, but the target run did not settle within 30s`);
    }
    console.log(`[SESSION_CONTROL] target_settled_status=${settledTarget.status}`);
    console.log(`[SESSION_CONTROL] target_output_jsonl=${settledTarget.output_jsonl_file || "<none>"}`);
    if (settledTarget.summary) console.log(`[SESSION_CONTROL] target_summary=${settledTarget.summary}`);
    if (settledTarget.error) console.log(`[SESSION_CONTROL] target_error=${settledTarget.error}`);
  }

  process.exit(0);
}

if (commandKind === "CLOSE_SESSION") {
  session = registry.sessions.find((entry) => entry.session_key === session.session_key) || session;
  const commandId = crypto.randomUUID();
  const request = buildSessionControlRequest({
    commandId,
    commandKind,
    wpId,
    role,
    sessionKey: session.session_key,
    localBranch: roleConfig.branch,
    localWorktreeDir: roleConfig.worktreeDir,
    absWorktreeDir,
    selectedModel: session.requested_model || selectedModel,
    prompt: `Close governed session ${role} ${wpId}. This must clear the steerable thread registration and leave no active run.`,
    threadId: session.session_thread_id || "",
    summary: `Close governed ${role} session for ${wpId}`,
    outputJsonlFile: defaultSessionOutputFile(repoRoot, session.session_key, commandId),
  });

  saveSessionRegistry(repoRoot, registry);

  let acpResponse;
  try {
    acpResponse = await callHandshakeAcpMethod({
      repoRoot,
      method: "session/close",
      params: { request },
      timeoutMs: 30000,
    });
  } catch (error) {
    const existingResult = loadSessionControlResults(repoRoot).results.find((entry) => entry.command_id === request.command_id);
    if (existingResult) {
      acpResponse = {
        result: {
          command_id: existingResult.command_id,
          session_id: existingResult.session_key,
          status: String(existingResult.status || "").toLowerCase(),
          output_jsonl_file: existingResult.output_jsonl_file || request.output_jsonl_file,
          error: existingResult.error || "",
          thread_id: existingResult.thread_id || "",
        },
      };
    } else {
      fail(`Broker close dispatch failed for ${request.command_id}: ${error.message || "Handshake ACP call failed"}`);
    }
  }

  const response = acpResponse.result || {};
  const settledClose = await waitForSettledResult(repoRoot, request.command_id);
  if (!settledClose) {
    fail(`Close request ${request.command_id} did not settle within 30s`);
  }

  console.log(`[SESSION_CONTROL] command_id=${request.command_id}`);
  console.log(`[SESSION_CONTROL] session_key=${session.session_key}`);
  console.log(`[SESSION_CONTROL] command_kind=${request.command_kind}`);
  console.log(`[SESSION_CONTROL] broker_status=${response.status || "unknown"}`);
  console.log(`[SESSION_CONTROL] settled_status=${settledClose.status}`);
  console.log(`[SESSION_CONTROL] output_jsonl=${settledClose.output_jsonl_file || request.output_jsonl_file}`);
  if (settledClose.summary) console.log(`[SESSION_CONTROL] summary=${settledClose.summary}`);
  if (settledClose.error) console.log(`[SESSION_CONTROL] error=${settledClose.error}`);
  process.exit(0);
}

if (!fs.existsSync(absWorktreeDir)) {
  execFileSync(
    process.execPath,
    [path.join(".GOV", "roles", "orchestrator", "scripts", "role-session-worktree-add.mjs"), role, wpId, roleConfig.branch, roleConfig.worktreeDir],
    { stdio: "inherit" },
  );
}

const prompt = commandKind === "START_SESSION"
  ? buildStartupPrompt({ role, wpId, roleConfig, selectedModel })
  : promptArg;

if (!prompt) {
  fail("SEND_PROMPT requires a non-empty prompt");
}
if (commandKind === "START_SESSION" && session.session_thread_id) {
  fail(`Session ${session.session_key} already has thread ${session.session_thread_id}. Use SEND_PROMPT to steer it.`);
}
if (commandKind === "SEND_PROMPT" && !session.session_thread_id) {
  fail(`No steerable thread id is registered yet for ${session.session_key}. Start the session first.`);
}

const summary = commandKind === "START_SESSION"
  ? `Start governed ${role} session for ${wpId}`
  : `Steer ${role} session for ${wpId}: ${prompt.split(/\r?\n/, 1)[0]}`;
const commandId = crypto.randomUUID();

const request = buildSessionControlRequest({
  commandId,
  commandKind,
  wpId,
  role,
  sessionKey: session.session_key,
  localBranch: roleConfig.branch,
  localWorktreeDir: roleConfig.worktreeDir,
  absWorktreeDir,
  selectedModel,
  prompt,
  threadId: session.session_thread_id || "",
  summary,
  outputJsonlFile: defaultSessionOutputFile(repoRoot, session.session_key, commandId),
});

saveSessionRegistry(repoRoot, registry);

let acpResponse;
try {
  acpResponse = await callHandshakeAcpMethod({
    repoRoot,
    method: commandKind === "START_SESSION" ? "session/new" : "session/prompt",
    params: { request },
    timeoutMs: (SESSION_CONTROL_RUN_TIMEOUT_SECONDS + SESSION_CONTROL_RUN_STALE_GRACE_SECONDS + 30) * 1000,
  });
} catch (error) {
  const existingResult = loadSessionControlResults(repoRoot).results.find((entry) => entry.command_id === request.command_id);
  if (existingResult) {
    acpResponse = {
      result: {
        command_id: existingResult.command_id,
        session_id: existingResult.session_key,
        thread_id: existingResult.thread_id || "",
        status: String(existingResult.status || "").toLowerCase(),
        output_jsonl_file: existingResult.output_jsonl_file || request.output_jsonl_file,
        last_agent_message: existingResult.last_agent_message || "",
        error: existingResult.error || "",
        duration_ms: existingResult.duration_ms || 0,
      },
    };
  } else {
    fail(`Broker dispatch failed for ${request.command_id}: ${error.message || "Handshake ACP call failed"}`);
  }
}

const response = acpResponse.result || {};
const refreshedRegistry = loadSessionRegistry(repoRoot).registry;
const refreshedSession = refreshedRegistry.sessions.find((entry) => entry.session_key === session.session_key) || session;

if (String(response.status || "").toLowerCase() !== "completed") {
  fail(`Command ${request.command_id} failed (${response.error || "no broker error reported"})`);
}

console.log(`[SESSION_CONTROL] command_id=${request.command_id}`);
console.log(`[SESSION_CONTROL] session_key=${session.session_key}`);
console.log(`[SESSION_CONTROL] thread_id=${refreshedSession.session_thread_id || response.thread_id || "<missing>"}`);
console.log(`[SESSION_CONTROL] runtime_state=${refreshedSession.runtime_state}`);
console.log(`[SESSION_CONTROL] command_kind=${request.command_kind}`);
console.log(`[SESSION_CONTROL] output_jsonl=${response.output_jsonl_file || request.output_jsonl_file}`);
if (response.last_agent_message) {
  console.log(`[SESSION_CONTROL] last_agent_message=${response.last_agent_message}`);
}

