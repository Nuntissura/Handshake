import crypto from "node:crypto";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { spawn, spawnSync } from "node:child_process";
import {
  CLI_SESSION_TOOL,
  ROLE_SESSION_FALLBACK_MODEL,
  ROLE_SESSION_PRIMARY_MODEL,
  ROLE_SESSION_REASONING_CONFIG_KEY,
  ROLE_SESSION_REASONING_CONFIG_VALUE,
  SESSION_COMMAND_KINDS,
  SESSION_COMMAND_STATUSES,
  SESSION_CONTROL_BROKER_AUTH_MODE,
  SESSION_CONTROL_BROKER_BUILD_ID,
  SESSION_CONTROL_OUTPUT_DIR,
  defaultCoderBranch,
  defaultCoderWorktreeDir,
  defaultIntegrationValidatorBranch,
  defaultIntegrationValidatorWorktreeDir,
  defaultWpValidatorBranch,
  defaultWpValidatorWorktreeDir,
  roleNextCommand,
  roleStartupCommand,
} from "./session-policy.mjs";

export const SESSION_CONTROL_REQUEST_SCHEMA_ID = "hsk.session_control_request@1";
export const SESSION_CONTROL_REQUEST_SCHEMA_VERSION = "session_control_request_v1";
export const SESSION_CONTROL_RESULT_SCHEMA_ID = "hsk.session_control_result@1";
export const SESSION_CONTROL_RESULT_SCHEMA_VERSION = "session_control_result_v1";

function nowIso() {
  return new Date().toISOString();
}

function writeJsonlEvent(outputStream, event) {
  outputStream.write(`${JSON.stringify({ timestamp: nowIso(), ...event })}\n`);
}

function resolveCliTool() {
  if (process.platform !== "win32") return CLI_SESSION_TOOL;
  const result = spawnSync("where.exe", [CLI_SESSION_TOOL], { encoding: "utf8", stdio: ["ignore", "pipe", "ignore"] });
  if (result.status !== 0) return `${CLI_SESSION_TOOL}.cmd`;
  const matches = result.stdout
    .split(/\r?\n/)
    .map((line) => line.trim())
    .filter(Boolean);
  return matches.find((entry) => /\.cmd$/i.test(entry)) || matches[0] || `${CLI_SESSION_TOOL}.cmd`;
}

function quotePsLiteral(value) {
  return `'${String(value ?? "").replace(/'/g, "''")}'`;
}

export function normalizePath(value) {
  return String(value || "").replace(/\\/g, "/");
}

export function sanitizeSessionKey(value) {
  return String(value || "")
    .trim()
    .replace(/[^A-Za-z0-9._-]+/g, "_");
}

function repoScopeKey(repoRoot) {
  return crypto
    .createHash("sha256")
    .update(normalizePath(path.resolve(repoRoot)))
    .digest("hex")
    .slice(0, 24);
}

export function brokerAuthTokenFile(repoRoot) {
  return path.join(os.tmpdir(), "handshake-acp-bridge", repoScopeKey(repoRoot), "auth-token.txt");
}

export function ensureBrokerAuthToken(repoRoot) {
  const filePath = brokerAuthTokenFile(repoRoot);
  if (fs.existsSync(filePath)) {
    const existing = fs.readFileSync(filePath, "utf8").trim();
    if (existing) return existing;
  }
  fs.mkdirSync(path.dirname(filePath), { recursive: true });
  const token = crypto.randomBytes(32).toString("hex");
  fs.writeFileSync(filePath, `${token}\n`, { encoding: "utf8", mode: 0o600 });
  return token;
}

