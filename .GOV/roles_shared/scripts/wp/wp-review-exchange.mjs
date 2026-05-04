#!/usr/bin/env node

import crypto from "node:crypto";
import path from "node:path";
import { fileURLToPath } from "node:url";
import {
  communicationTransactionLockPathForWp,
  REVIEW_OPEN_RECEIPT_KIND_VALUES,
  REVIEW_RESOLUTION_RECEIPT_KIND_VALUES,
} from "../lib/wp-communications-lib.mjs";
import { resolveDeclaredWpMicrotaskByScopeRef } from "../lib/wp-microtask-lib.mjs";
import { mergeHeuristicRiskContract } from "../lib/heuristic-risk-lib.mjs";
import { isInvokedAsMain } from "../lib/invocation-path-lib.mjs";
import { withFileLockSync } from "../session/session-registry-lib.mjs";
import { appendWpReceipt, validateWpReceiptAppendPreconditions } from "./wp-receipt-append.mjs";
import { appendWpThreadEntry } from "./wp-thread-append.mjs";
import { registerFailCaptureHook, failWithMemory } from "../lib/fail-capture-lib.mjs";

registerFailCaptureHook("wp-review-exchange.mjs", { role: "SHARED" });

const SUPPORTED_RECEIPT_KINDS = [
  ...REVIEW_OPEN_RECEIPT_KIND_VALUES,
  ...REVIEW_RESOLUTION_RECEIPT_KIND_VALUES,
];
const EXPLICIT_REVIEW_ROLE_VALUES = ["CODER", "WP_VALIDATOR", "INTEGRATION_VALIDATOR"];

function fail(message) {
  failWithMemory("wp-review-exchange.mjs", message, { role: "SHARED" });
}

function normalizeRole(value) {
  return String(value || "").trim().toUpperCase();
}

function nullableValue(value) {
  const raw = String(value ?? "").trim();
  if (!raw || /^null$/i.test(raw) || /^none$/i.test(raw) || /^n\/a$/i.test(raw) || /^<unassigned>$/i.test(raw)) return null;
  return raw;
}

const REVIEW_CLI_OPTION_FIELDS = [
  "correlationId",
  "specAnchor",
  "packetRowRef",
  "ackFor",
  "microtaskJson",
];
const VALIDATOR_KICKOFF_CLI_OPTION_FIELDS = [
  "specAnchor",
  "packetRowRef",
  "microtaskJson",
];
const REVIEW_CLI_OPTION_FIELD_BY_KEY = new Map([
  ["correlation", "correlationId"],
  ["correlationid", "correlationId"],
  ["specanchor", "specAnchor"],
  ["packetrowref", "packetRowRef"],
  ["ackfor", "ackFor"],
  ["microtask", "microtaskJson"],
  ["microtaskjson", "microtaskJson"],
]);

function normalizedCliOptionKey(value) {
  return String(value || "").trim().toLowerCase().replace(/[-_]/g, "");
}

function parseNamedCliOption(raw, fieldByKey) {
  const match = String(raw ?? "").match(/^([A-Za-z][A-Za-z0-9_-]*)=(.*)$/s);
  if (!match) return null;
  const outerField = fieldByKey.get(normalizedCliOptionKey(match[1]));
  if (!outerField) return null;
  const innerMatch = String(match[2] ?? "").match(/^([A-Za-z][A-Za-z0-9_-]*)=(.*)$/s);
  if (innerMatch) {
    const innerField = fieldByKey.get(normalizedCliOptionKey(innerMatch[1]));
    if (innerField) return { field: innerField, value: innerMatch[2] };
  }
  return { field: outerField, value: match[2] };
}

