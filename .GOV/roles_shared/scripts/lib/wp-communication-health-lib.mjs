import {
  activeWorkflowInvalidityReceipt,
  DIRECT_REVIEW_SESSION_ROLE_VALUES,
  DIRECT_REVIEW_CONTRACT_VERSION,
  DIRECT_REVIEW_HEALTH_GATE,
  FINAL_AUTHORITY_DIRECT_REVIEW_PACKET_FORMAT_VERSION,
  DIRECT_REVIEW_PACKET_FORMAT_VERSION,
  OPERATOR_RULE_RESTATEMENT_INVALIDITY_CODE,
  REVIEW_RESOLUTION_RECEIPT_KIND_VALUES,
  REVIEW_TRACKED_RECEIPT_KIND_VALUES,
  workflowInvalidityReceipts,
} from "./wp-communications-lib.mjs";
import {
  normalizeRepoPath,
  parsePacketScopeList,
  parsePacketSingleField,
} from "./scope-surface-lib.mjs";

export const COMMUNICATION_HEALTH_STAGE_VALUES = ["STATUS", "KICKOFF", "HANDOFF", "VERDICT"];
export const COMMUNICATION_MONITOR_STATE_VALUES = [
  "COMM_NA",
  "COMM_MISCONFIGURED",
  "COMM_MISSING_KICKOFF",
  "COMM_WAITING_FOR_INTENT",
  "COMM_WAITING_FOR_INTENT_CHECKPOINT",
  "COMM_WAITING_FOR_HANDOFF",
  "COMM_REPAIR_REQUIRED",
  "COMM_WAITING_FOR_REVIEW",
  "COMM_WAITING_FOR_FINAL_REVIEW",
  "COMM_BLOCKED_OPEN_ITEMS",
  "COMM_WORKFLOW_INVALID",
  "COMM_OK",
  "COMM_STALE",
];

export const VALIDATOR_REVIEW_OUTCOME_VALUES = [
  "UNKNOWN",
  "REPAIR_REQUIRED",
  "APPROVED_FOR_FINAL_REVIEW",
];
export const VALIDATOR_ASSESSMENT_VERDICT_VALUES = ["ASSESSED", "FAIL", "PASS"];
const INTENT_CHECKPOINT_CLEARANCE_RECEIPT_KIND_VALUES = new Set(["VALIDATOR_RESPONSE", "SPEC_CONFIRMATION"]);
const CONTRACT_HEAVY_GOVERNED_VALIDATOR_REPORT_PROFILE_VALUES = new Set([
  "SPLIT_DIFF_SCOPED_RIGOR_V2",
  "SPLIT_DIFF_SCOPED_RIGOR_V3",
]);
const CONTRACT_HEAVY_CODER_HANDOFF_RIGOR_PROFILE_VALUES = new Set(["RUBRIC_SELF_AUDIT_V2"]);
const CONTRACT_HEAVY_CLAUSE_MONITOR_PROFILE_VALUES = new Set(["CLAUSE_MONITOR_V1"]);
const CONTRACT_HEAVY_SEMANTIC_PROOF_PROFILE_VALUES = new Set(["DIFF_SCOPED_SEMANTIC_V1"]);

function normalizeRole(value) {
  return String(value || "").trim().toUpperCase();
}

function normalizeReceiptKind(value) {
  return String(value || "").trim().toUpperCase();
}

function normalizeSession(value) {
  const raw = String(value || "").trim();
  if (!raw || /^<unassigned>$/i.test(raw)) return null;
  return raw || null;
}

function normalizeReviewOutcome(value) {
  const normalized = String(value || "").trim().toUpperCase();
  return VALIDATOR_REVIEW_OUTCOME_VALUES.includes(normalized) ? normalized : "UNKNOWN";
}

function normalizeAssessmentVerdict(value) {
  const normalized = String(value || "").trim().toUpperCase();
  return VALIDATOR_ASSESSMENT_VERDICT_VALUES.includes(normalized) ? normalized : "ASSESSED";
}

function parseListField(value) {
  return Array.isArray(value)
    ? value.map((entry) => String(entry || "").trim()).filter(Boolean)
    : [];
}

function isVersionAtLeast(current, minimum) {
  const currentValue = String(current || "").trim();
  const minimumValue = String(minimum || "").trim();
  if (!currentValue) return false;
  return currentValue.localeCompare(minimumValue) >= 0;
}

export function communicationContractApplies({
  workflowLane = "",
  packetFormatVersion = "",
  communicationContract = "",
} = {}) {
  return normalizeRole(workflowLane) === "ORCHESTRATOR_MANAGED"
    && (
      String(communicationContract || "").trim().toUpperCase() === DIRECT_REVIEW_CONTRACT_VERSION
      || isVersionAtLeast(packetFormatVersion, DIRECT_REVIEW_PACKET_FORMAT_VERSION)
    );
}

function matchingReply(openReceipt, resolutionReceipts) {
  const correlationId = String(openReceipt?.correlation_id || "").trim();
  if (!correlationId) return null;
  return resolutionReceipts.find((entry) => {
    const replyCorrelation = String(entry?.correlation_id || "").trim();
    const ackFor = String(entry?.ack_for || "").trim();
    const openActorRole = normalizeRole(openReceipt?.actor_role);
    const openTargetRole = normalizeRole(openReceipt?.target_role);
    const replyActorRole = normalizeRole(entry?.actor_role);
    const replyTargetRole = normalizeRole(entry?.target_role);
    if (replyCorrelation !== correlationId || ackFor !== correlationId) return false;
    if (openActorRole !== replyTargetRole || openTargetRole !== replyActorRole) return false;
    const directReviewSessions = DIRECT_REVIEW_SESSION_ROLE_VALUES.includes(openActorRole)
      && DIRECT_REVIEW_SESSION_ROLE_VALUES.includes(openTargetRole);
    if (!directReviewSessions) return true;
    const openActorSession = normalizeSession(openReceipt?.actor_session);
    const openTargetSession = normalizeSession(openReceipt?.target_session);
    const replyActorSession = normalizeSession(entry?.actor_session);
    const replyTargetSession = normalizeSession(entry?.target_session);
    if (!openActorSession || !replyActorSession || !replyTargetSession) return false;
    if (openActorSession !== replyTargetSession) return false;
    if (openTargetSession && openTargetSession !== replyActorSession) return false;
    return true;
  }) || null;
}

