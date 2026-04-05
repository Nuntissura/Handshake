#!/usr/bin/env node
/**
 * Orchestrator "resume without context" helper.
 *
 * This is intentionally read-only: it inspects Orchestrator gates + filesystem
 * state and prints the next minimal commands to run.
 */

import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import {
  defaultRefinementPath,
  validateRefinementFile,
} from "../../../roles_shared/checks/refinement-check.mjs";
import {
  activeOrchestratorCandidates,
  currentGitContext,
  inferOrchestratorWpId,
  loadOrchestratorGateLogs,
  preparedWorktreeSyncState,
  printConfidence,
  printFindings,
  printLifecycle,
  printNextCommands,
  printBlockerClass,
  printOperatorAction,
  printState,
  taskBoardStatus,
} from "../../../roles_shared/scripts/lib/role-resume-utils.mjs";
import { EXECUTION_OWNER_RANGE_HELP } from "../../../roles_shared/scripts/session/session-policy.mjs";
import { loadSessionRegistry } from "../../../roles_shared/scripts/session/session-registry-lib.mjs";
import { evaluateWpTokenBudget } from "../../../roles_shared/scripts/session/wp-token-budget-lib.mjs";
import { readWpTokenUsageLedger } from "../../../roles_shared/scripts/session/wp-token-usage-lib.mjs";
import { GOV_ROOT_REPO_REL, REPO_ROOT, repoPathAbs, resolveOrchestratorGatesPath, resolveWorkPacketPath } from "../../../roles_shared/scripts/lib/runtime-paths.mjs";
import { evaluatePacketRuntimeProjectionDrift } from "../../../roles_shared/scripts/lib/packet-runtime-projection-lib.mjs";
import { deriveLatestValidatorAssessment, evaluateWpCommunicationHealth } from "../../../roles_shared/scripts/lib/wp-communication-health-lib.mjs";
import { evaluateWpRelayEscalation } from "../../../roles_shared/scripts/lib/wp-relay-escalation-lib.mjs";
import { parseJsonFile, parseJsonlFile } from "../../../roles_shared/scripts/lib/wp-communications-lib.mjs";
import { checkAllNotifications } from "../../../roles_shared/scripts/wp/wp-check-notifications.mjs";
import { parsePolicyWaiverLedger } from "../../../roles_shared/scripts/lib/computed-policy-gate-lib.mjs";

const STATE_FILE = resolveOrchestratorGatesPath();
const STATE_FILE_ABS = repoPathAbs(STATE_FILE);
const TASK_BOARD_PATH = `${GOV_ROOT_REPO_REL}/roles_shared/records/TASK_BOARD.md`;
const TASK_BOARD_ABS_PATH = repoPathAbs(TASK_BOARD_PATH);
const EXECUTION_OWNER_USAGE = `{${EXECUTION_OWNER_RANGE_HELP}}`;
const GOVERNED_ROLE_RELAY_TARGETS = new Set(["CODER", "WP_VALIDATOR", "INTEGRATION_VALIDATOR"]);
const TERMINAL_ORCHESTRATOR_BOARD_STATUSES = new Set(["VALIDATED", "FAIL", "OUTDATED_ONLY", "ABANDONED", "SUPERSEDED"]);
const TOKEN_POLICY_CONFLICT_RE = /TOKEN_BUDGET_EXCEEDED|POLICY_CONFLICT|token\s+budget|token-ledger\s+drift/i;

function fail(message, details = []) {
  console.error(`[ORCHESTRATOR_NEXT] ${message}`);
  for (const line of details) console.error(`- ${line}`);
  process.exit(1);
}

function loadState() {
  if (!fs.existsSync(STATE_FILE_ABS)) return { gate_logs: [] };
  try {
    return JSON.parse(fs.readFileSync(STATE_FILE_ABS, "utf8"));
  } catch (e) {
    fail("Failed to read ORCHESTRATOR_GATES.json", [String(e?.message || e)]);
  }
}

function lastLog(state, wpId, type) {
  const logs = Array.isArray(state.gate_logs) ? state.gate_logs : [];
  return [...logs].reverse().find((l) => l?.wpId === wpId && l?.type === type) || null;
}

function exists(p) {
  try {
    return fs.existsSync(p);
  } catch {
    return false;
  }
}

function printOperatorEnvelope(action = "NONE", blockerClass = "NONE") {
  printOperatorAction(action);
  printBlockerClass(blockerClass);
}

function hasStubLine(wpId) {
  if (!exists(TASK_BOARD_ABS_PATH)) return false;
  const content = fs.readFileSync(TASK_BOARD_ABS_PATH, "utf8");
  return content.includes(`- **[${wpId}]** - [STUB]`);
}

function parseSingleField(text, label) {
  const re = new RegExp(`^\\s*-\\s*(?:\\*\\*)?${label}(?:\\*\\*)?\\s*:\\s*(.+)\\s*$`, "mi");
  const match = String(text || "").match(re);
  return match ? match[1].trim() : "";
}

