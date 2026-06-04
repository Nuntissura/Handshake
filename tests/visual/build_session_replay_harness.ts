// Builds the REAL <SessionReplayPanel> component (and its real Disclosure host)
// into a single self-contained IIFE bundle that the Playwright visual spec
// injects into a genuine Chromium page. Mirrors build_terminal_harness.ts: the
// visual gate drives the actual shipped React component, not a hand-authored
// static HTML fixture.
//
// Uses Vite's programmatic `build` in lib mode (rooted at the app workspace so
// react/react-dom resolve from the app's store) to bundle the harness entry
// (app/src/components/session/session_replay_visual_harness.tsx). The result is
// cached in-process so repeated spec runs pay the build cost once.

const path = require("node:path");

const repoRoot = path.resolve(__dirname, "..", "..");
const appRoot = path.join(repoRoot, "app");
const harnessEntry = path.join(
  appRoot,
  "src",
  "components",
  "session",
  "session_replay_visual_harness.tsx",
);

export interface HarnessBundle {
  /** The IIFE JS bundle (React + react-dom + the real SessionReplayPanel + mount). */
  js: string;
  /** Concatenated CSS the component imports (usually empty for this panel). */
  css: string;
}

let cached: Promise<HarnessBundle> | null = null;

export function buildSessionReplayHarness(): Promise<HarnessBundle> {
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
        lib: { entry: harnessEntry, formats: ["iife"], name: "HsSessionReplayHarness" },
      },
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
    } as any)) as any;

    const output = Array.isArray(result) ? result[0].output : result.output;
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    const jsChunk = output.find((o: any) => o.type === "chunk");
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    const cssAsset = output.find((o: any) => o.type === "asset" && o.fileName.endsWith(".css"));
    if (!jsChunk) throw new Error("session replay harness build produced no JS chunk");
    const css = cssAsset
      ? (typeof cssAsset.source === "string" ? cssAsset.source : Buffer.from(cssAsset.source).toString("utf8"))
      : "";
    return { js: jsChunk.code as string, css };
  })();
  return cached;
}
