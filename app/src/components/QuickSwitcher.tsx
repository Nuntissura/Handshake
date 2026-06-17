import { useEffect, useMemo, useRef, useState } from "react";
import {
  listQuickSwitcherRecents,
  recordQuickSwitcherRecent,
  searchLoomGraph,
  type LoomGraphSearchHit,
  type LoomGraphSearchSourceKind,
} from "../lib/api";
import { openTargetForLoomSearchHit } from "../lib/loom_search_open_target";

const QUICK_SWITCHER_SOURCE_KINDS: LoomGraphSearchSourceKind[] = [
  "loom_block",
  "file",
  "tag_hub",
  "document",
  "symbol",
  "work_packet",
  "micro_task",
  "user_manual_page",
  "wiki_page",
];

const SOURCE_LABELS: Record<LoomGraphSearchSourceKind, string> = {
  loom_block: "Loom Block",
  file: "File",
  tag_hub: "Tag Hub",
  document: "Document",
  symbol: "Symbol",
  work_packet: "Work Packet",
  micro_task: "Microtask",
  user_manual_page: "UserManual Page",
  wiki_page: "Wiki Page",
};

type Props = {
  open: boolean;
  workspaceId: string | null;
  onClose: () => void;
  onOpenCodeSymbol: (symbolEntityId: string) => void;
  onOpenDocument: (documentId: string) => void;
  onOpenLoomBlock: (blockId: string) => void;
  onOpenMicroTask: (target: { mtId: string; wpId?: string | null }) => void;
  onOpenWorkPacket: (wpId: string) => void;
  onOpenUserManualPage: (slug: string) => void;
  onOpenWikiPage: (projectionId: string) => void;
};

function stablePart(value: string): string {
  const stable = value
    .trim()
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-+|-+$/g, "");
  return stable || "item";
}

function hitKey(hit: LoomGraphSearchHit): string {
  return `${hit.source_kind}:${hit.ref_id}`;
}

