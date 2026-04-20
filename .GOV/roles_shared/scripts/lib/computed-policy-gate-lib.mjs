import {
  COMPLETION_LAYER_VERDICTS_MIN_VERSION,
  packetRequiresCompletionLayerVerdicts,
  packetRequiresSpecClauseMap,
  packetUsesStructuredValidationReport,
} from "../session/session-policy.mjs";
import { validateClauseReportConsistency, validatePacketClosureMonitoring } from "./packet-closure-monitor-lib.mjs";
import { validateSemanticProofAssets } from "./semantic-proof-lib.mjs";
import {
  validatorReportProfileRequiresDualTrack,
  validatorReportProfileRequiresPrimitiveAudit,
  validatorReportProfileRequiresRiskAudit,
} from "./validator-report-profile-lib.mjs";

export const COMPUTED_POLICY_OUTCOMES = ["PASS", "FAIL", "REVIEW_REQUIRED", "WAIVED", "BLOCKED"];
export const POLICY_WAIVER_STATUS_VALUES = ["ACTIVE", "EXPIRED", "REVOKED", "CLOSED"];
export const POLICY_WAIVER_COVERAGE_VALUES = [
  "SCOPE",
  "PROOF",
  "INTEGRATION",
  "DOMAIN",
  "PROTECTED_SURFACE",
  "TEST",
  "ENVIRONMENT",
  "HEURISTIC",
  "SPEC",
  "GOVERNANCE",
];

function parseSingleField(text, label) {
  const re = new RegExp(`^\\s*-\\s*(?:\\*\\*)?${label}(?:\\*\\*)?\\s*:\\s*(.+)\\s*$`, "mi");
  const match = String(text || "").match(re);
  return match ? match[1].trim() : "";
}

function parseStatus(text) {
  return (
    (String(text || "").match(/^\s*-\s*\*\*Status:\*\*\s*(.+)\s*$/mi) || [])[1]
    || (String(text || "").match(/^\s*\*\*Status:\*\*\s*(.+)\s*$/mi) || [])[1]
    || (String(text || "").match(/^\s*Status:\s*(.+)\s*$/mi) || [])[1]
    || ""
  ).trim();
}

