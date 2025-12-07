import { useEffect, useState } from "react";
import { DocumentWithBlocks, getDocument } from "../lib/api";

type Props = {
  documentId: string | null;
};

export function DocumentView({ documentId }: Props) {
  const [doc, setDoc] = useState<DocumentWithBlocks | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!documentId) {
      setDoc(null);
      setError(null);
      return;
    }
    const load = async () => {
      setLoading(true);
      setError(null);
      try {
        const response = await getDocument(documentId);
        setDoc(response);
      } catch (err) {
        setError(err instanceof Error ? err.message : "Failed to load document");
      } finally {
        setLoading(false);
      }
    };
    void load();
  }, [documentId]);

  if (!documentId) {
    return (
      <div className="content-card">
        <h2>Document</h2>
        <p className="muted">No document selected.</p>
      </div>
    );
  }

  if (loading) {
    return (
      <div className="content-card">
        <h2>Document</h2>
        <p className="muted">Loading…</p>
      </div>
    );
  }

  if (error) {
    return (
      <div className="content-card">
        <h2>Document</h2>
        <p className="muted">Error: {error}</p>
      </div>
    );
  }

  if (!doc) return null;

  return (
    <div className="content-card">
      <h2>{doc.title}</h2>
      <p className="muted">Workspace: {doc.workspace_id}</p>
      <p className="muted">
        Created: {new Date(doc.created_at).toLocaleString()} — Updated: {new Date(doc.updated_at).toLocaleString()}
      </p>
      <h3>Blocks</h3>
      {doc.blocks.length === 0 ? (
        <p className="muted">No blocks yet.</p>
      ) : (
        <ul className="list-inline">
          {doc.blocks.map((block) => (
            <li key={block.id} className="content-card" style={{ padding: "10px" }}>
              <strong>
                {block.sequence}. {block.kind}
              </strong>
              <p className="muted">{block.raw_content.slice(0, 200)}</p>
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}
