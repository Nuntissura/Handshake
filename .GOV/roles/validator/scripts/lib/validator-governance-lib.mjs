import fs from "node:fs";
import path from "node:path";
import { spawnSync } from "node:child_process";
import { buildPhaseCheckCommand } from "../../../../roles_shared/checks/phase-check-lib.mjs";
import { captureCheckFinding } from "../../../../roles_shared/scripts/memory/memory-capture-from-check.mjs";
import { registerFailCaptureHook } from "../../../../roles_shared/scripts/lib/fail-capture-lib.mjs";
registerFailCaptureHook("validator-governance-lib.mjs", { role: process.env.HANDSHAKE_VALIDATOR_ROLE || "VALIDATOR" });
import {
  computedPolicyOutcomeAllowsClosure,
  evaluateComputedPolicyGateFromPacketText,
} from "../../../../roles_shared/scripts/lib/computed-policy-gate-lib.mjs";
import {
  currentGitContext,
  lastGateLog,
  loadOrchestratorGateLogs,
  loadPacket,
  packetExists,
  packetPath,
  parseClaimField,
  parseMergeBaseSha,
  preparedWorktreeSyncState,
  resolvePrepareWorktreeAbs,
} from "../../../../roles_shared/scripts/lib/role-resume-utils.mjs";
import {
  GOV_ROOT_ABS,
  GOV_ROOT_REPO_REL,
  normalizePath,
  REPO_ROOT,
  repoPathAbs,
  resolveWorkPacketPath,
  workPacketAbsPath,
  workPacketPath,
  WORK_PACKET_STORAGE_ROOT_REPO_REL,
} from "../../../../roles_shared/scripts/lib/runtime-paths.mjs";
import { formatBoundedItemList, parsePacketScopeList } from "../../../../roles_shared/scripts/lib/scope-surface-lib.mjs";
import {
  evaluateWpCommunicationHealth,
  deriveLatestValidatorAssessment,
  isOverlapMicrotaskReviewItem,
} from "../../../../roles_shared/scripts/lib/wp-communication-health-lib.mjs";
import {
  activeWorkflowInvalidityReceipt,
  parseJsonFile,
  parseJsonlFile,
  workflowInvalidityReceipts,
} from "../../../../roles_shared/scripts/lib/wp-communications-lib.mjs";
import { ensureValidatorGateDir, resolveValidatorGatePath } from "../../../../roles_shared/scripts/lib/validator-gate-paths.mjs";
import { loadSessionRegistry } from "../../../../roles_shared/scripts/session/session-registry-lib.mjs";
import {
  defaultIntegrationValidatorBranch,
  defaultIntegrationValidatorWorktreeDir,
  defaultWpValidatorBranch,
  defaultWpValidatorWorktreeDir,
  packetRequiresCompletionLayerVerdicts,
  packetRequiresSpecClauseMap,
  packetUsesStructuredValidationReport,
} from "../../../../roles_shared/scripts/session/session-policy.mjs";
import {
  committedEvidenceForCloseout,
  livePrepareWorktreeHealthEvidence,
  recordCommittedValidationRun,
} from "./committed-validation-evidence-lib.mjs";
import { validateClauseReportConsistency, validatePacketClosureMonitoring } from "../../../../roles_shared/scripts/lib/packet-closure-monitor-lib.mjs";
import { validateMergeProgressionTruth } from "../../../../roles_shared/scripts/lib/merge-progression-truth-lib.mjs";
import { validateSemanticProofAssets } from "../../../../roles_shared/scripts/lib/semantic-proof-lib.mjs";
import { validateSignedScopeCompatibilityTruth } from "../../../../roles_shared/scripts/lib/signed-scope-compatibility-lib.mjs";
import { validateContainedMainCommitAgainstSignedScope } from "../../../../roles_shared/scripts/lib/signed-scope-surface-lib.mjs";
import {
  packetUsesDataContractProfile,
  parseDataContractProfile,
  validateDataContractDecisionSection,
  validateDataContractSection,
} from "../../../../roles_shared/scripts/lib/data-contract-lib.mjs";
import { evaluateWpDeclaredTopology } from "../../../../roles_shared/scripts/lib/wp-declared-topology-lib.mjs";
import {
  validatorReportProfileRequiresAntiVibe,
  validatorReportProfileRequiresPrimitiveAudit,
  validatorReportProfileRequiresRiskAudit,
  validatorReportProfileUsesHeuristicRigor,
} from "../../../../roles_shared/scripts/lib/validator-report-profile-lib.mjs";

function parseStatus(packetContent) {
  return (
    (String(packetContent || "").match(/^\s*-\s*\*\*Status:\*\*\s*(.+)\s*$/mi) || [])[1]
    || (String(packetContent || "").match(/^\s*\*\*Status:\*\*\s*(.+)\s*$/mi) || [])[1]
    || (String(packetContent || "").match(/^\s*Status:\s*(.+)\s*$/mi) || [])[1]
    || ""
  ).trim();
}

function repoRelativeFileExists(filePath = "") {
  const relPath = String(filePath || "").trim();
  if (!relPath) return false;
  try {
    return path.isAbsolute(relPath) ? false : fs.existsSync(repoPathAbs(relPath));
  } catch {
    return false;
  }
}

function normalizeSession(value) {
  const raw = String(value || "").trim();
  if (!raw || /^<unassigned>$/i.test(raw)) return null;
  return raw || null;
}

function matchesSessionOfRecord(sessionOfRecord, candidates = []) {
  const expected = normalizeSession(sessionOfRecord);
  if (!expected) return true;
  return candidates.some((candidate) => normalizeSession(candidate) === expected);
}

export function normalizeValidatorRole(value) {
  const normalized = String(value || "").trim().toUpperCase().replace(/[\s-]+/g, "_");
  if (!normalized) return "";
  if (normalized === "VALIDATOR" || normalized === "CLASSICAL") return "CLASSICAL_VALIDATOR";
  if (normalized === "WPVALIDATOR") return "WP_VALIDATOR";
  if (normalized === "INTEGRATIONVALIDATOR") return "INTEGRATION_VALIDATOR";
  return normalized;
}

function currentWorktreeRepoRelative(repoRoot, gitContext = {}) {
  const root = path.resolve(repoRoot || REPO_ROOT);
  const topLevel = String(gitContext?.topLevel || "").trim();
  if (!topLevel) return "";
  return normalizePath(path.relative(root, path.resolve(topLevel))) || ".";
}

function currentWorktreeAbsolute(gitContext = {}) {
  const topLevel = String(gitContext?.topLevel || "").trim();
  if (!topLevel) return "";
  return normalizePath(path.resolve(topLevel));
}

function resolveConfiguredWorktreeAbsolute(repoRoot, worktreeDir = "") {
  if (!String(worktreeDir || "").trim()) return "";
  return normalizePath(path.resolve(repoRoot || REPO_ROOT, String(worktreeDir || "").trim()));
}

function currentBranchName(gitContext = {}) {
  return String(gitContext?.branch || "").trim();
}

function sameWorktreePath(left, right) {
  return normalizePath(left).toLowerCase() === normalizePath(right).toLowerCase();
}

function resolveLiveGovernanceRootAbs(governanceRootAbs = "") {
  return normalizePath(path.resolve(governanceRootAbs || GOV_ROOT_ABS || path.resolve(REPO_ROOT, ".GOV")));
}

function matchRegistrySessionToGitContext(session, repoRoot, gitContext = {}) {
  const branch = currentBranchName(gitContext);
  const currentWorktreeAbs = currentWorktreeAbsolute(gitContext);
  const sessionWorktreeAbs = resolveConfiguredWorktreeAbsolute(repoRoot, session?.local_worktree_dir || "");
  return (
    String(session?.local_branch || "").trim() === branch
    && !!currentWorktreeAbs
    && !!sessionWorktreeAbs
    && sameWorktreePath(sessionWorktreeAbs, currentWorktreeAbs)
  );
}

export function readValidatorAuthority(packetContent = "") {
  const workflowLane = String(parseClaimField(packetContent, "WORKFLOW_LANE") || "").trim().toUpperCase();
  const technicalAuthority =
    normalizeValidatorRole(parseClaimField(packetContent, "TECHNICAL_AUTHORITY"))
    || (workflowLane === "ORCHESTRATOR_MANAGED" ? "INTEGRATION_VALIDATOR" : "CLASSICAL_VALIDATOR");
  const mergeAuthority =
    normalizeValidatorRole(parseClaimField(packetContent, "MERGE_AUTHORITY"))
    || technicalAuthority;
  return {
    workflowLane,
    technicalAuthority,
    mergeAuthority,
    wpValidatorOfRecord: String(parseClaimField(packetContent, "WP_VALIDATOR_OF_RECORD") || "").trim(),
    integrationValidatorOfRecord: String(parseClaimField(packetContent, "INTEGRATION_VALIDATOR_OF_RECORD") || "").trim(),
  };
}

function validatorReadyMessage(actorRole, waitingOn, communicationState = null) {
  const normalizedRole = normalizeValidatorRole(actorRole);
  const waiting = String(waitingOn || "").trim().toUpperCase();
  const latestAssessment = communicationState?.latestValidatorAssessment || null;
  if (normalizedRole === "WP_VALIDATOR") {
    if (waiting === "WP_VALIDATOR_INTENT_CHECKPOINT") {
      return "Coder intent is recorded; WP validator checkpoint review is required before implementation or full handoff.";
    }
    if (waiting === "WP_VALIDATOR_REVIEW") {
      return "Coder handoff recorded; WP validator review is required now.";
    }
    if (waiting.startsWith("OPEN_REVIEW_ITEM_")) {
      return "Open review traffic is targeted to WP Validator and requires a reply.";
    }
    return "WP Validator is the projected next actor for the current governed validation step.";
  }
  if (normalizedRole === "INTEGRATION_VALIDATOR") {
    if (waiting.startsWith("OPEN_REVIEW_ITEM_REVIEW_REQUEST")) {
      return "Coder opened the final direct review exchange; Integration Validator response is required now.";
    }
    if (latestAssessment?.verdict === "PASS") {
      return "Integration Validator assessment is positioned for final review progression.";
    }
    return "Integration Validator is the projected next actor for the current governed validation step.";
  }
  return "Validator lane is the projected next actor for the current governed validation step.";
}

