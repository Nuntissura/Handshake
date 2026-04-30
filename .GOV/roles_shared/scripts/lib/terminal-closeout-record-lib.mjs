import fs from "node:fs";
import path from "node:path";

import {
  repoPathAbs,
  SHARED_GOV_WP_COMMUNICATIONS_ROOT,
} from "./runtime-paths.mjs";
import { writeJsonFile } from "../session/session-registry-lib.mjs";

export const TERMINAL_CLOSEOUT_RECORD_SCHEMA_VERSION = "terminal_closeout_record@1";
export const TERMINAL_CLOSEOUT_RECORD_FILE = "TERMINAL_CLOSEOUT_RECORD.json";

export const TERMINAL_CLOSEOUT_STATES = Object.freeze([
  "NO_VERDICT",
  "VERDICT_OF_RECORD",
  "MERGED",
  "SETTLEMENT_DEBT",
  "TERMINAL_SETTLED",
]);

const TERMINAL_CLOSEOUT_STATE_RANK = Object.freeze({
  NO_VERDICT: 0,
  VERDICT_OF_RECORD: 1,
  MERGED: 2,
  SETTLEMENT_DEBT: 2,
  TERMINAL_SETTLED: 3,
});

const TERMINAL_VERDICTS = new Set(["PASS", "FAIL", "OUTDATED_ONLY", "ABANDONED"]);

function isPlainObject(value) {
  return !!value && typeof value === "object" && !Array.isArray(value);
}

function normalizeText(value, fallback = "") {
  const text = String(value ?? "").trim();
  return text || fallback;
}

function normalizeUpper(value, fallback = "") {
  return normalizeText(value, fallback).toUpperCase();
}

function normalizeNullableText(value) {
  const text = normalizeText(value);
  return text || null;
}

function uniqueStrings(values = []) {
  return [...new Set(
    (Array.isArray(values) ? values : [])
      .map((value) => normalizeText(value))
      .filter(Boolean),
  )];
}

function normalizeTimestamp(value, fallback = "") {
  const text = normalizeText(value, fallback);
  if (!text) return "";
  const parsed = Date.parse(text);
  return Number.isNaN(parsed) ? text : new Date(parsed).toISOString();
}

function timestampMillis(value) {
  const parsed = Date.parse(normalizeText(value));
  return Number.isNaN(parsed) ? null : parsed;
}

function normalizeTerminalState(value) {
  const state = normalizeUpper(value, "NO_VERDICT");
  return TERMINAL_CLOSEOUT_STATES.includes(state) ? state : "NO_VERDICT";
}

function normalizeVerdict(value) {
  const verdict = normalizeUpper(value, "UNKNOWN");
  return TERMINAL_VERDICTS.has(verdict) ? verdict : "UNKNOWN";
}

function terminalVerdictConflict(left, right) {
  const leftVerdict = normalizeVerdict(left);
  const rightVerdict = normalizeVerdict(right);
  return TERMINAL_VERDICTS.has(leftVerdict)
    && TERMINAL_VERDICTS.has(rightVerdict)
    && leftVerdict !== rightVerdict;
}

function normalizeProductOutcomeState({
  verdict = "",
  mainContainmentStatus = "",
  productOutcomeBlockers = [],
} = {}) {
  const normalizedVerdict = normalizeVerdict(verdict);
  if (productOutcomeBlockers.length > 0) return "BLOCKED";
  if (normalizedVerdict === "UNKNOWN") return "NO_VERDICT";
  if (normalizedVerdict === "PASS") {
    return normalizeUpper(mainContainmentStatus) === "CONTAINED_IN_MAIN"
      ? "PASS_MERGED"
      : "PASS_VERDICT";
  }
  return normalizedVerdict;
}

function deriveTerminalState({
  verdict = "",
  closeoutMode = "",
  mainContainmentStatus = "",
  productOutcomeBlockers = [],
  governanceDebtKeys = [],
  projectionDebtKeys = [],
  settlementBlockers = [],
  terminalPublicationRecorded = false,
} = {}) {
  const normalizedVerdict = normalizeVerdict(verdict);
  if (normalizedVerdict === "UNKNOWN") return "NO_VERDICT";
  if (productOutcomeBlockers.length > 0) return "VERDICT_OF_RECORD";

  const debtKeys = uniqueStrings([
    ...governanceDebtKeys,
    ...projectionDebtKeys,
    ...settlementBlockers,
  ]);
  if (debtKeys.length > 0) return "SETTLEMENT_DEBT";

  const normalizedMode = normalizeUpper(closeoutMode);
  const normalizedContainment = normalizeUpper(mainContainmentStatus);
  if (normalizedVerdict === "PASS" && normalizedContainment === "CONTAINED_IN_MAIN") {
    return terminalPublicationRecorded ? "TERMINAL_SETTLED" : "MERGED";
  }
  if (["FAIL", "OUTDATED_ONLY", "ABANDONED"].includes(normalizedMode)) return "TERMINAL_SETTLED";
  if (normalizedMode === "CONTAINED_IN_MAIN") return "TERMINAL_SETTLED";
  return "VERDICT_OF_RECORD";
}

