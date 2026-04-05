import {
  COMPLETION_LAYER_VERDICTS_MIN_VERSION,
  packetRequiresCompletionLayerVerdicts,
  packetRequiresSpecClauseMap,
  packetUsesStructuredValidationReport,
} from "../session/session-policy.mjs";
import { validateClauseReportConsistency, validatePacketClosureMonitoring } from "./packet-closure-monitor-lib.mjs";
import { validateSemanticProofAssets } from "./semantic-proof-lib.mjs";

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
  const re = new RegExp(`^\\s*${label}\\s*:\\s*(.+)\\s*$`, "im");
  const match = String(sectionText || "").match(re);
  return match ? match[1].trim() : "";
}

function extractListItemsAfterLabel(sectionText, label) {
  const lines = String(sectionText || "").split(/\r?\n/);
  const labelRe = new RegExp(`^\\s*${label}\\s*:\\s*$`, "i");
  const headingRe = /^#{1,6}\s+\S/;
  const nextLabelRe = /^\s*[A-Z][A-Z0-9_ ()/-]*\s*:\s*$/;

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
  const status = parseStatus(packetText);
  const report = parseValidationReport(packetText);
  const waiverLedger = parsePolicyWaiverLedger(packetText);
  const usesStructuredReport = packetUsesStructuredValidationReport(packetFormatVersion);
  const requiresCompletionLayer = packetRequiresCompletionLayerVerdicts(packetFormatVersion);
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
  return evaluation?.applicable === true && (evaluation.outcome === "PASS" || evaluation.outcome === "WAIVED");
}
