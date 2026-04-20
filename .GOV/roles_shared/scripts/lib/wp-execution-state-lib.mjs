import { isTerminalPacketStatus, isTerminalTaskBoardStatus, taskBoardStatusForPacketStatus } from "./wp-authority-projection-lib.mjs";

export const EXECUTION_STATE_SCHEMA_VERSION = "wp_execution_state@1";
export const EXECUTION_STATE_LINEAGE_SCHEMA_VERSION = "wp_execution_checkpoint_lineage@1";

const CHECKPOINT_HISTORY_LIMIT = 32;
const EXECUTION_CLOSEOUT_MODE_SPECS = Object.freeze({
  MERGE_PENDING: Object.freeze({
    mode: "MERGE_PENDING",
    task_board_status: "DONE_MERGE_PENDING",
    packet_status: "Done",
    main_containment_status: "MERGE_PENDING",
    require_merged_main_commit: false,
    required_validation_verdict: "PASS",
  }),
  CONTAINED_IN_MAIN: Object.freeze({
    mode: "CONTAINED_IN_MAIN",
    task_board_status: "DONE_VALIDATED",
    packet_status: "Validated (PASS)",
    main_containment_status: "CONTAINED_IN_MAIN",
    require_merged_main_commit: true,
    required_validation_verdict: "PASS",
  }),
  FAIL: Object.freeze({
    mode: "FAIL",
    task_board_status: "DONE_FAIL",
    packet_status: "Validated (FAIL)",
    main_containment_status: "NOT_REQUIRED",
    require_merged_main_commit: false,
    required_validation_verdict: "FAIL",
  }),
  OUTDATED_ONLY: Object.freeze({
    mode: "OUTDATED_ONLY",
    task_board_status: "DONE_OUTDATED_ONLY",
    packet_status: "Validated (OUTDATED_ONLY)",
    main_containment_status: "NOT_REQUIRED",
    require_merged_main_commit: false,
    required_validation_verdict: "OUTDATED_ONLY",
  }),
  ABANDONED: Object.freeze({
    mode: "ABANDONED",
    task_board_status: "DONE_ABANDONED",
    packet_status: "Validated (ABANDONED)",
    main_containment_status: "NOT_REQUIRED",
    require_merged_main_commit: false,
    required_validation_verdict: "ABANDONED",
  }),
});
const EXECUTION_CLOSEOUT_MODE_ALIASES = new Map([
  ["MERGE_PENDING", "MERGE_PENDING"],
  ["DONE_MERGE_PENDING", "MERGE_PENDING"],
  ["CONTAINED_IN_MAIN", "CONTAINED_IN_MAIN"],
  ["DONE_VALIDATED", "CONTAINED_IN_MAIN"],
  ["FAIL", "FAIL"],
  ["DONE_FAIL", "FAIL"],
  ["OUTDATED_ONLY", "OUTDATED_ONLY"],
  ["DONE_OUTDATED_ONLY", "OUTDATED_ONLY"],
  ["ABANDONED", "ABANDONED"],
  ["DONE_ABANDONED", "ABANDONED"],
]);

function isPlainObject(value) {
  return !!value && typeof value === "object" && !Array.isArray(value);
}

function normalizeNullableString(value) {
  if (value === null || value === undefined) return null;
  const raw = String(value).trim();
  if (!raw) return null;
  return raw;
}

function normalizeNullableInteger(value) {
  if (value === null || value === undefined || value === "") return null;
  const parsed = Number.parseInt(String(value), 10);
  return Number.isInteger(parsed) ? parsed : null;
}

function normalizeBoolean(value, fallback = false) {
  return typeof value === "boolean" ? value : fallback;
}

function cloneExecutionCloseoutModeSpec(spec = null) {
  return spec ? { ...spec } : null;
}

function sanitizeEventToken(value) {
  const raw = String(value || "").trim().toLowerCase().replace(/[^a-z0-9]+/g, "_").replace(/^_+|_+$/g, "");
  return raw || "runtime_sync";
}

function checkpointKindFromEventName(value) {
  const raw = String(value || "").trim().toUpperCase().replace(/[^A-Z0-9]+/g, "_").replace(/^_+|_+$/g, "");
  return raw || "SYNC";
}

function checkpointTimestampToken(value) {
  return String(value || "")
    .replace(/[-:.]/g, "")
    .replace("T", "t")
    .replace("Z", "z");
}

