import fs from "node:fs";
import path from "node:path";
import {
  parseJsonFile,
  parseJsonlFile,
  REVIEW_OPEN_RECEIPT_KIND_VALUES,
  REVIEW_RESOLUTION_RECEIPT_KIND_VALUES,
} from "../lib/wp-communications-lib.mjs";
import {
  repoPathAbs,
  REPO_ROOT,
  resolveWorkPacketPath,
} from "../lib/runtime-paths.mjs";
import { resolveValidatorGatePath } from "../lib/validator-gate-paths.mjs";
import { renderInterRoleVerbReceipt } from "../lib/inter-role-verb-lib.mjs";
import {
  SESSION_CONTROL_REQUESTS_FILE,
  SESSION_CONTROL_RESULTS_FILE,
} from "./session-policy.mjs";
import {
  evaluateWpTokenBudget,
} from "./wp-token-budget-lib.mjs";
import {
  readWpTokenUsageLedger,
} from "./wp-token-usage-lib.mjs";

function parseSingleField(text, label) {
  const re = new RegExp(`^\\s*-\\s*(?:\\*\\*)?${label}(?:\\*\\*)?\\s*:\\s*(.+)\\s*$`, "mi");
  const match = String(text || "").match(re);
  return match ? match[1].trim() : "";
}

function normalizeText(value) {
  return String(value || "").trim();
}

function normalizeRole(value) {
  return normalizeText(value).toUpperCase();
}

function normalizePath(value) {
  return String(value || "").replace(/\\/g, "/");
}

function parseTimestamp(value) {
  const text = normalizeText(value);
  if (!text) return null;
  const parsed = Date.parse(text);
  return Number.isNaN(parsed) ? null : parsed;
}

function isoOrNull(value) {
  return Number.isFinite(value) ? new Date(value).toISOString() : null;
}

function durationMs(startTs, endTs) {
  if (!Number.isFinite(startTs) || !Number.isFinite(endTs) || endTs < startTs) return null;
  return endTs - startTs;
}

export const WORKFLOW_DOSSIER_IDLE_THRESHOLD_MS = 15 * 60 * 1000;

function compactLabel(value, fallback = "<none>") {
  const text = normalizeText(value);
  return text || fallback;
}

function normalizeReceiptKind(value) {
  return normalizeText(value).toUpperCase();
}

function normalizeCommandKind(value) {
  return normalizeText(value).toUpperCase();
}

function microtaskScopeRef(entry = {}) {
  return compactLabel(entry?.microtask_contract?.scope_ref, "");
}

function microtaskReviewMode(entry = {}) {
  return normalizeText(entry?.microtask_contract?.review_mode).toUpperCase();
}

function microtaskPhaseGate(entry = {}) {
  return normalizeText(entry?.microtask_contract?.phase_gate).toUpperCase();
}

function buildSpanId(sequence) {
  return `SPN-${String(sequence).padStart(4, "0")}`;
}

function deriveControlStage(commandKind = "") {
  switch (normalizeCommandKind(commandKind)) {
    case "START_SESSION":
      return "LAUNCH";
    case "SEND_PROMPT":
      return "RELAY";
    case "CLOSE_SESSION":
    case "CANCEL_SESSION":
      return "CLOSEOUT";
    default:
      return "CONTROL";
  }
}

function deriveReceiptStage(receiptKind = "", entry = {}) {
  const normalized = normalizeReceiptKind(receiptKind);
  if (normalized === "VALIDATOR_KICKOFF") return "BOOTSTRAP_GATE";
  if (normalized === "CODER_INTENT") return "MICROTASK_EXECUTION";
  if (normalized === "REPAIR") return "REPAIR_LOOP";
  if (normalized === "CODER_HANDOFF") return "HANDOFF";
  if (normalized === "REVIEW_REQUEST") {
    return microtaskReviewMode(entry) === "OVERLAP" ? "MICROTASK_REVIEW" : "REVIEW";
  }
  if (REVIEW_RESOLUTION_RECEIPT_KIND_VALUES.includes(normalized)) {
    return microtaskReviewMode(entry) === "OVERLAP" ? "MICROTASK_REVIEW" : "REVIEW";
  }
  return "WORKFLOW";
}

function detailLineIfValue(label, value) {
  const normalized = compactLabel(value, "");
  return normalized ? `${label}=${normalized}` : null;
}

function matchingResolutionForOpenReceipt(openReceipt, orderedResolutions) {
  const correlationId = compactLabel(openReceipt?.correlation_id, "");
  if (!correlationId) return null;
  const openActor = normalizeRole(openReceipt?.actor_role);
  const openTarget = normalizeRole(openReceipt?.target_role);
  const openTimestamp = parseTimestamp(openReceipt?.timestamp_utc);
  return orderedResolutions.find((entry) => {
    if (compactLabel(entry?.correlation_id, "") !== correlationId) return false;
    if (normalizeRole(entry?.actor_role) !== openTarget) return false;
    if (normalizeRole(entry?.target_role) !== openActor) return false;
    const resolutionTs = parseTimestamp(entry?.timestamp_utc);
    if (Number.isFinite(openTimestamp) && Number.isFinite(resolutionTs) && resolutionTs < openTimestamp) return false;
    return true;
  }) || null;
}

function matchingMicrotaskTerminalReceipt(startReceipt, orderedReceipts) {
  const startTimestamp = parseTimestamp(startReceipt?.timestamp_utc);
  const startScope = microtaskScopeRef(startReceipt);
  if (!startScope) return null;

  return orderedReceipts.find((entry) => {
    const entryTimestamp = parseTimestamp(entry?.timestamp_utc);
    if (!Number.isFinite(entryTimestamp) || (Number.isFinite(startTimestamp) && entryTimestamp < startTimestamp)) {
      return false;
    }
    if (entry === startReceipt) return false;

    const entryKind = normalizeReceiptKind(entry?.receipt_kind);
    const entryRole = normalizeRole(entry?.actor_role);
    const entryScope = microtaskScopeRef(entry);

    if (entryRole === "CODER" && ["CODER_INTENT", "REPAIR"].includes(entryKind) && entryScope && entryScope !== startScope) {
      return true;
    }
    if (entryScope && entryScope === startScope) {
      if (entryRole === "CODER" && ["REVIEW_REQUEST", "CODER_HANDOFF"].includes(entryKind)) return true;
      if (REVIEW_RESOLUTION_RECEIPT_KIND_VALUES.includes(entryKind)) return true;
    }
    if (!entryScope && entryRole === "CODER" && entryKind === "CODER_HANDOFF") {
      return true;
    }
    return false;
  }) || null;
}

function countByKind(spans = [], spanKind = "") {
  return spans.filter((entry) => entry.span_kind === spanKind).length;
}

function sumBy(spans = [], predicate = () => true, selector = () => 0) {
  return spans.reduce((total, entry) => (
    predicate(entry) ? total + Number(selector(entry) || 0) : total
  ), 0);
}

