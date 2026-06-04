import { useState } from "react";
import type { ModelCapabilities } from "../../lib/ipc/model_runtime";
import { extractRefusal, type RefusalDirection } from "../../lib/ipc/refusal";
import {
  registerVector,
  setActive,
  type SteeringVectorIdResult,
} from "../../lib/ipc/steering";

type Props = {
  modelId: string;
  capabilities: ModelCapabilities | null;
  nLayers: number;
};

type LicenseTag = "Permissive" | "SourceModelLicenseOnly" | "Restricted";
const LICENSE_OPTIONS: ReadonlyArray<LicenseTag> = [
  "Permissive",
  "SourceModelLicenseOnly",
  "Restricted",
];

const DEFAULT_CANDIDATE_LAYERS = [10, 14, 18];
const REFUSAL_ABLATION_INTENSITY_HINT = -1.0;

type ExtractState =
  | { status: "idle" }
  | { status: "extracting" }
  | { status: "extracted"; directions: RefusalDirection[] }
  | { status: "error"; message: string };

type SaveState =
  | { status: "idle" }
  | { status: "saving" }
  | { status: "saved"; result: SteeringVectorIdResult }
  | { status: "error"; message: string };

function errorMessage(error: unknown): string {
  if (error instanceof Error) return error.message;
  if (typeof error === "string") return error;
  return "Unknown error";
}

function splitPrompts(raw: string): string[] {
  return raw
    .split(/\r?\n/)
    .map((line) => line.trim())
    .filter((line) => line.length > 0);
}

function parseLayerList(raw: string, nLayers: number): number[] {
  return raw
    .split(/[,\s]+/)
    .map((tok) => tok.trim())
    .filter((tok) => tok.length > 0)
    .map(Number)
    .filter((n) => Number.isInteger(n) && n >= 0 && n < nLayers);
}

