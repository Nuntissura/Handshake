import React, { useEffect, useState } from "react";
import {
  AiJob,
  asFemsJobOutput,
  Diagnostic,
  FlightEvent,
  getJob,
  getEvents,
  isFemsProtocolId,
  listDiagnostics,
  listJobs,
  resumeJob,
  submitCloudEscalationConsent,
} from "../../lib/api";
import { EvidenceSelection } from "./EvidenceDrawer";
import { DebugBundleExport } from "./DebugBundleExport";

type Props = {
  onSelect: (selection: EvidenceSelection) => void;
  focusJobId?: string | null;
};

type JobFilters = {
  status: string;
  job_kind: string;
  wsid: string;
  from: string;
  to: string;
};

const defaultFilters: JobFilters = {
  status: "",
  job_kind: "",
  wsid: "",
  from: "",
  to: "",
};

type Tab = "summary" | "timeline" | "io" | "memory" | "diagnostics" | "policy";

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

function isRecord(value: unknown): value is Record<string, unknown> {
  return !!value && typeof value === "object" && !Array.isArray(value);
}

const LOCAL_USER_ID_KEY = "hsk.local_user_id.v1";

function getOrCreateLocalUserId(): string {
  const uuid =
    typeof crypto !== "undefined" && typeof crypto.randomUUID === "function"
      ? crypto.randomUUID()
      : `${Date.now()}-${Math.random().toString(16).slice(2)}`;

  try {
    const existing = localStorage.getItem(LOCAL_USER_ID_KEY);
    if (existing && existing.startsWith("local:") && existing.length <= 128) return existing;

    const minted = `local:${uuid}`;
    localStorage.setItem(LOCAL_USER_ID_KEY, minted);
    return minted;
  } catch {
    return `local:${uuid}`;
  }
}