export function evaluateWpRelayCostPolicy({
  workflowLane = "",
  spans = [],
  tokenLedger = {},
} = {}) {
  const relayControlSpans = (Array.isArray(spans) ? spans : []).filter((entry) =>
    entry?.span_kind === "CONTROL_COMMAND" && entry?.span_stage === "RELAY"
  );
  const relayTokenSpans = (Array.isArray(spans) ? spans : []).filter((entry) =>
    entry?.span_kind === "TOKEN_COMMAND" && entry?.span_stage === "RELAY"
  );
  const relayCommandIds = new Set([
    ...relayControlSpans.map((entry) => compactLabel(entry?.command_id, "")).filter(Boolean),
    ...relayTokenSpans.map((entry) => compactLabel(entry?.command_id, "")).filter(Boolean),
  ]);
  const relayDurationMs = sumBy(relayControlSpans, () => true, (entry) => entry.duration_ms);
  const relayTurnCount = sumBy(relayTokenSpans, () => true, (entry) => entry.turn_count);
  const relayInputTotal = sumBy(relayTokenSpans, () => true, (entry) => entry.token_input_total);
  const relayCachedInputTotal = sumBy(relayTokenSpans, () => true, (entry) => entry.token_cached_input_total);
  const relayOutputTotal = sumBy(relayTokenSpans, () => true, (entry) => entry.token_output_total);
  const totalInput = Number(tokenLedger?.summary?.usage_totals?.input_tokens || 0);
  const totalOutput = Number(tokenLedger?.summary?.usage_totals?.output_tokens || 0);
  const totalLiveTokens = totalInput + totalOutput;
  const relayLiveTokens = relayInputTotal + relayOutputTotal;
  const relayTokenShare = totalLiveTokens > 0 ? Number((relayLiveTokens / totalLiveTokens).toFixed(4)) : 0;

  let burdenLevel = "LOW";
  if (relayTurnCount >= 8 || relayDurationMs >= (15 * 60 * 1000) || relayTokenShare >= 0.35) {
    burdenLevel = "HIGH";
  } else if (relayTurnCount >= 4 || relayDurationMs >= (5 * 60 * 1000) || relayTokenShare >= 0.15) {
    burdenLevel = "MEDIUM";
  }

  const currentLane = compactLabel(workflowLane, "<missing>");
  const defaultLane = "ORCHESTRATOR_MANAGED";
  let recommendedLane = defaultLane;
  let recommendationReason = "Default to ORCHESTRATOR_MANAGED for future governed sessions; use MANUAL_RELAY deliberately when the operator explicitly wants the classic combined lane.";
  let assessment = "DEFAULT_ORCHESTRATOR_MANAGED";

  if (currentLane === "ORCHESTRATOR_MANAGED") {
    assessment = "ALIGNED_WITH_DEFAULT";
    recommendedLane = "ORCHESTRATOR_MANAGED";
    recommendationReason = burdenLevel === "HIGH"
      ? "Current lane already matches the future default, but observed relay burden is high; keep ORCHESTRATOR_MANAGED only when that autonomy is actually paying for itself."
      : burdenLevel === "MEDIUM"
        ? "Current lane already matches the future default; observed relay burden is visible but still within the intended managed-lane tradeoff."
        : "Current lane already matches the future default and relay burden is light.";
  } else if (currentLane === "MANUAL_RELAY") {
    assessment = burdenLevel === "HIGH"
      ? "CLASSIC_MANUAL_BURDEN_HIGH"
      : burdenLevel === "MEDIUM"
        ? "CLASSIC_MANUAL_BURDEN_VISIBLE"
        : "CLASSIC_MANUAL_BURDEN_LIGHT";
    recommendedLane = "ORCHESTRATOR_MANAGED";
    recommendationReason = burdenLevel === "HIGH"
      ? "Observed manual-relay burden is high; future runs should prefer ORCHESTRATOR_MANAGED unless the operator still wants explicit classic brokering."
      : burdenLevel === "MEDIUM"
        ? "Observed manual-relay burden is visible; future runs should generally prefer ORCHESTRATOR_MANAGED unless the operator explicitly wants the classic lane."
        : "Manual relay stayed light, but future-default lane policy still prefers ORCHESTRATOR_MANAGED unless the operator explicitly wants the classic path.";
  }

  return {
    current_lane: currentLane,
    default_lane: defaultLane,
    recommended_lane: recommendedLane,
    assessment,
    burden_level: burdenLevel,
    relay_command_count: relayCommandIds.size,
    relay_control_span_count: relayControlSpans.length,
    relay_token_span_count: relayTokenSpans.length,
    relay_turn_count: relayTurnCount,
    relay_duration_ms: relayDurationMs,
    relay_input_total: relayInputTotal,
    relay_cached_input_total: relayCachedInputTotal,
    relay_output_total: relayOutputTotal,
    relay_token_share: relayTokenShare,
    recommendation_reason: recommendationReason,
  };
}

export function parseThreadEntriesText(threadText = "") {
  const lines = String(threadText || "").split(/\r?\n/);
  const entries = [];
  let current = null;
  for (const line of lines) {
    if (/^\s*-\s+\d{4}-\d{2}-\d{2}T[^\s]+Z\s+\|/.test(line)) {
      const parts = line.replace(/^\s*-\s+/, "").split("|").map((value) => value.trim()).filter(Boolean);
      const [timestamp, actorRole, ...metadata] = parts;
      const entry = {
        timestamp: timestamp || "",
        actorRole: actorRole || "",
        actorSession: "",
        targetRole: "",
        targetSession: "",
        correlationId: "",
        specAnchor: "",
        packetRowRef: "",
        messageLines: [],
      };
      for (const item of metadata) {
        if (item.startsWith("session=")) entry.actorSession = item.slice("session=".length).trim();
        else if (item.startsWith("target_role=")) entry.targetRole = item.slice("target_role=".length).trim();
        else if (item.startsWith("target_session=")) entry.targetSession = item.slice("target_session=".length).trim();
        else if (item.startsWith("correlation_id=")) entry.correlationId = item.slice("correlation_id=".length).trim();
        else if (item.startsWith("spec_anchor=")) entry.specAnchor = item.slice("spec_anchor=".length).trim();
        else if (item.startsWith("packet_row_ref=")) entry.packetRowRef = item.slice("packet_row_ref=".length).trim();
      }
      if (current) entries.push(current);
      current = entry;
      continue;
    }
    if (current && /^\s{2,}\S/.test(line)) {
      current.messageLines.push(line.trim());
    }
  }
  if (current) entries.push(current);
  return entries;
}

function formatTarget(role, session) {
  const targetRole = normalizeText(role);
  const targetSession = normalizeText(session);
  if (!targetRole) return "";
  return targetSession ? `${targetRole}:${targetSession}` : targetRole;
}

