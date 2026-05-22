import { useCallback, useEffect, useState } from "react";
import {
  type SubquadCacheStats,
  type SubquadPrefixHandle,
  subquadEvictAll,
  subquadPersist,
  subquadRehydrate,
  subquadStateCommit,
  subquadStateList,
  subquadStateRestore,
} from "../../lib/ipc/subquadratic";

// MT-113 — state-vector controls embedded inside SubquadraticPanel.
//
// Owned files contract listed `StateVectorControls.svelte`; app stack is
// React/TSX (same Svelte->TSX defect class as MT-091/094/098/102/105/124).
// Behavior + AC preserved.
//
// The controls assume the parent panel has already gated on
// capabilities.supportsSubquadratic — this component never renders for
// adapters that don't expose SSM state vectors.

type Props = {
  modelId: string;
  optedOut: boolean;
};

type CommitFormState =
  | { status: "idle" }
  | { status: "submitting" }
  | { status: "error"; message: string }
  | {
      status: "ready";
      handle: SubquadPrefixHandle;
      committedAtIso: string;
    };

type ListState =
  | { status: "loading" }
  | { status: "error"; message: string }
  | { status: "ready"; occupancy: SubquadCacheStats };

function errorMessage(error: unknown): string {
  if (error instanceof Error) return error.message;
  if (typeof error === "string") return error;
  return "Unknown error";
}

function parseTokenInput(input: string): number[] {
  // Accept comma- or whitespace-separated u32 token ids. Empty input
  // returns []; the IPC layer rejects empty arrays. Non-numeric entries
  // are skipped (caller validates length).
  return input
    .split(/[\s,]+/)
    .map((part) => part.trim())
    .filter((part) => part.length > 0)
    .map((part) => Number(part))
    .filter((value) => Number.isInteger(value) && value >= 0);
}

