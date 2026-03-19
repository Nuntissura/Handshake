#!/usr/bin/env node
/**
 * Packet completeness checker for validators.
 * Ensures required fields are present and sane.
 */
import { readFileSync } from "node:fs";
import { GOV_ROOT_REPO_REL, resolveWorkPacketPath } from "../../../roles_shared/scripts/lib/runtime-paths.mjs";
import { packetUsesStructuredValidationReport, packetRequiresSpecClauseMap } from "../../../roles_shared/scripts/session/session-policy.mjs";
import { validateClauseReportConsistency, validatePacketClosureMonitoring } from "../../../roles_shared/scripts/lib/packet-closure-monitor-lib.mjs";
import { validateSemanticProofAssets } from "../../../roles_shared/scripts/lib/semantic-proof-lib.mjs";

const wpId = process.argv[2];
if (!wpId) {
  console.error("Usage: just validator-packet-complete WP-1-Example");
  process.exit(1);
}

const resolved = resolveWorkPacketPath(wpId);
const packetPath = resolved?.packetPath || `${GOV_ROOT_REPO_REL}/task_packets/${wpId}.md`;

function fail(msg) {
  console.error(`validator-packet-complete: FAIL - ${msg}`);
  process.exit(1);
}

let text;
try {
  text = readFileSync(packetPath, "utf8");
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

const statusMatch = text.match(/(?:\*\*Status:\*\*|STATUS:)\s*(Ready for Dev|In Progress|Blocked|Done(?:\s*\(Historical\))?|Validated\s*\((?:PASS|FAIL|OUTDATED_ONLY)\))\b/i);
if (!statusMatch) {
  fail("STATUS missing or invalid (must be Ready for Dev / In Progress / Blocked / Done / Done (Historical) / Validated (PASS|FAIL|OUTDATED_ONLY))");
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

  if (closureStatus && packetUsesStructuredValidationReport(packetFormatVersion)) {
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
    if (!/^\s*Verdict\s*:\s*(PASS|FAIL|OUTDATED_ONLY)\b/im.test(validationReports)) {
      fail("VALIDATION_REPORTS missing top-level Verdict: PASS|FAIL|OUTDATED_ONLY");
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
    const mainBodyGaps = extractListItemsAfterLabel(validationReports, "MAIN_BODY_GAPS");
    const qualityRisks = extractListItemsAfterLabel(validationReports, "QUALITY_RISKS");
    const attackSurfaces = extractListItemsAfterLabel(validationReports, "DIFF_ATTACK_SURFACES");
    const independentChecks = extractListItemsAfterLabel(validationReports, "INDEPENDENT_CHECKS_RUN");
    const counterfactualChecks = extractListItemsAfterLabel(validationReports, "COUNTERFACTUAL_CHECKS");
    const residualUncertainty = extractListItemsAfterLabel(validationReports, "RESIDUAL_UNCERTAINTY");
    const boundaryProbes = extractListItemsAfterLabel(validationReports, "BOUNDARY_PROBES");
    const negativePathChecks = extractListItemsAfterLabel(validationReports, "NEGATIVE_PATH_CHECKS");
    if (usesHeuristicRigorReport && specAlignmentVerdict === "PASS" && !hasOnlyNoneList(mainBodyGaps)) {
      fail("SPEC_ALIGNMENT_VERDICT=PASS requires MAIN_BODY_GAPS to be exactly '- NONE'");
    }
    if (usesHeuristicRigorReport && heuristicReviewVerdict === "PASS" && !hasOnlyNoneList(qualityRisks)) {
      fail("HEURISTIC_REVIEW_VERDICT=PASS requires QUALITY_RISKS to be exactly '- NONE'");
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
  }
}

console.log(`validator-packet-complete: PASS - ${wpId} has required fields.`);
