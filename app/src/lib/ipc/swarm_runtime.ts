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

// ---------------------------------------------------------------------------
// rank-4: live operator board (Jira-style). Columns = ModelSessionState,
// swimlanes = the swarm/worktree grouping, cards = sessions. The board is a
// READ-MODEL projection: fetch a snapshot on mount + on resync, then apply
// pushed `swarm://event` deltas in place; a `seq` gap or a `swarm://resync`
// event triggers a full reconcile (no partial-stream drift). UI writes are
// commands (spawn/cancel), never direct column mutations.
// ---------------------------------------------------------------------------

/** Lifecycle states = board columns (SCREAMING_SNAKE_CASE per backend serde). */
export const BOARD_COLUMNS = [
  "QUEUED",
  "LOADING",
  "READY",
  "GENERATING",
  "COMPLETED",
  "FAILED",
  "CANCELLED",
] as const;
export type SwarmSessionState = (typeof BOARD_COLUMNS)[number];

export interface SwarmBoardCard {
  instanceId: SwarmInstanceId;
  state: string;
  provider: string;
  runtimeBinding: string;
  swarmId: string | null;
  worktreeId: string | null;
}

export interface SwarmBoardSnapshot {
  cards: SwarmBoardCard[];
  liveSessions: number;
}

/** Raw (snake_case) ModelInstanceId as it appears inside a SwarmEvent. */
interface RawInstanceId {
  model_id: string;
  instance: number;
}

/** Typed SwarmEvent (externally-tagged, matching the Rust enum serde). */
export type SwarmEvent =
  | { SessionSpawned: { instance_id: RawInstanceId; parent_session_id: string; process_uuid: string } }
  | { SessionReady: { instance_id: RawInstanceId } }
  | { SessionStateChanged: { instance_id: RawInstanceId; from: string; to: string } }
  | { SessionCancelled: { instance_id: RawInstanceId; reason: string } }
  | { SessionCompleted: { instance_id: RawInstanceId } }
  | { SessionFailed: { instance_id: RawInstanceId; error: string } }
  | { ResourceAllocated: { instance_id: RawInstanceId; permits_in_use: number; permits_cap: number } }
  | { ResourceEvicted: { instance_id: RawInstanceId; terminal_state: string } }
  | { BreakerTripped: { signature: string } }
  | { LeaseExpired: { instance_id: RawInstanceId } }
  | { SpawnRejected: { instance_id: RawInstanceId; reason: string } };

export interface SwarmBoardDelta {
  seq: number;
  event: SwarmEvent;
}

export async function boardSnapshot(): Promise<SwarmBoardSnapshot> {
  return await invoke<SwarmBoardSnapshot>("kernel_swarm_board_snapshot");
}

/** Canonical card key `<model_id>#<instance>` from a raw or typed instance id. */
export function cardKey(id: { modelId: string; instance: number } | RawInstanceId): string {
  const modelId = "modelId" in id ? id.modelId : id.model_id;
  return `${modelId}#${id.instance}`;
}

/**
 * The lifecycle state a SwarmEvent moves a card TO, or null if the event does
 * not change a card's column (e.g. BreakerTripped). Used by the board reducer.
 */
export function eventTargetState(event: SwarmEvent): { key: string; state: string } | null {
  if ("SessionReady" in event) return { key: cardKey(event.SessionReady.instance_id), state: "READY" };
  if ("SessionStateChanged" in event)
    return { key: cardKey(event.SessionStateChanged.instance_id), state: event.SessionStateChanged.to };
  if ("SessionCompleted" in event) return { key: cardKey(event.SessionCompleted.instance_id), state: "COMPLETED" };
  if ("SessionFailed" in event) return { key: cardKey(event.SessionFailed.instance_id), state: "FAILED" };
  if ("SessionCancelled" in event) return { key: cardKey(event.SessionCancelled.instance_id), state: "CANCELLED" };
  if ("ResourceEvicted" in event)
    return { key: cardKey(event.ResourceEvicted.instance_id), state: event.ResourceEvicted.terminal_state };
  if ("LeaseExpired" in event) return { key: cardKey(event.LeaseExpired.instance_id), state: "CANCELLED" };
  // SessionSpawned introduces a NEW card whose full data (provider/grouping) is
  // not in the event -> caller reconciles. ResourceAllocated/BreakerTripped/
  // SpawnRejected do not move a known card's column.
  return null;
}

/**
 * Subscribe to the live board stream. Calls `onDelta` for each pushed
 * `swarm://event`, and `onResync` when the backend signals dropped events.
 * Returns an unlisten function. Uses Tauri's event system (no polling).
 */
export async function subscribeBoardEvents(
  onDelta: (delta: SwarmBoardDelta) => void,
  onResync: (dropped: number) => void,
): Promise<() => void> {
  const { listen } = await import("@tauri-apps/api/event");
  const unDelta = await listen<SwarmBoardDelta>("swarm://event", (e) => onDelta(e.payload));
  const unResync = await listen<{ dropped: number }>("swarm://resync", (e) => onResync(e.payload.dropped));
  return () => {
    unDelta();
    unResync();
  };
}
