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
  mutateSessionRegistrySync,
} from "../../../roles_shared/scripts/session/session-registry-lib.mjs";
import {
  buildRoleEnvironmentOverrides,
  buildSessionControlRequest,
  buildStartupPrompt,
  defaultSessionOutputFile,
  resolveRoleConfig,
  resolveRoleLaunchSelection,
  assertRoleLaunchProfileSupported,
} from "../../../roles_shared/scripts/session/session-control-lib.mjs";
import { settleRecoverableSessionControlResults } from "../../../roles_shared/scripts/session/session-control-self-settle-lib.mjs";
import { syncWpTokenUsageLedger } from "../../../roles_shared/scripts/session/wp-token-usage-lib.mjs";
import { callHandshakeAcpMethod } from "../../../roles_shared/scripts/session/handshake-acp-client.mjs";
import { reclaimOwnedSessionTerminals } from "../../../roles_shared/scripts/session/terminal-ownership-lib.mjs";
import {
  SESSION_CONTROL_RUN_STALE_GRACE_SECONDS,
  SESSION_CONTROL_RUN_TIMEOUT_SECONDS,
} from "../../../roles_shared/scripts/session/session-policy.mjs";
import { evaluateSessionGovernanceState } from "../../../roles_shared/scripts/session/session-governance-state-lib.mjs";
import { GOV_ROOT_REPO_REL } from "../../../roles_shared/scripts/lib/runtime-paths.mjs";
import { registerFailCaptureHook, failWithMemory } from "../../../roles_shared/scripts/lib/fail-capture-lib.mjs";
registerFailCaptureHook("session-control-command.mjs", { role: "ORCHESTRATOR" });

const commandKind = String(process.argv[2] || "").trim().toUpperCase();
const role = String(process.argv[3] || "").trim().toUpperCase();
const wpId = String(process.argv[4] || "").trim();
const commandContextArgs = process.argv.slice(5);
const promptArg = String(commandContextArgs[0] || "").trim();
const debugMode = String(process.env.HANDSHAKE_SESSION_CONTROL_DEBUG || "").trim() === "1"
  || commandContextArgs.some((arg) => String(arg || "").trim() === "--debug");
const requestedModel = (() => {
  const args = commandContextArgs;
  for (const candidate of args) {
    const value = String(candidate || "").trim().toUpperCase();
    if (!value || value.startsWith("--")) continue;
    return value;
  }
  return "PRIMARY";
})();
if (debugMode) {
  console.log("[SESSION_CONTROL] debug_mode=enabled");
}

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
  failWithMemory("session-control-command.mjs", message, { role: "ORCHESTRATOR" });
}

function reclaimOwnedTerminalsForSession(context = "") {
  if (debugMode) {
    const tag = context ? ` [${context}]` : "";
    console.log(`[SESSION_CONTROL] debug_mode=enabled${tag} skip_terminal_reclaim`);
    return [];
  }
  const sessionKey = String(session?.session_key || "").trim();
  if (!sessionKey) return [];
  try {
    const results = reclaimOwnedSessionTerminals(repoRoot, { sessionKey });
    const prefix = context ? `${context} ` : "";
    for (const reclaim of results) {
      console.log(`[SESSION_CONTROL] ${prefix}terminal_reclaim_session=${reclaim.session_key}`);
      console.log(`[SESSION_CONTROL] ${prefix}terminal_reclaim_process_id=${reclaim.process_id}`);
      console.log(`[SESSION_CONTROL] ${prefix}terminal_reclaim_status=${reclaim.reclaim_status}`);
      if (reclaim.error) console.log(`[SESSION_CONTROL] ${prefix}terminal_reclaim_error=${reclaim.error}`);
    }
    return results;
  } catch (error) {
    console.log(`[SESSION_CONTROL] terminal_reclaim_error=${String(error?.message || error || "").slice(0, 200)}`);
    return [];
  }
}

function runGit(args) {
  return execFileSync("git", args, { encoding: "utf8", stdio: ["ignore", "pipe", "pipe"] }).trim();
}

