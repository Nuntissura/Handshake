import { describe, expect, it, vi, beforeEach } from "vitest";

const invokeMock = vi.hoisted(() => vi.fn());

vi.mock("@tauri-apps/api/core", () => ({
  invoke: invokeMock,
}));

import { extractCaa } from "./caa";

describe("CAA IPC binding", () => {
  beforeEach(() => {
    invokeMock.mockReset();
  });

  it("invokes the CAA extract command with the camelCase request envelope", async () => {
    invokeMock.mockResolvedValueOnce({
      vectorId: "019a1b2c-0000-7000-8000-aaaaaaaaaaaa",
      values: [4.0, -4.0],
      layer: 14,
      eventType: "FR-EVT-LLM-INFER-CAA-EXTRACT",
    });

    const result = await extractCaa({
      modelId: "019a1b2c-0000-7000-8000-bbbbbbbbbbbb",
      name: "syco-caa",
      description: "CAA sycophancy direction",
      pairs: [
        { context: "ctx", positive: "yes", negative: "no" },
      ],
      layer: 14,
    });

    expect(invokeMock).toHaveBeenCalledWith("kernel_model_runtime_caa_extract", {
      request: {
        modelId: "019a1b2c-0000-7000-8000-bbbbbbbbbbbb",
        name: "syco-caa",
        description: "CAA sycophancy direction",
        pairs: [{ context: "ctx", positive: "yes", negative: "no" }],
        layer: 14,
      },
    });
    expect(result.layer).toBe(14);
    expect(result.values).toHaveLength(2);
  });
});
