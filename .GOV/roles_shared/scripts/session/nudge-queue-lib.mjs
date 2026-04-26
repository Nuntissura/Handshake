import crypto from "node:crypto";
import fs from "node:fs";
import path from "node:path";
import { GOVERNANCE_RUNTIME_ROOT_ABS } from "../lib/runtime-paths.mjs";

export const NUDGE_KIND_VALUES = [
  "STEER",
  "RELAUNCH_REQUEST",
  "PHASE_TRANSITION",
  "MT_VERDICT",
  "GOVERNANCE_REMINDER",
];
export const NUDGE_PRIORITY_VALUES = ["normal", "urgent"];
export const NUDGE_MAX_QUEUE_DEPTH = 50;
export const NUDGE_ORPHAN_RECOVERY_AGE_MS = 5 * 60 * 1000;
export const NUDGE_DEFAULT_TTL_SEC = 1800;
export const NUDGE_URGENT_TTL_SEC = 7200;

function nowIso() {
  return new Date().toISOString();
}

function safeSegment(value = "") {
  return String(value || "").trim().replace(/[^A-Za-z0-9_.-]+/g, "_") || "unknown";
}

function inferWpIdFromSessionId(sessionId = "") {
  const match = String(sessionId || "").match(/\b(WP-[A-Za-z0-9_.-]+)/);
  return match ? match[1] : "";
}

function queueRoot(runtimeRootAbs = GOVERNANCE_RUNTIME_ROOT_ABS) {
  return path.join(path.resolve(runtimeRootAbs), "nudges");
}

export function nudgeQueueDir({ sessionId = "", wpId = "", runtimeRootAbs = GOVERNANCE_RUNTIME_ROOT_ABS } = {}) {
  const resolvedWpId = String(wpId || "").trim() || inferWpIdFromSessionId(sessionId) || "WP-UNKNOWN";
  return path.join(queueRoot(runtimeRootAbs), safeSegment(resolvedWpId), safeSegment(sessionId));
}

function listSessionQueueDirs({ sessionId = "", wpId = "", runtimeRootAbs = GOVERNANCE_RUNTIME_ROOT_ABS } = {}) {
  if (wpId) return [nudgeQueueDir({ sessionId, wpId, runtimeRootAbs })];
  const root = queueRoot(runtimeRootAbs);
  if (!fs.existsSync(root)) return [];
  const sessionSegment = safeSegment(sessionId);
  return fs.readdirSync(root, { withFileTypes: true })
    .filter((entry) => entry.isDirectory())
    .map((entry) => path.join(root, entry.name, sessionSegment))
    .filter((dir) => fs.existsSync(dir));
}

function listFiles(dir, suffix) {
  if (!fs.existsSync(dir)) return [];
  return fs.readdirSync(dir, { withFileTypes: true })
    .filter((entry) => entry.isFile() && entry.name.endsWith(suffix))
    .map((entry) => path.join(dir, entry.name))
    .sort((left, right) => path.basename(left).localeCompare(path.basename(right)));
}

function parseTimestampMs(value) {
  const ms = Date.parse(String(value || "").trim());
  return Number.isNaN(ms) ? null : ms;
}

function isExpired(payload = {}, nowMs = Date.now()) {
  const expiresMs = parseTimestampMs(payload.expires_at);
  return Number.isFinite(expiresMs) && expiresMs <= nowMs;
}

function normalizePriority(value = "") {
  const priority = String(value || "").trim().toLowerCase();
  return NUDGE_PRIORITY_VALUES.includes(priority) ? priority : "normal";
}

