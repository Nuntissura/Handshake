import { useEffect, useState } from "react";
import { getHealth, getLogTail, HealthResponse, LogTailResponse } from "../lib/api";
import { DebugEvent, subscribeDebugEvents } from "../state/debugEvents";

export function DebugPanel() {
  const [health, setHealth] = useState<HealthResponse | null>(null);
  const [healthError, setHealthError] = useState<string | null>(null);
  const [healthCheckedAt, setHealthCheckedAt] = useState<string | null>(null);
  const [logs, setLogs] = useState<string[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);
  const [events, setEvents] = useState<DebugEvent[]>([]);

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
    return () => unsub();
  }, []);

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
