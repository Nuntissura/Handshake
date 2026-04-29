import fs from "node:fs";
import path from "node:path";

import { parseMergeProgressionTruth } from "./merge-progression-truth-lib.mjs";
import { evaluatePacketRuntimeProjectionDrift, parseRuntimeProjectionFromPacket } from "./packet-runtime-projection-lib.mjs";
import { parseSignedScopeCompatibilityTruth, validateSignedScopeCompatibilityTruth } from "./signed-scope-compatibility-lib.mjs";
import {
  GOVERNANCE_RUNTIME_ROOT_ABS,
  REPO_ROOT,
  governanceRuntimeAbsPath,
  normalizePath,
  repoPathAbs,
  resolveWorkPacketPath,
} from "./runtime-paths.mjs";
import { taskBoardStatus } from "./role-resume-utils.mjs";
import { parseJsonFile, parseJsonlFile } from "./wp-communications-lib.mjs";
import { loadSessionControlRequests, loadSessionControlResults, loadSessionRegistry, writeJsonFile } from "../session/session-registry-lib.mjs";
import { evaluateWpTokenBudget } from "../session/wp-token-budget-lib.mjs";
import { readWpTokenUsageLedger } from "../session/wp-token-usage-lib.mjs";
import { evaluateWpRepomemCoverage } from "../memory/repomem-coverage-lib.mjs";
import { resolveValidatorGatePath } from "./validator-gate-paths.mjs";

export const WP_TRUTH_BUNDLE_SCHEMA_ID = "hsk.wp_truth_bundle@1";
export const WP_TRUTH_BUNDLE_SCHEMA_VERSION = "wp_truth_bundle_v1";
export const WP_TRUTH_BUNDLE_MAX_COMPACT_LINES = 80;

function nowIso() {
  return new Date().toISOString();
}

function normalizeText(value = "") {
  return String(value || "").trim();
}

function parseSingleField(text, label) {
  const re = new RegExp(`^\\s*-\\s*(?:\\*\\*)?${label}(?:\\*\\*)?\\s*:\\s*(.+)\\s*$`, "mi");
  const match = String(text || "").match(re);
  return match ? match[1].trim() : "";
}

function safeReadJson(filePath = "", fallback = null) {
  const normalized = normalizeText(filePath);
  if (!normalized) return fallback;
  const absPath = path.isAbsolute(normalized) ? normalized : repoPathAbs(normalized);
  if (!fs.existsSync(absPath)) return fallback;
  try {
    return parseJsonFile(absPath);
  } catch {
    return fallback;
  }
}

function safeReadJsonl(filePath = "") {
  const normalized = normalizeText(filePath);
  if (!normalized) return [];
  const absPath = path.isAbsolute(normalized) ? normalized : repoPathAbs(normalized);
  if (!fs.existsSync(absPath)) return [];
  try {
    return parseJsonlFile(absPath);
  } catch {
    return [];
  }
}

function latestByTimestamp(entries = [], field = "timestamp_utc") {
  return [...(Array.isArray(entries) ? entries : [])]
    .sort((left, right) => String(right?.[field] || "").localeCompare(String(left?.[field] || "")))[0] || null;
}

