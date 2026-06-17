import { useEffect, useState } from "react";
import { getLoomWikiProjection, type LoomWikiProjection } from "../lib/api";

type Props = {
  workspaceId: string;
  projectionId: string;
};

function optionalText(value: string | number | boolean | null | undefined): string {
  if (value === null || value === undefined || value === "") return "none";
  return String(value);
}

function pageSummary(page: LoomWikiProjection): string {
  return page.rendered_content.trim() || "No rendered wiki content.";
}

export function LoomWikiPagePanel({ workspaceId, projectionId }: Props) {
  const [page, setPage] = useState<LoomWikiProjection | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    let cancelled = false;
    const timer = window.setTimeout(() => {
      if (cancelled) return;
      setLoading(true);
      setError(null);
      setPage(null);

      getLoomWikiProjection(workspaceId, projectionId)
        .then((response) => {
          if (!cancelled) setPage(response);
        })
        .catch((err) => {
          if (!cancelled) setError(err instanceof Error ? err.message : "Loom wiki page lookup failed");
        })
        .finally(() => {
          if (!cancelled) setLoading(false);
        });
    }, 0);

    return () => {
      cancelled = true;
      window.clearTimeout(timer);
    };
  }, [workspaceId, projectionId]);

  if (loading) {
    return (
      <div className="content-card loom-block-panel loom-wiki-page-panel" data-testid="loom-wiki-page-panel">
        <p>Loading Loom wiki page...</p>
      </div>
    );
  }

  if (error || !page) {
    return (
      <div className="content-card loom-block-panel loom-wiki-page-panel error" data-testid="loom-wiki-page-panel">
        <h2>Wiki Page</h2>
        <p>{error ?? "Loom wiki page unavailable"}</p>
      </div>
    );
  }

  return (
    <div className="content-card loom-block-panel loom-wiki-page-panel" data-testid="loom-wiki-page-panel">
      <header className="loom-block-panel__header">
        <div>
          <p className="app-eyebrow">Wiki Page</p>
          <h2>{page.title}</h2>
        </div>
        <span className="kernel-dcc__badge">{optionalText(page.page_type)}</span>
      </header>
      <dl className="loom-block-panel__facts">
        <div>
          <dt>Projection</dt>
          <dd>{page.projection_id}</dd>
        </div>
        <div>
          <dt>Workspace</dt>
          <dd>{page.workspace_id}</dd>
        </div>
        <div>
          <dt>Rebuild</dt>
          <dd>{page.rebuild_status}</dd>
        </div>
        <div>
          <dt>Sources</dt>
          <dd>{page.source_block_ids.length}</dd>
        </div>
      </dl>
      <section className="loom-block-panel__body">
        <h3>Rendered Content</h3>
        <p>{pageSummary(page)}</p>
      </section>
    </div>
  );
}
