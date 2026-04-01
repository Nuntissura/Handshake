import fs from "node:fs";
import { packetPath, parseCurrentWpStatus, parseStatus, taskBoardStatus } from "../../../../roles_shared/scripts/lib/role-resume-utils.mjs";
import { evaluateWpDeclaredTopology } from "../../../../roles_shared/scripts/lib/wp-declared-topology-lib.mjs";
import { REPO_ROOT } from "../../../../roles_shared/scripts/lib/runtime-paths.mjs";
import {
  evaluateIntegrationValidatorCloseoutState,
  latestCloseoutSyncEvent,
} from "./integration-validator-closeout-lib.mjs";
import {
  evaluateValidatorPacketGovernanceState,
  readValidatorAuthority,
  resolveValidatorActorContext,
} from "./validator-governance-lib.mjs";
import {
  committedEvidenceForCloseout,
  livePrepareWorktreeHealthEvidence,
} from "./committed-validation-evidence-lib.mjs";

const COMMAND_SURFACE_PATH = ".GOV/roles_shared/docs/COMMAND_SURFACE_REFERENCE.md";

function normalizePath(value) {
  return String(value || "").trim().replace(/\\/g, "/");
}

function normalizeStatus(value, fallback = "<none>") {
  const normalized = String(value || "").trim();
  return normalized || fallback;
}

function pushUnique(target, value) {
  const normalized = String(value || "").trim();
  if (!normalized || target.includes(normalized)) return;
  target.push(normalized);
}

function formatBoundedList(items = [], maxItems = 4) {
  const normalized = (items || []).map((item) => String(item || "").trim()).filter(Boolean);
  if (normalized.length === 0) return ["<none>"];
  if (normalized.length <= maxItems) return normalized;
  return [...normalized.slice(0, maxItems), `... (${normalized.length - maxItems} more)`];
}

function requiredCommandsForState({ wpId, actorContext, governanceBlocked }) {
  if (governanceBlocked) {
    return [
      `just validator-policy-gate ${wpId}`,
      `just validator-packet-complete ${wpId}`,
    ];
  }

  return [
    `just integration-validator-context-brief ${wpId}`,
    `just check-notifications ${wpId} INTEGRATION_VALIDATOR`,
    `just ack-notifications ${wpId} INTEGRATION_VALIDATOR ${actorContext.actorSessionId || "<integration-validator-session>"}`,
    `just wp-communication-health-check ${wpId} VERDICT`,
    `just validator-handoff-check ${wpId}`,
    `just integration-validator-closeout-check ${wpId}`,
  ];
}

