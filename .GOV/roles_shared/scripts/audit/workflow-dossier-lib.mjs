import fs from "node:fs";
import path from "node:path";
import { GOV_ROOT_REPO_REL } from "../lib/runtime-paths.mjs";

export const WORKFLOW_DOSSIER_TIMEZONE = "Europe/Brussels";

export const WORKFLOW_DOSSIER_SECTION_HEADINGS = {
  ORCHESTRATOR_DIAGNOSTIC: "LIVE_ORCHESTRATOR_DIAGNOSTIC_LOG",
  ACP_TRACE: "LIVE_ACP_SESSION_TRACE",
  TERMINAL_REPOMEM: "CLOSEOUT_REPOMEM_IMPORT",
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
  ORCHESTRATOR_DIAGNOSTIC: "ORCHESTRATOR_DIAGNOSTIC",
  ORCH_DIAGNOSTIC: "ORCHESTRATOR_DIAGNOSTIC",
  LIVE_ORCHESTRATOR_DIAGNOSTIC_LOG: "ORCHESTRATOR_DIAGNOSTIC",
  ACP_TRACE: "ACP_TRACE",
  LIVE_ACP_SESSION_TRACE: "ACP_TRACE",
  SESSION_TRACE: "ACP_TRACE",
  TERMINAL_REPOMEM: "TERMINAL_REPOMEM",
  CLOSEOUT_REPOMEM: "TERMINAL_REPOMEM",
  CLOSEOUT_REPOMEM_IMPORT: "TERMINAL_REPOMEM",
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

export function formatRepomemDossierSnapshotEntry(entry = {}) {
  const checkpointType = normalizeCheckpointType(entry?.checkpoint_type) || "CHECKPOINT";
  const timestamp = formatWorkflowDossierTimestamp(entry?.timestamp_utc);
  const role = normalizeScalar(entry?.role).toUpperCase() || "ROLE";
  const sessionId = normalizeScalar(entry?.session_id) || "NO_SESSION";
  const wpId = normalizeScalar(entry?.wp_id) || "GLOBAL";
  const topic = normalizeScalar(entry?.topic).replace(/\s+/g, " ");
  const content = normalizeScalar(entry?.content).replace(/\s+/g, " ");
  const entryId = normalizeScalar(entry?.id || entry?.checkpoint_id || entry?.rowid);
  const idPart = entryId ? ` id=${entryId}` : "";
  const payload = content && content !== topic
    ? `${topic} :: ${content}`
    : topic || content || "<empty>";
  return {
    section: "TERMINAL_REPOMEM",
    tag: `REPOMEM_${checkpointType}`,
    timestamp,
    sessionId,
    line: `- [${timestamp}] [${role}] [REPOMEM_${checkpointType}] [GOVERNANCE_MEMORY] [${sessionId}] wp=${wpId}${idPart} :: ${payload}`,
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

function escapeRegex(value = "") {
  return String(value).replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

function findWorkflowDossierSectionHeadingIndex(lines = [], canonicalSection = "") {
  const heading = WORKFLOW_DOSSIER_SECTION_HEADINGS[canonicalSection];
  if (!heading) return -1;
  const pattern = new RegExp(`^##\\s+${escapeRegex(heading)}(?:\\s|\\(|$)`);
  return lines.findIndex((entry) => pattern.test(String(entry || "").trim()));
}

function findNextSectionHeadingIndex(lines = [], headingIndex = -1) {
  if (headingIndex < 0) return -1;
  const nextIndex = lines.findIndex((entry, index) => index > headingIndex && /^##\s+/.test(entry));
  return nextIndex === -1 ? lines.length : nextIndex;
}

function findTopSectionPlacement(lines = []) {
  const metadataIndex = lines.findIndex((entry) => String(entry || "").trim() === "## METADATA");
  if (metadataIndex !== -1) {
    const separatorIndex = lines.findIndex((entry, index) => index > metadataIndex && String(entry || "").trim() === "---");
    if (separatorIndex !== -1) return separatorIndex + 1;
    const nextHeading = findNextSectionHeadingIndex(lines, metadataIndex);
    if (nextHeading !== -1) return nextHeading;
  }

  const h1Index = lines.findIndex((entry) => /^#\s+/.test(String(entry || "").trim()));
  if (h1Index !== -1) {
    let index = h1Index + 1;
    while (index < lines.length && String(lines[index] || "").trim() === "") index += 1;
    return index;
  }
  return 0;
}

function createWorkflowDossierSection(lines = [], canonicalSection = "", insertMode = "section-append") {
  const heading = `## ${WORKFLOW_DOSSIER_SECTION_HEADINGS[canonicalSection]}`;
  const normalizedMode = String(insertMode || "").trim().toLowerCase();
  if (normalizedMode === "section-prepend" || normalizedMode === "top-prepend") {
    const insertAt = findTopSectionPlacement(lines);
    const block = ["", heading, ""];
    lines.splice(insertAt, 0, ...block);
    return insertAt + 1;
  }

  while (lines.length > 0 && String(lines[lines.length - 1] || "").trim() === "") {
    lines.pop();
  }
  lines.push("", heading, "");
  return lines.length - 2;
}

function findPrependInsertIndex(lines = [], headingIndex = -1) {
  const nextHeading = findNextSectionHeadingIndex(lines, headingIndex);
  const firstEntry = lines.findIndex((entry, index) =>
    index > headingIndex
    && index < nextHeading
    && /^\s*-\s+/.test(String(entry || ""))
  );
  if (firstEntry !== -1) return firstEntry;
  let insertIndex = nextHeading;
  while (insertIndex > headingIndex + 1 && String(lines[insertIndex - 1] || "").trim() === "") {
    insertIndex -= 1;
  }
  return insertIndex;
}

function findAppendInsertIndex(lines = [], headingIndex = -1) {
  let insertIndex = findNextSectionHeadingIndex(lines, headingIndex);
  while (insertIndex > (headingIndex + 1) && String(lines[insertIndex - 1] || "").trim() === "") {
    insertIndex -= 1;
  }
  return insertIndex;
}

function shouldSkipConsecutiveDuplicate({
  lines = [],
  headingIndex = -1,
  insertIndex = -1,
  line = "",
  dedupeSuffix = "",
  insertMode = "section-append",
} = {}) {
  const normalizedDedupeSuffix = String(dedupeSuffix || "").trim();
  if (!normalizedDedupeSuffix) return false;

  const normalizedMode = String(insertMode || "").trim().toLowerCase();
  let compareIndex = normalizedMode === "section-prepend" || normalizedMode === "top-prepend"
    ? insertIndex
    : insertIndex - 1;
  const step = normalizedMode === "section-prepend" || normalizedMode === "top-prepend" ? 1 : -1;
  while (
    compareIndex > headingIndex
    && compareIndex < lines.length
    && String(lines[compareIndex] || "").trim() === ""
  ) {
    compareIndex += step;
  }
  const previousEntry = compareIndex > headingIndex && compareIndex < lines.length
    ? String(lines[compareIndex] || "").trimEnd()
    : "";
  const nextEntry = String(line || "").trimEnd();
  return previousEntry.endsWith(normalizedDedupeSuffix) && nextEntry.endsWith(normalizedDedupeSuffix);
}

export function appendWorkflowDossierEntry({
  repoRoot,
  wpId = "",
  filePath = "",
  section = "",
  line = "",
  dedupeSuffix = "",
  insertMode = "section-append",
} = {}) {
  const canonicalSection = normalizeWorkflowDossierSection(section);
  const dossierPath = resolveWorkflowDossierPath(repoRoot, { wpId, filePath });
  if (!dossierPath || !line || !canonicalSection) return "";

  const content = fs.readFileSync(dossierPath, "utf8");
  const lines = content.split(/\r?\n/);
  const normalizedMode = String(insertMode || "section-append").trim().toLowerCase();
  let headingIndex = findWorkflowDossierSectionHeadingIndex(lines, canonicalSection);

  if (headingIndex === -1) {
    headingIndex = createWorkflowDossierSection(lines, canonicalSection, normalizedMode);
  }

  const insertIndex = normalizedMode === "section-prepend" || normalizedMode === "top-prepend"
    ? findPrependInsertIndex(lines, headingIndex)
    : findAppendInsertIndex(lines, headingIndex);
  if (shouldSkipConsecutiveDuplicate({ lines, headingIndex, insertIndex, line, dedupeSuffix, insertMode: normalizedMode })) {
    return dossierPath;
  }

  lines.splice(insertIndex, 0, line);
  fs.writeFileSync(dossierPath, `${lines.join("\n").replace(/\n*$/, "\n")}`, "utf8");
  return dossierPath;
}

function lineNumberForOffset(text = "", offset = 0) {
  return String(text || "").slice(0, Math.max(0, offset)).split(/\r?\n/).length;
}

function collectPatternDiagnostics(content = "", patterns = []) {
  const diagnostics = [];
  for (const { pattern, code, message } of patterns) {
    for (const match of String(content || "").matchAll(pattern)) {
      diagnostics.push({
        code,
        line: lineNumberForOffset(content, match.index || 0),
        message,
        evidence: String(match[0] || "").trim().slice(0, 160),
      });
    }
  }
  return diagnostics;
}

export function evaluateWorkflowDossierJudgment({
  content = "",
  terminalTruth = {},
} = {}) {
  const terminal = Boolean(terminalTruth?.terminal)
    || /^(PASS|FAIL|OUTDATED_ONLY|ABANDONED|SUPERSEDED)$/i.test(String(terminalTruth?.verdict || "").trim());
  const verdict = String(terminalTruth?.verdict || (terminal ? "TERMINAL" : "UNKNOWN")).trim().toUpperCase();
  const diagnostics = [];

  if (!terminal) {
    return {
      ok: true,
      terminal: false,
      verdict,
      diagnostics,
      summary: "Workflow Dossier judgment check skipped because terminal verdict truth is not asserted.",
    };
  }

  diagnostics.push(...collectPatternDiagnostics(content, [
    {
      pattern: /\bNONE\s+yet\b/gi,
      code: "PLACEHOLDER_NONE_YET",
      message: "Closeout judgment still contains NONE yet placeholder text.",
    },
    {
      pattern: /<SET_AT_CLOSEOUT>|<[^>\n]{1,80}>/gi,
      code: "PLACEHOLDER_ANGLE_BRACKET",
      message: "Closeout judgment still contains angle-bracket placeholder text.",
    },
    {
      pattern: /\b(?:TBD|TODO|PENDING JUDGMENT)\b/gi,
      code: "PLACEHOLDER_TBD",
      message: "Closeout judgment still contains unresolved TBD/TODO/PENDING text.",
    },
    {
      pattern: /\bMT-[A-Za-z0-9._-]+\s+not\s+started\b/gi,
      code: "TERMINAL_MT_NOT_STARTED",
      message: "Terminal dossier cannot claim a microtask was not started after final verdict.",
    },
  ]));

  const rubricHeadings = [
    /##\s+LIVE_EXECUTION_LOG\b/i,
    /##\s+LIVE_IDLE_LEDGER\b/i,
    /##\s+LIVE_GOVERNANCE_CHANGE_LOG\b/i,
    /##\s+LIVE_CONCERNS_LOG\b/i,
    /##\s+LIVE_FINDINGS_LOG\b/i,
  ];
  for (const heading of rubricHeadings) {
    if (!heading.test(String(content || ""))) {
      diagnostics.push({
        code: "MISSING_LIVE_DOSSIER_SECTION",
        line: 1,
        message: `Required live dossier section is missing: ${heading.source.replace(/\\s\+/g, " ").replace(/\\b|\^|##/g, "").trim()}`,
        evidence: "",
      });
    }
  }

  return {
    ok: diagnostics.length === 0,
    terminal: true,
    verdict,
    diagnostics,
    summary: diagnostics.length === 0
      ? "Workflow Dossier terminal judgment has no unresolved placeholders in required live sections."
      : `Workflow Dossier terminal judgment has ${diagnostics.length} unresolved closeout judgment issue(s).`,
  };
}
