#!/usr/bin/env node
/**
 * Packet completeness checker for validators.
 * Ensures required fields are present and sane.
 */
import { readFileSync } from "node:fs";
import { REPO_ROOT, workPacketAbsPath, workPacketPath } from "../../../roles_shared/scripts/lib/runtime-paths.mjs";
import {
  packetRequiresCompletionLayerVerdicts,
  packetRequiresSpecClauseMap,
  packetUsesStructuredValidationReport,
} from "../../../roles_shared/scripts/session/session-policy.mjs";
import { validateClauseReportConsistency, validatePacketClosureMonitoring } from "../../../roles_shared/scripts/lib/packet-closure-monitor-lib.mjs";
import {
  computedPolicyOutcomeAllowsClosure,
  evaluateComputedPolicyGateFromPacketText,
} from "../../../roles_shared/scripts/lib/computed-policy-gate-lib.mjs";
import { validateMergeProgressionTruth } from "../../../roles_shared/scripts/lib/merge-progression-truth-lib.mjs";
import { validateSemanticProofAssets } from "../../../roles_shared/scripts/lib/semantic-proof-lib.mjs";
import { validateSignedScopeCompatibilityTruth } from "../../../roles_shared/scripts/lib/signed-scope-compatibility-lib.mjs";
import { validateContainedMainCommitAgainstSignedScope } from "../../../roles_shared/scripts/lib/signed-scope-surface-lib.mjs";
import {
  packetUsesDataContractProfile,
  parseDataContractProfile,
  validateDataContractDecisionSection,
  validateDataContractSection,
} from "../../../roles_shared/scripts/lib/data-contract-lib.mjs";
import { parsePacketScopeList } from "../../../roles_shared/scripts/lib/scope-surface-lib.mjs";
import {
  activeWorkflowInvalidityReceipt,
  parseJsonlFile,
  workflowInvalidityReceipts,
} from "../../../roles_shared/scripts/lib/wp-communications-lib.mjs";
import { evaluateWpDeclaredTopology } from "../../../roles_shared/scripts/lib/wp-declared-topology-lib.mjs";

const wpId = process.argv[2];
if (!wpId) {
  console.error("Usage: just validator-packet-complete WP-1-Example");
  process.exit(1);
}

const packetPath = workPacketPath(wpId);
const packetAbsPath = workPacketAbsPath(wpId);

function fail(msg) {
  console.error(`validator-packet-complete: FAIL - ${msg}`);
  process.exit(1);
}

let text;
try {
  text = readFileSync(packetAbsPath, "utf8");
} catch (err) {
  fail(`cannot read ${packetPath}: ${err.message}`);
}

const lines = text.split(/\r?\n/);

function hasLine(re) {
  return re.test(text);
}

function isPlaceholder(value) {
  const v = (value || "").trim();
  if (!v) return true;
  if (/^\{.+\}$/.test(v)) return true;
  if (/^<fill/i.test(v)) return true;
  if (/^<pending>$/i.test(v)) return true;
  if (/^<unclaimed>$/i.test(v)) return true;
  if (/^tbd$/i.test(v)) return true;
  return false;
}

function parseSingleField(label) {
  const re = new RegExp(`^\\s*-\\s*(?:\\*\\*)?${label}(?:\\*\\*)?\\s*:\\s*(.*)\\s*$`, "i");
  for (const line of lines) {
    const m = line.match(re);
    if (m) return (m[1] ?? "").trim();
  }
  return "";
}

function loadWorkflowInvalidityEntries() {
  const receiptsFile = parseSingleField("WP_RECEIPTS_FILE");
  if (!receiptsFile) return { history: [], active: null };
  try {
    const receipts = parseJsonlFile(receiptsFile);
    return {
      history: workflowInvalidityReceipts(receipts),
      active: activeWorkflowInvalidityReceipt(receipts),
    };
  } catch (error) {
    fail(`cannot read workflow invalidity receipts from ${receiptsFile}: ${error.message}`);
  }
}

function hasNonPlaceholderListItemAfterLabel(label) {
  const labelRe = new RegExp(`^\\s*-\\s*(?:\\*\\*)?${label}(?:\\*\\*)?\\s*:\\s*$`, "i");
  const topLevelBulletRe = /^\s*-\s*[A-Z0-9_]+\s*:/i;
  const sectionHeaderRe = /^\s*##\s+/;

  const labelIdx = lines.findIndex((line) => labelRe.test(line));
  if (labelIdx === -1) return false;

  for (let i = labelIdx + 1; i < lines.length; i += 1) {
    const line = lines[i];
    if (sectionHeaderRe.test(line)) break;
    if (topLevelBulletRe.test(line)) break;

    const m = line.match(/^\s*-\s+(.+)\s*$/);
    if (!m) continue;
    const v = (m[1] ?? "").trim().replace(/^`|`$/g, "");
    if (!isPlaceholder(v)) return true;
  }

  return false;
}

