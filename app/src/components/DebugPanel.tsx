import { useCallback, useEffect, useState } from "react";
import { getHealth, getLogTail, HealthResponse, LogTailResponse, recordRuntimeChatEvent } from "../lib/api";
import { isTauri } from "@tauri-apps/api/core";
import type { ResponseBehaviorContract } from "../lib/ans001";
import { sha256Hex, stableStringify } from "../lib/crypto";
import { sessionChatAppend, sessionChatGetSessionId } from "../lib/sessionChat";
import { DebugEvent, subscribeDebugEvents } from "../state/debugEvents";

function sampleAns001Payload(): ResponseBehaviorContract {
  return {
    answer: {
      content: "Acknowledged. This is a sample frontend assistant message used to validate ANS-001 logging + UI inspection.",
      addresses_all_questions: true,
      clarifying_question: null,
    },
    intent: {
      understood_request: "Validate that ANS-001 payloads are persisted and inspectable.",
      scope: { included: ["ANS-001 logging", "timeline inspection"], excluded: ["backend model roles"] },
      assumptions: [],
    },
    mode_context: {
      mode: "Free",
      determinism: true,
      can_edit: false,
      layer: "L3",
    },
    operation_plan: null,
    proactive: { risks: [], conflicts: [], alternatives: [], findings: [] },
    next_steps: null,
  };
}

