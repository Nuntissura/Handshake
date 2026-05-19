import { invoke } from "@tauri-apps/api/core";

export interface CaaPromptPair {
  context: string;
  positive: string;
  negative: string;
}

export interface CaaExtractRequest {
  modelId: string;
  name: string;
  description: string;
  pairs: CaaPromptPair[];
  layer: number;
}

export interface CaaExtractResult {
  vectorId: string;
  values: number[];
  layer: number;
  eventType: string;
}

export async function extractCaa(request: CaaExtractRequest): Promise<CaaExtractResult> {
  return await invoke<CaaExtractResult>("kernel_model_runtime_caa_extract", { request });
}