export function terminalCloseoutStateRank(state = "") {
  return TERMINAL_CLOSEOUT_STATE_RANK[normalizeTerminalState(state)] ?? 0;
}

export function terminalCloseoutRecordRelPath(wpId = "") {
  const normalizedWpId = normalizeText(wpId);
  if (!normalizedWpId || !/^WP-[A-Za-z0-9][A-Za-z0-9._-]*$/.test(normalizedWpId)) {
    throw new Error("WP_ID is required for terminal closeout record path resolution");
  }
  return path.join(SHARED_GOV_WP_COMMUNICATIONS_ROOT, normalizedWpId, TERMINAL_CLOSEOUT_RECORD_FILE);
}

export function terminalCloseoutRecordAbsPath(wpId = "") {
  return repoPathAbs(terminalCloseoutRecordRelPath(wpId));
}

export function normalizeTerminalCloseoutRecord(raw = null, {
  fallbackWpId = "",
} = {}) {
  if (!isPlainObject(raw)) return null;
  const wpId = normalizeText(raw.wp_id, fallbackWpId);
  const terminalState = normalizeTerminalState(raw.terminal_state || raw.status);
  const verdict = normalizeVerdict(raw.verdict || raw.product_verdict || raw.outcome);
  const governanceDebtKeys = uniqueStrings(raw.governance_debt_keys);
  const projectionDebtKeys = uniqueStrings(raw.projection_debt_keys);
  const productOutcomeBlockers = uniqueStrings(raw.product_outcome_blockers);
  const settlementBlockers = uniqueStrings(raw.settlement_blockers);
  const recordedAtUtc = normalizeTimestamp(raw.recorded_at_utc || raw.recorded_at || raw.timestamp_utc);
  const updatedAtUtc = normalizeTimestamp(raw.updated_at_utc || raw.updated_at || recordedAtUtc);
  return {
    schema_version: normalizeText(raw.schema_version, TERMINAL_CLOSEOUT_RECORD_SCHEMA_VERSION),
    wp_id: wpId,
    terminal_state: terminalState,
    terminal_state_rank: terminalCloseoutStateRank(terminalState),
    status: terminalState,
    product_outcome_state: normalizeText(raw.product_outcome_state)
      || normalizeProductOutcomeState({
        verdict,
        mainContainmentStatus: raw.main_containment_status,
        productOutcomeBlockers,
      }),
    closeout_mode: normalizeUpper(raw.closeout_mode, "UNSET"),
    verdict,
    verdict_recorded_at_utc: normalizeNullableText(raw.verdict_recorded_at_utc),
    verdict_actor_role: normalizeNullableText(raw.verdict_actor_role),
    verdict_actor_session: normalizeNullableText(raw.verdict_actor_session),
    verdict_evidence_pointer: normalizeNullableText(raw.verdict_evidence_pointer),
    product_outcome_blockers: productOutcomeBlockers,
    governance_debt_keys: governanceDebtKeys,
    governance_debt_summaries: uniqueStrings(raw.governance_debt_summaries),
    projection_debt_keys: projectionDebtKeys,
    settlement_blockers: settlementBlockers,
    settlement_blocker_summaries: uniqueStrings(raw.settlement_blocker_summaries),
    packet_status: normalizeNullableText(raw.packet_status),
    task_board_status: normalizeNullableText(raw.task_board_status),
    main_containment_status: normalizeNullableText(raw.main_containment_status),
    merged_main_commit: normalizeNullableText(raw.merged_main_commit),
    terminal_publication_recorded: Boolean(raw.terminal_publication_recorded),
    target_head_sha: normalizeNullableText(raw.target_head_sha),
    current_main_head_sha: normalizeNullableText(raw.current_main_head_sha),
    actor_role: normalizeNullableText(raw.actor_role || raw.writer_role),
    actor_session: normalizeNullableText(raw.actor_session || raw.writer_session),
    source: normalizeText(raw.source, "UNKNOWN"),
    recorded_at_utc: recordedAtUtc,
    updated_at_utc: updatedAtUtc || recordedAtUtc,
    previous_terminal_state: normalizeNullableText(raw.previous_terminal_state),
  };
}

