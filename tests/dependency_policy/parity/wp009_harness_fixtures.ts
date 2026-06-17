// WP-KERNEL-009 / MT-263 — loopback API route fixtures for the offline parity
// proofs that drive Loom / draft-recovery harnesses.
//
// These fixtures fulfil the SAME loopback backend routes the standalone Loom
// specs (loom-bookmarks / loom-hover-preview / loom-search-operators /
// loom-transclusion) and the editor-draft-recovery spec fulfil, so the MT-263
// runner can EXECUTE a real per-row runtime proof against the BUILT harness in
// this offline lane — instead of merely citing another spec. Every fixture
// mirrors the route contract of the product harness it serves; the harness UI,
// editor, and rendering are real product code, only the loopback API surface is
// stubbed (the same boundary the owning specs use).
//
// The product harnesses point lib/api BASE_URL at this fixed host.
export const API_BASE_URL = "http://127.0.0.1:37501";

const CORS_HEADERS = {
  "access-control-allow-origin": "*",
  "access-control-allow-methods": "GET, POST, PATCH, PUT, DELETE, OPTIONS",
  "access-control-allow-headers": "content-type",
};

import type { Page, Route } from "@playwright/test";

/** A route fixture installs page.route handlers for one harness. It must call
 *  route.continue() for loopback (harness asset) requests, fulfil the harness's
 *  API routes, and abort + record any non-loopback request so the offline
 *  guarantee is enforced per row. */
export type RouteFixture = (input: {
  page: Page;
  baseUrl: string;
  recordExternal: (url: string) => void;
}) => Promise<void>;

function json(route: Route, body: unknown, status = 200): Promise<void> {
  return route.fulfill({
    status,
    headers: { ...CORS_HEADERS, "content-type": "application/json" },
    body: JSON.stringify(body),
  });
}

/** Shared request router: loopback asset -> continue; API host -> handler;
 *  anything else -> recorded external + aborted (offline guarantee). */
async function routeWith(
  input: { page: Page; baseUrl: string; recordExternal: (url: string) => void },
  handler: (route: Route, url: URL, method: string) => Promise<boolean>,
): Promise<void> {
  await input.page.route("**/*", async (route) => {
    const url = route.request().url();
    if (url.startsWith(input.baseUrl)) {
      await route.continue();
      return;
    }
    if (url.startsWith(API_BASE_URL)) {
      const parsed = new URL(url);
      const method = route.request().method();
      if (method === "OPTIONS") {
        await route.fulfill({ status: 204, headers: CORS_HEADERS, body: "" });
        return;
      }
      const handled = await handler(route, parsed, method);
      if (handled) return;
      // Unhandled loopback API probe (e.g. an optional /search-bookmarks call):
      // it never leaves localhost, so it is NOT an offline violation. Abort it
      // without recording it as external (mirrors the owning specs, which only
      // record genuinely non-loopback requests).
      await route.abort("connectionfailed");
      return;
    }
    if (!url.startsWith("about:")) input.recordExternal(url);
    await route.abort("connectionfailed");
  });
}

// ---------------------------------------------------------------------------
// loom-bookmarks fixture (mirrors tests/visual/loom-bookmarks.spec.ts routes).
// ---------------------------------------------------------------------------
const BOOKMARKS_WS = "w1";

interface LoomBlockFixture {
  block_id: string;
  workspace_id: string;
  content_type: string;
  document_id: string | null;
  asset_id: string | null;
  title: string | null;
  original_filename: string | null;
  content_hash: string | null;
  pinned: boolean;
  favorite: boolean;
  pin_order: number | null;
  journal_date: string | null;
  created_at: string;
  updated_at: string;
  imported_at: string | null;
  derived: {
    full_text_index: string;
    backlink_count: number;
    mention_count: number;
    tag_count: number;
    preview_status: string;
  };
}

function makeBlock(blockId: string, title: string, pinned: boolean, pinOrder: number | null): LoomBlockFixture {
  return {
    block_id: blockId,
    workspace_id: BOOKMARKS_WS,
    content_type: "note",
    document_id: null,
    asset_id: null,
    title,
    original_filename: null,
    content_hash: null,
    pinned,
    favorite: false,
    pin_order: pinOrder,
    journal_date: null,
    created_at: "2026-06-16T00:00:00Z",
    updated_at: "2026-06-16T00:00:00Z",
    imported_at: null,
    derived: {
      full_text_index: `${title} indexed body text.`,
      backlink_count: 1,
      mention_count: 2,
      tag_count: 3,
      preview_status: "ready",
    },
  };
}

