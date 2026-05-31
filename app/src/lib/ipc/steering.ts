import { invoke } from "@tauri-apps/api/core";

export interface SteeringVectorMeta {
  vectorId: string;
  name: string;
  layer: number;
  hookPoint: string;
  intensity: number;
  description: string;
}

export interface SteeringCaptureRequest {
  modelId: string;
  prompts: string[];
  layers: number[];
}

export interface LayerActivations {
  layer: number;
  activations: number[][];
}

export interface SteeringCaptureResult {
  tokensSeen: number;
  activationsByLayer: LayerActivations[];
  eventType: string;
}

export interface SteeringProvenanceInput {
  technique: string;
  positivePrompts?: string[];
  negativePrompts?: string[];
  author?: string | null;
  notes?: string | null;
}

export interface SteeringRegisterVectorRequest {
  modelId: string;
  name: string;
  layer: number;
  hookPoint: string;
  values: number[];
  intensity: number;
  description: string;
  provenance: SteeringProvenanceInput;
  // MT-097 persistence metadata. Optional so the IPC remains compatible with
  // callers that do not opt into governed persistence (test composition).
  licenseTag?: string;
  modelCompatTag?: string;
}

export interface SteeringVectorIdResult {
  vectorId: string;
  eventType: string;
}

export interface SteeringSetActiveResult {
  activeIds: string[];
  eventType: string;
}

export interface SteeringMutationResult {
  eventType: string;
}

export async function listVectors(modelId: string): Promise<SteeringVectorMeta[]> {
  return await invoke<SteeringVectorMeta[]>("kernel_model_runtime_steering_list_vectors", {
    request: { modelId },
  });
}

export async function setActive(
  modelId: string,
  vectorIds: string[],
): Promise<SteeringSetActiveResult> {
  return await invoke<SteeringSetActiveResult>("kernel_model_runtime_steering_set_active", {
    request: { modelId, vectorIds },
  });
}

export async function unregister(
  modelId: string,
  vectorId: string,
): Promise<SteeringMutationResult> {
  return await invoke<SteeringMutationResult>("kernel_model_runtime_steering_unregister", {
    request: { modelId, vectorId },
  });
}

export async function capture(
  request: SteeringCaptureRequest,
): Promise<SteeringCaptureResult> {
  return await invoke<SteeringCaptureResult>("kernel_model_runtime_steering_capture", {
    request,
  });
}

export async function registerVector(
  request: SteeringRegisterVectorRequest,
): Promise<SteeringVectorIdResult> {
  return await invoke<SteeringVectorIdResult>("kernel_model_runtime_steering_register_vector", {
    request,
  });
}

// MT-098: AB-compare (live BEFORE/AFTER generation).

export interface SteeringGenerateAbRequest {
  modelId: string;
  prompts: string[];
  activeVectorIds: string[];
  inactiveVectorIds?: string[];
  maxTokens?: number;
}

export interface SteeringAbComparison {
  prompt: string;
  /** Completion with the steering vectors INACTIVE (baseline / BEFORE). */
  inactiveCompletion: string;
  /** Completion with the steering vectors ACTIVE (steered / AFTER). */
  activeCompletion: string;
}

export interface SteeringGenerateAbResult {
  comparisons: SteeringAbComparison[];
  activeVectorIds: string[];
  inactiveVectorIds: string[];
  eventType: string;
}

/**
 * Run a live AB-compare generation pass: each prompt is generated twice through
 * the live CandleRuntime adapter — once with the proposed steering vectors
 * active and once with them inactive — returning both completions side by side.
 * The kernel runs the REAL candle generate via the live-runtime + steering path;
 * nothing is faked. When no live runtime is attached the kernel returns a typed
 * `capture_not_available` reason which callers should surface verbatim.
 */
export async function generateAb(
  request: SteeringGenerateAbRequest,
): Promise<SteeringGenerateAbResult> {
  return await invoke<SteeringGenerateAbResult>("kernel_model_runtime_steering_generate_ab", {
    request,
  });
}
