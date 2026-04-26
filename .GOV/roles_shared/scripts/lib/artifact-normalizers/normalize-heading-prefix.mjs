import { resultFor, transformOutsideFences } from "./text-utils.mjs";

const KNOWN_BARE_FIELD_LABELS = new Set([
  "VERDICT",
  "VALIDATION_CONTEXT",
  "GOVERNANCE_VERDICT",
  "TEST_VERDICT",
  "CODE_REVIEW_VERDICT",
  "SPEC_ALIGNMENT_VERDICT",
  "HEURISTIC_REVIEW_VERDICT",
  "SPEC_RETENTION_TRACK_VERDICT",
  "MECHANICAL_TRACK_VERDICT",
  "ENVIRONMENT_VERDICT",
  "LEGAL_VERDICT",
  "CLAUSES_REVIEWED",
  "SPEC_CLAUSE_MAP",
  "NEGATIVE_PROOF",
  "DATA_CONTRACT_PROOF",
  "PRIMITIVE_RETENTION_PROOF",
  "SHARED_SURFACE_INTERACTION_CHECKS",
  "CURRENT_MAIN_INTERACTION_CHECKS",
]);

export function normalizeHeadingPrefix(input = "") {
  const output = transformOutsideFences(input, (line) => {
    const match = line.match(/^\s*#{1,6}\s+([A-Za-z][A-Za-z0-9_ ()/-]*)\s*:\s*(.*)$/u);
    if (!match) return line;
    const label = String(match[1] || "").trim();
    if (!KNOWN_BARE_FIELD_LABELS.has(label.toUpperCase())) return line;
    return `${label}: ${match[2] || ""}`;
  });
  return resultFor("normalizeHeadingPrefix", input, output, "removed heading prefix from bare key-value fields");
}
