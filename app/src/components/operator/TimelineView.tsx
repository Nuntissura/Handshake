import React, { FormEvent, useEffect, useState } from "react";
import { FlightEvent, getEvents } from "../../lib/api";
import { EvidenceSelection } from "./EvidenceDrawer";
import { DebugBundleExport } from "./DebugBundleExport";

type Props = {
  onSelect: (selection: EvidenceSelection) => void;
};

type TimelineFilters = {
  job_id: string;
  wsid: string;
  actor: "" | "human" | "agent" | "system";
  event_type: "" | FlightEvent["event_type"];
  from: string;
  to: string;
};

const defaultFilters: TimelineFilters = {
  job_id: "",
  wsid: "",
  actor: "",
  event_type: "",
  from: "",
  to: "",
};

export const TimelineView: React.FC<Props> = ({ onSelect }) => {
  const [filters, setFilters] = useState<TimelineFilters>(defaultFilters);
  const [events, setEvents] = useState<FlightEvent[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [pinnedSlices, setPinnedSlices] = useState<TimelineFilters[]>([]);
  const [exportOpen, setExportOpen] = useState(false);

  const fetchEvents = async (override?: TimelineFilters) => {
    const active = override ?? filters;
    setLoading(true);
    try {
      const data = await getEvents({
        jobId: active.job_id || undefined,
        wsid: active.wsid || undefined,
        actor: active.actor || undefined,
        eventType: active.event_type || undefined,
        from: active.from || undefined,
        to: active.to || undefined,
      });
      setEvents(data);
      setError(null);
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to load events");
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    fetchEvents();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const onSubmit = (e: FormEvent) => {
    e.preventDefault();
    fetchEvents();
  };

  const pinSlice = () => {
    setPinnedSlices((prev) => [...prev, filters]);
  };

  return (
    <div className="content-card">
      <div className="card-header">
        <div>
          <h2>Timeline</h2>
          <p className="muted">
            Flight Recorder events with filters for job, workspace, actor, and event types. Pin slices for bundle export.
          </p>
        </div>
        <div className="card-actions">
          <button className="secondary" onClick={pinSlice}>
            Pin this slice
          </button>
          <button className="primary" type="button" onClick={() => setExportOpen(true)}>
            Export time range
          </button>
        </div>
      </div>

      <form className="filters-grid" onSubmit={onSubmit}>
        <label>
          Job ID
          <input
            value={filters.job_id}
            onChange={(e) => setFilters({ ...filters, job_id: e.target.value })}
            placeholder="job uuid"
          />
        </label>
        <label>
          Workspace
          <input
            value={filters.wsid}
            onChange={(e) => setFilters({ ...filters, wsid: e.target.value })}
            placeholder="wsid"
          />
        </label>
        <label>
          Actor
          <select
            value={filters.actor}
            onChange={(e) => setFilters({ ...filters, actor: e.target.value as TimelineFilters["actor"] })}
          >
            <option value="">Any</option>
            <option value="human">Human</option>
            <option value="agent">Agent</option>
            <option value="system">System</option>
          </select>
        </label>
        <label>
          Event Types
          <input
            value={filters.event_type}
            onChange={(e) => setFilters({ ...filters, event_type: e.target.value as TimelineFilters["event_type"] })}
            placeholder="diagnostic, capability_action, workflow_recovery"
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
        <div className="filter-actions">
          <button type="submit">Apply</button>
          <button
            type="button"
            className="secondary"
            onClick={() => {
              setFilters(defaultFilters);
              fetchEvents(defaultFilters);
            }}
          >
            Reset
          </button>
        </div>
      </form>

      {pinnedSlices.length > 0 && (
        <div className="pinned-slices">
          <h4>Pinned slices</h4>
          <ul>
            {pinnedSlices.map((slice, idx) => (
              <li key={idx} className="muted small">
                job_id={slice.job_id || "any"}, wsid={slice.wsid || "any"}, actor={slice.actor || "any"}, event={slice.event_type || "any"}
              </li>
            ))}
          </ul>
        </div>
      )}

      {loading && events.length === 0 ? (
        <p>Loading events...</p>
      ) : error ? (
        <p className="error">Error: {error}</p>
      ) : (
        <div className="table-scroll">
          <table className="data-table">
            <thead>
              <tr>
                <th>Time</th>
                <th>Type</th>
                <th>Actor</th>
                <th>WSIDs</th>
                <th>Payload</th>
              </tr>
            </thead>
            <tbody>
              {events.map((event) => (
                <tr
                  key={event.event_id}
                  className="clickable-row"
                  onClick={() => onSelect({ kind: "event", event })}
                >
                  <td className="muted">{new Date(event.timestamp).toLocaleString()}</td>
                  <td>{event.event_type}</td>
                  <td className="muted">{event.actor}</td>
                  <td className="muted small">{event.wsids.join(", ") || "n/a"}</td>
                  <td className="muted small">{JSON.stringify(event.payload)}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
      {exportOpen && (
        <DebugBundleExport
          isOpen={exportOpen}
          defaultScope={{
            kind: "time_window",
            time_range: {
              start: filters.from ? new Date(filters.from).toISOString() : new Date().toISOString(),
              end: filters.to ? new Date(filters.to).toISOString() : new Date().toISOString(),
            },
            wsid: filters.wsid || undefined,
          }}
          onClose={() => setExportOpen(false)}
        />
      )}
    </div>
  );
};
