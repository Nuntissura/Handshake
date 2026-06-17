// WP-KERNEL-009 / MT-260 — AI Loom job review panel (GAP-LM-011).
//
// Operator-facing surface for the AI Loom jobs: run an auto-tag / auto-caption
// / link-suggest job over selected LoomBlocks, then review the resulting
// PENDING proposals grouped by kind. Every suggestion is confirm-to-promote:
// per-item Accept / Reject, plus Accept-all-of-kind. The backend enforces
// per-item authority (a non-operator promotes nothing) and writes the real
// edge/derived field + receipts only on accept — this panel is a projection of
// that authority, never a parallel store.
//
// data-testid surfaces (for offline Playwright + vitest):
//   - loom-ai-review-panel            (root)
//   - loom-ai-run-<kind>              (run a job of that kind)
//   - loom-ai-empty                   (no suggestions)
//   - loom-ai-group-<kind>           (group header per kind)
//   - loom-ai-suggestion-<id>        (one suggestion row)
//   - loom-ai-accept-<id> / loom-ai-reject-<id>
//   - loom-ai-accept-all-<kind>
//   - loom-ai-error                   (role=alert)
//   - loom-ai-no-model                (typed no-model decline)

import { useCallback, useEffect, useMemo, useState } from "react";
import {
  acceptAllLoomAiSuggestions,
  acceptLoomAiSuggestion,
  listLoomAiSuggestions,
  queryLoomView,
  rejectLoomAiSuggestion,
  runLoomAiJob,
  ApiRequestError,
  type LoomAiJobKind,
  type LoomAiReviewerContext,
  type LoomAiSuggestion,
} from "../lib/api";

type Props = {
  workspaceId: string;
  /**
   * The block ids a job runs over. When empty, the panel auto-loads the
   * workspace's blocks (the "all" view) so the operator can run a job without
   * a pre-selection.
   */
  blockIds?: string[];
  /** Optional candidate tag names seeding auto_tag prompts. */
  tagCandidates?: string[];
  /** Reviewer identity (defaults to operator). */
  reviewer?: LoomAiReviewerContext;
  onClose?: () => void;
};

const KIND_LABELS: Record<LoomAiJobKind, string> = {
  auto_tag: "Auto-tag",
  auto_caption: "Auto-caption",
  link_suggest: "Link suggestions",
};

const KIND_ORDER: LoomAiJobKind[] = ["auto_tag", "auto_caption", "link_suggest"];

function describeSuggestion(s: LoomAiSuggestion): string {
  const value = s.suggested_value ?? {};
  if (s.kind === "auto_tag") return `#${String(value.tag ?? "")}`;
  if (s.kind === "auto_caption") return String(value.caption ?? "");
  if (s.kind === "link_suggest")
    return `→ ${s.target_block_id ?? "(unknown)"} — ${String(value.reason ?? "")}`;
  return JSON.stringify(value);
}

