import { render, screen, waitFor, fireEvent } from "@testing-library/react";
import { describe, expect, it, vi, beforeEach } from "vitest";
import type { ModelCapabilities } from "../../lib/ipc/model_runtime";
import type { LoraStackItem } from "../../lib/ipc/lora";

const loraListMock = vi.hoisted(() => vi.fn());
const loraMountMock = vi.hoisted(() => vi.fn());
const loraUnmountMock = vi.hoisted(() => vi.fn());
const loraSwapMock = vi.hoisted(() => vi.fn());
const dialogOpenMock = vi.hoisted(() => vi.fn());

vi.mock("../../lib/ipc/lora", async () => {
  const actual =
    await vi.importActual<typeof import("../../lib/ipc/lora")>("../../lib/ipc/lora");
  return {
    ...actual,
    loraList: loraListMock,
    loraMount: loraMountMock,
    loraUnmount: loraUnmountMock,
    loraSwap: loraSwapMock,
  };
});

vi.mock("@tauri-apps/plugin-dialog", () => ({
  open: dialogOpenMock,
}));

import { LoraStackComposer } from "./LoraStackComposer";

const MODEL_ID = "019a1b2c-0000-7000-8000-aaaaaaaaaaaa";
const LORA_ID_A = "019a1b2c-0000-7000-8000-000000000001";
const LORA_ID_B = "019a1b2c-0000-7000-8000-000000000002";
const SHA256_64 =
  "aabbccddeeff00112233445566778899aabbccddeeff00112233445566778899";

const LORA_SUPPORTED: ModelCapabilities = {
  supportsLora: true,
  supportsKvPrefixCache: false,
  supportsKvQuantization: "none",
  supportsActivationSteering: false,
  supportsSubquadratic: false,
  supportsSpeculativeDraft: false,
  supportsEagle3: false,
};

const LORA_UNSUPPORTED: ModelCapabilities = {
  ...LORA_SUPPORTED,
  supportsLora: false,
};

function lightEntry(loraId: string, strength: number) {
  return {
    loraId,
    strength,
    mountedAtUtc: "2026-05-22T10:00:00Z",
  };
}

