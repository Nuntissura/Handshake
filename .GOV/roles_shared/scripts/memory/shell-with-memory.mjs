#!/usr/bin/env node
/**
 * shell-with-memory.mjs
 *
 * Command-family memory injection wrapper for ad hoc shell commands.
 *
 * Usage:
 *   node shell-with-memory.mjs <ROLE> <COMMAND_FAMILY> "<COMMAND>" [--wp WP-{ID}] [--action COMMAND]
 *     [--shell powershell|bash|cmd] [--scope "files"] [--on-fail "<insight>"] [--on-success "<insight>"]
 *
 * The wrapper:
 *   1. prints trigger-aware memory recall for the command family
 *   2. best-effort records PRE_TASK context when a repomem session is open
 *   3. runs the command in the selected shell
 *   4. optionally captures structured shell-command memory on success/failure
 */

import { spawnSync } from "node:child_process";
import path from "node:path";
import { fileURLToPath } from "node:url";

function parseFlags(args) {
  const flags = {};
  const positional = [];
  for (let i = 0; i < args.length; i += 1) {
    const token = String(args[i] || "");
    if (token.startsWith("--") && i + 1 < args.length) {
      flags[token.slice(2)] = args[i + 1];
      i += 1;
    } else {
      positional.push(token);
    }
  }
  return { flags, positional };
}

function normalizeRole(value) {
  return String(value || "").trim().toUpperCase();
}

function normalizeShell(value) {
  const normalized = String(value || "powershell").trim().toLowerCase();
  if (["powershell", "pwsh"].includes(normalized)) return "powershell";
  if (normalized === "bash") return "bash";
  if (normalized === "cmd") return "cmd";
  throw new Error(`Unsupported shell: ${value}`);
}

function usage() {
  console.error('Usage: shell-with-memory.mjs <ROLE> <COMMAND_FAMILY> "<COMMAND>" [--wp WP-{ID}] [--action COMMAND] [--shell powershell|bash|cmd] [--scope "files"] [--on-fail "<insight>"] [--on-success "<insight>"]');
  process.exit(1);
}

function repoRootFromScript() {
  return path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../../../..");
}

function isDirectExecution() {
  const entry = process.argv[1];
  if (!entry) return false;
  return path.resolve(entry) === fileURLToPath(import.meta.url);
}

export function buildShellExecutionArgs(shell, command) {
  if (shell === "bash") return { tool: "bash", args: ["-lc", command] };
  if (shell === "cmd") return { tool: "cmd.exe", args: ["/d", "/s", "/c", command] };
  return { tool: "powershell.exe", args: ["-NoLogo", "-NonInteractive", "-Command", command] };
}

export function buildShellMemoryMetadata({
  commandFamily,
  rawCommand,
  shell,
  exitCode,
  action,
}) {
  return {
    command_family: commandFamily,
    raw_command: rawCommand,
    shell,
    exit_code: exitCode,
    trigger: commandFamily,
    action,
    wrapper: "shell-with-memory",
  };
}

function runNodeScript(scriptPath, args, { stdio = "inherit" } = {}) {
  return spawnSync(process.execPath, [scriptPath, ...args], {
    cwd: repoRootFromScript(),
    stdio,
    encoding: "utf8",
  });
}

function tryRepomemContext({ role, commandFamily, command, wpId = "" }) {
  const repomemPath = path.join(path.dirname(fileURLToPath(import.meta.url)), "repomem.mjs");
  const content = `Running shell command family ${commandFamily}: ${command}`;
  const args = ["context", content, "--trigger", `shell:${commandFamily}`];
  if (wpId) args.push("--wp", wpId);
  const result = runNodeScript(repomemPath, args, { stdio: "pipe" });
  if (result.status === 0) {
    process.stdout.write(result.stdout || "");
  }
}

function captureShellMemory({
  memoryType = "procedural",
  insight,
  role,
  wpId = "",
  scope = "",
  commandFamily,
  command,
  shell,
  exitCode,
  action,
}) {
  if (!insight) return;
  const cliPath = path.join(path.dirname(fileURLToPath(import.meta.url)), "governance-memory-cli.mjs");
  const metadata = JSON.stringify(buildShellMemoryMetadata({
    commandFamily,
    rawCommand: command,
    shell,
    exitCode,
    action,
  }));
  const args = [
    "capture",
    memoryType,
    insight,
    "--topic",
    `${commandFamily}: ${String(insight).slice(0, 80)}`,
    "--source",
    "shell-command",
    "--role",
    role,
    "--importance",
    "0.7",
    "--metadata",
    metadata,
  ];
  if (wpId) {
    args.push("--wp", wpId);
  }
  if (scope) {
    args.push("--scope", scope);
  }
  const result = runNodeScript(cliPath, args);
  if (result.status !== 0) {
    console.error("[shell-with-memory] shell memory capture failed");
  }
}

function main() {
  const { flags, positional } = parseFlags(process.argv.slice(2));
  const [roleRaw, commandFamilyRaw, command] = positional;
  if (!roleRaw || !commandFamilyRaw || !command) usage();

  const role = normalizeRole(roleRaw);
  const commandFamily = String(commandFamilyRaw || "").trim();
  const wpId = String(flags.wp || "").trim();
  const action = String(flags.action || "COMMAND").trim().toUpperCase();
  const shell = normalizeShell(flags.shell || "powershell");
  const scope = String(flags.scope || "").trim();

  const recallPath = path.join(path.dirname(fileURLToPath(import.meta.url)), "memory-recall.mjs");
  const recallArgs = [action, "--role", role, "--trigger", commandFamily, "--script", commandFamily];
  if (wpId) recallArgs.push("--wp", wpId);

  const recallResult = runNodeScript(recallPath, recallArgs);
  if (recallResult.status !== 0) {
    console.error("[shell-with-memory] memory-recall failed; continuing with command execution");
  }

  tryRepomemContext({ role, commandFamily, command, wpId });

  const { tool, args } = buildShellExecutionArgs(shell, command);
  console.log(`[shell-with-memory] role=${role} family=${commandFamily} shell=${shell} action=${action}`);
  const commandResult = spawnSync(tool, args, {
    cwd: process.cwd(),
    stdio: "inherit",
    shell: false,
  });
  const exitCode = Number(commandResult.status ?? 0);

  if (exitCode === 0 && flags["on-success"]) {
    captureShellMemory({
      insight: flags["on-success"],
      role,
      wpId,
      scope,
      commandFamily,
      command,
      shell,
      exitCode,
      action,
    });
  }

  if (exitCode !== 0 && flags["on-fail"]) {
    captureShellMemory({
      insight: flags["on-fail"],
      role,
      wpId,
      scope,
      commandFamily,
      command,
      shell,
      exitCode,
      action,
    });
  }

  process.exit(exitCode);
}

if (isDirectExecution()) {
  main();
}
