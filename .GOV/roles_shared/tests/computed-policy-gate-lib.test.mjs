import assert from "node:assert/strict";
import test from "node:test";
import {
  computedPolicyOutcomeAllowsClosure,
  evaluateComputedPolicyGateFromPacketText,
} from "../scripts/lib/computed-policy-gate-lib.mjs";

function packetFixture({
  packetFormatVersion = "2026-03-23",
  status = "Done",
  riskTier = "LOW",
  validatorReportProfile = "SPLIT_DIFF_SCOPED_RIGOR_V3",
  validatorRiskTier = "LOW",
  sharedSurfaceRisk = "NO",
  currentMainCompatibilityStatus = "NOT_RUN",
  waiverBlock = "- NONE",
  verdict = "PASS",
  governanceVerdict = "PASS",
  testVerdict = "PASS",
  codeReviewVerdict = "PASS",
  heuristicReviewVerdict = "PASS",
  specAlignmentVerdict = "PASS",
  environmentVerdict = "PASS",
  disposition = "NONE",
  legalVerdict = "PASS",
  workflowValidity = "VALID",
  scopeValidity = "IN_SCOPE",
  proofCompleteness = "PROVEN",
  integrationReadiness = "READY",
  domainGoalCompletion = "COMPLETE",
  mechanicalTrackVerdict = "",
  specRetentionTrackVerdict = "",
  notProvenBlock = "- NONE",
  boundaryProbeBlock = "- NONE",
  negativePathBlock = "- NONE",
  negativeProofBlock = "- Clause B remains outside this diff and is not fully implemented yet",
  primitiveRetentionProofBlock = "- retained `feature::apply_clause_a()` in `src/backend/feature.rs:10` and preserved caller `consumer::render()` in `src/frontend/consumer.rs:15`",
  primitiveRetentionGapsBlock = "- NONE",
  sharedSurfaceInteractionChecksBlock = "- checked producer `src/backend/feature.rs:10` against consumer `src/backend/shared_surface.rs:20` and confirmed payload compatibility",
  currentMainInteractionChecksBlock = "- replayed current-main caller `src/legacy/consumer.rs:12` against `src/backend/feature.rs:10` and confirmed behavior remains stable",
} = {}) {
  const hotFilesBlock = sharedSurfaceRisk === "YES"
    ? "  - src/backend/shared_surface.rs"
    : "  - NONE";
  const tripwireBlock = sharedSurfaceRisk === "YES"
    ? "  - cargo test shared_surface_tripwire"
    : "  - NONE";

  return `# Task Packet: WP-TEST-POLICY-v1

**Status:** ${status}

## METADATA
- WP_ID: WP-TEST-POLICY-v1
- PACKET_FORMAT_VERSION: ${packetFormatVersion}
- RISK_TIER: ${riskTier}
- GOVERNED_VALIDATOR_REPORT_PROFILE: ${validatorReportProfile}
- CURRENT_MAIN_COMPATIBILITY_STATUS: ${currentMainCompatibilityStatus}
- CLAUSE_CLOSURE_MONITOR_PROFILE: CLAUSE_MONITOR_V1
- SEMANTIC_PROOF_PROFILE: DIFF_SCOPED_SEMANTIC_V1

## CLAUSE_CLOSURE_MATRIX (AUTHORITATIVE SNAPSHOT; MUTABLE)
- CLAUSE_ROWS:
  - CLAUSE: Clause A | CODE_SURFACES: src/backend/feature.rs | TESTS: cargo test clause_a | EXAMPLES: NONE | DEBT_IDS: NONE | CODER_STATUS: PROVED | VALIDATOR_STATUS: CONFIRMED

## SPEC_DEBT_STATUS (AUTHORITATIVE SNAPSHOT; MUTABLE)
- OPEN_SPEC_DEBT: NO
- BLOCKING_SPEC_DEBT: NO
- DEBT_IDS: NONE

## SHARED_SURFACE_MONITORING (AUTHORITATIVE SNAPSHOT; MUTABLE)
- SHARED_SURFACE_RISK: ${sharedSurfaceRisk}
- HOT_FILES:
${hotFilesBlock}
- REQUIRED_TRIPWIRE_TESTS:
${tripwireBlock}
- POST_MERGE_SPOTCHECK_REQUIRED: ${sharedSurfaceRisk === "YES" ? "YES" : "NO"}

## SEMANTIC_PROOF_ASSETS (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- SEMANTIC_TRIPWIRE_TESTS:
  - cargo test clause_a
- CANONICAL_CONTRACT_EXAMPLES:
  - NONE

## WAIVERS GRANTED
${waiverBlock}

## VALIDATION_REPORTS
Verdict: ${verdict}
VALIDATION_CONTEXT: OK
GOVERNANCE_VERDICT: ${governanceVerdict}
TEST_VERDICT: ${testVerdict}
CODE_REVIEW_VERDICT: ${codeReviewVerdict}
HEURISTIC_REVIEW_VERDICT: ${heuristicReviewVerdict}
SPEC_ALIGNMENT_VERDICT: ${specAlignmentVerdict}
ENVIRONMENT_VERDICT: ${environmentVerdict}
DISPOSITION: ${disposition}
LEGAL_VERDICT: ${legalVerdict}
SPEC_CONFIDENCE: REVIEWED_DIFF_SCOPED
WORKFLOW_VALIDITY: ${workflowValidity}
SCOPE_VALIDITY: ${scopeValidity}
PROOF_COMPLETENESS: ${proofCompleteness}
INTEGRATION_READINESS: ${integrationReadiness}
DOMAIN_GOAL_COMPLETION: ${domainGoalCompletion}
${mechanicalTrackVerdict ? `MECHANICAL_TRACK_VERDICT: ${mechanicalTrackVerdict}\n` : ""}${specRetentionTrackVerdict ? `SPEC_RETENTION_TRACK_VERDICT: ${specRetentionTrackVerdict}\n` : ""}VALIDATOR_RISK_TIER: ${validatorRiskTier}
CLAUSES_REVIEWED:
- Clause A -> src/backend/feature.rs:10
NOT_PROVEN:
${notProvenBlock}
MAIN_BODY_GAPS:
- NONE
QUALITY_RISKS:
- NONE
DIFF_ATTACK_SURFACES:
- src/backend/feature.rs::apply_clause_a
INDEPENDENT_CHECKS_RUN:
- cargo test clause_a => PASS
COUNTERFACTUAL_CHECKS:
- If src/backend/feature.rs::apply_clause_a were removed, tests/clause_a.rs would fail
BOUNDARY_PROBES:
${boundaryProbeBlock}
NEGATIVE_PATH_CHECKS:
${negativePathBlock}
INDEPENDENT_FINDINGS:
- Validator confirmed Clause A at src/backend/feature.rs:10
RESIDUAL_UNCERTAINTY:
- NONE
SPEC_CLAUSE_MAP:
- Clause A -> src/backend/feature.rs:10
NEGATIVE_PROOF:
${negativeProofBlock}
PRIMITIVE_RETENTION_PROOF:
${primitiveRetentionProofBlock}
PRIMITIVE_RETENTION_GAPS:
${primitiveRetentionGapsBlock}
SHARED_SURFACE_INTERACTION_CHECKS:
${sharedSurfaceInteractionChecksBlock}
CURRENT_MAIN_INTERACTION_CHECKS:
${currentMainInteractionChecksBlock}`.trim();
}

