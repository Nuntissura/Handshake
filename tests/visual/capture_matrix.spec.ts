import fs from "node:fs/promises";
import path from "node:path";
import { expect, test } from "./console_error_scan";

import {
  BOOTSTRAP_BASELINE_HASH,
  assertBaselineDirectoryPolicy,
  captureAndCompareEntry,
  captureMatrixEntryFromProject,
  compareCaptureToBaseline,
  ensureContentAddressedBaseline,
  validateVisualRegressionReceipt,
} from "./capture_matrix";

test("capture matrix project entry bootstraps a content-addressed baseline", async ({ page }, testInfo) => {
  const entry = captureMatrixEntryFromProject(testInfo);
  const result = await captureAndCompareEntry(page, entry, {
    baselineRoot: testInfo.outputPath("baselines"),
    receiptRoot: testInfo.outputPath("receipts"),
    allowBootstrap: true,
    projectName: testInfo.project.name,
  });

  expect(result.status).toBe("bootstrapped");
  expect(result.captured_hash).toMatch(/^[0-9a-f]{64}$/);
  expect(result.manifest_entry.baseline_hash).toBe(result.captured_hash);
  expect(result.manifest_entry.baseline_hash).not.toBe(BOOTSTRAP_BASELINE_HASH);
  await expect(fs.stat(result.baseline_path)).resolves.toHaveProperty("size");
  expect(result.receipt).toBeNull();
});

test("identical render produces no receipt and deliberate drift emits a schema-valid receipt", async ({ page }, testInfo) => {
  const entry = {
    scenario_id: "fixture-drift-receipt",
    route: "fixture:normal",
    viewport: { width: 640, height: 360 },
    color_scheme: "light",
    locale: "en-US",
    edge_state_tag: "normal",
    wait_for: "[data-testid='capture-root']",
    mask_selectors: ["[data-testid='volatile-clock']"],
    baseline_hash: BOOTSTRAP_BASELINE_HASH,
  } as const;
  const baselineRoot = testInfo.outputPath("baselines");
  const receiptRoot = testInfo.outputPath("receipts");

  const baseline = await captureAndCompareEntry(page, entry, {
    baselineRoot,
    receiptRoot,
    allowBootstrap: true,
    projectName: testInfo.project.name,
  });
  const approvedEntry = baseline.manifest_entry;

  const identical = await captureAndCompareEntry(page, approvedEntry, {
    baselineRoot,
    receiptRoot,
    projectName: testInfo.project.name,
  });
  expect(identical.status).toBe("passed");
  expect(identical.receipt_path).toBeNull();

  const drift = await captureAndCompareEntry(page, {
    ...approvedEntry,
    route: "fixture:normal-drift",
  }, {
    baselineRoot,
    receiptRoot,
    projectName: testInfo.project.name,
  });
  expect(drift.status).toBe("drift");
  expect(drift.receipt_path).toMatch(/hbr-vis-regression-fixture-drift-receipt-.+\.jsonl$/);
  expect(drift.receipt?.pixel_diff_percent).toBeGreaterThan(0);
  expect(drift.receipt?.drift_regions.length).toBeGreaterThan(0);
  expect(validateVisualRegressionReceipt(drift.receipt!)).toBe(true);

  const receiptText = await fs.readFile(drift.receipt_path!, "utf8");
  expect(receiptText.trim()).toBeTruthy();
  expect(JSON.parse(receiptText)).toEqual(drift.receipt);
});

test("mask selectors suppress volatile-region drift", async ({ page }, testInfo) => {
  const entry = {
    scenario_id: "fixture-mask-proof",
    route: "fixture:masked-a",
    viewport: { width: 520, height: 320 },
    color_scheme: "dark",
    locale: "en-US",
    edge_state_tag: "normal",
    wait_for: "[data-testid='capture-root']",
    mask_selectors: ["[data-testid='volatile-clock']"],
    baseline_hash: BOOTSTRAP_BASELINE_HASH,
  } as const;
  const baselineRoot = testInfo.outputPath("baselines");

  const baseline = await captureAndCompareEntry(page, entry, {
    baselineRoot,
    receiptRoot: testInfo.outputPath("receipts"),
    allowBootstrap: true,
    projectName: testInfo.project.name,
  });
  const maskedDrift = await captureAndCompareEntry(page, {
    ...baseline.manifest_entry,
    route: "fixture:masked-b",
  }, {
    baselineRoot,
    receiptRoot: testInfo.outputPath("receipts"),
    projectName: testInfo.project.name,
  });

  expect(maskedDrift.status).toBe("passed");
  expect(maskedDrift.pixel_diff_percent).toBe(0);
  expect(maskedDrift.receipt).toBeNull();
});

test("baseline directory cap blocks oversized local stores", async ({}, testInfo) => {
  const baselineRoot = testInfo.outputPath("oversized-baselines");
  await fs.mkdir(baselineRoot, { recursive: true });
  await fs.writeFile(path.join(baselineRoot, "oversized.bin"), Buffer.alloc(4));

  await expect(assertBaselineDirectoryPolicy(baselineRoot, 3)).rejects.toThrow(/exceeds 3 bytes/);
});

test("content-addressed helper writes filename equal to sha256", async ({}, testInfo) => {
  const pngBytes = Buffer.from(
    "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8z8BQDwAFgwJ/l2fu1AAAAABJRU5ErkJggg==",
    "base64",
  );
  const baseline = await ensureContentAddressedBaseline(testInfo.outputPath("baselines"), pngBytes);
  expect(path.basename(baseline.path)).toBe(`${baseline.hash}.png`);

  const identical = await compareCaptureToBaseline({
    scenario_id: "content-address-proof",
    route: "fixture:normal",
    viewport: { width: 1, height: 1 },
    color_scheme: "light",
    locale: "en-US",
    edge_state_tag: "normal",
    wait_for: "[data-testid='capture-root']",
    mask_selectors: [],
    baseline_hash: baseline.hash,
  }, pngBytes, {
    baselineRoot: path.dirname(baseline.path),
    receiptRoot: testInfo.outputPath("receipts"),
  });
  expect(identical.status).toBe("passed");
});
