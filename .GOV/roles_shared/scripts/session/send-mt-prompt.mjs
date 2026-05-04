#!/usr/bin/env node
/**
 * Builds and dispatches a governed microtask prompt to the coder session.
 *
 * Auto-includes:
 * - Session keys (from registry)
 * - wp-review-request command with correct parameters
 * - "STOP and wait" instruction
 * - CARGO_TARGET_DIR hint
 *
 * Usage: node send-mt-prompt.mjs <WP_ID> <MT_ID> <description> [model]
 *
 * Example: node send-mt-prompt.mjs WP-1-FR-ModelSessionId-v1 MT-001 "Add model_session_id field to FlightRecorderEvent"
 */

import { execFileSync } from "node:child_process";
import fs from "node:fs";
import { sessionKey, defaultCoderWorktreeDir } from "./session-policy.mjs";
import { loadSessionRegistry } from "./session-registry-lib.mjs";
import { buildEphemeralContextBlock } from "./ephemeral-injection-lib.mjs";
import { REPO_ROOT, repoPathAbs, workPacketPath } from "../lib/runtime-paths.mjs";
import path from "node:path";

const wpId = String(process.argv[2] || "").trim();
const mtId = String(process.argv[3] || "").trim();
const description = String(process.argv[4] || "").trim();
const modelArgs = process.argv.slice(5);
const model = (() => {
  for (const candidate of modelArgs) {
    const value = String(candidate || "").trim().toUpperCase();
    if (!value || value.startsWith("--")) continue;
    return value;
  }
  return "PRIMARY";
})();
const debugMode = modelArgs.some((arg) => String(arg || "").trim() === "--debug");
const sessionControlEnv = {
  ...process.env,
  ...(debugMode ? { HANDSHAKE_SESSION_CONTROL_DEBUG: "1" } : {}),
};

if (!wpId || !mtId || !description) {
  console.error("Usage: node send-mt-prompt.mjs <WP_ID> <MT_ID> <description> [model] [--debug]");
  console.error("Example: node send-mt-prompt.mjs WP-1-FR-ModelSessionId-v1 MT-001 \"Add model_session_id field\"");
  process.exit(1);
}

const coderKey = sessionKey("CODER", wpId);
const validatorKey = sessionKey("WP_VALIDATOR", wpId);

function normalizeSession(value) {
  const text = String(value || "").trim();
  if (!text || /^<unassigned>$/i.test(text) || /^none$/i.test(text) || /^null$/i.test(text)) return "";
  return text;
}

function parseSingleField(text, label) {
  const re = new RegExp(`^\\s*-\\s*(?:\\*\\*)?${label}(?:\\*\\*)?\\s*:\\s*(.+)\\s*$`, "mi");
  const match = String(text || "").match(re);
  return match ? match[1].trim() : "";
}

function readRuntimeStatusForWp(targetWpId) {
  const packetPath = repoPathAbs(workPacketPath(targetWpId));
  if (!fs.existsSync(packetPath)) return {};
  const packetText = fs.readFileSync(packetPath, "utf8");
  const runtimeStatusFile = parseSingleField(packetText, "WP_RUNTIME_STATUS_FILE");
  if (!runtimeStatusFile) return {};
  const runtimePath = repoPathAbs(runtimeStatusFile);
  if (!fs.existsSync(runtimePath)) return {};
  try {
    return JSON.parse(fs.readFileSync(runtimePath, "utf8"));
  } catch {
    return {};
  }
}

function latestRoleSessionFromRuntime(runtimeStatus, role) {
  const normalizedRole = String(role || "").trim().toUpperCase();
  const activeSessions = Array.isArray(runtimeStatus?.active_role_sessions)
    ? runtimeStatus.active_role_sessions
    : [];
  const matching = activeSessions
    .filter((entry) => String(entry?.role || "").trim().toUpperCase() === normalizedRole)
    .map((entry) => normalizeSession(entry?.session_id))
    .filter(Boolean);
  return matching.length ? matching[matching.length - 1] : "";
}

function roleActorSession({ runtimeStatus, registry, role, key }) {
  const normalizedRole = String(role || "").trim().toUpperCase();
  if (String(runtimeStatus?.next_expected_actor || "").trim().toUpperCase() === normalizedRole) {
    const projected = normalizeSession(runtimeStatus?.next_expected_session);
    if (projected) return projected;
  }
  if (normalizedRole === "WP_VALIDATOR") {
    const routeTarget = normalizeSession(runtimeStatus?.route_anchor_target_session);
    if (String(runtimeStatus?.route_anchor_actor || "").trim().toUpperCase() === "WP_VALIDATOR" && routeTarget) {
      return routeTarget;
    }
    const authoritative = normalizeSession(runtimeStatus?.authoritative_review_actor_session);
    if (authoritative) return authoritative;
    const ofRecord = normalizeSession(runtimeStatus?.wp_validator_of_record);
    if (ofRecord) return ofRecord;
  }
  if (normalizedRole === "CODER") {
    const routeTarget = normalizeSession(runtimeStatus?.route_anchor_target_session);
    if (String(runtimeStatus?.route_anchor_actor || "").trim().toUpperCase() === "CODER" && routeTarget) {
      return routeTarget;
    }
    const authoritativeTarget = normalizeSession(runtimeStatus?.authoritative_review_target_session);
    if (authoritativeTarget) return authoritativeTarget;
  }
  const runtimeSession = latestRoleSessionFromRuntime(runtimeStatus, normalizedRole);
  if (runtimeSession) return runtimeSession;
  const registrySession = (registry.sessions || []).find((entry) => entry.session_key === key);
  return normalizeSession(registrySession?.session_id) || key;
}

