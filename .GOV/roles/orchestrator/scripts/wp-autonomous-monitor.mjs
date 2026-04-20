#!/usr/bin/env node

import fs from "node:fs";
import path from "node:path";
import { execFileSync } from "node:child_process";
import { fileURLToPath } from "node:url";
import { loadSessionRegistry } from "../../../roles_shared/scripts/session/session-registry-lib.mjs";
import { parseJsonFile } from "../../../roles_shared/scripts/lib/wp-communications-lib.mjs";
import { materializeRuntimeAuthorityView } from "../../../roles_shared/scripts/lib/wp-execution-state-lib.mjs";
import { repoPathAbs, resolveWorkPacketPath } from "../../../roles_shared/scripts/lib/runtime-paths.mjs";

const REPO_ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../../../..");
const WATCHDOG_SCRIPT = path.resolve(REPO_ROOT, ".GOV/roles/orchestrator/scripts/wp-relay-watchdog.mjs");
const STEER_SCRIPT = path.resolve(REPO_ROOT, ".GOV/roles/orchestrator/scripts/orchestrator-steer-next.mjs");
const LOG_DIR = path.resolve(REPO_ROOT, "../gov_runtime/roles_shared/SESSION_MONITORS");
const ACTIVE_ROLE_VALUES = new Set(["CODER", "WP_VALIDATOR", "INTEGRATION_VALIDATOR", "ACTIVATION_MANAGER"]);

function usage() {
  console.error(
    "Usage: node .GOV/roles/orchestrator/scripts/wp-autonomous-monitor.mjs"
    + " WP-{ID} [--interval-seconds N] [--once] [--log-file PATH]"
  );
  process.exit(1);
}

function sleep(ms) {
  Atomics.wait(new Int32Array(new SharedArrayBuffer(4)), 0, 0, Math.max(1, Math.trunc(ms)));
}

function parseArgs(argv = process.argv.slice(2)) {
  const args = {
    wpId: "",
    intervalSeconds: 900,
    once: false,
    logFile: "",
  };
  const rest = [...argv];
  if (rest[0] && !String(rest[0]).startsWith("--")) {
    args.wpId = String(rest.shift() || "").trim();
  }
  for (let index = 0; index < rest.length; index += 1) {
    const token = String(rest[index] || "").trim();
    if (token === "--once") {
      args.once = true;
      continue;
    }
    if (token === "--interval-seconds") {
      const next = Number.parseInt(String(rest[index + 1] || "").trim(), 10);
      if (!Number.isInteger(next) || next < 30) {
        throw new Error("--interval-seconds requires an integer >= 30");
      }
      args.intervalSeconds = next;
      index += 1;
      continue;
    }
    if (token === "--log-file") {
      const next = String(rest[index + 1] || "").trim();
      if (!next) throw new Error("--log-file requires a path");
      args.logFile = next;
      index += 1;
      continue;
    }
    throw new Error(`Unknown argument: ${token}`);
  }
  if (!args.wpId || !/^WP-/.test(args.wpId)) usage();
  return args;
}

function ensureLogFile(logFile) {
  const resolved = logFile
    ? path.resolve(REPO_ROOT, logFile)
    : path.resolve(LOG_DIR, `${process.argv[2]}-monitor.log`);
  fs.mkdirSync(path.dirname(resolved), { recursive: true });
  return resolved;
}

function appendLog(logFile, line) {
  const stamped = `[${new Date().toISOString()}] ${line}`;
  fs.appendFileSync(logFile, `${stamped}\n`, "utf8");
  console.log(stamped);
}

function runNode(scriptPath, args = [], timeoutMs = 120000) {
  return execFileSync(process.execPath, [scriptPath, ...args], {
    cwd: REPO_ROOT,
    encoding: "utf8",
    stdio: ["ignore", "pipe", "pipe"],
    timeout: timeoutMs,
    windowsHide: true,
  });
}

function loadPacketRuntime(wpId) {
  const packetInfo = resolveWorkPacketPath(wpId);
  const packetPath = packetInfo?.packetPath || `.GOV/task_packets/${wpId}/packet.md`;
  const packetAbs = repoPathAbs(packetPath);
  if (!fs.existsSync(packetAbs)) {
    throw new Error(`Packet missing: ${packetPath}`);
  }
  const packetText = fs.readFileSync(packetAbs, "utf8");
  const runtimeMatch = packetText.match(/^\s*-\s*(?:\*\*)?WP_RUNTIME_STATUS_FILE(?:\*\*)?\s*:\s*(.+)\s*$/mi);
  if (!runtimeMatch) {
    throw new Error(`Packet missing WP_RUNTIME_STATUS_FILE: ${packetPath}`);
  }
  const runtimePath = String(runtimeMatch[1] || "").trim();
  return {
    packetPath,
    runtimePath,
    runtimeStatus: materializeRuntimeAuthorityView(parseJsonFile(runtimePath)),
  };
}

function loadRegistrySession(registrySessions = [], role = "", wpId = "") {
  const targetKey = `${String(role || "").trim().toUpperCase()}:${String(wpId || "").trim()}`;
  return (registrySessions || []).find((entry) => String(entry?.session_key || "").trim() === targetKey) || null;
}

function wpIsTerminal(runtimeStatus = {}) {
  const packetStatus = String(runtimeStatus?.current_packet_status || "").trim().toUpperCase();
  const boardStatus = String(runtimeStatus?.current_task_board_status || "").trim().toUpperCase();
  const containment = String(runtimeStatus?.main_containment_status || "").trim().toUpperCase();
  const runtimeState = String(runtimeStatus?.runtime_status || "").trim().toUpperCase();
  if (containment === "CONTAINED_IN_MAIN") return true;
  if (packetStatus.includes("VALIDATED")) return true;
  if (boardStatus.startsWith("DONE")) return true;
  return ["COMPLETED", "FAILED", "CANCELED"].includes(runtimeState);
}

