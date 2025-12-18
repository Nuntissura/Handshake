import { useCallback, useEffect, useState } from "react";
import { getHealth } from "../lib/api";

type Status = "loading" | "ok" | "error";

export function SystemStatus() {
  const [status, setStatus] = useState<Status>("loading");
  const [message, setMessage] = useState<string | null>(null);
  const [dbStatus, setDbStatus] = useState<string | null>(null);

  const checkHealth = useCallback(async () => {
    setStatus("loading");
    setMessage(null);
    setDbStatus(null);

    try {
      const data = await getHealth();
      if (data?.status === "ok") {
        setStatus("ok");
        setMessage(null);
        setDbStatus(data.db_status ?? "ok");
      } else {
        setStatus("error");
        setMessage("Unexpected response");
        setDbStatus(data?.db_status ?? "error");
      }
    } catch (error) {
      setStatus("error");
      const reason = error instanceof Error ? error.message : "Unknown error";
      setMessage(reason);
      setDbStatus("error");
    }
  }, []);

  useEffect(() => {
    const initialId = window.setTimeout(() => void checkHealth(), 0);
    const id = window.setInterval(() => void checkHealth(), 8000);
    return () => {
      window.clearTimeout(initialId);
      window.clearInterval(id);
    };
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
      <p className="status-message">DB: {dbStatus ?? "unknown"}</p>
      {message ? <p className="status-message">Last error: {message}</p> : <p className="status-message">Healthy</p>}
    </div>
  );
}
