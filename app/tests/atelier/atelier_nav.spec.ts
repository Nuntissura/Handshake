import { expect, test } from "@playwright/test";

// WP-KERNEL-005 front-end navigation proof: drive the real React app to the
// Atelier > CKC panel and through every section, asserting each renders live
// data from the backend HTTP API. Targets the stable data-testid hooks the
// AtelierPanel exposes. Headless; no desktop window.

test.describe("Atelier panel navigation (WP-KERNEL-005)", () => {
  test("navigates Atelier > CKC through all four sections with live data", async ({ page }) => {
    await page.goto("/");
    await expect(page.getByTestId("main-window")).toBeVisible({ timeout: 30_000 });

    // The module buttons live in the project rail; open it if collapsed.
    const ckc = page.getByTestId("module-ckc");
    if (!(await ckc.isVisible().catch(() => false))) {
      await page.getByTestId("project-drawer.toggle").click();
    }
    await expect(ckc).toBeVisible();
    await ckc.click();

    // Atelier defaults to the CKC tab; click the stable atelier tab id explicitly if present for safety.
    const atelierTab = page.locator('[data-testid$=".tab.atelier"]').first();
    if (await atelierTab.isVisible().catch(() => false)) {
      await atelierTab.click();
    }

    const panel = page.getByTestId("atelier-panel");
    await expect(panel).toBeVisible({ timeout: 15_000 });

    // --- Overview: live table counts from the backend ---
    await page.getByTestId("atelier-subnav-overview").click();
    await expect(page.getByTestId("atelier-section-overview")).toBeVisible();
    await expect(page.getByTestId("atelier-overview-table")).toBeVisible({ timeout: 15_000 });
    // The overview table must list the curated atelier tables (real rows).
    await expect(
      page.getByTestId("atelier-overview-table").getByText("atelier_intake_item"),
    ).toBeVisible();

    // --- Intake: list + create a batch round-trips through the API ---
    await page.getByTestId("atelier-subnav-intake").click();
    await expect(page.getByTestId("atelier-section-intake")).toBeVisible();
    await expect(page.getByTestId("atelier-intake-batches")).toBeVisible({ timeout: 15_000 });

    const label = `playwright-nav-${Date.now()}`;
    await page.getByTestId("atelier-intake-source-label").fill(label);
    await page.getByTestId("atelier-intake-open").click();
    // The newly-opened batch must appear in the live list.
    await expect(
      page.getByTestId("atelier-intake-batches").getByText(label),
    ).toBeVisible({ timeout: 15_000 });

    // --- Command Corpus section renders ---
    await page.getByTestId("atelier-subnav-corpus").click();
    await expect(page.getByTestId("atelier-section-corpus")).toBeVisible();
    await expect(page.getByTestId("atelier-corpus-table")).toBeVisible({ timeout: 15_000 });

    // --- Stealth Windows section renders ---
    await page.getByTestId("atelier-subnav-stealth").click();
    await expect(page.getByTestId("atelier-section-stealth")).toBeVisible();
    await expect(page.getByTestId("atelier-stealth-table")).toBeVisible({ timeout: 15_000 });
  });
});
