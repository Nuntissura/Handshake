function normalizeToken(value) {
  return String(value || "").trim().toUpperCase();
}

export function steerActionForSession(session) {
  if (!session) return "START_SESSION";
  const threadId = String(session.session_thread_id || "").trim();
  if (!threadId) return "START_SESSION";
  const runtimeState = normalizeToken(session.runtime_state);
  if (runtimeState === "CLOSED") return "START_SESSION";
  return "SEND_PROMPT";
}