test("computed policy gate returns PASS for fully proven closure", () => {
  const evaluation = evaluateComputedPolicyGateFromPacketText(packetFixture(), {
    wpId: "WP-TEST-POLICY-v1",
    requireClosedStatus: true,
  });

  assert.equal(evaluation.applicable, true);
  assert.equal(evaluation.outcome, "PASS");
  assert.equal(computedPolicyOutcomeAllowsClosure(evaluation), true);
});

test("computed policy gate returns REVIEW_REQUIRED for honest not-proven closure", () => {
  const evaluation = evaluateComputedPolicyGateFromPacketText(packetFixture({
    verdict: "NOT_PROVEN",
    specAlignmentVerdict: "PARTIAL",
    legalVerdict: "PENDING",
    proofCompleteness: "NOT_PROVEN",
    notProvenBlock: "- Clause A needs runtime proof at src/backend/feature.rs:10",
  }), {
    wpId: "WP-TEST-POLICY-v1",
    requireClosedStatus: true,
  });

  assert.equal(evaluation.outcome, "REVIEW_REQUIRED");
  assert.equal(computedPolicyOutcomeAllowsClosure(evaluation), false);
  assert.ok(evaluation.issues.reviewRequired.some((item) => item.code === "PROOF_COMPLETENESS_NOT_PROVEN"));
});