export function parseReviewExchangeCliArgs(argv = []) {
  const [receiptKind, wpId, actorRole, actorSession, targetRole, targetSession, summary, ...rawOptions] = argv;
  const normalizedReceiptKind = String(receiptKind || "").trim().toUpperCase();
  const positionalFields = normalizedReceiptKind === "VALIDATOR_KICKOFF"
    ? VALIDATOR_KICKOFF_CLI_OPTION_FIELDS
    : REVIEW_CLI_OPTION_FIELDS;
  const parsed = {};
  const positional = [];
  for (const value of rawOptions) {
    const named = parseNamedCliOption(value, REVIEW_CLI_OPTION_FIELD_BY_KEY);
    if (named) {
      parsed[named.field] = named.value;
      continue;
    }
    positional.push(String(value ?? ""));
  }

  const compacted = normalizedReceiptKind === "VALIDATOR_KICKOFF"
    ? positional.filter((value) => String(value || "").trim())
    : positional;
  let positionalIndex = 0;
  for (const field of positionalFields) {
    if (parsed[field] !== undefined) continue;
    if (positionalIndex >= compacted.length) break;
    parsed[field] = compacted[positionalIndex];
    positionalIndex += 1;
  }

  return {
    receiptKind,
    wpId,
    actorRole,
    actorSession,
    targetRole,
    targetSession,
    summary,
    correlationId: parsed.correlationId,
    specAnchor: parsed.specAnchor,
    packetRowRef: parsed.packetRowRef,
    ackFor: parsed.ackFor,
    microtaskJson: parsed.microtaskJson,
  };
}

function inferTargetRole(receiptKind, actorRole) {
  const role = normalizeRole(actorRole);
  if (!EXPLICIT_REVIEW_ROLE_VALUES.includes(role)) return null;
  if (role === "CODER") return "WP_VALIDATOR";
  if (role === "WP_VALIDATOR" || role === "INTEGRATION_VALIDATOR") return "CODER";
  return null;
}

function requiresAck(receiptKind) {
  return REVIEW_OPEN_RECEIPT_KIND_VALUES.includes(receiptKind);
}

function buildCorrelationId(wpId, receiptKind) {
  return `review:${wpId}:${receiptKind.toLowerCase()}:${Date.now().toString(36)}:${crypto.randomBytes(3).toString("hex")}`;
}

function buildTargetLabel(targetRole, targetSession) {
  if (!targetRole) return "";
  return targetSession ? `${targetRole}:${targetSession}` : targetRole;
}

function inferMicrotaskScopeRef(packetRowRef, summary) {
  const rowRef = String(packetRowRef || "").trim();
  if (/^MT-\d{3}$/i.test(rowRef)) return rowRef.toUpperCase();
  const summaryMatch = String(summary || "").match(/\b(MT-\d{3})\b/i);
  return summaryMatch ? summaryMatch[1].toUpperCase() : null;
}

function inferReviewMode({ receiptKind, actorRole, targetRole, packetRowRef, summary }) {
  const normalizedSummary = String(summary || "");
  if (/review_mode\s*=\s*OVERLAP/i.test(normalizedSummary)) return "OVERLAP";
  if (
    /^REVIEW_REQUEST$/i.test(receiptKind || "")
    && normalizeRole(actorRole) === "CODER"
    && normalizeRole(targetRole) === "WP_VALIDATOR"
    && /^MT-\d{3}$/i.test(String(packetRowRef || "").trim())
  ) {
    return "OVERLAP";
  }
  return null;
}

export function deriveFallbackReviewMicrotaskContract({
  wpId,
  receiptKind,
  actorRole,
  targetRole,
  packetRowRef,
  summary,
  microtaskContract = null,
} = {}) {
  if (microtaskContract && typeof microtaskContract === "object" && !Array.isArray(microtaskContract)) {
    const scopeRef = String(microtaskContract.scope_ref || "").trim();
    if (!scopeRef) return microtaskContract;
    const resolution = resolveDeclaredWpMicrotaskByScopeRef(wpId, scopeRef);
    return resolution.match
      ? mergeHeuristicRiskContract(microtaskContract, resolution.match.heuristicRisk)
      : microtaskContract;
  }
  const normalizedReceiptKind = String(receiptKind || "").trim().toUpperCase();
  if (![
    ...REVIEW_OPEN_RECEIPT_KIND_VALUES,
    ...REVIEW_RESOLUTION_RECEIPT_KIND_VALUES,
  ].includes(normalizedReceiptKind)) {
    return null;
  }

  const scopeRef = inferMicrotaskScopeRef(packetRowRef, summary);
  if (!scopeRef) return null;

  const resolution = resolveDeclaredWpMicrotaskByScopeRef(wpId, scopeRef);
  if (!resolution.match) return null;

  const contract = {
    scope_ref: resolution.match.mtId,
    file_targets: resolution.match.codeSurfaces,
    proof_commands: resolution.match.expectedTests,
    phase_gate: "MICROTASK",
  };

  const reviewMode = inferReviewMode({
    receiptKind: normalizedReceiptKind,
    actorRole,
    targetRole,
    packetRowRef,
    summary,
  });
  if (reviewMode) contract.review_mode = reviewMode;
  if (normalizedReceiptKind === "REVIEW_REQUEST") {
    contract.expected_receipt_kind = "REVIEW_RESPONSE";
  }
  return mergeHeuristicRiskContract(contract, resolution.match.heuristicRisk);
}

