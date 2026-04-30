import { parsePacketStatus } from "./wp-authority-projection-lib.mjs";
import { readVerdictSettlementTruth } from "./merge-progression-truth-lib.mjs";
import {
  inferValidationVerdictFromPublication,
  readExecutionPublicationView,
} from "./wp-execution-state-lib.mjs";
import {
  buildProjectionDebtKeys,
  inferTerminalStateFromCloseoutView,
  normalizeTerminalCloseoutRecord,
} from "./terminal-closeout-record-lib.mjs";
import {
  authorityClassForCloseoutDependency,
  dependencyBlocksProductOutcome,
} from "./closeout-blocking-authority-lib.mjs";

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

function uniqueStrings(values = []) {
  return [...new Set(
    (Array.isArray(values) ? values : [])
      .map((value) => String(value || "").trim())
      .filter(Boolean),
  )];
}

function normalizeTerminalRecordInput(input = null) {
  if (!input) {
    return {
      status: "UNASSESSED",
      path: "",
      record: null,
      errors: [],
    };
  }
  if (input.status || input.record || input.errors || input.path) {
    const record = normalizeTerminalCloseoutRecord(input.record || null);
    return {
      status: normalizeText(input.status, record ? "PRESENT" : "ABSENT"),
      path: normalizeText(input.path),
      record,
      errors: Array.isArray(input.errors) ? input.errors.map((entry) => normalizeText(entry)).filter(Boolean) : [],
    };
  }
  const record = normalizeTerminalCloseoutRecord(input);
  return {
    status: record ? "PRESENT" : "ABSENT",
    path: "",
    record,
    errors: [],
  };
}

function terminalRecordSummary(readResult = {}) {
  const status = normalizeText(readResult.status, "ABSENT");
  const record = readResult.record || null;
  if (record) {
    return [
      `state=${record.terminal_state}`,
      `verdict=${record.verdict}`,
      `mode=${record.closeout_mode}`,
      `updated_at=${record.updated_at_utc || "<missing>"}`,
      `debt=${uniqueStrings([...(record.governance_debt_keys || []), ...(record.projection_debt_keys || [])]).join(",") || "none"}`,
    ].join(" | ");
  }
  if (status === "INVALID") {
    return `Terminal closeout record is invalid: ${(readResult.errors || []).join(" | ") || "unknown error"}`;
  }
  if (status === "UNASSESSED") {
    return "Terminal closeout record was not read by this caller; packet/runtime/provenance fallback is being used.";
  }
  return "Terminal closeout record is absent; packet/runtime/provenance fallback is being used.";
}

function terminalRecordDependencyStatus(readResult = {}, verdictOfRecord = "") {
  const status = normalizeText(readResult.status, "UNASSESSED").toUpperCase();
  if (status === "PRESENT") return "PRESENT";
  if (status === "INVALID") return "INVALID";
  if (status === "UNASSESSED") return "UNASSESSED";
  return verdictOfRecord ? "MISSING" : "ABSENT";
}

function dependencyEntry({
  key,
  required,
  status,
  summary,
} = {}) {
  const normalizedKey = normalizeText(key);
  const normalizedStatus = normalizeText(status, "UNKNOWN");
  const requiredFailure = Boolean(required) && normalizedStatus === "FAIL";
  const productOutcomeBlocker = dependencyBlocksProductOutcome({
    key: normalizedKey,
    required,
    status: normalizedStatus,
  });
  return {
    key: normalizedKey,
    required: Boolean(required),
    status: normalizedStatus,
    summary: normalizeText(summary, "No summary available."),
    authority_class: authorityClassForCloseoutDependency(normalizedKey),
    blocks_product_outcome: productOutcomeBlocker,
    blocking_disposition: !requiredFailure
      ? "NONE"
      : (productOutcomeBlocker ? "OUTCOME_BLOCKER" : "SETTLEMENT_DEBT"),
  };
}

