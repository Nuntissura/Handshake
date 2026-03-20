#!/usr/bin/env node

import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import {
  communicationPathsForWp,
  normalize,
  NOTIFICATIONS_FILE_NAME,
  ROUTABLE_ROLE_VALUES,
} from "../lib/wp-communications-lib.mjs";
import { workPacketPath } from "../lib/runtime-paths.mjs";

function parseSingleField(text, label) {
  const re = new RegExp(`^\\s*-\\s*(?:\\*\\*)?${label}(?:\\*\\*)?\\s*:\\s*(.+)\\s*$`, "mi");
  const match = text.match(re);
  return match ? match[1].trim() : "";
}

function resolveNotificationsFile(wpId) {
  const packetPath = workPacketPath(wpId);
  if (fs.existsSync(packetPath)) {
    const text = fs.readFileSync(packetPath, "utf8");
    const commDir = parseSingleField(text, "WP_COMMUNICATION_DIR");
    if (commDir && fs.existsSync(commDir)) {
      return normalize(path.join(commDir, NOTIFICATIONS_FILE_NAME));
    }
  }
  const paths = communicationPathsForWp(wpId);
  return normalize(path.join(paths.dir, NOTIFICATIONS_FILE_NAME));
}

export function appendWpNotification({
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
  const dir = path.dirname(notificationsFile);
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

  fs.appendFileSync(notificationsFile, `${JSON.stringify(entry)}\n`, "utf8");
  return entry;
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
  } else {
    console.log(`[WP_NOTIFICATION] skipped (no valid target or directory missing)`);
  }
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  runCli();
}