function parseMicrotaskContract(value) {
  const raw = String(value ?? "").trim();
  if (!raw || /^null$/i.test(raw) || /^none$/i.test(raw) || /^n\/a$/i.test(raw)) return null;
  let parsed;
  try {
    parsed = JSON.parse(raw);
  } catch (error) {
    fail(`MICROTASK_JSON must be valid JSON: ${error.message}`);
  }
  if (!parsed || typeof parsed !== "object" || Array.isArray(parsed)) {
    fail("MICROTASK_JSON must decode to an object");
  }
  return parsed;
}

function buildThreadMessage({ receiptKind, summary, specAnchor, packetRowRef, correlationId, microtaskContract }) {
  const lines = [`${receiptKind}: ${summary}`];
  if (specAnchor) lines.push(`spec_anchor=${specAnchor}`);
  if (packetRowRef) lines.push(`packet_row_ref=${packetRowRef}`);
  if (microtaskContract?.scope_ref) lines.push(`microtask_scope_ref=${microtaskContract.scope_ref}`);
  if (Array.isArray(microtaskContract?.file_targets) && microtaskContract.file_targets.length > 0) {
    lines.push(`microtask_files=${microtaskContract.file_targets.join(", ")}`);
  }
  if (Array.isArray(microtaskContract?.proof_commands) && microtaskContract.proof_commands.length > 0) {
    lines.push(`microtask_proof=${microtaskContract.proof_commands.join(" ; ")}`);
  }
  if (microtaskContract?.risk_focus) lines.push(`microtask_risk=${microtaskContract.risk_focus}`);
  if (microtaskContract?.heuristic_risk) lines.push(`microtask_heuristic_risk=${microtaskContract.heuristic_risk}`);
  if (microtaskContract?.heuristic_risk_class) lines.push(`microtask_heuristic_class=${microtaskContract.heuristic_risk_class}`);
  if (Array.isArray(microtaskContract?.required_evidence) && microtaskContract.required_evidence.length > 0) {
    lines.push(`microtask_required_evidence=${microtaskContract.required_evidence.join(", ")}`);
  }
  if (microtaskContract?.strategy_escalation) lines.push(`microtask_strategy_escalation=${microtaskContract.strategy_escalation}`);
  if (microtaskContract?.expected_receipt_kind) lines.push(`microtask_expected_receipt=${microtaskContract.expected_receipt_kind}`);
  if (microtaskContract?.review_mode) lines.push(`microtask_review_mode=${microtaskContract.review_mode}`);
  if (microtaskContract?.phase_gate) lines.push(`microtask_phase_gate=${microtaskContract.phase_gate}`);
  if (microtaskContract?.review_outcome) lines.push(`microtask_review_outcome=${microtaskContract.review_outcome}`);
  lines.push(`correlation_id=${correlationId}`);
  return lines.join("\n");
}

export function requiresSplitCommittedCoderHandoffValidation({
  receiptKind = "",
  actorRole = "",
} = {}) {
  return String(receiptKind || "").trim().toUpperCase() === "CODER_HANDOFF"
    && normalizeRole(actorRole) === "CODER";
}

