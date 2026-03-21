import {
  DIRECT_REVIEW_CONTRACT_VERSION,
  DIRECT_REVIEW_HEALTH_GATE,
  DIRECT_REVIEW_PACKET_FORMAT_VERSION,
} from "./wp-communications-lib.mjs";

export const COMMUNICATION_HEALTH_STAGE_VALUES = ["STATUS", "KICKOFF", "HANDOFF", "VERDICT"];
export const COMMUNICATION_MONITOR_STATE_VALUES = [
  "COMM_NA",
  "COMM_MISCONFIGURED",
  "COMM_MISSING_KICKOFF",
  "COMM_WAITING_FOR_INTENT",
  "COMM_WAITING_FOR_HANDOFF",
  "COMM_WAITING_FOR_REVIEW",
  "COMM_BLOCKED_OPEN_ITEMS",
  "COMM_OK",
  "COMM_STALE",
];

function normalizeRole(value) {
  return String(value || "").trim().toUpperCase();
}

function normalizeReceiptKind(value) {
  return String(value || "").trim().toUpperCase();
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
    return replyCorrelation === correlationId || ackFor === correlationId;
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

function receiptFilter(receipts, { receiptKind, actorRole, targetRole }) {
  return (receipts || []).filter((entry) =>
    normalizeReceiptKind(entry.receipt_kind) === receiptKind
    && normalizeRole(entry.actor_role) === actorRole
    && normalizeRole(entry.target_role) === targetRole
  );
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
  openReviewItems = [],
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
    `open_review_items=${openReviewItems.length}`,
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
} = {}) {
  return {
    applicable: Boolean(applicable),
    ok: Boolean(ok),
    state,
    message,
    details,
    counts,
    correlations,
  };
}

export function evaluateWpCommunicationHealth({
  wpId = "",
  stage = "STATUS",
  packetPath = "",
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

  const kickoffIntentPair = latestMatchingPair(validatorKickoffs, coderIntents);
  const handoffReviewPair = latestMatchingPair(coderHandoffs, validatorReviews);
  const details = buildBaseDetails({
    wpId,
    stage: normalizedStage,
    packetPath,
    packetFormatVersion,
    validatorKickoffs,
    coderIntents,
    coderHandoffs,
    validatorReviews,
    openReviewItems,
  });
  const counts = {
    validatorKickoffs: validatorKickoffs.length,
    coderIntents: coderIntents.length,
    coderHandoffs: coderHandoffs.length,
    validatorReviews: validatorReviews.length,
    openReviewItems: openReviewItems.length,
  };
  const correlations = {
    kickoff: kickoffIntentPair?.openReceipt?.correlation_id || null,
    handoff: handoffReviewPair?.openReceipt?.correlation_id || null,
  };

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
    if (coderIntents.length === 0 || !kickoffIntentPair) {
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
    if (validatorReviews.length === 0 || !handoffReviewPair) {
      return result({
        applicable: true,
        ok: false,
        state: "COMM_WAITING_FOR_REVIEW",
        message: "Waiting on WP validator review reply",
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
        message: "Review exchange is complete, but open review items still block verdict",
        details: [
          ...details,
          ...openReviewItems.map((entry) =>
            `open_review_item=${entry.receipt_kind}:${entry.correlation_id}:${entry.summary}`
          ),
        ],
        counts,
        correlations,
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
    });
  }

  if (coderIntents.length === 0 || !kickoffIntentPair) {
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

  if (normalizedStage === "KICKOFF") {
    return result({
      applicable: true,
      ok: true,
      state: "COMM_OK",
      message: "Kickoff exchange is complete",
      details,
      counts,
      correlations,
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

  if (validatorReviews.length === 0 || !handoffReviewPair) {
    return result({
      applicable: true,
      ok: false,
      state: "COMM_WAITING_FOR_REVIEW",
      message: "Waiting on WP validator review reply",
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
      message: "Review exchange is complete, but open review items still block verdict",
      details: [
        ...details,
        ...openReviewItems.map((entry) =>
          `open_review_item=${entry.receipt_kind}:${entry.correlation_id}:${entry.summary}`
        ),
      ],
      counts,
      correlations,
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
  });
}

export function communicationMonitorState(evaluation, { stale = false } = {}) {
  if (!evaluation?.applicable) return "COMM_NA";
  if (stale) return "COMM_STALE";
  return COMMUNICATION_MONITOR_STATE_VALUES.includes(evaluation.state)
    ? evaluation.state
    : (evaluation.ok ? "COMM_OK" : "COMM_MISCONFIGURED");
}
