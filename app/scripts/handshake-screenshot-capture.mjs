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
const waitTimeout = positiveInt(args["wait-timeout"] ?? "30000", "wait-timeout");

const findings = [];
const recordFinding = (code, message, details = null) => {
  findings.push({
    code,
    message,
    details,
  });
};

const browser = await launchBrowser();
let exitCode = 0;

try {
  const page = await browser.newPage({ viewport: { width, height } });

  page.on("pageerror", (error) => {
    recordFinding("RUNTIME_ERROR", error?.message ?? String(error), {
      stage: "runtime",
      stack: error?.stack,
    });
  });

  page.on("console", (message) => {
    if (message.type() !== "error") {
      return;
    }
    recordFinding("CONSOLE_ERROR", message.text(), {
      location: message.location(),
      type: message.type(),
    });
  });

  await page.goto(sourceUrl, { waitUntil: "networkidle", timeout: Math.min(waitTimeout, 45_000) });
  await prepareVisibleTarget(page, targetRef, scope);

  const selector = selectorOverride ?? selectorFor(scope, targetRef);
  let bytes;
  if (selector) {
    const locator = page.locator(selector).first();
    try {
      await locator.waitFor({ state: "visible", timeout: waitTimeout });
    } catch (error) {
      recordFinding(
        "MISSING_SELECTOR",
        `Selector ${selector} was not visible for scope=${scope} target_ref=${targetRef}`,
        { error: error instanceof Error ? error.message : String(error) },
      );
      exitCode = 1;
    }
    if ((await locator.count()) === 0) {
      recordFinding("MISSING_SELECTOR", `Selector ${selector} matched no nodes.`, { selector });
      exitCode = 1;
      bytes = await page.screenshot({ type: "png", fullPage: true });
    } else {
      bytes = await locator.screenshot({ type: "png" });
    }
  } else {
    bytes = await page.screenshot({ type: "png", fullPage: true });
  }

  await mkdir(dirname(resolve(output)), { recursive: true });
  await writeFile(output, bytes);

  const result = {
    schema_id: "hsk.product_screenshot_browser_adapter_result@1",
    scope,
    selector: selector ?? null,
    target_ref: targetRef,
    source_url: sourceUrl,
    output_path: resolve(output),
    width,
    height,
    dimensions: { width, height },
    status: findings.length === 0 ? "ok" : "failed",
    findings,
  };

  process.stdout.write(JSON.stringify(result) + "\n");
  if (findings.length > 0) {
    exitCode = 1;
  }
} finally {
  await browser.close();
}

process.exitCode = exitCode;

async function launchBrowser() {
  const attempts = [{}, { channel: "chrome" }, { channel: "msedge" }];
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

async function prepareVisibleTarget(page, targetRef, scope) {
  const normalizedScope = scope.trim().toLowerCase();
  const normalizedTarget = targetRef.trim();

  if (normalizedScope === "panel") {
    const target = normalizedTarget.replace(/^panel:\/\//, "");
    if (target.includes("session-spawn-tree")) {
      const dccButton = page.getByRole("button", { name: /CKC|Kernel DCC/i });
      if (await dccButton.count()) {
        await dccButton.first().click();
      }
    }
    return;
  }

  if (normalizedScope === "module") {
    const moduleId = normalizedTarget.replace(/^module:\/\//, "");
    const moduleButton = page.getByTestId(`module-${moduleId}`);
    if (await moduleButton.count()) {
      await moduleButton.first().click();
    }
    return;
  }

  if (normalizedScope === "pane") {
    const paneId = normalizedTarget.replace(/^pane:\/\//, "");
    const pane = page.locator(`[data-pane-id="${cssAttributeEscape(paneId)}"]`);
    if (await pane.count() === 0) {
      return;
    }
    const paneTab = pane.locator("[data-pane-tab]");
    if (await paneTab.count() > 0) {
      await paneTab.first().click();
    }
    return;
  }

  if (normalizedScope === "drawer") {
    const drawerRef = normalizedTarget.replace(/^drawer:\/\//, "");
    if (/project/i.test(drawerRef)) {
      const toggle = page.getByTestId("project-drawer.toggle");
      if ((await toggle.count()) > 0 && (await page.locator('[data-project-drawer-open="false"]').count()) > 0) {
        await toggle.first().click();
      }
      return;
    }
    if (/file/i.test(drawerRef)) {
      const toggle = page.getByTestId("file-drawer.toggle");
      if ((await toggle.count()) > 0 && (await page.locator('[data-file-drawer-open="false"]').count()) > 0) {
        await toggle.first().click();
      }
      return;
    }
    if (/bottom|search|status/i.test(drawerRef)) {
      const toggle = page.getByTestId("bottom-drawer.toggle");
      if ((await toggle.count()) > 0 && (await page.locator('[data-bottom-drawer-open="false"]').count()) > 0) {
        await toggle.first().click();
      }
    }
    return;
  }

  if (normalizedScope === "status") {
    const toggle = page.getByTestId("bottom-drawer.toggle");
    if ((await toggle.count()) > 0 && (await page.locator('[data-bottom-drawer-open="false"]').count()) > 0) {
      await toggle.first().click();
    }
    return;
  }

  if (normalizedScope === "main-window" || normalizedScope === "app") {
    await page.waitForSelector("#main-window", { state: "visible", timeout: waitTimeout });
  }
}

function selectorFor(scope, targetRef) {
  const normalizedScope = scope.trim().toLowerCase();
  const normalizedTarget = targetRef.trim();

  if (normalizedScope === "full-app") {
    return null;
  }

  if (normalizedScope === "main-window" || normalizedScope === "app") {
    return "#main-window";
  }

  if (normalizedScope === "pane") {
    const paneId = normalizedTarget.replace(/^pane:\/\//, "");
    return `[data-pane-id="${cssAttributeEscape(paneId)}"]`;
  }

  if (normalizedScope === "drawer") {
    const drawerRef = normalizedTarget.replace(/^drawer:\/\//, "");
    if (/project/i.test(drawerRef)) {
      return `[data-testid="project-drawer.toggle"]`;
    }
    if (/file/i.test(drawerRef)) {
      return `[data-testid="file-drawer"]`;
    }
    if (/bottom|search|status/i.test(drawerRef)) {
      return `[data-testid="search-status-region"], [data-stable-id="search-status-region"]`;
    }
    return `[data-testid="project-rail"], [data-stable-id="module-rail"]`;
  }

  if (normalizedScope === "status") {
    return `[data-testid="search-status-region"], [data-stable-id="search-status-region"]`;
  }

  if (normalizedTarget.includes("session-spawn-tree")) {
    return '[data-testid="kernel-dcc-session-spawn-tree"]';
  }

  if (normalizedTarget.includes("kernel-dcc") || normalizedTarget.includes("dcc/") || normalizedTarget.includes("kernel_dcc")) {
    return '[data-testid="kernel-dcc-projection"], [data-testid="kernel-dcc-projection-loading"], [data-stable-id="kernel-dcc-projection-unavailable"]';
  }

  if (normalizedScope === "panel") {
    const panelId = normalizedTarget.replace(/^panel:\/\//, "").split("/").pop();
    return `[aria-labelledby="${cssAttributeEscape(panelId)}"]`;
  }

  if (normalizedScope === "module") {
    const moduleId = normalizedTarget.replace(/^module:\/\//, "").replaceAll("/", "-");
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