export function recordReviewExchange({
  receiptKind,
  wpId,
  actorRole,
  actorSession,
  targetRole = null,
  targetSession = null,
  summary,
  correlationId = null,
  specAnchor = null,
  packetRowRef = null,
  ackFor = null,
  microtaskJson = null,
} = {}) {
  const RECEIPT_KIND = String(receiptKind || "").trim().toUpperCase();
  const WP_ID = String(wpId || "").trim();
  const ACTOR_ROLE = normalizeRole(actorRole);
  const ACTOR_SESSION = String(actorSession || "").trim();
  const SUMMARY = String(summary || "").trim();
  const TARGET_ROLE = normalizeRole(targetRole) || inferTargetRole(RECEIPT_KIND, ACTOR_ROLE);
  const TARGET_SESSION = nullableValue(targetSession);
  const SPEC_ANCHOR = nullableValue(specAnchor);
  const PACKET_ROW_REF = nullableValue(packetRowRef);
  const MICROTASK_CONTRACT = deriveFallbackReviewMicrotaskContract({
    wpId: WP_ID,
    receiptKind: RECEIPT_KIND,
    actorRole: ACTOR_ROLE,
    targetRole: TARGET_ROLE,
    packetRowRef: PACKET_ROW_REF,
    summary: SUMMARY,
    microtaskContract: parseMicrotaskContract(microtaskJson),
  });
  let ACK_FOR = nullableValue(ackFor);

  if (!SUPPORTED_RECEIPT_KINDS.includes(RECEIPT_KIND)) {
    fail(`Unsupported review receipt kind: ${RECEIPT_KIND}`);
  }
  if (!WP_ID || !/^WP-/.test(WP_ID)) fail("WP_ID is required");
  if (!EXPLICIT_REVIEW_ROLE_VALUES.includes(ACTOR_ROLE)) {
    fail(`ACTOR_ROLE must be one of ${EXPLICIT_REVIEW_ROLE_VALUES.join(", ")}`);
  }
  if (!ACTOR_SESSION) fail("ACTOR_SESSION is required");
  if (!SUMMARY) fail("SUMMARY is required");
  if (!TARGET_ROLE || !EXPLICIT_REVIEW_ROLE_VALUES.includes(TARGET_ROLE)) {
    fail(`TARGET_ROLE must resolve to one of ${EXPLICIT_REVIEW_ROLE_VALUES.join(", ")}`);
  }

  const CORRELATION_ID = nullableValue(correlationId)
    || (REVIEW_OPEN_RECEIPT_KIND_VALUES.includes(RECEIPT_KIND) ? buildCorrelationId(WP_ID, RECEIPT_KIND) : null);
  if (!CORRELATION_ID) {
    fail(`CORRELATION_ID is required for ${RECEIPT_KIND}`);
  }
  if (!ACK_FOR && REVIEW_RESOLUTION_RECEIPT_KIND_VALUES.includes(RECEIPT_KIND)) {
    ACK_FOR = CORRELATION_ID;
  }

  const validationArgs = {
    wpId: WP_ID,
    actorRole: ACTOR_ROLE,
    actorSession: ACTOR_SESSION,
    receiptKind: RECEIPT_KIND,
    summary: SUMMARY,
    targetRole: TARGET_ROLE,
    targetSession: TARGET_SESSION,
    correlationId: CORRELATION_ID,
    requiresAck: requiresAck(RECEIPT_KIND),
    ackFor: ACK_FOR,
    specAnchor: SPEC_ANCHOR,
    packetRowRef: PACKET_ROW_REF,
    microtaskContract: MICROTASK_CONTRACT,
  };
  const splitCommittedCoderHandoffValidation = requiresSplitCommittedCoderHandoffValidation({
    receiptKind: RECEIPT_KIND,
    actorRole: ACTOR_ROLE,
  });

  if (splitCommittedCoderHandoffValidation) {
    // The committed coder handoff preflight runs STARTUP/HANDOFF checks that
    // re-enter WP communication helpers. Run that preflight before the outer
    // transaction lock so the final in-lock route validation does not self-deadlock.
    validateWpReceiptAppendPreconditions(validationArgs);
  }

  return withFileLockSync(communicationTransactionLockPathForWp(WP_ID), () => {
    validateWpReceiptAppendPreconditions(validationArgs, splitCommittedCoderHandoffValidation
      ? { skipCommittedCoderHandoffGate: true }
      : {});

    const threadResult = appendWpThreadEntry({
      wpId: WP_ID,
      actorRole: ACTOR_ROLE,
      actorSession: ACTOR_SESSION,
      message: buildThreadMessage({
        receiptKind: RECEIPT_KIND,
        summary: SUMMARY,
        specAnchor: SPEC_ANCHOR,
        packetRowRef: PACKET_ROW_REF,
        correlationId: CORRELATION_ID,
        microtaskContract: MICROTASK_CONTRACT,
      }),
      target: buildTargetLabel(TARGET_ROLE, TARGET_SESSION),
      recordReceipt: false,
      emitNotification: false,
      targetRole: TARGET_ROLE,
      targetSession: TARGET_SESSION,
      correlationId: CORRELATION_ID,
      requiresAck: requiresAck(RECEIPT_KIND),
      ackFor: ACK_FOR,
      specAnchor: SPEC_ANCHOR,
      packetRowRef: PACKET_ROW_REF,
      microtaskContract: MICROTASK_CONTRACT,
    }, { assumeTransactionLock: true });

    const receiptResult = appendWpReceipt({
      wpId: WP_ID,
      actorRole: ACTOR_ROLE,
      actorSession: ACTOR_SESSION,
      receiptKind: RECEIPT_KIND,
      summary: SUMMARY,
      targetRole: TARGET_ROLE,
      targetSession: TARGET_SESSION,
      correlationId: CORRELATION_ID,
      requiresAck: requiresAck(RECEIPT_KIND),
      ackFor: ACK_FOR,
      specAnchor: SPEC_ANCHOR,
      packetRowRef: PACKET_ROW_REF,
      microtaskContract: MICROTASK_CONTRACT,
      refs: [threadResult.threadFile],
    }, { assumeTransactionLock: true, skipPreflight: true });

    return {
      correlationId: CORRELATION_ID,
      threadFile: threadResult.threadFile,
      receiptsFile: receiptResult.context.receiptsFile,
      runtimeStatusFile: receiptResult.context.runtimeStatusFile,
      receipt: receiptResult.entry,
      microtaskContract: MICROTASK_CONTRACT,
    };
  });
}

