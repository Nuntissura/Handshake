export const DATA_CONTRACT_PACKET_MIN_VERSION = "2026-04-01";
export const DATA_CONTRACT_PROFILE_VALUES = ["NONE", "LLM_FIRST_DATA_V1"];
export const DATA_CONTRACT_DECISION_VALUES = ["ACTIVE_REQUIRED", "WAIVED_NOT_DATA_BEARING"];
export const DATA_CONTRACT_SQL_POSTURE_VALUES = [
  "SQLITE_NOW_POSTGRES_READY",
  "BACKEND_NEUTRAL",
  "NOT_APPLICABLE",
];
export const DATA_CONTRACT_LLM_POSTURE_VALUES = ["REQUIRED", "NOT_APPLICABLE"];
export const DATA_CONTRACT_LOOM_POSTURE_VALUES = ["REQUIRED_WHEN_APPLICABLE", "NOT_APPLICABLE"];

function parseSingleField(text, label) {
  const re = new RegExp(`^\\s*-\\s*(?:\\*\\*)?${label}(?:\\*\\*)?\\s*:\\s*(.+)\\s*$`, "mi");
  const match = String(text || "").match(re);
  return match ? match[1].trim() : "";
}

function isPlaceholder(value) {
  const token = String(value || "").trim();
  if (!token) return true;
  if (/^\{.+\}$/.test(token)) return true;
  if (/^<fill/i.test(token)) return true;
  if (/^<pending>/i.test(token)) return true;
  if (/^<unclaimed>/i.test(token)) return true;
  return false;
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
  return lines.slice(startIndex + 1, endIndex).join("\n");
}

function extractListItemsAfterLabel(sectionText, label) {
  const lines = String(sectionText || "").split(/\r?\n/);
  const labelRe = new RegExp(`^\\s*-\\s*${label}\\s*:\\s*$`, "i");
  const headingRe = /^#{1,6}\s+\S/;
  const nextLabelRe = /^\s*-\s*[A-Z][A-Z0-9_ ()/-]*\s*:\s*$/;
  const items = [];

  const labelIdx = lines.findIndex((line) => labelRe.test(line));
  if (labelIdx === -1) return items;

  for (let index = labelIdx + 1; index < lines.length; index += 1) {
    const line = lines[index];
    if (headingRe.test(line)) break;
    if (nextLabelRe.test(line)) break;
    const match = line.match(/^\s*-\s+(.+)\s*$/);
    if (!match) continue;
    const value = (match[1] || "").trim();
    if (!isPlaceholder(value)) items.push(value);
  }

  return items;
}

function normalizeValue(value, allowedValues = []) {
  const token = String(value || "").trim().toUpperCase();
  return allowedValues.includes(token) ? token : "";
}

function hasOnlyNoneList(items = []) {
  return items.length === 1 && String(items[0] || "").trim().toUpperCase() === "NONE";
}

function uniqueEvidence(items = []) {
  return Array.from(new Set(
    (items || [])
      .map((item) => String(item || "").trim())
      .filter(Boolean),
  ));
}

