// WP-KERNEL-009 / MT-027 — frontend worker bundling check (+ MT-026 dynamic
// no-stray-writes proof).
//
// Runs the REAL builds (no mocks, no dev server):
//   1. `pnpm run build`          → app/dist          (tsc + vite, main app)
//   2. `pnpm run build:harness`  → app/dist-harness  (dependency-policy
//      harness — the product surface that mounts the bundled Monaco+Tiptap
//      stack; built from the same source tree and lockfile)
// then proves, on the BUILT artifacts:
//   - zero worker loads referencing external origins: every `new Worker(` /
//     `new SharedWorker(` / `importScripts(` site in any built JS file must
//     not point at an http(s) origin;
//   - zero CDN host references (patterns from the runtime dependency
//     allowlist, MT-017/MT-018) anywhere in the built output;
//   - the Monaco worker chunks (editor/ts/json/css/html) exist as local
//     files in every dist tree that bundles Monaco. Today that is
//     app/dist-harness: the main app does not mount Monaco yet (no product
//     view imports src/lib/monaco/setup). The detection is content-based
//     ("MonacoEnvironment" marker), so the moment Monaco lands in the main
//     app bundle, app/dist is held to the same worker-chunk requirement
//     automatically;
//   - MT-026 dynamic boundary proof: the build writes NOTHING into app/src,
//     the repo root, or new top-level entries under app/. (src/backend is
//     covered by the static config boundary in
//     scripts/lib/portable_artifacts_check.mjs instead of a snapshot,
//     because parallel agent sessions legitimately edit it while builds run.)
//
// Output contract (consumed by the MT-032 validator hook
// scripts/check-dependency-policy.mjs): human detail on stderr, ONE JSON
// summary object on stdout, exit 0 only when every assertion holds.
//
// Usage:
//   node scripts/check-worker-bundling.mjs            # full build + scan
//   node scripts/check-worker-bundling.mjs --skip-build   # scan existing dists
//
// Rust-side note: cargo outputs are routed outside the repo by
// .cargo/config.toml (MT-026) and are not part of this frontend scan.

import { spawnSync } from "node:child_process";
import { existsSync, readFileSync, readdirSync, statSync } from "node:fs";
import { join, relative, sep } from "node:path";
import { fileURLToPath } from "node:url";
import {
  externalWorkerLoads,
  loadAllowlist,
  partitionCdnHits,
} from "./lib/dependency_policy_scans.mjs";

const appDir = join(fileURLToPath(new URL(".", import.meta.url)), "..");
const repoRoot = join(appDir, "..");
const skipBuild = process.argv.includes("--skip-build");

const REQUIRED_MONACO_WORKERS = ["editor", "ts", "json", "css", "html"];

function listFilesRecursive(rootDir, skipNames = new Set()) {
  if (!existsSync(rootDir)) return [];
  const out = [];
  const stack = [rootDir];
  while (stack.length > 0) {
    const dir = stack.pop();
    for (const entry of readdirSync(dir, { withFileTypes: true })) {
      if (skipNames.has(entry.name)) continue;
      const full = join(dir, entry.name);
      if (entry.isDirectory()) stack.push(full);
      else out.push(full);
    }
  }
  return out;
}

function rel(fullPath) {
  return relative(repoRoot, fullPath).split(sep).join("/");
}

/** path → "size:mtimeMs" snapshot for the MT-026 dynamic no-writes proof. */
function snapshotTree(rootDir, skipNames) {
  const snapshot = new Map();
  for (const file of listFilesRecursive(rootDir, skipNames)) {
    const stats = statSync(file);
    snapshot.set(rel(file), `${stats.size}:${stats.mtimeMs}`);
  }
  return snapshot;
}

function topLevelNames(dir) {
  return new Set(readdirSync(dir));
}

function diffSnapshots(before, after) {
  const changes = [];
  for (const [path, sig] of after) {
    if (!before.has(path)) changes.push({ path, kind: "created" });
    else if (before.get(path) !== sig) changes.push({ path, kind: "modified" });
  }
  for (const path of before.keys()) {
    if (!after.has(path)) changes.push({ path, kind: "deleted" });
  }
  return changes;
}

function runBuild(scriptName) {
  process.stderr.write(`[worker-bundling] running pnpm run ${scriptName}\n`);
  // Build output goes to STDERR (fd 2): this script's stdout contract is ONE
  // JSON summary object, and the MT-032 hook parses it programmatically.
  const result = spawnSync("pnpm", ["run", scriptName], {
    cwd: appDir,
    stdio: ["ignore", 2, 2],
    shell: process.platform === "win32",
  });
  if (result.status !== 0) {
    throw new Error(`pnpm run ${scriptName} failed with exit code ${result.status}`);
  }
}

