import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { execFileSync, spawn, spawnSync } from "node:child_process";
import {
  CLI_SESSION_TOOL,
  defaultCoderBranch,
  defaultCoderWorktreeDir,
  defaultIntegrationValidatorBranch,
  defaultIntegrationValidatorWorktreeDir,
  defaultWpValidatorBranch,
  defaultWpValidatorWorktreeDir,
  ROLE_SESSION_PRIMARY_MODEL,
  ROLE_SESSION_REASONING_CONFIG_KEY,
  ROLE_SESSION_REASONING_CONFIG_VALUE,
  ROLE_SESSION_FALLBACK_MODEL,
  roleNextCommand,
  roleStartupCommand,
  SESSION_HOST_FALLBACK,
  SESSION_HOST_PREFERENCE,
} from "./session-policy.mjs";

const role = String(process.argv[2] || "").trim().toUpperCase();
const wpId = String(process.argv[3] || "").trim();
const requestedHost = String(process.argv[4] || "").trim().toUpperCase() || "AUTO";
const requestedModel = String(process.argv[5] || "").trim().toUpperCase() || "PRIMARY";

function fail(message) {
  console.error(`[LAUNCH_CLI_SESSION] ${message}`);
  process.exit(1);
}

if (!wpId || !wpId.startsWith("WP-")) {
  fail("Usage: node .GOV/scripts/launch-cli-session.mjs <CODER|WP_VALIDATOR|INTEGRATION_VALIDATOR> <WP_ID> [AUTO|PRINT|CURRENT|WINDOWS_TERMINAL|VSCODE] [PRIMARY|FALLBACK]");
}
if (!["PRIMARY", "FALLBACK"].includes(requestedModel)) {
  fail(`Invalid model selector: ${requestedModel} (expected PRIMARY or FALLBACK)`);
}

function runGit(args) {
  return execFileSync("git", args, { encoding: "utf8", stdio: ["ignore", "pipe", "pipe"] }).trim();
}

function commandExists(command) {
  const lookup = process.platform === "win32" ? "where.exe" : "which";
  const result = spawnSync(lookup, [command], { stdio: "ignore" });
  return result.status === 0;
}

function resolveRoleConfig(roleName, workPacketId) {
  if (roleName === "CODER") {
    return {
      branch: defaultCoderBranch(workPacketId),
      worktreeDir: defaultCoderWorktreeDir(workPacketId),
      title: `CODER ${workPacketId}`,
      startupCommand: roleStartupCommand("CODER"),
      nextCommand: roleNextCommand("CODER", workPacketId),
      focus: "implementation, governance paperwork, and coder-side delegation only when the packet allows it",
    };
  }
  if (roleName === "WP_VALIDATOR") {
    return {
      branch: defaultWpValidatorBranch(workPacketId),
      worktreeDir: defaultWpValidatorWorktreeDir(workPacketId),
      title: `WPVAL ${workPacketId}`,
      startupCommand: roleStartupCommand("WP_VALIDATOR"),
      nextCommand: roleNextCommand("WP_VALIDATOR", workPacketId),
      focus: "advisory technical review, steering, and packet-scoped validation receipts",
    };
  }
  if (roleName === "INTEGRATION_VALIDATOR") {
    return {
      branch: defaultIntegrationValidatorBranch(workPacketId),
      worktreeDir: defaultIntegrationValidatorWorktreeDir(workPacketId),
      title: `INTVAL ${workPacketId}`,
      startupCommand: roleStartupCommand("INTEGRATION_VALIDATOR"),
      nextCommand: roleNextCommand("INTEGRATION_VALIDATOR", workPacketId),
      focus: "final technical verdict, merge authority, and main/origin integration decisions only after validation handoff",
    };
  }
  return null;
}

const roleConfig = resolveRoleConfig(role, wpId);
if (!roleConfig) fail(`Unknown role: ${role}`);
const selectedModel = requestedModel === "FALLBACK" ? ROLE_SESSION_FALLBACK_MODEL : ROLE_SESSION_PRIMARY_MODEL;

const repoRoot = runGit(["rev-parse", "--show-toplevel"]);
const absWorktreeDir = path.resolve(repoRoot, roleConfig.worktreeDir);

if (!fs.existsSync(absWorktreeDir)) {
  execFileSync(
    process.execPath,
    [path.join(".GOV", "scripts", "role-session-worktree-add.mjs"), role, wpId, roleConfig.branch, roleConfig.worktreeDir],
    { stdio: "inherit" },
  );
}

