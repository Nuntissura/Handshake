// WP-KERNEL-009 MT-260 AILoomJobs — OFFLINE Playwright runtime proof.
//
// Proves the named GUI capabilities against a REAL browser with ZERO external
// network (every request is intercepted by page.route). The route handler is a
// faithful in-memory model of the backend's confirm-to-promote authority:
//   - run a job  -> records PENDING suggestions (no edge/field yet),
//   - accept     -> the suggestion is promoted (leaves the pending list),
//   - reject     -> the suggestion is rejected (leaves the pending list,
//                   authority untouched),
//   - accept-all -> promotes every pending suggestion of the kind,
//   - no-model   -> the run returns 409 HSK-409-LOOM-AI-NO-MODEL and the panel
//                   renders the typed decline (NEVER a fabricated suggestion).
//
// This mounts the REAL product panel (LoomAiReviewPanel) — not a mock — so it
// is a GUI runtime proof of the review surface, not a unit test.

import { expect, test } from "./console_error_scan";

import { buildLoomAiReviewHarness } from "./build_loom_ai_review_harness";

const apiBase = "http://127.0.0.1:37501";
const ws = "ws-mt260";

const PAGE_SHELL = (css: string) => `<!doctype html>
<html><head><meta charset="utf-8"><style>${css}
  body { margin:0; font-family: ui-sans-serif, system-ui, sans-serif; }
</style></head>
<body><main data-testid="capture-root" style="padding:16px; width:1000px;">
  <div id="harness-root"></div>
</main></body></html>`;

type Suggestion = {
  suggestion_id: string;
  job_id: string;
  workspace_id: string;
  kind: "auto_tag" | "auto_caption" | "link_suggest";
  block_id: string;
  target_block_id: string | null;
  suggested_value: Record<string, unknown>;
  model_attribution: Record<string, unknown>;
  prompt_sha256: string;
  output_sha256: string;
  review_state: "pending" | "accepted" | "rejected" | "promoted";
  decided_by: string | null;
  decision_reason: string | null;
  recorded_event_id: string;
  decided_event_id: string | null;
  promotion_requested_event_id: string | null;
  promotion_accepted_event_id: string | null;
  promoted_artifact_ref: string | null;
  value_hash: string;
  created_at_utc: string;
};

let suggestionSeq = 0;
function newSuggestion(over: Partial<Suggestion>): Suggestion {
  suggestionSeq += 1;
  const id = `LAIS-${suggestionSeq.toString().padStart(8, "0")}`;
  return {
    suggestion_id: id,
    job_id: "LAIJ-1",
    workspace_id: ws,
    kind: "auto_tag",
    block_id: "blk-1",
    target_block_id: null,
    suggested_value: { tag: "roadmap" },
    model_attribution: { model: "qwen-test", trace_id: "t1" },
    prompt_sha256: "a".repeat(64),
    output_sha256: "b".repeat(64),
    review_state: "pending",
    decided_by: null,
    decision_reason: null,
    recorded_event_id: "EV-1",
    decided_event_id: null,
    promotion_requested_event_id: null,
    promotion_accepted_event_id: null,
    promoted_artifact_ref: null,
    value_hash: "c".repeat(64),
    created_at_utc: "2026-06-17T00:00:00Z",
    ...over,
  };
}

