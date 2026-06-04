import { invoke } from "@tauri-apps/api/core";

// MT-112 SSM variant enum mirrors src/backend/handshake_core/src/model_runtime/candle/state_vector.rs
// serde(rename_all = "snake_case") emits these strings.
export type SubquadVariant = "mamba2" | "rwkv_v5" | "rwkv_v6" | "rwkv_v7";

export interface SubquadCacheStats {
  bytesUsed: number;
  bytesCapacity: number;
  prefixCacheEntries: number;
  prefixCacheHitCount: number;
  prefixCacheMissCount: number;
  quantLevelCurrent: "none" | "q4" | "q8" | "q4_q8_mix";
}

export interface SubquadPrefixHandle {
  prefixId: string;
  contentHashHex: string;
  tokenCount: number;
}

export interface SubquadStateCommitRequest {
  modelId: string;
  prefixTokens: number[];
}

export interface SubquadStateCommitResult {
  modelId: string;
  eventType: string;
  prefixHandle: SubquadPrefixHandle;
  occupancy: SubquadCacheStats;
}

export interface SubquadStateRestoreRequest {
  modelId: string;
  prefixHandle: SubquadPrefixHandle;
}

export interface SubquadStateRestoreResult {
  modelId: string;
  eventType: string;
  prefixHandle: SubquadPrefixHandle;
  hit: boolean;
  occupancy: SubquadCacheStats;
}

export interface SubquadStateListRequest {
  modelId: string;
}

export interface SubquadStateListResult {
  modelId: string;
  occupancy: SubquadCacheStats;
}

export interface SubquadEvictAllRequest {
  modelId: string;
}

export interface SubquadEvictAllResult {
  modelId: string;
  previousOccupancy: SubquadCacheStats;
  currentOccupancy: SubquadCacheStats;
}

export interface SubquadPersistRequest {
  modelId: string;
  prefixHandle: SubquadPrefixHandle;
}

export interface SubquadRehydrateRequest {
  modelId: string;
}

// AC-INFER-LAB-UI-TOGGLES: variants the SSM panel surfaces. The runtime
// derives the actual variant from the loaded model's config (Mamba2 vs
// RWKV v5/v6/v7); the panel reads it back via a model-metadata lookup
// that lands alongside the variant-aware model loader (deferred to a
// follow-up MT). Until then the badge falls back to the variant the
// operator selected at model-register time, propagated through
// LoadedModelRuntime.engineOrigin.
export const SUBQUAD_VARIANT_LABELS: Record<SubquadVariant, string> = {
  mamba2: "Mamba2",
  rwkv_v5: "RWKV v5",
  rwkv_v6: "RWKV v6",
  rwkv_v7: "RWKV v7",
};

export async function subquadStateCommit(
  request: SubquadStateCommitRequest,
): Promise<SubquadStateCommitResult> {
  return await invoke<SubquadStateCommitResult>(
    "kernel_model_runtime_subquad_state_commit",
    { request },
  );
}

export async function subquadStateRestore(
  request: SubquadStateRestoreRequest,
): Promise<SubquadStateRestoreResult> {
  return await invoke<SubquadStateRestoreResult>(
    "kernel_model_runtime_subquad_state_restore",
    { request },
  );
}

export async function subquadStateList(
  request: SubquadStateListRequest,
): Promise<SubquadStateListResult> {
  return await invoke<SubquadStateListResult>(
    "kernel_model_runtime_subquad_state_list",
    { request },
  );
}

export async function subquadEvictAll(
  request: SubquadEvictAllRequest,
): Promise<SubquadEvictAllResult> {
  return await invoke<SubquadEvictAllResult>(
    "kernel_model_runtime_subquad_state_evict_all",
    { request },
  );
}

// MT-117 deferred. The Tauri command exists and returns a typed error
// ("subquadratic_persist_disk_deferred_mt117") so the UI can render a
// disabled control with a tooltip rather than failing on the wire.
export async function subquadPersist(
  request: SubquadPersistRequest,
): Promise<void> {
  return await invoke<void>("kernel_model_runtime_subquad_persist", {
    request,
  });
}

// MT-117 deferred (same pattern as subquadPersist).
export async function subquadRehydrate(
  request: SubquadRehydrateRequest,
): Promise<void> {
  return await invoke<void>("kernel_model_runtime_subquad_rehydrate", {
    request,
  });
}