export function buildWpTimelineEntries({
  threadEntries = [],
  receipts = [],
  notifications = [],
  controlRequests = [],
  controlResults = [],
  tokenCommands = [],
} = {}) {
  const entries = [];
  let sequence = 0;

  for (const entry of threadEntries) {
    const target = formatTarget(entry.targetRole, entry.targetSession);
    entries.push({
      timestamp: entry.timestamp || "",
      timestamp_ms: parseTimestamp(entry.timestamp),
      sequence: sequence += 1,
      kind: "THREAD",
      role: normalizeRole(entry.actorRole),
      header: `${entry.timestamp || "<no-ts>"} | THREAD | ${entry.actorRole || "<unknown>"}${entry.actorSession ? `:${entry.actorSession}` : ""}${target ? ` -> ${target}` : ""}`,
      detailLines: [
        ...(entry.messageLines?.length ? entry.messageLines : ["<no body>"]),
        ...(entry.correlationId ? [`corr=${entry.correlationId}`] : []),
        ...(entry.specAnchor ? [`spec=${entry.specAnchor}`] : []),
        ...(entry.packetRowRef ? [`packet=${entry.packetRowRef}`] : []),
      ],
    });
  }

  for (const entry of receipts) {
    const target = formatTarget(entry.target_role, entry.target_session);
    const verbLine = renderInterRoleVerbReceipt(entry);
    entries.push({
      timestamp: entry.timestamp_utc || "",
      timestamp_ms: parseTimestamp(entry.timestamp_utc),
      sequence: sequence += 1,
      kind: "RECEIPT",
      role: normalizeRole(entry.actor_role),
      header: `${entry.timestamp_utc || "<no-ts>"} | RECEIPT | ${entry.actor_role || "<unknown>"} | ${entry.receipt_kind || "<unknown>"}`,
      detailLines: [
        ...(verbLine ? [`verb=${verbLine}`] : []),
        entry.summary || "<no summary>",
        ...(target ? [`target=${target}`] : []),
        ...(entry.correlation_id ? [`corr=${entry.correlation_id}`] : []),
        ...(entry.spec_anchor ? [`spec=${entry.spec_anchor}`] : []),
        ...(entry.packet_row_ref ? [`packet=${entry.packet_row_ref}`] : []),
      ],
    });
  }

  for (const entry of notifications) {
    const target = formatTarget(entry.target_role, entry.target_session);
    entries.push({
      timestamp: entry.timestamp_utc || "",
      timestamp_ms: parseTimestamp(entry.timestamp_utc),
      sequence: sequence += 1,
      kind: "NOTIFICATION",
      role: normalizeRole(entry.source_role),
      header: `${entry.timestamp_utc || "<no-ts>"} | NOTIFICATION | ${entry.source_role || "<unknown>"} -> ${target || "<unknown>"}`,
      detailLines: [
        `${entry.source_kind || "THREAD_MESSAGE"} | ${entry.summary || "<no summary>"}`,
        ...(entry.correlation_id ? [`corr=${entry.correlation_id}`] : []),
      ],
    });
  }

  for (const entry of controlRequests) {
    entries.push({
      timestamp: entry.created_at || "",
      timestamp_ms: parseTimestamp(entry.created_at),
      sequence: sequence += 1,
      kind: "CONTROL_REQUEST",
      role: normalizeRole(entry.role),
      header: `${entry.created_at || "<no-ts>"} | CONTROL_REQUEST | ${entry.role || "<unknown>"} | ${entry.command_kind || "<unknown>"}`,
      detailLines: [
        entry.summary || String(entry.prompt || "").split(/\r?\n/, 1)[0] || "<no summary>",
        ...(entry.command_id ? [`command_id=${entry.command_id}`] : []),
      ],
    });
  }

  for (const entry of controlResults) {
    entries.push({
      timestamp: entry.processed_at || "",
      timestamp_ms: parseTimestamp(entry.processed_at),
      sequence: sequence += 1,
      kind: "CONTROL_RESULT",
      role: normalizeRole(entry.role),
      header: `${entry.processed_at || "<no-ts>"} | CONTROL_RESULT | ${entry.role || "<unknown>"} | ${entry.command_kind || "<unknown>"} | ${entry.status || "<unknown>"}`,
      detailLines: [
        entry.summary || entry.error || "<no summary>",
        ...(entry.command_id ? [`command_id=${entry.command_id}`] : []),
        ...(Number.isFinite(Number(entry.duration_ms)) ? [`duration_ms=${Number(entry.duration_ms)}`] : []),
      ],
    });
  }

  for (const command of tokenCommands) {
    const role = normalizeRole(command.role);
    const turnUsage = Array.isArray(command.turn_usage) ? command.turn_usage : [];
    for (const usageEntry of turnUsage) {
      entries.push({
        timestamp: usageEntry.timestamp || "",
        timestamp_ms: parseTimestamp(usageEntry.timestamp),
        sequence: sequence += 1,
        kind: "TURN_USAGE",
        role,
        header: `${usageEntry.timestamp || "<no-ts>"} | TURN_USAGE | ${role || "<unknown>"} | ${command.command_kind || "<unknown>"}`,
        detailLines: [
          `command_id=${command.command_id || "<missing>"}`,
          `input=${Number(usageEntry.input_tokens || 0)} | cached=${Number(usageEntry.cached_input_tokens || 0)} | output=${Number(usageEntry.output_tokens || 0)}`,
        ],
      });
    }
  }

  return entries.sort((left, right) =>
    String(left.timestamp || "").localeCompare(String(right.timestamp || ""))
    || (left.sequence - right.sequence)
  );
}

