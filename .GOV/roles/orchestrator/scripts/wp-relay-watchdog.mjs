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
  communicationTransactionLockPathForWp,
  parseJsonFile,
  parseJsonlFile,
  validateRuntimeStatus,
} from "../../../roles_shared/scripts/lib/wp-communications-lib.mjs";
import { evaluateWpCommunicationHealth } from "../../../roles_shared/scripts/lib/wp-communication-health-lib.mjs";
import { evaluateWpRelayEscalation } from "../../../roles_shared/scripts/lib/wp-relay-escalation-lib.mjs";
import { checkAllNotifications } from "../../../roles_shared/scripts/wp/wp-check-notifications.mjs";
import { appendWpNotification } from "../../../roles_shared/scripts/wp/wp-notification-append.mjs";
import { appendWpReceipt } from "../../../roles_shared/scripts/wp/wp-receipt-append.mjs";
import { loadSessionRegistry, withFileLockSync } from "../../../roles_shared/scripts/session/session-registry-lib.mjs";
import { SESSION_CONTROL_BROKER_STATE_FILE, sessionKey } from "../../../roles_shared/scripts/session/session-policy.mjs";
import {
  activeRunsForTarget,
  buildRelayRepairSignal,
  buildRelayWatchdogSummary,
  deriveRelayLaneVerdict,
  deriveRelayWatchdogDecision,
  deriveRelayWatchdogRestartDecision,
  relayRepairSignalAlreadyPending,
} from "./lib/wp-relay-watchdog-lib.mjs";

const ORCHESTRATOR_STEER_SCRIPT_PATH = path.resolve(REPO_ROOT, ".GOV/roles/orchestrator/scripts/orchestrator-steer-next.mjs");
const SESSION_CONTROL_CANCEL_SCRIPT_PATH = path.resolve(REPO_ROOT, ".GOV/roles/orchestrator/scripts/session-control-cancel.mjs");
const SESSION_STALL_SCAN_SCRIPT_PATH = path.resolve(REPO_ROOT, ".GOV/roles_shared/scripts/session/session-stall-scan.mjs");
const WATCHDOG_SESSION = "ORCHESTRATOR_WATCHDOG";
const ACTIVE_TARGET_ROLE_VALUES = new Set(["CODER", "WP_VALIDATOR", "INTEGRATION_VALIDATOR"]);
const RESTART_ELIGIBLE_ROLE_VALUES = new Set(["CODER", "WP_VALIDATOR", "INTEGRATION_VALIDATOR"]);
const DEFAULT_RESTART_OUTPUT_IDLE_SECONDS = 900;
const DEFAULT_ACTIVE_RUN_OUTPUT_FRESH_SECONDS = 180;

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

function parseNonNegativeInteger(value, fallback = 0) {
  const parsed = Number.parseInt(String(value ?? "").trim(), 10);
  if (!Number.isInteger(parsed) || parsed < 0) return fallback;
  return parsed;
}

function parseTimestamp(value) {
  const ms = Date.parse(String(value || "").trim());
  return Number.isNaN(ms) ? null : ms;
}

function parseSessionControlField(text, label) {
  const re = new RegExp(`^\\[SESSION_CONTROL\\]\\s+${label}=([^\\r\\n]+)$`, "mi");
  const match = String(text || "").match(re);
  return match ? match[1].trim() : "";
}