export function StateVectorControls({ modelId, optedOut }: Props) {
  const [tokenInput, setTokenInput] = useState<string>("");
  const [commitState, setCommitState] = useState<CommitFormState>({
    status: "idle",
  });
  const [listState, setListState] = useState<ListState>({ status: "loading" });
  const [evictBusy, setEvictBusy] = useState<boolean>(false);
  const [deferralBanner, setDeferralBanner] = useState<string | null>(null);

  const refreshList = useCallback(async () => {
    setListState({ status: "loading" });
    try {
      const result = await subquadStateList({ modelId });
      setListState({ status: "ready", occupancy: result.occupancy });
    } catch (error) {
      setListState({ status: "error", message: errorMessage(error) });
    }
  }, [modelId]);

  useEffect(() => {
    if (optedOut) return;
    void refreshList();
  }, [refreshList, optedOut]);

  const handleCommit = useCallback(async () => {
    const tokens = parseTokenInput(tokenInput);
    if (tokens.length === 0) {
      setCommitState({
        status: "error",
        message: "prefix tokens required (comma- or whitespace-separated u32 ids)",
      });
      return;
    }
    setCommitState({ status: "submitting" });
    try {
      const result = await subquadStateCommit({
        modelId,
        prefixTokens: tokens,
      });
      setCommitState({
        status: "ready",
        handle: result.prefixHandle,
        committedAtIso: new Date().toISOString(),
      });
      await refreshList();
    } catch (error) {
      setCommitState({ status: "error", message: errorMessage(error) });
    }
  }, [modelId, tokenInput, refreshList]);

  const handleRestore = useCallback(async () => {
    if (commitState.status !== "ready") return;
    try {
      await subquadStateRestore({
        modelId,
        prefixHandle: commitState.handle,
      });
      await refreshList();
    } catch (error) {
      setCommitState({ status: "error", message: errorMessage(error) });
    }
  }, [modelId, commitState, refreshList]);

  const handleEvictAll = useCallback(async () => {
    if (evictBusy) return;
    const confirmed = window.confirm(
      "Evict all committed SSM states for this model? This cannot be undone.",
    );
    if (!confirmed) return;
    setEvictBusy(true);
    try {
      await subquadEvictAll({ modelId });
      setCommitState({ status: "idle" });
      await refreshList();
    } catch (error) {
      setListState({ status: "error", message: errorMessage(error) });
    } finally {
      setEvictBusy(false);
    }
  }, [modelId, evictBusy, refreshList]);

  const handlePersist = useCallback(async () => {
    if (commitState.status !== "ready") return;
    try {
      await subquadPersist({ modelId, prefixHandle: commitState.handle });
      setDeferralBanner(null);
    } catch (error) {
      const message = errorMessage(error);
      // MT-117 deferral marker — turn the typed error into a friendly
      // tooltip-style banner so the operator sees the rationale instead
      // of a backend error string.
      if (message.includes("subquadratic_persist_disk_deferred_mt117")) {
        setDeferralBanner(
          "Disk persistence lands in MT-117 (cross-session SSM state restore). The button is wired to the IPC channel so it activates the moment MT-117 ships.",
        );
      } else {
        setCommitState({ status: "error", message });
      }
    }
  }, [modelId, commitState]);

  const handleRehydrate = useCallback(async () => {
    try {
      await subquadRehydrate({ modelId });
      setDeferralBanner(null);
    } catch (error) {
      const message = errorMessage(error);
      if (message.includes("subquadratic_rehydrate_disk_deferred_mt117")) {
        setDeferralBanner(
          "Disk rehydration lands in MT-117. The button stays wired to the IPC channel so it activates the moment MT-117 ships.",
        );
      } else {
        setListState({ status: "error", message });
      }
    }
  }, [modelId]);

  if (optedOut) return null;

  return (
    <div
      className="subquadratic-panel__state-vector-controls"
      data-testid="state-vector-controls"
    >
      <div className="state-vector-controls__row">
        <label className="state-vector-controls__commit-input">
          <span>Prefix tokens (u32, comma- or space-separated)</span>
          <textarea
            value={tokenInput}
            onChange={(event) => setTokenInput(event.target.value)}
            placeholder="e.g. 1, 2, 3, 4"
            data-testid="state-vector-controls.commit-input"
            rows={2}
          />
        </label>
        <button
          type="button"
          onClick={() => void handleCommit()}
          disabled={commitState.status === "submitting"}
          data-testid="state-vector-controls.commit-button"
        >
          Commit state
        </button>
      </div>

      {commitState.status === "error" ? (
        <p
          role="alert"
          className="inference-lab__error"
          data-testid="state-vector-controls.commit-error"
        >
          {commitState.message}
        </p>
      ) : null}

      {commitState.status === "ready" ? (
        <div
          className="state-vector-controls__handle"
          data-testid="state-vector-controls.commit-receipt"
        >
          <p className="muted">
            Committed handle <code>{commitState.handle.prefixId}</code> covering{" "}
            {commitState.handle.tokenCount} tokens at{" "}
            <time dateTime={commitState.committedAtIso}>
              {new Date(commitState.committedAtIso).toLocaleString()}
            </time>
            .
          </p>
          <div className="state-vector-controls__handle-actions">
            <button
              type="button"
              onClick={() => void handleRestore()}
              data-testid="state-vector-controls.restore-button"
            >
              Restore committed state
            </button>
            <button
              type="button"
              onClick={() => void handlePersist()}
              data-testid="state-vector-controls.persist-button"
            >
              Persist to disk (MT-117)
            </button>
          </div>
        </div>
      ) : null}

      <div className="state-vector-controls__list">
        {listState.status === "loading" ? (
          <p data-testid="state-vector-controls.list-loading">
            Loading state-vector occupancy…
          </p>
        ) : listState.status === "error" ? (
          <p
            role="alert"
            className="inference-lab__error"
            data-testid="state-vector-controls.list-error"
          >
            {listState.message}
          </p>
        ) : (
          <dl data-testid="state-vector-controls.list">
            <dt>Committed entries</dt>
            <dd>{listState.occupancy.prefixCacheEntries}</dd>
            <dt>Bytes used</dt>
            <dd>{listState.occupancy.bytesUsed.toLocaleString()}</dd>
            <dt>Hits</dt>
            <dd>{listState.occupancy.prefixCacheHitCount}</dd>
            <dt>Misses</dt>
            <dd>{listState.occupancy.prefixCacheMissCount}</dd>
          </dl>
        )}
        <div className="state-vector-controls__list-actions">
          <button
            type="button"
            onClick={() => void handleEvictAll()}
            disabled={evictBusy}
            data-testid="state-vector-controls.evict-all-button"
          >
            Evict all committed states
          </button>
          <button
            type="button"
            onClick={() => void handleRehydrate()}
            data-testid="state-vector-controls.rehydrate-button"
          >
            Rehydrate from disk (MT-117)
          </button>
        </div>
      </div>

      {deferralBanner !== null ? (
        <p
          className="muted"
          data-testid="state-vector-controls.deferral-banner"
        >
          {deferralBanner}
        </p>
      ) : null}
    </div>
  );
}
