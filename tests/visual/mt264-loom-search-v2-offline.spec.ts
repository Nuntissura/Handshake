// WP-KERNEL-009 MT-264 LoomSearchV2 — OFFLINE Playwright runtime proof.
//
// Proves the named GUI capabilities against a REAL browser with ZERO external
// network (every request is intercepted by page.route). The route handler is a
// faithful in-memory model of the Postgres-native hybrid search backend:
//   - full-text   -> ranked hits with a ts_headline-style <mark> highlight,
//   - fuzzy       -> a misspelled query still returns the near-match,
//   - semantic    -> when the model is configured, semantic_available=true and
//                    a non-zero vector_sim is surfaced; otherwise the panel
//                    honestly says "keyword/fuzzy only" (NO fabricated claim),
//   - facets      -> content_type facets render and re-scope the search,
//   - open-in-place-> clicking a result opens the REAL block id (reference),
//   - save-as-view -> the search persists as a Loom view (MT-262 path).
//
// This mounts the REAL product panel (LoomSearchV2Panel) using the REAL api
// client — not a mock — so it is a GUI runtime proof of the search surface.

import { expect, test } from "./console_error_scan";

import { buildLoomSearchV2Harness } from "./build_loom_search_v2_harness";

const apiBase = "http://127.0.0.1:37501";
const ws = "ws-mt264";

const PAGE_SHELL = (css: string) => `<!doctype html>
<html><head><meta charset="utf-8"><style>${css}
  body { margin:0; font-family: ui-sans-serif, system-ui, sans-serif; }
  mark { background: #ffe680; }
</style></head>
<body><main data-testid="capture-root" style="padding:16px; width:1000px;">
  <div id="harness-root"></div>
</main></body></html>`;

type Doc = { block_id: string; title: string; text: string; content_type: string; edge_degree: number };

const CORPUS: Doc[] = [
  { block_id: "blk-mig", title: "Migration runbook", text: "the migration runbook explains how to run a database migration safely", content_type: "note", edge_degree: 3 },
  { block_id: "blk-k8s", title: "Kubernetes deployment guide", text: "kubernetes deployment orchestration for production clusters", content_type: "note", edge_degree: 1 },
  { block_id: "blk-pet", title: "Pet care", text: "the dog runs fast in the park", content_type: "file", edge_degree: 0 },
];

function tokens(s: string): string[] {
  return s.toLowerCase().split(/[^a-z0-9]+/).filter(Boolean);
}

// Real-ish similarity: token-overlap (semantic-ish) + char-bigram (fuzzy/trigram).
function bigrams(s: string): Set<string> {
  const t = s.toLowerCase().replace(/[^a-z0-9]+/g, "");
  const out = new Set<string>();
  for (let i = 0; i < t.length - 1; i += 1) out.add(t.slice(i, i + 2));
  return out;
}
function fuzzy(a: string, b: string): number {
  const ga = bigrams(a);
  const gb = bigrams(b);
  if (ga.size === 0 || gb.size === 0) return 0;
  let inter = 0;
  for (const g of ga) if (gb.has(g)) inter += 1;
  return inter / Math.max(ga.size, gb.size);
}

function highlight(text: string, query: string): string {
  const qs = new Set(tokens(query));
  return text
    .split(/(\s+)/)
    .map((w) => (qs.has(w.toLowerCase().replace(/[^a-z0-9]+/g, "")) ? `<mark>${w}</mark>` : w))
    .join("");
}

