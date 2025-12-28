import React, { useEffect, useMemo, useState } from "react";
import {
  AiJob,
  Diagnostic,
  FlightEvent,
  getEvents,
  listDiagnostics,
  listJobs,
} from "../../lib/api";
import { EvidenceSelection } from "./EvidenceDrawer";

type Props = {
  onSelect: (selection: EvidenceSelection) => void;
};

type JobFilters = {
  status: string;
  job_kind: string;
  from: string;
  to: string;
};

const defaultFilters: JobFilters = {
  status: "",
  job_kind: "",
  from: "",
  to: "",
};

type Tab = "summary" | "timeline" | "io" | "diagnostics" | "policy";

function hashString(value: string): string {
  let hash = 0;
  for (let i = 0; i < value.length; i += 1) {
    hash = (hash * 31 + value.charCodeAt(i)) >>> 0;
  }
  return hash.toString(16);
}

export const JobsView: React.FC<Props> = ({ onSelect }) => {
  const [filters, setFilters] = useState<JobFilters>(defaultFilters);
  const [jobs, setJobs] = useState<AiJob[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [selectedJob, setSelectedJob] = useState<AiJob | null>(null);
  const [events, setEvents] = useState<FlightEvent[]>([]);
  const [jobDiagnostics, setJobDiagnostics] = useState<Diagnostic[]>([]);
  const [activeTab, setActiveTab] = useState<Tab>("summary");

  const fetchJobs = async (override?: JobFilters) => {
    const active = override ?? filters;
    setLoading(true);
    try {
      const data = await listJobs({
        status: active.status || undefined,
        job_kind: active.job_kind || undefined,
        from: active.from || undefined,
        to: active.to || undefined,
      });
      setJobs(data);
      setError(null);
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to load jobs");
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    fetchJobs();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  useEffect(() => {
    if (!selectedJob) return;
    getEvents({ jobId: selectedJob.job_id })
      .then(setEvents)
      .catch(() => setEvents([]));
    listDiagnostics({ job_id: selectedJob.job_id })
      .then(setJobDiagnostics)
      .catch(() => setJobDiagnostics([]));
  }, [selectedJob]);

  const activeJobInputsHash = useMemo(() => {
    if (!selectedJob?.job_inputs) return "n/a";
    const normalized =
      typeof selectedJob.job_inputs === "string"
        ? selectedJob.job_inputs
        : JSON.stringify(selectedJob.job_inputs);
    return hashString(normalized);
  }, [selectedJob]);

  const activeJobOutputsHash = useMemo(() => {
    if (!selectedJob?.job_outputs) return "n/a";
    const normalized =
      typeof selectedJob.job_outputs === "string"
        ? selectedJob.job_outputs
        : JSON.stringify(selectedJob.job_outputs);
    return hashString(normalized);
  }, [selectedJob]);

  return (
    <div className="content-card">
      <div className="card-header">
        <div>
          <h2>Jobs</h2>
          <p className="muted">
            Jobs with inspector tabs for Summary, Timeline, Inputs/Outputs (hash-based), Diagnostics, and Policy.
          </p>
        </div>
      </div>

      <div className="filters-grid">
        <label>
          Status
          <input
            placeholder="queued/running/completed"
            value={filters.status}
            onChange={(e) => setFilters({ ...filters, status: e.target.value })}
          />
        </label>
        <label>
          Kind
          <input
            placeholder="job kind"
            value={filters.job_kind}
            onChange={(e) => setFilters({ ...filters, job_kind: e.target.value })}
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
          <button type="button" onClick={() => fetchJobs()}>
            Apply
          </button>
          <button
            type="button"
            className="secondary"
            onClick={() => {
              setFilters(defaultFilters);
              fetchJobs(defaultFilters);
            }}
          >
            Reset
          </button>
        </div>
      </div>

      {loading && jobs.length === 0 ? (
        <p>Loading jobs...</p>
      ) : error ? (
        <p className="error">Error: {error}</p>
      ) : (
        <div className="jobs-layout">
          <div className="jobs-list">
            {jobs.map((job) => (
              <div
                key={job.job_id}
                className={`job-card ${selectedJob?.job_id === job.job_id ? "job-card--active" : ""}`}
                onClick={() => {
                  setSelectedJob(job);
                  setActiveTab("summary");
                }}
              >
                <div className="chip-row">
                  <span className="chip chip--ghost">{job.job_kind}</span>
                  <span className="chip">{job.state}</span>
                </div>
                <p className="muted small">{job.job_id}</p>
                {job.error_message && <p className="error small">{job.error_message}</p>}
              </div>
            ))}
          </div>

          <div className="job-inspector">
            {selectedJob ? (
              <>
                <div className="tabs">
                  {(["summary", "timeline", "io", "diagnostics", "policy"] as Tab[]).map((tab) => (
                    <button
                      key={tab}
                      className={activeTab === tab ? "active" : ""}
                      onClick={() => setActiveTab(tab)}
                    >
                      {tab.toUpperCase()}
                    </button>
                  ))}
                </div>
                <div className="tab-content">
                  {activeTab === "summary" && (
                    <div>
                      <h3>Summary</h3>
                      <ul className="meta-list">
                        <li>Job ID: {selectedJob.job_id}</li>
                        <li>Status: {selectedJob.state}</li>
                        <li>Kind: {selectedJob.job_kind}</li>
                        <li>Protocol: {selectedJob.protocol_id}</li>
                        <li>Access Mode: {selectedJob.access_mode}</li>
                        <li>Safety Mode: {selectedJob.safety_mode}</li>
                        <li>Created: {new Date(selectedJob.created_at).toLocaleString()}</li>
                        <li>Updated: {new Date(selectedJob.updated_at).toLocaleString()}</li>
                      </ul>
                    </div>
                  )}
                  {activeTab === "timeline" && (
                    <div className="table-scroll">
                      <table className="data-table">
                        <thead>
                          <tr>
                            <th>Time</th>
                            <th>Type</th>
                            <th>Actor</th>
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
                              <td className="muted">{event.actor_id}</td>
                              <td className="muted small">{JSON.stringify(event.payload)}</td>
                            </tr>
                          ))}
                        </tbody>
                      </table>
                    </div>
                  )}
                  {activeTab === "io" && (
                    <div>
                      <h3>Inputs / Outputs (hash-based)</h3>
                      <p className="muted small">
                        Hashes provide stable references without leaking payloads. Use Debug Bundle to fetch full content.
                      </p>
                      <ul className="meta-list">
                        <li>Inputs hash: {activeJobInputsHash}</li>
                        <li>Outputs hash: {activeJobOutputsHash}</li>
                      </ul>
                    </div>
                  )}
                  {activeTab === "diagnostics" && (
                    <div className="table-scroll">
                      <table className="data-table">
                        <thead>
                          <tr>
                            <th>Severity</th>
                            <th>Title</th>
                            <th>Link</th>
                            <th>When</th>
                          </tr>
                        </thead>
                        <tbody>
                          {jobDiagnostics.map((diag) => (
                            <tr
                              key={diag.id}
                              className="clickable-row"
                              onClick={() => onSelect({ kind: "diagnostic", diagnostic: diag })}
                            >
                              <td>
                                <span className={`chip chip--${diag.severity}`}>{diag.severity}</span>
                              </td>
                              <td>{diag.title}</td>
                              <td className="muted">{diag.link_confidence}</td>
                              <td className="muted">{new Date(diag.timestamp).toLocaleString()}</td>
                            </tr>
                          ))}
                        </tbody>
                      </table>
                    </div>
                  )}
                  {activeTab === "policy" && (
                    <div>
                      <h3>Policy Context</h3>
                      <ul className="meta-list">
                        <li>Capability Profile: {selectedJob.capability_profile_id}</li>
                        <li>Access Mode: {selectedJob.access_mode}</li>
                        <li>Safety Mode: {selectedJob.safety_mode}</li>
                        <li>Profile ID: {selectedJob.profile_id}</li>
                      </ul>
                      <p className="muted small">
                        Policy decisions are linked through diagnostics and capability actions in Flight Recorder.
                      </p>
                    </div>
                  )}
                </div>
              </>
            ) : (
              <p className="muted">Select a job to inspect details.</p>
            )}
          </div>
        </div>
      )}
    </div>
  );
};
