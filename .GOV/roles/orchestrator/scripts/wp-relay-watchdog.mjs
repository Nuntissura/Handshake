#!/usr/bin/env node

import fs from "node:fs";
import path from "node:path";
import { execFileSync, spawnSync } from "node:child_process";
import {
  REPO_ROOT,
  repoPathAbs,
  resolveWorkPacketPath,
  WORK_PACKET_STORAGE_ROOT_ABS,
} from "../../../roles_shared/scripts/lib/runtime-paths.mjs";
import {
  parseJsonFile,
  parseJsonlFile,
} from "../../../roles_shared/scripts/lib/wp-communications-lib.mjs";
import { evaluateWpCommunicationHealth } from "../../../roles_shared/scripts/lib/wp-communication-health-lib.mjs";
import { evaluateWpRelayEscalation } from "../../../roles_shared/scripts/lib/wp-relay-escalation-lib.mjs";
import { checkAllNotifications } from "../../../roles_shared/scripts/wp/wp-check-notifications.mjs";
import { appendWpReceipt } from "../../../roles_shared/scripts/wp/wp-receipt-append.mjs";
import { loadSessionRegistry } from "../../../roles_shared/scripts/session/session-registry-lib.mjs";
import { SESSION_CONTROL_BROKER_STATE_FILE } from "../../../roles_shared/scripts/session/session-policy.mjs";
import {
  activeRunsForTarget,
  buildRelayWatchdogSummary,
  deriveRelayWatchdogDecision,
} from "./lib/wp-relay-watchdog-lib.mjs";

const ORCHESTRATOR_STEER_SCRIPT_PATH = path.resolve(REPO_ROOT, ".GOV/roles/orchestrator/scripts/orchestrator-steer-next.mjs");
const SESSION_STALL_SCAN_SCRIPT_PATH = path.resolve(REPO_ROOT, ".GOV/roles_shared/scripts/session/session-stall-scan.mjs");
const WATCHDOG_SESSION = "ORCHESTRATOR_WATCHDOG";
const ACTIVE_TARGET_ROLE_VALUES = new Set(["CODER", "WP_VALIDATOR", "INTEGRATION_VALIDATOR"]);

function sleep(ms) {
  Atomics.wait(new Int32Array(new SharedArrayBuffer(4)), 0, 0, Math.max(1, Math.trunc(ms)));
}

function parseSingleField(text, label) {
  const re = new RegExp(`^\\s*-\\s*(?:\\*\\*)?${label}(?:\\*\\*)?\\s*:\\s*(.+)\\s*$`, "mi");
  const match = String(text || "").match(re);
  return match ? match[1].trim() : "";
}

function normalizeRole(value = "") {
  return String(value || "").trim().toUpperCase();
}

function parseArgs(argv = process.argv.slice(2)) {
  const args = {
    wpId: "",
    loop: false,
    intervalSeconds: 30,
    allowWatchSteer: true,
  };
  const rest = [...argv];
  if (rest[0] && !String(rest[0]).startsWith("--")) {
    args.wpId = String(rest.shift() || "").trim();
  }
  for (let index = 0; index < rest.length; index += 1) {
    const token = String(rest[index] || "").trim();
    if (token === "--loop") {
      args.loop = true;
      continue;
    }
    if (token === "--no-watch-steer") {
      args.allowWatchSteer = false;
      continue;
    }
    if (token === "--interval-seconds") {
      const next = Number.parseInt(String(rest[index + 1] || "").trim(), 10);
      if (!Number.isInteger(next) || next < 1) {
        throw new Error("--interval-seconds requires an integer >= 1");
      }
      args.intervalSeconds = next;
      index += 1;
      continue;
    }
    throw new Error(`Unknown argument: ${token}`);
  }
  return args;
}

function loadBrokerState() {
  const brokerStatePath = path.resolve(REPO_ROOT, SESSION_CONTROL_BROKER_STATE_FILE);
  if (!fs.existsSync(brokerStatePath)) {
    return { active_runs: [] };
  }
  try {
    return JSON.parse(fs.readFileSync(brokerStatePath, "utf8"));
  } catch {
    return { active_runs: [] };
  }
}