function loadProjectionDriftState(wpId, packetPath, packetText) {
  const runtimeStatusFile = parseSingleField(packetText, "WP_RUNTIME_STATUS_FILE");
  const receiptsFile = parseSingleField(packetText, "WP_RECEIPTS_FILE");
  const runtimeStatusAbs = repoPathAbs(runtimeStatusFile);
  const receiptsAbs = repoPathAbs(receiptsFile);
  if (!runtimeStatusFile || !exists(runtimeStatusAbs)) return null;

  const runtimeStatus = parseJsonFile(runtimeStatusFile);
  const receipts = receiptsFile && exists(receiptsAbs) ? parseJsonlFile(receiptsFile) : [];
  const communicationEvaluation = evaluateWpCommunicationHealth({
    wpId,
    stage: "STATUS",
    packetPath,
    packetContent: packetText,
    workflowLane: parseSingleField(packetText, "WORKFLOW_LANE"),
    packetFormatVersion: parseSingleField(packetText, "PACKET_FORMAT_VERSION"),
    communicationContract: parseSingleField(packetText, "COMMUNICATION_CONTRACT"),
    communicationHealthGate: parseSingleField(packetText, "COMMUNICATION_HEALTH_GATE"),
    receipts,
    runtimeStatus,
  });
  return {
    runtimeStatus,
    receipts,
    communicationEvaluation,
    drift: evaluatePacketRuntimeProjectionDrift(packetText, runtimeStatus, {
      communicationEvaluation,
    }),
  };
}

function loadRelayEscalationState(repoRoot, wpId, packetRuntimeState, pendingNotifications = []) {
  if (!packetRuntimeState?.runtimeStatus || !packetRuntimeState?.communicationEvaluation) return null;
  const { registry } = loadSessionRegistry(repoRoot);
  return evaluateWpRelayEscalation({
    wpId,
    runtimeStatus: packetRuntimeState.runtimeStatus,
    communicationEvaluation: packetRuntimeState.communicationEvaluation,
    receipts: packetRuntimeState.receipts || [],
    pendingNotifications,
    registrySessions: registry.sessions || [],
  });
}

function loadNotificationState(wpId) {
  const byRole = checkAllNotifications({ wpId });
  return {
    byRole,
    pendingNotifications: Object.values(byRole).flatMap((entry) => entry.notifications || []),
  };
}

export function latestOrchestratorGovernanceCheckpoint(notificationsByRole = {}) {
  const notifications = notificationsByRole?.ORCHESTRATOR?.notifications || [];
  const checkpoints = notifications.filter((entry) => String(entry?.source_kind || "").trim().toUpperCase() === "GOVERNANCE_CHECKPOINT");
  return checkpoints.sort((left, right) => String(right?.timestamp_utc || "").localeCompare(String(left?.timestamp_utc || "")))[0] || null;
}

export function findActiveTokenBudgetContinuationWaiver(packetText = "") {
  const waiverLedger = parsePolicyWaiverLedger(packetText);
  return waiverLedger.activeEntries.find((entry) => {
    if (!entry.coverage.includes("GOVERNANCE")) return false;
    if (!String(entry.approver || "").trim()) return false;
    const joinedText = [entry.scope, entry.justification, entry.raw].filter(Boolean).join(" | ");
    return TOKEN_POLICY_CONFLICT_RE.test(joinedText);
  }) || null;
}

function confidenceDetailWithPolicyConflictWaiver(detail = "", waiver = null) {
  if (!waiver) return detail;
  const note = `token-budget continuation waiver active (${waiver.waiverId})`;
  return detail ? `${detail}; ${note}` : note;
}

export function tokenPolicyContinuationDecision({
  workflowLane = "",
  boardStatus = "",
  ledgerHealthSeverity = "",
  tokenBudgetStatus = "",
  waiver = null,
} = {}) {
  const orchestratorManaged = String(workflowLane || "").trim().toUpperCase() === "ORCHESTRATOR_MANAGED";
  const boardTerminal = String(boardStatus || "").trim().toUpperCase() === "VALIDATED";
  const continuationActive = orchestratorManaged && !boardTerminal && Boolean(waiver);
  const blockLedgerHealth = orchestratorManaged
    && !boardTerminal
    && String(ledgerHealthSeverity || "").trim().toUpperCase() === "FAIL"
    && !continuationActive;
  const blockBudget = orchestratorManaged
    && !boardTerminal
    && String(tokenBudgetStatus || "").trim().toUpperCase() === "FAIL"
    && !continuationActive;
  const findings = [];

  if (continuationActive && String(ledgerHealthSeverity || "").trim().toUpperCase() === "FAIL") {
    findings.push(
      `Continuation waiver ${waiver.waiverId} active: token-ledger policy remains FAIL and is tolerated for bounded continuation.`,
    );
  }
  if (continuationActive && String(tokenBudgetStatus || "").trim().toUpperCase() === "FAIL") {
    findings.push(
      `Continuation waiver ${waiver.waiverId} active: token-budget policy remains FAIL and is tolerated for bounded continuation.`,
    );
  }

  return {
    continuationActive,
    blockLedgerHealth,
    blockBudget,
    findings,
  };
}

