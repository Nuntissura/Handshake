// WP-KERNEL-009 / MT-258 - Loom search-operators browser proof.
//
// Serves the production-built WorkspaceSearchPanel harness and proves that each
// inline search operator (tag:, path:/folder:, kind:, mention:) actually drives
// the RENDERED result set: the backend mock filters its rows by the operator
// the panel encoded into the /loom/graph-search query params, and the test
// asserts the visible result list reflects the filter. Offline, zero external
// network.

import { createReadStream, existsSync, statSync } from "node:fs";
import { createServer, type Server } from "node:http";
import type { AddressInfo } from "node:net";
import path from "node:path";
import type { Route } from "@playwright/test";
import { expect, test } from "./console_error_scan";

const repoRoot = path.resolve(__dirname, "..", "..");
const distHarness = path.join(repoRoot, "app", "dist-harness");
const apiBaseUrl = "http://127.0.0.1:37501";
const workspaceId = "w1";

const CONTENT_TYPES: Record<string, string> = {
  ".html": "text/html; charset=utf-8",
  ".js": "text/javascript; charset=utf-8",
  ".mjs": "text/javascript; charset=utf-8",
  ".css": "text/css; charset=utf-8",
  ".json": "application/json",
  ".svg": "image/svg+xml",
  ".png": "image/png",
  ".ttf": "font/ttf",
  ".woff": "font/woff",
  ".woff2": "font/woff2",
  ".wasm": "application/wasm",
};

const CORS_HEADERS = {
  "access-control-allow-origin": "*",
  "access-control-allow-methods": "GET, OPTIONS",
  "access-control-allow-headers": "content-type",
};

type Row = {
  result_kind: "loom_block" | "knowledge_entity";
  source_kind: string;
  ref_id: string;
  title: string;
  excerpt: string;
  tag_ids: string[];
  mention_ids: string[];
  path: string;
};

// A fixed corpus the backend mock filters by the operator params the panel
// sends — proving the operator (not just free text) shapes the rendered rows.
const CORPUS: Row[] = [
  {
    result_kind: "loom_block",
    source_kind: "loom_block",
    ref_id: "blk-alpha",
    title: "Alpha block",
    excerpt: "alpha body text",
    tag_ids: ["t-alpha"],
    mention_ids: ["m-one"],
    path: "notes/alpha",
  },
  {
    result_kind: "loom_block",
    source_kind: "loom_block",
    ref_id: "blk-beta",
    title: "Beta block",
    excerpt: "beta body text",
    tag_ids: ["t-beta"],
    mention_ids: ["m-two"],
    path: "archive/beta",
  },
  {
    result_kind: "knowledge_entity",
    source_kind: "document",
    ref_id: "doc-gamma",
    title: "Gamma document",
    excerpt: "gamma body text",
    tag_ids: ["t-alpha"],
    mention_ids: ["m-one"],
    path: "notes/gamma",
  },
];

function serveDistHarness(): Promise<Server> {
  const server = createServer((req, res) => {
    const urlPath = decodeURIComponent((req.url ?? "/").split("?")[0]);
    const safePath = path
      .normalize(urlPath)
      .replace(/^([/\\])+/, "")
      .replace(/^(\.\.([/\\]|$))+/, "");
    const filePath = path.join(distHarness, safePath);
    if (!filePath.startsWith(distHarness) || !existsSync(filePath) || !statSync(filePath).isFile()) {
      res.writeHead(404);
      res.end("not found");
      return;
    }
    res.writeHead(200, {
      "content-type": CONTENT_TYPES[path.extname(filePath).toLowerCase()] ?? "application/octet-stream",
    });
    createReadStream(filePath).pipe(res);
  });
  return new Promise((resolve, reject) => {
    server.once("error", reject);
    server.listen(0, "127.0.0.1", () => resolve(server));
  });
}

function jsonResponse(route: Route, body: unknown) {
  return route.fulfill({
    status: 200,
    headers: { ...CORS_HEADERS, "content-type": "application/json" },
    body: JSON.stringify(body),
  });
}

// Filter the corpus by the operator-derived query params the panel sent.
function filterByParams(params: URLSearchParams): Row[] {
  const tagIds = (params.get("tag_ids") ?? "").split(",").map((v) => v.trim()).filter(Boolean);
  const mentionIds = (params.get("mention_ids") ?? "").split(",").map((v) => v.trim()).filter(Boolean);
  const sourceKinds = (params.get("source_kinds") ?? "").split(",").map((v) => v.trim()).filter(Boolean);
  const pathOp = (params.get("path") ?? "").trim();
  return CORPUS.filter((row) => {
    if (tagIds.length > 0 && !tagIds.every((t) => row.tag_ids.includes(t))) return false;
    if (mentionIds.length > 0 && !mentionIds.every((m) => row.mention_ids.includes(m))) return false;
    if (sourceKinds.length > 0 && !sourceKinds.includes(row.source_kind)) return false;
    if (pathOp && !row.path.startsWith(pathOp)) return false;
    return true;
  });
}

