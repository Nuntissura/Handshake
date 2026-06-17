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

import { useCallback, useEffect, useMemo, useRef, useState, useSyncExternalStore } from "react";
import { JSONContent } from "@tiptap/core";
import { Doc as YDoc } from "yjs";
import {
  type RichDocument,
  RichDocLoad,
  RichDocVersion,
  RichDocEmbed,
  RichDocBacklink,
  loadRichDocument,
  loadRichDocumentHistory,
  loadRichDocumentVersion,
  listRichDocumentEmbeds,
  listRichDocumentBrokenEmbeds,
  listRichDocumentBacklinks,
  saveRichDocument,
  loadRichDocumentDraft,
  upsertRichDocumentDraft,
  clearRichDocumentDraft,
} from "../lib/api";
import { RichTextEditor } from "./RichTextEditor";
import {
  classifySaveError,
  codeIntegrityError,
  schemaMismatchError,
  schemaSaveBlockedError,
  type EditorBackendError,
} from "../lib/editor/backend_error";
import { assertEditorSchema } from "../lib/editor/schema_versioning";
import { verifyDocCodeBlockIntegrity } from "../lib/editor/code_block_serialization";
import { saveTextToFile } from "../lib/editor/save_to_file";
import {
  applyRichDocumentMergePlan,
  buildRichDocumentDiff,
  planRichDocumentMerge,
  type RichDocumentDiff,
  type RichDocumentJson,
  type RichDocumentMergeChoices,
} from "../lib/editor/document_diff_merge";
import type { EmbedResolverContext } from "../lib/editor/embed_assets";
import type { EditorCommandPaletteRequest } from "../lib/editor/editor_command_palette_request";
import type { EditorFindRequest } from "../lib/editor/editor_find_request";
import { logEvent } from "../state/debugEvents";

type Props = {
  documentId: string;
  onDeleted?: () => void;
  onDirtyChange?: (dirty: boolean) => void;
  commandPaletteRequest?: EditorCommandPaletteRequest | null;
  findRequest?: EditorFindRequest | null;
};

/**
 * Iteration-3 H5: a preserved copy of the operator's local edits, taken before
 * any path that could lose them (save conflict, document switch while dirty).
 * Never discarded silently — only an explicit operator action clears it.
 */
interface LocalSnapshot {
  reason: "conflict" | "doc_switch" | "draft_recovery";
  takenAtUtc: string;
  /** The document the edits belong to (restore is offered only on that doc). */
  forDocumentId: string;
  /** Server/base content at the moment the local snapshot was taken. */
  baseContent: JSONContent | null;
  content: JSONContent;
}

interface HistoryDiffState {
  leftVersion: number;
  rightVersion: number;
  diff: RichDocumentDiff;
}

interface ReloadOptions {
  resetCollaboration?: boolean;
}

// MT-244: one embed context object per workspace id, module-cached so the
// RichTextEditor extension memo sees a stable reference across renders.
const EMBED_CONTEXT_CACHE = new Map<string, EmbedResolverContext>();
interface CollaborationDocCacheEntry {
  document: YDoc;
  activeCount: number;
  lastAccessed: number;
  snapshot: CollaborationDocLease;
}

interface CollaborationDocLease {
  key: string;
  document: YDoc;
}

interface RichDocumentAuthorityEntry {
  document: RichDocument | null;
  listeners: Set<() => void>;
}

const COLLABORATION_DOC_CACHE = new Map<string, CollaborationDocCacheEntry>();
const COLLABORATION_DOC_KEY_ALIASES = new Map<string, string>();
const RICH_DOCUMENT_AUTHORITY_CACHE = new Map<string, RichDocumentAuthorityEntry>();
const MAX_COLLABORATION_DOC_CACHE_SIZE = 64;
let collaborationDocAccessCounter = 0;

function embedContextFor(workspaceId: string): EmbedResolverContext {
  let context = EMBED_CONTEXT_CACHE.get(workspaceId);
  if (!context) {
    context = { workspaceId };
    EMBED_CONTEXT_CACHE.set(workspaceId, context);
  }
  return context;
}

function defaultRichDocumentCrdtDocumentId(documentId: string): string {
  return documentId.startsWith("KRD-") ? documentId.replace(/^KRD-/, "KCRDT-") : `KCRDT-${documentId}`;
}

function collaborationDocKey(documentId: string, crdtDocumentId: string | null | undefined): string {
  return resolveCollaborationDocKey(crdtDocumentId ?? defaultRichDocumentCrdtDocumentId(documentId));
}

function resolveCollaborationDocKey(key: string): string {
  let resolved = key;
  const seen = new Set<string>();
  while (COLLABORATION_DOC_KEY_ALIASES.has(resolved) && !seen.has(resolved)) {
    seen.add(resolved);
    resolved = COLLABORATION_DOC_KEY_ALIASES.get(resolved) ?? resolved;
  }
  return resolved;
}