export function buildWpTimelineSpans({
  receipts = [],
  controlRequests = [],
  controlResults = [],
  tokenCommands = [],
} = {}) {
  const spans = [];
  let sequence = 0;
  const pushSpan = (span) => {
    const nextSequence = sequence += 1;
    spans.push({
      span_id: buildSpanId(nextSequence),
      sequence: nextSequence,
      ...span,
    });
  };
  const tokenCommandMap = new Map(
    (Array.isArray(tokenCommands) ? tokenCommands : [])
      .filter((entry) => compactLabel(entry.command_id, ""))
      .map((entry) => [compactLabel(entry.command_id, ""), entry]),
  );

  const resultByCommandId = new Map(
    (Array.isArray(controlResults) ? controlResults : [])
      .filter((entry) => compactLabel(entry.command_id, ""))
      .map((entry) => [compactLabel(entry.command_id, ""), entry]),
  );
  const requestByCommandId = new Map(
    (Array.isArray(controlRequests) ? controlRequests : [])
      .filter((entry) => compactLabel(entry.command_id, ""))
      .map((entry) => [compactLabel(entry.command_id, ""), entry]),
  );

  for (const request of Array.isArray(controlRequests) ? controlRequests : []) {
    const commandId = compactLabel(request.command_id, "");
    if (!commandId) continue;
    const result = resultByCommandId.get(commandId) || null;
    const tokenCommand = tokenCommandMap.get(commandId) || null;
    const startedAt = compactLabel(request.created_at, "");
    const endedAt = compactLabel(result?.processed_at, "");
    const startedTs = parseTimestamp(startedAt);
    const endedTs = parseTimestamp(endedAt);
    const turnUsage = Array.isArray(tokenCommand?.turn_usage) ? tokenCommand.turn_usage : [];
    const turnWindowStart = turnUsage.length > 0 ? compactLabel(turnUsage[0]?.timestamp, "") : "";
    const turnWindowEnd = turnUsage.length > 0 ? compactLabel(turnUsage.at(-1)?.timestamp, "") : "";
    const turnCount = Number(tokenCommand?.turn_count || 0);
    const usageTotals = tokenCommand?.usage_totals || {};
    const stage = deriveControlStage(request.command_kind);

    const measuredDuration = durationMs(startedTs, endedTs);
    const fallbackDuration = Number(result?.duration_ms || 0);
    pushSpan({
      span_kind: "CONTROL_COMMAND",
      span_stage: stage,
      started_at: startedAt || null,
      ended_at: endedAt || null,
      started_at_ms: startedTs,
      ended_at_ms: endedTs,
      duration_ms: measuredDuration ?? (fallbackDuration > 0 ? fallbackDuration : null),
      role: normalizeRole(request.role),
      actor_role: normalizeRole(request.role),
      actor_session: compactLabel(request.session_id, ""),
      command_id: commandId || null,
      command_kind: normalizeCommandKind(request.command_kind) || null,
      command_status: compactLabel(result?.status, "<pending>"),
      header: `${startedAt || "<no-ts>"} -> ${endedAt || "<open>"} | CONTROL_COMMAND | stage=${stage} | ${compactLabel(request.role)} | ${compactLabel(request.command_kind)}`,
      detailLines: [
        `command_id=${commandId}`,
        `status=${compactLabel(result?.status, "<pending>")}`,
        `summary=${compactLabel(result?.summary || request.summary || String(request.prompt || "").split(/\r?\n/, 1)[0])}`,
        `turn_count=${turnCount} | input=${Number(usageTotals.input_tokens || 0)} | cached=${Number(usageTotals.cached_input_tokens || 0)} | output=${Number(usageTotals.output_tokens || 0)}`,
        ...(turnWindowStart ? [`turn_window=${turnWindowStart} -> ${turnWindowEnd || turnWindowStart}`] : []),
      ],
    });
  }

  for (const tokenCommand of Array.isArray(tokenCommands) ? tokenCommands : []) {
    const commandId = compactLabel(tokenCommand.command_id, "");
    const turnUsage = Array.isArray(tokenCommand.turn_usage) ? tokenCommand.turn_usage : [];
    if (!commandId) continue;
    const request = requestByCommandId.get(commandId) || null;
    const result = resultByCommandId.get(commandId) || null;
    const startedAt = compactLabel(turnUsage[0]?.timestamp, "") || compactLabel(request?.created_at, "") || compactLabel(result?.processed_at, "");
    const endedAt = compactLabel(turnUsage.at(-1)?.timestamp, "") || compactLabel(result?.processed_at, "") || compactLabel(request?.created_at, "");
    if (!startedAt && !endedAt) continue;
    const startedTs = parseTimestamp(startedAt);
    const endedTs = parseTimestamp(endedAt);
    const usageTotals = tokenCommand.usage_totals || {};
    const stage = deriveControlStage(tokenCommand.command_kind);
    pushSpan({
      span_kind: "TOKEN_COMMAND",
      span_stage: stage,
      started_at: startedAt || null,
      ended_at: endedAt || null,
      started_at_ms: startedTs,
      ended_at_ms: endedTs,
      duration_ms: durationMs(startedTs, endedTs),
      role: normalizeRole(tokenCommand.role),
      actor_role: normalizeRole(tokenCommand.role),
      command_id: commandId,
      command_kind: normalizeCommandKind(tokenCommand.command_kind) || null,
      turn_count: Number(tokenCommand.turn_count || turnUsage.length || 0),
      token_input_total: Number(usageTotals.input_tokens || 0),
      token_cached_input_total: Number(usageTotals.cached_input_tokens || 0),
      token_output_total: Number(usageTotals.output_tokens || 0),
      header: `${startedAt || "<no-ts>"} -> ${endedAt || "<open>"} | TOKEN_COMMAND | stage=${stage} | ${compactLabel(tokenCommand.role)} | ${compactLabel(tokenCommand.command_kind)}`,
      detailLines: [
        `command_id=${commandId}`,
        `turn_count=${Number(tokenCommand.turn_count || turnUsage.length || 0)}`,
        `input=${Number(usageTotals.input_tokens || 0)} | cached=${Number(usageTotals.cached_input_tokens || 0)} | output=${Number(usageTotals.output_tokens || 0)}`,
      ],
    });
  }

  const openReceipts = (Array.isArray(receipts) ? receipts : [])
    .filter((entry) => REVIEW_OPEN_RECEIPT_KIND_VALUES.includes(normalizeReceiptKind(entry?.receipt_kind)))
    .sort((left, right) => String(left?.timestamp_utc || "").localeCompare(String(right?.timestamp_utc || "")));
  const resolutionReceipts = (Array.isArray(receipts) ? receipts : [])
    .filter((entry) => REVIEW_RESOLUTION_RECEIPT_KIND_VALUES.includes(normalizeReceiptKind(entry?.receipt_kind)))
    .sort((left, right) => String(left?.timestamp_utc || "").localeCompare(String(right?.timestamp_utc || "")));

  for (const openReceipt of openReceipts) {
    const resolution = matchingResolutionForOpenReceipt(openReceipt, resolutionReceipts);
    const startedAt = compactLabel(openReceipt?.timestamp_utc, "");
    const endedAt = compactLabel(resolution?.timestamp_utc, "");
    const startedTs = parseTimestamp(startedAt);
    const endedTs = parseTimestamp(endedAt);
    const spanStage = deriveReceiptStage(openReceipt?.receipt_kind, openReceipt);
    pushSpan({
      span_kind: "REVIEW_EXCHANGE",
      span_stage: spanStage,
      started_at: startedAt || null,
      ended_at: endedAt || null,
      started_at_ms: startedTs,
      ended_at_ms: endedTs,
      duration_ms: durationMs(startedTs, endedTs),
      role: normalizeRole(openReceipt?.actor_role),
      actor_role: normalizeRole(openReceipt?.actor_role),
      actor_session: compactLabel(openReceipt?.actor_session, ""),
      target_role: normalizeRole(openReceipt?.target_role),
      target_session: compactLabel(openReceipt?.target_session, ""),
      receipt_kind: normalizeReceiptKind(openReceipt?.receipt_kind) || null,
      resolution_receipt_kind: normalizeReceiptKind(resolution?.receipt_kind) || null,
      correlation_id: compactLabel(openReceipt?.correlation_id, "") || null,
      microtask_scope_ref: microtaskScopeRef(openReceipt) || null,
      review_mode: microtaskReviewMode(openReceipt) || null,
      phase_gate: microtaskPhaseGate(openReceipt) || null,
      header: `${startedAt || "<no-ts>"} -> ${endedAt || "<open>"} | REVIEW_EXCHANGE | stage=${spanStage} | ${compactLabel(openReceipt?.receipt_kind)} | ${compactLabel(openReceipt?.actor_role)} -> ${compactLabel(openReceipt?.target_role)}`,
      detailLines: [
        `correlation_id=${compactLabel(openReceipt?.correlation_id)}`,
        `open_summary=${compactLabel(openReceipt?.summary)}`,
        `resolution_kind=${compactLabel(resolution?.receipt_kind, "<open>")}`,
        `resolution_summary=${compactLabel(resolution?.summary, "<pending>")}`,
        ...[
          detailLineIfValue("microtask_scope_ref", microtaskScopeRef(openReceipt)),
          detailLineIfValue("review_mode", microtaskReviewMode(openReceipt)),
          detailLineIfValue("phase_gate", microtaskPhaseGate(openReceipt)),
        ].filter(Boolean),
      ],
    });
  }

  const orderedReceipts = [...(Array.isArray(receipts) ? receipts : [])]
    .sort((left, right) => String(left?.timestamp_utc || "").localeCompare(String(right?.timestamp_utc || "")));
  const executionStarts = orderedReceipts.filter((entry) =>
    normalizeRole(entry?.actor_role) === "CODER"
    && ["CODER_INTENT", "REPAIR"].includes(normalizeReceiptKind(entry?.receipt_kind))
    && microtaskScopeRef(entry)
  );

  for (const startReceipt of executionStarts) {
    const endReceipt = matchingMicrotaskTerminalReceipt(startReceipt, orderedReceipts);
    const startedAt = compactLabel(startReceipt?.timestamp_utc, "");
    const endedAt = compactLabel(endReceipt?.timestamp_utc, "");
    const startedTs = parseTimestamp(startedAt);
    const endedTs = parseTimestamp(endedAt);
    const stage = deriveReceiptStage(startReceipt?.receipt_kind, startReceipt);
    pushSpan({
      span_kind: "MICROTASK_EXECUTION",
      span_stage: stage,
      started_at: startedAt || null,
      ended_at: endedAt || null,
      started_at_ms: startedTs,
      ended_at_ms: endedTs,
      duration_ms: durationMs(startedTs, endedTs),
      role: normalizeRole(startReceipt?.actor_role),
      actor_role: normalizeRole(startReceipt?.actor_role),
      actor_session: compactLabel(startReceipt?.actor_session, ""),
      receipt_kind: normalizeReceiptKind(startReceipt?.receipt_kind) || null,
      terminal_receipt_kind: normalizeReceiptKind(endReceipt?.receipt_kind) || null,
      correlation_id: compactLabel(startReceipt?.correlation_id, "") || null,
      microtask_scope_ref: microtaskScopeRef(startReceipt) || null,
      review_mode: microtaskReviewMode(startReceipt) || null,
      phase_gate: microtaskPhaseGate(startReceipt) || null,
      header: `${startedAt || "<no-ts>"} -> ${endedAt || "<open>"} | MICROTASK_EXECUTION | ${microtaskScopeRef(startReceipt) || "<unknown>"}`,
      detailLines: [
        `start_kind=${compactLabel(startReceipt?.receipt_kind)}`,
        `start_summary=${compactLabel(startReceipt?.summary)}`,
        `end_kind=${compactLabel(endReceipt?.receipt_kind, "<open>")}`,
        `end_summary=${compactLabel(endReceipt?.summary, "<pending>")}`,
        ...[
          detailLineIfValue("review_mode", microtaskReviewMode(startReceipt)),
          detailLineIfValue("phase_gate", microtaskPhaseGate(startReceipt)),
          detailLineIfValue("correlation_id", compactLabel(startReceipt?.correlation_id, "")),
        ].filter(Boolean),
      ],
    });
  }

  return spans.sort((left, right) =>
    String(left.started_at || "").localeCompare(String(right.started_at || ""))
    || (left.sequence - right.sequence)
  );
}

