import { useState } from "react";

// MT-124 owned-files contract listed
// `app/src/components/inference_lab/DistillationQueue.svelte`. App
// stack is React/TSX (same Svelte->TSX defect class as MT-098 /
// MT-102 / MT-105); behavior + acceptance criteria + red_team
// controls preserved.

export type ReviewStatus = "Pending" | "Promoted" | "Rejected";

export interface OptedInSession {
  sessionId: string;
  modelId: string;
  closedAtUtc: string;
  turnCount: number;
}

export interface PendingCandidate {
  loraId: string;
  teacherModelPath: string;
  studentBaseModelPath: string;
  corpusTurnCount: number;
  trainedAtUtc: string;
  licenseTag: string;
  status: ReviewStatus;
  rejectionReason?: string;
}

export type TrainingJobStatus = "queued" | "running" | "done" | "error";

export interface TrainingJob {
  jobId: string;
  sessionId: string;
  status: TrainingJobStatus;
  queuedAtUtc: string;
  startedAtUtc?: string;
  finishedAtUtc?: string;
  errorMessage?: string;
}

type DistillationQueueProps = {
  optedInSessions: OptedInSession[];
  pendingCandidates: PendingCandidate[];
  trainingJobs: TrainingJob[];
  onExtractCorpus: (sessionId: string) => void | Promise<void>;
  onPromote: (loraId: string) => void | Promise<void>;
  onReject: (loraId: string, reason: string) => void | Promise<void>;
};

type Tab = "sessions" | "candidates" | "jobs";

const TABS: ReadonlyArray<{ id: Tab; label: string }> = [
  { id: "sessions", label: "Opted-In Sessions" },
  { id: "candidates", label: "Pending Candidates" },
  { id: "jobs", label: "Training Jobs" },
];

