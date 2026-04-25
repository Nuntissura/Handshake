import fs from "node:fs";
import path from "node:path";
import { GOV_ROOT_REPO_REL } from "../lib/runtime-paths.mjs";

export const WORKFLOW_DOSSIER_TIMEZONE = "Europe/Brussels";

export const WORKFLOW_DOSSIER_SECTION_HEADINGS = {
  EXECUTION: "LIVE_EXECUTION_LOG",
  IDLE: "LIVE_IDLE_LEDGER",
  GOV_CHANGE: "LIVE_GOVERNANCE_CHANGE_LOG",
  CONCERN: "LIVE_CONCERNS_LOG",
  FINDING: "LIVE_FINDINGS_LOG",
};

export const REPOMEM_CHECKPOINT_TO_SECTION = {
  SESSION_OPEN: "EXECUTION",
  SESSION_CLOSE: "EXECUTION",
  PRE_TASK: "EXECUTION",
  DECISION: "EXECUTION",
  ERROR: "EXECUTION",
  ABANDON: "EXECUTION",
  INSIGHT: "FINDING",
  RESEARCH_CLOSE: "FINDING",
  CONCERN: "CONCERN",
  ESCALATION: "CONCERN",
};

export const REPOMEM_CHECKPOINT_TO_TAG = {
  SESSION_OPEN: "REPOMEM_OPEN",
  SESSION_CLOSE: "REPOMEM_CLOSE",
  PRE_TASK: "REPOMEM_PRE",
  DECISION: "REPOMEM_DECISION",
  ERROR: "REPOMEM_ERROR",
  ABANDON: "REPOMEM_ABANDON",
  INSIGHT: "REPOMEM_INSIGHT",
  RESEARCH_CLOSE: "REPOMEM_RESEARCH",
  CONCERN: "REPOMEM_CONCERN",
  ESCALATION: "REPOMEM_ESCALATION",
};

const WORKFLOW_DOSSIER_SECTION_ALIASES = {
  EXECUTION: "EXECUTION",
  LIVE_EXECUTION_LOG: "EXECUTION",
  IDLE: "IDLE",
  IDLE_LEDGER: "IDLE",
  LIVE_IDLE_LEDGER: "IDLE",
  GOV_CHANGE: "GOV_CHANGE",
  GOVERNANCE_CHANGE: "GOV_CHANGE",
  CHANGE: "GOV_CHANGE",
  LIVE_GOVERNANCE_CHANGE_LOG: "GOV_CHANGE",
  CONCERN: "CONCERN",
  CONCERNS: "CONCERN",
  LIVE_CONCERNS_LOG: "CONCERN",
  FINDING: "FINDING",
  FINDINGS: "FINDING",
  LIVE_FINDINGS_LOG: "FINDING",
};

export function normalizePath(value) {
  return String(value || "").replace(/\\/g, "/");
}

function dateTimeParts(value = new Date(), timeZone = WORKFLOW_DOSSIER_TIMEZONE) {
  const date = value instanceof Date ? value : new Date(value);
  return Object.fromEntries(
    new Intl.DateTimeFormat("en-CA", {
      timeZone,
      year: "numeric",
      month: "2-digit",
      day: "2-digit",
      hour: "2-digit",
      minute: "2-digit",
      second: "2-digit",
      hourCycle: "h23",
    }).formatToParts(date)
      .filter((part) => part.type !== "literal")
      .map((part) => [part.type, part.value]),
  );
}

export function formatWorkflowDossierTimestamp(value = new Date(), timeZone = WORKFLOW_DOSSIER_TIMEZONE) {
  const date = value instanceof Date ? value : new Date(value);
  if (Number.isNaN(date.getTime())) {
    return `INVALID_TIME ${timeZone}`;
  }
  const parts = dateTimeParts(date, timeZone);
  return `${parts.year}-${parts.month}-${parts.day} ${parts.hour}:${parts.minute}:${parts.second} ${timeZone}`;
}

export function normalizeWorkflowDossierSection(section = "") {
  const key = String(section || "").trim().toUpperCase();
  return WORKFLOW_DOSSIER_SECTION_ALIASES[key] || "";
}

function normalizeScalar(value = "") {
  return String(value || "").trim();
}

function normalizeCheckpointType(value = "") {
  return normalizeScalar(value).toUpperCase();
}

function previewText(value = "", maxLength = 200) {
  return normalizeScalar(value).replace(/\s+/g, " ").slice(0, maxLength);
}

export function selectRepomemEntriesForWorkflowDossier(entries = [], { wpId = "" } = {}) {
  const normalizedWpId = normalizeScalar(wpId);
  if (!normalizedWpId) return [];

  const normalizedEntries = (Array.isArray(entries) ? entries : [])
    .map((entry, index) => ({
      entry,
      index,
      wpId: normalizeScalar(entry?.wp_id),
      sessionId: normalizeScalar(entry?.session_id),
    }));
  const wpSessionIds = new Set(
    normalizedEntries
      .filter((item) => item.wpId === normalizedWpId && item.sessionId)
      .map((item) => item.sessionId),
  );

  return normalizedEntries
    .filter((item) =>
      item.wpId === normalizedWpId
      || (item.wpId === "" && item.sessionId && wpSessionIds.has(item.sessionId))
    )
    .sort((left, right) => {
      const leftTime = Date.parse(normalizeScalar(left.entry?.timestamp_utc));
      const rightTime = Date.parse(normalizeScalar(right.entry?.timestamp_utc));
      if (Number.isFinite(leftTime) && Number.isFinite(rightTime) && leftTime !== rightTime) {
        return leftTime - rightTime;
      }
      return left.index - right.index;
    })
    .map((item) => item.entry);
}

