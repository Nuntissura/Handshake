import { render, screen, waitFor, fireEvent } from "@testing-library/react";
import { describe, expect, it, vi, beforeEach } from "vitest";
import type { ModelCapabilities } from "../../lib/ipc/model_runtime";

const listVectorsMock = vi.hoisted(() => vi.fn());
const setActiveMock = vi.hoisted(() => vi.fn());
const unregisterMock = vi.hoisted(() => vi.fn());
const generateAbMock = vi.hoisted(() => vi.fn());

vi.mock("../../lib/ipc/steering", () => ({
  listVectors: listVectorsMock,
  setActive: setActiveMock,
  unregister: unregisterMock,
  generateAb: generateAbMock,
  capture: vi.fn(),
  registerVector: vi.fn(),
}));

import { SteeringVectorEditor } from "./SteeringVectorEditor";

const STEERING_SUPPORTED: ModelCapabilities = {
  supportsLora: false,
  supportsKvPrefixCache: false,
  supportsKvQuantization: "none",
  supportsActivationSteering: true,
  supportsSubquadratic: false,
  supportsSpeculativeDraft: false,
  supportsEagle3: false,
};

const STEERING_UNSUPPORTED: ModelCapabilities = {
  ...STEERING_SUPPORTED,
  supportsActivationSteering: false,
};

const MODEL_ID = "019a1b2c-0000-7000-8000-aaaaaaaaaaaa";

