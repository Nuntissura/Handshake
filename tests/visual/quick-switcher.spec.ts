// WP-KERNEL-009 / MT-256 - global QuickSwitcher browser proof.
//
// Starts the real Vite app shell on loopback, fulfills only backend API calls
// needed for the quick-open workflow, blocks non-loopback network, and drives
// the actual Ctrl+P / Ctrl+Shift+P app key paths in Chromium.

import { expect, test, type Route } from "@playwright/test";
import { spawn, type ChildProcessWithoutNullStreams } from "node:child_process";
import { existsSync, readFileSync } from "node:fs";
import { createServer, get as httpGet, type Server } from "node:http";
import { createRequire } from "node:module";
import path from "node:path";

const repoRoot = path.resolve(__dirname, "..", "..");
const appDir = path.join(repoRoot, "app");
const appRequire = createRequire(path.join(appDir, "package.json"));
const vitePkgPath = appRequire.resolve("vite/package.json");
const vitePkg = JSON.parse(readFileSync(vitePkgPath, "utf8")) as { bin?: string | Record<string, string> };
const viteBinRel = typeof vitePkg.bin === "string" ? vitePkg.bin : vitePkg.bin?.vite;
if (!viteBinRel) throw new Error(`vite package.json at ${vitePkgPath} declares no vite bin`);
const viteCli = path.join(path.dirname(vitePkgPath), viteBinRel);
const apiBaseUrl = "http://127.0.0.1:37501";
const standaloneRichDocumentId = "KRD-00000000000000000000000000000001";
const standaloneRichDocumentContent = {
  type: "doc",
  content: [
    {
      type: "paragraph",
      content: [{ type: "text", text: "GraphSearchAlpha standalone document" }],
    },
  ],
};

type GraphHit = {
  result_kind: "loom_block" | "knowledge_entity" | "user_manual_page" | "wiki_page";
  source_kind:
    | "loom_block"
    | "file"
    | "tag_hub"
    | "document"
    | "symbol"
    | "work_packet"
    | "micro_task"
    | "user_manual_page"
    | "wiki_page";
  ref_id: string;
  title: string;
  excerpt: string;
  block?: { document_id?: string };
  score: number;
  metadata: Record<string, string>;
};

type QuickSwitcherRecent = Omit<GraphHit, "block" | "score"> & {
  workspace_id: string;
  hit_key: string;
  selected_count: number;
  selected_at: string;
  event_ledger_event_id: string;
};

const quickSwitcherRecents: QuickSwitcherRecent[] = [];

function jsonResponse(route: Route, body: unknown) {
  return route.fulfill({
    status: 200,
    contentType: "application/json",
    body: JSON.stringify(body),
  });
}

