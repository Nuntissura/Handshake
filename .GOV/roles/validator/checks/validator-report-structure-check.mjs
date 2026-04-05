import fs from "node:fs";
import path from "node:path";
import {
  packetRequiresCompletionLayerVerdicts,
  packetRequiresSpecClauseMap,
  packetUsesStructuredValidationReport,
} from "../../../roles_shared/scripts/session/session-policy.mjs";
import { GOV_ROOT_REPO_REL, listOfficialWorkPacketPaths } from "../../../roles_shared/scripts/lib/runtime-paths.mjs";
import {
  packetUsesDataContractProfile,
  parseDataContractProfile,
  validateDataContractDecisionSection,
  validateDataContractSection,
} from "../../../roles_shared/scripts/lib/data-contract-lib.mjs";
import { parsePacketScopeList } from "../../../roles_shared/scripts/lib/scope-surface-lib.mjs";
import {
  validatorReportProfileRequiresAntiVibe,
  validatorReportProfileRequiresPrimitiveAudit,
  validatorReportProfileRequiresRiskAudit,
  validatorReportProfileUsesHeuristicRigor,
} from "../../../roles_shared/scripts/lib/validator-report-profile-lib.mjs";

function fail(message, details = []) {
  console.error(`[VALIDATOR_REPORT_STRUCTURE_CHECK] ${message}`);
  for (const line of details) console.error(`  - ${line}`);
  process.exit(1);
}

function parseSingleField(text, label) {
  const re = new RegExp(`^\\s*-\\s*(?:\\*\\*)?${label}(?:\\*\\*)?\\s*:\\s*(.+)\\s*$`, "mi");
  const match = text.match(re);
  return match ? match[1].trim() : "";
}

