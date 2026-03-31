#!/usr/bin/env node

import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import {
  communicationTransactionLockPathForWp,
  communicationPathsForWp,
  normalize,
  NOTIFICATIONS_FILE_NAME,
  NOTIFICATION_CURSOR_FILE_NAME,
  ROUTABLE_ROLE_VALUES,
} from "../lib/wp-communications-lib.mjs";
import { repoPathAbs, workPacketPath } from "../lib/runtime-paths.mjs";
import { withFileLockSync, writeJsonFile } from "../session/session-registry-lib.mjs";

function parseSingleField(text, label) {
  const re = new RegExp(`^\\s*-\\s*(?:\\*\\*)?${label}(?:\\*\\*)?\\s*:\\s*(.+)\\s*$`, "mi");
  const match = text.match(re);
  return match ? match[1].trim() : "";
}

function resolveCommDir(wpId) {
  const packetPath = workPacketPath(wpId);
  const packetAbsPath = repoPathAbs(packetPath);
  if (fs.existsSync(packetAbsPath)) {
    const text = fs.readFileSync(packetAbsPath, "utf8");
    const commDir = parseSingleField(text, "WP_COMMUNICATION_DIR");
    if (commDir && fs.existsSync(repoPathAbs(commDir))) return normalize(commDir);
  }
  return communicationPathsForWp(wpId).dir;
}

function loadCursor(cursorPath) {
  const cursorAbsPath = repoPathAbs(cursorPath);
  if (!fs.existsSync(cursorAbsPath)) return {};
  try {
    return JSON.parse(fs.readFileSync(cursorAbsPath, "utf8"));
  } catch {
    return {};
  }
}

function saveCursor(cursorPath, cursorData) {
  writeJsonFile(repoPathAbs(cursorPath), cursorData);
}

function normalizeSession(value) {
  const raw = String(value || "").trim();
  return raw || null;
}

function cursorKey(role, session = null) {
  const ROLE = String(role || "").trim().toUpperCase();
  const SESSION = normalizeSession(session);
  return SESSION ? `${ROLE}:${SESSION}` : ROLE;
}

function cursorEntry(cursorData, role, session = null) {
  const cursors = cursorData?.cursors && typeof cursorData.cursors === "object"
    ? cursorData.cursors
    : {};
  return cursors[cursorKey(role, session)] || null;
}

function notificationLastReadAt(cursorData, notification) {
  const role = String(notification?.target_role || "").trim().toUpperCase();
  const session = normalizeSession(notification?.target_session);
  if (!role) return null;

  if (session) {
    const sessionCursor = cursorEntry(cursorData, role, session);
    if (sessionCursor?.last_read_at) return sessionCursor.last_read_at;

    const legacyRoleCursor = cursorEntry(cursorData, role);
    if (legacyRoleCursor?.last_read_at && normalizeSession(legacyRoleCursor?.last_read_by_session) === session) {
      return legacyRoleCursor.last_read_at;
    }
    return null;
  }

  return cursorEntry(cursorData, role)?.last_read_at || null;
}

function filterPendingNotifications(allNotifications, cursorData, role, session = null) {
  const ROLE = String(role || "").trim().toUpperCase();
  const SESSION = normalizeSession(session);
  return allNotifications.filter((entry) => {
    if (String(entry.target_role || "").toUpperCase() !== ROLE) return false;
    const targetSession = normalizeSession(entry?.target_session);
    if (SESSION && targetSession && targetSession !== SESSION) return false;
    const lastReadAt = notificationLastReadAt(cursorData, entry);
    if (lastReadAt && entry.timestamp_utc <= lastReadAt) return false;
    return true;
  });
}

function ensureCursorStorage(cursorData) {
  if (!cursorData.schema_version) {
    cursorData.schema_version = "wp_notification_cursor@1";
  }
  if (!cursorData.cursors || typeof cursorData.cursors !== "object") {
    cursorData.cursors = {};
  }
}

function loadNotifications(notificationsPath) {
  const notificationsAbsPath = repoPathAbs(notificationsPath);
  if (!fs.existsSync(notificationsAbsPath)) return [];
  const text = fs.readFileSync(notificationsAbsPath, "utf8");
  const lines = text.split(/\r?\n/).map((line) => line.trim()).filter(Boolean);
  return lines.map((line, index) => {
    try {
      return JSON.parse(line);
    } catch {
      return null;
    }
  }).filter(Boolean);
}