function graphHits(query: string): GraphHit[] {
  if (query.toLowerCase().includes("recent")) {
    return [
      {
        result_kind: "user_manual_page",
        source_kind: "user_manual_page",
        ref_id: "recent-alpha",
        title: "Recent Alpha",
        excerpt: "Backend returned this first.",
        score: 5,
        metadata: { page_slug: "recent-alpha" },
      },
      {
        result_kind: "user_manual_page",
        source_kind: "user_manual_page",
        ref_id: "recent-beta",
        title: "Recent Beta",
        excerpt: "Selected once, then promoted locally.",
        score: 4,
        metadata: { page_slug: "recent-beta" },
      },
    ];
  }

  return [
    {
      result_kind: "loom_block",
      source_kind: "loom_block",
      ref_id: "block-alpha",
      title: "GraphSearchAlpha Loom note",
      excerpt: "GraphSearchAlpha joins notes to code and manuals.",
      score: 4.2,
      metadata: { content_type: "note" },
    },
    {
      result_kind: "loom_block",
      source_kind: "loom_block",
      ref_id: "block-doc-alpha",
      title: "GraphSearchAlpha document-backed note",
      excerpt: "GraphSearchAlpha opens the linked source document.",
      block: { document_id: "doc-alpha" },
      score: 4.1,
      metadata: { content_type: "note" },
    },
    {
      result_kind: "loom_block",
      source_kind: "file",
      ref_id: "file-alpha",
      title: "GraphSearchAlpha source file",
      excerpt: "Open the indexed file block for GraphSearchAlpha.",
      score: 4,
      metadata: { content_type: "file" },
    },
    {
      result_kind: "loom_block",
      source_kind: "tag_hub",
      ref_id: "tag-alpha",
      title: "GraphSearchAlpha tag hub",
      excerpt: "Open the indexed tag hub for GraphSearchAlpha.",
      score: 3.9,
      metadata: { content_type: "tag_hub" },
    },
    {
      result_kind: "knowledge_entity",
      source_kind: "document",
      ref_id: standaloneRichDocumentId,
      title: "GraphSearchAlpha standalone document",
      excerpt: "Open the standalone rich document authority record.",
      score: 3.8,
      metadata: {
        authority_table: "knowledge_rich_documents",
        rich_document_id: standaloneRichDocumentId,
      },
    },
    {
      result_kind: "knowledge_entity",
      source_kind: "symbol",
      ref_id: "KEN-symbol-alpha",
      title: "GraphSearchAlpha",
      excerpt: "rust:src/backend/graph_search.rs#GraphSearchAlpha",
      score: 3.6,
      metadata: {
        authority_table: "knowledge_entities",
        entity_key: "rust:src/backend/graph_search.rs#GraphSearchAlpha",
      },
    },
    {
      result_kind: "knowledge_entity",
      source_kind: "work_packet",
      ref_id: "KEN-wp-app-backend",
      title: "WP-KERNEL-002",
      excerpt: "Work packet hit.",
      score: 3.2,
      metadata: {
        authority_table: "knowledge_entities",
        entity_key: "wp:WP-KERNEL-002-GraphSearchAlpha",
      },
    },
    {
      result_kind: "knowledge_entity",
      source_kind: "micro_task",
      ref_id: "KEN-mt-app-backend",
      title: "MT-DCC-APP",
      excerpt: "Microtask hit.",
      score: 3.1,
      metadata: {
        authority_table: "knowledge_entities",
        entity_key: "mt:MT-DCC-APP-GraphSearchAlpha",
        work_packet: "WP-KERNEL-002-GraphSearchAlpha",
      },
    },
    {
      result_kind: "wiki_page",
      source_kind: "wiki_page",
      ref_id: "KWP-alpha",
      title: "GraphSearchAlpha Wiki Page",
      excerpt: "Open the compiled project wiki projection for GraphSearchAlpha.",
      score: 2.9,
      metadata: { projection_id: "KWP-alpha", page_type: "concept" },
    },
    {
      result_kind: "user_manual_page",
      source_kind: "user_manual_page",
      ref_id: "graph-search-alpha",
      title: "GraphSearchAlpha UserManual page",
      excerpt: "Open the UserManual page for GraphSearchAlpha.",
      score: 2.8,
      metadata: { page_slug: "graph-search-alpha" },
    },
  ];
}

function richDocumentLoadResponse() {
  return {
    document: {
      rich_document_id: standaloneRichDocumentId,
      workspace_id: "w1",
      document_id: null,
      title: "GraphSearchAlpha standalone document",
      schema_version: "rich_document_v1",
      doc_version: 1,
      content_json: standaloneRichDocumentContent,
      content_sha256: "a".repeat(64),
      crdt_document_id: null,
      crdt_snapshot_id: null,
      promotion_receipt_event_id: null,
      projection_refs: [],
      project_ref: null,
      folder_ref: null,
      authority_label: "promoted",
      owner_actor_kind: null,
      owner_actor_id: null,
      created_at: "2026-06-15T00:00:00Z",
      updated_at: "2026-06-15T00:00:00Z",
    },
    tree: {
      schema_version: "rich_document_v1",
      schema_matches: true,
      block_ids: ["rich-block-alpha"],
      blocks: [
        {
          block_id: "rich-block-alpha",
          kind: "paragraph",
          heading_level: null,
          sequence: 0,
          content: {
            raw: standaloneRichDocumentContent.content[0],
            derived: {
              plain_text: "GraphSearchAlpha standalone document",
              word_count: 3,
              preview: "GraphSearchAlpha standalone document",
            },
            display: {},
          },
        },
      ],
    },
    code_nodes: [],
  };
}

