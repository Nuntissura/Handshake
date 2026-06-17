import { useCallback, useEffect, useRef, useState } from "react";
import {
  createCanvas,
  createDocument,
  createWorkspace,
  deleteWorkspace,
  listCanvases,
  listDocuments,
  listWorkspaces,
  queryLoomView,
  setLoomBlockPinOrder,
  updateLoomBlock,
  CanvasSummary,
  DocumentSummary,
  LoomBlock,
  Workspace,
} from "../lib/api";

type Props = {
  refreshKey: number;
  activeWorkspaceId?: string | null;
  onSelectWorkspace?: (id: string) => void;
  onSelectDocument: (id: string | null) => void;
  onSelectCanvas: (id: string | null) => void;
  onOpenLoomBlock?: (id: string) => void;
  selectedDocumentId: string | null;
  selectedCanvasId: string | null;
  onWorkspaceDeleted: () => void;
};

type DocumentDeletedDetail = { documentId: string; workspaceId?: string | null };
type CanvasDeletedDetail = { canvasId: string; workspaceId?: string | null };
type LoomBookmarksChangedDetail = { workspaceId?: string | null };

function stablePart(value: string): string {
  return value.trim().toLowerCase().replace(/[^a-z0-9_-]+/g, "-").replace(/^-+|-+$/g, "") || "item";
}

function blockTitle(block: LoomBlock): string {
  return block.title?.trim() || block.original_filename?.trim() || block.block_id;
}

function bookmarkKind(block: LoomBlock): string {
  if (block.document_id?.trim()) return "document";
  if (block.content_type === "file" || block.content_type === "tag_hub" || block.content_type === "journal") {
    return block.content_type;
  }
  return "block";
}