function summarizeSessions(sessions = [], wpId = "") {
  const normalizedWpId = normalizeText(wpId);
  const wpSessions = (Array.isArray(sessions) ? sessions : [])
    .filter((session) => normalizeText(session?.wp_id) === normalizedWpId);
  const activeStates = new Set(["PLUGIN_REQUESTED", "TERMINAL_COMMAND_DISPATCHED", "PLUGIN_CONFIRMED", "CLI_ESCALATION_READY", "CLI_ESCALATION_USED", "STARTING", "READY", "COMMAND_RUNNING", "ACTIVE", "WAITING"]);
  const queued = wpSessions.filter((session) => Array.isArray(session?.pending_control_queue) && session.pending_control_queue.length > 0);
  const active = wpSessions.filter((session) => activeStates.has(normalizeText(session?.runtime_state).toUpperCase()));
  const stale = wpSessions.filter((session) => ["STALE", "FAILED"].includes(normalizeText(session?.runtime_state).toUpperCase()));
  const closed = wpSessions.filter((session) => normalizeText(session?.runtime_state).toUpperCase() === "CLOSED");
  const terminalResidue = wpSessions.filter((session) =>
    ["READY", "ACTIVE", "WAITING", "COMMAND_RUNNING"].includes(normalizeText(session?.runtime_state).toUpperCase())
  );
  return {
    total: wpSessions.length,
    active: active.length,
    queued: queued.length,
    stale: stale.length,
    closed: closed.length,
    terminal_residue: terminalResidue.length,
    active_sessions: active.map((session) => session.session_key).filter(Boolean),
    queued_sessions: queued.map((session) => session.session_key).filter(Boolean),
    stale_sessions: stale.map((session) => session.session_key).filter(Boolean),
    terminal_residue_sessions: terminalResidue.map((session) => session.session_key).filter(Boolean),
  };
}

function deriveFinalVerdict({ packetText = "", runtimeStatus = {}, taskBoard = "" } = {}) {
  const mergeTruth = parseMergeProgressionTruth(packetText);
  const runtimeVerdict = normalizeText(runtimeStatus?.validation_verdict || runtimeStatus?.final_verdict).toUpperCase();
  const board = normalizeText(taskBoard).toUpperCase();
  if (mergeTruth.validationVerdict) return mergeTruth.validationVerdict;
  if (["PASS", "FAIL", "OUTDATED_ONLY", "ABANDONED"].includes(runtimeVerdict)) return runtimeVerdict;
  if (board === "VALIDATED") return "PASS";
  if (["FAIL", "OUTDATED_ONLY", "ABANDONED"].includes(board)) return board;
  return "UNKNOWN";
}

function candidateCommitFromRuntime(runtimeStatus = {}, receipts = []) {
  const runtimeCandidate = normalizeText(
    runtimeStatus?.target_head_sha
    || runtimeStatus?.candidate_commit
    || runtimeStatus?.committed_target_head_sha
    || runtimeStatus?.execution_state?.target_head_sha,
  );
  if (runtimeCandidate) return runtimeCandidate;
  const handoff = [...(Array.isArray(receipts) ? receipts : [])]
    .reverse()
    .find((entry) =>
      ["CODER_HANDOFF", "HANDOFF", "STATUS"].includes(normalizeText(entry?.receipt_kind).toUpperCase())
      && normalizeText(entry?.verb_body?.commit || entry?.commit || entry?.target_head_sha)
    );
  return normalizeText(handoff?.verb_body?.commit || handoff?.commit || handoff?.target_head_sha);
}

function sessionCoverageStatus(repomemCoverage = null) {
  const state = normalizeText(repomemCoverage?.state || "UNASSESSED").toUpperCase();
  if (state === "PASS") return "PASS";
  if (state === "NO_ACTIVE_ROLES") return "NO_ACTIVE_ROLES";
  if (state === "DEBT") return "DEBT";
  return "UNASSESSED";
}

function deriveBlockers({ signedScopeValidation = null, drift = null, repomemCoverage = null, runtimeStatus = {} } = {}) {
  const productBlockers = [];
  const governanceDebtKeys = [];
  for (const error of signedScopeValidation?.errors || []) {
    if (/PASS-ready|candidate|signed-scope|CURRENT_MAIN_COMPATIBILITY|PACKET_WIDENING/i.test(error)) {
      productBlockers.push(error);
    }
  }
  if (drift && !drift.ok) {
    governanceDebtKeys.push(...(drift.owner_classes || ["RUNTIME_PROJECTION_DRIFT"]));
  }
  if (sessionCoverageStatus(repomemCoverage) === "DEBT") {
    governanceDebtKeys.push(...(repomemCoverage.debt_keys || ["REPOMEM_COVERAGE_DEBT"]));
  }
  if (runtimeStatus?.attention_required) {
    governanceDebtKeys.push("RUNTIME_ATTENTION_REQUIRED");
  }
  return {
    open_product_blockers: [...new Set(productBlockers)],
    governance_debt_keys: [...new Set(governanceDebtKeys.map((value) => normalizeText(value)).filter(Boolean))],
  };
}