function orchestratorAssessmentState(checkpointNotification, assessment = null, runtimeStatus = {}) {
  const nextActor = String(runtimeStatus?.next_expected_actor || "").trim().toUpperCase() || "UNCHANGED";
  if (!checkpointNotification && !(assessment && nextActor === "ORCHESTRATOR")) return null;
  const verdict = assessment?.verdict || "ASSESSED";
  const why = assessment?.reason || checkpointNotification?.summary || "Validator assessment recorded.";
  const waitingOn = String(runtimeStatus?.waiting_on || "").trim() || "<missing>";
  return {
    verdict,
    state: `Validator assessment recorded: ${verdict}. ${why}`,
    findings: [
      `Projected next actor: ${nextActor}${runtimeStatus?.next_expected_session ? `:${runtimeStatus.next_expected_session}` : ""}`,
      `Projected waiting_on: ${waitingOn}`,
      checkpointNotification ? `Pending orchestrator checkpoint: ${checkpointNotification.summary}` : null,
    ].filter(Boolean),
  };
}

export function closeoutModeFromPacketStatus(packetStatus = "") {
  const normalized = String(packetStatus || "").trim();
  if (normalized === "Done") return "DONE_MERGE_PENDING";
  if (/^Validated\s*\(\s*PASS\s*\)$/i.test(normalized)) return "DONE_VALIDATED";
  if (/^Validated\s*\(\s*FAIL\s*\)$/i.test(normalized)) return "DONE_FAIL";
  if (/^Validated\s*\(\s*ABANDONED\s*\)$/i.test(normalized)) return "DONE_ABANDONED";
  if (/^Validated\s*\(\s*OUTDATED_ONLY\s*\)$/i.test(normalized)) return "DONE_OUTDATED_ONLY";
  return "";
}

export function isTerminalOrchestratorBoardStatus(status = "") {
  return TERMINAL_ORCHESTRATOR_BOARD_STATUSES.has(String(status || "").trim().toUpperCase());
}

function closeoutSyncCommandForProjection(wpId, projection = {}, runtimeStatus = {}, communicationEvaluation = null) {
  const packetStatus = String(projection.current_packet_status || "").trim();
  const closeoutMode = closeoutModeFromPacketStatus(packetStatus);
  if (closeoutMode) {
    return `just task-board-set ${wpId} ${closeoutMode}`;
  }
  if (
    communicationEvaluation?.ok
    && String(communicationEvaluation.state || "").trim().toUpperCase() === "COMM_OK"
    && String(projection.current_main_compatibility_status || "").trim().toUpperCase() === "NOT_RUN"
  ) {
    return `just integration-validator-closeout-sync ${wpId} DONE_MERGE_PENDING`;
  }
  return `just integration-validator-context-brief ${wpId}`;
}

function relayCommandForRuntime(wpId, workflowLane = "", runtimeStatus = {}) {
  if (String(workflowLane || "").trim().toUpperCase() !== "ORCHESTRATOR_MANAGED") return "";
  const nextActor = String(runtimeStatus?.next_expected_actor || "").trim().toUpperCase();
  if (!GOVERNED_ROLE_RELAY_TARGETS.has(nextActor)) return "";
  return `just orchestrator-steer-next ${wpId}`;
}

function stageScore(stage, detail = {}) {
  switch (stage) {
    case "REFINEMENT":
      return detail.ready === false ? 170 : 160;
    case "APPROVAL":
      return 150;
    case "PREPARE":
      return 140;
    case "PACKET_CREATE":
      return 130;
    case "STATUS_SYNC":
      return 120;
    case "DELEGATION":
      return detail.needsStubCleanup ? 110 : 90;
    default:
      return 50;
  }
}

function freshnessBoost(timestamp) {
  const parsed = Date.parse(String(timestamp || ""));
  if (Number.isNaN(parsed)) return 0;
  const ageHours = Math.max(0, (Date.now() - parsed) / (1000 * 60 * 60));
  return Math.max(0, 24 - Math.min(24, ageHours / 2));
}