export function WorkspaceSidebar({
  refreshKey,
  activeWorkspaceId,
  onSelectWorkspace,
  onSelectDocument,
  onSelectCanvas,
  onOpenLoomBlock,
  selectedDocumentId,
  selectedCanvasId,
  onWorkspaceDeleted,
}: Props) {
  const [workspaces, setWorkspaces] = useState<Workspace[]>([]);
  const [selectedWorkspaceId, setSelectedWorkspaceId] = useState<string | null>(null);
  const [documents, setDocuments] = useState<DocumentSummary[]>([]);
  const [canvases, setCanvases] = useState<CanvasSummary[]>([]);
  const [bookmarks, setBookmarks] = useState<LoomBlock[]>([]);
  const [bookmarksLoading, setBookmarksLoading] = useState(false);
  const [bookmarksError, setBookmarksError] = useState<string | null>(null);
  const [bookmarksStatus, setBookmarksStatus] = useState<string | null>(null);
  const [loading, setLoading] = useState<boolean>(false);
  const [workspaceError, setWorkspaceError] = useState<string | null>(null);
  const workspaceDetailsLoadRef = useRef(0);
  const bookmarksLoadRef = useRef(0);
  const effectiveWorkspaceId = activeWorkspaceId !== undefined ? activeWorkspaceId : selectedWorkspaceId;
  const effectiveWorkspaceIdRef = useRef<string | null>(effectiveWorkspaceId);

  const clearWorkspaceDetails = useCallback(() => {
    workspaceDetailsLoadRef.current += 1;
    setDocuments([]);
    setCanvases([]);
  }, []);

  const clearBookmarks = useCallback(() => {
    bookmarksLoadRef.current += 1;
    setBookmarks([]);
    setBookmarksLoading(false);
    setBookmarksError(null);
    setBookmarksStatus(null);
  }, []);

  const loadBookmarks = useCallback(async (workspaceId: string) => {
    const loadId = bookmarksLoadRef.current + 1;
    bookmarksLoadRef.current = loadId;
    setBookmarksLoading(true);
    setBookmarksError(null);
    try {
      const response = await queryLoomView(workspaceId, "pins", { limit: 100, offset: 0 });
      if (bookmarksLoadRef.current !== loadId) return;
      setBookmarks(response.view_type === "pins" ? response.blocks : []);
      setBookmarksStatus(null);
    } catch (err) {
      if (bookmarksLoadRef.current !== loadId) return;
      setBookmarksError(err instanceof Error ? err.message : "Failed to load bookmarks");
    } finally {
      if (bookmarksLoadRef.current === loadId) {
        setBookmarksLoading(false);
      }
    }
  }, []);

  const loadWorkspaceDetails = useCallback(
    async (id: string) => {
      const loadId = workspaceDetailsLoadRef.current + 1;
      workspaceDetailsLoadRef.current = loadId;
      setDocuments([]);
      setCanvases([]);
      try {
        const [docs, cvs] = await Promise.all([listDocuments(id), listCanvases(id)]);
        if (workspaceDetailsLoadRef.current !== loadId) {
          return;
        }
        setDocuments(docs);
        setCanvases(cvs);
        setWorkspaceError(null);
      } catch (err) {
        if (workspaceDetailsLoadRef.current !== loadId) {
          return;
        }
        setWorkspaceError(err instanceof Error ? err.message : "Failed to load workspace details");
      }
    },
    [],
  );

  const selectWorkspace = useCallback(
    (id: string) => {
      if (activeWorkspaceId === undefined) {
        setSelectedWorkspaceId(id);
        void loadWorkspaceDetails(id);
      }
      onSelectWorkspace?.(id);
    },
    [activeWorkspaceId, loadWorkspaceDetails, onSelectWorkspace],
  );

  const loadWorkspaces = useCallback(async () => {
    // Do NOT clear workspaces on error or at request start; only update them on successful response.
    setLoading(true);
    try {
      const ws = await listWorkspaces();
      setWorkspaces(ws);
      setWorkspaceError(null);
      // Do not auto-select; leave selection to user click.
    } catch (err) {
      setWorkspaceError(err instanceof Error ? err.message : "Failed to load workspaces");
      // keep existing workspaces on failure
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void loadWorkspaces();
  }, [refreshKey, loadWorkspaces]);

  useEffect(() => {
    effectiveWorkspaceIdRef.current = effectiveWorkspaceId;
  }, [effectiveWorkspaceId]);

  useEffect(() => {
    if (activeWorkspaceId === undefined) {
      return;
    }
    setSelectedWorkspaceId(activeWorkspaceId);
    if (activeWorkspaceId) {
      void loadWorkspaceDetails(activeWorkspaceId);
    } else {
      clearWorkspaceDetails();
    }
  }, [activeWorkspaceId, clearWorkspaceDetails, loadWorkspaceDetails]);

  useEffect(() => {
    if (effectiveWorkspaceId) {
      void loadBookmarks(effectiveWorkspaceId);
    } else {
      clearBookmarks();
    }
  }, [clearBookmarks, effectiveWorkspaceId, loadBookmarks]);

  useEffect(() => {
    const handleDocumentDeleted = (event: Event) => {
      const detail = (event as CustomEvent<DocumentDeletedDetail>).detail;
      if (!detail) return;
      setDocuments((prev) => prev.filter((doc) => doc.id !== detail.documentId));
    };
    const handleCanvasDeleted = (event: Event) => {
      const detail = (event as CustomEvent<CanvasDeletedDetail>).detail;
      if (!detail) return;
      setCanvases((prev) => prev.filter((canvas) => canvas.id !== detail.canvasId));
    };
    const handleRefreshWorkspaces = () => {
      void loadWorkspaces();
    };
    const handleLoomBookmarksChanged = (event: Event) => {
      const detail = (event as CustomEvent<LoomBookmarksChangedDetail>).detail;
      const workspaceId = effectiveWorkspaceIdRef.current;
      if (!workspaceId) return;
      if (detail?.workspaceId && detail.workspaceId !== workspaceId) return;
      void loadBookmarks(workspaceId);
    };

    window.addEventListener("handshake:document-deleted", handleDocumentDeleted);
    window.addEventListener("handshake:canvas-deleted", handleCanvasDeleted);
    window.addEventListener("handshake:refresh-workspaces", handleRefreshWorkspaces);
    window.addEventListener("handshake:loom-bookmarks-changed", handleLoomBookmarksChanged);
    return () => {
      window.removeEventListener("handshake:document-deleted", handleDocumentDeleted);
      window.removeEventListener("handshake:canvas-deleted", handleCanvasDeleted);
      window.removeEventListener("handshake:refresh-workspaces", handleRefreshWorkspaces);
      window.removeEventListener("handshake:loom-bookmarks-changed", handleLoomBookmarksChanged);
    };
  }, [loadBookmarks, loadWorkspaces]);

  async function handleCreateWorkspace() {
    const name = window.prompt("Workspace name?");
    if (!name) return;
    try {
      const ws = await createWorkspace(name);
      setWorkspaces((prev) => [...prev, ws]);
      selectWorkspace(ws.id);
    } catch (err) {
      setWorkspaceError(err instanceof Error ? err.message : "Failed to create workspace");
    }
  }

  async function handleDeleteWorkspace() {
    if (!effectiveWorkspaceId) return;
    const confirmed = window.confirm("Delete this workspace and all its documents/canvases? This cannot be undone.");
    if (!confirmed) return;
    try {
      await deleteWorkspace(effectiveWorkspaceId);
      setWorkspaces((prev) => prev.filter((w) => w.id !== effectiveWorkspaceId));
      if (activeWorkspaceId === undefined) {
        setSelectedWorkspaceId(null);
      }
      clearWorkspaceDetails();
      onSelectDocument(null);
      onSelectCanvas(null);
      onWorkspaceDeleted();
    } catch (err) {
      setWorkspaceError(err instanceof Error ? err.message : "Failed to delete workspace");
    }
  }

  async function handleCreateDocument() {
    const workspaceId = effectiveWorkspaceId;
    if (!workspaceId) return;
    const title = window.prompt("Document title?");
    if (!title) return;
    try {
      const doc = await createDocument(workspaceId, title);
      if (effectiveWorkspaceIdRef.current !== workspaceId) {
        return;
      }
      setDocuments((prev) => [...prev, doc]);
      onSelectDocument(doc.id);
      onSelectCanvas(null);
    } catch (err) {
      setWorkspaceError(err instanceof Error ? err.message : "Failed to create document");
    }
  }

  async function handleCreateCanvas() {
    const workspaceId = effectiveWorkspaceId;
    if (!workspaceId) return;
    const title = window.prompt("Canvas title?");
    if (!title) return;
    try {
      const canvas = await createCanvas(workspaceId, title);
      if (effectiveWorkspaceIdRef.current !== workspaceId) {
        return;
      }
      setCanvases((prev) => [...prev, canvas]);
      onSelectCanvas(canvas.id);
      onSelectDocument(null);
    } catch (err) {
      setWorkspaceError(err instanceof Error ? err.message : "Failed to create canvas");
    }
  }

  async function handleRemoveBookmark(block: LoomBlock) {
    const workspaceId = effectiveWorkspaceId;
    if (!workspaceId) return;
    setBookmarksStatus(null);
    setBookmarksError(null);
    try {
      const pinOrderCleared = await setLoomBlockPinOrder(workspaceId, block.block_id, null);
      const updated = await updateLoomBlock(workspaceId, block.block_id, { pinned: false });
      if (effectiveWorkspaceIdRef.current !== workspaceId) return;
      const updatedBlock = { ...updated, pin_order: pinOrderCleared.pin_order ?? null };
      setBookmarks((prev) => prev.filter((item) => item.block_id !== block.block_id));
      setBookmarksStatus("Bookmark removed");
      window.dispatchEvent(
        new CustomEvent("handshake:loom-block-updated", {
          detail: { workspaceId, block: updatedBlock },
        }),
      );
    } catch (err) {
      if (effectiveWorkspaceIdRef.current !== workspaceId) return;
      setBookmarksError(err instanceof Error ? err.message : "Failed to remove bookmark");
    }
  }

  function handleOpenBookmark(block: LoomBlock) {
    const documentId = block.document_id?.trim();
    if (documentId) {
      onSelectDocument(documentId);
      onSelectCanvas(null);
      return;
    }
    onOpenLoomBlock?.(block.block_id);
  }

  return (
    <aside className="sidebar">
      <section>
        <h3>Workspaces</h3>
        <div className="button-row">
          <button onClick={handleCreateWorkspace}>New Workspace</button>
          <button onClick={handleDeleteWorkspace} disabled={!effectiveWorkspaceId}>
            Delete Workspace
          </button>
        </div>
        {loading && <p className="muted">Loading...</p>}
        {workspaceError && (
          <div
            className="content-card"
            style={{ padding: "8px 10px", background: "#fff6f2", border: "1px solid #f4b8a7", marginTop: 8 }}
          >
            <p className="muted" style={{ marginBottom: 8 }}>
              Could not refresh the workspace list. Your existing workspaces are safe; this is likely a temporary
              connection issue. You can continue using the list below or press Retry.
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
              className={`list-item ${effectiveWorkspaceId === ws.id ? "selected" : ""}`}
              onClick={() => selectWorkspace(ws.id)}
            >
              <span>{ws.name || "Untitled"}</span>
            </li>
          ))}
          {workspaces.length === 0 && !loading && <li className="muted">No workspaces yet.</li>}
        </ul>
      </section>

      <section>
        <h3>Bookmarks</h3>
        {bookmarksLoading ? <p className="muted">Loading bookmarks...</p> : null}
        {bookmarksError ? (
          <div className="content-card" style={{ padding: "8px 10px", background: "#fff6f2", border: "1px solid #f4b8a7", marginTop: 8 }}>
            <p className="muted" style={{ marginBottom: 8 }}>
              {bookmarksError}
            </p>
            <button
              type="button"
              onClick={() => {
                if (effectiveWorkspaceId) void loadBookmarks(effectiveWorkspaceId);
              }}
              disabled={!effectiveWorkspaceId || bookmarksLoading}
            >
              Retry
            </button>
          </div>
        ) : null}
        {bookmarksStatus ? (
          <p className="muted" role="status" data-testid="loom-bookmarks.status">
            {bookmarksStatus}
          </p>
        ) : null}
        <ul className="list loom-bookmarks-tree" data-testid="loom-bookmarks-tree">
          {bookmarks.map((block) => {
            const idPart = stablePart(block.block_id);
            const kind = bookmarkKind(block);
            const canOpen = Boolean(block.document_id?.trim() || onOpenLoomBlock);
            return (
              <li
                key={block.block_id}
                className="list-item loom-bookmark"
                data-testid={`loom-bookmark.${idPart}`}
                data-bookmark-kind={kind}
                data-block-id={block.block_id}
              >
                <button
                  type="button"
                  className="loom-bookmark__open"
                  onClick={() => handleOpenBookmark(block)}
                  disabled={!canOpen}
                  data-testid={`loom-bookmark.${idPart}.open`}
                >
                  <span>{blockTitle(block)}</span>
                  <small>{kind}</small>
                </button>
                <button
                  type="button"
                  className="loom-bookmark__remove"
                  aria-label={`Remove bookmark ${blockTitle(block)}`}
                  onClick={() => void handleRemoveBookmark(block)}
                  data-testid={`loom-bookmark.${idPart}.remove`}
                >
                  Remove
                </button>
              </li>
            );
          })}
          {bookmarks.length === 0 && !bookmarksLoading ? <li className="muted">No bookmarks.</li> : null}
        </ul>
      </section>

      <section>
        <h3>Documents</h3>
        <button onClick={handleCreateDocument} disabled={!effectiveWorkspaceId}>
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
        <button onClick={handleCreateCanvas} disabled={!effectiveWorkspaceId}>
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