export const bookmarksFixture: RouteFixture = async (input) => {
  const blocks = new Map<string, LoomBlockFixture>([
    ["block-alpha", makeBlock("block-alpha", "Pinned Alpha", true, 10)],
    ["block-unpinned", makeBlock("block-unpinned", "Unpinned Source", false, null)],
  ]);
  await routeWith(input, async (route, parsed, method) => {
    if (parsed.pathname === "/workspaces" && method === "GET") {
      await json(route, [
        { id: BOOKMARKS_WS, name: "MT-258 Workspace", created_at: "2026-06-16T00:00:00Z", updated_at: "2026-06-16T00:00:00Z" },
      ]);
      return true;
    }
    if (parsed.pathname === `/workspaces/${BOOKMARKS_WS}/documents` && method === "GET") {
      await json(route, []);
      return true;
    }
    if (parsed.pathname === `/workspaces/${BOOKMARKS_WS}/canvases` && method === "GET") {
      await json(route, []);
      return true;
    }
    if (parsed.pathname === `/workspaces/${BOOKMARKS_WS}/loom/views/pins` && method === "GET") {
      const pinned = [...blocks.values()]
        .filter((b) => b.pinned)
        .sort((l, r) => (l.pin_order ?? 999_999) - (r.pin_order ?? 999_999));
      await json(route, { view_type: "pins", blocks: pinned });
      return true;
    }
    const blockMatch = parsed.pathname.match(/^\/workspaces\/w1\/loom\/blocks\/([^/]+)(?:\/pin-order)?$/);
    if (blockMatch) {
      const id = decodeURIComponent(blockMatch[1]);
      const current = blocks.get(id);
      if (!current) {
        await route.fulfill({ status: 404, headers: CORS_HEADERS, body: "not found" });
        return true;
      }
      if (!parsed.pathname.endsWith("/pin-order") && method === "GET") {
        await json(route, current);
        return true;
      }
      if (!parsed.pathname.endsWith("/pin-order") && method === "PATCH") {
        const body = route.request().postDataJSON() as Partial<LoomBlockFixture>;
        blocks.set(id, { ...current, ...body, updated_at: "2026-06-16T00:10:00Z" });
        await json(route, blocks.get(id));
        return true;
      }
      if (parsed.pathname.endsWith("/pin-order") && method === "PUT") {
        const body = route.request().postDataJSON() as { pin_order: number | null };
        blocks.set(id, { ...current, pin_order: body.pin_order, updated_at: "2026-06-16T00:11:00Z" });
        await json(route, blocks.get(id));
        return true;
      }
    }
    return false;
  });
};

// ---------------------------------------------------------------------------
// loom-hover-preview fixture (mirrors tests/visual/loom-hover-preview.spec.ts).
// ---------------------------------------------------------------------------
const PREVIEW_WS = "ws-mt258-preview";
const PREVIEW_BLOCK = "block-alpha";

export const hoverPreviewFixture: RouteFixture = async (input) => {
  await routeWith(input, async (route, parsed, method) => {
    if (parsed.pathname === `/workspaces/${PREVIEW_WS}/loom/blocks/${PREVIEW_BLOCK}` && method === "GET") {
      await json(route, {
        block_id: PREVIEW_BLOCK,
        workspace_id: PREVIEW_WS,
        content_type: "note",
        document_id: null,
        asset_id: null,
        title: "Alpha Loom note",
        original_filename: null,
        content_hash: null,
        pinned: false,
        favorite: false,
        pin_order: null,
        journal_date: null,
        created_at: "2026-06-16T00:00:00Z",
        updated_at: "2026-06-16T00:00:00Z",
        imported_at: null,
        derived: {
          full_text_index: "Alpha hover preview text from the indexed Loom block body.",
          backlink_count: 2,
          mention_count: 3,
          tag_count: 4,
          preview_status: "ready",
        },
      });
      return true;
    }
    return false;
  });
};

// ---------------------------------------------------------------------------
// loom-search-operators fixture (mirrors tests/visual/loom-search-operators).
// ---------------------------------------------------------------------------
const SEARCH_WS = "w1";

interface SearchRow {
  result_kind: string;
  source_kind: string;
  ref_id: string;
  title: string;
  excerpt: string;
  path: string;
  tag_ids: string[];
  mention_ids: string[];
}

