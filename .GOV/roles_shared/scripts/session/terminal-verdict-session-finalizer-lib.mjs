const TERMINAL_CLOSE_ROLE_VALUES = new Set([
  "ACTIVATION_MANAGER",
  "CODER",
  "WP_VALIDATOR",
  "INTEGRATION_VALIDATOR",
  "MEMORY_MANAGER",
]);

const ACTIVE_RUNTIME_STATES = new Set([
  "STARTING",
  "COMMAND_RUNNING",
  "PLUGIN_REQUESTED",
  "TERMINAL_COMMAND_DISPATCHED",
]);

function normalizeRole(value) {
  return String(value || "").trim().toUpperCase();
}

function normalizeWpId(value) {
  return String(value || "").trim();
}

function pendingQueueCount(session = {}) {
  const explicitCount = Number(session?.pending_control_queue_count);
  if (Number.isInteger(explicitCount) && explicitCount >= 0) return explicitCount;
  if (Array.isArray(session?.pending_control_queue)) return session.pending_control_queue.length;
  if (session?.next_queued_control_request && typeof session.next_queued_control_request === "object") return 1;
  return 0;
}

function isTerminalVerdict(record = {}) {
  const status = normalizeRole(record?.status || record?.verdict || record?.packet_status || record?.task_board_status);
  return ["PASS", "FAIL", "OUTDATED_ONLY", "ABANDONED", "SUPERSEDED", "DONE_VALIDATED", "DONE_FAIL", "DONE_OUTDATED_ONLY", "VALIDATED"].includes(status);
}

export function finalizeTerminalSessions({
  wpId = "",
  terminalRecord = {},
  sessions = [],
  controlRequests = [],
  controlResults = [],
} = {}) {
  const normalizedWpId = normalizeWpId(wpId || terminalRecord?.wp_id);
  const terminal = isTerminalVerdict(terminalRecord);
  const wpSessions = (Array.isArray(sessions) ? sessions : [])
    .filter((session) => normalizeWpId(session?.wp_id) === normalizedWpId)
    .filter((session) => TERMINAL_CLOSE_ROLE_VALUES.has(normalizeRole(session?.role)));
  const resultIds = new Set(
    (Array.isArray(controlResults) ? controlResults : [])
      .filter((result) => normalizeWpId(result?.wp_id) === normalizedWpId)
      .map((result) => String(result?.command_id || "").trim())
      .filter(Boolean),
  );
  const requestIds = new Set(
    (Array.isArray(controlRequests) ? controlRequests : [])
      .filter((request) => normalizeWpId(request?.wp_id) === normalizedWpId)
      .map((request) => String(request?.command_id || "").trim())
      .filter(Boolean),
  );

  const staleReadySessions = [];
  const activeBlockers = [];
  const queuedBlockers = [];
  const terminalResidue = [];

  for (const session of wpSessions) {
    const runtimeState = normalizeRole(session?.runtime_state);
    const queueDepth = pendingQueueCount(session);
    const lastCommandId = String(session?.last_command_id || "").trim();
    const lastCommandStatus = normalizeRole(session?.last_command_status);
    if (queueDepth > 0) {
      queuedBlockers.push({ ...session, queue_depth: queueDepth });
      continue;
    }
    if (ACTIVE_RUNTIME_STATES.has(runtimeState) || lastCommandStatus === "RUNNING") {
      activeBlockers.push({ ...session });
      continue;
    }
    if (runtimeState === "READY") {
      staleReadySessions.push({ ...session });
      terminalResidue.push({ ...session, residue_kind: "STALE_READY_SESSION" });
      continue;
    }
    if (lastCommandId && requestIds.has(lastCommandId) && !resultIds.has(lastCommandId)) {
      activeBlockers.push({ ...session, residue_kind: "UNSETTLED_CONTROL_REQUEST" });
    }
  }

  const status = !terminal
    ? "NOT_TERMINAL"
    : activeBlockers.length > 0 || queuedBlockers.length > 0
      ? "BLOCKED"
      : terminalResidue.length > 0
        ? "FINALIZE_READY"
        : "CLEAN";

  return {
    wp_id: normalizedWpId,
    terminal,
    status,
    staleReadySessions,
    activeBlockers,
    queuedBlockers,
    terminalResidue,
    summary:
      status === "BLOCKED"
        ? `terminal session finalization is blocked by ${activeBlockers.length} active and ${queuedBlockers.length} queued governed session(s)`
        : status === "FINALIZE_READY"
          ? `${staleReadySessions.length} stale READY governed session(s) should be closed after terminal verdict`
          : status === "CLEAN"
            ? "no terminal governed session residue remains"
            : "terminal verdict has not been recorded yet",
  };
}
