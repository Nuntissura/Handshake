#!/usr/bin/env node
/**
 * Orchestrator "resume without context" helper.
 *
 * This is intentionally read-only: it inspects Orchestrator gates + filesystem
 * state and prints the next minimal commands to run.
 */

import fs from "node:fs";
import path from "node:path";
import { registerFailCaptureHook, failWithMemory } from "../../../roles_shared/scripts/lib/fail-capture-lib.mjs";
import { fileURLToPath } from "node:url";
import {
  defaultRefinementPath,
  validateRefinementFile,
} from "../../../roles_shared/checks/refinement-check.mjs";
import {
  activeOrchestratorCandidates,
  currentGitContext,
  displayRepoRelativePath,
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
import { buildPhaseCheckCommand } from "../../../roles_shared/checks/phase-check-lib.mjs";
import { loadSessionRegistry, registrySessionSummary } from "../../../roles_shared/scripts/session/session-registry-lib.mjs";
import { evaluateWpTokenBudget } from "../../../roles_shared/scripts/session/wp-token-budget-lib.mjs";
import { readWpTokenUsageLedger } from "../../../roles_shared/scripts/session/wp-token-usage-lib.mjs";
import { GOV_ROOT_REPO_REL, REPO_ROOT, repoPathAbs, resolveOrchestratorGatesPath, resolveWorkPacketPath, WORK_PACKET_STORAGE_ROOT_REPO_REL } from "../../../roles_shared/scripts/lib/runtime-paths.mjs";
import { evaluatePacketRuntimeProjectionDrift } from "../../../roles_shared/scripts/lib/packet-runtime-projection-lib.mjs";
import { deriveLatestValidatorAssessment, evaluateWpCommunicationHealth } from "../../../roles_shared/scripts/lib/wp-communication-health-lib.mjs";
import {
  normalizeRelayEscalationPolicy,
  relayEscalationPolicyBudgetLabel,
} from "../../../roles_shared/scripts/lib/wp-relay-policy-lib.mjs";
import { evaluateWpRelayEscalation } from "../../../roles_shared/scripts/lib/wp-relay-escalation-lib.mjs";
import {
  inferExecutionCloseoutMode,
  materializeRuntimeAuthorityView,
  readExecutionPublicationView,
} from "../../../roles_shared/scripts/lib/wp-execution-state-lib.mjs";
import { parseJsonFile, parseJsonlFile } from "../../../roles_shared/scripts/lib/wp-communications-lib.mjs";
import { checkAllNotifications } from "../../../roles_shared/scripts/wp/wp-check-notifications.mjs";
import { parsePolicyWaiverLedger } from "../../../roles_shared/scripts/lib/computed-policy-gate-lib.mjs";
import {
  buildActivationManagerLaunchCommands,
  buildDownstreamGovernedLaunchCommands,
  buildManualRelayCommands,
  normalizeWorkflowLane,
  readActivationReadinessState,
} from "./lib/workflow-lane-guidance-lib.mjs";
import { nextQueuedControlRequest, pendingControlQueueCount } from "./lib/orchestrator-steer-lib.mjs";

function displayPathForOperator(value) {
  const normalized = String(value || "").trim();
  if (!normalized) return "<unknown>";
  return displayRepoRelativePath(normalized, REPO_ROOT) || ".";
}

// RGF-129/G4: Surface governance memory insights for the active WP
function loadMemoryInsights(wpId) {
  try {
    const { DatabaseSync } = require("node:sqlite");
    const dbPath = path.join(REPO_ROOT, "..", "gov_runtime", "roles_shared", "GOVERNANCE_MEMORY.db");
    if (!fs.existsSync(dbPath)) return [];
    const db = new DatabaseSync(dbPath, { readOnly: true });
    try {
      // Count procedural (fail log) entries for this WP
      const procCount = db.prepare(
        "SELECT COUNT(*) as cnt FROM memory_index WHERE memory_type = 'procedural' AND consolidated = 0 AND (wp_id = ? OR wp_id = '')"
      ).get(wpId)?.cnt || 0;
      // Find high-access systemic patterns (accessed 5+ times)
      const systemic = db.prepare(
        "SELECT topic, access_count FROM memory_index WHERE consolidated = 0 AND access_count >= 5 ORDER BY access_count DESC LIMIT 3"
      ).all();
      // Count REPAIR receipts for this WP
      const repairCount = db.prepare(
        "SELECT COUNT(*) as cnt FROM memory_index WHERE memory_type = 'procedural' AND consolidated = 0 AND wp_id = ? AND topic LIKE 'Fix pattern:%'"
      ).get(wpId)?.cnt || 0;

      const lines = [];
      if (procCount > 0) lines.push(`Memory: ${procCount} procedural fix patterns available (${repairCount} WP-specific REPAIRs)`);
      if (systemic.length > 0) lines.push(`Memory: systemic patterns — ${systemic.map(s => `"${s.topic}" (${s.access_count}x)`).join(", ")}`);
      return lines;
    } finally { try { db.close(); } catch {} }
  } catch { return []; }
}

const STATE_FILE = resolveOrchestratorGatesPath();
const STATE_FILE_ABS = repoPathAbs(STATE_FILE);
const TASK_BOARD_PATH = `${GOV_ROOT_REPO_REL}/roles_shared/records/TASK_BOARD.md`;
const TASK_BOARD_ABS_PATH = repoPathAbs(TASK_BOARD_PATH);
const EXECUTION_OWNER_USAGE = `{${EXECUTION_OWNER_RANGE_HELP}}`;
const GOVERNED_ROLE_RELAY_TARGETS = new Set(["CODER", "WP_VALIDATOR", "INTEGRATION_VALIDATOR"]);
const TERMINAL_ORCHESTRATOR_BOARD_STATUSES = new Set(["VALIDATED", "FAIL", "OUTDATED_ONLY", "ABANDONED", "SUPERSEDED"]);
const TOKEN_POLICY_CONFLICT_RE = /TOKEN_BUDGET_EXCEEDED|POLICY_CONFLICT|token\s+budget|token-ledger\s+drift/i;

registerFailCaptureHook("orchestrator-next.mjs", { role: "ORCHESTRATOR" });

function fail(message, details = []) {
  failWithMemory("orchestrator-next.mjs", message, { role: "ORCHESTRATOR", details });
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

  const runtimeStatus = materializeRuntimeAuthorityView(parseJsonFile(runtimeStatusFile));
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

function loadRelayEscalationState(wpId, packetRuntimeState, pendingNotifications = [], registrySessions = []) {
  if (!packetRuntimeState?.runtimeStatus || !packetRuntimeState?.communicationEvaluation) return null;
  return evaluateWpRelayEscalation({
    wpId,
    runtimeStatus: packetRuntimeState.runtimeStatus,
    communicationEvaluation: packetRuntimeState.communicationEvaluation,
    receipts: packetRuntimeState.receipts || [],
    pendingNotifications,
    registrySessions,
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

export function latestOrchestratorAcpHealthAlert(notificationsByRole = {}) {
  const notifications = notificationsByRole?.ORCHESTRATOR?.notifications || [];
  const alerts = notifications.filter((entry) => String(entry?.source_kind || "").trim().toUpperCase() === "ACP_HEALTH_ALERT");
  return alerts.sort((left, right) => String(right?.timestamp_utc || "").localeCompare(String(left?.timestamp_utc || "")))[0] || null;
}

export function latestOrchestratorRelayWatchdogRepair(notificationsByRole = {}) {
  const notifications = notificationsByRole?.ORCHESTRATOR?.notifications || [];
  const repairs = notifications.filter((entry) => String(entry?.source_kind || "").trim().toUpperCase() === "RELAY_WATCHDOG_REPAIR");
  return repairs.sort((left, right) => String(right?.timestamp_utc || "").localeCompare(String(left?.timestamp_utc || "")))[0] || null;
}

export function relayEscalationPolicyFindings(policy = null) {
  const normalized = normalizeRelayEscalationPolicy(policy);
  if (!normalized) return [];
  return [
    `Relay failure class: ${normalized.failure_class}`,
    `Relay policy: ${normalized.policy_state} -> ${normalized.next_strategy}`,
    normalized.budget_scope === "NONE"
      ? "Relay strategy budget: NONE"
      : `Relay strategy budget: ${relayEscalationPolicyBudgetLabel(normalized)}`,
    normalized.reason_code ? `Relay policy reason: ${normalized.reason_code}` : null,
    normalized.summary ? `Relay policy summary: ${normalized.summary}` : null,
  ].filter(Boolean);
}

export function queuedGovernedWaitState({
  workflowLane = "",
  runtimeStatus = {},
  registrySessions = [],
} = {}) {
  if (normalizeWorkflowLane(workflowLane) !== "ORCHESTRATOR_MANAGED") return null;
  const nextActor = String(runtimeStatus?.next_expected_actor || "").trim().toUpperCase();
  if (!GOVERNED_ROLE_RELAY_TARGETS.has(nextActor)) return null;

  const matchingSession = (registrySessions || []).find((entry) =>
    String(entry?.role || "").trim().toUpperCase() === nextActor
  ) || null;
  if (!matchingSession) return null;

  const queueCount = pendingControlQueueCount(matchingSession);
  if (queueCount <= 0) return null;

  const queuedRequest = nextQueuedControlRequest(matchingSession);
  return {
    role: nextActor,
    session: matchingSession,
    queueCount,
    queuedRequest,
    target: `${nextActor}${runtimeStatus?.next_expected_session ? `:${runtimeStatus.next_expected_session}` : ""}`,
  };
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
  const note = `legacy token-budget waiver recorded (${waiver.waiverId}; diagnostic-only cost policy no longer requires continuation waivers)`;
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
  const laneRequiresDiagnostics = orchestratorManaged && !boardTerminal;
  const ledgerFail = laneRequiresDiagnostics
    && String(ledgerHealthSeverity || "").trim().toUpperCase() === "FAIL";
  const budgetFail = laneRequiresDiagnostics
    && String(tokenBudgetStatus || "").trim().toUpperCase() === "FAIL";
  const findings = [];

  if (ledgerFail) {
    findings.push(
      "Token-ledger policy is FAIL, but ledger drift is governance telemetry only and must not stop orchestrator-managed continuation.",
    );
  }
  if (budgetFail) {
    findings.push(
      "Token-budget policy is FAIL, but high token spend is diagnostic only and must be recorded mechanically rather than blocking the WP.",
    );
  }
  if ((ledgerFail || budgetFail) && continuationActive) {
    findings.push(
      `Legacy continuation waiver ${waiver.waiverId} is recorded, but diagnostic-only cost policy no longer requires a waiver to continue.`,
    );
  }

  return {
    continuationActive,
    blockLedgerHealth: false,
    blockBudget: false,
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
  return inferExecutionCloseoutMode({ packetStatus })?.task_board_status || "";
}

export function isTerminalOrchestratorBoardStatus(status = "") {
  return TERMINAL_ORCHESTRATOR_BOARD_STATUSES.has(String(status || "").trim().toUpperCase());
}

export function publicationTaskBoardHistoryStatus(status = "") {
  switch (String(status || "").trim().toUpperCase()) {
    case "DONE_VALIDATED":
      return "VALIDATED";
    case "DONE_MERGE_PENDING":
      return "MERGE_PENDING";
    case "DONE_FAIL":
      return "FAIL";
    case "DONE_OUTDATED_ONLY":
      return "OUTDATED_ONLY";
    case "DONE_ABANDONED":
      return "ABANDONED";
    default:
      return String(status || "").trim().toUpperCase();
  }
}

export function closeoutSyncCommandForProjection(
  wpId,
  projection = {},
  runtimeStatus = {},
  communicationEvaluation = null,
  currentBoardStatus = "",
) {
  const publication = readExecutionPublicationView({
    runtimeStatus,
    packetStatus: projection.current_packet_status,
    taskBoardStatus: projection.current_task_board_status,
  });
  const closeoutMode = inferExecutionCloseoutMode({
    packetStatus: publication.packet_status,
    taskBoardStatus: publication.task_board_status,
    mainContainmentStatus: publication.runtime?.main_containment_status,
  });
  if (closeoutMode) {
    if (publication.has_canonical_authority && publication.packet_projection_drift) {
      const mergedMainShaSegment = closeoutMode.require_merged_main_commit
        ? " --merged-main-sha <MERGED_MAIN_SHA>"
        : "";
      return `just phase-check CLOSEOUT ${wpId} --sync-mode ${closeoutMode.mode}${mergedMainShaSegment} --context "<why this closeout truth is being recorded, >=40 chars>"`;
    }
    const currentBoardToken = publicationTaskBoardHistoryStatus(currentBoardStatus);
    const targetBoardToken = publicationTaskBoardHistoryStatus(closeoutMode.task_board_status);
    if (currentBoardToken && targetBoardToken && currentBoardToken === targetBoardToken) {
      return "";
    }
    return `just task-board-set ${wpId} ${closeoutMode.task_board_status}`;
  }
  if (
    communicationEvaluation?.ok
    && String(communicationEvaluation.state || "").trim().toUpperCase() === "COMM_OK"
    && String(projection.current_main_compatibility_status || "").trim().toUpperCase() === "NOT_RUN"
  ) {
    return `just phase-check CLOSEOUT ${wpId} --sync-mode MERGE_PENDING --context "<why this closeout truth is being recorded, >=40 chars>"`;
  }
  return `just integration-validator-context-brief ${wpId}`;
}

function relayCommandForRuntime(wpId, workflowLane = "", runtimeStatus = {}) {
  if (normalizeWorkflowLane(workflowLane) !== "ORCHESTRATOR_MANAGED") return "";
  const nextActor = String(runtimeStatus?.next_expected_actor || "").trim().toUpperCase();
  if (!GOVERNED_ROLE_RELAY_TARGETS.has(nextActor)) return "";
  return `just orchestrator-steer-next ${wpId} "<why this stalled relay should be re-woken, >=40 chars>"`;
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
  const currentPacketPath = (resolveWorkPacketPath(wpId)?.packetPath || path.join(WORK_PACKET_STORAGE_ROOT_REPO_REL, `${wpId}.md`)).replace(/\\/g, "/");
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
  const cliArgs = process.argv.slice(2);
  const debugMode = cliArgs.some((arg) => String(arg || "").trim() === "--debug");
  const positionalArgs = cliArgs.filter((arg) => {
    const normalized = String(arg || "").trim();
    return normalized && !normalized.startsWith("--");
  });

  const providedWpId = (positionalArgs[0] || "").trim();

  if (debugMode) {
    console.log("[ORCHESTRATOR_NEXT] debug_mode=enabled");
  }

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
      `Current worktree: ${displayPathForOperator(gitContext.topLevel)}`,
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
    const packetPath = (resolveWorkPacketPath(wpId)?.packetPath || path.join(WORK_PACKET_STORAGE_ROOT_REPO_REL, `${wpId}.md`)).replace(/\\/g, "/");
    printLifecycle({ wpId, stage: "STATUS_SYNC", next: "STOP" });
    printOperatorEnvelope("NONE", "NONE");
    printConfidence(inferred.source === "explicit" ? "HIGH" : "MEDIUM", inferred.source);
    printState(`WP is terminal on TASK_BOARD (${boardStatus}); this is packet history, not an active orchestrator resume target.`);
    printFindings([
      `Packet: ${packetPath}`,
      `Current branch: ${gitContext.branch || "<unknown>"}`,
      `Current worktree: ${displayPathForOperator(gitContext.topLevel)}`,
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
  const packetPath = (resolveWorkPacketPath(wpId)?.packetPath || path.join(WORK_PACKET_STORAGE_ROOT_REPO_REL, `${wpId}.md`)).replace(/\\/g, "/");
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
    const startupCommand = buildPhaseCheckCommand({ phase: "STARTUP", wpId, role: "CODER" });
    printLifecycle({ wpId, stage: "PACKET_CREATE", next: "PRE_WORK" });
    printOperatorEnvelope("NONE", "NONE");
    printConfidence(confidence.level, confidenceDetail);
    printState("Prepare recorded; work packet file does not exist yet.");
    const nextCommands = [`just create-task-packet ${wpId}`];
    if (!/^HYDRATED_RESEARCH_V1$/i.test(refinementParsed?.refinementEnforcementProfile || "")) {
      nextCommands.push(`# Fill legacy packet placeholders (UI/stub metadata, SCOPE, TEST_PLAN, DONE_MEANS, BOOTSTRAP, SPEC_ANCHOR).`);
    }
    nextCommands.push(startupCommand);
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
  const packetRuntimeState = loadProjectionDriftState(wpId, packetPath, packetText);
  const notificationState = packetRuntimeState ? loadNotificationState(wpId) : { byRole: {}, pendingNotifications: [] };
  const { registry } = packetRuntimeState ? loadSessionRegistry(repoRoot) : { registry: { sessions: [] } };
  const governedSessions = (registry.sessions || []).filter((entry) => String(entry?.wp_id || "").trim() === wpId);
  const governedSessionSummaries = governedSessions.map((entry) => registrySessionSummary(entry));
  const relayEscalation = packetRuntimeState
    ? loadRelayEscalationState(wpId, packetRuntimeState, notificationState.pendingNotifications, governedSessions)
    : null;
  const orchestratorCheckpoint = latestOrchestratorGovernanceCheckpoint(notificationState.byRole);
  const relayWatchdogRepair = latestOrchestratorRelayWatchdogRepair(notificationState.byRole);
  if (relayWatchdogRepair) {
    printLifecycle({ wpId, stage: "DELEGATION", next: "STOP" });
    printOperatorEnvelope("NONE", "RETRY_SUPPRESSION_ACTIVE");
    printConfidence(confidence.level, confidenceDetail);
    printState("Relay watchdog has suppressed duplicate automatic re-wakes for the current failure class; route repair must change the lane state before another governed wake is attempted.");
    printFindings([
      ...tokenPolicyContinuation.findings,
      ...relayEscalationPolicyFindings(packetRuntimeState?.runtimeStatus?.relay_escalation_policy),
      `Repair summary: ${relayWatchdogRepair.summary || "<missing>"}`,
      `Repair correlation: ${relayWatchdogRepair.correlation_id || "<missing>"}`,
      `Packet: ${packetPath}`,
    ]);
    printNextCommands([
      `just check-notifications ${wpId} ORCHESTRATOR`,
      `just session-registry-status ${wpId}`,
      `just wp-relay-watchdog ${wpId} --observe-only`,
      `just wp-lane-health ${wpId}`,
      `just orchestrator-next ${wpId}`,
    ]);
    return;
  }
  const acpHealthAlert = latestOrchestratorAcpHealthAlert(notificationState.byRole);
  if (acpHealthAlert) {
    printLifecycle({ wpId, stage: "DELEGATION", next: "STOP" });
    printOperatorEnvelope("NONE", "ACP_SESSION_HEALTH");
    printConfidence(confidence.level, confidenceDetail);
    printState("ACP/session health alert is blocking reliable governed dispatch until session-control health is repaired.");
    printFindings([
      ...tokenPolicyContinuation.findings,
      `Alert summary: ${acpHealthAlert.summary || "<missing>"}`,
      `Alert correlation: ${acpHealthAlert.correlation_id || "<missing>"}`,
      `Packet: ${packetPath}`,
    ]);
    printNextCommands([
      `just check-notifications ${wpId} ORCHESTRATOR`,
      `just session-registry-status ${wpId}`,
      `just wp-lane-health ${wpId}`,
      `just broker-status`,
      `just wp-relay-watchdog ${wpId} --observe-only`,
      `just orchestrator-next ${wpId}`,
    ]);
    return;
  }
  const latestValidatorAssessment = packetRuntimeState
    ? deriveLatestValidatorAssessment(packetRuntimeState.receipts || [])
    : null;
  const assessmentState = orchestratorAssessmentState(
    orchestratorCheckpoint,
    latestValidatorAssessment,
    packetRuntimeState?.runtimeStatus || {},
  );
  if (packetRuntimeState && !packetRuntimeState.drift.ok) {
    const driftOwners = Array.isArray(packetRuntimeState.drift.owner_classes)
      ? packetRuntimeState.drift.owner_classes
      : [];
    const blockerClass = driftOwners.length > 1
      ? "TRUTH_AUTHORITY_DRIFT"
      : (driftOwners[0] || "STATUS_SYNC_DRIFT");
    printLifecycle({ wpId, stage: "STATUS_SYNC", next: "STOP" });
    printOperatorEnvelope("NONE", blockerClass);
    printConfidence(confidence.level, confidenceDetail);
    printState("Packet/runtime closeout projection drift is blocking further delegation until status truth is reconciled.");
    printFindings([
      ...tokenPolicyContinuation.findings,
      ...(packetRuntimeState.drift.owner_summary ? [packetRuntimeState.drift.owner_summary] : []),
      ...(driftOwners.length > 0 ? [`Repair order: ${driftOwners.join(" -> ")}`] : []),
      `Packet: ${packetPath}`,
      `Runtime: ${parseSingleField(packetText, "WP_RUNTIME_STATUS_FILE") || "<missing>"}`,
      ...(Array.isArray(packetRuntimeState.drift.issue_details) && packetRuntimeState.drift.issue_details.length > 0
        ? packetRuntimeState.drift.issue_details.map((detail) => `[${detail.owner}] ${detail.message}`)
        : packetRuntimeState.drift.issues),
    ]);
    printNextCommands([
      `just integration-validator-context-brief ${wpId}`,
      closeoutSyncCommandForProjection(
        wpId,
        packetRuntimeState.drift.projection,
        packetRuntimeState.runtimeStatus,
        packetRuntimeState.communicationEvaluation,
        boardStatus,
      ),
      `just orchestrator-next ${wpId}`,
    ]);
    return;
  }
  const queueWaitState = queuedGovernedWaitState({
    workflowLane,
    runtimeStatus: packetRuntimeState?.runtimeStatus || {},
    registrySessions: governedSessionSummaries,
  });
  if (queueWaitState) {
    printLifecycle({ wpId, stage: "DELEGATION", next: "DELEGATION" });
    printOperatorEnvelope("NONE", "NONE");
    printConfidence(confidence.level, confidenceDetail);
    printState(`Queue-backed governed follow-up is already accepted for ${queueWaitState.target}; wait for broker drain instead of resending another steer.`);
    printFindings([
      ...tokenPolicyContinuation.findings,
      `Target: ${queueWaitState.target}`,
      `Queue depth: ${queueWaitState.queueCount}`,
      `Queued command: ${queueWaitState.queuedRequest?.command_kind || "<unknown>"}`,
      `Queued at: ${queueWaitState.queuedRequest?.queued_at || queueWaitState.session?.updated_at || "<none>"}`,
      `Blocking command: ${queueWaitState.queuedRequest?.blocking_command_id || "<none>"}`,
      `Thread: ${queueWaitState.session?.session_thread_id || "<none>"}`,
      ...(queueWaitState.queuedRequest?.summary ? [`Queue summary: ${queueWaitState.queuedRequest.summary}`] : []),
    ]);
    printNextCommands([
      `just active-lane-brief ${queueWaitState.role} ${wpId}`,
      `just session-registry-status ${wpId}`,
      `just wp-lane-health ${wpId}`,
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
      ...relayEscalationPolicyFindings(packetRuntimeState?.runtimeStatus?.relay_escalation_policy),
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
    const startupCommand = buildPhaseCheckCommand({ phase: "STARTUP", wpId, role: "CODER" });
    printLifecycle({ wpId, stage: "STATUS_SYNC", next: "STOP" });
    printOperatorEnvelope("NONE", "NONE");
    printConfidence(confidence.level, confidenceDetail);
    printState("Work packet exists, but the assigned WP worktree is stale and coder handoff is blocked.");
    printFindings([
      ...tokenPolicyContinuation.findings,
      `Assigned worktree: ${syncState.worktreeDisplay || "<missing>"}`,
      `Expected branch: ${syncState.expectedBranch || "<missing>"}`,
      ...(syncState.actualBranch ? [`Actual branch: ${syncState.actualBranch}`] : []),
      ...syncState.issues,
    ]);
    printNextCommands([
      `# Validator: fast-forward ${syncState.expectedBranch || "the assigned WP branch"} and ${syncState.worktreeDisplay || "the assigned WP worktree"} until they contain the official packet, current SPEC_CURRENT snapshot, current TASK_BOARD/traceability state, and current PREPARE record.`,
      `# Then re-run in ${syncState.worktreeDisplay || "the assigned WP worktree"}: ${startupCommand}`,
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
      `Current worktree: ${displayPathForOperator(gitContext.topLevel)}`,
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
          boardStatus,
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
  const normalizedWorkflowLane = normalizeWorkflowLane(workflowLane);
  const activationReadiness = normalizedWorkflowLane === "ORCHESTRATOR_MANAGED"
    ? readActivationReadinessState(wpId)
    : null;

  printFindings([
    ...tokenPolicyContinuation.findings,
    `Resume source: ${inferred.source}`,
    `Current branch: ${gitContext.branch || "<unknown>"}`,
    `Current worktree: ${displayPathForOperator(gitContext.topLevel)}`,
    ...(packetFormatVersion >= "2026-04-01"
      ? [`Packet law: format=${packetFormatVersion} | data_contract=${dataContractProfile} | handoff_rigor=${coderHandoffRigorProfile || "<unknown>"} | validator_report=${validatorReportProfile || "<unknown>"}`]
      : []),
    ...(packetFormatVersion >= "2026-04-01"
      ? ['Packet law: coder handoff must include anti-vibe + signed-scope-debt self-audit; validator PASS requires both lists to be exactly "- NONE".']
      : []),
    ...(packetFormatVersion >= "2026-04-05"
      ? ['Packet law: medium/high V4 validator closeout is dual-track; PASS later requires both MECHANICAL_TRACK_VERDICT=PASS and SPEC_RETENTION_TRACK_VERDICT=PASS.']
      : []),
    ...(packetFormatVersion >= "2026-04-01" && /^LLM_FIRST_DATA_V1$/i.test(dataContractProfile)
      ? ['Packet law: active data contract packet - DATA_CONTRACT_MONITORING must stay credible now, and validator closeout later requires concrete DATA_CONTRACT_PROOF plus DATA_CONTRACT_GAPS.']
      : []),
    ...(relayEscalation?.applicable && relayEscalation.status === "WATCH"
      ? [relayEscalation.summary]
      : []),
    ...(activationReadiness
      ? [`ACTIVATION_READINESS: ${activationReadiness.verdict} (${activationReadiness.path})`]
      : []),
    ...loadMemoryInsights(wpId),
  ]);
  const runtimeRelayCommand = relayCommandForRuntime(wpId, workflowLane, packetRuntimeState?.runtimeStatus || {});
  const cmds = [`cat ${packetPath}`];
  if (runtimeRelayCommand) {
    cmds.push(runtimeRelayCommand);
    cmds.push(`just session-registry-status ${wpId}`);
  } else if (normalizedWorkflowLane === "MANUAL_RELAY") {
    cmds.push(...buildManualRelayCommands(wpId));
  } else if (normalizedWorkflowLane === "ORCHESTRATOR_MANAGED") {
    if (activationReadiness?.readyForDownstreamLaunch) {
      cmds.push(buildPhaseCheckCommand({ phase: "STARTUP", wpId, role: "CODER" }));
      cmds.push(...buildDownstreamGovernedLaunchCommands(wpId));
    } else {
      cmds.push(...buildActivationManagerLaunchCommands(wpId, activationReadiness));
    }
  } else {
    cmds.push(`just session-registry-status ${wpId}`);
  }
  if (needsStubCleanup) cmds.push(`just task-board-set ${wpId} READY_FOR_DEV`);
  printNextCommands(cmds);
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  main();
}