function deriveExactNextCommand(bundle = {}) {
  const wpId = bundle.wp_id;
  const finalVerdict = normalizeText(bundle.final_verdict).toUpperCase();
  if (finalVerdict === "PASS" && bundle.product_main_containment_status === "MERGE_PENDING") {
    return `just phase-check CLOSEOUT ${wpId} --sync-mode CONTAINED_IN_MAIN --merged-main-sha <MERGED_MAIN_SHA> --context "<why contained-main closure is now valid, >=40 chars>"`;
  }
  if (["PASS", "FAIL", "OUTDATED_ONLY", "ABANDONED"].includes(finalVerdict) && bundle.task_board_status !== "IN_PROGRESS") {
    return `just phase-check CLOSEOUT ${wpId}`;
  }
  if (bundle.next_actor && !["NONE", "ORCHESTRATOR"].includes(bundle.next_actor)) {
    return `just orchestrator-steer-next ${wpId} "<why this governed role is the next legal actor, >=40 chars>"`;
  }
  if (bundle.governance_debt_keys.length > 0) {
    return `just closeout-repair ${wpId}`;
  }
  return `just orchestrator-next ${wpId}`;
}

export function truthBundleOutputPath(wpId = "", timestamp = nowIso(), { runtimeRootAbs = GOVERNANCE_RUNTIME_ROOT_ABS } = {}) {
  const safeTimestamp = String(timestamp || nowIso()).replace(/[:.]/g, "-");
  return path.join(path.resolve(runtimeRootAbs), "roles_shared", "WP_COMMUNICATIONS", wpId, "truth_bundle", `${safeTimestamp}.json`);
}