function buildCheckpointId(eventName, eventAt, nextIndex) {
  return `cp-${String(nextIndex).padStart(4, "0")}-${sanitizeEventToken(eventName)}-${checkpointTimestampToken(eventAt)}`;
}

function cloneCheckpointList(value) {
  return Array.isArray(value) ? value.map((entry) => ({ ...entry })) : [];
}

function normalizeRouteAnchor(routeAnchor = {}) {
  const source = isPlainObject(routeAnchor) ? routeAnchor : {};
  return {
    state: normalizeNullableString(source.state),
    kind: normalizeNullableString(source.kind),
    correlation_id: normalizeNullableString(source.correlation_id),
    target_role: normalizeNullableString(source.target_role),
    target_session: normalizeNullableString(source.target_session),
  };
}

function normalizeReviewAnchor(reviewAnchor = {}) {
  const source = isPlainObject(reviewAnchor) ? reviewAnchor : {};
  return {
    receipt_kind: normalizeNullableString(source.receipt_kind),
    correlation_id: normalizeNullableString(source.correlation_id),
    actor_session: normalizeNullableString(source.actor_session),
    target_session: normalizeNullableString(source.target_session),
    round: normalizeNullableInteger(source.round),
    committed_handoff_base_sha: normalizeNullableString(source.committed_handoff_base_sha),
    committed_handoff_head_sha: normalizeNullableString(source.committed_handoff_head_sha),
    committed_handoff_range_source: normalizeNullableString(source.committed_handoff_range_source),
  };
}

export function buildExecutionAuthorityFromRuntime(runtimeStatus = {}) {
  return {
    packet_status: normalizeNullableString(runtimeStatus.current_packet_status),
    task_board_status: normalizeNullableString(runtimeStatus.current_task_board_status),
    milestone: normalizeNullableString(runtimeStatus.current_milestone),
    phase: normalizeNullableString(runtimeStatus.current_phase),
    runtime_status: normalizeNullableString(runtimeStatus.runtime_status),
    next_expected_actor: normalizeNullableString(runtimeStatus.next_expected_actor),
    next_expected_session: normalizeNullableString(runtimeStatus.next_expected_session),
    waiting_on: normalizeNullableString(runtimeStatus.waiting_on),
    waiting_on_session: normalizeNullableString(runtimeStatus.waiting_on_session),
    validator_trigger: normalizeNullableString(runtimeStatus.validator_trigger),
    validator_trigger_reason: normalizeNullableString(runtimeStatus.validator_trigger_reason),
    attention_required: normalizeBoolean(runtimeStatus.attention_required),
    ready_for_validation: normalizeBoolean(runtimeStatus.ready_for_validation),
    ready_for_validation_reason: normalizeNullableString(runtimeStatus.ready_for_validation_reason),
    wp_validator_of_record: normalizeNullableString(runtimeStatus.wp_validator_of_record),
    integration_validator_of_record: normalizeNullableString(runtimeStatus.integration_validator_of_record),
    main_containment_status: normalizeNullableString(runtimeStatus.main_containment_status),
    merged_main_commit: normalizeNullableString(runtimeStatus.merged_main_commit),
    main_containment_verified_at_utc: normalizeNullableString(runtimeStatus.main_containment_verified_at_utc),
    current_main_compatibility_status: normalizeNullableString(runtimeStatus.current_main_compatibility_status),
    current_main_compatibility_baseline_sha: normalizeNullableString(runtimeStatus.current_main_compatibility_baseline_sha),
    current_main_compatibility_verified_at_utc: normalizeNullableString(runtimeStatus.current_main_compatibility_verified_at_utc),
    packet_widening_decision: normalizeNullableString(runtimeStatus.packet_widening_decision),
    packet_widening_evidence: normalizeNullableString(runtimeStatus.packet_widening_evidence),
    route_anchor: normalizeRouteAnchor({
      state: runtimeStatus.route_anchor_state,
      kind: runtimeStatus.route_anchor_kind,
      correlation_id: runtimeStatus.route_anchor_correlation_id,
      target_role: runtimeStatus.route_anchor_target_role,
      target_session: runtimeStatus.route_anchor_target_session,
    }),
    review_anchor: normalizeReviewAnchor({
      receipt_kind: runtimeStatus.authoritative_review_receipt_kind,
      correlation_id: runtimeStatus.authoritative_review_correlation_id,
      actor_session: runtimeStatus.authoritative_review_actor_session,
      target_session: runtimeStatus.authoritative_review_target_session,
      round: runtimeStatus.authoritative_review_round,
      committed_handoff_base_sha: runtimeStatus.committed_handoff_base_sha,
      committed_handoff_head_sha: runtimeStatus.committed_handoff_head_sha,
      committed_handoff_range_source: runtimeStatus.committed_handoff_range_source,
    }),
  };
}

