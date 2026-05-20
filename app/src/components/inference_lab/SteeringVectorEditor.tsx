import { useCallback, useEffect, useState } from "react";
import type { ModelCapabilities } from "../../lib/ipc/model_runtime";
import {
  listVectors,
  setActive,
  unregister,
  type SteeringVectorMeta,
} from "../../lib/ipc/steering";
import { ContrastiveCaptureWizard } from "./ContrastiveCaptureWizard";

type Props = {
  modelId: string;
  capabilities: ModelCapabilities | null;
  nLayers: number;
};

type LoadState =
  | { status: "loading" }
  | { status: "error"; message: string }
  | { status: "ready"; vectors: SteeringVectorMeta[]; activeIds: Set<string> };

function errorMessage(error: unknown): string {
  if (error instanceof Error) return error.message;
  if (typeof error === "string") return error;
  return "Unknown error";
}

export function SteeringVectorEditor({ modelId, capabilities, nLayers }: Props) {
  const [state, setState] = useState<LoadState>({ status: "loading" });
  const [reloadKey, setReloadKey] = useState(0);

  const reload = useCallback(() => {
    setReloadKey((k) => k + 1);
  }, []);

  useEffect(() => {
    if (!capabilities?.supportsActivationSteering) {
      return;
    }
    let active = true;
    setState({ status: "loading" });

    listVectors(modelId)
      .then((vectors) => {
        if (active) {
          setState({ status: "ready", vectors, activeIds: new Set() });
        }
      })
      .catch((error) => {
        if (active) {
          setState({ status: "error", message: errorMessage(error) });
        }
      });

    return () => {
      active = false;
    };
  }, [modelId, capabilities, reloadKey]);

  if (!capabilities?.supportsActivationSteering) {
    return null;
  }

  const handleToggle = async (vectorId: string, nextActive: boolean) => {
    if (state.status !== "ready") return;
    const nextActiveIds = new Set(state.activeIds);
    if (nextActive) {
      nextActiveIds.add(vectorId);
    } else {
      nextActiveIds.delete(vectorId);
    }
    try {
      const result = await setActive(modelId, Array.from(nextActiveIds));
      setState({ ...state, activeIds: new Set(result.activeIds) });
    } catch (error) {
      setState({ status: "error", message: errorMessage(error) });
    }
  };

  const handleUnregister = async (vectorId: string) => {
    try {
      await unregister(modelId, vectorId);
      reload();
    } catch (error) {
      setState({ status: "error", message: errorMessage(error) });
    }
  };

  return (
    <section
      className="inference-lab__panel inference-lab__steering"
      data-testid="steering-vector-editor"
      aria-labelledby="steering-vector-editor-title"
    >
      <header className="inference-lab__panel-header">
        <h3 id="steering-vector-editor-title">Activation Steering Vectors</h3>
        <p className="muted" data-testid="steering-vector-editor.note">
          Manual intensity slider clamped -10..10 per SteeringVector contract (MT-065). Capture
          and mutation calls dispatch to the live CandleRuntime adapter (MT-082). If no adapter is
          attached to this model in the current session, the kernel returns a typed
          capture_not_available reason which is surfaced verbatim.
        </p>
      </header>

      {state.status === "loading" ? (
        <p data-testid="steering-vector-editor.loading">Loading registered vectors...</p>
      ) : state.status === "error" ? (
        <p role="alert" data-testid="steering-vector-editor.error">
          Steering surface error: {state.message}
        </p>
      ) : state.vectors.length === 0 ? (
        <p data-testid="steering-vector-editor.empty">
          No steering vectors registered for this model. Use the capture wizard below to create one.
        </p>
      ) : (
        <table data-testid="steering-vector-editor.table">
          <thead>
            <tr>
              <th>Name</th>
              <th>Layer</th>
              <th>Hook</th>
              <th>Intensity</th>
              <th>Active</th>
              <th>Description</th>
              <th></th>
            </tr>
          </thead>
          <tbody>
            {state.vectors.map((vector) => {
              const isActive = state.activeIds.has(vector.vectorId);
              return (
                <tr
                  key={vector.vectorId}
                  data-vector-id={vector.vectorId}
                  data-testid={`steering-vector-editor.row.${vector.vectorId}`}
                >
                  <td>{vector.name}</td>
                  <td>{vector.layer}</td>
                  <td>
                    <code>{vector.hookPoint}</code>
                  </td>
                  <td>
                    <input
                      type="range"
                      min={-10}
                      max={10}
                      step={0.1}
                      defaultValue={vector.intensity}
                      data-testid={`steering-vector-editor.row.${vector.vectorId}.intensity`}
                      aria-label={`Intensity for ${vector.name}`}
                    />
                    <span className="muted">{vector.intensity.toFixed(2)}</span>
                  </td>
                  <td>
                    <label className="inference-lab__toggle">
                      <input
                        type="checkbox"
                        checked={isActive}
                        onChange={(event) => void handleToggle(vector.vectorId, event.target.checked)}
                        data-testid={`steering-vector-editor.row.${vector.vectorId}.active`}
                      />
                      <span>{isActive ? "on" : "off"}</span>
                    </label>
                  </td>
                  <td>{vector.description}</td>
                  <td>
                    <button
                      type="button"
                      onClick={() => void handleUnregister(vector.vectorId)}
                      data-testid={`steering-vector-editor.row.${vector.vectorId}.unregister`}
                    >
                      Remove
                    </button>
                  </td>
                </tr>
              );
            })}
          </tbody>
        </table>
      )}

      <ContrastiveCaptureWizard modelId={modelId} nLayers={nLayers} onVectorSaved={reload} />
    </section>
  );
}
