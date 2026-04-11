const TERMINAL_PACKET_STATUS_VALUES = new Set([
  "Done",
  "Validated (PASS)",
  "Validated (FAIL)",
  "Validated (OUTDATED_ONLY)",
  "Validated (ABANDONED)",
]);

export const TASK_BOARD_STATUS_VALUES = Object.freeze([
  "READY_FOR_DEV",
  "STUB",
  "IN_PROGRESS",
  "DONE_VALIDATED",
  "DONE_MERGE_PENDING",
  "DONE_FAIL",
  "DONE_OUTDATED_ONLY",
  "DONE_ABANDONED",
  "BLOCKED",
  "SUPERSEDED",
]);

export const TERMINAL_TASK_BOARD_STATUS_VALUES = Object.freeze([
  "VALIDATED",
  "FAIL",
  "OUTDATED_ONLY",
  "ABANDONED",
  "SUPERSEDED",
  "FAILED_HISTORICAL_SMOKETEST_BASELINE",
]);

export const ACTIVE_ORCHESTRATOR_TASK_BOARD_STATUS_VALUES = Object.freeze([
  "READY_FOR_DEV",
  "IN_PROGRESS",
  "BLOCKED",
  "MERGE_PENDING",
]);

export const RUNTIME_MILESTONE_VALUES = Object.freeze([
  "BOOTSTRAP",
  "SKELETON",
  "MICROTASK",
  "HANDOFF",
  "VERDICT",
  "CONTAINMENT",
  "WORKFLOW_REPAIR",
]);

const PRE_CLAIM_COMMUNICATION_STATE_VALUES = new Set([
  "COMM_MISSING_KICKOFF",
  "COMM_WAITING_FOR_INTENT",
  "COMM_WAITING_FOR_INTENT_CHECKPOINT",
]);

const COMMUNICATION_STATE_MILESTONE_MAP = Object.freeze({
  COMM_MISSING_KICKOFF: "BOOTSTRAP",
  COMM_WAITING_FOR_INTENT: "BOOTSTRAP",
  COMM_WAITING_FOR_INTENT_CHECKPOINT: "SKELETON",
  COMM_WAITING_FOR_HANDOFF: "MICROTASK",
  COMM_DEFERRED_REPAIR_QUEUE: "MICROTASK",
  COMM_REPAIR_REQUIRED: "MICROTASK",
  COMM_WAITING_FOR_REVIEW: "HANDOFF",
  COMM_WAITING_FOR_FINAL_REVIEW: "VERDICT",
  COMM_BLOCKED_OPEN_ITEMS: "VERDICT",
  COMM_OK: "VERDICT",
  COMM_MISCONFIGURED: "WORKFLOW_REPAIR",
  COMM_WORKFLOW_INVALID: "WORKFLOW_REPAIR",
});

function normalize(value) {
  return String(value || "").trim();
}

function normalizeUpper(value) {
  return normalize(value).toUpperCase();
}

