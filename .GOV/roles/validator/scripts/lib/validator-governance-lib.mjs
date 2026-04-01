import fs from "node:fs";
import path from "node:path";
import { evaluateComputedPolicyGateFromPacketText } from "../../../../roles_shared/scripts/lib/computed-policy-gate-lib.mjs";
import { parseClaimField } from "../../../../roles_shared/scripts/lib/role-resume-utils.mjs";
import { GOV_ROOT_ABS, normalizePath, REPO_ROOT, repoPathAbs } from "../../../../roles_shared/scripts/lib/runtime-paths.mjs";
import { evaluateWpCommunicationHealth, deriveLatestValidatorAssessment } from "../../../../roles_shared/scripts/lib/wp-communication-health-lib.mjs";
import { parseJsonFile, parseJsonlFile } from "../../../../roles_shared/scripts/lib/wp-communications-lib.mjs";
import { loadSessionRegistry } from "../../../../roles_shared/scripts/session/session-registry-lib.mjs";
import {
  defaultIntegrationValidatorBranch,
  defaultIntegrationValidatorWorktreeDir,
  defaultWpValidatorBranch,
  defaultWpValidatorWorktreeDir,
} from "../../../../roles_shared/scripts/session/session-policy.mjs";

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

  if (!["WP_VALIDATOR", "INTEGRATION_VALIDATOR"].includes(normalizedRole) || !communicationApplicable) {
    return {
      ready: false,
      blockedByRoute: false,
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
      nextExpectedActor,
      waitingOn,
      latestAssessment,
      message: validatorReadyMessage(normalizedRole, waitingOn, communicationState),
    };
  }

  if (nextExpectedActor) {
    return {
      ready: false,
      blockedByRoute: true,
      nextExpectedActor,
      waitingOn,
      latestAssessment,
      message: blockedValidatorMessage(normalizedRole, nextExpectedActor, waitingOn, communicationState),
    };
  }

  return {
    ready: false,
    blockedByRoute: false,
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
      && actorSessionId
      && authority.integrationValidatorOfRecord !== actorSessionId
    ) {
      issues.push(
        `Integration validator of record mismatch (packet=${authority.integrationValidatorOfRecord}, current=${actorSessionId}).`,
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
} = {}) {
  const role = normalizeValidatorRole(actorRole);
  if (role === "INTEGRATION_VALIDATOR") {
    const session = actorSessionId || "<integration-validator-session>";
    return [
      `just integration-validator-context-brief ${wpId}`,
      `just check-notifications ${wpId} INTEGRATION_VALIDATOR`,
      `just ack-notifications ${wpId} INTEGRATION_VALIDATOR ${session}`,
      `just validator-packet-complete ${wpId}`,
      `just wp-communication-health-check ${wpId} VERDICT`,
      `just validator-handoff-check ${wpId}`,
      `just integration-validator-closeout-check ${wpId}`,
    ];
  }
  if (role === "WP_VALIDATOR") {
    const session = actorSessionId || "<wp-validator-session>";
    return [
      `just check-notifications ${wpId} WP_VALIDATOR`,
      `just ack-notifications ${wpId} WP_VALIDATOR ${session}`,
      `just validator-packet-complete ${wpId}`,
      `just wp-communication-health-check ${wpId} HANDOFF`,
      `just validator-handoff-check ${wpId}`,
    ];
  }
  return [
    `just validator-packet-complete ${wpId}`,
    `just validator-handoff-check ${wpId}`,
    postWorkCommand,
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