export function buildIntegrationValidatorContextBrief({
  repoRoot = REPO_ROOT,
  wpId = "",
  packetContent = "",
  packetPathValueOverride = "",
  gitContext = {},
  gateState = null,
  committedEvidence = null,
  requests = null,
  results = null,
  registrySessions = null,
  brokerState = null,
  gitRunner = null,
  worktreeExists = fs.existsSync,
  fileExists = fs.existsSync,
  declaredTopologyEvaluation = null,
  gateStatePath = "",
} = {}) {
  if (!String(wpId || "").trim()) {
    throw new Error("WP_ID is required for integration-validator context brief generation.");
  }

  const packetPathValue = normalizePath(packetPathValueOverride || packetPath(wpId));
  const packetStatus = normalizeStatus(parseStatus(packetContent), "<missing>");
  const currentWpStatus = normalizeStatus(parseCurrentWpStatus(packetContent), "<empty>");
  const boardStatus = normalizeStatus(taskBoardStatus(wpId), "<none>");
  const authority = readValidatorAuthority(packetContent);
  const actorContext = resolveValidatorActorContext({
    repoRoot,
    wpId,
    packetContent,
    gitContext,
    registrySessions,
  });
  const governanceState = evaluateValidatorPacketGovernanceState({
    wpId,
    packetPath: packetPathValue,
    packetContent,
    currentWpStatus,
    taskBoardStatus: boardStatus,
    actorContext,
  });
  const topologyEvaluation = declaredTopologyEvaluation ?? evaluateWpDeclaredTopology({
    repoRoot,
    wpId,
    packetContent,
  });
  const closeoutEvaluation = evaluateIntegrationValidatorCloseoutState({
    repoRoot,
    wpId,
    packetContent,
    actorContext,
    committedEvidence,
    requests,
    results,
    registrySessions,
    brokerState,
    gitRunner,
    worktreeExists,
    fileExists,
  });
  const durableCommittedProof = committedEvidenceForCloseout(committedEvidence);
  const livePrepareHealth = livePrepareWorktreeHealthEvidence(committedEvidence);
  const latestCloseoutEvent = latestCloseoutSyncEvent(gateState, wpId);

  const contextStatus = !governanceState.allowValidationResume
    ? "GOVERNANCE_BLOCKED"
    : actorContext.actorRole !== "INTEGRATION_VALIDATOR"
      ? "CONTEXT_MISMATCH"
      : "OK";
  const closeoutReadiness = closeoutEvaluation.ok ? "READY" : "NOT_READY";
  const contextNotes = [];

  if (!governanceState.allowValidationResume) {
    pushUnique(contextNotes, governanceState.message);
  }
  if (actorContext.actorRole !== "INTEGRATION_VALIDATOR") {
    pushUnique(
      contextNotes,
      `Current lane resolved to ${actorContext.actorRole || "UNKNOWN"} from ${actorContext.source || "UNRESOLVED"}; final merge-ready authority belongs to INTEGRATION_VALIDATOR.`,
    );
  } else if (actorContext.source !== "SESSION_REGISTRY") {
    pushUnique(
      contextNotes,
      `Integration-validator lane is present but not yet proven by governed session identity; current source is ${actorContext.source || "UNRESOLVED"}.`,
    );
  }
  if (!topologyEvaluation.ok && topologyEvaluation.issues.length > 0) {
    pushUnique(contextNotes, `Declared topology issue: ${topologyEvaluation.issues[0]}`);
  }
  if (!closeoutEvaluation.ok && closeoutEvaluation.issues.length > 0) {
    pushUnique(contextNotes, `Closeout blocker: ${closeoutEvaluation.issues[0]}`);
  }

  return {
    schema_id: "hsk.integration_validator_context_brief@1",
    schema_version: "integration_validator_context_brief_v1",
    wp_id: wpId,
    context_status: contextStatus,
    closeout_readiness: closeoutReadiness,
    workflow_lane: authority.workflowLane || "<missing>",
    packet_path: packetPathValue,
    packet_status: packetStatus,
    current_wp_status: currentWpStatus,
    task_board_status: boardStatus,
    command_surface_path: COMMAND_SURFACE_PATH,
    minimal_live_read_set: [
      "startup output",
      "active packet",
      "active WP thread/notifications",
      COMMAND_SURFACE_PATH,
    ],
    anti_rediscovery_rule:
      "Do not rebuild final-lane branch/worktree/authority truth by rereading large protocols or rediscovering commands once this bundle is available.",
    startup_sequence: [
      "just validator-startup",
      `just validator-next ${wpId}`,
      `just integration-validator-context-brief ${wpId}`,
    ],
    authority: {
      technical_authority: authority.technicalAuthority || "<missing>",
      merge_authority: authority.mergeAuthority || "<missing>",
      integration_validator_of_record: authority.integrationValidatorOfRecord || "<unassigned>",
      wp_validator_of_record: authority.wpValidatorOfRecord || "<unassigned>",
    },
    actor_context: {
      role: actorContext.actorRole || "UNKNOWN",
      source: actorContext.source || "UNRESOLVED",
      session_key: actorContext.actorSessionKey || "<missing>",
      session_id: actorContext.actorSessionId || "<missing>",
      thread_id: actorContext.actorThreadId || "<missing>",
      branch: actorContext.actorBranch || "<unknown>",
      worktree_dir: actorContext.actorWorktreeDir || "<unknown>",
    },
    governance_root: {
      live_root: normalizeStatus(closeoutEvaluation.topology?.liveGovernanceRootAbs, "<missing>"),
      local_main_backup_root: normalizeStatus(closeoutEvaluation.topology?.localMainGovernanceAbs, "<missing>"),
      mode: closeoutEvaluation.topology?.liveGovernanceRootAbs
        && closeoutEvaluation.topology?.localMainGovernanceAbs
        && normalizePath(closeoutEvaluation.topology.liveGovernanceRootAbs) === normalizePath(closeoutEvaluation.topology.localMainGovernanceAbs)
          ? "MAIN_BACKUP"
          : "KERNEL",
    },
    declared_topology: {
      status: topologyEvaluation.ok ? "PASS" : "FAIL",
      issues: topologyEvaluation.issues || [],
    },
    committed_handoff: {
      status: normalizeStatus(durableCommittedProof?.status, "NONE"),
      live_prepare_worktree_status: normalizeStatus(livePrepareHealth?.status, "NONE"),
      committed_validation_mode: normalizeStatus(durableCommittedProof?.committed_validation_mode, "NONE"),
      committed_validation_target: normalizeStatus(durableCommittedProof?.committed_validation_target, "NONE"),
      target_head_sha: normalizeStatus(
        closeoutEvaluation.topology?.targetHeadSha || durableCommittedProof?.target_head_sha,
        "<missing>",
      ),
      prepare_worktree_dir: normalizeStatus(
        durableCommittedProof?.prepare_worktree_dir || livePrepareHealth?.prepare_worktree_dir,
        "<missing>",
      ),
      gate_state_path: normalizePath(gateStatePath) || "<missing>",
    },
    current_main_compatibility: {
      status: normalizeStatus(closeoutEvaluation.scopeCompatibility?.parsed?.currentMainCompatibilityStatus, "<missing>"),
      baseline_sha: normalizeStatus(closeoutEvaluation.scopeCompatibility?.parsed?.currentMainCompatibilityBaselineSha, "<missing>"),
      verified_at_utc: normalizeStatus(closeoutEvaluation.scopeCompatibility?.parsed?.currentMainCompatibilityVerifiedAtUtc, "<missing>"),
      packet_widening_decision: normalizeStatus(closeoutEvaluation.scopeCompatibility?.parsed?.packetWideningDecision, "<missing>"),
      packet_widening_evidence: normalizeStatus(closeoutEvaluation.scopeCompatibility?.parsed?.packetWideningEvidence, "<missing>"),
      current_main_head_sha: normalizeStatus(closeoutEvaluation.topology?.currentMainHeadSha, "<missing>"),
      errors: closeoutEvaluation.scopeCompatibility?.errors || [],
    },
    closeout_bundle: {
      status: closeoutEvaluation.closeoutBundle?.ok ? "PASS" : "FAIL",
      request_count: closeoutEvaluation.closeoutBundle?.summary?.request_count ?? 0,
      result_count: closeoutEvaluation.closeoutBundle?.summary?.result_count ?? 0,
      session_count: closeoutEvaluation.closeoutBundle?.summary?.session_count ?? 0,
      active_run_count: closeoutEvaluation.closeoutBundle?.summary?.active_run_count ?? 0,
    },
    closeout_provenance: {
      status: latestCloseoutEvent ? "RECORDED" : "MISSING",
      mode: normalizeStatus(latestCloseoutEvent?.mode, "NONE"),
      actor_role: normalizeStatus(latestCloseoutEvent?.actor_role, "NONE"),
      actor_session_id: normalizeStatus(latestCloseoutEvent?.actor_session_id, "NONE"),
      actor_source: normalizeStatus(latestCloseoutEvent?.actor_source, "NONE"),
      recorded_at_utc: normalizeStatus(latestCloseoutEvent?.timestamp_utc, "NONE"),
      main_containment_status: normalizeStatus(latestCloseoutEvent?.main_containment_status, "NONE"),
      merged_main_commit: normalizeStatus(latestCloseoutEvent?.merged_main_commit, "NONE"),
      baseline_sha: normalizeStatus(latestCloseoutEvent?.current_main_compatibility_baseline_sha, "NONE"),
    },
    required_commands: requiredCommandsForState({
      wpId,
      actorContext,
      governanceBlocked: !governanceState.allowValidationResume,
    }),
    context_notes: contextNotes,
  };
}

