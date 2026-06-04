import { useCallback, useEffect, useState } from "react";
import type { ModelCapabilities } from "../../lib/ipc/model_runtime";
import {
  specGetMode,
  specModeOptions,
  specSetMode,
  type SpeculativeMode,
} from "../../lib/ipc/speculative";

// MT-110 owned-files contract listed Svelte sources. App stack is
// React/TSX (same Svelte->TSX defect class as MT-091/094/098/102/105/124).
// Behavior + AC + red_team controls preserved.
//
// AC-INFER-LAB-UI-TOGGLES: hidden (return null) when neither speculative
// nor Eagle3 is supported. Eagle3 is always visible-but-disabled with the
// deferral note when supportsEagle3=false (per operator E-4).
// MT-098 now owns live A/B compare through ABCompare + steering_generate_ab.
// This panel remains scoped to speculative decoding controls.

type Props = {
  modelId: string;
  capabilities: ModelCapabilities | null;
};

type PanelState =
  | { status: "loading" }
  | { status: "error"; message: string }
  | { status: "ready"; currentMode: SpeculativeMode | null };

function errorMessage(error: unknown): string {
  if (error instanceof Error) return error.message;
  if (typeof error === "string") return error;
  return "Unknown error";
}

export function SpeculativeDecodingPanel({ modelId, capabilities }: Props) {
  const supportsSpec = capabilities?.supportsSpeculativeDraft === true;
  const supportsEagle3 = capabilities?.supportsEagle3 === true;
  const panelHidden = !supportsSpec && !supportsEagle3;

  const [state, setState] = useState<PanelState>({ status: "loading" });
  const [pendingMode, setPendingMode] = useState<string | null>(null);

  useEffect(() => {
    if (panelHidden) {
      setState({ status: "ready", currentMode: null });
      return;
    }
    let active = true;
    setState({ status: "loading" });
    specGetMode({ modelId })
      .then((result) => {
        if (!active) return;
        setState({ status: "ready", currentMode: result.currentMode });
      })
      .catch((error) => {
        if (active) setState({ status: "error", message: errorMessage(error) });
      });
    return () => {
      active = false;
    };
  }, [modelId, panelHidden]);

  const handleModeChange = useCallback(
    async (kind: string) => {
      setPendingMode(kind);
      let mode: SpeculativeMode | null = null;
      if (kind === "ngram") {
        mode = { mode: "ngram", lookback: 32, maxDraft: 8 };
      } else if (kind === "draft_model") {
        // Draft-model picker UX (operator picks the draft modelId) is
        // a follow-up MT — the Tauri command will reject until the
        // operator binds a real draft modelId. We mark the mode as
        // pending without committing.
        setPendingMode(null);
        setState({
          status: "error",
          message:
            "DraftModel picker UX is a follow-up MT — operator must bind a draft model id first.",
        });
        return;
      } else if (kind === "eagle3_deferred") {
        // Eagle3 is always rejected by the backend until adapter
        // signals supports_eagle3=true. Don't even dispatch.
        setPendingMode(null);
        setState({
          status: "error",
          message:
            "Eagle3 is deferred (operator E-4) until llama.cpp PR #18039 merges.",
        });
        return;
      }
      try {
        const result = await specSetMode({ modelId, mode });
        setState({ status: "ready", currentMode: result.currentMode });
      } catch (error) {
        setState({ status: "error", message: errorMessage(error) });
      } finally {
        setPendingMode(null);
      }
    },
    [modelId],
  );

  if (panelHidden) return null;

  const options = specModeOptions(supportsSpec, supportsEagle3);
  const currentKind =
    state.status === "ready" && state.currentMode !== null
      ? state.currentMode.mode
      : "none";

  return (
    <section
      className="inference-lab__panel inference-lab__speculative"
      data-testid="speculative-decoding-panel"
      aria-labelledby="speculative-decoding-panel-title"
    >
      <header className="inference-lab__panel-header">
        <h3 id="speculative-decoding-panel-title">
          Self-Speculative Decoding
        </h3>
        <p className="muted" data-testid="speculative-decoding-panel.note">
          Operator commits the saved mode via Work Profile knob
          settings.exec_policy.speculative. Eagle3 is visible-but-deferred
          until the adapter signals supports_eagle3 (operator E-4, llama.cpp
          PR #18039).
        </p>
      </header>

      {state.status === "loading" ? (
        <p data-testid="speculative-decoding-panel.loading">
          Loading speculative mode…
        </p>
      ) : state.status === "error" ? (
        <p
          role="alert"
          className="inference-lab__error"
          data-testid="speculative-decoding-panel.error"
        >
          {state.message}
        </p>
      ) : (
        <div className="speculative-decoding-panel__body">
          <label className="speculative-decoding-panel__mode">
            <span>Mode</span>
            <select
              value={pendingMode ?? currentKind}
              onChange={(event) => void handleModeChange(event.target.value)}
              disabled={pendingMode !== null}
              data-testid="speculative-decoding-panel.mode-picker"
            >
              {options.map((opt) => {
                if (opt.kind === "eagle3_deferred") {
                  return (
                    <option key="eagle3_deferred" value="eagle3_deferred">
                      Eagle3 (deferred — PR #18039 pending)
                    </option>
                  );
                }
                return (
                  <option key={opt.kind} value={opt.kind}>
                    {opt.kind}
                  </option>
                );
              })}
            </select>
          </label>
          {/* Eagle3 deferral note — always rendered when the option is
              in the list, so the operator sees the rationale even when
              the picker is on a different mode. */}
          {options.some((o) => o.kind === "eagle3_deferred") ? (
            <p
              className="muted"
              data-testid="speculative-decoding-panel.eagle3-deferred-note"
            >
              Eagle3 is deferred until llama.cpp PR #18039 merges and the
              adapter signals supports_eagle3=true (operator decision E-4).
            </p>
          ) : null}
        </div>
      )}
    </section>
  );
}