function normalizePayload({ sessionId = "", payload = {}, ttl = null, priority = "" } = {}) {
  const normalizedPriority = normalizePriority(priority || payload.priority);
  const ttlSec = Number.isInteger(ttl) && ttl > 0
    ? ttl
    : normalizedPriority === "urgent"
      ? NUDGE_URGENT_TTL_SEC
      : NUDGE_DEFAULT_TTL_SEC;
  const enqueuedAt = payload.enqueued_at || nowIso();
  const enqueuedMs = parseTimestampMs(enqueuedAt) || Date.now();
  const expiresAt = payload.expires_at || new Date(enqueuedMs + ttlSec * 1000).toISOString();
  return {
    ...payload,
    kind: String(payload.kind || "").trim().toUpperCase(),
    from_role: String(payload.from_role || "").trim().toUpperCase(),
    wp_id: String(payload.wp_id || inferWpIdFromSessionId(sessionId) || "").trim(),
    correlation_id: String(payload.correlation_id || "").trim(),
    body: payload.body && typeof payload.body === "object" && !Array.isArray(payload.body) ? payload.body : {},
    enqueued_at: enqueuedAt,
    expires_at: expiresAt,
    priority: normalizedPriority,
    delivery_attempts: Number.isInteger(payload.delivery_attempts) && payload.delivery_attempts >= 0
      ? payload.delivery_attempts
      : 0,
  };
}

export function validateNudgePayload(payload = {}) {
  const errors = [];
  if (!NUDGE_KIND_VALUES.includes(payload.kind)) errors.push(`kind must be one of ${NUDGE_KIND_VALUES.join(" | ")}`);
  if (!payload.from_role) errors.push("from_role is required");
  if (!payload.wp_id || !/^WP-/.test(payload.wp_id)) errors.push("wp_id must be a WP id");
  if (!payload.correlation_id) errors.push("correlation_id is required");
  if (!payload.body || typeof payload.body !== "object" || Array.isArray(payload.body)) errors.push("body must be an object");
  if (!payload.enqueued_at || Number.isNaN(Date.parse(payload.enqueued_at))) errors.push("enqueued_at must be ISO8601");
  if (!payload.expires_at || Number.isNaN(Date.parse(payload.expires_at))) errors.push("expires_at must be ISO8601");
  if (!NUDGE_PRIORITY_VALUES.includes(payload.priority)) errors.push("priority must be normal or urgent");
  if (!Number.isInteger(payload.delivery_attempts) || payload.delivery_attempts < 0) {
    errors.push("delivery_attempts must be an integer >= 0");
  }
  return errors;
}

function recoverOrphansInDir(dir, { nowMs = Date.now(), orphanRecoveryAgeMs = NUDGE_ORPHAN_RECOVERY_AGE_MS } = {}) {
  let recovered = 0;
  for (const claimedPath of listFiles(dir, ".claimed")) {
    let stat;
    try {
      stat = fs.statSync(claimedPath);
    } catch {
      continue;
    }
    if (nowMs - stat.mtimeMs < orphanRecoveryAgeMs) continue;
    const jsonPath = claimedPath.replace(/\.claimed$/, ".json");
    try {
      fs.renameSync(claimedPath, jsonPath);
      recovered += 1;
    } catch {
      // Another drainer may have recovered it first.
    }
  }
  return recovered;
}

export function expirePastTtl(sessionId, {
  wpId = "",
  runtimeRootAbs = GOVERNANCE_RUNTIME_ROOT_ABS,
  nowMs = Date.now(),
} = {}) {
  let expired = 0;
  for (const dir of listSessionQueueDirs({ sessionId, wpId, runtimeRootAbs })) {
    for (const filePath of listFiles(dir, ".json")) {
      let payload;
      try {
        payload = JSON.parse(fs.readFileSync(filePath, "utf8"));
      } catch {
        continue;
      }
      if (!isExpired(payload, nowMs)) continue;
      fs.unlinkSync(filePath);
      expired += 1;
    }
  }
  return expired;
}

export function listQueueDepth(sessionId, {
  wpId = "",
  runtimeRootAbs = GOVERNANCE_RUNTIME_ROOT_ABS,
} = {}) {
  expirePastTtl(sessionId, { wpId, runtimeRootAbs });
  return listSessionQueueDirs({ sessionId, wpId, runtimeRootAbs })
    .reduce((count, dir) => count + listFiles(dir, ".json").length, 0);
}