export function LoomAiReviewPanel({
  workspaceId,
  blockIds,
  tagCandidates = [],
  reviewer,
  onClose,
}: Props) {
  const [suggestions, setSuggestions] = useState<LoomAiSuggestion[]>([]);
  const [resolvedBlockIds, setResolvedBlockIds] = useState<string[]>(blockIds ?? []);
  const [loading, setLoading] = useState(false);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [noModel, setNoModel] = useState(false);

  // Auto-load workspace blocks when no explicit selection was passed.
  useEffect(() => {
    if (blockIds && blockIds.length > 0) {
      setResolvedBlockIds(blockIds);
      return;
    }
    let cancelled = false;
    queryLoomView(workspaceId, "all", { limit: 50 })
      .then((view) => {
        if (cancelled) return;
        const ids = "blocks" in view ? view.blocks.map((b) => b.block_id) : [];
        setResolvedBlockIds(ids);
      })
      .catch(() => {
        if (!cancelled) setResolvedBlockIds([]);
      });
    return () => {
      cancelled = true;
    };
  }, [workspaceId, blockIds]);

  const refresh = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const rows = await listLoomAiSuggestions(workspaceId, { state: "pending" });
      setSuggestions(rows);
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to load suggestions");
    } finally {
      setLoading(false);
    }
  }, [workspaceId]);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  const runJob = useCallback(
    async (kind: LoomAiJobKind) => {
      setBusy(true);
      setError(null);
      setNoModel(false);
      try {
        await runLoomAiJob(workspaceId, {
          kind,
          block_ids: resolvedBlockIds,
          tag_candidates: tagCandidates,
        });
        await refresh();
      } catch (err) {
        if (err instanceof ApiRequestError && err.status === 409 && err.body.includes("NO-MODEL")) {
          setNoModel(true);
        } else {
          setError(err instanceof Error ? err.message : "Job failed");
        }
      } finally {
        setBusy(false);
      }
    },
    [workspaceId, resolvedBlockIds, tagCandidates, refresh],
  );

  const accept = useCallback(
    async (id: string) => {
      setBusy(true);
      setError(null);
      try {
        await acceptLoomAiSuggestion(workspaceId, id, reviewer);
        await refresh();
      } catch (err) {
        setError(err instanceof Error ? err.message : "Accept failed");
      } finally {
        setBusy(false);
      }
    },
    [workspaceId, reviewer, refresh],
  );

  const reject = useCallback(
    async (id: string) => {
      setBusy(true);
      setError(null);
      try {
        await rejectLoomAiSuggestion(workspaceId, id, reviewer);
        await refresh();
      } catch (err) {
        setError(err instanceof Error ? err.message : "Reject failed");
      } finally {
        setBusy(false);
      }
    },
    [workspaceId, reviewer, refresh],
  );

  const acceptAll = useCallback(
    async (kind: LoomAiJobKind, jobId: string) => {
      setBusy(true);
      setError(null);
      try {
        await acceptAllLoomAiSuggestions(workspaceId, jobId, kind, reviewer);
        await refresh();
      } catch (err) {
        setError(err instanceof Error ? err.message : "Accept-all failed");
      } finally {
        setBusy(false);
      }
    },
    [workspaceId, reviewer, refresh],
  );

  const grouped = useMemo(() => {
    const map = new Map<LoomAiJobKind, LoomAiSuggestion[]>();
    for (const s of suggestions) {
      const list = map.get(s.kind) ?? [];
      list.push(s);
      map.set(s.kind, list);
    }
    return map;
  }, [suggestions]);

  return (
    <div className="content-card loom-ai-review-panel" data-testid="loom-ai-review-panel">
      <header className="loom-ai-review-panel__header">
        <h2>AI Loom Review</h2>
        {onClose ? (
          <button type="button" onClick={onClose} data-testid="loom-ai-close">
            Close
          </button>
        ) : null}
      </header>

      <div className="loom-ai-review-panel__run">
        {KIND_ORDER.map((kind) => (
          <button
            key={kind}
            type="button"
            disabled={busy || resolvedBlockIds.length === 0}
            onClick={() => void runJob(kind)}
            data-testid={`loom-ai-run-${kind}`}
          >
            Run {KIND_LABELS[kind]}
          </button>
        ))}
      </div>

      {noModel ? (
        <p role="alert" data-testid="loom-ai-no-model">
          No model is configured. AI Loom jobs decline loudly — configure a model to run them.
        </p>
      ) : null}

      {error ? (
        <p role="alert" data-testid="loom-ai-error">
          {error}
        </p>
      ) : null}

      {loading ? <p data-testid="loom-ai-loading">Loading suggestions…</p> : null}

      {!loading && suggestions.length === 0 ? (
        <p data-testid="loom-ai-empty">No pending AI suggestions. Run a job to generate some.</p>
      ) : null}

      {KIND_ORDER.filter((kind) => grouped.has(kind)).map((kind) => {
        const rows = grouped.get(kind) ?? [];
        const jobId = rows[0]?.job_id ?? "";
        return (
          <section key={kind} data-testid={`loom-ai-group-${kind}`} className="loom-ai-group">
            <div className="loom-ai-group__header">
              <h3>
                {KIND_LABELS[kind]} ({rows.length})
              </h3>
              <button
                type="button"
                disabled={busy}
                onClick={() => void acceptAll(kind, jobId)}
                data-testid={`loom-ai-accept-all-${kind}`}
              >
                Accept all {KIND_LABELS[kind]}
              </button>
            </div>
            <ul>
              {rows.map((s) => (
                <li key={s.suggestion_id} data-testid={`loom-ai-suggestion-${s.suggestion_id}`}>
                  <span className="loom-ai-suggestion__value">{describeSuggestion(s)}</span>
                  <span className="loom-ai-suggestion__model muted">
                    {String((s.model_attribution ?? {}).model ?? "model")}
                  </span>
                  <button
                    type="button"
                    disabled={busy}
                    onClick={() => void accept(s.suggestion_id)}
                    data-testid={`loom-ai-accept-${s.suggestion_id}`}
                  >
                    Accept
                  </button>
                  <button
                    type="button"
                    disabled={busy}
                    onClick={() => void reject(s.suggestion_id)}
                    data-testid={`loom-ai-reject-${s.suggestion_id}`}
                  >
                    Reject
                  </button>
                </li>
              ))}
            </ul>
          </section>
        );
      })}
    </div>
  );
}
