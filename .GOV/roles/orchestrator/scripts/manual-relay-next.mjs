#!/usr/bin/env node

import fs from "node:fs";
import path from "node:path";
import { loadSessionRegistry } from "../../../roles_shared/scripts/session/session-registry-lib.mjs";
import { resolveRoleConfig } from "../../../roles_shared/scripts/session/session-control-lib.mjs";
import { sessionKey } from "../../../roles_shared/scripts/session/session-policy.mjs";
import { parseJsonFile } from "../../../roles_shared/scripts/lib/wp-communications-lib.mjs";
import { repoPathAbs, resolveWorkPacketPath, REPO_ROOT } from "../../../roles_shared/scripts/lib/runtime-paths.mjs";
import { checkNotifications } from "../../../roles_shared/scripts/wp/wp-check-notifications.mjs";
import { steerActionForSession } from "./lib/orchestrator-steer-lib.mjs";

const ACTIVE_ROLE_SET = new Set(["CODER", "WP_VALIDATOR", "INTEGRATION_VALIDATOR"]);

function fail(message, details = []) {
  console.error(`[MANUAL_RELAY_NEXT] ${message}`);
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

function preferredTargetSession(runtimeStatus = {}, governedSession = null) {
  return normalizeSession(runtimeStatus?.next_expected_session)
    || normalizeSession(governedSession?.session_id)
    || null;
}

const wpId = String(process.argv[2] || "").trim();
if (!wpId || !/^WP-/.test(wpId)) {
  fail("Usage: node .GOV/roles/orchestrator/scripts/manual-relay-next.mjs WP-{ID}");
}

const packetPath = resolveWorkPacketPath(wpId)?.packetPath
  || path.join(".GOV", "task_packets", `${wpId}.md`);
const packetAbsPath = repoPathAbs(packetPath);
if (!fs.existsSync(packetAbsPath)) {
  fail("Packet file is missing", [packetPath]);
}

const packetText = fs.readFileSync(packetAbsPath, "utf8");
const workflowLane = normalizeRole(parseSingleField(packetText, "WORKFLOW_LANE"));
if (workflowLane !== "MANUAL_RELAY") {
  fail("manual-relay-next is only valid for MANUAL_RELAY lanes", [`WORKFLOW_LANE=${workflowLane || "<missing>"}`]);
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
const dispatchAction = steerActionForSession(governedSession);
const targetSession = preferredTargetSession(runtimeStatus, governedSession);
const notifications = checkNotifications({ wpId, role: nextActor, session: targetSession });

console.log(`[MANUAL_RELAY_NEXT] wp_id=${wpId}`);
console.log(`[MANUAL_RELAY_NEXT] workflow_lane=${workflowLane}`);
console.log(`[MANUAL_RELAY_NEXT] next_actor=${nextActor}`);
console.log(`[MANUAL_RELAY_NEXT] next_session=${targetSession || "<none>"}`);
console.log(`[MANUAL_RELAY_NEXT] waiting_on=${runtimeStatus.waiting_on || "<missing>"}`);
console.log(`[MANUAL_RELAY_NEXT] runtime_status=${runtimeStatus.runtime_status || "<missing>"}`);
console.log(`[MANUAL_RELAY_NEXT] current_phase=${runtimeStatus.current_phase || "<missing>"}`);
console.log(`[MANUAL_RELAY_NEXT] dispatch_action=${dispatchAction}`);
console.log(`[MANUAL_RELAY_NEXT] notifications_pending=${notifications.pendingCount}`);
console.log(`[MANUAL_RELAY_NEXT] notifications_by_kind=${JSON.stringify(notifications.byKind || {})}`);
console.log(`[MANUAL_RELAY_NEXT] state=Operator remains the relay; route the lane to ${nextActor} using governed session commands.`);
console.log(`[MANUAL_RELAY_NEXT] next_commands=just active-lane-brief ${nextActor} ${wpId} | just check-notifications ${wpId} ${nextActor} ${targetSession || "<your-session>"} | just manual-relay-dispatch ${wpId} | just session-registry-status ${wpId}`);