test("computed policy gate returns WAIVED when the only remaining issue is waiver-covered", () => {
  const evaluation = evaluateComputedPolicyGateFromPacketText(packetFixture({
    sharedSurfaceRisk: "YES",
    waiverBlock: "- WAIVER_ID: CX-TEST-1 | STATUS: ACTIVE | COVERS: PROTECTED_SURFACE | SCOPE: WP-TEST-POLICY-v1 | JUSTIFICATION: temporary probe deferral | APPROVER: USER | EXPIRES: after hardening",
  }), {
    wpId: "WP-TEST-POLICY-v1",
    requireClosedStatus: true,
  });

  assert.equal(evaluation.outcome, "WAIVED");
  assert.equal(computedPolicyOutcomeAllowsClosure(evaluation), true);
  assert.ok(evaluation.issues.waived.some((item) => item.code === "PROTECTED_SURFACE_PARTIAL"));
});

test("computed policy gate turns narrative PASS over proof gaps into FAIL", () => {
  const evaluation = evaluateComputedPolicyGateFromPacketText(packetFixture({
    verdict: "PASS",
    specAlignmentVerdict: "PARTIAL",
    proofCompleteness: "NOT_PROVEN",
    notProvenBlock: "- Clause A still lacks negative-path proof",
  }), {
    wpId: "WP-TEST-POLICY-v1",
    requireClosedStatus: true,
  });

  assert.equal(evaluation.outcome, "FAIL");
  assert.ok(evaluation.issues.fail.some((item) => item.code === "NARRATIVE_PASS_OVERRUN"));
});

test("computed policy gate flags closed structured pre-threshold packets for remediation instead of silent skip", () => {
  const evaluation = evaluateComputedPolicyGateFromPacketText(packetFixture({
    packetFormatVersion: "2026-03-18",
  }), {
    wpId: "WP-TEST-POLICY-v1",
    packetPath: ".GOV/task_packets/WP-TEST-POLICY-v1/packet.md",
    requireClosedStatus: true,
  });

  assert.equal(evaluation.applicable, false);
  assert.equal(evaluation.applicability_reason, "PRE_COMPLETION_LAYER_THRESHOLD");
  assert.equal(evaluation.legacy_remediation_required, true);
  assert.ok(evaluation.issues.blocked.some((item) => item.code === "LEGACY_CLOSED_PACKET_REMEDIATION_REQUIRED"));
  assert.equal(computedPolicyOutcomeAllowsClosure(evaluation), false);
});

test("computed policy gate fails V4 medium-risk closures that omit primitive-retention and interaction audit proof", () => {
  const evaluation = evaluateComputedPolicyGateFromPacketText(packetFixture({
    packetFormatVersion: "2026-04-01",
    riskTier: "MEDIUM",
    validatorReportProfile: "SPLIT_DIFF_SCOPED_RIGOR_V4",
    sharedSurfaceRisk: "YES",
    currentMainCompatibilityStatus: "PASS",
    primitiveRetentionProofBlock: "- NONE",
    sharedSurfaceInteractionChecksBlock: "- NONE",
    currentMainInteractionChecksBlock: "- NONE",
  }), {
    wpId: "WP-TEST-POLICY-v1",
    requireClosedStatus: true,
  });

  assert.equal(evaluation.outcome, "FAIL");
  assert.ok(evaluation.issues.fail.some((item) => item.code === "PRIMITIVE_RETENTION_AUDIT_MISSING"));
  assert.ok(evaluation.issues.fail.some((item) => item.code === "SHARED_SURFACE_INTERACTION_AUDIT_MISSING"));
  assert.ok(evaluation.issues.fail.some((item) => item.code === "CURRENT_MAIN_INTERACTION_AUDIT_MISSING"));
});

test("computed policy gate accepts V4 closures when primitive-retention and interaction proof is explicit", () => {
  const evaluation = evaluateComputedPolicyGateFromPacketText(packetFixture({
    packetFormatVersion: "2026-04-01",
    riskTier: "MEDIUM",
    validatorReportProfile: "SPLIT_DIFF_SCOPED_RIGOR_V4",
    sharedSurfaceRisk: "YES",
    currentMainCompatibilityStatus: "PASS",
    boundaryProbeBlock: "- checked producer `src/backend/feature.rs:10` against boundary consumer `src/backend/shared_surface.rs:20`",
    negativePathBlock: "- removed required field at `src/backend/shared_surface.rs:24` and confirmed guarded failure path stayed intact",
  }), {
    wpId: "WP-TEST-POLICY-v1",
    requireClosedStatus: true,
  });

  assert.equal(evaluation.outcome, "PASS");
  assert.equal(computedPolicyOutcomeAllowsClosure(evaluation), true);
});

