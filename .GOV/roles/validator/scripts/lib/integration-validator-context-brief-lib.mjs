import fs from "node:fs";
import { packetPath, parseCurrentWpStatus, parseStatus, taskBoardStatus } from "../../../../roles_shared/scripts/lib/role-resume-utils.mjs";
import { evaluateWpDeclaredTopology } from "../../../../roles_shared/scripts/lib/wp-declared-topology-lib.mjs";
import { evaluateIntegrationValidatorCloseoutState } from "./integration-validator-closeout-lib.mjs";
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
  repoRoot = process.cwd(),
  wpId = "",
  packetContent = "",
  packetPathValueOverride = "",
  gitContext = {},
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
  const governanceState = evaluateValidatorPacketGovernanceState({
    wpId,
    packetPath: packetPathValue,
    packetContent,
    currentWpStatus,
    taskBoardStatus: boardStatus,
  });
  const actorContext = resolveValidatorActorContext({
    repoRoot,
    wpId,
    packetContent,
    gitContext,
    registrySessions,
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
    `- WORKFLOW_LANE: ${brief.workflow_lane}`,
    `- PACKET_PATH: ${brief.packet_path}`,
    `- PACKET_STATUS: ${brief.packet_status}`,
    `- CURRENT_WP_STATUS: ${brief.current_wp_status}`,
    `- TASK_BOARD_STATUS: ${brief.task_board_status}`,
    `- COMMAND_SURFACE_PATH: ${brief.command_surface_path}`,
    `- TECHNICAL_AUTHORITY: ${brief.authority.technical_authority}`,
    `- MERGE_AUTHORITY: ${brief.authority.merge_authority}`,
    `- INTEGRATION_VALIDATOR_OF_RECORD: ${brief.authority.integration_validator_of_record}`,
    `- WP_VALIDATOR_OF_RECORD: ${brief.authority.wp_validator_of_record}`,
    `- ACTOR_ROLE: ${brief.actor_context.role}`,
    `- ACTOR_SOURCE: ${brief.actor_context.source}`,
    `- ACTOR_SESSION_KEY: ${brief.actor_context.session_key}`,
    `- ACTOR_SESSION_ID: ${brief.actor_context.session_id}`,
    `- ACTOR_THREAD_ID: ${brief.actor_context.thread_id}`,
    `- ACTOR_BRANCH: ${brief.actor_context.branch}`,
    `- ACTOR_WORKTREE_DIR: ${brief.actor_context.worktree_dir}`,
    `- DECLARED_TOPOLOGY_STATUS: ${brief.declared_topology.status}`,
    `- COMMITTED_HANDOFF_STATUS: ${brief.committed_handoff.status}`,
    `- LIVE_PREPARE_WORKTREE_STATUS: ${brief.committed_handoff.live_prepare_worktree_status}`,
    `- COMMITTED_VALIDATION_MODE: ${brief.committed_handoff.committed_validation_mode}`,
    `- COMMITTED_VALIDATION_TARGET: ${brief.committed_handoff.committed_validation_target}`,
    `- COMMITTED_TARGET_HEAD_SHA: ${brief.committed_handoff.target_head_sha}`,
    `- PREPARE_WORKTREE_DIR: ${brief.committed_handoff.prepare_worktree_dir}`,
    `- GATE_STATE_PATH: ${brief.committed_handoff.gate_state_path}`,
    `- CURRENT_MAIN_COMPATIBILITY_STATUS: ${brief.current_main_compatibility.status}`,
    `- CURRENT_MAIN_COMPATIBILITY_BASELINE_SHA: ${brief.current_main_compatibility.baseline_sha}`,
    `- CURRENT_MAIN_COMPATIBILITY_VERIFIED_AT_UTC: ${brief.current_main_compatibility.verified_at_utc}`,
    `- PACKET_WIDENING_DECISION: ${brief.current_main_compatibility.packet_widening_decision}`,
    `- PACKET_WIDENING_EVIDENCE: ${brief.current_main_compatibility.packet_widening_evidence}`,
    `- CURRENT_MAIN_HEAD_SHA: ${brief.current_main_compatibility.current_main_head_sha}`,
    `- WP_SCOPED_REQUEST_COUNT: ${brief.closeout_bundle.request_count}`,
    `- WP_SCOPED_RESULT_COUNT: ${brief.closeout_bundle.result_count}`,
    `- WP_SCOPED_SESSION_COUNT: ${brief.closeout_bundle.session_count}`,
    `- WP_SCOPED_ACTIVE_RUN_COUNT: ${brief.closeout_bundle.active_run_count}`,
    `- MINIMAL_LIVE_READ_SET: ${brief.minimal_live_read_set.join(" | ")}`,
    `- STARTUP_SEQUENCE: ${brief.startup_sequence.join(" -> ")}`,
    `- ANTI_REDISCOVERY_RULE: ${brief.anti_rediscovery_rule}`,
  ];

  if (brief.declared_topology.issues.length > 0) {
    lines.push("- DECLARED_TOPOLOGY_ISSUES:");
    for (const issue of brief.declared_topology.issues) lines.push(`  - ${issue}`);
  }

  if ((brief.current_main_compatibility.errors || []).length > 0) {
    lines.push("- SIGNED_SCOPE_COMPATIBILITY_ERRORS:");
    for (const issue of brief.current_main_compatibility.errors) lines.push(`  - ${issue}`);
  }

  if ((brief.context_notes || []).length > 0) {
    lines.push("- CONTEXT_NOTES:");
    for (const note of brief.context_notes) lines.push(`  - ${note}`);
  }

  if ((brief.required_commands || []).length > 0) {
    lines.push("- REQUIRED_COMMANDS:");
    for (const command of brief.required_commands) lines.push(`  - ${command}`);
  }

  return `${lines.join("\n")}\n`;
}