function latestMatchingPair(openReceipts, resolutionReceipts) {
  const ordered = [...openReceipts].sort((left, right) =>
    String(right.timestamp_utc || "").localeCompare(String(left.timestamp_utc || ""))
  );
  for (const openReceipt of ordered) {
    const reply = matchingReply(openReceipt, resolutionReceipts);
    if (reply) return { openReceipt, reply };
  }
  return null;
}

function latestOpenReceiptStatus(openReceipts, resolutionReceipts) {
  const ordered = [...(openReceipts || [])].sort((left, right) =>
    String(right.timestamp_utc || "").localeCompare(String(left.timestamp_utc || ""))
  );
  const openReceipt = ordered[0] || null;
  return {
    openReceipt,
    reply: openReceipt ? matchingReply(openReceipt, resolutionReceipts) : null,
  };
}

function summarySuggestsRepairRequired(summary) {
  const normalized = String(summary || "").trim();
  if (!normalized) return false;
  return /\brepair required\b/i.test(normalized)
    || /\bremediation required\b/i.test(normalized)
    || /\bplease repair\b/i.test(normalized)
    || /\brework required\b/i.test(normalized)
    || /\bfix required\b/i.test(normalized)
    || /\bre-handoff\b/i.test(normalized);
}

function summarySuggestsFinalReviewApproval(summary) {
  const normalized = String(summary || "").trim();
  if (!normalized) return false;
  return /\bsuitable for integration review\b/i.test(normalized)
    || /\bapproved for final review\b/i.test(normalized)
    || /\bready for final review\b/i.test(normalized)
    || /\binitiate final review\b/i.test(normalized)
    || /\bfinal review exchange\b/i.test(normalized)
    || /\badvisory review complete\b/i.test(normalized);
}

function explicitSummaryAssessmentVerdict(summary) {
  const normalized = String(summary || "").trim();
  if (!normalized) return null;
  if (/^FAIL\b/i.test(normalized)) return "FAIL";
  if (/^PASS\b/i.test(normalized)) return "PASS";
  return null;
}

function packetIsContractHeavy(packetContent = "") {
  const reportProfile = normalizeRole(parsePacketSingleField(packetContent, "GOVERNED_VALIDATOR_REPORT_PROFILE"));
  const handoffProfile = normalizeRole(parsePacketSingleField(packetContent, "CODER_HANDOFF_RIGOR_PROFILE"));
  const clauseMonitorProfile = normalizeRole(parsePacketSingleField(packetContent, "CLAUSE_CLOSURE_MONITOR_PROFILE"));
  const semanticProofProfile = normalizeRole(parsePacketSingleField(packetContent, "SEMANTIC_PROOF_PROFILE"));

  return CONTRACT_HEAVY_GOVERNED_VALIDATOR_REPORT_PROFILE_VALUES.has(reportProfile)
    || CONTRACT_HEAVY_CODER_HANDOFF_RIGOR_PROFILE_VALUES.has(handoffProfile)
    || CONTRACT_HEAVY_CLAUSE_MONITOR_PROFILE_VALUES.has(clauseMonitorProfile)
    || CONTRACT_HEAVY_SEMANTIC_PROOF_PROFILE_VALUES.has(semanticProofProfile)
    || /(^|\n)##\s+CLAUSE_CLOSURE_MATRIX\b/im.test(String(packetContent || ""));
}

function summarizeIntentCheckpointRequirement({
  packetContent = "",
  intentReceipt = null,
} = {}) {
  const contractHeavy = packetIsContractHeavy(packetContent);
  const inScopePaths = parsePacketScopeList(packetContent, "IN_SCOPE_PATHS", { stopLabels: ["OUT_OF_SCOPE"] });
  const hasSignedSurface = inScopePaths.length > 0;
  const intentContract = intentReceipt?.microtask_contract && typeof intentReceipt.microtask_contract === "object"
    ? intentReceipt.microtask_contract
    : null;
  const fileTargets = parseListField(intentContract?.file_targets).map((entry) => normalizeRepoPath(entry)).filter(Boolean);
  const proofCommands = parseListField(intentContract?.proof_commands);
  const missingSignedSurfaces = inScopePaths.filter((entry) => !fileTargets.includes(normalizeRepoPath(entry)));
  const reasons = [];

  if (contractHeavy) reasons.push("contract_heavy_packet");
  if (!intentContract && (contractHeavy || hasSignedSurface)) reasons.push("intent_missing_microtask_contract");
  if (hasSignedSurface && fileTargets.length === 0) reasons.push("intent_missing_file_targets");
  if (contractHeavy && proofCommands.length === 0) reasons.push("intent_missing_proof_commands");
  if (hasSignedSurface && fileTargets.length > 0 && missingSignedSurfaces.length > 0) {
    reasons.push(`intent_missing_signed_surfaces=${missingSignedSurfaces.slice(0, 6).join(", ")}`);
  }

  return {
    required: contractHeavy || reasons.some((entry) => !entry.startsWith("contract_heavy_packet")),
    contractHeavy,
    underSpecified: reasons.some((entry) => !entry.startsWith("contract_heavy_packet")),
    reasons,
  };
}

function latestIntentCheckpointClearance(receipts = [], intentReceipt = null) {
  const intentTimestamp = String(intentReceipt?.timestamp_utc || "").trim();
  if (!intentTimestamp) return null;
  return [...(receipts || [])]
    .filter((entry) =>
      normalizeRole(entry?.actor_role) === "WP_VALIDATOR"
      && normalizeRole(entry?.target_role) === "CODER"
      && INTENT_CHECKPOINT_CLEARANCE_RECEIPT_KIND_VALUES.has(normalizeReceiptKind(entry?.receipt_kind))
      && String(entry?.timestamp_utc || "").trim() > intentTimestamp
    )
    .sort((left, right) => String(right?.timestamp_utc || "").localeCompare(String(left?.timestamp_utc || "")))[0]
    || null;
}