function parseStatus(text) {
  return (
    (text.match(/^\s*-\s*\*\*Status:\*\*\s*(.+)\s*$/mi) || [])[1] ||
    (text.match(/^\s*\*\*Status:\*\*\s*(.+)\s*$/mi) || [])[1] ||
    (text.match(/^\s*Status:\s*(.+)\s*$/mi) || [])[1] ||
    ""
  ).trim();
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

function negativeProofLeaksToGovernance(value) {
  return /\.GOV\/|gov_runtime\/|TASK_BOARD|RUNTIME_STATUS|ROLE_SESSION_REGISTRY|SESSION_CONTROL|VALIDATOR_PROTOCOL|ORCHESTRATOR_PROTOCOL|COMMAND_SURFACE_REFERENCE|governance closeout|outside the signed product scope/i.test(String(value || ""));
}

function lacksConcreteListEvidence(items = []) {
  return items.some((item) => !/^NONE$/i.test(String(item || "").trim()) && !hasConcreteCodeReference(item));
}

function isClosedStatus(status) {
  return /\b(done|validated)\b/i.test(String(status || ""));
}

const files = listOfficialWorkPacketPaths();
if (files.length === 0) {
  fail("No official work packets found", [path.join(GOV_ROOT_REPO_REL, "task_packets").replace(/\\/g, "/")]);
}
const violations = [];

for (const rel of files) {
  const text = fs.readFileSync(rel, "utf8");
  const packetFormatVersion = parseSingleField(text, "PACKET_FORMAT_VERSION");
  if (!packetUsesStructuredValidationReport(packetFormatVersion)) continue;
  const usesDataContractProfile = packetUsesDataContractProfile(packetFormatVersion);
  const dataContractProfile = parseDataContractProfile(text);
  const inScopePaths = parsePacketScopeList(text, "IN_SCOPE_PATHS", { stopLabels: ["OUT_OF_SCOPE"] });
  const reportProfile = parseSingleField(text, "GOVERNED_VALIDATOR_REPORT_PROFILE");
  const packetRiskTier = parseSingleField(text, "RISK_TIER").toUpperCase();
  const currentMainCompatibilityStatus = parseSingleField(text, "CURRENT_MAIN_COMPATIBILITY_STATUS").toUpperCase();
  const sharedSurfaceRisk = parseSingleField(text, "SHARED_SURFACE_RISK").toUpperCase();
  const requiresHeuristicRigor = validatorReportProfileUsesHeuristicRigor(reportProfile);
  const requiresRiskAudit = validatorReportProfileRequiresRiskAudit(reportProfile);
  const requiresPrimitiveAudit = validatorReportProfileRequiresPrimitiveAudit(reportProfile);
  const enforcesAntiVibeRigor = validatorReportProfileRequiresAntiVibe(reportProfile, packetFormatVersion);
  const requiresCompletionLayerVerdicts = packetRequiresCompletionLayerVerdicts(packetFormatVersion);

  const status = parseStatus(text);
  if (!isClosedStatus(status)) continue;

  if (usesDataContractProfile) {
    const dataContractDecisionValidation = validateDataContractDecisionSection(text, {
      packetPath: rel,
      inScopePaths,
    });
    violations.push(...dataContractDecisionValidation.errors);
    const dataContractValidation = validateDataContractSection(text, { packetPath: rel });
    violations.push(...dataContractValidation.errors);
  }

  const reports = extractSectionAfterHeading(text, "VALIDATION_REPORTS");
  if (!reports.trim()) {
    violations.push(`${rel}: VALIDATION_REPORTS missing/empty for closed packet`);
    continue;
  }

  const requiredFields = [
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
  if (requiresHeuristicRigor) {
    requiredFields.splice(4, 0, "HEURISTIC_REVIEW_VERDICT");
  }
  if (requiresRiskAudit) {
    requiredFields.push("VALIDATOR_RISK_TIER");
  }
  if (requiresCompletionLayerVerdicts) {
    requiredFields.push(
      "WORKFLOW_VALIDITY",
      "SCOPE_VALIDITY",
      "PROOF_COMPLETENESS",
      "INTEGRATION_READINESS",
      "DOMAIN_GOAL_COMPLETION",
    );
  }

  for (const label of requiredFields) {
    const value = parseSectionField(reports, label);
    if (!value) {
      violations.push(`${rel}: ${label} missing in VALIDATION_REPORTS`);
    }
  }

  if (!/^\s*Verdict\s*:\s*(PASS|FAIL|NOT_PROVEN|OUTDATED_ONLY|ABANDONED|BLOCKED)\b/im.test(reports)) {
    violations.push(`${rel}: VALIDATION_REPORTS missing top-level Verdict: PASS|FAIL|NOT_PROVEN|OUTDATED_ONLY|ABANDONED|BLOCKED`);
  }

  const clausesReviewed = extractListItemsAfterLabel(reports, "CLAUSES_REVIEWED");
  if (clausesReviewed.length === 0) {
    violations.push(`${rel}: CLAUSES_REVIEWED missing bullet items in VALIDATION_REPORTS`);
  }

  const notProven = extractListItemsAfterLabel(reports, "NOT_PROVEN");
  if (notProven.length === 0) {
    violations.push(`${rel}: NOT_PROVEN missing bullet items in VALIDATION_REPORTS`);
  }

  const mainBodyGaps = extractListItemsAfterLabel(reports, "MAIN_BODY_GAPS");
  const qualityRisks = extractListItemsAfterLabel(reports, "QUALITY_RISKS");
  if (requiresHeuristicRigor && mainBodyGaps.length === 0) {
    violations.push(`${rel}: MAIN_BODY_GAPS missing bullet items in VALIDATION_REPORTS`);
  }
  if (requiresHeuristicRigor && qualityRisks.length === 0) {
    violations.push(`${rel}: QUALITY_RISKS missing bullet items in VALIDATION_REPORTS`);
  }
  const attackSurfaces = extractListItemsAfterLabel(reports, "DIFF_ATTACK_SURFACES");
  const independentChecks = extractListItemsAfterLabel(reports, "INDEPENDENT_CHECKS_RUN");
  const counterfactualChecks = extractListItemsAfterLabel(reports, "COUNTERFACTUAL_CHECKS");
  const independentFindings = extractListItemsAfterLabel(reports, "INDEPENDENT_FINDINGS");
  const residualUncertainty = extractListItemsAfterLabel(reports, "RESIDUAL_UNCERTAINTY");
  const boundaryProbes = extractListItemsAfterLabel(reports, "BOUNDARY_PROBES");
  const negativePathChecks = extractListItemsAfterLabel(reports, "NEGATIVE_PATH_CHECKS");
  const specClauseMap = extractListItemsAfterLabel(reports, "SPEC_CLAUSE_MAP");
  const negativeProof = extractListItemsAfterLabel(reports, "NEGATIVE_PROOF");
  const antiVibeFindings = extractListItemsAfterLabel(reports, "ANTI_VIBE_FINDINGS");
  const signedScopeDebt = extractListItemsAfterLabel(reports, "SIGNED_SCOPE_DEBT");
  const dataContractProof = extractListItemsAfterLabel(reports, "DATA_CONTRACT_PROOF");
  const dataContractGaps = extractListItemsAfterLabel(reports, "DATA_CONTRACT_GAPS");
  if (requiresRiskAudit && attackSurfaces.length === 0) {
    violations.push(`${rel}: DIFF_ATTACK_SURFACES missing bullet items in VALIDATION_REPORTS`);
  }
  if (requiresRiskAudit && independentChecks.length === 0) {
    violations.push(`${rel}: INDEPENDENT_CHECKS_RUN missing bullet items in VALIDATION_REPORTS`);
  }
  if (requiresRiskAudit && counterfactualChecks.length === 0) {
    violations.push(`${rel}: COUNTERFACTUAL_CHECKS missing bullet items in VALIDATION_REPORTS`);
  }
  if (requiresRiskAudit && independentFindings.length === 0) {
    violations.push(`${rel}: INDEPENDENT_FINDINGS missing bullet items in VALIDATION_REPORTS`);
  }
  if (requiresRiskAudit && residualUncertainty.length === 0) {
    violations.push(`${rel}: RESIDUAL_UNCERTAINTY missing bullet items in VALIDATION_REPORTS`);
  }
  if (requiresRiskAudit && enforcesAntiVibeRigor && antiVibeFindings.length === 0) {
    violations.push(`${rel}: ANTI_VIBE_FINDINGS missing bullet items in VALIDATION_REPORTS`);
  }
  if (requiresRiskAudit && enforcesAntiVibeRigor && signedScopeDebt.length === 0) {
    violations.push(`${rel}: SIGNED_SCOPE_DEBT missing bullet items in VALIDATION_REPORTS`);
  }
  const primitiveRetentionProof = extractListItemsAfterLabel(reports, "PRIMITIVE_RETENTION_PROOF");
  const primitiveRetentionGaps = extractListItemsAfterLabel(reports, "PRIMITIVE_RETENTION_GAPS");
  const sharedSurfaceInteractionChecks = extractListItemsAfterLabel(reports, "SHARED_SURFACE_INTERACTION_CHECKS");
  const currentMainInteractionChecks = extractListItemsAfterLabel(reports, "CURRENT_MAIN_INTERACTION_CHECKS");
  if (requiresPrimitiveAudit && primitiveRetentionProof.length === 0) {
    violations.push(`${rel}: PRIMITIVE_RETENTION_PROOF missing bullet items in VALIDATION_REPORTS`);
  }
  if (requiresPrimitiveAudit && primitiveRetentionGaps.length === 0) {
    violations.push(`${rel}: PRIMITIVE_RETENTION_GAPS missing bullet items in VALIDATION_REPORTS`);
  }
  if (requiresPrimitiveAudit && sharedSurfaceInteractionChecks.length === 0) {
    violations.push(`${rel}: SHARED_SURFACE_INTERACTION_CHECKS missing bullet items in VALIDATION_REPORTS`);
  }
  if (requiresPrimitiveAudit && currentMainInteractionChecks.length === 0) {
    violations.push(`${rel}: CURRENT_MAIN_INTERACTION_CHECKS missing bullet items in VALIDATION_REPORTS`);
  }
  if (dataContractProfile === "LLM_FIRST_DATA_V1" && (dataContractProof.length === 0 || hasOnlyNoneList(dataContractProof))) {
    violations.push(`${rel}: DATA_CONTRACT_PROOF must list concrete proof items in VALIDATION_REPORTS for active data contract packet`);
  }
  if (dataContractProfile === "LLM_FIRST_DATA_V1" && dataContractGaps.length === 0) {
    violations.push(`${rel}: DATA_CONTRACT_GAPS missing bullet items in VALIDATION_REPORTS for active data contract packet`);
  }

  const specAlignmentVerdict = parseSectionField(reports, "SPEC_ALIGNMENT_VERDICT").toUpperCase();
  if (specAlignmentVerdict === "PASS") {
    if (!hasOnlyNoneList(notProven)) {
      violations.push(`${rel}: SPEC_ALIGNMENT_VERDICT=PASS requires NOT_PROVEN to be exactly "- NONE"`);
    }
    if (requiresHeuristicRigor && !hasOnlyNoneList(mainBodyGaps)) {
      violations.push(`${rel}: SPEC_ALIGNMENT_VERDICT=PASS requires MAIN_BODY_GAPS to be exactly "- NONE"`);
    }
    if (dataContractProfile === "LLM_FIRST_DATA_V1" && !hasOnlyNoneList(dataContractGaps)) {
      violations.push(`${rel}: SPEC_ALIGNMENT_VERDICT=PASS requires DATA_CONTRACT_GAPS to be exactly "- NONE" for active data contract packet`);
    }
  }

  const topLevelVerdict = parseSectionField(reports, "Verdict").toUpperCase();
  const validationContext = parseSectionField(reports, "VALIDATION_CONTEXT").toUpperCase();
  const governanceVerdict = parseSectionField(reports, "GOVERNANCE_VERDICT").toUpperCase();
  const environmentVerdict = parseSectionField(reports, "ENVIRONMENT_VERDICT").toUpperCase();
  const disposition = parseSectionField(reports, "DISPOSITION").toUpperCase();
  const legalVerdict = parseSectionField(reports, "LEGAL_VERDICT").toUpperCase();
  const heuristicReviewVerdict = parseSectionField(reports, "HEURISTIC_REVIEW_VERDICT").toUpperCase();
  if (topLevelVerdict === "ABANDONED") {
    if (!/^Validated\s*\(\s*ABANDONED\s*\)$/i.test(status)) {
      violations.push(`${rel}: Verdict=ABANDONED requires packet Status: Validated (ABANDONED)`);
    }
    if (disposition !== "ABANDONED") {
      violations.push(`${rel}: Verdict=ABANDONED requires DISPOSITION=ABANDONED`);
    }
  }

  if (requiresCompletionLayerVerdicts) {
    const workflowValidity = parseSectionField(reports, "WORKFLOW_VALIDITY").toUpperCase();
    const scopeValidity = parseSectionField(reports, "SCOPE_VALIDITY").toUpperCase();
    const proofCompleteness = parseSectionField(reports, "PROOF_COMPLETENESS").toUpperCase();
    const integrationReadiness = parseSectionField(reports, "INTEGRATION_READINESS").toUpperCase();
    const domainGoalCompletion = parseSectionField(reports, "DOMAIN_GOAL_COMPLETION").toUpperCase();

    if (workflowValidity === "VALID" && validationContext !== "OK") {
      violations.push(`${rel}: WORKFLOW_VALIDITY=VALID requires VALIDATION_CONTEXT=OK`);
    }
    if (workflowValidity === "VALID" && governanceVerdict !== "PASS") {
      violations.push(`${rel}: WORKFLOW_VALIDITY=VALID requires GOVERNANCE_VERDICT=PASS`);
    }
    if (proofCompleteness === "PROVEN" && !hasOnlyNoneList(notProven)) {
      violations.push(`${rel}: PROOF_COMPLETENESS=PROVEN requires NOT_PROVEN to be exactly "- NONE"`);
    }
    if (legalVerdict === "PASS" && proofCompleteness !== "PROVEN") {
      violations.push(`${rel}: LEGAL_VERDICT=PASS requires PROOF_COMPLETENESS=PROVEN`);
    }
    if (topLevelVerdict === "PASS") {
      if (validationContext !== "OK") {
        violations.push(`${rel}: Verdict=PASS requires VALIDATION_CONTEXT=OK`);
      }
      if (workflowValidity !== "VALID") {
        violations.push(`${rel}: Verdict=PASS requires WORKFLOW_VALIDITY=VALID`);
      }
      if (scopeValidity !== "IN_SCOPE") {
        violations.push(`${rel}: Verdict=PASS requires SCOPE_VALIDITY=IN_SCOPE`);
      }
      if (proofCompleteness !== "PROVEN") {
        violations.push(`${rel}: Verdict=PASS requires PROOF_COMPLETENESS=PROVEN`);
      }
      if (integrationReadiness !== "READY") {
        violations.push(`${rel}: Verdict=PASS requires INTEGRATION_READINESS=READY`);
      }
      if (domainGoalCompletion !== "COMPLETE") {
        violations.push(`${rel}: Verdict=PASS requires DOMAIN_GOAL_COMPLETION=COMPLETE`);
      }
      if (legalVerdict !== "PASS") {
        violations.push(`${rel}: Verdict=PASS requires LEGAL_VERDICT=PASS`);
      }
      if (environmentVerdict !== "PASS") {
        violations.push(`${rel}: Verdict=PASS requires ENVIRONMENT_VERDICT=PASS`);
      }
      if (disposition !== "NONE") {
        violations.push(`${rel}: Verdict=PASS requires DISPOSITION=NONE`);
      }
    }
  }

  if (requiresHeuristicRigor && heuristicReviewVerdict === "PASS" && !hasOnlyNoneList(qualityRisks)) {
    violations.push(`${rel}: HEURISTIC_REVIEW_VERDICT=PASS requires QUALITY_RISKS to be exactly "- NONE"`);
  }
  if (requiresRiskAudit && enforcesAntiVibeRigor && heuristicReviewVerdict === "PASS" && !hasOnlyNoneList(antiVibeFindings)) {
    violations.push(`${rel}: HEURISTIC_REVIEW_VERDICT=PASS requires ANTI_VIBE_FINDINGS to be exactly "- NONE"`);
  }
  if (requiresRiskAudit && enforcesAntiVibeRigor && heuristicReviewVerdict === "PASS" && !hasOnlyNoneList(signedScopeDebt)) {
    violations.push(`${rel}: HEURISTIC_REVIEW_VERDICT=PASS requires SIGNED_SCOPE_DEBT to be exactly "- NONE"`);
  }

  if (requiresRiskAudit) {
    const validatorRiskTier = parseSectionField(reports, "VALIDATOR_RISK_TIER").toUpperCase();
    const validatorRiskTierRank = riskTierRank(validatorRiskTier);
    const packetRiskTierRank = riskTierRank(packetRiskTier);
    if (validatorRiskTierRank === 0) {
      violations.push(`${rel}: VALIDATOR_RISK_TIER must be LOW | MEDIUM | HIGH`);
    }
    if (packetRiskTierRank > 0 && validatorRiskTierRank > 0 && validatorRiskTierRank < packetRiskTierRank) {
      violations.push(`${rel}: VALIDATOR_RISK_TIER must not be lower than packet RISK_TIER (${packetRiskTier})`);
    }

    const requiredIndependentChecks = validatorRiskTier === "HIGH" ? 2 : 1;
    const requiredCounterfactualChecks = validatorRiskTier === "HIGH" ? 2 : 1;
    if (validatorRiskTierRank > 0 && independentChecks.length < requiredIndependentChecks) {
      violations.push(
        `${rel}: VALIDATOR_RISK_TIER=${validatorRiskTier} requires at least ${requiredIndependentChecks} INDEPENDENT_CHECKS_RUN item(s)`,
      );
    }
    if (validatorRiskTierRank > 0 && counterfactualChecks.length < requiredCounterfactualChecks) {
      violations.push(
        `${rel}: VALIDATOR_RISK_TIER=${validatorRiskTier} requires at least ${requiredCounterfactualChecks} COUNTERFACTUAL_CHECKS item(s)`,
      );
    }
    if (validatorRiskTier === "HIGH" && hasOnlyNoneList(residualUncertainty)) {
      violations.push(`${rel}: VALIDATOR_RISK_TIER=HIGH requires RESIDUAL_UNCERTAINTY to list real remaining uncertainty`);
    }
    if ((validatorRiskTier === "MEDIUM" || validatorRiskTier === "HIGH") && boundaryProbes.length === 0) {
      violations.push(`${rel}: VALIDATOR_RISK_TIER=${validatorRiskTier} requires BOUNDARY_PROBES`);
    }
    if ((validatorRiskTier === "MEDIUM" || validatorRiskTier === "HIGH") && negativePathChecks.length === 0) {
      violations.push(`${rel}: VALIDATOR_RISK_TIER=${validatorRiskTier} requires NEGATIVE_PATH_CHECKS`);
    }

    if (legalVerdict === "PASS") {
      if (attackSurfaces.length === 0) {
        violations.push(`${rel}: LEGAL_VERDICT=PASS requires DIFF_ATTACK_SURFACES`);
      }
      if (independentChecks.length === 0) {
        violations.push(`${rel}: LEGAL_VERDICT=PASS requires INDEPENDENT_CHECKS_RUN`);
      }
      if (counterfactualChecks.length === 0) {
        violations.push(`${rel}: LEGAL_VERDICT=PASS requires COUNTERFACTUAL_CHECKS`);
      }
      if (enforcesAntiVibeRigor && !hasOnlyNoneList(antiVibeFindings)) {
        violations.push(`${rel}: LEGAL_VERDICT=PASS requires ANTI_VIBE_FINDINGS to be exactly "- NONE"`);
      }
      if (enforcesAntiVibeRigor && !hasOnlyNoneList(signedScopeDebt)) {
        violations.push(`${rel}: LEGAL_VERDICT=PASS requires SIGNED_SCOPE_DEBT to be exactly "- NONE"`);
      }
      if ((validatorRiskTier === "MEDIUM" || validatorRiskTier === "HIGH") && boundaryProbes.length === 0) {
        violations.push(`${rel}: LEGAL_VERDICT=PASS requires BOUNDARY_PROBES for ${validatorRiskTier} risk`);
      }
      if ((validatorRiskTier === "MEDIUM" || validatorRiskTier === "HIGH") && negativePathChecks.length === 0) {
        violations.push(`${rel}: LEGAL_VERDICT=PASS requires NEGATIVE_PATH_CHECKS for ${validatorRiskTier} risk`);
      }
      for (const item of counterfactualChecks) {
        if (!hasConcreteCodeReference(item)) {
          violations.push(
            `${rel}: LEGAL_VERDICT=PASS requires COUNTERFACTUAL_CHECKS entries to name a concrete code path or symbol (${item})`,
          );
        }
      }
      if (packetRequiresSpecClauseMap(packetFormatVersion)) {
        for (const item of specClauseMap) {
          if (!hasConcreteCodeReference(item)) {
            violations.push(
              `${rel}: LEGAL_VERDICT=PASS requires SPEC_CLAUSE_MAP entries to include file:line evidence (${item})`,
            );
          }
        }
      }
      for (const item of negativeProof) {
        if (!hasConcreteCodeReference(item) || negativeProofLeaksToGovernance(item)) {
          violations.push(
            `${rel}: LEGAL_VERDICT=PASS requires NEGATIVE_PROOF entries to stay inside signed product scope with concrete product code evidence (${item})`,
          );
        }
      }
      if (dataContractProfile === "LLM_FIRST_DATA_V1") {
        if (dataContractProof.length === 0) {
          violations.push(`${rel}: LEGAL_VERDICT=PASS requires DATA_CONTRACT_PROOF for active data contract packet`);
        }
        if (!hasOnlyNoneList(dataContractGaps)) {
          violations.push(`${rel}: LEGAL_VERDICT=PASS requires DATA_CONTRACT_GAPS to be exactly "- NONE" for active data contract packet`);
        }
        for (const item of dataContractProof) {
          if (!hasConcreteCodeReference(item)) {
            violations.push(`${rel}: LEGAL_VERDICT=PASS requires DATA_CONTRACT_PROOF entries to include concrete code or query evidence (${item})`);
          }
        }
      }
      if (requiresPrimitiveAudit) {
        if (!hasOnlyNoneList(primitiveRetentionGaps)) {
          violations.push(`${rel}: LEGAL_VERDICT=PASS requires PRIMITIVE_RETENTION_GAPS to be exactly "- NONE"`);
        }
        if (lacksConcreteListEvidence(primitiveRetentionProof)) {
          violations.push(`${rel}: LEGAL_VERDICT=PASS requires PRIMITIVE_RETENTION_PROOF entries to include concrete code or symbol evidence`);
        }
        if (lacksConcreteListEvidence(sharedSurfaceInteractionChecks)) {
          violations.push(`${rel}: LEGAL_VERDICT=PASS requires SHARED_SURFACE_INTERACTION_CHECKS entries to include concrete code or symbol evidence`);
        }
        if (lacksConcreteListEvidence(currentMainInteractionChecks)) {
          violations.push(`${rel}: LEGAL_VERDICT=PASS requires CURRENT_MAIN_INTERACTION_CHECKS entries to include concrete code or symbol evidence`);
        }
        if (packetRiskTierRank >= riskTierRank("MEDIUM") && (primitiveRetentionProof.length === 0 || hasOnlyNoneList(primitiveRetentionProof))) {
          violations.push(`${rel}: LEGAL_VERDICT=PASS requires non-empty PRIMITIVE_RETENTION_PROOF for packet RISK_TIER=${packetRiskTier}`);
        }
        if (packetRiskTierRank >= riskTierRank("MEDIUM") && (sharedSurfaceInteractionChecks.length === 0 || hasOnlyNoneList(sharedSurfaceInteractionChecks))) {
          violations.push(`${rel}: LEGAL_VERDICT=PASS requires non-empty SHARED_SURFACE_INTERACTION_CHECKS for packet RISK_TIER=${packetRiskTier}`);
        }
        if (packetRiskTierRank >= riskTierRank("MEDIUM") && (currentMainInteractionChecks.length === 0 || hasOnlyNoneList(currentMainInteractionChecks))) {
          violations.push(`${rel}: LEGAL_VERDICT=PASS requires non-empty CURRENT_MAIN_INTERACTION_CHECKS for packet RISK_TIER=${packetRiskTier}`);
        }
        if (sharedSurfaceRisk === "YES" && (sharedSurfaceInteractionChecks.length === 0 || hasOnlyNoneList(sharedSurfaceInteractionChecks))) {
          violations.push(`${rel}: LEGAL_VERDICT=PASS requires non-empty SHARED_SURFACE_INTERACTION_CHECKS when SHARED_SURFACE_RISK=YES`);
        }
        if (currentMainCompatibilityStatus === "PASS" && (currentMainInteractionChecks.length === 0 || hasOnlyNoneList(currentMainInteractionChecks))) {
          violations.push(`${rel}: LEGAL_VERDICT=PASS requires non-empty CURRENT_MAIN_INTERACTION_CHECKS when CURRENT_MAIN_COMPATIBILITY_STATUS=PASS`);
        }
      }
    }

    if (packetRequiresSpecClauseMap(packetFormatVersion) && specClauseMap.length === 0) {
      violations.push(`${rel}: SPEC_CLAUSE_MAP missing bullet items in VALIDATION_REPORTS (required for RIGOR_V3)`);
    }
    if (packetRequiresSpecClauseMap(packetFormatVersion) && (negativeProof.length === 0 || hasOnlyNoneList(negativeProof))) {
      violations.push(`${rel}: NEGATIVE_PROOF must list at least one spec requirement verified as NOT fully implemented (required for RIGOR_V3)`);
    }
    if (enforcesAntiVibeRigor && topLevelVerdict === "PASS" && !hasOnlyNoneList(antiVibeFindings)) {
      violations.push(`${rel}: Verdict=PASS requires ANTI_VIBE_FINDINGS to be exactly "- NONE"`);
    }
    if (enforcesAntiVibeRigor && topLevelVerdict === "PASS" && !hasOnlyNoneList(signedScopeDebt)) {
      violations.push(`${rel}: Verdict=PASS requires SIGNED_SCOPE_DEBT to be exactly "- NONE"`);
    }
    if (requiresPrimitiveAudit && topLevelVerdict === "PASS" && !hasOnlyNoneList(primitiveRetentionGaps)) {
      violations.push(`${rel}: Verdict=PASS requires PRIMITIVE_RETENTION_GAPS to be exactly "- NONE"`);
    }
  }
}

if (violations.length > 0) {
  fail("Structured validator report violations found", violations);
}

console.log("validator-report-structure-check ok");
