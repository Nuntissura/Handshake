#!/usr/bin/env node

import { execFileSync } from "node:child_process";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import {
  communicationTransactionLockPathForWp,
  deriveAuthorityKinds,
  normalize,
  normalizeWorkflowInvalidityCode,
  parseJsonFile,
  parseJsonlFile,
  REVIEW_OPEN_RECEIPT_KIND_VALUES,
  REVIEW_RESOLUTION_RECEIPT_KIND_VALUES,
  REVIEW_TRACKED_RECEIPT_KIND_VALUES,
  validateReceipt,
  validateRuntimeStatus,
  WORKFLOW_INVALIDITY_RECEIPT_KIND,
} from "../lib/wp-communications-lib.mjs";
import {
  classifyWpChangedPath,
  deriveWpScopeContract,
  formatBoundedItemList,
  isGovernanceOnlyPath,
  isTransientProofArtifactPath,
  normalizeRepoPath,
} from "../lib/scope-surface-lib.mjs";
import {
  buildPhaseCheckCommand,
  buildPostWorkCommand,
  lastGateLog,
  loadOrchestratorGateLogs,
  preparedWorktreeSyncState,
  resolveCommittedCoderHandoffRange,
  resolvePrepareWorktreeAbs,
} from "../lib/role-resume-utils.mjs";
import {
  deriveLatestValidatorAssessment,
  deriveValidatorAssessmentVerdict,
  deriveWpCommunicationAutoRoute,
  evaluateWpCommunicationHealth,
  buildRoleInbox,
  isDuplicateDecisiveValidatorAssessment,
  isOverlapMicrotaskReviewItem,
  MAX_OVERLAP_MICROTASK_REVIEW_ITEMS,
} from "../lib/wp-communication-health-lib.mjs";
import {
  deriveWpMicrotaskPlan,
  listDeclaredWpMicrotasks,
  resolveDeclaredWpMicrotaskByScopeRef,
  summarizeMicrotaskFileTargetBudget,
} from "../lib/wp-microtask-lib.mjs";
import { GOV_ROOT_REPO_REL, REPO_ROOT, repoPathAbs, workPacketPath } from "../lib/runtime-paths.mjs";
import { isInvokedAsMain } from "../lib/invocation-path-lib.mjs";
import { runAbsorber } from "../lib/artifact-normalizers/index.mjs";
import {
  HEURISTIC_RISK_REPAIR_ESCALATION_THRESHOLD,
  isHeuristicRiskContract,
  mergeHeuristicRiskContract,
  summarizeHeuristicRiskContract,
} from "../lib/heuristic-risk-lib.mjs";
import { renderInterRoleVerbReceipt, validateInterRoleVerbBody } from "../lib/inter-role-verb-lib.mjs";
import { appendJsonlLine, withFileLockSync, writeJsonFile } from "../session/session-registry-lib.mjs";
import { tryAppendSessionEvent } from "../session/predecessor-lookup-lib.mjs";
import { appendWpNotification, appendWpNotificationCore } from "./wp-notification-append.mjs";
import { reconcileWpCommunicationTruth, syncProjectedTaskBoardTruth } from "./ensure-wp-communications.mjs";
import { openGovernanceMemoryDb, closeDb as closeMemDb, extractMemoryFromReceipt, HIGH_SIGNAL_RECEIPT_KINDS } from "../memory/governance-memory-lib.mjs";

const ACTIVE_AUTO_RELAY_ROLE_VALUES = new Set(["CODER", "WP_VALIDATOR", "INTEGRATION_VALIDATOR"]);
const VALIDATOR_ASSESSMENT_ROLE_VALUES = new Set(["WP_VALIDATOR", "INTEGRATION_VALIDATOR", "VALIDATOR"]);
const GOVERNANCE_CHECKPOINT_RECEIPT_KIND_VALUES = new Set(REVIEW_RESOLUTION_RECEIPT_KIND_VALUES);
export const REVIEW_NOTIFICATION_APPEND_OPTIONS = Object.freeze({
  assumeTransactionLock: true,
  autoRelay: false,
});
const ORCHESTRATOR_STEER_SCRIPT_PATH = path.resolve(
  path.dirname(fileURLToPath(import.meta.url)),
  "../../..",
  "roles",
  "orchestrator",
  "scripts",
  "orchestrator-steer-next.mjs",
);
const POST_WORK_CHECK_SCRIPT_PATH = path.resolve(
  path.dirname(fileURLToPath(import.meta.url)),
  "../../..",
  "roles",
  "coder",
  "checks",
  "post-work-check.mjs",
);
const TASK_BOARD_ABS_PATH = repoPathAbs(`${GOV_ROOT_REPO_REL}/roles_shared/records/TASK_BOARD.md`);
const BUILD_ORDER_ABS_PATH = repoPathAbs(`${GOV_ROOT_REPO_REL}/roles_shared/records/BUILD_ORDER.md`);
const PHASE_CHECK_SCRIPT_ABS_PATH = repoPathAbs(`${GOV_ROOT_REPO_REL}/roles_shared/checks/phase-check.mjs`);
const MICROTASK_SCOPE_BUDGET_RECEIPT_KIND_VALUES = new Set(["CODER_INTENT", "REVIEW_REQUEST"]);
const MT_FIX_CYCLE_ESCALATION_THRESHOLD = 3;
function parseSingleField(text, label) {
  const re = new RegExp(`^\\s*-\\s*(?:\\*\\*)?${label}(?:\\*\\*)?\\s*:\\s*(.+)\\s*$`, "mi");
  const match = text.match(re);
  return match ? match[1].trim() : "";
}

function nullableValue(value) {
  const raw = String(value ?? "").trim();
  if (
    !raw
    || /^null$/i.test(raw)
    || /^none$/i.test(raw)
    || /^n\/a$/i.test(raw)
    || /^false$/i.test(raw)
    || /^<unassigned>$/i.test(raw)
  ) return null;
  return raw;
}

function parseBooleanLike(value) {
  const raw = String(value ?? "").trim();
  if (!raw) return false;
  return ["1", "true", "yes", "y"].includes(raw.toLowerCase());
}

function readOptionalText(absPath) {
  return fs.existsSync(absPath) ? fs.readFileSync(absPath, "utf8") : null;
}

function restoreOptionalText(absPath, text) {
  if (text === null) {
    if (fs.existsSync(absPath)) fs.unlinkSync(absPath);
    return;
  }
  fs.writeFileSync(absPath, text, "utf8");
}

function normalizeRole(value) {
  return String(value || "").trim().toUpperCase();
}

function normalizeSession(value) {
  const raw = String(value || "").trim();
  if (!raw || /^<unassigned>$/i.test(raw)) return null;
  return raw || null;
}

function normalizeReceiptKind(value) {
  return String(value || "").trim().toUpperCase();
}

function parseVerbBodyValue(value) {
  if (value && typeof value === "object" && !Array.isArray(value)) return value;
  const text = String(value || "").trim();
  if (!text) return null;
  try {
    const parsed = JSON.parse(text);
    return parsed && typeof parsed === "object" && !Array.isArray(parsed) ? parsed : null;
  } catch {
    return null;
  }
}

function absorbReceiptAppendArgs(args = {}) {
  const absorbed = runAbsorber(JSON.stringify(args || {}), {
    artifactKind: "receipt_args",
    wpId: args?.wpId,
  });
  if (absorbed.applied.length === 0) {
    return { args, applied: [] };
  }
  try {
    return {
      args: JSON.parse(absorbed.output),
      applied: absorbed.applied,
    };
  } catch {
    return { args, applied: [] };
  }
}

function enrichReceiptArgsWithHeuristicRisk(args = {}) {
  const microtaskContract = args?.microtaskContract;
  if (!microtaskContract || typeof microtaskContract !== "object" || Array.isArray(microtaskContract)) {
    return args;
  }
  const wpId = String(args?.wpId || "").trim();
  const scopeRef = String(microtaskContract.scope_ref || "").trim();
  if (!wpId || !scopeRef) return args;
  const resolution = resolveDeclaredWpMicrotaskByScopeRef(wpId, scopeRef);
  if (!resolution.match?.heuristicRisk) return args;
  return {
    ...args,
    microtaskContract: mergeHeuristicRiskContract(microtaskContract, resolution.match.heuristicRisk),
  };
}

function heuristicRiskContractForReceipt({ wpId = "", mtId = "", entry = {} } = {}) {
  if (isHeuristicRiskContract(entry?.microtask_contract)) return entry.microtask_contract;
  if (!wpId || !mtId) return null;
  const resolution = resolveDeclaredWpMicrotaskByScopeRef(wpId, mtId);
  if (!resolution.match?.heuristicRisk) return null;
  const merged = mergeHeuristicRiskContract({}, resolution.match.heuristicRisk);
  return isHeuristicRiskContract(merged) ? merged : null;
}