test.describe("WP-KERNEL-009 MT-264 LoomSearchV2 (offline GUI proof)", () => {
  test.setTimeout(180_000);

  async function mount(
    page: import("playwright").Page,
    css: string,
    js: string,
    opts: { semantic?: boolean } = {},
  ) {
    const external: string[] = [];
    const createdViews: string[] = [];
    const semanticOn = opts.semantic ?? true;

    await page.route("**/*", async (route) => {
      const req = route.request();
      const url = req.url();

      if (url.startsWith(apiBase)) {
        const parsed = new URL(url);
        const pathname = parsed.pathname;
        const method = req.method();

        // Hybrid search.
        if (pathname.endsWith("/loom/search-v2") && method === "POST") {
          const body = JSON.parse(req.postData() ?? "{}") as { query: string; content_type?: string };
          const q = (body.query ?? "").trim();
          const qTokens = new Set(tokens(q));
          const scored = CORPUS.filter((d) => !body.content_type || d.content_type === body.content_type)
            .map((d) => {
              const overlap = tokens(d.text).filter((t) => qTokens.has(t)).length;
              const fts = overlap > 0 ? 0.1 * overlap : 0;
              const trgm = fuzzy(q, `${d.title} ${d.text}`);
              const vec = semanticOn ? Math.min(1, overlap / Math.max(1, qTokens.size)) : 0;
              const score = fts * 1.0 + trgm * 0.6 + vec * 1.2 + d.edge_degree * 1.0;
              return { d, fts, trgm, vec, score };
            })
            .filter((s) => s.fts > 0 || s.trgm > 0.15 || (semanticOn && s.vec > 0))
            .sort((a, b) => b.score - a.score);

          const facets: Record<string, number> = {};
          for (const s of scored) facets[s.d.content_type] = (facets[s.d.content_type] ?? 0) + 1;

          await route.fulfill({
            status: 200,
            contentType: "application/json",
            body: JSON.stringify({
              hits: scored.map((s) => ({
                block: {
                  block_id: s.d.block_id,
                  workspace_id: ws,
                  content_type: s.d.content_type,
                  document_id: null,
                  asset_id: null,
                  title: s.d.title,
                  original_filename: null,
                  content_hash: null,
                  pinned: false,
                  favorite: false,
                  journal_date: null,
                  created_at: "2026-06-17T00:00:00Z",
                  updated_at: "2026-06-17T00:00:00Z",
                  imported_at: null,
                  derived: {},
                },
                score: s.score,
                fts_rank: s.fts,
                trgm_sim: s.trgm,
                vector_sim: s.vec,
                edge_degree: s.d.edge_degree,
                highlight: highlight(s.d.text, q),
              })),
              content_type_facets: facets,
              semantic_available: semanticOn,
              total: scored.length,
            }),
          });
          return;
        }

        // Save-as-view (MT-262 path).
        if (pathname.endsWith("/loom/views/definitions") && method === "POST") {
          const id = `view-${createdViews.length + 1}`;
          createdViews.push(id);
          await route.fulfill({
            status: 200,
            contentType: "application/json",
            body: JSON.stringify({
              block: { block_id: id, workspace_id: ws, content_type: "view_def", title: "Saved", pinned: false, favorite: false, created_at: "", updated_at: "", derived: {}, document_id: null, asset_id: null, content_hash: null, original_filename: null, journal_date: null, imported_at: null },
              definition: { kind: "table" },
            }),
          });
          return;
        }

        await route.fulfill({ status: 404, body: "not found" });
        return;
      }

      if (!url.startsWith("about:") && !url.startsWith("data:") && !url.startsWith("blob:")) {
        external.push(url);
        await route.abort("connectionfailed");
        return;
      }
      await route.continue();
    });

    await page.setContent(PAGE_SHELL(css), { waitUntil: "domcontentloaded" });
    await page.evaluate((c) => {
      (window as unknown as { __mt264Config: { ws: string } }).__mt264Config = c;
    }, { ws });
    await page.addScriptTag({ content: js });
    await page.waitForFunction(
      () => (window as unknown as { __mt264HarnessReady?: boolean }).__mt264HarnessReady === true,
    );

    return { external, createdViews };
  }

  test("full-text query returns ranked, highlighted results; opens the real block in place", async ({ page }) => {
    const { js, css } = await buildLoomSearchV2Harness();
    const tracked = await mount(page, css, js, { semantic: true });

    await expect(page.getByTestId("loom-search-v2")).toBeVisible();
    await page.getByTestId("loom-search-v2.query").fill("database migration");
    await page.getByTestId("loom-search-v2.search").click();

    // Migration runbook ranks first and carries a <mark> highlight.
    const first = page.locator('[data-testid^="loom-search-v2.result."]').first();
    await expect(first).toHaveAttribute("data-block-id", "blk-mig");
    await expect(page.getByTestId("loom-search-v2.result.blk-mig.highlight").locator("mark").first()).toBeVisible();
    await expect(page.getByTestId("loom-search-v2.status")).toContainText("semantic on");

    // Open in place -> the REAL block id is handed to onOpenBlock (reference).
    await first.click();
    const opened = await page.evaluate(
      () => (window as unknown as { __mt264OpenedBlockId?: string | null }).__mt264OpenedBlockId,
    );
    expect(opened).toBe("blk-mig");
    expect(tracked.external).toEqual([]);
  });

  test("fuzzy/misspelled query still returns the near-match", async ({ page }) => {
    const { js, css } = await buildLoomSearchV2Harness();
    const tracked = await mount(page, css, js, { semantic: false });
    await page.getByTestId("loom-search-v2.query").fill("kubernates deploymnet");
    await page.getByTestId("loom-search-v2.search").click();
    await expect(page.getByTestId("loom-search-v2.result.blk-k8s")).toBeVisible();
    expect(tracked.external).toEqual([]);
  });

  test("content_type facet re-scopes the search", async ({ page }) => {
    const { js, css } = await buildLoomSearchV2Harness();
    const tracked = await mount(page, css, js, { semantic: true });
    await page.getByTestId("loom-search-v2.query").fill("the runs migration");
    await page.getByTestId("loom-search-v2.search").click();
    await expect(page.getByTestId("loom-search-v2.facet.note")).toBeVisible();
    // Scope to file -> only the pet (file) block survives.
    await page.getByTestId("loom-search-v2.facet.file").click();
    await expect(page.getByTestId("loom-search-v2.result.blk-pet")).toBeVisible();
    await expect(page.getByTestId("loom-search-v2.result.blk-mig")).toHaveCount(0);
    expect(tracked.external).toEqual([]);
  });

  test("save-as-view persists the search as a Loom view (MT-262)", async ({ page }) => {
    const { js, css } = await buildLoomSearchV2Harness();
    const tracked = await mount(page, css, js, { semantic: true });
    await page.getByTestId("loom-search-v2.query").fill("database migration");
    await page.getByTestId("loom-search-v2.search").click();
    await expect(page.locator('[data-testid^="loom-search-v2.result."]').first()).toBeVisible();
    await page.getByTestId("loom-search-v2.save-view").click();
    await expect(page.getByTestId("loom-search-v2.view-status")).toContainText("view-1");
    expect(tracked.createdViews).toEqual(["view-1"]);
    expect(tracked.external).toEqual([]);
  });

  test("no semantic model -> honest keyword/fuzzy-only status, no fabricated semantic", async ({ page }) => {
    const { js, css } = await buildLoomSearchV2Harness();
    const tracked = await mount(page, css, js, { semantic: false });
    await page.getByTestId("loom-search-v2.query").fill("database migration");
    await page.getByTestId("loom-search-v2.search").click();
    await expect(page.getByTestId("loom-search-v2.status")).toContainText("keyword/fuzzy only");
    // No result reports a fabricated non-zero vector similarity.
    const sims = await page
      .locator('[data-testid^="loom-search-v2.result."]')
      .evaluateAll((els) => els.map((e) => Number((e as HTMLElement).getAttribute("data-vector-sim"))));
    expect(sims.every((v) => v === 0)).toBe(true);
    expect(tracked.external).toEqual([]);
  });
});
