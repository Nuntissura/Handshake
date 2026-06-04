import { expect, test } from "./console_error_scan";

import {
  WebView2CdpDriver,
  assertReadOnlyExpression,
  isPngBuffer,
} from "./webview2_cdp_driver";

test("read-only screenshot API returns non-empty PNG bytes", async ({ page }) => {
  await page.setContent(`
    <main data-testid="visual-smoke" style="width: 320px; height: 160px; padding: 16px">
      <h1>Handshake Visual Debug</h1>
      <p>MT-021 sample screenshot fixture</p>
    </main>
  `);

  const readOnly = WebView2CdpDriver.readOnly(page);
  await expect(readOnly.locatorCount("[data-testid='visual-smoke']")).resolves.toBe(1);

  const screenshot = await readOnly.screenshot({ fullPage: true });
  expect(screenshot.byteLength).toBeGreaterThan(8);
  expect(isPngBuffer(screenshot)).toBe(true);
});

test("read-only evaluate rejects obvious write paths", () => {
  expect(() => assertReadOnlyExpression("document.body.appendChild(document.createElement('div'))"))
    .toThrow(/DOM tree mutation/);
  expect(() => assertReadOnlyExpression("document.querySelector('[data-testid=visual-smoke]')?.textContent"))
    .not.toThrow();
});

test("can attach to a running WebView2 CDP endpoint when one is supplied", async () => {
  const endpoint = WebView2CdpDriver.endpointFromEnv();
  test.skip(!endpoint, "Set HANDSHAKE_WEBVIEW2_CDP_PORT or HANDSHAKE_WEBVIEW2_CDP_ENDPOINT to run the live WebView2 attach smoke.");

  const driver = new WebView2CdpDriver({ endpointUrl: endpoint, verifyTauriIpcPort: true });
  try {
    await driver.connect();
    const screenshot = await driver.readOnly().screenshot({ fullPage: true });
    expect(screenshot.byteLength).toBeGreaterThan(8);
    expect(isPngBuffer(screenshot)).toBe(true);
  } finally {
    await driver.disconnect();
  }
});