function summarizeResumeState(state, wpId) {
  const lastRefinement = lastLog(state, wpId, "REFINEMENT");
  const lastSignature = lastLog(state, wpId, "SIGNATURE");
  const lastPrepare = lastLog(state, wpId, "PREPARE");

  const refinementPath = defaultRefinementPath(wpId);
  const currentPacketPath = (resolveWorkPacketPath(wpId)?.packetPath || path.join(GOV_ROOT_REPO_REL, "task_packets", `${wpId}.md`)).replace(/\\/g, "/");
  const currentPacketAbsPath = repoPathAbs(currentPacketPath);
  const refinementExists = exists(refinementPath);
  const currentPacketExists = exists(currentPacketAbsPath);
  const boardStatus = taskBoardStatus(wpId) || "<none>";
  const needsStubCleanup = hasStubLine(wpId);
  const notificationState = currentPacketExists ? loadNotificationState(wpId) : { byRole: {}, pendingNotifications: [] };
  const orchestratorCheckpoint = latestOrchestratorGovernanceCheckpoint(notificationState.byRole);

  let refinementReady = false;
  let refinementError = "";
  if (refinementExists) {
    const ready = validateRefinementFile(refinementPath, {
      expectedWpId: wpId,
      requireSignature: false,
    });
    refinementReady = !!ready.ok;
    refinementError = ready.ok ? "" : (ready.errors || [])[0] || "Refinement is incomplete.";
  }

  let stage = "DELEGATION";
  let reason = "Work packet exists; ready to delegate to Coder.";
  let ready = true;
  const syncState = lastPrepare && currentPacketExists
    ? preparedWorktreeSyncState(wpId, lastPrepare, REPO_ROOT)
    : null;

  if (!refinementExists) {
    stage = "REFINEMENT";
    ready = false;
    reason = "Refinement file does not exist yet.";
  } else if (!refinementReady) {
    stage = "REFINEMENT";
    ready = false;
    reason = refinementError;
  } else if (!lastRefinement) {
    stage = "REFINEMENT";
    reason = "Refinement file looks reviewable, but no refinement gate log exists yet.";
  } else if (!lastSignature) {
    stage = "APPROVAL";
    reason = "Refinement recorded; signature not yet recorded.";
  } else if (!lastPrepare) {
    stage = "PREPARE";
    reason = "Signature recorded; WP prepare record missing.";
  } else if (!currentPacketExists) {
    stage = "PACKET_CREATE";
    reason = "Prepare recorded; work packet file does not exist yet.";
  } else if (syncState && !syncState.ok) {
    stage = "STATUS_SYNC";
    reason = syncState.issues[0] || "Assigned WP worktree is stale.";
  } else if (needsStubCleanup) {
    stage = "DELEGATION";
    reason = "Work packet exists; Task Board still lists this WP as [STUB].";
  } else if (orchestratorCheckpoint) {
    stage = "DELEGATION";
    reason = `Pending orchestrator governance checkpoint: ${orchestratorCheckpoint.summary}`;
  }

  const timestamp =
    lastPrepare?.timestamp ||
    lastSignature?.timestamp ||
    lastRefinement?.timestamp ||
    "";
  const score =
    stageScore(stage, { ready, needsStubCleanup }) +
    freshnessBoost(timestamp) +
    (orchestratorCheckpoint ? 35 : 0) +
    (boardStatus === "READY_FOR_DEV" ? 8 : 0) +
    (boardStatus === "IN_PROGRESS" ? 4 : 0);

  return {
    wpId,
    stage,
    reason,
    boardStatus,
    timestamp,
    score,
  };
}