function markCollaborationDocAccess(entry: CollaborationDocCacheEntry): void {
  collaborationDocAccessCounter += 1;
  entry.lastAccessed = collaborationDocAccessCounter;
}

function pruneCollaborationDocAliasesForKey(evictedKey: string): void {
  for (const [aliasKey, targetKey] of COLLABORATION_DOC_KEY_ALIASES.entries()) {
    if (aliasKey === evictedKey || targetKey === evictedKey) {
      COLLABORATION_DOC_KEY_ALIASES.delete(aliasKey);
    }
  }
}

function trimInactiveCollaborationDocs(protectedKey?: string): void {
  const inactiveEntries = Array.from(COLLABORATION_DOC_CACHE.entries())
    .filter(([key, entry]) => key !== protectedKey && entry.activeCount === 0)
    .sort((left, right) => left[1].lastAccessed - right[1].lastAccessed);

  while (COLLABORATION_DOC_CACHE.size > MAX_COLLABORATION_DOC_CACHE_SIZE && inactiveEntries.length > 0) {
    const [oldestKey, oldestEntry] = inactiveEntries.shift()!;
    COLLABORATION_DOC_CACHE.delete(oldestKey);
    pruneCollaborationDocAliasesForKey(oldestKey);
    oldestEntry.document.destroy();
  }
}

function retainCollaborationDoc(
  documentId: string,
  crdtDocumentId: string | null | undefined,
): CollaborationDocLease {
  const fallbackKey = defaultRichDocumentCrdtDocumentId(documentId);
  const cacheKey = crdtDocumentId ?? resolveCollaborationDocKey(fallbackKey);

  if (crdtDocumentId) {
    const provisionalKey = resolveCollaborationDocKey(fallbackKey);
    const provisionalEntry = COLLABORATION_DOC_CACHE.get(provisionalKey);
    if (provisionalEntry && provisionalKey !== cacheKey && !COLLABORATION_DOC_CACHE.has(cacheKey)) {
      COLLABORATION_DOC_CACHE.delete(provisionalKey);
      provisionalEntry.snapshot = { key: cacheKey, document: provisionalEntry.document };
      COLLABORATION_DOC_CACHE.set(cacheKey, provisionalEntry);
    }
    COLLABORATION_DOC_KEY_ALIASES.set(fallbackKey, cacheKey);
    COLLABORATION_DOC_KEY_ALIASES.set(documentId, cacheKey);
  }

  let entry = COLLABORATION_DOC_CACHE.get(cacheKey);
  if (!entry) {
    const document = new YDoc({ guid: cacheKey });
    entry = {
      document,
      activeCount: 0,
      lastAccessed: 0,
      snapshot: { key: cacheKey, document },
    };
    COLLABORATION_DOC_CACHE.set(cacheKey, entry);
  }
  entry.activeCount += 1;
  markCollaborationDocAccess(entry);
  trimInactiveCollaborationDocs(cacheKey);
  return entry.snapshot;
}

function releaseCollaborationDoc(key: string): void {
  const entry = COLLABORATION_DOC_CACHE.get(resolveCollaborationDocKey(key));
  if (!entry) return;
  entry.activeCount = Math.max(0, entry.activeCount - 1);
  markCollaborationDocAccess(entry);
  trimInactiveCollaborationDocs();
}

function richDocumentAuthorityEntry(documentId: string): RichDocumentAuthorityEntry {
  let entry = RICH_DOCUMENT_AUTHORITY_CACHE.get(documentId);
  if (!entry) {
    entry = { document: null, listeners: new Set() };
    RICH_DOCUMENT_AUTHORITY_CACHE.set(documentId, entry);
  }
  return entry;
}

function latestRichDocumentAuthority(documentId: string): RichDocument | null {
  return RICH_DOCUMENT_AUTHORITY_CACHE.get(documentId)?.document ?? null;
}

function publishRichDocumentAuthority(document: RichDocument): boolean {
  const entry = richDocumentAuthorityEntry(document.rich_document_id);
  const current = entry.document;
  if (current && current.doc_version > document.doc_version) {
    return false;
  }
  if (current && current.doc_version === document.doc_version && current.crdt_document_id && !document.crdt_document_id) {
    return false;
  }
  entry.document = document;
  for (const listener of entry.listeners) listener();
  return true;
}

function subscribeRichDocumentAuthority(documentId: string, listener: () => void): () => void {
  const entry = richDocumentAuthorityEntry(documentId);
  entry.listeners.add(listener);
  return () => {
    entry.listeners.delete(listener);
    if (entry.listeners.size === 0) {
      RICH_DOCUMENT_AUTHORITY_CACHE.delete(documentId);
    }
  };
}

function jsonContentDeepEquals(left: JSONContent | null, right: unknown): boolean {
  try {
    return JSON.stringify(left) === JSON.stringify(right);
  } catch {
    return false;
  }
}

