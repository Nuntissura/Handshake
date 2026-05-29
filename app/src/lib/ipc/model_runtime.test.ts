import { describe, expect, it, vi } from "vitest";

const invokeMock = vi.hoisted(() => vi.fn());

vi.mock("@tauri-apps/api/core", () => ({
  invoke: invokeMock,
}));

import { capabilities, listLoaded } from "./model_runtime";

describe("model runtime IPC bindings", () => {
  it("requests capabilities with the Tauri camelCase modelId payload", async () => {
    invokeMock.mockResolvedValueOnce({
      supportsLora: true,
      supportsKvPrefixCache: true,
      supportsKvQuantization: "q4",
      supportsActivationSteering: false,
      supportsSubquadratic: false,
      supportsSpeculativeDraft: false,
      supportsEagle3: false,
    });

    const result = await capabilities("019a1b2c-0000-7000-8000-000000000001");

    expect(invokeMock).toHaveBeenCalledWith("kernel_model_runtime_capabilities", {
      modelId: "019a1b2c-0000-7000-8000-000000000001",
    });
    expect(result.supportsActivationSteering).toBe(false);
  });

  it("requests the loaded model projection through the read-only IPC channel", async () => {
    invokeMock.mockResolvedValueOnce([
      {
        modelId: "019a1b2c-0000-7000-8000-000000000002",
        runtimeBinding: "candle",
        artifactPath: "fixtures/models/local-test.safetensors",
        sha256: "0707070707070707070707070707070707070707070707070707070707070707",
        perfStats: {
          tokensPerSecond: null,
          contextTokens: null,
          lastLatencyMs: null,
        },
      },
    ]);

    const result = await listLoaded();

    expect(invokeMock).toHaveBeenCalledWith("kernel_model_runtime_list_loaded");
    expect(result[0].runtimeBinding).toBe("candle");
    expect(result[0].perfStats.tokensPerSecond).toBeNull();
  });
});
