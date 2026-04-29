import fs from "node:fs";
import path from "node:path";

export const REPO_GOVERNANCE_BOARD_PATH = ".GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_TASK_BOARD.md";
export const REPO_GOVERNANCE_CHANGELOG_PATH = ".GOV/roles_shared/records/REPO_GOVERNANCE_CHANGELOG.md";
export const WP1_POSTMORTEM_GUIDE_PATH = ".GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_IMPLEMENTATION_GUIDE_20260429_WP1_POSTMORTEM.md";

const STATUS_VALUES = new Set(["PLANNED", "QUEUED", "IN_PROGRESS", "BLOCKED", "DONE", "HOLD"]);
const RGF_ID_RE = /\bRGF-(\d+)\b/g;
const IMPLEMENTED_RE = /\b(?:implemented|complete|completed|verified|done)\b/i;
const PLANNED_RE = /\bplanned\b/i;
const QUEUED_RE = /\bqueued\b/i;

function normalizePathText(value = "") {
  return String(value || "").replace(/\\/g, "/").trim();
}

function normalizeStatus(value = "") {
  return String(value || "").trim().toUpperCase();
}

function stripCode(value = "") {
  return String(value || "").replace(/`/g, "").trim();
}

function splitMarkdownTableRow(line = "") {
  const trimmed = String(line || "").trim();
  if (!trimmed.startsWith("|") || !trimmed.endsWith("|")) return [];
  return trimmed
    .slice(1, -1)
    .split("|")
    .map((cell) => cell.trim());
}

function tableSeparator(cells = []) {
  return cells.length > 0 && cells.every((cell) => /^:?-{3,}:?$/.test(cell));
}

function rgfNumber(id = "") {
  const match = String(id || "").match(/^RGF-(\d+)$/);
  return match ? Number(match[1]) : null;
}

function extractHeading(line = "") {
  const match = String(line || "").match(/^(#{1,6})\s+(.+?)\s*$/);
  if (!match) return null;
  return { level: match[1].length, title: match[2].trim() };
}

function headingStackForLine(lines, index) {
  const stack = [];
  for (let i = 0; i <= index; i += 1) {
    const heading = extractHeading(lines[i]);
    if (!heading) continue;
    while (stack.length > 0 && stack.at(-1).level >= heading.level) stack.pop();
    stack.push(heading);
  }
  return stack.map((entry) => entry.title).join(" > ");
}

export function parseRepoGovernanceBoard(boardText = "") {
  const lines = String(boardText || "").split(/\r?\n/);
  const rows = [];
  const pathReferences = [];
  const statusLine = lines.find((line) => /^\*\*Status:\*\*/i.test(line)) || "";

  for (let index = 0; index < lines.length; index += 1) {
    const line = lines[index];
    for (const match of line.matchAll(/`([^`]+)`/g)) {
      const raw = normalizePathText(match[1]);
      if (/^\.GOV\/.+\.md$/i.test(raw)) {
        pathReferences.push({ path: raw, line: index + 1 });
      }
    }

    const cells = splitMarkdownTableRow(line);
    if (cells.length < 6 || tableSeparator(cells)) continue;
    const id = stripCode(cells[0]);
    if (!/^RG[FR]-\d+$/.test(id)) continue;
    const status = normalizeStatus(stripCode(cells[1]));
    const isRgf = /^RGF-/.test(id);
    rows.push({
      id,
      status,
      workstream: stripCode(cells[2]),
      dependsOn: stripCode(cells[3]),
      evidence: isRgf ? stripCode(cells[4]) : "",
      primarySurfaces: isRgf ? stripCode(cells[5]) : stripCode(cells[4]),
      exitSignal: isRgf ? stripCode(cells[6] || "") : stripCode(cells[5] || ""),
      line: index + 1,
      section: headingStackForLine(lines, index),
    });
  }

  const sectionSequences = [];
  let activeSequence = null;
  for (let index = 0; index < lines.length; index += 1) {
    const heading = extractHeading(lines[index]);
    if (heading && /sequence/i.test(heading.title)) {
      activeSequence = { title: heading.title, line: index + 1, ids: [] };
      sectionSequences.push(activeSequence);
      continue;
    }
    if (heading && activeSequence && heading.level <= 2) {
      activeSequence = null;
    }
    if (!activeSequence) continue;
    const itemMatch = lines[index].match(/^\s*\d+\.\s+`?(RGF-\d+)`?/);
    if (itemMatch) {
      activeSequence.ids.push({ id: itemMatch[1], line: index + 1 });
    }
  }

  return {
    rows,
    statusLine,
    pathReferences,
    sectionSequences,
    rowsById: new Map(rows.map((row) => [row.id, row])),
  };
}

