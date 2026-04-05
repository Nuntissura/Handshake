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
  resolveWorkPacketPath,
} from "../lib/runtime-paths.mjs";
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
    entries.push({
      timestamp: entry.timestamp_utc || "",
      timestamp_ms: parseTimestamp(entry.timestamp_utc),
      sequence: sequence += 1,
      kind: "RECEIPT",
      role: normalizeRole(entry.actor_role),
      header: `${entry.timestamp_utc || "<no-ts>"} | RECEIPT | ${entry.actor_role || "<unknown>"} | ${entry.receipt_kind || "<unknown>"}`,
      detailLines: [
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
    token_input_total: Number(tokenLedger?.summary?.usage_totals?.input_tokens || 0),
    token_cached_input_total: Number(tokenLedger?.summary?.usage_totals?.cached_input_tokens || 0),
    token_output_total: Number(tokenLedger?.summary?.usage_totals?.output_tokens || 0),
    token_turn_count: Number(tokenLedger?.summary?.turn_count || 0),
    token_command_count: Number(tokenLedger?.summary?.command_count || 0),
    ledger_health_status: tokenLedger?.ledger_health?.status || "<missing>",
    ledger_health_severity: tokenLedger?.ledger_health?.severity || "<missing>",
    budget_status: tokenBudget.status,
    budget_summary: tokenBudget.summary,
    cost_estimate: null,
    cost_estimate_note: "unavailable_without_pricing_manifest",
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