function main() {
  const providedWpId = (process.argv[2] || "").trim();
  const gitContext = currentGitContext();
  const gateLogs = loadOrchestratorGateLogs();
  const repoRoot = gitContext.topLevel || REPO_ROOT;
  let inferred = providedWpId
    ? { wpId: providedWpId, source: "explicit", candidates: [providedWpId] }
    : inferOrchestratorWpId(gateLogs, gitContext, repoRoot);

  if (!providedWpId && !inferred.wpId) {
    const state = loadState();
    const ranked = activeOrchestratorCandidates(gateLogs, repoRoot)
      .map((entry) => summarizeResumeState(state, entry.wpId))
      .sort((left, right) => {
        if (right.score !== left.score) return right.score - left.score;
        return String(right.timestamp || "").localeCompare(String(left.timestamp || ""));
      });

    if (ranked.length === 1) {
      inferred = { wpId: ranked[0].wpId, source: "heuristic-ranked", candidates: [ranked[0].wpId] };
    } else if (ranked.length >= 2 && ranked[0].score - ranked[1].score >= 12) {
      inferred = { wpId: ranked[0].wpId, source: "heuristic-ranked", candidates: ranked.map((entry) => entry.wpId) };
    } else if (!ranked.length) {
      inferred = { wpId: null, source: "heuristic-ranked", candidates: [] };
    } else {
      inferred = { wpId: null, source: "heuristic-ranked", candidates: ranked.map((entry) => entry.wpId), ranked };
    }
  }

  const wpId = inferred.wpId;
  if (!wpId || !wpId.startsWith("WP-")) {
    const activeCandidates =
      inferred.ranked ||
      activeOrchestratorCandidates(gateLogs, repoRoot).map((entry) => ({
        wpId: entry.wpId,
        stage: entry.type,
        reason: `Latest orchestrator log: ${entry.type}`,
      }));
    const findings = [
      `Current branch: ${gitContext.branch || "<unknown>"}`,
      `Current worktree: ${gitContext.topLevel || "<unknown>"}`,
    ];

    if (activeCandidates.length > 0) {
      findings.push(
        `Active candidates: ${activeCandidates
          .slice(0, 5)
          .map((entry) => `${entry.wpId} (${entry.stage}: ${entry.reason})`)
          .join(", ")}`,
      );
    } else {
      findings.push("No active WP candidates were inferred from ORCHESTRATOR_GATES.json.");
    }

    printLifecycle({ wpId: "N/A", stage: "REFINEMENT", next: "STOP" });
    printOperatorEnvelope("NONE", "NONE");
    printConfidence("LOW", "multiple-or-no-candidates");
    printState("Unable to infer a single orchestrator WP to resume.");
    printFindings(findings);
    printNextCommands(
      activeCandidates.length > 0
        ? activeCandidates.slice(0, 5).map((entry) => `just orchestrator-next ${entry.wpId}`)
        : ["just orchestrator-next WP-{ID}"],
    );
    process.exit(1);
  }

  const boardStatus = taskBoardStatus(wpId);
  if (isTerminalOrchestratorBoardStatus(boardStatus)) {
    const packetPath = (resolveWorkPacketPath(wpId)?.packetPath || path.join(GOV_ROOT_REPO_REL, "task_packets", `${wpId}.md`)).replace(/\\/g, "/");
    printLifecycle({ wpId, stage: "STATUS_SYNC", next: "STOP" });
    printOperatorEnvelope("NONE", "NONE");
    printConfidence(inferred.source === "explicit" ? "HIGH" : "MEDIUM", inferred.source);
    printState(`WP is terminal on TASK_BOARD (${boardStatus}); this is packet history, not an active orchestrator resume target.`);
    printFindings([
      `Packet: ${packetPath}`,
      `Current branch: ${gitContext.branch || "<unknown>"}`,
      `Current worktree: ${gitContext.topLevel || "<unknown>"}`,
    ]);
    printNextCommands([
      `cat ${packetPath}`,
      `just orchestrator-next WP-{ACTIVE_ID}`,
    ]);
    return;
  }

  const confidence =
    inferred.source === "explicit" || inferred.source === "branch" || inferred.source === "prepare"
      ? { level: "HIGH", detail: inferred.source }
      : { level: "MEDIUM", detail: inferred.source };

  const state = loadState();
  const lastRefinement = lastLog(state, wpId, "REFINEMENT");
  const lastSignature = lastLog(state, wpId, "SIGNATURE");
  const lastPrepare = lastLog(state, wpId, "PREPARE");

  const refinementPath = defaultRefinementPath(wpId);
  const packetPath = (resolveWorkPacketPath(wpId)?.packetPath || path.join(GOV_ROOT_REPO_REL, "task_packets", `${wpId}.md`)).replace(/\\/g, "/");
  const packetAbsPath = repoPathAbs(packetPath);

  const refinementExists = exists(refinementPath);
  const packetExists = exists(packetAbsPath);

  let refinementReady = false;
  let refinementSigned = false;
  let refinementErrors = [];
  let refinementParsed = null;
  let confidenceDetail = confidence.detail;
  if (refinementExists) {
    const ready = validateRefinementFile(refinementPath, {
      expectedWpId: wpId,
      requireSignature: false,
    });
    refinementReady = !!ready.ok;
    refinementParsed = ready.parsed || refinementParsed;
    if (!ready.ok) refinementErrors = ready.errors || [];

    const signed = validateRefinementFile(refinementPath, {
      expectedWpId: wpId,
      requireSignature: true,
    });
    refinementSigned = !!signed.ok;
    refinementParsed = signed.parsed || refinementParsed;
  }

  // Phase inference (minimal and deterministic).
  if (!refinementExists) {
    printLifecycle({ wpId, stage: "REFINEMENT", next: "REFINEMENT" });
    printOperatorEnvelope("NONE", "NONE");
    printConfidence(confidence.level, confidenceDetail);
    printState("Refinement file does not exist yet.");
    printNextCommands([
      `just create-task-packet ${wpId}  # scaffolds ${refinementPath.replace(/\\/g, "/")} and exits BLOCKED`,
      `cat ${refinementPath.replace(/\\/g, "/")}`,
      `# Present the Technical Refinement Block in-chat; wait for explicit review.`,
      `just record-refinement ${wpId}`,
    ]);
    return;
  }

  if (!refinementReady) {
    printLifecycle({ wpId, stage: "REFINEMENT", next: "REFINEMENT" });
    printOperatorEnvelope("NONE", "NONE");
    printConfidence(confidence.level, confidenceDetail);
    const detail = refinementErrors.length > 0 ? refinementErrors[0] : "Refinement is incomplete.";
    printState(detail);
    printNextCommands([
      `cat ${refinementPath.replace(/\\/g, "/")}`,
      `# Fix refinement fields until it is reviewable.`,
      `just record-refinement ${wpId}`,
    ]);
    return;
  }

  if (!lastRefinement) {
    printLifecycle({ wpId, stage: "REFINEMENT", next: "APPROVAL" });
    printOperatorEnvelope("NONE", "NONE");
    printConfidence(confidence.level, confidenceDetail);
    printState("Refinement file looks reviewable, but no refinement gate log exists yet.");
    printNextCommands([`just record-refinement ${wpId}`]);
    return;
  }

  if (!lastSignature) {
    printLifecycle({ wpId, stage: "APPROVAL", next: "SIGNATURE" });
    printOperatorEnvelope(
      `Collect explicit approval + one-time signature bundle for ${wpId} (signature + workflow lane + execution owner)`,
      "PRE_SIGNATURE_APPROVAL_REQUIRED",
    );
    printConfidence(confidence.level, confidenceDetail);
    printState("Refinement recorded; signature not yet recorded.");
    printNextCommands([
      `# Paste the FULL Technical Refinement Block from ${refinementPath.replace(/\\/g, "/")} in chat (verbatim; no summary).`,
      `# Ensure refinement METADATA contains: - USER_APPROVAL_EVIDENCE: APPROVE REFINEMENT ${wpId}`,
      `just record-signature ${wpId} {usernameDDMMYYYYHHMM} {MANUAL_RELAY|ORCHESTRATOR_MANAGED} ${EXECUTION_OWNER_USAGE}`,
    ]);
    return;
  }

  if (!lastPrepare) {
    printLifecycle({ wpId, stage: "PREPARE", next: "PACKET_CREATE" });
    const workflowLane = lastSignature.workflow_lane || "";
    const executionLane = lastSignature.execution_lane || "";
    if (!workflowLane || !executionLane) {
      const prompt = !workflowLane && !executionLane
        ? `Choose workflow lane + execution owner for legacy PREPARE recovery (MANUAL_RELAY|ORCHESTRATOR_MANAGED + ${EXECUTION_OWNER_RANGE_HELP})`
        : !workflowLane
          ? `Choose workflow lane for legacy PREPARE recovery (${executionLane}; MANUAL_RELAY|ORCHESTRATOR_MANAGED)`
          : `Choose execution owner for legacy PREPARE recovery (${EXECUTION_OWNER_RANGE_HELP})`;
      printOperatorEnvelope(prompt, "LEGACY_SIGNATURE_TUPLE_REPAIR");
      printConfidence(confidence.level, confidenceDetail);
      printState("Signature recorded; WP prepare record missing and the legacy signature bundle did not capture the full workflow tuple.");
      printNextCommands([
        `just record-signature ${wpId} ${lastSignature.signature} ${workflowLane || '{MANUAL_RELAY|ORCHESTRATOR_MANAGED}'} ${executionLane || EXECUTION_OWNER_USAGE}`,
      ]);
      return;
    }

    printOperatorEnvelope("NONE", "NONE");
    printConfidence(confidence.level, confidenceDetail);
    printState(`Signature recorded; WP prepare record missing. Workflow lane from signature bundle: ${workflowLane}; execution owner: ${executionLane}.`);
    printNextCommands([
      `just orchestrator-prepare-and-packet ${wpId}`,
    ]);
    return;
  }

  if (!packetExists) {
    printLifecycle({ wpId, stage: "PACKET_CREATE", next: "PRE_WORK" });
    printOperatorEnvelope("NONE", "NONE");
    printConfidence(confidence.level, confidenceDetail);
    printState("Prepare recorded; work packet file does not exist yet.");
    const nextCommands = [`just create-task-packet ${wpId}`];
    if (!/^HYDRATED_RESEARCH_V1$/i.test(refinementParsed?.refinementEnforcementProfile || "")) {
      nextCommands.push(`# Fill legacy packet placeholders (UI/stub metadata, SCOPE, TEST_PLAN, DONE_MEANS, BOOTSTRAP, SPEC_ANCHOR).`);
    }
    nextCommands.push(`just pre-work ${wpId}`);
    nextCommands.push(`just task-board-set ${wpId} READY_FOR_DEV`);
    printNextCommands(nextCommands);
    return;
  }

  const needsStubCleanup = hasStubLine(wpId);
  const syncState = preparedWorktreeSyncState(wpId, lastPrepare, repoRoot);
  const packetText = fs.readFileSync(packetAbsPath, "utf8");
  const workflowLane = parseSingleField(packetText, "WORKFLOW_LANE");
  const packetFormatVersion = parseSingleField(packetText, "PACKET_FORMAT_VERSION");
  const dataContractProfile = parseSingleField(packetText, "DATA_CONTRACT_PROFILE") || "NONE";
  const coderHandoffRigorProfile = parseSingleField(packetText, "CODER_HANDOFF_RIGOR_PROFILE");
  const validatorReportProfile = parseSingleField(packetText, "GOVERNED_VALIDATOR_REPORT_PROFILE");
  const tokenBudgetContinuationWaiver = findActiveTokenBudgetContinuationWaiver(packetText);
  confidenceDetail = confidenceDetailWithPolicyConflictWaiver(
    confidence.detail,
    tokenBudgetContinuationWaiver,
  );
  const tokenLedger = readWpTokenUsageLedger(repoRoot, wpId).ledger;
  const tokenBudget = evaluateWpTokenBudget(tokenLedger);
  const tokenPolicyContinuation = tokenPolicyContinuationDecision({
    workflowLane,
    boardStatus,
    ledgerHealthSeverity: tokenLedger?.ledger_health?.severity,
    tokenBudgetStatus: tokenBudget?.status,
    waiver: tokenBudgetContinuationWaiver,
  });
  if (tokenPolicyContinuation.blockLedgerHealth) {
    printLifecycle({ wpId, stage: "DELEGATION", next: "STOP" });
    printOperatorEnvelope("NONE", tokenLedger.ledger_health.blocker_class || "POLICY_CONFLICT");
    printConfidence(confidence.level, confidenceDetail);
    printState(tokenLedger.ledger_health.summary);
    printFindings([
      `WP token ledger policy: ${tokenLedger.ledger_health.policy_id}`,
      ...tokenLedger.ledger_health.failures,
    ]);
    printNextCommands([
      `just session-registry-status ${wpId}`,
      `just wp-token-usage ${wpId}`,
      `# Repair token-ledger drift before resuming orchestrator-managed work.`,
    ]);
    return;
  }
  if (tokenPolicyContinuation.blockBudget) {
    printLifecycle({ wpId, stage: "DELEGATION", next: "STOP" });
    printOperatorEnvelope("NONE", tokenBudget.blocker_class || "POLICY_CONFLICT");
    printConfidence(confidence.level, confidenceDetail);
    printState(tokenBudget.summary);
    printFindings([
      `WP token budget policy: ${tokenBudget.policy_id}`,
      ...tokenBudget.failures,
    ]);
    printNextCommands([
      `just session-registry-status ${wpId}`,
      `just wp-token-usage ${wpId}`,
      `# Compact the lane and remove ambiguity-driven retries before resuming orchestrator-managed work.`,
    ]);
    return;
  }
  const packetRuntimeState = loadProjectionDriftState(wpId, packetPath, packetText);
  const notificationState = packetRuntimeState ? loadNotificationState(wpId) : { byRole: {}, pendingNotifications: [] };
  const relayEscalation = packetRuntimeState
    ? loadRelayEscalationState(repoRoot, wpId, packetRuntimeState, notificationState.pendingNotifications)
    : null;
  const orchestratorCheckpoint = latestOrchestratorGovernanceCheckpoint(notificationState.byRole);
  const latestValidatorAssessment = packetRuntimeState
    ? deriveLatestValidatorAssessment(packetRuntimeState.receipts || [])
    : null;
  const assessmentState = orchestratorAssessmentState(
    orchestratorCheckpoint,
    latestValidatorAssessment,
    packetRuntimeState?.runtimeStatus || {},
  );
  if (packetRuntimeState && !packetRuntimeState.drift.ok) {
    printLifecycle({ wpId, stage: "STATUS_SYNC", next: "STOP" });
    printOperatorEnvelope("NONE", "NONE");
    printConfidence(confidence.level, confidenceDetail);
    printState("Packet/runtime closeout projection drift is blocking further delegation until status truth is reconciled.");
    printFindings([
      ...tokenPolicyContinuation.findings,
      `Packet: ${packetPath}`,
      `Runtime: ${parseSingleField(packetText, "WP_RUNTIME_STATUS_FILE") || "<missing>"}`,
      ...packetRuntimeState.drift.issues,
    ]);
    printNextCommands([
      `just integration-validator-context-brief ${wpId}`,
      closeoutSyncCommandForProjection(
        wpId,
        packetRuntimeState.drift.projection,
        packetRuntimeState.runtimeStatus,
        packetRuntimeState.communicationEvaluation,
      ),
      `just orchestrator-next ${wpId}`,
    ]);
    return;
  }
  if (relayEscalation?.applicable && relayEscalation.status === "ESCALATED") {
    printLifecycle({ wpId, stage: "DELEGATION", next: "DELEGATION" });
    printOperatorEnvelope("NONE", "NONE");
    printConfidence(confidence.level, confidenceDetail);
    printState(
      assessmentState
        ? `${assessmentState.state} ${relayEscalation.summary}`
        : relayEscalation.summary
    );
    printFindings([
      ...tokenPolicyContinuation.findings,
      ...(assessmentState?.findings || []),
      `Target: ${relayEscalation.target_role}${relayEscalation.target_session ? `:${relayEscalation.target_session}` : ""}`,
      `Route anchor: ${relayEscalation.metrics.route_anchor_at || "<missing>"}`,
      `Latest notification: ${relayEscalation.metrics.latest_notification_at || "<none>"}`,
      `Latest target receipt: ${relayEscalation.metrics.latest_target_receipt_at || "<none>"}`,
      `Latest session activity: ${relayEscalation.metrics.latest_session_activity_at || "<none>"}`,
      ...relayEscalation.failures,
    ]);
    printNextCommands([
      relayEscalation.recommended_command,
      `just active-lane-brief ${relayEscalation.target_role} ${wpId}`,
      `just session-registry-status ${wpId}`,
      `just orchestrator-next ${wpId}`,
    ]);
    return;
  }
  if (!syncState.ok) {
    printLifecycle({ wpId, stage: "STATUS_SYNC", next: "STOP" });
    printOperatorEnvelope("NONE", "NONE");
    printConfidence(confidence.level, confidenceDetail);
    printState("Work packet exists, but the assigned WP worktree is stale and coder handoff is blocked.");
    printFindings([
      ...tokenPolicyContinuation.findings,
      `Assigned worktree: ${syncState.worktreeAbs || "<missing>"}`,
      `Expected branch: ${syncState.expectedBranch || "<missing>"}`,
      ...(syncState.actualBranch ? [`Actual branch: ${syncState.actualBranch}`] : []),
      ...syncState.issues,
    ]);
    printNextCommands([
      `# Validator: fast-forward ${syncState.expectedBranch || "the assigned WP branch"} and ${syncState.worktreeAbs || "the assigned WP worktree"} until they contain the official packet, current SPEC_CURRENT snapshot, current TASK_BOARD/traceability state, and current PREPARE record.`,
      `# Then re-run in ${syncState.worktreeAbs || "the assigned WP worktree"}: just pre-work ${wpId}`,
      `just orchestrator-next ${wpId}`,
    ]);
    return;
  }
  if (assessmentState) {
    printLifecycle({ wpId, stage: "DELEGATION", next: "DELEGATION" });
    printOperatorEnvelope("NONE", "NONE");
    printConfidence(confidence.level, confidenceDetail);
    printState(assessmentState.state);
    printFindings([
      ...tokenPolicyContinuation.findings,
      `Resume source: ${inferred.source}`,
      `Current branch: ${gitContext.branch || "<unknown>"}`,
      `Current worktree: ${gitContext.topLevel || "<unknown>"}`,
      ...assessmentState.findings,
      ...(relayEscalation?.applicable && relayEscalation.status === "WATCH"
        ? [relayEscalation.summary]
        : []),
    ]);
    const runtimeRelayCommand = relayCommandForRuntime(wpId, workflowLane, packetRuntimeState?.runtimeStatus || {});
    const nextCommands = [
      `just check-notifications ${wpId} ORCHESTRATOR`,
      runtimeRelayCommand || null,
      !runtimeRelayCommand && String(packetRuntimeState?.runtimeStatus?.next_expected_actor || "").trim().toUpperCase() === "ORCHESTRATOR"
        ? closeoutSyncCommandForProjection(
          wpId,
          packetRuntimeState?.drift?.projection || {},
          packetRuntimeState?.runtimeStatus || {},
          packetRuntimeState?.communicationEvaluation,
        )
        : null,
      `just session-registry-status ${wpId}`,
    ].filter(Boolean);
    printNextCommands(nextCommands);
    return;
  }
  printLifecycle({ wpId, stage: "DELEGATION", next: "DELEGATION" });
  printOperatorEnvelope("NONE", "NONE");
  printConfidence(confidence.level, confidenceDetail);
  printState(
    needsStubCleanup
      ? "Work packet exists; Task Board still lists this WP as [STUB]."
      : "Work packet exists; ready to delegate to Coder."
  );
  printFindings([
    ...tokenPolicyContinuation.findings,
    `Resume source: ${inferred.source}`,
    `Current branch: ${gitContext.branch || "<unknown>"}`,
    `Current worktree: ${gitContext.topLevel || "<unknown>"}`,
    ...(packetFormatVersion >= "2026-04-01"
      ? [`Packet law: format=${packetFormatVersion} | data_contract=${dataContractProfile} | handoff_rigor=${coderHandoffRigorProfile || "<unknown>"} | validator_report=${validatorReportProfile || "<unknown>"}`]
      : []),
    ...(packetFormatVersion >= "2026-04-01"
      ? ['Packet law: coder handoff must include anti-vibe + signed-scope-debt self-audit; validator PASS requires both lists to be exactly "- NONE".']
      : []),
    ...(packetFormatVersion >= "2026-04-01" && /^LLM_FIRST_DATA_V1$/i.test(dataContractProfile)
      ? ['Packet law: active data contract packet - DATA_CONTRACT_MONITORING must stay credible now, and validator closeout later requires concrete DATA_CONTRACT_PROOF plus DATA_CONTRACT_GAPS.']
      : []),
    ...(relayEscalation?.applicable && relayEscalation.status === "WATCH"
      ? [relayEscalation.summary]
      : []),
  ]);
  const runtimeRelayCommand = relayCommandForRuntime(wpId, workflowLane, packetRuntimeState?.runtimeStatus || {});
  const cmds = [
    `cat ${packetPath}`,
    `just pre-work ${wpId}`,
  ];
  if (runtimeRelayCommand) cmds.push(runtimeRelayCommand);
  if (String(workflowLane || "").trim().toUpperCase() === "MANUAL_RELAY") {
    cmds.push(`just manual-relay-next ${wpId}`);
  }
  if (needsStubCleanup) cmds.push(`just task-board-set ${wpId} READY_FOR_DEV`);
  if (!runtimeRelayCommand && String(workflowLane || "").trim().toUpperCase() !== "MANUAL_RELAY") {
    cmds.push(`just launch-coder-session ${wpId}`);
    cmds.push(`just launch-wp-validator-session ${wpId}`);
  }
  cmds.push(`just session-registry-status ${wpId}`);
  cmds.push(`# Integration Validator is downstream of WP validation PASS; launch later with: just launch-integration-validator-session ${wpId}`);
  printNextCommands(cmds);
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  main();
}
