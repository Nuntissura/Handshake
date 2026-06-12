// WP-KERNEL-009 RichDocumentCore (MT-145..MT-160): the editor UI wired to the
// REAL backend RichDocument authority (knowledge_rich_documents via
// /knowledge/documents) — NO mocks. Loads the typed block tree, saves with
// optimistic concurrency (MT-149), and surfaces history (MT-156), embeds
// (MT-152) + the broken-embed repair queue (MT-153), and backlinks (MT-155).
//
// Stable selectors (data-testid + data-stable-id) are exposed for document
// load, save, block render, history, embed, and broken-embed states so the
// later TiptapMonaco/Loom GUI groups (MT-161+) and visual-debug can drive this
// surface deterministically. The rich EDITING UX polish (full toolbar, Monaco
// mounting, [[link]] syntax, slash menus) is the NEXT group; this provides the
// data plumbing + block rendering + selectors.

import { useCallback, useEffect, useRef, useState } from "react";
import { JSONContent } from "@tiptap/core";
import {
  RichDocLoad,
  RichDocVersion,
  RichDocEmbed,
  RichDocBacklink,
  loadRichDocument,
  loadRichDocumentHistory,
  listRichDocumentEmbeds,
  listRichDocumentBrokenEmbeds,
  listRichDocumentBacklinks,
  saveRichDocument,
} from "../lib/api";
import { RichTextEditor } from "./RichTextEditor";
import {
  classifySaveError,
  schemaMismatchError,
  type EditorBackendError,
} from "../lib/editor/backend_error";
import { assertEditorSchema } from "../lib/editor/schema_versioning";
import type { EmbedResolverContext } from "../lib/editor/embed_assets";
import { logEvent } from "../state/debugEvents";

type Props = {
  documentId: string;
  onDeleted?: () => void;
};

// MT-244: one embed context object per workspace id, module-cached so the
// RichTextEditor extension memo sees a stable reference across renders.
const EMBED_CONTEXT_CACHE = new Map<string, EmbedResolverContext>();

function embedContextFor(workspaceId: string): EmbedResolverContext {
  let context = EMBED_CONTEXT_CACHE.get(workspaceId);
  if (!context) {
    context = { workspaceId };
    EMBED_CONTEXT_CACHE.set(workspaceId, context);
  }
  return context;
}

