// MT-124: TypeScript IPC wrappers for the Distillation Queue Tauri
// commands. The wrappers mirror the camelCase IPC DTOs defined in
// `app/src-tauri/src/commands/distillation.rs` and the
// `DistillationQueue.tsx` component prop shapes.
//
// Each list wrapper returns a real Tauri `invoke()` call into the
// Rust backend, which reads through the production stores
// (`SessionFlagStore::list_opted_in`, `CandidateRegistry::list_pending`,
// `FlightRecorder::list_events` filtered to FR-EVT-DISTILL-*). The
// `InferenceLab.tsx` parent calls these on mount and passes the
// resulting arrays to the `DistillationQueue` component.

import { invoke } from "@tauri-apps/api/core";

export interface OptedInSession {
  sessionId: string;
  /// Best-effort field: the in-memory `SessionFlagStore` does not yet
  /// carry the model_id. The Tauri command returns `"unknown"` until
  /// the Postgres `governed_sessions` follow-on attaches the column;
  /// the UI surfaces the raw value as-is.
  modelId: string;
  closedAtUtc: string;
  /// Best-effort field; `0` until the Postgres `governed_sessions`
  /// follow-on attaches turn counts.
  turnCount: number;
}

export type ReviewStatus = "Pending" | "Promoted" | "Rejected";

export interface PendingCandidate {
  loraId: string;
  teacherModelPath: string;
  studentBaseModelPath: string;
  corpusTurnCount: number;
  trainedAtUtc: string;
  licenseTag: string;
  status: ReviewStatus;
  rejectionReason: string | null;
}

export type TrainingJobStatus = "queued" | "running" | "done" | "error";

export interface TrainingJobSummary {
  jobId: string;
  sessionId: string;
  status: TrainingJobStatus;
  queuedAtUtc: string;
  startedAtUtc: string | null;
  finishedAtUtc: string | null;
  errorMessage: string | null;
}

export interface PromoteCandidateRequest {
  loraId: string;
  operatorSignature: string;
}

export interface RejectCandidateRequest {
  loraId: string;
  operatorSignature: string;
  reason: string;
}

export interface CandidateActionReceipt {
  loraId: string;
  newStatus: ReviewStatus;
  eventType: string;
}

export interface ExtractCorpusRequest {
  sessionId: string;
}

export interface ExtractCorpusReceipt {
  sessionId: string;
  status: string;
  eventType: string;
}

export async function listDistillSessions(): Promise<OptedInSession[]> {
  return await invoke<OptedInSession[]>("list_distill_sessions");
}

export async function listDistillCandidates(): Promise<PendingCandidate[]> {
  return await invoke<PendingCandidate[]>("list_distill_candidates");
}

export async function listDistillJobs(): Promise<TrainingJobSummary[]> {
  return await invoke<TrainingJobSummary[]>("list_distill_jobs");
}

export async function extractDistillCorpus(
  request: ExtractCorpusRequest,
): Promise<ExtractCorpusReceipt> {
  return await invoke<ExtractCorpusReceipt>("extract_distill_corpus", {
    request,
  });
}

export async function promoteDistillCandidate(
  request: PromoteCandidateRequest,
): Promise<CandidateActionReceipt> {
  return await invoke<CandidateActionReceipt>("promote_distill_candidate", {
    request,
  });
}

export async function rejectDistillCandidate(
  request: RejectCandidateRequest,
): Promise<CandidateActionReceipt> {
  return await invoke<CandidateActionReceipt>("reject_distill_candidate", {
    request,
  });
}