export const JobsView: React.FC<Props> = ({ onSelect, focusJobId }) => {
  const [filters, setFilters] = useState<JobFilters>(defaultFilters);
  const [jobs, setJobs] = useState<AiJob[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [selectedJob, setSelectedJob] = useState<AiJob | null>(null);
  const [events, setEvents] = useState<FlightEvent[]>([]);
  const [jobDiagnostics, setJobDiagnostics] = useState<Diagnostic[]>([]);
  const [activeTab, setActiveTab] = useState<Tab>("summary");
  const [exportOpen, setExportOpen] = useState(false);
  const [inputsHash, setInputsHash] = useState("n/a");
  const [outputsHash, setOutputsHash] = useState("n/a");
  const [localUserId] = useState(() => getOrCreateLocalUserId());
  const [consentNotes, setConsentNotes] = useState("");
  const [consentSubmitting, setConsentSubmitting] = useState(false);
  const [consentError, setConsentError] = useState<string | null>(null);

  const fetchJobs = async (override?: JobFilters) => {
    const active = override ?? filters;
    setLoading(true);
    try {
      const data = await listJobs({
        status: active.status || undefined,
        job_kind: active.job_kind || undefined,
        wsid: active.wsid || undefined,
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
    if (!focusJobId) return;
    getJob(focusJobId)
      .then((job) => {
        setSelectedJob(job);
        setJobs((prev) => (prev.some((j) => j.job_id === job.job_id) ? prev : [job, ...prev]));
        setActiveTab("summary");
      })
      .catch(() => {});
  }, [focusJobId]);

  useEffect(() => {
    if (!selectedJob) return;
    getEvents({ jobId: selectedJob.job_id })
      .then(setEvents)
      .catch(() => setEvents([]));
    listDiagnostics({ job_id: selectedJob.job_id })
      .then(setJobDiagnostics)
      .catch(() => setJobDiagnostics([]));
  }, [selectedJob]);

  useEffect(() => {
    let cancelled = false;
    const run = async () => {
      if (!selectedJob?.job_inputs) {
        setInputsHash("n/a");
        return;
      }
      const normalized =
        typeof selectedJob.job_inputs === "string"
          ? selectedJob.job_inputs
          : stableStringify(selectedJob.job_inputs);
      const hash = await sha256Hex(normalized);
      if (!cancelled) setInputsHash(hash);
    };
    run().catch(() => {
      if (!cancelled) setInputsHash("error");
    });
    return () => {
      cancelled = true;
    };
  }, [selectedJob?.job_inputs]);

  useEffect(() => {
    let cancelled = false;
    const run = async () => {
      if (!selectedJob?.job_outputs) {
        setOutputsHash("n/a");
        return;
      }
      const normalized =
        typeof selectedJob.job_outputs === "string"
          ? selectedJob.job_outputs
          : stableStringify(selectedJob.job_outputs);
      const hash = await sha256Hex(normalized);
      if (!cancelled) setOutputsHash(hash);
    };
    run().catch(() => {
      if (!cancelled) setOutputsHash("error");
    });
    return () => {
      cancelled = true;
    };
  }, [selectedJob?.job_outputs]);

  useEffect(() => {
    setConsentError(null);
    setConsentNotes("");
  }, [selectedJob?.job_id]);

  const cloudConsentOutput = (() => {
    if (!selectedJob || selectedJob.state !== "awaiting_user") return null;
    if (!isRecord(selectedJob.job_outputs)) return null;
    if (selectedJob.job_outputs["reason"] !== "cloud_escalation_consent_required") return null;
    return selectedJob.job_outputs;
  })();
  const femsOutput = selectedJob ? asFemsJobOutput(selectedJob.job_outputs) : null;
  const memoryEvents = events.filter((event) => event.event_type.startsWith("memory_"));
  const femsMemoryPack =
    femsOutput && isRecord(femsOutput.memory_pack) ? (femsOutput.memory_pack as Record<string, unknown>) : null;
  const femsReview = femsOutput?.review;
  const femsProposal =
    femsOutput && isRecord(femsOutput.proposal) ? (femsOutput.proposal as Record<string, unknown>) : null;
  const femsCommitReport =
    femsOutput && isRecord(femsOutput.commit_report)
      ? (femsOutput.commit_report as Record<string, unknown>)
      : null;

  const submitConsent = async (approved: boolean) => {
    if (!selectedJob || !cloudConsentOutput) return;

    const requestId = cloudConsentOutput["request_id"];
    if (typeof requestId !== "string" || requestId.trim().length === 0) {
      setConsentError("Missing request_id in job outputs");
      return;
    }

    setConsentSubmitting(true);
    setConsentError(null);
    try {
      await submitCloudEscalationConsent(selectedJob.job_id, {
        request_id: requestId,
        approved,
        user_id: localUserId,
        ui_surface: "operator_console",
        notes: consentNotes.trim().length > 0 ? consentNotes.trim() : undefined,
      });

      await resumeJob(selectedJob.job_id);

      const refreshed = await getJob(selectedJob.job_id);
      setSelectedJob(refreshed);
      setJobs((prev) => prev.map((j) => (j.job_id === refreshed.job_id ? refreshed : j)));
    } catch (err) {
      setConsentError(err instanceof Error ? err.message : "Failed to record cloud consent");
    } finally {
      setConsentSubmitting(false);
    }
  };

  return (
    <div className="content-card">
      <div className="card-header">
        <div>
          <h2>Jobs</h2>
          <p className="muted">
            Jobs with inspector tabs for Summary, Timeline, Inputs/Outputs (hash-based), Diagnostics, and Policy.
          </p>
        </div>
        <div className="card-actions">
          <button
            className="primary"
            type="button"
            disabled={!selectedJob}
            onClick={() => setExportOpen(true)}
          >
            Export Debug Bundle
          </button>
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
          Workspace
          <input
            placeholder="wsid"
            value={filters.wsid}
            onChange={(e) => setFilters({ ...filters, wsid: e.target.value })}
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
                  {(["summary", "timeline", "io", "memory", "diagnostics", "policy"] as Tab[]).map((tab) => (
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

                      {cloudConsentOutput && (
                        <div className="content-card" style={{ marginTop: 16 }}>
                          <h4>Cloud Escalation Consent Required</h4>
                          <p className="muted small">
                            This job is paused before a cloud invocation. Approving records a ConsentReceipt and resumes
                            the workflow.
                          </p>
                          <ul className="meta-list">
                            <li>User ID: {localUserId}</li>
                            <li>Request ID: {String(cloudConsentOutput["request_id"] ?? "n/a")}</li>
                            <li>Requested Model: {String(cloudConsentOutput["requested_model_id"] ?? "n/a")}</li>
                            <li>Payload SHA-256: {String(cloudConsentOutput["payload_sha256"] ?? "n/a")}</li>
                            <li>Projection Plan ID: {String(cloudConsentOutput["projection_plan_id"] ?? "n/a")}</li>
                          </ul>
                          <details>
                            <summary>ProjectionPlan</summary>
                            <pre className="muted small">
                              {JSON.stringify(cloudConsentOutput["projection_plan"] ?? null, null, 2)}
                            </pre>
                          </details>
                          <label>
                            Notes (optional)
                            <textarea
                              rows={3}
                              placeholder="Add optional notes (no secrets)"
                              value={consentNotes}
                              onChange={(e) => setConsentNotes(e.target.value)}
                            />
                          </label>
                          <div className="filter-actions">
                            <button type="button" disabled={consentSubmitting} onClick={() => submitConsent(true)}>
                              Approve + Resume
                            </button>
                            <button
                              type="button"
                              className="secondary"
                              disabled={consentSubmitting}
                              onClick={() => submitConsent(false)}
                            >
                              Deny
                            </button>
                          </div>
                          {consentError && <p className="error small">Error: {consentError}</p>}
                        </div>
                      )}
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
                        <li>Inputs hash: {inputsHash}</li>
                        <li>Outputs hash: {outputsHash}</li>
                      </ul>
                    </div>
                  )}
                  {activeTab === "memory" && (
                    <div>
                      <h3>Memory</h3>
                      {!selectedJob || !isFemsProtocolId(selectedJob.protocol_id) ? (
                        <p className="muted small">
                          Memory preview/review is available for FEMS protocols (`memory_extract_v0.1`,
                          `memory_consolidate_v0.1`, `memory_forget_v0.1`).
                        </p>
                      ) : (
                        <>
                          <ul className="meta-list">
                            <li>Protocol: {selectedJob.protocol_id}</li>
                            <li>Memory policy: {femsOutput?.memory_policy ?? "n/a"}</li>
                            <li>Memory state ref: {femsOutput?.memory_state_ref ?? "n/a"}</li>
                            <li>Pack hash: {femsOutput?.memory_pack_hash ?? "n/a"}</li>
                            <li>Proposal hash: {femsOutput?.proposal_hash ?? "n/a"}</li>
                            <li>Commit report hash: {femsOutput?.commit_report_hash ?? "n/a"}</li>
                            <li>Review status: {femsReview?.status ?? "n/a"}</li>
                            <li>Review required ops: {femsReview?.required_ops ?? 0}</li>
                            <li>Reviewer kind: {femsReview?.reviewer_kind ?? "n/a"}</li>
                          </ul>
                          {femsOutput?.warning && <p className="error small">Warning: {femsOutput.warning}</p>}

                          {femsMemoryPack && (
                            <details>
                              <summary>MemoryPack Preview</summary>
                              <ul className="meta-list">
                                <li>Pack ID: {String(femsMemoryPack["pack_id"] ?? "n/a")}</li>
                                <li>Item count: {String(femsMemoryPack["item_count"] ?? "n/a")}</li>
                                <li>Token estimate: {String(femsMemoryPack["token_estimate"] ?? "n/a")}</li>
                                <li>
                                  Truncation occurred: {String(femsMemoryPack["truncation_occurred"] ?? "n/a")}
                                </li>
                                <li>Redaction applied: {String(femsMemoryPack["redaction_applied"] ?? "n/a")}</li>
                              </ul>
                            </details>
                          )}

                          {(femsProposal || femsCommitReport) && (
                            <details>
                              <summary>Review Artifacts</summary>
                              <ul className="meta-list">
                                <li>Proposal ID: {String(femsProposal?.["proposal_id"] ?? "n/a")}</li>
                                <li>Commit ID: {String(femsCommitReport?.["commit_id"] ?? "n/a")}</li>
                                <li>Decision: {String(femsCommitReport?.["decision"] ?? "n/a")}</li>
                              </ul>
                            </details>
                          )}

                          <h4 style={{ marginTop: 16 }}>Memory Events</h4>
                          {memoryEvents.length === 0 ? (
                            <p className="muted small">No memory FR events recorded for this job yet.</p>
                          ) : (
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
                                  {memoryEvents.map((event) => (
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
                        </>
                      )}
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
      {exportOpen && (
        <DebugBundleExport
          isOpen={exportOpen}
          defaultScope={
            selectedJob
              ? { kind: "job", job_id: selectedJob.job_id }
              : { kind: "job", job_id: "" }
          }
          onClose={() => setExportOpen(false)}
        />
      )}
    </div>
  );
};
