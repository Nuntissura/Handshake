#!/usr/bin/env node

import {
  buildPostWorkCommand,
  currentGitContext,
  escapeRegex,
  failWithContext,
  hasCommitSubject,
  inferWpIdFromPrepare,
  loadOrchestratorGateLogs,
  loadPacket,
  packetExists,
  packetPath,
  parseClaimField,
  parseCurrentWpStatus,
  parseStatus,
  printConfidence,
  printFindings,
  printLifecycle,
  printNextCommands,
  printOperatorAction,
  printState,
  sectionHasMaterialContent,
} from "../../../roles_shared/scripts/lib/role-resume-utils.mjs";
import { REPO_ROOT } from "../../../roles_shared/scripts/lib/runtime-paths.mjs";
import {
  classifyWpChangedPath,
  deriveWpScopeContract,
} from "../../../roles_shared/scripts/lib/scope-surface-lib.mjs";
import {
  deriveCoderResumeState,
  evaluateCoderPacketGovernanceState,
  loadCoderCommunicationState,
} from "./lib/coder-governance-lib.mjs";

function resolveWpId() {
  const provided = String(process.argv[2] || "").trim();
  const gitContext = currentGitContext();
  const logs = loadOrchestratorGateLogs();

  if (provided) return { wpId: provided, gitContext, confidence: "HIGH", confidenceDetail: "explicit" };

  const inferred = inferWpIdFromPrepare(logs, gitContext, gitContext.topLevel || REPO_ROOT);
  if (inferred.wpId) {
    return {
      wpId: inferred.wpId,
      gitContext,
      confidence: "HIGH",
      confidenceDetail: inferred.source,
    };
  }

  const nextCommands = inferred.candidates.length
    ? inferred.candidates.map((candidate) => `just coder-next ${candidate}`)
    : ["just coder-next WP-{ID}"];

  failWithContext({
    state: "Unable to infer the active WP for the current coder worktree.",
    findings: [
      `Current branch: ${gitContext.branch || "<unknown>"}`,
      `Current worktree: ${gitContext.topLevel || "<unknown>"}`,
      inferred.candidates.length
        ? `Ambiguous WP candidates from PREPARE: ${inferred.candidates.join(", ")}`
        : "No PREPARE entry matched the current branch/worktree.",
    ],
    nextCommands,
  });
}

function parseChangedPaths(statusPorcelain) {
  return String(statusPorcelain || "")
    .split(/\r?\n/)
    .map((line) => line.trimEnd())
    .filter(Boolean)
    .map((line) => line.slice(3).split(" -> ").at(-1)?.trim() || "")
    .filter(Boolean);
}

function summarizeDirtyTree(statusPorcelain, scopeContract) {
  const changedPaths = parseChangedPaths(statusPorcelain);
  const summary = {
    changedPaths,
    dirty: changedPaths.length > 0,
    governanceJunction: [],
    governanceCompanion: [],
    inScope: [],
    outOfScope: [],
  };

  for (const changedPath of changedPaths) {
    const classification = classifyWpChangedPath(changedPath, scopeContract);
    if (classification.kind === "GOVERNANCE_JUNCTION_DRIFT") {
      summary.governanceJunction.push(classification.path);
    } else if (classification.kind === "GOVERNANCE_COMPANION") {
      summary.governanceCompanion.push(classification.path);
    } else if (classification.allowed) {
      summary.inScope.push(classification.path);
    } else {
      summary.outOfScope.push(`${classification.path} (${classification.kind})`);
    }
  }

  return summary;
}

const { wpId, gitContext, confidence, confidenceDetail } = resolveWpId();

if (!packetExists(wpId)) {
  failWithContext({
    wpId,
    stage: "BOOTSTRAP",
    next: "STOP",
    confidence,
    confidenceDetail,
    state: "work packet is missing; coder work cannot resume deterministically.",
    findings: [`Expected packet: ${packetPath(wpId).replace(/\\/g, "/")}`],
    nextCommands: [
      `cat ORCHESTRATOR_GATES.json (in gov_runtime)`,
      `just orchestrator-next ${wpId}`,
    ],
  });
}

