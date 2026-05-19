import { invoke } from "@tauri-apps/api/core";

export interface RefusalDirection {
  layer: number;
  values: number[];
}

export interface RefusalExtractResult {
  directions: RefusalDirection[];
  eventType: string;
}

export interface RefusalExtractRequest {
  modelId: string;
  harmfulPrompts: string[];
  harmlessPrompts: string[];
  layers: number[];
}

export async function extractRefusal(
  request: RefusalExtractRequest,
): Promise<RefusalExtractResult> {
  return await invoke<RefusalExtractResult>("kernel_model_runtime_refusal_extract", {
    request,
  });
}
