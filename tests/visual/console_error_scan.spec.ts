import { expect, test } from "./console_error_scan";

test("console scan passes when the page has no renderer errors", async ({ page, consoleScan }) => {
  await page.setContent(`
    <main data-testid="console-scan-happy">
      <h1>Console Scan Happy Path</h1>
    </main>
  `);

  await expect(page.locator("[data-testid='console-scan-happy']")).toBeVisible();
  expect(consoleScan.events()).toHaveLength(0);
});

test("console scan fails when a runtime exception is thrown", async ({ page }) => {
  test.fail(true, "The auto consoleScan fixture must fail this test after catching the injected error.");

  await page.setContent("<main>Console scan injected failure</main>");
  await page.evaluate(() => {
    window.setTimeout(() => {
      throw new Error("inject-test-runtime-exception");
    }, 0);
  });
  await page.waitForTimeout(100);
});