function escapeRegex(value) {
  return String(value || "").replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

export function parsePacketStatus(packetText) {
  return (
    (String(packetText || "").match(/^\s*-\s*\*\*Status:\*\*\s*(.+)\s*$/mi) || [])[1]
    || (String(packetText || "").match(/^\s*\*\*Status:\*\*\s*(.+)\s*$/mi) || [])[1]
    || ""
  ).trim() || "Ready for Dev";
}

export function isTerminalPacketStatus(status) {
  return TERMINAL_PACKET_STATUS_VALUES.has(normalize(status));
}

export function isClosedPacketStatus(status) {
  return isTerminalPacketStatus(status);
}

export function isTerminalTaskBoardStatus(status) {
  return TERMINAL_TASK_BOARD_STATUS_VALUES.includes(normalizeUpper(status));
}

export function isActiveOrchestratorTaskBoardStatus(status) {
  return ACTIVE_ORCHESTRATOR_TASK_BOARD_STATUS_VALUES.includes(normalizeUpper(status));
}

export function parseTaskBoardStatus(taskBoardText, wpId) {
  const normalizedWpId = normalize(wpId);
  if (!normalizedWpId) return "";
  const match = String(taskBoardText || "").match(
    new RegExp(`- \\*\\*\\[${escapeRegex(normalizedWpId)}\\]\\*\\* - \\[([^\\]]+)\\]`, "i"),
  );
  return match ? normalizeUpper(match[1]) : "";
}

export function taskBoardStatusForPacketStatus(packetStatus) {
  switch (normalize(packetStatus)) {
    case "Ready for Dev":
      return "READY_FOR_DEV";
    case "In Progress":
      return "IN_PROGRESS";
    case "Blocked":
      return "BLOCKED";
    case "Done":
      return "DONE_MERGE_PENDING";
    case "Validated (PASS)":
      return "DONE_VALIDATED";
    case "Validated (FAIL)":
      return "DONE_FAIL";
    case "Validated (OUTDATED_ONLY)":
      return "DONE_OUTDATED_ONLY";
    case "Validated (ABANDONED)":
      return "DONE_ABANDONED";
    default:
      return null;
  }
}

export function expectedPacketStatusForTaskBoardStatus(taskBoardStatus) {
  switch (normalizeUpper(taskBoardStatus)) {
    case "READY_FOR_DEV":
      return "Ready for Dev";
    case "IN_PROGRESS":
      return "In Progress";
    case "BLOCKED":
      return "Blocked";
    case "DONE_MERGE_PENDING":
      return "Done";
    case "DONE_VALIDATED":
      return "Validated (PASS)";
    case "DONE_FAIL":
      return "Validated (FAIL)";
    case "DONE_OUTDATED_ONLY":
      return "Validated (OUTDATED_ONLY)";
    case "DONE_ABANDONED":
      return "Validated (ABANDONED)";
    default:
      return null;
  }
}

export function packetStatusForCommunicationState(evaluationState, currentPacketStatus) {
  if (isTerminalPacketStatus(currentPacketStatus)) return null;
  const normalizedState = normalizeUpper(evaluationState);
  switch (normalizedState) {
    case "COMM_MISCONFIGURED":
    case "COMM_WORKFLOW_INVALID":
      return "Blocked";
    case "COMM_NA":
      return null;
    default:
      break;
  }

  if (
    normalize(currentPacketStatus) === "Ready for Dev"
    && PRE_CLAIM_COMMUNICATION_STATE_VALUES.has(normalizedState)
  ) {
    return null;
  }

  return "In Progress";
}

export function derivePacketMilestone({
  packetStatus = "",
  communicationState = "",
  currentMilestone = "",
} = {}) {
  const normalizedPacketStatus = normalize(packetStatus);
  const normalizedState = normalizeUpper(communicationState);
  if (normalizedPacketStatus === "Done" || isTerminalPacketStatus(normalizedPacketStatus)) {
    return "CONTAINMENT";
  }
  if (COMMUNICATION_STATE_MILESTONE_MAP[normalizedState]) {
    return COMMUNICATION_STATE_MILESTONE_MAP[normalizedState];
  }
  if (normalizedPacketStatus === "Ready for Dev") return "BOOTSTRAP";
  if (normalizedPacketStatus === "Blocked") return "WORKFLOW_REPAIR";

  const normalizedCurrentMilestone = normalizeUpper(currentMilestone);
  if (RUNTIME_MILESTONE_VALUES.includes(normalizedCurrentMilestone)) {
    return normalizedCurrentMilestone;
  }
  return "MICROTASK";
}

export function runtimePhaseForMilestone(milestone, fallback = "BOOTSTRAP") {
  switch (normalizeUpper(milestone)) {
    case "BOOTSTRAP":
    case "SKELETON":
      return "BOOTSTRAP";
    case "MICROTASK":
      return "IMPLEMENTATION";
    case "HANDOFF":
    case "VERDICT":
      return "VALIDATION";
    case "CONTAINMENT":
      return "STATUS_SYNC";
    case "WORKFLOW_REPAIR":
      return "WORKFLOW_REPAIR";
    default:
      return fallback;
  }
}
