// Builds the REAL <SwarmBoard> into a single self-contained IIFE bundle that
// Playwright injects into Chromium. The visual gate drives the shipped board
// component, its real snapshot call, and its real Tauri event subscription.
const path = require("node:path");

const repoRoot = path.resolve(__dirname, "..", "..");
const appRoot = path.join(repoRoot, "app");
const harnessEntry = path.join(
  appRoot,
  "src",
  "components",
  "swarm",
  "swarm_board_visual_harness.tsx",
);

export interface HarnessBundle {
  js: string;
  css: string;
}

let cached: Promise<HarnessBundle> | null = null;

export function buildSwarmBoardHarness(): Promise<HarnessBundle> {
  if (cached) return cached;
  cached = (async (): Promise<HarnessBundle> => {
    const { createRequire } = require("node:module") as typeof import("node:module");
    const appRequire = createRequire(path.join(appRoot, "package.json"));
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    const { build } = appRequire("vite") as { build: (opts: unknown) => Promise<unknown> };
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    const react = appRequire("@vitejs/plugin-react") as (...args: any[]) => unknown;

    const result = (await build({
      root: appRoot,
      logLevel: "warn",
      configFile: false,
      plugins: [react()],
      define: { "process.env.NODE_ENV": '"production"' },
      build: {
        write: false,
        minify: false,
        cssCodeSplit: false,
        lib: { entry: harnessEntry, formats: ["iife"], name: "HsSwarmBoardHarness" },
      },
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
    } as any)) as any;

    const output = Array.isArray(result) ? result[0].output : result.output;
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    const jsChunk = output.find((o: any) => o.type === "chunk");
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    const cssAsset = output.find((o: any) => o.type === "asset" && o.fileName.endsWith(".css"));
    if (!jsChunk) throw new Error("swarm board harness build produced no JS chunk");
    const css = cssAsset
      ? (typeof cssAsset.source === "string" ? cssAsset.source : Buffer.from(cssAsset.source).toString("utf8"))
      : "";
    return { js: jsChunk.code as string, css };
  })();
  return cached;
}
