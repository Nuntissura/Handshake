import fs from "node:fs";
import path from "node:path";
import {
  matchesAnyScopeEntry,
  normalizeRepoPath,
  normalizeScopeEntries,
  parsePacketSingleField,
} from "./scope-surface-lib.mjs";
import { normalizePath, resolveWorkPacketPath } from "./runtime-paths.mjs";
import { classifyHeuristicRiskText } from "./heuristic-risk-lib.mjs";

const MICROTASK_FILE_RE = /^MT-\d{3}\.md$/i;

function normalizeRole(value) {
  return String(value || "").trim().toUpperCase();
}

function normalizeReceiptKind(value) {
  return String(value || "").trim().toUpperCase();
}

const MICROTASK_STATE_RECEIPT_KINDS = new Set([
  "CODER_INTENT",
  "REVIEW_REQUEST",
  "CODER_HANDOFF",
  "REPAIR",
  "VALIDATOR_QUERY",
  "VALIDATOR_RESPONSE",
  "VALIDATOR_REVIEW",
  "REVIEW_RESPONSE",
  "SPEC_CONFIRMATION",
]);

function normalizeReviewOutcome(value) {
  const normalized = String(value || "").trim().toUpperCase();
  if (normalized === "REPAIR_REQUIRED") return "REPAIR_REQUIRED";
  if (normalized === "APPROVED_FOR_FINAL_REVIEW") return "APPROVED_FOR_FINAL_REVIEW";
  return "UNKNOWN";
}

function parseTimestamp(value) {
  const text = String(value || "").trim();
  if (!text) return null;
  const parsed = Date.parse(text);
  return Number.isNaN(parsed) ? null : parsed;
}

function normalizeScopeRefKey(value) {
  return String(value || "")
    .trim()
    .replace(/^`|`$/g, "")
    .replace(/\s+/g, " ")
    .replace(/\/+/g, "/")
    .toUpperCase();
}

