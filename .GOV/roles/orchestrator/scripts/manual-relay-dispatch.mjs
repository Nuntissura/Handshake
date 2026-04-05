#!/usr/bin/env node

import fs from "node:fs";
import path from "node:path";
import { execFileSync } from "node:child_process";
import { fileURLToPath } from "node:url";
import { buildSteeringPrompt, resolveRoleConfig } from "../../../roles_shared/scripts/session/session-control-lib.mjs";
import { loadSessionRegistry } from "../../../roles_shared/scripts/session/session-registry-lib.mjs";
import { sessionKey } from "../../../roles_shared/scripts/session/session-policy.mjs";
import { parseJsonFile } from "../../../roles_shared/scripts/lib/wp-communications-lib.mjs";
import { repoPathAbs, resolveWorkPacketPath, REPO_ROOT, WORK_PACKET_STORAGE_ROOT_REPO_REL } from "../../../roles_shared/scripts/lib/runtime-paths.mjs";
import { checkNotifications } from "../../../roles_shared/scripts/wp/wp-check-notifications.mjs";
import { steerActionForSession } from "./lib/orchestrator-steer-lib.mjs";
import {
  buildManualRelayDispatchPrompt,
  deriveManualRelayEnvelope,
  preferredTargetSession,
} from "./lib/manual-relay-envelope-lib.mjs";

const ACTIVE_ROLE_SET = new Set(["CODER", "WP_VALIDATOR", "INTEGRATION_VALIDATOR"]);
const SESSION_CONTROL_COMMAND_PATH = path.resolve(
  path.dirname(fileURLToPath(import.meta.url)),
  "session-control-command.mjs",
);

function fail(message, details = []) {
  console.error(`[MANUAL_RELAY_DISPATCH] ${message}`);
  for (const line of details) console.error(`- ${line}`);
  process.exit(1);
}

function parseSingleField(text, label) {
  const re = new RegExp(`^\\s*-\\s*(?:\\*\\*)?${label}(?:\\*\\*)?\\s*:\\s*(.+)\\s*$`, "mi");
  const match = String(text || "").match(re);
  return match ? match[1].trim() : "";
}

function normalizeRole(value) {
  return String(value || "").trim().toUpperCase();
}

function normalizeSession(value) {
  const raw = String(value || "").trim();
  return raw || null;
}

const wpId = String(process.argv[2] || "").trim();
const requestedModel = String(process.argv[3] || "").trim().toUpperCase() || "PRIMARY";
if (!wpId || !/^WP-/.test(wpId)) {
  fail("Usage: node .GOV/roles/orchestrator/scripts/manual-relay-dispatch.mjs WP-{ID} [PRIMARY|FALLBACK]");
}

const packetPath = resolveWorkPacketPath(wpId)?.packetPath
  || path.join(WORK_PACKET_STORAGE_ROOT_REPO_REL, `${wpId}.md`);
const packetAbsPath = repoPathAbs(packetPath);
if (!fs.existsSync(packetAbsPath)) {
  fail("Packet file is missing", [packetPath]);
}

const packetText = fs.readFileSync(packetAbsPath, "utf8");
const workflowLane = normalizeRole(parseSingleField(packetText, "WORKFLOW_LANE"));
if (workflowLane !== "MANUAL_RELAY") {
  fail("manual-relay-dispatch is only valid for MANUAL_RELAY lanes", [`WORKFLOW_LANE=${workflowLane || "<missing>"}`]);
}

const runtimeStatusFile = parseSingleField(packetText, "WP_RUNTIME_STATUS_FILE");
if (!runtimeStatusFile || !fs.existsSync(repoPathAbs(runtimeStatusFile))) {
  fail("WP runtime status file is missing", [runtimeStatusFile || "<missing>"]);
}

const runtimeStatus = parseJsonFile(runtimeStatusFile);
const nextActor = normalizeRole(runtimeStatus.next_expected_actor);
if (!ACTIVE_ROLE_SET.has(nextActor)) {
  fail("No governed next actor is currently projected for manual relay", [
    `next_expected_actor=${runtimeStatus.next_expected_actor || "<missing>"}`,
    `waiting_on=${runtimeStatus.waiting_on || "<missing>"}`,
    runtimeStatusFile,
  ]);
}

const roleConfig = resolveRoleConfig(nextActor, wpId);
if (!roleConfig) {
  fail(`No role config resolved for ${nextActor}`);
}

const { registry } = loadSessionRegistry(REPO_ROOT);
const governedSession = (registry.sessions || []).find((entry) => entry.session_key === sessionKey(nextActor, wpId)) || null;
const action = steerActionForSession(governedSession);
const targetSession = preferredTargetSession(runtimeStatus, governedSession);
const notifications = checkNotifications({ wpId, role: nextActor, session: targetSession });
const envelope = deriveManualRelayEnvelope({
  wpId,
  runtimeStatus,
  nextActor,
  targetSession,
  notifications,
  dispatchAction: action,
});
const prompt = buildManualRelayDispatchPrompt({
  basePrompt: buildSteeringPrompt({ role: nextActor, wpId, roleConfig }),
  envelope,
});

console.log(`[MANUAL_RELAY_DISPATCH] wp_id=${wpId}`);
console.log(`[MANUAL_RELAY_DISPATCH] workflow_lane=${workflowLane}`);
console.log(`[MANUAL_RELAY_DISPATCH] next_actor=${nextActor}`);
console.log(`[MANUAL_RELAY_DISPATCH] next_session=${targetSession || "<none>"}`);
console.log(`[MANUAL_RELAY_DISPATCH] action=${action}`);
console.log(`[MANUAL_RELAY_DISPATCH] relay_kind=${envelope.relayKind}`);
console.log(`[MANUAL_RELAY_DISPATCH] source_kind=${envelope.sourceKind}`);
console.log("ROLE_TO_ROLE_MESSAGE [CX-MANUAL-RELAY-002]");
console.log(`- ${envelope.message}`);
console.log(`[MANUAL_RELAY_DISPATCH] state=${action === "START_SESSION"
  ? "Starting the governed target session and then dispatching the structured manual relay payload."
  : "Dispatching the structured manual relay payload into the governed target session."}`);

if (action === "START_SESSION") {
  execFileSync(process.execPath, [SESSION_CONTROL_COMMAND_PATH, "START_SESSION", nextActor, wpId, "", requestedModel], {
    stdio: "inherit",
  });
}

execFileSync(process.execPath, [SESSION_CONTROL_COMMAND_PATH, "SEND_PROMPT", nextActor, wpId, prompt, requestedModel], {
  stdio: "inherit",
});
