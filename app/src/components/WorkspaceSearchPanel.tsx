import { useEffect, useMemo, useState } from "react";
import {
  DEFAULT_RICH_DOC_CONTEXT,
  getWorkspaceSearchBookmarkState,
  loadRichDocument,
  saveRichDocument,
  saveWorkspaceSearchBookmarkState,
  searchLoomGraph,
  type JSONContentLike,
  type LoomGraphSearchHit,
  type LoomGraphSearchSourceKind,
} from "../lib/api";
import type { EditorFindOptions } from "../lib/editor/editor_find_request";
import { documentIdFromLoomSearchHit, openTargetForLoomSearchHit } from "../lib/loom_search_open_target";
import { parseLoomSearchOperators } from "../lib/loom_search_query";

const SEARCHABLE_KINDS: Array<{ value: "all" | LoomGraphSearchSourceKind; label: string }> = [
  { value: "all", label: "All kinds" },
  { value: "document", label: "Documents" },
  { value: "loom_block", label: "Loom blocks" },
  { value: "file", label: "Files" },
  { value: "tag_hub", label: "Tags" },
  { value: "symbol", label: "Symbols" },
  { value: "work_packet", label: "Work packets" },
  { value: "micro_task", label: "Microtasks" },
  { value: "user_manual_page", label: "UserManual" },
  { value: "wiki_page", label: "Wiki pages" },
];

const SEARCH_PAGE_SIZE = 500;
// MT-258: saved searches are durable, canonical state. They persist to
// PostgreSQL + EventLedger via the search-bookmarks route; this schema_id must
// match the backend blob contract (hsk.workspace_search_bookmark_state@1).
const WORKSPACE_SEARCH_BOOKMARK_SCHEMA_ID = "hsk.workspace_search_bookmark_state@1";
const MAX_WORKSPACE_SEARCH_BOOKMARKS = 20;

const KIND_LABELS: Record<LoomGraphSearchSourceKind, string> = {
  loom_block: "Loom Block",
  file: "File",
  tag_hub: "Tag Hub",
  document: "Document",
  symbol: "Symbol",
  work_packet: "Work Packet",
  micro_task: "Microtask",
  user_manual_page: "UserManual Page",
  wiki_page: "Wiki Page",
};

type Props = {
  open: boolean;
  workspaceId: string | null;
  onClose: () => void;
  onOpenDocument: (documentId: string, findOptions?: EditorFindOptions) => void;
  onOpenLoomBlock: (blockId: string) => void;
  onOpenCodeSymbol?: (symbolEntityId: string) => void;
  onOpenMicroTask?: (target: { mtId: string; wpId?: string | null }) => void;
  onOpenWorkPacket?: (wpId: string) => void;
  onOpenUserManualPage?: (slug: string) => void;
  onOpenWikiPage?: (projectionId: string) => void;
};

type MatchOptions = {
  caseSensitive: boolean;
  wholeWord: boolean;
  isRegex: boolean;
};

type WorkspaceSearchBookmark = {
  id: string;
  label: string;
  query: string;
  kind: "all" | LoomGraphSearchSourceKind;
  tagFilter: string;
  pathFilter: string;
  caseSensitive: boolean;
  wholeWord: boolean;
  isRegex: boolean;
  savedAt: string;
};

type ReplacementPlan = {
  documentId: string;
  title: string;
  expectedVersion: number;
  contentJson: JSONContentLike;
  crdtDocumentId: string | null;
  matchCount: number;
  beforePreview: string;
  afterPreview: string;
  matchPreviews: ReplacementMatchPreview[];
};

type ReplacementMatchPreview = {
  index: number;
  beforePreview: string;
  afterPreview: string;
};

type SegmentMatchPreview = {
  beforePreview: string;
  afterPreview: string;
};

type SegmentReplaceResult = {
  text: string;
  count: number;
  beforePreview: string | null;
  afterPreview: string | null;
  matchPreviews: SegmentMatchPreview[];
};

const MATCH_PREVIEW_CONTEXT_CHARS = 24;

function stablePart(value: string): string {
  const stable = value
    .trim()
    .replace(/[^a-zA-Z0-9_-]+/g, "-")
    .replace(/^-+|-+$/g, "");
  return stable || "item";
}