export function buildWpTimelineSummary({
  wpId = "",
  packetPath = "",
  workflowLane = "",
  runtimeStatus = null,
  receipts = [],
  notifications = [],
  controlRequests = [],
  controlResults = [],
  laneVerdict = null,
  pendingNotificationCount = null,
  tokenLedger = {},
  entries = [],
  spans = [],
} = {}) {
  const timestamps = entries
    .map((entry) => entry.timestamp_ms)
    .filter((value) => Number.isFinite(value))
    .sort((left, right) => left - right);
  const firstEventAt = timestamps.length > 0 ? timestamps[0] : null;
  const lastEventAt = timestamps.length > 0 ? timestamps[timestamps.length - 1] : null;
  const spanStartedTimestamps = spans
    .map((entry) => entry.started_at_ms)
    .filter((value) => Number.isFinite(value))
    .sort((left, right) => left - right);
  const spanEndedTimestamps = spans
    .map((entry) => entry.ended_at_ms)
    .filter((value) => Number.isFinite(value))
    .sort((left, right) => left - right);
  const stageCounts = {};
  let measuredSpanDurationMs = 0;
  for (const span of spans) {
    const stage = compactLabel(span?.span_stage, "UNKNOWN");
    stageCounts[stage] = Number(stageCounts[stage] || 0) + 1;
    if (Number.isFinite(span?.duration_ms)) measuredSpanDurationMs += Number(span.duration_ms);
  }
  const tokenBudget = evaluateWpTokenBudget(tokenLedger);
  const tokenInputTotal = Number(tokenLedger?.summary?.usage_totals?.input_tokens || 0);
  const tokenCachedInputTotal = Number(tokenLedger?.summary?.usage_totals?.cached_input_tokens || 0);
  const tokenFreshInputTotal = Math.max(0, tokenInputTotal - tokenCachedInputTotal);
  const tokenOutputTotal = Number(tokenLedger?.summary?.usage_totals?.output_tokens || 0);
  const tokenTurnCount = Number(tokenLedger?.summary?.turn_count || 0);
  const tokenCommandCount = Number(tokenLedger?.summary?.command_count || 0);
  const relayPolicy = evaluateWpRelayCostPolicy({
    workflowLane,
    spans,
    tokenLedger,
  });
  const idleMetrics = buildWorkflowDossierIdleMetrics({
    entries,
    spans,
    receipts,
    notifications,
    controlRequests,
    controlResults,
    runtimeStatus,
    laneVerdict,
    pendingNotificationCount,
  });

  return {
    wp_id: wpId,
    packet_path: packetPath,
    workflow_lane: workflowLane || "<missing>",
    runtime_status: runtimeStatus?.runtime_status || "<missing>",
    current_phase: runtimeStatus?.current_phase || "<missing>",
    next_expected_actor: runtimeStatus?.next_expected_actor || "<missing>",
    waiting_on: runtimeStatus?.waiting_on || "<missing>",
    event_window_start: isoOrNull(firstEventAt),
    event_window_end: isoOrNull(lastEventAt),
    event_window_duration_ms: durationMs(firstEventAt, lastEventAt),
    span_window_start: isoOrNull(spanStartedTimestamps.length > 0 ? spanStartedTimestamps[0] : null),
    span_window_end: isoOrNull(spanEndedTimestamps.length > 0 ? spanEndedTimestamps.at(-1) : null),
    span_window_duration_ms: durationMs(
      spanStartedTimestamps.length > 0 ? spanStartedTimestamps[0] : null,
      spanEndedTimestamps.length > 0 ? spanEndedTimestamps.at(-1) : null,
    ),
    measured_span_duration_ms: measuredSpanDurationMs,
    event_count: entries.length,
    span_count: spans.length,
    control_span_count: countByKind(spans, "CONTROL_COMMAND"),
    review_span_count: countByKind(spans, "REVIEW_EXCHANGE"),
    token_command_span_count: countByKind(spans, "TOKEN_COMMAND"),
    microtask_execution_span_count: countByKind(spans, "MICROTASK_EXECUTION"),
    stage_counts: stageCounts,
    thread_count: threadEntriesCount(entries),
    receipt_count: receipts.length,
    notification_count: notifications.length,
    control_request_count: controlRequests.length,
    control_result_count: controlResults.length,
    turn_usage_count: turnUsageCount(entries),
    token_summary_source: tokenLedger?.summary_source || "<missing>",
    token_input_total: tokenInputTotal,
    token_gross_input_total: tokenInputTotal,
    token_cached_input_total: tokenCachedInputTotal,
    token_fresh_input_total: tokenFreshInputTotal,
    token_output_total: tokenOutputTotal,
    token_turn_count: tokenTurnCount,
    token_command_count: tokenCommandCount,
    ledger_health_status: tokenLedger?.ledger_health?.status || "<missing>",
    ledger_health_severity: tokenLedger?.ledger_health?.severity || "<missing>",
    ledger_health_policy_id: tokenLedger?.ledger_health?.policy_id || "<missing>",
    ledger_health_drift_class: tokenLedger?.ledger_health?.drift_class || "<missing>",
    budget_status: tokenBudget.status,
    budget_policy_id: tokenBudget.policy_id,
    budget_enforcement_mode: tokenBudget.enforcement_mode || "<missing>",
    budget_blocker_class: tokenBudget.blocker_class || "NONE",
    budget_summary: tokenBudget.summary,
    relay_policy: relayPolicy,
    downtime_attribution: idleMetrics.downtime_attribution,
    queue_pressure: idleMetrics.queue_pressure,
    cost_estimate: null,
    cost_estimate_note: "unavailable_without_pricing_manifest",
  };
}

function summarizeDurations(values = []) {
  const finiteValues = values.filter((value) => Number.isFinite(value));
  return {
    count: finiteValues.length,
    latest_ms: finiteValues.length > 0 ? finiteValues.at(-1) : null,
    max_ms: finiteValues.length > 0 ? Math.max(...finiteValues) : null,
  };
}

function sumSpanDurations(spans = [], predicate = () => true) {
  return (Array.isArray(spans) ? spans : []).reduce((total, span) => {
    if (!predicate(span) || !Number.isFinite(span?.duration_ms)) return total;
    return total + Number(span.duration_ms);
  }, 0);
}