function expectedStatusFromSummaryText(text = "") {
  if (IMPLEMENTED_RE.test(text) && !/planned/i.test(text)) return "DONE";
  if (PLANNED_RE.test(text)) return "PLANNED";
  if (QUEUED_RE.test(text)) return "QUEUED";
  return "";
}

function expandSummaryRanges(statusLine = "") {
  const ranges = [];
  const rangeRe = /`RGF-(\d+)`\s+through\s+`RGF-(\d+)`([^.,;]*)/gi;
  for (const match of statusLine.matchAll(rangeRe)) {
    const start = Number(match[1]);
    const end = Number(match[2]);
    const expectedStatus = expectedStatusFromSummaryText(match[0]);
    if (!expectedStatus) continue;
    ranges.push({ start, end, expectedStatus, text: match[0] });
  }
  const singleRe = /`(RGF-\d+)`([^.,;]*)/gi;
  for (const match of statusLine.matchAll(singleRe)) {
    if (/through/i.test(match[0])) continue;
    const expectedStatus = expectedStatusFromSummaryText(match[0]);
    if (!expectedStatus) continue;
    const num = rgfNumber(match[1]);
    ranges.push({ start: num, end: num, expectedStatus, text: match[0] });
  }
  return ranges;
}

function extractGuideRows(guideText = "") {
  return parseRepoGovernanceBoard(guideText).rows.filter((row) => /^RGF-/.test(row.id));
}

function proposedSequenceAllowsStatus(sectionTitle = "", status = "") {
  const normalizedTitle = String(sectionTitle || "").trim().toUpperCase();
  const normalizedStatus = normalizeStatus(status);
  if (/PROPOSED/.test(normalizedTitle)) {
    return !["DONE", "HOLD"].includes(normalizedStatus);
  }
  return true;
}

function changelogMentions(changelogText = "", id = "") {
  return new RegExp(`\\b${String(id || "").replace("-", "\\-")}\\b`).test(changelogText);
}

function referenceExists(repoRoot, referencePath) {
  const normalized = normalizePathText(referencePath);
  if (!normalized) return true;
  return fs.existsSync(path.resolve(repoRoot, normalized));
}