function parallelMicrotaskReviewItems(actorRole, communicationState = null) {
  const normalizedRole = normalizeValidatorRole(actorRole);
  if (normalizedRole !== "WP_VALIDATOR") return [];
  const runtimeStatus = communicationState?.runtimeStatus || {};
  const actorSession = normalizeSession(runtimeStatus?.wp_validator_of_record)
    || normalizeSession(
      (Array.isArray(runtimeStatus?.active_role_sessions) ? runtimeStatus.active_role_sessions : [])
        .find((entry) => normalizeValidatorRole(entry?.role) === normalizedRole)?.session_id,
    );
  return (Array.isArray(runtimeStatus?.open_review_items) ? runtimeStatus.open_review_items : []).filter((item) => {
    if (!isOverlapMicrotaskReviewItem(item)) return false;
    const targetSession = String(item?.target_session || "").trim();
    if (!targetSession || !actorSession) return true;
    return targetSession === actorSession;
  });
}

function blockedValidatorMessage(actorRole, nextExpectedActor, waitingOn, communicationState = null) {
  const normalizedRole = normalizeValidatorRole(actorRole);
  const nextActor = normalizeValidatorRole(nextExpectedActor);
  const waiting = String(waitingOn || "").trim();
  const latestAssessment = communicationState?.latestValidatorAssessment || null;

  if (nextActor === "CODER") {
    if (latestAssessment?.verdict === "FAIL") {
      return `Latest validator assessment already recorded FAIL; coder remediation is next (${waiting || "CODER_REPAIR_HANDOFF"}).`;
    }
    return `Validator work is not the current route target; runtime expects CODER next (${waiting || "implementation"}).`;
  }
  if (nextActor === "ORCHESTRATOR") {
    if (latestAssessment) {
      return `Latest validator assessment already recorded ${latestAssessment.verdict}; orchestrator progression is next (${waiting || "governance"}).`;
    }
    return `Validator work is not the current route target; runtime expects ORCHESTRATOR next (${waiting || "governance"}).`;
  }
  if (nextActor && nextActor !== normalizedRole) {
    return `Validator work is not the current route target; runtime expects ${nextActor} next (${waiting || "governed progression"}).`;
  }
  return "Validator work is not the current route target yet.";
}

export function loadValidatorCommunicationState({
  wpId = "",
  packetPath = "",
  packetContent = "",
} = {}) {
  const runtimeStatusFile = String(parseClaimField(packetContent, "WP_RUNTIME_STATUS_FILE") || "").trim();
  const receiptsFile = String(parseClaimField(packetContent, "WP_RECEIPTS_FILE") || "").trim();
  if (!runtimeStatusFile || !repoRelativeFileExists(runtimeStatusFile)) return null;

  const runtimeStatus = parseJsonFile(runtimeStatusFile);
  const receipts = receiptsFile && repoRelativeFileExists(receiptsFile) ? parseJsonlFile(receiptsFile) : [];
  const communicationEvaluation = evaluateWpCommunicationHealth({
    wpId,
    stage: "STATUS",
    packetPath,
    packetContent,
    workflowLane: parseClaimField(packetContent, "WORKFLOW_LANE"),
    packetFormatVersion: parseClaimField(packetContent, "PACKET_FORMAT_VERSION"),
    communicationContract: parseClaimField(packetContent, "COMMUNICATION_CONTRACT"),
    communicationHealthGate: parseClaimField(packetContent, "COMMUNICATION_HEALTH_GATE"),
    receipts,
    runtimeStatus,
  });

  return {
    runtimeStatus,
    receipts,
    communicationEvaluation,
    latestValidatorAssessment: deriveLatestValidatorAssessment(receipts),
  };
}

export function deriveValidatorResumeState({
  actorRole = "",
  communicationState = null,
} = {}) {
  const normalizedRole = normalizeValidatorRole(actorRole);
  const nextExpectedActor = normalizeValidatorRole(communicationState?.runtimeStatus?.next_expected_actor);
  const waitingOn = String(communicationState?.runtimeStatus?.waiting_on || "").trim();
  const latestAssessment = communicationState?.latestValidatorAssessment || null;
  const communicationApplicable = Boolean(communicationState?.communicationEvaluation?.applicable);
  const overlapQueue = parallelMicrotaskReviewItems(normalizedRole, communicationState);

  if (!["WP_VALIDATOR", "INTEGRATION_VALIDATOR"].includes(normalizedRole) || !communicationApplicable) {
    return {
      ready: false,
      blockedByRoute: false,
      parallelReviewReady: false,
      nextExpectedActor,
      waitingOn,
      latestAssessment,
      message: "",
    };
  }

  if (nextExpectedActor === normalizedRole) {
    return {
      ready: true,
      blockedByRoute: false,
      parallelReviewReady: false,
      nextExpectedActor,
      waitingOn,
      latestAssessment,
      message: validatorReadyMessage(normalizedRole, waitingOn, communicationState),
    };
  }

  if (overlapQueue.length > 0) {
    return {
      ready: true,
      blockedByRoute: false,
      parallelReviewReady: true,
      nextExpectedActor,
      waitingOn,
      latestAssessment,
      message: `Parallel microtask review queue is available for WP validator while ${nextExpectedActor || "CODER"} continues implementation.`,
    };
  }

  if (nextExpectedActor) {
    return {
      ready: false,
      blockedByRoute: true,
      parallelReviewReady: false,
      nextExpectedActor,
      waitingOn,
      latestAssessment,
      message: blockedValidatorMessage(normalizedRole, nextExpectedActor, waitingOn, communicationState),
    };
  }

  return {
    ready: false,
    blockedByRoute: false,
    parallelReviewReady: false,
    nextExpectedActor,
    waitingOn,
    latestAssessment,
    message: "",
  };
}

export function resolveValidatorActorContext({
  repoRoot = REPO_ROOT,
  wpId = "",
  packetContent = "",
  gitContext = {},
  registrySessions = null,
} = {}) {
  const root = path.resolve(repoRoot || REPO_ROOT);
  const branch = currentBranchName(gitContext);
  const worktreeDir = currentWorktreeRepoRelative(root, gitContext);
  const worktreeAbs = currentWorktreeAbsolute(gitContext);
  const authority = readValidatorAuthority(packetContent);
  const sessions = Array.isArray(registrySessions)
    ? registrySessions
    : loadSessionRegistry(root).registry.sessions || [];

  const matchingGovernedSession = sessions.find((session) =>
    session?.wp_id === wpId
    && ["WP_VALIDATOR", "INTEGRATION_VALIDATOR"].includes(normalizeValidatorRole(session?.role))
    && matchRegistrySessionToGitContext(session, root, gitContext)
  );
  if (matchingGovernedSession) {
    return {
      actorRole: normalizeValidatorRole(matchingGovernedSession.role),
      actorSessionKey: String(matchingGovernedSession.session_key || "").trim(),
      actorSessionId: String(matchingGovernedSession.session_id || "").trim(),
      actorThreadId: String(matchingGovernedSession.session_thread_id || "").trim(),
      actorBranch: String(matchingGovernedSession.local_branch || branch || "").trim(),
      actorWorktreeDir: normalizePath(String(matchingGovernedSession.local_worktree_dir || worktreeDir || "").trim()),
      source: "SESSION_REGISTRY",
      authority,
    };
  }

  const expectedIntegrationWorktreeDir = defaultIntegrationValidatorWorktreeDir(wpId);
  const expectedIntegrationWorktreeAbs = resolveConfiguredWorktreeAbsolute(root, expectedIntegrationWorktreeDir);
  if (
    branch === defaultIntegrationValidatorBranch(wpId)
    && !!worktreeAbs
    && sameWorktreePath(worktreeAbs, expectedIntegrationWorktreeAbs)
  ) {
    return {
      actorRole: "INTEGRATION_VALIDATOR",
      actorSessionKey: "",
      actorSessionId: "",
      actorThreadId: "",
      actorBranch: branch,
      actorWorktreeDir: expectedIntegrationWorktreeDir,
      source: "WORKTREE_POLICY",
      authority,
    };
  }

  const expectedWpValidatorWorktreeDir = defaultWpValidatorWorktreeDir(wpId);
  const expectedWpValidatorWorktreeAbs = resolveConfiguredWorktreeAbsolute(root, expectedWpValidatorWorktreeDir);
  if (
    branch === defaultWpValidatorBranch(wpId)
    && !!worktreeAbs
    && sameWorktreePath(worktreeAbs, expectedWpValidatorWorktreeAbs)
  ) {
    return {
      actorRole: "WP_VALIDATOR",
      actorSessionKey: "",
      actorSessionId: "",
      actorThreadId: "",
      actorBranch: branch,
      actorWorktreeDir: expectedWpValidatorWorktreeDir,
      source: "WORKTREE_POLICY",
      authority,
    };
  }

  if (authority.workflowLane !== "ORCHESTRATOR_MANAGED" && branch === "main") {
    return {
      actorRole: "CLASSICAL_VALIDATOR",
      actorSessionKey: "",
      actorSessionId: "",
      actorThreadId: "",
      actorBranch: branch,
      actorWorktreeDir: worktreeDir,
      source: "MAIN_BRANCH_FALLBACK",
      authority,
    };
  }

  return {
    actorRole: "UNKNOWN",
    actorSessionKey: "",
    actorSessionId: "",
    actorThreadId: "",
    actorBranch: branch,
    actorWorktreeDir: worktreeDir,
    source: "UNRESOLVED",
    authority,
  };
}