export function readRuntimeExecutionAuthority(runtimeStatus = {}) {
  const runtimeAuthority = buildExecutionAuthorityFromRuntime(runtimeStatus);
  const executionStateAuthority = isPlainObject(runtimeStatus?.execution_state?.authority)
    ? runtimeStatus.execution_state.authority
    : {};
  return mergeExecutionAuthority(runtimeAuthority, executionStateAuthority);
}

function mergeExecutionAuthority(previousAuthority = {}, nextAuthority = {}) {
  const previousRouteAnchor = normalizeRouteAnchor(previousAuthority.route_anchor);
  const previousReviewAnchor = normalizeReviewAnchor(previousAuthority.review_anchor);
  const currentRouteAnchor = normalizeRouteAnchor(nextAuthority.route_anchor);
  const currentReviewAnchor = normalizeReviewAnchor(nextAuthority.review_anchor);

  return {
    packet_status: normalizeNullableString(nextAuthority.packet_status ?? previousAuthority.packet_status),
    task_board_status: normalizeNullableString(nextAuthority.task_board_status ?? previousAuthority.task_board_status),
    milestone: normalizeNullableString(nextAuthority.milestone ?? previousAuthority.milestone),
    phase: normalizeNullableString(nextAuthority.phase ?? previousAuthority.phase),
    runtime_status: normalizeNullableString(nextAuthority.runtime_status ?? previousAuthority.runtime_status),
    next_expected_actor: normalizeNullableString(nextAuthority.next_expected_actor ?? previousAuthority.next_expected_actor),
    next_expected_session: normalizeNullableString(nextAuthority.next_expected_session ?? previousAuthority.next_expected_session),
    waiting_on: normalizeNullableString(nextAuthority.waiting_on ?? previousAuthority.waiting_on),
    waiting_on_session: normalizeNullableString(nextAuthority.waiting_on_session ?? previousAuthority.waiting_on_session),
    validator_trigger: normalizeNullableString(nextAuthority.validator_trigger ?? previousAuthority.validator_trigger),
    validator_trigger_reason: normalizeNullableString(nextAuthority.validator_trigger_reason ?? previousAuthority.validator_trigger_reason),
    attention_required: normalizeBoolean(
      nextAuthority.attention_required,
      normalizeBoolean(previousAuthority.attention_required),
    ),
    ready_for_validation: normalizeBoolean(
      nextAuthority.ready_for_validation,
      normalizeBoolean(previousAuthority.ready_for_validation),
    ),
    ready_for_validation_reason: normalizeNullableString(
      nextAuthority.ready_for_validation_reason ?? previousAuthority.ready_for_validation_reason,
    ),
    wp_validator_of_record: normalizeNullableString(
      nextAuthority.wp_validator_of_record ?? previousAuthority.wp_validator_of_record,
    ),
    integration_validator_of_record: normalizeNullableString(
      nextAuthority.integration_validator_of_record ?? previousAuthority.integration_validator_of_record,
    ),
    main_containment_status: normalizeNullableString(
      nextAuthority.main_containment_status ?? previousAuthority.main_containment_status,
    ),
    merged_main_commit: normalizeNullableString(nextAuthority.merged_main_commit ?? previousAuthority.merged_main_commit),
    main_containment_verified_at_utc: normalizeNullableString(
      nextAuthority.main_containment_verified_at_utc ?? previousAuthority.main_containment_verified_at_utc,
    ),
    current_main_compatibility_status: normalizeNullableString(
      nextAuthority.current_main_compatibility_status ?? previousAuthority.current_main_compatibility_status,
    ),
    current_main_compatibility_baseline_sha: normalizeNullableString(
      nextAuthority.current_main_compatibility_baseline_sha ?? previousAuthority.current_main_compatibility_baseline_sha,
    ),
    current_main_compatibility_verified_at_utc: normalizeNullableString(
      nextAuthority.current_main_compatibility_verified_at_utc ?? previousAuthority.current_main_compatibility_verified_at_utc,
    ),
    packet_widening_decision: normalizeNullableString(
      nextAuthority.packet_widening_decision ?? previousAuthority.packet_widening_decision,
    ),
    packet_widening_evidence: normalizeNullableString(
      nextAuthority.packet_widening_evidence ?? previousAuthority.packet_widening_evidence,
    ),
    route_anchor: {
      state: currentRouteAnchor.state ?? previousRouteAnchor.state,
      kind: currentRouteAnchor.kind ?? previousRouteAnchor.kind,
      correlation_id: currentRouteAnchor.correlation_id ?? previousRouteAnchor.correlation_id,
      target_role: currentRouteAnchor.target_role ?? previousRouteAnchor.target_role,
      target_session: currentRouteAnchor.target_session ?? previousRouteAnchor.target_session,
    },
    review_anchor: {
      receipt_kind: currentReviewAnchor.receipt_kind ?? previousReviewAnchor.receipt_kind,
      correlation_id: currentReviewAnchor.correlation_id ?? previousReviewAnchor.correlation_id,
      actor_session: currentReviewAnchor.actor_session ?? previousReviewAnchor.actor_session,
      target_session: currentReviewAnchor.target_session ?? previousReviewAnchor.target_session,
      round: currentReviewAnchor.round ?? previousReviewAnchor.round,
      committed_handoff_base_sha: currentReviewAnchor.committed_handoff_base_sha ?? previousReviewAnchor.committed_handoff_base_sha,
      committed_handoff_head_sha: currentReviewAnchor.committed_handoff_head_sha ?? previousReviewAnchor.committed_handoff_head_sha,
      committed_handoff_range_source: currentReviewAnchor.committed_handoff_range_source ?? previousReviewAnchor.committed_handoff_range_source,
    },
  };
}