function parseChangedPaths(statusPorcelain) {
  return String(statusPorcelain || "")
    .split(/\r?\n/)
    .map((line) => line.trimEnd())
    .filter(Boolean)
    .map((line) => line.slice(3).split(" -> ").at(-1)?.trim() || "")
    .filter(Boolean);
}

function runInWorktree(worktreeAbs, command, args) {
  const result = execFileSync(command, args, {
    cwd: worktreeAbs,
    encoding: "utf8",
    stdio: ["ignore", "pipe", "pipe"],
  });
  return String(result || "").trim();
}

export function summarizeCommittedCoderHandoffDirtyState(statusPorcelain, scopeContract, {
  allowAmbientOutOfScope = false,
} = {}) {
  const changedPaths = parseChangedPaths(statusPorcelain);
  const governanceNoisePaths = [];
  const transientArtifactPaths = [];
  const ambientOutOfScopePaths = [];
  const blockingPaths = [];

  for (const changedPath of changedPaths) {
    const normalizedPath = normalizeRepoPath(changedPath) || changedPath;
    if (isTransientProofArtifactPath(normalizedPath)) {
      transientArtifactPaths.push(normalizedPath);
      continue;
    }
    if (isGovernanceOnlyPath(normalizedPath)) {
      governanceNoisePaths.push(normalizedPath);
      continue;
    }
    const classification = classifyWpChangedPath(changedPath, scopeContract);
    const classifiedPath = `${classification.path} (${classification.kind})`;
    if (classification.allowed) {
      blockingPaths.push(classifiedPath);
      continue;
    }
    if (allowAmbientOutOfScope) {
      ambientOutOfScopePaths.push(classifiedPath);
      continue;
    }
    blockingPaths.push(classifiedPath);
  }

  return {
    changedPaths,
    governanceNoisePaths,
    transientArtifactPaths,
    ambientOutOfScopePaths,
    blockingPaths,
    ok: blockingPaths.length === 0,
  };
}

function addedDirtyEntries(beforeEntries = [], afterEntries = []) {
  const beforeSet = new Set((beforeEntries || []).map((entry) => String(entry || "").trim()).filter(Boolean));
  return Array.from(new Set(
    (afterEntries || [])
      .map((entry) => String(entry || "").trim())
      .filter(Boolean)
      .filter((entry) => !beforeSet.has(entry)),
  ));
}

function formatCommittedCoderHandoffFailure(summary = {}) {
  const detailLines = [];
  if ((summary.blockingPaths || []).length > 0) {
    detailLines.push(
      `blocking_paths=${formatBoundedItemList(summary.blockingPaths, { noun: "entry" })}`,
    );
  }
  if ((summary.governanceNoisePaths || []).length > 0) {
    detailLines.push(
      `governance_noise=${formatBoundedItemList(summary.governanceNoisePaths, { noun: "path" })}`,
    );
  }
  if ((summary.transientArtifactPaths || []).length > 0) {
    detailLines.push(
      `transient_artifacts=${formatBoundedItemList(summary.transientArtifactPaths, { noun: "path" })}`,
    );
  }
  if ((summary.ambientOutOfScopePaths || []).length > 0) {
    detailLines.push(
      `ambient_out_of_scope=${formatBoundedItemList(summary.ambientOutOfScopePaths, { noun: "path" })}`,
    );
  }
  return detailLines;
}

function committedCoderHandoffGateApplies({ context, actorRole, receiptKind }) {
  return normalizeRole(context?.workflowLane) === "ORCHESTRATOR_MANAGED"
    && normalizeRole(actorRole) === "CODER"
    && normalizeReceiptKind(receiptKind) === "CODER_HANDOFF";
}

function buildReceiptValidationEntry({
  args = {},
  context,
  runtimeStatus = null,
  timestamp = null,
} = {}) {
  const { authorityKind, validatorRoleKind } = deriveAuthorityKinds({
    actorRole: args?.actorRole,
    actorSession: args?.actorSession,
    runtimeStatus,
  });
  const entry = {
    schema_version: "wp_receipt@1",
    timestamp_utc: String(timestamp || args?.timestamp || new Date().toISOString()),
    wp_id: String(args?.wpId || "").trim(),
    actor_role: String(args?.actorRole || "").trim().toUpperCase(),
    actor_session: String(args?.actorSession || "").trim(),
    actor_authority_kind: authorityKind,
    validator_role_kind: validatorRoleKind,
    receipt_kind: String(args?.receiptKind || "").trim().toUpperCase(),
    verb: args?.verb === undefined || args?.verb === null ? null : String(args.verb || "").trim().toUpperCase(),
    verb_body: args?.verb ? parseVerbBodyValue(args?.verbBody) : null,
    summary: String(args?.summary || "").trim(),
    branch: args?.branch === undefined ? context.branch : nullableValue(args?.branch),
    worktree_dir: args?.worktreeDir === undefined ? context.worktreeDir : nullableValue(args?.worktreeDir),
    state_before: nullableValue(args?.stateBefore),
    state_after: nullableValue(args?.stateAfter),
    target_role: nullableValue(args?.targetRole),
    target_session: nullableValue(args?.targetSession),
    correlation_id: nullableValue(args?.correlationId),
    requires_ack: Boolean(args?.requiresAck),
    ack_for: nullableValue(args?.ackFor),
    spec_anchor: nullableValue(args?.specAnchor),
    packet_row_ref: nullableValue(args?.packetRowRef),
    microtask_contract: args?.microtaskContract && typeof args?.microtaskContract === "object" ? args.microtaskContract : null,
    mechanical_result: args?.mechanicalResult && typeof args?.mechanicalResult === "object" ? args.mechanicalResult : null,
    workflow_invalidity_code: nullableValue(normalizeWorkflowInvalidityCode(args?.workflowInvalidityCode)),
    // [CX-109D] Capture resolved cwd on CODER_INTENT receipts for worktree confinement verification.
    resolved_cwd: normalizeRole(args?.actorRole) === "CODER"
      && normalizeReceiptKind(args?.receiptKind) === "CODER_INTENT"
      ? normalize(process.cwd())
      : null,
    refs: [context.packetPath, ...(Array.isArray(args?.refs) ? args.refs : []).filter(Boolean).map((value) => normalize(value))],
  };

  if (entry.verb && !entry.summary) {
    entry.summary = renderInterRoleVerbReceipt(entry) || `${entry.verb} receipt`;
  }
  if (entry.verb) {
    const verbValidation = validateInterRoleVerbBody(entry.verb, entry.verb_body);
    if (verbValidation.ok) entry.verb_body = verbValidation.body;
  }

  if (context.runtimeStatusFile && !entry.refs.includes(context.runtimeStatusFile)) entry.refs.push(context.runtimeStatusFile);
  if (context.threadFile && !entry.refs.includes(context.threadFile)) entry.refs.push(context.threadFile);
  if (!entry.refs.includes(context.receiptsFile)) entry.refs.push(context.receiptsFile);
  return entry;
}

function appendNotificationTarget(targets, seenTargets, target) {
  const targetRole = normalizeRole(target?.targetRole);
  const targetSession = normalizeSession(target?.targetSession);
  const actorRole = normalizeRole(target?.actorRole);
  if (!targetRole || targetRole === actorRole) return;
  const dedupeKey = `${targetRole}::${targetSession || ""}`;
  if (seenTargets.has(dedupeKey)) {
    const existing = targets.find((entry) => `${entry.targetRole}::${entry.targetSession || ""}` === dedupeKey);
    if (existing && target?.autoRelay === true) {
      existing.autoRelay = true;
    }
    return;
  }
  seenTargets.add(dedupeKey);
  targets.push({
    targetRole,
    targetSession,
    sourceKind: String(target?.sourceKind || "").trim().toUpperCase(),
    summary: String(target?.summary || "").trim(),
    autoRelay: target?.autoRelay === true,
  });
}

function shouldAppendOrchestratorGovernanceCheckpoint({ workflowLane, entry }) {
  if (normalizeRole(workflowLane) !== "ORCHESTRATOR_MANAGED") return false;
  if (!VALIDATOR_ASSESSMENT_ROLE_VALUES.has(normalizeRole(entry?.actor_role))) return false;
  return GOVERNANCE_CHECKPOINT_RECEIPT_KIND_VALUES.has(normalizeReceiptKind(entry?.receipt_kind));
}

