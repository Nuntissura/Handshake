function normalizeRole(value) {
  return String(value || "").trim().toUpperCase();
}

function normalizeSession(value) {
  const raw = String(value || "").trim();
  return raw || null;
}

function normalizeText(value, fallback = "<none>") {
  const raw = String(value || "").trim();
  return raw || fallback;
}

function formatEndpoint(role, session = null) {
  const normalizedRole = normalizeRole(role) || "UNKNOWN";
  const normalizedSession = normalizeSession(session);
  return normalizedSession ? `${normalizedRole}:${normalizedSession}` : normalizedRole;
}

export function preferredTargetSession(runtimeStatus = {}, governedSession = null) {
  return normalizeSession(runtimeStatus?.next_expected_session)
    || normalizeSession(governedSession?.session_id)
    || null;
}

function relayKindForSourceKind(sourceKind) {
  const normalized = normalizeRole(sourceKind);
  if (["CODER_HANDOFF", "HANDOFF"].includes(normalized)) return "HANDOFF";
  if (["VALIDATOR_QUERY", "REVIEW_REQUEST", "SPEC_GAP"].includes(normalized)) return "QUESTION";
  if (["VALIDATOR_RESPONSE", "REVIEW_RESPONSE", "SPEC_CONFIRMATION"].includes(normalized)) return "ANSWER";
  if (["VALIDATOR_REVIEW"].includes(normalized)) return "VERDICT";
  if (["CODER_INTENT", "VALIDATOR_KICKOFF"].includes(normalized)) return "INTENT";
  if (["THREAD_MESSAGE"].includes(normalized)) return "MESSAGE";
  if (["REPAIR"].includes(normalized)) return "REPAIR";
  if (["WORKFLOW_INVALIDITY"].includes(normalized)) return "INVALIDITY";
  return "ACTION";
}

function targetOpenReviewItem(runtimeStatus = {}, nextActor = "", targetSession = null) {
  const items = Array.isArray(runtimeStatus?.open_review_items) ? runtimeStatus.open_review_items : [];
  return items
    .filter((item) => normalizeRole(item?.target_role) === nextActor)
    .filter((item) => {
      const itemTargetSession = normalizeSession(item?.target_session);
      if (!targetSession) return true;
      if (!itemTargetSession) return true;
      return itemTargetSession === targetSession;
    })
    .sort((left, right) => String(right?.updated_at || right?.opened_at || "").localeCompare(String(left?.updated_at || left?.opened_at || "")))[0] || null;
}

function latestTargetNotification(notifications = []) {
  return [...(Array.isArray(notifications) ? notifications : [])]
    .sort((left, right) => String(right?.timestamp_utc || "").localeCompare(String(left?.timestamp_utc || "")))[0] || null;
}

export function deriveRelayEnvelope({
  wpId,
  runtimeStatus = {},
  nextActor = "",
  targetSession = null,
  notifications = {},
  dispatchAction = "SEND_PROMPT",
} = {}) {
  const notification = latestTargetNotification(notifications?.notifications || []);
  const reviewItem = targetOpenReviewItem(runtimeStatus, nextActor, targetSession);
  const sourceKind = normalizeRole(notification?.source_kind || reviewItem?.receipt_kind || runtimeStatus?.waiting_on || "ACTION");
  const relayKind = relayKindForSourceKind(sourceKind);
  const fromRole = normalizeRole(notification?.source_role || reviewItem?.opened_by_role || "RUNTIME");
  const fromSession = normalizeSession(notification?.source_session || reviewItem?.opened_by_session);
  const message = normalizeText(
    notification?.summary || reviewItem?.summary || `Runtime is waiting on ${runtimeStatus?.waiting_on || "the next governed action"}.`,
  );
  const correlationId = normalizeText(notification?.correlation_id || reviewItem?.correlation_id);
  const ackRequired = Boolean(reviewItem?.requires_ack);

  return {
    fromRole,
    fromSession,
    toRole: normalizeRole(nextActor),
    toSession: normalizeSession(targetSession),
    relayKind,
    sourceKind,
    correlationId,
    ackRequired,
    message,
    fromEndpoint: formatEndpoint(fromRole, fromSession),
    toEndpoint: formatEndpoint(nextActor, targetSession),
    operatorExplainer: [
      "Operator is broker-only on MANUAL_RELAY; do not mix this role message with hard-gate commentary.",
      `Runtime projects ${formatEndpoint(nextActor, targetSession)} next because waiting_on=${runtimeStatus?.waiting_on || "<missing>"} during ${runtimeStatus?.current_phase || "<missing>"}.`,
      `Dispatch action is ${dispatchAction}; after the role responds, rerun just manual-relay-next ${wpId}.`,
    ],
  };
}

export function buildRelayDispatchPrompt({
  basePrompt = "",
  envelope,
  contextLabel = "RELAY_CONTEXT [CX-RELAY-001]",
  messageLabel = "DIRECT_ROLE_MESSAGE [CX-RELAY-002]",
  terminalInstructions = [],
} = {}) {
  const prompt = String(basePrompt || "").trim();
  const relayEnvelope = envelope && typeof envelope === "object" ? envelope : {};
  const trailingInstructions = Array.isArray(terminalInstructions)
    ? terminalInstructions.map((entry) => String(entry || "").trim()).filter(Boolean)
    : [];
  return [
    prompt,
    "",
    contextLabel,
    `- FROM: ${relayEnvelope.fromEndpoint || formatEndpoint(relayEnvelope.fromRole, relayEnvelope.fromSession)}`,
    `- TO: ${relayEnvelope.toEndpoint || formatEndpoint(relayEnvelope.toRole, relayEnvelope.toSession)}`,
    `- RELAY_KIND: ${normalizeText(relayEnvelope.relayKind, "ACTION")}`,
    `- SOURCE_KIND: ${normalizeText(relayEnvelope.sourceKind, "ACTION")}`,
    `- CORRELATION_ID: ${normalizeText(relayEnvelope.correlationId)}`,
    `- ACK_REQUIRED: ${relayEnvelope.ackRequired ? "YES" : "NO"}`,
    "",
    messageLabel,
    `- ${normalizeText(relayEnvelope.message, "Review the active packet/runtime/notifications and perform the next governed action.")}`,
    "",
    ...trailingInstructions,
  ].join("\n");
}

export function deriveManualRelayEnvelope(args = {}) {
  return deriveRelayEnvelope(args);
}

export function buildManualRelayDispatchPrompt({ basePrompt = "", envelope } = {}) {
  const relayEnvelope = envelope && typeof envelope === "object" ? envelope : {};
  return buildRelayDispatchPrompt({
    basePrompt,
    envelope: relayEnvelope,
    contextLabel: "MANUAL_RELAY_CONTEXT [CX-MANUAL-RELAY-004]",
    messageLabel: "DIRECT_ROLE_MESSAGE [CX-MANUAL-RELAY-005]",
    terminalInstructions: [
      "Treat DIRECT_ROLE_MESSAGE as the current brokered role-to-role payload for WORKFLOW_LANE=MANUAL_RELAY.",
      "Do not answer as if the Operator authored the message; respond through the governed role lane.",
      `If you emit a paired acknowledgement, question, or response, preserve correlation_id=${normalizeText(relayEnvelope.correlationId)} when applicable.`,
    ],
  });
}
