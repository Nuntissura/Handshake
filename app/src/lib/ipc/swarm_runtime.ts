import { invoke } from "@tauri-apps/api/core";

// IPC bindings for the multi-model SWARM operator surface (MT-205). These call
// the REAL managed SwarmCoordinator backend commands registered in
// `app/src-tauri/src/lib.rs`. No mocks: spawn loads a real model session, chat
// runs a real generate, list/snapshot reflect the live coordinator registry.

export type SwarmProvider = "local" | "byok_cloud" | "official_cli";
export type ByokCloudProvider = "anthropic" | "openai";
export type SwarmRuntimeBinding = "candle" | "llama_cpp";

/**
 * Recorded-only isolation tier the operator INTENDS for a session. Mirrors the
 * core `IsolationTier` vocabulary. Tier3 is load-bearing for local llama.cpp
 * spawns when the backend sandbox registry is wired; other provider/runtime
 * combinations remain attribution.
 */
export type SwarmIsolationTier = "tier1_container" | "tier2_syscall" | "tier3_microvm";

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
  /** BYOK cloud only: exact Anthropic/OpenAI lane when selected. */
  byokCloudProvider?: ByokCloudProvider;
  /** Concurrent instance index of this model (default 0). */
  instance?: number;
  /** Parent session id for ledger lineage. */
  parentSessionId?: string;
  /** Optional swarm grouping id for board/Terminal/orchestration attribution. */
  swarmId?: string;
  /**
   * Optional VM/sandbox worktree this session is ASSIGNED to. Drives the board
   * swimlane + per-worktree recovery grouping. Blank/whitespace => omit (the
   * backend records `None` = honest "unassigned").
   */
  worktreeId?: string;
  /**
   * Optional on-disk place the operator assigns the session to (absolute OR
   * repo-relative; disk-agnostic). Recorded verbatim — never resolved, created,
   * or used as a real cwd here. Blank/whitespace => omit.
   */
  workingDir?: string;
  /**
   * Optional recorded-only isolation tier (see {@link SwarmIsolationTier}).
   * Tier3 is enforced for local llama.cpp spawns when the backend sandbox
   * registry is wired; other combinations remain attribution.
   */
  isolationTier?: SwarmIsolationTier;
  /**
   * Optional local committed-memory estimate in bytes. The backend reserves this
   * only for local model/VM spawns; cloud requests ignore it so cloud fallback can
   * escape local host-memory pressure.
   */
  committedMemoryBytes?: number;
}

export interface SwarmSpawnAttempt {
  provider: string;
  instanceId: SwarmInstanceId | null;
  error: string | null;
}

export interface SwarmLocalCloudPairRequest {
  local: SwarmSpawnRequest;
  cloud: SwarmSpawnRequest;
}

export interface SwarmLocalCloudPairResult {
  local: SwarmSpawnAttempt;
  cloud: SwarmSpawnAttempt;
}

export interface SwarmCloudEscalationRequest {
  local: SwarmSpawnRequest;
  cloud: SwarmSpawnRequest;
}

export interface SwarmCloudEscalationResult {
  selected: "local" | "cloud" | null;
  escalated: boolean;
  escalationReason: string | null;
  local: SwarmSpawnAttempt;
  cloud: SwarmSpawnAttempt | null;
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
  /** Assigned VM/sandbox worktree (board swimlane + recovery grouping), or null. */
  worktreeId: string | null;
  /** Recorded on-disk working dir attribution, or null. Never executed in. */
  workingDir: string | null;
}

/**
 * A worktree the operator can assign sessions to, surfaced for discovery so the
 * spawn form can offer existing worktrees alongside a free-text "new" entry.
 */
export interface SwarmWorktree {
  worktreeId: string;
  /** Count of LIVE sessions currently assigned to this worktree. */
  liveSessionCount: number;
}

export interface SwarmResourceSnapshot {
  concurrencyCap: number;
  concurrencyInUse: number;
  concurrencyAvailable: number;
  liveSessions: number;
  lifetimeSpawnsRemaining: number;
  tokensRemaining: number | null;
  costMicrosRemaining: number | null;
  committedMemoryBytesRemaining?: number | null;
  committedMemoryBytesCap?: number | null;
  budgetExhausted: boolean;
}

export interface SwarmChatResponse {
  text: string;
  tokenCount: number;
  finishReason: string | null;
}

export type SwarmEscalationTaskClass =
  | "routine"
  | "classification"
  | "hard_reasoning"
  | "force_cloud"
  | "force_local";