export function validateTerminalCloseoutRecord(record = null) {
  const normalized = normalizeTerminalCloseoutRecord(record);
  const errors = [];
  if (!normalized) return ["terminal closeout record must be an object"];
  if (normalized.schema_version !== TERMINAL_CLOSEOUT_RECORD_SCHEMA_VERSION) {
    errors.push(`schema_version must be ${TERMINAL_CLOSEOUT_RECORD_SCHEMA_VERSION}`);
  }
  if (!normalized.wp_id || !/^WP-[A-Za-z0-9][A-Za-z0-9._-]*$/.test(normalized.wp_id)) {
    errors.push("wp_id must be a WP identifier");
  }
  if (!TERMINAL_CLOSEOUT_STATES.includes(normalized.terminal_state)) {
    errors.push(`terminal_state invalid (${normalized.terminal_state})`);
  }
  if (!["UNKNOWN", ...TERMINAL_VERDICTS].includes(normalized.verdict)) {
    errors.push(`verdict invalid (${normalized.verdict})`);
  }
  if (!normalized.recorded_at_utc) errors.push("recorded_at_utc is required");
  if (!normalized.updated_at_utc) errors.push("updated_at_utc is required");
  return errors;
}

export function readTerminalCloseoutRecord({
  wpId = "",
  recordPathAbs = "",
  fileExists = fs.existsSync,
  readFile = fs.readFileSync,
} = {}) {
  const pathAbs = recordPathAbs || terminalCloseoutRecordAbsPath(wpId);
  if (!fileExists(pathAbs)) {
    return {
      status: "ABSENT",
      path: pathAbs,
      record: null,
      errors: [],
    };
  }
  try {
    const parsed = JSON.parse(readFile(pathAbs, "utf8"));
    const record = normalizeTerminalCloseoutRecord(parsed, { fallbackWpId: wpId });
    const errors = validateTerminalCloseoutRecord(record);
    return {
      status: errors.length > 0 ? "INVALID" : "PRESENT",
      path: pathAbs,
      record,
      errors,
    };
  } catch (error) {
    return {
      status: "INVALID",
      path: pathAbs,
      record: null,
      errors: [String(error?.message || error)],
    };
  }
}

export function buildProjectionDebtKeys(publication = {}) {
  return uniqueStrings([
    publication?.packet_projection_drift ? "PACKET_PROJECTION_DRIFT" : "",
    publication?.task_board_projection_drift ? "TASK_BOARD_PROJECTION_DRIFT" : "",
  ]);
}

export function inferTerminalStateFromCloseoutView({
  verdict = "",
  closeoutMode = "",
  mainContainmentStatus = "",
  productOutcomeBlockers = [],
  governanceDebtKeys = [],
  projectionDebtKeys = [],
  settlementBlockers = [],
  terminalPublicationRecorded = false,
} = {}) {
  return deriveTerminalState({
    verdict,
    closeoutMode,
    mainContainmentStatus,
    productOutcomeBlockers: uniqueStrings(productOutcomeBlockers),
    governanceDebtKeys: uniqueStrings(governanceDebtKeys),
    projectionDebtKeys: uniqueStrings(projectionDebtKeys),
    settlementBlockers: uniqueStrings(settlementBlockers),
    terminalPublicationRecorded,
  });
}