function projectCheckpointSnapshot(authority = {}) {
  return {
    packet_status: normalizeNullableString(authority.packet_status),
    task_board_status: normalizeNullableString(authority.task_board_status),
    milestone: normalizeNullableString(authority.milestone),
    phase: normalizeNullableString(authority.phase),
    runtime_status: normalizeNullableString(authority.runtime_status),
    next_expected_actor: normalizeNullableString(authority.next_expected_actor),
    next_expected_session: normalizeNullableString(authority.next_expected_session),
    waiting_on: normalizeNullableString(authority.waiting_on),
    waiting_on_session: normalizeNullableString(authority.waiting_on_session),
    route_anchor_state: normalizeNullableString(authority.route_anchor?.state),
    route_anchor_kind: normalizeNullableString(authority.route_anchor?.kind),
    route_anchor_correlation_id: normalizeNullableString(authority.route_anchor?.correlation_id),
    route_anchor_target_role: normalizeNullableString(authority.route_anchor?.target_role),
    route_anchor_target_session: normalizeNullableString(authority.route_anchor?.target_session),
    review_anchor_kind: normalizeNullableString(authority.review_anchor?.receipt_kind),
    review_anchor_correlation_id: normalizeNullableString(authority.review_anchor?.correlation_id),
    committed_handoff_base_sha: normalizeNullableString(authority.review_anchor?.committed_handoff_base_sha),
    committed_handoff_head_sha: normalizeNullableString(authority.review_anchor?.committed_handoff_head_sha),
  };
}

function applyCheckpointSnapshotToAuthority(checkpoint = {}) {
  return {
    packet_status: normalizeNullableString(checkpoint.packet_status),
    task_board_status: normalizeNullableString(checkpoint.task_board_status),
    milestone: normalizeNullableString(checkpoint.milestone),
    phase: normalizeNullableString(checkpoint.phase),
    runtime_status: normalizeNullableString(checkpoint.runtime_status),
    next_expected_actor: normalizeNullableString(checkpoint.next_expected_actor),
    next_expected_session: normalizeNullableString(checkpoint.next_expected_session),
    waiting_on: normalizeNullableString(checkpoint.waiting_on),
    waiting_on_session: normalizeNullableString(checkpoint.waiting_on_session),
    route_anchor: {
      state: normalizeNullableString(checkpoint.route_anchor_state),
      kind: normalizeNullableString(checkpoint.route_anchor_kind),
      correlation_id: normalizeNullableString(checkpoint.route_anchor_correlation_id),
      target_role: normalizeNullableString(checkpoint.route_anchor_target_role),
      target_session: normalizeNullableString(checkpoint.route_anchor_target_session),
    },
    review_anchor: {
      receipt_kind: normalizeNullableString(checkpoint.review_anchor_kind),
      correlation_id: normalizeNullableString(checkpoint.review_anchor_correlation_id),
      actor_session: null,
      target_session: null,
      round: null,
      committed_handoff_base_sha: normalizeNullableString(checkpoint.committed_handoff_base_sha),
      committed_handoff_head_sha: normalizeNullableString(checkpoint.committed_handoff_head_sha),
      committed_handoff_range_source: null,
    },
  };
}