function buildOrchestratorGovernanceCheckpointSummary({ entry, autoRoute }) {
  const assessment = deriveLatestValidatorAssessment([entry]);
  const nextActor = normalizeRole(autoRoute?.nextExpectedActor) || "UNCHANGED";
  const result = assessment?.verdict || "ASSESSED";
  const why = String(assessment?.reason || entry?.summary || "").trim();
  return [
    `GOVERNANCE_CHECKPOINT: ${normalizeRole(entry?.actor_role)} recorded ${normalizeReceiptKind(entry?.receipt_kind)} result=${result};`,
    why ? `why=${why}` : null,
    `verify governance truth and ACP steering.`,
    `projected_next_actor=${nextActor}.`,
  ].filter(Boolean).join(" ");
}

export function deriveReviewNotificationTargets({ workflowLane = "", entry, autoRoute } = {}) {
  const targets = [];
  const seenTargets = new Set();
  const actorRole = normalizeRole(entry?.actor_role);
  const explicitTargetRole = normalizeRole(entry?.target_role);
  const explicitTargetSession = normalizeSession(entry?.target_session);

  if (explicitTargetRole) {
    appendNotificationTarget(targets, seenTargets, {
      actorRole,
      targetRole: explicitTargetRole,
      targetSession: explicitTargetSession,
      sourceKind: entry?.receipt_kind,
      summary: `${entry.receipt_kind}: ${entry.summary}`,
    });
  }

  if (autoRoute?.notification?.targetRole) {
    appendNotificationTarget(targets, seenTargets, {
      actorRole,
      targetRole: autoRoute.notification.targetRole,
      targetSession: autoRoute.notification.targetSession,
      sourceKind: "AUTO_ROUTE",
      summary: autoRoute.notification.summary,
    });
  }

  if (Array.isArray(autoRoute?.secondaryNotifications)) {
    for (const notification of autoRoute.secondaryNotifications) {
      appendNotificationTarget(targets, seenTargets, {
        actorRole,
        targetRole: notification?.targetRole,
        targetSession: notification?.targetSession,
        sourceKind: String(notification?.sourceKind || "AUTO_ROUTE").trim().toUpperCase(),
        summary: notification?.summary,
        autoRelay: notification?.autoRelay === true,
      });
    }
  }

  if (shouldAppendOrchestratorGovernanceCheckpoint({ workflowLane, entry })) {
    appendNotificationTarget(targets, seenTargets, {
      actorRole,
      targetRole: "ORCHESTRATOR",
      targetSession: null,
      sourceKind: "GOVERNANCE_CHECKPOINT",
      summary: buildOrchestratorGovernanceCheckpointSummary({ entry, autoRoute }),
    });
  }

  return targets;
}

function updateOpenReviewItems(runtimeStatus, entry) {
  if (!runtimeStatus || typeof runtimeStatus !== "object") return;
  const currentItems = Array.isArray(runtimeStatus.open_review_items) ? runtimeStatus.open_review_items : [];
  const correlationId = String(entry.correlation_id || "").trim();
  if (!correlationId) {
    runtimeStatus.open_review_items = currentItems;
    return;
  }

  const withoutCorrelation = currentItems.filter((item) => String(item?.correlation_id || "").trim() !== correlationId);
  if (REVIEW_OPEN_RECEIPT_KIND_VALUES.includes(entry.receipt_kind)) {
    withoutCorrelation.push({
      correlation_id: correlationId,
      receipt_kind: entry.receipt_kind,
      summary: entry.summary,
      opened_by_role: entry.actor_role,
      opened_by_session: entry.actor_session,
      target_role: entry.target_role,
      target_session: entry.target_session ?? null,
      spec_anchor: entry.spec_anchor ?? null,
      packet_row_ref: entry.packet_row_ref ?? null,
      microtask_contract: entry.microtask_contract ?? null,
      requires_ack: entry.requires_ack,
      opened_at: entry.timestamp_utc,
      updated_at: entry.timestamp_utc,
    });
  } else if (REVIEW_RESOLUTION_RECEIPT_KIND_VALUES.includes(entry.receipt_kind)) {
    // Resolution receipts close the matching open review item.
  } else {
    runtimeStatus.open_review_items = currentItems;
    return;
  }

  runtimeStatus.open_review_items = withoutCorrelation.sort((left, right) =>
    String(left.opened_at || "").localeCompare(String(right.opened_at || ""))
  );
}

function loadPacketContext(wpId) {
  const packetPath = workPacketPath(wpId);
  const packetAbsPath = repoPathAbs(packetPath);
  if (!fs.existsSync(packetAbsPath)) {
    throw new Error(`Official packet not found: ${normalize(packetPath)}`);
  }
  const packetText = fs.readFileSync(packetAbsPath, "utf8");
  const receiptsFile = parseSingleField(packetText, "WP_RECEIPTS_FILE");
  const runtimeStatusFile = parseSingleField(packetText, "WP_RUNTIME_STATUS_FILE");
  const threadFile = parseSingleField(packetText, "WP_THREAD_FILE");
  const branch = parseSingleField(packetText, "LOCAL_BRANCH") || null;
  const worktreeDir = parseSingleField(packetText, "LOCAL_WORKTREE_DIR") || null;

  if (!receiptsFile) {
    throw new Error(`${normalize(packetPath)} does not declare WP_RECEIPTS_FILE`);
  }
  const receiptsAbsPath = repoPathAbs(receiptsFile);
  if (!fs.existsSync(receiptsAbsPath)) {
    throw new Error(`Receipts ledger missing on disk: ${normalize(receiptsFile)}`);
  }

  return {
    packetPath: normalize(packetPath),
    packetAbsPath: normalize(packetAbsPath),
    packetText,
    receiptsFile: normalize(receiptsFile),
    receiptsAbsPath: normalize(receiptsAbsPath),
    runtimeStatusFile: normalize(runtimeStatusFile),
    runtimeStatusAbsPath: runtimeStatusFile ? normalize(repoPathAbs(runtimeStatusFile)) : "",
    threadFile: normalize(threadFile),
    threadAbsPath: threadFile ? normalize(repoPathAbs(threadFile)) : "",
    branch: branch ? normalize(branch) : null,
    worktreeDir: worktreeDir ? normalize(worktreeDir) : null,
    workflowLane: parseSingleField(packetText, "WORKFLOW_LANE") || "",
    packetFormatVersion: parseSingleField(packetText, "PACKET_FORMAT_VERSION") || "",
    communicationContract: parseSingleField(packetText, "COMMUNICATION_CONTRACT") || "",
    communicationHealthGate: parseSingleField(packetText, "COMMUNICATION_HEALTH_GATE") || "",
  };
}

export function buildGovernedPhaseCheckInvocation({
  phase = "",
  wpId = "",
  role = "",
  session = "",
  args = [],
} = {}) {
  const invocationArgs = [
    PHASE_CHECK_SCRIPT_ABS_PATH,
    String(phase || "").trim().toUpperCase(),
    String(wpId || "").trim(),
  ];
  const normalizedRole = String(role || "").trim().toUpperCase();
  const normalizedSession = String(session || "").trim();
  if (normalizedRole) invocationArgs.push(normalizedRole);
  if (normalizedSession) invocationArgs.push(normalizedSession);
  if (Array.isArray(args)) {
    for (const value of args) {
      const normalized = String(value || "").trim();
      if (normalized) invocationArgs.push(normalized);
    }
  }
  return {
    command: process.execPath,
    args: invocationArgs,
  };
}