export function deriveValidatorReviewOutcome(reviewReceipt = null) {
  const microtaskOutcome = normalizeReviewOutcome(reviewReceipt?.microtask_contract?.review_outcome);
  if (microtaskOutcome !== "UNKNOWN") return microtaskOutcome;
  if (summarySuggestsRepairRequired(reviewReceipt?.summary)) return "REPAIR_REQUIRED";
  if (summarySuggestsFinalReviewApproval(reviewReceipt?.summary)) return "APPROVED_FOR_FINAL_REVIEW";
  return "UNKNOWN";
}

export function deriveValidatorAssessmentVerdict(reviewReceipt = null) {
  const explicitVerdict = explicitSummaryAssessmentVerdict(reviewReceipt?.summary);
  if (explicitVerdict) return explicitVerdict;
  const outcome = deriveValidatorReviewOutcome(reviewReceipt);
  if (outcome === "REPAIR_REQUIRED") return "FAIL";
  if (outcome === "APPROVED_FOR_FINAL_REVIEW") return "PASS";
  return "ASSESSED";
}

export function deriveLatestValidatorAssessment(receipts = []) {
  const ordered = [...(receipts || [])].sort((left, right) =>
    String(right?.timestamp_utc || "").localeCompare(String(left?.timestamp_utc || ""))
  );
  for (const entry of ordered) {
    const actorRole = normalizeRole(entry?.actor_role);
    if (!["WP_VALIDATOR", "INTEGRATION_VALIDATOR", "VALIDATOR"].includes(actorRole)) continue;
    const receiptKind = normalizeReceiptKind(entry?.receipt_kind);
    if (!REVIEW_RESOLUTION_RECEIPT_KIND_VALUES.includes(receiptKind)) continue;
    return {
      receiptKind,
      actorRole,
      actorSession: normalizeSession(entry?.actor_session),
      targetRole: normalizeRole(entry?.target_role) || null,
      targetSession: normalizeSession(entry?.target_session),
      timestampUtc: String(entry?.timestamp_utc || "").trim() || null,
      reviewOutcome: deriveValidatorReviewOutcome(entry),
      verdict: normalizeAssessmentVerdict(deriveValidatorAssessmentVerdict(entry)),
      reason: String(entry?.summary || "").trim(),
      correlationId: String(entry?.correlation_id || "").trim() || null,
    };
  }
  return null;
}

function receiptFilter(receipts, { receiptKind, actorRole, targetRole }) {
  return (receipts || []).filter((entry) =>
    normalizeReceiptKind(entry.receipt_kind) === receiptKind
    && normalizeRole(entry.actor_role) === actorRole
    && normalizeRole(entry.target_role) === targetRole
  );
}

function receiptKindsFilter(receipts, { receiptKinds, actorRole, targetRole }) {
  const allowedKinds = new Set((receiptKinds || []).map((value) => normalizeReceiptKind(value)));
  return (receipts || []).filter((entry) =>
    allowedKinds.has(normalizeReceiptKind(entry.receipt_kind))
    && normalizeRole(entry.actor_role) === actorRole
    && normalizeRole(entry.target_role) === targetRole
  );
}

function requiresFinalAuthorityDirectReview(packetFormatVersion = "") {
  return isVersionAtLeast(packetFormatVersion, FINAL_AUTHORITY_DIRECT_REVIEW_PACKET_FORMAT_VERSION);
}

function buildBaseDetails({
  wpId = "",
  stage = "STATUS",
  packetPath = "",
  packetFormatVersion = "",
  validatorKickoffs = [],
  coderIntents = [],
  coderHandoffs = [],
  validatorReviews = [],
  integrationFinalOpenReceipts = [],
  integrationFinalResolutionReceipts = [],
  openReviewItems = [],
  workflowInvalidities = [],
} = {}) {
  return [
    `wp_id=${wpId || "<unknown>"}`,
    `stage=${stage}`,
    `packet=${packetPath || "<unknown>"}`,
    `packet_format_version=${packetFormatVersion || "<missing>"}`,
    `kickoffs=${validatorKickoffs.length}`,
    `coder_intents=${coderIntents.length}`,
    `coder_handoffs=${coderHandoffs.length}`,
    `validator_reviews=${validatorReviews.length}`,
    `integration_final_open=${integrationFinalOpenReceipts.length}`,
    `integration_final_resolution=${integrationFinalResolutionReceipts.length}`,
    `open_review_items=${openReviewItems.length}`,
    `workflow_invalidities=${workflowInvalidities.length}`,
  ];
}

function result({
  applicable,
  ok,
  state,
  message,
  details = [],
  counts = {},
  correlations = {},
  activeWorkflowInvalidityCode = null,
  latestWpValidatorReviewOutcome = "UNKNOWN",
} = {}) {
  return {
    applicable: Boolean(applicable),
    ok: Boolean(ok),
    state,
    message,
    details,
    counts,
    correlations,
    activeWorkflowInvalidityCode,
    latestWpValidatorReviewOutcome: normalizeReviewOutcome(latestWpValidatorReviewOutcome),
  };
}

