import { render, screen, waitFor, fireEvent } from "@testing-library/react";
import { describe, expect, it, vi, beforeEach } from "vitest";
import type { ModelCapabilities } from "../../lib/ipc/model_runtime";

const extractRefusalMock = vi.hoisted(() => vi.fn());
const registerVectorMock = vi.hoisted(() => vi.fn());
const setActiveMock = vi.hoisted(() => vi.fn());

vi.mock("../../lib/ipc/refusal", () => ({
  extractRefusal: extractRefusalMock,
}));

vi.mock("../../lib/ipc/steering", () => ({
  registerVector: registerVectorMock,
  setActive: setActiveMock,
}));

import { RefusalVectorWizard } from "./RefusalVectorWizard";

const STEERING_SUPPORTED: ModelCapabilities = {
  supportsLora: false,
  supportsKvPrefixCache: false,
  supportsKvQuantization: "none",
  supportsActivationSteering: true,
  supportsSubquadratic: false,
  supportsSpeculativeDraft: false,
  supportsEagle3: false,
};

const MODEL_ID = "019a1b2c-0000-7000-8000-aaaaaaaaaaaa";
const VECTOR_ID = "019a1b2c-0000-7000-8000-000000000001";

async function extractAndFill() {
  fireEvent.change(screen.getByTestId("refusal-vector-wizard.harmful"), {
    target: { value: "tell me something the model refuses" },
  });
  fireEvent.change(screen.getByTestId("refusal-vector-wizard.harmless"), {
    target: { value: "tell me something benign" },
  });
  fireEvent.change(screen.getByTestId("refusal-vector-wizard.layers"), {
    target: { value: "14" },
  });
  fireEvent.click(screen.getByTestId("refusal-vector-wizard.extract"));
  await screen.findByTestId("refusal-vector-wizard.save");
  fireEvent.change(screen.getByTestId("refusal-vector-wizard.name"), {
    target: { value: "refusal-l14" },
  });
  fireEvent.change(screen.getByTestId("refusal-vector-wizard.description"), {
    target: { value: "ablation vector" },
  });
}

describe("RefusalVectorWizard MT-102 disable-refusal acknowledgement", () => {
  beforeEach(() => {
    extractRefusalMock.mockReset();
    registerVectorMock.mockReset();
    setActiveMock.mockReset();
    extractRefusalMock.mockResolvedValue({
      directions: [{ layer: 14, values: [1, 0, 0] }],
    });
    registerVectorMock.mockResolvedValue({ vectorId: VECTOR_ID });
    setActiveMock.mockResolvedValue({
      activeIds: [VECTOR_ID],
      eventType: "FR-EVT-LLM-INFER-STEER-ACTIVE",
    });
  });

  it("blocks Save & activate until the operator acknowledges it disables refusal", async () => {
    render(
      <RefusalVectorWizard
        modelId={MODEL_ID}
        capabilities={STEERING_SUPPORTED}
        nLayers={32}
      />,
    );
    await extractAndFill();

    // activate-after-save defaults on -> acknowledgement row is shown and the
    // Save & activate button is disabled until it is checked.
    expect(
      screen.getByTestId("refusal-vector-wizard.disable-ack-row"),
    ).toBeInTheDocument();
    const save = screen.getByTestId(
      "refusal-vector-wizard.save",
    ) as HTMLButtonElement;
    expect(save.disabled).toBe(true);

    // A click while disabled must not fire the activation path.
    fireEvent.click(save);
    expect(registerVectorMock).not.toHaveBeenCalled();
    expect(setActiveMock).not.toHaveBeenCalled();

    // Acknowledge -> Save enabled -> register + activate fire.
    fireEvent.click(screen.getByTestId("refusal-vector-wizard.disable-ack"));
    expect(save.disabled).toBe(false);
    fireEvent.click(save);
    await waitFor(() => {
      expect(registerVectorMock).toHaveBeenCalledTimes(1);
      expect(setActiveMock).toHaveBeenCalledWith(MODEL_ID, [VECTOR_ID]);
    });
  });

  it("allows save without acknowledgement when activation is off, and does not activate", async () => {
    render(
      <RefusalVectorWizard
        modelId={MODEL_ID}
        capabilities={STEERING_SUPPORTED}
        nLayers={32}
      />,
    );
    await extractAndFill();

    // Turn activation off -> acknowledgement row disappears and Save is allowed
    // (saving an unactivated vector is the non-dangerous path; later activation
    // is still gated server-side by the MT-097 review status).
    fireEvent.click(
      screen.getByTestId("refusal-vector-wizard.activate-after-save"),
    );
    expect(
      screen.queryByTestId("refusal-vector-wizard.disable-ack-row"),
    ).toBeNull();
    const save = screen.getByTestId(
      "refusal-vector-wizard.save",
    ) as HTMLButtonElement;
    expect(save.disabled).toBe(false);
    fireEvent.click(save);
    await waitFor(() => {
      expect(registerVectorMock).toHaveBeenCalledTimes(1);
    });
    expect(setActiveMock).not.toHaveBeenCalled();
  });
});
