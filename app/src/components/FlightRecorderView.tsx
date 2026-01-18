import { FormEvent, useCallback, useEffect, useMemo, useRef, useState } from "react";
import {
  createDiagnostic,
  FlightEvent,
  FlightEventFilters,
  getEvents,
  SecurityViolationPayload,
} from "../lib/api";

type UiFilters = {
  jobId: string;
  traceId: string;
  eventId: string;
  wsid: string;
  actor: "" | NonNullable<FlightEventFilters["actor"]>;
  eventType: "" | FlightEvent["event_type"];
  from: string;
  to: string;
};

const eventTypeOptions: FlightEvent["event_type"][] = [
  "system",
  "llm_inference",
  "diagnostic",
  "capability_action",
  "security_violation",
  "workflow_recovery",
  "debug_bundle_export",
  "governance_pack_export",
];

const defaultFilters: UiFilters = {
  jobId: "",
  traceId: "",
  eventId: "",
  wsid: "",
  actor: "",
  eventType: "",
  from: "",
  to: "",
};

function normalizeFilters(filters: UiFilters): UiFilters {
  return {
    ...filters,
    jobId: filters.jobId.trim(),
    traceId: filters.traceId.trim(),
    eventId: filters.eventId.trim(),
    wsid: filters.wsid.trim(),
  };
}

function toApiFilters(filters: UiFilters): FlightEventFilters | undefined {
  const normalized = normalizeFilters(filters);
  const apiFilters: FlightEventFilters = {};
  if (normalized.eventId) apiFilters.eventId = normalized.eventId;
  if (normalized.jobId) apiFilters.jobId = normalized.jobId;
  if (normalized.traceId) apiFilters.traceId = normalized.traceId;
  if (normalized.wsid) apiFilters.wsid = normalized.wsid;
  if (normalized.actor) apiFilters.actor = normalized.actor;
  if (normalized.eventType) apiFilters.eventType = normalized.eventType;
  if (normalized.from) apiFilters.from = normalized.from;
  if (normalized.to) apiFilters.to = normalized.to;
  return Object.keys(apiFilters).length > 0 ? apiFilters : undefined;
}

async function emitNavFailure(
  action: string,
  reason: string,
  ctx: { wsid?: string; jobId?: string; traceId?: string; eventId?: string; diagnosticId?: string } = {},
) {
  try {
    await createDiagnostic({
      title: "VAL-NAV-001 Navigation failure",
      message: `${action}: ${reason}`,
      severity: "warning",
      source: "system",
      surface: "system",
      code: "VAL-NAV-001",
      wsid: ctx.wsid ?? null,
      job_id: ctx.jobId ?? null,
      evidence_refs: ctx.eventId ? { fr_event_ids: [ctx.eventId] } : null,
      link_confidence: "unlinked",
    });
  } catch {
    // Ignore emission failures (do not cascade UI failure into more failures).
  }
}

function readInitialFiltersFromUrl(): UiFilters {
  if (typeof window === "undefined") return defaultFilters;

  const params = new URLSearchParams(window.location.search);
  const actor = params.get("actor");
  const eventType = params.get("event_type");
  const from = params.get("from");
  const to = params.get("to");

  return normalizeFilters({
    jobId: params.get("job_id") ?? "",
    traceId: params.get("trace_id") ?? "",
    eventId: params.get("event_id") ?? "",
    wsid: params.get("wsid") ?? "",
    actor: actor === "human" || actor === "agent" || actor === "system" ? actor : "",
    eventType: eventTypeOptions.includes(eventType as FlightEvent["event_type"])
      ? (eventType as FlightEvent["event_type"])
      : "",
    from: from ?? "",
    to: to ?? "",
  });
}

function syncFiltersToUrl(filters: UiFilters) {
  if (typeof window === "undefined") return;

  const params = new URLSearchParams();
  if (filters.jobId) params.set("job_id", filters.jobId);
  if (filters.traceId) params.set("trace_id", filters.traceId);
  if (filters.eventId) params.set("event_id", filters.eventId);
  if (filters.wsid) params.set("wsid", filters.wsid);
  if (filters.actor) params.set("actor", filters.actor);
  if (filters.eventType) params.set("event_type", filters.eventType);
  if (filters.from) params.set("from", filters.from);
  if (filters.to) params.set("to", filters.to);

  const query = params.toString();
  const next = `${window.location.pathname}${query.length > 0 ? `?${query}` : ""}`;
  try {
    window.history.replaceState(null, "", next);
  } catch {
    // Ignore URL sync failures (e.g., restricted history APIs).
  }
}

