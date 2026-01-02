import React, { FormEvent, useEffect, useState } from "react";
import { BundleScopeInput, FlightEvent, getEvents } from "../../lib/api";
import { EvidenceSelection } from "./EvidenceDrawer";
import { DebugBundleExport } from "./DebugBundleExport";

type Props = {
  onSelect: (selection: EvidenceSelection) => void;
  navigation?: { job_id?: string; wsid?: string; event_id?: string } | null;
  onTimeWindowChange?: (window: { start: string; end: string; wsid?: string } | null) => void;
};

type TimelineFilters = {
  event_id: string;
  job_id: string;
  wsid: string;
  actor: "" | "human" | "agent" | "system";
  surface: string;
  event_type: "" | FlightEvent["event_type"];
  from: string;
  to: string;
};

type PinnedSliceQuery = {
  time_range: { start: string; end: string };
  wsid: string | null;
  job_id: string | null;
  actor: TimelineFilters["actor"] | null;
  surface: string | null;
  event_type: TimelineFilters["event_type"] | null;
  event_id: string | null;
};

type PinnedSlice = {
  slice_id: string;
  pinned_at: string;
  query: PinnedSliceQuery;
};

const defaultFilters: TimelineFilters = {
  event_id: "",
  job_id: "",
  wsid: "",
  actor: "",
  surface: "",
  event_type: "",
  from: "",
  to: "",
};

const PINNED_SLICES_STORAGE_KEY = "handshake.timeline.pinnedSlices.v1";

function stableStringify(value: unknown): string {
  const seen = new WeakSet<object>();
  const normalize = (input: unknown): unknown => {
    if (!input || typeof input !== "object") return input;
    if (seen.has(input as object)) return "[Circular]";
    seen.add(input as object);

    if (Array.isArray(input)) return input.map(normalize);

    const record = input as Record<string, unknown>;
    const keys = Object.keys(record).sort();
    const out: Record<string, unknown> = {};
    for (const key of keys) {
      out[key] = normalize(record[key]);
    }
    return out;
  };

  return JSON.stringify(normalize(value));
}

async function sha256Hex(value: string): Promise<string> {
  const data = new TextEncoder().encode(value);
  const digest = await crypto.subtle.digest("SHA-256", data);
  return Array.from(new Uint8Array(digest))
    .map((b) => b.toString(16).padStart(2, "0"))
    .join("");
}

function toIsoFromLocal(value: string): string | null {
  const date = new Date(value);
  return Number.isNaN(date.getTime()) ? null : date.toISOString();
}

function activeTimeWindowFromFilters(filters: TimelineFilters): { start: string; end: string; wsid?: string } | null {
  if (!filters.from || !filters.to) return null;
  const start = toIsoFromLocal(filters.from);
  const end = toIsoFromLocal(filters.to);
  if (!start || !end) return null;
  if (new Date(start).getTime() > new Date(end).getTime()) return null;

  const wsid = filters.wsid.trim();
  return wsid.length > 0 ? { start, end, wsid } : { start, end };
}

function normalizePinnedSliceQuery(filters: TimelineFilters, timeWindow: { start: string; end: string }): PinnedSliceQuery {
  const wsid = filters.wsid.trim();
  const job_id = filters.job_id.trim();
  const surface = filters.surface.trim();
  const event_id = filters.event_id.trim();

  return {
    time_range: { start: timeWindow.start, end: timeWindow.end },
    wsid: wsid.length > 0 ? wsid : null,
    job_id: job_id.length > 0 ? job_id : null,
    actor: filters.actor || null,
    surface: surface.length > 0 ? surface : null,
    event_type: filters.event_type || null,
    event_id: event_id.length > 0 ? event_id : null,
  };
}

