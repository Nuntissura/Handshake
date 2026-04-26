#!/usr/bin/env node

import fs from "node:fs";
import path from "node:path";
import { execFileSync } from "node:child_process";
import { registerFailCaptureHook, failWithMemory } from "../../../roles_shared/scripts/lib/fail-capture-lib.mjs";
import { buildSteeringPrompt, resolveRoleConfig } from "../../../roles_shared/scripts/session/session-control-lib.mjs";
import { buildEphemeralContextBlock } from "../../../roles_shared/scripts/session/ephemeral-injection-lib.mjs";
import { loadSessionRegistry } from "../../../roles_shared/scripts/session/session-registry-lib.mjs";
import { sessionKey } from "../../../roles_shared/scripts/session/session-policy.mjs";
import { parseJsonFile, parseJsonlFile } from "../../../roles_shared/scripts/lib/wp-communications-lib.mjs";
import {
  normalizeRelayEscalationPolicy,
  relayEscalationPolicyBudgetLabel,
} from "../../../roles_shared/scripts/lib/wp-relay-policy-lib.mjs";
import { GOV_ROOT_REPO_REL, resolveWorkPacketPath, WORK_PACKET_STORAGE_ROOT_REPO_REL } from "../../../roles_shared/scripts/lib/runtime-paths.mjs";
import { checkAllNotifications, checkNotifications } from "../../../roles_shared/scripts/wp/wp-check-notifications.mjs";
import { evaluateWpCommunicationBoundary, evaluateWpCommunicationHealth } from "../../../roles_shared/scripts/lib/wp-communication-health-lib.mjs";
import { evaluateWpRelayEscalation } from "../../../roles_shared/scripts/lib/wp-relay-escalation-lib.mjs";
import {
  nextQueuedControlRequest,
  pendingControlQueueCount,
  steerActionForSession,
} from "./lib/orchestrator-steer-lib.mjs";
import { activationReadinessRequiresActivationManager } from "./lib/workflow-lane-guidance-lib.mjs";
import {
  buildRelayDispatchPrompt,
  deriveRelayEnvelope,
  preferredTargetSession,
} from "./lib/manual-relay-envelope-lib.mjs";
import { capturePreTaskSnapshot } from "../../../roles_shared/scripts/memory/memory-snapshot.mjs";

const wpId = String(process.argv[2] || "").trim();
const debugMode = process.argv.slice(3).some((arg) => String(arg || "").trim() === "--debug");
const explicitTargetRole = (() => {
  for (const candidate of process.argv.slice(3)) {
    const value = String(candidate || "").trim();
    if (!value.startsWith("--target-role=")) continue;
    return normalizeRole(value.slice("--target-role=".length));
  }
  return "";
})();
const explicitTargetSession = (() => {
  for (const candidate of process.argv.slice(3)) {
    const value = String(candidate || "").trim();
    if (!value.startsWith("--target-session=")) continue;
    return String(value.slice("--target-session=".length) || "").trim();
  }
  return "";
})();
const requestedModel = (() => {
  for (const candidate of process.argv.slice(3)) {
    const value = String(candidate || "").trim().toUpperCase();
    if (!value || value.startsWith("--")) continue;
    if (["PRIMARY", "FALLBACK"].includes(value)) return value;
    fail(`Invalid model selector: ${value} (expected PRIMARY or FALLBACK)`);
  }
  return "PRIMARY";
})();
const ACTIVE_ROLE_SET = new Set(["ACTIVATION_MANAGER", "CODER", "WP_VALIDATOR", "INTEGRATION_VALIDATOR"]);
const sessionControlEnv = {
  ...process.env,
  ...(debugMode ? { HANDSHAKE_SESSION_CONTROL_DEBUG: "1" } : {}),
};

registerFailCaptureHook("orchestrator-steer-next.mjs", { role: "ORCHESTRATOR" });

function fail(message, details = []) {
  failWithMemory("orchestrator-steer-next.mjs", message, { role: "ORCHESTRATOR", details });
}

function runGit(args) {
  return execFileSync("git", args, { encoding: "utf8", stdio: ["ignore", "pipe", "pipe"], windowsHide: true }).trim();
}

function parseSingleField(text, label) {
  const re = new RegExp(`^\\s*-\\s*(?:\\*\\*)?${label}(?:\\*\\*)?\\s*:\\s*(.+)\\s*$`, "mi");
  const match = String(text || "").match(re);
  return match ? match[1].trim() : "";
}

function normalizeRole(raw) {
  return String(raw || "").trim().toUpperCase();
}