export function RichDocumentView({ documentId }: Props) {
  const [load, setLoad] = useState<RichDocLoad | null>(null);
  // Iteration-3 H1: the editor receives ONLY genuinely loaded content
  // (load/reload/conflict restore). The live editing state stays in a ref —
  // feeding onChange JSON back into initialContent was the echo loop that
  // teleported the caret on every keystroke.
  const [loadedContent, setLoadedContent] = useState<JSONContent | null>(null);
  const editorContentRef = useRef<JSONContent | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [isSaving, setIsSaving] = useState(false);
  const [saveError, setSaveError] = useState<string | null>(null);
  const [backendError, setBackendError] = useState<EditorBackendError | null>(null);
  const [lastSavedAt, setLastSavedAt] = useState<string | null>(null);
  const [isDirty, setIsDirty] = useState(false);

  const [versions, setVersions] = useState<RichDocVersion[] | null>(null);
  const [embeds, setEmbeds] = useState<RichDocEmbed[] | null>(null);
  const [brokenEmbeds, setBrokenEmbeds] = useState<RichDocEmbed[] | null>(null);
  const [backlinks, setBacklinks] = useState<RichDocBacklink[] | null>(null);

  const reload = useCallback(async () => {
    setLoading(true);
    setError(null);
    setSaveError(null);
    setBackendError(null);
    setIsDirty(false);
    try {
      const response = await loadRichDocument(documentId);
      setLoad(response);
      // MT-162: reconcile the persisted schema version with the running editor
      // before feeding content into the editor. A newer/unknown schema surfaces
      // as a typed backend error (MT-174) instead of crashing the load.
      const assertion = assertEditorSchema(
        response.document.schema_version,
        response.document.content_json,
      );
      if (assertion.ok) {
        setLoadedContent(assertion.content as JSONContent);
        editorContentRef.current = assertion.content as JSONContent;
      } else {
        setLoadedContent(response.document.content_json as JSONContent);
        editorContentRef.current = response.document.content_json as JSONContent;
        setBackendError(schemaMismatchError(assertion.reason));
      }
      logEvent({ type: "doc-load", targetId: documentId, result: "ok" });
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to load rich document");
      logEvent({ type: "doc-load", targetId: documentId, result: "error", message: String(err) });
    } finally {
      setLoading(false);
    }
  }, [documentId]);

  useEffect(() => {
    void reload();
  }, [reload]);

  const refreshSidecars = useCallback(async () => {
    try {
      const [history, embedResp, brokenResp, backlinkResp] = await Promise.all([
        loadRichDocumentHistory(documentId),
        listRichDocumentEmbeds(documentId),
        listRichDocumentBrokenEmbeds(documentId),
        listRichDocumentBacklinks(documentId),
      ]);
      setVersions(history.versions);
      setEmbeds(embedResp.embeds);
      setBrokenEmbeds(brokenResp.broken_embeds);
      setBacklinks(backlinkResp.backlinks);
    } catch (err) {
      // Sidecars are non-fatal to editing; surface but do not block.
      logEvent({ type: "doc-load", targetId: documentId, result: "error", message: String(err) });
    }
  }, [documentId]);

  useEffect(() => {
    if (load) void refreshSidecars();
  }, [load, refreshSidecars]);

  const onSave = useCallback(async () => {
    const editorContent = editorContentRef.current;
    if (!load || !editorContent) return;
    setIsSaving(true);
    setSaveError(null);
    setBackendError(null);
    try {
      const result = await saveRichDocument(
        documentId,
        load.document.doc_version,
        editorContent as never,
      );
      setLoad({ ...load, document: result.document, tree: load.tree });
      setLastSavedAt(new Date().toLocaleTimeString());
      setIsDirty(false);
      logEvent({ type: "doc-save", targetId: documentId, result: "ok" });
      // Backlinks + history change on save.
      void refreshSidecars();
    } catch (err) {
      const message = err instanceof Error ? err.message : "Failed to save";
      setSaveError(message);
      // MT-174: classify into a typed inline backend error (conflict/schema/save).
      setBackendError(classifySaveError(err));
      logEvent({ type: "doc-save", targetId: documentId, result: "error", message: String(err) });
    } finally {
      setIsSaving(false);
    }
  }, [load, documentId, refreshSidecars]);

  if (loading && !load) {
    return (
      <div className="content-card" data-testid="rich-document-loading">
        <h2>Document</h2>
        <p className="muted">Loading...</p>
      </div>
    );
  }

  if (error) {
    return (
      <div className="content-card" data-testid="rich-document-error">
        <h2>Document</h2>
        <p className="muted">Error: {error}</p>
      </div>
    );
  }

  if (!load) return null;

  const doc = load.document;
  // Stable per-workspace embed context (MT-244): a fresh object every render
  // would rebuild the whole extension set, so reuse one per workspace id.
  const embedContext = embedContextFor(doc.workspace_id);

  return (
    <div
      className="content-card"
      data-testid="rich-document-view"
      data-stable-id={doc.rich_document_id}
      data-doc-version={doc.doc_version}
      data-authority-label={doc.authority_label}
      data-schema-matches={String(load.tree.schema_matches)}
    >
      <h2 data-testid="rich-document-title">{doc.title}</h2>
      <p className="muted">
        Workspace: {doc.workspace_id}
        {doc.project_ref ? ` - Project: ${doc.project_ref}` : ""}
        {doc.folder_ref ? ` - Folder: ${doc.folder_ref}` : ""}
      </p>
      <p className="muted">
        Authority: {doc.authority_label} - v{doc.doc_version} - owner{" "}
        {doc.owner_actor_kind ?? "unknown"}
      </p>

      <div className="document-editor">
        <div className="document-editor__header">
          <h3>Editor</h3>
          <div className="document-editor__actions">
            <button
              data-testid="rich-document-save"
              onClick={() => void onSave()}
              disabled={isSaving || !isDirty}
            >
              {isSaving ? "Saving..." : "Save"}
            </button>
            <button data-testid="rich-document-reload" onClick={() => void reload()} disabled={isSaving}>
              Reload
            </button>
          </div>
        </div>

        <div className="document-editor__body">
          <div className="document-editor__main">
            <RichTextEditor
              initialContent={loadedContent}
              onChange={(next) => {
                // H1: track live content WITHOUT echoing it back down.
                editorContentRef.current = next;
                setIsDirty(true);
              }}
              backendError={backendError}
              // MT-244: bind media embed NodeViews to the document's workspace
              // so [[HS_images:…]]/[[video:…]]/album/slideshow resolve REAL
              // backend assets; memoized so the editor is not rebuilt per render.
              embedContext={embedContext}
              documentTitle={doc.title}
            />
            <div className="document-editor__status" data-testid="rich-document-status">
              {lastSavedAt && (
                <span className="muted" data-testid="rich-document-saved-at">
                  Saved at {lastSavedAt}
                </span>
              )}
              {saveError && (
                <span className="muted" data-testid="rich-document-save-error">
                  Error: {saveError}
                </span>
              )}
            </div>
          </div>
        </div>
      </div>

      {/* Block tree render with stable per-block selectors (MT-146/148). */}
      <div className="document-blocks" data-testid="rich-document-blocks">
        <h3 className="document-blocks__title">Blocks ({load.tree.blocks.length})</h3>
        <div className="document-blocks__list">
          {load.tree.blocks.map((block) => (
            <div
              key={block.block_id}
              className="content-card document-blocks__card"
              data-testid="rich-document-block"
              data-stable-id={block.block_id}
              data-block-kind={block.kind}
            >
              <strong>
                {block.sequence}. {block.kind}
                {block.heading_level ? ` (h${block.heading_level})` : ""}
              </strong>
              <p className="muted">{block.content.derived.preview}</p>
            </div>
          ))}
        </div>
      </div>

      {/* Embeds + broken-embed repair queue (MT-152/153). */}
      <div className="document-embeds" data-testid="rich-document-embeds">
        <h3>Embeds ({embeds?.length ?? 0})</h3>
        {(embeds ?? []).map((embed) => (
          <div
            key={embed.embed_id}
            data-testid="rich-document-embed"
            data-stable-id={embed.embed_id}
            data-embed-state={embed.repair_state}
            className="muted"
          >
            [{embed.ref_kind}] {embed.ref_value}
            {embed.repair_state === "broken" ? ` - BROKEN: ${embed.repair_reason}` : ""}
          </div>
        ))}
        {(brokenEmbeds?.length ?? 0) > 0 && (
          <div data-testid="rich-document-broken-embeds" className="error">
            {brokenEmbeds!.length} broken embed(s) need repair.
          </div>
        )}
      </div>

      {/* Backlinks (MT-155). */}
      <div className="document-backlinks" data-testid="rich-document-backlinks">
        <h3>Backlinks ({backlinks?.length ?? 0})</h3>
        {(backlinks ?? []).map((b) => (
          <div key={b.backlink_id} className="muted" data-testid="rich-document-backlink" data-link-kind={b.link_kind}>
            {b.link_kind} -&gt; {b.target}
          </div>
        ))}
      </div>

      {/* Revision history (MT-156). */}
      <div className="document-history" data-testid="rich-document-history">
        <h3>History ({versions?.length ?? 0})</h3>
        {(versions ?? []).map((v) => (
          <div key={v.doc_version} className="muted" data-testid="rich-document-version" data-doc-version={v.doc_version}>
            v{v.doc_version} - {new Date(v.created_at).toLocaleString()}
          </div>
        ))}
      </div>
    </div>
  );
}
