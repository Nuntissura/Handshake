#!/usr/bin/env node

import fs from "node:fs";
import path from "node:path";
import { execFileSync } from "node:child_process";
import { buildSteeringPrompt, resolveRoleConfig } from "../../../roles_shared/scripts/session/session-control-lib.mjs";
import { loadSessionRegistry } from "../../../roles_shared/scripts/session/session-registry-lib.mjs";
import { sessionKey } from "../../../roles_shared/scripts/session/session-policy.mjs";
import { parseJsonFile, parseJsonlFile } from "../../../roles_shared/scripts/lib/wp-communications-lib.mjs";
import { GOV_ROOT_REPO_REL, resolveWorkPacketPath } from "../../../roles_shared/scripts/lib/runtime-paths.mjs";
import { checkAllNotifications, checkNotifications } from "../../../roles_shared/scripts/wp/wp-check-notifications.mjs";
import { evaluateWpCommunicationBoundary, evaluateWpCommunicationHealth } from "../../../roles_shared/scripts/lib/wp-communication-health-lib.mjs";
import { evaluateWpRelayEscalation } from "../../../roles_shared/scripts/lib/wp-relay-escalation-lib.mjs";
import { steerActionForSession } from "./lib/orchestrator-steer-lib.mjs";
import {
  buildRelayDispatchPrompt,
  deriveRelayEnvelope,
  preferredTargetSession,
} from "./lib/manual-relay-envelope-lib.mjs";

const wpId = String(process.argv[2] || "").trim();
const requestedModel = String(process.argv[3] || "").trim().toUpperCase() || "PRIMARY";
const ACTIVE_ROLE_SET = new Set(["CODER", "WP_VALIDATOR", "INTEGRATION_VALIDATOR"]);

function fail(message, details = []) {
  console.error(`[ORCHESTRATOR_STEER_NEXT] ${message}`);
  for (const line of details) console.error(`- ${line}`);
  process.exit(1);
}

function runGit(args) {
  return execFileSync("git", args, { encoding: "utf8", stdio: ["ignore", "pipe", "pipe"] }).trim();
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
  fail("Usage: node .GOV/roles/orchestrator/scripts/orchestrator-steer-next.mjs WP-{ID} [PRIMARY|FALLBACK]");
}

const packetPath = resolveWorkPacketPath(wpId)?.packetPath
  || path.join(GOV_ROOT_REPO_REL, "task_packets", `${wpId}.md`);
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
const boundaryEvaluation = evaluateWpCommunicationBoundary({
  stage: "STATUS",
  statusEvaluation: communicationEvaluation,
  runtimeStatus,
  latestReceipt: receipts.at(-1) || null,
  pendingNotifications,
});
if (communicationEvaluation.applicable && !boundaryEvaluation.ok) {
  fail("Runtime route drift prevents mechanical relay", boundaryEvaluation.issues);
}

const nextActor = normalizeRole(runtimeStatus.next_expected_actor);
if (!ACTIVE_ROLE_SET.has(nextActor)) {
  fail("No governed next actor is currently projected for automatic relay", [
    `next_expected_actor=${runtimeStatus.next_expected_actor || "<missing>"}`,
    `waiting_on=${runtimeStatus.waiting_on || "<missing>"}`,
    runtimeStatusFile,
  ]);
}

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
const governedSession = (registry.sessions || []).find((entry) => entry.session_key === sessionKey(nextActor, wpId)) || null;
const roleConfig = resolveRoleConfig(nextActor, wpId);
if (!roleConfig) {
  fail(`No role config resolved for ${nextActor}`);
}

const commandScript = path.join(GOV_ROOT_REPO_REL, "roles", "orchestrator", "scripts", "session-control-command.mjs");
const action = steerActionForSession(governedSession);
const nextSession = preferredTargetSession(runtimeStatus, governedSession);
const targetNotifications = checkNotifications({ wpId, role: nextActor, session: nextSession });
const envelope = deriveRelayEnvelope({
  wpId,
  runtimeStatus,
  nextActor,
  targetSession: nextSession,
  notifications: targetNotifications,
  dispatchAction: action,
});
const prompt = buildRelayDispatchPrompt({
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

console.log(`[ORCHESTRATOR_STEER_NEXT] wp_id=${wpId}`);
console.log(`[ORCHESTRATOR_STEER_NEXT] next_actor=${nextActor}`);
console.log(`[ORCHESTRATOR_STEER_NEXT] next_session=${nextSession || "<none>"}`);
console.log(`[ORCHESTRATOR_STEER_NEXT] waiting_on=${runtimeStatus.waiting_on || "<missing>"}`);
console.log(`[ORCHESTRATOR_STEER_NEXT] relay_status=${relayEscalation.status}`);
if (relayEscalation.status !== "NOT_APPLICABLE") {
  console.log(`[ORCHESTRATOR_STEER_NEXT] relay_summary=${relayEscalation.summary}`);
}
console.log(`[ORCHESTRATOR_STEER_NEXT] action=${action}`);
console.log(`[ORCHESTRATOR_STEER_NEXT] relay_kind=${envelope.relayKind}`);
console.log(`[ORCHESTRATOR_STEER_NEXT] source_kind=${envelope.sourceKind}`);
console.log("DIRECT_ROLE_MESSAGE [CX-ROUTE-002]");
console.log(`- ${envelope.message}`);

if (action === "START_SESSION") {
  execFileSync(process.execPath, [commandScript, "START_SESSION", nextActor, wpId, "", requestedModel], {
    stdio: "inherit",
  });
}

execFileSync(process.execPath, [commandScript, "SEND_PROMPT", nextActor, wpId, prompt, requestedModel], {
  stdio: "inherit",
});
