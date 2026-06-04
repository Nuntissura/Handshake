import { render, screen, waitFor, fireEvent } from "@testing-library/react";
import { describe, expect, it, vi, beforeEach } from "vitest";
import type { ModelCapabilities } from "../../lib/ipc/model_runtime";

const specGetModeMock = vi.hoisted(() => vi.fn());
const specSetModeMock = vi.hoisted(() => vi.fn());

vi.mock("../../lib/ipc/speculative", async () => {
  const actual = await vi.importActual<typeof import("../../lib/ipc/speculative")>(
    "../../lib/ipc/speculative",
  );
  return {
    ...actual,
    specGetMode: specGetModeMock,
    specSetMode: specSetModeMock,
  };
});

import { SpeculativeDecodingPanel } from "./SpeculativeDecodingPanel";
import { specModeOptions } from "../../lib/ipc/speculative";

const MODEL_ID = "019a1b2c-0000-7000-8000-aaaaaaaaaaaa";

function caps(overrides: Partial<ModelCapabilities> = {}): ModelCapabilities {
  return {
    supportsLora: false,
    supportsKvPrefixCache: false,
    supportsKvQuantization: "none",
    supportsActivationSteering: false,
    supportsSubquadratic: false,
    supportsSpeculativeDraft: true,
    supportsEagle3: false,
    ...overrides,
  };
}

describe("SpeculativeDecodingPanel", () => {
  beforeEach(() => {
    specGetModeMock.mockReset();
    specSetModeMock.mockReset();
  });

  it("renders nothing when neither speculative nor eagle3 is supported", () => {
    const { container } = render(
      <SpeculativeDecodingPanel
        modelId={MODEL_ID}
        capabilities={caps({ supportsSpeculativeDraft: false, supportsEagle3: false })}
      />,
    );
    expect(container.firstChild).toBeNull();
    expect(specGetModeMock).not.toHaveBeenCalled();
  });

  it("renders nothing when capabilities is null (not yet probed)", () => {
    const { container } = render(
      <SpeculativeDecodingPanel modelId={MODEL_ID} capabilities={null} />,
    );
    expect(container.firstChild).toBeNull();
  });

  it("renders the panel + Eagle3 deferred note when supportsSpeculativeDraft=true + supportsEagle3=false", async () => {
    specGetModeMock.mockResolvedValueOnce({ modelId: MODEL_ID, currentMode: null });
    render(
      <SpeculativeDecodingPanel modelId={MODEL_ID} capabilities={caps()} />,
    );
    await waitFor(() =>
      screen.getByTestId("speculative-decoding-panel.mode-picker"),
    );
    expect(
      screen.getByTestId("speculative-decoding-panel.eagle3-deferred-note").textContent,
    ).toContain("Eagle3");
  });

  it("Mode picker dispatches specSetMode when operator switches to ngram", async () => {
    specGetModeMock.mockResolvedValueOnce({ modelId: MODEL_ID, currentMode: null });
    specSetModeMock.mockResolvedValueOnce({
      modelId: MODEL_ID,
      eventType: "FR-EVT-LLM-INFER-SPEC-MODE-CHANGE",
      previousMode: null,
      currentMode: { mode: "ngram", lookback: 32, maxDraft: 8 },
    });
    render(
      <SpeculativeDecodingPanel modelId={MODEL_ID} capabilities={caps()} />,
    );
    await waitFor(() =>
      screen.getByTestId("speculative-decoding-panel.mode-picker"),
    );
    const picker = screen.getByTestId(
      "speculative-decoding-panel.mode-picker",
    ) as HTMLSelectElement;
    fireEvent.change(picker, { target: { value: "ngram" } });
    await waitFor(() => {
      expect(specSetModeMock).toHaveBeenCalledTimes(1);
    });
    const call = specSetModeMock.mock.calls[0][0];
    expect(call.modelId).toBe(MODEL_ID);
    expect(call.mode).toEqual({ mode: "ngram", lookback: 32, maxDraft: 8 });
  });

  it("Eagle3 selection does NOT dispatch — surfaces deferral message instead (operator E-4)", async () => {
    specGetModeMock.mockResolvedValueOnce({ modelId: MODEL_ID, currentMode: null });
    render(
      <SpeculativeDecodingPanel modelId={MODEL_ID} capabilities={caps()} />,
    );
    await waitFor(() =>
      screen.getByTestId("speculative-decoding-panel.mode-picker"),
    );
    const picker = screen.getByTestId(
      "speculative-decoding-panel.mode-picker",
    ) as HTMLSelectElement;
    fireEvent.change(picker, { target: { value: "eagle3_deferred" } });
    await waitFor(() => {
      expect(screen.getByTestId("speculative-decoding-panel.error").textContent).toContain(
        "Eagle3 is deferred",
      );
    });
    expect(specSetModeMock).not.toHaveBeenCalled();
  });

  it("draft_model selection does NOT dispatch — surfaces draft-picker-pending message", async () => {
    specGetModeMock.mockResolvedValueOnce({ modelId: MODEL_ID, currentMode: null });
    render(
      <SpeculativeDecodingPanel modelId={MODEL_ID} capabilities={caps()} />,
    );
    await waitFor(() =>
      screen.getByTestId("speculative-decoding-panel.mode-picker"),
    );
    const picker = screen.getByTestId(
      "speculative-decoding-panel.mode-picker",
    ) as HTMLSelectElement;
    fireEvent.change(picker, { target: { value: "draft_model" } });
    await waitFor(() => {
      expect(screen.getByTestId("speculative-decoding-panel.error").textContent).toContain(
        "DraftModel picker UX is a follow-up MT",
      );
    });
    expect(specSetModeMock).not.toHaveBeenCalled();
  });

  it("specModeOptions cascade — supportsSpeculativeDraft=false yields None + eagle3_deferred only", () => {
    const options = specModeOptions(false, false);
    expect(options.map((o) => o.kind)).toEqual(["none", "eagle3_deferred"]);
  });

  it("specModeOptions cascade — supportsSpeculativeDraft=true yields None + Ngram + DraftModel + eagle3_deferred", () => {
    const options = specModeOptions(true, false);
    expect(options.map((o) => o.kind)).toEqual([
      "none",
      "ngram",
      "draft_model",
      "eagle3_deferred",
    ]);
  });

  it("specGetMode error surfaces inline without crashing", async () => {
    specGetModeMock.mockRejectedValueOnce(new Error("kernel offline"));
    render(
      <SpeculativeDecodingPanel modelId={MODEL_ID} capabilities={caps()} />,
    );
    await waitFor(() => {
      expect(
        screen.getByTestId("speculative-decoding-panel.error").textContent,
      ).toContain("kernel offline");
    });
  });
});