export interface SwarmChatGenerateWithCloudEscalationRequest {
  localInstanceId: string;
  prompt: string;
  cloud: SwarmSpawnRequest;
  taskClass?: SwarmEscalationTaskClass;
}

export interface SwarmChatGenerateWithCloudEscalationResponse {
  selected: "local" | "cloud" | null;
  escalated: boolean;
  escalationReason: string | null;
  localError: string | null;
  local: SwarmChatResponse | null;
  cloudInstance: SwarmInstanceId | null;
  cloud: SwarmChatResponse | null;
  cloudError: string | null;
}

export async function spawnSession(request: SwarmSpawnRequest): Promise<SwarmInstanceId> {
  return await invoke<SwarmInstanceId>("kernel_swarm_spawn_session", { request });
}

export async function spawnLocalCloudPair(
  request: SwarmLocalCloudPairRequest,
): Promise<SwarmLocalCloudPairResult> {
  return await invoke<SwarmLocalCloudPairResult>("kernel_swarm_spawn_local_cloud_pair", {
    request,
  });
}

export async function spawnWithCloudEscalation(
  request: SwarmCloudEscalationRequest,
): Promise<SwarmCloudEscalationResult> {
  return await invoke<SwarmCloudEscalationResult>(
    "kernel_swarm_spawn_with_cloud_escalation",
    { request },
  );
}

// ---------------------------------------------------------------------------
// ROI #3 STATE RECOVERY (resume-from-session).
//
// At spawn time the backend persists a resume-complete spawn template keyed by
// the session's composite instance_id (mirrors the calendar scheduler's
// SpawnTemplate + atomic JSON store). The operator can then RESUME a recorded
// session: re-spawn a FRESH session carrying the original's captured config
// (provider/model/artifact/sha/worktree/working_dir/isolation), driving the SAME
// validated spawn path — the integrity sha gate and coordinator bounds are
// preserved, and lineage (parent = the resumed-from session) is recorded in the
// real FR SessionSpawned event. Two thin read paths over the same store:
//   - resumeSession      => one-click "continue as-is".
//   - getSpawnTemplate   => prefill the spawn form for edit-then-resume.
// ---------------------------------------------------------------------------

/**
 * The persisted resume template for a session (camelCase IPC DTO; mirrors the
 * backend `SessionSpawnTemplateIpc` serde). Captured at spawn time so the exact
 * session can be reconstructed later. Cloud sessions carry only the model NAME
 * (never the BYOK key — keys stay in the OS keychain vault, re-resolved on
 * resume); local sessions carry path + sha (the integrity gate is re-applied on
 * resume). `originSessionId` is the lineage root the resumed-from session id.
 */
export interface SessionSpawnTemplate {
  provider: SwarmProvider;
  artifactPath?: string | null;
  sha256Expected?: string | null;
  runtimeBinding?: SwarmRuntimeBinding | null;
  cloudModelName?: string | null;
  byokCloudProvider?: ByokCloudProvider | null;
  swarmId?: string | null;
  worktreeId?: string | null;
  workingDir?: string | null;
  isolationTier?: SwarmIsolationTier | null;
  committedMemoryBytes?: number | null;
  originSessionId: string;
  capturedAt: string;
}

/**
 * Resume a recorded session: re-spawn a FRESH session from its stored template.
 * Drives the SAME validated `kernel_swarm_spawn_session` backend path (a new
 * model id is minted; lineage parent = `sessionId`). Returns the new session's
 * instance id. Rejects with a verbatim `SESSION_NOT_RESUMABLE:` error if no
 * template is stored for `sessionId` (e.g. a chat session, a pre-feature spawn,
 * or a template GC'd between list and click). The UI gates on
 * `SessionSummary.resumable` BEFORE calling, and surfaces this error verbatim if
 * a race slips through.
 */
export async function resumeSession(sessionId: string): Promise<SwarmInstanceId> {
  return await invoke<SwarmInstanceId>("kernel_swarm_resume_session", { sessionId });
}

/**
 * Read the stored spawn template for a session, or null when it is not
 * resumable. Used by the edit-then-resume path to PREFILL the existing spawn
 * form (so the operator can repoint a moved artifact, change worktree, etc.
 * before spawning) — reusing the whole spawn form + its validation.
 */
export async function getSpawnTemplate(
  sessionId: string,
): Promise<SessionSpawnTemplate | null> {
  return await invoke<SessionSpawnTemplate | null>("kernel_swarm_get_spawn_template", {
    sessionId,
  });
}