test("computed policy gate fails dual-track packets that omit explicit track verdicts", () => {
  const evaluation = evaluateComputedPolicyGateFromPacketText(packetFixture({
    packetFormatVersion: "2026-04-05",
    riskTier: "MEDIUM",
    validatorReportProfile: "SPLIT_DIFF_SCOPED_RIGOR_V4",
    validatorRiskTier: "MEDIUM",
    sharedSurfaceRisk: "YES",
    currentMainCompatibilityStatus: "PASS",
    boundaryProbeBlock: "- checked producer `src/backend/feature.rs:10` against boundary consumer `src/backend/shared_surface.rs:20`",
    negativePathBlock: "- removed required field at `src/backend/shared_surface.rs:24` and confirmed guarded failure path stayed intact",
  }), {
    wpId: "WP-TEST-POLICY-v1",
    requireClosedStatus: true,
  });

  assert.equal(evaluation.outcome, "FAIL");
  assert.ok(evaluation.issues.blocked.some((item) => item.code === "MECHANICAL_TRACK_VERDICT_MISSING"));
  assert.ok(evaluation.issues.blocked.some((item) => item.code === "SPEC_RETENTION_TRACK_VERDICT_MISSING"));
  assert.ok(evaluation.issues.fail.some((item) => item.code === "PASS_TRACK_MECHANICAL_MISMATCH"));
  assert.ok(evaluation.issues.fail.some((item) => item.code === "PASS_TRACK_SPEC_RETENTION_MISMATCH"));
});

test("computed policy gate accepts dual-track V4 closures when both tracks are explicit and honest", () => {
  const evaluation = evaluateComputedPolicyGateFromPacketText(packetFixture({
    packetFormatVersion: "2026-04-05",
    riskTier: "MEDIUM",
    validatorReportProfile: "SPLIT_DIFF_SCOPED_RIGOR_V4",
    validatorRiskTier: "MEDIUM",
    sharedSurfaceRisk: "YES",
    currentMainCompatibilityStatus: "PASS",
    mechanicalTrackVerdict: "PASS",
    specRetentionTrackVerdict: "PASS",
    boundaryProbeBlock: "- checked producer `src/backend/feature.rs:10` against boundary consumer `src/backend/shared_surface.rs:20`",
    negativePathBlock: "- removed required field at `src/backend/shared_surface.rs:24` and confirmed guarded failure path stayed intact",
    negativeProofBlock: "- `src/backend/shared_surface.rs:24` still omits one fallback branch under malformed payloads",
  }), {
    wpId: "WP-TEST-POLICY-v1",
    requireClosedStatus: true,
  });

  assert.equal(evaluation.outcome, "PASS");
  assert.equal(computedPolicyOutcomeAllowsClosure(evaluation), true);
});

test("computed policy gate allows honest OUTDATED_ONLY terminal closure without treating non-pass fields as a regression", () => {
  const evaluation = evaluateComputedPolicyGateFromPacketText(packetFixture({
    packetFormatVersion: "2026-04-05",
    status: "Validated (OUTDATED_ONLY)",
    riskTier: "MEDIUM",
    validatorReportProfile: "SPLIT_DIFF_SCOPED_RIGOR_V4",
    validatorRiskTier: "MEDIUM",
    sharedSurfaceRisk: "YES",
    verdict: "OUTDATED_ONLY",
    testVerdict: "PARTIAL",
    specAlignmentVerdict: "PARTIAL",
    disposition: "OUTDATED_ONLY",
    legalVerdict: "FAIL",
    integrationReadiness: "NOT_READY",
    domainGoalCompletion: "PARTIAL",
    mechanicalTrackVerdict: "PARTIAL",
    specRetentionTrackVerdict: "PARTIAL",
    boundaryProbeBlock: "- checked producer `src/backend/feature.rs:10` against boundary consumer `src/backend/shared_surface.rs:20`",
    negativePathBlock: "- removed required field at `src/backend/shared_surface.rs:24` and confirmed guarded failure path stayed intact",
    negativeProofBlock: "- current local main blocks contained-main proof outside the signed diff at `src/backend/adjacent.rs:42`",
  }), {
    wpId: "WP-TEST-POLICY-v1",
    requireClosedStatus: true,
  });

  assert.equal(evaluation.outcome, "FAIL");
  assert.ok(evaluation.issues.fail.some((item) => item.code === "LEGAL_VERDICT_FAIL"));
  assert.ok(evaluation.issues.reviewRequired.some((item) => item.code === "DISPOSITION_OUTDATED_ONLY"));
  assert.equal(computedPolicyOutcomeAllowsClosure(evaluation), true);
});
