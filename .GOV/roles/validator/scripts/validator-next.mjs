#!/usr/bin/env node

import fs from "node:fs";
import path from "node:path";
import {
  buildPostWorkCommand,
  currentGitContext,
  failWithContext,
  inferWpIdFromPrepare,
  isTerminalTaskBoardStatus,
  loadJson,
  loadOrchestratorGateLogs,
  loadPacket,
  normalizeVerdict,
  packetExists,
  packetPath,
  parseCurrentWpStatus,
  parseStatus,
  printConfidence,
  printFindings,
  printLifecycle,
  printNextCommands,
  printOperatorAction,
  printState,
  printVerdict,
  taskBoardStatus,
} from "../../../roles_shared/scripts/lib/role-resume-utils.mjs";
import { listValidatorGateStateFiles, resolveValidatorGatePath } from "../../../roles_shared/scripts/lib/validator-gate-paths.mjs";
import { GOV_ROOT_REPO_REL, REPO_ROOT, inferWpIdFromPacketPath, repoPathAbs } from "../../../roles_shared/scripts/lib/runtime-paths.mjs";
import {
  buildValidatorReadyCommands,
  deriveValidatorResumeState,
  evaluateValidatorPacketGovernanceState,
  evaluateValidatorPassAuthority,
  loadValidatorCommunicationState,
  resolveValidatorActorContext,
} from "./lib/validator-governance-lib.mjs";

function freshnessBoost(timestampMs) {
  const ageHours = Math.max(0, (Date.now() - timestampMs) / (1000 * 60 * 60));
  return Math.max(0, 24 - Math.min(24, ageHours / 2));
}

function validatorSessionScore(status) {
  switch (String(status || "").trim().toUpperCase()) {
    case "REPORT_PRESENTED":
      return 190;
    case "COMMITTED":
      return 180;
    case "WP_APPENDED":
      return 170;
    default:
      return 0;
  }
}

function isValidationReadyStatus(currentWpStatus) {
  return /(validator|validation|review|audit|implementation complete|ready for review|ready for audit|ready for validator|post-work)/i.test(
    String(currentWpStatus || ""),
  );
}

function projectedValidatorReadyState(communicationState = null) {
  const nextActor = String(communicationState?.runtimeStatus?.next_expected_actor || "").trim().toUpperCase();
  return ["WP_VALIDATOR", "INTEGRATION_VALIDATOR"].includes(nextActor);
}

function projectedValidatorReadyReason(communicationState = null) {
  const nextActor = String(communicationState?.runtimeStatus?.next_expected_actor || "").trim().toUpperCase();
  const waitingOn = String(communicationState?.runtimeStatus?.waiting_on || "").trim();
  if (!nextActor) return "";
  return `${nextActor} projected next via runtime${waitingOn ? ` (${waitingOn})` : ""}`;
}

function collectPendingSessions() {
  const candidates = [];
  for (const filePath of listValidatorGateStateFiles()) {
    const wpId = path.basename(filePath).replace(/\.json$/i, "");
    const session = loadValidationSession(wpId);
    if (!session || session.status === "USER_ACKNOWLEDGED") continue;

    const boardStatus = taskBoardStatus(wpId);
    if (isTerminalTaskBoardStatus(boardStatus)) continue;
    const packetContent = loadPacket(wpId);
    if (!packetContent) continue;
    const validatorGovernanceState = evaluateValidatorPacketGovernanceState({
      wpId,
      packetPath: packetPath(wpId),
      packetContent,
      currentWpStatus: parseCurrentWpStatus(packetContent),
      taskBoardStatus: boardStatus,
      sessionStatus: session.status,
    });
    if (!validatorGovernanceState.allowValidationResume) continue;

    const timestamp = Date.parse(
      String(session.gates?.[session.gates.length - 1]?.timestamp || session.started || ""),
    );
    const score =
      validatorSessionScore(session.status) +
      (Number.isNaN(timestamp) ? 0 : freshnessBoost(timestamp));

    candidates.push({
      wpId,
      reason: `validator gate session ${session.status}`,
      score,
      timestamp: Number.isNaN(timestamp) ? 0 : timestamp,
    });
  }

  return candidates.sort((left, right) => {
    if (right.score !== left.score) return right.score - left.score;
    return right.timestamp - left.timestamp;
  });
}

