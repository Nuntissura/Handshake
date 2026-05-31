// Builds the REAL <SwarmControlRoom> spawn form (its real useSwarmRoom hook +
// SwarmSpawnSection + SwarmSessionsSection) into a single self-contained IIFE
// bundle that the Playwright visual spec injects into a genuine Chromium page.
// Mirrors build_session_replay_harness.ts / build_terminal_harness.ts: the
// visual gate drives the ACTUAL shipped React component, not a static mockup.
//
// The only stand-in is a deterministic Tauri IPC mock (installed via
// @tauri-apps/api/mocks `mockIPC` inside the harness entry) so listWorktrees /
// spawnSession / listActiveSessions / resourceSnapshot resolve without a live
// backend. Everything the operator sees — the worktree picker, the new-entry
// reveal, the disk working-dir field, the recorded-only isolation-tier selector
// + its honesty note, and the sessions-table Worktree column — is the REAL
// component's own render output.

const path = require("node:path");

const repoRoot = path.resolve(__dirname, "..", "..");
const appRoot = path.join(repoRoot, "app");
const harnessEntry = path.join(
  appRoot,
  "src",
  "components",
  "swarm",
  "swarm_spawn_visual_harness.tsx",
);

export interface HarnessBundle {
  /** The IIFE JS bundle (React + react-dom + the real SwarmControlRoom + mount). */
  js: string;
  /** Concatenated CSS the component imports (usually empty for this form). */
  css: string;
}

let cached: Promise<HarnessBundle> | null = null;

export function buildSwarmSpawnHarness(): Promise<HarnessBundle> {
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
        lib: { entry: harnessEntry, formats: ["iife"], name: "HsSwarmSpawnHarness" },
      },
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
    } as any)) as any;

    const output = Array.isArray(result) ? result[0].output : result.output;
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    const jsChunk = output.find((o: any) => o.type === "chunk");
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    const cssAsset = output.find((o: any) => o.type === "asset" && o.fileName.endsWith(".css"));
    if (!jsChunk) throw new Error("swarm spawn harness build produced no JS chunk");
    const css = cssAsset
      ? (typeof cssAsset.source === "string" ? cssAsset.source : Buffer.from(cssAsset.source).toString("utf8"))
      : "";
    return { js: jsChunk.code as string, css };
  })();
  return cached;
}