const prompt = [
  `ROLE LOCK: You are the ${role}.`,
  `WP_ID: ${wpId}`,
  `WORKTREE: ${roleConfig.worktreeDir}`,
  `BRANCH: ${roleConfig.branch}`,
  `FIRST COMMAND: ${roleConfig.startupCommand}`,
  `AFTER STARTUP: ${roleConfig.nextCommand}`,
  `AUTHORITY: AGENTS.md + startup output + the role protocol + .GOV/task_packets/${wpId}.md`,
  `FOCUS: ${roleConfig.focus}.`,
  `MODEL POLICY: selected ${selectedModel}; primary ${ROLE_SESSION_PRIMARY_MODEL} with ${ROLE_SESSION_REASONING_CONFIG_KEY}=${ROLE_SESSION_REASONING_CONFIG_VALUE}; fallback ${ROLE_SESSION_FALLBACK_MODEL} with the same reasoning value if primary is unavailable.`,
  `REPO POLICY: do not switch to Codex model aliases for repo-governed sessions.`,
  `REMINDER: the Orchestrator remains workflow authority; only the Integration Validator can own merge-to-main authority.`,
].join("\n");

const codexArgs = [
  "-m",
  selectedModel,
  "-c",
  `${ROLE_SESSION_REASONING_CONFIG_KEY}="${ROLE_SESSION_REASONING_CONFIG_VALUE}"`,
  "-C",
  absWorktreeDir,
  prompt,
];

function writeLaunchScript() {
  const psPath = path.join(os.tmpdir(), `handshake-${role.toLowerCase()}-${wpId}-${Date.now()}.ps1`);
  const script = [
    `$ErrorActionPreference = 'Stop'`,
    `Set-Location -LiteralPath '${absWorktreeDir.replace(/'/g, "''")}'`,
    `${CLI_SESSION_TOOL} -m ${selectedModel} -c '${ROLE_SESSION_REASONING_CONFIG_KEY}="${ROLE_SESSION_REASONING_CONFIG_VALUE}"' -C '${absWorktreeDir.replace(/'/g, "''")}' @'`,
    prompt,
    `'@`,
  ].join("\r\n");
  fs.writeFileSync(psPath, script, "utf8");
  return psPath;
}

function launchCurrent() {
  const child = spawn(CLI_SESSION_TOOL, codexArgs, {
    cwd: absWorktreeDir,
    stdio: "inherit",
    shell: false,
  });
  child.on("exit", (code) => process.exit(code ?? 0));
}

function launchWindowsTerminal() {
  const launchScript = writeLaunchScript();
  const child = spawn(
    "wt.exe",
    [
      "new-tab",
      "--title",
      roleConfig.title,
      "powershell.exe",
      "-NoLogo",
      "-NoExit",
      "-File",
      launchScript,
    ],
    {
      cwd: repoRoot,
      detached: true,
      stdio: "ignore",
    },
  );
  child.unref();
  console.log(`[LAUNCH_CLI_SESSION] launched via WINDOWS_TERMINAL (${roleConfig.title})`);
  console.log(`[LAUNCH_CLI_SESSION] worktree=${absWorktreeDir}`);
  console.log(`[LAUNCH_CLI_SESSION] selected_model=${selectedModel}`);
  console.log(`[LAUNCH_CLI_SESSION] startup=${roleConfig.startupCommand}`);
  console.log(`[LAUNCH_CLI_SESSION] next=${roleConfig.nextCommand}`);
}

function printOnly(reason, resolvedHost) {
  console.log(`[LAUNCH_CLI_SESSION] host_preference=${SESSION_HOST_PREFERENCE}`);
  console.log(`[LAUNCH_CLI_SESSION] host_fallback=${SESSION_HOST_FALLBACK}`);
  console.log(`[LAUNCH_CLI_SESSION] host_resolved=${resolvedHost}`);
  console.log(`[LAUNCH_CLI_SESSION] reason=${reason}`);
  console.log(`[LAUNCH_CLI_SESSION] worktree=${absWorktreeDir}`);
  console.log(`[LAUNCH_CLI_SESSION] branch=${roleConfig.branch}`);
  console.log(`[LAUNCH_CLI_SESSION] selected_model=${selectedModel}`);
  console.log(`[LAUNCH_CLI_SESSION] startup=${roleConfig.startupCommand}`);
  console.log(`[LAUNCH_CLI_SESSION] next=${roleConfig.nextCommand}`);
  console.log(`[LAUNCH_CLI_SESSION] command=${CLI_SESSION_TOOL} ${codexArgs.map((part) => JSON.stringify(part)).join(" ")}`);
}

if (requestedHost === "PRINT") {
  printOnly("print-only requested", "PRINT");
  process.exit(0);
}

if (requestedHost === "CURRENT") {
  launchCurrent();
  process.exit(0);
}

if (requestedHost === "WINDOWS_TERMINAL") {
  if (!commandExists("wt")) {
    printOnly("wt.exe is unavailable on this host", "PRINT");
    process.exit(0);
  }
  launchWindowsTerminal();
  process.exit(0);
}

if (requestedHost === "VSCODE") {
  printOnly("VS Code integrated-terminal automation is not implemented in this repo launcher; use the printed command inside VS Code or fall back to WINDOWS_TERMINAL", "PRINT");
  process.exit(0);
}

if (commandExists("wt")) {
  launchWindowsTerminal();
  process.exit(0);
}

printOnly("No supported auto-launch host is available; run the printed command manually", "PRINT");
