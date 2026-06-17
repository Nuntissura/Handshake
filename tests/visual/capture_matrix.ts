const AjvModule = require("ajv/dist/2020");
const crypto = require("node:crypto");
const fs = require("node:fs/promises");
const fsSync = require("node:fs");
const path = require("node:path");
const PixelmatchModule = require("pixelmatch");
const { PNG } = require("pngjs");

const Ajv = AjvModule.default ?? AjvModule;
const pixelmatch = PixelmatchModule.default ?? PixelmatchModule;
const BOOTSTRAP_BASELINE_HASH = "0".repeat(64);
const MAX_LOCAL_BASELINE_BYTES = 100 * 1024 * 1024;
const HASH_PATTERN = /^[0-9a-f]{64}$/;

type Page = import("playwright").Page;
type TestInfo = import("playwright").TestInfo;

type CaptureMatrixEntry = {
  scenario_id: string;
  route: string;
  viewport: { width: number; height: number };
  color_scheme: "light" | "dark";
  locale: string;
  edge_state_tag: "normal" | "empty" | "error" | "loading" | "multi_item";
  wait_for: string;
  mask_selectors: string[];
  baseline_hash: string;
};

type DriftRegion = {
  x: number;
  y: number;
  w: number;
  h: number;
};

type VisualRegressionReceipt = {
  receipt_kind: "HBR_VIS_REGRESSION";
  schema_version: 1;
  receipt_uuid: string;
  emitted_at_utc: string;
  scenario_id: string;
  project_name: string | null;
  route: string;
  viewport: { width: number; height: number };
  color_scheme: "light" | "dark";
  locale: string;
  edge_state_tag: CaptureMatrixEntry["edge_state_tag"];
  baseline_hash: string;
  captured_hash: string;
  pixel_diff_percent: number;
  drift_regions: DriftRegion[];
  evidence_pointer: string | null;
};

type CaptureCompareOptions = {
  baselineRoot?: string;
  receiptRoot?: string;
  allowBootstrap?: boolean;
  maxDiffPercent?: number;
  projectName?: string;
};

type CaptureCompareResult = {
  status: "passed" | "bootstrapped" | "drift";
  captured_hash: string;
  baseline_hash: string;
  pixel_diff_percent: number;
  drift_regions: DriftRegion[];
  baseline_path: string;
  receipt_path: string | null;
  receipt: VisualRegressionReceipt | null;
  manifest_entry: CaptureMatrixEntry;
};

function repoRoot(): string {
  return path.resolve(__dirname, "..", "..");
}

function defaultBaselineRoot(): string {
  return path.join(repoRoot(), ".GOV", "visual_baselines");
}

function defaultReceiptRoot(): string {
  const artifactRoot = process.env.HANDSHAKE_ARTIFACT_ROOT
    ?? path.resolve(repoRoot(), "..", "Handshake_Artifacts");
  return path.join(artifactRoot, "visual-regression-receipts");
}

function defaultSchemaPath(): string {
  return path.join(repoRoot(), ".GOV", "roles_shared", "schemas", "hbr-vis-regression.schema.json");
}

function sha256Hex(bytes: Buffer | Uint8Array): string {
  return crypto.createHash("sha256").update(bytes).digest("hex");
}

function baselinePathForHash(baselineRoot: string, hash: string): string {
  if (!HASH_PATTERN.test(hash)) {
    throw new Error(`baseline_hash must be 64 lowercase hex characters: ${hash}`);
  }
  return path.join(baselineRoot, `${hash}.png`);
}

async function ensureContentAddressedBaseline(
  baselineRoot: string,
  bytes: Buffer,
): Promise<{ hash: string; path: string }> {
  const hash = sha256Hex(bytes);
  const baselinePath = baselinePathForHash(baselineRoot, hash);
  await fs.mkdir(baselineRoot, { recursive: true });
  if (!fsSync.existsSync(baselinePath)) {
    await fs.writeFile(baselinePath, bytes);
  }
  return { hash, path: baselinePath };
}

