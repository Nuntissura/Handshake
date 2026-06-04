import { invoke } from "@tauri-apps/api/core";

// LoRA descriptor mirrors LoraDescriptorIpc in
// app/src-tauri/src/commands/lora.rs. All fields are required by the backend.
export interface LoraDescriptor {
  loraId: string;
  artifactPath: string;
  sha256: string;
  rank: number;
  targetModules: string[];
  baseModelCompat: string;
  licenseTag: string;
}

// Single stack entry passed in via settings.execPolicy.loraStack or the raw
// stack[] swap intent. Strength is clamped to LoraStrength bounds server-side
// (0..2 for the safe wrapper) and pre-validated client-side here too.
export interface LoraStackItem {
  descriptor: LoraDescriptor;
  strength: number;
}

export interface LoraExecPolicy {
  loraStack: LoraStackItem[];
}

export interface LoraCommandSettings {
  execPolicy?: LoraExecPolicy;
}

export interface LoraMountRequest {
  modelId: string;
  descriptor: LoraDescriptor;
  strength: number;
}

export interface LoraUnmountRequest {
  modelId: string;
  loraId: string;
}

export interface LoraSwapRequest {
  modelId: string;
  stack: LoraStackItem[];
  settings?: LoraCommandSettings;
}

export interface LoraListRequest {
  modelId: string;
}

// Active stack entry as returned by mount/unmount/swap/list — backend keeps
// only id + strength + mounted_at, full descriptor lives in the snapshot.
export interface LoraStackEntry {
  loraId: string;
  strength: number;
  mountedAtUtc: string;
}

export interface LoraStackSnapshotEntry {
  descriptor: LoraDescriptor;
  strength: number;
  mountedAtUtc: string;
}

export interface LoraMutationResult {
  modelId: string;
  eventType: string;
  activeStack: LoraStackEntry[];
}

export interface LoraSwapResult {
  modelId: string;
  eventType: string;
  previousStack: LoraStackSnapshotEntry[];
  activeStack: LoraStackEntry[];
}

export interface LoraListResult {
  modelId: string;
  activeStack: LoraStackEntry[];
}

// LoraStrength server-side bound (LoraStrength::try_new in handshake_core).
// Mirrored here so the UI rejects out-of-range slider values before they
// reach the runtime — required by MT-091 red_team minimum control on
// "All mutations route through KernelActionCatalogV1 (verified pre-mutation)".
export const LORA_STRENGTH_MIN = 0;
export const LORA_STRENGTH_MAX = 2;

export class LoraStrengthRangeError extends Error {
  readonly strength: number;
  constructor(strength: number) {
    super(
      `LoRA strength must be within [${LORA_STRENGTH_MIN}, ${LORA_STRENGTH_MAX}]; got ${strength}`,
    );
    this.name = "LoraStrengthRangeError";
    this.strength = strength;
  }
}

export function assertValidStrength(strength: number): void {
  if (!Number.isFinite(strength)) {
    throw new LoraStrengthRangeError(strength);
  }
  if (strength < LORA_STRENGTH_MIN || strength > LORA_STRENGTH_MAX) {
    throw new LoraStrengthRangeError(strength);
  }
}

export async function loraMount(
  request: LoraMountRequest,
): Promise<LoraMutationResult> {
  assertValidStrength(request.strength);
  return await invoke<LoraMutationResult>("kernel_model_runtime_lora_mount", {
    request,
  });
}

export async function loraUnmount(
  request: LoraUnmountRequest,
): Promise<LoraMutationResult> {
  return await invoke<LoraMutationResult>("kernel_model_runtime_lora_unmount", {
    request,
  });
}

export async function loraSwap(
  request: LoraSwapRequest,
): Promise<LoraSwapResult> {
  for (const item of request.stack) {
    assertValidStrength(item.strength);
  }
  if (request.settings?.execPolicy?.loraStack) {
    for (const item of request.settings.execPolicy.loraStack) {
      assertValidStrength(item.strength);
    }
  }
  return await invoke<LoraSwapResult>("kernel_model_runtime_lora_swap", {
    request,
  });
}

export async function loraList(
  request: LoraListRequest,
): Promise<LoraListResult> {
  return await invoke<LoraListResult>("kernel_model_runtime_lora_list", {
    request,
  });
}