function loadRuntimeStatus(packetText) {
  const runtimeStatusFile = parseSingleField(packetText, "WP_RUNTIME_STATUS_FILE");
  const receiptsFile = parseSingleField(packetText, "WP_RECEIPTS_FILE");
  if (!runtimeStatusFile) {
    fail("Packet does not declare WP_RUNTIME_STATUS_FILE");
  }
  if (!fs.existsSync(runtimeStatusFile)) {
    fail("WP runtime status file is missing", [runtimeStatusFile]);
  }
  return {
    runtimeStatusFile,
    receiptsFile,
    runtimeStatus: parseJsonFile(runtimeStatusFile),
  };
}

if (!wpId || !/^WP-/.test(wpId)) {
  fail("Usage: node .GOV/roles/orchestrator/scripts/orchestrator-steer-next.mjs WP-{ID} [PRIMARY|FALLBACK] [--target-role=ROLE] [--target-session=SESSION]");
}

if (debugMode) {
  console.log("[ORCHESTRATOR_STEER_NEXT] debug_mode=enabled");
}

const packetPath = resolveWorkPacketPath(wpId)?.packetPath
  || path.join(WORK_PACKET_STORAGE_ROOT_REPO_REL, `${wpId}.md`);
if (!fs.existsSync(packetPath)) {
  fail("Packet file is missing", [packetPath]);
}

const packetText = fs.readFileSync(packetPath, "utf8");
const workflowLane = String(parseSingleField(packetText, "WORKFLOW_LANE") || "").trim().toUpperCase();
if (workflowLane !== "ORCHESTRATOR_MANAGED") {
  fail("orchestrator-steer-next is only valid for orchestrator-managed lanes", [`WORKFLOW_LANE=${workflowLane || "<missing>"}`]);
}

const { runtimeStatusFile, receiptsFile, runtimeStatus } = loadRuntimeStatus(packetText);
const receipts = receiptsFile && fs.existsSync(receiptsFile) ? parseJsonlFile(receiptsFile) : [];

// RGF-145: pre-task snapshot before steering evaluation
capturePreTaskSnapshot({
  snapshotType: "PRE_STEERING",
  wpId,
  triggerScript: "orchestrator-steer-next.mjs",
  context: {
    nextExpectedActor: runtimeStatus.next_expected_actor || "",
    waitingOn: runtimeStatus.waiting_on || "",
    workflowLane,
    receiptCount: receipts.length,
    lastReceiptKind: receipts.at(-1)?.receipt_kind || "",
  },
});

const communicationEvaluation = evaluateWpCommunicationHealth({
  wpId,
  stage: "STATUS",
  packetPath,
  packetContent: packetText,
  workflowLane,
  packetFormatVersion: parseSingleField(packetText, "PACKET_FORMAT_VERSION"),
  communicationContract: parseSingleField(packetText, "COMMUNICATION_CONTRACT"),
  communicationHealthGate: parseSingleField(packetText, "COMMUNICATION_HEALTH_GATE"),
  receipts,
  runtimeStatus,
});
const pendingNotifications = Object.values(checkAllNotifications({ wpId })).flatMap((entry) => entry.notifications || []);
const repoRoot = runGit(["rev-parse", "--show-toplevel"]);
const { registry } = loadSessionRegistry(repoRoot);
const relayEscalation = evaluateWpRelayEscalation({
  wpId,
  runtimeStatus,
  communicationEvaluation,
  receipts,
  pendingNotifications,
  registrySessions: registry.sessions || [],
});
const relayPolicy = normalizeRelayEscalationPolicy(runtimeStatus?.relay_escalation_policy);
const boundaryEvaluation = evaluateWpCommunicationBoundary({
  stage: "STATUS",
  statusEvaluation: communicationEvaluation,
  runtimeStatus,
  latestReceipt: receipts.at(-1) || null,
  pendingNotifications,
});
const boundaryIssues = (boundaryEvaluation.issues || []).filter((issue) => {
  if (!issue.startsWith("runtime.attention_required")) return true;
  return relayEscalation.status === "NOT_APPLICABLE";
});
if (!explicitTargetRole && communicationEvaluation.applicable && boundaryIssues.length > 0) {
  fail("Runtime route drift prevents mechanical relay", boundaryIssues);
}

const activationGate = activationReadinessRequiresActivationManager(wpId);
const defaultNextActor = activationGate.requiresActivationManager
  ? "ACTIVATION_MANAGER"
  : normalizeRole(runtimeStatus.next_expected_actor);
const nextActor = explicitTargetRole || defaultNextActor;
if (!ACTIVE_ROLE_SET.has(nextActor)) {
  fail("No governed next actor is currently projected for automatic relay", [
    `next_expected_actor=${runtimeStatus.next_expected_actor || "<missing>"}`,
    `waiting_on=${runtimeStatus.waiting_on || "<missing>"}`,
    runtimeStatusFile,
  ]);
}

