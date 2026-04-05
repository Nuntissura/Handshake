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

function normalizeText(value, fallback = "<none>") {
  const raw = String(value || "").trim();
  return raw || fallback;
}

function preferredTargetSession(runtimeStatus = {}, governedSession = null) {
  return normalizeSession(runtimeStatus?.next_expected_session)
    || normalizeSession(governedSession?.session_id)
    || null;
}

function relayKindForSourceKind(sourceKind) {
  const normalized = normalizeRole(sourceKind);
  if (["CODER_HANDOFF", "HANDOFF"].includes(normalized)) return "HANDOFF";
  if (["VALIDATOR_QUERY", "REVIEW_REQUEST", "SPEC_GAP"].includes(normalized)) return "QUESTION";
  if (["VALIDATOR_RESPONSE", "REVIEW_RESPONSE", "SPEC_CONFIRMATION"].includes(normalized)) return "ANSWER";
  if (["VALIDATOR_REVIEW"].includes(normalized)) return "VERDICT";
  if (["CODER_INTENT", "VALIDATOR_KICKOFF"].includes(normalized)) return "INTENT";
  if (["THREAD_MESSAGE"].includes(normalized)) return "MESSAGE";
  if (["REPAIR"].includes(normalized)) return "REPAIR";
  if (["WORKFLOW_INVALIDITY"].includes(normalized)) return "INVALIDITY";
  return "ACTION";
}

function targetOpenReviewItem(runtimeStatus = {}, nextActor = "", targetSession = null) {
  const items = Array.isArray(runtimeStatus?.open_review_items) ? runtimeStatus.open_review_items : [];
  return items
    .filter((item) => normalizeRole(item?.target_role) === nextActor)
    .filter((item) => {
      const itemTargetSession = normalizeSession(item?.target_session);
      if (!targetSession) return true;
      if (!itemTargetSession) return true;
      return itemTargetSession === targetSession;
    })
    .sort((left, right) => String(right?.updated_at || right?.opened_at || "").localeCompare(String(left?.updated_at || left?.opened_at || "")))[0] || null;
}

function latestTargetNotification(notifications = []) {
  return [...(Array.isArray(notifications) ? notifications : [])]
    .sort((left, right) => String(right?.timestamp_utc || "").localeCompare(String(left?.timestamp_utc || "")))[0] || null;
}

function deriveRelayEnvelope({ runtimeStatus, nextActor, targetSession, notifications, dispatchAction }) {
  const notification = latestTargetNotification(notifications?.notifications || []);
  const reviewItem = targetOpenReviewItem(runtimeStatus, nextActor, targetSession);
  const sourceKind = normalizeRole(notification?.source_kind || reviewItem?.receipt_kind || runtimeStatus?.waiting_on || "ACTION");
  const relayKind = relayKindForSourceKind(sourceKind);
  const fromRole = normalizeRole(notification?.source_role || reviewItem?.opened_by_role || "RUNTIME");
  const fromSession = normalizeSession(notification?.source_session || reviewItem?.opened_by_session);
  const message = normalizeText(notification?.summary || reviewItem?.summary || `Runtime is waiting on ${runtimeStatus?.waiting_on || "the next governed action"}.`);
  const correlationId = normalizeText(notification?.correlation_id || reviewItem?.correlation_id);
  const ackRequired = Boolean(reviewItem?.requires_ack);

  return {
    fromRole,
    fromSession,
    toRole: nextActor,
    toSession: targetSession,
    relayKind,
    sourceKind,
    correlationId,
    ackRequired,
    message,
    operatorExplainer: [
      `Operator is broker-only on MANUAL_RELAY; do not mix this role message with hard-gate commentary.`,
      `Runtime projects ${nextActor}${targetSession ? `:${targetSession}` : ""} next because waiting_on=${runtimeStatus?.waiting_on || "<missing>"} during ${runtimeStatus?.current_phase || "<missing>"}.`,
      `Dispatch action is ${dispatchAction}; after the role responds, rerun just manual-relay-next ${wpId}.`,
    ],
  };
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
const envelope = deriveRelayEnvelope({
  runtimeStatus,
  nextActor,
  targetSession,
  notifications,
  dispatchAction,
});

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
console.log(`[MANUAL_RELAY_NEXT] state=Operator remains the relay; use the structured envelope below to separate role-to-role traffic from operator explanation.`);
console.log("RELAY_ENVELOPE [CX-MANUAL-RELAY-001]");
console.log(`- FROM: ${envelope.fromRole}${envelope.fromSession ? `:${envelope.fromSession}` : ""}`);
console.log(`- TO: ${envelope.toRole}${envelope.toSession ? `:${envelope.toSession}` : ""}`);
console.log(`- RELAY_KIND: ${envelope.relayKind}`);
console.log(`- SOURCE_KIND: ${envelope.sourceKind}`);
console.log(`- CORRELATION_ID: ${envelope.correlationId}`);
console.log(`- ACK_REQUIRED: ${envelope.ackRequired ? "YES" : "NO"}`);
console.log("");
console.log("ROLE_TO_ROLE_MESSAGE [CX-MANUAL-RELAY-002]");
console.log(`- ${envelope.message}`);
console.log("");
console.log("OPERATOR_EXPLAINER [CX-MANUAL-RELAY-003]");
for (const line of envelope.operatorExplainer) {
  console.log(`- ${line}`);
}
console.log("");
console.log(`[MANUAL_RELAY_NEXT] next_commands=just active-lane-brief ${nextActor} ${wpId} | just check-notifications ${wpId} ${nextActor} ${targetSession || "<your-session>"} | just manual-relay-dispatch ${wpId} | just session-registry-status ${wpId}`);
