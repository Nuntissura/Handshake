import { useEffect, useState } from "react";
import {
  createCanvas,
  createDocument,
  createWorkspace,
  listCanvases,
  listDocuments,
  listWorkspaces,
  CanvasSummary,
  DocumentSummary,
  Workspace,
} from "../lib/api";

type Props = {
  onSelectDocument: (id: string | null) => void;
  onSelectCanvas: (id: string | null) => void;
  selectedDocumentId: string | null;
  selectedCanvasId: string | null;
};

export function WorkspaceSidebar({
  onSelectDocument,
  onSelectCanvas,
  selectedDocumentId,
  selectedCanvasId,
}: Props) {
  const [workspaces, setWorkspaces] = useState<Workspace[]>([]);
  const [selectedWorkspaceId, setSelectedWorkspaceId] = useState<string | null>(null);
  const [documents, setDocuments] = useState<DocumentSummary[]>([]);
  const [canvases, setCanvases] = useState<CanvasSummary[]>([]);
  const [loading, setLoading] = useState<boolean>(false);
  const [workspaceError, setWorkspaceError] = useState<string | null>(null);

  useEffect(() => {
    void loadWorkspaces();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  async function selectWorkspace(id: string) {
    setSelectedWorkspaceId(id);
    onSelectDocument(null);
    onSelectCanvas(null);
    setDocuments([]);
    setCanvases([]);
    try {
      const [docs, cvs] = await Promise.all([listDocuments(id), listCanvases(id)]);
      setDocuments(docs);
      setCanvases(cvs);
    } catch (err) {
      setWorkspaceError(err instanceof Error ? err.message : "Failed to load workspace details");
    }
  }

  async function handleCreateWorkspace() {
    const name = window.prompt("Workspace name?");
    if (!name) return;
    try {
      const ws = await createWorkspace(name);
      setWorkspaces((prev) => [...prev, ws]);
      await selectWorkspace(ws.id);
    } catch (err) {
      setWorkspaceError(err instanceof Error ? err.message : "Failed to create workspace");
    }
  }

  async function handleCreateDocument() {
    if (!selectedWorkspaceId) return;
    const title = window.prompt("Document title?");
    if (!title) return;
    try {
      const doc = await createDocument(selectedWorkspaceId, title);
      setDocuments((prev) => [...prev, doc]);
      onSelectDocument(doc.id);
      onSelectCanvas(null);
    } catch (err) {
      setWorkspaceError(err instanceof Error ? err.message : "Failed to create document");
    }
  }

  async function handleCreateCanvas() {
    if (!selectedWorkspaceId) return;
    const title = window.prompt("Canvas title?");
    if (!title) return;
    try {
      const canvas = await createCanvas(selectedWorkspaceId, title);
      setCanvases((prev) => [...prev, canvas]);
      onSelectCanvas(canvas.id);
      onSelectDocument(null);
    } catch (err) {
      setWorkspaceError(err instanceof Error ? err.message : "Failed to create canvas");
    }
  }

  async function loadWorkspaces() {
    setLoading(true);
    try {
      const ws = await listWorkspaces();
      setWorkspaces(ws);
      setWorkspaceError(null);
      if (ws.length > 0) {
        selectWorkspace(ws[0].id);
      }
    } catch (err) {
      setWorkspaceError(err instanceof Error ? err.message : "Failed to load workspaces");
      // keep existing workspaces on failure
    } finally {
      setLoading(false);
    }
  }

  return (
    <aside className="sidebar">
      <section>
        <h3>Workspaces</h3>
        <button onClick={handleCreateWorkspace}>New Workspace</button>
        {loading && <p className="muted">Loading…</p>}
        {workspaceError && (
          <div
            className="content-card"
            style={{ padding: "8px 10px", background: "#fff6f2", border: "1px solid #f4b8a7", marginTop: 8 }}
          >
            <p className="muted" style={{ marginBottom: 8 }}>
              Could not refresh the workspace list. Your existing workspaces are safe; this is likely a temporary connection issue. You can continue using the list below or press Retry.
            </p>
            <button type="button" onClick={() => void loadWorkspaces()} disabled={loading}>
              Retry
            </button>
          </div>
        )}
        <ul className="list">
          {workspaces.map((ws) => (
            <li
              key={ws.id}
              className={`list-item ${selectedWorkspaceId === ws.id ? "selected" : ""}`}
              onClick={() => selectWorkspace(ws.id)}
            >
              <span>{ws.name || "Untitled"}</span>
            </li>
          ))}
          {workspaces.length === 0 && !loading && <li className="muted">No workspaces yet.</li>}
        </ul>
      </section>

      <section>
        <h3>Documents</h3>
        <button onClick={handleCreateDocument} disabled={!selectedWorkspaceId}>
          New Document
        </button>
        <ul className="list">
          {documents.map((doc) => (
            <li
              key={doc.id}
              className={`list-item ${selectedDocumentId === doc.id ? "selected" : ""}`}
              onClick={() => {
                onSelectDocument(doc.id);
                onSelectCanvas(null);
              }}
            >
              <span>{doc.title || "Untitled document"}</span>
              <small>{new Date(doc.created_at).toLocaleDateString()}</small>
            </li>
          ))}
          {documents.length === 0 && <li className="muted">No documents.</li>}
        </ul>
      </section>

      <section>
        <h3>Canvases</h3>
        <button onClick={handleCreateCanvas} disabled={!selectedWorkspaceId}>
          New Canvas
        </button>
        <ul className="list">
          {canvases.map((canvas) => (
            <li
              key={canvas.id}
              className={`list-item ${selectedCanvasId === canvas.id ? "selected" : ""}`}
              onClick={() => {
                onSelectCanvas(canvas.id);
                onSelectDocument(null);
              }}
            >
              <span>{canvas.title || "Untitled canvas"}</span>
              <small>{new Date(canvas.created_at).toLocaleDateString()}</small>
            </li>
          ))}
          {canvases.length === 0 && <li className="muted">No canvases.</li>}
        </ul>
      </section>
    </aside>
  );
}