function listManagedPacketIds() {
  if (!fs.existsSync(WORK_PACKET_STORAGE_ROOT_ABS)) return [];
  const results = [];
  for (const entry of fs.readdirSync(WORK_PACKET_STORAGE_ROOT_ABS, { withFileTypes: true })) {
    if (entry.name === "stubs" || entry.name === "_archive") continue;
    if (entry.isDirectory() && /^WP-/.test(entry.name)) {
      const packetPath = path.join(WORK_PACKET_STORAGE_ROOT_ABS, entry.name, "packet.md");
      if (fs.existsSync(packetPath)) results.push(entry.name);
      continue;
    }
    if (entry.isFile() && /^WP-.*\.md$/i.test(entry.name)) {
      results.push(entry.name.replace(/\.md$/i, ""));
    }
  }
  return [...new Set(results)].sort();
}

function runStallScan(role, wpId) {
  const result = spawnSync(process.execPath, [SESSION_STALL_SCAN_SCRIPT_PATH, role, wpId], {
    cwd: REPO_ROOT,
    encoding: "utf8",
    windowsHide: true,
  });
  const stdout = String(result.stdout || "").trim();
  const stderr = String(result.stderr || "").trim();
  if (result.status === 0) {
    return { status: "CLEAR", summary: stdout || stderr || "stall scan clear" };
  }
  return { status: "STALL", summary: stdout || stderr || "stall scan detected a stuck pattern" };
}

function loadWpRelayInputs(wpId, registrySessions) {
  const resolved = resolveWorkPacketPath(wpId);
  if (!resolved?.packetPath || !fs.existsSync(resolved.packetAbsPath)) {
    return { wpId, applicable: false, skipReason: "PACKET_MISSING" };
  }

  const packetText = fs.readFileSync(resolved.packetAbsPath, "utf8");
  const workflowLane = String(parseSingleField(packetText, "WORKFLOW_LANE") || "").trim().toUpperCase();
  if (workflowLane !== "ORCHESTRATOR_MANAGED") {
    return { wpId, applicable: false, skipReason: `WORKFLOW_LANE_${workflowLane || "MISSING"}` };
  }

  const runtimeStatusFile = parseSingleField(packetText, "WP_RUNTIME_STATUS_FILE");
  if (!runtimeStatusFile || !fs.existsSync(repoPathAbs(runtimeStatusFile))) {
    return { wpId, applicable: false, skipReason: "RUNTIME_STATUS_MISSING" };
  }

  const receiptsFile = parseSingleField(packetText, "WP_RECEIPTS_FILE");
  const runtimeStatus = parseJsonFile(runtimeStatusFile);
  const receipts = receiptsFile && fs.existsSync(repoPathAbs(receiptsFile)) ? parseJsonlFile(receiptsFile) : [];
  const communicationEvaluation = evaluateWpCommunicationHealth({
    wpId,
    stage: "STATUS",
    packetPath: resolved.packetPath,
    packetContent: packetText,
    workflowLane,
    packetFormatVersion: parseSingleField(packetText, "PACKET_FORMAT_VERSION"),
    communicationContract: parseSingleField(packetText, "COMMUNICATION_CONTRACT"),
    communicationHealthGate: parseSingleField(packetText, "COMMUNICATION_HEALTH_GATE"),
    receipts,
    runtimeStatus,
  });
  const pendingNotifications = Object.values(checkAllNotifications({ wpId })).flatMap((entry) => entry.notifications || []);
  const relayStatus = evaluateWpRelayEscalation({
    wpId,
    runtimeStatus,
    communicationEvaluation,
    receipts,
    pendingNotifications,
    registrySessions,
  });
  return {
    wpId,
    applicable: true,
    packetPath: resolved.packetPath,
    runtimeStatus,
    relayStatus,
  };
}

function steerWp(wpId) {
  const output = execFileSync(process.execPath, [ORCHESTRATOR_STEER_SCRIPT_PATH, wpId, "PRIMARY"], {
    cwd: REPO_ROOT,
    encoding: "utf8",
    stdio: ["ignore", "pipe", "pipe"],
    windowsHide: true,
  });
  return output.split(/\r?\n/).map((line) => line.trim()).filter(Boolean);
}