const packetContent = loadPacket(wpId);
const packetStatus = parseStatus(packetContent);
const currentWpStatus = parseCurrentWpStatus(packetContent);
const workflowLane = parseClaimField(packetContent, "WORKFLOW_LANE").toUpperCase();
const usesSkeletonCheckpointGate = workflowLane !== "ORCHESTRATOR_MANAGED";
const coderModel = parseClaimField(packetContent, "CODER_MODEL");
const bootstrapClaim = hasCommitSubject(`^docs: bootstrap claim \\[${escapeRegex(wpId)}\\]$`);
const skeletonCheckpoint = hasCommitSubject(`^docs: skeleton checkpoint \\[${escapeRegex(wpId)}\\]$`);
const skeletonApproved = hasCommitSubject(`^docs: skeleton approved \\[${escapeRegex(wpId)}\\]$`);
const implementationFilled = sectionHasMaterialContent(packetContent, "IMPLEMENTATION");
const hygieneFilled = sectionHasMaterialContent(packetContent, "HYGIENE");
const validationFilled = sectionHasMaterialContent(packetContent, "VALIDATION");
const scopeContract = deriveWpScopeContract({ wpId, packetContent });
const dirtySummary = summarizeDirtyTree(gitContext.statusPorcelain, scopeContract);
const dirtyTree = dirtySummary.dirty;
const postWorkCommand = buildPostWorkCommand(wpId, packetContent);
const currentWpStatusLower = currentWpStatus.toLowerCase();
const skeletonApprover =
  workflowLane === "ORCHESTRATOR_MANAGED" ? "Orchestrator/Validator/Operator" : "Validator/Operator";
const dirtyTreeFinding = !dirtyTree
  ? "Working tree dirty: no"
  : dirtySummary.outOfScope.length > 0
    ? `Working tree dirty: yes (${dirtySummary.outOfScope.length} out-of-scope path(s) require correction)`
    : dirtySummary.governanceJunction.length > 0
      ? `Working tree dirty: yes (shared .GOV junction drift only across ${dirtySummary.governanceJunction.length} path(s))`
      : `Working tree dirty: yes (${dirtySummary.inScope.length + dirtySummary.governanceCompanion.length} packet-scoped path(s))`;
const dirtyNoiseFindings = [
  dirtySummary.governanceJunction.length > 0
    ? `Shared .GOV junction drift: ${dirtySummary.governanceJunction.length} path(s) treated as read-only noise by default`
    : "",
  dirtySummary.governanceCompanion.length > 0
    ? `Governance companion paths touched: ${dirtySummary.governanceCompanion.length} (${dirtySummary.governanceCompanion.slice(0, 3).join(", ")})`
    : "",
  dirtySummary.outOfScope.length > 0
    ? `Out-of-scope changes detected: ${dirtySummary.outOfScope.slice(0, 3).join(", ")}`
    : "",
].filter(Boolean);