function assertCommittedCoderHandoffPreflight({ wpId, context }) {
  const prepareEntry = lastGateLog(loadOrchestratorGateLogs(), wpId, "PREPARE");
  if (!prepareEntry) {
    throw new Error("Governed CODER_HANDOFF rejected: PREPARE authority is missing for this WP.");
  }

  const syncState = preparedWorktreeSyncState(wpId, prepareEntry, REPO_ROOT);
  if (!syncState.ok) {
    throw new Error(
      `Governed CODER_HANDOFF rejected: PREPARE worktree is not synchronized with current packet truth (${formatBoundedItemList(syncState.issues, { noun: "issue" })}).`
    );
  }

  const worktreeAbs = resolvePrepareWorktreeAbs(prepareEntry, REPO_ROOT);
  if (!worktreeAbs || !fs.existsSync(worktreeAbs)) {
    throw new Error("Governed CODER_HANDOFF rejected: PREPARE worktree is unavailable.");
  }

  const scopeContract = deriveWpScopeContract({ wpId, packetContent: context.packetText });
  const preferredRange = resolveCommittedCoderHandoffRange(context.packetText, wpId);
  const allowAmbientOutOfScope = Boolean(preferredRange);

  const initialStatus = execFileSync("git", ["status", "--porcelain=v1"], {
    cwd: worktreeAbs,
    encoding: "utf8",
    stdio: ["ignore", "pipe", "pipe"],
  });
  const initialDirty = summarizeCommittedCoderHandoffDirtyState(initialStatus, scopeContract, {
    allowAmbientOutOfScope,
  });
  if (!initialDirty.ok) {
    throw new Error(
      `Governed CODER_HANDOFF rejected: PREPARE worktree is not committed/reviewable yet (${formatCommittedCoderHandoffFailure(initialDirty).join("; ")}).`
    );
  }

  try {
    const invocation = buildGovernedPhaseCheckInvocation({
      phase: "STARTUP",
      wpId,
      role: "CODER",
      args: ["--committed-handoff-preflight"],
    });
    runInWorktree(worktreeAbs, invocation.command, invocation.args);
  } catch (error) {
    const output = String(error?.stdout || error?.stderr || error?.message || "").trim();
    throw new Error(
      "Governed CODER_HANDOFF rejected: "
      + `${buildPhaseCheckCommand({ phase: "STARTUP", wpId, role: "CODER", args: ["--committed-handoff-preflight"] })}`
      + ` failed${output ? ` (${output})` : ""}.`
    );
  }

  try {
    runInWorktree(worktreeAbs, "just", ["cargo-clean"]);
  } catch (error) {
    const output = String(error?.stdout || error?.stderr || error?.message || "").trim();
    throw new Error(`Governed CODER_HANDOFF rejected: just cargo-clean failed${output ? ` (${output})` : ""}.`);
  }

  const postWorkArgs = preferredRange
    ? [POST_WORK_CHECK_SCRIPT_PATH, wpId, "--range", `${preferredRange.baseRev}..${preferredRange.headRev}`]
    : [POST_WORK_CHECK_SCRIPT_PATH, wpId];
  try {
    runInWorktree(worktreeAbs, process.execPath, postWorkArgs);
  } catch (error) {
    const output = String(error?.stdout || error?.stderr || error?.message || "").trim();
    throw new Error(`Governed CODER_HANDOFF rejected: ${buildPostWorkCommand(wpId, context.packetText)} failed${output ? ` (${output})` : ""}.`);
  }

  const finalStatus = execFileSync("git", ["status", "--porcelain=v1"], {
    cwd: worktreeAbs,
    encoding: "utf8",
    stdio: ["ignore", "pipe", "pipe"],
  });
  const finalDirty = summarizeCommittedCoderHandoffDirtyState(finalStatus, scopeContract, {
    allowAmbientOutOfScope,
  });
  if (!finalDirty.ok) {
    throw new Error(
      `Governed CODER_HANDOFF rejected: PREPARE worktree gained non-junction dirty state during preflight (${formatCommittedCoderHandoffFailure(finalDirty).join("; ")}).`
    );
  }
  if (allowAmbientOutOfScope) {
    const newAmbientOutOfScope = addedDirtyEntries(initialDirty.ambientOutOfScopePaths, finalDirty.ambientOutOfScopePaths);
    if (newAmbientOutOfScope.length > 0) {
      throw new Error(
        "Governed CODER_HANDOFF rejected: PREPARE worktree gained new out-of-scope dirty state during preflight "
        + `(${formatBoundedItemList(newAmbientOutOfScope, { noun: "path" })}).`
      );
    }
  }
}

function assertCoderHandoffRoutePreflight({ wpId, context, runtimeStatus }) {
  if (!runtimeStatus || normalizeRole(context?.workflowLane) !== "ORCHESTRATOR_MANAGED") return;
  const pendingOverlapReviews = (Array.isArray(runtimeStatus?.open_review_items) ? runtimeStatus.open_review_items : [])
    .filter((item) => isOverlapMicrotaskReviewItem(item));
  if (pendingOverlapReviews.length > 0) {
    throw new Error("Governed CODER_HANDOFF rejected: pending overlap microtask reviews must be resolved before full handoff.");
  }
  const receipts = parseJsonlFile(context.receiptsFile);
  const latestReceipt = receipts.at(-1) || null;
  const evaluation = evaluateWpCommunicationHealth({
    wpId,
    stage: "STATUS",
    packetPath: context.packetPath,
    packetContent: context.packetText,
    workflowLane: context.workflowLane || runtimeStatus.workflow_lane || "",
    packetFormatVersion: context.packetFormatVersion || "",
    communicationContract: context.communicationContract || "",
    communicationHealthGate: context.communicationHealthGate || "",
    receipts,
    runtimeStatus,
  });
  const autoRoute = deriveWpCommunicationAutoRoute({
    evaluation,
    runtimeStatus,
    latestReceipt,
  });
  const nextActor = normalizeRole(autoRoute?.nextExpectedActor);
  const waitingOn = String(autoRoute?.waitingOn || "").trim().toUpperCase();
  if (nextActor === "CODER" && ["CODER_HANDOFF", "CODER_REPAIR_HANDOFF"].includes(waitingOn)) return;

  let reason = `lane is not currently waiting on a coder handoff (${nextActor || "NONE"} / ${waitingOn || "UNKNOWN"}).`;
  if (String(evaluation?.state || "").trim().toUpperCase() === "COMM_WAITING_FOR_INTENT_CHECKPOINT") {
    reason = "WP validator checkpoint review of CODER_INTENT is still required before full handoff.";
  } else if (String(evaluation?.state || "").trim().toUpperCase() === "COMM_BLOCKED_OPEN_ITEMS") {
    reason = "open review items still block direct-review progression; resolve them before full handoff.";
  }
  throw new Error(`Governed CODER_HANDOFF rejected: ${reason}`);
}

function overlapReviewPreflightApplies({ context, actorRole, receiptKind, targetRole, microtaskContract }) {
  return normalizeRole(context?.workflowLane) === "ORCHESTRATOR_MANAGED"
    && normalizeRole(actorRole) === "CODER"
    && normalizeReceiptKind(receiptKind) === "REVIEW_REQUEST"
    && normalizeRole(targetRole) === "WP_VALIDATOR"
    && String(microtaskContract?.review_mode || "").trim().toUpperCase() === "OVERLAP";
}

function assertOverlapReviewBackpressurePreflight({ runtimeStatus }) {
  const overlapQueue = (Array.isArray(runtimeStatus?.open_review_items) ? runtimeStatus.open_review_items : [])
    .filter((item) => isOverlapMicrotaskReviewItem(item));
  if (overlapQueue.length >= MAX_OVERLAP_MICROTASK_REVIEW_ITEMS) {
    throw new Error(
      `Governed REVIEW_REQUEST rejected: overlap microtask review backlog already reached ${MAX_OVERLAP_MICROTASK_REVIEW_ITEMS}; wait for WP validator to drain the queue before opening another parallel review item.`,
    );
  }
}

// Inbox-clear gate applies when a coder emits forward-progress receipts
// (REVIEW_REQUEST or CODER_HANDOFF) — these should be blocked if the coder
// has unresolved steer/rejection debt from the validator.
function inboxClearGateApplies(args) {
  const role = String(args?.actorRole || "").trim().toUpperCase();
  const kind = String(args?.receiptKind || "").trim().toUpperCase();
  if (role !== "CODER") return false;
  return kind === "REVIEW_REQUEST" || kind === "CODER_HANDOFF";
}

