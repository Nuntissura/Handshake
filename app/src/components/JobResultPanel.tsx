import { useEffect, useState } from "react";
import { AiJob, getJob } from "../lib/api";

type Props = {
  jobId: string;
  onDismiss: () => void;
};

export function JobResultPanel({ jobId, onDismiss }: Props) {
  const [job, setJob] = useState<AiJob | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let active = true;
    const fetchJob = async () => {
      try {
        setLoading(true);
        const data = await getJob(jobId);
        if (active) {
          setJob(data);
          setLoading(false);
          // If still running/queued, poll again
          if (data.state === "running" || data.state === "queued") {
            setTimeout(fetchJob, 1000);
          }
        }
      } catch (err) {
        if (active) {
          setError(err instanceof Error ? err.message : "Failed to load job");
          setLoading(false);
        }
      }
    };
    fetchJob();
    return () => {
      active = false;
    };
  }, [jobId]);

  if (!job) {
    return (
      <div className="job-result-panel">
        <div className="job-result-header">
          <h3>AI Job</h3>
          <button onClick={onDismiss}>Close</button>
        </div>
        <p className="muted">Loading job details...</p>
      </div>
    );
  }

  let outputSummary: string | null = null;
  if (job.state === "completed" && job.job_outputs) {
    if (typeof job.job_outputs === "string") {
      try {
        const outputs = JSON.parse(job.job_outputs);
        outputSummary = outputs.summary ?? JSON.stringify(outputs, null, 2);
      } catch {
        outputSummary = "Failed to parse job outputs.";
      }
    } else if (typeof job.job_outputs === "object") {
      outputSummary = JSON.stringify(job.job_outputs, null, 2);
    }
  }

  return (
    <div className="job-result-panel">
      <div className="job-result-header">
        <h3>AI Job: {job.job_kind}</h3>
        <button onClick={onDismiss}>Close</button>
      </div>
      <div className="job-result-content">
        <p>
          <strong>Status:</strong> {job.state}
        </p>
        {loading && (job.state === "running" || job.state === "queued") && (
          <p className="muted">Processing...</p>
        )}
        {error && <p className="error">Error: {error}</p>}
        {job.error_message && <p className="error">Job Error: {job.error_message}</p>}
        {outputSummary && (
          <div className="job-summary">
            <h4>Summary</h4>
            <pre>{outputSummary}</pre>
          </div>
        )}
      </div>
    </div>
  );
}