export function buildTerminalCloseoutRecordFromCloseoutSync({
  wpId = "",
  mode = "",
  packetStatus = "",
  taskBoardStatus = "",
  mainContainmentStatus = "",
  mergedMainCommit = "",
  verdict = "",
  verdictRecordedAtUtc = "",
  verdictActorRole = "",
  verdictActorSession = "",
  verdictEvidencePointer = "",
  productOutcomeBlockers = [],
  governanceDebtKeys = [],
  governanceDebtSummaries = [],
  projectionDebtKeys = [],
  settlementBlockers = [],
  settlementBlockerSummaries = [],
  terminalPublicationRecorded = true,
  targetHeadSha = "",
  currentMainHeadSha = "",
  actorRole = "",
  actorSession = "",
  source = "CLOSEOUT_SYNC",
  recordedAtUtc = "",
  previousRecord = null,
} = {}) {
  const timestamp = normalizeTimestamp(recordedAtUtc, new Date().toISOString());
  const normalizedVerdict = normalizeVerdict(verdict);
  const normalizedProductOutcomeBlockers = uniqueStrings(productOutcomeBlockers);
  const normalizedGovernanceDebtKeys = uniqueStrings(governanceDebtKeys);
  const normalizedProjectionDebtKeys = uniqueStrings(projectionDebtKeys);
  const normalizedSettlementBlockers = uniqueStrings(settlementBlockers);
  const terminalState = deriveTerminalState({
    verdict: normalizedVerdict,
    closeoutMode: mode,
    mainContainmentStatus,
    productOutcomeBlockers: normalizedProductOutcomeBlockers,
    governanceDebtKeys: normalizedGovernanceDebtKeys,
    projectionDebtKeys: normalizedProjectionDebtKeys,
    settlementBlockers: normalizedSettlementBlockers,
    terminalPublicationRecorded,
  });
  return normalizeTerminalCloseoutRecord({
    schema_version: TERMINAL_CLOSEOUT_RECORD_SCHEMA_VERSION,
    wp_id: wpId,
    terminal_state: terminalState,
    status: terminalState,
    product_outcome_state: normalizeProductOutcomeState({
      verdict: normalizedVerdict,
      mainContainmentStatus,
      productOutcomeBlockers: normalizedProductOutcomeBlockers,
    }),
    closeout_mode: mode,
    verdict: normalizedVerdict,
    verdict_recorded_at_utc: verdictRecordedAtUtc,
    verdict_actor_role: verdictActorRole,
    verdict_actor_session: verdictActorSession,
    verdict_evidence_pointer: verdictEvidencePointer,
    product_outcome_blockers: normalizedProductOutcomeBlockers,
    governance_debt_keys: normalizedGovernanceDebtKeys,
    governance_debt_summaries: governanceDebtSummaries,
    projection_debt_keys: normalizedProjectionDebtKeys,
    settlement_blockers: normalizedSettlementBlockers,
    settlement_blocker_summaries: settlementBlockerSummaries,
    packet_status: packetStatus,
    task_board_status: taskBoardStatus,
    main_containment_status: mainContainmentStatus,
    merged_main_commit: mergedMainCommit,
    terminal_publication_recorded: terminalPublicationRecorded,
    target_head_sha: targetHeadSha,
    current_main_head_sha: currentMainHeadSha,
    actor_role: actorRole,
    actor_session: actorSession,
    source,
    recorded_at_utc: previousRecord?.recorded_at_utc || timestamp,
    updated_at_utc: timestamp,
    previous_terminal_state: previousRecord?.terminal_state || null,
  }, { fallbackWpId: wpId });
}

export function buildTerminalCloseoutRecordFromDependencyView({
  wpId = "",
  dependencyView = {},
  actorRole = "",
  actorSession = "",
  source = "CLOSEOUT_DEPENDENCY_VIEW",
  recordedAtUtc = "",
  previousRecord = null,
} = {}) {
  const publication = dependencyView?.publication || {};
  const settlement = dependencyView?.settlement || {};
  return buildTerminalCloseoutRecordFromCloseoutSync({
    wpId,
    mode: publication.closeout_mode,
    packetStatus: publication.packet_status,
    taskBoardStatus: publication.task_board_status,
    mainContainmentStatus: publication.main_containment_status,
    mergedMainCommit: publication.merged_main_commit,
    verdict: publication.verdict_of_record || publication.validation_verdict,
    verdictRecordedAtUtc: publication.verdict_recorded_at_utc,
    verdictActorRole: publication.verdict_actor_role,
    verdictActorSession: publication.verdict_actor_session,
    verdictEvidencePointer: publication.verdict_evidence_pointer,
    productOutcomeBlockers: dependencyView.product_outcome_blocking_keys,
    governanceDebtKeys: dependencyView.governance_debt_keys,
    governanceDebtSummaries: dependencyView.governance_debt_summaries,
    projectionDebtKeys: buildProjectionDebtKeys(publication),
    settlementBlockers: settlement.blockers,
    settlementBlockerSummaries: settlement.blocker_summaries,
    terminalPublicationRecorded: settlement.terminal_publication_recorded,
    actorRole,
    actorSession,
    source,
    recordedAtUtc,
    previousRecord,
  });
}

