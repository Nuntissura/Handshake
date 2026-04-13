#!/usr/bin/env node

import { execFileSync } from "node:child_process";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { isInvokedAsMain } from "../lib/invocation-path-lib.mjs";
import {
  communicationTransactionLockPathForWp,
  communicationPathsForWp,
  normalize,
  NOTIFICATIONS_FILE_NAME,
  ROUTABLE_ROLE_VALUES,
} from "../lib/wp-communications-lib.mjs";
import { repoPathAbs, workPacketPath } from "../lib/runtime-paths.mjs";
import { appendJsonlLine, withFileLockSync } from "../session/session-registry-lib.mjs";

const ACTIVE_AUTO_RELAY_ROLE_VALUES = new Set(["CODER", "WP_VALIDATOR", "INTEGRATION_VALIDATOR"]);
const ORCHESTRATOR_STEER_SCRIPT_PATH = path.resolve(
  path.dirname(fileURLToPath(import.meta.url)),
  "../../..",
  "roles",
  "orchestrator",
  "scripts",
  "orchestrator-steer-next.mjs",
);

function parseSingleField(text, label) {
  const re = new RegExp(`^\\s*-\\s*(?:\\*\\*)?${label}(?:\\*\\*)?\\s*:\\s*(.+)\\s*$`, "mi");
  const match = text.match(re);
  return match ? match[1].trim() : "";
}

function resolveNotificationsFile(wpId) {
  const packetPath = workPacketPath(wpId);
  const packetAbsPath = repoPathAbs(packetPath);
  if (fs.existsSync(packetAbsPath)) {
    const text = fs.readFileSync(packetAbsPath, "utf8");
    const commDir = parseSingleField(text, "WP_COMMUNICATION_DIR");
    if (commDir && fs.existsSync(repoPathAbs(commDir))) {
      return normalize(path.join(commDir, NOTIFICATIONS_FILE_NAME));
    }
  }
  const paths = communicationPathsForWp(wpId);
  return normalize(path.join(paths.dir, NOTIFICATIONS_FILE_NAME));
}

function workflowLaneForWp(wpId) {
  const packetPath = workPacketPath(wpId);
  const packetAbsPath = repoPathAbs(packetPath);
  if (!fs.existsSync(packetAbsPath)) return "";
  const text = fs.readFileSync(packetAbsPath, "utf8");
  return parseSingleField(text, "WORKFLOW_LANE") || "";
}

function attemptOrchestratorAutoRelay({ wpId, notification }) {
  if (String(workflowLaneForWp(wpId) || "").trim().toUpperCase() !== "ORCHESTRATOR_MANAGED") {
    return { status: "NOT_APPLICABLE", reason: "NON_ORCHESTRATOR_MANAGED" };
  }
  const targetRole = String(notification?.target_role || "").trim().toUpperCase();
  const targetSession = String(notification?.target_session || "").trim();
  if (!ACTIVE_AUTO_RELAY_ROLE_VALUES.has(targetRole)) {
    return { status: "NOT_APPLICABLE", reason: "NO_GOVERNED_TARGET_ROLE" };
  }

  try {
    const steerArgs = [ORCHESTRATOR_STEER_SCRIPT_PATH, wpId, "PRIMARY", `--target-role=${targetRole}`];
    if (targetSession) steerArgs.push(`--target-session=${targetSession}`);
    const output = execFileSync(process.execPath, steerArgs, {
      encoding: "utf8",
      stdio: ["ignore", "pipe", "pipe"],
    });
    const outputLines = String(output || "")
      .split(/\r?\n/)
      .map((line) => line.trim())
      .filter(Boolean)
      .slice(-6);
    return {
      status: "DISPATCHED",
      reason: "AUTO_RELAY_TRIGGERED",
      next_actor: targetRole,
      output_lines: outputLines,
    };
  } catch (error) {
    const stderr = String(error?.stderr || "").trim();
    const stdout = String(error?.stdout || "").trim();
    return {
      status: "FAILED",
      reason: "AUTO_RELAY_FAILED",
      next_actor: targetRole,
      error: stderr || stdout || (error instanceof Error ? error.message : String(error)),
    };
  }
}

export function orchestratorSteerScriptPath() {
  return ORCHESTRATOR_STEER_SCRIPT_PATH;
}