export function evaluateWpCommunicationHealth({
  wpId = "",
  stage = "STATUS",
  packetPath = "",
  packetContent = "",
  workflowLane = "",
  packetFormatVersion = "",
  communicationContract = "",
  communicationHealthGate = "",
  receipts = [],
  runtimeStatus = {},
} = {}) {
  const normalizedStage = String(stage || "STATUS").trim().toUpperCase();
  if (!COMMUNICATION_HEALTH_STAGE_VALUES.includes(normalizedStage)) {
    throw new Error(`Invalid communication health stage: ${stage}`);
  }

  const contractApplies = communicationContractApplies({
    workflowLane,
    packetFormatVersion,
    communicationContract,
  });

  if (!contractApplies) {
    return result({
      applicable: false,
      ok: true,
      state: "COMM_NA",
      message: "Direct review contract is not active for this packet",
      details: [
        `wp_id=${wpId || "<unknown>"}`,
        `workflow_lane=${workflowLane || "<missing>"}`,
        `packet_format_version=${packetFormatVersion || "<missing>"}`,
      ],
    });
  }

  if (String(communicationHealthGate || "").trim().toUpperCase() !== DIRECT_REVIEW_HEALTH_GATE) {
    return result({
      applicable: true,
      ok: false,
      state: "COMM_MISCONFIGURED",
      message: "Direct review contract is active, but the required communication health gate is missing",
      details: [
        `wp_id=${wpId || "<unknown>"}`,
        `packet=${packetPath || "<unknown>"}`,
        `COMMUNICATION_HEALTH_GATE=${communicationHealthGate || "<missing>"}`,
        `expected=${DIRECT_REVIEW_HEALTH_GATE}`,
      ],
    });
  }

  const openReviewItems = Array.isArray(runtimeStatus?.open_review_items) ? runtimeStatus.open_review_items : [];
  const workflowInvalidities = workflowInvalidityReceipts(receipts);
  const activeWorkflowInvalidity = activeWorkflowInvalidityReceipt(receipts);
  const validatorKickoffs = receiptFilter(receipts, {
    receiptKind: "VALIDATOR_KICKOFF",
    actorRole: "WP_VALIDATOR",
    targetRole: "CODER",
  });
  const coderIntents = receiptFilter(receipts, {
    receiptKind: "CODER_INTENT",
    actorRole: "CODER",
    targetRole: "WP_VALIDATOR",
  });
  const coderHandoffs = receiptFilter(receipts, {
    receiptKind: "CODER_HANDOFF",
    actorRole: "CODER",
    targetRole: "WP_VALIDATOR",
  });
  const validatorReviews = receiptFilter(receipts, {
    receiptKind: "VALIDATOR_REVIEW",
    actorRole: "WP_VALIDATOR",
    targetRole: "CODER",
  });

  const kickoffIntentPair = latestOpenReceiptStatus(validatorKickoffs, coderIntents);
  const intentCheckpoint = summarizeIntentCheckpointRequirement({
    packetContent,
    intentReceipt: kickoffIntentPair.reply,
  });
  const intentCheckpointClearance = latestIntentCheckpointClearance(receipts, kickoffIntentPair.reply);
  const handoffReviewPair = latestOpenReceiptStatus(coderHandoffs, validatorReviews);
  const latestWpValidatorReviewOutcome = deriveValidatorReviewOutcome(handoffReviewPair.reply);
  const integrationFinalOpenReceipts = [
    ...receiptKindsFilter(receipts, {
      receiptKinds: ["CODER_HANDOFF", "REVIEW_REQUEST"],
      actorRole: "CODER",
      targetRole: "INTEGRATION_VALIDATOR",
    }),
    ...receiptKindsFilter(receipts, {
      receiptKinds: ["VALIDATOR_QUERY", "REVIEW_REQUEST", "SPEC_GAP"],
      actorRole: "INTEGRATION_VALIDATOR",
      targetRole: "CODER",
    }),
  ];
  const integrationFinalResolutionReceipts = [
    ...receiptKindsFilter(receipts, {
      receiptKinds: ["VALIDATOR_REVIEW", "VALIDATOR_RESPONSE", "REVIEW_RESPONSE", "SPEC_CONFIRMATION"],
      actorRole: "INTEGRATION_VALIDATOR",
      targetRole: "CODER",
    }),
    ...receiptKindsFilter(receipts, {
      receiptKinds: ["REVIEW_RESPONSE", "SPEC_CONFIRMATION", "VALIDATOR_RESPONSE"],
      actorRole: "CODER",
      targetRole: "INTEGRATION_VALIDATOR",
    }),
  ];
  const integrationFinalPair = latestOpenReceiptStatus(integrationFinalOpenReceipts, integrationFinalResolutionReceipts);
  const details = buildBaseDetails({
    wpId,
    stage: normalizedStage,
    packetPath,
    packetFormatVersion,
    validatorKickoffs,
    coderIntents,
    coderHandoffs,
    validatorReviews,
    integrationFinalOpenReceipts,
    integrationFinalResolutionReceipts,
    openReviewItems,
    workflowInvalidities,
  });
  const counts = {
    workflowInvalidities: workflowInvalidities.length,
    validatorKickoffs: validatorKickoffs.length,
    coderIntents: coderIntents.length,
    coderHandoffs: coderHandoffs.length,
    validatorReviews: validatorReviews.length,
    integrationFinalOpenReceipts: integrationFinalOpenReceipts.length,
    integrationFinalResolutionReceipts: integrationFinalResolutionReceipts.length,
    openReviewItems: openReviewItems.length,
  };
  const correlations = {
    kickoff: kickoffIntentPair.openReceipt?.correlation_id || null,
    handoff: handoffReviewPair.openReceipt?.correlation_id || null,
    finalReview: integrationFinalPair.openReceipt?.correlation_id || null,
  };

  if (activeWorkflowInvalidity) {
    return result({
      applicable: true,
      ok: false,
      state: "COMM_WORKFLOW_INVALID",
      message: "Workflow invalidity was recorded for this orchestrator-managed WP",
      details: [
        ...details,
        `latest_invalidity_code=${activeWorkflowInvalidity?.workflow_invalidity_code || "<missing>"}`,
        `latest_invalidity_summary=${activeWorkflowInvalidity?.summary || "<missing>"}`,
        `lane_reset_required=${String(activeWorkflowInvalidity?.workflow_invalidity_code || "").trim().toUpperCase() === OPERATOR_RULE_RESTATEMENT_INVALIDITY_CODE ? "YES" : "NO"}`,
      ],
      counts,
      correlations,
      activeWorkflowInvalidityCode: String(activeWorkflowInvalidity?.workflow_invalidity_code || "").trim().toUpperCase() || null,
      latestWpValidatorReviewOutcome,
    });
  }

  if (normalizedStage === "STATUS") {
    if (validatorKickoffs.length === 0) {
      return result({
        applicable: true,
        ok: false,
        state: "COMM_MISSING_KICKOFF",
        message: "Waiting on WP validator kickoff",
        details,
        counts,
        correlations,
      });
    }
    if (coderIntents.length === 0 || !kickoffIntentPair.reply) {
      return result({
        applicable: true,
        ok: false,
        state: "COMM_WAITING_FOR_INTENT",
        message: "Waiting on coder intent reply to the validator kickoff",
        details,
        counts,
        correlations,
      });
    }
    if (openReviewItems.length > 0) {
      return result({
        applicable: true,
        ok: false,
        state: "COMM_BLOCKED_OPEN_ITEMS",
        message: "Open review items still block direct review progression",
        details: [
          ...details,
          ...openReviewItems.map((entry) =>
            `open_review_item=${entry.receipt_kind}:${entry.correlation_id}:${entry.summary}`
          ),
        ],
        counts,
        correlations,
        latestWpValidatorReviewOutcome,
      });
    }
    if (intentCheckpoint.required && !intentCheckpointClearance) {
      return result({
        applicable: true,
        ok: false,
        state: "COMM_WAITING_FOR_INTENT_CHECKPOINT",
        message: "Waiting on WP validator checkpoint review of the coder intent before full handoff",
        details: [
          ...details,
          `intent_checkpoint_required=YES`,
          ...intentCheckpoint.reasons.map((reason) => `intent_checkpoint_reason=${reason}`),
        ],
        counts,
        correlations,
        latestWpValidatorReviewOutcome,
      });
    }
    if (coderHandoffs.length === 0) {
      return result({
        applicable: true,
        ok: false,
        state: "COMM_WAITING_FOR_HANDOFF",
        message: "Waiting on coder handoff to WP validator",
        details,
        counts,
        correlations,
      });
    }
    if (validatorReviews.length === 0 || !handoffReviewPair.reply) {
      return result({
        applicable: true,
        ok: false,
        state: "COMM_WAITING_FOR_REVIEW",
        message: "Waiting on WP validator review reply",
        details,
        counts,
        correlations,
        latestWpValidatorReviewOutcome,
      });
    }
    if (latestWpValidatorReviewOutcome === "REPAIR_REQUIRED") {
      return result({
        applicable: true,
        ok: false,
        state: "COMM_REPAIR_REQUIRED",
        message: "WP validator review requires coder remediation before re-handoff",
        details: [
          ...details,
          `wp_validator_review_outcome=${latestWpValidatorReviewOutcome}`,
        ],
        counts,
        correlations,
        latestWpValidatorReviewOutcome,
      });
    }
    if (openReviewItems.length > 0) {
      return result({
        applicable: true,
        ok: false,
        state: "COMM_BLOCKED_OPEN_ITEMS",
        message: "Open review items still block direct review progression",
        details: [
          ...details,
          ...openReviewItems.map((entry) =>
            `open_review_item=${entry.receipt_kind}:${entry.correlation_id}:${entry.summary}`
          ),
        ],
        counts,
        correlations,
        latestWpValidatorReviewOutcome,
      });
    }
    if (requiresFinalAuthorityDirectReview(packetFormatVersion) && !integrationFinalPair.reply) {
      return result({
        applicable: true,
        ok: false,
        state: "COMM_WAITING_FOR_FINAL_REVIEW",
        message: "Waiting on direct coder <-> integration-validator final review exchange",
        details,
        counts,
        correlations,
        latestWpValidatorReviewOutcome,
      });
    }
    return result({
      applicable: true,
      ok: true,
      state: "COMM_OK",
      message: "Direct review lane is complete",
      details,
      counts,
      correlations,
      latestWpValidatorReviewOutcome,
    });
  }

  if (validatorKickoffs.length === 0) {
    return result({
      applicable: true,
      ok: false,
      state: "COMM_MISSING_KICKOFF",
      message: "Waiting on WP validator kickoff",
      details,
      counts,
      correlations,
      latestWpValidatorReviewOutcome,
    });
  }

  if (coderIntents.length === 0 || !kickoffIntentPair.reply) {
    return result({
      applicable: true,
      ok: false,
      state: "COMM_WAITING_FOR_INTENT",
      message: "Waiting on coder intent reply to the validator kickoff",
      details,
      counts,
      correlations,
      latestWpValidatorReviewOutcome,
    });
  }

  if (normalizedStage === "KICKOFF") {
    return result({
      applicable: true,
      ok: true,
      state: "COMM_OK",
      message: "Kickoff exchange is complete",
      details,
      counts,
      correlations,
      latestWpValidatorReviewOutcome,
    });
  }

  if (openReviewItems.length > 0) {
    return result({
      applicable: true,
      ok: false,
      state: "COMM_BLOCKED_OPEN_ITEMS",
      message: "Open review items still block direct review progression",
      details: [
        ...details,
        ...openReviewItems.map((entry) =>
          `open_review_item=${entry.receipt_kind}:${entry.correlation_id}:${entry.summary}`
        ),
      ],
      counts,
      correlations,
      latestWpValidatorReviewOutcome,
    });
  }

  if (intentCheckpoint.required && !intentCheckpointClearance) {
    return result({
      applicable: true,
      ok: false,
      state: "COMM_WAITING_FOR_INTENT_CHECKPOINT",
      message: "Waiting on WP validator checkpoint review of the coder intent before full handoff",
      details: [
        ...details,
        `intent_checkpoint_required=YES`,
        ...intentCheckpoint.reasons.map((reason) => `intent_checkpoint_reason=${reason}`),
      ],
      counts,
      correlations,
      latestWpValidatorReviewOutcome,
    });
  }

  if (coderHandoffs.length === 0) {
    return result({
      applicable: true,
      ok: false,
      state: "COMM_WAITING_FOR_HANDOFF",
      message: "Waiting on coder handoff to WP validator",
      details,
      counts,
      correlations,
    });
  }

  if (normalizedStage === "HANDOFF") {
    return result({
      applicable: true,
      ok: true,
      state: "COMM_OK",
      message: "Handoff exchange is complete",
      details,
      counts,
      correlations,
    });
  }

  if (validatorReviews.length === 0 || !handoffReviewPair.reply) {
    return result({
      applicable: true,
      ok: false,
      state: "COMM_WAITING_FOR_REVIEW",
      message: "Waiting on WP validator review reply",
      details,
      counts,
      correlations,
      latestWpValidatorReviewOutcome,
    });
  }

  if (latestWpValidatorReviewOutcome === "REPAIR_REQUIRED") {
    return result({
      applicable: true,
      ok: false,
      state: "COMM_REPAIR_REQUIRED",
      message: "WP validator review requires coder remediation before re-handoff",
      details: [
        ...details,
        `wp_validator_review_outcome=${latestWpValidatorReviewOutcome}`,
      ],
      counts,
      correlations,
      latestWpValidatorReviewOutcome,
    });
  }

  if (requiresFinalAuthorityDirectReview(packetFormatVersion) && !integrationFinalPair.reply) {
    return result({
      applicable: true,
      ok: false,
      state: "COMM_WAITING_FOR_FINAL_REVIEW",
      message: "Waiting on direct coder <-> integration-validator final review exchange",
      details,
      counts,
      correlations,
      latestWpValidatorReviewOutcome,
    });
  }

  if (openReviewItems.length > 0) {
    return result({
      applicable: true,
      ok: false,
      state: "COMM_BLOCKED_OPEN_ITEMS",
      message: "Open review items still block direct review progression",
      details: [
        ...details,
        ...openReviewItems.map((entry) =>
          `open_review_item=${entry.receipt_kind}:${entry.correlation_id}:${entry.summary}`
        ),
      ],
      counts,
      correlations,
      latestWpValidatorReviewOutcome,
    });
  }

  return result({
    applicable: true,
    ok: true,
    state: "COMM_OK",
    message: "Direct review lane is complete",
    details,
    counts,
    correlations,
    latestWpValidatorReviewOutcome,
  });
}