export function resolveTerminalCloseoutPublication({
  currentRecord = null,
  nextRecord = null,
} = {}) {
  const current = normalizeTerminalCloseoutRecord(currentRecord);
  const next = normalizeTerminalCloseoutRecord(nextRecord, { fallbackWpId: current?.wp_id || "" });
  if (!next) {
    return {
      ok: false,
      code: "TERMINAL_CLOSEOUT_RECORD_INVALID",
      message: "next terminal closeout record is invalid",
    };
  }
  const nextErrors = validateTerminalCloseoutRecord(next);
  if (nextErrors.length > 0) {
    return {
      ok: false,
      code: "TERMINAL_CLOSEOUT_RECORD_INVALID",
      message: nextErrors.join("; "),
    };
  }
  if (!current) {
    return {
      ok: true,
      code: "TERMINAL_CLOSEOUT_RECORD_CREATE",
      record: next,
    };
  }

  const currentRank = terminalCloseoutStateRank(current.terminal_state);
  const nextRank = terminalCloseoutStateRank(next.terminal_state);
  if (nextRank < currentRank) {
    return {
      ok: false,
      code: "TERMINAL_STATE_DOWNGRADE_REJECTED",
      message: `refusing terminal state downgrade ${current.terminal_state} -> ${next.terminal_state}`,
      current,
      next,
    };
  }
  if (terminalVerdictConflict(current.verdict, next.verdict)) {
    return {
      ok: false,
      code: "TERMINAL_VERDICT_CONFLICT",
      message: `refusing terminal verdict conflict ${current.verdict} -> ${next.verdict}`,
      current,
      next,
    };
  }
  if (TERMINAL_VERDICTS.has(current.verdict) && next.verdict === "UNKNOWN") {
    return {
      ok: false,
      code: "TERMINAL_VERDICT_ERASURE_REJECTED",
      message: `refusing to erase terminal verdict ${current.verdict}`,
      current,
      next,
    };
  }

  const currentUpdated = timestampMillis(current.updated_at_utc);
  const nextUpdated = timestampMillis(next.updated_at_utc);
  if (currentUpdated !== null && nextUpdated !== null && nextUpdated < currentUpdated) {
    return {
      ok: false,
      code: "TERMINAL_STALE_WRITER_REJECTED",
      message: `refusing stale terminal writer ${next.updated_at_utc} older than ${current.updated_at_utc}`,
      current,
      next,
    };
  }

  const currentProductBlockers = uniqueStrings(current.product_outcome_blockers);
  const nextProductBlockers = uniqueStrings(next.product_outcome_blockers);
  if (currentRank >= 1 && currentProductBlockers.length === 0 && nextProductBlockers.length > 0 && nextRank <= currentRank) {
    return {
      ok: false,
      code: "TERMINAL_PRODUCT_OUTCOME_WEAKENING_REJECTED",
      message: "refusing to weaken established product outcome authority with same-rank product blockers",
      current,
      next,
    };
  }

  return {
    ok: true,
    code: currentRank === nextRank ? "TERMINAL_CLOSEOUT_RECORD_UPDATE" : "TERMINAL_CLOSEOUT_RECORD_ADVANCE",
    record: {
      ...next,
      previous_terminal_state: current.terminal_state,
      recorded_at_utc: current.recorded_at_utc || next.recorded_at_utc,
    },
    current,
  };
}

export function publishTerminalCloseoutRecord({
  wpId = "",
  record = null,
  recordPathAbs = "",
  existingRecord = undefined,
} = {}) {
  const pathAbs = recordPathAbs || terminalCloseoutRecordAbsPath(wpId || record?.wp_id);
  const readResult = existingRecord === undefined
    ? readTerminalCloseoutRecord({
      wpId: wpId || record?.wp_id,
      recordPathAbs: pathAbs,
    })
    : {
      status: existingRecord ? "PRESENT" : "ABSENT",
      path: pathAbs,
      record: existingRecord,
      errors: [],
    };
  if (readResult.status === "INVALID") {
    const error = new Error(`Existing terminal closeout record is invalid: ${readResult.errors.join("; ")}`);
    error.code = "TERMINAL_CLOSEOUT_RECORD_INVALID";
    error.details = readResult.errors;
    throw error;
  }

  const decision = resolveTerminalCloseoutPublication({
    currentRecord: readResult.record,
    nextRecord: record,
  });
  if (!decision.ok) {
    const error = new Error(decision.message || decision.code);
    error.code = decision.code;
    error.current = decision.current;
    error.next = decision.next;
    throw error;
  }

  writeJsonFile(pathAbs, decision.record);
  return {
    ok: true,
    code: decision.code,
    path: pathAbs,
    record: decision.record,
  };
}