function isClosedStatus(status) {
  return /\b(done|validated)\b/i.test(String(status || ""));
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

function parseSectionField(sectionText, label) {
  // RGF-90 + RGF-197: accept bullet, heading, or bare prefix.
  // Use first match — in append-only reports the authoritative Integration Validator
  // report comes before later WP Validator reports that may have different verdicts.
  const re = new RegExp(`^(?:\\s*-\\s*|\\s*#{1,6}\\s+|\\s*)(?:\\*\\*)?${label}(?:\\*\\*)?\\s*:\\s*(.+)\\s*$`, "im");
  const match = String(sectionText || "").match(re);
  return match ? match[1].trim() : "";
}

function extractListItemsAfterLabel(sectionText, label) {
  const lines = String(sectionText || "").split(/\r?\n/);
  // RGF-90 + RGF-197: accept heading prefix; use first match to stay consistent
  // with parseSectionField (both read the authoritative report, not a later WP Validator report).
  const labelRe = new RegExp(`^(?:\\s*#{1,6}\\s+|\\s*-\\s*|\\s*)(?:\\*\\*)?${label}(?:\\*\\*)?\\s*:\\s*$`, "i");
  const headingRe = /^#{1,6}\s+\S/;
  const nextLabelRe = /^(?:\s*-\s*|\s*)[A-Z][A-Z0-9_ ()/-]*\s*:\s*$/;

  const labelIdx = lines.findIndex((line) => labelRe.test(line));
  if (labelIdx === -1) return [];

  const items = [];
  for (let index = labelIdx + 1; index < lines.length; index += 1) {
    const line = lines[index];
    if (headingRe.test(line)) break;
    if (nextLabelRe.test(line)) break;
    const match = line.match(/^\s*-\s+(.+)\s*$/);
    if (match) items.push((match[1] || "").trim());
  }
  return items;
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

function normalizeNoneFiltered(items) {
  return uniqueOrdered(items).filter((item) => !/^NONE$/i.test(item));
}

function hasOnlyNoneList(items) {
  return items.length === 1 && String(items[0] || "").trim().toUpperCase() === "NONE";
}

function riskTierRank(value) {
  const normalized = String(value || "").trim().toUpperCase();
  if (normalized === "LOW") return 1;
  if (normalized === "MEDIUM") return 2;
  if (normalized === "HIGH") return 3;
  return 0;
}

function lacksConcreteListEvidence(items = []) {
  // RGF-197: majority-based — require at least half the substantive items to have concrete refs.
  const substantive = items.filter((item) => !/^NONE$/i.test(String(item || "").trim()));
  if (substantive.length === 0) return false;
  const withEvidence = substantive.filter((item) => hasConcreteCodeReference(item));
  return withEvidence.length < Math.ceil(substantive.length / 2);
}

function hasConcreteCodeReference(value) {
  const text = String(value || "").trim();
  if (!text) return false;
  return (
    /`[^`]+`/.test(text) ||
    /\b[\w./-]+\.(?:rs|ts|tsx|js|jsx|mjs|cjs|py|go|java|cs|cpp|c|h|hpp|json|ya?ml|toml|sql)(?::\d+)?\b/i.test(text) ||
    /\b[A-Za-z_][A-Za-z0-9_]*::[A-Za-z_][A-Za-z0-9_]*\b/.test(text) ||
    /\b[A-Za-z_][A-Za-z0-9_]*\([^)]*\)/.test(text)
  );
}

function negativeProofLeaksToGovernance(value) {
  return /\.GOV\/|gov_runtime\/|TASK_BOARD|RUNTIME_STATUS|ROLE_SESSION_REGISTRY|SESSION_CONTROL|VALIDATOR_PROTOCOL|ORCHESTRATOR_PROTOCOL|COMMAND_SURFACE_REFERENCE|governance closeout|outside the signed product scope/i.test(String(value || ""));
}

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

function normalizeCoverageTokens(rawValue) {
  const tokens = String(rawValue || "")
    .split(",")
    .map((item) => item.trim().toUpperCase())
    .filter(Boolean);
  return uniqueOrdered(tokens).filter((token) => POLICY_WAIVER_COVERAGE_VALUES.includes(token));
}

function inferCoverageTokens(rawLine) {
  const coverage = [];
  const raw = String(rawLine || "");
  if (/CX-573F|dirty\s*tree|git\s*hygiene|scope/i.test(raw)) coverage.push("SCOPE");
  if (/proof|not[-\s]?proven|evidence/i.test(raw)) coverage.push("PROOF");
  if (/integration|merge/i.test(raw)) coverage.push("INTEGRATION");
  if (/domain|done means|goal/i.test(raw)) coverage.push("DOMAIN");
  if (/protected surface|tripwire|boundary|negative path/i.test(raw)) coverage.push("PROTECTED_SURFACE");
  if (/test/i.test(raw)) coverage.push("TEST");
  if (/environment|tooling|context mismatch/i.test(raw)) coverage.push("ENVIRONMENT");
  if (/heuristic|quality|maintainability/i.test(raw)) coverage.push("HEURISTIC");
  if (/spec/i.test(raw)) coverage.push("SPEC");
  if (/governance|workflow/i.test(raw)) coverage.push("GOVERNANCE");
  return uniqueOrdered(coverage);
}

export function parsePolicyWaiverLedger(packetText) {
  const section = extractSectionAfterHeading(packetText, "WAIVERS GRANTED");
  const lines = String(section || "")
    .split(/\r?\n/)
    .map((line) => line.trim())
    .filter((line) => /^-\s+/.test(line));

  const entries = [];
  for (const line of lines) {
    const body = line.replace(/^\-\s+/, "").trim();
    if (!body || /^NONE$/i.test(body) || /^\(/.test(body)) continue;
    const record = parsePipeRecord(body);
    const waiverId = record.WAIVER_ID || record.ID || "";
    const status = String(record.STATUS || "ACTIVE").trim().toUpperCase();
    const coverage = normalizeCoverageTokens(record.COVERS || "").length > 0
      ? normalizeCoverageTokens(record.COVERS)
      : inferCoverageTokens(body);
    entries.push({
      waiverId: waiverId || body,
      status: POLICY_WAIVER_STATUS_VALUES.includes(status) ? status : "ACTIVE",
      coverage,
      scope: record.SCOPE || "",
      justification: record.JUSTIFICATION || body,
      approver: record.APPROVER || "",
      expires: record.EXPIRES || "",
      raw: body,
    });
  }

  const activeEntries = entries.filter((entry) => entry.status === "ACTIVE");
  const activeCoverageTokens = uniqueOrdered(activeEntries.flatMap((entry) => entry.coverage));

  return {
    raw: section,
    entries,
    activeEntries,
    activeCoverageTokens,
  };
}

function parseValidationReport(packetText) {
  const section = extractSectionAfterHeading(packetText, "VALIDATION_REPORTS");
  return {
    raw: section,
    verdict: parseSectionField(section, "Verdict").toUpperCase(),
    validationContext: parseSectionField(section, "VALIDATION_CONTEXT").toUpperCase(),
    governanceVerdict: parseSectionField(section, "GOVERNANCE_VERDICT").toUpperCase(),
    testVerdict: parseSectionField(section, "TEST_VERDICT").toUpperCase(),
    codeReviewVerdict: parseSectionField(section, "CODE_REVIEW_VERDICT").toUpperCase(),
    heuristicReviewVerdict: parseSectionField(section, "HEURISTIC_REVIEW_VERDICT").toUpperCase(),
    specAlignmentVerdict: parseSectionField(section, "SPEC_ALIGNMENT_VERDICT").toUpperCase(),
    environmentVerdict: parseSectionField(section, "ENVIRONMENT_VERDICT").toUpperCase(),
    disposition: parseSectionField(section, "DISPOSITION").toUpperCase(),
    legalVerdict: parseSectionField(section, "LEGAL_VERDICT").toUpperCase(),
    specConfidence: parseSectionField(section, "SPEC_CONFIDENCE").toUpperCase(),
    workflowValidity: parseSectionField(section, "WORKFLOW_VALIDITY").toUpperCase(),
    scopeValidity: parseSectionField(section, "SCOPE_VALIDITY").toUpperCase(),
    proofCompleteness: parseSectionField(section, "PROOF_COMPLETENESS").toUpperCase(),
    integrationReadiness: parseSectionField(section, "INTEGRATION_READINESS").toUpperCase(),
    domainGoalCompletion: parseSectionField(section, "DOMAIN_GOAL_COMPLETION").toUpperCase(),
    mechanicalTrackVerdict: parseSectionField(section, "MECHANICAL_TRACK_VERDICT").toUpperCase(),
    specRetentionTrackVerdict: parseSectionField(section, "SPEC_RETENTION_TRACK_VERDICT").toUpperCase(),
    validatorRiskTier: parseSectionField(section, "VALIDATOR_RISK_TIER").toUpperCase(),
    clausesReviewed: normalizeNoneFiltered(extractListItemsAfterLabel(section, "CLAUSES_REVIEWED")),
    notProven: normalizeNoneFiltered(extractListItemsAfterLabel(section, "NOT_PROVEN")),
    mainBodyGaps: normalizeNoneFiltered(extractListItemsAfterLabel(section, "MAIN_BODY_GAPS")),
    qualityRisks: normalizeNoneFiltered(extractListItemsAfterLabel(section, "QUALITY_RISKS")),
    diffAttackSurfaces: normalizeNoneFiltered(extractListItemsAfterLabel(section, "DIFF_ATTACK_SURFACES")),
    independentChecks: normalizeNoneFiltered(extractListItemsAfterLabel(section, "INDEPENDENT_CHECKS_RUN")),
    counterfactualChecks: normalizeNoneFiltered(extractListItemsAfterLabel(section, "COUNTERFACTUAL_CHECKS")),
    boundaryProbes: normalizeNoneFiltered(extractListItemsAfterLabel(section, "BOUNDARY_PROBES")),
    negativePathChecks: normalizeNoneFiltered(extractListItemsAfterLabel(section, "NEGATIVE_PATH_CHECKS")),
    independentFindings: normalizeNoneFiltered(extractListItemsAfterLabel(section, "INDEPENDENT_FINDINGS")),
    residualUncertainty: normalizeNoneFiltered(extractListItemsAfterLabel(section, "RESIDUAL_UNCERTAINTY")),
    specClauseMap: normalizeNoneFiltered(extractListItemsAfterLabel(section, "SPEC_CLAUSE_MAP")),
    negativeProof: normalizeNoneFiltered(extractListItemsAfterLabel(section, "NEGATIVE_PROOF")),
    primitiveRetentionProof: normalizeNoneFiltered(extractListItemsAfterLabel(section, "PRIMITIVE_RETENTION_PROOF")),
    primitiveRetentionGaps: normalizeNoneFiltered(extractListItemsAfterLabel(section, "PRIMITIVE_RETENTION_GAPS")),
    sharedSurfaceInteractionChecks: normalizeNoneFiltered(extractListItemsAfterLabel(section, "SHARED_SURFACE_INTERACTION_CHECKS")),
    currentMainInteractionChecks: normalizeNoneFiltered(extractListItemsAfterLabel(section, "CURRENT_MAIN_INTERACTION_CHECKS")),
    dataContractProof: normalizeNoneFiltered(extractListItemsAfterLabel(section, "DATA_CONTRACT_PROOF")),
    dataContractGaps: normalizeNoneFiltered(extractListItemsAfterLabel(section, "DATA_CONTRACT_GAPS")),
  };
}

function classifyProtectedSurfaceState(closureParsed, report) {
  if (!closureParsed || closureParsed.sharedSurfaceRisk !== "YES") {
    return "CLEAR";
  }
  const hasBoundaryEvidence = report.boundaryProbes.length > 0;
  const hasNegativePathEvidence = report.negativePathChecks.length > 0;
  if (hasBoundaryEvidence && hasNegativePathEvidence) return "REVIEWED";
  if (hasBoundaryEvidence || hasNegativePathEvidence || report.independentChecks.length > 0) return "PARTIAL";
  return "NOT_REVIEWED";
}

function buildWitnessMatrix(closureParsed, report) {
  return (closureParsed?.clauseRows || []).map((row) => ({
    clause: row.clause,
    coder_status: row.coderStatus,
    validator_status: row.validatorStatus,
    tests: row.tests,
    examples: row.examples,
    clause_reviewed: report.clausesReviewed.some((item) => item.includes(row.clause)),
    listed_not_proven: report.notProven.some((item) => item.includes(row.clause)),
  }));
}

function issue(code, severity, message, { coverage = null, waivable = false, source = "POLICY" } = {}) {
  return { code, severity, message, coverage, waivable, source };
}

function applyWaivers(issues, waiverLedger) {
  const activeCoverage = new Set(waiverLedger.activeCoverageTokens);
  const unwaived = [];
  const waived = [];

  for (const item of issues) {
    if (item.waivable && item.coverage && activeCoverage.has(item.coverage)) {
      waived.push({
        ...item,
        waived_by: waiverLedger.activeEntries
          .filter((entry) => entry.coverage.includes(item.coverage))
          .map((entry) => entry.waiverId),
      });
    } else {
      unwaived.push(item);
    }
  }

  return { unwaived, waived };
}

function addVerdictIssue(issues, value, passValue, {
  codePrefix,
  failValues = [],
  reviewValues = [],
  blockedValues = [],
  failCoverage = null,
  reviewCoverage = null,
  blockedCoverage = null,
  failWaivable = false,
  reviewWaivable = false,
  blockedWaivable = false,
  source = "REPORT",
}) {
  if (!value || value === passValue) return;
  if (failValues.includes(value)) {
    issues.push(issue(`${codePrefix}_${value}`, "FAIL", `${codePrefix}=${value}`, {
      coverage: failCoverage,
      waivable: failWaivable,
      source,
    }));
    return;
  }
  if (reviewValues.includes(value)) {
    issues.push(issue(`${codePrefix}_${value}`, "REVIEW_REQUIRED", `${codePrefix}=${value}`, {
      coverage: reviewCoverage,
      waivable: reviewWaivable,
      source,
    }));
    return;
  }
  if (blockedValues.includes(value)) {
    issues.push(issue(`${codePrefix}_${value}`, "BLOCKED", `${codePrefix}=${value}`, {
      coverage: blockedCoverage,
      waivable: blockedWaivable,
      source,
    }));
    return;
  }
  issues.push(issue(`${codePrefix}_${value}`, "BLOCKED", `${codePrefix}=${value}`, { source }));
}

export function evaluateComputedPolicyGateFromPacketText(packetText, {
  wpId = "",
  packetPath = "",
  requireClosedStatus = true,
} = {}) {
  const packetFormatVersion = parseSingleField(packetText, "PACKET_FORMAT_VERSION");
  const packetRiskTier = parseSingleField(packetText, "RISK_TIER").toUpperCase();
  const validatorReportProfile = parseSingleField(packetText, "GOVERNED_VALIDATOR_REPORT_PROFILE");
  const dataContractProfile = parseSingleField(packetText, "DATA_CONTRACT_PROFILE").toUpperCase();
  const sharedSurfaceRisk = parseSingleField(packetText, "SHARED_SURFACE_RISK").toUpperCase();
  const currentMainCompatibilityStatus = parseSingleField(packetText, "CURRENT_MAIN_COMPATIBILITY_STATUS").toUpperCase();
  const status = parseStatus(packetText);
  const report = parseValidationReport(packetText);
  const waiverLedger = parsePolicyWaiverLedger(packetText);
  const usesStructuredReport = packetUsesStructuredValidationReport(packetFormatVersion);
  const requiresCompletionLayer = packetRequiresCompletionLayerVerdicts(packetFormatVersion);
  const requiresRiskAudit = validatorReportProfileRequiresRiskAudit(validatorReportProfile);
  const requiresPrimitiveAudit = validatorReportProfileRequiresPrimitiveAudit(validatorReportProfile);
  const requiresDualTrack = validatorReportProfileRequiresDualTrack(
    validatorReportProfile,
    packetFormatVersion,
    packetRiskTier,
  );
  const closedStatus = isClosedStatus(status);
  const folderPacket = /\/packet\.md$/i.test(String(packetPath || "").trim());
  const applicable = usesStructuredReport && requiresCompletionLayer && (!requireClosedStatus || closedStatus);
  const applicabilityReason = !usesStructuredReport
    ? "UNSTRUCTURED_REPORT_PROFILE"
    : !requiresCompletionLayer
      ? "PRE_COMPLETION_LAYER_THRESHOLD"
      : requireClosedStatus && !closedStatus
        ? "NOT_CLOSED_STATUS"
        : "APPLICABLE";
  const legacyRemediationRequired = usesStructuredReport && closedStatus && !requiresCompletionLayer && folderPacket;

  if (!applicable) {
    const blockedIssues = legacyRemediationRequired
      ? [
        issue(
          "LEGACY_CLOSED_PACKET_REMEDIATION_REQUIRED",
          "BLOCKED",
          `Closed structured packet predates completion-layer threshold ${COMPLETION_LAYER_VERDICTS_MIN_VERSION} and must be remediated in a newer packet revision instead of being treated as PASS`,
          { source: "POLICY" },
        ),
      ]
      : [];
    return {
      applicable: false,
      applicability_reason: applicabilityReason,
      legacy_remediation_required: legacyRemediationRequired,
      wpId,
      packetFormatVersion,
      status,
      outcome: "BLOCKED",
      report,
      artifacts: {
        requirementRegistry: [],
        diffClaims: [],
        witnessMatrix: [],
        protectedSurfaceRegistry: null,
        waiverLedger,
      },
      issues: {
        all: blockedIssues,
        fail: [],
        reviewRequired: [],
        blocked: blockedIssues,
        waived: [],
      },
    };
  }

  const issues = [];
  const clauseClosure = validatePacketClosureMonitoring(packetText, {
    requireRows: true,
    requireClosedConsistency: true,
  });
  const clauseReportConsistency = validateClauseReportConsistency(packetText);
  const semanticProofProfile = parseSingleField(packetText, "SEMANTIC_PROOF_PROFILE");
  const semanticProof = /^DIFF_SCOPED_SEMANTIC_V1$/i.test(semanticProofProfile)
    ? validateSemanticProofAssets(packetText)
    : { errors: [], parsed: null };

  for (const error of clauseClosure.errors) {
    issues.push(issue("CLAUSE_CLOSURE_INVALID", "FAIL", error, { source: "CLAUSE_CLOSURE" }));
  }
  for (const error of clauseReportConsistency.errors) {
    issues.push(issue("CLAUSE_REPORT_MISMATCH", "FAIL", error, { source: "CLAUSE_REPORT" }));
  }
  for (const error of semanticProof.errors) {
    issues.push(issue("SEMANTIC_PROOF_INVALID", "FAIL", error, {
      coverage: "PROTECTED_SURFACE",
      waivable: false,
      source: "SEMANTIC_PROOF",
    }));
  }

  if (!report.raw.trim()) {
    issues.push(issue("VALIDATION_REPORT_MISSING", "BLOCKED", "VALIDATION_REPORTS missing/empty", {
      source: "REPORT",
    }));
  }

  addVerdictIssue(issues, report.validationContext, "OK", {
    codePrefix: "VALIDATION_CONTEXT",
    blockedValues: ["CONTEXT_MISMATCH", "NOT_RUN"],
    blockedCoverage: "ENVIRONMENT",
    blockedWaivable: true,
  });
  addVerdictIssue(issues, report.governanceVerdict, "PASS", {
    codePrefix: "GOVERNANCE_VERDICT",
    failValues: ["FAIL"],
    reviewValues: ["PARTIAL"],
    blockedValues: ["BLOCKED", "NOT_RUN"],
  });
  addVerdictIssue(issues, report.testVerdict, "PASS", {
    codePrefix: "TEST_VERDICT",
    failValues: ["FAIL"],
    reviewValues: ["PARTIAL"],
    blockedValues: ["BLOCKED", "NOT_RUN"],
    failCoverage: "TEST",
    reviewCoverage: "TEST",
    blockedCoverage: "TEST",
    failWaivable: true,
    reviewWaivable: true,
    blockedWaivable: true,
  });
  addVerdictIssue(issues, report.codeReviewVerdict, "PASS", {
    codePrefix: "CODE_REVIEW_VERDICT",
    failValues: ["FAIL"],
    reviewValues: ["PARTIAL"],
    blockedValues: ["BLOCKED", "NOT_RUN"],
  });
  addVerdictIssue(issues, report.heuristicReviewVerdict, "PASS", {
    codePrefix: "HEURISTIC_REVIEW_VERDICT",
    failValues: ["FAIL"],
    reviewValues: ["PARTIAL"],
    blockedValues: ["BLOCKED", "NOT_RUN"],
    failCoverage: "HEURISTIC",
    reviewCoverage: "HEURISTIC",
    blockedCoverage: "HEURISTIC",
    failWaivable: true,
    reviewWaivable: true,
    blockedWaivable: true,
  });
  addVerdictIssue(issues, report.specAlignmentVerdict, "PASS", {
    codePrefix: "SPEC_ALIGNMENT_VERDICT",
    failValues: ["FAIL"],
    reviewValues: ["PARTIAL"],
    blockedValues: ["BLOCKED", "NOT_RUN"],
  });
  addVerdictIssue(issues, report.environmentVerdict, "PASS", {
    codePrefix: "ENVIRONMENT_VERDICT",
    failValues: ["FAIL"],
    reviewValues: ["PARTIAL"],
    blockedValues: ["BLOCKED", "NOT_RUN"],
    failCoverage: "ENVIRONMENT",
    reviewCoverage: "ENVIRONMENT",
    blockedCoverage: "ENVIRONMENT",
    failWaivable: true,
    reviewWaivable: true,
    blockedWaivable: true,
  });
  addVerdictIssue(issues, report.workflowValidity, "VALID", {
    codePrefix: "WORKFLOW_VALIDITY",
    failValues: ["INVALID"],
    reviewValues: ["PARTIAL"],
    blockedValues: ["BLOCKED", "NOT_RUN"],
  });
  addVerdictIssue(issues, report.scopeValidity, "IN_SCOPE", {
    codePrefix: "SCOPE_VALIDITY",
    failValues: ["OUT_OF_SCOPE"],
    reviewValues: ["PARTIAL"],
    blockedValues: ["BLOCKED", "NOT_RUN"],
    failCoverage: "SCOPE",
    reviewCoverage: "SCOPE",
    blockedCoverage: "SCOPE",
    failWaivable: true,
    reviewWaivable: true,
    blockedWaivable: true,
  });
  addVerdictIssue(issues, report.proofCompleteness, "PROVEN", {
    codePrefix: "PROOF_COMPLETENESS",
    reviewValues: ["NOT_PROVEN", "PARTIAL"],
    blockedValues: ["BLOCKED", "NOT_RUN"],
    reviewCoverage: "PROOF",
    blockedCoverage: "PROOF",
    reviewWaivable: true,
    blockedWaivable: true,
  });
  addVerdictIssue(issues, report.integrationReadiness, "READY", {
    codePrefix: "INTEGRATION_READINESS",
    failValues: ["NOT_READY"],
    reviewValues: ["PARTIAL"],
    blockedValues: ["BLOCKED", "NOT_RUN"],
  });
  addVerdictIssue(issues, report.domainGoalCompletion, "COMPLETE", {
    codePrefix: "DOMAIN_GOAL_COMPLETION",
    failValues: ["INCOMPLETE"],
    reviewValues: ["PARTIAL"],
    blockedValues: ["BLOCKED", "NOT_RUN"],
  });
  if (requiresDualTrack && !report.mechanicalTrackVerdict) {
    issues.push(issue("MECHANICAL_TRACK_VERDICT_MISSING", "BLOCKED", "MECHANICAL_TRACK_VERDICT missing for dual-track validator packet", {
      coverage: "GOVERNANCE",
      waivable: false,
      source: "REPORT",
    }));
  }
  if (requiresDualTrack && !report.specRetentionTrackVerdict) {
    issues.push(issue("SPEC_RETENTION_TRACK_VERDICT_MISSING", "BLOCKED", "SPEC_RETENTION_TRACK_VERDICT missing for dual-track validator packet", {
      coverage: "SPEC",
      waivable: false,
      source: "REPORT",
    }));
  }
  if (requiresDualTrack) {
    addVerdictIssue(issues, report.mechanicalTrackVerdict, "PASS", {
      codePrefix: "MECHANICAL_TRACK_VERDICT",
      failValues: ["FAIL"],
      reviewValues: ["PARTIAL"],
      blockedValues: ["BLOCKED", "NOT_RUN"],
      source: "REPORT",
    });
    addVerdictIssue(issues, report.specRetentionTrackVerdict, "PASS", {
      codePrefix: "SPEC_RETENTION_TRACK_VERDICT",
      failValues: ["FAIL"],
      reviewValues: ["PARTIAL"],
      blockedValues: ["BLOCKED", "NOT_RUN"],
      source: "REPORT",
    });
  }
  addVerdictIssue(issues, report.legalVerdict, "PASS", {
    codePrefix: "LEGAL_VERDICT",
    failValues: ["FAIL"],
    reviewValues: ["PENDING"],
    blockedValues: ["BLOCKED", "NOT_RUN"],
  });

  if (report.notProven.length > 0 && report.proofCompleteness === "PROVEN") {
    issues.push(issue(
      "PROOF_NOT_PROVEN_CONFLICT",
      "FAIL",
      "PROOF_COMPLETENESS=PROVEN conflicts with non-empty NOT_PROVEN",
      { source: "REPORT" },
    ));
  }

  if (report.specAlignmentVerdict === "PASS" && report.notProven.length > 0) {
    issues.push(issue(
      "SPEC_ALIGNMENT_OVERRUN",
      "FAIL",
      "SPEC_ALIGNMENT_VERDICT=PASS conflicts with non-empty NOT_PROVEN",
      { source: "REPORT" },
    ));
  }

  if (report.legalVerdict === "PASS" && report.proofCompleteness !== "PROVEN") {
    issues.push(issue(
      "LEGAL_PROOF_CONFLICT",
      "FAIL",
      "LEGAL_VERDICT=PASS requires PROOF_COMPLETENESS=PROVEN",
      { source: "REPORT" },
    ));
  }

  if (report.disposition === "OUTDATED_ONLY") {
    issues.push(issue("DISPOSITION_OUTDATED_ONLY", "REVIEW_REQUIRED", "DISPOSITION=OUTDATED_ONLY", {
      coverage: "DOMAIN",
      waivable: false,
      source: "REPORT",
    }));
  } else if (report.disposition === "ABANDONED") {
    // Terminal discard/abandonment is governed separately from technical PASS/FAIL semantics.
  } else if (report.disposition && report.disposition !== "NONE") {
    issues.push(issue("DISPOSITION_NON_STANDARD", "REVIEW_REQUIRED", `DISPOSITION=${report.disposition}`, {
      source: "REPORT",
    }));
  }

  if (packetRequiresSpecClauseMap(packetFormatVersion) && report.specClauseMap.length === 0) {
    issues.push(issue("SPEC_CLAUSE_MAP_MISSING", "BLOCKED", "SPEC_CLAUSE_MAP missing for packet format that requires it", {
      source: "REPORT",
    }));
  }

  if (requiresPrimitiveAudit) {
    if (report.primitiveRetentionProof.length === 0) {
      issues.push(issue("PRIMITIVE_RETENTION_PROOF_MISSING", "BLOCKED", "PRIMITIVE_RETENTION_PROOF missing for V4 validator report", {
        coverage: "SPEC",
        waivable: false,
        source: "REPORT",
      }));
    }
    if (report.primitiveRetentionGaps.length === 0 && !/PRIMITIVE_RETENTION_GAPS\s*:/i.test(report.raw)) {
      issues.push(issue("PRIMITIVE_RETENTION_GAPS_MISSING", "BLOCKED", "PRIMITIVE_RETENTION_GAPS missing for V4 validator report", {
        coverage: "SPEC",
        waivable: false,
        source: "REPORT",
      }));
    }
    if (report.sharedSurfaceInteractionChecks.length === 0 && !/SHARED_SURFACE_INTERACTION_CHECKS\s*:/i.test(report.raw)) {
      issues.push(issue("SHARED_SURFACE_INTERACTION_CHECKS_MISSING", "BLOCKED", "SHARED_SURFACE_INTERACTION_CHECKS missing for V4 validator report", {
        coverage: "PROTECTED_SURFACE",
        waivable: false,
        source: "REPORT",
      }));
    }
    if (report.currentMainInteractionChecks.length === 0 && !/CURRENT_MAIN_INTERACTION_CHECKS\s*:/i.test(report.raw)) {
      issues.push(issue("CURRENT_MAIN_INTERACTION_CHECKS_MISSING", "BLOCKED", "CURRENT_MAIN_INTERACTION_CHECKS missing for V4 validator report", {
        coverage: "INTEGRATION",
        waivable: false,
        source: "REPORT",
      }));
    }
    if (report.specAlignmentVerdict === "PASS" && report.primitiveRetentionGaps.length > 0) {
      issues.push(issue("PRIMITIVE_RETENTION_GAPS_OPEN", "FAIL", "SPEC_ALIGNMENT_VERDICT=PASS conflicts with non-empty PRIMITIVE_RETENTION_GAPS", {
        coverage: "SPEC",
        waivable: false,
        source: "REPORT",
      }));
    }
    if (report.verdict === "PASS" && report.primitiveRetentionGaps.length > 0) {
      issues.push(issue("PASS_PRIMITIVE_RETENTION_GAPS_OPEN", "FAIL", "Verdict=PASS conflicts with non-empty PRIMITIVE_RETENTION_GAPS", {
        coverage: "SPEC",
        waivable: false,
        source: "REPORT",
      }));
    }
    if (lacksConcreteListEvidence(report.primitiveRetentionProof)) {
      issues.push(issue("PRIMITIVE_RETENTION_PROOF_NON_CONCRETE", "FAIL", "PRIMITIVE_RETENTION_PROOF must cite concrete code or symbol evidence", {
        coverage: "SPEC",
        waivable: false,
        source: "REPORT",
      }));
    }
    if (lacksConcreteListEvidence(report.sharedSurfaceInteractionChecks)) {
      issues.push(issue("SHARED_SURFACE_INTERACTION_NON_CONCRETE", "FAIL", "SHARED_SURFACE_INTERACTION_CHECKS must cite concrete code or symbol evidence", {
        coverage: "PROTECTED_SURFACE",
        waivable: false,
        source: "REPORT",
      }));
    }
    if (lacksConcreteListEvidence(report.currentMainInteractionChecks)) {
      issues.push(issue("CURRENT_MAIN_INTERACTION_NON_CONCRETE", "FAIL", "CURRENT_MAIN_INTERACTION_CHECKS must cite concrete code or symbol evidence", {
        coverage: "INTEGRATION",
        waivable: false,
        source: "REPORT",
      }));
    }
    if (riskTierRank(packetRiskTier) >= riskTierRank("MEDIUM") && report.primitiveRetentionProof.length === 0) {
      issues.push(issue("PRIMITIVE_RETENTION_AUDIT_MISSING", "FAIL", `Packet RISK_TIER=${packetRiskTier} requires explicit PRIMITIVE_RETENTION_PROOF`, {
        coverage: "SPEC",
        waivable: false,
        source: "REPORT",
      }));
    }
    if (riskTierRank(packetRiskTier) >= riskTierRank("MEDIUM") && report.sharedSurfaceInteractionChecks.length === 0) {
      issues.push(issue("SHARED_SURFACE_INTERACTION_AUDIT_MISSING", "FAIL", `Packet RISK_TIER=${packetRiskTier} requires explicit SHARED_SURFACE_INTERACTION_CHECKS`, {
        coverage: "PROTECTED_SURFACE",
        waivable: false,
        source: "REPORT",
      }));
    }
    if (riskTierRank(packetRiskTier) >= riskTierRank("MEDIUM") && report.currentMainInteractionChecks.length === 0) {
      issues.push(issue("CURRENT_MAIN_INTERACTION_AUDIT_MISSING", "FAIL", `Packet RISK_TIER=${packetRiskTier} requires explicit CURRENT_MAIN_INTERACTION_CHECKS`, {
        coverage: "INTEGRATION",
        waivable: false,
        source: "REPORT",
      }));
    }
    if (sharedSurfaceRisk === "YES" && report.sharedSurfaceInteractionChecks.length === 0) {
      issues.push(issue("SHARED_SURFACE_INTERACTION_REQUIRED", "FAIL", "SHARED_SURFACE_RISK=YES requires explicit SHARED_SURFACE_INTERACTION_CHECKS", {
        coverage: "PROTECTED_SURFACE",
        waivable: true,
        source: "REPORT",
      }));
    }
    if (currentMainCompatibilityStatus === "PASS" && report.currentMainInteractionChecks.length === 0) {
      issues.push(issue("CURRENT_MAIN_INTERACTION_REQUIRED", "FAIL", "CURRENT_MAIN_COMPATIBILITY_STATUS=PASS requires explicit CURRENT_MAIN_INTERACTION_CHECKS", {
        coverage: "INTEGRATION",
        waivable: false,
        source: "REPORT",
      }));
    }
  }

  const mechanicalTrackPassState = [
    report.governanceVerdict === "PASS",
    report.testVerdict === "PASS",
    report.codeReviewVerdict === "PASS",
    report.heuristicReviewVerdict === "PASS",
    report.environmentVerdict === "PASS",
    report.workflowValidity === "VALID",
    report.scopeValidity === "IN_SCOPE",
    report.proofCompleteness === "PROVEN",
    report.integrationReadiness === "READY",
    report.domainGoalCompletion === "COMPLETE",
  ].every(Boolean);

  const specRetentionTrackPassState =
    report.specAlignmentVerdict === "PASS"
    && report.notProven.length === 0
    && (!requiresRiskAudit || report.mainBodyGaps.length === 0)
    && (!requiresPrimitiveAudit || report.primitiveRetentionGaps.length === 0)
    && (!packetRequiresSpecClauseMap(packetFormatVersion) || report.specClauseMap.length > 0)
    && report.negativeProof.length > 0
    && (!requiresPrimitiveAudit || report.primitiveRetentionProof.length > 0)
    && (!requiresPrimitiveAudit || report.sharedSurfaceInteractionChecks.length > 0)
    && (!requiresPrimitiveAudit || report.currentMainInteractionChecks.length > 0)
    && (!requiresPrimitiveAudit || !lacksConcreteListEvidence(report.primitiveRetentionProof))
    && (!requiresPrimitiveAudit || !lacksConcreteListEvidence(report.sharedSurfaceInteractionChecks))
    && (!requiresPrimitiveAudit || !lacksConcreteListEvidence(report.currentMainInteractionChecks))
    && (!packetRequiresSpecClauseMap(packetFormatVersion) || !lacksConcreteListEvidence(report.specClauseMap))
    && !lacksConcreteListEvidence(report.negativeProof)
    && !report.negativeProof.some((item) => negativeProofLeaksToGovernance(item))
    && (dataContractProfile !== "LLM_FIRST_DATA_V1" || (report.dataContractProof.length > 0 && report.dataContractGaps.length === 0 && !lacksConcreteListEvidence(report.dataContractProof)));

  if (requiresDualTrack && report.mechanicalTrackVerdict === "PASS" && !mechanicalTrackPassState) {
    issues.push(issue("MECHANICAL_TRACK_OVERRUN", "FAIL", "MECHANICAL_TRACK_VERDICT=PASS conflicts with non-pass mechanical review fields", {
      coverage: "GOVERNANCE",
      waivable: false,
      source: "REPORT",
    }));
  }

  if (requiresDualTrack && report.specRetentionTrackVerdict === "PASS" && !specRetentionTrackPassState) {
    issues.push(issue("SPEC_RETENTION_TRACK_OVERRUN", "FAIL", "SPEC_RETENTION_TRACK_VERDICT=PASS conflicts with non-pass spec-retention evidence", {
      coverage: "SPEC",
      waivable: false,
      source: "REPORT",
    }));
  }

  if (requiresDualTrack && report.legalVerdict === "PASS" && report.mechanicalTrackVerdict !== "PASS") {
    issues.push(issue("LEGAL_TRACK_MECHANICAL_MISMATCH", "FAIL", "LEGAL_VERDICT=PASS requires MECHANICAL_TRACK_VERDICT=PASS for dual-track packets", {
      coverage: "GOVERNANCE",
      waivable: false,
      source: "REPORT",
    }));
  }

  if (requiresDualTrack && report.legalVerdict === "PASS" && report.specRetentionTrackVerdict !== "PASS") {
    issues.push(issue("LEGAL_TRACK_SPEC_RETENTION_MISMATCH", "FAIL", "LEGAL_VERDICT=PASS requires SPEC_RETENTION_TRACK_VERDICT=PASS for dual-track packets", {
      coverage: "SPEC",
      waivable: false,
      source: "REPORT",
    }));
  }

  const protectedSurfaceState = classifyProtectedSurfaceState(clauseClosure.parsed, report);
  if (clauseClosure.parsed?.sharedSurfaceRisk === "YES" && protectedSurfaceState !== "REVIEWED") {
    issues.push(issue(
      `PROTECTED_SURFACE_${protectedSurfaceState}`,
      protectedSurfaceState === "NOT_REVIEWED" ? "FAIL" : "REVIEW_REQUIRED",
      `Shared-surface packet requires both boundary and negative-path review evidence (state: ${protectedSurfaceState})`,
      {
        coverage: "PROTECTED_SURFACE",
        waivable: true,
        source: "PROTECTED_SURFACE",
      },
    ));
  }

  if (report.verdict === "PASS") {
    if (requiresDualTrack && report.mechanicalTrackVerdict !== "PASS") {
      issues.push(issue("PASS_TRACK_MECHANICAL_MISMATCH", "FAIL", "Verdict=PASS requires MECHANICAL_TRACK_VERDICT=PASS for dual-track packets", {
        coverage: "GOVERNANCE",
        waivable: false,
        source: "REPORT",
      }));
    }
    if (requiresDualTrack && report.specRetentionTrackVerdict !== "PASS") {
      issues.push(issue("PASS_TRACK_SPEC_RETENTION_MISMATCH", "FAIL", "Verdict=PASS requires SPEC_RETENTION_TRACK_VERDICT=PASS for dual-track packets", {
        coverage: "SPEC",
        waivable: false,
        source: "REPORT",
      }));
    }
    const rawNonPassIssues = issues.filter((item) => item.severity !== "BLOCKED" ? item.severity !== "PASS" : true);
    const rawUnwaivedPreview = applyWaivers(rawNonPassIssues, waiverLedger).unwaived;
    if (rawUnwaivedPreview.some((item) => item.severity === "FAIL" || item.severity === "REVIEW_REQUIRED" || item.severity === "BLOCKED")) {
      issues.push(issue(
        "NARRATIVE_PASS_OVERRUN",
        "FAIL",
        "Verdict=PASS outruns the computed policy inputs",
        { source: "POLICY" },
      ));
    }
  }

  const waiverResolution = applyWaivers(issues, waiverLedger);
  const failIssues = waiverResolution.unwaived.filter((item) => item.severity === "FAIL");
  const blockedIssues = waiverResolution.unwaived.filter((item) => item.severity === "BLOCKED");
  const reviewIssues = waiverResolution.unwaived.filter((item) => item.severity === "REVIEW_REQUIRED");

  let outcome = "PASS";
  if (failIssues.length > 0) {
    outcome = "FAIL";
  } else if (blockedIssues.length > 0) {
    outcome = "BLOCKED";
  } else if (reviewIssues.length > 0) {
    outcome = "REVIEW_REQUIRED";
  } else if (waiverResolution.waived.length > 0) {
    outcome = "WAIVED";
  }

  return {
    applicable: true,
    applicability_reason: applicabilityReason,
    legacy_remediation_required: false,
    wpId,
    packetFormatVersion,
    status,
    outcome,
    policyInputs: report,
    artifacts: {
      requirementRegistry: clauseClosure.parsed?.clauseRows || [],
      diffClaims: {
        clausesReviewed: report.clausesReviewed,
        specClauseMap: report.specClauseMap,
        negativeProof: report.negativeProof,
      },
      witnessMatrix: buildWitnessMatrix(clauseClosure.parsed, report),
      protectedSurfaceRegistry: clauseClosure.parsed
        ? {
            sharedSurfaceRisk: clauseClosure.parsed.sharedSurfaceRisk,
            hotFiles: clauseClosure.parsed.hotFiles,
            requiredTripwireTests: clauseClosure.parsed.requiredTripwireTests,
            postMergeSpotcheckRequired: clauseClosure.parsed.postMergeSpotcheckRequired,
            protectedSurfaceState,
          }
        : null,
      waiverLedger,
    },
    issues: {
      all: waiverResolution.unwaived,
      fail: failIssues,
      reviewRequired: reviewIssues,
      blocked: blockedIssues,
      waived: waiverResolution.waived,
    },
  };
}

export function computedPolicyOutcomeAllowsClosure(evaluation) {
  if (evaluation?.applicable !== true) return false;
  if (evaluation.outcome === "PASS" || evaluation.outcome === "WAIVED") return true;

  const status = String(evaluation?.status || "").trim();
  const blockedIssues = Array.isArray(evaluation?.issues?.blocked) ? evaluation.issues.blocked : [];
  if (blockedIssues.length > 0) return false;

  if (!/^Validated\s*\(\s*(FAIL|OUTDATED_ONLY|ABANDONED)\s*\)$/i.test(status)) {
    return false;
  }

  const terminalIssues = [
    ...(Array.isArray(evaluation?.issues?.fail) ? evaluation.issues.fail : []),
    ...(Array.isArray(evaluation?.issues?.reviewRequired) ? evaluation.issues.reviewRequired : []),
  ];
  const allowedTerminalIssueRe =
    /^(VALIDATION_CONTEXT|GOVERNANCE_VERDICT|TEST_VERDICT|CODE_REVIEW_VERDICT|HEURISTIC_REVIEW_VERDICT|SPEC_ALIGNMENT_VERDICT|ENVIRONMENT_VERDICT|LEGAL_VERDICT|WORKFLOW_VALIDITY|SCOPE_VALIDITY|PROOF_COMPLETENESS|INTEGRATION_READINESS|DOMAIN_GOAL_COMPLETION|MECHANICAL_TRACK_VERDICT|SPEC_RETENTION_TRACK_VERDICT)_[A-Z_]+$/;

  return terminalIssues.every((item) => {
    const code = String(item?.code || "").trim().toUpperCase();
    if (!code) return false;
    if (allowedTerminalIssueRe.test(code)) return true;
    return /^Validated\s*\(\s*OUTDATED_ONLY\s*\)$/i.test(status) && code === "DISPOSITION_OUTDATED_ONLY";
  });
}
