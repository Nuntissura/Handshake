import crypto from "node:crypto";
import {
  buildGovernedActionResult,
  summarizeGovernedAction,
} from "../session/session-governed-action-lib.mjs";

const VALIDATOR_GATE_RULES = Object.freeze({
  APPEND: {
    ruleId: "VALIDATOR_GATE_APPEND_APPROVE",
    actionKind: "APPROVE",
    commandKind: "APPEND",
  },
  COMMIT: {
    ruleId: "VALIDATOR_GATE_COMMIT_APPROVE",
    actionKind: "APPROVE",
    commandKind: "COMMIT",
  },
  PRESENT_REPORT: {
    ruleId: "VALIDATOR_GATE_PRESENT_APPROVE",
    actionKind: "APPROVE",
    commandKind: "PRESENT_REPORT",
  },
  ACKNOWLEDGE: {
    ruleId: "VALIDATOR_GATE_ACKNOWLEDGE_APPROVE",
    actionKind: "APPROVE",
    commandKind: "ACKNOWLEDGE",
  },
  RESET: {
    ruleId: "VALIDATOR_GATE_RESET_EXTERNAL_EXECUTE",
    actionKind: "EXTERNAL_EXECUTE",
    commandKind: "RESET",
  },
});

function cleanString(value) {
  return String(value ?? "").trim();
}

function normalizeMetadata(value) {
  if (!value || typeof value !== "object" || Array.isArray(value)) return {};
  return Object.fromEntries(
    Object.entries(value)
      .map(([key, entryValue]) => [cleanString(key), cleanString(entryValue)])
      .filter(([key, entryValue]) => key && entryValue),
  );
}

export function validatorGateRuleForAction(gateAction = "") {
  return VALIDATOR_GATE_RULES[cleanString(gateAction).toUpperCase()] || null;
}

function normalizeValidatorGateActionSummary(summary = null) {
  const normalized = summarizeGovernedAction(summary || {});
  if (!normalized) return null;
  const metadata = normalizeMetadata(summary?.metadata);
  return {
    ...normalized,
    gate_action: metadata.gate_action || cleanString(summary?.gate_action).toUpperCase() || "",
    gate_status: metadata.gate_status || cleanString(summary?.gate_status) || "",
    gate_verdict: metadata.gate_verdict || cleanString(summary?.gate_verdict).toUpperCase() || "",
  };
}

export function normalizeValidatorGateActionHistory(entries = []) {
  if (!Array.isArray(entries)) return [];
  return entries
    .map((entry) => normalizeValidatorGateActionSummary(entry))
    .filter(Boolean);
}

export function deriveValidatorGateSessionStatus(session = {}) {
  const normalizedHistory = normalizeValidatorGateActionHistory(session?.governed_action_history);
  const lastHistoryAction = normalizedHistory.at(-1) || null;
  const lastAction = normalizeValidatorGateActionSummary(session?.last_governed_action) || lastHistoryAction;
  const effectiveStatus = cleanString(lastAction?.gate_status) || cleanString(session?.status);
  return {
    status: effectiveStatus,
    lastGovernedAction: lastAction,
    governedActionHistory: normalizedHistory,
  };
}

export function normalizeValidatorGateSession(session = {}) {
  const normalizedSession = session && typeof session === "object" ? { ...session } : {};
  const derived = deriveValidatorGateSessionStatus(normalizedSession);
  normalizedSession.status = derived.status;
  normalizedSession.last_governed_action = derived.lastGovernedAction;
  normalizedSession.governed_action_history = derived.governedActionHistory;
  normalizedSession.gates = Array.isArray(normalizedSession.gates) ? normalizedSession.gates : [];
  return normalizedSession;
}

export function buildValidatorGateGovernedAction({
  wpId = "",
  gateAction = "",
  sessionKey = "",
  role = "",
  summary = "",
  gateStatus = "",
  gateVerdict = "",
  previousStatus = "",
  processedAt = "",
  metadata = {},
} = {}) {
  const rule = validatorGateRuleForAction(gateAction);
  if (!rule) {
    throw new Error(`Unknown validator gate action: ${gateAction || "<missing>"}`);
  }
  const timestamp = cleanString(processedAt) || new Date().toISOString();
  const commandId = `validator-gate-${cleanString(gateAction).toLowerCase() || "action"}-${crypto.randomUUID()}`;
  return buildGovernedActionResult({
    actionId: commandId,
    ruleId: rule.ruleId,
    actionKind: rule.actionKind,
    commandKind: rule.commandKind,
    commandId,
    sessionKey: cleanString(sessionKey) || `${cleanString(role).toUpperCase() || "VALIDATOR"}:${cleanString(wpId) || "WP-UNKNOWN"}`,
    wpId: cleanString(wpId) || "WP-UNKNOWN",
    role: cleanString(role).toUpperCase() || "VALIDATOR",
    status: "COMPLETED",
    outcomeState: "SETTLED",
    summary: cleanString(summary),
    processedAt: timestamp,
    metadata: {
      gate_action: cleanString(gateAction).toUpperCase(),
      gate_status: cleanString(gateStatus),
      gate_verdict: cleanString(gateVerdict).toUpperCase(),
      previous_status: cleanString(previousStatus),
      ...normalizeMetadata(metadata),
    },
  });
}

export function appendValidatorGateGovernedAction(session = {}, action = null) {
  const normalizedSession = normalizeValidatorGateSession(session);
  const normalizedAction = normalizeValidatorGateActionSummary(action);
  if (!normalizedAction) return normalizedSession;
  normalizedSession.governed_action_history = [
    ...normalizedSession.governed_action_history,
    normalizedAction,
  ];
  normalizedSession.last_governed_action = normalizedAction;
  if (normalizedAction.gate_status) {
    normalizedSession.status = normalizedAction.gate_status;
  }
  return normalizedSession;
}
