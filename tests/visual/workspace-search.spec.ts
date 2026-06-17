import { expect, test, type Route } from "./console_error_scan";

import path from "node:path";

import { buildWorkspaceSearchHarness } from "./build_workspace_search_harness";

const apiBaseUrl = "http://127.0.0.1:37501";
const richDocumentId = "KRD-00000000000000000000000000000001";
const repoRoot = path.resolve(__dirname, "..", "..");
const baselineDir = path.join(repoRoot, ".GOV", "visual_baselines");

const PAGE_SHELL = (css: string) => `<!doctype html>
<html>
  <head><meta charset="utf-8"><style>${css}
    body { margin: 0; font-family: ui-sans-serif, system-ui, sans-serif; background: #f8fafc; }
  </style></head>
  <body>
    <main data-testid="capture-root" style="padding:16px; width:1040px;">
      <div id="harness-root"></div>
    </main>
  </body>
</html>`;

function jsonResponse(route: Route, body: unknown) {
  return route.fulfill({
    status: 200,
    contentType: "application/json",
    body: JSON.stringify(body),
  });
}

function richDocumentLoadResponse() {
  return {
    document: {
      rich_document_id: richDocumentId,
      workspace_id: "w1",
      document_id: null,
      title: "Alpha design note",
      schema_version: "hsk.rich_document@1",
      doc_version: 4,
      content_json: {
        type: "doc",
        content: [{ type: "paragraph", content: [{ type: "text", text: "alpha beta alpha" }] }],
      },
      content_sha256: "a".repeat(64),
      crdt_document_id: `crdt-${richDocumentId}`,
      crdt_snapshot_id: null,
      promotion_receipt_event_id: null,
      projection_refs: {},
      project_ref: null,
      folder_ref: null,
      authority_label: "rich_document",
      owner_actor_kind: "operator",
      owner_actor_id: "operator",
      created_at: "2026-06-16T00:00:00Z",
      updated_at: "2026-06-16T00:00:00Z",
    },
    tree: {
      schema_version: "hsk.rich_document_tree@1",
      schema_matches: true,
      block_ids: [],
      blocks: [],
    },
    code_nodes: [],
  };
}