export function projectExecutionAuthorityOntoRuntime(runtimeStatus = {}, authority = {}) {
  const nextRuntime = { ...(runtimeStatus || {}) };
  const routeAnchor = normalizeRouteAnchor(authority.route_anchor);
  const reviewAnchor = normalizeReviewAnchor(authority.review_anchor);

  if (authority.packet_status !== undefined) nextRuntime.current_packet_status = authority.packet_status;
  if (authority.task_board_status !== undefined) nextRuntime.current_task_board_status = authority.task_board_status;
  if (authority.milestone !== undefined) nextRuntime.current_milestone = authority.milestone;
  if (authority.phase !== undefined) nextRuntime.current_phase = authority.phase;
  if (authority.runtime_status !== undefined) nextRuntime.runtime_status = authority.runtime_status;
  if (authority.next_expected_actor !== undefined) nextRuntime.next_expected_actor = authority.next_expected_actor;
  if (authority.next_expected_session !== undefined) nextRuntime.next_expected_session = authority.next_expected_session;
  if (authority.waiting_on !== undefined) nextRuntime.waiting_on = authority.waiting_on;
  if (authority.waiting_on_session !== undefined) nextRuntime.waiting_on_session = authority.waiting_on_session;
  if (authority.validator_trigger !== undefined) nextRuntime.validator_trigger = authority.validator_trigger;
  if (authority.validator_trigger_reason !== undefined) nextRuntime.validator_trigger_reason = authority.validator_trigger_reason;
  if (authority.attention_required !== undefined) nextRuntime.attention_required = normalizeBoolean(authority.attention_required);
  if (authority.ready_for_validation !== undefined) nextRuntime.ready_for_validation = normalizeBoolean(authority.ready_for_validation);
  if (authority.ready_for_validation_reason !== undefined) nextRuntime.ready_for_validation_reason = authority.ready_for_validation_reason;
  if (authority.wp_validator_of_record !== undefined) nextRuntime.wp_validator_of_record = authority.wp_validator_of_record;
  if (authority.integration_validator_of_record !== undefined) nextRuntime.integration_validator_of_record = authority.integration_validator_of_record;
  if (authority.main_containment_status !== undefined) nextRuntime.main_containment_status = authority.main_containment_status;
  if (authority.merged_main_commit !== undefined) nextRuntime.merged_main_commit = authority.merged_main_commit;
  if (authority.main_containment_verified_at_utc !== undefined) nextRuntime.main_containment_verified_at_utc = authority.main_containment_verified_at_utc;
  if (authority.current_main_compatibility_status !== undefined) {
    nextRuntime.current_main_compatibility_status = authority.current_main_compatibility_status;
  }
  if (authority.current_main_compatibility_baseline_sha !== undefined) {
    nextRuntime.current_main_compatibility_baseline_sha = authority.current_main_compatibility_baseline_sha;
  }
  if (authority.current_main_compatibility_verified_at_utc !== undefined) {
    nextRuntime.current_main_compatibility_verified_at_utc = authority.current_main_compatibility_verified_at_utc;
  }
  if (authority.packet_widening_decision !== undefined) nextRuntime.packet_widening_decision = authority.packet_widening_decision;
  if (authority.packet_widening_evidence !== undefined) nextRuntime.packet_widening_evidence = authority.packet_widening_evidence;

  nextRuntime.route_anchor_state = routeAnchor.state;
  nextRuntime.route_anchor_kind = routeAnchor.kind;
  nextRuntime.route_anchor_correlation_id = routeAnchor.correlation_id;
  nextRuntime.route_anchor_target_role = routeAnchor.target_role;
  nextRuntime.route_anchor_target_session = routeAnchor.target_session;
  nextRuntime.authoritative_review_receipt_kind = reviewAnchor.receipt_kind;
  nextRuntime.authoritative_review_correlation_id = reviewAnchor.correlation_id;
  nextRuntime.authoritative_review_actor_session = reviewAnchor.actor_session;
  nextRuntime.authoritative_review_target_session = reviewAnchor.target_session;
  nextRuntime.authoritative_review_round = reviewAnchor.round;
  nextRuntime.committed_handoff_base_sha = reviewAnchor.committed_handoff_base_sha;
  nextRuntime.committed_handoff_head_sha = reviewAnchor.committed_handoff_head_sha;
  nextRuntime.committed_handoff_range_source = reviewAnchor.committed_handoff_range_source;

  return nextRuntime;
}

