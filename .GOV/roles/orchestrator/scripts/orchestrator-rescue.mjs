#!/usr/bin/env node

import fs from "node:fs";
import os from "node:os";
import { spawn, spawnSync } from "node:child_process";
import {
  REPO_ROOT,
  repoPathAbs,
  resolveWorkPacketPath,
} from "../../../roles_shared/scripts/lib/runtime-paths.mjs";
import {
  parseJsonFile,
  parseJsonlFile,
} from "../../../roles_shared/scripts/lib/wp-communications-lib.mjs";
import { checkAllNotifications } from "../../../roles_shared/scripts/wp/wp-check-notifications.mjs";
import { appendWpNotification } from "../../../roles_shared/scripts/wp/wp-notification-append.mjs";
import { loadSessionRegistry } from "../../../roles_shared/scripts/session/session-registry-lib.mjs";
import {
  formatProtectedWorktreeResolutionDiagnostics,
  resolveProtectedWorktree,
} from "../../../roles_shared/scripts/topology/git-topology-lib.mjs";
import { captureFailure, registerFailCaptureHook } from "../../../roles_shared/scripts/lib/fail-capture-lib.mjs";
import { evaluateOrchestratorDowntime } from "./lib/orchestrator-downtime-alert-lib.mjs";
import {
  buildTakeoverAttemptNotification,
  buildOrchestratorRescuePrompt,
  buildRescuePowershellScript,
  buildVisibleLaunchPlan,
  evaluateRescueTakeoverGuard,
  rescueScriptPath,
} from "./orchestrator-rescue-lib.mjs";

registerFailCaptureHook("orchestrator-rescue.mjs", { role: "ORCHESTRATOR" });

function parseArgs(argv = process.argv.slice(2)) {
  const args = {
    wpId: "",
    dryRun: false,
    printPrompt: false,
    forceTakeover: false,
  };
  for (const token of argv) {
    const value = String(token || "").trim();
    if (!value) continue;
    if (value === "--dry-run") {
      args.dryRun = true;
      continue;
    }
    if (value === "--print-prompt") {
      args.printPrompt = true;
      continue;
    }
    if (value === "--force-takeover") {
      args.forceTakeover = true;
      continue;
    }
    if (value.startsWith("--")) {
      throw new Error(`Unknown argument: ${value}`);
    }
    if (!args.wpId) {
      args.wpId = value;
      continue;
    }
    throw new Error(`Unexpected extra positional argument: ${value}`);
  }
  return args;
}

let args;
try {
  args = parseArgs();
} catch (error) {
  fail(error?.message || String(error || ""));
}
const { wpId, dryRun, printPrompt, forceTakeover } = args;

function fail(message, details = []) {
  const rows = Array.isArray(details) ? details : [String(details || "")];
  console.error(["ORCHESTRATOR_RESCUE_FAIL", message, ...rows].filter(Boolean).join("\n"));
  process.exit(1);
}

function commandAvailable(command) {
  if (process.platform !== "win32") return false;
  const result = spawnSync("where.exe", [command], { encoding: "utf8", stdio: ["ignore", "pipe", "ignore"] });
  return result.status === 0;
}

function parseSingleField(text, label) {
  const re = new RegExp(`^\\s*-\\s*(?:\\*\\*)?${label}(?:\\*\\*)?\\s*:\\s*(.+)\\s*$`, "mi");
  const match = String(text || "").match(re);
  return match ? match[1].trim() : "";
}

function loadRescueGuardContext(targetWpId = "") {
  const normalizedWpId = String(targetWpId || "").trim();
  if (!normalizedWpId) {
    return {
      downtimeEvaluation: null,
      pendingNotifications: [],
      guardContextStatus: "NO_WP_SCOPE",
    };
  }
  const resolved = resolveWorkPacketPath(normalizedWpId);
  if (!resolved?.packetAbsPath || !fs.existsSync(resolved.packetAbsPath)) {
    return {
      downtimeEvaluation: null,
      pendingNotifications: [],
      guardContextStatus: "PACKET_MISSING",
    };
  }
  const packetText = fs.readFileSync(resolved.packetAbsPath, "utf8");
  const workflowLane = parseSingleField(packetText, "WORKFLOW_LANE");
  const runtimeStatusFile = parseSingleField(packetText, "WP_RUNTIME_STATUS_FILE");
  const receiptsFile = parseSingleField(packetText, "WP_RECEIPTS_FILE");
  const runtimeStatus = runtimeStatusFile && fs.existsSync(repoPathAbs(runtimeStatusFile))
    ? parseJsonFile(runtimeStatusFile)
    : {};
  const receipts = receiptsFile && fs.existsSync(repoPathAbs(receiptsFile))
    ? parseJsonlFile(receiptsFile)
    : [];
  const pendingNotifications = Object.values(checkAllNotifications({ wpId: normalizedWpId }))
    .flatMap((entry) => entry.notifications || []);
  const { registry } = loadSessionRegistry(REPO_ROOT);
  const registrySessions = (registry.sessions || [])
    .filter((entry) => String(entry?.wp_id || "").trim() === normalizedWpId);
  return {
    downtimeEvaluation: evaluateOrchestratorDowntime({
      wpId: normalizedWpId,
      workflowLane,
      runtimeStatus,
      receipts,
      pendingNotifications,
      registrySessions,
    }),
    pendingNotifications,
    guardContextStatus: "OK",
  };
}