function parseArgs(argv = process.argv.slice(2)) {
  const args = {
    wpId: "",
    loop: false,
    intervalSeconds: 30,
    allowWatchSteer: true,
    allowRestart: false,
    observeOnly: false,
    json: false,
    restartOutputIdleSeconds: DEFAULT_RESTART_OUTPUT_IDLE_SECONDS,
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
    if (token === "--allow-restart") {
      args.allowRestart = true;
      continue;
    }
    if (token === "--observe-only") {
      args.observeOnly = true;
      continue;
    }
    if (token === "--json") {
      args.json = true;
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
    if (token === "--restart-output-idle-seconds") {
      const next = Number.parseInt(String(rest[index + 1] || "").trim(), 10);
      if (!Number.isInteger(next) || next < 60) {
        throw new Error("--restart-output-idle-seconds requires an integer >= 60");
      }
      args.restartOutputIdleSeconds = next;
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

function findTargetRegistrySession(registrySessions = [], {
  wpId = "",
  role = "",
  session = null,
} = {}) {
  const normalizedRole = normalizeRole(role);
  const normalizedSession = String(session || "").trim();
  const fallbackSessionKey = sessionKey(normalizedRole, wpId);
  return (Array.isArray(registrySessions) ? registrySessions : []).find((entry) => {
    if (String(entry?.wp_id || "").trim() !== String(wpId || "").trim()) return false;
    if (normalizeRole(entry?.role) !== normalizedRole) return false;
    const entrySessionKey = String(entry?.session_key || "").trim();
    if (!normalizedSession) return entrySessionKey === fallbackSessionKey;
    return entrySessionKey === normalizedSession || entrySessionKey === fallbackSessionKey;
  }) || null;
}

function resolveTargetOutputFile(targetSessionRecord = null) {
  const relativePath = String(targetSessionRecord?.last_command_output_file || "").trim();
  if (!relativePath) {
    return {
      relativePath: "",
      absPath: "",
      exists: false,
      modifiedAt: "",
      modifiedAtMs: null,
    };
  }
  const absPath = path.resolve(REPO_ROOT, relativePath);
  if (!fs.existsSync(absPath)) {
    return {
      relativePath,
      absPath,
      exists: false,
      modifiedAt: "",
      modifiedAtMs: null,
    };
  }
  const stats = fs.statSync(absPath);
  return {
    relativePath,
    absPath,
    exists: true,
    modifiedAt: stats.mtime.toISOString(),
    modifiedAtMs: stats.mtimeMs,
  };
}

function inspectActiveRunOutputFreshness(targetSessionRecord = null, {
  now = new Date(),
  freshSeconds = DEFAULT_ACTIVE_RUN_OUTPUT_FRESH_SECONDS,
} = {}) {
  const outputFile = resolveTargetOutputFile(targetSessionRecord);
  if (!outputFile.exists || outputFile.modifiedAtMs === null) {
    return {
      status: "MISSING",
      reason: "OUTPUT_FILE_MISSING",
      outputFile,
      outputIdleSeconds: null,
      freshSeconds,
    };
  }

  const nowMs = now.getTime();
  const outputIdleSeconds = Math.max(0, Math.trunc((nowMs - outputFile.modifiedAtMs) / 1000));
  return {
    status: outputIdleSeconds <= freshSeconds ? "RECENT" : "STALE",
    reason: outputIdleSeconds <= freshSeconds ? "OUTPUT_PROGRESS_RECENT" : "OUTPUT_PROGRESS_STALE",
    outputFile,
    outputIdleSeconds,
    freshSeconds,
  };
}

function inspectRestartFreshness({
  wpId,
  targetRole,
  targetSession,
  targetSessionRecord,
  activeRuns = [],
  restartOutputIdleSeconds = DEFAULT_RESTART_OUTPUT_IDLE_SECONDS,
  now = new Date(),
} = {}) {
  const nowMs = now.getTime();
  const normalizedRole = normalizeRole(targetRole);
  const normalizedSession = String(targetSession || "").trim();
  if (!targetSessionRecord) {
    return {
      eligible: false,
      reason: "TARGET_SESSION_NOT_FOUND",
      targetRole: normalizedRole,
      targetSession: normalizedSession,
    };
  }
  if (!RESTART_ELIGIBLE_ROLE_VALUES.has(normalizeRole(targetSessionRecord.role || normalizedRole))) {
    return {
      eligible: false,
      reason: "ROLE_NOT_RESTART_ELIGIBLE",
      targetRole: normalizedRole,
      targetSession: normalizedSession || String(targetSessionRecord.session_key || "").trim(),
    };
  }
  if (String(targetSessionRecord.runtime_state || "").trim().toUpperCase() !== "COMMAND_RUNNING") {
    return {
      eligible: false,
      reason: `SESSION_RUNTIME_${String(targetSessionRecord.runtime_state || "UNKNOWN").trim().toUpperCase() || "UNKNOWN"}`,
      targetRole: normalizedRole,
      targetSession: normalizedSession || String(targetSessionRecord.session_key || "").trim(),
    };
  }

  const outputFile = resolveTargetOutputFile(targetSessionRecord);
  if (!outputFile.exists || outputFile.modifiedAtMs === null) {
    return {
      eligible: false,
      reason: "OUTPUT_FILE_MISSING",
      targetRole: normalizedRole,
      targetSession: normalizedSession || String(targetSessionRecord.session_key || "").trim(),
      outputFile,
    };
  }

  const outputIdleSeconds = Math.max(0, Math.trunc((nowMs - outputFile.modifiedAtMs) / 1000));
  if (outputIdleSeconds < restartOutputIdleSeconds) {
    return {
      eligible: false,
      reason: "OUTPUT_RECENTLY_UPDATED",
      targetRole: normalizedRole,
      targetSession: normalizedSession || String(targetSessionRecord.session_key || "").trim(),
      outputFile,
      outputIdleSeconds,
      restartOutputIdleSeconds,
    };
  }

  const sessionReferenceAtMs = parseTimestamp(
    targetSessionRecord.last_event_at
      || targetSessionRecord.last_heartbeat_at
      || targetSessionRecord.last_command_prompt_at
      || targetSessionRecord.last_command_completed_at,
  );
  if (sessionReferenceAtMs === null) {
    return {
      eligible: false,
      reason: "SESSION_IDLE_TIMESTAMP_MISSING",
      targetRole: normalizedRole,
      targetSession: normalizedSession || String(targetSessionRecord.session_key || "").trim(),
      outputFile,
      outputIdleSeconds,
      restartOutputIdleSeconds,
    };
  }

  const sessionIdleSeconds = Math.max(0, Math.trunc((nowMs - sessionReferenceAtMs) / 1000));
  if (sessionIdleSeconds < restartOutputIdleSeconds) {
    return {
      eligible: false,
      reason: "SESSION_RECENTLY_ACTIVE",
      targetRole: normalizedRole,
      targetSession: normalizedSession || String(targetSessionRecord.session_key || "").trim(),
      outputFile,
      outputIdleSeconds,
      sessionIdleSeconds,
      restartOutputIdleSeconds,
    };
  }

  const matchingRuns = Array.isArray(activeRuns) ? activeRuns : [];
  if (matchingRuns.length === 0) {
    return {
      eligible: false,
      reason: "NO_ACTIVE_RUN",
      targetRole: normalizedRole,
      targetSession: normalizedSession || String(targetSessionRecord.session_key || "").trim(),
      outputFile,
      outputIdleSeconds,
      sessionIdleSeconds,
      restartOutputIdleSeconds,
    };
  }

  const nonExpiredRuns = matchingRuns.filter((run) => {
    const timeoutAtMs = parseTimestamp(run?.timeout_at);
    return timeoutAtMs === null || timeoutAtMs > nowMs;
  });
  if (nonExpiredRuns.length > 0) {
    return {
      eligible: false,
      reason: "ACTIVE_RUN_NOT_EXPIRED",
      targetRole: normalizedRole,
      targetSession: normalizedSession || String(targetSessionRecord.session_key || "").trim(),
      outputFile,
      outputIdleSeconds,
      sessionIdleSeconds,
      restartOutputIdleSeconds,
      nextTimeoutAt: String(nonExpiredRuns[0]?.timeout_at || "").trim() || "",
    };
  }

  return {
    eligible: true,
    reason: "STALE_ACTIVE_RUN_CONFIRMED",
    wpId: String(wpId || "").trim(),
    targetRole: normalizedRole,
    targetSession: normalizedSession || String(targetSessionRecord.session_key || "").trim(),
    outputFile,
    outputIdleSeconds,
    sessionIdleSeconds,
    restartOutputIdleSeconds,
    lastOutputAt: outputFile.modifiedAt,
    lastSessionEventAt: targetSessionRecord.last_event_at || targetSessionRecord.last_heartbeat_at || "",
  };
}

function cancelTargetSession(role, wpId) {
  const result = spawnSync(process.execPath, [SESSION_CONTROL_CANCEL_SCRIPT_PATH, role, wpId], {
    cwd: REPO_ROOT,
    encoding: "utf8",
    windowsHide: true,
  });
  const stdout = String(result.stdout || "").trim();
  const stderr = String(result.stderr || "").trim();
  const combined = [stdout, stderr].filter(Boolean).join("\n");
  const cancelStatus = parseSessionControlField(combined, "cancel_status");
  const settledStatus = parseSessionControlField(combined, "settled_status");
  const targetSettledStatus = parseSessionControlField(combined, "target_settled_status");
  const commandId = parseSessionControlField(combined, "command_id");
  const acceptedStatuses = new Set(["cancellation_requested", "target_already_settled", "not_running"]);
  return {
    ok: result.status === 0 && acceptedStatuses.has(String(cancelStatus || "").trim().toLowerCase()),
    statusCode: result.status,
    stdout,
    stderr,
    cancelStatus,
    settledStatus,
    targetSettledStatus,
    commandId,
  };
}

function maybeRestartStalledLane({
  wpId,
  registrySessions = [],
  targetRole = "",
  targetSession = null,
  decision = null,
  activeRuns = [],
  allowRestart = false,
  executeRestart = true,
  restartOutputIdleSeconds = DEFAULT_RESTART_OUTPUT_IDLE_SECONDS,
} = {}) {
  const targetSessionRecord = findTargetRegistrySession(registrySessions, {
    wpId,
    role: targetRole,
    session: targetSession,
  });
  const freshness = inspectRestartFreshness({
    wpId,
    targetRole,
    targetSession,
    targetSessionRecord,
    activeRuns,
    restartOutputIdleSeconds,
  });
  const restartDecision = deriveRelayWatchdogRestartDecision({
    decision,
    allowRestart,
    freshness,
  });
  if (!restartDecision.shouldRestart) {
    return {
      status: "SKIPPED",
      reason: restartDecision.reason,
      freshness,
      restartDecision,
      targetSessionRecord,
    };
  }

  if (!executeRestart) {
    return {
      status: "WOULD_RESTART",
      reason: restartDecision.reason,
      freshness,
      restartDecision,
      targetSessionRecord,
    };
  }

  const cancel = cancelTargetSession(targetRole, wpId);
  if (!cancel.ok) {
    return {
      status: "FAILED",
      reason: cancel.cancelStatus || cancel.settledStatus || "CANCEL_SESSION_FAILED",
      freshness,
      restartDecision,
      targetSessionRecord,
      cancel,
    };
  }

  const outputLines = steerWp(wpId);
  const correlationId = [
    "relay-watchdog-restart",
    String(wpId || "").trim() || "WP-UNKNOWN",
    normalizeRole(targetRole) || "UNKNOWN",
    String(targetSession || targetSessionRecord?.session_key || "NONE").trim() || "NONE",
    cancel.commandId || "NO_COMMAND_ID",
  ].join(":");
  appendWpReceipt({
    wpId,
    actorRole: "ORCHESTRATOR",
    actorSession: WATCHDOG_SESSION,
    receiptKind: "STEERING",
    summary: `RELAY_WATCHDOG restart repair: canceled stale active run for ${normalizeRole(targetRole)}${targetSession ? `:${targetSession}` : ""} and re-steered the governed lane.`,
    stateBefore: "REPORT_STALLED_ACTIVE_RUN",
    stateAfter: "RELAY_WATCHDOG_RESTARTED",
    targetRole: normalizeRole(targetRole),
    targetSession: String(targetSession || targetSessionRecord?.session_key || "").trim() || null,
    correlationId,
    refs: [
      freshness.outputFile?.relativePath || "",
      targetSessionRecord?.last_command_output_file || "",
    ].filter(Boolean),
  }, { autoRelay: false });

  return {
    status: "RESTARTED",
    reason: restartDecision.reason,
    freshness,
    restartDecision,
    targetSessionRecord,
    cancel,
    outputLines,
    correlationId,
  };
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
    runtimeStatusFile,
    runtimeStatusAbsPath: repoPathAbs(runtimeStatusFile),
    runtimeStatus,
    pendingNotifications,
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

function shouldPersistRelayWatchdogState(decision = null, restartRepair = null) {
  if (restartRepair?.status === "RESTARTED" && restartRepair?.restartDecision?.shouldRestart) {
    return true;
  }
  return ["INCREMENT", "RESET"].includes(String(decision?.cycleAction || "").trim().toUpperCase())
    || ["ESCALATE_RELAY_LIMIT", "REPORT_STALLED_ACTIVE_RUN"].includes(String(decision?.action || "").trim().toUpperCase());
}

function updateRelayWatchdogRuntimeState({
  wpId,
  runtimeStatusFile = "",
  runtimeStatusAbsPath = "",
  decision = null,
  restartRepair = null,
} = {}) {
  if (!runtimeStatusFile || !runtimeStatusAbsPath || !fs.existsSync(runtimeStatusAbsPath) || !shouldPersistRelayWatchdogState(decision, restartRepair)) {
    return null;
  }

  const lockPath = repoPathAbs(communicationTransactionLockPathForWp(wpId));
  return withFileLockSync(lockPath, () => {
    const runtime = parseJsonFile(runtimeStatusFile);
    const previousCycle = parseNonNegativeInteger(runtime.current_relay_escalation_cycle, 0);
    const maxCycle = Math.max(1, parseNonNegativeInteger(runtime.max_relay_escalation_cycles, 1));
    let nextCycle = previousCycle;
    const restartApplied = restartRepair?.status === "RESTARTED" && restartRepair?.restartDecision?.shouldRestart;
    const cycleAction = restartApplied
      ? "INCREMENT"
      : String(decision?.cycleAction || "").trim().toUpperCase();
    if (cycleAction === "RESET") {
      nextCycle = 0;
    } else if (cycleAction === "INCREMENT") {
      const requestedCycle = restartApplied
        ? parseNonNegativeInteger(restartRepair?.restartDecision?.nextCycle, previousCycle + 1)
        : parseNonNegativeInteger(decision?.nextCycle, previousCycle + 1);
      nextCycle = Math.min(maxCycle, Math.max(previousCycle + 1, requestedCycle));
    }

    runtime.current_relay_escalation_cycle = nextCycle;
    const lastEventAction = restartApplied
      ? "cancel_and_resteer"
      : String(decision?.action || "skip").trim().toLowerCase();
    runtime.last_event = `relay_watchdog_${lastEventAction}`;
    runtime.last_event_at = new Date().toISOString();
    if (restartApplied) {
      runtime.attention_required = false;
    } else if (["ESCALATE_RELAY_LIMIT", "REPORT_STALLED_ACTIVE_RUN"].includes(String(decision?.action || "").trim().toUpperCase())) {
      runtime.attention_required = true;
    }

    const runtimeErrors = validateRuntimeStatus(runtime);
    if (runtimeErrors.length > 0) {
      throw new Error(`Runtime status validation failed after relay watchdog update: ${runtimeErrors.join("; ")}`);
    }
    fs.writeFileSync(runtimeStatusAbsPath, `${JSON.stringify(runtime, null, 2)}\n`, "utf8");
    return {
      previousCycle,
      nextCycle,
      maxCycle,
      attentionRequired: runtime.attention_required === true,
      lastEvent: runtime.last_event,
      lastEventAt: runtime.last_event_at,
    };
  });
}

function maybeEmitRelayRepairSignal({
  wpId,
  base = null,
  decision = null,
  stallScan = null,
} = {}) {
  const repairSignal = buildRelayRepairSignal({
    wpId,
    relayStatus: base?.relayStatus,
    decision,
    stallScanStatus: stallScan?.status,
  });
  if (!repairSignal) {
    return { status: "NOT_APPLICABLE", reason: "NO_REPAIR_SIGNAL" };
  }
  if (relayRepairSignalAlreadyPending(base?.pendingNotifications, repairSignal)) {
    return {
      status: "ALREADY_PENDING",
      reason: "DUPLICATE_CORRELATION",
      correlationId: repairSignal.correlationId,
      summary: repairSignal.summary,
    };
  }
  const notification = appendWpNotification({
    wpId,
    sourceKind: repairSignal.sourceKind,
    sourceRole: "ORCHESTRATOR",
    sourceSession: WATCHDOG_SESSION,
    targetRole: repairSignal.targetRole,
    targetSession: repairSignal.targetSession,
    correlationId: repairSignal.correlationId,
    summary: repairSignal.summary,
  }, { autoRelay: false });
  return {
    status: notification ? "EMITTED" : "SKIPPED",
    reason: notification ? "REPAIR_SIGNAL_APPENDED" : "NOTIFICATION_APPEND_SKIPPED",
    correlationId: repairSignal.correlationId,
    summary: repairSignal.summary,
  };
}

function evaluateWp(wpId, {
  registrySessions,
  brokerState,
  allowWatchSteer,
  allowRestart,
  observeOnly,
  restartOutputIdleSeconds,
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
  const targetSessionRecord = activeRuns.length > 0
    ? findTargetRegistrySession(registrySessions, { wpId, role: targetRole, session: targetSession })
    : null;
  const stallScan = activeRuns.length > 0 && ACTIVE_TARGET_ROLE_VALUES.has(targetRole)
    ? runStallScan(targetRole, wpId)
    : { status: "UNKNOWN", summary: "stall scan not applicable" };
  const outputFreshness = activeRuns.length > 0
    ? inspectActiveRunOutputFreshness(targetSessionRecord)
    : { status: "UNKNOWN", reason: "NO_ACTIVE_RUN", outputFile: null, outputIdleSeconds: null };
  const decision = deriveRelayWatchdogDecision({
    relayStatus: base.relayStatus,
    activeRuns,
    stallScanStatus: stallScan.status,
    outputFreshnessStatus: outputFreshness.status,
    allowWatchSteer,
  });
  const laneVerdict = deriveRelayLaneVerdict({
    relayStatus: base.relayStatus,
    decision,
    activeRuns,
    stallScanStatus: stallScan.status,
    stallScanSummary: stallScan.summary,
    outputFreshnessStatus: outputFreshness.status,
    waitingOn: base.runtimeStatus?.waiting_on || "",
  });
  const restartRepair = maybeRestartStalledLane({
    wpId,
    registrySessions,
    targetRole,
    targetSession,
    decision,
    activeRuns,
    allowRestart,
    executeRestart: !observeOnly,
    restartOutputIdleSeconds,
  });
  const summary = buildRelayWatchdogSummary({
    wpId,
    relayStatus: base.relayStatus,
    decision,
    laneVerdict,
    activeRuns,
    stallScanStatus: stallScan.status,
    outputFreshnessStatus: outputFreshness.status,
  });

  if (observeOnly) {
    const previewAction = restartRepair?.status === "WOULD_RESTART"
      ? restartRepair.restartDecision.action
      : decision.action;
    const previewReason = restartRepair?.status === "WOULD_RESTART"
      ? restartRepair.reason
      : decision.reason;
    return {
      wpId,
      action: previewAction,
      reason: previewReason,
      summary,
      relayStatus: base.relayStatus,
      stallScan,
      outputFreshness,
      activeRuns,
      decision,
      laneVerdict,
      restartRepair,
      runtimeUpdate: null,
      repairSignal: { status: "NOT_APPLICABLE", reason: "OBSERVE_ONLY" },
      observeOnly: true,
      skipped: false,
    };
  }

  if (restartRepair?.status === "RESTARTED") {
    const runtimeUpdate = updateRelayWatchdogRuntimeState({
      wpId,
      runtimeStatusFile: base.runtimeStatusFile,
      runtimeStatusAbsPath: base.runtimeStatusAbsPath,
      decision,
      restartRepair,
    });
    return {
      wpId,
      action: restartRepair.restartDecision.action,
      reason: restartRepair.reason,
      summary,
      relayStatus: base.relayStatus,
      stallScan,
      outputFreshness,
      activeRuns,
      decision,
      laneVerdict,
      restartRepair,
      runtimeUpdate,
      outputLines: restartRepair.outputLines,
      repairSignal: { status: "NOT_APPLICABLE", reason: "RESTARTED" },
      skipped: false,
    };
  }

  if (!decision.shouldSteer) {
    const runtimeUpdate = updateRelayWatchdogRuntimeState({
      wpId,
      runtimeStatusFile: base.runtimeStatusFile,
      runtimeStatusAbsPath: base.runtimeStatusAbsPath,
      decision,
      restartRepair,
    });
    const repairSignal = maybeEmitRelayRepairSignal({
      wpId,
      base,
      decision,
      stallScan,
    });
    return {
      wpId,
      action: decision.action,
      reason: decision.reason,
      summary,
      relayStatus: base.relayStatus,
      stallScan,
      outputFreshness,
      activeRuns,
      decision,
      laneVerdict,
      runtimeUpdate,
      repairSignal,
      skipped: true,
    };
  }

  const outputLines = steerWp(wpId);
  const runtimeUpdate = updateRelayWatchdogRuntimeState({
    wpId,
    runtimeStatusFile: base.runtimeStatusFile,
    runtimeStatusAbsPath: base.runtimeStatusAbsPath,
    decision,
  });
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
    outputFreshness,
    activeRuns,
    outputLines,
    decision,
    laneVerdict,
    restartRepair,
    runtimeUpdate,
    repairSignal: { status: "NOT_APPLICABLE", reason: "STEER_ACTION" },
    skipped: false,
  };
}

function serializeResult(result) {
  return {
    kind: "RELAY_WATCHDOG_RESULT",
    wpId: result.wpId,
    action: result.action,
    reason: result.reason || "",
    observeOnly: result.observeOnly === true,
    summary: result.summary,
    laneVerdict: result.laneVerdict || null,
    relayStatus: result.relayStatus || null,
    decision: result.decision || null,
    stallScan: result.stallScan || null,
    outputFreshness: result.outputFreshness || null,
    runtimeUpdate: result.runtimeUpdate || null,
    restartRepair: result.restartRepair || null,
    repairSignal: result.repairSignal || null,
    activeRunCount: Array.isArray(result.activeRuns) ? result.activeRuns.length : 0,
  };
}

function printResult(result, { json = false } = {}) {
  if (json) {
    console.log(JSON.stringify(serializeResult(result)));
    return;
  }
  console.log(`RELAY_WATCHDOG_RESULT ${result.wpId}`);
  console.log(`- action: ${result.action}`);
  console.log(`- reason: ${result.reason || "<none>"}`);
  if (result.observeOnly) {
    console.log(`- observe_only: YES`);
  }
  console.log(`- summary: ${result.summary}`);
  if (result.laneVerdict) {
    console.log(`- lane_verdict: ${result.laneVerdict.verdict}`);
    console.log(`- lane_verdict_reason: ${result.laneVerdict.reasonCode}`);
    console.log(`- lane_poke_target: ${result.laneVerdict.pokeTarget}`);
    console.log(`- lane_worker_interrupt_policy: ${result.laneVerdict.workerInterruptPolicy}`);
  }
  if (result.relayStatus?.applicable) {
    console.log(`- relay_status: ${result.relayStatus.status}`);
    console.log(`- relay_reason_code: ${result.relayStatus.reason_code}`);
    console.log(`- target: ${result.relayStatus.target_role || "<none>"}${result.relayStatus.target_session ? `:${result.relayStatus.target_session}` : ""}`);
  }
  if (result.decision) {
    console.log(`- relay_cycle: ${result.decision.currentCycle}/${result.decision.maxCycle}`);
    if (result.decision.nextCycle !== result.decision.currentCycle) {
      console.log(`- relay_cycle_after: ${result.decision.nextCycle}/${result.decision.maxCycle}`);
    }
    console.log(`- relay_limit_reached: ${result.decision.limitReached ? "YES" : "NO"}`);
  }
  if (result.stallScan) {
    console.log(`- stall_scan_status: ${result.stallScan.status}`);
    console.log(`- stall_scan_summary: ${result.stallScan.summary}`);
  }
  if (result.outputFreshness) {
    console.log(`- output_freshness_status: ${result.outputFreshness.status}`);
    console.log(`- output_freshness_reason: ${result.outputFreshness.reason}`);
    if (Number.isInteger(result.outputFreshness.outputIdleSeconds)) {
      console.log(`- output_idle_seconds: ${result.outputFreshness.outputIdleSeconds}`);
    }
  }
  if (result.runtimeUpdate) {
    console.log(`- runtime_cycle_before: ${result.runtimeUpdate.previousCycle}/${result.runtimeUpdate.maxCycle}`);
    console.log(`- runtime_cycle_after: ${result.runtimeUpdate.nextCycle}/${result.runtimeUpdate.maxCycle}`);
    console.log(`- runtime_attention_required: ${result.runtimeUpdate.attentionRequired ? "YES" : "NO"}`);
  }
  if (result.restartRepair) {
    console.log(`- restart_status: ${result.restartRepair.status}`);
    console.log(`- restart_reason: ${result.restartRepair.reason}`);
    if (result.restartRepair.freshness?.reason) console.log(`- restart_freshness_reason: ${result.restartRepair.freshness.reason}`);
    if (Number.isInteger(result.restartRepair.freshness?.outputIdleSeconds)) {
      console.log(`- restart_output_idle_seconds: ${result.restartRepair.freshness.outputIdleSeconds}`);
    }
    if (Number.isInteger(result.restartRepair.freshness?.sessionIdleSeconds)) {
      console.log(`- restart_session_idle_seconds: ${result.restartRepair.freshness.sessionIdleSeconds}`);
    }
    if (result.restartRepair.freshness?.nextTimeoutAt) {
      console.log(`- restart_next_timeout_at: ${result.restartRepair.freshness.nextTimeoutAt}`);
    }
    if (result.restartRepair.cancel?.cancelStatus) {
      console.log(`- restart_cancel_status: ${result.restartRepair.cancel.cancelStatus}`);
    }
  }
  if (result.repairSignal && result.repairSignal.status !== "NOT_APPLICABLE") {
    console.log(`- repair_signal_status: ${result.repairSignal.status}`);
    console.log(`- repair_signal_reason: ${result.repairSignal.reason}`);
    if (result.repairSignal.correlationId) console.log(`- repair_signal_correlation: ${result.repairSignal.correlationId}`);
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
    allowRestart: args.allowRestart,
    observeOnly: args.observeOnly,
    restartOutputIdleSeconds: args.restartOutputIdleSeconds,
  }));
  for (const result of results) {
    printResult(result, { json: args.json });
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