async function assertBaselineDirectoryPolicy(
  baselineRoot = defaultBaselineRoot(),
  maxBytes = MAX_LOCAL_BASELINE_BYTES,
): Promise<number> {
  if (process.env.HANDSHAKE_VISUAL_BASELINE_ROOT) return 0;
  const total = await directorySizeBytes(baselineRoot);
  if (total > maxBytes) {
    throw new Error(
      `visual baseline directory exceeds ${maxBytes} bytes; set HANDSHAKE_VISUAL_BASELINE_ROOT to externalize baselines`,
    );
  }
  return total;
}

async function directorySizeBytes(root: string): Promise<number> {
  if (!fsSync.existsSync(root)) return 0;
  let total = 0;
  const entries = await fs.readdir(root, { withFileTypes: true });
  for (const entry of entries) {
    const child = path.join(root, entry.name);
    if (entry.isDirectory()) {
      total += await directorySizeBytes(child);
    } else if (entry.isFile()) {
      total += (await fs.stat(child)).size;
    }
  }
  return total;
}

function validateCaptureMatrixEntry(value: unknown): CaptureMatrixEntry {
  const entry = value as CaptureMatrixEntry;
  if (!entry || typeof entry !== "object") throw new Error("capture matrix entry is missing");
  if (!entry.scenario_id || !/^[a-z0-9][a-z0-9_-]*$/.test(entry.scenario_id)) {
    throw new Error(`invalid scenario_id: ${entry.scenario_id}`);
  }
  if (!entry.route || typeof entry.route !== "string") throw new Error("route is required");
  if (!entry.viewport || !Number.isInteger(entry.viewport.width) || !Number.isInteger(entry.viewport.height)) {
    throw new Error("viewport width and height must be integers");
  }
  if (!["light", "dark"].includes(entry.color_scheme)) throw new Error("color_scheme must be light or dark");
  if (!entry.locale || typeof entry.locale !== "string") throw new Error("locale is required");
  if (!["normal", "empty", "error", "loading", "multi_item"].includes(entry.edge_state_tag)) {
    throw new Error(`invalid edge_state_tag: ${entry.edge_state_tag}`);
  }
  if (!entry.wait_for || typeof entry.wait_for !== "string") throw new Error("wait_for selector is required");
  if (!Array.isArray(entry.mask_selectors)) throw new Error("mask_selectors must be an array");
  if (!HASH_PATTERN.test(entry.baseline_hash)) throw new Error(`invalid baseline_hash: ${entry.baseline_hash}`);
  return entry;
}

function captureMatrixEntryFromProject(testInfo: TestInfo): CaptureMatrixEntry {
  return validateCaptureMatrixEntry(testInfo.project.metadata?.capture_matrix_entry);
}

async function captureEntryScreenshot(page: Page, entry: CaptureMatrixEntry): Promise<Buffer> {
  validateCaptureMatrixEntry(entry);
  await page.setViewportSize(entry.viewport);
  await page.emulateMedia({ colorScheme: entry.color_scheme });
  await routeCapturePage(page, entry);
  await page.waitForSelector(entry.wait_for, { state: "attached", timeout: 5_000 });
  const masks = entry.mask_selectors.map((selector) => page.locator(selector));
  return page.screenshot({
    type: "png",
    fullPage: true,
    animations: "disabled",
    caret: "hide",
    mask: masks,
    maskColor: "#00ff00",
    scale: "css",
  });
}

async function routeCapturePage(page: Page, entry: CaptureMatrixEntry): Promise<void> {
  if (entry.route.startsWith("fixture:")) {
    await page.setContent(captureFixtureHtml(entry), { waitUntil: "domcontentloaded" });
    return;
  }
  await page.goto(entry.route);
}

