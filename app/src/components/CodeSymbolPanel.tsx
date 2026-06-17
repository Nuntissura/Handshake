import { useEffect, useState } from "react";
import {
  getCodeFileLens,
  getCodeSymbol,
  type CodeFileLensResponse,
  type CodeSymbolNavProjection,
  type CodeSymbolResponse,
} from "../lib/api";
import { codeSymbolStalenessLabel } from "../lib/monaco/code_intelligence";

type Props = {
  symbolEntityId: string;
  workspaceId: string | null;
};

function definitionLabel(response: CodeSymbolResponse): string {
  const definition = response.symbol.definition;
  if (!definition) return "Definition span unavailable";
  return `${definition.source_id}:${definition.line_start}-${definition.line_end}`;
}

function symbolFilePath(symbolKey: string): string | null {
  const beforeHash = symbolKey.split("#")[0];
  const separator = beforeHash.indexOf(":");
  if (separator < 0) return null;
  const path = beforeHash.slice(separator + 1).trim();
  return path.length > 0 ? path : null;
}

function fileLensInputs(symbol: CodeSymbolNavProjection): {
  relativePath: string;
  contentHash: string;
  parserVersion: string;
} | null {
  if (!symbol.staleness || typeof symbol.staleness !== "object") return null;
  const staleness = symbol.staleness as Record<string, unknown>;
  const contentHash = staleness.indexed_content_hash;
  const parserVersion = staleness.indexed_parser_version;
  const relativePath = symbolFilePath(symbol.symbol_key);
  if (typeof contentHash !== "string" || typeof parserVersion !== "string" || !relativePath) {
    return null;
  }
  return { relativePath, contentHash, parserVersion };
}

export function CodeSymbolPanel({ symbolEntityId, workspaceId }: Props) {
  const [response, setResponse] = useState<CodeSymbolResponse | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const [fileLens, setFileLens] = useState<CodeFileLensResponse | null>(null);
  const [fileLensError, setFileLensError] = useState<string | null>(null);
  const [fileLensLoading, setFileLensLoading] = useState(false);

  useEffect(() => {
    let cancelled = false;
    const timer = window.setTimeout(() => {
      if (cancelled) return;
      setLoading(true);
      setError(null);
      setResponse(null);

      getCodeSymbol(symbolEntityId)
        .then((nextResponse) => {
          if (!cancelled) setResponse(nextResponse);
        })
        .catch((err) => {
          if (!cancelled) setError(err instanceof Error ? err.message : "Code symbol lookup failed");
        })
        .finally(() => {
          if (!cancelled) setLoading(false);
        });
    }, 0);

    return () => {
      cancelled = true;
      window.clearTimeout(timer);
    };
  }, [symbolEntityId]);

  useEffect(() => {
    let cancelled = false;
    const timer = window.setTimeout(() => {
      if (cancelled) return;
      if (!response || !workspaceId) {
        setFileLens(null);
        setFileLensError(null);
        setFileLensLoading(false);
        return;
      }
      const inputs = fileLensInputs(response.symbol);
      if (!inputs) {
        setFileLens(null);
        setFileLensError(null);
        setFileLensLoading(false);
        return;
      }
      setFileLensLoading(true);
      setFileLensError(null);
      getCodeFileLens(workspaceId, inputs.relativePath, inputs.contentHash, inputs.parserVersion)
        .then((nextLens) => {
          if (!cancelled) setFileLens(nextLens);
        })
        .catch((err) => {
          if (!cancelled) setFileLensError(err instanceof Error ? err.message : "Code file lens lookup failed");
        })
        .finally(() => {
          if (!cancelled) setFileLensLoading(false);
        });
    }, 0);
    return () => {
      cancelled = true;
      window.clearTimeout(timer);
    };
  }, [response, workspaceId]);

  if (loading) {
    return (
      <div className="content-card code-symbol-panel" data-testid="code-symbol-panel">
        <p>Loading code symbol...</p>
      </div>
    );
  }

  if (error || !response) {
    return (
      <div className="content-card code-symbol-panel error" data-testid="code-symbol-panel">
        <h2>Code Symbol</h2>
        <p>{error ?? "Code symbol unavailable"}</p>
      </div>
    );
  }

  const symbol = response.symbol;

  return (
    <div className="content-card code-symbol-panel" data-testid="code-symbol-panel">
      <header className="code-symbol-panel__header">
        <div>
          <p className="app-eyebrow">Code Symbol</p>
          <h2>{symbol.display_name}</h2>
        </div>
        <span className="kernel-dcc__badge">{symbol.lifecycle_state}</span>
      </header>
      <dl className="code-symbol-panel__facts">
        <div>
          <dt>Entity</dt>
          <dd>{symbol.symbol_entity_id}</dd>
        </div>
        <div>
          <dt>Key</dt>
          <dd>{symbol.symbol_key}</dd>
        </div>
        <div>
          <dt>Kind</dt>
          <dd>{symbol.symbol_kind}</dd>
        </div>
        <div>
          <dt>Owning WP</dt>
          <dd>{symbol.owning_wp ?? "none"}</dd>
        </div>
        <div>
          <dt>Definition</dt>
          <dd>{definitionLabel(response)}</dd>
        </div>
        <div>
          <dt>Staleness</dt>
          <dd data-testid="code-symbol-panel.staleness">{codeSymbolStalenessLabel(symbol.staleness)}</dd>
        </div>
        <div>
          <dt>Navigation Receipt</dt>
          <dd>{response.nav_receipt_event_id}</dd>
        </div>
      </dl>
      <section className="code-symbol-panel__file-lens" data-testid="code-symbol-panel.file-lens">
        <h3>Target File Lens</h3>
        {fileLensLoading ? <p>Loading file lens...</p> : null}
        {fileLensError ? <p className="error">{fileLensError}</p> : null}
        {!fileLensLoading && !fileLensError && !fileLens ? <p>File lens unavailable for this symbol.</p> : null}
        {fileLens ? (
          <>
            <p className="muted">
              {fileLens.relative_path} · {codeSymbolStalenessLabel(fileLens.staleness)}
              {fileLens.truncated ? " · truncated" : ""}
            </p>
            <ul>
              {fileLens.entries.map((entry) => (
                <li key={entry.symbol_entity_id}>
                  <strong>{entry.display_name}</strong> {entry.symbol_kind} line {entry.definition.start_line}
                  {entry.definition.end_line !== entry.definition.start_line ? `-${entry.definition.end_line}` : ""}
                  {entry.doc ? <p>{entry.doc}</p> : null}
                </li>
              ))}
            </ul>
          </>
        ) : null}
      </section>
    </div>
  );
}
