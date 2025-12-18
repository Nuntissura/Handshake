import { useEffect, useMemo, useState } from "react";
import { JSONContent } from "@tiptap/core";
import {
  Block,
  BlockInput,
  DocumentWithBlocks,
  WorkflowRun,
  createJob,
  deleteDocument,
  getDocument,
  updateDocumentBlocks,
} from "../lib/api";
import { TiptapEditor } from "./TiptapEditor";
import { logEvent } from "../state/debugEvents";

type Props = {
  documentId: string | null;
  onDeleted: () => void;
};

export function DocumentView({ documentId, onDeleted }: Props) {
  const [doc, setDoc] = useState<DocumentWithBlocks | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [editorContent, setEditorContent] = useState<JSONContent | null>(null);
  const [isSaving, setIsSaving] = useState(false);
  const [saveError, setSaveError] = useState<string | null>(null);
  const [isDeleting, setIsDeleting] = useState(false);
  const [deleteError, setDeleteError] = useState<string | null>(null);
  const [lastSavedAt, setLastSavedAt] = useState<string | null>(null);
  const [aiJobStatus, setAiJobStatus] = useState<"idle" | "pending" | "success" | "error">("idle");
  const [aiJobMessage, setAiJobMessage] = useState<string | null>(null);

  useEffect(() => {
    if (!documentId) {
      setDoc(null);
      setError(null);
      setEditorContent(null);
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
        setEditorContent(blocksToTiptap(response.blocks));
        logEvent({ type: "doc-load", targetId: documentId, result: "ok" });
      } catch (err) {
        const message = err instanceof Error ? err.message : "Failed to load document";
        if (isDocumentNotFound(err)) {
          window.dispatchEvent(
            new CustomEvent("handshake:document-deleted", { detail: { documentId, workspaceId: null } }),
          );
          onDeleted();
          return;
        }
        setError(message);
        logEvent({ type: "doc-load", targetId: documentId, result: "error", message: String(err) });
      } finally {
        setLoading(false);
      }
    };
    void load();
  }, [documentId, onDeleted]);

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
        <p className="muted">Loading...</p>
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
        Created: {new Date(doc.created_at).toLocaleString()} - Updated: {new Date(doc.updated_at).toLocaleString()}
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
                setDeleteError(null);
                try {
                  const blocks = tiptapToBlocks(editorContent, blocksForDisplay);
                  const updated = await updateDocumentBlocks(documentId, blocks);
                  setLastSavedAt(new Date().toLocaleTimeString());
                  setDoc({
                    ...doc,
                    blocks: updated,
                  });
                  logEvent({ type: "doc-save", targetId: documentId, result: "ok" });
                } catch (err) {
                  setSaveError(err instanceof Error ? err.message : "Failed to save");
                  logEvent({ type: "doc-save", targetId: documentId, result: "error", message: String(err) });
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
                setAiJobStatus("pending");
                setAiJobMessage(null);
                try {
                  const run: WorkflowRun = await createJob("doc_test", "doc-proto-001");
                  setAiJobStatus("success");
                  setAiJobMessage(`Job ${run.job_id} (${run.status})`);
                  logEvent({
                    type: "ai-job",
                    targetId: documentId,
                    result: "ok",
                    message: `job_id=${run.job_id}`,
                  });
                } catch (err) {
                  const message = err instanceof Error ? err.message : "Failed to start AI job";
                  setAiJobStatus("error");
                  setAiJobMessage(message);
                  logEvent({
                    type: "ai-job",
                    targetId: documentId,
                    result: "error",
                    message: String(err),
                  });
                }
              }}
              disabled={isSaving || isDeleting || aiJobStatus === "pending" || !documentId}
            >
              {aiJobStatus === "pending" ? "Starting AI job..." : "Run AI Job"}
            </button>
            <button
              onClick={async () => {
                if (!documentId) return;
                setLoading(true);
                setSaveError(null);
                setDeleteError(null);
                try {
                  const refreshed = await getDocument(documentId);
                  setDoc(refreshed);
                  setEditorContent(blocksToTiptap(refreshed.blocks));
                  logEvent({ type: "doc-load", targetId: documentId, result: "ok" });
                } catch (err) {
                  setSaveError(err instanceof Error ? err.message : "Failed to reload");
                  logEvent({ type: "doc-load", targetId: documentId, result: "error", message: String(err) });
                } finally {
                  setLoading(false);
                }
              }}
              disabled={isSaving || !documentId}
            >
              Reload
            </button>
            <button
              onClick={async () => {
                if (!documentId) return;
                const confirmed = window.confirm("Delete this document? This cannot be undone.");
                if (!confirmed) return;
                setIsDeleting(true);
                setDeleteError(null);
                try {
                  await deleteDocument(documentId);
                  const workspaceId = doc?.workspace_id ?? null;
                  window.dispatchEvent(
                    new CustomEvent("handshake:document-deleted", { detail: { documentId, workspaceId } }),
                  );
                  logEvent({ type: "doc-delete", targetId: documentId, result: "ok" });
                  onDeleted();
                } catch (err) {
                  if (isDocumentNotFound(err)) {
                    const workspaceId = doc?.workspace_id ?? null;
                    window.dispatchEvent(
                      new CustomEvent("handshake:document-deleted", { detail: { documentId, workspaceId } }),
                    );
                    logEvent({ type: "doc-delete", targetId: documentId, result: "ok" });
                    onDeleted();
                    return;
                  }
                  const message = err instanceof Error ? err.message : "Failed to delete document";
                  setDeleteError(message);
                  logEvent({ type: "doc-delete", targetId: documentId, result: "error", message: String(err) });
                } finally {
                  setIsDeleting(false);
                }
              }}
              disabled={isSaving || isDeleting || !documentId}
            >
              {isDeleting ? "Deleting..." : "Delete"}
            </button>
          </div>
        </div>
        <TiptapEditor initialContent={editorContent} onChange={setEditorContent} />
        <div className="document-editor__status">
          {lastSavedAt && <span className="muted">Saved at {lastSavedAt}</span>}
          {saveError && <span className="muted">Error: {saveError}</span>}
          {deleteError && <span className="muted">Error: {deleteError}</span>}
          {aiJobMessage && (
            <span className={aiJobStatus === "error" ? "muted" : "muted"}>
              AI Job: {aiJobMessage}
            </span>
          )}
        </div>
      </div>

      <div className="document-blocks">
        <h3 className="document-blocks__title">Storage blocks (debug, read-only)</h3>
        {blocksForDisplay.length === 0 ? (
          <p className="muted">No blocks yet.</p>
        ) : (
          <div className="document-blocks__list">
            {blocksForDisplay.map((block) => (
              <div key={block.id} className="content-card document-blocks__card">
                <strong>
                  {block.sequence}. {block.kind}
                </strong>
                <p className="muted">{block.raw_content.slice(0, 200)}</p>
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  );
}

const EMPTY_DOC: JSONContent = { type: "doc", content: [] };

function blocksToTiptap(blocks: Block[]): JSONContent {
  if (!blocks || blocks.length === 0) return EMPTY_DOC;
  const sorted = [...blocks].sort((a, b) => a.sequence - b.sequence);
  const content = sorted
    .map((block) => blockToNode(block))
    .filter((node): node is JSONContent => Boolean(node));
  return { type: "doc", content };
}

function blockToNode(block: Block): JSONContent | null {
  const maybeDerived = block.derived_content as JSONContent | null;
  if (maybeDerived && typeof maybeDerived === "object" && "type" in maybeDerived) {
    return maybeDerived;
  }

  const plainText = block.raw_content ?? "";
  const kind = block.kind ?? "paragraph";

  switch (true) {
    case kind.startsWith("heading_"): {
      const levelStr = kind.split("_")[1];
      const level = Number(levelStr);
      return makeHeading(isFinite(level) ? level : 1, plainText);
    }
    case kind === "heading":
      return makeHeading(1, plainText);
    case kind === "bulletList":
      return makeListNode("bulletList", plainText);
    case kind === "orderedList":
      return makeListNode("orderedList", plainText);
    case kind === "codeBlock":
      return {
        type: "codeBlock",
        content: textNodeOrEmpty(plainText),
      };
    case kind === "blockquote":
      return {
        type: "blockquote",
        content: [{ type: "paragraph", content: textNodeOrEmpty(plainText) }],
      };
    default:
      return {
        type: "paragraph",
        content: textNodeOrEmpty(plainText),
      };
  }
}

function makeHeading(level: number, text: string): JSONContent {
  return {
    type: "heading",
    attrs: { level: Math.min(Math.max(level, 1), 3) },
    content: textNodeOrEmpty(text),
  };
}

function makeListNode(type: "bulletList" | "orderedList", text: string): JSONContent {
  const items = text
    .split("\n")
    .map((s) => s.trim())
    .filter(Boolean)
    .map<JSONContent>((item) => ({
      type: "listItem",
      content: [{ type: "paragraph", content: textNodeOrEmpty(item) }],
    }));
  return { type, content: items.length > 0 ? items : [{ type: "listItem", content: [{ type: "paragraph" }] }] };
}

function tiptapToBlocks(docJson: JSONContent | null, existing: Block[]): BlockInput[] {
  const content = docJson?.content ?? [];
  const blocks: BlockInput[] = [];

  content.forEach((node, index) => {
    if (!node || typeof node !== "object") {
      return;
    }
    const existingId = index < existing.length ? existing[index].id : undefined;
    const raw = extractPlainText(node);
    const kind = kindFromNode(node);

    blocks.push({
      id: existingId,
      kind,
      sequence: index,
      raw_content: raw,
      display_content: raw,
    });
  });

  return blocks;
}

function kindFromNode(node: JSONContent): string {
  if (node.type === "heading") {
    const level = Number((node.attrs as { level?: unknown } | undefined)?.level);
    if (Number.isFinite(level)) {
      return `heading_${level}`;
    }
  }
  return node.type ?? "paragraph";
}

function extractPlainText(node: JSONContent): string {
  if (!node) return "";
  if (node.type === "text" && typeof node.text === "string") {
    return node.text;
  }

  const childContent = node.content ?? [];
  if (!Array.isArray(childContent) || childContent.length === 0) return "";

  return childContent
    .map((child) => extractPlainText(child))
    .filter(Boolean)
    .join(node.type === "listItem" ? "\n" : node.type?.includes("List") ? "\n" : " ");
}

function textNodeOrEmpty(text: string): JSONContent[] {
  return text ? [{ type: "text", text }] : [];
}

function isDocumentNotFound(err: unknown): boolean {
  const message = err instanceof Error ? err.message : String(err);
  return message.toLowerCase().includes("document_not_found") || message.includes("404");
}
