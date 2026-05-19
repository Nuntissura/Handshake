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