function asRichDocumentJson(content: unknown): RichDocumentJson {
  if (content && typeof content === "object" && !Array.isArray(content)) {
    return content as RichDocumentJson;
  }
  return { type: "doc", content: [] };
}

function diffBlockLeftText(block: RichDocumentDiff["blocks"][number]): string {
  return block.kind === "code" ? block.leftCode?.code ?? "" : block.leftText ?? "";
}

function diffBlockRightText(block: RichDocumentDiff["blocks"][number]): string {
  return block.kind === "code" ? block.rightCode?.code ?? "" : block.rightText ?? "";
}

function diffBlockLanguage(block: RichDocumentDiff["blocks"][number]): string {
  return block.rightCode?.language ?? block.leftCode?.language ?? "plaintext";
}

function useCollaborationDocumentLease(
  documentId: string | null,
  crdtDocumentId: string | null | undefined,
): CollaborationDocLease | null {
  const getSnapshot = useCallback(() => {
    if (!documentId) return null;
    return COLLABORATION_DOC_CACHE.get(collaborationDocKey(documentId, crdtDocumentId))?.snapshot ?? null;
  }, [documentId, crdtDocumentId]);

  const subscribe = useCallback(
    (onStoreChange: () => void) => {
      if (!documentId) {
        return () => {};
      }

      const nextLease = retainCollaborationDoc(documentId, crdtDocumentId);
      onStoreChange();
      return () => releaseCollaborationDoc(nextLease.key);
    },
    [documentId, crdtDocumentId],
  );

  return useSyncExternalStore(subscribe, getSnapshot, getSnapshot);
}