export function DebugPanel() {
  const tauriAvailable = isTauri();
  const [health, setHealth] = useState<HealthResponse | null>(null);
  const [healthError, setHealthError] = useState<string | null>(null);
  const [healthCheckedAt, setHealthCheckedAt] = useState<string | null>(null);
  const [logs, setLogs] = useState<string[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);
  const [events, setEvents] = useState<DebugEvent[]>([]);
  const [chatSessionId, setChatSessionId] = useState<string | null>(null);
  const [chatBusy, setChatBusy] = useState(false);
  const [chatError, setChatError] = useState<string | null>(null);

  const refreshChatSessionId = useCallback(async () => {
    if (!tauriAvailable) return;
    try {
      const sid = await sessionChatGetSessionId();
      setChatSessionId(sid);
      setChatError(null);
    } catch (err) {
      setChatSessionId(null);
      setChatError(err instanceof Error ? err.message : "Failed to read session_id");
    }
  }, [tauriAvailable]);

  const fetchLogs = async () => {
    setLoading(true);
    setError(null);
    try {
      const response: LogTailResponse = await getLogTail();
      setLogs(response.lines);
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to load logs");
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    const unsub = subscribeDebugEvents((evts) => setEvents(evts.slice(0, 5)));
    void fetchLogs();
    void fetchHealth();
    if (tauriAvailable) {
      void refreshChatSessionId();
    }
    return () => unsub();
  }, [refreshChatSessionId, tauriAvailable]);

  const appendSampleChatEntries = async () => {
    setChatBusy(true);
    setChatError(null);
    try {
      const sessionId = chatSessionId ?? (await sessionChatGetSessionId());
      setChatSessionId(sessionId);

      const userMessageId = crypto.randomUUID();
      const userContent = "User (sample): validate ANS-001 timeline + persistence.";
      await sessionChatAppend({ role: "user", content: userContent, message_id: userMessageId });

      const userBodySha256 = await sha256Hex(userContent);
      await recordRuntimeChatEvent({
        schema_version: "hsk.fr.runtime_chat@0.1",
        event_id: "FR-EVT-RUNTIME-CHAT-101",
        ts_utc: new Date().toISOString(),
        session_id: sessionId,
        type: "runtime_chat_message_appended",
        message_id: userMessageId,
        role: "user",
        body_sha256: userBodySha256,
      });

      const ans001 = sampleAns001Payload();
      const assistantMessageId = crypto.randomUUID();
      const assistantContent = ans001.answer.content;
      await sessionChatAppend({
        role: "assistant",
        model_role: "frontend",
        content: assistantContent,
        ans001,
        ans001_validation: { compliant: true, violation_clauses: [] },
        message_id: assistantMessageId,
      });

      const assistantBodySha256 = await sha256Hex(assistantContent);
      const ans001Sha256 = await sha256Hex(stableStringify(ans001));

      await recordRuntimeChatEvent({
        schema_version: "hsk.fr.runtime_chat@0.1",
        event_id: "FR-EVT-RUNTIME-CHAT-101",
        ts_utc: new Date().toISOString(),
        session_id: sessionId,
        type: "runtime_chat_message_appended",
        message_id: assistantMessageId,
        role: "assistant",
        model_role: "frontend",
        body_sha256: assistantBodySha256,
        ans001_sha256: ans001Sha256,
      });

      await recordRuntimeChatEvent({
        schema_version: "hsk.fr.runtime_chat@0.1",
        event_id: "FR-EVT-RUNTIME-CHAT-102",
        ts_utc: new Date().toISOString(),
        session_id: sessionId,
        type: "runtime_chat_ans001_validation",
        message_id: assistantMessageId,
        role: "assistant",
        model_role: "frontend",
        ans001_compliant: true,
        violation_clauses: [],
        body_sha256: assistantBodySha256,
        ans001_sha256: ans001Sha256,
      });
    } catch (err) {
      setChatError(err instanceof Error ? err.message : "Failed to append sample chat entries");
    } finally {
      setChatBusy(false);
    }
  };

  const emitSampleChatSessionClosed = async () => {
    setChatBusy(true);
    setChatError(null);
    try {
      const sessionId = chatSessionId ?? (await sessionChatGetSessionId());
      setChatSessionId(sessionId);
      await recordRuntimeChatEvent({
        schema_version: "hsk.fr.runtime_chat@0.1",
        event_id: "FR-EVT-RUNTIME-CHAT-103",
        ts_utc: new Date().toISOString(),
        session_id: sessionId,
        type: "runtime_chat_session_closed",
      });
    } catch (err) {
      setChatError(err instanceof Error ? err.message : "Failed to emit session closed event");
    } finally {
      setChatBusy(false);
    }
  };

  const fetchHealth = async () => {
    setHealthError(null);
    try {
      const res = await getHealth();
      setHealth(res);
      setHealthCheckedAt(new Date().toLocaleTimeString());
    } catch (err) {
      setHealthError(err instanceof Error ? err.message : "Failed to fetch health");
      setHealth(null);
    }
  };

  const healthStatus = health?.status ?? "unknown";
  const dbStatus = health?.db_status ?? "unknown";

  return (
    <div className="content-card debug-panel">
      <div className="debug-panel__header">
        <div>
          <h3>Debug / Status</h3>
          <p className="muted">Backend health + recent activity</p>
        </div>
        <button onClick={fetchLogs} disabled={loading} className="debug-panel__refresh">
          {loading ? "Refreshing..." : "Refresh"}
        </button>
      </div>
      <div className="debug-panel__section">
        <div className="debug-panel__row">
          <span className="muted">Health</span>
          <strong>{healthStatus}</strong>
        </div>
        <div className="debug-panel__row">
          <span className="muted">DB</span>
          <strong>{dbStatus}</strong>
        </div>
        <div className="debug-panel__row">
          <span className="muted">Checked</span>
          <strong>{healthCheckedAt ?? "never"}</strong>
        </div>
        {healthError && <p className="muted">Error: {healthError}</p>}
        <button type="button" onClick={fetchHealth} className="debug-panel__refresh">
          Refresh health
        </button>
      </div>

      <div className="debug-panel__section">
        <h4>Recent events</h4>
        <div className="debug-events-list">
          {events.length === 0 ? (
            <p className="muted">No recent events yet.</p>
          ) : (
            <ul className="list-inline" data-testid="events-list">
              {events.map((evt) => (
                <li key={evt.id} className="debug-panel__line">
                  {new Date(evt.ts).toLocaleTimeString()} ƒ?» {evt.type} {evt.targetId ? `id=${evt.targetId}` : ""} →
                  {` ${evt.result.toUpperCase()}`}
                </li>
              ))}
            </ul>
          )}
        </div>
      </div>

      {tauriAvailable && (
        <div className="debug-panel__section">
          <h4>ANS-001 session chat log (dev harness)</h4>
          <div className="debug-panel__row">
            <span className="muted">Session</span>
            <strong>{chatSessionId ?? "unknown"}</strong>
          </div>
          {chatError && <p className="muted">Error: {chatError}</p>}
          <div className="drawer-actions" style={{ justifyContent: "flex-start", marginTop: 10 }}>
            <button type="button" className="secondary" onClick={refreshChatSessionId} disabled={chatBusy}>
              Refresh session_id
            </button>
            <button type="button" className="secondary" onClick={appendSampleChatEntries} disabled={chatBusy}>
              {chatBusy ? "Working..." : "Append sample entries + emit FR events"}
            </button>
            <button type="button" className="secondary" onClick={emitSampleChatSessionClosed} disabled={chatBusy}>
              Emit session closed (FR-EVT-RUNTIME-CHAT-103)
            </button>
          </div>
          <p className="muted small" style={{ marginTop: 8 }}>
            Use the header button "ANS-001 Timeline" to inspect the session log and payloads.
          </p>
        </div>
      )}

      <div className="debug-panel__section">
        <h4>Backend logs (tail)</h4>
        {error && <p className="muted">Error: {error}</p>}
        <div className="debug-panel__logbox">
          {logs.length === 0 ? (
            <p className="muted">No logs yet.</p>
          ) : (
            logs.map((line, idx) => (
              <pre key={idx} className="debug-panel__line">
                {line}
              </pre>
            ))
          )}
        </div>
      </div>
    </div>
  );
}