const governedSession = (registry.sessions || []).find((entry) => entry.session_key === sessionKey(nextActor, wpId)) || null;
const roleConfig = resolveRoleConfig(nextActor, wpId);
if (!roleConfig) {
  fail(`No role config resolved for ${nextActor}`);
}

const commandScript = path.join(GOV_ROOT_REPO_REL, "roles", "orchestrator", "scripts", "session-control-command.mjs");
const action = steerActionForSession(governedSession);
const nextSession = explicitTargetRole
  ? (explicitTargetSession || preferredTargetSession(runtimeStatus, governedSession))
  : nextActor === "ACTIVATION_MANAGER"
  ? sessionKey("ACTIVATION_MANAGER", wpId)
  : preferredTargetSession(runtimeStatus, governedSession);
const queuedControlCount = pendingControlQueueCount(governedSession);
const queuedControlRequest = nextQueuedControlRequest(governedSession);
if (nextActor !== "ACTIVATION_MANAGER" && action === "SEND_PROMPT" && queuedControlCount > 0 && queuedControlRequest) {
  console.log(`[ORCHESTRATOR_STEER_NEXT] wp_id=${wpId}`);
  console.log(`[ORCHESTRATOR_STEER_NEXT] next_actor=${nextActor}`);
  console.log(`[ORCHESTRATOR_STEER_NEXT] next_session=${nextSession || "<none>"}`);
  if (explicitTargetRole) {
    console.log(`[ORCHESTRATOR_STEER_NEXT] explicit_target_role=${explicitTargetRole}`);
    console.log(`[ORCHESTRATOR_STEER_NEXT] explicit_target_session=${explicitTargetSession || "<none>"}`);
  }
  console.log(`[ORCHESTRATOR_STEER_NEXT] waiting_on=${runtimeStatus.waiting_on || "<missing>"}`);
  console.log(`[ORCHESTRATOR_STEER_NEXT] action=${action}`);
  console.log(`[ORCHESTRATOR_STEER_NEXT] queue_pending=${queuedControlCount}`);
  console.log(`[ORCHESTRATOR_STEER_NEXT] queued_command=${queuedControlRequest.command_kind || "<unknown>"}`);
  console.log(`[ORCHESTRATOR_STEER_NEXT] queued_at=${queuedControlRequest.queued_at || "<no-ts>"}`);
  if (queuedControlRequest.blocking_command_id) {
    console.log(`[ORCHESTRATOR_STEER_NEXT] blocking_command=${queuedControlRequest.blocking_command_id}`);
  }
  if (queuedControlRequest.summary) {
    console.log(`[ORCHESTRATOR_STEER_NEXT] queued_summary=${queuedControlRequest.summary}`);
  }
  console.log("[ORCHESTRATOR_STEER_NEXT] state=queue-backed follow-up already exists for this governed session; wait for broker drain instead of sending another prompt");
  process.exit(0);
}
let envelope = null;
let prompt = "";
if (nextActor === "ACTIVATION_MANAGER") {
  prompt = [
    buildSteeringPrompt({ role: nextActor, wpId, roleConfig }),
    buildEphemeralContextBlock({
      source: "ACTIVATION_GATE_OVERRIDE [CX-ACT-OVERRIDE-001]",
      trust: "required",
      body: [
        "ACTIVATION_GATE_OVERRIDE [CX-ACT-OVERRIDE-001]",
        `- ACTIVATION_READINESS_PATH: ${activationGate.readiness.path}`,
        `- ACTIVATION_READINESS_VERDICT: ${activationGate.readiness.verdict}`,
        "- REASON: Downstream governed lanes remain blocked until Activation Manager writes truthful readiness for the signed packet/worktree state.",
        "- REQUIRED_NOW: refresh packet/worktree/backup/readiness artifacts for the current signed packet and write/update ACTIVATION_READINESS before any coder or validator launch.",
      ].join("\n"),
    }),
  ].join("\n");
} else {
  const targetNotifications = checkNotifications({ wpId, role: nextActor, session: nextSession });
  if (explicitTargetRole && (targetNotifications.notifications || []).length === 0) {
    fail("Explicit target role has no pending routed notification to dispatch", [
      `target_role=${nextActor}`,
      `target_session=${nextSession || "<none>"}`,
      `runtime_next_expected_actor=${runtimeStatus.next_expected_actor || "<missing>"}`,
      `runtime_waiting_on=${runtimeStatus.waiting_on || "<missing>"}`,
    ]);
  }
  envelope = deriveRelayEnvelope({
    wpId,
    runtimeStatus,
    nextActor,
    targetSession: nextSession,
    notifications: targetNotifications,
    dispatchAction: action,
  });
  prompt = buildRelayDispatchPrompt({
    basePrompt: buildSteeringPrompt({ role: nextActor, wpId, roleConfig }),
    envelope,
    contextLabel: "GOVERNED_ROUTE_CONTEXT [CX-ROUTE-001]",
    messageLabel: "DIRECT_ROLE_MESSAGE [CX-ROUTE-002]",
    terminalInstructions: [
      "Treat DIRECT_ROLE_MESSAGE as the current receipt/notification-derived payload for WORKFLOW_LANE=ORCHESTRATOR_MANAGED.",
      "Do not rediscover the relay type from scratch before acting; use RELAY_KIND and SOURCE_KIND as the current route context.",
      `If you emit a paired acknowledgement, question, or response, preserve correlation_id=${envelope.correlationId} when applicable.`,
    ],
  });
}