export function QuickSwitcher({
  open,
  workspaceId,
  onClose,
  onOpenCodeSymbol,
  onOpenDocument,
  onOpenLoomBlock,
  onOpenMicroTask,
  onOpenWorkPacket,
  onOpenUserManualPage,
  onOpenWikiPage,
}: Props) {
  const searchRef = useRef<HTMLInputElement | null>(null);
  const [query, setQuery] = useState("");
  const [results, setResults] = useState<LoomGraphSearchHit[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [recentsError, setRecentsError] = useState<string | null>(null);
  const [recents, setRecents] = useState<string[]>([]);
  const [selectedIndex, setSelectedIndex] = useState(0);

  useEffect(() => {
    if (!open) return;
    window.setTimeout(() => {
      searchRef.current?.focus();
      searchRef.current?.select();
    }, 0);
  }, [open]);

  useEffect(() => {
    if (!open || !workspaceId) return;
    let cancelled = false;
    listQuickSwitcherRecents(workspaceId, 20)
      .then((items) => {
        if (!cancelled) {
          setRecents(items.map((item) => item.hit_key));
          setRecentsError(null);
        }
      })
      .catch((err) => {
        if (!cancelled) {
          setRecents([]);
          setRecentsError(err instanceof Error ? err.message : "Durable recents failed to load");
        }
      });
    return () => {
      cancelled = true;
    };
  }, [open, workspaceId]);

  useEffect(() => {
    if (!open) return;
    const trimmed = query.trim();
    if (!trimmed) {
      return;
    }
    if (!workspaceId) {
      return;
    }

    let cancelled = false;
    const timer = window.setTimeout(() => {
      if (cancelled) return;
      setLoading(true);
      setError(null);
      searchLoomGraph(workspaceId, {
        q: trimmed,
        limit: 25,
        sourceKinds: QUICK_SWITCHER_SOURCE_KINDS,
      })
        .then((hits) => {
          if (!cancelled) {
            setResults(hits);
            setSelectedIndex(0);
          }
        })
        .catch((err) => {
          if (!cancelled) {
            setResults([]);
            setError(err instanceof Error ? err.message : "Graph search failed");
          }
        })
        .finally(() => {
          if (!cancelled) setLoading(false);
        });
    }, 150);

    return () => {
      cancelled = true;
      window.clearTimeout(timer);
    };
  }, [open, query, workspaceId]);

  const orderedResults = useMemo(() => {
    const recentRank = new Map(recents.map((key, index) => [key, index]));
    return [...results].sort((a, b) => {
      const aRank = recentRank.get(hitKey(a));
      const bRank = recentRank.get(hitKey(b));
      if (aRank !== undefined && bRank !== undefined) return aRank - bRank;
      if (aRank !== undefined) return -1;
      if (bRank !== undefined) return 1;
      return 0;
    });
  }, [recents, results]);

  if (!open) return null;
  const hasSearchQuery = query.trim().length > 0;
  const hasWorkspace = Boolean(workspaceId);
  const visibleResults = hasSearchQuery && hasWorkspace ? orderedResults : [];
  const safeSelectedIndex = Math.min(Math.max(selectedIndex, 0), Math.max(visibleResults.length - 1, 0));
  const visibleLoading = hasWorkspace && loading;

  const selectHit = (hit: LoomGraphSearchHit) => {
    if (!workspaceId) return;
    const target = openTargetForLoomSearchHit(hit);
    if (!target.enabled) return;
    void recordQuickSwitcherRecent(workspaceId, {
      result_kind: hit.result_kind,
      source_kind: hit.source_kind,
      ref_id: hit.ref_id,
      title: hit.title,
      excerpt: hit.excerpt ?? "",
      metadata: hit.metadata ?? {},
    })
      .then((recent) => {
        setRecents((current) => [recent.hit_key, ...current.filter((key) => key !== recent.hit_key)].slice(0, 20));
        setRecentsError(null);
      })
      .catch((err) => {
        setRecentsError(err instanceof Error ? err.message : "Durable recent was not recorded");
      });
    if (target.kind === "user_manual") {
      onOpenUserManualPage(target.slug);
    } else if (target.kind === "wiki_page") {
      onOpenWikiPage(target.projectionId);
    } else if (target.kind === "document") {
      onOpenDocument(target.documentId);
    } else if (target.kind === "loom_block") {
      onOpenLoomBlock(target.blockId);
    } else if (target.kind === "code_symbol") {
      onOpenCodeSymbol(target.symbolEntityId);
    } else if (target.kind === "work_packet") {
      onOpenWorkPacket(target.wpId);
    } else if (target.kind === "micro_task") {
      onOpenMicroTask(target.target);
    }
    onClose();
  };

  return (
    <div
      className="quick-switcher-overlay"
      role="dialog"
      aria-modal="true"
      aria-label="Quick switcher"
      onMouseDown={(event) => {
        if (event.target === event.currentTarget) onClose();
      }}
      onKeyDown={(event) => {
        if (event.key === "Escape") {
          event.preventDefault();
          onClose();
          return;
        }
        if (event.key === "ArrowDown") {
          event.preventDefault();
          setSelectedIndex((index) => Math.min(index + 1, Math.max(visibleResults.length - 1, 0)));
          return;
        }
        if (event.key === "ArrowUp") {
          event.preventDefault();
          setSelectedIndex((index) => Math.max(index - 1, 0));
          return;
        }
        if (event.key === "Enter") {
          event.preventDefault();
          const hit = visibleResults[safeSelectedIndex];
          if (hit) selectHit(hit);
        }
      }}
      data-testid="quick-switcher"
    >
      <div className="quick-switcher" onMouseDown={(event) => event.stopPropagation()}>
        <div className="quick-switcher__header">
          <div>
            <p className="drawer-eyebrow">Open</p>
            <h3>Quick Switcher</h3>
          </div>
          <button type="button" className="secondary" onClick={onClose}>
            Close
          </button>
        </div>
        <input
          ref={searchRef}
          className="quick-switcher__search"
          type="search"
          value={query}
          placeholder="Open by title, block, symbol, WP, MT, manual, or wiki page"
          aria-label="Quick switcher search"
          data-testid="quick-switcher.search"
          onChange={(event) => {
            const nextQuery = event.target.value;
            const hasNextQuery = nextQuery.trim().length > 0;
            setQuery(nextQuery);
            setSelectedIndex(0);
            setResults([]);
            setLoading(hasNextQuery && hasWorkspace);
            setError(null);
          }}
        />
        <div className="quick-switcher__status" aria-live="polite" data-testid="quick-switcher.status">
          {!hasSearchQuery
            ? "Type to search the project graph"
            : !hasWorkspace
              ? "No workspace selected"
              : visibleLoading
              ? "Searching graph..."
              : error ??
                `${visibleResults.length} result${visibleResults.length === 1 ? "" : "s"}${
                  recentsError ? `; durable recents unavailable: ${recentsError}` : ""
                }`}
        </div>
        <ul className="quick-switcher__results" role="listbox" aria-label="Quick switcher results">
          {visibleResults.map((hit, index) => {
            const target = openTargetForLoomSearchHit(hit);
            const testId = `quick-switcher.result.${hit.source_kind}.${stablePart(hit.ref_id)}`;
            const selected = index === safeSelectedIndex;
            return (
              <li key={hitKey(hit)}>
                <button
                  type="button"
                  className={
                    selected
                      ? "quick-switcher__result quick-switcher__result--selected"
                      : "quick-switcher__result"
                  }
                  disabled={!target.enabled}
                  onClick={() => selectHit(hit)}
                  onMouseEnter={() => setSelectedIndex(index)}
                  role="option"
                  aria-selected={selected}
                  data-testid={testId}
                  data-source-kind={hit.source_kind}
                  data-result-kind={hit.result_kind}
                  data-ref-id={hit.ref_id}
                  title={target.label}
                >
                  <span className="quick-switcher__kind">{SOURCE_LABELS[hit.source_kind]}</span>
                  <span className="quick-switcher__title">{hit.title}</span>
                  <span className="quick-switcher__excerpt">{hit.excerpt}</span>
                  <span className="quick-switcher__target">{target.label}</span>
                </button>
              </li>
            );
          })}
        </ul>
      </div>
    </div>
  );
}