function classifyCurrentWait({
  runtimeStatus = null,
  laneVerdict = null,
  currentIdleMs = null,
} = {}) {
  const waitingOn = compactLabel(runtimeStatus?.waiting_on, "");
  const nextActor = compactLabel(runtimeStatus?.next_expected_actor, "");
  const verdict = compactLabel(laneVerdict?.verdict, "");
  const reasonCode = compactLabel(laneVerdict?.reasonCode, "");
  const combined = [waitingOn, nextActor, verdict, reasonCode].join(" ").toUpperCase();

  let bucket = "UNCLASSIFIED";
  if (/HUMAN|OPERATOR|APPROVAL|SIGNATURE|MERGE_PUSH/.test(combined)) {
    bucket = "HUMAN_WAIT";
  } else if (/DEPENDENCY|BLOCKED|OPEN_REVIEW_ITEM/.test(combined)) {
    bucket = "DEPENDENCY_WAIT";
  } else if (/VALIDATOR|FINAL_REVIEW|REVIEW/.test(combined)) {
    bucket = "VALIDATOR_WAIT";
  } else if (/REPAIR/.test(combined)) {
    bucket = "REPAIR_WAIT";
  } else if (/CODER|HANDOFF|INTENT/.test(combined)) {
    bucket = "CODER_WAIT";
  } else if (/ORCHESTRATOR|ROUTE|RELAY|CHECKPOINT|VERDICT|SESSION_CONTROL|STEER/.test(combined)) {
    bucket = "ROUTE_WAIT";
  }

  return {
    bucket,
    reason: waitingOn || reasonCode || verdict || nextActor || "NONE",
    duration_ms: Number.isFinite(currentIdleMs) ? currentIdleMs : null,
  };
}

function deriveQueuePressure({
  notifications = [],
  pendingNotificationCount = null,
  openReviewCount = 0,
  unresolvedControlCount = 0,
  runtimeStatus = null,
} = {}) {
  const pendingNotifications = Number.isFinite(Number(pendingNotificationCount))
    ? Number(pendingNotificationCount)
    : (Array.isArray(notifications) ? notifications.length : 0);
  const activeRoleSessionCount = Array.isArray(runtimeStatus?.active_role_sessions)
    ? runtimeStatus.active_role_sessions.length
    : 0;
  const score = pendingNotifications + Number(openReviewCount || 0) + Number(unresolvedControlCount || 0);
  let level = "LOW";
  if (pendingNotifications >= 2 || openReviewCount >= 2 || unresolvedControlCount >= 2 || score >= 4) {
    level = "HIGH";
  } else if (score >= 2 || activeRoleSessionCount >= 2) {
    level = "MEDIUM";
  }
  return {
    level,
    score,
    pending_notification_count: pendingNotifications,
    open_review_count: Number(openReviewCount || 0),
    unresolved_control_count: Number(unresolvedControlCount || 0),
    active_role_session_count: activeRoleSessionCount,
  };
}

function isValidatorRole(role) {
  return ["WP_VALIDATOR", "INTEGRATION_VALIDATOR", "VALIDATOR"].includes(normalizeRole(role));
}

function isValidatorPassReceipt(receipt = {}) {
  if (!isValidatorRole(receipt?.actor_role)) return false;
  const outcome = normalizeText(receipt?.microtask_contract?.review_outcome).toUpperCase();
  if (outcome === "APPROVED_FOR_FINAL_REVIEW") return true;
  const receiptKind = normalizeReceiptKind(receipt?.receipt_kind);
  if (!REVIEW_RESOLUTION_RECEIPT_KIND_VALUES.includes(receiptKind) && receiptKind !== "VALIDATOR_REVIEW") {
    return false;
  }
  return /\b(PASS|APPROVED|CLEARED)\b/i.test(compactLabel(receipt?.summary, ""));
}

function nextCoderActionTimestamp(afterTimestampMs, entries = []) {
  for (const entry of entries) {
    if (!Number.isFinite(entry?.timestamp_ms) || entry.timestamp_ms <= afterTimestampMs) continue;
    if (normalizeRole(entry?.role) !== "CODER") continue;
    if (String(entry?.kind || "").trim().toUpperCase() === "NOTIFICATION") continue;
    return entry.timestamp_ms;
  }
  return null;
}

function duplicateReceiptCount(receipts = []) {
  const counts = new Map();
  let duplicates = 0;
  for (const receipt of Array.isArray(receipts) ? receipts : []) {
    const key = [
      normalizeReceiptKind(receipt?.receipt_kind),
      normalizeRole(receipt?.actor_role),
      normalizeRole(receipt?.target_role),
      compactLabel(receipt?.target_session, ""),
      compactLabel(receipt?.correlation_id, ""),
      compactLabel(receipt?.packet_row_ref, ""),
      microtaskScopeRef(receipt),
      compactLabel(receipt?.summary, ""),
    ].join("|");
    const nextCount = Number(counts.get(key) || 0) + 1;
    counts.set(key, nextCount);
    if (nextCount > 1) duplicates += 1;
  }
  return duplicates;
}

export function buildWorkflowDossierIdleMetrics({
  entries = [],
  spans = [],
  receipts = [],
  notifications = [],
  controlRequests = [],
  controlResults = [],
  runtimeStatus = null,
  laneVerdict = null,
  pendingNotificationCount = null,
  now = Date.now(),
  idleThresholdMs = WORKFLOW_DOSSIER_IDLE_THRESHOLD_MS,
} = {}) {
  const sortedEntries = [...(Array.isArray(entries) ? entries : [])]
    .filter((entry) => Number.isFinite(entry?.timestamp_ms))
    .sort((left, right) => left.timestamp_ms - right.timestamp_ms || left.sequence - right.sequence);
  const gapDurations = [];
  for (let index = 1; index < sortedEntries.length; index += 1) {
    const gapMs = durationMs(sortedEntries[index - 1].timestamp_ms, sortedEntries[index].timestamp_ms);
    if (Number.isFinite(gapMs) && gapMs >= idleThresholdMs) gapDurations.push(gapMs);
  }

  const lastEventTimestampMs = sortedEntries.length > 0 ? sortedEntries.at(-1).timestamp_ms : null;
  const currentIdleMs = Number.isFinite(lastEventTimestampMs)
    ? Math.max(0, now - lastEventTimestampMs)
    : null;
  const countedIdleGaps = [...gapDurations];
  if (Number.isFinite(currentIdleMs) && currentIdleMs >= idleThresholdMs) {
    countedIdleGaps.push(currentIdleMs);
  }
  const reviewDurations = (Array.isArray(spans) ? spans : [])
    .filter((span) => span?.span_kind === "REVIEW_EXCHANGE" && Number.isFinite(span?.duration_ms))
    .map((span) => Number(span.duration_ms));
  const openReviewCount = (Array.isArray(spans) ? spans : [])
    .filter((span) => span?.span_kind === "REVIEW_EXCHANGE" && !Number.isFinite(span?.duration_ms))
    .length;

  const sortedReceipts = [...(Array.isArray(receipts) ? receipts : [])]
    .filter((entry) => Number.isFinite(parseTimestamp(entry?.timestamp_utc)))
    .sort((left, right) => parseTimestamp(left?.timestamp_utc) - parseTimestamp(right?.timestamp_utc));
  const passToCoderDurations = [];
  let validatorPassWaitingCount = 0;
  for (const receipt of sortedReceipts) {
    if (!isValidatorPassReceipt(receipt)) continue;
    const passTimestampMs = parseTimestamp(receipt?.timestamp_utc);
    const nextCoderActionMs = nextCoderActionTimestamp(passTimestampMs, sortedEntries);
    if (Number.isFinite(nextCoderActionMs)) {
      passToCoderDurations.push(nextCoderActionMs - passTimestampMs);
      continue;
    }
    validatorPassWaitingCount += 1;
  }

  const unresolvedControlCount = Math.max(
    0,
    Number((controlRequests || []).length || 0) - Number((controlResults || []).length || 0),
  );
  const currentWait = classifyCurrentWait({
    runtimeStatus,
    laneVerdict,
    currentIdleMs,
  });
  const measuredActiveBuildMs = sumSpanDurations(spans, (span) =>
    span?.span_kind === "MICROTASK_EXECUTION" && span?.span_stage !== "REPAIR_LOOP"
  );
  const measuredRepairOverheadMs = sumSpanDurations(spans, (span) =>
    (span?.span_kind === "MICROTASK_EXECUTION" && span?.span_stage === "REPAIR_LOOP")
    || (span?.span_kind === "CONTROL_COMMAND" && normalizeCommandKind(span?.command_kind) === "CANCEL_SESSION")
  );
  const measuredValidatorWaitMs = sumSpanDurations(spans, (span) =>
    span?.span_kind === "REVIEW_EXCHANGE" && isValidatorRole(span?.target_role)
  );
  const measuredRouteWaitMs = sumSpanDurations(spans, (span) =>
    span?.span_kind === "CONTROL_COMMAND" && span?.span_stage === "RELAY"
  );
  const queuePressure = deriveQueuePressure({
    notifications,
    pendingNotificationCount,
    openReviewCount,
    unresolvedControlCount,
    runtimeStatus,
  });

  return {
    idle_threshold_ms: idleThresholdMs,
    current_idle_ms: currentIdleMs,
    current_idle_exceeds_threshold: Number.isFinite(currentIdleMs) ? currentIdleMs >= idleThresholdMs : false,
    idle_gap_count: countedIdleGaps.length,
    latest_idle_gap_ms: countedIdleGaps.length > 0 ? countedIdleGaps.at(-1) : null,
    max_idle_gap_ms: countedIdleGaps.length > 0 ? Math.max(...countedIdleGaps) : null,
    review_response: {
      ...summarizeDurations(reviewDurations),
      open_count: openReviewCount,
    },
    validator_pass_to_next_coder_action: {
      ...summarizeDurations(passToCoderDurations),
      waiting_count: validatorPassWaitingCount,
    },
    downtime_attribution: {
      active_build_ms: measuredActiveBuildMs,
      validator_wait_ms: measuredValidatorWaitMs + (currentWait.bucket === "VALIDATOR_WAIT" ? Number(currentIdleMs || 0) : 0),
      route_wait_ms: measuredRouteWaitMs + (currentWait.bucket === "ROUTE_WAIT" ? Number(currentIdleMs || 0) : 0),
      dependency_wait_ms: currentWait.bucket === "DEPENDENCY_WAIT" ? Number(currentIdleMs || 0) : 0,
      human_wait_ms: currentWait.bucket === "HUMAN_WAIT" ? Number(currentIdleMs || 0) : 0,
      repair_overhead_ms: measuredRepairOverheadMs + (currentWait.bucket === "REPAIR_WAIT" ? Number(currentIdleMs || 0) : 0),
      coder_wait_ms: currentWait.bucket === "CODER_WAIT" ? Number(currentIdleMs || 0) : 0,
      unattributed_current_idle_ms: currentWait.bucket === "UNCLASSIFIED" ? Number(currentIdleMs || 0) : 0,
      current_wait: currentWait,
    },
    queue_pressure: queuePressure,
    drift_markers: {
      duplicate_receipt_count: duplicateReceiptCount(receipts),
      open_review_count: openReviewCount,
      unresolved_control_count: unresolvedControlCount,
    },
  };
}

