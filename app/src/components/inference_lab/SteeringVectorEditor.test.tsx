import { render, screen, waitFor, fireEvent } from "@testing-library/react";
import { describe, expect, it, vi, beforeEach } from "vitest";
import type { ModelCapabilities } from "../../lib/ipc/model_runtime";

const listVectorsMock = vi.hoisted(() => vi.fn());
const setActiveMock = vi.hoisted(() => vi.fn());
const unregisterMock = vi.hoisted(() => vi.fn());

vi.mock("../../lib/ipc/steering", () => ({
  listVectors: listVectorsMock,
  setActive: setActiveMock,
  unregister: unregisterMock,
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
    expect(screen.getByText("calm-tone")).toBeInTheDocument();
    const row = screen.getByTestId(
      "steering-vector-editor.row.019a1b2c-0000-7000-8000-000000000001",
    );
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

  it("surfaces backend errors verbatim (runtime-unavailable path)", async () => {
    listVectorsMock.mockRejectedValueOnce(
      "activation_steering live model runtime manager is not attached for adapter llama_cpp",
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
      expect(errEl.textContent).toContain("live model runtime manager");
    });
  });
});