if (!["START_SESSION", "SEND_PROMPT", "CANCEL_SESSION", "CLOSE_SESSION"].includes(commandKind)) {
  fail(`Usage: node ${GOV_ROOT_REPO_REL}/roles/orchestrator/scripts/session-control-command.mjs <START_SESSION|SEND_PROMPT|CANCEL_SESSION|CLOSE_SESSION> <ACTIVATION_MANAGER|CODER|WP_VALIDATOR|INTEGRATION_VALIDATOR> <WP_ID> [PROMPT] [PRIMARY|FALLBACK] [--debug]`);
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
const environmentOverrides = buildRoleEnvironmentOverrides({ role });

const sessionDescriptor = {
  wp_id: wpId,
  role,
  local_branch: roleConfig.branch,
  local_worktree_dir: roleConfig.worktreeDir,
  terminal_title: roleConfig.title,
};
let selectedModel = "";
let selectedProfileId = "";
let selectedProfile = null;

if (commandKind === "START_SESSION") {
  const selection = resolveRoleLaunchSelection({
    role,
    wpId,
    modelSelector: requestedModel,
  });
  selectedProfileId = selection.selectedProfileId;
  selectedProfile = assertRoleLaunchProfileSupported({
    role,
    wpId,
    selectedProfileId,
    selectedProfile: selection.selectedProfile,
  });
  selectedModel = selectedProfile.launch_model;
  sessionDescriptor.requested_model = selectedModel;
  sessionDescriptor.requested_profile_id = selectedProfileId;
  sessionDescriptor.reasoning_config_key = selectedProfile.launch_reasoning_config_key;
  sessionDescriptor.reasoning_config_value = selectedProfile.launch_reasoning_config_value;
}
function ensureSessionRecord() {
  return mutateSessionRegistrySync(repoRoot, (registry) => {
    const session = getOrCreateSessionRecord(registry, sessionDescriptor);
    return {
      session_key: session.session_key,
      session_thread_id: session.session_thread_id || "",
      last_command_status: session.last_command_status || "NONE",
      last_command_id: session.last_command_id || "",
      requested_model: session.requested_model || selectedModel,
      requested_profile_id: session.requested_profile_id || selectedProfileId,
      reasoning_config_key: session.reasoning_config_key || selectedProfile?.launch_reasoning_config_key || "",
      reasoning_config_value: session.reasoning_config_value || selectedProfile?.launch_reasoning_config_value || "",
      runtime_state: session.runtime_state || "UNSTARTED",
    };
  });
}

const startGovernance = commandKind === "START_SESSION"
  ? evaluateSessionGovernanceState(repoRoot, sessionDescriptor)
  : null;

if (commandKind === "START_SESSION" && startGovernance && !startGovernance.launchAllowed) {
  fail(
    `Governed session ${role}:${wpId} cannot be started: ${startGovernance.launchBlockers.join("; ")}`,
  );
}

ensureSessionStateFiles(repoRoot);
let session = ensureSessionRecord();
selectedModel = commandKind === "START_SESSION" ? selectedModel : (session.requested_model || "");
selectedProfileId = commandKind === "START_SESSION" ? selectedProfileId : (session.requested_profile_id || "");
const governance = commandKind === "START_SESSION"
  ? startGovernance
  : evaluateSessionGovernanceState(repoRoot, {
    ...sessionDescriptor,
    ...session,
  });

if (commandKind === "SEND_PROMPT" && !governance.steeringAllowed) {
  fail(
    `Governed session ${session.session_key} cannot be steered: ${governance.steeringBlockers.join("; ")}`,
  );
}

if (commandKind === "CANCEL_SESSION") {
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
    environmentOverrides,
    targetCommandId,
  });

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
      settleRecoverableSessionControlResults(repoRoot, { commandIds: [request.command_id, targetCommandId].filter(Boolean) });
      const recoveredResult = loadSessionControlResults(repoRoot).results.find((entry) => entry.command_id === request.command_id);
      if (recoveredResult) {
        acpResponse = {
          result: {
            command_id: recoveredResult.command_id,
            session_id: recoveredResult.session_key,
            status: String(recoveredResult.cancel_status || recoveredResult.status || "").toLowerCase(),
            output_jsonl_file: recoveredResult.output_jsonl_file || request.output_jsonl_file,
            error: recoveredResult.error || "",
            run_id: recoveredResult.target_command_id || targetCommandId,
          },
        };
      } else {
        fail(`Broker cancel dispatch failed for ${request.command_id}: ${error.message || "Handshake ACP call failed"}`);
      }
    }
  }

  const response = acpResponse.result || {};
  let settledCancel = await waitForSettledResult(repoRoot, request.command_id);
  if (!settledCancel) {
    settleRecoverableSessionControlResults(repoRoot, { commandIds: [request.command_id, targetCommandId].filter(Boolean) });
    settledCancel = loadSessionControlResults(repoRoot).results.find((entry) => entry.command_id === request.command_id) || null;
  }
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
  syncWpTokenUsageLedger(repoRoot, settledCancel, { session });

  if ((settledCancel.cancel_status || response.status) === "cancellation_requested") {
    let settledTarget = await waitForSettledResult(repoRoot, targetCommandId);
    if (!settledTarget) {
      settleRecoverableSessionControlResults(repoRoot, { commandIds: [targetCommandId].filter(Boolean) });
      settledTarget = loadSessionControlResults(repoRoot).results.find((entry) => entry.command_id === targetCommandId) || null;
    }
    if (!settledTarget) {
      fail(`Cancellation requested for ${targetCommandId}, but the target run did not settle within 30s`);
    }
    console.log(`[SESSION_CONTROL] target_settled_status=${settledTarget.status}`);
    console.log(`[SESSION_CONTROL] target_output_jsonl=${settledTarget.output_jsonl_file || "<none>"}`);
    if (settledTarget.summary) console.log(`[SESSION_CONTROL] target_summary=${settledTarget.summary}`);
    if (settledTarget.error) console.log(`[SESSION_CONTROL] target_error=${settledTarget.error}`);
    syncWpTokenUsageLedger(repoRoot, settledTarget, { session });
  }

  process.exit(0);
}