function assertReviewResolutionCorrelationPreflight({
  context,
  runtimeStatus,
  actorRole,
  actorSession,
  receiptKind,
  targetRole,
  targetSession,
  correlationId,
  ackFor,
  microtaskContract,
}) {
  const normalizedReceiptKind = normalizeReceiptKind(receiptKind);
  if (!REVIEW_RESOLUTION_RECEIPT_KIND_VALUES.includes(normalizedReceiptKind)) return;

  const normalizedWaitingOn = String(runtimeStatus?.waiting_on || "").trim().toUpperCase();
  const normalizedPhaseGate = String(microtaskContract?.phase_gate || "").trim().toUpperCase();
  const bootstrapIntentReply = normalizedReceiptKind === "CODER_INTENT"
    && (normalizedWaitingOn === "CODER_INTENT" || normalizedPhaseGate === "BOOTSTRAP");
  if (normalizedReceiptKind === "CODER_INTENT" && !bootstrapIntentReply) return;

  const normalizedCorrelationId = String(correlationId || "").trim();
  const normalizedAckFor = String(ackFor || "").trim();
  if (!normalizedCorrelationId || !normalizedAckFor) return;

  const normalizedActorRole = normalizeRole(actorRole);
  const normalizedTargetRole = normalizeRole(targetRole);
  const normalizedActorSession = normalizeSession(actorSession);
  const normalizedTargetSession = normalizeSession(targetSession);
  const receipts = parseJsonlFile(context.receiptsFile);
  const matchingOpenReceipt = [...receipts].reverse().find((entry) => {
    if (!REVIEW_OPEN_RECEIPT_KIND_VALUES.includes(normalizeReceiptKind(entry?.receipt_kind))) return false;
    if (String(entry?.correlation_id || "").trim() !== normalizedCorrelationId) return false;
    if (normalizeRole(entry?.actor_role) !== normalizedTargetRole) return false;
    if (normalizeRole(entry?.target_role) !== normalizedActorRole) return false;

    const openActorRole = normalizeRole(entry?.actor_role);
    const openTargetRole = normalizeRole(entry?.target_role);
    const requiresSessionMatch = ACTIVE_AUTO_RELAY_ROLE_VALUES.has(openActorRole)
      && ACTIVE_AUTO_RELAY_ROLE_VALUES.has(openTargetRole);
    if (!requiresSessionMatch) return true;

    const openActorSession = normalizeSession(entry?.actor_session);
    const openTargetSession = normalizeSession(entry?.target_session);
    if (!openActorSession || !normalizedActorSession || !normalizedTargetSession) return false;
    if (openActorSession !== normalizedTargetSession) return false;
    if (openTargetSession && openTargetSession !== normalizedActorSession) return false;
    return true;
  });

  if (matchingOpenReceipt) return;

  const validatorIntentCheckpointClearance = normalizedReceiptKind === "VALIDATOR_RESPONSE"
    && VALIDATOR_ASSESSMENT_ROLE_VALUES.has(normalizedActorRole)
    && normalizedTargetRole === "CODER"
    && normalizedWaitingOn === "WP_VALIDATOR_INTENT_CHECKPOINT";
  if (validatorIntentCheckpointClearance) {
    const matchingCheckpointDriver = [...receipts].reverse().find((entry) => {
      const priorReceiptKind = normalizeReceiptKind(entry?.receipt_kind);
      if (!REVIEW_RESOLUTION_RECEIPT_KIND_VALUES.includes(priorReceiptKind)) return false;
      if (priorReceiptKind === "VALIDATOR_RESPONSE") return false;
      if (String(entry?.correlation_id || "").trim() !== normalizedCorrelationId) return false;
      if (normalizeRole(entry?.actor_role) !== normalizedTargetRole) return false;
      if (normalizeRole(entry?.target_role) !== normalizedActorRole) return false;

      const priorActorRole = normalizeRole(entry?.actor_role);
      const priorTargetRole = normalizeRole(entry?.target_role);
      const requiresSessionMatch = ACTIVE_AUTO_RELAY_ROLE_VALUES.has(priorActorRole)
        && ACTIVE_AUTO_RELAY_ROLE_VALUES.has(priorTargetRole);
      if (!requiresSessionMatch) return true;

      const priorActorSession = normalizeSession(entry?.actor_session);
      const priorTargetSession = normalizeSession(entry?.target_session);
      if (!priorActorSession || !normalizedActorSession || !normalizedTargetSession) return false;
      if (priorActorSession !== normalizedTargetSession) return false;
      if (priorTargetSession && priorTargetSession !== normalizedActorSession) return false;
      return true;
    });
    if (matchingCheckpointDriver) return;
  }

  throw new Error(
    `Governed ${normalizedReceiptKind} rejected: correlation_id/ack_for must reference an existing open review receipt from ${normalizedTargetRole || "<target>"} to ${normalizedActorRole || "<actor>"}.`,
  );
}

function overlapReviewResolutionPreflightApplies({ context, actorRole, receiptKind, microtaskContract }) {
  return normalizeRole(context?.workflowLane) === "ORCHESTRATOR_MANAGED"
    && VALIDATOR_ASSESSMENT_ROLE_VALUES.has(normalizeRole(actorRole))
    && REVIEW_RESOLUTION_RECEIPT_KIND_VALUES.includes(normalizeReceiptKind(receiptKind))
    && String(microtaskContract?.review_mode || "").trim().toUpperCase() === "OVERLAP";
}

function microtaskScopeBudgetPreflightApplies({ context, actorRole, receiptKind }) {
  return normalizeRole(context?.workflowLane) === "ORCHESTRATOR_MANAGED"
    && normalizeRole(actorRole) === "CODER"
    && MICROTASK_SCOPE_BUDGET_RECEIPT_KIND_VALUES.has(normalizeReceiptKind(receiptKind));
}

function deriveDeclaredMicrotaskContext({ wpId, context, runtimeStatus, scopeRef }) {
  const declaredMicrotasks = listDeclaredWpMicrotasks(wpId);
  if (declaredMicrotasks.length === 0) {
    return {
      declaredMicrotasks,
      resolution: { match: null, ambiguousMatches: [] },
      plan: { declared_count: 0, active_microtask: null, previous_microtask: null, suggested_next_microtask: null, items: [] },
    };
  }

  const receipts = parseJsonlFile(context.receiptsFile);
  return {
    declaredMicrotasks,
    resolution: resolveDeclaredWpMicrotaskByScopeRef(wpId, scopeRef, declaredMicrotasks),
    plan: deriveWpMicrotaskPlan({
      wpId,
      receipts,
      runtimeStatus,
      microtasks: declaredMicrotasks,
    }),
  };
}

function assertDeclaredMicrotaskScopeBudgetPreflight({
  wpId,
  context,
  runtimeStatus,
  actorRole,
  receiptKind,
  targetRole,
  microtaskContract,
}) {
  const scopeRef = String(microtaskContract?.scope_ref || "").trim();
  const {
    declaredMicrotasks,
    resolution,
    plan,
  } = deriveDeclaredMicrotaskContext({
    wpId,
    context,
    runtimeStatus,
    scopeRef,
  });
  if (declaredMicrotasks.length === 0) return;

  // For REVIEW_REQUEST and REVIEW_RESPONSE, auto-infer microtask context from summary
  // when microtaskContract is missing. This keeps strict enforcement for CODER_INTENT
  // (which writes code) but relaxes it for review communication (which is coordination).
  const isReviewExchange = /^REVIEW_REQUEST$|^REVIEW_RESPONSE$/i.test(receiptKind || "");
  if (!microtaskContract || typeof microtaskContract !== "object" || Array.isArray(microtaskContract)) {
    if (isReviewExchange) {
      // Auto-infer: skip strict microtask contract enforcement for review exchanges.
      // The summary typically contains "MT-001 complete" or similar; the receipt will
      // still be persisted and the auto-relay will fire.
      return;
    }
    throw new Error(
      `Governed ${receiptKind} rejected: declared microtask contract is required when MT packets exist for ${wpId}.`,
    );
  }

  if (!scopeRef) {
    if (isReviewExchange) return; // Same relaxation for review exchanges.
    throw new Error(
      `Governed ${receiptKind} rejected: microtask_contract.scope_ref must point to a declared MT (for example MT-001 or CLAUSE_CLOSURE_MATRIX/CX-...).`,
    );
  }

  if (resolution.ambiguousMatches.length > 0) {
    throw new Error(
      `Governed ${receiptKind} rejected: microtask_contract.scope_ref=${scopeRef} matches multiple MT packets (${formatBoundedItemList(resolution.ambiguousMatches.map((item) => item.mtId), { noun: "MT" })}).`,
    );
  }
  if (!resolution.match) {
    throw new Error(
      `Governed ${receiptKind} rejected: microtask_contract.scope_ref=${scopeRef} does not resolve to a declared MT in ${wpId}.`,
    );
  }

  const budget = summarizeMicrotaskFileTargetBudget(microtaskContract.file_targets, resolution.match);
  if (budget.normalizedTargets.length === 0) {
    throw new Error(
      `Governed ${receiptKind} rejected: microtask_contract.file_targets must name concrete files inside ${resolution.match.mtId}.`,
    );
  }
  if (!budget.ok) {
    throw new Error(
      `Governed ${receiptKind} rejected: microtask_contract.file_targets escape ${resolution.match.mtId} CODE_SURFACES (${formatBoundedItemList(budget.outOfBudgetTargets, { noun: "path" })}).`,
    );
  }

  const proofCommands = Array.isArray(microtaskContract.proof_commands)
    ? microtaskContract.proof_commands.map((entry) => String(entry || "").trim()).filter(Boolean)
    : [];
  if (proofCommands.length === 0) {
    throw new Error(
      `Governed ${receiptKind} rejected: microtask_contract.proof_commands must declare proof for ${resolution.match.mtId}.`,
    );
  }

  const normalizedActorRole = normalizeRole(actorRole);
  const normalizedReceiptKind = normalizeReceiptKind(receiptKind);
  const activeMicrotask = plan.active_microtask;
  const previousMicrotask = plan.previous_microtask;
  if (normalizedActorRole === "CODER" && normalizedReceiptKind === "CODER_INTENT") {
    const expectedMtId = activeMicrotask?.mt_id || declaredMicrotasks[0]?.mtId || null;
    if (expectedMtId && resolution.match.mtId !== expectedMtId) {
      throw new Error(
        `Governed ${receiptKind} rejected: microtask_contract.scope_ref=${scopeRef} is out of sequence; active execution budget is ${expectedMtId}, not ${resolution.match.mtId}.`,
      );
    }
  }

  const overlapMode = String(microtaskContract.review_mode || "").trim().toUpperCase();
  if (
    normalizedActorRole === "CODER"
    && normalizedReceiptKind === "REVIEW_REQUEST"
    && normalizeRole(targetRole) === "WP_VALIDATOR"
    && overlapMode === "OVERLAP"
  ) {
    const expectedMtId = activeMicrotask?.mt_id || declaredMicrotasks[0]?.mtId || null;
    if (!expectedMtId) {
      throw new Error(
        `Governed ${receiptKind} rejected: no active microtask execution budget is available for overlap review in ${wpId}.`,
      );
    }
    if (resolution.match.mtId !== expectedMtId) {
      throw new Error(
        `Governed ${receiptKind} rejected: overlap review must bind to the current active microtask ${expectedMtId}, not ${resolution.match.mtId}.`,
      );
    }
  }

  if (
    VALIDATOR_ASSESSMENT_ROLE_VALUES.has(normalizedActorRole)
    && REVIEW_RESOLUTION_RECEIPT_KIND_VALUES.includes(normalizedReceiptKind)
    && overlapMode === "OVERLAP"
  ) {
    const expectedMtId = previousMicrotask?.mt_id || null;
    if (!expectedMtId) {
      throw new Error(
        `Governed ${receiptKind} rejected: overlap review resolution requires an immediately previous microtask under review, but none is projected for ${wpId}.`,
      );
    }
    if (resolution.match.mtId !== expectedMtId) {
      throw new Error(
        `Governed ${receiptKind} rejected: overlap review resolution must bind to previous microtask ${expectedMtId}, not ${resolution.match.mtId}.`,
      );
    }
  }
}

