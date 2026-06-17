import { useMemo, useState, type ReactNode } from "react";
import {
  createBlockView,
  loomSearchV2,
  type LoomBlockContentType,
  type LoomSearchV2Hit,
  type LoomSearchV2Response,
} from "../lib/api";

// MT-264 LoomSearchV2 surface: query box, ranked highlighted results,
// content_type facets, open-in-place, and save-results-as-Loom-view (reuses
// the MT-262 block-collection view). Results are REFERENCES into LoomBlocks
// (open the real block in place), never content copies.

type Props = {
  workspaceId: string | null;
  /** Open the REAL block in place (reference, not copy). */
  onOpenBlock: (blockId: string) => void;
  /** Optional injected search fn for offline/visual harness proof. */
  searchFn?: typeof loomSearchV2;
  /** Optional injected view-create fn for offline/visual harness proof. */
  createViewFn?: typeof createBlockView;
};

/** Render a ts_headline highlight (already containing <mark> markers) safely as
 * React nodes by splitting on the marker tokens -- no dangerouslySetInnerHTML. */
function renderHighlight(highlight: string): ReactNode {
  if (!highlight) return null;
  const parts = highlight.split(/(<mark>|<\/mark>)/g);
  const nodes: ReactNode[] = [];
  let marked = false;
  let key = 0;
  for (const part of parts) {
    if (part === "<mark>") {
      marked = true;
      continue;
    }
    if (part === "</mark>") {
      marked = false;
      continue;
    }
    if (part === "") continue;
    nodes.push(
      marked ? (
        <mark key={key++} data-testid="loom-search-v2.highlight-mark">
          {part}
        </mark>
      ) : (
        <span key={key++}>{part}</span>
      ),
    );
  }
  return nodes;
}

export function LoomSearchV2Panel({ workspaceId, onOpenBlock, searchFn, createViewFn }: Props) {
  const search = searchFn ?? loomSearchV2;
  const createView = createViewFn ?? createBlockView;

  const [query, setQuery] = useState("");
  const [contentType, setContentType] = useState<LoomBlockContentType | "">("");
  const [response, setResponse] = useState<LoomSearchV2Response | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [status, setStatus] = useState<string | null>(null);

  const facetEntries = useMemo(
    () => Object.entries(response?.content_type_facets ?? {}).sort((a, b) => b[1] - a[1]),
    [response],
  );

  const runSearch = async (overrideContentType?: LoomBlockContentType | "") => {
    if (!workspaceId) {
      setError("No workspace selected");
      return;
    }
    if (!query.trim()) {
      setError("Search query is required");
      return;
    }
    const ct = overrideContentType ?? contentType;
    setLoading(true);
    setError(null);
    setStatus(null);
    try {
      const result = await search(workspaceId, {
        query: query.trim(),
        contentType: ct === "" ? undefined : ct,
        graphBoost: 1.0,
        limit: 25,
      });
      setResponse(result);
    } catch (err) {
      setResponse(null);
      setError(err instanceof Error ? err.message : "Loom search failed");
    } finally {
      setLoading(false);
    }
  };

  const applyFacet = (ct: LoomBlockContentType) => {
    const next = contentType === ct ? "" : ct;
    setContentType(next);
    void runSearch(next);
  };

  const saveAsView = async () => {
    if (!workspaceId || !response) return;
    setStatus(null);
    try {
      const record = await createView(
        workspaceId,
        {
          kind: "table",
          query: contentType === "" ? {} : { content_type: contentType },
          columns: ["title", "content_type", "updated"],
        },
        { title: `Search: ${query.trim()}` },
      );
      setStatus(`Saved search as Loom view ${record.block.block_id}`);
    } catch (err) {
      setStatus(err instanceof Error ? `Save view failed: ${err.message}` : "Save view failed");
    }
  };

  return (
    <section className="loom-search-v2" aria-label="Loom search" data-testid="loom-search-v2">
      <header className="loom-search-v2__header">
        <h3>Loom Search</h3>
        <p className="drawer-eyebrow">Hybrid: full-text + fuzzy + semantic</p>
      </header>

      <div className="loom-search-v2__controls">
        <input
          type="search"
          value={query}
          placeholder="Search the Loom"
          aria-label="Loom search query"
          data-testid="loom-search-v2.query"
          onChange={(event) => setQuery(event.target.value)}
          onKeyDown={(event) => {
            if (event.key === "Enter") {
              event.preventDefault();
              void runSearch();
            }
          }}
        />
        <button type="button" onClick={() => void runSearch()} data-testid="loom-search-v2.search">
          Search
        </button>
        <button
          type="button"
          onClick={() => void saveAsView()}
          disabled={!response || response.hits.length === 0}
          data-testid="loom-search-v2.save-view"
        >
          Save as view
        </button>
      </div>

      <div className="loom-search-v2__status" aria-live="polite" data-testid="loom-search-v2.status">
        {loading
          ? "Searching..."
          : error ??
            (response
              ? `${response.total} result${response.total === 1 ? "" : "s"}${
                  response.semantic_available ? " (semantic on)" : " (keyword/fuzzy only)"
                }`
              : "Enter a query")}
      </div>
      {status ? (
        <div className="loom-search-v2__view-status" role="status" data-testid="loom-search-v2.view-status">
          {status}
        </div>
      ) : null}

      {facetEntries.length > 0 ? (
        <div className="loom-search-v2__facets" data-testid="loom-search-v2.facets">
          {facetEntries.map(([ct, count]) => (
            <button
              type="button"
              key={ct}
              className={contentType === ct ? "loom-search-v2__facet loom-search-v2__facet--active" : "loom-search-v2__facet"}
              aria-pressed={contentType === ct}
              data-testid={`loom-search-v2.facet.${ct}`}
              onClick={() => applyFacet(ct as LoomBlockContentType)}
            >
              {ct} ({count})
            </button>
          ))}
        </div>
      ) : null}

      <ul className="loom-search-v2__results" role="listbox" aria-label="Loom search results">
        {(response?.hits ?? []).map((hit: LoomSearchV2Hit) => (
          <li key={hit.block.block_id}>
            <button
              type="button"
              data-testid={`loom-search-v2.result.${hit.block.block_id}`}
              data-block-id={hit.block.block_id}
              data-vector-sim={hit.vector_sim}
              data-fts-rank={hit.fts_rank}
              data-trgm-sim={hit.trgm_sim}
              onClick={() => onOpenBlock(hit.block.block_id)}
            >
              <span className="loom-search-v2__title">{hit.block.title ?? hit.block.block_id}</span>
              <span className="loom-search-v2__type">{hit.block.content_type}</span>
              <span className="loom-search-v2__highlight" data-testid={`loom-search-v2.result.${hit.block.block_id}.highlight`}>
                {renderHighlight(hit.highlight)}
              </span>
              <span className="loom-search-v2__score">score {hit.score.toFixed(3)}</span>
            </button>
          </li>
        ))}
      </ul>
    </section>
  );
}
