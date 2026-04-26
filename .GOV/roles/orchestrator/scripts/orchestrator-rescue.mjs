#!/usr/bin/env node

import fs from "node:fs";
import os from "node:os";
import { spawn, spawnSync } from "node:child_process";
import {
  formatProtectedWorktreeResolutionDiagnostics,
  resolveProtectedWorktree,
} from "../../../roles_shared/scripts/topology/git-topology-lib.mjs";
import { captureFailure, registerFailCaptureHook } from "../../../roles_shared/scripts/lib/fail-capture-lib.mjs";
import {
  buildOrchestratorRescuePrompt,
  buildRescuePowershellScript,
  buildVisibleLaunchPlan,
  rescueScriptPath,
} from "./orchestrator-rescue-lib.mjs";

registerFailCaptureHook("orchestrator-rescue.mjs", { role: "ORCHESTRATOR" });

const wpId = String(process.argv[2] || "").trim();
const flags = new Set(process.argv.slice(3));
const dryRun = flags.has("--dry-run");
const printPrompt = flags.has("--print-prompt");

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

const prompt = buildOrchestratorRescuePrompt({ wpId });
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

if (printPrompt) {
  console.log("");
  console.log("RESCUE_PROMPT");
  console.log(prompt);
}

if (dryRun) {
  console.log("- dry_run: YES");
  process.exit(0);
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
