export const RUNNING_SESSION_STATES = new Set(["STARTING", "COMMAND_RUNNING", "RUNNING"]);

export function parseTimestampMs(value = "") {
  const text = String(value || "").trim();
  if (!text) return 0;
  const parsed = Date.parse(text);
  return Number.isFinite(parsed) ? parsed : 0;
}

export function newestTimestampIso(values = []) {
  let newest = "";
  let newestMs = 0;
  for (const value of values) {
    const ms = parseTimestampMs(value);
    if (ms > newestMs) {
      newestMs = ms;
      newest = String(value || "").trim();
    }
  }
  return newest;
}

export function secondsSince(value = "", now = new Date()) {
  const timestampMs = parseTimestampMs(value);
  if (!timestampMs) return null;
  return Math.max(0, Math.floor((now.getTime() - timestampMs) / 1000));
}

export function formatDuration(seconds) {
  if (seconds === null || seconds === undefined || seconds === "") return "<unknown>";
  if (!Number.isFinite(Number(seconds))) return "<unknown>";
  const total = Math.max(0, Number(seconds));
  if (total < 90) return `${Math.floor(total)}s`;
  const minutes = Math.floor(total / 60);
  if (minutes < 90) return `${minutes}m`;
  const hours = Math.floor(minutes / 60);
  const remMinutes = minutes % 60;
  return remMinutes > 0 ? `${hours}h${remMinutes}m` : `${hours}h`;
}

export function latestSessionActivityIso(session = {}) {
  return newestTimestampIso([
    session.updated_at,
    session.health_updated_at,
    session.session_thread_started_at,
    session.owned_terminal_recorded_at,
    session.owned_terminal_reclaimed_at,
    session.effective_governed_action?.updated_at,
    session.effective_governed_action?.completed_at,
    session.effective_governed_action?.requested_at,
    session.last_governed_action?.updated_at,
    session.last_governed_action?.completed_at,
    session.last_governed_action?.requested_at,
    session.next_queued_control_request?.updated_at,
    session.next_queued_control_request?.queued_at,
  ]);
}

export function sessionHealthLine(session = {}, rawSession = {}, now = new Date()) {
  const latestActivity = latestSessionActivityIso(session);
  const staleSeconds = secondsSince(latestActivity, now);
  const commandKind = session.effective_governed_action?.command_kind || session.last_command_kind || "NONE";
  const commandStatus = session.effective_governed_action?.status || session.last_command_status || "NONE";
  const outcome = session.effective_governed_action?.outcome_state || "<none>";
  return [
    `role=${session.role || "<unknown>"}`,
    `runtime=${session.runtime_state || "<unknown>"}`,
    `model=${rawSession.requested_model || "<none>"}`,
    `profile=${session.requested_profile_id || rawSession.requested_profile_id || "<none>"}`,
    `thread=${session.session_thread_id || "<none>"}`,
    `command=${commandKind}/${commandStatus}`,
    `outcome=${outcome}`,
    `queued=${session.pending_control_queue_count || 0}`,
    `stale=${formatDuration(staleSeconds)}`,
    `worktree=${session.local_worktree_dir || "<unknown>"}`,
  ].join(" | ");
}

export function nextSafeCommand({ wpId = "", sessions = [], brokerActiveRunCount = 0 } = {}) {
  const target = wpId ? ` ${wpId}` : "";
  if (brokerActiveRunCount > 0) return `just orchestrator-next${target}`;
  const hasRunning = sessions.some((session) => RUNNING_SESSION_STATES.has(String(session.runtime_state || "").trim().toUpperCase()));
  if (hasRunning) return `just orchestrator-next${target}`;
  const hasQueued = sessions.some((session) => Number(session.pending_control_queue_count || 0) > 0);
  if (hasQueued) return `just orchestrator-next${target}`;
  return `just orchestrator-next${target}`;
}
