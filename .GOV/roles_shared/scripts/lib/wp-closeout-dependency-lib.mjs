import { parsePacketStatus } from "./wp-authority-projection-lib.mjs";
import {
  inferExecutionCloseoutMode,
  inferValidationVerdictFromPublication,
  readExecutionPublicationView,
} from "./wp-execution-state-lib.mjs";

function normalizeText(value, fallback = "") {
  const text = String(value || "").trim();
  return text || fallback;
}

function firstMessage(values = [], fallback = "") {
  const first = (Array.isArray(values) ? values : [])
    .map((value) => String(value || "").trim())
    .find(Boolean);
  return first || fallback;
}

function countSummary(name, value) {
  return `${name}=${Number(value || 0)}`;
}

function dependencyEntry({
  key,
  required,
  status,
  summary,
} = {}) {
  return {
    key,
    required: Boolean(required),
    status: normalizeText(status, "UNKNOWN"),
    summary: normalizeText(summary, "No summary available."),
  };
}

export function buildCloseoutDependencyView({
  packetContent = "",
  runtimeStatus = {},
  taskBoardStatus = "",
  closeoutRequirements = null,
  topology = {},
  closeoutBundle = {},
  scopeCompatibility = {},
  candidateSignedScope = {},
  closeoutSyncGovernance = {},
} = {}) {
  const packetStatusArtifact = parsePacketStatus(packetContent);
  const publication = readExecutionPublicationView({
    runtimeStatus,
    packetStatus: packetStatusArtifact,
    taskBoardStatus,
  });
  const validationVerdict = inferValidationVerdictFromPublication({
    packetStatus: publication.packet_status || packetStatusArtifact,
    taskBoardStatus: publication.task_board_status || taskBoardStatus,
  });
  const closeoutMode = inferExecutionCloseoutMode({
    packetStatus: publication.packet_status || packetStatusArtifact,
    taskBoardStatus: publication.task_board_status || taskBoardStatus,
    mainContainmentStatus: publication.runtime?.main_containment_status || "",
    fallbackVerdict: validationVerdict,
  });
  const requirements = {
    requireReadyForPass: Boolean(closeoutRequirements?.requireReadyForPass),
    requireRecordedScopeCompatibility: Boolean(closeoutRequirements?.requireRecordedScopeCompatibility),
    terminalNonPass: Boolean(closeoutRequirements?.terminalNonPass),
  };
  const topologyIssues = Array.isArray(topology?.issues) ? topology.issues : [];
  const closeoutBundleIssues = Array.isArray(closeoutBundle?.issues) ? closeoutBundle.issues : [];
  const scopeErrors = Array.isArray(scopeCompatibility?.errors) ? scopeCompatibility.errors : [];
  const candidateErrors = Array.isArray(candidateSignedScope?.errors) ? candidateSignedScope.errors : [];
  const latestEvent = closeoutSyncGovernance?.latestEvent || null;
  const latestGovernedAction = closeoutSyncGovernance?.latestGovernedAction || null;

  const dependencies = {
    topology: dependencyEntry({
      key: "topology",
      required: true,
      status: topology?.ok ? "PASS" : "FAIL",
      summary: topology?.ok
        ? [
            `worktree=${normalizeText(topology?.resolvedWorktreeAbs, "<unknown>")}`,
            `target_head_sha=${normalizeText(topology?.targetHeadSha, "<missing>")}`,
            `current_main_head_sha=${normalizeText(topology?.currentMainHeadSha, "<missing>")}`,
          ].join(" | ")
        : firstMessage(topologyIssues, "Final-lane topology is not closeout-ready."),
    }),
    closeout_bundle: dependencyEntry({
      key: "closeout_bundle",
      required: true,
      status: closeoutBundle?.ok ? "PASS" : "FAIL",
      summary: closeoutBundle?.ok
        ? [
            countSummary("requests", closeoutBundle?.summary?.request_count),
            countSummary("results", closeoutBundle?.summary?.result_count),
            countSummary("sessions", closeoutBundle?.summary?.session_count),
            countSummary("active_runs", closeoutBundle?.summary?.active_run_count),
          ].join(" | ")
        : firstMessage(closeoutBundleIssues, "Session-control closeout bundle is not settled."),
    }),
    scope_compatibility: dependencyEntry({
      key: "scope_compatibility",
      required: requirements.requireRecordedScopeCompatibility,
      status: requirements.requireRecordedScopeCompatibility
        ? (scopeErrors.length === 0 ? "PASS" : "FAIL")
        : "SKIP",
      summary: requirements.requireRecordedScopeCompatibility
        ? (
            scopeErrors.length === 0
              ? [
                  `status=${normalizeText(scopeCompatibility?.parsed?.currentMainCompatibilityStatus, "<missing>")}`,
                  `baseline_sha=${normalizeText(scopeCompatibility?.parsed?.currentMainCompatibilityBaselineSha, "<missing>")}`,
                  `verified_at=${normalizeText(scopeCompatibility?.parsed?.currentMainCompatibilityVerifiedAtUtc, "<missing>")}`,
                ].join(" | ")
              : firstMessage(scopeErrors, "Signed-scope compatibility truth is invalid.")
          )
        : "Recorded scope compatibility is not required for the current terminal non-pass closeout.",
    }),
    candidate_target: dependencyEntry({
      key: "candidate_target",
      required: true,
      status: candidateErrors.length === 0 ? "PASS" : "FAIL",
      summary: candidateErrors.length === 0
        ? "Committed target and signed-scope surface agree."
        : firstMessage(candidateErrors, "Candidate target validation failed."),
    }),
    sync_provenance: dependencyEntry({
      key: "sync_provenance",
      required: false,
      status: latestEvent || latestGovernedAction ? "RECORDED" : "MISSING",
      summary: latestEvent || latestGovernedAction
        ? [
            `mode=${normalizeText(latestEvent?.mode, "NONE")}`,
            `governed_action=${normalizeText(latestGovernedAction?.rule_id, "NONE")}`,
            `recorded_at=${normalizeText(latestGovernedAction?.updated_at || latestEvent?.timestamp_utc, "<missing>")}`,
          ].join(" | ")
        : "No governed closeout sync provenance is recorded yet.",
    }),
  };

  const blockingKeys = Object.values(dependencies)
    .filter((dependency) => dependency.required && dependency.status === "FAIL")
    .map((dependency) => dependency.key);

  return {
    ok: blockingKeys.length === 0,
    publication: {
      has_canonical_authority: Boolean(publication?.has_canonical_authority),
      packet_status: normalizeText(publication?.packet_status, "<missing>"),
      task_board_status: normalizeText(publication?.task_board_status, "<missing>"),
      validation_verdict: normalizeText(validationVerdict, "UNKNOWN"),
      main_containment_status: normalizeText(publication?.runtime?.main_containment_status, "UNKNOWN"),
      merged_main_commit: normalizeText(publication?.runtime?.merged_main_commit, "<missing>"),
      closeout_mode: normalizeText(closeoutMode?.mode, "UNSET"),
      packet_projection_drift: Boolean(publication?.packet_projection_drift),
      task_board_projection_drift: Boolean(publication?.task_board_projection_drift),
    },
    requirements,
    dependencies,
    blocking_keys: blockingKeys,
    summary: [
      `mode=${normalizeText(closeoutMode?.mode, "UNSET")}`,
      `verdict=${normalizeText(validationVerdict, "UNKNOWN")}`,
      `blockers=${blockingKeys.length > 0 ? blockingKeys.join(",") : "none"}`,
    ].join(" | "),
  };
}