function assertNoDuplicateDecisiveValidatorAssessment({ context, entry }) {
  const receipts = parseJsonlFile(context.receiptsFile);
  if (!isDuplicateDecisiveValidatorAssessment(receipts, entry)) return;
  const verdict = String(entry?.summary || "").trim()
    ? deriveLatestValidatorAssessment([...receipts, entry])?.verdict || "ASSESSMENT"
    : "ASSESSMENT";
  throw new Error(
    `Duplicate decisive validator outcome suppressed: ${entry.receipt_kind} would repeat an existing ${verdict} assessment for the latest review round on correlation ${entry.correlation_id}.`
  );
}

export function validateWpReceiptAppendPreconditions(args = {}, options = {}) {
  const wpId = String(args?.wpId || "").trim();
  if (!wpId || !/^WP-/.test(wpId)) {
    throw new Error("WP_ID is required");
  }

  const context = options.context || loadPacketContext(wpId);
  const runtimeStatus = context.runtimeStatusAbsPath && fs.existsSync(context.runtimeStatusAbsPath)
    ? parseJsonFile(context.runtimeStatusFile)
    : null;
  if (committedCoderHandoffGateApplies({
    context,
    actorRole: args?.actorRole,
    receiptKind: args?.receiptKind,
  })) {
    if (!options.skipCommittedCoderHandoffGate) {
      assertCommittedCoderHandoffPreflight({ wpId, context });
    }
    assertCoderHandoffRoutePreflight({ wpId, context, runtimeStatus });
  }
  if (overlapReviewPreflightApplies({
    context,
    actorRole: args?.actorRole,
    receiptKind: args?.receiptKind,
    targetRole: args?.targetRole,
    microtaskContract: args?.microtaskContract,
  })) {
    assertOverlapReviewBackpressurePreflight({ runtimeStatus });
  }

  // Inbox-clear gate: block new REVIEW_REQUEST emissions when the actor has
  // unresolved remediation debt (steer/rejection on a prior MT).
  if (runtimeStatus && inboxClearGateApplies(args)) {
    const inbox = buildRoleInbox(
      String(args?.actorRole || "").trim(),
      runtimeStatus,
    );
    const steers = inbox.items.filter((item) => item.is_steer);
    if (steers.length > 0) {
      throw new Error(
        `Inbox-clear gate: ${steers.length} unresolved steer(s) must be remediated before emitting `
        + `${normalize(args?.receiptKind)}. Pending: ${steers.map((s) => s.mt || s.kind).join(", ")}. `
        + `Address remediation debt first, then retry.`
      );
    }
  }

  if (microtaskScopeBudgetPreflightApplies({
    context,
    actorRole: args?.actorRole,
    receiptKind: args?.receiptKind,
  })) {
    assertDeclaredMicrotaskScopeBudgetPreflight({
      wpId,
      context,
      runtimeStatus,
      actorRole: args?.actorRole,
      receiptKind: normalizeReceiptKind(args?.receiptKind),
      targetRole: args?.targetRole,
      microtaskContract: args?.microtaskContract,
    });
  }
  assertReviewResolutionCorrelationPreflight({
    context,
    runtimeStatus,
    actorRole: args?.actorRole,
    actorSession: args?.actorSession,
    receiptKind: args?.receiptKind,
    targetRole: args?.targetRole,
    targetSession: args?.targetSession,
    correlationId: args?.correlationId,
    ackFor: args?.ackFor,
    microtaskContract: args?.microtaskContract,
  });
  if (overlapReviewResolutionPreflightApplies({
    context,
    actorRole: args?.actorRole,
    receiptKind: args?.receiptKind,
    microtaskContract: args?.microtaskContract,
  })) {
    assertDeclaredMicrotaskScopeBudgetPreflight({
      wpId,
      context,
      runtimeStatus,
      actorRole: args?.actorRole,
      receiptKind: normalizeReceiptKind(args?.receiptKind),
      targetRole: args?.targetRole,
      microtaskContract: args?.microtaskContract,
    });
  }

  const entry = buildReceiptValidationEntry({
    args,
    context,
    runtimeStatus,
  });
  const errors = validateReceipt(entry);
  if (errors.length > 0) {
    throw new Error(`Receipt validation failed: ${errors.join("; ")}`);
  }

  assertNoDuplicateDecisiveValidatorAssessment({ context, entry });

  return { context, runtimeStatus, entry };
}

function appendReviewNotifications({ wpId, workflowLane, entry, autoRoute }) {
  const targets = deriveReviewNotificationTargets({ workflowLane, entry, autoRoute });
  for (const target of targets) {
    appendWpNotification({
      wpId,
      sourceKind: target.sourceKind,
      sourceRole: entry.actor_role,
      sourceSession: entry.actor_session,
      targetRole: target.targetRole,
      targetSession: target.targetSession,
      correlationId: entry.correlation_id ?? null,
      summary: target.summary,
      timestamp: entry.timestamp_utc,
    }, target.autoRelay === true
      ? { assumeTransactionLock: true, autoRelay: true }
      : REVIEW_NOTIFICATION_APPEND_OPTIONS);
  }
}

function syncReviewGovernanceTruth({
  wpId,
  context,
  entry,
  evaluation,
  runtimeStatus,
}) {
  if (!evaluation?.applicable) {
    return { ...(runtimeStatus || {}) };
  }

  const originalPacketText = fs.readFileSync(context.packetAbsPath, "utf8");
  const originalTaskBoardText = readOptionalText(TASK_BOARD_ABS_PATH);
  const originalBuildOrderText = readOptionalText(BUILD_ORDER_ABS_PATH);
  const originalRuntimeText = context.runtimeStatusAbsPath && fs.existsSync(context.runtimeStatusAbsPath)
    ? fs.readFileSync(context.runtimeStatusAbsPath, "utf8")
    : null;
  const receipts = [...parseJsonlFile(context.receiptsFile), entry];
  const reconciliation = reconcileWpCommunicationTruth({
    wpId,
    packetPath: context.packetPath,
    packetText: originalPacketText,
    runtimeStatus,
    receipts,
  });

  try {
    if (reconciliation.nextPacketText !== originalPacketText) {
      fs.writeFileSync(context.packetAbsPath, reconciliation.nextPacketText, "utf8");
    }
    if (context.runtimeStatusAbsPath) {
      fs.writeFileSync(
        context.runtimeStatusAbsPath,
        `${JSON.stringify(reconciliation.nextRuntimeStatus, null, 2)}\n`,
        "utf8",
      );
    }
    syncProjectedTaskBoardTruth(wpId, reconciliation.packetProjection);
    return reconciliation.nextRuntimeStatus;
  } catch (error) {
    restoreOptionalText(context.packetAbsPath, originalPacketText);
    restoreOptionalText(TASK_BOARD_ABS_PATH, originalTaskBoardText);
    restoreOptionalText(BUILD_ORDER_ABS_PATH, originalBuildOrderText);
    if (context.runtimeStatusAbsPath) {
      restoreOptionalText(context.runtimeStatusAbsPath, originalRuntimeText);
    }
    const stderr = String(error?.stderr || "").trim();
    const stdout = String(error?.stdout || "").trim();
    throw new Error(
      `Review governance sync failed: ${stderr || stdout || (error instanceof Error ? error.message : String(error))}`
    );
  }
}

