#!/usr/bin/env node

import fs from "node:fs";
import path from "node:path";
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";
import { registerFailCaptureHook, failWithMemory } from "../../../roles_shared/scripts/lib/fail-capture-lib.mjs";
import { buildSteeringPrompt, resolveRoleConfig } from "../../../roles_shared/scripts/session/session-control-lib.mjs";
import { loadSessionRegistry } from "../../../roles_shared/scripts/session/session-registry-lib.mjs";
import { sessionKey } from "../../../roles_shared/scripts/session/session-policy.mjs";
import { defaultSessionOutputFile } from "../../../roles_shared/scripts/session/session-control-lib.mjs";
import { parseJsonFile, parseJsonlFile } from "../../../roles_shared/scripts/lib/wp-communications-lib.mjs";
import { repoPathAbs, resolveWorkPacketPath, REPO_ROOT, WORK_PACKET_STORAGE_ROOT_REPO_REL } from "../../../roles_shared/scripts/lib/runtime-paths.mjs";
import { checkNotifications } from "../../../roles_shared/scripts/wp/wp-check-notifications.mjs";
import { steerActionForSession } from "./lib/orchestrator-steer-lib.mjs";
import {
  buildManualRelayDispatchPrompt,
  deriveManualRelayEnvelope,
  preferredTargetSession,
} from "./lib/manual-relay-envelope-lib.mjs";
import { capturePreTaskSnapshot } from "../../../roles_shared/scripts/memory/memory-snapshot.mjs";
import { evaluateWpCommunicationHealth } from "../../../roles_shared/scripts/lib/wp-communication-health-lib.mjs";
import { evaluatePacketRuntimeProjectionDrift } from "../../../roles_shared/scripts/lib/packet-runtime-projection-lib.mjs";

const ACTIVE_ROLE_SET = new Set(["CODER", "WP_VALIDATOR", "INTEGRATION_VALIDATOR"]);
const SESSION_CONTROL_COMMAND_PATH = path.resolve(
  path.dirname(fileURLToPath(import.meta.url)),
  "session-control-command.mjs",
);

registerFailCaptureHook("manual-relay-dispatch.mjs", { role: "CLASSIC_ORCHESTRATOR" });

