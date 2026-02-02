import { useEffect, useMemo, useState } from "react";
import { AiJob, sha256HexUtf8 } from "../lib/api";

function stableNormalize(value: unknown, seen = new Set<object>()): unknown {
  if (value === null || typeof value !== "object") return value;
  if (seen.has(value as object)) return "[Circular]";
  seen.add(value as object);

  if (Array.isArray(value)) return value.map((entry) => stableNormalize(entry, seen));

  const record = value as Record<string, unknown>;
  const keys = Object.keys(record).sort();
  const out: Record<string, unknown> = {};
  for (const key of keys) {
    out[key] = stableNormalize(record[key], seen);
  }
  return out;
}

function stableStringify(value: unknown): string {
  return JSON.stringify(stableNormalize(value));
}

type Props = {
  job: AiJob | null;
  loading?: boolean;
  error?: string | null;
  onDismiss: () => void;
};

export function JobResultPanel({ job, loading = false, error = null, onDismiss }: Props) {
  const [outputsHash, setOutputsHash] = useState<string>("n/a");
  const [outputsPreviewRevealedForJobId, setOutputsPreviewRevealedForJobId] = useState<string | null>(null);
  const showOutputsPreview = outputsPreviewRevealedForJobId !== null && outputsPreviewRevealedForJobId === job?.job_id;
  const jobOutputs = job?.job_outputs;

  useEffect(() => {
    let cancelled = false;

    const run = async () => {
      if (jobOutputs === undefined || jobOutputs === null) {
        setOutputsHash("n/a");
        return;
      }

      setOutputsHash("computing...");
      const normalized =
        typeof jobOutputs === "string" ? jobOutputs : stableStringify(jobOutputs);
      const hash = await sha256HexUtf8(normalized);
      if (!cancelled) setOutputsHash(hash);
    };

    run().catch(() => {
      if (!cancelled) setOutputsHash("error");
    });

    return () => {
      cancelled = true;
    };
  }, [jobOutputs]);

  const outputsPreviewText = useMemo(() => {
    if (jobOutputs === undefined || jobOutputs === null) return null;
    if (typeof jobOutputs === "string") return jobOutputs;
    return JSON.stringify(stableNormalize(jobOutputs), null, 2);
  }, [jobOutputs]);

  const previewDisabledBySafetyMode = job?.safety_mode === "strict";

  if (!job) {
    return (
      <div className="job-result-panel">
        <div className="job-result-header">
          <h3>AI Job</h3>
          <button type="button" onClick={onDismiss}>
            Close
          </button>
        </div>
        {error ? <p className="error">Error: {error}</p> : <p className="muted">Loading job details...</p>}
      </div>
    );
  }

  return (
    <div className="job-result-panel">
      <div className="job-result-header">
        <h3>AI Job: {job.job_kind}</h3>
        <button type="button" onClick={onDismiss}>
          Close
        </button>
      </div>
      <div className="job-result-content">
        <p>
          <strong>Status:</strong> {job.state}
        </p>
        <p>
          <strong>Job ID:</strong> {job.job_id}
        </p>
        <p>
          <strong>Trace ID:</strong> {job.trace_id}
        </p>
        {loading && (job.state === "running" || job.state === "queued") && (
          <p className="muted">Processing...</p>
        )}
        {error && <p className="error">Error: {error}</p>}
        {job.error_message && <p className="error">Job Error: {job.error_message}</p>}
        <div className="job-summary">
          <h4>Summary</h4>
          <p className="muted small">Job outputs are not auto-rendered. Hash is shown by default.</p>
          <p>
            <strong>Outputs hash:</strong> {outputsHash}
          </p>

          <button
            type="button"
            className="secondary"
            disabled={previewDisabledBySafetyMode || !outputsPreviewText}
            onClick={() => setOutputsPreviewRevealedForJobId(job.job_id)}
          >
            Reveal output preview
          </button>

          {previewDisabledBySafetyMode && (
            <p className="muted small">Preview disabled in strict safety mode.</p>
          )}

          {showOutputsPreview && outputsPreviewText && (
            <div className="job-summary__preview">
              <p className="muted small">
                Warning: output preview may contain sensitive content. Rendered as plain text.
              </p>
              <pre>{outputsPreviewText}</pre>
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