function attemptOrchestratorAutoRelay({ wpId, context, entry, autoRoute }) {
  if (String(context?.workflowLane || "").trim().toUpperCase() !== "ORCHESTRATOR_MANAGED") {
    return { status: "NOT_APPLICABLE", reason: "NON_ORCHESTRATOR_MANAGED" };
  }
  if (!autoRoute?.applicable) {
    return { status: "NOT_APPLICABLE", reason: "NO_AUTO_ROUTE" };
  }
  const nextActor = normalizeRole(autoRoute.nextExpectedActor);
  if (!ACTIVE_AUTO_RELAY_ROLE_VALUES.has(nextActor)) {
    return { status: "NOT_APPLICABLE", reason: "NO_GOVERNED_NEXT_ACTOR" };
  }
  if (nextActor === normalizeRole(entry?.actor_role)) {
    return { status: "SKIPPED", reason: "NEXT_ACTOR_IS_CURRENT_ACTOR", next_actor: nextActor };
  }

  try {
    const output = execFileSync(process.execPath, [ORCHESTRATOR_STEER_SCRIPT_PATH, wpId, "PRIMARY"], {
      encoding: "utf8",
      stdio: ["ignore", "pipe", "pipe"],
    });
    const outputLines = String(output || "")
      .split(/\r?\n/)
      .map((line) => line.trim())
      .filter(Boolean)
      .slice(-6);
    return {
      status: "DISPATCHED",
      reason: "AUTO_RELAY_TRIGGERED",
      next_actor: nextActor,
      output_lines: outputLines,
    };
  } catch (error) {
    const stderr = String(error?.stderr || "").trim();
    const stdout = String(error?.stdout || "").trim();
    return {
      status: "FAILED",
      reason: "AUTO_RELAY_FAILED",
      next_actor: nextActor,
      error: stderr || stdout || (error instanceof Error ? error.message : String(error)),
    };
  }
}

export function orchestratorSteerScriptPath() {
  return ORCHESTRATOR_STEER_SCRIPT_PATH;
}

export function applyWorkflowInvalidityRuntimeProjection(runtimeStatus = {}, entry = {}) {
  runtimeStatus.next_expected_actor = "ORCHESTRATOR";
  runtimeStatus.next_expected_session = null;
  runtimeStatus.waiting_on = "WORKFLOW_INVALIDITY";
  runtimeStatus.waiting_on_session = null;
  runtimeStatus.validator_trigger = "NONE";
  runtimeStatus.validator_trigger_reason = "Workflow invalidity flagged";
  runtimeStatus.attention_required = true;
  runtimeStatus.ready_for_validation = false;
  runtimeStatus.ready_for_validation_reason = null;
  if (!["completed", "failed", "canceled"].includes(String(runtimeStatus.runtime_status || "").trim().toLowerCase())) {
    runtimeStatus.runtime_status = "input_required";
  }
  return runtimeStatus;
}

function appendWpReceiptCore({
  wpId,
  actorRole,
  actorSession,
  receiptKind,
  summary,
  stateBefore = null,
  stateAfter = null,
  refs = [],
  branch = null,
  worktreeDir = null,
  timestamp = null,
  targetRole = null,
  targetSession = null,
  correlationId = null,
  requiresAck = false,
  ackFor = null,
  specAnchor = null,
  packetRowRef = null,
  microtaskContract = null,
  mechanicalResult = null,
  workflowInvalidityCode = null,
  verb = null,
  verbBody = null,
} = {}, options = {}) {
  const WP_ID = String(wpId || "").trim();
  if (!WP_ID || !/^WP-/.test(WP_ID)) {
    throw new Error("WP_ID is required");
  }

  const absorbed = absorbReceiptAppendArgs({
    wpId: WP_ID,
    actorRole,
    actorSession,
    receiptKind,
    summary,
    stateBefore,
    stateAfter,
    refs,
    branch,
    worktreeDir,
    timestamp,
    targetRole,
    targetSession,
    correlationId,
    requiresAck,
    ackFor,
    specAnchor,
    packetRowRef,
    microtaskContract,
    mechanicalResult,
    workflowInvalidityCode,
    verb,
    verbBody,
  });
  const receiptArgs = enrichReceiptArgsWithHeuristicRisk(absorbed.args);

  const preflight = options.skipPreflight
    ? null
    : validateWpReceiptAppendPreconditions({
      wpId: receiptArgs.wpId,
      actorRole: receiptArgs.actorRole,
      actorSession: receiptArgs.actorSession,
      receiptKind: receiptArgs.receiptKind,
      summary: receiptArgs.summary,
      stateBefore: receiptArgs.stateBefore,
      stateAfter: receiptArgs.stateAfter,
      refs: receiptArgs.refs,
      branch: receiptArgs.branch,
      worktreeDir: receiptArgs.worktreeDir,
      timestamp: receiptArgs.timestamp,
      targetRole: receiptArgs.targetRole,
      targetSession: receiptArgs.targetSession,
      correlationId: receiptArgs.correlationId,
      requiresAck: receiptArgs.requiresAck,
      ackFor: receiptArgs.ackFor,
      specAnchor: receiptArgs.specAnchor,
      packetRowRef: receiptArgs.packetRowRef,
      microtaskContract: receiptArgs.microtaskContract,
      mechanicalResult: receiptArgs.mechanicalResult,
      workflowInvalidityCode: receiptArgs.workflowInvalidityCode,
      verb: receiptArgs.verb,
      verbBody: receiptArgs.verbBody,
    });
  const context = preflight?.context || loadPacketContext(WP_ID);
  let runtimeStatus = preflight?.runtimeStatus;
  if (runtimeStatus === undefined) {
    runtimeStatus = context.runtimeStatusAbsPath && fs.existsSync(context.runtimeStatusAbsPath)
      ? parseJsonFile(context.runtimeStatusFile)
      : null;
  }
  const reviewTrackedReceipt = REVIEW_TRACKED_RECEIPT_KIND_VALUES.includes(String(receiptArgs.receiptKind || "").trim().toUpperCase());
  const entry = buildReceiptValidationEntry({
    args: {
      ...receiptArgs,
      wpId: WP_ID,
    },
    context,
    runtimeStatus,
    timestamp: receiptArgs.timestamp,
  });
  if (absorbed.applied.length > 0) {
    entry.metadata = {
      absorbers_applied: absorbed.applied.map((item) => item.name),
    };
  }

  const errors = validateReceipt(entry);
  if (errors.length > 0) {
    throw new Error(`Receipt validation failed: ${errors.join("; ")}`);
  }

  let autoRoute = null;
  let evaluation = null;
  const rerouteOnRepair = entry.receipt_kind === "REPAIR";
  if (runtimeStatus) {
    updateOpenReviewItems(runtimeStatus, entry);
    runtimeStatus.last_event = `receipt_${entry.receipt_kind.toLowerCase()}`;
    runtimeStatus.last_event_at = entry.timestamp_utc;
    if (entry.receipt_kind === WORKFLOW_INVALIDITY_RECEIPT_KIND) {
      applyWorkflowInvalidityRuntimeProjection(runtimeStatus, entry);
    } else if (reviewTrackedReceipt || rerouteOnRepair) {
      const receipts = parseJsonlFile(context.receiptsFile);
      evaluation = evaluateWpCommunicationHealth({
        wpId: WP_ID,
        stage: "STATUS",
        packetPath: context.packetPath,
        packetContent: context.packetText,
        workflowLane: context.workflowLane || runtimeStatus.workflow_lane || "",
        packetFormatVersion: context.packetFormatVersion || "",
        communicationContract: context.communicationContract || "",
        communicationHealthGate: context.communicationHealthGate || "",
        receipts: [...receipts, entry],
        runtimeStatus,
      });
      autoRoute = deriveWpCommunicationAutoRoute({
        evaluation,
        runtimeStatus,
        latestReceipt: entry,
      });
      runtimeStatus = syncReviewGovernanceTruth({
        wpId: WP_ID,
        context,
        entry,
        evaluation,
        runtimeStatus,
      });
    }
    const runtimeErrors = validateRuntimeStatus(runtimeStatus);
    if (runtimeErrors.length > 0) {
      throw new Error(`Runtime status validation failed after receipt append: ${runtimeErrors.join("; ")}`);
    }
    writeJsonFile(context.runtimeStatusAbsPath, runtimeStatus);
  }

  appendJsonlLine(context.receiptsAbsPath, entry);
  tryAppendSessionEvent({
    wpId: WP_ID,
    role: entry.actor_role,
    sessionId: entry.actor_session,
    eventType: "receipt_emitted",
    event: {
      receipt_kind: entry.receipt_kind,
      verb: entry.verb || "",
      mt_id: String(entry.microtask_contract?.scope_ref || "").trim(),
      correlation_id: entry.correlation_id || "",
      summary: entry.summary || "",
    },
    timestamp: entry.timestamp_utc,
  });
  if (reviewTrackedReceipt) {
    appendReviewNotifications({ wpId: WP_ID, workflowLane: context.workflowLane, entry, autoRoute });
  }

  // RGF-100: MT retry counter — after a REVIEW_RESPONSE is written, count non-PASS
  // responses for the same MT. Escalate to orchestrator when the threshold is reached.
  try {
    if (entry.receipt_kind === "REVIEW_RESPONSE") {
      const mtId = String(entry.microtask_contract?.scope_ref || "").trim()
        || (String(entry.correlation_id || "").match(/(MT-\d+)/i) || [])[1]
        || (String(entry.summary || "").match(/(MT-\d+)/i) || [])[1]
        || null;
      if (mtId) {
        const allReceipts = parseJsonlFile(context.receiptsFile);
        const steerCount = allReceipts.filter((r) => {
          if (String(r.receipt_kind || "").trim().toUpperCase() !== "REVIEW_RESPONSE") return false;
          const rMtId = String(r.microtask_contract?.scope_ref || "").trim()
            || (String(r.correlation_id || "").match(/(MT-\d+)/i) || [])[1]
            || (String(r.summary || "").match(/(MT-\d+)/i) || [])[1]
            || null;
          if (rMtId !== mtId) return false;
          const verdict = deriveValidatorAssessmentVerdict(r);
          return verdict !== "PASS";
        }).length;
        const heuristicContract = heuristicRiskContractForReceipt({ wpId: WP_ID, mtId, entry });
        const heuristicThreshold = Number.isInteger(heuristicContract?.repair_cycle_strategy_threshold)
          ? heuristicContract.repair_cycle_strategy_threshold
          : HEURISTIC_RISK_REPAIR_ESCALATION_THRESHOLD;
        if (heuristicContract && steerCount === heuristicThreshold) {
          const riskSummary = summarizeHeuristicRiskContract(heuristicContract);
          console.warn(
            `[WP_RECEIPT] Heuristic-risk strategy escalation reached (${steerCount} REVIEW_RESPONSE receipts for ${mtId}). ${riskSummary}`,
          );
          appendWpNotificationCore({
            wpId: WP_ID,
            sourceKind: "HEURISTIC_RISK_STRATEGY_ESCALATION",
            sourceRole: entry.actor_role,
            targetRole: "ORCHESTRATOR",
            sourceSession: entry.actor_session,
            correlationId: entry.correlation_id ?? null,
            summary: `Heuristic-risk strategy escalation: ${mtId} has ${steerCount} non-PASS review response(s). ${riskSummary}. Change strategy before more threshold tuning.`,
            timestamp: entry.timestamp_utc,
          });
        }
        if (steerCount >= MT_FIX_CYCLE_ESCALATION_THRESHOLD) {
          console.warn(
            `[WP_RECEIPT] MT fix cycle limit reached (${steerCount} STEER responses for ${mtId}). Escalating to orchestrator.`,
          );
          appendWpNotificationCore({
            wpId: WP_ID,
            sourceKind: "MT_FIX_CYCLE_ESCALATION",
            sourceRole: entry.actor_role,
            targetRole: "ORCHESTRATOR",
            sourceSession: entry.actor_session,
            correlationId: entry.correlation_id ?? null,
            summary: `MT fix cycle limit reached: ${steerCount} STEER responses for ${mtId} without PASS. Review the repeated failure pattern.`,
            timestamp: entry.timestamp_utc,
          });
        }
      }
    }
  } catch (escalationError) {
    // Best-effort: counter/escalation failure must not block the receipt write.
    console.warn(`[WP_RECEIPT] MT fix cycle escalation check failed (non-fatal): ${escalationError?.message || escalationError}`);
  }

  // RGF-126: event-driven memory extraction — skip DB open entirely for non-signal receipts
  if (HIGH_SIGNAL_RECEIPT_KINDS.has(entry.receipt_kind)) {
    try {
      const { db: memDb } = openGovernanceMemoryDb();
      try {
        extractMemoryFromReceipt(memDb, WP_ID, entry);
      } finally { closeMemDb(memDb); }
    } catch {
      // Best-effort: memory extraction failure must not block the receipt write
    }
  }

  return { context, entry, autoRoute };
}

