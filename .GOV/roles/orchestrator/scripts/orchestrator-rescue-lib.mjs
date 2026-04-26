import path from "node:path";

export const RESCUE_MODEL = "gpt-5.5";
export const RESCUE_REASONING = "xhigh";

export function buildOrchestratorRescuePrompt({ wpId = "" } = {}) {
  const wpSuffix = wpId ? ` --wp ${wpId}` : "";
  const healthCommand = wpId ? `just orchestrator-health ${wpId}` : "just orchestrator-health";
  const nextCommand = wpId ? `just orchestrator-next ${wpId}` : "just orchestrator-next";
  return [
    "ROLE LOCK: You are the ORCHESTRATOR. Do not change roles unless explicitly reassigned.",
    "FIRST COMMAND: just orchestrator-startup",
    "AFTER STARTUP: Continue this explicit Operator rescue task; do not start refinement, packet creation, delegation, or status changes beyond the rescue checks unless lifecycle truth requires it.",
    `SESSION_OPEN: before any governed mutation, run \`just repomem open "Visible Orchestrator rescue takeover and status recovery${wpId ? ` for ${wpId}` : ""}" --role ORCHESTRATOR${wpSuffix}\`.`,
    "AUTHORITY: ../handshake_main/AGENTS.md + .GOV/codex/Handshake_Codex_v1.4.md + .GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md + startup output",
    "FOCUS: workflow authority, launch roles via ACP, mechanical governance (phase-check, closeout-repair), stall detection, and status sync. Does NOT create refinements/worktrees/MTs (Activation Manager does). Does NOT validate or approve (validators do).",
    "LANE_BOUNDARY: this role is `ORCHESTRATOR_MANAGED` only. If the operator deliberately chooses `MANUAL_RELAY`, stop and switch to the `CLASSIC_ORCHESTRATOR` startup prompt instead of continuing under this role.",
    "MECHANICAL_GOVERNANCE: run deterministic checks via direct just/node calls, never via ACP SEND_PROMPT. ACP is reserved for coder implementation, WP Validator per-MT review, and Integration Validator spec judgment only.",
    "VISIBLE_RESCUE_EXCEPTION: this session is intentionally visible and interactive for Operator takeover. Do not convert this Orchestrator rescue lane into a headless ACP role launch.",
    `RESCUE_TASK: take over the current orchestrator-managed workflow${wpId ? ` for ${wpId}` : ""}; run \`${healthCommand}\` to inspect ACP broker health, active roles, models, threads, queues, stale duration, and lifecycle; then run \`${nextCommand}\` and continue only from mechanical truth.`,
    "RESCUE_SINGLE_AUTHORITY_GUARD: if another Orchestrator is actively mutating the same lane, stop after health/status output and ask the Operator which Orchestrator owns the lane. Do not double-steer.",
    "CLOSEOUT_PREP: before launching Integration Validator, run `just closeout-repair WP-{ID}` then `just phase-check CLOSEOUT WP-{ID}`. Do NOT launch IntVal with broken mechanical truth.",
    "REMINDER: use `just orchestrator-next` to inspect or resume, `just orchestrator-steer-next` to re-wake governed lanes, and `just orchestrator-prepare-and-packet` only after signature and role-model profiles are recorded.",
    "WORKFLOW_DOSSIER: after `just orchestrator-prepare-and-packet WP-{ID}`, keep the live Workflow Dossier under `.GOV/Audits/smoketest/` current during the run.",
    "WORKTREE: operate from `wt-gov-kernel` on branch `gov_kernel`.",
    "FAIL CAPTURE: when you encounter a tool failure, wrong tool call, or discover a workaround, IMMEDIATELY run `just memory-capture procedural \"<what failed and the fix>\" --role ORCHESTRATOR`.",
  ].join("\n");
}

function psQuote(value = "") {
  return `'${String(value).replace(/'/g, "''")}'`;
}

export function buildRescuePowershellScript({
  repoRoot,
  wpId = "",
  prompt = "",
  model = RESCUE_MODEL,
  reasoning = RESCUE_REASONING,
} = {}) {
  const healthCommand = wpId ? `just orchestrator-health ${wpId}` : "just orchestrator-health";
  const promptText = String(prompt || buildOrchestratorRescuePrompt({ wpId }));
  return [
    "$ErrorActionPreference = 'Continue'",
    `$repoRoot = ${psQuote(repoRoot)}`,
    `$wpId = ${psQuote(wpId)}`,
    `Set-Location -LiteralPath $repoRoot`,
    `$Host.UI.RawUI.WindowTitle = ${psQuote(`ORCHESTRATOR RESCUE ${wpId || "ALL"}`)}`,
    `Write-Host '[ORCHESTRATOR_RESCUE] visible rescue terminal opened'`,
    `Write-Host '[ORCHESTRATOR_RESCUE] worktree=' $repoRoot`,
    `Write-Host '[ORCHESTRATOR_RESCUE] preflight=${healthCommand}'`,
    `& just orchestrator-health $wpId`,
    `$prompt = @'`,
    promptText,
    `'@`,
    `$codexArgs = @('-m', ${psQuote(model)}, '-c', ${psQuote(`model_reasoning_effort="${reasoning}"`)}, '-C', $repoRoot, $prompt)`,
    `Write-Host '[ORCHESTRATOR_RESCUE] launching codex visible takeover'`,
    `& codex @codexArgs`,
    `if ($LASTEXITCODE -ne 0) {`,
    `  Write-Host '[ORCHESTRATOR_RESCUE] codex launch failed; manual prompt follows'`,
    `  Write-Host $prompt`,
    `  Write-Host '[ORCHESTRATOR_RESCUE] manual command:'`,
    `  Write-Host ('codex -m ${model} -c ''model_reasoning_effort="${reasoning}"'' -C "' + $repoRoot + '" "<paste prompt above>"')`,
    `}`,
  ].join("\r\n");
}

export function buildVisibleLaunchPlan({ platform = process.platform, wtAvailable = false, powershellAvailable = false } = {}) {
  if (platform !== "win32") {
    return ["manual-script"];
  }
  const stages = [];
  if (wtAvailable) stages.push("windows-terminal");
  if (powershellAvailable) stages.push("visible-powershell");
  stages.push("manual-script");
  return stages;
}

export function rescueScriptPath(tempRoot, wpId = "", now = new Date()) {
  const stamp = now.toISOString()
    .replace(/[-:]/g, "")
    .replace(/\.\d{3}Z$/, "Z")
    .replace("T", "-");
  const safeWp = String(wpId || "all").replace(/[^A-Za-z0-9._-]+/g, "_");
  return path.join(tempRoot, `handshake-orchestrator-rescue-${safeWp}-${stamp}.ps1`);
}
