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
} from "./role-resume-utils.mjs";

function resolveWpId() {
  const provided = String(process.argv[2] || "").trim();
  const gitContext = currentGitContext();
  const logs = loadOrchestratorGateLogs();

  if (provided) return { wpId: provided, gitContext, confidence: "HIGH", confidenceDetail: "explicit" };

  const inferred = inferWpIdFromPrepare(logs, gitContext);
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

const { wpId, gitContext, confidence, confidenceDetail } = resolveWpId();

if (!packetExists(wpId)) {
  failWithContext({
    wpId,
    stage: "BOOTSTRAP",
    next: "STOP",
    confidence,
    confidenceDetail,
    state: "Task packet is missing; coder work cannot resume deterministically.",
    findings: [`Expected packet: ${packetPath(wpId).replace(/\\/g, "/")}`],
    nextCommands: [
      `cat .GOV/roles/orchestrator/ORCHESTRATOR_GATES.json`,
      `just orchestrator-next ${wpId}`,
    ],
  });
}

const packetContent = loadPacket(wpId);
const packetStatus = parseStatus(packetContent);
const currentWpStatus = parseCurrentWpStatus(packetContent);
const workflowLane = parseClaimField(packetContent, "WORKFLOW_LANE").toUpperCase();
const coderModel = parseClaimField(packetContent, "CODER_MODEL");
const bootstrapClaim = hasCommitSubject(`^docs: bootstrap claim \\[${escapeRegex(wpId)}\\]$`);
const skeletonCheckpoint = hasCommitSubject(`^docs: skeleton checkpoint \\[${escapeRegex(wpId)}\\]$`);
const skeletonApproved = hasCommitSubject(`^docs: skeleton approved \\[${escapeRegex(wpId)}\\]$`);
const implementationFilled = sectionHasMaterialContent(packetContent, "IMPLEMENTATION");
const hygieneFilled = sectionHasMaterialContent(packetContent, "HYGIENE");
const validationFilled = sectionHasMaterialContent(packetContent, "VALIDATION");
const dirtyTree = Boolean(gitContext.statusPorcelain);
const postWorkCommand = buildPostWorkCommand(wpId, packetContent);
const currentWpStatusLower = currentWpStatus.toLowerCase();
const skeletonApprover =
  workflowLane === "ORCHESTRATOR_MANAGED" ? "Orchestrator/Validator/Operator" : "Validator/Operator";

const commonFindings = [
  `Current branch: ${gitContext.branch || "<unknown>"}`,
  `Packet status: ${packetStatus || "<missing>"}`,
  `Current WP_STATUS: ${currentWpStatus || "<empty>"}`,
  `Workflow lane: ${workflowLane || "<missing>"}`,
  `Bootstrap claim commit: ${bootstrapClaim ? "present" : "missing"}`,
  `Skeleton checkpoint: ${skeletonCheckpoint ? "present" : "missing"}`,
  `Skeleton approval: ${skeletonApproved ? "present" : "missing"}`,
];

if (!bootstrapClaim) {
  printLifecycle({ wpId, stage: "BOOTSTRAP", next: "BOOTSTRAP" });
  printOperatorAction("NONE");
  printConfidence(confidence, confidenceDetail);
  printState("Coder claim/bootstrap commit is missing; resume at BOOTSTRAP.");
  printFindings(commonFindings);
  printNextCommands([
    `cat ${packetPath(wpId).replace(/\\/g, "/")}`,
    `git add ${packetPath(wpId).replace(/\\/g, "/")}`,
    `git commit -m "docs: bootstrap claim [${wpId}]"`,
  ]);
  process.exit(0);
}

if (!skeletonCheckpoint) {
  printLifecycle({ wpId, stage: "SKELETON", next: "SKELETON" });
  printOperatorAction("NONE");
  printConfidence(confidence, confidenceDetail);
  printState("Bootstrap is claimed; the docs-only skeleton checkpoint is the next required step.");
  printFindings(commonFindings);
  printNextCommands([
    `cat ${packetPath(wpId).replace(/\\/g, "/")}`,
    `just coder-skeleton-checkpoint ${wpId}`,
    `just pre-work ${wpId}`,
  ]);
  process.exit(0);
}

if (!skeletonApproved) {
  printLifecycle({ wpId, stage: "SKELETON", next: "STOP" });
  printOperatorAction(`${skeletonApprover} must create skeleton approval commit for ${wpId}`);
  printConfidence(confidence, confidenceDetail);
  printState("Skeleton checkpoint exists; implementation remains blocked until the approval commit lands.");
  printFindings(commonFindings);
  printNextCommands([
    `# STOP: Await skeleton approval (${skeletonApprover} runs: just skeleton-approved ${wpId})`,
    `just pre-work ${wpId}`,
  ]);
  process.exit(0);
}

if (
  currentWpStatusLower.includes("validator") ||
  currentWpStatusLower.includes("validation") ||
  currentWpStatusLower.includes("handoff")
) {
  printLifecycle({ wpId, stage: "HANDOFF", next: "STOP" });
  printOperatorAction("NONE");
  printConfidence(confidence, confidenceDetail);
  printState("Packet already indicates Validator handoff; coder should not resume implementation.");
  printFindings(commonFindings);
  printNextCommands([
    postWorkCommand,
    `# STOP: Await Validator review/handoff.`,
  ]);
  process.exit(0);
}

if (implementationFilled && hygieneFilled && (validationFilled || !dirtyTree)) {
  printLifecycle({ wpId, stage: "POST_WORK", next: "HANDOFF" });
  printOperatorAction("NONE");
  printConfidence(confidence, confidenceDetail);
  printState("Implementation and hygiene evidence exist; resume at post-work closure.");
  printFindings([
    ...commonFindings,
    `Working tree dirty: ${dirtyTree ? "yes" : "no"}`,
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
  printState("Implementation is present; resume at hygiene/evidence capture before post-work.");
  printFindings([
    ...commonFindings,
    `Working tree dirty: ${dirtyTree ? "yes" : "no"}`,
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
printState("Skeleton is approved and no handoff markers are present; implementation may continue.");
printFindings([
  ...commonFindings,
  `Working tree dirty: ${dirtyTree ? "yes" : "no"}`,
]);
printNextCommands([
  `just pre-work ${wpId}`,
  "# Continue implementation within IN_SCOPE_PATHS.",
  postWorkCommand,
]);
