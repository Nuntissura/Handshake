import { invoke } from "@tauri-apps/api/core";

// Mirrors KvQuantSupport in src/backend/handshake_core/src/model_runtime/capabilities.rs.
// serde(rename_all = "snake_case") emits these lowercase strings, so the
// TS union must use the same exact spelling.
export type KvQuantSupport = "none" | "q4" | "q8" | "q4_q8_mix";

export interface KvCacheStats {
  bytesUsed: number;
  bytesCapacity: number;
  prefixCacheEntries: number;
  prefixCacheHitCount: number;
  prefixCacheMissCount: number;
  quantLevelCurrent: KvQuantSupport;
}

export interface KvPrefixHandle {
  prefixId: string;
  contentHashHex: string;
  tokenCount: number;
}

export interface KvCacheExecPolicy {
  quantization?: KvQuantSupport;
  prefixCacheTtlSeconds?: number;
  maxBytes?: number;
}

export interface KvCacheCommandSettings {
  execPolicy?: KvCacheExecPolicy;
}

export interface KvSetQuantizationRequest {
  modelId: string;
  level?: KvQuantSupport;
  settings?: KvCacheCommandSettings;
}

export interface KvSetQuantizationResult {
  modelId: string;
  eventType: string;
  previousQuantization: KvQuantSupport;
  currentQuantization: KvQuantSupport;
}

export interface KvPrefixCommitRequest {
  modelId: string;
  prefixTokens: number[];
}

export interface KvPrefixCommitResult {
  modelId: string;
  eventType: string;
  prefixHandle: KvPrefixHandle;
  occupancy: KvCacheStats;
}

export interface KvPrefixRestoreRequest {
  modelId: string;
  prefixHandle: KvPrefixHandle;
}

export interface KvPrefixRestoreResult {
  modelId: string;
  eventType: string;
  prefixHandle: KvPrefixHandle;
  occupancy: KvCacheStats;
}

export interface KvEvictAllRequest {
  modelId: string;
}

export interface KvEvictAllResult {
  modelId: string;
  eventType: string;
  previousOccupancy: KvCacheStats;
  currentOccupancy: KvCacheStats;
}

export interface KvOccupancyRequest {
  modelId: string;
}

export interface KvOccupancyResult {
  modelId: string;
  occupancy: KvCacheStats;
}

export async function kvSetQuantization(
  request: KvSetQuantizationRequest,
): Promise<KvSetQuantizationResult> {
  return await invoke<KvSetQuantizationResult>(
    "kernel_model_runtime_kv_set_quantization",
    { request },
  );
}

export async function kvPrefixCommit(
  request: KvPrefixCommitRequest,
): Promise<KvPrefixCommitResult> {
  return await invoke<KvPrefixCommitResult>(
    "kernel_model_runtime_kv_prefix_commit",
    { request },
  );
}

export async function kvPrefixRestore(
  request: KvPrefixRestoreRequest,
): Promise<KvPrefixRestoreResult> {
  return await invoke<KvPrefixRestoreResult>(
    "kernel_model_runtime_kv_prefix_restore",
    { request },
  );
}

export async function kvEvictAll(
  request: KvEvictAllRequest,
): Promise<KvEvictAllResult> {
  return await invoke<KvEvictAllResult>(
    "kernel_model_runtime_kv_evict_all",
    { request },
  );
}

export async function kvOccupancy(
  request: KvOccupancyRequest,
): Promise<KvOccupancyResult> {
  return await invoke<KvOccupancyResult>(
    "kernel_model_runtime_kv_occupancy",
    { request },
  );
}

// AC-INFER-LAB-UI-TOGGLES "hidden, not greyed" calculator. Returns the
// list of KvQuantSupport values the picker should expose for a given
// `supportsKvQuantization` declaration. None → empty (picker hidden);
// Q4 → ["none","q4"]; Q8 → ["none","q8"]; Q4Q8Mix → all four.
export function quantOptionsFor(support: KvQuantSupport): KvQuantSupport[] {
  switch (support) {
    case "none":
      return [];
    case "q4":
      return ["none", "q4"];
    case "q8":
      return ["none", "q8"];
    case "q4_q8_mix":
      return ["none", "q4", "q8", "q4_q8_mix"];
    default:
      return [];
  }
}