export function formatIntegrationValidatorContextBrief(brief) {
  const lines = [
    "INTEGRATION_VALIDATOR_CONTEXT_BRIEF [CX-VAL-INT-001]",
    `- WP_ID: ${brief.wp_id}`,
    `- CONTEXT_STATUS: ${brief.context_status}`,
    `- CLOSEOUT_READINESS: ${brief.closeout_readiness}`,
    `- WORKFLOW_LANE: ${brief.workflow_lane} | PACKET_STATUS: ${brief.packet_status} | CURRENT_WP_STATUS: ${brief.current_wp_status} | TASK_BOARD_STATUS: ${brief.task_board_status}`,
    `- AUTHORITIES: technical=${brief.authority.technical_authority} | merge=${brief.authority.merge_authority} | integration_validator=${brief.authority.integration_validator_of_record} | wp_validator=${brief.authority.wp_validator_of_record}`,
    `- ACTOR_CONTEXT: role=${brief.actor_context.role} | source=${brief.actor_context.source} | session=${brief.actor_context.session_id} | thread=${brief.actor_context.thread_id} | branch=${brief.actor_context.branch}`,
    `- GOVERNANCE_ROOT: live=${brief.governance_root.live_root} | main_backup=${brief.governance_root.local_main_backup_root} | mode=${brief.governance_root.mode}`,
    `- COMMITTED_HANDOFF: status=${brief.committed_handoff.status} | live_prepare=${brief.committed_handoff.live_prepare_worktree_status} | mode=${brief.committed_handoff.committed_validation_mode} | target=${brief.committed_handoff.committed_validation_target}`,
    `- MAIN_COMPATIBILITY: status=${brief.current_main_compatibility.status} | baseline=${brief.current_main_compatibility.baseline_sha} | verified_at=${brief.current_main_compatibility.verified_at_utc} | main_head=${brief.current_main_compatibility.current_main_head_sha}`,
    `- CLOSEOUT_BUNDLE: requests=${brief.closeout_bundle.request_count} | results=${brief.closeout_bundle.result_count} | sessions=${brief.closeout_bundle.session_count} | active_runs=${brief.closeout_bundle.active_run_count}`,
    `- CLOSEOUT_PROVENANCE: status=${brief.closeout_provenance.status} | mode=${brief.closeout_provenance.mode} | actor=${brief.closeout_provenance.actor_role}/${brief.closeout_provenance.actor_session_id} | recorded_at=${brief.closeout_provenance.recorded_at_utc}`,
    `- ARTIFACT_POINTERS: packet=${brief.packet_path} | command_surface=${brief.command_surface_path} | gate_state=${brief.committed_handoff.gate_state_path} | prepare_worktree=${brief.committed_handoff.prepare_worktree_dir}`,
    `- MINIMAL_LIVE_READ_SET: ${formatBoundedList(brief.minimal_live_read_set, 6).join(" | ")}`,
    `- STARTUP_SEQUENCE: ${brief.startup_sequence.join(" -> ")}`,
    `- ANTI_REDISCOVERY_RULE: ${brief.anti_rediscovery_rule}`,
    `- FULL_OUTPUT_RULE: use --json for the machine-readable full brief instead of rereading protocols or packet history`,
  ];

  if (brief.declared_topology.issues.length > 0) {
    lines.push("- DECLARED_TOPOLOGY_ISSUES:");
    for (const issue of formatBoundedList(brief.declared_topology.issues)) lines.push(`  - ${issue}`);
  }

  if ((brief.current_main_compatibility.errors || []).length > 0) {
    lines.push("- SIGNED_SCOPE_COMPATIBILITY_ERRORS:");
    for (const issue of formatBoundedList(brief.current_main_compatibility.errors)) lines.push(`  - ${issue}`);
  }

  if ((brief.context_notes || []).length > 0) {
    lines.push("- CONTEXT_NOTES:");
    for (const note of formatBoundedList(brief.context_notes)) lines.push(`  - ${note}`);
  }

  if ((brief.required_commands || []).length > 0) {
    lines.push("- REQUIRED_COMMANDS:");
    for (const command of formatBoundedList(brief.required_commands, 6)) lines.push(`  - ${command}`);
  }

  return `${lines.join("\n")}\n`;
}
