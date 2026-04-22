import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { buildPhaseCheckCommand } from "../../../../roles_shared/checks/phase-check-lib.mjs";
import {
  currentGitContext,
  loadPacket,
  packetExists,
  packetPath,
  parseClaimField,
  parseCurrentWpStatus,
  resolveCommittedCoderHandoffRange,
  taskBoardStatus,
} from "../../../../roles_shared/scripts/lib/role-resume-utils.mjs";
import { evaluateWpDeclaredTopology } from "../../../../roles_shared/scripts/lib/wp-declared-topology-lib.mjs";
import { buildCloseoutDependencyView } from "../../../../roles_shared/scripts/lib/wp-closeout-dependency-lib.mjs";
import { REPO_ROOT, repoPathAbs } from "../../../../roles_shared/scripts/lib/runtime-paths.mjs";
import { resolveValidatorGatePath } from "../../../../roles_shared/scripts/lib/validator-gate-paths.mjs";
import {
  evaluateIntegrationValidatorCloseoutState,
  latestCloseoutSyncEvent,
  latestCloseoutSyncGovernedAction,
  loadDeclaredRuntimeStatus,
  resolveIntegrationValidatorCloseoutRequirements,
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
import { parsePacketStatus, taskBoardStatusForPacketStatus } from "../../../../roles_shared/scripts/lib/wp-authority-projection-lib.mjs";
import { readExecutionPublicationView } from "../../../../roles_shared/scripts/lib/wp-execution-state-lib.mjs";

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
      buildPhaseCheckCommand({ phase: "CLOSEOUT", wpId }),
    ];
  }

  return [
    `just check-notifications ${wpId} INTEGRATION_VALIDATOR`,
    `just ack-notifications ${wpId} INTEGRATION_VALIDATOR ${actorContext.actorSessionId || "<integration-validator-session>"}`,
    buildPhaseCheckCommand({ phase: "CLOSEOUT", wpId }),
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
  runtimeStatus = undefined,
  taskBoardStatusOverride = "",
  gitRunner = null,
  worktreeExists = fs.existsSync,
  fileExists = fs.existsSync,
  declaredTopologyEvaluation = null,
  gateStatePath = "",
  repomemCoverage = null,
} = {}) {
  if (!String(wpId || "").trim()) {
    throw new Error("WP_ID is required for integration-validator context brief generation.");
  }

  const packetPathValue = normalizePath(packetPathValueOverride || packetPath(wpId));
  const packetReadPath = normalizePath(repoPathAbs(packetPathValue));
  const packetStatusArtifact = parsePacketStatus(packetContent);
  const currentWpStatusArtifact = parseCurrentWpStatus(packetContent);
  const boardStatusArtifact = String(taskBoardStatusOverride || taskBoardStatus(wpId) || "").trim();
  const publication = readExecutionPublicationView({
    runtimeStatus: runtimeStatus || {},
    packetStatus: packetStatusArtifact,
    taskBoardStatus: boardStatusArtifact || taskBoardStatusForPacketStatus(packetStatusArtifact || ""),
  });
  const packetStatus = normalizeStatus(publication.packet_status || packetStatusArtifact, "<missing>");
  const currentWpStatus = normalizeStatus(
    publication.has_canonical_authority
      ? (publication.task_board_status || currentWpStatusArtifact)
      : (currentWpStatusArtifact || publication.task_board_status),
    "<empty>",
  );
  const boardStatus = normalizeStatus(
    publication.task_board_status || boardStatusArtifact || taskBoardStatusForPacketStatus(packetStatusArtifact || ""),
    "<none>",
  );
  const authority = readValidatorAuthority(packetContent);
  const actorContext = resolveValidatorActorContext({
    repoRoot,
    wpId,
    packetContent,
    gitContext,
    registrySessions,
  });
  const closeoutRequirements = resolveIntegrationValidatorCloseoutRequirements({
    packetContent,
    runtimeStatus,
    taskBoardStatus: publication.task_board_status || boardStatusArtifact,
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
    requireReadyForPass: closeoutRequirements.requireReadyForPass,
    requireRecordedScopeCompatibility: closeoutRequirements.requireRecordedScopeCompatibility,
    repomemCoverage,
  });
  const durableCommittedProof = committedEvidenceForCloseout(committedEvidence);
  const livePrepareHealth = livePrepareWorktreeHealthEvidence(committedEvidence);
  const latestCloseoutEvent = latestCloseoutSyncEvent(gateState, wpId);
  const latestCloseoutGovernedAction = latestCloseoutSyncGovernedAction(gateState, wpId);
  const candidateRange = resolveCommittedCoderHandoffRange(packetContent, wpId);
  const closeoutDependencyView = buildCloseoutDependencyView({
    packetContent,
    runtimeStatus,
    taskBoardStatus: publication.task_board_status || boardStatusArtifact,
    closeoutRequirements,
    topology: closeoutEvaluation.topology,
    closeoutBundle: closeoutEvaluation.closeoutBundle,
    scopeCompatibility: closeoutEvaluation.scopeCompatibility,
    candidateSignedScope: closeoutEvaluation.candidateSignedScope,
    closeoutSyncGovernance: {
      latestEvent: latestCloseoutEvent,
      latestGovernedAction: latestCloseoutGovernedAction,
    },
    repomemCoverage: closeoutEvaluation.repomemCoverage,
  });

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
  if (!closeoutEvaluation.productOutcomeOk && closeoutEvaluation.issues.length > 0) {
    pushUnique(contextNotes, `Closeout outcome blocker: ${closeoutEvaluation.issues[0]}`);
  } else if (!closeoutEvaluation.ok && closeoutEvaluation.issues.length > 0) {
    pushUnique(contextNotes, `Closeout blocker: ${closeoutEvaluation.issues[0]}`);
  }
  if ((closeoutDependencyView.governance_debt_keys || []).length > 0) {
    pushUnique(
      contextNotes,
      `Closeout governance debt: ${closeoutDependencyView.governance_debt_keys.join(", ")}.`,
    );
  }

  return {
    schema_id: "hsk.integration_validator_context_brief@1",
    schema_version: "integration_validator_context_brief_v1",
    wp_id: wpId,
    context_status: contextStatus,
    closeout_readiness: closeoutReadiness,
    closeout_requirements: {
      require_ready_for_pass: closeoutRequirements.requireReadyForPass,
      require_recorded_scope_compatibility: closeoutRequirements.requireRecordedScopeCompatibility,
      terminal_non_pass_packet: closeoutRequirements.terminalNonPass,
    },
    closeout_dependency_summary: closeoutDependencyView.summary,
    closeout_publication: closeoutDependencyView.publication,
    closeout_settlement: closeoutDependencyView.settlement,
    closeout_dependencies: closeoutDependencyView.dependencies,
    closeout_product_outcome_blockers: closeoutDependencyView.product_outcome_blocking_keys,
    closeout_governance_debt: closeoutDependencyView.governance_debt_keys,
    workflow_lane: authority.workflowLane || "<missing>",
    packet_path: packetPathValue,
    packet_read_path: packetReadPath || "<missing>",
    packet_status: packetStatus,
    current_wp_status: currentWpStatus,
    task_board_status: boardStatus,
    command_surface_path: COMMAND_SURFACE_PATH,
    minimal_live_read_set: [
      "startup output",
      "packet_read_path from integration-validator-context-brief",
      "active WP thread/notifications",
      COMMAND_SURFACE_PATH,
    ],
    anti_rediscovery_rule:
      "Do not rebuild final-lane branch/worktree/authority truth by rereading large protocols or rediscovering commands once this bundle is available.",
    startup_sequence: [
      "just validator-startup INTEGRATION_VALIDATOR",
      `just validator-next INTEGRATION_VALIDATOR ${wpId}`,
      `just integration-validator-context-brief ${wpId}`,
    ],
    authority: {
      technical_authority: authority.technicalAuthority || "<missing>",
      merge_authority: authority.mergeAuthority || "<missing>",
      integration_validator_of_record: authority.integrationValidatorOfRecord || "<unassigned>",
      wp_validator_of_record: authority.wpValidatorOfRecord || "<unassigned>",
    },
    candidate_under_review: {
      branch: normalizeStatus(parseClaimField(packetContent, "LOCAL_BRANCH"), "<missing>"),
      worktree_dir: normalizeStatus(parseClaimField(packetContent, "LOCAL_WORKTREE_DIR"), "<missing>"),
      validator_policy_branch: normalizeStatus(parseClaimField(packetContent, "WP_VALIDATOR_LOCAL_BRANCH"), "<missing>"),
      validator_policy_worktree_dir: normalizeStatus(parseClaimField(packetContent, "WP_VALIDATOR_LOCAL_WORKTREE_DIR"), "<missing>"),
      handoff_range: candidateRange
        ? `${candidateRange.baseRev}..${candidateRange.headRev}`
        : "<missing>",
      handoff_range_source: candidateRange?.source || "<missing>",
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
      governed_action_rule: normalizeStatus(latestCloseoutGovernedAction?.rule_id, "NONE"),
      governed_action_kind: normalizeStatus(latestCloseoutGovernedAction?.action_kind, "NONE"),
      governed_action_resume_disposition: normalizeStatus(latestCloseoutGovernedAction?.resume_disposition, "NONE"),
      governed_action_updated_at: normalizeStatus(latestCloseoutGovernedAction?.updated_at, "NONE"),
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
    `- CLOSEOUT_DEPENDENCY_SUMMARY: ${brief.closeout_dependency_summary}`,
    `- WORKFLOW_LANE: ${brief.workflow_lane} | PACKET_STATUS: ${brief.packet_status} | CURRENT_WP_STATUS: ${brief.current_wp_status} | TASK_BOARD_STATUS: ${brief.task_board_status}`,
    `- CLOSEOUT_REQUIREMENTS: require_ready_for_pass=${brief.closeout_requirements.require_ready_for_pass ? "YES" : "NO"} | require_recorded_scope_compatibility=${brief.closeout_requirements.require_recorded_scope_compatibility ? "YES" : "NO"} | terminal_non_pass_packet=${brief.closeout_requirements.terminal_non_pass_packet ? "YES" : "NO"}`,
    `- CLOSEOUT_PUBLICATION: mode=${brief.closeout_publication.closeout_mode} | verdict=${brief.closeout_publication.verdict_of_record} | containment=${brief.closeout_publication.main_containment_status} | canonical=${brief.closeout_publication.has_canonical_authority ? "YES" : "NO"}`,
    `- CLOSEOUT_SETTLEMENT: state=${brief.closeout_settlement.state} | blockers=${brief.closeout_settlement.blockers.join(",") || "none"} | terminal_publication_recorded=${brief.closeout_settlement.terminal_publication_recorded ? "YES" : "NO"}`,
    `- CLOSEOUT_DEPENDENCIES: topology=${brief.closeout_dependencies.topology.status} | bundle=${brief.closeout_dependencies.closeout_bundle.status} | scope=${brief.closeout_dependencies.scope_compatibility.status} | candidate=${brief.closeout_dependencies.candidate_target.status} | provenance=${brief.closeout_dependencies.sync_provenance.status} | repomem=${brief.closeout_dependencies.repomem_coverage.status}`,
    `- CLOSEOUT_AUTHORITY_SPLIT: outcome_blockers=${brief.closeout_product_outcome_blockers.join(",") || "none"} | governance_debt=${brief.closeout_governance_debt.join(",") || "none"}`,
    `- REPOMEM_COVERAGE: ${brief.closeout_dependencies.repomem_coverage.summary}`,
    `- AUTHORITIES: technical=${brief.authority.technical_authority} | merge=${brief.authority.merge_authority} | integration_validator=${brief.authority.integration_validator_of_record} | wp_validator=${brief.authority.wp_validator_of_record}`,
    `- CANDIDATE_UNDER_REVIEW: branch=${brief.candidate_under_review.branch} | worktree=${brief.candidate_under_review.worktree_dir} | handoff_range=${brief.candidate_under_review.handoff_range} | handoff_range_source=${brief.candidate_under_review.handoff_range_source} | validator_policy_branch=${brief.candidate_under_review.validator_policy_branch}`,
    `- ACTOR_CONTEXT: role=${brief.actor_context.role} | source=${brief.actor_context.source} | session=${brief.actor_context.session_id} | thread=${brief.actor_context.thread_id} | branch=${brief.actor_context.branch}`,
    `- GOVERNANCE_ROOT: live=${brief.governance_root.live_root} | main_backup=${brief.governance_root.local_main_backup_root} | mode=${brief.governance_root.mode}`,
    `- COMMITTED_HANDOFF: status=${brief.committed_handoff.status} | live_prepare=${brief.committed_handoff.live_prepare_worktree_status} | mode=${brief.committed_handoff.committed_validation_mode} | target=${brief.committed_handoff.committed_validation_target}`,
    `- MAIN_COMPATIBILITY: status=${brief.current_main_compatibility.status} | baseline=${brief.current_main_compatibility.baseline_sha} | verified_at=${brief.current_main_compatibility.verified_at_utc} | main_head=${brief.current_main_compatibility.current_main_head_sha}`,
    `- CLOSEOUT_BUNDLE: requests=${brief.closeout_bundle.request_count} | results=${brief.closeout_bundle.result_count} | sessions=${brief.closeout_bundle.session_count} | active_runs=${brief.closeout_bundle.active_run_count}`,
    `- CLOSEOUT_PROVENANCE: status=${brief.closeout_provenance.status} | mode=${brief.closeout_provenance.mode} | actor=${brief.closeout_provenance.actor_role}/${brief.closeout_provenance.actor_session_id} | governed_action=${brief.closeout_provenance.governed_action_rule}/${brief.closeout_provenance.governed_action_resume_disposition} | recorded_at=${brief.closeout_provenance.recorded_at_utc}`,
    `- ARTIFACT_POINTERS: packet_logical=${brief.packet_path} | packet_read=${brief.packet_read_path} | command_surface=${brief.command_surface_path} | gate_state=${brief.committed_handoff.gate_state_path} | prepare_worktree=${brief.committed_handoff.prepare_worktree_dir}`,
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

export function buildIntegrationValidatorContextBriefFromEnvironment({
  wpId = "",
  repoRoot = "",
  gitContext = null,
  gateState = null,
  gateStatePath = "",
} = {}) {
  const normalizedWpId = String(wpId || "").trim();
  if (!normalizedWpId) {
    throw new Error("WP_ID is required");
  }
  if (!packetExists(normalizedWpId)) {
    throw new Error(`Task packet not found: ${packetPath(normalizedWpId)}`);
  }

  const resolvedGitContext = gitContext || currentGitContext();
  const resolvedGateStatePath = gateStatePath || resolveValidatorGatePath(normalizedWpId);
  let resolvedGateState = gateState;
  if (!resolvedGateState) {
    resolvedGateState = {};
    if (fs.existsSync(repoPathAbs(resolvedGateStatePath))) {
      resolvedGateState = JSON.parse(fs.readFileSync(repoPathAbs(resolvedGateStatePath), "utf8"));
    }
  }
  const packetContent = loadPacket(normalizedWpId);
  const declaredRuntime = loadDeclaredRuntimeStatus({
    repoRoot: repoRoot || resolvedGitContext.topLevel || REPO_ROOT,
    packetContent,
  });

  return buildIntegrationValidatorContextBrief({
    repoRoot: repoRoot || resolvedGitContext.topLevel || REPO_ROOT,
    wpId: normalizedWpId,
    packetContent,
    gitContext: resolvedGitContext,
    gateState: resolvedGateState,
    committedEvidence: resolvedGateState?.committed_validation_evidence?.[normalizedWpId] || null,
    runtimeStatus: declaredRuntime.runtimeStatus,
    gateStatePath: resolvedGateStatePath,
  });
}

export function runIntegrationValidatorContextBriefCli(argv = process.argv.slice(2)) {
  const wpId = String(argv[0] || "").trim();
  if (!wpId || !/^WP-[A-Za-z0-9][A-Za-z0-9._-]*$/.test(wpId)) {
    console.error("Usage: node .GOV/roles/validator/scripts/lib/integration-validator-context-brief-lib.mjs WP-{ID} [--json]");
    process.exit(1);
  }

  const json = argv.slice(1).includes("--json");
  try {
    const brief = buildIntegrationValidatorContextBriefFromEnvironment({ wpId });
    if (json) {
      process.stdout.write(`${JSON.stringify(brief, null, 2)}\n`);
    } else {
      process.stdout.write(formatIntegrationValidatorContextBrief(brief));
    }
  } catch (error) {
    console.error(`[INTEGRATION_VALIDATOR_CONTEXT_BRIEF] ${error?.message || String(error || "")}`);
    process.exit(1);
  }
}

const isMain = process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url);
if (isMain) {
  runIntegrationValidatorContextBriefCli();
}
