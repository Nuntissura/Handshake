import { useState } from "react";
import {
  generateAb,
  type SteeringAbComparison,
} from "../../lib/ipc/steering";

type Props = {
  modelId: string;
  /**
   * The proposed steering vector to contrast. Applied active on the AFTER side
   * and inactive on the BEFORE side. Must be a registered (saved) vector id so
   * the live runtime can resolve it via `steering_overrides`.
   */
  activeVectorId: string;
  /** Human label for the proposed vector, shown in the panel header. */
  vectorName?: string;
};

type AbState =
  | { status: "idle" }
  | { status: "generating" }
  | { status: "done"; comparisons: SteeringAbComparison[] }
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

/**
 * MT-098 AB-compare panel. Renders a side-by-side BEFORE/AFTER generation with
 * the proposed steering vector active vs inactive, by calling the live
 * `generateAb` IPC. The kernel runs the REAL candle generate through the live
 * runtime + steering path for both sides; this component only renders the two
 * completions it returns. Operator-authored prompt text is sent verbatim
 * (GLOBAL-PRODUCTION-005..009): no UI-level filtering or rewording.
 */
export function ABCompare({ modelId, activeVectorId, vectorName }: Props) {
  const [prompts, setPrompts] = useState("");
  const [maxTokens, setMaxTokens] = useState<number>(64);
  const [state, setState] = useState<AbState>({ status: "idle" });

  const promptList = splitPrompts(prompts);
  const canGenerate = promptList.length > 0 && activeVectorId.trim().length > 0;

  const handleGenerate = async () => {
    if (!canGenerate) return;
    setState({ status: "generating" });
    try {
      const result = await generateAb({
        modelId,
        prompts: promptList,
        activeVectorIds: [activeVectorId],
        inactiveVectorIds: [],
        maxTokens,
      });
      setState({ status: "done", comparisons: result.comparisons });
    } catch (error) {
      setState({ status: "error", message: errorMessage(error) });
    }
  };

  return (
    <section
      className="inference-lab__ab-compare"
      data-testid="ab-compare"
      aria-labelledby="ab-compare-title"
    >
      <h4 id="ab-compare-title">A/B compare (live generation)</h4>
      <p className="muted" data-testid="ab-compare.note">
        Side-by-side generation with the proposed vector
        {vectorName ? ` "${vectorName}"` : ""} active vs inactive. Each prompt is
        generated twice through the live CandleRuntime adapter — once steered,
        once baseline. Operator-authored text is sent verbatim.
      </p>

      <label>
        <span>Prompts (one per line)</span>
        <textarea
          rows={3}
          value={prompts}
          onChange={(event) => setPrompts(event.target.value)}
          data-testid="ab-compare.prompts"
          placeholder="One prompt per line"
        />
      </label>

      <div className="inference-lab__ab-compare-row">
        <label>
          <span>Max tokens</span>
          <input
            type="number"
            min={1}
            max={256}
            step={1}
            value={maxTokens}
            onChange={(event) => setMaxTokens(Number(event.target.value))}
            data-testid="ab-compare.max-tokens"
          />
        </label>

        <button
          type="button"
          onClick={() => void handleGenerate()}
          disabled={!canGenerate || state.status === "generating"}
          data-testid="ab-compare.generate"
        >
          {state.status === "generating" ? "Generating..." : "Generate A/B"}
        </button>
      </div>

      {state.status === "error" ? (
        <p role="alert" data-testid="ab-compare.error">
          A/B compare failed: {state.message}
        </p>
      ) : null}

      {state.status === "done" ? (
        <div data-testid="ab-compare.results">
          {state.comparisons.map((comparison, index) => (
            <div
              key={`${index}-${comparison.prompt}`}
              className="inference-lab__ab-compare-pair"
              data-testid={`ab-compare.pair.${index}`}
            >
              <p className="muted" data-testid={`ab-compare.pair.${index}.prompt`}>
                Prompt: {comparison.prompt}
              </p>
              <div className="inference-lab__ab-compare-grid">
                <div
                  className="inference-lab__ab-compare-pane"
                  data-testid={`ab-compare.pair.${index}.inactive`}
                >
                  <h5>Before (vector inactive)</h5>
                  <pre data-testid={`ab-compare.pair.${index}.inactive-text`}>
                    {comparison.inactiveCompletion}
                  </pre>
                </div>
                <div
                  className="inference-lab__ab-compare-pane"
                  data-testid={`ab-compare.pair.${index}.active`}
                >
                  <h5>After (vector active)</h5>
                  <pre data-testid={`ab-compare.pair.${index}.active-text`}>
                    {comparison.activeCompletion}
                  </pre>
                </div>
              </div>
            </div>
          ))}
        </div>
      ) : null}
    </section>
  );
}