function threadEntriesCount(entries = []) {
  return entries.filter((entry) => entry.kind === "THREAD").length;
}

function turnUsageCount(entries = []) {
  return entries.filter((entry) => entry.kind === "TURN_USAGE").length;
}

export function loadWpTimelineArtifacts(repoRoot, wpId) {
  const packet = resolveWorkPacketPath(wpId);
  if (!packet?.packetPath) {
    throw new Error(`Official packet not found for ${wpId}`);
  }
  const packetText = fs.readFileSync(packet.packetAbsPath, "utf8");
  const workflowLane = parseSingleField(packetText, "WORKFLOW_LANE");
  const runtimeStatusFile = parseSingleField(packetText, "WP_RUNTIME_STATUS_FILE");
  const receiptsFile = parseSingleField(packetText, "WP_RECEIPTS_FILE");
  const threadFile = parseSingleField(packetText, "WP_THREAD_FILE");
  const communicationDir = parseSingleField(packetText, "WP_COMMUNICATION_DIR");
  const notificationsFile = communicationDir
    ? normalizePath(path.join(communicationDir, "NOTIFICATIONS.jsonl"))
    : "";

  const runtimeStatus = runtimeStatusFile && fs.existsSync(repoPathAbs(runtimeStatusFile))
    ? parseJsonFile(runtimeStatusFile)
    : null;
  const receipts = receiptsFile && fs.existsSync(repoPathAbs(receiptsFile))
    ? parseJsonlFile(receiptsFile)
    : [];
  const threadEntries = threadFile && fs.existsSync(repoPathAbs(threadFile))
    ? parseThreadEntriesText(fs.readFileSync(repoPathAbs(threadFile), "utf8"))
    : [];
  const notifications = notificationsFile && fs.existsSync(repoPathAbs(notificationsFile))
    ? parseJsonlFile(notificationsFile)
    : [];
  const controlRequests = fs.existsSync(repoPathAbs(SESSION_CONTROL_REQUESTS_FILE))
    ? parseJsonlFile(SESSION_CONTROL_REQUESTS_FILE).filter((entry) => normalizeText(entry.wp_id) === wpId)
    : [];
  const controlResults = fs.existsSync(repoPathAbs(SESSION_CONTROL_RESULTS_FILE))
    ? parseJsonlFile(SESSION_CONTROL_RESULTS_FILE).filter((entry) => normalizeText(entry.wp_id) === wpId)
    : [];
  const tokenLedger = readWpTokenUsageLedger(repoRoot, wpId).ledger;

  return {
    wpId,
    packetPath: packet.packetPath,
    workflowLane,
    runtimeStatus,
    threadEntries,
    receipts,
    notifications,
    controlRequests,
    controlResults,
    tokenLedger,
  };
}

// --- WP Metrics (TG-012: same-domain functions, not a separate script) ---

function msToMinutes(ms) {
  return ms != null ? Math.round((ms / 60_000) * 10) / 10 : null;
}

function safeRatio(numerator, denominator) {
  if (!denominator || denominator === 0) return null;
  return Math.round((numerator / denominator) * 1000) / 1000;
}

export function countReceiptsByKind(receipts) {
  const counts = {};
  for (const r of receipts) {
    const kind = String(r.receipt_kind || "UNKNOWN").trim();
    counts[kind] = (counts[kind] || 0) + 1;
  }
  return counts;
}

function extractMicrotask(receipt) {
  const summary = String(receipt.summary || "");
  const match = summary.match(/\bMT-\d+\b/i);
  return match ? match[0].toUpperCase() : null;
}

export function countFixCycles(receipts) {
  let total = 0;
  const byMt = {};
  for (const r of receipts) {
    if (r.receipt_kind !== "REVIEW_RESPONSE") continue;
    const summary = String(r.summary || "").toLowerCase();
    const isSteer = summary.includes("steer")
      || summary.includes("not pass")
      || summary.includes("not-pass")
      || summary.includes("rejected")
      || summary.includes("remediation");
    if (!isSteer) continue;
    total += 1;
    const mt = extractMicrotask(r);
    if (mt) {
      byMt[mt] = (byMt[mt] || 0) + 1;
    }
  }
  return { total, by_mt: byMt };
}

export function countMicrotasks(receipts) {
  const mts = new Set();
  for (const r of receipts) {
    const mt = extractMicrotask(r);
    if (mt) mts.add(mt);
  }
  return mts.size;
}

