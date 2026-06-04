import { describe, expect, it, vi, beforeEach } from "vitest";

const invokeMock = vi.hoisted(() => vi.fn());

vi.mock("@tauri-apps/api/core", () => ({
  invoke: invokeMock,
}));

import {
  capture,
  generateAb,
  listVectors,
  registerVector,
  setActive,
  unregister,
} from "./steering";

describe("steering IPC bindings", () => {
  beforeEach(() => {
    invokeMock.mockReset();
  });

  it("lists vectors with a camelCase request envelope", async () => {
    invokeMock.mockResolvedValueOnce([
      {
        vectorId: "019a1b2c-0000-7000-8000-000000000001",
        name: "calm",
        layer: 12,
        hookPoint: "resid_stream",
        intensity: 1.0,
        description: "test",
      },
    ]);

    const result = await listVectors("019a1b2c-0000-7000-8000-aaaaaaaaaaaa");

    expect(invokeMock).toHaveBeenCalledWith("kernel_model_runtime_steering_list_vectors", {
      request: { modelId: "019a1b2c-0000-7000-8000-aaaaaaaaaaaa" },
    });
    expect(result[0].name).toBe("calm");
  });

  it("sets active vectors", async () => {
    invokeMock.mockResolvedValueOnce({
      activeIds: ["019a1b2c-0000-7000-8000-000000000001"],
      eventType: "FR-EVT-LLM-INFER-STEER-ACTIVE",
    });

    const result = await setActive("019a1b2c-0000-7000-8000-aaaaaaaaaaaa", [
      "019a1b2c-0000-7000-8000-000000000001",
    ]);

    expect(invokeMock).toHaveBeenCalledWith("kernel_model_runtime_steering_set_active", {
      request: {
        modelId: "019a1b2c-0000-7000-8000-aaaaaaaaaaaa",
        vectorIds: ["019a1b2c-0000-7000-8000-000000000001"],
      },
    });
    expect(result.activeIds).toHaveLength(1);
  });

  it("unregisters a single vector", async () => {
    invokeMock.mockResolvedValueOnce({ eventType: "FR-EVT-LLM-INFER-STEER-APPLY" });

    await unregister(
      "019a1b2c-0000-7000-8000-aaaaaaaaaaaa",
      "019a1b2c-0000-7000-8000-000000000001",
    );

    expect(invokeMock).toHaveBeenCalledWith("kernel_model_runtime_steering_unregister", {
      request: {
        modelId: "019a1b2c-0000-7000-8000-aaaaaaaaaaaa",
        vectorId: "019a1b2c-0000-7000-8000-000000000001",
      },
    });
  });

  it("captures activations with prompts and layer indices", async () => {
    invokeMock.mockResolvedValueOnce({
      tokensSeen: 12,
      activationsByLayer: [{ layer: 12, activations: [[0.1, 0.2]] }],
      eventType: "FR-EVT-LLM-INFER-STEER-CAPTURE",
    });

    const result = await capture({
      modelId: "019a1b2c-0000-7000-8000-aaaaaaaaaaaa",
      prompts: ["positive prompt"],
      layers: [12],
    });

    expect(invokeMock).toHaveBeenCalledWith("kernel_model_runtime_steering_capture", {
      request: {
        modelId: "019a1b2c-0000-7000-8000-aaaaaaaaaaaa",
        prompts: ["positive prompt"],
        layers: [12],
      },
    });
    expect(result.tokensSeen).toBe(12);
  });

  it("registers a vector with contrastive provenance", async () => {
    invokeMock.mockResolvedValueOnce({
      vectorId: "019a1b2c-0000-7000-8000-000000000001",
      eventType: "FR-EVT-LLM-INFER-STEER-REGISTER",
    });

    const result = await registerVector({
      modelId: "019a1b2c-0000-7000-8000-aaaaaaaaaaaa",
      name: "wizard saved",
      layer: 12,
      hookPoint: "resid_stream",
      values: [0.1, 0.2, 0.3],
      intensity: 1.0,
      description: "captured via contrastive wizard",
      provenance: {
        technique: "repe",
        positivePrompts: ["pos"],
        negativePrompts: ["neg"],
      },
    });

    expect(invokeMock).toHaveBeenCalledWith(
      "kernel_model_runtime_steering_register_vector",
      expect.objectContaining({
        request: expect.objectContaining({
          name: "wizard saved",
          provenance: expect.objectContaining({ technique: "repe" }),
        }),
      }),
    );
    expect(result.vectorId).toBeDefined();
  });

  it("runs AB compare generation with active and inactive vector sets", async () => {
    invokeMock.mockResolvedValueOnce({
      comparisons: [
        {
          prompt: "compare tone",
          inactiveCompletion: "BEFORE",
          activeCompletion: "AFTER",
        },
      ],
      activeVectorIds: ["019a1b2c-0000-7000-8000-000000000001"],
      inactiveVectorIds: ["019a1b2c-0000-7000-8000-000000000002"],
      eventType: "FR-EVT-LLM-INFER-STEER-AB-COMPARE",
    });

    const result = await generateAb({
      modelId: "019a1b2c-0000-7000-8000-aaaaaaaaaaaa",
      prompts: ["compare tone"],
      activeVectorIds: ["019a1b2c-0000-7000-8000-000000000001"],
      inactiveVectorIds: ["019a1b2c-0000-7000-8000-000000000002"],
      maxTokens: 32,
    });

    expect(invokeMock).toHaveBeenCalledWith(
      "kernel_model_runtime_steering_generate_ab",
      {
        request: {
          modelId: "019a1b2c-0000-7000-8000-aaaaaaaaaaaa",
          prompts: ["compare tone"],
          activeVectorIds: ["019a1b2c-0000-7000-8000-000000000001"],
          inactiveVectorIds: ["019a1b2c-0000-7000-8000-000000000002"],
          maxTokens: 32,
        },
      },
    );
    expect(result.comparisons[0].activeCompletion).toBe("AFTER");
  });
});
