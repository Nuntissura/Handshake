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
  classifySessionControlOutcomeState,
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
import {
  appendWorkflowDossierEntry,
  formatWorkflowDossierTimestamp,
  normalizePath,
} from "../../../roles_shared/scripts/audit/workflow-dossier-lib.mjs";
import { emitOperatorGateNotificationIfNeeded } from "./lib/operator-gate-notification-lib.mjs";
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

function shortenDossierToken(value = "", prefix = 8, suffix = 6) {
  const text = String(value || "").trim();
  if (!text) return "";
  if (text.length <= (prefix + suffix + 2)) return text;
  return `${text.slice(0, prefix)}..${text.slice(-suffix)}`;
}

function acpFlowRoute(stage = "", settledRole = role) {
  const normalizedStage = String(stage || "").trim().toLowerCase();
  if (normalizedStage === "terminal.reclaimed") {
    return "ACP -> ORCHESTRATOR";
  }
  if (normalizedStage === "run.started" || normalizedStage === "process.spawned") {
    return `ORCHESTRATOR -> ACP -> ${settledRole}`;
  }
  return `${settledRole} -> ACP -> ORCHESTRATOR`;
}

function buildAcpExecutionLogLine({
  when = new Date(),
  commandKind: settledCommandKind = commandKind,
  settledRole = role,
  targetWpId = wpId,
  status = "UNKNOWN",
  outcomeState = "",
  threadId = "",
  outputJsonlFile = "",
  detail = "",
}) {
  const whenIso = formatWorkflowDossierTimestamp(when);
  const route = acpFlowRoute("completed", settledRole);
  const parts = [
    `status=${status}`,
    outcomeState ? `outcome=${outcomeState}` : "",
    threadId ? `thread=${threadId}` : "",
    outputJsonlFile ? `output=${normalizePath(outputJsonlFile)}` : "",
    targetWpId ? `wp=${targetWpId}` : "",
    detail ? `detail=${detail}` : "",
  ].filter(Boolean);
  const suffix = parts.length > 0 ? ` | ${parts.join(" | ")}` : "";
  return `- [${whenIso}] [ORCHESTRATOR] [ACP_SESSION_CONTROL] \`${route}\` ${settledCommandKind}/${status}${suffix}`;
}

function buildAcpUpdateLogLine({
  notification,
  commandKind: settledCommandKind = commandKind,
  settledRole = role,
  targetWpId = wpId,
}) {
  if (!notification || notification.method !== "session/update") return "";
  const params = notification.params || {};
  const stage = String(params.stage || "event").trim();
  const whenIso = formatWorkflowDossierTimestamp(params.timestamp || new Date());
  const route = acpFlowRoute(stage, settledRole);
  const parts = [
    params.command_id ? `cmd=${shortenDossierToken(params.command_id)}` : "",
    targetWpId ? `wp=${targetWpId}` : "",
    params.thread_id ? `thread=${shortenDossierToken(params.thread_id)}` : "",
    params.pid ? `pid=${params.pid}` : "",
    params.reclaim_status ? `reclaim=${params.reclaim_status}` : "",
    params.process_id ? `process=${params.process_id}` : "",
    params.terminal_batch_id ? `batch=${shortenDossierToken(params.terminal_batch_id)}` : "",
  ].filter(Boolean);
  const suffix = parts.length > 0 ? ` | ${parts.join(" | ")}` : "";
  return `- [${whenIso}] [ORCHESTRATOR] [ACP_UPDATE] \`${route}\` ${settledCommandKind}/${stage}${suffix}`;
}

function appendWorkflowDossierExecutionLog(targetWpId, summaryLine) {
  if (!summaryLine) return;
  try {
    appendWorkflowDossierEntry({
      repoRoot,
      wpId: targetWpId,
      section: "EXECUTION",
      line: summaryLine,
    });
  } catch {
    // Non-fatal: dossier append is observability only.
  }
}

