#!/usr/bin/env node
/**
 * launch-memory-manager-session.mjs — Launch a model session for intelligent memory maintenance.
 *
 * Two-phase approach:
 *   1. Runs the mechanical pre-pass (launch-memory-manager.mjs --force) for stats, extraction, decay
 *   2. Launches a model session (Claude Code or Codex) with the Memory Manager startup prompt
 *      so the model can do judgment-based work: quality assessment, contradiction resolution,
 *      stale entry analysis, RGF candidate drafting with real reasoning.
 *
 * Usage:
 *   node launch-memory-manager-session.mjs [--model <model>] [--host <HEADLESS|CURRENT|PRINT>]
 *
 * Defaults:
 *   --model: gpt-5.5
 *   --host:  HEADLESS
 */

import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { spawnSync, spawn } from "node:child_process";
import { fileURLToPath } from "node:url";
import {
  GOVERNANCE_RUNTIME_ROOT_ABS,
} from "../../../roles_shared/scripts/lib/runtime-paths.mjs";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const GOV_ROOT = path.resolve(__dirname, "..", "..", "..");
const REPO_ROOT = path.resolve(GOV_ROOT, "..");

const args = process.argv.slice(2);
function flagValue(name, fallback) {
  const idx = args.indexOf(`--${name}`);
  return idx >= 0 && idx + 1 < args.length ? args[idx + 1] : fallback;
}

const selectedModel = flagValue("model", "gpt-5.5");
const hostMode = flagValue("host", "HEADLESS").toUpperCase();

const REPORT_PATH = path.join(GOVERNANCE_RUNTIME_ROOT_ABS, "roles_shared", "MEMORY_HYGIENE_REPORT.md");

// ---------------------------------------------------------------------------
// Phase 1: Mechanical pre-pass
// ---------------------------------------------------------------------------

console.log("[MEMORY_MANAGER_SESSION] Phase 1: Running mechanical pre-pass...");
const mechanicalScript = path.join(__dirname, "launch-memory-manager.mjs");
const prePass = spawnSync(process.execPath, [mechanicalScript, "--force"], {
  stdio: "inherit",
  cwd: REPO_ROOT,
  timeout: 120000,
});

if (prePass.status !== 0) {
  console.error("[MEMORY_MANAGER_SESSION] Mechanical pre-pass failed. Launching session anyway — model can diagnose.");
}

// Read the report if it exists
let reportSummary = "";
try {
  if (fs.existsSync(REPORT_PATH)) {
    reportSummary = fs.readFileSync(REPORT_PATH, "utf8").slice(0, 3000);
  }
} catch {}

// ---------------------------------------------------------------------------
// Phase 2: Build startup prompt and launch model session
// ---------------------------------------------------------------------------

const prompt = [
  `ROLE LOCK: You are the MEMORY MANAGER. Do not change roles unless explicitly reassigned.`,
  `AUTHORITY: .GOV/roles/memory_manager/MEMORY_MANAGER_PROTOCOL.md + .GOV/roles/memory_manager/docs/MEMORY_HYGIENE_RUBRIC.md + .GOV/roles_shared/docs/COMMAND_SURFACE_REFERENCE.md`,
  `WORKTREE: wt-gov-kernel on branch gov_kernel.`,
  ``,
  `The mechanical pre-pass has already run. The report is at: gov_runtime/roles_shared/MEMORY_HYGIENE_REPORT.md`,
  ``,
  `YOUR JOB: Intelligent maintenance that the script cannot do. Read the protocol and rubric first, then:`,
  ``,
  `1. READ the hygiene report for the mechanical pass results.`,
  `2. QUERY the memory DB directly using \`just memory-search\` to inspect entries flagged or concerning.`,
  `3. JUDGE quality: are procedural fix patterns still correct? Are semantic memories still true? Are any entries vague, misleading, or factually wrong?`,
  `4. RESOLVE contradictions with context — read both entries, decide which is correct, flag the wrong one with \`just memory-flag <id> "<reason>"\`.`,
  `5. ASSESS stale entries — if file_scope references are gone, is the knowledge still generally useful? Flag entries that are not.`,
  `6. REVIEW operator-reported and memory-capture entries — these are high-value. Verify they are still accurate and well-worded. Do NOT flag or prune them unless factually wrong.`,
  `7. DRAFT RGF candidates with real reasoning — explain WHY a pattern should be codified, what evidence supports it, and what the governance rule should say.`,
  `8. CHECK conversation log insights — are they being promoted correctly? Are there insights that should be promoted but weren't caught by the FTS similarity?`,
  `9. WRITE your findings as a structured update to MEMORY_HYGIENE_REPORT.md (append an \`## Intelligent Review\` section, do not overwrite the mechanical results).`,
  `10. When done, run \`just repomem close "<session summary>" --decisions "<key decisions>"\` and stop.`,
  ``,
  `AVAILABLE COMMANDS:`,
  `  just memory-stats`,
  `  just memory-search "<query>" [--type T] [--wp WP-{ID}]`,
  `  just memory-recall <ACTION> [--wp WP-{ID}]`,
  `  just memory-flag <id> "<reason>"`,
  `  just memory-capture <type> "<insight>" [--wp WP-{ID}] [--scope "files"]`,
  `  just memory-prime <WP-{ID}> [--budget N]`,
  `  just memory-debug-snapshot [WP-{ID}|INTENT]`,
  `  just memory-patterns [--min-wps N]`,
  `  just repomem open/close/insight/log`,
  ``,
  `CONSTRAINT: Do NOT edit protocols, codex, AGENTS.md, or product code. Memory DB and report only.`,
  `CONSTRAINT: Do NOT leave the session running after your work is done. Self-terminate.`,
  ``,
  `FIRST COMMAND: just repomem open "Memory Manager intelligent review session — reading hygiene report and performing judgment-based maintenance" --role MEMORY_MANAGER`,
].join("\n");

