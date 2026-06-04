// Builds the REAL <SwarmOperatorSurface> (its real useSwarmRoom hook + the
// SessionWorkbench, provider-rich OperatorChat picker, shared TerminalPanel, and
// SessionReplayPanel drawers) into a single self-contained IIFE bundle that the
// Playwright visual spec injects into a genuine Chromium page. Mirrors
// build_swarm_spawn_harness.ts / build_session_replay_harness.ts: the visual
// gate drives the ACTUAL shipped React component, not a static mockup.
//
// The only stand-in is a deterministic Tauri IPC mock (installed via
// @tauri-apps/api/mocks `mockIPC` inside the harness entry) so the swarm /
// terminal / session-transcript commands resolve without a live backend.
// Everything the operator sees — the all-providers chat picker, the chat
// generate, the "Show captured terminal" focus, and the "Open full transcript"
// reveal — is the REAL component's own render output.

const path = require("node:path");

const repoRoot = path.resolve(__dirname, "..", "..");
const appRoot = path.join(repoRoot, "app");
const harnessEntry = path.join(
  appRoot,
  "src",
  "components",
  "swarm",
  "session_workbench_visual_harness.tsx",
);

export interface HarnessBundle {
  /** The IIFE JS bundle (React + react-dom + the real SwarmOperatorSurface + mount). */
  js: string;
  /** Concatenated CSS the component imports. */
  css: string;
}

let cached: Promise<HarnessBundle> | null = null;

export function buildSessionWorkbenchHarness(): Promise<HarnessBundle> {
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
        lib: { entry: harnessEntry, formats: ["iife"], name: "HsSessionWorkbenchHarness" },
      },
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
    } as any)) as any;

    const output = Array.isArray(result) ? result[0].output : result.output;
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    const jsChunk = output.find((o: any) => o.type === "chunk");
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    const cssAsset = output.find((o: any) => o.type === "asset" && o.fileName.endsWith(".css"));
    if (!jsChunk) throw new Error("session workbench harness build produced no JS chunk");
    const css = cssAsset
      ? (typeof cssAsset.source === "string" ? cssAsset.source : Buffer.from(cssAsset.source).toString("utf8"))
      : "";
    return { js: jsChunk.code as string, css };
  })();
  return cached;
}
