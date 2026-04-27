import { resultFor, transformOutsideFences } from "./text-utils.mjs";

const TOP_LEVEL_HEADINGS = new Set([
  "METADATA",
  "BOOTSTRAP",
  "SKELETON",
  "IMPLEMENTATION",
  "VALIDATION",
  "VALIDATION_REPORTS",
  "CLAUSE_CLOSURE_MATRIX",
  "HANDOFF",
  "CLOSEOUT",
  "EXECUTION",
]);

export function normalizeHeadingLevels(input = "") {
  const output = transformOutsideFences(input, (line) => {
    const match = line.match(/^(#{3,6})(\s+([A-Z][A-Z0-9_ -]*)\b.*)$/u);
    if (!match) return line;
    const headingLabel = String(match[3] || "").trim().toUpperCase();
    if (!TOP_LEVEL_HEADINGS.has(headingLabel)) return line;
    return `##${match[2]}`;
  });
  return resultFor("normalizeHeadingLevels", input, output, "normalized unambiguous top-level heading depth to ##");
}