function emitSessionOutcomeLines({
  sessionKey = session?.session_key || "",
  threadId = "",
  runtimeState = session?.runtime_state || "",
  settledCommandKind = commandKind,
  outcomeState = "FAILED",
  outputJsonlFile = "",
  detail = "",
  requestCommandId = "",
  lastAgentMessage = "",
} = {}) {
  if (requestCommandId) {
    console.log(`[SESSION_CONTROL] command_id=${requestCommandId}`);
  }
  if (sessionKey) console.log(`[SESSION_CONTROL] session_key=${sessionKey}`);
  console.log(`[SESSION_CONTROL] thread_id=${threadId || "<missing>"}`);
  if (runtimeState) console.log(`[SESSION_CONTROL] runtime_state=${runtimeState}`);
  console.log(`[SESSION_CONTROL] command_kind=${settledCommandKind}`);
  console.log(`[SESSION_CONTROL] outcome_state=${outcomeState}`);
  if (outputJsonlFile) console.log(`[SESSION_CONTROL] output_jsonl=${outputJsonlFile}`);
  if (lastAgentMessage) {
    console.log(`[SESSION_CONTROL] last_agent_message=${lastAgentMessage}`);
  }
  if (detail) {
    console.log(`[SESSION_CONTROL] detail=${detail}`);
  }
}

function sessionAlreadyReady(currentSession) {
  const threadId = String(currentSession?.session_thread_id || "").trim();
  const runtimeState = String(currentSession?.runtime_state || "").trim().toUpperCase();
  const startupProofState = String(currentSession?.startup_proof_state || "").trim().toUpperCase();
  return Boolean(threadId) && (runtimeState === "READY" || startupProofState === "READY");
}

function loadSessionRecord(sessionKey) {
  const registry = loadSessionRegistry(repoRoot).registry;
  return (registry.sessions || []).find((entry) => entry.session_key === sessionKey) || null;
}

async function waitForSessionReady(sessionKey, timeoutMs = 2500) {
  const startedAt = Date.now();
  while ((Date.now() - startedAt) < timeoutMs) {
    const currentSession = loadSessionRecord(sessionKey);
    if (currentSession && sessionAlreadyReady(currentSession)) return currentSession;
    await sleep(250);
  }
  const finalSession = loadSessionRecord(sessionKey);
  return finalSession && sessionAlreadyReady(finalSession) ? finalSession : null;
}

function classifyResponseOutcome({ settledCommandKind = commandKind, response = {}, refreshedSession = session } = {}) {
  return classifySessionControlOutcomeState({
    status: String(response.status || "").trim().toUpperCase(),
    commandKind: settledCommandKind,
    error: response.error || "",
    summary: response.last_agent_message || "",
    cancelStatus: response.cancel_status || "",
  });
}

