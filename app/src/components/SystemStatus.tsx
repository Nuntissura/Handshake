import { useCallback, useEffect, useState } from "react";

type Status = "loading" | "ok" | "error";

const HEALTH_URL = "http://127.0.0.1:37501/health";

export function SystemStatus() {
  const [status, setStatus] = useState<Status>("loading");
  const [message, setMessage] = useState<string | null>(null);

  const checkHealth = useCallback(async () => {
    setStatus("loading");
    setMessage(null);

    try {
      const response = await fetch(HEALTH_URL, { method: "GET" });
      if (!response.ok) {
        throw new Error(`HTTP ${response.status}`);
      }

      const data: { status?: string } = await response.json();
      if (data?.status === "ok") {
        setStatus("ok");
        setMessage(null);
      } else {
        setStatus("error");
        setMessage("Unexpected response");
      }
    } catch (error) {
      setStatus("error");
      const reason = error instanceof Error ? error.message : "Unknown error";
      setMessage(reason);
    }
  }, []);

  useEffect(() => {
    void checkHealth();
    const id = window.setInterval(() => void checkHealth(), 8000);
    return () => window.clearInterval(id);
  }, [checkHealth]);

  const indicatorColor =
    status === "ok" ? "status-pill success" : status === "loading" ? "status-pill neutral" : "status-pill error";

  return (
    <div className="status-card">
      <div className="status-header">
        <span className={indicatorColor} aria-live="polite">
          {status === "ok" ? "Coordinator: OK" : status === "loading" ? "Coordinator: Checking..." : "Coordinator: ERROR"}
        </span>
        <button className="status-refresh" type="button" onClick={() => void checkHealth()}>
          Refresh
        </button>
      </div>
      {message ? <p className="status-message">Last error: {message}</p> : <p className="status-message">Healthy</p>}
    </div>
  );
}