function collectValidationReadyPackets() {
  const taskPacketDir = repoPathAbs(path.join(GOV_ROOT_REPO_REL, "task_packets"));
  if (!fs.existsSync(taskPacketDir)) return [];

  const candidates = [];
  for (const entry of fs.readdirSync(taskPacketDir, { withFileTypes: true })) {
    let filePath = "";
    if (entry.isDirectory()) {
      if (!entry.name.startsWith("WP-")) continue;
      filePath = path.join(taskPacketDir, entry.name, "packet.md");
      if (!fs.existsSync(filePath)) continue;
    } else if (entry.isFile() && entry.name.endsWith(".md")) {
      filePath = path.join(taskPacketDir, entry.name);
    } else {
      continue;
    }

    const wpId = inferWpIdFromPacketPath(filePath);
    if (!wpId) continue;

    const boardStatus = taskBoardStatus(wpId);
    if (isTerminalTaskBoardStatus(boardStatus)) continue;

    const packetContent = loadPacket(wpId);
    const validatorGovernanceState = evaluateValidatorPacketGovernanceState({
      wpId,
      packetPath: packetPath(wpId),
      packetContent,
      currentWpStatus: parseCurrentWpStatus(packetContent),
      taskBoardStatus: boardStatus,
    });
    if (!validatorGovernanceState.allowValidationResume) continue;
    const currentWpStatus = parseCurrentWpStatus(packetContent);
    const packetStatus = parseStatus(packetContent);
    const communicationState = loadValidatorCommunicationState({
      wpId,
      packetPath: packetPath(wpId),
      packetContent,
    });
    const runtimeReady = projectedValidatorReadyState(communicationState);
    if (!runtimeReady && !isValidationReadyStatus(currentWpStatus) && !/^done$/i.test(packetStatus)) continue;

    const stat = fs.statSync(filePath);
    const score =
      (runtimeReady ? 150 : isValidationReadyStatus(currentWpStatus) ? 140 : 120) +
      (boardStatus === "IN_PROGRESS" ? 10 : 0) +
      freshnessBoost(stat.mtimeMs);

    candidates.push({
      wpId,
      reason: runtimeReady
        ? projectedValidatorReadyReason(communicationState)
        : currentWpStatus || `packet status ${packetStatus || "<missing>"}`,
      score,
      timestamp: stat.mtimeMs,
    });
  }

  return candidates.sort((left, right) => {
    if (right.score !== left.score) return right.score - left.score;
    return right.timestamp - left.timestamp;
  });
}

function loadValidationSession(wpId) {
  const filePath = resolveValidatorGatePath(wpId);
  if (!fs.existsSync(repoPathAbs(filePath))) return null;
  const raw = loadJson(filePath, {});
  return raw?.validation_sessions?.[wpId] || null;
}