function redactMessage(message: string, visibleChars = 180): string {
  if (message.length <= visibleChars) return message;
  return `${message.slice(0, visibleChars)}... [redacted preview]`;
}

function redactJsonValue(value: unknown, visibleChars = 180, depth = 0): unknown {
  const MAX_DEPTH = 6;
  const MAX_ARRAY = 50;
  if (depth > MAX_DEPTH) return "[redacted: depth limit]";

  if (typeof value === "string") return redactMessage(value, visibleChars);
  if (value === null || value === undefined) return value;
  if (typeof value !== "object") return value;
  if (Array.isArray(value)) return value.slice(0, MAX_ARRAY).map((v) => redactJsonValue(v, visibleChars, depth + 1));

  const obj = value as Record<string, unknown>;
  const out: Record<string, unknown> = {};
  for (const [key, val] of Object.entries(obj)) {
    if (/(token|secret|password|api[_-]?key)/i.test(key)) {
      out[key] = "[redacted]";
      continue;
    }
    out[key] = redactJsonValue(val, visibleChars, depth + 1);
  }
  return out;
}

function extractDiagnosticId(payload: unknown): string | null {
  if (!payload || typeof payload !== "object") return null;
  const record = payload as Record<string, unknown>;
  const raw = record.diagnostic_id ?? record.diagnosticId;
  return typeof raw === "string" && raw.trim().length > 0 ? raw.trim() : null;
}