function captureFixtureHtml(entry: CaptureMatrixEntry): string {
  const variant = entry.route.slice("fixture:".length);
  const dark = entry.color_scheme === "dark";
  const drift = variant.includes("drift");
  const maskedB = variant.includes("masked-b");
  const constrained = variant.includes("constrained");
  const empty = variant.includes("empty");
  const title = empty ? "No Runs Yet" : constrained ? "Compact Capture" : "Handshake Visual Matrix";
  const body = empty
    ? "Empty-state evidence keeps the matrix honest when no operator data exists."
    : drift
      ? "Intentional drift changed this copy so the receipt path is exercised."
      : "Normal-state evidence for no-context visual verification.";
  const items = empty
    ? ""
    : "<li>HBR-VIS-001</li><li>HBR-VIS-002</li><li>HBR-VIS-003</li>";
  const volatileText = maskedB ? "masked-dynamic-B" : "masked-dynamic-A";
  const bg = dark ? "#111827" : "#f8fafc";
  const fg = dark ? "#f9fafb" : "#111827";
  const panel = dark ? "#1f2937" : "#ffffff";
  const accent = drift ? "#dc2626" : "#2563eb";

  return `<!doctype html>
<html lang="${entry.locale}">
<head>
  <meta charset="utf-8">
  <style>
    html, body { margin: 0; min-height: 100%; background: ${bg}; color: ${fg}; font-family: Arial, sans-serif; }
    [data-testid="capture-root"] { box-sizing: border-box; min-height: ${entry.viewport.height}px; padding: 24px; }
    .panel { background: ${panel}; border: 1px solid ${dark ? "#374151" : "#cbd5e1"}; border-radius: 8px; padding: 20px; max-width: 760px; }
    h1 { margin: 0 0 8px; font-size: ${constrained ? "24px" : "32px"}; }
    p { margin: 0 0 16px; line-height: 1.45; }
    .status { color: ${accent}; font-weight: 700; }
    .volatile { display: inline-block; margin-top: 16px; padding: 8px 10px; background: ${dark ? "#0f172a" : "#e2e8f0"}; }
  </style>
</head>
<body>
  <main data-testid="capture-root">
    <section class="panel">
      <h1>${title}</h1>
      <p>${body}</p>
      <p class="status">${entry.edge_state_tag}</p>
      <ul>${items}</ul>
      <span data-testid="volatile-clock" class="volatile">${volatileText}</span>
    </section>
  </main>
</body>
</html>`;
}

async function captureAndCompareEntry(
  page: Page,
  entry: CaptureMatrixEntry,
  options: CaptureCompareOptions = {},
): Promise<CaptureCompareResult> {
  const captured = await captureEntryScreenshot(page, entry);
  return compareCaptureToBaseline(entry, captured, options);
}

async function compareCaptureToBaseline(
  entry: CaptureMatrixEntry,
  captured: Buffer,
  options: CaptureCompareOptions = {},
): Promise<CaptureCompareResult> {
  const baselineRoot = options.baselineRoot ?? process.env.HANDSHAKE_VISUAL_BASELINE_ROOT ?? defaultBaselineRoot();
  const receiptRoot = options.receiptRoot ?? defaultReceiptRoot();
  const capturedHash = sha256Hex(captured);
  const allowBootstrap = options.allowBootstrap ?? false;
  const maxDiffPercent = options.maxDiffPercent ?? 0;

  await assertBaselineDirectoryPolicy(baselineRoot);

  if (entry.baseline_hash === BOOTSTRAP_BASELINE_HASH && allowBootstrap) {
    const baseline = await ensureContentAddressedBaseline(baselineRoot, captured);
    const manifestEntry = { ...entry, baseline_hash: baseline.hash };
    return {
      status: "bootstrapped",
      captured_hash: capturedHash,
      baseline_hash: baseline.hash,
      pixel_diff_percent: 0,
      drift_regions: [],
      baseline_path: baseline.path,
      receipt_path: null,
      receipt: null,
      manifest_entry: manifestEntry,
    };
  }

  const baselinePath = baselinePathForHash(baselineRoot, entry.baseline_hash);
  if (!fsSync.existsSync(baselinePath)) {
    throw new Error(`visual baseline missing for ${entry.scenario_id}: ${baselinePath}`);
  }

  const baselineBytes = await fs.readFile(baselinePath);
  if (capturedHash === entry.baseline_hash) {
    return {
      status: "passed",
      captured_hash: capturedHash,
      baseline_hash: entry.baseline_hash,
      pixel_diff_percent: 0,
      drift_regions: [],
      baseline_path: baselinePath,
      receipt_path: null,
      receipt: null,
      manifest_entry: entry,
    };
  }

  const diff = diffPngs(baselineBytes, captured);
  if (diff.pixel_diff_percent <= maxDiffPercent) {
    return {
      status: "passed",
      captured_hash: capturedHash,
      baseline_hash: entry.baseline_hash,
      pixel_diff_percent: diff.pixel_diff_percent,
      drift_regions: diff.drift_regions,
      baseline_path: baselinePath,
      receipt_path: null,
      receipt: null,
      manifest_entry: entry,
    };
  }

  const receipt = createVisualRegressionReceipt({
    entry,
    capturedHash,
    pixelDiffPercent: diff.pixel_diff_percent,
    driftRegions: diff.drift_regions,
    projectName: options.projectName ?? null,
  });
  const receiptPath = await writeVisualRegressionReceipt(receiptRoot, receipt);
  return {
    status: "drift",
    captured_hash: capturedHash,
    baseline_hash: entry.baseline_hash,
    pixel_diff_percent: diff.pixel_diff_percent,
    drift_regions: diff.drift_regions,
    baseline_path: baselinePath,
    receipt_path: receiptPath,
    receipt,
    manifest_entry: entry,
  };
}