export function validateRepoGovernanceBoard({
  repoRoot = process.cwd(),
  boardText = "",
  changelogText = "",
  guideText = "",
} = {}) {
  const parsed = parseRepoGovernanceBoard(boardText);
  const errors = [];
  const warnings = [];
  const rowsById = parsed.rowsById;
  const seen = new Map();

  for (const row of parsed.rows) {
    if (seen.has(row.id)) {
      errors.push(`${REPO_GOVERNANCE_BOARD_PATH}:${row.line}: duplicate row id ${row.id}; first seen at line ${seen.get(row.id)}`);
    } else {
      seen.set(row.id, row.line);
    }
    if (row.id.startsWith("RGF-") && !STATUS_VALUES.has(row.status)) {
      errors.push(`${REPO_GOVERNANCE_BOARD_PATH}:${row.line}: ${row.id} has invalid status ${row.status || "<missing>"}`);
    }
    const dependencies = row.dependsOn
      .split(",")
      .map((value) => stripCode(value))
      .filter((value) => value && value !== "-");
    for (const dependency of dependencies) {
      if (/^RGF-\d+$/.test(dependency) && !rowsById.has(dependency)) {
        const rowNumber = rgfNumber(row.id);
        if (rowNumber !== null && rowNumber < 198) {
          warnings.push(`${REPO_GOVERNANCE_BOARD_PATH}:${row.line}: legacy ${row.id} depends on unknown ${dependency}`);
        } else {
          errors.push(`${REPO_GOVERNANCE_BOARD_PATH}:${row.line}: ${row.id} depends on unknown ${dependency}`);
        }
      }
    }
    if (row.status === "DONE") {
      const hasEvidence = Boolean(row.evidence && row.evidence !== "-")
        || Boolean(row.primarySurfaces && row.primarySurfaces !== "-")
        || changelogMentions(changelogText, row.id);
      if (!hasEvidence) {
        errors.push(`${REPO_GOVERNANCE_BOARD_PATH}:${row.line}: ${row.id} is DONE without evidence, primary surfaces, or changelog mention`);
      }
    }
  }

  for (const range of expandSummaryRanges(parsed.statusLine)) {
    for (let idNumber = range.start; idNumber <= range.end; idNumber += 1) {
      const id = `RGF-${idNumber}`;
      const row = rowsById.get(id);
      if (!row) {
        errors.push(`${REPO_GOVERNANCE_BOARD_PATH}: status summary references missing ${id}`);
        continue;
      }
      if (row.status !== range.expectedStatus) {
        errors.push(`${REPO_GOVERNANCE_BOARD_PATH}:${row.line}: status summary says ${id} is ${range.expectedStatus}, row is ${row.status}`);
      }
    }
  }

  for (const ref of parsed.pathReferences) {
    if (!referenceExists(repoRoot, ref.path)) {
      errors.push(`${REPO_GOVERNANCE_BOARD_PATH}:${ref.line}: referenced file does not exist: ${ref.path}`);
    }
  }

  for (const sequence of parsed.sectionSequences) {
    const seenInSequence = new Set();
    for (const item of sequence.ids) {
      const row = rowsById.get(item.id);
      if (!row) {
        errors.push(`${REPO_GOVERNANCE_BOARD_PATH}:${item.line}: ${sequence.title} references unknown ${item.id}`);
        continue;
      }
      if (seenInSequence.has(item.id)) {
        errors.push(`${REPO_GOVERNANCE_BOARD_PATH}:${item.line}: ${sequence.title} repeats ${item.id}`);
      }
      seenInSequence.add(item.id);
      if (!proposedSequenceAllowsStatus(sequence.title, row.status)) {
        errors.push(`${REPO_GOVERNANCE_BOARD_PATH}:${item.line}: ${sequence.title} includes ${item.id} with row status ${row.status}`);
      }
    }
  }

  if (guideText) {
    const guideRows = extractGuideRows(guideText)
      .filter((row) => {
        const number = rgfNumber(row.id);
        return number !== null && number >= 255 && number <= 264;
      });
    for (const guideRow of guideRows) {
      const boardRow = rowsById.get(guideRow.id);
      if (!boardRow) {
        errors.push(`${WP1_POSTMORTEM_GUIDE_PATH}:${guideRow.line}: mirrored guide row ${guideRow.id} is absent from board`);
        continue;
      }
      for (const field of ["status", "workstream", "dependsOn", "primarySurfaces", "exitSignal"]) {
        if (String(boardRow[field] || "").trim() !== String(guideRow[field] || "").trim()) {
          errors.push(`${WP1_POSTMORTEM_GUIDE_PATH}:${guideRow.line}: mirrored ${guideRow.id} ${field} disagrees with board`);
        }
      }
    }
  } else {
    warnings.push(`${WP1_POSTMORTEM_GUIDE_PATH} not supplied; mirrored RGF-255..264 table was not checked`);
  }

  return {
    ok: errors.length === 0,
    errors,
    warnings,
    summary: {
      row_count: parsed.rows.length,
      rgf_count: parsed.rows.filter((row) => row.id.startsWith("RGF-")).length,
      done_count: parsed.rows.filter((row) => row.status === "DONE").length,
      planned_count: parsed.rows.filter((row) => row.status === "PLANNED").length,
      sequence_count: parsed.sectionSequences.length,
    },
    parsed,
  };
}
