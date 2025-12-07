import { useEffect, useMemo, useState } from "react";
import { BlockInput, DocumentWithBlocks, getDocument, updateDocumentBlocks } from "../lib/api";

type Props = {
  documentId: string | null;
};

export function DocumentView({ documentId }: Props) {
  const [doc, setDoc] = useState<DocumentWithBlocks | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [editorText, setEditorText] = useState<string>("");
  const [isSaving, setIsSaving] = useState(false);
  const [saveError, setSaveError] = useState<string | null>(null);
  const [lastSavedAt, setLastSavedAt] = useState<string | null>(null);

  useEffect(() => {
    if (!documentId) {
      setDoc(null);
      setError(null);
      setEditorText("");
      return;
    }
    const load = async () => {
      setLoading(true);
      setError(null);
      setSaveError(null);
      setLastSavedAt(null);
      try {
        const response = await getDocument(documentId);
        setDoc(response);
        setEditorText(blocksToEditorText(response));
      } catch (err) {
        setError(err instanceof Error ? err.message : "Failed to load document");
      } finally {
        setLoading(false);
      }
    };
    void load();
  }, [documentId]);

  const blocksForDisplay = useMemo(() => doc?.blocks ?? [], [doc]);

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
      <div className="document-editor">
        <div className="document-editor__header">
          <h3>Editor</h3>
          <div className="document-editor__actions">
            <button
              onClick={async () => {
                if (!documentId) return;
                setIsSaving(true);
                setSaveError(null);
                try {
                  const blocks = editorTextToBlocks(editorText, blocksForDisplay);
                  const updated = await updateDocumentBlocks(documentId, blocks);
                  setLastSavedAt(new Date().toLocaleTimeString());
                  setDoc({
                    ...doc,
                    blocks: updated,
                  });
                } catch (err) {
                  setSaveError(err instanceof Error ? err.message : "Failed to save");
                } finally {
                  setIsSaving(false);
                }
              }}
              disabled={isSaving || !documentId}
            >
              {isSaving ? "Saving..." : "Save"}
            </button>
            <button
              onClick={async () => {
                if (!documentId) return;
                setLoading(true);
                setSaveError(null);
                try {
                  const refreshed = await getDocument(documentId);
                  setDoc(refreshed);
                  setEditorText(blocksToEditorText(refreshed));
                } catch (err) {
                  setSaveError(err instanceof Error ? err.message : "Failed to reload");
                } finally {
                  setLoading(false);
                }
              }}
              disabled={isSaving || !documentId}
            >
              Reload
            </button>
          </div>
        </div>
        <textarea
          className="document-editor__textarea"
          rows={12}
          value={editorText}
          onChange={(e) => setEditorText(e.target.value)}
          placeholder="Start typing your document. Separate paragraphs with a blank line."
        />
        <div className="document-editor__status">
          {lastSavedAt && <span className="muted">Saved at {lastSavedAt}</span>}
          {saveError && <span className="muted">Error: {saveError}</span>}
        </div>
      </div>

      <h3>Blocks</h3>
      {blocksForDisplay.length === 0 ? (
        <p className="muted">No blocks yet.</p>
      ) : (
        <ul className="list-inline">
          {blocksForDisplay.map((block) => (
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

function blocksToEditorText(document: DocumentWithBlocks): string {
  if (document.blocks.length === 0) return "";
  const sorted = [...document.blocks].sort((a, b) => a.sequence - b.sequence);
  return sorted.map((b) => b.raw_content).join("\n\n");
}

function editorTextToBlocks(text: string, existing: DocumentWithBlocks["blocks"]): BlockInput[] {
  const segments = text
    .split(/\n{2,}/)
    .map((s) => s.trim())
    .filter((s) => s.length > 0);

  const blocks: BlockInput[] = [];
  for (let i = 0; i < segments.length; i++) {
    const existingId = i < existing.length ? existing[i].id : undefined;
    blocks.push({
      id: existingId,
      kind: "paragraph",
      sequence: i,
      raw_content: segments[i],
      display_content: segments[i],
      derived_content: existingId ? existing[i].derived_content : undefined,
    });
  }

  return blocks;
}
