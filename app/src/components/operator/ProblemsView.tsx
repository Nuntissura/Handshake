import React, { FormEvent, useEffect, useState } from "react";
import {
  Diagnostic,
  DiagnosticSeverity,
  DiagnosticStatus,
  DiagnosticSurface,
  ProblemGroup,
  listDiagnostics,
  listProblemGroups,
} from "../../lib/api";
import { EvidenceSelection } from "./EvidenceDrawer";
import { DebugBundleExport } from "./DebugBundleExport";

type Props = {
  onSelect: (selection: EvidenceSelection) => void;
};

type ProblemFilters = {
  severity: "" | DiagnosticSeverity;
  source: string;
  surface: "" | DiagnosticSurface;
  wsid: string;
  job_id: string;
  from: string;
  to: string;
};

const defaultFilters: ProblemFilters = {
  severity: "",
  source: "",
  surface: "",
  wsid: "",
  job_id: "",
  from: "",
  to: "",
};

export const ProblemsView: React.FC<Props> = ({ onSelect }) => {
  const [filters, setFilters] = useState<ProblemFilters>(defaultFilters);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [problems, setProblems] = useState<ProblemGroup[]>([]);
  const [selectedProblem, setSelectedProblem] = useState<ProblemGroup | null>(null);
  const [rawInstances, setRawInstances] = useState<Diagnostic[]>([]);
  const [rawLoading, setRawLoading] = useState(false);
  const [statusByFingerprint, setStatusByFingerprint] = useState<Record<string, DiagnosticStatus>>({});
  const [exportOpen, setExportOpen] = useState(false);

  useEffect(() => {
    try {
      const raw = localStorage.getItem("handshake.problems.statusByFingerprint");
      if (!raw) return;
      const parsed = JSON.parse(raw) as Record<string, DiagnosticStatus>;
      setStatusByFingerprint(parsed);
    } catch {
      setStatusByFingerprint({});
    }
  }, []);

  const fetchProblems = async (override?: ProblemFilters) => {
    const active = override ?? filters;
    setLoading(true);
    const query = {
      severity: active.severity || undefined,
      source: active.source || undefined,
      surface: active.surface || undefined,
      wsid: active.wsid || undefined,
      job_id: active.job_id || undefined,
      from: active.from || undefined,
      to: active.to || undefined,
      limit: 200,
    };

    try {
      const data = await listProblemGroups(query);
      setProblems(data);
      setError(null);
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to load diagnostics");
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    fetchProblems();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const selectedFingerprint = selectedProblem?.fingerprint;
  useEffect(() => {
    if (!selectedFingerprint) {
      setRawInstances([]);
      return;
    }
    setRawLoading(true);
    listDiagnostics({ fingerprint: selectedFingerprint, limit: 200 })
      .then(setRawInstances)
      .catch(() => setRawInstances([]))
      .finally(() => setRawLoading(false));
  }, [selectedFingerprint]);

  const setProblemStatus = (fingerprint: string, status: DiagnosticStatus) => {
    setStatusByFingerprint((prev) => {
      const next = { ...prev, [fingerprint]: status };
      try {
        localStorage.setItem("handshake.problems.statusByFingerprint", JSON.stringify(next));
      } catch {
        // ignore localStorage failures
      }
      return next;
    });
  };

  const onSubmit = (e: FormEvent) => {
    e.preventDefault();
    fetchProblems();
  };

  return (
    <div className="content-card">
      <div className="card-header">
        <div>
          <h2>Problems</h2>
          <p className="muted">
            Grouped diagnostics with fingerprint-based clustering. Filters align to DIAG-SCHEMA-001/003.
          </p>
        </div>
        <div className="card-actions">
          <button
            className="primary"
            type="button"
            disabled={!selectedProblem}
            onClick={() => setExportOpen(true)}
          >
            Export Debug Bundle
          </button>
        </div>
      </div>

      <form className="filters-grid" onSubmit={onSubmit}>
        <label>
          Severity
          <select
            value={filters.severity}
            onChange={(e) => setFilters({ ...filters, severity: e.target.value as ProblemFilters["severity"] })}
          >
            <option value="">Any</option>
            <option value="fatal">Fatal</option>
            <option value="error">Error</option>
            <option value="warning">Warning</option>
            <option value="info">Info</option>
            <option value="hint">Hint</option>
          </select>
        </label>
        <label>
          Source
          <input
            placeholder="validator, plugin:spellcheck"
            value={filters.source}
            onChange={(e) => setFilters({ ...filters, source: e.target.value })}
          />
        </label>
        <label>
          Surface
          <input
            placeholder="monaco / canvas / terminal"
            value={filters.surface}
            onChange={(e) =>
              setFilters({
                ...filters,
                surface: e.target.value as ProblemFilters["surface"],
              })
            }
          />
        </label>
        <label>
          Workspace
          <input
            placeholder="wsid-123"
            value={filters.wsid}
            onChange={(e) => setFilters({ ...filters, wsid: e.target.value })}
          />
        </label>
        <label>
          Job ID
          <input
            placeholder="job uuid"
            value={filters.job_id}
            onChange={(e) => setFilters({ ...filters, job_id: e.target.value })}
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
              fetchProblems(defaultFilters);
            }}
          >
            Reset
          </button>
        </div>
      </form>

      {loading && problems.length === 0 ? (
        <p>Loading diagnostics...</p>
      ) : error ? (
        <p className="error">Error: {error}</p>
      ) : (
        <div className="table-scroll">
          <table className="data-table">
            <thead>
              <tr>
                <th>Severity</th>
                <th>Status</th>
                <th>Title</th>
                <th>Source</th>
                <th>Surface</th>
                <th>Link</th>
                <th>Count</th>
                <th>First</th>
                <th>Last</th>
              </tr>
            </thead>
            <tbody>
              {problems.map((problem) => {
                const status = statusByFingerprint[problem.fingerprint] ?? "open";
                return (
                  <tr
                    key={problem.fingerprint}
                    onClick={() => {
                      setSelectedProblem(problem);
                      onSelect({ kind: "diagnostic", diagnostic: problem.sample });
                    }}
                    className="clickable-row"
                  >
                    <td>
                      <span className={`chip chip--${problem.sample.severity}`}>{problem.sample.severity}</span>
                    </td>
                    <td onClick={(e) => e.stopPropagation()}>
                      <select
                        value={status}
                        onChange={(e) => setProblemStatus(problem.fingerprint, e.target.value as DiagnosticStatus)}
                      >
                        <option value="open">open</option>
                        <option value="acknowledged">ack</option>
                        <option value="muted">mute</option>
                        <option value="resolved">resolved</option>
                      </select>
                    </td>
                    <td>
                      <strong>{problem.sample.title}</strong>
                      <div className="muted small">{problem.sample.message}</div>
                    </td>
                    <td className="muted">{problem.sample.source}</td>
                    <td className="muted">{problem.sample.surface}</td>
                    <td className="muted">{problem.sample.link_confidence}</td>
                    <td>{problem.count}</td>
                    <td className="muted">{new Date(problem.first_seen).toLocaleString()}</td>
                    <td className="muted">{new Date(problem.last_seen).toLocaleString()}</td>
                  </tr>
                );
              })}
            </tbody>
          </table>
        </div>
      )}

      {selectedProblem && (
        <div className="content-card">
          <h3>Raw instances</h3>
          <p className="muted small">Fingerprint: {selectedProblem.fingerprint}</p>
          {rawLoading ? (
            <p className="muted">Loading instances...</p>
          ) : rawInstances.length === 0 ? (
            <p className="muted">No instances found for this fingerprint.</p>
          ) : (
            <div className="table-scroll">
              <table className="data-table">
                <thead>
                  <tr>
                    <th>When</th>
                    <th>Severity</th>
                    <th>Job</th>
                    <th>Message</th>
                  </tr>
                </thead>
                <tbody>
                  {rawInstances.map((diag) => (
                    <tr
                      key={diag.id}
                      className="clickable-row"
                      onClick={() => onSelect({ kind: "diagnostic", diagnostic: diag })}
                    >
                      <td className="muted">{new Date(diag.timestamp).toLocaleString()}</td>
                      <td>
                        <span className={`chip chip--${diag.severity}`}>{diag.severity}</span>
                      </td>
                      <td className="muted small">{diag.job_id ?? "n/a"}</td>
                      <td className="muted small">{diag.message}</td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          )}
        </div>
      )}
      {exportOpen && (
        <DebugBundleExport
          isOpen={exportOpen}
          defaultScope={
            selectedProblem
              ? { kind: "problem", problem_id: selectedProblem.sample.id }
              : { kind: "problem", problem_id: "" }
          }
          onClose={() => setExportOpen(false)}
        />
      )}
    </div>
  );
};
