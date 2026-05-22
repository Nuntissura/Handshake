import { useCallback, useEffect, useState } from "react";
import {
  extractDistillCorpus,
  listDistillCandidates,
  listDistillJobs,
  listDistillSessions,
  promoteDistillCandidate,
  rejectDistillCandidate,
  type OptedInSession as IpcOptedInSession,
  type PendingCandidate as IpcPendingCandidate,
  type TrainingJobSummary as IpcTrainingJob,
} from "../../lib/ipc/distillation";
import {
  capabilities as fetchCapabilities,
  listLoaded,
  type LoadedModelRuntime,
  type ModelCapabilities,
} from "../../lib/ipc/model_runtime";
import { CaaWizard } from "./CaaWizard";
import {
  DistillationQueue,
  type OptedInSession,
  type PendingCandidate,
  type TrainingJob,
} from "./DistillationQueue";
import { KvCachePanel } from "./KvCachePanel";
import { LoraStackComposer } from "./LoraStackComposer";
import { RefusalVectorWizard } from "./RefusalVectorWizard";
import { SpeculativeDecodingPanel } from "./SpeculativeDecodingPanel";
import { SteeringVectorEditor } from "./SteeringVectorEditor";

// Default visible layer range for steering layer pickers when the kernel does not
// expose n_layers yet. Per spec 10.14.2 the picker should reflect the loaded
// model's layer count; this is a conservative interim ceiling so the dropdown
// still renders sensible options. Future MTs (LoRA / KV / Subquadratic panels)
// will replace this with kernel-supplied metadata.
const DEFAULT_LAYER_COUNT = 32;

// Operator signature attached to per-session promote/reject calls. Until
// the operator-identity UI surface lands, the lab uses the operator's
// known signature (matches the MT-121 / MT-123 operator-signature
// vocabulary). The Tauri backend rejects empty signatures.
const OPERATOR_SIGNATURE = "operator-inference-lab";

type ModelsState =
  | { status: "loading" }
  | { status: "error"; message: string }
  | { status: "ready"; models: LoadedModelRuntime[] };

type CapState =
  | { status: "idle" }
  | { status: "loading" }
  | { status: "error"; message: string }
  | { status: "ready"; capabilities: ModelCapabilities };

type DistillationQueueState =
  | { status: "loading" }
  | { status: "error"; message: string }
  | {
      status: "ready";
      sessions: OptedInSession[];
      candidates: PendingCandidate[];
      jobs: TrainingJob[];
    };

function errorMessage(error: unknown): string {
  if (error instanceof Error) return error.message;
  if (typeof error === "string") return error;
  return "Unknown error";
}

function adaptSession(row: IpcOptedInSession): OptedInSession {
  return {
    sessionId: row.sessionId,
    modelId: row.modelId,
    closedAtUtc: row.closedAtUtc,
    turnCount: row.turnCount,
  };
}

function adaptCandidate(row: IpcPendingCandidate): PendingCandidate {
  return {
    loraId: row.loraId,
    teacherModelPath: row.teacherModelPath,
    studentBaseModelPath: row.studentBaseModelPath,
    corpusTurnCount: row.corpusTurnCount,
    trainedAtUtc: row.trainedAtUtc,
    licenseTag: row.licenseTag,
    status: row.status,
    rejectionReason: row.rejectionReason ?? undefined,
  };
}

function adaptJob(row: IpcTrainingJob): TrainingJob {
  return {
    jobId: row.jobId,
    sessionId: row.sessionId,
    status: row.status,
    queuedAtUtc: row.queuedAtUtc,
    startedAtUtc: row.startedAtUtc ?? undefined,
    finishedAtUtc: row.finishedAtUtc ?? undefined,
    errorMessage: row.errorMessage ?? undefined,
  };
}

