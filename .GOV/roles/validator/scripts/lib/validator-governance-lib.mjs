import path from "node:path";
import { evaluateComputedPolicyGateFromPacketText } from "../../../../roles_shared/scripts/lib/computed-policy-gate-lib.mjs";
import { parseClaimField } from "../../../../roles_shared/scripts/lib/role-resume-utils.mjs";
import { normalizePath } from "../../../../roles_shared/scripts/lib/runtime-paths.mjs";
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

export function normalizeValidatorRole(value) {
  const normalized = String(value || "").trim().toUpperCase().replace(/[\s-]+/g, "_");
  if (!normalized) return "";
  if (normalized === "VALIDATOR" || normalized === "CLASSICAL") return "CLASSICAL_VALIDATOR";
  if (normalized === "WPVALIDATOR") return "WP_VALIDATOR";
  if (normalized === "INTEGRATIONVALIDATOR") return "INTEGRATION_VALIDATOR";
  return normalized;
}

function currentWorktreeRepoRelative(repoRoot, gitContext = {}) {
  const root = path.resolve(repoRoot || process.cwd());
  const topLevel = String(gitContext?.topLevel || "").trim();
  if (!topLevel) return "";
  return normalizePath(path.relative(root, path.resolve(topLevel))) || ".";
}

function currentBranchName(gitContext = {}) {
  return String(gitContext?.branch || "").trim();
}

function sameWorktreePath(left, right) {
  return normalizePath(left).toLowerCase() === normalizePath(right).toLowerCase();
}

function matchRegistrySessionToGitContext(session, repoRoot, gitContext = {}) {
  const branch = currentBranchName(gitContext);
  const worktreeDir = currentWorktreeRepoRelative(repoRoot, gitContext);
  return (
    String(session?.local_branch || "").trim() === branch
    && sameWorktreePath(String(session?.local_worktree_dir || "").trim(), worktreeDir)
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

export function resolveValidatorActorContext({
  repoRoot = process.cwd(),
  wpId = "",
  packetContent = "",
  gitContext = {},
  registrySessions = null,
} = {}) {
  const root = path.resolve(repoRoot || process.cwd());
  const branch = currentBranchName(gitContext);
  const worktreeDir = currentWorktreeRepoRelative(root, gitContext);
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
      actorBranch: branch,
      actorWorktreeDir: worktreeDir,
      source: "SESSION_REGISTRY",
      authority,
    };
  }

  if (
    branch === defaultIntegrationValidatorBranch(wpId)
    && sameWorktreePath(worktreeDir, defaultIntegrationValidatorWorktreeDir(wpId))
  ) {
    return {
      actorRole: "INTEGRATION_VALIDATOR",
      actorSessionKey: "",
      actorSessionId: "",
      actorThreadId: "",
      actorBranch: branch,
      actorWorktreeDir: worktreeDir,
      source: "WORKTREE_POLICY",
      authority,
    };
  }

  if (
    branch === defaultWpValidatorBranch(wpId)
    && sameWorktreePath(worktreeDir, defaultWpValidatorWorktreeDir(wpId))
  ) {
    return {
      actorRole: "WP_VALIDATOR",
      actorSessionKey: "",
      actorSessionId: "",
      actorThreadId: "",
      actorBranch: branch,
      actorWorktreeDir: worktreeDir,
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
      `just wp-communication-health-check ${wpId} HANDOFF`,
      `just validator-handoff-check ${wpId}`,
    ];
  }
  return [
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
