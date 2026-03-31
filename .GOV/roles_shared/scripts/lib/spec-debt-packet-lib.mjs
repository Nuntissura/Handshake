import fs from "node:fs";
import { repoPathAbs } from "./runtime-paths.mjs";
import { formatClauseClosureMatrixSection, formatSpecDebtStatusSection } from "./packet-closure-monitor-lib.mjs";

function parsePipeRecord(item) {
  const record = {};
  for (const part of String(item || "").split("|")) {
    const trimmed = part.trim();
    if (!trimmed) continue;
    const idx = trimmed.indexOf(":");
    if (idx === -1) continue;
    const key = trimmed.slice(0, idx).trim().toUpperCase().replace(/\s+/g, "_");
    const value = trimmed.slice(idx + 1).trim();
    record[key] = value;
  }
  return record;
}

function extractSectionAfterHeading(text, heading) {
  const lines = String(text || "").split(/\r?\n/);
  const headingRe = new RegExp(`^##\\s+${heading}\\b`, "i");
  const startIndex = lines.findIndex((line) => headingRe.test(line));
  if (startIndex === -1) return "";

  let endIndex = lines.length;
  for (let index = startIndex + 1; index < lines.length; index += 1) {
    if (/^##\s+\S/.test(lines[index])) {
      endIndex = index;
      break;
    }
  }
  return lines.slice(startIndex, endIndex).join("\n").trim();
}

function extractListItemsAfterLabel(sectionText, label) {
  const lines = String(sectionText || "").split(/\r?\n/);
  const labelRe = new RegExp(`^\\s*-\\s*(?:\\*\\*)?${label}(?:\\*\\*)?\\s*:\\s*$`, "i");
  const headingRe = /^#{1,6}\s+\S/;
  const nextLabelRe = /^\s*-\s*(?:\*\*)?[A-Z][A-Z0-9_ ()/.-]*(?:\*\*)?\s*:\s*$/;
  const labelIdx = lines.findIndex((line) => labelRe.test(line));
  if (labelIdx === -1) return [];

  const items = [];
  for (let index = labelIdx + 1; index < lines.length; index += 1) {
    const line = lines[index];
    if (headingRe.test(line)) break;
    if (nextLabelRe.test(line)) break;
    const match = line.match(/^\s{2,}-\s+(.+)\s*$/);
    if (match) items.push((match[1] || "").trim());
  }
  return items;
}

export function replaceSingleField(text, label, value) {
  const re = new RegExp(`^(\\s*-\\s*(?:\\*\\*)?${label}(?:\\*\\*)?\\s*:\\s*)(.+)\\s*$`, "mi");
  if (!re.test(text)) {
    throw new Error(`Missing single field in packet: ${label}`);
  }
  return text.replace(re, `$1${value}`);
}

export function replaceSection(text, heading, replacement) {
  const lines = String(text || "").split(/\r?\n/);
  const headingRe = new RegExp(`^##\\s+${heading}\\b`, "i");
  const startIndex = lines.findIndex((line) => headingRe.test(line));
  if (startIndex === -1) {
    throw new Error(`Missing packet section heading: ${heading}`);
  }

  let endIndex = lines.length;
  for (let index = startIndex + 1; index < lines.length; index += 1) {
    if (/^##\s+\S/.test(lines[index])) {
      endIndex = index;
      break;
    }
  }

  const replacementLines = String(replacement || "").replace(/\r/g, "").split("\n");
  return [
    ...lines.slice(0, startIndex),
    ...replacementLines,
    ...lines.slice(endIndex),
  ].join("\n");
}

export function parseClauseRows(packetText) {
  const section = extractSectionAfterHeading(packetText, "CLAUSE_CLOSURE_MATRIX");
  const rawRows = extractListItemsAfterLabel(section, "CLAUSE_ROWS");
  return rawRows
    .filter((item) => !/^NONE$/i.test(item || ""))
    .map((item) => {
      const record = parsePipeRecord(item);
      const debtIdsRaw = String(record.DEBT_IDS || "NONE").trim() || "NONE";
      return {
        clause: String(record.CLAUSE || "").trim(),
        codeSurfaces: String(record.CODE_SURFACES || "").trim(),
        tests: String(record.TESTS || "NONE").trim() || "NONE",
        examples: String(record.EXAMPLES || "NONE").trim() || "NONE",
        debtIdsRaw,
        debtIds: /^NONE$/i.test(debtIdsRaw) ? [] : debtIdsRaw.split(",").map((value) => value.trim()).filter(Boolean),
        coderStatus: String(record.CODER_STATUS || "UNPROVEN").trim().toUpperCase(),
        validatorStatus: String(record.VALIDATOR_STATUS || "PENDING").trim().toUpperCase(),
      };
    });
}

export function formatClauseRow(row) {
  const debtIds = (row.debtIds || []).map((item) => String(item || "").trim()).filter(Boolean);
  return `CLAUSE: ${row.clause} | CODE_SURFACES: ${row.codeSurfaces} | TESTS: ${row.tests || "NONE"} | EXAMPLES: ${row.examples || "NONE"} | DEBT_IDS: ${debtIds.length > 0 ? debtIds.join(", ") : "NONE"} | CODER_STATUS: ${row.coderStatus || "UNPROVEN"} | VALIDATOR_STATUS: ${row.validatorStatus || "PENDING"}`;
}

export function formatUpdatedPacket(packetText, clauseRows, debtState) {
  if (!/##\s+CLAUSE_CLOSURE_MATRIX\b/i.test(packetText) || !/##\s+SPEC_DEBT_STATUS\b/i.test(packetText)) {
    throw new Error("Packet is not spec-debt managed yet; expected CLAUSE_CLOSURE_MATRIX and SPEC_DEBT_STATUS sections");
  }
  let nextText = replaceSection(packetText, "CLAUSE_CLOSURE_MATRIX", formatClauseClosureMatrixSection(clauseRows.map(formatClauseRow)));
  nextText = replaceSection(nextText, "SPEC_DEBT_STATUS", formatSpecDebtStatusSection(debtState));
  return nextText;
}

export function readPacket(packetPath) {
  const packetAbsPath = repoPathAbs(packetPath);
  if (!fs.existsSync(packetAbsPath)) {
    throw new Error(`Missing packet: ${packetPath.replace(/\\/g, "/")}`);
  }
  return fs.readFileSync(packetAbsPath, "utf8");
}

export function writePacket(packetPath, content) {
  fs.writeFileSync(repoPathAbs(packetPath), content, "utf8");
}