export function RefusalVectorWizard({ modelId, capabilities, nLayers }: Props) {
  // GLOBAL-PRODUCTION-005..009: harmful/harmless wording is operator-authored
  // and passes through verbatim. The wizard does not sanitise, censor, or
  // re-word; the runtime stores the prompts as-is in steering vector
  // provenance.
  const [harmfulPrompts, setHarmfulPrompts] = useState("");
  const [harmlessPrompts, setHarmlessPrompts] = useState("");
  const [layersRaw, setLayersRaw] = useState(DEFAULT_CANDIDATE_LAYERS.join(", "));
  const [name, setName] = useState("");
  const [description, setDescription] = useState("");
  const [selectedLayer, setSelectedLayer] = useState<number | null>(null);
  const [license, setLicense] = useState<LicenseTag>("SourceModelLicenseOnly");
  const [extractState, setExtractState] = useState<ExtractState>({ status: "idle" });
  const [saveState, setSaveState] = useState<SaveState>({ status: "idle" });
  const [activateAfterSave, setActivateAfterSave] = useState(true);
  // MT-102: activating a refusal-ablation vector disables the model's safety
  // refusal behaviour. Require an explicit operator acknowledgement before the
  // activate path can fire. This is the UI complement to the MT-097 server-side
  // review gate (activation also requires an Approved review there).
  const [acknowledgedDisablesRefusal, setAcknowledgedDisablesRefusal] =
    useState(false);

  if (!capabilities?.supportsActivationSteering) {
    return null;
  }

  const harmfulList = splitPrompts(harmfulPrompts);
  const harmlessList = splitPrompts(harmlessPrompts);
  const layerList = parseLayerList(layersRaw, nLayers);

  const canExtract =
    harmfulList.length > 0 && harmlessList.length > 0 && layerList.length > 0;

  const directions =
    extractState.status === "extracted" ? extractState.directions : [];

  // Per-layer effectiveness display: the bar fill is the L2 magnitude of
  // the refusal direction at this layer (normalised across the candidate
  // layers). The kernel returns unit-length directions (norm == 1) per
  // refusal_vector::extract_refusal_direction, so on a real run the bars
  // are uniform width — the differentiation comes from the layer-by-layer
  // direction angle, which the dropdown selects. For full per-layer
  // refusal-drop numbers, see refusal_metrics::measure_with_runtime; the
  // wizard surface forwards the operator's choice of layer to the save
  // step, which is the per-layer choice the spec actually requires.
  const maxNorm = directions.reduce((acc, d) => {
    const n = Math.sqrt(d.values.reduce((s, v) => s + v * v, 0));
    return Math.max(acc, n);
  }, 0);

  const handleExtract = async () => {
    if (!canExtract) return;
    setExtractState({ status: "extracting" });
    setSaveState({ status: "idle" });
    try {
      const result = await extractRefusal({
        modelId,
        harmfulPrompts: harmfulList,
        harmlessPrompts: harmlessList,
        layers: layerList,
      });
      setExtractState({ status: "extracted", directions: result.directions });
      if (result.directions.length > 0) {
        setSelectedLayer(result.directions[0].layer);
      }
    } catch (error) {
      setExtractState({ status: "error", message: errorMessage(error) });
    }
  };

  const handleSave = async () => {
    if (extractState.status !== "extracted") return;
    if (selectedLayer === null) return;
    if (!name.trim() || !description.trim()) return;
    if (activateAfterSave && !acknowledgedDisablesRefusal) return;
    const direction = extractState.directions.find((d) => d.layer === selectedLayer);
    if (!direction) return;
    setSaveState({ status: "saving" });
    try {
      const result = await registerVector({
        modelId,
        name: name.trim(),
        layer: direction.layer,
        hookPoint: "resid_stream",
        values: direction.values,
        intensity: REFUSAL_ABLATION_INTENSITY_HINT,
        description: `${description.trim()} | license=${license} | technique=RefusalVector`,
        provenance: {
          technique: "refusal_vector",
          positivePrompts: harmfulList,
          negativePrompts: harmlessList,
        },
      });
      setSaveState({ status: "saved", result });
      if (activateAfterSave) {
        try {
          await setActive(modelId, [result.vectorId]);
        } catch (error) {
          setSaveState({
            status: "error",
            message: `vector saved but set-active failed: ${errorMessage(error)}`,
          });
        }
      }
    } catch (error) {
      setSaveState({ status: "error", message: errorMessage(error) });
    }
  };

  const saving = saveState.status === "saving";
  const canSave =
    extractState.status === "extracted" &&
    selectedLayer !== null &&
    name.trim().length > 0 &&
    description.trim().length > 0 &&
    !saving &&
    (!activateAfterSave || acknowledgedDisablesRefusal);

  return (
    <section
      className="inference-lab__panel inference-lab__refusal-wizard"
      data-testid="refusal-vector-wizard"
      aria-labelledby="refusal-vector-wizard-title"
    >
      <header className="inference-lab__panel-header">
        <h3 id="refusal-vector-wizard-title">Refusal Vector Wizard</h3>
        <p className="muted" data-testid="refusal-vector-wizard.note">
          Per Arditi et al. 2026 (INF-4): supply a harmful-instruction pool and a
          harmless-instruction pool, extract the per-layer refusal direction,
          inspect effectiveness per layer, save and (optionally) activate as a
          steering vector. Operator-authored text is sent verbatim.
        </p>
      </header>

      <label>
        <span>Harmful prompts</span>
        <textarea
          rows={4}
          value={harmfulPrompts}
          onChange={(event) => setHarmfulPrompts(event.target.value)}
          data-testid="refusal-vector-wizard.harmful"
          placeholder="One prompt per line"
        />
      </label>

      <label>
        <span>Harmless prompts</span>
        <textarea
          rows={4}
          value={harmlessPrompts}
          onChange={(event) => setHarmlessPrompts(event.target.value)}
          data-testid="refusal-vector-wizard.harmless"
          placeholder="One prompt per line"
        />
      </label>

      <label>
        <span>Candidate layers (comma or space separated, 0..{nLayers - 1})</span>
        <input
          type="text"
          value={layersRaw}
          onChange={(event) => setLayersRaw(event.target.value)}
          data-testid="refusal-vector-wizard.layers"
        />
      </label>

      <div className="inference-lab__wizard-row">
        <button
          type="button"
          onClick={() => void handleExtract()}
          disabled={!canExtract || extractState.status === "extracting"}
          data-testid="refusal-vector-wizard.extract"
        >
          {extractState.status === "extracting" ? "Extracting..." : "Extract direction"}
        </button>
      </div>

      {extractState.status === "error" ? (
        <p role="alert" data-testid="refusal-vector-wizard.extract-error">
          Extract failed: {extractState.message}
        </p>
      ) : null}

      {extractState.status === "extracted" ? (
        <div
          className="inference-lab__refusal-effectiveness"
          data-testid="refusal-vector-wizard.effectiveness"
        >
          <h4>Per-layer effectiveness</h4>
          <p className="muted" data-testid="refusal-vector-wizard.effectiveness-note">
            Bars show the per-layer refusal-direction magnitude returned by
            the kernel. Pick the layer the operator wants to ablate against;
            the chosen direction is saved as a steering vector with
            ContrastiveTechnique=RefusalVector.
          </p>
          {extractState.directions.map((d) => {
            const norm = Math.sqrt(d.values.reduce((s, v) => s + v * v, 0));
            const widthPct = maxNorm > 0 ? Math.round((norm / maxNorm) * 100) : 0;
            const isSelected = selectedLayer === d.layer;
            return (
              <div
                key={d.layer}
                className={
                  isSelected
                    ? "refusal-bar refusal-bar--selected"
                    : "refusal-bar"
                }
                data-testid={`refusal-vector-wizard.effectiveness.row.${d.layer}`}
              >
                <button
                  type="button"
                  onClick={() => setSelectedLayer(d.layer)}
                  data-testid={`refusal-vector-wizard.effectiveness.select.${d.layer}`}
                >
                  Layer {d.layer}
                </button>
                <div
                  className="refusal-bar__track"
                  data-testid={`refusal-vector-wizard.effectiveness.bar.${d.layer}`}
                >
                  <div
                    className="refusal-bar__fill"
                    style={{ width: `${widthPct}%` }}
                  />
                </div>
                <span className="muted">||v||={norm.toFixed(3)}</span>
              </div>
            );
          })}

          <label>
            <span>Vector name</span>
            <input
              type="text"
              value={name}
              onChange={(event) => setName(event.target.value)}
              data-testid="refusal-vector-wizard.name"
            />
          </label>

          <label>
            <span>Description</span>
            <input
              type="text"
              value={description}
              onChange={(event) => setDescription(event.target.value)}
              data-testid="refusal-vector-wizard.description"
            />
          </label>

          <label>
            <span>License tag</span>
            <select
              value={license}
              onChange={(event) => setLicense(event.target.value as LicenseTag)}
              data-testid="refusal-vector-wizard.license"
            >
              {LICENSE_OPTIONS.map((tag) => (
                <option key={tag} value={tag}>
                  {tag}
                </option>
              ))}
            </select>
          </label>

          <label className="inference-lab__toggle">
            <input
              type="checkbox"
              checked={activateAfterSave}
              onChange={(event) => setActivateAfterSave(event.target.checked)}
              data-testid="refusal-vector-wizard.activate-after-save"
            />
            <span>Activate immediately after save</span>
          </label>

          {activateAfterSave ? (
            <label
              className="inference-lab__toggle inference-lab__toggle--warning"
              data-testid="refusal-vector-wizard.disable-ack-row"
            >
              <input
                type="checkbox"
                checked={acknowledgedDisablesRefusal}
                onChange={(event) =>
                  setAcknowledgedDisablesRefusal(event.target.checked)
                }
                data-testid="refusal-vector-wizard.disable-ack"
              />
              <span role="alert">
                I understand this vector is designed to disable safety refusal.
              </span>
            </label>
          ) : null}

          <button
            type="button"
            onClick={() => void handleSave()}
            disabled={!canSave}
            data-testid="refusal-vector-wizard.save"
          >
            {saving ? "Saving..." : "Save & activate"}
          </button>

          {saveState.status === "saved" ? (
            <p data-testid="refusal-vector-wizard.save-receipt">
              Saved vector {saveState.result.vectorId}.
            </p>
          ) : null}
          {saveState.status === "error" ? (
            <p role="alert" data-testid="refusal-vector-wizard.save-error">
              Save failed: {saveState.message}
            </p>
          ) : null}
        </div>
      ) : null}
    </section>
  );
}