// Detect if this is a Claude Code model
const isClaudeCode = selectedModel.startsWith("claude-");

function buildClaudeCodeArgs() {
  return [
    "--model", selectedModel,
    "--effort", "xhigh",
    "--dangerously-skip-permissions",
    prompt,
  ];
}

function buildCodexArgs() {
  return [
    "-m", selectedModel,
    "-c", `model_reasoning_effort="xhigh"`,
    "-C", REPO_ROOT,
    prompt,
  ];
}

const cliTool = isClaudeCode ? "claude" : "codex";
const cliArgs = isClaudeCode ? buildClaudeCodeArgs() : buildCodexArgs();

function psQuote(value) {
  return `'${String(value).replace(/'/g, "''")}'`;
}

if (hostMode === "PRINT") {
  console.log(`\n[MEMORY_MANAGER_SESSION] Launch command (paste into terminal):\n`);
  console.log(`cd "${REPO_ROOT}"`);
  console.log(`${cliTool} ${cliArgs.map(a => a.includes(" ") || a.includes("\n") ? `"${a.slice(0, 80)}..."` : a).join(" ")}`);
  console.log(`\nOr paste the startup prompt from Handshake_Role_Startup_Prompts.md`);
  process.exit(0);
}

if (hostMode === "CURRENT") {
  console.log(`[MEMORY_MANAGER_SESSION] Launching ${cliTool} in current terminal...`);
  const child = spawn(cliTool, cliArgs, {
    cwd: REPO_ROOT,
    stdio: "inherit",
    shell: false,
  });
  child.on("exit", (code) => process.exit(code ?? 0));
} else {
  // HEADLESS (default). Legacy SYSTEM_TERMINAL maps here but stays hidden.
  const psPath = path.join(os.tmpdir(), `handshake-memory-manager-${Date.now()}.ps1`);
  const psArgsLines = cliArgs.map((arg) => `  ${psQuote(arg)}`).join(",\r\n");
  const script = [
    `$ErrorActionPreference = 'Stop'`,
    `Set-Location -LiteralPath ${psQuote(REPO_ROOT)}`,
    `try { [Console]::Title = 'Handshake Memory Manager' } catch {}`,
    `$cliArgs = @(`,
    psArgsLines,
    `)`,
    `& ${psQuote(cliTool)} @cliArgs`,
  ].join("\r\n");
  fs.writeFileSync(psPath, script, "utf8");

  console.log(`[MEMORY_MANAGER_SESSION] Launching ${cliTool} headless...`);
  const child = spawn("powershell.exe", ["-NoProfile", "-ExecutionPolicy", "Bypass", "-File", psPath], {
    detached: true,
    stdio: "ignore",
    windowsHide: true,
  });
  child.unref();
  console.log(`[MEMORY_MANAGER_SESSION] model=${selectedModel}`);
  console.log(`[MEMORY_MANAGER_SESSION] worktree=${REPO_ROOT}`);
  console.log(`[MEMORY_MANAGER_SESSION] report=${REPORT_PATH}`);
  console.log(`[MEMORY_MANAGER_SESSION] Session launched. It will self-terminate when done.`);
}