function fail(message, details = []) {
  failWithMemory("manual-relay-dispatch.mjs", message, { role: "CLASSIC_ORCHESTRATOR", details });
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
const argList = process.argv.slice(3);
const requestedModel = (() => {
  for (const candidate of argList) {
    const value = String(candidate || "").trim().toUpperCase();
    if (!value || value.startsWith("--")) continue;
    return value;
  }
  return "PRIMARY";
})();
const debugMode = argList.some((arg) => String(arg || "").trim() === "--debug");
const sessionControlEnv = {
  ...process.env,
  ...(debugMode ? { HANDSHAKE_SESSION_CONTROL_DEBUG: "1" } : {}),
};
if (!wpId || !/^WP-/.test(wpId)) {
  fail("Usage: node .GOV/roles/orchestrator/scripts/manual-relay-dispatch.mjs WP-{ID} [PRIMARY|FALLBACK] [--debug]");
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

const receiptsFile = parseSingleField(packetText, "WP_RECEIPTS_FILE");
const runtimeStatus = parseJsonFile(runtimeStatusFile);
const receipts = receiptsFile && fs.existsSync(repoPathAbs(receiptsFile)) ? parseJsonlFile(receiptsFile) : [];
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
const packetRuntimeDrift = evaluatePacketRuntimeProjectionDrift(packetText, runtimeStatus, {
  communicationEvaluation,
});
if (!packetRuntimeDrift.ok) {
  fail("Packet/runtime projection drift blocks manual relay dispatch until workflow truth is reconciled", [
    packetRuntimeDrift.owner_summary || "drift owner unknown",
    ...packetRuntimeDrift.issues,
    packetPath,
    runtimeStatusFile,
  ]);
}
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

// RGF-145: pre-task snapshot before manual relay dispatch
capturePreTaskSnapshot({
  snapshotType: "PRE_RELAY_DISPATCH",
  wpId,
  triggerScript: "manual-relay-dispatch.mjs",
  context: {
    nextActor,
    workflowLane,
    waitingOn: runtimeStatus.waiting_on || "",
    nextExpectedActor: runtimeStatus.next_expected_actor || "",
  },
});

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
const managedSessionKey = sessionKey(nextActor, wpId);

function runSessionControl(commandKind, args = []) {
  const result = spawnSync(process.execPath, [SESSION_CONTROL_COMMAND_PATH, commandKind, nextActor, wpId, ...args], {
    encoding: "utf8",
    env: sessionControlEnv,
    windowsHide: true,
  });
  const output = `${result.stdout || ""}${result.stderr || ""}`;
  if (output) console.log(output.trimEnd());
  return { code: Number.isFinite(result.status) ? result.status : 1, output };
}

function extractSessionCommandId(output = "") {
  const match = String(output).match(/(?:command_id=|Command\s+)([0-9a-f-]{36})/i);
  return match ? match[1] : "";
}

function outputFileHasUsageLimitText(outputFilePath) {
  if (!fs.existsSync(outputFilePath)) return false;
  try {
    const raw = fs.readFileSync(outputFilePath, "utf8");
    if (!raw) return false;
    return raw.split(/\r?\n/).some((line) => {
      if (!line) return false;
      try {
        const parsed = JSON.parse(line);
        const haystack = [
          parsed.message,
          parsed.error?.message,
          parsed.summary,
          parsed.error,
          parsed.last_agent_message,
        ].filter(Boolean).join(" ");
        return /usage limit|credits|quota exceeded/i.test(haystack);
      } catch {
        return /usage limit|credits|quota exceeded/i.test(line);
      }
    });
  } catch {
    return false;
  }
}

function isUsageLimitFailure(output = "") {
  if (/usage limit|credits|quota exceeded/i.test(String(output || ""))) return true;
  const commandId = extractSessionCommandId(output);
  if (!commandId) return false;
  return outputFileHasUsageLimitText(defaultSessionOutputFile(REPO_ROOT, managedSessionKey, commandId));
}

function isAlreadyHasThreadError(output = "") {
  return /already has thread/i.test(String(output || ""));
}

function isNoSteerableThreadError(output = "") {
  return /no steerable thread id is registered/i.test(String(output || ""));
}

function logUsageLimitDefer(kind) {
  console.log(`[MANUAL_RELAY_DISPATCH] Memory/usage-control deferred for ${nextActor} ${wpId} during ${kind}.`);
  console.log("[MANUAL_RELAY_DISPATCH] Re-run when usage window resets or route with a fresh model profile.");
}

console.log(`[MANUAL_RELAY_DISPATCH] wp_id=${wpId}`);
console.log(`[MANUAL_RELAY_DISPATCH] workflow_lane=${workflowLane}`);
console.log("[MANUAL_RELAY_DISPATCH] lane_owner=CLASSIC_ORCHESTRATOR");
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

let deferred = false;

if (action === "START_SESSION") {
  const startResult = runSessionControl("START_SESSION", ["", requestedModel]);
  if (startResult.code !== 0) {
    if (isAlreadyHasThreadError(startResult.output)) {
      console.log("[MANUAL_RELAY_DISPATCH] Existing session thread detected; reusing existing thread via SEND_PROMPT.");
      action = "SEND_PROMPT";
    } else if (isUsageLimitFailure(startResult.output)) {
      logUsageLimitDefer("START_SESSION");
      deferred = true;
    } else {
      fail("START_SESSION failed", [startResult.output.slice(0, 240)]);
    }
  }
}

if (!deferred) {
  let sendResult = runSessionControl("SEND_PROMPT", [prompt, requestedModel]);
  if (sendResult.code !== 0 && isNoSteerableThreadError(sendResult.output) && action === "START_SESSION") {
    console.log("[MANUAL_RELAY_DISPATCH] SEND_PROMPT failed with no steerable thread; attempting recovery restart path.");
    const closeResult = runSessionControl("CLOSE_SESSION");
    if (closeResult.code === 0) {
      const restartResult = runSessionControl("START_SESSION", ["", requestedModel]);
      if (restartResult.code === 0) {
        sendResult = runSessionControl("SEND_PROMPT", [prompt, requestedModel]);
      } else if (isUsageLimitFailure(restartResult.output)) {
        logUsageLimitDefer("START_SESSION after recovery");
        deferred = true;
      } else if (!isAlreadyHasThreadError(restartResult.output)) {
        fail("START_SESSION recovery failed", [restartResult.output.slice(0, 240)]);
      }
    } else {
      fail("CLOSE_SESSION recovery failed", [closeResult.output.slice(0, 240)]);
    }
  }

  if (!deferred && sendResult.code !== 0 && isUsageLimitFailure(sendResult.output)) {
    logUsageLimitDefer("SEND_PROMPT");
    deferred = true;
  } else if (!deferred && sendResult.code !== 0) {
    fail("SEND_PROMPT failed", [sendResult.output.slice(0, 240)]);
  }
}

if (deferred) process.exit(0);