export async function cancelSession(instanceId: string): Promise<void> {
  await invoke("kernel_swarm_cancel_session", { instanceId });
}

export async function listActiveSessions(): Promise<SwarmSession[]> {
  return await invoke<SwarmSession[]>("kernel_swarm_list_active_sessions");
}

/**
 * Distinct worktrees currently in use across live sessions (for the spawn-form
 * worktree picker). The form ALSO always offers a free-text "new worktree"
 * entry, so an empty/stale discovery list never blocks assigning a brand-new
 * worktree.
 */
export async function listWorktrees(): Promise<SwarmWorktree[]> {
  return await invoke<SwarmWorktree[]>("kernel_swarm_list_worktrees");
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

export async function chatGenerateWithCloudEscalation(
  request: SwarmChatGenerateWithCloudEscalationRequest,
): Promise<SwarmChatGenerateWithCloudEscalationResponse> {
  return await invoke<SwarmChatGenerateWithCloudEscalationResponse>(
    "kernel_swarm_chat_generate_with_cloud_escalation",
    { request },
  );
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
  | {
      SessionSpawned: {
        instance_id: RawInstanceId;
        parent_session_id: string;
        process_uuid: string;
        swarm_id?: string | null;
        worktree_id?: string | null;
      };
    }
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
 * The composite `<model_id>#<instance>` of the session a SwarmEvent concerns, or
 * null for events that carry no instance id (e.g. BreakerTripped). This is the
 * SAME key the session_transcript aggregator uses as the session_id for a
 * swarm/agent session, so the Session Replay live tail can correlate a pushed
 * `swarm://event` to the focused session WITHOUT a backend lookup.
 *
 * Distinct from {@link eventTargetState}: that returns null for events that do
 * not move a board column (SessionSpawned/ResourceAllocated/SpawnRejected/
 * LeaseExpired's allocation events) even though they DO carry an instance id.
 * The live tail only needs "is this event about the focused session?", so we
 * extract the instance id from EVERY instance-bearing variant.
 */
export function eventInstanceKey(event: SwarmEvent): string | null {
  if ("SessionSpawned" in event) return cardKey(event.SessionSpawned.instance_id);
  if ("SessionReady" in event) return cardKey(event.SessionReady.instance_id);
  if ("SessionStateChanged" in event) return cardKey(event.SessionStateChanged.instance_id);
  if ("SessionCancelled" in event) return cardKey(event.SessionCancelled.instance_id);
  if ("SessionCompleted" in event) return cardKey(event.SessionCompleted.instance_id);
  if ("SessionFailed" in event) return cardKey(event.SessionFailed.instance_id);
  if ("ResourceAllocated" in event) return cardKey(event.ResourceAllocated.instance_id);
  if ("ResourceEvicted" in event) return cardKey(event.ResourceEvicted.instance_id);
  if ("LeaseExpired" in event) return cardKey(event.LeaseExpired.instance_id);
  if ("SpawnRejected" in event) return cardKey(event.SpawnRejected.instance_id);
  // BreakerTripped carries only a signature, no instance id.
  return null;
}

/**
 * Whether a SwarmEvent moves the focused session to a TERMINAL lifecycle state
 * (so the live tail can flip to idle — no more streaming for an ended session).
 * Returns the terminal state for the concerned instance, or null otherwise.
 */
export function eventTerminalState(event: SwarmEvent): { key: string; state: string } | null {
  if ("SessionCompleted" in event) return { key: cardKey(event.SessionCompleted.instance_id), state: "COMPLETED" };
  if ("SessionFailed" in event) return { key: cardKey(event.SessionFailed.instance_id), state: "FAILED" };
  if ("SessionCancelled" in event) return { key: cardKey(event.SessionCancelled.instance_id), state: "CANCELLED" };
  if ("LeaseExpired" in event) return { key: cardKey(event.LeaseExpired.instance_id), state: "CANCELLED" };
  if ("ResourceEvicted" in event)
    return { key: cardKey(event.ResourceEvicted.instance_id), state: event.ResourceEvicted.terminal_state };
  if ("SessionStateChanged" in event) {
    const to = event.SessionStateChanged.to;
    if (to === "COMPLETED" || to === "FAILED" || to === "CANCELLED")
      return { key: cardKey(event.SessionStateChanged.instance_id), state: to };
  }
  return null;
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