export function materializeRuntimeAuthorityView(runtimeStatus = {}) {
  if (!isPlainObject(runtimeStatus)) return {};
  const authority = readRuntimeExecutionAuthority(runtimeStatus);
  const nextRuntime = projectExecutionAuthorityOntoRuntime({ ...(runtimeStatus || {}) }, authority);
  if (isPlainObject(runtimeStatus.execution_state)) {
    nextRuntime.execution_state = {
      ...runtimeStatus.execution_state,
      authority,
    };
  }
  return nextRuntime;
}

export function readExecutionProjectionView(runtimeStatus = {}) {
  const runtime = materializeRuntimeAuthorityView(runtimeStatus);
  const hasCanonicalAuthority = isPlainObject(runtimeStatus?.execution_state?.authority);
  const packetStatus = normalizeNullableString(runtime.current_packet_status);
  const taskBoardStatus = normalizeNullableString(runtime.current_task_board_status);
  const runtimeStatusValue = normalizeNullableString(runtime.runtime_status ?? runtime.status);
  const phase = normalizeNullableString(runtime.current_phase);
  const milestone = normalizeNullableString(runtime.current_milestone);
  const nextExpectedActor = normalizeNullableString(runtime.next_expected_actor);
  const nextExpectedSession = normalizeNullableString(runtime.next_expected_session);
  const waitingOn = normalizeNullableString(runtime.waiting_on);
  const waitingOnSession = normalizeNullableString(runtime.waiting_on_session);
  const lastEvent = normalizeNullableString(runtime.last_event);
  const lastEventAt = normalizeNullableString(runtime.last_event_at);
  const openReviewItemsCount = Array.isArray(runtime.open_review_items) ? runtime.open_review_items.length : 0;
  const terminalPacketStatus = Boolean(packetStatus && isTerminalPacketStatus(packetStatus));
  const terminalTaskBoardStatus = Boolean(taskBoardStatus && isTerminalTaskBoardStatus(taskBoardStatus));

  return {
    runtime,
    has_canonical_authority: hasCanonicalAuthority,
    packet_status: packetStatus,
    task_board_status: taskBoardStatus,
    runtime_status: runtimeStatusValue,
    phase,
    milestone,
    next_expected_actor: nextExpectedActor,
    next_expected_session: nextExpectedSession,
    waiting_on: waitingOn,
    waiting_on_session: waitingOnSession,
    last_event: lastEvent,
    last_event_at: lastEventAt,
    open_review_items_count: openReviewItemsCount,
    terminal_packet_status: terminalPacketStatus,
    terminal_task_board_status: terminalTaskBoardStatus,
    terminal: terminalPacketStatus || terminalTaskBoardStatus,
  };
}

export function readExecutionPublicationView({
  runtimeStatus = {},
  packetStatus = null,
  taskBoardStatus = null,
} = {}) {
  const projection = readExecutionProjectionView(runtimeStatus);
  const artifactPacketStatus = normalizeNullableString(packetStatus);
  const artifactTaskBoardStatus = normalizeNullableString(taskBoardStatus);
  const canonicalPacketStatus = projection.packet_status;
  const canonicalTaskBoardStatus = projection.task_board_status
    || taskBoardStatusForPacketStatus(canonicalPacketStatus || "");

  const effectivePacketStatus = projection.has_canonical_authority
    ? (canonicalPacketStatus ?? artifactPacketStatus)
    : (artifactPacketStatus ?? canonicalPacketStatus);
  const effectiveTaskBoardStatus = projection.has_canonical_authority
    ? (canonicalTaskBoardStatus ?? artifactTaskBoardStatus)
    : (
      artifactTaskBoardStatus
      ?? canonicalTaskBoardStatus
      ?? taskBoardStatusForPacketStatus(artifactPacketStatus || "")
    );
  const terminalPacketStatus = Boolean(effectivePacketStatus && isTerminalPacketStatus(effectivePacketStatus));
  const terminalTaskBoardStatus = Boolean(effectiveTaskBoardStatus && isTerminalTaskBoardStatus(effectiveTaskBoardStatus));

  return {
    ...projection,
    packet_status: effectivePacketStatus,
    task_board_status: effectiveTaskBoardStatus,
    canonical_packet_status: canonicalPacketStatus,
    canonical_task_board_status: canonicalTaskBoardStatus,
    packet_projection_drift: Boolean(
      projection.has_canonical_authority
      && artifactPacketStatus
      && canonicalPacketStatus
      && artifactPacketStatus !== canonicalPacketStatus
    ),
    task_board_projection_drift: Boolean(
      projection.has_canonical_authority
      && artifactTaskBoardStatus
      && canonicalTaskBoardStatus
      && artifactTaskBoardStatus !== canonicalTaskBoardStatus
    ),
    terminal_packet_status: terminalPacketStatus,
    terminal_task_board_status: terminalTaskBoardStatus,
    terminal: terminalPacketStatus || terminalTaskBoardStatus,
  };
}