export const TimelineView: React.FC<Props> = ({ onSelect, navigation, onTimeWindowChange }) => {
  const [filters, setFilters] = useState<TimelineFilters>(defaultFilters);
  const [events, setEvents] = useState<FlightEvent[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [pinnedSlices, setPinnedSlices] = useState<PinnedSlice[]>([]);
  const [exportScope, setExportScope] = useState<BundleScopeInput | null>(null);

  const fetchEvents = async (override?: TimelineFilters) => {
    const active = override ?? filters;
    setLoading(true);
    try {
      const data = await getEvents({
        eventId: active.event_id || undefined,
        jobId: active.job_id || undefined,
        wsid: active.wsid || undefined,
        actor: active.actor || undefined,
        surface: active.surface || undefined,
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

  useEffect(() => {
    try {
      const raw = localStorage.getItem(PINNED_SLICES_STORAGE_KEY);
      if (!raw) return;
      const parsed = JSON.parse(raw) as PinnedSlice[];
      if (!Array.isArray(parsed)) return;
      setPinnedSlices(parsed);
    } catch {
      setPinnedSlices([]);
    }
  }, []);

  useEffect(() => {
    try {
      localStorage.setItem(PINNED_SLICES_STORAGE_KEY, JSON.stringify(pinnedSlices));
    } catch {
      // ignore localStorage failures
    }
  }, [pinnedSlices]);

  useEffect(() => {
    if (!navigation) return;
    const next: TimelineFilters = {
      ...defaultFilters,
      job_id: navigation.job_id ?? "",
      wsid: navigation.wsid ?? "",
      event_id: navigation.event_id ?? "",
    };
    setFilters(next);
    fetchEvents(next);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [navigation]);

  useEffect(() => {
    onTimeWindowChange?.(activeTimeWindowFromFilters(filters));
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [filters.from, filters.to, filters.wsid]);

  const onSubmit = (e: FormEvent) => {
    e.preventDefault();
    fetchEvents();
  };

  const timeWindow = activeTimeWindowFromFilters(filters);

  const pinSlice = async () => {
    if (!timeWindow) {
      setError("Set a valid From/To window before pinning a slice.");
      return;
    }

    const query = normalizePinnedSliceQuery(filters, timeWindow);
    const sliceId = await sha256Hex(stableStringify(query));
    const pinned: PinnedSlice = { slice_id: sliceId, pinned_at: new Date().toISOString(), query };
    setPinnedSlices((prev) => (prev.some((p) => p.slice_id === pinned.slice_id) ? prev : [pinned, ...prev]));
  };

  const exportCurrentWindow = () => {
    if (!timeWindow) {
      setError("Set a valid From/To window before exporting a time-window bundle.");
      return;
    }

    setExportScope({ kind: "time_window", time_range: { start: timeWindow.start, end: timeWindow.end }, wsid: timeWindow.wsid });
  };

  const exportPinnedSlice = (slice: PinnedSlice) => {
    const wsid = slice.query.wsid ?? undefined;
    setExportScope({ kind: "time_window", time_range: slice.query.time_range, wsid });
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
          <button className="secondary" onClick={pinSlice} disabled={!timeWindow}>
            Pin this slice
          </button>
          <button className="primary" type="button" onClick={exportCurrentWindow} disabled={!timeWindow}>
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
          Surface
          <input
            value={filters.surface}
            onChange={(e) => setFilters({ ...filters, surface: e.target.value })}
            placeholder="monaco, canvas, terminal, system"
          />
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
          <p className="muted small">
            Export uses time window (+wsid) only; actor/surface/event_type filters are UI-only until Debug Bundle contract expands in a separate WP.
          </p>
          <div className="table-scroll">
            <table className="data-table">
              <thead>
                <tr>
                  <th>Slice ID</th>
                  <th>Window</th>
                  <th>WSID</th>
                  <th>UI Filters</th>
                  <th></th>
                </tr>
              </thead>
              <tbody>
                {pinnedSlices.map((slice) => (
                  <tr key={slice.slice_id}>
                    <td className="muted small">{slice.slice_id}</td>
                    <td className="muted small">
                      {new Date(slice.query.time_range.start).toLocaleString()} - {new Date(slice.query.time_range.end).toLocaleString()}
                    </td>
                    <td className="muted small">{slice.query.wsid ?? "any"}</td>
                    <td className="muted small">
                      job_id={slice.query.job_id ?? "any"}, actor={slice.query.actor ?? "any"}, surface={slice.query.surface ?? "any"}, event={slice.query.event_type ?? "any"}
                    </td>
                    <td style={{ whiteSpace: "nowrap" }}>
                      <button className="secondary" type="button" onClick={() => exportPinnedSlice(slice)}>
                        Export
                      </button>{" "}
                      <button
                        className="secondary"
                        type="button"
                        onClick={() => setPinnedSlices((prev) => prev.filter((p) => p.slice_id !== slice.slice_id))}
                      >
                        Remove
                      </button>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
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
      {exportScope && (
        <DebugBundleExport
          isOpen={true}
          defaultScope={exportScope}
          onClose={() => setExportScope(null)}
        />
      )}
    </div>
  );
};
