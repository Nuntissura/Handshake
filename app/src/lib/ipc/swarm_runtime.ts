import { invoke } from "@tauri-apps/api/core";

// IPC bindings for the multi-model SWARM operator surface (MT-205). These call
// the REAL managed SwarmCoordinator backend commands registered in
// `app/src-tauri/src/lib.rs`. No mocks: spawn loads a real model session, chat
// runs a real generate, list/snapshot reflect the live coordinator registry.

export type SwarmProvider = "local" | "byok_cloud" | "official_cli";
export type SwarmRuntimeBinding = "candle" | "llama_cpp";

export interface SwarmSpawnRequest {
  provider: SwarmProvider;
  /** Local: required. On-disk model artifact (safetensors / GGUF). */
  artifactPath?: string;
  /** Local: required. Expected sha256 hex of the artifact (integrity gate). */
  sha256Expected?: string;
  /** Local: required. candle | llama_cpp. */
  runtimeBinding?: SwarmRuntimeBinding;
  /** Cloud: required. Allowlisted cloud model name (e.g. gpt-4o). */
  cloudModelName?: string;
  /** Concurrent instance index of this model (default 0). */
  instance?: number;
  /** Parent session id for ledger lineage. */
  parentSessionId?: string;
}

export interface SwarmInstanceId {
  modelId: string;
  instance: number;
  /** Canonical `<model_id>#<instance>` string passed back to cancel/chat. */
  composite: string;
}

export interface SwarmSession {
  instanceId: SwarmInstanceId;
  state: string;
  provider: string;
  runtimeBinding: string;
  artifactPath: string | null;
  cloudModelName: string | null;
}

export interface SwarmResourceSnapshot {
  concurrencyCap: number;
  concurrencyInUse: number;
  concurrencyAvailable: number;
  liveSessions: number;
  lifetimeSpawnsRemaining: number;
  tokensRemaining: number | null;
  costMicrosRemaining: number | null;
  budgetExhausted: boolean;
}

export interface SwarmChatResponse {
  text: string;
  tokenCount: number;
  finishReason: string | null;
}

export async function spawnSession(request: SwarmSpawnRequest): Promise<SwarmInstanceId> {
  return await invoke<SwarmInstanceId>("kernel_swarm_spawn_session", { request });
}

export async function cancelSession(instanceId: string): Promise<void> {
  await invoke("kernel_swarm_cancel_session", { instanceId });
}

export async function listActiveSessions(): Promise<SwarmSession[]> {
  return await invoke<SwarmSession[]>("kernel_swarm_list_active_sessions");
}

export async function resourceSnapshot(): Promise<SwarmResourceSnapshot> {
  return await invoke<SwarmResourceSnapshot>("kernel_swarm_resource_snapshot");
}

export async function chatGenerate(
  instanceId: string,
  prompt: string,
): Promise<SwarmChatResponse> {
  return await invoke<SwarmChatResponse>("kernel_swarm_chat_generate", {
    instanceId,
    prompt,
  });
}