function normalizeRepomemCoverage(repomemCoverage = null) {
  const state = normalizeText(repomemCoverage?.state, "UNASSESSED");
  const activeRoles = Array.isArray(repomemCoverage?.active_roles) ? repomemCoverage.active_roles : [];
  const debtRoles = Array.isArray(repomemCoverage?.debt_roles) ? repomemCoverage.debt_roles : [];
  const debtKeys = Array.isArray(repomemCoverage?.debt_keys) ? repomemCoverage.debt_keys : [];
  if (repomemCoverage?.summary) {
    return {
      state,
      dependencyStatus: state === "NO_ACTIVE_ROLES" ? "NO_ACTIVITY" : state,
      summary: repomemCoverage.summary,
    };
  }
  if (state === "NO_ACTIVE_ROLES") {
    return {
      state,
      dependencyStatus: "NO_ACTIVITY",
      summary: "No materially active roles were detected for this WP.",
    };
  }
  if (state === "PASS") {
    return {
      state,
      dependencyStatus: "PASS",
      summary: `state=PASS | active_roles=${activeRoles.join(",") || "none"} | debt_keys=none`,
    };
  }
  if (state === "DEBT") {
    return {
      state,
      dependencyStatus: "DEBT",
      summary: `state=DEBT | active_roles=${activeRoles.join(",") || "none"} | debt_roles=${debtRoles.join(",") || "none"} | debt_keys=${debtKeys.join(",") || "none"}`,
    };
  }
  return {
    state,
    dependencyStatus: "UNASSESSED",
    summary: "Repomem coverage was not assessed for this closeout view.",
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
  repomemCoverage = null,
  terminalCloseoutRecord = null,
} = {}) {
  const packetStatusArtifact = parsePacketStatus(packetContent);
  const publication = readExecutionPublicationView({
    runtimeStatus,
    packetStatus: packetStatusArtifact,
    taskBoardStatus,
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
  const latestEventSettlementDebtKeys = uniqueStrings(latestEvent?.settlement_debt_keys);
  const latestEventSettlementDebtSummaries = Array.isArray(latestEvent?.settlement_debt_summaries)
    ? latestEvent.settlement_debt_summaries.map((value) => normalizeText(value)).filter(Boolean)
    : [];
  const normalizedRepomemCoverage = normalizeRepomemCoverage(repomemCoverage);
  const terminalRecordRead = normalizeTerminalRecordInput(terminalCloseoutRecord);

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
            `settlement_debt=${latestEventSettlementDebtKeys.join(",") || "none"}`,
          ].join(" | ")
        : "No governed closeout sync provenance is recorded yet.",
    }),
    repomem_coverage: dependencyEntry({
      key: "repomem_coverage",
      required: false,
      status: normalizedRepomemCoverage.dependencyStatus,
      summary: normalizedRepomemCoverage.summary,
    }),
  };

  const blockingKeys = Object.values(dependencies)
    .filter((dependency) => dependency.required && dependency.status === "FAIL")
    .map((dependency) => dependency.key);
  const productOutcomeBlockingKeys = Object.values(dependencies)
    .filter((dependency) => dependency.blocks_product_outcome)
    .map((dependency) => dependency.key);
  const dependencyGovernanceDebtSummaries = Object.values(dependencies)
    .filter((dependency) => dependency.blocking_disposition === "SETTLEMENT_DEBT")
    .map((dependency) => ({ key: dependency.key, summary: dependency.summary }));
  const governanceDebtSummaryMap = new Map(
    dependencyGovernanceDebtSummaries.map((entry) => [entry.key, entry.summary]),
  );
  latestEventSettlementDebtKeys.forEach((key, index) => {
    governanceDebtSummaryMap.set(
      key,
      latestEventSettlementDebtSummaries[index] || governanceDebtSummaryMap.get(key) || key,
    );
  });
  const baseGovernanceDebtKeys = uniqueStrings([
    ...dependencyGovernanceDebtSummaries.map((entry) => entry.key),
    ...latestEventSettlementDebtKeys,
  ]);
  const verdictSettlement = readVerdictSettlementTruth({
    packetText: packetContent,
    runtimeStatus,
    taskBoardStatus,
    blockingKeys,
  });
  const validationVerdict = verdictSettlement.verdictOfRecord
    || inferValidationVerdictFromPublication({
      packetStatus: publication.packet_status || packetStatusArtifact,
      taskBoardStatus: publication.task_board_status || taskBoardStatus,
    });
  const closeoutMode = verdictSettlement.closeoutMode;
  const settlementBlockers = verdictSettlement.terminalPublicationRecorded
    ? []
    : verdictSettlement.settlementBlockers;
  const settlementBlockerSummaries = settlementBlockers.map((blocker) => {
    if (blocker === "TERMINAL_PUBLICATION_PENDING") {
      return "Final closeout publication has not been recorded yet.";
    }
    if (governanceDebtSummaryMap.has(blocker)) {
      return governanceDebtSummaryMap.get(blocker);
    }
    return dependencies[String(blocker || "").trim().toLowerCase()]?.summary || blocker;
  });
  const terminalRecordStatus = terminalRecordDependencyStatus(terminalRecordRead, verdictSettlement.verdictOfRecord);
  dependencies.terminal_record = dependencyEntry({
    key: "terminal_record",
    required: false,
    status: terminalRecordStatus,
    summary: terminalRecordSummary(terminalRecordRead),
  });

  const projectionDebtKeys = buildProjectionDebtKeys(publication);
  if (publication?.packet_projection_drift) {
    governanceDebtSummaryMap.set(
      "PACKET_PROJECTION_DRIFT",
      "Packet closeout status is a stale projection of canonical terminal authority.",
    );
  }
  if (publication?.task_board_projection_drift) {
    governanceDebtSummaryMap.set(
      "TASK_BOARD_PROJECTION_DRIFT",
      "Task-board closeout status is a stale projection of canonical terminal authority.",
    );
  }
  if (terminalRecordStatus === "MISSING") {
    governanceDebtSummaryMap.set(
      "TERMINAL_CLOSEOUT_RECORD_MISSING",
      "Verdict-of-record exists but the schema-versioned terminal closeout record is not yet published.",
    );
  }
  if (terminalRecordStatus === "INVALID") {
    governanceDebtSummaryMap.set(
      "TERMINAL_CLOSEOUT_RECORD_INVALID",
      terminalRecordSummary(terminalRecordRead),
    );
  }
  for (const key of terminalRecordRead.record?.governance_debt_keys || []) {
    governanceDebtSummaryMap.set(key, governanceDebtSummaryMap.get(key) || key);
  }
  for (const [index, key] of (terminalRecordRead.record?.projection_debt_keys || []).entries()) {
    governanceDebtSummaryMap.set(
      key,
      terminalRecordRead.record?.governance_debt_summaries?.[index] || governanceDebtSummaryMap.get(key) || key,
    );
  }
  const governanceDebtKeys = uniqueStrings([
    ...baseGovernanceDebtKeys,
    ...projectionDebtKeys,
    ...(terminalRecordStatus === "MISSING" ? ["TERMINAL_CLOSEOUT_RECORD_MISSING"] : []),
    ...(terminalRecordStatus === "INVALID" ? ["TERMINAL_CLOSEOUT_RECORD_INVALID"] : []),
    ...(terminalRecordRead.record?.governance_debt_keys || []),
    ...(terminalRecordRead.record?.projection_debt_keys || []),
  ]);
  const terminalState = terminalRecordRead.record?.terminal_state || inferTerminalStateFromCloseoutView({
    verdict: verdictSettlement.verdictOfRecord || validationVerdict,
    closeoutMode,
    mainContainmentStatus: publication?.runtime?.main_containment_status,
    productOutcomeBlockers: productOutcomeBlockingKeys,
    governanceDebtKeys,
    projectionDebtKeys,
    settlementBlockers,
    terminalPublicationRecorded: verdictSettlement.terminalPublicationRecorded,
  });
  const effectiveSettlementState = terminalRecordRead.record
    ? terminalState
    : verdictSettlement.settlementState;
  const effectiveSettlementBlockers = terminalRecordRead.record
    ? uniqueStrings([
      ...(terminalRecordRead.record.settlement_blockers || []),
      ...(terminalRecordRead.record.governance_debt_keys || []),
      ...(terminalRecordRead.record.projection_debt_keys || []),
    ])
    : settlementBlockers;
  const effectiveSettlementBlockerSummaries = terminalRecordRead.record
    ? effectiveSettlementBlockers.map((blocker) => governanceDebtSummaryMap.get(blocker) || blocker)
    : settlementBlockerSummaries;
  const effectiveVerdictOfRecord = terminalRecordRead.record?.verdict && terminalRecordRead.record.verdict !== "UNKNOWN"
    ? terminalRecordRead.record.verdict
    : verdictSettlement.verdictOfRecord;
  const effectiveCloseoutMode = terminalRecordRead.record?.closeout_mode && terminalRecordRead.record.closeout_mode !== "UNSET"
    ? terminalRecordRead.record.closeout_mode
    : closeoutMode;

  return {
    ok: blockingKeys.length === 0,
    outcome_ok: productOutcomeBlockingKeys.length === 0,
    publication: {
      has_canonical_authority: Boolean(publication?.has_canonical_authority),
      packet_status: normalizeText(publication?.packet_status, "<missing>"),
      task_board_status: normalizeText(publication?.task_board_status, "<missing>"),
      validation_verdict: normalizeText(validationVerdict, "UNKNOWN"),
      verdict_of_record: normalizeText(effectiveVerdictOfRecord, "UNKNOWN"),
      verdict_recorded_at_utc: normalizeText(terminalRecordRead.record?.verdict_recorded_at_utc || verdictSettlement.verdictRecordedAtUtc, "UNKNOWN"),
      verdict_actor_role: normalizeText(terminalRecordRead.record?.verdict_actor_role || verdictSettlement.verdictActorRole, "UNKNOWN"),
      verdict_actor_session: normalizeText(terminalRecordRead.record?.verdict_actor_session || verdictSettlement.verdictActorSession, "UNKNOWN"),
      verdict_evidence_pointer: normalizeText(terminalRecordRead.record?.verdict_evidence_pointer || verdictSettlement.verdictEvidencePointer, "UNKNOWN"),
      main_containment_status: normalizeText(terminalRecordRead.record?.main_containment_status || publication?.runtime?.main_containment_status, "UNKNOWN"),
      merged_main_commit: normalizeText(terminalRecordRead.record?.merged_main_commit || publication?.runtime?.merged_main_commit, "<missing>"),
      closeout_mode: normalizeText(effectiveCloseoutMode, "UNSET"),
      packet_projection_drift: Boolean(publication?.packet_projection_drift),
      task_board_projection_drift: Boolean(publication?.task_board_projection_drift),
    },
    settlement: {
      state: normalizeText(effectiveSettlementState, "VERDICT_PENDING"),
      terminal_state: normalizeText(terminalState, "NO_VERDICT"),
      blockers: effectiveSettlementBlockers,
      blocker_summaries: effectiveSettlementBlockerSummaries,
      blocker_count: effectiveSettlementBlockers.length,
      terminal_publication_recorded: Boolean(
        terminalRecordRead.record?.terminal_publication_recorded ?? verdictSettlement.terminalPublicationRecorded,
      ),
    },
    terminal_closeout_record: {
      status: terminalRecordStatus,
      path: terminalRecordRead.path || "<none>",
      authoritative: Boolean(terminalRecordRead.record),
      schema_version: terminalRecordRead.record?.schema_version || "terminal_closeout_record@1",
      terminal_state: normalizeText(terminalState, "NO_VERDICT"),
      verdict: normalizeText(effectiveVerdictOfRecord, "UNKNOWN"),
      updated_at_utc: normalizeText(terminalRecordRead.record?.updated_at_utc, "UNKNOWN"),
      projection_debt_keys: projectionDebtKeys,
      errors: terminalRecordRead.errors || [],
      summary: terminalRecordSummary(terminalRecordRead),
    },
    requirements,
    dependencies,
    blocking_keys: blockingKeys,
    product_outcome_blocking_keys: productOutcomeBlockingKeys,
    governance_debt_keys: governanceDebtKeys,
    governance_debt_summaries: governanceDebtKeys.map((key) => governanceDebtSummaryMap.get(key) || key),
    summary: [
      `mode=${normalizeText(effectiveCloseoutMode, "UNSET")}`,
      `verdict=${normalizeText(validationVerdict, "UNKNOWN")}`,
      `terminal_state=${normalizeText(terminalState, "NO_VERDICT")}`,
      `settlement=${normalizeText(effectiveSettlementState, "VERDICT_PENDING")}`,
      `blockers=${effectiveSettlementBlockers.length > 0 ? effectiveSettlementBlockers.join(",") : "none"}`,
      `outcome_blockers=${productOutcomeBlockingKeys.length > 0 ? productOutcomeBlockingKeys.join(",") : "none"}`,
      `governance_debt=${governanceDebtKeys.length > 0 ? governanceDebtKeys.join(",") : "none"}`,
      `terminal_record=${terminalRecordStatus}`,
      `repomem=${dependencies.repomem_coverage.status}`,
    ].join(" | "),
  };
}
