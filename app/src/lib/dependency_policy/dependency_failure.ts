// WP-KERNEL-009 / MT-020 + MT-031 — typed bundled-dependency failure surface.
//
// When a bundled dependency fails to initialize at runtime (Monaco worker
// construction, Tiptap extension init, module load), the failure must surface
// as a TYPED, user-visible diagnostic — never a blank screen or a silent
// console-only error. This module is the single registry those failures flow
// through; UI surfaces subscribe to it (DependencyFailureNotice component).

export type DependencyFailurePhase =
  | "module_load"
  | "worker_construction"
  | "extension_init"
  | "editor_mount";

export interface DependencyFailure {
  /** Bundled dependency that failed, e.g. "monaco-editor", "@tiptap/extension-table". */
  dependency: string;
  /** Component within the dependency, e.g. "worker:typescript", "extension:table". */
  component: string;
  phase: DependencyFailurePhase;
  /** Human-readable, user-facing message (stable prefix for tests). */
  message: string;
  /** Underlying error message, when available. */
  cause?: string;
  occurred_at: string;
}

export type DependencyFailureListener = (failure: DependencyFailure) => void;

/** In-memory registry of bundled-dependency failures (per window). */
export class DependencyFailureRegistry {
  private failures: readonly DependencyFailure[] = [];
  private listeners = new Set<DependencyFailureListener>();

  report(failure: Omit<DependencyFailure, "occurred_at">): DependencyFailure {
    const entry: DependencyFailure = {
      ...failure,
      occurred_at: new Date().toISOString(),
    };
    this.failures = [...this.failures, entry];
    for (const listener of this.listeners) {
      try {
        listener(entry);
      } catch {
        // A faulty listener must never mask the original dependency failure.
      }
    }
    return entry;
  }

  list(): readonly DependencyFailure[] {
    return this.failures;
  }

  subscribe(listener: DependencyFailureListener): () => void {
    this.listeners.add(listener);
    return () => this.listeners.delete(listener);
  }

  clear(): void {
    this.failures = [];
  }
}

/** Application-wide registry instance. */
export const dependencyFailures = new DependencyFailureRegistry();

/** Builds the stable user-facing message for a bundled-dependency failure. */
export function formatDependencyFailureMessage(
  failure: Pick<DependencyFailure, "dependency" | "component" | "phase">,
): string {
  return `Bundled dependency failed to load: ${failure.dependency} (${failure.component}, ${failure.phase}). The editor runs in degraded mode; no external download is attempted because all assets ship inside Handshake.`;
}