describe("LoraStackComposer", () => {
  beforeEach(() => {
    loraListMock.mockReset();
    loraMountMock.mockReset();
    loraUnmountMock.mockReset();
    loraSwapMock.mockReset();
    dialogOpenMock.mockReset();
  });

  // ------------------------------------------------------------------
  // AC-INFER-LAB-UI-TOGGLES: hidden (display:none), not greyed.
  // ------------------------------------------------------------------
  it("renders nothing when the model adapter does not support LoRA", () => {
    const { container } = render(
      <LoraStackComposer modelId={MODEL_ID} capabilities={LORA_UNSUPPORTED} />,
    );
    expect(container.firstChild).toBeNull();
    expect(loraListMock).not.toHaveBeenCalled();
  });

  it("renders nothing when capabilities is null (not yet probed)", () => {
    const { container } = render(
      <LoraStackComposer modelId={MODEL_ID} capabilities={null} />,
    );
    expect(container.firstChild).toBeNull();
  });

  // ------------------------------------------------------------------
  // Initial load
  // ------------------------------------------------------------------
  it("renders the empty state when no LoRAs are mounted", async () => {
    loraListMock.mockResolvedValueOnce({
      modelId: MODEL_ID,
      activeStack: [],
    });
    render(<LoraStackComposer modelId={MODEL_ID} capabilities={LORA_SUPPORTED} />);
    await waitFor(() => {
      expect(screen.getByTestId("lora-stack-composer.empty")).toBeInTheDocument();
    });
    expect(screen.queryByTestId("lora-stack-composer.list")).not.toBeInTheDocument();
  });

  it("renders a backend error when loraList fails", async () => {
    loraListMock.mockRejectedValueOnce(new Error("kernel offline"));
    render(<LoraStackComposer modelId={MODEL_ID} capabilities={LORA_SUPPORTED} />);
    await waitFor(() => {
      expect(screen.getByTestId("lora-stack-composer.error").textContent).toContain(
        "kernel offline",
      );
    });
  });

  it("renders the mounted stack with license badge per row", async () => {
    loraListMock.mockResolvedValueOnce({
      modelId: MODEL_ID,
      activeStack: [lightEntry(LORA_ID_A, 0.5)],
    });
    render(<LoraStackComposer modelId={MODEL_ID} capabilities={LORA_SUPPORTED} />);
    await waitFor(() => {
      expect(
        screen.getByTestId(`lora-stack-composer.row.${LORA_ID_A}`),
      ).toBeInTheDocument();
    });
    // License badge is always rendered, even when descriptor is unknown
    // client-side (the row shows "license unknown" until re-mount).
    expect(
      screen.getByTestId(`lora-stack-composer.row.${LORA_ID_A}.license`),
    ).toHaveTextContent("license unknown");
    expect(
      screen.getByTestId(`lora-stack-composer.row.${LORA_ID_A}.missing-descriptor`),
    ).toBeInTheDocument();
  });

  // ------------------------------------------------------------------
  // Mount-from-disk
  // ------------------------------------------------------------------
  it("mounts a LoRA after the operator picks a file and fills the form", async () => {
    loraListMock.mockResolvedValueOnce({ modelId: MODEL_ID, activeStack: [] });
    dialogOpenMock.mockResolvedValueOnce("/loras/story.safetensors");
    loraMountMock.mockImplementationOnce(async (request) => {
      return {
        modelId: MODEL_ID,
        eventType: "FR-EVT-LLM-INFER-LORA-MOUNT",
        activeStack: [lightEntry(request.descriptor.loraId, request.strength)],
      };
    });

    render(<LoraStackComposer modelId={MODEL_ID} capabilities={LORA_SUPPORTED} />);
    await waitFor(() => {
      expect(screen.getByTestId("lora-stack-composer.empty")).toBeInTheDocument();
    });

    fireEvent.click(screen.getByTestId("lora-stack-composer.mount-from-disk"));
    await waitFor(() => {
      expect(screen.getByTestId("lora-stack-composer.mount-form")).toBeInTheDocument();
    });

    // Verify artifact path was filled from the file dialog
    const artifactPathInput = screen.getByTestId(
      "lora-stack-composer.mount-form.artifact-path",
    ) as HTMLInputElement;
    expect(artifactPathInput.value).toBe("/loras/story.safetensors");

    // Fill required fields
    fireEvent.change(screen.getByTestId("lora-stack-composer.mount-form.sha256"), {
      target: { value: SHA256_64 },
    });
    fireEvent.change(
      screen.getByTestId("lora-stack-composer.mount-form.base-model-compat"),
      { target: { value: "local-test-base" } },
    );
    fireEvent.change(screen.getByTestId("lora-stack-composer.mount-form.license-tag"), {
      target: { value: "operator-local" },
    });
    fireEvent.change(screen.getByTestId("lora-stack-composer.mount-form.strength"), {
      target: { value: "0.75" },
    });

    fireEvent.click(screen.getByTestId("lora-stack-composer.mount-form.confirm"));

    await waitFor(() => {
      expect(loraMountMock).toHaveBeenCalledTimes(1);
    });
    const call = loraMountMock.mock.calls[0][0];
    expect(call.modelId).toBe(MODEL_ID);
    expect(call.strength).toBeCloseTo(0.75);
    expect(call.descriptor.sha256).toBe(SHA256_64);
    expect(call.descriptor.licenseTag).toBe("operator-local");
    expect(call.descriptor.baseModelCompat).toBe("local-test-base");
    expect(call.descriptor.rank).toBe(8);
    expect(call.descriptor.targetModules).toEqual(["q_proj", "v_proj"]);
    // LoRA ID is hand-rolled v7 client-side; must be a v7-shaped UUID.
    expect(call.descriptor.loraId).toMatch(
      /^[0-9a-f]{8}-[0-9a-f]{4}-7[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/,
    );
  });

  it("rejects mount when sha256 is malformed and does not call the backend", async () => {
    loraListMock.mockResolvedValueOnce({ modelId: MODEL_ID, activeStack: [] });
    dialogOpenMock.mockResolvedValueOnce("/loras/story.safetensors");

    render(<LoraStackComposer modelId={MODEL_ID} capabilities={LORA_SUPPORTED} />);
    await waitFor(() => screen.getByTestId("lora-stack-composer.empty"));

    fireEvent.click(screen.getByTestId("lora-stack-composer.mount-from-disk"));
    await waitFor(() =>
      expect(screen.getByTestId("lora-stack-composer.mount-form")).toBeInTheDocument(),
    );

    // Leave sha256 empty (malformed)
    fireEvent.change(
      screen.getByTestId("lora-stack-composer.mount-form.base-model-compat"),
      { target: { value: "local-test-base" } },
    );
    fireEvent.change(screen.getByTestId("lora-stack-composer.mount-form.license-tag"), {
      target: { value: "operator-local" },
    });

    fireEvent.click(screen.getByTestId("lora-stack-composer.mount-form.confirm"));

    await waitFor(() => {
      expect(screen.getByTestId("lora-stack-composer.mount-form.error").textContent)
        .toContain("sha256");
    });
    expect(loraMountMock).not.toHaveBeenCalled();
  });

  it("rejects mount when license tag is empty (operator licensing discipline)", async () => {
    loraListMock.mockResolvedValueOnce({ modelId: MODEL_ID, activeStack: [] });
    dialogOpenMock.mockResolvedValueOnce("/loras/story.safetensors");

    render(<LoraStackComposer modelId={MODEL_ID} capabilities={LORA_SUPPORTED} />);
    await waitFor(() => screen.getByTestId("lora-stack-composer.empty"));

    fireEvent.click(screen.getByTestId("lora-stack-composer.mount-from-disk"));
    await waitFor(() =>
      expect(screen.getByTestId("lora-stack-composer.mount-form")).toBeInTheDocument(),
    );

    fireEvent.change(screen.getByTestId("lora-stack-composer.mount-form.sha256"), {
      target: { value: SHA256_64 },
    });
    fireEvent.change(
      screen.getByTestId("lora-stack-composer.mount-form.base-model-compat"),
      { target: { value: "local-test-base" } },
    );
    // license tag left empty
    fireEvent.click(screen.getByTestId("lora-stack-composer.mount-form.confirm"));

    await waitFor(() => {
      expect(screen.getByTestId("lora-stack-composer.mount-form.error").textContent)
        .toContain("licenseTag");
    });
    expect(loraMountMock).not.toHaveBeenCalled();
  });

  it("rejects mount when strength is out of [0,2]", async () => {
    loraListMock.mockResolvedValueOnce({ modelId: MODEL_ID, activeStack: [] });
    dialogOpenMock.mockResolvedValueOnce("/loras/story.safetensors");

    render(<LoraStackComposer modelId={MODEL_ID} capabilities={LORA_SUPPORTED} />);
    await waitFor(() => screen.getByTestId("lora-stack-composer.empty"));

    fireEvent.click(screen.getByTestId("lora-stack-composer.mount-from-disk"));
    await waitFor(() =>
      expect(screen.getByTestId("lora-stack-composer.mount-form")).toBeInTheDocument(),
    );

    fireEvent.change(screen.getByTestId("lora-stack-composer.mount-form.sha256"), {
      target: { value: SHA256_64 },
    });
    fireEvent.change(
      screen.getByTestId("lora-stack-composer.mount-form.base-model-compat"),
      { target: { value: "local-test-base" } },
    );
    fireEvent.change(screen.getByTestId("lora-stack-composer.mount-form.license-tag"), {
      target: { value: "operator-local" },
    });
    fireEvent.change(screen.getByTestId("lora-stack-composer.mount-form.strength"), {
      target: { value: "5.0" },
    });

    fireEvent.click(screen.getByTestId("lora-stack-composer.mount-form.confirm"));

    await waitFor(() => {
      expect(screen.getByTestId("lora-stack-composer.mount-form.error").textContent)
        .toContain("strength");
    });
    expect(loraMountMock).not.toHaveBeenCalled();
  });

  it("cancels the mount dialog and clears state when the operator declines", async () => {
    loraListMock.mockResolvedValueOnce({ modelId: MODEL_ID, activeStack: [] });
    dialogOpenMock.mockResolvedValueOnce("/loras/story.safetensors");

    render(<LoraStackComposer modelId={MODEL_ID} capabilities={LORA_SUPPORTED} />);
    await waitFor(() => screen.getByTestId("lora-stack-composer.empty"));

    fireEvent.click(screen.getByTestId("lora-stack-composer.mount-from-disk"));
    await waitFor(() => screen.getByTestId("lora-stack-composer.mount-form"));

    fireEvent.click(screen.getByTestId("lora-stack-composer.mount-form.cancel"));

    expect(screen.queryByTestId("lora-stack-composer.mount-form")).not.toBeInTheDocument();
    expect(loraMountMock).not.toHaveBeenCalled();
  });

  it("does not open the mount form when the file dialog is dismissed", async () => {
    loraListMock.mockResolvedValueOnce({ modelId: MODEL_ID, activeStack: [] });
    dialogOpenMock.mockResolvedValueOnce(null);

    render(<LoraStackComposer modelId={MODEL_ID} capabilities={LORA_SUPPORTED} />);
    await waitFor(() => screen.getByTestId("lora-stack-composer.empty"));

    fireEvent.click(screen.getByTestId("lora-stack-composer.mount-from-disk"));

    await waitFor(() => {
      expect(dialogOpenMock).toHaveBeenCalledTimes(1);
    });
    expect(screen.queryByTestId("lora-stack-composer.mount-form")).not.toBeInTheDocument();
  });

  // ------------------------------------------------------------------
  // Strength slider
  // ------------------------------------------------------------------
  it("strength slider triggers loraSwap with settings.execPolicy.loraStack", async () => {
    // Bootstrap with a mounted LoRA (descriptor cached via mount call).
    loraListMock.mockResolvedValueOnce({ modelId: MODEL_ID, activeStack: [] });
    dialogOpenMock.mockResolvedValueOnce("/loras/story.safetensors");
    // Mock must echo the client-generated LoRA ID so the descriptor cache
    // lookup matches the rendered row.
    loraMountMock.mockImplementationOnce(async (request) => ({
      modelId: MODEL_ID,
      eventType: "FR-EVT-LLM-INFER-LORA-MOUNT",
      activeStack: [lightEntry(request.descriptor.loraId, request.strength)],
    }));
    loraSwapMock.mockImplementationOnce(async (request) => ({
      modelId: MODEL_ID,
      eventType: "FR-EVT-LLM-INFER-LORA-SWAP",
      previousStack: [],
      activeStack: request.settings.execPolicy.loraStack.map((it: LoraStackItem) =>
        lightEntry(it.descriptor.loraId, it.strength),
      ),
    }));

    render(<LoraStackComposer modelId={MODEL_ID} capabilities={LORA_SUPPORTED} />);
    await waitFor(() => screen.getByTestId("lora-stack-composer.empty"));

    // Mount through the form so the descriptor is cached client-side.
    fireEvent.click(screen.getByTestId("lora-stack-composer.mount-from-disk"));
    await waitFor(() => screen.getByTestId("lora-stack-composer.mount-form"));
    fireEvent.change(screen.getByTestId("lora-stack-composer.mount-form.sha256"), {
      target: { value: SHA256_64 },
    });
    fireEvent.change(
      screen.getByTestId("lora-stack-composer.mount-form.base-model-compat"),
      { target: { value: "local-test-base" } },
    );
    fireEvent.change(screen.getByTestId("lora-stack-composer.mount-form.license-tag"), {
      target: { value: "operator-local" },
    });
    fireEvent.click(screen.getByTestId("lora-stack-composer.mount-form.confirm"));
    await waitFor(() => expect(loraMountMock).toHaveBeenCalled());

    const mountedLoraId = loraMountMock.mock.calls[0][0].descriptor.loraId as string;
    await waitFor(() =>
      expect(screen.getByTestId(`lora-stack-composer.row.${mountedLoraId}`))
        .toBeInTheDocument(),
    );

    // Now slide the strength.
    const slider = screen.getByTestId(
      `lora-stack-composer.row.${mountedLoraId}.strength`,
    );
    fireEvent.change(slider, { target: { value: "1.25" } });

    await waitFor(() => {
      expect(loraSwapMock).toHaveBeenCalledTimes(1);
    });
    const swapCall = loraSwapMock.mock.calls[0][0];
    expect(swapCall.modelId).toBe(MODEL_ID);
    expect(swapCall.stack).toEqual([]);
    expect(swapCall.settings.execPolicy.loraStack).toHaveLength(1);
    expect(swapCall.settings.execPolicy.loraStack[0].descriptor.loraId).toBe(mountedLoraId);
    expect(swapCall.settings.execPolicy.loraStack[0].strength).toBeCloseTo(1.25);
  });

  it("strength slider is disabled when the descriptor is not cached client-side", async () => {
    loraListMock.mockResolvedValueOnce({
      modelId: MODEL_ID,
      activeStack: [lightEntry(LORA_ID_A, 0.5)],
    });
    render(<LoraStackComposer modelId={MODEL_ID} capabilities={LORA_SUPPORTED} />);
    await waitFor(() =>
      screen.getByTestId(`lora-stack-composer.row.${LORA_ID_A}.missing-descriptor`),
    );
    const slider = screen.getByTestId(
      `lora-stack-composer.row.${LORA_ID_A}.strength`,
    ) as HTMLInputElement;
    expect(slider.disabled).toBe(true);
  });

  // ------------------------------------------------------------------
  // Unmount
  // ------------------------------------------------------------------
  it("Unmount button calls loraUnmount and removes the row", async () => {
    loraListMock.mockResolvedValueOnce({
      modelId: MODEL_ID,
      activeStack: [lightEntry(LORA_ID_A, 0.5), lightEntry(LORA_ID_B, 1.0)],
    });
    loraUnmountMock.mockResolvedValueOnce({
      modelId: MODEL_ID,
      eventType: "FR-EVT-LLM-INFER-LORA-UNMOUNT",
      activeStack: [lightEntry(LORA_ID_B, 1.0)],
    });

    render(<LoraStackComposer modelId={MODEL_ID} capabilities={LORA_SUPPORTED} />);
    await waitFor(() =>
      screen.getByTestId(`lora-stack-composer.row.${LORA_ID_A}.unmount`),
    );

    fireEvent.click(screen.getByTestId(`lora-stack-composer.row.${LORA_ID_A}.unmount`));

    await waitFor(() => {
      expect(loraUnmountMock).toHaveBeenCalledWith({ modelId: MODEL_ID, loraId: LORA_ID_A });
    });
    await waitFor(() => {
      expect(screen.queryByTestId(`lora-stack-composer.row.${LORA_ID_A}`)).toBeNull();
      expect(screen.getByTestId(`lora-stack-composer.row.${LORA_ID_B}`)).toBeInTheDocument();
    });
  });

  it("renders backend error on Unmount failure without removing the row", async () => {
    loraListMock.mockResolvedValueOnce({
      modelId: MODEL_ID,
      activeStack: [lightEntry(LORA_ID_A, 0.5)],
    });
    loraUnmountMock.mockRejectedValueOnce(new Error("unknown lora"));
    render(<LoraStackComposer modelId={MODEL_ID} capabilities={LORA_SUPPORTED} />);
    await waitFor(() =>
      screen.getByTestId(`lora-stack-composer.row.${LORA_ID_A}.unmount`),
    );
    fireEvent.click(screen.getByTestId(`lora-stack-composer.row.${LORA_ID_A}.unmount`));
    await waitFor(() => {
      expect(screen.getByTestId("lora-stack-composer.error").textContent).toContain(
        "unknown lora",
      );
    });
  });

  // ------------------------------------------------------------------
  // Save as Work Profile
  // ------------------------------------------------------------------
  it("Save as Work Profile is disabled when any descriptor is missing client-side", async () => {
    loraListMock.mockResolvedValueOnce({
      modelId: MODEL_ID,
      activeStack: [lightEntry(LORA_ID_A, 0.5)],
    });
    render(<LoraStackComposer modelId={MODEL_ID} capabilities={LORA_SUPPORTED} />);
    await waitFor(() =>
      screen.getByTestId(`lora-stack-composer.row.${LORA_ID_A}.missing-descriptor`),
    );
    const saveButton = screen.getByTestId(
      "lora-stack-composer.save-profile",
    ) as HTMLButtonElement;
    expect(saveButton.disabled).toBe(true);
  });

  it("Save as Work Profile dispatches swap with settings.execPolicy.loraStack", async () => {
    loraListMock.mockResolvedValueOnce({ modelId: MODEL_ID, activeStack: [] });
    dialogOpenMock.mockResolvedValueOnce("/loras/story.safetensors");
    loraMountMock.mockImplementationOnce(async (request) => ({
      modelId: MODEL_ID,
      eventType: "FR-EVT-LLM-INFER-LORA-MOUNT",
      activeStack: [lightEntry(request.descriptor.loraId, request.strength)],
    }));
    loraSwapMock.mockImplementationOnce(async (request) => ({
      modelId: MODEL_ID,
      eventType: "FR-EVT-LLM-INFER-LORA-SWAP",
      previousStack: [],
      activeStack: request.settings.execPolicy.loraStack.map((it: LoraStackItem) =>
        lightEntry(it.descriptor.loraId, it.strength),
      ),
    }));

    render(<LoraStackComposer modelId={MODEL_ID} capabilities={LORA_SUPPORTED} />);
    await waitFor(() => screen.getByTestId("lora-stack-composer.empty"));

    fireEvent.click(screen.getByTestId("lora-stack-composer.mount-from-disk"));
    await waitFor(() => screen.getByTestId("lora-stack-composer.mount-form"));
    fireEvent.change(screen.getByTestId("lora-stack-composer.mount-form.sha256"), {
      target: { value: SHA256_64 },
    });
    fireEvent.change(
      screen.getByTestId("lora-stack-composer.mount-form.base-model-compat"),
      { target: { value: "local-test-base" } },
    );
    fireEvent.change(screen.getByTestId("lora-stack-composer.mount-form.license-tag"), {
      target: { value: "operator-local" },
    });
    fireEvent.click(screen.getByTestId("lora-stack-composer.mount-form.confirm"));
    await waitFor(() => expect(loraMountMock).toHaveBeenCalled());

    const mountedLoraId = loraMountMock.mock.calls[0][0].descriptor.loraId as string;
    await waitFor(() =>
      screen.getByTestId(`lora-stack-composer.row.${mountedLoraId}`),
    );

    fireEvent.click(screen.getByTestId("lora-stack-composer.save-profile"));

    await waitFor(() => {
      expect(loraSwapMock).toHaveBeenCalledTimes(1);
    });
    const call = loraSwapMock.mock.calls[0][0];
    expect(call.settings.execPolicy.loraStack).toHaveLength(1);
    expect(call.settings.execPolicy.loraStack[0].descriptor.loraId).toBe(mountedLoraId);
    await waitFor(() => {
      expect(screen.getByTestId("lora-stack-composer.profile-notice").textContent)
        .toContain("Saved as Work Profile");
    });
  });

  // ------------------------------------------------------------------
  // Drag-to-reorder
  // ------------------------------------------------------------------
  it("drag-to-reorder triggers loraSwap with the reordered stack", async () => {
    // Mount two LoRAs through the form so both descriptors are cached.
    loraListMock.mockResolvedValueOnce({ modelId: MODEL_ID, activeStack: [] });
    dialogOpenMock.mockResolvedValueOnce("/loras/a.safetensors");
    dialogOpenMock.mockResolvedValueOnce("/loras/b.safetensors");

    // Backend state shared across the two mount calls so the second
    // response can reference the first mounted id without us needing to
    // capture the client-generated uuid out-of-band.
    const backendStack: { loraId: string; strength: number }[] = [];
    loraMountMock.mockImplementation(async (request) => {
      backendStack.push({
        loraId: request.descriptor.loraId,
        strength: request.strength,
      });
      return {
        modelId: MODEL_ID,
        eventType: "FR-EVT-LLM-INFER-LORA-MOUNT",
        activeStack: backendStack.map((it) => lightEntry(it.loraId, it.strength)),
      };
    });
    loraSwapMock.mockImplementation(async (request) => ({
      modelId: MODEL_ID,
      eventType: "FR-EVT-LLM-INFER-LORA-SWAP",
      previousStack: [],
      activeStack: request.settings.execPolicy.loraStack.map((it: LoraStackItem) =>
        lightEntry(it.descriptor.loraId, it.strength),
      ),
    }));

    render(<LoraStackComposer modelId={MODEL_ID} capabilities={LORA_SUPPORTED} />);
    await waitFor(() => screen.getByTestId("lora-stack-composer.empty"));

    // First mount.
    fireEvent.click(screen.getByTestId("lora-stack-composer.mount-from-disk"));
    await waitFor(() => screen.getByTestId("lora-stack-composer.mount-form"));
    fireEvent.change(screen.getByTestId("lora-stack-composer.mount-form.sha256"), {
      target: { value: SHA256_64 },
    });
    fireEvent.change(
      screen.getByTestId("lora-stack-composer.mount-form.base-model-compat"),
      { target: { value: "local-test-base" } },
    );
    fireEvent.change(screen.getByTestId("lora-stack-composer.mount-form.license-tag"), {
      target: { value: "license-a" },
    });
    fireEvent.click(screen.getByTestId("lora-stack-composer.mount-form.confirm"));
    await waitFor(() => expect(loraMountMock).toHaveBeenCalledTimes(1));
    const firstId = loraMountMock.mock.calls[0][0].descriptor.loraId as string;
    await waitFor(() =>
      screen.getByTestId(`lora-stack-composer.row.${firstId}`),
    );

    // Second mount.
    fireEvent.click(screen.getByTestId("lora-stack-composer.mount-from-disk"));
    await waitFor(() => screen.getByTestId("lora-stack-composer.mount-form"));
    fireEvent.change(screen.getByTestId("lora-stack-composer.mount-form.sha256"), {
      target: { value: SHA256_64 },
    });
    fireEvent.change(
      screen.getByTestId("lora-stack-composer.mount-form.base-model-compat"),
      { target: { value: "local-test-base" } },
    );
    fireEvent.change(screen.getByTestId("lora-stack-composer.mount-form.license-tag"), {
      target: { value: "license-b" },
    });
    fireEvent.change(screen.getByTestId("lora-stack-composer.mount-form.strength"), {
      target: { value: "1.0" },
    });
    fireEvent.click(screen.getByTestId("lora-stack-composer.mount-form.confirm"));
    await waitFor(() => expect(loraMountMock).toHaveBeenCalledTimes(2));
    const secondId = loraMountMock.mock.calls[1][0].descriptor.loraId as string;
    await waitFor(() =>
      screen.getByTestId(`lora-stack-composer.row.${secondId}`),
    );

    // Simulate drag from row 0 onto row 1.
    const rowA = screen.getByTestId(`lora-stack-composer.row.${firstId}`);
    const rowB = screen.getByTestId(`lora-stack-composer.row.${secondId}`);
    fireEvent.dragStart(rowA);
    fireEvent.dragOver(rowB);
    fireEvent.drop(rowB);

    await waitFor(() => {
      expect(loraSwapMock).toHaveBeenCalledTimes(1);
    });
    const swapCall = loraSwapMock.mock.calls[0][0];
    expect(
      swapCall.settings.execPolicy.loraStack.map((it: LoraStackItem) => it.descriptor.loraId),
    ).toEqual([secondId, firstId]);
  });

  it("drag-to-reorder blocks when any LoRA has no cached descriptor", async () => {
    // Initial load gives two light entries with no descriptor cache.
    loraListMock.mockResolvedValueOnce({
      modelId: MODEL_ID,
      activeStack: [lightEntry(LORA_ID_A, 0.5), lightEntry(LORA_ID_B, 0.5)],
    });

    render(<LoraStackComposer modelId={MODEL_ID} capabilities={LORA_SUPPORTED} />);
    await waitFor(() =>
      screen.getByTestId(`lora-stack-composer.row.${LORA_ID_A}`),
    );

    const rowA = screen.getByTestId(`lora-stack-composer.row.${LORA_ID_A}`);
    const rowB = screen.getByTestId(`lora-stack-composer.row.${LORA_ID_B}`);
    fireEvent.dragStart(rowA);
    fireEvent.dragOver(rowB);
    fireEvent.drop(rowB);

    await waitFor(() => {
      expect(screen.getByTestId("lora-stack-composer.error").textContent).toContain(
        "Descriptor unavailable",
      );
    });
    expect(loraSwapMock).not.toHaveBeenCalled();
  });
});