export function buildWpTruthBundle({
  repoRoot = REPO_ROOT,
  wpId = "",
  packetText = null,
  runtimeStatus = null,
  sessions = null,
  controlRequests = null,
  controlResults = null,
  receipts = null,
  notifications = null,
  writeDetail = true,
  runtimeRootAbs = GOVERNANCE_RUNTIME_ROOT_ABS,
  timestamp = nowIso(),
} = {}) {
  const normalizedWpId = normalizeText(wpId);
  if (!/^WP-/.test(normalizedWpId)) {
    return {
      ok: false,
      error: "WP_ID is required and must start with WP-",
    };
  }

  const resolvedPacket = resolveWorkPacketPath(normalizedWpId);
  const packetPath = resolvedPacket?.packetPath || "";
  const packetAbsPath = resolvedPacket?.packetAbsPath || "";
  const hasPacket = packetText !== null || (packetAbsPath && fs.existsSync(packetAbsPath));
  if (!hasPacket) {
    return {
      ok: false,
      error: `No active or archived work packet found for ${normalizedWpId}`,
      wp_id: normalizedWpId,
    };
  }

  const text = packetText !== null ? String(packetText || "") : fs.readFileSync(packetAbsPath, "utf8");
  const runtimePath = parseSingleField(text, "WP_RUNTIME_STATUS_FILE");
  const receiptsPath = parseSingleField(text, "WP_RECEIPTS_FILE");
  const notificationsPath = parseSingleField(text, "WP_NOTIFICATIONS_FILE");
  const runtime = runtimeStatus !== null ? (runtimeStatus || {}) : safeReadJson(runtimePath, {});
  const resolvedReceipts = receipts !== null ? receipts : safeReadJsonl(receiptsPath);
  const resolvedNotifications = notifications !== null ? notifications : safeReadJsonl(notificationsPath);
  const boardStatus = taskBoardStatus(normalizedWpId) || normalizeText(runtime?.current_task_board_status) || "UNKNOWN";
  const runtimeProjection = runtime && Object.keys(runtime).length > 0
    ? evaluatePacketRuntimeProjectionDrift(text, runtime)
    : null;
  const signedScope = parseSignedScopeCompatibilityTruth(text);
  const signedScopeValidation = validateSignedScopeCompatibilityTruth(text, {
    packetPath: packetPath || `<${normalizedWpId}>`,
    requireReadyForPass: /^Validated\s+\(PASS\)|Done$/i.test(parseSingleField(text, "Status")),
  });
  const { registry } = sessions === null ? loadSessionRegistry(repoRoot) : { registry: { sessions } };
  const resolvedRequests = controlRequests === null ? loadSessionControlRequests(repoRoot).requests : controlRequests;
  const resolvedResults = controlResults === null ? loadSessionControlResults(repoRoot).results : controlResults;
  const repomemCoverage = evaluateWpRepomemCoverage({
    repoRoot,
    wpId: normalizedWpId,
    packetContent: text,
    receipts: resolvedReceipts,
    sessions: registry.sessions || [],
    controlRequests: resolvedRequests,
    controlResults: resolvedResults,
  });
  const sessionSummary = summarizeSessions(registry.sessions || [], normalizedWpId);
  const tokenLedgerResult = readWpTokenUsageLedger(repoRoot, normalizedWpId);
  const tokenBudget = evaluateWpTokenBudget(tokenLedgerResult.ledger);
  const validatorGatePath = resolveValidatorGatePath(normalizedWpId);
  const validatorGate = safeReadJson(validatorGatePath, {});
  const latestReceipt = latestByTimestamp(resolvedReceipts);
  const latestNotification = latestByTimestamp(resolvedNotifications);
  const mergeTruth = parseMergeProgressionTruth(text);
  const finalVerdict = deriveFinalVerdict({ packetText: text, runtimeStatus: runtime, taskBoard: boardStatus });
  const projection = parseRuntimeProjectionFromPacket(text);
  const blockerSummary = deriveBlockers({
    signedScopeValidation,
    drift: runtimeProjection,
    repomemCoverage,
    runtimeStatus: runtime,
  });
  const bundle = {
    schema_id: WP_TRUTH_BUNDLE_SCHEMA_ID,
    schema_version: WP_TRUTH_BUNDLE_SCHEMA_VERSION,
    generated_at_utc: timestamp,
    wp_id: normalizedWpId,
    packet_path: packetPath,
    packet_status: parseSingleField(text, "Status") || projection.current_packet_status || "UNKNOWN",
    runtime_status: normalizeText(runtime?.runtime_status || "UNKNOWN"),
    task_board_status: boardStatus,
    active_mt: normalizeText(runtime?.active_microtask || runtime?.active_mt || ""),
    next_mt: normalizeText(runtime?.next_microtask || runtime?.next_mt || ""),
    next_actor: normalizeText(runtime?.next_expected_actor || "UNKNOWN").toUpperCase(),
    waiting_on: normalizeText(runtime?.waiting_on || "UNKNOWN").toUpperCase(),
    validator_gate_state: {
      path: validatorGatePath,
      status: validatorGate?.state || validatorGate?.status || (Object.keys(validatorGate).length > 0 ? "RECORDED" : "MISSING"),
      verdict: validatorGate?.verdict || validatorGate?.latest_verdict || null,
    },
    final_verdict: finalVerdict,
    candidate_commit: candidateCommitFromRuntime(runtime, resolvedReceipts),
    candidate_branch: normalizeText(runtime?.candidate_branch || parseSingleField(text, "LOCAL_BRANCH") || ""),
    signed_scope_status: signedScope.currentMainCompatibilityStatus || "UNKNOWN",
    signed_scope: signedScope,
    signed_scope_errors: signedScopeValidation.errors || [],
    product_main_containment_status: mergeTruth.mainContainmentStatus || projection.main_containment_status || "UNKNOWN",
    closeout_dependency_summary: runtimeProjection
      ? `runtime_projection=${runtimeProjection.ok ? "PASS" : "DRIFT"} | issues=${runtimeProjection.issues?.length || 0}`
      : "runtime_projection=UNASSESSED",
    session_summary: sessionSummary,
    repomem_coverage_status: sessionCoverageStatus(repomemCoverage),
    repomem_coverage: repomemCoverage,
    token_budget_status: tokenBudget.status,
    token_budget_summary: tokenBudget.summary,
    receipt_summary: {
      count: resolvedReceipts.length,
      latest_kind: latestReceipt?.receipt_kind || "NONE",
      latest_at: latestReceipt?.timestamp_utc || null,
    },
    notification_summary: {
      count: resolvedNotifications.length,
      latest_kind: latestNotification?.source_kind || "NONE",
      latest_at: latestNotification?.timestamp_utc || null,
    },
    ...blockerSummary,
    artifact_path: "",
    exact_next_command: "",
  };
  bundle.exact_next_command = deriveExactNextCommand(bundle);

  if (writeDetail) {
    const outPath = truthBundleOutputPath(normalizedWpId, timestamp, { runtimeRootAbs });
    writeJsonFile(outPath, bundle);
    bundle.artifact_path = normalizePath(outPath);
  } else {
    bundle.artifact_path = normalizePath(truthBundleOutputPath(normalizedWpId, timestamp, { runtimeRootAbs }));
  }

  return {
    ok: true,
    bundle,
  };
}

