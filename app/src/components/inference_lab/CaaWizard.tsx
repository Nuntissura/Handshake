import { useState } from "react";
import type { ModelCapabilities } from "../../lib/ipc/model_runtime";
import { extractCaa, type CaaExtractResult, type CaaPromptPair } from "../../lib/ipc/caa";
import { setActive } from "../../lib/ipc/steering";

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

const DEFAULT_LAYER = 14;

type ExtractState =
  | { status: "idle" }
  | { status: "extracting" }
  | { status: "extracted"; result: CaaExtractResult }
  | { status: "error"; message: string };

function errorMessage(error: unknown): string {
  if (error instanceof Error) return error.message;
  if (typeof error === "string") return error;
  return "Unknown error";
}

function blankPair(): CaaPromptPair {
  return { context: "", positive: "", negative: "" };
}

export function CaaWizard({ modelId, capabilities, nLayers }: Props) {
  if (!capabilities?.supportsActivationSteering) {
    return null;
  }

  // GLOBAL-PRODUCTION-005..009: paired prompt cells accept verbatim
  // operator wording. The wizard does not filter, censor, or moralise.
  const [pairs, setPairs] = useState<CaaPromptPair[]>([blankPair(), blankPair(), blankPair()]);
  const [name, setName] = useState("");
  const [description, setDescription] = useState("");
  const [layer, setLayer] = useState<number>(Math.min(DEFAULT_LAYER, Math.max(0, nLayers - 1)));
  const [license, setLicense] = useState<LicenseTag>("SourceModelLicenseOnly");
  const [activateAfterSave, setActivateAfterSave] = useState(true);
  const [extractState, setExtractState] = useState<ExtractState>({ status: "idle" });
  const [activateError, setActivateError] = useState<string | null>(null);

  const validPairs = pairs.filter(
    (p) => p.positive.trim().length > 0 && p.negative.trim().length > 0,
  );

  const canExtract =
    validPairs.length > 0 &&
    name.trim().length > 0 &&
    description.trim().length > 0 &&
    layer >= 0 &&
    layer < nLayers;

  const updatePair = (index: number, field: keyof CaaPromptPair, value: string) => {
    setPairs((prev) =>
      prev.map((pair, idx) => (idx === index ? { ...pair, [field]: value } : pair)),
    );
  };

  const addRow = () => setPairs((prev) => [...prev, blankPair()]);
  const removeRow = (index: number) =>
    setPairs((prev) =>
      prev.length > 1 ? prev.filter((_, idx) => idx !== index) : prev,
    );

  const importJson = (raw: string) => {
    try {
      const parsed = JSON.parse(raw) as unknown;
      if (!Array.isArray(parsed)) throw new Error("Imported JSON must be an array of pairs");
      const next: CaaPromptPair[] = parsed.map((item, idx) => {
        if (typeof item !== "object" || item === null) {
          throw new Error(`Pair ${idx} is not an object`);
        }
        const obj = item as Partial<CaaPromptPair>;
        return {
          context: typeof obj.context === "string" ? obj.context : "",
          positive: typeof obj.positive === "string" ? obj.positive : "",
          negative: typeof obj.negative === "string" ? obj.negative : "",
        };
      });
      if (next.length > 0) setPairs(next);
    } catch (error) {
      setExtractState({ status: "error", message: errorMessage(error) });
    }
  };

  const handleExtract = async () => {
    if (!canExtract) return;
    setExtractState({ status: "extracting" });
    setActivateError(null);
    try {
      const result = await extractCaa({
        modelId,
        name: name.trim(),
        description: `${description.trim()} | license=${license} | technique=CAA`,
        pairs: validPairs.map((p) => ({
          context: p.context,
          positive: p.positive,
          negative: p.negative,
        })),
        layer,
      });
      setExtractState({ status: "extracted", result });
      if (activateAfterSave) {
        try {
          await setActive(modelId, [result.vectorId]);
        } catch (error) {
          setActivateError(errorMessage(error));
        }
      }
    } catch (error) {
      setExtractState({ status: "error", message: errorMessage(error) });
    }
  };

  return (
    <section
      className="inference-lab__panel inference-lab__caa-wizard"
      data-testid="caa-wizard"
      aria-labelledby="caa-wizard-title"
    >
      <header className="inference-lab__panel-header">
        <h3 id="caa-wizard-title">CAA Wizard (Contrastive Activation Addition)</h3>
        <p className="muted" data-testid="caa-wizard.note">
          Per Rimsky 2024 INF-5: build paired prompts that share context and
          differ in completion direction. Activation difference at the
          completion-token position becomes the steering vector. Operator-
          authored text is sent verbatim.
        </p>
      </header>

      <table data-testid="caa-wizard.table">
        <thead>
          <tr>
            <th>Context</th>
            <th>Positive completion</th>
            <th>Negative completion</th>
            <th></th>
          </tr>
        </thead>
        <tbody>
          {pairs.map((pair, idx) => (
            <tr key={idx} data-testid={`caa-wizard.row.${idx}`}>
              <td>
                <textarea
                  rows={2}
                  value={pair.context}
                  onChange={(event) => updatePair(idx, "context", event.target.value)}
                  data-testid={`caa-wizard.row.${idx}.context`}
                />
              </td>
              <td>
                <textarea
                  rows={2}
                  value={pair.positive}
                  onChange={(event) => updatePair(idx, "positive", event.target.value)}
                  data-testid={`caa-wizard.row.${idx}.positive`}
                />
              </td>
              <td>
                <textarea
                  rows={2}
                  value={pair.negative}
                  onChange={(event) => updatePair(idx, "negative", event.target.value)}
                  data-testid={`caa-wizard.row.${idx}.negative`}
                />
              </td>
              <td>
                <button
                  type="button"
                  onClick={() => removeRow(idx)}
                  data-testid={`caa-wizard.row.${idx}.remove`}
                  disabled={pairs.length === 1}
                >
                  Remove
                </button>
              </td>
            </tr>
          ))}
        </tbody>
      </table>

      <div className="inference-lab__wizard-row">
        <button type="button" onClick={addRow} data-testid="caa-wizard.add-row">
          Add row
        </button>

        <label>
          <span>Import from JSON</span>
          <input
            type="text"
            placeholder='[{"context":"...","positive":"...","negative":"..."}]'
            onBlur={(event) => {
              if (event.target.value.trim().length > 0) {
                importJson(event.target.value);
                event.target.value = "";
              }
            }}
            data-testid="caa-wizard.import"
          />
        </label>

        <label>
          <span>Layer</span>
          <select
            value={layer}
            onChange={(event) => setLayer(Number(event.target.value))}
            data-testid="caa-wizard.layer"
          >
            {Array.from({ length: nLayers }, (_, i) => i).map((i) => (
              <option key={i} value={i}>
                {i}
              </option>
            ))}
          </select>
        </label>
      </div>

      <label>
        <span>Vector name</span>
        <input
          type="text"
          value={name}
          onChange={(event) => setName(event.target.value)}
          data-testid="caa-wizard.name"
        />
      </label>

      <label>
        <span>Description</span>
        <input
          type="text"
          value={description}
          onChange={(event) => setDescription(event.target.value)}
          data-testid="caa-wizard.description"
        />
      </label>

      <label>
        <span>License tag</span>
        <select
          value={license}
          onChange={(event) => setLicense(event.target.value as LicenseTag)}
          data-testid="caa-wizard.license"
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
          data-testid="caa-wizard.activate-after-save"
        />
        <span>Activate immediately after save</span>
      </label>

      <button
        type="button"
        onClick={() => void handleExtract()}
        disabled={!canExtract || extractState.status === "extracting"}
        data-testid="caa-wizard.extract"
      >
        {extractState.status === "extracting" ? "Extracting..." : "Extract & save CAA vector"}
      </button>

      {extractState.status === "error" ? (
        <p role="alert" data-testid="caa-wizard.extract-error">
          Extract failed: {extractState.message}
        </p>
      ) : null}
      {extractState.status === "extracted" ? (
        <p data-testid="caa-wizard.extract-result">
          Vector {extractState.result.vectorId} saved at layer {extractState.result.layer}.
        </p>
      ) : null}
      {activateError ? (
        <p role="alert" data-testid="caa-wizard.activate-error">
          Activate failed: {activateError}
        </p>
      ) : null}
    </section>
  );
}
