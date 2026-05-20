import { useState } from "react";
import {
  capture,
  registerVector,
  type LayerActivations,
} from "../../lib/ipc/steering";

type Props = {
  modelId: string;
  nLayers: number;
  onVectorSaved: () => void;
};

type LicenseTag = "Permissive" | "SourceModelLicenseOnly" | "Restricted";

const LICENSE_OPTIONS: ReadonlyArray<LicenseTag> = [
  "Permissive",
  "SourceModelLicenseOnly",
  "Restricted",
];

type CaptureState =
  | { status: "idle" }
  | { status: "capturing" }
  | {
      status: "captured";
      diffVector: number[];
      positivePrompts: string[];
      negativePrompts: string[];
      layer: number;
    }
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

function meanByLayer(activations: LayerActivations[], layer: number): number[] | null {
  const layerData = activations.find((entry) => entry.layer === layer);
  if (!layerData || layerData.activations.length === 0) return null;
  const dims = layerData.activations[0].length;
  const accumulator = new Array<number>(dims).fill(0);
  for (const row of layerData.activations) {
    if (row.length !== dims) return null;
    for (let i = 0; i < dims; i += 1) {
      accumulator[i] += row[i];
    }
  }
  return accumulator.map((sum) => sum / layerData.activations.length);
}

function diffVectors(a: number[], b: number[]): number[] | null {
  if (a.length !== b.length) return null;
  return a.map((value, idx) => value - b[idx]);
}

