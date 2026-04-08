#!/usr/bin/env node

import {
  buildPhaseCheckCommand,
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
import { deriveWpMicrotaskPlan } from "../../../roles_shared/scripts/lib/wp-microtask-lib.mjs";
import {
  classifyWpChangedPath,
  deriveWpScopeContract,
  isGovernanceOnlyPath,
  isTransientProofArtifactPath,
  normalizeRepoPath,
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
    governanceNoise: [],
    governanceCompanion: [],
    transientArtifacts: [],
    inScope: [],
    outOfScope: [],
  };

  for (const changedPath of changedPaths) {
    const classification = classifyWpChangedPath(changedPath, scopeContract);
    const normalizedPath = normalizeRepoPath(changedPath) || changedPath;
    if (classification.kind === "GOVERNANCE_COMPANION") {
      summary.governanceCompanion.push(classification.path);
    } else if (isTransientProofArtifactPath(normalizedPath)) {
      summary.transientArtifacts.push(normalizedPath);
    } else if (isGovernanceOnlyPath(normalizedPath)) {
      summary.governanceNoise.push(normalizedPath);
    } else if (classification.allowed) {
      summary.inScope.push(classification.path);
    } else {
      summary.outOfScope.push(`${classification.path} (${classification.kind})`);
    }
  }

  return summary;
}

const { wpId, gitContext, confidence, confidenceDetail } = resolveWpId();
const startupCommand = buildPhaseCheckCommand({ phase: "STARTUP", wpId, role: "CODER" });

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
    : dirtySummary.inScope.length + dirtySummary.governanceCompanion.length > 0
      ? `Working tree dirty: yes (${dirtySummary.inScope.length + dirtySummary.governanceCompanion.length} packet-scoped path(s))`
      : dirtySummary.governanceNoise.length + dirtySummary.transientArtifacts.length > 0
        ? `Working tree dirty: yes (governance/proof noise only across ${dirtySummary.governanceNoise.length + dirtySummary.transientArtifacts.length} path(s))`
      : `Working tree dirty: yes (${dirtySummary.inScope.length + dirtySummary.governanceCompanion.length} packet-scoped path(s))`;
const dirtyNoiseFindings = [
  dirtySummary.governanceNoise.length > 0
    ? `Governance-only drift: ${dirtySummary.governanceNoise.length} path(s) treated as non-blocking noise by default`
    : "",
  dirtySummary.governanceCompanion.length > 0
    ? `Governance companion paths touched: ${dirtySummary.governanceCompanion.length} (${dirtySummary.governanceCompanion.slice(0, 3).join(", ")})`
    : "",
  dirtySummary.transientArtifacts.length > 0
    ? `Transient proof artifacts present: ${dirtySummary.transientArtifacts.length} (${dirtySummary.transientArtifacts.slice(0, 3).join(", ")})`
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
const microtaskPlan = deriveWpMicrotaskPlan({
  wpId,
  receipts: coderCommunicationState?.receipts || [],
  runtimeStatus: coderCommunicationState?.runtimeStatus || {},
});
const coderResumeState = deriveCoderResumeState({
  communicationState: coderCommunicationState,
});
const activeMicrotask = microtaskPlan.active_microtask;
const previousMicrotask = microtaskPlan.previous_microtask;
const suggestedMicrotask = microtaskPlan.suggested_next_microtask;
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
  microtaskPlan.declared_count > 0
    ? `Declared microtasks: ${microtaskPlan.declared_count} | active=${activeMicrotask?.mt_id || "<none>"} | next=${suggestedMicrotask?.mt_id || "<none>"}`
    : null,
  activeMicrotask
    ? `Active microtask: ${activeMicrotask.mt_id} (${activeMicrotask.state}) - ${activeMicrotask.clause}`
    : null,
  previousMicrotask
    ? `Previous microtask under review: ${previousMicrotask.mt_id} (${previousMicrotask.state}) - ${previousMicrotask.clause}`
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
      `just active-lane-brief CODER ${wpId}`,
      `just check-notifications ${wpId} CODER`,
      activeMicrotask?.mt_id || suggestedMicrotask?.mt_id
        ? `# Publish intent against ${activeMicrotask?.mt_id || suggestedMicrotask?.mt_id} with matching microtask_json.scope_ref and bounded file_targets.`
        : null,
      `just wp-coder-intent ${wpId} <coder-session> ${wpValidatorSession} "<summary>" ${kickoffCorrelation}`,
      `just ack-notifications ${wpId} CODER <coder-session>`,
    ].filter(Boolean);
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
    startupCommand,
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
    startupCommand,
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
    startupCommand,
    microtaskPlan.declared_count > 0
      ? `just active-lane-brief CODER ${wpId}`
      : null,
    "just product-scan",
    postWorkCommand,
  ].filter(Boolean));
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
  startupCommand,
  microtaskPlan.declared_count > 0
    ? `just active-lane-brief CODER ${wpId}`
    : null,
  activeMicrotask?.mt_id || suggestedMicrotask?.mt_id
    ? `# Continue implementation within ${(activeMicrotask?.mt_id || suggestedMicrotask?.mt_id)} and keep coder writes inside its bounded CODE_SURFACES.`
    : "# Continue implementation within IN_SCOPE_PATHS.",
  postWorkCommand,
].filter(Boolean));