if (!["START_SESSION", "SEND_PROMPT", "CANCEL_SESSION", "CLOSE_SESSION"].includes(commandKind)) {
  fail(`Usage: node ${GOV_ROOT_REPO_REL}/roles/orchestrator/scripts/session-control-command.mjs <START_SESSION|SEND_PROMPT|CANCEL_SESSION|CLOSE_SESSION> <ACTIVATION_MANAGER|CODER|WP_VALIDATOR|INTEGRATION_VALIDATOR|MEMORY_MANAGER> <WP_ID> [PROMPT] [PRIMARY|FALLBACK] [--debug]`);
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
      notificationDrainMs: 250,
      onNotification: (notification) => {
        appendWorkflowDossierExecutionLog(wpId, buildAcpUpdateLogLine({
          notification,
          commandKind,
          settledRole: role,
          targetWpId: wpId,
        }));
      },
    });
  } catch (error) {
    const existingResult = loadSessionControlResults(repoRoot).results.find((entry) => entry.command_id === request.command_id);
    if (existingResult) {
      acpResponse = {
        result: {
          command_id: existingResult.command_id,
          session_id: existingResult.session_key,
          status: String(existingResult.cancel_status || existingResult.status || "").toLowerCase(),
          outcome_state: existingResult.outcome_state || "",
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
            outcome_state: recoveredResult.outcome_state || "",
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

  appendWorkflowDossierExecutionLog(wpId, buildAcpExecutionLogLine({
    when: settledCancel.processed_at || new Date(),
    commandKind,
    settledRole: role,
    targetWpId: wpId,
    status: settledCancel.cancel_status || settledCancel.status || response.status || "UNKNOWN",
    threadId: session.session_thread_id || "",
    outputJsonlFile: settledCancel.output_jsonl_file || request.output_jsonl_file || "",
    detail: settledCancel.summary || "",
  }));

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
      notificationDrainMs: 250,
      onNotification: (notification) => {
        appendWorkflowDossierExecutionLog(wpId, buildAcpUpdateLogLine({
          notification,
          commandKind,
          settledRole: role,
          targetWpId: wpId,
        }));
      },
    });
  } catch (error) {
    const existingResult = loadSessionControlResults(repoRoot).results.find((entry) => entry.command_id === request.command_id);
    if (existingResult) {
      acpResponse = {
        result: {
          command_id: existingResult.command_id,
          session_id: existingResult.session_key,
          status: String(existingResult.status || "").toLowerCase(),
          outcome_state: existingResult.outcome_state || "",
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
            outcome_state: recoveredResult.outcome_state || "",
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
  appendWorkflowDossierExecutionLog(wpId, buildAcpExecutionLogLine({
    when: settledClose.processed_at || new Date(),
    commandKind,
    settledRole: role,
    targetWpId: wpId,
    status: settledClose.status || response.status || "UNKNOWN",
    threadId: settledClose.thread_id || session.session_thread_id || "",
    outputJsonlFile: settledClose.output_jsonl_file || request.output_jsonl_file || "",
    detail: settledClose.summary || "",
  }));

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
if (commandKind === "START_SESSION" && sessionAlreadyReady(session)) {
  const detail = `Session ${session.session_key} already has steerable thread ${session.session_thread_id}.`;
  emitSessionOutcomeLines({
    sessionKey: session.session_key,
    threadId: session.session_thread_id,
    runtimeState: session.runtime_state,
    settledCommandKind: commandKind,
    outcomeState: "ALREADY_READY",
    detail,
  });
  appendWorkflowDossierExecutionLog(wpId, buildAcpExecutionLogLine({
    when: new Date(),
    commandKind,
    settledRole: role,
    targetWpId: wpId,
    status: "COMPLETED",
    outcomeState: "ALREADY_READY",
    threadId: session.session_thread_id,
    detail,
  }));
  process.exit(0);
}
if (commandKind === "SEND_PROMPT" && !session.session_thread_id) {
  const detail = `No steerable thread id is registered yet for ${session.session_key}. Start the session first.`;
  emitSessionOutcomeLines({
    sessionKey: session.session_key,
    threadId: session.session_thread_id,
    runtimeState: session.runtime_state,
    settledCommandKind: commandKind,
    outcomeState: "REQUIRES_START",
    detail,
  });
  appendWorkflowDossierExecutionLog(wpId, buildAcpExecutionLogLine({
    when: new Date(),
    commandKind,
    settledRole: role,
    targetWpId: wpId,
    status: "FAILED",
    outcomeState: "REQUIRES_START",
    threadId: session.session_thread_id,
    detail,
  }));
  fail(detail);
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
  busyIngressMode: commandKind === "SEND_PROMPT" ? "ENQUEUE_ON_BUSY" : "REJECT",
});

let acpResponse;
try {
  acpResponse = await callHandshakeAcpMethod({
    repoRoot,
    method: commandKind === "START_SESSION" ? "session/new" : "session/prompt",
    params: { request },
    timeoutMs: (SESSION_CONTROL_RUN_TIMEOUT_SECONDS + SESSION_CONTROL_RUN_STALE_GRACE_SECONDS + 30) * 1000,
    notificationDrainMs: 250,
    onNotification: (notification) => {
      appendWorkflowDossierExecutionLog(wpId, buildAcpUpdateLogLine({
        notification,
        commandKind,
        settledRole: role,
        targetWpId: wpId,
      }));
    },
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
          outcome_state: existingResult.outcome_state || "",
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
          outcome_state: recoveredResult.outcome_state || "",
          output_jsonl_file: recoveredResult.output_jsonl_file || request.output_jsonl_file,
          last_agent_message: recoveredResult.last_agent_message || "",
          error: recoveredResult.error || "",
          duration_ms: recoveredResult.duration_ms || 0,
        },
      };
    } else {
      const recoveredReadySession = commandKind === "START_SESSION"
        ? await waitForSessionReady(session.session_key)
        : null;
      if (recoveredReadySession) {
        const detail = `Session ${recoveredReadySession.session_key} became ready while broker dispatch recovery was converging.`;
        emitSessionOutcomeLines({
          requestCommandId: request.command_id,
          sessionKey: recoveredReadySession.session_key,
          threadId: recoveredReadySession.session_thread_id || "",
          runtimeState: recoveredReadySession.runtime_state,
          settledCommandKind: request.command_kind,
          outcomeState: "ALREADY_READY",
          outputJsonlFile: request.output_jsonl_file || "",
          detail,
        });
        appendWorkflowDossierExecutionLog(wpId, buildAcpExecutionLogLine({
          when: new Date(),
          commandKind,
          settledRole: role,
          targetWpId: wpId,
          status: "COMPLETED",
          outcomeState: "ALREADY_READY",
          threadId: recoveredReadySession.session_thread_id || "",
          outputJsonlFile: request.output_jsonl_file || "",
          detail,
        }));
        process.exit(0);
      }
      const dispatchFailure = `Broker dispatch failed for ${request.command_id}: ${error.message || "Handshake ACP call failed"}`;
      appendWorkflowDossierExecutionLog(wpId, buildAcpExecutionLogLine({
        when: new Date(),
        commandKind,
        settledRole: role,
        targetWpId: wpId,
        status: "BROKER_DISPATCH_FAILED",
        outputJsonlFile: request.output_jsonl_file || "",
        detail: dispatchFailure,
      }));
      reclaimOwnedTerminalsForSession("START_OR_SEND_DISPATCH_FAILURE");
      fail(dispatchFailure);
    }
  }
}

const response = acpResponse.result || {};
const refreshedRegistry = loadSessionRegistry(repoRoot).registry;
const refreshedSession = refreshedRegistry.sessions.find((entry) => entry.session_key === session.session_key) || session;
const outcomeState = String(
  response.outcome_state
  || classifyResponseOutcome({ settledCommandKind: commandKind, response, refreshedSession }),
).trim().toUpperCase() || "FAILED";

if (String(response.status || "").toLowerCase() !== "completed") {
  if (
    commandKind === "SEND_PROMPT"
    && String(response.status || "").toLowerCase() === "queued"
    && outcomeState === "ACCEPTED_PENDING"
  ) {
    const detail = response.error
      || response.last_agent_message
      || `Queued ${session.session_key} behind active run ${response.blocking_run_id || "<unknown>"}.`;
    emitSessionOutcomeLines({
      requestCommandId: request.command_id,
      sessionKey: session.session_key,
      threadId: refreshedSession.session_thread_id || response.thread_id || "",
      runtimeState: refreshedSession.runtime_state,
      settledCommandKind: request.command_kind,
      outcomeState,
      outputJsonlFile: response.output_jsonl_file || request.output_jsonl_file || "",
      detail,
      lastAgentMessage: response.last_agent_message || "",
    });
    appendWorkflowDossierExecutionLog(wpId, buildAcpExecutionLogLine({
      when: new Date(),
      commandKind,
      settledRole: role,
      targetWpId: wpId,
      status: "QUEUED",
      outcomeState,
      threadId: refreshedSession.session_thread_id || response.thread_id || "",
      outputJsonlFile: response.output_jsonl_file || request.output_jsonl_file || "",
      detail,
    }));
    process.exit(0);
  }
  if (commandKind === "START_SESSION" && ["BUSY_ACTIVE_RUN", "REQUIRES_RECOVERY"].includes(outcomeState)) {
    const recoveredReadySession = await waitForSessionReady(session.session_key);
    if (recoveredReadySession) {
      const detail = response.error
        || response.last_agent_message
        || `Session ${recoveredReadySession.session_key} became ready while the broker reported ${outcomeState}.`;
      emitSessionOutcomeLines({
        requestCommandId: request.command_id,
        sessionKey: recoveredReadySession.session_key,
        threadId: recoveredReadySession.session_thread_id || "",
        runtimeState: recoveredReadySession.runtime_state,
        settledCommandKind: request.command_kind,
        outcomeState: "ALREADY_READY",
        outputJsonlFile: response.output_jsonl_file || request.output_jsonl_file || "",
        lastAgentMessage: response.last_agent_message || "",
        detail,
      });
      appendWorkflowDossierExecutionLog(wpId, buildAcpExecutionLogLine({
        when: new Date(),
        commandKind,
        settledRole: role,
        targetWpId: wpId,
        status: "COMPLETED",
        outcomeState: "ALREADY_READY",
        threadId: recoveredReadySession.session_thread_id || "",
        outputJsonlFile: response.output_jsonl_file || request.output_jsonl_file || "",
        detail,
      }));
      process.exit(0);
    }
  }
  // RGF-172/RGF-206: if the broker still rejects instead of queueing, surface the busy collision explicitly.
  if (commandKind === "SEND_PROMPT" && outcomeState === "BUSY_ACTIVE_RUN") {
    const detail = response.error
      || `Session ${session.session_key} has a concurrent active run. Wait for it to complete before sending another prompt.`;
    emitSessionOutcomeLines({
      requestCommandId: request.command_id,
      sessionKey: session.session_key,
      threadId: refreshedSession.session_thread_id || response.thread_id || "",
      runtimeState: refreshedSession.runtime_state,
      settledCommandKind: request.command_kind,
      outcomeState: "BUSY_ACTIVE_RUN",
      detail,
    });
    appendWorkflowDossierExecutionLog(wpId, buildAcpExecutionLogLine({
      when: new Date(),
      commandKind,
      settledRole: role,
      targetWpId: wpId,
      status: "BUSY",
      outcomeState: "BUSY_ACTIVE_RUN",
      threadId: refreshedSession.session_thread_id || response.thread_id || "",
      detail,
    }));
    fail(`SEND_PROMPT rejected [BUSY_ACTIVE_RUN]: ${detail}`);
  }
  appendWorkflowDossierExecutionLog(wpId, buildAcpExecutionLogLine({
    when: new Date(),
    commandKind,
    settledRole: role,
    targetWpId: wpId,
    status: String(response.status || "FAILED").trim().toUpperCase() || "FAILED",
    outcomeState,
    threadId: refreshedSession.session_thread_id || response.thread_id || "",
    outputJsonlFile: response.output_jsonl_file || request.output_jsonl_file || "",
    detail: response.error || response.last_agent_message || "no broker error reported",
  }));
  emitSessionOutcomeLines({
    requestCommandId: request.command_id,
    sessionKey: session.session_key,
    threadId: refreshedSession.session_thread_id || response.thread_id || "",
    runtimeState: refreshedSession.runtime_state,
    settledCommandKind: request.command_kind,
    outcomeState,
    outputJsonlFile: response.output_jsonl_file || request.output_jsonl_file || "",
    lastAgentMessage: response.last_agent_message || "",
    detail: response.error || response.last_agent_message || "no broker error reported",
  });
  reclaimOwnedTerminalsForSession("START_OR_SEND_COMPLETION_FAILURE");
  fail(`Command ${request.command_id} failed [${outcomeState}] (${response.error || "no broker error reported"})`);
}

emitSessionOutcomeLines({
  requestCommandId: request.command_id,
  sessionKey: session.session_key,
  threadId: refreshedSession.session_thread_id || response.thread_id || "",
  runtimeState: refreshedSession.runtime_state,
  settledCommandKind: request.command_kind,
  outcomeState,
  outputJsonlFile: response.output_jsonl_file || request.output_jsonl_file,
  lastAgentMessage: response.last_agent_message || "",
});
appendWorkflowDossierExecutionLog(wpId, buildAcpExecutionLogLine({
  when: new Date(),
  commandKind,
  settledRole: role,
  targetWpId: wpId,
  status: "COMPLETED",
  outcomeState,
  threadId: refreshedSession.session_thread_id || response.thread_id || "",
  outputJsonlFile: response.output_jsonl_file || request.output_jsonl_file || "",
  detail: response.last_agent_message || "",
}));
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

if (commandKind === "SEND_PROMPT") {
  const operatorGateNotification = emitOperatorGateNotificationIfNeeded({
    repoRoot,
    wpId,
    sourceSession: session.session_key,
  });
  if (operatorGateNotification.status === "EMITTED") {
    console.log(`[SESSION_CONTROL] operator_gate_status=${operatorGateNotification.status}`);
    console.log(`[SESSION_CONTROL] operator_gate_reason=${operatorGateNotification.reason}`);
    console.log(`[SESSION_CONTROL] operator_gate_correlation=${operatorGateNotification.candidate?.correlationId || ""}`);
    console.log(`[SESSION_CONTROL] operator_gate_summary=${operatorGateNotification.candidate?.summary || ""}`);
    appendWorkflowDossierExecutionLog(wpId, buildAcpExecutionLogLine({
      when: new Date(),
      commandKind: "OPERATOR_GATE",
      settledRole: "ORCHESTRATOR",
      targetWpId: wpId,
      status: "EMITTED",
      outcomeState: String(operatorGateNotification.lifecycle?.blockerClass || "").trim().toUpperCase() || "OPERATOR_GATE",
      detail: operatorGateNotification.candidate?.summary || "",
    }));
  } else if (operatorGateNotification.status === "FAILED") {
    console.log(`[SESSION_CONTROL] operator_gate_status=${operatorGateNotification.status}`);
    console.log(`[SESSION_CONTROL] operator_gate_reason=${operatorGateNotification.reason}`);
  }
}