export function communicationMonitorState(evaluation, { stale = false } = {}) {
  if (!evaluation?.applicable) return "COMM_NA";
  if (stale) return "COMM_STALE";
  return COMMUNICATION_MONITOR_STATE_VALUES.includes(evaluation.state)
    ? evaluation.state
    : (evaluation.ok ? "COMM_OK" : "COMM_MISCONFIGURED");
}

function mostRecentActiveSessionForRole(runtimeStatus, role) {
  const ROLE = normalizeRole(role);
  const sessions = Array.isArray(runtimeStatus?.active_role_sessions) ? runtimeStatus.active_role_sessions : [];
  return sessions
    .filter((entry) =>
      normalizeRole(entry?.role) === ROLE
      && normalizeSession(entry?.session_id)
      && String(entry?.state || "").trim().toLowerCase() !== "completed"
    )
    .sort((left, right) => String(right?.last_heartbeat_at || "").localeCompare(String(left?.last_heartbeat_at || "")))[0]
    ?.session_id || null;
}

function sessionForRole(runtimeStatus, role, preferredSession = null) {
  const ROLE = normalizeRole(role);
  const explicitSession = normalizeSession(preferredSession);
  if (explicitSession) return explicitSession;
  if (ROLE === "WP_VALIDATOR") {
    return normalizeSession(runtimeStatus?.wp_validator_of_record) || mostRecentActiveSessionForRole(runtimeStatus, ROLE);
  }
  if (ROLE === "INTEGRATION_VALIDATOR") {
    return normalizeSession(runtimeStatus?.integration_validator_of_record) || mostRecentActiveSessionForRole(runtimeStatus, ROLE);
  }
  return mostRecentActiveSessionForRole(runtimeStatus, ROLE);
}

