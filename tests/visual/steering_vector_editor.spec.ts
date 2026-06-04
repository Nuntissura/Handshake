import { expect, test } from "./console_error_scan";

import path from "node:path";

import { buildSteeringVectorEditorHarness } from "./build_steering_vector_editor_harness";

// MT-098: Activation Steering Vector Editor + A/B compare.
//
// The visual smoke mounts the REAL <SteeringVectorEditor> component into
// Chromium via a Vite IIFE bundle. Deterministic Tauri IPC mocks are installed
// inside the harness so this can run offline, but selectors, layout, state
// transitions, and the generateAb request shape come from the shipped component.

const repoRoot = path.resolve(__dirname, "..", "..");
const baselineDir = path.join(repoRoot, ".GOV", "visual_baselines");

const PAGE_SHELL = (css: string) => `<!doctype html>
<html>
  <head><meta charset="utf-8"><style>${css}
    body { margin: 0; font-family: ui-sans-serif, system-ui, sans-serif; background: #fff; }
    label { display: block; margin: 4px 0; }
    label > span { display: block; font-size: 12px; }
  </style></head>
  <body>
    <main data-testid="capture-root" style="padding:16px; background:#fff; color:#111; width:1100px;">
      <div id="harness-root"></div>
    </main>
  </body>
</html>`;

// eslint-disable-next-line @typescript-eslint/no-explicit-any
async function mountEditor(page: any): Promise<void> {
  const { js, css } = await buildSteeringVectorEditorHarness();
  await page.setContent(PAGE_SHELL(css), { waitUntil: "domcontentloaded" });
  await page.addScriptTag({ content: js });
  await page.locator("[data-testid='steering-vector-editor']").waitFor({ state: "attached" });
  await page.locator("[data-testid='steering-vector-editor.table']").waitFor({ state: "visible" });
}

test("steering_vector_editor renders the shipped editor and contrastive controls", async ({ page }) => {
  await mountEditor(page);

  await expect(page.locator("[data-testid='steering-vector-editor']")).toBeVisible();
  await expect(page.locator("[data-testid='steering-vector-editor.table']")).toBeVisible();
  await expect(page.locator("[data-testid='steering-vector-editor.ab-compare']")).toBeVisible();
  await expect(
    page.locator("[data-testid='steering-vector-editor.ab-compare.inactive-select']"),
  ).toBeVisible();
  await expect(
    page.locator("[data-testid='steering-vector-editor.ab-compare.active-select']"),
  ).toBeVisible();
  await expect(page.locator("[data-testid='contrastive-capture-wizard']")).toBeVisible();
  await expect(page.locator("[data-testid='contrastive-capture-wizard.capture']")).toBeVisible();

  await page.locator("[data-testid='capture-root']").screenshot({
    path: path.join(baselineDir, "steering-vector-editor.png"),
  });
});

test("steering_vector_editor A/B compare sends selected vectors and renders returned completions", async ({
  page,
}) => {
  await mountEditor(page);

  const beforeVectorId = "019a1b2c-0000-7000-8000-000000000002";
  const afterVectorId = "019a1b2c-0000-7000-8000-000000000001";

  await page
    .locator("[data-testid='steering-vector-editor.ab-compare.inactive-select']")
    .selectOption(beforeVectorId);
  await page.locator("[data-testid='ab-compare.prompts']").fill("describe the scene");
  await page.locator("[data-testid='ab-compare.generate']").click();

  await expect(page.locator("[data-testid='ab-compare.pair.0.inactive-text']")).toContainText(
    `before:${beforeVectorId}:describe the scene`,
  );
  await expect(page.locator("[data-testid='ab-compare.pair.0.active-text']")).toContainText(
    `after:${afterVectorId}:describe the scene`,
  );
  await expect(page.locator("[data-testid='ab-compare.apply-active']")).toBeVisible();
  await expect(page.locator("[data-testid='ab-compare.revert-inactive']")).toBeVisible();

  const inactiveBox = await page
    .locator("[data-testid='ab-compare.pair.0.inactive']")
    .boundingBox();
  const activeBox = await page
    .locator("[data-testid='ab-compare.pair.0.active']")
    .boundingBox();
  expect(inactiveBox).not.toBeNull();
  expect(activeBox).not.toBeNull();
  if (inactiveBox && activeBox) {
    const overlapWidth = Math.max(
      0,
      Math.min(inactiveBox.x + inactiveBox.width, activeBox.x + activeBox.width)
        - Math.max(inactiveBox.x, activeBox.x),
    );
    const overlapHeight = Math.max(
      0,
      Math.min(inactiveBox.y + inactiveBox.height, activeBox.y + activeBox.height)
        - Math.max(inactiveBox.y, activeBox.y),
    );
    expect(overlapWidth * overlapHeight).toBe(0);
  }

  const request = await page.evaluate(() => window.__HS_STEERING_GENERATE_AB_REQUEST__);
  expect(request).toMatchObject({
    modelId: "019a1b2c-0000-7000-8000-aaaaaaaaaaaa",
    prompts: ["describe the scene"],
    activeVectorIds: [afterVectorId],
    inactiveVectorIds: [beforeVectorId],
    maxTokens: 64,
  });
});