function scanDistTree(distDir, allowlist) {
  const cdnPatterns = allowlist.forbidden_runtime_dependency_classes
    .find((c) => c.id === "cdn_runtime_asset")
    .source_scan_patterns.map((p) => p.toLowerCase());

  const files = listFilesRecursive(distDir);
  const jsFiles = files.filter((f) => /\.(js|mjs|cjs)$/i.test(f));
  const textFiles = files.filter((f) => /\.(js|mjs|cjs|html|css|json|webmanifest)$/i.test(f));

  const externalWorkerRefs = [];
  const cdnHits = [];
  const cdnExceptionsApplied = [];
  let monacoMarker = false;

  for (const file of textFiles) {
    const content = readFileSync(file, "utf8");
    for (const pattern of cdnPatterns) {
      const { violations, exempted } = partitionCdnHits({
        content,
        relPath: rel(file),
        pattern,
        allowlist,
      });
      cdnHits.push(...violations);
      cdnExceptionsApplied.push(...exempted);
    }
    if (jsFiles.includes(file)) {
      externalWorkerRefs.push(...externalWorkerLoads(content, rel(file)));
      if (content.includes("MonacoEnvironment")) monacoMarker = true;
    }
  }

  const workerChunks = files
    .map((f) => rel(f))
    .filter((f) => /(^|\/)[^/]*\.worker-[^/]*\.js$/.test(f) || /(^|\/)worker-[^/]*\.js$/.test(f));

  const missingMonacoWorkers = monacoMarker
    ? REQUIRED_MONACO_WORKERS.filter(
        (kind) => !workerChunks.some((chunk) => chunk.includes(`${kind}.worker-`)),
      )
    : [];

  return {
    dist: rel(distDir),
    files_scanned: textFiles.length,
    bundles_monaco: monacoMarker,
    worker_chunks: workerChunks,
    missing_monaco_workers: missingMonacoWorkers,
    external_worker_refs: externalWorkerRefs,
    cdn_hits: cdnHits,
    cdn_exceptions_applied: cdnExceptionsApplied,
  };
}

function main() {
  const allowlist = loadAllowlist(repoRoot);
  const distMain = join(appDir, "dist");
  const distHarness = join(appDir, "dist-harness");

  // MT-026 dynamic proof: snapshot the protected trees around the real build.
  const protectedSnapshots = () => ({
    appSrc: snapshotTree(join(appDir, "src")),
    appTopLevel: topLevelNames(appDir),
    repoRootTopLevel: topLevelNames(repoRoot),
  });

  let strayWrites = [];
  if (!skipBuild) {
    const before = protectedSnapshots();
    runBuild("build");
    runBuild("build:harness");
    const after = protectedSnapshots();
    strayWrites = diffSnapshots(before.appSrc, after.appSrc);
    for (const name of after.appTopLevel) {
      // dist/dist-harness are the declared artifact dirs; everything else new
      // at app/ top level is a stray build write.
      if (!before.appTopLevel.has(name) && name !== "dist" && name !== "dist-harness") {
        strayWrites.push({ path: `app/${name}`, kind: "created" });
      }
    }
    for (const name of after.repoRootTopLevel) {
      if (!before.repoRootTopLevel.has(name)) {
        strayWrites.push({ path: name, kind: "created" });
      }
    }
  }

  for (const dist of [distMain, distHarness]) {
    if (!existsSync(dist)) {
      throw new Error(
        `${rel(dist)} missing — run without --skip-build (the check must scan real built assets)`,
      );
    }
  }

  const trees = [scanDistTree(distMain, allowlist), scanDistTree(distHarness, allowlist)];
  const harnessTree = trees.find((t) => t.dist.endsWith("dist-harness"));

  const failures = [];
  if (strayWrites.length > 0) {
    failures.push(`build wrote outside the artifact boundary: ${JSON.stringify(strayWrites)}`);
  }
  for (const tree of trees) {
    if (tree.external_worker_refs.length > 0) {
      failures.push(`${tree.dist}: worker loads reference external origins`);
    }
    if (tree.cdn_hits.length > 0) {
      failures.push(`${tree.dist}: CDN host references in built output`);
    }
    if (tree.missing_monaco_workers.length > 0) {
      failures.push(
        `${tree.dist}: bundles Monaco but lacks worker chunks: ${tree.missing_monaco_workers.join(", ")}`,
      );
    }
  }
  // The harness is the canonical Monaco surface: it must ALWAYS bundle Monaco
  // with all five worker chunks. If this fires, the harness regressed.
  if (!harnessTree.bundles_monaco) {
    failures.push("dist-harness no longer bundles Monaco — harness regression");
  }

  const summary = {
    check: "worker-bundling",
    mt: ["MT-027", "MT-026-dynamic"],
    pass: failures.length === 0,
    built: !skipBuild,
    stray_writes: strayWrites,
    trees,
    failures,
  };
  process.stdout.write(`${JSON.stringify(summary, null, 2)}\n`);
  process.exitCode = failures.length === 0 ? 0 : 1;
}

try {
  main();
} catch (error) {
  process.stdout.write(
    `${JSON.stringify({
      check: "worker-bundling",
      pass: false,
      error: error instanceof Error ? error.message : String(error),
    })}\n`,
  );
  process.exitCode = 1;
}
