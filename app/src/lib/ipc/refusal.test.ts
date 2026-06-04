import { describe, expect, it, vi, beforeEach } from "vitest";

const invokeMock = vi.hoisted(() => vi.fn());

vi.mock("@tauri-apps/api/core", () => ({
  invoke: invokeMock,
}));

import { extractRefusal } from "./refusal";

describe("refusal IPC binding", () => {
  beforeEach(() => {
    invokeMock.mockReset();
  });

  it("invokes the refusal extract command with the camelCase request envelope", async () => {
    invokeMock.mockResolvedValueOnce({
      directions: [
        { layer: 14, values: [0.707, -0.707] },
      ],
      eventType: "FR-EVT-LLM-INFER-REFUSAL-EXTRACT",
    });

    const result = await extractRefusal({
      modelId: "019a1b2c-0000-7000-8000-aaaaaaaaaaaa",
      harmfulPrompts: ["harmful one"],
      harmlessPrompts: ["harmless one"],
      layers: [10, 14, 18],
    });

    expect(invokeMock).toHaveBeenCalledWith("kernel_model_runtime_refusal_extract", {
      request: {
        modelId: "019a1b2c-0000-7000-8000-aaaaaaaaaaaa",
        harmfulPrompts: ["harmful one"],
        harmlessPrompts: ["harmless one"],
        layers: [10, 14, 18],
      },
    });
    expect(result.directions).toHaveLength(1);
    expect(result.directions[0].layer).toBe(14);
  });
});