function route({
  state,
  nextExpectedActor,
  nextExpectedSession = null,
  waitingOn,
  waitingOnSession = null,
  validatorTrigger = "NONE",
  validatorTriggerReason = null,
  readyForValidation = false,
  readyForValidationReason = null,
  attentionRequired = false,
  notificationSummary = null,
} = {}) {
  return {
    applicable: true,
    state,
    nextExpectedActor,
    nextExpectedSession: normalizeSession(nextExpectedSession),
    waitingOn,
    waitingOnSession: normalizeSession(waitingOnSession),
    validatorTrigger,
    validatorTriggerReason: validatorTrigger === "NONE" ? null : validatorTriggerReason,
    readyForValidation: Boolean(readyForValidation),
    readyForValidationReason: readyForValidation ? readyForValidationReason : null,
    attentionRequired: Boolean(attentionRequired),
    notificationSummary: notificationSummary ? String(notificationSummary).trim() : null,
  };
}

function latestInvalidityCode(latestReceipt = null) {
  return String(latestReceipt?.workflow_invalidity_code || "").trim().toUpperCase();
}

function sameRouteTarget(leftRole, leftSession, rightRole, rightSession) {
  return normalizeRole(leftRole) === normalizeRole(rightRole)
    && normalizeSession(leftSession) === normalizeSession(rightSession);
}