const commonFindings = [
  `Current branch: ${gitContext.branch || "<unknown>"}`,
  `Packet status: ${packetStatus || "<missing>"}`,
  `Current WP_STATUS: ${currentWpStatus || "<empty>"}`,
  `Workflow lane: ${workflowLane || "<missing>"}`,
  `Bootstrap claim commit: ${bootstrapClaim ? "present" : "missing"}`,
  `Skeleton checkpoint: ${usesSkeletonCheckpointGate ? (skeletonCheckpoint ? "present" : "missing") : "N/A (forbidden on ORCHESTRATOR_MANAGED)"}`,
  `Skeleton approval: ${usesSkeletonCheckpointGate ? (skeletonApproved ? "present" : "missing") : "N/A (forbidden on ORCHESTRATOR_MANAGED)"}`,
];
const coderGovernanceState = evaluateCoderPacketGovernanceState({
  wpId,
  packetPath: packetPath(wpId),
  packetContent,
  currentWpStatus,
});
const coderCommunicationState = loadCoderCommunicationState({
  wpId,
  packetPath: packetPath(wpId),
  packetContent,
});
const coderResumeState = deriveCoderResumeState({
  communicationState: coderCommunicationState,
});
const communicationFindings = [
  coderCommunicationState?.runtimeStatus?.next_expected_actor
    ? `Runtime next actor: ${coderCommunicationState.runtimeStatus.next_expected_actor}${coderCommunicationState.runtimeStatus.next_expected_session ? `:${coderCommunicationState.runtimeStatus.next_expected_session}` : ""}`
    : null,
  coderCommunicationState?.runtimeStatus?.waiting_on
    ? `Runtime waiting_on: ${coderCommunicationState.runtimeStatus.waiting_on}`
    : null,
  coderResumeState.latestAssessment
    ? `Latest validator assessment: ${coderResumeState.latestAssessment.verdict} via ${coderResumeState.latestAssessment.receiptKind} - ${coderResumeState.latestAssessment.reason}`
    : null,
].filter(Boolean);
const reviewRouteCommands = (() => {
  const wpValidatorSession =
    String(coderCommunicationState?.runtimeStatus?.wp_validator_of_record || "").trim()
    || "<wp-validator-session>";
  const integrationValidatorSession =
    String(coderCommunicationState?.runtimeStatus?.integration_validator_of_record || "").trim()
    || "<integration-validator-session>";
  const kickoffCorrelation =
    String(coderCommunicationState?.communicationEvaluation?.correlations?.kickoff || "").trim()
    || "<correlation_id>";
  const waitingOn = String(coderResumeState.waitingOn || "").trim().toUpperCase();

  if (waitingOn === "CODER_INTENT") {
    return [
      `just check-notifications ${wpId} CODER`,
      `just wp-coder-intent ${wpId} <coder-session> ${wpValidatorSession} "<summary>" ${kickoffCorrelation}`,
      `just ack-notifications ${wpId} CODER <coder-session>`,
    ];
  }
  if (waitingOn === "FINAL_REVIEW_EXCHANGE") {
    return [
      `just check-notifications ${wpId} CODER`,
      `just wp-review-exchange REVIEW_REQUEST ${wpId} CODER <coder-session> INTEGRATION_VALIDATOR ${integrationValidatorSession} "<summary>"`,
      `just ack-notifications ${wpId} CODER <coder-session>`,
    ];
  }
  if (waitingOn.startsWith("OPEN_REVIEW_ITEM_")) {
    return [
      `just check-notifications ${wpId} CODER`,
      `just session-registry-status ${wpId}`,
      `just active-lane-brief CODER ${wpId}`,
    ];
  }
  return [];
})();
const allFindings = [...commonFindings, ...communicationFindings];

if (!coderGovernanceState.allowResume) {
  const stopCommands = coderGovernanceState.legacyRemediationRequired
    ? [
        `just validator-policy-gate ${wpId}`,
        "# STOP: Request a NEW remediation WP variant from the Orchestrator.",
      ]
    : [
        postWorkCommand,
        "# STOP: Closed or validator-owned packet; do not resume coder implementation.",
      ];
  const operatorAction = coderGovernanceState.legacyRemediationRequired
    ? "Request NEW remediation WP variant; do not resume closed legacy packet in-place."
    : "NONE";

  printLifecycle({ wpId, stage: "HANDOFF", next: "STOP" });
  printOperatorAction(operatorAction);
  printConfidence(confidence, confidenceDetail);
  printState(coderGovernanceState.message);
  printFindings([
    ...allFindings,
    `Computed policy outcome: ${coderGovernanceState.computedPolicy.outcome}`,
    `Computed policy applicability: ${coderGovernanceState.computedPolicy.applicability_reason || "APPLICABLE"}`,
  ]);
  printNextCommands(stopCommands);
  process.exit(0);
}

if (!bootstrapClaim) {
  printLifecycle({ wpId, stage: "BOOTSTRAP", next: "BOOTSTRAP" });
  printOperatorAction("NONE");
  printConfidence(confidence, confidenceDetail);
  printState("Coder claim/bootstrap commit is missing; resume at BOOTSTRAP.");
  printFindings(allFindings);
  printNextCommands([
    `cat ${packetPath(wpId).replace(/\\/g, "/")}`,
    `node .GOV/roles/coder/checks/coder-bootstrap-claim.mjs ${wpId}`,
    `just backup-push feat/${wpId} feat/${wpId}`,
  ]);
  process.exit(0);
}

