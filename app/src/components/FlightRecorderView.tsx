import { useEffect, useState } from "react";
import { FlightEvent, getEvents } from "../lib/api";

export function FlightRecorderView() {
  const [events, setEvents] = useState<FlightEvent[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const fetchEvents = async () => {
    try {
      const data = await getEvents();
      setEvents(data);
      setError(null);
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to fetch events");
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    fetchEvents();
    const interval = setInterval(fetchEvents, 5000); // Poll every 5 seconds
    return () => clearInterval(interval);
  }, []);

  if (loading && events.length === 0) {
    return <div className="content-card"><p>Loading events...</p></div>;
  }

  if (error && events.length === 0) {
    return <div className="content-card error"><p>Error: {error}</p></div>;
  }

  return (
    <div className="content-card flight-recorder">
      <h2>Flight Recorder</h2>
      <p className="muted">Chronological log of AI actions and job lifecycle events.</p>
      
      <div className="flight-recorder__table-container">
        <table className="flight-recorder__table">
          <thead>
            <tr>
              <th>Timestamp</th>
              <th>Event Type</th>
              <th>Job ID</th>
              <th>Payload</th>
            </tr>
          </thead>
          <tbody>
            {events.map((event, i) => (
              <tr key={i}>
                <td className="nowrap">{new Date(event.timestamp).toLocaleString()}</td>
                <td><span className={`event-tag event-tag--${event.event_type}`}>{event.event_type}</span></td>
                <td className="muted">{event.job_id || "-"}</td>
                <td>
                  <details>
                    <summary>View Data</summary>
                    <pre className="event-payload">{JSON.stringify(event.payload, null, 2)}</pre>
                  </details>
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </div>
  );
}