console.log(`[ORCHESTRATOR_STEER_NEXT] wp_id=${wpId}`);
console.log(`[ORCHESTRATOR_STEER_NEXT] next_actor=${nextActor}`);
console.log(`[ORCHESTRATOR_STEER_NEXT] next_session=${nextSession || "<none>"}`);
if (explicitTargetRole) {
  console.log(`[ORCHESTRATOR_STEER_NEXT] explicit_target_role=${explicitTargetRole}`);
  console.log(`[ORCHESTRATOR_STEER_NEXT] explicit_target_session=${explicitTargetSession || "<none>"}`);
}
console.log(`[ORCHESTRATOR_STEER_NEXT] waiting_on=${runtimeStatus.waiting_on || "<missing>"}`);
console.log(`[ORCHESTRATOR_STEER_NEXT] relay_status=${relayEscalation.status}`);
if (relayEscalation.status !== "NOT_APPLICABLE") {
  console.log(`[ORCHESTRATOR_STEER_NEXT] relay_summary=${relayEscalation.summary}`);
  if (relayPolicy) {
    console.log(`[ORCHESTRATOR_STEER_NEXT] relay_failure_class=${relayPolicy.failure_class}`);
    console.log(`[ORCHESTRATOR_STEER_NEXT] relay_policy_state=${relayPolicy.policy_state}`);
    console.log(`[ORCHESTRATOR_STEER_NEXT] relay_next_strategy=${relayPolicy.next_strategy}`);
    console.log(`[ORCHESTRATOR_STEER_NEXT] relay_budget_scope=${relayPolicy.budget_scope}`);
    if (relayPolicy.budget_scope !== "NONE") {
      console.log(`[ORCHESTRATOR_STEER_NEXT] relay_budget=${relayEscalationPolicyBudgetLabel(relayPolicy)}`);
    }
    console.log(`[ORCHESTRATOR_STEER_NEXT] relay_policy_reason_code=${relayPolicy.reason_code}`);
    console.log(`[ORCHESTRATOR_STEER_NEXT] relay_policy_summary=${relayPolicy.summary}`);
  }
}
if (nextActor === "ACTIVATION_MANAGER") {
  console.log(`[ORCHESTRATOR_STEER_NEXT] activation_readiness_verdict=${activationGate.readiness.verdict}`);
  console.log(`[ORCHESTRATOR_STEER_NEXT] activation_readiness_path=${activationGate.readiness.path}`);
}
console.log(`[ORCHESTRATOR_STEER_NEXT] action=${action}`);
if (envelope) {
  console.log(`[ORCHESTRATOR_STEER_NEXT] relay_kind=${envelope.relayKind}`);
  console.log(`[ORCHESTRATOR_STEER_NEXT] source_kind=${envelope.sourceKind}`);
  console.log("DIRECT_ROLE_MESSAGE [CX-ROUTE-002]");
  console.log(`- ${envelope.message}`);
}

if (action === "START_SESSION") {
  execFileSync(process.execPath, [commandScript, "START_SESSION", nextActor, wpId, "", requestedModel], {
    stdio: "inherit",
    env: sessionControlEnv,
    windowsHide: true,
  });
  console.log("[ORCHESTRATOR_STEER_NEXT] state=start_session_requested; wait for the governed startup turn to register and settle before sending a follow-up prompt");
  process.exit(0);
}

execFileSync(process.execPath, [commandScript, "SEND_PROMPT", nextActor, wpId, prompt, requestedModel], {
  stdio: "inherit",
  env: sessionControlEnv,
  windowsHide: true,
});