export function formatWpTruthBundleCompact(bundle = {}) {
  const lines = [
    "WP_TRUTH_BUNDLE",
    `- wp_id: ${bundle.wp_id || "<missing>"}`,
    `- packet_status: ${bundle.packet_status || "UNKNOWN"}`,
    `- runtime_status: ${bundle.runtime_status || "UNKNOWN"}`,
    `- task_board_status: ${bundle.task_board_status || "UNKNOWN"}`,
    `- active_mt: ${bundle.active_mt || "NONE"}`,
    `- next_mt: ${bundle.next_mt || "NONE"}`,
    `- next_actor: ${bundle.next_actor || "UNKNOWN"}`,
    `- waiting_on: ${bundle.waiting_on || "UNKNOWN"}`,
    `- validator_gate_state: ${bundle.validator_gate_state?.status || "UNKNOWN"}`,
    `- final_verdict: ${bundle.final_verdict || "UNKNOWN"}`,
    `- candidate_commit: ${bundle.candidate_commit || "UNKNOWN"}`,
    `- candidate_branch: ${bundle.candidate_branch || "UNKNOWN"}`,
    `- signed_scope_status: ${bundle.signed_scope_status || "UNKNOWN"}`,
    `- product_main_containment_status: ${bundle.product_main_containment_status || "UNKNOWN"}`,
    `- closeout_dependency_summary: ${bundle.closeout_dependency_summary || "UNKNOWN"}`,
    `- sessions: active=${bundle.session_summary?.active ?? 0} queued=${bundle.session_summary?.queued ?? 0} stale=${bundle.session_summary?.stale ?? 0} terminal_residue=${bundle.session_summary?.terminal_residue ?? 0}`,
    `- repomem_coverage_status: ${bundle.repomem_coverage_status || "UNKNOWN"}`,
    `- token_budget_status: ${bundle.token_budget_status || "UNKNOWN"}`,
    `- open_product_blockers: ${bundle.open_product_blockers?.length ? bundle.open_product_blockers.join(" | ") : "NONE"}`,
    `- governance_debt_keys: ${bundle.governance_debt_keys?.length ? bundle.governance_debt_keys.join(",") : "NONE"}`,
    `- artifact_path: ${bundle.artifact_path || "NOT_WRITTEN"}`,
    `- exact_next_command: ${bundle.exact_next_command || "UNKNOWN"}`,
  ];
  return `${lines.slice(0, WP_TRUTH_BUNDLE_MAX_COMPACT_LINES).join("\n")}\n`;
}