function normalizeRepoLikePath(value) {
  return String(value || "").trim().replace(/\\/g, "/").replace(/^\.?\//, "");
}

function collectDataContractScopeEvidence(inScopePaths = []) {
  const evidence = [];
  for (const rawPath of Array.isArray(inScopePaths) ? inScopePaths : []) {
    const normalized = normalizeRepoLikePath(rawPath);
    if (!normalized) continue;
    if (/(^|\/)migrations?(\/|$)|\.sql$/i.test(normalized)) {
      evidence.push(`IN_SCOPE_PATH: ${normalized} (migration/sql surface)`);
      continue;
    }
    if (!/^src\/backend\//i.test(normalized)) continue;
    if (/(^|\/)(storage|locus)(\/|$)|role_mailbox|schema|models?\.rs$|types?\.rs$|registry|payload|dto|serialize|deserial/i.test(normalized)) {
      evidence.push(`IN_SCOPE_PATH: ${normalized} (backend data surface)`);
    }
  }
  return uniqueEvidence(evidence);
}

function collectStructuredDataContractEvidence(refinementData = null) {
  const evidence = [];
  const structuredSignals = [
    ...(Array.isArray(refinementData?.pillarsTouched) ? refinementData.pillarsTouched.map((value) => `PILLARS_TOUCHED: ${value}`) : []),
    ...(Array.isArray(refinementData?.pillarDecompositionRows) ? refinementData.pillarDecompositionRows.map((value) => `PILLAR_DECOMPOSITION: ${value}`) : []),
    ...(Array.isArray(refinementData?.executionRuntimeAlignmentRows) ? refinementData.executionRuntimeAlignmentRows.map((value) => `EXECUTION_RUNTIME_ALIGNMENT: ${value}`) : []),
    ...(Array.isArray(refinementData?.forceMultiplierResolutions) ? refinementData.forceMultiplierResolutions.map((value) => `FORCE_MULTIPLIER_EXPANSION: ${value}`) : []),
    ...(Array.isArray(refinementData?.codeRealitySummary) ? refinementData.codeRealitySummary.map((value) => `CODE_REALITY_EVIDENCE: ${value}`) : []),
  ];
  for (const signal of structuredSignals) {
    if (/(LLM[- ]friendly data|LLM[- ]first|SQL to PostgreSQL shift readiness|SQLite-now\s*\/\s*PostgreSQL-ready|postgresql-ready|Loom|Locus|persist(?:ed|ence)?|schema|machine-readable|provenance|stable ids?|relations?)/i.test(signal)) {
      evidence.push(signal);
    }
  }
  return uniqueEvidence(evidence);
}

export function packetUsesDataContractProfile(packetFormatVersion) {
  const version = String(packetFormatVersion || "").trim();
  return version >= DATA_CONTRACT_PACKET_MIN_VERSION;
}

export function normalizeDataContractProfile(value) {
  return normalizeValue(value, DATA_CONTRACT_PROFILE_VALUES);
}

export function parseDataContractProfile(packetContent = "") {
  return normalizeDataContractProfile(parseSingleField(packetContent, "DATA_CONTRACT_PROFILE"));
}

export function parseDataContractDecision(packetContent = "") {
  const raw = extractSectionAfterHeading(packetContent, "DATA_CONTRACT_DECISION");
  return {
    raw,
    decision: normalizeValue(parseSingleField(raw, "DECISION"), DATA_CONTRACT_DECISION_VALUES),
    reason: parseSingleField(raw, "REASON"),
    evidence: extractListItemsAfterLabel(raw, "EVIDENCE"),
  };
}

export function parseDataContractMonitoring(packetContent = "") {
  const raw = extractSectionAfterHeading(packetContent, "DATA_CONTRACT_MONITORING");
  return {
    raw,
    active: String(parseSingleField(raw, "DATA_CONTRACT_ACTIVE") || "").trim().toUpperCase(),
    sqlPosture: normalizeValue(parseSingleField(raw, "SQL_POSTURE"), DATA_CONTRACT_SQL_POSTURE_VALUES),
    llmReadabilityPosture: normalizeValue(parseSingleField(raw, "LLM_READABILITY_POSTURE"), DATA_CONTRACT_LLM_POSTURE_VALUES),
    loomIntertwinedPosture: normalizeValue(parseSingleField(raw, "LOOM_INTERTWINED_POSTURE"), DATA_CONTRACT_LOOM_POSTURE_VALUES),
    primaryDataSurfaces: extractListItemsAfterLabel(raw, "PRIMARY_DATA_SURFACES"),
    dataContractRules: extractListItemsAfterLabel(raw, "DATA_CONTRACT_RULES"),
    validatorDataProofHints: extractListItemsAfterLabel(raw, "VALIDATOR_DATA_PROOF_HINTS"),
  };
}

export function validateDataContractSection(packetContent = "", { packetPath = "" } = {}) {
  const profile = parseDataContractProfile(packetContent);
  const monitoring = parseDataContractMonitoring(packetContent);
  const errors = [];

  if (profile === "NONE") {
    return { profile, monitoring, errors };
  }

  if (profile !== "LLM_FIRST_DATA_V1") {
    errors.push(`${packetPath || "<packet>"}: DATA_CONTRACT_PROFILE must be NONE | LLM_FIRST_DATA_V1`);
    return { profile, monitoring, errors };
  }

  if (!String(monitoring.raw || "").trim()) {
    errors.push(`${packetPath || "<packet>"}: DATA_CONTRACT_MONITORING section missing for DATA_CONTRACT_PROFILE=LLM_FIRST_DATA_V1`);
    return { profile, monitoring, errors };
  }

  if (monitoring.active !== "YES") {
    errors.push(`${packetPath || "<packet>"}: DATA_CONTRACT_ACTIVE must be YES for DATA_CONTRACT_PROFILE=LLM_FIRST_DATA_V1`);
  }
  if (!monitoring.sqlPosture || monitoring.sqlPosture === "NOT_APPLICABLE") {
    errors.push(`${packetPath || "<packet>"}: SQL_POSTURE must be SQLITE_NOW_POSTGRES_READY or BACKEND_NEUTRAL for active data contract packets`);
  }
  if (monitoring.llmReadabilityPosture !== "REQUIRED") {
    errors.push(`${packetPath || "<packet>"}: LLM_READABILITY_POSTURE must be REQUIRED for active data contract packets`);
  }
  if (monitoring.loomIntertwinedPosture !== "REQUIRED_WHEN_APPLICABLE") {
    errors.push(`${packetPath || "<packet>"}: LOOM_INTERTWINED_POSTURE must be REQUIRED_WHEN_APPLICABLE for active data contract packets`);
  }
  if (monitoring.primaryDataSurfaces.length === 0 || hasOnlyNoneList(monitoring.primaryDataSurfaces)) {
    errors.push(`${packetPath || "<packet>"}: PRIMARY_DATA_SURFACES must list concrete packet data surfaces for active data contract packets`);
  }
  if (monitoring.dataContractRules.length === 0 || hasOnlyNoneList(monitoring.dataContractRules)) {
    errors.push(`${packetPath || "<packet>"}: DATA_CONTRACT_RULES must list concrete rules for active data contract packets`);
  }
  if (monitoring.validatorDataProofHints.length === 0 || hasOnlyNoneList(monitoring.validatorDataProofHints)) {
    errors.push(`${packetPath || "<packet>"}: VALIDATOR_DATA_PROOF_HINTS must list concrete proof expectations for active data contract packets`);
  }

  return { profile, monitoring, errors };
}

export function validateDataContractDecisionSection(packetContent = "", {
  packetPath = "",
  inScopePaths = [],
} = {}) {
  const profile = parseDataContractProfile(packetContent);
  const decision = parseDataContractDecision(packetContent);
  const errors = [];

  if (!String(decision.raw || "").trim()) {
    errors.push(`${packetPath || "<packet>"}: DATA_CONTRACT_DECISION section missing for PACKET_FORMAT_VERSION >= 2026-04-01`);
    return { profile, decision, errors };
  }
  if (!decision.decision) {
    errors.push(`${packetPath || "<packet>"}: DATA_CONTRACT_DECISION DECISION must be ${DATA_CONTRACT_DECISION_VALUES.join(" | ")}`);
  }
  if (isPlaceholder(decision.reason)) {
    errors.push(`${packetPath || "<packet>"}: DATA_CONTRACT_DECISION REASON must be explicit for PACKET_FORMAT_VERSION >= 2026-04-01`);
  }
  if (decision.evidence.length === 0 || hasOnlyNoneList(decision.evidence)) {
    errors.push(`${packetPath || "<packet>"}: DATA_CONTRACT_DECISION EVIDENCE must list reviewable justification for the activation or waiver`);
  }

  if (decision.decision === "ACTIVE_REQUIRED" && profile !== "LLM_FIRST_DATA_V1") {
    errors.push(`${packetPath || "<packet>"}: DATA_CONTRACT_DECISION=ACTIVE_REQUIRED requires DATA_CONTRACT_PROFILE=LLM_FIRST_DATA_V1`);
  }
  if (decision.decision === "WAIVED_NOT_DATA_BEARING" && profile !== "NONE") {
    errors.push(`${packetPath || "<packet>"}: DATA_CONTRACT_DECISION=WAIVED_NOT_DATA_BEARING requires DATA_CONTRACT_PROFILE=NONE`);
  }

  const scopeEvidence = collectDataContractScopeEvidence(inScopePaths);
  if (decision.decision === "WAIVED_NOT_DATA_BEARING" && scopeEvidence.length > 0) {
    errors.push(`${packetPath || "<packet>"}: DATA_CONTRACT_DECISION=WAIVED_NOT_DATA_BEARING conflicts with data-bearing IN_SCOPE_PATHS (${scopeEvidence.join("; ")})`);
  }

  return { profile, decision, errors };
}

export function formatDataContractMonitoringSection({ profile = "NONE", inScopePaths = [] } = {}) {
  const normalizedProfile = normalizeDataContractProfile(profile) || "NONE";
  const primaryDataSurfaces = Array.isArray(inScopePaths)
    ? inScopePaths.map((entry) => String(entry || "").trim()).filter(Boolean)
    : [];
  const formattedSurfaces = primaryDataSurfaces.length > 0
    ? primaryDataSurfaces.map((entry) => `  - ${entry}`).join("\n")
    : "  - NONE";

  if (normalizedProfile !== "LLM_FIRST_DATA_V1") {
    return `
## DATA_CONTRACT_MONITORING (AUTHORITATIVE SNAPSHOT; MUTABLE)
- DATA_CONTRACT_ACTIVE: NO
- SQL_POSTURE: NOT_APPLICABLE
- LLM_READABILITY_POSTURE: NOT_APPLICABLE
- LOOM_INTERTWINED_POSTURE: NOT_APPLICABLE
- PRIMARY_DATA_SURFACES:
  - NONE
- DATA_CONTRACT_RULES:
  - NONE
- VALIDATOR_DATA_PROOF_HINTS:
  - NONE
`;
  }

  return `
## DATA_CONTRACT_MONITORING (AUTHORITATIVE SNAPSHOT; MUTABLE)
- DATA_CONTRACT_ACTIVE: YES
- SQL_POSTURE: SQLITE_NOW_POSTGRES_READY
- LLM_READABILITY_POSTURE: REQUIRED
- LOOM_INTERTWINED_POSTURE: REQUIRED_WHEN_APPLICABLE
- PRIMARY_DATA_SURFACES:
${formattedSurfaces}
- DATA_CONTRACT_RULES:
  - Keep persisted and emitted structure SQL-backed and PostgreSQL-ready; do not introduce fresh SQLite-only semantics unless the packet or spec explicitly requires them.
  - Prefer explicit machine-readable fields, enums, ids, relations, and provenance over presentation-only strings, overloaded text blobs, or parser-only implied meaning.
  - Preserve stable ids, explicit relations, backlink-friendly fields, provenance anchors, and retrieval-friendly summaries so Loom and graph/search consumers can traverse the data without reparsing UI text.
- VALIDATOR_DATA_PROOF_HINTS:
  - Prove the touched data surfaces remain SQLite-now and PostgreSQL-ready or justify any backend-specific semantics explicitly.
  - Prove the emitted or persisted shapes stay LLM-first readable and parseable with stable field names and explicit structured values.
  - Prove Loom-facing ids, relations, provenance anchors, and retrieval fields remain explicit where the packet touches them.
`;
}

export function formatDataContractDecisionSection({
  decision = "WAIVED_NOT_DATA_BEARING",
  reason = "No concrete data-bearing or structure-bearing surface was identified in current packet scope.",
  evidence = [],
} = {}) {
  const normalizedDecision = normalizeValue(decision, DATA_CONTRACT_DECISION_VALUES) || "WAIVED_NOT_DATA_BEARING";
  const normalizedEvidence = uniqueEvidence(evidence);
  const formattedEvidence = normalizedEvidence.length > 0
    ? normalizedEvidence.map((entry) => `  - ${entry}`).join("\n")
    : "  - NONE";

  return `
## DATA_CONTRACT_DECISION (AUTHORITATIVE SNAPSHOT; MUTABLE)
- DECISION: ${normalizedDecision}
- REASON: ${reason}
- EVIDENCE:
${formattedEvidence}
`;
}

export function deriveDataContractDecisionFromRefinement({
  refinementData = null,
  refinementText = "",
  inScopePaths = [],
} = {}) {
  const scopeEvidence = collectDataContractScopeEvidence(inScopePaths);
  const structuredEvidence = collectStructuredDataContractEvidence(refinementData);
  const evidence = uniqueEvidence([...scopeEvidence, ...structuredEvidence]);

  if (evidence.length > 0) {
    return {
      profile: "LLM_FIRST_DATA_V1",
      decision: "ACTIVE_REQUIRED",
      reason: "Current packet scope includes concrete persisted, emitted, or Loom-facing data surfaces, so the data contract is explicitly active.",
      evidence,
    };
  }

  return {
    profile: "NONE",
    decision: "WAIVED_NOT_DATA_BEARING",
    reason: "No concrete data-bearing or structure-bearing surface was identified in current packet scope, so the data contract is explicitly waived unless scope changes later.",
    evidence: uniqueEvidence([
      Array.isArray(inScopePaths) && inScopePaths.length > 0
        ? `IN_SCOPE_PATHS reviewed: ${inScopePaths.map((entry) => normalizeRepoLikePath(entry)).filter(Boolean).join(", ")}`
        : "IN_SCOPE_PATHS reviewed: NONE",
      "No structured refinement rows explicitly marked LLM-friendly data, SQL-to-PostgreSQL readiness, Loom-facing data, or persisted/emitted schema surfaces.",
      refinementText ? "Refinement text was reviewed, but no packet-scope data-contract trigger was elevated without structured or scope evidence." : "",
    ]),
  };
}

export function deriveDataContractProfileFromRefinement({
  refinementData = null,
  inScopePaths = [],
  refinementText = "",
} = {}) {
  return deriveDataContractDecisionFromRefinement({
    refinementData,
    inScopePaths,
    refinementText,
  }).profile;
}