export function appendWpNotificationCore({
  wpId,
  sourceKind,
  sourceRole,
  sourceSession,
  targetRole,
  targetSession = null,
  correlationId = null,
  summary,
  timestamp = null,
} = {}) {
  const WP_ID = String(wpId || "").trim();
  const TARGET_ROLE = String(targetRole || "").trim().toUpperCase();
  const SOURCE_ROLE = String(sourceRole || "").trim().toUpperCase();

  if (!WP_ID || !/^WP-/.test(WP_ID)) return null;
  if (!TARGET_ROLE || !ROUTABLE_ROLE_VALUES.includes(TARGET_ROLE)) return null;
  if (!SOURCE_ROLE) return null;
  if (SOURCE_ROLE === TARGET_ROLE) return null;

  const notificationsFile = resolveNotificationsFile(WP_ID);
  const notificationsAbsPath = repoPathAbs(notificationsFile);
  const dir = path.dirname(notificationsAbsPath);
  if (!fs.existsSync(dir)) return null;

  const entry = {
    schema_version: "wp_notification@1",
    timestamp_utc: String(timestamp || new Date().toISOString()),
    wp_id: WP_ID,
    source_kind: String(sourceKind || "THREAD_MESSAGE").trim().toUpperCase(),
    source_role: SOURCE_ROLE,
    source_session: String(sourceSession || "").trim(),
    target_role: TARGET_ROLE,
    target_session: targetSession ? String(targetSession).trim() : null,
    correlation_id: correlationId ? String(correlationId).trim() : null,
    summary: String(summary || "").trim(),
  };

  appendJsonlLine(notificationsAbsPath, entry);
  return entry;
}

export function appendWpNotification(args = {}, options = {}) {
  const WP_ID = String(args?.wpId || "").trim();
  const run = () => appendWpNotificationCore(args);
  const relayEnabled = options.autoRelay !== false;
  if (options.assumeTransactionLock || !WP_ID || !/^WP-/.test(WP_ID)) {
    const result = run();
    if (relayEnabled && result) {
      result.relayAttempt = attemptOrchestratorAutoRelay({ wpId: WP_ID, notification: result });
    }
    return result;
  }
  const result = withFileLockSync(communicationTransactionLockPathForWp(WP_ID), run);
  if (relayEnabled && result) {
    result.relayAttempt = attemptOrchestratorAutoRelay({ wpId: WP_ID, notification: result });
  }
  return result;
}

function resolveTargetRoleFromMention(target) {
  const mention = String(target || "").trim().toLowerCase();
  if (!mention) return null;
  if (mention === "@coder" || mention === "@cod") return "CODER";
  if (mention === "@wpval" || mention === "@wp_validator" || mention === "@wpvalidator") return "WP_VALIDATOR";
  if (mention === "@intval" || mention === "@integration_validator" || mention === "@ival") return "INTEGRATION_VALIDATOR";
  if (mention === "@validator" || mention === "@val") return "VALIDATOR";
  if (mention === "@orchestrator" || mention === "@orc") return "ORCHESTRATOR";
  if (mention === "@operator" || mention === "@op") return "OPERATOR";
  return null;
}

export { resolveTargetRoleFromMention, resolveNotificationsFile };

function runCli() {
  const [wpId, sourceKind, sourceRole, sourceSession, targetRole, summary, correlationId, targetSession] =
    process.argv.slice(2);
  if (!wpId || !sourceRole || !targetRole || !summary) {
    console.error(
      "Usage: node .GOV/roles_shared/scripts/wp/wp-notification-append.mjs"
      + " WP-{ID} <SOURCE_KIND> <SOURCE_ROLE> <SOURCE_SESSION> <TARGET_ROLE> \"<SUMMARY>\" [CORRELATION_ID] [TARGET_SESSION]"
    );
    process.exit(1);
  }

  const result = appendWpNotification({
    wpId,
    sourceKind,
    sourceRole,
    sourceSession,
    targetRole,
    targetSession,
    correlationId,
    summary,
  });

  if (result) {
    console.log(`[WP_NOTIFICATION] appended for ${wpId} -> ${result.target_role}`);
    console.log(`- source: ${result.source_role}:${result.source_session}`);
    console.log(`- kind: ${result.source_kind}`);
    console.log(`- summary: ${result.summary}`);
    if (result.relayAttempt && result.relayAttempt.status !== "NOT_APPLICABLE") {
      console.log(`- auto_relay_status: ${result.relayAttempt.status}`);
      console.log(`- auto_relay_reason: ${result.relayAttempt.reason}`);
      if (result.relayAttempt.next_actor) console.log(`- auto_relay_next_actor: ${result.relayAttempt.next_actor}`);
      if (result.relayAttempt.error) console.log(`- auto_relay_error: ${result.relayAttempt.error}`);
      if (Array.isArray(result.relayAttempt.output_lines) && result.relayAttempt.output_lines.length > 0) {
        console.log(`- auto_relay_output: ${result.relayAttempt.output_lines.join(" | ")}`);
      }
    }
  } else {
    console.log(`[WP_NOTIFICATION] skipped (no valid target or directory missing)`);
  }
}

if (isInvokedAsMain(import.meta.url, process.argv[1])) {
  runCli();
}