function evaluateWp(wpId, {
  registrySessions,
  brokerState,
  allowWatchSteer,
} = {}) {
  const base = loadWpRelayInputs(wpId, registrySessions);
  if (!base.applicable) {
    return {
      wpId,
      action: "SKIP",
      summary: `RELAY_WATCHDOG | wp=${wpId} | decision=SKIP | reason=${base.skipReason}`,
      skipped: true,
      reason: base.skipReason,
    };
  }

  const targetRole = normalizeRole(base.relayStatus?.target_role);
  const targetSession = base.relayStatus?.target_session || null;
  const activeRuns = activeRunsForTarget(brokerState?.active_runs, {
    wpId,
    role: targetRole,
    session: targetSession,
  });
  const stallScan = activeRuns.length > 0 && ACTIVE_TARGET_ROLE_VALUES.has(targetRole)
    ? runStallScan(targetRole, wpId)
    : { status: "UNKNOWN", summary: "stall scan not applicable" };
  const decision = deriveRelayWatchdogDecision({
    relayStatus: base.relayStatus,
    activeRuns,
    stallScanStatus: stallScan.status,
    allowWatchSteer,
  });
  const summary = buildRelayWatchdogSummary({
    wpId,
    relayStatus: base.relayStatus,
    decision,
    activeRuns,
    stallScanStatus: stallScan.status,
  });

  if (!decision.shouldSteer) {
    return {
      wpId,
      action: decision.action,
      reason: decision.reason,
      summary,
      relayStatus: base.relayStatus,
      stallScan,
      activeRuns,
      skipped: true,
    };
  }

  const outputLines = steerWp(wpId);
  appendWpReceipt({
    wpId,
    actorRole: "ORCHESTRATOR",
    actorSession: WATCHDOG_SESSION,
    receiptKind: "STEERING",
    summary,
    stateBefore: `${base.relayStatus.status}/${base.relayStatus.reason_code}`,
    stateAfter: "RELAY_WATCHDOG_AUTO_STEER",
  }, { autoRelay: false });

  return {
    wpId,
    action: decision.action,
    reason: decision.reason,
    summary,
    relayStatus: base.relayStatus,
    stallScan,
    activeRuns,
    outputLines,
    skipped: false,
  };
}

function printResult(result) {
  console.log(`RELAY_WATCHDOG_RESULT ${result.wpId}`);
  console.log(`- action: ${result.action}`);
  console.log(`- reason: ${result.reason || "<none>"}`);
  console.log(`- summary: ${result.summary}`);
  if (result.relayStatus?.applicable) {
    console.log(`- relay_status: ${result.relayStatus.status}`);
    console.log(`- relay_reason_code: ${result.relayStatus.reason_code}`);
    console.log(`- target: ${result.relayStatus.target_role || "<none>"}${result.relayStatus.target_session ? `:${result.relayStatus.target_session}` : ""}`);
  }
  if (result.stallScan) {
    console.log(`- stall_scan_status: ${result.stallScan.status}`);
    console.log(`- stall_scan_summary: ${result.stallScan.summary}`);
  }
  if (Array.isArray(result.activeRuns) && result.activeRuns.length > 0) {
    console.log(`- active_runs: ${result.activeRuns.length}`);
    for (const run of result.activeRuns) {
      console.log(`  - ${run.role}:${run.session_key || "<unknown>"} command=${run.command_kind || "<unknown>"} started_at=${run.started_at || "<unknown>"}`);
    }
  }
  if (Array.isArray(result.outputLines) && result.outputLines.length > 0) {
    console.log(`- steer_output: ${result.outputLines.join(" | ")}`);
  }
}

function runCycle(args) {
  const { registry } = loadSessionRegistry(REPO_ROOT);
  const brokerState = loadBrokerState();
  const wpIds = args.wpId ? [args.wpId] : listManagedPacketIds();
  const results = wpIds.map((wpId) => evaluateWp(wpId, {
    registrySessions: registry.sessions || [],
    brokerState,
    allowWatchSteer: args.allowWatchSteer,
  }));
  for (const result of results) {
    printResult(result);
  }
  return results;
}

function runCli() {
  const args = parseArgs();
  do {
    runCycle(args);
    if (!args.loop) break;
    sleep(args.intervalSeconds * 1000);
  } while (true);
}

try {
  runCli();
} catch (error) {
  console.error(`[RELAY_WATCHDOG] ${error instanceof Error ? error.message : String(error)}`);
  process.exit(1);
}