async function fulfillApi(route: Route) {
  const url = new URL(route.request().url());
  if (url.pathname === "/workspaces") {
    return jsonResponse(route, [
      {
        id: "w1",
        name: "Workspace 1",
        created_at: "2026-06-15T00:00:00Z",
        updated_at: "2026-06-15T00:00:00Z",
      },
    ]);
  }
  if (url.pathname === "/workspaces/w1/documents") {
    return jsonResponse(route, [
      {
        id: "doc-alpha",
        workspace_id: "w1",
        title: "GraphSearchAlpha Document",
        created_at: "2026-06-15T00:00:00Z",
        updated_at: "2026-06-15T00:00:00Z",
      },
    ]);
  }
  if (url.pathname === "/workspaces/w1/canvases") {
    return jsonResponse(route, []);
  }
  if (url.pathname === "/workspaces/w1/workbench/layout") {
    return jsonResponse(route, {
      workspace_id: "w1",
      layout_state: null,
      updated_at: null,
      event_ledger_event_id: null,
    });
  }
  if (url.pathname === "/api/flight_recorder") {
    return jsonResponse(route, []);
  }
  if (url.pathname === "/logs/tail") {
    return jsonResponse(route, { lines: [] });
  }
  if (url.pathname === "/health") {
    return jsonResponse(route, { status: "ok", db_status: "ok", component: "quick-switcher-fixture" });
  }
  if (url.pathname === "/api/usermanual/access-points") {
    return jsonResponse(route, {
      count: 1,
      access_points: [
        {
          access_point_id: "ap.diagnostics.manual_tab",
          host_surface: "diagnostics",
          entry_kind: "panel",
          target_page_slug: "manual-toc",
          ui_wiring_route: "/usermanual/pages/manual-toc",
          stable_element_id: "hs-usermanual-diagnostics-tab",
          note: "Diagnostics manual tab",
          target_resolves: true,
        },
      ],
    });
  }
  if (url.pathname === "/api/usermanual/pages") {
    return jsonResponse(route, {
      manual_version: "2.0.0",
      route_namespace: "/usermanual",
      count: 4,
      pages: ["manual-toc", "graph-search-alpha", "recent-alpha", "recent-beta"].map((slug) => ({
        slug,
        title: slug.replace(/-/g, " "),
        page_kind: "guide",
        audience: "model",
        manual_version: "2.0.0",
        content_hash: `${slug}-hash`.padEnd(64, "0").slice(0, 64),
        status: "current",
        updated_at: "2026-06-15T00:00:00Z",
      })),
    });
  }
  if (url.pathname.startsWith("/api/usermanual/pages/")) {
    const slug = decodeURIComponent(url.pathname.slice("/api/usermanual/pages/".length));
    return jsonResponse(route, {
      page: {
        page_id: `page-${slug}`,
        slug,
        title: slug.replace(/-/g, " "),
        page_kind: "guide",
        audience: "model",
        manual_version: "2.0.0",
        content_hash: `${slug}-hash`.padEnd(64, "0").slice(0, 64),
        status: "current",
        updated_at: "2026-06-15T00:00:00Z",
      },
      sections: [
        {
          section_id: `section-${slug}`,
          page_id: `page-${slug}`,
          position: 0,
          section_kind: "body",
          title: "Quick open proof",
          body_md: `Opened ${slug} through the QuickSwitcher.`,
          body_json: null,
        },
      ],
      anchors: [],
      bootstrap_receipt_event_id: "evt-quick-switcher",
      bootstrap_identity_used: true,
    });
  }
  if (url.pathname === "/api/usermanual/search") {
    return jsonResponse(route, { query: url.searchParams.get("q") ?? "", count: 0, results: [] });
  }
  if (url.pathname === "/workspaces/w1/loom/graph-search") {
    return jsonResponse(route, graphHits(url.searchParams.get("q") ?? ""));
  }
  if (url.pathname === "/workspaces/w1/loom/quick-switcher/recents") {
    if (route.request().method() === "GET") {
      return jsonResponse(route, quickSwitcherRecents);
    }
    const payload = route.request().postDataJSON() as Omit<QuickSwitcherRecent, "workspace_id" | "hit_key" | "selected_count" | "selected_at" | "event_ledger_event_id">;
    const hitKey = `${payload.source_kind}:${payload.ref_id}`;
    const existingIndex = quickSwitcherRecents.findIndex((recent) => recent.hit_key === hitKey);
    const selectedCount = existingIndex >= 0 ? quickSwitcherRecents[existingIndex].selected_count + 1 : 1;
    if (existingIndex >= 0) quickSwitcherRecents.splice(existingIndex, 1);
    const recent: QuickSwitcherRecent = {
      workspace_id: "w1",
      hit_key: hitKey,
      result_kind: payload.result_kind,
      source_kind: payload.source_kind,
      ref_id: payload.ref_id,
      title: payload.title,
      excerpt: payload.excerpt,
      metadata: payload.metadata,
      selected_count: selectedCount,
      selected_at: "2026-06-15T00:00:00Z",
      event_ledger_event_id: `EVT-${hitKey.replace(/[^a-z0-9]+/gi, "-")}`,
    };
    quickSwitcherRecents.unshift(recent);
    return jsonResponse(route, recent);
  }
  if (url.pathname === `/knowledge/documents/${standaloneRichDocumentId}`) {
    return jsonResponse(route, richDocumentLoadResponse());
  }
  if (url.pathname === `/knowledge/documents/${standaloneRichDocumentId}/history`) {
    return jsonResponse(route, {
      rich_document_id: standaloneRichDocumentId,
      current_version: 1,
      authority_label: "promoted",
      owner_actor_kind: null,
      owner_actor_id: null,
      versions: [
        {
          rich_document_id: standaloneRichDocumentId,
          doc_version: 1,
          schema_version: "rich_document_v1",
          content_sha256: "a".repeat(64),
          promotion_receipt_event_id: null,
          created_at: "2026-06-15T00:00:00Z",
        },
      ],
    });
  }
  if (url.pathname === `/knowledge/documents/${standaloneRichDocumentId}/embeds`) {
    return jsonResponse(route, { rich_document_id: standaloneRichDocumentId, embeds: [] });
  }
  if (url.pathname === `/knowledge/documents/${standaloneRichDocumentId}/embeds/broken`) {
    return jsonResponse(route, {
      rich_document_id: standaloneRichDocumentId,
      broken_embeds: [],
      available_actions: [],
    });
  }
  if (url.pathname === `/knowledge/documents/${standaloneRichDocumentId}/backlinks`) {
    return jsonResponse(route, { source_document_id: standaloneRichDocumentId, backlinks: [] });
  }
  if (url.pathname === "/workspaces/w1/loom/wiki/KWP-alpha") {
    return jsonResponse(route, {
      projection_id: "KWP-alpha",
      workspace_id: "w1",
      title: "GraphSearchAlpha Wiki Page",
      source_block_ids: ["block-alpha"],
      rendered_content: "GraphSearchAlpha wiki rendered content.",
      staleness_hash: "hash-alpha",
      rebuild_status: "fresh",
      page_type: "concept",
      compile_stamp: null,
      page_links: [],
      created_at: "2026-06-15T00:00:00Z",
      updated_at: "2026-06-15T00:00:00Z",
      staleness_verdict: { status: "fresh" },
    });
  }
  if (url.pathname === "/workspaces/w1/loom/blocks/block-alpha") {
    return jsonResponse(route, {
      block_id: "block-alpha",
      workspace_id: "w1",
      content_type: "note",
      document_id: null,
      asset_id: null,
      title: "GraphSearchAlpha standalone Loom note",
      original_filename: null,
      content_hash: "hash-alpha",
      pinned: false,
      favorite: true,
      pin_order: null,
      journal_date: null,
      created_at: "2026-06-15T00:00:00Z",
      updated_at: "2026-06-15T00:00:00Z",
      imported_at: null,
      derived: {
        full_text_index: "GraphSearchAlpha joins notes to code and manuals.",
        backlink_count: 1,
        mention_count: 2,
        tag_count: 3,
        preview_status: "none",
      },
    });
  }
  if (url.pathname === "/workspaces/w1/loom/blocks/file-alpha") {
    return jsonResponse(route, {
      block_id: "file-alpha",
      workspace_id: "w1",
      content_type: "file",
      document_id: null,
      asset_id: null,
      title: "GraphSearchAlpha source file",
      original_filename: "graph-search-alpha.md",
      content_hash: "hash-file-alpha",
      pinned: false,
      favorite: false,
      pin_order: null,
      journal_date: null,
      created_at: "2026-06-15T00:00:00Z",
      updated_at: "2026-06-15T00:00:00Z",
      imported_at: null,
      derived: {
        full_text_index: "GraphSearchAlpha appears in a source file block.",
        backlink_count: 0,
        mention_count: 1,
        tag_count: 0,
        preview_status: "none",
      },
    });
  }
  if (url.pathname === "/workspaces/w1/loom/blocks/tag-alpha") {
    return jsonResponse(route, {
      block_id: "tag-alpha",
      workspace_id: "w1",
      content_type: "tag_hub",
      document_id: null,
      asset_id: null,
      title: "GraphSearchAlpha tag hub",
      original_filename: null,
      content_hash: "hash-tag-alpha",
      pinned: false,
      favorite: false,
      pin_order: null,
      journal_date: null,
      created_at: "2026-06-15T00:00:00Z",
      updated_at: "2026-06-15T00:00:00Z",
      imported_at: null,
      derived: {
        full_text_index: "GraphSearchAlpha appears in a tag hub block.",
        backlink_count: 1,
        mention_count: 0,
        tag_count: 2,
        preview_status: "none",
      },
    });
  }
  if (url.pathname === "/knowledge/code/symbols/KEN-symbol-alpha") {
    return jsonResponse(route, {
      symbol: {
        symbol_entity_id: "KEN-symbol-alpha",
        symbol_key: "rust:src/backend/graph_search.rs#GraphSearchAlpha",
        display_name: "GraphSearchAlpha",
        symbol_kind: "function",
        owning_wp: "WP-KERNEL-009",
        primary_source_id: "src-backend-graph-search-rs",
        lifecycle_state: "active",
        definition: {
          span_id: "span-alpha",
          source_id: "src-backend-graph-search-rs",
          line_start: 12,
          line_end: 34,
          range_start: 400,
          range_end: 900,
          section_path: "GraphSearchAlpha",
        },
        staleness: { status: "fresh" },
      },
      nav_receipt_event_id: "EVT-symbol-get",
      quiet_background_work_receipt_id: "quiet-symbol-get",
    });
  }
  if (url.pathname === "/api/kernel/dcc_projection") {
    return jsonResponse(route, {
      schema_id: "hsk.kernel.dcc_mvp_runtime_surface@1",
      surface_id: "dcc-quick-switcher-fixture",
      folded_stub_id: "WP-1-Dev-Command-Center-MVP-v1",
      panels: [],
      work_items: [
        {
          work_id: "work-app-backend-123",
          wp_id: "WP-KERNEL-002",
          mt_id: "MT-DCC-APP",
          status: "BACKEND_LOADED",
          worktree_id: "wt-app-backend",
          session_ids: [],
          proposal_ids: [],
          evidence_ids: [],
          allowed_action_ids: [],
        },
      ],
      worktrees: [],
      sessions: [],
      proposals: [],
      evidence: [],
      approval_previews: [],
      write_box_queue_rows: [],
      direct_edit_denials: [],
      promotion_previews: [],
      freshness_badges: [],
      stable_element_ids: [],
      catalog_action_refs: [],
      catalog_action_rows: [],
      direct_authority_mutation_allowed: false,
      ungoverned_tool_execution_allowed: false,
      destructive_git_ops_require_same_turn_approval: true,
      flight_recorder_event_types: ["dcc.work.selected"],
      product_authority_refs: ["kernel.write_box.queue"],
      folded_source_refs: [".GOV/task_packets/stubs/WP-1-Dev-Command-Center-MVP-v1.contract.json"],
      spawn_tree_projection: null,
    });
  }
  if (url.pathname === "/documents/doc-alpha") {
    return jsonResponse(route, {
      id: "doc-alpha",
      workspace_id: "w1",
      title: "GraphSearchAlpha Document",
      created_at: "2026-06-15T00:00:00Z",
      updated_at: "2026-06-15T00:00:00Z",
      blocks: [],
    });
  }
  if (url.pathname === `/documents/${standaloneRichDocumentId}`) {
    return jsonResponse(route, {
      id: standaloneRichDocumentId,
      workspace_id: "w1",
      title: "GraphSearchAlpha standalone document",
      created_at: "2026-06-15T00:00:00Z",
      updated_at: "2026-06-15T00:00:00Z",
      blocks: [],
    });
  }
  if (url.pathname === "/atelier/overview") {
    return jsonResponse(route, { tables: [], event_families: [] });
  }
  if (url.pathname === "/atelier/intake/batches") {
    return jsonResponse(route, []);
  }
  if (url.pathname === "/atelier/command-corpus") {
    return jsonResponse(route, []);
  }
  if (url.pathname === "/atelier/stealth/windows") {
    return jsonResponse(route, []);
  }
  return jsonResponse(route, {});
}

