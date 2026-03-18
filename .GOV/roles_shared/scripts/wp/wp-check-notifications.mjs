#!/usr/bin/env node

import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import {
  communicationPathsForWp,
  normalize,
  NOTIFICATIONS_FILE_NAME,
  NOTIFICATION_CURSOR_FILE_NAME,
  ROUTABLE_ROLE_VALUES,
} from "../lib/wp-communications-lib.mjs";

const PACKETS_DIR = path.join(".GOV", "task_packets");

function parseSingleField(text, label) {
  const re = new RegExp(`^\\s*-\\s*(?:\\*\\*)?${label}(?:\\*\\*)?\\s*:\\s*(.+)\\s*$`, "mi");
  const match = text.match(re);
  return match ? match[1].trim() : "";
}

function resolveCommDir(wpId) {
  const packetPath = path.join(PACKETS_DIR, `${wpId}.md`);
  if (fs.existsSync(packetPath)) {
    const text = fs.readFileSync(packetPath, "utf8");
    const commDir = parseSingleField(text, "WP_COMMUNICATION_DIR");
    if (commDir && fs.existsSync(commDir)) return normalize(commDir);
  }
  return communicationPathsForWp(wpId).dir;
}

function loadCursor(cursorPath) {
  if (!fs.existsSync(cursorPath)) return {};
  try {
    return JSON.parse(fs.readFileSync(cursorPath, "utf8"));
  } catch {
    return {};
  }
}

function saveCursor(cursorPath, cursorData) {
  fs.writeFileSync(cursorPath, `${JSON.stringify(cursorData, null, 2)}\n`, "utf8");
}

function loadNotifications(notificationsPath) {
  if (!fs.existsSync(notificationsPath)) return [];
  const text = fs.readFileSync(notificationsPath, "utf8");
  const lines = text.split(/\r?\n/).map((line) => line.trim()).filter(Boolean);
  return lines.map((line, index) => {
    try {
      return JSON.parse(line);
    } catch {
      return null;
    }
  }).filter(Boolean);
}

export function checkNotifications({ wpId, role, ack = false, session = null } = {}) {
  const WP_ID = String(wpId || "").trim();
  const ROLE = String(role || "").trim().toUpperCase();

  if (!WP_ID || !/^WP-/.test(WP_ID)) {
    throw new Error("WP_ID is required");
  }
  if (!ROLE || !ROUTABLE_ROLE_VALUES.includes(ROLE)) {
    throw new Error(`ROLE must be one of ${ROUTABLE_ROLE_VALUES.join(", ")}`);
  }

  const commDir = resolveCommDir(WP_ID);
  const notificationsPath = normalize(path.join(commDir, NOTIFICATIONS_FILE_NAME));
  const cursorPath = normalize(path.join(commDir, NOTIFICATION_CURSOR_FILE_NAME));

  const allNotifications = loadNotifications(notificationsPath);
  const cursorData = loadCursor(cursorPath);
  const roleCursor = cursorData.cursors?.[ROLE] || null;
  const lastReadAt = roleCursor?.last_read_at || null;

  const pending = allNotifications.filter((entry) => {
    if (String(entry.target_role || "").toUpperCase() !== ROLE) return false;
    if (lastReadAt && entry.timestamp_utc <= lastReadAt) return false;
    return true;
  });

  const byKind = {};
  for (const entry of pending) {
    const kind = entry.source_kind || "UNKNOWN";
    byKind[kind] = (byKind[kind] || 0) + 1;
  }

  const result = {
    wpId: WP_ID,
    role: ROLE,
    pendingCount: pending.length,
    byKind,
    notifications: pending,
    lastReadAt,
    cursorPath,
    notificationsPath,
  };

  if (ack && pending.length > 0) {
    const latestTimestamp = pending.reduce(
      (latest, entry) => (entry.timestamp_utc > latest ? entry.timestamp_utc : latest),
      "",
    );
    if (!cursorData.schema_version) {
      cursorData.schema_version = "wp_notification_cursor@1";
    }
    if (!cursorData.cursors) {
      cursorData.cursors = {};
    }
    cursorData.cursors[ROLE] = {
      last_read_at: latestTimestamp,
      last_read_by_session: session || null,
      acknowledged_at: new Date().toISOString(),
    };
    saveCursor(cursorPath, cursorData);
    result.acknowledged = true;
    result.newCursorAt = latestTimestamp;
  }

  return result;
}

export function checkAllNotifications({ wpId } = {}) {
  const WP_ID = String(wpId || "").trim();
  if (!WP_ID || !/^WP-/.test(WP_ID)) {
    throw new Error("WP_ID is required");
  }

  const results = {};
  for (const role of ROUTABLE_ROLE_VALUES) {
    const check = checkNotifications({ wpId: WP_ID, role });
    if (check.pendingCount > 0) {
      results[role] = check;
    }
  }
  return results;
}

function runCli() {
  const args = process.argv.slice(2);
  const wpId = args[0] || "";
  const role = args[1] || "";
  const ackFlag = args.includes("--ack");
  const session = args.find((arg) => arg.startsWith("--session="))?.slice("--session=".length) || null;
  const allFlag = args.includes("--all");

  if (!wpId) {
    console.error(
      "Usage: node .GOV/roles_shared/scripts/wp/wp-check-notifications.mjs WP-{ID} [ROLE] [--ack] [--session=ID] [--all]"
    );
    process.exit(1);
  }

  if (allFlag || !role) {
    const results = checkAllNotifications({ wpId });
    const roles = Object.keys(results);
    if (roles.length === 0) {
      console.log(`[WP_NOTIFICATIONS] ${wpId}: no pending notifications for any role`);
      return;
    }
    for (const [roleName, check] of Object.entries(results)) {
      console.log(`[WP_NOTIFICATIONS] ${wpId} ${roleName}: ${check.pendingCount} pending`);
      for (const [kind, count] of Object.entries(check.byKind)) {
        console.log(`  - ${kind}: ${count}`);
      }
    }
    return;
  }

  const result = checkNotifications({ wpId, role, ack: ackFlag, session });

  if (result.pendingCount === 0) {
    console.log(`[WP_NOTIFICATIONS] ${wpId} ${role}: no pending notifications`);
    if (result.lastReadAt) {
      console.log(`  last_read_at: ${result.lastReadAt}`);
    }
    return;
  }

  console.log(`[WP_NOTIFICATIONS] ${wpId} ${role}: ${result.pendingCount} pending notification(s)`);
  for (const [kind, count] of Object.entries(result.byKind)) {
    console.log(`  - ${kind}: ${count}`);
  }
  console.log("");
  for (const entry of result.notifications) {
    const source = `${entry.source_role}${entry.source_session ? `:${entry.source_session}` : ""}`;
    console.log(`  ${entry.timestamp_utc} | ${entry.source_kind} | from ${source}`);
    console.log(`    ${entry.summary}`);
  }

  if (result.acknowledged) {
    console.log("");
    console.log(`  [ACK] cursor updated to ${result.newCursorAt}`);
  }
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  runCli();
}