if (commandKind === "CLOSE_SESSION") {
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
    environmentOverrides,
  });

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
      settleRecoverableSessionControlResults(repoRoot, { commandIds: [request.command_id] });
      const recoveredResult = loadSessionControlResults(repoRoot).results.find((entry) => entry.command_id === request.command_id);
      if (recoveredResult) {
        acpResponse = {
          result: {
            command_id: recoveredResult.command_id,
            session_id: recoveredResult.session_key,
            status: String(recoveredResult.status || "").toLowerCase(),
            output_jsonl_file: recoveredResult.output_jsonl_file || request.output_jsonl_file,
            error: recoveredResult.error || "",
            thread_id: recoveredResult.thread_id || "",
          },
        };
      } else {
      reclaimOwnedTerminalsForSession("CLOSE_SESSION_DISPATCH_FAILURE");
      fail(`Broker close dispatch failed for ${request.command_id}: ${error.message || "Handshake ACP call failed"}`);
      }
    }
  }

  const response = acpResponse.result || {};
  let settledClose = await waitForSettledResult(repoRoot, request.command_id);
  if (!settledClose) {
    settleRecoverableSessionControlResults(repoRoot, { commandIds: [request.command_id] });
    settledClose = loadSessionControlResults(repoRoot).results.find((entry) => entry.command_id === request.command_id) || null;
  }
  if (!settledClose) {
    reclaimOwnedTerminalsForSession("CLOSE_SESSION_SETTLE_TIMEOUT");
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
  syncWpTokenUsageLedger(repoRoot, settledClose, { session });

  // RGF-136: session-end memory flush — capture a semantic summary of the closed session
  try {
    const { openGovernanceMemoryDb, closeDb: closeMemDb, addMemory } = await import("../../../roles_shared/scripts/memory/governance-memory-lib.mjs");
    const { parseJsonlFile } = await import("../../../roles_shared/scripts/lib/wp-communications-lib.mjs");
    const { communicationPathsForWp } = await import("../../../roles_shared/scripts/lib/wp-communications-lib.mjs");
    const commPaths = communicationPathsForWp(wpId);
    const receiptsAbs = path.resolve(repoRoot, commPaths.receiptsFile);
    if (fs.existsSync(receiptsAbs)) {
      const receipts = parseJsonlFile(commPaths.receiptsFile);
      const sessionReceipts = receipts.filter(r => r.actor_session === session.session_key || r.target_session === session.session_key);
      if (sessionReceipts.length > 0) {
        const kinds = {};
        for (const r of sessionReceipts) { kinds[r.receipt_kind] = (kinds[r.receipt_kind] || 0) + 1; }
        const kindSummary = Object.entries(kinds).map(([k, v]) => `${v}x ${k}`).join(", ");
        const { db: memDb } = openGovernanceMemoryDb();
        try {
          addMemory(memDb, {
            memoryType: "semantic",
            topic: `Session closed: ${role} on ${wpId}`,
            summary: `${role} session produced ${sessionReceipts.length} receipts (${kindSummary})`,
            wpId,
            importance: 0.4,
            content: `Role: ${role}\nSession: ${session.session_key}\nReceipts: ${sessionReceipts.length}\nBreakdown: ${kindSummary}\nOutcome: ${settledClose.status}`,
            sourceArtifact: "session-control-command",
            sourceRole: "ORCHESTRATOR",
            sourceSession: session.session_key,
            metadata: { session_flush: true, receipt_kinds: kinds },
          });
        } finally { closeMemDb(memDb); }
      }
    }
  } catch { /* best-effort: session flush failure must not block close */ }

  reclaimOwnedTerminalsForSession("CLOSE_SESSION_SUCCESS");
  process.exit(0);
}