function freePort(): Promise<number> {
  const server = createServer();
  return new Promise((resolve, reject) => {
    server.once("error", reject);
    server.listen(0, "127.0.0.1", () => {
      const address = server.address();
      server.close(() => {
        if (address && typeof address === "object") {
          resolve(address.port);
        } else {
          reject(new Error("could not allocate a loopback port"));
        }
      });
    });
  });
}

async function waitForHttp(url: string, proc: ChildProcessWithoutNullStreams, output: string[]) {
  const deadline = Date.now() + 45_000;
  while (Date.now() < deadline) {
    if (proc.exitCode !== null) {
      throw new Error(`vite exited early with ${proc.exitCode}\n${output.join("")}`);
    }
    const ready = await new Promise<boolean>((resolve) => {
      const req = httpGet(url, (res) => {
        res.resume();
        resolve((res.statusCode ?? 500) < 500);
      });
      req.once("error", () => resolve(false));
      req.setTimeout(1_000, () => {
        req.destroy();
        resolve(false);
      });
    });
    if (ready) return;
    await new Promise((resolve) => setTimeout(resolve, 250));
  }
  throw new Error(`vite did not become ready at ${url}\n${output.join("")}`);
}

test.describe("WP-KERNEL-009 MT-256 QuickSwitcher browser proof", () => {
  let viteProcess: ChildProcessWithoutNullStreams;
  let baseUrl: string;
  const serverOutput: string[] = [];

  test.beforeAll(async () => {
    expect(existsSync(viteCli), `vite executable missing at ${viteCli}`).toBe(true);
    const port = await freePort();
    baseUrl = `http://127.0.0.1:${port}`;
    viteProcess = spawn(process.execPath, [viteCli, "--host", "127.0.0.1", "--port", String(port), "--strictPort"], {
      cwd: appDir,
      env: { ...process.env, BROWSER: "none" },
      stdio: ["ignore", "pipe", "pipe"],
    });
    viteProcess.stdout.on("data", (chunk) => serverOutput.push(String(chunk)));
    viteProcess.stderr.on("data", (chunk) => serverOutput.push(String(chunk)));
    await waitForHttp(baseUrl, viteProcess, serverOutput);
  });

  test.afterAll(async () => {
    if (viteProcess && viteProcess.exitCode === null) {
      viteProcess.kill();
    }
  });

  test("opens nine graph source kinds, preserves command mode, and orders recents", async ({ page }, testInfo) => {
    test.setTimeout(60_000);
    const externalRequests: string[] = [];
    const browserDiagnostics: string[] = [];
    quickSwitcherRecents.length = 0;

    page.on("console", (message) => {
      browserDiagnostics.push(`[console:${message.type()}] ${message.text()}`);
    });
    page.on("pageerror", (error) => {
      browserDiagnostics.push(`[pageerror] ${error.message}\n${error.stack ?? ""}`);
    });

    await page.route("**/*", async (route) => {
      const url = route.request().url();
      if (url.startsWith(apiBaseUrl)) {
        await fulfillApi(route);
        return;
      }
      if (url.startsWith(baseUrl)) {
        await route.continue();
        return;
      }
      externalRequests.push(url);
      await route.abort("connectionfailed");
    });

    await page.goto(baseUrl, { waitUntil: "domcontentloaded" });
    await expect(page.getByTestId("main-window"))
      .toBeVisible({ timeout: 10_000 })
      .catch((error) => {
        throw new Error(`${error.message}\n\nBrowser diagnostics:\n${browserDiagnostics.join("\n")}`);
      });

    const openQuickSwitcherWithGraphQuery = async () => {
      await page.keyboard.press("Control+P");
      const search = page.getByTestId("quick-switcher.search");
      await expect(search).toBeFocused();
      await expect(page.getByTestId("quick-switcher.result.loom_block.block-alpha")).toHaveCount(0);
      await search.fill("GraphSearchAlpha");
      await expect(page.getByTestId("quick-switcher.result.loom_block.block-alpha")).toBeVisible();
      return search;
    };

    await openQuickSwitcherWithGraphQuery();

    await expect(page.getByTestId("quick-switcher.result.loom_block.block-alpha")).toContainText("Loom Block");
    await expect(page.getByTestId("quick-switcher.result.loom_block.block-alpha")).toContainText("Open Loom block");
    await expect(page.getByTestId("quick-switcher.result.loom_block.block-doc-alpha")).toContainText(
      "Open source document",
    );
    await expect(page.getByTestId("quick-switcher.result.file.file-alpha")).toContainText("File");
    await expect(page.getByTestId("quick-switcher.result.file.file-alpha")).toContainText("Open file");
    await expect(page.getByTestId("quick-switcher.result.tag_hub.tag-alpha")).toContainText("Tag Hub");
    await expect(page.getByTestId("quick-switcher.result.tag_hub.tag-alpha")).toContainText("Open tag hub");
    const standaloneDocumentResult = page.getByTestId(
      `quick-switcher.result.document.${standaloneRichDocumentId.toLowerCase()}`,
    );
    await expect(standaloneDocumentResult).toContainText("Document");
    await expect(standaloneDocumentResult).toContainText("Open document");
    await expect(page.getByTestId("quick-switcher.result.symbol.ken-symbol-alpha")).toContainText("Symbol");
    await expect(page.getByTestId("quick-switcher.result.symbol.ken-symbol-alpha")).toContainText("Open code symbol");
    await expect(page.getByTestId("quick-switcher.result.work_packet.ken-wp-app-backend")).toContainText(
      "Work Packet",
    );
    await expect(page.getByTestId("quick-switcher.result.work_packet.ken-wp-app-backend")).toContainText(
      "Open Kernel DCC work packet",
    );
    await expect(page.getByTestId("quick-switcher.result.micro_task.ken-mt-app-backend")).toContainText("Microtask");
    await expect(page.getByTestId("quick-switcher.result.micro_task.ken-mt-app-backend")).toContainText(
      "Open Kernel DCC microtask",
    );
    await expect(page.getByTestId("quick-switcher.result.user_manual_page.graph-search-alpha")).toContainText(
      "UserManual Page",
    );
    await expect(page.getByTestId("quick-switcher.result.wiki_page.kwp-alpha")).toContainText("Wiki Page");
    await expect(page.getByTestId("quick-switcher.result.wiki_page.kwp-alpha")).toContainText("Open wiki page");

    const dialogBox = await page.getByTestId("quick-switcher").boundingBox();
    expect(dialogBox?.width).toBeGreaterThan(320);
    expect(dialogBox?.height).toBeGreaterThan(220);
    await page.screenshot({ path: testInfo.outputPath("quick-switcher-nine-kinds.png"), fullPage: true });

    await standaloneDocumentResult.click();
    await expect(page.getByTestId("pane-pane-a")).toHaveAttribute("data-pane-type", "workspace");
    await expect(page.getByTestId("pane-pane-a")).toHaveAttribute(
      "data-pane-active-document-id",
      standaloneRichDocumentId,
    );
    await expect(page.getByTestId("pane-pane-a")).toContainText("GraphSearchAlpha standalone document");

    const richEditorSurface = page.getByTestId("pane-pane-a").locator("[data-testid='rich-text-editor-surface']");
    await richEditorSurface.click();
    await page.keyboard.press("Control+P");
    await expect(page.getByTestId("quick-switcher.search")).toBeFocused();
    await page.keyboard.press("Escape");
    await expect(page.getByTestId("quick-switcher")).toBeHidden();

    await richEditorSurface.click();
    await page.keyboard.press("Control+Shift+P");
    await expect(page.getByRole("dialog", { name: "App commands" })).toBeVisible();
    await expect(page.getByTestId("command-palette-action-hs-usermanual-palette-open")).toBeVisible();
    const boldEditorCommand = page.getByTestId("command-palette-action-hs-editor-command-format-bold");
    await expect(boldEditorCommand).toBeVisible();
    await expect(boldEditorCommand).toBeEnabled();
    await boldEditorCommand.click();
    await expect(page.getByTestId("pane-pane-a")).toHaveAttribute("data-pane-type", "workspace");
    await expect(page.getByTestId("pane-pane-a")).toHaveAttribute(
      "data-pane-active-document-id",
      standaloneRichDocumentId,
    );
    await expect(page.getByTestId("editor-command-palette")).toBeVisible();
    await expect(page.getByTestId("editor-command-palette-input")).toHaveValue("Bold");
    await expect(page.getByTestId("palette-cmd-format.bold")).toBeVisible();
    await page.keyboard.press("Escape");
    await expect(page.getByTestId("editor-command-palette")).toBeHidden();

    await openQuickSwitcherWithGraphQuery();
    await page.getByTestId("quick-switcher.result.file.file-alpha").click();
    await expect(page.getByTestId("pane-pane-a")).toHaveAttribute("data-pane-type", "loom-block");
    await expect(page.getByTestId("loom-block-panel")).toContainText("GraphSearchAlpha source file");

    await openQuickSwitcherWithGraphQuery();
    await page.getByTestId("quick-switcher.result.tag_hub.tag-alpha").click();
    await expect(page.getByTestId("pane-pane-a")).toHaveAttribute("data-pane-type", "loom-block");
    await expect(page.getByTestId("loom-block-panel")).toContainText("GraphSearchAlpha tag hub");

    await openQuickSwitcherWithGraphQuery();
    await page.getByTestId("quick-switcher.result.loom_block.block-doc-alpha").click();
    await expect(page.getByTestId("pane-pane-a")).toHaveAttribute("data-pane-type", "workspace");
    await expect(page.getByTestId("pane-pane-a")).toHaveAttribute("data-pane-active-document-id", "doc-alpha");
    await expect(page.getByTestId("pane-pane-a.document-tab.doc-alpha")).toHaveAttribute("data-active", "true");
    await expect(page.getByTestId("pane-pane-a")).toContainText("GraphSearchAlpha Document");

    await openQuickSwitcherWithGraphQuery();
    await page.getByTestId("quick-switcher.result.symbol.ken-symbol-alpha").click();
    await expect(page.getByTestId("code-symbol-panel")).toContainText("GraphSearchAlpha");
    await expect(page.getByTestId("code-symbol-panel")).toContainText("rust:src/backend/graph_search.rs#GraphSearchAlpha");
    await expect(page.getByTestId("pane-pane-a")).toHaveAttribute("data-pane-type", "code-symbol");

    await openQuickSwitcherWithGraphQuery();
    await expect(page.getByTestId("quick-switcher.search")).toBeFocused();
    await page.getByTestId("quick-switcher.result.work_packet.ken-wp-app-backend").click();
    await expect(page.getByTestId("pane-pane-a")).toHaveAttribute("data-pane-type", "kernel-dcc");
    await expect(page.getByTestId("dcc.work_selection.row.work-app-backend-123")).toHaveAttribute(
      "data-focused",
      "true",
    );

    await openQuickSwitcherWithGraphQuery();
    await page.getByTestId("quick-switcher.result.micro_task.ken-mt-app-backend").click();
    await expect(page.getByTestId("pane-pane-a")).toHaveAttribute("data-pane-type", "kernel-dcc");
    await expect(page.getByTestId("dcc.work_selection.row.work-app-backend-123")).toHaveAttribute(
      "data-focused",
      "true",
    );

    await openQuickSwitcherWithGraphQuery();
    await page.getByTestId("quick-switcher.result.loom_block.block-alpha").click();
    await expect(page.getByTestId("pane-pane-a")).toHaveAttribute("data-pane-type", "loom-block");
    await expect(page.getByTestId("loom-block-panel")).toContainText("GraphSearchAlpha standalone Loom note");
    await expect(page.getByTestId("loom-block-panel")).toContainText(
      "GraphSearchAlpha joins notes to code and manuals.",
    );

    await openQuickSwitcherWithGraphQuery();
    await page.getByTestId("quick-switcher.result.wiki_page.kwp-alpha").click();
    await expect(page.getByTestId("pane-pane-a")).toHaveAttribute("data-pane-type", "loom-wiki-page");
    await expect(page.getByTestId("loom-wiki-page-panel")).toContainText("GraphSearchAlpha Wiki Page");
    await expect(page.getByTestId("loom-wiki-page-panel")).toContainText("GraphSearchAlpha wiki rendered content.");

    await openQuickSwitcherWithGraphQuery();
    await page.getByTestId("quick-switcher.result.user_manual_page.graph-search-alpha").click();
    await expect(page.getByTestId("usermanual-panel")).toBeVisible();
    await expect(page.getByTestId("usermanual-panel")).toHaveAttribute("data-selected-slug", "graph-search-alpha");
    await expect(page.getByTestId("pane-pane-a")).toHaveAttribute("data-pane-type", "user-manual");

    await page.keyboard.press("Control+Shift+P");
    await expect(page.getByRole("dialog", { name: "App commands" })).toBeVisible();
    await expect(page.getByTestId("command-palette-action-hs-usermanual-palette-open")).toBeVisible();
    await page.keyboard.press("Escape");

    await page.keyboard.press("Control+P");
    await page.getByTestId("quick-switcher.search").fill("Recent");
    await expect(page.getByTestId("quick-switcher.result.user_manual_page.recent-alpha")).toHaveAttribute(
      "aria-selected",
      "true",
    );
    await page.keyboard.press("ArrowDown");
    await expect(page.getByTestId("quick-switcher.result.user_manual_page.recent-beta")).toHaveAttribute(
      "aria-selected",
      "true",
    );
    await page.keyboard.press("Enter");
    await expect(page.getByTestId("quick-switcher")).toBeHidden();

    await page.reload();
    await expect(page.getByTestId("main-window")).toBeVisible({ timeout: 10_000 });
    await page.keyboard.press("Control+P");
    await page.getByTestId("quick-switcher.search").fill("Recent");
    await expect(page.getByTestId("quick-switcher.result.user_manual_page.recent-alpha")).toBeVisible();
    await expect(page.getByTestId("quick-switcher.result.user_manual_page.recent-beta")).toBeVisible();
    const refIds = await page
      .getByRole("listbox", { name: "Quick switcher results" })
      .locator("[data-ref-id]")
      .evaluateAll((nodes) => nodes.map((node) => node.getAttribute("data-ref-id")));
    expect(refIds).toEqual(["recent-beta", "recent-alpha"]);

    expect(externalRequests).toEqual([]);
  });
});