if (usesSkeletonCheckpointGate && !skeletonCheckpoint) {
  printLifecycle({ wpId, stage: "SKELETON", next: "SKELETON" });
  printOperatorAction("NONE");
  printConfidence(confidence, confidenceDetail);
  printState("Bootstrap is claimed; the docs-only skeleton checkpoint is the next required step.");
  printFindings(allFindings);
  printNextCommands([
    `cat ${packetPath(wpId).replace(/\\/g, "/")}`,
    `just coder-skeleton-checkpoint ${wpId}`,
    `just pre-work ${wpId}`,
  ]);
  process.exit(0);
}

if (usesSkeletonCheckpointGate && !skeletonApproved) {
  printLifecycle({ wpId, stage: "SKELETON", next: "STOP" });
  printOperatorAction(`${skeletonApprover} must create skeleton approval commit for ${wpId}`);
  printConfidence(confidence, confidenceDetail);
  printState("Skeleton checkpoint exists; implementation remains blocked until the approval commit lands.");
  printFindings(allFindings);
  printNextCommands([
    `# STOP: Await skeleton approval (${skeletonApprover} runs: just skeleton-approved ${wpId})`,
    `just pre-work ${wpId}`,
  ]);
  process.exit(0);
}

if (reviewRouteCommands.length > 0) {
  printLifecycle({ wpId, stage: "DIRECT_REVIEW", next: "DIRECT_REVIEW" });
  printOperatorAction("NONE");
  printConfidence(confidence, confidenceDetail);
  printState(coderResumeState.message);
  printFindings(allFindings);
  printNextCommands(reviewRouteCommands);
  process.exit(0);
}

if (coderResumeState.blockedByRoute) {
  printLifecycle({ wpId, stage: "STATUS_SYNC", next: "STOP" });
  printOperatorAction("NONE");
  printConfidence(confidence, confidenceDetail);
  printState(coderResumeState.message);
  printFindings(allFindings);
  printNextCommands([
    `just check-notifications ${wpId} CODER`,
    `just session-registry-status ${wpId}`,
    "# STOP: Wait for the routed next actor to advance the governed lane.",
  ]);
  process.exit(0);
}

if (implementationFilled && hygieneFilled && (validationFilled || !dirtyTree)) {
  printLifecycle({ wpId, stage: "POST_WORK", next: "HANDOFF" });
  printOperatorAction("NONE");
  printConfidence(confidence, confidenceDetail);
  printState(
    coderResumeState.remediationRequired
      ? "Validator review recorded FAIL; the repair loop is active and this branch is already at post-work closure. Re-run post-work on the committed repair state before re-handoff."
      : "Implementation and hygiene evidence exist; resume at post-work closure."
  );
  printFindings([
    ...allFindings,
    dirtyTreeFinding,
    ...dirtyNoiseFindings,
  ]);
  printNextCommands([
    "just cargo-clean",
    postWorkCommand,
  ]);
  process.exit(0);
}

if (implementationFilled) {
  printLifecycle({ wpId, stage: "HYGIENE", next: "POST_WORK" });
  printOperatorAction("NONE");
  printConfidence(confidence, confidenceDetail);
  printState(
    coderResumeState.remediationRequired
      ? "Validator review recorded FAIL; resume remediation and refresh hygiene/evidence before the next post-work and re-handoff."
      : "Implementation is present; resume at hygiene/evidence capture before post-work."
  );
  printFindings([
    ...allFindings,
    dirtyTreeFinding,
    ...dirtyNoiseFindings,
  ]);
  printNextCommands([
    `just pre-work ${wpId}`,
    "just product-scan",
    postWorkCommand,
  ]);
  process.exit(0);
}

printLifecycle({ wpId, stage: "IMPLEMENTATION", next: "HYGIENE" });
printOperatorAction("NONE");
printConfidence(confidence, confidenceDetail);
printState(
  coderResumeState.remediationRequired
    ? "Validator review recorded FAIL; resume in-scope remediation against the latest review before re-handoff."
    : usesSkeletonCheckpointGate
      ? "Skeleton is approved and no handoff markers are present; implementation may continue."
      : "Bootstrap is claimed and the orchestrator-managed lane is active; implementation may continue."
);
printFindings([
  ...allFindings,
  dirtyTreeFinding,
  ...dirtyNoiseFindings,
]);
printNextCommands([
  `just pre-work ${wpId}`,
  "# Continue implementation within IN_SCOPE_PATHS.",
  postWorkCommand,
]);