function toHit(row: Row) {
  return {
    result_kind: row.result_kind,
    source_kind: row.source_kind,
    ref_id: row.ref_id,
    title: row.title,
    excerpt: row.excerpt,
    block: null,
    score: 1,
    metadata: row.result_kind === "knowledge_entity" ? { rich_document_id: row.ref_id } : {},
  };
}

test.describe("WP-KERNEL-009 MT-258 Loom search operators", () => {
  let server: Server;
  let baseUrl: string;

  test.beforeAll(async () => {
    expect(
      existsSync(path.join(distHarness, "harness", "loom-search-operators.html")),
      "dist-harness missing loom-search-operators.html; run pnpm run build:harness first",
    ).toBe(true);
    server = await serveDistHarness();
    const { port } = server.address() as AddressInfo;
    baseUrl = `http://127.0.0.1:${port}`;
  });

  test.afterAll(async () => {
    await new Promise((resolve) => server.close(resolve));
  });

  test("each operator filters the rendered results", async ({ page }) => {
    const externalRequests: string[] = [];
    const sentParams: string[] = [];

    await page.route("**/*", async (route) => {
      const url = route.request().url();
      if (url.startsWith(baseUrl)) {
        await route.continue();
        return;
      }
      if (url.startsWith(apiBaseUrl)) {
        const parsed = new URL(url);
        const method = route.request().method();
        if (method === "OPTIONS") {
          await route.fulfill({ status: 204, headers: CORS_HEADERS, body: "" });
          return;
        }
        if (parsed.pathname === `/workspaces/${workspaceId}/loom/graph-search` && method === "GET") {
          sentParams.push(parsed.search);
          const offset = Number(parsed.searchParams.get("offset") ?? "0");
          // Single page: only the first page returns rows; later offsets empty.
          const rows = offset === 0 ? filterByParams(parsed.searchParams) : [];
          return jsonResponse(route, rows.map(toHit));
        }
      }
      if (!url.startsWith("about:")) externalRequests.push(url);
      await route.abort("connectionfailed");
    });

    await page.goto(`${baseUrl}/harness/loom-search-operators.html`);
    await expect(page.getByTestId("loom-search-operators-harness-root")).toBeVisible();
    await expect(page.getByTestId("workspace-search")).toBeVisible();

    const queryInput = page.getByTestId("workspace-search.query");
    const results = page.getByRole("listbox", { name: "Workspace search results" });
    const runSearch = async (value: string) => {
      await queryInput.fill(value);
      await page.getByTestId("workspace-search.search").click();
    };

    // --- tag: filters to rows carrying that tag (alpha + gamma, not beta) ----
    await runSearch("body tag:t-alpha");
    await expect(results.getByRole("button")).toHaveCount(2);
    await expect(results).toContainText("Alpha block");
    await expect(results).toContainText("Gamma document");
    await expect(results).not.toContainText("Beta block");
    expect(sentParams.at(-1)).toContain("tag_ids=t-alpha");

    // --- kind: filters to one source kind (document only -> gamma) -----------
    await runSearch("body kind:document");
    await expect(results.getByRole("button")).toHaveCount(1);
    await expect(results).toContainText("Gamma document");
    await expect(results).not.toContainText("Alpha block");
    expect(sentParams.at(-1)).toContain("source_kinds=document");

    // --- path:/folder: filters by path prefix (archive -> beta only) ---------
    await runSearch("body path:archive");
    await expect(results.getByRole("button")).toHaveCount(1);
    await expect(results).toContainText("Beta block");
    expect(sentParams.at(-1)).toContain("path=archive");

    await runSearch("body folder:notes");
    await expect(results.getByRole("button")).toHaveCount(2);
    await expect(results).toContainText("Alpha block");
    await expect(results).toContainText("Gamma document");
    expect(sentParams.at(-1)).toContain("path=notes");

    // --- mention: filters by mention id (m-two -> beta only) -----------------
    await runSearch("body mention:m-two");
    await expect(results.getByRole("button")).toHaveCount(1);
    await expect(results).toContainText("Beta block");
    expect(sentParams.at(-1)).toContain("mention_ids=m-two");

    expect(externalRequests).toEqual([]);
  });
});