export function RichDocumentView({ documentId, onDirtyChange, commandPaletteRequest = null, findRequest = null }: Props) {
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
  // Iteration-3 H2 (schema fail-closed): a document whose persisted schema the
  // running editor cannot reconcile opens READ-ONLY with saving blocked —
  // ProseMirror drops unknown nodes, so one save of an editable mismatched doc
  // would persist the stripped content. Holds the assertion failure reason.
  const [schemaBlocked, setSchemaBlocked] = useState<string | null>(null);
  const [collaborationResetToken, setCollaborationResetToken] = useState(0);

  // Iteration-3 H5 (save/conflict data loss):
  //   - dirtyGenRef counts edit generations; a save only clears the dirty flag
  //     if NO keystroke landed while the save was in flight (the unconditional
  //     setIsDirty(false) was wiping edits typed during the await).
  //   - isDirtyRef mirrors isDirty for non-render readers (unload guard,
  //     doc-switch snapshot).
  //   - localSnapshot preserves the operator's edits across every lossy path
  //     (conflict, doc switch); cleared ONLY by explicit operator action.
  const dirtyGenRef = useRef(0);
  const isDirtyRef = useRef(false);
  const [localSnapshot, setLocalSnapshot] = useState<LocalSnapshot | null>(null);
  const [mergeChoices, setMergeChoices] = useState<RichDocumentMergeChoices>({});
  const isSavingRef = useRef(false);
  const draftPersistTimerRef = useRef<number | null>(null);

  const [versions, setVersions] = useState<RichDocVersion[] | null>(null);
  const [diffLeftVersion, setDiffLeftVersion] = useState<number | null>(null);
  const [diffRightVersion, setDiffRightVersion] = useState<number | null>(null);
  const [historyDiff, setHistoryDiff] = useState<HistoryDiffState | null>(null);
  const [historyDiffError, setHistoryDiffError] = useState<string | null>(null);
  const [historyDiffLoading, setHistoryDiffLoading] = useState(false);
  const [embeds, setEmbeds] = useState<RichDocEmbed[] | null>(null);
  const [brokenEmbeds, setBrokenEmbeds] = useState<RichDocEmbed[] | null>(null);
  const [backlinks, setBacklinks] = useState<RichDocBacklink[] | null>(null);
  const dirtyReportRef = useRef<{ documentId: string | null; dirty: boolean | null }>({
    documentId: null,
    dirty: null,
  });

  useEffect(() => {
    if (dirtyReportRef.current.documentId !== documentId || dirtyReportRef.current.dirty === null) {
      dirtyReportRef.current = { documentId, dirty: isDirty };
      return;
    }
    if (dirtyReportRef.current.dirty === isDirty) {
      return;
    }
    dirtyReportRef.current = { documentId, dirty: isDirty };
    onDirtyChange?.(isDirty);
  }, [documentId, isDirty, onDirtyChange]);

  const cancelDraftPersist = useCallback(() => {
    if (draftPersistTimerRef.current !== null) {
      window.clearTimeout(draftPersistTimerRef.current);
      draftPersistTimerRef.current = null;
    }
  }, []);

  useEffect(() => () => cancelDraftPersist(), [cancelDraftPersist, documentId]);

  const scheduleDraftPersist = useCallback(
    (content: JSONContent) => {
      if (!load || schemaBlocked !== null) return;
      const authorityDocument = latestRichDocumentAuthority(documentId) ?? load.document;
      cancelDraftPersist();
      draftPersistTimerRef.current = window.setTimeout(() => {
        draftPersistTimerRef.current = null;
        void upsertRichDocumentDraft(
          documentId,
          authorityDocument.doc_version,
          authorityDocument.content_sha256,
          content as never,
        ).catch((err) => {
          logEvent({
            type: "doc-save",
            targetId: documentId,
            result: "error",
            message: `draft persistence failed: ${String(err)}`,
          });
        });
      }, 600);
    },
    [cancelDraftPersist, documentId, load, schemaBlocked],
  );

  useEffect(
    () =>
      subscribeRichDocumentAuthority(documentId, () => {
        const latest = latestRichDocumentAuthority(documentId);
        if (!latest) return;
        setLoad((current) => {
          if (!current || current.document.rich_document_id !== latest.rich_document_id) return current;
          if (
            current.document.doc_version === latest.doc_version &&
            current.document.crdt_document_id === latest.crdt_document_id &&
            current.document.content_sha256 === latest.content_sha256
          ) {
            return current;
          }
          return { ...current, document: latest };
        });
        if (
          !isSavingRef.current &&
          isDirtyRef.current &&
          editorContentRef.current &&
          jsonContentDeepEquals(editorContentRef.current, latest.content_json)
        ) {
          isDirtyRef.current = false;
          setIsDirty(false);
        }
      }),
    [documentId],
  );

  const reload = useCallback(async (options?: ReloadOptions) => {
    setLoading(true);
    setError(null);
    setSaveError(null);
    setBackendError(null);
    // NOTE (H5): localSnapshot is deliberately NOT cleared here — reloading
    // after a conflict must never silently discard the preserved local edits.
    try {
      const response = await loadRichDocument(documentId);
      const latest = latestRichDocumentAuthority(documentId);
      if (latest && latest.doc_version > response.document.doc_version) {
        logEvent({
          type: "doc-load",
          targetId: documentId,
          result: "error",
          message: `stale rich document reload ignored: loaded v${response.document.doc_version} after v${latest.doc_version}`,
        });
        return;
      }
      if (!publishRichDocumentAuthority(response.document)) {
        logEvent({
          type: "doc-load",
          targetId: documentId,
          result: "error",
          message: `rich document reload ignored: loaded v${response.document.doc_version} would downgrade shared authority`,
        });
        return;
      }
      setIsDirty(false);
      isDirtyRef.current = false;
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
        setSchemaBlocked(null);
        // Iteration-3 M9: verify every embedded code block's round-trip hash on
        // load (the check existed but had no product caller). A violation is
        // surfaced as a typed banner; editing stays possible — the backend
        // content_sha256 is the durable authority and a re-save re-mints the
        // editor-layer hashes.
        const integrity = verifyDocCodeBlockIntegrity(assertion.content);
        if (integrity.violations.length > 0) {
          setBackendError(codeIntegrityError(integrity.violations.length, integrity.checked));
          logEvent({
            type: "doc-load",
            targetId: documentId,
            result: "error",
            message: `code-block integrity: ${integrity.violations.length}/${integrity.checked} hash mismatch(es)`,
          });
        }
      } else {
        // H2 fail-closed: render the best-effort view READ-ONLY and block the
        // save path entirely (typed banner explains why). ProseMirror may drop
        // nodes it does not know — read-only + save-block means the stripped
        // rendering can never overwrite the authority record.
        setLoadedContent(response.document.content_json as JSONContent);
        editorContentRef.current = response.document.content_json as JSONContent;
        setSchemaBlocked(assertion.reason);
        setBackendError(schemaMismatchError(assertion.reason));
      }
      if (options?.resetCollaboration) {
        setCollaborationResetToken((token) => token + 1);
      }
      try {
        const draftResponse = await loadRichDocumentDraft(documentId);
        const draft = draftResponse.draft;
        if (
          draft &&
          draft.draft_content_sha256 !== response.document.content_sha256 &&
          !jsonContentDeepEquals(draft.draft_content_json as JSONContent, response.document.content_json)
        ) {
          setLocalSnapshot((current) =>
            current && current.forDocumentId === documentId
              ? current
              : {
                  reason: "draft_recovery",
                  takenAtUtc: draft.updated_at,
                  forDocumentId: documentId,
                  baseContent: response.document.content_json as JSONContent,
                  content: draft.draft_content_json as JSONContent,
                },
          );
        }
      } catch (err) {
        logEvent({
          type: "doc-load",
          targetId: documentId,
          result: "error",
          message: `draft recovery load failed: ${String(err)}`,
        });
      }
      logEvent({ type: "doc-load", targetId: documentId, result: "ok" });
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to load rich document");
      logEvent({ type: "doc-load", targetId: documentId, result: "error", message: String(err) });
    } finally {
      setLoading(false);
    }
  }, [documentId]);

  // Iteration-3 H5 (doc-switch guard): switching documents while dirty must
  // preserve the un-saved edits BEFORE the new document loads over them.
  // Declared before the reload effect so the snapshot is taken first.
  const prevDocIdRef = useRef(documentId);
  useEffect(() => {
    if (prevDocIdRef.current !== documentId) {
      if (isDirtyRef.current && editorContentRef.current) {
        setLocalSnapshot({
          reason: "doc_switch",
          takenAtUtc: new Date().toISOString(),
          forDocumentId: prevDocIdRef.current,
          baseContent: loadedContent,
          content: editorContentRef.current,
        });
      }
      prevDocIdRef.current = documentId;
    }
  }, [documentId, loadedContent]);

  useEffect(() => {
    void reload();
  }, [reload]);

  // Iteration-3 H5 (unload guard): un-saved edits block a silent tab/window
  // close with the standard beforeunload prompt.
  useEffect(() => {
    if (!isDirty) return;
    const guard = (event: BeforeUnloadEvent) => {
      event.preventDefault();
      // Chromium requires returnValue to be set for the prompt to show.
      event.returnValue = "";
    };
    window.addEventListener("beforeunload", guard);
    return () => window.removeEventListener("beforeunload", guard);
  }, [isDirty]);

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
    if (schemaBlocked !== null) {
      // H2 defense-in-depth: the button is disabled, but ANY save path (future
      // keyboard save, programmatic callers) must also refuse.
      setBackendError(schemaSaveBlockedError(schemaBlocked));
      logEvent({
        type: "doc-save",
        targetId: documentId,
        result: "error",
        message: "save blocked: schema mismatch (fail-closed)",
      });
      return;
    }
    setIsSaving(true);
    isSavingRef.current = true;
    setSaveError(null);
    setBackendError(null);
    // H5: remember WHICH edit generation this save captured. Keystrokes typed
    // while the request is in flight bump the generation; the dirty flag may
    // only be cleared if nothing landed in between.
    const generationAtSave = dirtyGenRef.current;
    cancelDraftPersist();
    try {
      const authorityDocument = latestRichDocumentAuthority(documentId) ?? load.document;
      const result = await saveRichDocument(
        documentId,
        authorityDocument.doc_version,
        editorContent as never,
        undefined,
        {
          crdt_document_id:
            authorityDocument.crdt_document_id ??
            load.document.crdt_document_id ??
            defaultRichDocumentCrdtDocumentId(documentId),
        },
      );
      if (!publishRichDocumentAuthority(result.document)) {
        const message = `Save result ignored: saved v${result.document.doc_version} would downgrade shared authority`;
        setSaveError(message);
        setBackendError({
          kind: "conflict",
          message,
          hint: "Your local edits are still marked dirty; save again against the latest document version.",
        });
        logEvent({ type: "doc-save", targetId: documentId, result: "error", message });
        return;
      }
      setLoad((current) => (current ? { ...current, document: result.document } : { ...load, document: result.document }));
      setLastSavedAt(new Date().toLocaleTimeString());
      if (dirtyGenRef.current === generationAtSave) {
        setIsDirty(false);
        isDirtyRef.current = false;
      }
      setLocalSnapshot((current) => (current?.reason === "draft_recovery" ? null : current));
      logEvent({ type: "doc-save", targetId: documentId, result: "ok" });
      // Backlinks + history change on save.
      void refreshSidecars();
    } catch (err) {
      const message = err instanceof Error ? err.message : "Failed to save";
      setSaveError(message);
      // MT-174: classify into a typed inline backend error (conflict/schema/save).
      const classified = classifySaveError(err);
      setBackendError(classified);
      if (classified.kind === "conflict") {
        // H5: preserve the operator's local version BEFORE any reload can
        // replace it — the conflict surface must never silently discard edits.
        // (MT-247 builds the full diff/merge UI; minimum here: both versions
        // stay reachable — local via snapshot, server via Reload.)
        setLocalSnapshot({
          reason: "conflict",
          takenAtUtc: new Date().toISOString(),
          forDocumentId: documentId,
          baseContent: load.document.content_json as JSONContent,
          content: editorContent,
        });
      }
      logEvent({ type: "doc-save", targetId: documentId, result: "error", message: String(err) });
    } finally {
      isSavingRef.current = false;
      setIsSaving(false);
    }
  }, [cancelDraftPersist, load, documentId, refreshSidecars, schemaBlocked]);

  // H5: explicit snapshot actions — restore is offered only on the document
  // the edits belong to; discard is the ONLY way a snapshot disappears.
  const restoreSnapshot = useCallback(() => {
    if (!localSnapshot || localSnapshot.forDocumentId !== documentId) return;
    setLoadedContent(localSnapshot.content);
    editorContentRef.current = localSnapshot.content;
    setCollaborationResetToken((token) => token + 1);
    dirtyGenRef.current += 1;
    isDirtyRef.current = true;
    setIsDirty(true);
  }, [localSnapshot, documentId]);

  const downloadSnapshot = useCallback(() => {
    if (!localSnapshot) return;
    const stamp = localSnapshot.takenAtUtc.replace(/[:.]/g, "-");
    saveTextToFile(
      `${localSnapshot.forDocumentId}-local-${localSnapshot.reason}-${stamp}.json`,
      JSON.stringify(localSnapshot.content, null, 2),
      "application/json",
    );
  }, [localSnapshot]);

  const discardSnapshot = useCallback(async () => {
    const snapshot = localSnapshot;
    if (!snapshot) return;
    if (snapshot.reason === "draft_recovery" && snapshot.forDocumentId === documentId) {
      try {
        await clearRichDocumentDraft(documentId);
      } catch (err) {
        const message = err instanceof Error ? err.message : String(err);
        setSaveError(`Failed to discard recovery draft: ${message}`);
        logEvent({ type: "doc-save", targetId: documentId, result: "error", message });
        return;
      }
    }
    setLocalSnapshot((current) =>
      current?.takenAtUtc === snapshot.takenAtUtc &&
      current.reason === snapshot.reason &&
      current.forDocumentId === snapshot.forDocumentId
        ? null
        : current,
    );
  }, [documentId, localSnapshot]);

  useEffect(() => {
    setMergeChoices({});
  }, [localSnapshot?.takenAtUtc, load?.document.doc_version]);

  const snapshotMergePlan = useMemo(() => {
    if (!localSnapshot || localSnapshot.forDocumentId !== documentId || !localSnapshot.baseContent || !load) {
      return null;
    }
    return planRichDocumentMerge({
      base: asRichDocumentJson(localSnapshot.baseContent),
      local: asRichDocumentJson(localSnapshot.content),
      remote: asRichDocumentJson(load.document.content_json),
    });
  }, [documentId, load, localSnapshot]);

  const unresolvedMergeConflicts =
    snapshotMergePlan?.blocks.filter((block) => block.status === "conflict" && !mergeChoices[block.blockIndex])
      .length ?? 0;
  const mergeConflictCount =
    snapshotMergePlan?.blocks.filter((block) => block.status === "conflict").length ?? 0;

  const applySnapshotMerge = useCallback(() => {
    if (!snapshotMergePlan || unresolvedMergeConflicts > 0) return;
    const merged = applyRichDocumentMergePlan(snapshotMergePlan, mergeChoices) as JSONContent;
    setLoadedContent(merged);
    editorContentRef.current = merged;
    setCollaborationResetToken((token) => token + 1);
    dirtyGenRef.current += 1;
    isDirtyRef.current = true;
    setIsDirty(true);
    setLocalSnapshot(null);
    setMergeChoices({});
  }, [mergeChoices, snapshotMergePlan, unresolvedMergeConflicts]);

  const versionOptions = useMemo(
    () => [...(versions ?? [])].sort((left, right) => left.doc_version - right.doc_version),
    [versions],
  );

  useEffect(() => {
    setHistoryDiff(null);
    setHistoryDiffError(null);
  }, [documentId, loadedContent]);

  useEffect(() => {
    if (versionOptions.length < 2) {
      setDiffLeftVersion(null);
      setDiffRightVersion(null);
      setHistoryDiff(null);
      return;
    }
    const hasVersion = (docVersion: number | null) =>
      docVersion !== null && versionOptions.some((version) => version.doc_version === docVersion);
    const defaultLeft = versionOptions[versionOptions.length - 2].doc_version;
    const defaultRight = versionOptions[versionOptions.length - 1].doc_version;
    setDiffLeftVersion((current) => (hasVersion(current) ? current : defaultLeft));
    setDiffRightVersion((current) => (hasVersion(current) ? current : defaultRight));
  }, [versionOptions]);

  const canLoadHistoryDiff =
    diffLeftVersion !== null && diffRightVersion !== null && diffLeftVersion !== diffRightVersion;

  const loadSelectedHistoryDiff = useCallback(async () => {
    if (!canLoadHistoryDiff || diffLeftVersion === null || diffRightVersion === null) return;
    setHistoryDiffLoading(true);
    setHistoryDiffError(null);
    try {
      const [left, right] = await Promise.all([
        loadRichDocumentVersion(documentId, diffLeftVersion),
        loadRichDocumentVersion(documentId, diffRightVersion),
      ]);
      setHistoryDiff({
        leftVersion: diffLeftVersion,
        rightVersion: diffRightVersion,
        diff: buildRichDocumentDiff({
          left: asRichDocumentJson(left.version.content_json),
          right: asRichDocumentJson(right.version.content_json),
        }),
      });
    } catch (err) {
      const message = err instanceof Error ? err.message : String(err);
      setHistoryDiff(null);
      setHistoryDiffError(message);
    } finally {
      setHistoryDiffLoading(false);
    }
  }, [canLoadHistoryDiff, diffLeftVersion, diffRightVersion, documentId]);

  const collaborationLease = useCollaborationDocumentLease(
    load?.document.rich_document_id ?? null,
    load?.document.crdt_document_id ?? null,
  );

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
  if (!collaborationLease) {
    return (
      <div className="content-card" data-testid="rich-document-collaboration-loading">
        <h2>Document</h2>
        <p className="muted">Opening editor collaboration state...</p>
      </div>
    );
  }

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
      data-schema-blocked={schemaBlocked !== null ? "true" : "false"}
      data-dirty={isDirty ? "true" : "false"}
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
              disabled={isSaving || !isDirty || schemaBlocked !== null}
              title={
                schemaBlocked !== null
                  ? "Saving is blocked: the document schema is newer than this editor."
                  : undefined
              }
            >
              {isSaving ? "Saving..." : "Save"}
            </button>
            <button
              data-testid="rich-document-reload"
              onClick={() => void reload({ resetCollaboration: true })}
              disabled={isSaving}
            >
              Reload
            </button>
          </div>
        </div>

        <div className="document-editor__body">
          <div className="document-editor__main">
            <RichTextEditor
              key={`${doc.rich_document_id}:${collaborationLease.key}`}
              initialContent={loadedContent}
              onChange={(next) => {
                // H1: track live content WITHOUT echoing it back down.
                // H5: bump the edit generation so an in-flight save cannot
                // clear the dirty flag over these keystrokes.
                editorContentRef.current = next;
                dirtyGenRef.current += 1;
                isDirtyRef.current = true;
                setIsDirty(true);
                if (next) scheduleDraftPersist(next);
              }}
              readOnly={schemaBlocked !== null}
              // Iteration-3 L16: Mod-s / palette save routes through the SAME
              // guarded save path as the button (schema-block included).
              onSaveRequested={() => void onSave()}
              backendError={backendError}
              // MT-244: bind media embed NodeViews to the document's workspace
              // so [[HS_images:…]]/[[video:…]]/album/slideshow resolve REAL
              // backend assets; memoized so the editor is not rebuilt per render.
              embedContext={embedContext}
              documentTitle={doc.title}
              commandPaletteRequest={commandPaletteRequest}
              findRequest={findRequest}
              collaborationDocument={collaborationLease.document}
              collaborationResetToken={collaborationResetToken || undefined}
              // Iteration-3 L19: attributable per-document debug namespace.
              debugId={doc.rich_document_id}
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

            {/* Iteration-3 H5: preserved local edits (conflict / doc switch).
                Both versions stay reachable — local via download/restore here,
                server via Reload. Discard is explicit, never automatic. */}
            {localSnapshot && (
              <div
                className="document-editor__snapshot"
                role="status"
                data-testid="rich-document-local-snapshot"
                data-snapshot-reason={localSnapshot.reason}
                data-snapshot-for={localSnapshot.forDocumentId}
              >
                <strong>
                  {localSnapshot.reason === "draft_recovery" ? "Unsaved draft recovered." : "Local edits preserved."}
                </strong>{" "}
                {localSnapshot.reason === "conflict"
                  ? "Your version was NOT saved because the document changed elsewhere. " +
                    "Download it, or Reload the latest version and restore your copy to re-apply."
                  : localSnapshot.reason === "draft_recovery"
                    ? "Un-saved edits were found in the backend recovery draft. Review the diff, restore them, or discard the draft."
                  : `Un-saved edits from document ${localSnapshot.forDocumentId} were preserved before switching.`}
                <span className="muted"> Taken {localSnapshot.takenAtUtc}.</span>
                <div className="document-editor__snapshot-actions">
                  <button type="button" data-testid="snapshot-download" onClick={downloadSnapshot}>
                    Download local copy
                  </button>
                  {localSnapshot.forDocumentId === documentId && (
                    <button type="button" data-testid="snapshot-restore" onClick={restoreSnapshot}>
                      {localSnapshot.reason === "draft_recovery"
                        ? "Restore draft into editor"
                        : "Restore local version into editor"}
                    </button>
                  )}
                  <button type="button" data-testid="snapshot-discard" onClick={() => void discardSnapshot()}>
                    {localSnapshot.reason === "draft_recovery" ? "Discard recovery draft" : "Discard local snapshot"}
                  </button>
                </div>
                {snapshotMergePlan && (
                  <div
                    className="document-editor__merge"
                    data-testid="rich-document-merge-panel"
                    data-conflicts={mergeConflictCount}
                    data-unresolved-conflicts={unresolvedMergeConflicts}
                  >
                    {snapshotMergePlan.blocks.map((block) => (
                      <div
                        key={block.blockIndex}
                        className="document-editor__merge-row"
                        data-testid="snapshot-merge-block"
                        data-block-index={block.blockIndex}
                        data-merge-status={block.status}
                      >
                        <span className="muted">Block {block.blockIndex + 1}</span>
                        <span data-testid={`snapshot-merge-block-${block.blockIndex}-local-text`}>
                          {block.localText ?? ""}
                        </span>
                        <span data-testid={`snapshot-merge-block-${block.blockIndex}-remote-text`}>
                          {block.remoteText ?? ""}
                        </span>
                        {block.status === "conflict" && (
                          <span className="document-editor__merge-actions">
                            <button
                              type="button"
                              data-testid={`snapshot-merge-block-${block.blockIndex}-local`}
                              data-selected={mergeChoices[block.blockIndex] === "local" ? "true" : "false"}
                              onClick={() =>
                                setMergeChoices((current) => ({ ...current, [block.blockIndex]: "local" }))
                              }
                            >
                              Keep local
                            </button>
                            <button
                              type="button"
                              data-testid={`snapshot-merge-block-${block.blockIndex}-remote`}
                              data-selected={mergeChoices[block.blockIndex] === "remote" ? "true" : "false"}
                              onClick={() =>
                                setMergeChoices((current) => ({ ...current, [block.blockIndex]: "remote" }))
                              }
                            >
                              Use server
                            </button>
                          </span>
                        )}
                      </div>
                    ))}
                    <button
                      type="button"
                      data-testid="snapshot-merge-apply"
                      disabled={unresolvedMergeConflicts > 0}
                      onClick={applySnapshotMerge}
                    >
                      Apply merge
                    </button>
                  </div>
                )}
              </div>
            )}
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
        {versionOptions.length >= 2 && (
          <div className="document-history__diff-controls" data-testid="rich-document-diff-controls">
            <label>
              <span className="muted">From</span>
              <select
                data-testid="rich-document-diff-left-version"
                value={diffLeftVersion ?? ""}
                onChange={(event) => {
                  setDiffLeftVersion(Number(event.target.value));
                  setHistoryDiff(null);
                }}
              >
                {versionOptions.map((version) => (
                  <option key={version.doc_version} value={version.doc_version}>
                    v{version.doc_version}
                  </option>
                ))}
              </select>
            </label>
            <label>
              <span className="muted">To</span>
              <select
                data-testid="rich-document-diff-right-version"
                value={diffRightVersion ?? ""}
                onChange={(event) => {
                  setDiffRightVersion(Number(event.target.value));
                  setHistoryDiff(null);
                }}
              >
                {versionOptions.map((version) => (
                  <option key={version.doc_version} value={version.doc_version}>
                    v{version.doc_version}
                  </option>
                ))}
              </select>
            </label>
            <button
              type="button"
              data-testid="rich-document-diff-load"
              disabled={!canLoadHistoryDiff || historyDiffLoading}
              onClick={() => void loadSelectedHistoryDiff()}
            >
              {historyDiffLoading ? "Loading diff..." : "Load diff"}
            </button>
          </div>
        )}
        {historyDiffError && (
          <div className="error" data-testid="rich-document-diff-error">
            {historyDiffError}
          </div>
        )}
        {historyDiff && (
          <div
            className="document-history__diff"
            data-testid="rich-document-diff-panel"
            data-left-version={historyDiff.leftVersion}
            data-right-version={historyDiff.rightVersion}
          >
            {historyDiff.diff.blocks.map((block) => (
              <div
                key={block.blockIndex}
                className="document-history__diff-row"
                data-testid="rich-document-diff-block"
                data-block-index={block.blockIndex}
                data-diff-status={block.status}
                data-diff-kind={block.kind}
              >
                <div className="document-history__diff-row-header">
                  <span className="muted">Block {block.blockIndex + 1}</span>
                  <span className="muted">{block.status}</span>
                </div>
                <div className="document-history__diff-sides">
                  <pre data-testid={`rich-document-diff-block-${block.blockIndex}-left`}>
                    {diffBlockLeftText(block)}
                  </pre>
                  <pre data-testid={`rich-document-diff-block-${block.blockIndex}-right`}>
                    {diffBlockRightText(block)}
                  </pre>
                </div>
                {block.kind === "code" && (
                  <div
                    className="document-history__code-diff"
                    data-testid={`rich-document-code-diff-${block.blockIndex}`}
                    data-monaco-diff="true"
                    data-language={diffBlockLanguage(block)}
                  >
                    <pre data-testid={`rich-document-code-diff-${block.blockIndex}-left`}>
                      {block.leftCode?.code ?? ""}
                    </pre>
                    <pre data-testid={`rich-document-code-diff-${block.blockIndex}-right`}>
                      {block.rightCode?.code ?? ""}
                    </pre>
                  </div>
                )}
              </div>
            ))}
          </div>
        )}
        {versionOptions.map((v) => (
          <div key={v.doc_version} className="muted" data-testid="rich-document-version" data-doc-version={v.doc_version}>
            v{v.doc_version} - {new Date(v.created_at).toLocaleString()}
          </div>
        ))}
      </div>
    </div>
  );
}