function parseWorkspaceSearchBookmarks(bookmarkState: unknown): WorkspaceSearchBookmark[] {
  if (!bookmarkState || typeof bookmarkState !== "object" || Array.isArray(bookmarkState)) return [];
  const raw = (bookmarkState as Record<string, unknown>).bookmarks;
  if (!Array.isArray(raw)) return [];
  return raw.filter(isWorkspaceSearchBookmark).slice(0, MAX_WORKSPACE_SEARCH_BOOKMARKS);
}

function workspaceSearchBookmarkBlob(bookmarks: WorkspaceSearchBookmark[]): Record<string, unknown> {
  return {
    schema_id: WORKSPACE_SEARCH_BOOKMARK_SCHEMA_ID,
    bookmarks: bookmarks.slice(0, MAX_WORKSPACE_SEARCH_BOOKMARKS),
  };
}

function isSearchKind(value: unknown): value is "all" | LoomGraphSearchSourceKind {
  return SEARCHABLE_KINDS.some((item) => item.value === value);
}

function isWorkspaceSearchBookmark(value: unknown): value is WorkspaceSearchBookmark {
  if (!value || typeof value !== "object" || Array.isArray(value)) return false;
  const raw = value as Record<string, unknown>;
  return (
    typeof raw.id === "string" &&
    raw.id.trim().length > 0 &&
    typeof raw.label === "string" &&
    raw.label.trim().length > 0 &&
    typeof raw.query === "string" &&
    isSearchKind(raw.kind) &&
    typeof raw.tagFilter === "string" &&
    typeof raw.pathFilter === "string" &&
    typeof raw.caseSensitive === "boolean" &&
    typeof raw.wholeWord === "boolean" &&
    typeof raw.isRegex === "boolean" &&
    typeof raw.savedAt === "string"
  );
}

async function readWorkspaceSearchBookmarks(workspaceId: string): Promise<WorkspaceSearchBookmark[]> {
  const response = await getWorkspaceSearchBookmarkState(workspaceId);
  return parseWorkspaceSearchBookmarks(response.bookmark_state);
}

async function writeWorkspaceSearchBookmarks(
  workspaceId: string,
  bookmarks: WorkspaceSearchBookmark[],
): Promise<WorkspaceSearchBookmark[]> {
  const response = await saveWorkspaceSearchBookmarkState(
    workspaceId,
    workspaceSearchBookmarkBlob(bookmarks),
  );
  return parseWorkspaceSearchBookmarks(response.bookmark_state);
}

function bookmarkIdForSearch(input: Omit<WorkspaceSearchBookmark, "id" | "label" | "savedAt">): string {
  return stablePart(
    [
      input.query.trim() || "empty",
      input.kind,
      input.tagFilter.trim(),
      input.pathFilter.trim(),
      input.caseSensitive ? "case" : "",
      input.wholeWord ? "word" : "",
      input.isRegex ? "regex" : "",
    ].filter(Boolean).join(" ").toLowerCase(),
  );
}

function bookmarkLabelForSearch(input: Pick<WorkspaceSearchBookmark, "query" | "kind" | "tagFilter" | "pathFilter">): string {
  const queryLabel = input.query.trim();
  if (queryLabel) return queryLabel;
  const kindLabel = input.kind === "all" ? "" : SEARCHABLE_KINDS.find((item) => item.value === input.kind)?.label;
  return [kindLabel, input.tagFilter.trim(), input.pathFilter.trim()].filter(Boolean).join(" / ") || "Filtered search";
}

function targetLabel(hit: LoomGraphSearchHit): string {
  return openTargetForLoomSearchHit(hit).label;
}

function splitFilter(value: string): string[] {
  return value
    .split(",")
    .map((item) => item.trim())
    .filter(Boolean);
}