export function parseExecutionCloseoutMode(rawMode = "") {
  const normalized = String(rawMode || "").trim().toUpperCase();
  const canonicalMode = EXECUTION_CLOSEOUT_MODE_ALIASES.get(normalized);
  return cloneExecutionCloseoutModeSpec(
    canonicalMode ? EXECUTION_CLOSEOUT_MODE_SPECS[canonicalMode] : null,
  );
}

export function inferValidationVerdictFromPublication({
  packetStatus = "",
  taskBoardStatus = "",
  fallbackVerdict = "",
} = {}) {
  const normalizedPacketStatus = String(packetStatus || "").trim().toUpperCase();
  const normalizedTaskBoardStatus = String(taskBoardStatus || "").trim().toUpperCase();
  if (normalizedPacketStatus === "VALIDATED (PASS)" || normalizedTaskBoardStatus === "DONE_VALIDATED") return "PASS";
  if (normalizedPacketStatus === "VALIDATED (FAIL)" || normalizedTaskBoardStatus === "DONE_FAIL") return "FAIL";
  if (normalizedPacketStatus === "VALIDATED (OUTDATED_ONLY)" || normalizedTaskBoardStatus === "DONE_OUTDATED_ONLY") {
    return "OUTDATED_ONLY";
  }
  if (normalizedPacketStatus === "VALIDATED (ABANDONED)" || normalizedTaskBoardStatus === "DONE_ABANDONED") {
    return "ABANDONED";
  }
  return String(fallbackVerdict || "").trim().toUpperCase();
}

export function inferExecutionCloseoutMode({
  packetStatus = "",
  taskBoardStatus = "",
  mainContainmentStatus = "",
  fallbackVerdict = "",
} = {}) {
  const normalizedPacketStatus = String(packetStatus || "").trim().toUpperCase();
  const normalizedTaskBoardStatus = String(taskBoardStatus || "").trim().toUpperCase();
  const normalizedMainContainmentStatus = String(mainContainmentStatus || "").trim().toUpperCase();

  if (normalizedPacketStatus === "VALIDATED (PASS)" || normalizedTaskBoardStatus === "DONE_VALIDATED") {
    return parseExecutionCloseoutMode("CONTAINED_IN_MAIN");
  }
  if (normalizedMainContainmentStatus === "MERGE_PENDING" || normalizedTaskBoardStatus === "DONE_MERGE_PENDING" || normalizedPacketStatus === "DONE") {
    return parseExecutionCloseoutMode("MERGE_PENDING");
  }
  const verdict = inferValidationVerdictFromPublication({
    packetStatus,
    taskBoardStatus,
    fallbackVerdict,
  });
  if (verdict === "FAIL") return parseExecutionCloseoutMode("FAIL");
  if (verdict === "OUTDATED_ONLY") return parseExecutionCloseoutMode("OUTDATED_ONLY");
  if (verdict === "ABANDONED") return parseExecutionCloseoutMode("ABANDONED");
  return null;
}

