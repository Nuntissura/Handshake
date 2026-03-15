import { validatePacketClosureMonitoring } from "./packet-closure-monitor-lib.mjs";

function formatList(items, { indent = "  - ", none = "NONE" } = {}) {
  const normalized = (items || []).map((item) => String(item || "").trim()).filter(Boolean);
  if (normalized.length === 0) return `${indent}${none}`;
  return normalized.map((item) => `${indent}${item}`).join("\n");
}

function uniqueOrdered(items) {
  const seen = new Set();
  const result = [];
  for (const item of items || []) {
    const normalized = String(item || "").trim();
    if (!normalized || seen.has(normalized)) continue;
    seen.add(normalized);
    result.push(normalized);
  }
  return result;
}

function normalizeInlineBlock(value, fallback = "See TEST_PLAN") {
  const collapsed = String(value || "")
    .replace(/\r/g, "")
    .split("\n")
    .map((line) => line.trim())
    .filter((line) => line && !line.startsWith("#"))
    .join("; ");
  return collapsed || fallback;
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

function parseSingleField(text, label) {
  const re = new RegExp(`^\\s*-\\s*(?:\\*\\*)?${label}(?:\\*\\*)?\\s*:\\s*(.+)\\s*$`, "mi");
  const match = String(text || "").match(re);
  return match ? match[1].trim() : "";
}

function normalizeEvidenceItems(items) {
  return uniqueOrdered(items).filter((item) => !/^NONE$/i.test(item));
}

export function deriveSemanticProofAssets({
  semanticTripwireTests = [],
  canonicalContractExamples = [],
  testPlan = "",
  doneMeans = [],
  specAnchors = [],
} = {}) {
  const normalizedTripwires = uniqueOrdered(semanticTripwireTests);
  const normalizedExamples = uniqueOrdered(canonicalContractExamples);

  if (normalizedTripwires.length > 0 || normalizedExamples.length > 0) {
    return {
      semanticTripwireTests: normalizedTripwires,
      canonicalContractExamples: normalizedExamples,
    };
  }

  const anchorSummary = uniqueOrdered((specAnchors || []).map((anchor) => String(anchor?.specAnchor || "").trim()));
  const fallbackTripwire = normalizeInlineBlock(
    testPlan,
    uniqueOrdered(doneMeans).join("; ") || anchorSummary.join("; ") || "Legacy bridge: inspect signed refinement and packet TEST_PLAN",
  );

  return {
    semanticTripwireTests: [`${fallbackTripwire} [LEGACY_REFINEMENT_BRIDGE]`],
    canonicalContractExamples: [],
  };
}

export function formatSemanticProofAssetsSection({
  semanticTripwireTests = [],
  canonicalContractExamples = [],
} = {}) {
  return `## SEMANTIC_PROOF_ASSETS (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- SEMANTIC_TRIPWIRE_TESTS:
${formatList(semanticTripwireTests)}
- CANONICAL_CONTRACT_EXAMPLES:
${formatList(canonicalContractExamples)}
- Rule: for packets using \`SEMANTIC_PROOF_PROFILE=DIFF_SCOPED_SEMANTIC_V1\`, each clause row must point to TESTS, EXAMPLES, or governed debt, and shared-surface packets should carry at least one concrete tripwire or canonical example.`;
}

export function validateSemanticProofAssets(packetText, {
  requireSharedSurfaceCoverage = true,
} = {}) {
  const errors = [];
  const section = extractSectionAfterHeading(packetText, "SEMANTIC_PROOF_ASSETS");
  if (!section) {
    return { errors: ["SEMANTIC_PROOF_ASSETS section missing"], parsed: null };
  }

  const semanticTripwireTests = normalizeEvidenceItems(extractListItemsAfterLabel(section, "SEMANTIC_TRIPWIRE_TESTS"));
  const canonicalContractExamples = normalizeEvidenceItems(extractListItemsAfterLabel(section, "CANONICAL_CONTRACT_EXAMPLES"));
  const closure = validatePacketClosureMonitoring(packetText, { requireRows: true });
  for (const error of closure.errors) errors.push(error);

  if (semanticTripwireTests.length === 0 && canonicalContractExamples.length === 0) {
    errors.push("SEMANTIC_PROOF_ASSETS must list one or more semantic tripwire tests or canonical contract examples");
  }

  const sharedSurfaceSection = extractSectionAfterHeading(packetText, "SHARED_SURFACE_MONITORING");
  const sharedSurfaceRisk = parseSingleField(sharedSurfaceSection, "SHARED_SURFACE_RISK").toUpperCase();
  if (requireSharedSurfaceCoverage && sharedSurfaceRisk === "YES" && semanticTripwireTests.length === 0 && canonicalContractExamples.length === 0) {
    errors.push("Shared-surface packets must carry one or more semantic tripwire tests or canonical contract examples");
  }

  for (const row of closure.parsed?.clauseRows || []) {
    const hasTests = row.tests && !/^NONE$/i.test(row.tests);
    const hasExamples = row.examples && !/^NONE$/i.test(row.examples);
    const hasDebt = row.debtIds.length > 0;
    if (!hasTests && !hasExamples && !hasDebt) {
      errors.push(`CLAUSE_CLOSURE_MATRIX row must point to TESTS, EXAMPLES, or DEBT_IDS: ${row.clause}`);
    }
  }

  return {
    errors,
    parsed: {
      semanticTripwireTests,
      canonicalContractExamples,
      sharedSurfaceRisk,
      clauseRows: closure.parsed?.clauseRows || [],
    },
  };
}
