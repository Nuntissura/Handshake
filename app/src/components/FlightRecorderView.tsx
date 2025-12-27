import { FormEvent, useEffect, useState } from "react";
import { FlightEvent, getEvents, SecurityViolationPayload } from "../lib/api";

type Filters = {
  jobId: string;
  traceId: string;
  from: string;
  to: string;
};

const defaultFilters: Filters = {
  jobId: "",
  traceId: "",
  from: "",
  to: "",
};

export function FlightRecorderView() {
  const [events, setEvents] = useState<FlightEvent[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [filters, setFilters] = useState<Filters>(defaultFilters);

  const fetchEvents = async (useFilters: Filters = filters) => {
    try {
      const data = await getEvents({
        jobId: useFilters.jobId || undefined,
        traceId: useFilters.traceId || undefined,
        from: useFilters.from || undefined,
        to: useFilters.to || undefined,
      });
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
    const interval = setInterval(() => fetchEvents(), 5000); // Poll every 5 seconds
    return () => clearInterval(interval);
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const onSubmit = (e: FormEvent) => {
    e.preventDefault();
    setLoading(true);
    fetchEvents(filters);
  };

  const onReset = () => {
    setFilters(defaultFilters);
    setLoading(true);
    fetchEvents(defaultFilters);
  };

  const toggleTraceFilter = (traceId: string) => {
    if (filters.traceId === traceId) {
      onReset();
    } else {
      const newFilters = { ...filters, traceId };
      setFilters(newFilters);
      setLoading(true);
      fetchEvents(newFilters);
    }
  };

  if (loading && events.length === 0) {
    return (
      <div className="content-card">
        <p>Loading events...</p>
      </div>
    );
  }

  if (error && events.length === 0) {
    return (
      <div className="content-card error">
        <p>Error: {error}</p>
      </div>
    );
  }

  return (
    <div className="content-card flight-recorder">
      <h2>Flight Recorder</h2>
      <p className="muted">Chronological log of AI actions and job lifecycle events.</p>

      <form className="flight-recorder__filters" onSubmit={onSubmit}>
        <label>
          Job ID
          <input
            value={filters.jobId}
            onChange={(e) => setFilters({ ...filters, jobId: e.target.value })}
            placeholder="job-uuid"
          />
        </label>
        <label>
          Trace ID
          <input
            value={filters.traceId}
            onChange={(e) => setFilters({ ...filters, traceId: e.target.value })}
            placeholder="trace uuid"
          />
        </label>
        <label>
          From
          <input
            type="datetime-local"
            value={filters.from}
            onChange={(e) => setFilters({ ...filters, from: e.target.value })}
          />
        </label>
        <label>
          To
          <input
            type="datetime-local"
            value={filters.to}
            onChange={(e) => setFilters({ ...filters, to: e.target.value })}
          />
        </label>
        <div className="flight-recorder__filter-actions">
          <button type="submit">Apply</button>
          <button type="button" onClick={onReset}>
            Clear
          </button>
        </div>
      </form>

      <div className="flight-recorder__table-container">
        <table className="flight-recorder__table">
          <thead>
            <tr>
              <th>Timestamp</th>
              <th>Event</th>
              <th>Actor</th>
              <th>Job ID</th>
              <th>Trace ID</th>
              <th>Payload</th>
            </tr>
          </thead>
          <tbody>
            {events.map((event) => {
              const isSecurityViolation = event.event_type === "security_violation";
              const rowClass = isSecurityViolation ? "flight-recorder__row--violation" : "";
              const payload = event.payload as Partial<SecurityViolationPayload>;

              return (
                <tr key={event.event_id} className={rowClass}>
                  <td className="nowrap">{new Date(event.timestamp).toLocaleString()}</td>
                  <td>
                    <span className={`event-tag event-tag--${event.event_type}`}>{event.event_type}</span>
                  </td>
                  <td className="muted">{event.actor_id ?? event.actor}</td>
                  <td className="muted">{event.job_id || "-"}</td>
                  <td 
                    className="muted clickable" 
                    title="Filter by this trace"
                    onClick={() => toggleTraceFilter(event.trace_id)}
                  >
                    {event.trace_id}
                  </td>
                  <td>
                    {isSecurityViolation && payload.trigger && (
                      <div className="violation-context">
                        <strong>Trigger:</strong> <code>{payload.trigger}</code>
                        {payload.offset !== undefined && (
                          <span> | <strong>Offset:</strong> {payload.offset}</span>
                        )}
                        {payload.context && (
                          <div className="violation-snippet">
                            <strong>Snippet:</strong> <code>...{payload.context}...</code>
                          </div>
                        )}
                      </div>
                    )}
                    <details>
                      <summary>View Data</summary>
                      <pre className="event-payload">{JSON.stringify(event.payload, null, 2)}</pre>
                    </details>
                  </td>
                </tr>
              );
            })}
          </tbody>
        </table>
      </div>
    </div>
  );
}