export function enqueueNudge({
  sessionId = "",
  payload = {},
  ttl = null,
  priority = "normal",
  runtimeRootAbs = GOVERNANCE_RUNTIME_ROOT_ABS,
} = {}) {
  const normalizedSessionId = String(sessionId || "").trim();
  if (!normalizedSessionId) return { ok: false, queueDepth: 0, error: "sessionId is required" };
  const normalizedPayload = normalizePayload({ sessionId: normalizedSessionId, payload, ttl, priority });
  const errors = validateNudgePayload(normalizedPayload);
  if (errors.length > 0) return { ok: false, queueDepth: 0, error: errors.join("; ") };

  const dir = nudgeQueueDir({
    sessionId: normalizedSessionId,
    wpId: normalizedPayload.wp_id,
    runtimeRootAbs,
  });
  fs.mkdirSync(dir, { recursive: true });
  const queueDepth = listQueueDepth(normalizedSessionId, {
    wpId: normalizedPayload.wp_id,
    runtimeRootAbs,
  });
  if (queueDepth >= NUDGE_MAX_QUEUE_DEPTH) {
    return { ok: false, queueDepth, error: `nudge queue depth cap reached (${NUDGE_MAX_QUEUE_DEPTH})` };
  }

  const stamp = `${BigInt(Date.now()) * 1000000n}`;
  const fileName = `${stamp}-${crypto.randomBytes(4).toString("hex")}.json`;
  const filePath = path.join(dir, fileName);
  fs.writeFileSync(filePath, `${JSON.stringify(normalizedPayload, null, 2)}\n`, "utf8");
  return {
    ok: true,
    queueDepth: queueDepth + 1,
    filePath,
    payload: normalizedPayload,
  };
}

export function drainNudges({
  sessionId = "",
  wpId = "",
  drainerId = "",
  deliver = null,
  runtimeRootAbs = GOVERNANCE_RUNTIME_ROOT_ABS,
  nowMs = Date.now(),
  orphanRecoveryAgeMs = NUDGE_ORPHAN_RECOVERY_AGE_MS,
} = {}) {
  const normalizedSessionId = String(sessionId || "").trim();
  if (!normalizedSessionId) {
    return { nudges: [], failed: [], expired: 0, orphansRecovered: 0, error: "sessionId is required" };
  }

  const dirs = listSessionQueueDirs({ sessionId: normalizedSessionId, wpId, runtimeRootAbs });
  let orphansRecovered = 0;
  let expired = 0;
  const nudges = [];
  const failed = [];

  for (const dir of dirs) {
    orphansRecovered += recoverOrphansInDir(dir, { nowMs, orphanRecoveryAgeMs });
    for (const filePath of listFiles(dir, ".json")) {
      const claimedPath = filePath.replace(/\.json$/, ".claimed");
      try {
        fs.renameSync(filePath, claimedPath);
      } catch {
        continue;
      }

      let payload;
      try {
        payload = JSON.parse(fs.readFileSync(claimedPath, "utf8"));
      } catch (error) {
        failed.push({ filePath, error: error.message || String(error) });
        try { fs.renameSync(claimedPath, filePath); } catch {}
        continue;
      }

      if (isExpired(payload, nowMs)) {
        fs.unlinkSync(claimedPath);
        expired += 1;
        continue;
      }

      const deliveryPayload = {
        ...payload,
        delivery_attempts: Number.isInteger(payload.delivery_attempts) ? payload.delivery_attempts + 1 : 1,
        delivered_by: String(drainerId || "").trim() || null,
      };
      try {
        if (typeof deliver === "function") deliver(deliveryPayload);
        fs.unlinkSync(claimedPath);
        nudges.push(deliveryPayload);
      } catch (error) {
        failed.push({ payload: deliveryPayload, filePath, error: error.message || String(error) });
        try {
          fs.writeFileSync(claimedPath, `${JSON.stringify(deliveryPayload, null, 2)}\n`, "utf8");
          fs.renameSync(claimedPath, filePath);
        } catch {}
      }
    }
  }

  return { nudges, failed, expired, orphansRecovered };
}