function resolveWpId() {
  const provided = String(process.argv[2] || "").trim();
  const gitContext = currentGitContext();
  const logs = loadOrchestratorGateLogs();

  if (provided) {
    return { wpId: provided, gitContext, confidence: "HIGH", confidenceDetail: "explicit" };
  }

  const inferred = inferWpIdFromPrepare(logs, gitContext, gitContext.topLevel || REPO_ROOT);
  if (inferred.wpId) {
    return {
      wpId: inferred.wpId,
      gitContext,
      confidence: "HIGH",
      confidenceDetail: inferred.source,
    };
  }

  const pendingSessions = collectPendingSessions();
  if (pendingSessions.length === 1) {
    return {
      wpId: pendingSessions[0].wpId,
      gitContext,
      confidence: "MEDIUM",
      confidenceDetail: "single-pending-session",
    };
  }
  if (pendingSessions.length >= 2 && pendingSessions[0].score - pendingSessions[1].score >= 12) {
    return {
      wpId: pendingSessions[0].wpId,
      gitContext,
      confidence: "MEDIUM",
      confidenceDetail: "ranked-pending-session",
    };
  }

  const readyPackets = collectValidationReadyPackets();
  if (readyPackets.length === 1) {
    return {
      wpId: readyPackets[0].wpId,
      gitContext,
      confidence: "MEDIUM",
      confidenceDetail: "single-ready-packet",
    };
  }
  if (readyPackets.length >= 2 && readyPackets[0].score - readyPackets[1].score >= 12) {
    return {
      wpId: readyPackets[0].wpId,
      gitContext,
      confidence: "MEDIUM",
      confidenceDetail: "ranked-ready-packet",
    };
  }

  const suggested = [...pendingSessions, ...readyPackets]
    .sort((left, right) => {
      if (right.score !== left.score) return right.score - left.score;
      return right.timestamp - left.timestamp;
    })
    .reduce((acc, candidate) => {
      if (!acc.find((entry) => entry.wpId === candidate.wpId)) acc.push(candidate);
      return acc;
    }, []);

  const nextCommands = inferred.candidates.length
    ? inferred.candidates.map((candidate) => `just validator-next ${candidate}`)
    : suggested.length
      ? suggested.slice(0, 5).map((candidate) => `just validator-next ${candidate.wpId}`)
      : ["just validator-next WP-{ID}"];

  failWithContext({
    state: "Unable to infer the active WP for validation from the current branch/worktree.",
    findings: [
      `Current branch: ${gitContext.branch || "<unknown>"}`,
      `Current worktree: ${gitContext.topLevel || "<unknown>"}`,
      inferred.candidates.length
        ? `Ambiguous WP candidates from PREPARE: ${inferred.candidates.join(", ")}`
        : "No PREPARE entry matched the current branch/worktree.",
      pendingSessions.length
        ? `Pending validator sessions: ${pendingSessions
            .slice(0, 5)
            .map((candidate) => `${candidate.wpId} (${candidate.reason})`)
            .join(", ")}`
        : "No pending validator gate session matched a non-terminal WP.",
      readyPackets.length
        ? `Validation-ready packets: ${readyPackets
            .slice(0, 5)
            .map((candidate) => `${candidate.wpId} (${candidate.reason})`)
            .join(", ")}`
        : "No non-terminal packet advertised Validator-ready handoff markers.",
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
    state: "Work packet is missing; Validator cannot resume deterministically.",
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
const currentWpStatusLower = currentWpStatus.toLowerCase();
const boardStatus = taskBoardStatus(wpId) || "<none>";
const postWorkCommand = buildPostWorkCommand(wpId, packetContent);
const session = loadValidationSession(wpId);
const validatorActorContext = resolveValidatorActorContext({
  repoRoot: gitContext.topLevel || REPO_ROOT,
  wpId,
  packetContent,
  gitContext,
});
const validatorGovernanceState = evaluateValidatorPacketGovernanceState({
  wpId,
  packetPath: packetPath(wpId),
  packetContent,
  currentWpStatus,
  taskBoardStatus: boardStatus,
  sessionStatus: session?.status || "",
  actorContext: validatorActorContext,
});
const communicationState = loadValidatorCommunicationState({
  wpId,
  packetPath: packetPath(wpId),
  packetContent,
});
const validatorResumeState = deriveValidatorResumeState({
  actorRole: validatorActorContext.actorRole,
  communicationState,
});
const passAuthorityCheck = evaluateValidatorPassAuthority({
  packetContent,
  actorContext: validatorActorContext,
});

if (!validatorGovernanceState.allowValidationResume) {
  printVerdict("BLOCKED");
  printLifecycle({ wpId, stage: "STATUS_SYNC", next: "STOP" });
  printOperatorAction("Request NEW remediation WP variant; do not merge/sync closed legacy packet in-place.");
  printConfidence(confidence, confidenceDetail);
  printState(validatorGovernanceState.message);
  printFindings([
    `Current branch: ${gitContext.branch || "<unknown>"}`,
    `Packet status: ${packetStatus || "<missing>"}`,
    `Current WP_STATUS: ${currentWpStatus || "<empty>"}`,
    `Task Board status: ${boardStatus}`,
    `Validator gate status: ${session?.status || "<none>"}`,
    `Resolved validator lane: ${validatorActorContext.actorRole} (${validatorActorContext.source})`,
    `Computed policy outcome: ${validatorGovernanceState.computedPolicy.outcome}`,
    `Computed policy applicability: ${validatorGovernanceState.computedPolicy.applicability_reason || "APPLICABLE"}`,
  ]);
  printNextCommands([
    `just validator-policy-gate ${wpId}`,
    `just validator-packet-complete ${wpId}`,
    "# STOP: Request NEW remediation WP variant; do not merge or reopen this packet in-place.",
  ]);
  process.exit(0);
}

if (session) {
  const verdict = normalizeVerdict(session.verdict);
const findings = [
  `Current branch: ${gitContext.branch || "<unknown>"}`,
  `Packet status: ${packetStatus || "<missing>"}`,
  `Current WP_STATUS: ${currentWpStatus || "<empty>"}`,
  `Task Board status: ${boardStatus}`,
  `Validator gate status: ${session.status}`,
  `Resolved validator lane: ${validatorActorContext.actorRole} (${validatorActorContext.source})`,
];

  printVerdict(verdict);

  if (session.status === "WP_APPENDED") {
    if (verdict === "PASS" && !passAuthorityCheck.ok) {
      printVerdict("BLOCKED");
      printLifecycle({ wpId, stage: "VALIDATION", next: "STOP" });
      printOperatorAction("Route final PASS lane to INTEGRATION_VALIDATOR; WP_VALIDATOR cannot complete merge-ready authority.");
      printConfidence(confidence, confidenceDetail);
      printState("A PASS gate session exists, but the current validator lane does not satisfy final authority for this packet.");
      printFindings([...findings, ...passAuthorityCheck.issues]);
      printNextCommands([
        `just validator-gate-status ${wpId}`,
        `just session-registry-status ${wpId}`,
        "# STOP: Resume the Integration Validator lane for final PASS authority.",
      ]);
      process.exit(0);
    }

    printLifecycle({ wpId, stage: "VALIDATION", next: "VALIDATION" });
    printOperatorAction("NONE");
    printConfidence(confidence, confidenceDetail);
    printState(
      verdict === "PASS"
        ? (
          validatorActorContext.actorRole === "INTEGRATION_VALIDATOR"
            ? "Integration Validator lane is active; committed handoff validation and final PASS clearance are the next required gates."
            : "Validation report was appended; commit clearance is the next required gate."
        )
        : "Validation report was appended; present the FAIL report before any remediation begins.",
    );
    printFindings(findings);
    printNextCommands([
      verdict === "PASS"
        ? (
          validatorActorContext.actorRole === "INTEGRATION_VALIDATOR"
            ? `just integration-validator-context-brief ${wpId}`
            : `just validator-gate-commit ${wpId}`
        )
        : `just validator-gate-present ${wpId}`,
      verdict === "PASS" && validatorActorContext.actorRole === "INTEGRATION_VALIDATOR"
        ? `just validator-gate-commit ${wpId}`
        : null,
    ].filter(Boolean));
    process.exit(0);
  }

  if (session.status === "COMMITTED") {
    if (verdict === "PASS" && !passAuthorityCheck.ok) {
      printVerdict("BLOCKED");
      printLifecycle({ wpId, stage: "VALIDATION", next: "STOP" });
      printOperatorAction("Route final PASS lane to INTEGRATION_VALIDATOR; current lane cannot present the final merge-ready report.");
      printConfidence(confidence, confidenceDetail);
      printState("PASS commit clearance is recorded, but the current validator lane does not satisfy final report authority.");
      printFindings([...findings, ...passAuthorityCheck.issues]);
      printNextCommands([
        `just validator-gate-status ${wpId}`,
        `just session-registry-status ${wpId}`,
        "# STOP: Resume the Integration Validator lane for final report presentation.",
      ]);
      process.exit(0);
    }

    printLifecycle({ wpId, stage: "VALIDATION", next: "VALIDATION" });
    printOperatorAction("NONE");
    printConfidence(confidence, confidenceDetail);
    printState(
      validatorActorContext.actorRole === "INTEGRATION_VALIDATOR"
        ? "Integration Validator lane is active; present the final report to the user next."
        : "PASS commit clearance is already recorded; present the final report to the user next.",
    );
    printFindings(findings);
    printNextCommands([
      validatorActorContext.actorRole === "INTEGRATION_VALIDATOR"
        ? `just integration-validator-context-brief ${wpId}`
        : null,
      `just validator-gate-present ${wpId}`,
    ].filter(Boolean));
    process.exit(0);
  }

  if (session.status === "REPORT_PRESENTED") {
    printLifecycle({ wpId, stage: "VALIDATION", next: "STOP" });
    printOperatorAction(`User acknowledgment for ${wpId}`);
    printConfidence(confidence, confidenceDetail);
    printState("The full validation report is already presented; Validator must halt until the user acknowledges it.");
    printFindings(findings);
    printNextCommands([`just validator-gate-acknowledge ${wpId}`]);
    process.exit(0);
  }

  if (session.status === "USER_ACKNOWLEDGED") {
    printLifecycle({
      wpId,
      stage: verdict === "PASS" ? "MERGE" : "VALIDATION",
      next: "STOP",
    });
    printOperatorAction(
      verdict === "PASS"
        ? `Explicit sync authorization required to merge/push ${wpId}`
        : "NONE",
    );
    printConfidence(confidence, confidenceDetail);
    printState(
      verdict === "PASS"
        ? "Validation is fully acknowledged; merge/push remains blocked until the Operator authorizes sync actions in this turn."
        : "FAIL report was acknowledged; remediation belongs to the Coder/Orchestrator.",
    );
    printFindings(findings);
    printNextCommands(
      verdict === "PASS"
        ? [
            "# STOP: Await explicit Operator authorization for merge/push.",
            `just validator-gate-status ${wpId}`,
          ]
        : [
            "# STOP: Return the WP to Coder/Orchestrator for remediation.",
            `just validator-gate-status ${wpId}`,
          ],
    );
    process.exit(0);
  }
}

const findings = [
  `Current branch: ${gitContext.branch || "<unknown>"}`,
  `Packet status: ${packetStatus || "<missing>"}`,
  `Current WP_STATUS: ${currentWpStatus || "<empty>"}`,
  `Task Board status: ${boardStatus}`,
  `Resolved validator lane: ${validatorActorContext.actorRole} (${validatorActorContext.source})`,
  communicationState?.runtimeStatus?.next_expected_actor
    ? `Runtime next actor: ${communicationState.runtimeStatus.next_expected_actor}${communicationState.runtimeStatus.next_expected_session ? `:${communicationState.runtimeStatus.next_expected_session}` : ""}`
    : null,
  communicationState?.runtimeStatus?.waiting_on
    ? `Runtime waiting_on: ${communicationState.runtimeStatus.waiting_on}`
    : null,
  validatorResumeState.latestAssessment
    ? `Latest validator assessment: ${validatorResumeState.latestAssessment.verdict} via ${validatorResumeState.latestAssessment.receiptKind} - ${validatorResumeState.latestAssessment.reason}`
    : null,
].filter(Boolean);

if (["VALIDATED", "FAIL", "OUTDATED_ONLY", "ABANDONED", "SUPERSEDED"].includes(boardStatus)) {
  printVerdict(
    boardStatus === "VALIDATED"
      ? "PASS"
      : boardStatus === "FAIL"
        ? "FAIL"
        : boardStatus === "ABANDONED"
          ? "ABANDONED"
          : "PENDING",
  );
  printLifecycle({ wpId, stage: "STATUS_SYNC", next: "STOP" });
  printOperatorAction("NONE");
  printConfidence(confidence, confidenceDetail);
  printState("Task Board already records a terminal state for this WP; no fresh validation resume action is inferred.");
  printFindings(findings);
  printNextCommands([`just validator-gate-status ${wpId}`]);
  process.exit(0);
}

if (validatorResumeState.ready) {
  printVerdict("PENDING");
  printLifecycle({ wpId, stage: "VALIDATION", next: "VALIDATION" });
  printOperatorAction("NONE");
  printConfidence(confidence, confidenceDetail);
  printState(validatorResumeState.message);
  printFindings(findings);
  printNextCommands(buildValidatorReadyCommands({
    wpId,
    actorRole: validatorActorContext.actorRole,
    actorSessionId: validatorActorContext.actorSessionId,
    postWorkCommand,
  }));
  process.exit(0);
}

if (validatorResumeState.blockedByRoute) {
  printVerdict("PENDING");
  printLifecycle({ wpId, stage: "STATUS_SYNC", next: "STOP" });
  printOperatorAction("NONE");
  printConfidence(confidence, confidenceDetail);
  printState(validatorResumeState.message);
  printFindings(findings);
  printNextCommands([
    `just check-notifications ${wpId} ${validatorActorContext.actorRole || "WP_VALIDATOR"}`,
    `just session-registry-status ${wpId}`,
    "# STOP: Wait for the routed next actor to advance the governed lane.",
  ]);
  process.exit(0);
}

if (
  currentWpStatusLower.includes("validator") ||
  currentWpStatusLower.includes("validation") ||
  currentWpStatusLower.includes("ready for review") ||
  currentWpStatusLower.includes("ready for audit")
) {
  printVerdict("PENDING");
  printLifecycle({ wpId, stage: "VALIDATION", next: "VALIDATION" });
  printOperatorAction("NONE");
  printConfidence(confidence, confidenceDetail);
  printState(
    validatorActorContext.actorRole === "INTEGRATION_VALIDATOR"
      ? "Coder/WP-validator handoff markers indicate this WP is ready for final Integration Validator review."
      : validatorActorContext.actorRole === "WP_VALIDATOR"
        ? "Coder handoff markers indicate this WP is ready for WP Validator advisory review."
        : "Coder handoff markers indicate this WP is ready for Validator execution.",
  );
  printFindings(findings);
  printNextCommands(buildValidatorReadyCommands({
    wpId,
    actorRole: validatorActorContext.actorRole,
    actorSessionId: validatorActorContext.actorSessionId,
    postWorkCommand,
  }));
  process.exit(0);
}

if (/^done$/i.test(packetStatus)) {
  printVerdict("PENDING");
  printLifecycle({ wpId, stage: "STATUS_SYNC", next: "STOP" });
  printOperatorAction("NONE");
  printConfidence(confidence, confidenceDetail);
  printState("Packet status is already Done, but no validator gate session is active in this worktree.");
  printFindings(findings);
  printNextCommands([`just validator-gate-status ${wpId}`]);
  process.exit(0);
}

printVerdict("PENDING");
printLifecycle({ wpId, stage: "STATUS_SYNC", next: "STOP" });
printOperatorAction("NONE");
printConfidence(confidence, confidenceDetail);
printState("No validator handoff markers were found; resume requires either a ready-for-validation packet or an explicit WP choice.");
printFindings(findings);
printNextCommands([
  `cat ${packetPath(wpId).replace(/\\/g, "/")}`,
  `just validator-gate-status ${wpId}`,
]);