export function deriveWpCommunicationAutoRoute({
  evaluation,
  runtimeStatus = {},
  latestReceipt = null,
} = {}) {
  if (!evaluation?.applicable) {
    return {
      applicable: false,
      state: "COMM_NA",
      nextExpectedActor: null,
      nextExpectedSession: null,
      waitingOn: null,
      waitingOnSession: null,
      validatorTrigger: "NONE",
      validatorTriggerReason: null,
      readyForValidation: false,
      readyForValidationReason: null,
      attentionRequired: false,
      notification: null,
    };
  }

  const latestTargetRole = normalizeRole(latestReceipt?.target_role);
  const latestTargetSession = normalizeSession(latestReceipt?.target_session);
  const coderSession = sessionForRole(runtimeStatus, "CODER", latestTargetRole === "CODER" ? latestTargetSession : null);
  const wpValidatorSession = sessionForRole(runtimeStatus, "WP_VALIDATOR", latestTargetRole === "WP_VALIDATOR" ? latestTargetSession : null);
  const integrationValidatorSession = sessionForRole(
    runtimeStatus,
    "INTEGRATION_VALIDATOR",
    latestTargetRole === "INTEGRATION_VALIDATOR" ? latestTargetSession : null,
  );
  const openReviewItems = Array.isArray(runtimeStatus?.open_review_items) ? runtimeStatus.open_review_items : [];

  let projection;
  switch (evaluation.state) {
    case "COMM_MISCONFIGURED":
      projection = route({
        state: evaluation.state,
        nextExpectedActor: "ORCHESTRATOR",
        waitingOn: "COMMUNICATION_CONTRACT_REPAIR",
        attentionRequired: true,
        notificationSummary: "AUTO_ROUTE: communication contract misconfigured; orchestrator repair required",
      });
      break;
    case "COMM_MISSING_KICKOFF":
      projection = route({
        state: evaluation.state,
        nextExpectedActor: "WP_VALIDATOR",
        nextExpectedSession: wpValidatorSession,
        waitingOn: "VALIDATOR_KICKOFF",
        validatorTrigger: "BLOCKED_NEEDS_VALIDATOR",
        validatorTriggerReason: "WP validator kickoff is still missing",
        attentionRequired: true,
        notificationSummary: "AUTO_ROUTE: WP validator kickoff required",
      });
      break;
    case "COMM_WAITING_FOR_INTENT":
      projection = route({
        state: evaluation.state,
        nextExpectedActor: "CODER",
        nextExpectedSession: coderSession,
        waitingOn: "CODER_INTENT",
        notificationSummary: "AUTO_ROUTE: coder intent reply required",
      });
      break;
    case "COMM_WAITING_FOR_INTENT_CHECKPOINT":
      projection = route({
        state: evaluation.state,
        nextExpectedActor: "WP_VALIDATOR",
        nextExpectedSession: wpValidatorSession,
        waitingOn: "WP_VALIDATOR_INTENT_CHECKPOINT",
        waitingOnSession: wpValidatorSession,
        validatorTrigger: "BLOCKED_NEEDS_VALIDATOR",
        validatorTriggerReason: "Coder intent recorded; WP validator checkpoint review is required before full handoff",
        readyForValidation: true,
        readyForValidationReason: "Coder intent recorded; WP validator checkpoint review is required before full handoff",
        notificationSummary: "AUTO_ROUTE: WP validator checkpoint review required after coder intent",
      });
      break;
    case "COMM_WAITING_FOR_HANDOFF":
      projection = route({
        state: evaluation.state,
        nextExpectedActor: "CODER",
        nextExpectedSession: coderSession,
        waitingOn: "CODER_HANDOFF",
        notificationSummary: "AUTO_ROUTE: coder handoff required",
      });
      break;
    case "COMM_REPAIR_REQUIRED":
      projection = route({
        state: evaluation.state,
        nextExpectedActor: "CODER",
        nextExpectedSession: coderSession,
        waitingOn: "CODER_REPAIR_HANDOFF",
        notificationSummary: "AUTO_ROUTE: WP validator review requires coder remediation before re-handoff",
      });
      break;
    case "COMM_WAITING_FOR_REVIEW":
      projection = route({
        state: evaluation.state,
        nextExpectedActor: "WP_VALIDATOR",
        nextExpectedSession: wpValidatorSession,
        waitingOn: "WP_VALIDATOR_REVIEW",
        waitingOnSession: wpValidatorSession,
        validatorTrigger: "HANDOFF_READY",
        validatorTriggerReason: "Coder handoff recorded; WP validator review required",
        readyForValidation: true,
        readyForValidationReason: "Coder handoff recorded; WP validator review required",
        notificationSummary: "AUTO_ROUTE: WP validator review required after coder handoff",
      });
      break;
    case "COMM_WAITING_FOR_FINAL_REVIEW":
      projection = route({
        state: evaluation.state,
        nextExpectedActor: "CODER",
        nextExpectedSession: coderSession,
        waitingOn: "FINAL_REVIEW_EXCHANGE",
        notificationSummary: "AUTO_ROUTE: coder must initiate the final direct review exchange with Integration Validator",
      });
      break;
    case "COMM_BLOCKED_OPEN_ITEMS": {
      const nextItem = openReviewItems[0] || null;
      const targetRole = normalizeRole(nextItem?.target_role) || "ORCHESTRATOR";
      const targetSession = sessionForRole(runtimeStatus, targetRole, nextItem?.target_session ?? null);
      const needsValidator = targetRole === "WP_VALIDATOR" || targetRole === "INTEGRATION_VALIDATOR" || targetRole === "VALIDATOR";
      projection = route({
        state: evaluation.state,
        nextExpectedActor: targetRole,
        nextExpectedSession: targetSession,
        waitingOn: nextItem?.receipt_kind ? `OPEN_REVIEW_ITEM_${normalizeReceiptKind(nextItem.receipt_kind)}` : "OPEN_REVIEW_ITEM",
        waitingOnSession: targetSession,
        validatorTrigger: needsValidator ? "BLOCKED_NEEDS_VALIDATOR" : "NONE",
        validatorTriggerReason: needsValidator && nextItem
          ? `${normalizeReceiptKind(nextItem.receipt_kind)} requires ${targetRole} response`
          : null,
        attentionRequired: true,
        notificationSummary: nextItem
          ? `AUTO_ROUTE: open review item ${normalizeReceiptKind(nextItem.receipt_kind)} awaits ${targetRole}`
          : "AUTO_ROUTE: open review items still block verdict",
      });
      break;
    }
    case "COMM_WORKFLOW_INVALID":
      if ((evaluation.activeWorkflowInvalidityCode || latestInvalidityCode(latestReceipt)) === OPERATOR_RULE_RESTATEMENT_INVALIDITY_CODE) {
        projection = route({
          state: evaluation.state,
          nextExpectedActor: "ORCHESTRATOR",
          waitingOn: "LANE_RESET_REQUIRED",
          attentionRequired: true,
          notificationSummary: "AUTO_ROUTE: operator rule restatement recorded; orchestrator lane reset required",
        });
        break;
      }
      projection = route({
        state: evaluation.state,
        nextExpectedActor: "ORCHESTRATOR",
        waitingOn: "WORKFLOW_INVALIDITY",
        attentionRequired: true,
        notificationSummary: "AUTO_ROUTE: workflow invalidity flagged; orchestrator repair required",
      });
      break;
    case "COMM_OK":
      projection = route({
        state: evaluation.state,
        nextExpectedActor: "ORCHESTRATOR",
        waitingOn: "VERDICT_PROGRESSION",
        notificationSummary: "AUTO_ROUTE: direct review lane complete; orchestrator verdict progression ready",
      });
      break;
    default:
      projection = route({
        state: evaluation.state || "COMM_MISCONFIGURED",
        nextExpectedActor: "ORCHESTRATOR",
        waitingOn: "COMMUNICATION_REPAIR",
        attentionRequired: true,
        notificationSummary: "AUTO_ROUTE: communication state requires orchestrator repair",
      });
      break;
  }

  const notificationTargetRole = projection.nextExpectedActor;
  const notificationTargetSession = projection.nextExpectedSession;
  const notification = projection.notificationSummary
    && notificationTargetRole
    && notificationTargetRole !== "NONE"
    && normalizeRole(latestReceipt?.actor_role) !== notificationTargetRole
    && !sameRouteTarget(notificationTargetRole, notificationTargetSession, latestTargetRole, latestTargetSession)
      ? {
        targetRole: notificationTargetRole,
        targetSession: notificationTargetSession,
        summary: projection.notificationSummary,
      }
      : null;

  return {
    ...projection,
    notification,
  };
}