test.describe("WP-KERNEL-009 MT-260 AI Loom review (offline GUI proof)", () => {
  test.setTimeout(180_000);

  async function mount(
    page: import("playwright").Page,
    css: string,
    js: string,
    opts: { noModel?: boolean } = {},
  ) {
    const store = new Map<string, Suggestion>();
    const external: string[] = [];
    const posts: string[] = [];

    await page.route("**/*", async (route) => {
      const req = route.request();
      const url = req.url();

      if (url.startsWith(apiBase)) {
        const parsed = new URL(url);
        const pathname = parsed.pathname;
        const method = req.method();

        // The panel auto-loads workspace blocks via the "all" view.
        if (pathname.endsWith("/loom/views/all")) {
          await route.fulfill({
            status: 200,
            contentType: "application/json",
            body: JSON.stringify({
              view_type: "all",
              blocks: [
                { block_id: "blk-1", workspace_id: ws, content_type: "note", title: "Alpha", pinned: false, favorite: false, created_at: "", updated_at: "", derived: { backlink_count: 0, mention_count: 0, tag_count: 0, preview_status: "none" } },
                { block_id: "blk-2", workspace_id: ws, content_type: "note", title: "Beta", pinned: false, favorite: false, created_at: "", updated_at: "", derived: { backlink_count: 0, mention_count: 0, tag_count: 0, preview_status: "none" } },
              ],
            }),
          });
          return;
        }

        // List pending suggestions.
        if (pathname.endsWith("/loom/ai-suggestions") && method === "GET") {
          const stateFilter = parsed.searchParams.get("state");
          const rows = [...store.values()].filter((s) => !stateFilter || s.review_state === stateFilter);
          await route.fulfill({ status: 200, contentType: "application/json", body: JSON.stringify(rows) });
          return;
        }

        // Run a job.
        if (pathname.endsWith("/loom/ai-jobs") && method === "POST") {
          if (opts.noModel) {
            await route.fulfill({
              status: 409,
              contentType: "application/json",
              body: JSON.stringify({ error: "HSK-409-LOOM-AI-NO-MODEL" }),
            });
            return;
          }
          const payload = JSON.parse(req.postData() ?? "{}") as { kind: Suggestion["kind"] };
          // Record one PENDING suggestion per block (no authority artifact yet).
          const created: Suggestion[] = ["blk-1", "blk-2"].map((blk) =>
            newSuggestion({
              kind: payload.kind,
              block_id: blk,
              target_block_id: payload.kind === "link_suggest" ? (blk === "blk-1" ? "blk-2" : "blk-1") : null,
              suggested_value:
                payload.kind === "auto_caption"
                  ? { caption: `Caption for ${blk}` }
                  : payload.kind === "link_suggest"
                    ? { reason: "related" }
                    : { tag: "roadmap" },
            }),
          );
          for (const s of created) store.set(s.suggestion_id, s);
          await route.fulfill({
            status: 200,
            contentType: "application/json",
            body: JSON.stringify({ job_id: "LAIJ-1", kind: payload.kind, suggestions: created }),
          });
          return;
        }

        // Accept one (decide+promote): pending -> promoted.
        const acceptMatch = pathname.match(/\/loom\/ai-suggestions\/([^/]+)\/accept$/);
        if (acceptMatch && method === "POST") {
          posts.push(`accept:${acceptMatch[1]}`);
          const s = store.get(acceptMatch[1]);
          if (!s) {
            await route.fulfill({ status: 404, body: "not found" });
            return;
          }
          s.review_state = "promoted";
          s.promoted_artifact_ref = "edge-1";
          await route.fulfill({ status: 200, contentType: "application/json", body: JSON.stringify(s) });
          return;
        }

        // Reject one: pending -> rejected (authority untouched).
        const rejectMatch = pathname.match(/\/loom\/ai-suggestions\/([^/]+)\/reject$/);
        if (rejectMatch && method === "POST") {
          posts.push(`reject:${rejectMatch[1]}`);
          const s = store.get(rejectMatch[1]);
          if (!s) {
            await route.fulfill({ status: 404, body: "not found" });
            return;
          }
          s.review_state = "rejected";
          await route.fulfill({ status: 200, contentType: "application/json", body: JSON.stringify(s) });
          return;
        }

        // Accept-all-of-kind.
        const acceptAllMatch = pathname.match(/\/loom\/ai-jobs\/([^/]+)\/accept-all$/);
        if (acceptAllMatch && method === "POST") {
          const body = JSON.parse(req.postData() ?? "{}") as { kind?: Suggestion["kind"] };
          const promoted: string[] = [];
          for (const s of store.values()) {
            if (s.review_state !== "pending") continue;
            if (body.kind && s.kind !== body.kind) continue;
            s.review_state = "promoted";
            s.promoted_artifact_ref = "edge-bulk";
            promoted.push(s.suggestion_id);
          }
          posts.push(`accept-all:${promoted.length}`);
          await route.fulfill({
            status: 200,
            contentType: "application/json",
            body: JSON.stringify({ promoted, denied: [], skipped: [] }),
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
      (window as unknown as { __mt260Config: { ws: string } }).__mt260Config = c;
    }, { ws });
    await page.addScriptTag({ content: js });
    await page.waitForFunction(
      () => (window as unknown as { __mt260HarnessReady?: boolean }).__mt260HarnessReady === true,
    );

    return { store, external, posts };
  }

  test("run auto_tag -> pending suggestions render, accept promotes (leaves pending list)", async ({ page }) => {
    const { js, css } = await buildLoomAiReviewHarness();
    const tracked = await mount(page, css, js);

    await expect(page.getByTestId("loom-ai-review-panel")).toBeVisible();
    await expect(page.getByTestId("loom-ai-empty")).toBeVisible();

    await page.getByTestId("loom-ai-run-auto_tag").click();
    await expect(page.getByTestId("loom-ai-group-auto_tag")).toBeVisible();
    const rows = page.locator('[data-testid^="loom-ai-suggestion-"]');
    await expect(rows).toHaveCount(2);

    // Accept the first suggestion -> it promotes and leaves the pending list.
    // Scope to per-item accept buttons INSIDE list rows (the accept-all button
    // lives in the group header and also starts with "loom-ai-accept-").
    const firstAccept = page
      .locator('[data-testid^="loom-ai-suggestion-"] [data-testid^="loom-ai-accept-"]')
      .first();
    await firstAccept.click();
    await expect(rows).toHaveCount(1);
    expect(tracked.posts.some((p) => p.startsWith("accept:"))).toBe(true);
    expect(tracked.external).toEqual([]);
  });

  test("reject removes the suggestion from pending; authority untouched", async ({ page }) => {
    const { js, css } = await buildLoomAiReviewHarness();
    const tracked = await mount(page, css, js);
    await page.getByTestId("loom-ai-run-auto_tag").click();
    await expect(page.locator('[data-testid^="loom-ai-suggestion-"]')).toHaveCount(2);

    await page.locator('[data-testid^="loom-ai-reject-"]').first().click();
    await expect(page.locator('[data-testid^="loom-ai-suggestion-"]')).toHaveCount(1);
    expect(tracked.posts.some((p) => p.startsWith("reject:"))).toBe(true);
    // No suggestion was promoted by the reject.
    expect([...tracked.store.values()].some((s) => s.review_state === "promoted")).toBe(false);
    expect(tracked.external).toEqual([]);
  });

  test("accept-all-of-kind promotes every pending suggestion of the kind", async ({ page }) => {
    const { js, css } = await buildLoomAiReviewHarness();
    const tracked = await mount(page, css, js);
    await page.getByTestId("loom-ai-run-auto_tag").click();
    await expect(page.locator('[data-testid^="loom-ai-suggestion-"]')).toHaveCount(2);

    await page.getByTestId("loom-ai-accept-all-auto_tag").click();
    await expect(page.getByTestId("loom-ai-empty")).toBeVisible();
    expect(tracked.posts.some((p) => p === "accept-all:2")).toBe(true);
    expect([...tracked.store.values()].every((s) => s.review_state === "promoted")).toBe(true);
    expect(tracked.external).toEqual([]);
  });

  test("no model configured -> typed decline, never a fabricated suggestion", async ({ page }) => {
    const { js, css } = await buildLoomAiReviewHarness();
    const tracked = await mount(page, css, js, { noModel: true });
    await expect(page.getByTestId("loom-ai-review-panel")).toBeVisible();

    await page.getByTestId("loom-ai-run-auto_tag").click();
    await expect(page.getByTestId("loom-ai-no-model")).toBeVisible();
    await expect(page.locator('[data-testid^="loom-ai-suggestion-"]')).toHaveCount(0);
    expect(tracked.store.size).toBe(0);
    expect(tracked.external).toEqual([]);
  });
});