describe("SteeringVectorEditor", () => {
  beforeEach(() => {
    listVectorsMock.mockReset();
    setActiveMock.mockReset();
    unregisterMock.mockReset();
    generateAbMock.mockReset();
  });

  it("renders nothing when the model adapter does not support activation steering", () => {
    const { container } = render(
      <SteeringVectorEditor
        modelId={MODEL_ID}
        capabilities={STEERING_UNSUPPORTED}
        nLayers={32}
      />,
    );
    expect(container.firstChild).toBeNull();
    expect(listVectorsMock).not.toHaveBeenCalled();
  });

  it("renders the empty state when the model has no registered vectors", async () => {
    listVectorsMock.mockResolvedValueOnce([]);

    render(
      <SteeringVectorEditor
        modelId={MODEL_ID}
        capabilities={STEERING_SUPPORTED}
        nLayers={32}
      />,
    );

    await waitFor(() => {
      expect(screen.getByTestId("steering-vector-editor.empty")).toBeInTheDocument();
    });
  });

  it("renders the vector table when the kernel returns registered vectors", async () => {
    listVectorsMock.mockResolvedValueOnce([
      {
        vectorId: "019a1b2c-0000-7000-8000-000000000001",
        name: "calm-tone",
        layer: 14,
        hookPoint: "resid_stream",
        intensity: 1.5,
        description: "test vector",
      },
    ]);

    render(
      <SteeringVectorEditor
        modelId={MODEL_ID}
        capabilities={STEERING_SUPPORTED}
        nLayers={32}
      />,
    );

    await waitFor(() => {
      expect(screen.getByTestId("steering-vector-editor.table")).toBeInTheDocument();
    });
    expect(screen.getByTestId("steering-vector-editor.ab-compare")).toBeInTheDocument();
    const row = screen.getByTestId(
      "steering-vector-editor.row.019a1b2c-0000-7000-8000-000000000001",
    );
    expect(row.textContent).toContain("calm-tone");
    expect(row.textContent).toContain("14");
    expect(row.textContent).toContain("resid_stream");
  });

  it("calls setActive when the operator toggles a vector on", async () => {
    listVectorsMock.mockResolvedValueOnce([
      {
        vectorId: "019a1b2c-0000-7000-8000-000000000001",
        name: "calm",
        layer: 12,
        hookPoint: "resid_stream",
        intensity: 1.0,
        description: "x",
      },
    ]);
    setActiveMock.mockResolvedValueOnce({
      activeIds: ["019a1b2c-0000-7000-8000-000000000001"],
      eventType: "FR-EVT-LLM-INFER-STEER-ACTIVE",
    });

    render(
      <SteeringVectorEditor
        modelId={MODEL_ID}
        capabilities={STEERING_SUPPORTED}
        nLayers={32}
      />,
    );

    const toggle = await screen.findByTestId(
      "steering-vector-editor.row.019a1b2c-0000-7000-8000-000000000001.active",
    );
    fireEvent.click(toggle);

    await waitFor(() => {
      expect(setActiveMock).toHaveBeenCalledWith(MODEL_ID, [
        "019a1b2c-0000-7000-8000-000000000001",
      ]);
    });
  });

  it("runs live A/B compare from registered vector UI selections", async () => {
    const afterVectorId = "019a1b2c-0000-7000-8000-000000000001";
    const beforeVectorId = "019a1b2c-0000-7000-8000-000000000002";
    listVectorsMock.mockResolvedValueOnce([
      {
        vectorId: afterVectorId,
        name: "calm",
        layer: 12,
        hookPoint: "resid_stream",
        intensity: 1.0,
        description: "after vector",
      },
      {
        vectorId: beforeVectorId,
        name: "direct",
        layer: 14,
        hookPoint: "resid_stream",
        intensity: 0.75,
        description: "before vector",
      },
    ]);
    generateAbMock.mockResolvedValueOnce({
      comparisons: [
        {
          prompt: "compare this",
          inactiveCompletion: "DIRECT-OUTPUT",
          activeCompletion: "CALM-OUTPUT",
        },
      ],
      activeVectorIds: [afterVectorId],
      inactiveVectorIds: [beforeVectorId],
      eventType: "FR-EVT-LLM-INFER-STEER-AB-COMPARE",
    });

    render(
      <SteeringVectorEditor
        modelId={MODEL_ID}
        capabilities={STEERING_SUPPORTED}
        nLayers={32}
      />,
    );

    await screen.findByTestId("steering-vector-editor.ab-compare");
    fireEvent.change(
      screen.getByTestId("steering-vector-editor.ab-compare.inactive-select"),
      { target: { value: beforeVectorId } },
    );
    fireEvent.change(screen.getByTestId("ab-compare.prompts"), {
      target: { value: "compare this" },
    });
    fireEvent.click(screen.getByTestId("ab-compare.generate"));

    await waitFor(() => {
      expect(generateAbMock).toHaveBeenCalledWith({
        modelId: MODEL_ID,
        prompts: ["compare this"],
        activeVectorIds: [afterVectorId],
        inactiveVectorIds: [beforeVectorId],
        maxTokens: 64,
      });
    });
    expect(screen.getByTestId("ab-compare.pair.0.active-text").textContent).toContain(
      "CALM-OUTPUT",
    );
    expect(screen.getByTestId("ab-compare.pair.0.inactive-text").textContent).toContain(
      "DIRECT-OUTPUT",
    );
  });

  it("surfaces backend errors verbatim (capture-not-available path)", async () => {
    // MT-068 + MT-096 ship live ModelRuntime dispatch. When no adapter is
    // attached for the selected model (or the adapter's hook ops report the
    // model is not loaded), the kernel returns a typed `capture_not_available`
    // reason. The editor must surface it verbatim.
    listVectorsMock.mockRejectedValueOnce(
      "capture_not_available: list_vectors requires a live ModelRuntime adapter attached for model 019a1b2c-0000-7000-8000-aaaaaaaaaaaa",
    );

    render(
      <SteeringVectorEditor
        modelId={MODEL_ID}
        capabilities={STEERING_SUPPORTED}
        nLayers={32}
      />,
    );

    await waitFor(() => {
      const errEl = screen.getByTestId("steering-vector-editor.error");
      expect(errEl.textContent).toContain("capture_not_available");
    });
  });
});