export function resolveRoleConfig(roleName, workPacketId) {
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

export function selectModel(modelSelector) {
  return String(modelSelector || "").trim().toUpperCase() === "FALLBACK"
    ? ROLE_SESSION_FALLBACK_MODEL
    : ROLE_SESSION_PRIMARY_MODEL;
}

export function buildStartupPrompt({ role, wpId, roleConfig, selectedModel }) {
  return [
    `ROLE LOCK: You are the ${role}.`,
    `WP_ID: ${wpId}`,
    `WORKTREE: ${roleConfig.worktreeDir}`,
    `BRANCH: ${roleConfig.branch}`,
    `AUTHORITY: AGENTS.md + startup output + the role protocol + .GOV/task_packets/${wpId}.md`,
    `FOCUS: ${roleConfig.focus}.`,
    `MODEL POLICY: selected ${selectedModel}; primary ${ROLE_SESSION_PRIMARY_MODEL} with ${ROLE_SESSION_REASONING_CONFIG_KEY}=${ROLE_SESSION_REASONING_CONFIG_VALUE}; fallback ${ROLE_SESSION_FALLBACK_MODEL} with the same reasoning value if primary is unavailable.`,
    `REPO POLICY: do not switch to Codex model aliases for repo-governed sessions.`,
    `REMINDER: the Orchestrator remains workflow authority; only the Integration Validator can own merge-to-main authority.`,
    `Execute these commands now, in order, before any other work:`,
    `1. ${roleConfig.startupCommand}`,
    `2. ${roleConfig.nextCommand}`,
    `Do not start implementation yet. First run those commands, inspect their output, and report the resulting lifecycle/gate state.`,
  ].join("\n");
}

export function buildSessionControlRequest({
  commandId = "",
  commandKind,
  wpId,
  role,
  sessionKey,
  localBranch,
  localWorktreeDir,
  absWorktreeDir,
  selectedModel,
  prompt,
  threadId = "",
  summary = "",
  outputJsonlFile,
  targetCommandId = "",
  createdByRole = "ORCHESTRATOR",
}) {
  const COMMAND_KIND = String(commandKind || "").trim().toUpperCase();
  if (!SESSION_COMMAND_KINDS.includes(COMMAND_KIND)) {
    throw new Error(`Unknown SESSION_COMMAND kind: ${COMMAND_KIND}`);
  }
  return {
    schema_id: SESSION_CONTROL_REQUEST_SCHEMA_ID,
    schema_version: SESSION_CONTROL_REQUEST_SCHEMA_VERSION,
    command_id: commandId || crypto.randomUUID(),
    created_at: nowIso(),
    command_kind: COMMAND_KIND,
    created_by_role: createdByRole,
    session_key: sessionKey,
    wp_id: wpId,
    role,
    session_thread_id: threadId,
    local_branch: normalizePath(localBranch),
    local_worktree_dir: normalizePath(localWorktreeDir),
    abs_worktree_dir: normalizePath(absWorktreeDir),
    selected_model: selectedModel,
    reasoning_config_key: ROLE_SESSION_REASONING_CONFIG_KEY,
    reasoning_config_value: ROLE_SESSION_REASONING_CONFIG_VALUE,
    prompt,
    summary,
    output_jsonl_file: normalizePath(outputJsonlFile),
    target_command_id: targetCommandId,
  };
}

export function buildSessionControlResult({
  commandId,
  commandKind,
  sessionKey,
  wpId,
  role,
  status,
  threadId = "",
  summary = "",
  outputJsonlFile = "",
  lastAgentMessage = "",
  error = "",
  durationMs = 0,
  targetCommandId = "",
  cancelStatus = "",
  brokerBuildId = SESSION_CONTROL_BROKER_BUILD_ID,
}) {
  const STATUS = String(status || "").trim().toUpperCase();
  if (!SESSION_COMMAND_STATUSES.includes(STATUS)) {
    throw new Error(`Unknown SESSION_COMMAND status: ${STATUS}`);
  }
  return {
    schema_id: SESSION_CONTROL_RESULT_SCHEMA_ID,
    schema_version: SESSION_CONTROL_RESULT_SCHEMA_VERSION,
    command_id: commandId,
    processed_at: nowIso(),
    command_kind: commandKind,
    session_key: sessionKey,
    wp_id: wpId,
    role,
    status: STATUS,
    thread_id: threadId,
    summary,
    output_jsonl_file: normalizePath(outputJsonlFile),
    last_agent_message: lastAgentMessage,
    error,
    duration_ms: durationMs,
    target_command_id: targetCommandId,
    cancel_status: cancelStatus,
    broker_build_id: brokerBuildId,
  };
}

export function defaultSessionOutputFile(repoRoot, sessionKey, commandId) {
  const safeSessionKey = sanitizeSessionKey(sessionKey);
  return path.resolve(repoRoot, SESSION_CONTROL_OUTPUT_DIR, safeSessionKey, `${commandId}.jsonl`);
}

export async function runCodexThreadCommand({
  absWorktreeDir,
  selectedModel,
  prompt,
  outputFile,
  threadId = "",
  onEvent = null,
  onSpawn = null,
}) {
  const outputPath = path.resolve(outputFile);
  fs.mkdirSync(path.dirname(outputPath), { recursive: true });
  const outputStream = fs.createWriteStream(outputPath, { flags: "a" });
  const startedAt = Date.now();
  const cliToolPath = resolveCliTool();

  const args = threadId
    ? [
      "exec",
      "resume",
      threadId,
      "--json",
      "-c",
      'sandbox_mode="danger-full-access"',
      "-m",
      selectedModel,
      "-c",
      `${ROLE_SESSION_REASONING_CONFIG_KEY}=\"${ROLE_SESSION_REASONING_CONFIG_VALUE}\"`,
      prompt,
    ]
    : [
      "exec",
      "--json",
      "-c",
      'sandbox_mode="danger-full-access"',
      "-m",
      selectedModel,
      "-c",
      `${ROLE_SESSION_REASONING_CONFIG_KEY}=\"${ROLE_SESSION_REASONING_CONFIG_VALUE}\"`,
      "-C",
      absWorktreeDir,
      prompt,
    ];

  return await new Promise((resolve) => {
    const child = process.platform === "win32"
      ? spawn("powershell.exe", [
        "-NoLogo",
        "-NonInteractive",
        "-Command",
        [
          "$ErrorActionPreference = 'Stop'",
          `$codexArgs = @(${args.map((arg) => quotePsLiteral(arg)).join(", ")})`,
          `& ${quotePsLiteral(cliToolPath)} @codexArgs`,
        ].join("; "),
      ], {
        cwd: absWorktreeDir,
        shell: false,
        stdio: ["ignore", "pipe", "pipe"],
      })
      : spawn(cliToolPath, args, {
        cwd: absWorktreeDir,
        shell: false,
        stdio: ["ignore", "pipe", "pipe"],
      });

    if (typeof onSpawn === "function") onSpawn(child);

    let stderr = "";
    let stdoutBuffer = "";
    let observedThreadId = threadId || "";
    let lastAgentMessage = "";
    let settled = false;

    const finish = (result) => {
      if (settled) return;
      settled = true;
      outputStream.end();
      resolve(result);
    };

    const handleLine = (line) => {
      if (!line) return;
      try {
        const event = JSON.parse(line);
        const normalized = { timestamp: nowIso(), ...event };
        writeJsonlEvent(outputStream, event);
        if (typeof onEvent === "function") onEvent(normalized);
        if (normalized.type === "thread.started" && normalized.thread_id) {
          observedThreadId = normalized.thread_id;
        }
        if (normalized.type === "item.completed" && normalized.item?.type === "agent_message") {
          lastAgentMessage = normalized.item.text || lastAgentMessage;
        }
      } catch {
        const rawEvent = { type: "stdout.raw", text: line };
        writeJsonlEvent(outputStream, rawEvent);
        if (typeof onEvent === "function") onEvent({ timestamp: nowIso(), ...rawEvent });
      }
    };

    child.stdout.on("data", (chunk) => {
      stdoutBuffer += chunk.toString("utf8");
      const lines = stdoutBuffer.split(/\r?\n/);
      stdoutBuffer = lines.pop() || "";
      for (const line of lines) handleLine(line.trim());
    });

    child.stderr.on("data", (chunk) => {
      const text = chunk.toString("utf8");
      stderr += text;
      const event = { type: "stderr", text };
      writeJsonlEvent(outputStream, event);
      if (typeof onEvent === "function") onEvent({ timestamp: nowIso(), ...event });
    });

    child.on("error", (error) => {
      stderr += error.message;
      const event = { type: "spawn.error", message: error.message };
      writeJsonlEvent(outputStream, event);
      if (typeof onEvent === "function") onEvent({ timestamp: nowIso(), ...event });
      finish({
        ok: false,
        exitCode: 1,
        threadId: observedThreadId,
        lastAgentMessage,
        stderr: stderr.trim(),
        durationMs: Date.now() - startedAt,
        outputFile: outputPath,
      });
    });

    child.on("close", (code) => {
      if (stdoutBuffer.trim()) handleLine(stdoutBuffer.trim());
      const closedEvent = { type: "process.closed", exit_code: code ?? 1 };
      writeJsonlEvent(outputStream, closedEvent);
      if (typeof onEvent === "function") onEvent({ timestamp: nowIso(), ...closedEvent });
      finish({
        ok: code === 0,
        exitCode: code ?? 1,
        threadId: observedThreadId,
        lastAgentMessage,
        stderr: stderr.trim(),
        durationMs: Date.now() - startedAt,
        outputFile: outputPath,
      });
    });
  });
}

export function validateSessionControlRequestShape(request) {
  const errors = [];
  const commandKind = String(request?.command_kind || "").trim().toUpperCase();
  if (!request || typeof request !== "object") return ["request must be an object"];
  if (request.schema_id !== SESSION_CONTROL_REQUEST_SCHEMA_ID) errors.push(`schema_id must be ${SESSION_CONTROL_REQUEST_SCHEMA_ID}`);
  if (request.schema_version !== SESSION_CONTROL_REQUEST_SCHEMA_VERSION) errors.push(`schema_version must be ${SESSION_CONTROL_REQUEST_SCHEMA_VERSION}`);
  if (!SESSION_COMMAND_KINDS.includes(commandKind)) errors.push("command_kind is invalid");
  if (!request.command_id) errors.push("command_id is required");
  if (request.created_by_role !== "ORCHESTRATOR") errors.push("created_by_role must be ORCHESTRATOR");
  if (!request.session_key) errors.push("session_key is required");
  if (!request.wp_id) errors.push("wp_id is required");
  if (!request.role) errors.push("role is required");
  if (commandKind !== "CANCEL_SESSION" && !request.prompt) errors.push("prompt is required");
  if (!request.output_jsonl_file) errors.push("output_jsonl_file is required");
  if (commandKind === "CANCEL_SESSION" && !request.target_command_id) {
    errors.push("target_command_id is required for CANCEL_SESSION");
  }
  return errors;
}

export function validateSessionControlResultShape(result) {
  const errors = [];
  const commandKind = String(result?.command_kind || "").trim().toUpperCase();
  if (!result || typeof result !== "object") return ["result must be an object"];
  if (result.schema_id !== SESSION_CONTROL_RESULT_SCHEMA_ID) errors.push(`schema_id must be ${SESSION_CONTROL_RESULT_SCHEMA_ID}`);
  if (result.schema_version !== SESSION_CONTROL_RESULT_SCHEMA_VERSION) errors.push(`schema_version must be ${SESSION_CONTROL_RESULT_SCHEMA_VERSION}`);
  if (!SESSION_COMMAND_KINDS.includes(commandKind)) errors.push("command_kind is invalid");
  if (!SESSION_COMMAND_STATUSES.includes(String(result.status || "").trim().toUpperCase())) errors.push("status is invalid");
  if (!result.command_id) errors.push("command_id is required");
  if (!result.session_key) errors.push("session_key is required");
  if (!result.wp_id) errors.push("wp_id is required");
  if (!result.role) errors.push("role is required");
  if ("broker_build_id" in result && !String(result.broker_build_id || "").trim()) {
    errors.push("broker_build_id must be a non-empty string when present");
  }
  if (commandKind === "CANCEL_SESSION" && !result.target_command_id) {
    errors.push("target_command_id is required for CANCEL_SESSION");
  }
  return errors;
}