export function InferenceLab() {
  const [models, setModels] = useState<ModelsState>({ status: "loading" });
  const [selectedModelId, setSelectedModelId] = useState<string | null>(null);
  const [caps, setCaps] = useState<CapState>({ status: "idle" });
  const [queue, setQueue] = useState<DistillationQueueState>({ status: "loading" });

  useEffect(() => {
    let active = true;
    listLoaded()
      .then((loaded) => {
        if (!active) return;
        setModels({ status: "ready", models: loaded });
        if (loaded.length > 0 && selectedModelId === null) {
          setSelectedModelId(loaded[0].modelId);
        }
      })
      .catch((error) => {
        if (active) {
          setModels({ status: "error", message: errorMessage(error) });
        }
      });
    return () => {
      active = false;
    };
    // selectedModelId intentionally excluded: this effect seeds the selection once.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  useEffect(() => {
    if (!selectedModelId) {
      setCaps({ status: "idle" });
      return;
    }
    let active = true;
    setCaps({ status: "loading" });
    fetchCapabilities(selectedModelId)
      .then((capabilities) => {
        if (active) setCaps({ status: "ready", capabilities });
      })
      .catch((error) => {
        if (active) setCaps({ status: "error", message: errorMessage(error) });
      });
    return () => {
      active = false;
    };
  }, [selectedModelId]);

  const refreshQueue = useCallback(async () => {
    setQueue({ status: "loading" });
    try {
      const [sessions, candidates, jobs] = await Promise.all([
        listDistillSessions(),
        listDistillCandidates(),
        listDistillJobs(),
      ]);
      setQueue({
        status: "ready",
        sessions: sessions.map(adaptSession),
        candidates: candidates.map(adaptCandidate),
        jobs: jobs.map(adaptJob),
      });
    } catch (error) {
      setQueue({ status: "error", message: errorMessage(error) });
    }
  }, []);

  useEffect(() => {
    let active = true;
    refreshQueue().then(() => {
      if (!active) return;
    });
    return () => {
      active = false;
    };
  }, [refreshQueue]);

  const handleExtractCorpus = useCallback(
    async (sessionId: string) => {
      try {
        await extractDistillCorpus({ sessionId });
      } finally {
        await refreshQueue();
      }
    },
    [refreshQueue],
  );

  const handlePromote = useCallback(
    async (loraId: string) => {
      try {
        await promoteDistillCandidate({
          loraId,
          operatorSignature: OPERATOR_SIGNATURE,
        });
      } finally {
        await refreshQueue();
      }
    },
    [refreshQueue],
  );

  const handleReject = useCallback(
    async (loraId: string, reason: string) => {
      try {
        await rejectDistillCandidate({
          loraId,
          operatorSignature: OPERATOR_SIGNATURE,
          reason,
        });
      } finally {
        await refreshQueue();
      }
    },
    [refreshQueue],
  );

  return (
    <section
      className="inference-lab"
      data-testid="inference-lab"
      aria-labelledby="inference-lab-title"
    >
      <header className="inference-lab__header">
        <h2 id="inference-lab-title">Inference Lab</h2>
        <p className="muted">
          Per-model toggles for the eight production inference techniques. Unsupported
          techniques are hidden, not greyed (Master Spec 10.14.1).
        </p>
      </header>

      <div className="inference-lab__model-picker">
        <label>
          <span>Loaded model</span>
          {models.status === "loading" ? (
            <span data-testid="inference-lab.models.loading">Loading...</span>
          ) : models.status === "error" ? (
            <span role="alert" data-testid="inference-lab.models.error">
              {models.message}
            </span>
          ) : models.models.length === 0 ? (
            <span data-testid="inference-lab.models.empty">No models currently loaded.</span>
          ) : (
            <select
              value={selectedModelId ?? ""}
              onChange={(event) => setSelectedModelId(event.target.value || null)}
              data-testid="inference-lab.models.select"
            >
              {models.models.map((model) => (
                <option key={model.modelId} value={model.modelId}>
                  {model.modelId} ({model.runtimeBinding})
                </option>
              ))}
            </select>
          )}
        </label>
      </div>

      {selectedModelId === null ? null : caps.status === "loading" ? (
        <p data-testid="inference-lab.capabilities.loading">Probing model capabilities...</p>
      ) : caps.status === "error" ? (
        <p role="alert" data-testid="inference-lab.capabilities.error">
          Capability probe failed: {caps.message}
        </p>
      ) : caps.status === "ready" ? (
        <div className="inference-lab__panels">
          <LoraStackComposer
            modelId={selectedModelId}
            capabilities={caps.capabilities}
          />
          <KvCachePanel
            modelId={selectedModelId}
            capabilities={caps.capabilities}
          />
          <SpeculativeDecodingPanel
            modelId={selectedModelId}
            capabilities={caps.capabilities}
          />
          <SteeringVectorEditor
            modelId={selectedModelId}
            capabilities={caps.capabilities}
            nLayers={DEFAULT_LAYER_COUNT}
          />
          <RefusalVectorWizard
            modelId={selectedModelId}
            capabilities={caps.capabilities}
            nLayers={DEFAULT_LAYER_COUNT}
          />
          <CaaWizard
            modelId={selectedModelId}
            capabilities={caps.capabilities}
            nLayers={DEFAULT_LAYER_COUNT}
          />
          {queue.status === "loading" ? (
            <p data-testid="inference-lab.distillation-queue.loading">
              Loading distillation queue...
            </p>
          ) : queue.status === "error" ? (
            <p
              role="alert"
              data-testid="inference-lab.distillation-queue.error"
            >
              Distillation queue load failed: {queue.message}
            </p>
          ) : (
            <DistillationQueue
              optedInSessions={queue.sessions}
              pendingCandidates={queue.candidates}
              trainingJobs={queue.jobs}
              onExtractCorpus={handleExtractCorpus}
              onPromote={handlePromote}
              onReject={handleReject}
            />
          )}
          {!caps.capabilities.supportsActivationSteering ? (
            <p
              className="muted"
              data-testid="inference-lab.steering.unsupported"
            >
              Activation steering is not exposed by this model's adapter; the editor is
              hidden per spec.
            </p>
          ) : null}
        </div>
      ) : null}
    </section>
  );
}