export function appendWpReceipt(args = {}, options = {}) {
  const WP_ID = String(args?.wpId || "").trim();
  const run = () => appendWpReceiptCore(args, options);
  const relayEnabled = options.autoRelay !== false;
  if (options.assumeTransactionLock || !WP_ID || !/^WP-/.test(WP_ID)) {
    const result = run();
    if (relayEnabled) {
      result.relayAttempt = attemptOrchestratorAutoRelay({
        wpId: WP_ID,
        context: result.context,
        entry: result.entry,
        autoRoute: result.autoRoute,
      });
    }
    return result;
  }
  const result = withFileLockSync(communicationTransactionLockPathForWp(WP_ID), run);
  if (relayEnabled) {
    result.relayAttempt = attemptOrchestratorAutoRelay({
      wpId: WP_ID,
      context: result.context,
      entry: result.entry,
      autoRoute: result.autoRoute,
    });
  }
  return result;
}

function runCli() {
  const positionals = [];
  const flags = { verb: null, verbBody: null };
  const rawArgs = process.argv.slice(2);
  for (let index = 0; index < rawArgs.length; index += 1) {
    const arg = String(rawArgs[index] || "").trim();
    if (arg === "--verb") {
      flags.verb = rawArgs[++index] || "";
      continue;
    }
    if (arg === "--verb-body") {
      flags.verbBody = rawArgs[++index] || "";
      continue;
    }
    positionals.push(rawArgs[index]);
  }
  const [wpId, actorRole, actorSession, receiptKind, summary, stateBefore, stateAfter, targetRole, targetSession, correlationId, requiresAck, ackFor, specAnchor, packetRowRef, workflowInvalidityCode] = positionals;
  if (!wpId || !actorRole || !actorSession || !receiptKind || !summary) {
    console.error(
      "Usage: node .GOV/roles_shared/scripts/wp/wp-receipt-append.mjs"
      + " WP-{ID} <ACTOR_ROLE> <ACTOR_SESSION> <RECEIPT_KIND> \"<SUMMARY>\""
      + " [STATE_BEFORE] [STATE_AFTER] [TARGET_ROLE] [TARGET_SESSION] [CORRELATION_ID] [REQUIRES_ACK] [ACK_FOR] [SPEC_ANCHOR] [PACKET_ROW_REF] [WORKFLOW_INVALIDITY_CODE]"
      + " [--verb <NAME> --verb-body '<JSON>']"
    );
    process.exit(1);
  }

  const { context, entry, relayAttempt } = appendWpReceipt({
    wpId,
    actorRole,
    actorSession,
    receiptKind,
    summary,
    stateBefore,
    stateAfter,
    targetRole,
    targetSession,
    correlationId,
    requiresAck: parseBooleanLike(requiresAck),
    ackFor,
    specAnchor,
    packetRowRef,
    workflowInvalidityCode,
    verb: flags.verb,
    verbBody: flags.verbBody,
  });

  console.log(`[WP_RECEIPT] appended ${entry.receipt_kind} for ${entry.wp_id}`);
  console.log(`- ledger: ${context.receiptsFile}`);
  console.log(`- timestamp_utc: ${entry.timestamp_utc}`);
  if (entry.verb) console.log(`- verb: ${entry.verb}`);
  if (relayAttempt && relayAttempt.status !== "NOT_APPLICABLE") {
    console.log(`- auto_relay_status: ${relayAttempt.status}`);
    console.log(`- auto_relay_reason: ${relayAttempt.reason}`);
    if (relayAttempt.next_actor) console.log(`- auto_relay_next_actor: ${relayAttempt.next_actor}`);
    if (relayAttempt.error) console.log(`- auto_relay_error: ${relayAttempt.error}`);
    if (Array.isArray(relayAttempt.output_lines) && relayAttempt.output_lines.length > 0) {
      console.log(`- auto_relay_output: ${relayAttempt.output_lines.join(" | ")}`);
    }
  }
}

if (isInvokedAsMain(import.meta.url, process.argv[1])) {
  runCli();
}
