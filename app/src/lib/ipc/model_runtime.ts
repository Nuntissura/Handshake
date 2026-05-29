import { invoke } from "@tauri-apps/api/core";

export type KvQuantSupport = "none" | "q4" | "q8" | "q4_q8_mix";

export interface ModelCapabilities {
  supportsLora: boolean;
  supportsKvPrefixCache: boolean;
  supportsKvQuantization: KvQuantSupport;
  supportsActivationSteering: boolean;
  supportsSubquadratic: boolean;
  supportsSpeculativeDraft: boolean;
  supportsEagle3: boolean;
}

export type RuntimeBinding = "llama_cpp" | "candle";

export interface ModelRuntimePerfStats {
  tokensPerSecond: number | null;
  contextTokens: number | null;
  lastLatencyMs: number | null;
}

export interface LoadedModelRuntime {
  modelId: string;
  runtimeBinding: RuntimeBinding;
  artifactPath: string;
  sha256: string;
  perfStats: ModelRuntimePerfStats;
}

export async function capabilities(modelId: string): Promise<ModelCapabilities> {
  return await invoke<ModelCapabilities>("kernel_model_runtime_capabilities", { modelId });
}

export async function listLoaded(): Promise<LoadedModelRuntime[]> {
  return await invoke<LoadedModelRuntime[]>("kernel_model_runtime_list_loaded");
}