export function syncRuntimeExecutionState(runtimeStatus = {}, {
  eventName = "runtime_sync",
  eventAt = new Date().toISOString(),
  checkpointKind = null,
  forceCheckpoint = false,
} = {}) {
  const nextRuntime = { ...(runtimeStatus || {}) };
  const previousExecutionState = isPlainObject(runtimeStatus.execution_state) ? runtimeStatus.execution_state : {};
  const previousAuthority = isPlainObject(previousExecutionState.authority) ? previousExecutionState.authority : {};
  const authority = mergeExecutionAuthority(previousAuthority, buildExecutionAuthorityFromRuntime(runtimeStatus));
  const previousLineage = isPlainObject(previousExecutionState.checkpoint_lineage) ? previousExecutionState.checkpoint_lineage : {};
  const previousCheckpoints = cloneCheckpointList(previousLineage.checkpoints);
  const checkpointSnapshot = projectCheckpointSnapshot(authority);
  const fingerprint = JSON.stringify(checkpointSnapshot);
  const latestFingerprint = normalizeNullableString(previousLineage.latest_checkpoint_fingerprint);
  const shouldAppendCheckpoint = forceCheckpoint || previousCheckpoints.length === 0 || fingerprint !== latestFingerprint;

  let checkpoints = previousCheckpoints;
  let latestCheckpointId = normalizeNullableString(previousLineage.latest_checkpoint_id);
  let latestCheckpointAtUtc = normalizeNullableString(previousLineage.latest_checkpoint_at_utc);
  let latestCheckpointKind = normalizeNullableString(previousLineage.latest_checkpoint_kind);
  let latestRestorePointId = normalizeNullableString(previousLineage.latest_restore_point_id);

  if (shouldAppendCheckpoint) {
    const nextIndex = previousCheckpoints.length + 1;
    const nextCheckpointId = buildCheckpointId(eventName, eventAt, nextIndex);
    const checkpointEntry = {
      checkpoint_id: nextCheckpointId,
      parent_checkpoint_id: normalizeNullableString(previousLineage.latest_checkpoint_id),
      recorded_at_utc: eventAt,
      event_name: String(eventName || "runtime_sync"),
      checkpoint_kind: normalizeNullableString(checkpointKind) || checkpointKindFromEventName(eventName),
      restore_supported: true,
      restore_hint: "Replay this checkpoint through the execution-state restore helper.",
      ...checkpointSnapshot,
    };
    checkpoints = [...previousCheckpoints, checkpointEntry].slice(-CHECKPOINT_HISTORY_LIMIT);
    latestCheckpointId = checkpointEntry.checkpoint_id;
    latestCheckpointAtUtc = checkpointEntry.recorded_at_utc;
    latestCheckpointKind = checkpointEntry.checkpoint_kind;
    latestRestorePointId = checkpointEntry.checkpoint_id;
  }

  nextRuntime.execution_state = {
    schema_version: EXECUTION_STATE_SCHEMA_VERSION,
    authority,
    checkpoint_lineage: {
      schema_version: EXECUTION_STATE_LINEAGE_SCHEMA_VERSION,
      latest_checkpoint_id: latestCheckpointId,
      latest_checkpoint_at_utc: latestCheckpointAtUtc,
      latest_checkpoint_kind: latestCheckpointKind,
      latest_restore_point_id: latestRestorePointId,
      latest_checkpoint_fingerprint: shouldAppendCheckpoint ? fingerprint : latestFingerprint,
      checkpoint_count: checkpoints.length,
      checkpoints,
    },
  };

  return projectExecutionAuthorityOntoRuntime(nextRuntime, authority);
}

export function listExecutionStateCheckpoints(runtimeStatus = {}) {
  const checkpoints = runtimeStatus?.execution_state?.checkpoint_lineage?.checkpoints;
  return cloneCheckpointList(checkpoints);
}

export function restoreRuntimeExecutionCheckpoint(runtimeStatus = {}, checkpointId, {
  eventName = "runtime_restore",
  eventAt = new Date().toISOString(),
} = {}) {
  const checkpoints = listExecutionStateCheckpoints(runtimeStatus);
  const target = checkpoints.find((entry) => String(entry?.checkpoint_id || "").trim() === String(checkpointId || "").trim());
  if (!target) {
    throw new Error(`Unknown execution-state checkpoint: ${checkpointId}`);
  }
  const authorityPatch = applyCheckpointSnapshotToAuthority(target);
  const restoredRuntime = projectExecutionAuthorityOntoRuntime({ ...(runtimeStatus || {}) }, authorityPatch);
  return syncRuntimeExecutionState(restoredRuntime, {
    eventName,
    eventAt,
    checkpointKind: "RESTORE",
    forceCheckpoint: true,
  });
}