export function evaluateValidatorPassAuthority({
  packetContent = "",
  actorContext = {},
} = {}) {
  const authority = readValidatorAuthority(packetContent);
  const issues = [];
  const actorRole = normalizeValidatorRole(actorContext?.actorRole);
  const actorSessionKey = String(actorContext?.actorSessionKey || "").trim();
  const actorSessionId = String(actorContext?.actorSessionId || "").trim();
  const actorSource = String(actorContext?.source || "").trim().toUpperCase();

  if (authority.technicalAuthority && authority.mergeAuthority && authority.technicalAuthority !== authority.mergeAuthority) {
    issues.push(
      `Split final authority is not yet supported by validator gate automation (technical=${authority.technicalAuthority}, merge=${authority.mergeAuthority}).`,
    );
  }

  if (authority.workflowLane === "ORCHESTRATOR_MANAGED") {
    if (!actorRole || actorRole === "UNKNOWN") {
      issues.push(
        `Unable to prove final validator lane from the current branch/worktree. Expected ${authority.technicalAuthority}.`,
      );
    } else if (actorRole !== authority.technicalAuthority) {
      issues.push(
        `PASS authority belongs to ${authority.technicalAuthority}; current lane resolved to ${actorRole}.`,
      );
    }

    if (actorRole === authority.technicalAuthority) {
      if (actorSource !== "SESSION_REGISTRY") {
        issues.push(
          `Final PASS authority for orchestrator-managed packets requires a governed ${authority.technicalAuthority} session; current lane was inferred from ${actorSource || "UNRESOLVED"}.`,
        );
      }
      if (!actorSessionKey || !actorSessionId) {
        issues.push(
          `Final PASS authority for orchestrator-managed packets requires governed validator session identity (session_key + session_id).`,
        );
      }
    }

    if (
      authority.technicalAuthority === "INTEGRATION_VALIDATOR"
      && authority.integrationValidatorOfRecord
      && authority.integrationValidatorOfRecord !== "<unassigned>"
      && !matchesSessionOfRecord(authority.integrationValidatorOfRecord, [actorSessionKey, actorSessionId])
    ) {
      issues.push(
        `Integration validator of record mismatch (packet=${authority.integrationValidatorOfRecord}, current=${actorSessionKey || actorSessionId || "<missing>"}).`,
      );
    }
  }

  return {
    ok: issues.length === 0,
    authority,
    issues,
  };
}

export function buildValidatorReadyCommands({
  wpId = "",
  actorRole = "",
  actorSessionId = "",
  postWorkCommand = "",
  waitingOn = "",
  parallelReview = false,
} = {}) {
  const role = normalizeValidatorRole(actorRole);
  if (role === "INTEGRATION_VALIDATOR") {
    const session = actorSessionId || "<integration-validator-session>";
    return [
      `just integration-validator-context-brief ${wpId}`,
      `just check-notifications ${wpId} INTEGRATION_VALIDATOR`,
      `just ack-notifications ${wpId} INTEGRATION_VALIDATOR ${session}`,
      buildPhaseCheckCommand({ phase: "CLOSEOUT", wpId }),
    ];
  }
  if (role === "WP_VALIDATOR") {
    const session = actorSessionId || "<wp-validator-session>";
    if (parallelReview) {
      return [
        `just check-notifications ${wpId} WP_VALIDATOR`,
        `just ack-notifications ${wpId} WP_VALIDATOR ${session}`,
        `just active-lane-brief WP_VALIDATOR ${wpId}`,
        `just wp-validator-response ${wpId} WP_VALIDATOR ${session} <coder-session> "<summary>" <correlation_id>`,
        `just wp-review-exchange VALIDATOR_QUERY ${wpId} WP_VALIDATOR ${session} CODER <coder-session> "<summary>" <correlation_id> [spec_anchor] [packet_row_ref] [ack_for] [microtask_json]`,
      ];
    }
    if (String(waitingOn || "").trim().toUpperCase() === "WP_VALIDATOR_INTENT_CHECKPOINT") {
      return [
        `just check-notifications ${wpId} WP_VALIDATOR`,
        `just ack-notifications ${wpId} WP_VALIDATOR ${session}`,
        `just active-lane-brief WP_VALIDATOR ${wpId}`,
        `just wp-validator-response ${wpId} WP_VALIDATOR ${session} <coder-session> "<summary>" <correlation_id>`,
        `just wp-spec-gap ${wpId} WP_VALIDATOR ${session} CODER <coder-session> "<summary>" [correlation_id] [spec_anchor] [packet_row_ref]`,
      ];
    }
    return [
      `just check-notifications ${wpId} WP_VALIDATOR`,
      `just ack-notifications ${wpId} WP_VALIDATOR ${session}`,
      buildPhaseCheckCommand({ phase: "HANDOFF", wpId, role: "WP_VALIDATOR" }),
    ];
  }
  return [
    postWorkCommand || buildPhaseCheckCommand({ phase: "HANDOFF", wpId, role: "CODER" }),
    buildPhaseCheckCommand({ phase: "HANDOFF", wpId, role: "WP_VALIDATOR" }),
    buildPhaseCheckCommand({ phase: "CLOSEOUT", wpId }),
  ].filter(Boolean);
}

export function evaluateValidatorPacketGovernanceState({
  wpId = "",
  packetPath = "",
  packetContent = "",
  currentWpStatus = "",
  taskBoardStatus = "",
  sessionStatus = "",
  actorContext = {},
  governanceRootAbs = "",
} = {}) {
  const packetStatus = parseStatus(packetContent);
  const computedPolicy = evaluateComputedPolicyGateFromPacketText(packetContent, {
    wpId,
    packetPath,
    requireClosedStatus: true,
  });

  if (computedPolicy.legacy_remediation_required) {
    const blockedMessage = computedPolicy.issues.blocked[0]?.message
      || "Closed structured packet requires remediation in a newer packet revision.";
    return {
      allowValidationResume: false,
      legacyRemediationRequired: true,
      terminalReason: "LEGACY_REMEDIATION_REQUIRED",
      packetStatus,
      currentWpStatus,
      taskBoardStatus,
      sessionStatus,
      computedPolicy,
      message: blockedMessage,
    };
  }

  const authority = readValidatorAuthority(packetContent);
  const actorRole = normalizeValidatorRole(actorContext?.actorRole);
  if (authority.workflowLane === "ORCHESTRATOR_MANAGED" && actorRole === "INTEGRATION_VALIDATOR") {
    const integrationWorktreeAbs = resolveConfiguredWorktreeAbsolute(REPO_ROOT, defaultIntegrationValidatorWorktreeDir(wpId));
    const localMainGovernanceAbs = normalizePath(path.join(integrationWorktreeAbs, ".GOV"));
    const liveGovernanceRootAbs = resolveLiveGovernanceRootAbs(governanceRootAbs);
    if (sameWorktreePath(liveGovernanceRootAbs, localMainGovernanceAbs)) {
      return {
        allowValidationResume: false,
        legacyRemediationRequired: false,
        terminalReason: "INTEGRATION_VALIDATOR_GOV_ROOT_MISCONFIGURED",
        packetStatus,
        currentWpStatus,
        taskBoardStatus,
        sessionStatus,
        computedPolicy,
        message:
          "Integration Validator lane is misconfigured: live governance still resolves to handshake_main/.GOV instead of the kernel. Set HANDSHAKE_GOV_ROOT to wt-gov-kernel/.GOV before resuming governed final review.",
      };
    }
  }

  return {
    allowValidationResume: true,
    legacyRemediationRequired: false,
    terminalReason: "ACTIVE",
    packetStatus,
    currentWpStatus,
    taskBoardStatus,
    sessionStatus,
    computedPolicy,
    message: "Packet remains validator-resumable under current governance state.",
  };
}