function diffPngs(
  baselineBytes: Buffer,
  capturedBytes: Buffer,
): { pixel_diff_percent: number; drift_regions: DriftRegion[] } {
  const baseline = PNG.sync.read(baselineBytes);
  const captured = PNG.sync.read(capturedBytes);
  if (baseline.width !== captured.width || baseline.height !== captured.height) {
    const width = Math.max(baseline.width, captured.width);
    const height = Math.max(baseline.height, captured.height);
    return {
      pixel_diff_percent: 100,
      drift_regions: [{ x: 0, y: 0, w: width, h: height }],
    };
  }

  const diff = new PNG({ width: baseline.width, height: baseline.height });
  const diffPixels = pixelmatch(
    baseline.data,
    captured.data,
    diff.data,
    baseline.width,
    baseline.height,
    { threshold: 0.1 },
  );
  const totalPixels = baseline.width * baseline.height;
  return {
    pixel_diff_percent: totalPixels === 0 ? 0 : Number(((diffPixels / totalPixels) * 100).toFixed(6)),
    drift_regions: diffPixels === 0 ? [] : driftRegionsFromRawPixels(baseline, captured),
  };
}

function driftRegionsFromRawPixels(baseline: PNG, captured: PNG): DriftRegion[] {
  let minX = Number.POSITIVE_INFINITY;
  let minY = Number.POSITIVE_INFINITY;
  let maxX = -1;
  let maxY = -1;

  for (let y = 0; y < baseline.height; y += 1) {
    for (let x = 0; x < baseline.width; x += 1) {
      const offset = (baseline.width * y + x) * 4;
      if (
        baseline.data[offset] !== captured.data[offset]
        || baseline.data[offset + 1] !== captured.data[offset + 1]
        || baseline.data[offset + 2] !== captured.data[offset + 2]
        || baseline.data[offset + 3] !== captured.data[offset + 3]
      ) {
        minX = Math.min(minX, x);
        minY = Math.min(minY, y);
        maxX = Math.max(maxX, x);
        maxY = Math.max(maxY, y);
      }
    }
  }

  if (maxX < minX || maxY < minY) return [];
  return [{ x: minX, y: minY, w: maxX - minX + 1, h: maxY - minY + 1 }];
}