function overlapValidatorTargets(runtimeStatus = {}) {
  const openItems = Array.isArray(runtimeStatus?.open_review_items) ? runtimeStatus.open_review_items : [];
  return openItems
    .filter((item) => String(item?.target_role || "").trim().toUpperCase() === "WP_VALIDATOR")
    .map((item) => ({
      correlationId: String(item?.correlation_id || "").trim(),
      targetSession: String(item?.target_session || "").trim(),
    }))
    .filter((item) => item.correlationId);
}

function tryWatchdog(wpId, logFile) {
  try {
    const output = runNode(WATCHDOG_SCRIPT, [wpId], 180000);
    const summary = String(output || "").split(/\r?\n/).map((line) => line.trim()).filter(Boolean).slice(-5).join(" | ");
    appendLog(logFile, `watchdog=${summary || "ok"}`);
  } catch (error) {
    const stderr = String(error?.stderr || "").trim();
    const stdout = String(error?.stdout || "").trim();
    appendLog(logFile, `watchdog_error=${stderr || stdout || error.message}`);
  }
}

function trySteerPrimary(wpId, logFile) {
  try {
    const output = runNode(STEER_SCRIPT, [wpId, "PRIMARY"], 180000);
    const summary = String(output || "").split(/\r?\n/).map((line) => line.trim()).filter(Boolean).slice(-6).join(" | ");
    appendLog(logFile, `primary_steer=${summary || "dispatched"}`);
  } catch (error) {
    const stderr = String(error?.stderr || "").trim();
    const stdout = String(error?.stdout || "").trim();
    appendLog(logFile, `primary_steer_error=${stderr || stdout || error.message}`);
  }
}

function trySteerValidator(wpId, targetSession, logFile) {
  try {
    const args = [wpId, "PRIMARY", "--target-role=WP_VALIDATOR"];
    if (targetSession) args.push(`--target-session=${targetSession}`);
    const output = runNode(STEER_SCRIPT, args, 180000);
    const summary = String(output || "").split(/\r?\n/).map((line) => line.trim()).filter(Boolean).slice(-6).join(" | ");
    appendLog(logFile, `validator_sidecar=${summary || "dispatched"}`);
  } catch (error) {
    const stderr = String(error?.stderr || "").trim();
    const stdout = String(error?.stdout || "").trim();
    appendLog(logFile, `validator_sidecar_error=${stderr || stdout || error.message}`);
  }
}

function monitorOnce(wpId, logFile) {
  const { registry } = loadSessionRegistry(REPO_ROOT);
  const { runtimeStatus } = loadPacketRuntime(wpId);
  const coderSession = loadRegistrySession(registry.sessions, "CODER", wpId);
  const validatorSession = loadRegistrySession(registry.sessions, "WP_VALIDATOR", wpId);
  const nextActor = String(runtimeStatus?.next_expected_actor || "").trim().toUpperCase();

  appendLog(
    logFile,
    [
      `status=${runtimeStatus?.current_packet_status || "<missing>"}`,
      `phase=${runtimeStatus?.current_phase || "<missing>"}`,
      `next=${nextActor || "<none>"}`,
      `waiting_on=${runtimeStatus?.waiting_on || "<missing>"}`,
      `coder=${coderSession?.runtime_state || "NONE"}`,
      `wp_validator=${validatorSession?.runtime_state || "NONE"}`,
    ].join(" | "),
  );

  if (wpIsTerminal(runtimeStatus)) {
    appendLog(logFile, "terminal=YES");
    return { terminal: true };
  }

  tryWatchdog(wpId, logFile);

  const refreshed = loadPacketRuntime(wpId).runtimeStatus;
  const refreshedRegistry = loadSessionRegistry(REPO_ROOT).registry;
  const refreshedCoder = loadRegistrySession(refreshedRegistry.sessions, "CODER", wpId);
  const refreshedValidator = loadRegistrySession(refreshedRegistry.sessions, "WP_VALIDATOR", wpId);

  const validatorTargets = overlapValidatorTargets(refreshed);
  if (
    validatorTargets.length > 0
    && String(refreshedValidator?.runtime_state || "").trim().toUpperCase() !== "COMMAND_RUNNING"
  ) {
    trySteerValidator(wpId, validatorTargets[0].targetSession, logFile);
  }

  const refreshedNextActor = String(refreshed?.next_expected_actor || "").trim().toUpperCase();
  const refreshedPrimarySession = refreshedNextActor === "CODER"
    ? refreshedCoder
    : refreshedNextActor === "WP_VALIDATOR"
    ? refreshedValidator
    : loadRegistrySession(refreshedRegistry.sessions, refreshedNextActor, wpId);
  if (
    ACTIVE_ROLE_VALUES.has(refreshedNextActor)
    && String(refreshedPrimarySession?.runtime_state || "").trim().toUpperCase() !== "COMMAND_RUNNING"
  ) {
    trySteerPrimary(wpId, logFile);
  }

  return { terminal: false };
}

function main() {
  const args = parseArgs();
  const logFile = ensureLogFile(args.logFile || "");
  appendLog(logFile, `monitor_start wp=${args.wpId} interval_seconds=${args.intervalSeconds}`);

  while (true) {
    const result = monitorOnce(args.wpId, logFile);
    if (result.terminal || args.once) {
      appendLog(logFile, "monitor_stop");
      break;
    }
    sleep(args.intervalSeconds * 1000);
  }
}

main();
