import { useEffect, useState } from "react";
import { getLogTail, LogTailResponse } from "../lib/api";

export function DebugPanel() {
  const [logs, setLogs] = useState<string[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);

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
    void fetchLogs();
  }, []);

  return (
    <div className="content-card debug-panel">
      <div className="debug-panel__header">
        <div>
          <h3>Debug / Logs (dev)</h3>
          <p className="muted">Recent backend logs from handshake_core</p>
        </div>
        <button onClick={fetchLogs} disabled={loading} className="debug-panel__refresh">
          {loading ? "Refreshing..." : "Refresh"}
        </button>
      </div>
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
  );
}