function createVisualRegressionReceipt({
  entry,
  capturedHash,
  pixelDiffPercent,
  driftRegions,
  projectName = null,
}: {
  entry: CaptureMatrixEntry;
  capturedHash: string;
  pixelDiffPercent: number;
  driftRegions: DriftRegion[];
  projectName?: string | null;
}): VisualRegressionReceipt {
  return {
    receipt_kind: "HBR_VIS_REGRESSION",
    schema_version: 1,
    receipt_uuid: uuidV7(),
    emitted_at_utc: new Date().toISOString(),
    scenario_id: entry.scenario_id,
    project_name: projectName,
    route: entry.route,
    viewport: entry.viewport,
    color_scheme: entry.color_scheme,
    locale: entry.locale,
    edge_state_tag: entry.edge_state_tag,
    baseline_hash: entry.baseline_hash,
    captured_hash: capturedHash,
    pixel_diff_percent: pixelDiffPercent,
    drift_regions: driftRegions,
    evidence_pointer: null,
  };
}

async function writeVisualRegressionReceipt(
  receiptRoot: string,
  receipt: VisualRegressionReceipt,
): Promise<string> {
  await fs.mkdir(receiptRoot, { recursive: true });
  const fileName = `hbr-vis-regression-${sanitizeFilePart(receipt.scenario_id)}-${receipt.receipt_uuid}.jsonl`;
  const receiptPath = path.join(receiptRoot, fileName);
  receipt.evidence_pointer = toPosix(path.relative(repoRoot(), receiptPath));
  await fs.writeFile(receiptPath, `${canonicalJson(receipt)}\n`, "utf8");
  return receiptPath;
}

function validateVisualRegressionReceipt(
  receipt: VisualRegressionReceipt,
  schemaPath = defaultSchemaPath(),
): true {
  const schema = JSON.parse(fsSync.readFileSync(schemaPath, "utf8"));
  const ajv = new Ajv({ strict: false, allErrors: true });
  const validate = ajv.compile(schema);
  if (!validate(receipt)) {
    throw new Error(`HBR_VIS_REGRESSION receipt schema failed: ${ajv.errorsText(validate.errors)}`);
  }
  return true;
}

function canonicalJson(value: unknown): string {
  return JSON.stringify(sortJson(value));
}

function sortJson(value: unknown): unknown {
  if (Array.isArray(value)) return value.map(sortJson);
  if (value && typeof value === "object") {
    return Object.fromEntries(
      Object.entries(value as Record<string, unknown>)
        .sort(([left], [right]) => left.localeCompare(right))
        .map(([key, child]) => [key, sortJson(child)]),
    );
  }
  return value;
}

function uuidV7(nowMs = Date.now()): string {
  const bytes = crypto.randomBytes(16);
  const timestamp = BigInt(nowMs);
  bytes[0] = Number((timestamp >> 40n) & 0xffn);
  bytes[1] = Number((timestamp >> 32n) & 0xffn);
  bytes[2] = Number((timestamp >> 24n) & 0xffn);
  bytes[3] = Number((timestamp >> 16n) & 0xffn);
  bytes[4] = Number((timestamp >> 8n) & 0xffn);
  bytes[5] = Number(timestamp & 0xffn);
  bytes[6] = (bytes[6] & 0x0f) | 0x70;
  bytes[8] = (bytes[8] & 0x3f) | 0x80;
  return [
    bytes.subarray(0, 4).toString("hex"),
    bytes.subarray(4, 6).toString("hex"),
    bytes.subarray(6, 8).toString("hex"),
    bytes.subarray(8, 10).toString("hex"),
    bytes.subarray(10, 16).toString("hex"),
  ].join("-");
}

function sanitizeFilePart(value: string): string {
  return value.toLowerCase().replace(/[^a-z0-9_-]+/g, "-").replace(/^-+|-+$/g, "");
}

function toPosix(value: string): string {
  return value.replace(/\\/g, "/");
}

module.exports = {
  BOOTSTRAP_BASELINE_HASH,
  MAX_LOCAL_BASELINE_BYTES,
  assertBaselineDirectoryPolicy,
  baselinePathForHash,
  captureAndCompareEntry,
  captureEntryScreenshot,
  captureMatrixEntryFromProject,
  canonicalJson,
  compareCaptureToBaseline,
  createVisualRegressionReceipt,
  defaultBaselineRoot,
  defaultReceiptRoot,
  defaultSchemaPath,
  ensureContentAddressedBaseline,
  sha256Hex,
  validateCaptureMatrixEntry,
  validateVisualRegressionReceipt,
};