test("workspace search panel searches, opens, previews, cancels, and applies via rich-document save", async ({ page }) => {
  const { js, css } = await buildWorkspaceSearchHarness();
  const graphSearchUrls: string[] = [];
  const saveBodies: unknown[] = [];
  const externalRequests: string[] = [];

  await page.route("**/*", async (route) => {
    const url = route.request().url();
    if (!url.startsWith(apiBaseUrl)) {
      if (!url.startsWith("about:")) externalRequests.push(url);
      await route.abort("connectionfailed");
      return;
    }
    const parsed = new URL(url);
    if (parsed.pathname === "/workspaces/w1/loom/graph-search") {
      graphSearchUrls.push(url);
      return jsonResponse(route, [
        {
          result_kind: "knowledge_entity",
          source_kind: "document",
          ref_id: richDocumentId,
          title: "Alpha design note",
          excerpt: "alpha beta alpha",
          score: 7,
          metadata: { rich_document_id: richDocumentId, path: "src/app.md" },
        },
        {
          result_kind: "loom_block",
          source_kind: "loom_block",
          ref_id: "block-alpha",
          title: "Alpha Loom block",
          excerpt: "alpha from a Loom block",
          score: 4,
          metadata: { content_type: "note" },
        },
      ]);
    }
    if (parsed.pathname === `/knowledge/documents/${richDocumentId}` && route.request().method() === "GET") {
      return jsonResponse(route, richDocumentLoadResponse());
    }
    if (parsed.pathname === `/knowledge/documents/${richDocumentId}/save` && route.request().method() === "PUT") {
      saveBodies.push(route.request().postDataJSON());
      const loaded = richDocumentLoadResponse();
      return jsonResponse(route, {
        document: {
          ...loaded.document,
          doc_version: 5,
          content_json: {
            type: "doc",
            content: [{ type: "paragraph", content: [{ type: "text", text: "omega beta omega" }] }],
          },
        },
        save_receipt_event_id: "evt-workspace-search-save",
        backlinks_persisted: 0,
        backlinks_skipped_reason: null,
      });
    }
    return jsonResponse(route, {});
  });

  await page.setContent(PAGE_SHELL(css), { waitUntil: "domcontentloaded" });
  await page.addScriptTag({ content: js });
  await expect(page.getByTestId("workspace-search")).toBeVisible();

  await page.getByTestId("workspace-search.query").fill("Alpha");
  await page.getByTestId("workspace-search.replace").fill("omega");
  await page.getByTestId("workspace-search.kind-filter").selectOption("document");
  await page.getByTestId("workspace-search.tag-filter").fill("tag-1");
  await page.getByTestId("workspace-search.path-filter").fill("src/app");
  await page.getByTestId("workspace-search.search").click();

  await expect(page.getByTestId(`workspace-search.result.document.${richDocumentId}`)).toContainText("Document");
  const searchUrl = new URL(graphSearchUrls[0]);
  expect(searchUrl.searchParams.get("q")).toBe("Alpha");
  expect(searchUrl.searchParams.get("source_kinds")).toBe("document");
  expect(searchUrl.searchParams.get("tag_ids")).toBe("tag-1");
  expect(searchUrl.searchParams.get("path")).toBe("src/app");
  expect(searchUrl.searchParams.get("limit")).toBe("500");
  expect(searchUrl.searchParams.get("offset")).toBe("0");
  expect(searchUrl.searchParams.get("case_sensitive")).toBe("false");
  expect(searchUrl.searchParams.get("whole_word")).toBe("false");
  expect(searchUrl.searchParams.get("regex")).toBe("false");

  await page.getByTestId(`workspace-search.result.document.${richDocumentId}`).click();
  await expect
    .poll(async () => page.evaluate(() => window.__workspaceSearchOpenLog?.join("|") ?? ""))
    .toContain(`document:${richDocumentId}:Alpha:false:false:false`);

  await page.getByTestId("workspace-search.preview-replace").click();
  await expect(page.getByTestId(`workspace-search.preview.${richDocumentId}`)).toContainText("2 matches");
  await expect(page.getByTestId(`workspace-search.preview.${richDocumentId}`)).toContainText("omega beta omega");
  await expect(page.getByTestId(`workspace-search.preview.${richDocumentId}.match.1`)).toContainText("Match 1");
  await expect(page.getByTestId(`workspace-search.preview.${richDocumentId}.match.1.before`)).toContainText(
    "alpha beta alpha",
  );
  await expect(page.getByTestId(`workspace-search.preview.${richDocumentId}.match.1.after`)).toContainText(
    "omega beta alpha",
  );
  expect(saveBodies).toEqual([]);

  await page.getByTestId("workspace-search.cancel-replace").click();
  await expect(page.getByTestId(`workspace-search.preview.${richDocumentId}`)).toHaveCount(0);
  expect(saveBodies).toEqual([]);

  await page.getByTestId("workspace-search.preview-replace").click();
  await expect(page.getByTestId(`workspace-search.preview.${richDocumentId}`)).toBeVisible();
  await page.getByTestId("workspace-search.apply-replace").click();
  await expect(page.getByTestId("workspace-search.replace-status")).toContainText("evt-workspace-search-save");
  expect(saveBodies).toHaveLength(1);
  expect(JSON.stringify(saveBodies[0])).toContain("omega beta omega");
  expect(JSON.stringify(saveBodies[0])).toContain(`crdt-${richDocumentId}`);

  const box = await page.getByTestId("workspace-search").boundingBox();
  expect(box).not.toBeNull();
  if (box) {
    expect(box.width).toBeGreaterThan(600);
    expect(box.height).toBeGreaterThan(220);
  }
  await page.getByTestId("capture-root").screenshot({
    path: path.join(baselineDir, "workspace-search-panel.png"),
  });
  expect(externalRequests).toEqual([]);
});