export function countSessionControlByStatus(controlResults) {
  const counts = { total: 0, completed: 0, failed: 0 };
  const byKind = {};
  for (const r of controlResults) {
    counts.total += 1;
    const status = String(r.status || "").toUpperCase();
    if (status === "COMPLETED") counts.completed += 1;
    if (status === "FAILED") counts.failed += 1;
    const kind = String(r.command_kind || "UNKNOWN").trim();
    if (!byKind[kind]) byKind[kind] = { total: 0, completed: 0, failed: 0 };
    byKind[kind].total += 1;
    if (status === "COMPLETED") byKind[kind].completed += 1;
    if (status === "FAILED") byKind[kind].failed += 1;
  }
  return { ...counts, by_kind: byKind };
}

export function countSessionRestarts(controlResults) {
  const cancelledKeys = new Set();
  let restarts = 0;
  for (const r of controlResults) {
    const key = String(r.session_key || "").trim();
    const kind = String(r.command_kind || "").trim();
    const status = String(r.status || "").toUpperCase();
    if (kind === "CANCEL_SESSION" && status === "COMPLETED") {
      cancelledKeys.add(key);
    }
    if (kind === "START_SESSION" && status === "COMPLETED" && cancelledKeys.has(key)) {
      restarts += 1;
    }
  }
  return restarts;
}

export function loadValidationEvidence(wpId) {
  const gatePath = repoPathAbs(resolveValidatorGatePath(wpId));
  if (!fs.existsSync(gatePath)) return null;
  try {
    const raw = JSON.parse(fs.readFileSync(gatePath, "utf8"));
    const evidence = raw?.committed_validation_evidence?.[wpId];
    if (!evidence) return null;
    const history = Array.isArray(evidence.proof_history) ? evidence.proof_history : [];
    return {
      proof_runs: history.length,
      proof_pass: history.filter((p) => p.status === "PASS").length,
      proof_fail: history.filter((p) => p.status === "FAIL").length,
      zero_execution_incidents: history.filter((p) => p.zero_execution_detected).length,
      first_pass_success: history.length > 0 && history[0]?.status === "PASS",
    };
  } catch {
    return null;
  }
}

export function countStaleRouteIncidents(receipts, controlResults) {
  let count = 0;
  for (const r of receipts) {
    const summary = String(r.summary || "").toLowerCase();
    if (summary.includes("route_stale") || summary.includes("stale route")) count += 1;
  }
  for (const r of controlResults) {
    const summary = String(r.summary || r.error || "").toLowerCase();
    if (summary.includes("route_stale") || summary.includes("stale")) count += 1;
  }
  return count;
}

export function countDuplicateReceipts(receipts) {
  const seen = new Set();
  let duplicates = 0;
  for (const r of receipts) {
    const key = `${r.receipt_kind}:${r.correlation_id || ""}:${r.actor_role}:${r.target_role}`;
    if (r.correlation_id && seen.has(key)) duplicates += 1;
    seen.add(key);
  }
  return duplicates;
}

export function buildWpMetrics({ wpId, summary, receipts, controlResults }) {
  const dt = summary.downtime_attribution || {};
  const activeMs = dt.active_build_ms || 0;
  const repairMs = dt.repair_overhead_ms || 0;
  const validatorWaitMs = dt.validator_wait_ms || 0;
  const routeWaitMs = dt.route_wait_ms || 0;
  const coderWaitMs = dt.coder_wait_ms || 0;

  const fixCycles = countFixCycles(receipts);
  const sessionControl = countSessionControlByStatus(controlResults);
  const validationEvidence = loadValidationEvidence(wpId);

  return {
    wp_id: wpId,
    extracted_at: new Date().toISOString(),
    wall_clock_minutes: msToMinutes(summary.event_window_duration_ms),
    product_active_minutes: msToMinutes(activeMs),
    repair_minutes: msToMinutes(repairMs),
    validator_wait_minutes: msToMinutes(validatorWaitMs),
    route_wait_minutes: msToMinutes(routeWaitMs),
    coder_wait_minutes: msToMinutes(coderWaitMs),
    governance_overhead_ratio: safeRatio(repairMs + routeWaitMs, activeMs + validatorWaitMs + coderWaitMs),
    receipt_count: receipts.length,
    receipt_kinds: countReceiptsByKind(receipts),
    duplicate_receipts: countDuplicateReceipts(receipts),
    stale_route_incidents: countStaleRouteIncidents(receipts, controlResults),
    review_rtt_max_ms: dt.review_rtt_max_ms ?? null,
    acp_commands: sessionControl.total,
    acp_failures: sessionControl.failed,
    acp_by_kind: sessionControl.by_kind,
    session_restarts: countSessionRestarts(controlResults),
    mt_count: countMicrotasks(receipts),
    fix_cycles: fixCycles.total,
    fix_cycles_by_mt: fixCycles.by_mt,
    proof_runs: validationEvidence?.proof_runs ?? 0,
    proof_pass: validationEvidence?.proof_pass ?? 0,
    proof_fail: validationEvidence?.proof_fail ?? 0,
    zero_execution_incidents: validationEvidence?.zero_execution_incidents ?? 0,
    first_pass_compile_success: validationEvidence?.first_pass_success ?? null,
    token_input_total: summary.token_input_total,
    token_gross_input_total: summary.token_gross_input_total ?? summary.token_input_total,
    token_cached_input_total: summary.token_cached_input_total ?? 0,
    token_fresh_input_total: summary.token_fresh_input_total
      ?? Math.max(0, Number(summary.token_input_total || 0) - Number(summary.token_cached_input_total || 0)),
    token_output_total: summary.token_output_total,
    token_turn_count: summary.token_turn_count,
    token_command_count: summary.token_command_count ?? 0,
    ledger_health: summary.ledger_health_status,
    ledger_health_severity: summary.ledger_health_severity,
    ledger_health_policy_id: summary.ledger_health_policy_id,
    ledger_health_drift_class: summary.ledger_health_drift_class,
    budget_status: summary.budget_status,
    budget_policy_id: summary.budget_policy_id,
    budget_enforcement_mode: summary.budget_enforcement_mode,
    budget_blocker_class: summary.budget_blocker_class,
    cost_estimate: summary.cost_estimate,
    queue_pressure_max_score: summary.queue_pressure?.score ?? null,
    runtime_status: summary.runtime_status,
    current_phase: summary.current_phase,
    workflow_lane: summary.workflow_lane,
  };
}

export function buildWpMetricsComparison(metricsA, metricsB) {
  const fields = [
    ["wall_clock_minutes", "Wall clock (min)"],
    ["product_active_minutes", "Product active (min)"],
    ["repair_minutes", "Repair overhead (min)"],
    ["validator_wait_minutes", "Validator wait (min)"],
    ["governance_overhead_ratio", "Gov overhead ratio"],
    ["receipt_count", "Receipts"],
    ["duplicate_receipts", "Duplicate receipts"],
    ["stale_route_incidents", "Stale route incidents"],
    ["acp_commands", "ACP commands"],
    ["acp_failures", "ACP failures"],
    ["session_restarts", "Session restarts"],
    ["mt_count", "Microtasks"],
    ["fix_cycles", "Fix cycles"],
    ["zero_execution_incidents", "Zero-execution incidents"],
    ["token_gross_input_total", "Tokens in (gross)"],
    ["token_fresh_input_total", "Tokens in (fresh)"],
    ["token_cached_input_total", "Tokens in (cached)"],
    ["token_output_total", "Tokens out"],
    ["token_turn_count", "Turns"],
    ["token_command_count", "Token commands"],
    ["cost_estimate", "Cost estimate"],
  ];
  return fields.map(([key, label]) => {
    const a = metricsA[key];
    const b = metricsB[key];
    const delta = (a != null && b != null) ? Math.round((b - a) * 10) / 10 : null;
    const trend = delta == null ? "" : delta > 0 ? "UP" : delta < 0 ? "DOWN" : "SAME";
    return { metric: label, wp_a: a, wp_b: b, delta, trend };
  });
}