function extractSectionAfterHeading(heading) {
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

function hasListItemAfterLabel(text, label) {
  const sectionLines = String(text || "").split(/\r?\n/);
  const labelRe = new RegExp(`^\\s*${label}\\s*:\\s*$`, "i");
  const headingRe = /^#{1,6}\s+\S/;
  const nextLabelRe = /^\s*[A-Z][A-Z0-9_ ()/-]*\s*:\s*$/;

  const labelIdx = sectionLines.findIndex((line) => labelRe.test(line));
  if (labelIdx === -1) return false;

  for (let index = labelIdx + 1; index < sectionLines.length; index += 1) {
    const line = sectionLines[index];
    if (headingRe.test(line)) break;
    if (nextLabelRe.test(line)) break;
    const match = line.match(/^\s*-\s+(.+)\s*$/);
    if (!match) continue;
    const value = (match[1] ?? "").trim().replace(/^`|`$/g, "");
    if (!isPlaceholder(value)) return true;
  }

  return false;
}

function extractListItemsAfterLabel(text, label) {
  const sectionLines = String(text || "").split(/\r?\n/);
  const labelRe = new RegExp(`^\\s*${label}\\s*:\\s*$`, "i");
  const headingRe = /^#{1,6}\s+\S/;
  const nextLabelRe = /^\s*[A-Z][A-Z0-9_ ()/-]*\s*:\s*$/;
  const items = [];

  const labelIdx = sectionLines.findIndex((line) => labelRe.test(line));
  if (labelIdx === -1) return items;

  for (let index = labelIdx + 1; index < sectionLines.length; index += 1) {
    const line = sectionLines[index];
    if (headingRe.test(line)) break;
    if (nextLabelRe.test(line)) break;
    const match = line.match(/^\s*-\s+(.+)\s*$/);
    if (!match) continue;
    const value = (match[1] ?? "").trim().replace(/^`|`$/g, "");
    if (!isPlaceholder(value)) items.push(value);
  }

  return items;
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

const statusMatch = text.match(/(?:\*\*Status:\*\*|STATUS:)\s*(Ready for Dev|In Progress|Blocked|Done(?:\s*\(Historical\))?|Validated\s*\((?:PASS|FAIL|OUTDATED_ONLY|ABANDONED)\))(?=\s|$)/i);
if (!statusMatch) {
  fail("STATUS missing or invalid (must be Ready for Dev / In Progress / Blocked / Done / Done (Historical) / Validated (PASS|FAIL|OUTDATED_ONLY|ABANDONED))");
}
const statusValue = (statusMatch[1] || "").trim();
const closureStatus = /\b(done|validated)\b/i.test(statusValue);

const hasLegacySpec = hasLine(/SPEC_CURRENT/i);
const hasSpecBaseline = hasLine(/SPEC_BASELINE/i);
const hasSpecTarget = hasLine(/SPEC_TARGET/i);
if (!hasLegacySpec && !(hasSpecBaseline && hasSpecTarget)) {
  fail("SPEC reference missing (need SPEC_CURRENT or SPEC_BASELINE+SPEC_TARGET)");
}
if (!hasLine(/RISK_TIER/i)) {
  fail("RISK_TIER missing");
}
if (!hasLine(/DONE_MEANS/i) || hasLine(/DONE_MEANS\s*:\s*$/i) || hasLine(/DONE_MEANS\s*:\s*tbd/i)) {
  fail("DONE_MEANS missing or placeholder");
}
if (!hasLine(/TEST_PLAN/i) || hasLine(/TEST_PLAN\s*:\s*$/i) || hasLine(/TEST_PLAN\s*:\s*tbd/i)) {
  fail("TEST_PLAN missing or placeholder");
}
if (!hasLine(/BOOTSTRAP/i)) {
  fail("BOOTSTRAP missing");
}
if (!hasLine(/USER_SIGNATURE/i) && !hasLine(/User Signature Locked/i)) {
  fail("USER_SIGNATURE missing");
}

// Newer template-only requirements (avoid breaking legacy packets).
const packetFormatVersion = parseSingleField("PACKET_FORMAT_VERSION");
const workflowInvalidityState = loadWorkflowInvalidityEntries();
const workflowInvalidityEntries = workflowInvalidityState.history;
const activeWorkflowInvalidity = workflowInvalidityState.active;
const usesDataContractProfile = packetUsesDataContractProfile(packetFormatVersion);
const enforcesAntiVibeRigor = packetFormatVersion >= "2026-04-01";
const dataContractProfile = parseDataContractProfile(text);
const inScopePaths = parsePacketScopeList(text, "IN_SCOPE_PATHS", { stopLabels: ["OUT_OF_SCOPE"] });
const topologyEvaluation = evaluateWpDeclaredTopology({
  repoRoot: REPO_ROOT,
  wpId,
  packetContent: text,
});
if (packetFormatVersion) {
  if (isPlaceholder(packetFormatVersion)) {
    fail("PACKET_FORMAT_VERSION present but placeholder");
  }

  if (!hasLine(/^##\s*END_TO_END_CLOSURE_PLAN\b/im)) {
    fail("END_TO_END_CLOSURE_PLAN section missing (required for PACKET_FORMAT_VERSION packets)");
  }

  const applicable = parseSingleField("END_TO_END_CLOSURE_PLAN_APPLICABLE");
  if (!/^(YES|NO)$/i.test(applicable)) {
    fail("END_TO_END_CLOSURE_PLAN_APPLICABLE missing/invalid (must be YES or NO)");
  }

  if (/^YES$/i.test(applicable)) {
    const trustBoundary = parseSingleField("TRUST_BOUNDARY");
    if (isPlaceholder(trustBoundary)) {
      fail("TRUST_BOUNDARY missing/placeholder (required when END_TO_END_CLOSURE_PLAN_APPLICABLE is YES)");
    }

    const requiredLists = [
      "SERVER_SOURCES_OF_TRUTH",
      "REQUIRED_PROVENANCE_FIELDS",
      "VERIFICATION_PLAN",
      "ERROR_TAXONOMY_PLAN",
      "UI_GUARDRAILS",
      "VALIDATOR_ASSERTIONS",
    ];

    for (const label of requiredLists) {
      if (!hasNonPlaceholderListItemAfterLabel(label)) {
        fail(`${label} missing/placeholder list items (required when END_TO_END_CLOSURE_PLAN_APPLICABLE is YES)`);
      }
    }
  }

  const clauseClosureMonitorProfile = parseSingleField("CLAUSE_CLOSURE_MONITOR_PROFILE");
  const usesClauseClosureMonitor = /^CLAUSE_MONITOR_V1$/i.test(clauseClosureMonitorProfile);
  const semanticProofProfile = parseSingleField("SEMANTIC_PROOF_PROFILE");
  const usesSemanticProofProfile = /^DIFF_SCOPED_SEMANTIC_V1$/i.test(semanticProofProfile);
  const validatorReportProfile = parseSingleField("GOVERNED_VALIDATOR_REPORT_PROFILE");
  const usesRigorV2Report = /^SPLIT_DIFF_SCOPED_RIGOR_V2$/i.test(validatorReportProfile);
  const usesRigorV3Report = /^SPLIT_DIFF_SCOPED_RIGOR_V3$/i.test(validatorReportProfile);
  const usesHeuristicRigorReport = usesRigorV2Report || usesRigorV3Report;
  const usesCompletionLayerVerdicts = packetRequiresCompletionLayerVerdicts(packetFormatVersion);
  let computedPolicy = null;

  if (usesDataContractProfile) {
    const rawDataContractProfile = parseSingleField("DATA_CONTRACT_PROFILE");
    if (isPlaceholder(rawDataContractProfile)) {
      fail("DATA_CONTRACT_PROFILE missing/placeholder for PACKET_FORMAT_VERSION >= 2026-04-01");
    }
    const dataContractDecisionValidation = validateDataContractDecisionSection(text, {
      packetPath,
      inScopePaths,
    });
    if (dataContractDecisionValidation.errors.length > 0) {
      fail(`data contract decision invalid: ${dataContractDecisionValidation.errors.join("; ")}`);
    }
    const dataContractValidation = validateDataContractSection(text, {
      packetPath,
    });
    if (dataContractValidation.errors.length > 0) {
      fail(`data contract monitoring invalid: ${dataContractValidation.errors.join("; ")}`);
    }
  }

  if (closureStatus && packetUsesStructuredValidationReport(packetFormatVersion)) {
    computedPolicy = evaluateComputedPolicyGateFromPacketText(text, {
      wpId,
      packetPath,
      requireClosedStatus: true,
    });
    if (computedPolicy.legacy_remediation_required) {
      const details = computedPolicy.issues.blocked.map((item) => `${item.code}: ${item.message}`);
      fail(`legacy remediation required for closed structured packet${details.length > 0 ? ` (${details.join("; ")})` : ""}`);
    }

    if (usesClauseClosureMonitor) {
      const closureMonitorValidation = validatePacketClosureMonitoring(text, {
        requireRows: true,
        requireClosedConsistency: true,
      });
      if (closureMonitorValidation.errors.length > 0) {
        fail(`packet closure monitoring invalid for closed packet: ${closureMonitorValidation.errors.join("; ")}`);
      }
    }
    if (usesSemanticProofProfile) {
      const semanticProofValidation = validateSemanticProofAssets(text);
      if (semanticProofValidation.errors.length > 0) {
        fail(`semantic proof assets invalid for closed packet: ${semanticProofValidation.errors.join("; ")}`);
      }
    }
    const mergeProgressionTruth = validateMergeProgressionTruth(text, {
      packetPath,
    });
    if (mergeProgressionTruth.errors.length > 0) {
      fail(`merge progression truth invalid for closed packet: ${mergeProgressionTruth.errors.join("; ")}`);
    }
    const signedScopeCompatibilityTruth = validateSignedScopeCompatibilityTruth(text, {
      packetPath,
    });
    if (signedScopeCompatibilityTruth.errors.length > 0) {
      fail(`signed scope compatibility truth invalid for closed packet: ${signedScopeCompatibilityTruth.errors.join("; ")}`);
    }
    if (/^Validated\s*\(\s*PASS\s*\)$/i.test(statusValue)) {
      const containedMainScope = validateContainedMainCommitAgainstSignedScope(text, {
        repoRoot: REPO_ROOT,
        mergedMainCommit: mergeProgressionTruth?.parsed?.mergedMainCommit || "",
        requireExactArtifactMatch: false,
      });
      if (containedMainScope.errors.length > 0) {
        fail(`contained main commit violates signed scope surface: ${containedMainScope.errors.join("; ")}`);
      }
    }
    if (!topologyEvaluation.ok) {
      fail(`declared WP topology invalid for closed packet: ${topologyEvaluation.issues.join("; ")}`);
    }

    const validationReports = extractSectionAfterHeading("VALIDATION_REPORTS");
    if (!validationReports.trim()) {
      fail("VALIDATION_REPORTS missing/empty for closed packet");
    }

    const requiredSingleFields = [
      "VALIDATION_CONTEXT",
      "GOVERNANCE_VERDICT",
      "TEST_VERDICT",
      "CODE_REVIEW_VERDICT",
      "SPEC_ALIGNMENT_VERDICT",
      "ENVIRONMENT_VERDICT",
      "DISPOSITION",
      "LEGAL_VERDICT",
      "SPEC_CONFIDENCE",
    ];
    if (usesHeuristicRigorReport) {
      requiredSingleFields.splice(4, 0, "HEURISTIC_REVIEW_VERDICT");
    }
    if (usesRigorV3Report) {
      requiredSingleFields.push("VALIDATOR_RISK_TIER");
    }
    if (usesCompletionLayerVerdicts) {
      requiredSingleFields.push(
        "WORKFLOW_VALIDITY",
        "SCOPE_VALIDITY",
        "PROOF_COMPLETENESS",
        "INTEGRATION_READINESS",
        "DOMAIN_GOAL_COMPLETION",
      );
    }

    for (const label of requiredSingleFields) {
      const re = new RegExp(`^\\s*${label}\\s*:\\s*(.+)\\s*$`, "im");
      const match = validationReports.match(re);
      if (!match || isPlaceholder(match[1])) {
        fail(`${label} missing/placeholder in VALIDATION_REPORTS for closed packet`);
      }
    }

    if (!hasLine(/^##\s*VALIDATION_REPORTS\b/im)) {
      fail("VALIDATION_REPORTS heading missing");
    }
    if (!/^\s*Verdict\s*:\s*(PASS|FAIL|NOT_PROVEN|OUTDATED_ONLY|ABANDONED|BLOCKED)\b/im.test(validationReports)) {
      fail("VALIDATION_REPORTS missing top-level Verdict: PASS|FAIL|NOT_PROVEN|OUTDATED_ONLY|ABANDONED|BLOCKED");
    }
    if (!hasListItemAfterLabel(validationReports, "CLAUSES_REVIEWED")) {
      fail("CLAUSES_REVIEWED missing/placeholder list items in VALIDATION_REPORTS for closed packet");
    }
    if (!hasListItemAfterLabel(validationReports, "NOT_PROVEN")) {
      fail("NOT_PROVEN missing/placeholder list items in VALIDATION_REPORTS for closed packet");
    }
    if (usesHeuristicRigorReport && !hasListItemAfterLabel(validationReports, "MAIN_BODY_GAPS")) {
      fail("MAIN_BODY_GAPS missing/placeholder list items in VALIDATION_REPORTS for closed packet");
    }
    if (usesHeuristicRigorReport && !hasListItemAfterLabel(validationReports, "QUALITY_RISKS")) {
      fail("QUALITY_RISKS missing/placeholder list items in VALIDATION_REPORTS for closed packet");
    }
    if (usesRigorV3Report && enforcesAntiVibeRigor && !hasListItemAfterLabel(validationReports, "ANTI_VIBE_FINDINGS")) {
      fail("ANTI_VIBE_FINDINGS missing/placeholder list items in VALIDATION_REPORTS for closed packet");
    }
    if (usesRigorV3Report && enforcesAntiVibeRigor && !hasListItemAfterLabel(validationReports, "SIGNED_SCOPE_DEBT")) {
      fail("SIGNED_SCOPE_DEBT missing/placeholder list items in VALIDATION_REPORTS for closed packet");
    }
    if (usesRigorV3Report && !hasListItemAfterLabel(validationReports, "DIFF_ATTACK_SURFACES")) {
      fail("DIFF_ATTACK_SURFACES missing/placeholder list items in VALIDATION_REPORTS for closed packet");
    }
    if (usesRigorV3Report && !hasListItemAfterLabel(validationReports, "INDEPENDENT_CHECKS_RUN")) {
      fail("INDEPENDENT_CHECKS_RUN missing/placeholder list items in VALIDATION_REPORTS for closed packet");
    }
    if (usesRigorV3Report && !hasListItemAfterLabel(validationReports, "COUNTERFACTUAL_CHECKS")) {
      fail("COUNTERFACTUAL_CHECKS missing/placeholder list items in VALIDATION_REPORTS for closed packet");
    }
    if (usesRigorV3Report && !hasListItemAfterLabel(validationReports, "INDEPENDENT_FINDINGS")) {
      fail("INDEPENDENT_FINDINGS missing/placeholder list items in VALIDATION_REPORTS for closed packet");
    }
    if (usesRigorV3Report && !hasListItemAfterLabel(validationReports, "RESIDUAL_UNCERTAINTY")) {
      fail("RESIDUAL_UNCERTAINTY missing/placeholder list items in VALIDATION_REPORTS for closed packet");
    }
    if (usesRigorV3Report && packetRequiresSpecClauseMap(packetFormatVersion) && !hasListItemAfterLabel(validationReports, "SPEC_CLAUSE_MAP")) {
      fail("SPEC_CLAUSE_MAP missing/placeholder list items in VALIDATION_REPORTS for closed packet (required for RIGOR_V3)");
    }
    if (usesRigorV3Report && packetRequiresSpecClauseMap(packetFormatVersion)) {
      const negativeProofItems = extractListItemsAfterLabel(validationReports, "NEGATIVE_PROOF");
      if (negativeProofItems.length === 0 || hasOnlyNoneList(negativeProofItems)) {
        fail("NEGATIVE_PROOF must list at least one spec requirement verified as NOT fully implemented (required for RIGOR_V3)");
      }
    }
    if (dataContractProfile === "LLM_FIRST_DATA_V1") {
      const dataContractProofItems = extractListItemsAfterLabel(validationReports, "DATA_CONTRACT_PROOF");
      if (dataContractProofItems.length === 0 || hasOnlyNoneList(dataContractProofItems)) {
        fail("DATA_CONTRACT_PROOF must list concrete proof items in VALIDATION_REPORTS for active data contract packet");
      }
      if (!hasListItemAfterLabel(validationReports, "DATA_CONTRACT_GAPS")) {
        fail("DATA_CONTRACT_GAPS missing/placeholder list items in VALIDATION_REPORTS for active data contract packet");
      }
    }

    if (usesClauseClosureMonitor) {
      const reportConsistency = validateClauseReportConsistency(text);
      if (reportConsistency.errors.length > 0) {
        fail(`CLAUSE_CLOSURE_MATRIX / VALIDATION_REPORTS mismatch: ${reportConsistency.errors.join("; ")}`);
      }
    }

    const specAlignmentVerdictMatch = validationReports.match(/^\s*SPEC_ALIGNMENT_VERDICT\s*:\s*(.+)\s*$/im);
    const specAlignmentVerdict = specAlignmentVerdictMatch ? (specAlignmentVerdictMatch[1] || "").trim().toUpperCase() : "";
    const heuristicReviewVerdictMatch = validationReports.match(/^\s*HEURISTIC_REVIEW_VERDICT\s*:\s*(.+)\s*$/im);
    const heuristicReviewVerdict = heuristicReviewVerdictMatch ? (heuristicReviewVerdictMatch[1] || "").trim().toUpperCase() : "";
    const legalVerdictMatch = validationReports.match(/^\s*LEGAL_VERDICT\s*:\s*(.+)\s*$/im);
    const legalVerdict = legalVerdictMatch ? (legalVerdictMatch[1] || "").trim().toUpperCase() : "";
    const topLevelVerdictMatch = validationReports.match(/^\s*Verdict\s*:\s*(.+)\s*$/im);
    const topLevelVerdict = topLevelVerdictMatch ? (topLevelVerdictMatch[1] || "").trim().toUpperCase() : "";
    const validationContextMatch = validationReports.match(/^\s*VALIDATION_CONTEXT\s*:\s*(.+)\s*$/im);
    const validationContext = validationContextMatch ? (validationContextMatch[1] || "").trim().toUpperCase() : "";
    const governanceVerdictMatch = validationReports.match(/^\s*GOVERNANCE_VERDICT\s*:\s*(.+)\s*$/im);
    const governanceVerdict = governanceVerdictMatch ? (governanceVerdictMatch[1] || "").trim().toUpperCase() : "";
    const environmentVerdictMatch = validationReports.match(/^\s*ENVIRONMENT_VERDICT\s*:\s*(.+)\s*$/im);
    const environmentVerdict = environmentVerdictMatch ? (environmentVerdictMatch[1] || "").trim().toUpperCase() : "";
    const dispositionMatch = validationReports.match(/^\s*DISPOSITION\s*:\s*(.+)\s*$/im);
    const disposition = dispositionMatch ? (dispositionMatch[1] || "").trim().toUpperCase() : "";
    const mainBodyGaps = extractListItemsAfterLabel(validationReports, "MAIN_BODY_GAPS");
    const qualityRisks = extractListItemsAfterLabel(validationReports, "QUALITY_RISKS");
    const antiVibeFindings = extractListItemsAfterLabel(validationReports, "ANTI_VIBE_FINDINGS");
    const signedScopeDebt = extractListItemsAfterLabel(validationReports, "SIGNED_SCOPE_DEBT");
    const notProvenItems = extractListItemsAfterLabel(validationReports, "NOT_PROVEN");
    const attackSurfaces = extractListItemsAfterLabel(validationReports, "DIFF_ATTACK_SURFACES");
    const independentChecks = extractListItemsAfterLabel(validationReports, "INDEPENDENT_CHECKS_RUN");
    const counterfactualChecks = extractListItemsAfterLabel(validationReports, "COUNTERFACTUAL_CHECKS");
    const residualUncertainty = extractListItemsAfterLabel(validationReports, "RESIDUAL_UNCERTAINTY");
    const boundaryProbes = extractListItemsAfterLabel(validationReports, "BOUNDARY_PROBES");
    const negativePathChecks = extractListItemsAfterLabel(validationReports, "NEGATIVE_PATH_CHECKS");
    const negativeProofItems = extractListItemsAfterLabel(validationReports, "NEGATIVE_PROOF");
    const dataContractProof = extractListItemsAfterLabel(validationReports, "DATA_CONTRACT_PROOF");
    const dataContractGaps = extractListItemsAfterLabel(validationReports, "DATA_CONTRACT_GAPS");
    const abandonedClosure = topLevelVerdict === "ABANDONED" || /^Validated\s*\(\s*ABANDONED\s*\)$/i.test(statusValue);
    if (abandonedClosure) {
      if (topLevelVerdict !== "ABANDONED") {
        fail("Validated (ABANDONED) requires VALIDATION_REPORTS top-level Verdict: ABANDONED");
      }
      if (!/^Validated\s*\(\s*ABANDONED\s*\)$/i.test(statusValue)) {
        fail("Verdict=ABANDONED requires packet Status: Validated (ABANDONED)");
      }
      if (disposition !== "ABANDONED") {
        fail("Verdict=ABANDONED requires DISPOSITION=ABANDONED");
      }
    }

    if (activeWorkflowInvalidity && topLevelVerdict === "PASS") {
      fail(
        `Verdict=PASS prohibited when active WORKFLOW_INVALIDITY receipt exists (${activeWorkflowInvalidity?.workflow_invalidity_code || "UNKNOWN"}: ${activeWorkflowInvalidity?.summary || "<missing>"})`
      );
    }
    if (usesHeuristicRigorReport && specAlignmentVerdict === "PASS" && !hasOnlyNoneList(mainBodyGaps)) {
      fail("SPEC_ALIGNMENT_VERDICT=PASS requires MAIN_BODY_GAPS to be exactly '- NONE'");
    }
    if (dataContractProfile === "LLM_FIRST_DATA_V1" && specAlignmentVerdict === "PASS" && !hasOnlyNoneList(dataContractGaps)) {
      fail("SPEC_ALIGNMENT_VERDICT=PASS requires DATA_CONTRACT_GAPS to be exactly '- NONE' for active data contract packet");
    }
    if (usesCompletionLayerVerdicts) {
      const workflowValidityMatch = validationReports.match(/^\s*WORKFLOW_VALIDITY\s*:\s*(.+)\s*$/im);
      const workflowValidity = workflowValidityMatch ? (workflowValidityMatch[1] || "").trim().toUpperCase() : "";
      const scopeValidityMatch = validationReports.match(/^\s*SCOPE_VALIDITY\s*:\s*(.+)\s*$/im);
      const scopeValidity = scopeValidityMatch ? (scopeValidityMatch[1] || "").trim().toUpperCase() : "";
      const proofCompletenessMatch = validationReports.match(/^\s*PROOF_COMPLETENESS\s*:\s*(.+)\s*$/im);
      const proofCompleteness = proofCompletenessMatch ? (proofCompletenessMatch[1] || "").trim().toUpperCase() : "";
      const integrationReadinessMatch = validationReports.match(/^\s*INTEGRATION_READINESS\s*:\s*(.+)\s*$/im);
      const integrationReadiness = integrationReadinessMatch ? (integrationReadinessMatch[1] || "").trim().toUpperCase() : "";
      const domainGoalCompletionMatch = validationReports.match(/^\s*DOMAIN_GOAL_COMPLETION\s*:\s*(.+)\s*$/im);
      const domainGoalCompletion = domainGoalCompletionMatch ? (domainGoalCompletionMatch[1] || "").trim().toUpperCase() : "";

      if (workflowValidity === "VALID" && validationContext !== "OK") {
        fail("WORKFLOW_VALIDITY=VALID requires VALIDATION_CONTEXT=OK");
      }
      if (workflowValidity === "VALID" && governanceVerdict !== "PASS") {
        fail("WORKFLOW_VALIDITY=VALID requires GOVERNANCE_VERDICT=PASS");
      }
      if (activeWorkflowInvalidity) {
        if (workflowValidity === "VALID") {
          fail(
            `WORKFLOW_VALIDITY=VALID prohibited when active WORKFLOW_INVALIDITY receipt exists (${activeWorkflowInvalidity?.workflow_invalidity_code || "UNKNOWN"}: ${activeWorkflowInvalidity?.summary || "<missing>"})`
          );
        }
        if (governanceVerdict === "PASS") {
          fail(
            `GOVERNANCE_VERDICT=PASS prohibited when active WORKFLOW_INVALIDITY receipt exists (${activeWorkflowInvalidity?.workflow_invalidity_code || "UNKNOWN"}: ${activeWorkflowInvalidity?.summary || "<missing>"})`
          );
        }
      }
      if (proofCompleteness === "PROVEN" && !hasOnlyNoneList(notProvenItems)) {
        fail("PROOF_COMPLETENESS=PROVEN requires NOT_PROVEN to be exactly '- NONE'");
      }
      if (legalVerdict === "PASS" && proofCompleteness !== "PROVEN") {
        fail("LEGAL_VERDICT=PASS requires PROOF_COMPLETENESS=PROVEN");
      }
      if (topLevelVerdict === "PASS") {
        if (validationContext !== "OK") fail("Verdict=PASS requires VALIDATION_CONTEXT=OK");
        if (workflowValidity !== "VALID") fail("Verdict=PASS requires WORKFLOW_VALIDITY=VALID");
        if (scopeValidity !== "IN_SCOPE") fail("Verdict=PASS requires SCOPE_VALIDITY=IN_SCOPE");
        if (proofCompleteness !== "PROVEN") fail("Verdict=PASS requires PROOF_COMPLETENESS=PROVEN");
        if (integrationReadiness !== "READY") fail("Verdict=PASS requires INTEGRATION_READINESS=READY");
        if (domainGoalCompletion !== "COMPLETE") fail("Verdict=PASS requires DOMAIN_GOAL_COMPLETION=COMPLETE");
        if (legalVerdict !== "PASS") fail("Verdict=PASS requires LEGAL_VERDICT=PASS");
        if (environmentVerdict !== "PASS") fail("Verdict=PASS requires ENVIRONMENT_VERDICT=PASS");
        if (disposition !== "NONE") fail("Verdict=PASS requires DISPOSITION=NONE");
        const signedScopeCompatibilityForPass = validateSignedScopeCompatibilityTruth(text, {
          packetPath,
          requireReadyForPass: true,
        });
        if (signedScopeCompatibilityForPass.errors.length > 0) {
          fail(`Verdict=PASS requires signed scope compatibility truth to be PASS-ready: ${signedScopeCompatibilityForPass.errors.join("; ")}`);
        }
        for (const item of negativeProofItems) {
          if (!hasConcreteCodeReference(item) || /\.GOV\/|gov_runtime\/|TASK_BOARD|RUNTIME_STATUS|ROLE_SESSION_REGISTRY|SESSION_CONTROL|VALIDATOR_PROTOCOL|ORCHESTRATOR_PROTOCOL|COMMAND_SURFACE_REFERENCE|governance closeout|outside the signed product scope/i.test(item)) {
            fail(`Verdict=PASS requires NEGATIVE_PROOF to stay inside signed product scope with concrete product code evidence (${item})`);
          }
        }
        if (usesRigorV3Report && enforcesAntiVibeRigor && !hasOnlyNoneList(antiVibeFindings)) {
          fail("Verdict=PASS requires ANTI_VIBE_FINDINGS to be exactly '- NONE'");
        }
        if (usesRigorV3Report && enforcesAntiVibeRigor && !hasOnlyNoneList(signedScopeDebt)) {
          fail("Verdict=PASS requires SIGNED_SCOPE_DEBT to be exactly '- NONE'");
        }
      }
    }
    if (usesHeuristicRigorReport && heuristicReviewVerdict === "PASS" && !hasOnlyNoneList(qualityRisks)) {
      fail("HEURISTIC_REVIEW_VERDICT=PASS requires QUALITY_RISKS to be exactly '- NONE'");
    }
    if (usesRigorV3Report && enforcesAntiVibeRigor && heuristicReviewVerdict === "PASS" && !hasOnlyNoneList(antiVibeFindings)) {
      fail("HEURISTIC_REVIEW_VERDICT=PASS requires ANTI_VIBE_FINDINGS to be exactly '- NONE'");
    }
    if (usesRigorV3Report && enforcesAntiVibeRigor && heuristicReviewVerdict === "PASS" && !hasOnlyNoneList(signedScopeDebt)) {
      fail("HEURISTIC_REVIEW_VERDICT=PASS requires SIGNED_SCOPE_DEBT to be exactly '- NONE'");
    }
    if (usesRigorV3Report) {
      const packetRiskTier = parseSingleField("RISK_TIER").toUpperCase();
      const validatorRiskTierMatch = validationReports.match(/^\s*VALIDATOR_RISK_TIER\s*:\s*(.+)\s*$/im);
      const validatorRiskTier = validatorRiskTierMatch ? (validatorRiskTierMatch[1] || "").trim().toUpperCase() : "";
      const validatorRiskTierRank = riskTierRank(validatorRiskTier);
      const packetRiskTierRank = riskTierRank(packetRiskTier);
      if (validatorRiskTierRank === 0) {
        fail("VALIDATOR_RISK_TIER must be LOW | MEDIUM | HIGH");
      }
      if (packetRiskTierRank > 0 && validatorRiskTierRank < packetRiskTierRank) {
        fail(`VALIDATOR_RISK_TIER must not be lower than packet RISK_TIER (${packetRiskTier})`);
      }

      const requiredIndependentChecks = validatorRiskTier === "HIGH" ? 2 : 1;
      const requiredCounterfactualChecks = validatorRiskTier === "HIGH" ? 2 : 1;
      if (independentChecks.length < requiredIndependentChecks) {
        fail(`VALIDATOR_RISK_TIER=${validatorRiskTier} requires at least ${requiredIndependentChecks} INDEPENDENT_CHECKS_RUN item(s)`);
      }
      if (counterfactualChecks.length < requiredCounterfactualChecks) {
        fail(`VALIDATOR_RISK_TIER=${validatorRiskTier} requires at least ${requiredCounterfactualChecks} COUNTERFACTUAL_CHECKS item(s)`);
      }
      if (validatorRiskTier === "HIGH" && hasOnlyNoneList(residualUncertainty)) {
        fail("VALIDATOR_RISK_TIER=HIGH requires RESIDUAL_UNCERTAINTY to list real remaining uncertainty");
      }
      if ((validatorRiskTier === "MEDIUM" || validatorRiskTier === "HIGH") && boundaryProbes.length === 0) {
        fail(`VALIDATOR_RISK_TIER=${validatorRiskTier} requires BOUNDARY_PROBES`);
      }
      if ((validatorRiskTier === "MEDIUM" || validatorRiskTier === "HIGH") && negativePathChecks.length === 0) {
        fail(`VALIDATOR_RISK_TIER=${validatorRiskTier} requires NEGATIVE_PATH_CHECKS`);
      }
      if (legalVerdict === "PASS") {
        if (attackSurfaces.length === 0) fail("LEGAL_VERDICT=PASS requires DIFF_ATTACK_SURFACES");
        if (independentChecks.length === 0) fail("LEGAL_VERDICT=PASS requires INDEPENDENT_CHECKS_RUN");
        if (counterfactualChecks.length === 0) fail("LEGAL_VERDICT=PASS requires COUNTERFACTUAL_CHECKS");
        if (enforcesAntiVibeRigor && !hasOnlyNoneList(antiVibeFindings)) fail("LEGAL_VERDICT=PASS requires ANTI_VIBE_FINDINGS to be exactly '- NONE'");
        if (enforcesAntiVibeRigor && !hasOnlyNoneList(signedScopeDebt)) fail("LEGAL_VERDICT=PASS requires SIGNED_SCOPE_DEBT to be exactly '- NONE'");
        if ((validatorRiskTier === "MEDIUM" || validatorRiskTier === "HIGH") && boundaryProbes.length === 0) {
          fail(`LEGAL_VERDICT=PASS requires BOUNDARY_PROBES for ${validatorRiskTier} risk`);
        }
        if ((validatorRiskTier === "MEDIUM" || validatorRiskTier === "HIGH") && negativePathChecks.length === 0) {
          fail(`LEGAL_VERDICT=PASS requires NEGATIVE_PATH_CHECKS for ${validatorRiskTier} risk`);
        }
        for (const item of counterfactualChecks) {
          if (!hasConcreteCodeReference(item)) {
            fail(`LEGAL_VERDICT=PASS requires COUNTERFACTUAL_CHECKS entries to name a concrete code path or symbol (${item})`);
          }
        }
        if (packetRequiresSpecClauseMap(packetFormatVersion)) {
          const specClauseMapItems = extractListItemsAfterLabel(validationReports, "SPEC_CLAUSE_MAP");
          for (const item of specClauseMapItems) {
            if (!hasConcreteCodeReference(item)) {
              fail(`LEGAL_VERDICT=PASS requires SPEC_CLAUSE_MAP entries to include file:line evidence (${item})`);
            }
          }
        }
        if (dataContractProfile === "LLM_FIRST_DATA_V1") {
          if (dataContractProof.length === 0) {
            fail("LEGAL_VERDICT=PASS requires DATA_CONTRACT_PROOF for active data contract packet");
          }
          if (!hasOnlyNoneList(dataContractGaps)) {
            fail("LEGAL_VERDICT=PASS requires DATA_CONTRACT_GAPS to be exactly '- NONE' for active data contract packet");
          }
          for (const item of dataContractProof) {
            if (!hasConcreteCodeReference(item)) {
              fail(`LEGAL_VERDICT=PASS requires DATA_CONTRACT_PROOF entries to include concrete code or query evidence (${item})`);
            }
          }
        }
      }
    }
    if (usesClauseClosureMonitor && specAlignmentVerdict === "PASS") {
      const passConsistency = validatePacketClosureMonitoring(text, {
        requireRows: true,
        requireClosedConsistency: true,
        requirePassConsistency: true,
      });
      if (passConsistency.errors.length > 0) {
        fail(`SPEC pass closure monitoring invalid: ${passConsistency.errors.join("; ")}`);
      }
    }

    if (computedPolicy.applicable && !computedPolicyOutcomeAllowsClosure(computedPolicy) && !abandonedClosure) {
      const details = [
        ...computedPolicy.issues.fail,
        ...computedPolicy.issues.blocked,
        ...computedPolicy.issues.reviewRequired,
      ].map((item) => `${item.code}: ${item.message}`);
      fail(`computed policy gate outcome ${computedPolicy.outcome}${details.length > 0 ? ` (${details.join("; ")})` : ""}`);
    }
  }
}

console.log(`validator-packet-complete: PASS - ${wpId} has required fields.`);