export function ContrastiveCaptureWizard({ modelId, nLayers, onVectorSaved }: Props) {
  // GLOBAL-PRODUCTION-005..009: prompts are operator-authored raw text. The wizard
  // sends them through to the kernel verbatim; no UI-level filtering, censoring,
  // moralizing, or rewording.
  const [positivePrompts, setPositivePrompts] = useState("");
  const [negativePrompts, setNegativePrompts] = useState("");
  const [layer, setLayer] = useState<number>(Math.min(12, Math.max(0, nLayers - 1)));
  const [vectorName, setVectorName] = useState("");
  const [description, setDescription] = useState("");
  const [intensity, setIntensity] = useState<number>(1.0);
  const [license, setLicense] = useState<LicenseTag>("SourceModelLicenseOnly");
  const [captureState, setCaptureState] = useState<CaptureState>({ status: "idle" });
  const [saveError, setSaveError] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);

  const positiveList = splitPrompts(positivePrompts);
  const negativeList = splitPrompts(negativePrompts);

  const canCapture =
    positiveList.length > 0 &&
    negativeList.length > 0 &&
    layer >= 0 &&
    layer < nLayers;

  const canSave =
    captureState.status === "captured" &&
    vectorName.trim().length > 0 &&
    description.trim().length > 0;

  const handleCapture = async () => {
    if (!canCapture) return;
    setCaptureState({ status: "capturing" });
    try {
      const positive = await capture({ modelId, prompts: positiveList, layers: [layer] });
      const negative = await capture({ modelId, prompts: negativeList, layers: [layer] });
      const positiveMean = meanByLayer(positive.activationsByLayer, layer);
      const negativeMean = meanByLayer(negative.activationsByLayer, layer);
      if (!positiveMean || !negativeMean) {
        setCaptureState({
          status: "error",
          message: "Capture returned no activations for the selected layer.",
        });
        return;
      }
      const diff = diffVectors(positiveMean, negativeMean);
      if (!diff) {
        setCaptureState({
          status: "error",
          message: "Positive and negative captures returned incompatible activation shapes.",
        });
        return;
      }
      setCaptureState({
        status: "captured",
        diffVector: diff,
        positivePrompts: positiveList,
        negativePrompts: negativeList,
        layer,
      });
    } catch (error) {
      setCaptureState({ status: "error", message: errorMessage(error) });
    }
  };

  const handleSave = async () => {
    if (captureState.status !== "captured") return;
    if (!canSave) return;
    setSaving(true);
    setSaveError(null);
    try {
      await registerVector({
        modelId,
        name: vectorName.trim(),
        layer: captureState.layer,
        hookPoint: "resid_stream",
        values: captureState.diffVector,
        intensity,
        description: description.trim(),
        provenance: {
          technique: "repe",
          positivePrompts: captureState.positivePrompts,
          negativePrompts: captureState.negativePrompts,
        },
        licenseTag: license,
      });
      setVectorName("");
      setDescription("");
      setCaptureState({ status: "idle" });
      onVectorSaved();
    } catch (error) {
      setSaveError(errorMessage(error));
    } finally {
      setSaving(false);
    }
  };

  return (
    <section
      className="inference-lab__wizard"
      data-testid="contrastive-capture-wizard"
      aria-labelledby="contrastive-capture-wizard-title"
    >
      <h4 id="contrastive-capture-wizard-title">Capture vector from contrastive prompts</h4>
      <p className="muted" data-testid="contrastive-capture-wizard.note">
        Enter positive prompts (one per line) and negative prompts (one per line). The wizard
        captures activations for both sets, computes the layer-mean difference, and persists it
        as a steering vector with RepE provenance. Operator-authored text is sent verbatim.
      </p>

      <label>
        <span>Positive prompts</span>
        <textarea
          rows={4}
          value={positivePrompts}
          onChange={(event) => setPositivePrompts(event.target.value)}
          data-testid="contrastive-capture-wizard.positive"
          placeholder="One prompt per line"
        />
      </label>

      <label>
        <span>Negative prompts</span>
        <textarea
          rows={4}
          value={negativePrompts}
          onChange={(event) => setNegativePrompts(event.target.value)}
          data-testid="contrastive-capture-wizard.negative"
          placeholder="One prompt per line"
        />
      </label>

      <div className="inference-lab__wizard-row">
        <label>
          <span>Layer (0..{nLayers - 1})</span>
          <select
            value={layer}
            onChange={(event) => setLayer(Number(event.target.value))}
            data-testid="contrastive-capture-wizard.layer"
          >
            {Array.from({ length: nLayers }, (_, idx) => idx).map((i) => (
              <option key={i} value={i}>
                {i}
              </option>
            ))}
          </select>
        </label>

        <button
          type="button"
          onClick={() => void handleCapture()}
          disabled={!canCapture || captureState.status === "capturing"}
          data-testid="contrastive-capture-wizard.capture"
        >
          {captureState.status === "capturing" ? "Capturing..." : "Capture activations"}
        </button>
      </div>

      {captureState.status === "error" ? (
        <p role="alert" data-testid="contrastive-capture-wizard.capture-error">
          Capture failed: {captureState.message}
        </p>
      ) : null}

      {captureState.status === "captured" ? (
        <div className="inference-lab__wizard-save" data-testid="contrastive-capture-wizard.save-form">
          <p data-testid="contrastive-capture-wizard.capture-summary">
            Captured difference vector at layer {captureState.layer} (dim={captureState.diffVector.length}).
          </p>

          <label>
            <span>Vector name</span>
            <input
              type="text"
              value={vectorName}
              onChange={(event) => setVectorName(event.target.value)}
              data-testid="contrastive-capture-wizard.name"
            />
          </label>

          <label>
            <span>Description</span>
            <input
              type="text"
              value={description}
              onChange={(event) => setDescription(event.target.value)}
              data-testid="contrastive-capture-wizard.description"
            />
          </label>

          <label>
            <span>Intensity (-10..10)</span>
            <input
              type="number"
              min={-10}
              max={10}
              step={0.1}
              value={intensity}
              onChange={(event) => setIntensity(Number(event.target.value))}
              data-testid="contrastive-capture-wizard.intensity"
            />
          </label>

          <label>
            <span>License tag</span>
            <select
              value={license}
              onChange={(event) => setLicense(event.target.value as LicenseTag)}
              data-testid="contrastive-capture-wizard.license"
            >
              {LICENSE_OPTIONS.map((tag) => (
                <option key={tag} value={tag}>
                  {tag}
                </option>
              ))}
            </select>
          </label>

          <button
            type="button"
            onClick={() => void handleSave()}
            disabled={!canSave || saving}
            data-testid="contrastive-capture-wizard.save"
          >
            {saving ? "Saving..." : "Save vector"}
          </button>

          {saveError ? (
            <p role="alert" data-testid="contrastive-capture-wizard.save-error">
              Save failed: {saveError}
            </p>
          ) : null}
        </div>
      ) : null}

      <details className="inference-lab__ab-compare" data-testid="contrastive-capture-wizard.ab-compare">
        <summary>A/B compare (live generation)</summary>
        <p className="muted">
          Side-by-side generation with the proposed vector active vs inactive runs through the
          live CandleRuntime adapter (MT-082). If the runtime is not attached to this model in
          the current session, capture returns a typed capture_not_available reason which the
          wizard surfaces verbatim above.
        </p>
      </details>
    </section>
  );
}
