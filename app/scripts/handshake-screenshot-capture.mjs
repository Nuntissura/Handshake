#!/usr/bin/env node
import { mkdir, writeFile } from "node:fs/promises";
import { dirname, resolve } from "node:path";
import { chromium } from "playwright";

const args = parseArgs(process.argv.slice(2));
const scope = required(args, "scope");
const targetRef = required(args, "target-ref");
const sourceUrl = required(args, "source-url");
const output = required(args, "output");
const width = positiveInt(args.width ?? "1440", "width");
const height = positiveInt(args.height ?? "960", "height");
const selectorOverride = args.selector;

const browser = await launchBrowser();
try {
  const page = await browser.newPage({ viewport: { width, height } });
  await page.goto(sourceUrl, { waitUntil: "networkidle", timeout: 45_000 });
  await prepareVisibleTarget(page, targetRef);

  const selector = selectorOverride ?? selectorFor(scope, targetRef);
  let bytes;
  if (selector) {
    const locator = page.locator(selector).first();
    await locator.waitFor({ state: "visible", timeout: 30_000 });
    bytes = await locator.screenshot({ type: "png" });
  } else {
    bytes = await page.screenshot({ type: "png", fullPage: true });
  }

  await mkdir(dirname(resolve(output)), { recursive: true });
  await writeFile(output, bytes);
  process.stdout.write(
    JSON.stringify({
      schema_id: "hsk.product_screenshot_browser_adapter_result@1",
      scope,
      target_ref: targetRef,
      source_url: sourceUrl,
      output_path: resolve(output),
      width,
      height,
      selector: selector ?? "page",
    }) + "\n",
  );
} finally {
  await browser.close();
}

async function launchBrowser() {
  const attempts = [
    {},
    { channel: "chrome" },
    { channel: "msedge" },
  ];
  const failures = [];
  for (const options of attempts) {
    try {
      return await chromium.launch({ headless: true, ...options });
    } catch (error) {
      failures.push(`${options.channel ?? "bundled-chromium"}: ${error.message}`);
    }
  }
  throw new Error(`No Playwright Chromium browser could launch. ${failures.join(" | ")}`);
}

async function prepareVisibleTarget(page, targetRef) {
  if (!targetRef.includes("kernel-dcc") && !targetRef.includes("dcc/")) {
    return;
  }
  const dccButton = page.getByRole("button", { name: "Kernel DCC" });
  if (await dccButton.count()) {
    await dccButton.first().click();
    await page
      .locator('[data-testid="kernel-dcc-projection"], text=Kernel DCC projection unavailable')
      .first()
      .waitFor({ state: "visible", timeout: 30_000 });
  }
}

function selectorFor(scope, targetRef) {
  const normalizedScope = scope.trim().toLowerCase();
  if (normalizedScope === "full-app") {
    return null;
  }
  if (targetRef.includes("session-spawn-tree")) {
    return '[data-testid="kernel-dcc-session-spawn-tree"]';
  }
  if (targetRef.includes("kernel-dcc") || targetRef.includes("dcc/")) {
    return '[data-testid="kernel-dcc-projection"]';
  }
  if (normalizedScope === "panel") {
    const panelId = targetRef.replace(/^panel:\/\//, "").split("/").pop();
    return `[aria-labelledby="${cssAttributeEscape(panelId)}"]`;
  }
  if (normalizedScope === "module") {
    const moduleId = targetRef.replace(/^module:\/\//, "").replaceAll("/", "-");
    return `[data-testid="${cssAttributeEscape(moduleId)}"], [data-stable-id="${cssAttributeEscape(moduleId)}"]`;
  }
  throw new Error(`Unsupported screenshot scope: ${scope}`);
}

function parseArgs(argv) {
  const parsed = {};
  for (let index = 0; index < argv.length; index += 1) {
    const token = argv[index];
    if (!token.startsWith("--")) {
      throw new Error(`Unexpected positional argument: ${token}`);
    }
    const key = token.slice(2);
    const value = argv[index + 1];
    if (!value || value.startsWith("--")) {
      throw new Error(`Missing value for --${key}`);
    }
    parsed[key] = value;
    index += 1;
  }
  return parsed;
}

function required(values, key) {
  const value = values[key];
  if (!value || !value.trim()) {
    throw new Error(`Missing required --${key}`);
  }
  return value;
}

function positiveInt(value, key) {
  const parsed = Number.parseInt(value, 10);
  if (!Number.isInteger(parsed) || parsed <= 0) {
    throw new Error(`--${key} must be a positive integer`);
  }
  return parsed;
}

function cssAttributeEscape(value) {
  return String(value ?? "").replaceAll("\\", "\\\\").replaceAll('"', '\\"');
}
