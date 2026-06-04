import { invoke } from "@tauri-apps/api/core";

// IPC bindings for the TRACK 1 calendar swarm-scheduler surface. These call the
// REAL managed `SwarmSchedulerState` backend commands (see
// `app/src-tauri/src/commands/swarm_schedule.rs`). No mocks: add arms a real
// tokio-cron job that, on fire, builds a SpawnRequest from the stored template
// and drives the live SwarmCoordinator; list/remove/export reflect the durable
// (JSON-persisted) schedule set that survives an app restart.
//
// NOTE: the backend commands are NOT yet registered in lib.rs (the Integrate
// phase owns the invoke handler + `.manage(SwarmSchedulerState)`). Calls will
// resolve once that wiring lands; the names/shapes here are the contract.

export type ScheduleProvider = "local" | "byok_cloud" | "official_cli";
export type ScheduleRuntimeBinding = "candle" | "llama_cpp";
export type ScheduleLocalExecutionMode = "cold" | "warm_vm";
export type ScheduleIsolationTier = "tier1_container" | "tier2_syscall" | "tier3_microvm";

/** WHAT a scheduled spin-up launches (the orchestrator's spawn template). */
export interface SpawnTemplate {
  provider: ScheduleProvider;
  /** Local: on-disk model artifact path (safetensors / GGUF). */
  artifactPath?: string;
  /** Local: expected sha256 hex of the artifact (integrity gate). */
  sha256Expected?: string;
  /** Local: candle | llama_cpp. */
  runtimeBinding?: ScheduleRuntimeBinding;
  /** Local: cold/default or explicit warm VM execution. */
  localExecutionMode?: ScheduleLocalExecutionMode;
  /** Cloud: allowlisted cloud model name (e.g. claude-sonnet-4). */
  cloudModelName?: string;
  /** Concurrent instance index of this model (default 0). */
  instance?: number;
  /** Worktree binding (board swimlane / per-worktree recovery). */
  worktreeId?: string;
  /** Operator-intended isolation tier; warm VM requires tier3_microvm. */
  isolationTier?: ScheduleIsolationTier;
  /** Optional local committed-memory estimate in bytes. */
  committedMemoryBytes?: number;
  /** Parent session id for ledger lineage. */
  parentSessionId?: string;
}

/** The action a schedule fires (externally-tagged by `kind`). */
export type ScheduledAction =
  | { kind: "spin_up"; swarmId: string; timeBoxSecs?: number }
  | { kind: "teardown"; swarmId: string };

/** Add-schedule request: the schedule (id + cron + summary + action) + template. */
export interface AddScheduleRequest {
  id: string;
  /** 6-field cron (`sec min hour dom mon dow`), interpreted in UTC. */
  cron: string;
  summary: string;
  action: ScheduledAction;
  template: SpawnTemplate;
}

/** A registered schedule row returned to the calendar view. */
export interface RegisteredScheduleRow {
  id: string;
  cron: string;
  summary: string;
  /** "spin_up" or "teardown". */
  actionKind: string;
  swarmId: string;
  timeBoxSecs: number | null;
  provider: string;
  artifactPath: string | null;
  cloudModelName: string | null;
  worktreeId: string | null;
  localExecutionMode: ScheduleLocalExecutionMode | null;
  isolationTier: ScheduleIsolationTier | null;
  committedMemoryBytes: number | null;
  registeredAt: string;
}

/**
 * Register a calendar schedule + its spawn template. Arms a real cron job on
 * the live scheduler and persists the set. Rejects a duplicate id or an invalid
 * cron expression (the error surfaces as the rejected promise).
 */
export async function addSchedule(request: AddScheduleRequest): Promise<void> {
  await invoke("kernel_swarm_schedule_add", { request });
}

/** List the registered calendar schedules (stable id order). */
export async function listSchedules(): Promise<RegisteredScheduleRow[]> {
  return await invoke<RegisteredScheduleRow[]>("kernel_swarm_schedule_list");
}

/** Remove a registered schedule by id (stops it firing + persists). */
export async function removeSchedule(scheduleId: string): Promise<void> {
  await invoke("kernel_swarm_schedule_remove", { scheduleId });
}

/**
 * Export the registered schedules as an RFC-5545 ICS calendar string the
 * operator can subscribe to in any calendar app (CalDAV / Google / Apple).
 */
export async function exportScheduleIcs(): Promise<string> {
  return await invoke<string>("kernel_swarm_schedule_export_ics");
}