function checkNotificationsCore({ wpId, role, ack = false, session = null } = {}) {
  const WP_ID = String(wpId || "").trim();
  const ROLE = String(role || "").trim().toUpperCase();
  const SESSION = normalizeSession(session);

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
  const pending = filterPendingNotifications(allNotifications, cursorData, ROLE, SESSION);
  const roleLastReadAt = cursorEntry(cursorData, ROLE)?.last_read_at || null;
  const sessionLastReadAt = SESSION ? cursorEntry(cursorData, ROLE, SESSION)?.last_read_at || null : null;

  const byKind = {};
  for (const entry of pending) {
    const kind = entry.source_kind || "UNKNOWN";
    byKind[kind] = (byKind[kind] || 0) + 1;
  }

  const result = {
    wpId: WP_ID,
    role: ROLE,
    session: SESSION,
    pendingCount: pending.length,
    byKind,
    notifications: pending,
    lastReadAt: SESSION ? sessionLastReadAt : roleLastReadAt,
    roleLastReadAt,
    sessionLastReadAt,
    cursorPath,
    notificationsPath,
  };

  if (ack) {
    if (!SESSION) {
      throw new Error("SESSION is required when acknowledging notifications");
    }
    if (pending.length > 0) {
      ensureCursorStorage(cursorData);
      const acknowledgedAt = new Date().toISOString();
      const targetedPending = pending.filter((entry) => normalizeSession(entry?.target_session) === SESSION);
      const untargetedPending = pending.filter((entry) => !normalizeSession(entry?.target_session));
      const latestTargetedTimestamp = targetedPending.reduce(
        (latest, entry) => (entry.timestamp_utc > latest ? entry.timestamp_utc : latest),
        "",
      );
      const latestUntargetedTimestamp = untargetedPending.reduce(
        (latest, entry) => (entry.timestamp_utc > latest ? entry.timestamp_utc : latest),
        "",
      );

      if (latestTargetedTimestamp) {
        cursorData.cursors[cursorKey(ROLE, SESSION)] = {
          last_read_at: latestTargetedTimestamp,
          last_read_by_session: SESSION,
          acknowledged_at: acknowledgedAt,
        };
      }
      if (latestUntargetedTimestamp) {
        cursorData.cursors[cursorKey(ROLE)] = {
          last_read_at: latestUntargetedTimestamp,
          last_read_by_session: SESSION,
          acknowledged_at: acknowledgedAt,
        };
      }

      saveCursor(cursorPath, cursorData);
      result.acknowledged = true;
      result.newCursorAt = [latestTargetedTimestamp, latestUntargetedTimestamp]
        .filter(Boolean)
        .sort()
        .at(-1) || null;
    } else {
      result.acknowledged = false;
      result.newCursorAt = null;
    }
  }

  return result;
}

export function checkNotifications(args = {}, options = {}) {
  const WP_ID = String(args?.wpId || "").trim();
  const run = () => checkNotificationsCore(args);
  if (options.assumeTransactionLock || !WP_ID || !/^WP-/.test(WP_ID)) {
    return run();
  }
  return withFileLockSync(communicationTransactionLockPathForWp(WP_ID), run);
}

export function checkAllNotifications({ wpId } = {}) {
  const WP_ID = String(wpId || "").trim();
  if (!WP_ID || !/^WP-/.test(WP_ID)) {
    throw new Error("WP_ID is required");
  }

  return withFileLockSync(communicationTransactionLockPathForWp(WP_ID), () => {
    const results = {};
    for (const role of ROUTABLE_ROLE_VALUES) {
      const check = checkNotifications({ wpId: WP_ID, role }, { assumeTransactionLock: true });
      if (check.pendingCount > 0) {
        results[role] = check;
      }
    }
    return results;
  });
}

function runCli() {
  const args = process.argv.slice(2);
  const wpId = args[0] || "";
  const role = args[1] || "";
  const positionalSession = normalizeSession(args[2] || "");
  const ackFlag = args.includes("--ack");
  const session = args.find((arg) => arg.startsWith("--session="))?.slice("--session=".length) || null;
  const allFlag = args.includes("--all");
  const resolvedSession = normalizeSession(session || positionalSession);

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

  const result = checkNotifications({ wpId, role, ack: ackFlag, session: resolvedSession });

  if (result.pendingCount === 0) {
    console.log(`[WP_NOTIFICATIONS] ${wpId} ${role}: no pending notifications`);
    if (resolvedSession) {
      console.log(`  session: ${resolvedSession}`);
    }
    if (result.lastReadAt) {
      console.log(`  last_read_at: ${result.lastReadAt}`);
    }
    return;
  }

  console.log(`[WP_NOTIFICATIONS] ${wpId} ${role}: ${result.pendingCount} pending notification(s)`);
  if (resolvedSession) {
    console.log(`  session: ${resolvedSession}`);
  }
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