export function FlightRecorderView() {
  const initialFilters = useMemo(() => readInitialFiltersFromUrl(), []);

  const [events, setEvents] = useState<FlightEvent[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const [draftFilters, setDraftFilters] = useState<UiFilters>(initialFilters);
  const [appliedFilters, setAppliedFilters] = useState<UiFilters>(initialFilters);
  const [selectedEventId, setSelectedEventId] = useState<string | null>(initialFilters.eventId || null);
  const [rawPayloadEventId, setRawPayloadEventId] = useState<string | null>(null);

  const inFlightRef = useRef(false);
  const rowByEventIdRef = useRef<Record<string, HTMLTableRowElement | null>>({});
  const lastNavFailureKeyRef = useRef<string | null>(null);

  const fetchEvents = useCallback(async (filters: UiFilters) => {
    if (inFlightRef.current) return;
    inFlightRef.current = true;

    try {
      const apiFilters = toApiFilters(filters);
      const data = await getEvents(apiFilters);
      setEvents(data);
      setError(null);

      const trimmedEventId = filters.eventId.trim();
      const trimmedJobId = filters.jobId.trim();
      const trimmedTraceId = filters.traceId.trim();
      const trimmedWsid = filters.wsid.trim();

      const hasAnyDeepLinkId =
        trimmedEventId.length > 0 || trimmedJobId.length > 0 || trimmedTraceId.length > 0 || trimmedWsid.length > 0;

      if (data.length === 0) {
        if (hasAnyDeepLinkId) {
          const navKey = JSON.stringify({ mode: "no_results", jobId: trimmedJobId, traceId: trimmedTraceId, eventId: trimmedEventId, wsid: trimmedWsid });

          if (lastNavFailureKeyRef.current !== navKey) {
            lastNavFailureKeyRef.current = navKey;
            void emitNavFailure("resolve_filters", "no events found for deep link id(s)", {
              jobId: trimmedJobId || undefined,
              traceId: trimmedTraceId || undefined,
              eventId: trimmedEventId || undefined,
              wsid: trimmedWsid || undefined,
            });
          }
        }

        setNotice(
          hasAnyDeepLinkId
            ? "No events found for the provided filters (see VAL-NAV-001 diagnostic)."
            : "No events recorded yet.",
        );
      } else {
        if (trimmedEventId.length > 0 && !data.some((evt) => evt.event_id === trimmedEventId)) {
          const navKey = JSON.stringify({
            mode: "missing_event_id",
            jobId: trimmedJobId,
            traceId: trimmedTraceId,
            eventId: trimmedEventId,
            wsid: trimmedWsid,
          });

          if (lastNavFailureKeyRef.current !== navKey) {
            lastNavFailureKeyRef.current = navKey;
            void emitNavFailure("resolve_filters", "event_id not present in returned events", {
              jobId: trimmedJobId || undefined,
              traceId: trimmedTraceId || undefined,
              eventId: trimmedEventId,
              wsid: trimmedWsid || undefined,
            });
          }

          setNotice("Event focus failed: event_id not present in returned events (see VAL-NAV-001 diagnostic).");
        } else {
          setNotice(null);
        }
      }
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to fetch events");
    } finally {
      setLoading(false);
      inFlightRef.current = false;
    }
  }, []);

  const applyFilters = (next: UiFilters) => {
    const normalized = normalizeFilters(next);
    setDraftFilters(normalized);
    setAppliedFilters(normalized);
    setLoading(true);
    setNotice(null);
    syncFiltersToUrl(normalized);
  };

  useEffect(() => {
    void fetchEvents(appliedFilters);
    const interval = setInterval(() => void fetchEvents(appliedFilters), 5000);
    return () => clearInterval(interval);
  }, [appliedFilters, fetchEvents]);

  useEffect(() => {
    const target = appliedFilters.eventId.trim();
    if (!target) return;
    const row = rowByEventIdRef.current[target];
    if (!row) return;
    row.scrollIntoView({ block: "center" });
    setSelectedEventId(target);
  }, [events, appliedFilters.eventId]);

  const onSubmit = (e: FormEvent) => {
    e.preventDefault();
    applyFilters(draftFilters);
  };

  const onReset = () => {
    lastNavFailureKeyRef.current = null;
    setSelectedEventId(null);
    setRawPayloadEventId(null);
    applyFilters(defaultFilters);
  };

  const toggleFilter = (key: keyof UiFilters, value: string) => {
    const current = draftFilters[key];
    const nextValue = current === value ? "" : value;
    const next = { ...draftFilters, [key]: nextValue } as UiFilters;
    setSelectedEventId(null);
    setRawPayloadEventId(null);
    applyFilters(next);
  };

  const focusEvent = (eventId: string) => {
    const trimmed = eventId.trim();
    if (!trimmed) {
      void emitNavFailure("focus_event", "missing event_id");
      setNotice("Event focus failed: missing event_id (see VAL-NAV-001 diagnostic).");
      return;
    }

    const row = rowByEventIdRef.current[trimmed];
    if (!row) {
      toggleFilter("eventId", trimmed);
      return;
    }

    row.scrollIntoView({ block: "center" });
    setSelectedEventId(trimmed);
  };

  const copyText = async (text: string, action: string, ctx?: { eventId?: string; jobId?: string; wsid?: string }) => {
    try {
      await navigator.clipboard.writeText(text);
      setNotice("Copied to clipboard.");
    } catch {
      setNotice("Copy failed (see VAL-NAV-001 diagnostic).");
      void emitNavFailure(action, "clipboard write failed", ctx);
    }
  };

  const currentLinkTarget = useMemo(() => {
    if (typeof window === "undefined") return "";
    const apiFilters = toApiFilters(appliedFilters);
    const params = new URLSearchParams();
    if (apiFilters?.eventId) params.set("event_id", apiFilters.eventId);
    if (apiFilters?.jobId) params.set("job_id", apiFilters.jobId);
    if (apiFilters?.traceId) params.set("trace_id", apiFilters.traceId);
    if (apiFilters?.wsid) params.set("wsid", apiFilters.wsid);
    if (apiFilters?.actor) params.set("actor", apiFilters.actor);
    if (apiFilters?.eventType) params.set("event_type", apiFilters.eventType);
    if (apiFilters?.from) params.set("from", apiFilters.from);
    if (apiFilters?.to) params.set("to", apiFilters.to);
    const query = params.toString();
    return `${window.location.origin}${window.location.pathname}${query.length > 0 ? `?${query}` : ""}`;
  }, [appliedFilters]);

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
      <div className="flight-recorder__header">
        <div>
          <h2>Flight Recorder</h2>
          <p className="muted">Chronological log of AI actions and job lifecycle events.</p>
        </div>
        <button
          type="button"
          className="secondary"
          onClick={() => void fetchEvents(appliedFilters)}
          disabled={loading}
          title="Refresh now"
        >
          Refresh
        </button>
      </div>

      <form className="flight-recorder__filters" onSubmit={onSubmit}>
        <label>
          Job ID
          <input
            value={draftFilters.jobId}
            onChange={(e) => setDraftFilters({ ...draftFilters, jobId: e.target.value })}
            placeholder="job-uuid"
          />
        </label>
        <label>
          Trace ID
          <input
            value={draftFilters.traceId}
            onChange={(e) => setDraftFilters({ ...draftFilters, traceId: e.target.value })}
            placeholder="trace-uuid"
          />
        </label>
        <label>
          Event ID
          <input
            value={draftFilters.eventId}
            onChange={(e) => setDraftFilters({ ...draftFilters, eventId: e.target.value })}
            placeholder="event-uuid"
          />
        </label>
        <label>
          WSID
          <input
            value={draftFilters.wsid}
            onChange={(e) => setDraftFilters({ ...draftFilters, wsid: e.target.value })}
            placeholder="workspace id"
          />
        </label>
        <label>
          Actor
          <select
            value={draftFilters.actor}
            onChange={(e) => setDraftFilters({ ...draftFilters, actor: e.target.value as UiFilters["actor"] })}
          >
            <option value="">Any</option>
            <option value="human">human</option>
            <option value="agent">agent</option>
            <option value="system">system</option>
          </select>
        </label>
        <label>
          Event Type
          <select
            value={draftFilters.eventType}
            onChange={(e) =>
              setDraftFilters({ ...draftFilters, eventType: e.target.value as UiFilters["eventType"] })
            }
          >
            <option value="">Any</option>
            {eventTypeOptions.map((eventType) => (
              <option key={eventType} value={eventType}>
                {eventType}
              </option>
            ))}
          </select>
        </label>
        <label>
          From
          <input
            type="datetime-local"
            value={draftFilters.from}
            onChange={(e) => setDraftFilters({ ...draftFilters, from: e.target.value })}
          />
        </label>
        <label>
          To
          <input
            type="datetime-local"
            value={draftFilters.to}
            onChange={(e) => setDraftFilters({ ...draftFilters, to: e.target.value })}
          />
        </label>
        <div className="flight-recorder__filter-actions">
          <button type="submit" disabled={loading}>
            Apply
          </button>
          <button type="button" className="secondary" onClick={onReset} disabled={loading}>
            Clear
          </button>
          <button
            type="button"
            className="secondary"
            onClick={() => void copyText(currentLinkTarget, "copy_link_target")}
            disabled={!currentLinkTarget}
            title="Copy a deterministic link target for these filters"
          >
            Copy Link Target
          </button>
        </div>
      </form>

      {(notice || error) && (
        <div className={`flight-recorder__notice ${error ? "flight-recorder__notice--error" : ""}`}>
          {error ? `Error: ${error}` : notice}
        </div>
      )}

      <div className="flight-recorder__table-container">
        <table className="flight-recorder__table">
          <thead>
            <tr>
              <th>Timestamp</th>
              <th>Type</th>
              <th>Actor</th>
              <th>Job ID</th>
              <th>Trace ID</th>
              <th>WSIDs</th>
              <th>Event ID</th>
              <th>Payload</th>
            </tr>
          </thead>
          <tbody>
            {events.map((event) => {
              const isSecurityViolation = event.event_type === "security_violation";
              const isSelected = selectedEventId === event.event_id;
              const rowClass = [
                isSecurityViolation ? "flight-recorder__row--violation" : "",
                isSelected ? "flight-recorder__row--selected" : "",
              ]
                .filter(Boolean)
                .join(" ");

              const payload = event.payload as Partial<SecurityViolationPayload>;
              const diagnosticId = extractDiagnosticId(event.payload);
              const redactedPayload = redactJsonValue(event.payload);
              const showRaw = rawPayloadEventId === event.event_id;

              return (
                <tr
                  key={event.event_id}
                  className={rowClass}
                  ref={(node) => {
                    rowByEventIdRef.current[event.event_id] = node;
                  }}
                >
                  <td className="nowrap">{new Date(event.timestamp).toLocaleString()}</td>
                  <td>
                    <span
                      className={`event-tag event-tag--${event.event_type}${
                        isSecurityViolation ? " event-tag--security" : ""
                      }`}
                      title={isSecurityViolation ? "FR-EVT-SEC-VIOLATION" : undefined}
                    >
                      {event.event_type}
                    </span>
                  </td>
                  <td className="muted">{event.actor_id || event.actor}</td>
                  <td
                    className={`muted ${event.job_id ? "clickable" : ""}`}
                    title={event.job_id ? "Filter by this job_id" : "No job_id"}
                    onClick={() => event.job_id && toggleFilter("jobId", event.job_id)}
                  >
                    {event.job_id || "-"}
                  </td>
                  <td
                    className="muted clickable"
                    title="Filter by this trace_id"
                    onClick={() => toggleFilter("traceId", event.trace_id)}
                  >
                    {event.trace_id}
                  </td>
                  <td className="muted">
                    {event.wsids.length === 0 ? (
                      "-"
                    ) : (
                      <div className="flight-recorder__wsids">
                        {event.wsids.map((wsid) => (
                          <button
                            type="button"
                            key={wsid}
                            className="id-chip"
                            onClick={() => toggleFilter("wsid", wsid)}
                            title="Filter by this wsid"
                          >
                            {wsid}
                          </button>
                        ))}
                      </div>
                    )}
                  </td>
                  <td className="muted">
                    <button
                      type="button"
                      className="id-link"
                      onClick={() => focusEvent(event.event_id)}
                      title="Focus/select this event in the timeline"
                    >
                      {event.event_id}
                    </button>
                  </td>
                  <td>
                    {isSecurityViolation && payload.trigger && (
                      <div className="violation-context">
                        <strong>Trigger:</strong> <code>{redactMessage(payload.trigger)}</code>
                        {payload.offset !== undefined && (
                          <span>
                            {" "}
                            | <strong>Offset:</strong> {payload.offset}
                          </span>
                        )}
                        {payload.context && (
                          <div className="violation-snippet">
                            <strong>Snippet:</strong> <code>...{redactMessage(payload.context)}...</code>
                          </div>
                        )}
                      </div>
                    )}

                    {diagnosticId && (
                      <div className="flight-recorder__diag-link">
                        <span className="muted small">diagnostic_id:</span>{" "}
                        <button
                          type="button"
                          className="id-link"
                          onClick={() =>
                            void copyText(
                              `Problems: diagnostic_id=${diagnosticId}`,
                              "copy_diagnostic_id",
                              { eventId: event.event_id, jobId: event.job_id, wsid: event.wsids[0] },
                            )
                          }
                          title="Copy a deterministic link target"
                        >
                          {diagnosticId}
                        </button>
                      </div>
                    )}

                    <details className="flight-recorder__payload-details">
                      <summary>Payload (redacted by default)</summary>
                      <div className="flight-recorder__payload-actions">
                        <button
                          type="button"
                          className="secondary"
                          onClick={() => setRawPayloadEventId(showRaw ? null : event.event_id)}
                        >
                          {showRaw ? "Hide raw" : "Reveal raw (unsafe)"}
                        </button>
                        <button
                          type="button"
                          className="secondary"
                          onClick={() =>
                            void copyText(
                              JSON.stringify(showRaw ? event.payload : redactedPayload, null, 2),
                              "copy_payload_json",
                              { eventId: event.event_id, jobId: event.job_id, wsid: event.wsids[0] },
                            )
                          }
                        >
                          Copy JSON
                        </button>
                      </div>
                      {showRaw && (
                        <p className="flight-recorder__payload-warning">
                          Warning: raw payload may contain secrets/PII. Use only when explicitly necessary.
                        </p>
                      )}
                      <pre className="event-payload">
                        {JSON.stringify(showRaw ? event.payload : redactedPayload, null, 2)}
                      </pre>
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