const SEARCH_CORPUS: SearchRow[] = [
  {
    result_kind: "loom_block",
    source_kind: "loom_block",
    ref_id: "blk-alpha",
    title: "Alpha block",
    excerpt: "body alpha",
    path: "notes/alpha",
    tag_ids: ["t-alpha"],
    mention_ids: ["m-1"],
  },
  {
    result_kind: "loom_block",
    source_kind: "loom_block",
    ref_id: "blk-beta",
    title: "Beta block",
    excerpt: "body beta",
    path: "archive/beta",
    tag_ids: ["t-beta"],
    mention_ids: ["m-2"],
  },
  {
    result_kind: "knowledge_entity",
    source_kind: "document",
    ref_id: "doc-gamma",
    title: "Gamma document",
    excerpt: "body gamma",
    path: "notes/gamma",
    tag_ids: ["t-alpha"],
    mention_ids: ["m-1"],
  },
];

function filterByParams(params: URLSearchParams): SearchRow[] {
  const tagIds = (params.get("tag_ids") ?? "").split(",").map((v) => v.trim()).filter(Boolean);
  const mentionIds = (params.get("mention_ids") ?? "").split(",").map((v) => v.trim()).filter(Boolean);
  const sourceKinds = (params.get("source_kinds") ?? "").split(",").map((v) => v.trim()).filter(Boolean);
  const pathOp = (params.get("path") ?? "").trim();
  return SEARCH_CORPUS.filter((row) => {
    if (tagIds.length > 0 && !tagIds.every((t) => row.tag_ids.includes(t))) return false;
    if (mentionIds.length > 0 && !mentionIds.every((m) => row.mention_ids.includes(m))) return false;
    if (sourceKinds.length > 0 && !sourceKinds.includes(row.source_kind)) return false;
    if (pathOp && !row.path.startsWith(pathOp)) return false;
    return true;
  });
}

export const searchOperatorsFixture: RouteFixture = async (input) => {
  await routeWith(input, async (route, parsed, method) => {
    if (parsed.pathname === `/workspaces/${SEARCH_WS}/loom/graph-search` && method === "GET") {
      const offset = Number(parsed.searchParams.get("offset") ?? "0");
      const rows = offset === 0 ? filterByParams(parsed.searchParams) : [];
      await json(
        route,
        rows.map((row) => ({
          result_kind: row.result_kind,
          source_kind: row.source_kind,
          ref_id: row.ref_id,
          title: row.title,
          excerpt: row.excerpt,
          block: null,
          score: 1,
          metadata: row.result_kind === "knowledge_entity" ? { rich_document_id: row.ref_id } : {},
        })),
      );
      return true;
    }
    return false;
  });
};

// ---------------------------------------------------------------------------
// loom-transclusion fixture (mirrors tests/visual/loom-transclusion.spec.ts).
// ---------------------------------------------------------------------------
const TRANS_WS = "ws-mt258-transclusion";
const TRANS_BLOCK = "block-source";
const TRANS_SOURCE_DOC = "KRD-source-001";

function transclusionDoc(body: string) {
  return { type: "doc", content: [{ type: "paragraph", content: [{ type: "text", text: body }] }] };
}

export const transclusionFixture: RouteFixture = async (input) => {
  const sourceState = { version: 1, body: "ORIGINAL source body" };
  const transclusionPath = `/workspaces/${TRANS_WS}/loom/blocks/${TRANS_BLOCK}/transclusion`;
  const savePath = `/knowledge/documents/${TRANS_SOURCE_DOC}/save`;
  await routeWith(input, async (route, parsed, method) => {
    if (parsed.pathname === transclusionPath && method === "GET") {
      await json(route, {
        block_id: TRANS_BLOCK,
        workspace_id: TRANS_WS,
        source_document_id: TRANS_SOURCE_DOC,
        source_doc_version: sourceState.version,
        content_json: transclusionDoc(sourceState.body),
        resolved: true,
      });
      return true;
    }
    if (parsed.pathname === savePath && method === "PUT") {
      const body = route.request().postDataJSON() as { content_json: unknown };
      sourceState.version += 1;
      sourceState.body = "EDITED via transclusion source";
      await json(route, {
        document: { rich_document_id: TRANS_SOURCE_DOC, doc_version: sourceState.version, content_json: body.content_json },
        save_receipt_event_id: "EVT-MT258-SAVE",
        receipt_error: null,
      });
      return true;
    }
    return false;
  });
};