// Pre-flight: verify hook is installed
const worktreeDir = repoPathAbs(defaultCoderWorktreeDir(wpId));
let hookInstalled = false;
try {
  const dotGitPath = path.join(worktreeDir, ".git");
  if (fs.existsSync(dotGitPath) && fs.statSync(dotGitPath).isFile()) {
    const gitdirContent = fs.readFileSync(dotGitPath, "utf8").trim();
    const match = gitdirContent.match(/^gitdir:\s*(.+)$/);
    if (match) {
      const realGitDir = path.isAbsolute(match[1]) ? match[1] : path.resolve(worktreeDir, match[1]);
      hookInstalled = fs.existsSync(path.join(realGitDir, "hooks", "post-commit"));
    }
  }
} catch {}
if (!hookInstalled) {
  console.warn(`[SEND_MT] WARNING: post-commit hook not found in coder worktree. Auto-relay will not fire on commits.`);
  console.warn(`[SEND_MT] Fix: just install-mt-hook ${wpId}`);
}

// Pre-flight: verify validator session is steerable
const { registry } = loadSessionRegistry(REPO_ROOT);
const runtimeStatus = readRuntimeStatusForWp(wpId);
const coderActorSession = roleActorSession({
  runtimeStatus,
  registry,
  role: "CODER",
  key: coderKey,
});
const validatorActorSession = roleActorSession({
  runtimeStatus,
  registry,
  role: "WP_VALIDATOR",
  key: validatorKey,
});
const valSession = registry.sessions.find((s) => s.session_key === validatorKey);
if (!valSession || !["READY", "COMMAND_RUNNING"].includes(valSession.runtime_state)) {
  console.warn(`[SEND_MT] WARNING: Validator session ${validatorKey} is not steerable (state: ${valSession?.runtime_state || "NOT_FOUND"}). Auto-relay dispatch will fail.`);
  console.warn(`[SEND_MT] Fix: just start-wp-validator-session ${wpId} PRIMARY`);
}

const prompt = [
  buildEphemeralContextBlock({
    source: "SEND_MT_PROMPT",
    trust: "required",
    body: [
      `MICROTASK ${mtId}: ${description}`,
      ``,
      `Your broker session key: ${coderKey}`,
      `Your receipt actor_session: ${coderActorSession}`,
      `Validator broker session key: ${validatorKey}`,
      `Validator receipt target_session: ${validatorActorSession}`,
      ``,
      `Instructions:`,
      `1. Implement the microtask described above`,
      `2. Use CARGO_TARGET_DIR='../Handshake_Artifacts/handshake-cargo-target' for builds`,
      `3. Commit with message: feat: ${mtId} ${description}`,
      `4. After commit, the git hook will automatically send a review request to the validator`,
      `5. STOP and wait for the validator's response before starting the next MT`,
      ``,
      `If the git hook does not fire, run manually:`,
      `just wp-review-request ${wpId} CODER ${coderActorSession} WP_VALIDATOR ${validatorActorSession} '${mtId} complete: ${description}'`,
      ``,
      `Receipt routing rule: actor_session and target_session are exact strings. Use the open review item's opened_by_session when replying to a review, not a synthesized session key.`,
    ].join("\n"),
  }),
].join("\n");

console.log(`[SEND_MT] WP: ${wpId}`);
console.log(`[SEND_MT] MT: ${mtId}`);
console.log(`[SEND_MT] Coder key: ${coderKey}`);
console.log(`[SEND_MT] Coder actor_session: ${coderActorSession}`);
console.log(`[SEND_MT] Validator key: ${validatorKey}`);
console.log(`[SEND_MT] Validator target_session: ${validatorActorSession}`);
console.log(`[SEND_MT] Dispatching...`);

try {
  const scriptPath = path.resolve(REPO_ROOT, ".GOV", "roles", "orchestrator", "scripts", "session-control-command.mjs");
  const output = execFileSync(process.execPath, [scriptPath, "SEND_PROMPT", "CODER", wpId, prompt, model], {
    encoding: "utf8",
    stdio: ["ignore", "pipe", "pipe"],
    cwd: REPO_ROOT,
    env: sessionControlEnv,
    windowsHide: true,
  });
  const lines = output.trim().split(/\r?\n/).filter(Boolean);
  for (const line of lines) console.log(line);
} catch (error) {
  const stderr = String(error?.stderr || "").trim();
  const stdout = String(error?.stdout || "").trim();
  console.error(`[SEND_MT] Dispatch failed: ${stderr || stdout || error.message}`);
  process.exit(1);
}