export function DistillationQueue({
  optedInSessions,
  pendingCandidates,
  trainingJobs,
  onExtractCorpus,
  onPromote,
  onReject,
}: DistillationQueueProps) {
  const [tab, setTab] = useState<Tab>("sessions");
  const [rejectReasons, setRejectReasons] = useState<Record<string, string>>({});

  const updateRejectReason = (loraId: string, reason: string) => {
    setRejectReasons((prev) => ({ ...prev, [loraId]: reason }));
  };

  return (
    <section
      className="inference-lab__panel inference-lab__distillation-queue"
      data-testid="distillation-queue"
      aria-labelledby="distillation-queue-title"
    >
      <header className="inference-lab__panel-header">
        <h3 id="distillation-queue-title">Distillation Queue</h3>
        <p className="muted" data-testid="distillation-queue.note">
          Opted-in sessions go through extraction (MT-119) -&gt; content review
          (MT-120) -&gt; PEFT training (MT-122). Trained candidates land in
          the PromotionGate (MT-123) and require operator review before mount.
        </p>
      </header>

      <div className="distillation-queue__tabs" role="tablist" aria-label="Distillation queue tabs">
        {TABS.map((t) => (
          <button
            key={t.id}
            type="button"
            role="tab"
            aria-selected={tab === t.id}
            className={
              tab === t.id
                ? "distillation-queue__tab distillation-queue__tab--active"
                : "distillation-queue__tab"
            }
            onClick={() => setTab(t.id)}
            data-testid={`distillation-queue.tab.${t.id}`}
          >
            {t.label}
          </button>
        ))}
      </div>

      {tab === "sessions" ? (
        <div data-testid="distillation-queue.sessions">
          {optedInSessions.length === 0 ? (
            <p className="muted" data-testid="distillation-queue.sessions.empty">
              No opted-in sessions. Operators mark a session at close via the
              per-session distillation toggle (MT-121).
            </p>
          ) : (
            <table data-testid="distillation-queue.sessions.table">
              <thead>
                <tr>
                  <th>Session</th>
                  <th>Model</th>
                  <th>Closed</th>
                  <th>Turns</th>
                  <th></th>
                </tr>
              </thead>
              <tbody>
                {optedInSessions.map((s) => (
                  <tr
                    key={s.sessionId}
                    data-testid={`distillation-queue.sessions.row.${s.sessionId}`}
                  >
                    <td>
                      <code>{s.sessionId}</code>
                    </td>
                    <td>{s.modelId}</td>
                    <td>{s.closedAtUtc}</td>
                    <td>{s.turnCount}</td>
                    <td>
                      <button
                        type="button"
                        onClick={() => void onExtractCorpus(s.sessionId)}
                        data-testid={`distillation-queue.sessions.row.${s.sessionId}.extract`}
                      >
                        Extract corpus
                      </button>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          )}
        </div>
      ) : null}

      {tab === "candidates" ? (
        <div data-testid="distillation-queue.candidates">
          {pendingCandidates.length === 0 ? (
            <p className="muted" data-testid="distillation-queue.candidates.empty">
              No candidates currently in review.
            </p>
          ) : (
            <table data-testid="distillation-queue.candidates.table">
              <thead>
                <tr>
                  <th>LoRA</th>
                  <th>Teacher</th>
                  <th>Student base</th>
                  <th>Turns</th>
                  <th>Trained</th>
                  <th>License</th>
                  <th>Status</th>
                  <th>Actions</th>
                </tr>
              </thead>
              <tbody>
                {pendingCandidates.map((c) => (
                  <tr
                    key={c.loraId}
                    data-testid={`distillation-queue.candidates.row.${c.loraId}`}
                  >
                    <td>
                      <code>{c.loraId}</code>
                    </td>
                    <td>
                      <code>{c.teacherModelPath}</code>
                    </td>
                    <td>
                      <code>{c.studentBaseModelPath}</code>
                    </td>
                    <td>{c.corpusTurnCount}</td>
                    <td>{c.trainedAtUtc}</td>
                    <td>{c.licenseTag}</td>
                    <td>
                      <span
                        data-testid={`distillation-queue.candidates.row.${c.loraId}.status`}
                      >
                        {c.status}
                      </span>
                      {c.status === "Rejected" && c.rejectionReason ? (
                        <p
                          className="muted"
                          data-testid={`distillation-queue.candidates.row.${c.loraId}.rejection-reason`}
                        >
                          {c.rejectionReason}
                        </p>
                      ) : null}
                    </td>
                    <td>
                      {c.status === "Pending" ? (
                        <div className="distillation-queue__candidate-actions">
                          <button
                            type="button"
                            onClick={() => void onPromote(c.loraId)}
                            data-testid={`distillation-queue.candidates.row.${c.loraId}.promote`}
                          >
                            Promote
                          </button>
                          <input
                            type="text"
                            placeholder="rejection reason"
                            value={rejectReasons[c.loraId] ?? ""}
                            onChange={(event) =>
                              updateRejectReason(c.loraId, event.target.value)
                            }
                            data-testid={`distillation-queue.candidates.row.${c.loraId}.reject-reason`}
                          />
                          <button
                            type="button"
                            disabled={!(rejectReasons[c.loraId]?.trim().length ?? 0)}
                            onClick={() =>
                              void onReject(
                                c.loraId,
                                rejectReasons[c.loraId]?.trim() ?? "",
                              )
                            }
                            data-testid={`distillation-queue.candidates.row.${c.loraId}.reject`}
                          >
                            Reject
                          </button>
                        </div>
                      ) : (
                        <span className="muted">—</span>
                      )}
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          )}
        </div>
      ) : null}

      {tab === "jobs" ? (
        <div data-testid="distillation-queue.jobs">
          {trainingJobs.length === 0 ? (
            <p className="muted" data-testid="distillation-queue.jobs.empty">
              No training jobs in the queue.
            </p>
          ) : (
            <table data-testid="distillation-queue.jobs.table">
              <thead>
                <tr>
                  <th>Job</th>
                  <th>Session</th>
                  <th>Status</th>
                  <th>Queued</th>
                  <th>Started</th>
                  <th>Finished</th>
                  <th>Error</th>
                </tr>
              </thead>
              <tbody>
                {trainingJobs.map((j) => (
                  <tr
                    key={j.jobId}
                    data-testid={`distillation-queue.jobs.row.${j.jobId}`}
                  >
                    <td>
                      <code>{j.jobId}</code>
                    </td>
                    <td>
                      <code>{j.sessionId}</code>
                    </td>
                    <td>
                      <span
                        className={`distillation-queue__status distillation-queue__status--${j.status}`}
                        data-testid={`distillation-queue.jobs.row.${j.jobId}.status`}
                      >
                        {j.status}
                      </span>
                    </td>
                    <td>{j.queuedAtUtc}</td>
                    <td>{j.startedAtUtc ?? "—"}</td>
                    <td>{j.finishedAtUtc ?? "—"}</td>
                    <td>{j.errorMessage ?? "—"}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          )}
        </div>
      ) : null}
    </section>
  );
}