function runCli() {
  const {
    receiptKind,
    wpId,
    actorRole,
    actorSession,
    targetRole,
    targetSession,
    summary,
    correlationId,
    specAnchor,
    packetRowRef,
    ackFor,
    microtaskJson,
  } = parseReviewExchangeCliArgs(process.argv.slice(2));
  if (!receiptKind || !wpId || !actorRole || !actorSession || !summary) {
    console.error(
      "Usage: node .GOV/roles_shared/scripts/wp/wp-review-exchange.mjs"
      + " <RECEIPT_KIND> WP-{ID} <ACTOR_ROLE> <ACTOR_SESSION> <TARGET_ROLE> <TARGET_SESSION>"
      + " \"<SUMMARY>\" [CORRELATION_ID] [SPEC_ANCHOR] [PACKET_ROW_REF] [ACK_FOR] [MICROTASK_JSON]"
    );
    process.exit(1);
  }

  const result = recordReviewExchange({
    receiptKind,
    wpId,
    actorRole,
    actorSession,
    targetRole,
    targetSession,
    summary,
      correlationId,
      specAnchor,
      packetRowRef,
      ackFor,
      microtaskJson,
    });

  console.log(`[WP_REVIEW_EXCHANGE] appended ${String(receiptKind).trim().toUpperCase()} for ${wpId}`);
  console.log(`- correlation_id: ${result.correlationId}`);
  console.log(`- thread: ${result.threadFile}`);
  console.log(`- receipts: ${result.receiptsFile}`);
  console.log(`- runtime: ${result.runtimeStatusFile}`);
  if (result.microtaskContract) {
    console.log(`- microtask_contract: ${JSON.stringify(result.microtaskContract)}`);
  }
}

if (isInvokedAsMain(import.meta.url, process.argv[1])) {
  runCli();
}