export function formatRepomemDossierEntry(entry = {}) {
  const checkpointType = normalizeCheckpointType(entry?.checkpoint_type);
  const section = REPOMEM_CHECKPOINT_TO_SECTION[checkpointType] || "";
  const tag = REPOMEM_CHECKPOINT_TO_TAG[checkpointType] || "";
  if (!section || !tag) return null;

  const timestamp = formatWorkflowDossierTimestamp(entry?.timestamp_utc);
  const role = normalizeScalar(entry?.role).toUpperCase() || "ORCHESTRATOR";
  const topic = previewText(entry?.topic);
  const contentPreview = previewText(entry?.content);
  const display = contentPreview && contentPreview !== topic
    ? `${topic} :: ${contentPreview}`
    : topic;
  const sessionId = normalizeScalar(entry?.session_id);

  return {
    section,
    tag,
    timestamp,
    sessionId,
    line: `- [${timestamp}] [${role}] [${tag}] [GOVERNANCE_MEMORY] [${sessionId}] ${display}`,
  };
}

function workflowDossierDirectory(repoRoot) {
  return path.resolve(repoRoot, GOV_ROOT_REPO_REL, "Audits", "smoketest");
}

export function findOpenWorkflowDossierPath(repoRoot, wpId) {
  const dossierDir = workflowDossierDirectory(repoRoot);
  if (!fs.existsSync(dossierDir)) return "";
  const matches = fs.readdirSync(dossierDir)
    .filter((entry) => entry.toLowerCase().endsWith(".md"))
    .map((entry) => {
      const absPath = path.join(dossierDir, entry);
      try {
        const text = fs.readFileSync(absPath, "utf8");
        const targetsWp = text.includes(`- ACTIVE_RECOVERY_PACKET: ${wpId}`);
        const open = text.includes("- LIVE_REVIEW_STATUS: OPEN");
        return targetsWp && open
          ? { absPath, mtimeMs: fs.statSync(absPath).mtimeMs }
          : null;
      } catch {
        return null;
      }
    })
    .filter(Boolean)
    .sort((left, right) => right.mtimeMs - left.mtimeMs);
  return matches[0]?.absPath || "";
}

export function resolveWorkflowDossierPath(repoRoot, { wpId = "", filePath = "" } = {}) {
  if (filePath) {
    return path.isAbsolute(filePath)
      ? filePath
      : path.resolve(repoRoot, filePath);
  }
  if (!wpId) return "";
  return findOpenWorkflowDossierPath(repoRoot, wpId);
}

export function appendWorkflowDossierEntry({
  repoRoot,
  wpId = "",
  filePath = "",
  section = "",
  line = "",
  dedupeSuffix = "",
} = {}) {
  const canonicalSection = normalizeWorkflowDossierSection(section);
  const dossierPath = resolveWorkflowDossierPath(repoRoot, { wpId, filePath });
  if (!dossierPath || !line || !canonicalSection) return "";

  const heading = `## ${WORKFLOW_DOSSIER_SECTION_HEADINGS[canonicalSection]}`;
  const content = fs.readFileSync(dossierPath, "utf8");
  const lines = content.split(/\r?\n/);
  const headingIndex = lines.findIndex((entry) => entry.trim() === heading);

  if (headingIndex === -1) {
    const nextContent = `${content.trimEnd()}\n\n${heading}\n\n${line}\n`;
    fs.writeFileSync(dossierPath, nextContent, "utf8");
    return dossierPath;
  }

  let insertIndex = lines.findIndex((entry, index) => index > headingIndex && /^##\s+/.test(entry));
  if (insertIndex === -1) insertIndex = lines.length;
  while (insertIndex > (headingIndex + 1) && String(lines[insertIndex - 1] || "").trim() === "") {
    insertIndex -= 1;
  }
  const normalizedDedupeSuffix = String(dedupeSuffix || "").trim();
  if (normalizedDedupeSuffix) {
    let previousEntryIndex = insertIndex - 1;
    while (previousEntryIndex > headingIndex && String(lines[previousEntryIndex] || "").trim() === "") {
      previousEntryIndex -= 1;
    }
    const previousEntry = previousEntryIndex > headingIndex ? String(lines[previousEntryIndex] || "").trimEnd() : "";
    const nextEntry = String(line || "").trimEnd();
    if (previousEntry.endsWith(normalizedDedupeSuffix) && nextEntry.endsWith(normalizedDedupeSuffix)) {
      return dossierPath;
    }
  }
  lines.splice(insertIndex, 0, line);
  fs.writeFileSync(dossierPath, `${lines.join("\n").replace(/\n*$/, "\n")}`, "utf8");
  return dossierPath;
}