function escapeRegExp(text: string): string {
  return text.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

function compileSearchRegex(query: string, options: MatchOptions): RegExp | string {
  if (!query.trim()) return "Search query is required";
  try {
    return new RegExp(options.isRegex ? query : escapeRegExp(query), options.caseSensitive ? "gu" : "giu");
  } catch (error) {
    return `Invalid regular expression: ${error instanceof Error ? error.message : String(error)}`;
  }
}

const WORD_CHAR = /[\p{L}\p{N}_]/u;

function isWordBoundary(text: string, start: number, end: number): boolean {
  const before = start > 0 ? text[start - 1] : "";
  const after = end < text.length ? text[end] : "";
  const startsOnWord = text.length > start && WORD_CHAR.test(text[start] ?? "");
  const endsOnWord = end > 0 && WORD_CHAR.test(text[end - 1] ?? "");
  if (startsOnWord && before && WORD_CHAR.test(before)) return false;
  if (endsOnWord && after && WORD_CHAR.test(after)) return false;
  return true;
}

function expandReplacement(template: string, matchText: string, groups: string[]): string {
  return template.replace(/\$(\$|&|[1-9])/g, (_, token: string) => {
    if (token === "$") return "$";
    if (token === "&") return matchText;
    return groups[Number(token) - 1] ?? "";
  });
}

function replacementMatchPreview(text: string, start: number, end: number, inserted: string): SegmentMatchPreview {
  const previewStart = Math.max(0, start - MATCH_PREVIEW_CONTEXT_CHARS);
  const previewEnd = Math.min(text.length, end + MATCH_PREVIEW_CONTEXT_CHARS);
  return {
    beforePreview: text.slice(previewStart, previewEnd),
    afterPreview: `${text.slice(previewStart, start)}${inserted}${text.slice(end, previewEnd)}`,
  };
}

function replaceSegment(text: string, regex: RegExp, replacement: string, options: MatchOptions): SegmentReplaceResult {
  regex.lastIndex = 0;
  let next = "";
  let lastIndex = 0;
  let count = 0;
  let beforePreview: string | null = null;
  let afterPreview: string | null = null;
  const matchPreviews: SegmentMatchPreview[] = [];
  let result: RegExpExecArray | null;

  while ((result = regex.exec(text)) !== null) {
    const start = result.index;
    const end = start + result[0].length;
    if (result[0].length === 0) {
      regex.lastIndex = start + 1;
      if (regex.lastIndex > text.length) break;
      continue;
    }
    if (options.wholeWord && !isWordBoundary(text, start, end)) {
      continue;
    }

    const inserted = options.isRegex
      ? expandReplacement(replacement, result[0], result.slice(1).map((group) => group ?? ""))
      : replacement;
    next += text.slice(lastIndex, start) + inserted;
    lastIndex = end;
    count += 1;
    matchPreviews.push(replacementMatchPreview(text, start, end, inserted));

    if (!beforePreview) {
      beforePreview = text;
    }
  }

  if (count === 0) return { text, count: 0, beforePreview: null, afterPreview: null, matchPreviews: [] };
  next += text.slice(lastIndex);
  afterPreview = next;
  return { text: next, count, beforePreview, afterPreview, matchPreviews };
}

function replaceInContent(
  content: JSONContentLike,
  regex: RegExp,
  replacement: string,
  options: MatchOptions,
): {
  content: JSONContentLike;
  count: number;
  beforePreview: string | null;
  afterPreview: string | null;
  matchPreviews: ReplacementMatchPreview[];
} {
  let count = 0;
  let beforePreview: string | null = null;
  let afterPreview: string | null = null;
  const matchPreviews: ReplacementMatchPreview[] = [];

  const addMatchPreviews = (previews: SegmentMatchPreview[]) => {
    for (const preview of previews) {
      matchPreviews.push({
        index: matchPreviews.length + 1,
        beforePreview: preview.beforePreview,
        afterPreview: preview.afterPreview,
      });
    }
  };

  const visit = (node: JSONContentLike): JSONContentLike => {
    const next: JSONContentLike = { ...node };
    if (typeof next.text === "string") {
      const replaced = replaceSegment(next.text, regex, replacement, options);
      next.text = replaced.text;
      count += replaced.count;
      beforePreview ??= replaced.beforePreview;
      afterPreview ??= replaced.afterPreview;
      addMatchPreviews(replaced.matchPreviews);
    }
    if (next.attrs && typeof next.attrs.code === "string") {
      const replaced = replaceSegment(next.attrs.code, regex, replacement, options);
      next.attrs = { ...next.attrs, code: replaced.text };
      count += replaced.count;
      beforePreview ??= replaced.beforePreview;
      afterPreview ??= replaced.afterPreview;
      addMatchPreviews(replaced.matchPreviews);
    }
    if (Array.isArray(next.content)) {
      next.content = next.content.map(visit);
    }
    return next;
  };

  return { content: visit(content), count, beforePreview, afterPreview, matchPreviews };
}

function hitMatchesClientOptions(hit: LoomGraphSearchHit, query: string, options: MatchOptions): boolean {
  if (!query.trim()) return true;
  if (!options.caseSensitive && !options.wholeWord && !options.isRegex) return true;
  const regex = compileSearchRegex(query, options);
  if (typeof regex === "string") return true;
  const haystack = `${hit.title}\n${hit.excerpt ?? ""}`;
  regex.lastIndex = 0;
  let result: RegExpExecArray | null;
  while ((result = regex.exec(haystack)) !== null) {
    if (result[0].length === 0) {
      regex.lastIndex = result.index + 1;
      if (regex.lastIndex > haystack.length) break;
      continue;
    }
    if (!options.wholeWord || isWordBoundary(haystack, result.index, result.index + result[0].length)) return true;
  }
  return false;
}

export function WorkspaceSearchPanel({
  open,
  workspaceId,
  onClose,
  onOpenDocument,
  onOpenLoomBlock,
  onOpenCodeSymbol,
  onOpenMicroTask,
  onOpenWorkPacket,
  onOpenUserManualPage,
  onOpenWikiPage,
}: Props) {
  const [query, setQuery] = useState("");
  const [kind, setKind] = useState<"all" | LoomGraphSearchSourceKind>("all");
  const [tagFilter, setTagFilter] = useState("");
  const [pathFilter, setPathFilter] = useState("");
  const [replacement, setReplacement] = useState("");
  const [caseSensitive, setCaseSensitive] = useState(false);
  const [wholeWord, setWholeWord] = useState(false);
  const [isRegex, setIsRegex] = useState(false);
  const [results, setResults] = useState<LoomGraphSearchHit[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [replaceStatus, setReplaceStatus] = useState<string | null>(null);
  const [previewPlans, setPreviewPlans] = useState<ReplacementPlan[]>([]);
  const [previewPlanKey, setPreviewPlanKey] = useState<string | null>(null);
  const [resultSetKey, setResultSetKey] = useState<string | null>(null);
  const [bookmarks, setBookmarks] = useState<WorkspaceSearchBookmark[]>([]);
  const [bookmarkStatus, setBookmarkStatus] = useState<string | null>(null);
  const options = useMemo(() => ({ caseSensitive, wholeWord, isRegex }), [caseSensitive, wholeWord, isRegex]);
  const parsedQuery = useMemo(() => parseLoomSearchOperators(query), [query]);
  const effectiveQuery = parsedQuery.q.trim();
  const effectiveSourceKinds = useMemo(() => {
    if (parsedQuery.sourceKinds.length > 0) return parsedQuery.sourceKinds;
    return kind === "all" ? undefined : [kind];
  }, [kind, parsedQuery.sourceKinds]);
  const effectiveTagIds = useMemo(
    () => [...new Set([...splitFilter(tagFilter), ...parsedQuery.tagIds])],
    [tagFilter, parsedQuery.tagIds],
  );
  const effectiveMentionIds = parsedQuery.mentionIds;
  const effectivePath = parsedQuery.path ?? (pathFilter.trim() || undefined);
  const searchPlanKey = useMemo(
    () =>
      JSON.stringify({
        query: effectiveQuery,
        sourceKinds: effectiveSourceKinds ?? [],
        tags: effectiveTagIds,
        mentions: effectiveMentionIds,
        path: effectivePath ?? null,
        options,
      }),
    [effectiveQuery, effectiveSourceKinds, effectiveTagIds, effectiveMentionIds, effectivePath, options],
  );
  const replacePlanKey = useMemo(
    () =>
      JSON.stringify({
        searchPlanKey,
        replacement,
      }),
    [searchPlanKey, replacement],
  );

  const visibleResults = useMemo(
    () => results.filter((hit) => hitMatchesClientOptions(hit, effectiveQuery, options)),
    [results, effectiveQuery, options],
  );

  useEffect(() => {
    setBookmarkStatus(null);
    if (!workspaceId) {
      setBookmarks([]);
      return;
    }
    let cancelled = false;
    readWorkspaceSearchBookmarks(workspaceId)
      .then((loaded) => {
        if (!cancelled) setBookmarks(loaded);
      })
      .catch((err) => {
        if (!cancelled) {
          setBookmarks([]);
          setBookmarkStatus(
            err instanceof Error ? `Saved searches failed to load: ${err.message}` : "Saved searches failed to load.",
          );
        }
      });
    return () => {
      cancelled = true;
    };
  }, [workspaceId]);

  if (!open) return null;

  const clearSearchResultState = () => {
    setResults([]);
    setPreviewPlans([]);
    setPreviewPlanKey(null);
    setResultSetKey(null);
    setReplaceStatus(null);
    setError(null);
  };

  const saveSearchBookmark = async () => {
    if (!workspaceId) {
      setBookmarkStatus("No workspace selected");
      return;
    }
    const bookmarkInput = {
      query,
      kind,
      tagFilter,
      pathFilter,
      caseSensitive,
      wholeWord,
      isRegex,
    };
    if (!query.trim() && kind === "all" && !tagFilter.trim() && !pathFilter.trim()) {
      setBookmarkStatus("Add a query or filter before bookmarking.");
      return;
    }
    const bookmark: WorkspaceSearchBookmark = {
      ...bookmarkInput,
      id: bookmarkIdForSearch(bookmarkInput),
      label: bookmarkLabelForSearch(bookmarkInput),
      savedAt: new Date().toISOString(),
    };
    const nextBookmarks = [bookmark, ...bookmarks.filter((item) => item.id !== bookmark.id)].slice(
      0,
      MAX_WORKSPACE_SEARCH_BOOKMARKS,
    );
    try {
      const persisted = await writeWorkspaceSearchBookmarks(workspaceId, nextBookmarks);
      setBookmarks(persisted);
      setBookmarkStatus(`Saved search bookmark ${bookmark.label}`);
    } catch (err) {
      setBookmarkStatus(
        err instanceof Error ? `Search bookmark could not be saved: ${err.message}` : "Search bookmark could not be saved.",
      );
    }
  };

  const restoreSearchBookmark = (bookmark: WorkspaceSearchBookmark) => {
    setQuery(bookmark.query);
    setKind(bookmark.kind);
    setTagFilter(bookmark.tagFilter);
    setPathFilter(bookmark.pathFilter);
    setCaseSensitive(bookmark.caseSensitive);
    setWholeWord(bookmark.wholeWord);
    setIsRegex(bookmark.isRegex);
    clearSearchResultState();
    setBookmarkStatus(`Restored search bookmark ${bookmark.label}`);
  };

  const removeSearchBookmark = async (bookmark: WorkspaceSearchBookmark) => {
    if (!workspaceId) return;
    const nextBookmarks = bookmarks.filter((item) => item.id !== bookmark.id);
    try {
      const persisted = await writeWorkspaceSearchBookmarks(workspaceId, nextBookmarks);
      setBookmarks(persisted);
      setBookmarkStatus(`Removed search bookmark ${bookmark.label}`);
    } catch (err) {
      setBookmarkStatus(
        err instanceof Error ? `Search bookmark could not be removed: ${err.message}` : "Search bookmark could not be removed.",
      );
    }
  };

  const runSearch = async (preserveReplaceStatus = false) => {
    if (!workspaceId) {
      setError("No workspace selected");
      return;
    }
    const trimmed = effectiveQuery;
    if (parsedQuery.errors.length > 0) {
      setResults([]);
      setPreviewPlans([]);
      setPreviewPlanKey(null);
      setResultSetKey(null);
      setError(parsedQuery.errors.join("; "));
      return;
    }
    if (!trimmed) {
      setError("Search query is required");
      return;
    }
    if (isRegex) {
      const compiled = compileSearchRegex(trimmed, options);
      if (typeof compiled === "string") {
        setResults([]);
        setPreviewPlans([]);
        setPreviewPlanKey(null);
        setResultSetKey(null);
        setError(compiled);
        return;
      }
    }
    setLoading(true);
    setError(null);
    if (!preserveReplaceStatus) setReplaceStatus(null);
    setPreviewPlans([]);
    setPreviewPlanKey(null);
    setResultSetKey(null);
    try {
      const nextHits: LoomGraphSearchHit[] = [];
      for (let offset = 0; ; offset += SEARCH_PAGE_SIZE) {
        const page = await searchLoomGraph(workspaceId, {
          q: trimmed,
          limit: SEARCH_PAGE_SIZE,
          offset,
          sourceKinds: effectiveSourceKinds,
          tagIds: effectiveTagIds,
          mentionIds: effectiveMentionIds.length > 0 ? effectiveMentionIds : undefined,
          caseSensitive,
          wholeWord,
          isRegex,
          path: effectivePath,
        });
        nextHits.push(...page);
        if (page.length < SEARCH_PAGE_SIZE) break;
      }
      setResults(nextHits);
      setResultSetKey(searchPlanKey);
    } catch (err) {
      setResults([]);
      setResultSetKey(null);
      setError(err instanceof Error ? err.message : "Workspace search failed");
    } finally {
      setLoading(false);
    }
  };

  const openHit = (hit: LoomGraphSearchHit) => {
    const target = openTargetForLoomSearchHit(hit);
    if (!target.enabled) return;
    if (target.kind === "document") {
      onOpenDocument(target.documentId, {
        query: effectiveQuery,
        caseSensitive,
        wholeWord,
        isRegex,
      });
      return;
    }
    if (target.kind === "loom_block") {
      onOpenLoomBlock(target.blockId);
    } else if (target.kind === "code_symbol") {
      onOpenCodeSymbol?.(target.symbolEntityId);
    } else if (target.kind === "work_packet") {
      onOpenWorkPacket?.(target.wpId);
    } else if (target.kind === "micro_task") {
      onOpenMicroTask?.(target.target);
    } else if (target.kind === "user_manual") {
      onOpenUserManualPage?.(target.slug);
    } else if (target.kind === "wiki_page") {
      onOpenWikiPage?.(target.projectionId);
    }
  };

  const previewReplace = async () => {
    const compiled = compileSearchRegex(effectiveQuery, options);
    if (typeof compiled === "string") {
      setError(compiled);
      setPreviewPlans([]);
      setPreviewPlanKey(null);
      return;
    }
    if (resultSetKey !== searchPlanKey) {
      setReplaceStatus("Search results are stale; run Search again before previewing replacements.");
      setPreviewPlans([]);
      setPreviewPlanKey(null);
      return;
    }
    const documentIds = [
      ...new Set(results.map(documentIdFromLoomSearchHit).filter((value): value is string => Boolean(value))),
    ];
    if (documentIds.length === 0) {
      setReplaceStatus("No editable rich documents in the backend result set.");
      setPreviewPlans([]);
      setPreviewPlanKey(null);
      return;
    }

    setLoading(true);
    setError(null);
    setReplaceStatus(null);
    try {
      const plans: ReplacementPlan[] = [];
      for (const documentId of documentIds) {
        const loaded = await loadRichDocument(documentId);
        const replaced = replaceInContent(loaded.document.content_json, compiled, replacement, options);
        if (replaced.count === 0) continue;
        plans.push({
          documentId,
          title: loaded.document.title,
          expectedVersion: loaded.document.doc_version,
          contentJson: replaced.content,
          crdtDocumentId: loaded.document.crdt_document_id,
          matchCount: replaced.count,
          beforePreview: replaced.beforePreview ?? "",
          afterPreview: replaced.afterPreview ?? "",
          matchPreviews: replaced.matchPreviews,
        });
      }
      setPreviewPlans(plans);
      setPreviewPlanKey(replacePlanKey);
      setReplaceStatus(plans.length === 0 ? "No replacements matched in editable rich documents." : null);
    } catch (err) {
      setPreviewPlans([]);
      setPreviewPlanKey(null);
      setError(err instanceof Error ? err.message : "Replace preview failed");
    } finally {
      setLoading(false);
    }
  };

  const applyReplace = async () => {
    if (previewPlans.length === 0) return;
    if (previewPlanKey !== replacePlanKey) {
      setReplaceStatus("Preview is stale; run Preview Replace again before applying.");
      return;
    }
    setLoading(true);
    setError(null);
    const receipts: string[] = [];
    try {
      for (const plan of previewPlans) {
        const metadata = plan.crdtDocumentId ? { crdt_document_id: plan.crdtDocumentId } : {};
        const result = await saveRichDocument(
          plan.documentId,
          plan.expectedVersion,
          plan.contentJson,
          DEFAULT_RICH_DOC_CONTEXT,
          metadata,
        );
        receipts.push(result.save_receipt_event_id);
      }
      setReplaceStatus(`Applied ${previewPlans.length} document replacement plan(s); receipts: ${receipts.join(", ")}`);
      setPreviewPlans([]);
      setPreviewPlanKey(null);
      await runSearch(true);
    } catch (err) {
      if (receipts.length > 0) {
        setReplaceStatus(`Applied ${receipts.length} document replacement plan(s) before failure; receipts: ${receipts.join(", ")}`);
        setPreviewPlans([]);
        setPreviewPlanKey(null);
      }
      setError(err instanceof Error ? err.message : "Replace apply failed");
    } finally {
      setLoading(false);
    }
  };

  return (
    <section className="workspace-search-panel" aria-label="Workspace search" data-testid="workspace-search">
      <div className="workspace-search-panel__header">
        <div>
          <p className="drawer-eyebrow">Find in Files</p>
          <h3>Workspace Search</h3>
        </div>
        <button type="button" className="secondary" onClick={onClose} data-testid="workspace-search.close">
          Close
        </button>
      </div>

      <div className="workspace-search-panel__controls">
        <input
          type="search"
          value={query}
          placeholder="Search workspace"
          aria-label="Workspace search query"
          data-testid="workspace-search.query"
          onChange={(event) => setQuery(event.target.value)}
          onKeyDown={(event) => {
            if (event.key === "Enter") {
              event.preventDefault();
              void runSearch();
            }
          }}
        />
        <input
          type="text"
          value={replacement}
          placeholder="Replace with"
          aria-label="Replace with"
          data-testid="workspace-search.replace"
          onChange={(event) => setReplacement(event.target.value)}
        />
        <select
          value={kind}
          aria-label="Kind filter"
          data-testid="workspace-search.kind-filter"
          onChange={(event) => setKind(event.target.value as "all" | LoomGraphSearchSourceKind)}
        >
          {SEARCHABLE_KINDS.map((item) => (
            <option key={item.value} value={item.value}>
              {item.label}
            </option>
          ))}
        </select>
        <input
          type="text"
          value={tagFilter}
          placeholder="tag ids"
          aria-label="Tag filter"
          data-testid="workspace-search.tag-filter"
          onChange={(event) => setTagFilter(event.target.value)}
        />
        <input
          type="text"
          value={pathFilter}
          placeholder="path operator"
          aria-label="Path filter"
          data-testid="workspace-search.path-filter"
          onChange={(event) => setPathFilter(event.target.value)}
        />
      </div>

      <div className="workspace-search-panel__actions">
        <button
          type="button"
          className={caseSensitive ? "tt-button tt-button--active" : "tt-button"}
          aria-pressed={caseSensitive}
          data-testid="workspace-search.toggle-case"
          onClick={() => setCaseSensitive((value) => !value)}
        >
          Aa
        </button>
        <button
          type="button"
          className={wholeWord ? "tt-button tt-button--active" : "tt-button"}
          aria-pressed={wholeWord}
          data-testid="workspace-search.toggle-word"
          onClick={() => setWholeWord((value) => !value)}
        >
          W
        </button>
        <button
          type="button"
          className={isRegex ? "tt-button tt-button--active" : "tt-button"}
          aria-pressed={isRegex}
          data-testid="workspace-search.toggle-regex"
          onClick={() => setIsRegex((value) => !value)}
        >
          .*
        </button>
        <button type="button" onClick={() => void runSearch()} data-testid="workspace-search.search">
          Search
        </button>
        <button
          type="button"
          onClick={() => void previewReplace()}
          disabled={loading || results.length === 0}
          data-testid="workspace-search.preview-replace"
        >
          Preview Replace
        </button>
        <button
          type="button"
          onClick={() => void applyReplace()}
          disabled={loading || previewPlans.length === 0}
          data-testid="workspace-search.apply-replace"
        >
          Apply
        </button>
        <button
          type="button"
          onClick={() => {
            setPreviewPlans([]);
            setPreviewPlanKey(null);
            setReplaceStatus(null);
          }}
          disabled={previewPlans.length === 0}
          data-testid="workspace-search.cancel-replace"
        >
          Cancel
        </button>
      </div>

      <div className="workspace-search-panel__bookmarks" data-testid="workspace-search.bookmarks">
        <div className="workspace-search-panel__bookmarks-header">
          <span>Saved searches</span>
          <button type="button" onClick={() => void saveSearchBookmark()} data-testid="workspace-search.save-bookmark">
            Bookmark Search
          </button>
        </div>
        {bookmarkStatus ? (
          <div className="workspace-search-panel__bookmark-status" role="status" data-testid="workspace-search.bookmark-status">
            {bookmarkStatus}
          </div>
        ) : null}
        {bookmarks.length > 0 ? (
          <ul className="workspace-search-panel__bookmark-list">
            {bookmarks.map((bookmark) => (
              <li
                key={bookmark.id}
                className="workspace-search-panel__bookmark"
                data-testid={`workspace-search.bookmark.${bookmark.id}`}
              >
                <button
                  type="button"
                  className="workspace-search-panel__bookmark-restore"
                  onClick={() => restoreSearchBookmark(bookmark)}
                  data-testid={`workspace-search.bookmark.${bookmark.id}.restore`}
                >
                  {bookmark.label}
                </button>
                <button
                  type="button"
                  className="workspace-search-panel__bookmark-remove"
                  onClick={() => void removeSearchBookmark(bookmark)}
                  aria-label={`Remove saved search ${bookmark.label}`}
                  data-testid={`workspace-search.bookmark.${bookmark.id}.remove`}
                >
                  Remove
                </button>
              </li>
            ))}
          </ul>
        ) : null}
      </div>

      <div className="workspace-search-panel__status" aria-live="polite" data-testid="workspace-search.status">
        {loading ? "Working..." : error ?? `${visibleResults.length} result${visibleResults.length === 1 ? "" : "s"}`}
      </div>
      {replaceStatus ? (
        <div className="workspace-search-panel__status" aria-live="polite" data-testid="workspace-search.replace-status">
          {replaceStatus}
        </div>
      ) : null}

      {previewPlans.length > 0 ? (
        <div className="workspace-search-panel__preview" data-testid="workspace-search.preview-list">
          {previewPlans.map((plan) => (
            <article
              key={plan.documentId}
              className="workspace-search-panel__preview-item"
              data-testid={`workspace-search.preview.${plan.documentId}`}
            >
              <strong>{plan.title}</strong>
              <span>{plan.matchCount} matches</span>
              <code>{plan.beforePreview}</code>
              <code>{plan.afterPreview}</code>
              <div className="workspace-search-panel__preview-matches">
                {plan.matchPreviews.map((match) => (
                  <div
                    key={match.index}
                    className="workspace-search-panel__preview-match"
                    data-testid={`workspace-search.preview.${plan.documentId}.match.${match.index}`}
                  >
                    <span>Match {match.index}</span>
                    <code data-testid={`workspace-search.preview.${plan.documentId}.match.${match.index}.before`}>
                      {match.beforePreview}
                    </code>
                    <code data-testid={`workspace-search.preview.${plan.documentId}.match.${match.index}.after`}>
                      {match.afterPreview}
                    </code>
                  </div>
                ))}
              </div>
            </article>
          ))}
        </div>
      ) : null}

      <ul className="workspace-search-panel__results" role="listbox" aria-label="Workspace search results">
        {visibleResults.map((hit) => (
          <li key={`${hit.source_kind}:${hit.ref_id}`}>
            <button
              type="button"
              onClick={() => openHit(hit)}
              data-testid={`workspace-search.result.${hit.source_kind}.${stablePart(hit.ref_id)}`}
              data-source-kind={hit.source_kind}
              data-ref-id={hit.ref_id}
            >
              <span className="workspace-search-panel__kind">{KIND_LABELS[hit.source_kind]}</span>
              <span className="workspace-search-panel__title">{hit.title}</span>
              <span className="workspace-search-panel__excerpt">{hit.excerpt}</span>
              <span className="workspace-search-panel__target">{targetLabel(hit)}</span>
            </button>
          </li>
        ))}
      </ul>
    </section>
  );
}