function parseDelimitedList(rawValue, { normalizeAsRepoPath = false } = {}) {
  const raw = String(rawValue || "");
  const backtickEntries = Array.from(raw.matchAll(/`([^`]+)`/g))
    .map((match) => String(match[1] || "").trim())
    .filter(Boolean);
  const entries = (backtickEntries.length > 0 ? backtickEntries : raw.split(/[;,]/))
    .map((value) => value.trim())
    .filter(Boolean);
  if (!normalizeAsRepoPath) return Array.from(new Set(entries));
  return normalizeScopeEntries(entries);
}

function parseMicrotaskDefinition(mtAbsPath, mtRelPath) {
  const text = fs.readFileSync(mtAbsPath, "utf8");
  const mtId = String(parsePacketSingleField(text, "MT_ID") || "").trim();
  const clause = String(parsePacketSingleField(text, "CLAUSE") || "").trim();
  const codeSurfaces = parseDelimitedList(parsePacketSingleField(text, "CODE_SURFACES"), { normalizeAsRepoPath: true });
  const expectedTests = parseDelimitedList(parsePacketSingleField(text, "EXPECTED_TESTS"));
  const dependsOn = String(parsePacketSingleField(text, "DEPENDS_ON") || "").trim() || "NONE";
  const heuristicRisk = classifyHeuristicRiskText(text);

  if (!mtId) {
    throw new Error(`Malformed microtask file ${normalizePath(mtRelPath)}: missing MT_ID`);
  }

  const clauseTokenMatches = Array.from(clause.matchAll(/\[([^\]]+)\]/g))
    .map((match) => String(match[1] || "").trim())
    .filter(Boolean);
  const aliases = new Set([
    mtId,
    clause,
    `CLAUSE_CLOSURE_MATRIX/${clause}`,
  ]);
  for (const token of clauseTokenMatches) {
    aliases.add(token);
    aliases.add(`[${token}]`);
    aliases.add(`CLAUSE_CLOSURE_MATRIX/${token}`);
    aliases.add(`CLAUSE_CLOSURE_MATRIX/[${token}]`);
  }

  return {
    mtId,
    clause,
    codeSurfaces,
    expectedTests,
    dependsOn,
    heuristicRisk,
    packetPath: normalizePath(mtRelPath),
    scopeRefKeys: Array.from(aliases).map((value) => normalizeScopeRefKey(value)).filter(Boolean),
  };
}

export function listDeclaredWpMicrotasks(wpId) {
  const resolved = resolveWorkPacketPath(wpId);
  if (!resolved?.isFolder || !resolved.packetDirAbs || !fs.existsSync(resolved.packetDirAbs)) {
    return [];
  }

  return fs.readdirSync(resolved.packetDirAbs, { withFileTypes: true })
    .filter((entry) => entry.isFile() && MICROTASK_FILE_RE.test(entry.name))
    .sort((left, right) => left.name.localeCompare(right.name))
    .map((entry) => {
      const mtAbsPath = path.join(resolved.packetDirAbs, entry.name);
      const mtRelPath = path.posix.join(resolved.packetDir, entry.name);
      return parseMicrotaskDefinition(mtAbsPath, mtRelPath);
    });
}

export function resolveDeclaredWpMicrotaskByScopeRef(wpId, scopeRef, microtasks = null) {
  const declaredMicrotasks = Array.isArray(microtasks) ? microtasks : listDeclaredWpMicrotasks(wpId);
  const scopeRefKey = normalizeScopeRefKey(scopeRef);
  if (!scopeRefKey) {
    return {
      declaredMicrotasks,
      match: null,
      ambiguousMatches: [],
    };
  }

  const matches = declaredMicrotasks.filter((definition) => definition.scopeRefKeys.includes(scopeRefKey));
  return {
    declaredMicrotasks,
    match: matches.length === 1 ? matches[0] : null,
    ambiguousMatches: matches.length > 1 ? matches : [],
  };
}

export function summarizeMicrotaskFileTargetBudget(fileTargets, microtaskDefinition) {
  const normalizedTargets = Array.isArray(fileTargets)
    ? fileTargets.map((entry) => normalizeRepoPath(entry)).filter(Boolean)
    : [];
  const allowedSurfaces = Array.isArray(microtaskDefinition?.codeSurfaces)
    ? microtaskDefinition.codeSurfaces
    : [];
  const outOfBudgetTargets = normalizedTargets.filter((target) => !matchesAnyScopeEntry(target, allowedSurfaces));

  return {
    normalizedTargets,
    allowedSurfaces,
    outOfBudgetTargets,
    ok: outOfBudgetTargets.length === 0,
  };
}

function reviewOutcomeFromSummary(summary = "") {
  const text = String(summary || "").trim();
  if (!text) return "UNKNOWN";
  if (
    /^MT-\d{3}\s+STEER\s*:/i.test(text)
    || /\brepair required\b/i.test(text)
    || /\bremediation required\b/i.test(text)
    || /\brepair by\b/i.test(text)
    || /\bfix required\b/i.test(text)
  ) {
    return "REPAIR_REQUIRED";
  }
  if (/\bapproved for final review\b/i.test(text) || /\bready for final review\b/i.test(text) || /\bcleared\b/i.test(text)) {
    return "APPROVED_FOR_FINAL_REVIEW";
  }
  return "UNKNOWN";
}

function resolvedMicrotaskForScopeRef(wpId, scopeRef, microtasks) {
  const resolution = resolveDeclaredWpMicrotaskByScopeRef(wpId, scopeRef, microtasks);
  return resolution.match || null;
}

function statePriority(state = "") {
  switch (String(state || "").trim().toUpperCase()) {
    case "REPAIR_REQUIRED":
      return 5;
    case "IN_REVIEW":
      return 4;
    case "ACTIVE":
      return 3;
    case "CLEARED":
      return 2;
    case "DECLARED":
    default:
      return 1;
  }
}

function reviewBoundaryState(state = "") {
  const normalized = String(state || "").trim().toUpperCase();
  return normalized === "CLEARED" || normalized === "IN_REVIEW" || normalized === "REPAIR_REQUIRED";
}

function normalizeReviewMode(value = "") {
  return String(value || "").trim().toUpperCase();
}

function scopeRefFromEntry(entry = {}, correlationScopeRefs = new Map()) {
  const explicitScopeRef = String(entry?.microtask_contract?.scope_ref || "").trim();
  if (explicitScopeRef) return explicitScopeRef;
  const packetRowRef = String(entry?.packet_row_ref || "").trim();
  if (/^MT-\d{3}$/i.test(packetRowRef)) return packetRowRef.toUpperCase();
  const summaryMatch = String(entry?.summary || "").match(/\b(MT-\d{3})\b/i);
  if (summaryMatch) return summaryMatch[1].toUpperCase();
  const correlationId = String(entry?.correlation_id || "").trim();
  if (correlationId && correlationScopeRefs.has(correlationId)) {
    return correlationScopeRefs.get(correlationId);
  }
  return "";
}

function reviewModeFromEntry(entry = {}) {
  const explicitReviewModeRaw = String(entry?.microtask_contract?.review_mode || "").trim();
  if (explicitReviewModeRaw) return normalizeReviewMode(explicitReviewModeRaw);
  if (/review_mode\s*=\s*OVERLAP/i.test(String(entry?.summary || ""))) return "OVERLAP";
  if (
    normalizeReceiptKind(entry?.receipt_kind) === "REVIEW_REQUEST"
    && normalizeRole(entry?.opened_by_role || entry?.actor_role) === "CODER"
    && normalizeRole(entry?.target_role) === "WP_VALIDATOR"
    && /^MT-\d{3}$/i.test(String(entry?.packet_row_ref || "").trim())
  ) {
    return "OVERLAP";
  }
  return null;
}

function receiptCanAdvanceMicrotaskState(receipt = {}) {
  return MICROTASK_STATE_RECEIPT_KINDS.has(normalizeReceiptKind(receipt?.receipt_kind));
}

function stateFromReceipt(receipt = {}) {
  const actorRole = normalizeRole(receipt?.actor_role);
  const receiptKind = normalizeReceiptKind(receipt?.receipt_kind);
  if (actorRole === "CODER") {
    if (["CODER_INTENT", "REVIEW_REQUEST", "CODER_HANDOFF", "REPAIR"].includes(receiptKind)) return "ACTIVE";
    return "DECLARED";
  }

  if (["WP_VALIDATOR", "INTEGRATION_VALIDATOR", "VALIDATOR"].includes(actorRole)) {
    const explicitOutcome = normalizeReviewOutcome(receipt?.microtask_contract?.review_outcome);
    const derivedOutcome = explicitOutcome !== "UNKNOWN" ? explicitOutcome : reviewOutcomeFromSummary(receipt?.summary);
    if (derivedOutcome === "REPAIR_REQUIRED") return "REPAIR_REQUIRED";
    if (["VALIDATOR_RESPONSE", "REVIEW_RESPONSE", "SPEC_CONFIRMATION", "VALIDATOR_REVIEW"].includes(receiptKind)) {
      return "CLEARED";
    }
  }

  return "DECLARED";
}

function findFirstItemByState(items = [], expectedState = "") {
  const normalized = String(expectedState || "").trim().toUpperCase();
  return items.find((entry) => String(entry?.state || "").trim().toUpperCase() === normalized) || null;
}

function findMostRecentItemByState(items = [], expectedState = "") {
  const normalized = String(expectedState || "").trim().toUpperCase();
  const matches = items
    .map((entry, index) => ({ entry, index }))
    .filter(({ entry }) => String(entry?.state || "").trim().toUpperCase() === normalized)
    .sort((left, right) =>
      String(right.entry?.last_activity_at || "").localeCompare(String(left.entry?.last_activity_at || ""))
      || right.index - left.index
    );
  return matches[0]?.entry || null;
}

function deriveActiveExecutionMicrotask(items = []) {
  return findMostRecentItemByState(items, "ACTIVE")
    || items.find((entry) =>
      String(entry?.state || "").trim().toUpperCase() === "IN_REVIEW"
      && normalizeReviewMode(entry?.review_mode) !== "OVERLAP"
    )
    || findFirstItemByState(items, "REPAIR_REQUIRED")
    || findFirstItemByState(items, "DECLARED")
    || null;
}

function derivePreviousExecutionMicrotask(items = [], activeMicrotask = null) {
  if (!Array.isArray(items) || items.length === 0) return null;

  if (activeMicrotask?.mt_id) {
    const activeIndex = items.findIndex((entry) => entry.mt_id === activeMicrotask.mt_id);
    if (activeIndex > 0) {
      const previous = items[activeIndex - 1];
      return reviewBoundaryState(previous?.state) ? previous : null;
    }
    return null;
  }

  const reversed = [...items].reverse();
  return reversed.find((entry) => reviewBoundaryState(entry?.state)) || null;
}

export function deriveWpMicrotaskPlan({
  wpId,
  receipts = [],
  runtimeStatus = {},
  microtasks = null,
} = {}) {
  const declaredMicrotasks = Array.isArray(microtasks) ? microtasks : listDeclaredWpMicrotasks(wpId);
  const byId = new Map(declaredMicrotasks.map((definition) => [
    definition.mtId,
    {
      mt_id: definition.mtId,
      clause: definition.clause,
      packet_path: definition.packetPath,
      depends_on: definition.dependsOn,
      code_surfaces: definition.codeSurfaces,
      expected_tests: definition.expectedTests,
      heuristic_risk: definition.heuristicRisk,
      state: "DECLARED",
      state_reason: "declared_only",
      last_activity_at: null,
      last_receipt_kind: null,
      last_actor_role: null,
      correlation_id: null,
      review_mode: null,
    },
  ]));

  const orderedReceipts = [...(Array.isArray(receipts) ? receipts : [])].sort((left, right) =>
    String(left?.timestamp_utc || "").localeCompare(String(right?.timestamp_utc || ""))
  );
  const correlationScopeRefs = new Map();

  for (const receipt of orderedReceipts) {
    if (!receiptCanAdvanceMicrotaskState(receipt)) continue;
    const scopeRef = scopeRefFromEntry(receipt, correlationScopeRefs);
    if (!scopeRef) continue;
    const correlationId = String(receipt?.correlation_id || "").trim();
    if (correlationId) correlationScopeRefs.set(correlationId, scopeRef);
    const definition = resolvedMicrotaskForScopeRef(wpId, scopeRef, declaredMicrotasks);
    if (!definition) continue;
    const entry = byId.get(definition.mtId);
    if (!entry) continue;
    const nextState = stateFromReceipt(receipt);
    const currentTs = parseTimestamp(entry.last_activity_at);
    const receiptTs = parseTimestamp(receipt?.timestamp_utc);
    if (Number.isFinite(currentTs) && Number.isFinite(receiptTs) && receiptTs < currentTs) continue;
    entry.state = nextState;
    entry.state_reason = `receipt:${normalizeReceiptKind(receipt?.receipt_kind)}`;
    entry.last_activity_at = String(receipt?.timestamp_utc || "").trim() || null;
    entry.last_receipt_kind = normalizeReceiptKind(receipt?.receipt_kind) || null;
    entry.last_actor_role = normalizeRole(receipt?.actor_role) || null;
    entry.correlation_id = correlationId || null;
    entry.review_mode = reviewModeFromEntry(receipt) || entry.review_mode;
  }

  for (const item of Array.isArray(runtimeStatus?.open_review_items) ? runtimeStatus.open_review_items : []) {
    const scopeRef = scopeRefFromEntry(item, correlationScopeRefs);
    if (!scopeRef) continue;
    const definition = resolvedMicrotaskForScopeRef(wpId, scopeRef, declaredMicrotasks);
    if (!definition) continue;
    const entry = byId.get(definition.mtId);
    if (!entry) continue;
    entry.state = "IN_REVIEW";
    entry.state_reason = `open_review_item:${normalizeReceiptKind(item?.receipt_kind)}`;
    entry.last_activity_at = String(item?.updated_at || item?.opened_at || entry.last_activity_at || "").trim() || entry.last_activity_at;
    entry.last_receipt_kind = normalizeReceiptKind(item?.receipt_kind) || entry.last_receipt_kind;
    entry.last_actor_role = normalizeRole(item?.opened_by_role) || entry.last_actor_role;
    entry.correlation_id = String(item?.correlation_id || "").trim() || entry.correlation_id;
    entry.review_mode = reviewModeFromEntry(item) || entry.review_mode;
  }

  const items = declaredMicrotasks.map((definition) => byId.get(definition.mtId));
  const rankedActive = [...items]
    .filter((entry) => entry.state !== "DECLARED")
    .sort((left, right) =>
      statePriority(right.state) - statePriority(left.state)
      || String(right.last_activity_at || "").localeCompare(String(left.last_activity_at || ""))
      || String(left.mt_id || "").localeCompare(String(right.mt_id || ""))
    );
  const attentionMicrotask = rankedActive[0] || null;
  const activeMicrotask = deriveActiveExecutionMicrotask(items);
  const previousMicrotask = derivePreviousExecutionMicrotask(items, activeMicrotask);

  let suggestedNextMicrotask = null;
  const deferredRepairMicrotask = activeMicrotask?.mt_id
    ? items
      .slice(0, items.findIndex((entry) => entry.mt_id === activeMicrotask.mt_id))
      .find((entry) => entry.state === "REPAIR_REQUIRED") || null
    : null;
  if (deferredRepairMicrotask) {
    suggestedNextMicrotask = deferredRepairMicrotask;
  } else if (activeMicrotask && ["ACTIVE", "DECLARED"].includes(activeMicrotask.state)) {
    const activeIndex = items.findIndex((entry) => entry.mt_id === activeMicrotask.mt_id);
    suggestedNextMicrotask = items.slice(activeIndex + 1).find((entry) => entry.state === "DECLARED") || null;
  }
  if (!suggestedNextMicrotask) {
    suggestedNextMicrotask = items.find((entry) => entry.state === "REPAIR_REQUIRED")
      || items.find((entry) => entry.state === "DECLARED" && entry.mt_id !== activeMicrotask?.mt_id)
      || null;
  }

  return {
    declared_count: items.length,
    active_microtask: activeMicrotask,
    previous_microtask: previousMicrotask,
    attention_microtask: attentionMicrotask,
    suggested_next_microtask: suggestedNextMicrotask,
    items,
  };
}
