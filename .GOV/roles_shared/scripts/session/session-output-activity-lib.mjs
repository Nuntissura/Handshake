import fs from "node:fs";

export const SESSION_PROGRESS_ITEM_TYPES = new Set([
  "command_execution",
  "file_change",
  "web_search",
  "todo_list",
]);

function readTailLines(filePath, count = 120) {
  const content = fs.readFileSync(filePath, "utf8");
  const lines = content.split(/\r?\n/).filter((line) => line.trim().length > 0);
  return lines.slice(-Math.max(1, count));
}

export function parseSessionOutputEntries(filePath, { tailLines = 120 } = {}) {
  if (!filePath || !fs.existsSync(filePath)) return [];
  const entries = [];
  for (const line of readTailLines(filePath, tailLines)) {
    try {
      entries.push(JSON.parse(line));
    } catch {
      // Ignore malformed rows in append-only output logs.
    }
  }
  return entries;
}

export function itemTypeOfEvent(event = null) {
  return String(event?.item?.type || "").trim();
}

export function isProgressItemType(itemType = "") {
  return SESSION_PROGRESS_ITEM_TYPES.has(String(itemType || "").trim());
}

export function isProgressEvent(event = null) {
  const eventType = String(event?.type || "").trim();
  if (!["item.started", "item.completed"].includes(eventType)) return false;
  return isProgressItemType(itemTypeOfEvent(event));
}

export function eventTimestampMs(event = null) {
  const raw = event?.timestamp || event?.item?.timestamp || "";
  const parsed = Date.parse(String(raw || "").trim());
  return Number.isNaN(parsed) ? null : parsed;
}

function latestMatchingEvent(entries = [], predicate = () => false) {
  for (let index = entries.length - 1; index >= 0; index -= 1) {
    const entry = entries[index];
    if (predicate(entry)) return entry;
  }
  return null;
}

export function findLatestProgressEvent(entries = []) {
  return latestMatchingEvent(entries, (entry) => isProgressEvent(entry));
}

export function findLatestAgentMessageEvent(entries = []) {
  return latestMatchingEvent(
    entries,
    (entry) => String(entry?.type || "").trim() === "item.completed" && itemTypeOfEvent(entry) === "agent_message",
  );
}

export function ageSecondsFromEvent(event = null, nowMs = Date.now()) {
  const timestampMs = eventTimestampMs(event);
  if (!Number.isFinite(timestampMs)) return null;
  return Math.max(0, Math.trunc((nowMs - timestampMs) / 1000));
}

function formatAgeCompact(ageSeconds) {
  if (!Number.isInteger(ageSeconds)) return "N/A";
  if (ageSeconds < 60) return `${ageSeconds}s`;
  const minutes = Math.trunc(ageSeconds / 60);
  if (minutes < 60) return `${minutes}m`;
  const hours = Math.trunc(minutes / 60);
  const remainingMinutes = minutes % 60;
  return remainingMinutes > 0 ? `${hours}h${remainingMinutes}m` : `${hours}h`;
}

export function summarizeActivityEvent(event = null, { nowMs = Date.now() } = {}) {
  if (!event) return "none";
  const eventType = String(event.type || "event").trim() || "event";
  const itemType = itemTypeOfEvent(event);
  const ageSeconds = ageSecondsFromEvent(event, nowMs);
  const label = itemType ? `${eventType}:${itemType}` : eventType;
  return `${label}@${formatAgeCompact(ageSeconds)}`;
}

export function inspectSessionOutputActivity(filePath, { tailLines = 120, nowMs = Date.now() } = {}) {
  if (!filePath || !fs.existsSync(filePath)) {
    return {
      exists: false,
      entries: [],
      latestEvent: null,
      latestProgressEvent: null,
      latestAgentMessageEvent: null,
      outputFileIdleSeconds: null,
    };
  }

  const stats = fs.statSync(filePath);
  const entries = parseSessionOutputEntries(filePath, { tailLines });
  const latestEvent = entries.length > 0 ? entries.at(-1) : null;
  return {
    exists: true,
    entries,
    latestEvent,
    latestProgressEvent: findLatestProgressEvent(entries),
    latestAgentMessageEvent: findLatestAgentMessageEvent(entries),
    outputFileIdleSeconds: Math.max(0, Math.trunc((nowMs - stats.mtimeMs) / 1000)),
  };
}
