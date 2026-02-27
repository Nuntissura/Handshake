import { AiJob, asFemsJobOutput, FemsJobOutput, getJob, isFemsProtocolId } from "../lib/api";

export type AiJobTrackerEntry = {
  jobId: string;
  jobKind: string;
  protocolId: string;
  docId: string;
  docTitle?: string;
  createdAt: number;
};

export type AiJobTrackerSnapshot = {
  entries: AiJobTrackerEntry[];
  jobsById: Record<string, AiJob | undefined>;
  errorsById: Record<string, string | undefined>;
};

type Listener = (snapshot: AiJobTrackerSnapshot) => void;

const STORAGE_KEY = "handshake.aiJobs.v1";
const MAX_ENTRIES = 50;
const POLL_INTERVAL_MS = 2000;

let pollingTimer: number | null = null;
let entries: AiJobTrackerEntry[] = loadEntries();
let jobsById: Record<string, AiJob | undefined> = {};
let errorsById: Record<string, string | undefined> = {};
const listeners = new Set<Listener>();

function safeReadStorage(key: string): string | null {
  try {
    if (typeof window === "undefined" || !window.localStorage) return null;
    return window.localStorage.getItem(key);
  } catch {
    return null;
  }
}

function safeWriteStorage(key: string, value: string) {
  try {
    if (typeof window === "undefined" || !window.localStorage) return;
    window.localStorage.setItem(key, value);
  } catch {
    // ignore persistence failures
  }
}

function persistEntries(nextEntries: AiJobTrackerEntry[]) {
  safeWriteStorage(STORAGE_KEY, JSON.stringify(nextEntries));
}

function isValidEntry(value: unknown): value is AiJobTrackerEntry {
  if (!value || typeof value !== "object") return false;
  const obj = value as Record<string, unknown>;
  return (
    typeof obj.jobId === "string" &&
    typeof obj.jobKind === "string" &&
    typeof obj.protocolId === "string" &&
    typeof obj.docId === "string" &&
    typeof obj.createdAt === "number"
  );
}

function loadEntries(): AiJobTrackerEntry[] {
  const raw = safeReadStorage(STORAGE_KEY);
  if (!raw) return [];
  try {
    const parsed = JSON.parse(raw);
    if (!Array.isArray(parsed)) return [];
    const out = parsed.filter(isValidEntry);
    const deduped = new Map<string, AiJobTrackerEntry>();
    for (const entry of out) {
      deduped.set(entry.jobId, entry);
    }
    return Array.from(deduped.values())
      .sort((a, b) => b.createdAt - a.createdAt)
      .slice(0, MAX_ENTRIES);
  } catch {
    return [];
  }
}

function notify() {
  const snapshot = getSnapshot();
  listeners.forEach((listener) => listener(snapshot));
}

export function getSnapshot(): AiJobTrackerSnapshot {
  return { entries, jobsById, errorsById };
}

export function subscribe(listener: Listener) {
  listeners.add(listener);
  listener(getSnapshot());
  return () => {
    listeners.delete(listener);
  };
}

export function addJob(entry: AiJobTrackerEntry) {
  entries = [entry, ...entries.filter((e) => e.jobId !== entry.jobId)]
    .sort((a, b) => b.createdAt - a.createdAt)
    .slice(0, MAX_ENTRIES);
  persistEntries(entries);
  notify();
}

export function removeJob(jobId: string) {
  entries = entries.filter((e) => e.jobId !== jobId);
  delete jobsById[jobId];
  delete errorsById[jobId];
  persistEntries(entries);
  notify();
}

export function startPolling() {
  if (pollingTimer !== null) return;
  if (typeof window === "undefined") return;
  const tick = async () => {
    const ids = entries.map((e) => e.jobId);
    const toFetch = ids.filter((jobId) => {
      const job = jobsById[jobId];
      if (!job) return true;
      return job.state === "queued" || job.state === "running";
    });

    if (toFetch.length === 0) return;

    const results = await Promise.allSettled(toFetch.map((jobId) => getJob(jobId)));
    let changed = false;
    const removedJobIds: string[] = [];

    for (let idx = 0; idx < toFetch.length; idx += 1) {
      const jobId = toFetch[idx]!;
      const result = results[idx]!;

      if (result.status === "fulfilled") {
        jobsById = { ...jobsById, [jobId]: result.value };
        if (errorsById[jobId]) {
          errorsById = { ...errorsById, [jobId]: undefined };
        }
        changed = true;
      } else {
        const message = result.reason instanceof Error ? result.reason.message : String(result.reason);
        if (/Request failed:\\s*404\\b/.test(message)) {
          removedJobIds.push(jobId);
          changed = true;
        } else {
          errorsById = { ...errorsById, [jobId]: message };
          changed = true;
        }
      }
    }

    if (removedJobIds.length > 0) {
      const removed = new Set(removedJobIds);
      entries = entries.filter((entry) => !removed.has(entry.jobId));
      for (const jobId of removedJobIds) {
        delete jobsById[jobId];
        delete errorsById[jobId];
      }
      persistEntries(entries);
    }

    if (changed) notify();
  };

  void tick();
  pollingTimer = window.setInterval(() => {
    void tick();
  }, POLL_INTERVAL_MS);
}

export function isFemsJob(job: AiJob): boolean {
  return isFemsProtocolId(job.protocol_id);
}

export function femsOutputForJob(job: AiJob): FemsJobOutput | null {
  return asFemsJobOutput(job.job_outputs);
}