function nullableComparable(value) {
  const raw = String(value ?? "").trim();
  return raw || null;
}

function boundaryCorrelationId(statusEvaluation, runtimeStatus = {}) {
  switch (String(statusEvaluation?.state || "").trim().toUpperCase()) {
    case "COMM_WAITING_FOR_INTENT":
    case "COMM_WAITING_FOR_INTENT_CHECKPOINT":
    case "COMM_WAITING_FOR_HANDOFF":
      return nullableComparable(statusEvaluation?.correlations?.kickoff);
    case "COMM_REPAIR_REQUIRED":
    case "COMM_WAITING_FOR_REVIEW":
      return nullableComparable(statusEvaluation?.correlations?.handoff);
    case "COMM_WAITING_FOR_FINAL_REVIEW":
      return nullableComparable(statusEvaluation?.correlations?.finalReview);
    case "COMM_BLOCKED_OPEN_ITEMS":
      return nullableComparable(
        Array.isArray(runtimeStatus?.open_review_items) && runtimeStatus.open_review_items.length > 0
          ? runtimeStatus.open_review_items[0]?.correlation_id
          : statusEvaluation?.correlations?.finalReview,
      );
    default:
      return null;
  }
}

function boundaryActorRequiresAck(actorRole) {
  return ["CODER", "WP_VALIDATOR", "INTEGRATION_VALIDATOR"].includes(normalizeRole(actorRole));
}

function notificationMatchesBoundaryTarget(notification, actorRole, actorSession) {
  const targetRole = normalizeRole(notification?.target_role);
  const targetSession = normalizeSession(notification?.target_session);
  const expectedRole = normalizeRole(actorRole);
  const expectedSession = normalizeSession(actorSession);
  if (targetRole !== expectedRole) return false;
  if (!expectedSession || !targetSession) return true;
  return targetSession === expectedSession;
}

function notificationMatchesBoundaryCorrelation(notification, correlationId) {
  const expectedCorrelation = nullableComparable(correlationId);
  if (!expectedCorrelation) return true;
  const notificationCorrelation = nullableComparable(notification?.correlation_id);
  return notificationCorrelation === expectedCorrelation;
}

function notificationMatchesBoundaryKind(notification) {
  const sourceKind = normalizeReceiptKind(notification?.source_kind);
  return REVIEW_TRACKED_RECEIPT_KIND_VALUES.includes(sourceKind) || sourceKind === "AUTO_ROUTE";
}

export function evaluateWpCommunicationBoundary({
  stage = "STATUS",
  statusEvaluation,
  runtimeStatus = {},
  latestReceipt = null,
  pendingNotifications = [],
} = {}) {
  const normalizedStage = String(stage || "STATUS").trim().toUpperCase();
  if (!statusEvaluation?.applicable) {
    return {
      applicable: false,
      ok: true,
      autoRoute: null,
      boundaryNotifications: [],
      issues: [],
    };
  }

  const autoRoute = deriveWpCommunicationAutoRoute({
    evaluation: statusEvaluation,
    runtimeStatus,
    latestReceipt,
  });
  const issues = [];

  const compareField = (runtimeFieldName, expectedValue, formatter = (value) => nullableComparable(value)) => {
    const actualValue = formatter(runtimeStatus?.[runtimeFieldName]);
    const normalizedExpected = formatter(expectedValue);
    if (actualValue !== normalizedExpected) {
      issues.push(`runtime.${runtimeFieldName} expected ${normalizedExpected ?? "<null>"} but found ${actualValue ?? "<null>"}`);
    }
  };

  compareField("next_expected_actor", autoRoute.nextExpectedActor, normalizeRole);
  compareField("next_expected_session", autoRoute.nextExpectedSession, normalizeSession);
  compareField("waiting_on", autoRoute.waitingOn, nullableComparable);
  compareField("waiting_on_session", autoRoute.waitingOnSession, normalizeSession);
  compareField("validator_trigger", autoRoute.validatorTrigger, nullableComparable);
  compareField("validator_trigger_reason", autoRoute.validatorTriggerReason, nullableComparable);
  compareField("ready_for_validation", autoRoute.readyForValidation, (value) => Boolean(value));
  compareField("ready_for_validation_reason", autoRoute.readyForValidationReason, nullableComparable);
  compareField("attention_required", autoRoute.attentionRequired, (value) => Boolean(value));

  let boundaryNotifications = [];
  if (normalizedStage !== "STATUS" && boundaryActorRequiresAck(autoRoute.nextExpectedActor)) {
    const correlationId = boundaryCorrelationId(statusEvaluation, runtimeStatus);
    boundaryNotifications = (pendingNotifications || []).filter((entry) =>
      notificationMatchesBoundaryTarget(entry, autoRoute.nextExpectedActor, autoRoute.nextExpectedSession)
      && notificationMatchesBoundaryCorrelation(entry, correlationId)
      && notificationMatchesBoundaryKind(entry)
    );
    if (boundaryNotifications.length > 0) {
      issues.push(
        `Pending notifications for ${autoRoute.nextExpectedActor}${autoRoute.nextExpectedSession ? `:${autoRoute.nextExpectedSession}` : ""}`
        + ` must be acknowledged before ${normalizedStage} can pass`,
      );
    }
  }

  return {
    applicable: true,
    ok: issues.length === 0,
    autoRoute,
    boundaryNotifications,
    issues,
  };
}
