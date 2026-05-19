import { useEffect, useState } from "react";
import {
  capabilities as fetchCapabilities,
  listLoaded,
  type LoadedModelRuntime,
  type ModelCapabilities,
} from "../../lib/ipc/model_runtime";
import { CaaWizard } from "./CaaWizard";
import { RefusalVectorWizard } from "./RefusalVectorWizard";
import { SteeringVectorEditor } from "./SteeringVectorEditor";

// Default visible layer range for steering layer pickers when the kernel does not
// expose n_layers yet. Per spec 10.14.2 the picker should reflect the loaded
// model's layer count; this is a conservative interim ceiling so the dropdown
// still renders sensible options. Future MTs (LoRA / KV / Subquadratic panels)
// will replace this with kernel-supplied metadata.
const DEFAULT_LAYER_COUNT = 32;

type ModelsState =
  | { status: "loading" }
  | { status: "error"; message: string }
  | { status: "ready"; models: LoadedModelRuntime[] };

type CapState =
  | { status: "idle" }
  | { status: "loading" }
  | { status: "error"; message: string }
  | { status: "ready"; capabilities: ModelCapabilities };

function errorMessage(error: unknown): string {
  if (error instanceof Error) return error.message;
  if (typeof error === "string") return error;
  return "Unknown error";
}

export function InferenceLab() {
  const [models, setModels] = useState<ModelsState>({ status: "loading" });
  const [selectedModelId, setSelectedModelId] = useState<string | null>(null);
  const [caps, setCaps] = useState<CapState>({ status: "idle" });

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