function recordTakeoverAttempt({ targetWpId = "", guardDecision = null, downtimeEvaluation = null } = {}) {
  const candidate = buildTakeoverAttemptNotification({
    wpId: targetWpId,
    guardDecision,
    downtimeEvaluation,
  });
  if (!candidate) return { status: "NOT_APPLICABLE", reason: "NO_WP_SCOPE" };
  const notification = appendWpNotification({
    wpId: targetWpId,
    sourceKind: candidate.sourceKind,
    sourceRole: "SYSTEM",
    sourceSession: "ORCHESTRATOR_RESCUE",
    targetRole: candidate.targetRole,
    targetSession: candidate.targetSession,
    correlationId: candidate.correlationId,
    summary: candidate.summary,
  }, { autoRelay: false });
  return {
    status: notification ? "RECORDED" : "SKIPPED",
    reason: notification ? "TAKEOVER_ATTEMPT_NOTIFICATION_APPENDED" : "NOTIFICATION_APPEND_SKIPPED",
    correlationId: candidate.correlationId,
  };
}

function launchDetached(command, args) {
  const child = spawn(command, args, {
    detached: true,
    stdio: "ignore",
    windowsHide: false,
  });
  child.unref();
  return child.pid || 0;
}

const kernelResolution = resolveProtectedWorktree("wt-gov-kernel");
if (!kernelResolution.ok) {
  fail(
    "Cannot resolve visible Orchestrator rescue worktree.",
    formatProtectedWorktreeResolutionDiagnostics(kernelResolution),
  );
}

const guardContext = loadRescueGuardContext(wpId);
const guardDecision = evaluateRescueTakeoverGuard({
  wpId,
  forceTakeover,
  downtimeEvaluation: guardContext.downtimeEvaluation,
});
const prompt = buildOrchestratorRescuePrompt({ wpId, guardDecision });
const scriptPath = rescueScriptPath(os.tmpdir(), wpId);
const script = buildRescuePowershellScript({
  repoRoot: kernelResolution.absDir,
  wpId,
  prompt,
});
fs.writeFileSync(scriptPath, script, "utf8");

const wtAvailable = commandAvailable("wt.exe");
const powershellAvailable = commandAvailable("powershell.exe");
const launchPlan = buildVisibleLaunchPlan({
  platform: process.platform,
  wtAvailable,
  powershellAvailable,
});

console.log("ORCHESTRATOR_RESCUE");
console.log(`- wp_id: ${wpId || "<all>"}`);
console.log(`- worktree: ${kernelResolution.absDir}`);
console.log(`- rescue_script: ${scriptPath}`);
console.log(`- launch_plan: ${launchPlan.join(" -> ")}`);
console.log(`- visible_terminal_exception: YES`);
console.log(`- acp_launch_used: NO`);
console.log(`- takeover_mode: ${guardDecision.mode}`);
console.log(`- takeover_reason: ${guardDecision.reason}`);
console.log(`- guard_context: ${guardContext.guardContextStatus}`);
if (guardContext.downtimeEvaluation?.status) {
  console.log(`- downtime_status: ${guardContext.downtimeEvaluation.status}`);
  console.log(`- downtime_reason: ${guardContext.downtimeEvaluation.reason}`);
}

if (printPrompt) {
  console.log("");
  console.log("RESCUE_PROMPT");
  console.log(prompt);
}

if (dryRun) {
  console.log("- dry_run: YES");
  if (wpId) console.log("- takeover_attempt_record: DRY_RUN_SKIPPED");
  process.exit(0);
}

const takeoverAttemptRecord = recordTakeoverAttempt({
  targetWpId: wpId,
  guardDecision,
  downtimeEvaluation: guardContext.downtimeEvaluation,
});
console.log(`- takeover_attempt_record: ${takeoverAttemptRecord.status}`);
console.log(`- takeover_attempt_reason: ${takeoverAttemptRecord.reason}`);
if (takeoverAttemptRecord.correlationId) {
  console.log(`- takeover_attempt_correlation: ${takeoverAttemptRecord.correlationId}`);
}

for (const stage of launchPlan) {
  try {
    if (stage === "windows-terminal") {
      const pid = launchDetached("wt.exe", [
        "new-tab",
        "--title",
        `ORCHESTRATOR RESCUE ${wpId || "ALL"}`,
        "powershell.exe",
        "-NoExit",
        "-ExecutionPolicy",
        "Bypass",
        "-File",
        scriptPath,
      ]);
      console.log(`- launched: windows-terminal pid=${pid || "<unknown>"}`);
      process.exit(0);
    }
    if (stage === "visible-powershell") {
      const pid = launchDetached("powershell.exe", [
        "-NoExit",
        "-ExecutionPolicy",
        "Bypass",
        "-File",
        scriptPath,
      ]);
      console.log(`- launched: visible-powershell pid=${pid || "<unknown>"}`);
      process.exit(0);
    }
  } catch (error) {
    const message = `${stage} launch failed: ${error?.message || String(error || "")}`;
    captureFailure("orchestrator-rescue.mjs", message, { role: "ORCHESTRATOR" });
    console.error(`- launch_stage_failed: ${message}`);
  }
}

console.log("- launched: NO");
console.log("- fallback: run the rescue script manually in a visible terminal");
console.log(`  powershell -NoExit -ExecutionPolicy Bypass -File "${scriptPath}"`);