export function buildValidatorPacketCompleteResult({
  wpId: requestedWpId = "",
} = {}) {
  const wpId = String(requestedWpId || "").trim();
  if (!wpId) {
    return {
      ok: false,
      exitCode: 1,
      message: "validator-packet-complete: FAIL - WP_ID is required",
    };
  }

  try {
    const packetPath = workPacketPath(wpId);
    const packetAbsPath = workPacketAbsPath(wpId);

    function fail(msg) {
      throw new Error(String(msg || "").trim());
    }

    let text;
    try {
      text = fs.readFileSync(packetAbsPath, "utf8");
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

    function loadWorkflowInvalidityEntries() {
      const receiptsFile = parseSingleField("WP_RECEIPTS_FILE");
      if (!receiptsFile) return { history: [], active: null };
      try {
        const receipts = parseJsonlFile(receiptsFile);
        return {
          history: workflowInvalidityReceipts(receipts),
          active: activeWorkflowInvalidityReceipt(receipts),
        };
      } catch (error) {
        fail(`cannot read workflow invalidity receipts from ${receiptsFile}: ${error.message}`);
      }
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

    function hasListItemAfterLabel(sectionText, label) {
      const sectionLines = String(sectionText || "").split(/\r?\n/);
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

    function extractListItemsAfterLabel(sectionText, label) {
      const sectionLines = String(sectionText || "").split(/\r?\n/);
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
      const sectionValue = String(value || "").trim();
      if (!sectionValue) return false;
      return (
        /`[^`]+`/.test(sectionValue)
        || /\b[\w./-]+\.(?:rs|ts|tsx|js|jsx|mjs|cjs|py|go|java|cs|cpp|c|h|hpp|json|ya?ml|toml|sql)(?::\d+)?\b/i.test(sectionValue)
        || /\b[A-Za-z_][A-Za-z0-9_]*::[A-Za-z_][A-Za-z0-9_]*\b/.test(sectionValue)
        || /\b[A-Za-z_][A-Za-z0-9_]*\([^)]*\)/.test(sectionValue)
      );
    }

    function lacksConcreteListEvidence(items = []) {
      return items.some((item) => !/^NONE$/i.test(String(item || "").trim()) && !hasConcreteCodeReference(item));
    }

    const statusMatch = text.match(/(?:\*\*Status:\*\*|STATUS:)\s*(Ready for Dev|In Progress|Blocked|Done(?:\s*\(Historical\))?|Validated\s*\((?:PASS|FAIL|OUTDATED_ONLY|ABANDONED)\))(?=\s|$)/i);
    if (!statusMatch) {
      fail("STATUS missing or invalid (must be Ready for Dev / In Progress / Blocked / Done / Done (Historical) / Validated (PASS|FAIL|OUTDATED_ONLY|ABANDONED))");
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

    const packetFormatVersion = parseSingleField("PACKET_FORMAT_VERSION");
    const workflowInvalidityState = loadWorkflowInvalidityEntries();
    const workflowInvalidityEntries = workflowInvalidityState.history;
    const activeWorkflowInvalidity = workflowInvalidityState.active;
    const usesDataContractProfile = packetUsesDataContractProfile(packetFormatVersion);
    const dataContractProfile = parseDataContractProfile(text);
    const inScopePaths = parsePacketScopeList(text, "IN_SCOPE_PATHS", { stopLabels: ["OUT_OF_SCOPE"] });
    const topologyEvaluation = evaluateWpDeclaredTopology({
      repoRoot: REPO_ROOT,
      wpId,
      packetContent: text,
    });
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
      const usesHeuristicRigorReport = validatorReportProfileUsesHeuristicRigor(validatorReportProfile);
      const usesRiskAuditReport = validatorReportProfileRequiresRiskAudit(validatorReportProfile);
      const usesPrimitiveAuditReport = validatorReportProfileRequiresPrimitiveAudit(validatorReportProfile);
      const usesAntiVibeRigorReport = validatorReportProfileRequiresAntiVibe(validatorReportProfile, packetFormatVersion);
      const usesCompletionLayerVerdicts = packetRequiresCompletionLayerVerdicts(packetFormatVersion);
      let computedPolicy = null;

      if (usesDataContractProfile) {
        const rawDataContractProfile = parseSingleField("DATA_CONTRACT_PROFILE");
        if (isPlaceholder(rawDataContractProfile)) {
          fail("DATA_CONTRACT_PROFILE missing/placeholder for PACKET_FORMAT_VERSION >= 2026-04-01");
        }
        const dataContractDecisionValidation = validateDataContractDecisionSection(text, {
          packetPath,
          inScopePaths,
        });
        if (dataContractDecisionValidation.errors.length > 0) {
          fail(`data contract decision invalid: ${dataContractDecisionValidation.errors.join("; ")}`);
        }
        const dataContractValidation = validateDataContractSection(text, {
          packetPath,
        });
        if (dataContractValidation.errors.length > 0) {
          fail(`data contract monitoring invalid: ${dataContractValidation.errors.join("; ")}`);
        }
      }

      if (closureStatus && packetUsesStructuredValidationReport(packetFormatVersion)) {
        computedPolicy = evaluateComputedPolicyGateFromPacketText(text, {
          wpId,
          packetPath,
          requireClosedStatus: true,
        });
        if (computedPolicy.legacy_remediation_required) {
          const details = computedPolicy.issues.blocked.map((item) => `${item.code}: ${item.message}`);
          fail(`legacy remediation required for closed structured packet${details.length > 0 ? ` (${details.join("; ")})` : ""}`);
        }

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
        const mergeProgressionTruth = validateMergeProgressionTruth(text, {
          packetPath,
        });
        if (mergeProgressionTruth.errors.length > 0) {
          fail(`merge progression truth invalid for closed packet: ${mergeProgressionTruth.errors.join("; ")}`);
        }
        const signedScopeCompatibilityTruth = validateSignedScopeCompatibilityTruth(text, {
          packetPath,
        });
        if (signedScopeCompatibilityTruth.errors.length > 0) {
          fail(`signed scope compatibility truth invalid for closed packet: ${signedScopeCompatibilityTruth.errors.join("; ")}`);
        }
        if (/^Validated\s*\(\s*PASS\s*\)$/i.test(statusValue)) {
          const containedMainScope = validateContainedMainCommitAgainstSignedScope(text, {
            repoRoot: REPO_ROOT,
            mergedMainCommit: mergeProgressionTruth?.parsed?.mergedMainCommit || "",
            requireExactArtifactMatch: false,
          });
          if (containedMainScope.errors.length > 0) {
            fail(`contained main commit violates signed scope surface: ${containedMainScope.errors.join("; ")}`);
          }
        }
        if (!topologyEvaluation.ok) {
          fail(`declared WP topology invalid for closed packet: ${topologyEvaluation.issues.join("; ")}`);
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
        if (usesRiskAuditReport) {
          requiredSingleFields.push("VALIDATOR_RISK_TIER");
        }
        if (usesCompletionLayerVerdicts) {
          requiredSingleFields.push(
            "WORKFLOW_VALIDITY",
            "SCOPE_VALIDITY",
            "PROOF_COMPLETENESS",
            "INTEGRATION_READINESS",
            "DOMAIN_GOAL_COMPLETION",
          );
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
        if (!/^\s*Verdict\s*:\s*(PASS|FAIL|NOT_PROVEN|OUTDATED_ONLY|ABANDONED|BLOCKED)\b/im.test(validationReports)) {
          fail("VALIDATION_REPORTS missing top-level Verdict: PASS|FAIL|NOT_PROVEN|OUTDATED_ONLY|ABANDONED|BLOCKED");
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
        if (usesRiskAuditReport && usesAntiVibeRigorReport && !hasListItemAfterLabel(validationReports, "ANTI_VIBE_FINDINGS")) {
          fail("ANTI_VIBE_FINDINGS missing/placeholder list items in VALIDATION_REPORTS for closed packet");
        }
        if (usesRiskAuditReport && usesAntiVibeRigorReport && !hasListItemAfterLabel(validationReports, "SIGNED_SCOPE_DEBT")) {
          fail("SIGNED_SCOPE_DEBT missing/placeholder list items in VALIDATION_REPORTS for closed packet");
        }
        if (usesRiskAuditReport && !hasListItemAfterLabel(validationReports, "DIFF_ATTACK_SURFACES")) {
          fail("DIFF_ATTACK_SURFACES missing/placeholder list items in VALIDATION_REPORTS for closed packet");
        }
        if (usesRiskAuditReport && !hasListItemAfterLabel(validationReports, "INDEPENDENT_CHECKS_RUN")) {
          fail("INDEPENDENT_CHECKS_RUN missing/placeholder list items in VALIDATION_REPORTS for closed packet");
        }
        if (usesRiskAuditReport && !hasListItemAfterLabel(validationReports, "COUNTERFACTUAL_CHECKS")) {
          fail("COUNTERFACTUAL_CHECKS missing/placeholder list items in VALIDATION_REPORTS for closed packet");
        }
        if (usesRiskAuditReport && !hasListItemAfterLabel(validationReports, "INDEPENDENT_FINDINGS")) {
          fail("INDEPENDENT_FINDINGS missing/placeholder list items in VALIDATION_REPORTS for closed packet");
        }
        if (usesRiskAuditReport && !hasListItemAfterLabel(validationReports, "RESIDUAL_UNCERTAINTY")) {
          fail("RESIDUAL_UNCERTAINTY missing/placeholder list items in VALIDATION_REPORTS for closed packet");
        }
        if (usesRiskAuditReport && packetRequiresSpecClauseMap(packetFormatVersion) && !hasListItemAfterLabel(validationReports, "SPEC_CLAUSE_MAP")) {
          fail("SPEC_CLAUSE_MAP missing/placeholder list items in VALIDATION_REPORTS for closed packet (required for RIGOR_V3)");
        }
        if (usesRiskAuditReport && packetRequiresSpecClauseMap(packetFormatVersion)) {
          const negativeProofItems = extractListItemsAfterLabel(validationReports, "NEGATIVE_PROOF");
          if (negativeProofItems.length === 0 || hasOnlyNoneList(negativeProofItems)) {
            fail("NEGATIVE_PROOF must list at least one spec requirement verified as NOT fully implemented (required for RIGOR_V3)");
          }
        }
        if (usesPrimitiveAuditReport && !hasListItemAfterLabel(validationReports, "PRIMITIVE_RETENTION_PROOF")) {
          fail("PRIMITIVE_RETENTION_PROOF missing/placeholder list items in VALIDATION_REPORTS for closed packet");
        }
        if (usesPrimitiveAuditReport && !hasListItemAfterLabel(validationReports, "PRIMITIVE_RETENTION_GAPS")) {
          fail("PRIMITIVE_RETENTION_GAPS missing/placeholder list items in VALIDATION_REPORTS for closed packet");
        }
        if (usesPrimitiveAuditReport && !hasListItemAfterLabel(validationReports, "SHARED_SURFACE_INTERACTION_CHECKS")) {
          fail("SHARED_SURFACE_INTERACTION_CHECKS missing/placeholder list items in VALIDATION_REPORTS for closed packet");
        }
        if (usesPrimitiveAuditReport && !hasListItemAfterLabel(validationReports, "CURRENT_MAIN_INTERACTION_CHECKS")) {
          fail("CURRENT_MAIN_INTERACTION_CHECKS missing/placeholder list items in VALIDATION_REPORTS for closed packet");
        }
        if (dataContractProfile === "LLM_FIRST_DATA_V1") {
          const dataContractProofItems = extractListItemsAfterLabel(validationReports, "DATA_CONTRACT_PROOF");
          if (dataContractProofItems.length === 0 || hasOnlyNoneList(dataContractProofItems)) {
            fail("DATA_CONTRACT_PROOF must list concrete proof items in VALIDATION_REPORTS for active data contract packet");
          }
          if (!hasListItemAfterLabel(validationReports, "DATA_CONTRACT_GAPS")) {
            fail("DATA_CONTRACT_GAPS missing/placeholder list items in VALIDATION_REPORTS for active data contract packet");
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
        const topLevelVerdictMatch = validationReports.match(/^\s*Verdict\s*:\s*(.+)\s*$/im);
        const topLevelVerdict = topLevelVerdictMatch ? (topLevelVerdictMatch[1] || "").trim().toUpperCase() : "";
        const validationContextMatch = validationReports.match(/^\s*VALIDATION_CONTEXT\s*:\s*(.+)\s*$/im);
        const validationContext = validationContextMatch ? (validationContextMatch[1] || "").trim().toUpperCase() : "";
        const governanceVerdictMatch = validationReports.match(/^\s*GOVERNANCE_VERDICT\s*:\s*(.+)\s*$/im);
        const governanceVerdict = governanceVerdictMatch ? (governanceVerdictMatch[1] || "").trim().toUpperCase() : "";
        const environmentVerdictMatch = validationReports.match(/^\s*ENVIRONMENT_VERDICT\s*:\s*(.+)\s*$/im);
        const environmentVerdict = environmentVerdictMatch ? (environmentVerdictMatch[1] || "").trim().toUpperCase() : "";
        const dispositionMatch = validationReports.match(/^\s*DISPOSITION\s*:\s*(.+)\s*$/im);
        const disposition = dispositionMatch ? (dispositionMatch[1] || "").trim().toUpperCase() : "";
        const mainBodyGaps = extractListItemsAfterLabel(validationReports, "MAIN_BODY_GAPS");
        const qualityRisks = extractListItemsAfterLabel(validationReports, "QUALITY_RISKS");
        const antiVibeFindings = extractListItemsAfterLabel(validationReports, "ANTI_VIBE_FINDINGS");
        const signedScopeDebt = extractListItemsAfterLabel(validationReports, "SIGNED_SCOPE_DEBT");
        const notProvenItems = extractListItemsAfterLabel(validationReports, "NOT_PROVEN");
        const attackSurfaces = extractListItemsAfterLabel(validationReports, "DIFF_ATTACK_SURFACES");
        const independentChecks = extractListItemsAfterLabel(validationReports, "INDEPENDENT_CHECKS_RUN");
        const counterfactualChecks = extractListItemsAfterLabel(validationReports, "COUNTERFACTUAL_CHECKS");
        const residualUncertainty = extractListItemsAfterLabel(validationReports, "RESIDUAL_UNCERTAINTY");
        const boundaryProbes = extractListItemsAfterLabel(validationReports, "BOUNDARY_PROBES");
        const negativePathChecks = extractListItemsAfterLabel(validationReports, "NEGATIVE_PATH_CHECKS");
        const negativeProofItems = extractListItemsAfterLabel(validationReports, "NEGATIVE_PROOF");
        const dataContractProof = extractListItemsAfterLabel(validationReports, "DATA_CONTRACT_PROOF");
        const dataContractGaps = extractListItemsAfterLabel(validationReports, "DATA_CONTRACT_GAPS");
        const primitiveRetentionProof = extractListItemsAfterLabel(validationReports, "PRIMITIVE_RETENTION_PROOF");
        const primitiveRetentionGaps = extractListItemsAfterLabel(validationReports, "PRIMITIVE_RETENTION_GAPS");
        const sharedSurfaceInteractionChecks = extractListItemsAfterLabel(validationReports, "SHARED_SURFACE_INTERACTION_CHECKS");
        const currentMainInteractionChecks = extractListItemsAfterLabel(validationReports, "CURRENT_MAIN_INTERACTION_CHECKS");
        const packetRiskTier = parseSingleField("RISK_TIER").toUpperCase();
        const currentMainCompatibilityStatus = parseSingleField("CURRENT_MAIN_COMPATIBILITY_STATUS").toUpperCase();
        const sharedSurfaceRisk = parseSingleField("SHARED_SURFACE_RISK").toUpperCase();
        const abandonedClosure = topLevelVerdict === "ABANDONED" || /^Validated\s*\(\s*ABANDONED\s*\)$/i.test(statusValue);
        if (abandonedClosure) {
          if (topLevelVerdict !== "ABANDONED") {
            fail("Validated (ABANDONED) requires VALIDATION_REPORTS top-level Verdict: ABANDONED");
          }
          if (!/^Validated\s*\(\s*ABANDONED\s*\)$/i.test(statusValue)) {
            fail("Verdict=ABANDONED requires packet Status: Validated (ABANDONED)");
          }
          if (disposition !== "ABANDONED") {
            fail("Verdict=ABANDONED requires DISPOSITION=ABANDONED");
          }
        }

        if (activeWorkflowInvalidity && topLevelVerdict === "PASS") {
          fail(
            `Verdict=PASS prohibited when active WORKFLOW_INVALIDITY receipt exists (${activeWorkflowInvalidity?.workflow_invalidity_code || "UNKNOWN"}: ${activeWorkflowInvalidity?.summary || "<missing>"})`,
          );
        }
        if (usesHeuristicRigorReport && specAlignmentVerdict === "PASS" && !hasOnlyNoneList(mainBodyGaps)) {
          fail("SPEC_ALIGNMENT_VERDICT=PASS requires MAIN_BODY_GAPS to be exactly '- NONE'");
        }
        if (dataContractProfile === "LLM_FIRST_DATA_V1" && specAlignmentVerdict === "PASS" && !hasOnlyNoneList(dataContractGaps)) {
          fail("SPEC_ALIGNMENT_VERDICT=PASS requires DATA_CONTRACT_GAPS to be exactly '- NONE' for active data contract packet");
        }
        if (usesPrimitiveAuditReport && specAlignmentVerdict === "PASS" && !hasOnlyNoneList(primitiveRetentionGaps)) {
          fail("SPEC_ALIGNMENT_VERDICT=PASS requires PRIMITIVE_RETENTION_GAPS to be exactly '- NONE'");
        }
        if (usesCompletionLayerVerdicts) {
          const workflowValidityMatch = validationReports.match(/^\s*WORKFLOW_VALIDITY\s*:\s*(.+)\s*$/im);
          const workflowValidity = workflowValidityMatch ? (workflowValidityMatch[1] || "").trim().toUpperCase() : "";
          const scopeValidityMatch = validationReports.match(/^\s*SCOPE_VALIDITY\s*:\s*(.+)\s*$/im);
          const scopeValidity = scopeValidityMatch ? (scopeValidityMatch[1] || "").trim().toUpperCase() : "";
          const proofCompletenessMatch = validationReports.match(/^\s*PROOF_COMPLETENESS\s*:\s*(.+)\s*$/im);
          const proofCompleteness = proofCompletenessMatch ? (proofCompletenessMatch[1] || "").trim().toUpperCase() : "";
          const integrationReadinessMatch = validationReports.match(/^\s*INTEGRATION_READINESS\s*:\s*(.+)\s*$/im);
          const integrationReadiness = integrationReadinessMatch ? (integrationReadinessMatch[1] || "").trim().toUpperCase() : "";
          const domainGoalCompletionMatch = validationReports.match(/^\s*DOMAIN_GOAL_COMPLETION\s*:\s*(.+)\s*$/im);
          const domainGoalCompletion = domainGoalCompletionMatch ? (domainGoalCompletionMatch[1] || "").trim().toUpperCase() : "";

          if (workflowValidity === "VALID" && validationContext !== "OK") {
            fail("WORKFLOW_VALIDITY=VALID requires VALIDATION_CONTEXT=OK");
          }
          if (workflowValidity === "VALID" && governanceVerdict !== "PASS") {
            fail("WORKFLOW_VALIDITY=VALID requires GOVERNANCE_VERDICT=PASS");
          }
          if (activeWorkflowInvalidity) {
            if (workflowValidity === "VALID") {
              fail(
                `WORKFLOW_VALIDITY=VALID prohibited when active WORKFLOW_INVALIDITY receipt exists (${activeWorkflowInvalidity?.workflow_invalidity_code || "UNKNOWN"}: ${activeWorkflowInvalidity?.summary || "<missing>"})`,
              );
            }
            if (governanceVerdict === "PASS") {
              fail(
                `GOVERNANCE_VERDICT=PASS prohibited when active WORKFLOW_INVALIDITY receipt exists (${activeWorkflowInvalidity?.workflow_invalidity_code || "UNKNOWN"}: ${activeWorkflowInvalidity?.summary || "<missing>"})`,
              );
            }
          }
          if (proofCompleteness === "PROVEN" && !hasOnlyNoneList(notProvenItems)) {
            fail("PROOF_COMPLETENESS=PROVEN requires NOT_PROVEN to be exactly '- NONE'");
          }
          if (legalVerdict === "PASS" && proofCompleteness !== "PROVEN") {
            fail("LEGAL_VERDICT=PASS requires PROOF_COMPLETENESS=PROVEN");
          }
          if (topLevelVerdict === "PASS") {
            if (validationContext !== "OK") fail("Verdict=PASS requires VALIDATION_CONTEXT=OK");
            if (workflowValidity !== "VALID") fail("Verdict=PASS requires WORKFLOW_VALIDITY=VALID");
            if (scopeValidity !== "IN_SCOPE") fail("Verdict=PASS requires SCOPE_VALIDITY=IN_SCOPE");
            if (proofCompleteness !== "PROVEN") fail("Verdict=PASS requires PROOF_COMPLETENESS=PROVEN");
            if (integrationReadiness !== "READY") fail("Verdict=PASS requires INTEGRATION_READINESS=READY");
            if (domainGoalCompletion !== "COMPLETE") fail("Verdict=PASS requires DOMAIN_GOAL_COMPLETION=COMPLETE");
            if (legalVerdict !== "PASS") fail("Verdict=PASS requires LEGAL_VERDICT=PASS");
            if (environmentVerdict !== "PASS") fail("Verdict=PASS requires ENVIRONMENT_VERDICT=PASS");
            if (disposition !== "NONE") fail("Verdict=PASS requires DISPOSITION=NONE");
            const signedScopeCompatibilityForPass = validateSignedScopeCompatibilityTruth(text, {
              packetPath,
              requireReadyForPass: true,
            });
            if (signedScopeCompatibilityForPass.errors.length > 0) {
              fail(`Verdict=PASS requires signed scope compatibility truth to be PASS-ready: ${signedScopeCompatibilityForPass.errors.join("; ")}`);
            }
            for (const item of negativeProofItems) {
              if (!hasConcreteCodeReference(item) || /\.GOV\/|gov_runtime\/|TASK_BOARD|RUNTIME_STATUS|ROLE_SESSION_REGISTRY|SESSION_CONTROL|VALIDATOR_PROTOCOL|ORCHESTRATOR_PROTOCOL|COMMAND_SURFACE_REFERENCE|governance closeout|outside the signed product scope/i.test(item)) {
                fail(`Verdict=PASS requires NEGATIVE_PROOF to stay inside signed product scope with concrete product code evidence (${item})`);
              }
            }
            if (usesRiskAuditReport && usesAntiVibeRigorReport && !hasOnlyNoneList(antiVibeFindings)) {
              fail("Verdict=PASS requires ANTI_VIBE_FINDINGS to be exactly '- NONE'");
            }
            if (usesRiskAuditReport && usesAntiVibeRigorReport && !hasOnlyNoneList(signedScopeDebt)) {
              fail("Verdict=PASS requires SIGNED_SCOPE_DEBT to be exactly '- NONE'");
            }
            if (usesPrimitiveAuditReport && !hasOnlyNoneList(primitiveRetentionGaps)) {
              fail("Verdict=PASS requires PRIMITIVE_RETENTION_GAPS to be exactly '- NONE'");
            }
          }
        }
        if (usesHeuristicRigorReport && heuristicReviewVerdict === "PASS" && !hasOnlyNoneList(qualityRisks)) {
          fail("HEURISTIC_REVIEW_VERDICT=PASS requires QUALITY_RISKS to be exactly '- NONE'");
        }
        if (usesRiskAuditReport && usesAntiVibeRigorReport && heuristicReviewVerdict === "PASS" && !hasOnlyNoneList(antiVibeFindings)) {
          fail("HEURISTIC_REVIEW_VERDICT=PASS requires ANTI_VIBE_FINDINGS to be exactly '- NONE'");
        }
        if (usesRiskAuditReport && usesAntiVibeRigorReport && heuristicReviewVerdict === "PASS" && !hasOnlyNoneList(signedScopeDebt)) {
          fail("HEURISTIC_REVIEW_VERDICT=PASS requires SIGNED_SCOPE_DEBT to be exactly '- NONE'");
        }

        if (usesRiskAuditReport) {
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
            if (usesAntiVibeRigorReport && !hasOnlyNoneList(antiVibeFindings)) fail("LEGAL_VERDICT=PASS requires ANTI_VIBE_FINDINGS to be exactly '- NONE'");
            if (usesAntiVibeRigorReport && !hasOnlyNoneList(signedScopeDebt)) fail("LEGAL_VERDICT=PASS requires SIGNED_SCOPE_DEBT to be exactly '- NONE'");
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
            if (dataContractProfile === "LLM_FIRST_DATA_V1") {
              if (dataContractProof.length === 0) {
                fail("LEGAL_VERDICT=PASS requires DATA_CONTRACT_PROOF for active data contract packet");
              }
              if (!hasOnlyNoneList(dataContractGaps)) {
                fail("LEGAL_VERDICT=PASS requires DATA_CONTRACT_GAPS to be exactly '- NONE' for active data contract packet");
              }
              for (const item of dataContractProof) {
                if (!hasConcreteCodeReference(item)) {
                  fail(`LEGAL_VERDICT=PASS requires DATA_CONTRACT_PROOF entries to include concrete code or query evidence (${item})`);
                }
              }
            }
            if (usesPrimitiveAuditReport) {
              if (!hasOnlyNoneList(primitiveRetentionGaps)) {
                fail("LEGAL_VERDICT=PASS requires PRIMITIVE_RETENTION_GAPS to be exactly '- NONE'");
              }
              if (lacksConcreteListEvidence(primitiveRetentionProof)) {
                fail("LEGAL_VERDICT=PASS requires PRIMITIVE_RETENTION_PROOF entries to include concrete code or symbol evidence");
              }
              if (lacksConcreteListEvidence(sharedSurfaceInteractionChecks)) {
                fail("LEGAL_VERDICT=PASS requires SHARED_SURFACE_INTERACTION_CHECKS entries to include concrete code or symbol evidence");
              }
              if (lacksConcreteListEvidence(currentMainInteractionChecks)) {
                fail("LEGAL_VERDICT=PASS requires CURRENT_MAIN_INTERACTION_CHECKS entries to include concrete code or symbol evidence");
              }
              if (packetRiskTierRank >= riskTierRank("MEDIUM") && (primitiveRetentionProof.length === 0 || hasOnlyNoneList(primitiveRetentionProof))) {
                fail(`LEGAL_VERDICT=PASS requires non-empty PRIMITIVE_RETENTION_PROOF for packet RISK_TIER=${packetRiskTier}`);
              }
              if (packetRiskTierRank >= riskTierRank("MEDIUM") && (sharedSurfaceInteractionChecks.length === 0 || hasOnlyNoneList(sharedSurfaceInteractionChecks))) {
                fail(`LEGAL_VERDICT=PASS requires non-empty SHARED_SURFACE_INTERACTION_CHECKS for packet RISK_TIER=${packetRiskTier}`);
              }
              if (packetRiskTierRank >= riskTierRank("MEDIUM") && (currentMainInteractionChecks.length === 0 || hasOnlyNoneList(currentMainInteractionChecks))) {
                fail(`LEGAL_VERDICT=PASS requires non-empty CURRENT_MAIN_INTERACTION_CHECKS for packet RISK_TIER=${packetRiskTier}`);
              }
              if (sharedSurfaceRisk === "YES" && (sharedSurfaceInteractionChecks.length === 0 || hasOnlyNoneList(sharedSurfaceInteractionChecks))) {
                fail("LEGAL_VERDICT=PASS requires non-empty SHARED_SURFACE_INTERACTION_CHECKS when SHARED_SURFACE_RISK=YES");
              }
              if (currentMainCompatibilityStatus === "PASS" && (currentMainInteractionChecks.length === 0 || hasOnlyNoneList(currentMainInteractionChecks))) {
                fail("LEGAL_VERDICT=PASS requires non-empty CURRENT_MAIN_INTERACTION_CHECKS when CURRENT_MAIN_COMPATIBILITY_STATUS=PASS");
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

        if (computedPolicy.applicable && !computedPolicyOutcomeAllowsClosure(computedPolicy) && !abandonedClosure) {
          const details = [
            ...computedPolicy.issues.fail,
            ...computedPolicy.issues.blocked,
            ...computedPolicy.issues.reviewRequired,
          ].map((item) => `${item.code}: ${item.message}`);
          fail(`computed policy gate outcome ${computedPolicy.outcome}${details.length > 0 ? ` (${details.join("; ")})` : ""}`);
        }
      }
    }

    return {
      ok: true,
      exitCode: 0,
      message: `validator-packet-complete: PASS - ${wpId} has required fields.`,
    };
  } catch (error) {
    return {
      ok: false,
      exitCode: 1,
      message: `validator-packet-complete: FAIL - ${error?.message || String(error || "unknown error")}`,
    };
  }
}

export function formatValidatorPacketCompleteResult(result = {}) {
  return `${String(result.message || "").trim()}\n`;
}

function normalizeAuthorityRole(value) {
  return String(value || "").trim().toUpperCase().replace(/[\s-]+/g, "_");
}

function authorityRoleToBranch(authorityRole) {
  const normalized = normalizeAuthorityRole(authorityRole);
  if (normalized === "ORCHESTRATOR") return "gov_kernel";
  if (normalized === "VALIDATOR" || normalized === "WP_VALIDATOR" || normalized === "INTEGRATION_VALIDATOR") {
    return "gov_kernel";
  }
  if (normalized === "OPERATOR") return "user_ilja";
  return "gov_kernel";
}

function ensureValidatorGateStateDir() {
  ensureValidatorGateDir();
}

function validatorGateStateFilePath(wpId) {
  return resolveValidatorGatePath(wpId);
}

function normalizeValidatorGateState(raw) {
  const validationSessions =
    raw?.validation_sessions && typeof raw.validation_sessions === "object"
      ? raw.validation_sessions
      : {};
  const committedValidationEvidence =
    raw?.committed_validation_evidence && typeof raw.committed_validation_evidence === "object"
      ? raw.committed_validation_evidence
      : {};

  return {
    validation_sessions: validationSessions,
    archived_sessions: Array.isArray(raw?.archived_sessions) ? raw.archived_sessions : [],
    committed_validation_evidence: committedValidationEvidence,
  };
}

function loadValidatorGateState(wpId) {
  ensureValidatorGateStateDir();
  const filePath = repoPathAbs(resolveValidatorGatePath(wpId));
  if (!fs.existsSync(filePath)) return normalizeValidatorGateState({});
  return normalizeValidatorGateState(JSON.parse(fs.readFileSync(filePath, "utf8")));
}

function saveValidatorGateState(wpId, state) {
  ensureValidatorGateStateDir();
  fs.writeFileSync(
    repoPathAbs(validatorGateStateFilePath(wpId)),
    `${JSON.stringify(normalizeValidatorGateState(state), null, 2)}\n`,
    "utf8",
  );
}

function runCommandInWorktree(worktreeAbs, command, args) {
  const result = spawnSync(command, args, {
    cwd: worktreeAbs,
    encoding: "utf8",
    stdio: ["ignore", "pipe", "pipe"],
  });
  return {
    code: typeof result.status === "number" ? result.status : 1,
    output: `${result.stdout || ""}${result.stderr || ""}`.trim(),
  };
}

function gitInWorktree(worktreeAbs, args) {
  const result = runCommandInWorktree(worktreeAbs, "git", args);
  if (result.code !== 0) {
    throw new Error(result.output || `git ${args.join(" ")} failed`);
  }
  return result.output.trim();
}

function extractPreWorkErrors(output) {
  const lines = String(output || "").split(/\r?\n/);
  const errors = [];
  let inErrors = false;
  for (const line of lines) {
    if (/^\s*Errors:\s*$/i.test(line)) {
      inErrors = true;
      continue;
    }
    if (!inErrors) continue;
    if (/^\s*(Warnings:|Fix these issues before)/i.test(line.trim())) break;
    const itemMatch = line.match(/^\s*\d+\.\s+(.*)$/);
    if (itemMatch) {
      errors.push(itemMatch[1].trim());
      continue;
    }
    if (errors.length > 0 && /^\s{2,}\S/.test(line)) {
      errors[errors.length - 1] = `${errors[errors.length - 1]} ${line.trim()}`.trim();
    }
  }
  return errors;
}

function preWorkFailureIsNonBlockingForCommittedTarget(output) {
  const errors = extractPreWorkErrors(output);
  if (errors.length === 0) return false;
  return errors.every((entry) =>
    /^Branch-local out-of-scope edits detected before work starts:/i.test(String(entry || "").trim())
  );
}

function validatorHandoffFailure(kind, message, details = [], exitCode = 1) {
  if (exitCode === 1) {
    captureCheckFinding({ check: "validator-handoff-check", finding: `${kind}: ${message}` });
  }
  return {
    ok: false,
    exitCode,
    kind,
    message,
    details,
  };
}

function repoRelativeDisplayPath(repoRoot, absPath) {
  return String(path.relative(repoRoot, absPath) || "").replace(/\\/g, "/");
}

function selectCommittedValidationTarget(worktreeAbs, packetContent, { rev = "", range = "" } = {}) {
  if (range) {
    return {
      mode: "COMMITTED_RANGE",
      args: ["--range", range],
      summary: range,
      targetHeadSha: range.split("..")[1].trim(),
    };
  }
  if (rev) {
    return {
      mode: "COMMITTED_REV",
      args: ["--rev", rev],
      summary: rev,
      targetHeadSha: rev,
    };
  }

  const mergeBaseSha = parseMergeBaseSha(packetContent);
  if (mergeBaseSha) {
    return {
      mode: "COMMITTED_RANGE",
      args: ["--range", `${mergeBaseSha}..HEAD`],
      summary: `${mergeBaseSha}..HEAD`,
      targetHeadSha: "HEAD",
    };
  }

  return {
    mode: "COMMITTED_REV",
    args: ["--rev", "HEAD"],
    summary: "HEAD",
    targetHeadSha: "HEAD",
  };
}

// RGF-184: detect zero-execution in test proof output.
// A test command that exits 0 but matched zero tests is a false green.
// Returns { zeroExecution: boolean, detail: string }.
function detectZeroExecutionEvidence(output) {
  const text = String(output || "");
  // Rust/Cargo: "running 0 tests" or "0 passed; 0 failed; 0 ignored"
  if (/running 0 tests/i.test(text)) {
    return { zeroExecution: true, detail: "cargo test matched 0 tests (running 0 tests)" };
  }
  if (/test result: ok\.\s+0 passed/i.test(text) && !/[1-9]\d* passed/i.test(text)) {
    return { zeroExecution: true, detail: "cargo test result: 0 passed" };
  }
  // Node/Jest: "Tests: 0 passed, 0 total" or "No tests found"
  if (/no tests found/i.test(text)) {
    return { zeroExecution: true, detail: "test runner found no tests" };
  }
  if (/Tests:\s+0 passed,\s+0 total/i.test(text)) {
    return { zeroExecution: true, detail: "jest: 0 passed, 0 total" };
  }
  // Generic: "0 passing" (mocha-style)
  if (/\b0 passing\b/i.test(text) && !/[1-9]\d* passing/i.test(text)) {
    return { zeroExecution: true, detail: "test runner: 0 passing" };
  }
  return { zeroExecution: false, detail: "" };
}

function persistCommittedValidationEvidence(wpId, evidence) {
  const state = loadValidatorGateState(wpId);
  state.committed_validation_evidence[wpId] = recordCommittedValidationRun(
    state.committed_validation_evidence[wpId],
    evidence,
  );
  saveValidatorGateState(wpId, state);
  return state.committed_validation_evidence[wpId];
}

const PHASE_CHECK_SCRIPT = repoPathAbs(path.join(GOV_ROOT_REPO_REL, "roles_shared", "checks", "phase-check.mjs"));
const CARGO_CLEAN_ARGS = [
  "clean",
  "-p",
  "handshake_core",
  "--manifest-path",
  "src/backend/handshake_core/Cargo.toml",
  "--target-dir",
  "../Handshake Artifacts/handshake-cargo-target",
];

export function buildValidatorHandoffCheckResult({
  wpId = "",
  rev = "",
  range = "",
} = {}) {
  const normalizedWpId = String(wpId || "").trim();
  const normalizedRev = String(rev || "").trim();
  const normalizedRange = String(range || "").trim();
  if (!normalizedWpId || !/^WP-[A-Za-z0-9][A-Za-z0-9._-]*$/.test(normalizedWpId)) {
    return validatorHandoffFailure(
      "FAIL",
      `Usage: ${buildPhaseCheckCommand({ phase: "HANDOFF", wpId: "WP-{ID}", role: "WP_VALIDATOR" })} [--rev <git-rev> | --range <base>..<head>]`,
      [],
      1,
    );
  }
  if (normalizedRev && normalizedRange) {
    return validatorHandoffFailure("FAIL", "Use either --rev or --range, not both.", [], 1);
  }

  try {
    const gitContext = currentGitContext();
    const repoRoot = gitContext.topLevel || REPO_ROOT;
    if (!packetExists(normalizedWpId)) {
      return validatorHandoffFailure("FAIL", "Task packet not found", [packetPath(normalizedWpId)], 1);
    }

    const packetContentForContext = loadPacket(normalizedWpId);
    const workflowLane = parseClaimField(packetContentForContext, "WORKFLOW_LANE");
    const workflowAuthority = parseClaimField(packetContentForContext, "WORKFLOW_AUTHORITY")
      || (String(workflowLane || "").trim().toUpperCase() === "ORCHESTRATOR_MANAGED" ? "ORCHESTRATOR" : "ORCHESTRATOR");
    const expectedGovernanceBranch = authorityRoleToBranch(workflowAuthority);
    const validatorGovernanceStateForContext = evaluateValidatorPacketGovernanceState({
      wpId: normalizedWpId,
      packetPath: packetPath(normalizedWpId),
      packetContent: packetContentForContext,
    });
    if (!validatorGovernanceStateForContext.allowValidationResume) {
      return validatorHandoffFailure("FAIL", "Committed handoff validation is blocked for this packet", [
        validatorGovernanceStateForContext.message,
        `computed_policy_outcome=${validatorGovernanceStateForContext.computedPolicy.outcome}`,
        `computed_policy_applicability=${validatorGovernanceStateForContext.computedPolicy.applicability_reason || "APPLICABLE"}`,
      ], 1);
    }

    const logs = loadOrchestratorGateLogs();
    const prepareEntry = lastGateLog(logs, normalizedWpId, "PREPARE");
    if (!prepareEntry) {
      if (gitContext.branch !== expectedGovernanceBranch) {
        return validatorHandoffFailure("CONTEXT_MISMATCH", "PREPARE gate entry is unavailable in this checkout", [
          `current_branch=${gitContext.branch || "<detached>"}`,
          `expected_governance_branch=${expectedGovernanceBranch}`,
          `rerun=${buildPhaseCheckCommand({ phase: "HANDOFF", wpId: normalizedWpId, role: "WP_VALIDATOR" })}`,
        ], 2);
      }
      return validatorHandoffFailure("FAIL", "PREPARE gate entry is missing", [`Run: just orchestrator-next ${normalizedWpId}`], 1);
    }

    const syncState = preparedWorktreeSyncState(normalizedWpId, prepareEntry, repoRoot);
    const worktreeAbs = resolvePrepareWorktreeAbs(prepareEntry, repoRoot);
    if (!worktreeAbs || !fs.existsSync(worktreeAbs)) {
      return validatorHandoffFailure("CONTEXT_MISMATCH", "Assigned PREPARE worktree is unavailable in this environment", [
        `recorded_worktree_dir=${String(prepareEntry.worktree_dir || "<missing>")}`,
        `current_branch=${gitContext.branch || "<detached>"}`,
        "This blocks committed handoff validation in this checkout but does not by itself prove a WP failure.",
      ], 2);
    }
    if (!String(syncState.actualBranch || "").trim()) {
      return validatorHandoffFailure("CONTEXT_MISMATCH", "Assigned PREPARE worktree branch could not be resolved", [
        worktreeAbs,
        "The committed handoff source cannot be inspected from this environment.",
      ], 2);
    }
    if (
      String(syncState.expectedBranch || "").trim()
      && String(syncState.actualBranch || "").trim() !== String(syncState.expectedBranch || "").trim()
    ) {
      return validatorHandoffFailure("FAIL", "Assigned PREPARE worktree branch does not match PREPARE", [
        `expected=${syncState.expectedBranch}`,
        `actual=${syncState.actualBranch}`,
      ], 1);
    }

    const nonBlockingSyncWarnings = (syncState.issues || []).filter((issue) =>
      !/does not exist|branch mismatch|PREPARE is missing worktree_dir|could not be resolved/i.test(issue),
    );

    const resolvedPacket = resolveWorkPacketPath(normalizedWpId);
    const packetPathRel = resolvedPacket?.packetPath || path.join(WORK_PACKET_STORAGE_ROOT_REPO_REL, `${normalizedWpId}.md`);
    const worktreePacketPath = path.join(worktreeAbs, packetPathRel);
    const packetContent = fs.existsSync(worktreePacketPath)
      ? fs.readFileSync(worktreePacketPath, "utf8")
      : fs.readFileSync(repoPathAbs(packetPathRel), "utf8");

    const communicationHealth = runCommandInWorktree(repoRoot, process.execPath, [
      repoPathAbs(path.join(GOV_ROOT_REPO_REL, "roles_shared", "checks", "wp-communication-health-check.mjs")),
      normalizedWpId,
      "HANDOFF",
    ]);
    if (communicationHealth.code !== 0) {
      return validatorHandoffFailure("FAIL", "Direct review communication contract is not ready for validator handoff", [
        ...communicationHealth.output.split(/\r?\n/).filter(Boolean),
      ], 1);
    }

    const packetComplete = buildValidatorPacketCompleteResult({
      wpId: normalizedWpId,
    });
    if (!packetComplete.ok) {
      return validatorHandoffFailure("FAIL", "Packet closure hygiene is incomplete for validator handoff", [
        ...formatValidatorPacketCompleteResult(packetComplete).split(/\r?\n/).filter(Boolean),
      ], 1);
    }

    const committedTarget = selectCommittedValidationTarget(worktreeAbs, packetContent, {
      rev: normalizedRev,
      range: normalizedRange,
    });
    let targetHeadSha = committedTarget.targetHeadSha;
    try {
      targetHeadSha = gitInWorktree(worktreeAbs, ["rev-parse", committedTarget.targetHeadSha]);
    } catch {
      // Keep the user-specified ref literal if rev-parse fails.
    }

    const preWork = runCommandInWorktree(worktreeAbs, process.execPath, [
      PHASE_CHECK_SCRIPT,
      "STARTUP",
      normalizedWpId,
      "CODER",
      "--verbose",
    ]);
    const cargoClean = runCommandInWorktree(worktreeAbs, "cargo", CARGO_CLEAN_ARGS);
    const postWork = runCommandInWorktree(worktreeAbs, process.execPath, [
      PHASE_CHECK_SCRIPT,
      "HANDOFF",
      normalizedWpId,
      "CODER",
      ...committedTarget.args,
    ]);
    const cargoCleanStatus = cargoClean.code === 0 ? "PASS" : "FAIL";
    const preWorkNonBlockingForCommittedTarget =
      preWork.code !== 0 && preWorkFailureIsNonBlockingForCommittedTarget(preWork.output);
    const livePrepareWorktreeStatus = preWork.code === 0 && cargoClean.code === 0 && postWork.code === 0 ? "PASS" : "FAIL";
    const committedTargetStatus =
      cargoClean.code === 0
      && postWork.code === 0
      && (preWork.code === 0 || preWorkNonBlockingForCommittedTarget)
        ? "PASS"
        : "FAIL";

    const evidence = {
      wp_id: normalizedWpId,
      status: committedTargetStatus,
      live_prepare_worktree_status: livePrepareWorktreeStatus,
      committed_target_status: committedTargetStatus,
      validated_at: new Date().toISOString(),
      source_truth: "PREPARE_WORKTREE",
      prepare_branch: String(prepareEntry.branch || "").trim(),
      prepare_worktree_dir: String(prepareEntry.worktree_dir || "").trim(),
      prepare_worktree_sync_warnings: nonBlockingSyncWarnings,
      committed_validation_mode: committedTarget.mode,
      committed_validation_target: committedTarget.summary,
      target_head_sha: targetHeadSha,
      pre_work_status: preWork.code === 0 ? "PASS" : "FAIL",
      cargo_clean_required: true,
      cargo_clean_status: cargoCleanStatus,
      post_work_status: postWork.code === 0 ? "PASS" : "FAIL",
      pre_work_command: buildPhaseCheckCommand({
        phase: "STARTUP",
        wpId: normalizedWpId,
        role: "CODER",
        args: ["--verbose"],
      }),
      cargo_clean_command: `cargo ${CARGO_CLEAN_ARGS.join(" ")}`,
      post_work_command: buildPhaseCheckCommand({
        phase: "HANDOFF",
        wpId: normalizedWpId,
        role: "CODER",
        args: committedTarget.args,
      }),
      pre_work_output: preWork.output,
      cargo_clean_output: cargoClean.output,
      post_work_output: postWork.output,
    };

    // RGF-184: reject PASS when proof commands matched zero tests (false green).
    // A test command exiting 0 with zero executed tests is not valid evidence.
    if (evidence.status === "PASS") {
      const postWorkZero = detectZeroExecutionEvidence(postWork.output);
      if (postWorkZero.zeroExecution) {
        evidence.status = "FAIL";
        evidence.committed_target_status = "FAIL";
        evidence.zero_execution_detected = true;
        evidence.zero_execution_detail = postWorkZero.detail;
      }
    }

    const persistedEvidence = persistCommittedValidationEvidence(normalizedWpId, evidence);
    const durableCommittedProof = committedEvidenceForCloseout(persistedEvidence);
    const livePrepareHealth = livePrepareWorktreeHealthEvidence(persistedEvidence);

    if (evidence.status !== "PASS") {
      const extraDetails = [];
      if (durableCommittedProof?.status === "PASS") {
        extraDetails.push(`durable_committed_target_head_sha=${durableCommittedProof.target_head_sha}`);
        extraDetails.push(`durable_committed_proof_validated_at=${durableCommittedProof.validated_at}`);
      }
      return validatorHandoffFailure("FAIL", "Committed handoff validation failed", [
        `prepare_worktree_dir=${evidence.prepare_worktree_dir}`,
        `committed_validation_target=${evidence.committed_validation_target}`,
        `pre_work_status=${evidence.pre_work_status}`,
        `cargo_clean_status=${evidence.cargo_clean_status}`,
        `post_work_status=${evidence.post_work_status}`,
        ...(livePrepareHealth ? [`live_prepare_worktree_status=${livePrepareHealth.status}`] : []),
        ...extraDetails,
        `evidence_file=${validatorGateStateFilePath(normalizedWpId).replace(/\\/g, "/")}`,
      ], 1);
    }

    return {
      ok: true,
      exitCode: 0,
      kind: "PASS",
      message: "",
      details: [
        `wp_id=${normalizedWpId}`,
        `prepare_worktree_dir=${evidence.prepare_worktree_dir}`,
        `committed_validation_mode=${evidence.committed_validation_mode}`,
        `committed_validation_target=${evidence.committed_validation_target}`,
        `target_head_sha=${evidence.target_head_sha}`,
        `live_prepare_worktree_status=${livePrepareHealth?.status || evidence.status}`,
        `durable_committed_proof_status=${durableCommittedProof?.status || evidence.status}`,
        ...(preWorkNonBlockingForCommittedTarget ? ["non_blocking_pre_work_failure=BRANCH_LOCAL_OUT_OF_SCOPE_EDITS"] : []),
        `evidence_file=${validatorGateStateFilePath(normalizedWpId).replace(/\\/g, "/")}`,
        ...(nonBlockingSyncWarnings.length > 0
          ? [`sync_warnings=${formatBoundedItemList(nonBlockingSyncWarnings, { noun: "warning" })}`]
          : []),
      ],
    };
  } catch (error) {
    return validatorHandoffFailure("FAIL", error?.message || String(error || ""), [], 1);
  }
}

export function formatValidatorHandoffCheckResult(result = {}) {
  if (result.ok) {
    return [
      "[VALIDATOR_HANDOFF_CHECK] PASS",
      ...(result.details || []).map((detail) => `  ${detail}`),
      "",
    ].join("\n");
  }

  return [
    `[VALIDATOR_HANDOFF_CHECK] ${result.kind || "FAIL"}: ${result.message || "Unknown failure"}`,
    ...((result.details || []).map((detail) => `  - ${detail}`)),
    "",
  ].join("\n");
}