if (!fs.existsSync(absWorktreeDir)) {
  execFileSync(
    process.execPath,
    [path.join(GOV_ROOT_REPO_REL, "roles", "orchestrator", "scripts", "role-session-worktree-add.mjs"), role, wpId, roleConfig.branch, roleConfig.worktreeDir],
    { stdio: "inherit" },
  );
}

const prompt = commandKind === "START_SESSION"
  ? buildStartupPrompt({ role, wpId, roleConfig, selectedModel, selectedProfileId, selectedProfile })
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
  selectedProfileId: commandKind === "START_SESSION" ? selectedProfileId : (session.requested_profile_id || ""),
  prompt,
  threadId: session.session_thread_id || "",
  summary,
  outputJsonlFile: defaultSessionOutputFile(repoRoot, session.session_key, commandId),
  environmentOverrides,
  reasoningConfigKey: commandKind === "START_SESSION"
    ? (selectedProfile?.launch_reasoning_config_key || "")
    : (session.reasoning_config_key || ""),
  reasoningConfigValue: commandKind === "START_SESSION"
    ? (selectedProfile?.launch_reasoning_config_value || "")
    : (session.reasoning_config_value || ""),
});

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
    settleRecoverableSessionControlResults(repoRoot, { commandIds: [request.command_id] });
    const recoveredResult = loadSessionControlResults(repoRoot).results.find((entry) => entry.command_id === request.command_id);
    if (recoveredResult) {
      acpResponse = {
        result: {
          command_id: recoveredResult.command_id,
          session_id: recoveredResult.session_key,
          thread_id: recoveredResult.thread_id || "",
          status: String(recoveredResult.status || "").toLowerCase(),
          output_jsonl_file: recoveredResult.output_jsonl_file || request.output_jsonl_file,
          last_agent_message: recoveredResult.last_agent_message || "",
          error: recoveredResult.error || "",
          duration_ms: recoveredResult.duration_ms || 0,
        },
      };
    } else {
      reclaimOwnedTerminalsForSession("START_OR_SEND_DISPATCH_FAILURE");
      fail(`Broker dispatch failed for ${request.command_id}: ${error.message || "Handshake ACP call failed"}`);
    }
  }
}

const response = acpResponse.result || {};
const refreshedRegistry = loadSessionRegistry(repoRoot).registry;
const refreshedSession = refreshedRegistry.sessions.find((entry) => entry.session_key === session.session_key) || session;

if (String(response.status || "").toLowerCase() !== "completed") {
  reclaimOwnedTerminalsForSession("START_OR_SEND_COMPLETION_FAILURE");
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
syncWpTokenUsageLedger(repoRoot, {
  command_id: request.command_id,
  command_kind: request.command_kind,
  session_key: session.session_key,
  wp_id: wpId,
  role,
  status: "COMPLETED",
  thread_id: refreshedSession.session_thread_id || response.thread_id || "",
  processed_at: new Date().toISOString(),
  output_jsonl_file: response.output_jsonl_file || request.output_jsonl_file,
}, {
  session: refreshedSession,
});