// ---------------------------------------------------------------------------
// editor-draft-recovery fixture (mirrors tests/dependency_policy/
// editor_draft_recovery.spec.ts) — seeds an EXISTING backend draft so the
// recovery banner surfaces on first load (the capability under proof).
// ---------------------------------------------------------------------------
const DRAFT_DOC = "KRD-00000000000000000000000000000255";
const DRAFT_WS = "ws-mt255";

function draftDocResponse(text: string, version: number, hash: string) {
  const content = { type: "doc", content: [{ type: "paragraph", content: [{ type: "text", text }] }] };
  return {
    document: {
      rich_document_id: DRAFT_DOC,
      workspace_id: DRAFT_WS,
      document_id: null,
      title: "MT-255 Draft Recovery",
      schema_version: "rich_document_v1",
      doc_version: version,
      content_json: content,
      content_sha256: hash,
      crdt_document_id: null,
      crdt_snapshot_id: null,
      promotion_receipt_event_id: null,
      projection_refs: [],
      project_ref: null,
      folder_ref: null,
      authority_label: "promoted",
      owner_actor_kind: "operator",
      owner_actor_id: "operator",
      created_at: "2026-06-12T00:00:00Z",
      updated_at: "2026-06-12T00:00:00Z",
    },
    tree: {
      schema_version: "rich_document_v1",
      schema_matches: true,
      block_ids: ["KBL-MT255-1"],
      blocks: [
        {
          block_id: "KBL-MT255-1",
          kind: "paragraph",
          heading_level: null,
          sequence: 0,
          content: {
            raw: content.content[0],
            derived: { plain_text: text, word_count: text.split(/\s+/).filter(Boolean).length, preview: text },
            display: {},
          },
        },
      ],
    },
    code_nodes: [],
  };
}

export const draftRecoveryFixture: RouteFixture = async (input) => {
  const savedText = "saved head";
  const savedVersion = 1;
  const savedHash = "0".repeat(64);
  // An EXISTING crash draft that diverges from the saved head — this is what
  // triggers the recovery banner the proof asserts.
  const draftText = "MT-255 byte-exact crash draft";
  await routeWith(input, async (route, parsed, method) => {
    if (method === "GET" && parsed.pathname === `/knowledge/documents/${DRAFT_DOC}`) {
      await json(route, draftDocResponse(savedText, savedVersion, savedHash));
      return true;
    }
    if (method === "GET" && parsed.pathname === `/knowledge/documents/${DRAFT_DOC}/draft`) {
      await json(route, {
        rich_document_id: DRAFT_DOC,
        current_doc_version: savedVersion,
        current_content_sha256: savedHash,
        draft: {
          rich_document_id: DRAFT_DOC,
          workspace_id: DRAFT_WS,
          base_doc_version: savedVersion,
          base_content_sha256: savedHash,
          draft_content_json: { type: "doc", content: [{ type: "paragraph", content: [{ type: "text", text: draftText }] }] },
          draft_content_sha256: "d".repeat(64),
          actor_kind: "operator",
          actor_id: "operator",
          kernel_task_run_id: "KTR-EDITOR-UI",
          session_run_id: "SR-EDITOR-UI",
          created_at: "2026-06-12T00:00:00Z",
          updated_at: "2026-06-12T00:01:00Z",
        },
      });
      return true;
    }
    if (method === "GET" && parsed.pathname === `/knowledge/documents/${DRAFT_DOC}/history`) {
      await json(route, {
        rich_document_id: DRAFT_DOC,
        current_version: savedVersion,
        authority_label: "promoted",
        owner_actor_kind: "operator",
        owner_actor_id: "operator",
        versions: [
          {
            rich_document_id: DRAFT_DOC,
            doc_version: savedVersion,
            schema_version: "rich_document_v1",
            content_sha256: savedHash,
            crdt_snapshot_id: null,
            promotion_receipt_event_id: null,
            created_at: "2026-06-12T00:00:00Z",
          },
        ],
      });
      return true;
    }
    if (method === "GET" && parsed.pathname === `/knowledge/documents/${DRAFT_DOC}/embeds`) {
      await json(route, { rich_document_id: DRAFT_DOC, embeds: [] });
      return true;
    }
    if (method === "GET" && parsed.pathname === `/knowledge/documents/${DRAFT_DOC}/embeds/broken`) {
      await json(route, { rich_document_id: DRAFT_DOC, broken_embeds: [], available_actions: [] });
      return true;
    }
    if (method === "GET" && parsed.pathname === `/knowledge/documents/${DRAFT_DOC}/backlinks`) {
      await json(route, { source_document_id: DRAFT_DOC, backlinks: [] });
      return true;
    }
    return false;
  });
};
